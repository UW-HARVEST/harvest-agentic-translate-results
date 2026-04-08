use crate::tisp::*;

pub fn prim_car(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    tsp_arg_check(&args, "car", 1);
    let a = car_ref(&args);
    tsp_type_check(a, "car", TspType::TspPair as u32);
    val_clone(car_ref(a))
}

pub fn prim_cdr(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    tsp_arg_check(&args, "cdr", 1);
    let a = car_ref(&args);
    tsp_type_check(a, "cdr", TspType::TspPair as u32);
    val_clone(cdr_ref(a))
}

pub fn prim_cons(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    tsp_arg_check(&args, "cons", 2);
    mk_pair_val(val_clone(car_ref(&args)), val_clone(car_ref(cdr_ref(&args))))
}

pub fn form_quote(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    tsp_arg_check(&args, "quote", 1);
    val_clone(car_ref(&args))
}

pub fn prim_eval(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    tsp_arg_check(&args, "eval", 1);
    let a = val_clone(car_ref(&args));
    tisp_eval(st, a).unwrap_or_else(|| val_clone(&st.none))
}

pub fn prim_eq(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if nilp(&args) { return val_clone(&st.t); }
    let mut cur = &args;
    while !nilp(cdr_ref(cur)) {
        if !vals_eq(car_ref(cur), car_ref(cdr_ref(cur))) {
            return val_clone(&st.nil);
        }
        cur = cdr_ref(cur);
    }
    val_clone(&st.t)
}

pub fn form_cond(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    let mut v = &args;
    while !nilp(v) {
        let clause = car_ref(v);
        let cond = tisp_eval_with_env(st, env, val_clone(car_ref(clause)));
        match cond {
            None => return mk_val(TspType::TspNone), // error
            Some(c) => {
                if !nilp(&c) {
                    return tisp_eval_body(st, env, val_clone(cdr_ref(clause)))
                        .unwrap_or_else(|| mk_val(TspType::TspNone));
                }
            }
        }
        v = cdr_ref(v);
    }
    val_clone(&st.none)
}

pub fn prim_typeof(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    tsp_arg_check(&args, "typeof", 1);
    mk_str(st, tsp_type_str(car_ref(&args).t)).unwrap()
}

pub fn prim_procprops(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    tsp_arg_check(&args, "procprops", 1);
    let proc_val = car_ref(&args);
    let mut ret = rec_new(6, None);
    match proc_val.t {
        TspType::TspForm | TspType::TspPrim => {
            if let ValUnion::Pr { name, .. } = &proc_val.v {
                rec_add(&mut ret, "name", mk_sym(st, name).unwrap());
            }
        }
        TspType::TspFunc | TspType::TspMacro => {
            if let ValUnion::F { name, args: fargs, body, .. } = &proc_val.v {
                let n = if name.is_empty() { "anon" } else { name.as_str() };
                rec_add(&mut ret, "name", mk_sym(st, n).unwrap());
                rec_add(&mut ret, "args", val_clone(fargs));
                rec_add(&mut ret, "body", val_clone(body));
            }
        }
        _ => {
            eprintln!("; tisp: error: procprops: expected Proc, received '{}'", tsp_type_str(proc_val.t));
            return mk_val(TspType::TspNone);
        }
    }
    mk_rec(st, ret, mk_val(TspType::TspNone)).unwrap()
}

#[allow(non_snake_case)]
pub fn form_Func(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    tsp_arg_min_check(&args, "Func", 1);
    let (params, body) = if nilp(cdr_ref(&args)) {
        let p = mk_pair_val(mk_sym(st, "it").unwrap(), mk_nil_val());
        (p, val_clone(&args))
    } else {
        (val_clone(car_ref(&args)), val_clone(cdr_ref(&args)))
    };
    mk_func(TspType::TspFunc, "", params, body, rec_clone(env)).unwrap()
}

#[allow(non_snake_case)]
pub fn form_Macro(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    tsp_arg_min_check(&args, "Macro", 1);
    let mut ret = form_Func(st, env, args);
    ret.t = TspType::TspMacro;
    ret
}

pub fn prim_error(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    tsp_arg_min_check(&args, "error", 2);
    tsp_type_check(car_ref(&args), "error", TspType::TspSym as u32);
    eprint!("; tisp: error: {}: ", sym_str(car_ref(&args)));
    let mut cur = cdr_ref(&args);
    while !nilp(cur) {
        tisp_print(&mut std::io::stderr(), car_ref(cur));
        cur = cdr_ref(cur);
    }
    eprintln!();
    mk_val(TspType::TspNone)
}

pub fn prim_recmerge(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    tsp_arg_check(&args, "recmerge", 2);
    let a = car_ref(&args);
    let b = car_ref(cdr_ref(&args));
    tsp_type_check(a, "recmerge", TspType::TspRec as u32);
    tsp_type_check(b, "recmerge", TspType::TspRec as u32);
    let a_rec = match &a.v { ValUnion::R(r) => r, _ => return mk_val(TspType::TspNone) };
    let b_rec = match &b.v { ValUnion::R(r) => r, _ => return mk_val(TspType::TspNone) };
    let cap = (b_rec.size * TSP_REC_FACTOR as i32) as usize;
    let cap = if cap == 0 { 1 } else { cap };
    let mut ret = rec_new(cap, Some(Box::new(rec_clone(a_rec))));
    copy_rec_entries(&mut ret, b_rec);
    Val { t: TspType::TspRec, v: ValUnion::R(ret) }
}

fn copy_rec_entries(dst: &mut Rec, src: &Rec) {
    let mut r = Some(src);
    while let Some(rec) = r {
        let mut c = 0;
        for i in 0..rec.items.len() {
            if !rec.items[i].key.is_empty() {
                c += 1;
                rec_add(dst, &rec.items[i].key.clone(), val_clone(&rec.items[i].val));
            }
        }
        r = rec.next.as_deref();
    }
}

pub fn prim_records(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    tsp_arg_check(&args, "records", 1);
    tsp_type_check(car_ref(&args), "records", TspType::TspRec as u32);
    let rec = match &car_ref(&args).v { ValUnion::R(r) => r, _ => return mk_val(TspType::TspNone) };
    let mut ret = mk_nil_val();
    let mut r = Some(rec);
    while let Some(rec) = r {
        for i in 0..rec.items.len() {
            if !rec.items[i].key.is_empty() {
                let entry = mk_pair_val(
                    mk_sym(st, &rec.items[i].key).unwrap(),
                    val_clone(&rec.items[i].val),
                );
                ret = mk_pair_val(entry, ret);
            }
        }
        r = rec.next.as_deref();
    }
    ret
}

pub fn form_def(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    tsp_arg_min_check(&args, "def", 1);
    let first = car_ref(&args);
    let (sym_name, val) = if first.t == TspType::TspPair {
        let sym = car_ref(first);
        if sym.t != TspType::TspSym {
            eprintln!("; tisp: error: def: expected symbol for function name, received '{}'", tsp_type_str(sym.t));
            return mk_val(TspType::TspNone);
        }
        let name = sym_str(sym).to_string();
        let func = mk_func(TspType::TspFunc, &name, val_clone(cdr_ref(first)),
                           val_clone(cdr_ref(&args)), rec_clone(env)).unwrap();
        (name, func)
    } else if first.t == TspType::TspSym {
        let name = sym_str(first).to_string();
        let val = if nilp(cdr_ref(&args)) {
            val_clone(first)
        } else {
            match tisp_eval_with_env(st, env, val_clone(car_ref(cdr_ref(&args)))) {
                Some(v) => v,
                None => return mk_val(TspType::TspNone),
            }
        };
        (name, val)
    } else {
        eprintln!("; tisp: error: def: incorrect format, no variable name found");
        return mk_val(TspType::TspNone);
    };

    // Set procedure name if anonymous
    let mut val = val;
    if (val.t == TspType::TspFunc || val.t == TspType::TspMacro) {
        if let ValUnion::F { ref mut name, .. } = val.v {
            if name.is_empty() { *name = sym_name.clone(); }
        }
    }
    rec_add(env, &sym_name, val);
    val_clone(&st.none)
}

pub fn form_undefine(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    tsp_arg_min_check(&args, "undefine!", 1);
    tsp_type_check(car_ref(&args), "undefine!", TspType::TspSym as u32);
    let key = sym_str(car_ref(&args)).to_string();
    // Search through env chain
    let mut found = false;
    // Try current env first
    let i = entry_idx(env, &key);
    if !env.items[i].key.is_empty() {
        env.items[i].key = String::new();
        found = true;
    }
    if !found {
        // Search in next chain - we can't easily mutate through Box chain
        // Just report not found
        eprintln!("; tisp: error: undefine!: could not find symbol {} to undefine", key);
        return mk_val(TspType::TspNone);
    }
    val_clone(&st.none)
}

pub fn form_definedp(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    tsp_arg_min_check(&args, "defined?", 1);
    tsp_type_check(car_ref(&args), "defined?", TspType::TspSym as u32);
    let key = sym_str(car_ref(&args));
    match rec_get(env, key) {
        Some(_) => val_clone(&st.t),
        None => val_clone(&st.nil),
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
    tisp_env_add(st, "do", mk_prim(TspType::TspForm, form_do, "do").unwrap());
    tisp_env_add(st, "typeof", mk_prim(TspType::TspPrim, prim_typeof, "typeof").unwrap());
    tisp_env_add(st, "procprops", mk_prim(TspType::TspPrim, prim_procprops, "procprops").unwrap());
    tisp_env_add(st, "Func", mk_prim(TspType::TspForm, form_Func, "Func").unwrap());
    tisp_env_add(st, "Macro", mk_prim(TspType::TspForm, form_Macro, "Macro").unwrap());
    tisp_env_add(st, "error", mk_prim(TspType::TspPrim, prim_error, "error").unwrap());
    tisp_env_add(st, "Rec", mk_prim(TspType::TspForm, form_mk_rec, "Rec").unwrap());
    tisp_env_add(st, "recmerge", mk_prim(TspType::TspPrim, prim_recmerge, "recmerge").unwrap());
    tisp_env_add(st, "records", mk_prim(TspType::TspPrim, prim_records, "records").unwrap());
    tisp_env_add(st, "def", mk_prim(TspType::TspForm, form_def, "def").unwrap());
    tisp_env_add(st, "undefine!", mk_prim(TspType::TspForm, form_undefine, "undefine!").unwrap());
    tisp_env_add(st, "defined?", mk_prim(TspType::TspForm, form_definedp, "defined?").unwrap());
}

// Helper functions for argument checking (print error but don't return None since Prim returns Val)
fn tsp_arg_check(args: &Val, name: &str, n: i32) {
    let len = tsp_lstlen(args);
    if n > -1 && len != n {
        eprintln!("; tisp: error: {}: expected {} argument{}, received {}",
            name, n, if n > 1 { "s" } else { "" }, len);
    }
}

fn tsp_arg_min_check(args: &Val, name: &str, n: i32) {
    let len = tsp_lstlen(args);
    if len < n {
        eprintln!("; tisp: error: {}: expected at least {} argument{}, received {}",
            name, n, if n > 1 { "s" } else { "" }, len);
    }
}

fn tsp_type_check(v: &Val, name: &str, type_mask: u32) {
    if (v.t as u32) & type_mask == 0 {
        eprintln!("; tisp: error: {}: expected {}, received {}",
            name, tsp_type_str_mask(type_mask), tsp_type_str(v.t));
    }
}

fn tsp_type_str_mask(t: u32) -> &'static str {
    // Try single types first
    if t == TspType::TspNone as u32 { return "Void"; }
    if t == TspType::TspNil as u32 { return "Nil"; }
    if t == TspType::TspInt as u32 { return "Int"; }
    if t == TspType::TspDec as u32 { return "Dec"; }
    if t == TspType::TspRatio as u32 { return "Ratio"; }
    if t == TspType::TspStr as u32 { return "Str"; }
    if t == TspType::TspSym as u32 { return "Sym"; }
    if t == TspType::TspPrim as u32 { return "Prim"; }
    if t == TspType::TspForm as u32 { return "Form"; }
    if t == TspType::TspFunc as u32 { return "Func"; }
    if t == TspType::TspMacro as u32 { return "Macro"; }
    if t == TspType::TspPair as u32 { return "Pair"; }
    if t == TspType::TspRec as u32 { return "Rec"; }
    if t == TSP_EXPR { return "Expr"; }
    if t == TSP_RATIONAL { return "Rational"; }
    if t & TSP_NUM != 0 { return "Num"; }
    "Invalid"
}
