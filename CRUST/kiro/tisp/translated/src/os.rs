use crate::tisp::*;

pub fn prim_cd(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if tsp_arg_num_check_pub(&args, "cd!", 1).is_none() { return mk_error(); }
    let dir = car_pub(&args);
    if !is_type_pub(dir, TspType::TspStr as u32 | TspType::TspSym as u32) {
        eprintln!("; tisp: error: cd!: expected string or symbol, received {}", tsp_type_str(dir.t));
        return mk_error();
    }
    let path = sym_str_pub(dir);
    if std::env::set_current_dir(path).is_err() {
        eprintln!("; error: cd");
        return mk_error();
    }
    mk_none_pub()
}
pub fn prim_pwd(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if tsp_arg_num_check_pub(&args, "pwd", 0).is_none() { return mk_error(); }
    match std::env::current_dir() {
        Ok(p) => mk_str(st, &p.to_string_lossy()),
        Err(_) => { eprintln!("; tisp: error: pwd: could not get current directory"); mk_error() }
    }
}
pub fn prim_exit(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if tsp_arg_num_check_pub(&args, "exit!", 1).is_none() { return mk_error(); }
    if tsp_arg_type_check_pub(car_pub(&args), "exit!", TspType::TspInt as u32).is_none() { return mk_error(); }
    std::process::exit(num_pub(car_pub(&args)) as i32);
}
pub fn prim_now(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if tsp_arg_num_check_pub(&args, "now", 0).is_none() { return mk_error(); }
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
    mk_int(now as i32)
}
pub fn form_time(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    if tsp_arg_num_check_pub(&args, "time", 1).is_none() { return mk_error(); }
    let expr = clone_val_pub(car_pub(&args));
    let start = std::time::Instant::now();
    match tisp_eval_with_env_pub(st, env, expr) {
        Some(_v) => {
            let elapsed = start.elapsed().as_secs_f64() * 100.0;
            mk_dec(elapsed).unwrap()
        }
        None => mk_error(),
    }
}
pub fn tib_env_os(st: &mut Tsp) {
    tisp_env_add(st, "cd!", mk_prim(TspType::TspPrim, prim_cd, "cd!"));
    tisp_env_add(st, "pwd", mk_prim(TspType::TspPrim, prim_pwd, "pwd"));
    tisp_env_add(st, "exit!", mk_prim(TspType::TspPrim, prim_exit, "exit!"));
    tisp_env_add(st, "now", mk_prim(TspType::TspPrim, prim_now, "now"));
    tisp_env_add(st, "time", mk_prim(TspType::TspForm, form_time, "time"));
}
