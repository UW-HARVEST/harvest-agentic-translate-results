use LTRE::ltre::*;

fn helper(regex: &str, input: &[u8], expected: bool) {
    let nfa = ltre_parse(regex).unwrap();
    let dfa = ltre_compile(nfa);
    assert_eq!(ltre_matches(&dfa, input), expected,
        "/{regex}/ vs {:?}", std::str::from_utf8(input));
}

fn helper_full(regex: &str, input: &[u8], expected: bool, partial: bool, ignorecase: bool, complement: bool) {
    let mut nfa = ltre_parse(regex).unwrap();
    if partial { ltre_partial(&mut nfa).unwrap(); }
    if ignorecase { ltre_ignorecase(&mut nfa).unwrap(); }
    if complement { ltre_complement(&mut nfa); }
    let dfa = ltre_compile(nfa.clone());
    assert_eq!(ltre_matches(&dfa, input), expected,
        "/{regex}/ vs {:?} (p={partial},i={ignorecase},c={complement})", std::str::from_utf8(input));
}

fn helper_roundtrip(regex: &str, input: &[u8], expected: bool) {
    let nfa = ltre_parse(regex).unwrap();
    let dfa = ltre_compile(nfa.clone());
    // serialize -> deserialize
    let buf = dfa_serialize(&dfa);
    let (dfa2, sz) = dfa_deserialize(&buf).unwrap();
    assert_eq!(sz, buf.len());
    assert_eq!(ltre_matches(&dfa2, input), expected);
    // uncompile -> recompile
    let nfa2 = ltre_uncompile(&dfa);
    let dfa3 = ltre_compile(nfa2);
    assert_eq!(ltre_matches(&dfa3, input), expected);
}

// === Basic matching ===

#[test]
fn test_basic_literal() {
    helper("abba", b"abba", true);
    helper("abba", b"abcd", false);
    helper("abba", b"", false);
}

#[test]
fn test_basic_charclass() {
    helper("[ab]+", b"abba", true);
    helper("[ab]+", b"abc", false);
}

#[test]
fn test_dot_star() {
    helper(".*", b"abba", true);
    helper(".*", b"hello", true);
    helper(".*", b"", true);
}

#[test]
fn test_alternation() {
    helper("(a|b+){3}", b"abbba", true);
    helper("(a|b+){3}", b"abbab", false);
}

#[test]
fn test_hex_escape() {
    helper("\\x61\\+", b"a+", true);
}

#[test]
fn test_empty_regex() {
    helper("", b"", true);
    helper("", b"\n", false);
}

#[test]
fn test_empty_set() {
    helper("[]", b"", false);
    helper("[]", b" ", false);
    helper("[]*", b"", true);
    helper("[]+", b"", false);
    helper("[]?", b"", true);
}

#[test]
fn test_empty_group() {
    helper("()", b"", true);
    helper("()*", b"", true);
    helper("()+", b"", true);
    helper("()?", b"", true);
}

#[test]
fn test_space() {
    helper(" ", b" ", true);
}

#[test]
fn test_newline() {
    helper("\\n", b"\n", true);
    helper(".", b"\n", false);
    helper("\\\\n", b"\n", false);
    helper("(|n)(\\n)", b"\n", true);
    helper("\\r?\\n", b"\n", true);
    helper("\\r?\\n", b"\r\n", true);
}

#[test]
fn test_quantifiers_basic() {
    helper("(a*)*", b"a", true);
    helper("(a+)+", b"aa", true);
    helper("(a?)?", b"", true);
    helper("a+", b"aa", true);
    helper("a?", b"aa", false);
    helper("(a+)?", b"aa", true);
    helper("(ba+)?", b"baa", true);
    helper("(ab+)?", b"b", false);
    helper("(a+b)?", b"a", false);
    helper("(a+a+)+", b"a", false);
    helper("a+", b"", false);
}

#[test]
fn test_alternation_with_empty() {
    helper("(a+|)+", b"aa", true);
    helper("(a+|)+", b"", true);
    helper("(a|b)?", b"a", true);
    helper("(a|b)?", b"b", true);
    helper("x*|", b"xx", true);
    helper("x*|", b"", true);
    helper("x+|", b"xx", true);
    helper("x+|", b"", true);
    helper("x?|", b"x", true);
    helper("x?|", b"", true);
}

#[test]
fn test_concat_ordering() {
    helper("x*y*", b"yx", false);
    helper("x+y+", b"yx", false);
    helper("x?y?", b"yx", false);
    helper("x+y*", b"xyx", false);
    helper("x*y+", b"yxy", false);
}

#[test]
fn test_alternation_no_match() {
    helper("x*|y*", b"xy", false);
    helper("x+|y+", b"xy", false);
    helper("x?|y?", b"xy", false);
    helper("x+|y*", b"xy", false);
    helper("x*|y+", b"xy", false);
}

#[test]
fn test_bounded_quantifier() {
    helper("a{1,2}", b"", false);
    helper("a{1,2}", b"a", true);
    helper("a{1,2}", b"aa", true);
    helper("a{1,2}", b"aaa", false);
}

#[test]
fn test_unbounded_quantifier() {
    helper("a{0,}", b"", true);
    helper("a{0,}", b"a", true);
    helper("a{0,}", b"aa", true);
    helper("a{0,}", b"aaa", true);
    helper("a{1,}", b"", false);
    helper("a{1,}", b"a", true);
    helper("a{1,}", b"aa", true);
    helper("a{3,}", b"aa", false);
    helper("a{3,}", b"aaa", true);
    helper("a{3,}", b"aaaa", true);
}

#[test]
fn test_exact_quantifier() {
    helper("a{2}", b"a", false);
    helper("a{2}", b"aa", true);
    helper("a{2}", b"aaa", false);
    helper("a{0}", b"", true);
    helper("a{0}", b"a", false);
}

#[test]
fn test_optional_bounded() {
    helper("a{0,2}", b"", true);
    helper("a{0,2}", b"a", true);
    helper("a{0,2}", b"aa", true);
    helper("a{0,2}", b"aaa", false);
    helper("a{,2}", b"", true);
    helper("a{,2}", b"a", true);
    helper("a{,2}", b"aa", true);
    helper("a{,2}", b"aaa", false);
    helper("a{}", b"", true);
    helper("a{}", b"a", false);
    helper("a{,}", b"", true);
    helper("a{,}", b"a", true);
}

// === Partial, ignorecase, complement ===

#[test]
fn test_partial() {
    helper_full("", b"", true, true, false, false);
    helper_full("", b"abc", true, true, false, false);
    helper_full("b", b"abc", true, true, false, false);
    helper_full("ba", b"abc", false, true, false, false);
    helper_full("abc", b"abc", true, true, false, false);
    helper_full("[]", b"", false, true, false, false);
}

#[test]
fn test_ignorecase() {
    helper_full("", b"", true, false, true, false);
    helper_full("abCdEF", b"aBCdEf", true, false, true, false);
    helper_full("ab", b"abc", false, false, true, false);
}

#[test]
fn test_complement() {
    helper_full("a", b"", true, false, false, true);
    helper_full("a", b"aa", true, false, false, true);
    helper_full("a", b"a", false, false, false, true);
    helper_full("ab*", b"ac", true, false, false, true);
    helper_full("ab*", b"abb", false, false, false, true);
}

// === Catastrophic backtracking (should not hang) ===

#[test]
fn test_catastrophic_backtracking() {
    helper("(a*)*c", b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", false);
    helper("(x+x+)+y", b"xxxxxxxxxxxxxxxxxxxxxxxxxxxxxx", false);
}

// === Nonstandard features ===

#[test]
fn test_caret_complement() {
    helper("^a", b"z", true);
    helper("^a", b"a", false);
    helper("^\\n", b"\r", true);
    helper("^\\n", b"\n", false);
    helper("^.", b"\n", true);
    helper("^.", b"a", false);
}

#[test]
fn test_shorthand_classes() {
    helper("\\d+", b"0123456789", true);
    helper("\\s+", b" \x0c\n\r\t\x0b", true);
    helper("\\w+", b"azAZ09_", true);
}

#[test]
fn test_caret_range() {
    helper("^a-z*", b"1A!2$B", true);
    helper("^a-z*", b"1aA", false);
    helper("a-z*", b"abc", true);
}

#[test]
fn test_nested_charclass() {
    helper("^[\\d^\\w]+", b"abcABC", true);
    helper("^[\\d^\\w]+", b"abc123", false);
    helper("[[abc]]+", b"abc", true);
    helper("[a[bc]]+", b"abc", true);
    helper("[a[b]c]+", b"abc", true);
    helper("[a][b][c]", b"abc", true);
}

#[test]
fn test_caret_in_charclass() {
    helper("^[^a^b]", b"a", false);
    helper("^[^a^b]", b"b", false);
    helper("^[^a^b]", b"", false);
}

#[test]
fn test_angle_bracket_intersection() {
    helper("<ab>", b"a", false);
    helper("<ab>", b"b", false);
    helper("<ab>", b"", false);
    helper("\\^", b"^", true);
    helper("^\\^", b"^", false);
    helper("^[^\\^]", b"^", true);
}

#[test]
fn test_mixed_union_intersection() {
    helper("^[ ^[a b c]]+", b"abc", true);
    helper("^[ ^[a b c]]+", b"a c", false);
    helper("<[a b c]^ >+", b"abc", true);
    helper("<[a b c]^ >+", b"a c", false);
    helper("^[^0-74]+", b"0123567", true);
    helper("^[^0-74]+", b"89", false);
    helper("^[^0-74]+", b"4", false);
    helper("<0-7^4>+", b"0123567", true);
    helper("<0-7^4>+", b"89", false);
    helper("<0-7^4>+", b"4", false);
}

#[test]
fn test_empty_set_complement() {
    helper("^[]", b" ", true);
    helper("<>", b" ", true);
    helper("^<>", b" ", false);
}

#[test]
fn test_wrapping_range() {
    helper("9-0*", b"abc", true);
    helper("9-0*", b"18", false);
    helper("9-0*", b"09", true);
    helper("9-0*", b"/:", true);
    helper("b-a*", b"ab", true);
    helper("a-b*", b"ab", true);
    helper("a-a*", b"ab", false);
    helper("a-a*", b"aa", true);
}

// === Tilde complement ===

#[test]
fn test_tilde_complement() {
    helper("~0*", b"", false);
    helper("~0*", b"0", false);
    helper("~0*", b"00", false);
    helper("~0*", b"001", true);
}

// === Intersection ===

#[test]
fn test_intersection() {
    helper("ab&cd", b"", false);
    helper("ab&cd", b"ab", false);
    helper("ab&cd", b"cd", false);
    helper("\\w+&~\\d+", b"", false);
    helper("\\w+&~\\d+", b"abc", true);
    helper("\\w+&~\\d+", b"abc123", true);
    helper("\\w+&~\\d+", b"1a2b3c", true);
    helper("\\w+&~\\d+", b"123", false);
}

#[test]
fn test_complement_in_group() {
    helper("0x(~[0-9a-f]+)", b"0yz", false);
    helper("0x(~[0-9a-f]+)", b"0x12", false);
    helper("0x(~[0-9a-f]+)", b"0x", true);
    helper("0x(~[0-9a-f]+)", b"0xy", true);
    helper("0x(~[0-9a-f]+)", b"0xyz", true);
    helper("b(~a*)", b"", false);
    helper("b(~a*)", b"b", false);
    helper("b(~a*)", b"ba", false);
    helper("b(~a*)", b"bbaa", true);
}

// === Parse errors ===

#[test]
fn test_parse_errors() {
    let errors = [
        "abc]", "[abc", "abc)", "(abc", "+a", "a|*", "\\x0", "\\zzz",
        "[a\\x]", "\x08", "\t", "^^a",
        "a**", "a*+", "a*?", "a+*", "a++", "a+?", "a?*", "a?+", "a??",
        "abc>", "<abc", "[a?b]", "[a-]", "[--]", "[-]", "-", "a-",
        "a*{}", "a+{}", "a?{}", "a{}*", "a{}+", "a{}?", "a{}{}", "a{2,1}",
        "a{1 2}", "a{1, 2}", "a{a}", "a~b",
    ];
    for regex in &errors {
        assert!(ltre_parse(regex).is_err(), "expected error for /{regex}/");
    }
}

#[test]
fn test_overflow_errors() {
    let ovf = "9999999999999999999999999999999999999999";
    assert!(ltre_parse(&format!("a{{{ovf}}}")).is_err());
    assert!(ltre_parse(&format!("a{{{ovf},}}")).is_err());
    assert!(ltre_parse(&format!("a{{,{ovf}}}")).is_err());
    assert!(ltre_parse(&format!("a{{{ovf},{ovf}}}")).is_err());
}

// === Decompilation edge cases ===

#[test]
fn test_decompile_edge_cases() {
    helper("^aa*", b"ba", true);
    helper("a-zz*", b"abc", false);
    helper("\\x0a(0a)*", b"\x0a", true);
    helper("\\x0aa*", b"\x0a\x0a", false);
}

// === Fixed string ===

#[test]
fn test_fixed_string() {
    let nfa = ltre_fixed_string("hello");
    let dfa = ltre_compile(nfa);
    assert_eq!(ltre_matches(&dfa, b"hello"), true);
    assert_eq!(ltre_matches(&dfa, b"world"), false);
    assert_eq!(ltre_matches(&dfa, b""), false);
}

#[test]
fn test_fixed_string_empty() {
    let nfa = ltre_fixed_string("");
    let dfa = ltre_compile(nfa);
    assert_eq!(ltre_matches(&dfa, b""), true);
    assert_eq!(ltre_matches(&dfa, b"a"), false);
}

// === Serialize / Deserialize round-trip ===

#[test]
fn test_serialize_roundtrip() {
    helper_roundtrip("abba", b"abba", true);
    helper_roundtrip("abba", b"abcd", false);
    helper_roundtrip("[ab]+", b"abba", true);
    helper_roundtrip("a{2}", b"aa", true);
    helper_roundtrip("a{2}", b"a", false);
    helper_roundtrip("", b"", true);
}

// === Lazy matching ===

#[test]
fn test_lazy_matching() {
    let nfa = ltre_parse("ab+").unwrap();
    let mut ldfa: Option<Dfa> = None;
    assert_eq!(ltre_matches_lazy(&mut ldfa, &nfa, b"abb"), true);
    assert_eq!(ltre_matches_lazy(&mut ldfa, &nfa, b"a"), false);
    assert_eq!(ltre_matches_lazy(&mut ldfa, &nfa, b"ab"), true);
    assert_eq!(ltre_matches_lazy(&mut ldfa, &nfa, b""), false);
}

#[test]
fn test_lazy_vs_eager() {
    // Verify lazy and eager produce same results
    let cases: &[(&str, &[u8], bool)] = &[
        ("(a*)*c", b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", false),
        ("abba", b"abba", true),
        ("[ab]+", b"abba", true),
        ("a{1,2}", b"aa", true),
        ("a+", b"", false),
    ];
    for &(regex, input, expected) in cases {
        let nfa = ltre_parse(regex).unwrap();
        let dfa = ltre_compile(nfa.clone());
        assert_eq!(ltre_matches(&dfa, input), expected);
        let mut ldfa: Option<Dfa> = None;
        assert_eq!(ltre_matches_lazy(&mut ldfa, &nfa, input), expected,
            "lazy mismatch for /{regex}/");
    }
}

// === Uncompile ===

#[test]
fn test_uncompile_recompile() {
    let cases: &[(&str, &[u8], bool)] = &[
        ("abba", b"abba", true),
        ("abba", b"xyz", false),
        ("[ab]+", b"abba", true),
        ("a*", b"", true),
        ("a*", b"aaa", true),
        ("a*", b"b", false),
    ];
    for &(regex, input, expected) in cases {
        let nfa = ltre_parse(regex).unwrap();
        let dfa = ltre_compile(nfa);
        let nfa2 = ltre_uncompile(&dfa);
        let dfa2 = ltre_compile(nfa2);
        assert_eq!(ltre_matches(&dfa2, input), expected,
            "uncompile mismatch for /{regex}/ vs {:?}", std::str::from_utf8(input));
    }
}

// === Decompile ===

#[test]
fn test_decompile_reparse() {
    let cases: &[(&str, &[u8], bool)] = &[
        ("abba", b"abba", true),
        ("abba", b"xyz", false),
        ("[ab]+", b"abba", true),
        ("[ab]+", b"abc", false),
        ("a{1,2}", b"a", true),
        ("a{1,2}", b"aaa", false),
        ("a*", b"", true),
        ("a*", b"aaa", true),
        ("\\d+", b"123", true),
        ("\\d+", b"abc", false),
    ];
    for &(regex, input, expected) in cases {
        let nfa = ltre_parse(regex).unwrap();
        let dfa = ltre_compile(nfa);
        let re = ltre_decompile(&dfa);
        let nfa2 = ltre_parse(&re).unwrap();
        let dfa2 = ltre_compile(nfa2);
        assert_eq!(ltre_matches(&dfa2, input), expected,
            "decompile mismatch for /{regex}/ -> /{re}/ vs {:?}", std::str::from_utf8(input));
    }
}

// === Full pipeline: parse -> compile -> serialize -> deserialize -> uncompile -> compile -> decompile -> parse -> compile ===

#[test]
fn test_full_pipeline() {
    let cases: &[(&str, &[u8], bool)] = &[
        ("abba", b"abba", true),
        ("[ab]+", b"abba", true),
        ("a{2}", b"aa", true),
        ("a{2}", b"a", false),
        (".*", b"hello", true),
    ];
    for &(regex, input, expected) in cases {
        let nfa = ltre_parse(regex).unwrap();
        let dfa = ltre_compile(nfa);
        // serialize -> deserialize
        let buf = dfa_serialize(&dfa);
        let (dfa2, _) = dfa_deserialize(&buf).unwrap();
        assert_eq!(ltre_matches(&dfa2, input), expected);
        // uncompile -> recompile
        let nfa2 = ltre_uncompile(&dfa2);
        let dfa3 = ltre_compile(nfa2);
        assert_eq!(ltre_matches(&dfa3, input), expected);
        // decompile -> reparse -> recompile
        let re = ltre_decompile(&dfa3);
        let nfa3 = ltre_parse(&re).unwrap();
        let dfa4 = ltre_compile(nfa3);
        assert_eq!(ltre_matches(&dfa4, input), expected,
            "full pipeline mismatch for /{regex}/ -> /{re}/");
    }
}

// === SymSet ===

#[test]
fn test_symset_basic() {
    let mut ss = SymSet::empty();
    assert!(ss.is_empty());
    for c in 0..=255u8 { assert!(!ss.contains(c)); }
    ss.insert(b'a');
    assert!(ss.contains(b'a'));
    assert!(!ss.contains(b'b'));
    assert!(!ss.is_empty());
}

#[test]
fn test_symset_full() {
    let ss = SymSet::full();
    for c in 0..=255u8 { assert!(ss.contains(c)); }
    assert!(!ss.is_empty());
}

#[test]
fn test_symset_invert() {
    let mut ss = SymSet::empty();
    ss.insert(b'x');
    ss.invert();
    assert!(!ss.contains(b'x'));
    assert!(ss.contains(b'y'));
}

#[test]
fn test_symset_union_intersect() {
    let mut a = SymSet::empty();
    a.insert(b'a'); a.insert(b'b');
    let mut b = SymSet::empty();
    b.insert(b'b'); b.insert(b'c');
    let mut u = a; u.union_with(&b);
    assert!(u.contains(b'a'));
    assert!(u.contains(b'b'));
    assert!(u.contains(b'c'));
    let mut i = a; i.intersect_with(&b);
    assert!(!i.contains(b'a'));
    assert!(i.contains(b'b'));
    assert!(!i.contains(b'c'));
}

#[test]
fn test_symset_fmt_full() {
    let ss = SymSet::full();
    assert_eq!(symset_fmt(&ss), "<>");
}

#[test]
fn test_symset_fmt_single() {
    let mut ss = SymSet::empty();
    ss.insert(b'a');
    assert_eq!(symset_fmt(&ss), "a");
}

#[test]
fn test_symset_fmt_complement_single() {
    // Full set minus 'a' -> nnsym==1 -> should return "^a"
    let mut only_a = SymSet::empty();
    only_a.insert(b'a');
    only_a.invert(); // now everything except 'a'
    assert_eq!(symset_fmt(&only_a), "^a");
}

// === nfa_clone ===

#[test]
fn test_nfa_clone() {
    let nfa = ltre_parse("a+b").unwrap();
    let cloned = nfa_clone(&nfa);
    let dfa1 = ltre_compile(nfa);
    let dfa2 = ltre_compile(cloned);
    assert_eq!(ltre_matches(&dfa1, b"ab"), true);
    assert_eq!(ltre_matches(&dfa2, b"ab"), true);
    assert_eq!(ltre_matches(&dfa1, b"b"), false);
    assert_eq!(ltre_matches(&dfa2, b"b"), false);
}

// === Realistic regexes ===

#[test]
fn test_hex_rgb() {
    let re = "#([0-9a-fA-F]{3}){1,2}";
    helper(re, b"000", false);
    helper(re, b"#0aA", true);
    helper(re, b"#00ff", false);
    helper(re, b"#abcdef", true);
    helper(re, b"#abcdeff", false);
}

#[test]
fn test_json_num() {
    let re = "\\-?(0|1-90-9*)(\\.0-9+)?([eE][\\+\\-]?0-9+)?";
    helper(re, b"e", false);
    helper(re, b"1", true);
    helper(re, b"10", true);
    helper(re, b"01", false);
    helper(re, b"-5", true);
    helper(re, b"+5", false);
    helper(re, b".3", false);
    helper(re, b"2.", false);
    helper(re, b"2.3", true);
    helper(re, b"1e", false);
    helper(re, b"1e0", true);
    helper(re, b"1E+0", true);
    helper(re, b"1e-0", true);
    helper(re, b"1E10", true);
    helper(re, b"1e+00", true);
}

#[test]
fn test_json_primitives() {
    let json_str = "\"(^[\\x00-\\x1f\"\\\\]|\\\\[\"\\\\/bfnrt]|\\\\u[0-9a-fA-F]{4})*\"";
    let json_num = "\\-?(0|1-90-9*)(\\.0-9+)?([eE][\\+\\-]?0-9+)?";
    let json_bool = "true|false";
    let json_null = "null";
    let re = format!("{json_str}|{json_num}|{json_bool}|{json_null}");
    helper(&re, b"nul", false);
    helper(&re, b"null", true);
    helper(&re, b"nulll", false);
    helper(&re, b"true", true);
    helper(&re, b"false", true);
    helper(&re, b"{}", false);
    helper(&re, b"[]", false);
    helper(&re, b"1,", false);
    helper(&re, b"-5.6e2", true);
}

// === Determinization state blowout ===

#[test]
fn test_state_blowout() {
    let nfa = ltre_parse("[01]*1[01]{8}").unwrap();
    let dfa = ltre_compile(nfa);
    assert_eq!(ltre_matches(&dfa, b"11011100011100"), true);
    assert_eq!(ltre_matches(&dfa, b"01010010010010"), false);
}

#[test]
fn test_powerset_blowout() {
    helper(".*0.*|.*1.*|.*2.*|.*3.*|.*4.*|.*5.*", b"", false);
    helper(".*0.*|.*1.*|.*2.*|.*3.*|.*4.*|.*5.*", b"123", true);
}

// === ltre_complement function ===

#[test]
fn test_ltre_complement_toggle() {
    let mut nfa = ltre_parse("a").unwrap();
    assert_eq!(nfa.complemented, false);
    ltre_complement(&mut nfa);
    assert_eq!(nfa.complemented, true);
    ltre_complement(&mut nfa);
    assert_eq!(nfa.complemented, false);
}

// === nfa_free / dfa_free (just ensure they don't panic) ===

#[test]
fn test_free_functions() {
    let nfa = ltre_parse("abc").unwrap();
    let dfa = ltre_compile(nfa.clone());
    dfa_free(dfa);
    nfa_free(nfa);
}

fn main() {}
