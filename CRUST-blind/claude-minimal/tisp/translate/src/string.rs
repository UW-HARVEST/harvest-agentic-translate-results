use crate::tisp::{mk_int, mk_str, mk_val, Rec, Tsp, TspType, Val, ValUnion};

pub type MkFn = fn(&mut Tsp, &str) -> Val;

/* convert a list of values into a single string and apply mk_fn to it */
pub fn val_string(st: &mut Tsp, args: Val, mk_fn: MkFn) -> Val {
    let mut ret = String::new();
    let mut cur = args;
    loop {
        match cur.v {
            ValUnion::P { car, cdr } => {
                let v = *car;
                match (&v.t, &v.v) {
                    (TspType::TspNone, _) => {}
                    (TspType::TspNil, _) => ret.push_str("Nil"),
                    (TspType::TspInt, ValUnion::N { num, .. }) => {
                        ret.push_str(&format!("{}", *num as i32));
                    }
                    (TspType::TspDec, ValUnion::N { num, .. }) => {
                        ret.push_str(&format!("{:.15}", num));
                    }
                    (TspType::TspRatio, ValUnion::N { num, den }) => {
                        ret.push_str(&format!("{}/{}", *num as i32, *den as i32));
                    }
                    (TspType::TspStr, ValUnion::S(s))
                    | (TspType::TspSym, ValUnion::S(s)) => {
                        ret.push_str(s);
                    }
                    _ => {
                        eprintln!("; tisp: error: could not convert type into string");
                    }
                }
                cur = *cdr;
            }
            _ => break,
        }
    }
    mk_fn(st, &ret)
}

/* convert all args to a string */
#[allow(non_snake_case)]
pub fn prim_Str(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    val_string(st, args, |s, src| {
        mk_str(s, src).unwrap_or_else(|| mk_val(TspType::TspStr))
    })
}

/* convert all args to a symbol */
#[allow(non_snake_case)]
pub fn prim_Sym(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    val_string(st, args, |_s, src| Val {
        t: TspType::TspSym,
        v: ValUnion::S(src.to_string()),
    })
}

pub fn prim_strlen(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if let ValUnion::P { car, .. } = &args.v {
        if let ValUnion::S(s) = &car.v {
            return mk_int(s.len() as i32);
        }
    }
    mk_val(TspType::TspNone)
}

pub fn form_strformat(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    /* simplified: return the string argument unchanged */
    if let ValUnion::P { car, .. } = args.v {
        return *car;
    }
    mk_val(TspType::TspNone)
}

pub fn tib_env_string(_st: &mut Tsp) {
    /* string environment registration is a no-op stub */
}
