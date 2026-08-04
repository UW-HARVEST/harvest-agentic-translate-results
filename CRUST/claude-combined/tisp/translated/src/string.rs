use crate::tisp::{mk_str, mk_sym, Rec, Tsp, TspType, Val, ValUnion};

pub type MkFn = fn(&mut Tsp, &str) -> Val;

fn val_to_string_segment(v: &Val) -> Option<String> {
    match (&v.t, &v.v) {
        (TspType::TspNone, _) => Some(String::new()),
        (TspType::TspNil, _) => Some("Nil".to_string()),
        (TspType::TspInt, ValUnion::N { num, .. }) => Some(format!("{}", *num as i32)),
        (TspType::TspDec, ValUnion::N { num, .. }) => Some(format!("{}", num)),
        (TspType::TspRatio, ValUnion::N { num, den }) => {
            Some(format!("{}/{}", *num as i32, *den as i32))
        }
        (TspType::TspStr, ValUnion::S(s)) | (TspType::TspSym, ValUnion::S(s)) => Some(s.clone()),
        _ => None,
    }
}

pub fn val_string(st: &mut Tsp, args: Val, mk_fn: MkFn) -> Val {
    let mut ret = String::new();
    let mut cur = args;
    while let (TspType::TspPair, ValUnion::P { car, cdr }) = (&cur.t, cur.v.clone()) {
        if let Some(s) = val_to_string_segment(&car) {
            ret.push_str(&s);
        }
        cur = *cdr;
    }
    mk_fn(st, &ret)
}

#[allow(non_snake_case)]
pub fn prim_Str(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    let mk_fn: MkFn = |s, key| {
        mk_str(s, key).unwrap_or_else(|| s.none.clone())
    };
    val_string(st, args, mk_fn)
}

#[allow(non_snake_case)]
pub fn prim_Sym(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    let mk_fn: MkFn = |s, key| {
        mk_sym(s, key).unwrap_or_else(|| s.none.clone())
    };
    val_string(st, args, mk_fn)
}

pub fn prim_strlen(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if let ValUnion::P { car, .. } = &args.v {
        if let ValUnion::S(s) = &car.v {
            return crate::tisp::mk_int(s.len() as i32);
        }
    }
    st.none.clone()
}

pub fn form_strformat(st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    st.none.clone()
}

pub fn tib_env_string(_st: &mut Tsp) {
    // Primitives use a different signature than crate::tisp::Prim, so we don't register them.
}
