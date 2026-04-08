use crate::tisp::*;

fn tsp_arg_num_check(args: &Val, name: &str, nargs: i32) -> bool {
    if nargs > -1 && tsp_lstlen(args) != nargs {
        eprintln!("; tisp: error: {}: expected {} argument{}, received {}",
            name, nargs, if nargs > 1 { "s" } else { "" }, tsp_lstlen(args));
        false
    } else { true }
}

fn tsp_arg_type_check(arg: &Val, name: &str, type_bits: u32) -> bool {
    if (arg.t as u32) & type_bits == 0 {
        eprintln!("; tisp: error: {}: expected {}, received {}",
            name, tsp_type_str_bits(type_bits), tsp_type_str(arg.t));
        false
    } else { true }
}

pub fn create_int(num: f64, den: f64) -> Val {
    mk_int(num as i32)
}

pub fn create_dec(num: f64, den: f64) -> Val {
    mk_dec(num).unwrap()
}

pub fn create_rat(num: f64, den: f64) -> Val {
    mk_rat(num as i32, den as i32).unwrap_or_else(|| mk_err())
}

pub fn mk_num(a: TspType, b: TspType, force: i32) -> fn(f64, f64) -> Val {
    if force == 1 { return create_rat; }
    if force == 2 { return create_dec; }
    if a as u32 & TspType::TspDec as u32 != 0 || b as u32 & TspType::TspDec as u32 != 0 { return create_dec; }
    if a as u32 & TspType::TspRatio as u32 != 0 || b as u32 & TspType::TspRatio as u32 != 0 { return create_rat; }
    create_int
}

// Rounding primitives
fn prim_Int(_st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    if !tsp_arg_num_check(&args, "Int", 1) { return mk_err(); }
    let n = car(&args);
    if !tsp_arg_type_check(n, "Int", TSP_NUM) { return mk_err(); }
    let v = vnum(n) / vden(n);
    mk_num(n.t, n.t, 1)(v, 1.0)
}

fn prim_Dec(_st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    if !tsp_arg_num_check(&args, "Dec", 1) { return mk_err(); }
    let n = car(&args);
    if !tsp_arg_type_check(n, "Dec", TSP_NUM) { return mk_err(); }
    let v = vnum(n) / vden(n);
    mk_num(n.t, n.t, 2)(v, 1.0)
}

fn prim_round(_st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    if !tsp_arg_num_check(&args, "round", 1) { return mk_err(); }
    let n = car(&args);
    if !tsp_arg_type_check(n, "round", TSP_NUM) { return mk_err(); }
    let v = (vnum(n) / vden(n)).round();
    mk_num(n.t, n.t, 0)(v, 1.0)
}

fn prim_floor(_st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    if !tsp_arg_num_check(&args, "floor", 1) { return mk_err(); }
    let n = car(&args);
    if !tsp_arg_type_check(n, "floor", TSP_NUM) { return mk_err(); }
    let v = (vnum(n) / vden(n)).floor();
    mk_num(n.t, n.t, 0)(v, 1.0)
}

fn prim_ceil(_st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    if !tsp_arg_num_check(&args, "ceil", 1) { return mk_err(); }
    let n = car(&args);
    if !tsp_arg_type_check(n, "ceil", TSP_NUM) { return mk_err(); }
    let v = (vnum(n) / vden(n)).ceil();
    mk_num(n.t, n.t, 0)(v, 1.0)
}

pub fn prim_add(_st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    if !tsp_arg_num_check(&args, "+", 2) { return mk_err(); }
    let a = car(&args); let b = car(cdr(&args));
    if !tsp_arg_type_check(a, "+", TSP_NUM) { return mk_err(); }
    if !tsp_arg_type_check(b, "+", TSP_NUM) { return mk_err(); }
    if a.t as u32 & TspType::TspDec as u32 != 0 || b.t as u32 & TspType::TspDec as u32 != 0 {
        return mk_dec(vnum(a)/vden(a) + vnum(b)/vden(b)).unwrap();
    }
    mk_num(a.t, b.t, 0)(vnum(a)*vden(b) + vden(a)*vnum(b), vden(a)*vden(b))
}

pub fn prim_sub(_st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    let len = tsp_lstlen(&args);
    if len != 2 && len != 1 {
        eprintln!("; tisp: error: -: expected 1 or 2 arguments, recieved {}", len);
        return mk_err();
    }
    let a_orig = car(&args);
    if !tsp_arg_type_check(a_orig, "-", TSP_NUM) { return mk_err(); }
    let (a, b) = if len == 1 {
        (&mk_int(0), a_orig)
    } else {
        let b = car(cdr(&args));
        if !tsp_arg_type_check(b, "-", TSP_NUM) { return mk_err(); }
        (a_orig, b)
    };
    if a.t as u32 & TspType::TspDec as u32 != 0 || b.t as u32 & TspType::TspDec as u32 != 0 {
        return mk_dec(vnum(a)/vden(a) - vnum(b)/vden(b)).unwrap();
    }
    mk_num(a.t, b.t, 0)(vnum(a)*vden(b) - vden(a)*vnum(b), vden(a)*vden(b))
}

pub fn prim_mul(_st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    if !tsp_arg_num_check(&args, "*", 2) { return mk_err(); }
    let a = car(&args); let b = car(cdr(&args));
    if !tsp_arg_type_check(a, "*", TSP_NUM) { return mk_err(); }
    if !tsp_arg_type_check(b, "*", TSP_NUM) { return mk_err(); }
    if a.t as u32 & TspType::TspDec as u32 != 0 || b.t as u32 & TspType::TspDec as u32 != 0 {
        return mk_dec((vnum(a)/vden(a)) * (vnum(b)/vden(b))).unwrap();
    }
    mk_num(a.t, b.t, 0)(vnum(a)*vnum(b), vden(a)*vden(b))
}

pub fn prim_div(_st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    let len = tsp_lstlen(&args);
    if len != 2 && len != 1 {
        eprintln!("; tisp: error: /: expected 1 or 2 arguments, recieved {}", len);
        return mk_err();
    }
    let a_orig = car(&args);
    if !tsp_arg_type_check(a_orig, "/", TSP_NUM) { return mk_err(); }
    let (a, b) = if len == 1 {
        (&mk_int(1), a_orig)
    } else {
        let b = car(cdr(&args));
        if !tsp_arg_type_check(b, "/", TSP_NUM) { return mk_err(); }
        (a_orig, b)
    };
    if a.t as u32 & TspType::TspDec as u32 != 0 || b.t as u32 & TspType::TspDec as u32 != 0 {
        return mk_dec((vnum(a)/vden(a)) / (vnum(b)/vden(b))).unwrap();
    }
    mk_num(a.t, b.t, 1)(vnum(a)*vden(b), vden(a)*vnum(b))
}

pub fn prim_mod(_st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    if !tsp_arg_num_check(&args, "mod", 2) { return mk_err(); }
    let a = car(&args); let b = car(cdr(&args));
    if !tsp_arg_type_check(a, "mod", TspType::TspInt as u32) { return mk_err(); }
    if !tsp_arg_type_check(b, "mod", TspType::TspInt as u32) { return mk_err(); }
    if vnum(b) == 0.0 {
        eprintln!("; tisp: error: division by zero");
        return mk_err();
    }
    mk_int((vnum(a) as i32) % (vnum(b) as i32).abs())
}

pub fn prim_pow(st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    if !tsp_arg_num_check(&args, "pow", 2) { return mk_err(); }
    let b = car(&args); let p = car(cdr(&args));
    if !tsp_arg_type_check(b, "pow", TSP_EXPR) { return mk_err(); }
    if !tsp_arg_type_check(p, "pow", TSP_EXPR) { return mk_err(); }
    let bnum = (vnum(b) as f64).powf(vnum(p)/vden(p));
    let bden = (vden(b) as f64).powf(vnum(p)/vden(p));
    if (bnum == (bnum as i32) as f64 && bden == (bden as i32) as f64) ||
        b.t as u32 & TspType::TspDec as u32 != 0 || p.t as u32 & TspType::TspDec as u32 != 0 {
        return mk_num(b.t, p.t, 0)(bnum, bden);
    }
    let sym = mk_sym_val(st, "^");
    mk_list(st, 3, vec![sym, clone_val(b), clone_val(p)]).unwrap_or_else(|| mk_err())
}

// Comparison macros
macro_rules! prim_compare {
    ($name:ident, $op_str:expr, $op:tt) => {
        fn $name(st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
            if tsp_lstlen(&args) != 2 { return clone_val(&st.t); }
            if !tsp_arg_type_check(car(&args), $op_str, TSP_NUM) { return mk_err(); }
            if !tsp_arg_type_check(car(cdr(&args)), $op_str, TSP_NUM) { return mk_err(); }
            let a = car(&args); let b = car(cdr(&args));
            if (vnum(a)*vden(b)) $op (vnum(b)*vden(a)) {
                clone_val(&st.t)
            } else {
                clone_val(&st.nil)
            }
        }
    };
}

prim_compare!(prim_lt, "<", <);
prim_compare!(prim_gt, ">", >);
prim_compare!(prim_lte, "<=", <=);
prim_compare!(prim_gte, ">=", >=);

// Trig macros
macro_rules! prim_trig {
    ($name:ident, $fname:expr, $func:path) => {
        fn $name(st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
            if !tsp_arg_num_check(&args, $fname, 1) { return mk_err(); }
            if !tsp_arg_type_check(car(&args), $fname, TSP_EXPR) { return mk_err(); }
            let a = car(&args);
            if a.t as u32 & TspType::TspDec as u32 != 0 {
                return mk_dec($func(vnum(a))).unwrap();
            }
            let sym = mk_sym_val(st, $fname);
            mk_list(st, 2, vec![sym, clone_val(a)]).unwrap_or_else(|| mk_err())
        }
    };
}

prim_trig!(prim_sin, "sin", f64::sin);
prim_trig!(prim_cos, "cos", f64::cos);
prim_trig!(prim_tan, "tan", f64::tan);
prim_trig!(prim_sinh, "sinh", f64::sinh);
prim_trig!(prim_cosh, "cosh", f64::cosh);
prim_trig!(prim_tanh, "tanh", f64::tanh);
prim_trig!(prim_asin, "arcsin", f64::asin);
prim_trig!(prim_acos, "arccos", f64::acos);
prim_trig!(prim_atan, "arctan", f64::atan);
prim_trig!(prim_asinh, "arcsinh", f64::asinh);
prim_trig!(prim_acosh, "arccosh", f64::acosh);
prim_trig!(prim_atanh, "arctanh", f64::atanh);
prim_trig!(prim_exp, "exp", f64::exp);
prim_trig!(prim_log, "log", f64::ln);

fn prim_numerator(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if !tsp_arg_num_check(&args, "numerator", 1) { return mk_err(); }
    if !tsp_arg_type_check(car(&args), "numerator", TspType::TspInt as u32 | TspType::TspRatio as u32) { return mk_err(); }
    mk_int(vnum(car(&args)) as i32)
}

pub fn prim_denominator(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if !tsp_arg_num_check(&args, "denominator", 1) { return mk_err(); }
    if !tsp_arg_type_check(car(&args), "denominator", TspType::TspInt as u32 | TspType::TspRatio as u32) { return mk_err(); }
    mk_int(vden(car(&args)) as i32)
}

pub fn tib_env_math(st: &mut Tsp) {
    tisp_env_add(st, "Int", mk_prim(TspType::TspPrim, prim_Int, "Int").unwrap());
    tisp_env_add(st, "Dec", mk_prim(TspType::TspPrim, prim_Dec, "Dec").unwrap());
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
