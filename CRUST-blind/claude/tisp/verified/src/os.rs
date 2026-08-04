use crate::tisp::{
    mk_dec, mk_int, mk_prim, mk_str, stub_prim, tisp_env_add, tisp_eval, val_clone, Rec, Tsp,
    TspType, Val, ValUnion,
};

fn car(v: &Val) -> Val {
    if let ValUnion::P { car, .. } = &v.v {
        val_clone(car)
    } else {
        Val {
            t: TspType::TspNil,
            v: ValUnion::N { num: 0.0, den: 1.0 },
        }
    }
}

pub fn prim_cd(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    let dir = car(&args);
    if !matches!(dir.t, TspType::TspStr | TspType::TspSym) {
        return val_clone(&st.none);
    }
    let path = if let ValUnion::S(s) = &dir.v {
        s.clone()
    } else {
        return val_clone(&st.none);
    };
    let _ = std::env::set_current_dir(&path);
    val_clone(&st.none)
}

pub fn prim_pwd(st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    match std::env::current_dir() {
        Ok(p) => mk_str(st, &p.to_string_lossy()).unwrap_or_else(|| val_clone(&st.none)),
        Err(_) => val_clone(&st.none),
    }
}

pub fn prim_exit(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    let a = car(&args);
    let code = if let ValUnion::N { num, .. } = &a.v {
        *num as i32
    } else {
        0
    };
    std::process::exit(code);
}

pub fn prim_now(_st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i32)
        .unwrap_or(0);
    mk_int(now)
}

pub fn form_time(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    let v = car(&args);
    let start = std::time::Instant::now();
    let _ = tisp_eval(st, v);
    let dur = start.elapsed();
    let ms = dur.as_secs_f64() * 100.0;
    mk_dec(ms).unwrap_or_else(|| val_clone(&st.none))
}

pub fn tib_env_os(st: &mut Tsp) {
    let prims: &[(&str, TspType)] = &[
        ("cd!", TspType::TspPrim),
        ("pwd", TspType::TspPrim),
        ("exit!", TspType::TspPrim),
        ("now", TspType::TspPrim),
        ("time", TspType::TspForm),
    ];
    for (n, t) in prims {
        let v = mk_prim(*t, stub_prim, n).unwrap();
        tisp_env_add(st, n, v);
    }
}
