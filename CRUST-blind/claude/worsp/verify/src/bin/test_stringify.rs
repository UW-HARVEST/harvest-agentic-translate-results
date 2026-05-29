use worsp::worsp::*;

#[allow(dead_code)]
fn empty_env() -> Env {
    let mut env = Env {
        bindings: std::array::from_fn(|_| Binding {
            symbol_name: String::new(),
            value: None,
        }),
        parent: None,
    };
    init_env(&mut env);
    env
}

#[allow(dead_code)]
fn empty_object() -> Object {
    Object {
        marked: false,
        type_: ObjectType::Nil,
        value: ObjectValue::IntValue(0),
    }
}

#[allow(dead_code)]
fn eval_one(src: &str) -> Object {
    let mut state = ParseState { token: None, pos: 0 };
    let mut result = ParseResult { program: None };
    parse(src, &mut state, &mut result);
    let program = result.program.unwrap();
    let exprs = program.expressions.unwrap();
    let expr = exprs.expression.as_deref().unwrap().clone();
    let mut env = empty_env();
    let mut obj = empty_object();
    evaluate_expression_with_context(&expr, &mut obj, &mut env);
    obj
}

#[test]
fn test_stringify_int_zero() {
    let obj = Object {
        marked: false,
        type_: ObjectType::Integer,
        value: ObjectValue::IntValue(0),
    };
    assert_eq!(stringify_object(&obj), "0");
}

#[test]
fn test_stringify_int_positive() {
    let obj = Object {
        marked: false,
        type_: ObjectType::Integer,
        value: ObjectValue::IntValue(42),
    };
    assert_eq!(stringify_object(&obj), "42");
}

#[test]
fn test_stringify_int_large() {
    let obj = Object {
        marked: false,
        type_: ObjectType::Integer,
        value: ObjectValue::IntValue(2000000),
    };
    assert_eq!(stringify_object(&obj), "2000000");
}

#[test]
fn test_stringify_string() {
    let obj = Object {
        marked: false,
        type_: ObjectType::String,
        value: ObjectValue::StringValue("Hello World!".to_string()),
    };
    assert_eq!(stringify_object(&obj), "Hello World!");
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
fn test_stringify_quoted_list_of_ints() {
    // matches calc behavior of '(1 2 3)
    let obj = eval_one("'(1 2 3)");
    assert_eq!(stringify_object(&obj), "(1 2 3)");
}

#[test]
fn test_stringify_quoted_list_of_strings() {
    // C output for '("1st" "2nd" "3rd") used in map.wsp produced "(1st 2nd 3rd)"
    let obj = eval_one(r#"'("1st" "2nd" "3rd")"#);
    assert_eq!(stringify_object(&obj), "(1st 2nd 3rd)");
}

#[test]
fn test_stringify_cons_list() {
    // From C: (cons 1 '(2 3)) = (1 2 3)
    let obj = eval_one("(cons 1 '(2 3))");
    assert_eq!(stringify_object(&obj), "(1 2 3)");
}

#[test]
fn test_stringify_cons_with_nil() {
    // From C: (cons 1 nil) = (1)
    let obj = eval_one("(cons 1 nil)");
    assert_eq!(stringify_object(&obj), "(1)");
}

#[test]
fn test_stringify_cons_with_scalar() {
    // From C: (cons 1 2) = (1 2)
    let obj = eval_one("(cons 1 2)");
    assert_eq!(stringify_object(&obj), "(1 2)");
}

fn main() {}
