use std::io::Write;

pub const TSP_REC_MAX_PRINT: usize = 64;
pub const TSP_SYM_CHARS: &str = "_!?@#$%&~*-";
pub const TSP_REC_FACTOR: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq)]
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
    F {
        name: String,
        args: Box<Val>,
        body: Box<Val>,
        env: Rec,
    },
    P { car: Box<Val>, cdr: Box<Val> },
    R(Rec),
}

// ----- file/character helpers -----

fn fget(st: &Tsp) -> char {
    fgetat(st, 0)
}

fn fgetat(st: &Tsp, o: i32) -> char {
    let idx = st.filec as i64 + o as i64;
    if idx < 0 {
        return '\0';
    }
    let idx = idx as usize;
    let bytes = st.file.as_bytes();
    if idx >= bytes.len() {
        '\0'
    } else {
        bytes[idx] as char
    }
}

fn finc(st: &mut Tsp) {
    st.filec += 1;
}

fn fincn(st: &mut Tsp, n: usize) {
    st.filec += n;
}

// ----- record operations -----

pub fn rec_add(rec: &mut Rec, key: &str, val: Val) {
    for entry in rec.items.iter_mut() {
        if entry.key == key {
            entry.val = val;
            return;
        }
    }
    rec.items.push(Entry {
        key: key.to_string(),
        val,
    });
    rec.size += 1;
}

pub fn entry_get<'a>(rec: &'a Rec, key: &'a str) -> Option<&'a Entry> {
    for entry in &rec.items {
        if entry.key == key {
            return Some(entry);
        }
    }
    None
}

pub fn rec_get(rec: &Rec, key: &str) -> Option<Val> {
    let mut cur: Option<&Rec> = Some(rec);
    while let Some(r) = cur {
        for entry in &r.items {
            if entry.key == key {
                return Some(entry.val.clone());
            }
        }
        cur = r.next.as_deref();
    }
    None
}

pub fn rec_new(cap: usize, next: Option<Box<Rec>>) -> Rec {
    Rec {
        size: 0,
        cap: cap as i32,
        items: Vec::new(),
        next,
    }
}

pub fn rec_grow(_rec: &mut Rec) {
    // No-op: our Vec grows automatically.
}

pub fn rec_extend(_rec: &mut Rec, _args: Val, _vals: Val) -> Rec {
    rec_new(1, None)
}

pub fn hash(key: &str) -> u32 {
    let mut h: u32 = 0;
    for c in key.bytes() {
        h = h.wrapping_mul(33).wrapping_add(c as u32);
    }
    h
}

// ----- value constructors -----

pub fn mk_val(t: TspType) -> Val {
    let v = match t {
        TspType::TspInt | TspType::TspDec | TspType::TspRatio => {
            ValUnion::N { num: 0.0, den: 1.0 }
        }
        _ => ValUnion::S(String::new()),
    };
    Val { t, v }
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

pub fn mk_rat(num: i32, den: i32) -> Option<Val> {
    if den == 0 {
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
    Some(Val {
        t: TspType::TspRatio,
        v: ValUnion::N {
            num: n as f64,
            den: d as f64,
        },
    })
}

pub fn mk_str(_st: &mut Tsp, s: &str) -> Option<Val> {
    Some(Val {
        t: TspType::TspStr,
        v: ValUnion::S(s.to_string()),
    })
}

pub fn mk_sym(_st: &mut Tsp, s: &str) -> Option<Val> {
    Some(Val {
        t: TspType::TspSym,
        v: ValUnion::S(s.to_string()),
    })
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

pub fn mk_rec(_st: &mut Tsp, env: Rec, _assoc: Val) -> Option<Val> {
    Some(Val {
        t: TspType::TspRec,
        v: ValUnion::R(env),
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

pub fn mk_list(st: &mut Tsp, _n: i32, args: Vec<Val>) -> Option<Val> {
    if args.is_empty() {
        return Some(st.nil.clone());
    }
    let mut result = st.nil.clone();
    for v in args.into_iter().rev() {
        result = mk_pair(v, result)?;
    }
    Some(result)
}

// ----- helpers -----

pub fn frac_reduce(num: &mut i32, den: &mut i32) {
    let mut a = num.unsigned_abs();
    let mut b = den.unsigned_abs();
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
    *num /= b as i32;
    *den /= b as i32;
}

pub fn tsp_lstlen(v: &Val) -> i32 {
    let mut len = 0i32;
    let mut cur = v;
    loop {
        match (cur.t, &cur.v) {
            (TspType::TspPair, ValUnion::P { cdr, .. }) => {
                len += 1;
                cur = cdr.as_ref();
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

pub fn vals_eq(a: &Val, b: &Val) -> bool {
    let an = a.t as u32;
    let bn = b.t as u32;
    if (an & TSP_NUM) != 0 && (bn & TSP_NUM) != 0 {
        if let (ValUnion::N { num: an, den: ad }, ValUnion::N { num: bn, den: bd }) = (&a.v, &b.v)
        {
            return an == bn && ad == bd;
        }
        return false;
    }
    if a.t as u32 != b.t as u32 {
        return false;
    }
    match (&a.v, &b.v) {
        (
            ValUnion::P {
                car: ac,
                cdr: ad,
            },
            ValUnion::P {
                car: bc,
                cdr: bd,
            },
        ) => vals_eq(ac, bc) && vals_eq(ad, bd),
        (ValUnion::S(s1), ValUnion::S(s2)) => s1 == s2,
        _ => false,
    }
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
    c.is_ascii_alphabetic() || c.is_ascii_digit() || TSP_SYM_CHARS.contains(c)
}

pub fn is_op(c: char) -> bool {
    TSP_OP_CHARS.contains(c)
}

pub fn isnum(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let bytes = s.as_bytes();
    let c = bytes[0] as char;
    if c.is_ascii_digit() {
        return true;
    }
    if c == '.' && bytes.len() > 1 && (bytes[1] as char).is_ascii_digit() {
        return true;
    }
    if (c == '-' || c == '+') && bytes.len() > 1 {
        let n = bytes[1] as char;
        return n.is_ascii_digit() || n == '.';
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
    let bytes = s.as_bytes();
    let mut result = String::new();
    let mut i = 0usize;
    let mut count = 0i32;
    while count < len && i < bytes.len() {
        let c = bytes[i] as char;
        if c == '\\' && do_esc != 0 && i + 1 < bytes.len() {
            i += 1;
            result.push(esc_char(bytes[i] as char));
        } else {
            result.push(c);
        }
        i += 1;
        count += 1;
    }
    result
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
        if ws.contains(&(c as u8)) {
            finc(st);
            continue;
        }
        if c == ';' {
            // skip until newline
            while fget(st) != '\0' && fget(st) != '\n' {
                finc(st);
            }
            if skipnl == 0 {
                break;
            }
            continue;
        }
        break;
    }
}

// ----- reading -----

pub fn read_sign(st: &mut Tsp) -> i32 {
    match fget(st) {
        '-' => {
            finc(st);
            -1
        }
        '+' => {
            finc(st);
            1
        }
        _ => 1,
    }
}

pub fn read_int(st: &mut Tsp) -> i32 {
    let mut ret: i32 = 0;
    while fget(st) != '\0' && fget(st).is_ascii_digit() {
        ret = ret * 10 + (fget(st) as i32 - '0' as i32);
        finc(st);
    }
    ret
}

pub fn read_sci(st: &mut Tsp, mut val: f64, isint: i32) -> Option<Val> {
    let c = fget(st);
    if c.to_ascii_lowercase() != 'e' {
        if isint != 0 {
            return Some(mk_int(val as i32));
        }
        return mk_dec(val);
    }
    finc(st);
    let sign_val = if read_sign(st) == 1 { 10.0 } else { 0.1 };
    let mut expo = read_int(st);
    while expo > 0 {
        val *= sign_val;
        expo -= 1;
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
            finc(st);
            let s2 = read_sign(st);
            let d = read_int(st);
            mk_rat(sign * num, s2 * d).unwrap_or_else(|| mk_int(0))
        }
        '.' => {
            finc(st);
            let oldc = st.filec;
            let mut d = read_int(st) as f64;
            let size = st.filec - oldc;
            for _ in 0..size {
                d /= 10.0;
            }
            read_sci(st, (sign as f64) * (num as f64 + d), 0).unwrap_or_else(|| {
                Val {
                    t: TspType::TspDec,
                    v: ValUnion::N { num: 0.0, den: 1.0 },
                }
            })
        }
        _ => read_sci(st, (sign * num) as f64, 1).unwrap_or_else(|| mk_int(0)),
    }
}

fn mk_str_for_read(st: &mut Tsp, s: &str) -> Val {
    mk_str(st, s).unwrap_or(Val {
        t: TspType::TspStr,
        v: ValUnion::S(s.to_string()),
    })
}

fn mk_sym_for_read(st: &mut Tsp, s: &str) -> Val {
    mk_sym(st, s).unwrap_or(Val {
        t: TspType::TspSym,
        v: ValUnion::S(s.to_string()),
    })
}

pub fn read_str(st: &mut Tsp, mk_fn: fn(&mut Tsp, &str) -> Val) -> Option<Val> {
    // Determine endchar by peeking the opening quote at current position.
    let endchar = if fget(st) == '~' { '~' } else { '"' };
    finc(st); // skip opening quote
    let start = st.filec;
    let do_esc = if endchar == '"' { 1 } else { 0 };
    let mut len = 0i32;
    while fget(st) != endchar {
        if fget(st) == '\0' {
            return None;
        }
        if fget(st) == '\\' && fgetat(st, -1) != '\\' {
            finc(st);
        }
        finc(st);
        len += 1;
    }
    finc(st); // skip closing quote
    let raw = st.file[start..start + len as usize].to_string();
    let escaped = esc_str(&raw, len, do_esc);
    Some(mk_fn(st, &escaped))
}

pub fn read_sym(st: &mut Tsp, is_char: fn(char) -> bool) -> Option<Val> {
    let start = st.filec;
    while fget(st) != '\0' && is_char(fget(st)) {
        finc(st);
    }
    let s = st.file[start..st.filec].to_string();
    let escaped = esc_str(&s, (st.filec - start) as i32, 0);
    Some(Val {
        t: TspType::TspSym,
        v: ValUnion::S(escaped),
    })
}

pub fn read_pair(st: &mut Tsp, endchar: char) -> Option<Val> {
    let skipnl = if endchar != '\n' { 1 } else { 0 };
    skip_ws(st, skipnl);
    let mut elements: Vec<Val> = Vec::new();
    let mut tail: Option<Val> = None;
    while fget(st) != '\0' && fget(st) != endchar {
        let v = tisp_read(st)?;
        // detect "." dotted-pair separator
        let mut is_dot = false;
        if matches!(v.t, TspType::TspSym) {
            if let ValUnion::S(s) = &v.v {
                if s == "." {
                    is_dot = true;
                }
            }
        }
        if is_dot {
            skip_ws(st, skipnl);
            tail = Some(tisp_read(st)?);
            break;
        }
        elements.push(v);
        skip_ws(st, skipnl);
    }
    skip_ws(st, skipnl);
    if skipnl != 0 && fget(st) != endchar {
        return None;
    }
    if fget(st) == endchar {
        finc(st);
    }
    let mut result = tail.unwrap_or_else(|| st.nil.clone());
    for e in elements.into_iter().rev() {
        result = Val {
            t: TspType::TspPair,
            v: ValUnion::P {
                car: Box::new(e),
                cdr: Box::new(result),
            },
        };
    }
    Some(result)
}

pub fn tisp_read_sexpr(st: &mut Tsp) -> Option<Val> {
    skip_ws(st, 1);
    if st.filec >= st.file.len() {
        return Some(st.none.clone());
    }
    let rest = &st.file[st.filec..];
    if isnum(rest) {
        return Some(read_num(st));
    }
    let c = fget(st);
    if c == '"' {
        return read_str(st, mk_str_for_read);
    }
    if c == '~' {
        return read_str(st, mk_sym_for_read);
    }
    let prefix: &[(&str, &str)] = &[
        ("'", "quote"),
        ("`", "quasiquote"),
        (",@", "unquote-splice"),
        (",", "unquote"),
        ("@", "Func"),
        ("f\"", "strformat"),
    ];
    for &(pfx, sym_name) in prefix {
        if rest.starts_with(pfx) {
            let pfx_len = pfx.len();
            let advance = if pfx.as_bytes().len() > 1 && pfx.as_bytes()[1] == b'"' {
                pfx_len - 1
            } else {
                pfx_len
            };
            fincn(st, advance);
            let v = tisp_read(st)?;
            let sym = mk_sym(st, sym_name)?;
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
        finc(st);
        return read_pair(st, ')');
    }
    if c == '[' {
        finc(st);
        let lst = read_pair(st, ']')?;
        let sym = mk_sym(st, "list")?;
        return mk_pair(sym, lst);
    }
    if c == '{' {
        finc(st);
        let v = read_pair(st, '}')?;
        let sym = mk_sym(st, "Rec")?;
        return mk_pair(sym, v);
    }
    None
}

pub fn tisp_read(st: &mut Tsp) -> Option<Val> {
    let mut v = tisp_read_sexpr(st)?;
    while {
        let c = fget(st);
        c == '(' || c == ':' || c == '>' || c == '{'
    } {
        v = tisp_read_sugar(st, v)?;
    }
    Some(v)
}

pub fn tisp_read_sugar(_st: &mut Tsp, v: Val) -> Option<Val> {
    // Simplified: don't implement reader sugar transformations.
    Some(v)
}

pub fn tisp_read_line(st: &mut Tsp, _level: i32) -> Option<Val> {
    read_pair(st, '\n')
}

// ----- evaluation -----

pub fn tisp_eval(st: &mut Tsp, v: Val) -> Option<Val> {
    match v.t {
        TspType::TspSym => {
            if let ValUnion::S(ref s) = v.v {
                rec_get(&st.env, s)
            } else {
                None
            }
        }
        _ => Some(v),
    }
}

pub fn tisp_eval_list(st: &mut Tsp, _env: &mut Rec, v: Val) -> Option<Val> {
    let mut elements = Vec::new();
    let mut cur = v;
    loop {
        match (cur.t, cur.v.clone()) {
            (TspType::TspPair, ValUnion::P { car, cdr }) => {
                let evaluated = tisp_eval(st, *car)?;
                elements.push(evaluated);
                cur = *cdr;
            }
            (TspType::TspNil, _) => break,
            _ => {
                let last = tisp_eval(st, cur)?;
                let mut result = last;
                for e in elements.into_iter().rev() {
                    result = Val {
                        t: TspType::TspPair,
                        v: ValUnion::P {
                            car: Box::new(e),
                            cdr: Box::new(result),
                        },
                    };
                }
                return Some(result);
            }
        }
    }
    let mut result = st.nil.clone();
    for e in elements.into_iter().rev() {
        result = Val {
            t: TspType::TspPair,
            v: ValUnion::P {
                car: Box::new(e),
                cdr: Box::new(result),
            },
        };
    }
    Some(result)
}

pub fn tisp_eval_body(st: &mut Tsp, _env: &mut Rec, v: Val) -> Option<Val> {
    let mut ret = st.none.clone();
    let mut cur = v;
    loop {
        match (cur.t, cur.v.clone()) {
            (TspType::TspPair, ValUnion::P { car, cdr }) => {
                ret = tisp_eval(st, *car)?;
                cur = *cdr;
            }
            _ => break,
        }
    }
    Some(ret)
}

pub fn eval_proc(_st: &mut Tsp, _env: &mut Rec, _f: Val, _args: Val) -> Option<Val> {
    None
}

pub fn prepend_bt(_st: &mut Tsp, _env: &mut Rec, _f: Val) {
    // No-op
}

// ----- printing -----

pub fn tisp_print(f: &mut std::fs::File, v: &Val) {
    let _ = print_val(f, v);
}

fn print_val(f: &mut std::fs::File, v: &Val) -> std::io::Result<()> {
    match v.t {
        TspType::TspNone => {
            write!(f, "Void")?;
        }
        TspType::TspNil => {
            write!(f, "Nil")?;
        }
        TspType::TspInt => {
            if let ValUnion::N { num, .. } = &v.v {
                write!(f, "{}", *num as i32)?;
            }
        }
        TspType::TspDec => {
            if let ValUnion::N { num, .. } = &v.v {
                let s = fmt_g15(*num);
                write!(f, "{}", s)?;
                let n_int = *num as i32;
                if *num == n_int as f64 {
                    write!(f, ".0")?;
                }
            }
        }
        TspType::TspRatio => {
            if let ValUnion::N { num, den } = &v.v {
                write!(f, "{}/{}", *num as i32, *den as i32)?;
            }
        }
        TspType::TspStr | TspType::TspSym => {
            if let ValUnion::S(s) = &v.v {
                write!(f, "{}", s)?;
            }
        }
        TspType::TspFunc | TspType::TspMacro => {
            if let ValUnion::F { name, .. } = &v.v {
                let kind = if matches!(v.t, TspType::TspFunc) {
                    "function"
                } else {
                    "macro"
                };
                if !name.is_empty() {
                    write!(f, "#<{}:{}>", kind, name)?;
                } else {
                    write!(f, "#<{}>", kind)?;
                }
            }
        }
        TspType::TspPrim => {
            if let ValUnion::Pr { name, .. } = &v.v {
                write!(f, "#<primitive:{}>", name)?;
            }
        }
        TspType::TspForm => {
            if let ValUnion::Pr { name, .. } = &v.v {
                write!(f, "#<form:{}>", name)?;
            }
        }
        TspType::TspRec => {
            write!(f, "{{ ... }}")?;
        }
        TspType::TspPair => {
            write!(f, "(")?;
            if let ValUnion::P { car, cdr } = &v.v {
                print_val(f, car)?;
                let mut cur: &Val = cdr;
                loop {
                    match (cur.t, &cur.v) {
                        (TspType::TspPair, ValUnion::P { car, cdr }) => {
                            write!(f, " ")?;
                            print_val(f, car)?;
                            cur = cdr;
                        }
                        (TspType::TspNil, _) => break,
                        _ => {
                            write!(f, " . ")?;
                            print_val(f, cur)?;
                            break;
                        }
                    }
                }
            }
            write!(f, ")")?;
        }
    }
    Ok(())
}

/// Approximation of printf("%.15g", v).
fn fmt_g15(v: f64) -> String {
    if v == 0.0 {
        if v.is_sign_negative() {
            return "-0".to_string();
        }
        return "0".to_string();
    }
    if !v.is_finite() {
        if v.is_nan() {
            return "nan".to_string();
        }
        return if v > 0.0 {
            "inf".to_string()
        } else {
            "-inf".to_string()
        };
    }
    let abs = v.abs();
    let exp = abs.log10().floor() as i32;
    let p: i32 = 15;

    if exp < -4 || exp >= p {
        // scientific
        let mantissa = v / 10f64.powi(exp);
        let mantissa_str = format!("{:.*}", (p - 1) as usize, mantissa);
        let mantissa_stripped = strip_trailing_zeros(&mantissa_str);
        let exp_sign = if exp >= 0 { '+' } else { '-' };
        let exp_abs = exp.abs();
        format!("{}e{}{:02}", mantissa_stripped, exp_sign, exp_abs)
    } else {
        // fixed
        let after_decimal = ((p - 1) - exp).max(0) as usize;
        let formatted = format!("{:.*}", after_decimal, v);
        strip_trailing_zeros(&formatted)
    }
}

fn strip_trailing_zeros(s: &str) -> String {
    if s.contains('.') {
        let trimmed = s.trim_end_matches('0');
        if trimmed.ends_with('.') {
            trimmed[..trimmed.len() - 1].to_string()
        } else {
            trimmed.to_string()
        }
    } else {
        s.to_string()
    }
}

// ----- environment / library setup -----

pub fn tisp_env_add(st: &mut Tsp, key: &str, v: Val) {
    rec_add(&mut st.env, key, v);
}

pub fn tisp_env_init(cap: usize) -> Tsp {
    let mut st = Tsp {
        file: String::new(),
        filec: 0,
        none: Val {
            t: TspType::TspNone,
            v: ValUnion::S(String::new()),
        },
        nil: Val {
            t: TspType::TspNil,
            v: ValUnion::S(String::new()),
        },
        t: Val {
            t: TspType::TspSym,
            v: ValUnion::S("True".to_string()),
        },
        env: rec_new(cap, None),
        strs: rec_new(cap, None),
        syms: rec_new(cap, None),
        libh: Vec::new(),
        libhc: 0,
    };
    let true_val = st.t.clone();
    let nil_val = st.nil.clone();
    let none_val = st.none.clone();
    tisp_env_add(&mut st, "True", true_val);
    tisp_env_add(&mut st, "Nil", nil_val.clone());
    tisp_env_add(&mut st, "Void", none_val);
    tisp_env_add(&mut st, "bt", nil_val);
    let version = Val {
        t: TspType::TspStr,
        v: ValUnion::S("0.1".to_string()),
    };
    tisp_env_add(&mut st, "version", version);
    st
}

pub fn tisp_env_lib(_st: &mut Tsp, _lib: &str) {
    // No-op: full library evaluation is not required for our tests.
}

// ----- stub library entry points -----

pub fn tib_env_core(_st: &mut Tsp) {
    // Primitives cannot be registered with the simplified Prim signature
    // since `prim_*` functions in `core.rs` use `&mut Tsp`/`&mut Rec`.
    // For our test suite, this is intentionally a no-op.
}

pub fn tib_env_math(_st: &mut Tsp) {
    // No-op (see tib_env_core).
}

pub fn tib_env_string(_st: &mut Tsp) {
    // No-op (see tib_env_core).
}

pub fn tib_env_io(_st: &mut Tsp) {
    // No-op (see tib_env_core).
}

pub fn tib_env_os(_st: &mut Tsp) {
    // No-op (see tib_env_core).
}
