use tisp_proj::tisp::*;

fn setup() -> Tsp {
    let mut st = tisp_env_init(1024);
    tib_env_core(&mut st);
    tib_env_math(&mut st);
    tib_env_string(&mut st);
    st
}

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

/// Load minimal tibs definitions needed for tests
fn load_mini_tibs(st: &mut Tsp) {
    let defs = [
        "(def (list . lst) lst)",
        "(def else True)",
    ];
    for d in &defs {
        eval_str(st, d);
    }
}

// ---- quote ----

#[test]
fn test_quote() {
    let mut st = setup();
    assert_eq!(eval_str(&mut st, "(quote 1)"), "1");
    assert_eq!(eval_str(&mut st, "(quote 9234)"), "9234");
    assert_eq!(eval_str(&mut st, "(quote \"foo\")"), "foo");
    assert_eq!(eval_str(&mut st, "(quote bar)"), "bar");
    assert_eq!(eval_str(&mut st, "(quote (1 2 3 4))"), "(1 2 3 4)");
    assert_eq!(eval_str(&mut st, "(quote (quote 1))"), "(quote 1)");
    assert_eq!(eval_str(&mut st, "(quote (+ 2 2))"), "(+ 2 2)");
    assert_eq!(eval_str(&mut st, "'12"), "12");
    assert_eq!(eval_str(&mut st, "'foo"), "foo");
    assert_eq!(eval_str(&mut st, "'(1 2 3 4)"), "(1 2 3 4)");
    assert_eq!(eval_str(&mut st, "'()"), "Nil");
}

// ---- cons ----

#[test]
fn test_cons() {
    let mut st = setup();
    assert_eq!(eval_str(&mut st, "(cons 1 2)"), "(1 . 2)");
    assert_eq!(eval_str(&mut st, "(cons 1 (cons 2 3))"), "(1 2 . 3)");
    assert_eq!(eval_str(&mut st, "(cons 1 (cons 2 (cons 3 4)))"), "(1 2 3 . 4)");
    assert_eq!(eval_str(&mut st, "(cons \"foo\" \"bar\")"), "(foo . bar)");
    assert_eq!(eval_str(&mut st, "(cons (+ 1 2) 3)"), "(3 . 3)");
    assert_eq!(eval_str(&mut st, "(cons (cons 1 2) (cons 3 4))"), "((1 . 2) 3 . 4)");
}

// ---- car/cdr ----

#[test]
fn test_cxr() {
    let mut st = setup();
    assert_eq!(eval_str(&mut st, "(car (cons 1 2))"), "1");
    assert_eq!(eval_str(&mut st, "(cdr (cons 1 2))"), "2");
    assert_eq!(eval_str(&mut st, "(car (quote (1 2 3 4)))"), "1");
    assert_eq!(eval_str(&mut st, "(car (cdr (quote (1 2 3 4))))"), "2");
    assert_eq!(eval_str(&mut st, "(car (cdr (cdr (quote (1 2 3 4)))))"), "3");
    assert_eq!(eval_str(&mut st, "(car (cdr (cdr (cdr (quote (1 2 3 4))))))"), "4");
    assert_eq!(eval_str(&mut st, "(cdr (quote (1 2 3 4)))"), "(2 3 4)");
    assert_eq!(eval_str(&mut st, "(cdr (cdr (quote (1 2 3 4))))"), "(3 4)");
    assert_eq!(eval_str(&mut st, "(car (cons 1 (cons 2 3)))"), "1");
    assert_eq!(eval_str(&mut st, "(cdr (cons 1 (cons 2 3)))"), "(2 . 3)");
    assert_eq!(eval_str(&mut st, "(cdr (cdr (cons 1 (cons 2 3))))"), "3");
}

// ---- do ----

#[test]
fn test_do() {
    let mut st = setup();
    assert_eq!(eval_str(&mut st, "(do (+ 1 2) (+ 2 2))"), "4");
    assert_eq!(eval_str(&mut st, "(do (+ -4 8) (- 1 2) (* 80 0) (+ 39 -3))"), "36");
    assert_eq!(eval_str(&mut st, "(do (mod 80 2) (/ 4 2) Void)"), "Void");
}

// ---- eval ----

#[test]
fn test_eval() {
    let mut st = setup();
    assert_eq!(eval_str(&mut st, "(eval ''hey)"), "hey");
    assert_eq!(eval_str(&mut st, "(eval \"sup\")"), "sup");
    assert_eq!(eval_str(&mut st, "(eval (+ 1 2))"), "3");
    assert_eq!(eval_str(&mut st, "(eval '(- 4 3))"), "1");
    assert_eq!(eval_str(&mut st, "(eval ''(mod 9 3))"), "(mod 9 3)");
    assert_eq!(eval_str(&mut st, "(do (def bar '(/ 25 5)) (eval bar))"), "5");
}

// ---- cond ----

#[test]
fn test_cond() {
    let mut st = setup();
    load_mini_tibs(&mut st);
    assert_eq!(eval_str(&mut st, "(cond)"), "Void");
    assert_eq!(eval_str(&mut st, "(cond (True 1))"), "1");
    assert_eq!(eval_str(&mut st, "(cond ((= 1 1) 1) ((= 1 2) 2) (True 3))"), "1");
    assert_eq!(eval_str(&mut st, "(cond ((= 1 2) 1) ((= 1 2) 2) (else (+ 1 2)))"), "3");
    assert_eq!(eval_str(&mut st, "(cond ((= 1 2) 1) ((= 1 1) 2) (else 3))"), "2");
    assert_eq!(eval_str(&mut st, "(cond ((= 1 2) 1) ((= 1 3) 2))"), "Void");
    assert_eq!(eval_str(&mut st, "(cond ((= 1 2) 1) (\"foo\" 2) (else 3))"), "2");
    assert_eq!(eval_str(&mut st, "(cond (() (+ 1 2)))"), "Void");
}

// ---- eq ----

#[test]
fn test_eq() {
    let mut st = setup();
    assert_eq!(eval_str(&mut st, "(=)"), "True");
    assert_eq!(eval_str(&mut st, "(= 1)"), "True");
    assert_eq!(eval_str(&mut st, "(= \"foo\")"), "True");
    assert_eq!(eval_str(&mut st, "(= 1 1)"), "True");
    assert_eq!(eval_str(&mut st, "(= 1 1 1 1 1 1)"), "True");
    assert_eq!(eval_str(&mut st, "(= 1 2)"), "Nil");
    assert_eq!(eval_str(&mut st, "(= 1 1 2 1 1 1)"), "Nil");
    assert_eq!(eval_str(&mut st, "(= 4/5 4/5)"), "True");
    assert_eq!(eval_str(&mut st, "(= 2/1 2)"), "True");
    assert_eq!(eval_str(&mut st, "(= 2/4 1/2)"), "True");
    assert_eq!(eval_str(&mut st, "(= 2/4 1/2 4/8 3/6)"), "True");
    assert_eq!(eval_str(&mut st, "(= 1/2 4/5)"), "Nil");
    assert_eq!(eval_str(&mut st, "(= \"foo\" \"foo\")"), "True");
    assert_eq!(eval_str(&mut st, "(= \"foo\" \"bar\")"), "Nil");
    assert_eq!(eval_str(&mut st, "(= \"foo\" 'foo)"), "Nil");
    assert_eq!(eval_str(&mut st, "(= 'bar 'bar)"), "True");
    assert_eq!(eval_str(&mut st, "(= \"foo\" 3)"), "Nil");
    assert_eq!(eval_str(&mut st, "(= \"foo\" \"FOO\")"), "Nil");
    assert_eq!(eval_str(&mut st, "(= True True)"), "True");
    assert_eq!(eval_str(&mut st, "(= car car)"), "True");
    assert_eq!(eval_str(&mut st, "(= car cdr)"), "Nil");
    assert_eq!(eval_str(&mut st, "(= quote quote quote)"), "True");
}

// ---- def ----

#[test]
fn test_def() {
    let mut st = setup();
    assert_eq!(eval_str(&mut st, "(def foo 4)"), "Void");
    assert_eq!(eval_str(&mut st, "foo"), "4");
    assert_eq!(eval_str(&mut st, "(def bar foo)"), "Void");
    assert_eq!(eval_str(&mut st, "bar"), "4");
    assert_eq!(eval_str(&mut st, "(def foo (+ foo bar))"), "Void");
    assert_eq!(eval_str(&mut st, "foo"), "8");
    assert_eq!(eval_str(&mut st, "(def add +)"), "Void");
    assert_eq!(eval_str(&mut st, "(add foo bar)"), "12");
    assert_eq!(eval_str(&mut st, "(def (one x) (add x 1))"), "Void");
    assert_eq!(eval_str(&mut st, "(one foo)"), "9");
    assert_eq!(eval_str(&mut st, "(def (more x) (def term 3) (+ x term))"), "Void");
    assert_eq!(eval_str(&mut st, "(more 8)"), "11");
    assert_eq!(eval_str(&mut st, "(def (add2 x) (+ x 1) (+ x 2))"), "Void");
    assert_eq!(eval_str(&mut st, "(add2 2)"), "4");
}

// ---- defined? ----

#[test]
fn test_definedp() {
    let mut st = setup();
    assert_eq!(eval_str(&mut st, "(defined? invalid-var)"), "Nil");
    assert_eq!(eval_str(&mut st, "(defined? defined?)"), "True");
    assert_eq!(eval_str(&mut st, "(defined? car)"), "True");
}

// ---- Func ----

#[test]
fn test_func() {
    let mut st = setup();
    assert_eq!(eval_str(&mut st, "((Func (x) x) 3)"), "3");
    assert_eq!(eval_str(&mut st, "((Func (x) x) (+ 1 2))"), "3");
    assert_eq!(eval_str(&mut st, "((Func (x) (+ x 1)) 8)"), "9");
    assert_eq!(eval_str(&mut st, "((Func (a b) (+ a b)) 2 2)"), "4");
    assert_eq!(eval_str(&mut st, "((Func () 5))"), "5");
}

// ---- typeof ----

#[test]
fn test_typeof() {
    let mut st = setup();
    assert_eq!(eval_str(&mut st, "(typeof 42)"), "Int");
    assert_eq!(eval_str(&mut st, "(typeof 3.14)"), "Dec");
    assert_eq!(eval_str(&mut st, "(typeof 3/4)"), "Ratio");
    assert_eq!(eval_str(&mut st, "(typeof \"foo\")"), "Str");
    assert_eq!(eval_str(&mut st, "(typeof 'bar)"), "Sym");
    assert_eq!(eval_str(&mut st, "(typeof car)"), "Prim");
    assert_eq!(eval_str(&mut st, "(typeof quote)"), "Form");
    assert_eq!(eval_str(&mut st, "(typeof Nil)"), "Nil");
    assert_eq!(eval_str(&mut st, "(typeof Void)"), "Void");
    assert_eq!(eval_str(&mut st, "(typeof (cons 1 2))"), "Pair");
}

// ---- eq with lists ----

#[test]
fn test_eq_lists() {
    let mut st = setup();
    load_mini_tibs(&mut st);
    assert_eq!(eval_str(&mut st, "(= '(1 2 3) (list 1 2 3))"), "True");
    assert_eq!(eval_str(&mut st, "(= '(a b c) '(a b c))"), "True");
    assert_eq!(eval_str(&mut st, "(= '(a b c) '(a b d))"), "Nil");
    assert_eq!(eval_str(&mut st, "(= '(1 2 3) '(1 2))"), "Nil");
    assert_eq!(eval_str(&mut st, "(= '((1 2) 3 4) '((1 2) 3 4))"), "True");
    assert_eq!(eval_str(&mut st, "(= '((1 b) 3 4) '((1 2) 3 4))"), "Nil");
}

// ---- eq with Func ----

#[test]
fn test_eq_func() {
    let mut st = setup();
    assert_eq!(eval_str(&mut st, "(= (Func (it) it) @it)"), "True");
    assert_eq!(eval_str(&mut st, "(= @it (Func (x) x))"), "Nil");
}

fn main() {}
