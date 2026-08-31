//! `pcre2_match.c` (and `pcre2_match_data.c`, `pcre2_extuni.c`,
//! `pcre2_xclass.c`, `pcre2_script_run.c` via the interpreter).
mod common;

use common::*;
use std::ffi::c_void;

type MdCreate = unsafe extern "C" fn(u32, *mut c_void) -> *mut c_void;
type MdCreateFromPat = unsafe extern "C" fn(*const c_void, *mut c_void) -> *mut c_void;
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
type GetSizeFn = unsafe extern "C" fn(*mut c_void) -> PCRE2_SIZE;
type GetU32Fn = unsafe extern "C" fn(*mut c_void) -> u32;
type GetPtrFn = unsafe extern "C" fn(*mut c_void) -> *const u8;
type GetOvecFn = unsafe extern "C" fn(*mut c_void) -> *mut PCRE2_SIZE;
type CtxCreate = unsafe extern "C" fn(*mut c_void) -> *mut c_void;
type CtxFree = unsafe extern "C" fn(*mut c_void);
type SetU32 = unsafe extern "C" fn(*mut c_void, u32) -> i32;
type SetSize = unsafe extern "C" fn(*mut c_void, PCRE2_SIZE) -> i32;

/// The match-time option sets that are legal for `pcre2_match`.
const MATCH_OPTS: &[u32] = &[
    0,
    PCRE2_NOTBOL,
    PCRE2_NOTEOL,
    PCRE2_NOTBOL | PCRE2_NOTEOL,
    PCRE2_NOTEMPTY,
    PCRE2_NOTEMPTY_ATSTART,
    PCRE2_ANCHORED,
    PCRE2_ENDANCHORED,
    PCRE2_ANCHORED | PCRE2_ENDANCHORED,
    PCRE2_PARTIAL_SOFT,
    PCRE2_PARTIAL_HARD,
    PCRE2_NO_UTF_CHECK,
    PCRE2_NO_JIT,
    PCRE2_DISABLE_RECURSELOOP_CHECK,
];

struct MdPair {
    c: *mut c_void,
    r: *mut c_void,
    free_c: libloading::Symbol<'static, MdFree>,
    free_r: libloading::Symbol<'static, MdFree>,
}

impl Drop for MdPair {
    fn drop(&mut self) {
        unsafe {
            (self.free_c)(self.c);
            (self.free_r)(self.r);
        }
    }
}

fn md_pair(ovecsize: u32) -> MdPair {
    let (cc, rc) = both::<MdCreate>("pcre2_match_data_create_8");
    let (cf, rf) = both::<MdFree>("pcre2_match_data_free_8");
    unsafe {
        MdPair {
            c: cc(ovecsize, std::ptr::null_mut()),
            r: rc(ovecsize, std::ptr::null_mut()),
            free_c: cf,
            free_r: rf,
        }
    }
}

impl MdPair {
    /// Overwrite the volatile parts of both match datas so that fields neither
    /// library writes still compare equal.
    unsafe fn poison(&self) {
        unsafe {
            let (gcc, gcr) = both::<GetU32Fn>("pcre2_get_ovector_count_8");
            md_poison(self.c, gcc(self.c) as u16);
            md_poison(self.r, gcr(self.r) as u16);
        }
    }
}

/// Compare all the observable state of a match: return code, ovector, mark,
/// startchar and the internal bookkeeping fields.
unsafe fn assert_match_equal(
    label: &str,
    rc_c: i32,
    rc_r: i32,
    md_c: *mut c_void,
    md_r: *mut c_void,
    subject: *const u8,
    subject_len: usize,
) {
    unsafe {
        assert_eq!(rc_c, rc_r, "{label}: return code");
        let sc = md_snapshot(md_c, subject, subject_len);
        let sr = md_snapshot(md_r, subject, subject_len);
        assert_eq!(sc, sr, "{label}: match data");

        // Also go through the public accessors, which are separate exports.
        let (gsc, gsr) = both::<GetSizeFn>("pcre2_get_startchar_8");
        assert_eq!(gsc(md_c), gsr(md_r), "{label}: get_startchar");
        let (gcc, gcr) = both::<GetU32Fn>("pcre2_get_ovector_count_8");
        assert_eq!(gcc(md_c), gcr(md_r), "{label}: get_ovector_count");
        let (gmc, gmr) = both::<GetPtrFn>("pcre2_get_mark_8");
        // The mark field may still hold the poison pattern after an early error
        // return, so compare the raw words before dereferencing.
        let mc = gmc(md_c) as usize;
        let mr = gmr(md_r) as usize;
        if mc == MD_POISON_WORD || mr == MD_POISON_WORD || mc == 0 || mr == 0 {
            assert_eq!(
                mc == MD_POISON_WORD,
                mr == MD_POISON_WORD,
                "{label}: get_mark untouched-ness"
            );
            assert_eq!(mc == 0, mr == 0, "{label}: get_mark nullness");
        } else {
            let a = std::ffi::CStr::from_ptr(mc as *const std::ffi::c_char);
            let b = std::ffi::CStr::from_ptr(mr as *const std::ffi::c_char);
            assert_eq!(a, b, "{label}: get_mark");
        }
        let (goc, gor) = both::<GetOvecFn>("pcre2_get_ovector_pointer_8");
        let n = gcc(md_c) as usize * 2;
        let ov_c = std::slice::from_raw_parts(goc(md_c), n);
        let ov_r = std::slice::from_raw_parts(gor(md_r), n);
        assert_eq!(ov_c, ov_r, "{label}: ovector via accessor");
        let (gdc, gdr) = both::<GetSizeFn>("pcre2_get_match_data_size_8");
        assert_eq!(gdc(md_c), gdr(md_r), "{label}: get_match_data_size");
        let (ghc, ghr) = both::<GetSizeFn>("pcre2_get_match_data_heapframes_size_8");
        assert_eq!(
            ghc(md_c),
            ghr(md_r),
            "{label}: get_match_data_heapframes_size"
        );
    }
}

#[test]
fn match_data_create_matches() {
    let (cc, rc) = both::<MdCreate>("pcre2_match_data_create_8");
    let (cf, rf) = both::<MdFree>("pcre2_match_data_free_8");
    let (gcc, gcr) = both::<GetU32Fn>("pcre2_get_ovector_count_8");
    let (gdc, gdr) = both::<GetSizeFn>("pcre2_get_match_data_size_8");
    unsafe {
        for n in [0u32, 1, 2, 3, 10, 100, 65535] {
            let a = cc(n, std::ptr::null_mut());
            let b = rc(n, std::ptr::null_mut());
            assert!(!a.is_null() && !b.is_null(), "match_data_create({n})");
            assert_eq!(gcc(a), gcr(b), "ovector count for {n}");
            assert_eq!(gdc(a), gdr(b), "match data size for {n}");
            cf(a);
            rf(b);
        }
        // Freeing NULL is explicitly allowed.
        cf(std::ptr::null_mut());
        rf(std::ptr::null_mut());
    }
}

#[test]
fn match_data_create_from_pattern_matches() {
    let (cc, rc) = both::<MdCreateFromPat>("pcre2_match_data_create_from_pattern_8");
    let (cf, rf) = both::<MdFree>("pcre2_match_data_free_8");
    let (gcc, gcr) = both::<GetU32Fn>("pcre2_get_ovector_count_8");
    let (gdc, gdr) = both::<GetSizeFn>("pcre2_get_match_data_size_8");
    for p in patterns() {
        let Some(pair) = compile_both(p, 0) else {
            continue;
        };
        unsafe {
            let a = cc(pair.c, std::ptr::null_mut());
            let b = rc(pair.r, std::ptr::null_mut());
            assert_eq!(gcc(a), gcr(b));
            assert_eq!(gdc(a), gdr(b));
            cf(a);
            rf(b);
        }
    }
}

/// Core driver: run every pattern against every subject at several start
/// offsets and compare the outcome.
fn drive(opts_compile: u32, match_opts: &[u32], ovecsize: u32) {
    let (mc, mr) = both::<Match>("pcre2_match_8");
    let subs = subjects();
    let md = md_pair(ovecsize);
    for p in patterns() {
        let Some(pair) = compile_both(p, opts_compile) else {
            continue;
        };
        let show = String::from_utf8_lossy(p).to_string();
        for s in &subs {
            // Copy into a padded buffer so that reads at the end are defined.
            let mut buf = vec![0u8; s.len() + 16];
            buf[..s.len()].copy_from_slice(s);
            let base = buf.as_ptr();
            for &start in &[0usize, 1, 2, s.len() / 2, s.len()] {
                if start > s.len() {
                    continue;
                }
                for &mo in match_opts {
                    // Telling PCRE2 to skip the UTF validity check on an invalid
                    // subject is undefined behaviour in the C code (lookbehind can
                    // walk off the buffer), so only pair PCRE2_NO_UTF_CHECK with a
                    // fully well-formed subject and a character-aligned offset.
                    if (mo & PCRE2_NO_UTF_CHECK) != 0
                        && (opts_compile & PCRE2_UTF) != 0
                        && (std::str::from_utf8(s).is_err()
                            || !std::str::from_utf8(s).map(|t| t.is_char_boundary(start)).unwrap_or(false))
                    {
                        continue;
                    }
                    unsafe {
                        md.poison();
                        let rc = mc(pair.c, base, s.len(), start, mo, md.c, std::ptr::null_mut());
                        let rr = mr(pair.r, base, s.len(), start, mo, md.r, std::ptr::null_mut());
                        assert_match_equal(
                            &format!(
                                "match {show:?} / {s:02x?} start={start} mo={mo:#x} co={opts_compile:#x}"
                            ),
                            rc,
                            rr,
                            md.c,
                            md.r,
                            base,
                            s.len(),
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn match_default_options() {
    drive(0, &[0], 16);
}

#[test]
fn match_all_match_options() {
    drive(0, MATCH_OPTS, 16);
}

#[test]
fn match_utf_and_ucp() {
    for co in [
        PCRE2_UTF,
        PCRE2_UCP,
        PCRE2_UTF | PCRE2_UCP,
        PCRE2_UTF | PCRE2_UCP | PCRE2_CASELESS,
        PCRE2_UTF | PCRE2_MATCH_INVALID_UTF,
    ] {
        drive(co, &[0, PCRE2_NO_UTF_CHECK, PCRE2_PARTIAL_SOFT], 16);
    }
}

#[test]
fn match_other_compile_options() {
    for co in [
        PCRE2_CASELESS,
        PCRE2_MULTILINE,
        PCRE2_DOTALL,
        PCRE2_UNGREEDY,
        PCRE2_ANCHORED,
        PCRE2_ENDANCHORED,
        PCRE2_NO_START_OPTIMIZE,
        PCRE2_NO_AUTO_POSSESS,
        PCRE2_FIRSTLINE,
        PCRE2_DOLLAR_ENDONLY,
        PCRE2_MATCH_UNSET_BACKREF,
        PCRE2_NO_AUTO_CAPTURE,
        PCRE2_ALT_CIRCUMFLEX,
        PCRE2_DUPNAMES,
        PCRE2_LITERAL,
    ] {
        drive(co, &[0, PCRE2_NOTBOL | PCRE2_NOTEOL], 16);
    }
}

#[test]
fn match_tiny_ovector() {
    // ovecsize 0 and 1 exercise the "ovector too small" bookkeeping.
    for n in [0u32, 1, 2] {
        drive(0, &[0], n);
    }
}

#[test]
fn match_with_limits() {
    let (mc, mr) = both::<Match>("pcre2_match_8");
    let (cc, rc) = both::<CtxCreate>("pcre2_match_context_create_8");
    let (cf, rf) = both::<CtxFree>("pcre2_match_context_free_8");
    let md = md_pair(16);
    // Patterns prone to heavy backtracking, to trip the limits.
    let hard: &[&[u8]] = &[
        b"(a+)+b",
        b"(a|aa)+c",
        b"a{1,1000}b",
        b"(?:a*)*b",
        b"\\((?>[^()]|(?R))*\\)",
        b"a(?R)?b",
        b"(?:(?:(?:a?)?)?)?b",
        b"abc",
        b".*.*.*x",
    ];
    let subs: &[&[u8]] = &[
        b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaab",
        b"((((((()))))))",
        b"abc",
        b"",
    ];
    unsafe {
        let a = cc(std::ptr::null_mut());
        let b = rc(std::ptr::null_mut());
        let (sml, smr) = both::<SetU32>("pcre2_set_match_limit_8");
        let (sdl, sdr) = both::<SetU32>("pcre2_set_depth_limit_8");
        let (shl, shr) = both::<SetU32>("pcre2_set_heap_limit_8");
        let (sol, sor) = both::<SetSize>("pcre2_set_offset_limit_8");
        for &lim in &[0u32, 1, 2, 5, 10, 100, 1000] {
            assert_eq!(sml(a, lim), smr(b, lim));
            assert_eq!(sdl(a, lim), sdr(b, lim));
            for p in hard {
                let Some(pair) = compile_both(p, 0) else {
                    continue;
                };
                for s in subs {
                    md.poison();
                    let rcv = mc(pair.c, s.as_ptr(), s.len(), 0, 0, md.c, a);
                    let rrv = mr(pair.r, s.as_ptr(), s.len(), 0, 0, md.r, b);
                    assert_match_equal(
                        &format!("limited match {p:02x?} / {s:02x?} lim={lim}"),
                        rcv,
                        rrv,
                        md.c,
                        md.r,
                        s.as_ptr(),
                        s.len(),
                    );
                }
            }
        }
        assert_eq!(sml(a, 10_000_000), smr(b, 10_000_000));
        assert_eq!(sdl(a, 10_000_000), sdr(b, 10_000_000));
        for &lim in &[0u32, 1, 2, 20] {
            assert_eq!(shl(a, lim), shr(b, lim));
            for p in hard {
                let Some(pair) = compile_both(p, 0) else {
                    continue;
                };
                for s in subs {
                    md.poison();
                    let rcv = mc(pair.c, s.as_ptr(), s.len(), 0, 0, md.c, a);
                    let rrv = mr(pair.r, s.as_ptr(), s.len(), 0, 0, md.r, b);
                    assert_match_equal(
                        &format!("heap-limited match {p:02x?} / {s:02x?} lim={lim}"),
                        rcv,
                        rrv,
                        md.c,
                        md.r,
                        s.as_ptr(),
                        s.len(),
                    );
                }
            }
        }
        assert_eq!(shl(a, 20_000_000), shr(b, 20_000_000));

        // Offset limit only takes effect with PCRE2_USE_OFFSET_LIMIT.
        for &lim in &[0usize, 1, 2, 5, PCRE2_UNSET] {
            assert_eq!(sol(a, lim), sor(b, lim));
            for p in [b"abc".as_slice(), b"b", b"c", b"\\d"] {
                for co in [PCRE2_USE_OFFSET_LIMIT, 0] {
                    let Some(pair) = compile_both(p, co) else {
                        continue;
                    };
                    for s in [b"abcabcabc".as_slice(), b"xxbxx", b"", b"c"] {
                        md.poison();
                        let rcv = mc(pair.c, s.as_ptr(), s.len(), 0, 0, md.c, a);
                        let rrv = mr(pair.r, s.as_ptr(), s.len(), 0, 0, md.r, b);
                        assert_match_equal(
                            &format!("offset-limited match {p:02x?} / {s:02x?} lim={lim}"),
                            rcv,
                            rrv,
                            md.c,
                            md.r,
                            s.as_ptr(),
                            s.len(),
                        );
                    }
                }
            }
        }
        cf(a);
        rf(b);
    }
}

#[test]
fn match_zero_terminated_subject() {
    let (mc, mr) = both::<Match>("pcre2_match_8");
    let md = md_pair(16);
    for p in patterns() {
        let Some(pair) = compile_both(p, 0) else {
            continue;
        };
        for s in subjects() {
            if s.contains(&0) {
                continue;
            }
            let mut z = s.to_vec();
            z.push(0);
            unsafe {
                md.poison();
                let rc = mc(
                    pair.c,
                    z.as_ptr(),
                    PCRE2_ZERO_TERMINATED,
                    0,
                    0,
                    md.c,
                    std::ptr::null_mut(),
                );
                let rr = mr(
                    pair.r,
                    z.as_ptr(),
                    PCRE2_ZERO_TERMINATED,
                    0,
                    0,
                    md.r,
                    std::ptr::null_mut(),
                );
                assert_match_equal(
                    &format!("zt match {p:02x?} / {s:02x?}"),
                    rc,
                    rr,
                    md.c,
                    md.r,
                    z.as_ptr(),
                    s.len(),
                );
            }
        }
    }
}

#[test]
fn match_bad_start_offset() {
    let (mc, mr) = both::<Match>("pcre2_match_8");
    let md = md_pair(16);
    let Some(pair) = compile_both(b"a", 0) else {
        panic!("pattern must compile")
    };
    let s = b"abcdef";
    unsafe {
        for start in [6usize, 7, 100, usize::MAX] {
            let rc = mc(pair.c, s.as_ptr(), s.len(), start, 0, md.c, std::ptr::null_mut());
            let rr = mr(pair.r, s.as_ptr(), s.len(), start, 0, md.r, std::ptr::null_mut());
            assert_eq!(rc, rr, "bad start offset {start}");
        }
        // NULL code / NULL match data.
        assert_eq!(
            mc(std::ptr::null(), s.as_ptr(), s.len(), 0, 0, md.c, std::ptr::null_mut()),
            mr(std::ptr::null(), s.as_ptr(), s.len(), 0, 0, md.r, std::ptr::null_mut()),
            "NULL code"
        );
        assert_eq!(
            mc(pair.c, s.as_ptr(), s.len(), 0, 0, std::ptr::null_mut(), std::ptr::null_mut()),
            mr(pair.r, s.as_ptr(), s.len(), 0, 0, std::ptr::null_mut(), std::ptr::null_mut()),
            "NULL match data"
        );
        // Invalid option bits.
        for bad in [0x8000_0000u32 >> 1, 0x0080_0000, 0xffff_ffff] {
            assert_eq!(
                mc(pair.c, s.as_ptr(), s.len(), 0, bad, md.c, std::ptr::null_mut()),
                mr(pair.r, s.as_ptr(), s.len(), 0, bad, md.r, std::ptr::null_mut()),
                "bad options {bad:#x}"
            );
        }
    }
}

#[test]
fn next_match_matches() {
    // pcre2_next_match iterates over the alternative match ends recorded by
    // a PCRE2_PARTIAL / multi-end match.
    let (mc, mr) = both::<Match>("pcre2_match_8");
    let (nc, nr) = both::<
        unsafe extern "C" fn(*mut c_void, *mut PCRE2_SIZE, *mut u32) -> i32,
    >("pcre2_next_match_8");
    let md = md_pair(16);
    for p in patterns() {
        let Some(pair) = compile_both(p, 0) else {
            continue;
        };
        for s in subjects() {
            for mo in [0u32, PCRE2_PARTIAL_SOFT, PCRE2_PARTIAL_HARD] {
                unsafe {
                    let rc = mc(pair.c, s.as_ptr(), s.len(), 0, mo, md.c, std::ptr::null_mut());
                    let rr = mr(pair.r, s.as_ptr(), s.len(), 0, mo, md.r, std::ptr::null_mut());
                    assert_eq!(rc, rr);
                    // Drain both iterators in lockstep.
                    for _ in 0..8 {
                        let mut oc: PCRE2_SIZE = 0xdead;
                        let mut orr: PCRE2_SIZE = 0xdead;
                        let mut cc: u32 = 0xdead;
                        let mut cr: u32 = 0xdead;
                        let a = nc(md.c, &mut oc, &mut cc);
                        let b = nr(md.r, &mut orr, &mut cr);
                        assert_eq!(a, b, "next_match rc {p:02x?} / {s:02x?} mo={mo:#x}");
                        if a == 0 {
                            break;
                        }
                        assert_eq!(oc, orr, "next_match offset {p:02x?} / {s:02x?}");
                        assert_eq!(cc, cr, "next_match count {p:02x?} / {s:02x?}");
                    }
                }
            }
        }
    }
}
