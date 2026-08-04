use crate::tisp::{
    mk_func, mk_pair, mk_rec, mk_str, mk_sym, rec_add, rec_get, rec_new, tisp_eval,
    tisp_eval_body, tisp_eval_with_env, tsp_lstlen, tsp_type_str, vals_eq, Entry, Rec, Tsp,
    TspType, Val, ValUnion, TSP_REC_MAX_PRINT,
};

fn make_none() -> Val {
    Val {
        t: TspType::TspNone,
        v: ValUnion::S(String::new()),
    }
}

fn nilp(v: &Val) -> bool {
    matches!(v.t, TspType::TspNil)
}

fn car_of(v: &Val) -> Option<&Val> {
    if let ValUnion::P { car, .. } = &v.v {
        Some(car.as_ref())
    } else {
        None
    }
}

fn cdr_of(v: &Val) -> Option<&Val> {
    if let ValUnion::P { cdr, .. } = &v.v {
        Some(cdr.as_ref())
    } else {
        None
    }
}

pub fn prim_car(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if let Some(a) = car_of(&args) {
        if let Some(c) = car_of(a) {
            return c.clone();
        }
    }
    make_none()
}

pub fn prim_cdr(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if let Some(a) = car_of(&args) {
        if let Some(c) = cdr_of(a) {
            return c.clone();
        }
    }
    make_none()
}

pub fn prim_cons(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    let a = match car_of(&args) {
        Some(v) => v.clone(),
        None => return make_none(),
    };
    let b = match cdr_of(&args).and_then(car_of) {
        Some(v) => v.clone(),
        None => return make_none(),
    };
    mk_pair(a, b).unwrap_or_else(make_none)
}

pub fn form_quote(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if let Some(a) = car_of(&args) {
        return a.clone();
    }
    make_none()
}

pub fn prim_eval(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    let a = match car_of(&args) {
        Some(v) => v.clone(),
        None => return st.none.clone(),
    };
    tisp_eval(st, a).unwrap_or_else(|| st.none.clone())
}

pub fn prim_eq(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if nilp(&args) {
        return st.t.clone();
    }
    let mut cur = &args;
    while let Some(c) = cdr_of(cur) {
        if nilp(c) {
            break;
        }
        let a = match car_of(cur) {
            Some(v) => v,
            None => return st.nil.clone(),
        };
        let b = match car_of(c) {
            Some(v) => v,
            None => return st.nil.clone(),
        };
        if !vals_eq(a, b) {
            return st.nil.clone();
        }
        cur = c;
    }
    st.t.clone()
}

pub fn form_cond(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    let mut cur = args;
    while !nilp(&cur) {
        let (head, rest) = if let ValUnion::P { car, cdr } = cur.v {
            (*car, *cdr)
        } else {
            break;
        };
        let cond_expr = match car_of(&head) {
            Some(v) => v.clone(),
            None => return st.none.clone(),
        };
        let cond = match tisp_eval_with_env(st, env, cond_expr) {
            Some(v) => v,
            None => return st.none.clone(),
        };
        if !nilp(&cond) {
            let body = match cdr_of(&head) {
                Some(v) => v.clone(),
                None => return st.none.clone(),
            };
            return tisp_eval_body(st, env, body).unwrap_or_else(|| st.none.clone());
        }
        cur = rest;
    }
    st.none.clone()
}

pub fn prim_typeof(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    let a = match car_of(&args) {
        Some(v) => v,
        None => return st.none.clone(),
    };
    mk_str(st, tsp_type_str(a.t)).unwrap_or_else(|| st.none.clone())
}

pub fn prim_procprops(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    let proc = match car_of(&args) {
        Some(v) => v.clone(),
        None => return st.none.clone(),
    };
    let mut ret = rec_new(6, None);
    match proc.t {
        TspType::TspForm | TspType::TspPrim => {
            if let ValUnion::Pr { name, .. } = &proc.v {
                let s = mk_sym(st, name).unwrap_or_else(|| st.none.clone());
                rec_add(&mut ret, "name", s);
            }
        }
        TspType::TspFunc | TspType::TspMacro => {
            if let ValUnion::F { name, args: fa, body, .. } = &proc.v {
                let n = if name.is_empty() { "anon" } else { name.as_str() };
                let s = mk_sym(st, n).unwrap_or_else(|| st.none.clone());
                rec_add(&mut ret, "name", s);
                rec_add(&mut ret, "args", fa.as_ref().clone());
                rec_add(&mut ret, "body", body.as_ref().clone());
            }
        }
        _ => return st.none.clone(),
    }
    mk_rec(st, ret, st.none.clone()).unwrap_or_else(|| st.none.clone())
}

#[allow(non_snake_case)]
pub fn form_Func(_st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    if tsp_lstlen(&args) < 1 && !matches!(args.t, TspType::TspPair) {
        return make_none();
    }
    let cdr_v = match cdr_of(&args) {
        Some(v) => v.clone(),
        None => return make_none(),
    };
    let (params, body) = if nilp(&cdr_v) {
        let it = Val {
            t: TspType::TspSym,
            v: ValUnion::S("it".to_string()),
        };
        let nil = Val {
            t: TspType::TspNil,
            v: ValUnion::S(String::new()),
        };
        let params = mk_pair(it, nil).unwrap_or_else(make_none);
        (params, args.clone())
    } else {
        let p = match car_of(&args) {
            Some(v) => v.clone(),
            None => return make_none(),
        };
        (p, cdr_v)
    };
    mk_func(TspType::TspFunc, "", params, body, env.clone()).unwrap_or_else(make_none)
}

#[allow(non_snake_case)]
pub fn form_Macro(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    let mut ret = form_Func(st, env, args);
    ret.t = TspType::TspMacro;
    ret
}

pub fn prim_error(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    let _ = args;
    make_none()
}

pub fn prim_recmerge(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    let a = match car_of(&args) {
        Some(v) => v.clone(),
        None => return make_none(),
    };
    let b = match cdr_of(&args).and_then(car_of) {
        Some(v) => v.clone(),
        None => return make_none(),
    };
    let r1 = if let ValUnion::R(r) = &a.v { r.clone() } else { return make_none(); };
    let r2 = if let ValUnion::R(r) = &b.v { r.clone() } else { return make_none(); };
    let cap = (r2.size as usize).max(1) * 2;
    let mut new_rec = rec_new(cap, Some(Box::new(r1)));
    // copy entries from r2
    let mut chain: Vec<&Rec> = Vec::new();
    let mut cur: Option<&Rec> = Some(&r2);
    while let Some(r) = cur {
        chain.push(r);
        cur = r.next.as_deref();
    }
    for r in chain.iter().rev() {
        for entry in &r.items {
            if !entry.key.is_empty() {
                rec_add(&mut new_rec, &entry.key, entry.val.clone());
            }
        }
    }
    Val { t: TspType::TspRec, v: ValUnion::R(new_rec) }
}

pub fn prim_records(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    let a = match car_of(&args) {
        Some(v) => v.clone(),
        None => return st.nil.clone(),
    };
    let r = if let ValUnion::R(r) = &a.v { r.clone() } else { return st.nil.clone(); };
    let mut ret = st.nil.clone();
    let mut cur: Option<&Rec> = Some(&r);
    while let Some(rec) = cur {
        let mut count = 0;
        let mut printed = 0;
        for entry in &rec.items {
            if !entry.key.is_empty() {
                let sym = mk_sym(st, &entry.key).unwrap_or_else(|| st.none.clone());
                let entry_pair = mk_pair(sym, entry.val.clone()).unwrap_or_else(|| st.none.clone());
                ret = mk_pair(entry_pair, ret).unwrap_or_else(|| st.none.clone());
                count += 1;
                printed += 1;
                if printed >= rec.size {
                    break;
                }
                let _ = count;
            }
            if printed >= TSP_REC_MAX_PRINT as i32 {
                break;
            }
        }
        cur = rec.next.as_deref();
    }
    ret
}

pub fn form_def(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    if tsp_lstlen(&args) < 1 {
        return st.none.clone();
    }
    let first = match car_of(&args) {
        Some(v) => v.clone(),
        None => return st.none.clone(),
    };
    let rest = match cdr_of(&args) {
        Some(v) => v.clone(),
        None => return st.none.clone(),
    };
    let (sym_str, mut val) = match first.t {
        TspType::TspPair => {
            // function form (def (name args...) body...)
            let fname = match car_of(&first) {
                Some(v) => v.clone(),
                None => return st.none.clone(),
            };
            if !matches!(fname.t, TspType::TspSym) {
                return st.none.clone();
            }
            let fname_str = if let ValUnion::S(s) = &fname.v { s.clone() } else { return st.none.clone() };
            let fargs = match cdr_of(&first) {
                Some(v) => v.clone(),
                None => return st.none.clone(),
            };
            let func = mk_func(TspType::TspFunc, &fname_str, fargs, rest, env.clone())
                .unwrap_or_else(make_none);
            (fname_str, func)
        }
        TspType::TspSym => {
            let s = if let ValUnion::S(s) = &first.v { s.clone() } else { return st.none.clone() };
            let val = if nilp(&rest) {
                first.clone()
            } else {
                let inner = match car_of(&rest) {
                    Some(v) => v.clone(),
                    None => return st.none.clone(),
                };
                tisp_eval_with_env(st, env, inner).unwrap_or_else(|| st.none.clone())
            };
            (s, val)
        }
        _ => return st.none.clone(),
    };
    // set name on funcs/macros if missing
    if matches!(val.t, TspType::TspFunc | TspType::TspMacro) {
        if let ValUnion::F { name, .. } = &mut val.v {
            if name.is_empty() {
                *name = sym_str.clone();
            }
        }
    }
    rec_add(env, &sym_str, val);
    st.none.clone()
}

pub fn form_undefine(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    let sym = match car_of(&args) {
        Some(v) => v.clone(),
        None => return st.none.clone(),
    };
    let key = if let ValUnion::S(s) = &sym.v { s.clone() } else { return st.none.clone() };
    let mut cur: &mut Rec = env;
    loop {
        let cap = cur.cap as usize;
        if cap > 0 {
            for i in 0..cap {
                if cur.items[i].key == key {
                    cur.items[i].key.clear();
                    cur.items[i].val = st.none.clone();
                    return st.none.clone();
                }
            }
        }
        if cur.next.is_some() {
            cur = cur.next.as_mut().unwrap();
        } else {
            break;
        }
    }
    st.none.clone()
}

pub fn form_definedp(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    let sym = match car_of(&args) {
        Some(v) => v.clone(),
        None => return st.nil.clone(),
    };
    let key = if let ValUnion::S(s) = &sym.v { s.clone() } else { return st.nil.clone() };
    if rec_get(env, &key).is_some() {
        return st.t.clone();
    }
    if rec_get(&st.env, &key).is_some() {
        return st.t.clone();
    }
    st.nil.clone()
}

pub fn tib_env_core(st: &mut Tsp) {
    let names = [
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
    for (name, t) in names.iter() {
        let v = Val {
            t: *t,
            v: ValUnion::Pr { name: name.to_string(), pr: dummy_prim },
        };
        rec_add(&mut st.env, name, v);
    }
    let _ = Entry { key: String::new(), val: st.nil.clone() };
}

fn dummy_prim(_st: Tsp, _env: Rec, _args: Val) -> Val {
    make_none()
}
