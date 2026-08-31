//! Shared pattern / subject corpora and a randomized *valid*-pattern generator.
//!
//! Used by every Phase-B test file so that the same broad set of pattern shapes
//! is exercised under every option combination.
#![allow(dead_code)]

use super::Rng;

/// Hand-written patterns covering every construct the compiler special-cases.
/// Derived from the `switch` arms in `pcre2_compile.c` / `pcre2_compile_class.c`
/// / `pcre2_compile_cgroup.c` and the opcode list in `pcre2_internal.h`.
pub const PATTERNS: &[&str] = &[
    // literals & empty
    "",
    "a",
    "abc",
    "a\0b",
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    // dot / anchors
    ".",
    "..",
    "^a",
    "a$",
    "^a$",
    "^",
    "$",
    r"\A",
    r"\Z",
    r"\z",
    r"\G",
    r"\b",
    r"\B",
    r"\K",
    r"a\Kb",
    // simple classes
    "[a]",
    "[abc]",
    "[^abc]",
    "[a-z]",
    "[^a-z]",
    "[a-zA-Z0-9_]",
    "[]]",
    "[^]]",
    "[-a]",
    "[a-]",
    r"[\]]",
    r"[\\]",
    r"[\d]",
    r"[\D]",
    r"[\w\W]",
    r"[\s\S]",
    r"[\h\H]",
    r"[\v\V]",
    "[[:alpha:]]",
    "[[:^alpha:]]",
    "[[:digit:][:space:]]",
    "[[:alnum:][:punct:]]",
    "[[:ascii:]]",
    "[[:blank:]]",
    "[[:cntrl:]]",
    "[[:graph:]]",
    "[[:lower:]]",
    "[[:print:]]",
    "[[:upper:]]",
    "[[:word:]]",
    "[[:xdigit:]]",
    // escapes
    r"\d",
    r"\D",
    r"\w",
    r"\W",
    r"\s",
    r"\S",
    r"\h",
    r"\H",
    r"\v",
    r"\V",
    r"\R",
    r"\X",
    r"\C",
    r"\N",
    r"\n",
    r"\r",
    r"\t",
    r"\f",
    r"\a",
    r"\e",
    r"\0",
    r"\07",
    r"\101",
    r"\x41",
    r"\x{41}",
    r"\x{1F600}",
    r"\o{101}",
    r"\cA",
    r"\Q a+b \E",
    r"a\Qb*c\Ed",
    r"\Qabc",
    // quantifiers
    "a*",
    "a+",
    "a?",
    "a{2}",
    "a{2,}",
    "a{2,4}",
    "a{0,3}",
    "a{0}",
    "a{1}",
    "a*?",
    "a+?",
    "a??",
    "a{2,4}?",
    "a*+",
    "a++",
    "a?+",
    "a{2,4}+",
    ".*",
    ".+",
    ".*?",
    "[a-z]*",
    "[a-z]+?",
    "[a-z]{3,5}",
    r"\d{1,3}",
    r"(?:ab)*",
    r"(ab)+",
    r"(a|b)*",
    // alternation
    "a|b",
    "a|b|c",
    "|a",
    "a|",
    "(a|b|)",
    "^a|b$",
    // groups
    "(a)",
    "(a)(b)",
    "((a))",
    "(?:a)",
    "(?i)a",
    "(?i:a)",
    "(?-i:a)",
    "(?s)a.b",
    "(?m)^a$",
    "(?x) a  b ",
    "(?xx) a  b ",
    "(?U)a*",
    "(?J)(?<n>a)(?<n>b)",
    "(?n)(a)",
    "(?>a+)b",
    "(?<n>a)",
    "(?'n'a)",
    "(?P<n>a)",
    "(?<n>a)(?&n)",
    "(?P>n)",
    // backreferences
    r"(a)\1",
    r"(a)(b)\2\1",
    r"(?<n>a)\k<n>",
    r"(?<n>a)\k'n'",
    r"(?<n>a)\k{n}",
    r"(?<n>a)(?P=n)",
    r"(a)\g1",
    r"(a)\g{1}",
    r"(a)\g{-1}",
    r"(a)(b)\g{-2}",
    // recursion / subroutines
    r"(a(?R)?b)",
    r"(a)(?1)",
    r"(?1)(a)",
    r"(?<n>a)(?&n)",
    r"\((?>[^()]|(?R))*\)",
    r"(?(DEFINE)(?<x>a))(?&x)",
    // lookaround
    "(?=a)",
    "(?!a)",
    "(?<=a)b",
    "(?<!a)b",
    "a(?=b)",
    "a(?!b)",
    "(?<=ab)c",
    "(?<=a|bc)d",
    "(*positive_lookahead:a)",
    "(*pla:a)",
    "(*negative_lookahead:a)",
    "(*nla:a)",
    "(*positive_lookbehind:a)b",
    "(*plb:a)b",
    "(*negative_lookbehind:a)b",
    "(*nlb:a)b",
    "(*atomic:a+)b",
    "(*asr:a)",
    "(*script_run:abc)",
    "(*sr:abc)",
    // non-atomic lookaround
    "(*napla:a)",
    "(*naplb:a)b",
    // conditionals
    "(a)?(?(1)b|c)",
    "(a)?(?(1)b)",
    "(?(?=a)b|c)",
    "(?(?!a)b|c)",
    "(?<n>a)?(?(<n>)b|c)",
    "(?(R)a|b)",
    "(?(R1)a|b)",
    "(?(DEFINE)(?<x>a))b",
    "(?(VERSION>=10.0)a|b)",
    "(?(VERSION=10.48)a|b)",
    // verbs
    "(*FAIL)",
    "(*F)",
    "a(*ACCEPT)b",
    "(*COMMIT)a",
    "(*PRUNE)a",
    "(*PRUNE:x)a",
    "(*SKIP)a",
    "(*SKIP:x)a",
    "(*THEN)a",
    "(*THEN:x)a",
    "(*MARK:x)a",
    "(*:x)a",
    "a(*MARK:m1)b|c(*MARK:m2)d",
    // options embedded at the start
    "(*UTF)a",
    "(*UCP)a",
    "(*CR)a.b",
    "(*LF)a.b",
    "(*CRLF)a.b",
    "(*ANY)a.b",
    "(*ANYCRLF)a.b",
    "(*NUL)a.b",
    "(*BSR_UNICODE)a\\Rb",
    "(*BSR_ANYCRLF)a\\Rb",
    "(*LIMIT_MATCH=100)a",
    "(*LIMIT_DEPTH=100)a",
    "(*LIMIT_HEAP=1000)a",
    "(*NO_AUTO_POSSESS)a+b",
    "(*NO_START_OPT)ab",
    "(*NO_DOTSTAR_ANCHOR).*a",
    "(*NO_JIT)a",
    "(*NOTEMPTY)a*",
    "(*NOTEMPTY_ATSTART)a*",
    // comments and callouts
    "a(?#comment)b",
    "(?C)a",
    "(?C1)a",
    "(?C255)a",
    "(?C{text})a",
    "(?C`text`)a",
    "(?C'text')a",
    "(?C\"text\")a",
    "(?C^text^)a",
    "(?C%text%)a",
    "(?C#text#)a",
    "(?C$text$)a",
    // unicode properties
    r"\p{L}",
    r"\P{L}",
    r"\p{Lu}",
    r"\p{Ll}",
    r"\p{Nd}",
    r"\p{Greek}",
    r"\p{Latin}",
    r"\p{Cyrillic}",
    r"\p{Han}",
    r"\p{Arabic}",
    r"\p{Any}",
    r"\p{Xan}",
    r"\p{Xps}",
    r"\p{Xsp}",
    r"\p{Xuc}",
    r"\p{Xwd}",
    r"\p{Alphabetic}",
    r"\p{White_Space}",
    r"\p{Bidi_Control}",
    r"\p{scx:Greek}",
    r"\p{sc=Latin}",
    r"\p{gc=Lu}",
    r"\pL",
    r"\PL",
    r"[\p{L}\p{N}]",
    r"[^\p{L}]",
    // combining
    "a.*b",
    ".*.*",
    "(a+)+b",
    "(a|aa)+b",
    "^(a+)*$",
    r"\d+\.\d+",
    r"^[\w.]+@[\w.]+$",
    r"(?i)^(?:https?|ftp)://\S+$",
    r"(\w+)\s+\1",
    "[0-9]{3}-[0-9]{4}",
    r"(?<year>\d{4})-(?<mon>\d{2})-(?<day>\d{2})",
    "a(?:b(?:c(?:d(?:e)?)?)?)?",
    "((((((((((a))))))))))",
    "(?:(?:(?:(?:(?:a)))))",
    "[a-c][d-f][g-i]",
    "x{0,10}y{0,10}z{0,10}",
    // wide / multibyte literals
    "é",
    "日本語",
    "😀",
    "[é-ü]",
    "[\u{4e00}-\u{9fff}]",
    "\u{85}",
    "\u{2028}",
];

/// Patterns valid only with `PCRE2_ALT_EXTENDED_CLASS`.
pub const ECLASS_PATTERNS: &[&str] = &[
    "[[a][b]]",
    "[[a-z]&&[^aeiou]]",
    "[[a-z]--[aeiou]]",
    "[[a-m]||[n-z]]",
    "[[a-z]~~[aeiou]]",
    "[[[a-z]&&[b-y]]--[m]]",
    "[\\p{L}&&[a-z]]",
    "[[a] [b] [c]]",
    "[^[a]&&[b-z]]",
];

/// Patterns valid only with `PCRE2_ALLOW_EMPTY_CLASS`.
pub const EMPTY_CLASS_PATTERNS: &[&str] = &["[]", "[^]", "[]a", "a[]b", "[^]*"];

/// Subjects, chosen to hit boundary shapes and both ASCII and UTF-8.
pub const SUBJECTS: &[&str] = &[
    "",
    "a",
    "b",
    "ab",
    "abc",
    "abcabc",
    "aaa",
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "A",
    "ABC",
    "0",
    "123",
    "a1b2c3",
    " ",
    "\t",
    "\n",
    "\r",
    "\r\n",
    "\n\r",
    "a\nb",
    "a\r\nb",
    "a\rb",
    "a\u{85}b",
    "a\u{2028}b",
    "a\u{2029}b",
    "a\u{b}b",
    "a\u{c}b",
    "\0",
    "a\0b",
    "_",
    "a_b",
    "a-b",
    ".",
    "a.b",
    "()",
    "(a)",
    "((a))",
    "a@b.com",
    "2024-01-31",
    "555-1234",
    "https://example.com/x",
    "hello world hello",
    "the the",
    "é",
    "éé",
    "日本語テキスト",
    "😀😀",
    "aéb",
    "\u{10000}",
    "\u{10FFFF}",
    "aeiou",
    "xyz",
    "abcdefghijklmnopqrstuvwxyz",
    "ABCDEFGHIJKLMNOPQRSTUVWXYZ",
    "0123456789",
    "!@#$%^&*()",
    "  leading and trailing  ",
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaab",
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaac",
];

/// Byte-level subjects, including sequences that are not valid UTF-8.
pub fn byte_subjects() -> Vec<Vec<u8>> {
    let mut v: Vec<Vec<u8>> = SUBJECTS.iter().map(|s| s.as_bytes().to_vec()).collect();
    v.extend([
        vec![0x80],
        vec![0xFF],
        vec![0xC2],
        vec![0xC2, 0x41],
        vec![0xE0, 0xA0],
        vec![0xF0, 0x9F, 0x98],
        vec![0xED, 0xA0, 0x80],
        vec![0xF4, 0x90, 0x80, 0x80],
        vec![b'a', 0x80, b'b'],
        vec![b'a', 0xC2, b'b'],
        vec![0xFE, 0xFF],
    ]);
    v
}

/// Randomized generator of syntactically valid patterns. Builds an expression
/// tree so that groups always balance, then renders it.
pub struct PatternGen {
    depth: u32,
    groups: u32,
    names: Vec<String>,
}

impl PatternGen {
    pub fn new() -> PatternGen {
        PatternGen {
            depth: 0,
            groups: 0,
            names: Vec::new(),
        }
    }

    pub fn gen(rng: &mut Rng) -> String {
        let mut g = PatternGen::new();
        let mut s = String::new();
        g.alt(rng, &mut s);
        s
    }

    fn alt(&mut self, rng: &mut Rng, out: &mut String) {
        let n = 1 + rng.below(3);
        for i in 0..n {
            if i > 0 {
                out.push('|');
            }
            self.seq(rng, out);
        }
    }

    fn seq(&mut self, rng: &mut Rng, out: &mut String) {
        let n = 1 + rng.below(4);
        for _ in 0..n {
            self.term(rng, out);
        }
    }

    fn term(&mut self, rng: &mut Rng, out: &mut String) {
        self.atom(rng, out);
        // maybe quantify
        if rng.below(3) == 0 {
            let q = *rng.pick(&["*", "+", "?", "{0,2}", "{1,3}", "{2}", "{1,}"]);
            out.push_str(q);
            match rng.below(4) {
                0 => out.push('?'),
                1 => out.push('+'),
                _ => {}
            }
        }
    }

    fn atom(&mut self, rng: &mut Rng, out: &mut String) {
        let deep = self.depth >= 4;
        let choice = if deep { rng.below(4) } else { rng.below(14) };
        match choice {
            0 => out.push(*rng.pick(&['a', 'b', 'c', 'x', 'y', 'z', '0', '1', '_', ' '])),
            1 => out.push_str(*rng.pick(&[
                r"\d", r"\D", r"\w", r"\W", r"\s", r"\S", r"\h", r"\v", r"\R", r"\X", r"\N",
            ])),
            2 => out.push('.'),
            3 => {
                let cls = *rng.pick(&[
                    "[abc]",
                    "[^abc]",
                    "[a-z]",
                    "[0-9a-fA-F]",
                    r"[\d\s]",
                    "[[:alpha:]]",
                    "[[:^digit:]]",
                    r"[\p{L}]",
                    r"[^\p{Nd}]",
                    "[-a-z]",
                    r"[\x41-\x5A]",
                ]);
                out.push_str(cls);
            }
            4 => {
                out.push_str(*rng.pick(&[r"\b", r"\B", r"\A", r"\Z", r"\z", "^", "$"]));
            }
            5 => {
                out.push_str(*rng.pick(&[r"\p{L}", r"\P{L}", r"\p{Nd}", r"\p{Greek}", r"\pL"]));
            }
            6 => {
                // capturing group
                self.depth += 1;
                self.groups += 1;
                out.push('(');
                self.alt(rng, out);
                out.push(')');
                self.depth -= 1;
            }
            7 => {
                // non-capturing / atomic / option group
                self.depth += 1;
                let open = *rng.pick(&["(?:", "(?>", "(?i:", "(?-i:", "(?s:", "(?m:", "(?x:"]);
                out.push_str(open);
                self.alt(rng, out);
                out.push(')');
                self.depth -= 1;
            }
            8 => {
                // named group
                self.depth += 1;
                self.groups += 1;
                let name = format!("g{}", self.names.len());
                self.names.push(name.clone());
                out.push_str("(?<");
                out.push_str(&name);
                out.push('>');
                self.alt(rng, out);
                out.push(')');
                self.depth -= 1;
            }
            9 => {
                // lookaround (fixed-length body for lookbehind)
                self.depth += 1;
                match rng.below(4) {
                    0 => {
                        out.push_str("(?=");
                        self.alt(rng, out);
                        out.push(')');
                    }
                    1 => {
                        out.push_str("(?!");
                        self.alt(rng, out);
                        out.push(')');
                    }
                    2 => {
                        out.push_str("(?<=");
                        out.push_str(*rng.pick(&["a", "ab", "abc", "[a-z]", r"\d\d"]));
                        out.push(')');
                    }
                    _ => {
                        out.push_str("(?<!");
                        out.push_str(*rng.pick(&["a", "ab", "abc", "[a-z]", r"\d\d"]));
                        out.push(')');
                    }
                }
                self.depth -= 1;
            }
            10 => {
                // back reference to an existing group, if any
                if self.groups > 0 {
                    let g = 1 + rng.below(self.groups as usize);
                    out.push_str(&format!(r"\{}", g));
                } else {
                    out.push('a');
                }
            }
            11 => {
                // reference to an existing name, if any
                if !self.names.is_empty() {
                    let n = rng.below(self.names.len());
                    out.push_str(&format!(r"\k<{}>", self.names[n]));
                } else {
                    out.push('b');
                }
            }
            12 => {
                // verbs and marks
                out.push_str(*rng.pick(&[
                    "(*FAIL)",
                    "(*ACCEPT)",
                    "(*COMMIT)",
                    "(*PRUNE)",
                    "(*SKIP)",
                    "(*THEN)",
                    "(*MARK:m)",
                ]));
            }
            _ => {
                // literal escape / hex / octal
                out.push_str(*rng.pick(&[
                    r"\x41", r"\x{42}", r"\101", r"\o{103}", r"\cA", r"\t", r"\n", r"\Qa+b\E",
                    r"\-", r"\.", r"\*",
                ]));
            }
        }
    }
}

/// Random byte subject with a configurable shape.
pub fn gen_subject(rng: &mut Rng, utf: bool) -> Vec<u8> {
    let n = rng.below(24);
    if utf {
        let chars = [
            'a', 'b', 'c', 'x', 'y', 'z', '0', '1', ' ', '_', '\n', '\r', '\t', 'é', 'ü', '日',
            '本', '😀', '\u{85}', '\u{2028}', '\u{2029}', 'A', 'B', 'Z', '-', '.', '@',
        ];
        let s: String = (0..n).map(|_| *rng.pick(&chars)).collect();
        s.into_bytes()
    } else {
        (0..n)
            .map(|_| *rng.pick(&[b'a', b'b', b'c', b'x', b'0', b'1', b' ', b'\n', b'\r', b'\t', 0u8, 0x80, 0xFF, b'A', b'-', b'.']))
            .collect()
    }
}
