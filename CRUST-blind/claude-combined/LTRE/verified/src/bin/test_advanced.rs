use LTRE::ltre::*;

fn matches(regex: &str, input: &str) -> bool {
    let nfa = ltre_parse(regex).expect("parse failed");
    let dfa = ltre_compile(nfa);
    ltre_matches(&dfa, input.as_bytes())
}

#[test]
fn test_serialize_roundtrip() {
    let nfa = ltre_parse("a*b+c?").expect("parse failed");
    let dfa = ltre_compile(nfa);
    let buf = dfa_serialize(&dfa);
    let (dfa2, sz) = dfa_deserialize(&buf).expect("deserialize failed");
    assert_eq!(sz, buf.len());

    // Verify behavioral equivalence
    let test_inputs = ["", "abc", "b", "aab", "aabbb", "aac", "aabbbc", "ac", "aabc"];
    for input in &test_inputs {
        assert_eq!(
            ltre_matches(&dfa, input.as_bytes()),
            ltre_matches(&dfa2, input.as_bytes()),
            "mismatch on input '{}'",
            input
        );
    }
}

#[test]
fn test_serialize_basic() {
    // Simplest possible DFA
    let nfa = ltre_parse("").expect("parse failed");
    let dfa = ltre_compile(nfa);
    let buf = dfa_serialize(&dfa);
    let (dfa2, _) = dfa_deserialize(&buf).expect("deserialize failed");
    assert_eq!(dfa.states.len(), dfa2.states.len());
    assert_eq!(ltre_matches(&dfa2, b""), true);
    assert_eq!(ltre_matches(&dfa2, b"a"), false);
}

#[test]
fn test_uncompile() {
    let nfa = ltre_parse("a*b+").expect("parse failed");
    let dfa = ltre_compile(nfa);
    let nfa2 = ltre_uncompile(&dfa);
    let dfa2 = ltre_compile(nfa2);

    let test_inputs = ["", "a", "b", "ab", "aab", "abb", "aabb", "ba", "aba"];
    for input in &test_inputs {
        assert_eq!(
            ltre_matches(&dfa, input.as_bytes()),
            ltre_matches(&dfa2, input.as_bytes()),
            "mismatch on input '{}'",
            input
        );
    }
}

#[test]
fn test_decompile_roundtrip() {
    let regexes = [
        "a*b+", "(a|b)c", "ab*c", "[abc]+", "x+y*z",
    ];
    for r in &regexes {
        let nfa = ltre_parse(r).expect("parse failed");
        let dfa = ltre_compile(nfa);
        let regen = ltre_decompile(&dfa);
        // Reparse
        let nfa2 = ltre_parse(&regen).expect(&format!("reparse failed for {}", regen));
        let dfa2 = ltre_compile(nfa2);
        // Should match same things
        let inputs = ["", "a", "b", "c", "ab", "abc", "aabbcc", "x", "xyz", "yyz"];
        for input in &inputs {
            assert_eq!(
                ltre_matches(&dfa, input.as_bytes()),
                ltre_matches(&dfa2, input.as_bytes()),
                "regex '{}' decompiled to '{}' mismatch on input '{}'",
                r, regen, input
            );
        }
    }
}

#[test]
fn test_fixed_string() {
    let nfa = ltre_fixed_string("hello");
    let dfa = ltre_compile(nfa);
    assert_eq!(ltre_matches(&dfa, b"hello"), true);
    assert_eq!(ltre_matches(&dfa, b"helloo"), false);
    assert_eq!(ltre_matches(&dfa, b""), false);
    assert_eq!(ltre_matches(&dfa, b"hell"), false);

    // Test with empty string
    let nfa = ltre_fixed_string("");
    let dfa = ltre_compile(nfa);
    assert_eq!(ltre_matches(&dfa, b""), true);
    assert_eq!(ltre_matches(&dfa, b"a"), false);

    // Special chars
    let nfa = ltre_fixed_string(".*+?");
    let dfa = ltre_compile(nfa);
    assert_eq!(ltre_matches(&dfa, b".*+?"), true);
    assert_eq!(ltre_matches(&dfa, b"a"), false);
}

#[test]
fn test_symset_ops() {
    let mut s = SymSet::empty();
    assert!(s.is_empty());
    assert_eq!(s.contains(b'a'), false);

    s.insert(b'a');
    assert_eq!(s.contains(b'a'), true);
    assert_eq!(s.contains(b'b'), false);
    assert!(!s.is_empty());

    let f = SymSet::full();
    assert_eq!(f.contains(0), true);
    assert_eq!(f.contains(255), true);
    assert!(!f.is_empty());

    let mut s2 = SymSet::empty();
    s2.insert(b'b');
    s.union_with(&s2);
    assert_eq!(s.contains(b'a'), true);
    assert_eq!(s.contains(b'b'), true);

    let mut s3 = SymSet::empty();
    s3.insert(b'a');
    s.intersect_with(&s3);
    assert_eq!(s.contains(b'a'), true);
    assert_eq!(s.contains(b'b'), false);

    let mut s4 = SymSet::empty();
    s4.insert(b'a');
    s4.invert();
    assert_eq!(s4.contains(b'a'), false);
    assert_eq!(s4.contains(b'b'), true);
}

#[test]
fn test_symset_fmt_basic() {
    // Verified against C: empty set yields complement form because nsym==0
    let empty = SymSet::empty();
    assert_eq!(symset_fmt(&empty), "^\\x00-\\xff");

    // Verified against C: full set has nnsym==0, returns "<>"
    let full = SymSet::full();
    assert_eq!(symset_fmt(&full), "<>");

    // Single char 'a'
    let mut a = SymSet::empty();
    a.insert(b'a');
    assert_eq!(symset_fmt(&a), "a");

    // 'a','b','c' contiguous range -> "a-c"
    let mut abc = SymSet::empty();
    abc.insert(b'a');
    abc.insert(b'b');
    abc.insert(b'c');
    assert_eq!(symset_fmt(&abc), "a-c");

    // Digits 0-9 contiguous range -> "0-9"
    let mut digits = SymSet::empty();
    for c in b'0'..=b'9' {
        digits.insert(c);
    }
    assert_eq!(symset_fmt(&digits), "0-9");

    // Two non-adjacent chars 'a' and 'd' -> "[ad]"
    let mut ad = SymSet::empty();
    ad.insert(b'a');
    ad.insert(b'd');
    assert_eq!(symset_fmt(&ad), "[ad]");
}

#[test]
fn test_realistic_hex_rgb() {
    let r = "#([0-9a-fA-F]{3}){1,2}";
    assert_eq!(matches(r, "000"), false);
    assert_eq!(matches(r, "#0aA"), true);
    assert_eq!(matches(r, "#00ff"), false);
    assert_eq!(matches(r, "#abcdef"), true);
    assert_eq!(matches(r, "#abcdeff"), false);
}

#[test]
fn test_nfa_concat_basic() {
    // Build "ab" by parsing each, then concatenating
    let a = ltre_fixed_string("a");
    let b = ltre_fixed_string("b");
    let mut combined = a;
    nfa_concat(&mut combined, b);
    let dfa = ltre_compile(combined);
    assert_eq!(ltre_matches(&dfa, b"ab"), true);
    assert_eq!(ltre_matches(&dfa, b"a"), false);
    assert_eq!(ltre_matches(&dfa, b"b"), false);
    assert_eq!(ltre_matches(&dfa, b"abc"), false);
}

#[test]
fn test_nfa_clone() {
    let n = ltre_parse("a+b").expect("parse");
    let cl = nfa_clone(&n);
    let dfa1 = ltre_compile(n);
    let dfa2 = ltre_compile(cl);
    assert_eq!(ltre_matches(&dfa1, b"ab"), true);
    assert_eq!(ltre_matches(&dfa2, b"ab"), true);
    assert_eq!(ltre_matches(&dfa1, b"aab"), true);
    assert_eq!(ltre_matches(&dfa2, b"aab"), true);
    assert_eq!(ltre_matches(&dfa1, b"b"), false);
    assert_eq!(ltre_matches(&dfa2, b"b"), false);
}

#[test]
fn test_dfa_new_len() {
    let d = Dfa::new();
    assert_eq!(d.len(), 0);

    let n = ltre_parse("abc").expect("parse");
    assert_eq!(n.len(), 4);
    let dfa = ltre_compile(n);
    assert!(dfa.len() > 0);
}

#[test]
fn test_nfa_new_single_len() {
    let n = Nfa::new_single();
    assert_eq!(n.len(), 1);
    assert_eq!(n.initial, 0);
    assert_eq!(n.final_, 0);
    assert!(!n.complemented);
}

#[test]
fn test_complement_via_lazy() {
    let mut nfa = ltre_parse("a*").expect("parse");
    ltre_complement(&mut nfa);
    let mut dfa: Option<Dfa> = None;
    assert_eq!(ltre_matches_lazy(&mut dfa, &nfa, b""), false);
    assert_eq!(ltre_matches_lazy(&mut dfa, &nfa, b"a"), false);
    assert_eq!(ltre_matches_lazy(&mut dfa, &nfa, b"ab"), true);
    assert_eq!(ltre_matches_lazy(&mut dfa, &nfa, b"b"), true);
}

fn main() {}
