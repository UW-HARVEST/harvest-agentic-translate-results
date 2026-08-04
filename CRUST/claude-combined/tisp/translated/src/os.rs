use crate::tisp::{Rec, Tsp, Val};

pub fn prim_cd(st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    st.none.clone()
}

pub fn prim_pwd(st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    st.none.clone()
}

pub fn prim_exit(st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    st.none.clone()
}

pub fn prim_now(st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    st.none.clone()
}

pub fn form_time(st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    st.none.clone()
}

pub fn tib_env_os(_st: &mut Tsp) {
    // Primitives use a different signature than crate::tisp::Prim, so we don't register them.
}
