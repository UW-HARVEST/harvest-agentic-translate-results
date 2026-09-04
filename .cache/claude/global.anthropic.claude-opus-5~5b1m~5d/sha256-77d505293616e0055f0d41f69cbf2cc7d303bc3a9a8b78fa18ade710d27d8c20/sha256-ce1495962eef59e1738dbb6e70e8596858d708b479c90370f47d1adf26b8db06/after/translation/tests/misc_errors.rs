//! Phase C — error-path differential tests for `ERRORS.md` sections
//! L (`pcre2_match_data_*`, `pcre2_code_copy*`, `pcre2_maketables*`,
//!    `pcre2_jit_*`, `pcre2_next_match_8`, `pcre2_get_error_message_8`) and
//! N (generic FFI-boundary rows for every public entry point).
mod common;

use common::diff::*;
use common::*;
use std::ffi::c_void;

// ============================================================ alloc injection
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
    if i == 0 { inj_malloc_c } else { inj_malloc_r }
}

// ============================================== Section L rows 334-338, 340
/// Rows 334, 335: `pcre2_match_data_create_8` CLAMPS oveccount (0 -> 1,
/// 0xffffffff -> 65535) instead of failing.
#[test]
fn rows334_335_match_data_create_clamps_oveccount() {
    let (c, r) = both();
    unsafe {
        for n in (0u32..8)
            .chain([100, 1000, 65534, 65535, 65536, 65537, 0x7FFF_FFFF, 0xFFFF_FFFF])
        {
            let cmd = (c.match_data_create)(n, std::ptr::null_mut());
            let rmd = (r.match_data_create)(n, std::ptr::null_mut());
            assert_eq!(
                cmd.is_null(), rmd.is_null(),
                "match_data_create({}) nullness", n
            );
            if cmd.is_null() {
                continue;
            }
            let cn = (c.get_ovector_count)(cmd);
            let rn = (r.get_ovector_count)(rmd);
            assert_eq!(cn, rn, "match_data_create({}) ovector count", n);
            // the table's expectations
            if n == 0 {
                assert_eq!(cn, 1, "row334: oveccount 0 clamps up to 1");
            }
            if n == 0xFFFF_FFFF {
                assert_eq!(cn, 65535, "row335: clamps down to UINT16_MAX");
            }
            // the reported sizes must match too
            assert_eq!(
                (c.get_match_data_size)(cmd), (r.get_match_data_size)(rmd),
                "get_match_data_size for oveccount={}", n
            );
            assert_eq!(
                (c.get_match_data_heapframes_size)(cmd),
                (r.get_match_data_heapframes_size)(rmd),
                "get_match_data_heapframes_size for oveccount={}", n
            );
            (c.match_data_free)(cmd);
            (r.match_data_free)(rmd);
        }
    }
}

/// Row 336: `pcre2_match_data_create_8` with a failing allocator -> NULL.
#[test]
fn row336_match_data_create_alloc_failure() {
    let _guard = global_lock();
    let (c, r) = both();
    unsafe {
        for (i, api) in [c, r].iter().enumerate() {
            ALLOC_N[i] = 0;
            ALLOC_FAIL_AT[i] = u32::MAX;
            let gc = (api.general_context_create)(
                Some(injector(i)), Some(inj_free), std::ptr::null_mut(),
            );
            assert!(!gc.is_null());
            for n in [1u32, 10, 100] {
                ALLOC_N[i] = 0;
                ALLOC_FAIL_AT[i] = 1;
                let md = (api.match_data_create)(n, gc);
                ALLOC_FAIL_AT[i] = u32::MAX;
                assert!(
                    md.is_null(),
                    "{}: match_data_create({}) must be NULL on alloc failure",
                    api.name, n
                );
            }
            (api.general_context_free)(gc);
        }
    }
}

/// Row 337: `match_data_create_from_pattern(NULL, …)` -> NULL.
/// Row 338: `match_data_free(NULL)` is a no-op.
#[test]
fn rows337_338_from_pattern_null_and_free_null() {
    let (c, r) = both();
    unsafe {
        let cmd = (c.match_data_create_from_pattern)(
            std::ptr::null(), std::ptr::null_mut(),
        );
        let rmd = (r.match_data_create_from_pattern)(
            std::ptr::null(), std::ptr::null_mut(),
        );
        assert_eq!(cmd.is_null(), rmd.is_null(), "row337 nullness");
        assert!(cmd.is_null(), "row337: NULL code must yield NULL");
        // row 338
        (c.match_data_free)(std::ptr::null_mut());
        (r.match_data_free)(std::ptr::null_mut());
    }
}

/// Row 340: `get_mark` returns NULL when the match set no mark, and the mark
/// string when it did.
#[test]
fn row340_get_mark_null_and_set() {
    unsafe {
        let cases: [(&[u8], &[u8]); 6] = [
            (b"a", b"a"),                     // no mark at all
            (b"(*MARK:m)a", b"a"),            // mark set
            (b"(*MARK:x)a|(*MARK:y)b", b"b"), // second alternative's mark
            (b"a(*MARK:z)b", b"ab"),
            (b"(*:q)a", b"a"),
            (b"(*MARK:m)a", b"zzz"), // no match -> mark may still be set
        ];
        for (pat, subj) in cases {
            let (cc, rr) = compile_both(pat, pat.len(), &CompileCfg::new(0), "row340");
            if cc.code.is_null() {
                continue;
            }
            for engine in [Engine::Interpreter, Engine::Dfa] {
                assert_match_eq(
                    &cc, &rr, subj, subj.len(), 0, &MatchCfg::new(0), engine,
                    &format!("row340 {:?} on {:?}", pat, subj),
                );
            }
        }
    }
}

/// Rows 341-345: `code_copy` / `code_copy_with_tables` with NULL and under
/// allocation failure; `code_free(NULL)`.
#[test]
fn rows341_345_code_copy_null_and_alloc_failure() {
    let _guard = global_lock();
    let (c, r) = both();
    unsafe {
        // rows 341, 343: NULL code
        for which in 0..2 {
            let cp = if which == 0 {
                (c.code_copy)(std::ptr::null())
            } else {
                (c.code_copy_with_tables)(std::ptr::null())
            };
            let rp = if which == 0 {
                (r.code_copy)(std::ptr::null())
            } else {
                (r.code_copy_with_tables)(std::ptr::null())
            };
            assert_eq!(cp.is_null(), rp.is_null(), "code_copy({}) NULL nullness", which);
            assert!(cp.is_null(), "rows341/343: NULL in -> NULL out");
        }

        // rows 342, 344: allocation failure during the copy (with_tables makes
        // TWO allocations; failing the second must still free the first)
        for pat in [&b"abc"[..], b"(a)(b)(c)", b"(?<n>x)+"] {
            for with_tables in [false, true] {
                for (i, api) in [c, r].iter().enumerate() {
                    ALLOC_N[i] = 0;
                    ALLOC_FAIL_AT[i] = u32::MAX;
                    let gc = (api.general_context_create)(
                        Some(injector(i)), Some(inj_free), std::ptr::null_mut(),
                    );
                    let cx = (api.compile_context_create)(gc);
                    let mut ec = 0i32;
                    let mut eo = 0usize;
                    let code =
                        (api.compile)(pat.as_ptr(), pat.len(), 0, &mut ec, &mut eo, cx);
                    assert!(!code.is_null());
                    // baseline: how many allocations does the copy take?
                    ALLOC_N[i] = 0;
                    let cp = if with_tables {
                        (api.code_copy_with_tables)(code)
                    } else {
                        (api.code_copy)(code)
                    };
                    assert!(!cp.is_null());
                    let nalloc = ALLOC_N[i];
                    (api.code_free)(cp);
                    // now fail each allocation in turn
                    for fail_at in 1..=nalloc {
                        ALLOC_N[i] = 0;
                        ALLOC_FAIL_AT[i] = fail_at;
                        let cp = if with_tables {
                            (api.code_copy_with_tables)(code)
                        } else {
                            (api.code_copy)(code)
                        };
                        ALLOC_FAIL_AT[i] = u32::MAX;
                        assert!(
                            cp.is_null(),
                            "{}: copy(tables={}) must be NULL when alloc {} fails",
                            api.name, with_tables, fail_at
                        );
                    }
                    (api.code_free)(code);
                    (api.compile_context_free)(cx);
                    (api.general_context_free)(gc);
                    // record the allocation count so the two libs can be compared
                    ALLOC_N[i] = nalloc;
                }
                assert_eq!(
                    ALLOC_N[0], ALLOC_N[1],
                    "pat={:?} with_tables={}: copy allocation COUNT differs (C={} Rust={})",
                    String::from_utf8_lossy(pat), with_tables, ALLOC_N[0], ALLOC_N[1]
                );
            }
        }

        // row 345
        (c.code_free)(std::ptr::null_mut());
        (r.code_free)(std::ptr::null_mut());
    }
}

/// Rows 346, 347: `maketables` under allocation failure; `maketables_free(NULL)`.
#[test]
fn rows346_347_maketables_alloc_failure_and_free_null() {
    let _guard = global_lock();
    let (c, r) = both();
    unsafe {
        for (i, api) in [c, r].iter().enumerate() {
            ALLOC_N[i] = 0;
            ALLOC_FAIL_AT[i] = u32::MAX;
            let gc = (api.general_context_create)(
                Some(injector(i)), Some(inj_free), std::ptr::null_mut(),
            );
            assert!(!gc.is_null());
            ALLOC_N[i] = 0;
            ALLOC_FAIL_AT[i] = 1;
            let t = (api.maketables)(gc);
            ALLOC_FAIL_AT[i] = u32::MAX;
            assert!(t.is_null(), "{}: row346 maketables must be NULL", api.name);
            (api.general_context_free)(gc);
            // row 347
            (api.maketables_free)(std::ptr::null_mut(), std::ptr::null());
        }
    }
}

/// Rows 348-358: the entire `pcre2_jit_*` surface in this NON-JIT build.
#[test]
fn rows348_358_jit_surface_non_jit_build() {
    let (c, r) = both();
    unsafe {
        // row 348: jit_compile(NULL, …). NOTE: the PCRE2_JIT_TEST_ALLOC branch
        // at pcre2_jit_compile.c:14316 runs BEFORE the NULL check, so
        // options 0x200 gives JIT_UNSUPPORTED and 0x201 gives JIT_BADOPTION
        // rather than PCRE2_ERROR_NULL. Both libraries must agree on all of it.
        for opts in [0u32, 1, 2, 3, 4, 0x200, 0x201, 0x8000, 0xFFFF_FFFF] {
            let crc = (c.jit_compile)(std::ptr::null_mut(), opts);
            let rrc = (r.jit_compile)(std::ptr::null_mut(), opts);
            assert_eq!(crc, rrc, "row348 jit_compile(NULL, {:#x})", opts);
            if opts & 0x200 == 0 {
                assert_eq!(
                    crc, ERR_NULL,
                    "row348: without TEST_ALLOC, NULL code gives PCRE2_ERROR_NULL"
                );
            }
        }
        assert_eq!(
            (c.jit_compile)(std::ptr::null_mut(), 0x200), -68,
            "row348: TEST_ALLOC alone short-circuits to JIT_UNSUPPORTED"
        );
        assert_eq!(
            (c.jit_compile)(std::ptr::null_mut(), 0x201), ERR_JIT_BADOPTION,
            "row348: TEST_ALLOC with other bits short-circuits to JIT_BADOPTION"
        );

        // rows 349-352: every option word on a REAL code
        let cc = compile_in(c, b"abc", 3, &CompileCfg::new(0));
        let rr = compile_in(r, b"abc", 3, &CompileCfg::new(0));
        let mut opts: Vec<u32> = (0..12u32).map(|b| 1u32 << b).collect();
        opts.extend([
            0, 0x201, 0x200, 1, 2, 3, 4, 7, 0x8000, 0xFFFF_FFFF, 0x7FFF_FFFF,
        ]);
        for o in opts {
            let crc = (c.jit_compile)(cc.code, o);
            let rrc = (r.jit_compile)(rr.code, o);
            assert_eq!(crc, rrc, "jit_compile(code, {:#x}) rc", o);
        }
        // the table's specific expectations
        assert_eq!((c.jit_compile)(cc.code, 0x8000), ERR_JIT_BADOPTION, "row349");
        assert_eq!((c.jit_compile)(cc.code, 1), ERR_JIT_BADOPTION, "row350");
        assert_eq!((c.jit_compile)(cc.code, 0x200), -68, "row351 JIT_UNSUPPORTED");
        assert_eq!((c.jit_compile)(cc.code, 0x201), ERR_JIT_BADOPTION, "row352");

        // row 354: jit_match always fails identically in a non-JIT build
        let subj = b"abc";
        for o in [0u32, PCRE2_ANCHORED, PCRE2_NOTBOL, 0xFFFF_FFFF] {
            let cmd = (c.match_data_create)(4, std::ptr::null_mut());
            let rmd = (r.match_data_create)(4, std::ptr::null_mut());
            let crc = (c.jit_match)(
                cc.code, subj.as_ptr(), 3, 0, o, cmd, std::ptr::null_mut(),
            );
            let rrc = (r.jit_match)(
                rr.code, subj.as_ptr(), 3, 0, o, rmd, std::ptr::null_mut(),
            );
            assert_eq!(crc, rrc, "row354 jit_match options={:#x}", o);
            assert_eq!(crc, ERR_JIT_BADOPTION, "row354 expects -45");
            (c.match_data_free)(cmd);
            (r.match_data_free)(rmd);
        }

        // rows 356-357: jit_stack_create always NULL here
        for (s, m) in [
            (0usize, 0usize), (0, 1), (1, 0), (1, 1), (32 * 1024, 1024 * 1024),
            (1, usize::MAX), (usize::MAX, usize::MAX), (1024, 512),
        ] {
            let cp = (c.jit_stack_create)(s, m, std::ptr::null_mut());
            let rp = (r.jit_stack_create)(s, m, std::ptr::null_mut());
            assert_eq!(
                cp.is_null(), rp.is_null(),
                "jit_stack_create({}, {}) nullness", s, m
            );
            assert!(cp.is_null(), "row357: always NULL in a non-JIT build");
        }

        // row 358: the NULL no-ops
        for api in [c, r] {
            (api.jit_stack_free)(std::ptr::null_mut());
            (api.jit_free_unused_memory)(std::ptr::null_mut());
            let mcx = (api.match_context_create)(std::ptr::null_mut());
            (api.jit_stack_assign)(mcx, std::ptr::null_mut(), std::ptr::null_mut());
            (api.match_context_free)(mcx);
        }

        // PCRE2_INFO_JITSIZE (10) must be 0 in both
        let mut cv = 0usize;
        let mut rv = 0usize;
        assert_eq!(
            (c.pattern_info)(cc.code, 10, &mut cv as *mut _ as *mut c_void),
            (r.pattern_info)(rr.code, 10, &mut rv as *mut _ as *mut c_void),
            "INFO_JITSIZE rc"
        );
        assert_eq!(cv, rv, "INFO_JITSIZE value");
        assert_eq!(cv, 0, "JITSIZE must be 0 in a non-JIT build");
    }
}

/// Rows 359, 360: `pcre2_next_match_8` after a failed match / after an empty
/// match at the end of the subject, plus full iteration sequences.
#[test]
fn rows359_360_next_match() {
    let (c, r) = both();
    unsafe {
        let cases: [(&[u8], &[u8]); 12] = [
            (b"a", b"zzz"),        // row 359: NOMATCH first
            (b"a", b"aaa"),
            (b"a*", b"aaa"),       // empty matches
            (b"a*", b""),          // row 360: empty match at the end
            (b"", b"abc"),
            (b"(a)|(b)", b"ab"),
            (b"\\b", b"a b"),
            (b"(?=a)", b"aaa"),
            (b"a|ab|abc", b"abc"),
            (b"(*MARK:m)a", b"aa"),
            (b"x", b""),
            (b"(a)(b)?", b"ab"),
        ];
        for (pat, subj) in cases {
            let (cc, rr) = compile_both(pat, pat.len(), &CompileCfg::new(0), "next_match");
            if cc.code.is_null() {
                continue;
            }
            for engine in [Engine::Interpreter, Engine::Dfa] {
                let mut seqs: [Vec<(i32, usize, u32)>; 2] = [Vec::new(), Vec::new()];
                for (i, (api, code)) in
                    [(c, cc.code), (r, rr.code)].iter().enumerate()
                {
                    let md = (api.match_data_create_from_pattern)(
                        *code, std::ptr::null_mut(),
                    );
                    // one real match first
                    let rc = match engine {
                        Engine::Dfa => {
                            let mut ws = [0i32; 1000];
                            (api.dfa_match)(
                                *code, subj.as_ptr(), subj.len(), 0, 0, md,
                                std::ptr::null_mut(), ws.as_mut_ptr(), ws.len(),
                            )
                        }
                        _ => (api.do_match)(
                            *code, subj.as_ptr(), subj.len(), 0, 0, md,
                            std::ptr::null_mut(),
                        ),
                    };
                    seqs[i].push((rc, 0, 0));
                    // then iterate next_match to exhaustion
                    for _ in 0..8 {
                        let mut so = 0xAAAAusize;
                        let mut op = 0xAAAAu32;
                        let nrc = (api.next_match)(md, &mut so, &mut op);
                        if nrc == 0 {
                            seqs[i].push((0, 0, 0));
                            break;
                        }
                        seqs[i].push((nrc, so, op));
                        let rc = match engine {
                            Engine::Dfa => {
                                let mut ws = [0i32; 1000];
                                (api.dfa_match)(
                                    *code, subj.as_ptr(), subj.len(), so, op, md,
                                    std::ptr::null_mut(), ws.as_mut_ptr(), ws.len(),
                                )
                            }
                            _ => (api.do_match)(
                                *code, subj.as_ptr(), subj.len(), so, op, md,
                                std::ptr::null_mut(),
                            ),
                        };
                        seqs[i].push((rc, 0, 0));
                        if rc < 0 {
                            break;
                        }
                    }
                    (api.match_data_free)(md);
                }
                assert_eq!(
                    seqs[0], seqs[1],
                    "next_match sequence differs for {:?} on {:?} ({:?})",
                    String::from_utf8_lossy(pat),
                    String::from_utf8_lossy(subj),
                    engine
                );
            }
        }
    }
}

/// Rows 362-366: `pcre2_get_error_message_8` — buffer-too-small and
/// out-of-range error numbers. (Already swept broadly in lowlevel.rs; here we
/// pin the exact table expectations.)
#[test]
fn rows362_366_get_error_message() {
    let (c, r) = both();
    unsafe {
        // row 362: size == 0 -> PCRE2_ERROR_NOMEMORY
        let mut buf = [0xAAu8; 64];
        assert_eq!(
            (c.get_error_message)(-1, buf.as_mut_ptr(), 0),
            (r.get_error_message)(-1, buf.as_mut_ptr(), 0),
            "row362 size=0"
        );
        assert_eq!(
            (c.get_error_message)(-1, buf.as_mut_ptr(), 0), ERR_NOMEMORY,
            "row362 expects PCRE2_ERROR_NOMEMORY"
        );
        // row 363: buffer smaller than the message
        for size in 1..24usize {
            let mut cb = [0xAAu8; 64];
            let mut rb = [0xAAu8; 64];
            let crc = (c.get_error_message)(-1, cb.as_mut_ptr(), size);
            let rrc = (r.get_error_message)(-1, rb.as_mut_ptr(), size);
            assert_eq!(crc, rrc, "row363 size={} rc", size);
            assert_eq!(cb, rb, "row363 size={} bytes", size);
        }
        // rows 364-366: out-of-range error numbers
        for e in [
            9999i32, -9999, 0, 1, 99, 226, 300, 1000, -100, -1000,
            i32::MIN, i32::MAX, i32::MIN + 1,
        ] {
            let mut cb = [0xAAu8; 256];
            let mut rb = [0xAAu8; 256];
            let crc = (c.get_error_message)(e, cb.as_mut_ptr(), 256);
            let rrc = (r.get_error_message)(e, rb.as_mut_ptr(), 256);
            assert_eq!(crc, rrc, "get_error_message({}) rc", e);
            assert_eq!(cb, rb, "get_error_message({}) bytes", e);
        }
        assert_eq!(
            (c.get_error_message)(9999, buf.as_mut_ptr(), 64), ERR_BADDATA,
            "row364"
        );
        assert_eq!(
            (c.get_error_message)(-9999, buf.as_mut_ptr(), 64), ERR_BADDATA,
            "row365"
        );
        assert_eq!(
            (c.get_error_message)(0, buf.as_mut_ptr(), 64), ERR_BADDATA,
            "row366"
        );
    }
}

// ============================================== Section N rows 390-410
/// Row 390: `0xffffffff` passed as an option/selector word to EVERY entry point
/// that takes one. C accepts any `uint32_t` where an enum is meant, so each of
/// these has a defined rejection that the Rust must reproduce.
#[test]
fn row390_all_ones_option_word_everywhere() {
    let (c, r) = both();
    unsafe {
        let cc = compile_in(c, b"abc", 3, &CompileCfg::new(0));
        let rr = compile_in(r, b"abc", 3, &CompileCfg::new(0));
        let subj = b"abc";
        let bad = 0xFFFF_FFFFu32;

        // compile -> ERR17
        let (a, b) = compile_both(b"abc", 3, &CompileCfg::new(bad), "row390 compile");
        assert!(a.code.is_null() && b.code.is_null());
        assert_eq!(a.errorcode, 117, "row390 compile expects ERR17");

        // config / pattern_info -> BADOPTION
        assert_eq!(
            (c.config)(bad, std::ptr::null_mut()),
            (r.config)(bad, std::ptr::null_mut()),
            "row390 config"
        );
        let mut cv = 0usize;
        let mut rv = 0usize;
        assert_eq!(
            (c.pattern_info)(cc.code, bad, &mut cv as *mut _ as *mut c_void),
            (r.pattern_info)(rr.code, bad, &mut rv as *mut _ as *mut c_void),
            "row390 pattern_info"
        );

        // match / dfa_match -> BADOPTION
        let cmd = (c.match_data_create)(4, std::ptr::null_mut());
        let rmd = (r.match_data_create)(4, std::ptr::null_mut());
        assert_eq!(
            (c.do_match)(cc.code, subj.as_ptr(), 3, 0, bad, cmd, std::ptr::null_mut()),
            (r.do_match)(rr.code, subj.as_ptr(), 3, 0, bad, rmd, std::ptr::null_mut()),
            "row390 match"
        );
        let mut cws = [0i32; 100];
        let mut rws = [0i32; 100];
        assert_eq!(
            (c.dfa_match)(
                cc.code, subj.as_ptr(), 3, 0, bad, cmd, std::ptr::null_mut(),
                cws.as_mut_ptr(), cws.len()
            ),
            (r.dfa_match)(
                rr.code, subj.as_ptr(), 3, 0, bad, rmd, std::ptr::null_mut(),
                rws.as_mut_ptr(), rws.len()
            ),
            "row390 dfa_match"
        );

        // substitute -> BADOPTION
        let repl = b"X";
        let mut cbuf = [0u8; 64];
        let mut rbuf = [0u8; 64];
        let mut cl = 64usize;
        let mut rl = 64usize;
        assert_eq!(
            (c.substitute)(
                cc.code, subj.as_ptr(), 3, 0, bad, cmd, std::ptr::null_mut(),
                repl.as_ptr(), 1, cbuf.as_mut_ptr(), &mut cl
            ),
            (r.substitute)(
                rr.code, subj.as_ptr(), 3, 0, bad, rmd, std::ptr::null_mut(),
                repl.as_ptr(), 1, rbuf.as_mut_ptr(), &mut rl
            ),
            "row390 substitute"
        );
        assert_eq!(cl, rl, "row390 substitute outlength");

        // convert -> BADOPTION
        let mut cb: *mut u8 = std::ptr::null_mut();
        let mut rb2: *mut u8 = std::ptr::null_mut();
        let mut ccl = 0usize;
        let mut rcl = 0usize;
        assert_eq!(
            (c.pattern_convert)(subj.as_ptr(), 3, bad, &mut cb, &mut ccl, std::ptr::null_mut()),
            (r.pattern_convert)(subj.as_ptr(), 3, bad, &mut rb2, &mut rcl, std::ptr::null_mut()),
            "row390 pattern_convert"
        );
        assert_eq!(ccl, rcl, "row390 convert bufflenptr");

        // set_bsr / set_newline / set_glob_* -> BADDATA; set_optimize -> BADOPTION
        let ccx = (c.compile_context_create)(std::ptr::null_mut());
        let rcx = (r.compile_context_create)(std::ptr::null_mut());
        assert_eq!((c.set_bsr)(ccx, bad), (r.set_bsr)(rcx, bad), "row390 set_bsr");
        assert_eq!((c.set_bsr)(ccx, bad), ERR_BADDATA);
        assert_eq!(
            (c.set_newline)(ccx, bad), (r.set_newline)(rcx, bad), "row390 set_newline"
        );
        assert_eq!((c.set_newline)(ccx, bad), ERR_BADDATA);
        assert_eq!(
            (c.set_optimize)(ccx, bad), (r.set_optimize)(rcx, bad),
            "row390 set_optimize"
        );
        assert_eq!((c.set_optimize)(ccx, bad), ERR_BADOPTION);
        (c.compile_context_free)(ccx);
        (r.compile_context_free)(rcx);
        let cvx = (c.convert_context_create)(std::ptr::null_mut());
        let rvx = (r.convert_context_create)(std::ptr::null_mut());
        assert_eq!(
            (c.set_glob_separator)(cvx, bad), (r.set_glob_separator)(rvx, bad),
            "row390 set_glob_separator"
        );
        assert_eq!(
            (c.set_glob_escape)(cvx, bad), (r.set_glob_escape)(rvx, bad),
            "row390 set_glob_escape"
        );
        (c.convert_context_free)(cvx);
        (r.convert_context_free)(rvx);

        (c.match_data_free)(cmd);
        (r.match_data_free)(rmd);
    }
}

/// Row 391: an option bit that is valid for a DIFFERENT function.
#[test]
fn row391_cross_function_option_bits() {
    let (c, r) = both();
    unsafe {
        let cc = compile_in(c, b"abc", 3, &CompileCfg::new(0));
        let rr = compile_in(r, b"abc", 3, &CompileCfg::new(0));
        let subj = b"abc";
        // bits that belong to other functions
        let foreign = [
            PCRE2_SUBSTITUTE_GLOBAL,
            PCRE2_SUBSTITUTE_EXTENDED,
            PCRE2_SUBSTITUTE_LITERAL,
            PCRE2_SUBSTITUTE_MATCHED,
            PCRE2_SUBSTITUTE_UNSET_EMPTY,
            PCRE2_SUBSTITUTE_UNKNOWN_UNSET,
            PCRE2_SUBSTITUTE_OVERFLOW_LENGTH,
            PCRE2_SUBSTITUTE_REPLACEMENT_ONLY,
            PCRE2_DFA_RESTART,
            PCRE2_DFA_SHORTEST,
            PCRE2_NO_JIT,
            PCRE2_CASELESS,
            PCRE2_MULTILINE,
            PCRE2_UTF,
        ];
        for o in foreign {
            let cmd = (c.match_data_create)(4, std::ptr::null_mut());
            let rmd = (r.match_data_create)(4, std::ptr::null_mut());
            // pcre2_match
            assert_eq!(
                (c.do_match)(cc.code, subj.as_ptr(), 3, 0, o, cmd, std::ptr::null_mut()),
                (r.do_match)(rr.code, subj.as_ptr(), 3, 0, o, rmd, std::ptr::null_mut()),
                "row391 match with {:#x}", o
            );
            // pcre2_dfa_match
            let mut cws = [0i32; 200];
            let mut rws = [0i32; 200];
            assert_eq!(
                (c.dfa_match)(
                    cc.code, subj.as_ptr(), 3, 0, o, cmd, std::ptr::null_mut(),
                    cws.as_mut_ptr(), cws.len()
                ),
                (r.dfa_match)(
                    rr.code, subj.as_ptr(), 3, 0, o, rmd, std::ptr::null_mut(),
                    rws.as_mut_ptr(), rws.len()
                ),
                "row391 dfa_match with {:#x}", o
            );
            (c.match_data_free)(cmd);
            (r.match_data_free)(rmd);
        }
        // PCRE2_PARTIAL_HARD|PCRE2_PARTIAL_SOFT together
        for o in [
            PCRE2_PARTIAL_HARD | PCRE2_PARTIAL_SOFT,
            PCRE2_ANCHORED | PCRE2_ENDANCHORED,
        ] {
            let cmd = (c.match_data_create)(4, std::ptr::null_mut());
            let rmd = (r.match_data_create)(4, std::ptr::null_mut());
            assert_eq!(
                (c.do_match)(cc.code, subj.as_ptr(), 3, 0, o, cmd, std::ptr::null_mut()),
                (r.do_match)(rr.code, subj.as_ptr(), 3, 0, o, rmd, std::ptr::null_mut()),
                "row391 match combo {:#x}", o
            );
            (c.match_data_free)(cmd);
            (r.match_data_free)(rmd);
        }
    }
}

/// Rows 394, 395, 396: length / start-offset boundaries.
#[test]
fn rows394_396_length_and_offset_boundaries() {
    let (c, r) = both();
    unsafe {
        // row 394: patlen = 0 with a non-NULL pattern -> matches the empty string
        let (cc, rr) = compile_both(b"ignored", 0, &CompileCfg::new(0), "row394");
        assert!(!cc.code.is_null(), "row394: zero-length pattern must compile");
        for subj in [&b""[..], b"a", b"abc"] {
            for engine in [Engine::Interpreter, Engine::Dfa] {
                assert_match_eq(
                    &cc, &rr, subj, subj.len(), 0, &MatchCfg::new(0), engine, "row394",
                );
            }
        }

        // rows 395, 396: start_offset at the end (legal) and way past it
        for pat in [&b"a"[..], b"", b"a*", b"^a", b"a$", b"(a)(b)"] {
            let (cc, rr) = compile_both(pat, pat.len(), &CompileCfg::new(0), "row395");
            if cc.code.is_null() {
                continue;
            }
            for subj in [&b""[..], b"a", b"ab", b"abc"] {
                // row 395: == length is LEGAL
                for engine in [Engine::Interpreter, Engine::Dfa] {
                    assert_match_eq(
                        &cc, &rr, subj, subj.len(), subj.len(),
                        &MatchCfg::new(0), engine,
                        &format!("row395 {:?} start==len", pat),
                    );
                }
                // row 396: past the end -> PCRE2_ERROR_BADOFFSET
                for start in [
                    subj.len() + 1, subj.len() + 2, subj.len() + 1000,
                    usize::MAX, usize::MAX - 1, usize::MAX / 2,
                ] {
                    let cmd = (c.match_data_create)(4, std::ptr::null_mut());
                    let rmd = (r.match_data_create)(4, std::ptr::null_mut());
                    let crc = (c.do_match)(
                        cc.code, subj.as_ptr(), subj.len(), start, 0, cmd,
                        std::ptr::null_mut(),
                    );
                    let rrc = (r.do_match)(
                        rr.code, subj.as_ptr(), subj.len(), start, 0, rmd,
                        std::ptr::null_mut(),
                    );
                    assert_eq!(
                        crc, rrc,
                        "row396 match {:?} start={} len={}",
                        pat, start, subj.len()
                    );
                    assert_eq!(crc, ERR_BADOFFSET, "row396 expects BADOFFSET");
                    let mut cws = [0i32; 200];
                    let mut rws = [0i32; 200];
                    let crc = (c.dfa_match)(
                        cc.code, subj.as_ptr(), subj.len(), start, 0, cmd,
                        std::ptr::null_mut(), cws.as_mut_ptr(), cws.len(),
                    );
                    let rrc = (r.dfa_match)(
                        rr.code, subj.as_ptr(), subj.len(), start, 0, rmd,
                        std::ptr::null_mut(), rws.as_mut_ptr(), rws.len(),
                    );
                    assert_eq!(
                        crc, rrc,
                        "row396 dfa {:?} start={} len={}",
                        pat, start, subj.len()
                    );
                    (c.match_data_free)(cmd);
                    (r.match_data_free)(rmd);
                }
            }
        }
    }
}

/// Rows 397, 398: `code` pointing at junk (BADMAGIC) and at bytes with the
/// right magic but corrupted mode bits (BADMODE), across EVERY entry point that
/// validates a code.
#[test]
fn rows397_398_badmagic_and_badmode_everywhere() {
    let (c, r) = both();
    unsafe {
        let subj = b"abc";
        // ---- row 397: arbitrary memory
        let junks: [Vec<u8>; 4] = [
            vec![0u8; 512],
            vec![0xFFu8; 512],
            (0..512u32).map(|i| i as u8).collect(),
            {
                let mut v = vec![0u8; 512];
                v[0] = 0x50;
                v[1] = 0x43;
                v[2] = 0x52;
                v[3] = 0x45;
                v
            },
        ];
        for junk in &junks {
            let p = junk.as_ptr() as *const c_void;
            let cmd = (c.match_data_create)(4, std::ptr::null_mut());
            let rmd = (r.match_data_create)(4, std::ptr::null_mut());

            let mut cv = 0usize;
            let mut rv = 0usize;
            assert_eq!(
                (c.pattern_info)(p, 0, &mut cv as *mut _ as *mut c_void),
                (r.pattern_info)(p, 0, &mut rv as *mut _ as *mut c_void),
                "row397 pattern_info"
            );
            assert_eq!(
                (c.do_match)(p, subj.as_ptr(), 3, 0, 0, cmd, std::ptr::null_mut()),
                (r.do_match)(p, subj.as_ptr(), 3, 0, 0, rmd, std::ptr::null_mut()),
                "row397 match"
            );
            let mut cws = [0i32; 200];
            let mut rws = [0i32; 200];
            assert_eq!(
                (c.dfa_match)(
                    p, subj.as_ptr(), 3, 0, 0, cmd, std::ptr::null_mut(),
                    cws.as_mut_ptr(), cws.len()
                ),
                (r.dfa_match)(
                    p, subj.as_ptr(), 3, 0, 0, rmd, std::ptr::null_mut(),
                    rws.as_mut_ptr(), rws.len()
                ),
                "row397 dfa_match"
            );
            assert_eq!(
                (c.callout_enumerate)(p, std::ptr::null_mut(), std::ptr::null_mut()),
                (r.callout_enumerate)(p, std::ptr::null_mut(), std::ptr::null_mut()),
                "row397 callout_enumerate"
            );
            // substitute
            let repl = b"X";
            let mut cbuf = [0u8; 64];
            let mut rbuf = [0u8; 64];
            let mut cl = 64usize;
            let mut rl = 64usize;
            assert_eq!(
                (c.substitute)(
                    p, subj.as_ptr(), 3, 0, 0, cmd, std::ptr::null_mut(),
                    repl.as_ptr(), 1, cbuf.as_mut_ptr(), &mut cl
                ),
                (r.substitute)(
                    p, subj.as_ptr(), 3, 0, 0, rmd, std::ptr::null_mut(),
                    repl.as_ptr(), 1, rbuf.as_mut_ptr(), &mut rl
                ),
                "row397 substitute"
            );
            // serialize_encode
            let codes = [p];
            let mut cbp: *mut u8 = std::ptr::null_mut();
            let mut rbp: *mut u8 = std::ptr::null_mut();
            let mut cn = 0usize;
            let mut rn = 0usize;
            assert_eq!(
                (c.serialize_encode)(codes.as_ptr(), 1, &mut cbp, &mut cn, std::ptr::null_mut()),
                (r.serialize_encode)(codes.as_ptr(), 1, &mut rbp, &mut rn, std::ptr::null_mut()),
                "row397 serialize_encode"
            );
            // NOTE (row 397a): `pcre2_code_copy_8`,
            // `pcre2_code_copy_with_tables_8` and the `pcre2_substring_*_8`
            // family are deliberately NOT called here. They perform NO magic
            // check — `pcre2_code_copy` immediately calls
            // `code->memctl.malloc(code->blocksize, …)` through a garbage
            // function pointer, and `pcre2_substring.c` has no BADMAGIC test at
            // all. Verified: the C `.so` segfaults, so there is no defined
            // result for the Rust to reproduce.

            (c.match_data_free)(cmd);
            (r.match_data_free)(rmd);
        }

        // ---- row 398: BADMAGIC and BADMODE, targeted precisely.
        //
        // pcre2_pattern_info.c:112 checks `re->magic_number != MAGIC_NUMBER`
        // (-> BADMAGIC) and :116 checks `(re->flags & 1) == 0` (-> BADMODE).
        // We corrupt ONLY those two fields: corrupting arbitrary bytes of a
        // compiled code (e.g. a length or an offset) passes both checks and then
        // makes the C library read out of bounds, which is UB rather than a
        // comparable observable.
        //
        // The offset of `magic_number` is found by SCANNING for MAGIC_NUMBER
        // rather than hard-coded, and `flags` sits 4 uint32 fields after it
        // (compile_options, overall_options, extra_options, flags).
        const MAGIC: u32 = 0x5043_5245;
        let cc = compile_in(c, b"a(b)c", 5, &CompileCfg::new(0));
        let rr = compile_in(r, b"a(b)c", 5, &CompileCfg::new(0));
        let mut size = 0usize;
        (c.pattern_info)(cc.code, 22, &mut size as *mut _ as *mut c_void);
        let mut cbuf = vec![0u8; size];
        let mut rbuf = vec![0u8; size];
        std::ptr::copy_nonoverlapping(cc.code as *const u8, cbuf.as_mut_ptr(), size);
        std::ptr::copy_nonoverlapping(rr.code as *const u8, rbuf.as_mut_ptr(), size);

        let find_magic = |buf: &[u8]| -> usize {
            for off in (0..buf.len().saturating_sub(4)).step_by(4) {
                let v = u32::from_ne_bytes([
                    buf[off], buf[off + 1], buf[off + 2], buf[off + 3],
                ]);
                if v == MAGIC {
                    return off;
                }
            }
            panic!("MAGIC_NUMBER not found in the compiled code");
        };
        let cm = find_magic(&cbuf);
        let rm = find_magic(&rbuf);
        assert_eq!(
            cm, rm,
            "magic_number is at a different struct offset in C ({}) and Rust ({})",
            cm, rm
        );
        let flags_off = cm + 16;

        // (a) BADMAGIC — corrupt the magic number
        for bad in [0u32, 0xFFFF_FFFF, MAGIC ^ 1, MAGIC.swap_bytes()] {
            cbuf[cm..cm + 4].copy_from_slice(&bad.to_ne_bytes());
            rbuf[rm..rm + 4].copy_from_slice(&bad.to_ne_bytes());
            let cp = cbuf.as_ptr() as *const c_void;
            let rp = rbuf.as_ptr() as *const c_void;
            let mut cv = 0usize;
            let mut rv = 0usize;
            let crc = (c.pattern_info)(cp, 0, &mut cv as *mut _ as *mut c_void);
            let rrc = (r.pattern_info)(rp, 0, &mut rv as *mut _ as *mut c_void);
            assert_eq!(crc, rrc, "row398 BADMAGIC magic={:#x} pattern_info", bad);
            assert_eq!(crc, ERR_BADMAGIC, "row398 expects BADMAGIC for {:#x}", bad);
            assert_eq!(
                (c.callout_enumerate)(cp, std::ptr::null_mut(), std::ptr::null_mut()),
                (r.callout_enumerate)(rp, std::ptr::null_mut(), std::ptr::null_mut()),
                "row398 BADMAGIC callout_enumerate"
            );
        }
        // restore the magic number
        cbuf[cm..cm + 4].copy_from_slice(&MAGIC.to_ne_bytes());
        rbuf[rm..rm + 4].copy_from_slice(&MAGIC.to_ne_bytes());

        // (b) BADMODE — valid magic, but the 8-bit mode flag cleared
        let cflags = u32::from_ne_bytes([
            cbuf[flags_off], cbuf[flags_off + 1],
            cbuf[flags_off + 2], cbuf[flags_off + 3],
        ]);
        let rflags = u32::from_ne_bytes([
            rbuf[flags_off], rbuf[flags_off + 1],
            rbuf[flags_off + 2], rbuf[flags_off + 3],
        ]);
        assert_eq!(cflags, rflags, "row398: `flags` differs between C and Rust");
        assert_eq!(cflags & 1, 1, "row398: PCRE2_MODE8 must be set on a real code");
        for newflags in [cflags & !1u32, 0u32, 0xFFFF_FFFEu32] {
            cbuf[flags_off..flags_off + 4].copy_from_slice(&newflags.to_ne_bytes());
            rbuf[flags_off..flags_off + 4].copy_from_slice(&newflags.to_ne_bytes());
            let cp = cbuf.as_ptr() as *const c_void;
            let rp = rbuf.as_ptr() as *const c_void;
            let mut cv = 0usize;
            let mut rv = 0usize;
            let crc = (c.pattern_info)(cp, 0, &mut cv as *mut _ as *mut c_void);
            let rrc = (r.pattern_info)(rp, 0, &mut rv as *mut _ as *mut c_void);
            assert_eq!(crc, rrc, "row398 BADMODE flags={:#x} pattern_info", newflags);
            assert_eq!(
                crc, -32,
                "row398 expects PCRE2_ERROR_BADMODE for flags={:#x}", newflags
            );
            assert_eq!(
                (c.callout_enumerate)(cp, std::ptr::null_mut(), std::ptr::null_mut()),
                (r.callout_enumerate)(rp, std::ptr::null_mut(), std::ptr::null_mut()),
                "row398 BADMODE callout_enumerate"
            );
        }
    }
}

/// Row 401: `stringnumber = 0xffffffff` and other out-of-range group numbers.
#[test]
fn row401_out_of_range_group_numbers() {
    let (c, r) = both();
    unsafe {
        for pat in [&b"a"[..], b"(a)", b"(a)(b)(c)", b"(?<n>a)"] {
            let (cc, rr) = compile_both(pat, pat.len(), &CompileCfg::new(0), "row401");
            if cc.code.is_null() {
                continue;
            }
            let subj = b"abc";
            for engine in [Engine::Interpreter, Engine::Dfa] {
                let cmd = (c.match_data_create_from_pattern)(cc.code, std::ptr::null_mut());
                let rmd = (r.match_data_create_from_pattern)(rr.code, std::ptr::null_mut());
                match engine {
                    Engine::Dfa => {
                        let mut cws = [0i32; 500];
                        let mut rws = [0i32; 500];
                        (c.dfa_match)(
                            cc.code, subj.as_ptr(), 3, 0, 0, cmd,
                            std::ptr::null_mut(), cws.as_mut_ptr(), cws.len(),
                        );
                        (r.dfa_match)(
                            rr.code, subj.as_ptr(), 3, 0, 0, rmd,
                            std::ptr::null_mut(), rws.as_mut_ptr(), rws.len(),
                        );
                    }
                    _ => {
                        (c.do_match)(
                            cc.code, subj.as_ptr(), 3, 0, 0, cmd, std::ptr::null_mut(),
                        );
                        (r.do_match)(
                            rr.code, subj.as_ptr(), 3, 0, 0, rmd, std::ptr::null_mut(),
                        );
                    }
                }
                for n in [0u32, 1, 2, 3, 4, 10, 100, 65535, 65536, 0x7FFF_FFFF, 0xFFFF_FFFF] {
                    // length_bynumber
                    let mut cs = 0xAAusize;
                    let mut rs = 0xAAusize;
                    let crc = (c.substring_length_bynumber)(cmd, n, &mut cs);
                    let rrc = (r.substring_length_bynumber)(rmd, n, &mut rs);
                    assert_eq!(
                        crc, rrc,
                        "row401 length_bynumber({}) rc for {:?} {:?}",
                        n, pat, engine
                    );
                    if crc == 0 {
                        assert_eq!(cs, rs, "row401 length_bynumber({}) value", n);
                    }
                    // copy_bynumber
                    let mut cb = [0xAAu8; 64];
                    let mut rb = [0xAAu8; 64];
                    let mut cl = 64usize;
                    let mut rl = 64usize;
                    let crc = (c.substring_copy_bynumber)(cmd, n, cb.as_mut_ptr(), &mut cl);
                    let rrc = (r.substring_copy_bynumber)(rmd, n, rb.as_mut_ptr(), &mut rl);
                    assert_eq!(crc, rrc, "row401 copy_bynumber({}) rc", n);
                    assert_eq!(cl, rl, "row401 copy_bynumber({}) length", n);
                    if crc == 0 {
                        assert_eq!(cb, rb, "row401 copy_bynumber({}) bytes", n);
                    }
                    // get_bynumber
                    let mut cp: *mut u8 = std::ptr::null_mut();
                    let mut rp: *mut u8 = std::ptr::null_mut();
                    let mut cl = 0usize;
                    let mut rl = 0usize;
                    let crc = (c.substring_get_bynumber)(cmd, n, &mut cp, &mut cl);
                    let rrc = (r.substring_get_bynumber)(rmd, n, &mut rp, &mut rl);
                    assert_eq!(crc, rrc, "row401 get_bynumber({}) rc", n);
                    if crc == 0 {
                        assert_eq!(cl, rl, "row401 get_bynumber({}) length", n);
                        assert_eq!(
                            std::slice::from_raw_parts(cp, cl),
                            std::slice::from_raw_parts(rp, rl),
                            "row401 get_bynumber({}) bytes", n
                        );
                        (c.substring_free)(cp);
                        (r.substring_free)(rp);
                    }
                }
                (c.match_data_free)(cmd);
                (r.match_data_free)(rmd);
            }
        }
    }
}

/// Rows 405, 407: the NULL no-ops, and a match_data created from a DIFFERENT
/// pattern (too few ovector pairs) -> rc 0, not an error.
#[test]
fn rows405_407_free_null_and_mismatched_match_data() {
    let (c, r) = both();
    unsafe {
        // row 405
        for api in [c, r] {
            (api.code_free)(std::ptr::null_mut());
            (api.match_data_free)(std::ptr::null_mut());
            (api.general_context_free)(std::ptr::null_mut());
            (api.compile_context_free)(std::ptr::null_mut());
            (api.match_context_free)(std::ptr::null_mut());
            (api.convert_context_free)(std::ptr::null_mut());
            (api.jit_stack_free)(std::ptr::null_mut());
            (api.maketables_free)(std::ptr::null_mut(), std::ptr::null());
            (api.converted_pattern_free)(std::ptr::null_mut());
            (api.substring_free)(std::ptr::null_mut());
            (api.substring_list_free)(std::ptr::null_mut());
            (api.serialize_free)(std::ptr::null_mut());
        }

        // row 407: match_data from a small pattern used with a big one
        let small = b"x";
        let bigs: [&[u8]; 4] = [
            b"(a)(b)(c)(d)(e)", b"(a)(b)", b"(?<p>a)(?<q>b)(?<r>c)", b"((((a))))",
        ];
        for big in bigs {
            let (cs, rs) = compile_both(small, small.len(), &CompileCfg::new(0), "row407s");
            let (cb2, rb2) = compile_both(big, big.len(), &CompileCfg::new(0), "row407b");
            if cs.code.is_null() || cb2.code.is_null() {
                continue;
            }
            let subj = b"abcde";
            for engine in [Engine::Interpreter, Engine::Dfa] {
                // ovector sized for `small`, used to match `big`
                let cmd = (c.match_data_create_from_pattern)(cs.code, std::ptr::null_mut());
                let rmd = (r.match_data_create_from_pattern)(rs.code, std::ptr::null_mut());
                let (crc, rrc) = match engine {
                    Engine::Dfa => {
                        let mut cws = [0i32; 1000];
                        let mut rws = [0i32; 1000];
                        (
                            (c.dfa_match)(
                                cb2.code, subj.as_ptr(), 5, 0, 0, cmd,
                                std::ptr::null_mut(), cws.as_mut_ptr(), cws.len(),
                            ),
                            (r.dfa_match)(
                                rb2.code, subj.as_ptr(), 5, 0, 0, rmd,
                                std::ptr::null_mut(), rws.as_mut_ptr(), rws.len(),
                            ),
                        )
                    }
                    _ => (
                        (c.do_match)(
                            cb2.code, subj.as_ptr(), 5, 0, 0, cmd, std::ptr::null_mut(),
                        ),
                        (r.do_match)(
                            rb2.code, subj.as_ptr(), 5, 0, 0, rmd, std::ptr::null_mut(),
                        ),
                    ),
                };
                assert_eq!(
                    crc, rrc,
                    "row407 {:?} with a small match_data ({:?})",
                    String::from_utf8_lossy(big), engine
                );
                if crc == 0 {
                    // all pairs were filled
                    let cn = (c.get_ovector_count)(cmd) as usize;
                    let cov = (c.get_ovector_pointer)(cmd);
                    let rov = (r.get_ovector_pointer)(rmd);
                    assert_eq!(
                        std::slice::from_raw_parts(cov, cn * 2),
                        std::slice::from_raw_parts(rov, cn * 2),
                        "row407 ovector when rc==0"
                    );
                }
                (c.match_data_free)(cmd);
                (r.match_data_free)(rmd);
            }
        }
    }
}

/// Rows 339, 361, 367, 392, 393, 399, 400, 402, 403, 404, 406, 408, 409, 410:
/// documented UNDEFINED BEHAVIOUR in the C library.
///
/// These are NOT invoked. Each was checked against the C source (and, where
/// cheap, confirmed by running the C `.so` in a throwaway process): the C code
/// has no NULL check / no bound check and dereferences the argument, so there is
/// no defined result for the Rust to reproduce. Calling them would crash BOTH
/// libraries identically, which tells us nothing. They are enumerated here so
/// no ERRORS.md row is silently unaccounted for.
///
/// * 339 — `get_mark`/`get_ovector_*`/`get_startchar`/`get_match_data_size`/
///   `get_match_data_heapframes_size` with `match_data == NULL`.
/// * 361 — `next_match` with NULL `match_data`/`pstart_offset`/`poptions`.
/// * 367 — `get_error_message(…, buffer == NULL, size != 0)`.
/// * 392 — `PCRE2_ZERO_TERMINATED` on a buffer with no NUL terminator.
/// * 393 — `length = SIZE_MAX - 1`.
/// * 399 — `wscount` larger than the real workspace.
/// * 400 — `number_of_codes` larger than the real array.
/// * 402 — `stringname == NULL` to the `*_byname` functions.
/// * 403 — `sizeptr == NULL` to `substring_copy_*`.
/// * 404 — freeing a pointer that did not come from the matching getter.
/// * 406 — double `pcre2_code_free_8`.
/// * 408 — `pcre2_substitute_8` with `buffer == NULL` and `*blength != 0`
///   (verified: the C library segfaults).
/// * 409 — `general_context_create(NULL, NULL, …)`.
/// * 410 — `set_character_tables(cx, NULL)` then compile.
///
/// What IS verified here: the well-defined NEIGHBOURS of each of those rows, so
/// the boundary itself is pinned down.
#[test]
fn undefined_behaviour_rows_documented() {
    let (c, r) = both();
    unsafe {
        // 392's defined half: PCRE2_ZERO_TERMINATED on a properly NUL-terminated
        // buffer must behave identically.
        for pat in [&b"abc\0"[..], b"\0", b"a(b)c\0", b"\\d+\0"] {
            let (cc, rr) =
                compile_both(pat, PCRE2_ZERO_TERMINATED, &CompileCfg::new(0), "row392");
            if cc.code.is_null() {
                continue;
            }
            for subj in [&b"abc\0"[..], b"\0", b"123\0"] {
                for engine in [Engine::Interpreter, Engine::Dfa] {
                    assert_match_eq(
                        &cc, &rr, subj, PCRE2_ZERO_TERMINATED, 0,
                        &MatchCfg::new(0), engine, "row392 zero-terminated subject",
                    );
                }
            }
        }

        // 400's defined half: number_of_codes EQUAL to the real count.
        let cc = compile_in(c, b"a", 1, &CompileCfg::new(0));
        let rr = compile_in(r, b"a", 1, &CompileCfg::new(0));
        let ccodes = [cc.code as *const c_void];
        let rcodes = [rr.code as *const c_void];
        let mut cbp: *mut u8 = std::ptr::null_mut();
        let mut rbp: *mut u8 = std::ptr::null_mut();
        let mut cn = 0usize;
        let mut rn = 0usize;
        let crc = (c.serialize_encode)(ccodes.as_ptr(), 1, &mut cbp, &mut cn, std::ptr::null_mut());
        let rrc = (r.serialize_encode)(rcodes.as_ptr(), 1, &mut rbp, &mut rn, std::ptr::null_mut());
        assert_eq!(crc, rrc, "row400 encode rc");
        assert_eq!(cn, rn, "row400 encode length");
        if crc == 1 {
            assert_eq!(
                std::slice::from_raw_parts(cbp, cn),
                std::slice::from_raw_parts(rbp, rn),
                "row400 encode bytes"
            );
            // and number_of_codes SMALLER than the blob's count is well-defined
            // (decode clamps), so check 0 and 1
            for n in [0i32, 1] {
                let mut cout = [std::ptr::null_mut(); 4];
                let mut rout = [std::ptr::null_mut(); 4];
                let cd = (c.serialize_decode)(cout.as_mut_ptr(), n, cbp, std::ptr::null_mut());
                let rd = (r.serialize_decode)(rout.as_mut_ptr(), n, rbp, std::ptr::null_mut());
                assert_eq!(cd, rd, "row400 decode({}) rc", n);
                for i in 0..(cd.max(0) as usize).min(4) {
                    if !cout[i].is_null() {
                        (c.code_free)(cout[i]);
                    }
                    if !rout[i].is_null() {
                        (r.code_free)(rout[i]);
                    }
                }
            }
            (c.serialize_free)(cbp);
            (r.serialize_free)(rbp);
        }

        // 403's defined half: a non-NULL sizeptr, including a zero *sizeptr.
        let (cc2, rr2) = compile_both(b"(a)(b)", 6, &CompileCfg::new(0), "row403");
        let subj = b"ab";
        let cmd = (c.match_data_create_from_pattern)(cc2.code, std::ptr::null_mut());
        let rmd = (r.match_data_create_from_pattern)(rr2.code, std::ptr::null_mut());
        (c.do_match)(cc2.code, subj.as_ptr(), 2, 0, 0, cmd, std::ptr::null_mut());
        (r.do_match)(rr2.code, subj.as_ptr(), 2, 0, 0, rmd, std::ptr::null_mut());
        for n in [0u32, 1, 2] {
            for start in [0usize, 1, 2, 64] {
                let mut cb = [0xAAu8; 64];
                let mut rb = [0xAAu8; 64];
                let mut cl = start;
                let mut rl = start;
                let crc = (c.substring_copy_bynumber)(cmd, n, cb.as_mut_ptr(), &mut cl);
                let rrc = (r.substring_copy_bynumber)(rmd, n, rb.as_mut_ptr(), &mut rl);
                assert_eq!(crc, rrc, "row403 copy_bynumber({}) *sizeptr={} rc", n, start);
                assert_eq!(cl, rl, "row403 copy_bynumber({}) *sizeptr={} out", n, start);
                assert_eq!(cb, rb, "row403 copy_bynumber({}) *sizeptr={} bytes", n, start);
            }
        }
        (c.match_data_free)(cmd);
        (r.match_data_free)(rmd);

        // 410's defined half: set_character_tables with the library's OWN tables.
        for api in [c, r] {
            let t = (api.maketables)(std::ptr::null_mut());
            let cx = (api.compile_context_create)(std::ptr::null_mut());
            assert_eq!((api.set_character_tables)(cx, t), 0);
            let mut ec = 0i32;
            let mut eo = 0usize;
            let code = (api.compile)(b"abc".as_ptr(), 3, 0, &mut ec, &mut eo, cx);
            assert!(!code.is_null(), "{}: own tables compile", api.name);
            (api.code_free)(code);
            (api.compile_context_free)(cx);
            (api.maketables_free)(std::ptr::null_mut(), t);
        }
    }
}
