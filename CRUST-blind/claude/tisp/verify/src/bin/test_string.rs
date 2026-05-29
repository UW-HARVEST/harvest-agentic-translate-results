#![allow(dead_code, unused_imports)]
use tisp_proj::string::*;
use tisp_proj::tisp::{
    mk_int, mk_pair, mk_str, mk_sym, rec_get, rec_new, tisp_env_init, Rec, Tsp, TspType, Val,
    ValUnion,
};

fn fresh_env() -> Rec {
    rec_new(64, None)
}

fn make_st() -> Tsp {
    let mut st = tisp_env_init(64);
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

#[test]
fn test_prim_strlen_str() {
    let mut st = make_st();
    let mut env = fresh_env();
    // C: (strlen "hello") -> 5
    let s = mk_str(&mut st, "hello").unwrap();
    let r = prim_strlen(&mut st, &mut env, list1(s));
    assert!(matches!(r.t, TspType::TspInt));
    if let ValUnion::N { num, den } = &r.v {
        assert_eq!(*num, 5.0);
        assert_eq!(*den, 1.0);
    }
}

#[test]
fn test_prim_strlen_sym() {
    let mut st = make_st();
    let mut env = fresh_env();
    // C: (strlen 'foo) -> 3
    let s = mk_sym(&mut st, "foo").unwrap();
    let r = prim_strlen(&mut st, &mut env, list1(s));
    assert!(matches!(r.t, TspType::TspInt));
    if let ValUnion::N { num, .. } = &r.v {
        assert_eq!(*num, 3.0);
    }
}

#[test]
fn test_prim_strlen_empty() {
    let mut st = make_st();
    let mut env = fresh_env();
    let s = mk_str(&mut st, "").unwrap();
    let r = prim_strlen(&mut st, &mut env, list1(s));
    if let ValUnion::N { num, .. } = &r.v {
        assert_eq!(*num, 0.0);
    }
}

#[test]
fn test_prim_strlen_non_string() {
    let mut st = make_st();
    let mut env = fresh_env();
    // strlen on non-string should return None type
    let r = prim_strlen(&mut st, &mut env, list1(mk_int(42)));
    assert!(matches!(r.t, TspType::TspNone));
}

#[test]
fn test_prim_str_string_concat() {
    // C: (Str "abc") -> "abc"
    let mut st = make_st();
    let mut env = fresh_env();
    let s1 = mk_str(&mut st, "abc").unwrap();
    let r = prim_Str(&mut st, &mut env, list1(s1));
    assert!(matches!(r.t, TspType::TspStr));
    if let ValUnion::S(s) = &r.v {
        assert_eq!(s, "abc");
    }
}

#[test]
fn test_prim_str_concats_multiple_strings() {
    let mut st = make_st();
    let mut env = fresh_env();
    let s1 = mk_str(&mut st, "abc").unwrap();
    let s2 = mk_str(&mut st, "def").unwrap();
    let r = prim_Str(&mut st, &mut env, list2(s1, s2));
    assert!(matches!(r.t, TspType::TspStr));
    if let ValUnion::S(s) = &r.v {
        assert_eq!(s, "abcdef");
    }
}

#[test]
fn test_prim_sym_returns_sym() {
    let mut st = make_st();
    let mut env = fresh_env();
    let s1 = mk_sym(&mut st, "foo").unwrap();
    let s2 = mk_sym(&mut st, "bar").unwrap();
    let r = prim_Sym(&mut st, &mut env, list2(s1, s2));
    assert!(matches!(r.t, TspType::TspSym));
    if let ValUnion::S(s) = &r.v {
        assert_eq!(s, "foobar");
    }
}

#[test]
fn test_form_strformat_passthrough() {
    let mut st = make_st();
    let mut env = fresh_env();
    // No interpolation
    let s = mk_str(&mut st, "hello world").unwrap();
    let r = form_strformat(&mut st, &mut env, list1(s));
    assert!(matches!(r.t, TspType::TspStr));
    if let ValUnion::S(s) = &r.v {
        assert_eq!(s, "hello world");
    }
}

#[test]
fn test_tib_env_string_registers() {
    let mut st = tisp_env_init(64);
    tisp_proj::tisp::tib_env_string(&mut st);
    let names = ["Sym", "Str", "strlen", "strformat"];
    for n in names {
        assert!(rec_get(&st.env, n).is_some(), "expected '{}' to be registered", n);
    }
}

fn main() {}
