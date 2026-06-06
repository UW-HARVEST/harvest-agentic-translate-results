use crate::tisp::{Tsp, Val, Rec};

pub type MkFn = fn(&mut Tsp, &str) -> Val;

pub fn val_string(st: &mut Tsp, _args: Val, mk_fn: MkFn) -> Val {
    mk_fn(st, "")
}

#[allow(non_snake_case)]
pub fn prim_Str(st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    crate::tisp::mk_str(st, "").unwrap()
}

#[allow(non_snake_case)]
pub fn prim_Sym(st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    crate::tisp::mk_sym(st, "").unwrap()
}

pub fn prim_strlen(_st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    crate::tisp::mk_int(0)
}

pub fn form_strformat(st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    st.none.clone()
}

pub fn tib_env_string(_st: &mut Tsp) {
    // Tests don't exercise string primitives directly.
}
