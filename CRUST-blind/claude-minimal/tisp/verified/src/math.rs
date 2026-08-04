use crate::tisp::{
    mk_dec, mk_int, mk_rat, mk_val, Rec, Tsp, TspType, Val, ValUnion,
};

/* helper: get number values */
fn num_of(v: &Val) -> (f64, f64) {
    if let ValUnion::N { num, den } = &v.v {
        (*num, *den)
    } else {
        (0.0, 1.0)
    }
}

fn type_in(t: TspType, mask: u32) -> bool {
    (t as u32) & mask != 0
}

fn car(v: &Val) -> Option<&Val> {
    if let ValUnion::P { car, .. } = &v.v {
        Some(car)
    } else {
        None
    }
}

fn cadr(v: &Val) -> Option<&Val> {
    if let ValUnion::P { cdr, .. } = &v.v {
        if let ValUnion::P { car, .. } = &cdr.v {
            return Some(car);
        }
    }
    None
}

/* wrapper functions returned by mk_num */
pub fn create_int(num: f64, _den: f64) -> Val {
    mk_int(num as i32)
}

pub fn create_dec(num: f64, _den: f64) -> Val {
    mk_dec(num).unwrap_or_else(|| mk_val(TspType::TspDec))
}

pub fn create_rat(num: f64, den: f64) -> Val {
    mk_rat(num as i32, den as i32).unwrap_or_else(|| mk_val(TspType::TspRatio))
}

/* return appropriate constructor depending on operand types and force flag */
pub fn mk_num(a: TspType, b: TspType, force: i32) -> fn(f64, f64) -> Val {
    if force == 1 {
        return create_rat;
    }
    if force == 2 {
        return create_dec;
    }
    let dec_mask = TspType::TspDec as u32;
    let rat_mask = TspType::TspRatio as u32;
    if type_in(a, dec_mask) || type_in(b, dec_mask) {
        return create_dec;
    }
    if type_in(a, rat_mask) || type_in(b, rat_mask) {
        return create_rat;
    }
    create_int
}

pub fn prim_add(_st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    let a = match car(&args) {
        Some(v) => v,
        None => return mk_val(TspType::TspNone),
    };
    let b = match cadr(&args) {
        Some(v) => v,
        None => return mk_val(TspType::TspNone),
    };
    let (an, ad) = num_of(a);
    let (bn, bd) = num_of(b);
    let dec_mask = TspType::TspDec as u32;
    if type_in(a.t, dec_mask) || type_in(b.t, dec_mask) {
        return mk_dec(an / ad + bn / bd).unwrap_or_else(|| mk_val(TspType::TspDec));
    }
    let f = mk_num(a.t, b.t, 0);
    f(an * bd + ad * bn, ad * bd)
}

pub fn prim_sub(_st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    let a_val = match car(&args) {
        Some(v) => v,
        None => return mk_val(TspType::TspNone),
    };
    let b_opt = cadr(&args);
    let (an, ad) = if b_opt.is_some() {
        num_of(a_val)
    } else {
        (0.0, 1.0)
    };
    let a_t = if b_opt.is_some() {
        a_val.t
    } else {
        TspType::TspInt
    };
    let (bn, bd, b_t) = match b_opt {
        Some(b) => {
            let (n, d) = num_of(b);
            (n, d, b.t)
        }
        None => {
            let (n, d) = num_of(a_val);
            (n, d, a_val.t)
        }
    };
    let dec_mask = TspType::TspDec as u32;
    if type_in(a_t, dec_mask) || type_in(b_t, dec_mask) {
        return mk_dec(an / ad - bn / bd).unwrap_or_else(|| mk_val(TspType::TspDec));
    }
    let f = mk_num(a_t, b_t, 0);
    f(an * bd - ad * bn, ad * bd)
}

pub fn prim_mul(_st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    let a = match car(&args) {
        Some(v) => v,
        None => return mk_val(TspType::TspNone),
    };
    let b = match cadr(&args) {
        Some(v) => v,
        None => return mk_val(TspType::TspNone),
    };
    let (an, ad) = num_of(a);
    let (bn, bd) = num_of(b);
    let dec_mask = TspType::TspDec as u32;
    if type_in(a.t, dec_mask) || type_in(b.t, dec_mask) {
        return mk_dec((an / ad) * (bn / bd)).unwrap_or_else(|| mk_val(TspType::TspDec));
    }
    let f = mk_num(a.t, b.t, 0);
    f(an * bn, ad * bd)
}

pub fn prim_div(_st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    let a_val = match car(&args) {
        Some(v) => v,
        None => return mk_val(TspType::TspNone),
    };
    let b_opt = cadr(&args);
    let (an, ad, a_t) = match b_opt {
        Some(_) => {
            let (n, d) = num_of(a_val);
            (n, d, a_val.t)
        }
        None => (1.0, 1.0, TspType::TspInt),
    };
    let (bn, bd, b_t) = match b_opt {
        Some(b) => {
            let (n, d) = num_of(b);
            (n, d, b.t)
        }
        None => {
            let (n, d) = num_of(a_val);
            (n, d, a_val.t)
        }
    };
    let dec_mask = TspType::TspDec as u32;
    if type_in(a_t, dec_mask) || type_in(b_t, dec_mask) {
        let denom = bn / bd;
        if denom == 0.0 {
            eprintln!("; tisp: error: division by zero");
            return mk_val(TspType::TspNone);
        }
        return mk_dec((an / ad) / denom).unwrap_or_else(|| mk_val(TspType::TspDec));
    }
    let f = mk_num(a_t, b_t, 1);
    f(an * bd, ad * bn)
}

pub fn prim_mod(_st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    let a = match car(&args) {
        Some(v) => v,
        None => return mk_val(TspType::TspNone),
    };
    let b = match cadr(&args) {
        Some(v) => v,
        None => return mk_val(TspType::TspNone),
    };
    let (an, _) = num_of(a);
    let (bn, _) = num_of(b);
    if bn == 0.0 {
        eprintln!("; tisp: error: division by zero");
        return mk_val(TspType::TspNone);
    }
    mk_int((an as i32) % (bn as i32).abs())
}

pub fn prim_pow(_st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    let b = match car(&args) {
        Some(v) => v,
        None => return mk_val(TspType::TspNone),
    };
    let p = match cadr(&args) {
        Some(v) => v,
        None => return mk_val(TspType::TspNone),
    };
    let (bn_num, bn_den) = num_of(b);
    let (pn_num, pn_den) = num_of(p);
    let exp = pn_num / pn_den;
    let bnum = bn_num.powf(exp);
    let bden = bn_den.powf(exp);
    let dec_mask = TspType::TspDec as u32;
    if (bnum == (bnum as i32) as f64 && bden == (bden as i32) as f64)
        || type_in(b.t, dec_mask)
        || type_in(p.t, dec_mask)
    {
        return mk_num(b.t, p.t, 0)(bnum, bden);
    }
    /* fallback to dec result */
    mk_dec(bnum / bden).unwrap_or_else(|| mk_val(TspType::TspDec))
}

pub fn prim_denominator(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if let Some(first) = car(&args) {
        let (_, d) = num_of(first);
        return mk_int(d as i32);
    }
    mk_val(TspType::TspNone)
}

pub fn tib_env_math(_st: &mut Tsp) {
    /* Register math primitives. Stub: actual binding happens in the C code
     * via macros; in Rust we leave as placeholder. */
}
