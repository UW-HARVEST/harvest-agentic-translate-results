use crate::tisp::{
    mk_pair, mk_prim, mk_str, mk_sym, rec_clone, stub_prim, tisp_env_add, tisp_eval_body,
    tisp_read_line, tsp_lstlen, val_clone, Rec, Tsp, TspType, Val, ValUnion,
};

fn car(v: &Val) -> Val {
    if let ValUnion::P { car, .. } = &v.v {
        val_clone(car)
    } else {
        Val {
            t: TspType::TspNil,
            v: ValUnion::N { num: 0.0, den: 1.0 },
        }
    }
}

fn cdr(v: &Val) -> Val {
    if let ValUnion::P { cdr, .. } = &v.v {
        val_clone(cdr)
    } else {
        Val {
            t: TspType::TspNil,
            v: ValUnion::N { num: 0.0, den: 1.0 },
        }
    }
}

fn nilp(v: &Val) -> bool {
    matches!(v.t, TspType::TspNil)
}

pub fn count_parens(s: &str, len: i32) -> i32 {
    let mut pcount = 0i32;
    let mut bcount = 0i32;
    let mut ccount = 0i32;
    let bytes = s.as_bytes();
    let n = (len.max(0) as usize).min(bytes.len());
    for i in 0..n {
        let c = bytes[i];
        if c == 0 {
            break;
        }
        match c {
            b'(' => pcount += 1,
            b'[' => bcount += 1,
            b'{' => ccount += 1,
            b')' => pcount -= 1,
            b']' => bcount -= 1,
            b'}' => ccount -= 1,
            _ => {}
        }
    }
    if pcount != 0 {
        return pcount;
    }
    if bcount != 0 {
        return bcount;
    }
    ccount
}

pub fn read_file(fname: &str) -> String {
    if fname.is_empty() {
        // read all of stdin
        use std::io::Read;
        let mut s = String::new();
        let _ = std::io::stdin().read_to_string(&mut s);
        return s;
    }
    std::fs::read_to_string(fname).unwrap_or_default()
}

fn print_to_string(v: &Val) -> String {
    match v.t {
        TspType::TspNone => "Void".to_string(),
        TspType::TspNil => "Nil".to_string(),
        TspType::TspInt => {
            if let ValUnion::N { num, .. } = &v.v {
                format!("{}", *num as i32)
            } else {
                String::new()
            }
        }
        TspType::TspDec => {
            if let ValUnion::N { num, .. } = &v.v {
                let mut out = format!("{}", num);
                if *num == (*num as i32) as f64 {
                    out.push_str(".0");
                }
                out
            } else {
                String::new()
            }
        }
        TspType::TspRatio => {
            if let ValUnion::N { num, den } = &v.v {
                format!("{}/{}", *num as i32, *den as i32)
            } else {
                String::new()
            }
        }
        TspType::TspStr | TspType::TspSym => {
            if let ValUnion::S(s) = &v.v {
                s.clone()
            } else {
                String::new()
            }
        }
        TspType::TspPair => {
            let mut s = String::from("(");
            if let ValUnion::P { car, cdr } = &v.v {
                s.push_str(&print_to_string(car));
                let mut cur: &Val = cdr;
                while !nilp(cur) {
                    if matches!(cur.t, TspType::TspPair) {
                        if let ValUnion::P { car: c2, cdr: c3 } = &cur.v {
                            s.push(' ');
                            s.push_str(&print_to_string(c2));
                            cur = c3;
                        } else {
                            break;
                        }
                    } else {
                        s.push_str(" . ");
                        s.push_str(&print_to_string(cur));
                        break;
                    }
                }
            }
            s.push(')');
            s
        }
        _ => String::new(),
    }
}

pub fn prim_write(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    use std::io::Write;
    if tsp_lstlen(&args) < 2 {
        return val_clone(&st.none);
    }
    let target = car(&args);
    let _append = !nilp(&car(&cdr(&args)));
    let to_stdout = match &target.v {
        ValUnion::S(s) if matches!(target.t, TspType::TspSym) => s == "stdout",
        _ => true,
    };
    let to_stderr = match &target.v {
        ValUnion::S(s) if matches!(target.t, TspType::TspSym) => s == "stderr",
        _ => false,
    };
    let mut cur = cdr(&cdr(&args));
    let mut out = String::new();
    while !nilp(&cur) {
        let item = car(&cur);
        out.push_str(&print_to_string(&item));
        cur = cdr(&cur);
    }
    if to_stderr {
        let _ = std::io::stderr().write_all(out.as_bytes());
    } else if to_stdout {
        let _ = std::io::stdout().write_all(out.as_bytes());
    } else if let ValUnion::S(fname) = &target.v {
        if matches!(target.t, TspType::TspStr) {
            let mode = if _append {
                std::fs::OpenOptions::new().append(true).create(true).open(fname)
            } else {
                std::fs::OpenOptions::new()
                    .write(true)
                    .create(true)
                    .truncate(true)
                    .open(fname)
            };
            if let Ok(mut f) = mode {
                let _ = f.write_all(out.as_bytes());
            }
        }
    }
    val_clone(&st.none)
}

pub fn prim_read(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    let len = tsp_lstlen(&args);
    let fname = if len == 1 {
        let a = car(&args);
        if let ValUnion::S(s) = &a.v {
            s.clone()
        } else {
            String::new()
        }
    } else {
        String::new()
    };
    let s = read_file(&fname);
    if s.is_empty() {
        return val_clone(&st.nil);
    }
    mk_str(st, &s).unwrap_or_else(|| val_clone(&st.none))
}

pub fn prim_parse(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    let expr = car(&args);
    if nilp(&expr) {
        return mk_sym(st, "quit").unwrap_or_else(|| val_clone(&st.none));
    }
    if !matches!(expr.t, TspType::TspStr) {
        return val_clone(&st.none);
    }
    let body_str = if let ValUnion::S(s) = &expr.v {
        s.clone()
    } else {
        return val_clone(&st.none);
    };
    let saved_file = std::mem::take(&mut st.file);
    let saved_filec = st.filec;
    st.file = body_str;
    st.filec = 0;

    let do_sym = match mk_sym(st, "do") {
        Some(v) => v,
        None => {
            st.file = saved_file;
            st.filec = saved_filec;
            return val_clone(&st.none);
        }
    };
    let mut items: Vec<Val> = Vec::new();
    while st.filec < st.file.len() {
        match tisp_read_line(st, 0) {
            Some(v) => items.push(v),
            None => break,
        }
    }
    st.file = saved_file;
    st.filec = saved_filec;

    if items.len() == 1 {
        return items.into_iter().next().unwrap();
    }

    let mut acc = val_clone(&st.nil);
    for v in items.into_iter().rev() {
        acc = mk_pair(v, acc).unwrap_or_else(|| val_clone(&st.none));
    }
    mk_pair(do_sym, acc).unwrap_or_else(|| val_clone(&st.none))
}

pub fn prim_load(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    let tib = car(&args);
    let name = if let ValUnion::S(s) = &tib.v {
        s.clone()
    } else {
        return val_clone(&st.none);
    };
    let paths = ["/usr/local/lib/tisp/pkgs/", "/usr/lib/tisp/pkgs/", "./"];
    for p in paths.iter() {
        let path = format!("{}{}.tsp", p, name);
        if std::path::Path::new(&path).exists() {
            let body_str = read_file(&path);
            let body_val = mk_str(st, &body_str).unwrap_or_else(|| val_clone(&st.none));
            let single = mk_pair(body_val, val_clone(&st.nil))
                .unwrap_or_else(|| val_clone(&st.none));
            let parsed = prim_parse(st, env, single);
            let mut env_clone = rec_clone(env);
            let _ = tisp_eval_body(st, &mut env_clone, parsed);
            return val_clone(&st.none);
        }
    }
    val_clone(&st.none)
}

pub fn tib_env_io(st: &mut Tsp) {
    let names = ["write", "read", "parse", "load"];
    for n in names.iter() {
        let v = mk_prim(TspType::TspPrim, stub_prim, n).unwrap();
        tisp_env_add(st, n, v);
    }
}
