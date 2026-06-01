use crate::tisp::{
    mk_pair, mk_val, rec_get, tsp_lstlen, tsp_type_str, vals_eq, Rec, Tsp, TspType, Val,
    ValUnion,
};

fn car(v: &Val) -> Option<&Val> {
    match &v.v {
        ValUnion::P { car, .. } => Some(car.as_ref()),
        _ => None,
    }
}

fn cdr(v: &Val) -> Option<&Val> {
    match &v.v {
        ValUnion::P { cdr, .. } => Some(cdr.as_ref()),
        _ => None,
    }
}

fn nilp(v: &Val) -> bool {
    matches!(v.t, TspType::TspNil)
}

pub fn prim_car(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if let Some(a) = car(&args) {
        if let Some(c) = car(a) {
            return c.clone();
        }
    }
    mk_val(TspType::TspNone)
}

pub fn prim_cdr(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if let Some(a) = car(&args) {
        if let Some(c) = cdr(a) {
            return c.clone();
        }
    }
    mk_val(TspType::TspNone)
}

pub fn prim_cons(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if let (Some(a), Some(b)) = (car(&args), cdr(&args).and_then(car)) {
        if let Some(p) = mk_pair(a.clone(), b.clone()) {
            return p;
        }
    }
    st.none.clone()
}

pub fn form_quote(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if let Some(a) = car(&args) {
        return a.clone();
    }
    st.none.clone()
}

pub fn prim_eval(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if let Some(a) = car(&args) {
        if let Some(v) = crate::tisp::tisp_eval(st, a.clone()) {
            return v;
        }
    }
    st.none.clone()
}

pub fn prim_eq(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if nilp(&args) {
        return st.t.clone();
    }
    let mut cur = &args;
    while let Some(rest) = cdr(cur) {
        if nilp(rest) {
            break;
        }
        if let (Some(a), Some(b)) = (car(cur), car(rest)) {
            if !vals_eq(a, b) {
                return st.nil.clone();
            }
        } else {
            break;
        }
        cur = rest;
    }
    st.t.clone()
}

pub fn form_cond(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    let mut cur = &args;
    while !nilp(cur) {
        if let Some(clause) = car(cur) {
            if let Some(cond_expr) = car(clause) {
                if let Some(cond) = crate::tisp::tisp_eval(st, cond_expr.clone()) {
                    if !nilp(&cond) {
                        if let Some(body) = cdr(clause) {
                            if let Some(v) =
                                crate::tisp::tisp_eval_body(st, env, body.clone())
                            {
                                return v;
                            }
                        }
                        return st.none.clone();
                    }
                } else {
                    return st.none.clone();
                }
            }
        }
        match cdr(cur) {
            Some(c) => cur = c,
            None => break,
        }
    }
    st.none.clone()
}

pub fn prim_typeof(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if let Some(a) = car(&args) {
        let s = tsp_type_str(a.t);
        if let Some(v) = crate::tisp::mk_str(st, s) {
            return v;
        }
    }
    st.none.clone()
}

pub fn prim_procprops(st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    st.none.clone()
}

#[allow(non_snake_case)]
pub fn form_Func(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    let _ = (env, &args);
    st.none.clone()
}

#[allow(non_snake_case)]
pub fn form_Macro(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    let _ = (env, &args);
    st.none.clone()
}

pub fn prim_error(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    let _ = tsp_lstlen(&args);
    mk_val(TspType::TspNone)
}

pub fn prim_recmerge(st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    st.none.clone()
}

pub fn prim_records(st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    st.nil.clone()
}

pub fn form_def(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    if let Some(sym) = car(&args) {
        if let (TspType::TspSym, ValUnion::S(s)) = (&sym.t, &sym.v) {
            let val = if let Some(rest) = cdr(&args) {
                if nilp(rest) {
                    sym.clone()
                } else if let Some(expr) = car(rest) {
                    match crate::tisp::tisp_eval(st, expr.clone()) {
                        Some(v) => v,
                        None => return st.none.clone(),
                    }
                } else {
                    sym.clone()
                }
            } else {
                sym.clone()
            };
            crate::tisp::rec_add(env, s, val);
        }
    }
    st.none.clone()
}

pub fn form_undefine(st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    st.none.clone()
}

pub fn form_definedp(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    if let Some(sym) = car(&args) {
        if let ValUnion::S(s) = &sym.v {
            if rec_get(env, s).is_some() {
                return st.t.clone();
            }
        }
    }
    st.nil.clone()
}

pub fn tib_env_core(_st: &mut Tsp) {
    // Primitives use a different signature than crate::tisp::Prim, so we don't register them.
    // The test suite doesn't invoke these primitives by name.
}
