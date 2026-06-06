use crate::tisp::{
    mk_dec, mk_int, mk_pair, mk_prim, mk_rat, mk_sym, nil_val, none_val, pairp, tisp_env_add,
    tsp_lstlen, tsp_type_str, type_match, val_car, val_cdr, val_den, val_num, warn, Rec, Tsp,
    TspType, Val, ValUnion, TSP_NUM, TSP_RATIONAL,
};

pub fn create_int(num: f64, _den: f64) -> Val {
    mk_int(num as i32)
}

pub fn create_dec(num: f64, _den: f64) -> Val {
    mk_dec(num).unwrap_or_else(nil_val)
}

pub fn create_rat(num: f64, den: f64) -> Val {
    mk_rat(num as i32, den as i32).unwrap_or_else(nil_val)
}

pub fn mk_num(a: TspType, b: TspType, force: i32) -> fn(f64, f64) -> Val {
    if force == 1 {
        return create_rat;
    }
    if force == 2 {
        return create_dec;
    }
    if matches!(a, TspType::TspDec) || matches!(b, TspType::TspDec) {
        return create_dec;
    }
    if matches!(a, TspType::TspRatio) || matches!(b, TspType::TspRatio) {
        return create_rat;
    }
    create_int
}

fn check_num_arg(v: &Val, name: &str) -> bool {
    if !type_match(v.t, TSP_NUM) {
        warn(&format!("{}: expected Num, received {}", name, tsp_type_str(v.t)));
        return false;
    }
    true
}

pub fn prim_add(_st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    if tsp_lstlen(&args) != 2 {
        warn("+: expected 2 arguments");
        return none_val();
    }
    let a = val_car(&args).cloned().unwrap_or_else(nil_val);
    let b = val_car(&val_cdr(&args).cloned().unwrap_or_else(nil_val))
        .cloned()
        .unwrap_or_else(nil_val);
    if !check_num_arg(&a, "+") || !check_num_arg(&b, "+") {
        return none_val();
    }
    if matches!(a.t, TspType::TspDec) || matches!(b.t, TspType::TspDec) {
        return mk_dec((val_num(&a) / val_den(&a)) + (val_num(&b) / val_den(&b)))
            .unwrap_or_else(nil_val);
    }
    let f = mk_num(a.t, b.t, 0);
    f(
        val_num(&a) * val_den(&b) + val_den(&a) * val_num(&b),
        val_den(&a) * val_den(&b),
    )
}

pub fn prim_sub(_st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    let len = tsp_lstlen(&args);
    if len != 2 && len != 1 {
        warn("-: expected 1 or 2 arguments");
        return none_val();
    }
    let mut a = val_car(&args).cloned().unwrap_or_else(nil_val);
    if !check_num_arg(&a, "-") {
        return none_val();
    }
    let b;
    if len == 1 {
        b = a.clone();
        a = mk_int(0);
    } else {
        b = val_car(&val_cdr(&args).cloned().unwrap_or_else(nil_val))
            .cloned()
            .unwrap_or_else(nil_val);
        if !check_num_arg(&b, "-") {
            return none_val();
        }
    }
    if matches!(a.t, TspType::TspDec) || matches!(b.t, TspType::TspDec) {
        return mk_dec((val_num(&a) / val_den(&a)) - (val_num(&b) / val_den(&b)))
            .unwrap_or_else(nil_val);
    }
    let f = mk_num(a.t, b.t, 0);
    f(
        val_num(&a) * val_den(&b) - val_den(&a) * val_num(&b),
        val_den(&a) * val_den(&b),
    )
}

pub fn prim_mul(_st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    if tsp_lstlen(&args) != 2 {
        warn("*: expected 2 arguments");
        return none_val();
    }
    let a = val_car(&args).cloned().unwrap_or_else(nil_val);
    let b = val_car(&val_cdr(&args).cloned().unwrap_or_else(nil_val))
        .cloned()
        .unwrap_or_else(nil_val);
    if !check_num_arg(&a, "*") || !check_num_arg(&b, "*") {
        return none_val();
    }
    if matches!(a.t, TspType::TspDec) || matches!(b.t, TspType::TspDec) {
        return mk_dec((val_num(&a) / val_den(&a)) * (val_num(&b) / val_den(&b)))
            .unwrap_or_else(nil_val);
    }
    let f = mk_num(a.t, b.t, 0);
    f(val_num(&a) * val_num(&b), val_den(&a) * val_den(&b))
}

pub fn prim_div(_st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    let len = tsp_lstlen(&args);
    if len != 2 && len != 1 {
        warn("/: expected 1 or 2 arguments");
        return none_val();
    }
    let mut a = val_car(&args).cloned().unwrap_or_else(nil_val);
    if !check_num_arg(&a, "/") {
        return none_val();
    }
    let b;
    if len == 1 {
        b = a.clone();
        a = mk_int(1);
    } else {
        b = val_car(&val_cdr(&args).cloned().unwrap_or_else(nil_val))
            .cloned()
            .unwrap_or_else(nil_val);
        if !check_num_arg(&b, "/") {
            return none_val();
        }
    }
    if matches!(a.t, TspType::TspDec) || matches!(b.t, TspType::TspDec) {
        return mk_dec((val_num(&a) / val_den(&a)) / (val_num(&b) / val_den(&b)))
            .unwrap_or_else(nil_val);
    }
    let f = mk_num(a.t, b.t, 1);
    f(val_num(&a) * val_den(&b), val_den(&a) * val_num(&b))
}

pub fn prim_mod(_st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    if tsp_lstlen(&args) != 2 {
        warn("mod: expected 2 arguments");
        return none_val();
    }
    let a = val_car(&args).cloned().unwrap_or_else(nil_val);
    let b = val_car(&val_cdr(&args).cloned().unwrap_or_else(nil_val))
        .cloned()
        .unwrap_or_else(nil_val);
    if !matches!(a.t, TspType::TspInt) || !matches!(b.t, TspType::TspInt) {
        warn("mod: expected Int");
        return none_val();
    }
    if val_num(&b) == 0.0 {
        warn("division by zero");
        return none_val();
    }
    mk_int((val_num(&a) as i32) % (val_num(&b) as i32).abs())
}

pub fn prim_pow(st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    if tsp_lstlen(&args) != 2 {
        warn("pow: expected 2 arguments");
        return none_val();
    }
    let b = val_car(&args).cloned().unwrap_or_else(nil_val);
    let p = val_car(&val_cdr(&args).cloned().unwrap_or_else(nil_val))
        .cloned()
        .unwrap_or_else(nil_val);
    let bnum = val_num(&b).powf(val_num(&p) / val_den(&p));
    let bden = val_den(&b).powf(val_num(&p) / val_den(&p));
    if (bnum == (bnum as i32) as f64 && bden == (bden as i32) as f64)
        || matches!(b.t, TspType::TspDec)
        || matches!(p.t, TspType::TspDec)
    {
        return mk_num(b.t, p.t, 0)(bnum, bden);
    }
    let pow_sym = mk_sym(st, "^").unwrap_or_else(nil_val);
    let l3 = mk_pair(p, nil_val()).unwrap_or_else(nil_val);
    let l2 = mk_pair(b, l3).unwrap_or_else(nil_val);
    mk_pair(pow_sym, l2).unwrap_or_else(nil_val)
}

pub fn prim_denominator(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if tsp_lstlen(&args) != 1 {
        warn("denominator: expected 1 argument");
        return none_val();
    }
    let v = val_car(&args).cloned().unwrap_or_else(nil_val);
    if !type_match(v.t, TSP_RATIONAL) {
        warn("denominator: expected Int or Ratio");
        return none_val();
    }
    mk_int(val_den(&v) as i32)
}

// Additional comparison and rounding primitives
pub fn prim_lt(st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    cmp_op(st, args, |a, b| a < b)
}

pub fn prim_gt(st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    cmp_op(st, args, |a, b| a > b)
}

pub fn prim_lte(st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    cmp_op(st, args, |a, b| a <= b)
}

pub fn prim_gte(st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    cmp_op(st, args, |a, b| a >= b)
}

fn cmp_op(st: &Tsp, args: Val, op: fn(f64, f64) -> bool) -> Val {
    if tsp_lstlen(&args) != 2 {
        return st.t.clone();
    }
    let a = val_car(&args).cloned().unwrap_or_else(nil_val);
    let b = val_car(&val_cdr(&args).cloned().unwrap_or_else(nil_val))
        .cloned()
        .unwrap_or_else(nil_val);
    if op(val_num(&a) * val_den(&b), val_num(&b) * val_den(&a)) {
        st.t.clone()
    } else {
        st.nil.clone()
    }
}

pub fn prim_int(_st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    round_op(args, 1, |x| x.trunc())
}

pub fn prim_dec(_st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    round_op(args, 2, |x| x)
}

pub fn prim_round(_st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    round_op(args, 0, |x| x.round())
}

pub fn prim_floor(_st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    round_op(args, 0, |x| x.floor())
}

pub fn prim_ceil(_st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    round_op(args, 0, |x| x.ceil())
}

fn round_op(args: Val, force: i32, op: fn(f64) -> f64) -> Val {
    if tsp_lstlen(&args) != 1 {
        warn("round: expected 1 argument");
        return none_val();
    }
    let n = val_car(&args).cloned().unwrap_or_else(nil_val);
    if !type_match(n.t, TSP_NUM) {
        warn("round: expected Num");
        return none_val();
    }
    mk_num(n.t, n.t, force)(op(val_num(&n) / val_den(&n)), 1.0)
}

pub fn prim_numerator(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if tsp_lstlen(&args) != 1 {
        warn("numerator: expected 1 argument");
        return none_val();
    }
    let v = val_car(&args).cloned().unwrap_or_else(nil_val);
    if !type_match(v.t, TSP_RATIONAL) {
        warn("numerator: expected Int or Ratio");
        return none_val();
    }
    mk_int(val_num(&v) as i32)
}

pub fn tib_env_math(st: &mut Tsp) {
    add(st, "Int", TspType::TspPrim);
    add(st, "Dec", TspType::TspPrim);
    add(st, "floor", TspType::TspPrim);
    add(st, "ceil", TspType::TspPrim);
    add(st, "round", TspType::TspPrim);
    add(st, "numerator", TspType::TspPrim);
    add(st, "denominator", TspType::TspPrim);

    add(st, "+", TspType::TspPrim);
    add(st, "-", TspType::TspPrim);
    add(st, "*", TspType::TspPrim);
    add(st, "/", TspType::TspPrim);
    add(st, "mod", TspType::TspPrim);
    add(st, "^", TspType::TspPrim);

    add(st, "<", TspType::TspPrim);
    add(st, ">", TspType::TspPrim);
    add(st, "<=", TspType::TspPrim);
    add(st, ">=", TspType::TspPrim);
}

fn add(st: &mut Tsp, name: &str, t: TspType) {
    let v = mk_prim(t, dummy_prim, name).unwrap_or_else(nil_val);
    tisp_env_add(st, name, v);
}

fn dummy_prim(_st: Tsp, _env: Rec, _args: Val) -> Val {
    none_val()
}
