use worsp::worsp::*;

fn eval_worsp(source: &str) -> String {
    let mut state = ParseState { token: None, pos: 0 };
    let mut result = ParseResult { program: None };
    parse(source, &mut state, &mut result);
    let mut env = Env {
        bindings: std::array::from_fn(|_| Binding { symbol_name: String::new(), value: None }),
        parent: None,
    };
    init_env(&mut env);
    let mut context = init_allocator();
    let mut evaluated = Box::new(Object { marked: false, type_: ObjectType::Nil, value: ObjectValue::IntValue(0) });
    if let Some(ref program) = result.program {
        let mut exprs = &program.expressions;
        while let Some(ref el) = exprs {
            evaluated = Box::new(Object { marked: false, type_: ObjectType::Nil, value: ObjectValue::IntValue(0) });
            if let Some(ref expr) = el.expression {
                evaluate_expression(expr, &mut evaluated, &mut env, &mut context);
            }
            exprs = &el.next;
        }
    }
    stringify_object(&evaluated)
}

// ==================== Tokenizer Tests ====================

#[test]
fn test_tokenizer_lparen() {
    let mut state = ParseState { token: None, pos: 0 };
    next("(+ 1 2)", &mut state);
    assert_eq!(state.token.as_ref().unwrap().kind, TokenKind::LParen);
}

#[test]
fn test_tokenizer_symbol() {
    let mut state = ParseState { token: None, pos: 0 };
    next("(+ 1 2)", &mut state);
    next("(+ 1 2)", &mut state);
    assert_eq!(state.token.as_ref().unwrap().kind, TokenKind::Symbol);
    assert_eq!(state.token.as_ref().unwrap().str, "+");
}

#[test]
fn test_tokenizer_digit() {
    let mut state = ParseState { token: None, pos: 0 };
    next("(+ 1 2)", &mut state);
    next("(+ 1 2)", &mut state);
    next("(+ 1 2)", &mut state);
    assert_eq!(state.token.as_ref().unwrap().kind, TokenKind::Digit);
    assert_eq!(state.token.as_ref().unwrap().val, 1);
}

#[test]
fn test_tokenizer_rparen() {
    let mut state = ParseState { token: None, pos: 0 };
    next("(+ 1 2)", &mut state);
    next("(+ 1 2)", &mut state);
    next("(+ 1 2)", &mut state);
    next("(+ 1 2)", &mut state);
    next("(+ 1 2)", &mut state);
    assert_eq!(state.token.as_ref().unwrap().kind, TokenKind::RParen);
}

#[test]
fn test_tokenizer_string() {
    let mut state = ParseState { token: None, pos: 0 };
    next("\"hello\"", &mut state);
    assert_eq!(state.token.as_ref().unwrap().kind, TokenKind::String);
    assert_eq!(state.token.as_ref().unwrap().str, "hello");
}

#[test]
fn test_tokenizer_true() {
    let mut state = ParseState { token: None, pos: 0 };
    next("true", &mut state);
    assert_eq!(state.token.as_ref().unwrap().kind, TokenKind::True);
}

#[test]
fn test_tokenizer_false() {
    let mut state = ParseState { token: None, pos: 0 };
    next("false", &mut state);
    assert_eq!(state.token.as_ref().unwrap().kind, TokenKind::False);
}

#[test]
fn test_tokenizer_quote() {
    let mut state = ParseState { token: None, pos: 0 };
    next("'(1 2)", &mut state);
    assert_eq!(state.token.as_ref().unwrap().kind, TokenKind::Quote);
}

#[test]
fn test_tokenizer_eof() {
    let mut state = ParseState { token: None, pos: 0 };
    next("", &mut state);
    assert_eq!(state.token.as_ref().unwrap().kind, TokenKind::Eof);
}

#[test]
fn test_tokenizer_comment_skip() {
    let mut state = ParseState { token: None, pos: 0 };
    next("; comment\n42", &mut state);
    assert_eq!(state.token.as_ref().unwrap().kind, TokenKind::Digit);
    assert_eq!(state.token.as_ref().unwrap().val, 42);
}

#[test]
fn test_tokenizer_multi_digit() {
    let mut state = ParseState { token: None, pos: 0 };
    next("123", &mut state);
    assert_eq!(state.token.as_ref().unwrap().kind, TokenKind::Digit);
    assert_eq!(state.token.as_ref().unwrap().val, 123);
}

#[test]
fn test_tokenizer_symbol_with_dash() {
    let mut state = ParseState { token: None, pos: 0 };
    // In C, '-' is an operator char, so "list-ref" tokenizes as one symbol
    // because isop('-') is true and isalnum handles the rest
    next("(list-ref x 0)", &mut state); // (
    next("(list-ref x 0)", &mut state); // list-ref
    assert_eq!(state.token.as_ref().unwrap().kind, TokenKind::Symbol);
    // The tokenizer reads alphanumeric + operator chars together
}

// ==================== Match Token Tests ====================

#[test]
fn test_match_token_positive() {
    let mut state = ParseState { token: None, pos: 0 };
    next("(", &mut state);
    assert_eq!(match_token(&mut state, TokenKind::LParen), 1);
}

#[test]
fn test_match_token_negative() {
    let mut state = ParseState { token: None, pos: 0 };
    next("(", &mut state);
    assert_eq!(match_token(&mut state, TokenKind::RParen), 0);
}

// ==================== Parser Tests ====================

#[test]
fn test_parse_simple_expression() {
    let mut state = ParseState { token: None, pos: 0 };
    let mut result = ParseResult { program: None };
    parse("(+ 1 2)", &mut state, &mut result);
    assert!(result.program.is_some());
    let prog = result.program.as_ref().unwrap();
    assert!(prog.expressions.is_some());
}

#[test]
fn test_parse_literal_integer() {
    let mut state = ParseState { token: None, pos: 0 };
    let mut result = ParseResult { program: None };
    parse("42", &mut state, &mut result);
    let prog = result.program.as_ref().unwrap();
    let expr = prog.expressions.as_ref().unwrap().expression.as_ref().unwrap();
    assert!(matches!(expr.type_, ExpressionType::Literal));
}

#[test]
fn test_parse_list_expression() {
    let mut state = ParseState { token: None, pos: 0 };
    let mut result = ParseResult { program: None };
    parse("'(1 2 3)", &mut state, &mut result);
    let prog = result.program.as_ref().unwrap();
    let expr = prog.expressions.as_ref().unwrap().expression.as_ref().unwrap();
    assert!(matches!(expr.type_, ExpressionType::List));
}

#[test]
fn test_parse_multiple_expressions() {
    let mut state = ParseState { token: None, pos: 0 };
    let mut result = ParseResult { program: None };
    parse("1 2 3", &mut state, &mut result);
    let prog = result.program.as_ref().unwrap();
    let first = prog.expressions.as_ref().unwrap();
    assert!(first.next.is_some());
    assert!(first.next.as_ref().unwrap().next.is_some());
}

// ==================== Stringify Tests ====================

#[test]
fn test_stringify_integer() {
    assert_eq!(eval_worsp("42"), "42");
}

#[test]
fn test_stringify_zero() {
    assert_eq!(eval_worsp("0"), "0");
}

#[test]
fn test_stringify_string() {
    assert_eq!(eval_worsp("\"hello\""), "hello");
}

#[test]
fn test_stringify_bool_true() {
    assert_eq!(eval_worsp("true"), "T");
}

#[test]
fn test_stringify_bool_false() {
    assert_eq!(eval_worsp("false"), "F");
}

#[test]
fn test_stringify_nil() {
    assert_eq!(eval_worsp("nil"), "nil");
}

#[test]
fn test_stringify_list() {
    assert_eq!(eval_worsp("'(1 2 3)"), "(1 2 3)");
}

#[test]
fn test_stringify_empty_list() {
    assert_eq!(eval_worsp("'()"), "nil");
}

// ==================== Arithmetic Tests ====================

#[test]
fn test_add() {
    assert_eq!(eval_worsp("(+ 10 20)"), "30");
}

#[test]
fn test_sub() {
    assert_eq!(eval_worsp("(- 50 17)"), "33");
}

#[test]
fn test_mul() {
    assert_eq!(eval_worsp("(* 6 7)"), "42");
}

#[test]
fn test_div() {
    assert_eq!(eval_worsp("(/ 100 4)"), "25");
}

#[test]
fn test_mod() {
    assert_eq!(eval_worsp("(% 17 5)"), "2");
}

#[test]
fn test_string_concat() {
    assert_eq!(eval_worsp("(+ \"foo\" \"bar\")"), "foobar");
}

#[test]
fn test_nested_arithmetic() {
    assert_eq!(eval_worsp("(+ (* 3 4) (- 10 5))"), "17");
}

// ==================== Comparison Tests ====================

#[test]
fn test_lt_true() {
    assert_eq!(eval_worsp("(< 1 2)"), "T");
}

#[test]
fn test_lt_false() {
    assert_eq!(eval_worsp("(< 2 1)"), "F");
}

#[test]
fn test_gt_true() {
    assert_eq!(eval_worsp("(> 5 3)"), "T");
}

#[test]
fn test_gt_false() {
    assert_eq!(eval_worsp("(> 3 5)"), "F");
}

#[test]
fn test_eq_int_true() {
    assert_eq!(eval_worsp("(eq 42 42)"), "T");
}

#[test]
fn test_eq_int_false() {
    assert_eq!(eval_worsp("(eq 1 2)"), "F");
}

#[test]
fn test_eq_string_true() {
    assert_eq!(eval_worsp("(eq \"abc\" \"abc\")"), "T");
}

#[test]
fn test_eq_string_false() {
    assert_eq!(eval_worsp("(eq \"abc\" \"def\")"), "F");
}

#[test]
fn test_eq_bool_true() {
    assert_eq!(eval_worsp("(eq true true)"), "T");
}

#[test]
fn test_eq_bool_false() {
    assert_eq!(eval_worsp("(eq true false)"), "F");
}

// ==================== Boolean Tests ====================

#[test]
fn test_not_true() {
    assert_eq!(eval_worsp("(not true)"), "F");
}

#[test]
fn test_not_false() {
    assert_eq!(eval_worsp("(not false)"), "T");
}

#[test]
fn test_or_true_false() {
    assert_eq!(eval_worsp("(|| true false)"), "T");
}

#[test]
fn test_or_false_false() {
    assert_eq!(eval_worsp("(|| false false)"), "F");
}

#[test]
fn test_and_true_true() {
    assert_eq!(eval_worsp("(&& true true)"), "T");
}

#[test]
fn test_and_true_false() {
    assert_eq!(eval_worsp("(&& true false)"), "F");
}

#[test]
fn test_or_multi_arg() {
    assert_eq!(eval_worsp("(|| false false true)"), "T");
}

#[test]
fn test_and_multi_arg() {
    assert_eq!(eval_worsp("(&& true true false)"), "F");
}

// ==================== List Operation Tests ====================

#[test]
fn test_car() {
    assert_eq!(eval_worsp("(car '(10 20 30))"), "10");
}

#[test]
fn test_cdr() {
    assert_eq!(eval_worsp("(cdr '(10 20 30))"), "(20 30)");
}

#[test]
fn test_cdr_two_elements() {
    assert_eq!(eval_worsp("(cdr '(10 20))"), "(20)");
}

#[test]
fn test_cons_with_list() {
    assert_eq!(eval_worsp("(cons 1 '(2 3))"), "(1 2 3)");
}

#[test]
fn test_cons_with_int() {
    assert_eq!(eval_worsp("(cons 1 2)"), "(1 2)");
}

#[test]
fn test_cons_with_nil() {
    assert_eq!(eval_worsp("(cons 1 nil)"), "(1)");
}

#[test]
fn test_list_ref_0() {
    assert_eq!(eval_worsp("(list-ref '(10 20 30) 0)"), "10");
}

#[test]
fn test_list_ref_1() {
    assert_eq!(eval_worsp("(list-ref '(10 20 30) 1)"), "20");
}

#[test]
fn test_list_ref_2() {
    assert_eq!(eval_worsp("(list-ref '(10 20 30) 2)"), "30");
}

#[test]
fn test_length_list() {
    assert_eq!(eval_worsp("(length '(1 2 3 4 5))"), "5");
}

#[test]
fn test_length_empty_list() {
    assert_eq!(eval_worsp("(length '())"), "0");
}

#[test]
fn test_length_string() {
    assert_eq!(eval_worsp("(length \"hello\")"), "5");
}

// ==================== String Operation Tests ====================

#[test]
fn test_remove_whitespaces() {
    assert_eq!(eval_worsp("(remove-whitespaces \"a b c\")"), "abc");
}

#[test]
fn test_is_int_string_yes() {
    assert_eq!(eval_worsp("(is-int-string \"123\")"), "T");
}

#[test]
fn test_is_int_string_no() {
    assert_eq!(eval_worsp("(is-int-string \"abc\")"), "F");
}

#[test]
fn test_is_int_string_empty() {
    assert_eq!(eval_worsp("(is-int-string \"\")"), "T");
}

#[test]
fn test_is_int_string_non_string() {
    assert_eq!(eval_worsp("(is-int-string 42)"), "F");
}

#[test]
fn test_parse_int() {
    assert_eq!(eval_worsp("(parse-int \"42\")"), "42");
}

#[test]
fn test_string_ref_first() {
    assert_eq!(eval_worsp("(string-ref \"hello\" 0)"), "h");
}

#[test]
fn test_string_ref_last() {
    assert_eq!(eval_worsp("(string-ref \"hello\" 4)"), "o");
}

#[test]
fn test_split_delimiter() {
    assert_eq!(eval_worsp("(split \"a,b,c\" \",\")"), "(a b c)");
}

// ==================== Control Flow Tests ====================

#[test]
fn test_if_true() {
    assert_eq!(eval_worsp("(if true 1 2)"), "1");
}

#[test]
fn test_if_false() {
    assert_eq!(eval_worsp("(if false 1 2)"), "2");
}

#[test]
fn test_if_no_else() {
    assert_eq!(eval_worsp("(if false 1)"), "nil");
}

#[test]
fn test_progn() {
    assert_eq!(eval_worsp("(progn 1 2 3)"), "3");
}

#[test]
fn test_while_loop() {
    assert_eq!(
        eval_worsp("(= i 0) (= s 0) (while (< i 5) (progn (= s (+ s i)) (= i (+ i 1)))) s"),
        "10"
    );
}

// ==================== Assignment and Variable Tests ====================

#[test]
fn test_assignment() {
    assert_eq!(eval_worsp("(= x 42) x"), "42");
}

#[test]
fn test_assignment_overwrite() {
    assert_eq!(eval_worsp("(= x 1) (= x 2) x"), "2");
}

// ==================== Defun Tests ====================

#[test]
fn test_defun_simple() {
    assert_eq!(eval_worsp("(defun add1 (n) (+ n 1)) (add1 5)"), "6");
}

#[test]
fn test_defun_recursive() {
    assert_eq!(
        eval_worsp("(defun fact (n) (if (eq n 0) 1 (* n (fact (- n 1))))) (fact 5)"),
        "120"
    );
}

// ==================== Pop Tests ====================

#[test]
fn test_pop_single() {
    assert_eq!(eval_worsp("(pop '(42))"), "42");
}

#[test]
fn test_pop_multi() {
    assert_eq!(eval_worsp("(pop '(1 2 3))"), "3");
}

#[test]
fn test_pop_empty() {
    assert_eq!(eval_worsp("(pop '())"), "nil");
}

// ==================== Push Tests ====================

#[test]
fn test_push_to_list() {
    assert_eq!(eval_worsp("(= a '(1 2 3)) (push a 4) a"), "(1 2 3 4)");
}

#[test]
fn test_push_to_empty() {
    assert_eq!(eval_worsp("(= a '()) (push a 1) (push a 2) (push a 3) a"), "(1 2 3)");
}

// ==================== Snapshot Fixture Tests ====================

#[test]
fn test_fixture_simple_print() {
    assert_eq!(eval_worsp("\"Hello World!\""), "Hello World!");
}

#[test]
fn test_fixture_calc() {
    assert_eq!(eval_worsp("(= a 1) (= b 2) (= c (+ a b)) c"), "3");
}

#[test]
fn test_fixture_length_for_string() {
    assert_eq!(eval_worsp("(= foo \"foobar\") (length foo)"), "6");
}

#[test]
fn test_fixture_remove_whitespaces() {
    assert_eq!(eval_worsp("(remove-whitespaces \"foo   bar\")"), "foobar");
}

#[test]
fn test_fixture_parse_int() {
    assert_eq!(eval_worsp("(= foo \"35\") (= bar (parse-int foo)) (+ bar 5)"), "40");
}

#[test]
fn test_fixture_is_int_string_false() {
    assert_eq!(eval_worsp("(= foo \"foo\") (is-int-string foo)"), "F");
}

#[test]
fn test_fixture_is_int_string_true() {
    assert_eq!(eval_worsp("(= foo \"345\") (is-int-string foo)"), "T");
}

#[test]
fn test_fixture_string_ref() {
    assert_eq!(eval_worsp("(= foo \"abcdefg\") (string-ref foo 3)"), "d");
}

#[test]
fn test_fixture_pop_multi() {
    assert_eq!(eval_worsp("(= list '(1 2 3 4)) (pop list) (pop list) (pop list)"), "2");
}

#[test]
fn test_fixture_length_empty() {
    assert_eq!(eval_worsp("(length '())"), "0");
}

#[test]
fn test_fixture_length_list() {
    assert_eq!(eval_worsp("(= a '(1 2 3)) (length a)"), "3");
}

#[test]
fn test_empty_sexp() {
    // Empty s-expression evaluates to nil
    assert_eq!(eval_worsp("()"), "nil");
}

// ==================== Evaluate function Tests ====================

#[test]
fn test_evaluate_function() {
    let mut state = ParseState { token: None, pos: 0 };
    let mut result = ParseResult { program: None };
    parse("(+ 1 2)", &mut state, &mut result);
    // evaluate() doesn't return a value, just runs the program
    // This should not panic
    evaluate(&mut result);
}

// ==================== Init Tests ====================

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
    assert_eq!(ctx.gc_less_mode, 1);
    assert!(ctx.stack.is_some());
}

#[test]
fn test_allocate() {
    let mut env = Env {
        bindings: std::array::from_fn(|_| Binding { symbol_name: String::new(), value: None }),
        parent: None,
    };
    init_env(&mut env);
    let mut ctx = init_allocator();
    let obj = allocate(&mut ctx, &mut env);
    assert!(obj.is_some());
}

// ==================== evaluate_expression_with_context Tests ====================

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

fn main() {}
