//! `pcre2_dfa_match.c`.
mod common;

use common::*;
use std::ffi::{c_int, c_void};

type DfaMatch = unsafe extern "C" fn(
    *const c_void,
    PCRE2_SPTR,
    PCRE2_SIZE,
    PCRE2_SIZE,
    u32,
    *mut c_void,
    *mut c_void,
    *mut c_int,
    PCRE2_SIZE,
) -> i32;
type MdCreate = unsafe extern "C" fn(u32, *mut c_void) -> *mut c_void;
type MdFree = unsafe extern "C" fn(*mut c_void);
type GetU32Fn = unsafe extern "C" fn(*mut c_void) -> u32;
type GetSizeFn = unsafe extern "C" fn(*mut c_void) -> PCRE2_SIZE;
type CtxCreate = unsafe extern "C" fn(*mut c_void) -> *mut c_void;
type CtxFree = unsafe extern "C" fn(*mut c_void);
type SetU32 = unsafe extern "C" fn(*mut c_void, u32) -> i32;

const DFA_OPTS: &[u32] = &[
    0,
    PCRE2_NOTBOL,
    PCRE2_NOTEOL,
    PCRE2_NOTEMPTY,
    PCRE2_NOTEMPTY_ATSTART,
    PCRE2_ANCHORED,
    PCRE2_ENDANCHORED,
    PCRE2_PARTIAL_SOFT,
    PCRE2_PARTIAL_HARD,
    PCRE2_DFA_SHORTEST,
    PCRE2_DFA_SHORTEST | PCRE2_PARTIAL_SOFT,
];

struct Pair {
    c: *mut c_void,
    r: *mut c_void,
    fc: libloading::Symbol<'static, MdFree>,
    fr: libloading::Symbol<'static, MdFree>,
}

impl Drop for Pair {
    fn drop(&mut self) {
        unsafe {
            (self.fc)(self.c);
            (self.fr)(self.r);
        }
    }
}

fn md_pair(n: u32) -> Pair {
    let (cc, rc) = both::<MdCreate>("pcre2_match_data_create_8");
    let (cf, rf) = both::<MdFree>("pcre2_match_data_free_8");
    unsafe {
        Pair {
            c: cc(n, std::ptr::null_mut()),
            r: rc(n, std::ptr::null_mut()),
            fc: cf,
            fr: rf,
        }
    }
}

unsafe fn compare(
    label: &str,
    rc_c: i32,
    rc_r: i32,
    md_c: *mut c_void,
    md_r: *mut c_void,
    subject: *const u8,
    subject_len: usize,
    ws_c: &[c_int],
    ws_r: &[c_int],
) {
    unsafe {
        assert_eq!(rc_c, rc_r, "{label}: return code");
        assert_eq!(
            md_snapshot(md_c, subject, subject_len),
            md_snapshot(md_r, subject, subject_len),
            "{label}: match data"
        );
        let (gsc, gsr) = both::<GetSizeFn>("pcre2_get_startchar_8");
        assert_eq!(gsc(md_c), gsr(md_r), "{label}: get_startchar");
        assert_eq!(ws_c, ws_r, "{label}: dfa workspace");
    }
}

fn drive(compile_opts: u32, dfa_opts: &[u32], ovecsize: u32, wscount: usize) {
    let (dc, dr) = both::<DfaMatch>("pcre2_dfa_match_8");
    let (gcc, gcr) = both::<GetU32Fn>("pcre2_get_ovector_count_8");
    let md = md_pair(ovecsize);
    let subs = subjects();
    for p in patterns() {
        let Some(pair) = compile_both(p, compile_opts) else {
            continue;
        };
        let show = String::from_utf8_lossy(p).to_string();
        for s in &subs {
            let mut buf = vec![0u8; s.len() + 16];
            buf[..s.len()].copy_from_slice(s);
            for &start in &[0usize, 1, s.len() / 2, s.len()] {
                if start > s.len() {
                    continue;
                }
                for &mo in dfa_opts {
                    if (mo & PCRE2_NO_UTF_CHECK) != 0 && (compile_opts & PCRE2_UTF) != 0 {
                        continue;
                    }
                    let mut ws_c = vec![0x5A5A_5A5Ai32; wscount];
                    let mut ws_r = vec![0x5A5A_5A5Ai32; wscount];
                    unsafe {
                        md_poison(md.c, gcc(md.c) as u16);
                        md_poison(md.r, gcr(md.r) as u16);
                        let rc = dc(
                            pair.c,
                            buf.as_ptr(),
                            s.len(),
                            start,
                            mo,
                            md.c,
                            std::ptr::null_mut(),
                            ws_c.as_mut_ptr(),
                            wscount,
                        );
                        let rr = dr(
                            pair.r,
                            buf.as_ptr(),
                            s.len(),
                            start,
                            mo,
                            md.r,
                            std::ptr::null_mut(),
                            ws_r.as_mut_ptr(),
                            wscount,
                        );
                        compare(
                            &format!(
                                "dfa {show:?} / {s:02x?} start={start} mo={mo:#x} co={compile_opts:#x} ws={wscount}"
                            ),
                            rc,
                            rr,
                            md.c,
                            md.r,
                            buf.as_ptr(),
                            s.len(),
                            &ws_c,
                            &ws_r,
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn dfa_default_options() {
    drive(0, &[0], 16, 1000);
}

#[test]
fn dfa_all_options() {
    drive(0, DFA_OPTS, 16, 1000);
}

#[test]
fn dfa_utf_and_ucp() {
    for co in [
        PCRE2_UTF,
        PCRE2_UCP,
        PCRE2_UTF | PCRE2_UCP,
        PCRE2_UTF | PCRE2_UCP | PCRE2_CASELESS,
    ] {
        drive(co, &[0, PCRE2_PARTIAL_SOFT, PCRE2_DFA_SHORTEST], 16, 1000);
    }
}

#[test]
fn dfa_other_compile_options() {
    for co in [
        PCRE2_CASELESS,
        PCRE2_MULTILINE,
        PCRE2_DOTALL,
        PCRE2_UNGREEDY,
        PCRE2_ANCHORED,
        PCRE2_ENDANCHORED,
        PCRE2_NO_START_OPTIMIZE,
        PCRE2_FIRSTLINE,
        PCRE2_DOLLAR_ENDONLY,
        PCRE2_MATCH_UNSET_BACKREF,
    ] {
        drive(co, &[0, PCRE2_PARTIAL_SOFT], 16, 1000);
    }
}

#[test]
fn dfa_small_workspace_and_ovector() {
    // Tiny workspaces trip PCRE2_ERROR_DFA_WSSIZE; tiny ovectors trip the
    // "not enough room" bookkeeping.
    for ws in [20usize, 21, 30, 100] {
        drive(0, &[0], 2, ws);
    }
    for n in [0u32, 1, 2] {
        drive(0, &[0], n, 1000);
    }
}

#[test]
fn dfa_bad_arguments() {
    let (dc, dr) = both::<DfaMatch>("pcre2_dfa_match_8");
    let md = md_pair(16);
    let Some(pair) = compile_both(b"a(b)c", 0) else {
        panic!()
    };
    let s = b"abcdef";
    let mut ws_c = vec![0i32; 100];
    let mut ws_r = vec![0i32; 100];
    unsafe {
        // Workspace too small.
        for n in [0usize, 1, 10, 19] {
            assert_eq!(
                dc(pair.c, s.as_ptr(), s.len(), 0, 0, md.c, std::ptr::null_mut(), ws_c.as_mut_ptr(), n),
                dr(pair.r, s.as_ptr(), s.len(), 0, 0, md.r, std::ptr::null_mut(), ws_r.as_mut_ptr(), n),
                "wscount {n}"
            );
        }
        // Bad start offset, NULL args, bad options.
        for start in [7usize, 100, usize::MAX] {
            assert_eq!(
                dc(pair.c, s.as_ptr(), s.len(), start, 0, md.c, std::ptr::null_mut(), ws_c.as_mut_ptr(), 100),
                dr(pair.r, s.as_ptr(), s.len(), start, 0, md.r, std::ptr::null_mut(), ws_r.as_mut_ptr(), 100),
                "start {start}"
            );
        }
        assert_eq!(
            dc(std::ptr::null(), s.as_ptr(), s.len(), 0, 0, md.c, std::ptr::null_mut(), ws_c.as_mut_ptr(), 100),
            dr(std::ptr::null(), s.as_ptr(), s.len(), 0, 0, md.r, std::ptr::null_mut(), ws_r.as_mut_ptr(), 100),
            "NULL code"
        );
        for bad in [PCRE2_SUBSTITUTE_GLOBAL, 0xffff_ffff, PCRE2_NO_JIT] {
            assert_eq!(
                dc(pair.c, s.as_ptr(), s.len(), 0, bad, md.c, std::ptr::null_mut(), ws_c.as_mut_ptr(), 100),
                dr(pair.r, s.as_ptr(), s.len(), 0, bad, md.r, std::ptr::null_mut(), ws_r.as_mut_ptr(), 100),
                "bad options {bad:#x}"
            );
        }
    }
}

#[test]
fn dfa_restart_sequence() {
    // Feed the subject in two halves using PCRE2_DFA_RESTART, which requires the
    // workspace to be carried over between calls.
    let (dc, dr) = both::<DfaMatch>("pcre2_dfa_match_8");
    let (gcc, gcr) = both::<GetU32Fn>("pcre2_get_ovector_count_8");
    let md = md_pair(16);
    let pats: &[&[u8]] = &[b"abc", b"a.*z", b"\\d+", b"(foo|bar)baz", b"a+b+c+"];
    let subs: &[&[u8]] = &[b"abcabc", b"aXXXz", b"12345", b"foobaz", b"aabbcc", b""];
    for p in pats {
        let Some(pair) = compile_both(p, 0) else {
            continue;
        };
        for s in subs {
            for split in 0..=s.len() {
                let mut ws_c = vec![0x5A5A_5A5Ai32; 200];
                let mut ws_r = vec![0x5A5A_5A5Ai32; 200];
                unsafe {
                    md_poison(md.c, gcc(md.c) as u16);
                    md_poison(md.r, gcr(md.r) as u16);
                    let rc1 = dc(
                        pair.c, s.as_ptr(), split, 0, PCRE2_PARTIAL_SOFT, md.c,
                        std::ptr::null_mut(), ws_c.as_mut_ptr(), 200,
                    );
                    let rr1 = dr(
                        pair.r, s.as_ptr(), split, 0, PCRE2_PARTIAL_SOFT, md.r,
                        std::ptr::null_mut(), ws_r.as_mut_ptr(), 200,
                    );
                    compare(
                        &format!("dfa restart part1 {p:02x?}/{s:02x?} split={split}"),
                        rc1, rr1, md.c, md.r, s.as_ptr(), split, &ws_c, &ws_r,
                    );
                    if rc1 == -2 {
                        md_poison(md.c, gcc(md.c) as u16);
                        md_poison(md.r, gcr(md.r) as u16);
                        let rc2 = dc(
                            pair.c, s.as_ptr(), s.len(), split,
                            PCRE2_DFA_RESTART | PCRE2_PARTIAL_SOFT, md.c,
                            std::ptr::null_mut(), ws_c.as_mut_ptr(), 200,
                        );
                        let rr2 = dr(
                            pair.r, s.as_ptr(), s.len(), split,
                            PCRE2_DFA_RESTART | PCRE2_PARTIAL_SOFT, md.r,
                            std::ptr::null_mut(), ws_r.as_mut_ptr(), 200,
                        );
                        compare(
                            &format!("dfa restart part2 {p:02x?}/{s:02x?} split={split}"),
                            rc2, rr2, md.c, md.r, s.as_ptr(), s.len(), &ws_c, &ws_r,
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn dfa_with_limits() {
    let (dc, dr) = both::<DfaMatch>("pcre2_dfa_match_8");
    let (gcc, gcr) = both::<GetU32Fn>("pcre2_get_ovector_count_8");
    let (cc, rc) = both::<CtxCreate>("pcre2_match_context_create_8");
    let (cf, rf) = both::<CtxFree>("pcre2_match_context_free_8");
    let md = md_pair(16);
    let hard: &[&[u8]] = &[b"(a+)+b", b"a(?R)?b", b"\\((?>[^()]|(?R))*\\)", b"(?:a*)*b"];
    let subs: &[&[u8]] = &[b"aaaaaaaaaaaaaaaaaaaa", b"((((()))))", b"abc"];
    unsafe {
        let a = cc(std::ptr::null_mut());
        let b = rc(std::ptr::null_mut());
        let (smc, smr) = both::<SetU32>("pcre2_set_match_limit_8");
        let (sdc, sdr) = both::<SetU32>("pcre2_set_depth_limit_8");
        for lim in [0u32, 1, 5, 100, 10000] {
            assert_eq!(smc(a, lim), smr(b, lim));
            assert_eq!(sdc(a, lim), sdr(b, lim));
            for p in hard {
                let Some(pair) = compile_both(p, 0) else {
                    continue;
                };
                for s in subs {
                    let mut ws_c = vec![0x5A5A_5A5Ai32; 1000];
                    let mut ws_r = vec![0x5A5A_5A5Ai32; 1000];
                    md_poison(md.c, gcc(md.c) as u16);
                    md_poison(md.r, gcr(md.r) as u16);
                    let rcv = dc(pair.c, s.as_ptr(), s.len(), 0, 0, md.c, a, ws_c.as_mut_ptr(), 1000);
                    let rrv = dr(pair.r, s.as_ptr(), s.len(), 0, 0, md.r, b, ws_r.as_mut_ptr(), 1000);
                    compare(
                        &format!("dfa limited {p:02x?}/{s:02x?} lim={lim}"),
                        rcv, rrv, md.c, md.r, s.as_ptr(), s.len(), &ws_c, &ws_r,
                    );
                }
            }
        }
        cf(a);
        rf(b);
    }
}
