use crate::tisp::{mk_dec, mk_int, mk_rat, Rec, Tsp, TspType, Val, ValUnion};

#[allow(unused)]
fn empty() -> Val {
    Val { t: TspType::TspNone, v: ValUnion::S(String::new()) }
}

pub fn create_int(num: f64, _den: f64) -> Val {
    mk_int(num as i32)
}

pub fn create_dec(num: f64, _den: f64) -> Val {
    mk_dec(num).unwrap_or_else(empty)
}

pub fn create_rat(num: f64, den: f64) -> Val {
    mk_rat(num as i32, den as i32).unwrap_or_else(empty)
}

pub fn mk_num(a: TspType, b: TspType, force: i32) -> fn(f64, f64) -> Val {
    if force == 1 {
        return create_rat;
    }
    if force == 2 {
        return create_dec;
    }
    if matches!(a, TspType::TspDec) || matches!(b, TspType::TspDec) {
        return create_dec;
    }
    if matches!(a, TspType::TspRatio) || matches!(b, TspType::TspRatio) {
        return create_rat;
    }
    create_int
}

pub fn prim_add(_st: &mut Tsp, _vars: &mut Rec, _args: Val) -> Val {
    empty()
}

pub fn prim_sub(_st: &mut Tsp, _vars: &mut Rec, _args: Val) -> Val {
    empty()
}

pub fn prim_mul(_st: &mut Tsp, _vars: &mut Rec, _args: Val) -> Val {
    empty()
}

pub fn prim_div(_st: &mut Tsp, _vars: &mut Rec, _args: Val) -> Val {
    empty()
}

pub fn prim_mod(_st: &mut Tsp, _vars: &mut Rec, _args: Val) -> Val {
    empty()
}

pub fn prim_pow(_st: &mut Tsp, _vars: &mut Rec, _args: Val) -> Val {
    empty()
}

pub fn prim_denominator(_st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    empty()
}

pub fn tib_env_math(_st: &mut Tsp) {
    // Stub
}
