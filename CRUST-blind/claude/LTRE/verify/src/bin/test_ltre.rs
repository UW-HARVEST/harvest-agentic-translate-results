#![allow(non_snake_case)]
use LTRE::ltre::*;

fn compile_re(pattern: &str) -> Dfa {
    let nfa = ltre_parse(pattern).expect("parse failed");
    ltre_compile(nfa)
}

fn matches_re(pattern: &str, input: &str) -> bool {
    let dfa = compile_re(pattern);
    ltre_matches(&dfa, input.as_bytes())
}

// =========================================================================
// SymSet tests
// =========================================================================
#[test]
fn test_symset_empty() {
    let s = SymSet::empty();
    assert!(s.is_empty());
    for c in 0..=255u32 {
        assert!(!s.contains(c as u8));
    }
}

#[test]
fn test_symset_full() {
    let s = SymSet::full();
    assert!(!s.is_empty());
    for c in 0..=255u32 {
        assert!(s.contains(c as u8));
    }
}

#[test]
fn test_symset_insert_contains() {
    let mut s = SymSet::empty();
    assert!(!s.contains(b'a'));
    s.insert(b'a');
    assert!(s.contains(b'a'));
    assert!(!s.contains(b'b'));
    s.insert(b'b');
    assert!(s.contains(b'a'));
    assert!(s.contains(b'b'));
    assert!(!s.is_empty());
}

#[test]
fn test_symset_invert() {
    let mut s = SymSet::empty();
    s.insert(b'a');
    s.invert();
    assert!(!s.contains(b'a'));
    assert!(s.contains(b'b'));
    assert!(s.contains(0));
    assert!(s.contains(255));
    // count of set bits is 255
    let mut n = 0;
    for c in 0..=255u32 {
        if s.contains(c as u8) {
            n += 1;
        }
    }
    assert_eq!(n, 255);
}

#[test]
fn test_symset_union_intersect() {
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
    assert!(!u.contains(b'd'));

    let mut i = a;
    i.intersect_with(&b);
    assert!(!i.contains(b'a'));
    assert!(i.contains(b'b'));
    assert!(!i.contains(b'c'));
}

#[test]
fn test_symset_is_empty() {
    let mut s = SymSet::empty();
    assert!(s.is_empty());
    s.insert(0);
    assert!(!s.is_empty());
    let f = SymSet::full();
    assert!(!f.is_empty());
}

#[test]
fn test_symset_eq() {
    let a = SymSet::empty();
    let mut b = SymSet::empty();
    assert_eq!(a, b);
    b.insert(0);
    assert_ne!(a, b);
}

// =========================================================================
// symset_fmt tests (ground truth from C)
// =========================================================================
#[test]
fn test_symset_fmt_empty() {
    let s = SymSet::empty();
    // C: nnsym==256, nsym==0, returns "^\x00-\xff"
    assert_eq!(symset_fmt(&s), "^\\x00-\\xff");
}

#[test]
fn test_symset_fmt_full() {
    let s = SymSet::full();
    // C: nnsym==0 -> "<>"
    assert_eq!(symset_fmt(&s), "<>");
}

#[test]
fn test_symset_fmt_single_a() {
    let mut s = SymSet::empty();
    s.insert(b'a');
    // C: nsym==1 -> "a" (just the symbol, no brackets)
    assert_eq!(symset_fmt(&s), "a");
}

#[test]
fn test_symset_fmt_a_to_z() {
    let mut s = SymSet::empty();
    for c in b'a'..=b'z' {
        s.insert(c);
    }
    // From C (with proper bool return): "a-z" (after nsym==1 special case)
    assert_eq!(symset_fmt(&s), "a-z");
}

#[test]
fn test_symset_fmt_abc() {
    let mut s = SymSet::empty();
    s.insert(b'a');
    s.insert(b'b');
    s.insert(b'c');
    // From C (with proper bool return): "a-c"
    assert_eq!(symset_fmt(&s), "a-c");
}

#[test]
fn test_symset_fmt_a_c_noncontig() {
    let mut s = SymSet::empty();
    s.insert(b'a');
    s.insert(b'c');
    // C: '[ac]'
    assert_eq!(symset_fmt(&s), "[ac]");
}

#[test]
fn test_symset_fmt_digits() {
    let mut s = SymSet::empty();
    for c in b'0'..=b'9' {
        s.insert(c);
    }
    // From C: "0-9"
    assert_eq!(symset_fmt(&s), "0-9");
}

#[test]
fn test_symset_fmt_ab() {
    let mut s = SymSet::empty();
    s.insert(b'a');
    s.insert(b'b');
    // From C: "[ab]"
    assert_eq!(symset_fmt(&s), "[ab]");
}

#[test]
fn test_symset_fmt_not_a() {
    let mut s = SymSet::full();
    let mut na = SymSet::empty();
    na.insert(b'a');
    na.invert();
    s.intersect_with(&na);
    // C: '^a'
    assert_eq!(symset_fmt(&s), "^a");
}

#[test]
fn test_symset_fmt_not_newline() {
    let mut s = SymSet::full();
    let mut nb = SymSet::empty();
    nb.insert(b'\n');
    nb.invert();
    s.intersect_with(&nb);
    // C: '^\x0a'
    assert_eq!(symset_fmt(&s), "^\\x0a");
}

#[test]
fn test_symset_fmt_nul_a() {
    let mut s = SymSet::empty();
    s.insert(0);
    s.insert(b'a');
    // C: '[\x00a]'
    assert_eq!(symset_fmt(&s), "[\\x00a]");
}

// =========================================================================
// Nfa basic tests
// =========================================================================
#[test]
fn test_nfa_new_single() {
    let nfa = Nfa::new_single();
    assert_eq!(nfa.states.len(), 1);
    assert_eq!(nfa.initial, 0);
    assert_eq!(nfa.final_, 0);
    assert!(!nfa.complemented);
    assert_eq!(nfa.len(), 1);
}

#[test]
fn test_nfa_clone() {
    let nfa = ltre_parse("abc").unwrap();
    let cloned = nfa_clone(&nfa);
    assert_eq!(nfa.states.len(), cloned.states.len());
    assert_eq!(nfa.initial, cloned.initial);
    assert_eq!(nfa.final_, cloned.final_);
    assert_eq!(nfa.complemented, cloned.complemented);
}

#[test]
fn test_nfa_pad_initial() {
    let mut nfa = ltre_parse("a").unwrap();
    let orig_size = nfa.states.len();
    let orig_initial = nfa.initial;
    nfa_pad_initial(&mut nfa);
    assert_eq!(nfa.states.len(), orig_size + 1);
    assert_ne!(nfa.initial, orig_initial);
    assert_eq!(nfa.states[nfa.initial].epsilon0, Some(orig_initial));
    // After padding, can still compile and match correctly
    let dfa = ltre_compile(nfa);
    assert!(ltre_matches(&dfa, b"a"));
    assert!(!ltre_matches(&dfa, b"b"));
    assert!(!ltre_matches(&dfa, b""));
}

#[test]
fn test_nfa_pad_final() {
    let mut nfa = ltre_parse("a").unwrap();
    let orig_size = nfa.states.len();
    let orig_final = nfa.final_;
    nfa_pad_final(&mut nfa);
    assert_eq!(nfa.states.len(), orig_size + 1);
    assert_ne!(nfa.final_, orig_final);
    assert_eq!(nfa.states[orig_final].epsilon0, Some(nfa.final_));
    let dfa = ltre_compile(nfa);
    assert!(ltre_matches(&dfa, b"a"));
    assert!(!ltre_matches(&dfa, b"b"));
}

#[test]
fn test_nfa_uncomplement_noop() {
    let mut nfa = ltre_parse("a").unwrap();
    let orig_complemented = nfa.complemented;
    assert!(!orig_complemented);
    let r = nfa_uncomplement(&mut nfa);
    assert!(r.is_ok());
    assert!(!nfa.complemented);
}

#[test]
fn test_nfa_uncomplement_when_complemented() {
    let mut nfa = ltre_parse("a").unwrap();
    ltre_complement(&mut nfa);
    assert!(nfa.complemented);
    let r = nfa_uncomplement(&mut nfa);
    assert!(r.is_ok());
    assert!(!nfa.complemented);
    // semantics still complemented
    let dfa = ltre_compile(nfa);
    assert!(!ltre_matches(&dfa, b"a"));
    assert!(ltre_matches(&dfa, b"b"));
    assert!(ltre_matches(&dfa, b""));
}

#[test]
fn test_nfa_concat_basic() {
    let nfa1 = ltre_parse("ab").unwrap();
    let nfa2 = ltre_parse("cd").unwrap();
    let mut nfa = nfa1;
    nfa_concat(&mut nfa, nfa2);
    let dfa = ltre_compile(nfa);
    assert!(ltre_matches(&dfa, b"abcd"));
    assert!(!ltre_matches(&dfa, b"ab"));
    assert!(!ltre_matches(&dfa, b"cd"));
    assert!(!ltre_matches(&dfa, b"abc"));
    assert!(!ltre_matches(&dfa, b"bcd"));
}

#[test]
fn test_nfa_concat_with_empty_first() {
    let mut nfa1 = Nfa::new_single();
    let nfa2 = ltre_parse("ab").unwrap();
    nfa_concat(&mut nfa1, nfa2);
    let dfa = ltre_compile(nfa1);
    assert!(ltre_matches(&dfa, b"ab"));
    assert!(!ltre_matches(&dfa, b""));
    assert!(!ltre_matches(&dfa, b"a"));
}

#[test]
fn test_nfa_concat_with_empty_second() {
    let mut nfa1 = ltre_parse("ab").unwrap();
    let nfa2 = Nfa::new_single();
    nfa_concat(&mut nfa1, nfa2);
    let dfa = ltre_compile(nfa1);
    assert!(ltre_matches(&dfa, b"ab"));
    assert!(!ltre_matches(&dfa, b""));
    assert!(!ltre_matches(&dfa, b"a"));
}

// =========================================================================
// Dfa basic tests
// =========================================================================
#[test]
fn test_dfa_new() {
    let dfa = Dfa::new();
    assert_eq!(dfa.states.len(), 0);
    assert_eq!(dfa.initial, 0);
    assert_eq!(dfa.len(), 0);
}

#[test]
fn test_ltre_compile_simple() {
    // /a/ - DFA size 3 (initial, accepting, dead)
    let dfa = compile_re("a");
    assert_eq!(dfa.states.len(), 3);
}

#[test]
fn test_ltre_compile_empty_re() {
    // /[]/ matches nothing - DFA size 1
    let dfa = compile_re("[]");
    assert_eq!(dfa.states.len(), 1);
    assert!(!dfa.states[dfa.initial].accepting);
}

#[test]
fn test_ltre_compile_epsilon() {
    // // matches only "" - DFA size 2
    let dfa = compile_re("");
    assert_eq!(dfa.states.len(), 2);
    assert!(dfa.states[dfa.initial].accepting);
}

#[test]
fn test_ltre_compile_star() {
    // /a*/ - DFA size 2
    let dfa = compile_re("a*");
    assert_eq!(dfa.states.len(), 2);
}

#[test]
fn test_ltre_compile_abc() {
    // /abc/ - DFA size 5
    let dfa = compile_re("abc");
    assert_eq!(dfa.states.len(), 5);
}

#[test]
fn test_ltre_compile_charclass() {
    // /[abc]/ - DFA size 3 (initial, accept, dead)
    let dfa = compile_re("[abc]");
    assert_eq!(dfa.states.len(), 3);
}

// =========================================================================
// dfa_serialize / dfa_deserialize tests
// =========================================================================
#[test]
fn test_dfa_serialize_a() {
    // From C: [a] sz=14 bytes: 03 00 60 01 00 02 9d 01 01 ff 01 02 ff 01
    let dfa = compile_re("a");
    let buf = dfa_serialize(&dfa);
    let expected: Vec<u8> = vec![
        0x03, 0x00, 0x60, 0x01, 0x00, 0x02, 0x9d, 0x01, 0x01, 0xff, 0x01, 0x02, 0xff, 0x01,
    ];
    assert_eq!(buf, expected);
}

#[test]
fn test_dfa_serialize_emptyre() {
    // From C: [] sz=4 bytes - dfa_size=1
    let dfa = compile_re("[]");
    let buf = dfa_serialize(&dfa);
    // dfa_size = 1, single non-accepting non-terminating state
    // Actually it's terminating since all transitions to itself.
    // flags = (accepting<<1)|terminating = 0|1 = 1
    // run: (255<<7|1)... let's just verify with C output
    // From C: [[]] sz=4 bytes is 4 bytes for dfa_size=1
    // Format: 01 (size=1) 01 (flags: term=1) ff (run=255 = 256 chars) 00 (target=0)
    let expected: Vec<u8> = vec![0x01, 0x01, 0xff, 0x00];
    assert_eq!(buf, expected);
}

#[test]
fn test_dfa_serialize_star() {
    // From C: [a*] sz=11 bytes: 02 02 60 01 00 00 9d 01 01 ff 01
    let dfa = compile_re("a*");
    let buf = dfa_serialize(&dfa);
    let expected: Vec<u8> = vec![0x02, 0x02, 0x60, 0x01, 0x00, 0x00, 0x9d, 0x01, 0x01, 0xff, 0x01];
    assert_eq!(buf, expected);
}

#[test]
fn test_dfa_serialize_abc() {
    // From C: [abc] sz=28 bytes
    let dfa = compile_re("abc");
    let buf = dfa_serialize(&dfa);
    let expected: Vec<u8> = vec![
        0x05, 0x00, 0x60, 0x01, 0x00, 0x02, 0x9d, 0x01, 0x01, 0xff, 0x01, 0x00, 0x61, 0x01, 0x00,
        0x03, 0x9c, 0x01, 0x00, 0x62, 0x01, 0x00, 0x04, 0x9b, 0x01, 0x02, 0xff, 0x01,
    ];
    assert_eq!(buf, expected);
}

#[test]
fn test_dfa_deserialize_roundtrip() {
    let patterns = ["a", "abc", "[abc]", "a*", "a|b", "a+", "a?", "(ab|cd)+"];
    for pat in patterns.iter() {
        let dfa = compile_re(pat);
        let buf = dfa_serialize(&dfa);
        let (dfa2, used) = dfa_deserialize(&buf).expect("deserialize");
        assert_eq!(used, buf.len(), "pattern: {}", pat);
        assert_eq!(dfa.states.len(), dfa2.states.len(), "pattern: {}", pat);
        // Check semantics roundtrip
        for inp in ["", "a", "ab", "abc", "abcd", "b", "c", "x"].iter() {
            let m1 = ltre_matches(&dfa, inp.as_bytes());
            let m2 = ltre_matches(&dfa2, inp.as_bytes());
            assert_eq!(m1, m2, "/{}/ vs '{}': differ", pat, inp);
        }
    }
}

#[test]
fn test_dfa_deserialize_empty_re() {
    let dfa = compile_re("[]");
    let buf = dfa_serialize(&dfa);
    let (dfa2, _) = dfa_deserialize(&buf).expect("deserialize");
    assert_eq!(dfa2.states.len(), 1);
    assert!(!dfa2.states[0].accepting);
    assert!(dfa2.states[0].terminating);
}

// =========================================================================
// ltre_parse tests
// =========================================================================
#[test]
fn test_parse_simple_ok() {
    assert!(ltre_parse("a").is_ok());
    assert!(ltre_parse("abc").is_ok());
    assert!(ltre_parse("").is_ok());
    assert!(ltre_parse("[]").is_ok());
    assert!(ltre_parse(".").is_ok());
}

#[test]
fn test_parse_errors() {
    assert!(ltre_parse("abc]").is_err());
    assert!(ltre_parse("[abc").is_err());
    assert!(ltre_parse("abc)").is_err());
    assert!(ltre_parse("(abc").is_err());
    assert!(ltre_parse("+a").is_err());
    assert!(ltre_parse("a|*").is_err());
    assert!(ltre_parse("\\x0").is_err());
    assert!(ltre_parse("\\zzz").is_err());
    assert!(ltre_parse("[a\\x]").is_err());
    assert!(ltre_parse("\x08").is_err());
    assert!(ltre_parse("\t").is_err());
    assert!(ltre_parse("^^a").is_err());
    assert!(ltre_parse("a**").is_err());
    assert!(ltre_parse("a*+").is_err());
    assert!(ltre_parse("a*?").is_err());
    assert!(ltre_parse("a+*").is_err());
    assert!(ltre_parse("a++").is_err());
    assert!(ltre_parse("a+?").is_err());
    assert!(ltre_parse("a?*").is_err());
    assert!(ltre_parse("a?+").is_err());
    assert!(ltre_parse("a??").is_err());
    assert!(ltre_parse("abc>").is_err());
    assert!(ltre_parse("<abc").is_err());
    assert!(ltre_parse("[a?b]").is_err());
    assert!(ltre_parse("[a-]").is_err());
    assert!(ltre_parse("[--]").is_err());
    assert!(ltre_parse("[-]").is_err());
    assert!(ltre_parse("-").is_err());
    assert!(ltre_parse("a-").is_err());
    assert!(ltre_parse("a*{}").is_err());
    assert!(ltre_parse("a{2,1}").is_err());
    assert!(ltre_parse("a{1 2}").is_err());
    assert!(ltre_parse("a~b").is_err());
    assert!(ltre_parse("a{a}").is_err());
}

#[test]
fn test_parse_natural_overflow() {
    assert!(ltre_parse("a{9999999999999999999999999999999999999999}").is_err());
    assert!(ltre_parse("a{9999999999999999999999999999999999999999,}").is_err());
    assert!(ltre_parse("a{,9999999999999999999999999999999999999999}").is_err());
}

// =========================================================================
// ltre_matches tests (functional)
// =========================================================================
#[test]
fn test_match_basic() {
    assert!(matches_re("abba", "abba"));
    assert!(matches_re("[ab]+", "abba"));
    assert!(!matches_re("[ab]+", "abc"));
    assert!(matches_re(".*", "abba"));
    assert!(matches_re("(a|b+){3}", "abbba"));
    assert!(!matches_re("(a|b+){3}", "abbab"));
}

#[test]
fn test_match_empty_re() {
    assert!(matches_re("", ""));
    assert!(!matches_re("", "\n"));
    assert!(!matches_re("[]", ""));
    assert!(matches_re("[]*", ""));
    assert!(!matches_re("[]+", ""));
    assert!(matches_re("[]?", ""));
    assert!(matches_re("()", ""));
    assert!(matches_re("()*", ""));
    assert!(matches_re("()+", ""));
    assert!(matches_re("()?", ""));
}

#[test]
fn test_match_escapes() {
    assert!(matches_re("\\x61\\+", "a+"));
    assert!(matches_re("\\n", "\n"));
    assert!(!matches_re(".", "\n"));
    assert!(!matches_re("\\\\n", "\n"));
    assert!(matches_re("(|n)(\\n)", "\n"));
    assert!(matches_re("\\r?\\n", "\n"));
    assert!(matches_re("\\r?\\n", "\r\n"));
}

#[test]
fn test_match_quantifiers() {
    assert!(matches_re("(a*)*", "a"));
    assert!(matches_re("(a+)+", "aa"));
    assert!(matches_re("(a?)?", ""));
    assert!(matches_re("a+", "aa"));
    assert!(!matches_re("a?", "aa"));
    assert!(!matches_re("a+", ""));
    assert!(!matches_re("(ab+)?", "b"));
    assert!(!matches_re("(a+b)?", "a"));
    assert!(!matches_re("(a+a+)+", "a"));
}

#[test]
fn test_match_bounded() {
    assert!(!matches_re("a{1,2}", ""));
    assert!(matches_re("a{1,2}", "a"));
    assert!(matches_re("a{1,2}", "aa"));
    assert!(!matches_re("a{1,2}", "aaa"));

    assert!(matches_re("a{0,}", ""));
    assert!(matches_re("a{0,}", "a"));
    assert!(matches_re("a{0,}", "aa"));

    assert!(!matches_re("a{1,}", ""));
    assert!(matches_re("a{1,}", "a"));

    assert!(!matches_re("a{3,}", "aa"));
    assert!(matches_re("a{3,}", "aaa"));
    assert!(matches_re("a{3,}", "aaaa"));

    assert!(matches_re("a{2}", "aa"));
    assert!(!matches_re("a{2}", "a"));
    assert!(!matches_re("a{2}", "aaa"));

    assert!(matches_re("a{0}", ""));
    assert!(!matches_re("a{0}", "a"));

    // Optional bounded forms
    assert!(matches_re("a{,2}", ""));
    assert!(matches_re("a{,2}", "aa"));
    assert!(!matches_re("a{,2}", "aaa"));
    assert!(matches_re("a{}", ""));
    assert!(!matches_re("a{}", "a"));
    assert!(matches_re("a{,}", ""));
    assert!(matches_re("a{,}", "a"));
}

#[test]
fn test_match_char_classes() {
    assert!(matches_re("\\d+", "0123456789"));
    assert!(matches_re("\\s+", " \x0c\n\r\t\x0b"));
    assert!(matches_re("\\w+", "azAZ09_"));
}

#[test]
fn test_match_complement_re() {
    assert!(matches_re("^a", "z"));
    assert!(!matches_re("^a", "a"));
    assert!(matches_re("^\\n", "\r"));
    assert!(!matches_re("^\\n", "\n"));
    assert!(matches_re("^.", "\n"));
    assert!(!matches_re("^.", "a"));
}

#[test]
fn test_match_intersection() {
    assert!(!matches_re("ab&cd", ""));
    assert!(!matches_re("ab&cd", "ab"));
    assert!(!matches_re("ab&cd", "cd"));
    assert!(!matches_re("\\w+&~\\d+", ""));
    assert!(matches_re("\\w+&~\\d+", "abc"));
    assert!(matches_re("\\w+&~\\d+", "abc123"));
    assert!(!matches_re("\\w+&~\\d+", "123"));
}

#[test]
fn test_match_complement_op() {
    assert!(!matches_re("~0*", ""));
    assert!(!matches_re("~0*", "0"));
    assert!(matches_re("~0*", "001"));
}

// =========================================================================
// ltre_fixed_string tests
// =========================================================================
#[test]
fn test_fixed_string_basic() {
    let nfa = ltre_fixed_string("hello");
    let dfa = ltre_compile(nfa);
    assert!(ltre_matches(&dfa, b"hello"));
    assert!(!ltre_matches(&dfa, b"hell"));
    assert!(!ltre_matches(&dfa, b"world"));
    assert!(!ltre_matches(&dfa, b""));
}

#[test]
fn test_fixed_string_empty() {
    let nfa = ltre_fixed_string("");
    let dfa = ltre_compile(nfa);
    assert!(ltre_matches(&dfa, b""));
    assert!(!ltre_matches(&dfa, b"a"));
}

#[test]
fn test_fixed_string_metachars() {
    // metacharacters as literal
    let nfa = ltre_fixed_string("a+b");
    let dfa = ltre_compile(nfa);
    assert!(ltre_matches(&dfa, b"a+b"));
    assert!(!ltre_matches(&dfa, b"aab"));
    assert!(!ltre_matches(&dfa, b"ab"));
}

// =========================================================================
// ltre_partial tests
// =========================================================================
#[test]
fn test_partial_basic() {
    let mut nfa = ltre_parse("abc").unwrap();
    ltre_partial(&mut nfa).unwrap();
    let dfa = ltre_compile(nfa);
    assert!(ltre_matches(&dfa, b"abc"));
    assert!(ltre_matches(&dfa, b"xabcy"));
    assert!(ltre_matches(&dfa, b"xabc"));
    assert!(ltre_matches(&dfa, b"abcy"));
    assert!(!ltre_matches(&dfa, b"ab"));
    assert!(!ltre_matches(&dfa, b"bc"));
}

#[test]
fn test_partial_empty() {
    let mut nfa = ltre_parse("").unwrap();
    ltre_partial(&mut nfa).unwrap();
    let dfa = ltre_compile(nfa);
    assert!(ltre_matches(&dfa, b""));
    assert!(ltre_matches(&dfa, b"abc"));
}

#[test]
fn test_partial_b() {
    let mut nfa = ltre_parse("b").unwrap();
    ltre_partial(&mut nfa).unwrap();
    let dfa = ltre_compile(nfa);
    assert!(ltre_matches(&dfa, b"abc"));
    assert!(!ltre_matches(&dfa, b"ac"));
}

#[test]
fn test_partial_negative() {
    let mut nfa = ltre_parse("ba").unwrap();
    ltre_partial(&mut nfa).unwrap();
    let dfa = ltre_compile(nfa);
    assert!(!ltre_matches(&dfa, b"abc"));
    assert!(ltre_matches(&dfa, b"xbay"));
}

#[test]
fn test_partial_empty_set() {
    let mut nfa = ltre_parse("[]").unwrap();
    ltre_partial(&mut nfa).unwrap();
    let dfa = ltre_compile(nfa);
    assert!(!ltre_matches(&dfa, b""));
    assert!(!ltre_matches(&dfa, b"a"));
}

// =========================================================================
// ltre_ignorecase tests
// =========================================================================
#[test]
fn test_ignorecase_match() {
    let mut nfa = ltre_parse("abCdEF").unwrap();
    ltre_ignorecase(&mut nfa).unwrap();
    let dfa = ltre_compile(nfa);
    assert!(ltre_matches(&dfa, b"aBCdEf"));
    assert!(ltre_matches(&dfa, b"abcdef"));
    assert!(ltre_matches(&dfa, b"ABCDEF"));
    assert!(!ltre_matches(&dfa, b"xbCdEF"));
}

#[test]
fn test_ignorecase_empty() {
    let mut nfa = ltre_parse("").unwrap();
    ltre_ignorecase(&mut nfa).unwrap();
    let dfa = ltre_compile(nfa);
    assert!(ltre_matches(&dfa, b""));
    assert!(!ltre_matches(&dfa, b"a"));
}

#[test]
fn test_ignorecase_partial_no() {
    let mut nfa = ltre_parse("ab").unwrap();
    ltre_ignorecase(&mut nfa).unwrap();
    let dfa = ltre_compile(nfa);
    // not partial, must match whole string
    assert!(!ltre_matches(&dfa, b"abc"));
    assert!(ltre_matches(&dfa, b"AB"));
    assert!(ltre_matches(&dfa, b"Ab"));
}

// =========================================================================
// ltre_complement tests
// =========================================================================
#[test]
fn test_complement_flag() {
    let mut nfa = ltre_parse("a").unwrap();
    assert!(!nfa.complemented);
    ltre_complement(&mut nfa);
    assert!(nfa.complemented);
    ltre_complement(&mut nfa);
    assert!(!nfa.complemented);
}

#[test]
fn test_complement_match() {
    let mut nfa = ltre_parse("a").unwrap();
    ltre_complement(&mut nfa);
    let dfa = ltre_compile(nfa);
    assert!(!ltre_matches(&dfa, b"a"));
    assert!(ltre_matches(&dfa, b""));
    assert!(ltre_matches(&dfa, b"aa"));
    assert!(ltre_matches(&dfa, b"b"));
}

#[test]
fn test_complement_ab_star() {
    let mut nfa = ltre_parse("ab*").unwrap();
    ltre_complement(&mut nfa);
    let dfa = ltre_compile(nfa);
    assert!(ltre_matches(&dfa, b"ac"));
    assert!(!ltre_matches(&dfa, b"abb"));
    assert!(!ltre_matches(&dfa, b"a"));
    assert!(!ltre_matches(&dfa, b"abbbb"));
}

// =========================================================================
// ltre_uncompile / ltre_decompile tests
// =========================================================================
#[test]
fn test_uncompile_roundtrip() {
    let patterns = ["a", "abc", "[abc]", "a*", "a+", "a|b"];
    for pat in patterns.iter() {
        let nfa = ltre_parse(pat).unwrap();
        let dfa = ltre_compile(nfa);
        let nfa2 = ltre_uncompile(&dfa);
        let dfa2 = ltre_compile(nfa2);
        for inp in ["", "a", "ab", "abc", "aab", "b", "x"].iter() {
            let m1 = ltre_matches(&dfa, inp.as_bytes());
            let m2 = ltre_matches(&dfa2, inp.as_bytes());
            assert_eq!(m1, m2, "/{}/ vs '{}'", pat, inp);
        }
    }
}

#[test]
fn test_decompile_basic() {
    // Verified C outputs:
    // a -> "a"
    let dfa = compile_re("a");
    assert_eq!(ltre_decompile(&dfa), "a");
}

#[test]
fn test_decompile_empty_re() {
    // /[]/ -> "[]"
    let dfa = compile_re("[]");
    assert_eq!(ltre_decompile(&dfa), "[]");
}

#[test]
fn test_decompile_epsilon() {
    // // -> ""
    let dfa = compile_re("");
    assert_eq!(ltre_decompile(&dfa), "");
}

#[test]
fn test_decompile_charclass() {
    // [abc] -> "a-c" (since symset_fmt with nsym==1 returns range without brackets)
    let dfa = compile_re("[abc]");
    assert_eq!(ltre_decompile(&dfa), "a-c");
}

#[test]
fn test_decompile_star() {
    let dfa = compile_re("a*");
    assert_eq!(ltre_decompile(&dfa), "a*");
}

#[test]
fn test_decompile_plus() {
    let dfa = compile_re("a+");
    assert_eq!(ltre_decompile(&dfa), "a+");
}

#[test]
fn test_decompile_optional() {
    let dfa = compile_re("a?");
    assert_eq!(ltre_decompile(&dfa), "a?");
}

#[test]
fn test_decompile_alternation_chars() {
    // a|b -> "[ab]"
    let dfa = compile_re("a|b");
    assert_eq!(ltre_decompile(&dfa), "[ab]");
}

#[test]
fn test_decompile_dot() {
    // . -> "^\x0a"
    let dfa = compile_re(".");
    assert_eq!(ltre_decompile(&dfa), "^\\x0a");
}

#[test]
fn test_decompile_reparse_roundtrip() {
    let patterns = [
        "a", "abc", "[abc]", "a*", "a+", "a?", "a|b",
        "(ab|cd)+", "(a+b)*c", "[0-9]+",
    ];
    for pat in patterns.iter() {
        let nfa = ltre_parse(pat).unwrap();
        let dfa = ltre_compile(nfa);
        let decompiled = ltre_decompile(&dfa);
        let nfa2 = ltre_parse(&decompiled).unwrap_or_else(|e| panic!("re-parse {} -> {} failed: {}", pat, decompiled, e));
        let dfa2 = ltre_compile(nfa2);
        for inp in ["", "a", "ab", "abc", "abcd", "5", "12", "x"].iter() {
            let m1 = ltre_matches(&dfa, inp.as_bytes());
            let m2 = ltre_matches(&dfa2, inp.as_bytes());
            assert_eq!(m1, m2, "/{}/ -> /{}/ vs '{}'", pat, decompiled, inp);
        }
    }
}

// =========================================================================
// ltre_matches_lazy tests
// =========================================================================
#[test]
fn test_matches_lazy_basic() {
    let nfa = ltre_parse("abc").unwrap();
    let mut cache: Option<Dfa> = None;
    assert!(ltre_matches_lazy(&mut cache, &nfa, b"abc"));
    assert!(!ltre_matches_lazy(&mut cache, &nfa, b""));
    assert!(!ltre_matches_lazy(&mut cache, &nfa, b"ab"));
    assert!(!ltre_matches_lazy(&mut cache, &nfa, b"abcd"));
}

#[test]
fn test_matches_lazy_star() {
    let nfa = ltre_parse("a*").unwrap();
    let mut cache: Option<Dfa> = None;
    assert!(ltre_matches_lazy(&mut cache, &nfa, b""));
    assert!(ltre_matches_lazy(&mut cache, &nfa, b"a"));
    assert!(ltre_matches_lazy(&mut cache, &nfa, b"aaaa"));
    assert!(!ltre_matches_lazy(&mut cache, &nfa, b"b"));
}

#[test]
fn test_matches_lazy_complemented() {
    let mut nfa = ltre_parse("a").unwrap();
    ltre_complement(&mut nfa);
    let mut cache: Option<Dfa> = None;
    assert!(!ltre_matches_lazy(&mut cache, &nfa, b"a"));
    assert!(ltre_matches_lazy(&mut cache, &nfa, b"b"));
    assert!(ltre_matches_lazy(&mut cache, &nfa, b""));
    assert!(ltre_matches_lazy(&mut cache, &nfa, b"aa"));
}

// =========================================================================
// nfa_free / dfa_free tests (no-op, but should not panic)
// =========================================================================
#[test]
fn test_nfa_free_dfa_free() {
    let nfa = ltre_parse("a").unwrap();
    let dfa = ltre_compile(nfa.clone());
    dfa_free(dfa);
    nfa_free(nfa);
}

// =========================================================================
// Advanced regex tests (covering more parser features)
// =========================================================================
#[test]
fn test_match_charclass_features() {
    assert!(matches_re("^a-z*", "1A!2$B"));
    assert!(!matches_re("^a-z*", "1aA"));
    assert!(matches_re("a-z*", "abc"));
    assert!(matches_re("[[abc]]+", "abc"));
    assert!(matches_re("[a[bc]]+", "abc"));
    assert!(matches_re("[a[b]c]+", "abc"));
    assert!(matches_re("[a][b][c]", "abc"));
    assert!(!matches_re("^[^a^b]", "a"));
    assert!(!matches_re("^[^a^b]", "b"));
    assert!(!matches_re("^[^a^b]", ""));
    assert!(!matches_re("<ab>", "a"));
    assert!(!matches_re("<ab>", "b"));
    assert!(!matches_re("<ab>", ""));
    assert!(matches_re("\\^", "^"));
    assert!(!matches_re("^\\^", "^"));
    assert!(matches_re("^[^\\^]", "^"));
    assert!(!matches_re("[]", " "));
    assert!(matches_re("^[]", " "));
    assert!(matches_re("<>", " "));
    assert!(!matches_re("^<>", " "));
}

#[test]
fn test_match_complement_term() {
    assert!(!matches_re("~0*", ""));
    assert!(!matches_re("~0*", "0"));
    assert!(!matches_re("~0*", "00"));
    assert!(matches_re("~0*", "001"));
}

#[test]
fn test_match_intersection_complex() {
    assert!(!matches_re("0x(~[0-9a-f]+)", "0yz"));
    assert!(!matches_re("0x(~[0-9a-f]+)", "0x12"));
    assert!(matches_re("0x(~[0-9a-f]+)", "0x"));
    assert!(matches_re("0x(~[0-9a-f]+)", "0xy"));
}

#[test]
fn test_decompile_edge_cases() {
    // From C tests:
    assert!(matches_re("^aa*", "ba"));
    assert!(!matches_re("a-zz*", "abc"));
    assert!(matches_re("\\x0a(0a)*", "\x0a"));
    assert!(!matches_re("\\x0aa*", "\x0a\x0a"));
}

fn main() {}
