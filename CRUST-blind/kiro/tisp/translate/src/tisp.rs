use std::io::Write;

pub const TSP_REC_MAX_PRINT: usize = 64;
pub const TSP_SYM_CHARS: &str = "_!?@#$%&~*-";
pub const TSP_REC_FACTOR: usize = 2;
pub const TSP_OP_CHARS: &str = "_+-*/\\|=^<>.:";
pub const TSP_RATIONAL: u32 = TspType::TspInt as u32 | TspType::TspRatio as u32;
pub const TSP_NUM: u32 = TSP_RATIONAL | TspType::TspDec as u32;
pub const TSP_EXPR: u32 = TSP_NUM | TspType::TspSym as u32 | TspType::TspPair as u32;

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

pub struct Entry {
    pub key: String,
    pub val: Val,
}

pub type Prim = fn(&mut Tsp, &mut Rec, Val) -> Val;

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

fn default_union() -> ValUnion { ValUnion::N { num: 0.0, den: 0.0 } }

pub fn car_ref(v: &Val) -> &Val {
    match &v.v { ValUnion::P { car, .. } => car, _ => panic!("car on non-pair") }
}
pub fn cdr_ref(v: &Val) -> &Val {
    match &v.v { ValUnion::P { cdr, .. } => cdr, _ => panic!("cdr on non-pair") }
}
pub fn car_owned(v: Val) -> Val {
    match v.v { ValUnion::P { car, .. } => *car, _ => panic!("car on non-pair") }
}
pub fn cdr_owned(v: Val) -> Val {
    match v.v { ValUnion::P { cdr, .. } => *cdr, _ => panic!("cdr on non-pair") }
}
pub fn num_of(v: &Val) -> f64 {
    match &v.v { ValUnion::N { num, .. } => *num, _ => 0.0 }
}
pub fn den_of(v: &Val) -> f64 {
    match &v.v { ValUnion::N { den, .. } => *den, _ => 1.0 }
}
pub fn sym_str(v: &Val) -> &str {
    match &v.v { ValUnion::S(s) => s, _ => "" }
}
pub fn nilp(v: &Val) -> bool { v.t == TspType::TspNil }
pub fn type_matches(t: TspType, mask: u32) -> bool { (t as u32) & mask != 0 }

pub fn val_clone(v: &Val) -> Val {
    Val { t: v.t, v: vu_clone(&v.v) }
}

fn vu_clone(v: &ValUnion) -> ValUnion {
    match v {
        ValUnion::S(s) => ValUnion::S(s.clone()),
        ValUnion::N { num, den } => ValUnion::N { num: *num, den: *den },
        ValUnion::Pr { name, pr } => ValUnion::Pr { name: name.clone(), pr: *pr },
        ValUnion::F { name, args, body, env } => ValUnion::F {
            name: name.clone(), args: Box::new(val_clone(args)),
            body: Box::new(val_clone(body)), env: rec_clone(env),
        },
        ValUnion::P { car, cdr } => ValUnion::P {
            car: Box::new(val_clone(car)), cdr: Box::new(val_clone(cdr)),
        },
        ValUnion::R(r) => ValUnion::R(rec_clone(r)),
    }
}

pub fn rec_clone(r: &Rec) -> Rec {
    Rec {
        size: r.size, cap: r.cap,
        items: r.items.iter().map(|e| Entry { key: e.key.clone(), val: val_clone(&e.val) }).collect(),
        next: r.next.as_ref().map(|n| Box::new(rec_clone(n))),
    }
}

fn fget(st: &Tsp) -> Option<u8> {
    if st.filec < st.file.len() { Some(st.file.as_bytes()[st.filec]) } else { None }
}
fn fget_char(st: &Tsp) -> char {
    fget(st).map(|b| b as char).unwrap_or('\0')
}
fn fgetat(st: &Tsp, offset: isize) -> char {
    let pos = st.filec as isize + offset;
    if pos >= 0 && (pos as usize) < st.file.len() {
        st.file.as_bytes()[pos as usize] as char
    } else { '\0' }
}
fn finc(st: &mut Tsp) { st.filec += 1; }

pub fn mk_nil_val() -> Val { Val { t: TspType::TspNil, v: default_union() } }
pub fn mk_none_val() -> Val { Val { t: TspType::TspNone, v: default_union() } }
pub fn mk_pair_val(a: Val, b: Val) -> Val {
    Val { t: TspType::TspPair, v: ValUnion::P { car: Box::new(a), cdr: Box::new(b) } }
}

// ---- Public API functions ----

pub fn tsp_type_str(t: TspType) -> &'static str {
    match t {
        TspType::TspNone => "Void", TspType::TspNil => "Nil",
        TspType::TspInt => "Int", TspType::TspDec => "Dec",
        TspType::TspRatio => "Ratio", TspType::TspStr => "Str",
        TspType::TspSym => "Sym", TspType::TspPrim => "Prim",
        TspType::TspForm => "Form", TspType::TspFunc => "Func",
        TspType::TspMacro => "Macro", TspType::TspPair => "Pair",
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
    let b = s.as_bytes();
    if b.is_empty() { return false; }
    if b[0].is_ascii_digit() { return true; }
    if b[0] == b'.' && b.len() > 1 && b[1].is_ascii_digit() { return true; }
    if (b[0] == b'-' || b[0] == b'+') && b.len() > 1 &&
       (b[1].is_ascii_digit() || b[1] == b'.') { return true; }
    false
}

pub fn skip_ws(st: &mut Tsp, skipnl: i32) {
    let snl = skipnl != 0;
    loop {
        while st.filec < st.file.len() {
            let c = st.file.as_bytes()[st.filec] as char;
            match c {
                ' ' | '\t' => st.filec += 1,
                '\n' | '\r' if snl => st.filec += 1,
                _ => break,
            }
        }
        if st.filec < st.file.len() && st.file.as_bytes()[st.filec] == b';' {
            while st.filec < st.file.len() && st.file.as_bytes()[st.filec] != b'\n' {
                st.filec += 1;
            }
            if !snl {
                // don't skip newline
            } else if st.filec < st.file.len() {
                st.filec += 1;
            }
            continue;
        }
        break;
    }
}

pub fn tsp_lstlen(v: &Val) -> i32 {
    let mut len = 0i32;
    let mut cur = v;
    while cur.t == TspType::TspPair {
        len += 1;
        cur = cdr_ref(cur);
    }
    if nilp(cur) { len } else { -(len + 1) }
}

pub fn vals_eq(a: &Val, b: &Val) -> bool {
    if type_matches(a.t, TSP_NUM) && type_matches(b.t, TSP_NUM) {
        return num_of(a) == num_of(b) && den_of(a) == den_of(b);
    }
    if a.t != b.t { return false; }
    if a.t == TspType::TspPair {
        return vals_eq(car_ref(a), car_ref(b)) && vals_eq(cdr_ref(a), cdr_ref(b));
    }
    if a.t == TspType::TspFunc || a.t == TspType::TspMacro {
        if let (ValUnion::F { args: aa, body: ab, .. }, ValUnion::F { args: ba, body: bb, .. }) = (&a.v, &b.v) {
            return vals_eq(aa, ba) && vals_eq(ab, bb);
        }
    }
    match (&a.v, &b.v) {
        (ValUnion::S(sa), ValUnion::S(sb)) => sa == sb,
        (ValUnion::Pr { name: na, .. }, ValUnion::Pr { name: nb, .. }) => na == nb,
        (ValUnion::N { .. }, ValUnion::N { .. }) => true,
        _ => false,
    }
}

pub fn frac_reduce(num: &mut i32, den: &mut i32) {
    let mut a = num.unsigned_abs() as i32;
    let mut b = den.unsigned_abs() as i32;
    if b == 0 { return; }
    let mut c = a % b;
    while c > 0 { a = b; b = c; c = a % b; }
    *num /= b;
    *den /= b;
}

pub fn hash(key: &str) -> u32 {
    let mut h: u32 = 0;
    for c in key.bytes() {
        h = h.wrapping_mul(33).wrapping_add(c as u32);
    }
    h
}

pub fn rec_new(cap: usize, next: Option<Box<Rec>>) -> Rec {
    let cap = if cap == 0 { 1 } else { cap };
    let mut items = Vec::with_capacity(cap);
    for _ in 0..cap {
        items.push(Entry { key: String::new(), val: mk_val(TspType::TspNone) });
    }
    Rec { size: 0, cap: cap as i32, items, next }
}

pub fn entry_get<'a>(rec: &'a Rec, key: &'a str) -> Option<&'a Entry> {
    if rec.cap <= 0 { return None; }
    let mut i = (hash(key) % rec.cap as u32) as usize;
    loop {
        if rec.items[i].key.is_empty() { return Some(&rec.items[i]); }
        if rec.items[i].key == key { return Some(&rec.items[i]); }
        i = (i + 1) % rec.cap as usize;
    }
}

pub fn entry_idx(rec: &Rec, key: &str) -> usize {
    let mut i = (hash(key) % rec.cap as u32) as usize;
    loop {
        if rec.items[i].key.is_empty() || rec.items[i].key == key { return i; }
        i = (i + 1) % rec.cap as usize;
    }
}

pub fn rec_get(rec: &Rec, key: &str) -> Option<Val> {
    let mut cur = Some(rec);
    while let Some(r) = cur {
        if r.cap > 0 {
            let i = entry_idx(r, key);
            if !r.items[i].key.is_empty() {
                return Some(val_clone(&r.items[i].val));
            }
        }
        cur = r.next.as_deref();
    }
    None
}

pub fn rec_grow(rec: &mut Rec) {
    let old_items: Vec<Entry> = std::mem::take(&mut rec.items);
    rec.cap *= TSP_REC_FACTOR as i32;
    rec.size = 0;
    rec.items = (0..rec.cap).map(|_| Entry { key: String::new(), val: mk_val(TspType::TspNone) }).collect();
    for e in old_items {
        if !e.key.is_empty() { rec_add(rec, &e.key.clone(), e.val); }
    }
}

pub fn rec_add(rec: &mut Rec, key: &str, val: Val) {
    let i = entry_idx(rec, key);
    rec.items[i].val = val;
    if rec.items[i].key.is_empty() {
        rec.items[i].key = key.to_string();
        rec.size += 1;
        if rec.size > rec.cap / TSP_REC_FACTOR as i32 {
            rec_grow(rec);
        }
    }
}

pub fn rec_extend(rec: &mut Rec, args: Val, vals: Val) -> Rec {
    let argnum = TSP_REC_FACTOR as i32 * tsp_lstlen(&args);
    let cap = if argnum > 0 { argnum as usize } else { (-argnum + 1) as usize };
    let mut ret = rec_new(cap, Some(Box::new(rec_clone(rec))));
    let mut a = &args;
    let mut v = &vals;
    loop {
        if nilp(a) { break; }
        let (arg, val) = if a.t == TspType::TspPair {
            (car_ref(a), car_ref(v))
        } else { (a, v) };
        if arg.t != TspType::TspSym {
            eprintln!("; tisp: error: expected symbol for argument of function definition, recieved '{}'", tsp_type_str(arg.t));
        }
        rec_add(&mut ret, sym_str(arg), val_clone(val));
        if a.t != TspType::TspPair { break; }
        a = cdr_ref(a);
        v = cdr_ref(v);
    }
    ret
}

// ---- Make types ----

pub fn mk_val(t: TspType) -> Val { Val { t, v: default_union() } }

pub fn mk_int(i: i32) -> Val {
    Val { t: TspType::TspInt, v: ValUnion::N { num: i as f64, den: 1.0 } }
}

pub fn mk_dec(d: f64) -> Option<Val> {
    Some(Val { t: TspType::TspDec, v: ValUnion::N { num: d, den: 1.0 } })
}

pub fn mk_rat(num: i32, den: i32) -> Option<Val> {
    if den == 0 {
        eprintln!("; tisp: error: division by zero");
        return None;
    }
    let mut n = num; let mut d = den;
    frac_reduce(&mut n, &mut d);
    if d < 0 { d = d.abs(); n = -n; }
    if d == 1 { return Some(mk_int(n)); }
    Some(Val { t: TspType::TspRatio, v: ValUnion::N { num: n as f64, den: d as f64 } })
}

pub fn mk_str(st: &mut Tsp, s: &str) -> Option<Val> {
    if let Some(v) = rec_get(&st.strs, s) { return Some(v); }
    let v = Val { t: TspType::TspStr, v: ValUnion::S(s.to_string()) };
    rec_add(&mut st.strs, s, val_clone(&v));
    Some(v)
}

pub fn mk_str_val(st: &mut Tsp, s: &str) -> Val { mk_str(st, s).unwrap() }

pub fn mk_sym(st: &mut Tsp, s: &str) -> Option<Val> {
    if let Some(v) = rec_get(&st.syms, s) { return Some(v); }
    let v = Val { t: TspType::TspSym, v: ValUnion::S(s.to_string()) };
    rec_add(&mut st.syms, s, val_clone(&v));
    Some(v)
}

pub fn mk_sym_val(st: &mut Tsp, s: &str) -> Val { mk_sym(st, s).unwrap() }

pub fn mk_prim(t: TspType, pr: Prim, name: &str) -> Option<Val> {
    Some(Val { t, v: ValUnion::Pr { name: name.to_string(), pr } })
}

pub fn mk_func(t: TspType, name: &str, args: Val, body: Val, env: Rec) -> Option<Val> {
    Some(Val { t, v: ValUnion::F {
        name: name.to_string(), args: Box::new(args), body: Box::new(body), env,
    }})
}

pub fn mk_rec(st: &mut Tsp, env: Rec, assoc: Val) -> Option<Val> {
    // When called as form "Rec", assoc is the args list from the form call
    // If assoc is nil/none, just wrap env
    if assoc.t == TspType::TspNil || assoc.t == TspType::TspNone {
        return Some(Val { t: TspType::TspRec, v: ValUnion::R(env) });
    }
    let len = tsp_lstlen(&assoc);
    let cap = TSP_REC_FACTOR * (if len > 0 { len } else { -len + 1 }) as usize;
    let cap = if cap == 0 { 1 } else { cap };
    let mut rec = rec_new(cap, None);
    let mut r = rec_new(4, Some(Box::new(rec_clone(&env))));
    // add placeholder "this"
    let placeholder = Val { t: TspType::TspRec, v: ValUnion::R(rec_new(1, None)) };
    rec_add(&mut r, "this", placeholder);

    let mut cur = &assoc;
    while cur.t == TspType::TspPair {
        let item = car_ref(cur);
        if item.t == TspType::TspPair {
            let k = car_ref(item);
            if type_matches(k.t, TspType::TspSym as u32 | TspType::TspStr as u32) {
                let body_v = car_ref(cdr_ref(item));
                match tisp_eval_with_env(st, &mut r, val_clone(body_v)) {
                    Some(v) => rec_add(&mut rec, sym_str(k), v),
                    None => return None,
                }
            } else {
                eprintln!("; tisp: error: Rec: missing key symbol or string");
                return None;
            }
        } else if item.t == TspType::TspSym {
            match tisp_eval_with_env(st, &mut r, val_clone(item)) {
                Some(v) => rec_add(&mut rec, sym_str(item), v),
                None => return None,
            }
        } else {
            eprintln!("; tisp: error: Rec: missing key symbol or string");
            return None;
        }
        cur = cdr_ref(cur);
    }
    Some(Val { t: TspType::TspRec, v: ValUnion::R(rec) })
}

pub fn mk_pair(a: Val, b: Val) -> Option<Val> { Some(mk_pair_val(a, b)) }

pub fn mk_list(st: &mut Tsp, _n: i32, args: Vec<Val>) -> Option<Val> {
    if args.is_empty() { return Some(mk_nil_val()); }
    let mut result = mk_nil_val();
    for a in args.into_iter().rev() {
        result = mk_pair_val(a, result);
    }
    Some(result)
}

// ---- Read ----

pub fn read_sign(st: &mut Tsp) -> i32 {
    match fget_char(st) {
        '-' => { finc(st); -1 }
        '+' => { finc(st); 1 }
        _ => 1,
    }
}

pub fn read_int(st: &mut Tsp) -> i32 {
    let mut ret = 0i32;
    while st.filec < st.file.len() && (st.file.as_bytes()[st.filec] as char).is_ascii_digit() {
        ret = ret * 10 + (st.file.as_bytes()[st.filec] - b'0') as i32;
        finc(st);
    }
    ret
}

pub fn read_sci(st: &mut Tsp, val: f64, isint: i32) -> Option<Val> {
    let mut v = val;
    if st.filec < st.file.len() && (fget_char(st) == 'e' || fget_char(st) == 'E') {
        finc(st);
        let sign = if read_sign(st) == 1 { 10.0 } else { 0.1 };
        let expo = read_int(st);
        for _ in 0..expo { v *= sign; }
    }
    if isint != 0 { Some(mk_int(v as i32)) } else { mk_dec(v) }
}

pub fn read_num(st: &mut Tsp) -> Val {
    let sign = read_sign(st);
    let n = read_int(st);
    match fget_char(st) {
        '/' => {
            finc(st);
            if !isnum(&st.file[st.filec..]) {
                eprintln!("; tisp: error: incorrect ratio format, no denominator found");
                return mk_none_val();
            }
            let ds = read_sign(st);
            let d = read_int(st);
            mk_rat(sign * n, ds * d).unwrap_or_else(|| mk_none_val())
        }
        '.' => {
            finc(st);
            let oldc = st.filec;
            let d = read_int(st) as f64;
            let size = st.filec - oldc;
            let mut dec = d;
            for _ in 0..size { dec /= 10.0; }
            read_sci(st, sign as f64 * (n as f64 + dec), 0).unwrap_or_else(|| mk_none_val())
        }
        _ => read_sci(st, sign as f64 * n as f64, 1).unwrap_or_else(|| mk_none_val()),
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
    finc(st); // skip opening quote
    let start = st.filec;
    let is_str = {
        // Check if we're making a string (started with ") or symbol (started with ~)
        // The caller passes mk_str_val or mk_sym_val
        // We detect by checking the char before start
        if start > 0 && st.file.as_bytes()[start - 1] == b'"' { true } else { false }
    };
    let endchar = if is_str { '"' } else { '~' };
    let mut len = 0i32;
    while st.filec < st.file.len() && fget_char(st) != endchar {
        if fget_char(st) == '\\' && (st.filec == 0 || fgetat(st, -1) != '\\') {
            finc(st);
        }
        finc(st);
        len += 1;
    }
    if st.filec >= st.file.len() {
        eprintln!("; tisp: error: reached end before closing {}", endchar);
        return None;
    }
    finc(st); // skip closing quote
    let s_slice = &st.file[start..start + (st.filec - start - 1).min(st.file.len() - start)];
    let escaped = esc_str(&st.file[start..], len, if is_str { 1 } else { 0 });
    Some(mk_fn(st, &escaped))
}

pub fn read_sym(st: &mut Tsp, is_char: fn(char) -> bool) -> Option<Val> {
    let start = st.filec;
    while st.filec < st.file.len() && is_char(fget_char(st)) {
        finc(st);
    }
    let s = esc_str(&st.file[start..], (st.filec - start) as i32, 0);
    mk_sym(st, &s)
}

pub fn read_pair(st: &mut Tsp, endchar: char) -> Option<Val> {
    let skipnl = endchar != '\n';
    skip_ws(st, if skipnl { 1 } else { 0 });

    // Build list using a vector, then construct pairs at the end
    let mut items: Vec<Val> = Vec::new();
    let mut improper_end: Option<Val> = None;

    while st.filec < st.file.len() && fget_char(st) != endchar {
        let v = tisp_read(st)?;
        // Check for dot (improper list)
        if v.t == TspType::TspSym && sym_str(&v) == "." {
            skip_ws(st, if skipnl { 1 } else { 0 });
            let end = tisp_read(st)?;
            improper_end = Some(end);
            break;
        }
        items.push(v);
        skip_ws(st, if skipnl { 1 } else { 0 });
    }

    skip_ws(st, if skipnl { 1 } else { 0 });
    if skipnl && (st.filec >= st.file.len() || fget_char(st) != endchar) {
        eprintln!("; tisp: error: did not find closing '{}'", endchar);
        return None;
    }
    if st.filec < st.file.len() {
        finc(st); // skip endchar
    }

    // Build the list from back to front
    let mut result = improper_end.unwrap_or_else(|| mk_nil_val());
    for v in items.into_iter().rev() {
        result = mk_pair_val(v, result);
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
    if st.filec >= st.file.len() { return Some(mk_none_val()); }

    let remaining = &st.file[st.filec..];
    if remaining.is_empty() { return Some(mk_none_val()); }

    // number
    if isnum(remaining) { return Some(read_num(st)); }

    // string
    if fget_char(st) == '"' { return read_str(st, mk_str_val); }

    // explicit symbol
    if fget_char(st) == '~' { return read_str(st, mk_sym_val); }

    // character prefix
    for i in (0..prefixes.len()).step_by(1) {
        let (pfx, name) = prefixes[i];
        if remaining.starts_with(pfx) {
            let skip = pfx.len() - if pfx.len() > 1 && pfx.as_bytes()[1] == b'"' { 1 } else { 0 };
            st.filec += skip;
            let v = tisp_read(st)?;
            let sym = mk_sym(st, name)?;
            return mk_list(st, 2, vec![sym, v]);
        }
    }

    // operators
    if is_op(fget_char(st)) { return read_sym(st, is_op); }

    // symbols
    if is_sym(fget_char(st)) { return read_sym(st, is_sym); }

    // list with parens
    if fget_char(st) == '(' {
        finc(st);
        return read_pair(st, ')');
    }

    // list with brackets
    if fget_char(st) == '[' {
        finc(st);
        let lst = read_pair(st, ']')?;
        let sym = mk_sym(st, "list")?;
        return Some(mk_pair_val(sym, lst));
    }

    // record with braces
    if fget_char(st) == '{' {
        finc(st);
        let v = read_pair(st, '}')?;
        let sym = mk_sym(st, "Rec")?;
        return Some(mk_pair_val(sym, v));
    }

    eprintln!("; tisp: error: could not read given input '{}' ({})",
              fget_char(st), fget_char(st) as u32);
    None
}

pub fn tisp_read(st: &mut Tsp) -> Option<Val> {
    let mut v = tisp_read_sexpr(st)?;
    while st.filec < st.file.len() &&
          (fget_char(st) == '(' || fget_char(st) == ':' ||
           fget_char(st) == '>' || fget_char(st) == '{') {
        v = tisp_read_sugar(st, v)?;
    }
    Some(v)
}

pub fn tisp_read_sugar(st: &mut Tsp, v: Val) -> Option<Val> {
    if fget_char(st) == '(' {
        finc(st);
        let lst = read_pair(st, ')')?;
        return Some(mk_pair_val(v, lst));
    } else if fget_char(st) == '{' {
        finc(st);
        let lst = read_pair(st, '}')?;
        let sym = mk_sym(st, "recmerge")?;
        let rec_sym = mk_sym(st, "Rec")?;
        let rec_expr = mk_pair_val(rec_sym, lst);
        return mk_list(st, 3, vec![sym, v, rec_expr]);
    } else if fget_char(st) == ':' {
        finc(st);
        match fget_char(st) {
            '(' => {
                finc(st);
                let w = read_pair(st, ')')?;
                let map_sym = mk_sym(st, "map")?;
                return Some(mk_pair_val(map_sym, mk_pair_val(v, w)));
            }
            ':' => {
                finc(st);
                let w = read_sym(st, is_sym)?;
                let q = mk_sym(st, "quote")?;
                let quoted = mk_list(st, 2, vec![q, w])?;
                return mk_list(st, 2, vec![v, quoted]);
            }
            _ => {
                skip_ws(st, 1);
                let w = tisp_read(st)?;
                return mk_list(st, 2, vec![v, w]);
            }
        }
    } else if fget_char(st) == '>' && fgetat(st, 1) == '>' {
        finc(st); finc(st);
        let w = tisp_read(st)?;
        if w.t != TspType::TspPair {
            eprintln!("; tisp: error: invalid UFCS");
            return None;
        }
        let w_car = car_ref(&w);
        let w_cdr = cdr_ref(&w);
        let new_car = val_clone(w_car);
        let new_cdr = val_clone(w_cdr);
        return Some(mk_pair_val(new_car, mk_pair_val(v, new_cdr)));
    }
    Some(v)
}

pub fn tisp_read_line(st: &mut Tsp, level: i32) -> Option<Val> {
    let mut ret = read_pair(st, '\n')?;
    if ret.t != TspType::TspPair {
        ret = mk_pair_val(ret, mk_nil_val());
    }
    // Find last pair
    // We need to collect sub-expressions from indented lines
    let mut sub_exprs: Vec<Val> = Vec::new();
    while st.filec < st.file.len() {
        let remaining = &st.file[st.filec..];
        let newlevel = remaining.bytes().take_while(|&b| b == b'\t' || b == b' ').count() as i32;
        if newlevel <= level { break; }
        st.filec += newlevel as usize;
        if let Some(sub) = tisp_read_line(st, newlevel) {
            sub_exprs.push(sub);
        } else {
            return None;
        }
    }

    // Append sub_exprs to the end of ret
    if !sub_exprs.is_empty() {
        // We need to append sub_exprs at the end of the list
        // Convert ret to vec, append, rebuild
        let mut items = list_to_vec(&ret);
        items.extend(sub_exprs);
        ret = mk_nil_val();
        for item in items.into_iter().rev() {
            ret = mk_pair_val(item, ret);
        }
    }

    // If only 1 element, return just it
    if ret.t == TspType::TspPair && nilp(cdr_ref(&ret)) {
        return Some(car_owned(ret));
    }
    Some(ret)
}

pub fn list_to_vec(v: &Val) -> Vec<Val> {
    let mut result = Vec::new();
    let mut cur = v;
    while cur.t == TspType::TspPair {
        result.push(val_clone(car_ref(cur)));
        cur = cdr_ref(cur);
    }
    result
}

// ---- Eval ----

pub fn tisp_eval_list(st: &mut Tsp, env: &mut Rec, v: Val) -> Option<Val> {
    let mut items: Vec<Val> = Vec::new();
    let mut cur = &v;
    while !nilp(cur) {
        if cur.t != TspType::TspPair {
            // last element in improper list
            let ev = tisp_eval_with_env(st, env, val_clone(cur))?;
            // build result so far then append ev as improper end
            let mut result = ev;
            for item in items.into_iter().rev() {
                result = mk_pair_val(item, result);
            }
            return Some(result);
        }
        let ev = tisp_eval_with_env(st, env, val_clone(car_ref(cur)))?;
        items.push(ev);
        cur = cdr_ref(cur);
    }
    let mut result = mk_nil_val();
    for item in items.into_iter().rev() {
        result = mk_pair_val(item, result);
    }
    Some(result)
}

pub fn tisp_eval_body(st: &mut Tsp, env: &mut Rec, v: Val) -> Option<Val> {
    let mut ret = mk_none_val();
    let mut cur = &v;
    while cur.t == TspType::TspPair {
        let is_last = nilp(cdr_ref(cur));
        let item = car_ref(cur);

        if is_last && item.t == TspType::TspPair {
            // Potential tail call
            let f = tisp_eval_with_env(st, env, val_clone(car_ref(item)))?;
            if f.t != TspType::TspFunc {
                return eval_proc(st, env, f, val_clone(cdr_ref(item)));
            }
            // Tail call optimization for functions
            let (f_args, f_body, f_env, f_name) = match &f.v {
                ValUnion::F { args, body, env: fenv, name } =>
                    (val_clone(args), val_clone(body), rec_clone(fenv), name.clone()),
                _ => return None,
            };
            let expected = tsp_lstlen(&f_args);
            let actual_args = val_clone(cdr_ref(item));
            let actual_len = tsp_lstlen(&actual_args);
            if expected > -1 && actual_len != expected {
                let name = if f_name.is_empty() { "anon" } else { &f_name };
                eprintln!("; tisp: error: {}: expected {} argument{}, received {}",
                    name, expected, if expected > 1 { "s" } else { "" }, actual_len);
                return None;
            }
            let evaled_args = tisp_eval_list(st, env, actual_args)?;
            let mut fenv_mut = f_env;
            *env = rec_extend(&mut fenv_mut, f_args, evaled_args);
            // Continue loop from body of func call - rebuild cur
            // We need to iterate over f_body now
            let body_items = list_to_vec(&f_body);
            if body_items.is_empty() { return Some(mk_none_val()); }
            // Recursively eval body
            let mut body_list = mk_nil_val();
            for b in body_items.into_iter().rev() {
                body_list = mk_pair_val(b, body_list);
            }
            return tisp_eval_body(st, env, body_list);
        } else {
            ret = tisp_eval_with_env(st, env, val_clone(item))?;
        }
        cur = cdr_ref(cur);
    }
    Some(ret)
}

pub fn prepend_bt(st: &mut Tsp, env: &mut Rec, f: Val) {
    let fname = match &f.v {
        ValUnion::F { name, .. } if !name.is_empty() => name.clone(),
        _ => return,
    };
    // Find base env (last in chain)
    let mut r = env as *mut Rec;
    unsafe {
        while let Some(ref mut next) = (*r).next {
            r = next.as_mut() as *mut Rec;
        }
        let base = &mut *r;
        let i = entry_idx(base, "bt");
        if !base.items[i].key.is_empty() {
            let bt = &base.items[i].val;
            if bt.t == TspType::TspPair {
                let first = car_ref(bt);
                if first.t == TspType::TspSym && sym_str(first) == fname {
                    return; // don't record same function on recursion
                }
            }
            let old_bt = val_clone(&base.items[i].val);
            let sym = Val { t: TspType::TspSym, v: ValUnion::S(fname) };
            base.items[i].val = mk_pair_val(sym, old_bt);
        }
    }
}

pub fn eval_proc(st: &mut Tsp, env: &mut Rec, f: Val, args: Val) -> Option<Val> {
    match f.t {
        TspType::TspPrim => {
            let evaled = tisp_eval_list(st, env, args)?;
            match &f.v {
                ValUnion::Pr { pr, .. } => Some(pr(st, env, evaled)),
                _ => None,
            }
        }
        TspType::TspForm => {
            match &f.v {
                ValUnion::Pr { pr, .. } => {
                    let p = *pr;
                    Some(p(st, env, args))
                }
                _ => None,
            }
        }
        TspType::TspFunc => {
            let evaled = tisp_eval_list(st, env, args)?;
            let (f_args, f_body, f_env, f_name) = match &f.v {
                ValUnion::F { args, body, env: fenv, name } =>
                    (val_clone(args), val_clone(body), rec_clone(fenv), name.clone()),
                _ => return None,
            };
            let expected = tsp_lstlen(&f_args);
            let actual_len = tsp_lstlen(&evaled);
            if expected > -1 && actual_len != expected {
                let name = if f_name.is_empty() { "anon" } else { &f_name };
                eprintln!("; tisp: error: {}: expected {} argument{}, received {}",
                    name, expected, if expected > 1 { "s" } else { "" }, actual_len);
                return None;
            }
            let mut fenv_mut = f_env;
            let mut new_env = rec_extend(&mut fenv_mut, f_args, evaled);
            let ret = tisp_eval_body(st, &mut new_env, f_body);
            if ret.is_none() {
                prepend_bt(st, env, Val { t: f.t, v: ValUnion::F {
                    name: f_name, args: Box::new(mk_nil_val()), body: Box::new(mk_nil_val()),
                    env: rec_new(1, None),
                }});
            }
            ret
        }
        TspType::TspMacro => {
            let (f_args, f_body, f_env, f_name) = match &f.v {
                ValUnion::F { args, body, env: fenv, name } =>
                    (val_clone(args), val_clone(body), rec_clone(fenv), name.clone()),
                _ => return None,
            };
            let expected = tsp_lstlen(&f_args);
            let actual_len = tsp_lstlen(&args);
            if expected > -1 && actual_len != expected {
                let name = if f_name.is_empty() { "anon" } else { &f_name };
                eprintln!("; tisp: error: {}: expected {} argument{}, received {}",
                    name, expected, if expected > 1 { "s" } else { "" }, actual_len);
                return None;
            }
            let mut fenv_mut = f_env;
            let mut new_env = rec_extend(&mut fenv_mut, f_args, args);
            let ret = tisp_eval_body(st, &mut new_env, f_body);
            if ret.is_none() {
                prepend_bt(st, env, Val { t: f.t, v: ValUnion::F {
                    name: f_name, args: Box::new(mk_nil_val()), body: Box::new(mk_nil_val()),
                    env: rec_new(1, None),
                }});
                return None;
            }
            // Macro: eval result again
            tisp_eval_with_env(st, env, ret.unwrap())
        }
        TspType::TspRec => {
            let evaled = tisp_eval_list(st, env, args)?;
            let len = tsp_lstlen(&evaled);
            if len != 1 {
                eprintln!("; tisp: error: record: expected 1 argument{}, received {}", if 1 > 1 { "s" } else { "" }, len);
                return None;
            }
            let key_val = car_ref(&evaled);
            if !type_matches(key_val.t, TspType::TspSym as u32) {
                eprintln!("; tisp: error: record: expected Sym, received {}", tsp_type_str(key_val.t));
                return None;
            }
            let key = sym_str(key_val);
            if let ValUnion::R(ref r) = f.v {
                if let Some(v) = rec_get(r, key) { return Some(v); }
                if let Some(v) = rec_get(r, "else") { return Some(v); }
            }
            eprintln!("; tisp: error: could not find element '{}' in record", key);
            None
        }
        _ => {
            eprintln!("; tisp: error: attempt to evaluate non procedural type {}", tsp_type_str(f.t));
            None
        }
    }
}

// tisp_eval uses st.env
pub fn tisp_eval(st: &mut Tsp, v: Val) -> Option<Val> {
    match v.t {
        TspType::TspSym => {
            let key = match &v.v { ValUnion::S(s) => s.clone(), _ => return None };
            match rec_get(&st.env, &key) {
                Some(f) => Some(f),
                None => {
                    eprintln!("; tisp: error: could not find symbol '{}'", key);
                    None
                }
            }
        }
        TspType::TspPair => {
            let f_val = val_clone(car_ref(&v));
            let args_val = val_clone(cdr_ref(&v));
            let f = tisp_eval(st, f_val)?;
            // Need to use st.env for eval_proc
            let env_clone = rec_clone(&st.env);
            let mut env = env_clone;
            eval_proc(st, &mut env, f, args_val)
        }
        _ => Some(v),
    }
}

// Version that takes an explicit env
pub fn tisp_eval_with_env(st: &mut Tsp, env: &mut Rec, v: Val) -> Option<Val> {
    match v.t {
        TspType::TspSym => {
            let key = match &v.v { ValUnion::S(s) => s.clone(), _ => return None };
            match rec_get(env, &key) {
                Some(f) => Some(f),
                None => {
                    eprintln!("; tisp: error: could not find symbol '{}'", key);
                    None
                }
            }
        }
        TspType::TspPair => {
            let f_val = val_clone(car_ref(&v));
            let args_val = val_clone(cdr_ref(&v));
            let f = tisp_eval_with_env(st, env, f_val)?;
            eval_proc(st, env, f, args_val)
        }
        _ => Some(v),
    }
}

// ---- Print ----

pub fn tisp_print(f: &mut dyn Write, v: &Val) {
    match v.t {
        TspType::TspNone => { write!(f, "Void").ok(); }
        TspType::TspNil => { write!(f, "Nil").ok(); }
        TspType::TspInt => { write!(f, "{}", num_of(v) as i32).ok(); }
        TspType::TspDec => {
            let n = num_of(v);
            write!(f, "{}", format_dec(n)).ok();
        }
        TspType::TspRatio => {
            write!(f, "{}/{}", num_of(v) as i32, den_of(v) as i32).ok();
        }
        TspType::TspStr | TspType::TspSym => {
            write!(f, "{}", sym_str(v)).ok();
        }
        TspType::TspFunc | TspType::TspMacro => {
            if let ValUnion::F { name, .. } = &v.v {
                let tname = if v.t == TspType::TspFunc { "function" } else { "macro" };
                if name.is_empty() {
                    write!(f, "#<{}>", tname).ok();
                } else {
                    write!(f, "#<{}:{}>", tname, name).ok();
                }
            }
        }
        TspType::TspPrim => {
            if let ValUnion::Pr { name, .. } = &v.v {
                write!(f, "#<primitive:{}>", name).ok();
            }
        }
        TspType::TspForm => {
            if let ValUnion::Pr { name, .. } = &v.v {
                write!(f, "#<form:{}>", name).ok();
            }
        }
        TspType::TspRec => {
            if let ValUnion::R(ref r) = v.v {
                write!(f, "{{").ok();
                print_rec(f, r);
                write!(f, " }}").ok();
            }
        }
        TspType::TspPair => {
            write!(f, "(").ok();
            tisp_print(f, car_ref(v));
            let mut cur = cdr_ref(v);
            while !nilp(cur) {
                if cur.t == TspType::TspPair {
                    write!(f, " ").ok();
                    tisp_print(f, car_ref(cur));
                    cur = cdr_ref(cur);
                } else {
                    write!(f, " . ").ok();
                    tisp_print(f, cur);
                    break;
                }
            }
            write!(f, ")").ok();
        }
    }
}

fn print_rec(f: &mut dyn Write, r: &Rec) {
    let mut rec_opt = Some(r);
    while let Some(rec) = rec_opt {
        let mut c = 0;
        for i in 0..rec.items.len() {
            if !rec.items[i].key.is_empty() {
                c += 1;
                write!(f, " {}: ", rec.items[i].key).ok();
                tisp_print(f, &rec.items[i].val);
                if c >= TSP_REC_MAX_PRINT {
                    write!(f, " ...").ok();
                    return;
                }
            }
        }
        rec_opt = rec.next.as_deref();
    }
}

// Fix tisp_print for Dec - let me override with a proper version
fn format_dec(n: f64) -> String {
    // Emulate C's %.15g format
    let s = format!("{:.15e}", n);
    // Parse and reconstruct like %g
    let s = format_g(n, 15);
    if n == (n as i32) as f64 && !s.contains('.') && !s.contains('e') && !s.contains('E') {
        format!("{}.0", s)
    } else {
        s
    }
}

fn format_g(n: f64, precision: usize) -> String {
    if n == 0.0 { return "0".to_string(); }
    let exp = n.abs().log10().floor() as i32;
    if exp >= -(1 as i32) && exp < precision as i32 {
        // Use fixed notation
        let decimal_places = if precision as i32 - 1 - exp > 0 { (precision as i32 - 1 - exp) as usize } else { 0 };
        let s = format!("{:.prec$}", n, prec = decimal_places);
        // Trim trailing zeros after decimal point
        if s.contains('.') {
            let s = s.trim_end_matches('0');
            let s = s.trim_end_matches('.');
            s.to_string()
        } else {
            s
        }
    } else {
        // Use scientific notation
        let s = format!("{:.prec$e}", n, prec = precision - 1);
        // Format exponent like C (e+01 not e1)
        if let Some(epos) = s.find('e') {
            let mantissa = &s[..epos];
            let exp_str = &s[epos+1..];
            let exp_val: i32 = exp_str.parse().unwrap_or(0);
            let mantissa = mantissa.trim_end_matches('0').trim_end_matches('.');
            if exp_val == 0 {
                mantissa.to_string()
            } else {
                format!("{}e{:+03}", mantissa, exp_val)
            }
        } else {
            s
        }
    }
}

// ---- Environment ----

pub fn tisp_env_add(st: &mut Tsp, key: &str, v: Val) {
    rec_add(&mut st.env, key, v);
}

pub fn tisp_env_init(cap: usize) -> Tsp {
    let nil = mk_nil_val();
    let none = mk_none_val();
    let t_val = Val { t: TspType::TspSym, v: ValUnion::S("True".to_string()) };

    let mut st = Tsp {
        file: String::new(), filec: 0,
        none: val_clone(&none), nil: val_clone(&nil), t: val_clone(&t_val),
        env: rec_new(cap, None),
        strs: rec_new(cap, None),
        syms: rec_new(cap, None),
        libh: Vec::new(), libhc: 0,
    };

    rec_add(&mut st.env, "True", val_clone(&t_val));
    rec_add(&mut st.env, "Nil", val_clone(&nil));
    rec_add(&mut st.env, "Void", val_clone(&none));
    rec_add(&mut st.env, "bt", mk_nil_val());
    let ver = Val { t: TspType::TspStr, v: ValUnion::S("0.1".to_string()) };
    rec_add(&mut st.strs, "0.1", val_clone(&ver));
    rec_add(&mut st.env, "version", ver);

    st
}

pub fn tisp_env_lib(st: &mut Tsp, lib: &str) {
    let old_file = st.file.clone();
    let old_filec = st.filec;
    st.file = lib.to_string();
    st.filec = 0;
    skip_ws(st, 1);
    if let Some(v) = tisp_read(st) {
        // Use a raw pointer to st.env so modifications are visible through st.env
        // This mirrors the C behavior where st->env is passed directly
        let env_ptr = &mut st.env as *mut Rec;
        unsafe {
            tisp_eval_body(st, &mut *env_ptr, v);
        }
    }
    st.file = old_file;
    st.filec = old_filec;
}

// Helper for tib_env_* functions
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

// Wrapper for mk_rec as a Prim (form)
pub fn form_mk_rec(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    mk_rec(st, rec_clone(env), args).unwrap_or_else(|| mk_none_val())
}

// Wrapper for tisp_eval_body as a Prim (form "do")
pub fn form_do(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    tisp_eval_body(st, env, args).unwrap_or_else(|| mk_none_val())
}
