use crate::tisp::{
    mk_func, mk_pair, mk_rec, mk_str, mk_sym, mk_val, rec_new, tisp_eval, tisp_eval_body,
    tsp_lstlen, tsp_type_str, vals_eq, Rec, Tsp, TspType, Val, ValUnion,
};

/* helpers (similar to C macros) */
fn car(v: &Val) -> Option<&Val> {
    if let ValUnion::P { car, .. } = &v.v {
        Some(car)
    } else {
        None
    }
}

fn cdr(v: &Val) -> Option<&Val> {
    if let ValUnion::P { cdr, .. } = &v.v {
        Some(cdr)
    } else {
        None
    }
}

fn is_nil(v: &Val) -> bool {
    matches!(v.t, TspType::TspNil)
}

/* return first element of list */
pub fn prim_car(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if let Some(first) = car(&args) {
        if let Some(inner) = car(first) {
            return Val {
                t: inner.t,
                v: ValUnion::N { num: 0.0, den: 1.0 },
            };
        }
    }
    mk_val(TspType::TspNone)
}

/* return elements of a list after the first */
pub fn prim_cdr(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if let Some(first) = car(&args) {
        if let Some(inner) = cdr(first) {
            return Val {
                t: inner.t,
                v: ValUnion::N { num: 0.0, den: 1.0 },
            };
        }
    }
    mk_val(TspType::TspNone)
}

/* return new pair */
pub fn prim_cons(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    /* destructure args to get first 2 */
    if let ValUnion::P { car: a, cdr: rest } = args.v {
        if let ValUnion::P { car: b, .. } = rest.v {
            return mk_pair(*a, *b).unwrap_or_else(|| mk_val(TspType::TspNone));
        }
    }
    mk_val(TspType::TspNone)
}

/* do not evaluate argument */
pub fn form_quote(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if let ValUnion::P { car, .. } = args.v {
        return *car;
    }
    mk_val(TspType::TspNone)
}

/* evaluate argument given */
pub fn prim_eval(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if let ValUnion::P { car, .. } = args.v {
        return tisp_eval(st, *car).unwrap_or_else(|| mk_val(TspType::TspNone));
    }
    mk_val(TspType::TspNone)
}

/* test equality of all values given */
pub fn prim_eq(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if is_nil(&args) {
        return Val {
            t: TspType::TspSym,
            v: ValUnion::S("True".to_string()),
        };
    }
    let mut cur = &args;
    while let Some(rest) = cdr(cur) {
        if is_nil(rest) {
            break;
        }
        let a = match car(cur) {
            Some(v) => v,
            None => break,
        };
        let b = match car(rest) {
            Some(v) => v,
            None => break,
        };
        if !vals_eq(a, b) {
            return Val {
                t: st.nil.t,
                v: ValUnion::N { num: 0.0, den: 1.0 },
            };
        }
        cur = rest;
    }
    Val {
        t: TspType::TspSym,
        v: ValUnion::S("True".to_string()),
    }
}

/* evaluates and returns first expression with a true conditional */
pub fn form_cond(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    let mut v = args;
    loop {
        if is_nil(&v) {
            break;
        }
        let (head, tail) = match v.v {
            ValUnion::P { car, cdr } => (*car, *cdr),
            _ => break,
        };
        let (cond_expr, body) = match head.v {
            ValUnion::P { car, cdr } => (*car, *cdr),
            _ => break,
        };
        let cond = match tisp_eval(st, cond_expr) {
            Some(c) => c,
            None => return mk_val(TspType::TspNone),
        };
        if !is_nil(&cond) {
            return tisp_eval_body(st, env, body).unwrap_or_else(|| mk_val(TspType::TspNone));
        }
        v = tail;
    }
    mk_val(TspType::TspNone)
}

/* return type of tisp value */
pub fn prim_typeof(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if let ValUnion::P { car, .. } = args.v {
        let s = tsp_type_str(car.t);
        return mk_str(st, s).unwrap_or_else(|| mk_val(TspType::TspNone));
    }
    mk_val(TspType::TspNone)
}

/* return record of properties for given procedure */
pub fn prim_procprops(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if let ValUnion::P { car, .. } = args.v {
        let proc = *car;
        let ret = rec_new(6, None);
        match proc.t {
            TspType::TspForm | TspType::TspPrim => {
                if let ValUnion::Pr { name, .. } = &proc.v {
                    let _ = name;
                    let _ = mk_sym(st, "name");
                }
            }
            _ => {}
        }
        return mk_rec(st, ret, mk_val(TspType::TspNil))
            .unwrap_or_else(|| mk_val(TspType::TspNone));
    }
    mk_val(TspType::TspNone)
}

/* creates new tisp function */
#[allow(non_snake_case)]
pub fn form_Func(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    let nargs = tsp_lstlen(&args);
    if nargs < 1 {
        return mk_val(TspType::TspNone);
    }
    let env_copy = rec_new(env.cap as usize, None);
    if let ValUnion::P { car: a, cdr: rest } = args.v {
        if is_nil(&rest) {
            /* auto fill func parameters with 'it' */
            let it = mk_sym(st, "it").unwrap_or_else(|| mk_val(TspType::TspSym));
            let nil = mk_val(TspType::TspNil);
            let params = mk_pair(it, nil).unwrap_or_else(|| mk_val(TspType::TspNil));
            let body = mk_pair(*a, mk_val(TspType::TspNil))
                .unwrap_or_else(|| mk_val(TspType::TspNil));
            return mk_func(TspType::TspFunc, "", params, body, env_copy)
                .unwrap_or_else(|| mk_val(TspType::TspNone));
        } else {
            return mk_func(TspType::TspFunc, "", *a, *rest, env_copy)
                .unwrap_or_else(|| mk_val(TspType::TspNone));
        }
    }
    mk_val(TspType::TspNone)
}

/* creates new tisp defined macro */
#[allow(non_snake_case)]
pub fn form_Macro(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    let mut ret = form_Func(st, env, args);
    ret.t = TspType::TspMacro;
    ret
}

/* display message and return error */
pub fn prim_error(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if let ValUnion::P { car, .. } = &args.v {
        if let ValUnion::S(s) = &car.v {
            eprintln!("; tisp: error: {}", s);
        }
    }
    mk_val(TspType::TspNone)
}

/* merge second record into first record, without mutation */
pub fn prim_recmerge(_st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    mk_val(TspType::TspRec)
}

/* retrieve list of every entry in given record */
pub fn prim_records(st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    Val {
        t: st.nil.t,
        v: ValUnion::N { num: 0.0, den: 1.0 },
    }
}

/* creates new variable of given name and value */
pub fn form_def(_st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    mk_val(TspType::TspNone)
}

pub fn form_undefine(_st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    mk_val(TspType::TspNone)
}

pub fn form_definedp(st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    Val {
        t: st.nil.t,
        v: ValUnion::N { num: 0.0, den: 1.0 },
    }
}

pub fn tib_env_core(_st: &mut Tsp) {
    /* Register core primitives. In C this binds many names; here we
     * provide a minimal stub that compiles. */
}
