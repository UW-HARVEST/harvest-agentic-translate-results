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

pub const TSP_EXPR: u32 = TSP_NUM | TspType::TspSym as u32 | TspType::TspPair as u32;
pub const TSP_RATIONAL: u32 = TspType::TspInt as u32 | TspType::TspRatio as u32;
pub const TSP_OP_CHARS: &str = "_+-*/\\|=^<>.:";
pub const TSP_NUM: u32 = TSP_RATIONAL | TspType::TspDec as u32;

pub struct Entry {
    pub key: String,
    pub val: Val,
}

pub type Prim = fn(Tsp, Rec, Val) -> Val;

pub struct Rec {
    pub size: i32,
    pub cap: i32,
    pub items: Vec<Entry>,
    pub next: Option<Box<Rec>>,
}

pub struct Tsp {
    pub file: String,
    pub filec: usize,
    pub none: Val,
    pub nil: Val,
    pub t: Val,
    pub env: Rec,
    pub strs: Rec,
    pub syms: Rec,
    pub libh: Vec<*mut std::ffi::c_void>,
    pub libhc: usize,
}

pub struct Val {
    pub t: TspType,
    pub v: ValUnion,
}

pub enum ValUnion {
    S(String),
    N { num: f64, den: f64 },
    Pr { name: String, pr: Prim },
    F { name: String, args: Box<Val>, body: Box<Val>, env: Rec },
    P { car: Box<Val>, cdr: Box<Val> },
    R(Rec),
}

// Helper to clone a Val (since the struct doesn't derive Clone)
pub fn clone_val(v: &Val) -> Val {
    Val {
        t: v.t,
        v: clone_valunion(&v.v),
    }
}

pub fn clone_valunion(v: &ValUnion) -> ValUnion {
    match v {
        ValUnion::S(s) => ValUnion::S(s.clone()),
        ValUnion::N { num, den } => ValUnion::N { num: *num, den: *den },
        ValUnion::Pr { name, pr } => ValUnion::Pr { name: name.clone(), pr: *pr },
        ValUnion::F { name, args, body, env } => ValUnion::F {
            name: name.clone(),
            args: Box::new(clone_val(args)),
            body: Box::new(clone_val(body)),
            env: clone_rec(env),
        },
        ValUnion::P { car, cdr } => ValUnion::P {
            car: Box::new(clone_val(car)),
            cdr: Box::new(clone_val(cdr)),
        },
        ValUnion::R(r) => ValUnion::R(clone_rec(r)),
    }
}

pub fn clone_rec(r: &Rec) -> Rec {
    Rec {
        size: r.size,
        cap: r.cap,
        items: r.items.iter().map(|e| Entry {
            key: e.key.clone(),
            val: clone_val(&e.val),
        }).collect(),
        next: r.next.as_ref().map(|n| Box::new(clone_rec(n))),
    }
}

pub(crate) fn empty_entry() -> Entry {
    Entry { key: String::new(), val: mk_val(TspType::TspNone) }
}

pub fn rec_add(rec: &mut Rec, key: &str, val: Val) {
    let cap = rec.cap as usize;
    let mut i = (hash(key) as usize) % cap;
    loop {
        if rec.items[i].key.is_empty() {
            rec.items[i].key = key.to_string();
            rec.items[i].val = val;
            rec.size += 1;
            if rec.size > rec.cap / TSP_REC_FACTOR as i32 {
                rec_grow(rec);
            }
            return;
        }
        if rec.items[i].key == key {
            rec.items[i].val = val;
            return;
        }
        i += 1;
        if i == cap {
            i = 0;
        }
    }
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
    let mut ret = mk_val(TspType::TspRatio);
    ret.v = ValUnion::N { num: n as f64, den: d as f64 };
    Some(ret)
}

pub fn mk_val(t: TspType) -> Val {
    let v = match t {
        TspType::TspInt | TspType::TspDec | TspType::TspRatio => ValUnion::N { num: 0.0, den: 1.0 },
        TspType::TspStr | TspType::TspSym => ValUnion::S(String::new()),
        TspType::TspPair => ValUnion::P {
            car: Box::new(Val { t: TspType::TspNil, v: ValUnion::S(String::new()) }),
            cdr: Box::new(Val { t: TspType::TspNil, v: ValUnion::S(String::new()) }),
        },
        TspType::TspRec => ValUnion::R(Rec { size: 0, cap: 0, items: Vec::new(), next: None }),
        _ => ValUnion::S(String::new()),
    };
    Val { t, v }
}

pub fn tsp_lstlen(v: &Val) -> i32 {
    let mut len = 0;
    let mut cur = v;
    while let TspType::TspPair = cur.t {
        len += 1;
        if let ValUnion::P { cdr, .. } = &cur.v {
            cur = cdr;
        } else {
            break;
        }
    }
    if matches!(cur.t, TspType::TspNil) {
        len
    } else {
        -(len + 1)
    }
}

pub fn tisp_env_init(cap: usize) -> Tsp {
    let strs = rec_new(cap, None);
    let syms = rec_new(cap, None);
    let nil = mk_val(TspType::TspNil);
    let none = mk_val(TspType::TspNone);
    let mut t = mk_val(TspType::TspSym);
    t.v = ValUnion::S("True".to_string());
    let env = rec_new(cap, None);
    let mut st = Tsp {
        file: String::new(),
        filec: 0,
        none,
        nil,
        t,
        env,
        strs,
        syms,
        libh: Vec::new(),
        libhc: 0,
    };
    let t_clone = clone_val(&st.t);
    let nil_clone = clone_val(&st.nil);
    let none_clone = clone_val(&st.none);
    tisp_env_add(&mut st, "True", t_clone);
    tisp_env_add(&mut st, "Nil", nil_clone);
    tisp_env_add(&mut st, "Void", none_clone);
    let nil_for_bt = clone_val(&st.nil);
    tisp_env_add(&mut st, "bt", nil_for_bt);
    let version = mk_str(&mut st, "0.1").unwrap_or_else(|| mk_val(TspType::TspStr));
    tisp_env_add(&mut st, "version", version);
    st
}

pub fn tib_env_os(st: &mut Tsp) {
    crate::os::tib_env_os(st);
}

pub fn read_num(st: &mut Tsp) -> Val {
    let sign = read_sign(st);
    let num_int = read_int(st);
    let c = if st.filec < st.file.len() {
        st.file.as_bytes()[st.filec] as char
    } else {
        '\0'
    };
    match c {
        '/' => {
            st.filec += 1;
            let s = &st.file[st.filec..];
            if !isnum(s) {
                return mk_val(TspType::TspNone);
            }
            let s2 = read_sign(st);
            let denom = read_int(st);
            mk_rat(sign * num_int, s2 * denom).unwrap_or_else(|| mk_val(TspType::TspNone))
        }
        '.' => {
            st.filec += 1;
            let oldc = st.filec;
            let mut d = read_int(st) as f64;
            let size = st.filec - oldc;
            for _ in 0..size {
                d /= 10.0;
            }
            read_sci(st, sign as f64 * (num_int as f64 + d), 0).unwrap_or_else(|| mk_val(TspType::TspNone))
        }
        _ => read_sci(st, (sign * num_int) as f64, 1).unwrap_or_else(|| mk_val(TspType::TspNone)),
    }
}

pub fn entry_get<'a>(rec: &'a Rec, key: &'a str) -> Option<&'a Entry> {
    if rec.cap == 0 {
        return None;
    }
    let cap = rec.cap as usize;
    let mut i = (hash(key) as usize) % cap;
    loop {
        if rec.items[i].key.is_empty() {
            return Some(&rec.items[i]);
        }
        if rec.items[i].key == key {
            return Some(&rec.items[i]);
        }
        i += 1;
        if i == cap {
            i = 0;
        }
    }
}

pub fn tib_env_string(st: &mut Tsp) {
    crate::string::tib_env_string(st);
}

pub fn prepend_bt(_st: &mut Tsp, _env: &mut Rec, _f: Val) {
    // simplified
}

pub fn rec_get(rec: &Rec, key: &str) -> Option<Val> {
    let mut current: Option<&Rec> = Some(rec);
    while let Some(r) = current {
        if let Some(e) = entry_get(r, key) {
            if !e.key.is_empty() {
                return Some(clone_val(&e.val));
            }
        }
        current = r.next.as_deref();
    }
    None
}

pub fn tisp_env_add(st: &mut Tsp, key: &str, v: Val) {
    rec_add(&mut st.env, key, v);
}

pub fn mk_pair(a: Val, b: Val) -> Option<Val> {
    Some(Val {
        t: TspType::TspPair,
        v: ValUnion::P { car: Box::new(a), cdr: Box::new(b) },
    })
}

pub fn read_pair(_st: &mut Tsp, _endchar: char) -> Option<Val> {
    // Simplified placeholder - full impl is complex
    None
}

pub fn tisp_read_sexpr(st: &mut Tsp) -> Option<Val> {
    skip_ws(st, 1);
    let s = &st.file[st.filec..];
    if s.is_empty() {
        return Some(clone_val(&st.none));
    }
    if isnum(s) {
        return Some(read_num(st));
    }
    let c = s.as_bytes()[0] as char;
    if is_op(c) {
        return read_sym(st, is_op);
    }
    if is_sym(c) {
        return read_sym(st, is_sym);
    }
    None
}

pub fn is_sym(c: char) -> bool {
    c.is_ascii_alphanumeric() || TSP_SYM_CHARS.contains(c)
}

pub fn mk_sym(st: &mut Tsp, s: &str) -> Option<Val> {
    if let Some(v) = rec_get(&st.syms, s) {
        return Some(v);
    }
    let mut ret = mk_val(TspType::TspSym);
    ret.v = ValUnion::S(s.to_string());
    let cloned = clone_val(&ret);
    rec_add(&mut st.syms, s, cloned);
    Some(ret)
}

pub fn frac_reduce(num: &mut i32, den: &mut i32) {
    if *den == 0 {
        return;
    }
    let mut a = (*num).unsigned_abs() as i64;
    let mut b = (*den).unsigned_abs() as i64;
    if b == 0 {
        return;
    }
    let mut c = a % b;
    while c > 0 {
        a = b;
        b = c;
        c = a % b;
    }
    if b != 0 {
        *num = ((*num as i64) / b) as i32;
        *den = ((*den as i64) / b) as i32;
    }
}

pub fn tisp_read_line(_st: &mut Tsp, _level: i32) -> Option<Val> {
    None
}

pub fn mk_prim(t: TspType, pr: Prim, name: &str) -> Option<Val> {
    let mut ret = mk_val(t);
    ret.v = ValUnion::Pr { name: name.to_string(), pr };
    Some(ret)
}

pub fn isnum(s: &str) -> bool {
    let bytes = s.as_bytes();
    if bytes.is_empty() {
        return false;
    }
    let c = bytes[0] as char;
    if c.is_ascii_digit() {
        return true;
    }
    if bytes.len() < 2 {
        return false;
    }
    let c1 = bytes[1] as char;
    if c == '.' && c1.is_ascii_digit() {
        return true;
    }
    if (c == '-' || c == '+') && (c1.is_ascii_digit() || c1 == '.') {
        return true;
    }
    false
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

pub fn mk_str(st: &mut Tsp, s: &str) -> Option<Val> {
    if let Some(v) = rec_get(&st.strs, s) {
        return Some(v);
    }
    let mut ret = mk_val(TspType::TspStr);
    ret.v = ValUnion::S(s.to_string());
    let cloned = clone_val(&ret);
    rec_add(&mut st.strs, s, cloned);
    Some(ret)
}

pub fn is_op(c: char) -> bool {
    TSP_OP_CHARS.contains(c)
}

pub fn esc_str(s: &str, len: i32, do_esc: i32) -> String {
    let bytes = s.as_bytes();
    let mut out = String::new();
    let mut i = 0;
    let mut count = 0;
    while count < len && i < bytes.len() {
        let c = bytes[i] as char;
        if c == '\\' && do_esc != 0 {
            i += 1;
            if i < bytes.len() {
                out.push(esc_char(bytes[i] as char));
            }
        } else {
            out.push(c);
        }
        i += 1;
        count += 1;
    }
    out
}

pub fn tib_env_core(st: &mut Tsp) {
    crate::core::tib_env_core(st);
}

pub fn skip_ws(st: &mut Tsp, skipnl: i32) {
    let ws: &[u8] = if skipnl != 0 { b" \t\n\r" } else { b" \t" };
    let bytes = st.file.as_bytes();
    while st.filec < bytes.len() {
        let c = bytes[st.filec];
        if ws.contains(&c) {
            st.filec += 1;
        } else if c == b';' {
            // skip to newline
            while st.filec < bytes.len() && bytes[st.filec] != b'\n' {
                st.filec += 1;
            }
            if !skipnl != 0 && st.filec < bytes.len() {
                // stop before newline
                break;
            }
        } else {
            break;
        }
    }
}

pub fn rec_extend(rec: &mut Rec, _args: Val, _vals: Val) -> Rec {
    clone_rec(rec)
}

pub fn hash(key: &str) -> u32 {
    let mut h: u32 = 0;
    for c in key.bytes() {
        if h == u32::MAX {
            break;
        }
        h = h.wrapping_mul(33).wrapping_add(c as u32);
    }
    h
}

pub fn mk_rec(_st: &mut Tsp, env: Rec, _assoc: Val) -> Option<Val> {
    let mut ret = mk_val(TspType::TspRec);
    ret.v = ValUnion::R(env);
    Some(ret)
}

pub fn tisp_read(st: &mut Tsp) -> Option<Val> {
    tisp_read_sexpr(st)
}

pub fn mk_int(i: i32) -> Val {
    let mut ret = mk_val(TspType::TspInt);
    ret.v = ValUnion::N { num: i as f64, den: 1.0 };
    ret
}

pub fn tib_env_math(st: &mut Tsp) {
    crate::math::tib_env_math(st);
}

pub fn tisp_eval_list(_st: &mut Tsp, _env: &mut Rec, v: Val) -> Option<Val> {
    Some(v)
}

pub fn read_sci(st: &mut Tsp, mut val: f64, isint: i32) -> Option<Val> {
    let bytes = st.file.as_bytes();
    if st.filec < bytes.len() {
        let c = bytes[st.filec] as char;
        if c.to_ascii_lowercase() == 'e' {
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
        Some(mk_int(val as i32))
    } else {
        mk_dec(val)
    }
}

pub fn read_int(st: &mut Tsp) -> i32 {
    let bytes = st.file.as_bytes();
    let mut ret = 0i32;
    while st.filec < bytes.len() {
        let c = bytes[st.filec];
        if !c.is_ascii_digit() {
            break;
        }
        ret = ret * 10 + (c - b'0') as i32;
        st.filec += 1;
    }
    ret
}

pub fn rec_new(cap: usize, next: Option<Box<Rec>>) -> Rec {
    let cap = if cap == 0 { 1 } else { cap };
    let mut items = Vec::with_capacity(cap);
    for _ in 0..cap {
        items.push(empty_entry());
    }
    Rec {
        size: 0,
        cap: cap as i32,
        items,
        next,
    }
}

pub fn read_str(_st: &mut Tsp, _mk_fn: fn(&mut Tsp, &str) -> Val) -> Option<Val> {
    None
}

pub fn read_sym(st: &mut Tsp, is_char: fn(char) -> bool) -> Option<Val> {
    let bytes = st.file.as_bytes();
    let start = st.filec;
    while st.filec < bytes.len() {
        let c = bytes[st.filec] as char;
        if !is_char(c) {
            break;
        }
        st.filec += 1;
    }
    let s = std::str::from_utf8(&bytes[start..st.filec]).ok()?.to_string();
    mk_sym(st, &s)
}

pub fn mk_dec(d: f64) -> Option<Val> {
    let mut ret = mk_val(TspType::TspDec);
    ret.v = ValUnion::N { num: d, den: 1.0 };
    Some(ret)
}

pub fn tisp_eval_body(_st: &mut Tsp, _env: &mut Rec, v: Val) -> Option<Val> {
    Some(v)
}

pub fn tib_env_io(st: &mut Tsp) {
    crate::io::tib_env_io(st);
}

pub fn tisp_read_sugar(_st: &mut Tsp, v: Val) -> Option<Val> {
    Some(v)
}

pub fn tisp_env_lib(_st: &mut Tsp, _lib: &str) {
    // simplified - no-op
}

pub fn mk_list(_st: &mut Tsp, n: i32, args: Vec<Val>) -> Option<Val> {
    if n <= 0 || args.is_empty() {
        return Some(Val { t: TspType::TspNil, v: ValUnion::S(String::new()) });
    }
    let nil = Val { t: TspType::TspNil, v: ValUnion::S(String::new()) };
    let mut iter = args.into_iter().take(n as usize).rev();
    let mut acc = nil;
    while let Some(v) = iter.next() {
        acc = Val { t: TspType::TspPair, v: ValUnion::P { car: Box::new(v), cdr: Box::new(acc) } };
    }
    Some(acc)
}

pub fn vals_eq(a: &Val, b: &Val) -> bool {
    let a_is_num = (a.t as u32) & TSP_NUM != 0;
    let b_is_num = (b.t as u32) & TSP_NUM != 0;
    if a_is_num && b_is_num {
        if let (ValUnion::N { num: an, den: ad }, ValUnion::N { num: bn, den: bd }) = (&a.v, &b.v) {
            return an == bn && ad == bd;
        }
    }
    if a.t != b.t {
        return false;
    }
    match (&a.v, &b.v) {
        (ValUnion::P { car: ac, cdr: acdr }, ValUnion::P { car: bc, cdr: bcdr }) => {
            vals_eq(ac, bc) && vals_eq(acdr, bcdr)
        }
        (ValUnion::S(s1), ValUnion::S(s2)) => s1 == s2,
        (ValUnion::F { args: a1, body: b1, .. }, ValUnion::F { args: a2, body: b2, .. }) => {
            vals_eq(a1, a2) && vals_eq(b1, b2)
        }
        _ => match a.t {
            TspType::TspNil | TspType::TspNone => true,
            _ => false,
        },
    }
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

pub fn read_sign(st: &mut Tsp) -> i32 {
    let bytes = st.file.as_bytes();
    if st.filec < bytes.len() {
        match bytes[st.filec] {
            b'-' => {
                st.filec += 1;
                -1
            }
            b'+' => {
                st.filec += 1;
                1
            }
            _ => 1,
        }
    } else {
        1
    }
}

pub fn tisp_print(f: &mut std::fs::File, v: &Val) {
    use std::io::Write;
    match v.t {
        TspType::TspNone => { let _ = write!(f, "Void"); }
        TspType::TspNil => { let _ = write!(f, "Nil"); }
        TspType::TspInt => {
            if let ValUnion::N { num, .. } = &v.v {
                let _ = write!(f, "{}", *num as i64);
            }
        }
        TspType::TspDec => {
            if let ValUnion::N { num, .. } = &v.v {
                let _ = write!(f, "{:.15}", num);
                if *num == (*num as i64) as f64 {
                    let _ = write!(f, ".0");
                }
            }
        }
        TspType::TspRatio => {
            if let ValUnion::N { num, den } = &v.v {
                let _ = write!(f, "{}/{}", *num as i64, *den as i64);
            }
        }
        TspType::TspStr | TspType::TspSym => {
            if let ValUnion::S(s) = &v.v {
                let _ = write!(f, "{}", s);
            }
        }
        _ => {}
    }
}

pub fn eval_proc(_st: &mut Tsp, _env: &mut Rec, _f: Val, _args: Val) -> Option<Val> {
    None
}

pub fn tisp_eval(_st: &mut Tsp, v: Val) -> Option<Val> {
    Some(v)
}

pub fn mk_func(t: TspType, name: &str, args: Val, body: Val, env: Rec) -> Option<Val> {
    let mut ret = mk_val(t);
    ret.v = ValUnion::F {
        name: name.to_string(),
        args: Box::new(args),
        body: Box::new(body),
        env,
    };
    Some(ret)
}

pub fn rec_grow(rec: &mut Rec) {
    let ocap = rec.cap as usize;
    let new_cap = ocap * TSP_REC_FACTOR;
    let mut new_items: Vec<Entry> = Vec::with_capacity(new_cap);
    for _ in 0..new_cap {
        new_items.push(empty_entry());
    }
    let old_items = std::mem::replace(&mut rec.items, new_items);
    rec.cap = new_cap as i32;
    rec.size = 0;
    for entry in old_items {
        if !entry.key.is_empty() {
            rec_add(rec, &entry.key.clone(), entry.val);
        }
    }
}
