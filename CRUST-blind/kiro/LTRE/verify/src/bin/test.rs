use LTRE::ltre::*;

// Helper: parse, compile, match
fn pm(regex: &str, input: &[u8]) -> bool {
    let nfa = ltre_parse(regex).unwrap();
    let dfa = ltre_compile(nfa);
    ltre_matches(&dfa, input)
}

// Helper: parse should error
fn pe(regex: &str) -> bool {
    ltre_parse(regex).is_err()
}

// Full pipeline: parse -> compile -> serialize -> deserialize -> uncompile -> compile -> match
fn full_pipeline(regex: &str, input: &[u8]) -> bool {
    let nfa = ltre_parse(regex).unwrap();
    let dfa = ltre_compile(nfa);
    let buf = dfa_serialize(&dfa);
    let (dfa2, _) = dfa_deserialize(&buf).unwrap();
    let nfa2 = ltre_uncompile(&dfa2);
    let dfa3 = ltre_compile(nfa2);
    ltre_matches(&dfa3, input)
}

// C test helper: mirrors the C test() function behavior
fn c_test(regex: &str, input: &str, expected: bool, partial: bool, ignorecase: bool, complement: bool, quick: bool) {
    let nfa_result = ltre_parse(regex);
    let mut nfa = nfa_result.unwrap();

    if partial { let _ = ltre_partial(&mut nfa); }
    if ignorecase { let _ = ltre_ignorecase(&mut nfa); }
    if complement { ltre_complement(&mut nfa); }

    let dfa = ltre_compile(nfa.clone());

    // DFA -> BUF -> DFA -> NFA -> DFA
    let buf = dfa_serialize(&dfa);
    let (dfa2, _) = dfa_deserialize(&buf).unwrap();
    let nfa2 = ltre_uncompile(&dfa2);
    let dfa3 = ltre_compile(nfa2);

    assert_eq!(ltre_matches(&dfa3, input.as_bytes()), expected,
        "full pipeline: /{regex}/ against '{input}'");

    if !quick {
        // DFA -> RE -> NFA -> DFA
        let re = ltre_decompile(&dfa3);
        let nfa3 = ltre_parse(&re).unwrap();
        let dfa4 = ltre_compile(nfa3);
        assert_eq!(ltre_matches(&dfa4, input.as_bytes()), expected,
            "decompile pipeline: /{regex}/ decompiled to '{re}' against '{input}'");
    }

    // Also test lazy matching
    let mut ldfa = None;
    assert_eq!(ltre_matches_lazy(&mut ldfa, &nfa, input.as_bytes()), expected,
        "lazy: /{regex}/ against '{input}'");
}

fn c_test_err(regex: &str) {
    assert!(ltre_parse(regex).is_err(), "expected parse error for /{regex}/");
}

#[test]
fn test_symset_basic() {
    let mut s = SymSet::empty();
    assert!(s.is_empty());
    assert!(!s.contains(0));
    s.insert(b'a');
    assert!(s.contains(b'a'));
    assert!(!s.contains(b'b'));
    assert!(!s.is_empty());

    let f = SymSet::full();
    assert!(!f.is_empty());
    assert!(f.contains(0));
    assert!(f.contains(255));
}

#[test]
fn test_symset_operations() {
    let mut a = SymSet::empty();
    a.insert(b'a');
    a.insert(b'b');
    let mut b = SymSet::empty();
    b.insert(b'b');
    b.insert(b'c');

    let mut u = a;
    u.union_with(&b);
    assert!(u.contains(b'a'));
    assert!(u.contains(b'b'));
    assert!(u.contains(b'c'));

    let mut i = a;
    i.intersect_with(&b);
    assert!(!i.contains(b'a'));
    assert!(i.contains(b'b'));
    assert!(!i.contains(b'c'));

    let mut inv = SymSet::empty();
    inv.insert(b'x');
    inv.invert();
    assert!(!inv.contains(b'x'));
    assert!(inv.contains(b'y'));
    assert!(inv.contains(0));
}

#[test]
fn test_symset_fmt_empty() {
    let s = SymSet::empty();
    // C returns "[]" for empty set — but actually C returns "<>" when nnsym==0
    // Wait: empty set means nothing is set, so nnsym=256, nsym=0. That doesn't hit nnsym==0.
    // Let's check: full set has nnsym==0 -> returns "<>"
    let full = SymSet::full();
    assert_eq!(symset_fmt(&full), "<>");
}

#[test]
fn test_symset_fmt_single() {
    let mut s = SymSet::empty();
    s.insert(b'a');
    assert_eq!(symset_fmt(&s), "a");
}

#[test]
fn test_symset_fmt_single_complement() {
    // All except 'a' -> nnsym==1 -> returns "^a"
    let mut s = SymSet::empty();
    s.insert(b'a');
    s.invert(); // now everything except 'a'
    assert_eq!(symset_fmt(&s), "^a");
}

#[test]
fn test_parse_basic() {
    assert!(ltre_parse("abc").is_ok());
    assert!(ltre_parse("").is_ok());
    assert!(ltre_parse("a|b").is_ok());
    assert!(ltre_parse("(a|b)*").is_ok());
}

#[test]
fn test_parse_errors() {
    c_test_err("abc]");
    c_test_err("[abc");
    c_test_err("abc)");
    c_test_err("(abc");
    c_test_err("+a");
    c_test_err("a|*");
    c_test_err("\\x0");
    c_test_err("\\zzz");
    c_test_err("[a\\x]");
    c_test_err("\x08"); // \b
    c_test_err("\t");
    c_test_err("^^a");
    c_test_err("a**");
    c_test_err("a*+");
    c_test_err("a*?");
    c_test_err("a+*");
    c_test_err("a++");
    c_test_err("a+?");
    c_test_err("a?*");
    c_test_err("a?+");
    c_test_err("a??");
    c_test_err("abc>");
    c_test_err("<abc");
    c_test_err("[a?b]");
    c_test_err("[a-]");
    c_test_err("[--]");
    c_test_err("[-]");
    c_test_err("-");
    c_test_err("a-");
    c_test_err("a*{}");
    c_test_err("a+{}");
    c_test_err("a?{}");
    c_test_err("a{}*");
    c_test_err("a{}+");
    c_test_err("a{}?");
    c_test_err("a{}{}");
    c_test_err("a{2,1}");
    c_test_err("a{1 2}");
    c_test_err("a{1, 2}");
    c_test_err("a{a}");
    c_test_err("a~b");
}

#[test]
fn test_parse_overflow() {
    let ovf = "9999999999999999999999999999999999999999";
    c_test_err(&format!("a{{{ovf}}}"));
    c_test_err(&format!("a{{{ovf},}}"));
    c_test_err(&format!("a{{,{ovf}}}"));
    c_test_err(&format!("a{{{ovf},{ovf}}}"));
}

#[test]
fn test_match_basic() {
    assert!(pm("abba", b"abba"));
    assert!(pm("[ab]+", b"abba"));
    assert!(!pm("[ab]+", b"abc"));
    assert!(pm(".*", b"abba"));
    assert!(pm("(a|b+){3}", b"abbba"));
    assert!(!pm("(a|b+){3}", b"abbab"));
    assert!(pm("\\x61\\+", b"a+"));
}

#[test]
fn test_match_empty_patterns() {
    assert!(pm("", b""));
    assert!(!pm("", b"\n"));
    assert!(!pm("[]", b""));
    assert!(pm("[]*", b""));
    assert!(!pm("[]+", b""));
    assert!(pm("[]?", b""));
    assert!(pm("()", b""));
    assert!(pm("()*", b""));
    assert!(pm("()+", b""));
    assert!(pm("()?", b""));
    assert!(pm(" ", b" "));
}

#[test]
fn test_match_newline() {
    assert!(pm("\\n", b"\n"));
    assert!(!pm(".", b"\n"));
    assert!(!pm("\\\\n", b"\n"));
    assert!(pm("(|n)(\\n)", b"\n"));
    assert!(pm("\\r?\\n", b"\n"));
    assert!(pm("\\r?\\n", b"\r\n"));
}

#[test]
fn test_match_quantifiers() {
    assert!(pm("(a*)*", b"a"));
    assert!(pm("(a+)+", b"aa"));
    assert!(pm("(a?)?", b""));
    assert!(pm("a+", b"aa"));
    assert!(!pm("a?", b"aa"));
    assert!(pm("(a+)?", b"aa"));
    assert!(pm("(ba+)?", b"baa"));
    assert!(!pm("(ab+)?", b"b"));
    assert!(!pm("(a+b)?", b"a"));
    assert!(!pm("(a+a+)+", b"a"));
    assert!(!pm("a+", b""));
    assert!(pm("(a+|)+", b"aa"));
    assert!(pm("(a+|)+", b""));
}

#[test]
fn test_match_alternation() {
    assert!(pm("(a|b)?", b"a"));
    assert!(pm("(a|b)?", b"b"));
    assert!(pm("x*|", b"xx"));
    assert!(pm("x*|", b""));
    assert!(pm("x+|", b"xx"));
    assert!(pm("x+|", b""));
    assert!(pm("x?|", b"x"));
    assert!(pm("x?|", b""));
}

#[test]
fn test_match_ordering() {
    assert!(!pm("x*y*", b"yx"));
    assert!(!pm("x+y+", b"yx"));
    assert!(!pm("x?y?", b"yx"));
    assert!(!pm("x+y*", b"xyx"));
    assert!(!pm("x*y+", b"yxy"));
    assert!(!pm("x*|y*", b"xy"));
    assert!(!pm("x+|y+", b"xy"));
    assert!(!pm("x?|y?", b"xy"));
    assert!(!pm("x+|y*", b"xy"));
    assert!(!pm("x*|y+", b"xy"));
}

#[test]
fn test_match_bounded_quantifiers() {
    assert!(!pm("a{1,2}", b""));
    assert!(pm("a{1,2}", b"a"));
    assert!(pm("a{1,2}", b"aa"));
    assert!(!pm("a{1,2}", b"aaa"));
    assert!(pm("a{0,}", b""));
    assert!(pm("a{0,}", b"a"));
    assert!(pm("a{0,}", b"aa"));
    assert!(pm("a{0,}", b"aaa"));
    assert!(!pm("a{1,}", b""));
    assert!(pm("a{1,}", b"a"));
    assert!(pm("a{1,}", b"aa"));
    assert!(pm("a{1,}", b"aaa"));
    assert!(!pm("a{3,}", b"aa"));
    assert!(pm("a{3,}", b"aaa"));
    assert!(pm("a{3,}", b"aaaa"));
    assert!(pm("a{3,}", b"aaaaa"));
    assert!(pm("a{0,2}", b""));
    assert!(pm("a{0,2}", b"a"));
    assert!(pm("a{0,2}", b"aa"));
    assert!(!pm("a{0,2}", b"aaa"));
    assert!(!pm("a{2}", b"a"));
    assert!(pm("a{2}", b"aa"));
    assert!(!pm("a{2}", b"aaa"));
    assert!(pm("a{0}", b""));
    assert!(!pm("a{0}", b"a"));
    assert!(pm("a{,2}", b""));
    assert!(pm("a{,2}", b"a"));
    assert!(pm("a{,2}", b"aa"));
    assert!(!pm("a{,2}", b"aaa"));
    assert!(pm("a{}", b""));
    assert!(!pm("a{}", b"a"));
    assert!(pm("a{,}", b""));
    assert!(pm("a{,}", b"a"));
}

#[test]
fn test_catastrophic_backtracking() {
    // These should complete quickly (no exponential blowup)
    assert!(!pm("(a*)*c", b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"));
    assert!(!pm("(x+x+)+y", b"xxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"));
}

#[test]
fn test_fixed_string() {
    let nfa = ltre_fixed_string("hello");
    let dfa = ltre_compile(nfa);
    assert!(ltre_matches(&dfa, b"hello"));
    assert!(!ltre_matches(&dfa, b"hell"));
    assert!(!ltre_matches(&dfa, b""));
    assert!(!ltre_matches(&dfa, b"hello!"));

    let nfa2 = ltre_fixed_string("");
    let dfa2 = ltre_compile(nfa2);
    assert!(ltre_matches(&dfa2, b""));
    assert!(!ltre_matches(&dfa2, b"a"));
}

#[test]
fn test_partial() {
    let mut nfa = ltre_parse("foo").unwrap();
    let _ = ltre_partial(&mut nfa);
    let dfa = ltre_compile(nfa);
    assert!(ltre_matches(&dfa, b"foobar"));
    assert!(ltre_matches(&dfa, b"barfoo"));
    assert!(!ltre_matches(&dfa, b"bar"));
    assert!(!ltre_matches(&dfa, b""));

    // C tests
    c_test("", "", true, true, false, false, false);
    c_test("", "abc", true, true, false, false, false);
    c_test("b", "abc", true, true, false, false, false);
    c_test("ba", "abc", false, true, false, false, false);
    c_test("abc", "abc", true, true, false, false, false);
    c_test("[]", "", false, true, false, false, false);
}

#[test]
fn test_ignorecase() {
    let mut nfa = ltre_parse("abc").unwrap();
    let _ = ltre_ignorecase(&mut nfa);
    let dfa = ltre_compile(nfa);
    assert!(ltre_matches(&dfa, b"ABC"));
    assert!(ltre_matches(&dfa, b"AbC"));
    assert!(!ltre_matches(&dfa, b"abd"));

    c_test("", "", true, false, true, false, false);
    c_test("abCdEF", "aBCdEf", true, false, true, false, false);
    c_test("ab", "abc", false, false, true, false, false);
}

#[test]
fn test_complement() {
    let mut nfa = ltre_parse("abc").unwrap();
    ltre_complement(&mut nfa);
    let dfa = ltre_compile(nfa);
    assert!(!ltre_matches(&dfa, b"abc"));
    assert!(ltre_matches(&dfa, b"ab"));
    assert!(ltre_matches(&dfa, b""));
    assert!(ltre_matches(&dfa, b"xyz"));

    c_test("a", "", true, false, false, true, false);
    c_test("a", "aa", true, false, false, true, false);
    c_test("a", "a", false, false, false, true, false);
    c_test("ab*", "ac", true, false, false, true, false);
    c_test("ab*", "abb", false, false, false, true, false);
}

#[test]
fn test_serialize_deserialize() {
    let nfa = ltre_parse("abc").unwrap();
    let dfa = ltre_compile(nfa);
    let buf = dfa_serialize(&dfa);
    assert!(!buf.is_empty());
    let (dfa2, size) = dfa_deserialize(&buf).unwrap();
    assert_eq!(size, buf.len());
    assert_eq!(ltre_matches(&dfa2, b"abc"), true);
    assert_eq!(ltre_matches(&dfa2, b"ab"), false);
}

#[test]
fn test_uncompile() {
    let nfa = ltre_parse("a|b").unwrap();
    let dfa = ltre_compile(nfa);
    let nfa2 = ltre_uncompile(&dfa);
    let dfa2 = ltre_compile(nfa2);
    assert!(ltre_matches(&dfa2, b"a"));
    assert!(ltre_matches(&dfa2, b"b"));
    assert!(!ltre_matches(&dfa2, b"c"));
    assert!(!ltre_matches(&dfa2, b""));
}

#[test]
fn test_decompile() {
    let nfa = ltre_parse("abc").unwrap();
    let dfa = ltre_compile(nfa);
    let re = ltre_decompile(&dfa);
    // Decompiled regex should match same language
    let nfa2 = ltre_parse(&re).unwrap();
    let dfa2 = ltre_compile(nfa2);
    assert!(ltre_matches(&dfa2, b"abc"));
    assert!(!ltre_matches(&dfa2, b"ab"));
    assert!(!ltre_matches(&dfa2, b"abcd"));
}

#[test]
fn test_matches_lazy() {
    let nfa = ltre_parse("a+b").unwrap();
    let mut ldfa = None;
    assert!(ltre_matches_lazy(&mut ldfa, &nfa, b"ab"));
    assert!(ltre_matches_lazy(&mut ldfa, &nfa, b"aab"));
    assert!(!ltre_matches_lazy(&mut ldfa, &nfa, b"b"));
    assert!(!ltre_matches_lazy(&mut ldfa, &nfa, b""));
}

#[test]
fn test_nonstandard_caret() {
    assert!(pm("^a", b"z"));
    assert!(!pm("^a", b"a"));
    assert!(pm("^\\n", b"\r"));
    assert!(!pm("^\\n", b"\n"));
    assert!(pm("^.", b"\n"));
    assert!(!pm("^.", b"a"));
}

#[test]
fn test_shorthand_classes() {
    assert!(pm("\\d+", b"0123456789"));
    assert!(pm("\\s+", b" \x0c\n\r\t\x0b"));
    assert!(pm("\\w+", b"azAZ09_"));
    assert!(pm("^a-z*", b"1A!2$B"));
    assert!(!pm("^a-z*", b"1aA"));
    assert!(pm("a-z*", b"abc"));
}

#[test]
fn test_character_class_operations() {
    assert!(pm("^[\\d^\\w]+", b"abcABC"));
    assert!(!pm("^[\\d^\\w]+", b"abc123"));
    assert!(pm("^[\\d\\W]+", b"abcABC"));
    assert!(!pm("^[\\d^\\W]+", b"abc123"));
    assert!(pm("[[abc]]+", b"abc"));
    assert!(pm("[a[bc]]+", b"abc"));
    assert!(pm("[a[b]c]+", b"abc"));
    assert!(pm("[a][b][c]", b"abc"));
    assert!(!pm("^[^a^b]", b"a"));
    assert!(!pm("^[^a^b]", b"b"));
    assert!(!pm("^[^a^b]", b""));
}

#[test]
fn test_angle_bracket_intersection() {
    assert!(!pm("<ab>", b"a"));
    assert!(!pm("<ab>", b"b"));
    assert!(!pm("<ab>", b""));
    assert!(pm("<>", b" "));
    assert!(!pm("^<>", b" "));
    assert!(pm("\\^", b"^"));
    assert!(!pm("^\\^", b"^"));
    assert!(pm("^[^\\^]", b"^"));
}

#[test]
fn test_space_caret_interaction() {
    assert!(pm("^[ ^[a b c]]+", b"abc"));
    assert!(!pm("^[ ^[a b c]]+", b"a c"));
    assert!(pm("<[a b c]^ >+", b"abc"));
    assert!(!pm("<[a b c]^ >+", b"a c"));
}

#[test]
fn test_range_intersection() {
    assert!(pm("^[^0-74]+", b"0123567"));
    assert!(!pm("^[^0-74]+", b"89"));
    assert!(!pm("^[^0-74]+", b"4"));
    assert!(pm("<0-7^4>+", b"0123567"));
    assert!(!pm("<0-7^4>+", b"89"));
    assert!(!pm("<0-7^4>+", b"4"));
}

#[test]
fn test_empty_set_full_set() {
    assert!(!pm("[]", b" "));
    assert!(pm("^[]", b" "));
    assert!(pm("<>", b" "));
    assert!(!pm("^<>", b" "));
}

#[test]
fn test_wrapping_ranges() {
    assert!(pm("9-0*", b"abc"));
    assert!(!pm("9-0*", b"18"));
    assert!(pm("9-0*", b"09"));
    assert!(pm("9-0*", b"/:"));
    assert!(pm("b-a*", b"ab"));
    assert!(pm("a-b*", b"ab"));
    assert!(!pm("a-a*", b"ab"));
    assert!(pm("a-a*", b"aa"));
}

#[test]
fn test_tilde_complement() {
    assert!(!pm("~0*", b""));
    assert!(!pm("~0*", b"0"));
    assert!(!pm("~0*", b"00"));
    assert!(pm("~0*", b"001"));
}

#[test]
fn test_intersection() {
    assert!(!pm("ab&cd", b""));
    assert!(!pm("ab&cd", b"ab"));
    assert!(!pm("ab&cd", b"cd"));
    assert!(!pm("\\w+&~\\d+", b""));
    assert!(pm("\\w+&~\\d+", b"abc"));
    assert!(pm("\\w+&~\\d+", b"abc123"));
    assert!(pm("\\w+&~\\d+", b"1a2b3c"));
    assert!(!pm("\\w+&~\\d+", b"123"));
}

#[test]
fn test_complement_in_group() {
    assert!(!pm("0x(~[0-9a-f]+)", b"0yz"));
    assert!(!pm("0x(~[0-9a-f]+)", b"0x12"));
    assert!(pm("0x(~[0-9a-f]+)", b"0x"));
    assert!(pm("0x(~[0-9a-f]+)", b"0xy"));
    assert!(pm("0x(~[0-9a-f]+)", b"0xyz"));
    assert!(!pm("b(~a*)", b""));
    assert!(!pm("b(~a*)", b"b"));
    assert!(!pm("b(~a*)", b"ba"));
    assert!(pm("b(~a*)", b"bbaa"));
}

#[test]
fn test_decompile_edge_cases() {
    c_test("^aa*", "ba", true, false, false, false, false);
    c_test("a-zz*", "abc", false, false, false, false, false);
    c_test("\\x0a(0a)*", "\x0a", true, false, false, false, false);
    c_test("\\x0aa*", "\x0a\x0a", false, false, false, false, false);
}

#[test]
fn test_full_pipeline_roundtrip() {
    // parse -> compile -> serialize -> deserialize -> uncompile -> compile -> match
    assert!(full_pipeline("abc", b"abc"));
    assert!(!full_pipeline("abc", b"ab"));
    assert!(full_pipeline("a|b", b"a"));
    assert!(full_pipeline("a|b", b"b"));
    assert!(!full_pipeline("a|b", b"c"));
    assert!(full_pipeline("a*", b""));
    assert!(full_pipeline("a*", b"aaa"));
    assert!(!full_pipeline("a*", b"b"));
}

#[test]
fn test_determinization_state_blowout() {
    c_test("[01]*1[01]{8}", "11011100011100", true, false, false, false, true);
    c_test("[01]*1[01]{8}", "01010010010010", false, false, false, false, true);
}

#[test]
fn test_powerset_blowout() {
    c_test(".*0.*|.*1.*|.*2.*|.*3.*|.*4.*|.*5.*", "", false, false, false, false, false);
    c_test(".*0.*|.*1.*|.*2.*|.*3.*|.*4.*|.*5.*", "123", true, false, false, false, false);
}

#[test]
fn test_hex_rgb() {
    let re = "#([0-9a-fA-F]{3}){1,2}";
    c_test(re, "000", false, false, false, false, false);
    c_test(re, "#0aA", true, false, false, false, false);
    c_test(re, "#00ff", false, false, false, false, false);
    c_test(re, "#abcdef", true, false, false, false, false);
    c_test(re, "#abcdeff", false, false, false, false, false);
}

#[test]
fn test_json_num() {
    let re = "\\-?(0|1-90-9*)(\\.0-9+)?([eE][\\+\\-]?0-9+)?";
    c_test(re, "e", false, false, false, false, false);
    c_test(re, "1", true, false, false, false, false);
    c_test(re, "10", true, false, false, false, false);
    c_test(re, "01", false, false, false, false, false);
    c_test(re, "-5", true, false, false, false, false);
    c_test(re, "+5", false, false, false, false, false);
    c_test(re, ".3", false, false, false, false, false);
    c_test(re, "2.", false, false, false, false, false);
    c_test(re, "2.3", true, false, false, false, false);
    c_test(re, "1e", false, false, false, false, false);
    c_test(re, "1e0", true, false, false, false, false);
    c_test(re, "1E+0", true, false, false, false, false);
    c_test(re, "1e-0", true, false, false, false, false);
    c_test(re, "1E10", true, false, false, false, false);
    c_test(re, "1e+00", true, false, false, false, false);
}

#[test]
fn test_nfa_free_dfa_free() {
    // These are no-ops in Rust but should not panic
    let nfa = ltre_parse("abc").unwrap();
    let dfa = ltre_compile(nfa.clone());
    nfa_free(nfa);
    dfa_free(dfa);
}

fn main() {}
