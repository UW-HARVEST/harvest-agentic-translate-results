extern crate skp as skp_crate;
#[allow(unused_imports)]
use skp_crate::skp::{
    self, ast_close, ast_delete, ast_lastnode, ast_lastnodeisempty, ast_new, ast_open, asthaserr,
    astisleaf, astisnodeentry, astisnodeexit, astleft, astnewinfo, astnodefrom, astnodeinfo,
    astnodelen, astnoderule, astnodeto, astright, asterrcolnum, asterrline, asterrpos, asterrrule,
    chr_cmp, get_close, get_qclose, is_alnum, is_alpha, is_blank, is_break, is_ctrl, is_digit,
    is_idchr, is_lower, is_oneof, is_space, is_string, is_upper, is_xdigit, match_pat, skp_, skp_2,
    skp_3, skp_4, skp_loop_len, skp_next, ASTNULL, MATCHED, MATCHED_FAIL, MATCHED_GOAL,
    MATCHED_GOALNOT, SKP_VER, SKP_VER_STR,
};

#[test]
fn test_constants() {
    assert_eq!(SKP_VER, 0x0003001C);
    assert_eq!(SKP_VER_STR, "0.3.1rc");
    assert_eq!(MATCHED_FAIL, 0);
    assert_eq!(MATCHED, 1);
    assert_eq!(MATCHED_GOAL, 2);
    assert_eq!(MATCHED_GOALNOT, 3);
    assert_eq!(ASTNULL, -1);
}

#[test]
fn test_char_classes() {
    // is_digit
    assert!(is_digit(b'0' as u32));
    assert!(is_digit(b'9' as u32));
    assert!(!is_digit(b'a' as u32));
    assert!(!is_digit(b'/' as u32));

    // is_xdigit
    assert!(is_xdigit(b'0' as u32));
    assert!(is_xdigit(b'F' as u32));
    assert!(is_xdigit(b'a' as u32));
    assert!(!is_xdigit(b'g' as u32));
    assert!(!is_xdigit(b'G' as u32));

    // is_upper, is_lower, is_alpha
    assert!(is_upper(b'A' as u32));
    assert!(is_upper(b'Z' as u32));
    assert!(!is_upper(b'a' as u32));
    assert!(is_lower(b'a' as u32));
    assert!(is_lower(b'z' as u32));
    assert!(!is_lower(b'A' as u32));
    assert!(is_alpha(b'A' as u32));
    assert!(is_alpha(b'z' as u32));
    assert!(!is_alpha(b'1' as u32));

    // is_idchr, is_alnum
    assert!(is_idchr(b'_' as u32));
    assert!(is_idchr(b'0' as u32));
    assert!(is_idchr(b'A' as u32));
    assert!(!is_idchr(b'-' as u32));
    assert!(is_alnum(b'a' as u32));
    assert!(is_alnum(b'0' as u32));
    assert!(!is_alnum(b'_' as u32));

    // is_blank — matches C exactly:
    //   if (c < 0xFF) return (c == 0x20) || (c == 0x09);
    //   else switch on c & 0xFFFFFF00 ...
    assert!(is_blank(0x20));
    assert!(is_blank(0x09));
    assert!(!is_blank(0x0A));
    assert!(!is_blank(0xA0)); // 0xA0 < 0xFF, not 0x20 or 0x09
    assert!(is_blank(0xC2A0)); // UTF-8 NBSP
    assert!(is_blank(0xE19A80));
    assert!(is_blank(0xE28080));
    assert!(is_blank(0xE2808A));
    assert!(is_blank(0xE280AF));
    // Note: in C, `c & 0xFFFFFF00` for 0xE38080 yields 0xE38000, which does NOT match
    // the `case 0x00E38080` label, so this returns false (matching the C behavior).
    assert!(!is_blank(0xE38080));

    // is_break
    assert!(is_break(0x0A));
    assert!(is_break(0x0D));
    assert!(is_break(0x0C));
    assert!(is_break(0x85));
    assert!(is_break(0x0D0A));
    assert!(!is_break(0x20));

    // is_space
    assert!(is_space(0x20));
    assert!(is_space(0x0A));
    assert!(!is_space(b'a' as u32));

    // is_ctrl
    assert!(is_ctrl(0x00));
    assert!(is_ctrl(0x1F));
    assert!(!is_ctrl(0x20));
    assert!(is_ctrl(0x7F));
}

#[test]
fn test_chr_cmp() {
    // case-sensitive
    assert!(chr_cmp(b'A' as u32, b'A' as u32, 0));
    assert!(!chr_cmp(b'A' as u32, b'a' as u32, 0));
    // case-insensitive (fold)
    assert!(chr_cmp(b'A' as u32, b'a' as u32, 1));
    assert!(chr_cmp(b'Z' as u32, b'z' as u32, 1));
    assert!(!chr_cmp(b'A' as u32, b'b' as u32, 1));
    // outside ASCII -> not folded
    assert!(!chr_cmp(0xC3A8, b'a' as u32, 1));
}

#[test]
fn test_get_close() {
    assert_eq!(get_close(b'(' as u32), b')' as u32);
    assert_eq!(get_close(b'[' as u32), b']' as u32);
    assert_eq!(get_close(b'{' as u32), b'}' as u32);
    assert_eq!(get_close(b'<' as u32), b'>' as u32);
    assert_eq!(get_close(b'a' as u32), 0);
}

#[test]
fn test_get_qclose() {
    assert_eq!(get_qclose(b'\'' as u32), b'\'' as u32);
    assert_eq!(get_qclose(b'"' as u32), b'"' as u32);
    assert_eq!(get_qclose(b'`' as u32), b'`' as u32);
    assert_eq!(get_qclose(b'a' as u32), 0);
}

#[test]
fn test_skp_next() {
    let (c, rest) = skp_next("abc", 0);
    assert_eq!(c, b'a' as u32);
    assert_eq!(rest, "bc");

    let (c, rest) = skp_next("", 0);
    assert_eq!(c, 0);
    assert_eq!(rest, "");

    // CRLF combo
    let (c, rest) = skp_next("\r\nA", 0);
    assert_eq!(c, 0x0D0A);
    assert_eq!(rest, "A");

    // UTF-8 multi-byte: è is 0xC3 0xA8
    let (c, rest) = skp_next("è", 0);
    assert_eq!(c, 0xC3A8);
    assert_eq!(rest, "");

    // ISO mode reads single byte
    let (c, _rest) = skp_next("è", 1);
    assert_eq!(c, 0xC3);
}

#[test]
fn test_is_oneof() {
    // Set "[a-z]" -> we pass the part after '['
    assert!(is_oneof(b'a' as u32, "a-z]", 0));
    assert!(is_oneof(b'm' as u32, "a-z]", 0));
    assert!(is_oneof(b'z' as u32, "a-z]", 0));
    assert!(!is_oneof(b'A' as u32, "a-z]", 0));
    assert!(!is_oneof(b'0' as u32, "a-z]", 0));

    assert!(is_oneof(b'X' as u32, "ABCXYZ]", 0));
    assert!(!is_oneof(b'D' as u32, "ABCXYZ]", 0));

    // ch == 0
    assert!(!is_oneof(0, "a-z]", 0));

    // explicit ']' as first char
    assert!(is_oneof(b']' as u32, "]abc]", 0));
}

#[test]
fn test_is_string() {
    // simple match: "abc" matches "abc"
    let r = is_string("abcXYZ", "abc", 3, 0);
    assert_eq!(r, 3);

    // case-insensitive
    let r = is_string("ABCxyz", "abc", 3, 1);
    assert_eq!(r, 3);

    // mismatch with no alternative
    let r = is_string("abcXYZ", "xyz", 3, 0);
    assert_eq!(r, 0);

    // alternatives separated by 0x0E
    let alt_pat = "xyz\x0Eabc";
    let r = is_string("abcDEF", alt_pat, alt_pat.len() as i32, 0);
    assert_eq!(r, 3);
}

#[test]
fn test_skp_basic() {
    // From C reference: skp("123X", "D\2") -> alt=2 to=3 end=3
    let mut to: &str = "";
    let mut end: &str = "";
    let alt = skp_4("123X", "D\x02", Some(&mut to), Some(&mut end));
    assert_eq!(alt, 2);
    assert_eq!(end, "X");
    assert_eq!(to, "X");

    // skp("123X", "I\2") -> alt=0
    let alt = skp_2("123X", "I\x02");
    assert_eq!(alt, 0);

    // skp("123X", "'1'\2") -> alt=2, end=1
    let mut end: &str = "";
    let alt = skp_3("123X", "'1'\x02", Some(&mut end));
    assert_eq!(alt, 2);
    assert_eq!(end, "23X");

    // skp("123X", "'12'\2") -> alt=2 end=2
    let mut end: &str = "";
    let alt = skp_3("123X", "'12'\x02", Some(&mut end));
    assert_eq!(alt, 2);
    assert_eq!(end, "3X");

    // skp("123X", "?'12'\3") -> alt=3, len=2
    let mut to: &str = "";
    let mut end: &str = "";
    let alt = skp_4("123X", "?'12'\x03", Some(&mut to), Some(&mut end));
    assert_eq!(alt, 3);
    assert_eq!(end, "3X");

    // skp("123X", "?'23'\3") -> alt=3, len=0
    let mut to: &str = "";
    let mut end: &str = "";
    let alt = skp_4("123X", "?'23'\x03", Some(&mut to), Some(&mut end));
    assert_eq!(alt, 3);
    assert_eq!(end, "123X");

    // skp("123X", "!'12'\4") -> alt=0
    let alt = skp_2("123X", "!'12'\x04");
    assert_eq!(alt, 0);

    // skp("123X", "!'23'\4") -> alt=4, len=0
    let mut end: &str = "";
    let alt = skp_3("123X", "!'23'\x04", Some(&mut end));
    assert_eq!(alt, 4);
    assert_eq!(end, "123X");
}

#[test]
fn test_skp_strings_and_alternatives() {
    // skp("ABC", "'AB'") -> alt=1, end=2
    let (alt, _to, end) = skp_("ABC", "'AB'");
    assert_eq!(alt, 1);
    assert_eq!(end, "C");

    // skp("ABC", "'XB'") -> alt=0
    let (alt, _to, end) = skp_("ABC", "'XB'");
    assert_eq!(alt, 0);
    assert_eq!(end, "ABC");

    // alternatives 'XB\xEAB'
    let (alt, _to, end) = skp_("ABC", "'XB\x0EAB'");
    assert_eq!(alt, 1);
    assert_eq!(end, "C");

    let (alt, _to, end) = skp_("ABC", "'AB\x0EXB'");
    assert_eq!(alt, 1);
    assert_eq!(end, "C");
}

#[test]
fn test_skp_unicode() {
    // skp("aèi", "'a' . 'i'") -> alt=1 len=4
    let (alt, _to, end) = skp_("aèi", "'a' . 'i'");
    assert_eq!(alt, 1);
    // 'a' (1) + è (2) + 'i' (1) = 4 bytes
    assert_eq!(end, "");

    // skp("aèi", "'aè'\2 .") -> alt=2 len=3
    let (alt, _to, end) = skp_("aèi", "'aè'\x02 .");
    assert_eq!(alt, 2);
    // After 'aè' (3 bytes) we got the goal; the trailing '.' doesn't change end since we set goal
    // Per C: alt=2, len=3, so end is 3 bytes in.
    assert_eq!(end, "i");
}

#[test]
fn test_skp_case() {
    let (alt, _to, end) = skp_("abCD", "'abCD'");
    assert_eq!(alt, 1);
    assert_eq!(end, "");

    let (alt, _to, end) = skp_("abCD", "'abcd'");
    assert_eq!(alt, 0);
    assert_eq!(end, "abCD");

    let (alt, _to, end) = skp_("abCD", "!C'abcd'");
    assert_eq!(alt, 1);
    assert_eq!(end, "");
}

#[test]
fn test_skp_classes_and_quantifiers() {
    // hello world / I W I -> alt=1 to=11 end=11
    let (alt, to, end) = skp_("hello world", "I W I");
    assert_eq!(alt, 1);
    assert_eq!(to, "");
    assert_eq!(end, "");

    // floating point
    let (alt, _to, end) = skp_("12.34", "F");
    assert_eq!(alt, 1);
    assert_eq!(end, "");

    // hex with 0x prefix
    let (alt, _to, end) = skp_("0xABcd", "X");
    assert_eq!(alt, 1);
    assert_eq!(end, "");

    // integer + alphanumeric
    let (alt, _to, end) = skp_("12cm", "D @");
    assert_eq!(alt, 1);
    assert_eq!(end, "m");

    // Quoted string
    let (alt, _to, end) = skp_("\"hello\"", "Q");
    assert_eq!(alt, 1);
    assert_eq!(end, "");

    // Balanced parens
    let (alt, _to, end) = skp_("(abc)", "B");
    assert_eq!(alt, 1);
    assert_eq!(end, "");

    // +d on "12345"
    let (alt, _to, end) = skp_("12345", "+d\x07");
    assert_eq!(alt, 7);
    assert_eq!(end, "");

    // *@ on "a1b2"
    let (alt, _to, end) = skp_("a1b2", "*@\x07");
    assert_eq!(alt, 7);
    assert_eq!(end, "");
}

#[test]
fn test_skp_skip_to() {
    // > 'bar' on "foo bar" -> alt=1, to=4, end=7
    let (alt, to, end) = skp_("foo bar", "> 'bar'");
    assert_eq!(alt, 1);
    assert_eq!(to, "bar");
    assert_eq!(end, "");

    // > 'baz' on "foo bar" -> alt=0
    let (alt, to, end) = skp_("foo bar", ">'baz'");
    assert_eq!(alt, 0);
    assert_eq!(to, "foo bar");
    assert_eq!(end, "foo bar");
}

#[test]
fn test_skp_no_match() {
    // skp("foo", "'bar'") -> alt=0, to/end at start
    let (alt, to, end) = skp_("foo", "'bar'");
    assert_eq!(alt, 0);
    assert_eq!(to, "foo");
    assert_eq!(end, "foo");

    // skp("", "I") -> alt=0
    let (alt, _to, _end) = skp_("", "I");
    assert_eq!(alt, 0);
}

#[test]
fn test_match_pat() {
    let mut flg: i32 = 0;
    // Single digit match against "5" with pattern "d"
    let (ret, _src_end, _pat_end) = match_pat("d", "5", &mut flg);
    assert_eq!(ret, MATCHED);

    // Letter against digit pattern -> fail
    let mut flg: i32 = 0;
    let (ret, _, _) = match_pat("d", "x", &mut flg);
    assert_eq!(ret, MATCHED_FAIL);

    // & is goal
    let mut flg: i32 = 0;
    let (ret, _, _) = match_pat("&", "x", &mut flg);
    assert_eq!(ret, MATCHED_GOAL);

    // !& is goalnot
    let mut flg: i32 = 0;
    let (ret, _, _) = match_pat("!&", "x", &mut flg);
    assert_eq!(ret, MATCHED_GOALNOT);
}

#[test]
fn test_skp_loop_len() {
    let s = "hello world";
    let to = &s[5..];
    let len = skp_loop_len(s, to);
    assert_eq!(len, 5);

    // Same pointer -> 0
    let len = skp_loop_len(s, s);
    assert_eq!(len, 0);
}

#[test]
fn test_ast_new_and_open_close() {
    let mut ast = ast_new().unwrap();
    assert_eq!(ast.par_cnt, 0);
    assert_eq!(ast.nodes_cnt, 0);
    assert_eq!(ast.err_pos, -1);
    assert_eq!(ast.fail, 0);
    assert_eq!(ast.cur_node, ASTNULL);

    let par = ast_open(&mut ast, 0, "rule");
    assert_eq!(par, 0);
    assert_eq!(ast.par_cnt, 1);
    assert_eq!(ast.nodes_cnt, 1);
    assert_eq!(ast.par[0], 0);

    // Open inner
    let par2 = ast_open(&mut ast, 0, "inner");
    assert_eq!(par2, 1);

    // Close inner at pos 5
    let close2 = ast_close(&mut ast, 5, par2);
    assert_eq!(close2, 2);
    assert_eq!(ast.par_cnt, 3);
    assert_eq!(ast.par[2], -1); // delta = 1 (par2..close2)

    // Close outer at pos 5
    let close = ast_close(&mut ast, 5, par);
    assert_eq!(close, 3);
    assert_eq!(ast.par_cnt, 4);
    assert_eq!(ast.par[3], -3); // delta = 3 (0..3)

    assert_eq!(ast.nodes[0].rule, "rule");
    assert_eq!(ast.nodes[0].from, 0);
    assert_eq!(ast.nodes[0].to, 5);
    assert_eq!(ast.nodes[0].delta, 3);
    assert_eq!(ast.nodes[1].rule, "inner");
    assert_eq!(ast.nodes[1].delta, 1);
}

#[test]
fn test_ast_traversal() {
    let mut ast = ast_new().unwrap();
    // Build (root (a)(b))
    let root = ast_open(&mut ast, 0, "root");
    let a = ast_open(&mut ast, 0, "a");
    ast_close(&mut ast, 1, a);
    let b = ast_open(&mut ast, 1, "b");
    ast_close(&mut ast, 2, b);
    ast_close(&mut ast, 2, root);

    // par layout: [0=root, 1=a, -1, 2=b, -1, -5]
    assert_eq!(ast.par_cnt, 6);
    assert_eq!(ast.par[0], 0);
    assert_eq!(ast.par[5], -5);

    // root's first child is at index 1 (a)
    assert_eq!(skp::astdown(&ast, 0), 1);

    // a's right sibling is at index 3 (b)
    assert_eq!(skp::astright(&ast, 1), 3);

    // b's left sibling
    assert_eq!(skp::astleft(&ast, 3), 1);

    // last sibling of a is b
    assert_eq!(skp::astlast(&ast, 1), 3);

    // first sibling of b is a
    assert_eq!(skp::astfirst(&ast, 3), 1);

    // parent of a is root (index 0)
    assert_eq!(skp::astup(&ast, 1), 0);

    // node rules
    assert_eq!(astnoderule(&ast, 0), "root");
    assert_eq!(astnoderule(&ast, 1), "a");
    assert_eq!(astnoderule(&ast, 3), "b");

    // entry/exit checks
    assert!(astisnodeentry(&ast, 0));
    assert!(astisnodeexit(&ast, 5));
    assert!(astisleaf(&ast, 1));
    assert!(astisleaf(&ast, 3));
    assert!(!astisleaf(&ast, 0));

    // node lengths
    assert_eq!(astnodelen(&ast, 1), 1);
    assert_eq!(astnodelen(&ast, 3), 1);
    assert_eq!(astnodelen(&ast, 0), 2);
}

#[test]
fn test_ast_setinfo_newinfo() {
    let mut ast = ast_new().unwrap();
    let par = ast_open(&mut ast, 0, "r");
    ast_close(&mut ast, 1, par);
    skp::ast_setinfo(&mut ast, 42, par);
    assert_eq!(astnodeinfo(&ast, par), 42);
    // Via close par index
    assert_eq!(astnodeinfo(&ast, 1), 42);

    // astnewinfo
    let mut ast2 = ast_new().unwrap();
    astnewinfo(&mut ast2, 7);
    assert_eq!(ast2.lastinfo, 7);
    assert_eq!(ast2.par_cnt, 2);
    assert_eq!(astnodeinfo(&ast2, 0), 7);
}

#[test]
fn test_ast_lastnode_and_delete() {
    let mut ast = ast_new().unwrap();
    let r = ast_open(&mut ast, 0, "r");
    let a = ast_open(&mut ast, 0, "a");
    ast_close(&mut ast, 1, a);
    let b = ast_open(&mut ast, 1, "b");
    ast_close(&mut ast, 2, b);

    let last = ast_lastnode(&ast);
    assert_eq!(last, 3); // index of b's open par
    assert!(!ast_lastnodeisempty(&ast)); // b spans 1..2

    let cnt_before = ast.par_cnt;
    ast_delete(&mut ast);
    assert_eq!(ast.par_cnt, cnt_before - 2);

    // Now last is a
    let last = ast_lastnode(&ast);
    assert_eq!(last, 1);

    ast_close(&mut ast, 2, r);
}

#[test]
fn test_ast_lastnode_isempty() {
    let mut ast = ast_new().unwrap();
    let r = ast_open(&mut ast, 0, "r");
    let a = ast_open(&mut ast, 5, "a");
    ast_close(&mut ast, 5, a); // empty: from==to
    assert!(ast_lastnodeisempty(&ast));
    ast_close(&mut ast, 5, r);
}

#[test]
fn test_ast_noleaf_noemptyleaf() {
    // ast_noleaf removes a leaf
    let mut ast = ast_new().unwrap();
    let r = ast_open(&mut ast, 0, "r");
    let a = ast_open(&mut ast, 0, "a");
    ast_close(&mut ast, 1, a);
    let pcnt_before = ast.par_cnt;
    skp::ast_noleaf(&mut ast);
    assert_eq!(ast.par_cnt, pcnt_before - 2);
    ast_close(&mut ast, 1, r);

    // ast_noemptyleaf: empty leaf removed
    let mut ast2 = ast_new().unwrap();
    let r = ast_open(&mut ast2, 0, "r");
    let a = ast_open(&mut ast2, 5, "a");
    ast_close(&mut ast2, 5, a); // empty
    let pcnt = ast2.par_cnt;
    skp::ast_noemptyleaf(&mut ast2);
    assert_eq!(ast2.par_cnt, pcnt - 2);
    ast_close(&mut ast2, 5, r);

    // Non-empty leaf NOT removed
    let mut ast3 = ast_new().unwrap();
    let r = ast_open(&mut ast3, 0, "r");
    let a = ast_open(&mut ast3, 0, "a");
    ast_close(&mut ast3, 1, a); // non-empty
    let pcnt = ast3.par_cnt;
    skp::ast_noemptyleaf(&mut ast3);
    assert_eq!(ast3.par_cnt, pcnt);
    ast_close(&mut ast3, 1, r);
}

#[test]
fn test_ast_is_isn() {
    let mut ast = ast_new().unwrap();
    let r = ast_open(&mut ast, 0, "r");
    let a = ast_open(&mut ast, 0, "a");
    ast_close(&mut ast, 1, a);
    ast_close(&mut ast, 1, r);

    assert_eq!(skp::ast_is(&ast, 1, "a"), 1);
    assert_eq!(skp::ast_is(&ast, 1, "b"), 0);
    assert_eq!(skp::ast_is(&ast, 0, "r"), 1);

    assert_eq!(
        skp::ast_isn(&ast, 1, "x", Some("a"), None, None, None),
        1
    );
    assert_eq!(
        skp::ast_isn(&ast, 1, "x", Some("y"), Some("z"), None, None),
        0
    );
}

#[test]
fn test_ast_node_from_to() {
    let mut ast = ast_new().unwrap();
    ast.start = "hello world".to_string();
    let r = ast_open(&mut ast, 0, "r");
    let a = ast_open(&mut ast, 0, "word");
    ast_close(&mut ast, 5, a);
    ast_close(&mut ast, 5, r);

    assert_eq!(astnodefrom(&ast, 1), "hello world");
    assert_eq!(astnodeto(&ast, 1), " world");
    assert_eq!(astnodelen(&ast, 1), 5);
}

#[test]
fn test_ast_swap() {
    let mut ast = ast_new().unwrap();
    let r = ast_open(&mut ast, 0, "r");
    let a = ast_open(&mut ast, 0, "a");
    ast_close(&mut ast, 1, a);
    let b = ast_open(&mut ast, 1, "b");
    ast_close(&mut ast, 2, b);

    // before: par = [r=0, a=1, -1, b=2, -1, ...]
    let before_a_open = ast.par[1];
    let before_b_open = ast.par[3];
    assert_eq!(ast.nodes[before_a_open as usize].rule, "a");
    assert_eq!(ast.nodes[before_b_open as usize].rule, "b");

    skp::ast_swap(&mut ast);

    // After swap, the position-1 par should reference b's node, position-3 should reference a's.
    let after_first = ast.par[1];
    let after_second = ast.par[3];
    assert_eq!(ast.nodes[after_first as usize].rule, "b");
    assert_eq!(ast.nodes[after_second as usize].rule, "a");

    ast_close(&mut ast, 2, r);
}

#[test]
fn test_ast_lift() {
    // Build single-child case: (outer (inner)) — after lift, the outer wrapper is removed.
    // par layout before lift: [outer_open=0, inner_open=1, inner_close=-1, outer_close=-3]
    let mut ast = ast_new().unwrap();
    let outer = ast_open(&mut ast, 0, "outer");
    let inner = ast_open(&mut ast, 0, "inner");
    ast_close(&mut ast, 1, inner);
    ast_close(&mut ast, 1, outer);
    assert_eq!(ast.par_cnt, 4);
    // outer node has tag 0, so lift should remove it
    skp::ast_lift(&mut ast);
    assert_eq!(ast.par_cnt, 2);
    // What remains is the inner node:
    assert_eq!(ast.nodes[ast.par[0] as usize].rule, "inner");
    assert_eq!(ast.par[1], -1);
}

#[test]
fn test_ast_lower() {
    // Build (root (a)(b)(c))
    let mut ast = ast_new().unwrap();
    let r = ast_open(&mut ast, 0, "r");
    let a = ast_open(&mut ast, 0, "a");
    ast_close(&mut ast, 1, a);
    let b = ast_open(&mut ast, 1, "b");
    ast_close(&mut ast, 2, b);
    let c = ast_open(&mut ast, 2, "c");
    ast_close(&mut ast, 3, c);

    // par_cnt should be 7 (1 r open, then [a b c] with 2 each = 6, total 7)
    assert_eq!(ast.par_cnt, 7);

    // Lower a (par index 1) and b (par index 3) into a new "wrap" node
    skp::ast_lower(&mut ast, "wrap", 1, 3);

    // After: par_cnt grows by 2
    assert_eq!(ast.par_cnt, 9);

    // The new wrap node should be at par index 1
    let wrap_idx = ast.par[1];
    assert!(wrap_idx >= 0);
    assert_eq!(ast.nodes[wrap_idx as usize].rule, "wrap");

    ast_close(&mut ast, 3, r);
}

#[test]
fn test_asthaserr() {
    let mut ast = ast_new().unwrap();
    assert!(!asthaserr(&ast));
    ast.err_pos = 5;
    assert!(asthaserr(&ast));
}

#[test]
fn test_asterr_funcs() {
    let mut ast = ast_new().unwrap();
    ast.start = "line one\nline two\n".to_string();
    ast.err_pos = 12; // somewhere in "line two"
    ast.err_rule = Some("rule".to_string());

    let pos = asterrpos(&ast).unwrap();
    assert_eq!(pos, "e two\n");

    let rule = asterrrule(&ast).unwrap();
    assert_eq!(rule, "rule");

    let line = asterrline(&ast);
    assert_eq!(line, "line two\n");

    let col = asterrcolnum(&ast);
    assert_eq!(col, 3);
}

#[test]
fn test_asterr_funcs_no_err() {
    let ast = ast_new().unwrap();
    assert_eq!(asterrpos(&ast).unwrap(), "");
    assert_eq!(asterrrule(&ast).unwrap(), "");
    assert_eq!(asterrline(&ast), "");
    assert_eq!(asterrcolnum(&ast), 0);
}

#[test]
fn test_skp_debug2() {
    let mut ast = ast_new().unwrap();
    assert_eq!(skp::skp_debug2(&mut ast, 1), 1);
    assert!((ast.flg & skp::SKP_DEBUG) != 0);
    assert_eq!(skp::skp_debug2(&mut ast, 0), 0);
    assert_eq!(ast.flg & skp::SKP_DEBUG, 0);
    // Toggle
    skp::skp_debug2(&mut ast, 0xFF);
    assert!(ast.flg & skp::SKP_DEBUG != 0);
}

#[test]
fn test_astnextdf_isentry_isexit() {
    let mut ast = ast_new().unwrap();
    let r = ast_open(&mut ast, 0, "r");
    let a = ast_open(&mut ast, 0, "a");
    ast_close(&mut ast, 1, a);
    ast_close(&mut ast, 1, r);
    // par layout: [0,1,-1,-3]
    assert_eq!(ast.par_cnt, 4);

    // Walk from -1
    let mut node: i32 = ASTNULL;
    node = skp::astnextdf(&ast, node);
    assert_eq!(node, 0);
    assert!(astisnodeentry(&ast, node));
    assert!(!astisnodeexit(&ast, node));
    node = skp::astnextdf(&ast, node);
    assert_eq!(node, 1);
    assert!(astisnodeentry(&ast, node));
    node = skp::astnextdf(&ast, node);
    assert_eq!(node, 2);
    assert!(astisnodeexit(&ast, node));
    node = skp::astnextdf(&ast, node);
    assert_eq!(node, 3);
    assert!(astisnodeexit(&ast, node));
    node = skp::astnextdf(&ast, node);
    assert_eq!(node, ASTNULL);
}

#[test]
fn test_skp_abort_sets_state() {
    let mut ast = ast_new().unwrap();
    ast.pos = 7;
    skp::skp__abort(&mut ast, "boom", "rule1");
    assert_eq!(ast.fail, 1);
    assert_eq!(ast.err_pos, 7);
    assert_eq!(ast.err_rule.as_deref(), Some("rule1"));
    assert_eq!(ast.err_msg.as_deref(), Some("boom"));
}

#[test]
fn test_skp_parse_minimal() {
    fn rule(_ast: &mut skp::Ast, _ret: &mut i32) {}
    let ast = skp::skp_parse("hello", rule, "main", 0).unwrap();
    assert_eq!(ast.start, "hello");
    // Even a trivial rule should produce one node (the root)
    assert_eq!(ast.nodes_cnt, 1);
    assert_eq!(ast.par_cnt, 2);
    assert_eq!(ast.nodes[0].rule, "main");
}

fn main() {}
