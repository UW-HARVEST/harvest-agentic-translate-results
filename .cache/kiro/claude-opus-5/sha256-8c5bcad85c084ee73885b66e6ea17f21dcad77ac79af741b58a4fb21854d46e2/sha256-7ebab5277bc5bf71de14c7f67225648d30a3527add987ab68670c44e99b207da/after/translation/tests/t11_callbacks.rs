//! Callbacks that cross the FFI boundary: match callouts, substitute callouts,
//! substitute case callouts, the compile-time recursion guard, and custom
//! memory allocators (`_pcre2_memctl_malloc`).
mod common;

use common::*;
use std::ffi::c_void;
use std::sync::Mutex;

/* ---------------------------- match callouts ---------------------------- */

/// `pcre2_callout_block`, mirrored from pcre2.h (8-bit, version 3).
#[repr(C)]
struct CalloutBlock {
    version: u32,
    callout_number: u32,
    capture_top: u32,
    capture_last: u32,
    offset_vector: *const PCRE2_SIZE,
    mark: PCRE2_SPTR,
    subject: PCRE2_SPTR,
    subject_length: PCRE2_SIZE,
    start_match: PCRE2_SIZE,
    current_position: PCRE2_SIZE,
    pattern_position: PCRE2_SIZE,
    next_item_length: PCRE2_SIZE,
    callout_string_offset: PCRE2_SIZE,
    callout_string_length: PCRE2_SIZE,
    callout_string: PCRE2_SPTR,
    callout_flags: u32,
}

#[derive(Debug, PartialEq, Eq)]
struct CalloutRecord {
    version: u32,
    callout_number: u32,
    capture_top: u32,
    capture_last: u32,
    ovector: Vec<PCRE2_SIZE>,
    mark: Option<Vec<u8>>,
    subject_length: PCRE2_SIZE,
    start_match: PCRE2_SIZE,
    current_position: PCRE2_SIZE,
    pattern_position: PCRE2_SIZE,
    next_item_length: PCRE2_SIZE,
    callout_string_offset: PCRE2_SIZE,
    callout_string_length: PCRE2_SIZE,
    callout_string: Option<Vec<u8>>,
    callout_flags: u32,
}

static CALLOUT_LOG: Mutex<Vec<CalloutRecord>> = Mutex::new(Vec::new());
/// Value the callback returns; 0 continue, 1 fail, negative = error.
static CALLOUT_RET: Mutex<i32> = Mutex::new(0);
/// `pcre2_dfa_match` has no capture support: it reports `capture_top == 1` but
/// never writes the offset vector before invoking a callout, and for callouts
/// inside nested assertions the vector is an uninitialised local array. Reading
/// it would compare uninitialised memory, so skip it for DFA matching.
static RECORD_OVECTOR: Mutex<bool> = Mutex::new(true);

/// These tests share module-level logs, so they must not run concurrently.
static SERIAL: Mutex<()> = Mutex::new(());

fn serial() -> std::sync::MutexGuard<'static, ()> {
    SERIAL.lock().unwrap_or_else(|e| e.into_inner())
}

unsafe fn cstr(p: PCRE2_SPTR) -> Option<Vec<u8>> {
    unsafe {
        if p.is_null() {
            return None;
        }
        let mut v = Vec::new();
        let mut q = p;
        while *q != 0 {
            v.push(*q);
            q = q.add(1);
        }
        Some(v)
    }
}

unsafe extern "C" fn callout_cb(blk: *mut CalloutBlock, _data: *mut c_void) -> i32 {
    unsafe {
        let b = &*blk;
        // capture_top pairs are meaningful in the offset vector.
        let n = (b.capture_top as usize) * 2;
        let ovector: Vec<PCRE2_SIZE> = if *RECORD_OVECTOR.lock().unwrap() {
            (0..n).map(|i| *b.offset_vector.add(i)).collect()
        } else {
            Vec::new()
        };
        CALLOUT_LOG.lock().unwrap().push(CalloutRecord {
            version: b.version,
            callout_number: b.callout_number,
            capture_top: b.capture_top,
            capture_last: b.capture_last,
            ovector,
            mark: cstr(b.mark),
            subject_length: b.subject_length,
            start_match: b.start_match,
            current_position: b.current_position,
            pattern_position: b.pattern_position,
            next_item_length: b.next_item_length,
            callout_string_offset: b.callout_string_offset,
            callout_string_length: b.callout_string_length,
            callout_string: if b.callout_string.is_null() {
                None
            } else {
                Some(slice_at(b.callout_string, b.callout_string_length).to_vec())
            },
            callout_flags: b.callout_flags,
        });
        *CALLOUT_RET.lock().unwrap()
    }
}

/// Returns 1 (fail this match attempt) after the third callout.
unsafe extern "C" fn callout_cb_counted(blk: *mut CalloutBlock, d: *mut c_void) -> i32 {
    unsafe {
        callout_cb(blk, d);
        if CALLOUT_LOG.lock().unwrap().len() >= 3 { 1 } else { 0 }
    }
}

type SetCallout = unsafe extern "C" fn(
    *mut c_void,
    unsafe extern "C" fn(*mut CalloutBlock, *mut c_void) -> i32,
    *mut c_void,
) -> i32;
type CtxCreate = unsafe extern "C" fn(*mut c_void) -> *mut c_void;
type CtxFree = unsafe extern "C" fn(*mut c_void);
type MdCreate = unsafe extern "C" fn(u32, *mut c_void) -> *mut c_void;
type MdFree = unsafe extern "C" fn(*mut c_void);
type Match = unsafe extern "C" fn(
    *const c_void,
    PCRE2_SPTR,
    PCRE2_SIZE,
    PCRE2_SIZE,
    u32,
    *mut c_void,
    *mut c_void,
) -> i32;
type DfaMatch = unsafe extern "C" fn(
    *const c_void,
    PCRE2_SPTR,
    PCRE2_SIZE,
    PCRE2_SIZE,
    u32,
    *mut c_void,
    *mut c_void,
    *mut std::ffi::c_int,
    PCRE2_SIZE,
) -> i32;

const CALLOUT_PATTERNS: &[&[u8]] = &[
    b"(?C1)a(?C2)b(?C3)c",
    b"a(?C)b",
    b"(?C{start})x(?C{end})",
    b"(a)(?C1)(b)(?C2)",
    b"(?C1)\\d+(?C2)",
    b"(?C1)a*(?C2)b",
    b"(?<n>(?C1)a)(?C2)",
    b"(?C255)abc",
    b"abc",
    b"a(?C1)|b(?C2)",
    b"(?C1)(?=a)(?C2)a",
    b"(*MARK:m)(?C1)a",
];

const CALLOUT_SUBJECTS: &[&[u8]] = &[b"", b"a", b"ab", b"abc", b"xy", b"123", b"aaab", b"b"];

#[test]
fn match_callouts_match() {
    let _serial = serial();
    let (mc, mr) = both::<Match>("pcre2_match_8");
    let (cc, rc) = both::<CtxCreate>("pcre2_match_context_create_8");
    let (cf, rf) = both::<CtxFree>("pcre2_match_context_free_8");
    let (sc, sr) = both::<SetCallout>("pcre2_set_callout_8");
    let (mdc, mdr) = both::<MdCreate>("pcre2_match_data_create_8");
    let (mdfc, mdfr) = both::<MdFree>("pcre2_match_data_free_8");
    unsafe {
        let ctx_c = cc(std::ptr::null_mut());
        let ctx_r = rc(std::ptr::null_mut());
        assert_eq!(sc(ctx_c, callout_cb, std::ptr::null_mut()), sr(ctx_r, callout_cb, std::ptr::null_mut()));
        let md_c = mdc(16, std::ptr::null_mut());
        let md_r = mdr(16, std::ptr::null_mut());

        for pat in CALLOUT_PATTERNS {
            for &copts in &[0u32, PCRE2_AUTO_CALLOUT, PCRE2_UTF, PCRE2_CASELESS] {
                let Some(pair) = compile_both(pat, copts) else {
                    continue;
                };
                for subj in CALLOUT_SUBJECTS {
                    for ret in [0i32, 1, -1, -37] {
                        *CALLOUT_RET.lock().unwrap() = ret;
                        let label = format!(
                            "callout pat={pat:02x?} subj={subj:02x?} copts={copts:#x} ret={ret}"
                        );

                        CALLOUT_LOG.lock().unwrap().clear();
                        md_poison(md_c, 16);
                        let x = mc(pair.c, subj.as_ptr(), subj.len(), 0, 0, md_c, ctx_c);
                        let log_c = std::mem::take(&mut *CALLOUT_LOG.lock().unwrap());
                        md_poison(md_r, 16);
                        let y = mr(pair.r, subj.as_ptr(), subj.len(), 0, 0, md_r, ctx_r);
                        let log_r = std::mem::take(&mut *CALLOUT_LOG.lock().unwrap());

                        assert_eq!(x, y, "{label}: rc");
                        assert_eq!(log_c.len(), log_r.len(), "{label}: callout count");
                        for (i, (a, b)) in log_c.iter().zip(log_r.iter()).enumerate() {
                            assert_eq!(a, b, "{label}: callout block {i}");
                        }
                    }
                }
            }
        }

        // A callback that aborts after a few calls.
        *CALLOUT_RET.lock().unwrap() = 0;
        assert_eq!(
            sc(ctx_c, callout_cb_counted, std::ptr::null_mut()),
            sr(ctx_r, callout_cb_counted, std::ptr::null_mut())
        );
        for pat in CALLOUT_PATTERNS {
            let Some(pair) = compile_both(pat, PCRE2_AUTO_CALLOUT) else {
                continue;
            };
            for subj in CALLOUT_SUBJECTS {
                CALLOUT_LOG.lock().unwrap().clear();
                md_poison(md_c, 16);
                let x = mc(pair.c, subj.as_ptr(), subj.len(), 0, 0, md_c, ctx_c);
                let log_c = std::mem::take(&mut *CALLOUT_LOG.lock().unwrap());
                md_poison(md_r, 16);
                let y = mr(pair.r, subj.as_ptr(), subj.len(), 0, 0, md_r, ctx_r);
                let log_r = std::mem::take(&mut *CALLOUT_LOG.lock().unwrap());
                assert_eq!(x, y, "counted callout rc {pat:02x?}/{subj:02x?}");
                assert_eq!(log_c, log_r, "counted callout blocks {pat:02x?}/{subj:02x?}");
            }
        }

        mdfc(md_c);
        mdfr(md_r);
        cf(ctx_c);
        rf(ctx_r);
    }
}

#[test]
fn dfa_callouts_match() {
    let _serial = serial();
    let (dc, dr) = both::<DfaMatch>("pcre2_dfa_match_8");
    let (cc, rc) = both::<CtxCreate>("pcre2_match_context_create_8");
    let (cf, rf) = both::<CtxFree>("pcre2_match_context_free_8");
    let (sc, sr) = both::<SetCallout>("pcre2_set_callout_8");
    let (mdc, mdr) = both::<MdCreate>("pcre2_match_data_create_8");
    let (mdfc, mdfr) = both::<MdFree>("pcre2_match_data_free_8");
    unsafe {
        let ctx_c = cc(std::ptr::null_mut());
        let ctx_r = rc(std::ptr::null_mut());
        sc(ctx_c, callout_cb, std::ptr::null_mut());
        sr(ctx_r, callout_cb, std::ptr::null_mut());
        *RECORD_OVECTOR.lock().unwrap() = false;
        let md_c = mdc(16, std::ptr::null_mut());
        let md_r = mdr(16, std::ptr::null_mut());
        for pat in CALLOUT_PATTERNS {
            for &copts in &[0u32, PCRE2_AUTO_CALLOUT] {
                let Some(pair) = compile_both(pat, copts) else {
                    continue;
                };
                for subj in CALLOUT_SUBJECTS {
                    for ret in [0i32, 1, -1] {
                        *CALLOUT_RET.lock().unwrap() = ret;
                        let mut ws_c = vec![0x5A5A_5A5Ai32; 500];
                        let mut ws_r = vec![0x5A5A_5A5Ai32; 500];
                        CALLOUT_LOG.lock().unwrap().clear();
                        md_poison(md_c, 16);
                        let x = dc(
                            pair.c, subj.as_ptr(), subj.len(), 0, 0, md_c, ctx_c,
                            ws_c.as_mut_ptr(), 500,
                        );
                        let log_c = std::mem::take(&mut *CALLOUT_LOG.lock().unwrap());
                        md_poison(md_r, 16);
                        let y = dr(
                            pair.r, subj.as_ptr(), subj.len(), 0, 0, md_r, ctx_r,
                            ws_r.as_mut_ptr(), 500,
                        );
                        let log_r = std::mem::take(&mut *CALLOUT_LOG.lock().unwrap());
                        let label = format!(
                            "dfa callout pat={pat:02x?} subj={subj:02x?} copts={copts:#x} ret={ret}"
                        );
                        assert_eq!(x, y, "{label}: rc");
                        assert_eq!(log_c, log_r, "{label}: callout blocks");
                        assert_eq!(ws_c, ws_r, "{label}: workspace");
                    }
                }
            }
        }
        *RECORD_OVECTOR.lock().unwrap() = true;
        mdfc(md_c);
        mdfr(md_r);
        cf(ctx_c);
        rf(ctx_r);
    }
}

/* -------------------------- substitute callouts -------------------------- */

/// pcre2.h orders the fields: version, input, output, output_offsets[2],
/// ovector, oveccount, subscount.
#[repr(C)]
struct SubBlock {
    version: u32,
    input: PCRE2_SPTR,
    output: PCRE2_SPTR,
    output_offsets: [PCRE2_SIZE; 2],
    ovector: *const PCRE2_SIZE,
    oveccount: u32,
    subscount: u32,
}

#[derive(Debug, PartialEq, Eq)]
struct SubRecord {
    version: u32,
    output_offsets: [PCRE2_SIZE; 2],
    oveccount: u32,
    subscount: u32,
    ovector: Vec<PCRE2_SIZE>,
    output_slice: Vec<u8>,
}

static SUB_LOG: Mutex<Vec<SubRecord>> = Mutex::new(Vec::new());
static SUB_RET: Mutex<i32> = Mutex::new(0);

unsafe extern "C" fn sub_cb(blk: *mut SubBlock, _data: *mut c_void) -> i32 {
    unsafe {
        let b = &*blk;
        let n = (b.oveccount as usize) * 2;
        SUB_LOG.lock().unwrap().push(SubRecord {
            version: b.version,
            output_offsets: b.output_offsets,
            oveccount: b.oveccount,
            subscount: b.subscount,
            ovector: (0..n).map(|i| *b.ovector.add(i)).collect(),
            output_slice: slice_at(b.output.add(b.output_offsets[0]), b.output_offsets[1] - b.output_offsets[0])
                .to_vec(),
        });
        *SUB_RET.lock().unwrap()
    }
}

type SetSubCallout = unsafe extern "C" fn(
    *mut c_void,
    unsafe extern "C" fn(*mut SubBlock, *mut c_void) -> i32,
    *mut c_void,
) -> i32;
type Substitute = unsafe extern "C" fn(
    *const c_void,
    PCRE2_SPTR,
    PCRE2_SIZE,
    PCRE2_SIZE,
    u32,
    *mut c_void,
    *mut c_void,
    PCRE2_SPTR,
    PCRE2_SIZE,
    *mut PCRE2_UCHAR,
    *mut PCRE2_SIZE,
) -> i32;

#[test]
fn substitute_callouts_match() {
    let _serial = serial();
    let (sc, sr) = both::<Substitute>("pcre2_substitute_8");
    let (cc, rc) = both::<CtxCreate>("pcre2_match_context_create_8");
    let (cf, rf) = both::<CtxFree>("pcre2_match_context_free_8");
    let (ssc, ssr) = both::<SetSubCallout>("pcre2_set_substitute_callout_8");
    unsafe {
        let ctx_c = cc(std::ptr::null_mut());
        let ctx_r = rc(std::ptr::null_mut());
        assert_eq!(
            ssc(ctx_c, sub_cb, std::ptr::null_mut()),
            ssr(ctx_r, sub_cb, std::ptr::null_mut())
        );
        let pats: &[&[u8]] = &[b"a", b"(a)(b)?", b"\\d+", b"[aeiou]", b"", b"a*"];
        let subs: &[&[u8]] = &[b"", b"a", b"abcabc", b"aeiou", b"a1b22", b"aaa"];
        let reps: &[&[u8]] = &[b"", b"X", b"[$0]", b"$1$2", b"<$0>"];
        for pat in pats {
            let Some(pair) = compile_both(pat, 0) else {
                continue;
            };
            for subj in subs {
                for rep in reps {
                    for &opts in &[
                        0u32,
                        PCRE2_SUBSTITUTE_GLOBAL,
                        PCRE2_SUBSTITUTE_GLOBAL | PCRE2_SUBSTITUTE_EXTENDED,
                        PCRE2_SUBSTITUTE_REPLACEMENT_ONLY | PCRE2_SUBSTITUTE_GLOBAL,
                    ] {
                        for ret in [0i32, 1, -1] {
                            *SUB_RET.lock().unwrap() = ret;
                            let mut bc = vec![0xAAu8; 256];
                            let mut br = vec![0xAAu8; 256];
                            let mut la = 128usize;
                            let mut lb = 128usize;
                            SUB_LOG.lock().unwrap().clear();
                            let x = sc(
                                pair.c, subj.as_ptr(), subj.len(), 0, opts,
                                std::ptr::null_mut(), ctx_c, rep.as_ptr(), rep.len(),
                                bc.as_mut_ptr(), &mut la,
                            );
                            let log_c = std::mem::take(&mut *SUB_LOG.lock().unwrap());
                            let y = sr(
                                pair.r, subj.as_ptr(), subj.len(), 0, opts,
                                std::ptr::null_mut(), ctx_r, rep.as_ptr(), rep.len(),
                                br.as_mut_ptr(), &mut lb,
                            );
                            let log_r = std::mem::take(&mut *SUB_LOG.lock().unwrap());
                            let label = format!(
                                "sub callout pat={pat:02x?} subj={subj:02x?} rep={rep:02x?} opts={opts:#x} ret={ret}"
                            );
                            assert_eq!(x, y, "{label}: rc");
                            assert_eq!(la, lb, "{label}: length");
                            assert_bytes_eq(&format!("{label}: buffer"), &bc, &br);
                            assert_eq!(log_c, log_r, "{label}: callout blocks");
                        }
                    }
                }
            }
        }
        cf(ctx_c);
        rf(ctx_r);
    }
}

/* ----------------------- substitute case callout ------------------------ */

static CASE_LOG: Mutex<Vec<(Vec<u8>, PCRE2_SIZE, i32)>> = Mutex::new(Vec::new());

/// Upper/lower-cases ASCII in a deterministic way so both libraries see the
/// same transformation.
unsafe extern "C" fn case_cb(
    input: PCRE2_SPTR,
    inlen: PCRE2_SIZE,
    output: *mut PCRE2_UCHAR,
    outlen: PCRE2_SIZE,
    to_case: std::ffi::c_int,
    _data: *mut c_void,
) -> PCRE2_SIZE {
    unsafe {
        CASE_LOG
            .lock()
            .unwrap()
            .push((slice_at(input, inlen).to_vec(), outlen, to_case));
        if inlen > outlen {
            return inlen + 1; /* signal "needs more room" */
        }
        for i in 0..inlen {
            let b = *input.add(i);
            let o = match to_case {
                0 => b.to_ascii_lowercase(),
                1 => b.to_ascii_uppercase(),
                _ => b,
            };
            *output.add(i) = o;
        }
        inlen
    }
}

type SetCaseCallout = unsafe extern "C" fn(
    *mut c_void,
    unsafe extern "C" fn(
        PCRE2_SPTR,
        PCRE2_SIZE,
        *mut PCRE2_UCHAR,
        PCRE2_SIZE,
        std::ffi::c_int,
        *mut c_void,
    ) -> PCRE2_SIZE,
    *mut c_void,
) -> i32;

#[test]
fn substitute_case_callout_matches() {
    let _serial = serial();
    let (sc, sr) = both::<Substitute>("pcre2_substitute_8");
    let (cc, rc) = both::<CtxCreate>("pcre2_match_context_create_8");
    let (cf, rf) = both::<CtxFree>("pcre2_match_context_free_8");
    let (scc, scr) = both::<SetCaseCallout>("pcre2_set_substitute_case_callout_8");
    unsafe {
        let ctx_c = cc(std::ptr::null_mut());
        let ctx_r = rc(std::ptr::null_mut());
        assert_eq!(
            scc(ctx_c, case_cb, std::ptr::null_mut()),
            scr(ctx_r, case_cb, std::ptr::null_mut())
        );
        let pats: &[&[u8]] = &[b"(\\w+)", b"a", b"(.)(.)"];
        let subs: &[&[u8]] = &[b"hello World", b"aB", b"xy", b""];
        let reps: &[&[u8]] = &[
            b"\\U$0\\E",
            b"\\L$0\\E",
            b"\\u$1",
            b"\\l$1",
            b"\\U$1\\E-\\L$2\\E",
            b"\\U\\u$0",
        ];
        for pat in pats {
            let Some(pair) = compile_both(pat, 0) else {
                continue;
            };
            for subj in subs {
                for rep in reps {
                    for &opts in &[
                        PCRE2_SUBSTITUTE_EXTENDED,
                        PCRE2_SUBSTITUTE_EXTENDED | PCRE2_SUBSTITUTE_GLOBAL,
                    ] {
                        for outsz in [1usize, 4, 128] {
                            let mut bc = vec![0xAAu8; 256];
                            let mut br = vec![0xAAu8; 256];
                            let mut la = outsz;
                            let mut lb = outsz;
                            CASE_LOG.lock().unwrap().clear();
                            let x = sc(
                                pair.c, subj.as_ptr(), subj.len(), 0, opts,
                                std::ptr::null_mut(), ctx_c, rep.as_ptr(), rep.len(),
                                bc.as_mut_ptr(), &mut la,
                            );
                            let log_c = std::mem::take(&mut *CASE_LOG.lock().unwrap());
                            let y = sr(
                                pair.r, subj.as_ptr(), subj.len(), 0, opts,
                                std::ptr::null_mut(), ctx_r, rep.as_ptr(), rep.len(),
                                br.as_mut_ptr(), &mut lb,
                            );
                            let log_r = std::mem::take(&mut *CASE_LOG.lock().unwrap());
                            let label = format!(
                                "case callout pat={pat:02x?} subj={subj:02x?} rep={rep:02x?} opts={opts:#x} outsz={outsz}"
                            );
                            assert_eq!(x, y, "{label}: rc");
                            assert_eq!(la, lb, "{label}: length");
                            assert_bytes_eq(&format!("{label}: buffer"), &bc, &br);
                            assert_eq!(log_c, log_r, "{label}: case callout calls");
                        }
                    }
                }
            }
        }
        cf(ctx_c);
        rf(ctx_r);
    }
}

/* ------------------------ compile recursion guard ------------------------ */

static GUARD_LOG: Mutex<Vec<u32>> = Mutex::new(Vec::new());
static GUARD_LIMIT: Mutex<u32> = Mutex::new(u32::MAX);

unsafe extern "C" fn guard_cb(depth: u32, _data: *mut c_void) -> i32 {
    GUARD_LOG.lock().unwrap().push(depth);
    if depth > *GUARD_LIMIT.lock().unwrap() { 1 } else { 0 }
}

type SetGuard = unsafe extern "C" fn(
    *mut c_void,
    unsafe extern "C" fn(u32, *mut c_void) -> i32,
    *mut c_void,
) -> i32;

#[test]
fn compile_recursion_guard_matches() {
    let _serial = serial();
    let (cc, rc) = both::<CtxCreate>("pcre2_compile_context_create_8");
    let (cf, rf) = both::<CtxFree>("pcre2_compile_context_free_8");
    let (sgc, sgr) = both::<SetGuard>("pcre2_set_compile_recursion_guard_8");
    unsafe {
        let ctx_c = cc(std::ptr::null_mut());
        let ctx_r = rc(std::ptr::null_mut());
        assert_eq!(
            sgc(ctx_c, guard_cb, std::ptr::null_mut()),
            sgr(ctx_r, guard_cb, std::ptr::null_mut())
        );
        for limit in [u32::MAX, 0, 1, 2, 3, 5] {
            *GUARD_LIMIT.lock().unwrap() = limit;
            for p in patterns() {
                GUARD_LOG.lock().unwrap().clear();
                // compile_both_ctx asserts that error codes, offsets and the whole
                // compiled block agree; here we additionally require that the guard
                // was invoked with exactly the same depth sequence. The log holds
                // the C library's calls followed by the Rust library's calls.
                let one = compile_both_ctx(p, 0, ctx_c, ctx_r);
                let log = std::mem::take(&mut *GUARD_LOG.lock().unwrap());
                assert_eq!(
                    log.len() % 2,
                    0,
                    "odd guard call count for {p:02x?} limit={limit}"
                );
                let half = log.len() / 2;
                assert_eq!(
                    &log[..half],
                    &log[half..],
                    "guard depth sequence differs for {p:02x?} limit={limit}"
                );
                drop(one);
            }
        }
        cf(ctx_c);
        rf(ctx_r);
    }
}

/* --------------------------- custom allocators --------------------------- */

static ALLOC_LOG: Mutex<Vec<(usize, usize)>> = Mutex::new(Vec::new());
static FREE_COUNT: Mutex<usize> = Mutex::new(0);

unsafe extern "C" fn my_malloc(size: usize, data: *mut c_void) -> *mut c_void {
    ALLOC_LOG.lock().unwrap().push((size, data as usize));
    unsafe { libc_malloc(size) }
}

unsafe extern "C" fn my_free(p: *mut c_void, _data: *mut c_void) {
    *FREE_COUNT.lock().unwrap() += 1;
    unsafe { libc_free(p) }
}

/// Fails after N allocations, to exercise out-of-memory paths.
static ALLOC_BUDGET: Mutex<i64> = Mutex::new(i64::MAX);

unsafe extern "C" fn budget_malloc(size: usize, data: *mut c_void) -> *mut c_void {
    let mut b = ALLOC_BUDGET.lock().unwrap();
    if *b <= 0 {
        return std::ptr::null_mut();
    }
    *b -= 1;
    drop(b);
    ALLOC_LOG.lock().unwrap().push((size, data as usize));
    unsafe { libc_malloc(size) }
}

unsafe extern "C" {
    #[link_name = "malloc"]
    fn libc_malloc(n: usize) -> *mut c_void;
    #[link_name = "free"]
    fn libc_free(p: *mut c_void);
}

type GCtxCreate = unsafe extern "C" fn(
    unsafe extern "C" fn(usize, *mut c_void) -> *mut c_void,
    unsafe extern "C" fn(*mut c_void, *mut c_void),
    *mut c_void,
) -> *mut c_void;
type GCtxFree = unsafe extern "C" fn(*mut c_void);
type MemctlMalloc = unsafe extern "C" fn(usize, *mut c_void) -> *mut c_void;

#[test]
fn memctl_malloc_matches() {
    let _serial = serial();
    let (mc, mr) = both::<MemctlMalloc>("_pcre2_memctl_malloc_8");
    let (gc, gr) = both::<GCtxCreate>("pcre2_general_context_create_8");
    let (gfc, gfr) = both::<GCtxFree>("pcre2_general_context_free_8");
    unsafe {
        // With a NULL memctl the default malloc/free are installed in the block.
        for size in [24usize, 32, 64, 1024] {
            let a = mc(size, std::ptr::null_mut());
            let b = mr(size, std::ptr::null_mut());
            assert!(!a.is_null() && !b.is_null(), "memctl_malloc({size})");
            // memory_data (third word) must be NULL in both.
            assert_eq!(
                std::ptr::read_unaligned((a as *const u8).add(16) as *const usize),
                0
            );
            assert_eq!(
                std::ptr::read_unaligned((b as *const u8).add(16) as *const usize),
                0
            );
            libc_free(a);
            libc_free(b);
        }

        // With a supplied memctl the block must be an exact copy of it.
        let marker = 0x1234usize as *mut c_void;
        let ga = gc(my_malloc, my_free, marker);
        let gb = gr(my_malloc, my_free, marker);
        for size in [24usize, 40, 128] {
            ALLOC_LOG.lock().unwrap().clear();
            let a = mc(size, ga);
            let log_a = std::mem::take(&mut *ALLOC_LOG.lock().unwrap());
            let b = mr(size, gb);
            let log_b = std::mem::take(&mut *ALLOC_LOG.lock().unwrap());
            assert_eq!(log_a, log_b, "memctl_malloc({size}) allocator calls");
            assert_bytes_eq(
                &format!("memctl_malloc({size}) copied memctl"),
                slice_at(a as *const u8, 24),
                slice_at(b as *const u8, 24),
            );
            libc_free(a);
            libc_free(b);
        }
        gfc(ga);
        gfr(gb);
    }
}

#[test]
fn custom_allocator_compile_and_match() {
    let _serial = serial();
    let (gc, gr) = both::<GCtxCreate>("pcre2_general_context_create_8");
    let (gfc, gfr) = both::<GCtxFree>("pcre2_general_context_free_8");
    let (ccc, ccr) = both::<unsafe extern "C" fn(*mut c_void) -> *mut c_void>(
        "pcre2_compile_context_create_8",
    );
    let (ccf, crf) = both::<CtxFree>("pcre2_compile_context_free_8");
    unsafe {
        let ga = gc(my_malloc, my_free, std::ptr::null_mut());
        let gb = gr(my_malloc, my_free, std::ptr::null_mut());
        let ctx_c = ccc(ga);
        let ctx_r = ccr(gb);
        for p in patterns() {
            ALLOC_LOG.lock().unwrap().clear();
            *FREE_COUNT.lock().unwrap() = 0;
            let a = compile_both_ctx(p, 0, ctx_c, ctx_r);
            // The allocation sizes must be identical: the log holds the C calls
            // followed by the Rust calls, so both halves must agree.
            let log = std::mem::take(&mut *ALLOC_LOG.lock().unwrap());
            assert_eq!(log.len() % 2, 0, "odd allocation count for {p:02x?}");
            let half = log.len() / 2;
            assert_eq!(
                &log[..half],
                &log[half..],
                "allocation sizes differ for {p:02x?}"
            );
            drop(a);
        }
        ccf(ctx_c);
        crf(ctx_r);
        gfc(ga);
        gfr(gb);
    }
}

#[test]
fn allocation_failure_paths_match() {
    let _serial = serial();
    let (gc, gr) = both::<GCtxCreate>("pcre2_general_context_create_8");
    let (gfc, gfr) = both::<GCtxFree>("pcre2_general_context_free_8");
    let (ccc, ccr) = both::<unsafe extern "C" fn(*mut c_void) -> *mut c_void>(
        "pcre2_compile_context_create_8",
    );
    let (ccf, crf) = both::<CtxFree>("pcre2_compile_context_free_8");
    unsafe {
        let ga = gc(budget_malloc, my_free, std::ptr::null_mut());
        let gb = gr(budget_malloc, my_free, std::ptr::null_mut());
        let ctx_c = ccc(ga);
        let ctx_r = ccr(gb);
        assert!(!ctx_c.is_null() && !ctx_r.is_null());
        for budget in 0i64..6 {
            for p in patterns() {
                *ALLOC_BUDGET.lock().unwrap() = budget;
                let mut ec = -999;
                let mut eo = usize::MAX;
                let (cc, _) = both::<CompileFn>("pcre2_compile_8");
                let a = cc(p.as_ptr(), p.len(), 0, &mut ec, &mut eo, ctx_c);
                *ALLOC_BUDGET.lock().unwrap() = budget;
                let mut ec2 = -999;
                let mut eo2 = usize::MAX;
                let (_, rcf) = both::<CompileFn>("pcre2_compile_8");
                let b = rcf(p.as_ptr(), p.len(), 0, &mut ec2, &mut eo2, ctx_r);
                *ALLOC_BUDGET.lock().unwrap() = i64::MAX;
                assert_eq!(ec, ec2, "budget={budget} errorcode for {p:02x?}");
                assert_eq!(eo, eo2, "budget={budget} erroroffset for {p:02x?}");
                assert_eq!(a.is_null(), b.is_null(), "budget={budget} for {p:02x?}");
                let (cf, rf) = both::<CtxFree>("pcre2_code_free_8");
                if !a.is_null() {
                    cf(a);
                }
                if !b.is_null() {
                    rf(b);
                }
            }
        }
        *ALLOC_BUDGET.lock().unwrap() = i64::MAX;
        ccf(ctx_c);
        crf(ctx_r);
        gfc(ga);
        gfr(gb);
    }
}
