use tisp_proj::core::{form_quote, prim_car, prim_cdr, prim_cons, prim_eq, prim_typeof};
use tisp_proj::tisp::{
    clone_val, mk_int, mk_pair, mk_str, mk_val, tisp_env_init, TspType, Val, ValUnion,
};

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
fn test_prim_cons() {
    let mut st = tisp_env_init(16);
    let mut env = tisp_proj::tisp::rec_new(8, None);
    let args = make_args2(mk_int(1), mk_int(2));
    let result = prim_cons(&mut st, &mut env, args);
    assert!(matches!(result.t, TspType::TspPair));
    if let ValUnion::P { car, cdr } = result.v {
        if let ValUnion::N { num, .. } = car.v {
            assert_eq!(num, 1.0);
        }
        if let ValUnion::N { num, .. } = cdr.v {
            assert_eq!(num, 2.0);
        }
    }
}

#[test]
fn test_prim_car() {
    let mut st = tisp_env_init(16);
    let mut env = tisp_proj::tisp::rec_new(8, None);
    // (car (cons 1 2)) -> 1
    let pair = mk_pair(mk_int(1), mk_int(2)).unwrap();
    let args = make_args1(pair);
    let result = prim_car(&mut st, &mut env, args);
    assert!(matches!(result.t, TspType::TspInt));
    if let ValUnion::N { num, den } = result.v {
        assert_eq!(num, 1.0);
        assert_eq!(den, 1.0);
    }
}

#[test]
fn test_prim_cdr() {
    let mut st = tisp_env_init(16);
    let mut env = tisp_proj::tisp::rec_new(8, None);
    let pair = mk_pair(mk_int(1), mk_int(2)).unwrap();
    let args = make_args1(pair);
    let result = prim_cdr(&mut st, &mut env, args);
    assert!(matches!(result.t, TspType::TspInt));
    if let ValUnion::N { num, .. } = result.v {
        assert_eq!(num, 2.0);
    }
}

#[test]
fn test_form_quote() {
    let mut st = tisp_env_init(16);
    let mut env = tisp_proj::tisp::rec_new(8, None);
    let args = make_args1(mk_int(42));
    let result = form_quote(&mut st, &mut env, args);
    assert!(matches!(result.t, TspType::TspInt));
    if let ValUnion::N { num, .. } = result.v {
        assert_eq!(num, 42.0);
    }
}

#[test]
fn test_prim_eq_empty() {
    let mut st = tisp_env_init(16);
    let mut env = tisp_proj::tisp::rec_new(8, None);
    let nil = mk_val(TspType::TspNil);
    let result = prim_eq(&mut st, &mut env, nil);
    // (= ) -> True
    assert!(matches!(result.t, TspType::TspSym));
    if let ValUnion::S(s) = result.v {
        assert_eq!(s, "True");
    }
}

#[test]
fn test_prim_eq_single() {
    let mut st = tisp_env_init(16);
    let mut env = tisp_proj::tisp::rec_new(8, None);
    let args = make_args1(mk_int(1));
    let result = prim_eq(&mut st, &mut env, args);
    // (= 1) -> True
    assert!(matches!(result.t, TspType::TspSym));
    if let ValUnion::S(s) = result.v {
        assert_eq!(s, "True");
    }
}

#[test]
fn test_prim_eq_two_equal() {
    let mut st = tisp_env_init(16);
    let mut env = tisp_proj::tisp::rec_new(8, None);
    let args = make_args2(mk_int(5), mk_int(5));
    let result = prim_eq(&mut st, &mut env, args);
    assert!(matches!(result.t, TspType::TspSym));
    if let ValUnion::S(s) = result.v {
        assert_eq!(s, "True");
    }
}

#[test]
fn test_prim_eq_two_unequal() {
    let mut st = tisp_env_init(16);
    let mut env = tisp_proj::tisp::rec_new(8, None);
    let args = make_args2(mk_int(5), mk_int(6));
    let result = prim_eq(&mut st, &mut env, args);
    // (= 5 6) -> Nil
    assert!(matches!(result.t, TspType::TspNil));
}

#[test]
fn test_prim_typeof_int() {
    let mut st = tisp_env_init(16);
    let mut env = tisp_proj::tisp::rec_new(8, None);
    let args = make_args1(mk_int(1));
    let result = prim_typeof(&mut st, &mut env, args);
    assert!(matches!(result.t, TspType::TspStr));
    if let ValUnion::S(s) = result.v {
        assert_eq!(s, "Int");
    }
}

#[test]
fn test_prim_typeof_str() {
    let mut st = tisp_env_init(16);
    let mut env = tisp_proj::tisp::rec_new(8, None);
    let s = mk_str(&mut st, "foo").unwrap();
    let args = make_args1(s);
    let result = prim_typeof(&mut st, &mut env, args);
    if let ValUnion::S(s) = result.v {
        assert_eq!(s, "Str");
    }
}

#[test]
fn test_prim_typeof_ratio() {
    let mut st = tisp_env_init(16);
    let mut env = tisp_proj::tisp::rec_new(8, None);
    let r = tisp_proj::tisp::mk_rat(1, 2).unwrap();
    let args = make_args1(r);
    let result = prim_typeof(&mut st, &mut env, args);
    if let ValUnion::S(s) = result.v {
        assert_eq!(s, "Ratio");
    }
}

#[test]
fn test_prim_typeof_dec() {
    let mut st = tisp_env_init(16);
    let mut env = tisp_proj::tisp::rec_new(8, None);
    let d = tisp_proj::tisp::mk_dec(1.5).unwrap();
    let args = make_args1(d);
    let result = prim_typeof(&mut st, &mut env, args);
    if let ValUnion::S(s) = result.v {
        assert_eq!(s, "Dec");
    }
}

#[test]
fn test_prim_typeof_nil() {
    let mut st = tisp_env_init(16);
    let mut env = tisp_proj::tisp::rec_new(8, None);
    let nil = clone_val(&st.nil);
    let args = make_args1(nil);
    let result = prim_typeof(&mut st, &mut env, args);
    if let ValUnion::S(s) = result.v {
        assert_eq!(s, "Nil");
    }
}

fn main() {}
