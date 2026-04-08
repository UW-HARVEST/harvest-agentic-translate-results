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

// List proc
#[test] fn test_apply1()   { let mut st = setup(); assert_eq!(eval_str(&mut st, "(apply list '(1 2 3))"), "(1 2 3)"); }
#[test] fn test_apply2()   { let mut st = setup(); assert_eq!(eval_str(&mut st, "(apply + '(2 90))"), "92"); }
#[test] fn test_apply3()   { let mut st = setup(); assert_eq!(eval_str(&mut st, "(apply list '(a b c d e))"), "(a b c d e)"); }
#[test] fn test_map1()     { let mut st = setup(); assert_eq!(eval_str(&mut st, "(map car '((1 a) (2 b) (3 c)))"), "(1 2 3)"); }
#[test] fn test_map2()     { let mut st = setup(); assert_eq!(eval_str(&mut st, "(map cdr '((1 a) (2 b) (3 c)))"), "((a) (b) (c))"); }
#[test] fn test_map3()     { let mut st = setup(); assert_eq!(eval_str(&mut st, "(map (Func (x) (car (cdr x))) '((1 a) (2 b) (3 c)))"), "(a b c)"); }
#[test] fn test_map4()     { let mut st = setup(); assert_eq!(eval_str(&mut st, "(map cadr '((1/2 .5) (\"conky\" .25) ('bubbles .125)))"), "(0.5 0.25 0.125)"); }
#[test] fn test_map5()     { let mut st = setup(); assert_eq!(eval_str(&mut st, "(map inc (list 2 4 8 (^ 2 4)))"), "(3 5 9 17)"); }
#[test] fn test_convert1() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(convert 1 2 '(1 2 3 1 1 4 5 6 7 1))"), "(2 2 3 2 2 4 5 6 7 2)"); }
#[test] fn test_convert2() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(convert 'hey 'hello '(hi sup hey hey hello hola))"), "(hi sup hello hello hello hola)"); }
#[test] fn test_compose1() { let mut st = setup(); assert_eq!(eval_str(&mut st, "((compose - sqrt) 9)"), "-3"); }
#[test] fn test_compose2() { let mut st = setup(); assert_eq!(eval_str(&mut st, "((compose / sqrt sqr) 18)"), "1/18"); }
#[test] fn test_compose3() { let mut st = setup(); assert_eq!(eval_str(&mut st, "((compose - sqrt cube) 4)"), "-8"); }
#[test] fn test_compose4() { let mut st = setup(); assert_eq!(eval_str(&mut st, "((compose -) 5/3)"), "-5/3"); }
#[test] fn test_compose5() { let mut st = setup(); assert_eq!(eval_str(&mut st, "((compose - +) 5 6)"), "-11"); }
#[test] fn test_compose6() { let mut st = setup(); assert_eq!(eval_str(&mut st, "((compose sqrt Int *) 4.5 2)"), "3"); }

// List filter
#[test] fn test_filter1() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(filter positive? '(1 2 -4 5 -9 10))"), "(1 2 5 10)"); }
#[test] fn test_filter2() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(filter odd? '(8 6 17 9 82 34 27))"), "(17 9 27)"); }
#[test] fn test_filter3() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(filter integer? (list 1/2 3.e-2 9/3 3.2 0.0 8 17))"), "(3 8 17)"); }
#[test] fn test_keep1()   { let mut st = setup(); assert_eq!(eval_str(&mut st, "(keep '+ '(+ * - - + + - / sqrt))"), "(+ + +)"); }
#[test] fn test_keep2()   { let mut st = setup(); assert_eq!(eval_str(&mut st, "(keep 5 (list (+ 1 2) (- 10 5) 3/2 5 (/ 15 5)))"), "(5 5)"); }
#[test] fn test_keep3()   { let mut st = setup(); assert_eq!(eval_str(&mut st, "(keep 3.2 '(3. 3 3.02 3.12 3.20 3.7))"), "(3.2)"); }
#[test] fn test_keep4()   { let mut st = setup(); assert_eq!(eval_str(&mut st, "(keep 'a '('a b c d e))"), "Nil"); }
#[test] fn test_remove1() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(remove 1/2 (list 3/4 4/8 6/7 19/17 6/8 1/2))"), "(3/4 6/7 19/17 3/4)"); }
#[test] fn test_remove2() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(remove 2 '(1 3 4 5))"), "(1 3 4 5)"); }
#[test] fn test_remove3() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(remove \"greg\" '(greg 'greg \"wirt\" beatrice))"), "(greg (quote greg) wirt beatrice)"); }

// List mod
#[test] fn test_reverse1() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(reverse '(1 2 3 4 5))"), "(5 4 3 2 1)"); }
#[test] fn test_reverse2() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(reverse (list -20 5/2 .398))"), "(0.398 5/2 -20)"); }
#[test] fn test_reverse3() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(reverse '(a b))"), "(b a)"); }
#[test] fn test_reverse4() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(reverse (list \"foo\" \"bar\" \"baz\"))"), "(baz bar foo)"); }
#[test] fn test_reverse5() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(reverse (cons 1/2 Nil))"), "(1/2)"); }
#[test] fn test_reverse6() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(reverse ())"), "Nil"); }
#[test] fn test_append1()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(append '(1 2 3) '(4 5 6))"), "(1 2 3 4 5 6)"); }
#[test] fn test_append2()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(append (list (+ 1 2) 4) '(a b c))"), "(3 4 a b c)"); }

// Assoc
#[test] fn test_zip1()   { let mut st = setup(); assert_eq!(eval_str(&mut st, "(zip '(1 2 3 4) '(a b c d))"), "((1 . a) (2 . b) (3 . c) (4 . d))"); }
#[test] fn test_zip2()   { let mut st = setup(); assert_eq!(eval_str(&mut st, "(zip (list 'ricky 'lahey) (list \"julian\" \"randy\"))"), "((ricky . julian) (lahey . randy))"); }
#[test] fn test_assoc1() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(assoc 'baz '((foo . 3) (bar . 8) (baz . 14)))"), "(baz . 14)"); }
#[test] fn test_assoc2() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(assoc 'a '((a b) (3 2.1) (3.2 4/3) (3.2 3.2)))"), "(a b)"); }
#[test] fn test_assoc3() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(assoc 3 '((1 b)))"), "Nil"); }
#[test] fn test_assoc4() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(assoc 4/3 (list (list 1 pi) (list 4/3 1/2 3) (list 2 3)))"), "(4/3 1/2 3)"); }

// List member
#[test] fn test_memp1()    { let mut st = setup(); assert_eq!(eval_str(&mut st, "(memp even? (list 1 3 19 4 7 8 2))"), "(4 7 8 2)"); }
#[test] fn test_memp2()    { let mut st = setup(); assert_eq!(eval_str(&mut st, "(memp negative? (list 1/3 pi 3.2e-9 0 4 -7 2))"), "(-7 2)"); }
#[test] fn test_memp3()    { let mut st = setup(); assert_eq!(eval_str(&mut st, "(memp (Func (x) (> x 8)) '(1/3 1/2 5/3 8 9))"), "(9)"); }
#[test] fn test_memp4()    { let mut st = setup(); assert_eq!(eval_str(&mut st, "(memp (Func (x) (= x \"fry\")) '(\"fry\" \"nibbler\" \"prof\"))"), "(fry nibbler prof)"); }
#[test] fn test_member1()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(member 'foo '(foo bar baz))"), "(foo bar baz)"); }
#[test] fn test_member2()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(member 'bar '(foo bar baz))"), "(bar baz)"); }
#[test] fn test_member3()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(member 4 '(12 38 4 8))"), "(4 8)"); }
#[test] fn test_member4()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(member 3.2 '(4/3 2 8 2 3.14 3.2))"), "(3.2)"); }
#[test] fn test_member5()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(member \"quux\" (list 4.2 3 'quux))"), "Nil"); }
#[test] fn test_member6()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(member 'qux '(foo bar baz))"), "Nil"); }
#[test] fn test_everyp1()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(everyp? even? '(2 4 10 18))"), "True"); }
#[test] fn test_everyp2()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(everyp? odd? '(1 2 3 9 10))"), "Nil"); }
#[test] fn test_everyp3()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(everyp? integer? '(1. 2/3 3.14 4/5))"), "Nil"); }
#[test] fn test_every1()   { let mut st = setup(); assert_eq!(eval_str(&mut st, "(every? 'foo '(foo bar baz))"), "Nil"); }
#[test] fn test_every2()   { let mut st = setup(); assert_eq!(eval_str(&mut st, "(every? \"a\" '(a 'a \"a\"))"), "Nil"); }
#[test] fn test_every3()   { let mut st = setup(); assert_eq!(eval_str(&mut st, "(every? 3 (list 3 (+ 1 2) (- 5 2)))"), "True"); }

// Quasiquote
#[test] fn test_qq1()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "`7.2"), "7.2"); }
#[test] fn test_qq2()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "`cory"), "cory"); }
#[test] fn test_qq3()  {
    let mut st = setup();
    eval_str(&mut st, "(def foo 8)");
    assert_eq!(eval_str(&mut st, "`,foo"), "8");
}
#[test] fn test_qq4()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "`(1 2 3)"), "(1 2 3)"); }
#[test] fn test_qq5()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "`(\"sunnyvale\")"), "(sunnyvale)"); }
#[test] fn test_qq6()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "`(1/2 . 2/1)"), "(1/2 . 2)"); }
#[test] fn test_qq7()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "`(cory trevor)"), "(cory trevor)"); }
#[test] fn test_qq8()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "`(foo bar quax)"), "(foo bar quax)"); }
#[test] fn test_qq9()  {
    let mut st = setup();
    eval_str(&mut st, "(def foo 8)");
    eval_str(&mut st, "(def bar 4)");
    assert_eq!(eval_str(&mut st, "`(,foo ,bar)"), "(8 4)");
}
#[test] fn test_qq10() {
    let mut st = setup();
    eval_str(&mut st, "(def foo 8)");
    eval_str(&mut st, "(def bar 4)");
    assert_eq!(eval_str(&mut st, "`(,foo . ,bar)"), "(8 . 4)");
}
#[test] fn test_qq11() {
    let mut st = setup();
    eval_str(&mut st, "(def foo 8)");
    eval_str(&mut st, "(def bar 4)");
    assert_eq!(eval_str(&mut st, "`(,foo ,@bar)"), "(8 . 4)");
}
#[test] fn test_qq12() {
    let mut st = setup();
    eval_str(&mut st, "(def foo 8)");
    assert_eq!(eval_str(&mut st, "`(foo bar ,foo fry)"), "(foo bar 8 fry)");
}
#[test] fn test_qq13() { let mut st = setup(); assert_eq!(eval_str(&mut st, "`(1 ,(+ 1 2) 5 ,(- 9 2))"), "(1 3 5 7)"); }
#[test] fn test_qq14() { let mut st = setup(); assert_eq!(eval_str(&mut st, "`(1 ,@(list 4 9))"), "(1 4 9)"); }
#[test] fn test_qq15() {
    let mut st = setup();
    eval_str(&mut st, "(def foo 8)");
    assert_eq!(eval_str(&mut st, "`(3 ,@foo)"), "(3 . 8)");
}
#[test] fn test_qq16() {
    let mut st = setup();
    eval_str(&mut st, "(def foo 8)");
    assert_eq!(eval_str(&mut st, "`(a b c ,@foo)"), "(a b c . 8)");
}
#[test] fn test_qq17() { let mut st = setup(); assert_eq!(eval_str(&mut st, "`(0 ,@(list 1 2) 3 4)"), "(0 1 2 3 4)"); }

// Stack
#[test] fn test_peek1() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(peek '(1 2 3 4 5 6))"), "1"); }
#[test] fn test_peek2() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(peek (list 'a 'b 'c))"), "a"); }
#[test] fn test_pop1()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(pop (list 1/2 1/4))"), "(1/4)"); }
#[test] fn test_pop2()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(pop '(\"foo\" \"bar\" \"baz\"))"), "(bar baz)"); }
#[test] fn test_push1() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(push '(6 3 5/3 .38) .5)"), "(0.5 6 3 5/3 0.38)"); }
#[test] fn test_push2() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(push (list \"ni\" 'shrubbery) (* 3 2))"), "(6 ni shrubbery)"); }
#[test] fn test_swap1() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(swap '(1 2 3 5 7 11))"), "(2 1 3 5 7 11)"); }
#[test] fn test_swap2() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(swap (list 1/2 1/4 1/9 1/16))"), "(1/4 1/2 1/9 1/16)"); }

fn main() {}
