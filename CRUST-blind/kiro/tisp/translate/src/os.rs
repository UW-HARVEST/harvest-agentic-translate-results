use crate::tisp::*;
use std::time::Instant;

fn tsp_arg_check(args: &Val, name: &str, n: i32) {
    let len = tsp_lstlen(args);
    if n > -1 && len != n {
        eprintln!("; tisp: error: {}: expected {} argument{}, received {}",
            name, n, if n > 1 { "s" } else { "" }, len);
    }
}
fn tsp_type_check(v: &Val, name: &str, type_mask: u32) {
    if (v.t as u32) & type_mask == 0 {
        eprintln!("; tisp: error: {}: expected {}, received {}",
            name, tsp_type_str_mask(type_mask), tsp_type_str(v.t));
    }
}
fn tsp_type_str_mask(t: u32) -> &'static str {
    if t == TspType::TspInt as u32 { return "Int"; }
    if t == (TspType::TspStr as u32 | TspType::TspSym as u32) { return "Str"; }
    "Invalid"
}

pub fn prim_cd(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    tsp_arg_check(&args, "cd!", 1);
    let dir = car_ref(&args);
    if !type_matches(dir.t, TspType::TspStr as u32 | TspType::TspSym as u32) {
        eprintln!("; tisp: error: cd!: expected string or symbol, received {}", tsp_type_str(dir.t));
        return mk_val(TspType::TspNone);
    }
    let path = sym_str(dir);
    if std::env::set_current_dir(path).is_err() {
        eprintln!("; error: cd");
        return mk_val(TspType::TspNone);
    }
    val_clone(&st.none)
}

pub fn prim_pwd(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    tsp_arg_check(&args, "pwd", 0);
    match std::env::current_dir() {
        Ok(p) => mk_str(st, &p.to_string_lossy()).unwrap(),
        Err(_) => {
            eprintln!("; tisp: error: pwd: could not get current directory");
            mk_val(TspType::TspNone)
        }
    }
}

pub fn prim_exit(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    tsp_arg_check(&args, "exit!", 1);
    tsp_type_check(car_ref(&args), "exit!", TspType::TspInt as u32);
    std::process::exit(num_of(car_ref(&args)) as i32);
}

pub fn prim_now(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    tsp_arg_check(&args, "now", 0);
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    mk_int(secs as i32)
}

pub fn form_time(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    tsp_arg_check(&args, "time", 1);
    let start = Instant::now();
    let _v = tisp_eval_with_env(st, env, val_clone(car_ref(&args)));
    let elapsed = start.elapsed();
    let ms = elapsed.as_secs_f64() * 100.0;
    mk_dec(ms).unwrap()
}

pub fn tib_env_os(st: &mut Tsp) {
    tisp_env_add(st, "cd!", mk_prim(TspType::TspPrim, prim_cd, "cd!").unwrap());
    tisp_env_add(st, "pwd", mk_prim(TspType::TspPrim, prim_pwd, "pwd").unwrap());
    tisp_env_add(st, "exit!", mk_prim(TspType::TspPrim, prim_exit, "exit!").unwrap());
    tisp_env_add(st, "now", mk_prim(TspType::TspPrim, prim_now, "now").unwrap());
    tisp_env_add(st, "time", mk_prim(TspType::TspForm, form_time, "time").unwrap());
}
