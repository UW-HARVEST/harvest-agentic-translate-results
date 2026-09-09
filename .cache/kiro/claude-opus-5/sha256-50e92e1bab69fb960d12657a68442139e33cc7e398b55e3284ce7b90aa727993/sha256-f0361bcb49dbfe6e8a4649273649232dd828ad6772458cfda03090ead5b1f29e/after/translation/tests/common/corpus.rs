//! Pattern / subject corpora and a token-based random pattern generator.
#![allow(dead_code)]

use super::Rng;

/// Hand-written patterns covering the syntax the C compiler branches on.
/// (Both valid and invalid — the differential test does not care which.)
pub const PATTERNS: &[&str] = &[
    // --- trivial ---
    "", "a", "ab", "abc", "a*", "a+", "a?", "a{2}", "a{2,}", "a{2,4}", "a{0,}", "a{0}",
    "a{1}", "a{,3}", "a{3,2}", "a*+", "a++", "a?+", "a{2,4}+", "a*?", "a+?", "a??",
    "a{2,4}?", ".", ".*", ".+", ".*?", "^a", "a$", "^a$", "^", "$", "\\A", "\\Z", "\\z",
    "\\G", "\\b", "\\B", "\\K", "\\Q*+?\\E", "\\Qab", "\\E", "a|b", "a|b|c", "|", "||",
    "(a)", "(a|b)", "(a)(b)", "((a))", "(?:a)", "(?:a|b)", "(?>a)", "(?>a|b)",
    // --- classes ---
    "[a]", "[ab]", "[a-z]", "[^a-z]", "[]]", "[^]]", "[a-]", "[-a]", "[\\]]", "[\\\\]",
    "[[:alpha:]]", "[[:^alpha:]]", "[[:digit:][:space:]]", "[[:alnum:]]", "[[:ascii:]]",
    "[[:blank:]]", "[[:cntrl:]]", "[[:graph:]]", "[[:lower:]]", "[[:print:]]",
    "[[:punct:]]", "[[:upper:]]", "[[:word:]]", "[[:xdigit:]]", "[[:foo:]]", "[]", "[^]",
    "[a-\\d]", "[\\d-a]", "[\\w\\W]", "[\\s\\S]", "[\\h\\v]", "[\\H\\V]", "[\\Q-\\E]",
    "[z-a]", "[\\x{100}-\\x{200}]", "[^\\x{100}]", "[\\p{L}]", "[\\P{L}]", "[\\p{Latin}]",
    "[\\p{Any}]", "[[:alpha:]&&[a-c]]", "[[a-z]--[aeiou]]", "[[a-z]~~[aeiou]]",
    "[[a-z]&&[^aeiou]]", "[\\p{L}--\\p{Lu}]", "[[abc][def]]", "[a[bc]d]",
    // --- escapes ---
    "\\d", "\\D", "\\s", "\\S", "\\w", "\\W", "\\h", "\\H", "\\v", "\\V", "\\R", "\\X",
    "\\C", "\\N", "\\n", "\\r", "\\t", "\\f", "\\a", "\\e", "\\0", "\\00", "\\000",
    "\\07", "\\08", "\\1", "\\12", "\\123", "\\o{101}", "\\o{}", "\\o{8}", "\\x", "\\x41",
    "\\xg", "\\x{41}", "\\x{}", "\\x{110000}", "\\x{10ffff}", "\\x{d800}", "\\x{ffffffff}",
    "\\u0041", "\\u{41}", "\\N{U+0041}", "\\N{U+110000}", "\\N{FOO}", "\\g1", "\\g{1}",
    "\\g-1", "\\g{-1}", "\\g{+1}", "\\g<1>", "\\g'1'", "\\g{name}", "\\k<name>",
    "\\k'name'", "\\k{name}", "\\p{Lu}", "\\p{^Lu}", "\\P{Lu}", "\\p{Wombat}", "\\pL",
    "\\PL", "\\p{Greek}", "\\p{Bidi_Control}", "\\p{Xan}", "\\p{Xps}", "\\p{Xsp}",
    "\\p{Xuc}", "\\p{Xwd}", "\\q", "\\y", "\\_", "\\-", "\\ ", "\\\\", "\\",
    // --- groups, names, conditions ---
    "(?<n>a)", "(?'n'a)", "(?P<n>a)", "(?<n>a)(?<m>b)", "(?<n>a)(?<n>b)", "(?<>a)",
    "(?<1a>a)", "(?P=n)", "(?P>n)", "(?1)", "(?-1)", "(?+1)", "(?0)", "(?R)", "(?&n)",
    "(?(1)a|b)", "(?(1)a)", "(?(<n>)a|b)", "(?('n')a|b)", "(?(n)a|b)", "(?(R)a|b)",
    "(?(R1)a|b)", "(?(R&n)a|b)", "(?(DEFINE)(?<n>a))", "(?(VERSION>=10.0)a|b)",
    "(?(VERSION=10.48)a|b)", "(?(?=a)b|c)", "(?(?!a)b|c)", "(?(1)a|b|c)",
    "(?=a)", "(?!a)", "(?<=a)", "(?<!a)", "(?<=ab|cde)", "(?<=a*)", "(?<=a{2,4})",
    "(?<*a)", "(?*a)", "(?~a)", "(*atomic:a)", "(*script_run:abc)", "(*sr:abc)",
    "(*asr:abc)", "(*pla:a)", "(*plb:a)", "(*nla:a)", "(*nlb:a)", "(*positive_lookahead:a)",
    "(*napla:a)", "(*naplb:a)", "(?|(a)|(b))", "(?i)a", "(?-i)a", "(?i:a)", "(?-i:a)",
    "(?im)a", "(?imsx)a", "(?imsxJUn)a", "(?^i)a", "(?^)a", "(?x)a b", "(?xx)a b",
    "(?J)(?<n>a)(?<n>b)", "(?n)(a)", "(?i", "(?", "(?z)", "(?#comment)", "(?#", "a(?#c)b",
    // --- verbs ---
    "(*ACCEPT)", "(*FAIL)", "(*F)", "(*MARK:x)", "(*:x)", "(*COMMIT)", "(*PRUNE)",
    "(*PRUNE:x)", "(*SKIP)", "(*SKIP:x)", "(*THEN)", "(*THEN:x)", "(*ACCEPT:x)",
    "(*COMMIT:x)", "(*UNKNOWN)", "(*)", "(*MARK)", "(*ACCEPT)a",
    // --- start-of-pattern options ---
    "(*UTF)a", "(*UCP)a", "(*CR)a", "(*LF)a", "(*CRLF)a", "(*ANY)a", "(*ANYCRLF)a",
    "(*NUL)a", "(*BSR_UNICODE)\\R", "(*BSR_ANYCRLF)\\R", "(*LIMIT_MATCH=100)a",
    "(*LIMIT_DEPTH=100)a", "(*LIMIT_HEAP=100)a", "(*NO_START_OPT)a",
    "(*NO_AUTO_POSSESS)a+b", "(*NO_DOTSTAR_ANCHOR).*a", "(*NOTEMPTY)a*",
    "(*NOTEMPTY_ATSTART)a*", "(*NO_JIT)a", "(*UTF)(*UCP)(*CR)a", "(*LIMIT_MATCH=)a",
    "(*LIMIT_MATCH=abc)a",
    // --- callouts ---
    "(?C)a", "(?C1)a", "(?C255)a", "(?C256)a", "(?C`x`)a", "(?C'x')a", "(?C\"x\")a",
    "(?C^x^)a", "(?C%x%)a", "(?C#x#)a", "(?C$x$)a", "(?C{x})a", "(?C{x)a", "(?C1",
    // --- errors and edge cases ---
    "(", ")", "(((", ")))", "a)", "(a", "*", "+", "?", "{", "}", "{1}", "a**", "a++b",
    "[", "]", "a{1,2}{3}", "(?<=(?=a))", "(?=)", "()", "()*", "(){0}", "\\1(a)",
    "(a)\\2", "(?1)(a)", "a{65536}", "a{1,65536}", "a{2147483647}", "(?<=a+b)",
    "(?<=a{1,100})", "(?<=a{1,256})", "(?<=\\X)", "x(?#", "[[:al", "(?P", "(?P<", "(?P=",
    "(?P>", "\\p", "\\p{", "\\P{", "(?(", "(?(1", "(?(?", "(?(?=", "(*",
    // --- longer / realistic ---
    "^(?:[a-z0-9!#$%&'*+/=?^_`{|}~-]+(?:\\.[a-z0-9!#$%&'*+/=?^_`{|}~-]+)*)@(?:[a-z0-9](?:[a-z0-9-]*[a-z0-9])?\\.)+[a-z]{2,}$",
    "^(\\d{1,3})\\.(\\d{1,3})\\.(\\d{1,3})\\.(\\d{1,3})$",
    "(?<year>\\d{4})-(?<month>\\d{2})-(?<day>\\d{2})",
    "\\b(\\w+)\\s+\\1\\b",
    "(?s)(.*?)<a[^>]*href=\"([^\"]*)\"[^>]*>(.*?)</a>",
    "(a+)+b",
    "(?:(?:(?:(?:(?:(?:a)))))) ",
    "((((((((((a))))))))))",
    "(?:a|b|c|d|e|f|g|h|i|j|k|l|m|n|o|p|q|r|s|t|u|v|w|x|y|z)+",
    "[\\x00-\\xff]+",
    "\\p{Han}+\\p{Hiragana}*",
    "(?<A>a)(?<B>b)(?<C>c)(?<D>d)(?<E>e)(?<F>f)(?<G>g)(?<H>h)(?<I>i)(?<J>j)",
    "(?J)(?<n>a)|(?<n>b)|(?<n>c)",
    "(?(DEFINE)(?<word>\\w+))(?&word)\\s(?&word)",
    "a(?=b)(?<=a)c",
    "(?i)STRASSE",
    "\u{00df}",       // sharp s
    "\u{0130}",       // I with dot above (Turkish)
    "\u{0131}",       // dotless i
    "[\u{00e9}\u{00c9}]",
    "\u{1f600}",      // emoji
    "\\X\\X",
    "(*UTF)\\X+",
    "(*UTF)\\p{Greek}\\p{Latin}",
    "(?i)(?u)\u{03c3}\u{03c2}\u{03a3}",
];

/// Subjects for match / substitute rows.
pub const SUBJECTS: &[&str] = &[
    "",
    "a",
    "b",
    "ab",
    "abc",
    "aaa",
    "aaaa",
    "abcabc",
    "xyz",
    "AAA",
    "ABC",
    "a\nb",
    "a\rb",
    "a\r\nb",
    "a\u{0085}b",
    "a\u{2028}b",
    "a\u{2029}b",
    "\n",
    "\r\n",
    "  \t ",
    "0123456789",
    "a1b2c3",
    "hello world",
    "The quick brown fox",
    "127.0.0.1",
    "2024-11-05",
    "user@example.com",
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaab",
    "\u{00e9}\u{00c9}",
    "\u{00df}ss",
    "\u{0130}\u{0131}ii",
    "\u{03c3}\u{03c2}\u{03a3}",
    "\u{4e2d}\u{6587}",
    "\u{1f600}\u{1f601}",
    "a\u{0301}e\u{0302}",
    "\u{05d0}\u{05d1}",
    "\u{0627}\u{0628}",
    "abc\0def",
    "the the same same",
];

/// Raw byte subjects, including invalid UTF-8.
pub const RAW_SUBJECTS: &[&[u8]] = &[
    b"",
    b"\xff",
    b"\x80",
    b"\xc2",
    b"a\xc2b",
    b"\xed\xa0\x80",
    b"\xf4\x90\x80\x80",
    b"\xc0\x80",
    b"abc\xffdef",
    b"\x00\x01\x02\x03",
    b"\xfe\xfd\xfc",
    b"a\x80\x80b",
];

const TOK_ATOM: &[&str] = &[
    "a", "b", "c", "z", "0", "9", " ", ".", "\\d", "\\D", "\\w", "\\W", "\\s", "\\S",
    "\\h", "\\v", "\\R", "\\X", "\\N", "[a-c]", "[^a-c]", "[[:alpha:]]", "\\p{L}",
    "\\P{L}", "\\x41", "\\x{41}", "\\n", "\\t", "\\Qab\\E", "\u{00e9}", "\u{4e2d}",
    "\\C", "\\K", "\\b", "\\B", "^", "$", "\\A", "\\Z", "\\z", "\\G",
];
const TOK_QUANT: &[&str] = &[
    "", "*", "+", "?", "{2}", "{2,}", "{0,3}", "{1,4}", "*?", "+?", "??", "{2,4}?", "*+",
    "++", "?+", "{2,4}+",
];
const TOK_GROUP_OPEN: &[&str] = &[
    "(", "(?:", "(?>", "(?=", "(?!", "(?<=", "(?<!", "(?<n1>", "(?'n2'", "(?P<n3>", "(?i:",
    "(?-i:", "(?x:", "(?|", "(*atomic:", "(*sr:", "(*pla:", "(*nlb:",
];
const TOK_VERB: &[&str] = &[
    "(*ACCEPT)", "(*FAIL)", "(*MARK:m)", "(*COMMIT)", "(*PRUNE)", "(*SKIP)", "(*THEN)",
    "(?C1)", "(?C)", "(?#x)",
];

/// Random pattern built from the token pool; may be syntactically invalid, which
/// is exactly what we want (the C behaviour is the reference either way).
pub fn random_pattern(rng: &mut Rng) -> String {
    let n = 1 + rng.below(8);
    let mut s = String::new();
    let mut open = 0usize;
    for _ in 0..n {
        match rng.below(10) {
            0..=4 => {
                s.push_str(rng.pick(TOK_ATOM));
                s.push_str(rng.pick(TOK_QUANT));
            }
            5 | 6 => {
                s.push_str(rng.pick(TOK_GROUP_OPEN));
                open += 1;
                s.push_str(rng.pick(TOK_ATOM));
                if rng.bool() {
                    s.push('|');
                    s.push_str(rng.pick(TOK_ATOM));
                }
            }
            7 => {
                if open > 0 {
                    s.push(')');
                    open -= 1;
                    s.push_str(rng.pick(TOK_QUANT));
                } else {
                    s.push_str(rng.pick(TOK_ATOM));
                }
            }
            8 => s.push_str(rng.pick(TOK_VERB)),
            _ => s.push('|'),
        }
    }
    if rng.below(4) != 0 {
        for _ in 0..open {
            s.push(')');
        }
    }
    s
}

/// Random subject bytes: ASCII, Latin-1, valid UTF-8 or arbitrary bytes.
pub fn random_subject(rng: &mut Rng) -> Vec<u8> {
    let n = rng.below(24);
    let mode = rng.below(4);
    let mut v = Vec::with_capacity(n);
    for _ in 0..n {
        match mode {
            0 => v.push(b'a' + (rng.byte() % 6)),
            1 => v.push(rng.byte() & 0x7f),
            2 => {
                let cp = match rng.below(4) {
                    0 => rng.below(0x80) as u32,
                    1 => 0x80 + rng.below(0x780) as u32,
                    2 => 0x800 + rng.below(0xF800) as u32,
                    _ => 0x10000 + rng.below(0x100000) as u32,
                };
                let cp = if (0xD800..0xE000).contains(&cp) { 0x41 } else { cp };
                let ch = char::from_u32(cp).unwrap_or('A');
                let mut b = [0u8; 4];
                v.extend_from_slice(ch.encode_utf8(&mut b).as_bytes());
            }
            _ => v.push(rng.byte()),
        }
    }
    v
}
