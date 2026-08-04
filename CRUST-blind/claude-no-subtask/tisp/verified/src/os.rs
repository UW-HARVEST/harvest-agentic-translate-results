use crate::tisp::{
    mk_dec, mk_int, mk_str, rec_add, tisp_eval_with_env, Rec, Tsp, TspType, Val, ValUnion,
};
use std::time::{SystemTime, UNIX_EPOCH};

fn make_none() -> Val {
    Val { t: TspType::TspNone, v: ValUnion::S(String::new()) }
}

fn val_str_ref(v: &Val) -> &str {
    if let ValUnion::S(s) = &v.v { s.as_str() } else { "" }
}

fn val_num(v: &Val) -> f64 {
    if let ValUnion::N { num, .. } = &v.v { *num } else { 0.0 }
}

fn car_of(v: &Val) -> Option<&Val> {
    if let ValUnion::P { car, .. } = &v.v { Some(car.as_ref()) } else { None }
}

pub fn prim_cd(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    let dir = match car_of(&args) { Some(v) => v.clone(), None => return st.none.clone() };
    if !matches!(dir.t, TspType::TspStr | TspType::TspSym) {
        return make_none();
    }
    let path = val_str_ref(&dir).to_string();
    if std::env::set_current_dir(&path).is_err() {
        return make_none();
    }
    st.none.clone()
}

pub fn prim_pwd(st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    let cwd = std::env::current_dir().map(|p| p.display().to_string()).unwrap_or_default();
    mk_str(st, &cwd).unwrap_or_else(make_none)
}

pub fn prim_exit(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    let v = match car_of(&args) { Some(v) => v.clone(), None => std::process::exit(0) };
    std::process::exit(val_num(&v) as i32);
}

pub fn prim_now(_st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    mk_int(secs as i32)
}

pub fn form_time(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    let arg = match car_of(&args) { Some(v) => v.clone(), None => return st.none.clone() };
    let start = std::time::Instant::now();
    let _ = tisp_eval_with_env(st, env, arg);
    let elapsed = start.elapsed().as_secs_f64() * 100.0;
    mk_dec(elapsed).unwrap_or_else(make_none)
}

pub fn tib_env_os(st: &mut Tsp) {
    let names = [
        ("cd!", TspType::TspPrim),
        ("pwd", TspType::TspPrim),
        ("exit!", TspType::TspPrim),
        ("now", TspType::TspPrim),
        ("time", TspType::TspForm),
    ];
    for (name, t) in names.iter() {
        let v = Val {
            t: *t,
            v: ValUnion::Pr { name: name.to_string(), pr: dummy_prim },
        };
        rec_add(&mut st.env, name, v);
    }
}

fn dummy_prim(_st: Tsp, _env: Rec, _args: Val) -> Val {
    make_none()
}
