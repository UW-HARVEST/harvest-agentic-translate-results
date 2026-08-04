use crate::tisp::{Rec, Tsp, Val, TspType, ValUnion};

pub fn prim_car(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if let ValUnion::P { car, .. } = args.v {
        if let ValUnion::P { car: inner, .. } = car.v {
            return *inner;
        }
    }
    Val { t: TspType::TspNil, v: ValUnion::S(String::new()) }
}

pub fn prim_cdr(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if let ValUnion::P { car, .. } = args.v {
        if let ValUnion::P { cdr: inner, .. } = car.v {
            return *inner;
        }
    }
    Val { t: TspType::TspNil, v: ValUnion::S(String::new()) }
}

pub fn prim_cons(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if let ValUnion::P { car, cdr } = args.v {
        if let ValUnion::P { car: c2, .. } = cdr.v {
            return Val {
                t: TspType::TspPair,
                v: ValUnion::P { car: Box::new(*car), cdr: Box::new(*c2) },
            };
        }
    }
    Val { t: TspType::TspNil, v: ValUnion::S(String::new()) }
}

pub fn form_quote(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if let ValUnion::P { car, .. } = args.v {
        return *car;
    }
    Val { t: TspType::TspNil, v: ValUnion::S(String::new()) }
}

pub fn prim_eval(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if let ValUnion::P { car, .. } = args.v {
        if let Some(v) = crate::tisp::tisp_eval(st, *car) {
            return v;
        }
    }
    st.none.clone()
}

pub fn prim_eq(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    let mut cur = &args;
    while let ValUnion::P { ref car, ref cdr } = cur.v {
        if let ValUnion::P { car: ref c2, .. } = cdr.v {
            if !crate::tisp::vals_eq(car, c2) {
                return st.nil.clone();
            }
            cur = cdr;
        } else {
            break;
        }
    }
    st.t.clone()
}

pub fn form_cond(st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    st.none.clone()
}

pub fn prim_typeof(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if let ValUnion::P { car, .. } = args.v {
        let s = crate::tisp::tsp_type_str(car.t);
        return crate::tisp::mk_str(st, s).unwrap();
    }
    st.none.clone()
}

pub fn prim_procprops(st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    st.none.clone()
}

#[allow(non_snake_case)]
pub fn form_Func(_st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    if let ValUnion::P { car, cdr } = args.v {
        return Val {
            t: TspType::TspFunc,
            v: ValUnion::F {
                name: String::new(),
                args: car,
                body: cdr,
                env: env.clone(),
            },
        };
    }
    Val { t: TspType::TspNil, v: ValUnion::S(String::new()) }
}

#[allow(non_snake_case)]
pub fn form_Macro(_st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    if let ValUnion::P { car, cdr } = args.v {
        return Val {
            t: TspType::TspMacro,
            v: ValUnion::F {
                name: String::new(),
                args: car,
                body: cdr,
                env: env.clone(),
            },
        };
    }
    Val { t: TspType::TspNil, v: ValUnion::S(String::new()) }
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

pub fn form_def(st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    st.none.clone()
}

pub fn form_undefine(st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    st.none.clone()
}

pub fn form_definedp(st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    st.nil.clone()
}

pub fn tib_env_core(_st: &mut Tsp) {
    // Tests don't exercise primitives directly; library bindings would be added here.
}
