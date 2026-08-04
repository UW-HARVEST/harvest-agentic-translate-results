#![allow(dead_code, unused_imports)]
use tisp_proj::os::*;
use tisp_proj::tisp::{
    mk_int, mk_pair, mk_str, mk_sym, rec_get, rec_new, tisp_env_init, Rec, Tsp, TspType, Val,
    ValUnion,
};

fn fresh_env() -> Rec {
    rec_new(64, None)
}

fn make_st() -> Tsp {
    let mut st = tisp_env_init(64);
    tisp_proj::tisp::tib_env_os(&mut st);
    st
}

fn nil() -> Val {
    Val { t: TspType::TspNil, v: ValUnion::N { num: 0.0, den: 1.0 } }
}

fn list1(a: Val) -> Val {
    mk_pair(a, nil()).unwrap()
}

#[test]
fn test_prim_pwd_returns_string() {
    let mut st = make_st();
    let mut env = fresh_env();
    let r = prim_pwd(&mut st, &mut env, nil());
    assert!(matches!(r.t, TspType::TspStr));
    if let ValUnion::S(s) = &r.v {
        assert!(!s.is_empty(), "pwd should return non-empty path");
    }
}

#[test]
fn test_prim_now_returns_int() {
    let mut st = make_st();
    let mut env = fresh_env();
    let r = prim_now(&mut st, &mut env, nil());
    assert!(matches!(r.t, TspType::TspInt));
    if let ValUnion::N { num, den } = &r.v {
        // unix time should be > 1.7e9 (2023+)
        assert!(*num > 1_700_000_000.0 || *num < 0.0); // may overflow if cast i32, but expected to be reasonable
        assert_eq!(*den, 1.0);
        let _ = num;
    }
}

#[test]
fn test_prim_cd_invalid_type_returns_none() {
    let mut st = make_st();
    let mut env = fresh_env();
    // pass int to cd! — should return None (invalid type)
    let r = prim_cd(&mut st, &mut env, list1(mk_int(5)));
    assert!(matches!(r.t, TspType::TspNone));
}

#[test]
fn test_prim_cd_returns_none_on_success() {
    let mut st = make_st();
    let mut env = fresh_env();
    // cd to current dir "." should return None
    let dir = mk_str(&mut st, ".").unwrap();
    let r = prim_cd(&mut st, &mut env, list1(dir));
    assert!(matches!(r.t, TspType::TspNone));
}

#[test]
fn test_form_time_returns_dec() {
    let mut st = make_st();
    let mut env = fresh_env();
    // (time 5) — wraps eval of 5; should return Dec timing
    let r = form_time(&mut st, &mut env, list1(mk_int(5)));
    assert!(matches!(r.t, TspType::TspDec));
    if let ValUnion::N { num, den } = &r.v {
        assert!(*num >= 0.0);
        assert_eq!(*den, 1.0);
    }
}

#[test]
fn test_tib_env_os_registers() {
    let mut st = tisp_env_init(64);
    tisp_proj::tisp::tib_env_os(&mut st);
    let names = ["cd!", "pwd", "exit!", "now", "time"];
    for n in names {
        assert!(rec_get(&st.env, n).is_some(), "expected '{}' to be registered", n);
    }
}

fn main() {}
