use worsp::worsp::*;

// Helper: parse + evaluate a worsp source string, return the last evaluated object
fn eval_src(source: &str) -> Box<Object> {
    let mut state = ParseState { token: None, pos: 0 };
    let mut result = ParseResult { program: None };
    parse(source, &mut state, &mut result);

    let program = result.program.as_ref().unwrap();
    let mut env = Env {
        bindings: std::array::from_fn(|_| Binding { symbol_name: String::new(), value: None }),
        parent: None,
    };
    init_env(&mut env);
    let mut ctx = init_allocator();

    let mut last = Box::new(Object { marked: false, type_: ObjectType::Nil, value: ObjectValue::IntValue(0) });
    let mut cur = program.expressions.as_ref();
    while let Some(el) = cur {
        if let Some(expr) = &el.expression {
            evaluate_expression(expr, &mut last, &mut env, &mut ctx);
        }
        cur = el.next.as_ref();
    }
    last
}

fn eval_str(source: &str) -> String {
    stringify_object(&eval_src(source))
}

// ==================== Tokenizer / Parser ====================

#[test]
fn test_next_tokens() {
    let source = "(+ 1 2)";
    let mut state = ParseState { token: None, pos: 0 };
    next(source, &mut state);
    assert_eq!(match_token(&mut state, TokenKind::LParen), 1);
    next(source, &mut state);
    assert_eq!(match_token(&mut state, TokenKind::Symbol), 1);
    next(source, &mut state);
    assert_eq!(match_token(&mut state, TokenKind::Digit), 1);
    assert_eq!(state.token.as_ref().unwrap().val, 1);
    next(source, &mut state);
    assert_eq!(match_token(&mut state, TokenKind::Digit), 1);
    assert_eq!(state.token.as_ref().unwrap().val, 2);
    next(source, &mut state);
    assert_eq!(match_token(&mut state, TokenKind::RParen), 1);
    next(source, &mut state);
    assert_eq!(match_token(&mut state, TokenKind::Eof), 1);
}

#[test]
fn test_tokenize_string() {
    let source = r#""hello""#;
    let mut state = ParseState { token: None, pos: 0 };
    next(source, &mut state);
    assert_eq!(match_token(&mut state, TokenKind::String), 1);
    assert_eq!(state.token.as_ref().unwrap().str, "hello");
}

#[test]
fn test_tokenize_bool() {
    let mut state = ParseState { token: None, pos: 0 };
    next("true", &mut state);
    assert_eq!(match_token(&mut state, TokenKind::True), 1);

    let mut state = ParseState { token: None, pos: 0 };
    next("false", &mut state);
    assert_eq!(match_token(&mut state, TokenKind::False), 1);
}

#[test]
fn test_tokenize_quote() {
    let mut state = ParseState { token: None, pos: 0 };
    next("'(1)", &mut state);
    assert_eq!(match_token(&mut state, TokenKind::Quote), 1);
}

#[test]
fn test_comment_skipped() {
    let source = "; this is a comment\n(+ 1 2)";
    let mut state = ParseState { token: None, pos: 0 };
    next(source, &mut state);
    assert_eq!(match_token(&mut state, TokenKind::LParen), 1);
}

#[test]
fn test_parse_simple() {
    let mut state = ParseState { token: None, pos: 0 };
    let mut result = ParseResult { program: None };
    parse("(+ 1 2)", &mut state, &mut result);
    assert!(result.program.is_some());
    assert!(result.program.as_ref().unwrap().expressions.is_some());
}

#[test]
fn test_parse_multiple_expressions() {
    let mut state = ParseState { token: None, pos: 0 };
    let mut result = ParseResult { program: None };
    parse("1 2 3", &mut state, &mut result);
    let prog = result.program.as_ref().unwrap();
    let mut count = 0;
    let mut cur = prog.expressions.as_ref();
    while let Some(el) = cur {
        count += 1;
        cur = el.next.as_ref();
    }
    assert_eq!(count, 3);
}

// ==================== Arithmetic ====================

#[test]
fn test_add_integers() {
    assert_eq!(eval_str("(+ 1 2)"), "3");
}

#[test]
fn test_add_strings() {
    assert_eq!(eval_str(r#"(+ "hello" " world")"#), "hello world");
}

#[test]
fn test_sub() {
    assert_eq!(eval_str("(- 10 3)"), "7");
}

#[test]
fn test_mul() {
    assert_eq!(eval_str("(* 3 4)"), "12");
}

#[test]
fn test_div() {
    assert_eq!(eval_str("(/ 10 3)"), "3");
}

#[test]
fn test_mod() {
    assert_eq!(eval_str("(% 10 3)"), "1");
}

#[test]
fn test_add_zero() {
    assert_eq!(eval_str("(+ 0 0)"), "0");
}

#[test]
fn test_nested_arithmetic() {
    // (+ (* 2 3) (- 10 4)) = 6 + 6 = 12
    assert_eq!(eval_str("(+ (* 2 3) (- 10 4))"), "12");
}

// ==================== Boolean ====================

#[test]
fn test_or_true() {
    assert_eq!(eval_str("(|| false false true)"), "T");
}

#[test]
fn test_or_false() {
    assert_eq!(eval_str("(|| false false)"), "F");
}

#[test]
fn test_and_false() {
    assert_eq!(eval_str("(&& true true false)"), "F");
}

#[test]
fn test_and_true() {
    assert_eq!(eval_str("(&& true true)"), "T");
}

#[test]
fn test_not_true() {
    assert_eq!(eval_str("(not true)"), "F");
}

#[test]
fn test_not_false() {
    assert_eq!(eval_str("(not false)"), "T");
}

#[test]
fn test_lt_true() {
    assert_eq!(eval_str("(< 1 2)"), "T");
}

#[test]
fn test_lt_false() {
    assert_eq!(eval_str("(< 2 1)"), "F");
}

#[test]
fn test_gt_true() {
    assert_eq!(eval_str("(> 2 1)"), "T");
}

#[test]
fn test_gt_false() {
    assert_eq!(eval_str("(> 1 2)"), "F");
}

#[test]
fn test_eq_integers() {
    assert_eq!(eval_str("(eq 1 1)"), "T");
    assert_eq!(eval_str("(eq 1 2)"), "F");
}

#[test]
fn test_eq_strings() {
    assert_eq!(eval_str(r#"(eq "a" "a")"#), "T");
    assert_eq!(eval_str(r#"(eq "a" "b")"#), "F");
}

#[test]
fn test_eq_booleans() {
    assert_eq!(eval_str("(eq true true)"), "T");
    assert_eq!(eval_str("(eq true false)"), "F");
}

#[test]
fn test_eq_nil() {
    assert_eq!(eval_str("(eq nil nil)"), "T");
}

// ==================== List operations ====================

#[test]
fn test_car() {
    assert_eq!(eval_str("(car '(1 2 3))"), "1");
}

#[test]
fn test_cdr() {
    assert_eq!(eval_str("(cdr '(1 2 3))"), "(2 3)");
}

#[test]
fn test_cons_with_list() {
    assert_eq!(eval_str("(cons 1 '(2 3))"), "(1 2 3)");
}

#[test]
fn test_cons_with_nil() {
    assert_eq!(eval_str("(cons 1 nil)"), "(1)");
}

#[test]
fn test_cons_two_atoms() {
    assert_eq!(eval_str("(cons 1 2)"), "(1 2)");
}

#[test]
fn test_list_ref() {
    assert_eq!(eval_str("(list-ref '(10 20 30) 0)"), "10");
    assert_eq!(eval_str("(list-ref '(10 20 30) 1)"), "20");
    assert_eq!(eval_str("(list-ref '(10 20 30) 2)"), "30");
}

#[test]
fn test_length_list() {
    assert_eq!(eval_str("(length '(1 2 3))"), "3");
}

#[test]
fn test_length_empty_list() {
    assert_eq!(eval_str("(length '())"), "0");
}

#[test]
fn test_length_string() {
    assert_eq!(eval_str(r#"(length "foobar")"#), "6");
}

#[test]
fn test_push() {
    assert_eq!(eval_str("(= a '(1 2 3))\n(push a 4)\na"), "(1 2 3 4)");
}

#[test]
fn test_push_to_nil() {
    assert_eq!(eval_str("(= a '())\n(push a 1)\n(push a 2)\n(push a 3)\na"), "(1 2 3)");
}

#[test]
fn test_pop_multiple() {
    assert_eq!(eval_str("(= list '(1 2 3 4))\n(pop list)\n(pop list)\n(pop list)"), "2");
}

#[test]
fn test_pop_empty() {
    assert_eq!(eval_str("(pop '())"), "nil");
}

#[test]
fn test_pop_single() {
    assert_eq!(eval_str("(pop '(1))"), "1");
}

#[test]
fn test_empty_list_is_nil() {
    assert_eq!(eval_str("'()"), "nil");
}

// ==================== String operations ====================

#[test]
fn test_split_with_delimiter() {
    assert_eq!(eval_str(r#"(split "a-b-c" "-")"#), "(a b c)");
}

#[test]
fn test_split_empty_delimiter() {
    assert_eq!(eval_str(r#"(split "abc" "")"#), "(a b c)");
}

#[test]
fn test_remove_whitespaces() {
    assert_eq!(eval_str(r#"(remove-whitespaces "foo   bar")"#), "foobar");
}

#[test]
fn test_string_ref() {
    assert_eq!(eval_str(r#"(string-ref "hello" 0)"#), "h");
    assert_eq!(eval_str(r#"(string-ref "abcdefg" 3)"#), "d");
}

#[test]
fn test_is_int_string_true() {
    assert_eq!(eval_str(r#"(is-int-string "345")"#), "T");
}

#[test]
fn test_is_int_string_false() {
    assert_eq!(eval_str(r#"(is-int-string "foo")"#), "F");
}

#[test]
fn test_is_int_string_empty() {
    // C behavior: empty string returns T (while loop never executes)
    assert_eq!(eval_str(r#"(is-int-string "")"#), "T");
}

#[test]
fn test_parse_int() {
    assert_eq!(eval_str(r#"(= foo "35") (= bar (parse-int foo)) (+ bar 5)"#), "40");
}

// ==================== Control flow ====================

#[test]
fn test_if_true() {
    assert_eq!(eval_str("(if true 1 2)"), "1");
}

#[test]
fn test_if_false() {
    assert_eq!(eval_str("(if false 1 2)"), "2");
}

#[test]
fn test_if_no_else() {
    assert_eq!(eval_str("(if false 1)"), "nil");
}

#[test]
fn test_assignment() {
    assert_eq!(eval_str("(= a 1) (= b 2) (= c (+ a b)) c"), "3");
}

#[test]
fn test_while() {
    let src = r#"
(= counter 5)
(= result "")
(while (not (eq counter 0))
  (progn
    (= result (+ result "x"))
    (= counter (- counter 1))
  )
)
result
"#;
    assert_eq!(eval_str(src), "xxxxx");
}

#[test]
fn test_progn() {
    assert_eq!(eval_str("(progn 1 2 3)"), "3");
}

#[test]
fn test_defun_and_call() {
    let src = r#"
(defun fact (n)
  (if (eq n 0)
      1
      (* n (fact (- n 1)))
  ))
(fact 5)
"#;
    assert_eq!(eval_str(src), "120");
}

#[test]
fn test_defun_simple() {
    assert_eq!(eval_str("(defun add1 (x) (+ x 1)) (add1 5)"), "6");
}

// ==================== stringify_object ====================

#[test]
fn test_stringify_integer() {
    let obj = Object { marked: false, type_: ObjectType::Integer, value: ObjectValue::IntValue(42) };
    assert_eq!(stringify_object(&obj), "42");
}

#[test]
fn test_stringify_zero() {
    let obj = Object { marked: false, type_: ObjectType::Integer, value: ObjectValue::IntValue(0) };
    assert_eq!(stringify_object(&obj), "0");
}

#[test]
fn test_stringify_string() {
    let obj = Object { marked: false, type_: ObjectType::String, value: ObjectValue::StringValue("hello".to_string()) };
    assert_eq!(stringify_object(&obj), "hello");
}

#[test]
fn test_stringify_bool_true() {
    let obj = Object { marked: false, type_: ObjectType::Bool, value: ObjectValue::BoolValue(1) };
    assert_eq!(stringify_object(&obj), "T");
}

#[test]
fn test_stringify_bool_false() {
    let obj = Object { marked: false, type_: ObjectType::Bool, value: ObjectValue::BoolValue(0) };
    assert_eq!(stringify_object(&obj), "F");
}

#[test]
fn test_stringify_nil() {
    let obj = Object { marked: false, type_: ObjectType::Nil, value: ObjectValue::IntValue(0) };
    assert_eq!(stringify_object(&obj), "nil");
}

#[test]
fn test_stringify_function() {
    let obj = Object { marked: false, type_: ObjectType::Function, value: ObjectValue::FunctionValue(None) };
    assert_eq!(stringify_object(&obj), "<function>");
}

#[test]
fn test_stringify_list() {
    // Build (1 2 3) manually
    let obj = eval_src("'(1 2 3)");
    assert_eq!(stringify_object(&obj), "(1 2 3)");
}

// ==================== init_env / init_allocator / allocate ====================

#[test]
fn test_init_env() {
    let mut env = Env {
        bindings: std::array::from_fn(|_| Binding { symbol_name: String::new(), value: None }),
        parent: None,
    };
    init_env(&mut env);
    assert!(env.parent.is_none());
    assert!(env.bindings[0].symbol_name.is_empty());
}

#[test]
fn test_init_allocator() {
    let ctx = init_allocator();
    assert_eq!(ctx.gc_less_mode, 0);
    assert!(ctx.stack.is_some());
    assert_eq!(ctx.free_bitmap.iter().sum::<u8>(), 0);
}

#[test]
fn test_allocate() {
    let mut ctx = init_allocator();
    let mut env = Env {
        bindings: std::array::from_fn(|_| Binding { symbol_name: String::new(), value: None }),
        parent: None,
    };
    init_env(&mut env);
    let obj = allocate(&mut ctx, &mut env);
    assert!(obj.is_some());
}

// ==================== match_token ====================

#[test]
fn test_match_token_no_token() {
    let mut state = ParseState { token: None, pos: 0 };
    assert_eq!(match_token(&mut state, TokenKind::Eof), 0);
}

#[test]
fn test_match_token_mismatch() {
    let mut state = ParseState { token: None, pos: 0 };
    next("(", &mut state);
    assert_eq!(match_token(&mut state, TokenKind::RParen), 0);
    assert_eq!(match_token(&mut state, TokenKind::LParen), 1);
}

// ==================== evaluate (top-level) ====================

#[test]
fn test_evaluate_top_level() {
    let mut state = ParseState { token: None, pos: 0 };
    let mut result = ParseResult { program: None };
    parse("(= x 42)", &mut state, &mut result);
    evaluate(&mut result);
    // Just verify it doesn't panic
}

// ==================== evaluate_expression_with_context ====================

#[test]
fn test_evaluate_expression_with_context() {
    let mut state = ParseState { token: None, pos: 0 };
    let mut result = ParseResult { program: None };
    parse("(+ 1 2)", &mut state, &mut result);
    let prog = result.program.as_ref().unwrap();
    let expr = prog.expressions.as_ref().unwrap().expression.as_ref().unwrap();
    let mut env = Env {
        bindings: std::array::from_fn(|_| Binding { symbol_name: String::new(), value: None }),
        parent: None,
    };
    init_env(&mut env);
    let mut obj = Object { marked: false, type_: ObjectType::Nil, value: ObjectValue::IntValue(0) };
    evaluate_expression_with_context(expr, &mut obj, &mut env);
    assert_eq!(stringify_object(&obj), "3");
}

// ==================== Complex integration (snapshot fixtures) ====================

#[test]
fn test_map_fixture() {
    let src = r#"
(= list '("1st" "2nd" "3rd"))
(= list-length (length list))
(= results nil)
(= i 0)
(while (< i list-length)
  (progn
    (push results (list-ref list i))
    (= i (+ i 1))
  )
)
results
"#;
    assert_eq!(eval_str(src), "(1st 2nd 3rd)");
}

#[test]
fn test_splice_fixture() {
    let src = r#"
(defun string-ref (str index) (progn (list-ref (split str "") index)))
(defun splice (str start end)
  (progn
    (= result "")
    (= index start)
    (while (not (eq index end))
      (progn
        (= result (+ result (string-ref str index)))
        (= index (+ index 1))
      )
    )
    (progn result)
  )
)
(splice "foobar" 2 4)
"#;
    assert_eq!(eval_str(src), "ob");
}

#[test]
fn test_literal_values() {
    assert_eq!(eval_str("42"), "42");
    assert_eq!(eval_str("0"), "0");
    assert_eq!(eval_str(r#""hello""#), "hello");
    assert_eq!(eval_str("true"), "T");
    assert_eq!(eval_str("false"), "F");
    assert_eq!(eval_str("nil"), "nil");
}

#[test]
fn test_comment_handling() {
    assert_eq!(eval_str("; comment\n(+ 1 2)"), "3");
}

fn main() {}
