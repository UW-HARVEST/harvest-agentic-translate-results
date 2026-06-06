use crate::tisp::{Rec, Tsp, TspType, Val, ValUnion};

pub type MkFn = fn(&mut Tsp, &str) -> Val;

#[allow(unused)]
fn empty() -> Val {
    Val { t: TspType::TspNone, v: ValUnion::S(String::new()) }
}

pub fn val_string(_st: &mut Tsp, _args: Val, _mk_fn: MkFn) -> Val {
    empty()
}

#[allow(non_snake_case)]
pub fn prim_Str(_st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    empty()
}

#[allow(non_snake_case)]
pub fn prim_Sym(_st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    empty()
}

pub fn prim_strlen(_st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    empty()
}

pub fn form_strformat(_st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    empty()
}

pub fn tib_env_string(_st: &mut Tsp) {
    // Stub
}
