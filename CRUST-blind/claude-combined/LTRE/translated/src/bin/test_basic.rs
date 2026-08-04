use LTRE::ltre::*;

fn matches(regex: &str, input: &str) -> bool {
    let nfa = ltre_parse(regex).expect("parse failed");
    let dfa = ltre_compile(nfa);
    ltre_matches(&dfa, input.as_bytes())
}

fn matches_partial(regex: &str, input: &str) -> bool {
    let mut nfa = ltre_parse(regex).expect("parse failed");
    ltre_partial(&mut nfa).unwrap();
    let dfa = ltre_compile(nfa);
    ltre_matches(&dfa, input.as_bytes())
}

fn matches_ic(regex: &str, input: &str) -> bool {
    let mut nfa = ltre_parse(regex).expect("parse failed");
    ltre_ignorecase(&mut nfa).unwrap();
    let dfa = ltre_compile(nfa);
    ltre_matches(&dfa, input.as_bytes())
}

fn matches_complement(regex: &str, input: &str) -> bool {
    let mut nfa = ltre_parse(regex).expect("parse failed");
    ltre_complement(&mut nfa);
    let dfa = ltre_compile(nfa);
    ltre_matches(&dfa, input.as_bytes())
}

fn matches_lazy(regex: &str, input: &str) -> bool {
    let nfa = ltre_parse(regex).expect("parse failed");
    let mut dfa: Option<Dfa> = None;
    ltre_matches_lazy(&mut dfa, &nfa, input.as_bytes())
}

#[test]
fn test_catastrophic_backtracking() {
    assert_eq!(matches("(a*)*c", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"), false);
    assert_eq!(matches("(x+x+)+y", "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"), false);
}

#[test]
fn test_blowout() {
    assert_eq!(matches("[01]*1[01]{8}", "11011100011100"), true);
    assert_eq!(matches("[01]*1[01]{8}", "01010010010010"), false);
    assert_eq!(matches(".*0.*|.*1.*|.*2.*|.*3.*|.*4.*|.*5.*", ""), false);
    assert_eq!(matches(".*0.*|.*1.*|.*2.*|.*3.*|.*4.*|.*5.*", "123"), true);
}

#[test]
fn test_potential_edge_cases() {
    assert_eq!(matches("abba", "abba"), true);
    assert_eq!(matches("[ab]+", "abba"), true);
    assert_eq!(matches("[ab]+", "abc"), false);
    assert_eq!(matches(".*", "abba"), true);
    assert_eq!(matches("(a|b+){3}", "abbba"), true);
    assert_eq!(matches("(a|b+){3}", "abbab"), false);
    assert_eq!(matches("\\x61\\+", "a+"), true);
    assert_eq!(matches("", ""), true);
    assert_eq!(matches("[]", ""), false);
    assert_eq!(matches("[]*", ""), true);
    assert_eq!(matches("[]+", ""), false);
    assert_eq!(matches("[]?", ""), true);
    assert_eq!(matches("()", ""), true);
    assert_eq!(matches("()*", ""), true);
    assert_eq!(matches("()+", ""), true);
    assert_eq!(matches("()?", ""), true);
    assert_eq!(matches(" ", " "), true);
    assert_eq!(matches("", "\n"), false);
    assert_eq!(matches("\\n", "\n"), true);
    assert_eq!(matches(".", "\n"), false);
    assert_eq!(matches("\\\\n", "\n"), false);
    assert_eq!(matches("(|n)(\\n)", "\n"), true);
    assert_eq!(matches("\\r?\\n", "\n"), true);
    assert_eq!(matches("\\r?\\n", "\r\n"), true);
}

#[test]
fn test_quantifiers() {
    assert_eq!(matches("(a*)*", "a"), true);
    assert_eq!(matches("(a+)+", "aa"), true);
    assert_eq!(matches("(a?)?", ""), true);
    assert_eq!(matches("a+", "aa"), true);
    assert_eq!(matches("a?", "aa"), false);
    assert_eq!(matches("(a+)?", "aa"), true);
    assert_eq!(matches("(ba+)?", "baa"), true);
    assert_eq!(matches("(ab+)?", "b"), false);
    assert_eq!(matches("(a+b)?", "a"), false);
    assert_eq!(matches("(a+a+)+", "a"), false);
    assert_eq!(matches("a+", ""), false);
    assert_eq!(matches("(a+|)+", "aa"), true);
    assert_eq!(matches("(a+|)+", ""), true);
    assert_eq!(matches("(a|b)?", "a"), true);
    assert_eq!(matches("(a|b)?", "b"), true);
}

#[test]
fn test_alternation() {
    assert_eq!(matches("x*|", "xx"), true);
    assert_eq!(matches("x*|", ""), true);
    assert_eq!(matches("x+|", "xx"), true);
    assert_eq!(matches("x+|", ""), true);
    assert_eq!(matches("x?|", "x"), true);
    assert_eq!(matches("x?|", ""), true);
    assert_eq!(matches("x*y*", "yx"), false);
    assert_eq!(matches("x+y+", "yx"), false);
    assert_eq!(matches("x?y?", "yx"), false);
    assert_eq!(matches("x+y*", "xyx"), false);
    assert_eq!(matches("x*y+", "yxy"), false);
    assert_eq!(matches("x*|y*", "xy"), false);
    assert_eq!(matches("x+|y+", "xy"), false);
    assert_eq!(matches("x?|y?", "xy"), false);
    assert_eq!(matches("x+|y*", "xy"), false);
    assert_eq!(matches("x*|y+", "xy"), false);
}

#[test]
fn test_bounds() {
    assert_eq!(matches("a{1,2}", ""), false);
    assert_eq!(matches("a{1,2}", "a"), true);
    assert_eq!(matches("a{1,2}", "aa"), true);
    assert_eq!(matches("a{1,2}", "aaa"), false);
    assert_eq!(matches("a{0,}", ""), true);
    assert_eq!(matches("a{0,}", "a"), true);
    assert_eq!(matches("a{0,}", "aa"), true);
    assert_eq!(matches("a{0,}", "aaa"), true);
    assert_eq!(matches("a{1,}", ""), false);
    assert_eq!(matches("a{1,}", "a"), true);
    assert_eq!(matches("a{1,}", "aa"), true);
    assert_eq!(matches("a{1,}", "aaa"), true);
    assert_eq!(matches("a{3,}", "aa"), false);
    assert_eq!(matches("a{3,}", "aaa"), true);
    assert_eq!(matches("a{3,}", "aaaa"), true);
    assert_eq!(matches("a{3,}", "aaaaa"), true);
    assert_eq!(matches("a{0,2}", ""), true);
    assert_eq!(matches("a{0,2}", "a"), true);
    assert_eq!(matches("a{0,2}", "aa"), true);
    assert_eq!(matches("a{0,2}", "aaa"), false);
    assert_eq!(matches("a{2}", "a"), false);
    assert_eq!(matches("a{2}", "aa"), true);
    assert_eq!(matches("a{2}", "aaa"), false);
    assert_eq!(matches("a{0}", ""), true);
    assert_eq!(matches("a{0}", "a"), false);
    assert_eq!(matches("a{,2}", ""), true);
    assert_eq!(matches("a{,2}", "a"), true);
    assert_eq!(matches("a{,2}", "aa"), true);
    assert_eq!(matches("a{,2}", "aaa"), false);
    assert_eq!(matches("a{}", ""), true);
    assert_eq!(matches("a{}", "a"), false);
    assert_eq!(matches("a{,}", ""), true);
    assert_eq!(matches("a{,}", "a"), true);
}

#[test]
fn test_partial_ic_complement() {
    assert_eq!(matches_partial("", ""), true);
    assert_eq!(matches_partial("", "abc"), true);
    assert_eq!(matches_partial("b", "abc"), true);
    assert_eq!(matches_partial("ba", "abc"), false);
    assert_eq!(matches_partial("abc", "abc"), true);
    assert_eq!(matches_partial("[]", ""), false);
    assert_eq!(matches_ic("", ""), true);
    assert_eq!(matches_ic("abCdEF", "aBCdEf"), true);
    assert_eq!(matches_ic("ab", "abc"), false);
    assert_eq!(matches_complement("a", ""), true);
    assert_eq!(matches_complement("a", "aa"), true);
    assert_eq!(matches_complement("a", "a"), false);
    assert_eq!(matches_complement("ab*", "ac"), true);
    assert_eq!(matches_complement("ab*", "abb"), false);
}

#[test]
fn test_decompilation_edge_cases() {
    assert_eq!(matches("^aa*", "ba"), true);
    assert_eq!(matches("a-zz*", "abc"), false);
    assert_eq!(matches("\\x0a(0a)*", "\x0a"), true);
    assert_eq!(matches("\\x0aa*", "\x0a\x0a"), false);
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
    let nat_ovf = "9999999999999999999999999999999999999999";
    assert!(ltre_parse(&format!("a{{{}}}", nat_ovf)).is_err());
    assert!(ltre_parse(&format!("a{{{},}}", nat_ovf)).is_err());
    assert!(ltre_parse(&format!("a{{,{}}}", nat_ovf)).is_err());
    assert!(ltre_parse(&format!("a{{{},{}}}", nat_ovf, nat_ovf)).is_err());
    assert!(ltre_parse("abc>").is_err());
    assert!(ltre_parse("<abc").is_err());
    assert!(ltre_parse("[a?b]").is_err());
    assert!(ltre_parse("[a-]").is_err());
    assert!(ltre_parse("[--]").is_err());
    assert!(ltre_parse("[-]").is_err());
    assert!(ltre_parse("-").is_err());
    assert!(ltre_parse("a-").is_err());
    assert!(ltre_parse("a*{}").is_err());
    assert!(ltre_parse("a+{}").is_err());
    assert!(ltre_parse("a?{}").is_err());
    assert!(ltre_parse("a{}*").is_err());
    assert!(ltre_parse("a{}+").is_err());
    assert!(ltre_parse("a{}?").is_err());
    assert!(ltre_parse("a{}{}").is_err());
    assert!(ltre_parse("a{2,1}").is_err());
    assert!(ltre_parse("a{1 2}").is_err());
    assert!(ltre_parse("a{1, 2}").is_err());
    assert!(ltre_parse("a{a}").is_err());
    assert!(ltre_parse("a~b").is_err());
}

#[test]
fn test_nonstandard_features() {
    assert_eq!(matches("^a", "z"), true);
    assert_eq!(matches("^a", "a"), false);
    assert_eq!(matches("^\\n", "\r"), true);
    assert_eq!(matches("^\\n", "\n"), false);
    assert_eq!(matches("^.", "\n"), true);
    assert_eq!(matches("^.", "a"), false);
    assert_eq!(matches("\\d+", "0123456789"), true);
    assert_eq!(matches("\\s+", " \x0c\n\r\t\x0b"), true);
    assert_eq!(matches("\\w+", "azAZ09_"), true);
    assert_eq!(matches("^a-z*", "1A!2$B"), true);
    assert_eq!(matches("^a-z*", "1aA"), false);
    assert_eq!(matches("a-z*", "abc"), true);
    assert_eq!(matches("^[\\d^\\w]+", "abcABC"), true);
    assert_eq!(matches("^[\\d^\\w]+", "abc123"), false);
    assert_eq!(matches("^[\\d\\W]+", "abcABC"), true);
    assert_eq!(matches("^[\\d^\\W]+", "abc123"), false);
    assert_eq!(matches("[[abc]]+", "abc"), true);
    assert_eq!(matches("[a[bc]]+", "abc"), true);
    assert_eq!(matches("[a[b]c]+", "abc"), true);
    assert_eq!(matches("[a][b][c]", "abc"), true);
    assert_eq!(matches("^[^a^b]", "a"), false);
    assert_eq!(matches("^[^a^b]", "b"), false);
    assert_eq!(matches("^[^a^b]", ""), false);
    assert_eq!(matches("<ab>", "a"), false);
    assert_eq!(matches("<ab>", "b"), false);
    assert_eq!(matches("<ab>", ""), false);
    assert_eq!(matches("\\^", "^"), true);
    assert_eq!(matches("^\\^", "^"), false);
    assert_eq!(matches("^[^\\^]", "^"), true);
    assert_eq!(matches("^[ ^[a b c]]+", "abc"), true);
    assert_eq!(matches("^[ ^[a b c]]+", "a c"), false);
    assert_eq!(matches("<[a b c]^ >+", "abc"), true);
    assert_eq!(matches("<[a b c]^ >+", "a c"), false);
    assert_eq!(matches("^[^0-74]+", "0123567"), true);
    assert_eq!(matches("^[^0-74]+", "89"), false);
    assert_eq!(matches("^[^0-74]+", "4"), false);
    assert_eq!(matches("<0-7^4>+", "0123567"), true);
    assert_eq!(matches("<0-7^4>+", "89"), false);
    assert_eq!(matches("<0-7^4>+", "4"), false);
    assert_eq!(matches("[]", " "), false);
    assert_eq!(matches("^[]", " "), true);
    assert_eq!(matches("<>", " "), true);
    assert_eq!(matches("^<>", " "), false);
    assert_eq!(matches("9-0*", "abc"), true);
    assert_eq!(matches("9-0*", "18"), false);
    assert_eq!(matches("9-0*", "09"), true);
    assert_eq!(matches("9-0*", "/:"), true);
    assert_eq!(matches("b-a*", "ab"), true);
    assert_eq!(matches("a-b*", "ab"), true);
    assert_eq!(matches("a-a*", "ab"), false);
    assert_eq!(matches("a-a*", "aa"), true);
    assert_eq!(matches("~0*", ""), false);
    assert_eq!(matches("~0*", "0"), false);
    assert_eq!(matches("~0*", "00"), false);
    assert_eq!(matches("~0*", "001"), true);
    assert_eq!(matches("ab&cd", ""), false);
    assert_eq!(matches("ab&cd", "ab"), false);
    assert_eq!(matches("ab&cd", "cd"), false);
    assert_eq!(matches("\\w+&~\\d+", ""), false);
    assert_eq!(matches("\\w+&~\\d+", "abc"), true);
    assert_eq!(matches("\\w+&~\\d+", "abc123"), true);
    assert_eq!(matches("\\w+&~\\d+", "1a2b3c"), true);
    assert_eq!(matches("\\w+&~\\d+", "123"), false);
    assert_eq!(matches("0x(~[0-9a-f]+)", "0yz"), false);
    assert_eq!(matches("0x(~[0-9a-f]+)", "0x12"), false);
    assert_eq!(matches("0x(~[0-9a-f]+)", "0x"), true);
    assert_eq!(matches("0x(~[0-9a-f]+)", "0xy"), true);
    assert_eq!(matches("0x(~[0-9a-f]+)", "0xyz"), true);
    assert_eq!(matches("b(~a*)", ""), false);
    assert_eq!(matches("b(~a*)", "b"), false);
    assert_eq!(matches("b(~a*)", "ba"), false);
    assert_eq!(matches("b(~a*)", "bbaa"), true);
}

#[test]
fn test_lazy_matches() {
    assert_eq!(matches_lazy("abba", "abba"), true);
    assert_eq!(matches_lazy("[ab]+", "abba"), true);
    assert_eq!(matches_lazy("[ab]+", "abc"), false);
    assert_eq!(matches_lazy("a+", "aa"), true);
    assert_eq!(matches_lazy("a+", ""), false);
}

fn main() {}
