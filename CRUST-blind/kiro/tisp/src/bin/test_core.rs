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

// Eq
#[test] fn test_eq_empty()    { let mut st = setup(); assert_eq!(eval_str(&mut st, "(=)"), "True"); }
#[test] fn test_eq_one()      { let mut st = setup(); assert_eq!(eval_str(&mut st, "(= 1)"), "True"); }
#[test] fn test_eq_str()      { let mut st = setup(); assert_eq!(eval_str(&mut st, "(= \"foo\")"), "True"); }
#[test] fn test_eq_11()       { let mut st = setup(); assert_eq!(eval_str(&mut st, "(= 1 1)"), "True"); }
#[test] fn test_eq_many1()    { let mut st = setup(); assert_eq!(eval_str(&mut st, "(= 1 1 1 1 1 1)"), "True"); }
#[test] fn test_eq_12()       { let mut st = setup(); assert_eq!(eval_str(&mut st, "(= 1 2)"), "Nil"); }
#[test] fn test_eq_mid_diff() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(= 1 1 2 1 1 1)"), "Nil"); }
#[test] fn test_eq_end_diff() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(= 1 1 1 1 1 2)"), "Nil"); }
#[test] fn test_eq_start_diff(){ let mut st = setup(); assert_eq!(eval_str(&mut st, "(= 2 1 1 1 1 1)"), "Nil"); }
#[test] fn test_eq_rat()      { let mut st = setup(); assert_eq!(eval_str(&mut st, "(= 4/5 4/5)"), "True"); }
#[test] fn test_eq_rat_int()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(= 2/1 2)"), "True"); }
#[test] fn test_eq_rat_reduce(){ let mut st = setup(); assert_eq!(eval_str(&mut st, "(= 2/4 1/2)"), "True"); }
#[test] fn test_eq_rat_many() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(= 2/4 1/2 4/8 3/6)"), "True"); }
#[test] fn test_eq_rat_diff() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(= 1/2 4/5)"), "Nil"); }
#[test] fn test_eq_rat_diff2(){ let mut st = setup(); assert_eq!(eval_str(&mut st, "(= 5/4 4/5)"), "Nil"); }
#[test] fn test_eq_int_rat()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(= 3 3/2)"), "Nil"); }
#[test] fn test_eq_int_rat_many(){ let mut st = setup(); assert_eq!(eval_str(&mut st, "(= 3 3/2 3 3 3)"), "Nil"); }
#[test] fn test_eq_expr()     { let mut st = setup(); assert_eq!(eval_str(&mut st, "(= (+ 1 1) (+ 2 0))"), "True"); }
#[test] fn test_eq_str_eq()   { let mut st = setup(); assert_eq!(eval_str(&mut st, "(= \"foo\" \"foo\")"), "True"); }
#[test] fn test_eq_str_diff() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(= \"foo\" \"bar\")"), "Nil"); }
#[test] fn test_eq_str_sym()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(= \"foo\" 'foo)"), "Nil"); }
#[test] fn test_eq_sym()      { let mut st = setup(); assert_eq!(eval_str(&mut st, "(= 'bar 'bar)"), "True"); }
#[test] fn test_eq_str_many() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(= \"foo\" \"foo\" \"foo\" \"foo\" \"foo\")"), "True"); }
#[test] fn test_eq_str_many_diff(){ let mut st = setup(); assert_eq!(eval_str(&mut st, "(= \"foo\" \"bar\" \"foo\" \"foo\" \"foo\")"), "Nil"); }
#[test] fn test_eq_str_int()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(= \"foo\" 3)"), "Nil"); }
#[test] fn test_eq_str_int_mid(){ let mut st = setup(); assert_eq!(eval_str(&mut st, "(= \"foo\" \"foo\" 4 \"foo\" \"foo\")"), "Nil"); }
#[test] fn test_eq_str_case() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(= \"foo\" \"FOO\")"), "Nil"); }
#[test] fn test_eq_true()     { let mut st = setup(); assert_eq!(eval_str(&mut st, "(= True True)"), "True"); }
#[test] fn test_eq_car()      { let mut st = setup(); assert_eq!(eval_str(&mut st, "(= car car)"), "True"); }
#[test] fn test_eq_car_cdr()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(= car cdr)"), "Nil"); }
#[test] fn test_eq_quote3()   { let mut st = setup(); assert_eq!(eval_str(&mut st, "(= quote quote quote)"), "True"); }
#[test] fn test_eq_list()     { let mut st = setup(); assert_eq!(eval_str(&mut st, "(= '(1 2 3) (list 1 2 3))"), "True"); }
#[test] fn test_eq_sym_list() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(= '(a b c) '(a b c))"), "True"); }
#[test] fn test_eq_sym_list_diff(){ let mut st = setup(); assert_eq!(eval_str(&mut st, "(= '(a b c) '(a b d))"), "Nil"); }
#[test] fn test_eq_list_len() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(= '(1 2 3) '(1 2))"), "Nil"); }
#[test] fn test_eq_list_len2(){ let mut st = setup(); assert_eq!(eval_str(&mut st, "(= '(1 2 3) '(1))"), "Nil"); }
#[test] fn test_eq_nested()   { let mut st = setup(); assert_eq!(eval_str(&mut st, "(= '((1 2) 3 4) '((1 2) 3 4))"), "True"); }
#[test] fn test_eq_nested_diff(){ let mut st = setup(); assert_eq!(eval_str(&mut st, "(= '((1 b) 3 4) '((1 2) 3 4))"), "Nil"); }
#[test] fn test_eq_func()     { let mut st = setup(); assert_eq!(eval_str(&mut st, "(= (Func (it) it) @it)"), "True"); }
#[test] fn test_eq_func_diff(){ let mut st = setup(); assert_eq!(eval_str(&mut st, "(= @it (Func (x) x))"), "Nil"); }
#[test] fn test_neq_empty()   { let mut st = setup(); assert_eq!(eval_str(&mut st, "(/=)"), "Nil"); }
#[test] fn test_neq_one()     { let mut st = setup(); assert_eq!(eval_str(&mut st, "(/= 'a)"), "Nil"); }
#[test] fn test_neq_pair()    { let mut st = setup(); assert_eq!(eval_str(&mut st, "(/= '(1 . 2) (list* 1 2))"), "Nil"); }
#[test] fn test_neq_diff()    { let mut st = setup(); assert_eq!(eval_str(&mut st, "(/= 1 2)"), "True"); }
#[test] fn test_neq_mid()     { let mut st = setup(); assert_eq!(eval_str(&mut st, "(/= 1 1 2 1 1 1)"), "True"); }
#[test] fn test_neq_str()     { let mut st = setup(); assert_eq!(eval_str(&mut st, "(/= \"foo\" \"bar\")"), "True"); }
#[test] fn test_neq_sym_many(){ let mut st = setup(); assert_eq!(eval_str(&mut st, "(/= 'greg 'greg 'greg 'greg)"), "Nil"); }

// Def
#[test] fn test_def_basic() {
    let mut st = setup();
    assert_eq!(eval_str(&mut st, "(def foo 4)"), "Void");
    assert_eq!(eval_str(&mut st, "foo"), "4");
}
#[test] fn test_def_var() {
    let mut st = setup();
    eval_str(&mut st, "(def foo 4)");
    assert_eq!(eval_str(&mut st, "(def bar foo)"), "Void");
    assert_eq!(eval_str(&mut st, "bar"), "4");
}
#[test] fn test_def_redef() {
    let mut st = setup();
    eval_str(&mut st, "(def foo 4)");
    eval_str(&mut st, "(def bar foo)");
    assert_eq!(eval_str(&mut st, "(def foo (+ foo bar))"), "Void");
    assert_eq!(eval_str(&mut st, "foo"), "8");
}
#[test] fn test_def_prim() {
    let mut st = setup();
    eval_str(&mut st, "(def foo 4)");
    eval_str(&mut st, "(def bar foo)");
    eval_str(&mut st, "(def foo (+ foo bar))");
    assert_eq!(eval_str(&mut st, "(def add +)"), "Void");
    assert_eq!(eval_str(&mut st, "(add foo bar)"), "12");
}
#[test] fn test_def_func() {
    let mut st = setup();
    eval_str(&mut st, "(def foo 8)");
    eval_str(&mut st, "(def add +)");
    assert_eq!(eval_str(&mut st, "(def (one x) (add x 1))"), "Void");
    assert_eq!(eval_str(&mut st, "(one foo)"), "9");
}
#[test] fn test_def_func_body() {
    let mut st = setup();
    assert_eq!(eval_str(&mut st, "(def (more x) (def term 3) (+ x term))"), "Void");
    assert_eq!(eval_str(&mut st, "(more 8)"), "11");
}
#[test] fn test_def_func_multi_body() {
    let mut st = setup();
    assert_eq!(eval_str(&mut st, "(def (add2 x) (+ x 1) (+ x 2))"), "Void");
    assert_eq!(eval_str(&mut st, "(add2 2)"), "4");
}

// Defined?
#[test] fn test_definedp1() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(defined? invalid-var)"), "Nil"); }
#[test] fn test_definedp2() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(defined? defined?)"), "True"); }
#[test] fn test_definedp3() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(defined? car)"), "True"); }
#[test] fn test_definedp4() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(defined? when)"), "True"); }
#[test] fn test_definedp5() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(defined? apply)"), "True"); }

// Func
#[test] fn test_func1() { let mut st = setup(); assert_eq!(eval_str(&mut st, "((Func (x) x) 3)"), "3"); }
#[test] fn test_func2() { let mut st = setup(); assert_eq!(eval_str(&mut st, "((Func (x) x) (+ 1 2))"), "3"); }
#[test] fn test_func3() { let mut st = setup(); assert_eq!(eval_str(&mut st, "((Func (x) (+ x 1)) 8)"), "9"); }
#[test] fn test_func4() { let mut st = setup(); assert_eq!(eval_str(&mut st, "((Func (a b) (+ a b)) 2 2)"), "4"); }
#[test] fn test_func5() { let mut st = setup(); assert_eq!(eval_str(&mut st, "((Func () 5))"), "5"); }

fn main() {}
