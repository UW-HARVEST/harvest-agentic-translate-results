use crate::tisp::{
    clone_val, mk_pair, mk_val, tsp_lstlen, tsp_type_str, vals_eq, Rec, Tsp, TspType, Val, ValUnion,
};

fn first(args: &Val) -> Option<Val> {
    if let ValUnion::P { car, .. } = &args.v {
        return Some(clone_val(car));
    }
    None
}

fn cdr_of(args: &Val) -> Option<Val> {
    if let ValUnion::P { cdr, .. } = &args.v {
        return Some(clone_val(cdr));
    }
    None
}

fn second(args: &Val) -> Option<Val> {
    if let ValUnion::P { cdr, .. } = &args.v {
        if let ValUnion::P { car, .. } = &cdr.v {
            return Some(clone_val(car));
        }
    }
    None
}

pub fn prim_car(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if tsp_lstlen(&args) != 1 {
        return mk_val(TspType::TspNone);
    }
    let first_arg = match first(&args) {
        Some(v) => v,
        None => return mk_val(TspType::TspNone),
    };
    if !matches!(first_arg.t, TspType::TspPair) {
        return mk_val(TspType::TspNone);
    }
    if let ValUnion::P { car, .. } = first_arg.v {
        return *car;
    }
    mk_val(TspType::TspNone)
}

pub fn prim_cdr(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if tsp_lstlen(&args) != 1 {
        return mk_val(TspType::TspNone);
    }
    let first_arg = match first(&args) {
        Some(v) => v,
        None => return mk_val(TspType::TspNone),
    };
    if !matches!(first_arg.t, TspType::TspPair) {
        return mk_val(TspType::TspNone);
    }
    if let ValUnion::P { cdr, .. } = first_arg.v {
        return *cdr;
    }
    mk_val(TspType::TspNone)
}

pub fn prim_cons(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if tsp_lstlen(&args) != 2 {
        return mk_val(TspType::TspNone);
    }
    let a = match first(&args) {
        Some(v) => v,
        None => return mk_val(TspType::TspNone),
    };
    let b = match second(&args) {
        Some(v) => v,
        None => return mk_val(TspType::TspNone),
    };
    mk_pair(a, b).unwrap_or_else(|| mk_val(TspType::TspNone))
}

pub fn form_quote(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if tsp_lstlen(&args) != 1 {
        return mk_val(TspType::TspNone);
    }
    first(&args).unwrap_or_else(|| mk_val(TspType::TspNone))
}

pub fn prim_eval(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if tsp_lstlen(&args) != 1 {
        return mk_val(TspType::TspNone);
    }
    let arg = match first(&args) {
        Some(v) => v,
        None => return mk_val(TspType::TspNone),
    };
    crate::tisp::tisp_eval(st, arg).unwrap_or_else(|| mk_val(TspType::TspNone))
}

pub fn prim_eq(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if matches!(args.t, TspType::TspNil) {
        return clone_val(&st.t);
    }
    let mut cur = &args;
    while let ValUnion::P { car, cdr } = &cur.v {
        if matches!(cdr.t, TspType::TspNil) {
            break;
        }
        if let ValUnion::P { car: car2, .. } = &cdr.v {
            if !vals_eq(car, car2) {
                return clone_val(&st.nil);
            }
        }
        cur = cdr;
    }
    clone_val(&st.t)
}

pub fn form_cond(st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    clone_val(&st.none)
}

pub fn prim_typeof(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if tsp_lstlen(&args) != 1 {
        return mk_val(TspType::TspNone);
    }
    let arg = match first(&args) {
        Some(v) => v,
        None => return mk_val(TspType::TspNone),
    };
    let s = tsp_type_str(arg.t);
    crate::tisp::mk_str(st, s).unwrap_or_else(|| mk_val(TspType::TspNone))
}

pub fn prim_procprops(_st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    mk_val(TspType::TspNone)
}

#[allow(non_snake_case)]
pub fn form_Func(_st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    mk_val(TspType::TspNone)
}

#[allow(non_snake_case)]
pub fn form_Macro(_st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    mk_val(TspType::TspNone)
}

pub fn prim_error(_st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    mk_val(TspType::TspNone)
}

pub fn prim_recmerge(_st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    mk_val(TspType::TspNone)
}

pub fn prim_records(st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    clone_val(&st.nil)
}

pub fn form_def(st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    clone_val(&st.none)
}

pub fn form_undefine(st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    clone_val(&st.none)
}

pub fn form_definedp(st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    clone_val(&st.nil)
}

pub fn tib_env_core(_st: &mut Tsp) {
    // Primitives registration would happen here in full impl
}
