use crate::tisp::{
    mk_dec, mk_int, mk_prim, mk_rat, stub_prim, tisp_env_add, val_clone, Rec, Tsp, TspType, Val,
    ValUnion, TSP_NUM,
};

fn type_bits(t: TspType) -> u32 {
    t as u32
}

fn type_matches(t: TspType, mask: u32) -> bool {
    (type_bits(t) & mask) != 0
}

fn val_num(v: &Val) -> (f64, f64) {
    match &v.v {
        ValUnion::N { num, den } => (*num, *den),
        _ => (0.0, 1.0),
    }
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

fn none_val(st: &Tsp) -> Val {
    val_clone(&st.none)
}

pub fn create_int(num: f64, _den: f64) -> Val {
    mk_int(num as i32)
}

pub fn create_dec(num: f64, _den: f64) -> Val {
    mk_dec(num).unwrap_or_else(|| mk_int(0))
}

pub fn create_rat(num: f64, den: f64) -> Val {
    mk_rat(num as i32, den as i32).unwrap_or_else(|| mk_int(0))
}

pub fn mk_num(a: TspType, b: TspType, force: i32) -> fn(f64, f64) -> Val {
    if force == 1 {
        return create_rat;
    }
    if force == 2 {
        return create_dec;
    }
    if type_matches(a, TspType::TspDec as u32) || type_matches(b, TspType::TspDec as u32) {
        return create_dec;
    }
    if type_matches(a, TspType::TspRatio as u32) || type_matches(b, TspType::TspRatio as u32) {
        return create_rat;
    }
    create_int
}

pub fn prim_add(st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    let a = car(&args);
    let b = car(&cdr(&args));
    if !type_matches(a.t, TSP_NUM) || !type_matches(b.t, TSP_NUM) {
        return none_val(st);
    }
    let (an, ad) = val_num(&a);
    let (bn, bd) = val_num(&b);
    if matches!(a.t, TspType::TspDec) || matches!(b.t, TspType::TspDec) {
        return mk_dec(an / ad + bn / bd).unwrap_or_else(|| none_val(st));
    }
    let f = mk_num(a.t, b.t, 0);
    f(an * bd + ad * bn, ad * bd)
}

pub fn prim_sub(st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    let len = crate::tisp::tsp_lstlen(&args);
    if len != 1 && len != 2 {
        return none_val(st);
    }
    let a_in = car(&args);
    if !type_matches(a_in.t, TSP_NUM) {
        return none_val(st);
    }
    let (a, b) = if len == 1 {
        (mk_int(0), a_in)
    } else {
        let b = car(&cdr(&args));
        if !type_matches(b.t, TSP_NUM) {
            return none_val(st);
        }
        (a_in, b)
    };
    let (an, ad) = val_num(&a);
    let (bn, bd) = val_num(&b);
    if matches!(a.t, TspType::TspDec) || matches!(b.t, TspType::TspDec) {
        return mk_dec(an / ad - bn / bd).unwrap_or_else(|| none_val(st));
    }
    let f = mk_num(a.t, b.t, 0);
    f(an * bd - ad * bn, ad * bd)
}

pub fn prim_mul(st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    let a = car(&args);
    let b = car(&cdr(&args));
    if !type_matches(a.t, TSP_NUM) || !type_matches(b.t, TSP_NUM) {
        return none_val(st);
    }
    let (an, ad) = val_num(&a);
    let (bn, bd) = val_num(&b);
    if matches!(a.t, TspType::TspDec) || matches!(b.t, TspType::TspDec) {
        return mk_dec((an / ad) * (bn / bd)).unwrap_or_else(|| none_val(st));
    }
    let f = mk_num(a.t, b.t, 0);
    f(an * bn, ad * bd)
}

pub fn prim_div(st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    let len = crate::tisp::tsp_lstlen(&args);
    if len != 1 && len != 2 {
        return none_val(st);
    }
    let a_in = car(&args);
    if !type_matches(a_in.t, TSP_NUM) {
        return none_val(st);
    }
    let (a, b) = if len == 1 {
        (mk_int(1), a_in)
    } else {
        let b = car(&cdr(&args));
        if !type_matches(b.t, TSP_NUM) {
            return none_val(st);
        }
        (a_in, b)
    };
    let (an, ad) = val_num(&a);
    let (bn, bd) = val_num(&b);
    if matches!(a.t, TspType::TspDec) || matches!(b.t, TspType::TspDec) {
        if (bn / bd) == 0.0 {
            return none_val(st);
        }
        return mk_dec((an / ad) / (bn / bd)).unwrap_or_else(|| none_val(st));
    }
    let f = mk_num(a.t, b.t, 1);
    f(an * bd, ad * bn)
}

pub fn prim_mod(st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    let a = car(&args);
    let b = car(&cdr(&args));
    if !matches!(a.t, TspType::TspInt) || !matches!(b.t, TspType::TspInt) {
        return none_val(st);
    }
    let (an, _) = val_num(&a);
    let (bn, _) = val_num(&b);
    if bn == 0.0 {
        return none_val(st);
    }
    mk_int((an as i32) % (bn as i32).abs())
}

pub fn prim_pow(st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    let b = car(&args);
    let p = car(&cdr(&args));
    let (bn, bd) = val_num(&b);
    let (pn, pd) = val_num(&p);
    let exp = pn / pd;
    let bnum = bn.powf(exp);
    let bden = bd.powf(exp);
    let bnum_int = bnum == (bnum as i32) as f64;
    let bden_int = bden == (bden as i32) as f64;
    if (bnum_int && bden_int)
        || matches!(b.t, TspType::TspDec)
        || matches!(p.t, TspType::TspDec)
    {
        let f = mk_num(b.t, p.t, 0);
        return f(bnum, bden);
    }
    none_val(st)
}

pub fn prim_denominator(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    let a = car(&args);
    if !matches!(a.t, TspType::TspInt | TspType::TspRatio) {
        return none_val(st);
    }
    let (_, d) = val_num(&a);
    mk_int(d as i32)
}

pub fn prim_numerator(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    let a = car(&args);
    if !matches!(a.t, TspType::TspInt | TspType::TspRatio) {
        return none_val(st);
    }
    let (n, _) = val_num(&a);
    mk_int(n as i32)
}

pub fn prim_lt(st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    cmp_op(st, args, |x, y| x < y)
}

pub fn prim_gt(st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    cmp_op(st, args, |x, y| x > y)
}

pub fn prim_lte(st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    cmp_op(st, args, |x, y| x <= y)
}

pub fn prim_gte(st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    cmp_op(st, args, |x, y| x >= y)
}

fn cmp_op(st: &mut Tsp, args: Val, op: fn(f64, f64) -> bool) -> Val {
    if crate::tisp::tsp_lstlen(&args) != 2 {
        return val_clone(&st.t);
    }
    let a = car(&args);
    let b = car(&cdr(&args));
    if !type_matches(a.t, TSP_NUM) || !type_matches(b.t, TSP_NUM) {
        return val_clone(&st.nil);
    }
    let (an, ad) = val_num(&a);
    let (bn, bd) = val_num(&b);
    if op(an * bd, bn * ad) {
        val_clone(&st.t)
    } else {
        val_clone(&st.nil)
    }
}

fn round_op(st: &mut Tsp, args: Val, force: i32, op: fn(f64) -> f64) -> Val {
    let n = car(&args);
    if !type_matches(n.t, TSP_NUM) {
        return none_val(st);
    }
    let (nn, nd) = val_num(&n);
    let f = mk_num(n.t, n.t, force);
    f(op(nn / nd), 1.0)
}

pub fn prim_int(st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    round_op(st, args, 1, |x| x)
}

pub fn prim_dec(st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    round_op(st, args, 2, |x| x)
}

pub fn prim_round(st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    round_op(st, args, 0, |x| x.round())
}

pub fn prim_floor(st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    round_op(st, args, 0, |x| x.floor())
}

pub fn prim_ceil(st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    round_op(st, args, 0, |x| x.ceil())
}

fn trig_op(st: &mut Tsp, args: Val, op: fn(f64) -> f64) -> Val {
    let a = car(&args);
    if matches!(a.t, TspType::TspDec) {
        let (n, _) = val_num(&a);
        return mk_dec(op(n)).unwrap_or_else(|| none_val(st));
    }
    none_val(st)
}

pub fn prim_sin(st: &mut Tsp, _v: &mut Rec, a: Val) -> Val {
    trig_op(st, a, f64::sin)
}
pub fn prim_cos(st: &mut Tsp, _v: &mut Rec, a: Val) -> Val {
    trig_op(st, a, f64::cos)
}
pub fn prim_tan(st: &mut Tsp, _v: &mut Rec, a: Val) -> Val {
    trig_op(st, a, f64::tan)
}
pub fn prim_sinh(st: &mut Tsp, _v: &mut Rec, a: Val) -> Val {
    trig_op(st, a, f64::sinh)
}
pub fn prim_cosh(st: &mut Tsp, _v: &mut Rec, a: Val) -> Val {
    trig_op(st, a, f64::cosh)
}
pub fn prim_tanh(st: &mut Tsp, _v: &mut Rec, a: Val) -> Val {
    trig_op(st, a, f64::tanh)
}
pub fn prim_asin(st: &mut Tsp, _v: &mut Rec, a: Val) -> Val {
    trig_op(st, a, f64::asin)
}
pub fn prim_acos(st: &mut Tsp, _v: &mut Rec, a: Val) -> Val {
    trig_op(st, a, f64::acos)
}
pub fn prim_atan(st: &mut Tsp, _v: &mut Rec, a: Val) -> Val {
    trig_op(st, a, f64::atan)
}
pub fn prim_asinh(st: &mut Tsp, _v: &mut Rec, a: Val) -> Val {
    trig_op(st, a, f64::asinh)
}
pub fn prim_acosh(st: &mut Tsp, _v: &mut Rec, a: Val) -> Val {
    trig_op(st, a, f64::acosh)
}
pub fn prim_atanh(st: &mut Tsp, _v: &mut Rec, a: Val) -> Val {
    trig_op(st, a, f64::atanh)
}
pub fn prim_exp(st: &mut Tsp, _v: &mut Rec, a: Val) -> Val {
    trig_op(st, a, f64::exp)
}
pub fn prim_log(st: &mut Tsp, _v: &mut Rec, a: Val) -> Val {
    trig_op(st, a, f64::ln)
}

pub fn tib_env_math(st: &mut Tsp) {
    let names: &[&str] = &[
        "Int",
        "Dec",
        "floor",
        "ceil",
        "round",
        "numerator",
        "denominator",
        "+",
        "-",
        "*",
        "/",
        "mod",
        "^",
        "<",
        ">",
        "<=",
        ">=",
        "sin",
        "cos",
        "tan",
        "sinh",
        "cosh",
        "tanh",
        "arcsin",
        "arccos",
        "arctan",
        "arcsinh",
        "arccosh",
        "arctanh",
        "exp",
        "log",
    ];
    for name in names {
        let v = mk_prim(TspType::TspPrim, stub_prim, name).unwrap();
        tisp_env_add(st, name, v);
    }
}
