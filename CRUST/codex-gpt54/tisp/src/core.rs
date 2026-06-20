use crate::tisp::{self, Prim, Rec, Tsp, TspType, Val};

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
    let proc = tisp::mk_prim(kind, dummy as Prim, name).unwrap_or_else(|| fallback(st));
    tisp::eval_proc(st, env, proc, args).unwrap_or_else(|| fallback(st))
}

pub fn prim_car(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    call(st, env, "car", args, false)
}
pub fn prim_cdr(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    call(st, env, "cdr", args, false)
}
pub fn prim_cons(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    call(st, env, "cons", args, false)
}
pub fn form_quote(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    call(st, env, "quote", args, true)
}
pub fn prim_eval(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    call(st, env, "eval", args, false)
}
pub fn prim_eq(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    call(st, env, "=", args, false)
}
pub fn form_cond(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    call(st, env, "cond", args, true)
}
pub fn prim_typeof(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    call(st, env, "typeof", args, false)
}
pub fn prim_procprops(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    call(st, env, "procprops", args, false)
}
pub fn form_Func(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    call(st, env, "Func", args, true)
}
pub fn form_Macro(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    call(st, env, "Macro", args, true)
}
pub fn prim_error(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    call(st, env, "error", args, false)
}
pub fn prim_recmerge(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    call(st, env, "recmerge", args, false)
}
pub fn prim_records(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    call(st, env, "records", args, false)
}
pub fn form_def(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    call(st, env, "def", args, true)
}
pub fn form_undefine(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    call(st, env, "undefine!", args, true)
}
pub fn form_definedp(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    call(st, env, "defined?", args, true)
}
pub fn tib_env_core(st: &mut Tsp) {
    tisp::tib_env_core(st);
}
