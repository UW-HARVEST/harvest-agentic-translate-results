use tisp_proj::tisp::*;

/// Helper: init a full tisp environment with core+math+string libs loaded
fn setup() -> Tsp {
    let mut st = tisp_env_init(1024);
    tib_env_core(&mut st);
    tib_env_math(&mut st);
    tib_env_string(&mut st);
    st
}

/// Helper: evaluate a tisp expression string and return printed output
fn eval_str(st: &mut Tsp, input: &str) -> String {
    st.file = input.to_string();
    st.filec = 0;
    let v = tisp_read(st).expect(&format!("read failed for: {}", input));
    let mut env = clone_rec(&st.env);
    let v = tisp_eval_with_env(st, &mut env, v).expect(&format!("eval failed for: {}", input));
    st.env = env;
    let mut buf: Vec<u8> = Vec::new();
    tisp_print(&mut buf, &v);
    String::from_utf8(buf).unwrap()
}

// ---- Self-evaluating values ----

#[test]
fn test_self_eval_integers() {
    let mut st = setup();
    assert_eq!(eval_str(&mut st, "1"), "1");
    assert_eq!(eval_str(&mut st, "2"), "2");
    assert_eq!(eval_str(&mut st, "0"), "0");
    assert_eq!(eval_str(&mut st, "30"), "30");
    assert_eq!(eval_str(&mut st, "12"), "12");
    assert_eq!(eval_str(&mut st, "-4"), "-4");
    assert_eq!(eval_str(&mut st, "-083"), "-83");
    assert_eq!(eval_str(&mut st, "-0"), "0");
    assert_eq!(eval_str(&mut st, "+4"), "4");
    assert_eq!(eval_str(&mut st, "+123"), "123");
    assert_eq!(eval_str(&mut st, "08"), "8");
}

#[test]
fn test_self_eval_decimals() {
    let mut st = setup();
    assert_eq!(eval_str(&mut st, "12.0"), "12.0");
    assert_eq!(eval_str(&mut st, "+12.34"), "12.34");
    assert_eq!(eval_str(&mut st, ".34"), "0.34");
    assert_eq!(eval_str(&mut st, "2."), "2.0");
    assert_eq!(eval_str(&mut st, "-.05"), "-0.05");
    assert_eq!(eval_str(&mut st, "-.0"), "-0.0");
}

#[test]
fn test_self_eval_scientific() {
    let mut st = setup();
    assert_eq!(eval_str(&mut st, "1e0"), "1");
    assert_eq!(eval_str(&mut st, "1E+0"), "1");
    assert_eq!(eval_str(&mut st, "1e-0"), "1");
    assert_eq!(eval_str(&mut st, "1E4"), "10000");
    assert_eq!(eval_str(&mut st, ".1e-4"), "1e-05");
    assert_eq!(eval_str(&mut st, "-5.0e006"), "-5000000.0");
    assert_eq!(eval_str(&mut st, "-5.E+16"), "-5e+16");
    assert_eq!(eval_str(&mut st, "-1.e6"), "-1000000.0");
}

#[test]
fn test_self_eval_ratios() {
    let mut st = setup();
    assert_eq!(eval_str(&mut st, "3/4"), "3/4");
    assert_eq!(eval_str(&mut st, "4/3"), "4/3");
    assert_eq!(eval_str(&mut st, "+1/2"), "1/2");
    assert_eq!(eval_str(&mut st, "2/1"), "2");
    assert_eq!(eval_str(&mut st, "8/+1"), "8");
    assert_eq!(eval_str(&mut st, "8/4"), "2");
    assert_eq!(eval_str(&mut st, "4/8"), "1/2");
    assert_eq!(eval_str(&mut st, "02384/7238"), "1192/3619");
    assert_eq!(eval_str(&mut st, "-1/2"), "-1/2");
    assert_eq!(eval_str(&mut st, "1/-2"), "-1/2");
    assert_eq!(eval_str(&mut st, "-6/3"), "-2");
    assert_eq!(eval_str(&mut st, "-6/-3"), "2");
}

#[test]
fn test_self_eval_strings_and_special() {
    let mut st = setup();
    assert_eq!(eval_str(&mut st, "\"foo\""), "foo");
    assert_eq!(eval_str(&mut st, "\"foo bar\""), "foo bar");
    assert_eq!(eval_str(&mut st, "True"), "True");
    assert_eq!(eval_str(&mut st, "()"), "Nil");
    assert_eq!(eval_str(&mut st, "Nil"), "Nil");
    assert_eq!(eval_str(&mut st, "Void"), "Void");
}

// ---- Comments and whitespace ----

#[test]
fn test_comments() {
    let mut st = setup();
    assert_eq!(eval_str(&mut st, "; comment"), "Void");
    assert_eq!(eval_str(&mut st, "; (+ 1 1)"), "Void");
    assert_eq!(eval_str(&mut st, "(+ 1 ; more comments\n1)"), "2");
}

#[test]
fn test_whitespace() {
    let mut st = setup();
    assert_eq!(eval_str(&mut st, "\t \n  \n\n\t\n \t\n"), "Void");
    assert_eq!(eval_str(&mut st, "\t  \t(+   \t\t5 \n \n5  \n\t)"), "10");
}

// ---- Read/print round-trip for types ----

#[test]
fn test_mk_int() {
    let v = mk_int(42);
    assert_eq!(v.t, TspType::TspInt);
    assert_eq!(vnum(&v) as i32, 42);
    assert_eq!(vden(&v) as i32, 1);
}

#[test]
fn test_mk_dec() {
    let v = mk_dec(3.14).unwrap();
    assert_eq!(v.t, TspType::TspDec);
    assert_eq!(vnum(&v), 3.14);
    assert_eq!(vden(&v), 1.0);
}

#[test]
fn test_mk_rat() {
    let v = mk_rat(3, 4).unwrap();
    assert_eq!(v.t, TspType::TspRatio);
    assert_eq!(vnum(&v) as i32, 3);
    assert_eq!(vden(&v) as i32, 4);

    // Simplification
    let v = mk_rat(8, 4).unwrap();
    assert_eq!(v.t, TspType::TspInt);
    assert_eq!(vnum(&v) as i32, 2);

    // Negative denominator
    let v = mk_rat(1, -2).unwrap();
    assert_eq!(v.t, TspType::TspRatio);
    assert_eq!(vnum(&v) as i32, -1);
    assert_eq!(vden(&v) as i32, 2);

    // Division by zero
    assert!(mk_rat(1, 0).is_none());
}

#[test]
fn test_mk_pair() {
    let a = mk_int(1);
    let b = mk_int(2);
    let p = mk_pair(a, b).unwrap();
    assert_eq!(p.t, TspType::TspPair);
    assert_eq!(vnum(car(&p)) as i32, 1);
    assert_eq!(vnum(cdr(&p)) as i32, 2);
}

#[test]
fn test_mk_str_sym() {
    let mut st = setup();
    let s = mk_str(&mut st, "hello").unwrap();
    assert_eq!(s.t, TspType::TspStr);
    assert_eq!(vs(&s), "hello");

    let sym = mk_sym(&mut st, "foo").unwrap();
    assert_eq!(sym.t, TspType::TspSym);
    assert_eq!(vs(&sym), "foo");
}

// ---- Type string ----

#[test]
fn test_tsp_type_str() {
    assert_eq!(tsp_type_str(TspType::TspNone), "Void");
    assert_eq!(tsp_type_str(TspType::TspNil), "Nil");
    assert_eq!(tsp_type_str(TspType::TspInt), "Int");
    assert_eq!(tsp_type_str(TspType::TspDec), "Dec");
    assert_eq!(tsp_type_str(TspType::TspRatio), "Ratio");
    assert_eq!(tsp_type_str(TspType::TspStr), "Str");
    assert_eq!(tsp_type_str(TspType::TspSym), "Sym");
    assert_eq!(tsp_type_str(TspType::TspPrim), "Prim");
    assert_eq!(tsp_type_str(TspType::TspForm), "Form");
    assert_eq!(tsp_type_str(TspType::TspFunc), "Func");
    assert_eq!(tsp_type_str(TspType::TspMacro), "Macro");
    assert_eq!(tsp_type_str(TspType::TspPair), "Pair");
    assert_eq!(tsp_type_str(TspType::TspRec), "Rec");
}

// ---- List length ----

#[test]
fn test_tsp_lstlen() {
    let mut st = setup();
    // Proper list (1 2 3)
    let nil = clone_val(&st.nil);
    let l = mk_pair(mk_int(1), mk_pair(mk_int(2), mk_pair(mk_int(3), nil).unwrap()).unwrap()).unwrap();
    assert_eq!(tsp_lstlen(&l), 3);

    // Empty list
    assert_eq!(tsp_lstlen(&st.nil), 0);

    // Improper list (1 . 2)
    let imp = mk_pair(mk_int(1), mk_int(2)).unwrap();
    assert_eq!(tsp_lstlen(&imp), -2);
}

// ---- vals_eq ----

#[test]
fn test_vals_eq() {
    assert!(vals_eq(&mk_int(5), &mk_int(5)));
    assert!(!vals_eq(&mk_int(5), &mk_int(6)));
    // Int and ratio with same value
    assert!(vals_eq(&mk_int(2), &mk_rat(4, 2).unwrap()));
    // Pairs
    let a = mk_pair(mk_int(1), mk_int(2)).unwrap();
    let b = mk_pair(mk_int(1), mk_int(2)).unwrap();
    assert!(vals_eq(&a, &b));
    let c = mk_pair(mk_int(1), mk_int(3)).unwrap();
    assert!(!vals_eq(&a, &c));
}

// ---- frac_reduce ----

#[test]
fn test_frac_reduce() {
    let (mut n, mut d) = (8, 4);
    frac_reduce(&mut n, &mut d);
    assert_eq!(n, 2);
    assert_eq!(d, 1);

    let (mut n, mut d) = (6, 9);
    frac_reduce(&mut n, &mut d);
    assert_eq!(n, 2);
    assert_eq!(d, 3);

    let (mut n, mut d) = (-6, 3);
    frac_reduce(&mut n, &mut d);
    assert_eq!(n, -2);
    assert_eq!(d, 1);
}

// ---- hash ----

#[test]
fn test_hash_deterministic() {
    assert_eq!(hash("foo"), hash("foo"));
    assert_ne!(hash("foo"), hash("bar"));
}

// ---- rec operations ----

#[test]
fn test_rec_add_get() {
    let mut rec = rec_new(8, None);
    rec_add(&mut rec, "x", mk_int(42));
    let v = rec_get(&rec, "x").unwrap();
    assert_eq!(v.t, TspType::TspInt);
    assert_eq!(vnum(&v) as i32, 42);
    assert!(rec_get(&rec, "y").is_none());
}

// ---- isnum ----

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

// ---- is_sym, is_op ----

#[test]
fn test_is_sym_is_op() {
    assert!(is_sym('a'));
    assert!(is_sym('Z'));
    assert!(is_sym('0'));
    assert!(is_sym('_'));
    assert!(is_sym('!'));
    assert!(is_sym('?'));
    assert!(!is_sym('('));
    assert!(!is_sym(')'));

    assert!(is_op('+'));
    assert!(is_op('-'));
    assert!(is_op('*'));
    assert!(is_op('/'));
    assert!(is_op('='));
    assert!(is_op('<'));
    assert!(is_op('>'));
    assert!(!is_op('a'));
}

// ---- tisp_print ----

#[test]
fn test_print_int() {
    let v = mk_int(42);
    let mut buf: Vec<u8> = Vec::new();
    tisp_print(&mut buf, &v);
    assert_eq!(String::from_utf8(buf).unwrap(), "42");
}

#[test]
fn test_print_dec() {
    let v = mk_dec(3.14).unwrap();
    let mut buf: Vec<u8> = Vec::new();
    tisp_print(&mut buf, &v);
    assert_eq!(String::from_utf8(buf).unwrap(), "3.14");
}

#[test]
fn test_print_ratio() {
    let v = mk_rat(3, 4).unwrap();
    let mut buf: Vec<u8> = Vec::new();
    tisp_print(&mut buf, &v);
    assert_eq!(String::from_utf8(buf).unwrap(), "3/4");
}

#[test]
fn test_print_nil_none() {
    let mut st = setup();
    let mut buf: Vec<u8> = Vec::new();
    tisp_print(&mut buf, &st.nil);
    assert_eq!(String::from_utf8(buf).unwrap(), "Nil");

    let mut buf: Vec<u8> = Vec::new();
    tisp_print(&mut buf, &st.none);
    assert_eq!(String::from_utf8(buf).unwrap(), "Void");
}

#[test]
fn test_print_pair() {
    let p = mk_pair(mk_int(1), mk_int(2)).unwrap();
    let mut buf: Vec<u8> = Vec::new();
    tisp_print(&mut buf, &p);
    assert_eq!(String::from_utf8(buf).unwrap(), "(1 . 2)");
}

#[test]
fn test_print_list() {
    let mut st = setup();
    let nil = clone_val(&st.nil);
    let l = mk_pair(mk_int(1), mk_pair(mk_int(2), mk_pair(mk_int(3), nil).unwrap()).unwrap()).unwrap();
    let mut buf: Vec<u8> = Vec::new();
    tisp_print(&mut buf, &l);
    assert_eq!(String::from_utf8(buf).unwrap(), "(1 2 3)");
}

// ---- tisp_env_init ----

#[test]
fn test_env_init() {
    let st = tisp_env_init(64);
    assert_eq!(st.nil.t, TspType::TspNil);
    assert_eq!(st.none.t, TspType::TspNone);
    assert_eq!(st.t.t, TspType::TspSym);
    assert_eq!(vs(&st.t), "True");
    // Check built-in env vars
    let true_val = rec_get(&st.env, "True").unwrap();
    assert_eq!(true_val.t, TspType::TspSym);
    let nil_val = rec_get(&st.env, "Nil").unwrap();
    assert_eq!(nil_val.t, TspType::TspNil);
    let void_val = rec_get(&st.env, "Void").unwrap();
    assert_eq!(void_val.t, TspType::TspNone);
    let ver = rec_get(&st.env, "version").unwrap();
    assert_eq!(ver.t, TspType::TspStr);
    assert_eq!(vs(&ver), "0.1");
}

// ---- esc_char ----

#[test]
fn test_esc_char() {
    assert_eq!(esc_char('n'), '\n');
    assert_eq!(esc_char('r'), '\r');
    assert_eq!(esc_char('t'), '\t');
    assert_eq!(esc_char('\n'), ' ');
    assert_eq!(esc_char('\\'), '\\');
    assert_eq!(esc_char('"'), '"');
    assert_eq!(esc_char('a'), 'a');
}

// ---- esc_str ----

#[test]
fn test_esc_str() {
    assert_eq!(esc_str("hello", 5, 0), "hello");
    assert_eq!(esc_str("he\\nllo", 6, 1), "he\nllo");
    assert_eq!(esc_str("he\\nllo", 7, 0), "he\\nllo");
}

// ---- clone_val ----

#[test]
fn test_clone_val() {
    let v = mk_int(99);
    let c = clone_val(&v);
    assert_eq!(c.t, TspType::TspInt);
    assert_eq!(vnum(&c) as i32, 99);
}

fn main() {}
