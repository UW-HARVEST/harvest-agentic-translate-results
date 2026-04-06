use crate::tisp::*;

pub fn prim_car(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if tsp_arg_num_check_pub(&args, "car", 1).is_none() { return mk_error(); }
    if tsp_arg_type_check_pub(car_pub(&args), "car", TspType::TspPair as u32).is_none() { return mk_error(); }
    clone_val_pub(car_pub(car_pub(&args)))
}
pub fn prim_cdr(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if tsp_arg_num_check_pub(&args, "cdr", 1).is_none() { return mk_error(); }
    if tsp_arg_type_check_pub(car_pub(&args), "cdr", TspType::TspPair as u32).is_none() { return mk_error(); }
    clone_val_pub(cdr_pub(car_pub(&args)))
}
pub fn prim_cons(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if tsp_arg_num_check_pub(&args, "cons", 2).is_none() { return mk_error(); }
    let a = clone_val_pub(car_pub(&args));
    let b = clone_val_pub(car_pub(cdr_pub(&args)));
    mk_pair(a, b)
}
pub fn form_quote(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if tsp_arg_num_check_pub(&args, "quote", 1).is_none() { return mk_error(); }
    clone_val_pub(car_pub(&args))
}
pub fn prim_eval(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if tsp_arg_num_check_pub(&args, "eval", 1).is_none() { return mk_error(); }
    let v = clone_val_pub(car_pub(&args));
    // eval uses st.env (global)
    match tisp_eval(st, v) {
        Some(r) => r,
        None => mk_none_pub(),
    }
}
pub fn prim_eq(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if nilp_pub(&args) { return clone_val_pub(&st.t); }
    let mut cur = &args;
    while !nilp_pub(cdr_pub(cur)) {
        if !vals_eq(car_pub(cur), car_pub(cdr_pub(cur))) { return mk_nil_pub(); }
        cur = cdr_pub(cur);
    }
    clone_val_pub(&st.t)
}
pub fn form_cond(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    let mut cur = &args;
    while !nilp_pub(cur) {
        let pair = car_pub(cur);
        let cond_expr = clone_val_pub(car_pub(pair));
        match tisp_eval_with_env_pub(st, env, cond_expr) {
            None => return mk_error(),
            Some(cond) => {
                if !nilp_pub(&cond) {
                    let body = clone_val_pub(cdr_pub(pair));
                    match tisp_eval_body_pub(st, env, body) {
                        Some(r) => return r,
                        None => return mk_error(),
                    }
                }
            }
        }
        cur = cdr_pub(cur);
    }
    mk_none_pub()
}
pub fn prim_typeof(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if tsp_arg_num_check_pub(&args, "typeof", 1).is_none() { return mk_error(); }
    mk_str(st, tsp_type_str(car_pub(&args).t))
}
pub fn prim_procprops(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if tsp_arg_num_check_pub(&args, "procprops", 1).is_none() { return mk_error(); }
    let proc_val = car_pub(&args);
    let mut ret = rec_new(6, None);
    match proc_val.t {
        TspType::TspForm | TspType::TspPrim => {
            if let ValUnion::Pr { ref name, .. } = proc_val.v {
                let sym = mk_sym(st, name);
                rec_add(&mut ret, "name", sym);
            }
        }
        TspType::TspFunc | TspType::TspMacro => {
            if let ValUnion::F { ref name, ref args, ref body, .. } = proc_val.v {
                let n = if name.is_empty() { "anon" } else { name.as_str() };
                let sym = mk_sym(st, n);
                rec_add(&mut ret, "name", sym);
                rec_add(&mut ret, "args", clone_val_pub(args));
                rec_add(&mut ret, "body", clone_val_pub(body));
            }
        }
        _ => {
            eprintln!("; tisp: error: procprops: expected Proc, received '{}'", tsp_type_str(proc_val.t));
            return mk_error();
        }
    }
    mk_rec(st, ret, mk_nil_pub())
}
#[allow(non_snake_case)]
pub fn form_Func(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    if tsp_arg_min_check_pub(&args, "Func", 1).is_none() { return mk_error(); }
    let (params, body) = if nilp_pub(cdr_pub(&args)) {
        let p = mk_pair(mk_sym(st, "it"), mk_nil_pub());
        (p, clone_val_pub(&args))
    } else {
        (clone_val_pub(car_pub(&args)), clone_val_pub(cdr_pub(&args)))
    };
    mk_func(TspType::TspFunc, "", params, body, clone_rec_pub(env))
}
#[allow(non_snake_case)]
pub fn form_Macro(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    if tsp_arg_min_check_pub(&args, "Macro", 1).is_none() { return mk_error(); }
    let mut ret = form_Func(st, env, args);
    if is_error(&ret) { return ret; }
    ret.t = TspType::TspMacro;
    ret
}
pub fn prim_error(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if tsp_arg_min_check_pub(&args, "error", 2).is_none() { return mk_error(); }
    if tsp_arg_type_check_pub(car_pub(&args), "error", TspType::TspSym as u32).is_none() { return mk_error(); }
    eprint!("; tisp: error: {}: ", sym_str_pub(car_pub(&args)));
    let mut cur = cdr_pub(&args);
    while !nilp_pub(cur) {
        eprint!("{}", val_to_string_pub(car_pub(cur)));
        cur = cdr_pub(cur);
    }
    eprintln!();
    mk_error()
}
pub fn prim_recmerge(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if tsp_arg_num_check_pub(&args, "recmerge", 2).is_none() { return mk_error(); }
    if tsp_arg_type_check_pub(car_pub(&args), "recmerge", TspType::TspRec as u32).is_none() { return mk_error(); }
    if tsp_arg_type_check_pub(car_pub(cdr_pub(&args)), "recmerge", TspType::TspRec as u32).is_none() { return mk_error(); }
    let first = car_pub(&args);
    let second = car_pub(cdr_pub(&args));
    if let (ValUnion::R(r1), ValUnion::R(r2)) = (&first.v, &second.v) {
        let cap = (r2.size as usize) * TSP_REC_FACTOR;
        let mut new_rec = rec_new(if cap > 0 { cap } else { 1 }, Some(Box::new(clone_rec_pub(r1))));
        let mut r = Some(r2);
        while let Some(cur) = r {
            for e in &cur.items {
                if !e.key.is_empty() { rec_add(&mut new_rec, &e.key, clone_val_pub(&e.val)); }
            }
            r = cur.next.as_deref();
        }
        Val { t: TspType::TspRec, v: ValUnion::R(new_rec) }
    } else { mk_error() }
}
pub fn prim_records(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if tsp_arg_num_check_pub(&args, "records", 1).is_none() { return mk_error(); }
    if tsp_arg_type_check_pub(car_pub(&args), "records", TspType::TspRec as u32).is_none() { return mk_error(); }
    let mut ret = mk_nil_pub();
    if let ValUnion::R(ref r) = car_pub(&args).v {
        let mut rec = Some(r);
        while let Some(cur) = rec {
            for e in &cur.items {
                if !e.key.is_empty() {
                    let sym = mk_sym(st, &e.key);
                    let entry = mk_pair(sym, clone_val_pub(&e.val));
                    ret = mk_pair(entry, ret);
                }
            }
            rec = cur.next.as_deref();
        }
    }
    ret
}
pub fn form_def(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    if tsp_arg_min_check_pub(&args, "def", 1).is_none() { return mk_error(); }
    let first = car_pub(&args);
    let (sym_name, val) = if first.t == TspType::TspPair {
        let sym = car_pub(first);
        if sym.t != TspType::TspSym {
            eprintln!("; tisp: error: def: expected symbol for function name, received '{}'", tsp_type_str(sym.t));
            return mk_error();
        }
        let name = sym_str_pub(sym).to_string();
        let fargs = clone_val_pub(cdr_pub(first));
        let body = clone_val_pub(cdr_pub(&args));
        let f = mk_func(TspType::TspFunc, &name, fargs, body, clone_rec_pub(env));
        (name, f)
    } else if first.t == TspType::TspSym {
        let name = sym_str_pub(first).to_string();
        if nilp_pub(cdr_pub(&args)) {
            (name.clone(), clone_val_pub(first))
        } else {
            let expr = clone_val_pub(car_pub(cdr_pub(&args)));
            match tisp_eval_with_env_pub(st, env, expr) {
                Some(v) => (name, v),
                None => return mk_error(),
            }
        }
    } else {
        eprintln!("; tisp: error: def: incorrect format, no variable name found");
        return mk_error();
    };
    let mut val = val;
    if (val.t == TspType::TspFunc || val.t == TspType::TspMacro) {
        if let ValUnion::F { ref name, .. } = val.v {
            if name.is_empty() {
                if let ValUnion::F { ref mut name, .. } = val.v {
                    *name = sym_name.clone();
                }
            }
        }
    }
    rec_add(env, &sym_name, val);
    mk_none_pub()
}
pub fn form_undefine(_st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    if tsp_arg_min_check_pub(&args, "undefine!", 1).is_none() { return mk_error(); }
    if tsp_arg_type_check_pub(car_pub(&args), "undefine!", TspType::TspSym as u32).is_none() { return mk_error(); }
    let key = sym_str_pub(car_pub(&args)).to_string();
    // Walk the env chain looking for the key
    fn find_and_remove(rec: &mut Rec, key: &str) -> bool {
        if let Some(e) = entry_get(rec, key) {
            if !e.key.is_empty() {
                // Found it - need to clear the key
                let idx = rec.items.iter().position(|e| e.key == key).unwrap();
                rec.items[idx].key = String::new();
                return true;
            }
        }
        if let Some(ref mut next) = rec.next {
            return find_and_remove(next, key);
        }
        false
    }
    if find_and_remove(env, &key) {
        mk_none_pub()
    } else {
        eprintln!("; tisp: error: undefine!: could not find symbol {} to undefine", key);
        mk_error()
    }
}
pub fn form_definedp(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    if tsp_arg_min_check_pub(&args, "defined?", 1).is_none() { return mk_error(); }
    if tsp_arg_type_check_pub(car_pub(&args), "defined?", TspType::TspSym as u32).is_none() { return mk_error(); }
    let key = sym_str_pub(car_pub(&args));
    if rec_get(env, key).is_some() { clone_val_pub(&st.t) } else { mk_nil_pub() }
}

// Helper for do form - just calls tisp_eval_body
fn form_do(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    match tisp_eval_body_pub(st, env, args) {
        Some(v) => v,
        None => mk_error(),
    }
}

// Helper for Rec form
fn form_rec(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    mk_rec(st, clone_rec_pub(env), args)
}

pub fn tib_env_core(st: &mut Tsp) {
    tisp_env_add(st, "car", mk_prim(TspType::TspPrim, prim_car, "car"));
    tisp_env_add(st, "cdr", mk_prim(TspType::TspPrim, prim_cdr, "cdr"));
    tisp_env_add(st, "cons", mk_prim(TspType::TspPrim, prim_cons, "cons"));
    tisp_env_add(st, "quote", mk_prim(TspType::TspForm, form_quote, "quote"));
    tisp_env_add(st, "eval", mk_prim(TspType::TspPrim, prim_eval, "eval"));
    tisp_env_add(st, "=", mk_prim(TspType::TspPrim, prim_eq, "="));
    tisp_env_add(st, "cond", mk_prim(TspType::TspForm, form_cond, "cond"));
    tisp_env_add(st, "do", mk_prim(TspType::TspForm, form_do, "do"));
    tisp_env_add(st, "typeof", mk_prim(TspType::TspPrim, prim_typeof, "typeof"));
    tisp_env_add(st, "procprops", mk_prim(TspType::TspPrim, prim_procprops, "procprops"));
    tisp_env_add(st, "Func", mk_prim(TspType::TspForm, form_Func, "Func"));
    tisp_env_add(st, "Macro", mk_prim(TspType::TspForm, form_Macro, "Macro"));
    tisp_env_add(st, "error", mk_prim(TspType::TspPrim, prim_error, "error"));
    tisp_env_add(st, "Rec", mk_prim(TspType::TspForm, form_rec, "Rec"));
    tisp_env_add(st, "recmerge", mk_prim(TspType::TspPrim, prim_recmerge, "recmerge"));
    tisp_env_add(st, "records", mk_prim(TspType::TspPrim, prim_records, "records"));
    tisp_env_add(st, "def", mk_prim(TspType::TspForm, form_def, "def"));
    tisp_env_add(st, "undefine!", mk_prim(TspType::TspForm, form_undefine, "undefine!"));
    tisp_env_add(st, "defined?", mk_prim(TspType::TspForm, form_definedp, "defined?"));
}
