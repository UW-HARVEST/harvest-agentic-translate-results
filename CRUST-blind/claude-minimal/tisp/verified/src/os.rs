use crate::tisp::{mk_dec, mk_int, mk_str, mk_val, tisp_eval, Rec, Tsp, TspType, Val, ValUnion};
use std::time::{SystemTime, UNIX_EPOCH};

/* change to new directory */
pub fn prim_cd(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if let ValUnion::P { car, .. } = &args.v {
        if let ValUnion::S(s) = &car.v {
            if matches!(car.t, TspType::TspStr | TspType::TspSym) {
                let _ = std::env::set_current_dir(s);
                return mk_val(TspType::TspNone);
            }
        }
    }
    mk_val(TspType::TspNone)
}

/* return string of current working directory */
pub fn prim_pwd(st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    if let Ok(p) = std::env::current_dir() {
        if let Some(s) = p.to_str() {
            return mk_str(st, s).unwrap_or_else(|| mk_val(TspType::TspStr));
        }
    }
    mk_val(TspType::TspNone)
}

/* exit program with return value of given int */
pub fn prim_exit(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    let mut code = 0;
    if let ValUnion::P { car, .. } = &args.v {
        if let ValUnion::N { num, .. } = &car.v {
            code = *num as i32;
        }
    }
    std::process::exit(code);
}

/* return number of seconds since epoch */
pub fn prim_now(_st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    mk_int(secs as i32)
}

/* return time in milliseconds taken to run command given */
pub fn form_time(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    let start = std::time::Instant::now();
    if let ValUnion::P { car, .. } = args.v {
        let _ = tisp_eval(st, *car);
    }
    let elapsed = start.elapsed();
    mk_dec(elapsed.as_secs_f64() * 100.0).unwrap_or_else(|| mk_val(TspType::TspDec))
}

pub fn tib_env_os(_st: &mut Tsp) {
    /* os environment registration is a no-op stub */
}
