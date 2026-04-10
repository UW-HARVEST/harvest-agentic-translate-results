use crate::tisp::*;

fn tsp_arg_num_check(args: &Val, name: &str, nargs: i32) -> bool {
    if nargs > -1 && tsp_lstlen(args) != nargs {
        eprintln!("; tisp: error: {}: expected {} argument{}, received {}",
            name, nargs, if nargs > 1 { "s" } else { "" }, tsp_lstlen(args));
        return false;
    }
    true
}

fn tsp_arg_min_check(args: &Val, name: &str, nargs: i32) -> bool {
    if tsp_lstlen(args) < nargs {
        eprintln!("; tisp: error: {}: expected at least {} argument{}, received {}",
            name, nargs, if nargs > 1 { "s" } else { "" }, tsp_lstlen(args));
        return false;
    }
    true
}

fn tsp_arg_type_check(arg: &Val, name: &str, type_bits: u32) -> bool {
    if (arg.t as u32) & type_bits == 0 {
        eprintln!("; tisp: error: {}: expected {}, received {}",
            name, tsp_type_str_bits(type_bits), tsp_type_str(arg.t));
        return false;
    }
    true
}

pub fn prim_car(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if !tsp_arg_num_check(&args, "car", 1) { return mk_err(); }
    if !tsp_arg_type_check(car(&args), "car", TspType::TspPair as u32) { return mk_err(); }
    clone_val(car(car(&args)))
}

pub fn prim_cdr(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if !tsp_arg_num_check(&args, "cdr", 1) { return mk_err(); }
    if !tsp_arg_type_check(car(&args), "cdr", TspType::TspPair as u32) { return mk_err(); }
    clone_val(cdr(car(&args)))
}

pub fn prim_cons(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if !tsp_arg_num_check(&args, "cons", 2) { return mk_err(); }
    let a = clone_val(car(&args));
    let b = clone_val(car(cdr(&args)));
    mk_pair(a, b).unwrap_or_else(|| mk_err())
}

pub fn form_quote(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if !tsp_arg_num_check(&args, "quote", 1) { return mk_err(); }
    clone_val(car(&args))
}

pub fn prim_eval(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if !tsp_arg_num_check(&args, "eval", 1) { return mk_err(); }
    let v = clone_val(car(&args));
    match tisp_eval(st, v) {
        Some(r) => r,
        None => clone_val(&st.none),
    }
}

pub fn prim_eq(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if nilp(&args) { return clone_val(&st.t); }
    let mut cur = &args;
    while !nilp(cdr(cur)) {
        if !vals_eq(car(cur), car(cdr(cur))) {
            return clone_val(&st.nil);
        }
        cur = cdr(cur);
    }
    clone_val(&st.t)
}

pub fn form_cond(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    let mut v = &args;
    while !nilp(v) {
        let cond_expr = clone_val(car(car(v)));
        match tisp_eval_with_env(st, env, cond_expr) {
            None => return mk_err(),
            Some(cond) => {
                if !nilp(&cond) {
                    let body = clone_val(cdr(car(v)));
                    match tisp_eval_body(st, env, body) {
                        Some(r) => return r,
                        None => return mk_err(),
                    }
                }
            }
        }
        v = cdr(v);
    }
    clone_val(&st.none)
}

pub fn prim_typeof(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if !tsp_arg_num_check(&args, "typeof", 1) { return mk_err(); }
    mk_str(st, tsp_type_str(car(&args).t)).unwrap_or_else(|| mk_err())
}

pub fn prim_procprops(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if !tsp_arg_num_check(&args, "procprops", 1) { return mk_err(); }
    let proc_val = car(&args);
    let mut ret = rec_new(6, None);
    match proc_val.t {
        TspType::TspForm | TspType::TspPrim => {
            if let ValUnion::Pr { name, .. } = &proc_val.v {
                let sym = mk_sym_val(st, name);
                rec_add(&mut ret, "name", sym);
            }
        }
        TspType::TspFunc | TspType::TspMacro => {
            if let ValUnion::F { name, args: fargs, body, .. } = &proc_val.v {
                let n = if name.is_empty() { "anon" } else { name.as_str() };
                let sym = mk_sym_val(st, n);
                rec_add(&mut ret, "name", sym);
                rec_add(&mut ret, "args", clone_val(fargs));
                rec_add(&mut ret, "body", clone_val(body));
            }
        }
        _ => {
            eprintln!("; tisp: error: procprops: expected Proc, received '{}'", tsp_type_str(proc_val.t));
            return mk_err();
        }
    }
    mk_rec(st, ret, clone_val(&st.nil)).unwrap_or_else(|| mk_err())
}

#[allow(non_snake_case)]
pub fn form_Func(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    if !tsp_arg_min_check(&args, "Func", 1) { return mk_err(); }
    let (params, body) = if nilp(cdr(&args)) {
        let p = mk_pair(mk_sym_val(st, "it"), clone_val(&st.nil)).unwrap();
        (p, clone_val(&args))
    } else {
        (clone_val(car(&args)), clone_val(cdr(&args)))
    };
    let env_clone = clone_rec(env);
    mk_func(TspType::TspFunc, "", params, body, env_clone).unwrap_or_else(|| mk_err())
}

#[allow(non_snake_case)]
pub fn form_Macro(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    if !tsp_arg_min_check(&args, "Macro", 1) { return mk_err(); }
    let mut ret = form_Func(st, env, args);
    if is_err_val(&ret) { return ret; }
    ret.t = TspType::TspMacro;
    ret
}

pub fn prim_error(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if !tsp_arg_min_check(&args, "error", 2) { return mk_err(); }
    if !tsp_arg_type_check(car(&args), "error", TspType::TspSym as u32) { return mk_err(); }
    eprint!("; tisp: error: {}: ", vs(car(&args)));
    let mut cur = cdr(&args);
    while !nilp(cur) {
        tisp_print(&mut std::io::stderr(), car(cur));
        cur = cdr(cur);
    }
    eprintln!();
    mk_err()
}

pub fn prim_recmerge(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if !tsp_arg_num_check(&args, "recmerge", 2) { return mk_err(); }
    if !tsp_arg_type_check(car(&args), "recmerge", TspType::TspRec as u32) { return mk_err(); }
    if !tsp_arg_type_check(car(cdr(&args)), "recmerge", TspType::TspRec as u32) { return mk_err(); }
    let first = car(&args);
    let second = car(cdr(&args));
    if let (ValUnion::R(r1), ValUnion::R(r2)) = (&first.v, &second.v) {
        let cap = (r2.size * TSP_REC_FACTOR as i32) as usize;
        let cap = if cap == 0 { 1 } else { cap };
        let mut new_rec = rec_new(cap, Some(Box::new(clone_rec(r1))));
        let mut r = Some(r2 as &Rec);
        while let Some(rec) = r {
            for i in 0..rec.items.len() {
                if !rec.items[i].key.is_empty() {
                    rec_add(&mut new_rec, &rec.items[i].key.clone(), clone_val(&rec.items[i].val));
                }
            }
            r = rec.next.as_deref();
        }
        return Val { t: TspType::TspRec, v: ValUnion::R(new_rec) };
    }
    mk_err()
}

pub fn prim_records(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if !tsp_arg_num_check(&args, "records", 1) { return mk_err(); }
    if !tsp_arg_type_check(car(&args), "records", TspType::TspRec as u32) { return mk_err(); }
    let mut ret = clone_val(&st.nil);
    if let ValUnion::R(r) = &car(&args).v {
        let mut rec_opt = Some(r as &Rec);
        while let Some(rec) = rec_opt {
            for i in 0..rec.items.len() {
                if !rec.items[i].key.is_empty() {
                    let sym = mk_sym_val(st, &rec.items[i].key.clone());
                    let entry = mk_pair(sym, clone_val(&rec.items[i].val)).unwrap();
                    ret = mk_pair(entry, ret).unwrap();
                }
            }
            rec_opt = rec.next.as_deref();
        }
    }
    ret
}

pub fn form_def(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    if !tsp_arg_min_check(&args, "def", 1) { return mk_err(); }
    let first = car(&args);
    let (sym_name, val) = if first.t == TspType::TspPair {
        // Create function: (def (name args...) body...)
        let sym = car(first);
        if sym.t != TspType::TspSym {
            eprintln!("; tisp: error: def: expected symbol for function name, received '{}'", tsp_type_str(sym.t));
            return mk_err();
        }
        let name = vs(sym).to_string();
        let fargs = clone_val(cdr(first));
        let body = clone_val(cdr(&args));
        let env_clone = clone_rec(env);
        let v = mk_func(TspType::TspFunc, &name, fargs, body, env_clone).unwrap_or_else(|| mk_err());
        if is_err_val(&v) { return v; }
        (name, v)
    } else if first.t == TspType::TspSym {
        let name = vs(first).to_string();
        let val = if nilp(cdr(&args)) {
            clone_val(first) // self-evaluating
        } else {
            let expr = clone_val(car(cdr(&args)));
            match tisp_eval_with_env(st, env, expr) {
                Some(v) => v,
                None => return mk_err(),
            }
        };
        (name, val)
    } else {
        eprintln!("; tisp: error: def: incorrect format, no variable name found");
        return mk_err();
    };

    // Set procedure name if anonymous
    let mut val = val;
    if (val.t == TspType::TspFunc || val.t == TspType::TspMacro) {
        if let ValUnion::F { name, .. } = &val.v {
            if name.is_empty() {
                if let ValUnion::F { name: ref mut n, .. } = val.v {
                    *n = sym_name.clone();
                }
            }
        }
    }
    rec_add(env, &sym_name, val);
    clone_val(&st.none)
}

pub fn form_undefine(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    if !tsp_arg_min_check(&args, "undefine!", 1) { return mk_err(); }
    if !tsp_arg_type_check(car(&args), "undefine!", TspType::TspSym as u32) { return mk_err(); }
    let key = vs(car(&args)).to_string();
    let mut r: &mut Rec = env;
    loop {
        let idx = entry_get_idx(r, &key);
        if !r.items[idx].key.is_empty() {
            r.items[idx].key = String::new();
            return clone_val(&st.none);
        }
        if r.next.is_none() { break; }
        r = r.next.as_deref_mut().unwrap();
    }
    eprintln!("; tisp: error: undefine!: could not find symbol {} to undefine", key);
    mk_err()
}

pub fn form_definedp(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    if !tsp_arg_min_check(&args, "defined?", 1) { return mk_err(); }
    if !tsp_arg_type_check(car(&args), "defined?", TspType::TspSym as u32) { return mk_err(); }
    let key = vs(car(&args)).to_string();
    let mut r: &Rec = env;
    loop {
        let idx = entry_get_idx(r, &key);
        if !r.items[idx].key.is_empty() {
            return clone_val(&st.t);
        }
        match &r.next {
            Some(next) => r = next,
            None => break,
        }
    }
    clone_val(&st.nil)
}

// tisp_eval_body as a Prim for "do" form
fn prim_do(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    match tisp_eval_body(st, env, args) {
        Some(v) => v,
        None => mk_err(),
    }
}

pub fn tib_env_core(st: &mut Tsp) {
    tisp_env_add(st, "car", mk_prim(TspType::TspPrim, prim_car, "car").unwrap());
    tisp_env_add(st, "cdr", mk_prim(TspType::TspPrim, prim_cdr, "cdr").unwrap());
    tisp_env_add(st, "cons", mk_prim(TspType::TspPrim, prim_cons, "cons").unwrap());
    tisp_env_add(st, "quote", mk_prim(TspType::TspForm, form_quote, "quote").unwrap());
    tisp_env_add(st, "eval", mk_prim(TspType::TspPrim, prim_eval, "eval").unwrap());
    tisp_env_add(st, "=", mk_prim(TspType::TspPrim, prim_eq, "=").unwrap());
    tisp_env_add(st, "cond", mk_prim(TspType::TspForm, form_cond, "cond").unwrap());
    tisp_env_add(st, "do", mk_prim(TspType::TspForm, prim_do, "do").unwrap());

    tisp_env_add(st, "typeof", mk_prim(TspType::TspPrim, prim_typeof, "typeof").unwrap());
    tisp_env_add(st, "procprops", mk_prim(TspType::TspPrim, prim_procprops, "procprops").unwrap());
    tisp_env_add(st, "Func", mk_prim(TspType::TspForm, form_Func, "Func").unwrap());
    tisp_env_add(st, "Macro", mk_prim(TspType::TspForm, form_Macro, "Macro").unwrap());
    tisp_env_add(st, "error", mk_prim(TspType::TspPrim, prim_error, "error").unwrap());

    tisp_env_add(st, "Rec", mk_prim(TspType::TspForm, mk_rec_prim, "Rec").unwrap());
    tisp_env_add(st, "recmerge", mk_prim(TspType::TspPrim, prim_recmerge, "recmerge").unwrap());
    tisp_env_add(st, "records", mk_prim(TspType::TspPrim, prim_records, "records").unwrap());
    tisp_env_add(st, "def", mk_prim(TspType::TspForm, form_def, "def").unwrap());
    tisp_env_add(st, "undefine!", mk_prim(TspType::TspForm, form_undefine, "undefine!").unwrap());
    tisp_env_add(st, "defined?", mk_prim(TspType::TspForm, form_definedp, "defined?").unwrap());
}
