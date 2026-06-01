use emlang::data::{DataType, DataValue};
use emlang::em::EmType;
use emlang::parser::{Parser, ParserError, PARSER_MAX_NESTS, PARSER_MAX_TOKEN_LENGTH};

#[test]
fn test_constants() {
    assert_eq!(PARSER_MAX_TOKEN_LENGTH, 1024);
    assert_eq!(PARSER_MAX_NESTS, 256);
}

#[test]
fn test_parser_error_display() {
    assert_eq!(format!("{}", ParserError::UnexpectedEscape), "Unexpected escape");
    assert_eq!(format!("{}", ParserError::UnknownEscape), "Unknown escape");
    assert_eq!(format!("{}", ParserError::UnterminatedQuotes), "Unterminated quotes");
    assert_eq!(format!("{}", ParserError::UnexpectedEnd), "Unexpected end");
    assert_eq!(format!("{}", ParserError::IllegalPrintNest), "Illegal print nesting");
    assert_eq!(format!("{}", ParserError::ExpectedEnd), "Expected matching end");
}

#[test]
fn test_parser_new() {
    let p = Parser::new();
    assert_eq!(p.row, 1);
    assert_eq!(p.col, 0);
    assert_eq!(p.pos, 0);
    assert_eq!(p.from_file, false);
    assert_eq!(p.tok_len, 0);
}

#[test]
fn test_parse_empty() {
    let mut p = Parser::new();
    p.load_mem("");
    let r = p.parse();
    let prog = r.prog.expect("parse should succeed on empty");
    assert_eq!(prog.size, 0);
}

#[test]
fn test_parse_integer() {
    let mut p = Parser::new();
    p.load_mem("42");
    let r = p.parse();
    let prog = r.prog.expect("parse should succeed");
    // Note: token-without-trailing-whitespace is not pushed (matches C)
    assert_eq!(prog.size, 0);
}

#[test]
fn test_parse_integer_with_space() {
    let mut p = Parser::new();
    p.load_mem("42 ");
    let r = p.parse();
    let prog = r.prog.expect("parse should succeed");
    assert_eq!(prog.size, 1);
    assert_eq!(prog.ems[0].em_type, EmType::Push);
    assert_eq!(prog.ems[0].data.dtype, DataType::Int);
    if let DataValue::Int(v) = prog.ems[0].data.value {
        assert_eq!(v, 42);
    } else {
        panic!()
    }
}

#[test]
fn test_parse_negative_integer() {
    let mut p = Parser::new();
    p.load_mem("-15 ");
    let r = p.parse();
    let prog = r.prog.expect("parse should succeed");
    assert_eq!(prog.size, 1);
    assert_eq!(prog.ems[0].em_type, EmType::Push);
    if let DataValue::Int(v) = prog.ems[0].data.value {
        assert_eq!(v, -15);
    } else {
        panic!()
    }
}

#[test]
fn test_parse_dash_only_is_string() {
    // A single - is treated as a string token
    let mut p = Parser::new();
    p.load_mem("- ");
    let r = p.parse();
    let prog = r.prog.expect("parse should succeed");
    assert_eq!(prog.size, 1);
    assert_eq!(prog.ems[0].data.dtype, DataType::Str);
}

#[test]
fn test_parse_quoted_string() {
    let mut p = Parser::new();
    p.load_mem("\"Hello, world!\"");
    let r = p.parse();
    let prog = r.prog.expect("parse should succeed");
    assert_eq!(prog.size, 1);
    assert_eq!(prog.ems[0].em_type, EmType::Push);
    assert_eq!(prog.ems[0].data.dtype, DataType::Str);
    if let DataValue::Str(s) = &prog.ems[0].data.value {
        assert_eq!(s, "Hello, world!");
    } else {
        panic!()
    }
}

#[test]
fn test_parse_quoted_string_escapes() {
    let mut p = Parser::new();
    p.load_mem("\"a\\nb\\tc\"");
    let r = p.parse();
    let prog = r.prog.expect("parse should succeed");
    assert_eq!(prog.size, 1);
    if let DataValue::Str(s) = &prog.ems[0].data.value {
        assert_eq!(s, "a\nb\tc");
    } else {
        panic!()
    }
}

#[test]
fn test_parse_unterminated_quotes() {
    let mut p = Parser::new();
    p.load_mem("\"foo");
    let r = p.parse();
    assert!(r.prog.is_err());
    if let Err(e) = r.prog {
        assert_eq!(e, ParserError::UnterminatedQuotes);
    }
}

#[test]
fn test_parse_unknown_escape() {
    let mut p = Parser::new();
    p.load_mem("\"\\q\"");
    let r = p.parse();
    assert!(r.prog.is_err());
    if let Err(e) = r.prog {
        assert_eq!(e, ParserError::UnknownEscape);
    }
}

#[test]
fn test_parse_keywords() {
    let mut p = Parser::new();
    p.load_mem(":P ;) ;( x) x( :> :< :| x| :D :S X_X ");
    let r = p.parse();
    let prog = r.prog.expect("parse");
    assert_eq!(prog.ems[0].em_type, EmType::Pop);
    assert_eq!(prog.ems[1].em_type, EmType::Add);
    assert_eq!(prog.ems[2].em_type, EmType::Sub);
    assert_eq!(prog.ems[3].em_type, EmType::Mul);
    assert_eq!(prog.ems[4].em_type, EmType::Div);
    assert_eq!(prog.ems[5].em_type, EmType::Grt);
    assert_eq!(prog.ems[6].em_type, EmType::Less);
    assert_eq!(prog.ems[7].em_type, EmType::Equ);
    assert_eq!(prog.ems[8].em_type, EmType::Nequ);
    assert_eq!(prog.ems[9].em_type, EmType::Dup);
    assert_eq!(prog.ems[10].em_type, EmType::Swap);
    assert_eq!(prog.ems[11].em_type, EmType::Exit);
}

#[test]
fn test_parse_print_block() {
    let mut p = Parser::new();
    p.load_mem(":O Hello :)\n");
    let r = p.parse();
    let prog = r.prog.expect("parse");
    assert_eq!(prog.ems[0].em_type, EmType::PrintBegin);
    assert_eq!(prog.ems[1].em_type, EmType::Push);
    assert_eq!(prog.ems[2].em_type, EmType::PrintEnd);
    // PrintEnd carries DATA_STDOUT (1)
    if let DataValue::Int(v) = prog.ems[2].data.value {
        assert_eq!(v, 1);
    }
    // Cross-ref: print_begin.ref points to print_end's index
    assert_eq!(prog.ems[0].r#ref, 2);
    assert_eq!(prog.ems[2].r#ref, 0);
}

#[test]
fn test_parse_stderr_print() {
    let mut p = Parser::new();
    p.load_mem(":O Hello :(\n");
    let r = p.parse();
    let prog = r.prog.expect("parse");
    assert_eq!(prog.ems[2].em_type, EmType::PrintEnd);
    if let DataValue::Int(v) = prog.ems[2].data.value {
        assert_eq!(v, 2);
    }
}

#[test]
fn test_parse_special_strings() {
    let mut p = Parser::new();
    p.load_mem(":3 ;3 x3 ><> <3 ");
    let r = p.parse();
    let prog = r.prog.expect("parse");
    let expected = ["meow", "nya", "rawr", "le fishe", "i <3 emlang"];
    for (i, exp) in expected.iter().enumerate() {
        assert_eq!(prog.ems[i].em_type, EmType::Push);
        if let DataValue::Str(s) = &prog.ems[i].data.value {
            assert_eq!(s, exp);
        } else {
            panic!()
        }
    }
}

#[test]
fn test_parse_comment() {
    let mut p = Parser::new();
    p.load_mem(":x This is a comment\n42 ");
    let r = p.parse();
    let prog = r.prog.expect("parse");
    // Only the 42 should remain (comment ignored)
    assert_eq!(prog.size, 1);
    if let DataValue::Int(v) = prog.ems[0].data.value {
        assert_eq!(v, 42);
    }
}

#[test]
fn test_parse_if_block() {
    let mut p = Parser::new();
    p.load_mem("1 :/ 5 :\\\n");
    let r = p.parse();
    let prog = r.prog.expect("parse");
    assert_eq!(prog.ems[0].em_type, EmType::Push);
    assert_eq!(prog.ems[1].em_type, EmType::IfBegin);
    assert_eq!(prog.ems[2].em_type, EmType::Push);
    assert_eq!(prog.ems[3].em_type, EmType::IfEnd);
    assert_eq!(prog.ems[1].r#ref, 3);
    assert_eq!(prog.ems[3].r#ref, 1);
}

#[test]
fn test_parse_loop_block() {
    let mut p = Parser::new();
    p.load_mem("1 :@ 5 @:\n");
    let r = p.parse();
    let prog = r.prog.expect("parse");
    assert_eq!(prog.ems[0].em_type, EmType::Push);
    assert_eq!(prog.ems[1].em_type, EmType::LoopBegin);
    assert_eq!(prog.ems[3].em_type, EmType::LoopEnd);
    assert_eq!(prog.ems[1].r#ref, 3);
    assert_eq!(prog.ems[3].r#ref, 1);
}

#[test]
fn test_parse_unexpected_end() {
    let mut p = Parser::new();
    p.load_mem(":\\ ");
    let r = p.parse();
    assert!(r.prog.is_err());
    if let Err(e) = r.prog {
        assert_eq!(e, ParserError::UnexpectedEnd);
    }
}

#[test]
fn test_parse_expected_end() {
    let mut p = Parser::new();
    p.load_mem(":/ 5 ");
    let r = p.parse();
    assert!(r.prog.is_err());
    if let Err(e) = r.prog {
        assert_eq!(e, ParserError::ExpectedEnd);
    }
}

#[test]
fn test_parse_illegal_print_nest() {
    let mut p = Parser::new();
    p.load_mem(":O foo :O bar :) :) ");
    let r = p.parse();
    assert!(r.prog.is_err());
    if let Err(e) = r.prog {
        assert_eq!(e, ParserError::IllegalPrintNest);
    }
}

#[test]
fn test_parse_unquoted_string() {
    let mut p = Parser::new();
    p.load_mem("Hello! ");
    let r = p.parse();
    let prog = r.prog.expect("parse");
    assert_eq!(prog.size, 1);
    assert_eq!(prog.ems[0].data.dtype, DataType::Str);
    if let DataValue::Str(s) = &prog.ems[0].data.value {
        assert_eq!(s, "Hello!");
    }
}

#[test]
fn test_parse_load_file_missing() {
    let mut p = Parser::new();
    let result = p.load_file("/nonexistent/path/that/does/not/exist.eml");
    assert_eq!(result, -1);
}

fn main() {}
