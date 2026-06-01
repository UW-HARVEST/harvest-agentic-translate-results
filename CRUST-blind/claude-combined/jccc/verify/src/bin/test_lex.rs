use jccc::lex::{
    in_string, is_valid_numeric_or_id_char, starts_operator, test_ttype_from_string,
    test_ttype_many_chars, test_ttype_name, test_ttype_one_char, ttype_from_string,
    ttype_many_chars, ttype_name, ttype_one_char, valid_operator_sequence,
};
use jccc::token::TokenType;

#[test]
fn test_in_string() {
    assert_eq!(in_string('a', "abc"), 1);
    assert_eq!(in_string('z', "abc"), 0);
    assert_eq!(in_string('(', "(){}[];~#,.:?~"), 1);
    assert_eq!(in_string('!', "(){}[];~#,.:?~"), 0);
    assert_eq!(in_string('a', ""), 0);
}

#[test]
fn test_starts_operator_fn() {
    assert_eq!(starts_operator('-'), 1);
    assert_eq!(starts_operator('+'), 1);
    assert_eq!(starts_operator('*'), 1);
    assert_eq!(starts_operator('/'), 1);
    assert_eq!(starts_operator('='), 1);
    assert_eq!(starts_operator(':'), 1);
    assert_eq!(starts_operator('%'), 1);
    assert_eq!(starts_operator('&'), 1);
    assert_eq!(starts_operator('|'), 1);
    assert_eq!(starts_operator('<'), 1);
    assert_eq!(starts_operator('>'), 1);
    assert_eq!(starts_operator('!'), 1);
    assert_eq!(starts_operator('~'), 1);
    assert_eq!(starts_operator('^'), 1);
    assert_eq!(starts_operator('a'), 0);
    assert_eq!(starts_operator('('), 0);
    assert_eq!(starts_operator(';'), 0);
    assert_eq!(starts_operator('1'), 0);
}

#[test]
fn test_valid_operator_sequence_fn() {
    assert_eq!(valid_operator_sequence("+"), 1);
    assert_eq!(valid_operator_sequence("++"), 1);
    assert_eq!(valid_operator_sequence("+="), 1);
    assert_eq!(valid_operator_sequence("=="), 1);
    assert_eq!(valid_operator_sequence("<<="), 1);
    assert_eq!(valid_operator_sequence(">>="), 1);
    assert_eq!(valid_operator_sequence("->"), 1);
    assert_eq!(valid_operator_sequence("xyz"), 0);
    assert_eq!(valid_operator_sequence(""), 0);
    assert_eq!(valid_operator_sequence("+++"), 0);
    assert_eq!(valid_operator_sequence("&"), 1);
    assert_eq!(valid_operator_sequence("&&"), 1);
    assert_eq!(valid_operator_sequence("&&="), 1);
}

#[test]
fn test_is_valid_numeric_or_id_char_fn() {
    assert_eq!(is_valid_numeric_or_id_char('a'), 1);
    assert_eq!(is_valid_numeric_or_id_char('Z'), 1);
    assert_eq!(is_valid_numeric_or_id_char('0'), 1);
    assert_eq!(is_valid_numeric_or_id_char('9'), 1);
    assert_eq!(is_valid_numeric_or_id_char('_'), 1);
    assert_eq!(is_valid_numeric_or_id_char('.'), 1);
    assert_eq!(is_valid_numeric_or_id_char(' '), 0);
    assert_eq!(is_valid_numeric_or_id_char('('), 0);
    assert_eq!(is_valid_numeric_or_id_char('+'), 0);
    assert_eq!(is_valid_numeric_or_id_char('\n'), 0);
    assert_eq!(is_valid_numeric_or_id_char('-'), 0);
}

#[test]
fn test_ttype_one_char_fn() {
    assert_eq!(ttype_one_char('('), TokenType::TT_OPAREN);
    assert_eq!(ttype_one_char(')'), TokenType::TT_CPAREN);
    assert_eq!(ttype_one_char('{'), TokenType::TT_OBRACE);
    assert_eq!(ttype_one_char('}'), TokenType::TT_CBRACE);
    assert_eq!(ttype_one_char('['), TokenType::TT_OBRACKET);
    assert_eq!(ttype_one_char(']'), TokenType::TT_CBRACKET);
    assert_eq!(ttype_one_char(';'), TokenType::TT_SEMI);
    assert_eq!(ttype_one_char('.'), TokenType::TT_PERIOD);
    assert_eq!(ttype_one_char(','), TokenType::TT_COMMA);
    assert_eq!(ttype_one_char('-'), TokenType::TT_MINUS);
    assert_eq!(ttype_one_char('+'), TokenType::TT_PLUS);
    assert_eq!(ttype_one_char('*'), TokenType::TT_STAR);
    assert_eq!(ttype_one_char('/'), TokenType::TT_SLASH);
    assert_eq!(ttype_one_char('='), TokenType::TT_ASSIGN);
    assert_eq!(ttype_one_char(':'), TokenType::TT_COLON);
    assert_eq!(ttype_one_char('%'), TokenType::TT_MOD);
    assert_eq!(ttype_one_char('&'), TokenType::TT_BAND);
    assert_eq!(ttype_one_char('|'), TokenType::TT_BOR);
    assert_eq!(ttype_one_char('>'), TokenType::TT_GREATER);
    assert_eq!(ttype_one_char('<'), TokenType::TT_LESS);
    assert_eq!(ttype_one_char('!'), TokenType::TT_LNOT);
    assert_eq!(ttype_one_char('~'), TokenType::TT_BNOT);
    assert_eq!(ttype_one_char('^'), TokenType::TT_XOR);
    assert_eq!(ttype_one_char('#'), TokenType::TT_POUND);
    assert_eq!(ttype_one_char('?'), TokenType::TT_QMARK);
    assert_eq!(ttype_one_char('1'), TokenType::TT_LITERAL);
    assert_eq!(ttype_one_char('9'), TokenType::TT_LITERAL);
    assert_eq!(ttype_one_char('a'), TokenType::TT_IDENTIFIER);
    assert_eq!(ttype_one_char('Z'), TokenType::TT_IDENTIFIER);
}

#[test]
fn test_ttype_many_chars_fn() {
    assert_eq!(ttype_many_chars("auto"), TokenType::TT_AUTO);
    assert_eq!(ttype_many_chars("break"), TokenType::TT_BREAK);
    assert_eq!(ttype_many_chars("char"), TokenType::TT_CHAR);
    assert_eq!(ttype_many_chars("const"), TokenType::TT_CONST);
    assert_eq!(ttype_many_chars("case"), TokenType::TT_CASE);
    assert_eq!(ttype_many_chars("continue"), TokenType::TT_CONTINUE);
    assert_eq!(ttype_many_chars("do"), TokenType::TT_DO);
    assert_eq!(ttype_many_chars("double"), TokenType::TT_DOUBLE);
    assert_eq!(ttype_many_chars("default"), TokenType::TT_DEFAULT);
    assert_eq!(ttype_many_chars("enum"), TokenType::TT_ENUM);
    assert_eq!(ttype_many_chars("else"), TokenType::TT_ELSE);
    assert_eq!(ttype_many_chars("extern"), TokenType::TT_EXTERN);
    assert_eq!(ttype_many_chars("float"), TokenType::TT_FLOAT);
    assert_eq!(ttype_many_chars("for"), TokenType::TT_FOR);
    assert_eq!(ttype_many_chars("goto"), TokenType::TT_GOTO);
    assert_eq!(ttype_many_chars("int"), TokenType::TT_INT);
    assert_eq!(ttype_many_chars("if"), TokenType::TT_IF);
    assert_eq!(ttype_many_chars("long"), TokenType::TT_LONG);
    assert_eq!(ttype_many_chars("return"), TokenType::TT_RETURN);
    assert_eq!(ttype_many_chars("register"), TokenType::TT_REGISTER);
    assert_eq!(ttype_many_chars("static"), TokenType::TT_STATIC);
    assert_eq!(ttype_many_chars("switch"), TokenType::TT_SWITCH);
    assert_eq!(ttype_many_chars("short"), TokenType::TT_SHORT);
    assert_eq!(ttype_many_chars("signed"), TokenType::TT_SIGNED);
    assert_eq!(ttype_many_chars("struct"), TokenType::TT_STRUCT);
    assert_eq!(ttype_many_chars("sizeof"), TokenType::TT_SIZEOF);
    assert_eq!(ttype_many_chars("typedef"), TokenType::TT_TYPEDEF);
    assert_eq!(ttype_many_chars("union"), TokenType::TT_UNION);
    assert_eq!(ttype_many_chars("unsigned"), TokenType::TT_UNSIGNED);
    assert_eq!(ttype_many_chars("void"), TokenType::TT_VOID);
    assert_eq!(ttype_many_chars("volatile"), TokenType::TT_VOLATILE);
    assert_eq!(ttype_many_chars("while"), TokenType::TT_WHILE);

    // Operators
    assert_eq!(ttype_many_chars("&&"), TokenType::TT_LAND);
    assert_eq!(ttype_many_chars("||"), TokenType::TT_LOR);
    assert_eq!(ttype_many_chars("-="), TokenType::TT_DEC);
    assert_eq!(ttype_many_chars("+="), TokenType::TT_INC);
    assert_eq!(ttype_many_chars("++"), TokenType::TT_PLUSPLUS);
    assert_eq!(ttype_many_chars("--"), TokenType::TT_MINUSMINUS);
    assert_eq!(ttype_many_chars("/="), TokenType::TT_DIVEQ);
    assert_eq!(ttype_many_chars("*="), TokenType::TT_MULEQ);
    assert_eq!(ttype_many_chars("%="), TokenType::TT_MODEQ);
    assert_eq!(ttype_many_chars("&="), TokenType::TT_BANDEQ);
    assert_eq!(ttype_many_chars("|="), TokenType::TT_BOREQ);
    assert_eq!(ttype_many_chars("&&="), TokenType::TT_LANDEQ);
    assert_eq!(ttype_many_chars("||="), TokenType::TT_LOREQ);
    assert_eq!(ttype_many_chars("<="), TokenType::TT_LESSEQ);
    assert_eq!(ttype_many_chars(">="), TokenType::TT_GREATEREQ);
    assert_eq!(ttype_many_chars("<<"), TokenType::TT_LEFTSHIFT);
    assert_eq!(ttype_many_chars(">>"), TokenType::TT_RIGHTSHIFT);
    assert_eq!(ttype_many_chars("=="), TokenType::TT_EQUALS);
    assert_eq!(ttype_many_chars("^="), TokenType::TT_XOREQ);
    assert_eq!(ttype_many_chars("->"), TokenType::TT_POINT);
    assert_eq!(ttype_many_chars("<<="), TokenType::TT_LEFTSHIFTEQUALS);
    assert_eq!(ttype_many_chars(">>="), TokenType::TT_RIGHTSHIFTEQUALS);
    assert_eq!(ttype_many_chars("!="), TokenType::TT_NOTEQ);

    // Identifiers
    assert_eq!(ttype_many_chars("foo"), TokenType::TT_IDENTIFIER);
    assert_eq!(ttype_many_chars("Jaba"), TokenType::TT_IDENTIFIER);
    assert_eq!(ttype_many_chars("cat_"), TokenType::TT_IDENTIFIER);

    // Numeric literals
    assert_eq!(ttype_many_chars("123"), TokenType::TT_LITERAL);
    assert_eq!(ttype_many_chars("100u"), TokenType::TT_LITERAL);

    // Float literal (has period)
    assert_eq!(ttype_many_chars("1.2"), TokenType::TT_LITERAL);
    assert_eq!(ttype_many_chars("1.2f"), TokenType::TT_LITERAL);

    // Character literals (has ' or ")
    assert_eq!(ttype_many_chars("\"Planck\""), TokenType::TT_LITERAL);
    assert_eq!(ttype_many_chars("'Language'"), TokenType::TT_LITERAL);
}

#[test]
fn test_ttype_from_string_fn() {
    assert_eq!(ttype_from_string("+"), TokenType::TT_PLUS);
    assert_eq!(ttype_from_string("="), TokenType::TT_ASSIGN);
    assert_eq!(ttype_from_string("1"), TokenType::TT_LITERAL);
    assert_eq!(ttype_from_string("1.2"), TokenType::TT_LITERAL);
    assert_eq!(ttype_from_string("1u"), TokenType::TT_LITERAL);
    assert_eq!(ttype_from_string("1.2f"), TokenType::TT_LITERAL);
    assert_eq!(ttype_from_string("1.f"), TokenType::TT_LITERAL);
    assert_eq!(ttype_from_string("\"Planck\""), TokenType::TT_LITERAL);
    assert_eq!(ttype_from_string("'Language'"), TokenType::TT_LITERAL);
    assert_eq!(ttype_from_string("Jaba"), TokenType::TT_IDENTIFIER);
    assert_eq!(ttype_from_string("cat_"), TokenType::TT_IDENTIFIER);
    assert_eq!(ttype_from_string("("), TokenType::TT_OPAREN);
    assert_eq!(ttype_from_string("}"), TokenType::TT_CBRACE);
    assert_eq!(ttype_from_string(";"), TokenType::TT_SEMI);
    // Multi-char keyword
    assert_eq!(ttype_from_string("int"), TokenType::TT_INT);
    assert_eq!(ttype_from_string("while"), TokenType::TT_WHILE);
}

#[test]
fn test_ttype_name_fn() {
    assert_eq!(ttype_name(TokenType::TT_LITERAL), "literal");
    assert_eq!(ttype_name(TokenType::TT_IDENTIFIER), "identifier");
    assert_eq!(ttype_name(TokenType::TT_OPAREN), "open paren");
    assert_eq!(ttype_name(TokenType::TT_CPAREN), "close paren");
    assert_eq!(ttype_name(TokenType::TT_OBRACE), "open brace");
    assert_eq!(ttype_name(TokenType::TT_CBRACE), "close brace");
    assert_eq!(ttype_name(TokenType::TT_OBRACKET), "open bracket");
    assert_eq!(ttype_name(TokenType::TT_CBRACKET), "close bracket");
    assert_eq!(ttype_name(TokenType::TT_SEMI), "semicolon");
    assert_eq!(ttype_name(TokenType::TT_NO_TOKEN), "no token");
    assert_eq!(ttype_name(TokenType::TT_EOF), "end of file");
    assert_eq!(ttype_name(TokenType::TT_NEWLINE), "newline");
    assert_eq!(ttype_name(TokenType::TT_POUND), "pound");
    assert_eq!(ttype_name(TokenType::TT_PLUS), "+");
    assert_eq!(ttype_name(TokenType::TT_MINUS), "-");
    assert_eq!(ttype_name(TokenType::TT_STAR), "*");
    assert_eq!(ttype_name(TokenType::TT_SLASH), "/");
    assert_eq!(ttype_name(TokenType::TT_INT), "int");
    assert_eq!(ttype_name(TokenType::TT_WHILE), "while");
    assert_eq!(ttype_name(TokenType::TT_SIZEOF), "sizeof");
    assert_eq!(ttype_name(TokenType::TT_RETURN), "return");
    assert_eq!(ttype_name(TokenType::TT_AUTO), "auto");
    assert_eq!(ttype_name(TokenType::TT_BREAK), "break");
}

#[test]
fn test_test_ttype_helpers() {
    assert_eq!(test_ttype_one_char(), 0);
    assert_eq!(test_ttype_many_chars(), 0);
    assert_eq!(test_ttype_from_string(), 0);
    assert_eq!(test_ttype_name(), 0);
}

fn main() {}
