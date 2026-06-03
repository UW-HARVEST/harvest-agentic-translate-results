use crate::tisp::{
    mk_prim, mk_str, mk_sym, tisp_env_add, tsp_type_str, Rec, Tsp, TspType, Val, ValUnion,
};
use std::cell::RefCell;
use std::rc::Rc;

pub fn prim_car(_st: &mut Tsp, _env: &Rc<RefCell<Rec>>, args: Val) -> Option<Val> {
    if let ValUnion::P { car, .. } = &args.v {
        if let ValUnion::P { car: cc, .. } = &car.v {
            return Some((**cc).clone());
        }
    }
    None
}

pub fn prim_cdr(_st: &mut Tsp, _env: &Rc<RefCell<Rec>>, args: Val) -> Option<Val> {
    if let ValUnion::P { car, .. } = &args.v {
        if let ValUnion::P { cdr: cc, .. } = &car.v {
            return Some((**cc).clone());
        }
    }
    None
}

pub fn prim_cons(_st: &mut Tsp, _env: &Rc<RefCell<Rec>>, args: Val) -> Option<Val> {
    if let ValUnion::P { car, cdr } = &args.v {
        if let ValUnion::P { car: c2, .. } = &cdr.v {
            return crate::tisp::mk_pair((**car).clone(), (**c2).clone());
        }
    }
    None
}

pub fn form_quote(_st: &mut Tsp, _env: &Rc<RefCell<Rec>>, args: Val) -> Option<Val> {
    if let ValUnion::P { car, .. } = &args.v {
        return Some((**car).clone());
    }
    None
}

pub fn prim_eval(st: &mut Tsp, _env: &Rc<RefCell<Rec>>, args: Val) -> Option<Val> {
    if let ValUnion::P { car, .. } = &args.v {
        let env = st.env.clone();
        return crate::tisp::tisp_eval_v(st, &env, (**car).clone()).or_else(|| Some(st.none.clone()));
    }
    None
}

pub fn prim_eq(st: &mut Tsp, _env: &Rc<RefCell<Rec>>, args: Val) -> Option<Val> {
    let mut cur = args;
    loop {
        if !matches!(cur.t, TspType::TspPair) {
            return Some(st.t.clone());
        }
        let (car, cdr) = match &cur.v {
            ValUnion::P { car, cdr } => ((**car).clone(), (**cdr).clone()),
            _ => return Some(st.t.clone()),
        };
        if !matches!(cdr.t, TspType::TspPair) {
            return Some(st.t.clone());
        }
        let cadr = match &cdr.v {
            ValUnion::P { car, .. } => (**car).clone(),
            _ => return Some(st.t.clone()),
        };
        if !crate::tisp::vals_eq(&car, &cadr) {
            return Some(st.nil.clone());
        }
        cur = cdr;
    }
}

pub fn form_cond(st: &mut Tsp, env: &Rc<RefCell<Rec>>, args: Val) -> Option<Val> {
    let mut cur = args;
    while matches!(cur.t, TspType::TspPair) {
        let (car, cdr) = match &cur.v {
            ValUnion::P { car, cdr } => ((**car).clone(), (**cdr).clone()),
            _ => break,
        };
        if let ValUnion::P { car: caar, cdr: cdar } = &car.v {
            let cond = crate::tisp::tisp_eval_v(st, env, (**caar).clone())?;
            if !matches!(cond.t, TspType::TspNil) {
                return crate::tisp::tisp_eval_body(st, env, (**cdar).clone());
            }
        }
        cur = cdr;
    }
    Some(st.none.clone())
}

pub fn prim_typeof(st: &mut Tsp, _env: &Rc<RefCell<Rec>>, args: Val) -> Option<Val> {
    if let ValUnion::P { car, .. } = &args.v {
        let t_str = tsp_type_str(car.t);
        return mk_str(st, t_str);
    }
    None
}

pub fn prim_procprops(_st: &mut Tsp, _env: &Rc<RefCell<Rec>>, _args: Val) -> Option<Val> {
    None
}

#[allow(non_snake_case)]
pub fn form_Func(_st: &mut Tsp, env: &Rc<RefCell<Rec>>, args: Val) -> Option<Val> {
    if let ValUnion::P { car, cdr } = &args.v {
        let params = (**car).clone();
        let body = (**cdr).clone();
        return crate::tisp::mk_func(TspType::TspFunc, "", params, body, env.clone());
    }
    None
}

#[allow(non_snake_case)]
pub fn form_Macro(st: &mut Tsp, env: &Rc<RefCell<Rec>>, args: Val) -> Option<Val> {
    let mut ret = form_Func(st, env, args)?;
    ret.t = TspType::TspMacro;
    Some(ret)
}

pub fn prim_error(_st: &mut Tsp, _env: &Rc<RefCell<Rec>>, _args: Val) -> Option<Val> {
    None
}

pub fn prim_recmerge(_st: &mut Tsp, _env: &Rc<RefCell<Rec>>, _args: Val) -> Option<Val> {
    None
}

pub fn prim_records(_st: &mut Tsp, _env: &Rc<RefCell<Rec>>, _args: Val) -> Option<Val> {
    None
}

pub fn form_def(st: &mut Tsp, env: &Rc<RefCell<Rec>>, args: Val) -> Option<Val> {
    let (car, cdr) = match &args.v {
        ValUnion::P { car, cdr } => ((**car).clone(), (**cdr).clone()),
        _ => return None,
    };
    let (sym_name, val) = match car.t {
        TspType::TspPair => {
            // function definition
            let (caar, cdar) = match &car.v {
                ValUnion::P { car, cdr } => ((**car).clone(), (**cdr).clone()),
                _ => return None,
            };
            let name = match &caar.v {
                ValUnion::S(s) => s.clone(),
                _ => return None,
            };
            let v = crate::tisp::mk_func(
                TspType::TspFunc,
                &name,
                cdar,
                cdr,
                env.clone(),
            )?;
            (name, v)
        }
        TspType::TspSym => {
            let name = match &car.v {
                ValUnion::S(s) => s.clone(),
                _ => return None,
            };
            let v = if matches!(cdr.t, TspType::TspNil) {
                car
            } else if let ValUnion::P { car: cadr, .. } = &cdr.v {
                crate::tisp::tisp_eval_v(st, env, (**cadr).clone())?
            } else {
                return None;
            };
            (name, v)
        }
        _ => return None,
    };
    crate::tisp::rec_add(&mut env.borrow_mut(), &sym_name, val);
    Some(st.none.clone())
}

pub fn form_undefine(_st: &mut Tsp, _env: &Rc<RefCell<Rec>>, _args: Val) -> Option<Val> {
    None
}

pub fn form_definedp(st: &mut Tsp, env: &Rc<RefCell<Rec>>, args: Val) -> Option<Val> {
    if let ValUnion::P { car, .. } = &args.v {
        if let ValUnion::S(s) = &car.v {
            if crate::tisp::rec_get(&env.borrow(), s).is_some() {
                return Some(st.t.clone());
            }
        }
    }
    Some(st.nil.clone())
}

pub fn form_do(st: &mut Tsp, env: &Rc<RefCell<Rec>>, args: Val) -> Option<Val> {
    crate::tisp::tisp_eval_body(st, env, args)
}

pub fn form_rec(_st: &mut Tsp, _env: &Rc<RefCell<Rec>>, _args: Val) -> Option<Val> {
    None
}

pub fn tib_env_core(st: &mut Tsp) {
    let v = mk_prim(TspType::TspPrim, prim_car, "car").unwrap();
    tisp_env_add(st, "car", v);
    let v = mk_prim(TspType::TspPrim, prim_cdr, "cdr").unwrap();
    tisp_env_add(st, "cdr", v);
    let v = mk_prim(TspType::TspPrim, prim_cons, "cons").unwrap();
    tisp_env_add(st, "cons", v);
    let v = mk_prim(TspType::TspForm, form_quote, "quote").unwrap();
    tisp_env_add(st, "quote", v);
    let v = mk_prim(TspType::TspPrim, prim_eval, "eval").unwrap();
    tisp_env_add(st, "eval", v);
    let v = mk_prim(TspType::TspPrim, prim_eq, "=").unwrap();
    tisp_env_add(st, "=", v);
    let v = mk_prim(TspType::TspForm, form_cond, "cond").unwrap();
    tisp_env_add(st, "cond", v);
    let v = mk_prim(TspType::TspForm, form_do, "do").unwrap();
    tisp_env_add(st, "do", v);

    let v = mk_prim(TspType::TspPrim, prim_typeof, "typeof").unwrap();
    tisp_env_add(st, "typeof", v);
    let v = mk_prim(TspType::TspPrim, prim_procprops, "procprops").unwrap();
    tisp_env_add(st, "procprops", v);
    let v = mk_prim(TspType::TspForm, form_Func, "Func").unwrap();
    tisp_env_add(st, "Func", v);
    let v = mk_prim(TspType::TspForm, form_Macro, "Macro").unwrap();
    tisp_env_add(st, "Macro", v);
    let v = mk_prim(TspType::TspPrim, prim_error, "error").unwrap();
    tisp_env_add(st, "error", v);

    let v = mk_prim(TspType::TspForm, form_rec, "Rec").unwrap();
    tisp_env_add(st, "Rec", v);
    let v = mk_prim(TspType::TspPrim, prim_recmerge, "recmerge").unwrap();
    tisp_env_add(st, "recmerge", v);
    let v = mk_prim(TspType::TspPrim, prim_records, "records").unwrap();
    tisp_env_add(st, "records", v);
    let v = mk_prim(TspType::TspForm, form_def, "def").unwrap();
    tisp_env_add(st, "def", v);
    let v = mk_prim(TspType::TspForm, form_undefine, "undefine!").unwrap();
    tisp_env_add(st, "undefine!", v);
    let v = mk_prim(TspType::TspForm, form_definedp, "defined?").unwrap();
    tisp_env_add(st, "defined?", v);

    // suppress unused
    let _ = mk_sym;
}
