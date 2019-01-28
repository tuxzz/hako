(()=>{
  let search_box = document.getElementById("WorkSearchBox");
  let user_box = document.getElementById("WorkUserBox");
  let submit_button = document.getElementById("SubmitWorkSearchBox");
  let year_box = document.getElementById("year_box");
  let tag_box = document.getElementById("tag_box");
  let r18_box = document.getElementById("r18_box");
  let fav_box = document.getElementById("fav_box");
  let sort_mode = document.getElementById("WorkSortMode");
  let pager_submit = document.getElementById("PagerSubmit");
  let search_panel = document.getElementById("WorkSearchPanel");
  let pager_value = document.getElementById("PagerValue");
  const ord_map_list = ["ar", "dr", "al", "dl", "ak", "dk", "ad", "dd", "af", "df"];

  function bind_year(e) {
    let l = e.getElementsByClassName("WorkFilterItem");
    for(let i = 0; i < l.length; ++i) {
      l[i].addEventListener("click", function(ev){
        if(this.classList.contains("Positive"))
          this.classList.remove("Positive");
        else
          this.classList.add("Positive");
      });
    }
  }

  function bind_tag(e) {
    let l = e.getElementsByClassName("WorkFilterItem");
    for(let i = 0; i < l.length; ++i) {
      l[i].addEventListener("click", function(ev){
        if(this.classList.contains("Positive")) {
          this.classList.remove("Positive");
          this.classList.add("Negative");
        }
        else if(this.classList.contains("Negative"))
          this.classList.remove("Negative");
        else
          this.classList.add("Positive");
      });
    }
  }

  function bind_single3(e) {
    let l = e.getElementsByClassName("WorkFilterItem");
    for(let i = 0; i < l.length; ++i) {
      l[i].addEventListener("click", function(ev){
        if(this.classList.contains("Positive")) {
          this.classList.remove("Positive");
          this.classList.add("Negative");
          this.innerText = "否";
        }
        else if(this.classList.contains("Negative")) {
          this.classList.remove("Negative");
          this.innerText = "任意";
        }
        else {
          this.classList.add("Positive");
          this.innerText = "是";
        }
      });
    }
  }

  bind_year(year_box);
  bind_tag(tag_box);
  bind_single3(r18_box);
  bind_single3(fav_box);

  if(pager_submit) {
    pager_submit.addEventListener("click", function(ev){
      let v = parseInt(pager_value.value);
      let min = parseInt(pager_value.min);
      let max = parseInt(pager_value.max);
      if(v >= min && v <= max) {
        window.location = search_panel.dataset.origBase + "/" + ord_map_list[parseInt(sort_mode.dataset.ord)] + "/" + (v - 1) * 25;
        ev.stopPropagation();
        ev.preventDefault();
      }
    });
  }

  submit_button.addEventListener("click", function(ev){
    let kwd_list = [];
    let l = search_box.value.trim().split(" ");
    let i;
    while((i = l.indexOf("")) != -1)
      l.splice(i);
    for(let i = 0; i < l.length; ++i) {
      let x = l[i];
      if(x.substr(0, 2) == "-*" || x.substr(0, 2) == "*-")
        kwd_list.push([3, x.substr(2)])
      else if(x[0] == "-")
        kwd_list.push([1, x.substr(1)])
      else if(x[0] == "*")
        kwd_list.push([2, x.substr(1)])
      else
        kwd_list.push([0, x])
    }

    let tag_list = [];
    {
      let l = tag_box.getElementsByClassName("WorkFilterItem");
      for(let i = 0; i < l.length; ++i) {
        let x = l[i];
        if(x.classList.contains("Positive"))
          tag_list.push([1, x.innerText]);
        else if(x.classList.contains("Negative"))
          tag_list.push([0, x.innerText]);
      }
    }
    
    function translate_year(s) {
      if(s == "Now") {
        let y = 1900 + new Date().getYear();
        return [y, y + 1]
      }
      let l;
      if(l = s.match(/^\((\d{4})\.\.(\d{4})\)$/)) {
        let a = parseInt(l[1]);
        let b = parseInt(l[2]);
        return [a, b];
      }
      else if(l = s.match(/^\((\d{4})\.\.\)$/)) {
        let a = parseInt(l[1]);
        return [a, null];
      }
      else if(l = s.match(/^\(\.\.(\d{4})\)$/)) {
        let b = parseInt(l[1]);
        return [null, b];
      }
      else {
        let a = parseInt(s);
        return [a, a + 1];
      }
    }

    let year_list = [];
    {
      let l = year_box.getElementsByClassName("WorkFilterItem");
      for(let i = 0; i < l.length; ++i) {
        let x = l[i];
        if(x.classList.contains("Positive"))
          year_list.push(translate_year(x.innerText));
      }
    }
    
    let u = user_box.value;
    if(!u)
      u = null;
    
    let r18_mode = 3;
    {
      let x = r18_box.getElementsByClassName("WorkFilterItem")[0];
      if(x.classList.contains("Positive"))
        r18_mode = 2;
      else if(x.classList.contains("Negative"))
        r18_mode = 1;
    }

    let exclude_fav_mode;
    {
      let x = fav_box.getElementsByClassName("WorkFilterItem")[0];
      if(x.classList.contains("Positive"))
        exclude_fav_mode = 2;
      else if(x.classList.contains("Negative"))
        exclude_fav_mode = 1;
      else
        exclude_fav_mode = 3;
    }
    
    let s = JSON.stringify([kwd_list, tag_list, year_list, u, r18_mode, exclude_fav_mode]);
    if(s.length > 127){
      alert("查询字符串过长，请考虑缩短关键词长度");
      return;
    }
    if(s == "[[],[],[],null,3,1]")
      window.location = "/dr/0";
    else {
      let sort_mode = kwd_list.length > 0 ? "dl" : "dr";
      window.location = "/search/" + encodeURI(s) + "/" + sort_mode + "/0";
    }

    ev.stopPropagation();
    ev.preventDefault();
  })
})();