#![allow(dead_code, unused_imports)]
use tisp_proj::math::*;
use tisp_proj::tisp::{
    mk_dec, mk_int, mk_pair, mk_rat, rec_new, tisp_env_init, Rec, Tsp, TspType, Val, ValUnion,
};

fn fresh_env() -> Rec {
    rec_new(64, None)
}

fn make_st() -> Tsp {
    let mut st = tisp_env_init(64);
    tisp_proj::tisp::tib_env_core(&mut st);
    tisp_proj::tisp::tib_env_math(&mut st);
    tisp_proj::tisp::tib_env_string(&mut st);
    st
}

fn nil() -> Val {
    Val { t: TspType::TspNil, v: ValUnion::N { num: 0.0, den: 1.0 } }
}

fn list2(a: Val, b: Val) -> Val {
    mk_pair(a, mk_pair(b, nil()).unwrap()).unwrap()
}

fn list1(a: Val) -> Val {
    mk_pair(a, nil()).unwrap()
}

fn assert_int(v: &Val, expected: i32) {
    assert!(matches!(v.t, TspType::TspInt), "expected Int, got {:?}", v.t);
    if let ValUnion::N { num, den } = &v.v {
        assert_eq!(*num, expected as f64);
        assert_eq!(*den, 1.0);
    } else {
        panic!("expected N union");
    }
}

fn assert_dec(v: &Val, expected: f64) {
    assert!(matches!(v.t, TspType::TspDec), "expected Dec, got {:?}", v.t);
    if let ValUnion::N { num, den } = &v.v {
        assert!((*num - expected).abs() < 1e-12, "got {}", num);
        assert_eq!(*den, 1.0);
    }
}

fn assert_dec_close(v: &Val, expected: f64, tol: f64) {
    assert!(matches!(v.t, TspType::TspDec));
    if let ValUnion::N { num, .. } = &v.v {
        assert!((*num - expected).abs() < tol, "got {} expected {}", num, expected);
    }
}

fn assert_rat(v: &Val, n: f64, d: f64) {
    assert!(matches!(v.t, TspType::TspRatio), "expected Ratio, got {:?}", v.t);
    if let ValUnion::N { num, den } = &v.v {
        assert_eq!(*num, n);
        assert_eq!(*den, d);
    }
}

#[test]
fn test_create_int_dec_rat() {
    let v = create_int(7.0, 1.0);
    assert_int(&v, 7);
    let v = create_dec(3.14, 1.0);
    assert_dec(&v, 3.14);
    let v = create_rat(4.0, 8.0);
    // 4/8 reduces to 1/2
    assert_rat(&v, 1.0, 2.0);
}

#[test]
fn test_mk_num_dispatch() {
    // Force=1 always rat
    let f = mk_num(TspType::TspInt, TspType::TspInt, 1);
    let v = f(4.0, 8.0);
    assert_rat(&v, 1.0, 2.0);

    // Force=2 always dec
    let f = mk_num(TspType::TspInt, TspType::TspInt, 2);
    let v = f(3.5, 1.0);
    assert_dec(&v, 3.5);

    // Force=0, Dec wins
    let f = mk_num(TspType::TspDec, TspType::TspInt, 0);
    let v = f(2.5, 1.0);
    assert_dec(&v, 2.5);

    // Force=0, Ratio wins over Int
    let f = mk_num(TspType::TspRatio, TspType::TspInt, 0);
    let v = f(4.0, 8.0);
    assert_rat(&v, 1.0, 2.0);

    // Force=0, both Int -> int
    let f = mk_num(TspType::TspInt, TspType::TspInt, 0);
    let v = f(5.0, 1.0);
    assert_int(&v, 5);
}

#[test]
fn test_prim_add() {
    let mut st = make_st();
    let mut env = fresh_env();
    // 1 + 1 = 2
    let r = prim_add(&mut st, &mut env, list2(mk_int(1), mk_int(1)));
    assert_int(&r, 2);
    // 1029 + 283 = 1312
    let r = prim_add(&mut st, &mut env, list2(mk_int(1029), mk_int(283)));
    assert_int(&r, 1312);
    // 204 + 8.3 = 212.3
    let r = prim_add(&mut st, &mut env, list2(mk_int(204), mk_dec(8.3).unwrap()));
    assert_dec(&r, 212.3);
    // 33 + 3/4 = 135/4
    let r = prim_add(&mut st, &mut env, list2(mk_int(33), mk_rat(3, 4).unwrap()));
    assert_rat(&r, 135.0, 4.0);
    // 1/3 + 5 = 16/3
    let r = prim_add(&mut st, &mut env, list2(mk_rat(1, 3).unwrap(), mk_int(5)));
    assert_rat(&r, 16.0, 3.0);
    // 2/5 + 3/2 = 19/10
    let r = prim_add(&mut st, &mut env, list2(mk_rat(2, 5).unwrap(), mk_rat(3, 2).unwrap()));
    assert_rat(&r, 19.0, 10.0);
    // 2.1 + 2 = 4.1
    let r = prim_add(&mut st, &mut env, list2(mk_dec(2.1).unwrap(), mk_int(2)));
    assert_dec(&r, 4.1);
}

#[test]
fn test_prim_sub() {
    let mut st = make_st();
    let mut env = fresh_env();

    // unary - 3 = -3
    let r = prim_sub(&mut st, &mut env, list1(mk_int(3)));
    assert_int(&r, -3);

    // unary - 7/8 = -7/8
    let r = prim_sub(&mut st, &mut env, list1(mk_rat(7, 8).unwrap()));
    assert_rat(&r, -7.0, 8.0);

    // 5 - 4 = 1
    let r = prim_sub(&mut st, &mut env, list2(mk_int(5), mk_int(4)));
    assert_int(&r, 1);

    // 53 - 88 = -35
    let r = prim_sub(&mut st, &mut env, list2(mk_int(53), mk_int(88)));
    assert_int(&r, -35);

    // 33 - 3/4 = 129/4
    let r = prim_sub(&mut st, &mut env, list2(mk_int(33), mk_rat(3, 4).unwrap()));
    assert_rat(&r, 129.0, 4.0);

    // 1/3 - 5 = -14/3
    let r = prim_sub(&mut st, &mut env, list2(mk_rat(1, 3).unwrap(), mk_int(5)));
    assert_rat(&r, -14.0, 3.0);

    // 2/5 - 3/2 = -11/10
    let r = prim_sub(&mut st, &mut env, list2(mk_rat(2, 5).unwrap(), mk_rat(3, 2).unwrap()));
    assert_rat(&r, -11.0, 10.0);

    // 2.1 - 2 = 0.1
    let r = prim_sub(&mut st, &mut env, list2(mk_dec(2.1).unwrap(), mk_int(2)));
    assert_dec_close(&r, 0.1, 1e-12);
}

#[test]
fn test_prim_mul() {
    let mut st = make_st();
    let mut env = fresh_env();

    // 3 * 2 = 6
    let r = prim_mul(&mut st, &mut env, list2(mk_int(3), mk_int(2)));
    assert_int(&r, 6);

    // -2 * 8.89 = -17.78
    let r = prim_mul(&mut st, &mut env, list2(mk_int(-2), mk_dec(8.89).unwrap()));
    assert_dec_close(&r, -17.78, 1e-12);

    // 6 * 3/4 = 9/2
    let r = prim_mul(&mut st, &mut env, list2(mk_int(6), mk_rat(3, 4).unwrap()));
    assert_rat(&r, 9.0, 2.0);

    // 1/3 * 6 = 2
    let r = prim_mul(&mut st, &mut env, list2(mk_rat(1, 3).unwrap(), mk_int(6)));
    assert_int(&r, 2);

    // 6/8 * 8/7 = 6/7
    let r = prim_mul(&mut st, &mut env, list2(mk_rat(6, 8).unwrap(), mk_rat(8, 7).unwrap()));
    assert_rat(&r, 6.0, 7.0);
}

#[test]
fn test_prim_div() {
    let mut st = make_st();
    let mut env = fresh_env();

    // 1 / 2 = 1/2
    let r = prim_div(&mut st, &mut env, list2(mk_int(1), mk_int(2)));
    assert_rat(&r, 1.0, 2.0);

    // 8 / 4 = 2
    let r = prim_div(&mut st, &mut env, list2(mk_int(8), mk_int(4)));
    assert_int(&r, 2);

    // unary / 5 = 1/5
    let r = prim_div(&mut st, &mut env, list1(mk_int(5)));
    assert_rat(&r, 1.0, 5.0);

    // unary / 4473 = 1/4473
    let r = prim_div(&mut st, &mut env, list1(mk_int(4473)));
    assert_rat(&r, 1.0, 4473.0);

    // 4 / 4/3 = 3
    let r = prim_div(&mut st, &mut env, list2(mk_int(4), mk_rat(4, 3).unwrap()));
    assert_int(&r, 3);

    // 4/3 / 7 = 4/21
    let r = prim_div(&mut st, &mut env, list2(mk_rat(4, 3).unwrap(), mk_int(7)));
    assert_rat(&r, 4.0, 21.0);

    // 1/3 / 5/4 = 4/15
    let r = prim_div(&mut st, &mut env, list2(mk_rat(1, 3).unwrap(), mk_rat(5, 4).unwrap()));
    assert_rat(&r, 4.0, 15.0);
}

#[test]
fn test_prim_mod() {
    let mut st = make_st();
    let mut env = fresh_env();

    // 10 mod 3 = 1
    let r = prim_mod(&mut st, &mut env, list2(mk_int(10), mk_int(3)));
    assert_int(&r, 1);

    // -11 mod 3 = -2
    let r = prim_mod(&mut st, &mut env, list2(mk_int(-11), mk_int(3)));
    assert_int(&r, -2);

    // 10 mod -3 = 1 (C: %abs)
    let r = prim_mod(&mut st, &mut env, list2(mk_int(10), mk_int(-3)));
    assert_int(&r, 1);

    // -10 mod -3 = -1
    let r = prim_mod(&mut st, &mut env, list2(mk_int(-10), mk_int(-3)));
    assert_int(&r, -1);

    // 10 mod 5 = 0
    let r = prim_mod(&mut st, &mut env, list2(mk_int(10), mk_int(5)));
    assert_int(&r, 0);

    // mod by 0 -> none
    let r = prim_mod(&mut st, &mut env, list2(mk_int(10), mk_int(0)));
    assert!(matches!(r.t, TspType::TspNone));
}

#[test]
fn test_prim_pow() {
    let mut st = make_st();
    let mut env = fresh_env();

    // 2^3 = 8
    let r = prim_pow(&mut st, &mut env, list2(mk_int(2), mk_int(3)));
    assert_int(&r, 8);

    // 2.0^3.0 = 8.0
    let r = prim_pow(&mut st, &mut env, list2(mk_dec(2.0).unwrap(), mk_dec(3.0).unwrap()));
    assert_dec(&r, 8.0);
}

#[test]
fn test_prim_numerator_denominator() {
    let mut st = make_st();
    let mut env = fresh_env();

    // numerator 3 = 3
    let r = prim_numerator(&mut st, &mut env, list1(mk_int(3)));
    assert_int(&r, 3);
    // numerator 9/2 = 9
    let r = prim_numerator(&mut st, &mut env, list1(mk_rat(9, 2).unwrap()));
    assert_int(&r, 9);
    // numerator 9/15 = 3 (after reduce)
    let r = prim_numerator(&mut st, &mut env, list1(mk_rat(9, 15).unwrap()));
    assert_int(&r, 3);
    // denominator 83 = 1
    let r = prim_denominator(&mut st, &mut env, list1(mk_int(83)));
    assert_int(&r, 1);
    // denominator 3/2 = 2
    let r = prim_denominator(&mut st, &mut env, list1(mk_rat(3, 2).unwrap()));
    assert_int(&r, 2);
    // denominator 10/15 = 3 (after reduce)
    let r = prim_denominator(&mut st, &mut env, list1(mk_rat(10, 15).unwrap()));
    assert_int(&r, 3);
}

#[test]
fn test_prim_compare() {
    let mut st = make_st();
    let mut env = fresh_env();

    // < 2 3 -> True
    let r = prim_lt(&mut st, &mut env, list2(mk_int(2), mk_int(3)));
    assert!(matches!(r.t, TspType::TspSym));
    // < 3 3 -> Nil
    let r = prim_lt(&mut st, &mut env, list2(mk_int(3), mk_int(3)));
    assert!(matches!(r.t, TspType::TspNil));
    // > 89 34 -> True
    let r = prim_gt(&mut st, &mut env, list2(mk_int(89), mk_int(34)));
    assert!(matches!(r.t, TspType::TspSym));
    // <= -2 -2 -> True
    let r = prim_lte(&mut st, &mut env, list2(mk_int(-2), mk_int(-2)));
    assert!(matches!(r.t, TspType::TspSym));
    // >= 39 39 -> True
    let r = prim_gte(&mut st, &mut env, list2(mk_int(39), mk_int(39)));
    assert!(matches!(r.t, TspType::TspSym));
    // >= -32 -30 -> Nil
    let r = prim_gte(&mut st, &mut env, list2(mk_int(-32), mk_int(-30)));
    assert!(matches!(r.t, TspType::TspNil));
}

#[test]
fn test_prim_round() {
    let mut st = make_st();
    let mut env = fresh_env();

    // round 7/3 = 2 — for Ratio input, create_rat is used; reduce 2/1 -> int 2
    let r = prim_round(&mut st, &mut env, list1(mk_rat(7, 3).unwrap()));
    assert_int(&r, 2);

    // round -3/4 = -1 (rounds away)
    let r = prim_round(&mut st, &mut env, list1(mk_rat(-3, 4).unwrap()));
    assert_int(&r, -1);

    let r = prim_round(&mut st, &mut env, list1(mk_int(7)));
    assert_int(&r, 7);

    // round 6.3 = 6.0 (Dec input -> create_dec)
    let r = prim_round(&mut st, &mut env, list1(mk_dec(6.3).unwrap()));
    assert_dec(&r, 6.0);

    // floor 5/3 -> ratio input -> create_rat. floor(5/3) = 1. mk_rat(1, 1) -> int 1
    let r = prim_floor(&mut st, &mut env, list1(mk_rat(5, 3).unwrap()));
    assert_int(&r, 1);

    // floor -8.1 = -9.0
    let r = prim_floor(&mut st, &mut env, list1(mk_dec(-8.1).unwrap()));
    assert_dec(&r, -9.0);

    // ceil 1/2 = 1 (mk_rat(1,1) -> int 1)
    let r = prim_ceil(&mut st, &mut env, list1(mk_rat(1, 2).unwrap()));
    assert_int(&r, 1);

    // ceil -2 = -2
    let r = prim_ceil(&mut st, &mut env, list1(mk_int(-2)));
    assert_int(&r, -2);
}

#[test]
fn test_prim_int_dec_conv() {
    let mut st = make_st();
    let mut env = fresh_env();

    // Int 1/2 = 0
    let r = prim_int(&mut st, &mut env, list1(mk_rat(1, 2).unwrap()));
    assert_int(&r, 0);
    // Int 3.14 = 3
    let r = prim_int(&mut st, &mut env, list1(mk_dec(3.14).unwrap()));
    assert_int(&r, 3);
    // Int 3/-2 = -1 (3/-2 reduces to -3/2, then Int(-3/2) = -3/2 = -1.5 truncated = -1)
    let r = prim_int(&mut st, &mut env, list1(mk_rat(3, -2).unwrap()));
    assert_int(&r, -1);

    // Dec 1/2 = 0.5
    let r = prim_dec(&mut st, &mut env, list1(mk_rat(1, 2).unwrap()));
    assert_dec(&r, 0.5);
    // Dec 1 = 1.0
    let r = prim_dec(&mut st, &mut env, list1(mk_int(1)));
    assert_dec(&r, 1.0);
    // Dec 3.14 = 3.14
    let r = prim_dec(&mut st, &mut env, list1(mk_dec(3.14).unwrap()));
    assert_dec(&r, 3.14);
}

#[test]
fn test_trig_funcs() {
    let mut st = make_st();
    let mut env = fresh_env();

    // sin 0.0 = 0.0
    let r = prim_sin(&mut st, &mut env, list1(mk_dec(0.0).unwrap()));
    assert_dec(&r, 0.0);
    // cos 0.0 = 1.0
    let r = prim_cos(&mut st, &mut env, list1(mk_dec(0.0).unwrap()));
    assert_dec(&r, 1.0);
    // tan 0.0 = 0.0
    let r = prim_tan(&mut st, &mut env, list1(mk_dec(0.0).unwrap()));
    assert_dec(&r, 0.0);
    // exp 1.0 ≈ 2.71828...
    let r = prim_exp(&mut st, &mut env, list1(mk_dec(1.0).unwrap()));
    assert_dec_close(&r, 2.718281828459045, 1e-12);
    // log 1.0 = 0.0
    let r = prim_log(&mut st, &mut env, list1(mk_dec(1.0).unwrap()));
    assert_dec(&r, 0.0);

    // arcsin 0.0 = 0.0
    let r = prim_asin(&mut st, &mut env, list1(mk_dec(0.0).unwrap()));
    assert_dec(&r, 0.0);
    // arctan 1.0 ≈ pi/4
    let r = prim_atan(&mut st, &mut env, list1(mk_dec(1.0).unwrap()));
    assert_dec_close(&r, std::f64::consts::FRAC_PI_4, 1e-12);

    // Hyperbolic
    let r = prim_sinh(&mut st, &mut env, list1(mk_dec(0.0).unwrap()));
    assert_dec(&r, 0.0);
    let r = prim_cosh(&mut st, &mut env, list1(mk_dec(0.0).unwrap()));
    assert_dec(&r, 1.0);
    let r = prim_tanh(&mut st, &mut env, list1(mk_dec(0.0).unwrap()));
    assert_dec(&r, 0.0);
    let r = prim_asinh(&mut st, &mut env, list1(mk_dec(0.0).unwrap()));
    assert_dec(&r, 0.0);
    let r = prim_acosh(&mut st, &mut env, list1(mk_dec(1.0).unwrap()));
    assert_dec(&r, 0.0);
    let r = prim_atanh(&mut st, &mut env, list1(mk_dec(0.0).unwrap()));
    assert_dec(&r, 0.0);
    let r = prim_acos(&mut st, &mut env, list1(mk_dec(1.0).unwrap()));
    assert_dec(&r, 0.0);

    // Non-Dec input -> None
    let r = prim_sin(&mut st, &mut env, list1(mk_int(0)));
    assert!(matches!(r.t, TspType::TspNone));
}

#[test]
fn test_tib_env_math_registers() {
    let mut st = tisp_env_init(64);
    tisp_proj::tisp::tib_env_math(&mut st);
    // Should have all math primitive names
    let names = ["+", "-", "*", "/", "mod", "^", "<", ">", "<=", ">=",
                 "Int", "Dec", "round", "floor", "ceil", "numerator", "denominator",
                 "sin", "cos", "tan", "exp", "log",
                 "arcsin", "arccos", "arctan", "arcsinh", "arccosh", "arctanh",
                 "sinh", "cosh", "tanh"];
    for n in names {
        assert!(tisp_proj::tisp::rec_get(&st.env, n).is_some(),
                "expected '{}' to be registered", n);
    }
}

fn main() {}
