use tisp_proj::string::{prim_strlen, prim_Str, prim_Sym};
use tisp_proj::tisp::{
    mk_int, mk_pair, mk_str, mk_sym, mk_val, rec_new, tisp_env_init, TspType, Val, ValUnion,
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
fn test_prim_strlen_str() {
    let mut st = tisp_env_init(16);
    let mut env = rec_new(8, None);
    let s = mk_str(&mut st, "hello").unwrap();
    let args = make_args1(s);
    let r = prim_strlen(&mut st, &mut env, args);
    assert!(matches!(r.t, TspType::TspInt));
    if let ValUnion::N { num, .. } = r.v {
        assert_eq!(num, 5.0);
    }
}

#[test]
fn test_prim_strlen_sym() {
    let mut st = tisp_env_init(16);
    let mut env = rec_new(8, None);
    let s = mk_sym(&mut st, "abc").unwrap();
    let args = make_args1(s);
    let r = prim_strlen(&mut st, &mut env, args);
    if let ValUnion::N { num, .. } = r.v {
        assert_eq!(num, 3.0);
    }
}

#[test]
fn test_prim_strlen_empty() {
    let mut st = tisp_env_init(16);
    let mut env = rec_new(8, None);
    let s = mk_str(&mut st, "").unwrap();
    let args = make_args1(s);
    let r = prim_strlen(&mut st, &mut env, args);
    if let ValUnion::N { num, .. } = r.v {
        assert_eq!(num, 0.0);
    }
}

#[test]
fn test_prim_str_concat() {
    // (Str "foo" "bar") -> "foobar"
    let mut st = tisp_env_init(16);
    let mut env = rec_new(8, None);
    let a = mk_str(&mut st, "foo").unwrap();
    let b = mk_str(&mut st, "bar").unwrap();
    let args = make_args2(a, b);
    let r = prim_Str(&mut st, &mut env, args);
    assert!(matches!(r.t, TspType::TspStr));
    if let ValUnion::S(s) = r.v {
        assert_eq!(s, "foobar");
    }
}

#[test]
fn test_prim_str_int() {
    // (Str 42) -> "42"
    let mut st = tisp_env_init(16);
    let mut env = rec_new(8, None);
    let args = make_args1(mk_int(42));
    let r = prim_Str(&mut st, &mut env, args);
    assert!(matches!(r.t, TspType::TspStr));
    if let ValUnion::S(s) = r.v {
        assert_eq!(s, "42");
    }
}

#[test]
fn test_prim_sym_concat() {
    // (Sym "foo" "bar") -> 'foobar
    let mut st = tisp_env_init(16);
    let mut env = rec_new(8, None);
    let a = mk_str(&mut st, "foo").unwrap();
    let b = mk_str(&mut st, "bar").unwrap();
    let args = make_args2(a, b);
    let r = prim_Sym(&mut st, &mut env, args);
    assert!(matches!(r.t, TspType::TspSym));
    if let ValUnion::S(s) = r.v {
        assert_eq!(s, "foobar");
    }
}

fn main() {}
