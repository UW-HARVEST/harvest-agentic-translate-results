use crate::tisp::{Rec, Tsp, TspType, Val, ValUnion};

#[allow(unused)]
fn empty() -> Val {
    Val { t: TspType::TspNone, v: ValUnion::S(String::new()) }
}

pub fn prim_cd(_st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    empty()
}

pub fn prim_pwd(_st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    empty()
}

pub fn prim_exit(_st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    empty()
}

pub fn prim_now(_st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    empty()
}

pub fn form_time(_st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    empty()
}

pub fn tib_env_os(_st: &mut Tsp) {
    // Stub
}
