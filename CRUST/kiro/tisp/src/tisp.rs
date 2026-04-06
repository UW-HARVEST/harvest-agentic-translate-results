use std::io::Write;

pub const TSP_REC_MAX_PRINT: usize = 64;
pub const TSP_SYM_CHARS: &str = "_!?@#$%&~*-";
pub const TSP_REC_FACTOR: usize = 2;
pub const TSP_OP_CHARS: &str = "_+-*/\\|=^<>.:";
pub const TSP_RATIONAL: u32 = TspType::TspInt as u32 | TspType::TspRatio as u32;
pub const TSP_NUM: u32 = TSP_RATIONAL | TspType::TspDec as u32;
pub const TSP_EXPR: u32 = TSP_NUM | TspType::TspSym as u32 | TspType::TspPair as u32;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TspType {
    TspNone = 1 << 0, TspNil = 1 << 1, TspInt = 1 << 2, TspDec = 1 << 3,
    TspRatio = 1 << 4, TspStr = 1 << 5, TspSym = 1 << 6, TspPrim = 1 << 7,
    TspForm = 1 << 8, TspFunc = 1 << 9, TspMacro = 1 << 10, TspPair = 1 << 11,
    TspRec = 1 << 12,
}

pub struct Entry { pub key: String, pub val: Val }
pub type Prim = fn(&mut Tsp, &mut Rec, Val) -> Val;
pub struct Rec { pub size: i32, pub cap: i32, pub items: Vec<Entry>, pub next: Option<Box<Rec>> }
pub struct Tsp {
    pub file: String, pub filec: usize,
    pub none: Val, pub nil: Val, pub t: Val,
    pub env: Rec, pub strs: Rec, pub syms: Rec,
    pub libh: Vec<*mut std::ffi::c_void>, pub libhc: usize,
}
pub struct Val { pub t: TspType, pub v: ValUnion }
pub enum ValUnion {
    S(String),
    N { num: f64, den: f64 },
    Pr { name: String, pr: Prim },
    F { name: String, args: Box<Val>, body: Box<Val>, env: Rec },
    P { car: Box<Val>, cdr: Box<Val> },
    R(Rec),
}

// ---- helpers ----
pub fn car_pub(v: &Val) -> &Val { match &v.v { ValUnion::P { car, .. } => car, _ => panic!("car") } }
pub fn cdr_pub(v: &Val) -> &Val { match &v.v { ValUnion::P { cdr, .. } => cdr, _ => panic!("cdr") } }
pub fn num_pub(v: &Val) -> f64 { match &v.v { ValUnion::N { num, .. } => *num, _ => panic!("num") } }
pub fn den_pub(v: &Val) -> f64 { match &v.v { ValUnion::N { den, .. } => *den, _ => panic!("den") } }
pub fn sym_str_pub(v: &Val) -> &str { match &v.v { ValUnion::S(s) => s, _ => panic!("sym_str") } }
pub fn nilp_pub(v: &Val) -> bool { v.t == TspType::TspNil }
pub fn is_type_pub(v: &Val, mask: u32) -> bool { (v.t as u32 & mask) != 0 }
pub fn mk_nil_pub() -> Val { Val { t: TspType::TspNil, v: ValUnion::N { num: 0.0, den: 0.0 } } }
pub fn mk_none_pub() -> Val { Val { t: TspType::TspNone, v: ValUnion::N { num: 0.0, den: 0.0 } } }
pub fn mk_error() -> Val { Val { t: TspType::TspNil, v: ValUnion::S("__ERROR__".into()) } }
pub fn is_error(v: &Val) -> bool { v.t == TspType::TspNil && matches!(&v.v, ValUnion::S(s) if s == "__ERROR__") }

fn car(v: &Val) -> &Val { car_pub(v) }
fn cdr(v: &Val) -> &Val { cdr_pub(v) }
fn num(v: &Val) -> f64 { num_pub(v) }
fn den(v: &Val) -> f64 { den_pub(v) }
fn sym_str(v: &Val) -> &str { sym_str_pub(v) }
fn nilp(v: &Val) -> bool { nilp_pub(v) }
fn is_type(v: &Val, mask: u32) -> bool { is_type_pub(v, mask) }
fn mk_nil() -> Val { mk_nil_pub() }
fn mk_none() -> Val { mk_none_pub() }

pub fn clone_val_pub(v: &Val) -> Val { clone_val(v) }
pub fn clone_rec_pub(r: &Rec) -> Rec { clone_rec(r) }

fn clone_val(v: &Val) -> Val { Val { t: v.t, v: clone_union(&v.v) } }
fn clone_union(v: &ValUnion) -> ValUnion {
    match v {
        ValUnion::N { num, den } => ValUnion::N { num: *num, den: *den },
        ValUnion::S(s) => ValUnion::S(s.clone()),
        ValUnion::P { car, cdr } => ValUnion::P { car: Box::new(clone_val(car)), cdr: Box::new(clone_val(cdr)) },
        ValUnion::Pr { name, pr } => ValUnion::Pr { name: name.clone(), pr: *pr },
        ValUnion::F { name, args, body, env } => ValUnion::F {
            name: name.clone(), args: Box::new(clone_val(args)),
            body: Box::new(clone_val(body)), env: clone_rec(env),
        },
        ValUnion::R(r) => ValUnion::R(clone_rec(r)),
    }
}
fn clone_rec(r: &Rec) -> Rec {
    Rec {
        size: r.size, cap: r.cap,
        items: r.items.iter().map(|e| Entry { key: e.key.clone(), val: clone_val(&e.val) }).collect(),
        next: r.next.as_ref().map(|n| Box::new(clone_rec(n))),
    }
}

// ---- type str ----
pub fn tsp_type_str(t: TspType) -> &'static str {
    match t {
        TspType::TspNone => "Void", TspType::TspNil => "Nil", TspType::TspInt => "Int",
        TspType::TspDec => "Dec", TspType::TspRatio => "Ratio", TspType::TspStr => "Str",
        TspType::TspSym => "Sym", TspType::TspPrim => "Prim", TspType::TspForm => "Form",
        TspType::TspFunc => "Func", TspType::TspMacro => "Macro", TspType::TspPair => "Pair",
        TspType::TspRec => "Rec",
    }
}
pub fn tsp_type_str_mask_pub(t: u32) -> &'static str {
    if t == TSP_EXPR { "Expr" } else if t == TSP_RATIONAL { "Rational" }
    else if t & TSP_NUM != 0 { "Num" } else { "Invalid" }
}

// ---- predicates ----
pub fn is_sym(c: char) -> bool { c.is_ascii_alphanumeric() || TSP_SYM_CHARS.contains(c) }
pub fn is_op(c: char) -> bool { TSP_OP_CHARS.contains(c) }
pub fn isnum(s: &str) -> bool {
    let b = s.as_bytes();
    if b.is_empty() { return false; }
    b[0].is_ascii_digit()
        || (b[0] == b'.' && b.len() > 1 && b[1].is_ascii_digit())
        || ((b[0] == b'-' || b[0] == b'+') && b.len() > 1 && (b[1].is_ascii_digit() || b[1] == b'.'))
}

// ---- hash / rec ----
pub fn hash(key: &str) -> u32 {
    let mut h: u32 = 0;
    for c in key.bytes() { h = h.wrapping_mul(33).wrapping_add(c as u32); }
    h
}
pub fn rec_new(cap: usize, next: Option<Box<Rec>>) -> Rec {
    let mut items = Vec::with_capacity(cap);
    for _ in 0..cap { items.push(Entry { key: String::new(), val: mk_nil() }); }
    Rec { size: 0, cap: cap as i32, items, next }
}
pub fn entry_get<'a>(rec: &'a Rec, key: &str) -> Option<&'a Entry> {
    if rec.cap == 0 { return None; }
    let mut i = (hash(key) % rec.cap as u32) as usize;
    loop {
        if rec.items[i].key.is_empty() { return None; }
        if rec.items[i].key == key { return Some(&rec.items[i]); }
        i = (i + 1) % rec.cap as usize;
    }
}
fn entry_idx(rec: &Rec, key: &str) -> usize {
    let mut i = (hash(key) % rec.cap as u32) as usize;
    loop {
        if rec.items[i].key.is_empty() || rec.items[i].key == key { return i; }
        i = (i + 1) % rec.cap as usize;
    }
}
pub fn rec_get(rec: &Rec, key: &str) -> Option<Val> {
    let mut r = Some(rec);
    while let Some(cur) = r {
        if let Some(e) = entry_get(cur, key) { return Some(clone_val(&e.val)); }
        r = cur.next.as_deref();
    }
    None
}
pub fn rec_grow(rec: &mut Rec) {
    let old = std::mem::take(&mut rec.items);
    rec.cap *= TSP_REC_FACTOR as i32;
    rec.size = 0;
    rec.items = Vec::with_capacity(rec.cap as usize);
    for _ in 0..rec.cap { rec.items.push(Entry { key: String::new(), val: mk_nil() }); }
    for e in old { if !e.key.is_empty() { rec_add(rec, &e.key, e.val); } }
}
pub fn rec_add(rec: &mut Rec, key: &str, val: Val) {
    let i = entry_idx(rec, key);
    rec.items[i].val = val;
    if rec.items[i].key.is_empty() {
        rec.items[i].key = key.to_string();
        rec.size += 1;
        if rec.size > rec.cap / TSP_REC_FACTOR as i32 { rec_grow(rec); }
    }
}
pub fn rec_extend(rec: Rec, args: Val, vals: Val) -> Rec {
    let argnum = TSP_REC_FACTOR as i32 * tsp_lstlen(&args);
    let cap = if argnum > 0 { argnum as usize } else { (-argnum + 1) as usize };
    let mut ret = rec_new(cap, Some(Box::new(rec)));
    let mut a = &args; let mut v = &vals;
    loop {
        let (arg, val) = if a.t == TspType::TspPair { (car(a), car(v)) } else { (a, v) };
        if arg.t != TspType::TspSym {
            eprintln!("; tisp: error: expected symbol for argument of function definition, recieved '{}'", tsp_type_str(arg.t));
            return ret;
        }
        rec_add(&mut ret, sym_str(arg), clone_val(val));
        if a.t != TspType::TspPair { break; }
        // Need owned copies to continue iteration
        let a_next = clone_val(cdr(a));
        let v_next = clone_val(cdr(v));
        // We can't keep borrowing, so we'll use a different approach
        drop(a); drop(v);
        return rec_extend_loop(ret, a_next, v_next);
    }
    ret
}
fn rec_extend_loop(mut ret: Rec, mut args: Val, mut vals: Val) -> Rec {
    while !nilp(&args) {
        let (arg, val) = if args.t == TspType::TspPair {
            (clone_val(car(&args)), clone_val(car(&vals)))
        } else {
            let a = clone_val(&args); let v = clone_val(&vals);
            rec_add(&mut ret, sym_str(&a), v);
            break;
        };
        if arg.t != TspType::TspSym {
            eprintln!("; tisp: error: expected symbol for argument of function definition, recieved '{}'", tsp_type_str(arg.t));
            return ret;
        }
        rec_add(&mut ret, sym_str(&arg), val);
        args = clone_val(cdr(&args));
        vals = clone_val(cdr(&vals));
    }
    ret
}

// ---- vals_eq / frac_reduce ----
pub fn vals_eq(a: &Val, b: &Val) -> bool {
    if is_type(a, TSP_NUM) && is_type(b, TSP_NUM) { return num(a) == num(b) && den(a) == den(b); }
    if a.t != b.t { return false; }
    if a.t == TspType::TspPair { return vals_eq(car(a), car(b)) && vals_eq(cdr(a), cdr(b)); }
    if a.t == TspType::TspFunc || a.t == TspType::TspMacro {
        if let (ValUnion::F { args: aa, body: ab, .. }, ValUnion::F { args: ba, body: bb, .. }) = (&a.v, &b.v) {
            return vals_eq(aa, ba) && vals_eq(ab, bb);
        }
    }
    if let (ValUnion::S(sa), ValUnion::S(sb)) = (&a.v, &b.v) { return sa == sb; }
    if a.t == TspType::TspNil || a.t == TspType::TspNone { return true; }
    false
}
pub fn frac_reduce(num: &mut i32, den: &mut i32) {
    let (mut a, mut b) = (num.unsigned_abs(), den.unsigned_abs());
    let mut c = a % b;
    while c > 0 { a = b; b = c; c = a % b; }
    *num /= b as i32; *den /= b as i32;
}

// ---- mk_* ----
pub fn mk_val(t: TspType) -> Val { Val { t, v: ValUnion::N { num: 0.0, den: 0.0 } } }
pub fn mk_int(i: i32) -> Val { Val { t: TspType::TspInt, v: ValUnion::N { num: i as f64, den: 1.0 } } }
pub fn mk_dec(d: f64) -> Option<Val> { Some(Val { t: TspType::TspDec, v: ValUnion::N { num: d, den: 1.0 } }) }
pub fn mk_rat(num: i32, den: i32) -> Option<Val> {
    if den == 0 { eprintln!("; tisp: error: division by zero"); return None; }
    let (mut n, mut d) = (num, den);
    frac_reduce(&mut n, &mut d);
    if d < 0 { d = d.abs(); n = -n; }
    if d == 1 { return Some(mk_int(n)); }
    Some(Val { t: TspType::TspRatio, v: ValUnion::N { num: n as f64, den: d as f64 } })
}
pub fn mk_str(st: &mut Tsp, s: &str) -> Val {
    if let Some(v) = rec_get(&st.strs, s) { return v; }
    let v = Val { t: TspType::TspStr, v: ValUnion::S(s.to_string()) };
    rec_add(&mut st.strs, s, clone_val(&v)); v
}
pub fn mk_sym(st: &mut Tsp, s: &str) -> Val {
    if let Some(v) = rec_get(&st.syms, s) { return v; }
    let v = Val { t: TspType::TspSym, v: ValUnion::S(s.to_string()) };
    rec_add(&mut st.syms, s, clone_val(&v)); v
}
pub fn mk_prim(t: TspType, pr: Prim, name: &str) -> Val {
    Val { t, v: ValUnion::Pr { name: name.to_string(), pr } }
}
pub fn mk_func(t: TspType, name: &str, args: Val, body: Val, env: Rec) -> Val {
    Val { t, v: ValUnion::F { name: name.to_string(), args: Box::new(args), body: Box::new(body), env } }
}
pub fn mk_pair(a: Val, b: Val) -> Val {
    Val { t: TspType::TspPair, v: ValUnion::P { car: Box::new(a), cdr: Box::new(b) } }
}
pub fn mk_list(st: &mut Tsp, _n: i32, args: Vec<Val>) -> Val {
    let mut result = mk_nil();
    for v in args.into_iter().rev() { result = mk_pair(v, result); }
    result
}
pub fn mk_rec(st: &mut Tsp, env: Rec, assoc: Val) -> Val {
    let mut ret = mk_val(TspType::TspRec);
    // If assoc is nil with N variant, no assoc list
    if nilp(&assoc) {
        ret.v = ValUnion::R(env); return ret;
    }
    let cap = TSP_REC_FACTOR * tsp_lstlen(&assoc).unsigned_abs() as usize;
    let cap = if cap > 0 { cap } else { 1 };
    let mut inner = rec_new(cap, None);
    let mut env_rec = rec_new(4, Some(Box::new(env)));
    // We need to add "this" pointing to ret, but ret isn't built yet
    // Build inner first, then wrap
    let mut cur = &assoc;
    while cur.t == TspType::TspPair {
        let item = car(cur);
        if item.t == TspType::TspPair && is_type(car(item), TspType::TspSym as u32 | TspType::TspStr as u32) {
            let key = sym_str(car(item)).to_string();
            let val_expr = clone_val(car(cdr(item)));
            if let Some(v) = tisp_eval_with_env(st, &mut env_rec, val_expr) {
                rec_add(&mut inner, &key, v);
            } else { return mk_nil(); }
        } else if item.t == TspType::TspSym {
            let key = sym_str(item).to_string();
            let item_clone = clone_val(item);
            if let Some(v) = tisp_eval_with_env(st, &mut env_rec, item_clone) {
                rec_add(&mut inner, &key, v);
            } else { return mk_nil(); }
        } else {
            eprintln!("; tisp: error: Rec: missing key symbol or string");
            return mk_nil();
        }
        cur = cdr(cur);
    }
    ret.v = ValUnion::R(inner); ret
}

// ---- reader ----
fn fget(st: &Tsp) -> Option<char> { st.file.as_bytes().get(st.filec).map(|&b| b as char) }
fn fgetat(st: &Tsp, off: isize) -> Option<char> {
    let i = st.filec as isize + off;
    if i < 0 { None } else { st.file.as_bytes().get(i as usize).map(|&b| b as char) }
}

pub fn skip_ws(st: &mut Tsp, skipnl: i32) {
    let ws: &[u8] = if skipnl != 0 { b" \t\n\r" } else { b" \t" };
    loop {
        match fget(st) {
            Some(c) if ws.contains(&(c as u8)) => { st.filec += 1; }
            Some(';') => {
                while fget(st).is_some() && fget(st) != Some('\n') { st.filec += 1; }
                if skipnl != 0 { if fget(st) == Some('\n') { st.filec += 1; } }
            }
            _ => break,
        }
    }
}

pub fn read_sign(st: &mut Tsp) -> i32 {
    match fget(st) {
        Some('-') => { st.filec += 1; -1 }
        Some('+') => { st.filec += 1; 1 }
        _ => 1,
    }
}
pub fn read_int(st: &mut Tsp) -> i32 {
    let mut ret = 0i32;
    while let Some(c) = fget(st) {
        if !c.is_ascii_digit() { break; }
        ret = ret * 10 + (c as i32 - '0' as i32);
        st.filec += 1;
    }
    ret
}
pub fn read_sci(st: &mut Tsp, val: f64, isint: i32) -> Option<Val> {
    let mut v = val;
    if let Some(c) = fget(st) {
        if c == 'e' || c == 'E' {
            st.filec += 1;
            let s = if read_sign(st) == 1 { 10.0 } else { 0.1 };
            let expo = read_int(st);
            for _ in 0..expo { v *= s; }
        }
    }
    if isint != 0 { Some(mk_int(v as i32)) } else { mk_dec(v) }
}
pub fn read_num(st: &mut Tsp) -> Val {
    let sign = read_sign(st);
    let n = read_int(st);
    match fget(st) {
        Some('/') => {
            st.filec += 1;
            if !isnum(&st.file[st.filec..]) {
                eprintln!("; tisp: error: incorrect ratio format, no denominator found");
                return mk_nil();
            }
            let ds = read_sign(st); let d = read_int(st);
            mk_rat(sign * n, ds * d).unwrap_or_else(mk_nil)
        }
        Some('.') => {
            st.filec += 1;
            let oldc = st.filec;
            let _frac = read_int(st);
            let size = st.filec - oldc;
            let mut d = _frac as f64;
            for _ in 0..size { d /= 10.0; }
            read_sci(st, sign as f64 * (n as f64 + d), 0).unwrap_or_else(mk_nil)
        }
        _ => read_sci(st, sign as f64 * n as f64, 1).unwrap_or_else(mk_nil),
    }
}
pub fn esc_char(c: char) -> char {
    match c { 'n' => '\n', 'r' => '\r', 't' => '\t', '\n' => ' ', _ => c }
}
pub fn esc_str(s: &str, len: i32, do_esc: i32) -> String {
    let b = s.as_bytes();
    let mut ret = String::new();
    let mut i = 0usize; let mut count = 0;
    while count < len as usize && i < b.len() {
        if b[i] == b'\\' && do_esc != 0 {
            i += 1;
            if i < b.len() { ret.push(esc_char(b[i] as char)); }
        } else { ret.push(b[i] as char); }
        i += 1; count += 1;
    }
    ret
}
pub fn read_str(st: &mut Tsp, mk_fn: fn(&mut Tsp, &str) -> Val) -> Option<Val> {
    let endchar = if fget(st) == Some('"') { '"' } else { '~' };
    let do_esc = if endchar == '"' { 1 } else { 0 };
    st.filec += 1;
    let start = st.filec;
    let mut len = 0i32;
    loop {
        match fget(st) {
            None => { eprintln!("; tisp: error: reached end before closing {}", endchar); return None; }
            Some(c) if c == endchar => break,
            Some('\\') if do_esc != 0 => {
                st.filec += 1; // skip escaped char
                st.filec += 1; len += 1;
            }
            _ => { st.filec += 1; len += 1; }
        }
    }
    let raw = &st.file[start..st.filec].to_string();
    st.filec += 1;
    let escaped = esc_str(raw, len, do_esc);
    Some(mk_fn(st, &escaped))
}
pub fn read_sym(st: &mut Tsp, is_char: fn(char) -> bool) -> Option<Val> {
    let start = st.filec;
    while let Some(c) = fget(st) { if !is_char(c) { break; } st.filec += 1; }
    let s = st.file[start..st.filec].to_string();
    Some(mk_sym(st, &s))
}

pub fn read_pair(st: &mut Tsp, endchar: char) -> Option<Val> {
    let skipnl = endchar != '\n';
    skip_ws(st, if skipnl { 1 } else { 0 });
    let mut items: Vec<Val> = Vec::new();
    let mut improper_cdr: Option<Val> = None;
    while let Some(c) = fget(st) {
        if c == endchar || c == '\0' { break; }
        let v = tisp_read(st)?;
        if v.t == TspType::TspSym { if let ValUnion::S(ref s) = v.v { if s == "." {
            skip_ws(st, if skipnl { 1 } else { 0 });
            improper_cdr = Some(tisp_read(st)?);
            break;
        }}}
        items.push(v);
        skip_ws(st, if skipnl { 1 } else { 0 });
    }
    skip_ws(st, if skipnl { 1 } else { 0 });
    if skipnl && fget(st) != Some(endchar) {
        eprintln!("; tisp: error: did not find closing '{}'", endchar);
        return None;
    }
    st.filec += 1;
    let mut result = improper_cdr.unwrap_or_else(mk_nil);
    for v in items.into_iter().rev() { result = mk_pair(v, result); }
    Some(result)
}

fn mk_str_w(st: &mut Tsp, s: &str) -> Val { mk_str(st, s) }
fn mk_sym_w(st: &mut Tsp, s: &str) -> Val { mk_sym(st, s) }

pub fn tisp_read_sexpr(st: &mut Tsp) -> Option<Val> {
    static PREFIXES: &[(&str, &str)] = &[
        ("'", "quote"), ("`", "quasiquote"), (",@", "unquote-splice"),
        (",", "unquote"), ("@", "Func"), ("f\"", "strformat"),
    ];
    skip_ws(st, 1);
    if st.filec >= st.file.len() { return Some(mk_none()); }
    let remaining = &st.file[st.filec..];
    if remaining.is_empty() { return Some(mk_none()); }
    if isnum(remaining) { return Some(read_num(st)); }
    if fget(st) == Some('"') { return read_str(st, mk_str_w); }
    if fget(st) == Some('~') { return read_str(st, mk_sym_w); }
    for &(prefix, name) in PREFIXES {
        if remaining.starts_with(prefix) {
            let skip = prefix.len() - if prefix.len() > 1 && prefix.as_bytes()[1] == b'"' { 1 } else { 0 };
            st.filec += skip;
            let v = tisp_read(st)?;
            let sym = mk_sym(st, name);
            return Some(mk_list(st, 2, vec![sym, v]));
        }
    }
    if let Some(c) = fget(st) {
        if is_op(c) { return read_sym(st, is_op); }
        if is_sym(c) { return read_sym(st, is_sym); }
        if c == '(' { st.filec += 1; return read_pair(st, ')'); }
        if c == '[' { st.filec += 1; let l = read_pair(st, ']')?; let s = mk_sym(st, "list"); return Some(mk_pair(s, l)); }
        if c == '{' { st.filec += 1; let v = read_pair(st, '}')?; let s = mk_sym(st, "Rec"); return Some(mk_pair(s, v)); }
        eprintln!("; tisp: error: could not read given input '{}' ({})", c, c as u32);
        return None;
    }
    Some(mk_none())
}

pub fn tisp_read(st: &mut Tsp) -> Option<Val> {
    let mut v = tisp_read_sexpr(st)?;
    loop {
        match fget(st) {
            Some('(') | Some(':') | Some('{') => { v = tisp_read_sugar(st, v)?; }
            Some('>') if fgetat(st, 1) == Some('>') => { v = tisp_read_sugar(st, v)?; }
            _ => break,
        }
    }
    Some(v)
}

pub fn tisp_read_sugar(st: &mut Tsp, v: Val) -> Option<Val> {
    match fget(st) {
        Some('(') => { st.filec += 1; let l = read_pair(st, ')')?; Some(mk_pair(v, l)) }
        Some('{') => {
            st.filec += 1; let l = read_pair(st, '}')?;
            let s1 = mk_sym(st, "recmerge"); let s2 = mk_sym(st, "Rec");
            Some(mk_list(st, 3, vec![s1, v, mk_pair(s2, l)]))
        }
        Some(':') => {
            st.filec += 1;
            match fget(st) {
                Some('(') => { st.filec += 1; let w = read_pair(st, ')')?; let s = mk_sym(st, "map"); Some(mk_pair(s, mk_pair(v, w))) }
                Some(':') => {
                    st.filec += 1; let w = read_sym(st, is_sym)?;
                    let qs = mk_sym(st, "quote");
                    let q = mk_list(st, 2, vec![qs, w]);
                    Some(mk_list(st, 2, vec![v, q]))
                }
                _ => { skip_ws(st, 1); let w = tisp_read(st)?; Some(mk_list(st, 2, vec![v, w])) }
            }
        }
        Some('>') if fgetat(st, 1) == Some('>') => {
            st.filec += 2; let w = tisp_read(st)?;
            if w.t != TspType::TspPair { eprintln!("; tisp: error: invalid UFCS"); return None; }
            let wc = clone_val(car(&w)); let wd = clone_val(cdr(&w));
            Some(mk_pair(wc, mk_pair(v, wd)))
        }
        _ => Some(v),
    }
}

pub fn tisp_read_line(st: &mut Tsp, level: i32) -> Option<Val> {
    let mut ret = read_pair(st, '\n')?;
    if ret.t != TspType::TspPair { ret = mk_pair(ret, mk_nil()); }
    loop {
        if fget(st).is_none() { break; }
        let saved = st.filec;
        let mut newlevel = 0i32;
        while let Some(c) = fget(st) {
            if c == '\t' || c == ' ' { newlevel += 1; st.filec += 1; } else { break; }
        }
        if newlevel <= level { st.filec = saved; break; }
        let sub = tisp_read_line(st, newlevel)?;
        // Append sub before the last cdr
        ret = append_before_nil(ret, sub);
    }
    if ret.t == TspType::TspPair && nilp(cdr(&ret)) { return Some(clone_val(car(&ret))); }
    Some(ret)
}

fn append_before_nil(lst: Val, item: Val) -> Val {
    // Rebuild list with item inserted before the final nil cdr of the last pair
    if lst.t != TspType::TspPair { return mk_pair(item, lst); }
    let c = clone_val(car(&lst));
    let d = clone_val(cdr(&lst));
    if d.t == TspType::TspPair {
        mk_pair(c, append_before_nil(d, item))
    } else {
        mk_pair(c, mk_pair(item, d))
    }
}

// ---- eval ----
pub fn tsp_lstlen(v: &Val) -> i32 {
    let mut len = 0i32; let mut cur = v;
    while cur.t == TspType::TspPair { len += 1; cur = cdr(cur); }
    if nilp(cur) { len } else { -(len + 1) }
}

pub fn tsp_arg_num_check_pub(args: &Val, name: &str, nargs: i32) -> Option<()> {
    if nargs > -1 && tsp_lstlen(args) != nargs {
        eprintln!("; tisp: error: {}: expected {} argument{}, received {}", name, nargs, if nargs > 1 { "s" } else { "" }, tsp_lstlen(args));
        return None;
    }
    Some(())
}
pub fn tsp_arg_min_check_pub(args: &Val, name: &str, nargs: i32) -> Option<()> {
    if tsp_lstlen(args) < nargs {
        eprintln!("; tisp: error: {}: expected at least {} argument{}, received {}", name, nargs, if nargs > 1 { "s" } else { "" }, tsp_lstlen(args));
        return None;
    }
    Some(())
}
fn tsp_arg_max_check(args: &Val, name: &str, nargs: i32) -> Option<()> {
    if tsp_lstlen(args) > nargs {
        eprintln!("; tisp: error: {}: expected at no more than {} argument{}, received {}", name, nargs, if nargs > 1 { "s" } else { "" }, tsp_lstlen(args));
        return None;
    }
    Some(())
}
pub fn tsp_arg_type_check_pub(arg: &Val, name: &str, type_mask: u32) -> Option<()> {
    if (arg.t as u32 & type_mask) == 0 {
        let expected = match type_mask {
            m if m == TspType::TspPair as u32 => "Pair",
            m if m == TspType::TspSym as u32 => "Sym",
            m if m == TspType::TspStr as u32 => "Str",
            m if m == TspType::TspInt as u32 => "Int",
            m if m == TspType::TspRec as u32 => "Rec",
            m if m == TSP_NUM => "Num",
            m if m == TSP_EXPR => "Expr",
            m if m == TSP_RATIONAL => "Rational",
            m if m == (TspType::TspStr as u32 | TspType::TspSym as u32) => "Str",
            m if m == (TspType::TspInt as u32 | TspType::TspRatio as u32) => "Rational",
            _ => tsp_type_str_mask_pub(type_mask),
        };
        eprintln!("; tisp: error: {}: expected {}, received {}", name, expected, tsp_type_str(arg.t));
        return None;
    }
    Some(())
}
pub fn tsp_arg_max_check_pub(args: &Val, name: &str, nargs: i32) -> Option<()> { tsp_arg_max_check(args, name, nargs) }

pub fn tisp_eval_list(st: &mut Tsp, env: &mut Rec, v: Val) -> Option<Val> {
    let mut items: Vec<Val> = Vec::new();
    let mut cur = v;
    while !nilp(&cur) {
        if cur.t != TspType::TspPair {
            let ev = tisp_eval_with_env(st, env, cur)?;
            let mut result = ev;
            for item in items.into_iter().rev() { result = mk_pair(item, result); }
            return Some(result);
        }
        let c = clone_val(car(&cur));
        let ev = tisp_eval_with_env(st, env, c)?;
        items.push(ev);
        cur = clone_val(cdr(&cur));
    }
    let mut result = mk_nil();
    for item in items.into_iter().rev() { result = mk_pair(item, result); }
    Some(result)
}
pub fn tisp_eval_list_pub(st: &mut Tsp, env: &mut Rec, v: Val) -> Option<Val> { tisp_eval_list(st, env, v) }

pub fn tisp_eval_body(st: &mut Tsp, env: &mut Rec, v: Val) -> Option<Val> {
    tisp_eval_body_inner(st, env, v, None)
}
fn tisp_eval_body_inner(st: &mut Tsp, env: &mut Rec, v: Val, mut owned_env: Option<Rec>) -> Option<Val> {
    let mut ret = mk_none();
    let mut cur = v;
    loop {
        if cur.t != TspType::TspPair { return Some(ret); }
        let is_last = nilp(cdr(&cur));
        let cur_car = clone_val(car(&cur));
        if is_last && cur_car.t == TspType::TspPair {
            let f_expr = clone_val(car(&cur_car));
            let f_args = clone_val(cdr(&cur_car));
            let e = owned_env.as_mut().map_or(&mut *env, |e| e);
            let f = tisp_eval_with_env(st, e, f_expr)?;
            if f.t != TspType::TspFunc {
                return eval_proc(st, e, f, f_args);
            }
            // Tail call
            let (fname, fargs, fbody, fenv) = extract_func(&f);
            let e2 = owned_env.as_mut().map_or(&mut *env, |e| e);
            tsp_arg_num_check_pub(&f_args, &fname, tsp_lstlen(&fargs))?;
            let evaled = tisp_eval_list(st, e2, f_args)?;
            let new_env = rec_extend(fenv, fargs, evaled);
            owned_env = Some(new_env);
            cur = mk_pair(mk_nil(), fbody);
            continue;
        }
        let e = owned_env.as_mut().map_or(&mut *env, |e| e);
        ret = tisp_eval_with_env(st, e, cur_car)?;
        cur = clone_val(cdr(&cur));
    }
}
pub fn tisp_eval_body_pub(st: &mut Tsp, env: &mut Rec, v: Val) -> Option<Val> { tisp_eval_body(st, env, v) }

fn extract_func(f: &Val) -> (String, Val, Val, Rec) {
    if let ValUnion::F { name, args, body, env } = &f.v {
        (if name.is_empty() { "anon".into() } else { name.clone() },
         clone_val(args), clone_val(body), clone_rec(env))
    } else { panic!("not a func") }
}

pub fn prepend_bt(st: &mut Tsp, env: &mut Rec, f: Val) {
    if let ValUnion::F { ref name, .. } = f.v {
        if name.is_empty() { return; }
        let mut r = &mut *env;
        while r.next.is_some() { r = r.next.as_mut().unwrap(); }
        if let Some(e) = entry_get(r, "bt") {
            if e.val.t == TspType::TspPair && car(&e.val).t == TspType::TspSym {
                if let ValUnion::S(ref s) = car(&e.val).v {
                    if name.starts_with(s.as_str()) { return; }
                }
            }
        }
        let sym = mk_sym(st, name);
        if let Some(old_bt) = rec_get(r, "bt") {
            rec_add(r, "bt", mk_pair(sym, old_bt));
        }
    }
}

pub fn eval_proc(st: &mut Tsp, env: &mut Rec, f: Val, args: Val) -> Option<Val> {
    match f.t {
        TspType::TspPrim => {
            let evaled = tisp_eval_list(st, env, args)?;
            if let ValUnion::Pr { pr, .. } = &f.v {
                let pr = *pr;
                let r = pr(st, env, evaled);
                if is_error(&r) { None } else { Some(r) }
            } else { None }
        }
        TspType::TspForm => {
            if let ValUnion::Pr { pr, .. } = &f.v {
                let pr = *pr;
                let r = pr(st, env, args);
                if is_error(&r) { None } else { Some(r) }
            } else { None }
        }
        TspType::TspFunc => {
            let evaled = tisp_eval_list(st, env, args)?;
            let (fname, fargs, fbody, fenv) = extract_func(&f);
            tsp_arg_num_check_pub(&evaled, &fname, tsp_lstlen(&fargs))?;
            let new_env = rec_extend(fenv, fargs, evaled);
            let mut ne = new_env;
            let result = tisp_eval_body(st, &mut ne, fbody);
            if result.is_none() { prepend_bt(st, env, f); }
            result
        }
        TspType::TspMacro => {
            let (fname, fargs, fbody, fenv) = extract_func(&f);
            tsp_arg_num_check_pub(&args, &fname, tsp_lstlen(&fargs))?;
            let new_env = rec_extend(fenv, fargs, args);
            let mut ne = new_env;
            let result = tisp_eval_body(st, &mut ne, fbody);
            if result.is_none() { prepend_bt(st, env, f); return None; }
            tisp_eval_with_env(st, env, result.unwrap())
        }
        TspType::TspRec => {
            let evaled = tisp_eval_list(st, env, args)?;
            tsp_arg_num_check_pub(&evaled, "record", 1)?;
            tsp_arg_type_check_pub(car(&evaled), "record", TspType::TspSym as u32)?;
            let key = sym_str(car(&evaled)).to_string();
            if let ValUnion::R(ref r) = f.v {
                if let Some(v) = rec_get(r, &key) { return Some(v); }
                if let Some(v) = rec_get(r, "else") { return Some(v); }
            }
            eprintln!("; tisp: error: could not find element '{}' in record", key);
            None
        }
        _ => { eprintln!("; tisp: error: attempt to evaluate non procedural type {}", tsp_type_str(f.t)); None }
    }
}
pub fn eval_proc_pub(st: &mut Tsp, env: &mut Rec, f: Val, args: Val) -> Option<Val> { eval_proc(st, env, f, args) }

pub fn tisp_eval(st: &mut Tsp, v: Val) -> Option<Val> {
    match v.t {
        TspType::TspSym => {
            let s = sym_str(&v).to_string();
            rec_get(&st.env, &s).or_else(|| { eprintln!("; tisp: error: could not find symbol '{}'", s); None })
        }
        TspType::TspPair => {
            let cv = clone_val(car(&v)); let dv = clone_val(cdr(&v));
            let f = tisp_eval(st, cv)?;
            let mut env = std::mem::replace(&mut st.env, rec_new(0, None));
            let result = eval_proc(st, &mut env, f, dv);
            st.env = env;
            result
        }
        _ => Some(v),
    }
}

pub fn tisp_eval_with_env_pub(st: &mut Tsp, env: &mut Rec, v: Val) -> Option<Val> { tisp_eval_with_env(st, env, v) }
fn tisp_eval_with_env(st: &mut Tsp, env: &mut Rec, v: Val) -> Option<Val> {
    match v.t {
        TspType::TspSym => {
            let s = sym_str(&v).to_string();
            rec_get(env, &s).or_else(|| { eprintln!("; tisp: error: could not find symbol '{}'", s); None })
        }
        TspType::TspPair => {
            let cv = clone_val(car(&v)); let dv = clone_val(cdr(&v));
            let f = tisp_eval_with_env(st, env, cv)?;
            eval_proc(st, env, f, dv)
        }
        _ => Some(v),
    }
}

// ---- print ----
fn format_dec(n: f64) -> String {
    // Mimic C's %.15g behavior
    let s = format!("{:.15e}", n);
    // Parse mantissa and exponent
    let parts: Vec<&str> = s.split('e').collect();
    let mantissa = parts[0];
    let exp: i32 = parts[1].parse().unwrap();
    // Trim trailing zeros from mantissa
    let mantissa = mantissa.trim_end_matches('0');
    let mantissa = mantissa.trim_end_matches('.');
    // Reconstruct
    if exp == 0 {
        if mantissa.contains('.') { mantissa.to_string() }
        else { mantissa.to_string() }
    } else if exp > 0 && exp < 16 {
        // Try to represent without exponent if possible
        let digits: String = mantissa.replace('.', "");
        let dot_pos = if mantissa.contains('.') {
            mantissa.find('.').unwrap()
        } else {
            mantissa.len()
        };
        let new_dot = dot_pos as i32 + exp;
        if new_dot >= digits.len() as i32 {
            // All digits before decimal, pad with zeros
            let mut r = digits.clone();
            for _ in 0..(new_dot as usize - digits.len()) { r.push('0'); }
            r
        } else if new_dot <= 0 {
            let mut r = "0.".to_string();
            for _ in 0..(-new_dot) { r.push('0'); }
            r.push_str(&digits);
            r
        } else {
            let (left, right) = digits.split_at(new_dot as usize);
            if right.is_empty() { left.to_string() }
            else { format!("{}.{}", left, right) }
        }
    } else if exp < 0 && exp > -5 {
        let digits: String = mantissa.replace('.', "");
        let dot_pos = if mantissa.contains('.') { mantissa.find('.').unwrap() } else { mantissa.len() };
        let new_dot = dot_pos as i32 + exp;
        if new_dot <= 0 {
            let mut r = "0.".to_string();
            for _ in 0..(-new_dot) { r.push('0'); }
            r.push_str(&digits);
            r
        } else {
            let (left, right) = digits.split_at(new_dot as usize);
            format!("{}.{}", left, right)
        }
    } else {
        // Use e notation
        if mantissa.contains('.') {
            format!("{}e{:+03}", mantissa, exp)
        } else {
            format!("{}e{:+03}", mantissa, exp)
        }
    }
}

pub fn val_to_string_pub(v: &Val) -> String { val_to_string(v) }
fn val_to_string(v: &Val) -> String {
    match v.t {
        TspType::TspNone => "Void".into(),
        TspType::TspNil => "Nil".into(),
        TspType::TspInt => format!("{}", num(v) as i32),
        TspType::TspDec => {
            let n = num(v);
            let s = format_dec(n);
            if n == (n as i32) as f64 && !s.contains('e') && !s.contains('E') {
                format!("{}.0", s.trim_end_matches(".0"))
            } else { s }
        }
        TspType::TspRatio => format!("{}/{}", num(v) as i32, den(v) as i32),
        TspType::TspStr | TspType::TspSym => sym_str(v).to_string(),
        TspType::TspFunc | TspType::TspMacro => {
            if let ValUnion::F { ref name, .. } = v.v {
                let tn = if v.t == TspType::TspFunc { "function" } else { "macro" };
                if name.is_empty() { format!("#<{}>", tn) } else { format!("#<{}:{}>", tn, name) }
            } else { String::new() }
        }
        TspType::TspPrim => { if let ValUnion::Pr { ref name, .. } = v.v { format!("#<primitive:{}>", name) } else { String::new() } }
        TspType::TspForm => { if let ValUnion::Pr { ref name, .. } = v.v { format!("#<form:{}>", name) } else { String::new() } }
        TspType::TspRec => {
            let mut s = "{".to_string();
            if let ValUnion::R(ref r) = v.v {
                let mut rec = Some(r);
                while let Some(cur) = rec {
                    let mut c = 0;
                    for e in &cur.items {
                        if !e.key.is_empty() {
                            c += 1;
                            s.push_str(&format!(" {}: ", e.key));
                            s.push_str(&val_to_string(&e.val));
                            if c >= TSP_REC_MAX_PRINT { s.push_str(" ..."); break; }
                        }
                    }
                    rec = cur.next.as_deref();
                }
            }
            s.push_str(" }"); s
        }
        TspType::TspPair => {
            let mut s = "(".to_string();
            s.push_str(&val_to_string(car(v)));
            let mut cur = cdr(v);
            while !nilp(cur) {
                if cur.t == TspType::TspPair {
                    s.push(' '); s.push_str(&val_to_string(car(cur))); cur = cdr(cur);
                } else { s.push_str(" . "); s.push_str(&val_to_string(cur)); break; }
            }
            s.push(')'); s
        }
    }
}

pub fn tisp_print(f: &mut std::fs::File, v: &Val) { let _ = f.write_all(val_to_string(v).as_bytes()); }

// ---- env ----
pub fn tisp_env_add(st: &mut Tsp, key: &str, v: Val) { rec_add(&mut st.env, key, v); }
pub fn tisp_env_init(cap: usize) -> Tsp {
    let nil = mk_nil(); let none = mk_none();
    let t_val = Val { t: TspType::TspSym, v: ValUnion::S("True".into()) };
    let mut st = Tsp {
        file: String::new(), filec: 0,
        none: clone_val(&none), nil: clone_val(&nil), t: clone_val(&t_val),
        env: rec_new(cap, None), strs: rec_new(cap, None), syms: rec_new(cap, None),
        libh: Vec::new(), libhc: 0,
    };
    tisp_env_add(&mut st, "True", clone_val(&t_val));
    tisp_env_add(&mut st, "Nil", clone_val(&nil));
    tisp_env_add(&mut st, "Void", clone_val(&none));
    tisp_env_add(&mut st, "bt", mk_nil());
    let ver = mk_str(&mut st, "0.1");
    tisp_env_add(&mut st, "version", ver);
    st
}
pub fn tisp_env_lib(st: &mut Tsp, lib: &str) {
    let old_file = std::mem::replace(&mut st.file, lib.to_string());
    let old_filec = std::mem::replace(&mut st.filec, 0);
    skip_ws(st, 1);
    if let Some(v) = tisp_read(st) {
        let mut env = std::mem::replace(&mut st.env, rec_new(0, None));
        let _ = tisp_eval_body(st, &mut env, v);
        st.env = env;
    }
    st.file = old_file; st.filec = old_filec;
}

pub fn tib_env_core(st: &mut Tsp) { crate::core::tib_env_core(st); }
pub fn tib_env_math(st: &mut Tsp) { crate::math::tib_env_math(st); }
pub fn tib_env_string(st: &mut Tsp) { crate::string::tib_env_string(st); }
pub fn tib_env_io(st: &mut Tsp) { crate::io::tib_env_io(st); }
pub fn tib_env_os(st: &mut Tsp) { crate::os::tib_env_os(st); }
