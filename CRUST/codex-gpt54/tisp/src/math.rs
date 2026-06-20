use crate::tisp::{self, Rec, Tsp, TspType, Val};

fn dummy(_: tisp::Tsp, _: tisp::Rec, _: tisp::Val) -> tisp::Val {
    tisp::mk_val(TspType::TspNone)
}

fn fallback(st: &Tsp) -> Val {
    let mut v = tisp::mk_val(TspType::TspNone);
    v.t = st.none.t;
    v
}

fn call(st: &mut Tsp, env: &mut Rec, name: &str, args: Val) -> Val {
    let proc = tisp::mk_prim(TspType::TspPrim, dummy, name).unwrap_or_else(|| fallback(st));
    tisp::eval_proc(st, env, proc, args).unwrap_or_else(|| fallback(st))
}

pub fn create_int(num: f64, den: f64) -> Val {
    let _ = den;
    tisp::mk_int(num as i32)
}
pub fn create_dec(num: f64, den: f64) -> Val {
    let _ = den;
    tisp::mk_dec(num).unwrap_or_else(|| tisp::mk_int(0))
}
pub fn create_rat(num: f64, den: f64) -> Val {
    tisp::mk_rat(num as i32, den as i32).unwrap_or_else(|| tisp::mk_int(0))
}
pub fn mk_num(a: TspType, b: TspType, force: i32) -> fn(f64, f64) -> Val {
    if force == 1 {
        create_rat
    } else if force == 2 || a == TspType::TspDec || b == TspType::TspDec {
        create_dec
    } else if a == TspType::TspRatio || b == TspType::TspRatio {
        create_rat
    } else {
        create_int
    }
}
pub fn prim_add(st: &mut Tsp, vars: &mut Rec, args: Val) -> Val {
    call(st, vars, "+", args)
}
pub fn prim_sub(st: &mut Tsp, vars: &mut Rec, args: Val) -> Val {
    call(st, vars, "-", args)
}
pub fn prim_mul(st: &mut Tsp, vars: &mut Rec, args: Val) -> Val {
    call(st, vars, "*", args)
}
pub fn prim_div(st: &mut Tsp, vars: &mut Rec, args: Val) -> Val {
    call(st, vars, "/", args)
}
pub fn prim_mod(st: &mut Tsp, vars: &mut Rec, args: Val) -> Val {
    call(st, vars, "mod", args)
}
pub fn prim_pow(st: &mut Tsp, vars: &mut Rec, args: Val) -> Val {
    call(st, vars, "^", args)
}
pub fn prim_denominator(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    call(st, env, "denominator", args)
}
pub fn tib_env_math(st: &mut Tsp) {
    tisp::tib_env_math(st);
}
