use std::io::Write;

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
pub fn is_err_val(v: &Val) -> bool {
    matches!(v.t, TspType::TspNone) && matches!(&v.v, ValUnion::S(s) if s == "__err")
}
pub fn mk_err() -> Val { Val { t: TspType::TspNone, v: ValUnion::S("__err".into()) } }

pub fn clone_val(v: &Val) -> Val { Val { t: v.t, v: clone_vu(&v.v) } }
fn clone_vu(v: &ValUnion) -> ValUnion {
    match v {
        ValUnion::S(s) => ValUnion::S(s.clone()),
        ValUnion::N { num, den } => ValUnion::N { num: *num, den: *den },
        ValUnion::Pr { name, pr } => ValUnion::Pr { name: name.clone(), pr: *pr },
        ValUnion::F { name, args, body, env } => ValUnion::F {
            name: name.clone(), args: Box::new(clone_val(args)),
            body: Box::new(clone_val(body)), env: clone_rec(env),
        },
        ValUnion::P { car, cdr } => ValUnion::P {
            car: Box::new(clone_val(car)), cdr: Box::new(clone_val(cdr)),
        },
        ValUnion::R(r) => ValUnion::R(clone_rec(r)),
    }
}
pub fn clone_rec(r: &Rec) -> Rec {
    Rec {
        size: r.size, cap: r.cap,
        items: r.items.iter().map(|e| Entry { key: e.key.clone(), val: clone_val(&e.val) }).collect(),
        next: r.next.as_ref().map(|n| Box::new(clone_rec(n))),
    }
}

pub fn car(v: &Val) -> &Val { match &v.v { ValUnion::P { car, .. } => car, _ => panic!("car on non-pair") } }

// Emulate C's %.15g format
pub fn format_g(val: f64, precision: usize) -> String {
    // C's %g: use exponential if exponent < -4 or >= precision, else fixed
    let s = format!("{:.*e}", precision - 1, val);
    let parts: Vec<&str> = s.split('e').collect();
    if parts.len() == 2 {
        let exp: i32 = parts[1].parse().unwrap_or(0);
        if exp >= -4 && exp < precision as i32 {
            let dec_places = if precision as i32 - 1 - exp > 0 { (precision as i32 - 1 - exp) as usize } else { 0 };
            let fixed = format!("{:.*}", dec_places, val);
            if fixed.contains('.') {
                let trimmed = fixed.trim_end_matches('0').trim_end_matches('.');
                return trimmed.to_string();
            }
            return fixed;
        }
    }
    // Use exponential form, but format like C's %g
    let parts: Vec<&str> = s.split('e').collect();
    if parts.len() == 2 {
        let mantissa = parts[0].trim_end_matches('0').trim_end_matches('.');
        let exp: i32 = parts[1].parse().unwrap_or(0);
        if exp == 0 {
            return mantissa.to_string();
        }
        return format!("{}e{:+03}", mantissa, exp);
    }
    s
}
pub fn cdr(v: &Val) -> &Val { match &v.v { ValUnion::P { cdr, .. } => cdr, _ => panic!("cdr on non-pair") } }
pub fn nilp(v: &Val) -> bool { v.t == TspType::TspNil }
pub fn vnum(v: &Val) -> f64 { match &v.v { ValUnion::N { num, .. } => *num, _ => 0.0 } }
pub fn vden(v: &Val) -> f64 { match &v.v { ValUnion::N { den, .. } => *den, _ => 1.0 } }
pub fn vs(v: &Val) -> &str { match &v.v { ValUnion::S(s) => s, _ => "" } }

fn tsp_fget(st: &Tsp) -> Option<u8> {
    st.file.as_bytes().get(st.filec).copied()
}
fn tsp_fgetat(st: &Tsp, off: isize) -> Option<u8> {
    let idx = st.filec as isize + off;
    if idx < 0 { return None; }
    st.file.as_bytes().get(idx as usize).copied()
}

// ---- type str ----
pub fn tsp_type_str(t: TspType) -> &'static str {
    match t {
        TspType::TspNone => "Void", TspType::TspNil => "Nil",
        TspType::TspInt => "Int", TspType::TspDec => "Dec", TspType::TspRatio => "Ratio",
        TspType::TspStr => "Str", TspType::TspSym => "Sym",
        TspType::TspPrim => "Prim", TspType::TspForm => "Form",
        TspType::TspFunc => "Func", TspType::TspMacro => "Macro",
        TspType::TspPair => "Pair", TspType::TspRec => "Rec",
    }
}
pub fn tsp_type_str_bits(t: u32) -> &'static str {
    if t == TSP_EXPR { return "Expr"; }
    if t == TSP_RATIONAL { return "Rational"; }
    if t & TSP_NUM != 0 { return "Num"; }
    "Invalid"
}

pub fn is_sym(c: char) -> bool { c.is_ascii_alphanumeric() || TSP_SYM_CHARS.contains(c) }
pub fn is_op(c: char) -> bool { TSP_OP_CHARS.contains(c) }

pub fn isnum(s: &str) -> bool {
    let b = s.as_bytes();
    if b.is_empty() { return false; }
    if b[0].is_ascii_digit() { return true; }
    if b[0] == b'.' && b.len() > 1 && b[1].is_ascii_digit() { return true; }
    if (b[0] == b'-' || b[0] == b'+') && b.len() > 1 && (b[1].is_ascii_digit() || b[1] == b'.') { return true; }
    false
}

pub fn skip_ws(st: &mut Tsp, skipnl: i32) {
    let bytes = st.file.as_bytes();
    let len = bytes.len();
    loop {
        if st.filec >= len { break; }
        let c = bytes[st.filec];
        if c == b' ' || c == b'\t' || (skipnl != 0 && (c == b'\n' || c == b'\r')) {
            st.filec += 1; continue;
        }
        if c == b';' {
            while st.filec < len && bytes[st.filec] != b'\n' { st.filec += 1; }
            if skipnl == 0 { /* don't skip newline */ }
            else if st.filec < len { st.filec += 1; }
            continue;
        }
        break;
    }
}

pub fn tsp_lstlen(v: &Val) -> i32 {
    let mut len = 0i32;
    let mut cur = v;
    while cur.t == TspType::TspPair { len += 1; cur = cdr(cur); }
    if nilp(cur) { len } else { -(len + 1) }
}

pub fn vals_eq(a: &Val, b: &Val) -> bool {
    let at = a.t as u32; let bt = b.t as u32;
    if at & TSP_NUM != 0 && bt & TSP_NUM != 0 {
        return vnum(a) == vnum(b) && vden(a) == vden(b);
    }
    if a.t != b.t { return false; }
    if a.t == TspType::TspPair { return vals_eq(car(a), car(b)) && vals_eq(cdr(a), cdr(b)); }
    if at & (TspType::TspFunc as u32 | TspType::TspMacro as u32) != 0 {
        if let (ValUnion::F { args: aa, body: ab, .. }, ValUnion::F { args: ba, body: bb, .. }) = (&a.v, &b.v) {
            return vals_eq(aa, ba) && vals_eq(ab, bb);
        }
    }
    match (&a.v, &b.v) {
        (ValUnion::S(sa), ValUnion::S(sb)) => sa == sb,
        (ValUnion::Pr { name: na, .. }, ValUnion::Pr { name: nb, .. }) => na == nb,
        _ => matches!(a.t, TspType::TspNil | TspType::TspNone),
    }
}

pub fn frac_reduce(num: &mut i32, den: &mut i32) {
    let mut a = num.unsigned_abs(); let mut b = den.unsigned_abs();
    if b == 0 { return; }
    let mut c = a % b;
    while c > 0 { a = b; b = c; c = a % b; }
    *num /= b as i32; *den /= b as i32;
}

pub fn hash(key: &str) -> u32 {
    let mut h: u32 = 0;
    for c in key.bytes() { h = h.wrapping_mul(33).wrapping_add(c as u32); }
    h
}

pub fn rec_new(cap: usize, next: Option<Box<Rec>>) -> Rec {
    let cap = if cap == 0 { 1 } else { cap };
    let mut items = Vec::with_capacity(cap);
    for _ in 0..cap { items.push(Entry { key: String::new(), val: mk_val(TspType::TspNone) }); }
    Rec { size: 0, cap: cap as i32, items, next }
}

pub fn entry_get_idx(rec: &Rec, key: &str) -> usize {
    let cap = rec.cap as usize;
    if cap == 0 { return 0; }
    let mut i = (hash(key) as usize) % cap;
    loop {
        if rec.items[i].key.is_empty() || rec.items[i].key == key { return i; }
        i = (i + 1) % cap;
    }
}

pub fn entry_get<'a>(rec: &'a Rec, key: &'a str) -> Option<&'a Entry> {
    let i = entry_get_idx(rec, key);
    if i < rec.items.len() && !rec.items[i].key.is_empty() { Some(&rec.items[i]) } else { None }
}

pub fn rec_get(rec: &Rec, key: &str) -> Option<Val> {
    let mut cur = Some(rec);
    while let Some(r) = cur {
        let i = entry_get_idx(r, key);
        if i < r.items.len() && !r.items[i].key.is_empty() { return Some(clone_val(&r.items[i].val)); }
        cur = r.next.as_deref();
    }
    None
}

pub fn rec_grow(rec: &mut Rec) {
    let old_items: Vec<Entry> = std::mem::take(&mut rec.items);
    rec.cap *= TSP_REC_FACTOR as i32;
    let new_cap = rec.cap as usize;
    rec.items = Vec::with_capacity(new_cap);
    for _ in 0..new_cap { rec.items.push(Entry { key: String::new(), val: mk_val(TspType::TspNone) }); }
    rec.size = 0;
    for e in old_items {
        if !e.key.is_empty() { let k = e.key.clone(); rec_add(rec, &k, e.val); }
    }
}

pub fn rec_add(rec: &mut Rec, key: &str, val: Val) {
    let cap = rec.cap as usize;
    if cap == 0 { return; }
    let i = entry_get_idx(rec, key);
    rec.items[i].val = val;
    if rec.items[i].key.is_empty() {
        rec.items[i].key = key.to_string();
        rec.size += 1;
        if rec.size > rec.cap / TSP_REC_FACTOR as i32 { rec_grow(rec); }
    }
}

pub fn rec_extend(rec: &mut Rec, args: Val, vals: Val) -> Rec {
    let argnum = TSP_REC_FACTOR as i32 * tsp_lstlen(&args);
    let cap = if argnum > 0 { argnum as usize } else { (-argnum + 1) as usize };
    let mut ret = rec_new(cap, Some(Box::new(clone_rec(rec))));
    let mut a = &args; let mut v = &vals;
    loop {
        if nilp(a) { break; }
        let (arg, val) = if a.t == TspType::TspPair { (car(a), car(v)) } else { (a, v) };
        if arg.t != TspType::TspSym {
            eprintln!("; tisp: error: expected symbol for argument of function definition, received '{}'", tsp_type_str(arg.t));
        }
        rec_add(&mut ret, vs(arg), clone_val(val));
        if a.t != TspType::TspPair { break; }
        a = cdr(a); v = cdr(v);
    }
    ret
}

// ---- make types ----
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

pub fn mk_str(st: &mut Tsp, s: &str) -> Option<Val> {
    if let Some(v) = rec_get(&st.strs, s) { return Some(v); }
    let ret = Val { t: TspType::TspStr, v: ValUnion::S(s.to_string()) };
    rec_add(&mut st.strs, s, clone_val(&ret));
    Some(ret)
}
pub fn mk_str_val(st: &mut Tsp, s: &str) -> Val { mk_str(st, s).unwrap() }

pub fn mk_sym(st: &mut Tsp, s: &str) -> Option<Val> {
    if let Some(v) = rec_get(&st.syms, s) { return Some(v); }
    let ret = Val { t: TspType::TspSym, v: ValUnion::S(s.to_string()) };
    rec_add(&mut st.syms, s, clone_val(&ret));
    Some(ret)
}
pub fn mk_sym_val(st: &mut Tsp, s: &str) -> Val { mk_sym(st, s).unwrap() }

pub fn mk_prim(t: TspType, pr: Prim, name: &str) -> Option<Val> {
    Some(Val { t, v: ValUnion::Pr { name: name.to_string(), pr } })
}

pub fn mk_func(t: TspType, name: &str, args: Val, body: Val, env: Rec) -> Option<Val> {
    Some(Val { t, v: ValUnion::F { name: name.to_string(), args: Box::new(args), body: Box::new(body), env } })
}

pub fn mk_rec(st: &mut Tsp, env: Rec, assoc: Val) -> Option<Val> {
    // If assoc is nil (no associations), just wrap env
    if nilp(&assoc) {
        return Some(Val { t: TspType::TspRec, v: ValUnion::R(env) });
    }
    let lstlen = tsp_lstlen(&assoc);
    let cap = TSP_REC_FACTOR * (if lstlen > 0 { lstlen } else { -lstlen + 1 }) as usize;
    let mut rec = rec_new(cap, None);
    let mut r = rec_new(4, Some(Box::new(env)));
    // Add "this" placeholder
    rec_add(&mut r, "this", Val { t: TspType::TspRec, v: ValUnion::R(rec_new(1, None)) });

    let mut cur = &assoc;
    while cur.t == TspType::TspPair {
        let c = car(cur);
        if c.t == TspType::TspPair {
            let key = car(c);
            let kt = key.t as u32;
            if kt & (TspType::TspSym as u32 | TspType::TspStr as u32) != 0 {
                let body_val = car(cdr(c));
                let v = tisp_eval_with_env(st, &mut r, clone_val(body_val))?;
                rec_add(&mut rec, vs(key), v);
            } else {
                eprintln!("; tisp: error: Rec: missing key symbol or string");
                return None;
            }
        } else if c.t == TspType::TspSym {
            let v = tisp_eval_with_env(st, &mut r, clone_val(c))?;
            rec_add(&mut rec, vs(c), v);
        } else {
            eprintln!("; tisp: error: Rec: missing key symbol or string");
            return None;
        }
        cur = cdr(cur);
    }
    Some(Val { t: TspType::TspRec, v: ValUnion::R(rec) })
}

pub fn mk_rec_prim(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    let env_clone = clone_rec(env);
    mk_rec(st, env_clone, args).unwrap_or_else(|| mk_err())
}

pub fn mk_pair(a: Val, b: Val) -> Option<Val> {
    Some(Val { t: TspType::TspPair, v: ValUnion::P { car: Box::new(a), cdr: Box::new(b) } })
}

pub fn mk_list(st: &mut Tsp, _n: i32, args: Vec<Val>) -> Option<Val> {
    if args.is_empty() { return Some(clone_val(&st.nil)); }
    let nil = clone_val(&st.nil);
    let mut result = nil;
    for arg in args.into_iter().rev() {
        result = mk_pair(arg, result)?;
    }
    Some(result)
}

// mk_list_from_vec is same as mk_list
pub fn mk_list_from_vec(st: &mut Tsp, args: Vec<Val>) -> Option<Val> {
    mk_list(st, args.len() as i32, args)
}

// ---- read ----
pub fn read_sign(st: &mut Tsp) -> i32 {
    match tsp_fget(st) {
        Some(b'-') => { st.filec += 1; -1 }
        Some(b'+') => { st.filec += 1; 1 }
        _ => 1,
    }
}

pub fn read_int(st: &mut Tsp) -> i32 {
    let mut ret = 0i32;
    while let Some(c) = tsp_fget(st) {
        if !c.is_ascii_digit() { break; }
        ret = ret * 10 + (c - b'0') as i32;
        st.filec += 1;
    }
    ret
}

pub fn read_sci(st: &mut Tsp, val: f64, isint: i32) -> Option<Val> {
    let mut val = val;
    if let Some(c) = tsp_fget(st) {
        if c == b'e' || c == b'E' {
            st.filec += 1;
            let sign: f64 = if read_sign(st) == 1 { 10.0 } else { 0.1 };
            let expo = read_int(st);
            for _ in 0..expo { val *= sign; }
        }
    }
    if isint != 0 { Some(mk_int(val as i32)) } else { mk_dec(val) }
}

pub fn read_num(st: &mut Tsp) -> Val {
    let sign = read_sign(st);
    let n = read_int(st);
    match tsp_fget(st) {
        Some(b'/') => {
            st.filec += 1;
            if !isnum(&st.file[st.filec..]) {
                eprintln!("; tisp: error: incorrect ratio format, no denominator found");
                return mk_err();
            }
            let ds = read_sign(st);
            let di = read_int(st);
            mk_rat(sign * n, ds * di).unwrap_or_else(|| mk_err())
        }
        Some(b'.') => {
            st.filec += 1;
            let oldc = st.filec;
            let d = read_int(st) as f64;
            let size = st.filec - oldc;
            let mut frac = d;
            for _ in 0..size { frac /= 10.0; }
            read_sci(st, sign as f64 * (n as f64 + frac), 0).unwrap_or_else(|| mk_err())
        }
        _ => read_sci(st, sign as f64 * n as f64, 1).unwrap_or_else(|| mk_err()),
    }
}

pub fn esc_char(c: char) -> char {
    match c {
        'n' => '\n', 'r' => '\r', 't' => '\t', '\n' => ' ',
        _ => c,
    }
}

pub fn esc_str(s: &str, len: i32, do_esc: i32) -> String {
    let bytes = s.as_bytes();
    let mut ret = String::new();
    let mut i = 0usize;
    let mut count = 0;
    while count < len as usize && i < bytes.len() {
        if bytes[i] == b'\\' && do_esc != 0 {
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

pub fn read_str(st: &mut Tsp, mk_fn: fn(&mut Tsp, &str) -> Val) -> Option<Val> {
    st.filec += 1; // skip opening quote
    let start = st.filec;
    let is_str = {
        // Determine endchar based on whether mk_fn produces strings or symbols
        // In C: endchar = mk_fn == &mk_str ? '"' : '~'
        // We check by calling with a test... actually we need another approach
        // Let's check the byte before start (the opening char)
        let prev = st.file.as_bytes().get(start - 1).copied();
        prev == Some(b'"')
    };
    let endchar = if is_str { b'"' } else { b'~' };
    let mut len = 0i32;
    let bytes = st.file.as_bytes();
    while st.filec < bytes.len() && bytes[st.filec] != endchar {
        if bytes[st.filec] == b'\\' && (st.filec == 0 || bytes[st.filec - 1] != b'\\') {
            st.filec += 1;
        }
        st.filec += 1;
        len += 1;
    }
    if st.filec >= bytes.len() {
        eprintln!("; tisp: error: reached end before closing {}", endchar as char);
        return None;
    }
    st.filec += 1; // skip closing quote
    let s_slice = &st.file[start..st.filec - 1];
    let escaped = esc_str(s_slice, len, if is_str { 1 } else { 0 });
    Some(mk_fn(st, &escaped))
}

pub fn read_sym(st: &mut Tsp, is_char: fn(char) -> bool) -> Option<Val> {
    let start = st.filec;
    let bytes = st.file.as_bytes();
    while st.filec < bytes.len() && is_char(bytes[st.filec] as char) {
        st.filec += 1;
    }
    let s = esc_str(&st.file[start..st.filec], (st.filec - start) as i32, 0);
    mk_sym(st, &s)
}

pub fn read_pair(st: &mut Tsp, endchar: char) -> Option<Val> {
    let skipnl = if endchar != '\n' { 1 } else { 0 };
    skip_ws(st, skipnl);
    let nil = clone_val(&st.nil);
    // Collect elements into a vec, then build list
    let mut elements: Vec<Val> = Vec::new();
    let mut improper_end: Option<Val> = None;

    while let Some(c) = tsp_fget(st) {
        if c == endchar as u8 { break; }
        let v = tisp_read(st)?;
        // Check for dot (improper list)
        if v.t == TspType::TspSym && vs(&v) == "." {
            skip_ws(st, skipnl);
            let end_v = tisp_read(st)?;
            improper_end = Some(end_v);
            break;
        }
        elements.push(v);
        skip_ws(st, skipnl);
    }
    skip_ws(st, skipnl);
    if skipnl != 0 {
        if let Some(c) = tsp_fget(st) {
            if c != endchar as u8 {
                eprintln!("; tisp: error: did not find closing '{}'", endchar);
                return None;
            }
        } else {
            eprintln!("; tisp: error: did not find closing '{}'", endchar);
            return None;
        }
    }
    st.filec += 1; // skip endchar

    // Build list from elements
    let tail = improper_end.unwrap_or(nil);
    let mut result = tail;
    for elem in elements.into_iter().rev() {
        result = mk_pair(elem, result)?;
    }
    Some(result)
}

pub fn tisp_read_sexpr(st: &mut Tsp) -> Option<Val> {
    let prefixes: &[(&str, &str)] = &[
        ("'", "quote"), ("`", "quasiquote"),
        (",@", "unquote-splice"), (",", "unquote"),
        ("@", "Func"), ("f\"", "strformat"),
    ];
    skip_ws(st, 1);
    if st.filec >= st.file.len() { return Some(clone_val(&st.none)); }

    let remaining = &st.file[st.filec..];
    if remaining.is_empty() { return Some(clone_val(&st.none)); }

    if isnum(remaining) { return Some(read_num(st)); }

    if tsp_fget(st) == Some(b'"') { return read_str(st, mk_str_val); }
    if tsp_fget(st) == Some(b'~') { return read_str(st, mk_sym_val); }

    for i in (0..prefixes.len()).step_by(1) {
        let (prefix, sym_name) = prefixes[i];
        if remaining.starts_with(prefix) {
            let skip = prefix.len() - if prefix.ends_with('"') { 1 } else { 0 };
            st.filec += skip;
            let v = tisp_read(st)?;
            let sym = mk_sym(st, sym_name)?;
            return mk_list(st, 2, vec![sym, v]);
        }
    }

    if let Some(c) = tsp_fget(st) {
        let ch = c as char;
        if is_op(ch) { return read_sym(st, is_op); }
        if is_sym(ch) { return read_sym(st, is_sym); }
        if ch == '(' { st.filec += 1; return read_pair(st, ')'); }
        if ch == '[' {
            st.filec += 1;
            let lst = read_pair(st, ']')?;
            let sym = mk_sym(st, "list")?;
            return mk_pair(sym, lst);
        }
        if ch == '{' {
            st.filec += 1;
            let v = read_pair(st, '}')?;
            let sym = mk_sym(st, "Rec")?;
            return mk_pair(sym, v);
        }
        eprintln!("; tisp: error: could not read given input '{}' ({})", ch, c);
        return None;
    }
    Some(clone_val(&st.none))
}

pub fn tisp_read(st: &mut Tsp) -> Option<Val> {
    let mut v = tisp_read_sexpr(st)?;
    loop {
        match tsp_fget(st) {
            Some(b'(') | Some(b':') | Some(b'>') | Some(b'{') => {
                v = tisp_read_sugar(st, v)?;
            }
            _ => break,
        }
    }
    Some(v)
}

pub fn tisp_read_sugar(st: &mut Tsp, v: Val) -> Option<Val> {
    match tsp_fget(st) {
        Some(b'(') => {
            st.filec += 1;
            let lst = read_pair(st, ')')?;
            mk_pair(v, lst)
        }
        Some(b'{') => {
            st.filec += 1;
            let lst = read_pair(st, '}')?;
            let sym = mk_sym(st, "recmerge")?;
            let rec_sym = mk_sym(st, "Rec")?;
            let rec_pair = mk_pair(rec_sym, lst)?;
            mk_list(st, 3, vec![sym, v, rec_pair])
        }
        Some(b':') => {
            st.filec += 1;
            match tsp_fget(st) {
                Some(b'(') => {
                    st.filec += 1;
                    let w = read_pair(st, ')')?;
                    let sym = mk_sym(st, "map")?;
                    mk_pair(sym, mk_pair(v, w)?)
                }
                Some(b':') => {
                    st.filec += 1;
                    let w = read_sym(st, is_sym)?;
                    let quote_sym = mk_sym(st, "quote")?;
                    let quoted = mk_list(st, 2, vec![quote_sym, w])?;
                    mk_list(st, 2, vec![v, quoted])
                }
                _ => {
                    skip_ws(st, 1);
                    let w = tisp_read(st)?;
                    mk_list(st, 2, vec![v, w])
                }
            }
        }
        Some(b'>') => {
            if tsp_fgetat(st, 1) == Some(b'>') {
                st.filec += 2;
                let w = tisp_read(st)?;
                if w.t != TspType::TspPair {
                    eprintln!("; tisp: error: invalid UFCS");
                    return None;
                }
                let wcar = clone_val(car(&w));
                let wcdr = clone_val(cdr(&w));
                mk_pair(wcar, mk_pair(v, wcdr)?)
            } else {
                Some(v)
            }
        }
        _ => Some(v),
    }
}

pub fn tisp_read_line(st: &mut Tsp, level: i32) -> Option<Val> {
    let ret = read_pair(st, '\n')?;
    let mut ret = if ret.t != TspType::TspPair {
        let nil = clone_val(&st.nil);
        mk_pair(ret, nil)?
    } else {
        ret
    };

    // Collect sub-expressions from indented lines
    let mut subs: Vec<Val> = Vec::new();
    while tsp_fget(st).is_some() {
        let bytes = st.file.as_bytes();
        let mut newlevel = 0i32;
        let mut pos = st.filec;
        while pos < bytes.len() && (bytes[pos] == b'\t' || bytes[pos] == b' ') {
            newlevel += 1;
            pos += 1;
        }
        if newlevel <= level { break; }
        st.filec = pos;
        let sub = tisp_read_line(st, newlevel)?;
        subs.push(sub);
    }

    // Append subs to the end of ret
    if !subs.is_empty() {
        // Flatten ret into a vec, append subs, rebuild
        let mut elems: Vec<Val> = Vec::new();
        let mut cur = &ret;
        while cur.t == TspType::TspPair {
            elems.push(clone_val(car(cur)));
            cur = cdr(cur);
        }
        let tail = clone_val(cur);
        // Insert subs after the last pair element but before tail
        for s in subs.into_iter().rev() {
            elems.push(s);
        }
        // Rebuild
        let mut result = tail;
        for elem in elems.into_iter().rev() {
            result = mk_pair(elem, result)?;
        }
        ret = result;
    }

    // If only 1 element in list, return just it
    if ret.t == TspType::TspPair && nilp(cdr(&ret)) {
        let c = clone_val(car(&ret));
        return Some(c);
    }
    Some(ret)
}

// ---- eval ----
pub fn tisp_eval_list(st: &mut Tsp, env: &mut Rec, v: Val) -> Option<Val> {
    let mut elements: Vec<Val> = Vec::new();
    let mut cur = &v;
    while !nilp(cur) {
        if cur.t != TspType::TspPair {
            // last element in improper list
            let ev = tisp_eval_with_env(st, env, clone_val(cur))?;
            // Build list from elements with ev as tail
            let mut result = ev;
            for elem in elements.into_iter().rev() {
                result = mk_pair(elem, result)?;
            }
            return Some(result);
        }
        let ev = tisp_eval_with_env(st, env, clone_val(car(cur)))?;
        elements.push(ev);
        cur = cdr(cur);
    }
    let nil = clone_val(&st.nil);
    let mut result = nil;
    for elem in elements.into_iter().rev() {
        result = mk_pair(elem, result)?;
    }
    Some(result)
}

pub fn tisp_eval_body(st: &mut Tsp, env: &mut Rec, v: Val) -> Option<Val> {
    let mut ret = clone_val(&st.none);
    let mut cur = &v;
    while cur.t == TspType::TspPair {
        let is_last = nilp(cdr(cur));
        let c = car(cur);
        if is_last && c.t == TspType::TspPair {
            // Tail call optimization
            let f = tisp_eval_with_env(st, env, clone_val(car(c)))?;
            if f.t != TspType::TspFunc {
                return eval_proc(st, env, f, clone_val(cdr(c)));
            }
            // Get func info
            if let ValUnion::F { name, args: fargs, body: fbody, env: fenv } = &f.v {
                let fname = if name.is_empty() { "anon" } else { name.as_str() };
                let expected = tsp_lstlen(fargs);
                let got = tsp_lstlen(cdr(c));
                if expected > -1 && got != expected {
                    eprintln!("; tisp: error: {}: expected {} argument{}, received {}",
                        fname, expected, if expected > 1 { "s" } else { "" }, got);
                    return None;
                }
                let evaled_args = tisp_eval_list(st, env, clone_val(cdr(c)))?;
                let mut new_env = rec_extend(&mut clone_rec(fenv), clone_val(fargs), evaled_args);
                // Continue loop from body of func call
                let nil = clone_val(&st.nil);
                let body_clone = clone_val(fbody);
                // We need to iterate over body_clone now
                return tisp_eval_body(st, &mut new_env, body_clone);
            }
            return None;
        } else {
            ret = tisp_eval_with_env(st, env, clone_val(c))?;
        }
        cur = cdr(cur);
    }
    Some(ret)
}

pub fn prepend_bt(st: &mut Tsp, env: &mut Rec, f: Val) {
    if let ValUnion::F { name, .. } = &f.v {
        if name.is_empty() { return; }
        // Find base env (the one without a next)
        let mut r: &mut Rec = env;
        while r.next.is_some() {
            r = r.next.as_deref_mut().unwrap();
        }
        let idx = entry_get_idx(r, "bt");
        if r.items[idx].key.is_empty() { return; }
        let bt = &r.items[idx].val;
        if bt.t == TspType::TspPair {
            if car(bt).t == TspType::TspSym {
                if let ValUnion::S(s) = &car(bt).v {
                    if name.starts_with(s.as_str()) { return; }
                }
            }
        }
        let old_bt = clone_val(&r.items[idx].val);
        let sym = Val { t: TspType::TspSym, v: ValUnion::S(name.clone()) };
        if let Some(new_bt) = mk_pair(sym, old_bt) {
            r.items[idx].val = new_bt;
        }
    }
}

pub fn eval_proc(st: &mut Tsp, env: &mut Rec, f: Val, args: Val) -> Option<Val> {
    match f.t {
        TspType::TspPrim => {
            let evaled = tisp_eval_list(st, env, args)?;
            if let ValUnion::Pr { pr, .. } = &f.v {
                let result = pr(st, env, evaled);
                if is_err_val(&result) { return None; }
                return Some(result);
            }
            None
        }
        TspType::TspForm => {
            if let ValUnion::Pr { pr, .. } = &f.v {
                let result = pr(st, env, args);
                if is_err_val(&result) { return None; }
                return Some(result);
            }
            None
        }
        TspType::TspFunc => {
            let evaled = tisp_eval_list(st, env, args)?;
            if let ValUnion::F { name, args: fargs, body, env: fenv } = &f.v {
                let fname = if name.is_empty() { "anon" } else { name.as_str() };
                let expected = tsp_lstlen(fargs);
                let got = tsp_lstlen(&evaled);
                if expected > -1 && got != expected {
                    eprintln!("; tisp: error: {}: expected {} argument{}, received {}",
                        fname, expected, if expected > 1 { "s" } else { "" }, got);
                    return None;
                }
                let mut fenv_clone = clone_rec(fenv);
                let mut new_env = rec_extend(&mut fenv_clone, clone_val(fargs), evaled);
                let result = tisp_eval_body(st, &mut new_env, clone_val(body));
                if result.is_none() {
                    prepend_bt(st, env, f);
                }
                return result;
            }
            None
        }
        TspType::TspMacro => {
            if let ValUnion::F { name, args: fargs, body, env: fenv } = &f.v {
                let fname = if name.is_empty() { "anon" } else { name.as_str() };
                let expected = tsp_lstlen(fargs);
                let got = tsp_lstlen(&args);
                if expected > -1 && got != expected {
                    eprintln!("; tisp: error: {}: expected {} argument{}, received {}",
                        fname, expected, if expected > 1 { "s" } else { "" }, got);
                    return None;
                }
                let mut fenv_clone = clone_rec(fenv);
                let mut new_env = rec_extend(&mut fenv_clone, clone_val(fargs), args);
                let result = tisp_eval_body(st, &mut new_env, clone_val(body));
                if result.is_none() {
                    prepend_bt(st, env, f);
                    return None;
                }
                // Macro: eval the result
                return tisp_eval_with_env(st, env, result.unwrap());
            }
            None
        }
        TspType::TspRec => {
            let evaled = tisp_eval_list(st, env, args)?;
            let got = tsp_lstlen(&evaled);
            if got != 1 {
                eprintln!("; tisp: error: record: expected 1 argument, received {}", got);
                return None;
            }
            let key_val = car(&evaled);
            if key_val.t as u32 & TspType::TspSym as u32 == 0 {
                eprintln!("; tisp: error: record: expected Sym, received {}", tsp_type_str(key_val.t));
                return None;
            }
            if let ValUnion::R(r) = &f.v {
                if let Some(v) = rec_get(r, vs(key_val)) { return Some(v); }
                if let Some(v) = rec_get(r, "else") { return Some(v); }
                eprintln!("; tisp: error: could not find element '{}' in record", vs(key_val));
                return None;
            }
            None
        }
        _ => {
            eprintln!("; tisp: error: attempt to evaluate non procedural type {}", tsp_type_str(f.t));
            None
        }
    }
}

// Helper: eval with a given env (since tisp_eval uses st.env)
pub fn tisp_eval_with_env(st: &mut Tsp, env: &mut Rec, v: Val) -> Option<Val> {
    match v.t {
        TspType::TspSym => {
            let s = match &v.v { ValUnion::S(s) => s.clone(), _ => return None };
            if let Some(f) = rec_get(env, &s) { return Some(f); }
            eprintln!("; tisp: error: could not find symbol '{}'", s);
            None
        }
        TspType::TspPair => {
            let fv = clone_val(car(&v));
            let args = clone_val(cdr(&v));
            let f = tisp_eval_with_env(st, env, fv)?;
            eval_proc(st, env, f, args)
        }
        _ => Some(v),
    }
}

pub fn tisp_eval(st: &mut Tsp, v: Val) -> Option<Val> {
    let mut env = clone_rec(&st.env);
    tisp_eval_with_env(st, &mut env, v)
}

// ---- print ----
pub fn tisp_print(f: &mut dyn Write, v: &Val) {
    match v.t {
        TspType::TspNone => { let _ = write!(f, "Void"); }
        TspType::TspNil => { let _ = write!(f, "Nil"); }
        TspType::TspInt => { let _ = write!(f, "{}", vnum(v) as i32); }
        TspType::TspDec => {
            let n = vnum(v);
            let s = format_g(n, 15);
            let _ = write!(f, "{}", s);
            if n == (n as i32) as f64 {
                let _ = write!(f, ".0");
            }
        }
        TspType::TspRatio => {
            let _ = write!(f, "{}/{}", vnum(v) as i32, vden(v) as i32);
        }
        TspType::TspStr | TspType::TspSym => {
            let _ = write!(f, "{}", vs(v));
        }
        TspType::TspFunc | TspType::TspMacro => {
            if let ValUnion::F { name, .. } = &v.v {
                let tname = if v.t == TspType::TspFunc { "function" } else { "macro" };
                if name.is_empty() {
                    let _ = write!(f, "#<{}>", tname);
                } else {
                    let _ = write!(f, "#<{}:{}>", tname, name);
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
            if let ValUnion::R(r) = &v.v {
                let _ = write!(f, "{{");
                let mut rec_opt = Some(r);
                while let Some(rec) = rec_opt {
                    let mut c = 0;
                    for i in 0..rec.items.len() {
                        if !rec.items[i].key.is_empty() {
                            c += 1;
                            let _ = write!(f, " {}: ", rec.items[i].key);
                            tisp_print(f, &rec.items[i].val);
                            if c >= TSP_REC_MAX_PRINT {
                                let _ = write!(f, " ...");
                                break;
                            }
                        }
                    }
                    rec_opt = rec.next.as_deref();
                }
                let _ = write!(f, " }}");
            }
        }
        TspType::TspPair => {
            let _ = write!(f, "(");
            tisp_print(f, car(v));
            let mut cur = cdr(v);
            while !nilp(cur) {
                if cur.t == TspType::TspPair {
                    let _ = write!(f, " ");
                    tisp_print(f, car(cur));
                    cur = cdr(cur);
                } else {
                    let _ = write!(f, " . ");
                    tisp_print(f, cur);
                    break;
                }
            }
            let _ = write!(f, ")");
        }
        _ => {
            eprintln!("; tisp: could not print value type {}", tsp_type_str(v.t));
        }
    }
}

// Proper decimal printing matching C's %.15g
pub fn tisp_print_file(f: &mut std::fs::File, v: &Val) {
    tisp_print(f, v);
}

// ---- environment ----
pub fn tisp_env_add(st: &mut Tsp, key: &str, v: Val) {
    rec_add(&mut st.env, key, v);
}

pub fn tisp_env_init(cap: usize) -> Tsp {
    let nil = Val { t: TspType::TspNil, v: ValUnion::N { num: 0.0, den: 0.0 } };
    let none = Val { t: TspType::TspNone, v: ValUnion::N { num: 0.0, den: 0.0 } };
    let t_val = Val { t: TspType::TspSym, v: ValUnion::S("True".to_string()) };

    let mut st = Tsp {
        file: String::new(), filec: 0,
        none: clone_val(&none), nil: clone_val(&nil), t: clone_val(&t_val),
        env: rec_new(cap, None),
        strs: rec_new(cap, None), syms: rec_new(cap, None),
        libh: Vec::new(), libhc: 0,
    };

    rec_add(&mut st.env, "True", clone_val(&t_val));
    rec_add(&mut st.env, "Nil", clone_val(&nil));
    rec_add(&mut st.env, "Void", clone_val(&none));
    rec_add(&mut st.env, "bt", clone_val(&nil));
    let ver = Val { t: TspType::TspStr, v: ValUnion::S("0.1".to_string()) };
    rec_add(&mut st.strs, "0.1", clone_val(&ver));
    rec_add(&mut st.env, "version", ver);

    st
}

pub fn tisp_env_lib(st: &mut Tsp, lib: &str) {
    let file = st.file.clone();
    let filec = st.filec;
    st.file = lib.to_string();
    st.filec = 0;
    skip_ws(st, 1);
    if let Some(v) = tisp_read(st) {
        let mut env = clone_rec(&st.env);
        tisp_eval_body(st, &mut env, v);
        st.env = env;
    }
    st.file = file;
    st.filec = filec;
}

// ---- tib env stubs (delegated to other modules) ----
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
