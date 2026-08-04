use std::time::{SystemTime, UNIX_EPOCH};

use crate::tisp::{Rec, Tsp, TspType, Val, eval_in_env, expect_len, expect_type, mk_dec, mk_int, mk_prim, mk_str, pair_car, tisp_env_add, val_str};

pub fn prim_cd(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if !expect_len(st, &args, "cd!", 1) {
        return st.none.clone();
    }
    let dir = pair_car(&args).clone();
    if !expect_type(st, &dir, "cd!", TspType::TspStr as u32 | TspType::TspSym as u32) {
        return st.none.clone();
    }
    let _ = std::env::set_current_dir(val_str(&dir).unwrap_or_default());
    st.none.clone()
}

pub fn prim_pwd(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if !expect_len(st, &args, "pwd", 0) {
        return st.none.clone();
    }
    match std::env::current_dir() {
        Ok(dir) => mk_str(st, &dir.to_string_lossy()).unwrap_or_else(|| st.none.clone()),
        Err(_) => st.none.clone(),
    }
}

pub fn prim_exit(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    let code = if let crate::tisp::ValUnion::P { car, .. } = &args.v {
        crate::tisp::val_num(car) as i32
    } else {
        0
    };
    std::process::exit(code);
}

pub fn prim_now(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if !expect_len(st, &args, "now", 0) {
        return st.none.clone();
    }
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i32)
        .unwrap_or_default();
    mk_int(secs)
}

pub fn form_time(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    if !expect_len(st, &args, "time", 1) {
        return st.none.clone();
    }
    let start = std::time::Instant::now();
    let _ = eval_in_env(st, env, pair_car(&args).clone());
    mk_dec(start.elapsed().as_secs_f64() * 100.0).unwrap_or_else(|| st.none.clone())
}

pub fn tib_env_os(st: &mut Tsp) {
    tisp_env_add(st, "cd!", mk_prim(TspType::TspPrim, prim_cd, "cd!").unwrap());
    tisp_env_add(st, "pwd", mk_prim(TspType::TspPrim, prim_pwd, "pwd").unwrap());
    tisp_env_add(st, "exit!", mk_prim(TspType::TspPrim, prim_exit, "exit!").unwrap());
    tisp_env_add(st, "now", mk_prim(TspType::TspPrim, prim_now, "now").unwrap());
    tisp_env_add(st, "time", mk_prim(TspType::TspForm, form_time, "time").unwrap());
}
