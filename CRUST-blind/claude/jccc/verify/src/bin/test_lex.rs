use jccc::lex::{
    in_string, is_valid_numeric_or_id_char, starts_operator, ttype_from_string, ttype_many_chars,
    ttype_name, ttype_one_char, valid_operator_sequence,
};
use jccc::token::TokenType;

#[test]
fn test_ttype_one_char_punct() {
    assert!(matches!(ttype_one_char('('), TokenType::TT_OPAREN));
    assert!(matches!(ttype_one_char(')'), TokenType::TT_CPAREN));
    assert!(matches!(ttype_one_char('{'), TokenType::TT_OBRACE));
    assert!(matches!(ttype_one_char('}'), TokenType::TT_CBRACE));
    assert!(matches!(ttype_one_char('['), TokenType::TT_OBRACKET));
    assert!(matches!(ttype_one_char(']'), TokenType::TT_CBRACKET));
    assert!(matches!(ttype_one_char(';'), TokenType::TT_SEMI));
    assert!(matches!(ttype_one_char('.'), TokenType::TT_PERIOD));
    assert!(matches!(ttype_one_char(','), TokenType::TT_COMMA));
    assert!(matches!(ttype_one_char('?'), TokenType::TT_QMARK));
    assert!(matches!(ttype_one_char('#'), TokenType::TT_POUND));
}

#[test]
fn test_ttype_one_char_operators() {
    assert!(matches!(ttype_one_char('-'), TokenType::TT_MINUS));
    assert!(matches!(ttype_one_char('+'), TokenType::TT_PLUS));
    assert!(matches!(ttype_one_char('*'), TokenType::TT_STAR));
    assert!(matches!(ttype_one_char('/'), TokenType::TT_SLASH));
    assert!(matches!(ttype_one_char('='), TokenType::TT_ASSIGN));
    assert!(matches!(ttype_one_char(':'), TokenType::TT_COLON));
    assert!(matches!(ttype_one_char('%'), TokenType::TT_MOD));
    assert!(matches!(ttype_one_char('&'), TokenType::TT_BAND));
    assert!(matches!(ttype_one_char('|'), TokenType::TT_BOR));
    assert!(matches!(ttype_one_char('>'), TokenType::TT_GREATER));
    assert!(matches!(ttype_one_char('<'), TokenType::TT_LESS));
    assert!(matches!(ttype_one_char('!'), TokenType::TT_LNOT));
    assert!(matches!(ttype_one_char('~'), TokenType::TT_BNOT));
    assert!(matches!(ttype_one_char('^'), TokenType::TT_XOR));
}

#[test]
fn test_ttype_one_char_alphanumeric() {
    assert!(matches!(ttype_one_char('a'), TokenType::TT_IDENTIFIER));
    assert!(matches!(ttype_one_char('z'), TokenType::TT_IDENTIFIER));
    assert!(matches!(ttype_one_char('A'), TokenType::TT_IDENTIFIER));
    assert!(matches!(ttype_one_char('Z'), TokenType::TT_IDENTIFIER));
    assert!(matches!(ttype_one_char('_'), TokenType::TT_IDENTIFIER));
    assert!(matches!(ttype_one_char(' '), TokenType::TT_IDENTIFIER));

    assert!(matches!(ttype_one_char('0'), TokenType::TT_LITERAL));
    assert!(matches!(ttype_one_char('1'), TokenType::TT_LITERAL));
    assert!(matches!(ttype_one_char('9'), TokenType::TT_LITERAL));
}

#[test]
fn test_ttype_many_chars_keywords() {
    assert!(matches!(ttype_many_chars("auto"), TokenType::TT_AUTO));
    assert!(matches!(ttype_many_chars("break"), TokenType::TT_BREAK));
    assert!(matches!(ttype_many_chars("char"), TokenType::TT_CHAR));
    assert!(matches!(ttype_many_chars("const"), TokenType::TT_CONST));
    assert!(matches!(ttype_many_chars("case"), TokenType::TT_CASE));
    assert!(matches!(
        ttype_many_chars("continue"),
        TokenType::TT_CONTINUE
    ));
    assert!(matches!(ttype_many_chars("double"), TokenType::TT_DOUBLE));
    assert!(matches!(ttype_many_chars("do"), TokenType::TT_DO));
    assert!(matches!(
        ttype_many_chars("default"),
        TokenType::TT_DEFAULT
    ));
    assert!(matches!(ttype_many_chars("enum"), TokenType::TT_ENUM));
    assert!(matches!(ttype_many_chars("else"), TokenType::TT_ELSE));
    assert!(matches!(ttype_many_chars("extern"), TokenType::TT_EXTERN));
    assert!(matches!(ttype_many_chars("float"), TokenType::TT_FLOAT));
    assert!(matches!(ttype_many_chars("for"), TokenType::TT_FOR));
    assert!(matches!(ttype_many_chars("goto"), TokenType::TT_GOTO));
    assert!(matches!(ttype_many_chars("if"), TokenType::TT_IF));
    assert!(matches!(ttype_many_chars("int"), TokenType::TT_INT));
    assert!(matches!(ttype_many_chars("long"), TokenType::TT_LONG));
    assert!(matches!(ttype_many_chars("return"), TokenType::TT_RETURN));
    assert!(matches!(
        ttype_many_chars("register"),
        TokenType::TT_REGISTER
    ));
    assert!(matches!(ttype_many_chars("static"), TokenType::TT_STATIC));
    assert!(matches!(ttype_many_chars("switch"), TokenType::TT_SWITCH));
    assert!(matches!(ttype_many_chars("short"), TokenType::TT_SHORT));
    assert!(matches!(ttype_many_chars("signed"), TokenType::TT_SIGNED));
    assert!(matches!(ttype_many_chars("struct"), TokenType::TT_STRUCT));
    assert!(matches!(ttype_many_chars("sizeof"), TokenType::TT_SIZEOF));
    assert!(matches!(
        ttype_many_chars("typedef"),
        TokenType::TT_TYPEDEF
    ));
    assert!(matches!(ttype_many_chars("union"), TokenType::TT_UNION));
    assert!(matches!(
        ttype_many_chars("unsigned"),
        TokenType::TT_UNSIGNED
    ));
    assert!(matches!(ttype_many_chars("void"), TokenType::TT_VOID));
    assert!(matches!(
        ttype_many_chars("volatile"),
        TokenType::TT_VOLATILE
    ));
    assert!(matches!(ttype_many_chars("while"), TokenType::TT_WHILE));
}

#[test]
fn test_ttype_many_chars_operators() {
    assert!(matches!(ttype_many_chars("&&"), TokenType::TT_LAND));
    assert!(matches!(ttype_many_chars("||"), TokenType::TT_LOR));
    assert!(matches!(ttype_many_chars("-="), TokenType::TT_DEC));
    assert!(matches!(ttype_many_chars("+="), TokenType::TT_INC));
    assert!(matches!(ttype_many_chars("++"), TokenType::TT_PLUSPLUS));
    assert!(matches!(ttype_many_chars("--"), TokenType::TT_MINUSMINUS));
    assert!(matches!(ttype_many_chars("/="), TokenType::TT_DIVEQ));
    assert!(matches!(ttype_many_chars("*="), TokenType::TT_MULEQ));
    assert!(matches!(ttype_many_chars("%="), TokenType::TT_MODEQ));
    assert!(matches!(ttype_many_chars("&="), TokenType::TT_BANDEQ));
    assert!(matches!(ttype_many_chars("|="), TokenType::TT_BOREQ));
    assert!(matches!(ttype_many_chars("&&="), TokenType::TT_LANDEQ));
    assert!(matches!(ttype_many_chars("||="), TokenType::TT_LOREQ));
    assert!(matches!(ttype_many_chars("<="), TokenType::TT_LESSEQ));
    assert!(matches!(ttype_many_chars(">="), TokenType::TT_GREATEREQ));
    assert!(matches!(ttype_many_chars("<<"), TokenType::TT_LEFTSHIFT));
    assert!(matches!(ttype_many_chars(">>"), TokenType::TT_RIGHTSHIFT));
    assert!(matches!(ttype_many_chars("=="), TokenType::TT_EQUALS));
    assert!(matches!(ttype_many_chars("^="), TokenType::TT_XOREQ));
    assert!(matches!(ttype_many_chars("->"), TokenType::TT_POINT));
    assert!(matches!(
        ttype_many_chars("<<="),
        TokenType::TT_LEFTSHIFTEQUALS
    ));
    assert!(matches!(
        ttype_many_chars(">>="),
        TokenType::TT_RIGHTSHIFTEQUALS
    ));
    assert!(matches!(ttype_many_chars("!="), TokenType::TT_NOTEQ));
}

#[test]
fn test_ttype_many_chars_literals_and_identifiers() {
    // Numeric literals
    assert!(matches!(ttype_many_chars("123"), TokenType::TT_LITERAL));
    assert!(matches!(ttype_many_chars("123u"), TokenType::TT_LITERAL));
    assert!(matches!(ttype_many_chars("12.5"), TokenType::TT_LITERAL));
    assert!(matches!(ttype_many_chars("1.f"), TokenType::TT_LITERAL));
    assert!(matches!(ttype_many_chars("1.2f"), TokenType::TT_LITERAL));

    // Quoted: contains a quote => literal
    assert!(matches!(
        ttype_many_chars("\"Planck\""),
        TokenType::TT_LITERAL
    ));
    assert!(matches!(
        ttype_many_chars("'Language'"),
        TokenType::TT_LITERAL
    ));

    // Identifiers
    assert!(matches!(ttype_many_chars("foo"), TokenType::TT_IDENTIFIER));
    assert!(matches!(
        ttype_many_chars("Jaba"),
        TokenType::TT_IDENTIFIER
    ));
    assert!(matches!(
        ttype_many_chars("cat_"),
        TokenType::TT_IDENTIFIER
    ));
    assert!(matches!(
        ttype_many_chars("foo123"),
        TokenType::TT_IDENTIFIER
    ));
    // Two 'u's => not all-numeric literal => identifier
    assert!(matches!(
        ttype_many_chars("123uu"),
        TokenType::TT_IDENTIFIER
    ));
}

#[test]
fn test_ttype_from_string_single_char() {
    assert!(matches!(ttype_from_string("+"), TokenType::TT_PLUS));
    assert!(matches!(ttype_from_string("="), TokenType::TT_ASSIGN));
    assert!(matches!(ttype_from_string("("), TokenType::TT_OPAREN));
    assert!(matches!(ttype_from_string("}"), TokenType::TT_CBRACE));
    assert!(matches!(ttype_from_string(";"), TokenType::TT_SEMI));
    assert!(matches!(ttype_from_string("1"), TokenType::TT_LITERAL));
    assert!(matches!(ttype_from_string("A"), TokenType::TT_IDENTIFIER));
    assert!(matches!(ttype_from_string("_"), TokenType::TT_IDENTIFIER));
}

#[test]
fn test_ttype_from_string_multi_char() {
    assert!(matches!(ttype_from_string("1.2"), TokenType::TT_LITERAL));
    assert!(matches!(ttype_from_string("1u"), TokenType::TT_LITERAL));
    assert!(matches!(ttype_from_string("1.2f"), TokenType::TT_LITERAL));
    assert!(matches!(ttype_from_string("1.f"), TokenType::TT_LITERAL));
    assert!(matches!(
        ttype_from_string("\"Planck\""),
        TokenType::TT_LITERAL
    ));
    assert!(matches!(
        ttype_from_string("'Language'"),
        TokenType::TT_LITERAL
    ));
    assert!(matches!(
        ttype_from_string("Jaba"),
        TokenType::TT_IDENTIFIER
    ));
    assert!(matches!(
        ttype_from_string("cat_"),
        TokenType::TT_IDENTIFIER
    ));
    assert!(matches!(ttype_from_string("->"), TokenType::TT_POINT));
    assert!(matches!(ttype_from_string("=="), TokenType::TT_EQUALS));
    assert!(matches!(ttype_from_string("<<="), TokenType::TT_LEFTSHIFTEQUALS));
}

#[test]
fn test_ttype_name_all() {
    // Punctuation/single chars
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
    assert_eq!(ttype_name(TokenType::TT_PERIOD), ".");
    assert_eq!(ttype_name(TokenType::TT_COMMA), ",");
    assert_eq!(ttype_name(TokenType::TT_QMARK), "?");

    // Operators
    assert_eq!(ttype_name(TokenType::TT_MINUS), "-");
    assert_eq!(ttype_name(TokenType::TT_PLUS), "+");
    assert_eq!(ttype_name(TokenType::TT_STAR), "*");
    assert_eq!(ttype_name(TokenType::TT_SLASH), "/");
    assert_eq!(ttype_name(TokenType::TT_ASSIGN), "=");
    assert_eq!(ttype_name(TokenType::TT_COLON), ":");
    assert_eq!(ttype_name(TokenType::TT_MOD), "%");
    assert_eq!(ttype_name(TokenType::TT_BAND), "&");
    assert_eq!(ttype_name(TokenType::TT_LAND), "&&");
    assert_eq!(ttype_name(TokenType::TT_BOR), "|");
    assert_eq!(ttype_name(TokenType::TT_LOR), "||");
    assert_eq!(ttype_name(TokenType::TT_DEC), "-=");
    assert_eq!(ttype_name(TokenType::TT_INC), "+=");
    assert_eq!(ttype_name(TokenType::TT_PLUSPLUS), "++");
    assert_eq!(ttype_name(TokenType::TT_MINUSMINUS), "--");
    assert_eq!(ttype_name(TokenType::TT_DIVEQ), "/=");
    assert_eq!(ttype_name(TokenType::TT_MULEQ), "*=");
    assert_eq!(ttype_name(TokenType::TT_MODEQ), "%=");
    assert_eq!(ttype_name(TokenType::TT_BANDEQ), "&=");
    assert_eq!(ttype_name(TokenType::TT_BOREQ), "|=");
    assert_eq!(ttype_name(TokenType::TT_LANDEQ), "&&=");
    assert_eq!(ttype_name(TokenType::TT_LOREQ), "||=");
    assert_eq!(ttype_name(TokenType::TT_GREATER), ">");
    assert_eq!(ttype_name(TokenType::TT_LESS), "<");
    assert_eq!(ttype_name(TokenType::TT_LESSEQ), "<=");
    assert_eq!(ttype_name(TokenType::TT_GREATEREQ), ">=");
    assert_eq!(ttype_name(TokenType::TT_LEFTSHIFT), "<<");
    assert_eq!(ttype_name(TokenType::TT_RIGHTSHIFT), ">>");
    assert_eq!(ttype_name(TokenType::TT_LNOT), "!");
    assert_eq!(ttype_name(TokenType::TT_BNOT), "~");
    assert_eq!(ttype_name(TokenType::TT_EQUALS), "==");
    assert_eq!(ttype_name(TokenType::TT_NOTEQ), "!=");
    assert_eq!(ttype_name(TokenType::TT_XOR), "^");
    assert_eq!(ttype_name(TokenType::TT_XOREQ), "^=");
    assert_eq!(ttype_name(TokenType::TT_POINT), "->");
    assert_eq!(ttype_name(TokenType::TT_LEFTSHIFTEQUALS), "<<=");
    assert_eq!(ttype_name(TokenType::TT_RIGHTSHIFTEQUALS), ">>=");

    // Keywords
    assert_eq!(ttype_name(TokenType::TT_AUTO), "auto");
    assert_eq!(ttype_name(TokenType::TT_BREAK), "break");
    assert_eq!(ttype_name(TokenType::TT_CHAR), "char");
    assert_eq!(ttype_name(TokenType::TT_CONST), "const");
    assert_eq!(ttype_name(TokenType::TT_CASE), "case");
    assert_eq!(ttype_name(TokenType::TT_CONTINUE), "continue");
    assert_eq!(ttype_name(TokenType::TT_DOUBLE), "double");
    assert_eq!(ttype_name(TokenType::TT_DO), "do");
    assert_eq!(ttype_name(TokenType::TT_DEFAULT), "default");
    assert_eq!(ttype_name(TokenType::TT_ENUM), "enum");
    assert_eq!(ttype_name(TokenType::TT_ELSE), "else");
    assert_eq!(ttype_name(TokenType::TT_EXTERN), "extern");
    assert_eq!(ttype_name(TokenType::TT_FLOAT), "float");
    assert_eq!(ttype_name(TokenType::TT_FOR), "for");
    assert_eq!(ttype_name(TokenType::TT_GOTO), "goto");
    assert_eq!(ttype_name(TokenType::TT_IF), "if");
    assert_eq!(ttype_name(TokenType::TT_INT), "int");
    assert_eq!(ttype_name(TokenType::TT_LONG), "long");
    assert_eq!(ttype_name(TokenType::TT_RETURN), "return");
    assert_eq!(ttype_name(TokenType::TT_REGISTER), "register");
    assert_eq!(ttype_name(TokenType::TT_STATIC), "static");
    assert_eq!(ttype_name(TokenType::TT_SWITCH), "switch");
    assert_eq!(ttype_name(TokenType::TT_SHORT), "short");
    assert_eq!(ttype_name(TokenType::TT_SIGNED), "signed");
    assert_eq!(ttype_name(TokenType::TT_STRUCT), "struct");
    assert_eq!(ttype_name(TokenType::TT_SIZEOF), "sizeof");
    assert_eq!(ttype_name(TokenType::TT_TYPEDEF), "typedef");
    assert_eq!(ttype_name(TokenType::TT_UNSIGNED), "unsigned");
    assert_eq!(ttype_name(TokenType::TT_UNION), "union");
    assert_eq!(ttype_name(TokenType::TT_VOID), "void");
    assert_eq!(ttype_name(TokenType::TT_VOLATILE), "volatile");
    assert_eq!(ttype_name(TokenType::TT_WHILE), "while");
}

#[test]
fn test_starts_operator() {
    let yes = ['-', '+', '*', '/', '=', ':', '%', '&', '|', '<', '>', '!', '~', '^'];
    for c in yes.iter() {
        assert_eq!(starts_operator(*c), 1, "expected {} to start operator", c);
    }
    let no = ['a', 'A', '0', '_', ' ', '\t', '(', ';', '#', '?', '.', ','];
    for c in no.iter() {
        assert_eq!(starts_operator(*c), 0, "expected {} to NOT start operator", c);
    }
}

#[test]
fn test_valid_operator_sequence_basic() {
    let valid = [
        "-", "+", "*", "/", "=", ":", "%", "&", "&&", "|", "||", "-=", "+=", "++", "--", "/=",
        "*=", "%=", "&=", "|=", "&&=", "||=", ">", "<", "<=", ">=", "<<", ">>", "!", "==", "!=",
        "^", "^=", "->", "<<=", ">>=",
    ];
    for op in valid.iter() {
        assert_eq!(valid_operator_sequence(op), 1, "{} should be valid", op);
    }

    let invalid = ["abc", "1", "@", "+++", "===", "->>"];
    for op in invalid.iter() {
        assert_eq!(valid_operator_sequence(op), 0, "{} should NOT be valid", op);
    }
}

#[test]
fn test_in_string() {
    assert_eq!(in_string('a', "abc"), 1);
    assert_eq!(in_string('c', "abc"), 1);
    assert_eq!(in_string('d', "abc"), 0);
    assert_eq!(in_string('(', "(){}[];~#,.:?~"), 1);
    assert_eq!(in_string('z', "(){}[];~#,.:?~"), 0);
}

#[test]
fn test_is_valid_numeric_or_id_char() {
    assert_eq!(is_valid_numeric_or_id_char('a'), 1);
    assert_eq!(is_valid_numeric_or_id_char('Z'), 1);
    assert_eq!(is_valid_numeric_or_id_char('0'), 1);
    assert_eq!(is_valid_numeric_or_id_char('9'), 1);
    assert_eq!(is_valid_numeric_or_id_char('_'), 1);
    assert_eq!(is_valid_numeric_or_id_char('.'), 1);

    assert_eq!(is_valid_numeric_or_id_char(' '), 0);
    assert_eq!(is_valid_numeric_or_id_char('+'), 0);
    assert_eq!(is_valid_numeric_or_id_char('('), 0);
    assert_eq!(is_valid_numeric_or_id_char(';'), 0);
}

#[test]
fn test_internal_test_helpers() {
    // These are translations of the C test functions; they should return 0
    // (success).
    assert_eq!(jccc::lex::test_ttype_name(), 0);
    assert_eq!(jccc::lex::test_ttype_from_string(), 0);
    assert_eq!(jccc::lex::test_ttype_one_char(), 0);
    assert_eq!(jccc::lex::test_ttype_many_chars(), 0);
}

fn main() {}
