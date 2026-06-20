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

pub fn count_parens(s: &str, len: i32) -> i32 {
    tisp::count_parens(s, len)
}
pub fn read_file(fname: &str) -> String {
    tisp::read_file(fname)
}
pub fn prim_write(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    call(st, env, "write", args)
}
pub fn prim_read(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    call(st, env, "read", args)
}
pub fn prim_parse(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    call(st, env, "parse", args)
}
pub fn prim_load(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    call(st, env, "load", args)
}
pub fn tib_env_io(st: &mut Tsp) {
    tisp::tib_env_io(st);
}
