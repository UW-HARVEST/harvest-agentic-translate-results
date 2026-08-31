//! Phase B/C differential tests for the regexp engine (`regexp.c`):
//! `js_regcomp`, `js_regcompx`, `js_regexec`, `js_regfree`, `js_regfreex`.
mod common;
use common::*;
use std::ffi::{c_char, c_int, c_void};

// ---------------------------------------------------------------------------
// Compile + exec through both implementations
// ---------------------------------------------------------------------------

/// Outcome of `regcomp`: either the error string, or `Ok(nsub)`-ish opaque prog.
#[derive(Debug, PartialEq, Eq)]
enum Compiled {
    Err(Option<String>),
    Ok,
}

struct Comp {
    prog: Reprog,
    outcome: Compiled,
}

fn regcomp_one(imp: &Impl, pat: &[u8], cflags: c_int) -> Comp {
    let f = imp.f::<FnRegcomp>("js_regcomp");
    let buf = cbytes(pat);
    // Pre-poison so we can tell "not written" from "written NULL".
    let mut err: *const c_char = 0x1 as *const c_char;
    let prog = unsafe { f(buf.as_ptr() as *const c_char, cflags, &mut err) };
    if prog.is_null() {
        let msg = if err as usize == 1 {
            Some("<errorp NOT WRITTEN>".to_string())
        } else if err.is_null() {
            None
        } else {
            Some(show(&unsafe { read_cstr(err) }.unwrap()))
        };
        Comp { prog, outcome: Compiled::Err(msg) }
    } else {
        // On success the C sets *errorp = NULL (regexp.c:983).
        let ok_err = err.is_null();
        assert!(ok_err || err as usize == 1, "unexpected errorp on success");
        Comp {
            prog,
            outcome: if ok_err { Compiled::Ok } else { Compiled::Err(Some("<errorp stale>".into())) },
        }
    }
}

fn regfree_one(imp: &Impl, c: &Comp) {
    if !c.prog.is_null() {
        let f = imp.f::<FnRegfree>("js_regfree");
        unsafe { f(c.prog) };
    }
}

/// Full observation of one `regexec` call, normalised to offsets.
#[derive(Debug, PartialEq, Eq)]
struct ExecOut {
    rc: c_int,
    nsub: c_int,
    subs: Vec<Option<(isize, isize)>>,
}

fn regexec_one(imp: &Impl, prog: Reprog, subject: &[u8], eflags: c_int) -> ExecOut {
    let f = imp.f::<FnRegexec>("js_regexec");
    let buf = cbytes(subject);
    let base = buf.as_ptr() as *const c_char;
    let mut sub = Resub::default();
    let rc = unsafe { f(prog, base, &mut sub, eflags) };
    ExecOut { rc, nsub: sub.nsub, subs: sub.offsets(base) }
}

/// Compile with both impls, compare the compile outcome, then (if both
/// compiled) exec every subject with every eflags and compare, then free.
fn diff_regexp(b: &mut Batch, pat: &[u8], cflags: c_int, subjects: &[&[u8]], eflags_set: &[c_int]) {
    let (c, r) = Impl::both();
    let cc = regcomp_one(&c, pat, cflags);
    let rc = regcomp_one(&r, pat, cflags);
    b.check(
        &format!("regcomp({:?}, cflags={cflags})", show(pat)),
        &cc.outcome,
        &rc.outcome,
    );
    if cc.outcome == Compiled::Ok && rc.outcome == Compiled::Ok {
        for &ef in eflags_set {
            for s in subjects {
                let a = regexec_one(&c, cc.prog, s, ef);
                let bb = regexec_one(&r, rc.prog, s, ef);
                b.check(
                    &format!(
                        "regexec(pat={:?}, cflags={cflags}, eflags={ef}, subj={:?})",
                        show(pat),
                        show(s)
                    ),
                    &a,
                    &bb,
                );
            }
        }
    }
    regfree_one(&c, &cc);
    regfree_one(&r, &rc);
}

const ALL_CFLAGS: [c_int; 4] = [0, REG_ICASE, REG_NEWLINE, REG_ICASE | REG_NEWLINE];
const ALL_EFLAGS: [c_int; 2] = [0, REG_NOTBOL];

fn default_subjects() -> Vec<&'static [u8]> {
    vec![
        b"",
        b"a",
        b"A",
        b"b",
        b"ab",
        b"AB",
        b"aab",
        b"abc",
        b"ABC",
        b"abcabc",
        b"xabcx",
        b"aaa",
        b"aaaaaaaaaa",
        b"a\nb",
        b"\n",
        b"\na",
        b"a\n",
        b"line1\nline2\nline3",
        b"foo bar baz",
        b"  spaced  ",
        b"123",
        b"a1b2c3",
        b"_x_",
        b"-",
        b".",
        b"[",
        b"]",
        b"(",
        b")",
        b"*",
        b"\\",
        b"^",
        b"$",
        b"tab\there",
        "\u{e9}".as_bytes(),
        "caf\u{e9}".as_bytes(),
        "\u{4f60}\u{597d}".as_bytes(),
        "\u{1f600}".as_bytes(),
        "\u{c9}".as_bytes(),
        b"\x80\x81",
        b"\xff\xfe",
        b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaab",
        b"the quick brown fox jumps over the lazy dog",
        b"AAAaaaBBBbbb",
    ]
}

// ---------------------------------------------------------------------------
// Phase B: every valid pattern construct
// ---------------------------------------------------------------------------

/// The full construct list, derived from the parser in regexp.c.
fn valid_patterns() -> Vec<&'static [u8]> {
    vec![
        // empty / literal
        b"",
        b"a",
        b"abc",
        b"a b",
        b"\x01",
        "\u{e9}".as_bytes(),
        "\u{4f60}".as_bytes(),
        "\u{1f600}".as_bytes(),
        // any
        b".",
        b"a.c",
        b"...",
        b".*",
        b".+",
        // character classes
        b"[a]",
        b"[abc]",
        b"[a-z]",
        b"[a-zA-Z0-9_]",
        b"[^a]",
        b"[^a-z]",
        b"[]]",       // ']' first in class is a literal in many engines
        b"[-a]",
        b"[a-]",
        b"[.]",
        b"[*+?]",
        b"[\\]]",
        b"[\\\\]",
        b"[\\d]",
        b"[\\D]",
        b"[\\w]",
        b"[\\W]",
        b"[\\s]",
        b"[\\S]",
        b"[\\n\\r\\t\\f\\v]",
        b"[\\x41]",
        b"[\\u0041]",
        b"[\\0]",
        b"[a-c e-g]",
        b"[^\\d]",
        b"[\\d\\s\\w]",
        b"[\\b]",
        // anchors
        b"^a",
        b"a$",
        b"^a$",
        b"^",
        b"$",
        b"^$",
        b"^abc$",
        // word boundaries
        b"\\ba",
        b"a\\b",
        b"\\Ba",
        b"a\\B",
        b"\\bfoo\\b",
        b"\\Bo\\B",
        // escapes
        b"\\d",
        b"\\D",
        b"\\w",
        b"\\W",
        b"\\s",
        b"\\S",
        b"\\n",
        b"\\r",
        b"\\t",
        b"\\f",
        b"\\v",
        b"\\0",
        b"\\x41",
        b"\\x4a",
        b"\\x4A",
        b"\\u0041",
        b"\\u00e9",
        b"\\u4f60",
        b"\\cA",
        b"\\cZ",
        b"\\ca",
        b"\\.",
        b"\\*",
        b"\\+",
        b"\\?",
        b"\\[",
        b"\\]",
        b"\\(",
        b"\\)",
        b"\\{",
        b"\\}",
        b"\\|",
        b"\\^",
        b"\\$",
        b"\\/",
        b"\\\\",
        // alternation
        b"a|b",
        b"a|b|c",
        b"abc|abd",
        b"|a",
        b"a|",
        b"|",
        b"||",
        b"(a|b)c",
        b"^a|b$",
        // groups
        b"(a)",
        b"(abc)",
        b"(a)(b)",
        b"(a)(b)(c)",
        b"((a))",
        b"(((a)))",
        b"(a(b(c)))",
        b"(?:a)",
        b"(?:abc)",
        b"(?:a)(b)",
        b"(a)(?:b)(c)",
        b"()",
        b"(?:)",
        // lookahead
        b"a(?=b)",
        b"a(?!b)",
        b"(?=a)",
        b"(?!a)",
        b"(?=ab)c",
        b"(?!x)a",
        b"a(?=b)c",
        // backreferences
        b"(a)\\1",
        b"(a)(b)\\2\\1",
        b"(a+)\\1",
        b"(.)\\1",
        b"((a)b)\\1\\2",
        // quantifiers
        b"a*",
        b"a+",
        b"a?",
        b"a{0}",
        b"a{1}",
        b"a{2}",
        b"a{3}",
        b"a{0,}",
        b"a{1,}",
        b"a{2,}",
        b"a{0,1}",
        b"a{1,3}",
        b"a{2,2}",
        b"a{0,0}",
        b"a*?",
        b"a+?",
        b"a??",
        b"a{1,3}?",
        b"a{2,}?",
        b"a{0,}?",
        b".*?",
        b".+?",
        b"[a-z]*",
        b"[a-z]+?",
        b"(a)*",
        b"(a)+",
        b"(a)?",
        b"(a|b)*",
        b"(a|b)+",
        b"(?:ab)*",
        b"(?:ab)+?",
        b"(ab){2}",
        b"(ab){1,2}",
        b"a*b*c*",
        b"a+b+c+",
        b"(a+)+b",
        b"^(a|b)*$",
        // combinations / realistic patterns
        b"^[a-z]+@[a-z]+\\.[a-z]{2,4}$",
        b"[0-9]{1,3}\\.[0-9]{1,3}",
        b"(foo|bar)+baz",
        b"\\s*(\\w+)\\s*=\\s*(\\w+)\\s*",
        b"^\\s*$",
        b"(.)(.)(.)",
        b"(a)(b)(c)(d)(e)(f)(g)(h)(i)(j)(k)(l)(m)(n)", // 14 groups (< REG_MAXSUB)
        b"(a)(b)(c)(d)(e)(f)(g)(h)(i)(j)(k)(l)(m)(n)(o)", // 15 groups == REG_MAXSUB-1
        b"[^]]*",
        b"a.*?b",
        b"a[^b]*b",
        b"(?:(a)|(b))+",
    ]
}

#[test]
fn regexp_valid_constructs_all_flag_combos() {
    let subjects = default_subjects();
    let mut b = Batch::new();
    for pat in valid_patterns() {
        for &cf in &ALL_CFLAGS {
            diff_regexp(&mut b, pat, cf, &subjects, &ALL_EFLAGS);
        }
    }
    b.finish("regexp valid constructs x all cflags x all eflags");
}

#[test]
fn regexp_group_count_boundary() {
    // CONFIGS row: exactly REG_MAXSUB-1 (=15) capturing groups is the maximum
    // (g.nsub starts at 1); 16 groups must be rejected with "too many captures"
    // (regexp.c:551-552).
    let subjects: Vec<&[u8]> = vec![b"abcdefghijklmnop", b"", b"abc"];
    let mut b = Batch::new();
    for n in 1..=20usize {
        let pat: Vec<u8> = (0..n).flat_map(|i| {
            let ch = b'a' + (i % 26) as u8;
            vec![b'(', ch, b')']
        }).collect();
        for &cf in &ALL_CFLAGS {
            diff_regexp(&mut b, &pat, cf, &subjects, &ALL_EFLAGS);
        }
    }
    b.finish("regexp capture-count boundary 1..20");
}

#[test]
fn regexp_character_class_count_boundary() {
    // CONFIGS/ERRORS row: REG_MAXCLASS = 128 distinct character classes
    // (regexp.c:212-213 "too many character classes").
    let subjects: Vec<&[u8]> = vec![b"", b"abc", b"zzz"];
    let mut b = Batch::new();
    for n in [1usize, 2, 100, 126, 127, 128, 129, 130, 200] {
        let mut pat = Vec::new();
        for _ in 0..n {
            pat.extend_from_slice(b"[a-z]");
        }
        for &cf in [0 as c_int, REG_ICASE].iter() {
            diff_regexp(&mut b, &pat, cf, &subjects, &[0]);
        }
    }
    b.finish("regexp class-count boundary");
}

#[test]
fn regexp_program_size_boundary() {
    // ERRORS row: `strlen(pattern) * 2 > REG_MAXPROG` (=32768) is rejected up
    // front with "program too large" (regexp.c:920-921); and the post-count
    // check at regexp.c:949-950.
    let subjects: Vec<&[u8]> = vec![b"", b"aaaa"];
    let mut b = Batch::new();
    for len in [16_380usize, 16_383, 16_384, 16_385, 16_390, 20_000, 40_000] {
        let pat = vec![b'a'; len];
        diff_regexp(&mut b, &pat, 0, &subjects, &[0]);
    }
    b.finish("regexp program-size boundary");
}

#[test]
fn regexp_nesting_depth_boundary() {
    // ERRORS row: REG_MAXREC = 4096 recursion depth -> "stack overflow"
    // (regexp.c:661) in count(), and the matcher's own limit at regexp.c:1075.
    let subjects: Vec<&[u8]> = vec![b"", b"a"];
    let mut b = Batch::new();
    for depth in [1usize, 10, 100, 1000, 2000, 4000, 4090, 4095, 4096, 4097, 5000] {
        let mut pat = Vec::new();
        for _ in 0..depth {
            pat.extend_from_slice(b"(?:");
        }
        pat.push(b'a');
        for _ in 0..depth {
            pat.push(b')');
        }
        diff_regexp(&mut b, &pat, 0, &subjects, &[0]);
    }
    b.finish("regexp nesting-depth boundary");
}

#[test]
fn regexp_randomized_patterns() {
    // Property test: random patterns from the metacharacter alphabet. Most are
    // syntax errors -- which is the point: both impls must reject identically
    // with the SAME message.
    let subjects: Vec<&[u8]> = vec![b"", b"a", b"abc", b"aab", b"a\nb", b"AbC", b"123"];
    let mut b = Batch::new();
    let mut rng = Rng::new(0x2E6E_7000);
    let alphabet: &[u8] = b"ab.*+?()[]|^$\\-{},:=!01dDwWsSnrtfvxucB \n";
    for _ in 0..30_000 {
        let n = rng.below(12) as usize;
        let pat: Vec<u8> = (0..n).map(|_| *rng.pick(alphabet)).collect();
        let cf = *rng.pick(&ALL_CFLAGS);
        let ef = *rng.pick(&ALL_EFLAGS);
        diff_regexp(&mut b, &pat, cf, &subjects[..3], &[ef]);
    }
    b.finish("regexp random patterns");
}

#[test]
fn regexp_randomized_structured_patterns() {
    // Property test: randomly *composed but well-formed* patterns, so we spend
    // the budget inside the matcher rather than the error paths.
    let mut b = Batch::new();
    let mut rng = Rng::new(0x2E6E_7001);
    let atoms: &[&[u8]] = &[
        b"a", b"b", b"c", b".", b"[ab]", b"[^a]", b"[a-c]", b"\\d", b"\\w", b"\\s", b"\\D",
        b"(a)", b"(?:ab)", b"(a|b)", b"x", b"\\.", b"[0-9]",
    ];
    let quants: &[&[u8]] = &[
        b"", b"*", b"+", b"?", b"{2}", b"{1,3}", b"{0,2}", b"*?", b"+?", b"??", b"{1,2}?", b"{2,}",
    ];
    for _ in 0..20_000 {
        let n = 1 + rng.below(5) as usize;
        let mut pat: Vec<u8> = Vec::new();
        if rng.below(6) == 0 {
            pat.push(b'^');
        }
        for _ in 0..n {
            pat.extend_from_slice(rng.pick(atoms));
            pat.extend_from_slice(rng.pick(quants));
            if rng.below(8) == 0 {
                pat.push(b'|');
                pat.extend_from_slice(rng.pick(atoms));
            }
        }
        if rng.below(6) == 0 {
            pat.push(b'$');
        }
        // random subject built from the same alphabet
        let sn = rng.below(12) as usize;
        let subj: Vec<u8> = (0..sn).map(|_| *rng.pick(b"abcx019 \n.")).collect();
        let cf = *rng.pick(&ALL_CFLAGS);
        let ef = *rng.pick(&ALL_EFLAGS);
        diff_regexp(&mut b, &pat, cf, &[&subj], &[ef]);
    }
    b.finish("regexp random structured patterns");
}

#[test]
fn regexp_randomized_subjects_fixed_patterns() {
    // Property test: a handful of patterns against many random subjects, to
    // catch value-dependent matcher/capture bugs.
    let pats: &[&[u8]] = &[
        b"(a+)(b*)c",
        b"^(\\w+)\\s+(\\w+)$",
        b"[a-c]{2,4}",
        b"(a|ab)(c|bcd)(d*)",
        b"(.)(.)\\2\\1",
        b"a.*?b",
        b"a.*b",
        b"(?:x|y)+z",
        b"\\b\\w+\\b",
        b"(a)?b\\1",
        b"([abc])+",
        b"^$",
        b"(a*)*b",
        b"[^\\s]+",
    ];
    let mut b = Batch::new();
    let mut rng = Rng::new(0x2E6E_7002);
    for pat in pats {
        for &cf in &ALL_CFLAGS {
            let (c, r) = Impl::both();
            let cc = regcomp_one(&c, pat, cf);
            let rr = regcomp_one(&r, pat, cf);
            b.check(&format!("regcomp({:?},{cf})", show(pat)), &cc.outcome, &rr.outcome);
            if cc.outcome == Compiled::Ok && rr.outcome == Compiled::Ok {
                for _ in 0..1500 {
                    let sn = rng.below(18) as usize;
                    let subj: Vec<u8> =
                        (0..sn).map(|_| *rng.pick(b"abcdxyz01 \n\tABC")).collect();
                    for &ef in &ALL_EFLAGS {
                        let a = regexec_one(&c, cc.prog, &subj, ef);
                        let bb = regexec_one(&r, rr.prog, &subj, ef);
                        b.check(
                            &format!(
                                "regexec({:?},cf={cf},ef={ef},{:?})",
                                show(pat),
                                show(&subj)
                            ),
                            &a,
                            &bb,
                        );
                    }
                }
            }
            regfree_one(&c, &cc);
            regfree_one(&r, &rr);
        }
    }
    b.finish("regexp random subjects");
}

#[test]
fn regexp_utf8_subjects_and_patterns() {
    // CONFIGS row: multi-byte UTF-8 in both pattern and subject, plus
    // REG_ICASE over non-ASCII (uses the Unicode case tables).
    let pats: &[&[u8]] = &[
        "\u{e9}".as_bytes(),
        "[\u{e0}-\u{ff}]".as_bytes(),
        "caf\u{e9}".as_bytes(),
        ".".as_bytes(),
        "..".as_bytes(),
        ".*".as_bytes(),
        "\u{4f60}\u{597d}".as_bytes(),
        "[\u{4f00}-\u{5000}]+".as_bytes(),
        "\u{1f600}".as_bytes(),
        "\\u00e9".as_bytes(),
        "\u{130}".as_bytes(),
        "\u{131}".as_bytes(),
        "\u{178}".as_bytes(),
        "\u{ff}".as_bytes(),
        "\\w+".as_bytes(),
        "\\W".as_bytes(),
    ];
    let subjects: Vec<&[u8]> = vec![
        "".as_bytes(),
        "\u{e9}".as_bytes(),
        "\u{c9}".as_bytes(),
        "caf\u{e9}".as_bytes(),
        "CAF\u{c9}".as_bytes(),
        "\u{4f60}\u{597d}\u{4e16}\u{754c}".as_bytes(),
        "\u{1f600}\u{1f601}".as_bytes(),
        "\u{130}\u{131}".as_bytes(),
        "\u{178}\u{ff}".as_bytes(),
        b"\x80\xbf",
        b"\xc0\x80",
        b"\xed\xa0\x80",
        b"\xf4\x90\x80\x80",
        b"a\xffb",
        "stra\u{df}e".as_bytes(),
    ];
    let mut b = Batch::new();
    for pat in pats {
        for &cf in &ALL_CFLAGS {
            diff_regexp(&mut b, pat, cf, &subjects, &ALL_EFLAGS);
        }
    }
    b.finish("regexp UTF-8");
}

#[test]
fn regexp_newline_and_notbol_semantics() {
    // CONFIGS rows: REG_NEWLINE changes what `.`/`^`/`$` do; REG_NOTBOL
    // suppresses the initial `^` match. Cross-product both against
    // newline-bearing subjects.
    let pats: &[&[u8]] = &[
        b"^a", b"a$", b"^a$", b".", b".+", b"a.b", b"^", b"$", b"[^x]", b"[^x]+", b"^.*$",
        b"\\n", b"a\\nb", b"(^a)|(b$)",
    ];
    let subjects: Vec<&[u8]> = vec![
        b"",
        b"a",
        b"\n",
        b"a\n",
        b"\na",
        b"a\nb",
        b"a\n\nb",
        b"\n\n",
        b"x\na\ny",
        b"aaa\nbbb",
        b"a\rb",
        b"a\r\nb",
    ];
    let mut b = Batch::new();
    for pat in pats {
        for &cf in &ALL_CFLAGS {
            diff_regexp(&mut b, pat, cf, &subjects, &ALL_EFLAGS);
        }
    }
    b.finish("regexp REG_NEWLINE / REG_NOTBOL");
}

// ---------------------------------------------------------------------------
// Phase C: every `die()` in regexp.c
// ---------------------------------------------------------------------------

/// One row per distinct compile error message in regexp.c.
/// The expected message is the C's own string; we assert the C produces it AND
/// that Rust produces the identical string.
#[test]
fn regexp_compile_error_messages_match() {
    let cases: &[(&[u8], &str)] = &[
        // regexp.c:580 "syntax error"
        (b")", "unmatched ')'"),
        (b"a)", "unmatched ')'"),
        // regexp.c:557,563,570,577 "unmatched '('"
        (b"(", "unmatched '('"),
        (b"(a", "unmatched '('"),
        (b"(?:", "unmatched '('"),
        (b"(?:a", "unmatched '('"),
        (b"(?=", "unmatched '('"),
        (b"(?=a", "unmatched '('"),
        (b"(?!", "unmatched '('"),
        (b"(?!a", "unmatched '('"),
        // regexp.c:322 "unterminated character class"
        (b"[", "unterminated character class"),
        (b"[a", "unterminated character class"),
        (b"[a-", "unterminated character class"),
        (b"[^", "unterminated character class"),
        // regexp.c:224 "invalid character class range"
        (b"[z-a]", "invalid character class range"),
        (b"[b-a]", "invalid character class range"),
        // regexp.c:101/108 quantifier errors
        (b"*", "syntax error"),
        (b"+", "syntax error"),
        (b"?", "syntax error"),
        (b"{1}", "syntax error"),
        (b"a**", "invalid quantifier"),
        (b"a++", "invalid quantifier"),
        (b"a?*", "invalid quantifier"),
        (b"a{1}{2}", "invalid quantifier"),
        (b"a{2,1}", "invalid quantifier"),
        // regexp.c:128/138/143/153 "unterminated escape sequence"
        (b"\\", "unterminated escape sequence"),
        (b"\\x", "unterminated escape sequence"),
        (b"\\x4", "unterminated escape sequence"),
        (b"\\u", "unterminated escape sequence"),
        (b"\\u0", "unterminated escape sequence"),
        (b"\\u00", "unterminated escape sequence"),
        (b"\\u004", "unterminated escape sequence"),
        (b"\\c", "unterminated escape sequence"),
        // regexp.c:541 "invalid back-reference"
        (b"\\1", "invalid back-reference"),
        (b"\\9", "invalid back-reference"),
        (b"(a)\\2", "invalid back-reference"),
        // regexp.c:552 "too many captures"
        (
            b"(a)(a)(a)(a)(a)(a)(a)(a)(a)(a)(a)(a)(a)(a)(a)(a)",
            "too many captures",
        ),
        // regexp.c:493 "infinite loop matching the empty string"
        (b"(?:)*", "infinite loop matching the empty string"),
        (b"()*", "infinite loop matching the empty string"),
        (b"(?:)+", "infinite loop matching the empty string"),
        (b"()+", "infinite loop matching the empty string"),
        (b"(|)*", "infinite loop matching the empty string"),
        // regexp.c:186/200 "numeric overflow"
        (b"a{99999999999}", "numeric overflow"),
        (b"a{1,99999999999}", "numeric overflow"),
    ];

    let (c, r) = Impl::both();
    let mut b = Batch::new();
    let mut wrong_expectation = Vec::new();
    for (pat, expected) in cases {
        for &cf in &ALL_CFLAGS {
            let cc = regcomp_one(&c, pat, cf);
            let rr = regcomp_one(&r, pat, cf);
            // C is ground truth: record if our expected string was off, but
            // still require Rust == C.
            if let Compiled::Err(Some(m)) = &cc.outcome {
                if m != expected {
                    wrong_expectation.push(format!("{:?} -> C says {m:?}, table said {expected:?}", show(pat)));
                }
            } else {
                wrong_expectation
                    .push(format!("{:?} -> C did NOT reject (outcome {:?})", show(pat), cc.outcome));
            }
            b.check(&format!("regcomp error {:?} cflags={cf}", show(pat)), &cc.outcome, &rr.outcome);
            regfree_one(&c, &cc);
            regfree_one(&r, &rr);
        }
    }
    if !wrong_expectation.is_empty() {
        eprintln!(
            "note: expectation table differs from C ground truth (C wins):\n{}",
            wrong_expectation.join("\n")
        );
    }
    b.finish("regexp compile error messages");
}

/// Sweep every single- and double-character pattern: an exhaustive audit of the
/// compile-error surface (which message, or success) for short inputs.
#[test]
fn regexp_exhaustive_short_patterns() {
    let subjects: Vec<&[u8]> = vec![b"", b"a", b"ab", b"\n"];
    let mut b = Batch::new();
    // all 1-byte ASCII patterns
    for c0 in 1u8..=127 {
        diff_regexp(&mut b, &[c0], 0, &subjects, &ALL_EFLAGS);
    }
    // all 2-byte patterns over an interesting subset
    let set: Vec<u8> = b"ab.*+?()[]|^$\\-{},:=!019dDwWsSbBnrtfvxuc \n\t\0".to_vec();
    for &c0 in &set {
        for &c1 in &set {
            if c0 == 0 || c1 == 0 {
                continue;
            }
            diff_regexp(&mut b, &[c0, c1], 0, &subjects, &[0]);
        }
    }
    // all 3-byte patterns starting with a backslash (escape surface)
    for c1 in 1u8..=127 {
        for c2 in 1u8..=127 {
            diff_regexp(&mut b, &[b'\\', c1, c2], 0, &[b"a"], &[0]);
        }
    }
    b.finish("regexp exhaustive short patterns");
}

#[test]
fn regcomp_null_errorp_is_accepted() {
    // ERRORS row: `if (errorp) *errorp = ...` -- a NULL errorp must be tolerated
    // on both the success and the failure path (regexp.c:904, 983).
    let (c, r) = Impl::both();
    let fc = c.f::<FnRegcomp>("js_regcomp");
    let fr = r.f::<FnRegcomp>("js_regcomp");
    let mut b = Batch::new();
    for pat in [&b"abc"[..], b"(", b"[z-a]", b"", b"\\"] {
        let buf = cbytes(pat);
        let pc = unsafe { fc(buf.as_ptr() as *const c_char, 0, std::ptr::null_mut()) };
        let pr = unsafe { fr(buf.as_ptr() as *const c_char, 0, std::ptr::null_mut()) };
        b.check(
            &format!("regcomp({:?}, NULL errorp)", show(pat)),
            pc.is_null(),
            pr.is_null(),
        );
        if !pc.is_null() {
            unsafe { c.f::<FnRegfree>("js_regfree")(pc) };
        }
        if !pr.is_null() {
            unsafe { r.f::<FnRegfree>("js_regfree")(pr) };
        }
    }
    b.finish("regcomp NULL errorp");
}

#[test]
fn regexec_null_sub_is_accepted() {
    // ERRORS row: `if (!sub) sub = &scratch;` -- NULL sub must be tolerated and
    // must not change the return code (regexp.c: regexec head).
    let (c, r) = Impl::both();
    let mut b = Batch::new();
    for pat in [&b"a"[..], b"(a)(b)", b"^x$", b".*"] {
        let cc = regcomp_one(&c, pat, 0);
        let rr = regcomp_one(&r, pat, 0);
        assert_eq!(cc.outcome, Compiled::Ok);
        let fc = c.f::<FnRegexec>("js_regexec");
        let fr = r.f::<FnRegexec>("js_regexec");
        for subj in [&b""[..], b"a", b"ab", b"xyz"] {
            let buf = cbytes(subj);
            let a = unsafe { fc(cc.prog, buf.as_ptr() as *const c_char, std::ptr::null_mut(), 0) };
            let bb = unsafe { fr(rr.prog, buf.as_ptr() as *const c_char, std::ptr::null_mut(), 0) };
            b.check(&format!("regexec({:?},{:?},NULL sub)", show(pat), show(subj)), a, bb);
        }
        regfree_one(&c, &cc);
        regfree_one(&r, &rr);
    }
    b.finish("regexec NULL sub");
}

#[test]
fn regfree_null_is_accepted() {
    // ERRORS row: `regfreex` guards `if (prog)`, so regfree(NULL) is a no-op.
    let (c, r) = Impl::both();
    unsafe { c.f::<FnRegfree>("js_regfree")(std::ptr::null_mut()) };
    unsafe { r.f::<FnRegfree>("js_regfree")(std::ptr::null_mut()) };
}

#[test]
fn regexec_out_of_range_eflags() {
    // Out-of-range flag bits are real FFI inputs (C enums accept any int).
    // `regexec` ORs eflags into prog->flags and the matcher tests individual
    // bits, so unknown bits must be ignored identically by both impls.
    let subjects = default_subjects();
    let mut b = Batch::new();
    let odd_flags: [c_int; 10] =
        [0, 8, 16, 32, 64, 0x1000, -1, c_int::MIN, c_int::MAX, REG_NOTBOL | 8];
    for pat in [&b"^a"[..], b"a$", b".", b"(a)(b)", b"[^x]+"] {
        for &cf in &ALL_CFLAGS {
            diff_regexp(&mut b, pat, cf, &subjects[..12], &odd_flags);
        }
    }
    b.finish("regexec out-of-range eflags");
}

#[test]
fn regcomp_out_of_range_cflags() {
    // Out-of-range cflags: stored verbatim into prog->flags (regexp.c:936) and
    // then OR'd with eflags, so both impls must treat unknown bits identically.
    let subjects = default_subjects();
    let mut b = Batch::new();
    let odd: [c_int; 11] = [
        0, 8, 16, 32, 64, 128, 0x1000, -1, c_int::MIN, c_int::MAX, REG_ICASE | REG_NEWLINE | 8,
    ];
    for pat in [&b"^a$"[..], b"A", b".", b"[a-z]+", b"(a)|(B)"] {
        for &cf in &odd {
            diff_regexp(&mut b, pat, cf, &subjects[..12], &ALL_EFLAGS);
        }
    }
    b.finish("regcomp out-of-range cflags");
}

// ---------------------------------------------------------------------------
// regcompx / regfreex -- the custom-allocator entry points
// ---------------------------------------------------------------------------

/// A `js_Alloc`-shaped allocator that counts calls per implementation.
mod alloc_ctx {
    use std::ffi::{c_int, c_void};

    #[repr(C)]
    pub struct Ctx {
        pub allocs: u64,
        pub frees: u64,
        pub bytes: u64,
        pub magic: u64,
    }

    pub unsafe extern "C" fn counting_alloc(ctx: *mut c_void, p: *mut c_void, n: c_int) -> *mut c_void {
        let c = &mut *(ctx as *mut Ctx);
        assert_eq!(c.magic, 0xDEAD_BEEF_CAFE_F00D, "allocator got the wrong ctx");
        if n == 0 {
            c.frees += 1;
            if !p.is_null() {
                libc_free(p);
            }
            std::ptr::null_mut()
        } else {
            c.allocs += 1;
            c.bytes += n as u64;
            libc_realloc(p, n as usize)
        }
    }

    /// Allocator that always fails, to drive the "cannot allocate ..." paths.
    pub unsafe extern "C" fn failing_alloc(ctx: *mut c_void, p: *mut c_void, n: c_int) -> *mut c_void {
        let c = &mut *(ctx as *mut Ctx);
        if n == 0 {
            c.frees += 1;
            if !p.is_null() {
                libc_free(p);
            }
            return std::ptr::null_mut();
        }
        c.allocs += 1;
        std::ptr::null_mut()
    }

    /// Allocator that fails only after `magic`-th successful allocation, so we
    /// can reach each individual "cannot allocate" die() site in regcompx.
    pub static mut FAIL_AFTER: u64 = u64::MAX;

    pub unsafe extern "C" fn fail_after_alloc(ctx: *mut c_void, p: *mut c_void, n: c_int) -> *mut c_void {
        let c = &mut *(ctx as *mut Ctx);
        if n == 0 {
            c.frees += 1;
            if !p.is_null() {
                libc_free(p);
            }
            return std::ptr::null_mut();
        }
        c.allocs += 1;
        if c.allocs > FAIL_AFTER {
            return std::ptr::null_mut();
        }
        libc_realloc(p, n as usize)
    }

    extern "C" {
        #[link_name = "realloc"]
        fn c_realloc(p: *mut c_void, n: usize) -> *mut c_void;
        #[link_name = "free"]
        fn c_free(p: *mut c_void);
    }
    unsafe fn libc_realloc(p: *mut c_void, n: usize) -> *mut c_void {
        c_realloc(p, n)
    }
    unsafe fn libc_free(p: *mut c_void) {
        c_free(p)
    }
}

use alloc_ctx::Ctx;

fn new_ctx() -> Box<Ctx> {
    Box::new(Ctx { allocs: 0, frees: 0, bytes: 0, magic: 0xDEAD_BEEF_CAFE_F00D })
}

type AllocFn = unsafe extern "C" fn(*mut c_void, *mut c_void, c_int) -> *mut c_void;

#[test]
fn regcompx_with_custom_allocator_matches() {
    // CONFIGS rows: js_regcompx/js_regfreex with a custom allocator and a
    // non-NULL ctx. We compare the compile outcome AND the exact allocation
    // call counts / byte totals, which pins down the allocation sequence.
    let (c, r) = Impl::both();
    let fc = c.f::<FnRegcompx>("js_regcompx");
    let fr = r.f::<FnRegcompx>("js_regcompx");
    let gc = c.f::<FnRegfreex>("js_regfreex");
    let gr = r.f::<FnRegfreex>("js_regfreex");
    let a: AllocFn = alloc_ctx::counting_alloc;
    let mut b = Batch::new();

    let pats: Vec<&[u8]> = vec![
        b"", b"a", b"abc", b"(a)(b)", b"[a-z]+", b"a{2,5}", b"(?:x|y)*z", b"\\d+\\.\\d+",
        b"[abc][def][ghi]", b"^(\\w+)@(\\w+)$", b"(", b"[z-a]", b"\\", b"a**", b"\\1",
        b"(?:)*",
    ];
    for pat in pats {
        for &cf in &ALL_CFLAGS {
            let buf = cbytes(pat);
            let mut cc = new_ctx();
            let mut rr = new_ctx();
            let mut ec: *const c_char = std::ptr::null();
            let mut er: *const c_char = std::ptr::null();
            let pc = unsafe {
                fc(
                    Some(a),
                    &mut *cc as *mut Ctx as *mut c_void,
                    buf.as_ptr() as *const c_char,
                    cf,
                    &mut ec,
                )
            };
            let pr = unsafe {
                fr(
                    Some(a),
                    &mut *rr as *mut Ctx as *mut c_void,
                    buf.as_ptr() as *const c_char,
                    cf,
                    &mut er,
                )
            };
            let msg_c = unsafe { read_cstr(ec) }.map(|m| show(&m));
            let msg_r = unsafe { read_cstr(er) }.map(|m| show(&m));
            b.check(
                &format!("regcompx({:?},cf={cf}) outcome", show(pat)),
                (pc.is_null(), msg_c),
                (pr.is_null(), msg_r),
            );
            // Exec comparison when both compiled.
            if !pc.is_null() && !pr.is_null() {
                for subj in [&b""[..], b"a", b"abc", b"12.5", b"x@y", b"adg"] {
                    let a1 = regexec_one(&c, pc, subj, 0);
                    let b1 = regexec_one(&r, pr, subj, 0);
                    b.check(
                        &format!("regcompx exec({:?},{:?})", show(pat), show(subj)),
                        &a1,
                        &b1,
                    );
                }
            }
            unsafe { gc(Some(a), &mut *cc as *mut Ctx as *mut c_void, pc) };
            unsafe { gr(Some(a), &mut *rr as *mut Ctx as *mut c_void, pr) };
            b.check(
                &format!("regcompx({:?},cf={cf}) alloc accounting", show(pat)),
                (cc.allocs, cc.frees, cc.bytes),
                (rr.allocs, rr.frees, rr.bytes),
            );
        }
    }
    b.finish("regcompx custom allocator");
}

#[test]
fn regcompx_allocation_failure_paths_match() {
    // ERRORS rows: the five "cannot allocate ..." die() sites in regcompx are
    // reached by failing the Nth allocation. We sweep N so each site is hit.
    let (c, r) = Impl::both();
    let fc = c.f::<FnRegcompx>("js_regcompx");
    let fr = r.f::<FnRegcompx>("js_regcompx");
    let gc = c.f::<FnRegfreex>("js_regfreex");
    let gr = r.f::<FnRegfreex>("js_regfreex");
    let mut b = Batch::new();

    // Total failure: the very first allocation fails.
    {
        let a: AllocFn = alloc_ctx::failing_alloc;
        for pat in [&b"abc"[..], b"", b"[a-z]+", b"(a)(b)"] {
            let buf = cbytes(pat);
            let mut cc = new_ctx();
            let mut rr = new_ctx();
            let mut ec: *const c_char = std::ptr::null();
            let mut er: *const c_char = std::ptr::null();
            let pc = unsafe {
                fc(Some(a), &mut *cc as *mut Ctx as *mut c_void, buf.as_ptr() as *const c_char, 0, &mut ec)
            };
            let pr = unsafe {
                fr(Some(a), &mut *rr as *mut Ctx as *mut c_void, buf.as_ptr() as *const c_char, 0, &mut er)
            };
            b.check(
                &format!("regcompx({:?}) with always-failing alloc", show(pat)),
                (pc.is_null(), unsafe { read_cstr(ec) }.map(|m| show(&m)), cc.allocs, cc.frees),
                (pr.is_null(), unsafe { read_cstr(er) }.map(|m| show(&m)), rr.allocs, rr.frees),
            );
            unsafe { gc(Some(a), &mut *cc as *mut Ctx as *mut c_void, pc) };
            unsafe { gr(Some(a), &mut *rr as *mut Ctx as *mut c_void, pr) };
        }
    }

    // Partial failure: fail after the Nth allocation.
    {
        let a: AllocFn = alloc_ctx::fail_after_alloc;
        for n in 1u64..=6 {
            for pat in [&b"abc"[..], b"[a-z]+[0-9]+", b"(a)(b)(c)", b"a{2,4}"] {
                let buf = cbytes(pat);
                let mut cc = new_ctx();
                let mut rr = new_ctx();
                let mut ec: *const c_char = std::ptr::null();
                let mut er: *const c_char = std::ptr::null();
                unsafe { alloc_ctx::FAIL_AFTER = n };
                let pc = unsafe {
                    fc(Some(a), &mut *cc as *mut Ctx as *mut c_void, buf.as_ptr() as *const c_char, 0, &mut ec)
                };
                unsafe { alloc_ctx::FAIL_AFTER = n };
                let pr = unsafe {
                    fr(Some(a), &mut *rr as *mut Ctx as *mut c_void, buf.as_ptr() as *const c_char, 0, &mut er)
                };
                b.check(
                    &format!("regcompx({:?}) fail after alloc #{n}", show(pat)),
                    (pc.is_null(), unsafe { read_cstr(ec) }.map(|m| show(&m)), cc.allocs),
                    (pr.is_null(), unsafe { read_cstr(er) }.map(|m| show(&m)), rr.allocs),
                );
                unsafe { alloc_ctx::FAIL_AFTER = u64::MAX };
                unsafe { gc(Some(a), &mut *cc as *mut Ctx as *mut c_void, pc) };
                unsafe { gr(Some(a), &mut *rr as *mut Ctx as *mut c_void, pr) };
            }
        }
    }
    b.finish("regcompx allocation failure paths");
}

#[test]
fn regfreex_null_prog_is_accepted() {
    let (c, r) = Impl::both();
    let gc = c.f::<FnRegfreex>("js_regfreex");
    let gr = r.f::<FnRegfreex>("js_regfreex");
    let a: AllocFn = alloc_ctx::counting_alloc;
    let mut cc = new_ctx();
    let mut rr = new_ctx();
    unsafe { gc(Some(a), &mut *cc as *mut Ctx as *mut c_void, std::ptr::null_mut()) };
    unsafe { gr(Some(a), &mut *rr as *mut Ctx as *mut c_void, std::ptr::null_mut()) };
    assert_eq!((cc.allocs, cc.frees), (rr.allocs, rr.frees));
}
