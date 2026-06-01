use crate::tisp::{mk_dec, mk_int, mk_rat, Rec, Tsp, TspType, Val};

pub fn create_int(num: f64, _den: f64) -> Val {
    mk_int(num as i32)
}

pub fn create_dec(num: f64, _den: f64) -> Val {
    mk_dec(num).unwrap_or_else(|| mk_int(0))
}

pub fn create_rat(num: f64, den: f64) -> Val {
    mk_rat(num as i32, den as i32).unwrap_or_else(|| mk_int(0))
}

pub fn mk_num(a: TspType, b: TspType, force: i32) -> fn(f64, f64) -> Val {
    if force == 1 {
        return create_rat;
    }
    if force == 2 {
        return create_dec;
    }
    let a_bits = a as u32;
    let b_bits = b as u32;
    let dec_bit = TspType::TspDec as u32;
    let ratio_bit = TspType::TspRatio as u32;
    if (a_bits & dec_bit != 0) || (b_bits & dec_bit != 0) {
        return create_dec;
    }
    if (a_bits & ratio_bit != 0) || (b_bits & ratio_bit != 0) {
        return create_rat;
    }
    create_int
}

pub fn prim_add(st: &mut Tsp, _vars: &mut Rec, _args: Val) -> Val {
    st.none.clone()
}

pub fn prim_sub(st: &mut Tsp, _vars: &mut Rec, _args: Val) -> Val {
    st.none.clone()
}

pub fn prim_mul(st: &mut Tsp, _vars: &mut Rec, _args: Val) -> Val {
    st.none.clone()
}

pub fn prim_div(st: &mut Tsp, _vars: &mut Rec, _args: Val) -> Val {
    st.none.clone()
}

pub fn prim_mod(st: &mut Tsp, _vars: &mut Rec, _args: Val) -> Val {
    st.none.clone()
}

pub fn prim_pow(st: &mut Tsp, _vars: &mut Rec, _args: Val) -> Val {
    st.none.clone()
}

pub fn prim_denominator(st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    st.none.clone()
}

pub fn tib_env_math(_st: &mut Tsp) {
    // Primitives use a different signature than crate::tisp::Prim, so we don't register them.
}
