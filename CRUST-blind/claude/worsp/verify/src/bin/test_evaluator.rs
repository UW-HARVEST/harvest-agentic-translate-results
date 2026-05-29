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

/// Parse `src` as a single expression and evaluate it in a fresh env.
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

/// Parse `src` (multiple expressions) and evaluate them sequentially in one env, returning the
/// final result.
#[allow(dead_code)]
fn eval_program(src: &str) -> Object {
    let mut state = ParseState { token: None, pos: 0 };
    let mut result = ParseResult { program: None };
    parse(src, &mut state, &mut result);
    let program = result.program.unwrap();
    let mut env = empty_env();
    let mut context = init_allocator();
    let mut last = empty_object();
    let mut cur = program.expressions.as_deref();
    while let Some(node) = cur {
        if let Some(e) = &node.expression {
            let expr = (**e).clone();
            let mut obj = empty_object();
            evaluate_expression(&expr, &mut obj, &mut env, &mut context);
            last = obj;
        }
        cur = node.next.as_deref();
    }
    last
}

#[test]
fn test_eval_int_literal() {
    let obj = eval_one("42");
    assert!(matches!(obj.type_, ObjectType::Integer));
    match obj.value {
        ObjectValue::IntValue(v) => assert_eq!(v, 42),
        _ => panic!(),
    }
}

#[test]
fn test_eval_zero() {
    let obj = eval_one("0");
    assert!(matches!(obj.type_, ObjectType::Integer));
    match obj.value {
        ObjectValue::IntValue(v) => assert_eq!(v, 0),
        _ => panic!(),
    }
}

#[test]
fn test_eval_string_literal() {
    let obj = eval_one(r#""hello""#);
    assert!(matches!(obj.type_, ObjectType::String));
    match obj.value {
        ObjectValue::StringValue(v) => assert_eq!(v, "hello"),
        _ => panic!(),
    }
}

#[test]
fn test_eval_true() {
    let obj = eval_one("true");
    assert!(matches!(obj.type_, ObjectType::Bool));
    match obj.value {
        ObjectValue::BoolValue(v) => assert_eq!(v, 1),
        _ => panic!(),
    }
}

#[test]
fn test_eval_false() {
    let obj = eval_one("false");
    assert!(matches!(obj.type_, ObjectType::Bool));
    match obj.value {
        ObjectValue::BoolValue(v) => assert_eq!(v, 0),
        _ => panic!(),
    }
}

#[test]
fn test_eval_nil_symbol() {
    let obj = eval_one("nil");
    assert!(matches!(obj.type_, ObjectType::Nil));
}

#[test]
fn test_eval_addition() {
    let obj = eval_one("(+ 1 2)");
    assert!(matches!(obj.type_, ObjectType::Integer));
    match obj.value {
        ObjectValue::IntValue(v) => assert_eq!(v, 3),
        _ => panic!(),
    }
}

#[test]
fn test_eval_subtraction() {
    let obj = eval_one("(- 5 3)");
    assert!(matches!(obj.type_, ObjectType::Integer));
    match obj.value {
        ObjectValue::IntValue(v) => assert_eq!(v, 2),
        _ => panic!(),
    }
}

#[test]
fn test_eval_subtraction_negative() {
    let obj = eval_one("(- 0 5)");
    assert!(matches!(obj.type_, ObjectType::Integer));
    match obj.value {
        ObjectValue::IntValue(v) => assert_eq!(v, -5),
        _ => panic!(),
    }
}

#[test]
fn test_eval_multiplication() {
    let obj = eval_one("(* 4 5)");
    match obj.value {
        ObjectValue::IntValue(v) => assert_eq!(v, 20),
        _ => panic!(),
    }
}

#[test]
fn test_eval_division() {
    let obj = eval_one("(/ 10 3)");
    match obj.value {
        ObjectValue::IntValue(v) => assert_eq!(v, 3),
        _ => panic!(),
    }
}

#[test]
fn test_eval_modulo() {
    let obj = eval_one("(% 10 3)");
    match obj.value {
        ObjectValue::IntValue(v) => assert_eq!(v, 1),
        _ => panic!(),
    }
}

#[test]
fn test_eval_string_concat() {
    let obj = eval_one(r#"(+ "foo" "bar")"#);
    assert!(matches!(obj.type_, ObjectType::String));
    match obj.value {
        ObjectValue::StringValue(v) => assert_eq!(v, "foobar"),
        _ => panic!(),
    }
}

#[test]
fn test_eval_lt_true() {
    let obj = eval_one("(< 1 2)");
    assert!(matches!(obj.type_, ObjectType::Bool));
    match obj.value {
        ObjectValue::BoolValue(v) => assert_eq!(v, 1),
        _ => panic!(),
    }
}

#[test]
fn test_eval_lt_false() {
    let obj = eval_one("(< 2 1)");
    match obj.value {
        ObjectValue::BoolValue(v) => assert_eq!(v, 0),
        _ => panic!(),
    }
}

#[test]
fn test_eval_gt_true() {
    let obj = eval_one("(> 5 3)");
    match obj.value {
        ObjectValue::BoolValue(v) => assert_eq!(v, 1),
        _ => panic!(),
    }
}

#[test]
fn test_eval_gt_false() {
    let obj = eval_one("(> 1 2)");
    match obj.value {
        ObjectValue::BoolValue(v) => assert_eq!(v, 0),
        _ => panic!(),
    }
}

#[test]
fn test_eval_eq_int_true() {
    let obj = eval_one("(eq 1 1)");
    assert!(matches!(obj.type_, ObjectType::Bool));
    match obj.value {
        ObjectValue::BoolValue(v) => assert_eq!(v, 1),
        _ => panic!(),
    }
}

#[test]
fn test_eval_eq_int_false() {
    let obj = eval_one("(eq 1 2)");
    match obj.value {
        ObjectValue::BoolValue(v) => assert_eq!(v, 0),
        _ => panic!(),
    }
}

#[test]
fn test_eval_eq_str_true() {
    let obj = eval_one(r#"(eq "a" "a")"#);
    match obj.value {
        ObjectValue::BoolValue(v) => assert_eq!(v, 1),
        _ => panic!(),
    }
}

#[test]
fn test_eval_eq_str_false() {
    let obj = eval_one(r#"(eq "a" "b")"#);
    match obj.value {
        ObjectValue::BoolValue(v) => assert_eq!(v, 0),
        _ => panic!(),
    }
}

#[test]
fn test_eval_eq_nil_nil() {
    let obj = eval_one("(eq nil nil)");
    match obj.value {
        ObjectValue::BoolValue(v) => assert_eq!(v, 1),
        _ => panic!(),
    }
}

#[test]
fn test_eval_eq_diff_types() {
    let obj = eval_one(r#"(eq 1 "1")"#);
    match obj.value {
        ObjectValue::BoolValue(v) => assert_eq!(v, 0),
        _ => panic!(),
    }
}

#[test]
fn test_eval_not_true() {
    let obj = eval_one("(not true)");
    assert!(matches!(obj.type_, ObjectType::Bool));
    match obj.value {
        ObjectValue::BoolValue(v) => assert_eq!(v, 0),
        _ => panic!(),
    }
}

#[test]
fn test_eval_not_false() {
    let obj = eval_one("(not false)");
    match obj.value {
        ObjectValue::BoolValue(v) => assert_eq!(v, 1),
        _ => panic!(),
    }
}

#[test]
fn test_eval_or_true() {
    let obj = eval_one("(|| false false true)");
    match obj.value {
        ObjectValue::BoolValue(v) => assert_eq!(v, 1),
        _ => panic!(),
    }
}

#[test]
fn test_eval_or_all_false() {
    let obj = eval_one("(|| false false false)");
    match obj.value {
        ObjectValue::BoolValue(v) => assert_eq!(v, 0),
        _ => panic!(),
    }
}

#[test]
fn test_eval_and_false() {
    let obj = eval_one("(&& true true false)");
    match obj.value {
        ObjectValue::BoolValue(v) => assert_eq!(v, 0),
        _ => panic!(),
    }
}

#[test]
fn test_eval_and_all_true() {
    let obj = eval_one("(&& true true true)");
    match obj.value {
        ObjectValue::BoolValue(v) => assert_eq!(v, 1),
        _ => panic!(),
    }
}

#[test]
fn test_eval_if_then() {
    let obj = eval_one("(if true 1 2)");
    match obj.value {
        ObjectValue::IntValue(v) => assert_eq!(v, 1),
        _ => panic!(),
    }
}

#[test]
fn test_eval_if_else() {
    let obj = eval_one("(if false 1 2)");
    match obj.value {
        ObjectValue::IntValue(v) => assert_eq!(v, 2),
        _ => panic!(),
    }
}

#[test]
fn test_eval_if_no_else_returns_nil() {
    let obj = eval_one("(if false 1)");
    assert!(matches!(obj.type_, ObjectType::Nil));
}

#[test]
fn test_eval_progn_returns_last() {
    let obj = eval_one("(progn 1 2 3)");
    match obj.value {
        ObjectValue::IntValue(v) => assert_eq!(v, 3),
        _ => panic!(),
    }
}

#[test]
fn test_eval_assignment_and_lookup() {
    let obj = eval_program("(= a 5) a");
    assert!(matches!(obj.type_, ObjectType::Integer));
    match obj.value {
        ObjectValue::IntValue(v) => assert_eq!(v, 5),
        _ => panic!(),
    }
}

#[test]
fn test_eval_calc_program() {
    // Mirrors c_src/snapshot/fixtures/calc.wsp
    let obj = eval_program("(= a 1) (= b 2) (= c (+ a b)) c");
    match obj.value {
        ObjectValue::IntValue(v) => assert_eq!(v, 3),
        _ => panic!(),
    }
}

#[test]
fn test_eval_factorial() {
    let obj = eval_program(
        "(defun fact (n) (if (eq n 0) 1 (* n (fact (- n 1))))) (fact 5)",
    );
    match obj.value {
        ObjectValue::IntValue(v) => assert_eq!(v, 120),
        _ => panic!(),
    }
}

#[test]
fn test_eval_while_loop() {
    // Counter to 5: result accumulates 5x "while... "
    let obj = eval_program(
        r#"(= counter 5) (= result "") (while (not (eq counter 0)) (progn (= result (+ result "while... ")) (= counter (- counter 1)))) result"#,
    );
    assert!(matches!(obj.type_, ObjectType::String));
    match obj.value {
        ObjectValue::StringValue(v) => {
            assert_eq!(v, "while... while... while... while... while... ")
        }
        _ => panic!(),
    }
}

#[test]
fn test_eval_empty_quote_is_nil() {
    let obj = eval_one("'()");
    assert!(matches!(obj.type_, ObjectType::Nil));
}

#[test]
fn test_eval_quoted_list_size() {
    let obj = eval_one("'(1 2 3)");
    assert!(matches!(obj.type_, ObjectType::List));
}

#[test]
fn test_eval_function_call_simple() {
    let obj = eval_program("(defun add1 (n) (+ n 1)) (add1 41)");
    match obj.value {
        ObjectValue::IntValue(v) => assert_eq!(v, 42),
        _ => panic!(),
    }
}

fn main() {}
