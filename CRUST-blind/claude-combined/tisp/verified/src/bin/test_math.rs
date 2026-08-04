use tisp_proj::math::{
    create_dec, create_int, create_rat, mk_num, prim_add, prim_denominator, prim_div, prim_mod,
    prim_mul, prim_sub,
};
use tisp_proj::tisp::{mk_int, mk_pair, mk_rat, mk_dec, mk_val, rec_new, tisp_env_init, TspType, Val, ValUnion};

fn make_args1(v: Val) -> Val {
    let nil = mk_val(TspType::TspNil);
    mk_pair(v, nil).unwrap()
}

fn make_args2(a: Val, b: Val) -> Val {
    let nil = mk_val(TspType::TspNil);
    let p2 = mk_pair(b, nil).unwrap();
    mk_pair(a, p2).unwrap()
}

#[test]
fn test_create_int() {
    let v = create_int(7.0, 1.0);
    assert!(matches!(v.t, TspType::TspInt));
    if let ValUnion::N { num, den } = v.v {
        assert_eq!(num, 7.0);
        assert_eq!(den, 1.0);
    }
}

#[test]
fn test_create_dec() {
    let v = create_dec(2.5, 1.0);
    assert!(matches!(v.t, TspType::TspDec));
    if let ValUnion::N { num, .. } = v.v {
        assert_eq!(num, 2.5);
    }
}

#[test]
fn test_create_rat() {
    // 4/8 should reduce to 1/2
    let v = create_rat(4.0, 8.0);
    assert!(matches!(v.t, TspType::TspRatio));
    if let ValUnion::N { num, den } = v.v {
        assert_eq!(num, 1.0);
        assert_eq!(den, 2.0);
    }
}

#[test]
fn test_create_rat_to_int() {
    // 6/3 should be 2 as Int
    let v = create_rat(6.0, 3.0);
    assert!(matches!(v.t, TspType::TspInt));
    if let ValUnion::N { num, .. } = v.v {
        assert_eq!(num, 2.0);
    }
}

#[test]
fn test_mk_num_force_rat() {
    let f = mk_num(TspType::TspInt, TspType::TspInt, 1);
    let v = f(2.0, 4.0);
    assert!(matches!(v.t, TspType::TspRatio));
}

#[test]
fn test_mk_num_force_dec() {
    let f = mk_num(TspType::TspInt, TspType::TspInt, 2);
    let v = f(3.0, 1.0);
    assert!(matches!(v.t, TspType::TspDec));
}

#[test]
fn test_mk_num_int_int() {
    let f = mk_num(TspType::TspInt, TspType::TspInt, 0);
    let v = f(5.0, 1.0);
    assert!(matches!(v.t, TspType::TspInt));
}

#[test]
fn test_mk_num_int_dec() {
    let f = mk_num(TspType::TspInt, TspType::TspDec, 0);
    let v = f(5.0, 1.0);
    assert!(matches!(v.t, TspType::TspDec));
}

#[test]
fn test_mk_num_int_rat() {
    let f = mk_num(TspType::TspInt, TspType::TspRatio, 0);
    let v = f(2.0, 4.0);
    assert!(matches!(v.t, TspType::TspRatio));
}

#[test]
fn test_prim_add_ints() {
    let mut st = tisp_env_init(16);
    let mut env = rec_new(8, None);
    // (+ 1 2) == 3
    let args = make_args2(mk_int(1), mk_int(2));
    let r = prim_add(&mut st, &mut env, args);
    assert!(matches!(r.t, TspType::TspInt));
    if let ValUnion::N { num, .. } = r.v {
        assert_eq!(num, 3.0);
    }
}

#[test]
fn test_prim_add_dec() {
    // (+ 1.5 2.5) == 4.0 as Dec
    let mut st = tisp_env_init(16);
    let mut env = rec_new(8, None);
    let args = make_args2(mk_dec(1.5).unwrap(), mk_dec(2.5).unwrap());
    let r = prim_add(&mut st, &mut env, args);
    assert!(matches!(r.t, TspType::TspDec));
    if let ValUnion::N { num, .. } = r.v {
        assert_eq!(num, 4.0);
    }
}

#[test]
fn test_prim_add_rats() {
    // (+ 1/2 1/3) = 5/6
    let mut st = tisp_env_init(16);
    let mut env = rec_new(8, None);
    let args = make_args2(mk_rat(1, 2).unwrap(), mk_rat(1, 3).unwrap());
    let r = prim_add(&mut st, &mut env, args);
    assert!(matches!(r.t, TspType::TspRatio));
    if let ValUnion::N { num, den } = r.v {
        assert_eq!(num, 5.0);
        assert_eq!(den, 6.0);
    }
}

#[test]
fn test_prim_sub() {
    let mut st = tisp_env_init(16);
    let mut env = rec_new(8, None);
    // (- 5 3) = 2
    let args = make_args2(mk_int(5), mk_int(3));
    let r = prim_sub(&mut st, &mut env, args);
    if let ValUnion::N { num, .. } = r.v {
        assert_eq!(num, 2.0);
    }
}

#[test]
fn test_prim_sub_unary() {
    let mut st = tisp_env_init(16);
    let mut env = rec_new(8, None);
    // (- 5) = -5
    let args = make_args1(mk_int(5));
    let r = prim_sub(&mut st, &mut env, args);
    assert!(matches!(r.t, TspType::TspInt));
    if let ValUnion::N { num, .. } = r.v {
        assert_eq!(num, -5.0);
    }
}

#[test]
fn test_prim_mul() {
    let mut st = tisp_env_init(16);
    let mut env = rec_new(8, None);
    // (* 4 5) = 20
    let args = make_args2(mk_int(4), mk_int(5));
    let r = prim_mul(&mut st, &mut env, args);
    if let ValUnion::N { num, .. } = r.v {
        assert_eq!(num, 20.0);
    }
}

#[test]
fn test_prim_mul_rat() {
    let mut st = tisp_env_init(16);
    let mut env = rec_new(8, None);
    // (* 1/2 1/3) = 1/6
    let args = make_args2(mk_rat(1, 2).unwrap(), mk_rat(1, 3).unwrap());
    let r = prim_mul(&mut st, &mut env, args);
    assert!(matches!(r.t, TspType::TspRatio));
    if let ValUnion::N { num, den } = r.v {
        assert_eq!(num, 1.0);
        assert_eq!(den, 6.0);
    }
}

#[test]
fn test_prim_div() {
    let mut st = tisp_env_init(16);
    let mut env = rec_new(8, None);
    // (/ 10 3) = 10/3 as Ratio
    let args = make_args2(mk_int(10), mk_int(3));
    let r = prim_div(&mut st, &mut env, args);
    assert!(matches!(r.t, TspType::TspRatio));
    if let ValUnion::N { num, den } = r.v {
        assert_eq!(num, 10.0);
        assert_eq!(den, 3.0);
    }
}

#[test]
fn test_prim_div_unary() {
    let mut st = tisp_env_init(16);
    let mut env = rec_new(8, None);
    // (/ 1/2) = 2
    let args = make_args1(mk_rat(1, 2).unwrap());
    let r = prim_div(&mut st, &mut env, args);
    assert!(matches!(r.t, TspType::TspInt));
    if let ValUnion::N { num, .. } = r.v {
        assert_eq!(num, 2.0);
    }
}

#[test]
fn test_prim_mod() {
    let mut st = tisp_env_init(16);
    let mut env = rec_new(8, None);
    // (mod 10 3) = 1
    let args = make_args2(mk_int(10), mk_int(3));
    let r = prim_mod(&mut st, &mut env, args);
    assert!(matches!(r.t, TspType::TspInt));
    if let ValUnion::N { num, .. } = r.v {
        assert_eq!(num, 1.0);
    }
}

#[test]
fn test_prim_denominator_ratio() {
    let mut st = tisp_env_init(16);
    let mut env = rec_new(8, None);
    // (denominator 1/2) = 2
    let args = make_args1(mk_rat(1, 2).unwrap());
    let r = prim_denominator(&mut st, &mut env, args);
    assert!(matches!(r.t, TspType::TspInt));
    if let ValUnion::N { num, .. } = r.v {
        assert_eq!(num, 2.0);
    }
}

#[test]
fn test_prim_denominator_int() {
    let mut st = tisp_env_init(16);
    let mut env = rec_new(8, None);
    // (denominator 5) = 1
    let args = make_args1(mk_int(5));
    let r = prim_denominator(&mut st, &mut env, args);
    if let ValUnion::N { num, .. } = r.v {
        assert_eq!(num, 1.0);
    }
}

fn main() {}
