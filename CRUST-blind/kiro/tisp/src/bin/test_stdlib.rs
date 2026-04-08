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

// Control flow (if/when/unless/switch)
#[test] fn test_if1()     { let mut st = setup(); assert_eq!(eval_str(&mut st, "(if True 1 2)"), "1"); }
#[test] fn test_if2()     { let mut st = setup(); assert_eq!(eval_str(&mut st, "(if () 1 2)"), "2"); }
#[test] fn test_if3()     { let mut st = setup(); assert_eq!(eval_str(&mut st, "(if (integer? 3) True ())"), "True"); }
#[test] fn test_if4()     { let mut st = setup(); assert_eq!(eval_str(&mut st, "(if (ratio? car) (cons 1 2) (car '(1 2)))"), "1"); }
#[test] fn test_when1()   { let mut st = setup(); assert_eq!(eval_str(&mut st, "(when True 'foo)"), "foo"); }
#[test] fn test_when2()   { let mut st = setup(); assert_eq!(eval_str(&mut st, "(when () 'b  ar)"), "Void"); }
#[test] fn test_when3()   { let mut st = setup(); assert_eq!(eval_str(&mut st, "(when (= 1 1) 4)"), "4"); }
#[test] fn test_unless1() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(unless True 'foo)"), "Void"); }
#[test] fn test_unless2() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(unless () 'bar)"), "bar"); }
#[test] fn test_unless3() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(unless 3 4)"), "Void"); }
#[test] fn test_unless4() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(unless (< 5 4) 7)"), "7"); }
#[test] fn test_switch1() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(switch 5 (3 'yes) (5 'no))"), "no"); }
#[test] fn test_switch2() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(switch (+ 1 2) ((mod 8 5) 'yes) (err 'no))"), "yes"); }
#[test] fn test_switch3() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(switch 2 (3 'yes) (5 'no))"), "Void"); }
#[test] fn test_switch4() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(switch \"foo\" (e \"bar\") (\"foo\" 'zar) ('baz 3))"), "zar"); }

// Logic
#[test] fn test_not1()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(not ())"), "True"); }
#[test] fn test_not2()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(not True)"), "Nil"); }
#[test] fn test_and1()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(and () ())"), "Nil"); }
#[test] fn test_and2()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(and True ())"), "Nil"); }
#[test] fn test_and3()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(and () True)"), "Nil"); }
#[test] fn test_and4()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(and True True)"), "True"); }
#[test] fn test_or1()   { let mut st = setup(); assert_eq!(eval_str(&mut st, "(or () ())"), "Nil"); }
#[test] fn test_or2()   { let mut st = setup(); assert_eq!(eval_str(&mut st, "(or True ())"), "True"); }
#[test] fn test_or3()   { let mut st = setup(); assert_eq!(eval_str(&mut st, "(or () True)"), "True"); }
#[test] fn test_or4()   { let mut st = setup(); assert_eq!(eval_str(&mut st, "(or True True)"), "True"); }
#[test] fn test_xor1()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(xor? Nil ())"), "Nil"); }
#[test] fn test_xor2()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(xor? True Nil)"), "True"); }
#[test] fn test_xor3()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(xor? Nil True)"), "True"); }
#[test] fn test_xor4()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(xor? True True)"), "Nil"); }

// List
#[test] fn test_list1()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(list 1 2 3)"), "(1 2 3)"); }
#[test] fn test_list2()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(list (* 2 2) (+ 2 3))"), "(4 5)"); }
#[test] fn test_list3()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(list 'a 'b 'c 'd 'e 'f)"), "(a b c d e f)"); }
#[test] fn test_list4()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(list \"foo\")"), "(foo)"); }
#[test] fn test_list5()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(list)"), "Nil"); }
#[test] fn test_list6()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(list 1/2 2/8 . 1/8)"), "(1/2 1/4 . 1/8)"); }
#[test] fn test_list7()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(list* .5 .25 .125)"), "(0.5 0.25 . 0.125)"); }
#[test] fn test_list8()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(list* 1 2 3 4 5 6)"), "(1 2 3 4 5 . 6)"); }
#[test] fn test_list9()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(list* (sqr 3) (cube 4))"), "(9 . 64)"); }
#[test] fn test_list10() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(list* 5/4)"), "5/4"); }

// List get
#[test] fn test_last1()   { let mut st = setup(); assert_eq!(eval_str(&mut st, "(last '(1 2 3))"), "3"); }
#[test] fn test_last2()   { let mut st = setup(); assert_eq!(eval_str(&mut st, "(last (list 4 5))"), "5"); }
#[test] fn test_last3()   { let mut st = setup(); assert_eq!(eval_str(&mut st, "(last '(a b c d e f))"), "f"); }
#[test] fn test_last4()   { let mut st = setup(); assert_eq!(eval_str(&mut st, "(last (cons 1 (cons 2 ())))"), "2"); }
#[test] fn test_length1() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(length '(1 2 3))"), "3"); }
#[test] fn test_length2() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(length (list .3 -3/2 12 5))"), "4"); }
#[test] fn test_length3() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(length '(a b))"), "2"); }
#[test] fn test_length4() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(length (list list))"), "1"); }
#[test] fn test_length5() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(length ())"), "0"); }
#[test] fn test_length6() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(length Nil)"), "0"); }
#[test] fn test_nth1()    { let mut st = setup(); assert_eq!(eval_str(&mut st, "(nth '(1 2 3) 1)"), "2"); }
#[test] fn test_nth2()    { let mut st = setup(); assert_eq!(eval_str(&mut st, "(nth (list 3 5/2 .332 -2) 2)"), "0.332"); }
#[test] fn test_nth3()    { let mut st = setup(); assert_eq!(eval_str(&mut st, "(nth '(a b c) 0)"), "a"); }
#[test] fn test_nth4()    { let mut st = setup(); assert_eq!(eval_str(&mut st, "(nth (list 'foo 'bar 'zar 'baz) 3)"), "baz"); }
#[test] fn test_head1()   { let mut st = setup(); assert_eq!(eval_str(&mut st, "(head '(1.2 1.3 1.4 1.5) 2)"), "(1.2 1.3)"); }
#[test] fn test_head2()   { let mut st = setup(); assert_eq!(eval_str(&mut st, "(head '(1 1e1 1e2 1e3) 3)"), "(1 10 100)"); }
#[test] fn test_head3()   { let mut st = setup(); assert_eq!(eval_str(&mut st, "(head '(1 2 3) 1)"), "(1)"); }
#[test] fn test_head4()   { let mut st = setup(); assert_eq!(eval_str(&mut st, "(head '(1 2) 0)"), "Nil"); }
#[test] fn test_tail1()   { let mut st = setup(); assert_eq!(eval_str(&mut st, "(tail '(randy bobandy lahey bubs) 3)"), "(bubs)"); }
#[test] fn test_tail2()   { let mut st = setup(); assert_eq!(eval_str(&mut st, "(tail (list 1/2 1/3 1/4) 0)"), "(1/2 1/3 1/4)"); }
#[test] fn test_tail3()   { let mut st = setup(); assert_eq!(eval_str(&mut st, "(tail '(2 4 9 16 25 36) 2)"), "(9 16 25 36)"); }
#[test] fn test_count1()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(count 3 '(1 2 3 4))"), "1"); }
#[test] fn test_count2()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(count 1/2 (list 1/2 1/3 2/4 8 9.0))"), "2"); }
#[test] fn test_count3()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(count 'a '(b c a a f h a b c a))"), "4"); }
#[test] fn test_count4()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(count 3.2 Nil)"), "0"); }
#[test] fn test_count5()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(count \"Bobandy\" '(1/2 1/4 \"Jim\"))"), "0"); }

fn main() {}
