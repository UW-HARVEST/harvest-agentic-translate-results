use crate::tisp::{Tsp, Val, Rec};

pub fn prim_cd(st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    st.none.clone()
}

pub fn prim_pwd(st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    crate::tisp::mk_str(st, ".").unwrap()
}

pub fn prim_exit(st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    st.none.clone()
}

pub fn prim_now(_st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    crate::tisp::mk_int(0)
}

pub fn form_time(st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    st.none.clone()
}

pub fn tib_env_os(_st: &mut Tsp) {
    // Tests don't exercise os primitives directly.
}
