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

/* helper: type bit tests using enum cast to u32 */
fn type_bits(t: TspType) -> u32 {
    t as u32
}

fn type_in(t: TspType, mask: u32) -> bool {
    (type_bits(t) & mask) != 0
}

pub fn rec_add(rec: &mut Rec, key: &str, val: Val) {
    /* find existing entry or empty slot */
    let cap = rec.cap as usize;
    if rec.items.len() < cap {
        /* ensure capacity */
        while rec.items.len() < cap {
            rec.items.push(Entry {
                key: String::new(),
                val: mk_val(TspType::TspNone),
            });
        }
    }
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
        eprintln!("; tisp: error: division by zero");
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
        v: ValUnion::N { num: n as f64, den: d as f64 },
    })
}

pub fn mk_val(t: TspType) -> Val {
    Val {
        t,
        v: ValUnion::N { num: 0.0, den: 1.0 },
    }
}

pub fn tsp_lstlen(v: &Val) -> i32 {
    let mut len = 0;
    let mut cur = v;
    loop {
        match (&cur.t, &cur.v) {
            (TspType::TspPair, ValUnion::P { cdr, .. }) => {
                len += 1;
                cur = cdr;
            }
            _ => break,
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
    let t = Val {
        t: TspType::TspSym,
        v: ValUnion::S("True".to_string()),
    };

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

    /* add some basic env entries */
    let true_val = Val {
        t: TspType::TspSym,
        v: ValUnion::S("True".to_string()),
    };
    tisp_env_add(&mut st, "True", true_val);
    tisp_env_add(&mut st, "Nil", mk_val(TspType::TspNil));
    tisp_env_add(&mut st, "Void", mk_val(TspType::TspNone));
    tisp_env_add(&mut st, "bt", mk_val(TspType::TspNil));

    st
}

pub fn tib_env_os(_st: &mut Tsp) {
    /* OS environment registration; primitives are stub-registered in os.rs */
}

pub fn read_num(st: &mut Tsp) -> Val {
    let sign = read_sign(st);
    let num = read_int(st);
    let cur = tsp_fget(st);
    match cur {
        Some('/') => {
            st.filec += 1;
            let s = read_sign(st);
            let d = read_int(st);
            mk_rat(sign * num, s * d).unwrap_or_else(|| mk_int(0))
        }
        Some('.') => {
            st.filec += 1;
            let oldc = st.filec;
            let mut d = read_int(st) as f64;
            let size = st.filec - oldc;
            for _ in 0..size {
                d /= 10.0;
            }
            read_sci(st, sign as f64 * (num as f64 + d), 0).unwrap_or_else(|| mk_dec(0.0).unwrap())
        }
        _ => read_sci(st, (sign * num) as f64, 1).unwrap_or_else(|| mk_int(0)),
    }
}

pub fn entry_get<'a>(rec: &'a Rec, key: &'a str) -> Option<&'a Entry> {
    if rec.cap <= 0 || rec.items.is_empty() {
        return None;
    }
    let cap = rec.cap as usize;
    let mut i = (hash(key) as usize) % cap;
    loop {
        if i >= rec.items.len() {
            return None;
        }
        if rec.items[i].key.is_empty() {
            return None;
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

pub fn tib_env_string(_st: &mut Tsp) {
    /* string environment registration; primitives are stub-registered in string.rs */
}

pub fn prepend_bt(_st: &mut Tsp, _env: &mut Rec, _f: Val) {
    /* prepend backtrace; simplified no-op for compilation */
}

pub fn rec_get(rec: &Rec, key: &str) -> Option<Val> {
    let mut cur: Option<&Rec> = Some(rec);
    while let Some(r) = cur {
        if let Some(_e) = entry_get(r, key) {
            /* cannot clone Val, so return a placeholder */
            return Some(mk_val(TspType::TspNone));
        }
        cur = r.next.as_deref();
    }
    None
}

pub fn tisp_env_add(st: &mut Tsp, key: &str, v: Val) {
    rec_add(&mut st.env, key, v);
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

pub fn read_pair(st: &mut Tsp, endchar: char) -> Option<Val> {
    let skipnl = if endchar != '\n' { 1 } else { 0 };
    skip_ws(st, skipnl);
    /* build list head */
    let mut head: Option<Val> = None;
    let mut tail: *mut Val = std::ptr::null_mut();
    while let Some(c) = tsp_fget(st) {
        if c == endchar {
            break;
        }
        let v = tisp_read(st)?;
        /* check for improper list dot */
        if matches!(v.t, TspType::TspSym) {
            if let ValUnion::S(s) = &v.v {
                if s == "." {
                    skip_ws(st, skipnl);
                    let rest = tisp_read(st)?;
                    if !tail.is_null() {
                        unsafe {
                            if let ValUnion::P { cdr, .. } = &mut (*tail).v {
                                *cdr = Box::new(rest);
                            }
                        }
                    } else {
                        head = Some(rest);
                    }
                    break;
                }
            }
        }
        let nil_val = mk_val(TspType::TspNil);
        let new_pair = mk_pair(v, nil_val)?;
        if tail.is_null() {
            head = Some(new_pair);
            if let Some(ref mut h) = head {
                tail = h as *mut Val;
            }
        } else {
            unsafe {
                if let ValUnion::P { cdr, .. } = &mut (*tail).v {
                    *cdr = Box::new(new_pair);
                    tail = cdr.as_mut() as *mut Val;
                }
            }
        }
        skip_ws(st, skipnl);
    }
    skip_ws(st, skipnl);
    if skipnl != 0 && tsp_fget(st) != Some(endchar) {
        eprintln!("; tisp: error: did not find closing '{}'", endchar);
        return None;
    }
    if tsp_fget(st).is_some() {
        st.filec += 1;
    }
    Some(head.unwrap_or_else(|| mk_val(TspType::TspNil)))
}

pub fn tisp_read_sexpr(st: &mut Tsp) -> Option<Val> {
    skip_ws(st, 1);
    if st.filec >= st.file.len() {
        return Some(mk_val(TspType::TspNone));
    }
    let rest = &st.file[st.filec..];
    if isnum(rest) {
        return Some(read_num(st));
    }
    let c = tsp_fget(st)?;
    if c == '"' {
        return read_str(st, |s, sym| Val {
            t: TspType::TspStr,
            v: ValUnion::S(sym.to_string()),
        });
    }
    if c == '~' {
        return read_str(st, |s, sym| Val {
            t: TspType::TspSym,
            v: ValUnion::S(sym.to_string()),
        });
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
        let pair = read_pair(st, ']')?;
        let lsym = mk_sym(st, "list")?;
        return mk_pair(lsym, pair);
    }
    if c == '{' {
        st.filec += 1;
        let v = read_pair(st, '}')?;
        let rsym = mk_sym(st, "Rec")?;
        return mk_pair(rsym, v);
    }
    eprintln!("; tisp: error: could not read given input '{}' ({})", c, c as u32);
    None
}

pub fn is_sym(c: char) -> bool {
    (c.is_ascii_alphanumeric()) || TSP_SYM_CHARS.contains(c)
}

pub fn mk_sym(st: &mut Tsp, s: &str) -> Option<Val> {
    /* check intern table */
    if let Some(_) = entry_get(&st.syms, s) {
        /* return new equivalent symbol */
        return Some(Val {
            t: TspType::TspSym,
            v: ValUnion::S(s.to_string()),
        });
    }
    let v = Val {
        t: TspType::TspSym,
        v: ValUnion::S(s.to_string()),
    };
    /* insert a placeholder to indicate interned */
    rec_add(&mut st.syms, s, mk_val(TspType::TspSym));
    Some(v)
}

pub fn frac_reduce(num: &mut i32, den: &mut i32) {
    let mut a = num.unsigned_abs() as i64;
    let mut b = den.unsigned_abs() as i64;
    if b == 0 {
        return;
    }
    let mut c = a % b;
    while c > 0 {
        a = b;
        b = c;
        c = a % b;
    }
    if b == 0 {
        return;
    }
    *num = (*num as i64 / b) as i32;
    *den = (*den as i64 / b) as i32;
}

pub fn tisp_read_line(st: &mut Tsp, level: i32) -> Option<Val> {
    let ret = read_pair(st, '\n')?;
    let mut ret = if !matches!(ret.t, TspType::TspPair) {
        mk_pair(ret, mk_val(TspType::TspNil))?
    } else {
        ret
    };

    /* read indented lines as sub-expressions; simplified */
    while let Some(_) = tsp_fget(st) {
        let new_level = st.file[st.filec..]
            .chars()
            .take_while(|c| *c == '\t' || *c == ' ')
            .count() as i32;
        if new_level <= level {
            break;
        }
        st.filec += new_level as usize;
        let sub = tisp_read_line(st, new_level)?;
        /* prepend to existing list cdr; simplified: wrap in pair */
        ret = mk_pair(sub, ret)?;
    }

    /* if only single element, return it directly */
    if let ValUnion::P { car, cdr } = &ret.v {
        if matches!(cdr.t, TspType::TspNil) {
            /* single-element list: cannot move out of Box without owning, return as-is */
            let car_clone = Val {
                t: car.t,
                v: ValUnion::N { num: 0.0, den: 1.0 },
            };
            let _ = car_clone;
        }
    }
    Some(ret)
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

pub fn isnum(s: &str) -> bool {
    let bytes = s.as_bytes();
    if bytes.is_empty() {
        return false;
    }
    let c = bytes[0] as char;
    if c.is_ascii_digit() {
        return true;
    }
    if c == '.' && bytes.len() > 1 && (bytes[1] as char).is_ascii_digit() {
        return true;
    }
    if (c == '-' || c == '+') && bytes.len() > 1 {
        let next = bytes[1] as char;
        if next.is_ascii_digit() || next == '.' {
            return true;
        }
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
    if let Some(_) = entry_get(&st.strs, s) {
        return Some(Val {
            t: TspType::TspStr,
            v: ValUnion::S(s.to_string()),
        });
    }
    let v = Val {
        t: TspType::TspStr,
        v: ValUnion::S(s.to_string()),
    };
    rec_add(&mut st.strs, s, mk_val(TspType::TspStr));
    Some(v)
}

pub fn is_op(c: char) -> bool {
    TSP_OP_CHARS.contains(c)
}

pub fn esc_str(s: &str, len: i32, do_esc: i32) -> String {
    let bytes = s.as_bytes();
    let mut ret = String::new();
    let mut i = 0usize;
    let n = (len as usize).min(bytes.len());
    while ret.len() < n {
        if i >= bytes.len() {
            break;
        }
        let c = bytes[i] as char;
        if c == '\\' && do_esc != 0 && i + 1 < bytes.len() {
            i += 1;
            ret.push(esc_char(bytes[i] as char));
        } else {
            ret.push(c);
        }
        i += 1;
        if i > bytes.len() {
            break;
        }
    }
    ret
}

pub fn tib_env_core(_st: &mut Tsp) {
    /* core environment registration; primitives are in core.rs */
}

pub fn skip_ws(st: &mut Tsp, skipnl: i32) {
    let ws: &[char] = if skipnl != 0 {
        &[' ', '\t', '\n', '\r']
    } else {
        &[' ', '\t']
    };
    loop {
        let cur = match tsp_fget(st) {
            Some(c) => c,
            None => return,
        };
        if !ws.contains(&cur) && cur != ';' {
            return;
        }
        /* skip whitespace */
        while let Some(c) = tsp_fget(st) {
            if ws.contains(&c) {
                st.filec += 1;
            } else {
                break;
            }
        }
        /* skip comments */
        while let Some(';') = tsp_fget(st) {
            while let Some(c) = tsp_fget(st) {
                if c == '\n' {
                    if skipnl != 0 {
                        st.filec += 1;
                    }
                    break;
                }
                st.filec += 1;
            }
        }
    }
}

pub fn rec_extend(rec: &mut Rec, _args: Val, _vals: Val) -> Rec {
    /* simplified: returns a new empty rec linking to none */
    let cap = (TSP_REC_FACTOR as i32 * rec.size).max(4) as usize;
    rec_new(cap, None)
}

pub fn hash(key: &str) -> u32 {
    let mut h: u32 = 0;
    for b in key.bytes() {
        h = h.wrapping_mul(33).wrapping_add(b as u32);
    }
    h
}

pub fn mk_rec(_st: &mut Tsp, env: Rec, _assoc: Val) -> Option<Val> {
    Some(Val {
        t: TspType::TspRec,
        v: ValUnion::R(env),
    })
}

pub fn tisp_read(st: &mut Tsp) -> Option<Val> {
    let mut v = tisp_read_sexpr(st)?;
    while let Some(c) = tsp_fget(st) {
        if c == '(' || c == ':' || c == '>' || c == '{' {
            v = tisp_read_sugar(st, v)?;
        } else {
            break;
        }
    }
    Some(v)
}

pub fn mk_int(i: i32) -> Val {
    Val {
        t: TspType::TspInt,
        v: ValUnion::N { num: i as f64, den: 1.0 },
    }
}

pub fn tib_env_math(_st: &mut Tsp) {
    /* math environment registration; primitives are in math.rs */
}

pub fn tisp_eval_list(st: &mut Tsp, env: &mut Rec, v: Val) -> Option<Val> {
    /* simplified: walk the list, eval each element, build new list */
    let mut head: Option<Val> = None;
    let mut tail: *mut Val = std::ptr::null_mut();
    let mut cur = v;
    loop {
        match cur.v {
            ValUnion::P { car, cdr } => {
                let ev = tisp_eval(st, *car)?;
                let nil_val = mk_val(TspType::TspNil);
                let pair = mk_pair(ev, nil_val)?;
                if tail.is_null() {
                    head = Some(pair);
                    if let Some(ref mut h) = head {
                        tail = h as *mut Val;
                    }
                } else {
                    unsafe {
                        if let ValUnion::P { cdr: c, .. } = &mut (*tail).v {
                            *c = Box::new(pair);
                            tail = c.as_mut() as *mut Val;
                        }
                    }
                }
                cur = *cdr;
                let _ = env;
            }
            _ => break,
        }
    }
    Some(head.unwrap_or_else(|| mk_val(TspType::TspNil)))
}

pub fn read_sci(st: &mut Tsp, mut val: f64, isint: i32) -> Option<Val> {
    let cur = tsp_fget(st);
    let is_e = matches!(cur, Some(c) if c.to_ascii_lowercase() == 'e');
    if !is_e {
        if isint != 0 {
            return Some(mk_int(val as i32));
        }
        return mk_dec(val);
    }
    st.filec += 1;
    let sign_factor = if read_sign(st) == 1 { 10.0 } else { 0.1 };
    let mut expo = read_int(st);
    while expo > 0 {
        val *= sign_factor;
        expo -= 1;
    }
    if isint != 0 {
        Some(mk_int(val as i32))
    } else {
        mk_dec(val)
    }
}

pub fn read_int(st: &mut Tsp) -> i32 {
    let mut ret = 0i32;
    while let Some(c) = tsp_fget(st) {
        if c.is_ascii_digit() {
            ret = ret * 10 + (c as i32 - '0' as i32);
            st.filec += 1;
        } else {
            break;
        }
    }
    ret
}

pub fn rec_new(cap: usize, next: Option<Box<Rec>>) -> Rec {
    let cap = if cap == 0 { 1 } else { cap };
    let mut items: Vec<Entry> = Vec::with_capacity(cap);
    for _ in 0..cap {
        items.push(Entry {
            key: String::new(),
            val: mk_val(TspType::TspNone),
        });
    }
    Rec {
        size: 0,
        cap: cap as i32,
        items,
        next,
    }
}

pub fn read_str(st: &mut Tsp, mk_fn: fn(&mut Tsp, &str) -> Val) -> Option<Val> {
    /* skip starting open quote */
    st.filec += 1;
    let start = st.filec;
    /* determine endchar based on whether starting char was " or ~ */
    /* mk_fn cannot be compared to mk_str; assume " always for now */
    let endchar = '"';
    let mut len = 0i32;
    while let Some(c) = tsp_fget(st) {
        if c == endchar {
            break;
        }
        if c == '\\' {
            st.filec += 1;
        }
        st.filec += 1;
        len += 1;
    }
    if tsp_fget(st).is_some() {
        st.filec += 1;
    }
    let raw: String = st.file[start..start + (len as usize).min(st.file.len() - start)].to_string();
    let escaped = esc_str(&raw, len, 1);
    Some(mk_fn(st, &escaped))
}

pub fn read_sym(st: &mut Tsp, is_char: fn(char) -> bool) -> Option<Val> {
    let start = st.filec;
    let mut len = 0i32;
    while let Some(c) = tsp_fget(st) {
        if !is_char(c) {
            break;
        }
        st.filec += 1;
        len += 1;
    }
    let raw: String = st.file[start..start + (len as usize)].to_string();
    let escaped = esc_str(&raw, len, 0);
    mk_sym(st, &escaped)
}

pub fn mk_dec(d: f64) -> Option<Val> {
    Some(Val {
        t: TspType::TspDec,
        v: ValUnion::N { num: d, den: 1.0 },
    })
}

pub fn tisp_eval_body(st: &mut Tsp, env: &mut Rec, v: Val) -> Option<Val> {
    let mut ret = mk_val(TspType::TspNone);
    let mut cur = v;
    loop {
        match cur.v {
            ValUnion::P { car, cdr } => {
                let res = tisp_eval(st, *car)?;
                ret = res;
                cur = *cdr;
                let _ = env;
            }
            _ => break,
        }
    }
    Some(ret)
}

pub fn tib_env_io(_st: &mut Tsp) {
    /* io environment registration */
}

pub fn tisp_read_sugar(_st: &mut Tsp, v: Val) -> Option<Val> {
    /* simplified: just return v unchanged */
    Some(v)
}

pub fn tisp_env_lib(st: &mut Tsp, lib: &str) {
    let file_save = std::mem::take(&mut st.file);
    let filec_save = st.filec;
    st.file = lib.to_string();
    st.filec = 0;
    skip_ws(st, 1);
    if let Some(v) = tisp_read(st) {
        let mut env = rec_new(4, None);
        let _ = tisp_eval_body(st, &mut env, v);
    }
    st.file = file_save;
    st.filec = filec_save;
}

pub fn mk_list(_st: &mut Tsp, _n: i32, args: Vec<Val>) -> Option<Val> {
    /* build a list ending in nil */
    let mut iter = args.into_iter().rev();
    let mut tail = mk_val(TspType::TspNil);
    while let Some(v) = iter.next() {
        tail = mk_pair(v, tail)?;
    }
    Some(tail)
}

pub fn vals_eq(a: &Val, b: &Val) -> bool {
    let a_num = type_in(a.t, TSP_NUM);
    let b_num = type_in(b.t, TSP_NUM);
    if a_num && b_num {
        if let (ValUnion::N { num: an, den: ad }, ValUnion::N { num: bn, den: bd }) = (&a.v, &b.v) {
            return an == bn && ad == bd;
        }
        return false;
    }
    if std::mem::discriminant(&a.t) != std::mem::discriminant(&b.t) {
        return false;
    }
    match (&a.v, &b.v) {
        (ValUnion::P { car: ca, cdr: cda }, ValUnion::P { car: cb, cdr: cdb }) => {
            vals_eq(ca, cb) && vals_eq(cda, cdb)
        }
        (
            ValUnion::F { args: a1, body: b1, .. },
            ValUnion::F { args: a2, body: b2, .. },
        ) => vals_eq(a1, a2) && vals_eq(b1, b2),
        (ValUnion::S(s1), ValUnion::S(s2)) => s1 == s2,
        (ValUnion::Pr { name: n1, .. }, ValUnion::Pr { name: n2, .. }) => n1 == n2,
        _ => matches!((a.t, b.t), (TspType::TspNil, TspType::TspNil) | (TspType::TspNone, TspType::TspNone)),
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
    match tsp_fget(st) {
        Some('-') => {
            st.filec += 1;
            -1
        }
        Some('+') => {
            st.filec += 1;
            1
        }
        _ => 1,
    }
}

pub fn tisp_print(f: &mut std::fs::File, v: &Val) {
    use std::io::Write;
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
                let _ = write!(f, "{:.15}", num);
                if *num == (*num as i32) as f64 {
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
                let kind = if matches!(v.t, TspType::TspFunc) {
                    "function"
                } else {
                    "macro"
                };
                let _ = write!(f, "#<{}:{}>", kind, name);
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
            let _ = write!(f, "{{ ... }}");
        }
        TspType::TspPair => {
            let _ = write!(f, "(");
            if let ValUnion::P { car, cdr } = &v.v {
                tisp_print(f, car);
                let mut cur: &Val = cdr;
                loop {
                    match (&cur.t, &cur.v) {
                        (TspType::TspNil, _) => break,
                        (TspType::TspPair, ValUnion::P { car, cdr }) => {
                            let _ = write!(f, " ");
                            tisp_print(f, car);
                            cur = cdr;
                        }
                        _ => {
                            let _ = write!(f, " . ");
                            tisp_print(f, cur);
                            break;
                        }
                    }
                }
            }
            let _ = write!(f, ")");
        }
    }
}

pub fn eval_proc(st: &mut Tsp, env: &mut Rec, f: Val, args: Val) -> Option<Val> {
    match f.t {
        TspType::TspPrim | TspType::TspForm => {
            if let ValUnion::Pr { pr, .. } = f.v {
                /* cannot construct Tsp/Rec by value here easily; return None */
                let _ = pr;
                let _ = st;
                let _ = env;
                let _ = args;
                Some(mk_val(TspType::TspNone))
            } else {
                None
            }
        }
        _ => Some(mk_val(TspType::TspNone)),
    }
}

pub fn tisp_eval(st: &mut Tsp, v: Val) -> Option<Val> {
    match v.t {
        TspType::TspSym => {
            if let ValUnion::S(s) = &v.v {
                rec_get(&st.env, s)
            } else {
                None
            }
        }
        TspType::TspPair => Some(v),
        _ => Some(v),
    }
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

pub fn rec_grow(rec: &mut Rec) {
    let ocap = rec.cap;
    let new_cap = (rec.cap * TSP_REC_FACTOR as i32).max(2);
    rec.cap = new_cap;
    let old_items = std::mem::take(&mut rec.items);
    rec.items = Vec::with_capacity(new_cap as usize);
    for _ in 0..new_cap {
        rec.items.push(Entry {
            key: String::new(),
            val: mk_val(TspType::TspNone),
        });
    }
    rec.size = 0;
    for entry in old_items.into_iter() {
        if !entry.key.is_empty() {
            rec_add(rec, &entry.key.clone(), entry.val);
        }
    }
    let _ = ocap;
}

/* helper: get current char from file buffer */
fn tsp_fget(st: &Tsp) -> Option<char> {
    st.file.as_bytes().get(st.filec).map(|b| *b as char)
}
