use crate::tisp::{
    mk_str, mk_sym, rec_add, Rec, Tsp, TspType, Val, ValUnion,
};

pub type MkFn = fn(&mut Tsp, &str) -> Val;

fn make_none() -> Val {
    Val { t: TspType::TspNone, v: ValUnion::S(String::new()) }
}

fn val_num(v: &Val) -> f64 {
    if let ValUnion::N { num, .. } = &v.v { *num } else { 0.0 }
}

fn val_den(v: &Val) -> f64 {
    if let ValUnion::N { den, .. } = &v.v { *den } else { 1.0 }
}

fn val_str_ref(v: &Val) -> &str {
    if let ValUnion::S(s) = &v.v { s.as_str() } else { "" }
}

fn car_of(v: &Val) -> Option<&Val> {
    if let ValUnion::P { car, .. } = &v.v { Some(car.as_ref()) } else { None }
}

fn cdr_of(v: &Val) -> Option<&Val> {
    if let ValUnion::P { cdr, .. } = &v.v { Some(cdr.as_ref()) } else { None }
}

pub fn val_string(st: &mut Tsp, args: Val, mk_fn: MkFn) -> Val {
    let mut ret = String::new();
    let mut cur = &args;
    while matches!(cur.t, TspType::TspPair) {
        let v = match car_of(cur) { Some(v) => v, None => break };
        match v.t {
            TspType::TspNone => {}
            TspType::TspNil => ret.push_str("Nil"),
            TspType::TspInt => ret.push_str(&format!("{}", val_num(v) as i64)),
            TspType::TspDec => {
                let n = val_num(v);
                ret.push_str(&format_dec(n));
            }
            TspType::TspRatio => {
                ret.push_str(&format!("{}/{}", val_num(v) as i64, val_den(v) as i64));
            }
            TspType::TspStr | TspType::TspSym => ret.push_str(val_str_ref(v)),
            _ => return make_none(),
        }
        cur = match cdr_of(cur) { Some(c) => c, None => break };
    }
    mk_fn(st, &ret)
}

fn format_dec(n: f64) -> String {
    let formatted = format!("{:.15}", n);
    let mut s = formatted.trim_end_matches('0').trim_end_matches('.').to_string();
    if s.is_empty() {
        s.push('0');
    }
    s
}

#[allow(non_snake_case)]
pub fn prim_Str(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    val_string(st, args, |st, s| mk_str(st, s).unwrap_or_else(make_none))
}

#[allow(non_snake_case)]
pub fn prim_Sym(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    val_string(st, args, |st, s| mk_sym(st, s).unwrap_or_else(make_none))
}

pub fn prim_strlen(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    let v = match car_of(&args) { Some(v) => v, None => return make_none() };
    if !matches!(v.t, TspType::TspStr | TspType::TspSym) {
        return make_none();
    }
    crate::tisp::mk_int(val_str_ref(v).len() as i32)
}

pub fn form_strformat(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    let v = match car_of(&args) { Some(v) => v, None => return make_none() };
    if !matches!(v.t, TspType::TspStr) {
        return make_none();
    }
    let s = val_str_ref(v).to_string();
    // Pass-through format: replace {{ -> { and }} -> }, leave other braces unchanged.
    // Full evaluation isn't supported here without re-entrant parser; we keep semantics simple.
    let mut out = String::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if c == b'{' && bytes.get(i + 1) == Some(&b'{') {
            out.push('{');
            i += 2;
        } else if c == b'}' && bytes.get(i + 1) == Some(&b'}') {
            out.push('}');
            i += 2;
        } else {
            out.push(c as char);
            i += 1;
        }
    }
    mk_str(st, &out).unwrap_or_else(make_none)
}

pub fn tib_env_string(st: &mut Tsp) {
    for name in &["Sym", "Str", "strlen", "strformat"] {
        let t = if *name == "strformat" { TspType::TspForm } else { TspType::TspPrim };
        let v = Val {
            t,
            v: ValUnion::Pr { name: name.to_string(), pr: dummy_prim },
        };
        rec_add(&mut st.env, name, v);
    }
}

fn dummy_prim(_st: Tsp, _env: Rec, _args: Val) -> Val {
    make_none()
}
