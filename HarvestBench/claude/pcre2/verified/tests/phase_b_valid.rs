// Phase B — valid-path differential tests. Drives BOTH C and Rust .so through
// their exports and asserts byte-identical outcomes across randomized inputs.
mod common;
use common::*;

// A tiny deterministic PRNG (xorshift64) so runs are reproducible with a fixed seed.
struct Rng(u64);
impl Rng {
    fn new(seed: u64) -> Rng {
        Rng(seed)
    }
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn range(&mut self, n: usize) -> usize {
        (self.next() % (n as u64)) as usize
    }
}

fn rand_ascii_subject(rng: &mut Rng, alphabet: &[u8], maxlen: usize) -> Vec<u8> {
    let len = rng.range(maxlen + 1);
    (0..len).map(|_| alphabet[rng.range(alphabet.len())]).collect()
}

fn assert_match_eq(
    c: &Pcre2Lib,
    r: &Pcre2Lib,
    pattern: &[u8],
    opts: u32,
    subject: &[u8],
    subj_len: usize,
    start: usize,
    mopts: u32,
    ovec: u32,
    ctx: &str,
) {
    unsafe {
        let co = c.run_match(pattern, opts, subject, subj_len, start, mopts, ovec);
        let ro = r.run_match(pattern, opts, subject, subj_len, start, mopts, ovec);
        assert_eq!(
            co.compile_ok, ro.compile_ok,
            "compile_ok diff [{}] pat={:?}", ctx, String::from_utf8_lossy(pattern)
        );
        assert_eq!(co.compile_errcode, ro.compile_errcode, "compile errcode diff [{}]", ctx);
        if !co.compile_ok {
            assert_eq!(co.compile_erroffset, ro.compile_erroffset, "erroffset diff [{}]", ctx);
            return;
        }
        assert_eq!(
            co.rc, ro.rc,
            "rc diff [{}] pat={:?} subj={:?}",
            ctx, String::from_utf8_lossy(pattern), String::from_utf8_lossy(subject)
        );
        assert_eq!(
            co.ovector, ro.ovector,
            "ovector diff [{}] pat={:?} subj={:?}",
            ctx, String::from_utf8_lossy(pattern), String::from_utf8_lossy(subject)
        );
        assert_eq!(co.startchar, ro.startchar, "startchar diff [{}]", ctx);
    }
}

// Rows 1-21, 24, 25: compile+match with many pattern classes and random subjects.
#[test]
fn row_compile_match_matrix() {
    let (c, r) = both();
    let mut rng = Rng::new(0xC0FFEE_1234_5678);

    // (pattern, compile options) pairs covering the axes in CONFIGS.md
    let cases: &[(&[u8], u32)] = &[
        (b"abc", 0),                                   // row 1 literal
        (b"a.*c", 0),                                  // row 2 quantifiers
        (b"a+b*c?", 0),
        (b"^abc$", 0),                                 // row 3 anchored
        (b"a|bb|ccc", 0),                              // row 4 alternation
        (b"(a)(b)(c)", 0),                             // row 5 capture groups
        (b"(a+)(b+)?(c*)", 0),
        (b"(?<first>a+)(?<second>b+)", 0),             // row 6 named groups
        (b"[a-z0-9]+", 0),                             // row 7 classes
        (b"[^abc]+", 0),
        (b"abc", PCRE2_CASELESS),                      // row 8 caseless
        (b"^a.c$", PCRE2_MULTILINE),                   // row 9 multiline
        (b"a.c", PCRE2_DOTALL),                        // row 10 dotall
        (b"a b c # comment\n", PCRE2_EXTENDED),        // row 11 extended
        (b"(a)\\1", 0),                                // row 14 backref
        (b"a(?=b)", 0),                                // row 15 lookahead
        (b"(?<=a)b", 0),                               // lookbehind
        (b"a{2,4}", 0),                                // row 16 bounded quant
        (b"\\bword\\b", 0),                            // word boundary
        (b"(?:ab)+", 0),                               // non-capturing
    ];

    let alphabet = b"abcABC012 \n";
    for (pat, opts) in cases {
        for _ in 0..200 {
            let subj = rand_ascii_subject(&mut rng, alphabet, 12);
            // random start offset within subject
            let start = if subj.is_empty() { 0 } else { rng.range(subj.len() + 1) };
            assert_match_eq(&c, &r, pat, *opts, &subj, subj.len(), start, 0, 30, "matrix");
            // Also test zero-terminated length (row 21)
            let mut z = subj.clone();
            z.push(0);
            assert_match_eq(&c, &r, pat, *opts, &z, PCRE2_ZERO_TERMINATED, 0, 0, 30, "matrix-zt");
        }
    }
}

// Rows 18, 19: match-time option flags.
#[test]
fn row_match_options() {
    let (c, r) = both();
    let mut rng = Rng::new(0xABCDEF01);
    let alphabet = b"abc\nAB";
    let mopts_set = [
        0,
        PCRE2_ANCHORED,
        PCRE2_ENDANCHORED,
        PCRE2_NOTBOL,
        PCRE2_NOTEOL,
        PCRE2_NOTEMPTY,
        PCRE2_NOTEMPTY_ATSTART,
        PCRE2_ANCHORED | PCRE2_ENDANCHORED,
    ];
    let pats: &[&[u8]] = &[b"a*", b"^a.c$", b"(a|b)+", b".*"];
    for pat in pats {
        for &mo in &mopts_set {
            for _ in 0..80 {
                let subj = rand_ascii_subject(&mut rng, alphabet, 10);
                assert_match_eq(&c, &r, pat, 0, &subj, subj.len(), 0, mo, 30, "mopts");
            }
        }
    }
}

// Row 12, 13: UTF and UCP.
#[test]
fn row_utf_ucp() {
    let (c, r) = both();
    let mut rng = Rng::new(0x5EED_C0DE);
    // random UTF-8 subjects built from a set of code points
    let cps: [char; 8] = ['a', 'é', 'ω', '€', '中', 'A', '9', 'ñ'];
    let pats: &[(&[u8], u32)] = &[
        (b".", PCRE2_UTF),
        (b".+", PCRE2_UTF),
        (b"\\p{L}+", PCRE2_UTF | PCRE2_UCP),
        (b"\\p{N}", PCRE2_UTF | PCRE2_UCP),
        (b"[\\x{80}-\\x{ffff}]", PCRE2_UTF),
        (b"\\X", PCRE2_UTF),
    ];
    for (pat, opts) in pats {
        for _ in 0..200 {
            let n = rng.range(6);
            let mut s = String::new();
            for _ in 0..n {
                s.push(cps[rng.range(cps.len())]);
            }
            let bytes = s.into_bytes();
            assert_match_eq(&c, &r, pat, *opts, &bytes, bytes.len(), 0, 0, 30, "utf");
        }
    }
}

// Row 20: empty subjects, and small ovector sizes.
#[test]
fn row_empty_and_ovecsizes() {
    let (c, r) = both();
    let pats: &[&[u8]] = &[b"a*", b"", b"(?:)", b"^$", b"(a)(b)(c)(d)"];
    for pat in pats {
        for &ov in &[1u32, 2, 3, 10] {
            assert_match_eq(&c, &r, pat, 0, b"", 0, 0, 0, ov, "empty");
            assert_match_eq(&c, &r, pat, 0, b"abcd", 4, 0, 0, ov, "small-ovec");
        }
    }
}

// Rows 22, 23: DFA matching.
#[test]
fn row_dfa_match() {
    let (c, r) = both();
    let mut rng = Rng::new(0xD1FA_1234);
    let alphabet = b"abcAB012 \n";
    let cases: &[(&[u8], u32, u32)] = &[
        (b"a.*c", 0, 0),
        (b"a+b*c?", 0, 0),
        (b"^abc$", 0, 0),
        (b"a|bb|ccc", 0, 0),
        (b"[a-z0-9]+", 0, 0),
        (b"a.*c", 0, PCRE2_DFA_SHORTEST),
        (b"(a|b)+", 0, PCRE2_DFA_SHORTEST),
    ];
    for (pat, opts, dfaopt) in cases {
        for _ in 0..150 {
            let subj = rand_ascii_subject(&mut rng, alphabet, 12);
            unsafe {
                let mut ec = 0;
                let mut eo = 0;
                let cc = (c.compile)(pat.as_ptr(), pat.len(), *opts, &mut ec, &mut eo, std::ptr::null_mut());
                let rc_code = (r.compile)(pat.as_ptr(), pat.len(), *opts, &mut ec, &mut eo, std::ptr::null_mut());
                assert!(!cc.is_null() && !rc_code.is_null());
                let cmd = (c.md_create)(10, std::ptr::null_mut());
                let rmd = (r.md_create)(10, std::ptr::null_mut());
                let mut cws = [0i32; 40];
                let mut rws = [0i32; 40];
                let crc = (c.dfa_match)(cc, subj.as_ptr(), subj.len(), 0, *dfaopt, cmd, std::ptr::null_mut(), cws.as_mut_ptr(), 40);
                let rrc = (r.dfa_match)(rc_code, subj.as_ptr(), subj.len(), 0, *dfaopt, rmd, std::ptr::null_mut(), rws.as_mut_ptr(), 40);
                assert_eq!(crc, rrc, "dfa rc diff pat={:?} subj={:?}", String::from_utf8_lossy(pat), String::from_utf8_lossy(&subj));
                let cn = (c.ovector_count)(cmd) as usize;
                let cop = (c.ovector_ptr)(cmd);
                let rop = (r.ovector_ptr)(rmd);
                for i in 0..cn * 2 {
                    assert_eq!(*cop.add(i), *rop.add(i), "dfa ovector[{}] diff", i);
                }
                (c.md_free)(cmd);
                (r.md_free)(rmd);
                (c.code_free)(cc);
                (r.code_free)(rc_code);
            }
        }
    }
}
