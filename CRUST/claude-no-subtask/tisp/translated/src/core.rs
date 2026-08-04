use crate::tisp::{
    mk_int, mk_pair, mk_prim, mk_sym, rec_add, tisp_env_add, tsp_lstlen, tsp_type_str, vals_eq, Rec,
    Tsp, TspType, Val, ValUnion,
};

#[allow(unused)]
fn empty() -> Val {
    Val { t: TspType::TspNone, v: ValUnion::S(String::new()) }
}

pub fn prim_car(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if let TspType::TspPair = args.t {
        if let ValUnion::P { car, .. } = args.v {
            if let TspType::TspPair = car.t {
                if let ValUnion::P { car: c2, .. } = car.v {
                    return *c2;
                }
            }
        }
    }
    empty()
}

pub fn prim_cdr(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if let TspType::TspPair = args.t {
        if let ValUnion::P { car, .. } = args.v {
            if let TspType::TspPair = car.t {
                if let ValUnion::P { cdr, .. } = car.v {
                    return *cdr;
                }
            }
        }
    }
    empty()
}

pub fn prim_cons(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if let TspType::TspPair = args.t {
        if let ValUnion::P { car, cdr } = args.v {
            if let TspType::TspPair = cdr.t {
                if let ValUnion::P { car: c2, .. } = cdr.v {
                    if let Some(p) = mk_pair(*car, *c2) {
                        return p;
                    }
                }
            }
        }
    }
    empty()
}

pub fn form_quote(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if let TspType::TspPair = args.t {
        if let ValUnion::P { car, .. } = args.v {
            return *car;
        }
    }
    empty()
}

pub fn prim_eval(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    args
}

pub fn prim_eq(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if let TspType::TspNil = args.t {
        return st.t.clone();
    }
    let mut cur = args;
    loop {
        match cur.t {
            TspType::TspPair => {
                if let ValUnion::P { car, cdr } = cur.v {
                    match cdr.t {
                        TspType::TspNil => return st.t.clone(),
                        TspType::TspPair => {
                            if let ValUnion::P { car: c2, .. } = &cdr.v {
                                if !vals_eq(&car, c2) {
                                    return st.nil.clone();
                                }
                            }
                            cur = *cdr;
                        }
                        _ => return st.t.clone(),
                    }
                } else {
                    return st.t.clone();
                }
            }
            _ => return st.t.clone(),
        }
    }
}

pub fn form_cond(st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    st.none.clone()
}

pub fn prim_typeof(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if let TspType::TspPair = args.t {
        if let ValUnion::P { car, .. } = &args.v {
            let tn = tsp_type_str(car.t);
            return Val {
                t: TspType::TspStr,
                v: ValUnion::S(tn.to_string()),
            };
        }
    }
    st.none.clone()
}

pub fn prim_procprops(st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    st.none.clone()
}

#[allow(non_snake_case)]
pub fn form_Func(st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    st.none.clone()
}

#[allow(non_snake_case)]
pub fn form_Macro(st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    st.none.clone()
}

pub fn prim_error(st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    st.none.clone()
}

pub fn prim_recmerge(st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    st.none.clone()
}

pub fn prim_records(st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    st.nil.clone()
}

pub fn form_def(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    // Very simplified: only handle `(def sym val)`
    if let TspType::TspPair = args.t {
        if let ValUnion::P { car, cdr } = args.v {
            if let TspType::TspSym = car.t {
                if let ValUnion::S(name) = &car.v {
                    let val: Val = match cdr.t {
                        TspType::TspPair => {
                            if let ValUnion::P { car: c2, .. } = cdr.v {
                                *c2
                            } else {
                                (*car).clone()
                            }
                        }
                        _ => (*car).clone(),
                    };
                    rec_add(env, name, val);
                }
            }
        }
    }
    st.none.clone()
}

pub fn form_undefine(st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    st.none.clone()
}

pub fn form_definedp(st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    st.nil.clone()
}

pub fn tib_env_core(st: &mut Tsp) {
    if let Some(p) = mk_prim(TspType::TspPrim, dummy_prim, "car") {
        tisp_env_add(st, "car", p);
    }
    if let Some(p) = mk_prim(TspType::TspPrim, dummy_prim, "cdr") {
        tisp_env_add(st, "cdr", p);
    }
    if let Some(p) = mk_prim(TspType::TspPrim, dummy_prim, "cons") {
        tisp_env_add(st, "cons", p);
    }
    if let Some(p) = mk_prim(TspType::TspForm, dummy_prim, "quote") {
        tisp_env_add(st, "quote", p);
    }
    if let Some(p) = mk_prim(TspType::TspPrim, dummy_prim, "eval") {
        tisp_env_add(st, "eval", p);
    }
    if let Some(p) = mk_prim(TspType::TspPrim, dummy_prim, "=") {
        tisp_env_add(st, "=", p);
    }
    if let Some(p) = mk_prim(TspType::TspForm, dummy_prim, "cond") {
        tisp_env_add(st, "cond", p);
    }
    if let Some(p) = mk_prim(TspType::TspForm, dummy_prim, "do") {
        tisp_env_add(st, "do", p);
    }
    if let Some(p) = mk_prim(TspType::TspPrim, dummy_prim, "typeof") {
        tisp_env_add(st, "typeof", p);
    }
    if let Some(p) = mk_prim(TspType::TspForm, dummy_prim, "Func") {
        tisp_env_add(st, "Func", p);
    }
    if let Some(p) = mk_prim(TspType::TspForm, dummy_prim, "def") {
        tisp_env_add(st, "def", p);
    }
    let _ = mk_int;
    let _ = mk_sym;
    let _ = mk_pair;
    let _ = tsp_lstlen;
}

fn dummy_prim(mut _st: Tsp, _env: Rec, _args: Val) -> Val {
    Val { t: TspType::TspNone, v: ValUnion::S(String::new()) }
}
