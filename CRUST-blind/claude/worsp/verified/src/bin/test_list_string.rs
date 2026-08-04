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

// length
#[test]
fn test_length_empty_list_is_zero() {
    let obj = eval_one("(length '())");
    assert!(matches!(obj.type_, ObjectType::Integer));
    match obj.value {
        ObjectValue::IntValue(v) => assert_eq!(v, 0),
        _ => panic!(),
    }
}

#[test]
fn test_length_list_of_three() {
    let obj = eval_one("(length '(1 2 3))");
    match obj.value {
        ObjectValue::IntValue(v) => assert_eq!(v, 3),
        _ => panic!(),
    }
}

#[test]
fn test_length_string() {
    let obj = eval_one(r#"(length "foobar")"#);
    match obj.value {
        ObjectValue::IntValue(v) => assert_eq!(v, 6),
        _ => panic!(),
    }
}

#[test]
fn test_length_empty_string() {
    let obj = eval_one(r#"(length "")"#);
    match obj.value {
        ObjectValue::IntValue(v) => assert_eq!(v, 0),
        _ => panic!(),
    }
}

// car/cdr
#[test]
fn test_car_returns_first_int() {
    let obj = eval_one("(car '(1 2 3))");
    assert!(matches!(obj.type_, ObjectType::Integer));
    match obj.value {
        ObjectValue::IntValue(v) => assert_eq!(v, 1),
        _ => panic!(),
    }
}

#[test]
fn test_cdr_returns_rest() {
    let obj = eval_one("(cdr '(1 2 3))");
    assert!(matches!(obj.type_, ObjectType::List));
    assert_eq!(stringify_object(&obj), "(2 3)");
}

// list-ref
#[test]
fn test_list_ref_first() {
    let obj = eval_one("(list-ref '(10 20 30) 0)");
    match obj.value {
        ObjectValue::IntValue(v) => assert_eq!(v, 10),
        _ => panic!(),
    }
}

#[test]
fn test_list_ref_middle() {
    let obj = eval_one("(list-ref '(10 20 30) 1)");
    match obj.value {
        ObjectValue::IntValue(v) => assert_eq!(v, 20),
        _ => panic!(),
    }
}

#[test]
fn test_list_ref_last() {
    let obj = eval_one("(list-ref '(10 20 30) 2)");
    match obj.value {
        ObjectValue::IntValue(v) => assert_eq!(v, 30),
        _ => panic!(),
    }
}

// pop
#[test]
fn test_pop_empty_list_returns_nil() {
    let obj = eval_one("(pop '())");
    assert!(matches!(obj.type_, ObjectType::Nil));
}

#[test]
fn test_pop_singleton() {
    let obj = eval_one("(pop '(1))");
    match obj.value {
        ObjectValue::IntValue(v) => assert_eq!(v, 1),
        _ => panic!(),
    }
}

#[test]
fn test_pop_returns_last_element() {
    // Mirrors pop-1.wsp but verifies the actual returned value
    let obj = eval_program("(= list '(1 2 3 4)) (pop list) (pop list) (pop list)");
    assert!(matches!(obj.type_, ObjectType::Integer));
    match obj.value {
        ObjectValue::IntValue(v) => assert_eq!(v, 2),
        _ => panic!(),
    }
}

// push
#[test]
fn test_push_to_existing_list() {
    let obj = eval_program("(= a '(1 2 3)) (push a 4) a");
    assert!(matches!(obj.type_, ObjectType::List));
    assert_eq!(stringify_object(&obj), "(1 2 3 4)");
}

#[test]
fn test_push_to_nil() {
    let obj = eval_program("(= a '()) (push a 1) (push a 2) (push a 3) a");
    assert_eq!(stringify_object(&obj), "(1 2 3)");
}

// split
#[test]
fn test_split_empty_separator() {
    // From C output: (split "abc" "") returns list of single-char strings ("a" "b" "c")
    let obj = eval_one(r#"(split "abc" "")"#);
    assert!(matches!(obj.type_, ObjectType::List));
    // Verify length is 3 via `length`
    let len = eval_one(r#"(length (split "abc" ""))"#);
    match len.value {
        ObjectValue::IntValue(v) => assert_eq!(v, 3),
        _ => panic!(),
    }
}

#[test]
fn test_split_first_char() {
    let obj = eval_one(r#"(car (split "abc" ""))"#);
    assert!(matches!(obj.type_, ObjectType::String));
    match obj.value {
        ObjectValue::StringValue(v) => assert_eq!(v, "a"),
        _ => panic!(),
    }
}

#[test]
fn test_split_with_separator() {
    let obj = eval_one(r#"(length (split "a,b,c" ","))"#);
    match obj.value {
        ObjectValue::IntValue(v) => assert_eq!(v, 3),
        _ => panic!(),
    }
}

#[test]
fn test_split_first_chunk() {
    let obj = eval_one(r#"(car (split "a,b,c" ","))"#);
    match obj.value {
        ObjectValue::StringValue(v) => assert_eq!(v, "a"),
        _ => panic!(),
    }
}

// string-ref
#[test]
fn test_string_ref_returns_single_char_string() {
    let obj = eval_one(r#"(string-ref "abcdefg" 3)"#);
    assert!(matches!(obj.type_, ObjectType::String));
    match obj.value {
        ObjectValue::StringValue(v) => assert_eq!(v, "d"),
        _ => panic!(),
    }
}

#[test]
fn test_string_ref_first() {
    let obj = eval_one(r#"(string-ref "abcdefg" 0)"#);
    match obj.value {
        ObjectValue::StringValue(v) => assert_eq!(v, "a"),
        _ => panic!(),
    }
}

// remove-whitespaces
#[test]
fn test_remove_whitespaces_basic() {
    let obj = eval_one(r#"(remove-whitespaces "foo   bar")"#);
    assert!(matches!(obj.type_, ObjectType::String));
    match obj.value {
        ObjectValue::StringValue(v) => assert_eq!(v, "foobar"),
        _ => panic!(),
    }
}

#[test]
fn test_remove_whitespaces_empty() {
    let obj = eval_one(r#"(remove-whitespaces "")"#);
    match obj.value {
        ObjectValue::StringValue(v) => assert_eq!(v, ""),
        _ => panic!(),
    }
}

#[test]
fn test_remove_whitespaces_no_spaces() {
    let obj = eval_one(r#"(remove-whitespaces "abc")"#);
    match obj.value {
        ObjectValue::StringValue(v) => assert_eq!(v, "abc"),
        _ => panic!(),
    }
}

// is-int-string
#[test]
fn test_is_int_string_true() {
    let obj = eval_one(r#"(is-int-string "345")"#);
    match obj.value {
        ObjectValue::BoolValue(v) => assert_eq!(v, 1),
        _ => panic!(),
    }
}

#[test]
fn test_is_int_string_false_letters() {
    let obj = eval_one(r#"(is-int-string "foo")"#);
    match obj.value {
        ObjectValue::BoolValue(v) => assert_eq!(v, 0),
        _ => panic!(),
    }
}

#[test]
fn test_is_int_string_non_string() {
    // C returns false for non-strings
    let obj = eval_one("(is-int-string 1)");
    match obj.value {
        ObjectValue::BoolValue(v) => assert_eq!(v, 0),
        _ => panic!(),
    }
}

// parse-int
#[test]
fn test_parse_int_basic() {
    let obj = eval_one(r#"(parse-int "35")"#);
    assert!(matches!(obj.type_, ObjectType::Integer));
    match obj.value {
        ObjectValue::IntValue(v) => assert_eq!(v, 35),
        _ => panic!(),
    }
}

#[test]
fn test_parse_int_then_add() {
    let obj = eval_program(r#"(= foo "35") (= bar (parse-int foo)) (+ bar 5)"#);
    match obj.value {
        ObjectValue::IntValue(v) => assert_eq!(v, 40),
        _ => panic!(),
    }
}

#[test]
fn test_splice_function() {
    // Mirror splice.wsp but return the result
    let obj = eval_program(
        r#"(defun string-ref-fn (str index) (progn (list-ref (split str "") index)))
           (defun splice (str start end)
             (progn
               (= result "")
               (= index start)
               (while (not (eq index end))
                 (progn
                   (= result (+ result (string-ref-fn str index)))
                   (= index (+ index 1))))
               (progn result)))
           (splice "foobar" 2 4)"#,
    );
    assert!(matches!(obj.type_, ObjectType::String));
    match obj.value {
        ObjectValue::StringValue(v) => assert_eq!(v, "ob"),
        _ => panic!(),
    }
}

#[test]
fn test_map_program() {
    // Mirrors map.wsp
    let obj = eval_program(
        r#"(= list '("1st" "2nd" "3rd"))
           (= list-length (length list))
           (= results nil)
           (= i 0)
           (while (< i list-length)
             (progn
               (push results (list-ref list i))
               (= i (+ i 1))))
           results"#,
    );
    assert!(matches!(obj.type_, ObjectType::List));
    assert_eq!(stringify_object(&obj), "(1st 2nd 3rd)");
}

#[test]
fn test_evaluate_full_program() {
    // Use the higher-level `evaluate` function. Just verifying it runs without error.
    let mut state = ParseState { token: None, pos: 0 };
    let mut result = ParseResult { program: None };
    parse("(= a 1) (= b (+ a 2))", &mut state, &mut result);
    evaluate(&mut result);
}

#[test]
fn test_init_env_clears_bindings() {
    let mut env = Env {
        bindings: std::array::from_fn(|_| Binding {
            symbol_name: "stale".to_string(),
            value: Some(Box::new(Object {
                marked: false,
                type_: ObjectType::Integer,
                value: ObjectValue::IntValue(99),
            })),
        }),
        parent: Some(Box::new(empty_env())),
    };
    init_env(&mut env);
    assert!(env.parent.is_none());
    for i in 0..MAX_BINDINGS {
        assert_eq!(env.bindings[i].symbol_name, "");
        assert!(env.bindings[i].value.is_none());
    }
}

#[test]
fn test_init_allocator_basics() {
    let ctx = init_allocator();
    assert_eq!(ctx.gc_less_mode, 0);
    let stack = ctx.stack.as_ref().unwrap();
    assert_eq!(stack.top, -1);
    for byte in ctx.free_bitmap.iter() {
        assert_eq!(*byte, 0);
    }
}

#[test]
fn test_allocate_returns_some_object() {
    let mut ctx = init_allocator();
    let mut env = empty_env();
    let obj = allocate(&mut ctx, &mut env);
    assert!(obj.is_some());
}

fn main() {}
