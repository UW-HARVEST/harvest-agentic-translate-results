use skp::skp::*;

fn consumed(src: &str, end: &str) -> usize {
    src.len() - end.len()
}

// ============ Hex number (X) ============

#[test]
fn test_skp_hex_number_with_prefix() {
    let src = "0xFF";
    let (ret, _to, end) = skp_(src, "X");
    assert_eq!(ret, 1);
    assert_eq!(consumed(src, end), 4);
}

#[test]
fn test_skp_hex_number_no_prefix() {
    let src = "FF";
    let (ret, _to, end) = skp_(src, "X");
    assert_eq!(ret, 1);
    assert_eq!(consumed(src, end), 2);
}

#[test]
fn test_skp_xdigit_star() {
    let src = "FF";
    let (ret, _to, end) = skp_(src, "*x");
    assert_eq!(ret, 1);
    assert_eq!(consumed(src, end), 2);
}

// ============ Whitespace ============

#[test]
fn test_skp_blank_star() {
    let src = "  abc";
    let (ret, _to, end) = skp_(src, "*w");
    assert_eq!(ret, 1);
    assert_eq!(consumed(src, end), 2);
}

#[test]
fn test_skp_space_star() {
    let src = " \t\nabc";
    let (ret, _to, end) = skp_(src, "*s");
    assert_eq!(ret, 1);
    assert_eq!(consumed(src, end), 3);
}

// ============ Identifier (I) ============

#[test]
fn test_skp_identifier() {
    let src = "_foo123 bar";
    let (ret, _to, end) = skp_(src, "I");
    assert_eq!(ret, 1);
    assert_eq!(consumed(src, end), 7);
}

#[test]
fn test_skp_identifier_fail() {
    let src = "123foo";
    let (ret, _, _) = skp_(src, "I");
    assert_eq!(ret, 0);
}

// ============ Balanced (B) ============

#[test]
fn test_skp_balanced_parens() {
    let src = "(abc)rest";
    let (ret, _to, end) = skp_(src, "B");
    assert_eq!(ret, 1);
    assert_eq!(consumed(src, end), 5);
}

#[test]
fn test_skp_balanced_brackets() {
    let src = "[a[b]c]rest";
    let (ret, _to, end) = skp_(src, "B");
    assert_eq!(ret, 1);
    assert_eq!(consumed(src, end), 7);
}

#[test]
fn test_skp_balanced_unmatched() {
    let src = "{abc";
    let (ret, _, _) = skp_(src, "B");
    assert_eq!(ret, 0);
}

// ============ Quoted (Q) ============

#[test]
fn test_skp_quoted_double() {
    let src = "\"hello\\\"world\"rest";
    let (ret, _to, end) = skp_(src, "Q");
    assert_eq!(ret, 1);
    assert_eq!(consumed(src, end), 14);
}

#[test]
fn test_skp_quoted_single() {
    let src = "'abc'rest";
    let (ret, _to, end) = skp_(src, "Q");
    assert_eq!(ret, 1);
    assert_eq!(consumed(src, end), 5);
}

#[test]
fn test_skp_quoted_backtick() {
    let src = "`hello`rest";
    let (ret, _to, end) = skp_(src, "Q");
    assert_eq!(ret, 1);
    assert_eq!(consumed(src, end), 7);
}

#[test]
fn test_skp_quoted_unmatched() {
    let src = "'hello";
    let (ret, _, _) = skp_(src, "Q");
    assert_eq!(ret, 0);
}

// ============ Past end of line (N) ============

#[test]
fn test_skp_past_eol() {
    let src = "abc\ndef";
    let (ret, _to, end) = skp_(src, "N");
    assert_eq!(ret, 1);
    assert_eq!(consumed(src, end), 4);
}

// ============ Integer (D) ============

#[test]
fn test_skp_integer_negative() {
    let src = "-42rest";
    let (ret, _to, end) = skp_(src, "D");
    assert_eq!(ret, 1);
    assert_eq!(consumed(src, end), 3);
}

#[test]
fn test_skp_integer_positive() {
    let src = "+7rest";
    let (ret, _to, end) = skp_(src, "D");
    assert_eq!(ret, 1);
    assert_eq!(consumed(src, end), 2);
}

// ============ Float (F) ============

#[test]
fn test_skp_float_basic() {
    let src = "3.14rest";
    let (ret, _to, end) = skp_(src, "F");
    assert_eq!(ret, 1);
    assert_eq!(consumed(src, end), 4);
}

#[test]
fn test_skp_float_exponent() {
    let src = "1.5e10rest";
    let (ret, _to, end) = skp_(src, "F");
    assert_eq!(ret, 1);
    assert_eq!(consumed(src, end), 6);
}

#[test]
fn test_skp_float_dot_prefix() {
    let src = ".5rest";
    let (ret, _to, end) = skp_(src, "F");
    assert_eq!(ret, 1);
    assert_eq!(consumed(src, end), 2);
}

#[test]
fn test_skp_float_exp_sign() {
    let src = "1e-5rest";
    let (ret, _to, end) = skp_(src, "F");
    assert_eq!(ret, 1);
    assert_eq!(consumed(src, end), 4);
}

// ============ Negation ============

#[test]
fn test_skp_not_digit_on_alpha() {
    let src = "abc";
    let (ret, _to, end) = skp_(src, "!d");
    assert_eq!(ret, 1);
    assert_eq!(consumed(src, end), 1);
}

#[test]
fn test_skp_not_digit_on_digit() {
    let src = "123";
    let (ret, _, _) = skp_(src, "!d");
    assert_eq!(ret, 0);
}

// ============ Any (.) ============

#[test]
fn test_skp_any_single() {
    let src = "abc";
    let (ret, _to, end) = skp_(src, ".");
    assert_eq!(ret, 1);
    assert_eq!(consumed(src, end), 1);
}

#[test]
fn test_skp_any_star() {
    let src = "abc";
    let (ret, _to, end) = skp_(src, "*.");
    assert_eq!(ret, 1);
    assert_eq!(consumed(src, end), 3);
}

// ============ End of text (!.) ============

#[test]
fn test_skp_eot_on_empty() {
    let src = "";
    let (ret, _, _) = skp_(src, "!.");
    // C returns 1 with len=0 for empty string with !.
    assert_eq!(ret, 1);
}

// ============ End of line ($) ============

#[test]
fn test_skp_eol_empty() {
    let src = "";
    let (ret, _, _) = skp_(src, "$");
    assert_eq!(ret, 1);
}

#[test]
fn test_skp_eol_newline() {
    let src = "\n";
    let (ret, _to, end) = skp_(src, "$");
    assert_eq!(ret, 1);
    assert_eq!(consumed(src, end), 1);
}

// ============ Sets ============

#[test]
fn test_skp_set_basic() {
    let src = "abc";
    let (ret, _to, end) = skp_(src, "[abc]");
    assert_eq!(ret, 1);
    assert_eq!(consumed(src, end), 1);
}

#[test]
fn test_skp_set_star() {
    let src = "abc";
    let (ret, _to, end) = skp_(src, "*[abc]");
    assert_eq!(ret, 1);
    assert_eq!(consumed(src, end), 3);
}

#[test]
fn test_skp_set_no_match() {
    let src = "xyz";
    let (ret, _, _) = skp_(src, "[abc]");
    assert_eq!(ret, 0);
}

#[test]
fn test_skp_set_range() {
    let src = "c";
    let (ret, _to, end) = skp_(src, "[a-z]");
    assert_eq!(ret, 1);
    assert_eq!(consumed(src, end), 1);
}

// ============ Alnum (@) ============

#[test]
fn test_skp_alnum_star() {
    let src = "a1b2";
    let (ret, _to, end) = skp_(src, "*@");
    assert_eq!(ret, 1);
    assert_eq!(consumed(src, end), 4);
}

// ============ Control (c) ============

#[test]
fn test_skp_ctrl_star() {
    let src = "\x01\x02abc";
    let (ret, _to, end) = skp_(src, "*c");
    assert_eq!(ret, 1);
    assert_eq!(consumed(src, end), 2);
}

// ============ Idchr (i) ============

#[test]
fn test_skp_idchr_star() {
    let src = "_a1";
    let (ret, _to, end) = skp_(src, "*i");
    assert_eq!(ret, 1);
    assert_eq!(consumed(src, end), 3);
}

// ============ () balanced parens only ============

#[test]
fn test_skp_parens_only() {
    let src = "(abc)rest";
    let (ret, _to, end) = skp_(src, "()");
    assert_eq!(ret, 1);
    assert_eq!(consumed(src, end), 5);
}

#[test]
fn test_skp_parens_only_brackets_fail() {
    let src = "[abc]rest";
    let (ret, _, _) = skp_(src, "()");
    assert_eq!(ret, 0);
}

fn main() {}
