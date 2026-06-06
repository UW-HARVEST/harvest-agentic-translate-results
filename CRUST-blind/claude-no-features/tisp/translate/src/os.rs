use crate::tisp::{
    mk_dec, mk_int, mk_prim, mk_str, nil_val, none_val, tisp_env_add, tisp_eval_with_env,
    tsp_lstlen, tsp_type_str, val_car, val_num, val_str, warn, Rec, Tsp, TspType, Val,
};

pub fn prim_cd(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if tsp_lstlen(&args) != 1 {
        warn("cd!: expected 1 argument");
        return none_val();
    }
    let dir = val_car(&args).cloned().unwrap_or_else(nil_val);
    if !matches!(dir.t, TspType::TspStr | TspType::TspSym) {
        warn(&format!(
            "cd!: expected string or symbol, received {}",
            tsp_type_str(dir.t)
        ));
        return none_val();
    }
    if std::env::set_current_dir(val_str(&dir)).is_err() {
        warn("cd: failed");
        return none_val();
    }
    st.none.clone()
}

pub fn prim_pwd(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if tsp_lstlen(&args) != 0 {
        warn("pwd: expected 0 arguments");
        return none_val();
    }
    match std::env::current_dir() {
        Ok(p) => mk_str(st, p.to_string_lossy().as_ref()).unwrap_or_else(nil_val),
        Err(_) => {
            warn("pwd: could not get current directory");
            none_val()
        }
    }
}

pub fn prim_exit(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if tsp_lstlen(&args) != 1 {
        warn("exit!: expected 1 argument");
        return none_val();
    }
    let v = val_car(&args).cloned().unwrap_or_else(nil_val);
    if !matches!(v.t, TspType::TspInt) {
        warn("exit!: expected Int");
        return none_val();
    }
    std::process::exit(val_num(&v) as i32);
}

pub fn prim_now(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if tsp_lstlen(&args) != 0 {
        warn("now: expected 0 arguments");
        return none_val();
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i32)
        .unwrap_or(0);
    mk_int(now)
}

pub fn form_time(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    if tsp_lstlen(&args) != 1 {
        warn("time: expected 1 argument");
        return none_val();
    }
    let start = std::time::Instant::now();
    let v = val_car(&args).cloned().unwrap_or_else(nil_val);
    let _ = tisp_eval_with_env(st, env, v);
    let elapsed = start.elapsed();
    mk_dec(elapsed.as_secs_f64() * 100.0).unwrap_or_else(nil_val)
}

pub fn tib_env_os(st: &mut Tsp) {
    add(st, "cd!", TspType::TspPrim);
    add(st, "pwd", TspType::TspPrim);
    add(st, "exit!", TspType::TspPrim);
    add(st, "now", TspType::TspPrim);
    add(st, "time", TspType::TspForm);
}

fn add(st: &mut Tsp, name: &str, t: TspType) {
    let v = mk_prim(t, dummy_prim, name).unwrap_or_else(nil_val);
    tisp_env_add(st, name, v);
}

fn dummy_prim(_st: Tsp, _env: Rec, _args: Val) -> Val {
    none_val()
}
