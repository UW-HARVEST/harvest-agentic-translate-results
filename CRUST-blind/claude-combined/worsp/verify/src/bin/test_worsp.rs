use worsp::worsp::*;

fn make_state() -> ParseState {
    ParseState {
        token: None,
        pos: 0,
    }
}

fn make_result() -> ParseResult {
    ParseResult { program: None }
}

fn make_env() -> Env {
    let mut env = Env {
        bindings: [
            Binding { symbol_name: String::new(), value: None },
            Binding { symbol_name: String::new(), value: None },
            Binding { symbol_name: String::new(), value: None },
            Binding { symbol_name: String::new(), value: None },
            Binding { symbol_name: String::new(), value: None },
            Binding { symbol_name: String::new(), value: None },
            Binding { symbol_name: String::new(), value: None },
            Binding { symbol_name: String::new(), value: None },
            Binding { symbol_name: String::new(), value: None },
            Binding { symbol_name: String::new(), value: None },
        ],
        parent: None,
    };
    init_env(&mut env);
    env
}

fn make_obj() -> Object {
    Object {
        marked: false,
        type_: ObjectType::Nil,
        value: ObjectValue::IntValue(0),
    }
}

// ============================================================
//   Tokenizer tests
// ============================================================

#[test]
fn test_next_single_char_symbol() {
    let mut state = make_state();
    next("a", &mut state);
    let tok = state.token.as_ref().unwrap();
    assert_eq!(tok.kind, TokenKind::Symbol);
    assert_eq!(tok.str, "a");
}

#[test]
fn test_next_multiple_char_symbol() {
    let mut state = make_state();
    next("aaaa", &mut state);
    let tok = state.token.as_ref().unwrap();
    assert_eq!(tok.kind, TokenKind::Symbol);
    assert_eq!(tok.str, "aaaa");
}

#[test]
fn test_next_paren_and_digit() {
    let source = "(1)";
    let mut state = make_state();
    next(source, &mut state);
    assert_eq!(state.token.as_ref().unwrap().kind, TokenKind::LParen);
    next(source, &mut state);
    assert_eq!(state.token.as_ref().unwrap().kind, TokenKind::Digit);
    assert_eq!(state.token.as_ref().unwrap().val, 1);
    next(source, &mut state);
    assert_eq!(state.token.as_ref().unwrap().kind, TokenKind::RParen);
}

#[test]
fn test_next_string() {
    let source = "\"hello\" () 1 \"foo\" \"bar\"";
    let mut state = make_state();
    next(source, &mut state);
    assert_eq!(state.token.as_ref().unwrap().kind, TokenKind::String);
    assert_eq!(state.token.as_ref().unwrap().str, "hello");
    next(source, &mut state);
    assert_eq!(state.token.as_ref().unwrap().kind, TokenKind::LParen);
    next(source, &mut state);
    assert_eq!(state.token.as_ref().unwrap().kind, TokenKind::RParen);
    next(source, &mut state);
    assert_eq!(state.token.as_ref().unwrap().kind, TokenKind::Digit);
    assert_eq!(state.token.as_ref().unwrap().val, 1);
    next(source, &mut state);
    assert_eq!(state.token.as_ref().unwrap().kind, TokenKind::String);
    assert_eq!(state.token.as_ref().unwrap().str, "foo");
    next(source, &mut state);
    assert_eq!(state.token.as_ref().unwrap().kind, TokenKind::String);
    assert_eq!(state.token.as_ref().unwrap().str, "bar");
}

#[test]
fn test_next_add_op() {
    let source = "(+ 1 2)";
    let mut state = make_state();
    next(source, &mut state);
    assert_eq!(state.token.as_ref().unwrap().kind, TokenKind::LParen);
    next(source, &mut state);
    assert_eq!(state.token.as_ref().unwrap().kind, TokenKind::Symbol);
    assert_eq!(state.token.as_ref().unwrap().str, "+");
    next(source, &mut state);
    assert_eq!(state.token.as_ref().unwrap().kind, TokenKind::Digit);
    assert_eq!(state.token.as_ref().unwrap().val, 1);
    next(source, &mut state);
    assert_eq!(state.token.as_ref().unwrap().kind, TokenKind::Digit);
    assert_eq!(state.token.as_ref().unwrap().val, 2);
    next(source, &mut state);
    assert_eq!(state.token.as_ref().unwrap().kind, TokenKind::RParen);
}

#[test]
fn test_next_quote() {
    let source = "'(1 2 3)";
    let mut state = make_state();
    next(source, &mut state);
    assert_eq!(state.token.as_ref().unwrap().kind, TokenKind::Quote);
    next(source, &mut state);
    assert_eq!(state.token.as_ref().unwrap().kind, TokenKind::LParen);
}

#[test]
fn test_next_true_false() {
    let source = "true false";
    let mut state = make_state();
    next(source, &mut state);
    assert_eq!(state.token.as_ref().unwrap().kind, TokenKind::True);
    next(source, &mut state);
    assert_eq!(state.token.as_ref().unwrap().kind, TokenKind::False);
}

#[test]
fn test_next_eof() {
    let source = "";
    let mut state = make_state();
    next(source, &mut state);
    assert_eq!(state.token.as_ref().unwrap().kind, TokenKind::Eof);
}

// ============================================================
//   match_token tests
// ============================================================

#[test]
fn test_match_token_match() {
    let mut state = make_state();
    next("a", &mut state);
    assert_eq!(match_token(&mut state, TokenKind::Symbol), 1);
}

#[test]
fn test_match_token_no_match() {
    let mut state = make_state();
    next("a", &mut state);
    assert_eq!(match_token(&mut state, TokenKind::Digit), 0);
}

// ============================================================
//   Parser tests
// ============================================================

#[test]
fn test_parse_int_literal() {
    let source = "3";
    let mut state = make_state();
    let mut result = make_result();
    parse(source, &mut state, &mut result);
    let prog = result.program.as_ref().unwrap();
    let exprs = prog.expressions.as_ref().unwrap();
    let expr = exprs.expression.as_ref().unwrap();
    assert!(matches!(expr.type_, ExpressionType::Literal));
    if let ExpressionData::Literal(Some(lit)) = &expr.data {
        assert!(matches!(lit.type_, LiteralType::Integer));
        if let LiteralValue::IntValue(v) = &lit.value {
            assert_eq!(*v, 3);
        } else {
            panic!("expected int");
        }
    } else {
        panic!("expected literal");
    }
}

#[test]
fn test_parse_string_literal() {
    let source = "\"foo\"";
    let mut state = make_state();
    let mut result = make_result();
    parse(source, &mut state, &mut result);
    let exprs = result.program.as_ref().unwrap().expressions.as_ref().unwrap();
    let expr = exprs.expression.as_ref().unwrap();
    if let ExpressionData::Literal(Some(lit)) = &expr.data {
        assert!(matches!(lit.type_, LiteralType::String));
        if let LiteralValue::StringValue(s) = &lit.value {
            assert_eq!(s, "foo");
        } else {
            panic!("expected string");
        }
    } else {
        panic!("expected literal");
    }
}

#[test]
fn test_parse_multiple_literals() {
    let source = "3 \"foo\"";
    let mut state = make_state();
    let mut result = make_result();
    parse(source, &mut state, &mut result);
    let prog = result.program.as_ref().unwrap();
    let exprs = prog.expressions.as_ref().unwrap();
    let first = exprs.expression.as_ref().unwrap();
    if let ExpressionData::Literal(Some(lit)) = &first.data {
        if let LiteralValue::IntValue(v) = &lit.value {
            assert_eq!(*v, 3);
        }
    }
    let next_node = exprs.next.as_ref().unwrap();
    let second = next_node.expression.as_ref().unwrap();
    if let ExpressionData::Literal(Some(lit)) = &second.data {
        if let LiteralValue::StringValue(s) = &lit.value {
            assert_eq!(s, "foo");
        }
    }
}

#[test]
fn test_parse_symbolic_expr() {
    let source = "(1 2 3)";
    let mut state = make_state();
    let mut result = make_result();
    parse(source, &mut state, &mut result);
    let exprs = result.program.as_ref().unwrap().expressions.as_ref().unwrap();
    let expr = exprs.expression.as_ref().unwrap();
    assert!(matches!(expr.type_, ExpressionType::SymbolicExp));
    assert_eq!(match_token(&mut state, TokenKind::Eof), 1);
}

#[test]
fn test_parse_list_expr() {
    let source = "'(1 2 3)";
    let mut state = make_state();
    let mut result = make_result();
    parse(source, &mut state, &mut result);
    let exprs = result.program.as_ref().unwrap().expressions.as_ref().unwrap();
    let expr = exprs.expression.as_ref().unwrap();
    assert!(matches!(expr.type_, ExpressionType::List));
}

// ============================================================
//   evaluate tests
// ============================================================

fn parse_and_eval(source: &str) -> Object {
    let mut state = make_state();
    let mut result = make_result();
    parse(source, &mut state, &mut result);
    let prog = result.program.as_ref().unwrap();
    let exprs = prog.expressions.as_ref().unwrap();
    let expr = exprs.expression.as_ref().unwrap();
    let mut evaluated = make_obj();
    let mut env = make_env();
    evaluate_expression_with_context(expr, &mut evaluated, &mut env);
    evaluated
}

#[test]
fn test_eval_int_literal() {
    let evaluated = parse_and_eval("3");
    assert!(matches!(evaluated.type_, ObjectType::Integer));
    if let ObjectValue::IntValue(v) = &evaluated.value {
        assert_eq!(*v, 3);
    }
}

#[test]
fn test_eval_string_literal() {
    let evaluated = parse_and_eval("\"foo\"");
    assert!(matches!(evaluated.type_, ObjectType::String));
    if let ObjectValue::StringValue(s) = &evaluated.value {
        assert_eq!(s, "foo");
    }
}

#[test]
fn test_eval_nil() {
    let evaluated = parse_and_eval("nil");
    assert!(matches!(evaluated.type_, ObjectType::Nil));
}

#[test]
fn test_eval_empty_list() {
    let evaluated = parse_and_eval("'()");
    assert!(matches!(evaluated.type_, ObjectType::Nil));
}

#[test]
fn test_eval_empty_symbolic() {
    let evaluated = parse_and_eval("()");
    assert!(matches!(evaluated.type_, ObjectType::Nil));
}

#[test]
fn test_eval_list_with_int() {
    let evaluated = parse_and_eval("'(133)");
    assert!(matches!(evaluated.type_, ObjectType::List));
    if let ObjectValue::ListValue(Some(cc)) = &evaluated.value {
        let car = cc.car.as_ref().unwrap();
        assert!(matches!(car.type_, ObjectType::Integer));
        if let ObjectValue::IntValue(v) = &car.value {
            assert_eq!(*v, 133);
        }
        let cdr = cc.cdr.as_ref().unwrap();
        assert!(matches!(cdr.type_, ObjectType::Nil));
    } else {
        panic!("expected list");
    }
}

#[test]
fn test_eval_add() {
    let e = parse_and_eval("(+ 1222 21)");
    assert!(matches!(e.type_, ObjectType::Integer));
    if let ObjectValue::IntValue(v) = &e.value {
        assert_eq!(*v, 1243);
    }
}

#[test]
fn test_eval_sub() {
    let e = parse_and_eval("(- 1222 21)");
    if let ObjectValue::IntValue(v) = &e.value {
        assert_eq!(*v, 1201);
    }
}

#[test]
fn test_eval_mul() {
    let e = parse_and_eval("(* 1222 21)");
    if let ObjectValue::IntValue(v) = &e.value {
        assert_eq!(*v, 25662);
    }
}

#[test]
fn test_eval_div() {
    let e = parse_and_eval("(/ 1222 21)");
    if let ObjectValue::IntValue(v) = &e.value {
        assert_eq!(*v, 58);
    }
}

#[test]
fn test_eval_mod() {
    let e = parse_and_eval("(% 1222 21)");
    if let ObjectValue::IntValue(v) = &e.value {
        assert_eq!(*v, 4);
    }
}

#[test]
fn test_eval_or_true() {
    let e = parse_and_eval("(|| true false)");
    assert!(matches!(e.type_, ObjectType::Bool));
    if let ObjectValue::BoolValue(v) = &e.value {
        assert_eq!(*v, 1);
    }
}

#[test]
fn test_eval_or_false() {
    let e = parse_and_eval("(|| false false)");
    if let ObjectValue::BoolValue(v) = &e.value {
        assert_eq!(*v, 0);
    }
}

#[test]
fn test_eval_or_nil() {
    let e = parse_and_eval("(|| 1 nil)");
    if let ObjectValue::BoolValue(v) = &e.value {
        assert_eq!(*v, 1);
    }
}

#[test]
fn test_eval_and_false() {
    let e = parse_and_eval("(&& false false)");
    if let ObjectValue::BoolValue(v) = &e.value {
        assert_eq!(*v, 0);
    }
}

#[test]
fn test_eval_and_true() {
    let e = parse_and_eval("(&& true true)");
    if let ObjectValue::BoolValue(v) = &e.value {
        assert_eq!(*v, 1);
    }
}

#[test]
fn test_eval_lt_true() {
    let e = parse_and_eval("(< 1 2)");
    if let ObjectValue::BoolValue(v) = &e.value {
        assert_eq!(*v, 1);
    }
}

#[test]
fn test_eval_lt_false() {
    let e = parse_and_eval("(< 2 1)");
    if let ObjectValue::BoolValue(v) = &e.value {
        assert_eq!(*v, 0);
    }
}

#[test]
fn test_eval_gt_true() {
    let e = parse_and_eval("(> 2 1)");
    if let ObjectValue::BoolValue(v) = &e.value {
        assert_eq!(*v, 1);
    }
}

#[test]
fn test_eval_gt_false() {
    let e = parse_and_eval("(> 1 2)");
    if let ObjectValue::BoolValue(v) = &e.value {
        assert_eq!(*v, 0);
    }
}

#[test]
fn test_eval_not_true() {
    let e = parse_and_eval("(not false)");
    if let ObjectValue::BoolValue(v) = &e.value {
        assert_eq!(*v, 1);
    }
}

#[test]
fn test_eval_not_false() {
    let e = parse_and_eval("(not true)");
    if let ObjectValue::BoolValue(v) = &e.value {
        assert_eq!(*v, 0);
    }
}

#[test]
fn test_eval_eq_true() {
    let e = parse_and_eval("(eq 1 1)");
    if let ObjectValue::BoolValue(v) = &e.value {
        assert_eq!(*v, 1);
    }
}

#[test]
fn test_eval_eq_false() {
    let e = parse_and_eval("(eq 1 2)");
    if let ObjectValue::BoolValue(v) = &e.value {
        assert_eq!(*v, 0);
    }
}

#[test]
fn test_eval_eq_string() {
    let e = parse_and_eval("(eq \"foo\" \"foo\")");
    if let ObjectValue::BoolValue(v) = &e.value {
        assert_eq!(*v, 1);
    }
}

#[test]
fn test_eval_nested_ops() {
    let e = parse_and_eval("(+ 1 (- 2 (* 3 (/ 4 2))))");
    if let ObjectValue::IntValue(v) = &e.value {
        assert_eq!(*v, -3);
    }
}

#[test]
fn test_eval_string_concat() {
    let e = parse_and_eval("(+ \"foo\" \"bar\")");
    assert!(matches!(e.type_, ObjectType::String));
    if let ObjectValue::StringValue(s) = &e.value {
        assert_eq!(s, "foobar");
    }
}

#[test]
fn test_eval_if_true() {
    let e = parse_and_eval("(if true 1 2)");
    if let ObjectValue::IntValue(v) = &e.value {
        assert_eq!(*v, 1);
    }
}

#[test]
fn test_eval_if_false() {
    let e = parse_and_eval("(if false 1 2)");
    if let ObjectValue::IntValue(v) = &e.value {
        assert_eq!(*v, 2);
    }
}

#[test]
fn test_eval_if_no_else() {
    let e = parse_and_eval("(if false 1)");
    assert!(matches!(e.type_, ObjectType::Nil));
}

// ============================================================
//   stringify_object tests
// ============================================================

#[test]
fn test_stringify_int() {
    let obj = Object {
        marked: false,
        type_: ObjectType::Integer,
        value: ObjectValue::IntValue(42),
    };
    assert_eq!(stringify_object(&obj), "42");
}

#[test]
fn test_stringify_zero() {
    let obj = Object {
        marked: false,
        type_: ObjectType::Integer,
        value: ObjectValue::IntValue(0),
    };
    assert_eq!(stringify_object(&obj), "0");
}

#[test]
fn test_stringify_string() {
    let obj = Object {
        marked: false,
        type_: ObjectType::String,
        value: ObjectValue::StringValue("hello".to_string()),
    };
    assert_eq!(stringify_object(&obj), "hello");
}

#[test]
fn test_stringify_bool_true() {
    let obj = Object {
        marked: false,
        type_: ObjectType::Bool,
        value: ObjectValue::BoolValue(1),
    };
    assert_eq!(stringify_object(&obj), "T");
}

#[test]
fn test_stringify_bool_false() {
    let obj = Object {
        marked: false,
        type_: ObjectType::Bool,
        value: ObjectValue::BoolValue(0),
    };
    assert_eq!(stringify_object(&obj), "F");
}

#[test]
fn test_stringify_nil() {
    let obj = Object {
        marked: false,
        type_: ObjectType::Nil,
        value: ObjectValue::IntValue(0),
    };
    assert_eq!(stringify_object(&obj), "nil");
}

#[test]
fn test_stringify_function() {
    let obj = Object {
        marked: false,
        type_: ObjectType::Function,
        value: ObjectValue::FunctionValue(None),
    };
    assert_eq!(stringify_object(&obj), "<function>");
}

#[test]
fn test_stringify_list() {
    // Build a list (1 2 3)
    let e = parse_and_eval("'(1 2 3)");
    assert_eq!(stringify_object(&e), "(1 2 3)");
}

#[test]
fn test_stringify_list_strings() {
    let e = parse_and_eval("'(\"1st\" \"2nd\" \"3rd\")");
    assert_eq!(stringify_object(&e), "(1st 2nd 3rd)");
}

// ============================================================
//   init_env tests
// ============================================================

#[test]
fn test_init_env() {
    let mut env = make_env();
    init_env(&mut env);
    assert!(env.parent.is_none());
    assert_eq!(env.bindings[0].symbol_name, "");
    assert!(env.bindings[0].value.is_none());
}

// ============================================================
//   init_allocator tests
// ============================================================

#[test]
fn test_init_allocator() {
    let ctx = init_allocator();
    assert_eq!(ctx.gc_less_mode, 0);
    assert!(ctx.stack.is_some());
    let stack = ctx.stack.as_ref().unwrap();
    assert_eq!(stack.top, -1);
    for &b in ctx.free_bitmap.iter() {
        assert_eq!(b, 0);
    }
}

// ============================================================
//   allocate tests
// ============================================================

#[test]
fn test_allocate() {
    let mut ctx = init_allocator();
    let mut env = make_env();
    let obj = allocate(&mut ctx, &mut env);
    assert!(obj.is_some());
}

// ============================================================
//   evaluate (full program) tests
// ============================================================

fn parse_program(source: &str) -> ParseResult {
    let mut state = make_state();
    let mut result = make_result();
    parse(source, &mut state, &mut result);
    result
}

#[test]
fn test_evaluate_program() {
    // The 'evaluate' function runs program with side effects (print). We just check it doesn't crash.
    let mut result = parse_program("(= a 1)\n(= b 2)\n(= c (+ a b))");
    evaluate(&mut result);
}

#[test]
fn test_eval_assignment() {
    let mut state = make_state();
    let mut result = make_result();
    parse("(progn (= a 5) a)", &mut state, &mut result);
    let prog = result.program.as_ref().unwrap();
    let exprs = prog.expressions.as_ref().unwrap();
    let expr = exprs.expression.as_ref().unwrap();
    let mut evaluated = make_obj();
    let mut env = make_env();
    evaluate_expression_with_context(expr, &mut evaluated, &mut env);
    assert!(matches!(evaluated.type_, ObjectType::Integer));
    if let ObjectValue::IntValue(v) = &evaluated.value {
        assert_eq!(*v, 5);
    }
}

#[test]
fn test_eval_progn() {
    let e = parse_and_eval("(progn 1 2 3)");
    assert!(matches!(e.type_, ObjectType::Integer));
    if let ObjectValue::IntValue(v) = &e.value {
        assert_eq!(*v, 3);
    }
}

#[test]
fn test_eval_progn_empty_returns_nil() {
    let e = parse_and_eval("(progn)");
    assert!(matches!(e.type_, ObjectType::Nil));
}

#[test]
fn test_eval_car() {
    let e = parse_and_eval("(car '(1 2 3))");
    assert!(matches!(e.type_, ObjectType::Integer));
    if let ObjectValue::IntValue(v) = &e.value {
        assert_eq!(*v, 1);
    }
}

#[test]
fn test_eval_cdr() {
    let e = parse_and_eval("(cdr '(1 2 3))");
    assert!(matches!(e.type_, ObjectType::List));
}

#[test]
fn test_eval_cons() {
    let e = parse_and_eval("(cons 1 '(2 3))");
    assert!(matches!(e.type_, ObjectType::List));
    assert_eq!(stringify_object(&e), "(1 2 3)");
}

#[test]
fn test_eval_length_list() {
    let e = parse_and_eval("(length '(1 2 3))");
    assert!(matches!(e.type_, ObjectType::Integer));
    if let ObjectValue::IntValue(v) = &e.value {
        assert_eq!(*v, 3);
    }
}

#[test]
fn test_eval_length_string() {
    let e = parse_and_eval("(length \"foobar\")");
    if let ObjectValue::IntValue(v) = &e.value {
        assert_eq!(*v, 6);
    }
}

#[test]
fn test_eval_length_nil() {
    let e = parse_and_eval("(length nil)");
    if let ObjectValue::IntValue(v) = &e.value {
        assert_eq!(*v, 0);
    }
}

#[test]
fn test_eval_is_int_string_true() {
    let e = parse_and_eval("(is-int-string \"123\")");
    if let ObjectValue::BoolValue(v) = &e.value {
        assert_eq!(*v, 1);
    }
}

#[test]
fn test_eval_is_int_string_false() {
    let e = parse_and_eval("(is-int-string \"12a\")");
    if let ObjectValue::BoolValue(v) = &e.value {
        assert_eq!(*v, 0);
    }
}

#[test]
fn test_eval_parse_int() {
    let e = parse_and_eval("(parse-int \"40\")");
    if let ObjectValue::IntValue(v) = &e.value {
        assert_eq!(*v, 40);
    }
}

#[test]
fn test_eval_remove_whitespaces() {
    let e = parse_and_eval("(remove-whitespaces \"foo bar\")");
    if let ObjectValue::StringValue(s) = &e.value {
        assert_eq!(s, "foobar");
    }
}

#[test]
fn test_eval_string_ref() {
    let e = parse_and_eval("(string-ref \"abcdef\" 1)");
    assert!(matches!(e.type_, ObjectType::String));
    if let ObjectValue::StringValue(s) = &e.value {
        assert_eq!(s, "b");
    }
}

#[test]
fn test_eval_pop_returns_last() {
    // pop returns the last element of a list (the C semantic)
    let e = parse_and_eval("(pop '(1 2 3))");
    assert!(matches!(e.type_, ObjectType::Integer));
    if let ObjectValue::IntValue(v) = &e.value {
        assert_eq!(*v, 3);
    }
}

#[test]
fn test_eval_pop_nil() {
    let e = parse_and_eval("(pop nil)");
    assert!(matches!(e.type_, ObjectType::Nil));
}

#[test]
fn test_eval_list_ref() {
    let e = parse_and_eval("(list-ref '(10 20 30) 1)");
    assert!(matches!(e.type_, ObjectType::Integer));
    if let ObjectValue::IntValue(v) = &e.value {
        assert_eq!(*v, 20);
    }
}

#[test]
fn test_eval_split_empty_separator() {
    let e = parse_and_eval("(split \"abc\" \"\")");
    assert!(matches!(e.type_, ObjectType::List));
    assert_eq!(stringify_object(&e), "(a b c)");
}

#[test]
fn test_eval_user_function_factorial() {
    let mut state = make_state();
    let mut result = make_result();
    parse(
        "(progn (defun fact (n) (if (eq n 0) 1 (* n (fact (- n 1))))) (fact 5))",
        &mut state,
        &mut result,
    );
    let prog = result.program.as_ref().unwrap();
    let exprs = prog.expressions.as_ref().unwrap();
    let expr = exprs.expression.as_ref().unwrap();
    let mut evaluated = make_obj();
    let mut env = make_env();
    evaluate_expression_with_context(expr, &mut evaluated, &mut env);
    assert!(matches!(evaluated.type_, ObjectType::Integer));
    if let ObjectValue::IntValue(v) = &evaluated.value {
        assert_eq!(*v, 120);
    }
}

#[test]
fn test_eval_while() {
    let mut state = make_state();
    let mut result = make_result();
    parse(
        "(progn (= i 0) (= total 0) (while (< i 5) (progn (= total (+ total i)) (= i (+ i 1)))) total)",
        &mut state,
        &mut result,
    );
    let prog = result.program.as_ref().unwrap();
    let exprs = prog.expressions.as_ref().unwrap();
    let expr = exprs.expression.as_ref().unwrap();
    let mut evaluated = make_obj();
    let mut env = make_env();
    evaluate_expression_with_context(expr, &mut evaluated, &mut env);
    assert!(matches!(evaluated.type_, ObjectType::Integer));
    if let ObjectValue::IntValue(v) = &evaluated.value {
        assert_eq!(*v, 10);
    }
}

fn main() {}
