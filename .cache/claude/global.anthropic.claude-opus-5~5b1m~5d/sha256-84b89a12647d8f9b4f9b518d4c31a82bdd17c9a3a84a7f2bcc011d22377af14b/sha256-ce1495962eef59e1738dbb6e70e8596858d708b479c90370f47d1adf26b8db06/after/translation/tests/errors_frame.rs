//! Differential error-path tests for the **compression** side of `lz4frame.c`.
//!
//! Covers rows **1..23** of `translation/ERRORS.md` (the `LZ4F_*` compression
//! rejection paths). Rows 24..55 (the decompression side) live in a separate
//! test file and are deliberately NOT duplicated here.
//!
//! In addition this file covers the "generic FFI boundary" surface of the
//! lz4frame compression API: NULL pointers into every entry point, zero and
//! oversized lengths, out-of-range enum values (C enums accept any `int`, so
//! these are real inputs), non-zero `reserved` fields, unvalidated `version`
//! values and the full `LZ4F_isError` / `LZ4F_getErrorCode` /
//! `LZ4F_getErrorName` boundary.
//!
//! Rules obeyed here:
//!   * every call goes through a `.so` export via `libloading` — no Rust
//!     function is ever called directly;
//!   * an opaque context (`LZ4F_cctx`, `LZ4F_dctx`, `LZ4F_CDict`) is always
//!     created and destroyed by the *same* library, never crossed over;
//!   * the exact error sentinel is asserted, never merely "both failed".
//!
//! The rows that `ERRORS.md` describes as allocation failures (4, 5, 6, 10,
//! 11, 21) are forced for real by driving
//! `LZ4F_createCompressionContext_advanced` /
//! `LZ4F_createDecompressionContext_advanced` / `LZ4F_createCDict_advanced`
//! with a caller-supplied `LZ4F_CustomMem` whose alloc/calloc shims fail
//! (unconditionally, or on the Nth call). Rows 8 and 23 go through the
//! *non*-advanced constructors, which hard-code `LZ4F_defaultCMem`, so they are
//! not reachable without interposing libc's `malloc`/`calloc` for the whole
//! process; see the comments on `err_8_...` / `err_23_...`.
#![allow(unused_imports, non_snake_case, non_camel_case_types, unused_assignments)]

mod common;
use common::*;
use std::ffi::CStr;
use std::os::raw::{c_char, c_int, c_uint, c_void};
use std::ptr;

// ---------------------------------------------------------------------------
// libc (for the allocator shims)
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn calloc(n: usize, size: usize) -> *mut c_void;
    fn free(p: *mut c_void);
}

// ---------------------------------------------------------------------------
// LZ4F_CustomMem mirror (lz4frame.h:727-735)
// ---------------------------------------------------------------------------

type LZ4F_AllocFunction = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
type LZ4F_CallocFunction = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
type LZ4F_FreeFunction = unsafe extern "C" fn(*mut c_void, *mut c_void);

#[repr(C)]
#[derive(Copy, Clone)]
struct LZ4F_CustomMem {
    customAlloc: Option<LZ4F_AllocFunction>,
    customCalloc: Option<LZ4F_CallocFunction>,
    customFree: Option<LZ4F_FreeFunction>,
    opaqueState: *mut c_void,
}

/// Per-call allocator bookkeeping, reached through `opaqueState` so that the
/// shims stay reentrant and thread-safe (cargo runs tests in parallel).
#[repr(C)]
struct Hook {
    /// `-1` = fail every allocation; `0` = never fail; `N > 0` = fail the Nth
    /// alloc/calloc call.
    fail_at: i64,
    n_alloc: u64,
    n_calloc: u64,
    n_free: u64,
    live: i64,
}

impl Hook {
    fn new(fail_at: i64) -> Box<Hook> {
        Box::new(Hook { fail_at, n_alloc: 0, n_calloc: 0, n_free: 0, live: 0 })
    }
    fn calls(&self) -> u64 {
        self.n_alloc + self.n_calloc
    }
}

unsafe extern "C" fn hook_alloc(opaque: *mut c_void, size: usize) -> *mut c_void {
    let st = &mut *(opaque as *mut Hook);
    st.n_alloc += 1;
    if st.fail_at < 0 || (st.fail_at > 0 && st.calls() == st.fail_at as u64) {
        return ptr::null_mut();
    }
    let p = malloc(if size == 0 { 1 } else { size });
    if !p.is_null() {
        st.live += 1;
    }
    p
}

unsafe extern "C" fn hook_calloc(opaque: *mut c_void, size: usize) -> *mut c_void {
    let st = &mut *(opaque as *mut Hook);
    st.n_calloc += 1;
    if st.fail_at < 0 || (st.fail_at > 0 && st.calls() == st.fail_at as u64) {
        return ptr::null_mut();
    }
    let p = calloc(1, if size == 0 { 1 } else { size });
    if !p.is_null() {
        st.live += 1;
    }
    p
}

unsafe extern "C" fn hook_free(opaque: *mut c_void, addr: *mut c_void) {
    let st = &mut *(opaque as *mut Hook);
    st.n_free += 1;
    st.live -= 1;
    free(addr);
}

fn cmem_of(h: &mut Hook, with_calloc: bool) -> LZ4F_CustomMem {
    LZ4F_CustomMem {
        customAlloc: Some(hook_alloc),
        customCalloc: if with_calloc { Some(hook_calloc) } else { None },
        customFree: Some(hook_free),
        opaqueState: h as *mut Hook as *mut c_void,
    }
}

// ---------------------------------------------------------------------------
// lz4frame FFI signatures
// ---------------------------------------------------------------------------

type FnU32ToUsize = unsafe extern "C" fn(c_uint) -> usize;
type FnBound = unsafe extern "C" fn(usize, *const LZ4F_preferences_t) -> usize;
type FnCompressFrame = unsafe extern "C" fn(
    *mut c_void,
    usize,
    *const c_void,
    usize,
    *const LZ4F_preferences_t,
) -> usize;
type FnCompressFrameCDict = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    usize,
    *const c_void,
    usize,
    *const c_void,
    *const LZ4F_preferences_t,
) -> usize;
type FnCreateCtx = unsafe extern "C" fn(*mut *mut c_void, c_uint) -> usize;
type FnCreateCtxAdv = unsafe extern "C" fn(LZ4F_CustomMem, c_uint) -> *mut c_void;
type FnFreeCtx = unsafe extern "C" fn(*mut c_void) -> usize;
type FnBegin =
    unsafe extern "C" fn(*mut c_void, *mut c_void, usize, *const LZ4F_preferences_t) -> usize;
type FnBeginDict = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    usize,
    *const c_void,
    usize,
    *const LZ4F_preferences_t,
) -> usize;
type FnBeginCDict = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    usize,
    *const c_void,
    *const LZ4F_preferences_t,
) -> usize;
type FnUpdate = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    usize,
    *const c_void,
    usize,
    *const LZ4F_compressOptions_t,
) -> usize;
type FnFlushEnd =
    unsafe extern "C" fn(*mut c_void, *mut c_void, usize, *const LZ4F_compressOptions_t) -> usize;
type FnDecompress = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    *mut usize,
    *const c_void,
    *mut usize,
    *const LZ4F_decompressOptions_t,
) -> usize;
type FnCreateCDict = unsafe extern "C" fn(*const c_void, usize) -> *mut c_void;
type FnCreateCDictAdv =
    unsafe extern "C" fn(LZ4F_CustomMem, *const c_void, usize) -> *mut c_void;
type FnFreeCDict = unsafe extern "C" fn(*mut c_void);
type FnIsErr = unsafe extern "C" fn(usize) -> c_uint;
type FnErrName = unsafe extern "C" fn(usize) -> *const c_char;
type FnErrCode = unsafe extern "C" fn(usize) -> c_int;
type FnVoidToU = unsafe extern "C" fn() -> c_uint;
type FnVoidToI = unsafe extern "C" fn() -> c_int;

#[derive(Copy, Clone)]
struct Api {
    tag: &'static str,
    getBlockSize: FnU32ToUsize,
    compressBound: FnBound,
    compressFrameBound: FnBound,
    compressFrame: FnCompressFrame,
    compressFrame_usingCDict: FnCompressFrameCDict,
    createCCtx: FnCreateCtx,
    createCCtxAdv: FnCreateCtxAdv,
    freeCCtx: FnFreeCtx,
    createDCtx: FnCreateCtx,
    createDCtxAdv: FnCreateCtxAdv,
    freeDCtx: FnFreeCtx,
    decompress: FnDecompress,
    compressBegin: FnBegin,
    compressBegin_usingDict: FnBeginDict,
    compressBegin_usingCDict: FnBeginCDict,
    compressUpdate: FnUpdate,
    uncompressedUpdate: FnUpdate,
    flush: FnFlushEnd,
    compressEnd: FnFlushEnd,
    createCDict: FnCreateCDict,
    createCDictAdv: FnCreateCDictAdv,
    freeCDict: FnFreeCDict,
    isError: FnIsErr,
    getErrorName: FnErrName,
    getErrorCode: FnErrCode,
    getVersion: FnVoidToU,
    compressionLevel_max: FnVoidToI,
}

macro_rules! p2 {
    ($l:expr, $t:ty, $n:expr) => {{
        let (a, b) = $l.sym::<$t>($n);
        (*a, *b)
    }};
}

unsafe fn apis() -> (Api, Api) {
    let l = libs();
    {
        // Paranoia: the two libraries must really be two distinct code objects.
        let (a, b) = l.sym::<FnCompressFrame>("LZ4F_compressFrame");
        assert_ne!(
            *a as usize, *b as usize,
            "harness bug: LZ4F_compressFrame resolved to the same address in both libraries"
        );
    }
    let gbs = p2!(l, FnU32ToUsize, "LZ4F_getBlockSize");
    let cb = p2!(l, FnBound, "LZ4F_compressBound");
    let cfb = p2!(l, FnBound, "LZ4F_compressFrameBound");
    let cf = p2!(l, FnCompressFrame, "LZ4F_compressFrame");
    let cfcd = p2!(l, FnCompressFrameCDict, "LZ4F_compressFrame_usingCDict");
    let ccc = p2!(l, FnCreateCtx, "LZ4F_createCompressionContext");
    let ccca = p2!(l, FnCreateCtxAdv, "LZ4F_createCompressionContext_advanced");
    let fcc = p2!(l, FnFreeCtx, "LZ4F_freeCompressionContext");
    let cdc = p2!(l, FnCreateCtx, "LZ4F_createDecompressionContext");
    let cdca = p2!(l, FnCreateCtxAdv, "LZ4F_createDecompressionContext_advanced");
    let fdc = p2!(l, FnFreeCtx, "LZ4F_freeDecompressionContext");
    let dec = p2!(l, FnDecompress, "LZ4F_decompress");
    let cbg = p2!(l, FnBegin, "LZ4F_compressBegin");
    let cbgd = p2!(l, FnBeginDict, "LZ4F_compressBegin_usingDict");
    let cbgcd = p2!(l, FnBeginCDict, "LZ4F_compressBegin_usingCDict");
    let cu = p2!(l, FnUpdate, "LZ4F_compressUpdate");
    let uu = p2!(l, FnUpdate, "LZ4F_uncompressedUpdate");
    let fl = p2!(l, FnFlushEnd, "LZ4F_flush");
    let ce = p2!(l, FnFlushEnd, "LZ4F_compressEnd");
    let ccd = p2!(l, FnCreateCDict, "LZ4F_createCDict");
    let ccda = p2!(l, FnCreateCDictAdv, "LZ4F_createCDict_advanced");
    let fcd = p2!(l, FnFreeCDict, "LZ4F_freeCDict");
    let ie = p2!(l, FnIsErr, "LZ4F_isError");
    let en = p2!(l, FnErrName, "LZ4F_getErrorName");
    let ec = p2!(l, FnErrCode, "LZ4F_getErrorCode");
    let gv = p2!(l, FnVoidToU, "LZ4F_getVersion");
    let clm = p2!(l, FnVoidToI, "LZ4F_compressionLevel_max");

    macro_rules! mk {
        ($tag:expr, $i:tt) => {
            Api {
                tag: $tag,
                getBlockSize: gbs.$i,
                compressBound: cb.$i,
                compressFrameBound: cfb.$i,
                compressFrame: cf.$i,
                compressFrame_usingCDict: cfcd.$i,
                createCCtx: ccc.$i,
                createCCtxAdv: ccca.$i,
                freeCCtx: fcc.$i,
                createDCtx: cdc.$i,
                createDCtxAdv: cdca.$i,
                freeDCtx: fdc.$i,
                decompress: dec.$i,
                compressBegin: cbg.$i,
                compressBegin_usingDict: cbgd.$i,
                compressBegin_usingCDict: cbgcd.$i,
                compressUpdate: cu.$i,
                uncompressedUpdate: uu.$i,
                flush: fl.$i,
                compressEnd: ce.$i,
                createCDict: ccd.$i,
                createCDictAdv: ccda.$i,
                freeCDict: fcd.$i,
                isError: ie.$i,
                getErrorName: en.$i,
                getErrorCode: ec.$i,
                getVersion: gv.$i,
                compressionLevel_max: clm.$i,
            }
        };
    }
    (mk!("C", 0), mk!("Rust", 1))
}

// ---------------------------------------------------------------------------
// small helpers
// ---------------------------------------------------------------------------

/// Compare two return values that may be `(size_t)-code` sentinels.
#[track_caller]
fn same(ctx: &str, c: usize, r: usize) {
    assert_eq!(
        c as isize, r as isize,
        "{ctx}: return mismatch (C={c:#x} / {} , Rust={r:#x} / {})",
        c as isize, r as isize
    );
}

#[track_caller]
fn same_vec(ctx: &str, c: &[usize], r: &[usize]) {
    let cs: Vec<isize> = c.iter().map(|&x| x as isize).collect();
    let rs: Vec<isize> = r.iter().map(|&x| x as isize).collect();
    assert_eq!(cs, rs, "{ctx}: return vectors differ");
}

#[track_caller]
fn expect(ctx: &str, got: usize, want: usize) {
    assert_eq!(
        got as isize, want as isize,
        "{ctx}: expected {:#x} (LZ4F code {}), got {:#x} (LZ4F code {})",
        want,
        (0usize).wrapping_sub(want) as isize,
        got,
        (0usize).wrapping_sub(got) as isize
    );
}

unsafe fn new_cctx(api: &Api) -> *mut c_void {
    let mut p: *mut c_void = ptr::null_mut();
    let r = (api.createCCtx)(&mut p, LZ4F_VERSION);
    assert_eq!(r, 0, "{}: LZ4F_createCompressionContext failed {r:#x}", api.tag);
    assert!(!p.is_null(), "{}: NULL cctx despite success", api.tag);
    p
}

fn prefs_default() -> LZ4F_preferences_t {
    LZ4F_preferences_t::default()
}

/// The three "interesting" enum sweeps, as raw `unsigned` values crossing the
/// FFI boundary. C enums accept any `int`, so all of these are real inputs.
const BSID_SWEEP: [c_uint; 14] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 100, 0x7FFF_FFFF, 0x8000_0000, 0xFFFF_FFFF,
];
const BLOCKMODE_SWEEP: [c_uint; 6] = [0, 1, 2, 3, 7, 0xFFFF_FFFF];
const CCFLAG_SWEEP: [c_uint; 4] = [0, 1, 2, 0xFFFF_FFFF];
const BCFLAG_SWEEP: [c_uint; 4] = [0, 1, 2, 0xFFFF_FFFF];
const FRAMETYPE_SWEEP: [c_uint; 4] = [0, 1, 2, 0xFFFF_FFFF];

// ===========================================================================
// ERRORS row 1 — LZ4F_getBlockSize
// ===========================================================================

/// Row 1: `blockSizeID` (after `0 -> LZ4F_max64KB`) outside `4..=7` is
/// `LZ4F_ERROR_maxBlockSize_invalid` = `(size_t)-2` (lz4frame.c:338-339).
/// Also the full out-of-range enum sweep required by the FFI-boundary section.
#[test]
fn err_1_getBlockSize_rejects_out_of_range_blockSizeID() {
    unsafe {
        let (c, r) = apis();

        let mut ids: Vec<c_uint> = BSID_SWEEP.to_vec();
        ids.extend_from_slice(&[10, 11, 12, 63, 64, 255, 256, 0x7FFF_FFFE, 0x8000_0001]);
        // property axis: many random 32-bit ids with a fixed seed
        let mut rng = Rng::new(1);
        for _ in 0..2000 {
            ids.push(rng.next_u32());
        }

        for &id in &ids {
            let a = (c.getBlockSize)(id);
            let b = (r.getBlockSize)(id);
            same(&format!("row1: LZ4F_getBlockSize({id:#x})"), a, b);
            let want = match id {
                0 | 4 => 64 * 1024,
                5 => 256 * 1024,
                6 => 1024 * 1024,
                7 => 4 * 1024 * 1024,
                _ => err(2),
            };
            expect(&format!("row1: LZ4F_getBlockSize({id:#x})"), a, want);
        }
    }
}

// ===========================================================================
// ERRORS row 2 — LZ4F_compressFrame_usingCDict, dstCapacity too small
// ===========================================================================

/// Row 2: `dstCapacity < LZ4F_compressFrameBound(srcSize, &prefs)` is
/// `LZ4F_ERROR_dstMaxSize_tooSmall` = `(size_t)-11` (lz4frame.c:456).
/// Property-style: many shapes/sizes, each probed at `bound-1`, `bound/2`, `1`
/// and `0`, and confirmed to succeed at exactly `bound`.
#[test]
fn err_2_compressFrame_usingCDict_dstCapacity_below_bound() {
    unsafe {
        let (c, r) = apis();
        let mut rng = Rng::new(2);

        for shape in ALL_SHAPES {
            for &n in &[0usize, 1, 7, 100, 4096, 65535, 65536, 65537, 200_000] {
                let src = gen(&mut rng, shape, n);
                let mut prefs = prefs_default();
                prefs.frameInfo.blockSizeID = [LZ4F_DEFAULT, LZ4F_MAX64KB, LZ4F_MAX1MB][rng.below(3)];
                prefs.frameInfo.contentChecksumFlag = rng.below(2) as c_uint;
                prefs.frameInfo.blockChecksumFlag = rng.below(2) as c_uint;
                prefs.compressionLevel = [0i32, 1, 9, 12][rng.below(4)];

                // The bound compressFrame_usingCDict actually checks is computed
                // from the *adjusted* prefs, so ask both libraries and let the
                // observable behaviour (not our prediction) drive the test.
                let cb = (c.compressFrameBound)(n, &prefs);
                let rb = (r.compressFrameBound)(n, &prefs);
                same(&format!("row2: compressFrameBound({n})"), cb, rb);
                assert!(cb > 19 && cb < 8 << 20, "row2: implausible bound {cb} for n={n}");

                let mut caps: Vec<usize> = vec![0, 1, 19, cb / 2, cb - 1];
                caps.retain(|&x| x < cb);
                caps.sort();
                caps.dedup();

                for &cap in &caps {
                    let mut cd = vec![0xCDu8; cb + 16];
                    let mut rd = vec![0xCDu8; cb + 16];
                    let cctx_c = new_cctx(&c);
                    let cctx_r = new_cctx(&r);
                    let a = (c.compressFrame_usingCDict)(
                        cctx_c,
                        cd.as_mut_ptr() as *mut c_void,
                        cap,
                        src.as_ptr() as *const c_void,
                        n,
                        ptr::null(),
                        &prefs,
                    );
                    let b = (r.compressFrame_usingCDict)(
                        cctx_r,
                        rd.as_mut_ptr() as *mut c_void,
                        cap,
                        src.as_ptr() as *const c_void,
                        n,
                        ptr::null(),
                        &prefs,
                    );
                    (c.freeCCtx)(cctx_c);
                    (r.freeCCtx)(cctx_r);
                    let ctx = format!("row2: {shape:?} n={n} cap={cap} (bound={cb})");
                    same(&ctx, a, b);
                    expect(&ctx, a, err(11));
                    same_full_buffers(&ctx, &cd, &rd);
                }

                // exactly `bound` must succeed
                let mut cd = vec![0xCDu8; cb + 16];
                let mut rd = vec![0xCDu8; cb + 16];
                let cctx_c = new_cctx(&c);
                let cctx_r = new_cctx(&r);
                let a = (c.compressFrame_usingCDict)(
                    cctx_c,
                    cd.as_mut_ptr() as *mut c_void,
                    cb,
                    src.as_ptr() as *const c_void,
                    n,
                    ptr::null(),
                    &prefs,
                );
                let b = (r.compressFrame_usingCDict)(
                    cctx_r,
                    rd.as_mut_ptr() as *mut c_void,
                    cb,
                    src.as_ptr() as *const c_void,
                    n,
                    ptr::null(),
                    &prefs,
                );
                (c.freeCCtx)(cctx_c);
                (r.freeCCtx)(cctx_r);
                let ctx = format!("row2: {shape:?} n={n} cap=bound={cb}");
                same(&ctx, a, b);
                assert!(!is_err_range(a), "{ctx}: dstCapacity == bound must succeed, got {a:#x}");
                same_full_buffers(&ctx, &cd, &rd);
            }
        }
    }
}

// ===========================================================================
// ERRORS row 3 — LZ4F_compressFrame_usingCDict forwards inner errors
// ===========================================================================

/// Row 3: an error from `LZ4F_compressBegin_usingCDict` / `LZ4F_compressUpdate`
/// / `LZ4F_compressEnd` is forwarded verbatim through `FORWARD_IF_ERROR`
/// (lz4frame.c:459, 464, 469).
///
/// Forced with a failing custom allocator: the cctx itself is created (alloc
/// #1), then the `LZ4_stream_t` allocation (#2) or the `tmpBuff` allocation
/// (#3) inside `LZ4F_compressBegin_internal` is made to fail, so
/// `LZ4F_ERROR_allocation_failed` = `(size_t)-9` must surface out of
/// `LZ4F_compressFrame_usingCDict`.
#[test]
fn err_3_compressFrame_usingCDict_forwards_inner_error() {
    unsafe {
        let (c, r) = apis();
        let mut rng = Rng::new(3);
        // > 64 KB, so compressFrame keeps blockLinked and therefore also needs
        // the 64 KB tmpBuff allocation (alloc #3).
        let src = gen(&mut rng, Shape::TextLike, 200_000);
        let prefs = prefs_default();

        for &fail_at in &[2i64, 3] {
            for &with_calloc in &[true, false] {
                let mut rets: Vec<usize> = Vec::new();
                for api in [&c, &r] {
                    let mut hook = Hook::new(fail_at);
                    let cm = cmem_of(&mut hook, with_calloc);
                    let cctx = (api.createCCtxAdv)(cm, LZ4F_VERSION);
                    assert!(
                        !cctx.is_null(),
                        "{}: cctx creation must succeed for fail_at={fail_at}",
                        api.tag
                    );
                    let cap = (api.compressFrameBound)(src.len(), &prefs);
                    let mut dst = vec![0xCDu8; cap];
                    let ret = (api.compressFrame_usingCDict)(
                        cctx,
                        dst.as_mut_ptr() as *mut c_void,
                        cap,
                        src.as_ptr() as *const c_void,
                        src.len(),
                        ptr::null(),
                        &prefs,
                    );
                    (api.freeCCtx)(cctx);
                    assert_eq!(
                        hook.live, 0,
                        "{}: {} allocations leaked (fail_at={fail_at})",
                        api.tag, hook.live
                    );
                    rets.push(ret);
                }
                let ctx = format!("row3: fail_at={fail_at} with_calloc={with_calloc}");
                same(&ctx, rets[0], rets[1]);
                expect(&ctx, rets[0], err(9));
            }
        }

        // Sanity: with a *working* custom allocator the same call succeeds, so
        // the failure above really is the injected one.
        let mut rets: Vec<usize> = Vec::new();
        for api in [&c, &r] {
            let mut hook = Hook::new(0);
            let cm = cmem_of(&mut hook, true);
            let cctx = (api.createCCtxAdv)(cm, LZ4F_VERSION);
            assert!(!cctx.is_null());
            let cap = (api.compressFrameBound)(src.len(), &prefs);
            let mut dst = vec![0xCDu8; cap];
            let ret = (api.compressFrame_usingCDict)(
                cctx,
                dst.as_mut_ptr() as *mut c_void,
                cap,
                src.as_ptr() as *const c_void,
                src.len(),
                ptr::null(),
                &prefs,
            );
            (api.freeCCtx)(cctx);
            assert_eq!(hook.live, 0, "{}: leaked allocations", api.tag);
            assert!(hook.calls() >= 3, "{}: expected >= 3 allocations, saw {}", api.tag, hook.calls());
            rets.push(ret);
        }
        same("row3 control", rets[0], rets[1]);
        assert!(!is_err_range(rets[0]), "row3 control: unexpected error {:#x}", rets[0]);
    }
}

// ===========================================================================
// ERRORS rows 4 + 5 — LZ4F_createCDict_advanced allocation failures
// ===========================================================================

/// Row 4: `LZ4F_malloc(sizeof(LZ4F_CDict))` fails -> `NULL`
/// (lz4frame.c:541-544). Forced with an unconditionally-failing allocator and
/// with `fail_at == 1`.
#[test]
fn err_4_createCDict_advanced_first_allocation_fails() {
    unsafe {
        let (c, r) = apis();
        let mut rng = Rng::new(4);
        let dict = gen(&mut rng, Shape::TextLike, 5000);

        for &fail_at in &[-1i64, 1] {
            for &with_calloc in &[true, false] {
                let mut nulls = Vec::new();
                for api in [&c, &r] {
                    let mut hook = Hook::new(fail_at);
                    let cm = cmem_of(&mut hook, with_calloc);
                    let cd = (api.createCDictAdv)(
                        cm,
                        dict.as_ptr() as *const c_void,
                        dict.len(),
                    );
                    nulls.push(cd.is_null());
                    if !cd.is_null() {
                        (api.freeCDict)(cd);
                    }
                    assert_eq!(hook.live, 0, "{}: leaked (fail_at={fail_at})", api.tag);
                    assert_eq!(
                        hook.calls(),
                        1,
                        "{}: the first allocation must be the only one attempted",
                        api.tag
                    );
                }
                assert_eq!(
                    nulls[0], nulls[1],
                    "row4: NULL-ness differs (C={} Rust={}) fail_at={fail_at}",
                    nulls[0], nulls[1]
                );
                assert!(nulls[0], "row4: expected NULL from LZ4F_createCDict_advanced");
            }
        }
    }
}

/// Row 5: any of the `dictContent` / `fastCtx` / `HCCtx` allocations failing
/// frees the partial cdict and returns `NULL` (lz4frame.c:550-557).
/// The failure index is swept over 1..=8 to hit every distinct allocation site,
/// and the free-hook proves the partial cdict really was released.
#[test]
fn err_5_createCDict_advanced_inner_allocation_fails() {
    unsafe {
        let (c, r) = apis();
        let mut rng = Rng::new(5);

        for &dict_len in &[1usize, 8, 5000, 64 * 1024, 100_000] {
            let dict = gen(&mut rng, Shape::TextLike, dict_len);
            for fail_at in 1i64..=8 {
                for &with_calloc in &[true, false] {
                    let mut obs: Vec<(bool, u64, i64)> = Vec::new();
                    for api in [&c, &r] {
                        let mut hook = Hook::new(fail_at);
                        let cm = cmem_of(&mut hook, with_calloc);
                        let cd = (api.createCDictAdv)(
                            cm,
                            dict.as_ptr() as *const c_void,
                            dict.len(),
                        );
                        let was_null = cd.is_null();
                        if !cd.is_null() {
                            (api.freeCDict)(cd);
                        }
                        assert_eq!(
                            hook.live, 0,
                            "{}: leaked {} allocations (dict_len={dict_len} fail_at={fail_at})",
                            api.tag, hook.live
                        );
                        obs.push((was_null, hook.calls(), hook.n_free as i64));
                    }
                    let ctx =
                        format!("row5: dict_len={dict_len} fail_at={fail_at} calloc={with_calloc}");
                    assert_eq!(obs[0], obs[1], "{ctx}: observations differ {obs:?}");
                    // 4 allocation sites: cdict, dictContent, fastCtx, HCCtx.
                    // A failure of the very first one bails out immediately
                    // (lz4frame.c:544), the other three are all attempted
                    // before the NULL check at lz4frame.c:554.
                    let want_calls = if fail_at == 1 { 1 } else { 4 };
                    assert_eq!(
                        obs[0].1, want_calls,
                        "{ctx}: expected {want_calls} allocation attempts, saw {}",
                        obs[0].1
                    );
                    if fail_at <= 4 {
                        assert!(obs[0].0, "{ctx}: expected NULL");
                    } else {
                        assert!(!obs[0].0, "{ctx}: expected a valid CDict");
                    }
                }
            }
        }
    }
}

// ===========================================================================
// ERRORS row 6 — LZ4F_createCompressionContext_advanced allocation failure
// ===========================================================================

/// Row 6: `LZ4F_calloc(sizeof(LZ4F_cctx))` fails -> `NULL`
/// (lz4frame.c:598-600). Forced for real; the failure index is swept so we also
/// prove there is exactly ONE allocation inside this constructor.
#[test]
fn err_6_createCompressionContext_advanced_allocation_fails() {
    unsafe {
        let (c, r) = apis();
        for fail_at in [-1i64, 1, 2, 3, 4, 5, 6, 7, 8] {
            for &with_calloc in &[true, false] {
                for &version in &[0u32, 99, LZ4F_VERSION, 101, 0xFFFF_FFFF] {
                    let mut obs: Vec<(bool, u64)> = Vec::new();
                    for api in [&c, &r] {
                        let mut hook = Hook::new(fail_at);
                        let cm = cmem_of(&mut hook, with_calloc);
                        let ctx = (api.createCCtxAdv)(cm, version);
                        let was_null = ctx.is_null();
                        if !ctx.is_null() {
                            let fr = (api.freeCCtx)(ctx);
                            assert_eq!(fr, 0, "{}: freeCompressionContext != 0", api.tag);
                        }
                        assert_eq!(hook.live, 0, "{}: leaked", api.tag);
                        obs.push((was_null, hook.calls()));
                    }
                    let ctx = format!("row6: fail_at={fail_at} calloc={with_calloc} version={version}");
                    assert_eq!(obs[0], obs[1], "{ctx}: differ {obs:?}");
                    assert_eq!(obs[0].1, 1, "{ctx}: expected exactly 1 allocation");
                    assert_eq!(
                        obs[0].0,
                        fail_at < 0 || fail_at == 1,
                        "{ctx}: unexpected NULL-ness {}",
                        obs[0].0
                    );
                }
            }
        }
    }
}

// ===========================================================================
// ERRORS row 7 — LZ4F_createCompressionContext(NULL, ...)
// ===========================================================================

/// Row 7: `LZ4F_compressionContextPtr == NULL` ->
/// `LZ4F_ERROR_parameter_null` = `(size_t)-21` (lz4frame.c:622).
#[test]
fn err_7_createCompressionContext_null_pointer() {
    unsafe {
        let (c, r) = apis();
        for &version in &[0u32, 1, 99, LZ4F_VERSION, 101, 0xFFFF_FFFF] {
            let a = (c.createCCtx)(ptr::null_mut(), version);
            let b = (r.createCCtx)(ptr::null_mut(), version);
            let ctx = format!("row7: createCompressionContext(NULL, {version})");
            same(&ctx, a, b);
            expect(&ctx, a, err(21));
        }
    }
}

// ===========================================================================
// ERRORS row 8 — LZ4F_createCompressionContext allocation_failed
// ===========================================================================

/// Row 8: the inner `LZ4F_createCompressionContext_advanced` returning NULL is
/// reported as `LZ4F_ERROR_allocation_failed` = `(size_t)-9`
/// (lz4frame.c:625).
///
/// NOT FORCEABLE from a test: `LZ4F_createCompressionContext` hard-codes
/// `LZ4F_defaultCMem`, so the allocation goes straight to libc `calloc()` for a
/// few-hundred-byte object. Failing it would require interposing `calloc` for
/// the whole process (which would also break the Rust test harness's own
/// allocator) or exhausting the address space. There is no allocator hook on
/// this entry point.
///
/// What is asserted instead is the closest reachable observable behaviour:
///   * the *advanced* constructor returning NULL under an injected failure is
///     byte-identical between the two libraries (this is the branch condition
///     row 8 tests), and
///   * with a working allocator both libraries return `LZ4F_OK_NoError` and a
///     non-NULL context, i.e. neither takes the row-8 branch spuriously.
#[test]
fn err_8_createCompressionContext_allocation_failed_is_unforceable() {
    unsafe {
        let (c, r) = apis();

        // (a) the branch *condition* of row 8, forced through the advanced ctor
        for api in [&c, &r] {
            let mut hook = Hook::new(-1);
            let cm = cmem_of(&mut hook, true);
            let p = (api.createCCtxAdv)(cm, LZ4F_VERSION);
            assert!(
                p.is_null(),
                "{}: row8 precondition: advanced ctor must return NULL when calloc fails",
                api.tag
            );
        }

        // (b) the normal path: LZ4F_OK_NoError and a usable context
        for &version in &[0u32, LZ4F_VERSION, 0xFFFF_FFFF] {
            let mut pc: *mut c_void = ptr::null_mut();
            let mut pr: *mut c_void = ptr::null_mut();
            let a = (c.createCCtx)(&mut pc, version);
            let b = (r.createCCtx)(&mut pr, version);
            let ctx = format!("row8: createCompressionContext(&p, {version})");
            same(&ctx, a, b);
            expect(&ctx, a, 0);
            assert!(!pc.is_null() && !pr.is_null(), "{ctx}: NULL context despite success");
            same(&ctx, (c.freeCCtx)(pc), (r.freeCCtx)(pr));
        }

        // (c) pin the sentinel row 8 *would* return, so the constant is covered
        // by an executed comparison of both libraries' error machinery.
        let a = (c.getErrorName)(err(9));
        let b = (r.getErrorName)(err(9));
        assert_eq!(
            CStr::from_ptr(a).to_bytes(),
            CStr::from_ptr(b).to_bytes(),
            "row8: getErrorName(err(9)) differs"
        );
        assert_eq!(
            CStr::from_ptr(a).to_bytes(),
            b"ERROR_allocation_failed",
            "row8: unexpected name for err(9)"
        );
    }
}

// ===========================================================================
// ERRORS row 9 — LZ4F_compressBegin_internal, dstCapacity < maxFHSize
// ===========================================================================

/// Row 9: `dstCapacity < maxFHSize (19)` ->
/// `LZ4F_ERROR_dstMaxSize_tooSmall` = `(size_t)-11` (lz4frame.c:700).
/// Swept over every capacity 0..=24 and over all three `compressBegin*`
/// entry points, plus a preference matrix (the header length varies with
/// contentSize / dictID, but the *check* is always against 19).
#[test]
fn err_9_compressBegin_dstCapacity_below_maxFHSize() {
    unsafe {
        let (c, r) = apis();
        let mut rng = Rng::new(9);
        let dict = gen(&mut rng, Shape::TextLike, 1234);

        let mut prefsets: Vec<LZ4F_preferences_t> = Vec::new();
        prefsets.push(prefs_default());
        for &(cs, did, lvl) in &[
            (0u64, 0u32, 0i32),
            (1234, 0, 1),
            (0, 0xDEAD_BEEF, 9),
            (1234, 0xDEAD_BEEF, 12),
            (u64::MAX, 0xFFFF_FFFF, -5),
        ] {
            let mut p = prefs_default();
            p.frameInfo.contentSize = cs;
            p.frameInfo.dictID = did;
            p.compressionLevel = lvl;
            prefsets.push(p);
        }

        for (pi, prefs) in prefsets.iter().enumerate() {
            for cap in 0usize..=24 {
                // LZ4F_compressBegin
                let mut cv = Vec::new();
                let mut rv = Vec::new();
                let mut cd = vec![0xCDu8; 32];
                let mut rd = vec![0xCDu8; 32];
                for (api, out, dst) in [
                    (&c, &mut cv, &mut cd),
                    (&r, &mut rv, &mut rd),
                ] {
                    let cctx = new_cctx(api);
                    out.push((api.compressBegin)(
                        cctx,
                        dst.as_mut_ptr() as *mut c_void,
                        cap,
                        prefs,
                    ));
                    out.push((api.compressBegin_usingDict)(
                        cctx,
                        dst.as_mut_ptr() as *mut c_void,
                        cap,
                        dict.as_ptr() as *const c_void,
                        dict.len(),
                        prefs,
                    ));
                    out.push((api.compressBegin_usingCDict)(
                        cctx,
                        dst.as_mut_ptr() as *mut c_void,
                        cap,
                        ptr::null(),
                        prefs,
                    ));
                    (api.freeCCtx)(cctx);
                }
                let ctx = format!("row9: prefs#{pi} cap={cap}");
                same_vec(&ctx, &cv, &rv);
                same_full_buffers(&ctx, &cd, &rd);
                if cap < 19 {
                    for (i, &v) in cv.iter().enumerate() {
                        expect(&format!("{ctx} entry#{i}"), v, err(11));
                    }
                } else {
                    for (i, &v) in cv.iter().enumerate() {
                        assert!(
                            !is_err_range(v),
                            "{ctx} entry#{i}: cap >= 19 must succeed, got {v:#x}"
                        );
                    }
                }
            }
        }
    }
}

// ===========================================================================
// ERRORS rows 10 + 11 — allocation failures inside LZ4F_compressBegin_internal
// ===========================================================================

/// Row 10: the `LZ4_stream_t` / `LZ4_streamHC_t` allocation returning NULL is
/// `LZ4F_ERROR_allocation_failed` = `(size_t)-9` (lz4frame.c:714-722).
/// Forced: allocation #1 creates the cctx, allocation #2 is exactly this one.
#[test]
fn err_10_compressBegin_lz4ctx_allocation_fails() {
    unsafe {
        let (c, r) = apis();
        // level < 2 -> LZ4_stream_t, level >= 2 -> LZ4_streamHC_t: both go
        // through the same RETURN_ERROR_IF at lz4frame.c:722.
        for &lvl in &[-3i32, 0, 1, 2, 9, 12, 100] {
            for &with_calloc in &[true, false] {
                let mut rets = Vec::new();
                for api in [&c, &r] {
                    let mut hook = Hook::new(2);
                    let cm = cmem_of(&mut hook, with_calloc);
                    let cctx = (api.createCCtxAdv)(cm, LZ4F_VERSION);
                    assert!(!cctx.is_null(), "{}: cctx creation must succeed", api.tag);
                    let mut prefs = prefs_default();
                    prefs.compressionLevel = lvl;
                    let mut dst = vec![0xCDu8; 64];
                    let ret = (api.compressBegin)(
                        cctx,
                        dst.as_mut_ptr() as *mut c_void,
                        dst.len(),
                        &prefs,
                    );
                    (api.freeCCtx)(cctx);
                    assert_eq!(hook.live, 0, "{}: leaked (level={lvl})", api.tag);
                    assert_eq!(hook.calls(), 2, "{}: expected 2 allocation attempts", api.tag);
                    rets.push(ret);
                }
                let ctx = format!("row10: level={lvl} calloc={with_calloc}");
                same(&ctx, rets[0], rets[1]);
                expect(&ctx, rets[0], err(9));
            }
        }
    }
}

/// Row 11: `LZ4F_malloc(requiredBuffSize)` for `cctx->tmpBuff` returning NULL
/// is `LZ4F_ERROR_allocation_failed` = `(size_t)-9` (lz4frame.c:749-750).
/// Forced with `fail_at == 3` (cctx, lz4Ctx, tmpBuff). The failure index is
/// also swept over 1..=8 so every allocation site of the whole
/// create+begin sequence is exercised.
#[test]
fn err_11_compressBegin_tmpBuff_allocation_fails() {
    unsafe {
        let (c, r) = apis();

        // autoFlush == 0 -> requiredBuffSize = maxBlockSize (+128 KB when
        // blockLinked) > 0, so the tmpBuff allocation always happens.
        for &(bsid, bmode, aflush) in &[
            (LZ4F_DEFAULT, LZ4F_BLOCK_LINKED, 0u32),
            (LZ4F_MAX64KB, LZ4F_BLOCK_INDEPENDENT, 0),
            (LZ4F_MAX4MB, LZ4F_BLOCK_LINKED, 0),
            (LZ4F_MAX256KB, LZ4F_BLOCK_LINKED, 1),
        ] {
            let mut prefs = prefs_default();
            prefs.frameInfo.blockSizeID = bsid;
            prefs.frameInfo.blockMode = bmode;
            prefs.autoFlush = aflush;

            for fail_at in 1i64..=8 {
                let mut obs: Vec<(usize, u64)> = Vec::new();
                for api in [&c, &r] {
                    let mut hook = Hook::new(fail_at);
                    let cm = cmem_of(&mut hook, true);
                    let cctx = (api.createCCtxAdv)(cm, LZ4F_VERSION);
                    let ret: usize;
                    if cctx.is_null() {
                        ret = err(9); // stands in for "context creation failed"
                    } else {
                        let mut dst = vec![0xCDu8; 64];
                        ret = (api.compressBegin)(
                            cctx,
                            dst.as_mut_ptr() as *mut c_void,
                            dst.len(),
                            &prefs,
                        );
                        (api.freeCCtx)(cctx);
                    }
                    assert_eq!(hook.live, 0, "{}: leaked (fail_at={fail_at})", api.tag);
                    obs.push((ret, hook.calls()));
                }
                let ctx = format!(
                    "row11: bsid={bsid} bmode={bmode} autoFlush={aflush} fail_at={fail_at}"
                );
                assert_eq!(
                    obs[0].0 as isize, obs[1].0 as isize,
                    "{ctx}: return mismatch (C={:#x} Rust={:#x})",
                    obs[0].0, obs[1].0
                );
                assert_eq!(obs[0].1, obs[1].1, "{ctx}: allocation counts differ {obs:?}");
                // there are exactly 3 allocation sites in create+begin
                if fail_at <= 3 {
                    expect(&ctx, obs[0].0, err(9));
                } else {
                    assert!(
                        !is_err_range(obs[0].0),
                        "{ctx}: expected success, got {:#x}",
                        obs[0].0
                    );
                }
                assert_eq!(obs[0].1, 3.min(fail_at as u64), "{ctx}: allocation attempts {obs:?}");
            }
        }
    }
}

// ===========================================================================
// ERRORS row 12 — LZ4F_compressBegin_usingDict, dictSize > INT_MAX
// ===========================================================================

/// Row 12: `dictBuffer != NULL && dictSize > INT_MAX` ->
/// `LZ4F_ERROR_parameter_invalid` = `(size_t)-4` (lz4frame.c:768).
///
/// `lz4frame.c:768` checks the length *before* the pointer is dereferenced
/// (`LZ4_loadDict` is only reached afterwards), so it is safe to hand a small
/// real buffer together with a lying oversized length.
#[test]
fn err_12_compressBegin_usingDict_dictSize_over_INT_MAX() {
    unsafe {
        let (c, r) = apis();
        let mut rng = Rng::new(12);
        let dict = gen(&mut rng, Shape::TextLike, 4096);

        const INT_MAX: usize = 0x7FFF_FFFF;
        let sizes = [
            INT_MAX + 1,
            INT_MAX + 2,
            0x8000_0000usize,
            0xFFFF_FFFF,
            0x1_0000_0000,
            usize::MAX / 2 + 1,
            usize::MAX - 1,
            usize::MAX,
        ];

        for &lvl in &[0i32, 1, 9, 12] {
            for &ds in &sizes {
                let mut prefs = prefs_default();
                prefs.compressionLevel = lvl;
                let mut rets = Vec::new();
                let mut bufs: Vec<Vec<u8>> = Vec::new();
                for api in [&c, &r] {
                    let cctx = new_cctx(api);
                    let mut dst = vec![0xCDu8; 64];
                    let ret = (api.compressBegin_usingDict)(
                        cctx,
                        dst.as_mut_ptr() as *mut c_void,
                        dst.len(),
                        dict.as_ptr() as *const c_void,
                        ds,
                        &prefs,
                    );
                    (api.freeCCtx)(cctx);
                    rets.push(ret);
                    bufs.push(dst);
                }
                let ctx = format!("row12: level={lvl} dictSize={ds:#x}");
                same(&ctx, rets[0], rets[1]);
                expect(&ctx, rets[0], err(4));
                same_full_buffers(&ctx, &bufs[0], &bufs[1]);
            }
        }

        // A dictBuffer of NULL skips the check entirely (lz4frame.c:766), even
        // with an absurd dictSize: the header is written normally.
        for &ds in &[0usize, 1, usize::MAX] {
            let mut rets = Vec::new();
            let mut bufs: Vec<Vec<u8>> = Vec::new();
            for api in [&c, &r] {
                let cctx = new_cctx(api);
                let mut dst = vec![0xCDu8; 64];
                let ret = (api.compressBegin_usingDict)(
                    cctx,
                    dst.as_mut_ptr() as *mut c_void,
                    dst.len(),
                    ptr::null(),
                    ds,
                    ptr::null(),
                );
                (api.freeCCtx)(cctx);
                rets.push(ret);
                bufs.push(dst);
            }
            let ctx = format!("row12: dictBuffer=NULL dictSize={ds:#x}");
            same(&ctx, rets[0], rets[1]);
            expect(&ctx, rets[0], 7);
            same_full_buffers(&ctx, &bufs[0], &bufs[1]);
        }
    }
}

// ===========================================================================
// ERRORS row 13 — LZ4F_compressUpdate with cStage != 1
// ===========================================================================

/// Row 13: `cctxPtr->cStage != 1` ->
/// `LZ4F_ERROR_compressionState_uninitialized` = `(size_t)-20`
/// (lz4frame.c:1005), i.e. `compressBegin` was never called, or the frame has
/// already been ended.
#[test]
fn err_13_compressUpdate_state_uninitialized() {
    unsafe {
        let (c, r) = apis();
        let mut rng = Rng::new(13);

        for shape in ALL_SHAPES {
            for &n in &[0usize, 1, 100, 70_000] {
                let src = gen(&mut rng, shape, n);
                let mut cv = Vec::new();
                let mut rv = Vec::new();
                for (api, out) in [(&c, &mut cv), (&r, &mut rv)] {
                    let cctx = new_cctx(api);
                    let mut dst = vec![0xCDu8; 1 << 20];
                    // (a) fresh context, compressBegin never called
                    out.push((api.compressUpdate)(
                        cctx,
                        dst.as_mut_ptr() as *mut c_void,
                        dst.len(),
                        src.as_ptr() as *const c_void,
                        n,
                        ptr::null(),
                    ));
                    out.push((api.uncompressedUpdate)(
                        cctx,
                        dst.as_mut_ptr() as *mut c_void,
                        dst.len(),
                        src.as_ptr() as *const c_void,
                        n,
                        ptr::null(),
                    ));
                    // (b) a complete frame, then update again
                    let mut prefs = prefs_default();
                    prefs.frameInfo.blockMode = LZ4F_BLOCK_INDEPENDENT;
                    out.push((api.compressBegin)(
                        cctx,
                        dst.as_mut_ptr() as *mut c_void,
                        dst.len(),
                        &prefs,
                    ));
                    out.push((api.compressUpdate)(
                        cctx,
                        dst.as_mut_ptr() as *mut c_void,
                        dst.len(),
                        src.as_ptr() as *const c_void,
                        n,
                        ptr::null(),
                    ));
                    out.push((api.compressEnd)(
                        cctx,
                        dst.as_mut_ptr() as *mut c_void,
                        dst.len(),
                        ptr::null(),
                    ));
                    // cStage is back to 0 -> uninitialized again
                    out.push((api.compressUpdate)(
                        cctx,
                        dst.as_mut_ptr() as *mut c_void,
                        dst.len(),
                        src.as_ptr() as *const c_void,
                        n,
                        ptr::null(),
                    ));
                    out.push((api.uncompressedUpdate)(
                        cctx,
                        dst.as_mut_ptr() as *mut c_void,
                        dst.len(),
                        src.as_ptr() as *const c_void,
                        n,
                        ptr::null(),
                    ));
                    (api.freeCCtx)(cctx);
                }
                let ctx = format!("row13: {shape:?} n={n}");
                same_vec(&ctx, &cv, &rv);
                expect(&format!("{ctx} fresh compressUpdate"), cv[0], err(20));
                expect(&format!("{ctx} fresh uncompressedUpdate"), cv[1], err(20));
                assert!(!is_err_range(cv[2]), "{ctx}: compressBegin failed {:#x}", cv[2]);
                assert!(!is_err_range(cv[3]), "{ctx}: compressUpdate failed {:#x}", cv[3]);
                assert!(!is_err_range(cv[4]), "{ctx}: compressEnd failed {:#x}", cv[4]);
                expect(&format!("{ctx} post-end compressUpdate"), cv[5], err(20));
                expect(&format!("{ctx} post-end uncompressedUpdate"), cv[6], err(20));
            }
        }
    }
}

// ===========================================================================
// ERRORS row 14 — LZ4F_compressUpdate dstCapacity below the internal bound
// ===========================================================================

/// Row 14: `dstCapacity < LZ4F_compressBound_internal(srcSize, prefs,
/// tmpInSize)` -> `LZ4F_ERROR_dstMaxSize_tooSmall` = `(size_t)-11`
/// (lz4frame.c:1006-1007).
///
/// A fresh `compressBegin` is issued before every probe so that `tmpInSize`
/// (which feeds the bound) is always 0 and every probe is independent.
#[test]
fn err_14_compressUpdate_dstCapacity_below_bound() {
    unsafe {
        let (c, r) = apis();
        let mut rng = Rng::new(14);

        for shape in ALL_SHAPES {
            for &n in &[0usize, 1, 5, 4096, 65535, 65536, 65537, 130_000] {
                let src = gen(&mut rng, shape, n);
                for &(af, cc, bc) in &[
                    (0u32, 0u32, 0u32),
                    (0, 1, 0),
                    (0, 0, 1),
                    (1, 0, 0),
                    (1, 1, 1),
                ] {
                    let mut prefs = prefs_default();
                    prefs.autoFlush = af;
                    prefs.frameInfo.contentChecksumFlag = cc;
                    prefs.frameInfo.blockChecksumFlag = bc;

                    let mut caps: Vec<usize> = vec![0, 1, 2, 3, 4, 7, 8, 11, 12, 100];
                    if n > 0 {
                        caps.push(n - 1);
                        caps.push(n);
                        caps.push(n + 4);
                    }
                    caps.push(n + 4096);
                    caps.sort();
                    caps.dedup();

                    let mut cv = Vec::new();
                    let mut rv = Vec::new();
                    for (api, out) in [(&c, &mut cv), (&r, &mut rv)] {
                        for &cap in &caps {
                            let cctx = new_cctx(api);
                            let mut hdr = vec![0xCDu8; 32];
                            let hr = (api.compressBegin)(
                                cctx,
                                hdr.as_mut_ptr() as *mut c_void,
                                hdr.len(),
                                &prefs,
                            );
                            assert!(!is_err_range(hr), "{}: compressBegin {hr:#x}", api.tag);
                            let mut dst = vec![0xCDu8; cap.max(1)];
                            out.push((api.compressUpdate)(
                                cctx,
                                dst.as_mut_ptr() as *mut c_void,
                                cap,
                                src.as_ptr() as *const c_void,
                                n,
                                ptr::null(),
                            ));
                            (api.freeCCtx)(cctx);
                        }
                    }
                    let ctx = format!("row14: {shape:?} n={n} af={af} cc={cc} bc={bc}");
                    same_vec(&ctx, &cv, &rv);
                    // every failure must be exactly err(11), and the accepted
                    // capacities must form a suffix (monotone threshold)
                    let mut seen_ok = false;
                    for (i, &v) in cv.iter().enumerate() {
                        if is_err_range(v) {
                            expect(&format!("{ctx} cap={}", caps[i]), v, err(11));
                            assert!(
                                !seen_ok,
                                "{ctx}: non-monotone acceptance at cap={}",
                                caps[i]
                            );
                        } else {
                            seen_ok = true;
                        }
                    }
                    assert!(seen_ok, "{ctx}: no capacity was ever accepted");
                    // dstCapacity 0 and 1 are always rejected with err(11)
                    expect(&format!("{ctx} cap=0"), cv[0], err(11));
                    expect(&format!("{ctx} cap=1"), cv[1], err(11));
                }
            }
        }
    }
}

// ===========================================================================
// ERRORS row 15 — LZ4F_uncompressedUpdate with dstCapacity < srcSize
// ===========================================================================

/// Row 15: `blockCompression == LZ4B_UNCOMPRESSED && dstCapacity < srcSize` ->
/// `LZ4F_ERROR_dstMaxSize_tooSmall` = `(size_t)-11` (lz4frame.c:1009-1010).
///
/// With `autoFlush == 0` and `srcSize < blockSize` the row-14 bound collapses
/// to just the frame footer (4/8 bytes), so capacities between the bound and
/// `srcSize-1` isolate the row-15 check specifically.
#[test]
fn err_15_uncompressedUpdate_dstCapacity_below_srcSize() {
    unsafe {
        let (c, r) = apis();
        let mut rng = Rng::new(15);

        for shape in ALL_SHAPES {
            for &n in &[1usize, 2, 64, 1000, 40_000] {
                let src = gen(&mut rng, shape, n);
                let mut prefs = prefs_default();
                prefs.frameInfo.blockMode = LZ4F_BLOCK_INDEPENDENT;
                prefs.autoFlush = 0;

                let mut caps: Vec<usize> = vec![0, 1, 4, 8, 12, n / 2, n - 1, n, n + 8, n + 4096];
                caps.sort();
                caps.dedup();

                let mut cv = Vec::new();
                let mut rv = Vec::new();
                for (api, out) in [(&c, &mut cv), (&r, &mut rv)] {
                    for &cap in &caps {
                        let cctx = new_cctx(api);
                        let mut hdr = vec![0xCDu8; 32];
                        let hr = (api.compressBegin)(
                            cctx,
                            hdr.as_mut_ptr() as *mut c_void,
                            hdr.len(),
                            &prefs,
                        );
                        assert!(!is_err_range(hr), "{}: compressBegin {hr:#x}", api.tag);
                        let mut dst = vec![0xCDu8; cap.max(1)];
                        out.push((api.uncompressedUpdate)(
                            cctx,
                            dst.as_mut_ptr() as *mut c_void,
                            cap,
                            src.as_ptr() as *const c_void,
                            n,
                            ptr::null(),
                        ));
                        (api.freeCCtx)(cctx);
                    }
                }
                let ctx = format!("row15: {shape:?} n={n}");
                same_vec(&ctx, &cv, &rv);
                // Two checks apply, in this order:
                //   row 14: dstCapacity >= compressBound_internal(n, prefs, 0)
                //           which for autoFlush==0 and n < 64 KB is just the
                //           4-byte frame footer;
                //   row 15: dstCapacity >= srcSize.
                let need = n.max(4);
                for (i, &cap) in caps.iter().enumerate() {
                    if cap < need {
                        expect(&format!("{ctx} cap={cap}"), cv[i], err(11));
                    } else {
                        assert!(
                            !is_err_range(cv[i]),
                            "{ctx} cap={cap}: expected success, got {:#x}",
                            cv[i]
                        );
                    }
                }
                // and at least one capacity isolates row 15 specifically
                // (>= the row-14 bound of 4, but < srcSize)
                if n > 4 {
                    let i = caps.iter().position(|&x| x == n - 1).unwrap();
                    expect(&format!("{ctx} row15-only cap={}", n - 1), cv[i], err(11));
                }
            }
        }
    }
}

// ===========================================================================
// ERRORS row 16 — LZ4F_flush with tmpInSize > 0 and cStage != 1
// ===========================================================================

/// Row 16: `tmpInSize > 0 && cctxPtr->cStage != 1` ->
/// `LZ4F_ERROR_compressionState_uninitialized` = `(size_t)-20`
/// (lz4frame.c:1168).
///
/// UNREACHABLE by construction, not for want of an allocator hook: `cStage`
/// only becomes 0 at lz4frame.c:1233, inside `LZ4F_compressEnd`, and
/// `LZ4F_compressEnd` starts by calling `LZ4F_flush` (line 1213), which sets
/// `tmpInSize = 0` (line 1185) before line 1233 can run. Every writer of
/// `tmpInSize` (`LZ4F_compressUpdateImpl`, lz4frame.c:1097) is itself gated on
/// `cStage == 1` (line 1005). Hence `cStage == 0` implies `tmpInSize == 0` and
/// the guarded branch cannot be entered.
///
/// The closest reachable observable behaviour is asserted instead: on a fresh
/// context and on a context whose frame has just ended, `LZ4F_flush` returns
/// `0` — the `tmpInSize == 0` early-out at lz4frame.c:1167 fires *before* the
/// `cStage` test, so the answer is success, NOT `err(20)`. Both libraries must
/// agree on that ordering.
#[test]
fn err_16_flush_state_uninitialized_is_unreachable() {
    unsafe {
        let (c, r) = apis();
        let mut rng = Rng::new(16);
        let src = gen(&mut rng, Shape::TextLike, 5000);

        for &cap in &[0usize, 1, 3, 4, 8, 64, 1 << 16] {
            let mut cv = Vec::new();
            let mut rv = Vec::new();
            for (api, out) in [(&c, &mut cv), (&r, &mut rv)] {
                let cctx = new_cctx(api);
                let mut dst = vec![0xCDu8; (1 << 20).max(cap)];
                // (a) fresh cctx: cStage == 0, tmpInSize == 0
                out.push((api.flush)(cctx, dst.as_mut_ptr() as *mut c_void, cap, ptr::null()));
                // (b) after a full frame: cStage == 0, tmpInSize == 0
                let prefs = prefs_default();
                let h = (api.compressBegin)(
                    cctx,
                    dst.as_mut_ptr() as *mut c_void,
                    dst.len(),
                    &prefs,
                );
                assert!(!is_err_range(h));
                let u = (api.compressUpdate)(
                    cctx,
                    dst.as_mut_ptr() as *mut c_void,
                    dst.len(),
                    src.as_ptr() as *const c_void,
                    src.len(),
                    ptr::null(),
                );
                assert!(!is_err_range(u));
                let e = (api.compressEnd)(
                    cctx,
                    dst.as_mut_ptr() as *mut c_void,
                    dst.len(),
                    ptr::null(),
                );
                assert!(!is_err_range(e));
                out.push((api.flush)(cctx, dst.as_mut_ptr() as *mut c_void, cap, ptr::null()));
                (api.freeCCtx)(cctx);
            }
            let ctx = format!("row16: cap={cap}");
            same_vec(&ctx, &cv, &rv);
            expect(&format!("{ctx} fresh flush"), cv[0], 0);
            expect(&format!("{ctx} post-end flush"), cv[1], 0);
        }
    }
}

// ===========================================================================
// ERRORS row 17 — LZ4F_flush dstCapacity < tmpInSize + BHSize + BFSize
// ===========================================================================

/// Row 17: `dstCapacity < tmpInSize + BHSize(4) + BFSize(4)` ->
/// `LZ4F_ERROR_dstMaxSize_tooSmall` = `(size_t)-11` (lz4frame.c:1169).
/// `autoFlush = 0` plus a sub-block-size update leaves exactly `n` bytes
/// buffered, so the threshold is `n + 8` and can be swept precisely.
#[test]
fn err_17_flush_dstCapacity_below_tmpInSize_plus_8() {
    unsafe {
        let (c, r) = apis();
        let mut rng = Rng::new(17);

        for shape in ALL_SHAPES {
            for &n in &[1usize, 2, 17, 1000, 40_000] {
                let src = gen(&mut rng, shape, n);
                let mut prefs = prefs_default();
                prefs.autoFlush = 0;

                let mut caps: Vec<usize> =
                    vec![0, 1, 4, 7, 8, n, n + 6, n + 7, n + 8, n + 9, n + 4096];
                caps.sort();
                caps.dedup();

                let mut cv = Vec::new();
                let mut rv = Vec::new();
                for (api, out) in [(&c, &mut cv), (&r, &mut rv)] {
                    for &cap in &caps {
                        let cctx = new_cctx(api);
                        let mut scratch = vec![0xCDu8; n + 8192];
                        let h = (api.compressBegin)(
                            cctx,
                            scratch.as_mut_ptr() as *mut c_void,
                            scratch.len(),
                            &prefs,
                        );
                        assert!(!is_err_range(h), "{}: compressBegin {h:#x}", api.tag);
                        let u = (api.compressUpdate)(
                            cctx,
                            scratch.as_mut_ptr() as *mut c_void,
                            scratch.len(),
                            src.as_ptr() as *const c_void,
                            n,
                            ptr::null(),
                        );
                        // no autoFlush + n < 64 KB -> everything is buffered
                        assert_eq!(u, 0, "{}: expected pure buffering, got {u:#x}", api.tag);
                        let mut dst = vec![0xCDu8; cap.max(1)];
                        out.push((api.flush)(
                            cctx,
                            dst.as_mut_ptr() as *mut c_void,
                            cap,
                            ptr::null(),
                        ));
                        (api.freeCCtx)(cctx);
                    }
                }
                let ctx = format!("row17: {shape:?} n={n}");
                same_vec(&ctx, &cv, &rv);
                for (i, &cap) in caps.iter().enumerate() {
                    if cap < n + 8 {
                        expect(&format!("{ctx} cap={cap}"), cv[i], err(11));
                    } else {
                        assert!(
                            !is_err_range(cv[i]),
                            "{ctx} cap={cap}: expected success, got {:#x}",
                            cv[i]
                        );
                    }
                }
            }
        }
    }
}

// ===========================================================================
// ERRORS rows 18 + 19 — LZ4F_compressEnd dstCapacity too small
// ===========================================================================

/// Row 18: after the internal flush, `dstCapacity < 4` (no room for the
/// endMark) -> `LZ4F_ERROR_dstMaxSize_tooSmall` = `(size_t)-11`
/// (lz4frame.c:1221).
#[test]
fn err_18_compressEnd_dstCapacity_below_endMark() {
    unsafe {
        let (c, r) = apis();
        let mut rng = Rng::new(18);

        for &n in &[0usize, 1, 1000] {
            let src = gen(&mut rng, Shape::TextLike, n);
            // no content checksum: the only requirement is the 4-byte endMark
            let mut prefs = prefs_default();
            prefs.frameInfo.contentChecksumFlag = LZ4F_NO_CONTENT_CHECKSUM;
            prefs.autoFlush = 1; // nothing left buffered -> flushSize == 0

            let caps: Vec<usize> = vec![0, 1, 2, 3, 4, 5, 8, 64];
            let mut cv = Vec::new();
            let mut rv = Vec::new();
            let mut cbuf: Vec<Vec<u8>> = Vec::new();
            let mut rbuf: Vec<Vec<u8>> = Vec::new();
            for (api, out, bufs) in [(&c, &mut cv, &mut cbuf), (&r, &mut rv, &mut rbuf)] {
                for &cap in &caps {
                    let cctx = new_cctx(api);
                    let mut scratch = vec![0xCDu8; n + 8192];
                    let h = (api.compressBegin)(
                        cctx,
                        scratch.as_mut_ptr() as *mut c_void,
                        scratch.len(),
                        &prefs,
                    );
                    assert!(!is_err_range(h));
                    let u = (api.compressUpdate)(
                        cctx,
                        scratch.as_mut_ptr() as *mut c_void,
                        scratch.len(),
                        src.as_ptr() as *const c_void,
                        n,
                        ptr::null(),
                    );
                    assert!(!is_err_range(u));
                    let mut dst = vec![0xCDu8; 16];
                    out.push((api.compressEnd)(
                        cctx,
                        dst.as_mut_ptr() as *mut c_void,
                        cap,
                        ptr::null(),
                    ));
                    bufs.push(dst);
                    (api.freeCCtx)(cctx);
                }
            }
            let ctx = format!("row18: n={n}");
            same_vec(&ctx, &cv, &rv);
            for (i, &cap) in caps.iter().enumerate() {
                same_full_buffers(&format!("{ctx} cap={cap} dst"), &cbuf[i], &rbuf[i]);
                if cap < 4 {
                    expect(&format!("{ctx} cap={cap}"), cv[i], err(11));
                } else {
                    expect(&format!("{ctx} cap={cap}"), cv[i], 4);
                }
            }
        }
    }
}

/// Row 19: `contentChecksumFlag == LZ4F_contentChecksumEnabled` and the
/// remaining `dstCapacity < 8` -> `LZ4F_ERROR_dstMaxSize_tooSmall` =
/// `(size_t)-11` (lz4frame.c:1227).
///
/// Note the C writes the 4-byte endMark into `dst` *before* discovering there
/// is no room for the checksum (lines 1222-1227), so the destination buffer is
/// modified even on failure; that side effect is compared byte-for-byte too.
#[test]
fn err_19_compressEnd_dstCapacity_below_contentChecksum() {
    unsafe {
        let (c, r) = apis();
        let mut rng = Rng::new(19);

        for &n in &[0usize, 1, 1000] {
            let src = gen(&mut rng, Shape::TextLike, n);
            let mut prefs = prefs_default();
            prefs.frameInfo.contentChecksumFlag = LZ4F_CONTENT_CHECKSUM_ENABLED;
            prefs.autoFlush = 1;

            let caps: Vec<usize> = vec![0, 1, 3, 4, 5, 6, 7, 8, 9, 64];
            let mut cv = Vec::new();
            let mut rv = Vec::new();
            let mut cbuf: Vec<Vec<u8>> = Vec::new();
            let mut rbuf: Vec<Vec<u8>> = Vec::new();
            for (api, out, bufs) in [(&c, &mut cv, &mut cbuf), (&r, &mut rv, &mut rbuf)] {
                for &cap in &caps {
                    let cctx = new_cctx(api);
                    let mut scratch = vec![0xCDu8; n + 8192];
                    let h = (api.compressBegin)(
                        cctx,
                        scratch.as_mut_ptr() as *mut c_void,
                        scratch.len(),
                        &prefs,
                    );
                    assert!(!is_err_range(h));
                    let u = (api.compressUpdate)(
                        cctx,
                        scratch.as_mut_ptr() as *mut c_void,
                        scratch.len(),
                        src.as_ptr() as *const c_void,
                        n,
                        ptr::null(),
                    );
                    assert!(!is_err_range(u));
                    let mut dst = vec![0xCDu8; 16];
                    out.push((api.compressEnd)(
                        cctx,
                        dst.as_mut_ptr() as *mut c_void,
                        cap,
                        ptr::null(),
                    ));
                    bufs.push(dst);
                    (api.freeCCtx)(cctx);
                }
            }
            let ctx = format!("row19: n={n}");
            same_vec(&ctx, &cv, &rv);
            for (i, &cap) in caps.iter().enumerate() {
                same_full_buffers(&format!("{ctx} cap={cap} dst"), &cbuf[i], &rbuf[i]);
                if cap < 8 {
                    expect(&format!("{ctx} cap={cap}"), cv[i], err(11));
                } else {
                    expect(&format!("{ctx} cap={cap}"), cv[i], 8);
                }
                // the endMark side effect: for 4 <= cap < 8 the first four
                // bytes have been zeroed even though the call failed
                if (4..8).contains(&cap) {
                    assert_eq!(
                        &cbuf[i][..4],
                        &[0u8, 0, 0, 0],
                        "{ctx} cap={cap}: C should have written the endMark before failing"
                    );
                }
            }
        }
    }
}

// ===========================================================================
// ERRORS row 20 — LZ4F_compressEnd declared contentSize mismatch
// ===========================================================================

/// Row 20: `prefs.frameInfo.contentSize != cctxPtr->totalInSize` ->
/// `LZ4F_ERROR_frameSize_wrong` = `(size_t)-14` (lz4frame.c:1235-1237).
/// Property-style over random declared/actual pairs.
#[test]
fn err_20_compressEnd_declared_contentSize_wrong() {
    unsafe {
        let (c, r) = apis();
        let mut rng = Rng::new(20);

        let mut cases: Vec<(u64, usize)> = vec![
            (1, 0),
            (10, 9),
            (10, 11),
            (u64::MAX, 100),
            (1000, 1000), // must SUCCEED
            (65536, 65536),
        ];
        for _ in 0..40 {
            let actual = rng.range(0, 70_000);
            let declared = if rng.below(4) == 0 {
                actual as u64
            } else {
                rng.next_u64() % 200_000
            };
            cases.push((declared, actual));
        }

        for &(declared, actual) in &cases {
            let src = gen(&mut rng, Shape::TextLike, actual);
            let mut prefs = prefs_default();
            prefs.frameInfo.contentSize = declared;
            prefs.autoFlush = 1;

            let mut cv = Vec::new();
            let mut rv = Vec::new();
            for (api, out) in [(&c, &mut cv), (&r, &mut rv)] {
                let cctx = new_cctx(api);
                let mut dst = vec![0xCDu8; actual + (1 << 17)];
                out.push((api.compressBegin)(
                    cctx,
                    dst.as_mut_ptr() as *mut c_void,
                    dst.len(),
                    &prefs,
                ));
                out.push((api.compressUpdate)(
                    cctx,
                    dst.as_mut_ptr() as *mut c_void,
                    dst.len(),
                    src.as_ptr() as *const c_void,
                    actual,
                    ptr::null(),
                ));
                out.push((api.compressEnd)(
                    cctx,
                    dst.as_mut_ptr() as *mut c_void,
                    dst.len(),
                    ptr::null(),
                ));
                (api.freeCCtx)(cctx);
            }
            let ctx = format!("row20: declared={declared} actual={actual}");
            same_vec(&ctx, &cv, &rv);
            // declared == 0 means "unknown" and disables the check entirely
            if declared != 0 && declared != actual as u64 {
                expect(&format!("{ctx} compressEnd"), cv[2], err(14));
            } else {
                assert!(
                    !is_err_range(cv[2]),
                    "{ctx}: compressEnd should succeed, got {:#x}",
                    cv[2]
                );
            }
        }
    }
}

// ===========================================================================
// ERRORS row 21 — LZ4F_createDecompressionContext_advanced allocation failure
// ===========================================================================

/// Row 21: `LZ4F_calloc(sizeof(LZ4F_dctx))` fails -> `NULL`
/// (lz4frame.c:1286-1287). Forced for real, with the failure index swept to
/// prove there is exactly one allocation in this constructor.
#[test]
fn err_21_createDecompressionContext_advanced_allocation_fails() {
    unsafe {
        let (c, r) = apis();
        for fail_at in [-1i64, 1, 2, 3, 4, 5, 6, 7, 8] {
            for &with_calloc in &[true, false] {
                for &version in &[0u32, 99, LZ4F_VERSION, 101, 0xFFFF_FFFF] {
                    let mut obs: Vec<(bool, u64, usize)> = Vec::new();
                    for api in [&c, &r] {
                        let mut hook = Hook::new(fail_at);
                        let cm = cmem_of(&mut hook, with_calloc);
                        let dctx = (api.createDCtxAdv)(cm, version);
                        let was_null = dctx.is_null();
                        let mut freed = 0usize;
                        if !dctx.is_null() {
                            freed = (api.freeDCtx)(dctx);
                        }
                        assert_eq!(hook.live, 0, "{}: leaked", api.tag);
                        obs.push((was_null, hook.calls(), freed));
                    }
                    let ctx =
                        format!("row21: fail_at={fail_at} calloc={with_calloc} version={version}");
                    assert_eq!(obs[0], obs[1], "{ctx}: differ {obs:?}");
                    assert_eq!(obs[0].1, 1, "{ctx}: expected exactly 1 allocation");
                    assert_eq!(
                        obs[0].0,
                        fail_at < 0 || fail_at == 1,
                        "{ctx}: unexpected NULL-ness"
                    );
                    if !obs[0].0 {
                        // a freshly created dctx is at dstage_getFrameHeader(0)
                        assert_eq!(obs[0].2, 0, "{ctx}: freeDecompressionContext != 0");
                    }
                }
            }
        }
    }
}

// ===========================================================================
// ERRORS row 22 — LZ4F_createDecompressionContext(NULL, ...)
// ===========================================================================

/// Row 22: `LZ4F_decompressionContextPtr == NULL` ->
/// `LZ4F_ERROR_parameter_null` = `(size_t)-21` (lz4frame.c:1304).
#[test]
fn err_22_createDecompressionContext_null_pointer() {
    unsafe {
        let (c, r) = apis();
        for &version in &[0u32, 1, 99, LZ4F_VERSION, 101, 0xFFFF_FFFF] {
            let a = (c.createDCtx)(ptr::null_mut(), version);
            let b = (r.createDCtx)(ptr::null_mut(), version);
            let ctx = format!("row22: createDecompressionContext(NULL, {version})");
            same(&ctx, a, b);
            expect(&ctx, a, err(21));
        }
    }
}

// ===========================================================================
// ERRORS row 23 — LZ4F_createDecompressionContext allocation_failed
// ===========================================================================

/// Row 23: the inner `LZ4F_createDecompressionContext_advanced` returning NULL
/// is reported as `LZ4F_ERROR_allocation_failed` = `(size_t)-9`
/// (lz4frame.c:1307-1309).
///
/// NOT FORCEABLE, for exactly the same reason as row 8:
/// `LZ4F_createDecompressionContext` hard-codes `LZ4F_defaultCMem`, so the
/// `calloc` is libc's and there is no hook. Asserted instead: the branch
/// condition (advanced ctor returning NULL under an injected failure) is
/// identical in both libraries, and the normal path returns
/// `LZ4F_OK_NoError` + a non-NULL dctx in both.
#[test]
fn err_23_createDecompressionContext_allocation_failed_is_unforceable() {
    unsafe {
        let (c, r) = apis();

        for api in [&c, &r] {
            let mut hook = Hook::new(-1);
            let cm = cmem_of(&mut hook, true);
            let p = (api.createDCtxAdv)(cm, LZ4F_VERSION);
            assert!(
                p.is_null(),
                "{}: row23 precondition: advanced ctor must return NULL when calloc fails",
                api.tag
            );
            assert_eq!(hook.calls(), 1, "{}: expected exactly 1 allocation", api.tag);
        }

        for &version in &[0u32, LZ4F_VERSION, 0xFFFF_FFFF] {
            let mut pc: *mut c_void = ptr::null_mut();
            let mut pr: *mut c_void = ptr::null_mut();
            let a = (c.createDCtx)(&mut pc, version);
            let b = (r.createDCtx)(&mut pr, version);
            let ctx = format!("row23: createDecompressionContext(&p, {version})");
            same(&ctx, a, b);
            expect(&ctx, a, 0);
            assert!(!pc.is_null() && !pr.is_null(), "{ctx}: NULL dctx despite success");
            same(&ctx, (c.freeDCtx)(pc), (r.freeDCtx)(pr));
        }
    }
}

// ===========================================================================
// Generic FFI boundary: NULL pointers into every entry point
// ===========================================================================

#[test]
fn ffi_null_pointers_into_every_compression_entry_point() {
    unsafe {
        let (c, r) = apis();

        // --- context constructors with a NULL out-pointer (rows 7 / 22) ---
        same(
            "ffi: createCompressionContext(NULL, LZ4F_VERSION)",
            (c.createCCtx)(ptr::null_mut(), LZ4F_VERSION),
            (r.createCCtx)(ptr::null_mut(), LZ4F_VERSION),
        );
        expect(
            "ffi: createCompressionContext(NULL, LZ4F_VERSION)",
            (c.createCCtx)(ptr::null_mut(), LZ4F_VERSION),
            err(21),
        );
        same(
            "ffi: createDecompressionContext(NULL, LZ4F_VERSION)",
            (c.createDCtx)(ptr::null_mut(), LZ4F_VERSION),
            (r.createDCtx)(ptr::null_mut(), LZ4F_VERSION),
        );
        expect(
            "ffi: createDecompressionContext(NULL, LZ4F_VERSION)",
            (c.createDCtx)(ptr::null_mut(), LZ4F_VERSION),
            err(21),
        );

        // --- free-on-NULL is explicitly supported ---
        let a = (c.freeCCtx)(ptr::null_mut());
        let b = (r.freeCCtx)(ptr::null_mut());
        same("ffi: freeCompressionContext(NULL)", a, b);
        expect("ffi: freeCompressionContext(NULL)", a, 0);

        let a = (c.freeDCtx)(ptr::null_mut());
        let b = (r.freeDCtx)(ptr::null_mut());
        same("ffi: freeDecompressionContext(NULL)", a, b);
        expect("ffi: freeDecompressionContext(NULL)", a, 0);

        // returns void; must simply not crash in either library
        (c.freeCDict)(ptr::null_mut());
        (r.freeCDict)(ptr::null_mut());

        // --- bounds with a NULL preferences pointer ---
        for &n in &[0usize, 1, 100, 65536, 1 << 20] {
            let a = (c.compressBound)(n, ptr::null());
            let b = (r.compressBound)(n, ptr::null());
            same(&format!("ffi: compressBound({n}, NULL)"), a, b);
            let a = (c.compressFrameBound)(n, ptr::null());
            let b = (r.compressFrameBound)(n, ptr::null());
            same(&format!("ffi: compressFrameBound({n}, NULL)"), a, b);
        }
        // the two explicitly-required probes
        let a = (c.compressBound)(0, ptr::null());
        let b = (r.compressBound)(0, ptr::null());
        same("ffi: compressBound(0, NULL)", a, b);
        assert!(a >= 4, "ffi: compressBound(0, NULL) = {a} looks wrong");
        let a = (c.compressFrameBound)(0, ptr::null());
        let b = (r.compressFrameBound)(0, ptr::null());
        same("ffi: compressFrameBound(0, NULL)", a, b);
        assert!(a >= 19 + 4, "ffi: compressFrameBound(0, NULL) = {a} looks wrong");

        // --- LZ4F_createCDict(NULL, 0) ---
        let mut nulls = Vec::new();
        for api in [&c, &r] {
            let cd = (api.createCDict)(ptr::null(), 0);
            nulls.push(cd.is_null());
            if !cd.is_null() {
                (api.freeCDict)(cd);
            }
        }
        assert_eq!(
            nulls[0], nulls[1],
            "ffi: createCDict(NULL, 0) NULL-ness differs (C={} Rust={})",
            nulls[0], nulls[1]
        );
        // the C copies zero bytes and succeeds; whatever it does, Rust matches
        assert!(!nulls[0], "ffi: createCDict(NULL, 0) unexpectedly failed in C");

        // --- LZ4F_compressFrame with src == NULL and srcSize == 0 ---
        for &cc in &[LZ4F_NO_CONTENT_CHECKSUM, LZ4F_CONTENT_CHECKSUM_ENABLED] {
            for &bc in &[LZ4F_NO_BLOCK_CHECKSUM, LZ4F_BLOCK_CHECKSUM_ENABLED] {
                for use_prefs in [false, true] {
                    let mut prefs = prefs_default();
                    prefs.frameInfo.contentChecksumFlag = cc;
                    prefs.frameInfo.blockChecksumFlag = bc;
                    let pp = if use_prefs {
                        &prefs as *const LZ4F_preferences_t
                    } else {
                        ptr::null()
                    };
                    let cap = (c.compressFrameBound)(0, pp);
                    same(
                        "ffi: compressFrameBound(0, prefs)",
                        cap,
                        (r.compressFrameBound)(0, pp),
                    );
                    let mut cd = vec![0xCDu8; cap + 8];
                    let mut rd = vec![0xCDu8; cap + 8];
                    let a = (c.compressFrame)(
                        cd.as_mut_ptr() as *mut c_void,
                        cap,
                        ptr::null(),
                        0,
                        pp,
                    );
                    let b = (r.compressFrame)(
                        rd.as_mut_ptr() as *mut c_void,
                        cap,
                        ptr::null(),
                        0,
                        pp,
                    );
                    let ctx =
                        format!("ffi: compressFrame(src=NULL, srcSize=0, cc={cc} bc={bc} prefs={use_prefs})");
                    same(&ctx, a, b);
                    assert!(!is_err_range(a), "{ctx}: unexpected error {a:#x}");
                    same_full_buffers(&ctx, &cd, &rd);
                }
            }
        }

        // --- NULL dstBuffer ---
        // Every writer validates `dstCapacity` BEFORE touching `dstBuffer`
        // (lz4frame.c:456, :700, :1005-1010, :1167-1169, :1221), so a NULL
        // destination with a zero capacity is observable, not UB.
        // (A NULL `cctx` is NOT covered: the C dereferences it unconditionally
        // at lz4frame.c:702, so that is plain UB in both libraries.)
        {
            let mut cv = Vec::new();
            let mut rv = Vec::new();
            for (api, out) in [(&c, &mut cv), (&r, &mut rv)] {
                out.push((api.compressFrame)(ptr::null_mut(), 0, ptr::null(), 0, ptr::null()));
                let cctx = new_cctx(api);
                out.push((api.compressBegin)(cctx, ptr::null_mut(), 0, ptr::null()));
                out.push((api.compressBegin_usingDict)(
                    cctx,
                    ptr::null_mut(),
                    0,
                    ptr::null(),
                    0,
                    ptr::null(),
                ));
                out.push((api.compressBegin_usingCDict)(
                    cctx,
                    ptr::null_mut(),
                    0,
                    ptr::null(),
                    ptr::null(),
                ));
                out.push((api.compressFrame_usingCDict)(
                    cctx,
                    ptr::null_mut(),
                    0,
                    ptr::null(),
                    0,
                    ptr::null(),
                    ptr::null(),
                ));
                // uninitialized cctx -> the cStage check fires first
                out.push((api.compressUpdate)(
                    cctx,
                    ptr::null_mut(),
                    0,
                    ptr::null(),
                    0,
                    ptr::null(),
                ));
                out.push((api.uncompressedUpdate)(
                    cctx,
                    ptr::null_mut(),
                    0,
                    ptr::null(),
                    0,
                    ptr::null(),
                ));
                // nothing buffered -> flush returns 0 without touching dst
                out.push((api.flush)(cctx, ptr::null_mut(), 0, ptr::null()));
                out.push((api.compressEnd)(cctx, ptr::null_mut(), 0, ptr::null()));
                (api.freeCCtx)(cctx);
            }
            same_vec("ffi: NULL dstBuffer with dstCapacity 0", &cv, &rv);
            expect("ffi: compressFrame(NULL,0)", cv[0], err(11));
            expect("ffi: compressBegin(NULL,0)", cv[1], err(11));
            expect("ffi: compressBegin_usingDict(NULL,0)", cv[2], err(11));
            expect("ffi: compressBegin_usingCDict(NULL,0)", cv[3], err(11));
            expect("ffi: compressFrame_usingCDict(NULL,0)", cv[4], err(11));
            expect("ffi: compressUpdate(NULL,0) uninit", cv[5], err(20));
            expect("ffi: uncompressedUpdate(NULL,0) uninit", cv[6], err(20));
            expect("ffi: flush(NULL,0) empty", cv[7], 0);
            expect("ffi: compressEnd(NULL,0)", cv[8], err(11));
        }

        // --- NULL srcBuffer with srcSize 0 into the streaming updates ---
        {
            for &cc in &[LZ4F_NO_CONTENT_CHECKSUM, LZ4F_CONTENT_CHECKSUM_ENABLED] {
                let mut prefs = prefs_default();
                prefs.frameInfo.contentChecksumFlag = cc;
                prefs.frameInfo.blockMode = LZ4F_BLOCK_INDEPENDENT;
                let mut cv = Vec::new();
                let mut rv = Vec::new();
                let mut cb: Vec<u8> = Vec::new();
                let mut rb: Vec<u8> = Vec::new();
                for (api, out, buf) in [(&c, &mut cv, &mut cb), (&r, &mut rv, &mut rb)] {
                    let cctx = new_cctx(api);
                    let mut dst = vec![0xCDu8; 1 << 16];
                    let mut off = 0usize;
                    let h = (api.compressBegin)(
                        cctx,
                        dst.as_mut_ptr() as *mut c_void,
                        dst.len(),
                        &prefs,
                    );
                    out.push(h);
                    off += h;
                    for _ in 0..3 {
                        let u = (api.compressUpdate)(
                            cctx,
                            dst[off..].as_mut_ptr() as *mut c_void,
                            dst.len() - off,
                            ptr::null(),
                            0,
                            ptr::null(),
                        );
                        out.push(u);
                        off += u;
                        let u2 = (api.uncompressedUpdate)(
                            cctx,
                            dst[off..].as_mut_ptr() as *mut c_void,
                            dst.len() - off,
                            ptr::null(),
                            0,
                            ptr::null(),
                        );
                        out.push(u2);
                        off += u2;
                    }
                    let e = (api.compressEnd)(
                        cctx,
                        dst[off..].as_mut_ptr() as *mut c_void,
                        dst.len() - off,
                        ptr::null(),
                    );
                    out.push(e);
                    off += e;
                    (api.freeCCtx)(cctx);
                    dst.truncate(off);
                    *buf = dst;
                }
                let ctx = format!("ffi: NULL srcBuffer srcSize=0 cc={cc}");
                same_vec(&ctx, &cv, &rv);
                same_full_buffers(&ctx, &cb, &rb);
                for &v in &cv {
                    assert!(!is_err_range(v), "{ctx}: unexpected error {v:#x}");
                }
            }
        }

        // --- NULL preferences / options into the streaming pipeline ---
        let mut rng = Rng::new(0x4E554C4C);
        let src = gen(&mut rng, Shape::TextLike, 12_345);
        let mut cv = Vec::new();
        let mut rv = Vec::new();
        let mut cbuf: Vec<u8> = Vec::new();
        let mut rbuf: Vec<u8> = Vec::new();
        for (api, out, buf) in [(&c, &mut cv, &mut cbuf), (&r, &mut rv, &mut rbuf)] {
            let cctx = new_cctx(api);
            let mut dst = vec![0xCDu8; 1 << 19];
            let mut off = 0usize;
            let h = (api.compressBegin)(
                cctx,
                dst.as_mut_ptr() as *mut c_void,
                dst.len(),
                ptr::null(),
            );
            out.push(h);
            off += h;
            let u = (api.compressUpdate)(
                cctx,
                dst[off..].as_mut_ptr() as *mut c_void,
                dst.len() - off,
                src.as_ptr() as *const c_void,
                src.len(),
                ptr::null(),
            );
            out.push(u);
            off += u;
            let f = (api.flush)(
                cctx,
                dst[off..].as_mut_ptr() as *mut c_void,
                dst.len() - off,
                ptr::null(),
            );
            out.push(f);
            off += f;
            let e = (api.compressEnd)(
                cctx,
                dst[off..].as_mut_ptr() as *mut c_void,
                dst.len() - off,
                ptr::null(),
            );
            out.push(e);
            off += e;
            (api.freeCCtx)(cctx);
            dst.truncate(off);
            *buf = dst;
        }
        same_vec("ffi: NULL prefs/options pipeline", &cv, &rv);
        same_full_buffers("ffi: NULL prefs/options frame bytes", &cbuf, &rbuf);
        for &v in &cv {
            assert!(!is_err_range(v), "ffi: NULL prefs/options pipeline failed {v:#x}");
        }
    }
}

// ===========================================================================
// Generic FFI boundary: zero and tiny dstCapacity on every producer
// ===========================================================================

/// `dstCapacity` 0 and 1 on every function that writes to `dstBuffer`, plus
/// `srcSize == 0` everywhere it is accepted. The exact `err(11)` /
/// `err(20)` sentinel is asserted for each.
#[test]
fn ffi_zero_and_one_byte_dstCapacity_and_zero_srcSize() {
    unsafe {
        let (c, r) = apis();
        let mut rng = Rng::new(0x0D57);
        let src = gen(&mut rng, Shape::TextLike, 3000);

        for &cap in &[0usize, 1] {
            // --- LZ4F_compressFrame: row 2 check -> err(11) ---
            for &n in &[0usize, 1, 3000] {
                let mut cd = vec![0xCDu8; 8];
                let mut rd = vec![0xCDu8; 8];
                let a = (c.compressFrame)(
                    cd.as_mut_ptr() as *mut c_void,
                    cap,
                    src.as_ptr() as *const c_void,
                    n,
                    ptr::null(),
                );
                let b = (r.compressFrame)(
                    rd.as_mut_ptr() as *mut c_void,
                    cap,
                    src.as_ptr() as *const c_void,
                    n,
                    ptr::null(),
                );
                let ctx = format!("ffi: compressFrame(cap={cap}, n={n})");
                same(&ctx, a, b);
                expect(&ctx, a, err(11));
                same_full_buffers(&ctx, &cd, &rd);
            }

            // --- the streaming entry points ---
            let mut cv = Vec::new();
            let mut rv = Vec::new();
            for (api, out) in [(&c, &mut cv), (&r, &mut rv)] {
                let mut tiny = vec![0xCDu8; 8];
                let mut scratch = vec![0xCDu8; 1 << 17];

                // compressBegin with a tiny dstCapacity -> err(11) (row 9)
                let cctx = new_cctx(api);
                out.push((api.compressBegin)(
                    cctx,
                    tiny.as_mut_ptr() as *mut c_void,
                    cap,
                    ptr::null(),
                ));
                // ... the context is still uninitialized, so update -> err(20)
                out.push((api.compressUpdate)(
                    cctx,
                    tiny.as_mut_ptr() as *mut c_void,
                    cap,
                    src.as_ptr() as *const c_void,
                    src.len(),
                    ptr::null(),
                ));
                out.push((api.uncompressedUpdate)(
                    cctx,
                    tiny.as_mut_ptr() as *mut c_void,
                    cap,
                    src.as_ptr() as *const c_void,
                    src.len(),
                    ptr::null(),
                ));
                // ... flush with nothing buffered -> 0, regardless of capacity
                out.push((api.flush)(
                    cctx,
                    tiny.as_mut_ptr() as *mut c_void,
                    cap,
                    ptr::null(),
                ));
                // ... compressEnd needs 4 bytes -> err(11) (row 18)
                out.push((api.compressEnd)(
                    cctx,
                    tiny.as_mut_ptr() as *mut c_void,
                    cap,
                    ptr::null(),
                ));

                // now a properly begun frame with data buffered
                let mut prefs = prefs_default();
                prefs.autoFlush = 0;
                let h = (api.compressBegin)(
                    cctx,
                    scratch.as_mut_ptr() as *mut c_void,
                    scratch.len(),
                    &prefs,
                );
                out.push(h);
                out.push((api.compressUpdate)(
                    cctx,
                    scratch.as_mut_ptr() as *mut c_void,
                    scratch.len(),
                    src.as_ptr() as *const c_void,
                    src.len(),
                    ptr::null(),
                ));
                // compressUpdate / uncompressedUpdate / flush / compressEnd
                // now all fail on capacity
                out.push((api.compressUpdate)(
                    cctx,
                    tiny.as_mut_ptr() as *mut c_void,
                    cap,
                    src.as_ptr() as *const c_void,
                    src.len(),
                    ptr::null(),
                ));
                out.push((api.uncompressedUpdate)(
                    cctx,
                    tiny.as_mut_ptr() as *mut c_void,
                    cap,
                    src.as_ptr() as *const c_void,
                    src.len(),
                    ptr::null(),
                ));
                out.push((api.flush)(
                    cctx,
                    tiny.as_mut_ptr() as *mut c_void,
                    cap,
                    ptr::null(),
                ));
                out.push((api.compressEnd)(
                    cctx,
                    tiny.as_mut_ptr() as *mut c_void,
                    cap,
                    ptr::null(),
                ));
                // srcSize == 0 is always accepted (nothing to buffer)
                out.push((api.compressUpdate)(
                    cctx,
                    scratch.as_mut_ptr() as *mut c_void,
                    scratch.len(),
                    src.as_ptr() as *const c_void,
                    0,
                    ptr::null(),
                ));
                out.push((api.uncompressedUpdate)(
                    cctx,
                    scratch.as_mut_ptr() as *mut c_void,
                    scratch.len(),
                    src.as_ptr() as *const c_void,
                    0,
                    ptr::null(),
                ));
                (api.freeCCtx)(cctx);
            }
            let ctx = format!("ffi: tiny dstCapacity={cap}");
            same_vec(&ctx, &cv, &rv);
            expect(&format!("{ctx} compressBegin"), cv[0], err(11));
            expect(&format!("{ctx} compressUpdate uninit"), cv[1], err(20));
            expect(&format!("{ctx} uncompressedUpdate uninit"), cv[2], err(20));
            expect(&format!("{ctx} flush empty"), cv[3], 0);
            expect(&format!("{ctx} compressEnd uninit"), cv[4], err(11));
            assert!(!is_err_range(cv[5]), "{ctx}: compressBegin failed {:#x}", cv[5]);
            assert!(!is_err_range(cv[6]), "{ctx}: compressUpdate failed {:#x}", cv[6]);
            expect(&format!("{ctx} compressUpdate tiny"), cv[7], err(11));
            expect(&format!("{ctx} uncompressedUpdate tiny"), cv[8], err(11));
            expect(&format!("{ctx} flush tiny"), cv[9], err(11));
            expect(&format!("{ctx} compressEnd tiny"), cv[10], err(11));
            assert!(!is_err_range(cv[11]), "{ctx}: compressUpdate(0) {:#x}", cv[11]);
            assert!(!is_err_range(cv[12]), "{ctx}: uncompressedUpdate(0) {:#x}", cv[12]);
        }
    }
}

// ===========================================================================
// Generic FFI boundary: out-of-range enum values
// ===========================================================================

/// Drive every out-of-range enum value through `LZ4F_compressFrame` (the
/// one-shot path, which additionally runs them through `LZ4F_optimalBSID`) and
/// compare the return value *and* the produced frame bytes.
#[test]
fn ffi_out_of_range_enum_values_through_compressFrame() {
    unsafe {
        let (c, r) = apis();
        let mut rng = Rng::new(0xE0E0);

        // one axis at a time, so a divergence points straight at a field
        let mut cases: Vec<(String, LZ4F_preferences_t)> = Vec::new();
        for &v in &BSID_SWEEP {
            let mut p = prefs_default();
            p.frameInfo.blockSizeID = v;
            cases.push((format!("blockSizeID={v:#x}"), p));
        }
        for &v in &BLOCKMODE_SWEEP {
            let mut p = prefs_default();
            p.frameInfo.blockMode = v;
            cases.push((format!("blockMode={v:#x}"), p));
        }
        for &v in &CCFLAG_SWEEP {
            let mut p = prefs_default();
            p.frameInfo.contentChecksumFlag = v;
            cases.push((format!("contentChecksumFlag={v:#x}"), p));
        }
        for &v in &BCFLAG_SWEEP {
            let mut p = prefs_default();
            p.frameInfo.blockChecksumFlag = v;
            cases.push((format!("blockChecksumFlag={v:#x}"), p));
        }
        for &v in &FRAMETYPE_SWEEP {
            let mut p = prefs_default();
            p.frameInfo.frameType = v;
            cases.push((format!("frameType={v:#x}"), p));
        }

        for (name, prefs) in &cases {
            for &n in &[0usize, 1, 100, 65536, 65537, 300_000] {
                let src = gen(&mut rng, Shape::TextLike, n);

                let cb = (c.compressFrameBound)(n, prefs);
                let rb = (r.compressFrameBound)(n, prefs);
                same(&format!("enum {name} n={n}: compressFrameBound"), cb, rb);

                // A wildly-out-of-range checksum flag multiplies the footer
                // size and makes the bound astronomically large; then only the
                // err(11) path is reachable, which is exactly what we assert.
                let cap = if cb > 8 << 20 { 4 << 20 } else { cb };
                let mut cd = vec![0xCDu8; cap + 16];
                let mut rd = vec![0xCDu8; cap + 16];
                let a = (c.compressFrame)(
                    cd.as_mut_ptr() as *mut c_void,
                    cap,
                    src.as_ptr() as *const c_void,
                    n,
                    prefs,
                );
                let b = (r.compressFrame)(
                    rd.as_mut_ptr() as *mut c_void,
                    cap,
                    src.as_ptr() as *const c_void,
                    n,
                    prefs,
                );
                let ctx = format!("enum {name} n={n} cap={cap}");
                same(&ctx, a, b);
                same_full_buffers(&ctx, &cd, &rd);
                if cb > 8 << 20 {
                    expect(&ctx, a, err(11));
                }

                // and one byte short of the bound is always err(11)
                if cap > 0 {
                    let mut cd = vec![0xCDu8; cap + 16];
                    let mut rd = vec![0xCDu8; cap + 16];
                    let a = (c.compressFrame)(
                        cd.as_mut_ptr() as *mut c_void,
                        cap - 1,
                        src.as_ptr() as *const c_void,
                        n,
                        prefs,
                    );
                    let b = (r.compressFrame)(
                        rd.as_mut_ptr() as *mut c_void,
                        cap - 1,
                        src.as_ptr() as *const c_void,
                        n,
                        prefs,
                    );
                    let ctx = format!("enum {name} n={n} cap=bound-1");
                    same(&ctx, a, b);
                    same_full_buffers(&ctx, &cd, &rd);
                }
            }
        }
    }
}

/// The same enum sweep through the low-level
/// `LZ4F_compressBegin` / `LZ4F_compressUpdate` / `LZ4F_flush` /
/// `LZ4F_compressEnd` pipeline, which does NOT normalise `blockSizeID` through
/// `LZ4F_optimalBSID` — so out-of-range ids reach `LZ4F_getBlockSize` and turn
/// `maxBlockSize` into the `(size_t)-2` sentinel.
#[test]
fn ffi_out_of_range_enum_values_through_lowlevel_pipeline() {
    unsafe {
        let (c, r) = apis();
        let mut rng = Rng::new(0xE1E1);
        let src = gen(&mut rng, Shape::TextLike, 4000);

        let mut cases: Vec<(String, LZ4F_preferences_t)> = Vec::new();
        for &af in &[0u32, 1] {
            for &v in &BSID_SWEEP {
                for &bm in &[LZ4F_BLOCK_LINKED, LZ4F_BLOCK_INDEPENDENT] {
                    let mut p = prefs_default();
                    p.frameInfo.blockSizeID = v;
                    p.frameInfo.blockMode = bm;
                    p.autoFlush = af;
                    cases.push((format!("blockSizeID={v:#x} bm={bm} af={af}"), p));
                }
            }
            for &v in &BLOCKMODE_SWEEP {
                let mut p = prefs_default();
                p.frameInfo.blockMode = v;
                p.autoFlush = af;
                cases.push((format!("blockMode={v:#x} af={af}"), p));
            }
            for &v in &CCFLAG_SWEEP {
                let mut p = prefs_default();
                p.frameInfo.contentChecksumFlag = v;
                p.autoFlush = af;
                cases.push((format!("contentChecksumFlag={v:#x} af={af}"), p));
            }
            for &v in &BCFLAG_SWEEP {
                let mut p = prefs_default();
                p.frameInfo.blockChecksumFlag = v;
                p.autoFlush = af;
                cases.push((format!("blockChecksumFlag={v:#x} af={af}"), p));
            }
            for &v in &FRAMETYPE_SWEEP {
                let mut p = prefs_default();
                p.frameInfo.frameType = v;
                p.autoFlush = af;
                cases.push((format!("frameType={v:#x} af={af}"), p));
            }
        }

        for (name, prefs) in &cases {
            let (cv, cbytes) = drive_pipeline(&c, prefs, &src);
            let (rv, rbytes) = drive_pipeline(&r, prefs, &src);
            let ctx = format!("lowlevel enum {name}");
            same_vec(&ctx, &cv, &rv);
            same_full_buffers(&ctx, &cbytes, &rbytes);
        }
    }
}

/// begin / update / flush / end, stopping at the first error, returning every
/// return value and the whole destination buffer (padding included).
///
/// The loop also stops when a call reports having written MORE than the
/// capacity it was given. That is not a divergence but a documented hazard of
/// out-of-contract enum values: `LZ4F_flush`'s capacity check
/// (lz4frame.c:1169) only budgets `BHSize + BFSize`, while `LZ4F_makeBlock`
/// returns `BHSize + cSize + crcFlag * BFSize` (lz4frame.c:907) — so a
/// `blockChecksumFlag` of e.g. `0xFFFFFFFF` makes the *return value* overshoot
/// the buffer by ~16 GB. The C would then have `LZ4F_compressEnd` write the
/// endMark through a wild pointer; the return value itself is still compared
/// between the two libraries, we simply refuse to continue the sequence.
unsafe fn drive_pipeline(
    api: &Api,
    prefs: &LZ4F_preferences_t,
    src: &[u8],
) -> (Vec<usize>, Vec<u8>) {
    const CAP: usize = 1 << 20;
    let cctx = new_cctx(api);
    let mut dst = vec![0xCDu8; CAP];
    let mut out = Vec::new();
    let mut off = 0usize;
    let mut alive = true;

    macro_rules! step {
        ($call:expr) => {
            if alive {
                let v: usize = $call;
                out.push(v);
                if is_err_range(v) || v > CAP - off {
                    alive = false;
                } else {
                    off += v;
                }
            }
        };
    }

    step!((api.compressBegin)(
        cctx,
        dst.as_mut_ptr() as *mut c_void,
        CAP,
        prefs
    ));
    step!((api.compressUpdate)(
        cctx,
        dst[off..].as_mut_ptr() as *mut c_void,
        CAP - off,
        src.as_ptr() as *const c_void,
        src.len(),
        ptr::null()
    ));
    step!((api.flush)(
        cctx,
        dst[off..].as_mut_ptr() as *mut c_void,
        CAP - off,
        ptr::null()
    ));
    step!((api.compressEnd)(
        cctx,
        dst[off..].as_mut_ptr() as *mut c_void,
        CAP - off,
        ptr::null()
    ));

    (api.freeCCtx)(cctx);
    // the WHOLE buffer is returned (not just `off` bytes) so that any
    // difference in scribbling outside the reported output is caught too
    (out, dst)
}

// ===========================================================================
// Generic FFI boundary: non-zero reserved fields
// ===========================================================================

/// `LZ4F_preferences_t::reserved`, `LZ4F_compressOptions_t::reserved` and
/// `LZ4F_decompressOptions_t::reserved0/1` are documented as "must be zero"
/// but are never checked by the C. Verify both libraries ignore them
/// identically.
#[test]
fn ffi_nonzero_reserved_fields_are_ignored_identically() {
    unsafe {
        let (c, r) = apis();
        let mut rng = Rng::new(0x2E5E2E5E);
        let src = gen(&mut rng, Shape::TextLike, 40_000);

        let patterns: [[c_uint; 3]; 5] = [
            [0, 0, 0],
            [1, 0, 0],
            [0, 1, 0],
            [0, 0, 1],
            [0xFFFF_FFFF, 0xDEAD_BEEF, 0x8000_0000],
        ];

        for pat in patterns {
            // (a) preferences.reserved through compressFrame
            let mut prefs = prefs_default();
            prefs.reserved = pat;
            let cap = (c.compressFrameBound)(src.len(), &prefs);
            same(
                &format!("reserved{pat:?}: compressFrameBound"),
                cap,
                (r.compressFrameBound)(src.len(), &prefs),
            );
            let mut cd = vec![0xCDu8; cap];
            let mut rd = vec![0xCDu8; cap];
            let a = (c.compressFrame)(
                cd.as_mut_ptr() as *mut c_void,
                cap,
                src.as_ptr() as *const c_void,
                src.len(),
                &prefs,
            );
            let b = (r.compressFrame)(
                rd.as_mut_ptr() as *mut c_void,
                cap,
                src.as_ptr() as *const c_void,
                src.len(),
                &prefs,
            );
            let ctx = format!("reserved prefs {pat:?}");
            same(&ctx, a, b);
            assert!(!is_err_range(a), "{ctx}: unexpected error {a:#x}");
            same_full_buffers(&ctx, &cd, &rd);

            // (b) compressOptions.reserved through the streaming pipeline
            let mut copt = LZ4F_compressOptions_t::default();
            copt.reserved = pat;
            for &stable in &[0u32, 1] {
                copt.stableSrc = stable;
                let mut cv = Vec::new();
                let mut rv = Vec::new();
                let mut cb: Vec<u8> = Vec::new();
                let mut rb: Vec<u8> = Vec::new();
                for (api, out, buf) in [(&c, &mut cv, &mut cb), (&r, &mut rv, &mut rb)] {
                    let cctx = new_cctx(api);
                    let mut dst = vec![0xCDu8; 1 << 19];
                    let mut off = 0usize;
                    let h = (api.compressBegin)(
                        cctx,
                        dst.as_mut_ptr() as *mut c_void,
                        dst.len(),
                        &prefs,
                    );
                    out.push(h);
                    off += h;
                    let u = (api.compressUpdate)(
                        cctx,
                        dst[off..].as_mut_ptr() as *mut c_void,
                        dst.len() - off,
                        src.as_ptr() as *const c_void,
                        src.len(),
                        &copt,
                    );
                    out.push(u);
                    off += u;
                    let f = (api.flush)(
                        cctx,
                        dst[off..].as_mut_ptr() as *mut c_void,
                        dst.len() - off,
                        &copt,
                    );
                    out.push(f);
                    off += f;
                    let e = (api.compressEnd)(
                        cctx,
                        dst[off..].as_mut_ptr() as *mut c_void,
                        dst.len() - off,
                        &copt,
                    );
                    out.push(e);
                    off += e;
                    (api.freeCCtx)(cctx);
                    dst.truncate(off);
                    *buf = dst;
                }
                let ctx = format!("reserved cOpt {pat:?} stableSrc={stable}");
                same_vec(&ctx, &cv, &rv);
                same_full_buffers(&ctx, &cb, &rb);
            }

            // (c) LZ4F_decompressOptions_t::reserved0 / reserved1 are also
            // documented "must be zero" and also never checked. Feed the frame
            // produced above back through both libraries' LZ4F_decompress with
            // garbage in the reserved words; each library uses its OWN dctx.
            assert_eq!(
                std::mem::size_of::<LZ4F_decompressOptions_t>(),
                16,
                "LZ4F_decompressOptions_t must be 4 x unsigned"
            );
            let frame = cd[..a].to_vec();
            for &skip in &[0u32, 1] {
                let mut dopt = LZ4F_decompressOptions_t::default();
                dopt.skipChecksums = skip;
                dopt.reserved0 = pat[0];
                dopt.reserved1 = pat[1];

                let mut rets = Vec::new();
                let mut outs: Vec<Vec<u8>> = Vec::new();
                for api in [&c, &r] {
                    let mut dctx: *mut c_void = ptr::null_mut();
                    assert_eq!((api.createDCtx)(&mut dctx, LZ4F_VERSION), 0);
                    let mut out = vec![0u8; src.len() + 64];
                    let mut produced = 0usize;
                    let mut consumed = 0usize;
                    let mut last = 0usize;
                    loop {
                        let mut dsz = out.len() - produced;
                        let mut ssz = frame.len() - consumed;
                        let ret = (api.decompress)(
                            dctx,
                            out[produced..].as_mut_ptr() as *mut c_void,
                            &mut dsz,
                            frame[consumed..].as_ptr() as *const c_void,
                            &mut ssz,
                            &dopt,
                        );
                        last = ret;
                        if is_err_range(ret) {
                            break;
                        }
                        produced += dsz;
                        consumed += ssz;
                        if ret == 0 || (dsz == 0 && ssz == 0) {
                            break;
                        }
                    }
                    (api.freeDCtx)(dctx);
                    out.truncate(produced);
                    rets.push(last);
                    outs.push(out);
                }
                let ctx = format!("reserved dOpt {pat:?} skipChecksums={skip}");
                same(&ctx, rets[0], rets[1]);
                expect(&ctx, rets[0], 0);
                same_full_buffers(&ctx, &outs[0], &outs[1]);
                assert_eq!(outs[0], src, "{ctx}: round-trip content mismatch");
            }
        }
    }
}

// ===========================================================================
// Generic FFI boundary: unvalidated `version`
// ===========================================================================

/// `LZ4F_VERSION` is stored in the context but never validated
/// (`ERRORS.md` constants table, lz4frame.h:256). Any value must therefore
/// behave exactly like `LZ4F_VERSION`.
#[test]
fn ffi_context_version_is_stored_but_never_validated() {
    unsafe {
        let (c, r) = apis();
        let mut rng = Rng::new(0x5E51);
        let src = gen(&mut rng, Shape::TextLike, 20_000);

        same("ffi: getVersion", (c.getVersion)() as usize, (r.getVersion)() as usize);
        assert_eq!((c.getVersion)(), LZ4F_VERSION, "ffi: unexpected LZ4F_VERSION");
        assert_eq!(
            (c.compressionLevel_max)(),
            (r.compressionLevel_max)(),
            "ffi: compressionLevel_max differs"
        );

        for &version in &[0u32, 1, 99, 100, 101, 12345, 0xFFFF_FFFF] {
            // compression context
            let mut cv = Vec::new();
            let mut rv = Vec::new();
            let mut cb: Vec<u8> = Vec::new();
            let mut rb: Vec<u8> = Vec::new();
            for (api, out, buf) in [(&c, &mut cv, &mut cb), (&r, &mut rv, &mut rb)] {
                let mut cctx: *mut c_void = ptr::null_mut();
                let cr = (api.createCCtx)(&mut cctx, version);
                out.push(cr);
                assert_eq!(cr, 0, "{}: createCompressionContext({version}) = {cr:#x}", api.tag);
                assert!(!cctx.is_null());
                let mut dst = vec![0xCDu8; 1 << 18];
                let mut off = 0usize;
                let h =
                    (api.compressBegin)(cctx, dst.as_mut_ptr() as *mut c_void, dst.len(), ptr::null());
                out.push(h);
                off += h;
                let u = (api.compressUpdate)(
                    cctx,
                    dst[off..].as_mut_ptr() as *mut c_void,
                    dst.len() - off,
                    src.as_ptr() as *const c_void,
                    src.len(),
                    ptr::null(),
                );
                out.push(u);
                off += u;
                let e = (api.compressEnd)(
                    cctx,
                    dst[off..].as_mut_ptr() as *mut c_void,
                    dst.len() - off,
                    ptr::null(),
                );
                out.push(e);
                off += e;
                out.push((api.freeCCtx)(cctx));
                dst.truncate(off);
                *buf = dst;

                // decompression context
                let mut dctx: *mut c_void = ptr::null_mut();
                let dr = (api.createDCtx)(&mut dctx, version);
                out.push(dr);
                assert_eq!(dr, 0, "{}: createDecompressionContext({version}) = {dr:#x}", api.tag);
                assert!(!dctx.is_null());
                out.push((api.freeDCtx)(dctx));

                // advanced constructors with a working custom allocator
                let mut hook = Hook::new(0);
                let cm = cmem_of(&mut hook, true);
                let ac = (api.createCCtxAdv)(cm, version);
                assert!(!ac.is_null());
                out.push((api.freeCCtx)(ac));
                let ad = (api.createDCtxAdv)(cm, version);
                assert!(!ad.is_null());
                out.push((api.freeDCtx)(ad));
                assert_eq!(hook.live, 0, "{}: leaked", api.tag);
            }
            let ctx = format!("ffi: version={version}");
            same_vec(&ctx, &cv, &rv);
            same_full_buffers(&ctx, &cb, &rb);
        }
    }
}

// ===========================================================================
// Generic FFI boundary: the error-reporting helpers
// ===========================================================================

/// `LZ4F_isError` / `LZ4F_getErrorCode` / `LZ4F_getErrorName` over the whole
/// boundary. `LZ4F_isError` is true exactly on `[(size_t)-23 ..= (size_t)-1]`
/// (lz4frame.c:293-296).
#[test]
fn ffi_isError_getErrorCode_getErrorName_boundary() {
    unsafe {
        let (c, r) = apis();

        let mut codes: Vec<usize> = vec![0, 1, 23, 24, 25, err(100), usize::MAX, usize::MAX / 2];
        for k in 0..=25usize {
            codes.push(err(k));
        }
        codes.push(err(23));
        codes.push(err(24));
        codes.push(usize::MAX / 2 + 1);
        codes.push(1 << 62);
        let mut rng = Rng::new(0xE1201);
        for _ in 0..3000 {
            codes.push(rng.next_u64() as usize);
        }
        codes.sort();
        codes.dedup();

        for &code in &codes {
            let ia = (c.isError)(code);
            let ib = (r.isError)(code);
            assert_eq!(
                ia, ib,
                "ffi: LZ4F_isError({code:#x}) C={ia} Rust={ib}"
            );
            let want_is_err = code > err(24);
            assert_eq!(
                ia != 0, want_is_err,
                "ffi: LZ4F_isError({code:#x}) = {ia}, expected {want_is_err}"
            );

            let ca = (c.getErrorCode)(code);
            let cbb = (r.getErrorCode)(code);
            assert_eq!(
                ca, cbb,
                "ffi: LZ4F_getErrorCode({code:#x}) C={ca} Rust={cbb}"
            );
            let want_code = if want_is_err {
                (0i64).wrapping_sub(code as i64) as i32
            } else {
                0
            };
            assert_eq!(
                ca, want_code,
                "ffi: LZ4F_getErrorCode({code:#x}) = {ca}, expected {want_code}"
            );

            let na = (c.getErrorName)(code);
            let nb = (r.getErrorName)(code);
            assert!(!na.is_null(), "ffi: C getErrorName({code:#x}) returned NULL");
            assert!(
                !nb.is_null(),
                "ffi: Rust getErrorName({code:#x}) returned NULL while C returned {:?}",
                CStr::from_ptr(na)
            );
            let sa = CStr::from_ptr(na).to_bytes();
            let sb = CStr::from_ptr(nb).to_bytes();
            assert_eq!(
                sa,
                sb,
                "ffi: LZ4F_getErrorName({code:#x}) differs: C={:?} Rust={:?}",
                String::from_utf8_lossy(sa),
                String::from_utf8_lossy(sb)
            );
            if !want_is_err {
                assert_eq!(
                    sa, b"Unspecified error code",
                    "ffi: non-error {code:#x} should map to the literal"
                );
            }
        }

        // the exact strings for the whole enum, spelled out
        let expected: [(usize, &[u8]); 23] = [
            (err(1), b"ERROR_GENERIC"),
            (err(2), b"ERROR_maxBlockSize_invalid"),
            (err(3), b"ERROR_blockMode_invalid"),
            (err(4), b"ERROR_parameter_invalid"),
            (err(5), b"ERROR_compressionLevel_invalid"),
            (err(6), b"ERROR_headerVersion_wrong"),
            (err(7), b"ERROR_blockChecksum_invalid"),
            (err(8), b"ERROR_reservedFlag_set"),
            (err(9), b"ERROR_allocation_failed"),
            (err(10), b"ERROR_srcSize_tooLarge"),
            (err(11), b"ERROR_dstMaxSize_tooSmall"),
            (err(12), b"ERROR_frameHeader_incomplete"),
            (err(13), b"ERROR_frameType_unknown"),
            (err(14), b"ERROR_frameSize_wrong"),
            (err(15), b"ERROR_srcPtr_wrong"),
            (err(16), b"ERROR_decompressionFailed"),
            (err(17), b"ERROR_headerChecksum_invalid"),
            (err(18), b"ERROR_contentChecksum_invalid"),
            (err(19), b"ERROR_frameDecoding_alreadyStarted"),
            (err(20), b"ERROR_compressionState_uninitialized"),
            (err(21), b"ERROR_parameter_null"),
            (err(22), b"ERROR_io_write"),
            (err(23), b"ERROR_io_read"),
        ];
        for (code, name) in expected {
            let sa = CStr::from_ptr((c.getErrorName)(code)).to_bytes();
            let sb = CStr::from_ptr((r.getErrorName)(code)).to_bytes();
            assert_eq!(sa, name, "ffi: C getErrorName({code:#x}) = {:?}", String::from_utf8_lossy(sa));
            assert_eq!(sb, name, "ffi: Rust getErrorName({code:#x}) = {:?}", String::from_utf8_lossy(sb));
        }
    }
}

// ===========================================================================
// Property-style sweep across the whole compression error surface
// ===========================================================================

/// Randomized differential sweep with a fixed seed. Every axis that ERRORS.md
/// rows 1..23 depend on is randomized at once — preferences (including
/// out-of-contract enum values and non-zero `reserved` words), compression
/// level, chunking, and — crucially — a destination capacity that is
/// deliberately too small a lot of the time. Every return value, the produced
/// bytes AND the whole destination buffer must match.
///
/// `blockSizeID` is kept inside `{0,4,5,6,7}` here: with an out-of-range id the
/// C sets `maxBlockSize = (size_t)-2` while `tmpBuff` is only
/// `(size_t)-2 + 128 KB == 131070` bytes (lz4frame.c:742-751 wraps), so a
/// multi-chunk `autoFlush == 0` session would make the C itself overrun its own
/// heap buffer at lz4frame.c:1096. Out-of-range ids are swept exhaustively in
/// `ffi_out_of_range_enum_values_*` with a single small update instead.
#[test]
fn ffi_property_random_parameter_and_capacity_sweep() {
    unsafe {
        let (c, r) = apis();
        let mut rng = Rng::new(0xF00D_5EED);

        for iter in 0..500 {
            let shape = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
            let len = match rng.below(4) {
                0 => rng.range(0, 32),
                1 => rng.range(0, 4096),
                2 => rng.range(0, 70_000),
                _ => rng.range(60_000, 200_000),
            };
            let src = gen(&mut rng, shape, len);

            let mut prefs = LZ4F_preferences_t::default();
            prefs.frameInfo.blockSizeID =
                [LZ4F_DEFAULT, LZ4F_MAX64KB, LZ4F_MAX256KB, LZ4F_MAX1MB, LZ4F_MAX4MB][rng.below(5)];
            // out-of-contract values are legal *inputs* across the FFI boundary
            prefs.frameInfo.blockMode = BLOCKMODE_SWEEP[rng.below(BLOCKMODE_SWEEP.len())];
            prefs.frameInfo.contentChecksumFlag = [0u32, 1, 2][rng.below(3)];
            prefs.frameInfo.blockChecksumFlag = [0u32, 1, 2][rng.below(3)];
            prefs.frameInfo.frameType = FRAMETYPE_SWEEP[rng.below(FRAMETYPE_SWEEP.len())];
            prefs.frameInfo.dictID = if rng.below(2) == 0 { 0 } else { rng.next_u32() };
            prefs.frameInfo.contentSize = match rng.below(4) {
                0 => 0,
                1 => len as u64,
                2 => rng.next_u64() % 500_000,
                _ => u64::MAX,
            };
            prefs.compressionLevel =
                [-65540i32, -1, 0, 1, 2, 3, 9, 10, 12, 13, 1000, i32::MIN, i32::MAX][rng.below(13)];
            prefs.autoFlush = rng.below(2) as c_uint;
            prefs.favorDecSpeed = [0u32, 1, 2, 0xFFFF_FFFF][rng.below(4)];
            prefs.reserved = if rng.below(3) == 0 {
                [rng.next_u32(), rng.next_u32(), rng.next_u32()]
            } else {
                [0, 0, 0]
            };

            let mut copt = LZ4F_compressOptions_t::default();
            copt.stableSrc = rng.below(2) as c_uint;
            if rng.below(3) == 0 {
                copt.reserved = [rng.next_u32(), rng.next_u32(), rng.next_u32()];
            }
            let use_copt = rng.below(2) == 0;
            let coptp = if use_copt {
                &copt as *const LZ4F_compressOptions_t
            } else {
                ptr::null()
            };

            // random chunking
            let mut chunks: Vec<(usize, usize)> = Vec::new();
            let mut off = 0usize;
            while off < len {
                let capn = rng.range(1, 90_000);
                let n = rng.range(1, (len - off).min(capn));
                chunks.push((off, n));
                off += n;
            }
            if chunks.is_empty() {
                chunks.push((0, 0));
            }
            let use_uncompressed = prefs.frameInfo.blockMode == LZ4F_BLOCK_INDEPENDENT
                && rng.below(4) == 0;

            // Destination capacity: generous most of the time, but frequently
            // short enough to trip rows 9 / 14 / 15 / 17 / 18 / 19.
            let generous = len + (8 << 20) / 4;
            let cap = match rng.below(5) {
                0 => rng.range(0, 32),
                1 => rng.range(0, len.max(1)),
                2 => rng.range(0, generous),
                _ => generous,
            };

            // NOTE: every random decision must be made BEFORE the per-library
            // loop, otherwise the two libraries get different call sequences.
            let do_flush = rng.below(2) == 0;

            let mut cv = Vec::new();
            let mut rv = Vec::new();
            let mut cb: Vec<u8> = Vec::new();
            let mut rb: Vec<u8> = Vec::new();
            for (api, out, buf) in [(&c, &mut cv, &mut cb), (&r, &mut rv, &mut rb)] {
                let cctx = new_cctx(api);
                let mut dst = vec![0xCDu8; generous + 64];
                let mut w = 0usize; // bytes reported so far
                let mut alive = true;

                macro_rules! step {
                    ($e:expr) => {
                        if alive {
                            let v: usize = $e;
                            out.push(v);
                            if is_err_range(v) || v > cap - w {
                                alive = false;
                            } else {
                                w += v;
                            }
                        }
                    };
                }

                step!((api.compressBegin)(
                    cctx,
                    dst.as_mut_ptr() as *mut c_void,
                    cap,
                    &prefs
                ));
                for &(o, n) in &chunks {
                    let f = if use_uncompressed {
                        api.uncompressedUpdate
                    } else {
                        api.compressUpdate
                    };
                    step!(f(
                        cctx,
                        dst[w..].as_mut_ptr() as *mut c_void,
                        cap - w,
                        src[o..].as_ptr() as *const c_void,
                        n,
                        coptp
                    ));
                }
                if do_flush {
                    step!((api.flush)(
                        cctx,
                        dst[w..].as_mut_ptr() as *mut c_void,
                        cap - w,
                        coptp
                    ));
                }
                step!((api.compressEnd)(
                    cctx,
                    dst[w..].as_mut_ptr() as *mut c_void,
                    cap - w,
                    coptp
                ));
                (api.freeCCtx)(cctx);
                *buf = dst;
            }

            let ctx = format!(
                "property iter={iter} shape={shape:?} len={len} cap={cap} \
                 bsid={} bm={:#x} cc={} bc={} lvl={} af={} chunks={} unc={}",
                prefs.frameInfo.blockSizeID,
                prefs.frameInfo.blockMode,
                prefs.frameInfo.contentChecksumFlag,
                prefs.frameInfo.blockChecksumFlag,
                prefs.compressionLevel,
                prefs.autoFlush,
                chunks.len(),
                use_uncompressed
            );
            same_vec(&ctx, &cv, &rv);
            same_full_buffers(&ctx, &cb, &rb);
            // every error must be one of the sentinels ERRORS.md rows 9/13/14/
            // 15/17/18/19/20 can produce here
            for &v in &cv {
                if is_err_range(v) {
                    let code = (0usize).wrapping_sub(v);
                    assert!(
                        matches!(code, 11 | 14 | 20),
                        "{ctx}: unexpected error code {code} ({v:#x})"
                    );
                }
            }
        }
    }
}
