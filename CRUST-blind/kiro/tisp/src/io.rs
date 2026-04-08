use crate::tisp::*;
use std::io::{Read, Write};

fn tsp_arg_check(args: &Val, name: &str, n: i32) {
    let len = tsp_lstlen(args);
    if n > -1 && len != n {
        eprintln!("; tisp: error: {}: expected {} argument{}, received {}",
            name, n, if n > 1 { "s" } else { "" }, len);
    }
}
fn tsp_arg_min_check(args: &Val, name: &str, n: i32) {
    let len = tsp_lstlen(args);
    if len < n {
        eprintln!("; tisp: error: {}: expected at least {} argument{}, received {}",
            name, n, if n > 1 { "s" } else { "" }, len);
    }
}
fn tsp_arg_max_check(args: &Val, name: &str, n: i32) {
    let len = tsp_lstlen(args);
    if len > n {
        eprintln!("; tisp: error: {}: expected at no more than {} argument{}, received {}",
            name, n, if n > 1 { "s" } else { "" }, len);
    }
}
fn tsp_type_check(v: &Val, name: &str, type_mask: u32) {
    if (v.t as u32) & type_mask == 0 {
        eprintln!("; tisp: error: {}: expected {}, received {}",
            name, tsp_type_str_mask(type_mask), tsp_type_str(v.t));
    }
}
fn tsp_type_str_mask(t: u32) -> &'static str {
    if t == TspType::TspStr as u32 { return "Str"; }
    if t == TspType::TspSym as u32 { return "Sym"; }
    "Invalid"
}

pub fn count_parens(s: &str, len: i32) -> i32 {
    let mut pcount = 0i32;
    let mut bcount = 0i32;
    let mut ccount = 0i32;
    for (i, c) in s.chars().enumerate() {
        if i >= len as usize { break; }
        match c {
            '(' => pcount += 1, ')' => pcount -= 1,
            '[' => bcount += 1, ']' => bcount -= 1,
            '{' => ccount += 1, '}' => ccount -= 1,
            _ => {}
        }
    }
    if pcount != 0 { return pcount; }
    if bcount != 0 { return bcount; }
    ccount
}

pub fn read_file(fname: &str) -> String {
    if fname.is_empty() {
        // Read from stdin
        let mut buf = String::new();
        std::io::stdin().read_to_string(&mut buf).unwrap_or(0);
        return buf;
    }
    match std::fs::read_to_string(fname) {
        Ok(s) => s,
        Err(_) => {
            eprintln!("; tisp: error: could not find file '{}'", fname);
            String::new()
        }
    }
}

pub fn prim_write(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    tsp_arg_min_check(&args, "write", 2);
    let first = car_ref(&args);
    let second = car_ref(cdr_ref(&args));
    let append = !nilp(second);

    enum Target { Stdout, Stderr, File(String, bool) }
    let target = if first.t == TspType::TspSym {
        let s = sym_str(first);
        if s == "stdout" { Target::Stdout }
        else if s == "stderr" { Target::Stderr }
        else {
            eprintln!("; tisp: error: write: expected file name as string, or symbol stdout/stderr");
            return mk_val(TspType::TspNone);
        }
    } else if first.t == TspType::TspStr {
        Target::File(sym_str(first).to_string(), append)
    } else {
        eprintln!("; tisp: error: write: expected file name as string, received {}", tsp_type_str(first.t));
        return mk_val(TspType::TspNone);
    };

    // Collect output
    let mut output = Vec::new();
    let mut cur = cdr_ref(cdr_ref(&args));
    while !nilp(cur) {
        tisp_print(&mut output, car_ref(cur));
        cur = cdr_ref(cur);
    }

    match target {
        Target::Stdout => {
            std::io::stdout().write_all(&output).ok();
            std::io::stdout().flush().ok();
        }
        Target::Stderr => {
            std::io::stderr().write_all(&output).ok();
            std::io::stderr().flush().ok();
        }
        Target::File(path, append) => {
            let file = if append {
                std::fs::OpenOptions::new().append(true).create(true).open(&path)
            } else {
                std::fs::File::create(&path)
            };
            match file {
                Ok(mut f) => { f.write_all(&output).ok(); }
                Err(_) => {
                    eprintln!("; tisp: error: write: could not load file '{}'", path);
                    return mk_val(TspType::TspNone);
                }
            }
        }
    }
    val_clone(&st.none)
}

pub fn prim_read(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    tsp_arg_max_check(&args, "read", 1);
    let fname = if tsp_lstlen(&args) == 1 {
        tsp_type_check(car_ref(&args), "read", TspType::TspStr as u32);
        sym_str(car_ref(&args)).to_string()
    } else {
        String::new()
    };
    let file = read_file(&fname);
    if file.is_empty() && !fname.is_empty() {
        return val_clone(&st.nil);
    }
    mk_str(st, &file).unwrap()
}

pub fn prim_parse(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    tsp_arg_check(&args, "parse", 1);
    let expr = car_ref(&args);
    if nilp(expr) {
        return mk_sym(st, "quit").unwrap();
    }
    tsp_type_check(expr, "parse", TspType::TspStr as u32);
    let old_file = st.file.clone();
    let old_filec = st.filec;
    st.file = sym_str(expr).to_string();
    st.filec = 0;

    let do_sym = mk_sym(st, "do").unwrap();
    let mut items: Vec<Val> = vec![do_sym];
    while st.filec < st.file.len() {
        if let Some(e) = tisp_read_line(st, 0) {
            items.push(e);
        } else {
            break;
        }
    }
    st.file = old_file;
    st.filec = old_filec;

    if items.len() == 2 {
        // Only 1 expression parsed, return just it
        return items.pop().unwrap();
    }
    // Build list
    let mut result = mk_nil_val();
    for item in items.into_iter().rev() {
        result = mk_pair_val(item, result);
    }
    result
}

pub fn prim_load(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    tsp_arg_check(&args, "load", 1);
    let tib = car_ref(&args);
    tsp_type_check(tib, "load", TspType::TspStr as u32);
    let name = sym_str(tib).to_string();

    let paths = ["/usr/local/lib/tisp/pkgs/", "/usr/lib/tisp/pkgs/", "./"];
    for path in &paths {
        let fname = format!("{}{}.tsp", path, name);
        if std::path::Path::new(&fname).exists() {
            let file = read_file(&fname);
            let file_sym = mk_sym(st, &file).unwrap();
            let parse_args = mk_pair_val(file_sym, mk_nil_val());
            let body = prim_parse(st, env, parse_args);
            tisp_eval_body(st, env, body);
            return val_clone(&st.none);
        }
    }

    eprintln!("; tisp: error: load: could not load '{}'", name);
    mk_val(TspType::TspNone)
}

pub fn tib_env_io(st: &mut Tsp) {
    tisp_env_add(st, "write", mk_prim(TspType::TspPrim, prim_write, "write").unwrap());
    tisp_env_add(st, "read", mk_prim(TspType::TspPrim, prim_read, "read").unwrap());
    tisp_env_add(st, "parse", mk_prim(TspType::TspPrim, prim_parse, "parse").unwrap());
    tisp_env_add(st, "load", mk_prim(TspType::TspPrim, prim_load, "load").unwrap());
}
