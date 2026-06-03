use std::cell::RefCell;
use std::collections::HashMap;
use std::io::Write;
use std::rc::Rc;

pub const TSP_REC_MAX_PRINT: usize = 64;
pub const TSP_SYM_CHARS: &str = "_!?@#$%&~*-";
pub const TSP_REC_FACTOR: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TspType {
    TspNone = 1 << 0,
    TspNil = 1 << 1,
    TspInt = 1 << 2,
    TspDec = 1 << 3,
    TspRatio = 1 << 4,
    TspStr = 1 << 5,
    TspSym = 1 << 6,
    TspPrim = 1 << 7,
    TspForm = 1 << 8,
    TspFunc = 1 << 9,
    TspMacro = 1 << 10,
    TspPair = 1 << 11,
    TspRec = 1 << 12,
}

pub const TSP_RATIONAL: u32 = TspType::TspInt as u32 | TspType::TspRatio as u32;
pub const TSP_OP_CHARS: &str = "_+-*/\\|=^<>.:";
pub const TSP_NUM: u32 = TSP_RATIONAL | TspType::TspDec as u32;
pub const TSP_EXPR: u32 = TSP_NUM | TspType::TspSym as u32 | TspType::TspPair as u32;

pub struct Entry {
    pub key: String,
    pub val: Val,
}

// Primitive function signature - takes Tsp state, env, and args
pub type Prim = fn(&mut Tsp, &Rc<RefCell<Rec>>, Val) -> Option<Val>;

pub struct Rec {
    pub size: i32,
    pub cap: i32,
    pub items: HashMap<String, Val>,
    pub next: Option<Rc<RefCell<Rec>>>,
}

pub struct Tsp {
    pub file: String,
    pub filec: usize,
    pub none: Val,
    pub nil: Val,
    pub t: Val,
    pub env: Rc<RefCell<Rec>>,
    pub strs: Rc<RefCell<Rec>>,
    pub syms: Rc<RefCell<Rec>>,
    pub libh: Vec<*mut std::ffi::c_void>,
    pub libhc: usize,
}

#[derive(Clone)]
pub struct Val {
    pub t: TspType,
    pub v: ValUnion,
}

#[derive(Clone)]
pub enum ValUnion {
    None,
    S(String),
    N { num: f64, den: f64 },
    Pr { name: String, pr: Prim },
    F {
        name: String,
        args: Box<Val>,
        body: Box<Val>,
        env: Rc<RefCell<Rec>>,
    },
    P {
        car: Box<Val>,
        cdr: Box<Val>,
    },
    R(Rc<RefCell<Rec>>),
}

pub fn rec_new(_cap: usize, next: Option<Rc<RefCell<Rec>>>) -> Rec {
    Rec {
        size: 0,
        cap: 0,
        items: HashMap::new(),
        next,
    }
}

pub fn rec_add(rec: &mut Rec, key: &str, val: Val) {
    if !rec.items.contains_key(key) {
        rec.size += 1;
    }
    rec.items.insert(key.to_string(), val);
}

pub fn rec_get(rec: &Rec, key: &str) -> Option<Val> {
    if let Some(v) = rec.items.get(key) {
        return Some(v.clone());
    }
    if let Some(next) = &rec.next {
        return rec_get(&next.borrow(), key);
    }
    None
}

pub fn entry_get<'a>(_rec: &'a Rec, _key: &'a str) -> Option<&'a Entry> {
    None
}

pub fn rec_grow(_rec: &mut Rec) {}

pub fn rec_extend(rec: &Rc<RefCell<Rec>>, args: Val, vals: Val) -> Rc<RefCell<Rec>> {
    let new = Rc::new(RefCell::new(rec_new(8, Some(rec.clone()))));
    let mut a = args;
    let mut v = vals;
    loop {
        match (&a.t, &v.t) {
            (TspType::TspNil, _) => break,
            (TspType::TspPair, TspType::TspPair) => {
                let (acar, acdr) = match &a.v {
                    ValUnion::P { car, cdr } => ((**car).clone(), (**cdr).clone()),
                    _ => break,
                };
                let (vcar, vcdr) = match &v.v {
                    ValUnion::P { car, cdr } => ((**car).clone(), (**cdr).clone()),
                    _ => break,
                };
                if let ValUnion::S(s) = &acar.v {
                    rec_add(&mut new.borrow_mut(), s, vcar);
                }
                a = acdr;
                v = vcdr;
            }
            _ => {
                if let ValUnion::S(s) = &a.v {
                    rec_add(&mut new.borrow_mut(), s, v);
                }
                break;
            }
        }
    }
    new
}

pub fn hash(key: &str) -> u32 {
    let mut h: u32 = 0;
    for c in key.chars() {
        h = h.wrapping_mul(33).wrapping_add(c as u32);
    }
    h
}

pub fn mk_val(t: TspType) -> Val {
    Val {
        t,
        v: ValUnion::None,
    }
}

pub fn mk_int(i: i32) -> Val {
    Val {
        t: TspType::TspInt,
        v: ValUnion::N {
            num: i as f64,
            den: 1.0,
        },
    }
}

pub fn mk_dec(d: f64) -> Option<Val> {
    Some(Val {
        t: TspType::TspDec,
        v: ValUnion::N { num: d, den: 1.0 },
    })
}

pub fn frac_reduce(num: &mut i32, den: &mut i32) {
    let mut a = num.abs();
    let mut b = den.abs();
    if b == 0 {
        return;
    }
    let mut c = a % b;
    while c > 0 {
        a = b;
        b = c;
        c = a % b;
    }
    *num = *num / b;
    *den = *den / b;
}

pub fn mk_rat(num: i32, den: i32) -> Option<Val> {
    if den == 0 {
        return None;
    }
    let mut n = num;
    let mut d = den;
    frac_reduce(&mut n, &mut d);
    if d < 0 {
        d = d.abs();
        n = -n;
    }
    if d == 1 {
        return Some(mk_int(n));
    }
    Some(Val {
        t: TspType::TspRatio,
        v: ValUnion::N {
            num: n as f64,
            den: d as f64,
        },
    })
}

pub fn mk_str(st: &mut Tsp, s: &str) -> Option<Val> {
    {
        let strs = st.strs.borrow();
        if let Some(v) = rec_get(&strs, s) {
            return Some(v);
        }
    }
    let v = Val {
        t: TspType::TspStr,
        v: ValUnion::S(s.to_string()),
    };
    rec_add(&mut st.strs.borrow_mut(), s, v.clone());
    Some(v)
}

pub fn mk_sym(st: &mut Tsp, s: &str) -> Option<Val> {
    {
        let syms = st.syms.borrow();
        if let Some(v) = rec_get(&syms, s) {
            return Some(v);
        }
    }
    let v = Val {
        t: TspType::TspSym,
        v: ValUnion::S(s.to_string()),
    };
    rec_add(&mut st.syms.borrow_mut(), s, v.clone());
    Some(v)
}

pub fn mk_prim(t: TspType, pr: Prim, name: &str) -> Option<Val> {
    Some(Val {
        t,
        v: ValUnion::Pr {
            name: name.to_string(),
            pr,
        },
    })
}

pub fn mk_func(t: TspType, name: &str, args: Val, body: Val, env: Rc<RefCell<Rec>>) -> Option<Val> {
    Some(Val {
        t,
        v: ValUnion::F {
            name: name.to_string(),
            args: Box::new(args),
            body: Box::new(body),
            env,
        },
    })
}

pub fn mk_pair(a: Val, b: Val) -> Option<Val> {
    Some(Val {
        t: TspType::TspPair,
        v: ValUnion::P {
            car: Box::new(a),
            cdr: Box::new(b),
        },
    })
}

pub fn mk_list(_st: &mut Tsp, n: i32, args: Vec<Val>) -> Option<Val> {
    if args.is_empty() {
        return None;
    }
    let nil = Val {
        t: TspType::TspNil,
        v: ValUnion::None,
    };
    let mut result = nil.clone();
    for i in (0..n as usize).rev() {
        if i >= args.len() {
            continue;
        }
        result = mk_pair(args[i].clone(), result)?;
    }
    Some(result)
}

pub fn mk_rec(_st: &mut Tsp, _env: Rec, _assoc: Val) -> Option<Val> {
    None
}

pub fn tsp_lstlen(v: &Val) -> i32 {
    let mut len = 0;
    let mut cur = v.clone();
    loop {
        if matches!(cur.t, TspType::TspPair) {
            if let ValUnion::P { cdr, .. } = &cur.v {
                len += 1;
                cur = (**cdr).clone();
            } else {
                break;
            }
        } else if matches!(cur.t, TspType::TspNil) {
            return len;
        } else {
            return -(len + 1);
        }
    }
    len
}

pub fn tsp_type_str(t: TspType) -> &'static str {
    match t {
        TspType::TspNone => "Void",
        TspType::TspNil => "Nil",
        TspType::TspInt => "Int",
        TspType::TspDec => "Dec",
        TspType::TspRatio => "Ratio",
        TspType::TspStr => "Str",
        TspType::TspSym => "Sym",
        TspType::TspPrim => "Prim",
        TspType::TspForm => "Form",
        TspType::TspFunc => "Func",
        TspType::TspMacro => "Macro",
        TspType::TspPair => "Pair",
        TspType::TspRec => "Rec",
    }
}

pub fn is_sym(c: char) -> bool {
    c.is_ascii_alphanumeric() || TSP_SYM_CHARS.contains(c)
}

pub fn is_op(c: char) -> bool {
    TSP_OP_CHARS.contains(c)
}

pub fn isnum(s: &str) -> bool {
    let bytes = s.as_bytes();
    if bytes.is_empty() {
        return false;
    }
    let c0 = bytes[0] as char;
    if c0.is_ascii_digit() {
        return true;
    }
    if bytes.len() < 2 {
        return false;
    }
    let c1 = bytes[1] as char;
    if c0 == '.' && c1.is_ascii_digit() {
        return true;
    }
    if (c0 == '-' || c0 == '+') && (c1.is_ascii_digit() || c1 == '.') {
        return true;
    }
    false
}

pub fn esc_char(c: char) -> char {
    match c {
        'n' => '\n',
        'r' => '\r',
        't' => '\t',
        '\n' => ' ',
        _ => c,
    }
}

pub fn esc_str(s: &str, len: i32, do_esc: i32) -> String {
    let mut ret = String::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    let len = len as usize;
    let mut count = 0;
    while count < len && i < bytes.len() {
        if bytes[i] as char == '\\' && do_esc != 0 {
            i += 1;
            if i < bytes.len() {
                ret.push(esc_char(bytes[i] as char));
            }
        } else {
            ret.push(bytes[i] as char);
        }
        i += 1;
        count += 1;
    }
    ret
}

pub fn skip_ws(st: &mut Tsp, skipnl: i32) {
    let s: &[char] = if skipnl != 0 {
        &[' ', '\t', '\n', '\r']
    } else {
        &[' ', '\t']
    };
    while st.filec < st.file.len() {
        let c = st.file.as_bytes()[st.filec] as char;
        if s.contains(&c) {
            st.filec += 1;
        } else if c == ';' {
            // skip comment until newline
            while st.filec < st.file.len() && st.file.as_bytes()[st.filec] as char != '\n' {
                st.filec += 1;
            }
            if skipnl == 0 && st.filec < st.file.len() {
                // do not consume newline if not skipping newlines
                break;
            }
        } else {
            break;
        }
    }
}

pub fn read_sign(st: &mut Tsp) -> i32 {
    if st.filec >= st.file.len() {
        return 1;
    }
    let c = st.file.as_bytes()[st.filec] as char;
    match c {
        '-' => {
            st.filec += 1;
            -1
        }
        '+' => {
            st.filec += 1;
            1
        }
        _ => 1,
    }
}

pub fn read_int(st: &mut Tsp) -> i32 {
    let mut ret: i32 = 0;
    while st.filec < st.file.len() {
        let c = st.file.as_bytes()[st.filec] as char;
        if !c.is_ascii_digit() {
            break;
        }
        ret = ret * 10 + (c as i32 - '0' as i32);
        st.filec += 1;
    }
    ret
}

pub fn read_sci(st: &mut Tsp, mut val: f64, isint: i32) -> Option<Val> {
    if st.filec < st.file.len() {
        let c = st.file.as_bytes()[st.filec] as char;
        if c == 'e' || c == 'E' {
            st.filec += 1;
            let sign = if read_sign(st) == 1 { 10.0 } else { 0.1 };
            let mut expo = read_int(st);
            while expo > 0 {
                val *= sign;
                expo -= 1;
            }
        }
    }
    if isint != 0 {
        return Some(mk_int(val as i32));
    }
    mk_dec(val)
}

pub fn read_num(st: &mut Tsp) -> Val {
    let sign = read_sign(st);
    let num = read_int(st);
    if st.filec < st.file.len() {
        let c = st.file.as_bytes()[st.filec] as char;
        match c {
            '/' => {
                st.filec += 1;
                let sign2 = read_sign(st);
                let den = read_int(st);
                return mk_rat(sign * num, sign2 * den).unwrap_or(mk_int(0));
            }
            '.' => {
                st.filec += 1;
                let oldc = st.filec;
                let mut d = read_int(st) as f64;
                let size = st.filec - oldc;
                for _ in 0..size {
                    d /= 10.0;
                }
                return read_sci(st, sign as f64 * (num as f64 + d), 0)
                    .unwrap_or(mk_int(0));
            }
            _ => {}
        }
    }
    read_sci(st, (sign * num) as f64, 1).unwrap_or(mk_int(0))
}

pub fn read_str(st: &mut Tsp, mk_fn: fn(&mut Tsp, &str) -> Option<Val>) -> Option<Val> {
    // skip starting open quote
    st.filec += 1;
    let start = st.filec;
    let bytes = st.file.as_bytes().to_vec();
    let endchar = if (mk_fn as usize) == (mk_str as usize) {
        '"'
    } else {
        '~'
    };
    let mut len = 0;
    while st.filec < bytes.len() && bytes[st.filec] as char != endchar {
        if bytes[st.filec] as char == '\\'
            && st.filec > 0
            && bytes[st.filec - 1] as char != '\\'
        {
            st.filec += 1;
        }
        if st.filec >= bytes.len() {
            return None;
        }
        st.filec += 1;
        len += 1;
    }
    if st.filec >= bytes.len() {
        return None;
    }
    st.filec += 1; // skip last closing quote
    let raw = std::str::from_utf8(&bytes[start..start + len]).ok()?.to_string();
    let do_esc = if (mk_fn as usize) == (mk_str as usize) {
        1
    } else {
        0
    };
    let escaped = esc_str(&raw, len as i32, do_esc);
    mk_fn(st, &escaped)
}

pub fn read_sym(st: &mut Tsp, is_char: fn(char) -> bool) -> Option<Val> {
    let start = st.filec;
    let bytes = st.file.as_bytes().to_vec();
    let mut len = 0;
    while st.filec < bytes.len() && is_char(bytes[st.filec] as char) {
        st.filec += 1;
        len += 1;
    }
    let raw = std::str::from_utf8(&bytes[start..start + len]).ok()?.to_string();
    mk_sym(st, &raw)
}

pub fn read_pair(st: &mut Tsp, endchar: char) -> Option<Val> {
    let skipnl = if endchar != '\n' { 1 } else { 0 };
    skip_ws(st, skipnl);
    let nil = Val {
        t: TspType::TspNil,
        v: ValUnion::None,
    };
    let mut items: Vec<Val> = Vec::new();
    let mut tail: Option<Val> = None;
    while st.filec < st.file.len()
        && st.file.as_bytes()[st.filec] as char != endchar
    {
        let v = tisp_read(st)?;
        // check for "." improper list
        if matches!(v.t, TspType::TspSym) {
            if let ValUnion::S(s) = &v.v {
                if s == "." {
                    skip_ws(st, skipnl);
                    let v2 = tisp_read(st)?;
                    tail = Some(v2);
                    break;
                }
            }
        }
        items.push(v);
        skip_ws(st, skipnl);
    }
    skip_ws(st, skipnl);
    if skipnl != 0
        && (st.filec >= st.file.len() || st.file.as_bytes()[st.filec] as char != endchar)
    {
        return None;
    }
    if st.filec < st.file.len() {
        st.filec += 1;
    }
    let mut result = tail.unwrap_or(nil);
    for v in items.into_iter().rev() {
        result = mk_pair(v, result)?;
    }
    Some(result)
}

pub fn tisp_read_sexpr(st: &mut Tsp) -> Option<Val> {
    skip_ws(st, 1);
    if st.filec >= st.file.len() {
        return Some(st.none.clone());
    }
    // empty file remainder
    let remain = &st.file[st.filec..];
    if remain.is_empty() {
        return Some(st.none.clone());
    }
    if isnum(remain) {
        return Some(read_num(st));
    }
    let c = st.file.as_bytes()[st.filec] as char;
    if c == '"' {
        return read_str(st, mk_str);
    }
    if c == '~' {
        return read_str(st, mk_sym);
    }
    // prefixes
    let prefixes: &[(&str, &str)] = &[
        ("'", "quote"),
        ("`", "quasiquote"),
        (",@", "unquote-splice"),
        (",", "unquote"),
        ("@", "Func"),
        ("f\"", "strformat"),
    ];
    for (pre, expanded) in prefixes {
        if remain.starts_with(pre) {
            let inc = pre.len() - if pre.as_bytes().get(1) == Some(&b'"') { 1 } else { 0 };
            st.filec += inc;
            let v = tisp_read(st)?;
            let sym = mk_sym(st, expanded)?;
            return mk_list(st, 2, vec![sym, v]);
        }
    }
    if is_op(c) {
        return read_sym(st, is_op);
    }
    if is_sym(c) {
        return read_sym(st, is_sym);
    }
    if c == '(' {
        st.filec += 1;
        return read_pair(st, ')');
    }
    if c == '[' {
        st.filec += 1;
        let lst = read_pair(st, ']')?;
        let sym = mk_sym(st, "list")?;
        return mk_pair(sym, lst);
    }
    if c == '{' {
        st.filec += 1;
        let v = read_pair(st, '}')?;
        let sym = mk_sym(st, "Rec")?;
        return mk_pair(sym, v);
    }
    None
}

pub fn tisp_read(st: &mut Tsp) -> Option<Val> {
    let mut v = tisp_read_sexpr(st)?;
    while st.filec < st.file.len() {
        let c = st.file.as_bytes()[st.filec] as char;
        if c == '(' || c == ':' || c == '>' || c == '{' {
            v = tisp_read_sugar(st, v)?;
        } else {
            break;
        }
    }
    Some(v)
}

pub fn tisp_read_sugar(st: &mut Tsp, v: Val) -> Option<Val> {
    if st.filec >= st.file.len() {
        return Some(v);
    }
    let c = st.file.as_bytes()[st.filec] as char;
    if c == '(' {
        st.filec += 1;
        let lst = read_pair(st, ')')?;
        return mk_pair(v, lst);
    }
    if c == '{' {
        st.filec += 1;
        let lst = read_pair(st, '}')?;
        let recmerge = mk_sym(st, "recmerge")?;
        let recsym = mk_sym(st, "Rec")?;
        let inner = mk_pair(recsym, lst)?;
        return mk_list(st, 3, vec![recmerge, v, inner]);
    }
    if c == ':' {
        st.filec += 1;
        if st.filec >= st.file.len() {
            return Some(v);
        }
        let c2 = st.file.as_bytes()[st.filec] as char;
        if c2 == '(' {
            st.filec += 1;
            let w = read_pair(st, ')')?;
            let sym = mk_sym(st, "map")?;
            let inner = mk_pair(v, w)?;
            return mk_pair(sym, inner);
        }
        if c2 == ':' {
            st.filec += 1;
            let w = read_sym(st, is_sym)?;
            let qsym = mk_sym(st, "quote")?;
            let inner = mk_list(st, 2, vec![qsym, w])?;
            return mk_list(st, 2, vec![v, inner]);
        }
        skip_ws(st, 1);
        let w = tisp_read(st)?;
        return mk_list(st, 2, vec![v, w]);
    }
    if c == '>' && st.filec + 1 < st.file.len() && st.file.as_bytes()[st.filec + 1] as char == '>' {
        st.filec += 2;
        let w = tisp_read(st)?;
        if !matches!(w.t, TspType::TspPair) {
            return None;
        }
        if let ValUnion::P { car, cdr } = &w.v {
            let inner = mk_pair(v, (**cdr).clone())?;
            return mk_pair((**car).clone(), inner);
        }
    }
    Some(v)
}

pub fn tisp_read_line(_st: &mut Tsp, _level: i32) -> Option<Val> {
    None
}

pub fn tisp_eval_list(st: &mut Tsp, env: &Rc<RefCell<Rec>>, v: Val) -> Option<Val> {
    let nil = Val {
        t: TspType::TspNil,
        v: ValUnion::None,
    };
    let mut items: Vec<Val> = Vec::new();
    let mut cur = v;
    let mut tail: Option<Val> = None;
    loop {
        if matches!(cur.t, TspType::TspNil) {
            break;
        }
        if !matches!(cur.t, TspType::TspPair) {
            let ev = tisp_eval_v(st, env, cur)?;
            tail = Some(ev);
            break;
        }
        let (car, cdr) = match &cur.v {
            ValUnion::P { car, cdr } => ((**car).clone(), (**cdr).clone()),
            _ => break,
        };
        let ev = tisp_eval_v(st, env, car)?;
        items.push(ev);
        cur = cdr;
    }
    let mut result = tail.unwrap_or(nil);
    for v in items.into_iter().rev() {
        result = mk_pair(v, result)?;
    }
    Some(result)
}

pub fn tisp_eval_body(st: &mut Tsp, env: &Rc<RefCell<Rec>>, v: Val) -> Option<Val> {
    let mut ret = st.none.clone();
    let mut cur = v;
    while matches!(cur.t, TspType::TspPair) {
        let (car, cdr) = match &cur.v {
            ValUnion::P { car, cdr } => ((**car).clone(), (**cdr).clone()),
            _ => break,
        };
        ret = tisp_eval_v(st, env, car)?;
        cur = cdr;
    }
    Some(ret)
}

pub fn prepend_bt(_st: &mut Tsp, _env: &mut Rec, _f: Val) {}

pub fn eval_proc(st: &mut Tsp, env: &Rc<RefCell<Rec>>, f: Val, args: Val) -> Option<Val> {
    match f.t {
        TspType::TspPrim => {
            let evaled_args = tisp_eval_list(st, env, args)?;
            if let ValUnion::Pr { pr, .. } = &f.v {
                return pr(st, env, evaled_args);
            }
            None
        }
        TspType::TspForm => {
            if let ValUnion::Pr { pr, .. } = &f.v {
                return pr(st, env, args);
            }
            None
        }
        TspType::TspFunc => {
            let evaled_args = tisp_eval_list(st, env, args)?;
            if let ValUnion::F { args: fargs, body, env: fenv, .. } = &f.v {
                let new_env = rec_extend(fenv, (**fargs).clone(), evaled_args);
                return tisp_eval_body(st, &new_env, (**body).clone());
            }
            None
        }
        TspType::TspMacro => {
            if let ValUnion::F { args: fargs, body, env: fenv, .. } = &f.v {
                let new_env = rec_extend(fenv, (**fargs).clone(), args);
                let ret = tisp_eval_body(st, &new_env, (**body).clone())?;
                return tisp_eval_v(st, env, ret);
            }
            None
        }
        _ => None,
    }
}

// internal evaluator that takes &Rc<RefCell<Rec>>
pub fn tisp_eval_v(st: &mut Tsp, env: &Rc<RefCell<Rec>>, v: Val) -> Option<Val> {
    match v.t {
        TspType::TspSym => {
            if let ValUnion::S(s) = &v.v {
                let val = rec_get(&env.borrow(), s);
                return val;
            }
            None
        }
        TspType::TspPair => {
            let (car, cdr) = match &v.v {
                ValUnion::P { car, cdr } => ((**car).clone(), (**cdr).clone()),
                _ => return None,
            };
            let f = tisp_eval_v(st, env, car)?;
            eval_proc(st, env, f, cdr)
        }
        _ => Some(v),
    }
}

pub fn tisp_eval(st: &mut Tsp, v: Val) -> Option<Val> {
    let env = st.env.clone();
    tisp_eval_v(st, &env, v)
}

pub fn tisp_print(f: &mut std::fs::File, v: &Val) {
    match v.t {
        TspType::TspNone => {
            let _ = write!(f, "Void");
        }
        TspType::TspNil => {
            let _ = write!(f, "Nil");
        }
        TspType::TspInt => {
            if let ValUnion::N { num, .. } = &v.v {
                let _ = write!(f, "{}", *num as i32);
            }
        }
        TspType::TspDec => {
            if let ValUnion::N { num, .. } = &v.v {
                // Use %.15g equivalent format
                let s = format_g(*num);
                let _ = write!(f, "{}", s);
                // If it's an integer value, append ".0"
                if *num == (*num as i64) as f64 && !s.contains('.') && !s.contains('e') && !s.contains('E') {
                    let _ = write!(f, ".0");
                }
            }
        }
        TspType::TspRatio => {
            if let ValUnion::N { num, den } = &v.v {
                let _ = write!(f, "{}/{}", *num as i32, *den as i32);
            }
        }
        TspType::TspStr | TspType::TspSym => {
            if let ValUnion::S(s) = &v.v {
                let _ = write!(f, "{}", s);
            }
        }
        TspType::TspFunc | TspType::TspMacro => {
            if let ValUnion::F { name, .. } = &v.v {
                let label = if matches!(v.t, TspType::TspFunc) {
                    "function"
                } else {
                    "macro"
                };
                if name.is_empty() {
                    let _ = write!(f, "#<{}>", label);
                } else {
                    let _ = write!(f, "#<{}:{}>", label, name);
                }
            }
        }
        TspType::TspPrim => {
            if let ValUnion::Pr { name, .. } = &v.v {
                let _ = write!(f, "#<primitive:{}>", name);
            }
        }
        TspType::TspForm => {
            if let ValUnion::Pr { name, .. } = &v.v {
                let _ = write!(f, "#<form:{}>", name);
            }
        }
        TspType::TspRec => {
            let _ = write!(f, "{{ }}");
        }
        TspType::TspPair => {
            let _ = write!(f, "(");
            if let ValUnion::P { car, cdr } = &v.v {
                tisp_print(f, car);
                let mut cur = (**cdr).clone();
                loop {
                    if matches!(cur.t, TspType::TspNil) {
                        break;
                    }
                    if matches!(cur.t, TspType::TspPair) {
                        let _ = write!(f, " ");
                        if let ValUnion::P { car, cdr } = &cur.v {
                            tisp_print(f, car);
                            cur = (**cdr).clone();
                        } else {
                            break;
                        }
                    } else {
                        let _ = write!(f, " . ");
                        tisp_print(f, &cur);
                        break;
                    }
                }
            }
            let _ = write!(f, ")");
        }
    }
}

// Format like %.15g in C
pub fn format_g(num: f64) -> String {
    if num.is_nan() {
        return "nan".to_string();
    }
    if num.is_infinite() {
        return if num < 0.0 { "-inf".to_string() } else { "inf".to_string() };
    }
    // %.15g chooses between fixed and scientific based on the exponent.
    // It uses 15 significant digits.
    let abs = num.abs();
    let exp = if abs == 0.0 {
        0i32
    } else {
        abs.log10().floor() as i32
    };
    let precision = 15;
    if exp < -4 || exp >= precision {
        // scientific
        format_scientific(num, precision)
    } else {
        // fixed
        format_fixed(num, precision - 1 - exp)
    }
}

fn format_scientific(num: f64, precision: i32) -> String {
    // produce N significant digits in scientific form, like %.15g
    let s = format!("{:.*e}", (precision - 1).max(0) as usize, num);
    // Rust gives e.g. "1e0" -> "1e0" or "1.5e2"; we need format like "1e+0", "5e+16"
    // Normalize: trim trailing zeros from mantissa fraction, ensure sign on exponent
    // Split "mantissa" "e" "exp"
    if let Some(epos) = s.find('e') {
        let mantissa = &s[..epos];
        let exp_part = &s[epos + 1..];
        let exp_num: i32 = exp_part.parse().unwrap_or(0);
        // trim trailing zeros from mantissa fraction
        let mantissa = if mantissa.contains('.') {
            let m = mantissa.trim_end_matches('0').trim_end_matches('.');
            m.to_string()
        } else {
            mantissa.to_string()
        };
        // Format exponent with sign and at least 2 digits
        let sign = if exp_num < 0 { '-' } else { '+' };
        let abs_exp = exp_num.abs();
        return format!("{}e{}{:02}", mantissa, sign, abs_exp);
    }
    s
}

fn format_fixed(num: f64, precision: i32) -> String {
    if precision < 0 {
        return format!("{}", num.round() as i64);
    }
    let s = format!("{:.*}", precision as usize, num);
    // trim trailing zeros from fraction (but %.15g keeps at least one digit if integer, but typically removes trailing zeros)
    if s.contains('.') {
        let trimmed = s.trim_end_matches('0').trim_end_matches('.');
        return trimmed.to_string();
    }
    s
}

pub fn vals_eq(a: &Val, b: &Val) -> bool {
    let a_num = matches!(a.t, TspType::TspInt | TspType::TspDec | TspType::TspRatio);
    let b_num = matches!(b.t, TspType::TspInt | TspType::TspDec | TspType::TspRatio);
    if a_num && b_num {
        if let (ValUnion::N { num: an, den: ad }, ValUnion::N { num: bn, den: bd }) = (&a.v, &b.v) {
            return an == bn && ad == bd;
        }
        return false;
    }
    if a.t != b.t {
        return false;
    }
    match (&a.v, &b.v) {
        (ValUnion::S(s1), ValUnion::S(s2)) => s1 == s2,
        _ => true,
    }
}

pub fn tisp_env_add(st: &mut Tsp, key: &str, v: Val) {
    rec_add(&mut st.env.borrow_mut(), key, v);
}

pub fn tisp_env_init(cap: usize) -> Tsp {
    let none = Val {
        t: TspType::TspNone,
        v: ValUnion::None,
    };
    let nil = Val {
        t: TspType::TspNil,
        v: ValUnion::None,
    };
    let t = Val {
        t: TspType::TspSym,
        v: ValUnion::S("True".to_string()),
    };
    let env = Rc::new(RefCell::new(rec_new(cap, None)));
    let strs = Rc::new(RefCell::new(rec_new(cap, None)));
    let syms = Rc::new(RefCell::new(rec_new(cap, None)));
    let mut st = Tsp {
        file: String::new(),
        filec: 0,
        none: none.clone(),
        nil: nil.clone(),
        t: t.clone(),
        env,
        strs,
        syms,
        libh: Vec::new(),
        libhc: 0,
    };
    tisp_env_add(&mut st, "True", t);
    tisp_env_add(&mut st, "Nil", nil);
    tisp_env_add(&mut st, "Void", none);
    let bt = Val {
        t: TspType::TspNil,
        v: ValUnion::None,
    };
    tisp_env_add(&mut st, "bt", bt);
    let ver = Val {
        t: TspType::TspStr,
        v: ValUnion::S("0.1".to_string()),
    };
    tisp_env_add(&mut st, "version", ver);
    st
}

pub fn tisp_env_lib(st: &mut Tsp, lib: &str) {
    let file = std::mem::replace(&mut st.file, lib.to_string());
    let filec = std::mem::replace(&mut st.filec, 0);
    skip_ws(st, 1);
    if let Some(v) = tisp_read(st) {
        let env = st.env.clone();
        let _ = tisp_eval_body(st, &env, v);
    }
    st.file = file;
    st.filec = filec;
}

pub fn tib_env_core(st: &mut Tsp) {
    crate::core::tib_env_core(st);
}

pub fn tib_env_string(st: &mut Tsp) {
    crate::string::tib_env_string(st);
}

pub fn tib_env_math(st: &mut Tsp) {
    crate::math::tib_env_math(st);
}

pub fn tib_env_io(st: &mut Tsp) {
    crate::io::tib_env_io(st);
}

pub fn tib_env_os(st: &mut Tsp) {
    crate::os::tib_env_os(st);
}
