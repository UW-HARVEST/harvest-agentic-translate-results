//! Differential tests for `CONFIGS.md` rows **C158 - C169**.
//!
//! * C158 - `pcre2_dfa_match_8` argument validation / option legality / workspace floor
//! * C159 - workspace sizing (`PCRE2_ERROR_DFA_WSSIZE` thresholds)
//! * C160 - `PCRE2_DFA_RESTART` (workspace contents, sanity checks, cross-pattern restart)
//! * C161 - `PCRE2_DFA_SHORTEST`
//! * C162 - the multi-match ovector semantics (longest-first, overflow -> rc 0)
//! * C163 - unsupported items (`PCRE2_ERROR_DFA_UITEM`, `PCRE2_ERROR_DFA_RECURSE`, callouts)
//! * C164 - unsupported conditions (`PCRE2_ERROR_DFA_UCOND`)
//! * C165 - match / depth / heap limits (`more_workspace`)
//! * C166 - partial matching
//! * C167 - `PCRE2_ERROR_DFA_UFUNC` from the substring / substitute entry points
//! * C168 - `pcre2_next_match_8` three-way classification
//! * C169 - `pcre2_next_match_8` bumpalong (CRLF / UTF / ill-formed UTF / ordinary)
//!
//! Everything is compared C-`.so` against Rust-`.so`: the return code, the *whole*
//! ovector (as filled by the DFA multi-match logic), `startchar`, `mark`, and - for the
//! DFA - the full `int` workspace the call leaves behind.

mod common;
use common::*;

use std::alloc::{alloc_zeroed, dealloc, Layout};
use std::collections::HashMap;
use std::ptr;

/// `PCRE2_ERROR_DFA_RECURSE` (pcre2.h) - not present in `common`.
const PCRE2_ERROR_DFA_RECURSE: i32 = -39;

/// Sentinel written into every ovector slot before a call so that "not written by the
/// library" is observable and identical for both libraries.
const SENT: usize = 0x5A5A_5A5A_5A5A_5A5A;

/// `sizeof(stateblock)/sizeof(int)` (pcre2_dfa_match.c:303-309).
const INTS_PER_STATEBLOCK: usize = 3;

// ---------------------------------------------------------------------------------
// A zeroing allocator.  `pcre2_match_data_create` does NOT initialise `startchar`,
// `rc`, `subject_length` ... so a match data block obtained from the default malloc
// starts out with indeterminate contents.  Handing both libraries a *zeroed* block
// makes every field we read back deterministic even on the early-error return paths.
// ---------------------------------------------------------------------------------
const HDR: usize = 16;

unsafe extern "C" fn zmalloc(size: usize, _d: Ptr) -> Ptr {
    let total = size + HDR;
    let lay = Layout::from_size_align(total, 16).unwrap();
    let p = alloc_zeroed(lay);
    if p.is_null() {
        return ptr::null_mut();
    }
    (p as *mut usize).write(total);
    p.add(HDR) as Ptr
}

unsafe extern "C" fn zfree(p: Ptr, _d: Ptr) {
    if p.is_null() {
        return;
    }
    let base = (p as *mut u8).sub(HDR);
    let total = (base as *mut usize).read();
    dealloc(base, Layout::from_size_align(total, 16).unwrap());
}

/// Same, but refuses any "big" request.  `more_workspace()` asks for
/// `2 * DFA_START_RWS_SIZE = 61440` bytes, so this forces `PCRE2_ERROR_NOMEMORY`
/// there while leaving every small allocation (contexts, match data) working.
unsafe extern "C" fn zmalloc_smallonly(size: usize, d: Ptr) -> Ptr {
    if size > 20000 {
        return ptr::null_mut();
    }
    zmalloc(size, d)
}

// ---------------------------------------------------------------------------------
// Harness plumbing
// ---------------------------------------------------------------------------------

/// The two libraries plus a zeroing general context in each of them.
struct Pair {
    c: &'static Api,
    r: &'static Api,
    gc: Ptr,
    gr: Ptr,
    n: usize,
}

impl Pair {
    fn new() -> Pair {
        let (c, r) = both();
        unsafe {
            let gc = (c.pcre2_general_context_create_8)(Some(zmalloc), Some(zfree), ptr::null_mut());
            let gr = (r.pcre2_general_context_create_8)(Some(zmalloc), Some(zfree), ptr::null_mut());
            assert!(!gc.is_null() && !gr.is_null());
            Pair { c, r, gc, gr, n: 0 }
        }
    }

    unsafe fn md(&self, which: bool, ovn: u32) -> Ptr {
        let (api, g) = if which { (self.c, self.gc) } else { (self.r, self.gr) };
        let md = (api.pcre2_match_data_create_8)(ovn, g);
        assert!(!md.is_null());
        md
    }

    /// Compile the same pattern in both libraries; asserts that success/failure and the
    /// error code/offset agree.  Returns `None` when both refused the pattern.
    unsafe fn compile(
        &mut self,
        pat: &[u8],
        opts: u32,
        xopts: u32,
        nl: u32,
        bsr: u32,
    ) -> Option<(Ptr, Ptr)> {
        let mut code = [ptr::null_mut(); 2];
        let mut ec = [0i32; 2];
        let mut eo = [0usize; 2];
        for (i, api) in [self.c, self.r].into_iter().enumerate() {
            let cc = (api.pcre2_compile_context_create_8)(ptr::null_mut());
            assert!(!cc.is_null());
            if nl != 0 {
                assert_eq!((api.pcre2_set_newline_8)(cc, nl), 0);
            }
            if bsr != 0 {
                assert_eq!((api.pcre2_set_bsr_8)(cc, bsr), 0);
            }
            if xopts != 0 {
                assert_eq!((api.pcre2_set_compile_extra_options_8)(cc, xopts), 0);
            }
            let mut e = 0i32;
            let mut o = 0usize;
            code[i] = (api.pcre2_compile_8)(pat.as_ptr(), pat.len(), opts, &mut e, &mut o, cc);
            ec[i] = e;
            eo[i] = o;
            (api.pcre2_compile_context_free_8)(cc);
        }
        self.n += 1;
        assert_eq!(
            code[0].is_null(),
            code[1].is_null(),
            "compile disagreement pat={:?} opts={:#x} xopts={:#x} nl={} bsr={}: \
             C null={} (ec={} eo={}), RUST null={} (ec={} eo={})",
            String::from_utf8_lossy(pat),
            opts,
            xopts,
            nl,
            bsr,
            code[0].is_null(),
            ec[0],
            eo[0],
            code[1].is_null(),
            ec[1],
            eo[1]
        );
        if code[0].is_null() {
            assert_eq!(
                (ec[0], eo[0]),
                (ec[1], eo[1]),
                "compile error mismatch pat={:?} opts={:#x}",
                String::from_utf8_lossy(pat),
                opts
            );
            return None;
        }
        Some((code[0], code[1]))
    }

    unsafe fn free_code(&self, codes: (Ptr, Ptr)) {
        (self.c.pcre2_code_free_8)(codes.0);
        (self.r.pcre2_code_free_8)(codes.1);
    }
}

unsafe fn fill_ov(api: &Api, md: Ptr, v: usize) {
    let n = (api.pcre2_get_ovector_count_8)(md) as usize;
    let p = (api.pcre2_get_ovector_pointer_8)(md);
    for i in 0..n * 2 {
        *p.add(i) = v;
    }
}

/// Everything one `pcre2_dfa_match` call produces.
#[derive(Debug, PartialEq, Eq, Clone)]
struct DfaOut {
    m: MatchOut,
    ws: Vec<i32>,
}

/// One DFA call into an existing match data block, with an explicitly sized workspace.
/// `wsbuf` ints are actually allocated (>= `wsc`) so that an over-large `wscount`
/// cannot be produced accidentally.
#[allow(clippy::too_many_arguments)]
unsafe fn dfa_one(
    api: &Api,
    md: Ptr,
    code: Ptr,
    subj: *const u8,
    slen: usize,
    so: usize,
    opts: u32,
    mctx: Ptr,
    ws: &mut [i32],
    wsc: usize,
    prefill: bool,
) -> DfaOut {
    if prefill && !md.is_null() {
        fill_ov(api, md, SENT);
    }
    let rc = (api.pcre2_dfa_match_8)(code, subj, slen, so, opts, md, mctx, ws.as_mut_ptr(), wsc);
    let m = if md.is_null() {
        MatchOut { rc, ovector: vec![], startchar: 0, mark: None }
    } else {
        read_match(api, md, rc)
    };
    DfaOut { m, ws: ws.to_vec() }
}

/// The shape of one DFA comparison.
#[derive(Clone, Copy, Debug)]
struct K {
    so: usize,
    opts: u32,
    ovn: u32,
    /// value handed to the library as `wscount`
    wsc: usize,
    /// ints actually allocated
    wsbuf: usize,
}

/// Allocate generously more than `wscount` ints.  Hand-crafted `PCRE2_DFA_RESTART`
/// data can make the C `memcpy` past `wscount` (see C160), and the padding keeps such a
/// write inside our own allocation instead of corrupting the heap.  It also lets the
/// comparison assert that neither library writes past `wscount` in the normal cases.
fn wsbuf_for(wsc: usize) -> usize {
    wsc * 2 + 64
}

impl K {
    fn new(so: usize, opts: u32, ovn: u32, wsc: usize) -> K {
        K { so, opts, ovn, wsc, wsbuf: wsbuf_for(wsc) }
    }
}

/// Run one DFA call in both libraries and assert the complete observable result is
/// identical.  Returns the (shared) result.
#[allow(clippy::too_many_arguments)]
unsafe fn dfa_cmp(
    p: &mut Pair,
    codes: (Ptr, Ptr),
    mctx: (Ptr, Ptr),
    subj: &[u8],
    slen: usize,
    k: K,
    label: &str,
) -> DfaOut {
    let mdc = p.md(true, k.ovn);
    let mdr = p.md(false, k.ovn);
    let mut wsc = vec![0i32; k.wsbuf];
    let mut wsr = vec![0i32; k.wsbuf];
    let oc = dfa_one(
        p.c, mdc, codes.0, subj.as_ptr(), slen, k.so, k.opts, mctx.0, &mut wsc, k.wsc, true,
    );
    let or = dfa_one(
        p.r, mdr, codes.1, subj.as_ptr(), slen, k.so, k.opts, mctx.1, &mut wsr, k.wsc, true,
    );
    (p.c.pcre2_match_data_free_8)(mdc);
    (p.r.pcre2_match_data_free_8)(mdr);
    p.n += 1;
    assert_eq!(
        oc.m, or.m,
        "[{}] DFA match mismatch: subj={:?} slen={} {:?}\n  C   ={:?}\n  RUST={:?}",
        label,
        String::from_utf8_lossy(subj),
        slen,
        k,
        oc.m,
        or.m
    );
    assert_eq!(
        oc.ws, or.ws,
        "[{}] DFA workspace mismatch: subj={:?} slen={} {:?}",
        label,
        String::from_utf8_lossy(subj),
        slen,
        k
    );
    oc
}

// ---------------------------------------------------------------------------------
// C158 - argument validation, option legality, the wscount floor
// ---------------------------------------------------------------------------------

#[test]
fn c158_dfa_argument_validation() {
    println!("C158 pcre2_dfa_match_8 argument validation / option legality");
    let mut p = Pair::new();
    unsafe {
        let codes = p.compile(b"abc", 0, 0, 0, 0).unwrap();
        let subj = b"xxabcxx\0\0\0\0\0\0\0\0".to_vec();

        // ---- match_data == NULL -> PCRE2_ERROR_NULL, and nothing else is touched.
        {
            let mut wsc = vec![0i32; 100];
            let mut wsr = vec![0i32; 100];
            let a = dfa_one(
                p.c, ptr::null_mut(), codes.0, subj.as_ptr(), 7, 0, 0, ptr::null_mut(),
                &mut wsc, 100, false,
            );
            let b = dfa_one(
                p.r, ptr::null_mut(), codes.1, subj.as_ptr(), 7, 0, 0, ptr::null_mut(),
                &mut wsr, 100, false,
            );
            p.n += 1;
            assert_eq!(a.m.rc, PCRE2_ERROR_NULL);
            assert_eq!((a.m.rc, a.ws), (b.m.rc, b.ws), "C158 md==NULL");
        }

        // ---- code == NULL / subject == NULL / workspace == NULL
        for which in 0..3u32 {
            let mdc = p.md(true, 4);
            let mdr = p.md(false, 4);
            let mut wsc = vec![0i32; 100];
            let mut wsr = vec![0i32; 100];
            let go = |api: &Api, md: Ptr, code: Ptr, ws: &mut Vec<i32>| -> MatchOut {
                fill_ov(api, md, SENT);
                let rc = match which {
                    0 => (api.pcre2_dfa_match_8)(
                        ptr::null_mut(), subj.as_ptr(), 7, 0, 0, md, ptr::null_mut(),
                        ws.as_mut_ptr(), 100,
                    ),
                    1 => (api.pcre2_dfa_match_8)(
                        code, ptr::null(), 7, 0, 0, md, ptr::null_mut(), ws.as_mut_ptr(), 100,
                    ),
                    _ => (api.pcre2_dfa_match_8)(
                        code, subj.as_ptr(), 7, 0, 0, md, ptr::null_mut(), ptr::null_mut(), 100,
                    ),
                };
                read_match(api, md, rc)
            };
            let a = go(p.c, mdc, codes.0, &mut wsc);
            let b = go(p.r, mdr, codes.1, &mut wsr);
            (p.c.pcre2_match_data_free_8)(mdc);
            (p.r.pcre2_match_data_free_8)(mdr);
            p.n += 1;
            assert_eq!(a.rc, PCRE2_ERROR_NULL, "C158 NULL arg {}", which);
            assert_eq!(a, b, "C158 NULL arg {}", which);
            assert_eq!(wsc, wsr, "C158 NULL arg {} workspace", which);
        }

        // ---- subject == NULL with length 0 is an empty string, not an error.
        for pat in [&b""[..], &b"a"[..], &b"a*"[..], &b"(?=)"[..]] {
            let cs = p.compile(pat, 0, 0, 0, 0).unwrap();
            let mdc = p.md(true, 4);
            let mdr = p.md(false, 4);
            let mut wsc = vec![0i32; 100];
            let mut wsr = vec![0i32; 100];
            fill_ov(p.c, mdc, SENT);
            fill_ov(p.r, mdr, SENT);
            let rc_c = (p.c.pcre2_dfa_match_8)(
                cs.0, ptr::null(), 0, 0, 0, mdc, ptr::null_mut(), wsc.as_mut_ptr(), 100,
            );
            let rc_r = (p.r.pcre2_dfa_match_8)(
                cs.1, ptr::null(), 0, 0, 0, mdr, ptr::null_mut(), wsr.as_mut_ptr(), 100,
            );
            let a = read_match(p.c, mdc, rc_c);
            let b = read_match(p.r, mdr, rc_r);
            (p.c.pcre2_match_data_free_8)(mdc);
            (p.r.pcre2_match_data_free_8)(mdr);
            p.free_code(cs);
            p.n += 1;
            assert_eq!(a, b, "C158 NULL/len0 subject pat={:?}", pat);
            assert_eq!(wsc, wsr, "C158 NULL/len0 subject workspace pat={:?}", pat);
        }

        // ---- every single option bit: exactly PUBLIC_DFA_MATCH_OPTIONS is accepted
        // (pcre2_dfa_match.c:83-87).  PCRE2_DISABLE_RECURSELOOP_CHECK is NOT.
        for bit in 0..32u32 {
            let opt = 1u32 << bit;
            let out = dfa_cmp(
                &mut p, codes, (ptr::null_mut(), ptr::null_mut()), &subj, 7,
                K::new(0, opt, 4, 100), "C158 single option bit",
            );
            let legal = PCRE2_ANCHORED
                | PCRE2_ENDANCHORED
                | PCRE2_NOTBOL
                | PCRE2_NOTEOL
                | PCRE2_NOTEMPTY
                | PCRE2_NOTEMPTY_ATSTART
                | PCRE2_NO_UTF_CHECK
                | PCRE2_PARTIAL_HARD
                | PCRE2_PARTIAL_SOFT
                | PCRE2_DFA_SHORTEST
                | PCRE2_DFA_RESTART
                | PCRE2_COPY_MATCHED_SUBJECT;
            if opt & legal == 0 {
                assert_eq!(
                    out.m.rc, PCRE2_ERROR_BADOPTION,
                    "C158 option bit {:#x} should be rejected",
                    opt
                );
            } else {
                assert_ne!(
                    out.m.rc, PCRE2_ERROR_BADOPTION,
                    "C158 option bit {:#x} should be accepted",
                    opt
                );
            }
        }
        // A couple of named combinations, including the option the interpreter has and
        // the DFA has not.
        for opt in [
            PCRE2_DISABLE_RECURSELOOP_CHECK,
            PCRE2_NO_JIT,
            PCRE2_SUBSTITUTE_GLOBAL,
            PCRE2_ANCHORED | PCRE2_DISABLE_RECURSELOOP_CHECK,
            PCRE2_DFA_SHORTEST | PCRE2_DFA_RESTART,
            0xFFFF_FFFF,
        ] {
            dfa_cmp(
                &mut p, codes, (ptr::null_mut(), ptr::null_mut()), &subj, 7,
                K::new(0, opt, 4, 100), "C158 option combo",
            );
        }

        // ---- wscount floor: `if (wscount < 20)` (pcre2_dfa_match.c:3407) is checked
        // BEFORE the start_offset check, so combine the two.
        for &wsc in &[0usize, 1, 2, 19, 20, 21, 100, 1000, 20000] {
            for &so in &[0usize, 7, 8, 99] {
                let out = dfa_cmp(
                    &mut p, codes, (ptr::null_mut(), ptr::null_mut()), &subj, 7,
                    K { so, opts: 0, ovn: 4, wsc, wsbuf: wsbuf_for(wsc) },
                    "C158 wscount/start_offset",
                );
                if wsc < 20 {
                    assert_eq!(out.m.rc, PCRE2_ERROR_DFA_WSSIZE, "C158 wscount={}", wsc);
                } else if so > 7 {
                    assert_eq!(out.m.rc, PCRE2_ERROR_BADOFFSET, "C158 so={}", so);
                }
            }
        }

        // ---- PCRE2_ZERO_TERMINATED length
        for &so in &[0usize, 3, 7, 8] {
            dfa_cmp(
                &mut p, codes, (ptr::null_mut(), ptr::null_mut()), &subj,
                PCRE2_ZERO_TERMINATED, K::new(so, 0, 4, 100), "C158 zero-terminated",
            );
        }

        // ---- partial matching + ENDANCHORED is rejected (both at compile and match time)
        for &(copts, mopts) in &[
            (0u32, PCRE2_PARTIAL_SOFT),
            (0, PCRE2_PARTIAL_HARD),
            (0, PCRE2_PARTIAL_SOFT | PCRE2_ENDANCHORED),
            (0, PCRE2_PARTIAL_HARD | PCRE2_ENDANCHORED),
            (PCRE2_ENDANCHORED, PCRE2_PARTIAL_SOFT),
            (PCRE2_ENDANCHORED, PCRE2_PARTIAL_HARD),
            (PCRE2_ENDANCHORED, 0),
            (PCRE2_ENDANCHORED, PCRE2_ANCHORED),
        ] {
            let cs = p.compile(b"abc", copts, 0, 0, 0).unwrap();
            let out = dfa_cmp(
                &mut p, cs, (ptr::null_mut(), ptr::null_mut()), &subj, 7,
                K::new(0, mopts, 4, 100), "C158 partial+endanchored",
            );
            if mopts & (PCRE2_PARTIAL_SOFT | PCRE2_PARTIAL_HARD) != 0
                && (copts | mopts) & PCRE2_ENDANCHORED != 0
            {
                assert_eq!(out.m.rc, PCRE2_ERROR_BADOPTION, "C158 partial+endanchored");
            }
            p.free_code(cs);
        }

        // ---- PCRE2_MATCH_INVALID_UTF is refused outright: the DFA has no fragment logic
        // (pcre2_dfa_match.c:3419-3420).  Checked before BADMAGIC/BADMODE.
        for pat in [&b"abc"[..], &b"a"[..], &b"(*UTF)x"[..]] {
            let cs = p
                .compile(pat, PCRE2_UTF | PCRE2_MATCH_INVALID_UTF, 0, 0, 0)
                .unwrap();
            let out = dfa_cmp(
                &mut p, cs, (ptr::null_mut(), ptr::null_mut()), &subj, 7,
                K::new(0, 0, 4, 100), "C158 MATCH_INVALID_UTF",
            );
            assert_eq!(out.m.rc, PCRE2_ERROR_DFA_UINVALID_UTF);
            // ... but a too-small workspace is still detected first.
            let out = dfa_cmp(
                &mut p, cs, (ptr::null_mut(), ptr::null_mut()), &subj, 7,
                K::new(0, 0, 4, 19), "C158 MATCH_INVALID_UTF+wssize",
            );
            assert_eq!(out.m.rc, PCRE2_ERROR_DFA_WSSIZE);
            p.free_code(cs);
        }

        // ---- offset limit needs PCRE2_USE_OFFSET_LIMIT at compile time
        for &copt in &[0u32, PCRE2_USE_OFFSET_LIMIT] {
            let cs = p.compile(b"abc", copt, 0, 0, 0).unwrap();
            for &lim in &[PCRE2_UNSET, 0usize, 1, 2, 3, 7, 100] {
                let mcc = (p.c.pcre2_match_context_create_8)(ptr::null_mut());
                let mcr = (p.r.pcre2_match_context_create_8)(ptr::null_mut());
                assert_eq!((p.c.pcre2_set_offset_limit_8)(mcc, lim), 0);
                assert_eq!((p.r.pcre2_set_offset_limit_8)(mcr, lim), 0);
                let out = dfa_cmp(
                    &mut p, cs, (mcc, mcr), &subj, 7, K::new(0, 0, 4, 100),
                    "C158 offset limit",
                );
                if lim != PCRE2_UNSET && copt == 0 {
                    assert_eq!(out.m.rc, PCRE2_ERROR_BADOFFSETLIMIT);
                }
                (p.c.pcre2_match_context_free_8)(mcc);
                (p.r.pcre2_match_context_free_8)(mcr);
            }
            p.free_code(cs);
        }

        // ---- BADMAGIC / BADMODE.  A synthetic, fully-zeroed `pcre2_real_code`-sized
        // block: `magic_number` is at byte 88, `overall_options` at 96 and `flags` at
        // 104 (pcre2_intmodedep.h:660-686, verified with offsetof).  Both libraries are
        // handed the very same bytes.
        {
            let lay = Layout::from_size_align(4096, 64).unwrap();
            let fake = alloc_zeroed(lay);
            let run = |p: &mut Pair, expect: i32, what: &str| {
                let mdc = p.md(true, 4);
                let mdr = p.md(false, 4);
                let mut wsc = vec![0i32; 100];
                let mut wsr = vec![0i32; 100];
                fill_ov(p.c, mdc, SENT);
                fill_ov(p.r, mdr, SENT);
                let rc_c = (p.c.pcre2_dfa_match_8)(
                    fake as Ptr, subj.as_ptr(), 7, 0, 0, mdc, ptr::null_mut(),
                    wsc.as_mut_ptr(), 100,
                );
                let rc_r = (p.r.pcre2_dfa_match_8)(
                    fake as Ptr, subj.as_ptr(), 7, 0, 0, mdr, ptr::null_mut(),
                    wsr.as_mut_ptr(), 100,
                );
                let a = read_match(p.c, mdc, rc_c);
                let b = read_match(p.r, mdr, rc_r);
                (p.c.pcre2_match_data_free_8)(mdc);
                (p.r.pcre2_match_data_free_8)(mdr);
                p.n += 1;
                assert_eq!(a.rc, expect, "C158 {}", what);
                assert_eq!(a, b, "C158 {}", what);
                assert_eq!(wsc, wsr, "C158 {} workspace", what);
            };
            run(&mut p, PCRE2_ERROR_BADMAGIC, "zeroed code block -> BADMAGIC");
            (fake.add(88) as *mut u32).write(0x5043_5245); // MAGIC_NUMBER 'PCRE'
            run(&mut p, PCRE2_ERROR_BADMODE, "magic ok, flags mode bits 0 -> BADMODE");
            (fake.add(104) as *mut u32).write(2); // PCRE2_MODE16
            run(&mut p, PCRE2_ERROR_BADMODE, "flags = MODE16 -> BADMODE");
            (fake.add(96) as *mut u32).write(PCRE2_MATCH_INVALID_UTF);
            run(&mut p, PCRE2_ERROR_DFA_UINVALID_UTF, "INVALID_UTF beats BADMAGIC/BADMODE");
            (fake.add(104) as *mut u32).write(1); // PCRE2_MODE8 - now only INVALID_UTF left
            run(&mut p, PCRE2_ERROR_DFA_UINVALID_UTF, "INVALID_UTF with correct mode");
            dealloc(fake, lay);
        }

        // ---- randomized validation: the check ORDER matters, so randomise all the
        // inputs that participate in it at once.
        let mut rng = Rng::new(0xC158_0001);
        for _ in 0..600 {
            let copt = *rng.pick(&[
                0u32,
                PCRE2_ENDANCHORED,
                PCRE2_UTF,
                PCRE2_UTF | PCRE2_MATCH_INVALID_UTF,
                PCRE2_USE_OFFSET_LIMIT,
                PCRE2_ANCHORED,
            ]);
            let pat: &[u8] = rng.pick(&[&b"abc"[..], &b"a*"[..], &b"(a|b)+"[..], &b""[..]]);
            let Some(cs) = p.compile(pat, copt, 0, 0, 0) else { continue };
            let slen = rng.below(9) as usize;
            let sb: Vec<u8> = (0..slen + 8).map(|_| *rng.pick(b"abc\r\n\0")).collect();
            let so = rng.below(12) as usize;
            let wsc = *rng.pick(&[0usize, 1, 5, 19, 20, 21, 60, 300]);
            let mut opts = 0u32;
            for _ in 0..rng.below(4) {
                opts |= 1u32 << rng.below(32);
            }
            let ovn = *rng.pick(&[0u32, 1, 4]);
            let len = if rng.below(8) == 0 { PCRE2_ZERO_TERMINATED } else { slen };
            let mdc = p.md(true, ovn);
            let mdr = p.md(false, ovn);
            let mut wc = vec![0i32; wsbuf_for(wsc)];
            let mut wr = vec![0i32; wsbuf_for(wsc)];
            let a = dfa_one(
                p.c, mdc, cs.0, sb.as_ptr(), len, so, opts, ptr::null_mut(), &mut wc, wsc, true,
            );
            let b = dfa_one(
                p.r, mdr, cs.1, sb.as_ptr(), len, so, opts, ptr::null_mut(), &mut wr, wsc, true,
            );
            (p.c.pcre2_match_data_free_8)(mdc);
            (p.r.pcre2_match_data_free_8)(mdr);
            p.n += 1;
            assert_eq!(
                a, b,
                "C158 random validation pat={:?} copt={:#x} len={} so={} wsc={} opts={:#x}",
                pat, copt, len as i64, so, wsc, opts
            );
            p.free_code(cs);
        }

        p.free_code(codes);
    }
    println!("C158: {} comparisons", p.n);
    assert!(p.n > 600);
}

// ---------------------------------------------------------------------------------
// C159 - workspace sizing: the exact PCRE2_ERROR_DFA_WSSIZE threshold
// ---------------------------------------------------------------------------------

/// Patterns of increasing "simultaneous alternative" complexity plus a subject.
const WS_CASES: &[(&str, &str)] = &[
    ("a", "aaaa"),
    ("abc", "xxabcxx"),
    ("a*", "aaaaaaaaaa"),
    ("a{0,100}", "aaaaaaaaaa"),
    ("(a|b)*", "abababab"),
    ("(a|b|c|d)*", "abcdabcdabcd"),
    ("(a|b|c|d|e|f|g|h){20}", "abcdefghabcdefghabcd"),
    ("[a-z]{50}", "abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwx"),
    ("(a|b|c|d|e|f|g|h|i|j)+z", "abcdefghijabcdefghijz"),
    ("(?:ab|abc|abcd|abcde)+", "ababcabcdabcde"),
    ("\\((?:[^()]|(?R))*\\)", "((()))"),
    ("(a+|b+|c+){1,10}", "aaabbbccc"),
    ("(?:a|b){1,30}c", "ababababababababababababababc"),
    ("(x|y|(a|b|c)+|z){5}", "abcabcabcabcabc"),
    ("^(?:.|\\n){0,20}$", "hello\nworld"),
];

#[test]
fn c159_workspace_threshold() {
    println!("C159 pcre2_dfa_match_8 workspace sizing / DFA_WSSIZE threshold");
    let mut p = Pair::new();
    unsafe {
        for &(pat, subj) in WS_CASES {
            let codes = p.compile(pat.as_bytes(), 0, 0, 0, 0).unwrap();
            let sb = subj.as_bytes().to_vec();

            // A dense sweep: at every wscount both libraries must agree on rc, on the
            // whole ovector, and on the workspace they leave behind.
            let mut first_ok: Option<usize> = None;
            for wsc in 0..=260usize {
                let out = dfa_cmp(
                    &mut p, codes, (ptr::null_mut(), ptr::null_mut()), &sb, sb.len(),
                    K::new(0, 0, 8, wsc), "C159 wscount sweep",
                );
                if out.m.rc != PCRE2_ERROR_DFA_WSSIZE && first_ok.is_none() {
                    first_ok = Some(wsc);
                }
                if wsc < 20 {
                    assert_eq!(out.m.rc, PCRE2_ERROR_DFA_WSSIZE);
                }
            }
            // ... and the *threshold itself* must be identical.  Recompute it
            // independently in each library and compare the two numbers.
            let thresh = |api: &Api, code: Ptr, g: Ptr| -> Option<usize> {
                for wsc in 20..=4000usize {
                    let md = (api.pcre2_match_data_create_8)(8, g);
                    let mut ws = vec![0i32; wsbuf_for(wsc)];
                    let o = dfa_one(
                        api, md, code, sb.as_ptr(), sb.len(), 0, 0, ptr::null_mut(),
                        &mut ws, wsc, true,
                    );
                    (api.pcre2_match_data_free_8)(md);
                    if o.m.rc != PCRE2_ERROR_DFA_WSSIZE {
                        return Some(wsc);
                    }
                }
                None
            };
            let tc = thresh(p.c, codes.0, p.gc);
            let tr = thresh(p.r, codes.1, p.gr);
            p.n += 1;
            assert_eq!(
                tc, tr,
                "C159 smallest working wscount differs for {:?} on {:?}: C={:?} RUST={:?}",
                pat, subj, tc, tr
            );
            match tc {
                Some(t) if t <= 260 => {
                    assert_eq!(first_ok, Some(t), "C159 sweep vs threshold for {:?}", pat)
                }
                _ => assert_eq!(first_ok, None, "C159 sweep vs threshold for {:?}", pat),
            }
            assert!(tc.is_some(), "C159 no workable wscount for {:?}", pat);

            // Long subject with a generous and with a minimal workspace.
            let long: Vec<u8> = subj.as_bytes().iter().cycle().take(400).copied().collect();
            for &wsc in &[20usize, 21, 23, 26, 32, 50, 68, 100, 1000, 20000] {
                dfa_cmp(
                    &mut p, codes, (ptr::null_mut(), ptr::null_mut()), &long, long.len(),
                    K::new(0, 0, 8, wsc), "C159 long subject",
                );
            }
            p.free_code(codes);
        }
    }
    println!("C159: {} comparisons", p.n);
    assert!(p.n > 4000);
}

// ---------------------------------------------------------------------------------
// C160 - PCRE2_DFA_RESTART
// ---------------------------------------------------------------------------------

#[test]
fn c160_dfa_restart() {
    println!("C160 pcre2_dfa_match_8 PCRE2_DFA_RESTART");
    let mut p = Pair::new();
    unsafe {
        // ---- Two-stage partial match / restart.  The workspace contents left behind by
        // stage 1 are exactly what stage 2 reads back (pcre2_dfa_match.c:668-675), so
        // they are compared byte-for-byte at both stages.
        let cases: &[(&str, &str, &str)] = &[
            ("abcd", "xxab", "cd"),
            ("abcd", "xxab", "xy"),
            ("abcd", "a", "bcd"),
            ("a+b", "aaa", "ab"),
            ("(?:ab)+c", "abab", "abc"),
            ("\\d{4}-\\d{2}", "12", "34-56"),
            ("[a-z]+X", "abc", "defX"),
            ("^\\w+@\\w+", "user", "@host"),
            ("(a|ab|abc)d", "ab", "cd"),
            ("a\\r\\nb", "a\r", "\nb"),
            ("xyz", "xy", ""),
            ("a*b", "", "aab"),
            ("(?<=a)bcd", "ab", "cd"),
            ("(?:a|b)+z", "abab", "abz"),
        ];
        for &(pat, s1, s2) in cases {
            for &copts in &[0u32, PCRE2_MULTILINE, PCRE2_CASELESS] {
                let codes = p.compile(pat.as_bytes(), copts, 0, 0, 0).unwrap();
                for &wsc in &[20usize, 21, 30, 100, 1000] {
                    for &phard in &[PCRE2_PARTIAL_SOFT, PCRE2_PARTIAL_HARD] {
                        let b1 = s1.as_bytes().to_vec();
                        let b2 = s2.as_bytes().to_vec();
                        let mdc = p.md(true, 6);
                        let mdr = p.md(false, 6);
                        let mut wc = vec![0i32; wsbuf_for(wsc)];
                        let mut wr = vec![0i32; wsbuf_for(wsc)];
                        let a1 = dfa_one(
                            p.c, mdc, codes.0, b1.as_ptr(), b1.len(), 0, phard,
                            ptr::null_mut(), &mut wc, wsc, true,
                        );
                        let r1 = dfa_one(
                            p.r, mdr, codes.1, b1.as_ptr(), b1.len(), 0, phard,
                            ptr::null_mut(), &mut wr, wsc, true,
                        );
                        p.n += 1;
                        assert_eq!(
                            a1, r1,
                            "C160 stage1 pat={:?} s1={:?} wsc={} phard={:#x}",
                            pat, s1, wsc, phard
                        );
                        // stage 2: restart over the continuation, same workspace
                        for &extra in &[0u32, PCRE2_PARTIAL_SOFT, PCRE2_DFA_SHORTEST] {
                            let mut wc2 = wc.clone();
                            let mut wr2 = wr.clone();
                            let a2 = dfa_one(
                                p.c, mdc, codes.0, b2.as_ptr(), b2.len(), 0,
                                PCRE2_DFA_RESTART | extra, ptr::null_mut(), &mut wc2, wsc,
                                true,
                            );
                            let r2 = dfa_one(
                                p.r, mdr, codes.1, b2.as_ptr(), b2.len(), 0,
                                PCRE2_DFA_RESTART | extra, ptr::null_mut(), &mut wr2, wsc,
                                true,
                            );
                            p.n += 1;
                            assert_eq!(
                                a2, r2,
                                "C160 stage2 pat={:?} s1={:?} s2={:?} wsc={} extra={:#x}",
                                pat, s1, s2, wsc, extra
                            );
                        }
                        (p.c.pcre2_match_data_free_8)(mdc);
                        (p.r.pcre2_match_data_free_8)(mdr);
                    }
                }
                p.free_code(codes);
            }
        }

        // ---- The restart sanity check (pcre2_dfa_match.c:3454-3459):
        //      (workspace[0] & -2) == 0  &&  1 <= workspace[1] <= (wscount-2)/3
        let codes = p.compile(b"abcd", 0, 0, 0, 0).unwrap();
        let subj = b"cd".to_vec();
        for &wsc in &[20usize, 21, 22, 23, 100, 1000] {
            let maxst = (wsc - 2) / INTS_PER_STATEBLOCK;
            // The number of state blocks one of the two vectors can actually hold
            // (pcre2_dfa_match.c:570-572).  Note this is only about *half* of `maxst`:
            // for `1 + safe <= workspace[1] <= maxst` the restart sanity check passes but
            // the `memcpy(new_states, active_states, ...)` at :672 then writes past
            // `wscount`.  That is undefined behaviour in the C, so those values are not
            // exercised here (see the report).
            let safe = ((wsc - 2) - ((wsc - 2) % (2 * INTS_PER_STATEBLOCK)))
                / (2 * INTS_PER_STATEBLOCK);
            let w0s: [i32; 7] = [0, 1, 2, 3, -1, -2, 0x4000_0000];
            let mut w1s: Vec<i32> = vec![-1, 0, 1, maxst as i32 + 1, maxst as i32 + 100];
            if safe >= 2 {
                w1s.push(2);
            }
            w1s.push(safe as i32);
            for &w0 in &w0s {
                for &w1 in &w1s {
                    let mdc = p.md(true, 4);
                    let mdr = p.md(false, 4);
                    let mut wc = vec![0i32; wsbuf_for(wsc)];
                    let mut wr = vec![0i32; wsbuf_for(wsc)];
                    wc[0] = w0;
                    wc[1] = w1;
                    wr[0] = w0;
                    wr[1] = w1;
                    let a = dfa_one(
                        p.c, mdc, codes.0, subj.as_ptr(), 2, 0, PCRE2_DFA_RESTART,
                        ptr::null_mut(), &mut wc, wsc, true,
                    );
                    let b = dfa_one(
                        p.r, mdr, codes.1, subj.as_ptr(), 2, 0, PCRE2_DFA_RESTART,
                        ptr::null_mut(), &mut wr, wsc, true,
                    );
                    (p.c.pcre2_match_data_free_8)(mdc);
                    (p.r.pcre2_match_data_free_8)(mdr);
                    p.n += 1;
                    assert_eq!(
                        a, b,
                        "C160 restart sanity wsc={} ws0={} ws1={}",
                        wsc, w0, w1
                    );
                    let ok0 = (w0 as i64 & -2i64) == 0;
                    let ok1 = w1 >= 1 && w1 <= maxst as i32;
                    if !(ok0 && ok1) {
                        assert_eq!(
                            a.m.rc, PCRE2_ERROR_DFA_BADRESTART,
                            "C160 wsc={} ws0={} ws1={} should be BADRESTART",
                            wsc, w0, w1
                        );
                    } else {
                        assert_ne!(a.m.rc, PCRE2_ERROR_DFA_BADRESTART);
                    }
                }
            }
        }
        // A completely fresh, zeroed workspace with RESTART -> BADRESTART (workspace[1]<1)
        for &wsc in &[20usize, 100] {
            let out = dfa_cmp(
                &mut p, codes, (ptr::null_mut(), ptr::null_mut()), &subj, 2,
                K::new(0, PCRE2_DFA_RESTART, 4, wsc), "C160 fresh workspace restart",
            );
            assert_eq!(out.m.rc, PCRE2_ERROR_DFA_BADRESTART);
        }
        // Restart forces anchored mode / skips the start optimizations - so a restart at
        // a non-zero start offset behaves like an anchored match there.
        p.free_code(codes);

        // ---- restart data produced by a *different* pattern.  The C detects some of
        // these; whatever it does, the Rust must do the same.  The second pattern is
        // always the longer one so that the recorded state offsets stay inside its code.
        let pairs: &[(&str, &str)] = &[
            ("abc", "(?:xyzw|pqrs|hello|world)+ABCDEFGHIJKLMNOP"),
            ("ab", "[a-z]{1,20}[0-9]{1,20}QQQQ"),
            ("a+", "(?:aa|bb|cc|dd|ee|ff)+ZZZZZZZZ"),
            ("xy", "(?:one|two|three|four|five)+WWWWWWWW"),
        ];
        for &(pa, pb) in pairs {
            let ca = p.compile(pa.as_bytes(), 0, 0, 0, 0).unwrap();
            let cb = p.compile(pb.as_bytes(), 0, 0, 0, 0).unwrap();
            let s1 = format!("zz{}", pa.trim_end_matches(['+', '*']));
            let b1 = s1.as_bytes().to_vec();
            let b2 = b"cdefgh".to_vec();
            for &wsc in &[100usize, 1000] {
                let mdc = p.md(true, 6);
                let mdr = p.md(false, 6);
                let mut wc = vec![0i32; wsbuf_for(wsc)];
                let mut wr = vec![0i32; wsbuf_for(wsc)];
                let a1 = dfa_one(
                    p.c, mdc, ca.0, b1.as_ptr(), b1.len(), 0, PCRE2_PARTIAL_SOFT,
                    ptr::null_mut(), &mut wc, wsc, true,
                );
                let r1 = dfa_one(
                    p.r, mdr, ca.1, b1.as_ptr(), b1.len(), 0, PCRE2_PARTIAL_SOFT,
                    ptr::null_mut(), &mut wr, wsc, true,
                );
                p.n += 1;
                assert_eq!(a1, r1, "C160 cross-pattern stage1 {:?}", pa);
                let a2 = dfa_one(
                    p.c, mdc, cb.0, b2.as_ptr(), b2.len(), 0, PCRE2_DFA_RESTART,
                    ptr::null_mut(), &mut wc, wsc, true,
                );
                let r2 = dfa_one(
                    p.r, mdr, cb.1, b2.as_ptr(), b2.len(), 0, PCRE2_DFA_RESTART,
                    ptr::null_mut(), &mut wr, wsc, true,
                );
                p.n += 1;
                assert_eq!(
                    a2, r2,
                    "C160 cross-pattern restart {:?} -> {:?}",
                    pa, pb
                );
                (p.c.pcre2_match_data_free_8)(mdc);
                (p.r.pcre2_match_data_free_8)(mdr);
            }
            p.free_code(ca);
            p.free_code(cb);
        }

        // ---- multi-stage restart: feed a subject one code unit at a time.
        for pat in ["abcdef", "(?:ab|cd|ef)+", "a.c.e", "[a-f]{6}", "\\w+\\d"] {
            let codes = p.compile(pat.as_bytes(), 0, 0, 0, 0).unwrap();
            let full = b"abcde1".to_vec();
            for &wsc in &[20usize, 30, 100, 1000] {
                let mdc = p.md(true, 6);
                let mdr = p.md(false, 6);
                let mut wc = vec![0i32; wsbuf_for(wsc)];
                let mut wr = vec![0i32; wsbuf_for(wsc)];
                let mut opts = PCRE2_PARTIAL_SOFT;
                for i in 0..full.len() {
                    let chunk = full[i..i + 1].to_vec();
                    let a = dfa_one(
                        p.c, mdc, codes.0, chunk.as_ptr(), 1, 0, opts, ptr::null_mut(),
                        &mut wc, wsc, true,
                    );
                    let b = dfa_one(
                        p.r, mdr, codes.1, chunk.as_ptr(), 1, 0, opts, ptr::null_mut(),
                        &mut wr, wsc, true,
                    );
                    p.n += 1;
                    assert_eq!(
                        a, b,
                        "C160 incremental pat={:?} step={} wsc={}",
                        pat, i, wsc
                    );
                    if a.m.rc != PCRE2_ERROR_PARTIAL {
                        break;
                    }
                    opts = PCRE2_PARTIAL_SOFT | PCRE2_DFA_RESTART;
                }
                (p.c.pcre2_match_data_free_8)(mdc);
                (p.r.pcre2_match_data_free_8)(mdr);
            }
            p.free_code(codes);
        }
    }
    println!("C160: {} comparisons", p.n);
    assert!(p.n > 1000);
}

// ---------------------------------------------------------------------------------
// C161 - PCRE2_DFA_SHORTEST
// ---------------------------------------------------------------------------------

#[test]
fn c161_dfa_shortest() {
    println!("C161 pcre2_dfa_match_8 PCRE2_DFA_SHORTEST");
    let mut p = Pair::new();
    unsafe {
        let pats = [
            "a+", "ab|abcd", "a*", "(?:ab)+", "a|ab|abc|abcd", "a{1,4}", "[ab]+",
            "a+?", "(?:a|aa|aaa)+", "\\w+", ".*", ".+", "(a)(b)?(c)?", "x*",
        ];
        let subjs: [&[u8]; 10] = [
            b"", b"a", b"aa", b"aaaa", b"ab", b"abcd", b"abababab", b"xaaay",
            b"aaaaaaaaaaaaaaaaaaaa", b"abcdabcd",
        ];
        for pat in pats {
            for &copts in &[0u32, PCRE2_UNGREEDY, PCRE2_ANCHORED] {
                let Some(codes) = p.compile(pat.as_bytes(), copts, 0, 0, 0) else { continue };
                for s in subjs {
                    let sb = s.to_vec();
                    for &short in &[0u32, PCRE2_DFA_SHORTEST] {
                        for &ovn in &[1u32, 2, 4, 10] {
                            for so in 0..=sb.len() {
                                dfa_cmp(
                                    &mut p, codes, (ptr::null_mut(), ptr::null_mut()),
                                    &sb, sb.len(), K::new(so, short, ovn, 200),
                                    "C161 shortest",
                                );
                            }
                        }
                        // combined with the other match-time flags
                        for &extra in &[
                            PCRE2_ANCHORED,
                            PCRE2_NOTEMPTY,
                            PCRE2_NOTEMPTY_ATSTART,
                            PCRE2_ENDANCHORED,
                            PCRE2_NOTBOL | PCRE2_NOTEOL,
                        ] {
                            dfa_cmp(
                                &mut p, codes, (ptr::null_mut(), ptr::null_mut()), &sb,
                                sb.len(), K::new(0, short | extra, 4, 200),
                                "C161 shortest+extra",
                            );
                        }
                    }
                }
                p.free_code(codes);
            }
        }
    }
    println!("C161: {} comparisons", p.n);
    assert!(p.n > 2000);
}

// ---------------------------------------------------------------------------------
// C162 - the multi-match ovector semantics
// ---------------------------------------------------------------------------------

#[test]
fn c162_ovector_semantics() {
    println!("C162 pcre2_dfa_match_8 multi-match ovector semantics");
    let mut p = Pair::new();
    unsafe {
        // Patterns with several distinct match lengths at the same starting offset.
        let pats = [
            "a|ab|abc",
            "(?:a){1,3}",
            "a{1,5}",
            "ab|abcd",
            "a*",
            "a+",
            "(?:a|aa|aaa|aaaa|aaaaa)",
            "x|xy|xyz|xyzw|xyzwv",
            "\\w|\\w\\w|\\w\\w\\w",
            "(?:ab)+",
            "a(?:b|bc|bcd)?",
            "[a-c]{1,6}",
            "(a)|(ab)|(abc)",
            "(?:a|ab|abc|abcd|abcde|abcdef|abcdefg|abcdefgh)",
        ];
        let subjs: [&[u8]; 8] = [
            b"abc", b"aaaaa", b"xyzwv", b"abcdefgh", b"a", b"", b"ababab", b"aaaaaaaaaa",
        ];
        for pat in pats {
            let codes = p.compile(pat.as_bytes(), 0, 0, 0, 0).unwrap();
            for s in subjs {
                let sb = s.to_vec();
                // oveccount 0 is clamped to 1 by pcre2_match_data_create; sweep the
                // interesting sizes plus odd ones (offsetcount is rounded DOWN to a
                // whole number of pairs - pcre2_dfa_match.c:568).
                for &ovn in &[0u32, 1, 2, 3, 4, 5, 6, 10, 17] {
                    for &opts in &[0u32, PCRE2_DFA_SHORTEST, PCRE2_ANCHORED] {
                        let out = dfa_cmp(
                            &mut p, codes, (ptr::null_mut(), ptr::null_mut()), &sb,
                            sb.len(), K::new(0, opts, ovn, 200), "C162 ovector",
                        );
                        // The pairs must come back longest-first when there is more than
                        // one, and rc==0 signals "overflowed, longest ones present".
                        if out.m.rc > 1 {
                            let n = out.m.rc as usize;
                            for i in 1..n {
                                let prev = out.m.ovector[2 * i - 1] - out.m.ovector[2 * i - 2];
                                let cur = out.m.ovector[2 * i + 1] - out.m.ovector[2 * i];
                                assert!(
                                    prev >= cur,
                                    "C162 not longest-first: pat={:?} subj={:?} ov={:?}",
                                    pat, s, out.m.ovector
                                );
                            }
                        }
                    }
                }
            }
            p.free_code(codes);
        }
    }
    println!("C162: {} comparisons", p.n);
    assert!(p.n > 2000);
}

// ---------------------------------------------------------------------------------
// C163 - unsupported items (PCRE2_ERROR_DFA_UITEM / DFA_RECURSE), callouts
// ---------------------------------------------------------------------------------

/// (pattern, compile options, extra options, subject) - one entry per distinct
/// unsupported-opcode site reachable from `pcre2_dfa_match`.
const UITEM_CASES: &[(&str, u32, u32, &str)] = &[
    // ---- back references: OP_REF / OP_REFI / OP_DNREF (switch default, :3259)
    ("(a)\\1", 0, 0, "aa"),
    ("(?i)(a)\\1", 0, 0, "aA"),
    ("(?J)(?<n>a)|(?<n>b)\\k<n>", 0, 0, "bb"),
    ("(a)(b)\\2\\1", 0, 0, "abba"),
    ("(a+)\\1", 0, 0, "aaaa"),
    ("(?<x>a)\\k<x>", 0, 0, "aa"),
    ("(a)\\g{-1}", 0, 0, "aa"),
    // ---- backtracking verbs
    ("a(*SKIP)b", 0, 0, "ab"),
    ("a(*SKIP:x)b", 0, 0, "ab"),
    ("a(*THEN)b", 0, 0, "ab"),
    ("a(*PRUNE)b", 0, 0, "ab"),
    ("a(*PRUNE:y)b", 0, 0, "ab"),
    ("a(*COMMIT)b", 0, 0, "ab"),
    ("a(*ACCEPT)b", 0, 0, "ab"),
    ("a(*MARK:x)b", 0, 0, "ab"),
    ("(*MARK:z)a", 0, 0, "a"),
    // ---- \K -> OP_SET_SOM
    ("a\\Kb", 0, 0, "ab"),
    ("\\Kabc", 0, 0, "abc"),
    ("(?=b\\K)", 0, PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK, "b"),
    ("(?<=\\Ka)", 0, PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK, "ab"),
    // ---- OP_SCRIPT_RUN / OP_ASSERT_SCS
    ("(*sr:\\w+)", 0, 0, "ab"),
    ("(*asr:\\w+)", 0, 0, "ab"),
    ("(a)(*scs:(1)b)", 0, 0, "ab"),
    // ---- \C as a *quantified* type: the dedicated site at :825
    ("\\C+", PCRE2_UTF, 0, "ab"),
    ("\\C*", PCRE2_UTF, 0, "ab"),
    ("\\C{2,}", PCRE2_UTF, 0, "abc"),
    ("\\C+?", PCRE2_UTF, 0, "ab"),
    ("\\C", PCRE2_UTF, 0, "ab"),
    ("\\C+", 0, 0, "ab"), // ... supported when not in UTF mode
    ("\\C", 0, 0, "ab"),
    // ---- recursion with an argument list: OP_CREF after OP_RECURSE (:2943)
    ("(a)(b)(?1(2))", 0, 0, "aba"),
    ("(a)(?1(1))", 0, 0, "aa"),
    ("(?1(1))(a)", 0, 0, "aa"),
    ("(a)(?2(1))(b)", 0, 0, "abb"),
    ("(?&x(1))(?<x>a)", 0, 0, "aa"),
    ("(a)(?1(1))", 0, 0, ""),
    // ---- supported items, for contrast
    ("(*FAIL)", 0, 0, "ab"),
    ("(*F)", 0, 0, "ab"),
    ("(?!)", 0, 0, "ab"),
    ("(*pla:a)b", 0, 0, "ab"),
    ("(*plb:a)b", 0, 0, "ab"),
    ("(*nla:a)b", 0, 0, "cb"),
    ("(*nlb:a)b", 0, 0, "cb"),
    ("(?<=a|bb)c", 0, 0, "bbc"),
    ("(?1)(a)", 0, 0, "aa"),
    ("(?R)?z", 0, 0, "zz"),
    ("(?>a+)b", 0, 0, "aab"),
    ("(a|b)++c", 0, 0, "abc"),
    ("(?=a)b", 0, 0, "ab"),
    ("(?(?=a)b|c)", 0, 0, "ab"),
    ("(?(DEFINE)(?<x>a))(?&x)", 0, 0, "a"),
    ("\\X+", 0, 0, "abc"),
    ("(?=a(*ACCEPT))b", 0, 0, "ab"),
];

#[test]
fn c163_unsupported_items() {
    println!("C163 pcre2_dfa_match_8 unsupported items (DFA_UITEM / DFA_RECURSE)");
    let mut p = Pair::new();
    let mut seen_uitem = 0usize;
    unsafe {
        for &(pat, copts, xopts, subj) in UITEM_CASES {
            let Some(codes) = p.compile(pat.as_bytes(), copts, xopts, 0, 0) else {
                panic!("C163 pattern {:?} did not compile in either library", pat)
            };
            let sb = subj.as_bytes().to_vec();
            for &opts in &[
                0u32,
                PCRE2_ANCHORED,
                PCRE2_DFA_SHORTEST,
                PCRE2_PARTIAL_SOFT,
                PCRE2_NOTEMPTY,
            ] {
                for so in 0..=sb.len() {
                    for &ovn in &[1u32, 4] {
                        let out = dfa_cmp(
                            &mut p, codes, (ptr::null_mut(), ptr::null_mut()), &sb,
                            sb.len(), K::new(so, opts, ovn, 500), "C163 unsupported item",
                        );
                        if out.m.rc == PCRE2_ERROR_DFA_UITEM {
                            seen_uitem += 1;
                        }
                    }
                }
            }
            p.free_code(codes);
        }
        assert!(
            seen_uitem > 40,
            "C163 only saw {} DFA_UITEM results - the unsupported sites are not being hit",
            seen_uitem
        );

        // ---- PCRE2_ERROR_DFA_RECURSE (-39): the recursion's private offsets vector
        // (500 pairs) overflows (pcre2_dfa_match.c:2995).
        let mut seen_recurse = 0usize;
        for pat in ["(a*)(?1)", "(?1)(a*)", "(a*)(?1)b", "(a{0,600})(?1)"] {
            let codes = p.compile(pat.as_bytes(), 0, 0, 0, 0).unwrap();
            for n in [10usize, 100, 400, 499, 500, 501, 600, 700] {
                let sb = vec![b'a'; n];
                let out = dfa_cmp(
                    &mut p, codes, (ptr::null_mut(), ptr::null_mut()), &sb, sb.len(),
                    K::new(0, 0, 4, 4000), "C163 DFA_RECURSE",
                );
                if out.m.rc == PCRE2_ERROR_DFA_RECURSE {
                    seen_recurse += 1;
                }
            }
            p.free_code(codes);
        }
        assert!(
            seen_recurse > 0,
            "C163 never reached PCRE2_ERROR_DFA_RECURSE"
        );

        // ---- callouts: do_callout_dfa propagates a negative return value verbatim and
        // treats a positive one as "fail this thread" (pcre2_dfa_match.c:3244-3253,
        // 2838-2848 for the condition site).
        for pat in [
            "a(?C1)b",
            "(?C0)abc",
            "a(?C{X})b",
            "(?(?C7)(?=a)b|c)",
            "a(?C2)b|a(?C3)c",
            "(?C9)",
            "a(?C1)b(?C2)c",
        ] {
            for &acopt in &[0u32, PCRE2_AUTO_CALLOUT] {
                let Some(codes) = p.compile(pat.as_bytes(), acopt, 0, 0, 0) else { continue };
                for &ret in &[0i32, 1, 2, -1, -2, -12345, PCRE2_ERROR_NOMEMORY] {
                    for subj in ["abc", "ac", "a", "", "xabc"] {
                        let sb = subj.as_bytes().to_vec();
                        let (a, ta) = callout_run(p.c, p.gc, codes.0, &sb, ret);
                        let (b, tb) = callout_run(p.r, p.gr, codes.1, &sb, ret);
                        p.n += 1;
                        assert_eq!(
                            (a, &ta),
                            (b, &tb),
                            "C163 callout pat={:?} ret={} subj={:?}",
                            pat, ret, subj
                        );
                    }
                }
                p.free_code(codes);
            }
        }
    }
    println!("C163: {} comparisons", p.n);
    assert!(p.n > 1500);
}

/// The public `pcre2_callout_block` (pcre2.h:581-602).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct CalloutBlock {
    version: u32,
    callout_number: u32,
    capture_top: u32,
    capture_last: u32,
    offset_vector: *mut usize,
    mark: *const u8,
    subject: *const u8,
    subject_length: usize,
    start_match: usize,
    current_position: usize,
    pattern_position: usize,
    next_item_length: usize,
    callout_string_offset: usize,
    callout_string_length: usize,
    callout_string: *const u8,
    callout_flags: u32,
}

/// The fields of a callout block that are comparable across libraries (no pointers).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CalloutRec {
    version: u32,
    number: u32,
    capture_top: u32,
    capture_last: u32,
    subject_length: usize,
    start_match: usize,
    current_position: usize,
    pattern_position: usize,
    next_item_length: usize,
    cs_offset: usize,
    cs_length: usize,
    flags: u32,
    mark_null: bool,
    cs_null: bool,
}

struct CalloutState {
    ret: i32,
    trace: Vec<CalloutRec>,
}

unsafe extern "C" fn callout_cb(blk: Ptr, data: Ptr) -> i32 {
    let b = &*(blk as *const CalloutBlock);
    let st = &mut *(data as *mut CalloutState);
    st.trace.push(CalloutRec {
        version: b.version,
        number: b.callout_number,
        capture_top: b.capture_top,
        capture_last: b.capture_last,
        subject_length: b.subject_length,
        start_match: b.start_match,
        current_position: b.current_position,
        pattern_position: b.pattern_position,
        next_item_length: b.next_item_length,
        cs_offset: b.callout_string_offset,
        cs_length: b.callout_string_length,
        flags: b.callout_flags,
        mark_null: b.mark.is_null(),
        cs_null: b.callout_string.is_null(),
    });
    st.ret
}

unsafe fn callout_run(
    api: &Api,
    g: Ptr,
    code: Ptr,
    subj: &[u8],
    ret: i32,
) -> (MatchOut, Vec<CalloutRec>) {
    let mut st = CalloutState { ret, trace: vec![] };
    let mc = (api.pcre2_match_context_create_8)(ptr::null_mut());
    assert!(!mc.is_null());
    assert_eq!(
        (api.pcre2_set_callout_8)(mc, Some(callout_cb), &mut st as *mut CalloutState as Ptr),
        0
    );
    let md = (api.pcre2_match_data_create_8)(4, g);
    let mut ws = vec![0i32; 500];
    let o = dfa_one(api, md, code, subj.as_ptr(), subj.len(), 0, 0, mc, &mut ws, 500, true);
    (api.pcre2_match_data_free_8)(md);
    (api.pcre2_match_context_free_8)(mc);
    (o.m, st.trace)
}

// ---------------------------------------------------------------------------------
// C164 - unsupported conditions (PCRE2_ERROR_DFA_UCOND)
// ---------------------------------------------------------------------------------

#[test]
fn c164_unsupported_conditions() {
    println!("C164 pcre2_dfa_match_8 unsupported conditions (DFA_UCOND)");
    let mut p = Pair::new();
    let mut seen = 0usize;
    unsafe {
        // (pattern, subject) - the two UCOND sites are :2857 (CREF/DNCREF/DNRREF) and
        // :2877 (OP_RREF with a value other than RREF_ANY).
        let cases: &[(&str, &str)] = &[
            // OP_CREF
            ("(a)(?(1)b|c)", "ab"),
            ("(a)?(?(1)b|c)", "c"),
            ("(a)(?(+1)b|c)(d)", "abd"),
            ("(a)(?(-1)b|c)", "ab"),
            // OP_DNCREF
            ("(?<n>a)(?(<n>)b|c)", "ab"),
            ("(?<n>a)(?('n')b|c)", "ab"),
            ("(?J)(?<n>a)|(?<n>b)(?(<n>)x|y)", "by"),
            // OP_DNRREF
            ("(?<n>a)(?(R&n)b|c)", "ab"),
            ("(?<n>a(?(R&n)b|c))(?&n)", "acab"),
            // OP_RREF with a specific number
            ("(a)(?(R1)b|c)", "ac"),
            ("(a)(?(R2)b|c)(d)", "acd"),
            ("(a(?(R1)b|c))(?1)", "acab"),
            // supported: RREF_ANY
            ("(a)(?(R)b|c)", "ac"),
            ("(a(?(R)b|c))(?1)", "acab"),
            // supported: assertion conditions, DEFINE, always-true/false
            ("(?(?=a)b|c)", "ab"),
            ("(?(?!a)b|c)", "cb"),
            ("(?(?<=a)b|c)", "ab"),
            ("(?(?<!a)b|c)", "cb"),
            ("(?(DEFINE)(?<x>a))b", "b"),
            ("(?(VERSION>=10)a|b)", "a"),
        ];
        for &(pat, subj) in cases {
            let Some(codes) = p.compile(pat.as_bytes(), 0, 0, 0, 0) else {
                panic!("C164 {:?} did not compile", pat)
            };
            let sb = subj.as_bytes().to_vec();
            for &opts in &[0u32, PCRE2_ANCHORED, PCRE2_DFA_SHORTEST, PCRE2_PARTIAL_SOFT] {
                for so in 0..=sb.len() {
                    let out = dfa_cmp(
                        &mut p, codes, (ptr::null_mut(), ptr::null_mut()), &sb, sb.len(),
                        K::new(so, opts, 6, 500), "C164 condition",
                    );
                    if out.m.rc == PCRE2_ERROR_DFA_UCOND {
                        seen += 1;
                    }
                }
            }
            p.free_code(codes);
        }
        assert!(seen > 10, "C164 only {} DFA_UCOND results", seen);
    }
    println!("C164: {} comparisons ({} UCOND)", p.n, seen);
    assert!(p.n > 200);
}

// ---------------------------------------------------------------------------------
// C165 - match / depth / heap limits and more_workspace()
// ---------------------------------------------------------------------------------

#[test]
fn c165_dfa_limits() {
    println!("C165 pcre2_dfa_match_8 match/depth/heap limits, more_workspace");
    let mut p = Pair::new();
    unsafe {
        // Patterns that force nested internal_dfa_match() calls.
        let cases: &[(&str, &str)] = &[
            ("(?=a)b", "ab"),
            ("(?<=a)b", "ab"),
            ("(?(?=a)b|c)", "ab"),
            ("(?(?<=a)b|c)", "ab"),
            ("\\((?:[^()]|(?R))*\\)", "((((()))))"),
            ("\\((?:[^()]|(?R))*\\)", "(((((((((())))))))))"),
            ("(?>a+)b", "aaaaaaaaab"),
            ("(a|b)++c", "ababababc"),
            ("(?:(?=a)a)+b", "aaaaab"),
            ("(?:a(?=a))*ab", "aaaab"),
            ("(?R)?a", "aaaa"),
            ("(?1)|(a(?1)?)", "aaaa"),
        ];
        let mlimits = [0u32, 1, 2, 3, 5, 10, 50, 200, 1000, 10_000, 1_000_000];
        let dlimits = [0u32, 1, 2, 3, 4, 5, 10, 100, 10_000];
        let hlimits = [0u32, 1, 2, 3, 10, 30, 60, 100, 1000, 100_000];
        for &(pat, subj) in cases {
            let codes = p.compile(pat.as_bytes(), 0, 0, 0, 0).unwrap();
            let sb = subj.as_bytes().to_vec();

            let mcc = (p.c.pcre2_match_context_create_8)(ptr::null_mut());
            let mcr = (p.r.pcre2_match_context_create_8)(ptr::null_mut());
            for &ml in &mlimits {
                assert_eq!((p.c.pcre2_set_match_limit_8)(mcc, ml), 0);
                assert_eq!((p.r.pcre2_set_match_limit_8)(mcr, ml), 0);
                let out = dfa_cmp(
                    &mut p, codes, (mcc, mcr), &sb, sb.len(), K::new(0, 0, 6, 1000),
                    "C165 match_limit",
                );
                if ml == 0 {
                    assert_eq!(
                        out.m.rc, PCRE2_ERROR_MATCHLIMIT,
                        "C165 match_limit 0 pat={:?}",
                        pat
                    );
                }
            }
            assert_eq!((p.c.pcre2_set_match_limit_8)(mcc, 10_000_000), 0);
            assert_eq!((p.r.pcre2_set_match_limit_8)(mcr, 10_000_000), 0);
            for &dl in &dlimits {
                assert_eq!((p.c.pcre2_set_depth_limit_8)(mcc, dl), 0);
                assert_eq!((p.r.pcre2_set_depth_limit_8)(mcr, dl), 0);
                dfa_cmp(
                    &mut p, codes, (mcc, mcr), &sb, sb.len(), K::new(0, 0, 6, 1000),
                    "C165 depth_limit",
                );
            }
            assert_eq!((p.c.pcre2_set_depth_limit_8)(mcc, 10_000_000), 0);
            assert_eq!((p.r.pcre2_set_depth_limit_8)(mcr, 10_000_000), 0);
            for &hl in &hlimits {
                assert_eq!((p.c.pcre2_set_heap_limit_8)(mcc, hl), 0);
                assert_eq!((p.r.pcre2_set_heap_limit_8)(mcr, hl), 0);
                dfa_cmp(
                    &mut p, codes, (mcc, mcr), &sb, sb.len(), K::new(0, 0, 6, 1000),
                    "C165 heap_limit",
                );
            }
            (p.c.pcre2_match_context_free_8)(mcc);
            (p.r.pcre2_match_context_free_8)(mcr);

            // (*LIMIT_MATCH=..) / (*LIMIT_DEPTH=..) / (*LIMIT_HEAP=..) in the pattern
            // are applied as a *cap* on the context values.
            for pfx in [
                "(*LIMIT_MATCH=1)",
                "(*LIMIT_MATCH=3)",
                "(*LIMIT_DEPTH=0)",
                "(*LIMIT_DEPTH=2)",
                "(*LIMIT_HEAP=0)",
                "(*LIMIT_HEAP=1)",
            ] {
                let full = format!("{}{}", pfx, pat);
                let Some(cs) = p.compile(full.as_bytes(), 0, 0, 0, 0) else { continue };
                dfa_cmp(
                    &mut p, cs, (ptr::null_mut(), ptr::null_mut()), &sb, sb.len(),
                    K::new(0, 0, 6, 1000), "C165 pattern limit",
                );
                p.free_code(cs);
            }
            p.free_code(codes);
        }

        // more_workspace() -> PCRE2_ERROR_NOMEMORY when the allocator refuses.  The
        // allocator lives in the *test*, so both libraries get exactly the same refusal.
        let gc2 = (p.c.pcre2_general_context_create_8)(
            Some(zmalloc_smallonly), Some(zfree), ptr::null_mut(),
        );
        let gr2 = (p.r.pcre2_general_context_create_8)(
            Some(zmalloc_smallonly), Some(zfree), ptr::null_mut(),
        );
        assert!(!gc2.is_null() && !gr2.is_null());
        let mcc = (p.c.pcre2_match_context_create_8)(gc2);
        let mcr = (p.r.pcre2_match_context_create_8)(gr2);
        assert!(!mcc.is_null() && !mcr.is_null());
        let mut seen_nomem = 0usize;
        for &(pat, subj) in cases {
            let codes = p.compile(pat.as_bytes(), 0, 0, 0, 0).unwrap();
            let sb = subj.as_bytes().to_vec();
            let out = dfa_cmp(
                &mut p, codes, (mcc, mcr), &sb, sb.len(), K::new(0, 0, 6, 1000),
                "C165 more_workspace NOMEMORY",
            );
            if out.m.rc == PCRE2_ERROR_NOMEMORY {
                seen_nomem += 1;
            }
            p.free_code(codes);
        }
        (p.c.pcre2_match_context_free_8)(mcc);
        (p.r.pcre2_match_context_free_8)(mcr);
        (p.c.pcre2_general_context_free_8)(gc2);
        (p.r.pcre2_general_context_free_8)(gr2);
        println!("C165: reached NOMEMORY in {} of {} cases", seen_nomem, cases.len());
    }
    println!("C165: {} comparisons", p.n);
    assert!(p.n > 300);
}

// ---------------------------------------------------------------------------------
// C166 - partial matching
// ---------------------------------------------------------------------------------

#[test]
fn c166_partial_matching() {
    println!("C166 pcre2_dfa_match_8 partial matching");
    let mut p = Pair::new();
    unsafe {
        let pats = [
            ".", "$", "^", "\\Z", "\\z", "\\b", "\\R", "\\N", "abc", "a\\r\\nb",
            "(?<=a)b", "a*", "()", "abc$", "a.c", "[^x]", "\\d+", "a{2,4}",
            "(?:ab)+", "x?", "(?=a)", "a|", "\\A", "\\n", "\\r",
        ];
        let subjs: [&[u8]; 18] = [
            b"", b"a", b"ab", b"abc", b"abcd", b"a\r", b"\r", b"\r\n", b"a\n", b"\n",
            b"a\rb", b"ab\r", b"aa", b"\r\r", b"a\r\n", b"\n\r", b"abca", b"xyz",
        ];
        let nls = [
            PCRE2_NEWLINE_CR,
            PCRE2_NEWLINE_LF,
            PCRE2_NEWLINE_CRLF,
            PCRE2_NEWLINE_ANY,
            PCRE2_NEWLINE_ANYCRLF,
            PCRE2_NEWLINE_NUL,
        ];
        for pat in pats {
            for &nl in &nls {
                for &copts in &[0u32, PCRE2_MULTILINE, PCRE2_DOTALL] {
                    let Some(codes) = p.compile(pat.as_bytes(), copts, 0, nl, 0) else { continue };
                    for s in subjs {
                        let sb = s.to_vec();
                        for &pm in &[0u32, PCRE2_PARTIAL_SOFT, PCRE2_PARTIAL_HARD] {
                            dfa_cmp(
                                &mut p, codes, (ptr::null_mut(), ptr::null_mut()), &sb,
                                sb.len(), K::new(0, pm, 4, 200), "C166 partial",
                            );
                        }
                    }
                    p.free_code(codes);
                }
            }
        }

        // The "allowemptypartial" flag: a pattern with a lookbehind, or one that can
        // match empty, changes the final partial decision (pcre2_dfa_match.c:3274-3295).
        let aep = [
            "(?<=a)b", "(?<=abc)d", "a*", "(?:)", "()", "x?", "(?=a)", "\\b",
            "(?<!x)y", "a{0,3}", "(?<=a)", "b|",
        ];
        for pat in aep {
            for &nl in &nls {
                let codes = p.compile(pat.as_bytes(), 0, 0, nl, 0).unwrap();
                for s in ["", "a", "ab", "abc", "abcd", "\r", "\r\n", "b", "x"] {
                    let sb = s.as_bytes().to_vec();
                    for so in 0..=sb.len() {
                        for &pm in &[
                            0u32,
                            PCRE2_PARTIAL_SOFT,
                            PCRE2_PARTIAL_HARD,
                            PCRE2_PARTIAL_SOFT | PCRE2_NOTEMPTY,
                            PCRE2_PARTIAL_HARD | PCRE2_NOTEMPTY_ATSTART,
                            PCRE2_PARTIAL_SOFT | PCRE2_ANCHORED,
                            PCRE2_PARTIAL_HARD | PCRE2_DFA_SHORTEST,
                        ] {
                            dfa_cmp(
                                &mut p, codes, (ptr::null_mut(), ptr::null_mut()), &sb,
                                sb.len(), K::new(so, pm, 4, 200), "C166 allowemptypartial",
                            );
                        }
                    }
                }
                p.free_code(codes);
            }
        }
    }
    println!("C166: {} comparisons", p.n);
    assert!(p.n > 5000);
}

// ---------------------------------------------------------------------------------
// C167 - PCRE2_ERROR_DFA_UFUNC from the substring / substitute entry points
// ---------------------------------------------------------------------------------

#[test]
fn c167_dfa_ufunc() {
    println!("C167 DFA match data -> DFA_UFUNC from substring/substitute entry points");
    let mut p = Pair::new();
    unsafe {
        let cases: &[(&str, &str)] = &[
            ("(?<w>a)(?<x>b)", "ab"),
            ("(?<w>a+)", "aaa"),
            ("(?<w>a)|(?<x>b)", "b"),
            ("a|ab|abc", "abc"),
            ("(?<w>x)?y", "y"),
            ("(?<w>[a-z]+)", "hello"),
            ("(?<w>a)(b)(?<x>c)", "abc"),
        ];
        for &(pat, subj) in cases {
            let codes = p.compile(pat.as_bytes(), 0, 0, 0, 0).unwrap();
            let sb = subj.as_bytes().to_vec();
            for &opts in &[0u32, PCRE2_DFA_SHORTEST, PCRE2_PARTIAL_SOFT, PCRE2_ANCHORED] {
                for &ovn in &[1u32, 2, 4, 8] {
                    // Note: NO sentinel prefill here - `pcre2_substring_length_bynumber`
                    // reads ovector slots the DFA never wrote, and a sentinel would take
                    // it down a PCRE2_DEBUG_UNREACHABLE() path.
                    let mdc = p.md(true, ovn);
                    let mdr = p.md(false, ovn);
                    let mut wc = vec![0i32; 500];
                    let mut wr = vec![0i32; 500];
                    let a = dfa_one(
                        p.c, mdc, codes.0, sb.as_ptr(), sb.len(), 0, opts, ptr::null_mut(),
                        &mut wc, 500, false,
                    );
                    let b = dfa_one(
                        p.r, mdr, codes.1, sb.as_ptr(), sb.len(), 0, opts, ptr::null_mut(),
                        &mut wr, 500, false,
                    );
                    p.n += 1;
                    assert_eq!(a, b, "C167 dfa match pat={:?} opts={:#x}", pat, opts);

                    let probe = |api: &Api, md: Ptr, code: Ptr| -> Vec<i64> {
                        let mut v = vec![];
                        let name = b"w\0";
                        // ---- the four DFA_UFUNC entry points
                        let mut buf = [0u8; 64];
                        let mut sz: usize = 64;
                        v.push((api.pcre2_substring_copy_byname_8)(
                            md, name.as_ptr(), buf.as_mut_ptr(), &mut sz,
                        ) as i64);
                        v.push(sz as i64);
                        let mut sp: *mut u8 = ptr::null_mut();
                        let mut sz2: usize = 0;
                        let rc = (api.pcre2_substring_get_byname_8)(
                            md, name.as_ptr(), &mut sp, &mut sz2,
                        );
                        v.push(rc as i64);
                        v.push(sz2 as i64);
                        v.push(sp.is_null() as i64);
                        if rc == 0 && !sp.is_null() {
                            (api.pcre2_substring_free_8)(sp);
                        }
                        let mut sz3: usize = 0;
                        v.push((api.pcre2_substring_length_byname_8)(
                            md, name.as_ptr(), &mut sz3,
                        ) as i64);
                        v.push(sz3 as i64);
                        // ---- pcre2_substitute with PCRE2_SUBSTITUTE_MATCHED
                        let mut obuf = [0u8; 128];
                        let mut olen: usize = 128;
                        v.push((api.pcre2_substitute_8)(
                            code, sb.as_ptr(), sb.len(), 0, PCRE2_SUBSTITUTE_MATCHED, md,
                            ptr::null_mut(), b"Z".as_ptr(), 1, obuf.as_mut_ptr(), &mut olen,
                        ) as i64);
                        v.push(olen as i64);
                        // ---- the (top_bracket-free) DFA path in length_bynumber
                        for n in 0..6u32 {
                            let mut sz: usize = 0;
                            v.push((api.pcre2_substring_length_bynumber_8)(md, n, &mut sz) as i64);
                            v.push(sz as i64);
                        }
                        // ---- and the by-number entry points, which do NOT reject DFA data
                        for n in 0..3u32 {
                            let mut b2 = [0u8; 64];
                            let mut s2: usize = 64;
                            v.push((api.pcre2_substring_copy_bynumber_8)(
                                md, n, b2.as_mut_ptr(), &mut s2,
                            ) as i64);
                            v.push(s2 as i64);
                            let mut sp2: *mut u8 = ptr::null_mut();
                            let mut s3: usize = 0;
                            let rc = (api.pcre2_substring_get_bynumber_8)(
                                md, n, &mut sp2, &mut s3,
                            );
                            v.push(rc as i64);
                            v.push(s3 as i64);
                            if rc == 0 && !sp2.is_null() {
                                (api.pcre2_substring_free_8)(sp2);
                            }
                        }
                        // ---- nametable scan / number_from_name (code-only, no matchedby)
                        v.push((api.pcre2_substring_number_from_name_8)(code, name.as_ptr()) as i64);
                        v
                    };
                    let va = probe(p.c, mdc, codes.0);
                    let vb = probe(p.r, mdr, codes.1);
                    p.n += 1;
                    assert_eq!(
                        va, vb,
                        "C167 substring/substitute results differ pat={:?} opts={:#x} ovn={}",
                        pat, opts, ovn
                    );
                    // The four documented DFA_UFUNC sites.
                    if a.m.rc >= 0 {
                        assert_eq!(va[0], PCRE2_ERROR_DFA_UFUNC as i64, "copy_byname");
                        assert_eq!(va[2], PCRE2_ERROR_DFA_UFUNC as i64, "get_byname");
                        assert_eq!(va[5], PCRE2_ERROR_DFA_UFUNC as i64, "length_byname");
                        assert_eq!(va[7], PCRE2_ERROR_DFA_UFUNC as i64, "substitute MATCHED");
                    }
                    (p.c.pcre2_match_data_free_8)(mdc);
                    (p.r.pcre2_match_data_free_8)(mdr);
                }
            }
            p.free_code(codes);
        }
    }
    println!("C167: {} comparisons", p.n);
    assert!(p.n > 100);
}

// ---------------------------------------------------------------------------------
// C168 / C169 - pcre2_next_match_8
// ---------------------------------------------------------------------------------

/// One step of a `pcre2_match` + `pcre2_next_match` iteration.
#[derive(Debug, PartialEq, Eq, Clone)]
struct Step {
    m: MatchOut,
    cont: i32,
    off: usize,
    opts: u32,
}

/// Run the full documented iteration loop and return the entire sequence.
#[allow(clippy::too_many_arguments)]
unsafe fn iterate(
    api: &Api,
    g: Ptr,
    code: Ptr,
    subj: &[u8],
    slen: usize,
    start: usize,
    first_opts: u32,
    ovn: u32,
    limit: usize,
    dfa: bool,
) -> Vec<Step> {
    let md = (api.pcre2_match_data_create_8)(ovn, g);
    assert!(!md.is_null());
    let mut ws = vec![0i32; 400];
    let mut off = start;
    let mut opts = first_opts;
    let mut out = vec![];
    for _ in 0..limit {
        fill_ov(api, md, SENT);
        let rc = if dfa {
            (api.pcre2_dfa_match_8)(
                code, subj.as_ptr(), slen, off, opts, md, ptr::null_mut(),
                ws.as_mut_ptr(), 400,
            )
        } else {
            (api.pcre2_match_8)(code, subj.as_ptr(), slen, off, opts, md, ptr::null_mut())
        };
        let m = read_match(api, md, rc);
        // sentinels in the out-parameters: `pcre2_next_match` must not touch them when
        // it returns FALSE for a negative rc.
        let mut o2: usize = 0xDEAD_BEEF_DEAD_BEEF;
        let mut p2: u32 = 0xDEAD_BEEF;
        let cont = (api.pcre2_next_match_8)(md, &mut o2, &mut p2);
        out.push(Step { m, cont, off: o2, opts: p2 });
        if cont == 0 {
            break;
        }
        off = o2;
        opts = p2;
    }
    (api.pcre2_match_data_free_8)(md);
    out
}

#[test]
fn c168_next_match_classification() {
    println!("C168 pcre2_next_match_8 three-way classification");
    let mut p = Pair::new();
    unsafe {
        let pats = [
            "a*", "(?=a)", "\\b", "x?", "", "a|", "(?<=a)", "a", "a+", "\\B",
            "(?:)", "b*", "[ab]*", "\\d*", "$", "^", "(?m)^", "(?m)$", "\\Z", "\\z",
            "a??", ".*", ".", "\\R", "(a)(b)?",
        ];
        let subjs: [&[u8]; 16] = [
            b"", b"a", b"aa", b"aab", b"b", b"ab", b"\n", b"\n\n\n", b"\r\n\r\n",
            b"a\nb\nc", b"aaaa", b"xyz", b"a b c", b"\r", b"a\r\n", b"abcabc",
        ];
        for pat in pats {
            for &copts in &[0u32, PCRE2_MULTILINE, PCRE2_ANCHORED, PCRE2_UTF] {
                let Some(codes) = p.compile(pat.as_bytes(), copts, 0, 0, 0) else { continue };
                for s in subjs {
                    if copts & PCRE2_UTF != 0 && std::str::from_utf8(s).is_err() {
                        continue;
                    }
                    let sb = s.to_vec();
                    for so in 0..=sb.len() {
                        for &fo in &[0u32, PCRE2_NOTEMPTY_ATSTART, PCRE2_NOTEMPTY] {
                            let a = iterate(
                                p.c, p.gc, codes.0, &sb, sb.len(), so, fo, 4, 40, false,
                            );
                            let b = iterate(
                                p.r, p.gr, codes.1, &sb, sb.len(), so, fo, 4, 40, false,
                            );
                            p.n += 1;
                            assert_eq!(
                                a, b,
                                "C168 iteration differs pat={:?} copts={:#x} subj={:?} so={} fo={:#x}",
                                pat, copts, s, so, fo
                            );
                        }
                    }
                }
                p.free_code(codes);
            }
        }

        // ---- rc < 0 short circuit: FALSE and the outputs are left untouched.
        //      NOMATCH, PARTIAL, and a hard error (BADOFFSET / BADUTFOFFSET).
        let codes = p.compile(b"abcd", 0, 0, 0, 0).unwrap();
        let sb = b"xxab".to_vec();
        for &(so, mo) in &[
            (0usize, 0u32),
            (0, PCRE2_PARTIAL_SOFT),
            (0, PCRE2_PARTIAL_HARD),
            (4, 0),
            (5, 0),
            (99, 0),
            (0, PCRE2_ANCHORED),
            (0, PCRE2_ENDANCHORED),
        ] {
            let go = |api: &Api, g: Ptr, code: Ptr| -> (MatchOut, i32, usize, u32) {
                let md = (api.pcre2_match_data_create_8)(4, g);
                fill_ov(api, md, SENT);
                let rc = (api.pcre2_match_8)(
                    code, sb.as_ptr(), sb.len(), so, mo, md, ptr::null_mut(),
                );
                let m = read_match(api, md, rc);
                let mut o: usize = 0x1234_5678_9ABC_DEF0;
                let mut q: u32 = 0x0BAD_F00D;
                let cont = (api.pcre2_next_match_8)(md, &mut o, &mut q);
                (api.pcre2_match_data_free_8)(md);
                (m, cont, o, q)
            };
            let a = go(p.c, p.gc, codes.0);
            let b = go(p.r, p.gr, codes.1);
            p.n += 1;
            assert_eq!(a, b, "C168 negative rc so={} mo={:#x}", so, mo);
            if a.0.rc < 0 {
                assert_eq!(a.1, 0, "C168 rc<0 must give FALSE");
                assert_eq!(a.2, 0x1234_5678_9ABC_DEF0, "C168 offset must be untouched");
                assert_eq!(a.3, 0x0BAD_F00D, "C168 options must be untouched");
            }
        }
        p.free_code(codes);

        // ---- \K-in-lookaround: ovector[1] == start_offset but ovector[0] != it, both
        //      mid-subject and at the very end of the subject (the FALSE case).
        let mut seen_noprogress = 0usize;
        for pat in ["(?<=\\Ka)", "(?=b\\K)", "(?<=\\Kab)", "(?=ab\\K)", "(?<=a\\K)"] {
            let Some(codes) =
                p.compile(pat.as_bytes(), 0, PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK, 0, 0)
            else {
                panic!("C168 {:?} did not compile", pat)
            };
            for s in ["a", "ab", "abz", "az", "ba", "b", "zab", "aab"] {
                let sb = s.as_bytes().to_vec();
                for so in 0..=sb.len() {
                    let go = |api: &Api, g: Ptr, code: Ptr| -> (MatchOut, i32, usize, u32) {
                        let md = (api.pcre2_match_data_create_8)(4, g);
                        fill_ov(api, md, SENT);
                        let rc = (api.pcre2_match_8)(
                            code, sb.as_ptr(), sb.len(), so, 0, md, ptr::null_mut(),
                        );
                        let m = read_match(api, md, rc);
                        let mut o: usize = 0xAAAA_AAAA;
                        let mut q: u32 = 0xBBBB_BBBB;
                        let cont = (api.pcre2_next_match_8)(md, &mut o, &mut q);
                        (api.pcre2_match_data_free_8)(md);
                        (m, cont, o, q)
                    };
                    let a = go(p.c, p.gc, codes.0);
                    let b = go(p.r, p.gr, codes.1);
                    p.n += 1;
                    assert_eq!(a, b, "C168 \\K pat={:?} subj={:?} so={}", pat, s, so);
                    if a.0.rc >= 0 && a.0.ovector[1] == so && a.0.ovector[0] != so {
                        seen_noprogress += 1;
                    }
                }
                // and the whole iteration, which must terminate
                let a = iterate(p.c, p.gc, codes.0, &sb, sb.len(), 0, 0, 4, 40, false);
                let b = iterate(p.r, p.gr, codes.1, &sb, sb.len(), 0, 0, 4, 40, false);
                p.n += 1;
                assert_eq!(a, b, "C168 \\K iteration pat={:?} subj={:?}", pat, s);
            }
            p.free_code(codes);
        }
        assert!(
            seen_noprogress > 3,
            "C168 the no-progress branch was reached only {} times",
            seen_noprogress
        );

        // ---- pcre2_next_match also works on a match data filled by pcre2_dfa_match
        for pat in ["a*", "x?", "a", "(?=a)", "\\b", "[ab]+"] {
            let codes = p.compile(pat.as_bytes(), 0, 0, 0, 0).unwrap();
            for s in ["", "a", "aab", "\r\n\r\n", "abcabc"] {
                let sb = s.as_bytes().to_vec();
                let a = iterate(p.c, p.gc, codes.0, &sb, sb.len(), 0, 0, 4, 40, true);
                let b = iterate(p.r, p.gr, codes.1, &sb, sb.len(), 0, 0, 4, 40, true);
                p.n += 1;
                assert_eq!(a, b, "C168 dfa-driven iteration pat={:?} subj={:?}", pat, s);
            }
            p.free_code(codes);
        }
    }
    println!("C168: {} comparisons", p.n);
    assert!(p.n > 2000);
}

#[test]
fn c169_next_match_bumpalong() {
    println!("C169 pcre2_next_match_8 do_bumpalong");
    let mut p = Pair::new();
    unsafe {
        // The ONLY route into do_bumpalong is the no-progress case, i.e. a \K inside a
        // lookaround that leaves ovector[1] == start_offset (pcre2_match_next.c:126-140).
        // `(?<=\Ka)` matched at offset s has ovector = (s-1, s), so the bump point is
        // exactly `subject[s]` - which lets us aim it at each of the three exits.
        let bumps: &[(&str, usize)] = &[
            ("az", 1),                 // ordinary byte
            ("a\r\nx", 1),             // CR immediately followed by LF
            ("a\r", 1),                // CR at the last code unit
            ("a\r\n", 1),              // CRLF at the end of the subject
            ("a\n\r", 1),              // LF then CR
            ("a\u{e9}z", 1),           // 2-byte UTF character
            ("a\u{20ac}z", 1),         // 3-byte UTF character
            ("a\u{1F600}z", 1),        // 4-byte UTF character
            ("a", 1),                  // bump point is the end of the subject
            ("zab", 2),                // ordinary, mid-subject
            ("zaa\r\n", 3),            // CRLF, mid-subject
        ];
        let nls = [
            0u32,
            PCRE2_NEWLINE_CR,
            PCRE2_NEWLINE_LF,
            PCRE2_NEWLINE_CRLF,
            PCRE2_NEWLINE_ANY,
            PCRE2_NEWLINE_ANYCRLF,
            PCRE2_NEWLINE_NUL,
        ];
        // Ill-formed UTF byte sequences at the bump point; NO_UTF_CHECK is required.
        let bad: &[(&[u8], usize)] = &[
            (b"a\xc3", 1),
            (b"a\xc3\xa9", 1),
            (b"a\x80", 1),
            (b"a\x80\x80\x80", 1),
            (b"a\xff\x80z", 1),
            (b"a\xe2\x80", 1),
            (b"a\xf0\x9f\x98", 1),
            (b"a\xc0\x80", 1),
        ];

        let mut exits = [0usize; 3]; // [+2 CRLF, UTF char, +1]
        for pat in ["(?<=\\Ka)", "(?<=\\Ka)|(?<=\\Kb)"] {
            for &nl in &nls {
                for &utf in &[0u32, PCRE2_UTF] {
                    let Some(codes) = p.compile(
                        pat.as_bytes(), utf, PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK, nl, 0,
                    ) else {
                        continue;
                    };
                    let mut all: Vec<(Vec<u8>, usize)> = bumps
                        .iter()
                        .map(|&(s, o)| (s.as_bytes().to_vec(), o))
                        .collect();
                    for &(s, o) in bad {
                        all.push((s.to_vec(), o));
                    }
                    for (sb, so) in &all {
                        if *so > sb.len() {
                            continue;
                        }
                        let go = |api: &Api, g: Ptr, code: Ptr| -> (MatchOut, i32, usize, u32) {
                            let md = (api.pcre2_match_data_create_8)(4, g);
                            fill_ov(api, md, SENT);
                            let rc = (api.pcre2_match_8)(
                                code, sb.as_ptr(), sb.len(), *so, PCRE2_NO_UTF_CHECK, md,
                                ptr::null_mut(),
                            );
                            let m = read_match(api, md, rc);
                            let mut o: usize = 0xFEED_FACE;
                            let mut q: u32 = 0xCAFE_BABE;
                            let cont = (api.pcre2_next_match_8)(md, &mut o, &mut q);
                            (api.pcre2_match_data_free_8)(md);
                            (m, cont, o, q)
                        };
                        let a = go(p.c, p.gc, codes.0);
                        let b = go(p.r, p.gr, codes.1);
                        p.n += 1;
                        assert_eq!(
                            a, b,
                            "C169 bumpalong pat={:?} utf={:#x} nl={} subj={:?} so={}",
                            pat, utf, nl, sb, so
                        );
                        if a.0.rc >= 0 && a.1 != 0 && a.0.ovector[1] == *so
                            && a.0.ovector[0] != *so
                        {
                            match a.2 - *so {
                                2 if sb.get(*so) == Some(&b'\r')
                                    && sb.get(*so + 1) == Some(&b'\n') => exits[0] += 1,
                                1 => exits[2] += 1,
                                _ => exits[1] += 1,
                            }
                        }
                    }
                    p.free_code(codes);
                }
            }
        }
        println!(
            "C169 do_bumpalong exits reached: CRLF+2={} utf-char={} plain+1={}",
            exits[0], exits[1], exits[2]
        );
        assert!(exits[0] > 0, "C169 the atomic-CRLF exit was never reached");
        assert!(exits[1] > 0, "C169 the UTF-character exit was never reached");
        assert!(exits[2] > 0, "C169 the +1 exit was never reached");

        // ---- full iterations over newline-heavy and UTF subjects, for every newline
        // convention: the sequence of (rc, ovector, cont, offset, options) must match.
        let pats = ["a*", "x?", "", "(?=a)", "\\b", ".", "(?m)^", "(?m)$", "\\R", "a|"];
        let subjs: &[&[u8]] = &[
            b"", b"\r\n", b"\r\n\r\n\r\n", b"\n\r\n\r", b"\r\r\r", b"\n\n\n",
            b"a\r\nb\r\nc", b"\xc3\xa9\xc3\xa9", b"a\xe2\x82\xacb",
            b"\xf0\x9f\x98\x80x", b"\xc2\x85\xe2\x80\xa8\xe2\x80\xa9",
            b"a\0b\0c", b"\x0b\x0c\x85",
        ];
        for pat in pats {
            for &nl in &nls {
                for &utf in &[0u32, PCRE2_UTF] {
                    let Some(codes) = p.compile(pat.as_bytes(), utf, 0, nl, 0) else { continue };
                    for s in subjs {
                        if utf != 0 && std::str::from_utf8(s).is_err() {
                            continue;
                        }
                        let sb = s.to_vec();
                        let a = iterate(
                            p.c, p.gc, codes.0, &sb, sb.len(), 0, 0, 4, 60, false,
                        );
                        let b = iterate(
                            p.r, p.gr, codes.1, &sb, sb.len(), 0, 0, 4, 60, false,
                        );
                        p.n += 1;
                        assert_eq!(
                            a, b,
                            "C169 iteration pat={:?} utf={:#x} nl={} subj={:?}",
                            pat, utf, nl, s
                        );
                    }
                    p.free_code(codes);
                }
            }
        }
    }
    println!("C169: {} comparisons", p.n);
    assert!(p.n > 800);
}

// ---------------------------------------------------------------------------------
// Big randomized cross-cutting sweep over C158-C166
// ---------------------------------------------------------------------------------

const RAND_PATS: &[&str] = &[
    "a",
    "a+",
    "a*",
    "ab|abcd",
    "(?:ab)+",
    "[a-z]+",
    "\\d{2,4}",
    "^abc",
    "abc$",
    ".",
    ".*",
    "\\w+\\s\\w+",
    "a(?=b)",
    "a(?!b)",
    "(?<=a)b",
    "(?<!a)b",
    "\\bfoo\\b",
    "(a|b|c){2,5}",
    "x?y?z?",
    "\\R",
    "\\N",
    "[^a]*",
    "(?s).+",
    "(?i)abc",
    "\\Aab",
    "ab\\Z",
    "ab\\z",
    "\\X+",
    "\\p{L}+",
    "\\P{L}",
    "[[:alpha:]]+",
    "(?>a+)b",
    "(a|b)++c",
    "\\((?:[^()]|(?R))*\\)",
    "(?(?=a)b|c)",
    "(a)(b)(c)",
    "(?:a|ab|abc)",
    "[\\x{100}-\\x{200}]",
    "\\r\\n",
    "$",
    "^",
    "()",
    "(?:)",
    "(?m)^.$",
    "\\h+",
    "\\v+",
    "\\s*",
    "(?:.|\\n)*",
    "[\\d\\s]{1,5}",
    "(?=.*a)b*",
];

const RAND_COPTS: &[u32] = &[
    0,
    PCRE2_UTF,
    PCRE2_UTF | PCRE2_UCP,
    PCRE2_UCP,
    PCRE2_MULTILINE,
    PCRE2_DOTALL,
    PCRE2_UTF | PCRE2_MULTILINE,
    PCRE2_CASELESS,
    PCRE2_MULTILINE | PCRE2_DOTALL,
    PCRE2_NO_START_OPTIMIZE,
    PCRE2_UTF | PCRE2_UCP | PCRE2_CASELESS,
    PCRE2_ANCHORED,
    PCRE2_FIRSTLINE,
    PCRE2_MULTILINE | PCRE2_FIRSTLINE,
    PCRE2_DOLLAR_ENDONLY,
    PCRE2_UTF | PCRE2_DOTALL | PCRE2_MULTILINE,
    PCRE2_USE_OFFSET_LIMIT,
    PCRE2_USE_OFFSET_LIMIT | PCRE2_MULTILINE,
    PCRE2_NO_DOTSTAR_ANCHOR,
    PCRE2_NO_AUTO_POSSESS,
];

const NEWLINE_SHAPES: &[&[u8]] = &[
    b"\n", b"\r", b"\r\n", b"\x0b", b"\x0c", b"\xc2\x85", b"\xe2\x80\xa8",
    b"\xe2\x80\xa9", b"\0",
];

fn rand_subject(rng: &mut Rng) -> Vec<u8> {
    // Mostly 0..40 code units, with an occasional long one so that the bumpalong /
    // memchr first-code-unit scans and larger state counts get exercised too.
    let len = if rng.below(8) == 0 {
        41 + rng.below(160) as usize
    } else {
        rng.below(41) as usize
    };
    let alpha = rng.below(7);
    let mut v = Vec::with_capacity(len + 8);
    while v.len() < len {
        match alpha {
            0 => v.push(*rng.pick(b"abcABC012 \t.-_")),
            1 => v.push(rng.byte()),
            2 => v.push(0x80 | (rng.byte() & 0x7f)),
            3 => {
                // valid UTF-8, 1..4 bytes
                let cp = match rng.below(4) {
                    0 => rng.below(0x80),
                    1 => 0x80 + rng.below(0x780),
                    2 => 0x800 + rng.below(0xF800),
                    _ => 0x1_0000 + rng.below(0x10_0000),
                };
                let cp = if (0xD800..0xE000).contains(&cp) { 0x41 } else { cp };
                let mut b = [0u8; 4];
                let s = char::from_u32(cp).unwrap_or('A').encode_utf8(&mut b);
                v.extend_from_slice(s.as_bytes());
            }
            4 => v.extend_from_slice(rng.pick(NEWLINE_SHAPES)),
            5 => v.push(*rng.pick(b"a\0b\0\0")),
            _ => v.push(b'a'),
        }
    }
    v.truncate(len);
    // Pad so that a UTF-8 sequence truncated by `len` can never be read out of the
    // allocation by either library (GETCHARLENTEST does not bound-check).
    v.extend_from_slice(&[0u8; 8]);
    v
}

#[test]
fn c158_c166_random_sweep() {
    println!("C158-C166 randomized pcre2_dfa_match_8 sweep");
    let mut p = Pair::new();
    let mut rng = Rng::new(0x00DF_A5EE_D001);
    let nls: [u32; 7] = [
        0,
        PCRE2_NEWLINE_CR,
        PCRE2_NEWLINE_LF,
        PCRE2_NEWLINE_CRLF,
        PCRE2_NEWLINE_ANY,
        PCRE2_NEWLINE_ANYCRLF,
        PCRE2_NEWLINE_NUL,
    ];
    let bsrs: [u32; 3] = [0, PCRE2_BSR_UNICODE, PCRE2_BSR_ANYCRLF];
    let mopt_bits: [u32; 10] = [
        PCRE2_ANCHORED,
        PCRE2_ENDANCHORED,
        PCRE2_NOTBOL,
        PCRE2_NOTEOL,
        PCRE2_NOTEMPTY,
        PCRE2_NOTEMPTY_ATSTART,
        PCRE2_PARTIAL_SOFT,
        PCRE2_PARTIAL_HARD,
        PCRE2_DFA_SHORTEST,
        PCRE2_NO_UTF_CHECK,
    ];
    let mut cache: HashMap<(usize, usize, u32, u32), Option<(Ptr, Ptr)>> = HashMap::new();
    let iters: usize = 400_000;
    let mut rcs: HashMap<i32, usize> = HashMap::new();
    unsafe {
        let mcc = (p.c.pcre2_match_context_create_8)(ptr::null_mut());
        let mcr = (p.r.pcre2_match_context_create_8)(ptr::null_mut());
        assert!(!mcc.is_null() && !mcr.is_null());
        for it in 0..iters {
            let pi = rng.below(RAND_PATS.len() as u32) as usize;
            let ci = rng.below(RAND_COPTS.len() as u32) as usize;
            let nl = *rng.pick(&nls);
            let bsr = *rng.pick(&bsrs);
            let key = (pi, ci, nl, bsr);
            let entry = match cache.get(&key) {
                Some(e) => *e,
                None => {
                    let e = p.compile(RAND_PATS[pi].as_bytes(), RAND_COPTS[ci], 0, nl, bsr);
                    cache.insert(key, e);
                    e
                }
            };
            let Some(codes) = entry else { continue };

            let subj = rand_subject(&mut rng);
            let slen = subj.len() - 8;
            let mut so = match rng.below(30) {
                0 => slen + 1,
                1 => slen + 5,
                _ => rng.below(slen as u32 + 1) as usize,
            };
            let mut opts = 0u32;
            let nbits = rng.below(4);
            for _ in 0..nbits {
                opts |= *rng.pick(&mopt_bits);
            }
            if rng.below(20) == 0 {
                opts |= PCRE2_COPY_MATCHED_SUBJECT;
            }
            if rng.below(50) == 0 {
                opts |= 1u32 << rng.below(32);
            }
            // Handing PCRE2_UTF + PCRE2_NO_UTF_CHECK an ill-formed subject is explicitly
            // undefined behaviour: GETCHARLENTEST (pcre2_intmodedep.h:342) will build a
            // code point of up to 0x7FFFFFFF out of a 5/6-byte ill-formed sequence and
            // the C then indexes the UCD stage-1 table out of bounds.  Such subjects are
            // still exercised, but with the validity check left ON (from offset 0, so the
            // whole subject is validated), which is a well-defined PCRE2_ERROR_UTF8_*.
            let utf_mode = RAND_COPTS[ci] & PCRE2_UTF != 0;
            let subj_valid = std::str::from_utf8(&subj[..slen]).is_ok();
            if utf_mode && !subj_valid {
                opts &= !PCRE2_NO_UTF_CHECK;
                so = 0;
            }
            let ovn = *rng.pick(&[0u32, 1, 2, 3, 4, 6, 10]);
            let wsc = if rng.below(14) == 0 {
                *rng.pick(&[0usize, 1, 2, 19])
            } else {
                *rng.pick(&[20usize, 21, 22, 24, 30, 50, 80, 100, 200, 400, 2000])
            };
            // Every so often drive the call through a match context with randomised
            // limits (C165) and a randomised offset limit.
            let mctx = if rng.below(4) == 0 {
                let ml = *rng.pick(&[1u32, 2, 5, 50, 1000, 10_000_000]);
                let dl = *rng.pick(&[0u32, 1, 2, 5, 100, 10_000_000]);
                let hl = *rng.pick(&[0u32, 1, 30, 100, 20_000_000]);
                let ol = *rng.pick(&[
                    PCRE2_UNSET, PCRE2_UNSET, PCRE2_UNSET, PCRE2_UNSET, PCRE2_UNSET,
                    PCRE2_UNSET, PCRE2_UNSET, 0usize, 1, 3, 10, 1000,
                ]);
                for (api, mc) in [(p.c, mcc), (p.r, mcr)] {
                    assert_eq!((api.pcre2_set_match_limit_8)(mc, ml), 0);
                    assert_eq!((api.pcre2_set_depth_limit_8)(mc, dl), 0);
                    assert_eq!((api.pcre2_set_heap_limit_8)(mc, hl), 0);
                    assert_eq!((api.pcre2_set_offset_limit_8)(mc, ol), 0);
                }
                (mcc, mcr)
            } else {
                (ptr::null_mut(), ptr::null_mut())
            };
            let out = dfa_cmp(
                &mut p, codes, mctx, &subj, slen,
                K::new(so, opts, ovn, wsc), "random sweep",
            );
            *rcs.entry(out.m.rc.max(-100).min(20)).or_insert(0) += 1;

            // Every so often, chase a PARTIAL with a DFA_RESTART over a fresh random
            // continuation, comparing both stages.
            if out.m.rc == PCRE2_ERROR_PARTIAL && it % 3 == 0 {
                let cont = rand_subject(&mut rng);
                let clen = cont.len() - 8;
                if utf_mode && std::str::from_utf8(&cont[..clen]).is_err() {
                    continue;
                }
                let mdc = p.md(true, ovn.max(1));
                let mdr = p.md(false, ovn.max(1));
                let mut wc = vec![0i32; wsbuf_for(wsc.max(20))];
                let mut wr = vec![0i32; wsbuf_for(wsc.max(20))];
                let a1 = dfa_one(
                    p.c, mdc, codes.0, subj.as_ptr(), slen, so, opts, ptr::null_mut(),
                    &mut wc, wsc, true,
                );
                let b1 = dfa_one(
                    p.r, mdr, codes.1, subj.as_ptr(), slen, so, opts, ptr::null_mut(),
                    &mut wr, wsc, true,
                );
                p.n += 1;
                assert_eq!(a1, b1, "random restart stage1 pat={:?}", RAND_PATS[pi]);
                let a2 = dfa_one(
                    p.c, mdc, codes.0, cont.as_ptr(), clen, 0,
                    (opts & !(PCRE2_ANCHORED | PCRE2_ENDANCHORED)) | PCRE2_DFA_RESTART,
                    ptr::null_mut(), &mut wc, wsc, true,
                );
                let b2 = dfa_one(
                    p.r, mdr, codes.1, cont.as_ptr(), clen, 0,
                    (opts & !(PCRE2_ANCHORED | PCRE2_ENDANCHORED)) | PCRE2_DFA_RESTART,
                    ptr::null_mut(), &mut wr, wsc, true,
                );
                p.n += 1;
                assert_eq!(
                    a2, b2,
                    "random restart stage2 pat={:?} copts={:#x}",
                    RAND_PATS[pi], RAND_COPTS[ci]
                );
                (p.c.pcre2_match_data_free_8)(mdc);
                (p.r.pcre2_match_data_free_8)(mdr);
            }
        }
        (p.c.pcre2_match_context_free_8)(mcc);
        (p.r.pcre2_match_context_free_8)(mcr);
        for v in cache.values().flatten() {
            p.free_code(*v);
        }
    }
    let mut ks: Vec<_> = rcs.into_iter().collect();
    ks.sort();
    println!("C158-C166 random sweep: {} comparisons, rc histogram {:?}", p.n, ks);
    assert!(p.n >= iters);
}

#[test]
fn c168_c169_random_sweep() {
    println!("C168-C169 randomized pcre2_next_match_8 iteration sweep");
    let mut p = Pair::new();
    let mut rng = Rng::new(0x00_4E_4D_2024);
    let pats = [
        "a*", "(?=a)", "\\b", "x?", "", "a|", "(?<=a)", "a", "a+", "\\B", "(?:)",
        "[ab]*", "\\d*", "$", "^", "(?m)^", "(?m)$", "\\R", ".", ".*", "\\w*",
        "(a)(b)?", "\\Z", "[^\\n]*", "\\s*", "(?:ab)*", "\\X*", "\\p{L}*",
    ];
    let copts = [
        0u32,
        PCRE2_MULTILINE,
        PCRE2_UTF,
        PCRE2_UTF | PCRE2_UCP,
        PCRE2_DOTALL,
        PCRE2_ANCHORED,
        PCRE2_CASELESS,
        PCRE2_UTF | PCRE2_MULTILINE,
    ];
    let nls: [u32; 7] = [
        0,
        PCRE2_NEWLINE_CR,
        PCRE2_NEWLINE_LF,
        PCRE2_NEWLINE_CRLF,
        PCRE2_NEWLINE_ANY,
        PCRE2_NEWLINE_ANYCRLF,
        PCRE2_NEWLINE_NUL,
    ];
    let mut cache: HashMap<(usize, usize, u32), Option<(Ptr, Ptr)>> = HashMap::new();
    let iters = 120_000usize;
    unsafe {
        for _ in 0..iters {
            let pi = rng.below(pats.len() as u32) as usize;
            let ci = rng.below(copts.len() as u32) as usize;
            let nl = *rng.pick(&nls);
            let key = (pi, ci, nl);
            let entry = match cache.get(&key) {
                Some(e) => *e,
                None => {
                    let e = p.compile(pats[pi].as_bytes(), copts[ci], 0, nl, 0);
                    cache.insert(key, e);
                    e
                }
            };
            let Some(codes) = entry else { continue };
            let subj = rand_subject(&mut rng);
            let slen = subj.len() - 8;
            let mut so = rng.below(slen as u32 + 1) as usize;
            // An ill-formed subject in UTF mode is fine here (the iteration never passes
            // PCRE2_NO_UTF_CHECK, so it turns into a PCRE2_ERROR_UTF8_* every time), but
            // only if the validation covers the whole subject, i.e. from offset 0.
            if copts[ci] & PCRE2_UTF != 0 && std::str::from_utf8(&subj[..slen]).is_err() {
                so = 0;
            }
            let fo = *rng.pick(&[0u32, PCRE2_NOTEMPTY_ATSTART, PCRE2_NOTEMPTY]);
            let ovn = *rng.pick(&[1u32, 2, 4]);
            let dfa = rng.below(4) == 0;
            let a = iterate(p.c, p.gc, codes.0, &subj, slen, so, fo, ovn, 80, dfa);
            let b = iterate(p.r, p.gr, codes.1, &subj, slen, so, fo, ovn, 80, dfa);
            p.n += 1;
            assert_eq!(
                a, b,
                "C168/C169 random iteration pat={:?} copts={:#x} nl={} subj={:?} so={} fo={:#x} dfa={}",
                pats[pi], copts[ci], nl, &subj[..slen], so, fo, dfa
            );
        }
        for v in cache.values().flatten() {
            p.free_code(*v);
        }
    }
    println!("C168-C169 random sweep: {} iteration comparisons", p.n);
    assert!(p.n > 100_000);
}
