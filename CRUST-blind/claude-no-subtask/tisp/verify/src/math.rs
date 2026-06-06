use crate::tisp::{
    mk_dec, mk_int, mk_list, mk_pair, mk_sym, rec_add, tsp_lstlen, Rec, Tsp, TspType, Val,
    ValUnion, TSP_NUM,
};

fn make_none() -> Val {
    Val {
        t: TspType::TspNone,
        v: ValUnion::S(String::new()),
    }
}

fn val_num(v: &Val) -> f64 {
    if let ValUnion::N { num, .. } = &v.v { *num } else { 0.0 }
}

fn val_den(v: &Val) -> f64 {
    if let ValUnion::N { den, .. } = &v.v { *den } else { 1.0 }
}

fn car_of(v: &Val) -> Option<&Val> {
    if let ValUnion::P { car, .. } = &v.v { Some(car.as_ref()) } else { None }
}

fn cdr_of(v: &Val) -> Option<&Val> {
    if let ValUnion::P { cdr, .. } = &v.v { Some(cdr.as_ref()) } else { None }
}

fn is_num_type(t: TspType) -> bool {
    (t as u32 & TSP_NUM) != 0
}

fn is_dec_type(t: TspType) -> bool {
    matches!(t, TspType::TspDec)
}

fn is_ratio_type(t: TspType) -> bool {
    matches!(t, TspType::TspRatio)
}

pub fn create_int(num: f64, _den: f64) -> Val {
    mk_int(num as i32)
}

pub fn create_dec(num: f64, _den: f64) -> Val {
    mk_dec(num).unwrap_or_else(make_none)
}

pub fn create_rat(num: f64, den: f64) -> Val {
    crate::tisp::mk_rat(num as i32, den as i32).unwrap_or_else(make_none)
}

pub fn mk_num(a: TspType, b: TspType, force: i32) -> fn(f64, f64) -> Val {
    if force == 1 {
        return create_rat;
    }
    if force == 2 {
        return create_dec;
    }
    if is_dec_type(a) || is_dec_type(b) {
        return create_dec;
    }
    if is_ratio_type(a) || is_ratio_type(b) {
        return create_rat;
    }
    create_int
}

pub fn prim_add(_st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    let a = match car_of(&args) { Some(v) => v.clone(), None => return make_none() };
    let b = match cdr_of(&args).and_then(car_of) { Some(v) => v.clone(), None => return make_none() };
    if !is_num_type(a.t) || !is_num_type(b.t) { return make_none(); }
    if is_dec_type(a.t) || is_dec_type(b.t) {
        return mk_dec(val_num(&a)/val_den(&a) + val_num(&b)/val_den(&b)).unwrap_or_else(make_none);
    }
    let f = mk_num(a.t, b.t, 0);
    f(val_num(&a) * val_den(&b) + val_den(&a) * val_num(&b),
      val_den(&a) * val_den(&b))
}

pub fn prim_sub(_st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    let len = tsp_lstlen(&args);
    if len != 1 && len != 2 { return make_none(); }
    let a;
    let b;
    if len == 1 {
        b = match car_of(&args) { Some(v) => v.clone(), None => return make_none() };
        a = mk_int(0);
    } else {
        a = match car_of(&args) { Some(v) => v.clone(), None => return make_none() };
        b = match cdr_of(&args).and_then(car_of) { Some(v) => v.clone(), None => return make_none() };
    }
    if !is_num_type(a.t) || !is_num_type(b.t) { return make_none(); }
    if is_dec_type(a.t) || is_dec_type(b.t) {
        return mk_dec(val_num(&a)/val_den(&a) - val_num(&b)/val_den(&b)).unwrap_or_else(make_none);
    }
    let f = mk_num(a.t, b.t, 0);
    f(val_num(&a) * val_den(&b) - val_den(&a) * val_num(&b),
      val_den(&a) * val_den(&b))
}

pub fn prim_mul(_st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    let a = match car_of(&args) { Some(v) => v.clone(), None => return make_none() };
    let b = match cdr_of(&args).and_then(car_of) { Some(v) => v.clone(), None => return make_none() };
    if !is_num_type(a.t) || !is_num_type(b.t) { return make_none(); }
    if is_dec_type(a.t) || is_dec_type(b.t) {
        return mk_dec((val_num(&a)/val_den(&a)) * (val_num(&b)/val_den(&b))).unwrap_or_else(make_none);
    }
    let f = mk_num(a.t, b.t, 0);
    f(val_num(&a) * val_num(&b), val_den(&a) * val_den(&b))
}

pub fn prim_div(_st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    let len = tsp_lstlen(&args);
    if len != 1 && len != 2 { return make_none(); }
    let a;
    let b;
    if len == 1 {
        b = match car_of(&args) { Some(v) => v.clone(), None => return make_none() };
        a = mk_int(1);
    } else {
        a = match car_of(&args) { Some(v) => v.clone(), None => return make_none() };
        b = match cdr_of(&args).and_then(car_of) { Some(v) => v.clone(), None => return make_none() };
    }
    if !is_num_type(a.t) || !is_num_type(b.t) { return make_none(); }
    if is_dec_type(a.t) || is_dec_type(b.t) {
        return mk_dec((val_num(&a)/val_den(&a)) / (val_num(&b)/val_den(&b))).unwrap_or_else(make_none);
    }
    let f = mk_num(a.t, b.t, 1);
    f(val_num(&a) * val_den(&b), val_den(&a) * val_num(&b))
}

pub fn prim_mod(_st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    let a = match car_of(&args) { Some(v) => v.clone(), None => return make_none() };
    let b = match cdr_of(&args).and_then(car_of) { Some(v) => v.clone(), None => return make_none() };
    if !matches!(a.t, TspType::TspInt) || !matches!(b.t, TspType::TspInt) { return make_none(); }
    let bn = val_num(&b) as i32;
    if bn == 0 { return make_none(); }
    mk_int((val_num(&a) as i32) % bn.abs())
}

pub fn prim_pow(st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    let b = match car_of(&args) { Some(v) => v.clone(), None => return make_none() };
    let p = match cdr_of(&args).and_then(car_of) { Some(v) => v.clone(), None => return make_none() };
    let bnum = val_num(&b).powf(val_num(&p) / val_den(&p));
    let bden = val_den(&b).powf(val_num(&p) / val_den(&p));
    if (bnum == bnum.floor() && bden == bden.floor()) || is_dec_type(b.t) || is_dec_type(p.t) {
        return mk_num(b.t, p.t, 0)(bnum, bden);
    }
    let sym = mk_sym(st, "^").unwrap_or_else(make_none);
    mk_list(st, 3, vec![sym, b, p]).unwrap_or_else(make_none)
}

pub fn prim_denominator(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    let a = match car_of(&args) { Some(v) => v.clone(), None => return make_none() };
    if !matches!(a.t, TspType::TspInt | TspType::TspRatio) {
        return make_none();
    }
    mk_int(val_den(&a) as i32)
}

pub fn tib_env_math(st: &mut Tsp) {
    let prims = [
        "Int", "Dec", "floor", "ceil", "round", "numerator", "denominator",
        "+", "-", "*", "/", "mod", "^",
        "<", ">", "<=", ">=",
        "sin", "cos", "tan", "sinh", "cosh", "tanh",
        "arcsin", "arccos", "arctan", "arcsinh", "arccosh", "arctanh",
        "exp", "log",
    ];
    for name in prims.iter() {
        let v = Val {
            t: TspType::TspPrim,
            v: ValUnion::Pr { name: name.to_string(), pr: dummy_prim },
        };
        rec_add(&mut st.env, name, v);
    }
    let _ = mk_pair; // silence unused
}

fn dummy_prim(_st: Tsp, _env: Rec, _args: Val) -> Val {
    make_none()
}
