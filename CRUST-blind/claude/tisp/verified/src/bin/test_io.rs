#![allow(dead_code, unused_imports)]
use tisp_proj::io::*;
use tisp_proj::tisp::{
    mk_int, mk_pair, mk_str, mk_sym, rec_get, rec_new, tisp_env_init, Rec, Tsp, TspType, Val,
    ValUnion,
};

fn fresh_env() -> Rec {
    rec_new(64, None)
}

fn make_st() -> Tsp {
    let mut st = tisp_env_init(64);
    tisp_proj::tisp::tib_env_io(&mut st);
    tisp_proj::tisp::tib_env_string(&mut st);
    st
}

fn nil() -> Val {
    Val { t: TspType::TspNil, v: ValUnion::N { num: 0.0, den: 1.0 } }
}

fn list1(a: Val) -> Val {
    mk_pair(a, nil()).unwrap()
}

#[test]
fn test_count_parens_balanced() {
    // No parens
    assert_eq!(count_parens("hello", 5), 0);
    // Balanced
    assert_eq!(count_parens("()", 2), 0);
    assert_eq!(count_parens("(abc)", 5), 0);
    // Single open
    assert_eq!(count_parens("(", 1), 1);
    // Multi open
    assert_eq!(count_parens("(((", 3), 3);
    // Multi close (negative)
    assert_eq!(count_parens(")))", 3), -3);
    // Brackets
    assert_eq!(count_parens("[", 1), 1);
    assert_eq!(count_parens("[]", 2), 0);
    // Curly braces — only counted if parens and brackets balance
    assert_eq!(count_parens("{", 1), 1);
    assert_eq!(count_parens("{}", 2), 0);
}

#[test]
fn test_count_parens_paren_priority() {
    // C code: returns pcount first if non-zero, then bcount, then ccount
    // mix open/close
    assert_eq!(count_parens("(()", 3), 1);
    assert_eq!(count_parens("(()[])", 6), 0);
    // Only counts brackets if parens balance
    assert_eq!(count_parens("([", 2), 1);
    assert_eq!(count_parens("()[", 3), 1); // pcount = 0, bcount = 1 -> returns 1
}

#[test]
fn test_count_parens_truncation_by_len() {
    // len = 0 -> nothing read
    assert_eq!(count_parens("(((", 0), 0);
    assert_eq!(count_parens("(((", 1), 1);
    assert_eq!(count_parens("(((", 2), 2);
}

#[test]
fn test_prim_parse_simple_number() {
    let mut st = make_st();
    let mut env = fresh_env();
    // (parse "42") returns parsed expression
    let s = mk_str(&mut st, "42").unwrap();
    let r = prim_parse(&mut st, &mut env, list1(s));
    // Should be an int 42 (single expr)
    assert!(matches!(r.t, TspType::TspInt));
    if let ValUnion::N { num, .. } = &r.v {
        assert_eq!(*num, 42.0);
    }
}

#[test]
fn test_prim_parse_nil_returns_quit() {
    let mut st = make_st();
    let mut env = fresh_env();
    let r = prim_parse(&mut st, &mut env, list1(nil()));
    assert!(matches!(r.t, TspType::TspSym));
    if let ValUnion::S(s) = &r.v {
        assert_eq!(s, "quit");
    }
}

#[test]
fn test_prim_parse_non_string_returns_none() {
    let mut st = make_st();
    let mut env = fresh_env();
    let r = prim_parse(&mut st, &mut env, list1(mk_int(5)));
    assert!(matches!(r.t, TspType::TspNone));
}

#[test]
fn test_prim_read_nonexistent_returns_nil() {
    let mut st = make_st();
    let mut env = fresh_env();
    let s = mk_str(&mut st, "/nonexistent/file/path-12345.tsp").unwrap();
    let r = prim_read(&mut st, &mut env, list1(s));
    assert!(matches!(r.t, TspType::TspNil));
}

#[test]
fn test_read_file_nonexistent_empty() {
    let s = read_file("/nonexistent/does-not-exist-12345");
    assert_eq!(s, "");
}

#[test]
fn test_read_file_existing() {
    // Use Cargo.toml as a known existing file
    let s = read_file("Cargo.toml");
    assert!(s.contains("tisp_proj"));
}

#[test]
fn test_tib_env_io_registers() {
    let mut st = tisp_env_init(64);
    tisp_proj::tisp::tib_env_io(&mut st);
    let names = ["write", "read", "parse", "load"];
    for n in names {
        assert!(rec_get(&st.env, n).is_some(), "expected '{}' to be registered", n);
    }
}

#[test]
fn test_prim_write_returns_none() {
    let mut st = make_st();
    let mut env = fresh_env();
    // (write 'stderr Nil "x") - 3 args minimum
    let stderr_sym = mk_sym(&mut st, "stderr").unwrap();
    let args = mk_pair(stderr_sym,
                       mk_pair(nil(),
                               list1(mk_str(&mut st, "x").unwrap())).unwrap()).unwrap();
    let r = prim_write(&mut st, &mut env, args);
    assert!(matches!(r.t, TspType::TspNone));
}

#[test]
fn test_prim_write_too_few_args() {
    let mut st = make_st();
    let mut env = fresh_env();
    // tsp_arg_min for write is 2; passing 1 arg
    let stdout = mk_sym(&mut st, "stdout").unwrap();
    let r = prim_write(&mut st, &mut env, list1(stdout));
    // Should return None when less than 2 args
    assert!(matches!(r.t, TspType::TspNone));
}

fn main() {}
