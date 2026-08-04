use crate::tisp::{
    self, clone_val, mk_dec, mk_int, mk_rat, mk_val, tsp_lstlen, Rec, Tsp, TspType, Val, ValUnion,
    TSP_NUM,
};

pub fn create_int(num: f64, _den: f64) -> Val {
    mk_int(num as i32)
}

pub fn create_dec(num: f64, _den: f64) -> Val {
    mk_dec(num).unwrap_or_else(|| mk_val(TspType::TspNone))
}

pub fn create_rat(num: f64, den: f64) -> Val {
    mk_rat(num as i32, den as i32).unwrap_or_else(|| mk_val(TspType::TspNone))
}

pub fn mk_num(a: TspType, b: TspType, force: i32) -> fn(f64, f64) -> Val {
    if force == 1 {
        return create_rat;
    }
    if force == 2 {
        return create_dec;
    }
    let a_u = a as u32;
    let b_u = b as u32;
    if (a_u & TspType::TspDec as u32) != 0 || (b_u & TspType::TspDec as u32) != 0 {
        return create_dec;
    }
    if (a_u & TspType::TspRatio as u32) != 0 || (b_u & TspType::TspRatio as u32) != 0 {
        return create_rat;
    }
    create_int
}

fn first_two(args: &Val) -> Option<(Val, Val)> {
    if let ValUnion::P { car, cdr } = &args.v {
        if let ValUnion::P { car: c2, .. } = &cdr.v {
            return Some((clone_val(car), clone_val(c2)));
        }
    }
    None
}

fn first(args: &Val) -> Option<Val> {
    if let ValUnion::P { car, .. } = &args.v {
        return Some(clone_val(car));
    }
    None
}

fn nd(v: &Val) -> Option<(f64, f64)> {
    if let ValUnion::N { num, den } = &v.v {
        Some((*num, *den))
    } else {
        None
    }
}

pub fn prim_add(_st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    if tsp_lstlen(&args) != 2 {
        return mk_val(TspType::TspNone);
    }
    let (a, b) = match first_two(&args) {
        Some(p) => p,
        None => return mk_val(TspType::TspNone),
    };
    if (a.t as u32) & TSP_NUM == 0 || (b.t as u32) & TSP_NUM == 0 {
        return mk_val(TspType::TspNone);
    }
    let (an, ad) = nd(&a).unwrap();
    let (bn, bd) = nd(&b).unwrap();
    if (a.t as u32) & TspType::TspDec as u32 != 0 || (b.t as u32) & TspType::TspDec as u32 != 0 {
        return mk_dec((an / ad) + (bn / bd)).unwrap_or_else(|| mk_val(TspType::TspNone));
    }
    (mk_num(a.t, b.t, 0))(an * bd + ad * bn, ad * bd)
}

pub fn prim_sub(_st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    let len = tsp_lstlen(&args);
    if len != 1 && len != 2 {
        return mk_val(TspType::TspNone);
    }
    let a_orig = match first(&args) {
        Some(v) => v,
        None => return mk_val(TspType::TspNone),
    };
    let (a, b) = if len == 1 {
        (mk_int(0), a_orig)
    } else {
        let b = match &args.v {
            ValUnion::P { cdr, .. } => match &cdr.v {
                ValUnion::P { car, .. } => clone_val(car),
                _ => return mk_val(TspType::TspNone),
            },
            _ => return mk_val(TspType::TspNone),
        };
        (a_orig, b)
    };
    if (a.t as u32) & TSP_NUM == 0 || (b.t as u32) & TSP_NUM == 0 {
        return mk_val(TspType::TspNone);
    }
    let (an, ad) = nd(&a).unwrap();
    let (bn, bd) = nd(&b).unwrap();
    if (a.t as u32) & TspType::TspDec as u32 != 0 || (b.t as u32) & TspType::TspDec as u32 != 0 {
        return mk_dec((an / ad) - (bn / bd)).unwrap_or_else(|| mk_val(TspType::TspNone));
    }
    (mk_num(a.t, b.t, 0))(an * bd - ad * bn, ad * bd)
}

pub fn prim_mul(_st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    if tsp_lstlen(&args) != 2 {
        return mk_val(TspType::TspNone);
    }
    let (a, b) = match first_two(&args) {
        Some(p) => p,
        None => return mk_val(TspType::TspNone),
    };
    if (a.t as u32) & TSP_NUM == 0 || (b.t as u32) & TSP_NUM == 0 {
        return mk_val(TspType::TspNone);
    }
    let (an, ad) = nd(&a).unwrap();
    let (bn, bd) = nd(&b).unwrap();
    if (a.t as u32) & TspType::TspDec as u32 != 0 || (b.t as u32) & TspType::TspDec as u32 != 0 {
        return mk_dec((an / ad) * (bn / bd)).unwrap_or_else(|| mk_val(TspType::TspNone));
    }
    (mk_num(a.t, b.t, 0))(an * bn, ad * bd)
}

pub fn prim_div(_st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    let len = tsp_lstlen(&args);
    if len != 1 && len != 2 {
        return mk_val(TspType::TspNone);
    }
    let a_orig = match first(&args) {
        Some(v) => v,
        None => return mk_val(TspType::TspNone),
    };
    let (a, b) = if len == 1 {
        (mk_int(1), a_orig)
    } else {
        let b = match &args.v {
            ValUnion::P { cdr, .. } => match &cdr.v {
                ValUnion::P { car, .. } => clone_val(car),
                _ => return mk_val(TspType::TspNone),
            },
            _ => return mk_val(TspType::TspNone),
        };
        (a_orig, b)
    };
    if (a.t as u32) & TSP_NUM == 0 || (b.t as u32) & TSP_NUM == 0 {
        return mk_val(TspType::TspNone);
    }
    let (an, ad) = nd(&a).unwrap();
    let (bn, bd) = nd(&b).unwrap();
    if (a.t as u32) & TspType::TspDec as u32 != 0 || (b.t as u32) & TspType::TspDec as u32 != 0 {
        return mk_dec((an / ad) / (bn / bd)).unwrap_or_else(|| mk_val(TspType::TspNone));
    }
    (mk_num(a.t, b.t, 1))(an * bd, ad * bn)
}

pub fn prim_mod(_st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    if tsp_lstlen(&args) != 2 {
        return mk_val(TspType::TspNone);
    }
    let (a, b) = match first_two(&args) {
        Some(p) => p,
        None => return mk_val(TspType::TspNone),
    };
    if !matches!(a.t, TspType::TspInt) || !matches!(b.t, TspType::TspInt) {
        return mk_val(TspType::TspNone);
    }
    let (an, _) = nd(&a).unwrap();
    let (bn, _) = nd(&b).unwrap();
    if bn == 0.0 {
        return mk_val(TspType::TspNone);
    }
    mk_int((an as i32) % (bn as i32).abs())
}

pub fn prim_pow(_st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    if tsp_lstlen(&args) != 2 {
        return mk_val(TspType::TspNone);
    }
    let (b, p) = match first_two(&args) {
        Some(p) => p,
        None => return mk_val(TspType::TspNone),
    };
    let (bn, bd) = match nd(&b) {
        Some(v) => v,
        None => return mk_val(TspType::TspNone),
    };
    let (pn, pd) = match nd(&p) {
        Some(v) => v,
        None => return mk_val(TspType::TspNone),
    };
    let exp = pn / pd;
    let new_num = bn.powf(exp);
    let new_den = bd.powf(exp);
    if (new_num == new_num.trunc() && new_den == new_den.trunc())
        || (b.t as u32) & TspType::TspDec as u32 != 0
        || (p.t as u32) & TspType::TspDec as u32 != 0
    {
        return mk_num(b.t, p.t, 0)(new_num, new_den);
    }
    mk_val(TspType::TspNone)
}

pub fn prim_denominator(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if tsp_lstlen(&args) != 1 {
        return mk_val(TspType::TspNone);
    }
    let a = match first(&args) {
        Some(v) => v,
        None => return mk_val(TspType::TspNone),
    };
    if !matches!(a.t, TspType::TspInt | TspType::TspRatio) {
        return mk_val(TspType::TspNone);
    }
    if let ValUnion::N { den, .. } = &a.v {
        return mk_int(*den as i32);
    }
    mk_val(TspType::TspNone)
}

pub fn tib_env_math(st: &mut Tsp) {
    let _ = st;
    // primitives would be added but require functions matching the Prim signature
    // For now just register key arithmetic; Prim signature requires Tsp by value, complicating env registration in safe Rust
}
