extern crate bincode;
extern crate memmap;

use std::path::Path;

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum PackedSubjectSubtype {
  Unknown,
  TV,
  OVA,
  Web,
  Movie,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PackedSubject {
  pub subject_id: u32,
  pub rank: u32,
  pub name: String,
  pub name_cn: String,
  pub image_partial_url: String,
  pub tag_list: Vec<(u32, f32)>,
  pub score: f32,
  pub rating_count: u32,
  pub air_y: u16,
  pub air_m: u8,
  pub air_d: u8,
  pub sub_type: PackedSubjectSubtype,
  pub is_r18: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct PackedDatabasePersistenceTable {
  db_date: (u8, u8, u16),
  d_fac: [f32; 2],
  subject_packed_list: Vec<PackedSubject>,
  user_id_list: Vec<u32>,
  user_username_list: Vec<String>,
  tag_name_list: Vec<String>,
}

pub struct DB<'a> {
  persistence_table: PackedDatabasePersistenceTable,
  map_table_handle: memmap::Mmap,
  map_table: &'a [u16],
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Eq, PartialEq)]
pub enum SearchRange {
  Range(u32, u32),
  RangeTo(u32),
  RangeFrom(u32),
  RangeFull,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Eq, PartialEq)]
pub enum SortMode {
  Recommend,
  Relative,
  Name,
  Rank,
  Date,
  FavCount,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Relation<T: Clone> {
  Include(T),
  Exclude(T),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum SearchMode<T: Clone> {
  ExactMatch(T),
  PartialMatch(T),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SearchTicket {
  pub keyword_list: Vec<SearchMode<Relation<String>>>,
  pub tag_list: Vec<Relation<u32>>,
  pub year_list: Vec<SearchRange>,
  pub rank: SearchRange,
  pub rating_count: SearchRange,
  pub r18: Option<bool>,
  pub for_user: Option<u32>,
}

pub struct SearchResult<'a> {
  pub subject: &'a PackedSubject,
  pub keyword_relative: f32,
  pub user_recommend: u16,
}

pub fn match_keyword_exact(kwd: &str, target: &str) -> bool {
  target.to_lowercase().contains(kwd)
}

pub fn match_keyword_partial(a: &[char], b: &[char]) -> f32 {
  let (n, m) = (a.len(), b.len());
  let (s, p) = (n + 1, m + 1);
  let mut c = Vec::new();
  c.resize(s * p, 0);
  for i in 0..s {
    c[i * p] = i;
  }
  for i in 1..p {
    c[i] = i;
  }
  for i in 0..n {
    for j in 0..m {
      let x = c[i * p + (j + 1)] + 1;
      let y = c[(i + 1) * p + j] + 1;
      let z = if a[i] == b[j] { c[i * p + j] } else { c[i * p + j] + 1 };
      c[(i + 1) * p + (j + 1)] = x.min(y).min(z);
    }
  }
  (1.0 - c[n * p + m] as f32 / a.len().max(b.len()) as f32).max(0.0).min(1.0)
}

pub fn score_mapper(x: &PackedSubject) -> f32 {
  x.score * ((x.rating_count as f32 / 100.0).min(1.0) + 1.0).log2()
}

fn name_mapper(x: &PackedSubject) -> &str {
  if x.name_cn.is_empty() { x.name.as_str() } else { x.name_cn.as_str() }
}

fn date_mapper(x: &PackedSubject) -> u32 {
  (x.air_y as u32) << 16 | (x.air_m as u32) << 8 | (x.air_d as u32)
}

pub fn sort_result_unsearched(mut l: Vec<&PackedSubject>, mode: SortMode, ascent: bool) -> Vec<&PackedSubject> {
  if ascent {
    match mode {
      SortMode::Recommend | SortMode::Relative | SortMode::Rank => l.sort_by(|a, b| score_mapper(a).partial_cmp(&score_mapper(b)).unwrap()),
      SortMode::Name => l.sort_by(|a, b| name_mapper(a).cmp(&name_mapper(b))),
      SortMode::Date => l.sort_by(|a, b| date_mapper(a).cmp(&date_mapper(b))),
      SortMode::FavCount => l.sort_by(|a, b| a.rating_count.cmp(&b.rating_count)),
    };
  }
  else {
    match mode {
      SortMode::Recommend | SortMode::Relative | SortMode::Rank => l.sort_by(|b, a| score_mapper(a).partial_cmp(&score_mapper(b)).unwrap()),
      SortMode::Name => l.sort_by(|b, a| name_mapper(a).cmp(&name_mapper(b))),
      SortMode::Date => l.sort_by(|b, a| date_mapper(a).cmp(&date_mapper(b))),
      SortMode::FavCount => l.sort_by(|b, a| a.rating_count.cmp(&b.rating_count)),
    };
  }
  l
}

pub fn sort_result(mut l: Vec<SearchResult>, mode: SortMode, ascent: bool) -> Vec<SearchResult> {
  if ascent {
    match mode {
      SortMode::Recommend => l.sort_by(|a, b| a.user_recommend.partial_cmp(&b.user_recommend).unwrap()),
      SortMode::Relative => l.sort_by(|a, b| a.keyword_relative.partial_cmp(&b.keyword_relative).unwrap()),
      SortMode::Name => l.sort_by(|a, b| name_mapper(&a.subject).cmp(&name_mapper(&b.subject))),
      SortMode::Rank => l.sort_by(|a, b| score_mapper(&a.subject).partial_cmp(&score_mapper(&b.subject)).unwrap()),
      SortMode::Date => l.sort_by(|a, b| date_mapper(&a.subject).cmp(&date_mapper(&b.subject))),
      SortMode::FavCount => l.sort_by(|a, b| a.subject.rating_count.cmp(&b.subject.rating_count)),
    };
  }
  else {
    match mode {
      SortMode::Recommend => l.sort_by(|b, a| a.user_recommend.partial_cmp(&b.user_recommend).unwrap()),
      SortMode::Relative => l.sort_by(|b, a| a.keyword_relative.partial_cmp(&b.keyword_relative).unwrap()),
      SortMode::Name => l.sort_by(|b, a| name_mapper(&a.subject).cmp(&name_mapper(&b.subject))),
      SortMode::Rank => l.sort_by(|b, a| score_mapper(&a.subject).partial_cmp(&score_mapper(&b.subject)).unwrap()),
      SortMode::Date => l.sort_by(|b, a| date_mapper(&a.subject).cmp(&date_mapper(&b.subject))),
      SortMode::FavCount => l.sort_by(|b, a| a.subject.rating_count.cmp(&b.subject.rating_count)),
    };
  }
  l
}

pub trait SearchRangeList {
  fn range_contains(&self, x: u32) -> bool;
}

impl SearchRangeList for Vec<SearchRange> {
  fn range_contains(&self, x: u32) -> bool {
    self.iter().any(|year_range| {
      match year_range {
        SearchRange::Range(begin, end) => (*begin..*end).contains(&x),
        SearchRange::RangeTo(end) => (..*end).contains(&x),
        SearchRange::RangeFrom(begin) => (*begin..).contains(&x),
        SearchRange::RangeFull => true,
      }
    })
  }
}

impl <'a> DB<'a> {
  pub fn open<P>(path: P) -> Self
  where
    P: AsRef<Path>
  {
    eprintln!("* Load persistence_table");
    let persistence_table = bincode::deserialize_from::<_, PackedDatabasePersistenceTable>(std::fs::OpenOptions::new().read(true).open(path.as_ref()).unwrap()).unwrap();
    assert_eq!(persistence_table.user_id_list.len(), persistence_table.user_username_list.len());

    eprintln!("* Load map_table");
    let expected_bytes_count = persistence_table.subject_packed_list.len() * persistence_table.user_id_list.len() * core::mem::size_of::<u16>();
    let map_table_handle = {
      let f = std::fs::OpenOptions::new().read(true).open(format!("{}_mmap", path.as_ref().to_str().unwrap())).unwrap();
      if f.metadata().unwrap().len() != expected_bytes_count as u64 {
        println!("Unexpected length of map_table");
        panic!();
      }
      unsafe { memmap::Mmap::map(&f) }.unwrap()
    };
    assert_eq!(map_table_handle.len(), expected_bytes_count);
    let map_table = unsafe { core::slice::from_raw_parts(map_table_handle.as_ptr() as *const u16, expected_bytes_count / core::mem::size_of::<u16>()) };

    eprintln!("* Load finished");
    DB {
      persistence_table,
      map_table_handle,
      map_table: map_table,
    }
  }

  pub fn db_date(&self) -> (u8, u8, u16) {
    &self.map_table_handle;
    self.persistence_table.db_date
  }

  pub fn d_fac(&self) -> [f32; 2] {
    self.persistence_table.d_fac
  }

  pub fn subject_count(&self) -> usize {
    self.persistence_table.subject_packed_list.len()
  }

  pub fn subject_iter(&self) -> impl Iterator<Item = &PackedSubject> {
    self.persistence_table.subject_packed_list.iter()
  }

  pub fn get_user_id_by_username(&self, username: &str) -> Option<u32> {
    self.persistence_table.user_username_list.iter().enumerate().find(|(_, v)| v.to_lowercase() == username).map(|(i, _)| *self.persistence_table.user_id_list.get(i).unwrap())
  }

  pub fn get_user_subject_relation(&self, user_id: u32, subject_id: u32) -> Option<u16> {
    match self.persistence_table.user_id_list.binary_search(&user_id) {
      Ok(i_user) => {
        match self.persistence_table.subject_packed_list.binary_search_by_key(&subject_id, |x| x.subject_id) {
          Ok(i_subject) => {
            let n_subject = self.persistence_table.subject_packed_list.len();
            Some(*self.map_table.get(i_user * n_subject + i_subject).unwrap())
          },
          _ => None,
        }
      },
      _ => None,
    }
  }

  pub fn get_tag_id_by_name(&self, name: &str) -> Option<u32> {
    match self.persistence_table.tag_name_list.binary_search_by_key(&name, |x| x.as_str()) {
      Ok(i) => Some(i as u32),
      _ => None,
    }
  }

  pub fn search_by_ticket(&self, ticket: &SearchTicket) -> Vec<SearchResult> {
    self.persistence_table.subject_packed_list.iter().filter_map(|subject| {
      // r18
      match ticket.r18 {
        Some(x) => {
          if x != subject.is_r18 {
            return None;
          }
        }
        None => {}
      };
      // tag
      if !ticket.tag_list.iter().all(|tag| {
        match tag {
          Relation::Include(x) => subject.tag_list.iter().find(|(id, _)| id == x).is_some(),
          Relation::Exclude(x) => subject.tag_list.iter().find(|(id, _)| id == x).is_none(),
        }
      }) {
        return None;
      }
      // year
      if !(ticket.year_list.is_empty() || ticket.year_list.range_contains(subject.air_y as u32)) {
        return None;
      }
      // rank
      if !match &ticket.rank {
        SearchRange::Range(begin, end) => (*begin..*end).contains(&subject.rank),
        SearchRange::RangeTo(end) => (..*end).contains(&subject.rank),
        SearchRange::RangeFrom(begin) => (*begin..).contains(&subject.rank),
        SearchRange::RangeFull => true,
      } {
        return None;
      }
      // fav
      if !match &ticket.rating_count {
        SearchRange::Range(begin, end) => (*begin..*end).contains(&subject.rating_count),
        SearchRange::RangeTo(end) => (..*end).contains(&subject.rating_count),
        SearchRange::RangeFrom(begin) => (*begin..).contains(&subject.rating_count),
        SearchRange::RangeFull => true,
      } {
        return None;
      }

      // user
      let user_recommend = match ticket.for_user {
        Some(user_id) => {
          match self.get_user_subject_relation(user_id, subject.subject_id) {
            Some(relation) => relation,
            None => { return None; }
          }
        }
        None => { 65535 }
      };

      // keyword
      let mut keyword_relative = 0.0;
      if !ticket.keyword_list.is_empty() {
        let name_cache = subject.name.chars().collect::<Vec<_>>();
        let name_cn_cache = subject.name.chars().collect::<Vec<_>>();
        for kwd in ticket.keyword_list.iter() {
          match kwd {
            SearchMode::ExactMatch(kwd_relation) => {
              match kwd_relation {
                Relation::Include(x) => {
                  if !(match_keyword_exact(x.as_str(), subject.name_cn.as_str()) || match_keyword_exact(x.as_str(), subject.name.as_str())) {
                    return None;
                  }
                }
                Relation::Exclude(x) => {
                  if match_keyword_exact(x.as_str(), subject.name_cn.as_str()) || match_keyword_exact(x.as_str(), subject.name.as_str()) {
                    return None;
                  }
                }
              }
            },
            SearchMode::PartialMatch(kwd_relation) => {
              match kwd_relation {
                Relation::Include(x) => {
                  if match_keyword_exact(x.as_str(), subject.name_cn.as_str()) || match_keyword_exact(x.as_str(), subject.name.as_str()) {
                    keyword_relative += 1.0;
                  }
                  let cache = x.chars().collect::<Vec<_>>();
                  keyword_relative += match_keyword_partial(&cache, &name_cn_cache).max(match_keyword_partial(&cache, &name_cache));
                }
                Relation::Exclude(x) => {
                  if match_keyword_exact(x.as_str(), subject.name_cn.as_str()) || match_keyword_exact(x.as_str(), subject.name.as_str()) {
                    keyword_relative -= 1.0;
                  }
                  let cache = x.chars().collect::<Vec<_>>();
                  keyword_relative -= match_keyword_partial(&cache, &name_cn_cache).max(match_keyword_partial(&cache, &name_cache));
                }
              }
            },
          }
        }
        keyword_relative /= ticket.keyword_list.len() as f32;
      }

      Some(SearchResult {
        subject,
        keyword_relative,
        user_recommend,
      })
    }).collect()
  }
}