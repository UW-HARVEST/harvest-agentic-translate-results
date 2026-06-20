use crate::tisp::{
    Rec, TSP_NUM, Tsp, TspType, Val, ValUnion, expect_len, expect_type, mk_dec, mk_int, mk_list,
    mk_prim, mk_rat, mk_sym, pair_car, pair_cdr, tisp_env_add, type_matches, val_den, val_num,
};

pub fn create_int(num: f64, _den: f64) -> Val {
    mk_int(num as i32)
}

pub fn create_dec(num: f64, _den: f64) -> Val {
    mk_dec(num).unwrap()
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
    if a == TspType::TspDec || b == TspType::TspDec {
        return create_dec;
    }
    if a == TspType::TspRatio || b == TspType::TspRatio {
        return create_rat;
    }
    create_int
}

fn round_prim(st: &mut Tsp, args: Val, name: &str, force: i32, f: fn(f64) -> f64) -> Val {
    if !expect_len(st, &args, name, 1) {
        return st.none.clone();
    }
    let n = pair_car(&args).clone();
    if !expect_type(st, &n, name, TSP_NUM) {
        return st.none.clone();
    }
    mk_num(n.t, n.t, force)(f(val_num(&n) / val_den(&n)), 1.0)
}

pub fn prim_add(st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    if !expect_len(st, &args, "+", 2) {
        return st.none.clone();
    }
    let a = pair_car(&args).clone();
    let b = pair_car(pair_cdr(&args)).clone();
    if !expect_type(st, &a, "+", TSP_NUM) || !expect_type(st, &b, "+", TSP_NUM) {
        return st.none.clone();
    }
    if a.t == TspType::TspDec || b.t == TspType::TspDec {
        return mk_dec((val_num(&a) / val_den(&a)) + (val_num(&b) / val_den(&b))).unwrap();
    }
    mk_num(a.t, b.t, 0)(
        val_num(&a) * val_den(&b) + val_den(&a) * val_num(&b),
        val_den(&a) * val_den(&b),
    )
}

pub fn prim_sub(st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    let len = crate::tisp::tsp_lstlen(&args);
    if len != 1 && len != 2 {
        return st.none.clone();
    }
    let mut a = pair_car(&args).clone();
    if !expect_type(st, &a, "-", TSP_NUM) {
        return st.none.clone();
    }
    let b = if len == 1 {
        let out = a.clone();
        a = mk_int(0);
        out
    } else {
        let v = pair_car(pair_cdr(&args)).clone();
        if !expect_type(st, &v, "-", TSP_NUM) {
            return st.none.clone();
        }
        v
    };
    if a.t == TspType::TspDec || b.t == TspType::TspDec {
        return mk_dec((val_num(&a) / val_den(&a)) - (val_num(&b) / val_den(&b))).unwrap();
    }
    mk_num(a.t, b.t, 0)(
        val_num(&a) * val_den(&b) - val_den(&a) * val_num(&b),
        val_den(&a) * val_den(&b),
    )
}

pub fn prim_mul(st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    if !expect_len(st, &args, "*", 2) {
        return st.none.clone();
    }
    let a = pair_car(&args).clone();
    let b = pair_car(pair_cdr(&args)).clone();
    if !expect_type(st, &a, "*", TSP_NUM) || !expect_type(st, &b, "*", TSP_NUM) {
        return st.none.clone();
    }
    if a.t == TspType::TspDec || b.t == TspType::TspDec {
        return mk_dec((val_num(&a) / val_den(&a)) * (val_num(&b) / val_den(&b))).unwrap();
    }
    mk_num(a.t, b.t, 0)(val_num(&a) * val_num(&b), val_den(&a) * val_den(&b))
}

pub fn prim_div(st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    let len = crate::tisp::tsp_lstlen(&args);
    if len != 1 && len != 2 {
        return st.none.clone();
    }
    let mut a = pair_car(&args).clone();
    if !expect_type(st, &a, "/", TSP_NUM) {
        return st.none.clone();
    }
    let b = if len == 1 {
        let out = a.clone();
        a = mk_int(1);
        out
    } else {
        let v = pair_car(pair_cdr(&args)).clone();
        if !expect_type(st, &v, "/", TSP_NUM) {
            return st.none.clone();
        }
        v
    };
    if a.t == TspType::TspDec || b.t == TspType::TspDec {
        return mk_dec((val_num(&a) / val_den(&a)) / (val_num(&b) / val_den(&b))).unwrap();
    }
    mk_num(a.t, b.t, 1)(val_num(&a) * val_den(&b), val_den(&a) * val_num(&b))
}

pub fn prim_mod(st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    if !expect_len(st, &args, "mod", 2) {
        return st.none.clone();
    }
    let a = pair_car(&args).clone();
    let b = pair_car(pair_cdr(&args)).clone();
    if !expect_type(st, &a, "mod", TspType::TspInt as u32)
        || !expect_type(st, &b, "mod", TspType::TspInt as u32)
    {
        return st.none.clone();
    }
    let divisor = val_num(&b) as i32;
    if divisor == 0 {
        return st.none.clone();
    }
    mk_int((val_num(&a) as i32) % divisor.abs())
}

pub fn prim_pow(st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    if !expect_len(st, &args, "pow", 2) {
        return st.none.clone();
    }
    let b = pair_car(&args).clone();
    let p = pair_car(pair_cdr(&args)).clone();
    if !expect_type(st, &b, "pow", TSP_NUM | TspType::TspSym as u32 | TspType::TspPair as u32)
        || !expect_type(st, &p, "pow", TSP_NUM | TspType::TspSym as u32 | TspType::TspPair as u32)
    {
        return st.none.clone();
    }
    let bnum = val_num(&b).powf(val_num(&p) / val_den(&p));
    let bden = val_den(&b).powf(val_num(&p) / val_den(&p));
    if ((bnum.fract() == 0.0 && bden.fract() == 0.0) || b.t == TspType::TspDec || p.t == TspType::TspDec)
        && type_matches(b.t, TSP_NUM)
        && type_matches(p.t, TSP_NUM)
    {
        return mk_num(b.t, p.t, 0)(bnum, bden);
    }
    let caret = mk_sym(st, "^").unwrap_or_else(|| st.none.clone());
    mk_list(st, 3, vec![caret, b, p]).unwrap_or_else(|| st.none.clone())
}

pub fn prim_denominator(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if !expect_len(st, &args, "denominator", 1) {
        return st.none.clone();
    }
    let arg = pair_car(&args).clone();
    if !expect_type(st, &arg, "denominator", TspType::TspInt as u32 | TspType::TspRatio as u32) {
        return st.none.clone();
    }
    mk_int(val_den(&arg) as i32)
}

fn prim_numerator(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if !expect_len(st, &args, "numerator", 1) {
        return st.none.clone();
    }
    let arg = pair_car(&args).clone();
    if !expect_type(st, &arg, "numerator", TspType::TspInt as u32 | TspType::TspRatio as u32) {
        return st.none.clone();
    }
    mk_int(val_num(&arg) as i32)
}

fn prim_lt(st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    compare(st, args, "<", |a, b| a < b)
}

fn prim_gt(st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    compare(st, args, ">", |a, b| a > b)
}

fn prim_lte(st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    compare(st, args, "<=", |a, b| a <= b)
}

fn prim_gte(st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    compare(st, args, ">=", |a, b| a >= b)
}

fn compare(st: &mut Tsp, args: Val, name: &str, op: fn(f64, f64) -> bool) -> Val {
    if crate::tisp::tsp_lstlen(&args) != 2 {
        return st.t.clone();
    }
    let a = pair_car(&args).clone();
    let b = pair_car(pair_cdr(&args)).clone();
    if !expect_type(st, &a, name, TSP_NUM) || !expect_type(st, &b, name, TSP_NUM) {
        return st.nil.clone();
    }
    if op(val_num(&a) * val_den(&b), val_num(&b) * val_den(&a)) {
        st.t.clone()
    } else {
        st.nil.clone()
    }
}

fn trig(st: &mut Tsp, args: Val, name: &str, f: fn(f64) -> f64) -> Val {
    if !expect_len(st, &args, name, 1) {
        return st.none.clone();
    }
    let arg = pair_car(&args).clone();
    if !expect_type(st, &arg, name, TSP_NUM | TspType::TspSym as u32 | TspType::TspPair as u32) {
        return st.none.clone();
    }
    if arg.t == TspType::TspDec {
        return mk_dec(f(val_num(&arg))).unwrap();
    }
    let sym = mk_sym(st, name).unwrap_or_else(|| st.none.clone());
    mk_list(st, 2, vec![sym, arg]).unwrap_or_else(|| st.none.clone())
}

fn prim_int(st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    round_prim(st, args, "Int", 1, |x| x)
}

fn prim_dec(st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    round_prim(st, args, "Dec", 2, |x| x)
}

fn prim_floor(st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    round_prim(st, args, "floor", 0, f64::floor)
}

fn prim_ceil(st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    round_prim(st, args, "ceil", 0, f64::ceil)
}

fn prim_round(st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    round_prim(st, args, "round", 0, f64::round)
}

fn prim_sin(st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val { trig(st, args, "sin", f64::sin) }
fn prim_cos(st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val { trig(st, args, "cos", f64::cos) }
fn prim_tan(st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val { trig(st, args, "tan", f64::tan) }
fn prim_sinh(st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val { trig(st, args, "sinh", f64::sinh) }
fn prim_cosh(st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val { trig(st, args, "cosh", f64::cosh) }
fn prim_tanh(st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val { trig(st, args, "tanh", f64::tanh) }
fn prim_asin(st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val { trig(st, args, "arcsin", f64::asin) }
fn prim_acos(st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val { trig(st, args, "arccos", f64::acos) }
fn prim_atan(st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val { trig(st, args, "arctan", f64::atan) }
fn prim_asinh(st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val { trig(st, args, "arcsinh", f64::asinh) }
fn prim_acosh(st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val { trig(st, args, "arccosh", f64::acosh) }
fn prim_atanh(st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val { trig(st, args, "arctanh", f64::atanh) }
fn prim_exp(st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val { trig(st, args, "exp", f64::exp) }
fn prim_log(st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val { trig(st, args, "log", f64::ln) }

pub fn tib_env_math(st: &mut Tsp) {
    tisp_env_add(st, "Int", mk_prim(TspType::TspPrim, prim_int, "Int").unwrap());
    tisp_env_add(st, "Dec", mk_prim(TspType::TspPrim, prim_dec, "Dec").unwrap());
    tisp_env_add(st, "floor", mk_prim(TspType::TspPrim, prim_floor, "floor").unwrap());
    tisp_env_add(st, "ceil", mk_prim(TspType::TspPrim, prim_ceil, "ceil").unwrap());
    tisp_env_add(st, "round", mk_prim(TspType::TspPrim, prim_round, "round").unwrap());
    tisp_env_add(st, "numerator", mk_prim(TspType::TspPrim, prim_numerator, "numerator").unwrap());
    tisp_env_add(st, "denominator", mk_prim(TspType::TspPrim, prim_denominator, "denominator").unwrap());
    tisp_env_add(st, "+", mk_prim(TspType::TspPrim, prim_add, "+").unwrap());
    tisp_env_add(st, "-", mk_prim(TspType::TspPrim, prim_sub, "-").unwrap());
    tisp_env_add(st, "*", mk_prim(TspType::TspPrim, prim_mul, "*").unwrap());
    tisp_env_add(st, "/", mk_prim(TspType::TspPrim, prim_div, "/").unwrap());
    tisp_env_add(st, "mod", mk_prim(TspType::TspPrim, prim_mod, "mod").unwrap());
    tisp_env_add(st, "^", mk_prim(TspType::TspPrim, prim_pow, "^").unwrap());
    tisp_env_add(st, "<", mk_prim(TspType::TspPrim, prim_lt, "<").unwrap());
    tisp_env_add(st, ">", mk_prim(TspType::TspPrim, prim_gt, ">").unwrap());
    tisp_env_add(st, "<=", mk_prim(TspType::TspPrim, prim_lte, "<=").unwrap());
    tisp_env_add(st, ">=", mk_prim(TspType::TspPrim, prim_gte, ">=").unwrap());
    tisp_env_add(st, "sin", mk_prim(TspType::TspPrim, prim_sin, "sin").unwrap());
    tisp_env_add(st, "cos", mk_prim(TspType::TspPrim, prim_cos, "cos").unwrap());
    tisp_env_add(st, "tan", mk_prim(TspType::TspPrim, prim_tan, "tan").unwrap());
    tisp_env_add(st, "sinh", mk_prim(TspType::TspPrim, prim_sinh, "sinh").unwrap());
    tisp_env_add(st, "cosh", mk_prim(TspType::TspPrim, prim_cosh, "cosh").unwrap());
    tisp_env_add(st, "tanh", mk_prim(TspType::TspPrim, prim_tanh, "tanh").unwrap());
    tisp_env_add(st, "arcsin", mk_prim(TspType::TspPrim, prim_asin, "arcsin").unwrap());
    tisp_env_add(st, "arccos", mk_prim(TspType::TspPrim, prim_acos, "arccos").unwrap());
    tisp_env_add(st, "arctan", mk_prim(TspType::TspPrim, prim_atan, "arctan").unwrap());
    tisp_env_add(st, "arcsinh", mk_prim(TspType::TspPrim, prim_asinh, "arcsinh").unwrap());
    tisp_env_add(st, "arccosh", mk_prim(TspType::TspPrim, prim_acosh, "arccosh").unwrap());
    tisp_env_add(st, "arctanh", mk_prim(TspType::TspPrim, prim_atanh, "arctanh").unwrap());
    tisp_env_add(st, "exp", mk_prim(TspType::TspPrim, prim_exp, "exp").unwrap());
    tisp_env_add(st, "log", mk_prim(TspType::TspPrim, prim_log, "log").unwrap());
}
