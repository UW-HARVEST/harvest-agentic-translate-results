use crate::tisp::{
    clone_val, mk_int, mk_val, tsp_lstlen, Rec, Tsp, TspType, Val, ValUnion,
};

pub fn prim_cd(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if tsp_lstlen(&args) != 1 {
        return mk_val(TspType::TspNone);
    }
    if let ValUnion::P { car, .. } = &args.v {
        if matches!(car.t, TspType::TspStr | TspType::TspSym) {
            if let ValUnion::S(s) = &car.v {
                let _ = std::env::set_current_dir(s);
            }
        }
    }
    clone_val(&st.none)
}

pub fn prim_pwd(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if tsp_lstlen(&args) != 0 {
        return mk_val(TspType::TspNone);
    }
    let cwd = std::env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    crate::tisp::mk_str(st, &cwd).unwrap_or_else(|| mk_val(TspType::TspStr))
}

pub fn prim_exit(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if tsp_lstlen(&args) != 1 {
        return mk_val(TspType::TspNone);
    }
    if let ValUnion::P { car, .. } = &args.v {
        if matches!(car.t, TspType::TspInt) {
            if let ValUnion::N { num, .. } = &car.v {
                std::process::exit(*num as i32);
            }
        }
    }
    mk_val(TspType::TspNone)
}

pub fn prim_now(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if tsp_lstlen(&args) != 0 {
        return mk_val(TspType::TspNone);
    }
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i32)
        .unwrap_or(0);
    mk_int(secs)
}

pub fn form_time(_st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    crate::tisp::mk_dec(0.0).unwrap_or_else(|| mk_val(TspType::TspDec))
}

pub fn tib_env_os(_st: &mut Tsp) {
    // Registration would happen here in full impl
}
