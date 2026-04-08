use emlang::data::{DataType, DataValue};
use emlang::em::EmType;
use emlang::parser::{Parser, ParserError};

fn parse_ok(input: &str) -> emlang::parser::ParserResult {
    let mut p = Parser::new();
    p.load_mem(input);
    let r = p.parse();
    assert!(r.prog.is_ok(), "expected Ok, got {:?}", r.prog);
    r
}

fn parse_err(input: &str) -> ParserError {
    let mut p = Parser::new();
    p.load_mem(input);
    let r = p.parse();
    r.prog.unwrap_err()
}

#[test]
fn test_parse_integer() {
    let r = parse_ok("42 ");
    let prog = r.prog.unwrap();
    assert_eq!(prog.size, 1);
    assert_eq!(prog.ems[0].em_type, EmType::Push);
    match prog.ems[0].data.value { DataValue::Int(v) => assert_eq!(v, 42), _ => panic!() }
}

#[test]
fn test_parse_negative_integer() {
    let r = parse_ok("-5 ");
    let prog = r.prog.unwrap();
    assert_eq!(prog.size, 1);
    assert_eq!(prog.ems[0].em_type, EmType::Push);
    match prog.ems[0].data.value { DataValue::Int(v) => assert_eq!(v, -5), _ => panic!() }
}

#[test]
fn test_parse_string_literal() {
    let r = parse_ok("\"hello\" ");
    let prog = r.prog.unwrap();
    assert_eq!(prog.size, 1);
    assert_eq!(prog.ems[0].em_type, EmType::Push);
    assert_eq!(prog.ems[0].data.dtype, DataType::Str);
    match &prog.ems[0].data.value { DataValue::Str(s) => assert_eq!(s, "hello"), _ => panic!() }
}

#[test]
fn test_parse_keywords() {
    let r = parse_ok(":P ;) ;( x) x( :> :< :| x| ");
    let prog = r.prog.unwrap();
    assert_eq!(prog.size, 9);
    let expected = [EmType::Pop, EmType::Add, EmType::Sub, EmType::Mul, EmType::Div,
                    EmType::Grt, EmType::Less, EmType::Equ, EmType::Nequ];
    for (i, &et) in expected.iter().enumerate() {
        assert_eq!(prog.ems[i].em_type, et);
    }
}

#[test]
fn test_parse_print_block() {
    let r = parse_ok(":O 42 :) ");
    let prog = r.prog.unwrap();
    assert_eq!(prog.size, 3);
    assert_eq!(prog.ems[0].em_type, EmType::PrintBegin);
    assert_eq!(prog.ems[1].em_type, EmType::Push);
    assert_eq!(prog.ems[2].em_type, EmType::PrintEnd);
    assert_eq!(prog.ems[0].r#ref, 2);
    assert_eq!(prog.ems[2].r#ref, 0);
}

#[test]
fn test_parse_if_block() {
    let r = parse_ok("1 :/ 42 :\\ ");
    let prog = r.prog.unwrap();
    assert_eq!(prog.size, 4);
    assert_eq!(prog.ems[0].em_type, EmType::Push);
    assert_eq!(prog.ems[1].em_type, EmType::IfBegin);
    assert_eq!(prog.ems[2].em_type, EmType::Push);
    assert_eq!(prog.ems[3].em_type, EmType::IfEnd);
    assert_eq!(prog.ems[1].r#ref, 3);
    assert_eq!(prog.ems[3].r#ref, 1);
}

#[test]
fn test_parse_loop_block() {
    let r = parse_ok("1 :@ 42 @: ");
    let prog = r.prog.unwrap();
    assert_eq!(prog.size, 4);
    assert_eq!(prog.ems[0].em_type, EmType::Push);
    assert_eq!(prog.ems[1].em_type, EmType::LoopBegin);
    assert_eq!(prog.ems[2].em_type, EmType::Push);
    assert_eq!(prog.ems[3].em_type, EmType::LoopEnd);
    assert_eq!(prog.ems[1].r#ref, 3);
    assert_eq!(prog.ems[3].r#ref, 1);
}

#[test]
fn test_parse_comment() {
    let r = parse_ok(":x this is a comment\n42 ");
    let prog = r.prog.unwrap();
    assert_eq!(prog.size, 1);
    assert_eq!(prog.ems[0].em_type, EmType::Push);
    match prog.ems[0].data.value { DataValue::Int(v) => assert_eq!(v, 42), _ => panic!() }
}

#[test]
fn test_parse_emoticons() {
    let r = parse_ok(":3 ;3 <3 x3 ><> ");
    let prog = r.prog.unwrap();
    assert_eq!(prog.size, 5);
    let expected_strs = ["meow", "nya", "i <3 emlang", "rawr", "le fishe"];
    for (i, &exp) in expected_strs.iter().enumerate() {
        assert_eq!(prog.ems[i].em_type, EmType::Push);
        assert_eq!(prog.ems[i].data.dtype, DataType::Str);
        match &prog.ems[i].data.value { DataValue::Str(s) => assert_eq!(s, exp), _ => panic!() }
    }
}

#[test]
fn test_parse_escape_sequences() {
    let r = parse_ok("\"a\\nb\\tc\" ");
    let prog = r.prog.unwrap();
    assert_eq!(prog.size, 1);
    match &prog.ems[0].data.value {
        DataValue::Str(s) => {
            assert_eq!(s.len(), 5); // a \n b \t c
            assert_eq!(s, "a\nb\tc");
        }
        _ => panic!(),
    }
}

#[test]
fn test_parse_unterminated_quotes() {
    let err = parse_err("\"unterminated");
    assert_eq!(err, ParserError::UnterminatedQuotes);
}

#[test]
fn test_parse_unknown_escape() {
    let err = parse_err("\"\\z\"");
    assert_eq!(err, ParserError::UnknownEscape);
}

#[test]
fn test_parse_expected_end() {
    let err = parse_err(":/ 1 ");
    assert_eq!(err, ParserError::ExpectedEnd);
}

#[test]
fn test_parse_unexpected_end() {
    let err = parse_err(":\\ ");
    assert_eq!(err, ParserError::UnexpectedEnd);
}

#[test]
fn test_parse_empty() {
    let r = parse_ok("");
    let prog = r.prog.unwrap();
    assert_eq!(prog.size, 0);
}

#[test]
fn test_parse_minus_sign_as_string() {
    let r = parse_ok("- ");
    let prog = r.prog.unwrap();
    assert_eq!(prog.size, 1);
    assert_eq!(prog.ems[0].em_type, EmType::Push);
    assert_eq!(prog.ems[0].data.dtype, DataType::Str);
    match &prog.ems[0].data.value { DataValue::Str(s) => assert_eq!(s, "-"), _ => panic!() }
}

#[test]
fn test_parse_backslash_quote() {
    let r = parse_ok("\\\" ");
    let prog = r.prog.unwrap();
    assert_eq!(prog.size, 1);
    assert_eq!(prog.ems[0].em_type, EmType::Push);
    assert_eq!(prog.ems[0].data.dtype, DataType::Str);
    match &prog.ems[0].data.value { DataValue::Str(s) => assert_eq!(s, "\""), _ => panic!() }
}

#[test]
fn test_parse_exit() {
    let r = parse_ok("X_X ");
    let prog = r.prog.unwrap();
    assert_eq!(prog.size, 1);
    assert_eq!(prog.ems[0].em_type, EmType::Exit);
}

#[test]
fn test_parse_dup_swap() {
    let r = parse_ok(":D :S ");
    let prog = r.prog.unwrap();
    assert_eq!(prog.size, 2);
    assert_eq!(prog.ems[0].em_type, EmType::Dup);
    assert_eq!(prog.ems[1].em_type, EmType::Swap);
}

#[test]
fn test_parse_stderr_print() {
    let r = parse_ok(":O 1 :( ");
    let prog = r.prog.unwrap();
    assert_eq!(prog.size, 3);
    assert_eq!(prog.ems[2].em_type, EmType::PrintEnd);
    match prog.ems[2].data.value { DataValue::Int(v) => assert_eq!(v, 2), _ => panic!() }
}

#[test]
fn test_parse_nested_print_error() {
    let err = parse_err(":O :O :) ");
    assert_eq!(err, ParserError::IllegalPrintNest);
}

#[test]
fn test_parse_row_col_tracking() {
    let r = parse_ok("42 99\nhello ");
    let prog = r.prog.unwrap();
    assert_eq!(prog.ems[0].row, 1);
    assert_eq!(prog.ems[0].col, 1);
    assert_eq!(prog.ems[1].row, 1);
    assert_eq!(prog.ems[1].col, 4);
    assert_eq!(prog.ems[2].row, 2);
    assert_eq!(prog.ems[2].col, 1);
}

#[test]
fn test_parse_backslash_at_end() {
    let err = parse_err("\\ ");
    assert_eq!(err, ParserError::UnexpectedEscape);
}

#[test]
fn test_parse_no_trailing_whitespace_drops_last_token() {
    // C behavior: last token without trailing whitespace is not pushed
    // So ":O 42 :)" loses the :) token, leaving unclosed print block -> ExpectedEnd
    let err = parse_err(":O 42 :)");
    assert_eq!(err, ParserError::ExpectedEnd);
}

#[test]
fn test_parse_no_trailing_whitespace_is_error_for_blocks() {
    let mut p = Parser::new();
    p.load_mem(":O 42 :)");
    let r = p.parse();
    assert!(r.prog.is_err());
    assert_eq!(r.prog.unwrap_err(), ParserError::ExpectedEnd);
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
fn test_load_file_nonexistent() {
    let mut p = Parser::new();
    let ret = p.load_file("/nonexistent/path/file.eml");
    assert_eq!(ret, -1);
}

fn main() {}
