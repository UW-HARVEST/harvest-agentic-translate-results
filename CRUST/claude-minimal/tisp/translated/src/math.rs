use crate::tisp::{mk_prim, tisp_env_add, Rec, Tsp, TspType, Val};
use std::cell::RefCell;
use std::rc::Rc;

pub fn create_int(num: f64, _den: f64) -> Val {
    crate::tisp::mk_int(num as i32)
}

pub fn create_dec(num: f64, _den: f64) -> Val {
    crate::tisp::mk_dec(num).unwrap()
}

pub fn create_rat(num: f64, den: f64) -> Val {
    crate::tisp::mk_rat(num as i32, den as i32).unwrap_or_else(|| crate::tisp::mk_int(0))
}

pub fn mk_num(a: TspType, b: TspType, force: i32) -> fn(f64, f64) -> Val {
    if force == 1 {
        return create_rat;
    }
    if force == 2 {
        return create_dec;
    }
    let a = a as u32;
    let b = b as u32;
    if (a & TspType::TspDec as u32) != 0 || (b & TspType::TspDec as u32) != 0 {
        return create_dec;
    }
    if (a & TspType::TspRatio as u32) != 0 || (b & TspType::TspRatio as u32) != 0 {
        return create_rat;
    }
    create_int
}

pub fn prim_add(_st: &mut Tsp, _vars: &Rc<RefCell<Rec>>, _args: Val) -> Option<Val> {
    None
}
pub fn prim_sub(_st: &mut Tsp, _vars: &Rc<RefCell<Rec>>, _args: Val) -> Option<Val> {
    None
}
pub fn prim_mul(_st: &mut Tsp, _vars: &Rc<RefCell<Rec>>, _args: Val) -> Option<Val> {
    None
}
pub fn prim_div(_st: &mut Tsp, _vars: &Rc<RefCell<Rec>>, _args: Val) -> Option<Val> {
    None
}
pub fn prim_mod(_st: &mut Tsp, _vars: &Rc<RefCell<Rec>>, _args: Val) -> Option<Val> {
    None
}
pub fn prim_pow(_st: &mut Tsp, _vars: &Rc<RefCell<Rec>>, _args: Val) -> Option<Val> {
    None
}
pub fn prim_denominator(_st: &mut Tsp, _env: &Rc<RefCell<Rec>>, _args: Val) -> Option<Val> {
    None
}

pub fn tib_env_math(st: &mut Tsp) {
    let v = mk_prim(TspType::TspPrim, prim_add, "+").unwrap();
    tisp_env_add(st, "+", v);
    let v = mk_prim(TspType::TspPrim, prim_sub, "-").unwrap();
    tisp_env_add(st, "-", v);
    let v = mk_prim(TspType::TspPrim, prim_mul, "*").unwrap();
    tisp_env_add(st, "*", v);
    let v = mk_prim(TspType::TspPrim, prim_div, "/").unwrap();
    tisp_env_add(st, "/", v);
    let v = mk_prim(TspType::TspPrim, prim_mod, "mod").unwrap();
    tisp_env_add(st, "mod", v);
    let v = mk_prim(TspType::TspPrim, prim_pow, "^").unwrap();
    tisp_env_add(st, "^", v);
    let v = mk_prim(TspType::TspPrim, prim_denominator, "denominator").unwrap();
    tisp_env_add(st, "denominator", v);
}
