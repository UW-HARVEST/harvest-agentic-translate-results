//! Phase C: compile-time and match-time error paths.
//! ERRORS.md rows 1-57 (and row 18: every reachable compile error code).
mod harness;
use harness::*;
use std::collections::BTreeSet;
use std::ffi::c_void;
use std::os::raw::c_int;

/// Compile in both libraries and require identical (code, errorcode, erroroffset).
fn diff_compile(pat: &[u8], patlen: Sz, options: u32, xoptions: u32, ctxsetup: &dyn Fn(&Api, Ctx)) -> (bool, c_int, Sz) {
    let mut res = Vec::new();
    for api in [c(), r()] {
        unsafe {
            let cc = (api.compile_context_create)(std::ptr::null_mut());
            (api.set_compile_extra_options)(cc, xoptions);
            ctxsetup(api, cc);
            let mut err: c_int = -12345;
            let mut off: Sz = 0xdead_beef;
            let code = (api.compile)(pat.as_ptr(), patlen, options, &mut err, &mut off, cc);
            let ok = !code.is_null();
            if ok {
                (api.code_free)(code);
            }
            (api.compile_context_free)(cc);
            res.push((ok, err, off));
        }
    }
    assert_eq!(
        res[0], res[1],
        "COMPILE DIVERGENCE pat={:?} options={options:#x} xoptions={xoptions:#x}\n C   ={:?}\n Rust={:?}",
        String::from_utf8_lossy(pat),
        res[0],
        res[1]
    );
    res[0]
}

fn noop(_: &Api, _: Ctx) {}

// -------------------------------------------------------------- rows 1, 2, 3, 4
#[test]
fn compile_null_pointer_arguments() {
    unsafe {
        let mut out = Vec::new();
        for api in [c(), r()] {
            let mut v: Vec<(i64, i64, i64)> = Vec::new();
            // row 1: errorptr == NULL, erroroffset non-NULL
            let mut off: Sz = 0xdead;
            let code = (api.compile)(b"a".as_ptr(), 1, 0, std::ptr::null_mut(), &mut off,
                                     std::ptr::null_mut());
            v.push((code.is_null() as i64, 0, off as i64));
            // row 1b: both NULL
            let code2 = (api.compile)(b"a".as_ptr(), 1, 0, std::ptr::null_mut(),
                                      std::ptr::null_mut(), std::ptr::null_mut());
            v.push((code2.is_null() as i64, 0, 0));
            // row 2: erroroffset == NULL
            let mut err: c_int = -1;
            let code3 = (api.compile)(b"a".as_ptr(), 1, 0, &mut err, std::ptr::null_mut(),
                                      std::ptr::null_mut());
            v.push((code3.is_null() as i64, err as i64, 0));
            // row 3: NULL pattern, non-zero length
            let mut err: c_int = -1;
            let mut off: Sz = 0xdead;
            let code4 = (api.compile)(std::ptr::null(), 3, 0, &mut err, &mut off,
                                      std::ptr::null_mut());
            v.push((code4.is_null() as i64, err as i64, off as i64));
            // row 4: NULL pattern, zero length -> success
            let mut err: c_int = -1;
            let mut off: Sz = 0xdead;
            let code5 = (api.compile)(std::ptr::null(), 0, 0, &mut err, &mut off,
                                      std::ptr::null_mut());
            v.push((code5.is_null() as i64, err as i64, off as i64));
            if !code5.is_null() {
                // it must behave like an empty pattern
                let md = (api.match_data_create_from_pattern)(code5, std::ptr::null_mut());
                let rc = (api.do_match)(code5, b"xyz".as_ptr(), 3, 0, 0, md,
                                        std::ptr::null_mut());
                v.push((rc as i64, 0, 0));
                (api.match_data_free)(md);
                (api.code_free)(code5);
            }
            for cd in [code, code2, code3, code4] {
                if !cd.is_null() {
                    (api.code_free)(cd);
                }
            }
            out.push(v);
        }
        assert_eq!(out[0], out[1], "compile NULL-argument handling differs");
        eprintln!("compile NULL results = {:?}", out[0]);
        assert_eq!(out[0][2].1, 220, "erroroffset==NULL must give ERR_NULL_EROROFFSET (220)");
        assert_eq!(out[0][3].1, 116, "NULL pattern with length must give 116");
    }
}

// ---------------------------------------------------------------- rows 5, 6, 7, 8
#[test]
fn compile_option_validation() {
    // every single undefined compile-option bit
    let public: u32 = 0x8000_0000
        | 0x4000_0000
        | 0x2000_0000
        | 0x0fff_ffff & !0x0000_0000; // computed below instead
    let _ = public;
    for bit in 0..32u32 {
        let opt = 1u32 << bit;
        diff_compile(b"a", 1, opt, 0, &noop);
        diff_compile(b"a", 1, opt | PCRE2_CASELESS, 0, &noop);
        diff_compile(b"(a)(?<n>b)", 10, opt, 0, &noop);
    }
    // every single extra-option bit
    for bit in 0..32u32 {
        let x = 1u32 << bit;
        diff_compile(b"a", 1, 0, x, &noop);
        diff_compile(b"\\p{L}\\d\\w[[:alpha:]]", 20, PCRE2_UCP, x, &noop);
    }
    // LITERAL crossed with every other option bit and extra-option bit (rows 7, 8)
    for bit in 0..32u32 {
        let opt = 1u32 << bit;
        diff_compile(b"a.b*", 4, PCRE2_LITERAL | opt, 0, &noop);
        diff_compile(b"a.b*", 4, PCRE2_LITERAL, 1u32 << bit, &noop);
    }
    // full-garbage option words
    for opt in [0xffff_ffffu32, 0x5555_5555, 0xaaaa_aaaa, 0x1000_0000, 0x1000_0001] {
        diff_compile(b"a", 1, opt, 0, &noop);
        diff_compile(b"a", 1, 0, opt, &noop);
        diff_compile(b"a", 1, opt, opt, &noop);
    }
}

// ----------------------------------------------------------------- rows 9, 10
#[test]
fn compile_length_limits() {
    let pats: &[&[u8]] = &[b"a", b"abcdef", b"(a)(b)(c)", b"\\p{L}+(?<n>x)"];
    for p in pats {
        for lim in [0usize, 1, p.len() - 1, p.len(), p.len() + 1, usize::MAX] {
            diff_compile(p, p.len(), 0, 0, &|api, cc| unsafe {
                (api.set_max_pattern_length)(cc, lim);
            });
        }
        for lim in [0usize, 1, 8, 16, 32, 64, 128, 1024, usize::MAX] {
            diff_compile(p, p.len(), 0, 0, &|api, cc| unsafe {
                (api.set_max_pattern_compiled_length)(cc, lim);
            });
        }
    }
}

// ----------------------------------------------------- rows 11, 12, 13, 14, 16, 17
#[test]
fn compile_never_and_nest_limits() {
    diff_compile(b"(*UTF)a", 7, PCRE2_NEVER_UTF, 0, &noop);
    diff_compile(b"(*UTF)a", 7, 0, 0, &noop);
    diff_compile(b"(*UCP)\\w", 8, PCRE2_NEVER_UCP, 0, &noop);
    diff_compile(b"(*UCP)\\w", 8, 0, 0, &noop);
    diff_compile(b"\\C", 2, PCRE2_NEVER_BACKSLASH_C, 0, &noop);
    diff_compile(b"\\C", 2, 0, 0, &noop);
    diff_compile(b"(?C1)a", 6, 0, PCRE2_EXTRA_NEVER_CALLOUT, &noop);
    diff_compile(b"(?C1)a", 6, 0, 0, &noop);
    diff_compile(b"a(?C{x})b", 9, 0, PCRE2_EXTRA_NEVER_CALLOUT, &noop);

    // parens nest limit
    for depth in [0usize, 1, 2, 5, 20, 50, 200] {
        let pat = format!("{}a{}", "(".repeat(depth), ")".repeat(depth));
        for lim in [0u32, 1, 2, 5, 20, 50, 250] {
            diff_compile(pat.as_bytes(), pat.len(), 0, 0, &|api, cc| unsafe {
                (api.set_parens_nest_limit)(cc, lim);
            });
        }
    }
    // max_varlookbehind
    for pat in [
        "(?<=a{1,5})b",
        "(?<=a{1,300})b",
        "(?<=ab|cdefgh)x",
        "(?<!a{2,20})b",
        "(?<=(a|bc|def))x",
    ] {
        for lim in [0u32, 1, 2, 5, 255, 65535, u32::MAX] {
            diff_compile(pat.as_bytes(), pat.len(), 0, 0, &|api, cc| unsafe {
                (api.set_max_varlookbehind)(cc, lim);
            });
        }
    }
}

// -------------------------------------------------------------------- row 15
#[test]
fn compile_invalid_utf_patterns() {
    let bad: Vec<Vec<u8>> = vec![
        vec![0x80],
        vec![0xff],
        vec![0xfe],
        vec![0xc2],
        vec![0xc0, 0x80],
        vec![0xe0, 0x80, 0x80],
        vec![0xed, 0xa0, 0x80],
        vec![0xf4, 0x90, 0x80, 0x80],
        vec![b'a', 0x80, b'b'],
        vec![b'[', 0x80, b']'],
        vec![0xf8, 0x88, 0x80, 0x80, 0x80],
    ];
    for p in &bad {
        diff_compile(p, p.len(), PCRE2_UTF, 0, &noop);
        diff_compile(p, p.len(), 0, 0, &noop);
        // NO_UTF_CHECK on invalid input is undefined behaviour per the docs, so
        // it is deliberately not exercised here.
    }
}

// ---------------------------------------------- row 18: every compile error code
/// Curated invalid (and boundary-valid) patterns, one or more per error code.
const BAD_PATTERNS: &[&str] = &[
    "\\", "a\\", "[a\\", "(a\\",                                  // 101
    "\\Ca", "[\\C]",                                              // 102/168
    "\\y", "\\Y", "[\\y]", "\\i",                                 // 103
    "a{3,1}", "a{9,2}",                                           // 104
    "a{1,70000}", "a{70000}", "a{100000,}",                       // 105
    "[a-z", "[", "[^", "[]", "[a",                                // 106
    "[\\b]x", "[a-\\d]", "[\\A]",                                 // 107/150
    "[z-a]", "[\\x41-\\x30]",                                     // 108
    "*a", "+a", "?a", "{2}a", "a**", "a*{", "(?i)*a",             // 109/111
    "[[:alpha:]", "[:alpha:]", "x[:digit:]y",                     // 112
    "[[.ch.]]", "[[=e=]]",                                        // 113
    "(a", "(?:a", "(?<n>a", "(?(1)a", "(?>a", "(?=a",             // 114
    "(a)\\2", "\\1", "(?1)", "(?2)(a)", "\\g{2}(a)",              // 115
    "a)b", ")", "a))",                                            // 122
    "x(?#unterminated", "(?#",                                     // 118
    "(?(1a)b|c)", "(?(?<n)a|b)",                                   // 124/128
    "(?<=a*)b", "(?<=a+)b", "(?<=(?:ab)*)c",                       // 125
    "(a)\\g{-0}", "(?+0)", "(?-0)", "\\g{0}",                      // 126
    "(?(1)a|b|c)", "(?(1)a|b|c|d)",                                // 127
    "[[:foo:]]", "[[:^foo:]]",                                     // 130
    "(?P<n>a)(?P<n>b)", "(?<n>a)(?<n>b)",                          // 143
    "(?<>a)", "(?<1n>a)", "(?<n!>a)", "(?'>a)", "(?P<>a)",         // 144
    "\\p{}", "\\p{^}", "\\pX", "\\p", "\\P",                       // 146
    "\\p{NotAProperty}", "\\p{Zzzz_Nope}",                         // 147
    "(?<aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa>x)", // 148
    "\\400", "\\777", "[\\400]",                                   // 151
    "(?(DEFINE)a|b)",                                              // 154
    "\\o", "\\o{", "\\o{}", "\\o{9}", "\\o{777777777777}",         // 155/198/164
    "\\g", "\\g{", "\\g{}", "\\g<", "\\g'", "\\gx",                // 157
    "(?R", "(?R1", "(?&", "(?&n",                                  // 158
    "(*ACCEPT:x)", "(*FAIL:x)",                                    // 159
    "(*NOSUCHVERB)", "(*BADVERB:x)", "(*)",                        // 160
    "(?12345678901)", "\\12345678901",                             // 161
    "(?&)", "(?P>)", "(?P=)",                                      // 162
    "\\x{}", "\\x{zz}", "\\x{110000}", "\\x{ffffffffff}",          // 167/177
    "\\N{U+41}", "\\N{}", "\\N{U+}", "\\N{U+110000}",              // 137/178
    "(?<n>a)\\k", "\\k<", "\\k{", "\\k'", "\\kx",                  // 169
    "[\\N]", "[\\N{U+41}]",                                        // 171
    "\\x{d800}", "[\\x{d800}]",                                    // 173
    "(*MARK)", "(*MARK:)", "(*:)",                                 // 166
    "(*LIMIT_MATCH=)", "(*LIMIT_MATCH=x)", "(*LIMIT_HEAP=)",       // 179 etc.
    "(?C300)", "(?C{", "(?C1x)", "(?C\u{01}x\u{01})",              // 138/139/181/182
    "(?|(a)|(b))(?|(c)|(d))",
    "(?J:(?<n>a)(?<n>b))",
    "(*sr:)", "(*script_run:)", "(*asr:", "(*atomic_script_run:",
    "(*pla:a", "(*positive_lookahead:a", "(*NOTANASSERTION:a)",     // 195
    "(?[a])", "[[a]&&]", "[[a]&&[b]||[c]]", "[[a]--]", "[a&&b]",    // eclass errors
    "(?i)(?^i)a", "(?^)a", "(?-)a", "(?im-)a", "(?im-x-i)a",        // 194
    "a{,}", "a{}", "a{,,}", "a{1,2,3}",
    "(?<n>a)(?<m>b)(?(<z>)x|y)",
    "(?(VERSION>=99.0)a|b)", "(?(VERSION=)a|b)", "(?(VERSION))",
    "\\Q", "\\E", "a\\Eb",
    "(?P", "(?P<", "(?PX", "(?P=n)",
    "\\p{Bidi_Class:NoSuchValue}", "\\p{Script=NoSuchScript}",
    "(?<n>){0}(?&n)(?&nope)",
    "[[:word:]-[:digit:]]",
    "a(?=b)*", "a(?!b)+", "(?=a){2}",
    "\\1(a)",
    "(?(R99)a|b)", "(?(R&nope)a|b)",
    "((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((a",
    "(?xx) a b",
    "\\B{2}",
    "(?<n>a)(?<n>b)(?<n>c)",
    "[\\p{L}-\\p{N}]",
    "\\u", "\\u{", "\\u{110000}", "\\uZZZZ",
    "(?C{a", "(?C'a", "(?C\"a",
    "a\\c", "\\c", "\\c\u{80}", "[\\c]",
    "(?~)", "(?%)a", "(?\u{01})a",
    "(a)(?(1)x|y|z)", "(a)(?(1)x|y|z|w)",
    "(?-5)a", "(?+9)a", "(a)(?-2)",
    "(?<=\\C)a", "(?<!\\C)a",
    "(*MARK:a\\db)", "(*MARK:a\\Qb)",
    "(?|(?<a>x)|(?<b>y))",
    "(*plaz:a)", "(*nope_assertion:a)", "(*positive_lookbehindx:a)",
    "\\o{}", "\\o{8}", "\\0{", "\\o{ }",
    "(?<=a{300})b", "(?<=a{1000})b",
    "\\g<1", "\\g'1", "\\g{1", "(a)\\g<1",
    "(?&nosuchname)", "(?P>nosuchname)", "\\k<nosuchname>",
    "(?()a|b)", "(?(a)b|c)", "(?(-1)a|b)", "(?(+99)a|b)",
    "[[:alpha:]&&[:digit:]]", "[[a]~~[b]]", "[[a]&&", "[^[a]&&[b]",
    "(?[)", "(?[a", "(?[[a] + ]", "(?[[a] & ]", "(?[ ]",
    "(?[[a]-[b]|[c]])", "(?[!])", "(?[[a]!])",
    "(?xxx)a", "(?im-sx:a)", "(?i-i:a)", "(?--i:a)",
    "\\N{U+D800}", "\\N{U+FFFFFFFF}", "\\N{Uu+41}",
    "(?C4294967296)", "(?C99999999999999999999)",
    "(?<n>a){0}(?(<n>)x|y)",
    "\\x{7fffffff}", "\\x{80000000}", "\\x{ffffffffffffffff}",
    "a{1,2}{3,4}", "a??+", "a{2}{3}",
    "(?(?=a)b|c|d)",
    "[\\p{}]", "[\\p]", "[\\P{}]",
    "\\p{Bidi_Class}", "\\p{=L}", "\\p{Script_Extensions=}",
    "(*UTF", "(*UCP", "(*LIMIT_MATCH=1", "(*CR", "(*NUL",
    "(*BSR_NOPE)", "(*NOTANOPTION)",
    "(?J-J:a)", "(?J)(?<n>a)(?<n>b)(?<n>c)",
    "x{1,}{", "(?<n>a)(?&n){0}(?&m)",
];

#[test]
fn compile_error_code_corpus() {
    let mut codes: BTreeSet<c_int> = BTreeSet::new();
    let xopts = [
        0u32,
        PCRE2_EXTRA_ALT_BSUX,
        PCRE2_EXTRA_BAD_ESCAPE_IS_LITERAL,
        PCRE2_EXTRA_ALLOW_SURROGATE_ESCAPES,
        PCRE2_EXTRA_PYTHON_OCTAL,
        PCRE2_EXTRA_ESCAPED_CR_IS_LF,
        PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK,
        PCRE2_EXTRA_TURKISH_CASING,
        PCRE2_EXTRA_CASELESS_RESTRICT,
        PCRE2_EXTRA_NO_BS0,
    ];
    let opts = [
        0u32,
        PCRE2_UTF,
        PCRE2_UTF | PCRE2_UCP,
        PCRE2_UCP,
        PCRE2_DUPNAMES,
        PCRE2_ALT_BSUX,
        PCRE2_ALT_EXTENDED_CLASS,
        PCRE2_ALT_VERBNAMES,
        PCRE2_EXTENDED,
        PCRE2_ALLOW_EMPTY_CLASS,
        PCRE2_CASELESS,
        PCRE2_MULTILINE,
        PCRE2_AUTO_CALLOUT,
        PCRE2_NO_AUTO_CAPTURE,
    ];
    for p in BAD_PATTERNS {
        for &o in &opts {
            for &x in &xopts {
                let (ok, err, _off) = diff_compile(p.as_bytes(), p.len(), o, x, &noop);
                if !ok {
                    codes.insert(err);
                }
                // and via the zero-terminated path
                let z = cs(p);
                diff_compile(&z, PCRE2_ZERO_TERMINATED, o, x, &noop);
            }
        }
    }
    // every valid pattern too - the error code must be 0 and the code non-NULL
    for p in curated_patterns() {
        for &o in &opts {
            for &x in &xopts {
                diff_compile(p.as_bytes(), p.len(), o, x, &noop);
            }
        }
    }
    eprintln!(
        "distinct compile error codes exercised: {} -> {:?}",
        codes.len(),
        codes
    );
    assert!(
        codes.len() >= 70,
        "corpus only reached {} distinct compile error codes",
        codes.len()
    );
}

/// Byte-level and mutation fuzzing of the compiler: the error code AND the
/// error offset must agree for every input.
#[test]
fn compile_fuzz_error_offsets() {
    let mut rng = Rng::new(0x5EED_0F0F);
    let pool: &[u8] =
        b"()[]{}|*+?.^$\\/abzZ09 \t\n<>=!:;,-#&'\"`%~@_dwsWSDpPNRXKQEGABCcoxughkNvV";
    let opts = [
        0u32,
        PCRE2_UTF,
        PCRE2_UCP,
        PCRE2_UTF | PCRE2_UCP,
        PCRE2_EXTENDED,
        PCRE2_EXTENDED_MORE,
        PCRE2_DUPNAMES,
        PCRE2_ALT_BSUX,
        PCRE2_ALT_EXTENDED_CLASS,
        PCRE2_ALT_VERBNAMES,
        PCRE2_ALLOW_EMPTY_CLASS,
        PCRE2_AUTO_CALLOUT,
        PCRE2_LITERAL,
        PCRE2_CASELESS,
        PCRE2_NO_AUTO_CAPTURE,
    ];
    let xopts = [
        0u32,
        PCRE2_EXTRA_ALT_BSUX,
        PCRE2_EXTRA_BAD_ESCAPE_IS_LITERAL,
        PCRE2_EXTRA_PYTHON_OCTAL,
        PCRE2_EXTRA_ALLOW_SURROGATE_ESCAPES,
        PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK,
        PCRE2_EXTRA_ESCAPED_CR_IS_LF,
        PCRE2_EXTRA_TURKISH_CASING,
        PCRE2_EXTRA_CASELESS_RESTRICT,
        PCRE2_EXTRA_NEVER_CALLOUT,
    ];
    let mut codes: BTreeSet<c_int> = BTreeSet::new();

    // (a) pure random byte soup
    for _ in 0..30000 {
        let n = rng.range(1, 14);
        let pat: Vec<u8> = (0..n).map(|_| *rng.pick(pool)).collect();
        let (ok, err, _) = diff_compile(&pat, pat.len(), *rng.pick(&opts), *rng.pick(&xopts),
                                        &noop);
        if !ok {
            codes.insert(err);
        }
    }

    // (b) mutations of real patterns - reaches error paths deep inside the
    //     parser that random soup never gets to.
    let seeds: Vec<String> = curated_patterns()
        .iter()
        .map(|s| (*s).to_string())
        .chain(BAD_PATTERNS.iter().map(|s| (*s).to_string()))
        .collect();
    for _ in 0..60000 {
        let mut pat: Vec<u8> = rng.pick(&seeds).as_bytes().to_vec();
        let muts = rng.range(1, 3);
        for _ in 0..muts {
            if pat.is_empty() {
                pat.push(*rng.pick(pool));
                continue;
            }
            let i = rng.below(pat.len());
            match rng.below(4) {
                0 => pat[i] = *rng.pick(pool),
                1 => {
                    pat.insert(i, *rng.pick(pool));
                }
                2 => {
                    pat.remove(i);
                }
                _ => pat.truncate(i),
            }
        }
        let (ok, err, _) = diff_compile(&pat, pat.len(), *rng.pick(&opts), *rng.pick(&xopts),
                                        &noop);
        if !ok {
            codes.insert(err);
        }
    }
    eprintln!(
        "fuzz reached {} distinct compile error codes: {:?}",
        codes.len(),
        codes
    );
    assert!(codes.len() >= 60, "fuzz only reached {} codes", codes.len());
}

/// CONFIGS.md row 103 / ERRORS.md row 165: `_pcre2_check_escape_8` is only
/// reachable with a live `compile_block`, so it is driven through the compiler:
/// every escape letter, every numeric/hex/octal form, inside and outside a
/// class, under every extra option that changes its behaviour. Error code AND
/// offset must agree, and for the accepted escapes the compiled byte code is
/// compared too (see t09_internals).
#[test]
fn every_escape_sequence() {
    let mut letters: Vec<String> = Vec::new();
    for b in 0x20u8..0x7f {
        letters.push(format!("\\{}", b as char));
    }
    for b in 0x20u8..0x7f {
        letters.push(format!("[\\{}]", b as char));
    }
    let forms: &[&str] = &[
        "\\x", "\\x0", "\\x41", "\\xg", "\\x{}", "\\x{0}", "\\x{41}", "\\x{ 41 }",
        "\\x{7f}", "\\x{80}", "\\x{10ffff}", "\\x{110000}", "\\x{d800}", "\\x{zz}",
        "\\o", "\\o{}", "\\o{0}", "\\o{7}", "\\o{8}", "\\o{101}", "\\o{7777777777}",
        "\\0", "\\00", "\\000", "\\0000", "\\1", "\\7", "\\8", "\\9", "\\10", "\\012",
        "\\123", "\\400", "\\777", "\\8888",
        "\\N", "\\N{}", "\\N{U+0}", "\\N{U+41}", "\\N{U+D800}", "\\N{U+110000}", "\\N{X}",
        "\\g1", "\\g{1}", "\\g<1>", "\\g'1'", "\\g{-1}", "\\g{+1}", "\\g{name}", "\\g<name>",
        "\\k<n>", "\\k{n}", "\\k'n'", "\\k", "\\kn",
        "\\Q\\E", "\\Qabc\\E", "\\Qabc", "\\E",
        "\\u", "\\u0041", "\\u{41}", "\\u{}", "\\uZZZZ",
        "\\cA", "\\ca", "\\c", "\\c\u{7f}", "\\c1",
        "\\p{L}", "\\p{^L}", "\\P{L}", "\\pL", "\\p", "\\p{}", "\\p{Nope}",
        "\\b", "\\B", "\\A", "\\Z", "\\z", "\\G", "\\K", "\\C", "\\X", "\\R", "\\h", "\\H",
        "\\v", "\\V", "\\d", "\\D", "\\s", "\\S", "\\w", "\\W", "\\n", "\\r", "\\t", "\\f",
        "\\a", "\\e",
    ];
    for f in forms {
        letters.push((*f).to_string());
        letters.push(format!("[{f}]"));
        letters.push(format!("a{f}b"));
        letters.push(format!("[a{f}b]"));
    }
    let opts = [
        0u32,
        PCRE2_UTF,
        PCRE2_UCP,
        PCRE2_UTF | PCRE2_UCP,
        PCRE2_ALT_BSUX,
        PCRE2_ALT_EXTENDED_CLASS,
        PCRE2_CASELESS,
        PCRE2_EXTENDED,
    ];
    let xopts = [
        0u32,
        PCRE2_EXTRA_ALT_BSUX,
        PCRE2_EXTRA_BAD_ESCAPE_IS_LITERAL,
        PCRE2_EXTRA_ALLOW_SURROGATE_ESCAPES,
        PCRE2_EXTRA_PYTHON_OCTAL,
        PCRE2_EXTRA_ESCAPED_CR_IS_LF,
        PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK,
        PCRE2_EXTRA_NO_BS0,
        PCRE2_EXTRA_ASCII_BSD | PCRE2_EXTRA_ASCII_BSS | PCRE2_EXTRA_ASCII_BSW,
    ];
    let mut codes: BTreeSet<c_int> = BTreeSet::new();
    for p in &letters {
        for &o in &opts {
            for &x in &xopts {
                let (ok, err, _) = diff_compile(p.as_bytes(), p.len(), o, x, &noop);
                if !ok {
                    codes.insert(err);
                }
            }
        }
    }
    eprintln!(
        "escape sweep: {} patterns, {} distinct error codes",
        letters.len(),
        codes.len()
    );
    assert!(letters.len() > 500);
    assert!(codes.len() >= 20);
}

// ================================ match-time errors =========================

fn match_probe(api: &Api, code: Code, subject: &[u8], length: Sz, start: Sz, opts: u32,
               mcsetup: &dyn Fn(&Api, Ctx), dfa: bool, wscount: usize) -> (c_int, Sz) {
    unsafe {
        let mc = (api.match_context_create)(std::ptr::null_mut());
        mcsetup(api, mc);
        let md = (api.match_data_create_from_pattern)(code, std::ptr::null_mut());
        let sp = if subject.is_empty() { std::ptr::NonNull::dangling().as_ptr() } else { subject.as_ptr() };
        let rc = if dfa {
            let mut ws = vec![0i32; wscount.max(1)];
            (api.dfa_match)(code, sp, length, start, opts, md, mc, ws.as_mut_ptr(), wscount)
        } else {
            (api.do_match)(code, sp, length, start, opts, md, mc)
        };
        let n = (api.get_ovector_count)(md) as Sz;
        (api.match_data_free)(md);
        (api.match_context_free)(mc);
        (rc, n)
    }
}

fn diff_match(pat: &str, copts: u32, subject: &[u8], length: Sz, start: Sz, opts: u32,
              mcsetup: &dyn Fn(&Api, Ctx), dfa: bool, wscount: usize) -> c_int {
    let mut res = Vec::new();
    for api in [c(), r()] {
        unsafe {
            let mut err = 0;
            let mut off = 0;
            let p = pat.as_bytes();
            let code = (api.compile)(p.as_ptr(), p.len(), copts, &mut err, &mut off,
                                     std::ptr::null_mut());
            assert!(!code.is_null(), "{pat:?} did not compile: {err}");
            res.push(match_probe(api, code, subject, length, start, opts, mcsetup, dfa, wscount));
            (api.code_free)(code);
        }
    }
    assert_eq!(
        res[0], res[1],
        "MATCH DIVERGENCE pat={pat:?} copts={copts:#x} subj={:?} len={length} start={start} opts={opts:#x} dfa={dfa} ws={wscount}",
        String::from_utf8_lossy(subject)
    );
    res[0].0
}

// ------------------------------------------------------------- rows 19-28, 39-49
#[test]
fn match_argument_validation() {
    unsafe {
        let mut out = Vec::new();
        for api in [c(), r()] {
            let mut v: Vec<c_int> = Vec::new();
            let mut err = 0;
            let mut off = 0;
            let code = (api.compile)(b"a".as_ptr(), 1, 0, &mut err, &mut off,
                                     std::ptr::null_mut());
            let md = (api.match_data_create_from_pattern)(code, std::ptr::null_mut());
            let subj = b"aaa";
            // row 19: match_data == NULL
            v.push((api.do_match)(code, subj.as_ptr(), 3, 0, 0, std::ptr::null_mut(),
                                  std::ptr::null_mut()));
            // row 20: code == NULL
            v.push((api.do_match)(std::ptr::null_mut(), subj.as_ptr(), 3, 0, 0, md,
                                  std::ptr::null_mut()));
            // row 21: subject == NULL, non-zero length
            v.push((api.do_match)(code, std::ptr::null(), 3, 0, 0, md, std::ptr::null_mut()));
            // row 22: subject == NULL, zero length -> empty string
            v.push((api.do_match)(code, std::ptr::null(), 0, 0, 0, md, std::ptr::null_mut()));
            // DFA equivalents (rows 39-41)
            let mut ws = [0i32; 64];
            v.push((api.dfa_match)(code, subj.as_ptr(), 3, 0, 0, std::ptr::null_mut(),
                                   std::ptr::null_mut(), ws.as_mut_ptr(), 64));
            v.push((api.dfa_match)(std::ptr::null_mut(), subj.as_ptr(), 3, 0, 0, md,
                                   std::ptr::null_mut(), ws.as_mut_ptr(), 64));
            v.push((api.dfa_match)(code, std::ptr::null(), 3, 0, 0, md, std::ptr::null_mut(),
                                   ws.as_mut_ptr(), 64));
            v.push((api.dfa_match)(code, std::ptr::null(), 0, 0, 0, md, std::ptr::null_mut(),
                                   ws.as_mut_ptr(), 64));
            // row 42: wscount < 20 and boundary values
            for n in [0usize, 1, 19, 20, 21] {
                let mut w = vec![0i32; n.max(1)];
                v.push((api.dfa_match)(code, subj.as_ptr(), 3, 0, 0, md, std::ptr::null_mut(),
                                       w.as_mut_ptr(), n));
            }
            (api.match_data_free)(md);
            (api.code_free)(code);
            out.push(v);
        }
        assert_eq!(out[0], out[1], "match NULL-argument codes differ");
        eprintln!("match arg codes = {:?}", out[0]);
        assert_eq!(out[0][0], PCRE2_ERROR_NULL);
        assert_eq!(out[0][1], PCRE2_ERROR_NULL);
        assert_eq!(out[0][2], PCRE2_ERROR_NULL);
        assert_eq!(out[0][3], PCRE2_ERROR_NOMATCH);
        // indices 8..12 are wscount 0, 1, 19, 20, 21
        assert_eq!(out[0][8], PCRE2_ERROR_DFA_WSSIZE, "wscount 0");
        assert_eq!(out[0][9], PCRE2_ERROR_DFA_WSSIZE, "wscount 1");
        assert_eq!(out[0][10], PCRE2_ERROR_DFA_WSSIZE, "wscount 19");
        assert_eq!(out[0][11], 1, "wscount 20 is legal and 'a' matches 'aaa'");
        assert_eq!(out[0][12], 1, "wscount 21 is legal");
    }
}

// ---------------------------------------------------------- rows 23, 24, 27, 41-44
#[test]
fn match_option_and_offset_validation() {
    for bit in 0..32u32 {
        let o = 1u32 << bit;
        diff_match("a", 0, b"aaa", 3, 0, o, &noop, false, 64);
        diff_match("a", 0, b"aaa", 3, 0, o, &noop, true, 64);
        diff_match("a", PCRE2_ENDANCHORED, b"aaa", 3, 0, o, &noop, false, 64);
        diff_match("a", PCRE2_USE_OFFSET_LIMIT, b"aaa", 3, 0, o, &noop, false, 64);
    }
    for o in [0xffff_ffffu32, 0x5555_5555, 0xaaaa_aaaa] {
        diff_match("a", 0, b"aaa", 3, 0, o, &noop, false, 64);
        diff_match("a", 0, b"aaa", 3, 0, o, &noop, true, 64);
    }
    // start_offset past the end
    for start in [0usize, 1, 3, 4, 100, usize::MAX, usize::MAX - 1] {
        diff_match("a", 0, b"aaa", 3, start, 0, &noop, false, 64);
        diff_match("a", 0, b"aaa", 3, start, 0, &noop, true, 64);
    }
    // PARTIAL + ENDANCHORED (row 27 / 44)
    for copts in [0, PCRE2_ENDANCHORED] {
        for mopts in [
            PCRE2_PARTIAL_SOFT,
            PCRE2_PARTIAL_HARD,
            PCRE2_PARTIAL_SOFT | PCRE2_ENDANCHORED,
            PCRE2_PARTIAL_HARD | PCRE2_ENDANCHORED,
        ] {
            diff_match("abc", copts, b"ab", 2, 0, mopts, &noop, false, 64);
            diff_match("abc", copts, b"ab", 2, 0, mopts, &noop, true, 64);
        }
    }
    // offset limit without USE_OFFSET_LIMIT (rows 28 / 49)
    for copts in [0, PCRE2_USE_OFFSET_LIMIT] {
        for lim in [0usize, 2, PCRE2_UNSET] {
            diff_match("a", copts, b"aaa", 3, 0, 0, &|api, mc| unsafe {
                (api.set_offset_limit)(mc, lim);
            }, false, 64);
            diff_match("a", copts, b"aaa", 3, 0, 0, &|api, mc| unsafe {
                (api.set_offset_limit)(mc, lim);
            }, true, 64);
        }
    }
}

// ------------------------------------------------------------- rows 25, 26, 46, 47
#[test]
fn match_bad_magic_and_mode() {
    unsafe {
        let mut out = Vec::new();
        for api in [c(), r()] {
            let mut v = Vec::new();
            let mut err = 0;
            let mut off = 0;
            let good = (api.compile)(b"abc".as_ptr(), 3, 0, &mut err, &mut off,
                                     std::ptr::null_mut());
            let mut size: Sz = 0;
            (api.pattern_info)(good, 22, &mut size as *mut Sz as *mut c_void);
            // clone the compiled block, then corrupt the magic number / mode flags
            let raw = std::slice::from_raw_parts(good as *const u8, size);
            let md = (api.match_data_create)(4, std::ptr::null_mut());
            let mut ws = [0i32; 64];
            // Field offsets in pcre2_real_code (see c_src/src/pcre2_intmodedep.h):
            // memctl 0..24, tables 24, executable_jit 32, start_bitmap 40..72,
            // blocksize 72, code_start 80, magic_number 88, ..., flags 104.
            for (label, patch) in [("magic", 88usize), ("flags", 104usize)] {
                let _ = label;
                let mut copy = raw.to_vec();
                copy[patch] ^= 0xff;
                let fake = copy.as_mut_ptr() as Code;
                v.push((api.do_match)(fake, b"abc".as_ptr(), 3, 0, 0, md,
                                      std::ptr::null_mut()));
                v.push((api.dfa_match)(fake, b"abc".as_ptr(), 3, 0, 0, md,
                                       std::ptr::null_mut(), ws.as_mut_ptr(), 64));
                let mut u: u32 = 0;
                v.push((api.pattern_info)(fake, 0, &mut u as *mut u32 as *mut c_void));
            }
            // pattern_info(NULL) (row 98)
            let mut u: u32 = 0;
            v.push((api.pattern_info)(std::ptr::null_mut(), 0, &mut u as *mut u32 as *mut c_void));
            // callout_enumerate(NULL) (row 160)
            v.push((api.callout_enumerate)(std::ptr::null_mut(), None, std::ptr::null_mut()));
            (api.match_data_free)(md);
            (api.code_free)(good);
            out.push(v);
        }
        assert_eq!(out[0], out[1], "bad magic/mode handling differs");
        eprintln!("magic/mode codes = {:?}", out[0]);
        assert_eq!(out[0][0], PCRE2_ERROR_BADMAGIC, "match with bad magic");
        assert_eq!(out[0][1], PCRE2_ERROR_BADMAGIC, "dfa_match with bad magic");
        assert_eq!(out[0][2], PCRE2_ERROR_BADMAGIC, "pattern_info with bad magic");
        assert_eq!(out[0][3], PCRE2_ERROR_BADMODE, "match with bad mode flags");
        assert_eq!(out[0][4], PCRE2_ERROR_BADMODE, "dfa_match with bad mode flags");
        assert_eq!(out[0][5], PCRE2_ERROR_BADMODE, "pattern_info with bad mode flags");
        let n = out[0].len();
        assert_eq!(out[0][n - 2], PCRE2_ERROR_NULL, "pattern_info(NULL)");
        assert_eq!(out[0][n - 1], PCRE2_ERROR_NULL, "callout_enumerate(NULL)");
    }
}

// ------------------------------------------------------- rows 29, 30, 31, 45, 50
#[test]
fn match_invalid_utf_subjects() {
    let bad: Vec<Vec<u8>> = vec![
        vec![0x80],
        vec![0xff],
        vec![0xc2],
        vec![0xc0, 0x80],
        vec![0xe0, 0x80, 0x80],
        vec![0xed, 0xa0, 0x80],
        vec![0xf4, 0x90, 0x80, 0x80],
        vec![b'a', 0x80, b'b'],
        vec![0xe2, 0x82],
        vec![0xf0, 0x9f, 0x98],
        "héllo".as_bytes().to_vec(),
        vec![0xc3, 0xa9, 0x80],
    ];
    for s in &bad {
        for copts in [
            PCRE2_UTF,
            PCRE2_UTF | PCRE2_UCP,
            PCRE2_UTF | PCRE2_MATCH_INVALID_UTF,
        ] {
            for start in 0..=s.len() {
                diff_match(".", copts, s, s.len(), start, 0, &noop, false, 64);
                diff_match("a", copts, s, s.len(), start, 0, &noop, false, 64);
                diff_match("\\X", copts, s, s.len(), start, 0, &noop, false, 64);
                // DFA rejects MATCH_INVALID_UTF patterns (row 45)
                diff_match(".", copts, s, s.len(), start, 0, &noop, true, 64);
            }
        }
    }
}

// ------------------------------------------------------------- rows 32-36, 55
#[test]
fn match_runtime_limits() {
    let cases: &[(&str, &str)] = &[
        ("(a+)+b", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaac"),
        ("(a|aa)+c", "aaaaaaaaaaaaaaaaaaaaaaaad"),
        ("(?R)?a", "aaaaaaaaaaaaaaaaaaaa"),
        ("(?:a{0,100}){0,100}b", "aaaaaaaaaaaaaaaaaaaaaaac"),
        ("a*a*a*a*a*b", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaac"),
    ];
    for (p, s) in cases {
        for lim in [0u32, 1, 2, 10, 100, 1000, 10000] {
            diff_match(p, 0, s.as_bytes(), s.len(), 0, 0, &|api, mc| unsafe {
                (api.set_match_limit)(mc, lim);
            }, false, 64);
            diff_match(p, 0, s.as_bytes(), s.len(), 0, 0, &|api, mc| unsafe {
                (api.set_depth_limit)(mc, lim);
            }, false, 64);
            diff_match(p, 0, s.as_bytes(), s.len(), 0, 0, &|api, mc| unsafe {
                (api.set_heap_limit)(mc, lim);
            }, false, 64);
            diff_match(p, 0, s.as_bytes(), s.len(), 0, 0, &|api, mc| unsafe {
                (api.set_match_limit)(mc, lim);
            }, true, 64);
            diff_match(p, 0, s.as_bytes(), s.len(), 0, 0, &|api, mc| unsafe {
                (api.set_depth_limit)(mc, lim);
            }, true, 64);
        }
    }
    // \K in an assertion (row 35) and recursion loops (row 36)
    for (p, copts) in [
        ("(?=a\\K)ab", 0u32),
        ("(?<=a\\K)b", 0),
        ("(a)(?1)*", 0),
        ("(?:(?R))*a", 0),
        ("(?R)", 0),
        ("a(?1)?(b)", 0),
    ] {
        for s in ["ab", "aab", "", "b", "aaaa"] {
            for mo in [0, PCRE2_DISABLE_RECURSELOOP_CHECK] {
                let mut err = 0;
                let mut off = 0;
                let pb = p.as_bytes();
                let ok = unsafe {
                    let cd = (c().compile)(pb.as_ptr(), pb.len(), copts, &mut err, &mut off,
                                           std::ptr::null_mut());
                    let ok = !cd.is_null();
                    if ok {
                        (c().code_free)(cd);
                    }
                    ok
                };
                if ok {
                    diff_match(p, copts, s.as_bytes(), s.len(), 0, mo, &noop, false, 64);
                }
            }
        }
    }
}

// ------------------------------------------------------------- rows 48, 51-54
#[test]
fn dfa_specific_errors() {
    // rows 51/52: unsupported items
    for p in [
        "\\Ca", "a\\C", "(a)\\1", "(?<n>a)\\k<n>", "(a)(?1)", "(?(1)a|b)(c)", "(a)(?(1)b|c)",
        "(?(R)a|b)", "(?(R1)a|b)", "(?C1)a", "(*sr:\\w+)", "(?=a\\K)b", "(?>a)b",
    ] {
        for s in ["a", "aa", "ab", "abc", ""] {
            let mut err = 0;
            let mut off = 0;
            let pb = p.as_bytes();
            let ok = unsafe {
                let cd = (c().compile)(pb.as_ptr(), pb.len(), 0, &mut err, &mut off,
                                       std::ptr::null_mut());
                let ok = !cd.is_null();
                if ok {
                    (c().code_free)(cd);
                }
                ok
            };
            if ok {
                diff_match(p, 0, s.as_bytes(), s.len(), 0, 0, &noop, true, 64);
            }
        }
    }
    // row 54: workspace overflow with legal (>=20) but small sizes
    for ws in [20usize, 21, 24, 32, 40, 64, 128] {
        for p in [
            "(a|b|c|d|e|f|g|h|i|j|k|l|m|n|o|p|q|r|s|t)+",
            "(?:a?){0,40}b",
            "[a-z]{0,40}9",
            "((((((((((a))))))))))+",
        ] {
            diff_match(p, 0, b"abcdefghijabcdefghij", 20, 0, 0, &noop, true, ws);
        }
    }
    // row 48: DFA_RESTART without a preceding partial match
    unsafe {
        let mut out = Vec::new();
        for api in [c(), r()] {
            let mut err = 0;
            let mut off = 0;
            let code = (api.compile)(b"abcd".as_ptr(), 4, 0, &mut err, &mut off,
                                     std::ptr::null_mut());
            let md = (api.match_data_create_from_pattern)(code, std::ptr::null_mut());
            let mut v = Vec::new();
            // zeroed workspace + DFA_RESTART
            let mut ws = [0i32; 64];
            v.push((api.dfa_match)(code, b"abc".as_ptr(), 3, 0, PCRE2_DFA_RESTART, md,
                                   std::ptr::null_mut(), ws.as_mut_ptr(), 64));
            // a legitimate partial match then a restart continuation
            let mut ws2 = [0i32; 64];
            v.push((api.dfa_match)(code, b"ab".as_ptr(), 2, 0, PCRE2_PARTIAL_HARD, md,
                                   std::ptr::null_mut(), ws2.as_mut_ptr(), 64));
            v.push((api.dfa_match)(code, b"abcd".as_ptr(), 4, 2,
                                   PCRE2_DFA_RESTART | PCRE2_PARTIAL_HARD, md,
                                   std::ptr::null_mut(), ws2.as_mut_ptr(), 64));
            // garbage workspace
            let mut ws3 = [0x7fff_ffffi32; 64];
            v.push((api.dfa_match)(code, b"abc".as_ptr(), 3, 0, PCRE2_DFA_RESTART, md,
                                   std::ptr::null_mut(), ws3.as_mut_ptr(), 64));
            out.push(v);
            (api.match_data_free)(md);
            (api.code_free)(code);
        }
        assert_eq!(out[0], out[1], "DFA_RESTART handling differs");
        eprintln!("dfa restart codes = {:?}", out[0]);
    }
}
