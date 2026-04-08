use tisp_proj::tisp::*;
use tisp_proj::core;
use tisp_proj::math;
use tisp_proj::string;

fn setup() -> Tsp {
    let mut st = tisp_env_init(1024);
    core::tib_env_core(&mut st);
    math::tib_env_math(&mut st);
    string::tib_env_string(&mut st);
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

// Self-evaluating
#[test] fn test_self_int1()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "1"), "1"); }
#[test] fn test_self_int2()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "2"), "2"); }
#[test] fn test_self_int0()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "0"), "0"); }
#[test] fn test_self_int30() { let mut st = setup(); assert_eq!(eval_str(&mut st, "30"), "30"); }
#[test] fn test_self_int12() { let mut st = setup(); assert_eq!(eval_str(&mut st, "12"), "12"); }
#[test] fn test_self_neg4()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "-4"), "-4"); }
#[test] fn test_self_neg083(){ let mut st = setup(); assert_eq!(eval_str(&mut st, "-083"), "-83"); }
#[test] fn test_self_neg0()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "-0"), "0"); }
#[test] fn test_self_pos4()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "+4"), "4"); }
#[test] fn test_self_pos123(){ let mut st = setup(); assert_eq!(eval_str(&mut st, "+123"), "123"); }
#[test] fn test_self_dec12() { let mut st = setup(); assert_eq!(eval_str(&mut st, "12.0"), "12.0"); }
#[test] fn test_self_08()    { let mut st = setup(); assert_eq!(eval_str(&mut st, "08"), "8"); }
#[test] fn test_self_posdec(){ let mut st = setup(); assert_eq!(eval_str(&mut st, "+12.34"), "12.34"); }
#[test] fn test_self_dotdec(){ let mut st = setup(); assert_eq!(eval_str(&mut st, ".34"), "0.34"); }
#[test] fn test_self_2dot()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "2."), "2.0"); }
#[test] fn test_self_1e0()   { let mut st = setup(); assert_eq!(eval_str(&mut st, "1e0"), "1"); }
#[test] fn test_self_1epos0(){ let mut st = setup(); assert_eq!(eval_str(&mut st, "1E+0"), "1"); }
#[test] fn test_self_1eneg0(){ let mut st = setup(); assert_eq!(eval_str(&mut st, "1e-0"), "1"); }
#[test] fn test_self_1e4()   { let mut st = setup(); assert_eq!(eval_str(&mut st, "1E4"), "10000"); }
#[test] fn test_self_dot1eneg4() { let mut st = setup(); assert_eq!(eval_str(&mut st, ".1e-4"), "1e-05"); }
#[test] fn test_self_neg5e6()    { let mut st = setup(); assert_eq!(eval_str(&mut st, "-5.0e006"), "-5000000.0"); }
#[test] fn test_self_neg5epos16(){ let mut st = setup(); assert_eq!(eval_str(&mut st, "-5.E+16"), "-5e+16"); }
#[test] fn test_self_negdot05()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "-.05"), "-0.05"); }
#[test] fn test_self_negdot0()   { let mut st = setup(); assert_eq!(eval_str(&mut st, "-.0"), "-0.0"); }
#[test] fn test_self_neg1e6()    { let mut st = setup(); assert_eq!(eval_str(&mut st, "-1.e6"), "-1000000.0"); }
#[test] fn test_self_rat34()     { let mut st = setup(); assert_eq!(eval_str(&mut st, "3/4"), "3/4"); }
#[test] fn test_self_rat43()     { let mut st = setup(); assert_eq!(eval_str(&mut st, "4/3"), "4/3"); }
#[test] fn test_self_ratpos12()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "+1/2"), "1/2"); }
#[test] fn test_self_rat21()     { let mut st = setup(); assert_eq!(eval_str(&mut st, "2/1"), "2"); }
#[test] fn test_self_rat8pos1()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "8/+1"), "8"); }
#[test] fn test_self_rat84()     { let mut st = setup(); assert_eq!(eval_str(&mut st, "8/4"), "2"); }
#[test] fn test_self_rat48()     { let mut st = setup(); assert_eq!(eval_str(&mut st, "4/8"), "1/2"); }
#[test] fn test_self_ratbig()    { let mut st = setup(); assert_eq!(eval_str(&mut st, "02384/7238"), "1192/3619"); }
#[test] fn test_self_ratneg12()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "-1/2"), "-1/2"); }
#[test] fn test_self_rat1neg2()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "1/-2"), "-1/2"); }
#[test] fn test_self_ratneg63()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "-6/3"), "-2"); }
#[test] fn test_self_ratnegneg() { let mut st = setup(); assert_eq!(eval_str(&mut st, "-6/-3"), "2"); }
#[test] fn test_self_str_foo()   { let mut st = setup(); assert_eq!(eval_str(&mut st, "\"foo\""), "foo"); }
#[test] fn test_self_str_foobar(){ let mut st = setup(); assert_eq!(eval_str(&mut st, "\"foo bar\""), "foo bar"); }
#[test] fn test_self_true()      { let mut st = setup(); assert_eq!(eval_str(&mut st, "True"), "True"); }
#[test] fn test_self_nil_parens(){ let mut st = setup(); assert_eq!(eval_str(&mut st, "()"), "Nil"); }
#[test] fn test_self_nil()       { let mut st = setup(); assert_eq!(eval_str(&mut st, "Nil"), "Nil"); }
#[test] fn test_self_void()      { let mut st = setup(); assert_eq!(eval_str(&mut st, "Void"), "Void"); }

// Comments
#[test] fn test_comment1() { let mut st = setup(); assert_eq!(eval_str(&mut st, "; commment"), "Void"); }
#[test] fn test_comment2() { let mut st = setup(); assert_eq!(eval_str(&mut st, "; (+ 1 1)"), "Void"); }
#[test] fn test_comment3() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(+ 1 ; more comments\n1)"), "2"); }

// Whitespace
#[test] fn test_ws1() { let mut st = setup(); assert_eq!(eval_str(&mut st, "\t \n  \n\n\t\n \t\n"), "Void"); }
#[test] fn test_ws2() { let mut st = setup(); assert_eq!(eval_str(&mut st, "\t  \t(+   \t\t5 \n \n5  \n\t)"), "10"); }

// Quote
#[test] fn test_quote1() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(quote 1)"), "1"); }
#[test] fn test_quote2() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(quote 9234)"), "9234"); }
#[test] fn test_quote3() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(quote \"foo\")"), "foo"); }
#[test] fn test_quote4() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(quote bar)"), "bar"); }
#[test] fn test_quote5() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(quote (1 2 3 4))"), "(1 2 3 4)"); }
#[test] fn test_quote6() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(quote (quote 1))"), "(quote 1)"); }
#[test] fn test_quote7() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(quote (+ 2 2))"), "(+ 2 2)"); }
#[test] fn test_quote8() { let mut st = setup(); assert_eq!(eval_str(&mut st, "'12"), "12"); }
#[test] fn test_quote9() { let mut st = setup(); assert_eq!(eval_str(&mut st, "'foo"), "foo"); }
#[test] fn test_quote10(){ let mut st = setup(); assert_eq!(eval_str(&mut st, "'(1 2 3 4)"), "(1 2 3 4)"); }
#[test] fn test_quote11(){ let mut st = setup(); assert_eq!(eval_str(&mut st, "'()"), "Nil"); }

// Cons
#[test] fn test_cons1() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(cons 1 2)"), "(1 . 2)"); }
#[test] fn test_cons2() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(cons 1 (cons 2 3))"), "(1 2 . 3)"); }
#[test] fn test_cons3() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(cons 1 (cons 2 (cons 3 4)))"), "(1 2 3 . 4)"); }
#[test] fn test_cons4() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(cons \"foo\" \"bar\")"), "(foo . bar)"); }
#[test] fn test_cons5() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(cons (+ 1 2) 3)"), "(3 . 3)"); }
#[test] fn test_cons6() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(cons (cons 1 2) (cons 3 4))"), "((1 . 2) 3 . 4)"); }

// Cxr
#[test] fn test_cxr1()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(car (cons 1 2))"), "1"); }
#[test] fn test_cxr2()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(cdr (cons 1 2))"), "2"); }
#[test] fn test_cxr3()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(car (quote (1 2 3 4)))"), "1"); }
#[test] fn test_cxr4()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(car (cdr (quote (1 2 3 4))))"), "2"); }
#[test] fn test_cxr5()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(car (cdr (cdr (quote (1 2 3 4)))))"), "3"); }
#[test] fn test_cxr6()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(car (cdr (cdr (cdr (quote (1 2 3 4))))))"), "4"); }
#[test] fn test_cxr7()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(cdr (quote (1 2 3 4)))"), "(2 3 4)"); }
#[test] fn test_cxr8()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(cdr (cdr (quote (1 2 3 4))))"), "(3 4)"); }

// Do
#[test] fn test_do1() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(do (+ 1 2) (+ 2 2))"), "4"); }
#[test] fn test_do2() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(do (+ -4 8) (- 1 2) (* 80 0) (+ 39 -3))"), "36"); }
#[test] fn test_do3() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(do (mod 80 2) (/ 4 2) Void)"), "Void"); }

// Eval
#[test] fn test_eval1() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(eval ''hey)"), "hey"); }
#[test] fn test_eval2() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(eval \"sup\")"), "sup"); }
#[test] fn test_eval3() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(eval (+ 1 2))"), "3"); }
#[test] fn test_eval4() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(eval '(- 4 3))"), "1"); }
#[test] fn test_eval5() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(eval ''(mod 9 3))"), "(mod 9 3)"); }
#[test] fn test_eval6() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(do (def bar '(/ 25 5)) (eval bar))"), "5"); }

// Cond
#[test] fn test_cond1() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(cond)"), "Void"); }
#[test] fn test_cond2() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(cond (True 1))"), "1"); }
#[test] fn test_cond3() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(cond ((= 1 1) 1) ((= 1 2) 2) (True 3))"), "1"); }
#[test] fn test_cond4() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(cond ((= 1 2) 1) ((= 1 1) 2) (True 3))"), "2"); }
#[test] fn test_cond5() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(cond ((= 1 2) 1) ((= 1 3) 2))"), "Void"); }
#[test] fn test_cond6() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(cond ((= 1 2) 1) (\"foo\" 2) (True 3))"), "2"); }
#[test] fn test_cond7() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(cond (() (+ 1 2)))"), "Void"); }

// Eq
#[test] fn test_eq_empty()    { let mut st = setup(); assert_eq!(eval_str(&mut st, "(=)"), "True"); }
#[test] fn test_eq_one()      { let mut st = setup(); assert_eq!(eval_str(&mut st, "(= 1)"), "True"); }
#[test] fn test_eq_11()       { let mut st = setup(); assert_eq!(eval_str(&mut st, "(= 1 1)"), "True"); }
#[test] fn test_eq_many1()    { let mut st = setup(); assert_eq!(eval_str(&mut st, "(= 1 1 1 1 1 1)"), "True"); }
#[test] fn test_eq_12()       { let mut st = setup(); assert_eq!(eval_str(&mut st, "(= 1 2)"), "Nil"); }
#[test] fn test_eq_mid_diff() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(= 1 1 2 1 1 1)"), "Nil"); }
#[test] fn test_eq_rat()      { let mut st = setup(); assert_eq!(eval_str(&mut st, "(= 4/5 4/5)"), "True"); }
#[test] fn test_eq_rat_int()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(= 2/1 2)"), "True"); }
#[test] fn test_eq_rat_reduce(){ let mut st = setup(); assert_eq!(eval_str(&mut st, "(= 2/4 1/2)"), "True"); }
#[test] fn test_eq_expr()     { let mut st = setup(); assert_eq!(eval_str(&mut st, "(= (+ 1 1) (+ 2 0))"), "True"); }
#[test] fn test_eq_str_eq()   { let mut st = setup(); assert_eq!(eval_str(&mut st, "(= \"foo\" \"foo\")"), "True"); }
#[test] fn test_eq_str_diff() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(= \"foo\" \"bar\")"), "Nil"); }
#[test] fn test_eq_str_sym()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(= \"foo\" 'foo)"), "Nil"); }
#[test] fn test_eq_sym()      { let mut st = setup(); assert_eq!(eval_str(&mut st, "(= 'bar 'bar)"), "True"); }
#[test] fn test_eq_true()     { let mut st = setup(); assert_eq!(eval_str(&mut st, "(= True True)"), "True"); }
#[test] fn test_eq_car()      { let mut st = setup(); assert_eq!(eval_str(&mut st, "(= car car)"), "True"); }
#[test] fn test_eq_car_cdr()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(= car cdr)"), "Nil"); }
#[test] fn test_eq_list()     { let mut st = setup(); assert_eq!(eval_str(&mut st, "(= '(1 2 3) '(1 2 3))"), "True"); }
#[test] fn test_eq_list_diff(){ let mut st = setup(); assert_eq!(eval_str(&mut st, "(= '(a b c) '(a b d))"), "Nil"); }
#[test] fn test_eq_list_len() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(= '(1 2 3) '(1 2))"), "Nil"); }
#[test] fn test_eq_nested()   { let mut st = setup(); assert_eq!(eval_str(&mut st, "(= '((1 2) 3 4) '((1 2) 3 4))"), "True"); }
#[test] fn test_eq_func()     { let mut st = setup(); assert_eq!(eval_str(&mut st, "(= (Func (it) it) @it)"), "True"); }
#[test] fn test_eq_func_diff(){ let mut st = setup(); assert_eq!(eval_str(&mut st, "(= @it (Func (x) x))"), "Nil"); }

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

// Func
#[test] fn test_func1() { let mut st = setup(); assert_eq!(eval_str(&mut st, "((Func (x) x) 3)"), "3"); }
#[test] fn test_func2() { let mut st = setup(); assert_eq!(eval_str(&mut st, "((Func (x) x) (+ 1 2))"), "3"); }
#[test] fn test_func3() { let mut st = setup(); assert_eq!(eval_str(&mut st, "((Func (x) (+ x 1)) 8)"), "9"); }
#[test] fn test_func4() { let mut st = setup(); assert_eq!(eval_str(&mut st, "((Func (a b) (+ a b)) 2 2)"), "4"); }
#[test] fn test_func5() { let mut st = setup(); assert_eq!(eval_str(&mut st, "((Func () 5))"), "5"); }

fn main() {}
