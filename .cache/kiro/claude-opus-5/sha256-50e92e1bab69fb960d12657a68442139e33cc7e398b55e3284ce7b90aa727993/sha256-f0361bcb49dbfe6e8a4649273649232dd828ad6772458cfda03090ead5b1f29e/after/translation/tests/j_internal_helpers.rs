//! CONFIGS.md rows 150–155 — the internal compiler/matcher helpers that have no
//! callable standalone signature (`_pcre2_study`, `_pcre2_auto_possessify`,
//! `_pcre2_check_escape`, `_pcre2_xclass`, `_pcre2_eclass`,
//! `_pcre2_update_classbits`, `_pcre2_compile_class_{,not_}nested`, and the
//! name-table helpers). They are driven through the public API in the exact
//! configurations that reach them, and the results are compared through the
//! deterministic serialized image plus matching.
mod common;
use common::corpus::{RAW_SUBJECTS, SUBJECTS};
use common::*;
use std::ffi::c_int;

fn zp(p: &str) -> Vec<u8> {
    let mut v = p.as_bytes().to_vec();
    v.push(0);
    v
}
fn pad(s: &[u8]) -> Vec<u8> {
    let mut v = s.to_vec();
    v.extend_from_slice(&[0u8; 16]);
    v
}

/// Compile + serialize + info + match over a subject set, in both libraries.
fn full_check(pat: &str, copts: u32, xopts: u32, note: &str) {
    let (c, r) = pair();
    let cc = unsafe { (c.compile_context_create)(std::ptr::null_mut()) };
    let rc = unsafe { (r.compile_context_create)(std::ptr::null_mut()) };
    unsafe { (c.set_compile_extra_options)(cc, xopts) };
    unsafe { (r.set_compile_extra_options)(rc, xopts) };
    let v = zp(pat);
    let mut ce: c_int = 0;
    let mut co: Sz = 0;
    let mut re: c_int = 0;
    let mut ro: Sz = 0;
    let a = unsafe { (c.compile)(v.as_ptr(), pat.len(), copts, &mut ce, &mut co, cc) };
    let b = unsafe { (r.compile)(v.as_ptr(), pat.len(), copts, &mut re, &mut ro, rc) };
    assert_eq!(
        (ce, co, a.is_null()),
        (re, ro, b.is_null()),
        "[{note}] compile {pat:?} copts=0x{copts:08x} xopts=0x{xopts:08x}"
    );
    if !a.is_null() {
        assert_eq!(
            unsafe { serialize_bytes(c, a) },
            unsafe { serialize_bytes(r, b) },
            "[{note}] serialized {pat:?} copts=0x{copts:08x} xopts=0x{xopts:08x}"
        );
        assert_eq!(
            unsafe { info_dump(c, a) },
            unsafe { info_dump(r, b) },
            "[{note}] info {pat:?}"
        );
        let mut subs: Vec<Vec<u8>> = SUBJECTS.iter().map(|s| s.as_bytes().to_vec()).collect();
        subs.extend(RAW_SUBJECTS.iter().map(|s| s.to_vec()));
        let cmd = unsafe { (c.match_data_create)(16, std::ptr::null_mut()) };
        let rmd = unsafe { (r.match_data_create)(16, std::ptr::null_mut()) };
        let mut cws = [0 as c_int; 1000];
        let mut rws = [0 as c_int; 1000];
        for s in &subs {
            if !unsafe { subject_domain_is_defined(c, a, s, 0, 0) } {
                continue;
            }
            let sp = pad(s);
            for mopts in [0u32, PCRE2_ANCHORED, PCRE2_PARTIAL_HARD, PCRE2_NOTBOL] {
                let x = unsafe {
                    (c.pcre2_match)(a, sp.as_ptr(), s.len(), 0, mopts, cmd, std::ptr::null_mut())
                };
                let y = unsafe {
                    (r.pcre2_match)(b, sp.as_ptr(), s.len(), 0, mopts, rmd, std::ptr::null_mut())
                };
                assert_eq!(
                    unsafe { md_dump(c, cmd, x) },
                    unsafe { md_dump(r, rmd, y) },
                    "[{note}] match {pat:?} subj={s:02x?} mopts=0x{mopts:x}"
                );
                let x = unsafe {
                    (c.dfa_match)(a, sp.as_ptr(), s.len(), 0, mopts, cmd, std::ptr::null_mut(), cws.as_mut_ptr(), 1000)
                };
                let y = unsafe {
                    (r.dfa_match)(b, sp.as_ptr(), s.len(), 0, mopts, rmd, std::ptr::null_mut(), rws.as_mut_ptr(), 1000)
                };
                assert_eq!(
                    unsafe { md_dump(c, cmd, x) },
                    unsafe { md_dump(r, rmd, y) },
                    "[{note}] dfa {pat:?} subj={s:02x?} mopts=0x{mopts:x}"
                );
            }
        }
        unsafe { (c.match_data_free)(cmd) };
        unsafe { (r.match_data_free)(rmd) };
        unsafe { (c.code_free)(a) };
        unsafe { (r.code_free)(b) };
    }
    unsafe { (c.compile_context_free)(cc) };
    unsafe { (r.compile_context_free)(rc) };
}

// ---------------------------- row 151: _pcre2_auto_possessify on/off ----------

const POSSESS_PATTERNS: &[&str] = &[
    "a+b", "a*b", "a?b", "a{2,4}b", "\\d+\\D", "\\w+\\W", "\\s+\\S", "[a-z]+[0-9]",
    "a+[^a]", "\\p{L}+\\p{N}", ".+\\n", "a++b", "(?:ab)+c", "[abc]*d", "\\X+a",
    "a+\\b", "a*$", "\\d+(?=x)", "[\\x{100}-\\x{200}]+z", "a+\\R", "\\h+\\v",
    "(a|b)+c", "a+(?:b|c)", "\\D+\\d", "[^\\d]+\\d", "\\p{Lu}+\\p{Ll}",
    "a{1,}b", "a{0,5}b", "[[:alpha:]]+[[:digit:]]",
];

#[test]
fn row151_auto_possessify() {
    for p in POSSESS_PATTERNS {
        for copts in [
            0u32,
            PCRE2_NO_AUTO_POSSESS,
            PCRE2_UTF,
            PCRE2_UTF | PCRE2_NO_AUTO_POSSESS,
            PCRE2_UCP,
            PCRE2_UTF | PCRE2_UCP,
            PCRE2_CASELESS,
            PCRE2_CASELESS | PCRE2_UTF | PCRE2_UCP,
        ] {
            full_check(p, copts, 0, "row151 auto_possessify");
        }
        // and through set_optimize instead of the option bit
        let (c, r) = pair();
        for d in [0u32, 1, 64, 65] {
            let cc = unsafe { (c.compile_context_create)(std::ptr::null_mut()) };
            let rc = unsafe { (r.compile_context_create)(std::ptr::null_mut()) };
            assert_eq!(
                unsafe { (c.set_optimize)(cc, d) },
                unsafe { (r.set_optimize)(rc, d) }
            );
            let v = zp(p);
            let mut ce: c_int = 0;
            let mut co: Sz = 0;
            let mut re: c_int = 0;
            let mut ro: Sz = 0;
            let a = unsafe { (c.compile)(v.as_ptr(), p.len(), 0, &mut ce, &mut co, cc) };
            let b = unsafe { (r.compile)(v.as_ptr(), p.len(), 0, &mut re, &mut ro, rc) };
            assert_eq!((ce, co, a.is_null()), (re, ro, b.is_null()));
            if !a.is_null() {
                assert_eq!(
                    unsafe { serialize_bytes(c, a) },
                    unsafe { serialize_bytes(r, b) },
                    "row151 set_optimize({d}) {p:?}"
                );
                unsafe { (c.code_free)(a) };
                unsafe { (r.code_free)(b) };
            }
            unsafe { (c.compile_context_free)(cc) };
            unsafe { (r.compile_context_free)(rc) };
        }
    }
}

// ------------------------------------- row 152: _pcre2_xclass via matching ----

const XCLASS_PATTERNS: &[&str] = &[
    "[\\x{100}]", "[\\x{100}-\\x{200}]", "[^\\x{100}]", "[\\p{L}]", "[\\P{L}]",
    "[\\p{Lu}\\p{Ll}]", "[\\p{Greek}]", "[\\p{Han}]", "[[:alpha:]\\x{100}]",
    "[\\x{100}\\x{200}\\x{300}]", "[^\\p{L}]", "[\\p{Any}]", "[\\p{Xan}]", "[\\p{Xsp}]",
    "[\\p{Xps}]", "[\\p{Xwd}]", "[\\p{Xuc}]", "[a-\\x{ffff}]", "[\\x{10000}-\\x{10ffff}]",
    "[\\p{Bidi_Control}]", "[\\p{Sc=Latn}]", "[\\p{scx:Latin}]", "[\\p{Zs}]",
    "[\\d\\x{100}]", "[\\w\\x{100}]", "[\\s\\x{100}]", "[^\\d\\x{100}]",
    "[\\x{100}-\\x{10ffff}]", "\\p{L}", "\\P{L}", "[\\p{L}&&[a-z]]",
];

#[test]
fn row152_xclass() {
    for p in XCLASS_PATTERNS {
        for copts in [
            PCRE2_UTF,
            PCRE2_UTF | PCRE2_UCP,
            PCRE2_UTF | PCRE2_CASELESS,
            PCRE2_UTF | PCRE2_UCP | PCRE2_CASELESS,
            PCRE2_UCP,
            0,
            PCRE2_ALT_EXTENDED_CLASS | PCRE2_UTF,
        ] {
            for xopts in [0u32, PCRE2_EXTRA_CASELESS_RESTRICT, PCRE2_EXTRA_ASCII_POSIX] {
                full_check(p, copts, xopts, "row152 xclass");
            }
        }
    }
}

// --------------------------- row 153: _pcre2_check_escape, all escape forms ---

#[test]
fn row153_check_escape_all_forms() {
    let escapes: &[&str] = &[
        "\\d", "\\D", "\\s", "\\S", "\\w", "\\W", "\\h", "\\H", "\\v", "\\V", "\\R",
        "\\X", "\\C", "\\K", "\\b", "\\B", "\\A", "\\Z", "\\z", "\\G", "\\N",
        "\\Qab+\\E", "\\o{101}", "\\x{41}", "\\x41", "\\N{U+0041}", "\\g1", "\\g{1}",
        "\\g<1>", "\\g'1'", "\\k<n>", "\\k'n'", "\\k{n}", "\\p{L}", "\\P{L}", "\\1",
        "\\n", "\\r", "\\t", "\\f", "\\a", "\\e", "\\0", "\\00", "\\000", "\\cA", "\\c@",
        "\\u0041", "\\u{41}", "\\377", "\\400", "\\8", "\\9",
    ];
    for e in escapes {
        for wrap in ["{E}", "(n){E}", "[{E}]", "(?<n>x){E}", "a{E}b", "(?i){E}"] {
            let p = wrap.replace("{E}", e);
            for copts in [
                0u32,
                PCRE2_UTF,
                PCRE2_UCP,
                PCRE2_UTF | PCRE2_UCP,
                PCRE2_ALT_BSUX,
                PCRE2_CASELESS,
            ] {
                for xopts in [
                    0u32,
                    PCRE2_EXTRA_ALT_BSUX,
                    PCRE2_EXTRA_BAD_ESCAPE_IS_LITERAL,
                    PCRE2_EXTRA_PYTHON_OCTAL,
                    PCRE2_EXTRA_NO_BS0,
                    PCRE2_EXTRA_ALLOW_SURROGATE_ESCAPES,
                    PCRE2_EXTRA_ESCAPED_CR_IS_LF,
                    PCRE2_EXTRA_ASCII_BSD | PCRE2_EXTRA_ASCII_BSS | PCRE2_EXTRA_ASCII_BSW,
                    PCRE2_EXTRA_ASCII_DIGIT | PCRE2_EXTRA_ASCII_POSIX,
                ] {
                    full_check(&p, copts, xopts, "row153 check_escape");
                }
            }
        }
    }
}

// ------- row 154: extended classes (_pcre2_eclass / compile_class_* / bits) ---

const ECLASS_PATTERNS: &[&str] = &[
    "[[a-z]&&[^aeiou]]",
    "[[a-z]--[aeiou]]",
    "[[a-z]~~[aeiou]]",
    "[\\p{L}--\\p{Lu}]",
    "[\\p{L}&&\\p{Latin}]",
    "[\\p{L}~~\\p{N}]",
    "[[abc][def]]",
    "[a[bc]d]",
    "[[[a-z]&&[b-y]]--[m]]",
    "[[[a]]]",
    "[^[a-z]&&[b-y]]",
    "[[a-z]&&[[b-y]--[m]]]",
    "[[\\d][\\w]]",
    "[[[:alpha:]]&&[[:lower:]]]",
    "[[\\x{100}-\\x{200}]&&[\\x{150}-\\x{250}]]",
    "[[\\p{Greek}]--[\\p{Lu}]]",
    "[[a-z][A-Z][0-9]]",
    "[[a-z]&&[A-Z]]",
    "[[^a]&&[^b]]",
    "(?[[a-z]&&[^aeiou]])",
    "(?[[a-z]--[aeiou]])",
    "(?[\\p{L}&&\\p{Latin}])",
    "(?[[a][b][c]])",
    "(?[[[a-z]&&[b-y]]--[m]])",
];

#[test]
fn row154_extended_classes() {
    for p in ECLASS_PATTERNS {
        for copts in [
            PCRE2_ALT_EXTENDED_CLASS,
            PCRE2_ALT_EXTENDED_CLASS | PCRE2_UTF,
            PCRE2_ALT_EXTENDED_CLASS | PCRE2_UTF | PCRE2_UCP,
            PCRE2_ALT_EXTENDED_CLASS | PCRE2_CASELESS,
            PCRE2_ALT_EXTENDED_CLASS | PCRE2_UTF | PCRE2_CASELESS | PCRE2_UCP,
            0,
            PCRE2_UTF,
            PCRE2_UTF | PCRE2_UCP,
        ] {
            for xopts in [0u32, PCRE2_EXTRA_CASELESS_RESTRICT, PCRE2_EXTRA_ASCII_POSIX] {
                full_check(p, copts, xopts, "row154 eclass");
            }
        }
    }
}

// ------------------- row 155: name-table helpers via named/recursive groups ---

const NAME_PATTERNS: &[&str] = &[
    "(?<a>x)",
    "(?<a>x)(?<b>y)(?<c>z)",
    "(?J)(?<a>x)|(?<a>y)",
    "(?J)(?<a>x)(?<a>y)(?<a>z)",
    "(?<aa>x)(?<ab>y)(?<ba>z)",
    "(?<n>a)(?&n)",
    "(?<n>a)\\k<n>",
    "(?<n>a)\\g{n}",
    "(?<n>a)(?P=n)",
    "(?<n>a)(?P>n)",
    "(?(DEFINE)(?<w>\\w+))(?&w)-(?&w)",
    "(?|(?<a>x)|(?<a>y))",
    "(?<\u{00e9}>a)",
    "(?<n1>a)(?<n2>b)(?<n3>c)(?<n4>d)(?<n5>e)(?<n6>f)(?<n7>g)(?<n8>h)",
    "(?<z>a)(?<y>b)(?<x>c)(?<w>d)",
    "(?<recurse>a(?&recurse)?b)",
    "(*scs:(<n>)x)(?<n>y)",
    "(*scs:(1)x)(y)",
    "(?<n>a)(*scs:(<n>)b)",
    "(?<longname_aaaaaaaaaaaaaaaa>x)(?<longname_bbbbbbbbbbbbbbbb>y)",
];

#[test]
fn row155_name_table_helpers() {
    let (c, r) = pair();
    for p in NAME_PATTERNS {
        for copts in [0u32, PCRE2_DUPNAMES, PCRE2_UTF, PCRE2_UTF | PCRE2_DUPNAMES] {
            full_check(p, copts, 0, "row155 name table");
            // and explicitly compare the name table itself
            let v = zp(p);
            let mut ce: c_int = 0;
            let mut co: Sz = 0;
            let mut re: c_int = 0;
            let mut ro: Sz = 0;
            let a = unsafe {
                (c.compile)(v.as_ptr(), p.len(), copts, &mut ce, &mut co, std::ptr::null_mut())
            };
            let b = unsafe {
                (r.compile)(v.as_ptr(), p.len(), copts, &mut re, &mut ro, std::ptr::null_mut())
            };
            if a.is_null() {
                continue;
            }
            for name in [
                "a", "b", "c", "n", "n1", "n8", "w", "aa", "ab", "ba", "recurse", "z",
                "\u{00e9}", "longname_aaaaaaaaaaaaaaaa", "nope",
            ] {
                let nz = zp(name);
                assert_eq!(
                    unsafe { (c.substring_number_from_name)(a, nz.as_ptr()) },
                    unsafe { (r.substring_number_from_name)(b, nz.as_ptr()) },
                    "row155 number_from_name({name:?}) {p:?} copts=0x{copts:x}"
                );
                let mut cf: *const u8 = std::ptr::null();
                let mut clast: *const u8 = std::ptr::null();
                let mut rf: *const u8 = std::ptr::null();
                let mut rlast: *const u8 = std::ptr::null();
                let ci = unsafe { (c.substring_nametable_scan)(a, nz.as_ptr(), &mut cf, &mut clast) };
                let ri = unsafe { (r.substring_nametable_scan)(b, nz.as_ptr(), &mut rf, &mut rlast) };
                assert_eq!(ci, ri, "row155 nametable_scan({name:?}) {p:?}");
            }
            unsafe { (c.code_free)(a) };
            unsafe { (r.code_free)(b) };
        }
    }
}

// ----------------------------- row 150: _pcre2_study through the whole corpus --

#[test]
fn row150_study_via_start_optimizations() {
    // The start bitmap, minlength and first/req code unit that `_pcre2_study`
    // computes are all inside the serialized image, so any divergence shows up
    // as a byte difference. Drive the optimizer on/off in every way.
    let pats: &[&str] = &[
        "abc", "a|b|c", "^abc", "(?:abc|abd)", "[abc]def", "\\d\\d\\d", ".*abc",
        "(?i)abc", "\\babc", "(a)(b)(c)", "a{3,}", "(?=abc)def", "(?!abc)def",
        "\\Aabc", "(?m)^abc", "abc$", "(?s).*abc", "\\p{L}\\p{L}", "[^a]+b",
        "(?|(a)|(b))c", "(?R)?abc", "x(?<n>y)z", "\\Kabc", "(*COMMIT)abc",
        "(?<=x)abc", "(?<!x)abc", "\\x{100}abc", "(?i)\u{00e9}", "a\\z", "\\G(a)",
    ];
    let (c, r) = pair();
    for p in pats {
        for copts in [
            0u32,
            PCRE2_NO_START_OPTIMIZE,
            PCRE2_NO_DOTSTAR_ANCHOR,
            PCRE2_NO_AUTO_POSSESS,
            PCRE2_UTF,
            PCRE2_UTF | PCRE2_UCP,
            PCRE2_CASELESS,
            PCRE2_MULTILINE,
            PCRE2_DOTALL,
            PCRE2_ANCHORED,
            PCRE2_FIRSTLINE,
        ] {
            full_check(p, copts, 0, "row150 study");
            // and via set_optimize
            for d in [0u32, 1, 64, 65, 66, 67] {
                let cc = unsafe { (c.compile_context_create)(std::ptr::null_mut()) };
                let rc = unsafe { (r.compile_context_create)(std::ptr::null_mut()) };
                unsafe { (c.set_optimize)(cc, d) };
                unsafe { (r.set_optimize)(rc, d) };
                let v = zp(p);
                let mut ce: c_int = 0;
                let mut co: Sz = 0;
                let mut re: c_int = 0;
                let mut ro: Sz = 0;
                let a = unsafe { (c.compile)(v.as_ptr(), p.len(), copts, &mut ce, &mut co, cc) };
                let b = unsafe { (r.compile)(v.as_ptr(), p.len(), copts, &mut re, &mut ro, rc) };
                assert_eq!((ce, co, a.is_null()), (re, ro, b.is_null()));
                if !a.is_null() {
                    assert_eq!(
                        unsafe { serialize_bytes(c, a) },
                        unsafe { serialize_bytes(r, b) },
                        "row150 optimize={d} copts=0x{copts:x} {p:?}"
                    );
                    // calling _pcre2_study again must be equally idempotent
                    let x = unsafe { (c.priv_study)(a) };
                    let y = unsafe { (r.priv_study)(b) };
                    assert_eq!(x, y, "row150 re-study rc {p:?}");
                    assert_eq!(
                        unsafe { serialize_bytes(c, a) },
                        unsafe { serialize_bytes(r, b) },
                        "row150 after re-study {p:?}"
                    );
                    unsafe { (c.code_free)(a) };
                    unsafe { (r.code_free)(b) };
                }
                unsafe { (c.compile_context_free)(cc) };
                unsafe { (r.compile_context_free)(rc) };
            }
        }
    }
}

// ---------------------------- row 14: _pcre2_find_bracket_8 (direct calls) ----

/// `pcre2_real_code.code_start` (a `PCRE2_SIZE`) lives at offset 80:
/// memctl 24 + tables 8 + executable_jit 8 + start_bitmap[32] = 72, then
/// `blocksize` (8) at 72 and `code_start` at 80.
const CODE_START_OFF: usize = 80;

unsafe fn codestart(code: *const Code) -> *const u8 {
    let off = unsafe { *((code as *const u8).add(CODE_START_OFF) as *const Sz) };
    unsafe { (code as *const u8).add(off) }
}

#[test]
fn row14_find_bracket() {
    let (c, r) = pair();
    // Sanity-check CODE_START_OFF: the byte it points at must be OP_BRA, whose
    // value is taken from the library's own `_pcre2_OP_lengths_8` position for
    // OP_BRA — simplest reliable check: both libraries must agree, the opcode
    // must be a valid index into OP_lengths (len 0xad), and `_pcre2_find_bracket`
    // must be able to locate group 1 in "(a)".
    {
        let v = zp("(a)");
        let mut e: c_int = 0;
        let mut o: Sz = 0;
        let mut first = [0u8; 2];
        for (i, a) in [c, r].into_iter().enumerate() {
            let code = unsafe {
                (a.compile)(v.as_ptr(), 3, 0, &mut e, &mut o, std::ptr::null_mut())
            };
            assert!(!code.is_null());
            let cs = unsafe { codestart(code) };
            first[i] = unsafe { *cs };
            assert!(
                (first[i] as usize) < 0xad,
                "{}: CODE_START_OFF is wrong (opcode {} out of range)",
                a.name,
                first[i]
            );
            let found = unsafe { (a.priv_find_bracket)(cs, 0, 1) };
            assert!(
                !found.is_null(),
                "{}: find_bracket(1) failed on \"(a)\" — CODE_START_OFF is wrong",
                a.name
            );
            unsafe { (a.code_free)(code) };
        }
        assert_eq!(first[0], first[1], "first opcode differs between C and Rust");
    }
    let pats: &[&str] = &[
        "(a)", "(a)(b)", "(a)(b)(c)", "((a)(b))", "(?<n>a)(?<m>b)", "(?:a)(b)",
        "(a)|(b)", "(a(b(c)))", "(a)+(b)*", "(?|(a)|(b))", "(?<n>(a))(?&n)",
        "(a)(?1)(b)", "(?(1)(a)|(b))((c))", "(a)(?:b)(c)(?:d)(e)",
        "(a)(b)(c)(d)(e)(f)(g)(h)(i)(j)(k)(l)", "(?i)(a)(?-i)(b)",
        "(*UTF)(a)(\\x{100})", "((((((((((a))))))))))", "(a)[bc](d)", "(?>(a))(b)",
        "(?=(a))(b)", "(?<=(a))(b)", "(a)\\1(b)", "(?<n>a)\\k<n>(b)",
    ];
    for p in pats {
        for copts in [0u32, PCRE2_UTF, PCRE2_UTF | PCRE2_UCP, PCRE2_NO_AUTO_CAPTURE] {
            let v = zp(p);
            let mut ce: c_int = 0;
            let mut co: Sz = 0;
            let mut re: c_int = 0;
            let mut ro: Sz = 0;
            let a = unsafe {
                (c.compile)(v.as_ptr(), p.len(), copts, &mut ce, &mut co, std::ptr::null_mut())
            };
            let b = unsafe {
                (r.compile)(v.as_ptr(), p.len(), copts, &mut re, &mut ro, std::ptr::null_mut())
            };
            assert_eq!((ce, co, a.is_null()), (re, ro, b.is_null()), "{p:?}");
            if a.is_null() {
                continue;
            }
            let cs = unsafe { codestart(a) };
            let rs = unsafe { codestart(b) };
            let utf = if copts & PCRE2_UTF != 0 { 1 } else { 0 } as c_int;
            for num in -1..=20i32 {
                let cp = unsafe { (c.priv_find_bracket)(cs, utf, num) };
                let rp = unsafe { (r.priv_find_bracket)(rs, utf, num) };
                assert_eq!(
                    cp.is_null(),
                    rp.is_null(),
                    "row14 find_bracket({num}) nullness {p:?} copts=0x{copts:x}"
                );
                if !cp.is_null() {
                    assert_eq!(
                        cp as usize - cs as usize,
                        rp as usize - rs as usize,
                        "row14 find_bracket({num}) offset {p:?} copts=0x{copts:x}"
                    );
                }
            }
            unsafe { (c.code_free)(a) };
            unsafe { (r.code_free)(b) };
        }
    }
}

// -------------------------- row 79: in-pattern option settings, explicitly ----

#[test]
fn row79_in_pattern_option_settings() {
    let starts: &[&str] = &[
        "(*UTF)", "(*UCP)", "(*CR)", "(*LF)", "(*CRLF)", "(*ANY)", "(*ANYCRLF)",
        "(*NUL)", "(*BSR_UNICODE)", "(*BSR_ANYCRLF)", "(*LIMIT_MATCH=100)",
        "(*LIMIT_DEPTH=100)", "(*LIMIT_HEAP=100)", "(*NO_START_OPT)",
        "(*NO_AUTO_POSSESS)", "(*NO_DOTSTAR_ANCHOR)", "(*NOTEMPTY)",
        "(*NOTEMPTY_ATSTART)", "(*NO_JIT)", "(*UTF)(*UCP)", "(*CR)(*BSR_ANYCRLF)",
        "(*LIMIT_MATCH=1)", "(*LIMIT_DEPTH=1)", "(*LIMIT_HEAP=0)",
        "(*UTF)(*UCP)(*CRLF)(*BSR_ANYCRLF)(*NO_START_OPT)",
    ];
    let bodies: &[&str] = &[
        "a", ".*", "^a$", "\\R", "\\X", "a+b", "\\p{L}", "(a)(b)", "[a-z]+",
        "(?i)\u{00e9}", "\\d{2,4}", "(a+)+b",
    ];
    for st in starts {
        for body in bodies {
            let p = format!("{st}{body}");
            for copts in [0u32, PCRE2_MULTILINE, PCRE2_CASELESS, PCRE2_DOTALL] {
                full_check(&p, copts, 0, "row79 in-pattern options");
            }
        }
    }
}

// ---------------------------------------- row 105: get_startchar, explicitly ---

#[test]
fn row105_get_startchar() {
    let (c, r) = pair();
    let pats: &[&str] = &[
        "abc", "\\Kabc", "(?<=x)abc", "a", "(?=b)a", "\\babc", "x*abc", "(?i)ABC",
        "(*UTF)\\X", "\\p{L}+",
    ];
    let mut subs: Vec<Vec<u8>> = SUBJECTS.iter().map(|s| s.as_bytes().to_vec()).collect();
    subs.extend(RAW_SUBJECTS.iter().map(|s| s.to_vec()));
    for p in pats {
        for copts in [0u32, PCRE2_UTF] {
            let v = zp(p);
            let mut ce: c_int = 0;
            let mut co: Sz = 0;
            let mut re: c_int = 0;
            let mut ro: Sz = 0;
            let a = unsafe {
                (c.compile)(v.as_ptr(), p.len(), copts, &mut ce, &mut co, std::ptr::null_mut())
            };
            let b = unsafe {
                (r.compile)(v.as_ptr(), p.len(), copts, &mut re, &mut ro, std::ptr::null_mut())
            };
            if a.is_null() {
                continue;
            }
            let cmd = unsafe { (c.match_data_create)(8, std::ptr::null_mut()) };
            let rmd = unsafe { (r.match_data_create)(8, std::ptr::null_mut()) };
            for s in &subs {
                if !unsafe { subject_domain_is_defined(c, a, s, 0, 0) } {
                    continue;
                }
                let sp = pad(s);
                for mopts in [0u32, PCRE2_PARTIAL_SOFT, PCRE2_PARTIAL_HARD, PCRE2_ANCHORED] {
                    for start in 0..=s.len() {
                        let x = unsafe {
                            (c.pcre2_match)(a, sp.as_ptr(), s.len(), start, mopts, cmd, std::ptr::null_mut())
                        };
                        let y = unsafe {
                            (r.pcre2_match)(b, sp.as_ptr(), s.len(), start, mopts, rmd, std::ptr::null_mut())
                        };
                        assert_eq!(x, y, "row105 rc {p:?} {s:02x?} start={start}");
                        // `startchar` is defined after a match, a partial match,
                        // and after a UTF error (where it is the error offset).
                        if x >= 0 || x == PCRE2_ERROR_PARTIAL || x <= -3 {
                            assert_eq!(
                                unsafe { (c.get_startchar)(cmd) },
                                unsafe { (r.get_startchar)(rmd) },
                                "row105 startchar {p:?} {s:02x?} start={start} mopts=0x{mopts:x} rc={x}"
                            );
                        }
                    }
                }
            }
            unsafe { (c.match_data_free)(cmd) };
            unsafe { (r.match_data_free)(rmd) };
            unsafe { (c.code_free)(a) };
            unsafe { (r.code_free)(b) };
        }
    }
}
