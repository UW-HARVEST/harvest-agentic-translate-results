#![allow(dead_code, unused_imports)]
use tisp_proj::core::*;
use tisp_proj::tisp::{
    mk_dec, mk_int, mk_pair, mk_rat, mk_str, mk_sym, rec_get, rec_new, tisp_env_init,
    Rec, Tsp, TspType, Val, ValUnion,
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

fn list1(a: Val) -> Val {
    mk_pair(a, nil()).unwrap()
}

fn list2(a: Val, b: Val) -> Val {
    mk_pair(a, mk_pair(b, nil()).unwrap()).unwrap()
}

fn list3(a: Val, b: Val, c: Val) -> Val {
    mk_pair(a, mk_pair(b, mk_pair(c, nil()).unwrap()).unwrap()).unwrap()
}

#[test]
fn test_prim_car() {
    let mut st = make_st();
    let mut env = fresh_env();
    // (car '(1 2 3)) — args is wrapped as ((1 2 3))
    let inner = list3(mk_int(1), mk_int(2), mk_int(3));
    let r = prim_car(&mut st, &mut env, list1(inner));
    assert!(matches!(r.t, TspType::TspInt));
    if let ValUnion::N { num, .. } = &r.v {
        assert_eq!(*num, 1.0);
    }
}

#[test]
fn test_prim_cdr() {
    let mut st = make_st();
    let mut env = fresh_env();
    let inner = list3(mk_int(1), mk_int(2), mk_int(3));
    let r = prim_cdr(&mut st, &mut env, list1(inner));
    // Should be (2 3)
    assert!(matches!(r.t, TspType::TspPair));
    if let ValUnion::P { car, cdr } = &r.v {
        assert!(matches!(car.t, TspType::TspInt));
        if let ValUnion::N { num, .. } = &car.v {
            assert_eq!(*num, 2.0);
        }
        // cdr should be (3) which is a pair
        assert!(matches!(cdr.t, TspType::TspPair));
    }
}

#[test]
fn test_prim_cons() {
    let mut st = make_st();
    let mut env = fresh_env();
    // cons of (1 2) -> (1 . 2)
    let r = prim_cons(&mut st, &mut env, list2(mk_int(1), mk_int(2)));
    assert!(matches!(r.t, TspType::TspPair));
    if let ValUnion::P { car, cdr } = &r.v {
        if let ValUnion::N { num, .. } = &car.v {
            assert_eq!(*num, 1.0);
        }
        if let ValUnion::N { num, .. } = &cdr.v {
            assert_eq!(*num, 2.0);
        }
    }
}

#[test]
fn test_form_quote() {
    let mut st = make_st();
    let mut env = fresh_env();
    // quote returns first arg unevaluated
    let r = form_quote(&mut st, &mut env, list1(mk_int(42)));
    assert!(matches!(r.t, TspType::TspInt));
    if let ValUnion::N { num, .. } = &r.v {
        assert_eq!(*num, 42.0);
    }
}

#[test]
fn test_prim_eq_basic() {
    let mut st = make_st();
    let mut env = fresh_env();

    // (=) -> True
    let r = prim_eq(&mut st, &mut env, nil());
    assert!(matches!(r.t, TspType::TspSym));

    // (= 1) -> True
    let r = prim_eq(&mut st, &mut env, list1(mk_int(1)));
    assert!(matches!(r.t, TspType::TspSym));

    // (= 1 1) -> True
    let r = prim_eq(&mut st, &mut env, list2(mk_int(1), mk_int(1)));
    assert!(matches!(r.t, TspType::TspSym));

    // (= 1 2) -> Nil
    let r = prim_eq(&mut st, &mut env, list2(mk_int(1), mk_int(2)));
    assert!(matches!(r.t, TspType::TspNil));

    // (= 1 1 1) -> True
    let r = prim_eq(&mut st, &mut env, list3(mk_int(1), mk_int(1), mk_int(1)));
    assert!(matches!(r.t, TspType::TspSym));

    // (= 1 1 2) -> Nil
    let r = prim_eq(&mut st, &mut env, list3(mk_int(1), mk_int(1), mk_int(2)));
    assert!(matches!(r.t, TspType::TspNil));

    // (= 2/4 1/2) -> True
    let r = prim_eq(&mut st, &mut env,
        list2(mk_rat(2, 4).unwrap(), mk_rat(1, 2).unwrap()));
    assert!(matches!(r.t, TspType::TspSym));

    // (= 2/1 2) -> True
    let r = prim_eq(&mut st, &mut env,
        list2(mk_rat(2, 1).unwrap(), mk_int(2)));
    assert!(matches!(r.t, TspType::TspSym));
}

#[test]
fn test_prim_typeof() {
    let mut st = make_st();
    let mut env = fresh_env();

    // typeof 5 -> "Int"
    let r = prim_typeof(&mut st, &mut env, list1(mk_int(5)));
    assert!(matches!(r.t, TspType::TspStr));
    if let ValUnion::S(s) = &r.v {
        assert_eq!(s, "Int");
    }

    // typeof 3.14 -> "Dec"
    let r = prim_typeof(&mut st, &mut env, list1(mk_dec(3.14).unwrap()));
    if let ValUnion::S(s) = &r.v {
        assert_eq!(s, "Dec");
    }

    // typeof 1/2 -> "Ratio"
    let r = prim_typeof(&mut st, &mut env, list1(mk_rat(1, 2).unwrap()));
    if let ValUnion::S(s) = &r.v {
        assert_eq!(s, "Ratio");
    }

    // typeof Nil
    let r = prim_typeof(&mut st, &mut env, list1(nil()));
    if let ValUnion::S(s) = &r.v {
        assert_eq!(s, "Nil");
    }

    // typeof Void
    let void_v = Val { t: TspType::TspNone, v: ValUnion::N { num: 0.0, den: 1.0 } };
    let r = prim_typeof(&mut st, &mut env, list1(void_v));
    if let ValUnion::S(s) = &r.v {
        assert_eq!(s, "Void");
    }

    // typeof a Sym
    let s = mk_sym(&mut st, "foo").unwrap();
    let r = prim_typeof(&mut st, &mut env, list1(s));
    if let ValUnion::S(s) = &r.v {
        assert_eq!(s, "Sym");
    }

    // typeof string
    let s = mk_str(&mut st, "hello").unwrap();
    let r = prim_typeof(&mut st, &mut env, list1(s));
    if let ValUnion::S(s) = &r.v {
        assert_eq!(s, "Str");
    }

    // typeof a pair
    let p = mk_pair(mk_int(1), mk_int(2)).unwrap();
    let r = prim_typeof(&mut st, &mut env, list1(p));
    if let ValUnion::S(s) = &r.v {
        assert_eq!(s, "Pair");
    }
}

#[test]
fn test_form_func() {
    let mut st = make_st();
    let mut env = fresh_env();

    // Func with two args: parameters and body
    let it_sym = mk_sym(&mut st, "x").unwrap();
    let params = mk_pair(it_sym, nil()).unwrap();
    let body = list1(mk_int(42));
    let args = mk_pair(params, body).unwrap();
    let r = form_Func(&mut st, &mut env, args);
    assert!(matches!(r.t, TspType::TspFunc));
}

#[test]
fn test_form_macro() {
    let mut st = make_st();
    let mut env = fresh_env();
    let it_sym = mk_sym(&mut st, "x").unwrap();
    let params = mk_pair(it_sym, nil()).unwrap();
    let body = list1(mk_int(42));
    let args = mk_pair(params, body).unwrap();
    let r = form_Macro(&mut st, &mut env, args);
    assert!(matches!(r.t, TspType::TspMacro));
}

#[test]
fn test_form_def_variable() {
    let mut st = make_st();
    let mut env = fresh_env();
    // (def foo 4)
    let foo = mk_sym(&mut st, "foo").unwrap();
    let r = form_def(&mut st, &mut env, list2(foo, mk_int(4)));
    assert!(matches!(r.t, TspType::TspNone));
    // foo should be defined now in st.env
    let v = rec_get(&st.env, "foo").unwrap();
    assert!(matches!(v.t, TspType::TspInt));
    if let ValUnion::N { num, .. } = &v.v {
        assert_eq!(*num, 4.0);
    }
}

#[test]
fn test_form_definedp() {
    let mut st = make_st();
    let mut env = fresh_env();

    // (defined? car) -> True (car is in env after tib_env_core)
    let car_sym = mk_sym(&mut st, "car").unwrap();
    let r = form_definedp(&mut st, &mut env, list1(car_sym));
    assert!(matches!(r.t, TspType::TspSym));

    // (defined? bogus-symbol-name) -> Nil
    let bogus = mk_sym(&mut st, "this-is-not-defined-anywhere").unwrap();
    let r = form_definedp(&mut st, &mut env, list1(bogus));
    assert!(matches!(r.t, TspType::TspNil));
}

#[test]
fn test_form_undefine() {
    let mut st = make_st();
    let mut env = fresh_env();

    // first define something
    let foo = mk_sym(&mut st, "myundef_test").unwrap();
    form_def(&mut st, &mut env, list2(foo, mk_int(99)));
    assert!(rec_get(&st.env, "myundef_test").is_some());

    // undefine it
    let foo2 = mk_sym(&mut st, "myundef_test").unwrap();
    let r = form_undefine(&mut st, &mut env, list1(foo2));
    assert!(matches!(r.t, TspType::TspNone));
    assert!(rec_get(&st.env, "myundef_test").is_none());
}

#[test]
fn test_prim_recmerge_non_rec() {
    let mut st = make_st();
    let mut env = fresh_env();
    // Pass non-records — should return None type
    let r = prim_recmerge(&mut st, &mut env, list2(mk_int(1), mk_int(2)));
    assert!(matches!(r.t, TspType::TspNone));
}

#[test]
fn test_prim_records_non_rec() {
    let mut st = make_st();
    let mut env = fresh_env();
    let r = prim_records(&mut st, &mut env, list1(mk_int(1)));
    // Non-record arg returns None
    assert!(matches!(r.t, TspType::TspNone));
}

#[test]
fn test_tib_env_core_registers() {
    let mut st = tisp_env_init(64);
    tisp_proj::tisp::tib_env_core(&mut st);
    let names = ["car", "cdr", "cons", "quote", "eval", "=", "cond", "do",
                 "typeof", "procprops", "Func", "Macro", "error",
                 "Rec", "recmerge", "records", "def", "undefine!", "defined?"];
    for n in names {
        assert!(rec_get(&st.env, n).is_some(), "expected '{}' to be registered", n);
    }
}

#[test]
fn test_prim_eval_simple() {
    let mut st = make_st();
    let mut env = fresh_env();
    // (eval "sup") - eval of self-eval'ing string returns the string
    let s = mk_str(&mut st, "sup").unwrap();
    let r = prim_eval(&mut st, &mut env, list1(s));
    assert!(matches!(r.t, TspType::TspStr));
    if let ValUnion::S(s) = &r.v {
        assert_eq!(s, "sup");
    }

    // (eval 5) -> 5
    let r = prim_eval(&mut st, &mut env, list1(mk_int(5)));
    if let ValUnion::N { num, .. } = &r.v {
        assert_eq!(*num, 5.0);
    }
}

#[test]
fn test_prim_error_returns_none() {
    let mut st = make_st();
    let mut env = fresh_env();
    let r = prim_error(&mut st, &mut env, nil());
    assert!(matches!(r.t, TspType::TspNone));
}

fn main() {}
