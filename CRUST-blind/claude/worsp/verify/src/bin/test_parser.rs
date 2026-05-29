use worsp::worsp::*;

#[allow(dead_code)]
fn new_state() -> ParseState {
    ParseState { token: None, pos: 0 }
}

#[test]
fn test_parse_empty_program() {
    let mut state = ParseState { token: None, pos: 0 };
    let mut result = ParseResult { program: None };
    parse("", &mut state, &mut result);
    let program = result.program.unwrap();
    assert!(program.expressions.is_none());
}

#[test]
fn test_parse_single_int_literal() {
    let mut state = ParseState { token: None, pos: 0 };
    let mut result = ParseResult { program: None };
    parse("42", &mut state, &mut result);
    let program = result.program.unwrap();
    let exprs = program.expressions.unwrap();
    let expr = exprs.expression.unwrap();
    assert!(matches!(expr.type_, ExpressionType::Literal));
    if let ExpressionData::Literal(Some(lit)) = &expr.data {
        assert!(matches!(lit.type_, LiteralType::Integer));
        match &lit.value {
            LiteralValue::IntValue(v) => assert_eq!(*v, 42),
            _ => panic!("Expected int value"),
        }
    } else {
        panic!("Expected literal");
    }
    assert!(exprs.next.is_none());
}

#[test]
fn test_parse_string_literal() {
    let mut state = ParseState { token: None, pos: 0 };
    let mut result = ParseResult { program: None };
    parse(r#""hello""#, &mut state, &mut result);
    let program = result.program.unwrap();
    let exprs = program.expressions.unwrap();
    let expr = exprs.expression.unwrap();
    if let ExpressionData::Literal(Some(lit)) = &expr.data {
        assert!(matches!(lit.type_, LiteralType::String));
        match &lit.value {
            LiteralValue::StringValue(v) => assert_eq!(v, "hello"),
            _ => panic!("Expected string value"),
        }
    } else {
        panic!("Expected literal");
    }
}

#[test]
fn test_parse_true_literal() {
    let mut state = ParseState { token: None, pos: 0 };
    let mut result = ParseResult { program: None };
    parse("true", &mut state, &mut result);
    let program = result.program.unwrap();
    let exprs = program.expressions.unwrap();
    let expr = exprs.expression.unwrap();
    if let ExpressionData::Literal(Some(lit)) = &expr.data {
        assert!(matches!(lit.type_, LiteralType::Boolean));
        match &lit.value {
            LiteralValue::BooleanValue(v) => assert_eq!(*v, true),
            _ => panic!("Expected boolean"),
        }
    } else {
        panic!("Expected literal");
    }
}

#[test]
fn test_parse_false_literal() {
    let mut state = ParseState { token: None, pos: 0 };
    let mut result = ParseResult { program: None };
    parse("false", &mut state, &mut result);
    let program = result.program.unwrap();
    let exprs = program.expressions.unwrap();
    let expr = exprs.expression.unwrap();
    if let ExpressionData::Literal(Some(lit)) = &expr.data {
        match &lit.value {
            LiteralValue::BooleanValue(v) => assert_eq!(*v, false),
            _ => panic!("Expected boolean"),
        }
    } else {
        panic!("Expected literal");
    }
}

#[test]
fn test_parse_symbol() {
    let mut state = ParseState { token: None, pos: 0 };
    let mut result = ParseResult { program: None };
    parse("foo", &mut state, &mut result);
    let program = result.program.unwrap();
    let exprs = program.expressions.unwrap();
    let expr = exprs.expression.unwrap();
    assert!(matches!(expr.type_, ExpressionType::Symbol));
    if let ExpressionData::Symbol(Some(sym)) = &expr.data {
        assert_eq!(sym.symbol_name, "foo");
    } else {
        panic!("Expected symbol");
    }
}

#[test]
fn test_parse_symbolic_expression() {
    let mut state = ParseState { token: None, pos: 0 };
    let mut result = ParseResult { program: None };
    parse("(+ 1 2)", &mut state, &mut result);
    let program = result.program.unwrap();
    let exprs = program.expressions.unwrap();
    let expr = exprs.expression.unwrap();
    assert!(matches!(expr.type_, ExpressionType::SymbolicExp));
    if let ExpressionData::SymbolicExp(Some(se)) = &expr.data {
        // Walk: '+', 1, 2
        let lst = se.expressions.as_ref().unwrap();
        // first: '+'
        let head = lst.expression.as_ref().unwrap();
        assert!(matches!(head.type_, ExpressionType::Symbol));
        if let ExpressionData::Symbol(Some(s)) = &head.data {
            assert_eq!(s.symbol_name, "+");
        } else { panic!() }

        let n2 = lst.next.as_ref().unwrap();
        let arg1 = n2.expression.as_ref().unwrap();
        assert!(matches!(arg1.type_, ExpressionType::Literal));
        if let ExpressionData::Literal(Some(lit)) = &arg1.data {
            match &lit.value {
                LiteralValue::IntValue(v) => assert_eq!(*v, 1),
                _ => panic!(),
            }
        } else { panic!() }

        let n3 = n2.next.as_ref().unwrap();
        let arg2 = n3.expression.as_ref().unwrap();
        if let ExpressionData::Literal(Some(lit)) = &arg2.data {
            match &lit.value {
                LiteralValue::IntValue(v) => assert_eq!(*v, 2),
                _ => panic!(),
            }
        } else { panic!() }
        assert!(n3.next.is_none());
    } else {
        panic!("Expected symbolic expression");
    }
}

#[test]
fn test_parse_list_expression() {
    let mut state = ParseState { token: None, pos: 0 };
    let mut result = ParseResult { program: None };
    parse("'(1 2 3)", &mut state, &mut result);
    let program = result.program.unwrap();
    let exprs = program.expressions.unwrap();
    let expr = exprs.expression.unwrap();
    assert!(matches!(expr.type_, ExpressionType::List));
    if let ExpressionData::List(Some(lnode)) = &expr.data {
        let lst = lnode.expressions.as_ref().unwrap();
        let mut count = 0;
        let mut cur = Some(lst.as_ref());
        while let Some(node) = cur {
            count += 1;
            cur = node.next.as_deref();
        }
        assert_eq!(count, 3);
    } else {
        panic!("Expected list");
    }
}

#[test]
fn test_parse_empty_list() {
    let mut state = ParseState { token: None, pos: 0 };
    let mut result = ParseResult { program: None };
    parse("'()", &mut state, &mut result);
    let program = result.program.unwrap();
    let exprs = program.expressions.unwrap();
    let expr = exprs.expression.unwrap();
    assert!(matches!(expr.type_, ExpressionType::List));
    if let ExpressionData::List(Some(lnode)) = &expr.data {
        assert!(lnode.expressions.is_none());
    } else {
        panic!("Expected list");
    }
}

#[test]
fn test_parse_multiple_expressions() {
    let mut state = ParseState { token: None, pos: 0 };
    let mut result = ParseResult { program: None };
    parse("(= a 1) (= b 2)", &mut state, &mut result);
    let program = result.program.unwrap();
    let mut count = 0;
    let mut cur = program.expressions.as_deref();
    while let Some(n) = cur {
        count += 1;
        cur = n.next.as_deref();
    }
    assert_eq!(count, 2);
}

#[test]
fn test_parse_nested_sexp() {
    let mut state = ParseState { token: None, pos: 0 };
    let mut result = ParseResult { program: None };
    parse("(+ (* 2 3) 4)", &mut state, &mut result);
    let program = result.program.unwrap();
    let exprs = program.expressions.unwrap();
    let expr = exprs.expression.unwrap();
    if let ExpressionData::SymbolicExp(Some(se)) = &expr.data {
        let lst = se.expressions.as_ref().unwrap();
        let n2 = lst.next.as_ref().unwrap();
        let arg1 = n2.expression.as_ref().unwrap();
        // Inner is a symbolic exp (* 2 3)
        assert!(matches!(arg1.type_, ExpressionType::SymbolicExp));
    } else {
        panic!();
    }
}

fn main() {}
