use crate::tisp::{
    clone_val, mk_int, mk_val, tsp_lstlen, Rec, Tsp, TspType, Val, ValUnion,
};

pub type MkFn = fn(&mut Tsp, &str) -> Val;

pub fn val_string(st: &mut Tsp, args: Val, mk_fn: MkFn) -> Val {
    let mut ret = String::new();
    let mut cur = &args;
    while let ValUnion::P { car, cdr } = &cur.v {
        let v = car.as_ref();
        match v.t {
            TspType::TspNone => {}
            TspType::TspNil => ret.push_str("Nil"),
            TspType::TspInt => {
                if let ValUnion::N { num, .. } = &v.v {
                    ret.push_str(&format!("{}", *num as i64));
                }
            }
            TspType::TspDec => {
                if let ValUnion::N { num, .. } = &v.v {
                    ret.push_str(&format!("{}", num));
                }
            }
            TspType::TspRatio => {
                if let ValUnion::N { num, den } = &v.v {
                    ret.push_str(&format!("{}/{}", *num as i64, *den as i64));
                }
            }
            TspType::TspStr | TspType::TspSym => {
                if let ValUnion::S(s) = &v.v {
                    ret.push_str(s);
                }
            }
            _ => {
                return mk_val(TspType::TspNone);
            }
        }
        cur = cdr;
    }
    mk_fn(st, &ret)
}

#[allow(non_snake_case)]
pub fn prim_Str(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if tsp_lstlen(&args) < 1 {
        return mk_val(TspType::TspNone);
    }
    let mk_fn: MkFn = |st, s| crate::tisp::mk_str(st, s).unwrap_or_else(|| mk_val(TspType::TspStr));
    val_string(st, args, mk_fn)
}

#[allow(non_snake_case)]
pub fn prim_Sym(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if tsp_lstlen(&args) < 1 {
        return mk_val(TspType::TspNone);
    }
    let mk_fn: MkFn = |st, s| crate::tisp::mk_sym(st, s).unwrap_or_else(|| mk_val(TspType::TspSym));
    val_string(st, args, mk_fn)
}

pub fn prim_strlen(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if tsp_lstlen(&args) < 1 {
        return mk_val(TspType::TspNone);
    }
    if let ValUnion::P { car, .. } = &args.v {
        if matches!(car.t, TspType::TspStr | TspType::TspSym) {
            if let ValUnion::S(s) = &car.v {
                return mk_int(s.len() as i32);
            }
        }
    }
    mk_val(TspType::TspNone)
}

pub fn form_strformat(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    // Simplified: returns the input string unchanged if it's a string
    if let ValUnion::P { car, .. } = &args.v {
        if matches!(car.t, TspType::TspStr) {
            return clone_val(car);
        }
    }
    let _ = st;
    mk_val(TspType::TspNone)
}

pub fn tib_env_string(_st: &mut Tsp) {
    // Primitives registration would happen here in full impl
}
