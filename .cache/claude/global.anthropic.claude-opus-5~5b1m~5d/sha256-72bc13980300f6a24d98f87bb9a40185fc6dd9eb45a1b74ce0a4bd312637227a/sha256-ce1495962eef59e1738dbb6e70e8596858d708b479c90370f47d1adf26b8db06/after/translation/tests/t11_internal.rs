//! Phase B — the remaining exported `_pcre2_*` entry points that have a
//! self-contained contract, called DIRECTLY through the `.so` exports:
//! `_pcre2_find_bracket_8`, `_pcre2_study_8`, `_pcre2_memctl_malloc_8`,
//! `_pcre2_jit_free_8`, `_pcre2_jit_free_rodata_8`.
//!
//! The helpers whose parameters are private compile-time structures
//! (`_pcre2_check_escape_8`, `_pcre2_auto_possessify_8`,
//! `_pcre2_update_classbits_8`, `_pcre2_compile_*`) and the ones that take a
//! pointer into the middle of compiled bytecode together with a
//! `char_lists_end` cursor (`_pcre2_xclass_8`, `_pcre2_eclass_8`) cannot be
//! invoked from outside the library without reconstructing those private types;
//! they are covered end-to-end here instead, by compiling patterns that are
//! guaranteed to produce the corresponding opcodes and sweeping the whole code
//! point space through `pcre2_match`.
//!
//! CONFIGS.md rows 20-24, 27.
#![allow(non_snake_case)]

mod common;
use common::corpus::*;
use common::*;
use std::ffi::c_void;
use std::os::raw::c_int;

/// `LINK_SIZE` from `c_src/src/config.h:178`.
const LINK_SIZE: usize = 2;

/// `code_start` is the `CODE_BLOCKSIZE_TYPE` field immediately preceding
/// `magic_number` in `pcre2_real_code` (pcre2_intmodedep.h:666-667).
unsafe fn bytecode_ptr(code: Code) -> *const u8 {
    let m = magic_ptr(code) as *const u8;
    let code_start = *(m.sub(std::mem::size_of::<Sz>()) as *const Sz);
    (code as *const u8).add(code_start)
}

// ------------------------------------------------------------- find_bracket

#[test]
fn find_bracket_all_numbers() {
    for (pi, p) in PATTERNS.iter().enumerate() {
        for copts in [0u32, PCRE2_UTF, PCRE2_UCP, PCRE2_DUPNAMES, PCRE2_NO_AUTO_CAPTURE] {
            diff(&format!("find_bracket pat[{pi}] copts={copts:#x}"), |api| {
                let mut l = Log::new();
                unsafe {
                    let code =
                        compile_logged(api, p.as_bytes(), p.len(), copts, std::ptr::null_mut(), &mut l);
                    if code.is_null() {
                        return l;
                    }
                    let mut top: u32 = 0;
                    (api.pattern_info)(
                        code,
                        INFO_CAPTURECOUNT,
                        &mut top as *mut u32 as *mut c_void,
                    );
                    let bc = bytecode_ptr(code);
                    let utf = if copts & PCRE2_UTF != 0 { 1 } else { 0 };
                    // Every bracket number, plus one past the top, plus the
                    // "find a lookbehind" negative-number form.
                    for n in -3i32..=(top as i32 + 2) {
                        let r = (api.p_find_bracket)(bc, utf, n);
                        // Log the RESULT AS AN OFFSET, never as an address.
                        let off = if r.is_null() {
                            u64::MAX
                        } else {
                            (r as usize - bc as usize) as u64
                        };
                        l.i(n as i64).u(off);
                    }
                    (api.code_free)(code);
                }
                l
            });
        }
    }
}

#[test]
fn find_bracket_random_patterns() {
    let mut rng = Rng::new(0x1234_0001);
    for iter in 0..1500 {
        let pat = PatternGen::gen(&mut rng);
        let utf = rng.bool();
        let copts = if utf { PCRE2_UTF } else { 0 };
        diff(&format!("find_bracket_rand {iter} pat={pat:?}"), |api| {
            let mut l = Log::new();
            unsafe {
                let code =
                    compile_logged(api, pat.as_bytes(), pat.len(), copts, std::ptr::null_mut(), &mut l);
                if code.is_null() {
                    return l;
                }
                let mut top: u32 = 0;
                (api.pattern_info)(code, INFO_CAPTURECOUNT, &mut top as *mut u32 as *mut c_void);
                let bc = bytecode_ptr(code);
                for n in -1i32..=(top as i32 + 1) {
                    let r = (api.p_find_bracket)(bc, if utf { 1 } else { 0 }, n);
                    let off = if r.is_null() {
                        u64::MAX
                    } else {
                        (r as usize - bc as usize) as u64
                    };
                    l.u(off);
                }
                (api.code_free)(code);
            }
            l
        });
    }
}

// -------------------------------------------------------------------- study

/// `_pcre2_study` is idempotent-ish: re-running it on an already compiled code
/// must give the same return code and leave the same start bitmap / minlength.
#[test]
fn study_rerun() {
    for (pi, p) in PATTERNS.iter().enumerate() {
        for copts in [
            0u32,
            PCRE2_UTF,
            PCRE2_UCP,
            PCRE2_CASELESS,
            PCRE2_NO_START_OPTIMIZE,
            PCRE2_ANCHORED,
            PCRE2_MULTILINE,
        ] {
            diff(&format!("study pat[{pi}] copts={copts:#x}"), |api| {
                let mut l = Log::new();
                unsafe {
                    let code =
                        compile_logged(api, p.as_bytes(), p.len(), copts, std::ptr::null_mut(), &mut l);
                    if code.is_null() {
                        return l;
                    }
                    log_all_info(api, code, &mut l);
                    let rc = (api.p_study)(code);
                    l.tag("study").i(rc as i64);
                    log_all_info(api, code, &mut l);
                    // and again
                    let rc = (api.p_study)(code);
                    l.tag("study2").i(rc as i64);
                    log_all_info(api, code, &mut l);
                    (api.code_free)(code);
                }
                l
            });
        }
    }
}

// ------------------------------------------------------------ memctl_malloc

/// Mirrors `pcre2_memctl` (pcre2_internal.h): two function pointers and a
/// user-data pointer.
#[repr(C)]
struct MemCtl {
    malloc: Option<unsafe extern "C" fn(usize, *mut c_void) -> *mut c_void>,
    free: Option<unsafe extern "C" fn(*mut c_void, *mut c_void)>,
    memory_data: *mut c_void,
}

thread_local! {
    static ALLOC_LOG: std::cell::RefCell<Vec<u64>> = std::cell::RefCell::new(Vec::new());
}

unsafe extern "C" fn t_malloc(n: usize, _d: *mut c_void) -> *mut c_void {
    ALLOC_LOG.with(|v| v.borrow_mut().push(n as u64));
    c_malloc(n)
}
unsafe extern "C" fn t_free(p: *mut c_void, _d: *mut c_void) {
    c_free(p)
}
/// An allocator that always fails, so the NULL-return path is reached.
unsafe extern "C" fn t_malloc_fail(n: usize, _d: *mut c_void) -> *mut c_void {
    ALLOC_LOG.with(|v| v.borrow_mut().push(n as u64));
    std::ptr::null_mut()
}

extern "C" {
    #[link_name = "malloc"]
    fn c_malloc(n: usize) -> *mut c_void;
    #[link_name = "free"]
    fn c_free(p: *mut c_void);
}

#[test]
fn memctl_malloc_direct() {
    let sizes: Vec<usize> = vec![
        0,
        1,
        7,
        8,
        24,
        25,
        32,
        64,
        1024,
        4096,
        1 << 20,
        usize::MAX, // must fail
        usize::MAX - 24,
    ];
    diff("memctl_malloc", |api| {
        let mut l = Log::new();
        for &sz in &sizes {
            for fail in [false, true] {
                ALLOC_LOG.with(|v| v.borrow_mut().clear());
                let mut mc = MemCtl {
                    malloc: Some(if fail { t_malloc_fail } else { t_malloc }),
                    free: Some(t_free),
                    memory_data: 0x1234_5678 as *mut c_void,
                };
                let p = unsafe {
                    (api.p_memctl_malloc)(sz, &mut mc as *mut MemCtl as *mut c_void)
                };
                l.u(sz as u64).i(p.is_null() as i64);
                // the requested byte count the library asked our allocator for
                ALLOC_LOG.with(|v| {
                    for n in v.borrow().iter() {
                        l.u(*n);
                    }
                });
                if !p.is_null() {
                    // The block starts with a copy of the memctl; verify that
                    // the user-data pointer round-tripped.
                    let got = unsafe { &*(p as *const MemCtl) };
                    l.u(got.memory_data as u64);
                    l.i(got.malloc.is_some() as i64);
                    l.i(got.free.is_some() as i64);
                    unsafe { t_free(p, std::ptr::null_mut()) };
                }
            }
        }
        l
    });
}

// -------------------------------------------------------------- jit stubs

#[test]
fn jit_free_stubs_are_noops() {
    diff("jit_free_stubs", |api| {
        let mut l = Log::new();
        unsafe {
            let mut mc = MemCtl {
                malloc: Some(t_malloc),
                free: Some(t_free),
                memory_data: std::ptr::null_mut(),
            };
            (api.p_jit_free)(std::ptr::null_mut(), &mut mc as *mut MemCtl as *mut c_void);
            (api.p_jit_free_rodata)(std::ptr::null_mut(), std::ptr::null_mut());
            l.u((api.p_jit_get_size)(std::ptr::null_mut()) as u64);
            l.b(&cstr((api.p_jit_get_target)() as *const u8));
            l.tag("ok");
        }
        l
    });
}

// ------------------------------------------------ xclass / eclass end-to-end

/// Patterns that force `OP_XCLASS` (a class that cannot fit in a bitmap) and
/// `OP_ECLASS` (a class built with set operations). Every one is matched
/// against a wide sweep of code points so that both helpers' full decision
/// tables are exercised through the public API.
#[test]
fn xclass_and_eclass_code_point_sweep() {
    let xclass_pats: &[(&str, u32)] = &[
        (r"[\x{100}-\x{200}]", PCRE2_UTF),
        (r"[^\x{100}]", PCRE2_UTF),
        (r"[\x{100}\x{500}\x{1000}]", PCRE2_UTF),
        (r"[\p{L}]", PCRE2_UTF),
        (r"[^\p{L}]", PCRE2_UTF),
        (r"[\p{Greek}\p{Cyrillic}]", PCRE2_UTF),
        (r"[\p{Nd}a-f]", PCRE2_UTF),
        (r"[\x{80}-\x{10FFFF}]", PCRE2_UTF),
        (r"[\x{100}-\x{200}\p{Han}]", PCRE2_UTF),
        (r"[[:alpha:]\x{300}]", PCRE2_UTF),
        (r"[\x{100}-\x{200}]", PCRE2_UTF | PCRE2_CASELESS),
        (r"[\p{Lu}]", PCRE2_UTF | PCRE2_CASELESS),
        (r"[\p{Ll}]", PCRE2_UTF | PCRE2_UCP),
        (r"[\x{a0}-\x{ff}]", PCRE2_UTF),
        (r"[^\x{a0}-\x{ff}]", PCRE2_UTF),
        // non-UTF but still high-valued / negated-high classes
        (r"[\x80-\xff]", 0),
        (r"[^\x80-\xff]", 0),
        (r"[\p{L}]", PCRE2_UCP),
    ];
    let eclass_pats: &[(&str, u32)] = &[
        (r"[[\p{L}]&&[a-z]]", PCRE2_UTF | PCRE2_ALT_EXTENDED_CLASS),
        (r"[[\p{L}]--[aeiou]]", PCRE2_UTF | PCRE2_ALT_EXTENDED_CLASS),
        (r"[[a-z]||[\p{Nd}]]", PCRE2_UTF | PCRE2_ALT_EXTENDED_CLASS),
        (r"[[a-z]~~[aeiou]]", PCRE2_UTF | PCRE2_ALT_EXTENDED_CLASS),
        (
            r"[[[\p{L}]&&[\x{100}-\x{500}]]--[\x{200}]]",
            PCRE2_UTF | PCRE2_ALT_EXTENDED_CLASS,
        ),
        (r"[^[\p{L}]&&[a-z]]", PCRE2_UTF | PCRE2_ALT_EXTENDED_CLASS),
        (r"[[\p{Greek}]&&[\p{Lu}]]", PCRE2_UTF | PCRE2_ALT_EXTENDED_CLASS),
        (r"[[a-z]&&[b-y]]", PCRE2_ALT_EXTENDED_CLASS),
        (r"[[a-z]&&[b-y]]", PCRE2_ALT_EXTENDED_CLASS | PCRE2_CASELESS),
    ];

    // Code points: all of 0..=0x2FF, every boundary, and a deterministic sample
    // across the whole space.
    let mut cps: Vec<u32> = (0u32..=0x2FF).collect();
    cps.extend([
        0x300, 0x3A9, 0x400, 0x4FF, 0x500, 0x5D0, 0x627, 0x905, 0x1000, 0x1FFF, 0x2000, 0x2028,
        0x2029, 0x3042, 0x4E00, 0x9FFF, 0xAC00, 0xD7FF, 0xE000, 0xFFFD, 0xFFFF, 0x1_0000, 0x1F600,
        0x2_0000, 0x10_FFFE, 0x10_FFFF,
    ]);
    let mut rng = Rng::new(0x1234_0002);
    for _ in 0..3000 {
        cps.push(rng.next_u32() % 0x11_0000);
    }
    cps.retain(|&c| char::from_u32(c).is_some());

    for (pi, (p, copts)) in xclass_pats.iter().chain(eclass_pats.iter()).enumerate() {
        diff(&format!("xclass/eclass pat[{pi}]={p:?}"), |api| {
            let mut l = Log::new();
            unsafe {
                let code =
                    compile_logged(api, p.as_bytes(), p.len(), *copts, std::ptr::null_mut(), &mut l);
                if code.is_null() {
                    return l;
                }
                log_all_info(api, code, &mut l);
                let md = (api.match_data_create_from_pattern)(code, std::ptr::null_mut());
                for &cp in &cps {
                    let ch = char::from_u32(cp).unwrap();
                    let mut buf = [0u8; 4];
                    let s: &[u8] = if *copts & PCRE2_UTF != 0 {
                        ch.encode_utf8(&mut buf).as_bytes()
                    } else {
                        // non-UTF mode works on single bytes
                        if cp > 0xFF {
                            continue;
                        }
                        buf[0] = cp as u8;
                        &buf[..1]
                    };
                    let rc = (api.do_match)(
                        code,
                        s.as_ptr(),
                        s.len(),
                        0,
                        PCRE2_ANCHORED,
                        md,
                        std::ptr::null_mut(),
                    );
                    l.i(rc as i64);
                    if rc > 0 {
                        let ov = (api.get_ovector_pointer)(md);
                        l.u(*ov as u64).u(*ov.add(1) as u64);
                    }
                }
                (api.match_data_free)(md);
                (api.code_free)(code);
            }
            l
        });
    }
}

/// The same sweep through the DFA engine, which has its own XCLASS/ECLASS
/// handling path.
#[test]
fn xclass_and_eclass_sweep_dfa() {
    let pats: &[(&str, u32)] = &[
        (r"[\x{100}-\x{200}]", PCRE2_UTF),
        (r"[^\x{100}]", PCRE2_UTF),
        (r"[\p{L}]", PCRE2_UTF),
        (r"[^\p{L}]", PCRE2_UTF),
        (r"[[\p{L}]&&[a-z]]", PCRE2_UTF | PCRE2_ALT_EXTENDED_CLASS),
        (r"[[a-z]--[aeiou]]", PCRE2_UTF | PCRE2_ALT_EXTENDED_CLASS),
        (r"[\x80-\xff]", 0),
    ];
    let mut cps: Vec<u32> = (0u32..=0x2FF).collect();
    cps.extend([0x3A9, 0x4E00, 0x1F600, 0x10_FFFF, 0xFFFF, 0x1_0000]);
    cps.retain(|&c| char::from_u32(c).is_some());

    for (pi, (p, copts)) in pats.iter().enumerate() {
        diff(&format!("dfa xclass pat[{pi}]={p:?}"), |api| {
            let mut l = Log::new();
            unsafe {
                let code =
                    compile_logged(api, p.as_bytes(), p.len(), *copts, std::ptr::null_mut(), &mut l);
                if code.is_null() {
                    return l;
                }
                let md = (api.match_data_create_from_pattern)(code, std::ptr::null_mut());
                let mut ws: Vec<c_int> = vec![0; 1000];
                for &cp in &cps {
                    let ch = char::from_u32(cp).unwrap();
                    let mut buf = [0u8; 4];
                    let s: &[u8] = if *copts & PCRE2_UTF != 0 {
                        ch.encode_utf8(&mut buf).as_bytes()
                    } else {
                        if cp > 0xFF {
                            continue;
                        }
                        buf[0] = cp as u8;
                        &buf[..1]
                    };
                    let rc = (api.dfa_match)(
                        code,
                        s.as_ptr(),
                        s.len(),
                        0,
                        PCRE2_ANCHORED,
                        md,
                        std::ptr::null_mut(),
                        ws.as_mut_ptr(),
                        1000,
                    );
                    l.i(rc as i64);
                    if rc > 0 {
                        let ov = (api.get_ovector_pointer)(md);
                        l.u(*ov as u64).u(*ov.add(1) as u64);
                    }
                }
                (api.match_data_free)(md);
                (api.code_free)(code);
            }
            l
        });
    }
}

/// `_pcre2_auto_possessify_8` and `_pcre2_update_classbits_8` are only
/// reachable through `pcre2_compile`; the observable effect of auto-possessify
/// is the compiled size and the match behaviour, so compare both with the
/// optimization on and off across quantifier-heavy patterns.
#[test]
fn auto_possessify_effect() {
    let pats = [
        "a+b", "a*b", r"\d+[a-z]", "[a-z]+[0-9]", "a{2,5}b", r"\w+\W", r"\s*\S",
        r"[\p{L}]+[\p{N}]", "a++b", "(?:ab)+c", r"\D+\d", r"[^a]+a", ".+x", ".*?y",
        r"[\x{100}-\x{200}]+z", r"[[a-z]&&[b-y]]+q",
    ];
    for (pi, p) in pats.iter().enumerate() {
        for copts in [0u32, PCRE2_UTF, PCRE2_UCP, PCRE2_ALT_EXTENDED_CLASS, PCRE2_CASELESS] {
            for possess in [true, false] {
                diff(
                    &format!("autopossess pat[{pi}] copts={copts:#x} on={possess}"),
                    |api| {
                        let mut l = Log::new();
                        unsafe {
                            let cc = (api.compile_context_create)(std::ptr::null_mut());
                            l.i((api.set_optimize)(
                                cc,
                                if possess {
                                    PCRE2_AUTO_POSSESS
                                } else {
                                    PCRE2_AUTO_POSSESS_OFF
                                },
                            ) as i64);
                            let code =
                                compile_logged(api, p.as_bytes(), p.len(), copts, cc, &mut l);
                            if !code.is_null() {
                                log_all_info(api, code, &mut l);
                                log_serialized(api, code, &mut l);
                                let md = (api.match_data_create_from_pattern)(
                                    code,
                                    std::ptr::null_mut(),
                                );
                                for s in SUBJECTS {
                                    let rc = (api.do_match)(
                                        code,
                                        s.as_ptr(),
                                        s.len(),
                                        0,
                                        0,
                                        md,
                                        std::ptr::null_mut(),
                                    );
                                    log_match_result_full(api, code, md, rc, &mut l);
                                }
                                (api.match_data_free)(md);
                                (api.code_free)(code);
                            }
                            (api.compile_context_free)(cc);
                        }
                        l
                    },
                );
            }
        }
    }
}

/// `_pcre2_check_escape_8` is reached for every escape sequence during
/// parsing; sweep every `\<char>` for all 256 byte values under a range of
/// option settings and compare the compile result exactly.
#[test]
fn check_escape_every_byte() {
    for b in 0u32..=255 {
        let pat = vec![b'\\', b as u8];
        let in_class = vec![b'[', b'\\', b as u8, b']'];
        diff(&format!("check_escape {b:#04x}"), |api| {
            let mut l = Log::new();
            unsafe {
                for opts in [
                    0u32,
                    PCRE2_UTF,
                    PCRE2_UCP,
                    PCRE2_ALT_BSUX,
                    PCRE2_NEVER_BACKSLASH_C,
                    PCRE2_ALT_VERBNAMES,
                ] {
                    for extra in [
                        0u32,
                        PCRE2_EXTRA_ALT_BSUX,
                        PCRE2_EXTRA_BAD_ESCAPE_IS_LITERAL,
                        PCRE2_EXTRA_PYTHON_OCTAL,
                        PCRE2_EXTRA_NO_BS0,
                        PCRE2_EXTRA_ESCAPED_CR_IS_LF,
                        PCRE2_EXTRA_ALLOW_SURROGATE_ESCAPES,
                    ] {
                        let cc = (api.compile_context_create)(std::ptr::null_mut());
                        (api.set_compile_extra_options)(cc, extra);
                        for pp in [&pat, &in_class] {
                            let code = compile_logged(api, pp, pp.len(), opts, cc, &mut l);
                            if !code.is_null() {
                                log_all_info(api, code, &mut l);
                                log_serialized(api, code, &mut l);
                                (api.code_free)(code);
                            }
                        }
                        (api.compile_context_free)(cc);
                    }
                }
            }
            l
        });
    }
}
