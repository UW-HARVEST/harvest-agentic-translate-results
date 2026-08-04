use crate::tisp::{
    mk_func, mk_pair, mk_prim, mk_rec, mk_sym, nil_val, nilp, none_val, pairp, rec_add, rec_get,
    rec_new, tisp_env_add, tisp_eval_body, tisp_eval_with_env, tsp_lstlen, tsp_type_str, val_car,
    val_cdr, val_str, vals_eq, warn, Rec, Tsp, TspType, Val, ValUnion, TSP_REC_FACTOR,
};

pub fn prim_car(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if tsp_lstlen(&args) < 1 {
        warn("car: expected at least 1 argument");
        return none_val();
    }
    if let Some(first) = val_car(&args) {
        if !pairp(first) {
            warn(&format!("car: expected Pair, received {}", tsp_type_str(first.t)));
            return none_val();
        }
        if let Some(c) = val_car(first) {
            return c.clone();
        }
    }
    none_val()
}

pub fn prim_cdr(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if tsp_lstlen(&args) < 1 {
        warn("cdr: expected at least 1 argument");
        return none_val();
    }
    if let Some(first) = val_car(&args) {
        if !pairp(first) {
            warn(&format!("cdr: expected Pair, received {}", tsp_type_str(first.t)));
            return none_val();
        }
        if let Some(c) = val_cdr(first) {
            return c.clone();
        }
    }
    none_val()
}

pub fn prim_cons(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if tsp_lstlen(&args) != 2 {
        warn("cons: expected 2 arguments");
        return none_val();
    }
    let a = val_car(&args).cloned().unwrap_or_else(nil_val);
    let cdr = val_cdr(&args).cloned().unwrap_or_else(nil_val);
    let b = val_car(&cdr).cloned().unwrap_or_else(nil_val);
    mk_pair(a, b).unwrap_or_else(nil_val)
}

pub fn form_quote(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if tsp_lstlen(&args) < 1 {
        warn("quote: expected 1 argument");
        return none_val();
    }
    val_car(&args).cloned().unwrap_or_else(nil_val)
}

pub fn prim_eval(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if tsp_lstlen(&args) != 1 {
        warn("eval: expected 1 argument");
        return none_val();
    }
    let v = val_car(&args).cloned().unwrap_or_else(nil_val);
    let mut env = std::mem::replace(&mut st.env, rec_new(1, None));
    let result = tisp_eval_with_env(st, &mut env, v).unwrap_or_else(none_val);
    st.env = env;
    result
}

pub fn prim_eq(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if nilp(&args) {
        return st.t.clone();
    }
    let mut cur = &args;
    while let Some(rest) = val_cdr(cur) {
        if nilp(rest) {
            break;
        }
        if let (Some(a), Some(b)) = (val_car(cur), val_car(rest)) {
            if !vals_eq(a, b) {
                return st.nil.clone();
            }
        }
        cur = rest;
    }
    st.t.clone()
}

pub fn form_cond(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    let mut cur = args;
    while pairp(&cur) {
        let (clause_opt, rest) = if let ValUnion::P { car, cdr } = cur.v {
            (Some(*car), *cdr)
        } else {
            (None, nil_val())
        };
        if let Some(clause) = clause_opt {
            if let ValUnion::P { car: cond, cdr: body } = clause.v {
                let cond_val = match tisp_eval_with_env(st, env, *cond) {
                    Some(v) => v,
                    None => return none_val(),
                };
                if !nilp(&cond_val) {
                    return tisp_eval_body(st, env, *body).unwrap_or_else(none_val);
                }
            }
        }
        cur = rest;
    }
    st.none.clone()
}

pub fn prim_typeof(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if tsp_lstlen(&args) != 1 {
        warn("typeof: expected 1 argument");
        return none_val();
    }
    let first = val_car(&args).cloned().unwrap_or_else(nil_val);
    let type_str = tsp_type_str(first.t);
    mk_sym(st, type_str).unwrap_or_else(nil_val)
}

pub fn prim_procprops(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if tsp_lstlen(&args) != 1 {
        warn("procprops: expected 1 argument");
        return none_val();
    }
    let proc = val_car(&args).cloned().unwrap_or_else(nil_val);
    let mut ret = rec_new(6, None);
    match proc.t {
        TspType::TspForm | TspType::TspPrim => {
            if let ValUnion::Pr { name, .. } = &proc.v {
                let name_clone = name.clone();
                let sym = mk_sym(st, &name_clone).unwrap_or_else(nil_val);
                rec_add(&mut ret, "name", sym);
            }
        }
        TspType::TspFunc | TspType::TspMacro => {
            if let ValUnion::F { name, args, body, .. } = &proc.v {
                let name_str = if name.is_empty() { "anon" } else { name.as_str() };
                let sym = mk_sym(st, name_str).unwrap_or_else(nil_val);
                rec_add(&mut ret, "name", sym);
                rec_add(&mut ret, "args", (**args).clone());
                rec_add(&mut ret, "body", (**body).clone());
            }
        }
        _ => {
            warn(&format!(
                "procprops: expected Proc, received '{}'",
                tsp_type_str(proc.t)
            ));
            return none_val();
        }
    }
    Val { t: TspType::TspRec, v: ValUnion::R(ret) }
}

#[allow(non_snake_case)]
pub fn form_Func(_st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    if tsp_lstlen(&args) < 1 {
        warn("Func: expected at least 1 argument");
        return none_val();
    }
    let cdr = val_cdr(&args).cloned().unwrap_or_else(nil_val);
    let (params, body) = if nilp(&cdr) {
        // auto-fill params with 'it'
        let it_sym = Val { t: TspType::TspSym, v: ValUnion::S("it".to_string()) };
        let params = mk_pair(it_sym, nil_val()).unwrap_or_else(nil_val);
        (params, args.clone())
    } else {
        let p = val_car(&args).cloned().unwrap_or_else(nil_val);
        (p, cdr)
    };
    mk_func(TspType::TspFunc, "", params, body, env.clone()).unwrap_or_else(nil_val)
}

#[allow(non_snake_case)]
pub fn form_Macro(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    let mut ret = form_Func(st, env, args);
    ret.t = TspType::TspMacro;
    ret
}

pub fn prim_error(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if tsp_lstlen(&args) < 2 {
        warn("error: expected at least 2 arguments");
        return none_val();
    }
    if let Some(first) = val_car(&args) {
        if !matches!(first.t, TspType::TspSym) {
            warn("error: expected Sym");
            return none_val();
        }
        eprint!("; tisp: error: {}: ", val_str(first));
    }
    let mut cur = val_cdr(&args).cloned().unwrap_or_else(nil_val);
    while pairp(&cur) {
        if let Some(c) = val_car(&cur) {
            eprint!("{}", crate::tisp::tisp_print_to_string(c));
        }
        cur = val_cdr(&cur).cloned().unwrap_or_else(nil_val);
    }
    eprintln!();
    none_val()
}

pub fn prim_recmerge(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if tsp_lstlen(&args) != 2 {
        warn("recmerge: expected 2 arguments");
        return none_val();
    }
    let first = val_car(&args).cloned().unwrap_or_else(nil_val);
    let cdr = val_cdr(&args).cloned().unwrap_or_else(nil_val);
    let second = val_car(&cdr).cloned().unwrap_or_else(nil_val);
    if !matches!(first.t, TspType::TspRec) || !matches!(second.t, TspType::TspRec) {
        warn("recmerge: expected Rec");
        return none_val();
    }
    let r1 = if let ValUnion::R(r) = first.v { r } else { return none_val(); };
    let r2 = if let ValUnion::R(r) = second.v { r } else { return none_val(); };
    let mut new_rec = rec_new((r2.size as usize) * TSP_REC_FACTOR + 1, Some(Box::new(r1)));
    let mut current: Option<&Rec> = Some(&r2);
    while let Some(rec) = current {
        for entry in rec.items.iter() {
            if !entry.key.is_empty() {
                let k = entry.key.clone();
                rec_add(&mut new_rec, &k, entry.val.clone());
            }
        }
        current = rec.next.as_deref();
    }
    Val { t: TspType::TspRec, v: ValUnion::R(new_rec) }
}

pub fn prim_records(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if tsp_lstlen(&args) != 1 {
        warn("records: expected 1 argument");
        return none_val();
    }
    let first = val_car(&args).cloned().unwrap_or_else(nil_val);
    if !matches!(first.t, TspType::TspRec) {
        warn("records: expected Rec");
        return none_val();
    }
    let mut ret = st.nil.clone();
    if let ValUnion::R(r) = &first.v {
        let mut current: Option<&Rec> = Some(r);
        while let Some(rec) = current {
            for entry in rec.items.iter() {
                if !entry.key.is_empty() {
                    let sym = mk_sym(st, &entry.key).unwrap_or_else(nil_val);
                    let pair = mk_pair(sym, entry.val.clone()).unwrap_or_else(nil_val);
                    ret = mk_pair(pair, ret).unwrap_or_else(nil_val);
                }
            }
            current = rec.next.as_deref();
        }
    }
    ret
}

pub fn form_def(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    if tsp_lstlen(&args) < 1 {
        warn("def: expected at least 1 argument");
        return none_val();
    }
    let first = val_car(&args).cloned().unwrap_or_else(nil_val);
    let rest = val_cdr(&args).cloned().unwrap_or_else(nil_val);
    let (sym, val);
    if matches!(first.t, TspType::TspPair) {
        // function definition
        let name_v = val_car(&first).cloned().unwrap_or_else(nil_val);
        if !matches!(name_v.t, TspType::TspSym) {
            warn("def: expected symbol for function name");
            return none_val();
        }
        sym = name_v.clone();
        let params = val_cdr(&first).cloned().unwrap_or_else(nil_val);
        let name_str = val_str(&name_v).to_string();
        val = mk_func(TspType::TspFunc, &name_str, params, rest, env.clone())
            .unwrap_or_else(nil_val);
    } else if matches!(first.t, TspType::TspSym) {
        sym = first.clone();
        if nilp(&rest) {
            val = first;
        } else {
            let expr = val_car(&rest).cloned().unwrap_or_else(nil_val);
            val = match tisp_eval_with_env(st, env, expr) {
                Some(v) => v,
                None => return none_val(),
            };
        }
    } else {
        warn("def: incorrect format, no variable name found");
        return none_val();
    }
    let key = val_str(&sym).to_string();
    rec_add(env, &key, val.clone());
    // also add to global env
    tisp_env_add(st, &key, val);
    st.none.clone()
}

pub fn form_undefine(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    if tsp_lstlen(&args) < 1 {
        warn("undefine!: expected at least 1 argument");
        return none_val();
    }
    let first = val_car(&args).cloned().unwrap_or_else(nil_val);
    if !matches!(first.t, TspType::TspSym) {
        warn("undefine!: expected Sym");
        return none_val();
    }
    let key = val_str(&first).to_string();
    if undefine_in(env, &key) || undefine_in(&mut st.env, &key) {
        return st.none.clone();
    }
    warn(&format!("undefine!: could not find symbol {} to undefine", key));
    none_val()
}

fn undefine_in(rec: &mut Rec, key: &str) -> bool {
    let mut current: Option<&mut Rec> = Some(rec);
    while let Some(r) = current {
        if r.cap > 0 {
            let cap = r.cap as usize;
            let mut i = (crate::tisp::hash(key) as usize) % cap;
            loop {
                let e = &r.items[i];
                if e.key.is_empty() {
                    break;
                }
                if e.key == key {
                    r.items[i].key = String::new();
                    r.size -= 1;
                    return true;
                }
                i += 1;
                if i == cap {
                    i = 0;
                }
            }
        }
        current = r.next.as_deref_mut();
    }
    false
}

pub fn form_definedp(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    if tsp_lstlen(&args) < 1 {
        warn("defined?: expected at least 1 argument");
        return none_val();
    }
    let first = val_car(&args).cloned().unwrap_or_else(nil_val);
    if !matches!(first.t, TspType::TspSym) {
        warn("defined?: expected Sym");
        return none_val();
    }
    let key = val_str(&first).to_string();
    if rec_get(env, &key).is_some() || rec_get(&st.env, &key).is_some() {
        st.t.clone()
    } else {
        st.nil.clone()
    }
}

#[allow(non_snake_case)]
pub fn form_Rec(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    // Evaluate values of pair clauses
    // Build a record where each entry is a pair (key val) where val is evaluated
    let mut new_rec = rec_new(8, None);
    let mut cur = args;
    while pairp(&cur) {
        let (item_opt, rest) = if let ValUnion::P { car, cdr } = cur.v {
            (Some(*car), *cdr)
        } else {
            (None, nil_val())
        };
        if let Some(item) = item_opt {
            if pairp(&item) {
                let key = val_car(&item).cloned().unwrap_or_else(nil_val);
                let body = val_cdr(&item).cloned().unwrap_or_else(nil_val);
                if matches!(key.t, TspType::TspSym | TspType::TspStr) {
                    let key_str = val_str(&key).to_string();
                    let val_expr = val_car(&body).cloned().unwrap_or_else(nil_val);
                    let val = tisp_eval_with_env(st, env, val_expr).unwrap_or_else(none_val);
                    rec_add(&mut new_rec, &key_str, val);
                }
            } else if matches!(item.t, TspType::TspSym) {
                let key_str = val_str(&item).to_string();
                let val = tisp_eval_with_env(st, env, item).unwrap_or_else(none_val);
                rec_add(&mut new_rec, &key_str, val);
            }
        }
        cur = rest;
    }
    Val { t: TspType::TspRec, v: ValUnion::R(new_rec) }
}

pub fn tib_env_core(st: &mut Tsp) {
    add_prim(st, "car", TspType::TspPrim);
    add_prim(st, "cdr", TspType::TspPrim);
    add_prim(st, "cons", TspType::TspPrim);
    add_prim(st, "quote", TspType::TspForm);
    add_prim(st, "eval", TspType::TspPrim);
    add_prim(st, "=", TspType::TspPrim);
    add_prim(st, "cond", TspType::TspForm);
    add_prim(st, "do", TspType::TspForm);

    add_prim(st, "typeof", TspType::TspPrim);
    add_prim(st, "procprops", TspType::TspPrim);
    add_prim(st, "Func", TspType::TspForm);
    add_prim(st, "Macro", TspType::TspForm);
    add_prim(st, "error", TspType::TspPrim);

    add_prim(st, "Rec", TspType::TspForm);
    add_prim(st, "recmerge", TspType::TspPrim);
    add_prim(st, "records", TspType::TspPrim);
    add_prim(st, "def", TspType::TspForm);
    add_prim(st, "undefine!", TspType::TspForm);
    add_prim(st, "defined?", TspType::TspForm);
}

fn add_prim(st: &mut Tsp, name: &str, t: TspType) {
    let v = mk_prim(t, dummy_prim, name).unwrap_or_else(nil_val);
    tisp_env_add(st, name, v);
}

pub fn dummy_prim(_st: Tsp, _env: Rec, _args: Val) -> Val {
    none_val()
}
