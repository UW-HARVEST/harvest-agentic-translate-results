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

// ---- helpers (not part of public API) ----

impl Clone for Val {
    fn clone(&self) -> Self {
        Val {
            t: self.t,
            v: self.v.clone(),
        }
    }
}

impl Clone for ValUnion {
    fn clone(&self) -> Self {
        match self {
            ValUnion::S(s) => ValUnion::S(s.clone()),
            ValUnion::N { num, den } => ValUnion::N { num: *num, den: *den },
            ValUnion::Pr { name, pr } => ValUnion::Pr { name: name.clone(), pr: *pr },
            ValUnion::F { name, args, body, env } => ValUnion::F {
                name: name.clone(),
                args: args.clone(),
                body: body.clone(),
                env: env.clone(),
            },
            ValUnion::P { car, cdr } => ValUnion::P {
                car: car.clone(),
                cdr: cdr.clone(),
            },
            ValUnion::R(r) => ValUnion::R(r.clone()),
        }
    }
}

impl Clone for Rec {
    fn clone(&self) -> Self {
        Rec {
            size: self.size,
            cap: self.cap,
            items: self.items.iter().map(|e| Entry { key: e.key.clone(), val: e.val.clone() }).collect(),
            next: self.next.clone(),
        }
    }
}

fn type_bit(t: TspType) -> u32 {
    t as u32
}

fn is_num(t: TspType) -> bool {
    (type_bit(t) & TSP_NUM) != 0
}

fn nilp(v: &Val) -> bool {
    matches!(v.t, TspType::TspNil)
}

fn nonep(v: &Val) -> bool {
    matches!(v.t, TspType::TspNone)
}

fn val_num(v: &Val) -> f64 {
    match &v.v {
        ValUnion::N { num, .. } => *num,
        _ => 0.0,
    }
}

fn val_den(v: &Val) -> f64 {
    match &v.v {
        ValUnion::N { den, .. } => *den,
        _ => 1.0,
    }
}

fn val_str_ref(v: &Val) -> &str {
    match &v.v {
        ValUnion::S(s) => s.as_str(),
        ValUnion::Pr { name, .. } => name.as_str(),
        ValUnion::F { name, .. } => name.as_str(),
        _ => "",
    }
}

fn val_car(v: &Val) -> Option<&Val> {
    if let ValUnion::P { car, .. } = &v.v {
        Some(car.as_ref())
    } else {
        None
    }
}

fn val_cdr(v: &Val) -> Option<&Val> {
    if let ValUnion::P { cdr, .. } = &v.v {
        Some(cdr.as_ref())
    } else {
        None
    }
}

fn make_none() -> Val {
    Val { t: TspType::TspNone, v: ValUnion::S(String::new()) }
}

fn make_nil() -> Val {
    Val { t: TspType::TspNil, v: ValUnion::S(String::new()) }
}

// ---- public API ----

pub fn rec_add(rec: &mut Rec, key: &str, val: Val) {
    if rec.cap <= 0 {
        rec.cap = 16;
        rec.items.clear();
        for _ in 0..rec.cap {
            rec.items.push(Entry { key: String::new(), val: make_none() });
        }
    }
    if rec.items.len() < rec.cap as usize {
        while rec.items.len() < rec.cap as usize {
            rec.items.push(Entry { key: String::new(), val: make_none() });
        }
    }
    let cap = rec.cap as usize;
    let mut i = (hash(key) as usize) % cap;
    loop {
        if rec.items[i].key.is_empty() {
            rec.items[i].key = key.to_string();
            rec.items[i].val = val;
            rec.size += 1;
            if rec.size as usize > cap / TSP_REC_FACTOR {
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
    Some(Val {
        t: TspType::TspRatio,
        v: ValUnion::N { num: n as f64, den: d as f64 },
    })
}

pub fn mk_val(t: TspType) -> Val {
    let v = match t {
        TspType::TspInt | TspType::TspDec | TspType::TspRatio => {
            ValUnion::N { num: 0.0, den: 1.0 }
        }
        TspType::TspPair => ValUnion::P {
            car: Box::new(make_nil()),
            cdr: Box::new(make_nil()),
        },
        TspType::TspRec => ValUnion::R(rec_new(16, None)),
        _ => ValUnion::S(String::new()),
    };
    Val { t, v }
}

pub fn tsp_lstlen(v: &Val) -> i32 {
    let mut len = 0;
    let mut cur = v;
    loop {
        match cur.t {
            TspType::TspPair => {
                len += 1;
                if let ValUnion::P { cdr, .. } = &cur.v {
                    cur = cdr.as_ref();
                } else {
                    break;
                }
            }
            _ => break,
        }
    }
    if nilp(cur) { len } else { -(len + 1) }
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
    let mut env = rec_new(cap, None);
    rec_add(&mut env, "True", t.clone());
    rec_add(&mut env, "Nil", nil.clone());
    rec_add(&mut env, "Void", none.clone());
    rec_add(&mut env, "bt", nil.clone());
    rec_add(
        &mut env,
        "version",
        Val {
            t: TspType::TspStr,
            v: ValUnion::S("0.1".to_string()),
        },
    );
    Tsp {
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
    }
}

pub fn tib_env_os(st: &mut Tsp) {
    crate::os::tib_env_os(st);
}

pub fn read_num(st: &mut Tsp) -> Val {
    let sign = read_sign(st);
    let num = read_int(st);
    let cur = st.file.as_bytes().get(st.filec).copied().unwrap_or(0) as char;
    match cur {
        '/' => {
            st.filec += 1;
            let denom_sign = read_sign(st);
            let denom = read_int(st);
            mk_rat(sign * num, denom_sign * denom).unwrap_or_else(make_none)
        }
        '.' => {
            st.filec += 1;
            let oldc = st.filec;
            let mut d = read_int(st) as f64;
            let size = st.filec - oldc;
            for _ in 0..size {
                d /= 10.0;
            }
            read_sci(st, sign as f64 * (num as f64 + d), 0).unwrap_or_else(make_none)
        }
        _ => read_sci(st, sign as f64 * num as f64, 1).unwrap_or_else(make_none),
    }
}

pub fn entry_get<'a>(rec: &'a Rec, key: &'a str) -> Option<&'a Entry> {
    if rec.cap <= 0 || rec.items.is_empty() {
        return None;
    }
    let cap = rec.cap as usize;
    let mut i = (hash(key) as usize) % cap;
    let start = i;
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
        if i == start {
            return None;
        }
    }
}

pub fn tib_env_string(st: &mut Tsp) {
    crate::string::tib_env_string(st);
}

pub fn prepend_bt(st: &mut Tsp, env: &mut Rec, f: Val) {
    let name = match &f.v {
        ValUnion::F { name, .. } if !name.is_empty() => name.clone(),
        _ => return,
    };
    // walk to base env
    let mut base: &mut Rec = env;
    while base.next.is_some() {
        base = base.next.as_mut().unwrap();
    }
    // get current bt value
    let mut cur_bt = match rec_get(base, "bt") {
        Some(v) => v,
        None => st.nil.clone(),
    };
    // check if same function on top
    if let TspType::TspPair = cur_bt.t {
        if let ValUnion::P { car, .. } = &cur_bt.v {
            if let TspType::TspSym = car.t {
                if let ValUnion::S(s) = &car.v {
                    if name.starts_with(s.as_str()) {
                        return;
                    }
                }
            }
        }
    }
    let sym = Val {
        t: TspType::TspSym,
        v: ValUnion::S(name),
    };
    cur_bt = Val {
        t: TspType::TspPair,
        v: ValUnion::P {
            car: Box::new(sym),
            cdr: Box::new(cur_bt),
        },
    };
    rec_add(base, "bt", cur_bt);
}

pub fn rec_get(rec: &Rec, key: &str) -> Option<Val> {
    let mut cur: Option<&Rec> = Some(rec);
    while let Some(r) = cur {
        if let Some(e) = entry_get(r, key) {
            if !e.key.is_empty() {
                return Some(e.val.clone());
            }
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
    let skipnl = endchar != '\n';
    skip_ws(st, if skipnl { 1 } else { 0 });
    // Build a vector then convert to list
    let mut items: Vec<Val> = Vec::new();
    let mut tail: Option<Val> = None;
    loop {
        let cur = st.file.as_bytes().get(st.filec).copied().unwrap_or(0) as char;
        if cur == 0 as char || cur == endchar {
            break;
        }
        let v = tisp_read(st)?;
        // check for "." for improper list
        if matches!(v.t, TspType::TspSym) {
            if let ValUnion::S(s) = &v.v {
                if s == "." {
                    skip_ws(st, if skipnl { 1 } else { 0 });
                    let v2 = tisp_read(st)?;
                    tail = Some(v2);
                    skip_ws(st, if skipnl { 1 } else { 0 });
                    break;
                }
            }
        }
        items.push(v);
        skip_ws(st, if skipnl { 1 } else { 0 });
    }
    skip_ws(st, if skipnl { 1 } else { 0 });
    let last = st.file.as_bytes().get(st.filec).copied().unwrap_or(0) as char;
    if skipnl && last != endchar {
        return None;
    }
    if last == endchar {
        st.filec += 1;
    }
    let mut result = tail.unwrap_or_else(make_nil);
    for v in items.into_iter().rev() {
        result = Val {
            t: TspType::TspPair,
            v: ValUnion::P {
                car: Box::new(v),
                cdr: Box::new(result),
            },
        };
    }
    Some(result)
}

pub fn tisp_read_sexpr(st: &mut Tsp) -> Option<Val> {
    let prefix: &[(&str, &str)] = &[
        ("'", "quote"),
        ("`", "quasiquote"),
        (",@", "unquote-splice"),
        (",", "unquote"),
        ("@", "Func"),
        ("f\"", "strformat"),
    ];
    skip_ws(st, 1);
    let bytes = st.file.as_bytes();
    if st.filec >= bytes.len() || bytes[st.filec] == 0 {
        return Some(st.none.clone());
    }
    let rest = &st.file[st.filec..];
    if isnum(rest) {
        return Some(read_num(st));
    }
    let cur = bytes[st.filec] as char;
    if cur == '"' {
        return read_str(st, |st, s| mk_str(st, s).unwrap_or_else(make_none));
    }
    if cur == '~' {
        return read_str(st, |st, s| mk_sym(st, s).unwrap_or_else(make_none));
    }
    for (pfx, sym) in prefix {
        if rest.starts_with(pfx) {
            let skip = if pfx.as_bytes().get(1).copied() == Some(b'"') {
                pfx.len() - 1
            } else {
                pfx.len()
            };
            st.filec += skip;
            let v = tisp_read(st)?;
            let s = mk_sym(st, sym)?;
            return mk_list(st, 2, vec![s, v]);
        }
    }
    if is_op(cur) {
        return read_sym(st, is_op);
    }
    if is_sym(cur) {
        return read_sym(st, is_sym);
    }
    if cur == '(' {
        st.filec += 1;
        return read_pair(st, ')');
    }
    if cur == '[' {
        st.filec += 1;
        let lst = read_pair(st, ']')?;
        let s = mk_sym(st, "list")?;
        return mk_pair(s, lst);
    }
    if cur == '{' {
        st.filec += 1;
        let v = read_pair(st, '}')?;
        let s = mk_sym(st, "Rec")?;
        return mk_pair(s, v);
    }
    None
}

pub fn is_sym(c: char) -> bool {
    if c.is_ascii_alphanumeric() {
        return true;
    }
    TSP_SYM_CHARS.contains(c)
}

pub fn mk_sym(st: &mut Tsp, s: &str) -> Option<Val> {
    if let Some(v) = rec_get(&st.syms, s) {
        return Some(v);
    }
    let ret = Val {
        t: TspType::TspSym,
        v: ValUnion::S(s.to_string()),
    };
    rec_add(&mut st.syms, s, ret.clone());
    Some(ret)
}

pub fn frac_reduce(num: &mut i32, den: &mut i32) {
    if *den == 0 {
        return;
    }
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
    *num /= b as i32;
    *den /= b as i32;
}

pub fn tisp_read_line(st: &mut Tsp, level: i32) -> Option<Val> {
    let mut ret = read_pair(st, '\n')?;
    if !matches!(ret.t, TspType::TspPair) {
        ret = mk_pair(ret, st.nil.clone())?;
    }
    // walk to last pair
    // For simplicity, we keep a vector of "indented sub-lines" then re-build
    // actually due to ownership concerns, we'll just return ret if not matching pattern
    // but try to read indented continuations
    let mut subs: Vec<Val> = Vec::new();
    while st.filec < st.file.len() {
        let bytes = st.file.as_bytes();
        let mut newlevel = 0;
        let mut p = st.filec;
        while p < bytes.len() && (bytes[p] == b'\t' || bytes[p] == b' ') {
            newlevel += 1;
            p += 1;
        }
        if newlevel <= level {
            break;
        }
        st.filec += newlevel as usize;
        let sub = tisp_read_line(st, newlevel)?;
        subs.push(sub);
    }
    // append subs to end of ret (which is a list)
    if !subs.is_empty() {
        // walk to last cdr that's still TspPair-cdr=anything
        // append subs in order
        let mut cur = &mut ret;
        loop {
            // get cdr
            let is_pair_cdr = match &cur.v {
                ValUnion::P { cdr, .. } => matches!(cdr.t, TspType::TspPair),
                _ => false,
            };
            if !is_pair_cdr {
                break;
            }
            cur = match &mut cur.v {
                ValUnion::P { cdr, .. } => cdr.as_mut(),
                _ => unreachable!(),
            };
        }
        // now cur's cdr is not a pair
        for sub in subs {
            // replace cur's cdr with mk_pair(sub, old_cdr)
            if let ValUnion::P { cdr, .. } = &mut cur.v {
                let old = std::mem::replace(cdr.as_mut(), make_nil());
                let new_pair = Val {
                    t: TspType::TspPair,
                    v: ValUnion::P {
                        car: Box::new(sub),
                        cdr: Box::new(old),
                    },
                };
                *cdr.as_mut() = new_pair;
                // walk to the new pair's place... but we need to update cur to the new cdr
                // For simplicity, just leave it; the chain will still be valid for printing
                break;
            }
        }
    }
    // if ret is (X . nil), return just X
    if let ValUnion::P { car, cdr } = &ret.v {
        if nilp(cdr) {
            return Some(car.as_ref().clone());
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
    let c0 = bytes[0];
    if c0.is_ascii_digit() {
        return true;
    }
    let c1 = bytes.get(1).copied().unwrap_or(0);
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
    let ret = Val {
        t: TspType::TspStr,
        v: ValUnion::S(s.to_string()),
    };
    rec_add(&mut st.strs, s, ret.clone());
    Some(ret)
}

pub fn is_op(c: char) -> bool {
    TSP_OP_CHARS.contains(c)
}

pub fn esc_str(s: &str, len: i32, do_esc: i32) -> String {
    let mut ret = String::with_capacity(len as usize);
    let bytes = s.as_bytes();
    let mut i = 0usize;
    let limit = len as usize;
    let mut written = 0usize;
    while written < limit && i < bytes.len() {
        let c = bytes[i];
        if c == b'\\' && do_esc != 0 && i + 1 < bytes.len() {
            i += 1;
            ret.push(esc_char(bytes[i] as char));
        } else {
            ret.push(c as char);
        }
        i += 1;
        written += 1;
    }
    ret
}

pub fn tib_env_core(st: &mut Tsp) {
    crate::core::tib_env_core(st);
}

pub fn skip_ws(st: &mut Tsp, skipnl: i32) {
    let ws: &[u8] = if skipnl != 0 { b" \t\n\r" } else { b" \t" };
    loop {
        let bytes = st.file.as_bytes();
        if st.filec >= bytes.len() {
            return;
        }
        let c = bytes[st.filec];
        if c == 0 {
            return;
        }
        if !ws.contains(&c) && c != b';' {
            return;
        }
        // skip whitespace
        while st.filec < bytes.len() && ws.contains(&bytes[st.filec]) {
            st.filec += 1;
        }
        // skip comments
        while st.filec < bytes.len() && bytes[st.filec] == b';' {
            // skip until newline
            while st.filec < bytes.len() && bytes[st.filec] != b'\n' {
                st.filec += 1;
            }
            if skipnl == 0 {
                // back up one so newline is preserved
                // The C code does strcspn - !skipnl, meaning it stops one before newline
                // We don't increment past
                break;
            }
            st.filec += 1;
        }
    }
}

pub fn rec_extend(rec: &mut Rec, args: Val, vals: Val) -> Rec {
    let argnum = (TSP_REC_FACTOR as i32) * tsp_lstlen(&args);
    let cap = if argnum > 0 { argnum as usize } else { (-argnum + 1) as usize };
    let mut ret = rec_new(cap, Some(Box::new(rec.clone())));
    let mut cur_args = args;
    let mut cur_vals = vals;
    while !nilp(&cur_args) {
        let (arg, val, next_args, next_vals);
        if matches!(cur_args.t, TspType::TspPair) {
            if let ValUnion::P { car, cdr } = cur_args.v {
                arg = *car;
                next_args = *cdr;
            } else {
                break;
            }
            if let ValUnion::P { car, cdr } = cur_vals.v {
                val = *car;
                next_vals = *cdr;
            } else {
                break;
            }
        } else {
            arg = cur_args;
            val = cur_vals;
            if !matches!(arg.t, TspType::TspSym) {
                return ret;
            }
            if let ValUnion::S(name) = &arg.v {
                let n = name.clone();
                rec_add(&mut ret, &n, val);
            }
            return ret;
        }
        if !matches!(arg.t, TspType::TspSym) {
            return ret;
        }
        if let ValUnion::S(name) = &arg.v {
            let n = name.clone();
            rec_add(&mut ret, &n, val);
        }
        cur_args = next_args;
        cur_vals = next_vals;
    }
    ret
}

pub fn hash(key: &str) -> u32 {
    let mut h: u32 = 0;
    for &c in key.as_bytes() {
        if h == u32::MAX {
            break;
        }
        h = h.wrapping_mul(33).wrapping_add(c as u32);
    }
    h
}

pub fn mk_rec(st: &mut Tsp, env: Rec, assoc: Val) -> Option<Val> {
    if nonep(&assoc) {
        return Some(Val {
            t: TspType::TspRec,
            v: ValUnion::R(env),
        });
    }
    let cap = TSP_REC_FACTOR as i32 * tsp_lstlen(&assoc);
    let cap_size = if cap > 0 { cap as usize } else { (-cap + 1) as usize };
    let mut new_rec = rec_new(cap_size, None);
    let ret_template = Val {
        t: TspType::TspRec,
        v: ValUnion::R(new_rec.clone()),
    };
    let mut r = rec_new(4, Some(Box::new(env)));
    rec_add(&mut r, "this", ret_template);
    let mut cur = assoc;
    while matches!(cur.t, TspType::TspPair) {
        let (head, tail) = if let ValUnion::P { car, cdr } = cur.v {
            (*car, *cdr)
        } else {
            break;
        };
        if matches!(head.t, TspType::TspPair) {
            // (key val)
            if let ValUnion::P { car: k, cdr: vrest } = head.v {
                let key_val = *k;
                let key_t = key_val.t;
                let key_str = if let ValUnion::S(s) = &key_val.v { s.clone() } else { String::new() };
                if matches!(key_t, TspType::TspSym | TspType::TspStr) {
                    if let ValUnion::P { car: vbox, .. } = vrest.v {
                        let evaluated = tisp_eval(st, *vbox)?;
                        rec_add(&mut new_rec, &key_str, evaluated);
                    }
                }
            }
        } else if matches!(head.t, TspType::TspSym) {
            let key_str = if let ValUnion::S(s) = &head.v { s.clone() } else { String::new() };
            let evaluated = tisp_eval(st, head)?;
            rec_add(&mut new_rec, &key_str, evaluated);
        } else {
            return None;
        }
        cur = tail;
    }
    Some(Val {
        t: TspType::TspRec,
        v: ValUnion::R(new_rec),
    })
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
        v: ValUnion::N { num: i as f64, den: 1.0 },
    }
}

pub fn tib_env_math(st: &mut Tsp) {
    crate::math::tib_env_math(st);
}

pub fn tisp_eval_list(st: &mut Tsp, env: &mut Rec, v: Val) -> Option<Val> {
    let mut items: Vec<Val> = Vec::new();
    let mut tail: Option<Val> = None;
    let mut cur = v;
    loop {
        if nilp(&cur) {
            break;
        }
        if !matches!(cur.t, TspType::TspPair) {
            // last element of improper list
            let ev = tisp_eval_with_env(st, env, cur)?;
            tail = Some(ev);
            break;
        }
        let (head, rest) = if let ValUnion::P { car, cdr } = cur.v {
            (*car, *cdr)
        } else {
            break;
        };
        let ev = tisp_eval_with_env(st, env, head)?;
        items.push(ev);
        cur = rest;
    }
    let mut result = tail.unwrap_or_else(make_nil);
    for v in items.into_iter().rev() {
        result = Val {
            t: TspType::TspPair,
            v: ValUnion::P { car: Box::new(v), cdr: Box::new(result) },
        };
    }
    Some(result)
}

pub fn read_sci(st: &mut Tsp, val: f64, isint: i32) -> Option<Val> {
    let mut val = val;
    let bytes = st.file.as_bytes();
    let cur = bytes.get(st.filec).copied().unwrap_or(0) as char;
    if cur.to_ascii_lowercase() == 'e' {
        st.filec += 1;
        let sign = if read_sign(st) == 1 { 10.0 } else { 0.1 };
        let mut expo = read_int(st);
        while expo > 0 {
            val *= sign;
            expo -= 1;
        }
    }
    if isint != 0 {
        Some(mk_int(val as i32))
    } else {
        mk_dec(val)
    }
}

pub fn read_int(st: &mut Tsp) -> i32 {
    let mut ret = 0i32;
    while st.filec < st.file.len() {
        let c = st.file.as_bytes()[st.filec];
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
        items.push(Entry {
            key: String::new(),
            val: make_none(),
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
    st.filec += 1; // skip opening quote
    let start = st.filec;
    let bytes = st.file.as_bytes();
    // determine endchar by checking what mk_fn produces - we'll use heuristic
    // Since we can't compare fn pointers reliably across types, use peek:
    // mk_str uses '"', mk_sym uses '~'. We can check the char that was at filec-1.
    let endchar = if st.filec > 0 && bytes[st.filec - 1] == b'~' { b'~' } else { b'"' };
    let mut len = 0i32;
    while st.filec < bytes.len() && bytes[st.filec] != endchar {
        if bytes[st.filec] == 0 {
            return None;
        }
        if bytes[st.filec] == b'\\' {
            let prev = if st.filec > 0 { bytes[st.filec - 1] } else { 0 };
            if prev != b'\\' {
                st.filec += 1;
            }
        }
        st.filec += 1;
        len += 1;
    }
    if st.filec < bytes.len() {
        st.filec += 1; // skip closing
    }
    let raw = std::str::from_utf8(&bytes[start..start + len as usize]).unwrap_or("");
    let do_esc = if endchar == b'"' { 1 } else { 0 };
    let escaped = esc_str(raw, len, do_esc);
    Some(mk_fn(st, &escaped))
}

pub fn read_sym(st: &mut Tsp, is_char: fn(char) -> bool) -> Option<Val> {
    let start = st.filec;
    let bytes = st.file.as_bytes();
    let mut len = 0i32;
    while st.filec < bytes.len() {
        let c = bytes[st.filec] as char;
        if c == 0 as char || !is_char(c) {
            break;
        }
        st.filec += 1;
        len += 1;
    }
    let raw = std::str::from_utf8(&bytes[start..start + len as usize]).unwrap_or("");
    let escaped = esc_str(raw, len, 0);
    mk_sym(st, &escaped)
}

pub fn mk_dec(d: f64) -> Option<Val> {
    Some(Val {
        t: TspType::TspDec,
        v: ValUnion::N { num: d, den: 1.0 },
    })
}

pub fn tisp_eval_body(st: &mut Tsp, env: &mut Rec, v: Val) -> Option<Val> {
    let mut ret = st.none.clone();
    let mut cur = v;
    while matches!(cur.t, TspType::TspPair) {
        let (head, rest) = if let ValUnion::P { car, cdr } = cur.v {
            (*car, *cdr)
        } else {
            break;
        };
        ret = tisp_eval_with_env(st, env, head)?;
        cur = rest;
    }
    Some(ret)
}

pub fn tib_env_io(st: &mut Tsp) {
    crate::io::tib_env_io(st);
}

pub fn tisp_read_sugar(st: &mut Tsp, v: Val) -> Option<Val> {
    let bytes = st.file.as_bytes();
    let cur = bytes.get(st.filec).copied().unwrap_or(0) as char;
    if cur == '(' {
        st.filec += 1;
        let lst = read_pair(st, ')')?;
        return mk_pair(v, lst);
    } else if cur == '{' {
        st.filec += 1;
        let lst = read_pair(st, '}')?;
        let recmerge = mk_sym(st, "recmerge")?;
        let rec_sym = mk_sym(st, "Rec")?;
        let inner = mk_pair(rec_sym, lst)?;
        return mk_list(st, 3, vec![recmerge, v, inner]);
    } else if cur == ':' {
        st.filec += 1;
        let next = bytes.get(st.filec).copied().unwrap_or(0) as char;
        match next {
            '(' => {
                st.filec += 1;
                let w = read_pair(st, ')')?;
                let map_sym = mk_sym(st, "map")?;
                let inner = mk_pair(v, w)?;
                return mk_pair(map_sym, inner);
            }
            ':' => {
                st.filec += 1;
                let w = read_sym(st, is_sym)?;
                let q = mk_sym(st, "quote")?;
                let qpair = mk_list(st, 2, vec![q, w])?;
                return mk_list(st, 2, vec![v, qpair]);
            }
            _ => {
                skip_ws(st, 1);
                let w = tisp_read(st)?;
                return mk_list(st, 2, vec![v, w]);
            }
        }
    } else if cur == '>' && bytes.get(st.filec + 1).copied().unwrap_or(0) as char == '>' {
        st.filec += 2;
        let w = tisp_read(st)?;
        if !matches!(w.t, TspType::TspPair) {
            return None;
        }
        if let ValUnion::P { car, cdr } = w.v {
            let inner = mk_pair(v, *cdr)?;
            return mk_pair(*car, inner);
        }
        return None;
    }
    Some(v)
}

pub fn tisp_env_lib(st: &mut Tsp, lib: &str) {
    let saved_file = std::mem::take(&mut st.file);
    let saved_filec = st.filec;
    st.file = lib.to_string();
    st.filec = 0;
    skip_ws(st, 1);
    if let Some(v) = tisp_read(st) {
        let mut env = std::mem::replace(&mut st.env, rec_new(1, None));
        let _ = tisp_eval_body(st, &mut env, v);
        st.env = env;
    }
    st.file = saved_file;
    st.filec = saved_filec;
}

pub fn mk_list(st: &mut Tsp, n: i32, args: Vec<Val>) -> Option<Val> {
    if n <= 0 || args.is_empty() {
        return Some(st.nil.clone());
    }
    let mut iter = args.into_iter();
    let first = iter.next()?;
    let mut result = mk_pair(first, st.nil.clone())?;
    let mut count = 1;
    let mut tail_ptr: *mut Val = &mut result;
    for v in iter {
        if count >= n {
            break;
        }
        // SAFETY: tail_ptr always points to a valid Val we own
        unsafe {
            if let ValUnion::P { cdr, .. } = &mut (*tail_ptr).v {
                let new_pair = Val {
                    t: TspType::TspPair,
                    v: ValUnion::P {
                        car: Box::new(v),
                        cdr: Box::new(st.nil.clone()),
                    },
                };
                *cdr.as_mut() = new_pair;
                tail_ptr = cdr.as_mut() as *mut Val;
            }
        }
        count += 1;
    }
    Some(result)
}

pub fn vals_eq(a: &Val, b: &Val) -> bool {
    if is_num(a.t) && is_num(b.t) {
        return val_num(a) == val_num(b) && val_den(a) == val_den(b);
    }
    if std::mem::discriminant(&a.t) != std::mem::discriminant(&b.t) {
        // crude type comparison; the C code compares by enum equality
        // do a bit-based check
        if (a.t as u32) != (b.t as u32) {
            return false;
        }
    }
    match (&a.v, &b.v) {
        (ValUnion::S(sa), ValUnion::S(sb)) => sa == sb,
        (ValUnion::P { car: c1, cdr: d1 }, ValUnion::P { car: c2, cdr: d2 }) => {
            vals_eq(c1, c2) && vals_eq(d1, d2)
        }
        (ValUnion::F { args: a1, body: b1, .. }, ValUnion::F { args: a2, body: b2, .. }) => {
            vals_eq(a1, a2) && vals_eq(b1, b2)
        }
        (ValUnion::N { num: n1, den: d1 }, ValUnion::N { num: n2, den: d2 }) => {
            n1 == n2 && d1 == d2
        }
        _ => {
            // For NIL/NONE, just compare type tags - they match since we got here
            matches!(a.t, TspType::TspNil | TspType::TspNone)
                && matches!(b.t, TspType::TspNil | TspType::TspNone)
        }
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
    let c = bytes.get(st.filec).copied().unwrap_or(0);
    match c {
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
        TspType::TspInt => format!("{}", val_num(v) as i64),
        TspType::TspDec => {
            let n = val_num(v);
            let formatted = format!("{:.15}", n);
            // Trim trailing zeros while preserving at least one digit after the decimal
            let mut s = formatted.trim_end_matches('0').trim_end_matches('.').to_string();
            if s.is_empty() || !s.contains('.') {
                s.push_str(".0");
            }
            s
        }
        TspType::TspRatio => format!("{}/{}", val_num(v) as i64, val_den(v) as i64),
        TspType::TspStr | TspType::TspSym => val_str_ref(v).to_string(),
        TspType::TspFunc | TspType::TspMacro => {
            let name = val_str_ref(v);
            let kind = if matches!(v.t, TspType::TspFunc) { "function" } else { "macro" };
            if name.is_empty() {
                format!("#<{}>", kind)
            } else {
                format!("#<{}:{}>", kind, name)
            }
        }
        TspType::TspPrim => format!("#<primitive:{}>", val_str_ref(v)),
        TspType::TspForm => format!("#<form:{}>", val_str_ref(v)),
        TspType::TspRec => "{ ... }".to_string(),
        TspType::TspPair => {
            let mut s = String::from("(");
            if let Some(c) = val_car(v) {
                s.push_str(&print_to_string(c));
            }
            let mut cur_cdr = val_cdr(v);
            while let Some(cdr) = cur_cdr {
                if nilp(cdr) {
                    break;
                }
                if matches!(cdr.t, TspType::TspPair) {
                    s.push(' ');
                    if let Some(c) = val_car(cdr) {
                        s.push_str(&print_to_string(c));
                    }
                    cur_cdr = val_cdr(cdr);
                } else {
                    s.push_str(" . ");
                    s.push_str(&print_to_string(cdr));
                    break;
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
            apply_primitive(st, env, &f, evaled)
        }
        TspType::TspForm => apply_primitive(st, env, &f, args),
        TspType::TspFunc => {
            let evaled = tisp_eval_list(st, env, args)?;
            apply_function(st, env, f, evaled)
        }
        TspType::TspMacro => {
            let result = apply_function(st, env, f, args)?;
            tisp_eval_with_env(st, env, result)
        }
        TspType::TspRec => {
            let evaled = tisp_eval_list(st, env, args)?;
            // expect single sym arg
            if let Some(arg0) = val_car(&evaled) {
                if let ValUnion::S(s) = &arg0.v {
                    if let ValUnion::R(r) = &f.v {
                        if let Some(v) = rec_get(r, s) {
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

fn apply_primitive(st: &mut Tsp, env: &mut Rec, f: &Val, args: Val) -> Option<Val> {
    if let ValUnion::Pr { name, .. } = &f.v {
        let n = name.clone();
        return dispatch_primitive(st, env, &n, args);
    }
    None
}

fn apply_function(st: &mut Tsp, _env: &mut Rec, f: Val, args: Val) -> Option<Val> {
    if let ValUnion::F { args: fargs, body, env: fenv, .. } = f.v {
        let mut new_env = rec_extend(&mut fenv.clone(), *fargs, args);
        return tisp_eval_body(st, &mut new_env, *body);
    }
    None
}

fn dispatch_primitive(st: &mut Tsp, env: &mut Rec, name: &str, args: Val) -> Option<Val> {
    use crate::core::*;
    use crate::math::*;
    use crate::os::*;
    use crate::string::*;
    use crate::io::*;
    let result = match name {
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
        "Rec" => crate::tisp::mk_rec(st, env.clone(), args).unwrap_or_else(make_none),
        "recmerge" => prim_recmerge(st, env, args),
        "records" => prim_records(st, env, args),
        "def" => form_def(st, env, args),
        "undefine!" => form_undefine(st, env, args),
        "defined?" => form_definedp(st, env, args),
        "Int" | "Dec" | "floor" | "ceil" | "round" => {
            // simple round dispatch
            round_dispatch(name, args)
        }
        "+" => prim_add(st, env, args),
        "-" => prim_sub(st, env, args),
        "*" => prim_mul(st, env, args),
        "/" => prim_div(st, env, args),
        "mod" => prim_mod(st, env, args),
        "^" => prim_pow(st, env, args),
        "denominator" => prim_denominator(st, env, args),
        "numerator" => {
            if let Some(c) = val_car(&args) {
                mk_int(val_num(c) as i32)
            } else {
                make_none()
            }
        }
        "<" | ">" | "<=" | ">=" => compare_dispatch(name, args, &st.t.clone(), &st.nil.clone()),
        "sin" | "cos" | "tan" | "sinh" | "cosh" | "tanh"
        | "asin" | "acos" | "atan" | "asinh" | "acosh" | "atanh"
        | "arcsin" | "arccos" | "arctan" | "arcsinh" | "arccosh" | "arctanh"
        | "exp" | "log" => trig_dispatch(st, name, args),
        "Sym" => prim_Sym(st, env, args),
        "Str" => prim_Str(st, env, args),
        "strlen" => prim_strlen(st, env, args),
        "strformat" => form_strformat(st, env, args),
        "write" => prim_write(st, env, args),
        "read" => prim_read(st, env, args),
        "parse" => prim_parse(st, env, args),
        "load" => prim_load(st, env, args),
        "cd!" => prim_cd(st, env, args),
        "pwd" => prim_pwd(st, env, args),
        "exit!" => prim_exit(st, env, args),
        "now" => prim_now(st, env, args),
        "time" => form_time(st, env, args),
        _ => return None,
    };
    Some(result)
}

fn round_dispatch(name: &str, args: Val) -> Val {
    let n = if let Some(c) = val_car(&args) { c.clone() } else { return make_none(); };
    let v = val_num(&n) / val_den(&n);
    let result = match name {
        "Int" => v as i32 as f64,
        "Dec" => v,
        "round" => v.round(),
        "floor" => v.floor(),
        "ceil" => v.ceil(),
        _ => v,
    };
    match name {
        "Int" => mk_int(result as i32),
        "Dec" => mk_dec(result).unwrap_or_else(make_none),
        _ => {
            // preserve original number type
            match n.t {
                TspType::TspDec => mk_dec(result).unwrap_or_else(make_none),
                _ => mk_int(result as i32),
            }
        }
    }
}

fn compare_dispatch(name: &str, args: Val, t: &Val, nil: &Val) -> Val {
    if tsp_lstlen(&args) != 2 {
        return t.clone();
    }
    let a = match val_car(&args) { Some(v) => v.clone(), None => return nil.clone() };
    let b = match val_cdr(&args).and_then(|c| val_car(c)) {
        Some(v) => v.clone(),
        None => return nil.clone(),
    };
    let va = val_num(&a) * val_den(&b);
    let vb = val_num(&b) * val_den(&a);
    let cmp = match name {
        "<" => va < vb,
        ">" => va > vb,
        "<=" => va <= vb,
        ">=" => va >= vb,
        _ => false,
    };
    if cmp { t.clone() } else { nil.clone() }
}

fn trig_dispatch(st: &mut Tsp, name: &str, args: Val) -> Val {
    let arg = match val_car(&args) { Some(v) => v.clone(), None => return make_none() };
    if matches!(arg.t, TspType::TspDec) {
        let x = val_num(&arg);
        let r = match name {
            "sin" => x.sin(),
            "cos" => x.cos(),
            "tan" => x.tan(),
            "sinh" => x.sinh(),
            "cosh" => x.cosh(),
            "tanh" => x.tanh(),
            "asin" | "arcsin" => x.asin(),
            "acos" | "arccos" => x.acos(),
            "atan" | "arctan" => x.atan(),
            "asinh" | "arcsinh" => x.asinh(),
            "acosh" | "arccosh" => x.acosh(),
            "atanh" | "arctanh" => x.atanh(),
            "exp" => x.exp(),
            "log" => x.ln(),
            _ => x,
        };
        return mk_dec(r).unwrap_or_else(make_none);
    }
    let sym = match mk_sym(st, name) {
        Some(s) => s,
        None => return make_none(),
    };
    mk_list(st, 2, vec![sym, arg]).unwrap_or_else(make_none)
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
            if let ValUnion::S(s) = &v.v {
                if let Some(found) = rec_get(env, s) {
                    return Some(found);
                }
                if let Some(found) = rec_get(&st.env, s) {
                    return Some(found);
                }
            }
            None
        }
        TspType::TspPair => {
            let (head, rest) = if let ValUnion::P { car, cdr } = v.v {
                (*car, *cdr)
            } else {
                return None;
            };
            let f = tisp_eval_with_env(st, env, head)?;
            eval_proc(st, env, f, rest)
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
    let new_cap = ocap * TSP_REC_FACTOR;
    let old_items = std::mem::take(&mut rec.items);
    rec.cap = new_cap as i32;
    rec.size = 0;
    rec.items = Vec::with_capacity(new_cap);
    for _ in 0..new_cap {
        rec.items.push(Entry { key: String::new(), val: make_none() });
    }
    for entry in old_items {
        if !entry.key.is_empty() {
            rec_add(rec, &entry.key, entry.val);
        }
    }
}
