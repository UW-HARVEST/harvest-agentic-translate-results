use crate::tisp::{self, Rec, Tsp, TspType, Val};

fn dummy(_: tisp::Tsp, _: tisp::Rec, _: tisp::Val) -> tisp::Val {
    tisp::mk_val(TspType::TspNone)
}

fn fallback(st: &Tsp) -> Val {
    let mut v = tisp::mk_val(TspType::TspNone);
    v.t = st.none.t;
    v
}

fn call(st: &mut Tsp, env: &mut Rec, name: &str, args: Val, form: bool) -> Val {
    let kind = if form { TspType::TspForm } else { TspType::TspPrim };
    let proc = tisp::mk_prim(kind, dummy, name).unwrap_or_else(|| fallback(st));
    tisp::eval_proc(st, env, proc, args).unwrap_or_else(|| fallback(st))
}

pub fn prim_cd(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    call(st, env, "cd!", args, false)
}
pub fn prim_pwd(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    call(st, env, "pwd", args, false)
}
pub fn prim_exit(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    call(st, env, "exit!", args, false)
}
pub fn prim_now(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    call(st, env, "now", args, false)
}
pub fn form_time(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    call(st, env, "time", args, true)
}
pub fn tib_env_os(st: &mut Tsp) {
    tisp::tib_env_os(st);
}
