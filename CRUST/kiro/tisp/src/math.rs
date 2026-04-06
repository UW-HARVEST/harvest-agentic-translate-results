use crate::tisp::*;

pub fn create_int(num: f64, _den: f64) -> Val { mk_int(num as i32) }
pub fn create_dec(num: f64, _den: f64) -> Val { mk_dec(num).unwrap() }
pub fn create_rat(num: f64, den: f64) -> Val { mk_rat(num as i32, den as i32).unwrap_or_else(mk_nil_pub) }

pub fn mk_num(a: TspType, b: TspType, force: i32) -> fn(f64, f64) -> Val {
    if force == 1 { return create_rat; }
    if force == 2 { return create_dec; }
    if a == TspType::TspDec || b == TspType::TspDec { return create_dec; }
    if a == TspType::TspRatio || b == TspType::TspRatio { return create_rat; }
    create_int
}

macro_rules! prim_round {
    ($name:ident, $func:expr, $force:expr) => {
        fn $name(st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
            if tsp_arg_num_check_pub(&args, stringify!($name), 1).is_none() { return mk_error(); }
            let n = car_pub(&args);
            if tsp_arg_type_check_pub(n, stringify!($name), TSP_NUM).is_none() { return mk_error(); }
            let v = num_pub(n) / den_pub(n);
            (mk_num(n.t, n.t, $force))($func(v), 1.0)
        }
    };
}

fn identity(x: f64) -> f64 { x }
prim_round!(prim_Int, identity, 1);
prim_round!(prim_Dec, identity, 2);
prim_round!(prim_round, f64::round, 0);
prim_round!(prim_floor, f64::floor, 0);
prim_round!(prim_ceil, f64::ceil, 0);

pub fn prim_add(_st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    if tsp_arg_num_check_pub(&args, "+", 2).is_none() { return mk_error(); }
    let a = car_pub(&args); let b = car_pub(cdr_pub(&args));
    if tsp_arg_type_check_pub(a, "+", TSP_NUM).is_none() { return mk_error(); }
    if tsp_arg_type_check_pub(b, "+", TSP_NUM).is_none() { return mk_error(); }
    if a.t == TspType::TspDec || b.t == TspType::TspDec {
        return mk_dec(num_pub(a)/den_pub(a) + num_pub(b)/den_pub(b)).unwrap();
    }
    (mk_num(a.t, b.t, 0))(num_pub(a)*den_pub(b) + den_pub(a)*num_pub(b), den_pub(a)*den_pub(b))
}
pub fn prim_sub(_st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    let len = tsp_lstlen(&args);
    if len != 2 && len != 1 {
        eprintln!("; tisp: error: -: expected 1 or 2 arguments, recieved {}", len);
        return mk_error();
    }
    let a_orig = car_pub(&args);
    if tsp_arg_type_check_pub(a_orig, "-", TSP_NUM).is_none() { return mk_error(); }
    let (a, b) = if len == 1 {
        (mk_int(0), clone_val_pub(a_orig))
    } else {
        let b = car_pub(cdr_pub(&args));
        if tsp_arg_type_check_pub(b, "-", TSP_NUM).is_none() { return mk_error(); }
        (clone_val_pub(a_orig), clone_val_pub(b))
    };
    if a.t == TspType::TspDec || b.t == TspType::TspDec {
        return mk_dec(num_pub(&a)/den_pub(&a) - num_pub(&b)/den_pub(&b)).unwrap();
    }
    (mk_num(a.t, b.t, 0))(num_pub(&a)*den_pub(&b) - den_pub(&a)*num_pub(&b), den_pub(&a)*den_pub(&b))
}
pub fn prim_mul(_st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    if tsp_arg_num_check_pub(&args, "*", 2).is_none() { return mk_error(); }
    let a = car_pub(&args); let b = car_pub(cdr_pub(&args));
    if tsp_arg_type_check_pub(a, "*", TSP_NUM).is_none() { return mk_error(); }
    if tsp_arg_type_check_pub(b, "*", TSP_NUM).is_none() { return mk_error(); }
    if a.t == TspType::TspDec || b.t == TspType::TspDec {
        return mk_dec(num_pub(a)/den_pub(a) * (num_pub(b)/den_pub(b))).unwrap();
    }
    (mk_num(a.t, b.t, 0))(num_pub(a)*num_pub(b), den_pub(a)*den_pub(b))
}
pub fn prim_div(_st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    let len = tsp_lstlen(&args);
    if len != 2 && len != 1 {
        eprintln!("; tisp: error: /: expected 1 or 2 arguments, recieved {}", len);
        return mk_error();
    }
    let a_orig = car_pub(&args);
    if tsp_arg_type_check_pub(a_orig, "/", TSP_NUM).is_none() { return mk_error(); }
    let (a, b) = if len == 1 {
        (mk_int(1), clone_val_pub(a_orig))
    } else {
        let b = car_pub(cdr_pub(&args));
        if tsp_arg_type_check_pub(b, "/", TSP_NUM).is_none() { return mk_error(); }
        (clone_val_pub(a_orig), clone_val_pub(b))
    };
    if a.t == TspType::TspDec || b.t == TspType::TspDec {
        return mk_dec((num_pub(&a)/den_pub(&a)) / (num_pub(&b)/den_pub(&b))).unwrap();
    }
    (mk_num(a.t, b.t, 1))(num_pub(&a)*den_pub(&b), den_pub(&a)*num_pub(&b))
}
pub fn prim_mod(_st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    if tsp_arg_num_check_pub(&args, "mod", 2).is_none() { return mk_error(); }
    let a = car_pub(&args); let b = car_pub(cdr_pub(&args));
    if tsp_arg_type_check_pub(a, "mod", TspType::TspInt as u32).is_none() { return mk_error(); }
    if tsp_arg_type_check_pub(b, "mod", TspType::TspInt as u32).is_none() { return mk_error(); }
    if num_pub(b) == 0.0 { eprintln!("; tisp: error: division by zero"); return mk_error(); }
    mk_int((num_pub(a) as i32) % (num_pub(b) as i32).abs())
}
pub fn prim_pow(st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    if tsp_arg_num_check_pub(&args, "pow", 2).is_none() { return mk_error(); }
    let b = car_pub(&args); let p = car_pub(cdr_pub(&args));
    if tsp_arg_type_check_pub(b, "pow", TSP_EXPR).is_none() { return mk_error(); }
    if tsp_arg_type_check_pub(p, "pow", TSP_EXPR).is_none() { return mk_error(); }
    let bnum = (num_pub(b) as f64).powf(num_pub(p)/den_pub(p));
    let bden = (den_pub(b) as f64).powf(num_pub(p)/den_pub(p));
    if (bnum == (bnum as i32) as f64 && bden == (bden as i32) as f64)
        || b.t == TspType::TspDec || p.t == TspType::TspDec {
        return (mk_num(b.t, p.t, 0))(bnum, bden);
    }
    let sym = mk_sym(st, "^");
    let bc = clone_val_pub(b); let pc = clone_val_pub(p);
    mk_list(st, 3, vec![sym, bc, pc])
}

macro_rules! prim_compare {
    ($name:ident, $op:tt, $opname:expr) => {
        fn $name(st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
            if tsp_lstlen(&args) != 2 { return clone_val_pub(&st.t); }
            if tsp_arg_type_check_pub(car_pub(&args), $opname, TSP_NUM).is_none() { return mk_error(); }
            if tsp_arg_type_check_pub(car_pub(cdr_pub(&args)), $opname, TSP_NUM).is_none() { return mk_error(); }
            let a = car_pub(&args); let b = car_pub(cdr_pub(&args));
            if (num_pub(a)*den_pub(b)) $op (num_pub(b)*den_pub(a)) { clone_val_pub(&st.t) } else { mk_nil_pub() }
        }
    };
}
prim_compare!(prim_lt, <, "<");
prim_compare!(prim_gt, >, ">");
prim_compare!(prim_lte, <=, "<=");
prim_compare!(prim_gte, >=, ">=");

macro_rules! prim_trig {
    ($name:ident, $func:ident, $sname:expr) => {
        fn $name(st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
            if tsp_arg_num_check_pub(&args, $sname, 1).is_none() { return mk_error(); }
            if tsp_arg_type_check_pub(car_pub(&args), $sname, TSP_EXPR).is_none() { return mk_error(); }
            let a = car_pub(&args);
            if a.t == TspType::TspDec {
                return mk_dec(num_pub(a).$func()).unwrap();
            }
            let sym = mk_sym(st, $sname);
            let ac = clone_val_pub(a);
            mk_list(st, 2, vec![sym, ac])
        }
    };
}
prim_trig!(prim_sin, sin, "sin");
prim_trig!(prim_cos, cos, "cos");
prim_trig!(prim_tan, tan, "tan");
prim_trig!(prim_sinh, sinh, "sinh");
prim_trig!(prim_cosh, cosh, "cosh");
prim_trig!(prim_tanh, tanh, "tanh");
prim_trig!(prim_asin, asin, "asin");
prim_trig!(prim_acos, acos, "acos");
prim_trig!(prim_atan, atan, "atan");
prim_trig!(prim_asinh, asinh, "asinh");
prim_trig!(prim_acosh, acosh, "acosh");
prim_trig!(prim_atanh, atanh, "atanh");
prim_trig!(prim_exp, exp, "exp");
prim_trig!(prim_log, ln, "log");

fn prim_numerator(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if tsp_arg_num_check_pub(&args, "numerator", 1).is_none() { return mk_error(); }
    if tsp_arg_type_check_pub(car_pub(&args), "numerator", TSP_RATIONAL).is_none() { return mk_error(); }
    mk_int(num_pub(car_pub(&args)) as i32)
}
pub fn prim_denominator(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if tsp_arg_num_check_pub(&args, "denominator", 1).is_none() { return mk_error(); }
    if tsp_arg_type_check_pub(car_pub(&args), "denominator", TSP_RATIONAL).is_none() { return mk_error(); }
    mk_int(den_pub(car_pub(&args)) as i32)
}

pub fn tib_env_math(st: &mut Tsp) {
    tisp_env_add(st, "Int", mk_prim(TspType::TspPrim, prim_Int, "Int"));
    tisp_env_add(st, "Dec", mk_prim(TspType::TspPrim, prim_Dec, "Dec"));
    tisp_env_add(st, "floor", mk_prim(TspType::TspPrim, prim_floor, "floor"));
    tisp_env_add(st, "ceil", mk_prim(TspType::TspPrim, prim_ceil, "ceil"));
    tisp_env_add(st, "round", mk_prim(TspType::TspPrim, prim_round, "round"));
    tisp_env_add(st, "numerator", mk_prim(TspType::TspPrim, prim_numerator, "numerator"));
    tisp_env_add(st, "denominator", mk_prim(TspType::TspPrim, prim_denominator, "denominator"));
    tisp_env_add(st, "+", mk_prim(TspType::TspPrim, prim_add, "+"));
    tisp_env_add(st, "-", mk_prim(TspType::TspPrim, prim_sub, "-"));
    tisp_env_add(st, "*", mk_prim(TspType::TspPrim, prim_mul, "*"));
    tisp_env_add(st, "/", mk_prim(TspType::TspPrim, prim_div, "/"));
    tisp_env_add(st, "mod", mk_prim(TspType::TspPrim, prim_mod, "mod"));
    tisp_env_add(st, "^", mk_prim(TspType::TspPrim, prim_pow, "^"));
    tisp_env_add(st, "<", mk_prim(TspType::TspPrim, prim_lt, "<"));
    tisp_env_add(st, ">", mk_prim(TspType::TspPrim, prim_gt, ">"));
    tisp_env_add(st, "<=", mk_prim(TspType::TspPrim, prim_lte, "<="));
    tisp_env_add(st, ">=", mk_prim(TspType::TspPrim, prim_gte, ">="));
    tisp_env_add(st, "sin", mk_prim(TspType::TspPrim, prim_sin, "sin"));
    tisp_env_add(st, "cos", mk_prim(TspType::TspPrim, prim_cos, "cos"));
    tisp_env_add(st, "tan", mk_prim(TspType::TspPrim, prim_tan, "tan"));
    tisp_env_add(st, "sinh", mk_prim(TspType::TspPrim, prim_sinh, "sinh"));
    tisp_env_add(st, "cosh", mk_prim(TspType::TspPrim, prim_cosh, "cosh"));
    tisp_env_add(st, "tanh", mk_prim(TspType::TspPrim, prim_tanh, "tanh"));
    tisp_env_add(st, "arcsin", mk_prim(TspType::TspPrim, prim_asin, "arcsin"));
    tisp_env_add(st, "arccos", mk_prim(TspType::TspPrim, prim_acos, "arccos"));
    tisp_env_add(st, "arctan", mk_prim(TspType::TspPrim, prim_atan, "arctan"));
    tisp_env_add(st, "arcsinh", mk_prim(TspType::TspPrim, prim_asinh, "arcsinh"));
    tisp_env_add(st, "arccosh", mk_prim(TspType::TspPrim, prim_acosh, "arccosh"));
    tisp_env_add(st, "arctanh", mk_prim(TspType::TspPrim, prim_atanh, "arctanh"));
    tisp_env_add(st, "exp", mk_prim(TspType::TspPrim, prim_exp, "exp"));
    tisp_env_add(st, "log", mk_prim(TspType::TspPrim, prim_log, "log"));
}
