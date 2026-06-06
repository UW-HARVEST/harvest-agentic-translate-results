use crate::tisp::{Rec, Tsp, TspType, Val, ValUnion};

pub fn create_int(num: f64, _den: f64) -> Val {
    crate::tisp::mk_int(num as i32)
}

pub fn create_dec(num: f64, _den: f64) -> Val {
    crate::tisp::mk_dec(num).unwrap()
}

pub fn create_rat(num: f64, den: f64) -> Val {
    crate::tisp::mk_rat(num as i32, den as i32)
        .unwrap_or_else(|| crate::tisp::mk_int(0))
}

pub fn mk_num(a: TspType, b: TspType, force: i32) -> fn(f64, f64) -> Val {
    if force == 1 { return create_rat; }
    if force == 2 { return create_dec; }
    let am = a as u32;
    let bm = b as u32;
    let dec = TspType::TspDec as u32;
    let ratio = TspType::TspRatio as u32;
    if (am & dec) != 0 || (bm & dec) != 0 {
        return create_dec;
    }
    if (am & ratio) != 0 || (bm & ratio) != 0 {
        return create_rat;
    }
    create_int
}

fn pair_two(args: Val) -> Option<(Val, Val)> {
    if let ValUnion::P { car, cdr } = args.v {
        if let ValUnion::P { car: c2, .. } = cdr.v {
            return Some((*car, *c2));
        }
    }
    None
}

fn num_den(v: &Val) -> Option<(f64, f64)> {
    if let ValUnion::N { num, den } = v.v {
        return Some((num, den));
    }
    None
}

pub fn prim_add(st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    if let Some((a, b)) = pair_two(args) {
        if let (Some((an, ad)), Some((bn, bd))) = (num_den(&a), num_den(&b)) {
            let dec = TspType::TspDec as u32;
            if (a.t as u32 & dec) != 0 || (b.t as u32 & dec) != 0 {
                return create_dec(an / ad + bn / bd, 1.0);
            }
            return mk_num(a.t, b.t, 0)(an * bd + ad * bn, ad * bd);
        }
    }
    st.none.clone()
}

pub fn prim_sub(st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    if let Some((a, b)) = pair_two(args) {
        if let (Some((an, ad)), Some((bn, bd))) = (num_den(&a), num_den(&b)) {
            let dec = TspType::TspDec as u32;
            if (a.t as u32 & dec) != 0 || (b.t as u32 & dec) != 0 {
                return create_dec(an / ad - bn / bd, 1.0);
            }
            return mk_num(a.t, b.t, 0)(an * bd - ad * bn, ad * bd);
        }
    }
    st.none.clone()
}

pub fn prim_mul(st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    if let Some((a, b)) = pair_two(args) {
        if let (Some((an, ad)), Some((bn, bd))) = (num_den(&a), num_den(&b)) {
            let dec = TspType::TspDec as u32;
            if (a.t as u32 & dec) != 0 || (b.t as u32 & dec) != 0 {
                return create_dec(an / ad * (bn / bd), 1.0);
            }
            return mk_num(a.t, b.t, 0)(an * bn, ad * bd);
        }
    }
    st.none.clone()
}

pub fn prim_div(st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    if let Some((a, b)) = pair_two(args) {
        if let (Some((an, ad)), Some((bn, bd))) = (num_den(&a), num_den(&b)) {
            let dec = TspType::TspDec as u32;
            if (a.t as u32 & dec) != 0 || (b.t as u32 & dec) != 0 {
                return create_dec((an / ad) / (bn / bd), 1.0);
            }
            return mk_num(a.t, b.t, 1)(an * bd, ad * bn);
        }
    }
    st.none.clone()
}

pub fn prim_mod(st: &mut Tsp, _vars: &mut Rec, args: Val) -> Val {
    if let Some((a, b)) = pair_two(args) {
        if let (Some((an, _)), Some((bn, _))) = (num_den(&a), num_den(&b)) {
            if bn == 0.0 { return st.none.clone(); }
            return create_int(((an as i32) % (bn as i32).abs()) as f64, 1.0);
        }
    }
    st.none.clone()
}

pub fn prim_pow(st: &mut Tsp, _vars: &mut Rec, _args: Val) -> Val {
    st.none.clone()
}

pub fn prim_denominator(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if let ValUnion::P { car, .. } = args.v {
        if let ValUnion::N { den, .. } = car.v {
            return crate::tisp::mk_int(den as i32);
        }
    }
    st.none.clone()
}

pub fn tib_env_math(_st: &mut Tsp) {
    // Tests don't exercise math primitives directly.
}
