use cJSON::cjson::{parse, CJson};

#[test]
fn test_parse_null() {
    let v = parse("null", true).unwrap();
    assert_eq!(v, CJson::Null);
}

#[test]
fn test_parse_true() {
    let v = parse("true", true).unwrap();
    assert_eq!(v, CJson::Bool(true));
}

#[test]
fn test_parse_false() {
    let v = parse("false", true).unwrap();
    assert_eq!(v, CJson::Bool(false));
}

#[test]
fn test_parse_int() {
    let v = parse("42", true).unwrap();
    assert_eq!(v, CJson::Number(42.0));
}

#[test]
fn test_parse_negative_int() {
    let v = parse("-1", true).unwrap();
    assert_eq!(v, CJson::Number(-1.0));
}

#[test]
fn test_parse_zero() {
    let v = parse("0", true).unwrap();
    assert_eq!(v, CJson::Number(0.0));
}

#[test]
fn test_parse_float() {
    let v = parse("3.14", true).unwrap();
    match v {
        CJson::Number(n) => assert!((n - 3.14).abs() < 1e-10),
        _ => panic!("expected number"),
    }
}

#[test]
fn test_parse_negative_float() {
    let v = parse("-1.5", true).unwrap();
    match v {
        CJson::Number(n) => assert!((n - (-1.5)).abs() < 1e-10),
        _ => panic!("expected number"),
    }
}

#[test]
fn test_parse_exp_positive() {
    let v = parse("1e10", true).unwrap();
    match v {
        CJson::Number(n) => assert_eq!(n, 1e10),
        _ => panic!("expected number"),
    }
}

#[test]
fn test_parse_exp_negative() {
    let v = parse("1e-10", true).unwrap();
    match v {
        CJson::Number(n) => assert!((n - 1e-10).abs() < 1e-20),
        _ => panic!("expected number"),
    }
}

#[test]
fn test_parse_string_simple() {
    let v = parse("\"hello\"", true).unwrap();
    assert_eq!(v, CJson::String("hello".to_string()));
}

#[test]
fn test_parse_string_with_escape() {
    let v = parse("\"with\\nnewline\"", true).unwrap();
    assert_eq!(v, CJson::String("with\nnewline".to_string()));
}

#[test]
fn test_parse_string_escapes() {
    let v = parse("\"a\\tb\\\\c\\\"d\"", true).unwrap();
    assert_eq!(v, CJson::String("a\tb\\c\"d".to_string()));
}

#[test]
fn test_parse_empty_array() {
    let v = parse("[]", true).unwrap();
    assert_eq!(v, CJson::Array(vec![]));
}

#[test]
fn test_parse_array_ints() {
    let v = parse("[1,2,3]", true).unwrap();
    assert_eq!(
        v,
        CJson::Array(vec![CJson::Number(1.0), CJson::Number(2.0), CJson::Number(3.0)])
    );
}

#[test]
fn test_parse_nested_array() {
    let v = parse("[[1,2],[3,4]]", true).unwrap();
    assert_eq!(
        v,
        CJson::Array(vec![
            CJson::Array(vec![CJson::Number(1.0), CJson::Number(2.0)]),
            CJson::Array(vec![CJson::Number(3.0), CJson::Number(4.0)])
        ])
    );
}

#[test]
fn test_parse_empty_object() {
    let v = parse("{}", true).unwrap();
    match v {
        CJson::Object(m) => assert_eq!(m.len(), 0),
        _ => panic!("expected object"),
    }
}

#[test]
fn test_parse_object() {
    let v = parse("{\"a\":1}", true).unwrap();
    match &v {
        CJson::Object(m) => {
            assert_eq!(m.len(), 1);
            assert_eq!(m.get("a"), Some(&CJson::Number(1.0)));
        }
        _ => panic!("expected object"),
    }
}

#[test]
fn test_parse_object_multiple() {
    let v = parse("{\"a\":1,\"b\":\"hi\"}", true).unwrap();
    match &v {
        CJson::Object(m) => {
            assert_eq!(m.len(), 2);
            assert_eq!(m.get("a"), Some(&CJson::Number(1.0)));
            assert_eq!(m.get("b"), Some(&CJson::String("hi".to_string())));
        }
        _ => panic!("expected object"),
    }
}

#[test]
fn test_parse_with_whitespace() {
    let v = parse("  [ 1 , 2 , 3 ]  ", true).unwrap();
    assert_eq!(
        v,
        CJson::Array(vec![CJson::Number(1.0), CJson::Number(2.0), CJson::Number(3.0)])
    );
}

#[test]
fn test_parse_garbage_no_require() {
    // C: cJSON_ParseWithOpts("42 garbage", &end, 0) -> succeeds with value 42
    let v = parse("42 garbage", false).unwrap();
    assert_eq!(v, CJson::Number(42.0));
}

#[test]
fn test_parse_garbage_with_require_fails() {
    // C: cJSON_ParseWithOpts("42 garbage", &end, 1) -> fails
    let r = parse("42 garbage", true);
    assert!(r.is_err());
}

#[test]
fn test_parse_invalid() {
    assert!(parse("nope", true).is_err());
    assert!(parse("[1,]", true).is_err());
    assert!(parse("", true).is_err());
}

#[test]
fn test_print_unformatted_int() {
    // C: cJSON_PrintUnformatted(42) -> "42"
    let v = CJson::Number(42.0);
    assert_eq!(v.print_unformatted(), "42");
}

#[test]
fn test_print_unformatted_neg_int() {
    // C: -1 -> "-1"
    let v = CJson::Number(-1.0);
    assert_eq!(v.print_unformatted(), "-1");
}

#[test]
fn test_print_unformatted_zero() {
    // C: 0 -> "0"
    let v = CJson::Number(0.0);
    assert_eq!(v.print_unformatted(), "0");
}

#[test]
fn test_print_unformatted_float() {
    // C: 3.14 -> "3.140000"
    let v = CJson::Number(3.14);
    assert_eq!(v.print_unformatted(), "3.140000");
}

#[test]
fn test_print_unformatted_neg_float() {
    // C: -1.5 -> "-1.500000"
    let v = CJson::Number(-1.5);
    assert_eq!(v.print_unformatted(), "-1.500000");
}

#[test]
fn test_print_unformatted_large_int() {
    // C: 1e10 -> "10000000000"
    let v = CJson::Number(1e10);
    assert_eq!(v.print_unformatted(), "10000000000");
}

#[test]
fn test_print_unformatted_small_exp() {
    // C: 1e-10 -> "1.000000e-10"
    let v = CJson::Number(1e-10);
    assert_eq!(v.print_unformatted(), "1.000000e-10");
}

#[test]
fn test_print_unformatted_specific() {
    // C: -122.3959 -> "-122.395900"
    let v = CJson::Number(-122.395900);
    assert_eq!(v.print_unformatted(), "-122.395900");
}

#[test]
fn test_print_unformatted_long_decimal() {
    // C: 1.234567 -> "1.234567"
    let v = CJson::Number(1.234567);
    assert_eq!(v.print_unformatted(), "1.234567");
}

#[test]
fn test_print_unformatted_int_from_double() {
    // C: 123456789.0 -> "123456789"
    let v = CJson::Number(123456789.0);
    assert_eq!(v.print_unformatted(), "123456789");
}

#[test]
fn test_print_unformatted_null() {
    // C: null -> "null"
    let v = CJson::Null;
    assert_eq!(v.print_unformatted(), "null");
}

#[test]
fn test_print_unformatted_true() {
    // C: true -> "true"
    let v = CJson::Bool(true);
    assert_eq!(v.print_unformatted(), "true");
}

#[test]
fn test_print_unformatted_false() {
    // C: false -> "false"
    let v = CJson::Bool(false);
    assert_eq!(v.print_unformatted(), "false");
}

#[test]
fn test_print_unformatted_string() {
    // C: "hello" -> "\"hello\""
    let v = CJson::String("hello".to_string());
    assert_eq!(v.print_unformatted(), "\"hello\"");
}

#[test]
fn test_print_unformatted_string_with_escape() {
    // C: "with\nnewline" -> "\"with\\nnewline\""
    let v = CJson::String("with\nnewline".to_string());
    assert_eq!(v.print_unformatted(), "\"with\\nnewline\"");
}

#[test]
fn test_print_unformatted_string_escapes() {
    let v = CJson::String("a\tb".to_string());
    assert_eq!(v.print_unformatted(), "\"a\\tb\"");
    let v = CJson::String("c\"d".to_string());
    assert_eq!(v.print_unformatted(), "\"c\\\"d\"");
    let v = CJson::String("a\\b".to_string());
    assert_eq!(v.print_unformatted(), "\"a\\\\b\"");
}

#[test]
fn test_print_unformatted_array_empty() {
    let v = CJson::Array(vec![]);
    assert_eq!(v.print_unformatted(), "[]");
}

#[test]
fn test_print_unformatted_array_simple() {
    // C: [1,2,3] -> "[1,2,3]"
    let v = CJson::Array(vec![CJson::Number(1.0), CJson::Number(2.0), CJson::Number(3.0)]);
    assert_eq!(v.print_unformatted(), "[1,2,3]");
}

#[test]
fn test_print_unformatted_array_nested() {
    // C: [[1,2],[3,4]] -> "[[1,2],[3,4]]"
    let v = CJson::Array(vec![
        CJson::Array(vec![CJson::Number(1.0), CJson::Number(2.0)]),
        CJson::Array(vec![CJson::Number(3.0), CJson::Number(4.0)]),
    ]);
    assert_eq!(v.print_unformatted(), "[[1,2],[3,4]]");
}

#[test]
fn test_print_unformatted_object_empty() {
    let v = CJson::Object(std::collections::HashMap::new());
    assert_eq!(v.print_unformatted(), "{}");
}

#[test]
fn test_print_unformatted_object_simple() {
    // C: {"a":1} -> "{\"a\":1}"
    let mut m = std::collections::HashMap::new();
    m.insert("a".to_string(), CJson::Number(1.0));
    let v = CJson::Object(m);
    assert_eq!(v.print_unformatted(), "{\"a\":1}");
}

#[test]
fn test_print_formatted_null() {
    // C: cJSON_Print(null) -> "null"
    assert_eq!(CJson::Null.print_formatted(), "null");
}

#[test]
fn test_print_formatted_true() {
    assert_eq!(CJson::Bool(true).print_formatted(), "true");
}

#[test]
fn test_print_formatted_false() {
    assert_eq!(CJson::Bool(false).print_formatted(), "false");
}

#[test]
fn test_print_formatted_number() {
    assert_eq!(CJson::Number(42.0).print_formatted(), "42");
    assert_eq!(CJson::Number(3.14).print_formatted(), "3.140000");
}

#[test]
fn test_print_formatted_string() {
    assert_eq!(CJson::String("hello".to_string()).print_formatted(), "\"hello\"");
}

#[test]
fn test_print_formatted_array_empty() {
    assert_eq!(CJson::Array(vec![]).print_formatted(), "[]");
}

#[test]
fn test_print_formatted_array_simple() {
    // C: [1,2,3] -> "[1, 2, 3]"
    let v = CJson::Array(vec![CJson::Number(1.0), CJson::Number(2.0), CJson::Number(3.0)]);
    assert_eq!(v.print_formatted(), "[1, 2, 3]");
}

#[test]
fn test_print_formatted_nested_array() {
    // C: [[1,2],[3,4]] -> "[[1, 2], [3, 4]]"
    let v = CJson::Array(vec![
        CJson::Array(vec![CJson::Number(1.0), CJson::Number(2.0)]),
        CJson::Array(vec![CJson::Number(3.0), CJson::Number(4.0)]),
    ]);
    assert_eq!(v.print_formatted(), "[[1, 2], [3, 4]]");
}

#[test]
fn test_print_formatted_object_simple() {
    // C: {"a":1} -> "{\n\t\"a\":\t1\n}"
    let mut m = std::collections::HashMap::new();
    m.insert("a".to_string(), CJson::Number(1.0));
    let v = CJson::Object(m);
    assert_eq!(v.print_formatted(), "{\n\t\"a\":\t1\n}");
}

#[test]
fn test_print_formatted_object_empty() {
    // C: {} -> "{\n}"
    let v = CJson::Object(std::collections::HashMap::new());
    assert_eq!(v.print_formatted(), "{\n}");
}

#[test]
fn test_print_formatted_object_nested() {
    // C: {"nested":{"x":1}} -> "{\n\t\"nested\":\t{\n\t\t\"x\":\t1\n\t}\n}"
    let mut inner = std::collections::HashMap::new();
    inner.insert("x".to_string(), CJson::Number(1.0));
    let mut outer = std::collections::HashMap::new();
    outer.insert("nested".to_string(), CJson::Object(inner));
    let v = CJson::Object(outer);
    assert_eq!(
        v.print_formatted(),
        "{\n\t\"nested\":\t{\n\t\t\"x\":\t1\n\t}\n}"
    );
}

#[test]
fn test_get_array_size() {
    // C: cJSON_GetArraySize for [10,20,30] -> 3
    let v = parse("[10,20,30]", true).unwrap();
    assert_eq!(v.get_array_size(), Some(3));
}

#[test]
fn test_get_array_size_empty() {
    let v = parse("[]", true).unwrap();
    assert_eq!(v.get_array_size(), Some(0));
}

#[test]
fn test_get_array_size_non_array() {
    let v = CJson::Number(1.0);
    assert_eq!(v.get_array_size(), None);
}

#[test]
fn test_get_array_item() {
    // C: GetArrayItem of [10,20,30] at 0 -> 10, at 2 -> 30
    let v = parse("[10,20,30]", true).unwrap();
    assert_eq!(v.get_array_item(0), Some(&CJson::Number(10.0)));
    assert_eq!(v.get_array_item(2), Some(&CJson::Number(30.0)));
    assert_eq!(v.get_array_item(3), None);
}

#[test]
fn test_get_object_item_case_insensitive() {
    // C: GetObjectItem with "foo" (case-insensitive) finds "Foo" key
    let v = parse("{\"Foo\":1,\"Bar\":\"hello\"}", true).unwrap();
    let foo = v.get_object_item("foo");
    assert_eq!(foo, Some(&CJson::Number(1.0)));
    let bar = v.get_object_item("BAR");
    assert_eq!(bar, Some(&CJson::String("hello".to_string())));
}

#[test]
fn test_get_object_item_missing() {
    let v = parse("{\"a\":1}", true).unwrap();
    assert!(v.get_object_item("nope").is_none());
}

#[test]
fn test_create_null() {
    assert_eq!(CJson::create_null(), CJson::Null);
}

#[test]
fn test_create_bool() {
    assert_eq!(CJson::create_bool(true), CJson::Bool(true));
    assert_eq!(CJson::create_bool(false), CJson::Bool(false));
}

#[test]
fn test_create_number() {
    assert_eq!(CJson::create_number(42.0), CJson::Number(42.0));
    assert_eq!(CJson::create_number(-1.5), CJson::Number(-1.5));
}

#[test]
fn test_create_string() {
    assert_eq!(CJson::create_string("hi"), CJson::String("hi".to_string()));
    assert_eq!(
        CJson::create_string(String::from("world")),
        CJson::String("world".to_string())
    );
}

#[test]
fn test_create_array() {
    assert_eq!(CJson::create_array(), CJson::Array(vec![]));
}

#[test]
fn test_create_object() {
    let v = CJson::create_object();
    match v {
        CJson::Object(m) => assert!(m.is_empty()),
        _ => panic!("expected object"),
    }
}

#[test]
fn test_add_item_to_array() {
    let mut a = CJson::create_array();
    a.add_item_to_array(CJson::Number(1.0)).unwrap();
    a.add_item_to_array(CJson::Number(2.0)).unwrap();
    assert_eq!(a, CJson::Array(vec![CJson::Number(1.0), CJson::Number(2.0)]));
}

#[test]
fn test_add_item_to_array_wrong_type() {
    let mut v = CJson::create_object();
    let r = v.add_item_to_array(CJson::Number(1.0));
    assert!(r.is_err());
}

#[test]
fn test_add_item_to_object() {
    // Like cJSON_CreateObject + cJSON_AddNumberToObject + cJSON_AddStringToObject
    // C unformatted output: {"n":5,"s":"hi"}
    let mut o = CJson::create_object();
    o.add_item_to_object("n", CJson::Number(5.0)).unwrap();
    o.add_item_to_object("s", CJson::String("hi".to_string())).unwrap();
    let printed = o.print_unformatted();
    // Order of map iteration may differ, but our implementation sorts keys.
    assert_eq!(printed, "{\"n\":5,\"s\":\"hi\"}");
}

#[test]
fn test_add_item_to_object_wrong_type() {
    let mut v = CJson::create_array();
    let r = v.add_item_to_object("k", CJson::Null);
    assert!(r.is_err());
}

#[test]
fn test_display() {
    // The Display impl uses compact form
    let v = CJson::Array(vec![CJson::Number(1.0), CJson::Number(2.0)]);
    assert_eq!(format!("{}", v), "[1,2]");
}

#[test]
fn test_round_trip_simple_array() {
    // Round trip: parse -> print_unformatted should match for simple inputs
    let input = "[1,2,3]";
    let v = parse(input, true).unwrap();
    assert_eq!(v.print_unformatted(), input);
}

#[test]
fn test_round_trip_object() {
    // C unformatted: {"a":1}
    let input = "{\"a\":1}";
    let v = parse(input, true).unwrap();
    assert_eq!(v.print_unformatted(), input);
}

#[test]
fn test_parse_string_unicode_escape() {
    // A -> "A"
    let v = parse("\"\\u0041\"", true).unwrap();
    assert_eq!(v, CJson::String("A".to_string()));
}

#[test]
fn test_print_string_with_control_char() {
    // Embedded control char (e.g. ) should be escaped as \uXXXX
    // C: "" -> "\"\\u0001\""
    let v = CJson::String("\u{0001}".to_string());
    assert_eq!(v.print_unformatted(), "\"\\u0001\"");
}

#[test]
fn test_clone() {
    let v = parse("[1,2,3]", true).unwrap();
    let copy = v.clone();
    assert_eq!(v, copy);
}

fn main() {}
