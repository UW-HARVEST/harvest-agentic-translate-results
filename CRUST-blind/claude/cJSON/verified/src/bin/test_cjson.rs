use cJSON::cjson::*;
use std::collections::HashMap;

fn main() {}

// =========================================================================
// parse() tests for primitives
// =========================================================================

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
fn test_parse_zero() {
    let v = parse("0", true).unwrap();
    assert_eq!(v, CJson::Number(0.0));
}

#[test]
fn test_parse_int() {
    let v = parse("42", true).unwrap();
    assert_eq!(v, CJson::Number(42.0));
}

#[test]
fn test_parse_neg_int() {
    let v = parse("-42", true).unwrap();
    assert_eq!(v, CJson::Number(-42.0));
}

#[test]
fn test_parse_float() {
    let v = parse("3.14", true).unwrap();
    if let CJson::Number(n) = v {
        assert!((n - 3.14).abs() < 1e-10);
    } else {
        panic!("expected Number");
    }
}

#[test]
fn test_parse_neg_float() {
    let v = parse("-7.5", true).unwrap();
    assert_eq!(v, CJson::Number(-7.5));
}

#[test]
fn test_parse_exp() {
    let v = parse("1.5e10", true).unwrap();
    assert_eq!(v, CJson::Number(1.5e10));
}

#[test]
fn test_parse_neg_exp() {
    let v = parse("1.5e-7", true).unwrap();
    if let CJson::Number(n) = v {
        assert!((n - 1.5e-7).abs() < 1e-15);
    } else {
        panic!();
    }
}

#[test]
fn test_parse_exp_pos() {
    let v = parse("1e+5", true).unwrap();
    assert_eq!(v, CJson::Number(1e5));
}

#[test]
fn test_parse_decimal_specific() {
    let v = parse("37.7668", true).unwrap();
    if let CJson::Number(n) = v {
        assert!((n - 37.7668).abs() < 1e-10);
    } else {
        panic!();
    }
}

// =========================================================================
// parse() tests for strings
// =========================================================================

#[test]
fn test_parse_string_simple() {
    let v = parse(r#""hello""#, true).unwrap();
    assert_eq!(v, CJson::String("hello".to_string()));
}

#[test]
fn test_parse_string_empty() {
    let v = parse(r#""""#, true).unwrap();
    assert_eq!(v, CJson::String("".to_string()));
}

#[test]
fn test_parse_string_escaped_quote() {
    // "a\"b" -> a"b
    let v = parse(r#""a\"b""#, true).unwrap();
    assert_eq!(v, CJson::String(r#"a"b"#.to_string()));
}

#[test]
fn test_parse_string_escapes_all() {
    // "\b\f\n\r\t\\/"
    let v = parse(r#""\b\f\n\r\t\\\/""#, true).unwrap();
    assert_eq!(v, CJson::String("\u{0008}\u{000C}\n\r\t\\/".to_string()));
}

#[test]
fn test_parse_string_unicode_ascii() {
    let v = parse(r#""A""#, true).unwrap();
    assert_eq!(v, CJson::String("A".to_string()));
}

#[test]
fn test_parse_string_unicode_acute() {
    let v = parse(r#""é""#, true).unwrap();
    assert_eq!(v, CJson::String("é".to_string()));
}

#[test]
fn test_parse_string_unicode_surrogate() {
    let v = parse(r#""😀""#, true).unwrap();
    assert_eq!(v, CJson::String("😀".to_string()));
}

// =========================================================================
// parse() tests for arrays
// =========================================================================

#[test]
fn test_parse_array_empty() {
    let v = parse("[]", true).unwrap();
    assert_eq!(v, CJson::Array(vec![]));
}

#[test]
fn test_parse_array_nums() {
    let v = parse("[1, 2, 3]", true).unwrap();
    assert_eq!(
        v,
        CJson::Array(vec![CJson::Number(1.0), CJson::Number(2.0), CJson::Number(3.0)])
    );
}

#[test]
fn test_parse_array_mixed() {
    let v = parse(r#"[null, true, false, 42, "hi", [], {}]"#, true).unwrap();
    assert_eq!(
        v,
        CJson::Array(vec![
            CJson::Null,
            CJson::Bool(true),
            CJson::Bool(false),
            CJson::Number(42.0),
            CJson::String("hi".to_string()),
            CJson::Array(vec![]),
            CJson::Object(HashMap::new()),
        ])
    );
}

#[test]
fn test_parse_array_nested() {
    let v = parse("[[1,2],[3,4]]", true).unwrap();
    assert_eq!(
        v,
        CJson::Array(vec![
            CJson::Array(vec![CJson::Number(1.0), CJson::Number(2.0)]),
            CJson::Array(vec![CJson::Number(3.0), CJson::Number(4.0)]),
        ])
    );
}

// =========================================================================
// parse() tests for objects
// =========================================================================

#[test]
fn test_parse_object_empty() {
    let v = parse("{}", true).unwrap();
    assert_eq!(v, CJson::Object(HashMap::new()));
}

#[test]
fn test_parse_object_simple() {
    let v = parse(r#"{"a":1, "b":2}"#, true).unwrap();
    let mut expected = HashMap::new();
    expected.insert("a".to_string(), CJson::Number(1.0));
    expected.insert("b".to_string(), CJson::Number(2.0));
    assert_eq!(v, CJson::Object(expected));
}

#[test]
fn test_parse_object_nested() {
    let v = parse(r#"{"a":{"b":1}}"#, true).unwrap();
    let mut inner = HashMap::new();
    inner.insert("b".to_string(), CJson::Number(1.0));
    let mut outer = HashMap::new();
    outer.insert("a".to_string(), CJson::Object(inner));
    assert_eq!(v, CJson::Object(outer));
}

// =========================================================================
// whitespace handling
// =========================================================================

#[test]
fn test_parse_with_whitespace() {
    let v = parse("   \n\t  42  \n", true).unwrap();
    assert_eq!(v, CJson::Number(42.0));
}

#[test]
fn test_parse_array_whitespace() {
    let v = parse("[ 1 , 2 , 3 ]", true).unwrap();
    assert_eq!(
        v,
        CJson::Array(vec![CJson::Number(1.0), CJson::Number(2.0), CJson::Number(3.0)])
    );
}

// =========================================================================
// parse failures
// =========================================================================

#[test]
fn test_parse_invalid_token() {
    let r = parse("xyz", true);
    assert!(r.is_err());
}

#[test]
fn test_parse_empty_input() {
    let r = parse("", true);
    assert!(r.is_err());
}

#[test]
fn test_parse_garbage_after_value() {
    let r = parse("42abc", true);
    assert!(r.is_err());
}

#[test]
fn test_parse_garbage_after_value_loose() {
    // require_end=false should succeed
    let r = parse("42abc", false);
    assert!(r.is_ok());
    assert_eq!(r.unwrap(), CJson::Number(42.0));
}

// =========================================================================
// print_unformatted (PrintUnformatted) tests
// =========================================================================

#[test]
fn test_print_unfmt_null() {
    assert_eq!(CJson::Null.print_unformatted(), "null");
}

#[test]
fn test_print_unfmt_true() {
    assert_eq!(CJson::Bool(true).print_unformatted(), "true");
}

#[test]
fn test_print_unfmt_false() {
    assert_eq!(CJson::Bool(false).print_unformatted(), "false");
}

#[test]
fn test_print_unfmt_zero() {
    assert_eq!(CJson::Number(0.0).print_unformatted(), "0");
}

#[test]
fn test_print_unfmt_int_pos() {
    assert_eq!(CJson::Number(42.0).print_unformatted(), "42");
}

#[test]
fn test_print_unfmt_int_neg() {
    assert_eq!(CJson::Number(-42.0).print_unformatted(), "-42");
}

#[test]
fn test_print_unfmt_int_max() {
    // C: prints "2147483647" via %d (fits int32)
    assert_eq!(CJson::Number(2147483647.0).print_unformatted(), "2147483647");
}

#[test]
fn test_print_unfmt_int_min() {
    assert_eq!(CJson::Number(-2147483648.0).print_unformatted(), "-2147483648");
}

#[test]
fn test_print_unfmt_int_above_intmax() {
    // 1e10 doesn't fit in i32; C prints as "%.0f" because still integer-valued and < 1e60
    assert_eq!(CJson::Number(1.0e10).print_unformatted(), "10000000000");
}

#[test]
fn test_print_unfmt_int_above_intmax_2() {
    // 12345670000 fits in i64 but not i32
    assert_eq!(CJson::Number(12345670000.0).print_unformatted(), "12345670000");
}

#[test]
fn test_print_unfmt_int_eq_e9() {
    assert_eq!(CJson::Number(1.0e9).print_unformatted(), "1000000000");
}

#[test]
fn test_print_unfmt_decimal() {
    // C prints "3.140000" via %f
    assert_eq!(CJson::Number(3.14).print_unformatted(), "3.140000");
}

#[test]
fn test_print_unfmt_decimal_3_14159() {
    assert_eq!(CJson::Number(3.14159).print_unformatted(), "3.141590");
}

#[test]
fn test_print_unfmt_decimal_neg() {
    assert_eq!(CJson::Number(-122.026020).print_unformatted(), "-122.026020");
}

#[test]
fn test_print_unfmt_decimal_specific() {
    assert_eq!(CJson::Number(37.7668).print_unformatted(), "37.766800");
}

#[test]
fn test_print_unfmt_small() {
    // 0.000001 == 1e-6, in C abs(d) < 1e-6 is false (it's equal), so it uses %f -> "0.000001"
    assert_eq!(CJson::Number(0.000001).print_unformatted(), "0.000001");
}

#[test]
fn test_print_unfmt_smaller_than_e6() {
    // 0.0000001 = 1e-7 < 1e-6 -> %e -> "1.000000e-07"
    assert_eq!(CJson::Number(0.0000001).print_unformatted(), "1.000000e-07");
}

#[test]
fn test_print_unfmt_super_big() {
    // 1.234e60: integer-valued? No because fractional. 1e60 > 1e9 -> %e
    assert_eq!(CJson::Number(1.234e60).print_unformatted(), "1.234000e+60");
}

#[test]
fn test_print_unfmt_string_simple() {
    assert_eq!(CJson::String("hello".to_string()).print_unformatted(), r#""hello""#);
}

#[test]
fn test_print_unfmt_string_empty() {
    assert_eq!(CJson::String("".to_string()).print_unformatted(), r#""""#);
}

#[test]
fn test_print_unfmt_string_special() {
    let s = CJson::String("hello\nworld\t\"quote\"".to_string());
    assert_eq!(s.print_unformatted(), r#""hello\nworld\t\"quote\"""#);
}


#[test]
fn test_print_unfmt_string_control() {
    // C escapes control bytes < 32 as \uXXXX with lowercase hex.
    let s = CJson::String("a\u{0001}b".to_string());
    assert_eq!(s.print_unformatted(), "\"a\\u0001b\"");
}

#[test]
fn test_print_unfmt_string_all_escapes() {
    let s = CJson::String("\u{0008}\u{000C}\n\r\t\\".to_string());
    // C produces \b\f\n\r\t\\
    assert_eq!(s.print_unformatted(), r#""\b\f\n\r\t\\""#);
}

#[test]
fn test_print_unfmt_array_empty() {
    assert_eq!(CJson::Array(vec![]).print_unformatted(), "[]");
}

#[test]
fn test_print_unfmt_array_nums() {
    let arr = CJson::Array(vec![
        CJson::Number(1.0),
        CJson::Number(2.0),
        CJson::Number(3.0),
    ]);
    assert_eq!(arr.print_unformatted(), "[1,2,3]");
}

#[test]
fn test_print_unfmt_array_mixed() {
    let arr = CJson::Array(vec![
        CJson::Null,
        CJson::Bool(true),
        CJson::Bool(false),
        CJson::Number(42.0),
        CJson::String("hi".to_string()),
        CJson::Array(vec![]),
    ]);
    assert_eq!(arr.print_unformatted(), r#"[null,true,false,42,"hi",[]]"#);
}

#[test]
fn test_print_unfmt_object_empty() {
    assert_eq!(CJson::Object(HashMap::new()).print_unformatted(), "{}");
}

#[test]
fn test_print_unfmt_object_single_key() {
    let mut m = HashMap::new();
    m.insert("a".to_string(), CJson::Number(1.0));
    assert_eq!(CJson::Object(m).print_unformatted(), r#"{"a":1}"#);
}

// =========================================================================
// print_formatted (Print/pretty)
// =========================================================================

#[test]
fn test_print_fmt_null() {
    assert_eq!(CJson::Null.print_formatted(), "null");
}

#[test]
fn test_print_fmt_true() {
    assert_eq!(CJson::Bool(true).print_formatted(), "true");
}

#[test]
fn test_print_fmt_false() {
    assert_eq!(CJson::Bool(false).print_formatted(), "false");
}

#[test]
fn test_print_fmt_number() {
    assert_eq!(CJson::Number(42.0).print_formatted(), "42");
}

#[test]
fn test_print_fmt_string() {
    assert_eq!(
        CJson::String("hi".to_string()).print_formatted(),
        r#""hi""#
    );
}

#[test]
fn test_print_fmt_array_empty() {
    assert_eq!(CJson::Array(vec![]).print_formatted(), "[]");
}

#[test]
fn test_print_fmt_array_nums() {
    let arr = CJson::Array(vec![
        CJson::Number(1.0),
        CJson::Number(2.0),
        CJson::Number(3.0),
    ]);
    assert_eq!(arr.print_formatted(), "[1, 2, 3]");
}

#[test]
fn test_print_fmt_object_empty_top() {
    // C top-level: "{\n}"
    assert_eq!(CJson::Object(HashMap::new()).print_formatted(), "{\n}");
}

#[test]
fn test_print_fmt_object_single_key() {
    let mut m = HashMap::new();
    m.insert("a".to_string(), CJson::Number(1.0));
    let s = CJson::Object(m).print_formatted();
    // Should be "{\n\t\"a\":\t1\n}"
    assert_eq!(s, "{\n\t\"a\":\t1\n}");
}

#[test]
fn test_print_fmt_object_with_array() {
    // Single-key object so HashMap order is deterministic
    let mut m = HashMap::new();
    m.insert(
        "b".to_string(),
        CJson::Array(vec![CJson::Number(1.0), CJson::Number(2.0)]),
    );
    let s = CJson::Object(m).print_formatted();
    assert_eq!(s, "{\n\t\"b\":\t[1, 2]\n}");
}

#[test]
fn test_print_fmt_nested_object() {
    // Outer has 1 key "a" -> inner with 1 key "b":1
    let mut inner = HashMap::new();
    inner.insert("b".to_string(), CJson::Number(1.0));
    let mut outer = HashMap::new();
    outer.insert("a".to_string(), CJson::Object(inner));
    let s = CJson::Object(outer).print_formatted();
    // C: "{\n\t\"a\":\t{\n\t\t\"b\":\t1\n\t}\n}"
    assert_eq!(s, "{\n\t\"a\":\t{\n\t\t\"b\":\t1\n\t}\n}");
}

#[test]
fn test_print_fmt_nested_empty_object() {
    // {"a":{}} => "{\n\t\"a\":\t{\n}\n}"  (C output)
    let inner = HashMap::new();
    let mut outer = HashMap::new();
    outer.insert("a".to_string(), CJson::Object(inner));
    let s = CJson::Object(outer).print_formatted();
    assert_eq!(s, "{\n\t\"a\":\t{\n}\n}");
}

#[test]
fn test_print_fmt_array_of_arrays() {
    let arr = CJson::Array(vec![
        CJson::Array(vec![CJson::Number(1.0), CJson::Number(2.0)]),
        CJson::Array(vec![CJson::Number(3.0), CJson::Number(4.0)]),
    ]);
    assert_eq!(arr.print_formatted(), "[[1, 2], [3, 4]]");
}

// =========================================================================
// get_array_size
// =========================================================================

#[test]
fn test_get_array_size_array() {
    let arr = CJson::Array(vec![CJson::Number(1.0), CJson::Number(2.0)]);
    assert_eq!(arr.get_array_size(), Some(2));
}

#[test]
fn test_get_array_size_empty_array() {
    let arr = CJson::Array(vec![]);
    assert_eq!(arr.get_array_size(), Some(0));
}

#[test]
fn test_get_array_size_object() {
    let mut m = HashMap::new();
    m.insert("a".to_string(), CJson::Null);
    m.insert("b".to_string(), CJson::Bool(true));
    assert_eq!(CJson::Object(m).get_array_size(), Some(2));
}

#[test]
fn test_get_array_size_other() {
    assert_eq!(CJson::Null.get_array_size(), None);
    assert_eq!(CJson::Bool(true).get_array_size(), None);
    assert_eq!(CJson::Number(1.0).get_array_size(), None);
    assert_eq!(CJson::String("x".to_string()).get_array_size(), None);
}

// =========================================================================
// get_array_item
// =========================================================================

#[test]
fn test_get_array_item_valid() {
    let arr = CJson::Array(vec![
        CJson::Number(10.0),
        CJson::Number(20.0),
        CJson::Number(30.0),
    ]);
    assert_eq!(arr.get_array_item(0), Some(&CJson::Number(10.0)));
    assert_eq!(arr.get_array_item(1), Some(&CJson::Number(20.0)));
    assert_eq!(arr.get_array_item(2), Some(&CJson::Number(30.0)));
}

#[test]
fn test_get_array_item_out_of_bounds() {
    let arr = CJson::Array(vec![CJson::Number(1.0)]);
    assert_eq!(arr.get_array_item(5), None);
}

#[test]
fn test_get_array_item_non_array() {
    assert_eq!(CJson::Null.get_array_item(0), None);
    assert_eq!(CJson::Number(1.0).get_array_item(0), None);
}

// =========================================================================
// get_object_item (case-insensitive)
// =========================================================================

#[test]
fn test_get_object_item_exact() {
    let mut m = HashMap::new();
    m.insert("name".to_string(), CJson::String("Alice".to_string()));
    let obj = CJson::Object(m);
    assert_eq!(
        obj.get_object_item("name"),
        Some(&CJson::String("Alice".to_string()))
    );
}

#[test]
fn test_get_object_item_case_insensitive() {
    let mut m = HashMap::new();
    m.insert("Name".to_string(), CJson::String("Alice".to_string()));
    let obj = CJson::Object(m);
    assert_eq!(
        obj.get_object_item("name"),
        Some(&CJson::String("Alice".to_string()))
    );
    assert_eq!(
        obj.get_object_item("NAME"),
        Some(&CJson::String("Alice".to_string()))
    );
}

#[test]
fn test_get_object_item_missing() {
    let mut m = HashMap::new();
    m.insert("a".to_string(), CJson::Null);
    let obj = CJson::Object(m);
    assert_eq!(obj.get_object_item("missing"), None);
}

#[test]
fn test_get_object_item_non_object() {
    assert_eq!(CJson::Null.get_object_item("x"), None);
    assert_eq!(CJson::Array(vec![]).get_object_item("x"), None);
}

// =========================================================================
// create_xxx
// =========================================================================

#[test]
fn test_create_null() {
    assert_eq!(CJson::create_null(), CJson::Null);
}

#[test]
fn test_create_bool_true() {
    assert_eq!(CJson::create_bool(true), CJson::Bool(true));
}

#[test]
fn test_create_bool_false() {
    assert_eq!(CJson::create_bool(false), CJson::Bool(false));
}

#[test]
fn test_create_number() {
    assert_eq!(CJson::create_number(5.5), CJson::Number(5.5));
}

#[test]
fn test_create_string_str() {
    assert_eq!(
        CJson::create_string("hello"),
        CJson::String("hello".to_string())
    );
}

#[test]
fn test_create_string_string() {
    assert_eq!(
        CJson::create_string(String::from("hello")),
        CJson::String("hello".to_string())
    );
}

#[test]
fn test_create_array() {
    assert_eq!(CJson::create_array(), CJson::Array(vec![]));
}

#[test]
fn test_create_object() {
    assert_eq!(CJson::create_object(), CJson::Object(HashMap::new()));
}

// =========================================================================
// add_item_to_array / add_item_to_object
// =========================================================================

#[test]
fn test_add_item_to_array_ok() {
    let mut arr = CJson::create_array();
    arr.add_item_to_array(CJson::Number(1.0)).unwrap();
    arr.add_item_to_array(CJson::Number(2.0)).unwrap();
    arr.add_item_to_array(CJson::Number(3.0)).unwrap();
    assert_eq!(
        arr,
        CJson::Array(vec![
            CJson::Number(1.0),
            CJson::Number(2.0),
            CJson::Number(3.0)
        ])
    );
}

#[test]
fn test_add_item_to_array_wrong_type() {
    let mut x = CJson::create_object();
    let r = x.add_item_to_array(CJson::Null);
    assert!(r.is_err());
    assert_eq!(r.unwrap_err(), "not an array");
}

#[test]
fn test_add_item_to_object_ok() {
    let mut obj = CJson::create_object();
    obj.add_item_to_object("k", CJson::Number(7.0)).unwrap();
    let mut expected = HashMap::new();
    expected.insert("k".to_string(), CJson::Number(7.0));
    assert_eq!(obj, CJson::Object(expected));
}

#[test]
fn test_add_item_to_object_wrong_type() {
    let mut x = CJson::create_array();
    let r = x.add_item_to_object("k", CJson::Null);
    assert!(r.is_err());
    assert_eq!(r.unwrap_err(), "not an object");
}

// =========================================================================
// Round-trip parse + print
// =========================================================================

#[test]
fn test_roundtrip_array_pretty() {
    // Same as text2 from main.c
    let text = r#"["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"]"#;
    let v = parse(text, true).unwrap();
    // Pretty form for top-level array
    let s = v.print_formatted();
    assert_eq!(
        s,
        r#"["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"]"#
    );
}

#[test]
fn test_roundtrip_matrix_pretty() {
    // Same as text3 from main.c minified
    let v = parse("[[0, -1, 0], [1, 0, 0], [0, 0, 1]]", true).unwrap();
    let s = v.print_formatted();
    assert_eq!(s, "[[0, -1, 0], [1, 0, 0], [0, 0, 1]]");
}

// =========================================================================
// Display impl
// =========================================================================

#[test]
fn test_display_impl() {
    let v = CJson::Number(42.0);
    assert_eq!(format!("{}", v), "42");
    let v = CJson::String("hi".to_string());
    assert_eq!(format!("{}", v), "\"hi\"");
    let v = CJson::Array(vec![CJson::Bool(true), CJson::Null]);
    assert_eq!(format!("{}", v), "[true,null]");
}

// =========================================================================
// CJsonError display
// =========================================================================

#[test]
fn test_error_display_eof() {
    let e = CJsonError::UnexpectedEOF { pos: 5 };
    assert_eq!(e.to_string(), "Unexpected end of input at position 5");
}

#[test]
fn test_error_display_unexpected_token() {
    let e = CJsonError::UnexpectedToken { ch: 'x', pos: 3 };
    assert_eq!(e.to_string(), "Unexpected token 'x' at position 3");
}

#[test]
fn test_error_display_invalid_literal() {
    let e = CJsonError::InvalidLiteral { expected: "null", pos: 0 };
    assert_eq!(e.to_string(), "Invalid literal, expected 'null' at position 0");
}

#[test]
fn test_error_display_invalid_number() {
    let e = CJsonError::InvalidNumber { pos: 7 };
    assert_eq!(e.to_string(), "Invalid number at position 7");
}

#[test]
fn test_error_display_invalid_escape() {
    let e = CJsonError::InvalidEscape { pos: 4 };
    assert_eq!(e.to_string(), "Invalid escape sequence at position 4");
}

#[test]
fn test_error_display_invalid_unicode_escape() {
    let e = CJsonError::InvalidUnicodeEscape { pos: 2 };
    assert_eq!(e.to_string(), "Invalid unicode escape at position 2");
}

#[test]
fn test_error_display_expected_colon() {
    let e = CJsonError::ExpectedColon { pos: 8 };
    assert_eq!(e.to_string(), "Expected ':' at position 8");
}

#[test]
fn test_error_display_expected_comma_or_end() {
    let e = CJsonError::ExpectedCommaOrEnd { pos: 9 };
    assert_eq!(e.to_string(), "Expected ',' or end at position 9");
}

// =========================================================================
// Parse + print round trip for various complex cases
// =========================================================================

#[test]
fn test_parse_and_get_object_size() {
    let v = parse(r#"{"a":1,"b":2,"c":3}"#, true).unwrap();
    assert_eq!(v.get_array_size(), Some(3));
}

#[test]
fn test_parse_and_get_object_items_case_insensitive() {
    let v = parse(r#"{"Name":"Alice","AGE":30}"#, true).unwrap();
    assert_eq!(
        v.get_object_item("name"),
        Some(&CJson::String("Alice".to_string()))
    );
    assert_eq!(
        v.get_object_item("age"),
        Some(&CJson::Number(30.0))
    );
}

#[test]
fn test_parse_array_of_strings() {
    let v = parse(r#"["a","b","c"]"#, true).unwrap();
    assert_eq!(v.get_array_size(), Some(3));
    assert_eq!(
        v.get_array_item(1),
        Some(&CJson::String("b".to_string()))
    );
}

#[test]
fn test_parse_negative_decimal() {
    let v = parse("-2.5", true).unwrap();
    assert_eq!(v, CJson::Number(-2.5));
}

#[test]
fn test_parse_object_with_whitespace_padding() {
    let v = parse("  { \"x\" : 1 } ", true).unwrap();
    let mut m = HashMap::new();
    m.insert("x".to_string(), CJson::Number(1.0));
    assert_eq!(v, CJson::Object(m));
}

#[test]
fn test_print_fmt_top_array_with_strings() {
    let arr = CJson::Array(vec![
        CJson::String("Sunday".to_string()),
        CJson::String("Monday".to_string()),
    ]);
    assert_eq!(arr.print_formatted(), r#"["Sunday", "Monday"]"#);
}

#[test]
fn test_print_unfmt_array_with_strings() {
    let arr = CJson::Array(vec![
        CJson::String("Sunday".to_string()),
        CJson::String("Monday".to_string()),
    ]);
    assert_eq!(arr.print_unformatted(), r#"["Sunday","Monday"]"#);
}

#[test]
fn test_parse_text2_main_c() {
    // Same as text2 in c_src/tests/main.c
    let text = r#"["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"]"#;
    let v = parse(text, true).unwrap();
    assert_eq!(v.get_array_size(), Some(7));
    assert_eq!(
        v.get_array_item(0),
        Some(&CJson::String("Sunday".to_string()))
    );
    assert_eq!(
        v.get_array_item(6),
        Some(&CJson::String("Saturday".to_string()))
    );
}

#[test]
fn test_parse_text3_main_c() {
    // Matrix
    let text = "[\n    [0, -1, 0],\n    [1, 0, 0],\n    [0, 0, 1]\n	]\n";
    let v = parse(text, true).unwrap();
    assert_eq!(v.get_array_size(), Some(3));
    let row1 = v.get_array_item(1).unwrap();
    assert_eq!(row1.get_array_size(), Some(3));
    assert_eq!(row1.get_array_item(0), Some(&CJson::Number(1.0)));
}
