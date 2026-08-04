use crate::tisp::{
    mk_func, mk_pair, mk_prim, mk_str, mk_sym, rec_add, rec_clone, rec_get, rec_new,
    stub_prim, tisp_env_add, tisp_eval, tisp_eval_body, tsp_lstlen, tsp_type_str, val_clone,
    vals_eq, Rec, Tsp, TspType, Val, ValUnion,
};

fn nil_val(st: &Tsp) -> Val {
    val_clone(&st.nil)
}

fn none_val(st: &Tsp) -> Val {
    val_clone(&st.none)
}

fn t_val(st: &Tsp) -> Val {
    val_clone(&st.t)
}

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

fn cdr(v: &Val) -> Val {
    if let ValUnion::P { cdr, .. } = &v.v {
        val_clone(cdr)
    } else {
        Val {
            t: TspType::TspNil,
            v: ValUnion::N { num: 0.0, den: 1.0 },
        }
    }
}

pub fn prim_car(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    let a = car(&args);
    if !matches!(a.t, TspType::TspPair) {
        return none_val(st);
    }
    car(&a)
}

pub fn prim_cdr(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    let a = car(&args);
    if !matches!(a.t, TspType::TspPair) {
        return none_val(st);
    }
    cdr(&a)
}

pub fn prim_cons(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    let a = car(&args);
    let b = car(&cdr(&args));
    mk_pair(a, b).unwrap_or(Val {
        t: TspType::TspNil,
        v: ValUnion::N { num: 0.0, den: 1.0 },
    })
}

pub fn form_quote(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    car(&args)
}

pub fn prim_eval(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    let v = car(&args);
    match tisp_eval(st, v) {
        Some(r) => r,
        None => none_val(st),
    }
}

pub fn prim_eq(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if matches!(args.t, TspType::TspNil) {
        return t_val(st);
    }
    let mut cur = args;
    while {
        let cdr_v = cdr(&cur);
        !matches!(cdr_v.t, TspType::TspNil)
    } {
        let a = car(&cur);
        let b = car(&cdr(&cur));
        if !vals_eq(&a, &b) {
            return nil_val(st);
        }
        cur = cdr(&cur);
    }
    t_val(st)
}

pub fn form_cond(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    let mut cur = args;
    while !matches!(cur.t, TspType::TspNil) {
        let pair = car(&cur);
        let cond = car(&pair);
        let evaled = match tisp_eval(st, cond) {
            Some(v) => v,
            None => return none_val(st),
        };
        if !matches!(evaled.t, TspType::TspNil) {
            let body = cdr(&pair);
            return tisp_eval_body(st, env, body).unwrap_or_else(|| none_val(st));
        }
        cur = cdr(&cur);
    }
    none_val(st)
}

pub fn prim_typeof(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    let a = car(&args);
    let s = tsp_type_str(a.t);
    mk_str(st, s).unwrap_or_else(|| none_val(st))
}

pub fn prim_procprops(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    let proc = car(&args);
    let mut ret = rec_new(6, None);
    match proc.t {
        TspType::TspForm | TspType::TspPrim => {
            if let ValUnion::Pr { name, .. } = &proc.v {
                let s = mk_sym(st, name).unwrap_or_else(|| none_val(st));
                rec_add(&mut ret, "name", s);
            }
        }
        TspType::TspFunc | TspType::TspMacro => {
            if let ValUnion::F {
                name, args, body, ..
            } = &proc.v
            {
                let display = if name.is_empty() { "anon" } else { name };
                let s = mk_sym(st, display).unwrap_or_else(|| none_val(st));
                rec_add(&mut ret, "name", s);
                rec_add(&mut ret, "args", val_clone(args));
                rec_add(&mut ret, "body", val_clone(body));
            }
        }
        _ => return none_val(st),
    }
    let rec_val = Val {
        t: TspType::TspRec,
        v: ValUnion::R(ret),
    };
    rec_val
}

#[allow(non_snake_case)]
pub fn form_Func(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    let lst_len = tsp_lstlen(&args);
    if lst_len < 1 {
        return none_val(st);
    }
    let (params, body) = if matches!(cdr(&args).t, TspType::TspNil) {
        // auto-fill
        let it = match mk_sym(st, "it") {
            Some(v) => v,
            None => return none_val(st),
        };
        let nil = val_clone(&st.nil);
        let p = match mk_pair(it, nil) {
            Some(v) => v,
            None => return none_val(st),
        };
        (p, args)
    } else {
        (car(&args), cdr(&args))
    };
    mk_func(TspType::TspFunc, "", params, body, rec_clone(env)).unwrap_or_else(|| none_val(st))
}

#[allow(non_snake_case)]
pub fn form_Macro(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    let mut ret = form_Func(st, env, args);
    if matches!(ret.t, TspType::TspFunc) {
        ret.t = TspType::TspMacro;
    }
    ret
}

pub fn prim_error(st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    none_val(st)
}

pub fn prim_recmerge(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    let a = car(&args);
    let b = car(&cdr(&args));
    if !matches!(a.t, TspType::TspRec) || !matches!(b.t, TspType::TspRec) {
        return none_val(st);
    }
    let next = if let ValUnion::R(r) = &a.v {
        Some(Box::new(rec_clone(r)))
    } else {
        None
    };
    let cap = if let ValUnion::R(r) = &b.v {
        (r.size as usize) * 2
    } else {
        4
    };
    let mut merged = rec_new(cap.max(4), next);
    if let ValUnion::R(r) = &b.v {
        let mut cur = Some(r);
        while let Some(rec) = cur {
            for it in rec.items.iter() {
                if !it.key.is_empty() {
                    rec_add(&mut merged, &it.key, val_clone(&it.val));
                }
            }
            cur = rec.next.as_deref();
        }
    }
    Val {
        t: TspType::TspRec,
        v: ValUnion::R(merged),
    }
}

pub fn prim_records(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    let a = car(&args);
    if !matches!(a.t, TspType::TspRec) {
        return none_val(st);
    }
    let mut ret = nil_val(st);
    if let ValUnion::R(r) = &a.v {
        let mut cur = Some(r);
        while let Some(rec) = cur {
            for it in rec.items.iter() {
                if !it.key.is_empty() {
                    let key_sym = mk_sym(st, &it.key).unwrap_or_else(|| none_val(st));
                    let entry = mk_pair(key_sym, val_clone(&it.val))
                        .unwrap_or_else(|| none_val(st));
                    ret = mk_pair(entry, ret).unwrap_or_else(|| none_val(st));
                }
            }
            cur = rec.next.as_deref();
        }
    }
    ret
}

pub fn form_def(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    let head = car(&args);
    match head.t {
        TspType::TspPair => {
            let sym = car(&head);
            if !matches!(sym.t, TspType::TspSym) {
                return none_val(st);
            }
            let name = if let ValUnion::S(s) = &sym.v {
                s.clone()
            } else {
                return none_val(st);
            };
            let f_args = cdr(&head);
            let f_body = cdr(&args);
            let val = match mk_func(TspType::TspFunc, &name, f_args, f_body, rec_clone(env)) {
                Some(v) => v,
                None => return none_val(st),
            };
            rec_add(env, &name, val_clone(&val));
            tisp_env_add(st, &name, val);
            none_val(st)
        }
        TspType::TspSym => {
            let name = if let ValUnion::S(s) = &head.v {
                s.clone()
            } else {
                return none_val(st);
            };
            let val = if matches!(cdr(&args).t, TspType::TspNil) {
                val_clone(&head)
            } else {
                match tisp_eval(st, car(&cdr(&args))) {
                    Some(v) => v,
                    None => return none_val(st),
                }
            };
            // set name on funcs/macros
            let mut val = val;
            if matches!(val.t, TspType::TspFunc | TspType::TspMacro) {
                if let ValUnion::F { name: n, .. } = &mut val.v {
                    if n.is_empty() {
                        *n = name.clone();
                    }
                }
            }
            rec_add(env, &name, val_clone(&val));
            tisp_env_add(st, &name, val);
            none_val(st)
        }
        _ => none_val(st),
    }
}

pub fn form_undefine(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    let head = car(&args);
    if !matches!(head.t, TspType::TspSym) {
        return none_val(st);
    }
    let name = if let ValUnion::S(s) = &head.v {
        s.clone()
    } else {
        return none_val(st);
    };
    // remove from env chain
    fn remove_from_chain(rec: &mut Rec, key: &str) -> bool {
        for it in rec.items.iter_mut() {
            if it.key == key {
                it.key = String::new();
                return true;
            }
        }
        if let Some(next) = rec.next.as_deref_mut() {
            return remove_from_chain(next, key);
        }
        false
    }
    let _ = remove_from_chain(env, &name);
    let _ = remove_from_chain(&mut st.env, &name);
    none_val(st)
}

pub fn form_definedp(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    let head = car(&args);
    if !matches!(head.t, TspType::TspSym) {
        return nil_val(st);
    }
    let name = if let ValUnion::S(s) = &head.v {
        s.clone()
    } else {
        return nil_val(st);
    };
    if rec_get(env, &name).is_some() || rec_get(&st.env, &name).is_some() {
        t_val(st)
    } else {
        nil_val(st)
    }
}

pub fn tib_env_core(st: &mut Tsp) {
    let prims_form: &[(&str, TspType)] = &[
        ("car", TspType::TspPrim),
        ("cdr", TspType::TspPrim),
        ("cons", TspType::TspPrim),
        ("quote", TspType::TspForm),
        ("eval", TspType::TspPrim),
        ("=", TspType::TspPrim),
        ("cond", TspType::TspForm),
        ("do", TspType::TspForm),
        ("typeof", TspType::TspPrim),
        ("procprops", TspType::TspPrim),
        ("Func", TspType::TspForm),
        ("Macro", TspType::TspForm),
        ("error", TspType::TspPrim),
        ("Rec", TspType::TspForm),
        ("recmerge", TspType::TspPrim),
        ("records", TspType::TspPrim),
        ("def", TspType::TspForm),
        ("undefine!", TspType::TspForm),
        ("defined?", TspType::TspForm),
    ];
    for (name, t) in prims_form {
        let v = mk_prim(*t, stub_prim, name).unwrap();
        tisp_env_add(st, name, v);
    }
    // Suppress unused warnings on imports
    let _ = (
        mk_func as fn(TspType, &str, Val, Val, Rec) -> Option<Val>,
        mk_pair as fn(Val, Val) -> Option<Val>,
        mk_str as fn(&mut Tsp, &str) -> Option<Val>,
        mk_sym as fn(&mut Tsp, &str) -> Option<Val>,
        rec_add as fn(&mut Rec, &str, Val),
        rec_clone as fn(&Rec) -> Rec,
        rec_get as fn(&Rec, &str) -> Option<Val>,
        rec_new as fn(usize, Option<Box<Rec>>) -> Rec,
        tisp_eval as fn(&mut Tsp, Val) -> Option<Val>,
        tisp_eval_body as fn(&mut Tsp, &mut Rec, Val) -> Option<Val>,
        tsp_lstlen as fn(&Val) -> i32,
        tsp_type_str as fn(TspType) -> &'static str,
        vals_eq as fn(&Val, &Val) -> bool,
    );
    let _ = (val_clone(&st.nil),);
}
