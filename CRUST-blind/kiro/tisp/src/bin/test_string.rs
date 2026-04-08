use tisp_proj::tisp::*;
use tisp_proj::core;
use tisp_proj::math;
use tisp_proj::string;

const TIBS: &str = include_str!("../../tibs.tsp");

fn setup() -> Tsp {
    let mut st = tisp_env_init(1024);
    core::tib_env_core(&mut st);
    math::tib_env_math(&mut st);
    string::tib_env_string(&mut st);
    tisp_env_lib(&mut st, TIBS);
    st
}

fn eval_str(st: &mut Tsp, input: &str) -> String {
    st.file = input.to_string();
    st.filec = 0;
    let v = match tisp_read(st) {
        Some(v) => v,
        None => return "READ_ERROR".to_string(),
    };
    let v = match tisp_eval(st, v) {
        Some(v) => v,
        None => return "EVAL_ERROR".to_string(),
    };
    let mut buf = Vec::new();
    tisp_print(&mut buf, &v);
    String::from_utf8(buf).unwrap_or_default()
}

// Str
#[test] fn test_str1() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(Str 42)"), "42"); }
#[test] fn test_str2() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(Str 1/2)"), "1/2"); }
#[test] fn test_str3() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(Str \"hello\")"), "hello"); }
#[test] fn test_str4() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(Str 'world)"), "world"); }
#[test] fn test_str5() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(Str 3.14)"), "3.14"); }

// Sym
#[test] fn test_sym1() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(Sym \"hello\")"), "hello"); }
#[test] fn test_sym2() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(Sym 42)"), "42"); }

// strlen
#[test] fn test_strlen1() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(strlen \"hello\")"), "5"); }
#[test] fn test_strlen2() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(strlen \"\")"), "0"); }
#[test] fn test_strlen3() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(strlen \"foo bar\")"), "7"); }

// typeof
#[test] fn test_typeof1() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(typeof 1)"), "Int"); }
#[test] fn test_typeof2() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(typeof 3.14)"), "Dec"); }
#[test] fn test_typeof3() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(typeof 1/2)"), "Ratio"); }
#[test] fn test_typeof4() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(typeof \"foo\")"), "Str"); }
#[test] fn test_typeof5() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(typeof 'bar)"), "Sym"); }
#[test] fn test_typeof6() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(typeof '(1 2))"), "Pair"); }
#[test] fn test_typeof7() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(typeof Nil)"), "Nil"); }
#[test] fn test_typeof8() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(typeof Void)"), "Void"); }

// tisp utility functions
#[test]
fn test_hash_deterministic() {
    assert_eq!(hash("foo"), hash("foo"));
    assert_ne!(hash("foo"), hash("bar"));
}

#[test]
fn test_frac_reduce() {
    let mut n = 4;
    let mut d = 8;
    frac_reduce(&mut n, &mut d);
    assert_eq!(n, 1);
    assert_eq!(d, 2);
}

#[test]
fn test_frac_reduce_neg() {
    let mut n = -6;
    let mut d = 3;
    frac_reduce(&mut n, &mut d);
    assert_eq!(n, -2);
    assert_eq!(d, 1);
}

#[test]
fn test_mk_int() {
    let v = mk_int(42);
    assert_eq!(v.t, TspType::TspInt);
    assert_eq!(num_of(&v) as i32, 42);
    assert_eq!(den_of(&v) as i32, 1);
}

#[test]
fn test_mk_rat_reduces() {
    let v = mk_rat(4, 8).unwrap();
    assert_eq!(v.t, TspType::TspRatio);
    assert_eq!(num_of(&v) as i32, 1);
    assert_eq!(den_of(&v) as i32, 2);
}

#[test]
fn test_mk_rat_simplifies_to_int() {
    let v = mk_rat(6, 3).unwrap();
    assert_eq!(v.t, TspType::TspInt);
    assert_eq!(num_of(&v) as i32, 2);
}

#[test]
fn test_mk_rat_neg_den() {
    let v = mk_rat(1, -2).unwrap();
    assert_eq!(num_of(&v) as i32, -1);
    assert_eq!(den_of(&v) as i32, 2);
}

#[test]
fn test_mk_nil() {
    let v = mk_nil_val();
    assert_eq!(v.t, TspType::TspNil);
    assert!(nilp(&v));
}

#[test]
fn test_mk_pair() {
    let v = mk_pair_val(mk_int(1), mk_int(2));
    assert_eq!(v.t, TspType::TspPair);
    assert_eq!(num_of(car_ref(&v)) as i32, 1);
    assert_eq!(num_of(cdr_ref(&v)) as i32, 2);
}

#[test]
fn test_tsp_lstlen() {
    let lst = mk_pair_val(mk_int(1), mk_pair_val(mk_int(2), mk_pair_val(mk_int(3), mk_nil_val())));
    assert_eq!(tsp_lstlen(&lst), 3);
}

#[test]
fn test_tsp_lstlen_improper() {
    let lst = mk_pair_val(mk_int(1), mk_pair_val(mk_int(2), mk_int(3)));
    assert_eq!(tsp_lstlen(&lst), -3);
}

#[test]
fn test_vals_eq_ints() {
    assert!(vals_eq(&mk_int(5), &mk_int(5)));
    assert!(!vals_eq(&mk_int(5), &mk_int(6)));
}

#[test]
fn test_vals_eq_pairs() {
    let a = mk_pair_val(mk_int(1), mk_int(2));
    let b = mk_pair_val(mk_int(1), mk_int(2));
    let c = mk_pair_val(mk_int(1), mk_int(3));
    assert!(vals_eq(&a, &b));
    assert!(!vals_eq(&a, &c));
}

#[test]
fn test_isnum() {
    assert!(isnum("123"));
    assert!(isnum("-5"));
    assert!(isnum("+3"));
    assert!(isnum(".5"));
    assert!(isnum("-.3"));
    assert!(!isnum("abc"));
    assert!(!isnum(""));
}

#[test]
fn test_is_sym() {
    assert!(is_sym('a'));
    assert!(is_sym('Z'));
    assert!(is_sym('0'));
    assert!(is_sym('_'));
    assert!(is_sym('?'));
    assert!(!is_sym('('));
    assert!(!is_sym(' '));
}

#[test]
fn test_tsp_type_str() {
    assert_eq!(tsp_type_str(TspType::TspInt), "Int");
    assert_eq!(tsp_type_str(TspType::TspDec), "Dec");
    assert_eq!(tsp_type_str(TspType::TspNil), "Nil");
    assert_eq!(tsp_type_str(TspType::TspNone), "Void");
    assert_eq!(tsp_type_str(TspType::TspStr), "Str");
    assert_eq!(tsp_type_str(TspType::TspSym), "Sym");
    assert_eq!(tsp_type_str(TspType::TspPair), "Pair");
    assert_eq!(tsp_type_str(TspType::TspRec), "Rec");
}

#[test]
fn test_rec_add_get() {
    let mut rec = rec_new(8, None);
    rec_add(&mut rec, "x", mk_int(42));
    let v = rec_get(&rec, "x").unwrap();
    assert_eq!(num_of(&v) as i32, 42);
    assert!(rec_get(&rec, "y").is_none());
}

#[test]
fn test_env_init() {
    let st = tisp_env_init(64);
    let t = rec_get(&st.env, "True").unwrap();
    assert_eq!(t.t, TspType::TspSym);
    let n = rec_get(&st.env, "Nil").unwrap();
    assert_eq!(n.t, TspType::TspNil);
    let v = rec_get(&st.env, "Void").unwrap();
    assert_eq!(v.t, TspType::TspNone);
}

#[test]
fn test_tisp_print_int() {
    let mut buf = Vec::new();
    tisp_print(&mut buf, &mk_int(42));
    assert_eq!(String::from_utf8(buf).unwrap(), "42");
}

#[test]
fn test_tisp_print_nil() {
    let mut buf = Vec::new();
    tisp_print(&mut buf, &mk_nil_val());
    assert_eq!(String::from_utf8(buf).unwrap(), "Nil");
}

#[test]
fn test_tisp_print_pair() {
    let v = mk_pair_val(mk_int(1), mk_pair_val(mk_int(2), mk_nil_val()));
    let mut buf = Vec::new();
    tisp_print(&mut buf, &v);
    assert_eq!(String::from_utf8(buf).unwrap(), "(1 2)");
}

#[test]
fn test_tisp_print_dotted() {
    let v = mk_pair_val(mk_int(1), mk_int(2));
    let mut buf = Vec::new();
    tisp_print(&mut buf, &v);
    assert_eq!(String::from_utf8(buf).unwrap(), "(1 . 2)");
}

fn main() {}
