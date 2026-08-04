pub const TSP_REC_MAX_PRINT: usize = 64;
pub const TSP_SYM_CHARS: &str = "_!?@#$%&~*-";
pub const TSP_REC_FACTOR: usize = 2;
#[derive(Debug, Clone, Copy)]
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
#[derive(Clone)]
pub struct Entry {
pub key: String,
pub val: Val,
}
pub type Prim = fn(Tsp, Rec, Val) -> Val;
#[derive(Clone)]
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
#[derive(Clone)]
pub struct Val {
pub t: TspType,
pub v: ValUnion,
}
#[derive(Clone)]
pub enum ValUnion {
S(String),
N { num: f64, den: f64 },
Pr { name: String, pr: Prim },
F { name: String, args: Box<Val>, body: Box<Val>, env: Rec },
P { car: Box<Val>, cdr: Box<Val> },
R(Rec),
}

// ========== helper utilities ==========

pub fn type_id(t: TspType) -> u32 { t as u32 }

pub fn type_match(t: TspType, mask: u32) -> bool {
    (type_id(t) & mask) != 0
}

pub fn nilp(v: &Val) -> bool {
    matches!(v.t, TspType::TspNil)
}

pub fn nonep(v: &Val) -> bool {
    matches!(v.t, TspType::TspNone)
}

pub fn pairp(v: &Val) -> bool {
    matches!(v.t, TspType::TspPair)
}

pub fn val_car(v: &Val) -> Option<&Val> {
    if let ValUnion::P { car, .. } = &v.v { Some(car) } else { None }
}

pub fn val_cdr(v: &Val) -> Option<&Val> {
    if let ValUnion::P { cdr, .. } = &v.v { Some(cdr) } else { None }
}

pub fn val_num(v: &Val) -> f64 {
    if let ValUnion::N { num, .. } = &v.v { *num } else { 0.0 }
}

pub fn val_den(v: &Val) -> f64 {
    if let ValUnion::N { den, .. } = &v.v { *den } else { 1.0 }
}

pub fn val_str(v: &Val) -> &str {
    if let ValUnion::S(s) = &v.v { s.as_str() } else { "" }
}

pub fn nil_val() -> Val {
    Val { t: TspType::TspNil, v: ValUnion::N { num: 0.0, den: 0.0 } }
}

pub fn none_val() -> Val {
    Val { t: TspType::TspNone, v: ValUnion::N { num: 0.0, den: 0.0 } }
}

pub fn warn(msg: &str) {
    eprintln!("; tisp: error: {}", msg);
}

// ========== primitives ==========

pub fn mk_val(t: TspType) -> Val {
    Val { t, v: ValUnion::N { num: 0.0, den: 0.0 } }
}

pub fn mk_int(i: i32) -> Val {
    Val { t: TspType::TspInt, v: ValUnion::N { num: i as f64, den: 1.0 } }
}

pub fn mk_dec(d: f64) -> Option<Val> {
    Some(Val { t: TspType::TspDec, v: ValUnion::N { num: d, den: 1.0 } })
}

pub fn mk_rat(num: i32, den: i32) -> Option<Val> {
    if den == 0 {
        warn("division by zero");
        return None;
    }
    let mut n = num;
    let mut d = den;
    frac_reduce(&mut n, &mut d);
    if d < 0 {
        d = -d;
        n = -n;
    }
    if d == 1 {
        return Some(mk_int(n));
    }
    Some(Val { t: TspType::TspRatio, v: ValUnion::N { num: n as f64, den: d as f64 } })
}

pub fn mk_str(st: &mut Tsp, s: &str) -> Option<Val> {
    if let Some(v) = rec_get(&st.strs, s) {
        return Some(v);
    }
    let val = Val { t: TspType::TspStr, v: ValUnion::S(s.to_string()) };
    rec_add(&mut st.strs, s, val.clone());
    Some(val)
}

pub fn mk_sym(st: &mut Tsp, s: &str) -> Option<Val> {
    if let Some(v) = rec_get(&st.syms, s) {
        return Some(v);
    }
    let val = Val { t: TspType::TspSym, v: ValUnion::S(s.to_string()) };
    rec_add(&mut st.syms, s, val.clone());
    Some(val)
}

pub fn mk_prim(t: TspType, pr: Prim, name: &str) -> Option<Val> {
    Some(Val {
        t,
        v: ValUnion::Pr { name: name.to_string(), pr },
    })
}

pub fn mk_func(t: TspType, name: &str, args: Val, body: Val, env: Rec) -> Option<Val> {
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

pub fn mk_rec(st: &mut Tsp, env: Rec, assoc: Val) -> Option<Val> {
    // simplified: build a record from environment, ignoring assoc evaluation
    if matches!(assoc.t, TspType::TspNone) {
        return Some(Val { t: TspType::TspRec, v: ValUnion::R(env) });
    }
    let cap = TSP_REC_FACTOR * tsp_lstlen(&assoc).max(1) as usize;
    let mut new_rec = rec_new(if cap > 0 { cap } else { 4 }, None);
    let mut cur = &assoc;
    while pairp(cur) {
        if let Some(item) = val_car(cur) {
            // each item should be a pair with sym/str key and value
            if pairp(item) {
                if let (Some(key), Some(rest)) = (val_car(item), val_cdr(item)) {
                    if matches!(key.t, TspType::TspSym | TspType::TspStr) {
                        if let Some(val) = val_car(rest) {
                            rec_add(&mut new_rec, val_str(key), val.clone());
                        }
                    }
                }
            }
        }
        cur = match val_cdr(cur) {
            Some(c) => c,
            None => break,
        };
    }
    Some(Val { t: TspType::TspRec, v: ValUnion::R(new_rec) })
}

pub fn mk_pair(a: Val, b: Val) -> Option<Val> {
    Some(Val {
        t: TspType::TspPair,
        v: ValUnion::P { car: Box::new(a), cdr: Box::new(b) },
    })
}

pub fn mk_list(_st: &mut Tsp, n: i32, args: Vec<Val>) -> Option<Val> {
    if n <= 0 || args.is_empty() {
        return Some(nil_val());
    }
    let mut result = nil_val();
    for v in args.into_iter().rev() {
        result = mk_pair(v, result)?;
    }
    Some(result)
}

pub fn tsp_lstlen(v: &Val) -> i32 {
    let mut len = 0;
    let mut current = v;
    loop {
        if !pairp(current) {
            break;
        }
        len += 1;
        current = match val_cdr(current) {
            Some(c) => c,
            None => break,
        };
    }
    if nilp(current) { len } else { -(len + 1) }
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

pub fn vals_eq(a: &Val, b: &Val) -> bool {
    let a_num = type_match(a.t, TSP_NUM);
    let b_num = type_match(b.t, TSP_NUM);
    if a_num && b_num {
        return val_num(a) == val_num(b) && val_den(a) == val_den(b);
    }
    if type_id(a.t) != type_id(b.t) {
        return false;
    }
    match (&a.v, &b.v) {
        (ValUnion::P { car: ac, cdr: acd }, ValUnion::P { car: bc, cdr: bcd }) => {
            vals_eq(ac, bc) && vals_eq(acd, bcd)
        }
        (ValUnion::F { args: aa, body: ab, .. }, ValUnion::F { args: ba, body: bb, .. }) => {
            vals_eq(aa, ba) && vals_eq(ab, bb)
        }
        (ValUnion::S(a), ValUnion::S(b)) => a == b,
        (ValUnion::Pr { name: a, .. }, ValUnion::Pr { name: b, .. }) => a == b,
        _ => match a.t {
            TspType::TspNil | TspType::TspNone => true,
            _ => false,
        },
    }
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
    if b != 0 {
        *num = *num / b;
        *den = *den / b;
    }
}

// ========== records ==========

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
        items.push(Entry { key: String::new(), val: nil_val() });
    }
    Rec { size: 0, cap: cap as i32, items, next }
}

fn entry_get_idx(rec: &Rec, key: &str) -> usize {
    let cap = rec.cap as usize;
    let mut i = (hash(key) as usize) % cap;
    loop {
        let e = &rec.items[i];
        if e.key.is_empty() || e.key == key {
            return i;
        }
        i += 1;
        if i == cap {
            i = 0;
        }
    }
}

pub fn entry_get<'a>(rec: &'a Rec, key: &'a str) -> Option<&'a Entry> {
    if rec.cap <= 0 {
        return None;
    }
    let i = entry_get_idx(rec, key);
    Some(&rec.items[i])
}

pub fn rec_get(rec: &Rec, key: &str) -> Option<Val> {
    let mut current: Option<&Rec> = Some(rec);
    while let Some(r) = current {
        if r.cap > 0 {
            let i = entry_get_idx(r, key);
            let e = &r.items[i];
            if !e.key.is_empty() {
                return Some(e.val.clone());
            }
        }
        current = r.next.as_deref();
    }
    None
}

pub fn rec_add(rec: &mut Rec, key: &str, val: Val) {
    let i = entry_get_idx(rec, key);
    let was_empty = rec.items[i].key.is_empty();
    rec.items[i].val = val;
    if was_empty {
        rec.items[i].key = key.to_string();
        rec.size += 1;
        if rec.size > rec.cap / TSP_REC_FACTOR as i32 {
            rec_grow(rec);
        }
    }
}

pub fn rec_grow(rec: &mut Rec) {
    let new_cap = (rec.cap as usize) * TSP_REC_FACTOR;
    let mut new_items = Vec::with_capacity(new_cap);
    for _ in 0..new_cap {
        new_items.push(Entry { key: String::new(), val: nil_val() });
    }
    let oitems = std::mem::replace(&mut rec.items, new_items);
    rec.cap = new_cap as i32;
    rec.size = 0;
    for entry in oitems {
        if !entry.key.is_empty() {
            let key = entry.key.clone();
            rec_add(rec, &key, entry.val);
        }
    }
}

pub fn rec_extend(rec: &mut Rec, args: Val, vals: Val) -> Rec {
    let argnum = (TSP_REC_FACTOR as i32) * tsp_lstlen(&args);
    let cap = if argnum > 0 { argnum } else { -argnum + 1 };
    let mut ret = rec_new(cap as usize, Some(Box::new(rec.clone())));
    let mut cur_args = args;
    let mut cur_vals = vals;
    loop {
        if nilp(&cur_args) {
            break;
        }
        let (arg, val, next_args, next_vals);
        if pairp(&cur_args) {
            if let ValUnion::P { car: a_car, cdr: a_cdr } = cur_args.v {
                arg = *a_car;
                next_args = *a_cdr;
            } else {
                break;
            }
            if pairp(&cur_vals) {
                if let ValUnion::P { car: v_car, cdr: v_cdr } = cur_vals.v {
                    val = *v_car;
                    next_vals = *v_cdr;
                } else {
                    break;
                }
            } else {
                val = cur_vals;
                next_vals = nil_val();
            }
        } else {
            arg = cur_args;
            val = cur_vals;
            next_args = nil_val();
            next_vals = nil_val();
        }
        let was_pair = pairp(&next_args) || nilp(&next_args);
        if matches!(arg.t, TspType::TspSym) {
            let key = val_str(&arg).to_string();
            rec_add(&mut ret, &key, val);
        }
        if !was_pair {
            break;
        }
        cur_args = next_args;
        cur_vals = next_vals;
    }
    ret
}

// ========== reader ==========

pub fn is_sym(c: char) -> bool {
    (c >= 'a' && c <= 'z')
        || (c >= 'A' && c <= 'Z')
        || (c >= '0' && c <= '9')
        || TSP_SYM_CHARS.contains(c)
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

fn fget(st: &Tsp) -> char {
    let bytes = st.file.as_bytes();
    if st.filec >= bytes.len() {
        '\0'
    } else {
        bytes[st.filec] as char
    }
}

fn fgetat(st: &Tsp, off: i64) -> char {
    let pos = st.filec as i64 + off;
    if pos < 0 {
        return '\0';
    }
    let pos = pos as usize;
    let bytes = st.file.as_bytes();
    if pos >= bytes.len() {
        '\0'
    } else {
        bytes[pos] as char
    }
}

pub fn skip_ws(st: &mut Tsp, skipnl: i32) {
    let ws: &[u8] = if skipnl != 0 {
        b" \t\n\r"
    } else {
        b" \t"
    };
    loop {
        let c = fget(st);
        if c == '\0' {
            break;
        }
        if !ws.contains(&(c as u8)) && c != ';' {
            break;
        }
        // skip whitespace
        while st.filec < st.file.len() {
            let c = fget(st);
            if c == '\0' || !ws.contains(&(c as u8)) {
                break;
            }
            st.filec += 1;
        }
        // skip comments
        while fget(st) == ';' {
            // skip to newline
            while st.filec < st.file.len() {
                let c = fget(st);
                if c == '\n' || c == '\0' {
                    break;
                }
                st.filec += 1;
            }
            if skipnl == 0 {
                // don't consume the newline
                break;
            } else {
                if st.filec < st.file.len() {
                    st.filec += 1;
                }
            }
        }
    }
}

pub fn read_sign(st: &mut Tsp) -> i32 {
    match fget(st) {
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
        let c = fget(st);
        if !c.is_ascii_digit() {
            break;
        }
        ret = ret.saturating_mul(10).saturating_add((c as i32) - ('0' as i32));
        st.filec += 1;
    }
    ret
}

pub fn read_sci(st: &mut Tsp, val: f64, isint: i32) -> Option<Val> {
    let mut val = val;
    let c = fget(st);
    if c.to_ascii_lowercase() == 'e' {
        st.filec += 1;
        let sign = if read_sign(st) == 1 { 10.0 } else { 0.1 };
        let mut expo = read_int(st);
        while expo > 0 {
            val *= sign;
            expo -= 1;
        }
        return mk_dec(val);
    }
    if isint != 0 {
        Some(mk_int(val as i32))
    } else {
        mk_dec(val)
    }
}

pub fn read_num(st: &mut Tsp) -> Val {
    let sign = read_sign(st);
    let num = read_int(st);
    match fget(st) {
        '/' => {
            st.filec += 1;
            let rest = &st.file[st.filec..];
            if !isnum(rest) {
                warn("incorrect ratio format, no denominator found");
                return nil_val();
            }
            let den_sign = read_sign(st);
            let den = read_int(st);
            mk_rat(sign * num, den_sign * den).unwrap_or_else(nil_val)
        }
        '.' => {
            st.filec += 1;
            let oldc = st.filec;
            let mut d = read_int(st) as f64;
            let mut size = st.filec - oldc;
            while size > 0 {
                d /= 10.0;
                size -= 1;
            }
            read_sci(st, sign as f64 * (num as f64 + d), 0).unwrap_or_else(nil_val)
        }
        _ => read_sci(st, sign as f64 * num as f64, 1).unwrap_or_else(nil_val),
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

pub fn esc_str(s: &str, len: i32, do_esc: i32) -> String {
    let bytes = s.as_bytes();
    let mut out = String::with_capacity(len as usize);
    let mut i = 0usize;
    let total = (len as usize).min(bytes.len());
    while i < total {
        let c = bytes[i] as char;
        if c == '\\' && do_esc != 0 && i + 1 < bytes.len() {
            i += 1;
            out.push(esc_char(bytes[i] as char));
        } else {
            out.push(c);
        }
        i += 1;
    }
    out
}

pub fn read_str(st: &mut Tsp, mk_fn: fn(&mut Tsp, &str) -> Val) -> Option<Val> {
    st.filec += 1; // skip starting open quote
    let start = st.filec;
    // determine endchar based on start char by using a heuristic:
    // we check the char before start
    let endchar = if st.filec >= 1 && st.file.as_bytes().get(st.filec - 1) == Some(&b'"') {
        '"'
    } else {
        '~'
    };
    let mut len: usize = 0;
    while fget(st) != endchar {
        if fget(st) == '\0' {
            warn("reached end before closing quote");
            return None;
        }
        if fget(st) == '\\' && fgetat(st, -1) != '\\' {
            st.filec += 1;
            len += 1;
        }
        st.filec += 1;
        len += 1;
    }
    st.filec += 1;
    let raw = &st.file[start..start + len].to_string();
    let do_esc = if endchar == '"' { 1 } else { 0 };
    let s = esc_str(raw, len as i32, do_esc);
    Some(mk_fn(st, &s))
}

pub fn read_sym(st: &mut Tsp, is_char: fn(char) -> bool) -> Option<Val> {
    let start = st.filec;
    let mut len = 0usize;
    while st.filec < st.file.len() {
        let c = fget(st);
        if c == '\0' || !is_char(c) {
            break;
        }
        st.filec += 1;
        len += 1;
    }
    let raw = st.file[start..start + len].to_string();
    let s = esc_str(&raw, len as i32, 0);
    mk_sym(st, &s)
}

pub fn read_pair(st: &mut Tsp, endchar: char) -> Option<Val> {
    let skipnl = if endchar != '\n' { 1 } else { 0 };
    skip_ws(st, skipnl);
    let mut items: Vec<Val> = Vec::new();
    let mut tail: Option<Val> = None;
    while fget(st) != '\0' && fget(st) != endchar {
        let v = tisp_read(st)?;
        if matches!(v.t, TspType::TspSym) && val_str(&v) == "." {
            skip_ws(st, skipnl);
            let v2 = tisp_read(st)?;
            tail = Some(v2);
            break;
        }
        items.push(v);
        skip_ws(st, skipnl);
    }
    skip_ws(st, skipnl);
    if skipnl != 0 && fget(st) != endchar {
        warn(&format!("did not find closing '{}'", endchar));
    }
    if st.filec < st.file.len() {
        st.filec += 1;
    }
    let mut result = tail.unwrap_or_else(nil_val);
    for v in items.into_iter().rev() {
        result = mk_pair(v, result)?;
    }
    Some(result)
}

pub fn tisp_read_sexpr(st: &mut Tsp) -> Option<Val> {
    skip_ws(st, 1);
    if st.filec >= st.file.len() {
        return Some(none_val());
    }
    let rest = &st.file[st.filec..];
    if rest.is_empty() {
        return Some(none_val());
    }
    if isnum(rest) {
        return Some(read_num(st));
    }
    let c = fget(st);
    if c == '"' {
        return read_str_inner(st, '"', true);
    }
    if c == '~' {
        return read_str_inner(st, '~', false);
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
    for (prefix, name) in prefixes {
        if rest.starts_with(prefix) {
            let advance = prefix.len() - if prefix.as_bytes().get(1) == Some(&b'"') { 1 } else { 0 };
            st.filec += advance;
            let v = tisp_read(st)?;
            let sym = mk_sym(st, name)?;
            return mk_pair(sym, mk_pair(v, nil_val())?);
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
    warn(&format!("could not read given input '{}'", c));
    None
}

fn read_str_inner(st: &mut Tsp, endchar: char, do_esc: bool) -> Option<Val> {
    st.filec += 1; // skip starting quote
    let start = st.filec;
    let mut len = 0usize;
    while fget(st) != endchar {
        if fget(st) == '\0' {
            warn("reached end before closing quote");
            return None;
        }
        if fget(st) == '\\' && fgetat(st, -1) != '\\' {
            st.filec += 1;
            len += 1;
        }
        st.filec += 1;
        len += 1;
    }
    if st.filec < st.file.len() {
        st.filec += 1;
    }
    let raw = st.file[start..start + len].to_string();
    let s = esc_str(&raw, len as i32, if do_esc { 1 } else { 0 });
    if do_esc {
        mk_str(st, &s)
    } else {
        mk_sym(st, &s)
    }
}

pub fn tisp_read(st: &mut Tsp) -> Option<Val> {
    let mut v = tisp_read_sexpr(st)?;
    while matches!(fget(st), '(' | ':' | '>' | '{') {
        v = tisp_read_sugar(st, v)?;
    }
    Some(v)
}

pub fn tisp_read_sugar(st: &mut Tsp, v: Val) -> Option<Val> {
    let c = fget(st);
    if c == '(' {
        st.filec += 1;
        let lst = read_pair(st, ')')?;
        return mk_pair(v, lst);
    }
    if c == '{' {
        st.filec += 1;
        let lst = read_pair(st, '}')?;
        let recmerge = mk_sym(st, "recmerge")?;
        let rec_sym = mk_sym(st, "Rec")?;
        let inner = mk_pair(rec_sym, lst)?;
        let l1 = mk_pair(inner, nil_val())?;
        let l2 = mk_pair(v, l1)?;
        return mk_pair(recmerge, l2);
    }
    if c == ':' {
        st.filec += 1;
        let nc = fget(st);
        if nc == '(' {
            st.filec += 1;
            let w = read_pair(st, ')')?;
            let map_sym = mk_sym(st, "map")?;
            let inner = mk_pair(v, w)?;
            return mk_pair(map_sym, inner);
        } else if nc == ':' {
            st.filec += 1;
            let w = read_sym(st, is_sym)?;
            let quote_sym = mk_sym(st, "quote")?;
            let q = mk_pair(quote_sym, mk_pair(w, nil_val())?)?;
            return mk_pair(v, mk_pair(q, nil_val())?);
        } else {
            skip_ws(st, 1);
            let w = tisp_read(st)?;
            return mk_pair(v, mk_pair(w, nil_val())?);
        }
    }
    if c == '>' && fgetat(st, 1) == '>' {
        st.filec += 2;
        let w = tisp_read(st)?;
        if !pairp(&w) {
            warn("invalid UFCS");
            return None;
        }
        if let ValUnion::P { car, cdr } = w.v {
            return mk_pair(*car, mk_pair(v, *cdr)?);
        }
    }
    Some(v)
}

pub fn tisp_read_line(st: &mut Tsp, level: i32) -> Option<Val> {
    let ret = read_pair(st, '\n')?;
    let mut ret = if !pairp(&ret) {
        mk_pair(ret, nil_val())?
    } else {
        ret
    };
    // collect indented lines as sub-expressions appended to last cdr
    // We'll build the result iteratively by appending pairs.
    let mut suffix_items: Vec<Val> = Vec::new();
    while fget(st) != '\0' {
        let bytes = st.file.as_bytes();
        let mut newlevel = 0i32;
        let mut j = st.filec;
        while j < bytes.len() && (bytes[j] == b'\t' || bytes[j] == b' ') {
            newlevel += 1;
            j += 1;
        }
        if newlevel <= level {
            break;
        }
        st.filec += newlevel as usize;
        let sub = tisp_read_line(st, newlevel)?;
        suffix_items.push(sub);
    }
    if !suffix_items.is_empty() {
        // attach at end
        // Build a proper list: append each to the end of `ret`
        for s in suffix_items {
            ret = append_to_list(ret, s);
        }
    }
    // if only 1 element, return just it
    if let ValUnion::P { car, cdr } = &ret.v {
        if nilp(cdr) {
            return Some((**car).clone());
        }
    }
    Some(ret)
}

fn append_to_list(lst: Val, item: Val) -> Val {
    if !pairp(&lst) {
        return mk_pair(lst, mk_pair(item, nil_val()).unwrap_or_else(nil_val))
            .unwrap_or_else(nil_val);
    }
    if let ValUnion::P { car, cdr } = lst.v {
        let new_cdr = append_to_list(*cdr, item);
        return mk_pair(*car, new_cdr).unwrap_or_else(nil_val);
    }
    lst
}

// ========== eval ==========

pub fn tisp_eval_list(st: &mut Tsp, env: &mut Rec, v: Val) -> Option<Val> {
    let mut items: Vec<Val> = Vec::new();
    let mut tail = nil_val();
    let mut cur = v;
    loop {
        if nilp(&cur) {
            break;
        }
        if !pairp(&cur) {
            // last element of improper list
            let ev = tisp_eval_with_env(st, env, cur)?;
            tail = ev;
            break;
        }
        if let ValUnion::P { car, cdr } = cur.v {
            let ev = tisp_eval_with_env(st, env, *car)?;
            items.push(ev);
            cur = *cdr;
        } else {
            break;
        }
    }
    let mut result = tail;
    for v in items.into_iter().rev() {
        result = mk_pair(v, result)?;
    }
    Some(result)
}

pub fn tisp_eval_body(st: &mut Tsp, env: &mut Rec, v: Val) -> Option<Val> {
    let mut ret = none_val();
    let mut cur = v;
    while pairp(&cur) {
        if let ValUnion::P { car, cdr } = cur.v {
            ret = tisp_eval_with_env(st, env, *car)?;
            cur = *cdr;
        } else {
            break;
        }
    }
    Some(ret)
}

pub fn prepend_bt(_st: &mut Tsp, _env: &mut Rec, _f: Val) {
    // simplified: do nothing for backtrace
}

pub fn eval_proc(st: &mut Tsp, env: &mut Rec, f: Val, args: Val) -> Option<Val> {
    match f.t {
        TspType::TspPrim => {
            let evaluated = tisp_eval_list(st, env, args)?;
            if let ValUnion::Pr { name, .. } = &f.v {
                let name = name.clone();
                return dispatch_prim(&name, st, env, evaluated);
            }
            Some(none_val())
        }
        TspType::TspForm => {
            if let ValUnion::Pr { name, .. } = &f.v {
                let name = name.clone();
                return dispatch_prim(&name, st, env, args);
            }
            Some(none_val())
        }
        TspType::TspFunc => {
            let evaluated = tisp_eval_list(st, env, args)?;
            if let ValUnion::F { args: f_args, body, env: f_env, .. } = f.v {
                let mut new_env = rec_extend(&mut f_env.clone(), *f_args, evaluated);
                tisp_eval_body(st, &mut new_env, *body)
            } else {
                Some(none_val())
            }
        }
        TspType::TspMacro => {
            if let ValUnion::F { args: f_args, body, env: f_env, .. } = f.v {
                let mut new_env = rec_extend(&mut f_env.clone(), *f_args, args);
                let result = tisp_eval_body(st, &mut new_env, *body)?;
                tisp_eval_with_env(st, env, result)
            } else {
                Some(none_val())
            }
        }
        TspType::TspRec => {
            let evaluated = tisp_eval_list(st, env, args)?;
            if let ValUnion::R(r) = &f.v {
                if let Some(first) = val_car(&evaluated) {
                    if matches!(first.t, TspType::TspSym) {
                        if let Some(found) = rec_get(r, val_str(first)) {
                            return Some(found);
                        }
                        if let Some(found) = rec_get(r, "else") {
                            return Some(found);
                        }
                    }
                }
            }
            warn("could not find element in record");
            None
        }
        _ => {
            warn(&format!(
                "attempt to evaluate non procedural type {}",
                tsp_type_str(f.t)
            ));
            None
        }
    }
}

pub fn dispatch_prim(name: &str, st: &mut Tsp, env: &mut Rec, args: Val) -> Option<Val> {
    use crate::core::*;
    use crate::io::*;
    use crate::math::*;
    use crate::os::*;
    use crate::string::*;
    match name {
        // core primitives
        "car" => Some(prim_car(st, env, args)),
        "cdr" => Some(prim_cdr(st, env, args)),
        "cons" => Some(prim_cons(st, env, args)),
        "quote" => Some(form_quote(st, env, args)),
        "eval" => Some(prim_eval(st, env, args)),
        "=" | "eq" => Some(prim_eq(st, env, args)),
        "cond" => Some(form_cond(st, env, args)),
        "do" => tisp_eval_body(st, env, args),
        "typeof" => Some(prim_typeof(st, env, args)),
        "procprops" => Some(prim_procprops(st, env, args)),
        "Func" => Some(form_Func(st, env, args)),
        "Macro" => Some(form_Macro(st, env, args)),
        "error" => Some(prim_error(st, env, args)),
        "Rec" => Some(form_Rec(st, env, args)),
        "recmerge" => Some(prim_recmerge(st, env, args)),
        "records" => Some(prim_records(st, env, args)),
        "def" => Some(form_def(st, env, args)),
        "undefine!" => Some(form_undefine(st, env, args)),
        "defined?" => Some(form_definedp(st, env, args)),
        // string primitives
        "Str" => Some(prim_Str(st, env, args)),
        "Sym" => Some(prim_Sym(st, env, args)),
        "strlen" => Some(prim_strlen(st, env, args)),
        "strformat" => Some(form_strformat(st, env, args)),
        // math primitives
        "+" | "add" => Some(prim_add(st, env, args)),
        "-" | "sub" => Some(prim_sub(st, env, args)),
        "*" | "mul" => Some(prim_mul(st, env, args)),
        "/" | "div" => Some(prim_div(st, env, args)),
        "mod" => Some(prim_mod(st, env, args)),
        "^" | "pow" => Some(prim_pow(st, env, args)),
        "denominator" => Some(prim_denominator(st, env, args)),
        "numerator" => Some(prim_numerator(st, env, args)),
        "Int" => Some(prim_int(st, env, args)),
        "Dec" => Some(prim_dec(st, env, args)),
        "round" => Some(prim_round(st, env, args)),
        "floor" => Some(prim_floor(st, env, args)),
        "ceil" => Some(prim_ceil(st, env, args)),
        "<" => Some(prim_lt(st, env, args)),
        ">" => Some(prim_gt(st, env, args)),
        "<=" => Some(prim_lte(st, env, args)),
        ">=" => Some(prim_gte(st, env, args)),
        // io primitives
        "write" => Some(prim_write(st, env, args)),
        "read" => Some(prim_read(st, env, args)),
        "parse" => Some(prim_parse(st, env, args)),
        "load" => Some(prim_load(st, env, args)),
        // os primitives
        "cd!" => Some(prim_cd(st, env, args)),
        "pwd" => Some(prim_pwd(st, env, args)),
        "exit!" => Some(prim_exit(st, env, args)),
        "now" => Some(prim_now(st, env, args)),
        "time" => Some(form_time(st, env, args)),
        _ => {
            warn(&format!("unknown primitive '{}'", name));
            Some(none_val())
        }
    }
}

pub fn tisp_eval(st: &mut Tsp, v: Val) -> Option<Val> {
    let mut env = std::mem::replace(&mut st.env, rec_new(1, None));
    let result = tisp_eval_with_env(st, &mut env, v);
    st.env = env;
    result
}

pub fn tisp_eval_with_env(st: &mut Tsp, env: &mut Rec, v: Val) -> Option<Val> {
    match v.t {
        TspType::TspSym => {
            let key = val_str(&v).to_string();
            if let Some(found) = rec_get(env, &key) {
                Some(found)
            } else if let Some(found) = rec_get(&st.env, &key) {
                Some(found)
            } else {
                warn(&format!("could not find symbol '{}'", key));
                None
            }
        }
        TspType::TspPair => {
            if let ValUnion::P { car, cdr } = v.v {
                let f = tisp_eval_with_env(st, env, *car)?;
                eval_proc(st, env, f, *cdr)
            } else {
                Some(v)
            }
        }
        _ => Some(v),
    }
}

// ========== print ==========

pub fn tisp_print(f: &mut std::fs::File, v: &Val) {
    use std::io::Write;
    let s = tisp_print_to_string(v);
    let _ = f.write_all(s.as_bytes());
}

pub fn tisp_print_to_string(v: &Val) -> String {
    let mut out = String::new();
    print_val(&mut out, v);
    out
}

fn print_val(out: &mut String, v: &Val) {
    match v.t {
        TspType::TspNone => out.push_str("Void"),
        TspType::TspNil => out.push_str("Nil"),
        TspType::TspInt => {
            let n = val_num(v) as i64;
            out.push_str(&n.to_string());
        }
        TspType::TspDec => {
            let n = val_num(v);
            // mimic %.15g
            let s = format_g(n);
            out.push_str(&s);
            if n == (n as i64) as f64 {
                out.push_str(".0");
            }
        }
        TspType::TspRatio => {
            out.push_str(&format!("{}/{}", val_num(v) as i64, val_den(v) as i64));
        }
        TspType::TspStr | TspType::TspSym => {
            out.push_str(val_str(v));
        }
        TspType::TspFunc => {
            if let ValUnion::F { name, .. } = &v.v {
                if !name.is_empty() {
                    out.push_str(&format!("#<function:{}>", name));
                } else {
                    out.push_str("#<function>");
                }
            }
        }
        TspType::TspMacro => {
            if let ValUnion::F { name, .. } = &v.v {
                if !name.is_empty() {
                    out.push_str(&format!("#<macro:{}>", name));
                } else {
                    out.push_str("#<macro>");
                }
            }
        }
        TspType::TspPrim => {
            if let ValUnion::Pr { name, .. } = &v.v {
                out.push_str(&format!("#<primitive:{}>", name));
            }
        }
        TspType::TspForm => {
            if let ValUnion::Pr { name, .. } = &v.v {
                out.push_str(&format!("#<form:{}>", name));
            }
        }
        TspType::TspRec => {
            out.push('{');
            if let ValUnion::R(r) = &v.v {
                let mut current: Option<&Rec> = Some(r);
                while let Some(rec) = current {
                    let mut count = 0;
                    let mut printed = 0;
                    for entry in rec.items.iter() {
                        if !entry.key.is_empty() {
                            count += 1;
                            if count > TSP_REC_MAX_PRINT as i32 {
                                out.push_str(" ...");
                                break;
                            }
                            out.push_str(&format!(" {}: ", entry.key));
                            print_val(out, &entry.val);
                            printed += 1;
                            if printed >= rec.size {
                                break;
                            }
                        }
                    }
                    current = rec.next.as_deref();
                }
            }
            out.push_str(" }");
        }
        TspType::TspPair => {
            out.push('(');
            if let Some(car) = val_car(v) {
                print_val(out, car);
            }
            let mut cur = val_cdr(v);
            while let Some(c) = cur {
                if pairp(c) {
                    out.push(' ');
                    if let Some(ccar) = val_car(c) {
                        print_val(out, ccar);
                    }
                    cur = val_cdr(c);
                } else if nilp(c) {
                    break;
                } else {
                    out.push_str(" . ");
                    print_val(out, c);
                    break;
                }
            }
            out.push(')');
        }
    }
}

fn format_g(n: f64) -> String {
    if n == 0.0 {
        return "0".to_string();
    }
    let abs_n = n.abs();
    if abs_n < 1e-4 || abs_n >= 1e15 {
        // scientific
        format!("{:e}", n)
    } else {
        let s = format!("{:.15}", n);
        // trim trailing zeros and possibly the decimal point
        let s = s.trim_end_matches('0').trim_end_matches('.').to_string();
        if s.is_empty() {
            "0".to_string()
        } else {
            s
        }
    }
}

// ========== environment ==========

pub fn tisp_env_add(st: &mut Tsp, key: &str, v: Val) {
    rec_add(&mut st.env, key, v);
}

pub fn tisp_env_init(cap: usize) -> Tsp {
    let strs = rec_new(cap, None);
    let syms = rec_new(cap, None);

    let nil = Val { t: TspType::TspNil, v: ValUnion::N { num: 0.0, den: 0.0 } };
    let none = Val { t: TspType::TspNone, v: ValUnion::N { num: 0.0, den: 0.0 } };
    let t = Val { t: TspType::TspSym, v: ValUnion::S("True".to_string()) };

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

    let true_val = st.t.clone();
    tisp_env_add(&mut st, "True", true_val);
    let nil_val_clone = st.nil.clone();
    tisp_env_add(&mut st, "Nil", nil_val_clone);
    let none_val_clone = st.none.clone();
    tisp_env_add(&mut st, "Void", none_val_clone);
    let nil_for_bt = st.nil.clone();
    tisp_env_add(&mut st, "bt", nil_for_bt);
    let version = mk_str(&mut st, "0.1").unwrap_or_else(nil_val);
    tisp_env_add(&mut st, "version", version);

    st
}

pub fn tisp_env_lib(st: &mut Tsp, lib: &str) {
    let prev_file = std::mem::take(&mut st.file);
    let prev_filec = st.filec;
    st.file = lib.to_string();
    st.filec = 0;
    skip_ws(st, 1);
    if let Some(v) = tisp_read(st) {
        let mut env = std::mem::replace(&mut st.env, rec_new(1, None));
        let _ = tisp_eval_body(st, &mut env, v);
        st.env = env;
    }
    st.file = prev_file;
    st.filec = prev_filec;
}

// ========== builtins (trampoline placeholders) ==========

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
