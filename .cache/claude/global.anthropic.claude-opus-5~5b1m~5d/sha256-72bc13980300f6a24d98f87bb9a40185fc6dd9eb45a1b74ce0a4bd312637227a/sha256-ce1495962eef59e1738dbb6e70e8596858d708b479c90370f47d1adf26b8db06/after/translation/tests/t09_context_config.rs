//! Phase B — `pcre2_config`, the four context families and every context
//! setter, `pcre2_get_error_message`, `pcre2_maketables[_free]`, the
//! `pcre2_match_data_*` accessors, and the "NULL is a no-op" contract of every
//! `*_free` function.
//!
//! CONFIGS.md rows 1-8, 33-42, 43-46 · ERRORS.md rows 241-256.
#![allow(non_snake_case)]

mod common;
use common::corpus::*;
use common::*;
use std::ffi::c_void;
use std::os::raw::{c_char, c_int};

// --------------------------------------------------------- custom allocators

// The counters are thread-local because the libtest harness runs the tests in
// this file concurrently, and every test shares the same callback functions.
thread_local! {
    static T_NALLOC: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    static T_NFREE: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    static T_FAIL_AFTER: std::cell::Cell<i64> = const { std::cell::Cell::new(-1) };
    static T_UDATA: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

fn nalloc() -> u64 {
    T_NALLOC.with(|c| c.get())
}
fn nfree() -> u64 {
    T_NFREE.with(|c| c.get())
}
fn udata_seen() -> u64 {
    T_UDATA.with(|c| c.get())
}
fn set_fail_after(v: i64) {
    T_FAIL_AFTER.with(|c| c.set(v));
}
fn reset_alloc() {
    T_NALLOC.with(|c| c.set(0));
    T_NFREE.with(|c| c.set(0));
    T_FAIL_AFTER.with(|c| c.set(-1));
    T_UDATA.with(|c| c.set(0));
}

/// A tracking allocator. A 16-byte header keeps the layout size so that
/// `std::alloc::dealloc` can be used, and `data` is checked so that the
/// "memory_data" plumbing is exercised too.
unsafe extern "C" fn tr_malloc(size: usize, data: *mut c_void) -> *mut c_void {
    let n = T_NALLOC.with(|c| {
        c.set(c.get() + 1);
        c.get()
    });
    if !data.is_null() {
        let d = *(data as *const u64);
        T_UDATA.with(|c| c.set(c.get() + d));
    }
    let fa = T_FAIL_AFTER.with(|c| c.get());
    if fa >= 0 && n as i64 > fa {
        return std::ptr::null_mut();
    }
    let total = size + 16;
    let layout = std::alloc::Layout::from_size_align(total, 16).unwrap();
    let p = std::alloc::alloc(layout);
    if p.is_null() {
        return std::ptr::null_mut();
    }
    *(p as *mut usize) = total;
    p.add(16) as *mut c_void
}

unsafe extern "C" fn tr_free(block: *mut c_void, data: *mut c_void) {
    if block.is_null() {
        return;
    }
    T_NFREE.with(|c| c.set(c.get() + 1));
    if !data.is_null() {
        let d = *(data as *const u64);
        T_UDATA.with(|c| c.set(c.get() + d));
    }
    let p = (block as *mut u8).sub(16);
    let total = *(p as *mut usize);
    let layout = std::alloc::Layout::from_size_align(total, 16).unwrap();
    std::alloc::dealloc(p, layout);
}

static MEMDATA: u64 = 7;

/// Creates a general context with the tracking allocator (or NULL).
unsafe fn make_gcontext(api: &Api, custom: bool) -> GContext {
    if custom {
        (api.general_context_create)(
            Some(tr_malloc),
            Some(tr_free),
            &MEMDATA as *const u64 as *mut c_void,
        )
    } else {
        std::ptr::null_mut()
    }
}

// =========================================================== pcre2_config

/// Every `PCRE2_CONFIG_*` request, with an over-sized sink (so that the string
/// requests cannot overflow) and with `where == NULL` (the length query).
/// Out-of-range requests must all be `PCRE2_ERROR_BADOPTION`.
#[test]
fn config_every_request() {
    diff("config_every_request", |api| {
        let mut l = Log::new();
        unsafe {
            for what in 0..=16u32 {
                // A 256-byte sink: uint32 requests write 4 bytes, string
                // requests write the text plus a terminator, and the rest of
                // the buffer must stay zero in both libraries.
                let mut buf = [0u8; 256];
                let rc = (api.config)(what, buf.as_mut_ptr() as *mut c_void);
                l.tag("cfg").u(what as u64).i(rc as i64).b(&buf);
                // Length query.
                let rcn = (api.config)(what, std::ptr::null_mut());
                l.tag("cfgn").i(rcn as i64);
            }
            // The three string-valued requests, examined as C strings so the
            // returned length can be checked against the actual text.
            for what in [CONFIG_UNICODE_VERSION, CONFIG_VERSION, CONFIG_JITTARGET] {
                let mut buf = [0u8; 512];
                let rc = (api.config)(what, buf.as_mut_ptr() as *mut c_void);
                let s = cstr(buf.as_ptr());
                l.tag("str")
                    .u(what as u64)
                    .i(rc as i64)
                    .u(s.len() as u64)
                    .b(&s)
                    .b(&buf[..64]);
                let rcn = (api.config)(what, std::ptr::null_mut());
                l.i(rcn as i64);
            }
            // Out-of-range requests, both forms.
            for what in [17u32, 18, 25, 100, 255, 256, 9999, u32::MAX - 1, u32::MAX] {
                let mut buf = [0u8; 64];
                let rc = (api.config)(what, buf.as_mut_ptr() as *mut c_void);
                let rcn = (api.config)(what, std::ptr::null_mut());
                l.tag("bad").u(what as u64).i(rc as i64).i(rcn as i64).b(&buf);
            }
            // TABLES_LENGTH must agree with what maketables actually produces.
            let mut tl: u32 = 0;
            let rc = (api.config)(CONFIG_TABLES_LENGTH, &mut tl as *mut u32 as *mut c_void);
            l.tag("tl").i(rc as i64).u(tl as u64);
        }
        l
    });
}

// ================================================= contexts: create/copy/free

#[test]
fn context_create_copy_free_all() {
    diff("contexts", |api| {
        let mut l = Log::new();
        unsafe {
            for custom in [false, true] {
                reset_alloc();
                let gc = make_gcontext(api, custom);
                l.tag("gc").i(custom as i64).i(gc.is_null() as i64);
                if custom {
                    assert!(!gc.is_null());
                }

                // compile / match / convert contexts built on this gcontext.
                let cc = (api.compile_context_create)(gc);
                let mc = (api.match_context_create)(gc);
                let vc = (api.convert_context_create)(gc);
                l.i(cc.is_null() as i64)
                    .i(mc.is_null() as i64)
                    .i(vc.is_null() as i64);
                assert!(!cc.is_null() && !mc.is_null() && !vc.is_null());

                // Copies.
                let cc2 = (api.compile_context_copy)(cc);
                let mc2 = (api.match_context_copy)(mc);
                let vc2 = (api.convert_context_copy)(vc);
                l.i(cc2.is_null() as i64)
                    .i(mc2.is_null() as i64)
                    .i(vc2.is_null() as i64);

                // A copied compile context must compile identically.
                for (i, p) in ["a", "(a)(?<n>b)", "a\\Rb", "a.b"].iter().enumerate() {
                    for ctx in [cc, cc2] {
                        let code = compile_logged(api, p.as_bytes(), p.len(), 0, ctx, &mut l);
                        l.u(i as u64);
                        if !code.is_null() {
                            log_all_info(api, code, &mut l);
                            (api.code_free)(code);
                        }
                    }
                }

                (api.compile_context_free)(cc2);
                (api.match_context_free)(mc2);
                (api.convert_context_free)(vc2);
                (api.compile_context_free)(cc);
                (api.match_context_free)(mc);
                (api.convert_context_free)(vc);

                if custom {
                    let gc2 = (api.general_context_copy)(gc);
                    l.tag("gcopy").i(gc2.is_null() as i64);
                    // A context created from the copy must still work.
                    let cc3 = (api.compile_context_create)(gc2);
                    l.i(cc3.is_null() as i64);
                    (api.compile_context_free)(cc3);
                    (api.general_context_free)(gc2);
                    (api.general_context_free)(gc);
                    // Every allocation must have been paired with a free, and
                    // the memory_data pointer must have been handed back.
                    l.tag("bal").u(nalloc()).u(nfree()).i((udata_seen() > 0) as i64);
                }
            }

            // A general context with NULL callbacks falls back to the library
            // defaults (pcre2_context.c lines 115-116).
            let gd = (api.general_context_create)(None, None, std::ptr::null_mut());
            l.tag("gdef").i(gd.is_null() as i64);
            let cd = (api.compile_context_create)(gd);
            l.i(cd.is_null() as i64);
            let code = compile_logged(api, b"abc", 3, 0, cd, &mut l);
            if !code.is_null() {
                log_all_info(api, code, &mut l);
                (api.code_free)(code);
            }
            (api.compile_context_free)(cd);
            let gd2 = (api.general_context_copy)(gd);
            l.i(gd2.is_null() as i64);
            (api.general_context_free)(gd2);
            (api.general_context_free)(gd);
        }
        l
    });
}

/// A failing allocator must make every `*_create` return NULL rather than
/// crash (ERRORS: NOMEMORY on context creation).
#[test]
fn context_create_allocation_failure() {
    diff("context_alloc_fail", |api| {
        let mut l = Log::new();
        unsafe {
            reset_alloc();
            let gc = (api.general_context_create)(
                Some(tr_malloc),
                Some(tr_free),
                std::ptr::null_mut(),
            );
            assert!(!gc.is_null());
            for _ in 0..4 {
                T_NALLOC.with(|c| c.set(0));
                set_fail_after(0);
                let cc = (api.compile_context_create)(gc);
                let mc = (api.match_context_create)(gc);
                let vc = (api.convert_context_create)(gc);
                let md = (api.match_data_create)(4, gc);
                let tb = (api.maketables)(gc);
                l.i(cc.is_null() as i64)
                    .i(mc.is_null() as i64)
                    .i(vc.is_null() as i64)
                    .i(md.is_null() as i64)
                    .i(tb.is_null() as i64)
                    .u(nalloc());
                set_fail_after(-1);
                (api.compile_context_free)(cc);
                (api.match_context_free)(mc);
                (api.convert_context_free)(vc);
                (api.match_data_free)(md);
                (api.maketables_free)(gc, tb);
            }
            set_fail_after(-1);
            // A failing allocator must also make general_context_create fail.
            T_NALLOC.with(|c| c.set(0));
            set_fail_after(0);
            let g2 = (api.general_context_create)(
                Some(tr_malloc),
                Some(tr_free),
                std::ptr::null_mut(),
            );
            set_fail_after(-1);
            l.tag("gfail").i(g2.is_null() as i64);
            (api.general_context_free)(g2);
            (api.general_context_free)(gc);
        }
        l
    });
}

// ============================================== compile-context setters

/// Runs a compile + full-info + match sweep so that the *observable* effect of
/// a compile-context setting is compared, not just its return code.
unsafe fn probe_compiled(api: &Api, cc: CContext, pats: &[&str], subjects: &[&str], l: &mut Log) {
    for p in pats {
        let code = compile_logged(api, p.as_bytes(), p.len(), 0, cc, l);
        if code.is_null() {
            continue;
        }
        log_all_info(api, code, l);
        let md = (api.match_data_create_from_pattern)(code, std::ptr::null_mut());
        for s in subjects {
            let rc = (api.do_match)(
                code,
                s.as_bytes().as_ptr(),
                s.len(),
                0,
                0,
                md,
                std::ptr::null_mut(),
            );
            log_match_result(api, md, rc, l);
        }
        (api.match_data_free)(md);
        (api.code_free)(code);
    }
}

#[test]
fn set_bsr_all_values() {
    diff("set_bsr", |api| {
        let mut l = Log::new();
        unsafe {
            for v in [0u32, 1, 2, 3, 4, 5, 64, 100, 0xFFFF_FFFE, 0xFFFF_FFFF] {
                let cc = (api.compile_context_create)(std::ptr::null_mut());
                l.tag("bsr").u(v as u64).i((api.set_bsr)(cc, v) as i64);
                probe_compiled(
                    api,
                    cc,
                    &["a\\Rb", "\\R", "\\R+"],
                    &["a\nb", "a\rb", "a\r\nb", "a\u{b}b", "a\u{c}b", "a\u{85}b", "\n"],
                    &mut l,
                );
                (api.compile_context_free)(cc);
            }
        }
        l
    });
}

#[test]
fn set_newline_all_values() {
    diff("set_newline", |api| {
        let mut l = Log::new();
        unsafe {
            for v in [0u32, 1, 2, 3, 4, 5, 6, 7, 8, 100, 0xFFFF_FFFF] {
                let cc = (api.compile_context_create)(std::ptr::null_mut());
                l.tag("nl").u(v as u64).i((api.set_newline)(cc, v) as i64);
                probe_compiled(
                    api,
                    cc,
                    &["a.b", "(?m)^b", "(?m)a$", "\\N", "a$"],
                    &["a\nb", "a\rb", "a\r\nb", "a\0b", "a\u{85}b", "ab"],
                    &mut l,
                );
                (api.compile_context_free)(cc);
            }
        }
        l
    });
}

#[test]
fn set_size_and_count_limits() {
    diff("set_limits", |api| {
        let mut l = Log::new();
        unsafe {
            let deep: String = {
                let mut s = String::new();
                for _ in 0..40 {
                    s.push('(');
                }
                s.push('a');
                for _ in 0..40 {
                    s.push(')');
                }
                s
            };
            let pats: Vec<&str> = vec!["abc", "(a)(b)(c)", "(?<=a{1,30}b)c", &deep];
            for len in [0usize, 1, 3, 4, 100, usize::MAX - 1, PCRE2_UNSET] {
                let cc = (api.compile_context_create)(std::ptr::null_mut());
                l.tag("mpl")
                    .u(len as u64)
                    .i((api.set_max_pattern_length)(cc, len) as i64);
                probe_compiled(api, cc, &pats, &["abc", "aab", "aaabc"], &mut l);
                (api.compile_context_free)(cc);
            }
            for len in [0usize, 1, 16, 64, 4096, PCRE2_UNSET] {
                let cc = (api.compile_context_create)(std::ptr::null_mut());
                l.tag("mcl")
                    .u(len as u64)
                    .i((api.set_max_pattern_compiled_length)(cc, len) as i64);
                probe_compiled(api, cc, &pats, &["abc"], &mut l);
                (api.compile_context_free)(cc);
            }
            for v in [0u32, 1, 2, 30, 250, 255, 256, 1000, 0xFFFF_FFFF] {
                let cc = (api.compile_context_create)(std::ptr::null_mut());
                l.tag("pnl")
                    .u(v as u64)
                    .i((api.set_parens_nest_limit)(cc, v) as i64);
                l.tag("mvl")
                    .i((api.set_max_varlookbehind)(cc, v) as i64);
                probe_compiled(api, cc, &pats, &["abc"], &mut l);
                (api.compile_context_free)(cc);
            }
        }
        l
    });
}

#[test]
fn set_compile_extra_options_values() {
    diff("set_extra", |api| {
        let mut l = Log::new();
        unsafe {
            let mut vals: Vec<u32> = vec![0, 0xFFFF_FFFF, 0x8000_0000, 0x0FFF_FFFF];
            for b in 0..32u32 {
                vals.push(1u32 << b);
            }
            for v in vals {
                let cc = (api.compile_context_create)(std::ptr::null_mut());
                l.tag("xo")
                    .u(v as u64)
                    .i((api.set_compile_extra_options)(cc, v) as i64);
                probe_compiled(
                    api,
                    cc,
                    &["a", "\\d", "\\w", "(?C1)a", "\\ud800", "a\\C b"],
                    &["a", "1", "_"],
                    &mut l,
                );
                (api.compile_context_free)(cc);
            }
        }
        l
    });
}

/// `pcre2_set_optimize` — the only compile-context setter that checks for a
/// NULL context (pcre2_context.c line 414 → `PCRE2_ERROR_NULL`).
#[test]
fn set_optimize_values() {
    diff("set_optimize", |api| {
        let mut l = Log::new();
        unsafe {
            // ccontext == NULL must be PCRE2_ERROR_NULL for every directive.
            for d in [0u32, 1, 2, 63, 64, 65, 66, 67, 68, 69, 70, 100, 0xFFFF_FFFF] {
                l.tag("onull")
                    .u(d as u64)
                    .i((api.set_optimize)(std::ptr::null_mut(), d) as i64);
            }
            let directives: Vec<u32> = vec![
                0, 1, 2, 3, 10, 62, 63, 64, 65, 66, 67, 68, 69, 70, 71, 100, 1000, 0xFFFF_FFFE,
                0xFFFF_FFFF,
            ];
            for d in &directives {
                let cc = (api.compile_context_create)(std::ptr::null_mut());
                l.tag("opt").u(*d as u64).i((api.set_optimize)(cc, *d) as i64);
                probe_compiled(
                    api,
                    cc,
                    &["a+b", ".*abc", "abc", "(a|b)+c", "^a.*b$"],
                    &["abc", "aaab", "xxabc", "bbbc"],
                    &mut l,
                );
                (api.compile_context_free)(cc);
            }
            // Cumulative application: the flags word accumulates across calls.
            let cc = (api.compile_context_create)(std::ptr::null_mut());
            for d in &directives {
                l.tag("cum").i((api.set_optimize)(cc, *d) as i64);
            }
            probe_compiled(api, cc, &["a+b", ".*abc"], &["abc", "aaab"], &mut l);
            (api.compile_context_free)(cc);
        }
        l
    });
}

#[test]
fn set_character_tables_values() {
    diff("set_tables", |api| {
        let mut l = Log::new();
        unsafe {
            let tables = (api.maketables)(std::ptr::null_mut());
            l.tag("mk").i(tables.is_null() as i64);
            let cc = (api.compile_context_create)(std::ptr::null_mut());
            for t in [tables, std::ptr::null()] {
                l.i((api.set_character_tables)(cc, t) as i64);
                probe_compiled(
                    api,
                    cc,
                    &["(?i)abc", "[[:alpha:]]+", "\\w+", "\\d+", "[a-z]+"],
                    &["ABC", "abc", "a1_", "123", "  "],
                    &mut l,
                );
            }
            (api.compile_context_free)(cc);
            (api.maketables_free)(std::ptr::null_mut(), tables);
        }
        l
    });
}

// ================================================ match-context setters

#[test]
fn set_match_limits() {
    diff("set_match_limits", |api| {
        let mut l = Log::new();
        unsafe {
            // A backtracking pattern so the limits are actually reached, plus
            // a trivial one used with the huge limit values.
            let hard = "(a+)+b";
            let easy = "a(b)c";
            let hard_subj = "aaaaaaaaaaaaaaaaaaaac";
            for v in [0u32, 1, 2, 10, 100, 1000, 100000] {
                for (which, setter) in [
                    ("match", api.set_match_limit),
                    ("depth", api.set_depth_limit),
                    ("recur", api.set_recursion_limit),
                    ("heap", api.set_heap_limit),
                ] {
                    let mc = (api.match_context_create)(std::ptr::null_mut());
                    l.tag(which).u(v as u64).i(setter(mc, v) as i64);
                    for (p, s) in [(hard, hard_subj), (easy, "abc")] {
                        let code =
                            compile_logged(api, p.as_bytes(), p.len(), 0, std::ptr::null_mut(), &mut l);
                        if code.is_null() {
                            continue;
                        }
                        let md = (api.match_data_create_from_pattern)(code, std::ptr::null_mut());
                        let rc =
                            (api.do_match)(code, s.as_bytes().as_ptr(), s.len(), 0, 0, md, mc);
                        log_match_result(api, md, rc, &mut l);
                        l.u((api.get_match_data_heapframes_size)(md) as u64);
                        (api.match_data_free)(md);
                        (api.code_free)(code);
                    }
                    (api.match_context_free)(mc);
                }
            }
            // UINT32_MAX limits with a cheap pattern only.
            for (which, setter) in [
                ("mMax", api.set_match_limit),
                ("dMax", api.set_depth_limit),
                ("rMax", api.set_recursion_limit),
                ("hMax", api.set_heap_limit),
            ] {
                let mc = (api.match_context_create)(std::ptr::null_mut());
                l.tag(which).i(setter(mc, u32::MAX) as i64);
                let code =
                    compile_logged(api, easy.as_bytes(), easy.len(), 0, std::ptr::null_mut(), &mut l);
                let md = (api.match_data_create_from_pattern)(code, std::ptr::null_mut());
                let rc = (api.do_match)(code, b"abc".as_ptr(), 3, 0, 0, md, mc);
                log_match_result(api, md, rc, &mut l);
                (api.match_data_free)(md);
                (api.code_free)(code);
                (api.match_context_free)(mc);
            }
            // set_recursion_memory_management is a documented no-op since
            // 10.30 but must still return 0.
            let mc = (api.match_context_create)(std::ptr::null_mut());
            l.tag("rmm")
                .i((api.set_recursion_memory_management)(
                    mc,
                    Some(tr_malloc),
                    Some(tr_free),
                    std::ptr::null_mut(),
                ) as i64)
                .i((api.set_recursion_memory_management)(
                    mc,
                    None,
                    None,
                    std::ptr::null_mut(),
                ) as i64);
            let code = compile_logged(api, b"(a(?R)?b)", 9, 0, std::ptr::null_mut(), &mut l);
            let md = (api.match_data_create_from_pattern)(code, std::ptr::null_mut());
            let rc = (api.do_match)(code, b"aaabbb".as_ptr(), 6, 0, 0, md, mc);
            log_match_result(api, md, rc, &mut l);
            (api.match_data_free)(md);
            (api.code_free)(code);
            (api.match_context_free)(mc);
        }
        l
    });
}

#[test]
fn set_offset_limit_values() {
    diff("set_offset_limit", |api| {
        let mut l = Log::new();
        unsafe {
            let subj = b"xxxaxxxaxxxa";
            for opts in [0u32, PCRE2_USE_OFFSET_LIMIT] {
                for lim in [0usize, 1, 3, 4, 7, 11, 12, 13, 1000, PCRE2_UNSET] {
                    let mc = (api.match_context_create)(std::ptr::null_mut());
                    l.tag("ol")
                        .u(lim as u64)
                        .i((api.set_offset_limit)(mc, lim) as i64);
                    let code =
                        compile_logged(api, b"a", 1, opts, std::ptr::null_mut(), &mut l);
                    if !code.is_null() {
                        let md = (api.match_data_create_from_pattern)(code, std::ptr::null_mut());
                        for start in [0usize, 1, 4, 8] {
                            let rc = (api.do_match)(
                                code,
                                subj.as_ptr(),
                                subj.len(),
                                start,
                                0,
                                md,
                                mc,
                            );
                            log_match_result(api, md, rc, &mut l);
                        }
                        (api.match_data_free)(md);
                        (api.code_free)(code);
                    }
                    (api.match_context_free)(mc);
                }
            }
        }
        l
    });
}

// ============================================ pcre2_get_error_message

/// Every error number in the interesting range, with an ample buffer
/// (ERRORS rows 253-256).
#[test]
fn error_message_every_code() {
    diff("errmsg_all", |api| {
        let mut l = Log::new();
        unsafe {
            let mut buf = [0u8; 512];
            for e in -80i32..=225 {
                for b in buf.iter_mut() {
                    *b = 0xEE;
                }
                let rc = (api.get_error_message)(e as c_int, buf.as_mut_ptr(), buf.len());
                let s = cstr(buf.as_ptr());
                l.tag("em").i(e as i64).i(rc as i64).b(&s);
                // The byte just past the terminator must be untouched.
                if rc >= 0 {
                    l.u(buf[(rc as usize) + 1] as u64);
                }
            }
            // Extreme values.
            for e in [i32::MIN, i32::MIN + 1, -1000, -100, 226, 300, 10000, i32::MAX] {
                for b in buf.iter_mut() {
                    *b = 0xEE;
                }
                let rc = (api.get_error_message)(e as c_int, buf.as_mut_ptr(), buf.len());
                l.tag("emx").i(e as i64).i(rc as i64).b(&cstr(buf.as_ptr()));
            }
        }
        l
    });
}

/// The truncation path: buffer sizes 0,1,2,… up to len+2 for a spread of codes.
#[test]
fn error_message_truncation() {
    diff("errmsg_trunc", |api| {
        let mut l = Log::new();
        unsafe {
            let codes: [i32; 20] = [
                0, 1, 50, 99, 100, 101, 102, 106, 110, 120, 121, 220, 221, -1, -2, -3, -29, -34,
                -51, -64,
            ];
            for e in codes {
                let mut big = [0u8; 512];
                let full = (api.get_error_message)(e as c_int, big.as_mut_ptr(), big.len());
                let len = if full > 0 { full as usize } else { 0 };
                l.tag("full").i(e as i64).i(full as i64).u(len as u64);
                for size in 0..=(len + 3) {
                    let mut buf = [0xCDu8; 128];
                    let rc = (api.get_error_message)(e as c_int, buf.as_mut_ptr(), size);
                    l.u(size as u64).i(rc as i64).b(&buf[..64]);
                }
            }
        }
        l
    });
}

// ================================================ pcre2_maketables

#[test]
fn maketables_and_free() {
    diff("maketables", |api| {
        let mut l = Log::new();
        unsafe {
            // Default allocator.
            let t1 = (api.maketables)(std::ptr::null_mut());
            l.tag("t1").i(t1.is_null() as i64);
            assert!(!t1.is_null());
            l.b(std::slice::from_raw_parts(t1, TABLES_LENGTH));

            // Custom general context.
            reset_alloc();
            let gc = make_gcontext(api, true);
            assert!(!gc.is_null());
            let n0 = nalloc();
            let t2 = (api.maketables)(gc);
            l.tag("t2").i(t2.is_null() as i64).u(nalloc() - n0);
            assert!(!t2.is_null());
            let s1 = std::slice::from_raw_parts(t1, TABLES_LENGTH);
            let s2 = std::slice::from_raw_parts(t2, TABLES_LENGTH);
            // Same locale, so both table sets must be byte-identical.
            l.b(s2).i((s1 == s2) as i64);

            // The generated tables must also behave identically when used.
            let cc = (api.compile_context_create)(std::ptr::null_mut());
            l.i((api.set_character_tables)(cc, t2) as i64);
            probe_compiled(
                api,
                cc,
                &["(?i)aBc", "\\w+", "[[:punct:]]+", "[[:space:]]"],
                &["abc", "ABC", "a_1", "!?", " "],
                &mut l,
            );
            (api.compile_context_free)(cc);

            (api.maketables_free)(gc, t2);
            l.tag("freed").u(nfree());
            (api.general_context_free)(gc);
            (api.maketables_free)(std::ptr::null_mut(), t1);

            // maketables_free(NULL, NULL) must be a no-op, as must
            // maketables_free(gcontext, NULL).
            (api.maketables_free)(std::ptr::null_mut(), std::ptr::null());
            let gc2 = make_gcontext(api, true);
            (api.maketables_free)(gc2, std::ptr::null());
            (api.general_context_free)(gc2);
            l.tag("noop").i(0);
        }
        l
    });
}

/// The exported default tables must be byte-identical too.
#[test]
fn default_tables_data_symbol() {
    diff_data("_pcre2_default_tables_8", TABLES_LENGTH);
}

// ================================================ pcre2_match_data_*

#[test]
fn match_data_create_and_accessors() {
    diff("match_data_create", |api| {
        let mut l = Log::new();
        unsafe {
            for custom in [false, true] {
                reset_alloc();
                let gc = make_gcontext(api, custom);
                for ovec in [0u32, 1, 2, 3, 16, 1000, 65534, 65535, 65536, 100_000] {
                    let md = (api.match_data_create)(ovec, gc);
                    l.tag("mdc").u(ovec as u64).i(md.is_null() as i64);
                    if md.is_null() {
                        continue;
                    }
                    // Deterministic, initialised state only: the ovector and
                    // the mark/startchar fields are documented as valid only
                    // after a match.
                    l.u((api.get_ovector_count)(md) as u64)
                        .u((api.get_match_data_size)(md) as u64)
                        .u((api.get_match_data_heapframes_size)(md) as u64)
                        .i(((api.get_ovector_pointer)(md)).is_null() as i64);
                    (api.match_data_free)(md);
                }
                if custom {
                    l.tag("bal").u(nalloc()).u(nfree());
                    (api.general_context_free)(gc);
                }
            }
            // create_from_pattern with a NULL code must return NULL.
            l.tag("fpnull")
                .i((api.match_data_create_from_pattern)(
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                )
                .is_null() as i64);
            let gcx = make_gcontext(api, true);
            l.i((api.match_data_create_from_pattern)(std::ptr::null_mut(), gcx).is_null() as i64);
            (api.general_context_free)(gcx);
        }
        l
    });
}

#[test]
fn match_data_before_and_after_match() {
    let cases: &[(&str, &str)] = &[
        ("a", "xxa"),
        ("(a)(b)?", "ab"),
        ("(a)(b)?", "a"),
        ("(?<n>a)(b)(c)", "abc"),
        ("a(*MARK:m1)b", "ab"),
        ("a(*MARK:m1)b|c", "c"),
        ("(a+)+b", "aaab"),
        ("q", "abc"),
        ("(a(?R)?b)", "aabb"),
        (".*", "hello"),
    ];
    for (i, (p, s)) in cases.iter().enumerate() {
        diff(&format!("md_match[{i}]={p:?}/{s:?}"), |api| {
            let mut l = Log::new();
            unsafe {
                let code =
                    compile_logged(api, p.as_bytes(), p.len(), 0, std::ptr::null_mut(), &mut l);
                if code.is_null() {
                    return l;
                }
                for ovec in [0u32, 1, 2, 4, 32] {
                    for from_pattern in [false, true] {
                        let md = if from_pattern {
                            (api.match_data_create_from_pattern)(code, std::ptr::null_mut())
                        } else {
                            (api.match_data_create)(ovec, std::ptr::null_mut())
                        };
                        assert!(!md.is_null());
                        // Before: only the deterministic fields.
                        l.tag("pre")
                            .u((api.get_ovector_count)(md) as u64)
                            .u((api.get_match_data_size)(md) as u64)
                            .u((api.get_match_data_heapframes_size)(md) as u64);
                        let rc = (api.do_match)(
                            code,
                            s.as_bytes().as_ptr(),
                            s.len(),
                            0,
                            0,
                            md,
                            std::ptr::null_mut(),
                        );
                        l.tag("post").i(rc as i64);
                        log_match_result(api, md, rc, &mut l);
                        l.u((api.get_match_data_heapframes_size)(md) as u64);
                        // Reusing the same block for a second match must give
                        // the same answer and must not grow the heapframes.
                        let rc2 = (api.do_match)(
                            code,
                            s.as_bytes().as_ptr(),
                            s.len(),
                            0,
                            0,
                            md,
                            std::ptr::null_mut(),
                        );
                        log_match_result(api, md, rc2, &mut l);
                        l.u((api.get_match_data_heapframes_size)(md) as u64);
                        (api.match_data_free)(md);
                    }
                }
                (api.code_free)(code);
            }
            l
        });
    }
}

// ============================================ NULL is a no-op for every free

#[test]
fn all_free_functions_accept_null() {
    diff("null_frees", |api| {
        let mut l = Log::new();
        unsafe {
            // Each of these must return without touching memory.
            (api.match_data_free)(std::ptr::null_mut());
            (api.substring_free)(std::ptr::null_mut());
            (api.substring_list_free)(std::ptr::null_mut());
            (api.serialize_free)(std::ptr::null_mut());
            (api.converted_pattern_free)(std::ptr::null_mut());
            (api.general_context_free)(std::ptr::null_mut());
            (api.compile_context_free)(std::ptr::null_mut());
            (api.match_context_free)(std::ptr::null_mut());
            (api.convert_context_free)(std::ptr::null_mut());
            (api.code_free)(std::ptr::null_mut());
            (api.maketables_free)(std::ptr::null_mut(), std::ptr::null());
            (api.jit_stack_free)(std::ptr::null_mut());
            (api.jit_free_unused_memory)(std::ptr::null_mut());
            l.tag("survived").i(1);

            // Doing it twice, and after real work, must be just as safe.
            let code = compile_logged(api, b"(a)(b)", 6, 0, std::ptr::null_mut(), &mut l);
            let md = (api.match_data_create_from_pattern)(code, std::ptr::null_mut());
            let rc = (api.do_match)(code, b"ab".as_ptr(), 2, 0, 0, md, std::ptr::null_mut());
            log_match_result(api, md, rc, &mut l);
            (api.match_data_free)(md);
            (api.code_free)(code);
            (api.match_data_free)(std::ptr::null_mut());
            (api.code_free)(std::ptr::null_mut());
            (api.substring_free)(std::ptr::null_mut());
            (api.substring_list_free)(std::ptr::null_mut());
            (api.serialize_free)(std::ptr::null_mut());
            (api.converted_pattern_free)(std::ptr::null_mut());
            (api.jit_stack_free)(std::ptr::null_mut());
            l.tag("survived2").i(2);
        }
        l
    });
}

// =================================== cross-check: config vs context defaults

/// The values reported by `pcre2_config` must be the ones a freshly created
/// context actually uses, which is observable through `pcre2_pattern_info`.
#[test]
fn config_matches_context_defaults() {
    diff("config_vs_defaults", |api| {
        let mut l = Log::new();
        unsafe {
            for what in [
                CONFIG_BSR,
                CONFIG_NEWLINE,
                CONFIG_MATCHLIMIT,
                CONFIG_DEPTHLIMIT,
                CONFIG_HEAPLIMIT,
                CONFIG_PARENSLIMIT,
                CONFIG_LINKSIZE,
                CONFIG_EFFECTIVE_LINKSIZE,
                CONFIG_TABLES_LENGTH,
                CONFIG_UNICODE,
                CONFIG_JIT,
                CONFIG_STACKRECURSE,
                CONFIG_NEVER_BACKSLASH_C,
                CONFIG_COMPILED_WIDTHS,
            ] {
                let mut v: u32 = 0xDEAD_BEEF;
                let rc = (api.config)(what, &mut v as *mut u32 as *mut c_void);
                l.u(what as u64).i(rc as i64).u(v as u64);
            }
            // A default-context compile reports its own BSR/newline settings.
            let cc = (api.compile_context_create)(std::ptr::null_mut());
            for p in ["a", "a\\Rb", "a.b"] {
                for ctx in [cc, std::ptr::null_mut()] {
                    let code = compile_logged(api, p.as_bytes(), p.len(), 0, ctx, &mut l);
                    if !code.is_null() {
                        let mut bsr: u32 = 0;
                        let mut nl: u32 = 0;
                        (api.pattern_info)(code, INFO_BSR, &mut bsr as *mut u32 as *mut c_void);
                        (api.pattern_info)(code, INFO_NEWLINE, &mut nl as *mut u32 as *mut c_void);
                        l.u(bsr as u64).u(nl as u64);
                        (api.code_free)(code);
                    }
                }
            }
            (api.compile_context_free)(cc);
        }
        l
    });
}

/// A convert context reached through `pcre2_config`-independent defaults: the
/// separator/escape defaults must be identical in both libraries, which is
/// observable through a glob conversion with a NULL context.
#[test]
fn convert_context_defaults() {
    diff("convert_defaults", |api| {
        let mut l = Log::new();
        unsafe {
            for p in ["a*b", "a/b", "a\\b", "a.b", "**/c", "[a-z]/x"] {
                let mut buf: *mut u8 = std::ptr::null_mut();
                let mut bl: Sz = 0;
                let rc = (api.pattern_convert)(
                    p.as_bytes().as_ptr(),
                    p.len(),
                    PCRE2_CONVERT_GLOB,
                    &mut buf,
                    &mut bl,
                    std::ptr::null_mut(),
                );
                l.i(rc as i64).u(bl as u64);
                if !buf.is_null() {
                    if rc == 0 {
                        l.b(std::slice::from_raw_parts(buf, bl + 1));
                    }
                    (api.converted_pattern_free)(buf);
                }
                // …and through a freshly created (i.e. default-initialised)
                // convert context, which must behave the same.
                let vc = (api.convert_context_create)(std::ptr::null_mut());
                let mut buf2: *mut u8 = std::ptr::null_mut();
                let mut bl2: Sz = 0;
                let rc2 = (api.pattern_convert)(
                    p.as_bytes().as_ptr(),
                    p.len(),
                    PCRE2_CONVERT_GLOB,
                    &mut buf2,
                    &mut bl2,
                    vc,
                );
                l.i(rc2 as i64).u(bl2 as u64);
                if !buf2.is_null() {
                    if rc2 == 0 {
                        l.b(std::slice::from_raw_parts(buf2, bl2 + 1));
                    }
                    (api.converted_pattern_free)(buf2);
                }
                (api.convert_context_free)(vc);
            }
        }
        l
    });
}

// ============================================ convert-context setters

/// `pcre2_set_glob_separator` accepts only `/`, `\\` and `.`;
/// `pcre2_set_glob_escape` accepts 0 and any ASCII punctuation character
/// (pcre2_context.c lines 528-554). Both the return code and the resulting
/// behaviour are compared.
#[test]
fn set_glob_separator_and_escape_values() {
    diff("glob_setters", |api| {
        let mut l = Log::new();
        unsafe {
            // Return codes over the whole interesting range.
            let vc = (api.convert_context_create)(std::ptr::null_mut());
            assert!(!vc.is_null());
            for v in 0u32..=300 {
                l.u(v as u64)
                    .i((api.set_glob_separator)(vc, v) as i64)
                    .i((api.set_glob_escape)(vc, v) as i64);
            }
            for v in [
                0x10FFFFu32,
                0x8000_0000,
                0xFFFF_FFFE,
                0xFFFF_FFFF,
                b'a' as u32,
                b'A' as u32,
                b'0' as u32,
                b' ' as u32,
                0x7F,
                0x80,
                0xFF,
                0x100,
            ] {
                l.u(v as u64)
                    .i((api.set_glob_separator)(vc, v) as i64)
                    .i((api.set_glob_escape)(vc, v) as i64);
            }
            (api.convert_context_free)(vc);

            // Observable effect of every accepted separator/escape pair.
            let pats = [
                "a*b", "a/b", "a\\b", "a.b", "**/c", "[a-z]/x", "a!b", "a~b", "a%b", "a`b",
                "[!a-z]", "[.-/]", "*", "**", "?",
            ];
            for sep in [b'/' as u32, b'\\' as u32, b'.' as u32] {
                for esc in [
                    0u32,
                    b'!' as u32,
                    b'"' as u32,
                    b'#' as u32,
                    b'$' as u32,
                    b'%' as u32,
                    b'&' as u32,
                    b'\'' as u32,
                    b'(' as u32,
                    b')' as u32,
                    b'*' as u32,
                    b'+' as u32,
                    b',' as u32,
                    b'-' as u32,
                    b'.' as u32,
                    b'/' as u32,
                    b':' as u32,
                    b';' as u32,
                    b'<' as u32,
                    b'=' as u32,
                    b'>' as u32,
                    b'?' as u32,
                    b'@' as u32,
                    b'[' as u32,
                    b'\\' as u32,
                    b']' as u32,
                    b'^' as u32,
                    b'_' as u32,
                    b'`' as u32,
                    b'{' as u32,
                    b'|' as u32,
                    b'}' as u32,
                    b'~' as u32,
                ] {
                    let vc = (api.convert_context_create)(std::ptr::null_mut());
                    l.tag("set")
                        .i((api.set_glob_separator)(vc, sep) as i64)
                        .i((api.set_glob_escape)(vc, esc) as i64);
                    for p in pats {
                        let mut buf: *mut u8 = std::ptr::null_mut();
                        let mut bl: Sz = 0;
                        let rc = (api.pattern_convert)(
                            p.as_bytes().as_ptr(),
                            p.len(),
                            PCRE2_CONVERT_GLOB,
                            &mut buf,
                            &mut bl,
                            vc,
                        );
                        l.i(rc as i64).u(bl as u64);
                        if !buf.is_null() {
                            if rc == 0 {
                                l.b(std::slice::from_raw_parts(buf, bl + 1));
                            }
                            (api.converted_pattern_free)(buf);
                        }
                    }
                    (api.convert_context_free)(vc);
                }
            }
        }
        l
    });
}

// ============================================ randomized setter interactions

/// Random combinations of *all* compile- and match-context settings applied to
/// random patterns and subjects. Return codes, every `PCRE2_INFO_*` value and
/// the match outcome must all agree.
#[test]
fn random_setter_combinations() {
    let mut rng = Rng::new(0x0901_0001);
    for iter in 0..100000 {
        let pat = PatternGen::gen(&mut rng);
        let bsr = *rng.pick(&[0u32, 1, 2, 3, 0xFFFF_FFFF]);
        let nl = *rng.pick(&[0u32, 1, 2, 3, 4, 5, 6, 7, 0xFFFF_FFFF]);
        let opt = *rng.pick(&[0u32, 1, 2, 63, 64, 65, 66, 67, 68, 69, 70, 0xFFFF_FFFF]);
        let extra = *rng.pick(&[0u32, 1, 2, 4, 8, 0x80, 0x100, 0x8000]);
        let pnl = *rng.pick(&[0u32, 1, 5, 250, 1000]);
        let vlb = *rng.pick(&[0u32, 1, 20, 255, 0xFFFF_FFFF]);
        let mpl = *rng.pick(&[0usize, 1, 8, 1000, PCRE2_UNSET]);
        let mcl = *rng.pick(&[0usize, 1, 64, 4096, PCRE2_UNSET]);
        let ml = *rng.pick(&[0u32, 1, 100, 10000, u32::MAX]);
        let dl = *rng.pick(&[0u32, 1, 100, 10000, u32::MAX]);
        let hl = *rng.pick(&[0u32, 1, 100, 10000, u32::MAX]);
        let utf = rng.bool();
        let subj = gen_subject(&mut rng, utf);
        diff(&format!("rsc iter={iter} pat={pat:?}"), |api| {
            let mut l = Log::new();
            unsafe {
                let cc = (api.compile_context_create)(std::ptr::null_mut());
                let mc = (api.match_context_create)(std::ptr::null_mut());
                l.i((api.set_bsr)(cc, bsr) as i64)
                    .i((api.set_newline)(cc, nl) as i64)
                    .i((api.set_optimize)(cc, opt) as i64)
                    .i((api.set_compile_extra_options)(cc, extra) as i64)
                    .i((api.set_parens_nest_limit)(cc, pnl) as i64)
                    .i((api.set_max_varlookbehind)(cc, vlb) as i64)
                    .i((api.set_max_pattern_length)(cc, mpl) as i64)
                    .i((api.set_max_pattern_compiled_length)(cc, mcl) as i64)
                    .i((api.set_match_limit)(mc, ml) as i64)
                    .i((api.set_depth_limit)(mc, dl) as i64)
                    .i((api.set_heap_limit)(mc, hl) as i64)
                    .i((api.set_offset_limit)(mc, PCRE2_UNSET) as i64)
                    .i((api.set_recursion_limit)(mc, dl) as i64);
                let code = compile_logged(api, pat.as_bytes(), pat.len(), 0, cc, &mut l);
                if !code.is_null() {
                    log_all_info(api, code, &mut l);
                    let md = (api.match_data_create_from_pattern)(code, std::ptr::null_mut());
                    let sp = if subj.is_empty() {
                        b"\0".as_ptr()
                    } else {
                        subj.as_ptr()
                    };
                    let rc = (api.do_match)(code, sp, subj.len(), 0, 0, md, mc);
                    log_match_result(api, md, rc, &mut l);
                    l.u((api.get_match_data_heapframes_size)(md) as u64);
                    (api.match_data_free)(md);
                    (api.code_free)(code);
                }
                (api.match_context_free)(mc);
                (api.compile_context_free)(cc);
            }
            l
        });
    }
}

/// Fuzz `pcre2_config` with random request numbers: every unknown value must
/// be `PCRE2_ERROR_BADOPTION` and must not touch the caller's buffer.
#[test]
fn config_random_requests() {
    let mut rng = Rng::new(0x0901_0002);
    diff("config_random", |api| {
        let mut l = Log::new();
        unsafe {
            let mut r = Rng::new(0x0901_0002);
            for _ in 0..4000 {
                let what = match r.below(3) {
                    0 => r.next_u32(),
                    1 => r.below(64) as u32,
                    _ => r.below(20) as u32,
                };
                let mut buf = [0xA5u8; 512];
                let rc = (api.config)(what, buf.as_mut_ptr() as *mut c_void);
                let rcn = (api.config)(what, std::ptr::null_mut());
                l.u(what as u64).i(rc as i64).i(rcn as i64).b(&buf[..96]);
            }
        }
        l
    });
    let _ = rng.next_u32();
}

// ---------------------------------------------------------------- unused imports
#[allow(unused)]
fn _keep(_: &[&str]) {
    let _ = SUBJECTS;
    let _: Option<*const c_char> = None;
}
