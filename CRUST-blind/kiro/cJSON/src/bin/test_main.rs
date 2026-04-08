use cJSON::cjson::{parse, CJson, CJsonError};

// ── Parsing primitives ──────────────────────────────────────────────

#[test]
fn test_parse_null() {
    assert_eq!(parse("null", false).unwrap(), CJson::Null);
}

#[test]
fn test_parse_true() {
    assert_eq!(parse("true", false).unwrap(), CJson::Bool(true));
}

#[test]
fn test_parse_false() {
    assert_eq!(parse("false", false).unwrap(), CJson::Bool(false));
}

#[test]
fn test_parse_string_simple() {
    assert_eq!(
        parse("\"hello\"", false).unwrap(),
        CJson::String("hello".into())
    );
}

#[test]
fn test_parse_empty_string() {
    assert_eq!(
        parse("\"\"", false).unwrap(),
        CJson::String("".into())
    );
}

#[test]
fn test_parse_string_with_escapes() {
    let j = parse("\"hello\\nworld\"", false).unwrap();
    assert_eq!(j, CJson::String("hello\nworld".into()));
}

#[test]
fn test_parse_string_with_tab() {
    let j = parse("\"tab\\there\"", false).unwrap();
    assert_eq!(j, CJson::String("tab\there".into()));
}

#[test]
fn test_parse_string_with_backslash() {
    let j = parse("\"a\\\\b\"", false).unwrap();
    assert_eq!(j, CJson::String("a\\b".into()));
}

#[test]
fn test_parse_string_with_quote() {
    let j = parse("\"a\\\"b\"", false).unwrap();
    assert_eq!(j, CJson::String("a\"b".into()));
}

#[test]
fn test_parse_string_unicode_escape() {
    // \u0041 = 'A'
    let j = parse("\"\\u0041\"", false).unwrap();
    assert_eq!(j, CJson::String("A".into()));
}

// ── Parsing numbers ─────────────────────────────────────────────────

#[test]
fn test_parse_integer() {
    assert_eq!(parse("123", false).unwrap(), CJson::Number(123.0));
}

#[test]
fn test_parse_zero() {
    assert_eq!(parse("0", false).unwrap(), CJson::Number(0.0));
}

#[test]
fn test_parse_negative_zero() {
    // C: parse_number("-0") => n=0, sign=-1, result = -1*0*... = -0.0
    // But C prints it as "0"
    let j = parse("-0", false).unwrap();
    if let CJson::Number(n) = j {
        assert_eq!(n, 0.0);
    } else {
        panic!("expected number");
    }
}

#[test]
fn test_parse_negative_integer() {
    assert_eq!(parse("-7", false).unwrap(), CJson::Number(-7.0));
}

#[test]
fn test_parse_float() {
    if let CJson::Number(n) = parse("0.5", false).unwrap() {
        assert!((n - 0.5).abs() < f64::EPSILON);
    } else {
        panic!("expected number");
    }
}

#[test]
fn test_parse_scientific() {
    if let CJson::Number(n) = parse("1e10", false).unwrap() {
        assert!((n - 1e10).abs() < 1.0);
    } else {
        panic!("expected number");
    }
}

#[test]
fn test_parse_negative_exponent() {
    if let CJson::Number(n) = parse("1e-7", false).unwrap() {
        assert!((n - 1e-7).abs() < 1e-15);
    } else {
        panic!("expected number");
    }
}

// ── Parsing arrays ──────────────────────────────────────────────────

#[test]
fn test_parse_empty_array() {
    assert_eq!(parse("[]", false).unwrap(), CJson::Array(vec![]));
}

#[test]
fn test_parse_int_array() {
    let j = parse("[1,2,3]", false).unwrap();
    assert_eq!(
        j,
        CJson::Array(vec![
            CJson::Number(1.0),
            CJson::Number(2.0),
            CJson::Number(3.0),
        ])
    );
}

#[test]
fn test_parse_string_array() {
    let j = parse("[\"a\",\"b\"]", false).unwrap();
    assert_eq!(
        j,
        CJson::Array(vec![
            CJson::String("a".into()),
            CJson::String("b".into()),
        ])
    );
}

#[test]
fn test_parse_nested_array() {
    let j = parse("[[0,-1,0],[1,0,0],[0,0,1]]", false).unwrap();
    if let CJson::Array(outer) = &j {
        assert_eq!(outer.len(), 3);
        if let CJson::Array(inner) = &outer[0] {
            assert_eq!(inner.len(), 3);
            assert_eq!(inner[1], CJson::Number(-1.0));
        } else {
            panic!("expected inner array");
        }
    } else {
        panic!("expected array");
    }
}

#[test]
fn test_parse_mixed_array() {
    let j = parse("[1,\"two\",true,null,false]", false).unwrap();
    if let CJson::Array(items) = j {
        assert_eq!(items.len(), 5);
        assert_eq!(items[0], CJson::Number(1.0));
        assert_eq!(items[1], CJson::String("two".into()));
        assert_eq!(items[2], CJson::Bool(true));
        assert_eq!(items[3], CJson::Null);
        assert_eq!(items[4], CJson::Bool(false));
    } else {
        panic!("expected array");
    }
}

// ── Parsing objects ─────────────────────────────────────────────────

#[test]
fn test_parse_empty_object() {
    let j = parse("{}", false).unwrap();
    if let CJson::Object(map) = j {
        assert_eq!(map.len(), 0);
    } else {
        panic!("expected object");
    }
}

#[test]
fn test_parse_simple_object() {
    let j = parse("{\"a\":1,\"b\":2}", false).unwrap();
    if let CJson::Object(map) = &j {
        assert_eq!(map.len(), 2);
        assert_eq!(map.get("a"), Some(&CJson::Number(1.0)));
        assert_eq!(map.get("b"), Some(&CJson::Number(2.0)));
    } else {
        panic!("expected object");
    }
}

#[test]
fn test_parse_nested_object() {
    let j = parse("{\"a\":{\"b\":1}}", false).unwrap();
    if let CJson::Object(outer) = &j {
        if let Some(CJson::Object(inner)) = outer.get("a") {
            assert_eq!(inner.get("b"), Some(&CJson::Number(1.0)));
        } else {
            panic!("expected inner object");
        }
    } else {
        panic!("expected object");
    }
}

// ── Parse errors ────────────────────────────────────────────────────

#[test]
fn test_parse_invalid_returns_error() {
    assert!(parse("invalid", false).is_err());
}

#[test]
fn test_parse_empty_returns_error() {
    assert!(parse("", false).is_err());
}

#[test]
fn test_parse_trailing_comma_array() {
    // C: cJSON_Parse("[1,]") returns NULL
    assert!(parse("[1,]", false).is_err());
}

#[test]
fn test_parse_trailing_comma_object() {
    assert!(parse("{\"a\":1,}", false).is_err());
}

// ── require_end (ParseWithOpts) ─────────────────────────────────────

#[test]
fn test_parse_require_end_ok() {
    // "123 " with trailing whitespace — should succeed with require_end
    assert!(parse("123 ", true).is_ok());
}

#[test]
fn test_parse_require_end_fail() {
    // "123abc" with require_end — should fail
    assert!(parse("123abc", true).is_err());
}

#[test]
fn test_parse_no_require_end_trailing_garbage() {
    // "123abc" without require_end — should succeed (parses 123)
    assert!(parse("123abc", false).is_ok());
}

// ── Whitespace handling ─────────────────────────────────────────────

#[test]
fn test_parse_with_whitespace() {
    let j = parse("  { \"a\" : 1 }  ", false).unwrap();
    if let CJson::Object(map) = j {
        assert_eq!(map.get("a"), Some(&CJson::Number(1.0)));
    } else {
        panic!("expected object");
    }
}

#[test]
fn test_parse_with_newlines() {
    let j = parse("[\n1,\n2\n]", false).unwrap();
    assert_eq!(
        j,
        CJson::Array(vec![CJson::Number(1.0), CJson::Number(2.0)])
    );
}

// ── print_unformatted ───────────────────────────────────────────────

#[test]
fn test_print_null() {
    assert_eq!(CJson::Null.print_unformatted(), "null");
}

#[test]
fn test_print_true() {
    assert_eq!(CJson::Bool(true).print_unformatted(), "true");
}

#[test]
fn test_print_false() {
    assert_eq!(CJson::Bool(false).print_unformatted(), "false");
}

#[test]
fn test_print_string() {
    assert_eq!(
        CJson::String("hello".into()).print_unformatted(),
        "\"hello\""
    );
}

#[test]
fn test_print_string_with_escapes() {
    let j = CJson::String("hello\nworld".into());
    assert_eq!(j.print_unformatted(), "\"hello\\nworld\"");
}

#[test]
fn test_print_string_with_tab() {
    let j = CJson::String("tab\there".into());
    assert_eq!(j.print_unformatted(), "\"tab\\there\"");
}

#[test]
fn test_print_string_with_quote() {
    let j = CJson::String("a\"b".into());
    assert_eq!(j.print_unformatted(), "\"a\\\"b\"");
}

#[test]
fn test_print_number_zero() {
    // C: "0"
    assert_eq!(CJson::Number(0.0).print_unformatted(), "0");
}

#[test]
fn test_print_number_integer() {
    // C: "42"
    assert_eq!(CJson::Number(42.0).print_unformatted(), "42");
}

#[test]
fn test_print_number_negative_integer() {
    // C: "-7"
    assert_eq!(CJson::Number(-7.0).print_unformatted(), "-7");
}

#[test]
fn test_print_empty_array() {
    assert_eq!(CJson::Array(vec![]).print_unformatted(), "[]");
}

#[test]
fn test_print_empty_object() {
    use std::collections::HashMap;
    assert_eq!(
        CJson::Object(HashMap::new()).print_unformatted(),
        "{}"
    );
}

// ── CJson methods ───────────────────────────────────────────────────

#[test]
fn test_get_array_size() {
    let j = parse("[1,2,3]", false).unwrap();
    assert_eq!(j.get_array_size(), Some(3));
}

#[test]
fn test_get_array_size_empty() {
    let j = parse("[]", false).unwrap();
    assert_eq!(j.get_array_size(), Some(0));
}

#[test]
fn test_get_array_size_non_array() {
    assert_eq!(CJson::Null.get_array_size(), None);
}

#[test]
fn test_get_array_item() {
    let j = parse("[10,20,30]", false).unwrap();
    assert_eq!(j.get_array_item(0), Some(&CJson::Number(10.0)));
    assert_eq!(j.get_array_item(2), Some(&CJson::Number(30.0)));
}

#[test]
fn test_get_array_item_out_of_bounds() {
    let j = parse("[1,2,3]", false).unwrap();
    assert_eq!(j.get_array_item(5), None);
}

#[test]
fn test_get_array_item_non_array() {
    assert_eq!(CJson::Null.get_array_item(0), None);
}

#[test]
fn test_get_object_item() {
    let j = parse("{\"Name\":\"test\"}", false).unwrap();
    // Case insensitive like C
    assert_eq!(
        j.get_object_item("name"),
        Some(&CJson::String("test".into()))
    );
    assert_eq!(
        j.get_object_item("NAME"),
        Some(&CJson::String("test".into()))
    );
}

#[test]
fn test_get_object_item_missing() {
    let j = parse("{\"a\":1}", false).unwrap();
    assert_eq!(j.get_object_item("missing"), None);
}

#[test]
fn test_get_object_item_non_object() {
    assert_eq!(CJson::Null.get_object_item("x"), None);
}

// ── Create helpers ──────────────────────────────────────────────────

#[test]
fn test_create_null() {
    assert_eq!(CJson::create_null(), CJson::Null);
}

#[test]
fn test_create_bool() {
    assert_eq!(CJson::create_bool(false), CJson::Bool(false));
    assert_eq!(CJson::create_bool(true), CJson::Bool(true));
}

#[test]
fn test_create_number() {
    assert_eq!(CJson::create_number(42.0), CJson::Number(42.0));
}

#[test]
fn test_create_string() {
    assert_eq!(
        CJson::create_string("hello"),
        CJson::String("hello".into())
    );
}

#[test]
fn test_create_array() {
    assert_eq!(CJson::create_array(), CJson::Array(vec![]));
}

#[test]
fn test_create_object() {
    use std::collections::HashMap;
    assert_eq!(CJson::create_object(), CJson::Object(HashMap::new()));
}

// ── Mutating methods ────────────────────────────────────────────────

#[test]
fn test_add_item_to_array() {
    let mut arr = CJson::create_array();
    arr.add_item_to_array(CJson::Number(1.0)).unwrap();
    arr.add_item_to_array(CJson::Number(2.0)).unwrap();
    assert_eq!(arr.get_array_size(), Some(2));
    assert_eq!(arr.get_array_item(0), Some(&CJson::Number(1.0)));
    assert_eq!(arr.get_array_item(1), Some(&CJson::Number(2.0)));
}

#[test]
fn test_add_item_to_array_non_array() {
    let mut n = CJson::Null;
    assert!(n.add_item_to_array(CJson::Number(1.0)).is_err());
}

#[test]
fn test_add_item_to_object() {
    let mut obj = CJson::create_object();
    obj.add_item_to_object("key", CJson::String("val".into()))
        .unwrap();
    assert_eq!(
        obj.get_object_item("key"),
        Some(&CJson::String("val".into()))
    );
}

#[test]
fn test_add_item_to_object_non_object() {
    let mut n = CJson::Null;
    assert!(n.add_item_to_object("k", CJson::Null).is_err());
}

// ── Round-trip parse → print_unformatted ────────────────────────────

#[test]
fn test_roundtrip_null() {
    let j = parse("null", false).unwrap();
    assert_eq!(j.print_unformatted(), "null");
}

#[test]
fn test_roundtrip_bool() {
    assert_eq!(parse("true", false).unwrap().print_unformatted(), "true");
    assert_eq!(parse("false", false).unwrap().print_unformatted(), "false");
}

#[test]
fn test_roundtrip_integer() {
    assert_eq!(parse("123", false).unwrap().print_unformatted(), "123");
}

#[test]
fn test_roundtrip_string() {
    assert_eq!(
        parse("\"hello\"", false).unwrap().print_unformatted(),
        "\"hello\""
    );
}

#[test]
fn test_roundtrip_empty_array() {
    assert_eq!(parse("[]", false).unwrap().print_unformatted(), "[]");
}

#[test]
fn test_roundtrip_empty_object() {
    assert_eq!(parse("{}", false).unwrap().print_unformatted(), "{}");
}

// ── Complex parse from C test data ──────────────────────────────────

#[test]
fn test_parse_text1_video() {
    let text = "{\n\"name\": \"Jack (\\\"Bee\\\") Nimble\", \n\"format\": {\"type\":       \"rect\", \n\"width\":      1920, \n\"height\":     1080, \n\"interlace\":  false,\"frame rate\": 24\n}\n}";
    let j = parse(text, false).unwrap();
    if let CJson::Object(map) = &j {
        assert_eq!(
            map.get("name"),
            Some(&CJson::String("Jack (\"Bee\") Nimble".into()))
        );
        if let Some(CJson::Object(fmt)) = map.get("format") {
            assert_eq!(fmt.get("type"), Some(&CJson::String("rect".into())));
            assert_eq!(fmt.get("width"), Some(&CJson::Number(1920.0)));
            assert_eq!(fmt.get("height"), Some(&CJson::Number(1080.0)));
            assert_eq!(fmt.get("interlace"), Some(&CJson::Bool(false)));
            assert_eq!(fmt.get("frame rate"), Some(&CJson::Number(24.0)));
        } else {
            panic!("expected format object");
        }
    } else {
        panic!("expected object");
    }
}

#[test]
fn test_parse_text2_days() {
    let text = "[\"Sunday\", \"Monday\", \"Tuesday\", \"Wednesday\", \"Thursday\", \"Friday\", \"Saturday\"]";
    let j = parse(text, false).unwrap();
    if let CJson::Array(items) = &j {
        assert_eq!(items.len(), 7);
        assert_eq!(items[0], CJson::String("Sunday".into()));
        assert_eq!(items[6], CJson::String("Saturday".into()));
    } else {
        panic!("expected array");
    }
}

#[test]
fn test_parse_text3_matrix() {
    let text = "[\n    [0, -1, 0],\n    [1, 0, 0],\n    [0, 0, 1]\n\t]\n";
    let j = parse(text, false).unwrap();
    if let CJson::Array(rows) = &j {
        assert_eq!(rows.len(), 3);
        if let CJson::Array(row0) = &rows[0] {
            assert_eq!(row0, &vec![CJson::Number(0.0), CJson::Number(-1.0), CJson::Number(0.0)]);
        } else {
            panic!("expected inner array");
        }
    } else {
        panic!("expected array");
    }
}

#[test]
fn test_parse_text5_records() {
    let text = "[\n\t {\n\t \"precision\": \"zip\",\n\t \"Latitude\":  37.7668,\n\t \"Longitude\": -122.3959,\n\t \"Address\":   \"\",\n\t \"City\":      \"SAN FRANCISCO\",\n\t \"State\":     \"CA\",\n\t \"Zip\":       \"94107\",\n\t \"Country\":   \"US\"\n\t },\n\t {\n\t \"precision\": \"zip\",\n\t \"Latitude\":  37.371991,\n\t \"Longitude\": -122.026020,\n\t \"Address\":   \"\",\n\t \"City\":      \"SUNNYVALE\",\n\t \"State\":     \"CA\",\n\t \"Zip\":       \"94085\",\n\t \"Country\":   \"US\"\n\t }\n\t ]";
    let j = parse(text, false).unwrap();
    if let CJson::Array(records) = &j {
        assert_eq!(records.len(), 2);
        if let CJson::Object(r0) = &records[0] {
            assert_eq!(r0.get("precision"), Some(&CJson::String("zip".into())));
            assert_eq!(r0.get("City"), Some(&CJson::String("SAN FRANCISCO".into())));
        } else {
            panic!("expected object");
        }
    } else {
        panic!("expected array");
    }
}

// ── Display trait ───────────────────────────────────────────────────

#[test]
fn test_display_trait() {
    let j = CJson::Number(42.0);
    assert_eq!(format!("{}", j), "42");
}

// ── Escape string edge cases ────────────────────────────────────────

#[test]
fn test_print_string_backspace() {
    let j = CJson::String("\u{0008}".into());
    assert_eq!(j.print_unformatted(), "\"\\b\"");
}

#[test]
fn test_print_string_formfeed() {
    let j = CJson::String("\u{000C}".into());
    assert_eq!(j.print_unformatted(), "\"\\f\"");
}

#[test]
fn test_print_string_cr() {
    let j = CJson::String("\r".into());
    assert_eq!(j.print_unformatted(), "\"\\r\"");
}

#[test]
fn test_print_control_char() {
    // Control char < 32 that isn't a named escape → \u00XX
    let j = CJson::String("\u{0001}".into());
    assert_eq!(j.print_unformatted(), "\"\\u0001\"");
}

// ── Test file parsing (test1..test5 JSON files) ─────────────────────

#[test]
fn test_parse_test1_glossary() {
    let input = r#"{"glossary":{"title":"example glossary","GlossDiv":{"title":"S","GlossList":{"GlossEntry":{"ID":"SGML","SortAs":"SGML","GlossTerm":"Standard Generalized Markup Language","Acronym":"SGML","Abbrev":"ISO 8879:1986","GlossDef":{"para":"A meta-markup language, used to create markup languages such as DocBook.","GlossSeeAlso":["GML","XML"]},"GlossSee":"markup"}}}}}"#;
    let j = parse(input, false).unwrap();
    if let CJson::Object(root) = &j {
        let glossary = root.get("glossary").unwrap();
        if let CJson::Object(g) = glossary {
            assert_eq!(g.get("title"), Some(&CJson::String("example glossary".into())));
        } else {
            panic!("expected object");
        }
    } else {
        panic!("expected object");
    }
}

#[test]
fn test_parse_test5_menu_with_nulls() {
    // test5 has null items in an array
    let input = r#"{"menu":{"header":"SVG Viewer","items":[{"id":"Open"},null,{"id":"Find","label":"Find..."}]}}"#;
    let j = parse(input, false).unwrap();
    if let CJson::Object(root) = &j {
        if let Some(CJson::Object(menu)) = root.get("menu") {
            if let Some(CJson::Array(items)) = menu.get("items") {
                assert_eq!(items[1], CJson::Null);
            } else {
                panic!("expected items array");
            }
        } else {
            panic!("expected menu object");
        }
    } else {
        panic!("expected object");
    }
}

// ── Build and print objects (like create_objects in C) ──────────────

#[test]
fn test_build_object_and_print() {
    let mut root = CJson::create_object();
    root.add_item_to_object("name", CJson::create_string("Jack (\"Bee\") Nimble"))
        .unwrap();
    let mut fmt = CJson::create_object();
    fmt.add_item_to_object("type", CJson::create_string("rect")).unwrap();
    fmt.add_item_to_object("width", CJson::create_number(1920.0)).unwrap();
    root.add_item_to_object("format", fmt).unwrap();

    // Verify we can access what we built
    assert_eq!(
        root.get_object_item("name"),
        Some(&CJson::String("Jack (\"Bee\") Nimble".into()))
    );
    if let Some(CJson::Object(f)) = root.get_object_item("format") {
        assert_eq!(f.get("type"), Some(&CJson::String("rect".into())));
        assert_eq!(f.get("width"), Some(&CJson::Number(1920.0)));
    } else {
        panic!("expected format object");
    }
}

#[test]
fn test_build_array_and_print() {
    let mut arr = CJson::create_array();
    arr.add_item_to_array(CJson::create_number(1.0)).unwrap();
    arr.add_item_to_array(CJson::create_number(2.0)).unwrap();
    arr.add_item_to_array(CJson::create_number(3.0)).unwrap();
    assert_eq!(arr.get_array_size(), Some(3));
    assert_eq!(arr.get_array_item(0), Some(&CJson::Number(1.0)));
    assert_eq!(arr.get_array_item(2), Some(&CJson::Number(3.0)));
}

fn main() {}
