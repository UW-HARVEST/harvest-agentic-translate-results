use crate::tisp::{
    mk_pair, mk_str, mk_sym, rec_add, tisp_eval_body, tisp_eval_with_env, tisp_read_line,
    Rec, Tsp, TspType, Val, ValUnion,
};
use std::io::Read;

fn make_none() -> Val {
    Val { t: TspType::TspNone, v: ValUnion::S(String::new()) }
}

fn val_str_ref(v: &Val) -> &str {
    if let ValUnion::S(s) = &v.v { s.as_str() } else { "" }
}

fn nilp(v: &Val) -> bool {
    matches!(v.t, TspType::TspNil)
}

fn car_of(v: &Val) -> Option<&Val> {
    if let ValUnion::P { car, .. } = &v.v { Some(car.as_ref()) } else { None }
}

fn cdr_of(v: &Val) -> Option<&Val> {
    if let ValUnion::P { cdr, .. } = &v.v { Some(cdr.as_ref()) } else { None }
}

fn cddr_of(v: &Val) -> Option<&Val> {
    cdr_of(v).and_then(cdr_of)
}

pub fn count_parens(s: &str, len: i32) -> i32 {
    let mut pcount = 0;
    let mut bcount = 0;
    let mut ccount = 0;
    let bytes = s.as_bytes();
    let limit = (len as usize).min(bytes.len());
    for i in 0..limit {
        if bytes[i] == 0 {
            break;
        }
        match bytes[i] {
            b'(' => pcount += 1,
            b'[' => bcount += 1,
            b'{' => ccount += 1,
            b')' => pcount -= 1,
            b']' => bcount -= 1,
            b'}' => ccount -= 1,
            _ => {}
        }
    }
    if pcount != 0 { return pcount; }
    if bcount != 0 { return bcount; }
    ccount
}

pub fn read_file(fname: &str) -> String {
    if fname.is_empty() {
        // read from stdin until parens balance or EOF
        let mut buf = String::new();
        let mut total = String::new();
        let stdin = std::io::stdin();
        let mut handle = stdin.lock();
        loop {
            buf.clear();
            let n = match handle.read_to_string(&mut buf) {
                Ok(n) => n,
                Err(_) => break,
            };
            if n == 0 { break; }
            total.push_str(&buf);
            let parens = count_parens(&buf, buf.len() as i32);
            if parens <= 0 { break; }
        }
        total
    } else {
        std::fs::read_to_string(fname).unwrap_or_default()
    }
}

pub fn prim_write(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    use std::io::Write;
    let first = match car_of(&args) { Some(v) => v.clone(), None => return make_none() };
    let _append = match cdr_of(&args).and_then(car_of) {
        Some(v) => !nilp(v),
        None => false,
    };
    let to_stderr = if matches!(first.t, TspType::TspSym) {
        val_str_ref(&first) == "stderr"
    } else {
        false
    };
    let to_stdout = if matches!(first.t, TspType::TspSym) {
        val_str_ref(&first) == "stdout"
    } else {
        false
    };
    let rest = cddr_of(&args).cloned().unwrap_or_else(|| st.nil.clone());
    let mut text = String::new();
    let mut cur = &rest;
    while matches!(cur.t, TspType::TspPair) {
        if let Some(v) = car_of(cur) {
            text.push_str(&print_val(v));
        }
        cur = match cdr_of(cur) { Some(c) => c, None => break };
    }
    if to_stderr {
        let _ = write!(std::io::stderr(), "{}", text);
        let _ = std::io::stderr().flush();
    } else if to_stdout {
        let _ = write!(std::io::stdout(), "{}", text);
        let _ = std::io::stdout().flush();
    } else if matches!(first.t, TspType::TspStr) {
        let path = val_str_ref(&first).to_string();
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .append(_append)
            .truncate(!_append)
            .open(&path)
        {
            let _ = f.write_all(text.as_bytes());
        }
    }
    st.none.clone()
}

fn print_val(v: &Val) -> String {
    match v.t {
        TspType::TspNone => "Void".to_string(),
        TspType::TspNil => "Nil".to_string(),
        TspType::TspInt => {
            if let ValUnion::N { num, .. } = &v.v {
                format!("{}", *num as i64)
            } else { String::new() }
        }
        TspType::TspDec => {
            if let ValUnion::N { num, .. } = &v.v {
                let n = *num;
                let s = format!("{:.15}", n);
                let trimmed = s.trim_end_matches('0').trim_end_matches('.').to_string();
                if trimmed.contains('.') { trimmed } else { format!("{}.0", trimmed) }
            } else { String::new() }
        }
        TspType::TspRatio => {
            if let ValUnion::N { num, den } = &v.v {
                format!("{}/{}", *num as i64, *den as i64)
            } else { String::new() }
        }
        TspType::TspStr | TspType::TspSym => val_str_ref(v).to_string(),
        _ => String::new(),
    }
}

pub fn prim_read(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    let fname = if matches!(args.t, TspType::TspPair) {
        if let Some(v) = car_of(&args) {
            val_str_ref(v).to_string()
        } else {
            String::new()
        }
    } else {
        String::new()
    };
    let contents = read_file(&fname);
    if contents.is_empty() && !fname.is_empty() {
        return st.nil.clone();
    }
    mk_str(st, &contents).unwrap_or_else(make_none)
}

pub fn prim_parse(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    let expr = match car_of(&args) { Some(v) => v.clone(), None => return st.none.clone() };
    if nilp(&expr) {
        return mk_sym(st, "quit").unwrap_or_else(make_none);
    }
    if !matches!(expr.t, TspType::TspStr) {
        return st.none.clone();
    }
    let saved_file = std::mem::take(&mut st.file);
    let saved_filec = st.filec;
    let s = val_str_ref(&expr).to_string();
    st.file = s;
    st.filec = 0;

    let mut items: Vec<Val> = Vec::new();
    while st.filec < st.file.len() && st.file.as_bytes()[st.filec] != 0 {
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
    let do_sym = mk_sym(st, "do").unwrap_or_else(make_none);
    let mut result = st.nil.clone();
    for v in items.into_iter().rev() {
        result = mk_pair(v, result).unwrap_or_else(make_none);
    }
    mk_pair(do_sym, result).unwrap_or_else(make_none)
}

pub fn prim_load(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    let tib = match car_of(&args) { Some(v) => v.clone(), None => return st.none.clone() };
    if !matches!(tib.t, TspType::TspStr) {
        return st.none.clone();
    }
    let name = val_str_ref(&tib).to_string();
    let candidates = [
        format!("/usr/local/lib/tisp/pkgs/{}.tsp", name),
        format!("/usr/lib/tisp/pkgs/{}.tsp", name),
        format!("./{}.tsp", name),
    ];
    for path in &candidates {
        if std::path::Path::new(path).exists() {
            let contents = std::fs::read_to_string(path).unwrap_or_default();
            let s = mk_str(st, &contents).unwrap_or_else(make_none);
            let arg = mk_pair(s, st.nil.clone()).unwrap_or_else(make_none);
            let body = prim_parse(st, env, arg);
            let _ = tisp_eval_body(st, env, body);
            return st.none.clone();
        }
    }
    let _ = tisp_eval_with_env;
    st.none.clone()
}

pub fn tib_env_io(st: &mut Tsp) {
    for name in &["write", "read", "parse", "load"] {
        let v = Val {
            t: TspType::TspPrim,
            v: ValUnion::Pr { name: name.to_string(), pr: dummy_prim },
        };
        rec_add(&mut st.env, name, v);
    }
}

fn dummy_prim(_st: Tsp, _env: Rec, _args: Val) -> Val {
    make_none()
}
