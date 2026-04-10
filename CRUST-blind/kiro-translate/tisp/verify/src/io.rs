use crate::tisp::*;
use std::io::{Read, Write};

fn tsp_arg_num_check(args: &Val, name: &str, nargs: i32) -> bool {
    if nargs > -1 && tsp_lstlen(args) != nargs {
        eprintln!("; tisp: error: {}: expected {} argument{}, received {}",
            name, nargs, if nargs > 1 { "s" } else { "" }, tsp_lstlen(args));
        false
    } else { true }
}

fn tsp_arg_min_check(args: &Val, name: &str, nargs: i32) -> bool {
    if tsp_lstlen(args) < nargs {
        eprintln!("; tisp: error: {}: expected at least {} argument{}, received {}",
            name, nargs, if nargs > 1 { "s" } else { "" }, tsp_lstlen(args));
        false
    } else { true }
}

fn tsp_arg_max_check(args: &Val, name: &str, nargs: i32) -> bool {
    if tsp_lstlen(args) > nargs {
        eprintln!("; tisp: error: {}: expected at no more than {} argument{}, received {}",
            name, nargs, if nargs > 1 { "s" } else { "" }, tsp_lstlen(args));
        false
    } else { true }
}

fn tsp_arg_type_check(arg: &Val, name: &str, type_bits: u32) -> bool {
    if (arg.t as u32) & type_bits == 0 {
        eprintln!("; tisp: error: {}: expected {}, received {}",
            name, tsp_type_str_bits(type_bits), tsp_type_str(arg.t));
        false
    } else { true }
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
        let _ = std::io::stdin().read_to_string(&mut buf);
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
    if !tsp_arg_min_check(&args, "write", 2) { return mk_err(); }

    let first = car(&args);
    let second = car(cdr(&args));
    let append = !nilp(second);

    enum Target { Stdout, Stderr, File(String, bool) }

    let target = if first.t == TspType::TspSym {
        let s = vs(first);
        if s == "stdout" { Target::Stdout }
        else if s == "stderr" { Target::Stderr }
        else {
            eprintln!("; tisp: error: write: expected file name as string, or symbol stdout/stderr");
            return mk_err();
        }
    } else if first.t == TspType::TspStr {
        Target::File(vs(first).to_string(), append)
    } else {
        eprintln!("; tisp: error: write: expected file name as string, received {}", tsp_type_str(first.t));
        return mk_err();
    };

    // Get args after first two
    let mut cur = cdr(cdr(&args));
    match target {
        Target::Stdout => {
            let mut out = std::io::stdout();
            while !nilp(cur) {
                tisp_print(&mut out, car(cur));
                cur = cdr(cur);
            }
            let _ = out.flush();
        }
        Target::Stderr => {
            let mut out = std::io::stderr();
            while !nilp(cur) {
                tisp_print(&mut out, car(cur));
                cur = cdr(cur);
            }
            let _ = out.flush();
        }
        Target::File(fname, append) => {
            let file = if append {
                std::fs::OpenOptions::new().append(true).create(true).open(&fname)
            } else {
                std::fs::File::create(&fname)
            };
            match file {
                Ok(mut f) => {
                    while !nilp(cur) {
                        tisp_print(&mut f, car(cur));
                        cur = cdr(cur);
                    }
                }
                Err(_) => {
                    eprintln!("; tisp: error: write: could not load file '{}'", fname);
                    return mk_err();
                }
            }
        }
    }
    clone_val(&st.none)
}

pub fn prim_read(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if !tsp_arg_max_check(&args, "read", 1) { return mk_err(); }
    let fname = if tsp_lstlen(&args) == 1 {
        if !tsp_arg_type_check(car(&args), "read", TspType::TspStr as u32) { return mk_err(); }
        vs(car(&args)).to_string()
    } else {
        String::new()
    };
    let file = read_file(&fname);
    if file.is_empty() && !fname.is_empty() {
        return clone_val(&st.nil);
    }
    mk_str(st, &file).unwrap_or_else(|| mk_err())
}

pub fn prim_parse(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if !tsp_arg_num_check(&args, "parse", 1) { return mk_err(); }
    let expr = car(&args);
    if nilp(expr) {
        return mk_sym_val(st, "quit");
    }
    if !tsp_arg_type_check(expr, "parse", TspType::TspStr as u32) { return mk_err(); }

    let old_file = st.file.clone();
    let old_filec = st.filec;
    st.file = vs(expr).to_string();
    st.filec = 0;

    let do_sym = mk_sym_val(st, "do");
    let nil = clone_val(&st.nil);
    let mut elements: Vec<Val> = vec![do_sym];

    while st.filec < st.file.len() {
        if let Some(c) = st.file.as_bytes().get(st.filec) {
            if *c == 0 { break; }
        }
        match tisp_read_line(st, 0) {
            Some(e) => elements.push(e),
            None => break,
        }
    }

    st.file = old_file;
    st.filec = old_filec;

    if elements.len() == 2 {
        // Only 1 expression parsed (plus "do"), return just it
        return elements.pop().unwrap();
    }

    // Build (do expr1 expr2 ...)
    let mut result = nil;
    for elem in elements.into_iter().rev() {
        result = mk_pair(elem, result).unwrap();
    }
    result
}

pub fn prim_load(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    if !tsp_arg_num_check(&args, "load", 1) { return mk_err(); }
    let tib = car(&args);
    if !tsp_arg_type_check(tib, "load", TspType::TspStr as u32) { return mk_err(); }
    let tib_name = vs(tib).to_string();

    let paths = ["/usr/local/lib/tisp/pkgs/", "/usr/lib/tisp/pkgs/", "./"];
    for path in &paths {
        let fname = format!("{}{}.tsp", path, tib_name);
        if std::path::Path::new(&fname).exists() {
            let file = read_file(&fname);
            if file.is_empty() { continue; }
            let file_sym = mk_sym_val(st, &file);
            let nil = clone_val(&st.nil);
            let parse_args = mk_pair(file_sym, nil).unwrap();
            let body = prim_parse(st, env, parse_args);
            if is_err_val(&body) { return body; }
            match tisp_eval_body(st, env, body) {
                Some(_) => {}
                None => {}
            }
            return clone_val(&st.none);
        }
    }

    // Dynamic library loading not supported in pure Rust
    eprintln!("; tisp: error: load: could not load '{}'", tib_name);
    mk_err()
}

pub fn tib_env_io(st: &mut Tsp) {
    tisp_env_add(st, "write", mk_prim(TspType::TspPrim, prim_write, "write").unwrap());
    tisp_env_add(st, "read", mk_prim(TspType::TspPrim, prim_read, "read").unwrap());
    tisp_env_add(st, "parse", mk_prim(TspType::TspPrim, prim_parse, "parse").unwrap());
    tisp_env_add(st, "load", mk_prim(TspType::TspPrim, prim_load, "load").unwrap());
}
