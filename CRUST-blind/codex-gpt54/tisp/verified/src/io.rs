use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;

use crate::tisp::{
    Rec, Tsp, TspType, Val, expect_len, expect_max_len, expect_min_len, expect_type, mk_pair,
    mk_prim, mk_str, pair_car, pair_cdr, render_val, tisp_env_add, tisp_eval_body, tisp_read_line,
    val_is_nil, val_str,
};

pub fn count_parens(s: &str, len: i32) -> i32 {
    let mut pcount = 0;
    let mut bcount = 0;
    let mut ccount = 0;
    for ch in s.chars().take(len.max(0) as usize) {
        match ch {
            '(' => pcount += 1,
            '[' => bcount += 1,
            '{' => ccount += 1,
            ')' => pcount -= 1,
            ']' => bcount -= 1,
            '}' => ccount -= 1,
            _ => {}
        }
    }
    if pcount != 0 {
        pcount
    } else if bcount != 0 {
        bcount
    } else {
        ccount
    }
}

pub fn read_file(fname: &str) -> String {
    if fname.is_empty() {
        let mut buf = String::new();
        let mut stdin = std::io::stdin();
        let _ = stdin.read_to_string(&mut buf);
        return buf;
    }
    fs::read_to_string(fname).unwrap_or_default()
}

pub fn prim_write(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if !expect_min_len(st, &args, "write", 2) {
        return st.none.clone();
    }
    let target = pair_car(&args).clone();
    let append = !val_is_nil(pair_car(pair_cdr(&args)));
    let mut rendered = String::new();
    let mut cur = pair_cdr(pair_cdr(&args)).clone();
    while cur.t == TspType::TspPair {
        rendered.push_str(&render_val(pair_car(&cur)));
        cur = pair_cdr(&cur).clone();
    }
    match target.t {
        TspType::TspSym => match val_str(&target).unwrap_or_default() {
            "stdout" => {
                print!("{}", rendered);
                let _ = std::io::stdout().flush();
            }
            "stderr" => {
                eprint!("{}", rendered);
                let _ = std::io::stderr().flush();
            }
            _ => return st.none.clone(),
        },
        TspType::TspStr => {
            let fname = val_str(&target).unwrap_or_default();
            let mut opts = OpenOptions::new();
            opts.write(true).create(true);
            if append {
                opts.append(true);
            } else {
                opts.truncate(true);
            }
            if let Ok(mut file) = opts.open(fname) {
                let _ = file.write_all(rendered.as_bytes());
            } else {
                return st.none.clone();
            }
        }
        _ => return st.none.clone(),
    }
    st.none.clone()
}

pub fn prim_read(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if !expect_max_len(st, &args, "read", 1) {
        return st.nil.clone();
    }
    let fname = if crate::tisp::tsp_lstlen(&args) == 1 {
        let arg = pair_car(&args).clone();
        if !expect_type(st, &arg, "read", TspType::TspStr as u32) {
            return st.nil.clone();
        }
        val_str(&arg).unwrap_or_default().to_string()
    } else {
        String::new()
    };
    let file = read_file(&fname);
    if file.is_empty() {
        st.nil.clone()
    } else {
        mk_str(st, &file).unwrap_or_else(|| st.nil.clone())
    }
}

pub fn prim_parse(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if !expect_len(st, &args, "parse", 1) {
        return st.none.clone();
    }
    let expr = pair_car(&args).clone();
    if val_is_nil(&expr) {
        return crate::tisp::mk_sym(st, "quit").unwrap_or_else(|| st.none.clone());
    }
    if !expect_type(st, &expr, "parse", TspType::TspStr as u32) {
        return st.none.clone();
    }
    let old_file = st.file.clone();
    let old_filec = st.filec;
    st.file = val_str(&expr).unwrap_or_default().to_string();
    st.filec = 0;
    let mut forms = vec![crate::tisp::mk_sym(st, "do").unwrap_or_else(|| st.none.clone())];
    while st.filec < st.file.len() {
        if let Some(v) = tisp_read_line(st, 0) {
            forms.push(v);
        } else {
            break;
        }
    }
    st.file = old_file;
    st.filec = old_filec;
    if forms.len() == 2 {
        forms.pop().unwrap_or_else(|| st.none.clone())
    } else {
        crate::tisp::mk_list(st, forms.len() as i32, forms).unwrap_or_else(|| st.none.clone())
    }
}

pub fn prim_load(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    if !expect_len(st, &args, "load", 1) {
        return st.none.clone();
    }
    let lib = pair_car(&args).clone();
    if !expect_type(st, &lib, "load", TspType::TspStr as u32) {
        return st.none.clone();
    }
    let name = val_str(&lib).unwrap_or_default();
    let candidates = [
        format!("{name}.tsp"),
        format!("c_src/{name}.tsp"),
        format!("c_src/tib/{name}.tsp"),
    ];
    for candidate in candidates {
        if Path::new(&candidate).is_file() {
            let file = read_file(&candidate);
            let file_str = mk_str(st, &file).unwrap_or_else(|| st.none.clone());
            let parse_args = mk_pair(file_str, st.nil.clone()).unwrap_or_else(|| st.none.clone());
            let parsed = prim_parse(st, env, parse_args);
            let _ = tisp_eval_body(st, env, parsed);
            return st.none.clone();
        }
    }
    st.none.clone()
}

pub fn tib_env_io(st: &mut Tsp) {
    tisp_env_add(st, "write", mk_prim(TspType::TspPrim, prim_write, "write").unwrap());
    tisp_env_add(st, "read", mk_prim(TspType::TspPrim, prim_read, "read").unwrap());
    tisp_env_add(st, "parse", mk_prim(TspType::TspPrim, prim_parse, "parse").unwrap());
    tisp_env_add(st, "load", mk_prim(TspType::TspPrim, prim_load, "load").unwrap());
}
