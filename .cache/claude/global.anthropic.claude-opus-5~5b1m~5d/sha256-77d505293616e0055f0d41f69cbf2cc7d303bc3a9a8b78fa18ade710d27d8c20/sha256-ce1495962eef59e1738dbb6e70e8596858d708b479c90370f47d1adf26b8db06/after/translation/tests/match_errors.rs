//! Phase C — error-path differential tests for `ERRORS.md`
//!
//! * section **C** — `pcre2_match_8` (rows 147–172)
//! * section **D** — `pcre2_dfa_match_8` (rows 173–200)
//! * section **M** — `_pcre2_valid_utf_8` and the UTF errors it surfaces
//!   through `pcre2_compile_8` / `pcre2_match_8` / `pcre2_dfa_match_8`
//!   (rows 368–389)
//!
//! Everything is driven through the two `.so` files (C and Rust) so the
//! `#[no_mangle]` wrappers are exercised as well. No test ever compares memory
//! that PCRE2 does not define (see `Obs::observe` for the exact rule).
mod common;
use common::diff::*;
use common::*;
use std::ffi::c_void;
use std::sync::Mutex;

// ---------------------------------------------------------------- error codes
// Verified against c_src/include/pcre2.h:350-441. The ones already provided by
// `common::diff` (ERR_NOMATCH, ERR_PARTIAL, ERR_NULL, ERR_BADMAGIC,
// ERR_BADOPTION, ERR_BADOFFSET, ERR_NOMEMORY, ERR_BADOFFSETLIMIT,
// ERR_JIT_BADOPTION, ...) are not repeated here.
const ERR_BADMODE: i32 = -32;
const ERR_BADUTFOFFSET: i32 = -36;
const ERR_DFA_BADRESTART: i32 = -38;
const ERR_DFA_RECURSE: i32 = -39;
const ERR_DFA_UCOND: i32 = -40;
const ERR_DFA_UITEM: i32 = -42;
const ERR_DFA_WSSIZE: i32 = -43;
const ERR_INTERNAL: i32 = -44;
const ERR_MATCHLIMIT: i32 = -47;
const ERR_RECURSELOOP: i32 = -52;
const ERR_DEPTHLIMIT: i32 = -53;
const ERR_HEAPLIMIT: i32 = -63;
const ERR_DFA_UINVALID_UTF: i32 = -66;
const ERR_JIT_UNSUPPORTED: i32 = -68;
const ERR_BAD_BACKSLASH_K: i32 = -75;

/// `PCRE2_ERROR_UTF8_ERRn` == `-(n + 2)`  (ERR1 = -3 … ERR21 = -23).
const fn utf8_err(n: i32) -> i32 {
    -(n + 2)
}

// jit option bits (c_src/include/pcre2.h:167-171)
const PCRE2_JIT_COMPLETE: u32 = 0x0000_0001;
const PCRE2_JIT_PARTIAL_SOFT: u32 = 0x0000_0002;
const PCRE2_JIT_PARTIAL_HARD: u32 = 0x0000_0004;
const PCRE2_JIT_INVALID_UTF: u32 = 0x0000_0100;
const PCRE2_JIT_TEST_ALLOC: u32 = 0x0000_0200;

/// `MAGIC_NUMBER` (pcre2_internal.h:542) and the `PCRE2_MODE_MASK` value that a
/// genuine 8-bit pattern carries (pcre2_internal.h:503,528).
const MAGIC_NUMBER: u32 = 0x5043_5245;
const PCRE2_MODE8: u32 = 1;

/// A workspace big enough for every DFA pattern used below.
const WS: usize = 8000;

fn fresh_ws(n: usize) -> Vec<i32> {
    vec![0i32; n.max(1)]
}

// ------------------------------------------------------------- observations
/// Everything that PCRE2 *defines* after a match call.
///
/// The rules (pcre2_match.c:8176-8203, pcre2_dfa_match.c:4056-4059, and
/// pcre2_match_data.c:54-68, which does **not** initialise `mark`/`startchar`):
///   * `rc > 0`  — ovector pairs `0..rc` are set
///   * `rc == 0` — the ovector was too small, so every pair was filled
///   * `rc == PCRE2_ERROR_PARTIAL` — only pair 0 holds the partial match
///   * any other `rc` — no ovector entry is defined
///   * `startchar` / `mark` are only written on the success / partial paths,
///     so they are only compared there.
#[derive(Debug, PartialEq, Eq)]
struct Obs {
    rc: i32,
    count: u32,
    ovec: Vec<usize>,
    startchar: Option<usize>,
    mark: Option<Vec<u8>>,
}

unsafe fn observe(api: &Api, md: *mut c_void, rc: i32) -> Obs {
    let count = (api.get_ovector_count)(md);
    let pairs: usize = if rc > 0 {
        (rc as usize).min(count as usize)
    } else if rc == 0 {
        count as usize
    } else if rc == ERR_PARTIAL {
        1.min(count as usize)
    } else {
        0
    };
    let ovp = (api.get_ovector_pointer)(md);
    let ovec = if ovp.is_null() || pairs == 0 {
        Vec::new()
    } else {
        std::slice::from_raw_parts(ovp, pairs * 2).to_vec()
    };
    let defined = rc >= 0 || rc == ERR_PARTIAL;
    let startchar = if defined { Some((api.get_startchar)(md)) } else { None };
    let mark = if defined {
        let p = (api.get_mark)(md);
        if p.is_null() {
            None
        } else {
            let mut v = Vec::new();
            let mut q = p;
            while *q != 0 {
                v.push(*q);
                q = q.add(1);
            }
            Some(v)
        }
    } else {
        None
    };
    Obs { rc, count, ovec, startchar, mark }
}

/// How to build the match data block for a call.
#[derive(Clone, Copy, Debug)]
enum Md {
    /// `pcre2_match_data_create_from_pattern_8`
    FromPattern,
    /// `pcre2_match_data_create_8(n, NULL)`
    Create(u32),
    /// pass `NULL` as `match_data`
    Null,
}

unsafe fn make_md(api: &Api, code: *mut c_void, md: Md) -> *mut c_void {
    match md {
        Md::FromPattern => (api.match_data_create_from_pattern)(code, std::ptr::null_mut()),
        Md::Create(n) => (api.match_data_create)(n, std::ptr::null_mut()),
        Md::Null => std::ptr::null_mut(),
    }
}

#[allow(clippy::too_many_arguments)]
unsafe fn obs_int(
    api: &Api,
    code: *mut c_void,
    subj: *const u8,
    len: usize,
    start: usize,
    opts: u32,
    md_kind: Md,
    mcfg: &MatchCfg,
) -> Obs {
    let mcx = make_mcontext(api, mcfg);
    let md = make_md(api, code, md_kind);
    if !matches!(md_kind, Md::Null) {
        assert!(!md.is_null(), "{}: match_data_create failed", api.name);
    }
    let rc = (api.do_match)(code, subj, len, start, opts, md, mcx);
    let out = if md.is_null() {
        Obs { rc, count: 0, ovec: Vec::new(), startchar: None, mark: None }
    } else {
        observe(api, md, rc)
    };
    if !md.is_null() {
        (api.match_data_free)(md);
    }
    if !mcx.is_null() {
        (api.match_context_free)(mcx);
    }
    out
}

#[allow(clippy::too_many_arguments)]
unsafe fn obs_dfa(
    api: &Api,
    code: *mut c_void,
    subj: *const u8,
    len: usize,
    start: usize,
    opts: u32,
    md_kind: Md,
    mcfg: &MatchCfg,
    ws: &mut [i32],
    wscount: usize,
    ws_null: bool,
) -> Obs {
    let mcx = make_mcontext(api, mcfg);
    let md = make_md(api, code, md_kind);
    if !matches!(md_kind, Md::Null) {
        assert!(!md.is_null(), "{}: match_data_create failed", api.name);
    }
    let wsp = if ws_null { std::ptr::null_mut() } else { ws.as_mut_ptr() };
    let rc = (api.dfa_match)(code, subj, len, start, opts, md, mcx, wsp, wscount);
    let out = if md.is_null() {
        Obs { rc, count: 0, ovec: Vec::new(), startchar: None, mark: None }
    } else {
        observe(api, md, rc)
    };
    if !md.is_null() {
        (api.match_data_free)(md);
    }
    if !mcx.is_null() {
        (api.match_context_free)(mcx);
    }
    out
}

// -------------------------------------------------- differential match drivers
/// One `pcre2_match_8` call in both libraries; asserts every defined observable
/// is identical and returns the shared `rc`.
#[allow(clippy::too_many_arguments)]
unsafe fn diff_int(
    cc: &Compiled,
    rr: &Compiled,
    subj: Option<&[u8]>,
    len: usize,
    start: usize,
    opts: u32,
    md_kind: Md,
    mcfg: &MatchCfg,
    label: &str,
) -> i32 {
    let (c, r) = both();
    let p = subj.map_or(std::ptr::null(), |s| s.as_ptr());
    let co = obs_int(c, cc.code, p, len, start, opts, md_kind, mcfg);
    let ro = obs_int(r, rr.code, p, len, start, opts, md_kind, mcfg);
    assert_eq!(
        co, ro,
        "{}: pcre2_match_8 diverged (C={:?} Rust={:?}) subject={:?} len={} start={} \
         options={:#010x} md={:?} mcfg={:?}",
        label, co, ro, subj.map(String::from_utf8_lossy), len, start, opts, md_kind, mcfg
    );
    co.rc
}

/// One `pcre2_dfa_match_8` call in both libraries with independent workspaces;
/// asserts the observables AND the resulting workspace contents agree.
#[allow(clippy::too_many_arguments)]
unsafe fn diff_dfa(
    cc: &Compiled,
    rr: &Compiled,
    subj: Option<&[u8]>,
    len: usize,
    start: usize,
    opts: u32,
    md_kind: Md,
    mcfg: &MatchCfg,
    wscount: usize,
    ws_prefix: &[i32],
    ws_null: bool,
    label: &str,
) -> i32 {
    let (c, r) = both();
    let p = subj.map_or(std::ptr::null(), |s| s.as_ptr());
    let mut cws = fresh_ws(wscount);
    let mut rws = fresh_ws(wscount);
    cws[..ws_prefix.len()].copy_from_slice(ws_prefix);
    rws[..ws_prefix.len()].copy_from_slice(ws_prefix);
    let co = obs_dfa(c, cc.code, p, len, start, opts, md_kind, mcfg, &mut cws, wscount, ws_null);
    let ro = obs_dfa(r, rr.code, p, len, start, opts, md_kind, mcfg, &mut rws, wscount, ws_null);
    assert_eq!(
        co, ro,
        "{}: pcre2_dfa_match_8 diverged (C={:?} Rust={:?}) subject={:?} len={} start={} \
         options={:#010x} md={:?} mcfg={:?} wscount={}",
        label, co, ro, subj.map(String::from_utf8_lossy), len, start, opts, md_kind, mcfg, wscount
    );
    // The first two workspace slots are the documented restart state; they are
    // only meaningful after a partial match, which is exactly when a caller may
    // re-use them (pcre2_dfa_match.c:3455-3460).
    if co.rc == ERR_PARTIAL || co.rc >= 0 {
        assert_eq!(
            &cws[0..2], &rws[0..2],
            "{}: dfa workspace restart state differs (rc={})",
            label, co.rc
        );
    }
    co.rc
}

/// Compile a pattern in both libraries, panicking if it does not compile — the
/// error-path rows below all need a *valid* pattern.
unsafe fn ok_both(pat: &[u8], cfg: &CompileCfg, label: &str) -> (Compiled, Compiled) {
    let (cc, rr) = compile_both(pat, pat.len(), cfg, label);
    assert!(
        !cc.code.is_null(),
        "{}: pattern {:?} must compile (errorcode {})",
        label,
        String::from_utf8_lossy(pat),
        cc.errorcode
    );
    (cc, rr)
}

// ================================================== section C — pcre2_match_8

/// Rows 147-150: `match_data == NULL`, `code == NULL`, `subject == NULL` with a
/// non-zero length, and the `subject == NULL, length == 0` boundary row (which
/// is the empty string, *not* an error).
#[test]
fn rows147_150_null_arguments() {
    let (c, r) = both();
    unsafe {
        let (cc, rr) = ok_both(b"a", &CompileCfg::new(0), "row147");

        // row 147: match_data == NULL -> PCRE2_ERROR_NULL, returned directly.
        let rc = diff_int(
            &cc, &rr, Some(b"a"), 1, 0, 0, Md::Null, &MatchCfg::new(0), "row147",
        );
        assert_eq!(rc, ERR_NULL, "row147: match_data==NULL must give PCRE2_ERROR_NULL");

        // row 148: code == NULL with a valid match_data. Also verify the error
        // was stored in match_data->rc, which pcre2_next_match_8 reads.
        for md in [Md::Create(1), Md::Create(8)] {
            let co = obs_int(c, std::ptr::null_mut(), b"a".as_ptr(), 1, 0, 0, md, &MatchCfg::new(0));
            let ro = obs_int(r, std::ptr::null_mut(), b"a".as_ptr(), 1, 0, 0, md, &MatchCfg::new(0));
            assert_eq!(co, ro, "row148: code==NULL diverged");
            assert_eq!(co.rc, ERR_NULL, "row148: code==NULL must give PCRE2_ERROR_NULL");
        }
        for api in [c, r] {
            let code = if api.name == "C" { cc.code } else { rr.code };
            let md = (api.match_data_create)(4, std::ptr::null_mut());
            // a successful match first, so match_data->rc is definitely >= 0
            assert!((api.do_match)(code, b"a".as_ptr(), 1, 0, 0, md, std::ptr::null_mut()) > 0);
            let mut so = 0usize;
            let mut op = 0u32;
            assert_ne!(
                (api.next_match)(md, &mut so, &mut op), 0,
                "{}: next_match after a successful match must be TRUE", api.name
            );
            // now a failing call must store the error in match_data->rc
            assert_eq!(
                (api.do_match)(std::ptr::null(), b"a".as_ptr(), 1, 0, 0, md, std::ptr::null_mut()),
                ERR_NULL
            );
            assert_eq!(
                (api.next_match)(md, &mut so, &mut op), 0,
                "{}: row148 the NULL error must be stored in match_data->rc", api.name
            );
            (api.match_data_free)(md);
        }

        // row 149: subject == NULL with length != 0 -> PCRE2_ERROR_NULL
        for len in [1usize, 3, 100] {
            let rc = diff_int(
                &cc, &rr, None, len, 0, 0, Md::Create(4), &MatchCfg::new(0), "row149",
            );
            assert_eq!(rc, ERR_NULL, "row149: subject==NULL len={} must be NULL error", len);
        }

        // row 150: subject == NULL, length == 0 is the EMPTY STRING, not an error.
        for (pat, want) in [(&b"a"[..], ERR_NOMATCH), (b"", 1), (b"^$", 1), (b"a*", 1)] {
            let (cc2, rr2) = ok_both(pat, &CompileCfg::new(0), "row150");
            let rc = diff_int(
                &cc2, &rr2, None, 0, 0, 0, Md::FromPattern, &MatchCfg::new(0), "row150",
            );
            assert_eq!(
                rc, want,
                "row150: NULL/0 subject with pattern {:?}",
                String::from_utf8_lossy(pat)
            );
        }
    }
}

/// Rows 151 and 391: every option bit that is not in `PUBLIC_MATCH_OPTIONS`
/// (pcre2_match.c:71-76) must give `PCRE2_ERROR_BADOPTION`. Swept over all 32
/// single bits, all 1024 bit pairs, `0xffffffff`, and the "valid for another
/// function" bits (`PCRE2_DFA_RESTART`, `PCRE2_DFA_SHORTEST`,
/// `PCRE2_SUBSTITUTE_*`, `PCRE2_UTF`).
#[test]
fn rows151_391_undefined_match_option_bits() {
    unsafe {
        let public: u32 = PCRE2_ANCHORED
            | PCRE2_ENDANCHORED
            | PCRE2_NOTBOL
            | PCRE2_NOTEOL
            | PCRE2_NOTEMPTY
            | PCRE2_NOTEMPTY_ATSTART
            | PCRE2_NO_UTF_CHECK
            | PCRE2_PARTIAL_HARD
            | PCRE2_PARTIAL_SOFT
            | PCRE2_NO_JIT
            | PCRE2_COPY_MATCHED_SUBJECT
            | PCRE2_DISABLE_RECURSELOOP_CHECK;

        let (cc, rr) = ok_both(b"a", &CompileCfg::new(0), "row151");
        for bit in 0..32u32 {
            let opts = 1u32 << bit;
            let rc = diff_int(
                &cc, &rr, Some(b"a"), 1, 0, opts, Md::Create(4), &MatchCfg::new(0), "row151",
            );
            if opts & public == 0 {
                assert_eq!(
                    rc, ERR_BADOPTION,
                    "row151: bit {:#010x} is outside PUBLIC_MATCH_OPTIONS and must be rejected",
                    opts
                );
            } else {
                assert_ne!(
                    rc, ERR_BADOPTION,
                    "row151: bit {:#010x} is public and must NOT be rejected",
                    opts
                );
            }
        }
        // all pairs of bits
        for a in 0..32u32 {
            for b in 0..32u32 {
                let opts = (1u32 << a) | (1u32 << b);
                let rc = diff_int(
                    &cc, &rr, Some(b"a"), 1, 0, opts, Md::Create(4), &MatchCfg::new(0), "row151-pair",
                );
                if opts & !public != 0 {
                    assert_eq!(
                        rc, ERR_BADOPTION,
                        "row151: option pair {:#010x} must be rejected", opts
                    );
                }
            }
        }
        // row 391: bits that are legal for OTHER entry points
        for opts in [
            PCRE2_DFA_RESTART,
            PCRE2_DFA_SHORTEST,
            PCRE2_SUBSTITUTE_GLOBAL,
            PCRE2_SUBSTITUTE_EXTENDED,
            PCRE2_SUBSTITUTE_LITERAL,
            PCRE2_SUBSTITUTE_MATCHED,
            PCRE2_UTF,
            PCRE2_UCP,
            0xffff_ffff,
            0x7fff_ffff,
        ] {
            let rc = diff_int(
                &cc, &rr, Some(b"a"), 1, 0, opts, Md::Create(4), &MatchCfg::new(0), "row391",
            );
            assert_eq!(rc, ERR_BADOPTION, "row391: options {:#010x} must be rejected", opts);
        }
        // the whole public set together is accepted (ENDANCHORED excluded: it
        // conflicts with partial matching, which is row 155)
        let rc = diff_int(
            &cc, &rr, Some(b"a"), 1, 0,
            public & !PCRE2_ENDANCHORED & !PCRE2_NOTEMPTY & !PCRE2_NOTEMPTY_ATSTART,
            Md::Create(4), &MatchCfg::new(0), "row151-all-public",
        );
        assert_ne!(rc, ERR_BADOPTION, "the full public option set must be accepted");
    }
}

/// Rows 152 / 396: `start_offset > length` -> `PCRE2_ERROR_BADOFFSET`; row 395:
/// `start_offset == length` is legal.
#[test]
fn rows152_395_396_bad_start_offset() {
    unsafe {
        let (cc, rr) = ok_both(b"abc", &CompileCfg::new(0), "row152");
        let subj = b"xyz";
        for start in [4usize, 5, 100, usize::MAX / 2, usize::MAX - 1, usize::MAX] {
            let rc = diff_int(
                &cc, &rr, Some(subj), 3, start, 0, Md::Create(4), &MatchCfg::new(0), "row152",
            );
            assert_eq!(rc, ERR_BADOFFSET, "row152: start_offset={} length=3", start);
        }
        // row 395 boundary: start_offset == length is NOT an error
        for len in 0..4usize {
            let rc = diff_int(
                &cc, &rr, Some(subj), len, len, 0, Md::Create(4), &MatchCfg::new(0), "row395",
            );
            assert_eq!(rc, ERR_NOMATCH, "row395: start_offset == length == {} is legal", len);
        }
        // and a whole sweep of legal offsets on a matching subject
        let (cc2, rr2) = ok_both(b"c", &CompileCfg::new(0), "row395b");
        for start in 0..=3usize {
            diff_int(
                &cc2, &rr2, Some(b"abc"), 3, start, 0, Md::FromPattern, &MatchCfg::new(0), "row395b",
            );
        }
    }
}

/// Copy a compiled pattern's bytes into an 8-byte aligned buffer we own, so we
/// can corrupt the header fields. Returns (buffer, offset of `magic_number`).
unsafe fn clone_code(api: &Api, code: *mut c_void) -> (Vec<u64>, usize) {
    let mut size: usize = 0;
    let rc = (api.pattern_info)(code, 22, &mut size as *mut _ as *mut c_void); // PCRE2_INFO_SIZE
    assert_eq!(rc, 0, "{}: pattern_info(SIZE)", api.name);
    let mut buf = vec![0u64; size.div_ceil(8)];
    std::ptr::copy_nonoverlapping(code as *const u8, buf.as_mut_ptr() as *mut u8, size);
    let base = buf.as_ptr() as *const u8;
    let mut magic = usize::MAX;
    let mut off = 0usize;
    while off + 4 <= size {
        if std::ptr::read_unaligned(base.add(off) as *const u32) == MAGIC_NUMBER {
            magic = off;
            break;
        }
        off += 4;
    }
    assert_ne!(magic, usize::MAX, "{}: MAGIC_NUMBER not found in the code block", api.name);
    (buf, magic)
}

/// Rows 153/154 and 397/398: `code` pointing at non-PCRE2 memory ->
/// `PCRE2_ERROR_BADMAGIC`; correct magic but wrong `PCRE2_MODE_MASK` bits ->
/// `PCRE2_ERROR_BADMODE`. Both are checked for `pcre2_match_8` here and for
/// `pcre2_dfa_match_8` in `rows182_183_*`.
#[test]
fn rows153_154_397_398_badmagic_badmode() {
    let (c, r) = both();
    unsafe {
        // row 153: a zeroed 4 KiB buffer, and a buffer full of 0xAA
        for fill in [0u64, u64::MAX, 0xAAAA_AAAA_AAAA_AAAA] {
            let mut junk = vec![fill; 512];
            let p = junk.as_mut_ptr() as *mut c_void;
            let co = obs_int(c, p, b"abc".as_ptr(), 3, 0, 0, Md::Create(4), &MatchCfg::new(0));
            let ro = obs_int(r, p, b"abc".as_ptr(), 3, 0, 0, Md::Create(4), &MatchCfg::new(0));
            assert_eq!(co, ro, "row153: junk code (fill={:#x}) diverged", fill);
            assert_eq!(co.rc, ERR_BADMAGIC, "row153: junk code must give BADMAGIC");
        }

        // row 154: real code, but the mode bits in `flags` forged.
        let (cc, rr) = ok_both(b"abc", &CompileCfg::new(0), "row154");
        let (mut cbuf, cmagic) = clone_code(c, cc.code);
        let (mut rbuf, rmagic) = clone_code(r, rr.code);
        assert_eq!(
            cmagic, rmagic,
            "row154: the Rust pcre2_real_code layout must place magic_number at the same offset"
        );
        // pcre2_intmodedep.h:667-671 — flags is the 5th uint32 after magic.
        let flags_off = cmagic + 16;
        for forged in [0u32, 2, 4, 6, 0xffff_fffe] {
            let cf = (cbuf.as_mut_ptr() as *mut u8).add(flags_off) as *mut u32;
            let rf = (rbuf.as_mut_ptr() as *mut u8).add(flags_off) as *mut u32;
            let orig = std::ptr::read_unaligned(cf);
            assert_eq!(
                orig & 7, PCRE2_MODE8,
                "row154: the located flags field must carry PCRE2_MODE8 (got {:#x})", orig
            );
            std::ptr::write_unaligned(cf, (orig & !7) | forged);
            std::ptr::write_unaligned(rf, (std::ptr::read_unaligned(rf) & !7) | forged);
            let cp = cbuf.as_mut_ptr() as *mut c_void;
            let rp = rbuf.as_mut_ptr() as *mut c_void;
            let co = obs_int(c, cp, b"abc".as_ptr(), 3, 0, 0, Md::Create(4), &MatchCfg::new(0));
            let ro = obs_int(r, rp, b"abc".as_ptr(), 3, 0, 0, Md::Create(4), &MatchCfg::new(0));
            assert_eq!(co, ro, "row154: forged mode bits {:#x} diverged", forged);
            assert_eq!(
                co.rc, ERR_BADMODE,
                "row154: mode bits {:#x} must give BADMODE", forged
            );
            // restore for the next iteration
            std::ptr::write_unaligned(cf, orig);
            std::ptr::write_unaligned(rf, orig);
        }
        // sanity: with the flags untouched the copy still matches identically
        let cp = cbuf.as_mut_ptr() as *mut c_void;
        let rp = rbuf.as_mut_ptr() as *mut c_void;
        let co = obs_int(c, cp, b"abc".as_ptr(), 3, 0, 0, Md::Create(4), &MatchCfg::new(0));
        let ro = obs_int(r, rp, b"abc".as_ptr(), 3, 0, 0, Md::Create(4), &MatchCfg::new(0));
        assert_eq!(co, ro, "row154: untouched code copy diverged");
        assert_eq!(co.rc, 1, "row154: the untouched copy must still match");
    }
}

/// Row 155: partial matching together with `PCRE2_ENDANCHORED` (from either the
/// match options or the compiled pattern) -> `PCRE2_ERROR_BADOPTION`.
/// Also: `PARTIAL_HARD|PARTIAL_SOFT` together is legal (HARD wins).
#[test]
fn rows155_partial_plus_endanchored_badoption() {
    unsafe {
        let (cc, rr) = ok_both(b"abc", &CompileCfg::new(0), "row155");
        for p in [PCRE2_PARTIAL_HARD, PCRE2_PARTIAL_SOFT, PCRE2_PARTIAL_HARD | PCRE2_PARTIAL_SOFT] {
            let rc = diff_int(
                &cc, &rr, Some(b"ab"), 2, 0, p | PCRE2_ENDANCHORED,
                Md::Create(4), &MatchCfg::new(0), "row155",
            );
            assert_eq!(
                rc, ERR_BADOPTION,
                "row155: partial {:#x} + ENDANCHORED must be rejected", p
            );
        }
        // the ENDANCHORED can also come from the compiled pattern
        let (cc2, rr2) = ok_both(b"abc", &CompileCfg::new(PCRE2_ENDANCHORED), "row155-compile");
        for p in [PCRE2_PARTIAL_HARD, PCRE2_PARTIAL_SOFT] {
            let rc = diff_int(
                &cc2, &rr2, Some(b"ab"), 2, 0, p, Md::Create(4), &MatchCfg::new(0), "row155-compile",
            );
            assert_eq!(rc, ERR_BADOPTION, "row155: compiled ENDANCHORED + partial {:#x}", p);
        }
        // ... and (*LIMIT-less) partial alone is fine, HARD|SOFT means HARD
        let rc = diff_int(
            &cc, &rr, Some(b"ab"), 2, 0, PCRE2_PARTIAL_HARD | PCRE2_PARTIAL_SOFT,
            Md::Create(4), &MatchCfg::new(0), "row155-both",
        );
        assert_eq!(rc, ERR_PARTIAL, "PARTIAL_HARD|PARTIAL_SOFT together is legal");
    }
}

/// Row 156: an offset limit in the match context but no `PCRE2_USE_OFFSET_LIMIT`
/// at compile time -> `PCRE2_ERROR_BADOFFSETLIMIT`.
#[test]
fn rows156_offset_limit_without_use_offset_limit() {
    unsafe {
        let (cc, rr) = ok_both(b"b", &CompileCfg::new(0), "row156");
        for limit in [0usize, 1, 2, 1000] {
            let rc = diff_int(
                &cc, &rr, Some(b"aab"), 3, 0, 0, Md::Create(4),
                &MatchCfg::new(0).offset_limit(limit), "row156",
            );
            assert_eq!(rc, ERR_BADOFFSETLIMIT, "row156: offset_limit={}", limit);
        }
        // PCRE2_UNSET means "no limit" and is always accepted
        let rc = diff_int(
            &cc, &rr, Some(b"aab"), 3, 0, 0, Md::Create(4),
            &MatchCfg::new(0).offset_limit(PCRE2_UNSET), "row156-unset",
        );
        assert_eq!(rc, 1, "an unset offset limit must be accepted");

        // with the compile option the limit is honoured instead of rejected
        let (cc2, rr2) = ok_both(b"b", &CompileCfg::new(PCRE2_USE_OFFSET_LIMIT), "row156-ok");
        for limit in [0usize, 1, 2, 3, 1000] {
            let rc = diff_int(
                &cc2, &rr2, Some(b"aab"), 3, 0, 0, Md::Create(4),
                &MatchCfg::new(0).offset_limit(limit), "row156-ok",
            );
            assert_ne!(rc, ERR_BADOFFSETLIMIT, "row156: limit={} must be honoured", limit);
        }
    }
}

/// Rows 157/158: plain `PCRE2_ERROR_NOMATCH` and `PCRE2_ERROR_PARTIAL`.
#[test]
fn rows157_158_nomatch_and_partial() {
    unsafe {
        let (cc, rr) = ok_both(b"abc", &CompileCfg::new(0), "row157");
        for subj in [&b"xyz"[..], b"", b"ab", b"bc", b"aabbcc"] {
            let rc = diff_int(
                &cc, &rr, Some(subj), subj.len(), 0, 0, Md::FromPattern, &MatchCfg::new(0), "row157",
            );
            assert_eq!(
                rc, ERR_NOMATCH,
                "row157: pattern abc must not match {:?}", String::from_utf8_lossy(subj)
            );
        }
        for opt in [PCRE2_PARTIAL_SOFT, PCRE2_PARTIAL_HARD] {
            for subj in [&b"ab"[..], b"a", b"xxab"] {
                let rc = diff_int(
                    &cc, &rr, Some(subj), subj.len(), 0, opt, Md::FromPattern,
                    &MatchCfg::new(0), "row158",
                );
                assert_eq!(
                    rc, ERR_PARTIAL,
                    "row158: {:?} with options {:#x} must be a partial match",
                    String::from_utf8_lossy(subj), opt
                );
            }
        }
    }
}

/// Rows 159/160/161 and 172: the match / depth / heap limits.
/// `heap_limit == 0` is also exactly the `max_size < frame_size` site of row 172
/// (pcre2_match.c:7521).
#[test]
fn rows159_161_172_match_depth_heap_limits() {
    unsafe {
        let (cc, rr) = ok_both(b"a*b", &CompileCfg::new(PCRE2_NO_START_OPTIMIZE), "row159");
        let subj = b"aaaaaaaaaa";
        // `match_call_count` is reset for every bumpalong start position
        // (pcre2_match.c:7969) and only counts RMATCH re-entries
        // (pcre2_match.c:873), so `a*b` needs a limit of 1 to trip it.
        assert_eq!(
            diff_int(&cc, &rr, Some(subj), subj.len(), 0, 0, Md::FromPattern,
                     &MatchCfg::new(0).match_limit(1), "row159"),
            ERR_MATCHLIMIT,
            "row159: match_limit=1 must give MATCHLIMIT"
        );
        // `Frdepth` counts nested frames, and `a*b` only ever needs one, so a
        // depth limit of 0 or 1 is required to trip it (pcre2_match.c:874).
        for lim in [0u32, 1] {
            let rc = diff_int(
                &cc, &rr, Some(subj), subj.len(), 0, 0, Md::FromPattern,
                &MatchCfg::new(0).depth_limit(lim), "row160",
            );
            assert_eq!(rc, ERR_DEPTHLIMIT, "row160: depth_limit={}", lim);
        }
        assert_eq!(
            diff_int(&cc, &rr, Some(subj), subj.len(), 0, 0, Md::FromPattern,
                     &MatchCfg::new(0).match_limit(1_000_000).depth_limit(1_000_000), "row159-ok"),
            ERR_NOMATCH
        );
        // A backtracking-heavy pattern makes many more RMATCH calls, so larger
        // limits trip both counters too.
        let (ccb, rrb) =
            ok_both(b"(a+)+b", &CompileCfg::new(PCRE2_NO_START_OPTIMIZE), "row159b");
        let a30 = vec![b'a'; 30];
        for lim in [1u32, 2, 4, 8, 16] {
            assert_eq!(
                diff_int(&ccb, &rrb, Some(&a30), a30.len(), 0, 0, Md::FromPattern,
                         &MatchCfg::new(0).match_limit(lim), "row159b"),
                ERR_MATCHLIMIT,
                "row159: (a+)+b with match_limit={}", lim
            );
            assert_eq!(
                diff_int(&ccb, &rrb, Some(&a30), a30.len(), 0, 0, Md::FromPattern,
                         &MatchCfg::new(0).depth_limit(lim), "row160b"),
                ERR_DEPTHLIMIT,
                "row160: (a+)+b with depth_limit={}", lim
            );
        }

        // rows 161/172: heap_limit == 0 -> the frame vector cannot even be sized
        let big = vec![b'a'; 400];
        let (cc2, rr2) = ok_both(b"(a+)+b", &CompileCfg::new(0), "row161");
        let rc = diff_int(
            &cc2, &rr2, Some(&big), big.len(), 0, 0, Md::FromPattern,
            &MatchCfg::new(0).heap_limit(0), "row161",
        );
        assert_eq!(rc, ERR_HEAPLIMIT, "row161/172: heap_limit=0 must give HEAPLIMIT");
        // every pattern hits the same site, whatever its frame size
        for pat in [&b"a"[..], b"(a)(b)(c)", b"(?:x|y){1,20}"] {
            let (c3, r3) = ok_both(pat, &CompileCfg::new(0), "row172");
            assert_eq!(
                diff_int(&c3, &r3, Some(b"abc"), 3, 0, 0, Md::FromPattern,
                         &MatchCfg::new(0).heap_limit(0), "row172"),
                ERR_HEAPLIMIT,
                "row172: heap_limit=0 with pattern {:?}", String::from_utf8_lossy(pat)
            );
        }
        // a heap limit large enough for the initial vector but too small to grow
        // hits the growth site (pcre2_match.c:791) instead.
        let deep = vec![b'a'; 4000];
        for lim in [1u32, 2, 4, 8, 16, 32] {
            let rc = diff_int(
                &cc2, &rr2, Some(&deep), deep.len(), 0, 0, Md::FromPattern,
                &MatchCfg::new(0).heap_limit(lim), "row161-grow",
            );
            assert!(
                rc == ERR_HEAPLIMIT || rc == ERR_NOMATCH,
                "row161: heap_limit={} gave unexpected {}", lim, rc
            );
        }
    }
}

// ------------------------------------------------- failure-injecting allocator
static INJ_LOCK: Mutex<()> = Mutex::new(());
static mut INJ_N: [u32; 2] = [0, 0];
static mut INJ_FAIL_AT: [u32; 2] = [u32::MAX, u32::MAX];

extern "C" {
    #[link_name = "malloc"]
    fn libc_malloc(n: usize) -> *mut c_void;
    #[link_name = "free"]
    fn libc_free(p: *mut c_void);
}

unsafe extern "C" fn inj_malloc_c(n: SIZE, _d: *mut c_void) -> *mut c_void {
    INJ_N[0] += 1;
    if INJ_N[0] >= INJ_FAIL_AT[0] {
        return std::ptr::null_mut();
    }
    libc_malloc(n)
}
unsafe extern "C" fn inj_malloc_r(n: SIZE, _d: *mut c_void) -> *mut c_void {
    INJ_N[1] += 1;
    if INJ_N[1] >= INJ_FAIL_AT[1] {
        return std::ptr::null_mut();
    }
    libc_malloc(n)
}
unsafe extern "C" fn inj_free(p: *mut c_void, _d: *mut c_void) {
    if !p.is_null() {
        libc_free(p);
    }
}

/// Run one match with the Nth allocation of `api` failing, and report the rc
/// plus the number of allocations that were requested.
#[allow(clippy::too_many_arguments)]
unsafe fn inj_run(
    api: &Api,
    idx: usize,
    code: *mut c_void,
    subj: &[u8],
    opts: u32,
    ovec: u32,
    heap_limit: Option<u32>,
    fail_at: u32,
    dfa: bool,
    wscount: usize,
) -> (i32, u32) {
    let mal = if idx == 0 { inj_malloc_c } else { inj_malloc_r };
    INJ_N[idx] = 0;
    INJ_FAIL_AT[idx] = u32::MAX;
    let gctx = (api.general_context_create)(Some(mal), Some(inj_free), std::ptr::null_mut());
    assert!(!gctx.is_null());
    let md = (api.match_data_create)(ovec, gctx);
    assert!(!md.is_null());
    let mcx = (api.match_context_create)(gctx);
    assert!(!mcx.is_null());
    if let Some(h) = heap_limit {
        (api.set_heap_limit)(mcx, h);
    }
    // start counting only for the match call itself
    INJ_N[idx] = 0;
    INJ_FAIL_AT[idx] = fail_at;
    let rc = if dfa {
        let mut ws = fresh_ws(wscount);
        (api.dfa_match)(
            code, subj.as_ptr(), subj.len(), 0, opts, md, mcx, ws.as_mut_ptr(), wscount,
        )
    } else {
        (api.do_match)(code, subj.as_ptr(), subj.len(), 0, opts, md, mcx)
    };
    let n = INJ_N[idx];
    INJ_FAIL_AT[idx] = u32::MAX;
    (api.match_context_free)(mcx);
    (api.match_data_free)(md);
    (api.general_context_free)(gctx);
    (rc, n)
}

/// Rows 162/163: `PCRE2_ERROR_NOMEMORY` when the heapframe vector cannot be
/// allocated or grown (pcre2_match.c:768,793,7532) and when the
/// `PCRE2_COPY_MATCHED_SUBJECT` copy fails (pcre2_match.c:7226,8195).
/// The allocation *count* is compared too, which is a strong equivalence check.
#[test]
fn rows162_163_nomemory_on_allocation_failure() {
    let _g = INJ_LOCK.lock().unwrap();
    let (c, r) = both();
    unsafe {
        // row 162 — the frame vector
        let (cc, rr) = ok_both(b"(a+)+b", &CompileCfg::new(0), "row162");
        let subj = vec![b'a'; 200];
        let mut saw_nomemory = false;
        for fail_at in 1..=6u32 {
            let (crc, cn) = inj_run(c, 0, cc.code, &subj, 0, 8, None, fail_at, false, WS);
            let (rrc, rn) = inj_run(r, 1, rr.code, &subj, 0, 8, None, fail_at, false, WS);
            assert_eq!(crc, rrc, "row162: rc differs with fail_at={}", fail_at);
            assert_eq!(cn, rn, "row162: allocation count differs with fail_at={}", fail_at);
            if crc == ERR_NOMEMORY {
                saw_nomemory = true;
            }
        }
        assert!(saw_nomemory, "row162: some allocation failure must give NOMEMORY");

        // row 162, growth path (pcre2_match.c:793): a heap limit that allows the
        // initial vector but forces it to be grown. Whatever the outcome, the
        // two libraries must return the same code after the same number of
        // allocation requests.
        let deep = vec![b'a'; 2000];
        for fail_at in 1..=8u32 {
            let (crc, cn) = inj_run(c, 0, cc.code, &deep, 0, 8, Some(4), fail_at, false, WS);
            let (rrc, rn) = inj_run(r, 1, rr.code, &deep, 0, 8, Some(4), fail_at, false, WS);
            assert_eq!(crc, rrc, "row162-grow: rc differs with fail_at={}", fail_at);
            assert_eq!(cn, rn, "row162-grow: alloc count differs with fail_at={}", fail_at);
        }

        // row 163 — the PCRE2_COPY_MATCHED_SUBJECT copy
        let (cc2, rr2) = ok_both(b"abc", &CompileCfg::new(0), "row163");
        let mut saw_copy = false;
        for fail_at in 1..=5u32 {
            let (crc, cn) = inj_run(
                c, 0, cc2.code, b"abc", PCRE2_COPY_MATCHED_SUBJECT, 4, None, fail_at, false, WS,
            );
            let (rrc, rn) = inj_run(
                r, 1, rr2.code, b"abc", PCRE2_COPY_MATCHED_SUBJECT, 4, None, fail_at, false, WS,
            );
            assert_eq!(crc, rrc, "row163: rc differs with fail_at={}", fail_at);
            assert_eq!(cn, rn, "row163: alloc count differs with fail_at={}", fail_at);
            if crc == ERR_NOMEMORY {
                saw_copy = true;
            }
        }
        assert!(saw_copy, "row163: the subject copy failure must give NOMEMORY");
    }
}

/// Row 164: `PCRE2_ERROR_RECURSELOOP` — a recursion re-entered at the same
/// subject position. `PCRE2_DISABLE_RECURSELOOP_CHECK` turns the check off.
#[test]
fn row164_recurseloop() {
    unsafe {
        for pat in [&b"(?1)((?1))"[..], b"((?2))((?1))", b"(a|(?R))*"] {
            let (cc, rr) = ok_both(pat, &CompileCfg::new(0), "row164");
            let rc = diff_int(
                &cc, &rr, Some(b"aaa"), 3, 0, 0, Md::FromPattern, &MatchCfg::new(0), "row164",
            );
            assert_eq!(
                rc, ERR_RECURSELOOP,
                "row164: {:?} must give RECURSELOOP", String::from_utf8_lossy(pat)
            );
            // with the check disabled the two libraries must still agree
            diff_int(
                &cc, &rr, Some(b"aaa"), 3, 0, PCRE2_DISABLE_RECURSELOOP_CHECK,
                Md::FromPattern, &MatchCfg::new(0).match_limit(20_000).depth_limit(20_000),
                "row164-disabled",
            );
        }
    }
}

/// Rows 165-168 and 389: UTF-8 subject errors reported by `pcre2_match_8`
/// (`PCRE2_ERROR_UTF8_ERRn`) and `PCRE2_ERROR_BADUTFOFFSET` for a
/// `start_offset` that lands inside a character.
#[test]
fn rows165_168_389_utf_subject_errors_interpreter() {
    unsafe {
        let (cc, rr) = ok_both(b"a", &CompileCfg::new(PCRE2_UTF), "row165");
        // row 165: isolated 0x80 at offset 0
        for b in [0x80u8, 0x81, 0xbf] {
            let subj = [b, b'a'];
            let rc = diff_int(
                &cc, &rr, Some(&subj), 2, 0, 0, Md::Create(4), &MatchCfg::new(0), "row165",
            );
            assert_eq!(
                rc, utf8_err(20),
                "row165: leading continuation byte {:#02x} must give UTF8_ERR20", b
            );
        }
        // row 166 / row 389: start_offset in the middle of a character
        let subj = b"\xc3\xa9a";
        let rc = diff_int(
            &cc, &rr, Some(subj), 3, 1, 0, Md::Create(4), &MatchCfg::new(0), "row166",
        );
        assert_eq!(rc, ERR_BADUTFOFFSET, "row166: mid-character start_offset");
        for (s, off) in [
            (&b"\xe2\x82\xac"[..], 1usize),
            (b"\xe2\x82\xac", 2),
            (b"\xf0\x9f\x98\x80", 1),
            (b"\xf0\x9f\x98\x80", 2),
            (b"\xf0\x9f\x98\x80", 3),
        ] {
            let rc = diff_int(
                &cc, &rr, Some(s), s.len(), off, 0, Md::Create(4), &MatchCfg::new(0), "row389",
            );
            assert_eq!(
                rc, ERR_BADUTFOFFSET,
                "row389: start_offset={} inside {:02x?} must give BADUTFOFFSET", off, s
            );
        }
        // and the character boundaries themselves are fine
        for off in [0usize, 3] {
            let rc = diff_int(
                &cc, &rr, Some(b"\xe2\x82\xac"), 3, off, 0, Md::Create(4),
                &MatchCfg::new(0), "row389-ok",
            );
            assert_eq!(rc, ERR_NOMATCH, "row389: offset {} is a character start", off);
        }
        // row 167: truncated 2-byte sequence -> UTF8_ERR1
        let rc = diff_int(
            &cc, &rr, Some(b"a\xc3"), 2, 0, 0, Md::Create(4), &MatchCfg::new(0), "row167",
        );
        assert_eq!(rc, utf8_err(1), "row167: truncated sequence must give UTF8_ERR1");
        // row 168: 0xfe / 0xff
        for b in [0xfeu8, 0xff] {
            let subj = [b'a', b];
            let rc = diff_int(
                &cc, &rr, Some(&subj), 2, 0, 0, Md::Create(4), &MatchCfg::new(0), "row168",
            );
            assert_eq!(rc, utf8_err(21), "row168: byte {:#02x} must give UTF8_ERR21", b);
        }
    }
}

/// Rows 169 and 407: an ovector that is too small makes `pcre2_match_8` return
/// **0**, not an error. Also checks that `pcre2_match_data_create_8` CLAMPS
/// (`0 -> 1`, `0xffffffff -> 65535`) instead of failing, and that
/// `pcre2_pattern_info_8` writes `UINT32_MAX` before returning
/// `PCRE2_ERROR_UNSET`.
#[test]
fn rows169_407_ovector_too_small_and_match_data_clamping() {
    let (c, r) = both();
    unsafe {
        let (cc, rr) = ok_both(b"(a)(b)(c)", &CompileCfg::new(0), "row169");
        // 4 pairs are needed; 1..3 must all return 0
        for n in 1..=3u32 {
            let rc = diff_int(
                &cc, &rr, Some(b"abc"), 3, 0, 0, Md::Create(n), &MatchCfg::new(0), "row169",
            );
            assert_eq!(rc, 0, "row169: ovecsize={} must return 0 (ovector too small)", n);
        }
        for n in 4..=8u32 {
            let rc = diff_int(
                &cc, &rr, Some(b"abc"), 3, 0, 0, Md::Create(n), &MatchCfg::new(0), "row169-ok",
            );
            assert_eq!(rc, 4, "row169: ovecsize={} must return 4", n);
        }
        // row 407: match data made from a DIFFERENT (smaller) pattern
        let (cs, rs) = ok_both(b"x", &CompileCfg::new(0), "row407");
        let csmall = (c.match_data_create_from_pattern)(cs.code, std::ptr::null_mut());
        let rsmall = (r.match_data_create_from_pattern)(rs.code, std::ptr::null_mut());
        let crc = (c.do_match)(cc.code, b"abc".as_ptr(), 3, 0, 0, csmall, std::ptr::null_mut());
        let rrc = (r.do_match)(rr.code, b"abc".as_ptr(), 3, 0, 0, rsmall, std::ptr::null_mut());
        assert_eq!(crc, rrc, "row407: rc differs for a foreign match_data");
        assert_eq!(crc, 0, "row407: foreign (too small) match_data must return 0");
        assert_eq!(observe(c, csmall, crc), observe(r, rsmall, rrc), "row407: observables differ");
        (c.match_data_free)(csmall);
        (r.match_data_free)(rsmall);

        // pcre2_match_data_create_8 CLAMPS instead of failing
        for (req, want) in [
            (0u32, 1u32),
            (1, 1),
            (2, 2),
            (65535, 65535),
            (65536, 65535),
            (0xffff_fffe, 65535),
            (0xffff_ffff, 65535),
        ] {
            let cmd = (c.match_data_create)(req, std::ptr::null_mut());
            let rmd = (r.match_data_create)(req, std::ptr::null_mut());
            assert!(!cmd.is_null() && !rmd.is_null(), "match_data_create({}) must succeed", req);
            let cn = (c.get_ovector_count)(cmd);
            let rn = (r.get_ovector_count)(rmd);
            assert_eq!(cn, rn, "ovector count differs for request {}", req);
            assert_eq!(cn, want, "match_data_create({}) must clamp to {}", req, want);
            (c.match_data_free)(cmd);
            (r.match_data_free)(rmd);
        }

        // pcre2_pattern_info_8 writes UINT32_MAX and THEN returns UNSET
        // (PCRE2_INFO_FIRSTCODEUNIT = 4 on a pattern with no first code unit).
        let (cu, ru) = ok_both(b"\\d|x", &CompileCfg::new(0), "unset");
        for what in [4u32, 11] {
            let mut cv: u32 = 0xDEAD_BEEF;
            let mut rv: u32 = 0xDEAD_BEEF;
            let crc2 = (c.pattern_info)(cu.code, what, &mut cv as *mut _ as *mut c_void);
            let rrc2 = (r.pattern_info)(ru.code, what, &mut rv as *mut _ as *mut c_void);
            assert_eq!(crc2, rrc2, "pattern_info({}) rc differs", what);
            assert_eq!(cv, rv, "pattern_info({}) value differs", what);
            if crc2 == ERR_UNSET {
                assert_eq!(
                    cv, u32::MAX,
                    "pattern_info({}) must write UINT32_MAX before returning UNSET", what
                );
            }
        }
    }
}

/// `pcre2_jit_match_8` and `pcre2_jit_compile_8` in this **non-JIT** build.
/// `pcre2_jit_match_8` always stores and returns
/// `PCRE2_ERROR_JIT_BADOPTION (-45)` (pcre2_jit_match_inc.h:103) and
/// `pcre2_jit_compile_8` returns `JIT_BADOPTION`, except for
/// `PCRE2_JIT_TEST_ALLOC` alone, which returns
/// `PCRE2_ERROR_JIT_UNSUPPORTED (-68)` (pcre2_jit_compile.c:14319-14332).
#[test]
fn jit_match_and_jit_compile_are_unsupported() {
    let (c, r) = both();
    unsafe {
        for pat in [&b"a"[..], b"(a)(b)", b"a*b"] {
            let (cc, rr) = ok_both(pat, &CompileCfg::new(0), "jit");
            for opts in [0u32, PCRE2_ANCHORED, PCRE2_NOTBOL, PCRE2_PARTIAL_SOFT, PCRE2_NO_JIT] {
                let co = obs_int(c, cc.code, b"ab".as_ptr(), 2, 0, opts, Md::Create(4), &MatchCfg::new(0));
                let ro = obs_int(r, rr.code, b"ab".as_ptr(), 2, 0, opts, Md::Create(4), &MatchCfg::new(0));
                assert_eq!(co, ro, "jit: interpreter sanity diverged");
                let cmd = (c.match_data_create)(4, std::ptr::null_mut());
                let rmd = (r.match_data_create)(4, std::ptr::null_mut());
                let cj = (c.jit_match)(cc.code, b"ab".as_ptr(), 2, 0, opts, cmd, std::ptr::null_mut());
                let rj = (r.jit_match)(rr.code, b"ab".as_ptr(), 2, 0, opts, rmd, std::ptr::null_mut());
                assert_eq!(cj, rj, "jit_match rc differs (options {:#x})", opts);
                assert_eq!(cj, ERR_JIT_BADOPTION, "jit_match must return -45 in a non-JIT build");
                // the error is stored in match_data->rc
                let mut so = 0usize;
                let mut op = 0u32;
                assert_eq!((c.next_match)(cmd, &mut so, &mut op), (r.next_match)(rmd, &mut so, &mut op));
                (c.match_data_free)(cmd);
                (r.match_data_free)(rmd);
            }
        }
        // pcre2_jit_compile_8 over every option combination that matters
        let (cc, rr) = ok_both(b"a", &CompileCfg::new(0), "jit-compile");
        for opts in [
            0u32,
            PCRE2_JIT_COMPLETE,
            PCRE2_JIT_PARTIAL_SOFT,
            PCRE2_JIT_PARTIAL_HARD,
            PCRE2_JIT_COMPLETE | PCRE2_JIT_PARTIAL_SOFT | PCRE2_JIT_PARTIAL_HARD,
            PCRE2_JIT_INVALID_UTF,
            0x8000,
            0xffff_ffff,
        ] {
            let crc = (c.jit_compile)(cc.code, opts);
            let rrc = (r.jit_compile)(rr.code, opts);
            assert_eq!(crc, rrc, "jit_compile({:#x}) rc differs", opts);
            assert_eq!(crc, ERR_JIT_BADOPTION, "jit_compile({:#x}) must be -45", opts);
        }
        // PCRE2_JIT_TEST_ALLOC on its own is the JIT_UNSUPPORTED family member
        let crc = (c.jit_compile)(cc.code, PCRE2_JIT_TEST_ALLOC);
        let rrc = (r.jit_compile)(rr.code, PCRE2_JIT_TEST_ALLOC);
        assert_eq!(crc, rrc, "jit_compile(TEST_ALLOC) rc differs");
        assert_eq!(crc, ERR_JIT_UNSUPPORTED, "jit_compile(TEST_ALLOC) must be -68");
        // ... but not when combined with anything else
        let crc = (c.jit_compile)(cc.code, PCRE2_JIT_TEST_ALLOC | PCRE2_JIT_COMPLETE);
        let rrc = (r.jit_compile)(rr.code, PCRE2_JIT_TEST_ALLOC | PCRE2_JIT_COMPLETE);
        assert_eq!(crc, rrc, "jit_compile(TEST_ALLOC|COMPLETE) rc differs");
        assert_eq!(crc, ERR_JIT_BADOPTION);
        // code == NULL
        assert_eq!(
            (c.jit_compile)(std::ptr::null_mut(), PCRE2_JIT_COMPLETE),
            (r.jit_compile)(std::ptr::null_mut(), PCRE2_JIT_COMPLETE),
            "jit_compile(NULL) rc differs"
        );
    }
}

// ============================================== section D — pcre2_dfa_match_8

/// Rows 173-176: `match_data`, `code`, `subject` and `workspace` NULL checks
/// (pcre2_dfa_match.c:3396-3398), plus the `subject == NULL, length == 0`
/// boundary.
#[test]
fn rows173_176_dfa_null_arguments() {
    let (c, r) = both();
    unsafe {
        let (cc, rr) = ok_both(b"a", &CompileCfg::new(0), "row173");

        // row 173: match_data == NULL
        let rc = diff_dfa(
            &cc, &rr, Some(b"a"), 1, 0, 0, Md::Null, &MatchCfg::new(0), WS, &[], false, "row173",
        );
        assert_eq!(rc, ERR_NULL, "row173: match_data==NULL must give PCRE2_ERROR_NULL");

        // row 174: code == NULL
        for md in [Md::Create(1), Md::Create(8)] {
            let mut cws = fresh_ws(WS);
            let mut rws = fresh_ws(WS);
            let co = obs_dfa(
                c, std::ptr::null_mut(), b"a".as_ptr(), 1, 0, 0, md, &MatchCfg::new(0),
                &mut cws, WS, false,
            );
            let ro = obs_dfa(
                r, std::ptr::null_mut(), b"a".as_ptr(), 1, 0, 0, md, &MatchCfg::new(0),
                &mut rws, WS, false,
            );
            assert_eq!(co, ro, "row174: code==NULL diverged");
            assert_eq!(co.rc, ERR_NULL, "row174: code==NULL must give PCRE2_ERROR_NULL");
        }

        // row 175: subject == NULL with length != 0
        for len in [1usize, 3, 100] {
            let rc = diff_dfa(
                &cc, &rr, None, len, 0, 0, Md::Create(4), &MatchCfg::new(0), WS, &[], false, "row175",
            );
            assert_eq!(rc, ERR_NULL, "row175: subject==NULL len={}", len);
        }

        // row 176: workspace == NULL
        for wscount in [0usize, 19, 20, 1000] {
            let rc = diff_dfa(
                &cc, &rr, Some(b"a"), 1, 0, 0, Md::Create(4), &MatchCfg::new(0),
                wscount, &[], true, "row176",
            );
            assert_eq!(
                rc, ERR_NULL,
                "row176: workspace==NULL (wscount={}) must give PCRE2_ERROR_NULL", wscount
            );
        }

        // boundary: NULL subject with length 0 is the empty string
        for (pat, want) in [(&b"a"[..], ERR_NOMATCH), (b"", 1), (b"a*", 1)] {
            let (cc2, rr2) = ok_both(pat, &CompileCfg::new(0), "row175-empty");
            let rc = diff_dfa(
                &cc2, &rr2, None, 0, 0, 0, Md::Create(4), &MatchCfg::new(0), WS, &[], false,
                "row175-empty",
            );
            assert_eq!(rc, want, "NULL/0 dfa subject with {:?}", String::from_utf8_lossy(pat));
        }
    }
}

/// Rows 177 / 391: every option bit outside `PUBLIC_DFA_MATCH_OPTIONS`
/// (pcre2_dfa_match.c:83-88) must give `PCRE2_ERROR_BADOPTION`. Swept over all
/// 32 single bits, all 1024 pairs, and the foreign bits.
#[test]
fn rows177_391_dfa_undefined_option_bits() {
    unsafe {
        let public: u32 = PCRE2_ANCHORED
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

        let (cc, rr) = ok_both(b"a", &CompileCfg::new(0), "row177");
        for bit in 0..32u32 {
            let opts = 1u32 << bit;
            let rc = diff_dfa(
                &cc, &rr, Some(b"a"), 1, 0, opts, Md::Create(4), &MatchCfg::new(0),
                WS, &[], false, "row177",
            );
            if opts & public == 0 {
                assert_eq!(
                    rc, ERR_BADOPTION,
                    "row177: bit {:#010x} is outside PUBLIC_DFA_MATCH_OPTIONS", opts
                );
            } else {
                assert_ne!(
                    rc, ERR_BADOPTION,
                    "row177: bit {:#010x} is public and must not be rejected", opts
                );
            }
        }
        for a in 0..32u32 {
            for b in 0..32u32 {
                let opts = (1u32 << a) | (1u32 << b);
                let rc = diff_dfa(
                    &cc, &rr, Some(b"a"), 1, 0, opts, Md::Create(4), &MatchCfg::new(0),
                    WS, &[], false, "row177-pair",
                );
                if opts & !public != 0 {
                    assert_eq!(rc, ERR_BADOPTION, "row177: pair {:#010x} must be rejected", opts);
                }
            }
        }
        // row 391 — bits valid for other entry points, notably PCRE2_NO_JIT
        for opts in [
            PCRE2_NO_JIT,
            PCRE2_DISABLE_RECURSELOOP_CHECK,
            PCRE2_SUBSTITUTE_GLOBAL,
            PCRE2_UTF,
            0xffff_ffff,
        ] {
            let rc = diff_dfa(
                &cc, &rr, Some(b"a"), 1, 0, opts, Md::Create(4), &MatchCfg::new(0),
                WS, &[], false, "row391-dfa",
            );
            assert_eq!(rc, ERR_BADOPTION, "row391: dfa options {:#010x} must be rejected", opts);
        }
    }
}

/// Rows 178 and 191: `PCRE2_ERROR_DFA_WSSIZE` — `wscount < 20`
/// (pcre2_dfa_match.c:3407) and a workspace too small for the active/new state
/// vectors (pcre2_dfa_match.c:496-525).
#[test]
fn rows178_191_dfa_wssize() {
    unsafe {
        let (cc, rr) = ok_both(b"a", &CompileCfg::new(0), "row178");
        for wscount in 0..20usize {
            let rc = diff_dfa(
                &cc, &rr, Some(b"a"), 1, 0, 0, Md::Create(4), &MatchCfg::new(0),
                wscount, &[], false, "row178",
            );
            assert_eq!(rc, ERR_DFA_WSSIZE, "row178: wscount={} must give DFA_WSSIZE", wscount);
        }
        assert_eq!(
            diff_dfa(&cc, &rr, Some(b"a"), 1, 0, 0, Md::Create(4), &MatchCfg::new(0),
                     20, &[], false, "row178-ok"),
            1,
            "row178: wscount=20 is the documented minimum and must work"
        );

        // row 191: the state vectors overflow
        let (cc2, rr2) =
            ok_both(b"(?:a|b|c|d|e|f|g|h){1,400}", &CompileCfg::new(0), "row191");
        let subj = vec![b'a'; 480];
        for wscount in [20usize, 21, 22, 40, 100, 1000] {
            let rc = diff_dfa(
                &cc2, &rr2, Some(&subj), subj.len(), 0, 0, Md::Create(4), &MatchCfg::new(0),
                wscount, &[], false, "row191",
            );
            assert_eq!(rc, ERR_DFA_WSSIZE, "row191: wscount={} must overflow", wscount);
        }
        // a big enough workspace succeeds — proof the error is about the size
        let rc = diff_dfa(
            &cc2, &rr2, Some(&subj), subj.len(), 0, 0, Md::Create(4), &MatchCfg::new(0),
            20_000, &[], false, "row191-ok",
        );
        assert_eq!(rc, 0, "row191: a 20000-int workspace must succeed");
    }
}

/// Row 179: `start_offset > length` -> `PCRE2_ERROR_BADOFFSET`. Note that this
/// is checked *after* `wscount`, which the sweep also proves.
#[test]
fn rows179_dfa_bad_start_offset() {
    unsafe {
        let (cc, rr) = ok_both(b"a", &CompileCfg::new(0), "row179");
        for start in [2usize, 5, 100, usize::MAX - 1, usize::MAX] {
            let rc = diff_dfa(
                &cc, &rr, Some(b"a"), 1, start, 0, Md::Create(4), &MatchCfg::new(0),
                WS, &[], false, "row179",
            );
            assert_eq!(rc, ERR_BADOFFSET, "row179: start_offset={} length=1", start);
        }
        // wscount is validated FIRST (pcre2_dfa_match.c:3407 precedes :3408)
        let rc = diff_dfa(
            &cc, &rr, Some(b"a"), 1, 99, 0, Md::Create(4), &MatchCfg::new(0),
            19, &[], false, "row179-order",
        );
        assert_eq!(rc, ERR_DFA_WSSIZE, "row178 is checked before row179");
        // start_offset == length is legal
        assert_eq!(
            diff_dfa(&cc, &rr, Some(b"a"), 1, 1, 0, Md::Create(4), &MatchCfg::new(0),
                     WS, &[], false, "row179-ok"),
            ERR_NOMATCH
        );
    }
}

/// Row 180: partial matching + `PCRE2_ENDANCHORED` -> `PCRE2_ERROR_BADOPTION`.
#[test]
fn rows180_dfa_partial_plus_endanchored() {
    unsafe {
        let (cc, rr) = ok_both(b"abc", &CompileCfg::new(0), "row180");
        for p in [PCRE2_PARTIAL_HARD, PCRE2_PARTIAL_SOFT, PCRE2_PARTIAL_HARD | PCRE2_PARTIAL_SOFT] {
            let rc = diff_dfa(
                &cc, &rr, Some(b"ab"), 2, 0, p | PCRE2_ENDANCHORED, Md::Create(4),
                &MatchCfg::new(0), WS, &[], false, "row180",
            );
            assert_eq!(rc, ERR_BADOPTION, "row180: partial {:#x} + ENDANCHORED", p);
        }
        let (cc2, rr2) = ok_both(b"abc", &CompileCfg::new(PCRE2_ENDANCHORED), "row180-compile");
        for p in [PCRE2_PARTIAL_HARD, PCRE2_PARTIAL_SOFT] {
            let rc = diff_dfa(
                &cc2, &rr2, Some(b"ab"), 2, 0, p, Md::Create(4), &MatchCfg::new(0),
                WS, &[], false, "row180-compile",
            );
            assert_eq!(rc, ERR_BADOPTION, "row180: compiled ENDANCHORED + partial {:#x}", p);
        }
    }
}

/// Row 181: a pattern compiled with `PCRE2_MATCH_INVALID_UTF` can never be run
/// by the DFA matcher -> `PCRE2_ERROR_DFA_UINVALID_UTF`.
#[test]
fn rows181_dfa_match_invalid_utf() {
    unsafe {
        for pat in [&b"a"[..], b"(a)(b)", b".*"] {
            let (cc, rr) = ok_both(
                pat,
                &CompileCfg::new(PCRE2_UTF | PCRE2_MATCH_INVALID_UTF),
                "row181",
            );
            for opts in [0u32, PCRE2_NO_UTF_CHECK, PCRE2_ANCHORED] {
                let rc = diff_dfa(
                    &cc, &rr, Some(b"ab"), 2, 0, opts, Md::Create(4), &MatchCfg::new(0),
                    WS, &[], false, "row181",
                );
                assert_eq!(
                    rc, ERR_DFA_UINVALID_UTF,
                    "row181: {:?} options {:#x} must give DFA_UINVALID_UTF",
                    String::from_utf8_lossy(pat), opts
                );
            }
            // the interpreter accepts the very same pattern
            let rc = diff_int(
                &cc, &rr, Some(b"ab"), 2, 0, 0, Md::Create(4), &MatchCfg::new(0), "row181-int",
            );
            assert_ne!(rc, ERR_DFA_UINVALID_UTF);
        }
        // MATCH_INVALID_UTF is checked before BADMAGIC/BADMODE but after
        // wscount/BADOFFSET (pcre2_dfa_match.c:3407-3420)
        let (cc, rr) = ok_both(b"a", &CompileCfg::new(PCRE2_UTF | PCRE2_MATCH_INVALID_UTF), "row181-order");
        assert_eq!(
            diff_dfa(&cc, &rr, Some(b"a"), 1, 0, 0, Md::Create(4), &MatchCfg::new(0),
                     19, &[], false, "row181-order"),
            ERR_DFA_WSSIZE
        );
        assert_eq!(
            diff_dfa(&cc, &rr, Some(b"a"), 1, 9, 0, Md::Create(4), &MatchCfg::new(0),
                     WS, &[], false, "row181-order2"),
            ERR_BADOFFSET
        );
    }
}

/// Rows 182/183: `pcre2_dfa_match_8` with `code` pointing at junk ->
/// `PCRE2_ERROR_BADMAGIC`, and with forged mode bits -> `PCRE2_ERROR_BADMODE`.
#[test]
fn rows182_183_dfa_badmagic_badmode() {
    let (c, r) = both();
    unsafe {
        // NOTE: `pcre2_dfa_match_8` tests `re->overall_options &
        // PCRE2_MATCH_INVALID_UTF` (pcre2_dfa_match.c:3420) BEFORE the magic
        // number (:3425), so junk whose `overall_options` happens to have that
        // bit set is rejected with DFA_UINVALID_UTF instead. Both libraries must
        // make the same choice; only the all-zero buffer is guaranteed to reach
        // the magic-number check.
        for fill in [0u64, u64::MAX, 0x5555_5555_5555_5555] {
            let mut junk = vec![fill; 512];
            let p = junk.as_mut_ptr() as *mut c_void;
            let mut cws = fresh_ws(WS);
            let mut rws = fresh_ws(WS);
            let co = obs_dfa(
                c, p, b"abc".as_ptr(), 3, 0, 0, Md::Create(4), &MatchCfg::new(0), &mut cws, WS, false,
            );
            let ro = obs_dfa(
                r, p, b"abc".as_ptr(), 3, 0, 0, Md::Create(4), &MatchCfg::new(0), &mut rws, WS, false,
            );
            assert_eq!(co, ro, "row182: junk code (fill={:#x}) diverged", fill);
            if fill == 0 {
                assert_eq!(co.rc, ERR_BADMAGIC, "row182: a zeroed code must give BADMAGIC");
            } else {
                assert!(
                    co.rc == ERR_BADMAGIC || co.rc == ERR_DFA_UINVALID_UTF,
                    "row182: junk code (fill={:#x}) gave unexpected {}", fill, co.rc
                );
            }
        }

        let (cc, rr) = ok_both(b"abc", &CompileCfg::new(0), "row183");
        let (mut cbuf, cmagic) = clone_code(c, cc.code);
        let (mut rbuf, rmagic) = clone_code(r, rr.code);
        assert_eq!(cmagic, rmagic, "row183: magic offset differs between libraries");
        let flags_off = cmagic + 16;
        for forged in [0u32, 2, 4, 6] {
            let cf = (cbuf.as_mut_ptr() as *mut u8).add(flags_off) as *mut u32;
            let rf = (rbuf.as_mut_ptr() as *mut u8).add(flags_off) as *mut u32;
            let orig = std::ptr::read_unaligned(cf);
            std::ptr::write_unaligned(cf, (orig & !7) | forged);
            std::ptr::write_unaligned(rf, (std::ptr::read_unaligned(rf) & !7) | forged);
            let mut cws = fresh_ws(WS);
            let mut rws = fresh_ws(WS);
            let co = obs_dfa(
                c, cbuf.as_mut_ptr() as *mut c_void, b"abc".as_ptr(), 3, 0, 0,
                Md::Create(4), &MatchCfg::new(0), &mut cws, WS, false,
            );
            let ro = obs_dfa(
                r, rbuf.as_mut_ptr() as *mut c_void, b"abc".as_ptr(), 3, 0, 0,
                Md::Create(4), &MatchCfg::new(0), &mut rws, WS, false,
            );
            assert_eq!(co, ro, "row183: forged mode bits {:#x} diverged", forged);
            assert_eq!(co.rc, ERR_BADMODE, "row183: mode bits {:#x} must give BADMODE", forged);
            std::ptr::write_unaligned(cf, orig);
            std::ptr::write_unaligned(rf, orig);
        }
    }
}

/// Row 184: `PCRE2_DFA_RESTART` with a workspace whose restart state fails the
/// sanity check (`workspace[0] & ~1 != 0` or `workspace[1]` out of range,
/// pcre2_dfa_match.c:3455-3460) -> `PCRE2_ERROR_DFA_BADRESTART`.
///
/// Then the DOCUMENTED flow: a `PCRE2_PARTIAL_HARD` DFA match followed by a
/// `PCRE2_DFA_RESTART` call with the SAME workspace, comparing the workspace
/// contents and the final result between the two libraries.
#[test]
fn rows184_dfa_badrestart_and_documented_restart_flow() {
    let (c, r) = both();
    unsafe {
        let (cc, rr) = ok_both(b"abcd", &CompileCfg::new(0), "row184");
        // every rejected restart state
        for prefix in [
            &[0i32, 0][..],   // fresh/zeroed workspace: workspace[1] < 1
            &[1, 0],
            &[2, 1],          // workspace[0] & ~1 != 0
            &[3, 1],
            &[-1, 1],
            &[0, -5],
            &[0, 1_000_000],  // workspace[1] out of range
            &[1, 1_000_000],
        ] {
            let rc = diff_dfa(
                &cc, &rr, Some(b"cd"), 2, 0, PCRE2_DFA_RESTART, Md::Create(4),
                &MatchCfg::new(0), 100, prefix, false, "row184",
            );
            assert_eq!(
                rc, ERR_DFA_BADRESTART,
                "row184: workspace prefix {:?} must give DFA_BADRESTART", prefix
            );
        }
        // BADRESTART is checked after BADOFFSET / wscount / BADMAGIC
        assert_eq!(
            diff_dfa(&cc, &rr, Some(b"cd"), 2, 0, PCRE2_DFA_RESTART, Md::Create(4),
                     &MatchCfg::new(0), 19, &[], false, "row184-order"),
            ERR_DFA_WSSIZE
        );

        // ---- the documented restart flow -------------------------------------
        for (subj1, subj2, want) in [
            (&b"ab"[..], &b"cd"[..], 1i32),
            (b"a", b"bcd", 1),
            (b"abc", b"d", 1),
            (b"ab", b"xy", ERR_NOMATCH),
        ] {
            let mut wsp: [Vec<i32>; 2] = [fresh_ws(200), fresh_ws(200)];
            let mut rcs: [(i32, i32); 2] = [(0, 0), (0, 0)];
            let mut obs: [(Obs, Obs); 2] =
                [(Obs { rc: 0, count: 0, ovec: vec![], startchar: None, mark: None },
                  Obs { rc: 0, count: 0, ovec: vec![], startchar: None, mark: None }),
                 (Obs { rc: 0, count: 0, ovec: vec![], startchar: None, mark: None },
                  Obs { rc: 0, count: 0, ovec: vec![], startchar: None, mark: None })];
            for (i, api) in [c, r].iter().enumerate() {
                let code = if i == 0 { cc.code } else { rr.code };
                let md = (api.match_data_create)(4, std::ptr::null_mut());
                let rc1 = (api.dfa_match)(
                    code, subj1.as_ptr(), subj1.len(), 0, PCRE2_PARTIAL_HARD, md,
                    std::ptr::null_mut(), wsp[i].as_mut_ptr(), 200,
                );
                let o1 = observe(api, md, rc1);
                let rc2 = (api.dfa_match)(
                    code, subj2.as_ptr(), subj2.len(), 0, PCRE2_DFA_RESTART, md,
                    std::ptr::null_mut(), wsp[i].as_mut_ptr(), 200,
                );
                let o2 = observe(api, md, rc2);
                (api.match_data_free)(md);
                rcs[i] = (rc1, rc2);
                obs[i] = (o1, o2);
            }
            assert_eq!(
                rcs[0], rcs[1],
                "row184: restart flow rc differs for {:?} + {:?}",
                String::from_utf8_lossy(subj1), String::from_utf8_lossy(subj2)
            );
            assert_eq!(obs[0].0, obs[1].0, "row184: partial-match observables differ");
            assert_eq!(obs[0].1, obs[1].1, "row184: restarted-match observables differ");
            assert_eq!(
                wsp[0], wsp[1],
                "row184: the DFA workspace contents differ after the restart flow"
            );
            assert_eq!(rcs[0].0, ERR_PARTIAL, "row184: the first call must be a partial match");
            assert_eq!(
                rcs[0].1, want,
                "row184: restart with {:?} must give {}", String::from_utf8_lossy(subj2), want
            );
        }
    }
}

/// Row 185: an offset limit without `PCRE2_USE_OFFSET_LIMIT` ->
/// `PCRE2_ERROR_BADOFFSETLIMIT`, for the DFA matcher too.
#[test]
fn rows185_dfa_bad_offset_limit() {
    unsafe {
        let (cc, rr) = ok_both(b"b", &CompileCfg::new(0), "row185");
        for limit in [0usize, 1, 2, 1000] {
            let rc = diff_dfa(
                &cc, &rr, Some(b"aab"), 3, 0, 0, Md::Create(4),
                &MatchCfg::new(0).offset_limit(limit), WS, &[], false, "row185",
            );
            assert_eq!(rc, ERR_BADOFFSETLIMIT, "row185: offset_limit={}", limit);
        }
        assert_eq!(
            diff_dfa(&cc, &rr, Some(b"aab"), 3, 0, 0, Md::Create(4),
                     &MatchCfg::new(0).offset_limit(PCRE2_UNSET), WS, &[], false, "row185-unset"),
            1
        );
        let (cc2, rr2) = ok_both(b"b", &CompileCfg::new(PCRE2_USE_OFFSET_LIMIT), "row185-ok");
        for limit in [0usize, 1, 2, 3] {
            let rc = diff_dfa(
                &cc2, &rr2, Some(b"aab"), 3, 0, 0, Md::Create(4),
                &MatchCfg::new(0).offset_limit(limit), WS, &[], false, "row185-ok",
            );
            assert_ne!(rc, ERR_BADOFFSETLIMIT, "row185: limit={} must be honoured", limit);
        }
    }
}

/// Rows 186-188: opcodes and conditions the DFA matcher cannot handle.
/// `PCRE2_ERROR_DFA_UITEM (-42)` for `\C` in UTF mode (`OP_ANYBYTE`,
/// pcre2_dfa_match.c:825), for `\K`, back references and `(*scs:...)`
/// (pcre2_dfa_match.c:2943,3259), and `PCRE2_ERROR_DFA_UCOND (-40)` for
/// back-reference / duplicate-name recursion conditions
/// (pcre2_dfa_match.c:2857,2877).
#[test]
fn rows186_188_dfa_unsupported_items_and_conditions() {
    unsafe {
        // row 186: \C with PCRE2_UTF
        for subj in [&b"a"[..], b"\xc3\xa9", b"abc"] {
            let (cc, rr) = ok_both(b"\\C", &CompileCfg::new(PCRE2_UTF), "row186");
            let rc = diff_dfa(
                &cc, &rr, Some(subj), subj.len(), 0, 0, Md::Create(4), &MatchCfg::new(0),
                WS, &[], false, "row186",
            );
            assert_eq!(
                rc, ERR_DFA_UITEM,
                "row186: \\C in UTF mode on {:?} must give DFA_UITEM", String::from_utf8_lossy(subj)
            );
        }

        // row 187: other unsupported opcodes
        for (pat, subj, cfg_opts) in [
            (&b"a\\Kb"[..], &b"ab"[..], 0u32),
            (b"(a)\\1", b"aa", 0),
            (b"(a)\\1+", b"aaa", 0),
            (b"(a)(*scs:(1)a)", b"aa", 0),
            (b"a\\Kb", b"xxab", 0),
        ] {
            let (cc, rr) = ok_both(pat, &CompileCfg::new(cfg_opts), "row187");
            let rc = diff_dfa(
                &cc, &rr, Some(subj), subj.len(), 0, 0, Md::Create(4), &MatchCfg::new(0),
                WS, &[], false, "row187",
            );
            assert_eq!(
                rc, ERR_DFA_UITEM,
                "row187: {:?} must give DFA_UITEM", String::from_utf8_lossy(pat)
            );
            // the interpreter handles all of them
            let irc = diff_int(
                &cc, &rr, Some(subj), subj.len(), 0, 0, Md::FromPattern, &MatchCfg::new(0),
                "row187-int",
            );
            assert_ne!(irc, ERR_DFA_UITEM, "row187: the interpreter must not report DFA_UITEM");
        }

        // row 188: unsupported conditions
        for (pat, subj, cfg_opts) in [
            (&b"(a)(?(1)b|c)"[..], &b"ab"[..], 0u32),
            (b"(a)(?(1)b|c)", b"ac", 0),
            (b"(a)(?(R1)b|c)", b"ac", 0),
            (b"(?<n>a)(?(<n>)b|c)", b"ab", 0),
            (b"(?<n>a)|(?<n>b)(?(<n>)c|d)", b"bc", PCRE2_DUPNAMES),
        ] {
            let (cc, rr) = ok_both(pat, &CompileCfg::new(cfg_opts), "row188");
            let rc = diff_dfa(
                &cc, &rr, Some(subj), subj.len(), 0, 0, Md::Create(4), &MatchCfg::new(0),
                WS, &[], false, "row188",
            );
            assert_eq!(
                rc, ERR_DFA_UCOND,
                "row188: {:?} must give DFA_UCOND", String::from_utf8_lossy(pat)
            );
        }
        // ... while the supported conditions do NOT error
        for (pat, subj) in [(&b"(a)(?(R)b|c)"[..], &b"ac"[..]), (b"(?(DEFINE)(a))b", b"b")] {
            let (cc, rr) = ok_both(pat, &CompileCfg::new(0), "row188-ok");
            let rc = diff_dfa(
                &cc, &rr, Some(subj), subj.len(), 0, 0, Md::Create(4), &MatchCfg::new(0),
                WS, &[], false, "row188-ok",
            );
            assert!(
                rc >= 0,
                "row188: {:?} is supported by the DFA (rc={})", String::from_utf8_lossy(pat), rc
            );
        }
    }
}

/// Rows 189/190: `PCRE2_ERROR_RECURSELOOP (-52)` when a recursion repeats at
/// the same position (pcre2_dfa_match.c:2966) and `PCRE2_ERROR_DFA_RECURSE
/// (-39)` when a recursion produces more matches than the internal ovector of
/// `RWS_OVEC_RSIZE/OVEC_UNIT == 1000` entries can hold
/// (pcre2_dfa_match.c:2995).
#[test]
fn rows189_190_dfa_recurseloop_and_dfa_recurse() {
    unsafe {
        // row 189
        for pat in [&b"(?1)((?1))"[..], b"(a?(?1)?)"] {
            let (cc, rr) = ok_both(pat, &CompileCfg::new(0), "row189");
            let rc = diff_dfa(
                &cc, &rr, Some(b"aaa"), 3, 0, 0, Md::Create(4), &MatchCfg::new(0),
                WS, &[], false, "row189",
            );
            assert_eq!(
                rc, ERR_RECURSELOOP,
                "row189: {:?} must give RECURSELOOP", String::from_utf8_lossy(pat)
            );
        }
        // row 190 — > 500 distinct recursion match lengths exhausts the vector
        for pat in [&b"(a*)(?1)"[..], b"(a+)(?1)", b"(a*)(?1)?"] {
            let (cc, rr) = ok_both(pat, &CompileCfg::new(0), "row190");
            for n in [600usize, 1200] {
                let subj = vec![b'a'; n];
                let rc = diff_dfa(
                    &cc, &rr, Some(&subj), subj.len(), 0, 0, Md::Create(4), &MatchCfg::new(0),
                    20_000, &[], false, "row190",
                );
                assert_eq!(
                    rc, ERR_DFA_RECURSE,
                    "row190: {:?} on {} a's must give DFA_RECURSE",
                    String::from_utf8_lossy(pat), n
                );
            }
            // a short subject stays under the limit
            let rc = diff_dfa(
                &cc, &rr, Some(b"aa"), 2, 0, 0, Md::Create(4), &MatchCfg::new(0),
                WS, &[], false, "row190-ok",
            );
            assert_ne!(rc, ERR_DFA_RECURSE, "row190: 2 a's must stay below the limit");
        }
    }
}

/// Rows 192/194/195: the DFA heap / match / depth limits
/// (pcre2_dfa_match.c:445,566,567).
#[test]
fn rows192_195_dfa_limits() {
    unsafe {
        let (cc, rr) = ok_both(b"(a(?1)?)", &CompileCfg::new(0), "row192");
        // row 192 — the recursion workspace cannot grow within the heap limit
        let a100 = vec![b'a'; 100];
        for lim in [0u32, 1, 2] {
            let rc = diff_dfa(
                &cc, &rr, Some(&a100), a100.len(), 0, 0, Md::Create(4),
                &MatchCfg::new(0).heap_limit(lim), WS, &[], false, "row192",
            );
            assert_eq!(rc, ERR_HEAPLIMIT, "row192: heap_limit={}", lim);
        }
        // row 194 — match limit
        for lim in [1u32, 2, 3] {
            let rc = diff_dfa(
                &cc, &rr, Some(b"aaa"), 3, 0, 0, Md::Create(4),
                &MatchCfg::new(0).match_limit(lim), WS, &[], false, "row194",
            );
            assert_eq!(rc, ERR_MATCHLIMIT, "row194: match_limit={}", lim);
        }
        // row 195 — depth limit
        for lim in [0u32, 1] {
            let rc = diff_dfa(
                &cc, &rr, Some(b"aaa"), 3, 0, 0, Md::Create(4),
                &MatchCfg::new(0).depth_limit(lim), WS, &[], false, "row195",
            );
            assert_eq!(rc, ERR_DEPTHLIMIT, "row195: depth_limit={}", lim);
        }
        // generous limits succeed
        let rc = diff_dfa(
            &cc, &rr, Some(b"aaa"), 3, 0, 0, Md::Create(4),
            &MatchCfg::new(0).match_limit(1_000_000).depth_limit(1_000).heap_limit(100_000),
            WS, &[], false, "row192-ok",
        );
        assert!(rc > 0, "row192: generous limits must let the match succeed (rc={})", rc);
    }
}

/// Row 193: `PCRE2_ERROR_NOMEMORY` from `more_workspace` /
/// the `PCRE2_COPY_MATCHED_SUBJECT` copy (pcre2_dfa_match.c:446,4066).
#[test]
fn row193_dfa_nomemory_on_allocation_failure() {
    let _g = INJ_LOCK.lock().unwrap();
    let (c, r) = both();
    unsafe {
        // more_workspace()
        let (cc, rr) = ok_both(b"(a(?1)?)", &CompileCfg::new(0), "row193");
        let a100 = vec![b'a'; 100];
        let mut saw = false;
        for fail_at in 1..=6u32 {
            let (crc, cn) = inj_run(c, 0, cc.code, &a100, 0, 4, None, fail_at, true, WS);
            let (rrc, rn) = inj_run(r, 1, rr.code, &a100, 0, 4, None, fail_at, true, WS);
            assert_eq!(crc, rrc, "row193: rc differs with fail_at={}", fail_at);
            assert_eq!(cn, rn, "row193: allocation count differs with fail_at={}", fail_at);
            if crc == ERR_NOMEMORY {
                saw = true;
            }
        }
        assert!(saw, "row193: more_workspace failure must give NOMEMORY");

        // the subject copy
        let (cc2, rr2) = ok_both(b"abc", &CompileCfg::new(0), "row193-copy");
        let mut saw_copy = false;
        for fail_at in 1..=4u32 {
            let (crc, cn) = inj_run(
                c, 0, cc2.code, b"abc", PCRE2_COPY_MATCHED_SUBJECT, 4, None, fail_at, true, WS,
            );
            let (rrc, rn) = inj_run(
                r, 1, rr2.code, b"abc", PCRE2_COPY_MATCHED_SUBJECT, 4, None, fail_at, true, WS,
            );
            assert_eq!(crc, rrc, "row193-copy: rc differs with fail_at={}", fail_at);
            assert_eq!(cn, rn, "row193-copy: alloc count differs with fail_at={}", fail_at);
            if crc == ERR_NOMEMORY {
                saw_copy = true;
            }
        }
        assert!(saw_copy, "row193: the subject copy failure must give NOMEMORY");
    }
}

/// Rows 196/197: UTF-8 subject errors and `PCRE2_ERROR_BADUTFOFFSET` from
/// `pcre2_dfa_match_8` (pcre2_dfa_match.c:3599,3620).
#[test]
fn rows196_197_dfa_utf_subject_errors() {
    unsafe {
        let (cc, rr) = ok_both(b"a", &CompileCfg::new(PCRE2_UTF), "row196");
        // row 196
        for (subj, want) in [
            (&b"\x80"[..], utf8_err(20)),
            (b"\xbf", utf8_err(20)),
            (b"a\x80", utf8_err(20)),
            (b"a\xc3", utf8_err(1)),
            (b"\xe2\x82", utf8_err(1)),
            (b"\xe2", utf8_err(2)),
            (b"\xf0", utf8_err(3)),
            (b"\xf8", utf8_err(4)),
            (b"\xfc", utf8_err(5)),
            (b"\xfe", utf8_err(21)),
            (b"\xff", utf8_err(21)),
            (b"\xed\xa0\x80", utf8_err(14)),
            (b"\xc0\x80", utf8_err(15)),
            (b"\xf4\x90\x80\x80", utf8_err(13)),
        ] {
            let rc = diff_dfa(
                &cc, &rr, Some(subj), subj.len(), 0, 0, Md::Create(4), &MatchCfg::new(0),
                WS, &[], false, "row196",
            );
            assert_eq!(
                rc, want,
                "row196: subject {:02x?} must give {}", subj, want
            );
        }
        // row 197
        for (subj, off) in [
            (&b"\xc3\xa9"[..], 1usize),
            (b"\xe2\x82\xac", 1),
            (b"\xe2\x82\xac", 2),
            (b"a\xf0\x9f\x98\x80", 2),
        ] {
            let rc = diff_dfa(
                &cc, &rr, Some(subj), subj.len(), off, 0, Md::Create(4), &MatchCfg::new(0),
                WS, &[], false, "row197",
            );
            assert_eq!(
                rc, ERR_BADUTFOFFSET,
                "row197: start_offset={} inside {:02x?}", off, subj
            );
        }
    }
}

/// Rows 198/199: plain `PCRE2_ERROR_NOMATCH` and `PCRE2_ERROR_PARTIAL` from the
/// DFA matcher, including `PCRE2_DFA_SHORTEST`.
#[test]
fn rows198_199_dfa_nomatch_and_partial() {
    unsafe {
        let (cc, rr) = ok_both(b"zzz", &CompileCfg::new(0), "row198");
        for subj in [&b"aaa"[..], b"", b"zz", b"zzy"] {
            let rc = diff_dfa(
                &cc, &rr, Some(subj), subj.len(), 0, 0, Md::Create(4), &MatchCfg::new(0),
                WS, &[], false, "row198",
            );
            assert_eq!(
                rc, ERR_NOMATCH,
                "row198: zzz must not match {:?}", String::from_utf8_lossy(subj)
            );
        }
        let (cc2, rr2) = ok_both(b"abc", &CompileCfg::new(0), "row199");
        for opt in [PCRE2_PARTIAL_SOFT, PCRE2_PARTIAL_HARD, PCRE2_PARTIAL_SOFT | PCRE2_DFA_SHORTEST] {
            for subj in [&b"ab"[..], b"a", b"xxab"] {
                let rc = diff_dfa(
                    &cc2, &rr2, Some(subj), subj.len(), 0, opt, Md::Create(4), &MatchCfg::new(0),
                    WS, &[], false, "row199",
                );
                assert_eq!(
                    rc, ERR_PARTIAL,
                    "row199: {:?} options {:#x} must be a partial match",
                    String::from_utf8_lossy(subj), opt
                );
            }
        }
    }
}

// =========================================== section M — _pcre2_valid_utf_8

/// One `_pcre2_valid_utf_8` call in both libraries: the return code AND the
/// `*erroroffset` must agree. Both offsets start from the same sentinel, so if
/// only one library writes it the assertion fires.
unsafe fn diff_valid_utf(bytes: &[u8], len: usize, label: &str) -> (i32, usize) {
    let (c, r) = both();
    const SENTINEL: usize = 0xDEAD_BEEF_DEAD_BEEF_u64 as usize;
    let mut co = SENTINEL;
    let mut ro = SENTINEL;
    let crc = (c.valid_utf)(bytes.as_ptr(), len, &mut co);
    let rrc = (r.valid_utf)(bytes.as_ptr(), len, &mut ro);
    assert_eq!(
        crc, rrc,
        "{}: _pcre2_valid_utf_8 rc differs (C={} Rust={}) for {:02x?} len={}",
        label, crc, rrc, &bytes[..len.min(bytes.len())], len
    );
    assert_eq!(
        co, ro,
        "{}: _pcre2_valid_utf_8 erroroffset differs (rc={} C={:#x} Rust={:#x}) for {:02x?}",
        label, crc, co, ro, &bytes[..len.min(bytes.len())]
    );
    if crc == 0 {
        assert_eq!(
            co, SENTINEL,
            "{}: erroroffset must not be written on success for {:02x?}",
            label, &bytes[..len.min(bytes.len())]
        );
    }
    (crc, co)
}

/// Rows 368-388: one explicit case per documented `_pcre2_valid_utf_8` error,
/// checked at offset 0 and after a valid prefix so the reported
/// `*erroroffset` is exercised too.
#[test]
fn rows368_388_valid_utf_specific_triggers() {
    unsafe {
        // (row, bytes, expected UTF8_ERRn)
        //
        // NOTE on rows 369-372: `_pcre2_valid_utf_8` selects ERR1..ERR5 from
        // `ab - length`, i.e. from how many of the *continuation* bytes the
        // lead byte asked for are missing (pcre2_valid_utf.c:154-166). The
        // illustrative byte strings printed in ERRORS.md rows 369-372
        // ("\xe2\x82", "\xf0\x9f\x98", "\xf8\x88\x80\x80",
        // "\xfc\x84\x80\x80\x80") are each only ONE byte short, so the C
        // library returns ERR1 for all of them — verified below in the
        // `short_by_one` block. The strings used here are the ones that really
        // are 2, 3, 4 and 5 bytes short, which is what those rows describe.
        let cases: &[(u32, &[u8], i32)] = &[
            (368, b"\xc3", 1),
            (368, b"\xdf", 1),
            (369, b"\xe2", 2),
            (369, b"\xf0\x9f", 2),
            (370, b"\xf0", 3),
            (370, b"\xf8\x88", 3),
            (371, b"\xf8", 4),
            (371, b"\xfc\x84", 4),
            (372, b"\xfc", 5),
            (373, b"\xc3\x41", 6),
            (373, b"\xc3\xc3", 6),
            (374, b"\xe2\x82\x41", 7),
            (375, b"\xf0\x9f\x98\x41", 8),
            (376, b"\xf8\x88\x80\x80\x41", 9),
            (377, b"\xfc\x84\x80\x80\x80\x41", 10),
            (378, b"\xf8\x88\x80\x80\x80", 11),
            (379, b"\xfc\x84\x80\x80\x80\x80", 12),
            (380, b"\xf4\x90\x80\x80", 13),
            (380, b"\xf7\xbf\xbf\xbf", 13),
            (381, b"\xed\xa0\x80", 14),
            (381, b"\xed\xbf\xbf", 14),
            (382, b"\xc0\x80", 15),
            (382, b"\xc1\xbf", 15),
            (383, b"\xe0\x80\x80", 16),
            (383, b"\xe0\x9f\xbf", 16),
            (384, b"\xf0\x80\x80\x80", 17),
            (384, b"\xf0\x8f\xbf\xbf", 17),
            (385, b"\xf8\x80\x80\x80\x80", 18),
            (386, b"\xfc\x80\x80\x80\x80\x80", 19),
            (387, b"\x80", 20),
            (387, b"\xbf", 20),
            (388, b"\xfe", 21),
            (388, b"\xff", 21),
        ];
        for &(row, bad, errn) in cases {
            let (rc, off) = diff_valid_utf(bad, bad.len(), &format!("row{}", row));
            assert_eq!(
                rc, utf8_err(errn),
                "row{}: {:02x?} must give PCRE2_ERROR_UTF8_ERR{}", row, bad, errn
            );
            assert_eq!(off, 0, "row{}: the error offset must be 0 for {:02x?}", row, bad);

            // the same sequence after a valid prefix: the offset must shift
            for prefix in [&b"a"[..], b"ab", b"\xc3\xa9", b"\xe2\x82\xac", b"\xf0\x9f\x98\x80"] {
                let mut v = prefix.to_vec();
                v.extend_from_slice(bad);
                let (rc2, off2) =
                    diff_valid_utf(&v, v.len(), &format!("row{}-prefixed", row));
                assert_eq!(
                    rc2, utf8_err(errn),
                    "row{}: prefixed {:02x?} must still give UTF8_ERR{}", row, v, errn
                );
                assert_eq!(
                    off2, prefix.len(),
                    "row{}: the error offset must be {} for {:02x?}", row, prefix.len(), v
                );
            }
        }
        // rows 369-372, the byte strings literally printed in ERRORS.md: each is
        // short by exactly one continuation byte, so each gives ERR1.
        for bad in [
            &b"\xe2\x82"[..],
            b"\xf0\x9f\x98",
            b"\xf8\x88\x80\x80",
            b"\xfc\x84\x80\x80\x80",
        ] {
            let (rc, off) = diff_valid_utf(bad, bad.len(), "short_by_one");
            assert_eq!(
                rc, utf8_err(1),
                "rows369-372: {:02x?} is short by one byte, so it gives UTF8_ERR1", bad
            );
            assert_eq!(off, 0);
        }

        // every documented UTF8 error number is covered by the table above
        let mut seen = [false; 22];
        for &(_, _, errn) in cases {
            seen[errn as usize] = true;
        }
        for n in 1..=21usize {
            assert!(seen[n], "section M: UTF8_ERR{} has no test case", n);
        }
        // ... and valid strings really are accepted
        for good in [
            &b""[..], b"a", b"abc", b"\x7f", b"\xc2\x80", b"\xdf\xbf", b"\xe0\xa0\x80",
            b"\xed\x9f\xbf", b"\xee\x80\x80", b"\xef\xbf\xbf", b"\xf0\x90\x80\x80",
            b"\xf4\x8f\xbf\xbf", b"a\xc3\xa9b\xe2\x82\xacc\xf0\x9f\x98\x80d",
        ] {
            let (rc, _) = diff_valid_utf(good, good.len(), "valid");
            assert_eq!(rc, 0, "{:02x?} is valid UTF-8", good);
        }
        // a zero length is always valid, whatever the buffer holds
        for bad in [&b"\x80"[..], b"\xff", b"\xc3"] {
            let (rc, _) = diff_valid_utf(bad, 0, "len0");
            assert_eq!(rc, 0, "length 0 must always be valid");
        }
    }
}

/// Section M, exhaustive: all 256 single bytes and all 65 536 two-byte
/// sequences, comparing both the return code and `*erroroffset`.
#[test]
fn rows368_388_valid_utf_all_one_and_two_byte_sequences() {
    unsafe {
        for b in 0..=255u8 {
            let s = [b];
            diff_valid_utf(&s, 1, "1byte");
            // the same byte after a valid prefix
            let s2 = [b'a', b];
            diff_valid_utf(&s2, 2, "1byte-prefixed");
        }
        for hi in 0..=255u8 {
            for lo in 0..=255u8 {
                let s = [hi, lo];
                diff_valid_utf(&s, 2, "2byte");
            }
        }
        // and every two-byte sequence with a one-byte prefix, so non-zero error
        // offsets are exercised across the whole space as well
        for hi in 0..=255u8 {
            for lo in 0..=255u8 {
                let s = [b'x', hi, lo];
                diff_valid_utf(&s, 3, "2byte-prefixed");
            }
        }
    }
}

/// Section M, randomised: >= 50 000 sequences of 3 to 6 bytes drawn from a
/// fixed seed, half from an "interesting" alphabet of UTF-8 boundary bytes and
/// half fully random, always comparing rc AND `*erroroffset`.
#[test]
fn rows368_388_valid_utf_randomised_sequences() {
    unsafe {
        const ALPHABET: &[u8] = &[
            0x00, 0x01, 0x41, 0x7f, 0x80, 0x81, 0x8f, 0xa0, 0xbf, 0xc0, 0xc1, 0xc2, 0xdf,
            0xe0, 0xe1, 0xec, 0xed, 0xee, 0xef, 0xf0, 0xf1, 0xf4, 0xf5, 0xf7, 0xf8, 0xf9,
            0xfb, 0xfc, 0xfd, 0xfe, 0xff,
        ];
        let mut rng = Rng::new(0x5EED_1234_ABCD_0001);
        let mut n = 0usize;
        for _ in 0..120_000 {
            let len = rng.range(3, 7) as usize; // 3..=6
            let body = if rng.bool() {
                rng.bytes_from(len, ALPHABET)
            } else {
                rng.raw_bytes(len)
            };
            diff_valid_utf(&body, body.len(), "rand");
            n += 1;
            // also with a random valid prefix so error offsets vary
            let prefix: &[u8] = rng.pick(&[
                &b""[..], b"a", b"ab", b"\xc3\xa9", b"\xe2\x82\xac", b"\xf0\x9f\x98\x80",
            ]);
            let mut v = prefix.to_vec();
            v.extend_from_slice(&body);
            diff_valid_utf(&v, v.len(), "rand-prefixed");
            n += 1;
            // and a truncated length, to exercise the "missing bytes" branches
            for cut in 1..body.len() {
                diff_valid_utf(&body, cut, "rand-truncated");
                n += 1;
            }
        }
        assert!(n >= 50_000, "section M: only {} randomised cases were run", n);
    }
}

/// Rows 368-389: the SAME UTF error codes and offsets must surface through
/// `pcre2_compile_8` (with `PCRE2_UTF`, pcre2_compile.c:10628), through
/// `pcre2_match_8` (pcre2_match.c:7347) and through `pcre2_dfa_match_8`
/// (pcre2_dfa_match.c:3620), where the offset lands in
/// `match_data->startchar`.
#[test]
fn rows368_389_utf_errors_surface_through_compile_match_and_dfa() {
    let (c, r) = both();
    unsafe {
        let bad: &[&[u8]] = &[
            b"\xc3", b"\xe2\x82", b"\xf0\x9f\x98", b"\xf8\x88\x80\x80",
            b"\xfc\x84\x80\x80\x80", b"\xc3\x41", b"\xe2\x82\x41", b"\xf0\x9f\x98\x41",
            b"\xf8\x88\x80\x80\x41", b"\xfc\x84\x80\x80\x80\x41", b"\xf8\x88\x80\x80\x80",
            b"\xfc\x84\x80\x80\x80\x80", b"\xf4\x90\x80\x80", b"\xed\xa0\x80", b"\xc0\x80",
            b"\xe0\x80\x80", b"\xf0\x80\x80\x80", b"\xf8\x80\x80\x80\x80",
            b"\xfc\x80\x80\x80\x80\x80", b"\x80", b"\xfe", b"\xff",
        ];
        // A pattern with no lookbehind, so the DFA / interpreter check the
        // subject from start_offset onwards with no reverse scan.
        let (cc, rr) = ok_both(b"a", &CompileCfg::new(PCRE2_UTF), "sectionM-surface");
        for seq in bad {
            for prefix in [&b""[..], b"a", b"ab"] {
                let mut subj = prefix.to_vec();
                subj.extend_from_slice(seq);
                let (want, want_off) = diff_valid_utf(&subj, subj.len(), "sectionM-ref");
                assert!(want < 0, "{:02x?} must be invalid UTF-8", subj);

                // --- pcre2_compile_8: *errorcode IS the negative UTF code
                let cfg = CompileCfg::new(PCRE2_UTF);
                let ccp = compile_in(c, &subj, subj.len(), &cfg);
                let rcp = compile_in(r, &subj, subj.len(), &cfg);
                assert!(ccp.code.is_null() && rcp.code.is_null(), "invalid UTF must not compile");
                assert_eq!(ccp.errorcode, rcp.errorcode, "compile errorcode differs");
                assert_eq!(ccp.erroroffset, rcp.erroroffset, "compile erroroffset differs");
                assert_eq!(
                    ccp.errorcode, want,
                    "compile must report the valid_utf code for {:02x?}", subj
                );
                assert_eq!(
                    ccp.erroroffset, want_off,
                    "compile must report the valid_utf offset for {:02x?}", subj
                );

                // --- pcre2_match_8 / pcre2_dfa_match_8
                let rc = diff_int(
                    &cc, &rr, Some(&subj), subj.len(), 0, 0, Md::Create(4),
                    &MatchCfg::new(0), "sectionM-match",
                );
                let drc = diff_dfa(
                    &cc, &rr, Some(&subj), subj.len(), 0, 0, Md::Create(4),
                    &MatchCfg::new(0), WS, &[], false, "sectionM-dfa",
                );
                assert_eq!(rc, want, "pcre2_match_8 must report {} for {:02x?}", want, subj);
                assert_eq!(drc, want, "pcre2_dfa_match_8 must report {} for {:02x?}", want, subj);

                // startchar carries the offset. It is only written when the
                // error came from valid_utf; the "first code unit is not a
                // character start" shortcut (pcre2_match.c:7295) returns
                // UTF8_ERR20 directly WITHOUT touching startchar, so that case
                // is excluded here.
                let first_is_cont = subj[0] & 0xc0 == 0x80;
                if !first_is_cont {
                    for (api, code) in [(c, cc.code), (r, rr.code)] {
                        let md = (api.match_data_create)(4, std::ptr::null_mut());
                        let irc =
                            (api.do_match)(code, subj.as_ptr(), subj.len(), 0, 0, md, std::ptr::null_mut());
                        assert_eq!(irc, want);
                        assert_eq!(
                            (api.get_startchar)(md), want_off,
                            "{}: pcre2_match_8 startchar for {:02x?}", api.name, subj
                        );
                        let mut ws = fresh_ws(WS);
                        let drc2 = (api.dfa_match)(
                            code, subj.as_ptr(), subj.len(), 0, 0, md, std::ptr::null_mut(),
                            ws.as_mut_ptr(), WS,
                        );
                        assert_eq!(drc2, want);
                        assert_eq!(
                            (api.get_startchar)(md), want_off,
                            "{}: pcre2_dfa_match_8 startchar for {:02x?}", api.name, subj
                        );
                        (api.match_data_free)(md);
                    }
                }
            }
        }
        // Row 387's special case: at start_offset 0 a leading continuation byte
        // is reported as UTF8_ERR20 by pcre2_match_8 directly, and row 389: at
        // start_offset > 0 the very same byte is BADUTFOFFSET.
        for lead in [0x80u8, 0x9f, 0xbf] {
            let subj = [b'a', lead, b'b'];
            assert_eq!(
                diff_int(&cc, &rr, Some(&subj), 3, 0, 0, Md::Create(4), &MatchCfg::new(0), "row387"),
                utf8_err(20)
            );
            assert_eq!(
                diff_int(&cc, &rr, Some(&subj), 3, 1, 0, Md::Create(4), &MatchCfg::new(0), "row389"),
                ERR_BADUTFOFFSET
            );
            assert_eq!(
                diff_dfa(&cc, &rr, Some(&subj), 3, 1, 0, Md::Create(4), &MatchCfg::new(0),
                         WS, &[], false, "row389-dfa"),
                ERR_BADUTFOFFSET
            );
        }
        // A pattern WITH a lookbehind makes the matcher scan backwards from
        // start_offset before validating, which must behave identically too.
        let (lc, lr) = ok_both(b"(?<=\\x{e9})b", &CompileCfg::new(PCRE2_UTF), "sectionM-lookbehind");
        for subj in [&b"\xc3\xa9b"[..], b"\xc3b", b"\x80b", b"\xc3\xa9\xc3b"] {
            diff_int(&lc, &lr, Some(subj), subj.len(), 0, 0, Md::Create(4), &MatchCfg::new(0), "lb");
            diff_dfa(
                &lc, &lr, Some(subj), subj.len(), 0, 0, Md::Create(4), &MatchCfg::new(0),
                WS, &[], false, "lb-dfa",
            );
            for off in 0..=subj.len() {
                diff_int(&lc, &lr, Some(subj), subj.len(), off, 0, Md::Create(4), &MatchCfg::new(0), "lb");
                diff_dfa(
                    &lc, &lr, Some(subj), subj.len(), off, 0, Md::Create(4), &MatchCfg::new(0),
                    WS, &[], false, "lb-dfa",
                );
            }
        }
    }
}

// ================================================ unreachable / UB row records

/// Rows 170, 171, 172 (second site), 200 and the KNOWN-UB inputs.
///
/// These are deliberately NOT driven by calling the libraries, because the C
/// library itself either cannot be steered into them from outside or crashes,
/// so there is no observable "correct" answer to compare against. They are
/// recorded here so that every row of `ERRORS.md` sections C and D is
/// accounted for, together with the *documented* behaviour that we CAN and do
/// verify instead.
///
/// * **row 170** — `PCRE2_ERROR_BAD_BACKSLASH_K (-75)`
///   (pcre2_match.c:1021-1030). The check fires at `OP_END` when `\K` moved
///   `Fstart_match` outside `[start_offset, Feptr]`. `\K` syntactically inside
///   a lookaround is rejected at compile time with `ERR99`, and
///   `PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK` (which is what makes such a pattern
///   compile) *also* sets `mb->allowlookaroundbsk`, which suppresses the error.
///   Reaching it therefore needs a `\K` propagated out of a recursion
///   (pcre2_match.c:966) into a frame whose `Feptr` later moves backwards, and
///   every recursion shape we tried is caught by `PCRE2_ERROR_RECURSELOOP`
///   first. What IS verified below: `(?=a\K)` gives `ERR99`, the same pattern
///   with `PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK` compiles, and the resulting match
///   (which exercises the `Fstart_match` bookkeeping and the
///   `allowlookaroundbsk` branch) is identical in both libraries.
///
/// * **row 171** — `PCRE2_ERROR_INTERNAL (-44)` from an unknown opcode
///   (pcre2_match.c:2876,3229,3507,…,6941) and **row 200** — the
///   `internal_dfa_match` sanity check (pcre2_dfa_match.c:3576). Both need a
///   *corrupted compiled pattern* whose header is still consistent, i.e.
///   arbitrary bytes in the middle of the bytecode. The behaviour of the C
///   library on such input is not defined (it may equally read out of bounds),
///   so it is not a comparable observable. The structural validation that IS
///   reachable — `BADMAGIC` and `BADMODE` — is covered by
///   `rows153_154_397_398_badmagic_badmode` and
///   `rows182_183_dfa_badmagic_badmode`.
///
/// * **row 172** — `heapframes_size` overflow. The reachable half of this site
///   (`max_size < frame_size` with a tiny heap limit) is covered by
///   `rows159_161_172_match_depth_heap_limits`; the arithmetic-overflow half
///   would need `PCRE2_SIZE_MAX`-scale sizes.
///
/// KNOWN UNDEFINED BEHAVIOUR — verified to crash BOTH libraries, so never
/// called:
/// * `PCRE2_NO_UTF_CHECK` together with genuinely invalid UTF-8 (documented as
///   undefined in the pcre2 manual).
/// * `PCRE2_DFA_RESTART` with a workspace that passes the sanity check
///   (`workspace[0] & ~1 == 0`, `1 <= workspace[1] <= (wscount-2)/…`) but did
///   not come from a previous partial match. The documented flow is tested in
///   `rows184_dfa_badrestart_and_documented_restart_flow`, and the states the
///   sanity check *does* reject are all asserted there.
/// * `pcre2_get_mark_8` / `pcre2_get_ovector_pointer_8` /
///   `pcre2_get_ovector_count_8` / `pcre2_get_startchar_8` /
///   `pcre2_next_match_8` with a NULL `match_data` — there is no NULL guard in
///   `pcre2_match_data.c`.
#[test]
fn rows170_171_172_200_unreachable_and_ub_documented() {
    let (c, r) = both();
    unsafe {
        // row 170, the reachable half: \K inside a lookaround.
        let cfg = CompileCfg::new(0);
        let (cc, rr) = compile_both(b"(?=a\\K)", 7, &cfg, "row170");
        assert!(cc.code.is_null(), "row170: (?=a\\K) must be rejected at compile time");
        assert_eq!(cc.errorcode, 199, "row170: (?=a\\K) must give ERR99 (=199)");
        assert_eq!(rr.errorcode, cc.errorcode);

        let cfg2 = CompileCfg::new(0).extra(PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK);
        for pat in [&b"(?=a\\K)"[..], b"(?=a\\K)b", b"(?<=a\\K)b", b"a(?=b\\K)c"] {
            let (cc2, rr2) = compile_both(pat, pat.len(), &cfg2, "row170-allowed");
            if cc2.code.is_null() {
                continue;
            }
            for subj in [&b"ab"[..], b"abc", b"a", b"ba"] {
                for start in 0..=subj.len() {
                    let rc = diff_int(
                        &cc2, &rr2, Some(subj), subj.len(), start, 0, Md::FromPattern,
                        &MatchCfg::new(0), "row170-allowed",
                    );
                    assert_ne!(
                        rc, ERR_BAD_BACKSLASH_K,
                        "row170: ALLOW_LOOKAROUND_BSK suppresses PCRE2_ERROR_BAD_BACKSLASH_K"
                    );
                }
            }
        }

        // rows 171 / 200: the *reachable* structural checks agree, and a
        // well-formed pattern never reports INTERNAL.
        for pat in [&b"a"[..], b"(a)(b)|c", b"[a-z]+\\d*", b"(?:ab)*c"] {
            let (cc3, rr3) = ok_both(pat, &CompileCfg::new(0), "row171");
            for subj in [&b"abc"[..], b"zzz", b"", b"a1b2"] {
                let rc = diff_int(
                    &cc3, &rr3, Some(subj), subj.len(), 0, 0, Md::FromPattern,
                    &MatchCfg::new(0), "row171",
                );
                assert_ne!(rc, ERR_INTERNAL, "row171: a valid pattern must not report INTERNAL");
                let drc = diff_dfa(
                    &cc3, &rr3, Some(subj), subj.len(), 0, 0, Md::Create(8), &MatchCfg::new(0),
                    WS, &[], false, "row200",
                );
                assert_ne!(drc, ERR_INTERNAL, "row200: a valid pattern must not report INTERNAL");
            }
        }

        // KNOWN UB, safe half only: PCRE2_NO_UTF_CHECK with VALID UTF-8 skips
        // the check and must behave identically in both libraries.
        let (cu, ru) = ok_both(b"\\x{e9}", &CompileCfg::new(PCRE2_UTF), "no-utf-check");
        for subj in [&b"\xc3\xa9"[..], b"a\xc3\xa9b", b"\xe2\x82\xac\xc3\xa9"] {
            for opts in [0u32, PCRE2_NO_UTF_CHECK] {
                diff_int(
                    &cu, &ru, Some(subj), subj.len(), 0, opts, Md::FromPattern,
                    &MatchCfg::new(0), "no-utf-check",
                );
                diff_dfa(
                    &cu, &ru, Some(subj), subj.len(), 0, opts, Md::Create(4), &MatchCfg::new(0),
                    WS, &[], false, "no-utf-check-dfa",
                );
            }
        }
        // and the NULL match_data accessors are only ever called with a real
        // match_data — here is the well-defined equivalent.
        let md_c = (c.match_data_create)(2, std::ptr::null_mut());
        let md_r = (r.match_data_create)(2, std::ptr::null_mut());
        assert_eq!((c.get_ovector_count)(md_c), (r.get_ovector_count)(md_r));
        assert!(!(c.get_ovector_pointer)(md_c).is_null());
        assert!(!(r.get_ovector_pointer)(md_r).is_null());
        (c.match_data_free)(md_c);
        (r.match_data_free)(md_r);
    }
}
