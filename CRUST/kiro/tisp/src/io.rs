use crate::tisp::*;
use std::io::Write;

pub fn count_parens(s: &str, _len: i32) -> i32 {
    let (mut p, mut b, mut c) = (0i32, 0i32, 0i32);
    for ch in s.chars() {
        match ch {
            '(' => p += 1, ')' => p -= 1,
            '[' => b += 1, ']' => b -= 1,
            '{' => c += 1, '}' => c -= 1,
            _ => {}
        }
    }
    if p != 0 { p } else if b != 0 { b } else { c }
}

pub fn read_file(fname: &str) -> String {
    std::fs::read_to_string(fname).unwrap_or_default()
}

pub fn prim_write(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if tsp_arg_min_check_pub(&args, "write", 2).is_none() { return mk_error(); }
    let dest = car_pub(&args);
    let append = !nilp_pub(car_pub(cdr_pub(&args)));
    let is_stdout = dest.t == TspType::TspSym && sym_str_pub(dest) == "stdout";
    let is_stderr = dest.t == TspType::TspSym && sym_str_pub(dest) == "stderr";
    if !is_stdout && !is_stderr && dest.t == TspType::TspSym {
        eprintln!("; tisp: error: write: expected file name as string, or symbol stdout/stderr");
        return mk_error();
    }
    if !is_stdout && !is_stderr && dest.t != TspType::TspStr {
        eprintln!("; tisp: error: write: expected file name as string, received {}", tsp_type_str(dest.t));
        return mk_error();
    }
    let mut cur = cdr_pub(cdr_pub(&args));
    if is_stdout {
        while !nilp_pub(cur) {
            print!("{}", val_to_string_pub(car_pub(cur)));
            cur = cdr_pub(cur);
        }
        let _ = std::io::stdout().flush();
    } else if is_stderr {
        while !nilp_pub(cur) {
            eprint!("{}", val_to_string_pub(car_pub(cur)));
            cur = cdr_pub(cur);
        }
        let _ = std::io::stderr().flush();
    } else {
        let fname = sym_str_pub(dest);
        let mut f = if append {
            std::fs::OpenOptions::new().append(true).create(true).open(fname)
        } else {
            std::fs::File::create(fname).map(|f| f)
        };
        match f {
            Ok(ref mut file) => {
                while !nilp_pub(cur) {
                    let _ = write!(file, "{}", val_to_string_pub(car_pub(cur)));
                    cur = cdr_pub(cur);
                }
            }
            Err(_) => {
                eprintln!("; tisp: error: write: could not load file '{}'", fname);
                return mk_error();
            }
        }
    }
    mk_none_pub()
}

pub fn prim_read(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if tsp_arg_max_check_pub(&args, "read", 1).is_none() { return mk_error(); }
    if tsp_lstlen(&args) == 1 {
        if tsp_arg_type_check_pub(car_pub(&args), "read", TspType::TspStr as u32).is_none() { return mk_error(); }
        let fname = sym_str_pub(car_pub(&args));
        match std::fs::read_to_string(fname) {
            Ok(contents) => mk_str(st, &contents),
            Err(_) => mk_nil_pub(),
        }
    } else {
        // Read from stdin
        let mut input = String::new();
        match std::io::stdin().read_line(&mut input) {
            Ok(_) => mk_str(st, &input),
            Err(_) => mk_nil_pub(),
        }
    }
}

pub fn prim_parse(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if tsp_arg_num_check_pub(&args, "parse", 1).is_none() { return mk_error(); }
    let expr = car_pub(&args);
    if nilp_pub(expr) { return mk_sym(st, "quit"); }
    if tsp_arg_type_check_pub(expr, "parse", TspType::TspStr as u32).is_none() { return mk_error(); }
    let s = sym_str_pub(expr).to_string();
    let old_file = std::mem::replace(&mut st.file, s);
    let old_filec = std::mem::replace(&mut st.filec, 0);
    let do_sym = mk_sym(st, "do");
    let mut items = vec![do_sym];
    while st.filec < st.file.len() {
        if let Some(e) = tisp_read_line(st, 0) {
            items.push(e);
        } else { break; }
    }
    st.file = old_file;
    st.filec = old_filec;
    if items.len() == 2 {
        return items.into_iter().nth(1).unwrap();
    }
    mk_list(st, items.len() as i32, items)
}

pub fn prim_load(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    if tsp_arg_num_check_pub(&args, "load", 1).is_none() { return mk_error(); }
    let tib = car_pub(&args);
    if tsp_arg_type_check_pub(tib, "load", TspType::TspStr as u32).is_none() { return mk_error(); }
    let name = sym_str_pub(tib).to_string();
    let paths = ["/usr/local/lib/tisp/pkgs/", "/usr/lib/tisp/pkgs/", "./"];
    for path in &paths {
        let fname = format!("{}{}.tsp", path, name);
        if std::path::Path::new(&fname).exists() {
            let file = read_file(&fname);
            let sym = mk_sym(st, &file);
            let pair = mk_pair(sym, mk_nil_pub());
            let body = prim_parse(st, env, pair);
            if is_error(&body) { return mk_error(); }
            match tisp_eval_body_pub(st, env, body) {
                Some(_) => return mk_none_pub(),
                None => return mk_error(),
            }
        }
    }
    eprintln!("; tisp: error: load: could not load '{}'", name);
    mk_error()
}

pub fn tib_env_io(st: &mut Tsp) {
    tisp_env_add(st, "write", mk_prim(TspType::TspPrim, prim_write, "write"));
    tisp_env_add(st, "read", mk_prim(TspType::TspPrim, prim_read, "read"));
    tisp_env_add(st, "parse", mk_prim(TspType::TspPrim, prim_parse, "parse"));
    tisp_env_add(st, "load", mk_prim(TspType::TspPrim, prim_load, "load"));
}
