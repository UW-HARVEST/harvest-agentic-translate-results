//! Phase C — compile-time error-path differential tests.
//!
//! One case per row of the `ERRORS.md` compile-error section. Every case is run
//! through BOTH shared objects; the test asserts
//!   * identical `errorcode`
//!   * identical `erroroffset`
//!   * identical NULL-ness of the returned `pcre2_code *`
//! and additionally that the C library really produces the error code the table
//! claims (so the table cannot silently drift away from the C source).
//!
//! Run with `PCRE2_PROBE=1 cargo test --release --test errors_compile -- --nocapture`
//! to dump the C library's actual (code, offset) for every case.

mod common;
use common::*;
use std::ffi::c_void;

/// A compile-error case.
pub struct Case {
    /// ERRORS.md row id.
    pub row: &'static str,
    /// Pattern bytes.
    pub pat: &'static [u8],
    /// pcre2_compile options.
    pub opts: u32,
    /// compile-context extra options.
    pub xopts: u32,
    /// Expected error code from the C library (0 = "record only, no expectation").
    pub expect: i32,
}

const fn c(row: &'static str, pat: &'static [u8], opts: u32, xopts: u32, expect: i32) -> Case {
    Case { row, pat, opts, xopts, expect }
}

/// Compile-error corpus, derived from every `ERRn` assignment in
/// `c_src/src/pcre2_compile*.c`.
#[rustfmt::skip]
pub static CASES: &[Case] = &[
    c("ERR1",   br"\",                          0, 0, 101),
    c("ERR2",   br"\c",                         0, 0, 102),
    c("ERR3",   br"\y",                         0, 0, 103),
    c("ERR4",   br"a{3,2}",                     0, 0, 104),
    c("ERR5",   br"a{65536}",                   0, 0, 105),
    c("ERR5b",  br"a{1,70000}",                 0, 0, 105),
    c("ERR6",   br"[a",                         0, 0, 106),
    c("ERR7",   br"[\A]",                       0, 0, 107),
    c("ERR7b",  br"[\Z]",                       0, 0, 107),
    c("ERR8",   br"[z-a]",                      0, 0, 108),
    c("ERR9",   br"*a",                         0, 0, 109),
    c("ERR9b",  br"a**",                        0, 0, 109),
    c("ERR11",  br"(?~)",                       0, 0, 111),
    c("ERR12",  br"[:alpha:]",                  0, 0, 112),
    c("ERR13",  br"[[.ch.]]",                   0, 0, 113),
    c("ERR14",  br"(a",                         0, 0, 114),
    c("ERR15",  br"(a)\2",                      0, 0, 115),
    c("ERR17",  br"a",             0x1000_0000, 0, 117),
    c("ERR18",  br"(?#",                        0, 0, 118),
    c("ERR22",  br"a)",                         0, 0, 122),
    c("ERR24",  br"(?('n'x)a)",                 0, 0, 124),
    c("ERR25",  br"(?<=a*)",                    0, 0, 125),
    c("ERR26",  br"\g{+0}",                     0, 0, 126),
    c("ERR26b", br"(?+0)",                      0, 0, 126),
    c("ERR27",  br"(x)(?(1)a|b|c)",             0, 0, 127),
    c("ERR28",  br"(?(?i)a)",                   0, 0, 128),
    c("ERR29",  br"(?+x)",                      0, 0, 129),
    c("ERR30",  br"[[:foo:]]",                  0, 0, 130),
    c("ERR34",  br"\x{110000}",                 0, 0, 134),
    c("ERR34b", br"\o{4000000}",                0, 0, 134),
    c("ERR25b", br"(?<=a*)x",                    0, 0, 125),
    c("ERR36",  br"(?<=\C)",              o::UTF, 0, 136),
    c("ERR37",  br"\L",                         0, 0, 137),
    c("ERR37b", br"\u",                         0, 0, 137),
    c("ERR38",  br"(?C256)",                    0, 0, 138),
    c("ERR39",  br"(?C1",                       0, 0, 139),
    c("ERR40",  br"(*MARK:a\db)",  o::ALT_VERBNAMES, 0, 140),
    c("ERR41",  br"(?P~)",                       0, 0, 141),
    c("ERR42",  br"(?<abc",                      0, 0, 142),
    c("ERR43",  br"(?<a>x)(?<a>y)",              0, 0, 143),
    c("ERR44",  br"(?<1a>x)",                    0, 0, 144),
    c("ERR46",  br"\p{",                         0, 0, 146),
    c("ERR47",  br"\p{Foo}",                     0, 0, 147),
    c("ERR50",  br"[\d-z]",                      0, 0, 150),
    c("ERR51",  br"\400",                        0, 0, 151),
    c("ERR54",  br"(?(DEFINE)a|b)",              0, 0, 154),
    c("ERR55",  br"\o1",                         0, 0, 155),
    c("ERR57",  br"\g",                          0, 0, 157),
    c("ERR58",  br"(?R:",                        0, 0, 158),
    c("ERR60",  br"(*FOO)",                      0, 0, 160),
    c("ERR61",  br"\g{99999999}",                0, 0, 161),
    c("ERR62",  br"(?<>x)",                      0, 0, 162),
    c("ERR64",  br"\o{19}",                      0, 0, 164),
    c("ERR65",  br"(?|(?<a>x)|(?<b>y))",         0, 0, 165),
    c("ERR66",  br"(*MARK)",                     0, 0, 166),
    c("ERR67",  br"\x{z}",                       0, 0, 167),
    c("ERR68",  b"\\c\x80",                      0, 0, 168),
    c("ERR69",  br"\k",                          0, 0, 169),
    c("ERR71",  br"[\N]",                        0, 0, 171),
    c("ERR73",  br"\x{d800}",              o::UTF, 0, 173),
    c("ERR74",  br"a",     o::UTF | o::NEVER_UTF, 0, 174),
    c("ERR75",  br"a",     o::UCP | o::NEVER_UCP, 0, 175),
    c("ERR77",  br"\u{110000}", o::ALT_BSUX, o::X_ALT_BSUX, 177),
    c("ERR78",  br"\x{}",                        0, 0, 178),
    c("ERR78b", br"\o{}",                        0, 0, 178),
    c("ERR79",  br"(?(VERSION>=x)a)",            0, 0, 179),
    c("ERR81",  br"(?C{abc",                     0, 0, 181),
    c("ERR82",  br"(?C~abc~)",                   0, 0, 182),
    c("ERR83",  br"\C",     o::NEVER_BACKSLASH_C, 0, 183),
    c("ERR92",  br"a",   o::LITERAL | o::EXTENDED, 0, 192),
    c("ERR93",  br"\N{U+41}",                    0, 0, 193),
    c("ERR94",  br"(?i-m-s:a)",                  0, 0, 194),
    c("ERR95",  br"(*pla_foo:a)",                0, 0, 195),
    c("ERR98",  br"\0",              0, o::X_NO_BS0, 198),
    c("ERR99",  br"(?=\Ka)",                     0, 0, 199),
    c("ERR102", br"\400",       0, o::X_PYTHON_OCTAL, 202),
    c("ERR103", br"(?C)",         0, o::X_NEVER_CALLOUT, 203),
    c("ERR104", br"a",             0, o::X_TURKISH_CASING, 204),
    c("ERR105", br"a",       o::UCP, o::X_TURKISH_CASING, 205),
    c("ERR106", br"a", o::UTF, o::X_TURKISH_CASING | o::X_CASELESS_RESTRICT, 206),
    c("ERR108", br"[a&&&b]",  o::ALT_EXTENDED_CLASS, 0, 208),
    c("ERR109", br"(?[&&[a]])",                  0, 0, 209),
    c("ERR109b",br"[&&a]",    o::ALT_EXTENDED_CLASS, 0, 209),
    c("ERR110", br"[a&&]",    o::ALT_EXTENDED_CLASS, 0, 210),
    c("ERR111", br"[a--b&&c]",o::ALT_EXTENDED_CLASS, 0, 211),
    c("ERR112", br"[a[b]",    o::ALT_EXTENDED_CLASS, 0, 212),
    c("ERR113", br"(?[[a][b]])",                 0, 0, 213),
    c("ERR114", br"(?[])",                       0, 0, 214),
    c("ERR115", br"(?[[a]]",                     0, 0, 215),
    c("ERR116", br"(?[a])",                      0, 0, 216),
    c("ERR117", br"(*scan_substring:(!)a)",      0, 0, 217),
    c("ERR118", br"(*scan_substring:a)",         0, 0, 218),
    c("ERR119", br"\g{1",                        0, 0, 219),
    // Perl-extended-class edge branches (ERR14 / ERR22 inside `(?[...]`).
    c("ERR14-eclass", br"(?[([a]])",             0, 0, 114),
    c("ERR14-eclass2", br"(?[(]",                0, 0, 114),
    c("ERR22-eclass", br"(?[[a]])x)",            0, 0, 122),
    // Extra generic-boundary / one-past-range cases.
    c("BND-utf-esc",  b"\\x{7fffffff}",          0, 0, 134),
    c("BND-quant-0",  br"a{0}",                  0, 0, 100),
    c("BND-emptyclass", br"[]",                  0, 0, 106),
    c("BND-emptyclass-ok", br"[]", o::ALLOW_EMPTY_CLASS, 0, 100),
    c("BND-nested-neg", br"[^]]",                0, 0, 100),
    c("BND-bad-verb-arg", br"(*ACCEPT:x)",       0, 0, 100),
    c("BND-recurse-self", br"(a(?1))",           0, 0, 100),
    c("BND-cond-nonexist", br"(?(99)a)",         0, 0, 115),
    c("BND-name-ref",  br"\k<nope>",             0, 0, 115),
    c("BND-lb-var",    br"(?<=a|bc)",            0, 0, 100),
];

fn run(cases: &[Case], probe: bool) {
    let p = libs();
    let mut failures = Vec::new();
    for cs in cases {
        // A fresh compile context per case; extra options need one.
        let cc_c = unsafe { (p.c.compile_context_create)(std::ptr::null_mut()) };
        let cc_r = unsafe { (p.r.compile_context_create)(std::ptr::null_mut()) };
        assert!(!cc_c.is_null() && !cc_r.is_null());
        if cs.xopts != 0 {
            let a = unsafe { (p.c.set_compile_extra_options)(cc_c, cs.xopts) };
            let b = unsafe { (p.r.set_compile_extra_options)(cc_r, cs.xopts) };
            assert_eq!(a, b, "set_compile_extra_options rc [{}]", cs.row);
        }

        let mut ec_c = 0i32;
        let mut eo_c = 0usize;
        let mut ec_r = 0i32;
        let mut eo_r = 0usize;
        let code_c = unsafe {
            (p.c.compile)(cs.pat.as_ptr(), cs.pat.len(), cs.opts, &mut ec_c, &mut eo_c, cc_c)
        };
        let code_r = unsafe {
            (p.r.compile)(cs.pat.as_ptr(), cs.pat.len(), cs.opts, &mut ec_r, &mut eo_r, cc_r)
        };

        if probe {
            println!(
                "{:<20} pat={:<26} opts={:#010x} xopts={:#010x} -> C=({}, {}) RUST=({}, {})",
                cs.row,
                format!("{:?}", String::from_utf8_lossy(cs.pat)),
                cs.opts,
                cs.xopts,
                ec_c,
                eo_c,
                ec_r,
                eo_r
            );
        }

        if code_c.is_null() != code_r.is_null() {
            failures.push(format!("{}: null-ness differs (C {} / R {})", cs.row, code_c.is_null(), code_r.is_null()));
        }
        if (ec_c, eo_c) != (ec_r, eo_r) {
            failures.push(format!(
                "{}: pat={:?} opts={:#x} xopts={:#x}: C=({},{}) RUST=({},{})",
                cs.row,
                String::from_utf8_lossy(cs.pat),
                cs.opts,
                cs.xopts,
                ec_c,
                eo_c,
                ec_r,
                eo_r
            ));
        }
        if cs.expect != 0 && ec_c != cs.expect {
            failures.push(format!(
                "{}: ERRORS.md says C should give {} but C gave {} (trigger drifted)",
                cs.row, cs.expect, ec_c
            ));
        }
        // If both compiled, the compiled bytes must be identical too.
        if !code_c.is_null() {
            let cp = CodePair { c: code_c, r: code_r };
            cmp_all_pattern_info(p, &cp, cs.row);
            cmp_compiled_bytes(p, &cp, cs.row);
            unsafe {
                (p.c.code_free)(code_c);
                (p.r.code_free)(code_r);
            }
        }
        unsafe {
            (p.c.compile_context_free)(cc_c);
            (p.r.compile_context_free)(cc_r);
        }
    }
    if !failures.is_empty() {
        panic!("{} compile-error divergence(s):\n{}", failures.len(), failures.join("\n"));
    }
}

#[test]
fn compile_error_corpus() {
    run(CASES, std::env::var("PCRE2_PROBE").is_ok());
}

// ---------------------------------------------------------------------------
// Cases that need extra API setup rather than just options.
// ---------------------------------------------------------------------------

#[test]
fn err16_null_pattern_with_nonzero_length() {
    let p = libs();
    let mut ec_c = 0;
    let mut eo_c = 0;
    let mut ec_r = 0;
    let mut eo_r = 0;
    let cc = unsafe { (p.c.compile)(std::ptr::null(), 5, 0, &mut ec_c, &mut eo_c, std::ptr::null_mut()) };
    let cr = unsafe { (p.r.compile)(std::ptr::null(), 5, 0, &mut ec_r, &mut eo_r, std::ptr::null_mut()) };
    assert!(cc.is_null() && cr.is_null());
    assert_eq!((ec_c, eo_c), (ec_r, eo_r));
    assert_eq!(ec_c, 116, "ERR16 NULL_PATTERN");
}

#[test]
fn err16_null_pattern_zero_length_is_ok() {
    // C accepts a NULL pattern when the length is 0.
    let p = libs();
    let mut ec_c = 0;
    let mut eo_c = 0;
    let mut ec_r = 0;
    let mut eo_r = 0;
    let cc = unsafe { (p.c.compile)(std::ptr::null(), 0, 0, &mut ec_c, &mut eo_c, std::ptr::null_mut()) };
    let cr = unsafe { (p.r.compile)(std::ptr::null(), 0, 0, &mut ec_r, &mut eo_r, std::ptr::null_mut()) };
    assert_eq!(cc.is_null(), cr.is_null());
    assert_eq!((ec_c, eo_c), (ec_r, eo_r));
    if !cc.is_null() {
        let cp = CodePair { c: cc, r: cr };
        cmp_compiled_bytes(p, &cp, "null-pattern-len0");
        free_code_pair(p, cp);
    }
}

#[test]
fn err120_null_erroroffset() {
    let p = libs();
    let mut ec_c = 0;
    let mut ec_r = 0;
    let cc = unsafe {
        (p.c.compile)(b"a".as_ptr(), 1, 0, &mut ec_c, std::ptr::null_mut(), std::ptr::null_mut())
    };
    let cr = unsafe {
        (p.r.compile)(b"a".as_ptr(), 1, 0, &mut ec_r, std::ptr::null_mut(), std::ptr::null_mut())
    };
    assert!(cc.is_null() && cr.is_null());
    assert_eq!(ec_c, ec_r);
    assert_eq!(ec_c, 220, "ERR120 NULL_ERROROFFSET");
}

#[test]
fn null_errorcode_ptr_is_rejected_identically() {
    // pcre2_compile with errorptr == NULL returns NULL in both.
    let p = libs();
    let mut eo_c = 0usize;
    let mut eo_r = 0usize;
    let cc = unsafe {
        (p.c.compile)(b"a".as_ptr(), 1, 0, std::ptr::null_mut(), &mut eo_c, std::ptr::null_mut())
    };
    let cr = unsafe {
        (p.r.compile)(b"a".as_ptr(), 1, 0, std::ptr::null_mut(), &mut eo_r, std::ptr::null_mut())
    };
    assert_eq!(cc.is_null(), cr.is_null());
    if !cc.is_null() {
        unsafe {
            (p.c.code_free)(cc);
            (p.r.code_free)(cr);
        }
    }
}

#[test]
fn err19_parentheses_nest_too_deep() {
    let p = libs();
    // PARENS_NEST_LIMIT default is 250.
    for depth in [249usize, 250, 251, 400] {
        let mut pat = Vec::new();
        pat.extend(std::iter::repeat(b'(').take(depth));
        pat.push(b'a');
        pat.extend(std::iter::repeat(b')').take(depth));
        let label = format!("nest{}", depth);
        match compile_both(p, &pat, pat.len(), 0, std::ptr::null_mut(), std::ptr::null_mut(), &label) {
            Ok(cp) => {
                cmp_compiled_bytes(p, &cp, &label);
                free_code_pair(p, cp);
            }
            Err((ec, _)) => assert_eq!(ec, 119, "expected ERR19 at depth {}", depth),
        }
    }
    // And with an explicitly lowered limit.
    for limit in [0u32, 1, 2, 5] {
        let cc_c = unsafe { (p.c.compile_context_create)(std::ptr::null_mut()) };
        let cc_r = unsafe { (p.r.compile_context_create)(std::ptr::null_mut()) };
        unsafe {
            assert_eq!((p.c.set_parens_nest_limit)(cc_c, limit), (p.r.set_parens_nest_limit)(cc_r, limit));
        }
        let pat = b"((((((a))))))";
        let label = format!("parens_limit{}", limit);
        let _ = compile_both(p, pat, pat.len(), 0, cc_c, cc_r, &label).map(|cp| {
            cmp_compiled_bytes(p, &cp, &label);
            free_code_pair(p, cp)
        });
        unsafe {
            (p.c.compile_context_free)(cc_c);
            (p.r.compile_context_free)(cc_r);
        }
    }
}

#[test]
fn err88_pattern_string_too_long() {
    let p = libs();
    for maxlen in [0usize, 1, 2, 4, 5, 6] {
        let cc_c = unsafe { (p.c.compile_context_create)(std::ptr::null_mut()) };
        let cc_r = unsafe { (p.r.compile_context_create)(std::ptr::null_mut()) };
        unsafe {
            assert_eq!(
                (p.c.set_max_pattern_length)(cc_c, maxlen),
                (p.r.set_max_pattern_length)(cc_r, maxlen)
            );
        }
        let pat = b"abcde";
        let label = format!("maxpatlen{}", maxlen);
        match compile_both(p, pat, pat.len(), 0, cc_c, cc_r, &label) {
            Ok(cp) => {
                assert!(maxlen >= 5);
                cmp_compiled_bytes(p, &cp, &label);
                free_code_pair(p, cp);
            }
            Err((ec, _)) => assert_eq!(ec, 188, "ERR88 expected for maxlen {}", maxlen),
        }
        unsafe {
            (p.c.compile_context_free)(cc_c);
            (p.r.compile_context_free)(cc_r);
        }
    }
}

#[test]
fn err101_pattern_compiled_size_too_big() {
    let p = libs();
    for maxlen in [0usize, 1, 8, 32, 1 << 20] {
        let cc_c = unsafe { (p.c.compile_context_create)(std::ptr::null_mut()) };
        let cc_r = unsafe { (p.r.compile_context_create)(std::ptr::null_mut()) };
        unsafe {
            assert_eq!(
                (p.c.set_max_pattern_compiled_length)(cc_c, maxlen),
                (p.r.set_max_pattern_compiled_length)(cc_r, maxlen)
            );
        }
        let pat = b"(abc)+d[e-g]{2,4}";
        let label = format!("maxcompiled{}", maxlen);
        match compile_both(p, pat, pat.len(), 0, cc_c, cc_r, &label) {
            Ok(cp) => {
                cmp_compiled_bytes(p, &cp, &label);
                free_code_pair(p, cp);
            }
            Err((ec, _)) => assert_eq!(ec, 201, "ERR101 expected for maxcompiled {}", maxlen),
        }
        unsafe {
            (p.c.compile_context_free)(cc_c);
            (p.r.compile_context_free)(cc_r);
        }
    }
}

#[test]
fn err100_max_varlookbehind_exceeded() {
    let p = libs();
    for limit in [0u32, 1, 2, 3, 255, 65535] {
        let cc_c = unsafe { (p.c.compile_context_create)(std::ptr::null_mut()) };
        let cc_r = unsafe { (p.r.compile_context_create)(std::ptr::null_mut()) };
        unsafe {
            assert_eq!(
                (p.c.set_max_varlookbehind)(cc_c, limit),
                (p.r.set_max_varlookbehind)(cc_r, limit)
            );
        }
        for pat in [&b"(?<=a|bc)"[..], &b"(?<=ab|cd)"[..], &b"(?<=a{2,4})"[..], &b"(?<=abc)"[..]] {
            let label = format!("varlb{}:{:?}", limit, String::from_utf8_lossy(pat));
            match compile_both(p, pat, pat.len(), 0, cc_c, cc_r, &label) {
                Ok(cp) => {
                    cmp_compiled_bytes(p, &cp, &label);
                    free_code_pair(p, cp);
                }
                Err(_) => {}
            }
        }
        unsafe {
            (p.c.compile_context_free)(cc_c);
            (p.r.compile_context_free)(cc_r);
        }
    }
}

#[test]
fn err33_recursion_guard_rejects() {
    // pcre2_set_compile_recursion_guard: a non-zero return aborts the compile
    // with ERR33 (PARENTHESES_STACK_CHECK).
    unsafe extern "C" fn guard_always_fail(_depth: u32, _data: *mut c_void) -> i32 {
        1
    }
    unsafe extern "C" fn guard_ok(_depth: u32, _data: *mut c_void) -> i32 {
        0
    }
    unsafe extern "C" fn guard_deep(depth: u32, _data: *mut c_void) -> i32 {
        if depth > 3 { 1 } else { 0 }
    }
    let p = libs();
    for (name, g) in [
        ("fail", guard_always_fail as unsafe extern "C" fn(u32, *mut c_void) -> i32),
        ("ok", guard_ok),
        ("deep", guard_deep),
    ] {
        let cc_c = unsafe { (p.c.compile_context_create)(std::ptr::null_mut()) };
        let cc_r = unsafe { (p.r.compile_context_create)(std::ptr::null_mut()) };
        unsafe {
            assert_eq!(
                (p.c.set_compile_recursion_guard)(cc_c, Some(g), std::ptr::null_mut()),
                (p.r.set_compile_recursion_guard)(cc_r, Some(g), std::ptr::null_mut())
            );
        }
        for pat in [&b"a"[..], &b"(a)"[..], &b"((((a))))"[..], &b"((((((a))))))"[..]] {
            let label = format!("guard-{}:{:?}", name, String::from_utf8_lossy(pat));
            if let Ok(cp) = compile_both(p, pat, pat.len(), 0, cc_c, cc_r, &label) {
                cmp_compiled_bytes(p, &cp, &label);
                free_code_pair(p, cp);
            }
        }
        unsafe {
            (p.c.compile_context_free)(cc_c);
            (p.r.compile_context_free)(cc_r);
        }
    }
}

#[test]
fn err21_heap_failed_via_failing_allocator() {
    // A general context whose malloc always fails must make pcre2_compile
    // return ERR21 (HEAP_FAILED) in both libraries.
    unsafe extern "C" fn nomalloc(_n: usize, _d: *mut c_void) -> *mut c_void {
        std::ptr::null_mut()
    }
    unsafe extern "C" fn nofree(_p: *mut c_void, _d: *mut c_void) {}
    let p = libs();
    let gc_c = unsafe { (p.c.general_context_create)(Some(nomalloc), Some(nofree), std::ptr::null_mut()) };
    let gc_r = unsafe { (p.r.general_context_create)(Some(nomalloc), Some(nofree), std::ptr::null_mut()) };
    // general_context_create itself allocates through the supplied malloc.
    assert_eq!(gc_c.is_null(), gc_r.is_null(), "general_context_create null-ness");
    if gc_c.is_null() {
        return; // both refused; nothing more to compare
    }
    let cc_c = unsafe { (p.c.compile_context_create)(gc_c) };
    let cc_r = unsafe { (p.r.compile_context_create)(gc_r) };
    assert_eq!(cc_c.is_null(), cc_r.is_null());
    if !cc_c.is_null() {
        let mut ec_c = 0;
        let mut eo_c = 0;
        let mut ec_r = 0;
        let mut eo_r = 0;
        let a = unsafe { (p.c.compile)(b"abc".as_ptr(), 3, 0, &mut ec_c, &mut eo_c, cc_c) };
        let b = unsafe { (p.r.compile)(b"abc".as_ptr(), 3, 0, &mut ec_r, &mut eo_r, cc_r) };
        assert!(a.is_null() && b.is_null());
        assert_eq!((ec_c, eo_c), (ec_r, eo_r));
        assert_eq!(ec_c, 121, "ERR21 HEAP_FAILED");
    }
}

#[test]
fn err48_subpattern_name_too_long_boundary() {
    let p = libs();
    // MAX_NAME_SIZE == 128
    for n in [127usize, 128, 129, 200] {
        let mut pat = Vec::from(&b"(?<"[..]);
        pat.extend(std::iter::repeat(b'a').take(n));
        pat.extend_from_slice(b">x)");
        let label = format!("namelen{}", n);
        match compile_both(p, &pat, pat.len(), 0, std::ptr::null_mut(), std::ptr::null_mut(), &label) {
            Ok(cp) => {
                assert!(n <= 128);
                cmp_compiled_bytes(p, &cp, &label);
                free_code_pair(p, cp);
            }
            Err((ec, _)) => assert_eq!(ec, 148, "ERR48 expected at name length {}", n),
        }
    }
}

#[test]
fn err76_verb_name_too_long_boundary() {
    let p = libs();
    for n in [254usize, 255, 256, 300] {
        let mut pat = Vec::from(&b"(*MARK:"[..]);
        pat.extend(std::iter::repeat(b'a').take(n));
        pat.extend_from_slice(b")");
        let label = format!("verbname{}", n);
        let _ = compile_both(p, &pat, pat.len(), 0, std::ptr::null_mut(), std::ptr::null_mut(), &label)
            .map(|cp| {
                cmp_compiled_bytes(p, &cp, &label);
                free_code_pair(p, cp)
            });
    }
}

#[test]
fn err72_callout_string_too_long_boundary() {
    let p = libs();
    for n in [0usize, 1, 1000, 60000, 70000] {
        let mut pat = Vec::from(&b"(?C{"[..]);
        pat.extend(std::iter::repeat(b'x').take(n));
        pat.extend_from_slice(b"})a");
        let label = format!("calloutstr{}", n);
        let _ = compile_both(p, &pat, pat.len(), 0, std::ptr::null_mut(), std::ptr::null_mut(), &label)
            .map(|cp| {
                cmp_compiled_bytes(p, &cp, &label);
                free_code_pair(p, cp)
            });
    }
}

#[test]
fn err97_too_many_captures_and_err49_too_many_names() {
    let p = libs();
    // Very large numbers of capture groups. Kept modest so the test stays fast,
    // but crosses the interesting internal thresholds for name-table growth.
    for n in [1usize, 2, 100, 1000, 10001] {
        let mut pat = Vec::new();
        for i in 0..n {
            pat.extend_from_slice(format!("(?<n{}>a)", i).as_bytes());
        }
        let label = format!("names{}", n);
        match compile_both(p, &pat, pat.len(), 0, std::ptr::null_mut(), std::ptr::null_mut(), &label) {
            Ok(cp) => {
                cmp_all_pattern_info(p, &cp, &label);
                cmp_compiled_bytes(p, &cp, &label);
                free_code_pair(p, cp);
            }
            Err((ec, _)) => {
                assert!(ec == 149 || ec == 197 || ec == 120, "unexpected {} for {}", ec, label);
            }
        }
    }
}

#[test]
fn err107_eclass_nest_too_deep() {
    let p = libs();
    for depth in [1usize, 10, 100, 200, 300, 1000] {
        let mut pat = Vec::from(&b"(?["[..]);
        for _ in 0..depth {
            pat.extend_from_slice(b"[");
        }
        pat.extend_from_slice(b"a");
        for _ in 0..depth {
            pat.extend_from_slice(b"]");
        }
        pat.extend_from_slice(b"])");
        let label = format!("eclassnest{}", depth);
        let _ = compile_both(p, &pat, pat.len(), 0, std::ptr::null_mut(), std::ptr::null_mut(), &label)
            .map(|cp| {
                cmp_compiled_bytes(p, &cp, &label);
                free_code_pair(p, cp)
            });
    }
}

#[test]
fn err84_query_barjx_nest_too_deep() {
    let p = libs();
    for depth in [1usize, 100, 200, 255, 256, 300] {
        let mut pat = Vec::new();
        for _ in 0..depth {
            pat.extend_from_slice(b"(?|");
        }
        pat.extend_from_slice(b"a");
        for _ in 0..depth {
            pat.extend_from_slice(b")");
        }
        let label = format!("barjx{}", depth);
        let _ = compile_both(p, &pat, pat.len(), 0, std::ptr::null_mut(), std::ptr::null_mut(), &label)
            .map(|cp| {
                cmp_compiled_bytes(p, &cp, &label);
                free_code_pair(p, cp)
            });
    }
}

#[test]
fn err87_lookbehind_too_long() {
    let p = libs();
    for n in [10usize, 255, 256, 65535, 65536] {
        let pat = format!("(?<=a{{{}}})b", n).into_bytes();
        let label = format!("lbtoolong{}", n);
        let _ = compile_both(p, &pat, pat.len(), 0, std::ptr::null_mut(), std::ptr::null_mut(), &label)
            .map(|cp| {
                cmp_compiled_bytes(p, &cp, &label);
                free_code_pair(p, cp)
            });
    }
}

#[test]
fn every_single_option_bit_alone() {
    // Out-of-range / undefined option bits across the FFI boundary: C accepts
    // any uint32_t, so all 32 bits must be probed individually.
    let p = libs();
    for bit in 0..32u32 {
        let opt = 1u32 << bit;
        let label = format!("optbit{}", bit);
        let _ = compile_both(p, b"a(b)c", 5, opt, std::ptr::null_mut(), std::ptr::null_mut(), &label)
            .map(|cp| {
                cmp_all_pattern_info(p, &cp, &label);
                cmp_compiled_bytes(p, &cp, &label);
                free_code_pair(p, cp)
            });
    }
    // All bits set at once, and the full complement of valid bits.
    for opt in [u32::MAX, 0xF000_0000, 0x0FFF_FFFF] {
        let label = format!("opts{:#x}", opt);
        let _ = compile_both(p, b"a(b)c", 5, opt, std::ptr::null_mut(), std::ptr::null_mut(), &label)
            .map(|cp| free_code_pair(p, cp));
    }
}

#[test]
fn every_single_extra_option_bit_alone() {
    let p = libs();
    for bit in 0..32u32 {
        let x = 1u32 << bit;
        let cc_c = unsafe { (p.c.compile_context_create)(std::ptr::null_mut()) };
        let cc_r = unsafe { (p.r.compile_context_create)(std::ptr::null_mut()) };
        let a = unsafe { (p.c.set_compile_extra_options)(cc_c, x) };
        let b = unsafe { (p.r.set_compile_extra_options)(cc_r, x) };
        assert_eq!(a, b, "set_compile_extra_options({:#x}) rc", x);
        for pat in [&b"a(b)c"[..], &b"\\0"[..], &b"[\\d]"[..], &b"\\w\\s\\d"[..], &b"(?C)x"[..]] {
            let label = format!("xoptbit{}:{:?}", bit, String::from_utf8_lossy(pat));
            let _ = compile_both(p, pat, pat.len(), 0, cc_c, cc_r, &label).map(|cp| {
                cmp_all_pattern_info(p, &cp, &label);
                cmp_compiled_bytes(p, &cp, &label);
                free_code_pair(p, cp)
            });
        }
        unsafe {
            (p.c.compile_context_free)(cc_c);
            (p.r.compile_context_free)(cc_r);
        }
    }
}

/// Generated oversize / over-complex patterns: ERR20 (PATTERN_TOO_LARGE, i.e.
/// compiled size > MAX_PATTERN_SIZE = 1<<16 at LINK_SIZE 2), ERR86
/// (PATTERN_TOO_COMPLICATED, i.e. a single item overruns COMPILE_WORK_SIZE) and
/// ERR35 (LOOKBEHIND_TOO_COMPLICATED, > 2000 length-computation steps).
#[test]
fn generated_oversize_patterns() {
    let p = libs();
    let probe = std::env::var("PCRE2_PROBE").is_ok();

    let mut pats: Vec<(String, Vec<u8>, u32)> = Vec::new();

    // ERR20: lots of literal code units.
    for n in [1000usize, 30000, 40000, 70000] {
        pats.push((format!("many-literals-{}", n), vec![b'a'; n], 0));
    }
    // ERR20: lots of alternatives.
    for n in [100usize, 5000, 20000] {
        let mut v = Vec::new();
        for i in 0..n {
            if i > 0 {
                v.push(b'|');
            }
            v.push(b'a');
        }
        pats.push((format!("many-alts-{}", n), v, 0));
    }
    // ERR86: one huge XCLASS item.
    for n in [50usize, 500, 3000, 6000] {
        let mut v = Vec::from(&b"["[..]);
        for i in 0..n {
            v.extend_from_slice(format!("\\x{{{:x}}}", 0x200 + i * 3).as_bytes());
        }
        v.push(b']');
        pats.push((format!("huge-xclass-{}", n), v, o::UTF));
    }
    // ERR35: many (?| groups inside a lookbehind (their lengths cannot be cached).
    for n in [10usize, 200, 700, 1500, 3000] {
        let mut v = Vec::from(&b"(?<="[..]);
        for _ in 0..n {
            v.extend_from_slice(b"(?|a|b)");
        }
        v.extend_from_slice(b")x");
        pats.push((format!("lb-complicated-{}", n), v, 0));
    }
    // Deeply nested groups without hitting the parens limit (uses (?: which does
    // not consume a capture number).
    for n in [100usize, 249, 250, 251] {
        let mut v = Vec::new();
        for _ in 0..n {
            v.extend_from_slice(b"(?:");
        }
        v.push(b'a');
        for _ in 0..n {
            v.push(b')');
        }
        pats.push((format!("nested-noncapture-{}", n), v, 0));
    }

    for (name, pat, opts) in pats {
        let mut ec_c = 0i32;
        let mut eo_c = 0usize;
        let mut ec_r = 0i32;
        let mut eo_r = 0usize;
        let code_c =
            unsafe { (p.c.compile)(pat.as_ptr(), pat.len(), opts, &mut ec_c, &mut eo_c, std::ptr::null_mut()) };
        let code_r =
            unsafe { (p.r.compile)(pat.as_ptr(), pat.len(), opts, &mut ec_r, &mut eo_r, std::ptr::null_mut()) };
        if probe {
            println!("{:<26} len={:<7} -> C=({}, {}) RUST=({}, {})", name, pat.len(), ec_c, eo_c, ec_r, eo_r);
        }
        assert_eq!(code_c.is_null(), code_r.is_null(), "{}: null-ness", name);
        assert_eq!((ec_c, eo_c), (ec_r, eo_r), "{}: (errorcode, erroroffset)", name);
        if !code_c.is_null() {
            let cp = CodePair { c: code_c, r: code_r };
            cmp_all_pattern_info(p, &cp, &name);
            cmp_compiled_bytes(p, &cp, &name);
            free_code_pair(p, cp);
        }
    }
}
