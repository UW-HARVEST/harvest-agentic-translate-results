use crate::tisp::*;
use std::time::{SystemTime, UNIX_EPOCH, Instant};

fn tsp_arg_num_check(args: &Val, name: &str, nargs: i32) -> bool {
    if nargs > -1 && tsp_lstlen(args) != nargs {
        eprintln!("; tisp: error: {}: expected {} argument{}, received {}",
            name, nargs, if nargs > 1 { "s" } else { "" }, tsp_lstlen(args));
        false
    } else { true }
}

fn tsp_arg_type_check(arg: &Val, name: &str, type_bits: u32) -> bool {
    if (arg.t as u32) & type_bits == 0 {
        eprintln!("; tisp: error: {}: expected {}, received {}",
            name, tsp_type_str_bits(type_bits), tsp_type_str(arg.t));
        false
    } else { true }
}

pub fn prim_cd(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if !tsp_arg_num_check(&args, "cd!", 1) { return mk_err(); }
    let dir = car(&args);
    if (dir.t as u32) & (TspType::TspStr as u32 | TspType::TspSym as u32) == 0 {
        eprintln!("; tisp: error: cd!: expected string or symbol, received {}", tsp_type_str(dir.t));
        return mk_err();
    }
    if std::env::set_current_dir(vs(dir)).is_err() {
        eprintln!("; error: cd");
        return mk_err();
    }
    clone_val(&st.none)
}

pub fn prim_pwd(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if !tsp_arg_num_check(&args, "pwd", 0) { return mk_err(); }
    match std::env::current_dir() {
        Ok(p) => mk_str(st, &p.to_string_lossy()).unwrap_or_else(|| mk_err()),
        Err(_) => {
            eprintln!("; tisp: error: pwd: could not get current directory");
            mk_err()
        }
    }
}

pub fn prim_exit(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if !tsp_arg_num_check(&args, "exit!", 1) { return mk_err(); }
    if !tsp_arg_type_check(car(&args), "exit!", TspType::TspInt as u32) { return mk_err(); }
    std::process::exit(vnum(car(&args)) as i32);
}

pub fn prim_now(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if !tsp_arg_num_check(&args, "now", 0) { return mk_err(); }
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    mk_int(secs as i32)
}

pub fn form_time(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    if !tsp_arg_num_check(&args, "time", 1) { return mk_err(); }
    let start = Instant::now();
    let expr = clone_val(car(&args));
    match tisp_eval_with_env(st, env, expr) {
        None => mk_err(),
        Some(_) => {
            let elapsed = start.elapsed();
            let ms = elapsed.as_secs_f64() * 100.0; // match C: clock()/CLOCKS_PER_SEC*100
            mk_dec(ms).unwrap()
        }
    }
}

pub fn tib_env_os(st: &mut Tsp) {
    tisp_env_add(st, "cd!", mk_prim(TspType::TspPrim, prim_cd, "cd!").unwrap());
    tisp_env_add(st, "pwd", mk_prim(TspType::TspPrim, prim_pwd, "pwd").unwrap());
    tisp_env_add(st, "exit!", mk_prim(TspType::TspPrim, prim_exit, "exit!").unwrap());
    tisp_env_add(st, "now", mk_prim(TspType::TspPrim, prim_now, "now").unwrap());
    tisp_env_add(st, "time", mk_prim(TspType::TspForm, form_time, "time").unwrap());
}
