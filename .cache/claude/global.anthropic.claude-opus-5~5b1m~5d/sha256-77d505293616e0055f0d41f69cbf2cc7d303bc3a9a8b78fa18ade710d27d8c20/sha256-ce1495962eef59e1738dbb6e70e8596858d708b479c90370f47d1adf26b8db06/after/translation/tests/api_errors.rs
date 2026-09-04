//! Phase C — error-path differential tests for `ERRORS.md` sections
//! A (`pcre2_compile_8` argument/option/context validation),
//! H (contexts and `pcre2_set_*`),
//! I (`pcre2_config_8`) and
//! J (`pcre2_pattern_info_8` / `pcre2_callout_enumerate_8`).
mod common;

use common::diff::*;
use common::*;
use std::ffi::c_void;

// ============================================================ alloc injection
// A failure-injecting allocator: the Nth allocation returns NULL. Each library
// gets its own counter so we can ALSO compare how many allocations each made —
// a faithful translation must allocate the same number of times.
static mut ALLOC_N: [u32; 2] = [0, 0];
static mut ALLOC_FAIL_AT: [u32; 2] = [u32::MAX, u32::MAX];

unsafe extern "C" fn inj_malloc_c(n: SIZE, _d: *mut c_void) -> *mut c_void {
    ALLOC_N[0] += 1;
    if ALLOC_N[0] >= ALLOC_FAIL_AT[0] {
        return std::ptr::null_mut();
    }
    libc_malloc(n)
}
unsafe extern "C" fn inj_malloc_r(n: SIZE, _d: *mut c_void) -> *mut c_void {
    ALLOC_N[1] += 1;
    if ALLOC_N[1] >= ALLOC_FAIL_AT[1] {
        return std::ptr::null_mut();
    }
    libc_malloc(n)
}
unsafe extern "C" fn inj_free(p: *mut c_void, _d: *mut c_void) {
    if !p.is_null() {
        libc_free(p);
    }
}

extern "C" {
    #[link_name = "malloc"]
    fn libc_malloc(n: usize) -> *mut c_void;
    #[link_name = "free"]
    fn libc_free(p: *mut c_void);
}

fn injector(i: usize) -> unsafe extern "C" fn(SIZE, *mut c_void) -> *mut c_void {
    if i == 0 {
        inj_malloc_c
    } else {
        inj_malloc_r
    }
}

// ======================================================== Section A rows 1-26
/// Row 1: `errorptr == NULL` -> NULL, `*erroroffset = 0`, no code written.
#[test]
fn row1_errorptr_null() {
    let (c, r) = both();
    unsafe {
        for pat in [&b"a"[..], b"", b"(bad", b"a{2,1}"] {
            let mut ceo = 0xAAusize;
            let mut reo = 0xAAusize;
            let cp = (c.compile)(
                pat.as_ptr(), pat.len(), 0,
                std::ptr::null_mut(), &mut ceo, std::ptr::null_mut(),
            );
            let rp = (r.compile)(
                pat.as_ptr(), pat.len(), 0,
                std::ptr::null_mut(), &mut reo, std::ptr::null_mut(),
            );
            assert!(cp.is_null(), "C must reject errorptr==NULL");
            assert!(rp.is_null(), "Rust must reject errorptr==NULL");
            assert_eq!(ceo, reo, "erroroffset for pattern {:?}", pat);
            assert_eq!(ceo, 0, "C sets *erroroffset = 0");
        }
    }
}

/// Row 2 / row 146 (ERR120): `erroroffset == NULL`.
#[test]
fn row2_erroroffset_null_err120() {
    let (c, r) = both();
    unsafe {
        for pat in [&b"a"[..], b"", b"(bad"] {
            let mut cec = 0i32;
            let mut rec = 0i32;
            let cp = (c.compile)(
                pat.as_ptr(), pat.len(), 0,
                &mut cec, std::ptr::null_mut(), std::ptr::null_mut(),
            );
            let rp = (r.compile)(
                pat.as_ptr(), pat.len(), 0,
                &mut rec, std::ptr::null_mut(), std::ptr::null_mut(),
            );
            assert!(cp.is_null() && rp.is_null());
            assert_eq!(cec, rec, "errorcode for pattern {:?}", pat);
            assert_eq!(cec, 220, "ERR120 (=220) expected");
        }
    }
}

/// Rows 3 & 4: `pattern == NULL` with non-zero / zero length.
#[test]
fn rows3_4_null_pattern() {
    let (c, r) = both();
    unsafe {
        // row 3: patlen != 0 -> ERR16 (=116)
        for patlen in [1usize, 2, 100, PCRE2_ZERO_TERMINATED] {
            let mut cec = 0i32;
            let mut rec = 0i32;
            let mut ceo = 0xAAusize;
            let mut reo = 0xAAusize;
            let cp = (c.compile)(
                std::ptr::null(), patlen, 0, &mut cec, &mut ceo, std::ptr::null_mut(),
            );
            let rp = (r.compile)(
                std::ptr::null(), patlen, 0, &mut rec, &mut reo, std::ptr::null_mut(),
            );
            assert!(cp.is_null() && rp.is_null(), "patlen={}", patlen);
            assert_eq!(cec, rec, "errorcode patlen={}", patlen);
            assert_eq!(ceo, reo, "erroroffset patlen={}", patlen);
            assert_eq!(cec, 116, "ERR16 expected for patlen={}", patlen);
        }
        // row 4: patlen == 0 -> SUCCESS (empty pattern)
        {
            let mut cec = 0i32;
            let mut rec = 0i32;
            let mut ceo = 0usize;
            let mut reo = 0usize;
            let cp = (c.compile)(
                std::ptr::null(), 0, 0, &mut cec, &mut ceo, std::ptr::null_mut(),
            );
            let rp = (r.compile)(
                std::ptr::null(), 0, 0, &mut rec, &mut reo, std::ptr::null_mut(),
            );
            assert_eq!(cp.is_null(), rp.is_null(), "NULL pattern patlen=0 nullness");
            assert!(!cp.is_null(), "C treats NULL/0 as the empty pattern");
            assert_pattern_info_eq(cp, rp, "NULL pattern patlen=0");
            let cb = serialized_bytes(c, cp);
            let rb = serialized_bytes(r, rp);
            assert_eq!(cb, rb, "NULL/0 pattern bytecode");
            (c.code_free)(cp);
            (r.code_free)(rp);
        }
    }
}

/// Rows 5 & 6 (ERR17): undefined option / extra-option bits. Exhaustive over
/// every single bit of both words.
#[test]
fn rows5_6_undefined_option_bits_err17() {
    unsafe {
        for bit in 0..32u32 {
            let opt = 1u32 << bit;
            // Compile options: skip the bits that ARE defined.
            let defined_compile: u32 = PCRE2_ANCHORED
                | PCRE2_NO_UTF_CHECK
                | PCRE2_ENDANCHORED
                | PCRE2_ALLOW_EMPTY_CLASS
                | PCRE2_ALT_BSUX
                | PCRE2_AUTO_CALLOUT
                | PCRE2_CASELESS
                | PCRE2_DOLLAR_ENDONLY
                | PCRE2_DOTALL
                | PCRE2_DUPNAMES
                | PCRE2_EXTENDED
                | PCRE2_FIRSTLINE
                | PCRE2_MATCH_UNSET_BACKREF
                | PCRE2_MULTILINE
                | PCRE2_NEVER_UCP
                | PCRE2_NEVER_UTF
                | PCRE2_NO_AUTO_CAPTURE
                | PCRE2_NO_AUTO_POSSESS
                | PCRE2_NO_DOTSTAR_ANCHOR
                | PCRE2_NO_START_OPTIMIZE
                | PCRE2_UCP
                | PCRE2_UNGREEDY
                | PCRE2_UTF
                | PCRE2_NEVER_BACKSLASH_C
                | PCRE2_ALT_CIRCUMFLEX
                | PCRE2_ALT_VERBNAMES
                | PCRE2_USE_OFFSET_LIMIT
                | PCRE2_EXTENDED_MORE
                | PCRE2_LITERAL
                | PCRE2_MATCH_INVALID_UTF
                | PCRE2_ALT_EXTENDED_CLASS;
            // Whether defined or not, C and Rust must AGREE.
            let _ = compile_both(
                b"abc", 3, &CompileCfg::new(opt),
                &format!("compile option bit {:#x} (defined={})",
                         opt, defined_compile & opt != 0),
            );
            // extra options
            let _ = compile_both(
                b"abc", 3, &CompileCfg::new(0).extra(opt),
                &format!("extra option bit {:#x}", opt),
            );
        }
        // and the specific values the table names
        for opt in [0x8000_0000u32, 0x1000_0000, 0xFFFF_FFFF] {
            let _ = compile_both(
                b"abc", 3, &CompileCfg::new(opt),
                &format!("row5 options={:#x}", opt),
            );
        }
        for extra in [0x8000_0000u32, 0xFFFF_FFFF, 0x0002_0000] {
            let _ = compile_both(
                b"abc", 3, &CompileCfg::new(0).extra(extra),
                &format!("row6 extra={:#x}", extra),
            );
        }
    }
}

/// Rows 7 & 8 (ERR92): bits outside PUBLIC_LITERAL_COMPILE_OPTIONS together
/// with PCRE2_LITERAL. Exhaustive over every bit paired with LITERAL.
#[test]
fn rows7_8_literal_option_conflicts_err92() {
    unsafe {
        for bit in 0..32u32 {
            let opt = 1u32 << bit;
            if opt == PCRE2_LITERAL {
                continue;
            }
            let _ = compile_both(
                b"abc", 3, &CompileCfg::new(PCRE2_LITERAL | opt),
                &format!("row7 LITERAL|{:#x}", opt),
            );
            let _ = compile_both(
                b"abc", 3, &CompileCfg::new(PCRE2_LITERAL).extra(opt),
                &format!("row8 LITERAL+extra {:#x}", opt),
            );
        }
    }
}

/// Rows 9 & 10: max_pattern_length / max_pattern_compiled_length, sweeping the
/// limit across the boundary for several patterns.
#[test]
fn rows9_10_length_limits() {
    unsafe {
        for pat in [&b"ab"[..], b"abcdef", b"a", b"", b"(a)(b)(c)"] {
            for lim in 0..=pat.len() + 2 {
                let _ = compile_both(
                    pat, pat.len(), &CompileCfg::new(0).max_len(lim),
                    &format!("row9 max_len={} pat={:?}", lim, pat),
                );
            }
            for lim in [0usize, 1, 2, 4, 8, 16, 32, 64, 128, usize::MAX] {
                let _ = compile_both(
                    pat, pat.len(), &CompileCfg::new(0).max_compiled(lim),
                    &format!("row10 max_compiled={} pat={:?}", lim, pat),
                );
            }
        }
    }
}

/// Rows 11-19: the "disabled by the application" and TURKISH_CASING conflicts.
#[test]
fn rows11_19_never_and_turkish_conflicts() {
    unsafe {
        // row 11 ERR74, row 12 ERR75, row 13 ERR83, row 14 ERR103,
        // row 15 ERR98, row 16 ERR102
        let cases: [(&[u8], u32, u32, i32, &str); 6] = [
            (b"(*UTF)a", PCRE2_NEVER_UTF, 0, 174, "row11 ERR74"),
            (b"(*UCP)a", PCRE2_NEVER_UCP, 0, 175, "row12 ERR75"),
            (br"\C", PCRE2_NEVER_BACKSLASH_C, 0, 183, "row13 ERR83"),
            (b"(?C1)a", 0, PCRE2_EXTRA_NEVER_CALLOUT, 203, "row14 ERR103"),
            (br"\0", 0, PCRE2_EXTRA_NO_BS0, 198, "row15 ERR98"),
            (br"\400", 0, PCRE2_EXTRA_PYTHON_OCTAL, 202, "row16 ERR102"),
        ];
        for (pat, opts, extra, want, label) in cases {
            let cfg = CompileCfg::new(opts).extra(extra);
            let (cc, rr) = compile_both(pat, pat.len(), &cfg, label);
            assert!(cc.code.is_null(), "{}: expected failure", label);
            assert_eq!(cc.errorcode, want, "{}", label);
            assert_eq!(cc.errorcode, rr.errorcode, "{}", label);
            // …and WITHOUT the restricting option the same pattern must compile
            let _ = compile_both(
                pat, pat.len(), &CompileCfg::new(0),
                &format!("{} unrestricted", label),
            );
        }

        // rows 17-19: TURKISH_CASING requirements, all four UTF/UCP combos and
        // the CASELESS_RESTRICT conflict.
        for opts in [0u32, PCRE2_UCP, PCRE2_UTF, PCRE2_UTF | PCRE2_UCP] {
            let _ = compile_both(
                b"abc", 3,
                &CompileCfg::new(opts).extra(PCRE2_EXTRA_TURKISH_CASING),
                &format!("rows17-18 TURKISH_CASING opts={:#x}", opts),
            );
            let _ = compile_both(
                b"abc", 3,
                &CompileCfg::new(opts)
                    .extra(PCRE2_EXTRA_TURKISH_CASING | PCRE2_EXTRA_CASELESS_RESTRICT),
                &format!("row19 TURKISH|CASELESS_RESTRICT opts={:#x}", opts),
            );
        }
    }
}

/// Row 20 (ERR33): the compile recursion guard returning non-zero.
#[test]
fn row20_recursion_guard_err33() {
    let (c, r) = both();
    unsafe extern "C" fn guard_reject(_depth: u32, _data: *mut c_void) -> i32 {
        1
    }
    unsafe extern "C" fn guard_accept(_depth: u32, _data: *mut c_void) -> i32 {
        0
    }
    unsafe extern "C" fn guard_depth(depth: u32, _data: *mut c_void) -> i32 {
        // reject beyond depth 2
        if depth > 2 { 1 } else { 0 }
    }
    unsafe {
        for (name, g) in [
            ("reject", guard_reject as unsafe extern "C" fn(u32, *mut c_void) -> i32),
            ("accept", guard_accept),
            ("depth>2", guard_depth),
        ] {
            for pat in [&b"(((a)))"[..], b"a", b"(a)", b"((((((a))))))", b"(?:(?:a))"] {
                let mut out = Vec::new();
                for api in [c, r] {
                    let cx = (api.compile_context_create)(std::ptr::null_mut());
                    (api.set_compile_recursion_guard)(
                        cx, g as *mut c_void, std::ptr::null_mut(),
                    );
                    let mut ec = 0i32;
                    let mut eo = 0xAAusize;
                    let code = (api.compile)(
                        pat.as_ptr(), pat.len(), 0, &mut ec, &mut eo, cx,
                    );
                    let isnull = code.is_null();
                    if !isnull {
                        (api.code_free)(code);
                    }
                    (api.compile_context_free)(cx);
                    out.push((isnull, ec, eo));
                }
                assert_eq!(
                    out[0], out[1],
                    "guard={} pat={:?}: (null, errorcode, erroroffset) differ",
                    name, pat
                );
                if name == "reject" && pat == b"(((a)))" {
                    assert_eq!(out[0].1, 133, "row20 expects ERR33 (=133)");
                }
            }
        }
    }
}

/// Rows 21 / 293-295 (ERR21 + NULL from the context constructors): allocation
/// failure injected at every allocation index. Also asserts both libraries make
/// the SAME NUMBER of allocations, which is a strong equivalence check.
#[test]
fn row21_allocation_failure_err21() {
    let _guard = global_lock();
    let (c, r) = both();
    unsafe {
        for pat in [
            &b"abc"[..], b"(a)(b)", b"(?<n>a)", b"a{1,100}", b"[a-z]+\\d*",
            b"(?:a|b|c){2,4}", b"\\p{L}+",
        ] {
            // 1. how many allocations does a successful compile take?
            let mut counts = [0u32; 2];
            for (i, api) in [c, r].iter().enumerate() {
                ALLOC_N[i] = 0;
                ALLOC_FAIL_AT[i] = u32::MAX;
                let gc = (api.general_context_create)(
                    Some(injector(i)), Some(inj_free), std::ptr::null_mut(),
                );
                assert!(!gc.is_null());
                let cx = (api.compile_context_create)(gc);
                assert!(!cx.is_null());
                let mut ec = 0i32;
                let mut eo = 0usize;
                let code =
                    (api.compile)(pat.as_ptr(), pat.len(), 0, &mut ec, &mut eo, cx);
                assert!(!code.is_null(), "{}: baseline compile failed", api.name);
                (api.code_free)(code);
                (api.compile_context_free)(cx);
                (api.general_context_free)(gc);
                counts[i] = ALLOC_N[i];
            }
            assert_eq!(
                counts[0], counts[1],
                "pattern {:?}: C made {} allocations, Rust made {}",
                String::from_utf8_lossy(pat), counts[0], counts[1]
            );

            // 2. fail at each allocation index in turn; results must agree
            for fail_at in 1..=counts[0] {
                let mut out = Vec::new();
                for (i, api) in [c, r].iter().enumerate() {
                    ALLOC_N[i] = 0;
                    ALLOC_FAIL_AT[i] = fail_at;
                    let gc = (api.general_context_create)(
                        Some(injector(i)), Some(inj_free), std::ptr::null_mut(),
                    );
                    if gc.is_null() {
                        out.push((true, 0i32, 0usize, 0u32));
                        ALLOC_FAIL_AT[i] = u32::MAX;
                        continue;
                    }
                    let cx = (api.compile_context_create)(gc);
                    if cx.is_null() {
                        out.push((true, 0, 0, 1));
                        ALLOC_FAIL_AT[i] = u32::MAX;
                        (api.general_context_free)(gc);
                        continue;
                    }
                    let mut ec = 0i32;
                    let mut eo = 0xAAusize;
                    let code = (api.compile)(
                        pat.as_ptr(), pat.len(), 0, &mut ec, &mut eo, cx,
                    );
                    let isnull = code.is_null();
                    if !isnull {
                        (api.code_free)(code);
                    }
                    ALLOC_FAIL_AT[i] = u32::MAX;
                    (api.compile_context_free)(cx);
                    (api.general_context_free)(gc);
                    out.push((isnull, ec, eo, 2));
                }
                assert_eq!(
                    out[0], out[1],
                    "pattern {:?} fail_at={}: outcome differs",
                    String::from_utf8_lossy(pat), fail_at
                );
            }
        }
    }
}

/// Row 22 / section M: malformed UTF-8 pattern bytes with PCRE2_UTF.
#[test]
fn row22_malformed_utf8_pattern() {
    unsafe {
        let bad: [&[u8]; 16] = [
            b"\x80", b"\xff", b"\xc0", b"\xc0\x80", b"\xc1\xbf", b"\xc3",
            b"\xc3\x28", b"\xe0\x80\x80", b"\xe2\x80", b"\xed\xa0\x80",
            b"\xf0\x80\x80\x80", b"\xf4\x90\x80\x80", b"\xf5\x80\x80\x80",
            b"\xfe", b"a\x80b", b"a\xc3",
        ];
        for pat in bad {
            // with the check ON, both must report the same negative UTF error
            let (cc, rr) =
                compile_both(pat, pat.len(), &CompileCfg::new(PCRE2_UTF),
                             &format!("row22 {:02x?}", pat));
            assert!(cc.code.is_null(), "C must reject {:02x?}", pat);
            assert!(cc.errorcode < 0, "expected a negative UTF error, got {}", cc.errorcode);
            assert_eq!(cc.errorcode, rr.errorcode);
            // without PCRE2_UTF the same bytes are just literals
            let _ = compile_both(pat, pat.len(), &CompileCfg::new(0),
                                 &format!("row22 non-utf {:02x?}", pat));
        }
    }
}

// ======================================================== Section H rows 283+
/// Rows 283-292: every `pcre2_set_*` validation, swept exhaustively.
#[test]
fn rows283_292_setter_validation() {
    let (c, r) = both();
    unsafe {
        // row 283: set_bsr — valid 1,2 only
        for v in 0..8u32 {
            let cx = (c.compile_context_create)(std::ptr::null_mut());
            let rx = (r.compile_context_create)(std::ptr::null_mut());
            assert_eq!(
                (c.set_bsr)(cx, v), (r.set_bsr)(rx, v), "set_bsr({})", v
            );
            (c.compile_context_free)(cx);
            (r.compile_context_free)(rx);
        }
        for v in [0xFFFF_FFFFu32, 0x7FFF_FFFF, 100] {
            let cx = (c.compile_context_create)(std::ptr::null_mut());
            let rx = (r.compile_context_create)(std::ptr::null_mut());
            assert_eq!((c.set_bsr)(cx, v), (r.set_bsr)(rx, v), "set_bsr({})", v);
            (c.compile_context_free)(cx);
            (r.compile_context_free)(rx);
        }

        // row 284: set_newline — valid 1..6 only
        for v in (0..10u32).chain([0x7FFF_FFFF, 0xFFFF_FFFF]) {
            let cx = (c.compile_context_create)(std::ptr::null_mut());
            let rx = (r.compile_context_create)(std::ptr::null_mut());
            assert_eq!(
                (c.set_newline)(cx, v), (r.set_newline)(rx, v), "set_newline({})", v
            );
            (c.compile_context_free)(cx);
            (r.compile_context_free)(rx);
        }

        // rows 285-288: set_optimize — NULL context and every directive 0..80
        assert_eq!(
            (c.set_optimize)(std::ptr::null_mut(), 0),
            (r.set_optimize)(std::ptr::null_mut(), 0),
            "row285 set_optimize(NULL)"
        );
        assert_eq!(
            (c.set_optimize)(std::ptr::null_mut(), 0),
            ERR_NULL,
            "row285 expects PCRE2_ERROR_NULL"
        );
        for v in (0..80u32).chain([0x7FFF_FFFF, 0xFFFF_FFFF]) {
            let cx = (c.compile_context_create)(std::ptr::null_mut());
            let rx = (r.compile_context_create)(std::ptr::null_mut());
            assert_eq!(
                (c.set_optimize)(cx, v), (r.set_optimize)(rx, v),
                "set_optimize({})", v
            );
            (c.compile_context_free)(cx);
            (r.compile_context_free)(rx);
        }
        // set_optimize is ORDER-SENSITIVE (it mutates a flag word); check that
        // a SEQUENCE of directives leaves both libraries in the same state, as
        // observed through a compiled pattern's bytecode.
        for seq in [
            &[64u32, 66, 68][..], &[65, 67, 69], &[0, 64, 65], &[1, 65, 64],
            &[64, 65, 64], &[1, 0, 1], &[68, 69, 68],
        ] {
            let mut codes = Vec::new();
            for api in [c, r] {
                let cx = (api.compile_context_create)(std::ptr::null_mut());
                for &d in seq {
                    (api.set_optimize)(cx, d);
                }
                let pat = b"a*b(c|d)+e";
                let mut ec = 0i32;
                let mut eo = 0usize;
                let code =
                    (api.compile)(pat.as_ptr(), pat.len(), 0, &mut ec, &mut eo, cx);
                assert!(!code.is_null(), "seq {:?} compile failed", seq);
                codes.push((api, code));
                (api.compile_context_free)(cx);
            }
            assert_pattern_info_eq(codes[0].1, codes[1].1, &format!("optimize seq {:?}", seq));
            let cb = serialized_bytes(c, codes[0].1);
            let rb = serialized_bytes(r, codes[1].1);
            assert_eq!(cb, rb, "optimize seq {:?}: bytecode differs", seq);
            (c.code_free)(codes[0].1);
            (r.code_free)(codes[1].1);
        }

        // rows 289-292: glob separator / escape, exhaustive over 0..=256 + big
        for v in (0..=256u32).chain([0xFFFF_FFFF, 0x7FFF_FFFF]) {
            let cx = (c.convert_context_create)(std::ptr::null_mut());
            let rx = (r.convert_context_create)(std::ptr::null_mut());
            assert_eq!(
                (c.set_glob_separator)(cx, v), (r.set_glob_separator)(rx, v),
                "set_glob_separator({})", v
            );
            assert_eq!(
                (c.set_glob_escape)(cx, v), (r.set_glob_escape)(rx, v),
                "set_glob_escape({})", v
            );
            (c.convert_context_free)(cx);
            (r.convert_context_free)(rx);
        }
    }
}

/// Row 293: `pcre2_general_context_create_8` whose malloc returns NULL.
#[test]
fn row293_general_context_create_alloc_failure() {
    let _guard = global_lock();
    let (c, r) = both();
    unsafe {
        for (i, api) in [c, r].iter().enumerate() {
            ALLOC_N[i] = 0;
            ALLOC_FAIL_AT[i] = 1; // fail the very first allocation
            let gc = (api.general_context_create)(
                Some(injector(i)), Some(inj_free), std::ptr::null_mut(),
            );
            ALLOC_FAIL_AT[i] = u32::MAX;
            assert!(gc.is_null(), "{}: must return NULL on alloc failure", api.name);
        }
    }
}

/// Rows 294-295: context create/copy under allocation failure.
#[test]
fn rows294_295_context_create_copy_alloc_failure() {
    let _guard = global_lock();
    let (c, r) = both();
    unsafe {
        for (i, api) in [c, r].iter().enumerate() {
            // a working general context first
            ALLOC_N[i] = 0;
            ALLOC_FAIL_AT[i] = u32::MAX;
            let gc = (api.general_context_create)(
                Some(injector(i)), Some(inj_free), std::ptr::null_mut(),
            );
            assert!(!gc.is_null());

            // create with the NEXT allocation failing
            for which in 0..3 {
                ALLOC_N[i] = 0;
                ALLOC_FAIL_AT[i] = 1;
                let cx = match which {
                    0 => (api.compile_context_create)(gc),
                    1 => (api.match_context_create)(gc),
                    _ => (api.convert_context_create)(gc),
                };
                ALLOC_FAIL_AT[i] = u32::MAX;
                assert!(
                    cx.is_null(),
                    "{}: context_create({}) must be NULL on alloc failure",
                    api.name, which
                );
            }

            // copy with the allocation failing
            ALLOC_N[i] = 0;
            ALLOC_FAIL_AT[i] = u32::MAX;
            let cx = (api.compile_context_create)(gc);
            assert!(!cx.is_null());
            ALLOC_N[i] = 0;
            ALLOC_FAIL_AT[i] = 1;
            let cp = (api.compile_context_copy)(cx);
            ALLOC_FAIL_AT[i] = u32::MAX;
            assert!(cp.is_null(), "{}: context_copy must be NULL", api.name);
            (api.compile_context_free)(cx);

            ALLOC_N[i] = 0;
            ALLOC_FAIL_AT[i] = 1;
            let gp = (api.general_context_copy)(gc);
            ALLOC_FAIL_AT[i] = u32::MAX;
            assert!(gp.is_null(), "{}: general_context_copy must be NULL", api.name);

            (api.general_context_free)(gc);
        }
    }
}

/// Row 297: the unvalidated setters always return 0, for every value.
#[test]
fn row297_unvalidated_setters_always_return_zero() {
    let (c, r) = both();
    unsafe {
        let vals32 = [0u32, 1, 2, 255, 256, 65535, 65536, 0x7FFF_FFFF, 0xFFFF_FFFF];
        let valsz = [0usize, 1, 255, 65535, usize::MAX / 2, usize::MAX];
        for v in vals32 {
            let cx = (c.compile_context_create)(std::ptr::null_mut());
            let rx = (r.compile_context_create)(std::ptr::null_mut());
            for (name, f) in [
                ("max_varlookbehind", c.set_max_varlookbehind),
                ("parens_nest_limit", c.set_parens_nest_limit),
                ("compile_extra_options", c.set_compile_extra_options),
            ] {
                let rf = match name {
                    "max_varlookbehind" => r.set_max_varlookbehind,
                    "parens_nest_limit" => r.set_parens_nest_limit,
                    _ => r.set_compile_extra_options,
                };
                assert_eq!(f(cx, v), rf(rx, v), "{}({})", name, v);
                assert_eq!(f(cx, v), 0, "{}({}) must return 0", name, v);
            }
            (c.compile_context_free)(cx);
            (r.compile_context_free)(rx);

            let cm = (c.match_context_create)(std::ptr::null_mut());
            let rm = (r.match_context_create)(std::ptr::null_mut());
            for name in ["heap", "match", "depth", "recursion"] {
                let (f, rf) = match name {
                    "heap" => (c.set_heap_limit, r.set_heap_limit),
                    "match" => (c.set_match_limit, r.set_match_limit),
                    "depth" => (c.set_depth_limit, r.set_depth_limit),
                    _ => (c.set_recursion_limit, r.set_recursion_limit),
                };
                assert_eq!(f(cm, v), rf(rm, v), "set_{}_limit({})", name, v);
                assert_eq!(f(cm, v), 0, "set_{}_limit({}) must return 0", name, v);
            }
            (c.match_context_free)(cm);
            (r.match_context_free)(rm);
        }
        for v in valsz {
            let cx = (c.compile_context_create)(std::ptr::null_mut());
            let rx = (r.compile_context_create)(std::ptr::null_mut());
            assert_eq!(
                (c.set_max_pattern_length)(cx, v),
                (r.set_max_pattern_length)(rx, v),
                "set_max_pattern_length({})", v
            );
            assert_eq!(
                (c.set_max_pattern_compiled_length)(cx, v),
                (r.set_max_pattern_compiled_length)(rx, v),
                "set_max_pattern_compiled_length({})", v
            );
            (c.compile_context_free)(cx);
            (r.compile_context_free)(rx);
            let cm = (c.match_context_create)(std::ptr::null_mut());
            let rm = (r.match_context_create)(std::ptr::null_mut());
            assert_eq!(
                (c.set_offset_limit)(cm, v), (r.set_offset_limit)(rm, v),
                "set_offset_limit({})", v
            );
            (c.match_context_free)(cm);
            (r.match_context_free)(rm);
        }
        // set_character_tables with NULL (means "revert to default")
        let cx = (c.compile_context_create)(std::ptr::null_mut());
        let rx = (r.compile_context_create)(std::ptr::null_mut());
        assert_eq!(
            (c.set_character_tables)(cx, std::ptr::null()),
            (r.set_character_tables)(rx, std::ptr::null()),
            "set_character_tables(NULL)"
        );
        (c.compile_context_free)(cx);
        (r.compile_context_free)(rx);
    }
}

/// Row 298: `*_context_free(NULL)` is a no-op.
#[test]
fn row298_context_free_null_is_noop() {
    let (c, r) = both();
    unsafe {
        for api in [c, r] {
            (api.general_context_free)(std::ptr::null_mut());
            (api.compile_context_free)(std::ptr::null_mut());
            (api.match_context_free)(std::ptr::null_mut());
            (api.convert_context_free)(std::ptr::null_mut());
        }
    }
}

// ======================================================== Section I rows 299+
/// Rows 299-302: `pcre2_config_8` selectors — EVERY value 0..64 plus extremes,
/// with both `where != NULL` and `where == NULL` (the length request).
#[test]
fn rows299_302_config_selectors() {
    let (c, r) = both();
    unsafe {
        for what in (0..64u32).chain([0x7FFF_FFFF, 0xFFFF_FFFF, 1000, 17]) {
            // length request
            let crc = (c.config)(what, std::ptr::null_mut());
            let rrc = (r.config)(what, std::ptr::null_mut());
            assert_eq!(crc, rrc, "config({}, NULL) rc", what);
            // value request — buffer is generously sized for the string cases
            let mut cb = [0xAAu8; 64];
            let mut rb = [0xAAu8; 64];
            let crc2 = (c.config)(what, cb.as_mut_ptr() as *mut _);
            let rrc2 = (r.config)(what, rb.as_mut_ptr() as *mut _);
            assert_eq!(crc2, rrc2, "config({}, buf) rc", what);
            assert_eq!(cb, rb, "config({}, buf) written bytes", what);
        }
        // the table's specific expectations
        assert_eq!((c.config)(0x7FFF_FFFF, std::ptr::null_mut()), ERR_BADOPTION);
        assert_eq!((c.config)(17, std::ptr::null_mut()), ERR_BADOPTION);
        // row 302: JITTARGET in a non-JIT build
        let mut buf = [0u8; 64];
        assert_eq!(
            (c.config)(2, buf.as_mut_ptr() as *mut _),
            (r.config)(2, buf.as_mut_ptr() as *mut _),
            "config(JITTARGET)"
        );
    }
}

// ======================================================== Section J rows 304+
/// Rows 304-312: `pcre2_pattern_info_8` validation.
#[test]
fn rows304_312_pattern_info_validation() {
    let (c, r) = both();
    unsafe {
        // row 304: code == NULL, where != NULL -> PCRE2_ERROR_NULL
        for what in 0..30u32 {
            let mut cv = 0usize;
            let mut rv = 0usize;
            let crc = (c.pattern_info)(
                std::ptr::null(), what, &mut cv as *mut _ as *mut c_void,
            );
            let rrc = (r.pattern_info)(
                std::ptr::null(), what, &mut rv as *mut _ as *mut c_void,
            );
            assert_eq!(crc, rrc, "pattern_info(NULL, {}) rc", what);
        }
        // row 312: code == NULL AND where == NULL -> the field size
        for what in (0..30u32).chain([0x7FFF_FFFF, 0xFFFF_FFFF]) {
            let crc = (c.pattern_info)(std::ptr::null(), what, std::ptr::null_mut());
            let rrc = (r.pattern_info)(std::ptr::null(), what, std::ptr::null_mut());
            assert_eq!(crc, rrc, "pattern_info(NULL, {}, NULL) rc", what);
        }
        assert_eq!(
            (c.pattern_info)(std::ptr::null(), 0, std::ptr::null_mut()),
            4,
            "row312: ALLOPTIONS field size is 4"
        );

        // rows 305-306: unknown selectors on a REAL code
        let cc = compile_in(c, b"abc", 3, &CompileCfg::new(0));
        let rr = compile_in(r, b"abc", 3, &CompileCfg::new(0));
        for what in (0..40u32).chain([0x7FFF_FFFF, 0xFFFF_FFFF, 27, 100]) {
            // 7 = FIRSTBITMAP and 19 = NAMETABLE return POINTERS into each
            // library's own allocation, so their values cannot be compared
            // directly; `assert_pattern_info_eq` compares the pointed-to bytes.
            if what == 7 || what == 19 {
                continue;
            }
            let mut cv = 0usize;
            let mut rv = 0usize;
            let crc =
                (c.pattern_info)(cc.code, what, &mut cv as *mut _ as *mut c_void);
            let rrc =
                (r.pattern_info)(rr.code, what, &mut rv as *mut _ as *mut c_void);
            assert_eq!(crc, rrc, "pattern_info(code, {}) rc", what);
            if crc == 0 || crc == ERR_UNSET {
                // rows 309-311: the value is written EVEN on PCRE2_ERROR_UNSET
                assert_eq!(cv, rv, "pattern_info(code, {}) value (rc={})", what, crc);
            }
            let crc = (c.pattern_info)(cc.code, what, std::ptr::null_mut());
            let rrc = (r.pattern_info)(rr.code, what, std::ptr::null_mut());
            assert_eq!(crc, rrc, "pattern_info(code, {}, NULL) rc", what);
        }

        // rows 309-311: DEPTHLIMIT / HEAPLIMIT / MATCHLIMIT unset -> -55 with
        // UINT32_MAX still written; and SET via (*LIMIT_*) -> 0 with the value.
        for (what, verb) in [(21u32, "(*LIMIT_DEPTH=77)"), (25, "(*LIMIT_HEAP=88)"),
                             (14, "(*LIMIT_MATCH=99)")] {
            // unset
            let mut cv = 0u32;
            let mut rv = 0u32;
            let crc = (c.pattern_info)(cc.code, what, &mut cv as *mut _ as *mut c_void);
            let rrc = (r.pattern_info)(rr.code, what, &mut rv as *mut _ as *mut c_void);
            assert_eq!(crc, rrc, "unset info({}) rc", what);
            assert_eq!(crc, ERR_UNSET, "info({}) on a plain pattern must be UNSET", what);
            assert_eq!(cv, rv, "info({}) value written despite the error", what);
            assert_eq!(cv, u32::MAX, "UINT32_MAX is written");
            // set
            let pat = format!("{}a", verb).into_bytes();
            let c2 = compile_in(c, &pat, pat.len(), &CompileCfg::new(0));
            let r2 = compile_in(r, &pat, pat.len(), &CompileCfg::new(0));
            assert!(!c2.code.is_null(), "{} failed to compile", verb);
            let mut cv = 0u32;
            let mut rv = 0u32;
            let crc = (c.pattern_info)(c2.code, what, &mut cv as *mut _ as *mut c_void);
            let rrc = (r.pattern_info)(r2.code, what, &mut rv as *mut _ as *mut c_void);
            assert_eq!(crc, rrc, "set info({}) rc", what);
            assert_eq!(crc, 0, "{} must make info({}) available", verb, what);
            assert_eq!(cv, rv, "set info({}) value", what);
        }

        // row 307: BADMAGIC — a buffer that is not a compiled pattern
        let junk = [0u8; 256];
        for what in [0u32, 4, 22] {
            let mut cv = 0usize;
            let mut rv = 0usize;
            let crc = (c.pattern_info)(
                junk.as_ptr() as *const c_void, what, &mut cv as *mut _ as *mut c_void,
            );
            let rrc = (r.pattern_info)(
                junk.as_ptr() as *const c_void, what, &mut rv as *mut _ as *mut c_void,
            );
            assert_eq!(crc, rrc, "pattern_info(junk, {}) rc", what);
            assert_eq!(crc, ERR_BADMAGIC, "row307 expects PCRE2_ERROR_BADMAGIC");
        }
        // row 308: BADMODE — right magic, wrong code-unit width. Copy a real
        // code and corrupt only the mode field.
        {
            let cb = serialized_bytes(c, cc.code).unwrap();
            let _ = cb; // the serialized form is checked elsewhere
            let mut cbuf = vec![0u8; 4096];
            let mut rbuf = vec![0u8; 4096];
            let mut size = 0usize;
            (c.pattern_info)(cc.code, 22, &mut size as *mut _ as *mut c_void);
            assert!(size <= 4096);
            std::ptr::copy_nonoverlapping(cc.code as *const u8, cbuf.as_mut_ptr(), size);
            std::ptr::copy_nonoverlapping(rr.code as *const u8, rbuf.as_mut_ptr(), size);
            // `magic_number` is the first uint32, `compile_options`… the mode
            // bits live in the `flags`/`overall_options` area. Rather than guess,
            // flip the byte right after the magic number in BOTH copies the same
            // way and require the two libraries to agree on the verdict.
            for off in 4..12usize {
                let save_c = cbuf[off];
                let save_r = rbuf[off];
                cbuf[off] = 0xAB;
                rbuf[off] = 0xAB;
                let mut cv = 0usize;
                let mut rv = 0usize;
                let crc = (c.pattern_info)(
                    cbuf.as_ptr() as *const c_void, 0,
                    &mut cv as *mut _ as *mut c_void,
                );
                let rrc = (r.pattern_info)(
                    rbuf.as_ptr() as *const c_void, 0,
                    &mut rv as *mut _ as *mut c_void,
                );
                assert_eq!(crc, rrc, "corrupted byte {} rc", off);
                cbuf[off] = save_c;
                rbuf[off] = save_r;
            }
        }
    }
}

/// Rows 313-316: `pcre2_callout_enumerate_8`.
#[test]
fn rows313_316_callout_enumerate() {
    let _guard = global_lock();
    let (c, r) = both();

    // Records what the callback sees so the two libraries can be compared.
    static mut SEEN: [Vec<u64>; 2] = [Vec::new(), Vec::new()];
    static mut SLOT: usize = 0;
    static mut RETVAL: i32 = 0;

    unsafe extern "C" fn cb(block: *mut c_void, _data: *mut c_void) -> i32 {
        // pcre2_callout_enumerate_block starts with:
        //   uint32_t version; PCRE2_SIZE pattern_position;
        //   PCRE2_SIZE next_item_length; uint32_t callout_number;
        //   PCRE2_SIZE callout_string_offset; PCRE2_SIZE callout_string_length;
        //   PCRE2_SPTR callout_string;
        let w = block as *const usize;
        let v = *(block as *const u32);
        SEEN[SLOT].push(v as u64);
        SEEN[SLOT].push(*w.add(1) as u64); // pattern_position
        SEEN[SLOT].push(*w.add(2) as u64); // next_item_length
        SEEN[SLOT].push(*(w.add(3) as *const u32) as u64); // callout_number
        SEEN[SLOT].push(*w.add(4) as u64); // callout_string_offset
        SEEN[SLOT].push(*w.add(5) as u64); // callout_string_length
        RETVAL
    }

    unsafe {
        // row 313: code == NULL
        assert_eq!(
            (c.callout_enumerate)(std::ptr::null(), cb as *mut c_void, std::ptr::null_mut()),
            (r.callout_enumerate)(std::ptr::null(), cb as *mut c_void, std::ptr::null_mut()),
            "row313 callout_enumerate(NULL)"
        );
        assert_eq!(
            (c.callout_enumerate)(std::ptr::null(), cb as *mut c_void, std::ptr::null_mut()),
            ERR_NULL
        );

        // row 314: BADMAGIC
        let junk = [0u8; 256];
        assert_eq!(
            (c.callout_enumerate)(junk.as_ptr() as *const c_void, cb as *mut c_void, std::ptr::null_mut()),
            (r.callout_enumerate)(junk.as_ptr() as *const c_void, cb as *mut c_void, std::ptr::null_mut()),
            "row314 callout_enumerate(junk)"
        );

        // row 316 + the happy path: the callback sequence must be identical,
        // and a non-zero return must be propagated verbatim.
        let pats: [&[u8]; 10] = [
            b"a", b"(?C)a", b"(?C1)a(?C2)b", b"(?C255)x", b"(?C`str`)a",
            b"(?C{s})a", b"a(?C1)b(?C1)c", b"(?C0)(?C1)(?C2)", b"x",
            b"(?C1)(a)(?C2)(b)",
        ];
        for pat in pats {
            for ret in [0i32, 1, 7, -1, -99] {
                let mut rcs = [0i32; 2];
                for (i, api) in [c, r].iter().enumerate() {
                    let cfg = CompileCfg::new(0);
                    let comp = compile_in(api, pat, pat.len(), &cfg);
                    if comp.code.is_null() {
                        continue;
                    }
                    SLOT = i;
                    RETVAL = ret;
                    SEEN[i].clear();
                    rcs[i] = (api.callout_enumerate)(
                        comp.code, cb as *mut c_void, std::ptr::null_mut(),
                    );
                }
                assert_eq!(
                    rcs[0], rcs[1],
                    "callout_enumerate rc for {:?} ret={}",
                    String::from_utf8_lossy(pat), ret
                );
                assert_eq!(
                    SEEN[0], SEEN[1],
                    "callout_enumerate block sequence for {:?} ret={}",
                    String::from_utf8_lossy(pat), ret
                );
                if ret != 0 && !SEEN[0].is_empty() {
                    assert_eq!(
                        rcs[0], ret,
                        "row316: a non-zero callback return must be propagated"
                    );
                }
            }
        }
        // and with AUTO_CALLOUT, which inserts a callout at every item
        for pat in pats {
            let mut rcs = [0i32; 2];
            for (i, api) in [c, r].iter().enumerate() {
                let comp =
                    compile_in(api, pat, pat.len(), &CompileCfg::new(PCRE2_AUTO_CALLOUT));
                if comp.code.is_null() {
                    continue;
                }
                SLOT = i;
                RETVAL = 0;
                SEEN[i].clear();
                rcs[i] = (api.callout_enumerate)(
                    comp.code, cb as *mut c_void, std::ptr::null_mut(),
                );
            }
            assert_eq!(rcs[0], rcs[1], "AUTO_CALLOUT enumerate rc {:?}", pat);
            assert_eq!(
                SEEN[0], SEEN[1],
                "AUTO_CALLOUT enumerate blocks for {:?}",
                String::from_utf8_lossy(pat)
            );
        }
    }
}

/// Rows 296, 303, 317: documented UNDEFINED BEHAVIOUR in the C library. These
/// are NOT tested by calling them (the C library segfaults, so there is no
/// observable "correct" result to compare against); they are recorded here so
/// every ERRORS.md row is accounted for.
///
/// * row 296 — `pcre2_*_context_copy_8(NULL)` dereferences
///   `ctx->memctl.malloc` with no NULL check (`pcre2_context.c:381`).
/// * row 303 — `pcre2_config_8` with a `where` buffer smaller than the value.
/// * row 317 — `pcre2_callout_enumerate_8` with `callback == NULL` on a pattern
///   that contains a callout.
///
/// What we CAN verify is that the *documented* way of using each of them
/// behaves identically, which the tests above already do.
#[test]
fn rows296_303_317_undefined_behaviour_documented() {
    let (c, r) = both();
    unsafe {
        // row 317, safe half: callback == NULL on a pattern with NO callouts
        // never invokes the callback, so it is well-defined.
        for pat in [&b"a"[..], b"abc", b"(a)(b)"] {
            let cc = compile_in(c, pat, pat.len(), &CompileCfg::new(0));
            let rr = compile_in(r, pat, pat.len(), &CompileCfg::new(0));
            assert_eq!(
                (c.callout_enumerate)(cc.code, std::ptr::null_mut(), std::ptr::null_mut()),
                (r.callout_enumerate)(rr.code, std::ptr::null_mut(), std::ptr::null_mut()),
                "callout_enumerate(no callouts, NULL cb) for {:?}",
                String::from_utf8_lossy(pat)
            );
        }
    }
}
