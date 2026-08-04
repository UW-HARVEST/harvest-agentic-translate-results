use std::io::{Read, Write};
use std::path::Path;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

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

fn dummy_prim(_: Tsp, _: Rec, _: Val) -> Val {
    mk_val(TspType::TspNone)
}

fn type_bits(t: TspType) -> u32 {
    t as u32
}

fn type_matches(t: TspType, mask: u32) -> bool {
    type_bits(t) & mask != 0
}

fn clone_val(v: &Val) -> Val {
    Val {
        t: v.t,
        v: match &v.v {
            ValUnion::S(s) => ValUnion::S(s.clone()),
            ValUnion::N { num, den } => ValUnion::N {
                num: *num,
                den: *den,
            },
            ValUnion::Pr { name, pr } => ValUnion::Pr {
                name: name.clone(),
                pr: *pr,
            },
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
        },
    }
}

fn clone_rec(rec: &Rec) -> Rec {
    Rec {
        size: rec.size,
        cap: rec.cap,
        items: rec
            .items
            .iter()
            .map(|entry| Entry {
                key: entry.key.clone(),
                val: clone_val(&entry.val),
            })
            .collect(),
        next: rec.next.as_ref().map(|next| Box::new(clone_rec(next))),
    }
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

fn val_str(v: &Val) -> &str {
    match &v.v {
        ValUnion::S(s) => s.as_str(),
        _ => "",
    }
}

fn pair_car(v: &Val) -> Option<&Val> {
    match &v.v {
        ValUnion::P { car, .. } => Some(car),
        _ => None,
    }
}

fn pair_cdr(v: &Val) -> Option<&Val> {
    match &v.v {
        ValUnion::P { cdr, .. } => Some(cdr),
        _ => None,
    }
}

fn mk_bool(st: &Tsp, truthy: bool) -> Val {
    if truthy {
        clone_val(&st.t)
    } else {
        clone_val(&st.nil)
    }
}

fn mask_type_str(mask: u32) -> &'static str {
    if mask == TSP_EXPR {
        "Expr"
    } else if mask == TSP_RATIONAL {
        "Rational"
    } else if mask == TSP_NUM {
        "Num"
    } else if mask == TspType::TspNone as u32 {
        "Void"
    } else if mask == TspType::TspNil as u32 {
        "Nil"
    } else if mask == TspType::TspInt as u32 {
        "Int"
    } else if mask == TspType::TspDec as u32 {
        "Dec"
    } else if mask == TspType::TspRatio as u32 {
        "Ratio"
    } else if mask == TspType::TspStr as u32 {
        "Str"
    } else if mask == TspType::TspSym as u32 {
        "Sym"
    } else if mask == TspType::TspPrim as u32 {
        "Prim"
    } else if mask == TspType::TspForm as u32 {
        "Form"
    } else if mask == TspType::TspFunc as u32 {
        "Func"
    } else if mask == TspType::TspMacro as u32 {
        "Macro"
    } else if mask == TspType::TspPair as u32 {
        "Pair"
    } else if mask == TspType::TspRec as u32 {
        "Rec"
    } else {
        "Invalid"
    }
}

fn tsp_warnf(message: String) -> Option<Val> {
    eprintln!("; tisp: error: {message}");
    None
}

fn tsp_warn(message: &str) -> Option<Val> {
    tsp_warnf(message.to_string())
}

fn arg_len(args: &Val) -> i32 {
    tsp_lstlen(args)
}

fn arg_num(args: &Val, name: &str, nargs: i32) -> Result<(), Option<Val>> {
    let got = arg_len(args);
    if nargs > -1 && got != nargs {
        Err(tsp_warnf(format!(
            "{name}: expected {nargs} argument{}, received {got}",
            if nargs == 1 { "" } else { "s" }
        )))
    } else {
        Ok(())
    }
}

fn arg_min(args: &Val, name: &str, nargs: i32) -> Result<(), Option<Val>> {
    let got = arg_len(args);
    if got < nargs {
        Err(tsp_warnf(format!(
            "{name}: expected at least {nargs} argument{}, received {got}",
            if nargs == 1 { "" } else { "s" }
        )))
    } else {
        Ok(())
    }
}

fn arg_max(args: &Val, name: &str, nargs: i32) -> Result<(), Option<Val>> {
    let got = arg_len(args);
    if got > nargs {
        Err(tsp_warnf(format!(
            "{name}: expected at no more than {nargs} argument{}, received {got}",
            if nargs == 1 { "" } else { "s" }
        )))
    } else {
        Ok(())
    }
}

fn arg_type(arg: &Val, name: &str, mask: u32) -> Result<(), Option<Val>> {
    if type_matches(arg.t, mask) {
        Ok(())
    } else {
        Err(tsp_warnf(format!(
            "{name}: expected {}, received {}",
            mask_type_str(mask),
            tsp_type_str(arg.t)
        )))
    }
}

fn list_to_vec(v: &Val) -> Vec<Val> {
    let mut out = Vec::new();
    let mut cur = v;
    while cur.t == TspType::TspPair {
        if let Some(car) = pair_car(cur) {
            out.push(clone_val(car));
        }
        if let Some(cdr) = pair_cdr(cur) {
            cur = cdr;
        } else {
            break;
        }
    }
    out
}

fn vec_to_list(st: &Tsp, values: Vec<Val>) -> Val {
    let mut ret = clone_val(&st.nil);
    for value in values.into_iter().rev() {
        ret = mk_pair(value, ret).unwrap_or_else(|| clone_val(&st.nil));
    }
    ret
}

fn render_val(v: &Val) -> String {
    match v.t {
        TspType::TspNone => "Void".to_string(),
        TspType::TspNil => "Nil".to_string(),
        TspType::TspInt => format!("{}", val_num(v) as i32),
        TspType::TspDec => {
            format_dec(val_num(v))
        }
        TspType::TspRatio => format!("{}/{}", val_num(v) as i32, val_den(v) as i32),
        TspType::TspStr | TspType::TspSym => val_str(v).to_string(),
        TspType::TspFunc | TspType::TspMacro => {
            if let ValUnion::F { name, .. } = &v.v {
                let label = if v.t == TspType::TspFunc {
                    "function"
                } else {
                    "macro"
                };
                if name.is_empty() {
                    format!("#<{label}>")
                } else {
                    format!("#<{label}:{name}>")
                }
            } else {
                "#<function>".to_string()
            }
        }
        TspType::TspPrim | TspType::TspForm => {
            if let ValUnion::Pr { name, .. } = &v.v {
                let label = if v.t == TspType::TspPrim { "primitive" } else { "form" };
                format!("#<{label}:{name}>")
            } else {
                "#<primitive>".to_string()
            }
        }
        TspType::TspRec => {
            let mut s = String::from("{");
            if let ValUnion::R(r) = &v.v {
                let mut printed = 0usize;
                let mut cur = Some(r);
                while let Some(rec) = cur {
                    for entry in &rec.items {
                        if printed == TSP_REC_MAX_PRINT {
                            s.push_str(" ...");
                            s.push_str(" }");
                            return s;
                        }
                        s.push(' ');
                        s.push_str(&entry.key);
                        s.push_str(": ");
                        s.push_str(&render_val(&entry.val));
                        printed += 1;
                    }
                    cur = rec.next.as_deref();
                }
            }
            s.push_str(" }");
            s
        }
        TspType::TspPair => {
            let mut s = String::from("(");
            if let Some(car) = pair_car(v) {
                s.push_str(&render_val(car));
            }
            let mut cur = pair_cdr(v);
            while let Some(node) = cur {
                if node.t == TspType::TspPair {
                    if let Some(car) = pair_car(node) {
                        s.push(' ');
                        s.push_str(&render_val(car));
                    }
                    cur = pair_cdr(node);
                } else if node.t == TspType::TspNil {
                    break;
                } else {
                    s.push_str(" . ");
                    s.push_str(&render_val(node));
                    break;
                }
            }
            s.push(')');
            s
        }
    }
}

fn format_dec(num: f64) -> String {
    let abs = num.abs();
    if abs != 0.0 && !(1e-4..1e15).contains(&abs) {
        let scientific = format!("{:.14e}", num);
        if let Some((mantissa, exponent)) = scientific.split_once('e') {
            let mut mantissa = mantissa.trim_end_matches('0').trim_end_matches('.').to_string();
            if mantissa.is_empty() {
                mantissa.push('0');
            }
            let exp_num = exponent.parse::<i32>().unwrap_or(0);
            return format!("{mantissa}e{exp_num:+03}");
        }
    }
    let mut s = format!("{:.15}", num);
    while s.contains('.') && s.ends_with('0') {
        s.pop();
    }
    if s.ends_with('.') {
        s.push('0');
    }
    if !s.contains('.') && !s.contains('e') && !s.contains('E') {
        s.push_str(".0");
    }
    s
}

fn fget(st: &Tsp) -> Option<char> {
    st.file.as_bytes().get(st.filec).map(|b| *b as char)
}

fn fgetat(st: &Tsp, offset: isize) -> Option<char> {
    let idx = st.filec as isize + offset;
    if idx < 0 {
        None
    } else {
        st.file.as_bytes().get(idx as usize).map(|b| *b as char)
    }
}

fn finc(st: &mut Tsp) {
    st.filec += 1;
}

fn fincn(st: &mut Tsp, n: usize) {
    st.filec += n;
}

fn starts_at(st: &Tsp, prefix: &str) -> bool {
    st.file[st.filec..].starts_with(prefix)
}

fn prim_name(v: &Val) -> &str {
    match &v.v {
        ValUnion::Pr { name, .. } => name.as_str(),
        _ => "",
    }
}

fn func_name(v: &Val) -> &str {
    match &v.v {
        ValUnion::F { name, .. } => name.as_str(),
        _ => "",
    }
}

fn func_args(v: &Val) -> Option<Val> {
    match &v.v {
        ValUnion::F { args, .. } => Some(clone_val(args)),
        _ => None,
    }
}

fn func_body(v: &Val) -> Option<Val> {
    match &v.v {
        ValUnion::F { body, .. } => Some(clone_val(body)),
        _ => None,
    }
}

fn value_as_rec(v: &Val) -> Option<&Rec> {
    match &v.v {
        ValUnion::R(r) => Some(r),
        _ => None,
    }
}

fn read_str_force(st: &mut Tsp, s: &str) -> Val {
    mk_str(st, s).unwrap_or_else(|| clone_val(&st.none))
}

fn read_sym_force(st: &mut Tsp, s: &str) -> Val {
    mk_sym(st, s).unwrap_or_else(|| clone_val(&st.none))
}

pub fn rec_add(rec: &mut Rec, key: &str, val: Val) {
    if let Some(existing) = rec.items.iter_mut().find(|entry| entry.key == key) {
        existing.val = val;
        rec.size = rec.items.len() as i32;
        return;
    }
    rec.items.push(Entry {
        key: key.to_string(),
        val,
    });
    rec.size = rec.items.len() as i32;
    if rec.cap > 0 && rec.size as usize > rec.cap as usize / TSP_REC_FACTOR {
        rec_grow(rec);
    }
}

pub fn mk_rat(num: i32, den: i32) -> Option<Val> {
    if den == 0 {
        return tsp_warn("division by zero");
    }
    let mut num_mut = num;
    let mut den_mut = den;
    frac_reduce(&mut num_mut, &mut den_mut);
    if den_mut < 0 {
        den_mut = den_mut.abs();
        num_mut = -num_mut;
    }
    if den_mut == 1 {
        return Some(mk_int(num_mut));
    }
    Some(Val {
        t: TspType::TspRatio,
        v: ValUnion::N {
            num: num_mut as f64,
            den: den_mut as f64,
        },
    })
}

pub fn mk_val(t: TspType) -> Val {
    let v = match t {
        TspType::TspNone | TspType::TspNil | TspType::TspStr | TspType::TspSym => {
            ValUnion::S(String::new())
        }
        TspType::TspInt | TspType::TspDec | TspType::TspRatio => ValUnion::N { num: 0.0, den: 1.0 },
        TspType::TspPrim | TspType::TspForm => ValUnion::Pr {
            name: String::new(),
            pr: dummy_prim,
        },
        TspType::TspFunc | TspType::TspMacro => ValUnion::F {
            name: String::new(),
            args: Box::new(Val {
                t: TspType::TspNil,
                v: ValUnion::S(String::new()),
            }),
            body: Box::new(Val {
                t: TspType::TspNil,
                v: ValUnion::S(String::new()),
            }),
            env: rec_new(1, None),
        },
        TspType::TspPair => ValUnion::P {
            car: Box::new(Val {
                t: TspType::TspNil,
                v: ValUnion::S(String::new()),
            }),
            cdr: Box::new(Val {
                t: TspType::TspNil,
                v: ValUnion::S(String::new()),
            }),
        },
        TspType::TspRec => ValUnion::R(rec_new(1, None)),
    };
    Val { t, v }
}

pub fn tsp_lstlen(v: &Val) -> i32 {
    let mut len = 0;
    let mut cur = v;
    while cur.t == TspType::TspPair {
        len += 1;
        if let Some(next) = pair_cdr(cur) {
            cur = next;
        } else {
            break;
        }
    }
    if cur.t == TspType::TspNil {
        len
    } else {
        -(len + 1)
    }
}

pub fn tisp_env_init(cap: usize) -> Tsp {
    let nil = Val {
        t: TspType::TspNil,
        v: ValUnion::S(String::new()),
    };
    let none = Val {
        t: TspType::TspNone,
        v: ValUnion::S(String::new()),
    };
    let t = Val {
        t: TspType::TspSym,
        v: ValUnion::S("True".to_string()),
    };
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
    let true_val = clone_val(&st.t);
    let nil_val = clone_val(&st.nil);
    let none_val = clone_val(&st.none);
    rec_add(&mut st.env, "True", true_val);
    rec_add(&mut st.env, "Nil", nil_val);
    rec_add(&mut st.env, "Void", none_val);
    rec_add(&mut st.env, "bt", clone_val(&st.nil));
    if let Some(version) = mk_str(&mut st, "0.1") {
        rec_add(&mut st.env, "version", version);
    }
    st
}

pub fn tib_env_os(st: &mut Tsp) {
    register_prim(st, "cd!");
    register_prim(st, "pwd");
    register_prim(st, "exit!");
    register_prim(st, "now");
    register_form(st, "time");
}

pub fn read_num(st: &mut Tsp) -> Val {
    let sign = read_sign(st);
    let num = read_int(st);
    match fget(st) {
        Some('/') => {
            finc(st);
            let rem = st.file.get(st.filec..).unwrap_or_default().to_string();
            if !isnum(&rem) {
                return clone_val(&st.none);
            }
            mk_rat(sign * num, read_sign(st) * read_int(st)).unwrap_or_else(|| clone_val(&st.none))
        }
        Some('.') => {
            finc(st);
            let oldc = st.filec;
            let mut d = read_int(st) as f64;
            let mut size = (st.filec - oldc) as i32;
            while size > 0 {
                d /= 10.0;
                size -= 1;
            }
            read_sci(st, sign as f64 * (num as f64 + d), 0).unwrap_or_else(|| clone_val(&st.none))
        }
        _ => read_sci(st, (sign * num) as f64, 1).unwrap_or_else(|| clone_val(&st.none)),
    }
}

pub fn entry_get<'a>(rec: &'a Rec, key: &'a str) -> Option<&'a Entry> {
    rec.items.iter().find(|entry| entry.key == key)
}

pub fn tib_env_string(st: &mut Tsp) {
    register_prim(st, "Sym");
    register_prim(st, "Str");
    register_prim(st, "strlen");
    register_form(st, "strformat");
}

pub fn prepend_bt(_: &mut Tsp, _: &mut Rec, _: Val) {}

pub fn rec_get(rec: &Rec, key: &str) -> Option<Val> {
    if let Some(entry) = rec.items.iter().find(|entry| entry.key == key) {
        return Some(clone_val(&entry.val));
    }
    rec.next.as_deref().and_then(|next| rec_get(next, key))
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
    let ret = mk_pair(clone_val(&st.nil), clone_val(&st.nil))?;
    let skipnl = if endchar == '\n' { 0 } else { 1 };
    skip_ws(st, skipnl);
    let mut values = Vec::new();
    let mut dotted: Option<Val> = None;
    while let Some(ch) = fget(st) {
        if ch == endchar {
            break;
        }
        let v = tisp_read(st)?;
        if v.t == TspType::TspSym && val_str(&v) == "." {
            skip_ws(st, skipnl);
            dotted = Some(tisp_read(st)?);
            break;
        }
        values.push(v);
        skip_ws(st, skipnl);
    }
    skip_ws(st, skipnl);
    if skipnl != 0 && fget(st) != Some(endchar) {
        return tsp_warnf(format!("did not find closing '{endchar}'"));
    }
    if fget(st) == Some(endchar) {
        finc(st);
    }
    let mut out = dotted.unwrap_or_else(|| clone_val(&st.nil));
    for value in values.into_iter().rev() {
        out = mk_pair(value, out)?;
    }
    if ret.t == TspType::TspPair {
        Some(out)
    } else {
        None
    }
}

pub fn tisp_read_sexpr(st: &mut Tsp) -> Option<Val> {
    const PREFIX: [(&str, &str); 6] = [
        ("'", "quote"),
        ("`", "quasiquote"),
        (",@", "unquote-splice"),
        (",", "unquote"),
        ("@", "Func"),
        ("f\"", "strformat"),
    ];
    skip_ws(st, 1);
    if st.file.get(st.filec..).unwrap_or_default().is_empty() {
        return Some(clone_val(&st.none));
    }
    let remaining = st.file.get(st.filec..).unwrap_or_default().to_string();
    if isnum(&remaining) {
        return Some(read_num(st));
    }
    if fget(st) == Some('"') {
        return read_str(st, read_str_force);
    }
    if fget(st) == Some('~') {
        return read_str(st, read_sym_force);
    }
    for (prefix, name) in PREFIX {
        if starts_at(st, prefix) {
            let step = prefix.len() - usize::from(prefix.as_bytes().get(1) == Some(&b'"'));
            fincn(st, step);
            let v = tisp_read(st)?;
            let name_sym = mk_sym(st, name).unwrap_or_else(|| clone_val(&st.none));
            return mk_list(st, 2, vec![name_sym, v]);
        }
    }
    match fget(st) {
        Some(ch) if is_op(ch) => read_sym(st, is_op),
        Some(ch) if is_sym(ch) => read_sym(st, is_sym),
        Some('(') => {
            finc(st);
            read_pair(st, ')')
        }
        Some('[') => {
            finc(st);
            let list_sym = mk_sym(st, "list").unwrap_or_else(|| clone_val(&st.none));
            let body = read_pair(st, ']')?;
            mk_pair(list_sym, body)
        }
        Some('{') => {
            finc(st);
            let body = read_pair(st, '}')?;
            mk_pair(
                mk_sym(st, "Rec").unwrap_or_else(|| clone_val(&st.none)),
                body,
            )
        }
        Some(ch) => tsp_warnf(format!("could not read given input '{ch}' ({})", ch as i32)),
        None => Some(clone_val(&st.none)),
    }
}

pub fn is_sym(c: char) -> bool {
    c.is_ascii_alphanumeric() || TSP_SYM_CHARS.contains(c)
}

pub fn mk_sym(st: &mut Tsp, s: &str) -> Option<Val> {
    if let Some(v) = rec_get(&st.syms, s) {
        return Some(v);
    }
    let ret = Val {
        t: TspType::TspSym,
        v: ValUnion::S(s.to_string()),
    };
    rec_add(&mut st.syms, s, clone_val(&ret));
    Some(ret)
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
    *num /= b;
    *den /= b;
}

pub fn tisp_read_line(st: &mut Tsp, level: i32) -> Option<Val> {
    let mut ret = read_pair(st, '\n')?;
    if ret.t != TspType::TspPair {
        ret = mk_pair(ret, clone_val(&st.nil))?;
    }
    let mut values = list_to_vec(&ret);
    while let Some(ch) = fget(st) {
        let mut newlevel = 0usize;
        for byte in st.file.as_bytes().iter().skip(st.filec) {
            if *byte == b' ' || *byte == b'\t' {
                newlevel += 1;
            } else {
                break;
            }
        }
        if newlevel as i32 <= level || ch == '\n' {
            break;
        }
        st.filec += newlevel;
        values.push(tisp_read_line(st, newlevel as i32)?);
    }
    if values.len() == 1 {
        Some(values.remove(0))
    } else {
        mk_list(st, values.len() as i32, values)
    }
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

pub fn isnum(str: &str) -> bool {
    let bytes = str.as_bytes();
    match bytes {
        [] => false,
        [first, second, ..] if (*first == b'-' || *first == b'+') => {
            second.is_ascii_digit() || *second == b'.'
        }
        [first, second, ..] if *first == b'.' => second.is_ascii_digit(),
        [first, ..] => first.is_ascii_digit(),
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

pub fn mk_str(st: &mut Tsp, s: &str) -> Option<Val> {
    if let Some(v) = rec_get(&st.strs, s) {
        return Some(v);
    }
    let ret = Val {
        t: TspType::TspStr,
        v: ValUnion::S(s.to_string()),
    };
    rec_add(&mut st.strs, s, clone_val(&ret));
    Some(ret)
}

pub fn is_op(c: char) -> bool {
    TSP_OP_CHARS.contains(c)
}

pub fn esc_str(s: &str, len: i32, do_esc: i32) -> String {
    let mut ret = String::new();
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0usize;
    while i < len as usize && i < chars.len() {
        let c = chars[i];
        if c == '\\' && do_esc != 0 && i + 1 < chars.len() {
            i += 1;
            ret.push(esc_char(chars[i]));
        } else {
            ret.push(c);
        }
        i += 1;
    }
    ret
}

pub fn tib_env_core(st: &mut Tsp) {
    register_prim(st, "car");
    register_prim(st, "cdr");
    register_prim(st, "cons");
    register_form(st, "quote");
    register_prim(st, "eval");
    register_prim(st, "=");
    register_form(st, "cond");
    register_form(st, "do");
    register_prim(st, "typeof");
    register_prim(st, "procprops");
    register_form(st, "Func");
    register_form(st, "Macro");
    register_prim(st, "error");
    register_form(st, "Rec");
    register_prim(st, "recmerge");
    register_prim(st, "records");
    register_form(st, "def");
    register_form(st, "undefine!");
    register_form(st, "defined?");
}

pub fn skip_ws(st: &mut Tsp, skipnl: i32) {
    loop {
        while let Some(ch) = fget(st) {
            let is_ws = if skipnl != 0 {
                matches!(ch, ' ' | '\t' | '\n' | '\r')
            } else {
                matches!(ch, ' ' | '\t')
            };
            if is_ws {
                finc(st);
            } else {
                break;
            }
        }
        if fget(st) != Some(';') {
            break;
        }
        while let Some(ch) = fget(st) {
            if ch == '\n' {
                if skipnl == 0 {
                    break;
                }
                finc(st);
                break;
            }
            finc(st);
        }
    }
}

pub fn rec_extend(rec: &mut Rec, args: Val, vals: Val) -> Rec {
    let argnum = TSP_REC_FACTOR as i32 * tsp_lstlen(&args);
    let cap = if argnum > 0 { argnum as usize } else { (-argnum + 1) as usize };
    let mut ret = rec_new(cap, Some(Box::new(clone_rec(rec))));
    let mut args_cur = args;
    let mut vals_cur = vals;
    loop {
        let (arg, val, advance) = if args_cur.t == TspType::TspPair {
            (
                pair_car(&args_cur).map(clone_val),
                pair_car(&vals_cur).map(clone_val),
                true,
            )
        } else {
            (Some(clone_val(&args_cur)), Some(clone_val(&vals_cur)), false)
        };
        match (arg, val) {
            (Some(arg), Some(val)) => {
                if arg.t != TspType::TspSym {
                    break;
                }
                rec_add(&mut ret, val_str(&arg), val);
            }
            _ => break,
        }
        if !advance {
            break;
        }
        args_cur = match pair_cdr(&args_cur) {
            Some(next) => clone_val(next),
            None => break,
        };
        vals_cur = match pair_cdr(&vals_cur) {
            Some(next) => clone_val(next),
            None => break,
        };
    }
    ret
}

pub fn hash(key: &str) -> u32 {
    let mut h = 0u32;
    for c in key.bytes() {
        h = h.saturating_mul(33).saturating_add(c as u32);
    }
    h
}

pub fn mk_rec(st: &mut Tsp, env: Rec, assoc: Val) -> Option<Val> {
    let mut ret = Val {
        t: TspType::TspRec,
        v: ValUnion::R(rec_new(
            match tsp_lstlen(&assoc) {
                len if len > 0 => len as usize * TSP_REC_FACTOR,
                len => (-len + 1) as usize,
            },
            None,
        )),
    };
    let mut scope = rec_new(4, Some(Box::new(clone_rec(&env))));
    rec_add(&mut scope, "this", clone_val(&ret));
    let mut cur = assoc;
    while cur.t == TspType::TspPair {
        let item = pair_car(&cur).map(clone_val)?;
        if item.t == TspType::TspPair {
            let key_val = pair_car(&item)?;
            if key_val.t == TspType::TspSym || key_val.t == TspType::TspStr {
                let value_expr = pair_cdr(&item)
                    .and_then(pair_car)
                    .map(clone_val)
                    .ok_or_else(|| tsp_warn("Rec: missing key symbol or string"))
                    .ok()?;
                let v = eval_with_env(st, &mut scope, value_expr)?;
                if let ValUnion::R(r) = &mut ret.v {
                    rec_add(r, val_str(key_val), v);
                }
            } else {
                return tsp_warn("Rec: missing key symbol or string");
            }
        } else if item.t == TspType::TspSym {
            let v = eval_with_env(st, &mut scope, clone_val(&item))?;
            if let ValUnion::R(r) = &mut ret.v {
                rec_add(r, val_str(&item), v);
            }
        } else {
            return tsp_warn("Rec: missing key symbol or string");
        }
        cur = pair_cdr(&cur).map(clone_val)?;
    }
    Some(ret)
}

pub fn tisp_read(st: &mut Tsp) -> Option<Val> {
    let mut v = tisp_read_sexpr(st)?;
    while matches!(fget(st), Some('(' | ':' | '>' | '{')) {
        v = tisp_read_sugar(st, v)?;
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
    register_prim(st, "Int");
    register_prim(st, "Dec");
    register_prim(st, "floor");
    register_prim(st, "ceil");
    register_prim(st, "round");
    register_prim(st, "numerator");
    register_prim(st, "denominator");
    register_prim(st, "+");
    register_prim(st, "-");
    register_prim(st, "*");
    register_prim(st, "/");
    register_prim(st, "mod");
    register_prim(st, "^");
    register_prim(st, "<");
    register_prim(st, ">");
    register_prim(st, "<=");
    register_prim(st, ">=");
    for name in [
        "sin", "cos", "tan", "sinh", "cosh", "tanh", "arcsin", "arccos", "arctan", "arcsinh",
        "arccosh", "arctanh", "exp", "log",
    ] {
        register_prim(st, name);
    }
}

pub fn tisp_eval_list(st: &mut Tsp, env: &mut Rec, v: Val) -> Option<Val> {
    let mut out = Vec::new();
    let mut cur = v;
    while cur.t == TspType::TspPair {
        let value = pair_car(&cur).map(clone_val)?;
        out.push(eval_with_env(st, env, value)?);
        cur = pair_cdr(&cur).map(clone_val)?;
    }
    if cur.t == TspType::TspNil {
        mk_list(st, out.len() as i32, out)
    } else {
        let tail = eval_with_env(st, env, cur)?;
        let mut ret = tail;
        for value in out.into_iter().rev() {
            ret = mk_pair(value, ret)?;
        }
        Some(ret)
    }
}

pub fn read_sci(st: &mut Tsp, mut val: f64, isint: i32) -> Option<Val> {
    if !matches!(fget(st), Some('e' | 'E')) {
        return if isint != 0 {
            Some(mk_int(val as i32))
        } else {
            mk_dec(val)
        };
    }
    finc(st);
    let sign = if read_sign(st) == 1 { 10.0 } else { 0.1 };
    let mut expo = read_int(st);
    while expo > 0 {
        val *= sign;
        expo -= 1;
    }
    if isint != 0 {
        Some(mk_int(val as i32))
    } else {
        mk_dec(val)
    }
}

pub fn read_int(st: &mut Tsp) -> i32 {
    let mut ret = 0;
    while let Some(ch) = fget(st) {
        if ch.is_ascii_digit() {
            ret = ret * 10 + (ch as i32 - '0' as i32);
            finc(st);
        } else {
            break;
        }
    }
    ret
}

pub fn rec_new(cap: usize, next: Option<Box<Rec>>) -> Rec {
    Rec {
        size: 0,
        cap: cap.max(1) as i32,
        items: Vec::new(),
        next,
    }
}

pub fn read_str(st: &mut Tsp, mk_fn: fn(&mut Tsp, &str) -> Val) -> Option<Val> {
    let endchar = if std::ptr::fn_addr_eq(mk_fn, read_str_force as fn(&mut Tsp, &str) -> Val) {
        '"'
    } else {
        '~'
    };
    finc(st);
    let start = st.filec;
    let mut len = 0i32;
    while let Some(ch) = fget(st) {
        if ch == endchar {
            break;
        }
        if ch == '\\' && fgetat(st, -1) != Some('\\') {
            finc(st);
        }
        if fget(st).is_none() {
            return tsp_warnf(format!("reached end before closing {endchar}"));
        }
        finc(st);
        len += 1;
    }
    let raw = st.file[start..st.filec].to_string();
    if fget(st) == Some(endchar) {
        finc(st);
    }
    Some(mk_fn(st, &esc_str(&raw, len, i32::from(endchar == '"'))))
}

pub fn read_sym(st: &mut Tsp, is_char: fn(char) -> bool) -> Option<Val> {
    let start = st.filec;
    while let Some(ch) = fget(st) {
        if is_char(ch) {
            finc(st);
        } else {
            break;
        }
    }
    let s = st.file[start..st.filec].to_string();
    mk_sym(st, &esc_str(&s, s.len() as i32, 0))
}

pub fn mk_dec(d: f64) -> Option<Val> {
    Some(Val {
        t: TspType::TspDec,
        v: ValUnion::N { num: d, den: 1.0 },
    })
}

pub fn tisp_eval_body(st: &mut Tsp, env: &mut Rec, v: Val) -> Option<Val> {
    let mut ret = clone_val(&st.none);
    let mut cur = v;
    while cur.t == TspType::TspPair {
        let expr = pair_car(&cur).map(clone_val)?;
        ret = eval_with_env(st, env, expr)?;
        cur = pair_cdr(&cur).map(clone_val)?;
    }
    Some(ret)
}

pub fn tib_env_io(st: &mut Tsp) {
    register_prim(st, "write");
    register_prim(st, "read");
    register_prim(st, "parse");
    register_prim(st, "load");
}

pub fn tisp_read_sugar(st: &mut Tsp, v: Val) -> Option<Val> {
    match fget(st) {
        Some('(') => {
            finc(st);
            let lst = read_pair(st, ')')?;
            mk_pair(v, lst)
        }
        Some('{') => {
            finc(st);
            let lst = read_pair(st, '}')?;
            let recmerge_sym = mk_sym(st, "recmerge").unwrap_or_else(|| clone_val(&st.none));
            let rec_sym = mk_sym(st, "Rec").unwrap_or_else(|| clone_val(&st.none));
            let rec_form = mk_pair(rec_sym, lst)?;
            mk_list(st, 3, vec![recmerge_sym, v, rec_form])
        }
        Some(':') => {
            finc(st);
            match fget(st) {
                Some('(') => {
                    finc(st);
                    let w = read_pair(st, ')')?;
                    mk_pair(
                        mk_sym(st, "map").unwrap_or_else(|| clone_val(&st.none)),
                        mk_pair(v, w)?,
                    )
                }
                Some(':') => {
                    finc(st);
                    let w = read_sym(st, is_sym)?;
                    let quote_sym = mk_sym(st, "quote").unwrap_or_else(|| clone_val(&st.none));
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
        Some('>') if fgetat(st, 1) == Some('>') => {
            finc(st);
            finc(st);
            let w = tisp_read(st)?;
            if w.t != TspType::TspPair {
                return tsp_warn("invalid UFCS");
            }
            let head = pair_car(&w).map(clone_val)?;
            let tail = pair_cdr(&w).map(clone_val)?;
            mk_pair(head, mk_pair(v, tail)?)
        }
        _ => Some(v),
    }
}

pub fn tisp_env_lib(st: &mut Tsp, lib: &str) {
    let sources = if lib == "tibs" {
        vec![
            "c_src/tib/core.tsp",
            "c_src/tib/list.tsp",
            "c_src/tib/math.tsp",
            "c_src/tib/io.tsp",
            "c_src/tib/os.tsp",
            "c_src/tib/doc.tsp",
        ]
    } else {
        vec![lib]
    };
    let old_file = st.file.clone();
    let old_filec = st.filec;
    let mut body_parts = Vec::new();
    for source in sources {
        let content = std::fs::read_to_string(source).or_else(|_| std::fs::read_to_string(format!("{source}.tsp")));
        if let Ok(text) = content {
            body_parts.push(text);
        } else if Path::new(source).exists() {
            continue;
        }
    }
    if body_parts.is_empty() {
        st.file = old_file;
        st.filec = old_filec;
        return;
    }
    let source = body_parts.join("\n");
    let source_str = mk_str(st, &source).unwrap_or_else(|| clone_val(&st.none));
    let parse_arg = mk_pair(source_str, clone_val(&st.nil)).unwrap_or_else(|| clone_val(&st.nil));
    let parsed = builtin_parse(st, parse_arg);
    if let Some(v) = parsed {
        let mut env = std::mem::replace(&mut st.env, rec_new(1, None));
        let _ = tisp_eval_body(st, &mut env, v);
        st.env = env;
    }
    st.file = old_file;
    st.filec = old_filec;
}

pub fn mk_list(st: &mut Tsp, n: i32, args: Vec<Val>) -> Option<Val> {
    if n <= 0 || args.is_empty() {
        return Some(clone_val(&st.nil));
    }
    Some(vec_to_list(st, args))
}

pub fn vals_eq(a: &Val, b: &Val) -> bool {
    if type_matches(a.t, TSP_NUM) && type_matches(b.t, TSP_NUM) {
        return val_num(a) == val_num(b) && val_den(a) == val_den(b);
    }
    if a.t != b.t {
        return false;
    }
    match (&a.v, &b.v) {
        (ValUnion::S(sa), ValUnion::S(sb)) => sa == sb,
        (ValUnion::N { num: na, den: da }, ValUnion::N { num: nb, den: db }) => na == nb && da == db,
        (ValUnion::P { car: aca, cdr: acd }, ValUnion::P { car: bca, cdr: bcd }) => {
            vals_eq(aca, bca) && vals_eq(acd, bcd)
        }
        (
            ValUnion::F { args: aa, body: ab, .. },
            ValUnion::F { args: ba, body: bb, .. },
        ) => vals_eq(aa, ba) && vals_eq(ab, bb),
        (ValUnion::Pr { name: na, .. }, ValUnion::Pr { name: nb, .. }) => na == nb,
        (ValUnion::R(ra), ValUnion::R(rb)) => {
            if ra.items.len() != rb.items.len() {
                return false;
            }
            ra.items.iter().all(|entry| {
                rb.items
                    .iter()
                    .find(|other| other.key == entry.key)
                    .map(|other| vals_eq(&entry.val, &other.val))
                    .unwrap_or(false)
            })
        }
        _ => true,
    }
}

pub fn esc_char(c: char) -> char {
    match c {
        'n' => '\n',
        'r' => '\r',
        't' => '\t',
        '\n' => ' ',
        '\\' | '"' => c,
        _ => c,
    }
}

pub fn read_sign(st: &mut Tsp) -> i32 {
    match fget(st) {
        Some('-') => {
            finc(st);
            -1
        }
        Some('+') => {
            finc(st);
            1
        }
        _ => 1,
    }
}

pub fn tisp_print(f: &mut std::fs::File, v: &Val) {
    let _ = f.write_all(render_val(v).as_bytes());
}

pub fn eval_proc(st: &mut Tsp, env: &mut Rec, f: Val, args: Val) -> Option<Val> {
    eval_proc_internal(st, env, f, args)
}

pub fn tisp_eval(st: &mut Tsp, v: Val) -> Option<Val> {
    let mut env = std::mem::replace(&mut st.env, rec_new(1, None));
    let ret = eval_with_env(st, &mut env, v);
    st.env = env;
    ret
}

pub fn mk_func(t: TspType, name: &str, args: Val, body: Val, env: Rec) -> Option<Val> {
    let _ = env;
    Some(Val {
        t,
        v: ValUnion::F {
            name: name.to_string(),
            args: Box::new(args),
            body: Box::new(body),
            env: rec_new(1, None),
        },
    })
}

pub fn rec_grow(rec: &mut Rec) {
    rec.cap = (rec.cap.max(1) as usize * TSP_REC_FACTOR) as i32;
}

fn register_prim(st: &mut Tsp, name: &str) {
    if let Some(v) = mk_prim(TspType::TspPrim, dummy_prim, name) {
        tisp_env_add(st, name, v);
    }
}

fn register_form(st: &mut Tsp, name: &str) {
    if let Some(v) = mk_prim(TspType::TspForm, dummy_prim, name) {
        tisp_env_add(st, name, v);
    }
}

fn eval_with_env(st: &mut Tsp, env: &mut Rec, v: Val) -> Option<Val> {
    match v.t {
        TspType::TspSym => rec_get(env, val_str(&v))
            .or_else(|| rec_get(&st.env, val_str(&v)))
            .or_else(|| tsp_warnf(format!("could not find symbol '{}'", val_str(&v)))),
        TspType::TspPair => {
            let f = eval_with_env(st, env, pair_car(&v).map(clone_val)?)?;
            eval_proc_internal(st, env, f, pair_cdr(&v).map(clone_val)?)
        }
        _ => Some(v),
    }
}

fn eval_proc_internal(st: &mut Tsp, env: &mut Rec, f: Val, args: Val) -> Option<Val> {
    match f.t {
        TspType::TspPrim => {
            let evaled = tisp_eval_list(st, env, args)?;
            dispatch_builtin(st, env, prim_name(&f), evaled, false)
        }
        TspType::TspForm => {
            if prim_name(&f) == "do" {
                tisp_eval_body(st, env, args)
            } else if prim_name(&f) == "Rec" {
                mk_rec(st, clone_rec(env), args)
            } else {
                dispatch_builtin(st, env, prim_name(&f), args, true)
            }
        }
        TspType::TspFunc | TspType::TspMacro => {
            let call_args = if f.t == TspType::TspFunc {
                tisp_eval_list(st, env, args)?
            } else {
                args
            };
            let params = func_args(&f)?;
            if let Err(err) = arg_num(&call_args, if func_name(&f).is_empty() { "anon" } else { func_name(&f) }, tsp_lstlen(&params)) {
                return err;
            }
            let mut captured_env = clone_rec(env);
            let mut fenv = rec_extend(&mut captured_env, params, call_args);
            if !func_name(&f).is_empty() {
                rec_add(&mut fenv, func_name(&f), clone_val(&f));
            }
            let ret = tisp_eval_body(st, &mut fenv, func_body(&f)?)?;
            if f.t == TspType::TspMacro {
                eval_with_env(st, env, ret)
            } else {
                Some(ret)
            }
        }
        TspType::TspRec => {
            let evaled = tisp_eval_list(st, env, args)?;
            if let Err(err) = arg_num(&evaled, "record", 1) {
                return err;
            }
            let key = pair_car(&evaled)?;
            if let Err(err) = arg_type(key, "record", TspType::TspSym as u32) {
                return err;
            }
            let rec = value_as_rec(&f)?;
            rec_get(rec, val_str(key))
                .or_else(|| rec_get(rec, "else"))
                .or_else(|| tsp_warnf(format!("could not find element '{}' in record", val_str(key))))
        }
        _ => tsp_warnf(format!(
            "attempt to evaluate non procedural type {}",
            tsp_type_str(f.t)
        )),
    }
}

fn dispatch_builtin(st: &mut Tsp, env: &mut Rec, name: &str, args: Val, is_form: bool) -> Option<Val> {
    match (name, is_form) {
        ("car", false) => builtin_car(st, env, args),
        ("cdr", false) => builtin_cdr(st, env, args),
        ("cons", false) => builtin_cons(st, env, args),
        ("quote", true) => builtin_quote(st, env, args),
        ("eval", false) => builtin_eval(st, env, args),
        ("=", false) => builtin_eq(st, env, args),
        ("cond", true) => builtin_cond(st, env, args),
        ("typeof", false) => builtin_typeof(st, env, args),
        ("procprops", false) => builtin_procprops(st, env, args),
        ("Func", true) => builtin_func(st, env, args, false),
        ("Macro", true) => builtin_func(st, env, args, true),
        ("error", false) => builtin_error(st, env, args),
        ("recmerge", false) => builtin_recmerge(st, env, args),
        ("records", false) => builtin_records(st, env, args),
        ("def", true) => builtin_def(st, env, args),
        ("undefine!", true) => builtin_undefine(st, env, args),
        ("defined?", true) => builtin_definedp(st, env, args),
        ("Int", false) => builtin_round(st, args, "Int"),
        ("Dec", false) => builtin_round(st, args, "Dec"),
        ("round", false) => builtin_round(st, args, "round"),
        ("floor", false) => builtin_round(st, args, "floor"),
        ("ceil", false) => builtin_round(st, args, "ceil"),
        ("numerator", false) => builtin_numerator(st, args),
        ("denominator", false) => builtin_denominator(st, args),
        ("+", false) => builtin_add(st, args),
        ("-", false) => builtin_sub(st, args),
        ("*", false) => builtin_mul(st, args),
        ("/", false) => builtin_div(st, args),
        ("mod", false) => builtin_mod(st, args),
        ("^", false) => builtin_pow(st, args),
        ("<", false) => builtin_compare(st, args, "<"),
        (">", false) => builtin_compare(st, args, ">"),
        ("<=", false) => builtin_compare(st, args, "<="),
        (">=", false) => builtin_compare(st, args, ">="),
        ("sin", false) | ("cos", false) | ("tan", false) | ("sinh", false) | ("cosh", false)
        | ("tanh", false) | ("arcsin", false) | ("arccos", false) | ("arctan", false)
        | ("arcsinh", false) | ("arccosh", false) | ("arctanh", false) | ("exp", false)
        | ("log", false) => builtin_trig(st, args, name),
        ("write", false) => builtin_write(st, args),
        ("read", false) => builtin_read(st, args),
        ("parse", false) => builtin_parse(st, args),
        ("load", false) => builtin_load(st, env, args),
        ("cd!", false) => builtin_cd(st, args),
        ("pwd", false) => builtin_pwd(st, args),
        ("exit!", false) => builtin_exit(st, args),
        ("now", false) => builtin_now(st, args),
        ("time", true) => builtin_time(st, env, args),
        ("Str", false) => builtin_val_string(st, args, false),
        ("Sym", false) => builtin_val_string(st, args, true),
        ("strlen", false) => builtin_strlen(st, args),
        ("strformat", true) => builtin_strformat(st, env, args),
        _ => tsp_warnf(format!("could not dispatch builtin '{name}'")),
    }
}

fn builtin_car(_: &mut Tsp, _: &mut Rec, args: Val) -> Option<Val> {
    arg_num(&args, "car", 1).ok()?;
    let list = pair_car(&args)?;
    arg_type(list, "car", TspType::TspPair as u32).ok()?;
    Some(clone_val(pair_car(list)?))
}

fn builtin_cdr(_: &mut Tsp, _: &mut Rec, args: Val) -> Option<Val> {
    arg_num(&args, "cdr", 1).ok()?;
    let list = pair_car(&args)?;
    arg_type(list, "cdr", TspType::TspPair as u32).ok()?;
    Some(clone_val(pair_cdr(list)?))
}

fn builtin_cons(_: &mut Tsp, _: &mut Rec, args: Val) -> Option<Val> {
    arg_num(&args, "cons", 2).ok()?;
    mk_pair(clone_val(pair_car(&args)?), clone_val(pair_car(pair_cdr(&args)?)?))
}

fn builtin_quote(_: &mut Tsp, _: &mut Rec, args: Val) -> Option<Val> {
    arg_num(&args, "quote", 1).ok()?;
    Some(clone_val(pair_car(&args)?))
}

fn builtin_eval(st: &mut Tsp, _: &mut Rec, args: Val) -> Option<Val> {
    arg_num(&args, "eval", 1).ok()?;
    tisp_eval(st, clone_val(pair_car(&args)?)).or_else(|| Some(clone_val(&st.none)))
}

fn builtin_eq(st: &mut Tsp, _: &mut Rec, args: Val) -> Option<Val> {
    if args.t == TspType::TspNil {
        return Some(clone_val(&st.t));
    }
    let values = list_to_vec(&args);
    for pair in values.windows(2) {
        if !vals_eq(&pair[0], &pair[1]) {
            return Some(clone_val(&st.nil));
        }
    }
    Some(clone_val(&st.t))
}

fn builtin_cond(st: &mut Tsp, env: &mut Rec, args: Val) -> Option<Val> {
    let mut cur = args;
    while cur.t == TspType::TspPair {
        let clause = pair_car(&cur).map(clone_val)?;
        let cond_expr = pair_car(&clause).map(clone_val)?;
        let cond = eval_with_env(st, env, cond_expr)?;
        if cond.t != TspType::TspNil {
            return tisp_eval_body(st, env, pair_cdr(&clause).map(clone_val)?);
        }
        cur = pair_cdr(&cur).map(clone_val)?;
    }
    Some(clone_val(&st.none))
}

fn builtin_typeof(st: &mut Tsp, _: &mut Rec, args: Val) -> Option<Val> {
    arg_num(&args, "typeof", 1).ok()?;
    mk_str(st, tsp_type_str(pair_car(&args)?.t))
}

fn builtin_procprops(st: &mut Tsp, _: &mut Rec, args: Val) -> Option<Val> {
    arg_num(&args, "procprops", 1).ok()?;
    let proc = pair_car(&args)?;
    let mut ret = rec_new(6, None);
    match proc.t {
        TspType::TspForm | TspType::TspPrim => {
            rec_add(
                &mut ret,
                "name",
                mk_sym(st, prim_name(proc)).unwrap_or_else(|| clone_val(&st.none)),
            );
        }
        TspType::TspFunc | TspType::TspMacro => {
            rec_add(
                &mut ret,
                "name",
                mk_sym(
                    st,
                    if func_name(proc).is_empty() {
                        "anon"
                    } else {
                        func_name(proc)
                    },
                )
                .unwrap_or_else(|| clone_val(&st.none)),
            );
            rec_add(&mut ret, "args", func_args(proc)?);
            rec_add(&mut ret, "body", func_body(proc)?);
        }
        _ => {
            return tsp_warnf(format!(
                "procprops: expected Proc, received '{}'",
                tsp_type_str(proc.t)
            ))
        }
    }
    Some(Val {
        t: TspType::TspRec,
        v: ValUnion::R(ret),
    })
}

fn builtin_func(st: &mut Tsp, env: &mut Rec, args: Val, is_macro: bool) -> Option<Val> {
    arg_min(&args, if is_macro { "Macro" } else { "Func" }, 1).ok()?;
    let (params, body) = if pair_cdr(&args)?.t == TspType::TspNil {
        (
            mk_pair(
                mk_sym(st, "it").unwrap_or_else(|| clone_val(&st.none)),
                clone_val(&st.nil),
            )?,
            args,
        )
    } else {
        (clone_val(pair_car(&args)?), clone_val(pair_cdr(&args)?))
    };
    mk_func(
        if is_macro {
            TspType::TspMacro
        } else {
            TspType::TspFunc
        },
        "",
        params,
        body,
        clone_rec(env),
    )
}

fn builtin_error(_: &mut Tsp, _: &mut Rec, args: Val) -> Option<Val> {
    arg_min(&args, "error", 2).ok()?;
    let head = pair_car(&args)?;
    arg_type(head, "error", TspType::TspSym as u32).ok()?;
    let mut msg = format!("{}: ", val_str(head));
    let mut cur = pair_cdr(&args).map(clone_val)?;
    while cur.t == TspType::TspPair {
        msg.push_str(&render_val(pair_car(&cur)?));
        cur = pair_cdr(&cur).map(clone_val)?;
    }
    tsp_warnf(msg)
}

fn builtin_recmerge(_: &mut Tsp, _: &mut Rec, args: Val) -> Option<Val> {
    arg_num(&args, "recmerge", 2).ok()?;
    let left = pair_car(&args)?;
    let right = pair_car(pair_cdr(&args)?)?;
    arg_type(left, "recmerge", TspType::TspRec as u32).ok()?;
    arg_type(right, "recmerge", TspType::TspRec as u32).ok()?;
    let left_rec = value_as_rec(left)?;
    let right_rec = value_as_rec(right)?;
    let mut ret = rec_new(right_rec.items.len() * TSP_REC_FACTOR, Some(Box::new(clone_rec(left_rec))));
    let mut cur = Some(right_rec);
    while let Some(rec) = cur {
        for entry in &rec.items {
            rec_add(&mut ret, &entry.key, clone_val(&entry.val));
        }
        cur = rec.next.as_deref();
    }
    Some(Val {
        t: TspType::TspRec,
        v: ValUnion::R(ret),
    })
}

fn builtin_records(st: &mut Tsp, _: &mut Rec, args: Val) -> Option<Val> {
    arg_num(&args, "records", 1).ok()?;
    let rec_val = pair_car(&args)?;
    arg_type(rec_val, "records", TspType::TspRec as u32).ok()?;
    let mut out = Vec::new();
    let mut cur = value_as_rec(rec_val);
    while let Some(rec) = cur {
        for entry in &rec.items {
            out.push(mk_pair(
                mk_sym(st, &entry.key).unwrap_or_else(|| clone_val(&st.none)),
                clone_val(&entry.val),
            )?);
        }
        cur = rec.next.as_deref();
    }
    out.reverse();
    mk_list(st, out.len() as i32, out)
}

fn builtin_def(st: &mut Tsp, env: &mut Rec, args: Val) -> Option<Val> {
    arg_min(&args, "def", 1).ok()?;
    let head = pair_car(&args).map(clone_val)?;
    let (sym, mut val) = if head.t == TspType::TspPair {
        let sym = pair_car(&head).map(clone_val)?;
        if sym.t != TspType::TspSym {
            return tsp_warnf(format!(
                "def: expected symbol for function name, received '{}'",
                tsp_type_str(sym.t)
            ));
        }
        (
            sym,
            mk_func(
                TspType::TspFunc,
                val_str(&pair_car(&head).map(clone_val)?),
                pair_cdr(&head).map(clone_val)?,
                pair_cdr(&args).map(clone_val)?,
                clone_rec(env),
            )?,
        )
    } else if head.t == TspType::TspSym {
        let val = if pair_cdr(&args)?.t == TspType::TspNil {
            clone_val(&head)
        } else {
            eval_with_env(
                st,
                env,
                clone_val(pair_car(pair_cdr(&args)?)?),
            )?
        };
        (head, val)
    } else {
        return tsp_warn("def: incorrect format, no variable name found");
    };
    if matches!(val.t, TspType::TspFunc | TspType::TspMacro) && func_name(&val).is_empty() {
        if let ValUnion::F { name, .. } = &mut val.v {
            *name = val_str(&sym).to_string();
        }
    }
    rec_add(env, val_str(&sym), val);
    Some(clone_val(&st.none))
}

fn builtin_undefine(st: &mut Tsp, env: &mut Rec, args: Val) -> Option<Val> {
    arg_min(&args, "undefine!", 1).ok()?;
    let sym = pair_car(&args)?;
    arg_type(sym, "undefine!", TspType::TspSym as u32).ok()?;
    let key = val_str(sym);
    fn remove_key(rec: &mut Rec, key: &str) -> bool {
        if let Some(pos) = rec.items.iter().position(|entry| entry.key == key) {
            rec.items.remove(pos);
            rec.size = rec.items.len() as i32;
            return true;
        }
        if let Some(next) = rec.next.as_mut() {
            return remove_key(next, key);
        }
        false
    }
    if remove_key(env, key) || remove_key(&mut st.env, key) {
        Some(clone_val(&st.none))
    } else {
        tsp_warnf(format!("undefine!: could not find symbol {key} to undefine"))
    }
}

fn builtin_definedp(st: &mut Tsp, env: &mut Rec, args: Val) -> Option<Val> {
    arg_min(&args, "defined?", 1).ok()?;
    let sym = pair_car(&args)?;
    arg_type(sym, "defined?", TspType::TspSym as u32).ok()?;
    let key = val_str(sym);
    Some(mk_bool(
        st,
        rec_get(env, key).is_some() || rec_get(&st.env, key).is_some(),
    ))
}

fn create_int(num: f64, _: f64) -> Val {
    mk_int(num as i32)
}

fn create_dec(num: f64, _: f64) -> Val {
    mk_dec(num).unwrap_or_else(|| mk_int(0))
}

fn create_rat(num: f64, den: f64) -> Val {
    mk_rat(num as i32, den as i32).unwrap_or_else(|| mk_int(0))
}

fn mk_num_fn(a: TspType, b: TspType, force: i32) -> fn(f64, f64) -> Val {
    if force == 1 {
        return create_rat;
    }
    if force == 2 {
        return create_dec;
    }
    if a == TspType::TspDec || b == TspType::TspDec {
        create_dec
    } else if a == TspType::TspRatio || b == TspType::TspRatio {
        create_rat
    } else {
        create_int
    }
}

fn builtin_round(st: &mut Tsp, args: Val, name: &str) -> Option<Val> {
    arg_num(&args, name, 1).ok()?;
    let n = pair_car(&args)?;
    arg_type(n, name, TSP_NUM).ok()?;
    let value = val_num(n) / val_den(n);
    let out = match name {
        "Int" => create_rat(value, 1.0),
        "Dec" => create_dec(value, 1.0),
        "round" => mk_num_fn(n.t, n.t, 0)(value.round(), 1.0),
        "floor" => mk_num_fn(n.t, n.t, 0)(value.floor(), 1.0),
        "ceil" => mk_num_fn(n.t, n.t, 0)(value.ceil(), 1.0),
        _ => clone_val(&st.none),
    };
    Some(out)
}

fn builtin_add(_: &mut Tsp, args: Val) -> Option<Val> {
    arg_num(&args, "+", 2).ok()?;
    let a = pair_car(&args)?;
    let b = pair_car(pair_cdr(&args)?)?;
    arg_type(a, "+", TSP_NUM).ok()?;
    arg_type(b, "+", TSP_NUM).ok()?;
    if a.t == TspType::TspDec || b.t == TspType::TspDec {
        return mk_dec((val_num(a) / val_den(a)) + (val_num(b) / val_den(b)));
    }
    Some(mk_num_fn(a.t, b.t, 0)(
        val_num(a) * val_den(b) + val_den(a) * val_num(b),
        val_den(a) * val_den(b),
    ))
}

fn builtin_sub(_: &mut Tsp, args: Val) -> Option<Val> {
    let len = tsp_lstlen(&args);
    if len != 1 && len != 2 {
        return tsp_warnf(format!("-: expected 1 or 2 arguments, recieved {len}"));
    }
    let mut a = clone_val(pair_car(&args)?);
    arg_type(&a, "-", TSP_NUM).ok()?;
    let b = if len == 1 {
        let b = clone_val(&a);
        a = mk_int(0);
        b
    } else {
        let b = clone_val(pair_car(pair_cdr(&args)?)?);
        arg_type(&b, "-", TSP_NUM).ok()?;
        b
    };
    if a.t == TspType::TspDec || b.t == TspType::TspDec {
        return mk_dec((val_num(&a) / val_den(&a)) - (val_num(&b) / val_den(&b)));
    }
    Some(mk_num_fn(a.t, b.t, 0)(
        val_num(&a) * val_den(&b) - val_den(&a) * val_num(&b),
        val_den(&a) * val_den(&b),
    ))
}

fn builtin_mul(_: &mut Tsp, args: Val) -> Option<Val> {
    arg_num(&args, "*", 2).ok()?;
    let a = pair_car(&args)?;
    let b = pair_car(pair_cdr(&args)?)?;
    arg_type(a, "*", TSP_NUM).ok()?;
    arg_type(b, "*", TSP_NUM).ok()?;
    if a.t == TspType::TspDec || b.t == TspType::TspDec {
        return mk_dec((val_num(a) / val_den(a)) * (val_num(b) / val_den(b)));
    }
    Some(mk_num_fn(a.t, b.t, 0)(val_num(a) * val_num(b), val_den(a) * val_den(b)))
}

fn builtin_div(_: &mut Tsp, args: Val) -> Option<Val> {
    let len = tsp_lstlen(&args);
    if len != 1 && len != 2 {
        return tsp_warnf(format!("/: expected 1 or 2 arguments, recieved {len}"));
    }
    let mut a = clone_val(pair_car(&args)?);
    arg_type(&a, "/", TSP_NUM).ok()?;
    let b = if len == 1 {
        let b = clone_val(&a);
        a = mk_int(1);
        b
    } else {
        let b = clone_val(pair_car(pair_cdr(&args)?)?);
        arg_type(&b, "/", TSP_NUM).ok()?;
        b
    };
    if a.t == TspType::TspDec || b.t == TspType::TspDec {
        return mk_dec((val_num(&a) / val_den(&a)) / (val_num(&b) / val_den(&b)));
    }
    Some(mk_num_fn(a.t, b.t, 1)(
        val_num(&a) * val_den(&b),
        val_den(&a) * val_num(&b),
    ))
}

fn builtin_mod(_: &mut Tsp, args: Val) -> Option<Val> {
    arg_num(&args, "mod", 2).ok()?;
    let a = pair_car(&args)?;
    let b = pair_car(pair_cdr(&args)?)?;
    arg_type(a, "mod", TspType::TspInt as u32).ok()?;
    arg_type(b, "mod", TspType::TspInt as u32).ok()?;
    if val_num(b) == 0.0 {
        return tsp_warn("division by zero");
    }
    Some(mk_int((val_num(a) as i32) % (val_num(b).abs() as i32)))
}

fn builtin_pow(st: &mut Tsp, args: Val) -> Option<Val> {
    arg_num(&args, "pow", 2).ok()?;
    let b = pair_car(&args)?;
    let p = pair_car(pair_cdr(&args)?)?;
    arg_type(b, "pow", TSP_EXPR).ok()?;
    arg_type(p, "pow", TSP_EXPR).ok()?;
    let expo = val_num(p) / val_den(p);
    let bnum = val_num(b).powf(expo);
    let bden = val_den(b).powf(expo);
    if ((bnum == bnum.trunc()) && (bden == bden.trunc())) || b.t == TspType::TspDec || p.t == TspType::TspDec {
        return Some(mk_num_fn(b.t, p.t, 0)(bnum, bden));
    }
    let pow_sym = mk_sym(st, "^").unwrap_or_else(|| clone_val(&st.none));
    mk_list(st, 3, vec![pow_sym, clone_val(b), clone_val(p)])
}

fn builtin_compare(st: &mut Tsp, args: Val, op: &str) -> Option<Val> {
    if tsp_lstlen(&args) != 2 {
        return Some(clone_val(&st.t));
    }
    let a = pair_car(&args)?;
    let b = pair_car(pair_cdr(&args)?)?;
    arg_type(a, op, TSP_NUM).ok()?;
    arg_type(b, op, TSP_NUM).ok()?;
    let left = val_num(a) * val_den(b);
    let right = val_num(b) * val_den(a);
    Some(match op {
        "<" => mk_bool(st, left < right),
        ">" => mk_bool(st, left > right),
        "<=" => mk_bool(st, left <= right),
        ">=" => mk_bool(st, left >= right),
        _ => clone_val(&st.nil),
    })
}

fn builtin_trig(st: &mut Tsp, args: Val, name: &str) -> Option<Val> {
    arg_num(&args, name, 1).ok()?;
    let arg = pair_car(&args)?;
    arg_type(arg, name, TSP_EXPR).ok()?;
    if arg.t == TspType::TspDec {
        let n = val_num(arg);
        let out = match name {
            "sin" => n.sin(),
            "cos" => n.cos(),
            "tan" => n.tan(),
            "sinh" => n.sinh(),
            "cosh" => n.cosh(),
            "tanh" => n.tanh(),
            "arcsin" => n.asin(),
            "arccos" => n.acos(),
            "arctan" => n.atan(),
            "arcsinh" => n.asinh(),
            "arccosh" => n.acosh(),
            "arctanh" => n.atanh(),
            "exp" => n.exp(),
            "log" => n.ln(),
            _ => 0.0,
        };
        return mk_dec(out);
    }
    let trig_sym = mk_sym(st, name).unwrap_or_else(|| clone_val(&st.none));
    mk_list(st, 2, vec![trig_sym, clone_val(arg)])
}

fn builtin_numerator(_: &mut Tsp, args: Val) -> Option<Val> {
    arg_num(&args, "numerator", 1).ok()?;
    let arg = pair_car(&args)?;
    arg_type(arg, "numerator", TspType::TspInt as u32 | TspType::TspRatio as u32).ok()?;
    Some(mk_int(val_num(arg) as i32))
}

fn builtin_denominator(_: &mut Tsp, args: Val) -> Option<Val> {
    arg_num(&args, "denominator", 1).ok()?;
    let arg = pair_car(&args)?;
    arg_type(arg, "denominator", TspType::TspInt as u32 | TspType::TspRatio as u32).ok()?;
    Some(mk_int(val_den(arg) as i32))
}

pub fn count_parens(s: &str, len: i32) -> i32 {
    let mut pcount = 0;
    let mut bcount = 0;
    let mut ccount = 0;
    for ch in s.chars().take(len as usize) {
        match ch {
            '(' => pcount += 1,
            '[' => bcount += 1,
            '{' => ccount += 1,
            ')' => pcount -= 1,
            ']' => bcount -= 1,
            '}' => ccount -= 1,
            _ => {}
        }
    }
    if pcount != 0 {
        pcount
    } else if bcount != 0 {
        bcount
    } else {
        ccount
    }
}

pub fn read_file(fname: &str) -> String {
    if fname.is_empty() {
        let mut input = String::new();
        let _ = std::io::stdin().read_to_string(&mut input);
        input
    } else {
        std::fs::read_to_string(fname).unwrap_or_default()
    }
}

fn builtin_write(st: &mut Tsp, args: Val) -> Option<Val> {
    arg_min(&args, "write", 2).ok()?;
    let target = pair_car(&args)?;
    let append = pair_car(pair_cdr(&args)?)?.t != TspType::TspNil;
    let rendered = list_to_vec(pair_cdr(pair_cdr(&args)?)?)
        .into_iter()
        .map(|v| render_val(&v))
        .collect::<Vec<_>>()
        .join("");
    if target.t == TspType::TspSym {
        match val_str(target) {
            "stdout" => {
                let _ = std::io::stdout().write_all(rendered.as_bytes());
                let _ = std::io::stdout().flush();
            }
            "stderr" => {
                let _ = std::io::stderr().write_all(rendered.as_bytes());
                let _ = std::io::stderr().flush();
            }
            _ => return tsp_warn("write: expected file name as string, or symbol stdout/stderr"),
        }
    } else if target.t == TspType::TspStr {
        let mut options = std::fs::OpenOptions::new();
        options.create(true).write(true);
        if append {
            options.append(true);
        } else {
            options.truncate(true);
        }
        match options.open(val_str(target)) {
            Ok(mut file) => {
                let _ = file.write_all(rendered.as_bytes());
            }
            Err(_) => {
                return tsp_warnf(format!("write: could not load file '{}'", val_str(target)));
            }
        }
    } else {
        return tsp_warnf(format!(
            "write: expected file name as string, received {}",
            tsp_type_str(target.t)
        ));
    }
    Some(clone_val(&st.none))
}

fn builtin_read(st: &mut Tsp, args: Val) -> Option<Val> {
    arg_max(&args, "read", 1).ok()?;
    let fname = if tsp_lstlen(&args) == 1 {
        let arg = pair_car(&args)?;
        arg_type(arg, "read", TspType::TspStr as u32).ok()?;
        val_str(arg).to_string()
    } else {
        String::new()
    };
    let file = read_file(&fname);
    if file.is_empty() {
        Some(clone_val(&st.nil))
    } else {
        mk_str(st, &file)
    }
}

fn builtin_parse(st: &mut Tsp, args: Val) -> Option<Val> {
    arg_num(&args, "parse", 1).ok()?;
    let expr = pair_car(&args)?;
    if expr.t == TspType::TspNil {
        return mk_sym(st, "quit");
    }
    arg_type(expr, "parse", TspType::TspStr as u32).ok()?;
    let old_file = st.file.clone();
    let old_filec = st.filec;
    st.file = val_str(expr).to_string();
    st.filec = 0;
    let mut forms = Vec::new();
    while fget(st).is_some() {
        if let Some(form) = tisp_read_line(st, 0) {
            forms.push(form);
        } else {
            break;
        }
    }
    st.file = old_file;
    st.filec = old_filec;
    if forms.len() == 1 {
        Some(forms.remove(0))
    } else {
        let mut values = vec![mk_sym(st, "do").unwrap_or_else(|| clone_val(&st.none))];
        values.extend(forms);
        mk_list(st, values.len() as i32, values)
    }
}

fn builtin_load(st: &mut Tsp, env: &mut Rec, args: Val) -> Option<Val> {
    arg_num(&args, "load", 1).ok()?;
    let tib = pair_car(&args)?;
    arg_type(tib, "load", TspType::TspStr as u32).ok()?;
    let name = val_str(tib);
    let mut candidates = vec![
        format!("/usr/local/lib/tisp/pkgs/{name}.tsp"),
        format!("/usr/lib/tisp/pkgs/{name}.tsp"),
        format!("./{name}.tsp"),
        format!("c_src/tib/{name}.tsp"),
    ];
    if name == "tibs" {
        tisp_env_lib(st, "tibs");
        return Some(clone_val(&st.none));
    }
    if let Some(path) = candidates.drain(..).find(|p| Path::new(p).exists()) {
        let file = std::fs::read_to_string(&path).unwrap_or_default();
        let file_str = mk_str(st, &file).unwrap_or_else(|| clone_val(&st.none));
        let parse_arg = mk_pair(file_str, clone_val(&st.nil))?;
        let body = builtin_parse(st, parse_arg)?;
        let _ = tisp_eval_body(st, env, body);
        return Some(clone_val(&st.none));
    }
    tsp_warnf(format!("load: could not load '{name}'"))
}

fn builtin_cd(st: &mut Tsp, args: Val) -> Option<Val> {
    arg_num(&args, "cd!", 1).ok()?;
    let dir = pair_car(&args)?;
    if !(dir.t == TspType::TspStr || dir.t == TspType::TspSym) {
        return tsp_warnf(format!(
            "cd!: expected string or symbol, received {}",
            tsp_type_str(dir.t)
        ));
    }
    match std::env::set_current_dir(val_str(dir)) {
        Ok(_) => Some(clone_val(&st.none)),
        Err(_) => tsp_warn("cd!: could not change directory"),
    }
}

fn builtin_pwd(st: &mut Tsp, args: Val) -> Option<Val> {
    arg_num(&args, "pwd", 0).ok()?;
    match std::env::current_dir() {
        Ok(path) => mk_str(st, &path.to_string_lossy()),
        Err(_) => tsp_warn("pwd: could not get current directory"),
    }
}

fn builtin_exit(st: &mut Tsp, args: Val) -> Option<Val> {
    arg_num(&args, "exit!", 1).ok()?;
    let code = pair_car(&args)?;
    arg_type(code, "exit!", TspType::TspInt as u32).ok()?;
    Some(mk_int(val_num(code) as i32)).or_else(|| Some(clone_val(&st.none)))
}

fn builtin_now(_: &mut Tsp, args: Val) -> Option<Val> {
    arg_num(&args, "now", 0).ok()?;
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i32)
        .unwrap_or_default();
    Some(mk_int(secs))
}

fn builtin_time(st: &mut Tsp, env: &mut Rec, args: Val) -> Option<Val> {
    arg_num(&args, "time", 1).ok()?;
    let start = Instant::now();
    let v = eval_with_env(st, env, clone_val(pair_car(&args)?))?;
    let elapsed_ms = start.elapsed().as_secs_f64() * 100.0;
    let _ = v;
    mk_dec(elapsed_ms)
}

fn builtin_val_string(st: &mut Tsp, args: Val, to_sym: bool) -> Option<Val> {
    arg_min(&args, if to_sym { "Sym" } else { "Str" }, 1).ok()?;
    let mut ret = String::new();
    let mut cur = args;
    while cur.t == TspType::TspPair {
        let v = pair_car(&cur)?;
        match v.t {
            TspType::TspNone => {}
            TspType::TspNil => ret.push_str("Nil"),
            TspType::TspInt | TspType::TspRatio | TspType::TspStr | TspType::TspSym => {
                ret.push_str(&render_val(v));
            }
            TspType::TspDec => {
                let s = format_dec(val_num(v));
                ret.push_str(&s);
            }
            _ => {
                return tsp_warnf(format!(
                    "could not convert type {} into string",
                    tsp_type_str(v.t)
                ))
            }
        }
        cur = pair_cdr(&cur).map(clone_val)?;
    }
    if to_sym {
        mk_sym(st, &ret)
    } else {
        mk_str(st, &ret)
    }
}

fn builtin_strlen(_: &mut Tsp, args: Val) -> Option<Val> {
    arg_min(&args, "strlen", 1).ok()?;
    let arg = pair_car(&args)?;
    arg_type(arg, "strlen", TspType::TspStr as u32 | TspType::TspSym as u32).ok()?;
    Some(mk_int(val_str(arg).len() as i32))
}

fn builtin_strformat(st: &mut Tsp, env: &mut Rec, args: Val) -> Option<Val> {
    arg_num(&args, "strformat", 1).ok()?;
    let arg = pair_car(&args)?;
    arg_type(arg, "strformat", TspType::TspStr as u32).ok()?;
    let source = val_str(arg).to_string();
    let mut out = String::new();
    let mut chars = source.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '{' && chars.peek() != Some(&'{') {
            let mut expr = String::new();
            let mut depth = 1i32;
            for next in chars.by_ref() {
                if next == '{' {
                    depth += 1;
                } else if next == '}' {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                expr.push(next);
            }
            let old_file = st.file.clone();
            let old_filec = st.filec;
            st.file = expr;
            st.filec = 0;
            let v = read_pair(st, '}')?;
            st.file = old_file;
            st.filec = old_filec;
            let evaled = tisp_eval_list(st, env, v)?;
            let joined = builtin_val_string(st, evaled, false)?;
            out.push_str(val_str(&joined));
        } else {
            if (ch == '{' || ch == '}') && chars.peek() == Some(&ch) {
                let _ = chars.next();
            }
            out.push(ch);
        }
    }
    mk_str(st, &out)
}
