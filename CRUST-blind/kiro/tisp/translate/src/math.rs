use crate::tisp::*;

pub fn create_int(num: f64, den: f64) -> Val {
    mk_int(num as i32)
}

pub fn create_dec(num: f64, den: f64) -> Val {
    mk_dec(num).unwrap()
}

pub fn create_rat(num: f64, den: f64) -> Val {
    mk_rat(num as i32, den as i32).unwrap_or_else(|| mk_val(TspType::TspNone))
}

pub fn mk_num(a: TspType, b: TspType, force: i32) -> fn(f64, f64) -> Val {
    if force == 1 { return create_rat; }
    if force == 2 { return create_dec; }
    if type_matches(a, TspType::TspDec as u32) || type_matches(b, TspType::TspDec as u32) {
        return create_dec;
    }
    if type_matches(a, TspType::TspRatio as u32) || type_matches(b, TspType::TspRatio as u32) {
        return create_rat;
    }
    create_int
}

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
    if t == TspType::TspInt as u32 { return "Int"; }
    if t == TSP_EXPR { return "Expr"; }
    if t == TSP_RATIONAL { return "Rational"; }
    if t & TSP_NUM != 0 { return "Num"; }
    if t == (TspType::TspInt as u32 | TspType::TspRatio as u32) { return "Rational"; }
    "Invalid"
}

macro_rules! prim_round {
    ($name:ident, $fname:expr, $force:expr, $op:expr) => {
        fn $name(st: &mut Tsp, vars: &mut Rec, args: Val) -> Val {
            tsp_arg_check(&args, $fname, 1);
            let n = car_ref(&args);
            tsp_type_check(n, $fname, TSP_NUM);
            let val = $op(num_of(n) / den_of(n));
            mk_num(n.t, n.t, $force)(val, 1.0)
        }
    };
}

prim_round!(prim_Int, "Int", 1, |x: f64| x);
prim_round!(prim_Dec, "Dec", 2, |x: f64| x);
prim_round!(prim_round_inner, "round", 0, |x: f64| x.round());
prim_round!(prim_floor_inner, "floor", 0, |x: f64| x.floor());
prim_round!(prim_ceil_inner, "ceil", 0, |x: f64| x.ceil());

pub fn prim_add(st: &mut Tsp, vars: &mut Rec, args: Val) -> Val {
    tsp_arg_check(&args, "+", 2);
    let a = car_ref(&args);
    let b = car_ref(cdr_ref(&args));
    tsp_type_check(a, "+", TSP_NUM);
    tsp_type_check(b, "+", TSP_NUM);
    if type_matches(a.t, TspType::TspDec as u32) || type_matches(b.t, TspType::TspDec as u32) {
        return mk_dec(num_of(a)/den_of(a) + num_of(b)/den_of(b)).unwrap();
    }
    mk_num(a.t, b.t, 0)(
        num_of(a) * den_of(b) + den_of(a) * num_of(b),
        den_of(a) * den_of(b),
    )
}

pub fn prim_sub(st: &mut Tsp, vars: &mut Rec, args: Val) -> Val {
    let len = tsp_lstlen(&args);
    if len != 2 && len != 1 {
        eprintln!("; tisp: error: -: expected 1 or 2 arguments, recieved {}", len);
        return mk_val(TspType::TspNone);
    }
    let a_orig = car_ref(&args);
    tsp_type_check(a_orig, "-", TSP_NUM);
    let (a, b) = if len == 1 {
        (&mk_int(0), a_orig)
    } else {
        let b = car_ref(cdr_ref(&args));
        tsp_type_check(b, "-", TSP_NUM);
        (a_orig, b)
    };
    // Need to handle borrowing - clone values
    let (an, ad, bn, bd, at, bt) = (num_of(a), den_of(a), num_of(b), den_of(b), a.t, b.t);
    if type_matches(at, TspType::TspDec as u32) || type_matches(bt, TspType::TspDec as u32) {
        return mk_dec(an/ad - bn/bd).unwrap();
    }
    mk_num(at, bt, 0)(an * bd - ad * bn, ad * bd)
}

pub fn prim_mul(st: &mut Tsp, vars: &mut Rec, args: Val) -> Val {
    tsp_arg_check(&args, "*", 2);
    let a = car_ref(&args);
    let b = car_ref(cdr_ref(&args));
    tsp_type_check(a, "*", TSP_NUM);
    tsp_type_check(b, "*", TSP_NUM);
    if type_matches(a.t, TspType::TspDec as u32) || type_matches(b.t, TspType::TspDec as u32) {
        return mk_dec((num_of(a)/den_of(a)) * (num_of(b)/den_of(b))).unwrap();
    }
    mk_num(a.t, b.t, 0)(num_of(a) * num_of(b), den_of(a) * den_of(b))
}

pub fn prim_div(st: &mut Tsp, vars: &mut Rec, args: Val) -> Val {
    let len = tsp_lstlen(&args);
    if len != 2 && len != 1 {
        eprintln!("; tisp: error: /: expected 1 or 2 arguments, recieved {}", len);
        return mk_val(TspType::TspNone);
    }
    let a_orig = car_ref(&args);
    tsp_type_check(a_orig, "/", TSP_NUM);
    let (an, ad, bn, bd, at, bt) = if len == 1 {
        (1.0, 1.0, num_of(a_orig), den_of(a_orig), TspType::TspInt, a_orig.t)
    } else {
        let b = car_ref(cdr_ref(&args));
        tsp_type_check(b, "/", TSP_NUM);
        (num_of(a_orig), den_of(a_orig), num_of(b), den_of(b), a_orig.t, b.t)
    };
    if type_matches(at, TspType::TspDec as u32) || type_matches(bt, TspType::TspDec as u32) {
        return mk_dec((an/ad) / (bn/bd)).unwrap();
    }
    mk_num(at, bt, 1)(an * bd, ad * bn)
}

pub fn prim_mod(st: &mut Tsp, vars: &mut Rec, args: Val) -> Val {
    tsp_arg_check(&args, "mod", 2);
    let a = car_ref(&args);
    let b = car_ref(cdr_ref(&args));
    tsp_type_check(a, "mod", TspType::TspInt as u32);
    tsp_type_check(b, "mod", TspType::TspInt as u32);
    let bv = num_of(b) as i32;
    if bv == 0 {
        eprintln!("; tisp: error: division by zero");
        return mk_val(TspType::TspNone);
    }
    mk_int((num_of(a) as i32) % bv.abs())
}

pub fn prim_pow(st: &mut Tsp, vars: &mut Rec, args: Val) -> Val {
    tsp_arg_check(&args, "pow", 2);
    let b = car_ref(&args);
    let p = car_ref(cdr_ref(&args));
    tsp_type_check(b, "pow", TSP_EXPR);
    tsp_type_check(p, "pow", TSP_EXPR);
    let bnum = (num_of(b) as f64).powf(num_of(p) / den_of(p));
    let bden = (den_of(b) as f64).powf(num_of(p) / den_of(p));
    if (bnum == (bnum as i32) as f64 && bden == (bden as i32) as f64) ||
       type_matches(b.t, TspType::TspDec as u32) || type_matches(p.t, TspType::TspDec as u32) {
        return mk_num(b.t, p.t, 0)(bnum, bden);
    }
    // Return symbolic expression (^ b p)
    let sym = mk_sym(st, "^").unwrap();
    let bv = val_clone(b);
    let pv = val_clone(p);
    mk_list(st, 3, vec![sym, bv, pv]).unwrap()
}

macro_rules! prim_compare {
    ($name:ident, $op_name:expr, $op:tt) => {
        fn $name(st: &mut Tsp, vars: &mut Rec, args: Val) -> Val {
            if tsp_lstlen(&args) != 2 { return val_clone(&st.t); }
            tsp_type_check(car_ref(&args), $op_name, TSP_NUM);
            tsp_type_check(car_ref(cdr_ref(&args)), $op_name, TSP_NUM);
            let a = car_ref(&args);
            let b = car_ref(cdr_ref(&args));
            if (num_of(a) * den_of(b)) $op (num_of(b) * den_of(a)) {
                val_clone(&st.t)
            } else {
                val_clone(&st.nil)
            }
        }
    };
}

prim_compare!(prim_lt, "<", <);
prim_compare!(prim_gt, ">", >);
prim_compare!(prim_lte, "<=", <=);
prim_compare!(prim_gte, ">=", >=);

macro_rules! prim_trig {
    ($name:ident, $fname:expr, $op:ident) => {
        fn $name(st: &mut Tsp, vars: &mut Rec, args: Val) -> Val {
            tsp_arg_check(&args, $fname, 1);
            tsp_type_check(car_ref(&args), $fname, TSP_EXPR);
            if type_matches(car_ref(&args).t, TspType::TspDec as u32) {
                return mk_dec(num_of(car_ref(&args)).$op()).unwrap();
            }
            let sym = mk_sym(st, $fname).unwrap();
            let arg = val_clone(car_ref(&args));
            mk_list(st, 2, vec![sym, arg]).unwrap()
        }
    };
}

prim_trig!(prim_sin, "sin", sin);
prim_trig!(prim_cos, "cos", cos);
prim_trig!(prim_tan, "tan", tan);
prim_trig!(prim_sinh, "sinh", sinh);
prim_trig!(prim_cosh, "cosh", cosh);
prim_trig!(prim_tanh, "tanh", tanh);
prim_trig!(prim_asin, "arcsin", asin);
prim_trig!(prim_acos, "arccos", acos);
prim_trig!(prim_atan, "arctan", atan);
prim_trig!(prim_asinh, "arcsinh", asinh);
prim_trig!(prim_acosh, "arccosh", acosh);
prim_trig!(prim_atanh, "arctanh", atanh);
prim_trig!(prim_exp, "exp", exp);
prim_trig!(prim_log, "log", ln);

fn prim_numerator(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    tsp_arg_check(&args, "numerator", 1);
    tsp_type_check(car_ref(&args), "numerator", TspType::TspInt as u32 | TspType::TspRatio as u32);
    mk_int(num_of(car_ref(&args)) as i32)
}

pub fn prim_denominator(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    tsp_arg_check(&args, "denominator", 1);
    tsp_type_check(car_ref(&args), "denominator", TspType::TspInt as u32 | TspType::TspRatio as u32);
    mk_int(den_of(car_ref(&args)) as i32)
}

pub fn tib_env_math(st: &mut Tsp) {
    tisp_env_add(st, "Int", mk_prim(TspType::TspPrim, prim_Int, "Int").unwrap());
    tisp_env_add(st, "Dec", mk_prim(TspType::TspPrim, prim_Dec, "Dec").unwrap());
    tisp_env_add(st, "floor", mk_prim(TspType::TspPrim, prim_floor_inner, "floor").unwrap());
    tisp_env_add(st, "ceil", mk_prim(TspType::TspPrim, prim_ceil_inner, "ceil").unwrap());
    tisp_env_add(st, "round", mk_prim(TspType::TspPrim, prim_round_inner, "round").unwrap());
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
