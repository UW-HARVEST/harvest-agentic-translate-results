use emlang::parser::{Parser, ParserError};
use emlang::em::EmType;
use emlang::data::{DataType, DataValue};

#[test]
fn test_parser_new() {
    let p = Parser::new();
    assert_eq!(p.row, 1);
    assert_eq!(p.col, 0);
    assert!(!p.from_file);
    assert_eq!(p.prog.size, 0);
}

#[test]
fn test_parse_empty() {
    let mut p = Parser::new();
    p.load_mem("");
    let r = p.parse();
    assert!(r.prog.is_ok());
    let prog = r.prog.unwrap();
    assert_eq!(prog.size, 0);
}

#[test]
fn test_parse_integer() {
    let mut p = Parser::new();
    p.load_mem("42\n");
    let r = p.parse();
    let prog = r.prog.unwrap();
    assert_eq!(prog.size, 1);
    assert_eq!(prog.ems[0].em_type, EmType::Push);
    assert!(matches!(prog.ems[0].data.value, DataValue::Int(42)));
}

#[test]
fn test_parse_negative_integer() {
    let mut p = Parser::new();
    p.load_mem("-5\n");
    let r = p.parse();
    let prog = r.prog.unwrap();
    assert_eq!(prog.size, 1);
    assert!(matches!(prog.ems[0].data.value, DataValue::Int(-5)));
}

#[test]
fn test_parse_zero() {
    let mut p = Parser::new();
    p.load_mem("0\n");
    let r = p.parse();
    let prog = r.prog.unwrap();
    assert_eq!(prog.size, 1);
    assert!(matches!(prog.ems[0].data.value, DataValue::Int(0)));
}

#[test]
fn test_parse_bare_minus_is_string() {
    let mut p = Parser::new();
    p.load_mem("- ");
    let r = p.parse();
    let prog = r.prog.unwrap();
    assert_eq!(prog.size, 1);
    assert_eq!(prog.ems[0].em_type, EmType::Push);
    assert_eq!(prog.ems[0].data.dtype, DataType::Str);
}

#[test]
fn test_parse_string_literal() {
    let mut p = Parser::new();
    p.load_mem("\"hello\"\n");
    let r = p.parse();
    let prog = r.prog.unwrap();
    assert_eq!(prog.size, 1);
    assert_eq!(prog.ems[0].em_type, EmType::Push);
    if let DataValue::Str(ref s) = prog.ems[0].data.value {
        assert_eq!(s, "hello");
    } else {
        panic!("expected string");
    }
}

#[test]
fn test_parse_string_escape_n() {
    let mut p = Parser::new();
    p.load_mem("\"a\\nb\"\n");
    let r = p.parse();
    let prog = r.prog.unwrap();
    if let DataValue::Str(ref s) = prog.ems[0].data.value {
        assert_eq!(s, "a\nb");
    } else {
        panic!("expected string");
    }
}

#[test]
fn test_parse_string_escape_backslash() {
    let mut p = Parser::new();
    p.load_mem("\"a\\\\b\"\n");
    let r = p.parse();
    let prog = r.prog.unwrap();
    if let DataValue::Str(ref s) = prog.ems[0].data.value {
        assert_eq!(s, "a\\b");
    } else {
        panic!("expected string");
    }
}

#[test]
fn test_parse_string_escape_quote() {
    let mut p = Parser::new();
    p.load_mem("\"a\\\"b\"\n");
    let r = p.parse();
    let prog = r.prog.unwrap();
    if let DataValue::Str(ref s) = prog.ems[0].data.value {
        assert_eq!(s, "a\"b");
    } else {
        panic!("expected string");
    }
}

#[test]
fn test_parse_unterminated_quotes() {
    let mut p = Parser::new();
    p.load_mem("\"hello");
    let r = p.parse();
    assert!(r.prog.is_err());
    assert_eq!(r.prog.unwrap_err(), ParserError::UnterminatedQuotes);
}

#[test]
fn test_parse_unknown_escape() {
    let mut p = Parser::new();
    p.load_mem("\"\\z\"");
    let r = p.parse();
    assert!(r.prog.is_err());
    assert_eq!(r.prog.unwrap_err(), ParserError::UnknownEscape);
}

#[test]
fn test_parse_comment() {
    let mut p = Parser::new();
    p.load_mem(":x this is a comment\n42\n");
    let r = p.parse();
    let prog = r.prog.unwrap();
    assert_eq!(prog.size, 1);
    assert!(matches!(prog.ems[0].data.value, DataValue::Int(42)));
}

#[test]
fn test_parse_comment_only() {
    let mut p = Parser::new();
    p.load_mem(":x just a comment\n");
    let r = p.parse();
    let prog = r.prog.unwrap();
    assert_eq!(prog.size, 0);
}

#[test]
fn test_parse_keywords() {
    let cases = vec![
        (":P", EmType::Pop),
        (";)", EmType::Add),
        (";(", EmType::Sub),
        ("x)", EmType::Mul),
        ("x(", EmType::Div),
        (":>", EmType::Grt),
        (":<", EmType::Less),
        (":|", EmType::Equ),
        ("x|", EmType::Nequ),
        (":D", EmType::Dup),
        (":S", EmType::Swap),
        ("X_X", EmType::Exit),
    ];
    for (kw, expected_type) in cases {
        let mut p = Parser::new();
        p.load_mem(&format!("{}\n", kw));
        let r = p.parse();
        let prog = r.prog.unwrap();
        assert_eq!(prog.ems[0].em_type, expected_type, "keyword: {}", kw);
    }
}

#[test]
fn test_parse_print_begin_end() {
    let mut p = Parser::new();
    p.load_mem(":O 42 :)\n");
    let r = p.parse();
    let prog = r.prog.unwrap();
    assert_eq!(prog.ems[0].em_type, EmType::PrintBegin);
    assert_eq!(prog.ems[1].em_type, EmType::Push);
    assert_eq!(prog.ems[2].em_type, EmType::PrintEnd);
    assert_eq!(prog.ems[0].r#ref, 2);
    assert_eq!(prog.ems[2].r#ref, 0);
}

#[test]
fn test_parse_print_stderr() {
    let mut p = Parser::new();
    p.load_mem(":O hello :(\n");
    let r = p.parse();
    let prog = r.prog.unwrap();
    assert_eq!(prog.ems[2].em_type, EmType::PrintEnd);
    if let DataValue::Int(v) = prog.ems[2].data.value {
        assert_eq!(v, 2); // DATA_STDERR
    } else {
        panic!("expected int for PrintEnd data");
    }
}

#[test]
fn test_parse_if() {
    let mut p = Parser::new();
    p.load_mem("1 :/ 42 :\\\n");
    let r = p.parse();
    let prog = r.prog.unwrap();
    assert_eq!(prog.ems[1].em_type, EmType::IfBegin);
    assert_eq!(prog.ems[3].em_type, EmType::IfEnd);
    assert_eq!(prog.ems[1].r#ref, 3);
    assert_eq!(prog.ems[3].r#ref, 1);
}

#[test]
fn test_parse_loop() {
    let mut p = Parser::new();
    p.load_mem("1 :@ 42 @:\n");
    let r = p.parse();
    let prog = r.prog.unwrap();
    assert_eq!(prog.ems[1].em_type, EmType::LoopBegin);
    assert_eq!(prog.ems[3].em_type, EmType::LoopEnd);
    assert_eq!(prog.ems[1].r#ref, 3);
    assert_eq!(prog.ems[3].r#ref, 1);
}

#[test]
fn test_parse_unmatched_end() {
    let mut p = Parser::new();
    p.load_mem(":\\\n");
    let r = p.parse();
    assert!(r.prog.is_err());
    assert_eq!(r.prog.unwrap_err(), ParserError::UnexpectedEnd);
}

#[test]
fn test_parse_unmatched_begin() {
    let mut p = Parser::new();
    p.load_mem("1 :/\n");
    let r = p.parse();
    assert!(r.prog.is_err());
    assert_eq!(r.prog.unwrap_err(), ParserError::ExpectedEnd);
}

#[test]
fn test_parse_illegal_print_nest() {
    let mut p = Parser::new();
    p.load_mem(":O :O :) :)\n");
    let r = p.parse();
    assert!(r.prog.is_err());
    assert_eq!(r.prog.unwrap_err(), ParserError::IllegalPrintNest);
}

#[test]
fn test_parse_emoticon_meow() {
    let mut p = Parser::new();
    p.load_mem(":3\n");
    let r = p.parse();
    let prog = r.prog.unwrap();
    if let DataValue::Str(ref s) = prog.ems[0].data.value {
        assert_eq!(s, "meow");
    } else {
        panic!("expected string");
    }
}

#[test]
fn test_parse_emoticon_nya() {
    let mut p = Parser::new();
    p.load_mem(";3\n");
    let r = p.parse();
    let prog = r.prog.unwrap();
    if let DataValue::Str(ref s) = prog.ems[0].data.value {
        assert_eq!(s, "nya");
    } else {
        panic!("expected string");
    }
}

#[test]
fn test_parse_emoticon_rawr() {
    let mut p = Parser::new();
    p.load_mem("x3\n");
    let r = p.parse();
    let prog = r.prog.unwrap();
    if let DataValue::Str(ref s) = prog.ems[0].data.value {
        assert_eq!(s, "rawr");
    } else {
        panic!("expected string");
    }
}

#[test]
fn test_parse_emoticon_fishe() {
    let mut p = Parser::new();
    p.load_mem("><>\n");
    let r = p.parse();
    let prog = r.prog.unwrap();
    if let DataValue::Str(ref s) = prog.ems[0].data.value {
        assert_eq!(s, "le fishe");
    } else {
        panic!("expected string");
    }
}

#[test]
fn test_parse_emoticon_love() {
    let mut p = Parser::new();
    p.load_mem("<3\n");
    let r = p.parse();
    let prog = r.prog.unwrap();
    if let DataValue::Str(ref s) = prog.ems[0].data.value {
        assert_eq!(s, "i <3 emlang");
    } else {
        panic!("expected string");
    }
}

#[test]
fn test_parse_plain_word_as_string() {
    let mut p = Parser::new();
    p.load_mem("hello\n");
    let r = p.parse();
    let prog = r.prog.unwrap();
    assert_eq!(prog.ems[0].em_type, EmType::Push);
    assert_eq!(prog.ems[0].data.dtype, DataType::Str);
    if let DataValue::Str(ref s) = prog.ems[0].data.value {
        assert_eq!(s, "hello");
    } else {
        panic!("expected string");
    }
}

#[test]
fn test_parse_unexpected_escape() {
    let mut p = Parser::new();
    p.load_mem("\\ ");
    let r = p.parse();
    assert!(r.prog.is_err());
    assert_eq!(r.prog.unwrap_err(), ParserError::UnexpectedEscape);
}

#[test]
fn test_parse_escaped_quote_outside_string() {
    let mut p = Parser::new();
    p.load_mem("\\\" ");
    let r = p.parse();
    let prog = r.prog.unwrap();
    assert_eq!(prog.ems[0].data.dtype, DataType::Str);
    if let DataValue::Str(ref s) = prog.ems[0].data.value {
        assert_eq!(s, "\"");
    } else {
        panic!("expected string");
    }
}

#[test]
fn test_load_file_nonexistent() {
    let mut p = Parser::new();
    assert_ne!(p.load_file("/nonexistent/path.eml"), 0);
}

#[test]
fn test_load_file_and_parse() {
    let mut p = Parser::new();
    assert_eq!(p.load_file("resources/tests/hello_world.eml"), 0);
    let r = p.parse();
    let prog = r.prog.unwrap();
    assert!(prog.size >= 3);
    assert_eq!(prog.ems[0].em_type, EmType::PrintBegin);
}

#[test]
fn test_parse_multiple_tokens() {
    let mut p = Parser::new();
    p.load_mem("1 2 ;)\n");
    let r = p.parse();
    let prog = r.prog.unwrap();
    assert_eq!(prog.size, 3);
    assert!(matches!(prog.ems[0].data.value, DataValue::Int(1)));
    assert!(matches!(prog.ems[1].data.value, DataValue::Int(2)));
    assert_eq!(prog.ems[2].em_type, EmType::Add);
}

#[test]
fn test_parse_mismatched_end_types() {
    let mut p = Parser::new();
    p.load_mem("1 :/ 42 @:\n");
    let r = p.parse();
    assert!(r.prog.is_err());
    assert_eq!(r.prog.unwrap_err(), ParserError::UnexpectedEnd);
}

fn main() {}
