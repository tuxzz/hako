#![feature(range_contains, duration_as_u128)]

#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate tera;
use tera::{Tera, Context};
extern crate actix_web;
use actix_web::{http, server, App, Responder};
extern crate serde_json;
extern crate chrono;
use chrono::prelude::*;
extern crate percent_encoding;
use percent_encoding::{utf8_percent_encode, DEFAULT_ENCODE_SET};

use std::time::Instant;

mod db;

lazy_static! {
  static ref TERA: Tera = {
    let mut tera = compile_templates!("template/**/*");
    tera.autoescape_on(vec!["html", ".sql"]);
    tera
  };

  static ref DB: db::DB<'static> = db::DB::open("packed.db");
  static ref S_D_FAC: String = format!("{:?}", DB.d_fac());
  static ref S_DB_DATE: String = {
    let (m, d, y) = DB.db_date();
    format!("{:02}/{:02}/{:04}", m, d, y)
  };
}

const REV: u32 = 0;

/* Extract-Default Query String */
/* plain keyword */
/* r18:yes|no */
/* year:-?point,+|-?{[|(}from?, to?{)|]},+ */
/* rank:{[|(}from?, to?{)|]} */
/* fav:{[|(}from?, to?{)|]} */
/* tag:-?str,+ */

#[derive(Serialize, Deserialize, Debug, Clone)]
struct PresentationSubject {
  link_target: String,
  image_url: String,
  title_main: String,
  title_orig: Option<String>,
  sub_type: &'static str,
  info: String,
  rank: u32,
  recommend_rate: Option<String>,
  star_count: String,
  rating_count: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct PresentationPager {
  prev_link: Option<String>,
  next_link: Option<String>,
  min_link: Option<String>,
  max_link: Option<String>,
  page_list: Vec<(u32, Option<String>)>,
  curr_page: u32,
  max_page: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct PresentationSearch {
  kwd_str: String,
  user: String,
  year: (bool, bool, bool, bool, bool, bool),
  tag: (u8, u8, u8, u8, u8, u8, u8, u8, u8, u8,),
  ord: u8,
  r18: u8,
  base_url: String,
  curr_skip: u32,
}

fn construct_pager<F>(curr_page: u32, max_page: u32, link_gen: F) -> PresentationPager
where
  F: Fn(u32) -> String,
{
  assert!(curr_page < max_page);
  let prev_link = if curr_page == 0 { None } else { Some(link_gen(curr_page - 1)) };
  let next_link = if curr_page >= max_page - 1 { None } else { Some(link_gen(curr_page + 1)) };
  let (min_link, max_link) = if max_page <= 1 { (None, None) } else { 
    (if curr_page == 0 { None } else { Some(link_gen(0)) }, if curr_page >= max_page - 1 { None } else { Some(link_gen(max_page - 1)) })
  };

  let mut i_min_page = if curr_page <= 2 { 0 } else { curr_page - 2 };
  let i_max_page = if max_page <= 5 { 4 } else { if curr_page >= max_page - 3 { max_page - 1 } else { i_min_page + 4 } };
  i_min_page -= 4 - (i_max_page - i_min_page);
  
  let page_list = (i_min_page..i_max_page + 1).map(|i_page| if i_page < max_page { (i_page + 1, Some(link_gen(i_page))) } else { (i_page + 1, None) }).collect();
  PresentationPager { max_link, min_link, prev_link, next_link, page_list, curr_page: curr_page + 1, max_page }
}

fn parse_sort_mode_str(s: &str) -> Result<(bool, db::SortMode), actix_web::HttpResponse> {
  Ok(match s {
    "ar" => (true, db::SortMode::Recommend),
    "dr" => (false, db::SortMode::Recommend),
    "an" => (true, db::SortMode::Name),
    "dn" => (false, db::SortMode::Name),
    "ak" => (true, db::SortMode::Rank),
    "dk" => (false, db::SortMode::Rank),
    "ad" => (true, db::SortMode::Date),
    "dd" => (false, db::SortMode::Date),
    "af" => (true, db::SortMode::FavCount),
    "df" => (false, db::SortMode::FavCount),
    "al" => (true, db::SortMode::Relative),
    "dl" => (false, db::SortMode::Relative),
    _ => { return Err(actix_web::HttpResponse::BadRequest().content_type("text/plain").body("Bad sort_mode_str")); },
  })
}

fn encode_sort_mode_str_to_u8(s: &str) -> u8 {
  ["ar", "dr", "al", "dl", "ak", "dk", "ad", "dd", "af", "df"].iter().enumerate().find(|(_, x)| *x == &s).unwrap().0 as u8
}

fn encode_sub_type_to_str(t: db::PackedSubjectSubtype) -> &'static str {
  match t {
    db::PackedSubjectSubtype::Unknown => "",
    db::PackedSubjectSubtype::TV => "TV",
    db::PackedSubjectSubtype::OVA => "OVA",
    db::PackedSubjectSubtype::Web => "Web",
    db::PackedSubjectSubtype::Movie => "Movie",
  }
}

fn subject_to_presentation(x: &db::PackedSubject, user_recommend: Option<u16>) -> PresentationSubject {
  PresentationSubject {
    link_target: format!("https://bgm.tv/subject/{}", x.subject_id),
    image_url: format!("https://lain.bgm.tv/pic/cover/{}", &x.image_partial_url),
    title_main: if !x.name_cn.is_empty() { x.name_cn.clone() } else { x.name.clone() },
    title_orig: if x.name_cn.is_empty() { None } else { Some(x.name.clone()) },
    sub_type: encode_sub_type_to_str(x.sub_type),
    info: format!("{:02}/{:02}/{:04}", x.air_m, x.air_d, x.air_y),
    rank: x.rank,
    recommend_rate: user_recommend.map(|u| format!("{}", u)),
    star_count: format!("{:.2}", x.score),
    rating_count: x.rating_count,
  }
}

/* unsearched */
fn unsearched(info: actix_web::Path<(String, u32,)>) -> impl Responder {
  let mut code = 200;
  let start_time = Instant::now();
  let (sort_mode_str, n_skip,) = info.into_inner();

  let result_count = DB.subject_count();
  if n_skip as usize >= result_count {
    return actix_web::HttpResponse::NotFound().content_type("text/plain").body(format!("404 ({} out of {})", n_skip, result_count));
  }

  /* parse sort mode */
  let (is_sort_ascent, sort_mode) = match parse_sort_mode_str(sort_mode_str.as_str()) { Ok(x) => x, Err(r) => { return r; } };

  /* perform query */
  let result = db::sort_result_unsearched(DB.subject_iter().collect(), sort_mode, is_sort_ascent);

  /* PresentationSearch */
  let search_obj = PresentationSearch {
    kwd_str: String::new(),
    user: String::new(),
    year: (false, false, false, false, false, false,),
    tag: (2, 2, 2, 2, 2, 2, 2, 2, 2, 2,),
    ord: encode_sort_mode_str_to_u8(&sort_mode_str),
    r18: 3,
    base_url: String::new(),
    curr_skip: n_skip,
  };

  let mut context = Context::new();
  let result = result.into_iter().skip(n_skip as usize).take(25).map(|x| subject_to_presentation(x, None)).collect::<Vec<_>>();
  context.insert("subject_list", &result);
  context.insert("d_fac", S_D_FAC.as_str());
  context.insert("rev", &REV);
  context.insert("pager", &construct_pager(n_skip / 25, ((result_count + 24) / 25) as u32, move |x| format!("/{}/{}", sort_mode_str, x * 25)));
  context.insert("search", &search_obj);
  context.insert("db_date", S_DB_DATE.as_str());
  context.insert("query_time", &format!("{}μs", start_time.elapsed().as_micros()));
  context.insert("code", &code);
  match code {
    200 => actix_web::HttpResponse::Ok(),
    404 => actix_web::HttpResponse::NotFound(),
    500 => actix_web::HttpResponse::BadRequest(),
    _ => { unreachable!(); }
  }.content_type("text/html").body(TERA.render("hako_list_tiny.html", &context).unwrap())
}

/* searched */
fn searched(info: actix_web::Path<(String, String, u32,)>) -> impl Responder {
  let mut code = 200;
  let start_time = Instant::now();
  let (query_str, sort_mode_str, n_skip,) = info.into_inner();
  if query_str.len() > 127 {
    return actix_web::HttpResponse::BadRequest().content_type("text/plain").body("query_str is too long");
  }

  let total_subject_count = DB.subject_count();
  if n_skip as usize >= total_subject_count {
    return actix_web::HttpResponse::NotFound().content_type("text/plain").body(format!("404 ({} out of maximum possible size {})", n_skip, total_subject_count));
  }

  /* parse sort mode */
  let (is_sort_ascent, sort_mode) = match parse_sort_mode_str(sort_mode_str.as_str()) { Ok(x) => x, Err(r) => { return r; } };

  /* parse search ticket */
  let (s_kwd_list, s_tag_list, s_year_list, s_user, s_r18) = match serde_json::from_str::<(Vec<(u8, String)>, Vec<(u8, String)>, Vec<(Option<u16>, Option<u16>)>, Option<String>, u8)>(query_str.as_str()) {
    Ok(x) => x,
    Err(e) => { return actix_web::HttpResponse::BadRequest().content_type("text/plain").body(format!("Bad query_str: {:?}", e));  }
  };
  let ticket = {
    let keyword_list = s_kwd_list.iter().map(|(opt, kwd)| {
      let a = if *opt & 0b01 == 0 { db::Relation::Include(kwd.to_lowercase().clone()) } else { db::Relation::Exclude(kwd.to_lowercase().clone()) };
      if *opt & 0b10 == 0 { db::SearchMode::PartialMatch(a) } else { db::SearchMode::ExactMatch(a) }
    }).collect::<Vec<_>>();
    let tag_list = s_tag_list.iter().map(|(include, tag)| {
      let i = DB.get_tag_id_by_name(tag.to_lowercase().as_str()).unwrap_or(u32::max_value());
      if *include == 1 { db::Relation::Include(i) } else { db::Relation::Exclude(i) }
    }).collect::<Vec<_>>();
    let year_list = s_year_list.iter().map(|(from, to)| match from {
      Some(a) => {
        match to {
          Some(b) => db::SearchRange::Range(*a as u32, *b as u32),
          None => db::SearchRange::RangeFrom(*a as u32),
        }
      },
      None => {
        match to {
          Some(b) => db::SearchRange::RangeTo(*b as u32),
          None => db::SearchRange::RangeFull,
        }
      }
    }).collect::<Vec<_>>();
    let user_id = s_user.as_ref().map(|u| match u.parse::<u32>() {
      Ok(uid) => uid,
      Err(_) => match DB.get_user_id_by_username(u.to_lowercase().as_str()) {
        Some(uid) => uid,
        None => 0,
      },
    });

    let r18_mode = match s_r18 {
      0b01 => Some(false),
      0b10 => Some(true),
      0b11 => None,
      _ => return actix_web::HttpResponse::BadRequest().content_type("text/plain").body("Bad r18_mode"),
    };
    db::SearchTicket {
      keyword_list,
      tag_list,
      year_list,
      rank: db::SearchRange::RangeFull,
      rating_count: db::SearchRange::RangeFull,
      r18: r18_mode,
      for_user: user_id,
    }
  };

  /* perform query */
  let result = DB.search_by_ticket(&ticket);
  let result_count = result.len();
  if n_skip as usize >= result_count {
    //return actix_web::HttpResponse::NotFound().content_type("text/plain").body(format!("404 ({} out of {})\n{:#?}", n_skip, result_count, ticket));
    code = 404;
  }
  let result = db::sort_result(result, sort_mode, is_sort_ascent);

  /* PresentationSearch */
  let percent_query_str = utf8_percent_encode(&query_str, DEFAULT_ENCODE_SET).to_string();
  let this_year = Date::<FixedOffset>::from_utc(Utc::today().naive_utc(), FixedOffset::east(9 * 3600)).year() as u32;
  let search_obj = PresentationSearch {
    kwd_str: s_kwd_list.iter().map(|(opt, kwd)| format!("{}{}{}", if *opt & 0b10 == 0 { "" } else { "*" }, if *opt & 0b01 == 0 { "" } else { "-" }, kwd)).fold(String::new(), |acc, x| if acc.is_empty() { acc } else { acc + " " } + x.as_str()),
    user: s_user.unwrap_or(String::new()),
    year: (
      ticket.year_list.contains(&db::SearchRange::RangeTo(2000)),
      ticket.year_list.contains(&db::SearchRange::Range(2000, 2005)),
      ticket.year_list.contains(&db::SearchRange::Range(2005, 2009)),
      ticket.year_list.contains(&db::SearchRange::Range(2009, 2015)),
      ticket.year_list.contains(&db::SearchRange::RangeFrom(2015)),
      ticket.year_list.contains(&db::SearchRange::Range(this_year, this_year + 1)),
    ),
    tag: (
      match s_tag_list.iter().find(|x| x.1 == "奇幻") { Some(x) => x.0, None => 2 },
      match s_tag_list.iter().find(|x| x.1 == "科幻") { Some(x) => x.0, None => 2 },
      match s_tag_list.iter().find(|x| x.1 == "冒险") { Some(x) => x.0, None => 2 },
      match s_tag_list.iter().find(|x| x.1 == "轻小说改") { Some(x) => x.0, None => 2 },
      match s_tag_list.iter().find(|x| x.1 == "漫画改") { Some(x) => x.0, None => 2 },
      match s_tag_list.iter().find(|x| x.1 == "游戏改") { Some(x) => x.0, None => 2 },
      match s_tag_list.iter().find(|x| x.1 == "GAL改") { Some(x) => x.0, None => 2 },
      match s_tag_list.iter().find(|x| x.1 == "日常") { Some(x) => x.0, None => 2 },
      match s_tag_list.iter().find(|x| x.1 == "搞笑") { Some(x) => x.0, None => 2 },
      match s_tag_list.iter().find(|x| x.1 == "里番") { Some(x) => x.0, None => 2 },
    ),
    ord: encode_sort_mode_str_to_u8(&sort_mode_str),
    r18: s_r18,
    base_url: format!("/search/{}", percent_query_str),
    curr_skip: n_skip,
  };

  let mut context = Context::new();
  let result = result.into_iter().skip(n_skip as usize).take(25).map(|x| subject_to_presentation(&x.subject, if x.user_recommend == 65535 { None } else { Some(total_subject_count as u16 - x.user_recommend) })).collect::<Vec<_>>();
  context.insert("d_fac", S_D_FAC.as_str());
  context.insert("rev", &REV);
  if code == 200 {
    context.insert("pager", &construct_pager(n_skip / 25, ((result_count + 24) / 25) as u32, move |x| format!("/search/{}/{}/{}", percent_query_str, sort_mode_str, x * 25)));
    context.insert("subject_list", &result);
  }
  context.insert("search", &search_obj);
  context.insert("db_date", S_DB_DATE.as_str());
  context.insert("query_time", &format!("{}μs", start_time.elapsed().as_micros()));
  context.insert("code", &code);
  match code {
    200 => actix_web::HttpResponse::Ok(),
    404 => actix_web::HttpResponse::NotFound(),
    500 => actix_web::HttpResponse::BadRequest(),
    _ => { unreachable!(); }
  }.content_type("text/html").body(TERA.render("hako_list_tiny.html", &context).unwrap())
}

fn main() {
  DB.subject_iter();
  server::new(|| {
    App::new()
    .route("/{sort_mode}/{n_skip}", http::Method::GET, unsearched)
    .route("/search/{query_str}/{sort_mode}/{n_skip}", http::Method::GET, searched)
  }).bind("127.0.0.1:8080").unwrap().run();
}