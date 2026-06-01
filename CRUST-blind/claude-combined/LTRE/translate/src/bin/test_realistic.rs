use LTRE::ltre::*;

fn matches(regex: &str, input: &str) -> bool {
    let nfa = ltre_parse(regex).expect(&format!("parse failed: {}", regex));
    let dfa = ltre_compile(nfa);
    ltre_matches(&dfa, input.as_bytes())
}

fn matches_bytes(regex: &str, input: &[u8]) -> bool {
    let nfa = ltre_parse(regex).expect(&format!("parse failed: {}", regex));
    let dfa = ltre_compile(nfa);
    ltre_matches(&dfa, input)
}

const FIELD_WIDTH: &str = "(\\*|1-90-9*)?";
const PRECISION: &str = "(\\.|\\.\\*|\\.1-90-9*)?";

fn conv_spec() -> String {
    let di = format!("[\\-\\+ 0]*{}{}([hljzt]|hh|ll)?[di]", FIELD_WIDTH, PRECISION);
    let u = format!("[\\-0]*{}{}([hljzt]|hh|ll)?u", FIELD_WIDTH, PRECISION);
    let ox = format!("[\\-#0]*{}{}([hljzt]|hh|ll)?[oxX]", FIELD_WIDTH, PRECISION);
    let fega = format!("[\\-\\+ #0]*{}{}[lL]?[fFeEgGaA]", FIELD_WIDTH, PRECISION);
    let c = format!("\\-*{}l?c", FIELD_WIDTH);
    let s = format!("\\-*{}{}l?s", FIELD_WIDTH, PRECISION);
    let p = format!("\\-*{}p", FIELD_WIDTH);
    let n = format!("{}([hljzt]|hh|ll)?n", FIELD_WIDTH);
    format!("%({}|{}|{}|{}|{}|{}|{}|{}|%)", di, u, ox, fega, c, s, p, n)
}

#[test]
fn test_conv_spec() {
    let cs = conv_spec();
    assert_eq!(matches(&cs, "%"), false);
    assert_eq!(matches(&cs, "%*"), false);
    assert_eq!(matches(&cs, "%%"), true);
    assert_eq!(matches(&cs, "%5%"), false);
    assert_eq!(matches(&cs, "%p"), true);
    assert_eq!(matches(&cs, "%*p"), true);
    assert_eq!(matches(&cs, "% *p"), false);
    assert_eq!(matches(&cs, "%5p"), true);
    assert_eq!(matches(&cs, "d"), false);
    assert_eq!(matches(&cs, "%d"), true);
    assert_eq!(matches(&cs, "%.16s"), true);
    assert_eq!(matches(&cs, "% 5.3f"), true);
    assert_eq!(matches(&cs, "%*32.4g"), false);
    assert_eq!(matches(&cs, "%-#65.4g"), true);
    assert_eq!(matches(&cs, "%03c"), false);
    assert_eq!(matches(&cs, "%06i"), true);
    assert_eq!(matches(&cs, "%lu"), true);
    assert_eq!(matches(&cs, "%hhu"), true);
    assert_eq!(matches(&cs, "%Lu"), false);
    assert_eq!(matches(&cs, "%-*p"), true);
    assert_eq!(matches(&cs, "%-.*p"), false);
    assert_eq!(matches(&cs, "%id"), false);
    assert_eq!(matches(&cs, "%%d"), false);
    assert_eq!(matches(&cs, "i%d"), false);
    assert_eq!(matches(&cs, "%c%s"), false);
    assert_eq!(matches(&cs, "%0n"), false);
    assert_eq!(matches(&cs, "% u"), false);
    assert_eq!(matches(&cs, "%+c"), false);
    assert_eq!(matches(&cs, "%0-++ 0i"), true);
    assert_eq!(matches(&cs, "%30c"), true);
    assert_eq!(matches(&cs, "%03c"), false);
}

#[test]
fn test_json_num() {
    let r = "\\-?(0|1-90-9*)(\\.0-9+)?([eE][\\+\\-]?0-9+)?";
    assert_eq!(matches(r, "e"), false);
    assert_eq!(matches(r, "1"), true);
    assert_eq!(matches(r, "10"), true);
    assert_eq!(matches(r, "01"), false);
    assert_eq!(matches(r, "-5"), true);
    assert_eq!(matches(r, "+5"), false);
    assert_eq!(matches(r, ".3"), false);
    assert_eq!(matches(r, "2."), false);
    assert_eq!(matches(r, "2.3"), true);
    assert_eq!(matches(r, "1e"), false);
    assert_eq!(matches(r, "1e0"), true);
    assert_eq!(matches(r, "1E+0"), true);
    assert_eq!(matches(r, "1e-0"), true);
    assert_eq!(matches(r, "1E10"), true);
    assert_eq!(matches(r, "1e+00"), true);
}

#[test]
fn test_json_str() {
    let r = "\"(^[\\x00-\\x1f\"\\\\]|\\\\[\"\\\\/bfnrt]|\\\\u[0-9a-fA-F]{4})*\"";
    assert_eq!(matches(r, "foo"), false);
    assert_eq!(matches(r, "\"foo"), false);
    assert_eq!(matches(r, "\"\""), true);
    assert_eq!(matches(r, "\"foo\""), true);
    assert_eq!(matches(r, "\"foo\\\"\""), true);
    assert_eq!(matches(r, "\"foo\\\\\""), true);
    assert_eq!(matches(r, "\"\\nbar\""), true);
    assert_eq!(matches(r, "\"\nbar\""), false);
    assert_eq!(matches(r, "\"\\abar\""), false);
    assert_eq!(matches(r, "\"foo\\v\""), false);
    assert_eq!(matches(r, "\"\\u1A2b\""), true);
    assert_eq!(matches(r, "\"\\uDEAD\""), true);
    assert_eq!(matches(r, "\"\\uF00\""), false);
    assert_eq!(matches(r, "\"\\uF00BAR\""), true);
    assert_eq!(matches(r, "\"foo\\/\""), true);
    assert_eq!(matches_bytes(r, b"\"\xcf\x84\""), true);
    assert_eq!(matches_bytes(r, b"\"\x80\""), true);
    assert_eq!(matches_bytes(r, b"\"\x88x/\""), true);
}

#[test]
fn test_identifier() {
    // C-like identifier with universal character names
    let kw = "(auto|break|case|char|const|continue|default|do|double|else|enum|extern|\
              float|for|goto|if|inline|int|long|register|restrict|return|short|signed|\
              sizeof|static|struct|switch|typedef|union|unsigned|void|volatile|while|\
              _Bool|_Complex|_Imaginary)";
    let hex_quad = "[0-9a-fA-F]{4}";
    let identifier = format!(
        "(\\w|\\\\u{}|\\\\U{}{})*&~\\d.*&~{}",
        hex_quad, hex_quad, hex_quad, kw
    );
    assert_eq!(matches(&identifier, "_"), true);
    assert_eq!(matches(&identifier, "_foo"), true);
    assert_eq!(matches(&identifier, "_Bool"), false);
    assert_eq!(matches(&identifier, "a1"), true);
    assert_eq!(matches(&identifier, "5b"), false);
    assert_eq!(matches(&identifier, "if"), false);
    assert_eq!(matches(&identifier, "ifa"), true);
    assert_eq!(matches(&identifier, "bif"), true);
    assert_eq!(matches(&identifier, "if2"), true);
    assert_eq!(matches(&identifier, "1if"), false);
    assert_eq!(matches(&identifier, "\\u12"), false);
    assert_eq!(matches(&identifier, "\\u1A2b"), true);
    assert_eq!(matches(&identifier, "\\u1234"), true);
    assert_eq!(matches(&identifier, "\\u123x"), false);
    assert_eq!(matches(&identifier, "\\u1234x"), true);
    assert_eq!(matches(&identifier, "\\U12345678"), true);
    assert_eq!(matches(&identifier, "\\U1234567y"), false);
    assert_eq!(matches(&identifier, "\\U12345678y"), true);
}

#[test]
fn test_utf8_some() {
    let tail = "\\x80-\\xbf";
    let byte_pat = format!(
        "(\\x00-\\x7f|\\xc0-\\xdf{}|\\xe0-\\xef{}{}|\\xf0-\\xf7{}{}{})",
        tail, tail, tail, tail, tail, tail
    );
    let overlong = "(\\xc0-\\xc1<>|\\xe0\\x80-\\x9f<>|\\xf0\\x80-\\x8f<><>)";
    let surrogate = "\\xed\\xa0-\\xbf<>";
    let too_big = format!("(\\xf4\\x90-\\xff{}{}|\\xf5-\\xff{}{}{})", tail, tail, tail, tail, tail);
    let utf8_char_1 = format!("({}&~{}&~{}&~{})", byte_pat, overlong, surrogate, too_big);
    assert_eq!(matches_bytes(&utf8_char_1, b"ab"), false);
    assert_eq!(matches_bytes(&utf8_char_1, b"\x80x"), false);
    assert_eq!(matches_bytes(&utf8_char_1, b"\x80"), false);
    assert_eq!(matches_bytes(&utf8_char_1, b"\xbf"), false);
    assert_eq!(matches_bytes(&utf8_char_1, b"\xc0"), false);
    assert_eq!(matches_bytes(&utf8_char_1, b"\xc1"), false);
    assert_eq!(matches_bytes(&utf8_char_1, b"\xff"), false);
    assert_eq!(matches_bytes(&utf8_char_1, b"\xed\xa1\x8c"), false);
    assert_eq!(matches_bytes(&utf8_char_1, b"\xed\xbe\xb4"), false);
    assert_eq!(matches_bytes(&utf8_char_1, b"\xed\xa0\x80"), false);
    assert_eq!(matches_bytes(&utf8_char_1, b"\xc0\x80"), false);
    assert_eq!(matches_bytes(&utf8_char_1, b"\x7f"), true);
    assert_eq!(matches_bytes(&utf8_char_1, b"\xF0\x9E\x84\x93"), true);
    assert_eq!(matches_bytes(&utf8_char_1, b"\x2f"), true);
    assert_eq!(matches_bytes(&utf8_char_1, b"\xc0\xaf"), false);
    assert_eq!(matches_bytes(&utf8_char_1, b"\xe0\x80\xaf"), false);
    assert_eq!(matches_bytes(&utf8_char_1, b"\xf0\x80\x80\xaf"), false);
    assert_eq!(matches_bytes(&utf8_char_1, b"\xf7\xbf\xbf\xbf"), false);
}

fn main() {}
