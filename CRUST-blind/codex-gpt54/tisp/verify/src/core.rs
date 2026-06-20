use crate::tisp::{
    Entry, Rec, TSP_NUM, Tsp, TspType, Val, ValUnion, eval_in_env, expect_len, expect_min_len,
    expect_type, mk_func, mk_pair, mk_prim, mk_rec, mk_sym, pair_car, pair_cdr, rec_add, rec_get,
    rec_new, render_val, tisp_env_add, tisp_eval, tisp_eval_body, tsp_type_str, type_matches,
    val_is_nil, val_str, vals_eq,
};

pub fn prim_car(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if !expect_len(st, &args, "car", 1) {
        return st.none.clone();
    }
    let arg = pair_car(&args).clone();
    if !expect_type(st, &arg, "car", TspType::TspPair as u32) {
        return st.none.clone();
    }
    pair_car(&arg).clone()
}

pub fn prim_cdr(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if !expect_len(st, &args, "cdr", 1) {
        return st.none.clone();
    }
    let arg = pair_car(&args).clone();
    if !expect_type(st, &arg, "cdr", TspType::TspPair as u32) {
        return st.none.clone();
    }
    pair_cdr(&arg).clone()
}

pub fn prim_cons(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if !expect_len(st, &args, "cons", 2) {
        return st.none.clone();
    }
    mk_pair(pair_car(&args).clone(), pair_car(pair_cdr(&args)).clone()).unwrap_or_else(|| st.none.clone())
}

pub fn form_quote(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if !expect_len(st, &args, "quote", 1) {
        return st.none.clone();
    }
    pair_car(&args).clone()
}

pub fn prim_eval(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if !expect_len(st, &args, "eval", 1) {
        return st.none.clone();
    }
    tisp_eval(st, pair_car(&args).clone()).unwrap_or_else(|| st.none.clone())
}

pub fn prim_eq(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if val_is_nil(&args) {
        return st.t.clone();
    }
    let mut cur = args;
    while pair_cdr(&cur).t == TspType::TspPair {
        if !vals_eq(pair_car(&cur), pair_car(pair_cdr(&cur))) {
            return st.nil.clone();
        }
        cur = pair_cdr(&cur).clone();
    }
    st.t.clone()
}

pub fn form_cond(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    let mut cur = args;
    while cur.t == TspType::TspPair {
        let clause = pair_car(&cur).clone();
        if clause.t != TspType::TspPair {
            return st.none.clone();
        }
        let cond = eval_in_env(st, env, pair_car(&clause).clone()).unwrap_or_else(|| st.none.clone());
        if !val_is_nil(&cond) {
            return tisp_eval_body(st, env, pair_cdr(&clause).clone()).unwrap_or_else(|| st.none.clone());
        }
        cur = pair_cdr(&cur).clone();
    }
    st.none.clone()
}

pub fn prim_typeof(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if !expect_len(st, &args, "typeof", 1) {
        return st.none.clone();
    }
    crate::tisp::mk_str(st, tsp_type_str(pair_car(&args).t)).unwrap_or_else(|| st.none.clone())
}

pub fn prim_procprops(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if !expect_len(st, &args, "procprops", 1) {
        return st.none.clone();
    }
    let proc = pair_car(&args).clone();
    let mut ret = rec_new(6, None);
    match &proc.v {
        ValUnion::Pr { name, .. } if proc.t == TspType::TspForm || proc.t == TspType::TspPrim => {
            rec_add(&mut ret, "name", mk_sym(st, name).unwrap_or_else(|| st.none.clone()));
        }
        ValUnion::F { name, args, body, .. } if proc.t == TspType::TspFunc || proc.t == TspType::TspMacro => {
            let label = if name.is_empty() { "anon" } else { name };
            rec_add(&mut ret, "name", mk_sym(st, label).unwrap_or_else(|| st.none.clone()));
            rec_add(&mut ret, "args", (**args).clone());
            rec_add(&mut ret, "body", (**body).clone());
        }
        _ => return st.none.clone(),
    }
    Val {
        t: TspType::TspRec,
        v: ValUnion::R(ret),
    }
}

pub fn form_Func(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    if !expect_min_len(st, &args, "Func", 1) {
        return st.none.clone();
    }
    let (params, body) = if val_is_nil(pair_cdr(&args)) {
        let default = mk_pair(
            mk_sym(st, "it").unwrap_or_else(|| st.none.clone()),
            st.nil.clone(),
        )
        .unwrap_or_else(|| st.none.clone());
        (default, args)
    } else {
        (pair_car(&args).clone(), pair_cdr(&args).clone())
    };
    mk_func(TspType::TspFunc, "", params, body, env.clone()).unwrap_or_else(|| st.none.clone())
}

pub fn form_Macro(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    let mut ret = form_Func(st, env, args);
    ret.t = TspType::TspMacro;
    ret
}

pub fn prim_error(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if !expect_min_len(st, &args, "error", 2) {
        return st.none.clone();
    }
    let name = pair_car(&args).clone();
    if !expect_type(st, &name, "error", TspType::TspSym as u32) {
        return st.none.clone();
    }
    eprint!("; tisp: error: {}: ", val_str(&name).unwrap_or_default());
    let mut cur = pair_cdr(&args).clone();
    while cur.t == TspType::TspPair {
        eprint!("{}", render_val(pair_car(&cur)));
        cur = pair_cdr(&cur).clone();
    }
    eprintln!();
    st.none.clone()
}

pub fn prim_recmerge(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if !expect_len(st, &args, "recmerge", 2) {
        return st.none.clone();
    }
    let a = pair_car(&args).clone();
    let b = pair_car(pair_cdr(&args)).clone();
    if !expect_type(st, &a, "recmerge", TspType::TspRec as u32)
        || !expect_type(st, &b, "recmerge", TspType::TspRec as u32)
    {
        return st.none.clone();
    }
    let (ValUnion::R(base), ValUnion::R(extra)) = (&a.v, &b.v) else {
        return st.none.clone();
    };
    let mut merged = rec_new((extra.size.max(1) as usize) * 2, Some(Box::new(base.clone())));
    let mut cur = Some(extra);
    while let Some(rec) = cur {
        for entry in &rec.items {
            if !entry.key.is_empty() {
                rec_add(&mut merged, &entry.key, entry.val.clone());
            }
        }
        cur = rec.next.as_deref();
    }
    Val {
        t: TspType::TspRec,
        v: ValUnion::R(merged),
    }
}

pub fn prim_records(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if !expect_len(st, &args, "records", 1) {
        return st.none.clone();
    }
    let rec_val = pair_car(&args).clone();
    if !expect_type(st, &rec_val, "records", TspType::TspRec as u32) {
        return st.none.clone();
    }
    let ValUnion::R(rec) = &rec_val.v else {
        return st.none.clone();
    };
    let mut out = st.nil.clone();
    let mut cur = Some(rec);
    while let Some(r) = cur {
        for entry in &r.items {
            if !entry.key.is_empty() {
                let pair = mk_pair(
                    mk_sym(st, &entry.key).unwrap_or_else(|| st.none.clone()),
                    entry.val.clone(),
                )
                .unwrap_or_else(|| st.none.clone());
                out = mk_pair(pair, out).unwrap_or_else(|| st.none.clone());
            }
        }
        cur = r.next.as_deref();
    }
    out
}

pub fn form_def(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    if !expect_min_len(st, &args, "def", 1) {
        return st.none.clone();
    }
    let head = pair_car(&args).clone();
    let (sym, mut val) = if head.t == TspType::TspPair {
        let sym = pair_car(&head).clone();
        if sym.t != TspType::TspSym {
            return st.none.clone();
        }
        (
            sym.clone(),
            mk_func(
                TspType::TspFunc,
                val_str(&sym).unwrap_or_default(),
                pair_cdr(&head).clone(),
                pair_cdr(&args).clone(),
                env.clone(),
            )
            .unwrap_or_else(|| st.none.clone()),
        )
    } else if head.t == TspType::TspSym {
        let val = if val_is_nil(pair_cdr(&args)) {
            head.clone()
        } else {
            eval_in_env(st, env, pair_car(pair_cdr(&args)).clone()).unwrap_or_else(|| st.none.clone())
        };
        (head.clone(), val)
    } else {
        return st.none.clone();
    };
    if let ValUnion::F { name, .. } = &mut val.v {
        if name.is_empty() {
            *name = val_str(&sym).unwrap_or_default().to_string();
        }
    }
    rec_add(env, val_str(&sym).unwrap_or_default(), val);
    st.none.clone()
}

pub fn form_undefine(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    if !expect_min_len(st, &args, "undefine!", 1) {
        return st.none.clone();
    }
    let sym = pair_car(&args).clone();
    if !expect_type(st, &sym, "undefine!", TspType::TspSym as u32) {
        return st.none.clone();
    }
    let target = val_str(&sym).unwrap_or_default();
    let mut cur = Some(env);
    while let Some(rec) = cur {
        let idx = {
            let mut idx = None;
            for (i, entry) in rec.items.iter().enumerate() {
                if entry.key == target {
                    idx = Some(i);
                    break;
                }
            }
            idx
        };
        if let Some(i) = idx {
            rec.items[i].key.clear();
            rec.size -= 1;
            return st.none.clone();
        }
        cur = rec.next.as_deref_mut();
    }
    st.none.clone()
}

pub fn form_definedp(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    if !expect_min_len(st, &args, "defined?", 1) {
        return st.nil.clone();
    }
    let sym = pair_car(&args).clone();
    if !expect_type(st, &sym, "defined?", TspType::TspSym as u32) {
        return st.nil.clone();
    }
    if rec_get(env, val_str(&sym).unwrap_or_default()).is_some() {
        st.t.clone()
    } else {
        st.nil.clone()
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
    tisp_env_add(
        st,
        "do",
        mk_prim(TspType::TspForm, |st, env, args| tisp_eval_body(st, env, args).unwrap_or_else(|| st.none.clone()), "do").unwrap(),
    );
    tisp_env_add(st, "typeof", mk_prim(TspType::TspPrim, prim_typeof, "typeof").unwrap());
    tisp_env_add(st, "procprops", mk_prim(TspType::TspPrim, prim_procprops, "procprops").unwrap());
    tisp_env_add(st, "Func", mk_prim(TspType::TspForm, form_Func, "Func").unwrap());
    tisp_env_add(st, "Macro", mk_prim(TspType::TspForm, form_Macro, "Macro").unwrap());
    tisp_env_add(st, "error", mk_prim(TspType::TspPrim, prim_error, "error").unwrap());
    tisp_env_add(st, "Rec", mk_prim(TspType::TspForm, |st, _env, args| mk_rec(st, rec_new(1, None), args).unwrap_or_else(|| st.none.clone()), "Rec").unwrap());
    tisp_env_add(st, "recmerge", mk_prim(TspType::TspPrim, prim_recmerge, "recmerge").unwrap());
    tisp_env_add(st, "records", mk_prim(TspType::TspPrim, prim_records, "records").unwrap());
    tisp_env_add(st, "def", mk_prim(TspType::TspForm, form_def, "def").unwrap());
    tisp_env_add(st, "undefine!", mk_prim(TspType::TspForm, form_undefine, "undefine!").unwrap());
    tisp_env_add(st, "defined?", mk_prim(TspType::TspForm, form_definedp, "defined?").unwrap());
}
