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

// ======================== Helpers ========================

fn type_bits(t: TspType) -> u32 {
    t as u32
}

fn type_matches(t: TspType, mask: u32) -> bool {
    (type_bits(t) & mask) != 0
}

pub(crate) fn val_clone(v: &Val) -> Val {
    Val {
        t: v.t,
        v: match &v.v {
            ValUnion::S(s) => ValUnion::S(s.clone()),
            ValUnion::N { num, den } => ValUnion::N { num: *num, den: *den },
            ValUnion::Pr { name, pr } => ValUnion::Pr { name: name.clone(), pr: *pr },
            ValUnion::F { name, args, body, env } => ValUnion::F {
                name: name.clone(),
                args: Box::new(val_clone(args)),
                body: Box::new(val_clone(body)),
                env: rec_clone(env),
            },
            ValUnion::P { car, cdr } => ValUnion::P {
                car: Box::new(val_clone(car)),
                cdr: Box::new(val_clone(cdr)),
            },
            ValUnion::R(r) => ValUnion::R(rec_clone(r)),
        },
    }
}

pub(crate) fn rec_clone(r: &Rec) -> Rec {
    Rec {
        size: r.size,
        cap: r.cap,
        items: r
            .items
            .iter()
            .map(|e| Entry {
                key: e.key.clone(),
                val: val_clone(&e.val),
            })
            .collect(),
        next: r.next.as_ref().map(|n| Box::new(rec_clone(n))),
    }
}

pub(crate) fn nilp(v: &Val) -> bool {
    matches!(v.t, TspType::TspNil)
}

pub(crate) fn val_num(v: &Val) -> (f64, f64) {
    match &v.v {
        ValUnion::N { num, den } => (*num, *den),
        _ => (0.0, 1.0),
    }
}

#[allow(dead_code)]
pub(crate) fn val_str(v: &Val) -> String {
    match &v.v {
        ValUnion::S(s) => s.clone(),
        _ => String::new(),
    }
}

#[allow(dead_code)]
pub(crate) fn pair_car(v: &Val) -> Option<&Val> {
    if let ValUnion::P { car, .. } = &v.v {
        Some(car)
    } else {
        None
    }
}

#[allow(dead_code)]
pub(crate) fn pair_cdr(v: &Val) -> Option<&Val> {
    if let ValUnion::P { cdr, .. } = &v.v {
        Some(cdr)
    } else {
        None
    }
}

// Empty placeholder Prim function used for constructing Val::Prim where
// the source-level signatures preclude wiring in the actual handlers.
// Actual dispatch is by-name (see `dispatch_named`).
pub fn stub_prim(_st: Tsp, _env: Rec, _args: Val) -> Val {
    Val {
        t: TspType::TspNone,
        v: ValUnion::N { num: 0.0, den: 1.0 },
    }
}

// ======================== Constructors ========================

pub fn rec_add(rec: &mut Rec, key: &str, val: Val) {
    let i = find_slot(rec, key);
    if rec.items[i].key.is_empty() && key.is_empty() {
        // overwrite empty key with empty key — degenerate
        rec.items[i].val = val;
        return;
    }
    if rec.items[i].key == key {
        rec.items[i].val = val;
    } else {
        rec.items[i] = Entry {
            key: key.to_string(),
            val,
        };
        rec.size += 1;
        if rec.size > rec.cap / TSP_REC_FACTOR as i32 {
            rec_grow(rec);
        }
    }
}

fn find_slot(rec: &Rec, key: &str) -> usize {
    if rec.cap <= 0 || rec.items.is_empty() {
        return 0;
    }
    let cap = rec.cap as usize;
    let mut i = (hash(key) as usize) % cap;
    loop {
        if rec.items[i].key.is_empty() {
            return i;
        }
        if rec.items[i].key == key {
            return i;
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
    let mut num = num;
    let mut den = den;
    frac_reduce(&mut num, &mut den);
    if den < 0 {
        den = den.abs();
        num = -num;
    }
    if den == 1 {
        return Some(mk_int(num));
    }
    Some(Val {
        t: TspType::TspRatio,
        v: ValUnion::N {
            num: num as f64,
            den: den as f64,
        },
    })
}

pub fn mk_val(t: TspType) -> Val {
    let v = match t {
        TspType::TspStr | TspType::TspSym => ValUnion::S(String::new()),
        TspType::TspInt | TspType::TspDec | TspType::TspRatio => ValUnion::N { num: 0.0, den: 1.0 },
        TspType::TspPrim | TspType::TspForm => ValUnion::Pr {
            name: String::new(),
            pr: stub_prim,
        },
        TspType::TspFunc | TspType::TspMacro => ValUnion::F {
            name: String::new(),
            args: Box::new(Val {
                t: TspType::TspNil,
                v: ValUnion::N { num: 0.0, den: 1.0 },
            }),
            body: Box::new(Val {
                t: TspType::TspNil,
                v: ValUnion::N { num: 0.0, den: 1.0 },
            }),
            env: rec_new(4, None),
        },
        TspType::TspPair => ValUnion::P {
            car: Box::new(Val {
                t: TspType::TspNil,
                v: ValUnion::N { num: 0.0, den: 1.0 },
            }),
            cdr: Box::new(Val {
                t: TspType::TspNil,
                v: ValUnion::N { num: 0.0, den: 1.0 },
            }),
        },
        TspType::TspRec => ValUnion::R(rec_new(4, None)),
        TspType::TspNone | TspType::TspNil => ValUnion::N { num: 0.0, den: 1.0 },
    };
    Val { t, v }
}

pub fn tsp_lstlen(v: &Val) -> i32 {
    let mut len = 0i32;
    let mut cur = v;
    loop {
        match cur.t {
            TspType::TspPair => {
                len += 1;
                if let ValUnion::P { cdr, .. } = &cur.v {
                    cur = cdr;
                } else {
                    break;
                }
            }
            _ => break,
        }
    }
    if nilp(cur) {
        len
    } else {
        -(len + 1)
    }
}

pub fn tisp_env_init(cap: usize) -> Tsp {
    let nil = Val {
        t: TspType::TspNil,
        v: ValUnion::N { num: 0.0, den: 1.0 },
    };
    let none = Val {
        t: TspType::TspNone,
        v: ValUnion::N { num: 0.0, den: 1.0 },
    };
    let t = Val {
        t: TspType::TspSym,
        v: ValUnion::S("True".to_string()),
    };
    let t_clone = val_clone(&t);
    let nil_clone = val_clone(&nil);
    let none_clone = val_clone(&none);
    let nil_clone2 = val_clone(&nil);
    let mut st = Tsp {
        file: String::new(),
        filec: 0,
        none,
        nil,
        t,
        env: rec_new(cap, None),
        strs: rec_new(cap, None),
        syms: rec_new(cap, None),
        libh: Vec::new(),
        libhc: 0,
    };
    tisp_env_add(&mut st, "True", t_clone);
    tisp_env_add(&mut st, "Nil", nil_clone);
    tisp_env_add(&mut st, "Void", none_clone);
    tisp_env_add(&mut st, "bt", nil_clone2);
    let ver = Val {
        t: TspType::TspStr,
        v: ValUnion::S("0.1".to_string()),
    };
    tisp_env_add(&mut st, "version", ver);
    st
}

pub fn tib_env_os(st: &mut Tsp) {
    crate::os::tib_env_os(st)
}

pub fn read_num(st: &mut Tsp) -> Val {
    let sign = read_sign(st);
    let n = read_int(st);
    let bytes = st.file.as_bytes();
    let c = if st.filec < bytes.len() {
        bytes[st.filec] as char
    } else {
        '\0'
    };
    match c {
        '/' => {
            st.filec += 1;
            // Parse denominator; must be a number, else fall back to int
            if st.filec < bytes.len() && isnum(&st.file[st.filec..]) {
                let s2 = read_sign(st);
                let d = read_int(st);
                if let Some(v) = mk_rat(sign * n, s2 * d) {
                    return v;
                }
            }
            mk_int(sign * n)
        }
        '.' => {
            st.filec += 1;
            let oldc = st.filec;
            let mut d = read_int(st) as f64;
            let size = st.filec - oldc;
            for _ in 0..size {
                d /= 10.0;
            }
            match read_sci(st, sign as f64 * (n as f64 + d), 0) {
                Some(v) => v,
                None => mk_dec(sign as f64 * (n as f64 + d)).unwrap_or_else(|| mk_int(0)),
            }
        }
        _ => match read_sci(st, sign as f64 * n as f64, 1) {
            Some(v) => v,
            None => mk_int(sign * n),
        },
    }
}

pub fn entry_get<'a>(rec: &'a Rec, key: &'a str) -> Option<&'a Entry> {
    if rec.cap <= 0 || rec.items.is_empty() {
        return None;
    }
    let i = find_slot(rec, key);
    if rec.items[i].key == key && !key.is_empty() {
        Some(&rec.items[i])
    } else {
        None
    }
}

pub fn tib_env_string(st: &mut Tsp) {
    crate::string::tib_env_string(st)
}

pub fn prepend_bt(_st: &mut Tsp, _env: &mut Rec, _f: Val) {
    // Backtrace recording — best-effort no-op. The base "bt" var is initialised
    // to nil in tisp_env_init.
}

pub fn rec_get(rec: &Rec, key: &str) -> Option<Val> {
    let mut cur = Some(rec);
    while let Some(r) = cur {
        if let Some(e) = entry_get(r, key) {
            return Some(val_clone(&e.val));
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
    let mut items: Vec<Val> = Vec::new();
    let mut tail: Option<Val> = None;
    loop {
        let bytes = st.file.as_bytes();
        if st.filec >= bytes.len() {
            break;
        }
        let c = bytes[st.filec] as char;
        if c == endchar {
            break;
        }
        let v = tisp_read(st)?;
        // Detect "." dotted pair marker
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
    let bytes = st.file.as_bytes();
    if skipnl != 0 {
        let cur = if st.filec < bytes.len() {
            bytes[st.filec] as char
        } else {
            '\0'
        };
        if cur != endchar {
            return None;
        }
    }
    if st.filec < bytes.len() {
        st.filec += 1;
    }
    // Build linked list
    let nil_val = Val {
        t: TspType::TspNil,
        v: ValUnion::N { num: 0.0, den: 1.0 },
    };
    let mut acc = tail.unwrap_or(nil_val);
    for v in items.into_iter().rev() {
        acc = mk_pair(v, acc)?;
    }
    Some(acc)
}

pub fn tisp_read_sexpr(st: &mut Tsp) -> Option<Val> {
    skip_ws(st, 1);
    let bytes = st.file.as_bytes();
    if st.filec >= bytes.len() {
        return Some(val_clone(&st.none));
    }
    let rest = &st.file[st.filec..];
    if rest.is_empty() {
        return Some(val_clone(&st.none));
    }
    if isnum(rest) {
        return Some(read_num(st));
    }
    let c = bytes[st.filec] as char;
    if c == '"' {
        return read_str(st, |st, s| mk_str(st, s).unwrap_or_else(|| val_clone(&st.none)));
    }
    if c == '~' {
        return read_str(st, |st, s| mk_sym(st, s).unwrap_or_else(|| val_clone(&st.none)));
    }
    let prefix: &[(&str, &str)] = &[
        ("'", "quote"),
        ("`", "quasiquote"),
        (",@", "unquote-splice"),
        (",", "unquote"),
        ("@", "Func"),
        ("f\"", "strformat"),
    ];
    for (pre, sym) in prefix {
        if rest.starts_with(pre) {
            let strip = if pre.ends_with('"') { 1 } else { 0 };
            st.filec += pre.len() - strip;
            let v = tisp_read(st)?;
            let symv = mk_sym(st, sym)?;
            return mk_list(st, 2, vec![symv, v]);
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
        let s = mk_sym(st, "list")?;
        return mk_pair(s, lst);
    }
    if c == '{' {
        st.filec += 1;
        let v = read_pair(st, '}')?;
        let s = mk_sym(st, "Rec")?;
        return mk_pair(s, v);
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
    let v = Val {
        t: TspType::TspSym,
        v: ValUnion::S(s.to_string()),
    };
    rec_add(&mut st.syms, s, val_clone(&v));
    Some(v)
}

pub fn frac_reduce(num: &mut i32, den: &mut i32) {
    if *den == 0 {
        return;
    }
    let mut a = num.unsigned_abs() as i32;
    let mut b = den.unsigned_abs() as i32;
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
        *num /= b;
        *den /= b;
    }
}

pub fn tisp_read_line(st: &mut Tsp, level: i32) -> Option<Val> {
    let ret = read_pair(st, '\n')?;
    let mut ret = if !matches!(ret.t, TspType::TspPair) {
        let nil_val = val_clone(&st.nil);
        mk_pair(ret, nil_val)?
    } else {
        ret
    };
    // Walk to last pair where cdr is pair, then conditionally read sub-expressions.
    // We collect tail extensions in a vec and rebuild for safety.
    let mut tail_exprs: Vec<Val> = Vec::new();
    loop {
        let bytes = st.file.as_bytes();
        if st.filec >= bytes.len() {
            break;
        }
        // Count tabs/spaces
        let mut newlevel = 0i32;
        let mut i = st.filec;
        while i < bytes.len() && (bytes[i] == b'\t' || bytes[i] == b' ') {
            newlevel += 1;
            i += 1;
        }
        if newlevel <= level {
            break;
        }
        st.filec += newlevel as usize;
        let sub = tisp_read_line(st, newlevel)?;
        tail_exprs.push(sub);
    }
    // append tail_exprs to the end of ret list
    if !tail_exprs.is_empty() {
        ret = append_to_list(ret, tail_exprs, val_clone(&st.nil));
    }
    // if only 1 element, return car
    if let ValUnion::P { car, cdr } = &ret.v {
        if nilp(cdr) {
            return Some(val_clone(car));
        }
    }
    Some(ret)
}

fn append_to_list(list: Val, exprs: Vec<Val>, nil: Val) -> Val {
    // walks list pairs, when cdr is nil, replace with (expr . next)
    fn rec(v: Val, exprs: &mut Vec<Val>, nil: &Val) -> Val {
        match v.v {
            ValUnion::P { car, cdr } => {
                let new_cdr = rec(*cdr, exprs, nil);
                Val {
                    t: TspType::TspPair,
                    v: ValUnion::P {
                        car,
                        cdr: Box::new(new_cdr),
                    },
                }
            }
            _ => {
                if nilp(&v) {
                    // build a list from exprs
                    let mut acc = val_clone(nil);
                    while let Some(e) = exprs.pop() {
                        acc = Val {
                            t: TspType::TspPair,
                            v: ValUnion::P {
                                car: Box::new(e),
                                cdr: Box::new(acc),
                            },
                        };
                    }
                    acc
                } else {
                    v
                }
            }
        }
    }
    let mut exprs = exprs;
    rec(list, &mut exprs, &nil)
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
    let c0 = bytes[0];
    let c1 = if bytes.len() > 1 { bytes[1] } else { 0 };
    if c0.is_ascii_digit() {
        return true;
    }
    if c0 == b'.' && c1.is_ascii_digit() {
        return true;
    }
    if (c0 == b'-' || c0 == b'+') && (c1.is_ascii_digit() || c1 == b'.') {
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
    let v = Val {
        t: TspType::TspStr,
        v: ValUnion::S(s.to_string()),
    };
    rec_add(&mut st.strs, s, val_clone(&v));
    Some(v)
}

pub fn is_op(c: char) -> bool {
    TSP_OP_CHARS.contains(c)
}

pub fn esc_str(s: &str, len: i32, do_esc: i32) -> String {
    let bytes = s.as_bytes();
    let n = (len.max(0) as usize).min(bytes.len());
    let mut out = String::with_capacity(n);
    let mut i = 0;
    let mut count = 0;
    while count < n {
        let c = bytes[i];
        if c == b'\\' && do_esc != 0 {
            i += 1;
            if i < bytes.len() {
                out.push(esc_char(bytes[i] as char));
            }
        } else {
            out.push(c as char);
        }
        i += 1;
        count += 1;
    }
    out
}

pub fn tib_env_core(st: &mut Tsp) {
    crate::core::tib_env_core(st)
}

pub fn skip_ws(st: &mut Tsp, skipnl: i32) {
    let s: &[u8] = if skipnl != 0 {
        b" \t\n\r"
    } else {
        b" \t"
    };
    let bytes = st.file.as_bytes();
    loop {
        if st.filec >= bytes.len() {
            return;
        }
        let c = bytes[st.filec];
        if c == 0 {
            return;
        }
        if !s.contains(&c) && c != b';' {
            return;
        }
        // skip whitespace
        while st.filec < bytes.len() && s.contains(&bytes[st.filec]) {
            st.filec += 1;
        }
        // skip comments
        while st.filec < bytes.len() && bytes[st.filec] == b';' {
            // skip until newline
            while st.filec < bytes.len() && bytes[st.filec] != b'\n' {
                st.filec += 1;
            }
            if skipnl == 0 {
                // stop at newline (don't consume it)
                break;
            }
            // skip the newline
            if st.filec < bytes.len() {
                st.filec += 1;
            }
        }
    }
}

pub fn rec_extend(rec: &mut Rec, args: Val, vals: Val) -> Rec {
    let argnum = TSP_REC_FACTOR as i32 * tsp_lstlen(&args);
    let cap = if argnum > 0 { argnum } else { -argnum + 1 };
    let mut ret = rec_new(cap as usize, Some(Box::new(rec_clone(rec))));
    let mut a = args;
    let mut v = vals;
    loop {
        if nilp(&a) {
            break;
        }
        let (arg, val, more_a, more_v) = match (a.t, v.t) {
            (TspType::TspPair, TspType::TspPair) => {
                if let (ValUnion::P { car: ac, cdr: ad }, ValUnion::P { car: vc, cdr: vd }) =
                    (a.v, v.v)
                {
                    (*ac, *vc, *ad, *vd)
                } else {
                    return ret;
                }
            }
            _ => (a, v, val_make_nil(), val_make_nil()),
        };
        let key = match &arg.v {
            ValUnion::S(s) => s.clone(),
            _ => return ret,
        };
        rec_add(&mut ret, &key, val);
        if !matches!(more_a.t, TspType::TspPair) {
            break;
        }
        a = more_a;
        v = more_v;
    }
    ret
}

fn val_make_nil() -> Val {
    Val {
        t: TspType::TspNil,
        v: ValUnion::N { num: 0.0, den: 1.0 },
    }
}

pub fn hash(key: &str) -> u32 {
    let mut h: u32 = 0;
    for c in key.bytes() {
        h = h.wrapping_mul(33).wrapping_add(c as u32);
    }
    h
}

pub fn mk_rec(st: &mut Tsp, env: Rec, assoc: Val) -> Option<Val> {
    // If assoc is "missing" (we use nil to mean missing), return rec wrapping env.
    if nilp(&assoc) {
        return Some(Val {
            t: TspType::TspRec,
            v: ValUnion::R(env),
        });
    }
    let cap = TSP_REC_FACTOR as i32 * tsp_lstlen(&assoc);
    let cap = if cap > 0 { cap } else { -cap + 1 };
    let r = rec_new(cap as usize, None);
    let ret = Val {
        t: TspType::TspRec,
        v: ValUnion::R(r),
    };
    let mut outer = rec_new(4, Some(Box::new(env)));
    rec_add(&mut outer, "this", val_clone(&ret));

    let mut ret = ret;
    let mut cur = assoc;
    while matches!(cur.t, TspType::TspPair) {
        let (car_v, cdr_v) = if let ValUnion::P { car, cdr } = cur.v {
            (*car, *cdr)
        } else {
            break;
        };
        match car_v.t {
            TspType::TspPair => {
                if let ValUnion::P { car: caar, cdr: cdar } = car_v.v {
                    if matches!(caar.t, TspType::TspSym | TspType::TspStr) {
                        let key = match &caar.v {
                            ValUnion::S(s) => s.clone(),
                            _ => String::new(),
                        };
                        // cdar should be a pair, take its car
                        if let ValUnion::P { car: c2, .. } = cdar.v {
                            let v = tisp_eval(st, *c2)?;
                            if let ValUnion::R(rr) = &mut ret.v {
                                rec_add(rr, &key, v);
                            }
                        }
                    } else {
                        return None;
                    }
                }
            }
            TspType::TspSym => {
                let key = if let ValUnion::S(ref s) = car_v.v {
                    s.clone()
                } else {
                    String::new()
                };
                let v = tisp_eval(st, car_v)?;
                if let ValUnion::R(rr) = &mut ret.v {
                    rec_add(rr, &key, v);
                }
            }
            _ => return None,
        }
        cur = cdr_v;
    }
    Some(ret)
}

pub fn tisp_read(st: &mut Tsp) -> Option<Val> {
    let mut v = tisp_read_sexpr(st)?;
    loop {
        let bytes = st.file.as_bytes();
        if st.filec >= bytes.len() {
            break;
        }
        let c = bytes[st.filec] as char;
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
        v: ValUnion::N {
            num: i as f64,
            den: 1.0,
        },
    }
}

pub fn tib_env_math(st: &mut Tsp) {
    crate::math::tib_env_math(st)
}

pub fn tisp_eval_list(st: &mut Tsp, env: &mut Rec, v: Val) -> Option<Val> {
    let mut items: Vec<Val> = Vec::new();
    let mut cur = v;
    loop {
        if nilp(&cur) {
            // build nil-terminated list
            let mut acc = val_clone(&st.nil);
            for it in items.into_iter().rev() {
                acc = mk_pair(it, acc)?;
            }
            return Some(acc);
        }
        match cur.t {
            TspType::TspPair => {
                let (car_v, cdr_v) = if let ValUnion::P { car, cdr } = cur.v {
                    (*car, *cdr)
                } else {
                    return None;
                };
                let _ = env; // env parameter unused due to single-env eval
                let ev = tisp_eval(st, car_v)?;
                items.push(ev);
                cur = cdr_v;
            }
            _ => {
                // improper list — eval the tail and append
                let ev = tisp_eval(st, cur)?;
                let mut acc = ev;
                for it in items.into_iter().rev() {
                    acc = mk_pair(it, acc)?;
                }
                return Some(acc);
            }
        }
    }
}

pub fn read_sci(st: &mut Tsp, val: f64, isint: i32) -> Option<Val> {
    let bytes = st.file.as_bytes();
    let mut val = val;
    let mut isint = isint;
    if st.filec < bytes.len() {
        let c = (bytes[st.filec] as char).to_ascii_lowercase();
        if c == 'e' {
            st.filec += 1;
            let sign = if read_sign(st) == 1 { 10.0 } else { 0.1 };
            let mut expo = read_int(st);
            // If expo > 0 and sign is 0.1, it produces fraction. Match C semantics.
            while expo > 0 {
                val *= sign;
                expo -= 1;
            }
            isint = 0;
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
    let mut ret: i32 = 0;
    while st.filec < bytes.len() {
        let c = bytes[st.filec];
        if !c.is_ascii_digit() {
            break;
        }
        ret = ret.wrapping_mul(10).wrapping_add((c - b'0') as i32);
        st.filec += 1;
    }
    ret
}

pub fn rec_new(cap: usize, next: Option<Box<Rec>>) -> Rec {
    let cap = cap.max(1);
    let mut items = Vec::with_capacity(cap);
    for _ in 0..cap {
        items.push(Entry {
            key: String::new(),
            val: val_make_nil(),
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
    // skip starting open quote
    st.filec += 1;
    let bytes = st.file.as_bytes();
    let start = st.filec;
    // determine end char by sentinel: we look at character at position start-1
    let endchar = bytes[start - 1] as char;
    let endchar_byte = endchar as u8;
    let mut len = 0i32;
    while st.filec < bytes.len() && bytes[st.filec] != endchar_byte {
        if bytes[st.filec] == b'\\'
            && st.filec > 0
            && (st.filec == 0 || bytes[st.filec - 1] != b'\\')
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
    st.filec += 1; // skip closing quote
    let do_esc = if endchar == '"' { 1 } else { 0 };
    let raw = &st.file[start..start + len as usize].to_string();
    let unescaped = esc_str(raw, len, do_esc);
    Some(mk_fn(st, &unescaped))
}

pub fn read_sym(st: &mut Tsp, is_char: fn(char) -> bool) -> Option<Val> {
    let bytes = st.file.as_bytes();
    let start = st.filec;
    let mut len = 0i32;
    while st.filec < bytes.len() {
        let c = bytes[st.filec] as char;
        if c == '\0' {
            break;
        }
        if !is_char(c) {
            break;
        }
        st.filec += 1;
        len += 1;
    }
    let raw = st.file[start..start + len as usize].to_string();
    let s = esc_str(&raw, len, 0);
    mk_sym(st, &s)
}

pub fn mk_dec(d: f64) -> Option<Val> {
    Some(Val {
        t: TspType::TspDec,
        v: ValUnion::N { num: d, den: 1.0 },
    })
}

pub fn tisp_eval_body(st: &mut Tsp, env: &mut Rec, v: Val) -> Option<Val> {
    let mut ret = val_clone(&st.none);
    let mut cur = v;
    loop {
        match cur.t {
            TspType::TspPair => {
                let (car_v, cdr_v) = if let ValUnion::P { car, cdr } = cur.v {
                    (*car, *cdr)
                } else {
                    break;
                };
                ret = tisp_eval(st, car_v)?;
                cur = cdr_v;
            }
            _ => break,
        }
        let _ = env; // env parameter is mostly unused due to global env
    }
    Some(ret)
}

pub fn tib_env_io(st: &mut Tsp) {
    crate::io::tib_env_io(st)
}

pub fn tisp_read_sugar(st: &mut Tsp, v: Val) -> Option<Val> {
    let bytes = st.file.as_bytes();
    if st.filec >= bytes.len() {
        return Some(v);
    }
    let c = bytes[st.filec] as char;
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
        let next = if st.filec < bytes.len() {
            bytes[st.filec] as char
        } else {
            '\0'
        };
        match next {
            '(' => {
                st.filec += 1;
                let w = read_pair(st, ')')?;
                let mapsym = mk_sym(st, "map")?;
                let inner = mk_pair(v, w)?;
                return mk_pair(mapsym, inner);
            }
            ':' => {
                st.filec += 1;
                let w = read_sym(st, is_sym)?;
                let qsym = mk_sym(st, "quote")?;
                let qw = mk_list(st, 2, vec![qsym, w])?;
                return mk_list(st, 2, vec![v, qw]);
            }
            _ => {
                skip_ws(st, 1);
                let w = tisp_read(st)?;
                return mk_list(st, 2, vec![v, w]);
            }
        }
    }
    if c == '>' {
        let next = if st.filec + 1 < bytes.len() {
            bytes[st.filec + 1] as char
        } else {
            '\0'
        };
        if next == '>' {
            st.filec += 2;
            let w = tisp_read(st)?;
            if !matches!(w.t, TspType::TspPair) {
                return None;
            }
            if let ValUnion::P { car, cdr } = w.v {
                let inner = mk_pair(v, *cdr)?;
                return mk_pair(*car, inner);
            }
        }
    }
    Some(v)
}

pub fn tisp_env_lib(st: &mut Tsp, lib: &str) {
    let file = std::mem::take(&mut st.file);
    let filec = st.filec;
    st.file = lib.to_string();
    st.filec = 0;
    skip_ws(st, 1);
    if let Some(v) = tisp_read(st) {
        let mut env = rec_clone(&st.env);
        let _ = tisp_eval_body(st, &mut env, v);
    }
    st.file = file;
    st.filec = filec;
}

pub fn mk_list(st: &mut Tsp, n: i32, args: Vec<Val>) -> Option<Val> {
    let mut acc = val_clone(&st.nil);
    let take = if n < 0 { 0 } else { n as usize };
    let mut items: Vec<Val> = args.into_iter().take(take).collect();
    while let Some(v) = items.pop() {
        acc = mk_pair(v, acc)?;
    }
    Some(acc)
}

pub fn vals_eq(a: &Val, b: &Val) -> bool {
    let a_num = type_matches(a.t, TSP_NUM);
    let b_num = type_matches(b.t, TSP_NUM);
    if a_num && b_num {
        let (an, ad) = val_num(a);
        let (bn, bd) = val_num(b);
        return an == bn && ad == bd;
    }
    if type_bits(a.t) != type_bits(b.t) {
        return false;
    }
    match (&a.v, &b.v) {
        (ValUnion::P { car: ac, cdr: ad }, ValUnion::P { car: bc, cdr: bd }) => {
            vals_eq(ac, bc) && vals_eq(ad, bd)
        }
        (
            ValUnion::F {
                args: a_args,
                body: a_body,
                ..
            },
            ValUnion::F {
                args: b_args,
                body: b_body,
                ..
            },
        ) => vals_eq(a_args, b_args) && vals_eq(a_body, b_body),
        (ValUnion::S(s1), ValUnion::S(s2)) => s1 == s2,
        (ValUnion::N { num: n1, den: d1 }, ValUnion::N { num: n2, den: d2 }) => {
            n1 == n2 && d1 == d2
        }
        (ValUnion::Pr { name: n1, .. }, ValUnion::Pr { name: n2, .. }) => n1 == n2,
        (ValUnion::R(_), ValUnion::R(_)) => false,
        _ => matches!(a.t, TspType::TspNil | TspType::TspNone),
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
    if st.filec >= bytes.len() {
        return 1;
    }
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
}

pub fn tisp_print(f: &mut std::fs::File, v: &Val) {
    use std::io::Write;
    let s = print_to_string(v);
    let _ = f.write_all(s.as_bytes());
}

fn print_to_string(v: &Val) -> String {
    match v.t {
        TspType::TspNone => "Void".to_string(),
        TspType::TspNil => "Nil".to_string(),
        TspType::TspInt => {
            let (n, _) = val_num(v);
            format!("{}", n as i32)
        }
        TspType::TspDec => {
            let (n, _) = val_num(v);
            let s = format!("{:.15e}", n);
            // approximate "%.15g" — fallback to default formatting
            let mut out = format!("{}", n);
            if n == (n as i32) as f64 {
                out.push_str(".0");
            }
            // suppress unused warn
            let _ = s;
            out
        }
        TspType::TspRatio => {
            let (n, d) = val_num(v);
            format!("{}/{}", n as i32, d as i32)
        }
        TspType::TspStr | TspType::TspSym => match &v.v {
            ValUnion::S(s) => s.clone(),
            _ => String::new(),
        },
        TspType::TspFunc | TspType::TspMacro => {
            let kind = if matches!(v.t, TspType::TspFunc) {
                "function"
            } else {
                "macro"
            };
            if let ValUnion::F { name, .. } = &v.v {
                if name.is_empty() {
                    format!("#<{}>", kind)
                } else {
                    format!("#<{}:{}>", kind, name)
                }
            } else {
                format!("#<{}>", kind)
            }
        }
        TspType::TspPrim => match &v.v {
            ValUnion::Pr { name, .. } => format!("#<primitive:{}>", name),
            _ => "#<primitive>".to_string(),
        },
        TspType::TspForm => match &v.v {
            ValUnion::Pr { name, .. } => format!("#<form:{}>", name),
            _ => "#<form>".to_string(),
        },
        TspType::TspRec => {
            let mut s = String::from("{");
            if let ValUnion::R(r) = &v.v {
                let mut cur = Some(r);
                while let Some(rec) = cur {
                    let mut c = 0;
                    for it in rec.items.iter() {
                        if !it.key.is_empty() {
                            c += 1;
                            s.push_str(&format!(" {}: {}", it.key, print_to_string(&it.val)));
                            if c == TSP_REC_MAX_PRINT {
                                s.push_str(" ...");
                                break;
                            }
                        }
                    }
                    cur = rec.next.as_deref();
                }
            }
            s.push_str(" }");
            s
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
    }
}

pub fn eval_proc(st: &mut Tsp, env: &mut Rec, f: Val, args: Val) -> Option<Val> {
    match f.t {
        TspType::TspPrim => {
            let evaled = tisp_eval_list(st, env, args)?;
            if let ValUnion::Pr { name, .. } = f.v {
                let r = dispatch_named(st, env, &name, evaled)?;
                return Some(r);
            }
            None
        }
        TspType::TspForm => {
            if let ValUnion::Pr { name, .. } = f.v {
                let r = dispatch_named(st, env, &name, args)?;
                return Some(r);
            }
            None
        }
        TspType::TspFunc => {
            let evaled = tisp_eval_list(st, env, args)?;
            if let ValUnion::F {
                args: f_args,
                body,
                env: f_env,
                ..
            } = f.v
            {
                let mut new_env = rec_extend(&mut rec_clone(&f_env), *f_args, evaled);
                return tisp_eval_body(st, &mut new_env, *body);
            }
            None
        }
        TspType::TspMacro => {
            if let ValUnion::F {
                args: f_args,
                body,
                env: f_env,
                ..
            } = f.v
            {
                let mut new_env = rec_extend(&mut rec_clone(&f_env), *f_args, args);
                let r = tisp_eval_body(st, &mut new_env, *body)?;
                return tisp_eval(st, r);
            }
            None
        }
        TspType::TspRec => {
            let evaled = tisp_eval_list(st, env, args)?;
            // first arg should be sym
            if let ValUnion::P { car, .. } = &evaled.v {
                if let ValUnion::S(key) = &car.v {
                    if let ValUnion::R(r) = &f.v {
                        if let Some(v) = rec_get(r, key) {
                            return Some(v);
                        }
                        if let Some(v) = rec_get(r, "else") {
                            return Some(v);
                        }
                    }
                }
            }
            None
        }
        _ => None,
    }
}

fn dispatch_named(st: &mut Tsp, env: &mut Rec, name: &str, args: Val) -> Option<Val> {
    use crate::core::*;
    use crate::io::*;
    use crate::math::*;
    use crate::os::*;
    use crate::string::*;
    let r = match name {
        "car" => prim_car(st, env, args),
        "cdr" => prim_cdr(st, env, args),
        "cons" => prim_cons(st, env, args),
        "quote" => form_quote(st, env, args),
        "eval" => prim_eval(st, env, args),
        "=" => prim_eq(st, env, args),
        "cond" => form_cond(st, env, args),
        "do" => return tisp_eval_body(st, env, args),
        "typeof" => prim_typeof(st, env, args),
        "procprops" => prim_procprops(st, env, args),
        "Func" => form_Func(st, env, args),
        "Macro" => form_Macro(st, env, args),
        "error" => prim_error(st, env, args),
        "Rec" => match mk_rec(st, rec_clone(env), args) {
            Some(v) => v,
            None => return None,
        },
        "recmerge" => prim_recmerge(st, env, args),
        "records" => prim_records(st, env, args),
        "def" => form_def(st, env, args),
        "undefine!" => form_undefine(st, env, args),
        "defined?" => form_definedp(st, env, args),

        // string
        "Str" => prim_Str(st, env, args),
        "Sym" => prim_Sym(st, env, args),
        "strlen" => prim_strlen(st, env, args),
        "strformat" => form_strformat(st, env, args),

        // math
        "+" => prim_add(st, env, args),
        "-" => prim_sub(st, env, args),
        "*" => prim_mul(st, env, args),
        "/" => prim_div(st, env, args),
        "mod" => prim_mod(st, env, args),
        "^" => prim_pow(st, env, args),
        "Int" => prim_int(st, env, args),
        "Dec" => prim_dec(st, env, args),
        "round" => prim_round(st, env, args),
        "floor" => prim_floor(st, env, args),
        "ceil" => prim_ceil(st, env, args),
        "numerator" => prim_numerator(st, env, args),
        "denominator" => prim_denominator(st, env, args),
        "<" => prim_lt(st, env, args),
        ">" => prim_gt(st, env, args),
        "<=" => prim_lte(st, env, args),
        ">=" => prim_gte(st, env, args),
        "sin" => prim_sin(st, env, args),
        "cos" => prim_cos(st, env, args),
        "tan" => prim_tan(st, env, args),
        "sinh" => prim_sinh(st, env, args),
        "cosh" => prim_cosh(st, env, args),
        "tanh" => prim_tanh(st, env, args),
        "arcsin" => prim_asin(st, env, args),
        "arccos" => prim_acos(st, env, args),
        "arctan" => prim_atan(st, env, args),
        "arcsinh" => prim_asinh(st, env, args),
        "arccosh" => prim_acosh(st, env, args),
        "arctanh" => prim_atanh(st, env, args),
        "exp" => prim_exp(st, env, args),
        "log" => prim_log(st, env, args),

        // io
        "write" => prim_write(st, env, args),
        "read" => prim_read(st, env, args),
        "parse" => prim_parse(st, env, args),
        "load" => prim_load(st, env, args),

        // os
        "cd!" => prim_cd(st, env, args),
        "pwd" => prim_pwd(st, env, args),
        "exit!" => prim_exit(st, env, args),
        "now" => prim_now(st, env, args),
        "time" => form_time(st, env, args),

        _ => return None,
    };
    Some(r)
}

#[allow(dead_code)]
fn clone_tsp(st: &Tsp) -> Tsp {
    Tsp {
        file: st.file.clone(),
        filec: st.filec,
        none: val_clone(&st.none),
        nil: val_clone(&st.nil),
        t: val_clone(&st.t),
        env: rec_clone(&st.env),
        strs: rec_clone(&st.strs),
        syms: rec_clone(&st.syms),
        libh: Vec::new(),
        libhc: st.libhc,
    }
}

pub fn tisp_eval(st: &mut Tsp, v: Val) -> Option<Val> {
    match v.t {
        TspType::TspSym => {
            let key = if let ValUnion::S(ref s) = v.v {
                s.clone()
            } else {
                return None;
            };
            rec_get(&st.env, &key)
        }
        TspType::TspPair => {
            let (car_v, cdr_v) = if let ValUnion::P { car, cdr } = v.v {
                (*car, *cdr)
            } else {
                return None;
            };
            let f = tisp_eval(st, car_v)?;
            let mut env = rec_clone(&st.env);
            eval_proc(st, &mut env, f, cdr_v)
        }
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
    let ocap = rec.cap as usize;
    let new_cap = ocap.saturating_mul(TSP_REC_FACTOR).max(1);
    let mut new_items: Vec<Entry> = Vec::with_capacity(new_cap);
    for _ in 0..new_cap {
        new_items.push(Entry {
            key: String::new(),
            val: val_make_nil(),
        });
    }
    let old_items = std::mem::replace(&mut rec.items, new_items);
    rec.cap = new_cap as i32;
    rec.size = 0;
    for e in old_items {
        if !e.key.is_empty() {
            rec_add(rec, &e.key.clone(), e.val);
        }
    }
}
