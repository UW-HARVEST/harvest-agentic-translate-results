//! # t20_params — the PARAMETER surface, C vs Rust
//!
//! Phase B (valid inputs) + Phase C (error inputs) for everything that reads or
//! writes a compression / decompression *parameter*:
//!
//! * `ZSTD_cParam_getBounds` / `ZSTD_dParam_getBounds` (the two big switches),
//! * `ZSTD_CCtx_setParameter` / `getParameter`,
//!   `ZSTD_CCtxParams_setParameter` / `getParameter`,
//!   `ZSTD_DCtx_setParameter` / `getParameter`,
//! * the `BOUNDCHECK` / `if (value!=0) BOUNDCHECK` / clamp / no-check
//!   disciplines that differ *per parameter*,
//! * the `ZSTD_isUpdateAuthorized` stage gate and the `ZSTD_CCtx_reset` /
//!   `ZSTD_DCtx_reset` directives,
//! * `ZSTD_CCtx_setPledgedSrcSize`,
//! * `ZSTD_getCParams` / `ZSTD_getParams` / `ZSTD_checkCParams` /
//!   `ZSTD_adjustCParams` / `ZSTD_cycleLog`,
//! * `ZSTD_compressBound` / `ZSTD_decompressBound` / `ZSTD_sizeof_*` /
//!   `ZSTD_estimate*Size*`.
//!
//! Everything goes through `dlsym` on both shared objects, so the `#[no_mangle]`
//! wrappers are under test too. All sweeps use a fixed seed.
//!
//! ## Build-specific behaviour these tests pin down
//!
//! `ZSTD_MULTITHREAD` is **not** defined, which makes four parameters behave
//! asymmetrically and is easy to get wrong in a translation:
//!
//! * `nbWorkers` / `jobSize` / `overlapLog` report bounds `{0,0}` and *reject*
//!   any non-zero value with `parameter_unsupported` (40); `rsyncable` reports
//!   `{0,1}` but still rejects non-zero.
//! * `get(jobSize)` / `get(overlapLog)` / `get(rsyncable)` fail *unconditionally*
//!   with `parameter_unsupported`, while `get(nbWorkers)` **succeeds** and writes
//!   0 (the `assert(nbWorkers==0)` guarding it is compiled out at `DEBUGLEVEL=0`).
//!
//! Other non-obvious C behaviour asserted below: `compressionLevel` is *clamped*
//! rather than rejected; `targetCBlockSize` values in `1..1339` are silently
//! *raised* to 1340; `forceMaxWindow` and `enableDedicatedDictSearch` have no
//! bound check at all and store `value != 0`; parameters are *sticky* across a
//! session-only reset; `ZSTD_d_windowLogMax` maps `0` to 27 *before* the check
//! while `ZSTD_d_maxBlockSize` skips the check entirely for `0` and reads back
//! the raw `0`.

mod common;
use common::*;

use std::ffi::{c_int, c_uint, c_ulonglong, c_void};

// ---------------------------------------------------------------------------
// Extra FFI signatures (the ones `common` does not already declare)
// ---------------------------------------------------------------------------

type FnCCtxParamsInit = unsafe extern "C" fn(*mut c_void, c_int) -> SizeT;
type FnCCtxParamsInitAdvanced = unsafe extern "C" fn(*mut c_void, ZSTD_parameters) -> SizeT;
type FnCCtxParamsReset = unsafe extern "C" fn(*mut c_void) -> SizeT;
type FnCCtxParamsGetParameter = unsafe extern "C" fn(*const c_void, c_int, *mut c_int) -> SizeT;
type FnSetParametersUsingCCtxParams = unsafe extern "C" fn(*mut c_void, *const c_void) -> SizeT;
type FnSetCParams = unsafe extern "C" fn(*mut c_void, ZSTD_compressionParameters) -> SizeT;
type FnSetFParams = unsafe extern "C" fn(*mut c_void, ZSTD_frameParameters) -> SizeT;
type FnSetParams = unsafe extern "C" fn(*mut c_void, ZSTD_parameters) -> SizeT;
type FnSetPledgedSrcSize = unsafe extern "C" fn(*mut c_void, c_ulonglong) -> SizeT;
type FnGetCParams =
    unsafe extern "C" fn(c_int, c_ulonglong, SizeT) -> ZSTD_compressionParameters;
type FnGetParams = unsafe extern "C" fn(c_int, c_ulonglong, SizeT) -> ZSTD_parameters;
type FnCheckCParams = unsafe extern "C" fn(ZSTD_compressionParameters) -> SizeT;
type FnAdjustCParams = unsafe extern "C" fn(
    ZSTD_compressionParameters,
    c_ulonglong,
    SizeT,
) -> ZSTD_compressionParameters;
type FnCycleLog = unsafe extern "C" fn(c_uint, c_int) -> c_uint;
type FnSizeofOpaque = unsafe extern "C" fn(*const c_void) -> SizeT;
type FnEstimateFromLevel = unsafe extern "C" fn(c_int) -> SizeT;
type FnEstimateFromCParams = unsafe extern "C" fn(ZSTD_compressionParameters) -> SizeT;
type FnEstimateFromCCtxParams = unsafe extern "C" fn(*const c_void) -> SizeT;
type FnEstimateVoid = unsafe extern "C" fn() -> SizeT;
type FnEstimateFromSize = unsafe extern "C" fn(SizeT) -> SizeT;
type FnEstimateFromFrame = unsafe extern "C" fn(*const c_void, SizeT) -> SizeT;
type FnDecompressBound = unsafe extern "C" fn(*const c_void, SizeT) -> c_ulonglong;
type FnInitStaticCCtx = unsafe extern "C" fn(*mut c_void, SizeT) -> *mut c_void;
type FnSetMaxWindowSize = unsafe extern "C" fn(*mut c_void, SizeT) -> SizeT;
type FnEstimateCDictSize = unsafe extern "C" fn(SizeT, c_int) -> SizeT;
type FnEstimateCDictSizeAdvanced =
    unsafe extern "C" fn(SizeT, ZSTD_compressionParameters, c_int) -> SizeT;
type FnEstimateDDictSize = unsafe extern "C" fn(SizeT, c_int) -> SizeT;

// ---------------------------------------------------------------------------
// Constants the header defines but `common` does not export
// ---------------------------------------------------------------------------

/// `ZSTD_cParameter` ids that no enumerator uses. A C `switch` over an enum
/// accepts any `int`, so these must all land in the `default:` arm. The values
/// straddle every valid island: 108 is just past `strategy`, 131 just past
/// `targetCBlockSize`, 165 past `ldmHashRateLog`, 203 past `dictIDFlag`, 403
/// past `overlapLog`, 501 past `rsyncable`, 1003 is the retired
/// `experimentalParam6`, and 1018 is just past `blockSplitterLevel`.
const INVALID_CPARAM_IDS: &[c_int] = &[
    c_int::MIN,
    -1000,
    -1,
    0,
    1,
    2,
    9,
    11,
    12,
    99,
    108,
    109,
    120,
    129,
    131,
    132,
    159,
    165,
    166,
    199,
    203,
    204,
    399,
    403,
    404,
    499,
    501,
    502,
    999,
    1003,
    1018,
    1019,
    1100,
    9999,
    c_int::MAX,
];

/// `ZSTD_dParameter` ids that no enumerator uses (valid: 100, 1000..=1005).
const INVALID_DPARAM_IDS: &[c_int] = &[
    c_int::MIN,
    -1000,
    -1,
    0,
    1,
    2,
    99,
    101,
    102,
    107,
    130,
    200,
    400,
    999,
    1006,
    1007,
    1100,
    9999,
    c_int::MAX,
];

/// `ZSTD_ResetDirective` values outside `{1,2,3}`. `ZSTD_CCtx_reset` /
/// `ZSTD_DCtx_reset` compare `reset` against the three enumerators with `==`
/// only, so an out-of-range directive is a silent *no-op returning 0* — not an
/// error. That is easy to translate as an exhaustive `match` that panics or
/// errors instead.
const INVALID_RESET_DIRECTIVES: &[c_int] = &[c_int::MIN, -1, 0, 4, 5, 99, c_int::MAX];

/// Sentinel written into the `int*` out-parameter before every `getParameter`
/// call: if the C leaves it untouched on the error path, the Rust must too.
const SENTINEL: c_int = -0x5A5A_5A5A;

const CONTENTSIZE_UNKNOWN_U64: u64 = u64::MAX;

// ---------------------------------------------------------------------------
// Small helpers
// ---------------------------------------------------------------------------

fn min_clevel(l: &Lib) -> c_int {
    unsafe { l.sym::<FnMinCLevel>("ZSTD_minCLevel")() }
}
fn max_clevel(l: &Lib) -> c_int {
    unsafe { l.sym::<FnMaxCLevel>("ZSTD_maxCLevel")() }
}

fn cparam_bounds(l: &Lib, param: c_int) -> ZSTD_bounds {
    unsafe { l.sym::<FnCParamGetBounds>("ZSTD_cParam_getBounds")(param) }
}
fn dparam_bounds(l: &Lib, param: c_int) -> ZSTD_bounds {
    unsafe { l.sym::<FnDParamGetBounds>("ZSTD_dParam_getBounds")(param) }
}

/// Bounds rendered so a divergence names the error instead of printing
/// `18446744073709551576`.
fn bounds_triple(l: &Lib, b: ZSTD_bounds) -> (R, c_int, c_int) {
    (res(l, b.error), b.lowerBound, b.upperBound)
}

/// The value set every `setParameter` sweep uses: the documented bounds and
/// their neighbours (this is where CLAMP-vs-REJECT-vs-`0 means default` shows
/// up), the two `int` extremes, and 200 pseudo-random values biased towards the
/// interesting region. Fixed seed derived from the parameter id, so a failure is
/// reproducible.
fn value_set(lo: c_int, hi: c_int, seed: u64) -> Vec<c_int> {
    let mut rng = Rng::new(seed);
    let clamp = |x: i64| x.clamp(c_int::MIN as i64, c_int::MAX as i64) as c_int;
    let mid = clamp(lo as i64 + (hi as i64 - lo as i64) / 2);
    let mut v = vec![
        lo.saturating_sub(2),
        lo.saturating_sub(1),
        lo,
        lo.saturating_add(1),
        -2,
        -1,
        0,
        1,
        2,
        3,
        mid,
        hi.saturating_sub(1),
        hi,
        hi.saturating_add(1),
        hi.saturating_add(2),
        c_int::MIN,
        c_int::MIN + 1,
        c_int::MAX - 1,
        c_int::MAX,
    ];
    for _ in 0..200 {
        let x = match rng.below(4) {
            // tiny values: where the "0 means default" and the flag-like
            // parameters live
            0 => rng.range(-6, 10),
            // straddling this parameter's own bounds
            1 => rng.range(lo as i64 - 3, hi as i64 + 3),
            // arbitrary 32-bit patterns, incl. negatives from sign extension
            2 => rng.next_u32() as i32 as i64,
            // the log/size range plus the compressionLevel clamp range
            _ => rng.range(-140_000, 140_000),
        };
        v.push(clamp(x));
    }
    v.sort_unstable();
    v.dedup();
    v
}

/// `set(param, value)` then `get(param)` on a freshly created CCtx. The tuple is
/// what `diff` compares: the set result, the get result and the *value written
/// through the out-pointer* (still `SENTINEL` if the C wrote nothing).
fn cctx_set_then_get(l: &Lib, param: c_int, value: c_int) -> (R, R, c_int) {
    let ctx = Ctx::cctx(l);
    let set = l.sym::<FnCCtxSetParameter>("ZSTD_CCtx_setParameter");
    let get = l.sym::<FnCCtxGetParameter>("ZSTD_CCtx_getParameter");
    let sr = res(l, unsafe { set(ctx.ptr, param, value) });
    let mut out = SENTINEL;
    let gr = res(l, unsafe { get(ctx.ptr, param, &mut out) });
    (sr, gr, out)
}

/// Same, on a heap `ZSTD_CCtx_params` object. `ZSTD_CCtxParams_setParameter` has
/// **no** stage check and no `nbWorkers`-on-static-cctx check, so any difference
/// against `cctx_set_then_get` is exactly the CCtx-level pre-checks.
fn cctxparams_set_then_get(l: &Lib, param: c_int, value: c_int) -> (R, R, c_int) {
    let p = new_cctx_params(l);
    let set = l.sym::<FnCCtxSetParameter>("ZSTD_CCtxParams_setParameter");
    let get = l.sym::<FnCCtxParamsGetParameter>("ZSTD_CCtxParams_getParameter");
    let sr = res(l, unsafe { set(p.ptr, param, value) });
    let mut out = SENTINEL;
    let gr = res(l, unsafe { get(p.ptr, param, &mut out) });
    (sr, gr, out)
}

fn dctx_set_then_get(l: &Lib, param: c_int, value: c_int) -> (R, R, c_int) {
    let ctx = Ctx::dctx(l);
    let set = l.sym::<FnDCtxSetParameter>("ZSTD_DCtx_setParameter");
    let get = l.sym::<FnCCtxGetParameter>("ZSTD_DCtx_getParameter");
    let sr = res(l, unsafe { set(ctx.ptr, param, value) });
    let mut out = SENTINEL;
    let gr = res(l, unsafe { get(ctx.ptr, param, &mut out) });
    (sr, gr, out)
}

fn new_cctx_params(l: &Lib) -> Ctx<'_> {
    Ctx::new(l, "ZSTD_createCCtxParams", "ZSTD_freeCCtxParams")
}

/// Read *every* one of the 39 `ZSTD_c_*` parameters out of a CCtx (or a
/// CCtx_params). Used to prove that a rejected `set` leaves the whole object
/// untouched and that `setParametersUsingCCtxParams` copies all 39 faithfully.
fn snapshot_all(l: &Lib, ptr: *mut c_void, getter: &str) -> Vec<(&'static str, R, c_int)> {
    let get = l.sym::<FnCCtxParamsGetParameter>(getter);
    ALL_CPARAMS
        .iter()
        .map(|&(name, id)| {
            let mut out = SENTINEL;
            let r = res(l, unsafe { get(ptr, id, &mut out) });
            (name, r, out)
        })
        .collect()
}

fn snapshot_all_dparams(l: &Lib, dctx: *mut c_void) -> Vec<(&'static str, R, c_int)> {
    let get = l.sym::<FnCCtxParamsGetParameter>("ZSTD_DCtx_getParameter");
    ALL_DPARAMS
        .iter()
        .map(|&(name, id)| {
            let mut out = SENTINEL;
            let r = res(l, unsafe { get(dctx, id, &mut out) });
            (name, r, out)
        })
        .collect()
}

// ---- streaming helpers, used to move a CCtx/DCtx out of the init stage ------

fn stream_continue(l: &Lib, ctx: &Ctx, src: &[u8], out: &mut [u8]) -> (R, usize, usize) {
    let f = l.sym::<FnCompressStream2>("ZSTD_compressStream2");
    let mut i = ZSTD_inBuffer {
        src: src.as_ptr() as *const c_void,
        size: src.len(),
        pos: 0,
    };
    let mut o = ZSTD_outBuffer {
        dst: out.as_mut_ptr() as *mut c_void,
        size: out.len(),
        pos: 0,
    };
    let r = res(l, unsafe { f(ctx.ptr, &mut o, &mut i, ZSTD_e_continue) });
    (r, i.pos, o.pos)
}

/// Drive `ZSTD_e_end` to completion; returns the final status and the number of
/// bytes produced. `out` must be big enough or this returns a synthetic error
/// rather than looping forever.
fn stream_end(l: &Lib, ctx: &Ctx, src: &[u8], out: &mut [u8]) -> (R, usize) {
    let f = l.sym::<FnCompressStream2>("ZSTD_compressStream2");
    let mut ipos = 0usize;
    let mut opos = 0usize;
    for _ in 0..64 {
        let mut i = ZSTD_inBuffer {
            src: src.as_ptr() as *const c_void,
            size: src.len(),
            pos: ipos,
        };
        let mut o = ZSTD_outBuffer {
            dst: out.as_mut_ptr() as *mut c_void,
            size: out.len(),
            pos: opos,
        };
        let n = unsafe { f(ctx.ptr, &mut o, &mut i, ZSTD_e_end) };
        ipos = i.pos;
        opos = o.pos;
        match res(l, n) {
            R::Ok(0) => return (R::Ok(opos), opos),
            R::Ok(_) => {
                if opos >= out.len() {
                    return (R::Err(-999, "test harness: out buffer full".into()), opos);
                }
            }
            e => return (e, opos),
        }
    }
    (R::Err(-998, "test harness: e_end did not converge".into()), opos)
}

fn cctx_reset(l: &Lib, ctx: &Ctx, directive: c_int) -> R {
    let f = l.sym::<FnCCtxReset>("ZSTD_CCtx_reset");
    res(l, unsafe { f(ctx.ptr, directive) })
}
fn dctx_reset(l: &Lib, ctx: &Ctx, directive: c_int) -> R {
    let f = l.sym::<FnDCtxReset>("ZSTD_DCtx_reset");
    res(l, unsafe { f(ctx.ptr, directive) })
}

/// The CCtx states a parameter setter can be called from.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Stage {
    /// Fresh context, `streamStage == zcss_init`.
    Fresh,
    /// One `ZSTD_compressStream2(..., ZSTD_e_continue)` — frame open,
    /// `streamStage == zcss_load`.
    MidFrame,
    /// Frame opened and then finished with `ZSTD_e_end`; back to `zcss_init`
    /// but with the previously-set parameters still sticky.
    AfterEnd,
    /// Mid-frame, then `ZSTD_CCtx_reset(ZSTD_reset_session_only)`.
    MidThenResetSession,
    /// Mid-frame, then `ZSTD_CCtx_reset(ZSTD_reset_parameters)` — which itself
    /// must fail with `stage_wrong` and leave the stage alone.
    MidThenResetParams,
    /// Mid-frame, then `ZSTD_CCtx_reset(ZSTD_reset_session_and_parameters)`.
    MidThenResetBoth,
}

const ALL_STAGES: &[Stage] = &[
    Stage::Fresh,
    Stage::MidFrame,
    Stage::AfterEnd,
    Stage::MidThenResetSession,
    Stage::MidThenResetParams,
    Stage::MidThenResetBoth,
];

/// Build a CCtx in the requested stage. The returned `R` is the status of the
/// last set-up call and is folded into the compared tuple so a divergence in the
/// *set-up* is not silently attributed to the parameter call.
fn cctx_in_stage<'l>(l: &'l Lib, st: Stage, src: &[u8], out: &mut [u8]) -> (Ctx<'l>, R) {
    let ctx = Ctx::cctx(l);
    let pre = match st {
        Stage::Fresh => R::Ok(0),
        Stage::MidFrame => stream_continue(l, &ctx, src, &mut out[..64]).0,
        Stage::AfterEnd => {
            let a = stream_continue(l, &ctx, src, &mut out[..64]).0;
            if matches!(a, R::Err(..)) {
                a
            } else {
                stream_end(l, &ctx, src, out).0
            }
        }
        Stage::MidThenResetSession => {
            stream_continue(l, &ctx, src, &mut out[..64]);
            cctx_reset(l, &ctx, ZSTD_reset_session_only)
        }
        Stage::MidThenResetParams => {
            stream_continue(l, &ctx, src, &mut out[..64]);
            cctx_reset(l, &ctx, ZSTD_reset_parameters)
        }
        Stage::MidThenResetBoth => {
            stream_continue(l, &ctx, src, &mut out[..64]);
            cctx_reset(l, &ctx, ZSTD_reset_session_and_parameters)
        }
    };
    (ctx, pre)
}

/// Append the 7 `cParams` fields to a byte buffer. Used for the huge sweeps
/// where one `diff_bytes` over a packed buffer localises the first differing
/// case far better than 100 000 individual `diff` calls would run.
fn push_cparams(v: &mut Vec<u8>, c: &ZSTD_compressionParameters) {
    v.extend_from_slice(&c.windowLog.to_le_bytes());
    v.extend_from_slice(&c.chainLog.to_le_bytes());
    v.extend_from_slice(&c.hashLog.to_le_bytes());
    v.extend_from_slice(&c.searchLog.to_le_bytes());
    v.extend_from_slice(&c.minMatch.to_le_bytes());
    v.extend_from_slice(&c.targetLength.to_le_bytes());
    v.extend_from_slice(&c.strategy.to_le_bytes());
}

/// srcSizeHint values that straddle every `ZSTD_getCParamRowSize` /
/// `tableID = (r<=256K)+(r<=128K)+(r<=16K)` boundary, plus the
/// `ZSTD_CONTENTSIZE_UNKNOWN` wrap case.
const SRC_HINTS: &[u64] = &[
    0,
    1,
    256,
    16 * 1024,
    16 * 1024 + 1,
    128 * 1024,
    128 * 1024 + 1,
    256 * 1024,
    256 * 1024 + 1,
    1 << 20,
    CONTENTSIZE_UNKNOWN_U64,
];

/// dictSizes chosen around the `+500` fudge that `ZSTD_getCParamRowSize` adds
/// when srcSizeHint is unknown (which makes the sum *wrap*: 499 lands exactly on
/// `dictSize + 499 == 0` for `dictSize == 1`).
const DICT_SIZES: &[usize] = &[0, 1, 499, 500, 4096, 1 << 20];

// ===========================================================================
// 1. bounds
// ===========================================================================

/// The three compression-level constants feed every clamp in the library
/// (`ZSTD_cParam_getBounds(ZSTD_c_compressionLevel)`,
/// `ZSTD_cParam_clampBounds`, `ZSTD_getCParams_internal`'s negative-level
/// acceleration factor), so pin them first: if they differ, dozens of other
/// comparisons below become meaningless.
#[test]
fn clevel_constants() {
    covers(&["CFG:1"]);
    diff("ZSTD_minCLevel", |l| min_clevel(l));
    diff("ZSTD_maxCLevel", |l| max_clevel(l));
    diff("ZSTD_defaultCLevel", |l| unsafe {
        l.sym::<FnDefaultCLevel>("ZSTD_defaultCLevel")()
    });
}

/// `ZSTD_cParam_getBounds` for all 39 enumerators plus every out-of-range id:
/// the whole `switch` at `zstd_compress.c:435..636` including its `default:`
/// arm. Compared field by field (`error`, `lowerBound`, `upperBound`) because a
/// translation that returns `{0,0,0}` for an unsupported parameter instead of
/// `{ERROR(parameter_unsupported),0,0}` would otherwise look fine — the struct
/// is returned *by value*, so this also checks the `ZSTD_bounds` ABI.
#[test]
fn cparam_getbounds_every_enumerator_and_every_invalid_id() {
    covers(&["CFG:4", "ERR:compress/zstd_compress.c:634"]);
    for &(name, id) in ALL_CPARAMS {
        diff(&format!("cParam_getBounds({name}={id})"), |l| {
            bounds_triple(l, cparam_bounds(l, id))
        });
    }
    for &id in INVALID_CPARAM_IDS {
        diff(&format!("cParam_getBounds(invalid {id})"), |l| {
            bounds_triple(l, cparam_bounds(l, id))
        });
    }
    // Randomised ids, filtered so we never accidentally hit a real enumerator.
    let valid: Vec<c_int> = ALL_CPARAMS.iter().map(|&(_, id)| id).collect();
    let mut rng = Rng::new(0x20_0001);
    for _ in 0..2000 {
        let id = rng.next_u32() as i32 % 4000;
        if valid.contains(&id) {
            continue;
        }
        diff(&format!("cParam_getBounds(rand {id})"), |l| {
            bounds_triple(l, cparam_bounds(l, id))
        });
    }
}

/// `ZSTD_dParam_getBounds` (`zstd_decompress.c:1821..1859`) for the 7
/// enumerators and every out-of-range id. Note the C uses a `switch` with
/// `default:;` and then falls *through* to the error assignment, which is a
/// different shape from the compressor's `default: return`.
#[test]
fn dparam_getbounds_every_enumerator_and_every_invalid_id() {
    covers(&["CFG:5", "ERR:decompress/zstd_decompress.c:1857"]);
    for &(name, id) in ALL_DPARAMS {
        diff(&format!("dParam_getBounds({name}={id})"), |l| {
            bounds_triple(l, dparam_bounds(l, id))
        });
    }
    for &id in INVALID_DPARAM_IDS {
        diff(&format!("dParam_getBounds(invalid {id})"), |l| {
            bounds_triple(l, dparam_bounds(l, id))
        });
    }
    let valid: Vec<c_int> = ALL_DPARAMS.iter().map(|&(_, id)| id).collect();
    let mut rng = Rng::new(0x20_0002);
    for _ in 0..2000 {
        let id = rng.next_u32() as i32 % 2000;
        if valid.contains(&id) {
            continue;
        }
        diff(&format!("dParam_getBounds(rand {id})"), |l| {
            bounds_triple(l, dparam_bounds(l, id))
        });
    }
}

// ===========================================================================
// 2. setParameter / getParameter value sweeps
// ===========================================================================

/// The core CLAMP-vs-REJECT test. For every one of the 39 `ZSTD_c_*`
/// parameters, `ZSTD_CCtx_setParameter` is driven with the bound neighbours, 0,
/// 1, a mid value, the `int` extremes and 40 random values, and the value is
/// then read back with `ZSTD_CCtx_getParameter`.
///
/// This is the only way to distinguish the three disciplines
/// `ZSTD_CCtxParams_setParameter` uses:
///   * hard `BOUNDCHECK` → `parameter_outOfBound` (42);
///   * `if (value != 0) BOUNDCHECK` → 0 is a legal "use default" sentinel that
///     is *stored as 0* and read back as 0;
///   * no check at all (`forceMaxWindow`, `enableDedicatedDictSearch`,
///     `contentSizeFlag`, `checksumFlag`, `dictIDFlag`) → the C stores
///     `value != 0`, so 7 and -3 both read back as 1;
///   * plus the two one-offs: `compressionLevel` is *clamped* into
///     `[minCLevel, maxCLevel]` and never fails, and `targetCBlockSize` raises
///     `1..1339` to `ZSTD_TARGETCBLOCKSIZE_MIN` before checking.
///
/// Targets `zstd_compress.c:715` (stage), `:765` (unknown param) and every
/// per-parameter `BOUNDCHECK` at `:777`..`:1019`.
#[test]
fn cctx_setparameter_value_sweep_and_readback() {
    covers(&[
        "CFG:12-14",
        "CFG:106,CFG:107,CFG:110,CFG:111,CFG:113",
        "ERR:compress/zstd_compress.c:765,ERR:compress/zstd_compress.c:777",
        "ERR:compress/zstd_compress.c:782,ERR:compress/zstd_compress.c:793",
        "ERR:compress/zstd_compress.c:799,ERR:compress/zstd_compress.c:805",
        "ERR:compress/zstd_compress.c:811,ERR:compress/zstd_compress.c:817",
        "ERR:compress/zstd_compress.c:822,ERR:compress/zstd_compress.c:828",
        "ERR:compress/zstd_compress.c:854,ERR:compress/zstd_compress.c:861",
        "ERR:compress/zstd_compress.c:868,ERR:compress/zstd_compress.c:878",
        "ERR:compress/zstd_compress.c:892,ERR:compress/zstd_compress.c:902",
        "ERR:compress/zstd_compress.c:915,ERR:compress/zstd_compress.c:921",
        "ERR:compress/zstd_compress.c:927,ERR:compress/zstd_compress.c:933",
        "ERR:compress/zstd_compress.c:939,ERR:compress/zstd_compress.c:946",
        "ERR:compress/zstd_compress.c:953,ERR:compress/zstd_compress.c:958",
        "ERR:compress/zstd_compress.c:963,ERR:compress/zstd_compress.c:968",
        "ERR:compress/zstd_compress.c:973,ERR:compress/zstd_compress.c:978",
        "ERR:compress/zstd_compress.c:983,ERR:compress/zstd_compress.c:988",
        "ERR:compress/zstd_compress.c:993,ERR:compress/zstd_compress.c:998",
        "ERR:compress/zstd_compress.c:1003,ERR:compress/zstd_compress.c:1009",
        "ERR:compress/zstd_compress.c:1015,ERR:compress/zstd_compress.c:1019",
        "ERR:compress/zstd_compress.c:1086,ERR:compress/zstd_compress.c:1094",
        "ERR:compress/zstd_compress.c:1101",
    ]);
    let b = &pair().c;
    for (i, &(name, id)) in ALL_CPARAMS.iter().enumerate() {
        let bounds = cparam_bounds(b, id);
        assert_eq!(bounds.error, 0, "{name} has no bounds?");
        for v in value_set(bounds.lowerBound, bounds.upperBound, 0x2010_0000 + i as u64) {
            diff(&format!("CCtx_setParameter({name}={id}, {v})"), |l| {
                cctx_set_then_get(l, id, v)
            });
        }
    }
}

/// Identical sweep against a standalone `ZSTD_CCtx_params`. The value-checking
/// code is shared with `ZSTD_CCtx_setParameter`, but the CCtx wrapper adds a
/// stage check and an `nbWorkers`-on-static guard *before* it, so running both
/// sweeps pins which layer each rejection comes from — and it exercises
/// `ZSTD_CCtxParams_getParameter` directly, including the three
/// `parameter_unsupported` reads (`jobSize`, `overlapLog`, `rsyncable`) that
/// exist only because `ZSTD_MULTITHREAD` is undefined.
#[test]
fn cctxparams_setparameter_value_sweep_and_readback() {
    covers(&[
        "CFG:102,CFG:106,CFG:107,CFG:108,CFG:109,CFG:110,CFG:111",
        "ERR:compress/zstd_compress.c:1019,ERR:compress/zstd_compress.c:1166",
        "ERR:compress/zstd_compress.c:1086,ERR:compress/zstd_compress.c:1094",
        "ERR:compress/zstd_compress.c:1101",
    ]);
    let b = &pair().c;
    for (i, &(name, id)) in ALL_CPARAMS.iter().enumerate() {
        let bounds = cparam_bounds(b, id);
        for v in value_set(bounds.lowerBound, bounds.upperBound, 0x2020_0000 + i as u64) {
            diff(&format!("CCtxParams_setParameter({name}={id}, {v})"), |l| {
                cctxparams_set_then_get(l, id, v)
            });
        }
    }
}

/// The seven `ZSTD_d_*` parameters through `ZSTD_DCtx_setParameter` /
/// `ZSTD_DCtx_getParameter`, same value discipline. Two of them do *not*
/// round-trip and that asymmetry is the point:
///   * `ZSTD_d_windowLogMax` stores `1 << value` and the getter returns
///     `ZSTD_highbit32(maxWindowSize)`, and `value == 0` is rewritten to
///     `ZSTD_WINDOWLOG_LIMIT_DEFAULT (27)` *before* `CHECK_DBOUNDS`;
///   * `ZSTD_d_maxBlockSize` skips the bound check entirely when `value == 0`
///     and the getter reports the raw 0, not the resolved 131072.
#[test]
fn dctx_setparameter_value_sweep_and_readback() {
    covers(&[
        "CFG:311,CFG:320",
        "ERR:decompress/zstd_decompress.c:1874,ERR:decompress/zstd_decompress.c:1903",
        "ERR:decompress/zstd_decompress.c:1912,ERR:decompress/zstd_decompress.c:1916",
        "ERR:decompress/zstd_decompress.c:1920,ERR:decompress/zstd_decompress.c:1924",
        "ERR:decompress/zstd_decompress.c:1928,ERR:decompress/zstd_decompress.c:1935",
        "ERR:decompress/zstd_decompress.c:1939,ERR:decompress/zstd_decompress.c:1944",
    ]);
    let b = &pair().c;
    for (i, &(name, id)) in ALL_DPARAMS.iter().enumerate() {
        let bounds = dparam_bounds(b, id);
        assert_eq!(bounds.error, 0, "d_{name} has no bounds?");
        for v in value_set(bounds.lowerBound, bounds.upperBound, 0x2030_0000 + i as u64) {
            diff(&format!("DCtx_setParameter(d_{name}={id}, {v})"), |l| {
                dctx_set_then_get(l, id, v)
            });
        }
    }
}

/// Out-of-range *parameter ids* (not values) on every setter and getter. A C
/// `switch` over an enum has no notion of "invalid enumerator", so all of these
/// must reach `default: RETURN_ERROR(parameter_unsupported)` — and critically
/// the getters must **not** write through the `int*` on that path.
#[test]
fn invalid_parameter_ids_on_every_setter_and_getter() {
    covers(&[
        "ERR:compress/zstd_compress.c:765,ERR:compress/zstd_compress.c:1019",
        "ERR:compress/zstd_compress.c:1166,ERR:compress/zstd_compress.c:634",
        "ERR:decompress/zstd_decompress.c:1857,ERR:decompress/zstd_decompress.c:1903",
        "ERR:decompress/zstd_decompress.c:1944",
    ]);
    for &id in INVALID_CPARAM_IDS {
        for &v in &[0, 1, -1, c_int::MAX] {
            diff(&format!("CCtx_setParameter(bad id {id}, {v})"), |l| {
                cctx_set_then_get(l, id, v)
            });
            diff(&format!("CCtxParams_setParameter(bad id {id}, {v})"), |l| {
                cctxparams_set_then_get(l, id, v)
            });
        }
    }
    for &id in INVALID_DPARAM_IDS {
        for &v in &[0, 1, -1, c_int::MAX] {
            diff(&format!("DCtx_setParameter(bad id {id}, {v})"), |l| {
                dctx_set_then_get(l, id, v)
            });
        }
    }
}

/// A *rejected* `set` must leave every other parameter alone — the C returns
/// before assigning, so all 39 values must still read back at their defaults.
/// Also pins the default snapshot itself (39 + 7 values), which is what
/// `ZSTD_CCtxParams_init` / `ZSTD_DCtx_resetParameters` establish.
#[test]
fn rejected_set_leaves_the_whole_parameter_set_untouched() {
    covers(&["CFG:103,CFG:104,CFG:109,CFG:320"]);
    diff("default CCtx snapshot", |l| {
        let ctx = Ctx::cctx(l);
        snapshot_all(l, ctx.ptr, "ZSTD_CCtx_getParameter")
    });
    diff("default CCtxParams snapshot", |l| {
        let p = new_cctx_params(l);
        snapshot_all(l, p.ptr, "ZSTD_CCtxParams_getParameter")
    });
    diff("default DCtx snapshot", |l| {
        let ctx = Ctx::dctx(l);
        snapshot_all_dparams(l, ctx.ptr)
    });

    // One out-of-bounds value per parameter, then the full snapshot.
    let b = &pair().c;
    for &(name, id) in ALL_CPARAMS {
        let bounds = cparam_bounds(b, id);
        let bad = bounds.upperBound.saturating_add(1);
        diff(&format!("snapshot after bad set({name}={bad})"), |l| {
            let ctx = Ctx::cctx(l);
            let set = l.sym::<FnCCtxSetParameter>("ZSTD_CCtx_setParameter");
            let sr = res(l, unsafe { set(ctx.ptr, id, bad) });
            (sr, snapshot_all(l, ctx.ptr, "ZSTD_CCtx_getParameter"))
        });
    }
    for &(name, id) in ALL_DPARAMS {
        let bounds = dparam_bounds(b, id);
        let bad = bounds.upperBound.saturating_add(1);
        diff(&format!("dsnapshot after bad set(d_{name}={bad})"), |l| {
            let ctx = Ctx::dctx(l);
            let set = l.sym::<FnDCtxSetParameter>("ZSTD_DCtx_setParameter");
            let sr = res(l, unsafe { set(ctx.ptr, id, bad) });
            (sr, snapshot_all_dparams(l, ctx.ptr))
        });
    }
}

/// The parameters that consume a *nested* C enum, driven with every value from
/// `-2` to `upperBound + 3`. Each of these casts the `int` straight into an enum
/// type (`ZSTD_format_e`, `ZSTD_ParamSwitch_e`, `ZSTD_dictAttachPref_e`,
/// `ZSTD_SequenceFormat_e`, `ZSTD_bufferMode_e`, `ZSTD_strategy`,
/// `ZSTD_forceIgnoreChecksum_e`, `ZSTD_refMultipleDDicts_e`), so a Rust
/// translation using a real `enum` with `TryFrom` would reject values the C
/// happily stores, and one storing `value as u32` would accept values the C
/// rejects. Both directions are visible here because the value is read back.
#[test]
fn nested_enum_parameters_accept_exactly_the_c_range() {
    covers(&[
        "CFG:106,CFG:107,CFG:311",
        "ERR:compress/zstd_compress.c:777,ERR:compress/zstd_compress.c:828",
        "ERR:compress/zstd_compress.c:854,ERR:compress/zstd_compress.c:861",
        "ERR:compress/zstd_compress.c:915,ERR:compress/zstd_compress.c:958",
        "ERR:compress/zstd_compress.c:963,ERR:compress/zstd_compress.c:968",
        "ERR:compress/zstd_compress.c:978,ERR:compress/zstd_compress.c:988",
        "ERR:compress/zstd_compress.c:998,ERR:compress/zstd_compress.c:1015",
        "ERR:decompress/zstd_decompress.c:1916,ERR:decompress/zstd_decompress.c:1924",
        "ERR:decompress/zstd_decompress.c:1928",
    ]);
    // (name, id, the enum's highest valid value)
    let enum_cparams: &[(&str, c_int, c_int)] = &[
        ("format/ZSTD_format_e", ZSTD_c_format, ZSTD_f_zstd1_magicless),
        ("strategy/ZSTD_strategy", ZSTD_c_strategy, ZSTD_btultra2),
        ("forceAttachDict/ZSTD_dictAttachPref_e", ZSTD_c_forceAttachDict, 3),
        (
            "literalCompressionMode/ZSTD_ParamSwitch_e",
            ZSTD_c_literalCompressionMode,
            ZSTD_ps_disable,
        ),
        (
            "enableLongDistanceMatching/ZSTD_ParamSwitch_e",
            ZSTD_c_enableLongDistanceMatching,
            ZSTD_ps_disable,
        ),
        ("stableInBuffer/ZSTD_bufferMode_e", ZSTD_c_stableInBuffer, 1),
        ("stableOutBuffer/ZSTD_bufferMode_e", ZSTD_c_stableOutBuffer, 1),
        (
            "blockDelimiters/ZSTD_SequenceFormat_e",
            ZSTD_c_blockDelimiters,
            ZSTD_sf_explicitBlockDelimiters,
        ),
        (
            "splitAfterSequences/ZSTD_ParamSwitch_e",
            ZSTD_c_splitAfterSequences,
            ZSTD_ps_disable,
        ),
        (
            "useRowMatchFinder/ZSTD_ParamSwitch_e",
            ZSTD_c_useRowMatchFinder,
            ZSTD_ps_disable,
        ),
        (
            "prefetchCDictTables/ZSTD_ParamSwitch_e",
            ZSTD_c_prefetchCDictTables,
            ZSTD_ps_disable,
        ),
        (
            "repcodeResolution/ZSTD_ParamSwitch_e",
            ZSTD_c_repcodeResolution,
            ZSTD_ps_disable,
        ),
    ];
    for &(label, id, hi) in enum_cparams {
        for v in -3..=(hi + 4) {
            diff(&format!("enum-range CCtx {label} = {v}"), |l| {
                cctx_set_then_get(l, id, v)
            });
            diff(&format!("enum-range CCtxParams {label} = {v}"), |l| {
                cctxparams_set_then_get(l, id, v)
            });
        }
    }
    let enum_dparams: &[(&str, c_int, c_int)] = &[
        ("d_format/ZSTD_format_e", ZSTD_d_format, ZSTD_f_zstd1_magicless),
        (
            "d_forceIgnoreChecksum/ZSTD_forceIgnoreChecksum_e",
            ZSTD_d_forceIgnoreChecksum,
            1,
        ),
        (
            "d_refMultipleDDicts/ZSTD_refMultipleDDicts_e",
            ZSTD_d_refMultipleDDicts,
            1,
        ),
        ("d_stableOutBuffer/ZSTD_bufferMode_e", ZSTD_d_stableOutBuffer, 1),
    ];
    for &(label, id, hi) in enum_dparams {
        for v in -3..=(hi + 4) {
            diff(&format!("enum-range DCtx {label} = {v}"), |l| {
                dctx_set_then_get(l, id, v)
            });
        }
    }
}

// ===========================================================================
// 3. stage checks
// ===========================================================================

/// `ZSTD_isUpdateAuthorized` (`zstd_compress.c:658`) whitelists exactly eight
/// parameters — compressionLevel, hashLog, chainLog, searchLog, minMatch,
/// targetLength, strategy and (the only experimental one)
/// blockSplitterLevel — as changeable while a frame is open. Everything else
/// must return `stage_wrong` (60) from `zstd_compress.c:715`.
///
/// Every parameter is driven in all six stages with an in-bounds value, so a
/// translation that got the whitelist wrong *in either direction* fails here,
/// and the read-back proves an authorized mid-stream set really took effect
/// while a refused one changed nothing. `Stage::MidThenResetParams` additionally
/// covers `zstd_compress.c:1376` (`ZSTD_reset_parameters` mid-frame →
/// `stage_wrong`, stage left alone) and `Stage::MidThenResetBoth` the
/// `streamStage = zcss_init` that happens *before* that check.
#[test]
fn stage_gate_for_every_parameter_in_every_stage() {
    covers(&[
        "CFG:68,CFG:113",
        "ERR:compress/zstd_compress.c:715,ERR:compress/zstd_compress.c:1376",
    ]);
    let src = corpus(Corpus::Text, 4096, 0x2040);
    let b = &pair().c;
    let cap = compress_bound(b, src.len()) + 256;
    for &(name, id) in ALL_CPARAMS {
        let bounds = cparam_bounds(b, id);
        // upperBound is always in range, so the *only* possible failure is the
        // stage gate (or the MT `parameter_unsupported` for nbWorkers etc.).
        let v = bounds.upperBound;
        for &st in ALL_STAGES {
            diff(&format!("stage {st:?}: set({name}={id}, {v})"), |l| {
                let mut out = vec![0u8; cap];
                let (ctx, pre) = cctx_in_stage(l, st, &src, &mut out);
                let set = l.sym::<FnCCtxSetParameter>("ZSTD_CCtx_setParameter");
                let get = l.sym::<FnCCtxGetParameter>("ZSTD_CCtx_getParameter");
                let sr = res(l, unsafe { set(ctx.ptr, id, v) });
                let mut got = SENTINEL;
                let gr = res(l, unsafe { get(ctx.ptr, id, &mut got) });
                (pre, sr, gr, got)
            });
        }
    }
}

/// `ZSTD_DCtx_setParameter` has an *unconditional*
/// `streamStage != zdss_init -> stage_wrong` as its very first statement
/// (`zstd_decompress.c:1908`): there is no `ZSTD_isUpdateAuthorized` equivalent
/// on the decode side, so **all seven** decoder parameters are frozen mid-frame.
/// `ZSTD_DCtx_getParameter` in contrast has no stage check at all. Also covers
/// `ZSTD_DCtx_reset(ZSTD_reset_parameters)` mid-frame
/// (`zstd_decompress.c:1957`) and `ZSTD_DCtx_setMaxWindowSize`'s stage guard
/// (`:1809`).
#[test]
fn dctx_parameters_are_frozen_mid_frame() {
    covers(&[
        "CFG:312,CFG:313",
        "ERR:decompress/zstd_decompress.c:1908,ERR:decompress/zstd_decompress.c:1957",
        "ERR:decompress/zstd_decompress.c:1809,ERR:decompress/zstd_decompress.c:1810",
        "ERR:decompress/zstd_decompress.c:1811",
    ]);
    let src = corpus(Corpus::Text, 200_000, 0x2041);
    let frame = c_compress(&src, 3);

    // Feed 8 bytes into a DStream: enough to leave zdss_init, not enough to
    // finish the frame.
    let start_partial = |l: &Lib, ctx: &Ctx| -> R {
        let f = l.sym::<FnDecompressStream>("ZSTD_decompressStream");
        let mut out = vec![0u8; 64];
        let mut i = ZSTD_inBuffer {
            src: frame.as_ptr() as *const c_void,
            size: 8,
            pos: 0,
        };
        let mut o = ZSTD_outBuffer {
            dst: out.as_mut_ptr() as *mut c_void,
            size: out.len(),
            pos: 0,
        };
        res(l, unsafe { f(ctx.ptr, &mut o, &mut i) })
    };

    for &(name, id) in ALL_DPARAMS {
        let hi = dparam_bounds(&pair().c, id).upperBound;
        for &v in &[0, 1, hi] {
            diff(&format!("mid-frame DCtx_setParameter(d_{name}, {v})"), |l| {
                let ctx = Ctx::dstream(l);
                let pre = start_partial(l, &ctx);
                let set = l.sym::<FnDCtxSetParameter>("ZSTD_DCtx_setParameter");
                let get = l.sym::<FnCCtxGetParameter>("ZSTD_DCtx_getParameter");
                let sr = res(l, unsafe { set(ctx.ptr, id, v) });
                let mut got = SENTINEL;
                let gr = res(l, unsafe { get(ctx.ptr, id, &mut got) });
                // session reset then retry: must now succeed
                let rr = dctx_reset(l, &ctx, ZSTD_reset_session_only);
                let sr2 = res(l, unsafe { set(ctx.ptr, id, v) });
                let mut got2 = SENTINEL;
                let gr2 = res(l, unsafe { get(ctx.ptr, id, &mut got2) });
                (pre, sr, gr, got, rr, sr2, gr2, got2)
            });
        }
    }

    // ZSTD_DCtx_reset(ZSTD_reset_parameters) mid-frame must fail and leave the
    // parameters (which we deliberately made non-default) in place.
    diff("mid-frame DCtx_reset(parameters)", |l| {
        let ctx = Ctx::dstream(l);
        let set = l.sym::<FnDCtxSetParameter>("ZSTD_DCtx_setParameter");
        let s = res(l, unsafe { set(ctx.ptr, ZSTD_d_windowLogMax, 20) });
        let pre = start_partial(l, &ctx);
        let r2 = dctx_reset(l, &ctx, ZSTD_reset_parameters);
        let snap = snapshot_all_dparams(l, ctx.ptr);
        let r3 = dctx_reset(l, &ctx, ZSTD_reset_session_and_parameters);
        let snap2 = snapshot_all_dparams(l, ctx.ptr);
        (s, pre, r2, snap, r3, snap2)
    });

    // ZSTD_DCtx_setMaxWindowSize: stage gate + the two parameter_outOfBound
    // arms, and the fact that a non-power-of-two is accepted but does not
    // round-trip through ZSTD_DCtx_getParameter(ZSTD_d_windowLogMax).
    let sizes: &[usize] = &[0, 1, 1023, 1024, 1025, 4096, (1 << 27) + 1, 1 << 31];
    for &ws in sizes {
        diff(&format!("DCtx_setMaxWindowSize({ws})"), |l| {
            let ctx = Ctx::dstream(l);
            let g = l.sym::<FnSetMaxWindowSize>("ZSTD_DCtx_setMaxWindowSize");
            let sr = res(l, unsafe { g(ctx.ptr, ws) });
            let get = l.sym::<FnCCtxGetParameter>("ZSTD_DCtx_getParameter");
            let mut got = SENTINEL;
            let gr = res(l, unsafe { get(ctx.ptr, ZSTD_d_windowLogMax, &mut got) });
            let pre = start_partial(l, &ctx);
            let sr2 = res(l, unsafe { g(ctx.ptr, ws.max(1024)) });
            (sr, gr, got, pre, sr2)
        });
    }
    // (1<<31)+1 exceeds `1 << ZSTD_WINDOWLOG_MAX` and must be rejected.
    diff("DCtx_setMaxWindowSize((1<<31)+1)", |l| {
        let ctx = Ctx::dstream(l);
        let g = l.sym::<FnSetMaxWindowSize>("ZSTD_DCtx_setMaxWindowSize");
        res(l, unsafe { g(ctx.ptr, (1usize << 31) + 1) })
    });
}

/// `ZSTD_CCtx_reset` / `ZSTD_DCtx_reset` with directives outside `{1,2,3}`.
/// Both functions test `reset ==` against the three enumerators, so 0, 4, -1 and
/// `INT_MAX` are **silent no-ops that return 0** — they neither reset nor error.
/// A `match` in Rust that falls into an error or panic arm would diverge here.
/// The snapshot after the call proves nothing was reset.
#[test]
fn out_of_range_reset_directives_are_silent_no_ops() {
    covers(&["CFG:68", "ERR:compress/zstd_compress.c:1376"]);
    let src = corpus(Corpus::Text, 4096, 0x2042);
    let cap = compress_bound(&pair().c, src.len()) + 256;
    let directives: Vec<c_int> = INVALID_RESET_DIRECTIVES
        .iter()
        .copied()
        .chain([ZSTD_reset_session_only, ZSTD_reset_parameters, ZSTD_reset_session_and_parameters])
        .collect();

    for &d in &directives {
        // (a) fresh cctx with a non-default parameter set
        diff(&format!("CCtx_reset({d}) on fresh ctx"), |l| {
            let ctx = Ctx::cctx(l);
            let set = l.sym::<FnCCtxSetParameter>("ZSTD_CCtx_setParameter");
            let s = res(l, unsafe { set(ctx.ptr, ZSTD_c_windowLog, 25) });
            let r = cctx_reset(l, &ctx, d);
            (s, r, snapshot_all(l, ctx.ptr, "ZSTD_CCtx_getParameter"))
        });
        // (b) mid-frame — this is where ZSTD_reset_parameters must fail
        diff(&format!("CCtx_reset({d}) mid-frame"), |l| {
            let mut out = vec![0u8; cap];
            let ctx = Ctx::cctx(l);
            let set = l.sym::<FnCCtxSetParameter>("ZSTD_CCtx_setParameter");
            let s = res(l, unsafe { set(ctx.ptr, ZSTD_c_checksumFlag, 1) });
            let pre = stream_continue(l, &ctx, &src, &mut out[..64]).0;
            let r = cctx_reset(l, &ctx, d);
            // whether the session was reset is visible in whether e_end can
            // still finish the frame
            let (er, n) = stream_end(l, &ctx, &src, &mut out);
            (s, pre, r, er, n)
        });
        diff(&format!("DCtx_reset({d}) on fresh ctx"), |l| {
            let ctx = Ctx::dctx(l);
            let set = l.sym::<FnDCtxSetParameter>("ZSTD_DCtx_setParameter");
            let s = res(l, unsafe { set(ctx.ptr, ZSTD_d_windowLogMax, 20) });
            let r = dctx_reset(l, &ctx, d);
            (s, r, snapshot_all_dparams(l, ctx.ptr))
        });
    }
}

/// Parameters are *sticky*: `ZSTD_reset_session_only` must keep them,
/// `ZSTD_reset_parameters` and `ZSTD_reset_session_and_parameters` must restore
/// the documented defaults via `ZSTD_CCtxParams_reset` ->
/// `ZSTD_CCtxParams_init(params, ZSTD_CLEVEL_DEFAULT)`, which `memset`s the
/// whole struct and then re-establishes `compressionLevel=3` and
/// `fParams.contentSizeFlag=1`. Every one of the 39 values is compared after
/// each reset, so a translation that forgets a field (or resets one it
/// shouldn't) is caught.
#[test]
fn reset_directives_restore_exactly_the_documented_defaults() {
    covers(&["CFG:68,CFG:103,CFG:104"]);
    // Set a spread of non-default values first, covering every storage class in
    // ZSTD_CCtx_params (cParams, fParams, ldmParams, the plain ints and the
    // ZSTD_ParamSwitch_e enums).
    let setup: &[(c_int, c_int)] = &[
        (ZSTD_c_compressionLevel, 19),
        (ZSTD_c_windowLog, 25),
        (ZSTD_c_hashLog, 22),
        (ZSTD_c_chainLog, 23),
        (ZSTD_c_searchLog, 7),
        (ZSTD_c_minMatch, 5),
        (ZSTD_c_targetLength, 999),
        (ZSTD_c_strategy, ZSTD_btultra2),
        (ZSTD_c_targetCBlockSize, 2048),
        (ZSTD_c_enableLongDistanceMatching, ZSTD_ps_enable),
        (ZSTD_c_ldmHashLog, 20),
        (ZSTD_c_ldmMinMatch, 32),
        (ZSTD_c_ldmBucketSizeLog, 5),
        (ZSTD_c_ldmHashRateLog, 7),
        (ZSTD_c_contentSizeFlag, 0),
        (ZSTD_c_checksumFlag, 1),
        (ZSTD_c_dictIDFlag, 0),
        (ZSTD_c_format, ZSTD_f_zstd1_magicless),
        (ZSTD_c_forceMaxWindow, 1),
        (ZSTD_c_forceAttachDict, 2),
        (ZSTD_c_literalCompressionMode, ZSTD_lcm_uncompressed),
        (ZSTD_c_srcSizeHint, 123_456),
        (ZSTD_c_enableDedicatedDictSearch, 1),
        (ZSTD_c_stableInBuffer, 1),
        (ZSTD_c_stableOutBuffer, 1),
        (ZSTD_c_blockDelimiters, 1),
        (ZSTD_c_validateSequences, 1),
        (ZSTD_c_splitAfterSequences, ZSTD_ps_enable),
        (ZSTD_c_useRowMatchFinder, ZSTD_ps_disable),
        (ZSTD_c_deterministicRefPrefix, 1),
        (ZSTD_c_prefetchCDictTables, ZSTD_ps_enable),
        (ZSTD_c_enableSeqProducerFallback, 1),
        (ZSTD_c_maxBlockSize, 4096),
        (ZSTD_c_repcodeResolution, ZSTD_ps_enable),
        (ZSTD_c_blockSplitterLevel, 6),
    ];
    for &d in &[
        ZSTD_reset_session_only,
        ZSTD_reset_parameters,
        ZSTD_reset_session_and_parameters,
    ] {
        diff(&format!("sticky params across CCtx_reset({d})"), |l| {
            let ctx = Ctx::cctx(l);
            let set = l.sym::<FnCCtxSetParameter>("ZSTD_CCtx_setParameter");
            let mut rs = Vec::new();
            for &(id, v) in setup {
                rs.push(res(l, unsafe { set(ctx.ptr, id, v) }));
            }
            let before = snapshot_all(l, ctx.ptr, "ZSTD_CCtx_getParameter");
            let rr = cctx_reset(l, &ctx, d);
            let after = snapshot_all(l, ctx.ptr, "ZSTD_CCtx_getParameter");
            (rs, before, rr, after)
        });
    }
    // The same for a standalone params object via ZSTD_CCtxParams_reset, plus
    // ZSTD_CCtxParams_init over the whole interesting level range.
    diff("CCtxParams_reset restores defaults", |l| {
        let p = new_cctx_params(l);
        let set = l.sym::<FnCCtxSetParameter>("ZSTD_CCtxParams_setParameter");
        let mut rs = Vec::new();
        for &(id, v) in setup {
            rs.push(res(l, unsafe { set(p.ptr, id, v) }));
        }
        let before = snapshot_all(l, p.ptr, "ZSTD_CCtxParams_getParameter");
        let rr = res(l, unsafe {
            l.sym::<FnCCtxParamsReset>("ZSTD_CCtxParams_reset")(p.ptr)
        });
        let after = snapshot_all(l, p.ptr, "ZSTD_CCtxParams_getParameter");
        (rs, before, rr, after)
    });
    for lvl in [
        c_int::MIN,
        -131_073,
        -131_072,
        -131_071,
        -1000,
        -5,
        -1,
        0,
        1,
        3,
        19,
        22,
        23,
        c_int::MAX,
    ] {
        diff(&format!("CCtxParams_init({lvl})"), |l| {
            let p = new_cctx_params(l);
            let r = res(l, unsafe {
                l.sym::<FnCCtxParamsInit>("ZSTD_CCtxParams_init")(p.ptr, lvl)
            });
            (r, snapshot_all(l, p.ptr, "ZSTD_CCtxParams_getParameter"))
        });
    }
}

// ===========================================================================
// 4. CCtx_params <-> CCtx plumbing
// ===========================================================================

/// `ZSTD_CCtxParams_init_advanced` validates with `ZSTD_checkCParams` and then
/// runs the whole resolver cascade (`ZSTD_resolveRowMatchFinderMode`,
/// `resolveBlockSplitterMode`, `resolveEnableLdm`,
/// `resolveExternalSequenceValidation`, `resolveMaxBlockSize`,
/// `resolveExternalRepcodeSearch` — the last one keyed on
/// `compressionLevel == ZSTD_NO_CLEVEL`), so reading all 39 values back is the
/// only way to see whether every resolver ran. Also covers the NULL check at
/// `zstd_compress.c:397` and the bounds forward at `:398`.
#[test]
fn cctxparams_init_advanced_runs_every_resolver() {
    covers(&[
        "CFG:105",
        "ERR:compress/zstd_compress.c:359,ERR:compress/zstd_compress.c:397",
        "ERR:compress/zstd_compress.c:398",
    ]);
    let b = &pair().c;
    let getc = b.sym::<FnGetParams>("ZSTD_getParams");

    let mut cases: Vec<(String, ZSTD_parameters)> = Vec::new();
    for lvl in [-5, 0, 1, 3, 12, 19, 22] {
        for &hint in &[0u64, 1024, 1 << 20, CONTENTSIZE_UNKNOWN_U64] {
            for &ds in &[0usize, 4096] {
                let p = unsafe { getc(lvl, hint, ds) };
                cases.push((format!("getParams({lvl},{hint},{ds})"), p));
            }
        }
    }
    // an all-zero ZSTD_parameters (windowLog 0 is *not* accepted by
    // ZSTD_checkCParams, unlike setParameter's `0 == default`)
    cases.push(("all-zero".into(), ZSTD_parameters::default()));
    // out-of-range single fields → parameter_outOfBound, nothing written
    for (label, f) in [
        ("windowLog=40", 0usize),
        ("chainLog=31", 1),
        ("hashLog=31", 2),
        ("searchLog=0", 3),
        ("minMatch=8", 4),
        ("targetLength=131073", 5),
        ("strategy=10", 6),
    ] {
        let mut p = unsafe { getc(3, 1 << 20, 0) };
        match f {
            0 => p.cParams.windowLog = 40,
            1 => p.cParams.chainLog = 31,
            2 => p.cParams.hashLog = 31,
            3 => p.cParams.searchLog = 0,
            4 => p.cParams.minMatch = 8,
            5 => p.cParams.targetLength = 131_073,
            _ => p.cParams.strategy = 10,
        }
        cases.push((format!("bad {label}"), p));
    }
    for (label, params) in &cases {
        diff(&format!("CCtxParams_init_advanced({label})"), |l| {
            let p = new_cctx_params(l);
            let r = res(l, unsafe {
                l.sym::<FnCCtxParamsInitAdvanced>("ZSTD_CCtxParams_init_advanced")(p.ptr, *params)
            });
            (r, snapshot_all(l, p.ptr, "ZSTD_CCtxParams_getParameter"))
        });
    }
}

/// `ZSTD_CCtx_setParametersUsingCCtxParams` is a plain struct copy
/// (`cctx->requestedParams = *params`) guarded by two `stage_wrong` checks
/// (`zstd_compress.c:1182` streamStage, `:1184` cdict attached). This is the
/// round-trip test: build a params object, set N random parameters on it, apply
/// it to a CCtx, then read **all 39** values back from both objects and compare.
/// A field the Rust struct copy misses (or aliases) shows up as a single
/// mismatched entry.
#[test]
fn cctxparams_apply_to_cctx_roundtrips_all_39_values() {
    covers(&[
        "CFG:112,CFG:102",
        "ERR:compress/zstd_compress.c:1182,ERR:compress/zstd_compress.c:1184",
    ]);
    let b = &pair().c;
    let bounds: Vec<(c_int, ZSTD_bounds)> =
        ALL_CPARAMS.iter().map(|&(_, id)| (id, cparam_bounds(b, id))) .collect();

    let src = corpus(Corpus::Text, 4096, 0x2050);
    let cap = compress_bound(b, src.len()) + 256;

    let mut rng = Rng::new(0x2050_0001);
    for round in 0..200 {
        // pick a random in-bounds assignment for a random subset
        let mut chosen: Vec<(c_int, c_int)> = Vec::new();
        for &(id, bd) in &bounds {
            if rng.below(3) == 0 {
                continue;
            }
            let v = rng.range(bd.lowerBound as i64, bd.upperBound as i64) as c_int;
            chosen.push((id, v));
        }
        // exercise both the fresh-CCtx (must succeed) and mid-frame (must
        // return stage_wrong and copy nothing) paths
        for midframe in [false, true] {
            diff(
                &format!("setParametersUsingCCtxParams round {round} midframe={midframe}"),
                |l| {
                    let p = new_cctx_params(l);
                    let sp = l.sym::<FnCCtxSetParameter>("ZSTD_CCtxParams_setParameter");
                    let mut sets = Vec::new();
                    for &(id, v) in &chosen {
                        sets.push(res(l, unsafe { sp(p.ptr, id, v) }));
                    }
                    let params_snap = snapshot_all(l, p.ptr, "ZSTD_CCtxParams_getParameter");

                    let mut out = vec![0u8; cap];
                    let ctx = Ctx::cctx(l);
                    let pre = if midframe {
                        stream_continue(l, &ctx, &src, &mut out[..64]).0
                    } else {
                        R::Ok(0)
                    };
                    let apply = res(l, unsafe {
                        l.sym::<FnSetParametersUsingCCtxParams>(
                            "ZSTD_CCtx_setParametersUsingCCtxParams",
                        )(ctx.ptr, p.ptr)
                    });
                    let cctx_snap = snapshot_all(l, ctx.ptr, "ZSTD_CCtx_getParameter");
                    (sets, params_snap, pre, apply, cctx_snap)
                },
            );
        }
    }
}

/// `ZSTD_CCtx_setCParams` / `setFParams` / `setParams`: thin wrappers that
/// forward to `ZSTD_CCtx_setParameter` field by field after one up-front
/// `ZSTD_checkCParams`, so a single bad field must leave **no** parameter
/// updated (`zstd_compress.c:1197`) while a stage violation surfaces from the
/// forwarded `windowLog` / `contentSizeFlag` set (`:1198`, `:1212`, `:1224`).
/// Note `setFParams` normalises with `!= 0` / `== 0`, so any non-zero flag is 1.
#[test]
fn set_cparams_fparams_params_are_all_or_nothing() {
    covers(&[
        "CFG:115",
        "ERR:compress/zstd_compress.c:1197,ERR:compress/zstd_compress.c:1198",
        "ERR:compress/zstd_compress.c:1212,ERR:compress/zstd_compress.c:1222",
        "ERR:compress/zstd_compress.c:1224",
    ]);
    let b = &pair().c;
    let getcp = b.sym::<FnGetCParams>("ZSTD_getCParams");
    let src = corpus(Corpus::Text, 4096, 0x2051);
    let cap = compress_bound(b, src.len()) + 256;

    let good = unsafe { getcp(19, 0, 0) };
    let mut cparam_cases: Vec<(String, ZSTD_compressionParameters)> =
        vec![("getCParams(19,0,0)".into(), good)];
    for lvl in [-1000, -1, 1, 3, 22] {
        cparam_cases.push((format!("getCParams({lvl},0,0)"), unsafe { getcp(lvl, 0, 0) }));
    }
    // one bad field at a time, and the all-zero struct
    let mutators: &[(&str, fn(&mut ZSTD_compressionParameters))] = &[
        ("windowLog=40", |c| c.windowLog = 40),
        ("windowLog=9", |c| c.windowLog = 9),
        ("chainLog=0", |c| c.chainLog = 0),
        ("hashLog=31", |c| c.hashLog = 31),
        ("searchLog=0", |c| c.searchLog = 0),
        ("minMatch=8", |c| c.minMatch = 8),
        ("minMatch=2", |c| c.minMatch = 2),
        ("targetLength=131073", |c| c.targetLength = 131_073),
        ("strategy=0", |c| c.strategy = 0),
        ("strategy=10", |c| c.strategy = 10),
        ("strategy=77", |c| c.strategy = 77),
        ("strategy=-1", |c| c.strategy = -1),
    ];
    for &(label, f) in mutators {
        let mut c = good;
        f(&mut c);
        cparam_cases.push((label.to_string(), c));
    }
    cparam_cases.push(("all-zero".into(), ZSTD_compressionParameters::default()));

    for (label, cp) in &cparam_cases {
        for midframe in [false, true] {
            diff(&format!("setCParams({label}) midframe={midframe}"), |l| {
                let mut out = vec![0u8; cap];
                let ctx = Ctx::cctx(l);
                let pre = if midframe {
                    stream_continue(l, &ctx, &src, &mut out[..64]).0
                } else {
                    R::Ok(0)
                };
                let r = res(l, unsafe {
                    l.sym::<FnSetCParams>("ZSTD_CCtx_setCParams")(ctx.ptr, *cp)
                });
                (pre, r, snapshot_all(l, ctx.ptr, "ZSTD_CCtx_getParameter"))
            });
            diff(&format!("setParams({label}) midframe={midframe}"), |l| {
                let mut out = vec![0u8; cap];
                let ctx = Ctx::cctx(l);
                let pre = if midframe {
                    stream_continue(l, &ctx, &src, &mut out[..64]).0
                } else {
                    R::Ok(0)
                };
                let params = ZSTD_parameters {
                    cParams: *cp,
                    fParams: ZSTD_frameParameters {
                        contentSizeFlag: 1,
                        checksumFlag: 1,
                        noDictIDFlag: 1,
                    },
                };
                let r = res(l, unsafe {
                    l.sym::<FnSetParams>("ZSTD_CCtx_setParams")(ctx.ptr, params)
                });
                (pre, r, snapshot_all(l, ctx.ptr, "ZSTD_CCtx_getParameter"))
            });
        }
    }

    // fParams: every combination, including non-0/1 values, which the wrapper
    // normalises with `!= 0` / `== 0`.
    for a in [-1, 0, 1, 7] {
        for bb in [-1, 0, 1, 7] {
            for c in [-1, 0, 1, 7] {
                for midframe in [false, true] {
                    diff(
                        &format!("setFParams({a},{bb},{c}) midframe={midframe}"),
                        |l| {
                            let mut out = vec![0u8; cap];
                            let ctx = Ctx::cctx(l);
                            let pre = if midframe {
                                stream_continue(l, &ctx, &src, &mut out[..64]).0
                            } else {
                                R::Ok(0)
                            };
                            let fp = ZSTD_frameParameters {
                                contentSizeFlag: a,
                                checksumFlag: bb,
                                noDictIDFlag: c,
                            };
                            let r = res(l, unsafe {
                                l.sym::<FnSetFParams>("ZSTD_CCtx_setFParams")(ctx.ptr, fp)
                            });
                            (pre, r, snapshot_all(l, ctx.ptr, "ZSTD_CCtx_getParameter"))
                        },
                    );
                }
            }
        }
    }
}

// ===========================================================================
// 5. pledged source size
// ===========================================================================

/// `ZSTD_CCtx_setPledgedSrcSize` stores `pledgedSrcSize + 1`, so
/// `ZSTD_CONTENTSIZE_UNKNOWN` (`(unsigned long long)-1`) *wraps to 0*, which is
/// the same encoding as "never pledged". `u64::MAX` and
/// `ZSTD_CONTENTSIZE_UNKNOWN` are therefore the same call, and a translation
/// using checked arithmetic would trap. The pledge is then enforced in two
/// separate places — too much input errors in
/// `ZSTD_compressContinue_internal`, too little in `ZSTD_compressEnd_public` —
/// and it changes the frame header, so the full output blob is compared.
/// Mid-frame the setter must return `stage_wrong` (`zstd_compress.c:1233`).
#[test]
fn pledged_src_size_before_and_mid_frame() {
    covers(&["CFG:45,CFG:66", "ERR:compress/zstd_compress.c:1233"]);
    let src = corpus(Corpus::Text, 4096, 0x2060);
    let cap = compress_bound(&pair().c, src.len()) + 256;
    let pledges: &[u64] = &[
        0,
        1,
        10,
        4095,
        4096,
        4097,
        131_071,
        131_072,
        131_073,
        1 << 32,
        CONTENTSIZE_UNKNOWN_U64 - 1,
        ZSTD_CONTENTSIZE_UNKNOWN,
        u64::MAX,
    ];
    for &p in pledges {
        diff_bytes(&format!("pledged {p} then full stream"), |l| {
            let mut out = vec![0xCDu8; cap];
            let ctx = Ctx::cctx(l);
            let sr = res(l, unsafe {
                l.sym::<FnSetPledgedSrcSize>("ZSTD_CCtx_setPledgedSrcSize")(ctx.ptr, p)
            });
            let (er, n) = stream_end(l, &ctx, &src, &mut out);
            out.truncate(n.min(out.len()));
            (sr, er, Blob(out))
        });
        diff(&format!("pledged {p} mid-frame"), |l| {
            let mut out = vec![0u8; cap];
            let ctx = Ctx::cctx(l);
            let pre = stream_continue(l, &ctx, &src, &mut out[..64]).0;
            let sr = res(l, unsafe {
                l.sym::<FnSetPledgedSrcSize>("ZSTD_CCtx_setPledgedSrcSize")(ctx.ptr, p)
            });
            // a session reset clears pledgedSrcSizePlusOne, so the retry must
            // now succeed
            let rr = cctx_reset(l, &ctx, ZSTD_reset_session_only);
            let sr2 = res(l, unsafe {
                l.sym::<FnSetPledgedSrcSize>("ZSTD_CCtx_setPledgedSrcSize")(ctx.ptr, p)
            });
            (pre, sr, rr, sr2)
        });
    }
    // pledge != actual, in both directions, driven to e_end
    for (pledge, feed) in [(100u64, 50usize), (100, 150), (0, 1), (1, 0), (4096, 4096)] {
        diff_bytes(&format!("pledge {pledge} feed {feed}"), |l| {
            let mut out = vec![0xCDu8; cap];
            let ctx = Ctx::cctx(l);
            let sr = res(l, unsafe {
                l.sym::<FnSetPledgedSrcSize>("ZSTD_CCtx_setPledgedSrcSize")(ctx.ptr, pledge)
            });
            let (er, n) = stream_end(l, &ctx, &src[..feed], &mut out);
            out.truncate(n.min(out.len()));
            (sr, er, Blob(out))
        });
    }
}

// ===========================================================================
// 6. getCParams / getParams
// ===========================================================================

/// `ZSTD_getCParams` over the *entire* legal compression-level range
/// (`ZSTD_minCLevel() ..= ZSTD_maxCLevel()+2`, i.e. -131072..=24) for four
/// (srcSizeHint, dictSize) pairs. All 131 097 x 4 results are packed into one
/// buffer and compared with `diff_bytes`, which reports the first differing
/// byte — index / 28 is the offending level. This is the only way to cover the
/// negative-level `targetLength = -MAX(ZSTD_minCLevel(), level)` acceleration
/// factor across its whole domain in reasonable time.
#[test]
fn getcparams_over_the_entire_level_range() {
    covers(&["CFG:151,CFG:152,CFG:153"]);
    let b = &pair().c;
    let lo = min_clevel(b);
    let hi = max_clevel(b) + 2;
    assert!(lo < 0 && hi > 0, "unexpected clevel range {lo}..{hi}");
    let combos: &[(u64, usize)] = &[
        (0, 0),
        (CONTENTSIZE_UNKNOWN_U64, 0),
        (CONTENTSIZE_UNKNOWN_U64, 4096),
        (1 << 20, 1 << 20),
    ];
    for &(hint, ds) in combos {
        diff_bytes(
            &format!("getCParams all levels, hint={hint} dict={ds}"),
            |l| {
                let f = l.sym::<FnGetCParams>("ZSTD_getCParams");
                let mut v: Vec<u8> = Vec::with_capacity(((hi - lo + 1) as usize) * 28);
                let mut lvl = lo;
                loop {
                    push_cparams(&mut v, &unsafe { f(lvl, hint, ds) });
                    if lvl == hi {
                        break;
                    }
                    lvl += 1;
                }
                Blob(v)
            },
        );
    }
}

/// The `ZSTD_getCParamRowSize` / `tableID` selection grid: sampled levels x 11
/// srcSizeHints x 6 dictSizes, one `diff` per cell so a failure names the exact
/// triple. The srcSizeHints straddle the 16 KB / 128 KB / 256 KB table
/// boundaries and the dictSizes straddle the `+500` fudge that
/// `ZSTD_getCParamRowSize` adds when srcSizeHint is `ZSTD_CONTENTSIZE_UNKNOWN`
/// — where `srcSizeHint + dictSize + 500` **wraps** because srcSizeHint is
/// `(U64)-1`, so dictSize 1 yields exactly 0 and lands in tableID 3.
#[test]
fn getcparams_and_getparams_grid() {
    covers(&["CFG:151,CFG:152,CFG:153"]);
    let b = &pair().c;
    let mut levels: Vec<c_int> = (-24..=24).collect();
    levels.extend_from_slice(&[
        min_clevel(b),
        min_clevel(b) + 1,
        -131_073,
        -100_000,
        -65_537,
        -65_536,
        -65_535,
        -1000,
        -513,
        -512,
        -511,
        -100,
        -50,
        -25,
        25,
        100,
        1000,
        c_int::MAX,
    ]);
    let mut rng = Rng::new(0x2070_0001);
    for _ in 0..200 {
        levels.push(rng.range(min_clevel(b) as i64, 30) as c_int);
    }
    levels.sort_unstable();
    levels.dedup();

    for &lvl in &levels {
        for &hint in SRC_HINTS {
            for &ds in DICT_SIZES {
                diff(&format!("getCParams({lvl},{hint},{ds})"), |l| unsafe {
                    l.sym::<FnGetCParams>("ZSTD_getCParams")(lvl, hint, ds)
                });
            }
        }
    }
    // ZSTD_getParams additionally memsets the whole struct and sets ONLY
    // fParams.contentSizeFlag = 1, so checksumFlag / noDictIDFlag must be 0.
    for &lvl in levels.iter().step_by(3) {
        for &hint in SRC_HINTS {
            for &ds in DICT_SIZES {
                diff(&format!("getParams({lvl},{hint},{ds})"), |l| unsafe {
                    l.sym::<FnGetParams>("ZSTD_getParams")(lvl, hint, ds)
                });
            }
        }
    }
}

// ===========================================================================
// 7. checkCParams / adjustCParams / cycleLog
// ===========================================================================

/// `ZSTD_checkCParams` is seven independent `BOUNDCHECK`s in a fixed order and
/// reports only the *first* failing field, so the order is observable: this test
/// walks each field to `min-1 / min / min+1 / max-1 / max / max+1` with the
/// others valid (isolating each check) and then randomises all seven at once
/// (exercising the ordering). No cross-field consistency is checked by the C —
/// `hashLog < chainLog` is accepted — which is easy to "improve" by accident.
#[test]
fn checkcparams_field_by_field_and_randomised() {
    covers(&[
        "CFG:154",
        "ERR:compress/zstd_compress.c:1390,ERR:compress/zstd_compress.c:1391",
        "ERR:compress/zstd_compress.c:1392,ERR:compress/zstd_compress.c:1393",
        "ERR:compress/zstd_compress.c:1394,ERR:compress/zstd_compress.c:1395",
        "ERR:compress/zstd_compress.c:1396",
    ]);
    let b = &pair().c;
    let base = unsafe { b.sym::<FnGetCParams>("ZSTD_getCParams")(3, 1 << 20, 0) };
    let fields: &[(&str, c_int)] = &[
        ("windowLog", ZSTD_c_windowLog),
        ("chainLog", ZSTD_c_chainLog),
        ("hashLog", ZSTD_c_hashLog),
        ("searchLog", ZSTD_c_searchLog),
        ("minMatch", ZSTD_c_minMatch),
        ("targetLength", ZSTD_c_targetLength),
        ("strategy", ZSTD_c_strategy),
    ];
    let put = |c: &mut ZSTD_compressionParameters, idx: usize, v: i64| match idx {
        0 => c.windowLog = v as c_uint,
        1 => c.chainLog = v as c_uint,
        2 => c.hashLog = v as c_uint,
        3 => c.searchLog = v as c_uint,
        4 => c.minMatch = v as c_uint,
        5 => c.targetLength = v as c_uint,
        _ => c.strategy = v as c_int,
    };
    for (i, &(name, id)) in fields.iter().enumerate() {
        let bd = cparam_bounds(b, id);
        for v in [
            bd.lowerBound as i64 - 1,
            bd.lowerBound as i64,
            bd.lowerBound as i64 + 1,
            0,
            bd.upperBound as i64 - 1,
            bd.upperBound as i64,
            bd.upperBound as i64 + 1,
            0xFFFF_FFFFi64,
        ] {
            let mut c = base;
            put(&mut c, i, v);
            diff(&format!("checkCParams({name}={v})"), |l| {
                res(l, unsafe {
                    l.sym::<FnCheckCParams>("ZSTD_checkCParams")(c)
                })
            });
        }
    }
    diff("checkCParams(all-zero)", |l| {
        res(l, unsafe {
            l.sym::<FnCheckCParams>("ZSTD_checkCParams")(ZSTD_compressionParameters::default())
        })
    });
    // hashLog < chainLog must still be accepted
    {
        let mut c = base;
        c.hashLog = 6;
        c.chainLog = 30;
        diff("checkCParams(hashLog<chainLog)", |l| {
            res(l, unsafe {
                l.sym::<FnCheckCParams>("ZSTD_checkCParams")(c)
            })
        });
    }
    // 600 fully random structs: all seven fields independently drawn from the
    // union of {0, bound-1, bound, bound+1, small, huge}
    let mut rng = Rng::new(0x2080_0001);
    for n in 0..4000 {
        let mut c = ZSTD_compressionParameters::default();
        for (i, &(_, id)) in fields.iter().enumerate() {
            let bd = cparam_bounds(b, id);
            let v = match rng.below(6) {
                0 => 0,
                1 => bd.lowerBound as i64 - 1,
                2 => bd.lowerBound as i64,
                3 => bd.upperBound as i64,
                4 => bd.upperBound as i64 + 1,
                _ => rng.range(0, 0xFFFF_FFFF),
            };
            put(&mut c, i, v);
        }
        diff(&format!("checkCParams(random {n}) {c:?}"), |l| {
            res(l, unsafe {
                l.sym::<FnCheckCParams>("ZSTD_checkCParams")(c)
            })
        });
    }
}

/// `ZSTD_adjustCParams` is documented as "never fails, wide contract": it runs
/// `ZSTD_clampCParams` first, so *any* struct is legal input. What it then does
/// is a chain of easy-to-mistranslate integer steps:
///   * the windowLog downsize only when `srcSize <= 1<<30 && dictSize <= 1<<30`,
///     with `srcLog = (tSize < 64) ? 6 : ZSTD_highbit32(tSize-1)+1` on the
///     **truncated** `(U32)(srcSize + dictSize)`;
///   * `ZSTD_dictAndWindowLog`'s three-way branch;
///   * `cPar.chainLog -= (cycleLog - dictAndWindowLog)`, an **unsigned
///     subtraction that can wrap**;
///   * the row-matchfinder hashLog clamp to `24 + BOUNDED(4, searchLog, 6)`,
///     which treats `ZSTD_ps_auto` as "row matchfinder enabled".
/// `srcSize == 0` is mapped to `ZSTD_CONTENTSIZE_UNKNOWN` by the public wrapper,
/// so 0 and UNKNOWN must agree.
#[test]
fn adjustcparams_randomised_sweep() {
    covers(&["CFG:155,CFG:156,CFG:157"]);
    let b = &pair().c;
    let getcp = b.sym::<FnGetCParams>("ZSTD_getCParams");
    let src_sizes: &[u64] = &[
        0,
        1,
        63,
        64,
        65,
        512,
        513,
        1 << 10,
        1 << 20,
        (1 << 30) - 1,
        1 << 30,
        (1 << 30) + 1,
        1 << 40,
        CONTENTSIZE_UNKNOWN_U64,
    ];
    let dict_sizes: &[usize] = &[0, 1, 1000, 1 << 20, 1 << 30, (1 << 30) + 1];

    // (a) a valid high-level cPar over the whole size grid
    let base = unsafe { getcp(19, 0, 0) };
    for &ss in src_sizes {
        for &ds in dict_sizes {
            diff(&format!("adjustCParams(lvl19, {ss}, {ds})"), |l| unsafe {
                l.sym::<FnAdjustCParams>("ZSTD_adjustCParams")(base, ss, ds)
            });
        }
    }
    // (b) the deliberately invalid struct from CONFIGS row 156 — every field
    // must come back clamped, not an error
    let bad = ZSTD_compressionParameters {
        windowLog: 40,
        chainLog: 0,
        hashLog: 99,
        searchLog: 0,
        minMatch: 1,
        targetLength: 1 << 20,
        strategy: 77,
    };
    for &ss in src_sizes {
        for &ds in dict_sizes {
            diff(&format!("adjustCParams(garbage, {ss}, {ds})"), |l| unsafe {
                l.sym::<FnAdjustCParams>("ZSTD_adjustCParams")(bad, ss, ds)
            });
        }
    }
    // (c) the row-hash clamp: greedy/lazy/lazy2 x searchLog 3..7 x hashLog 30
    for strat in [ZSTD_greedy, ZSTD_lazy, ZSTD_lazy2, ZSTD_btlazy2, ZSTD_btultra2] {
        for slog in 1..=10u32 {
            for hlog in [6u32, 24, 28, 29, 30] {
                let c = ZSTD_compressionParameters {
                    windowLog: 27,
                    chainLog: 27,
                    hashLog: hlog,
                    searchLog: slog,
                    minMatch: 4,
                    targetLength: 0,
                    strategy: strat,
                };
                diff(
                    &format!("adjustCParams(rowhash s={strat} sl={slog} hl={hlog})"),
                    |l| unsafe {
                        l.sym::<FnAdjustCParams>("ZSTD_adjustCParams")(c, 1 << 20, 0)
                    },
                );
            }
        }
    }
    // (d) 20000 fully random structs x random sizes, packed so one diff_bytes
    // localises the first divergence
    diff_bytes("adjustCParams randomised 20000", |l| {
        let f = l.sym::<FnAdjustCParams>("ZSTD_adjustCParams");
        let mut rng = Rng::new(0x2090_0001);
        let mut v = Vec::with_capacity(20000 * 28);
        for _ in 0..20000 {
            let c = ZSTD_compressionParameters {
                windowLog: rng.range(0, 40) as c_uint,
                chainLog: rng.range(0, 40) as c_uint,
                hashLog: rng.range(0, 40) as c_uint,
                searchLog: rng.range(0, 40) as c_uint,
                minMatch: rng.range(0, 10) as c_uint,
                targetLength: rng.range(0, 200_000) as c_uint,
                strategy: rng.range(-2, 12) as c_int,
            };
            let ss = match rng.below(3) {
                0 => rng.range(0, 1 << 31) as u64,
                1 => *rng.pick(&[0u64, 1, 1 << 30, (1 << 30) + 1, CONTENTSIZE_UNKNOWN_U64]),
                _ => rng.next_u64(),
            };
            let ds = match rng.below(2) {
                0 => rng.range(0, 1 << 31) as usize,
                _ => *rng.pick(&[0usize, 1, 1 << 30, (1 << 30) + 1]),
            };
            push_cparams(&mut v, &unsafe { f(c, ss, ds) });
        }
        Blob(v)
    });
}

/// `ZSTD_cycleLog(hashLog, strat)` is literally
/// `hashLog - (strat >= ZSTD_btlazy2)`. Two things must be reproduced exactly:
/// `hashLog == 0` with a bt strategy **underflows to 0xFFFFFFFF** (the C does
/// not clamp), and out-of-range strategy values (0, 10, negative) are compared
/// numerically rather than matched against enum variants.
#[test]
fn cycle_log_full_sweep_including_underflow() {
    covers(&["CFG:158"]);
    for hash_log in 0u32..=32 {
        for strat in -2..=12 {
            diff(&format!("cycleLog({hash_log}, {strat})"), |l| unsafe {
                l.sym::<FnCycleLog>("ZSTD_cycleLog")(hash_log, strat)
            });
        }
    }
    // a few large hashLogs, where the `-1` matters for the 32-bit wrap
    for hash_log in [u32::MAX, u32::MAX - 1, 1 << 31, (1 << 31) + 1] {
        for &strat in ALL_STRATEGIES {
            diff(&format!("cycleLog({hash_log}, {strat})"), |l| unsafe {
                l.sym::<FnCycleLog>("ZSTD_cycleLog")(hash_log, strat)
            });
        }
    }
}

// ===========================================================================
// 8. bounds / sizes
// ===========================================================================

/// `ZSTD_compressBound` has two arms in the `ZSTD_COMPRESSBOUND` macro (the
/// `srcSize < 128 KB` margin term `((128KB - srcSize) >> 11)` versus no margin)
/// plus the `r == 0 -> ERROR(srcSize_wrong)` guard for
/// `srcSize >= ZSTD_MAX_INPUT_SIZE (0xFF00FF00FF00FF00)`. The values below
/// straddle both the 128 KB margin boundary and the overflow boundary exactly.
#[test]
fn compress_bound_including_the_overflow_arm() {
    covers(&["CFG:2", "ERR:compress/zstd_compress.c:72"]);
    let mut sizes: Vec<usize> = SIZES.to_vec();
    sizes.extend_from_slice(&[
        1 << 20,
        (128 << 10) - 1,
        128 << 10,
        (128 << 10) + 1,
        0xFF00_FF00_FF00_FEFF,
        0xFF00_FF00_FF00_FEFF + 1,
        0xFF00_FF00_FF00_FF00,
        0xFF00_FF00_FF00_FF01,
        usize::MAX - 1,
        usize::MAX,
    ]);
    let mut rng = Rng::new(0x20A0_0001);
    for _ in 0..2000 {
        sizes.push(rng.next_u64() as usize);
        sizes.push(rng.range(0, 300_000) as usize);
    }
    sizes.sort_unstable();
    sizes.dedup();
    for &n in &sizes {
        diff(&format!("compressBound({n})"), |l| {
            res(l, compress_bound(l, n))
        });
    }
}

/// `ZSTD_decompressBound` walks frames and returns `ZSTD_CONTENTSIZE_ERROR`
/// (`(unsigned long long)-2`) on any bad frame. `(NULL, 0)` is *in contract*:
/// the `while (srcSize > 0)` loop body never runs, so nothing is dereferenced.
#[test]
fn decompress_bound_over_valid_and_invalid_frames() {
    covers(&["ERR:decompress/zstd_decompress.c:828"]);
    let src = corpus(Corpus::Text, 50_000, 0x20B0);
    let one = c_compress(&src, 3);
    let mut two = one.clone();
    two.extend_from_slice(&one);
    let mut cases: Vec<(String, Vec<u8>)> = vec![
        ("empty".into(), Vec::new()),
        ("one frame".into(), one.clone()),
        ("two frames".into(), two),
        ("truncated".into(), one[..one.len() / 2].to_vec()),
        ("garbage".into(), vec![0xAAu8; 32]),
        ("zeros".into(), vec![0u8; 8]),
    ];
    // trailing garbage after a valid frame
    let mut tail = one.clone();
    tail.extend_from_slice(&[0u8; 5]);
    cases.push(("frame + 5 zero bytes".into(), tail));
    for (label, buf) in &cases {
        diff(&format!("decompressBound({label})"), |l| unsafe {
            let f = l.sym::<FnDecompressBound>("ZSTD_decompressBound");
            f(buf.as_ptr() as *const c_void, buf.len())
        });
    }
    diff("decompressBound(NULL, 0)", |l| unsafe {
        l.sym::<FnDecompressBound>("ZSTD_decompressBound")(std::ptr::null(), 0)
    });
}

/// `ZSTD_sizeof_*` report `sizeof(*obj) + workspace + ...`, so they expose the
/// *actual Rust struct sizes* and the workspace layout to the test — the most
/// direct check that the translated structs did not grow or shrink. All six
/// return 0 for NULL by an explicit early return (no UB), and the values are
/// compared for a fresh object, after a one-shot compression at several levels
/// (which sizes the workspace from the cParams) and after a full streaming
/// session (which additionally allocates `buffIn`/`buffOut`).
#[test]
fn sizeof_contexts_null_fresh_and_after_use() {
    covers(&["CFG:7"]);
    for name in [
        "ZSTD_sizeof_CCtx",
        "ZSTD_sizeof_CStream",
        "ZSTD_sizeof_DCtx",
        "ZSTD_sizeof_DStream",
        "ZSTD_sizeof_CDict",
        "ZSTD_sizeof_DDict",
    ] {
        diff(&format!("{name}(NULL)"), |l| unsafe {
            l.sym::<FnSizeofOpaque>(name)(std::ptr::null())
        });
    }
    diff("sizeof_CCtx(fresh)", |l| {
        let c = Ctx::cctx(l);
        unsafe { l.sym::<FnSizeofOpaque>("ZSTD_sizeof_CCtx")(c.ptr) }
    });
    diff("sizeof_DCtx(fresh)", |l| {
        let c = Ctx::dctx(l);
        unsafe { l.sym::<FnSizeofOpaque>("ZSTD_sizeof_DCtx")(c.ptr) }
    });

    for &(clen, level) in &[(100usize, -5i32), (100, 1), (100, 3), (300_000, 9), (300_000, 19), (300_000, 22)] {
        let src = corpus(Corpus::Text, clen, 0x20C0 + clen as u64);
        diff(&format!("sizeof_CCtx after compressCCtx({clen},{level})"), |l| {
            let c = Ctx::cctx(l);
            let f = l.sym::<FnCompressCCtx>("ZSTD_compressCCtx");
            let cap = compress_bound(l, src.len());
            let mut dst = vec![0u8; cap];
            let r = res(l, unsafe {
                f(
                    c.ptr,
                    dst.as_mut_ptr() as *mut c_void,
                    cap,
                    src.as_ptr() as *const c_void,
                    src.len(),
                    level,
                )
            });
            let sz = unsafe { l.sym::<FnSizeofOpaque>("ZSTD_sizeof_CCtx")(c.ptr) };
            (r, sz)
        });
    }
    // streaming: buffIn/buffOut are allocated, so sizeof grows
    let src = corpus(Corpus::Text, 200_000, 0x20C1);
    diff("sizeof_CStream after a full session", |l| {
        let c = Ctx::cstream(l);
        let cap = compress_bound(l, src.len()) + 256;
        let mut out = vec![0u8; cap];
        let (r, _) = stream_end(l, &c, &src, &mut out);
        let sz = unsafe { l.sym::<FnSizeofOpaque>("ZSTD_sizeof_CStream")(c.ptr) };
        (r, sz)
    });
    let frame = c_compress(&src, 3);
    diff("sizeof_DStream after a full session", |l| {
        let c = Ctx::dstream(l);
        let f = l.sym::<FnDecompressStream>("ZSTD_decompressStream");
        let mut out = vec![0u8; src.len() + 64];
        let mut i = ZSTD_inBuffer {
            src: frame.as_ptr() as *const c_void,
            size: frame.len(),
            pos: 0,
        };
        let mut o = ZSTD_outBuffer {
            dst: out.as_mut_ptr() as *mut c_void,
            size: out.len(),
            pos: 0,
        };
        let r = res(l, unsafe { f(c.ptr, &mut o, &mut i) });
        let sz = unsafe { l.sym::<FnSizeofOpaque>("ZSTD_sizeof_DStream")(c.ptr) };
        (r, sz, o.pos)
    });

    // CDict / DDict: the sizes come from the same match-state layout the
    // estimators compute, so they cross-check ZSTD_estimateCDictSize against a
    // real object.
    let dict = corpus(Corpus::Text, 1 << 20, 0x20C2);
    for &ds in &[0usize, 8, 1024, 1 << 20] {
        for lvl in [1, 3, 19] {
            diff(&format!("sizeof_CDict(dict {ds}, lvl {lvl})"), |l| {
                let create = l
                    .sym::<unsafe extern "C" fn(*const c_void, SizeT, c_int) -> *mut c_void>(
                        "ZSTD_createCDict",
                    );
                let p = unsafe { create(dict.as_ptr() as *const c_void, ds, lvl) };
                let nonnull = !p.is_null();
                let sz = unsafe { l.sym::<FnSizeofOpaque>("ZSTD_sizeof_CDict")(p) };
                let est = unsafe {
                    l.sym::<FnEstimateCDictSize>("ZSTD_estimateCDictSize")(ds, lvl)
                };
                if nonnull {
                    unsafe { l.sym::<FnFreeCCtx>("ZSTD_freeCDict")(p) };
                }
                (nonnull, sz, res(l, est))
            });
        }
        diff(&format!("sizeof_DDict(dict {ds})"), |l| {
            let create =
                l.sym::<unsafe extern "C" fn(*const c_void, SizeT) -> *mut c_void>(
                    "ZSTD_createDDict",
                );
            let p = unsafe { create(dict.as_ptr() as *const c_void, ds) };
            let nonnull = !p.is_null();
            let sz = unsafe { l.sym::<FnSizeofOpaque>("ZSTD_sizeof_DDict")(p) };
            if nonnull {
                unsafe { l.sym::<FnFreeCCtx>("ZSTD_freeDDict")(p) };
            }
            (nonnull, sz)
        });
    }
    // CCtx after ZSTD_CCtx_loadDictionary: `ZSTD_sizeof_localDict` adds the
    // copied dictionary buffer, so the size must grow by the dict size.
    for &ds in &[1024usize, 1 << 20] {
        diff(&format!("sizeof_CCtx after loadDictionary({ds})"), |l| {
            let c = Ctx::cctx(l);
            let load = l
                .sym::<unsafe extern "C" fn(*mut c_void, *const c_void, SizeT) -> SizeT>(
                    "ZSTD_CCtx_loadDictionary",
                );
            let r = res(l, unsafe { load(c.ptr, dict.as_ptr() as *const c_void, ds) });
            let sz = unsafe { l.sym::<FnSizeofOpaque>("ZSTD_sizeof_CCtx")(c.ptr) };
            (r, sz)
        });
    }
    // DCtx after a one-shot decode vs after a streaming decode, for a small and a
    // large window: ZSTD_decodingBufferSize_internal sizes inBuff/outBuff from
    // the frame's windowSize, and the one-shot path allocates neither.
    for wlog in [10u32, 27] {
        let f = {
            let l = &pair().c;
            let ctx = Ctx::cctx(l);
            let set = l.sym::<FnCCtxSetParameter>("ZSTD_CCtx_setParameter");
            unsafe {
                set(ctx.ptr, ZSTD_c_windowLog, wlog as c_int);
            };
            let cap2 = compress_bound(l, src.len()) + 256;
            let mut dst = vec![0u8; cap2];
            let n = unsafe {
                l.sym::<FnCompress2>("ZSTD_compress2")(
                    ctx.ptr,
                    dst.as_mut_ptr() as *mut c_void,
                    cap2,
                    src.as_ptr() as *const c_void,
                    src.len(),
                )
            };
            assert!(!is_error(l, n));
            dst.truncate(n);
            dst
        };
        diff(&format!("sizeof_DCtx one-shot wlog {wlog}"), |l| {
            let c = Ctx::dctx(l);
            let mut out = vec![0u8; src.len() + 64];
            let n = unsafe {
                l.sym::<FnDecompressDCtx>("ZSTD_decompressDCtx")(
                    c.ptr,
                    out.as_mut_ptr() as *mut c_void,
                    out.len(),
                    f.as_ptr() as *const c_void,
                    f.len(),
                )
            };
            let sz = unsafe { l.sym::<FnSizeofOpaque>("ZSTD_sizeof_DCtx")(c.ptr) };
            (res(l, n), sz)
        });
        diff(&format!("sizeof_DCtx streaming wlog {wlog}"), |l| {
            let c = Ctx::dstream(l);
            let g = l.sym::<FnDecompressStream>("ZSTD_decompressStream");
            let mut out = vec![0u8; src.len() + 64];
            let mut i = ZSTD_inBuffer {
                src: f.as_ptr() as *const c_void,
                size: f.len(),
                pos: 0,
            };
            let mut o = ZSTD_outBuffer {
                dst: out.as_mut_ptr() as *mut c_void,
                size: out.len(),
                pos: 0,
            };
            let r = res(l, unsafe { g(c.ptr, &mut o, &mut i) });
            let sz = unsafe { l.sym::<FnSizeofOpaque>("ZSTD_sizeof_DCtx")(c.ptr) };
            let sz2 = unsafe { l.sym::<FnSizeofOpaque>("ZSTD_sizeof_DStream")(c.ptr) };
            (r, sz, sz2, o.pos)
        });
    }
}

/// The whole `ZSTD_estimate*Size*` family. These are pure functions of the
/// resolved cParams, so they are the cheapest possible probe of
/// `ZSTD_estimateCCtxSize_usingCCtxParams_internal`'s branch set — chainTable
/// present or not (absent for `ZSTD_fast` and for the row matchfinder),
/// hashLog3 (only when `minMatch == 3`), the LDM tables, the row-hash tag table,
/// and `buffInSize`/`buffOutSize` (streaming only). The level loop in
/// `ZSTD_estimateCCtxSize` (`for level = MIN(cl,1); level <= cl; level++`) is
/// covered including the degenerate cases: level 0 iterates exactly `{0}` and a
/// negative level iterates exactly that one level.
#[test]
fn estimate_sizes_over_levels_cparams_and_cctxparams() {
    covers(&["CFG:159,CFG:160,CFG:161,CFG:162,CFG:164,CFG:165"]);
    let b = &pair().c;
    let levels: &[c_int] = &[
        c_int::MIN,
        -131_073,
        -131_072,
        -100_000,
        -1000,
        -100,
        -5,
        -1,
        0,
        1,
        2,
        3,
        6,
        12,
        19,
        22,
        23,
        100,
    ];
    for &lvl in levels {
        diff(&format!("estimateCCtxSize({lvl})"), |l| {
            res(l, unsafe {
                l.sym::<FnEstimateFromLevel>("ZSTD_estimateCCtxSize")(lvl)
            })
        });
        diff(&format!("estimateCStreamSize({lvl})"), |l| {
            res(l, unsafe {
                l.sym::<FnEstimateFromLevel>("ZSTD_estimateCStreamSize")(lvl)
            })
        });
    }

    // _usingCParams over cParams from the level/size grid, plus hand-built
    // structs that hit every strategy and both minMatch classes. Input must be
    // a *valid* cParams (the C only asserts it), so everything here is filtered
    // through ZSTD_checkCParams first.
    let getcp = b.sym::<FnGetCParams>("ZSTD_getCParams");
    let check = b.sym::<FnCheckCParams>("ZSTD_checkCParams");
    let mut cps: Vec<(String, ZSTD_compressionParameters)> = Vec::new();
    for lvl in [-5, 1, 3, 5, 7, 12, 19, 22] {
        for &hint in &[0u64, 1 << 14, 1 << 17, 1 << 20, CONTENTSIZE_UNKNOWN_U64] {
            let c = unsafe { getcp(lvl, hint, 0) };
            cps.push((format!("getCParams({lvl},{hint},0)"), c));
        }
    }
    for &strat in ALL_STRATEGIES {
        for mm in [3u32, 4, 5, 6, 7] {
            for slog in [1u32, 4, 5, 6, 7] {
                let c = ZSTD_compressionParameters {
                    windowLog: 27,
                    chainLog: 26,
                    hashLog: 26,
                    searchLog: slog,
                    minMatch: mm,
                    targetLength: 64,
                    strategy: strat,
                };
                cps.push((format!("hand(s={strat},mm={mm},sl={slog})"), c));
            }
        }
    }
    for (label, c) in &cps {
        assert_eq!(unsafe { check(*c) }, 0, "{label} is not a valid cParams");
        diff(&format!("estimateCCtxSize_usingCParams({label})"), |l| {
            res(l, unsafe {
                l.sym::<FnEstimateFromCParams>("ZSTD_estimateCCtxSize_usingCParams")(*c)
            })
        });
        diff(&format!("estimateCStreamSize_usingCParams({label})"), |l| {
            res(l, unsafe {
                l.sym::<FnEstimateFromCParams>("ZSTD_estimateCStreamSize_usingCParams")(*c)
            })
        });
    }

    // _usingCCtxParams: drive the parameters that change the estimate.
    // ZSTD_estimateCStreamSize_usingCCtxParams deliberately resolves
    // useRowMatchFinder from `&params->cParams` (the *raw* request) rather than
    // the derived cParams, unlike the CCtx variant — an asymmetry worth pinning.
    let combos: &[&[(c_int, c_int)]] = &[
        &[],
        &[(ZSTD_c_compressionLevel, 1)],
        &[(ZSTD_c_compressionLevel, 19)],
        &[(ZSTD_c_maxBlockSize, 1024)],
        &[(ZSTD_c_maxBlockSize, 131_072)],
        &[(ZSTD_c_strategy, ZSTD_greedy), (ZSTD_c_useRowMatchFinder, ZSTD_ps_auto)],
        &[(ZSTD_c_strategy, ZSTD_greedy), (ZSTD_c_useRowMatchFinder, ZSTD_ps_enable)],
        &[(ZSTD_c_strategy, ZSTD_greedy), (ZSTD_c_useRowMatchFinder, ZSTD_ps_disable)],
        // NOTE: enabling LDM here *requires* also setting ZSTD_c_ldmMinMatch.
        // `ZSTD_estimateCCtxSize_usingCCtxParams` feeds `&params->ldmParams`
        // straight to `ZSTD_ldm_getMaxNbSeq`, which computes
        // `maxChunkSize / params.minMatchLength` (zstd_ldm.c:181) with **no**
        // zero guard, and the default `ldmParams.minMatchLength` is 0 — the
        // regular compression path only survives because
        // `ZSTD_ldm_adjustParameters` fills the defaults in first, and the
        // estimator never calls it. `{enableLdm=1, ldmMinMatch=0}` therefore
        // SIGFPEs in the reference C and is out of contract; see the exclusion
        // note in the test report.
        &[
            (ZSTD_c_enableLongDistanceMatching, ZSTD_ps_enable),
            (ZSTD_c_ldmMinMatch, 64),
        ],
        &[
            (ZSTD_c_enableLongDistanceMatching, ZSTD_ps_enable),
            (ZSTD_c_ldmMinMatch, 4),
            (ZSTD_c_ldmHashLog, 20),
            (ZSTD_c_ldmBucketSizeLog, 8),
            (ZSTD_c_ldmHashRateLog, 4),
        ],
        &[
            (ZSTD_c_enableLongDistanceMatching, ZSTD_ps_enable),
            (ZSTD_c_ldmMinMatch, 4096),
            (ZSTD_c_ldmHashLog, 6),
            (ZSTD_c_ldmBucketSizeLog, 1),
        ],
        &[
            (ZSTD_c_enableLongDistanceMatching, ZSTD_ps_disable),
            (ZSTD_c_ldmMinMatch, 0),
        ],
        &[(ZSTD_c_minMatch, 3)],
        &[(ZSTD_c_minMatch, 4)],
        &[(ZSTD_c_windowLog, 10)],
        &[(ZSTD_c_windowLog, 27)],
        &[(ZSTD_c_windowLog, 27), (ZSTD_c_maxBlockSize, 1024)],
        &[(ZSTD_c_stableInBuffer, 1)],
        &[(ZSTD_c_stableOutBuffer, 1)],
        &[(ZSTD_c_stableInBuffer, 1), (ZSTD_c_stableOutBuffer, 1)],
        &[(ZSTD_c_strategy, ZSTD_fast), (ZSTD_c_windowLog, 31)],
        &[(ZSTD_c_strategy, ZSTD_btultra2), (ZSTD_c_windowLog, 25)],
        &[(ZSTD_c_nbWorkers, 0)],
    ];
    for (i, combo) in combos.iter().enumerate() {
        diff(&format!("estimate*_usingCCtxParams combo {i} {combo:?}"), |l| {
            let p = new_cctx_params(l);
            let set = l.sym::<FnCCtxSetParameter>("ZSTD_CCtxParams_setParameter");
            let mut rs = Vec::new();
            for &(id, v) in combo.iter() {
                rs.push(res(l, unsafe { set(p.ptr, id, v) }));
            }
            let a = res(l, unsafe {
                l.sym::<FnEstimateFromCCtxParams>("ZSTD_estimateCCtxSize_usingCCtxParams")(p.ptr)
            });
            let bb = res(l, unsafe {
                l.sym::<FnEstimateFromCCtxParams>("ZSTD_estimateCStreamSize_usingCCtxParams")(p.ptr)
            });
            (rs, a, bb)
        });
    }

    // decoder side
    diff("estimateDCtxSize", |l| {
        res(l, unsafe {
            l.sym::<FnEstimateVoid>("ZSTD_estimateDCtxSize")()
        })
    });
    let window_sizes: &[usize] = &[
        0,
        1,
        63,
        64,
        1024,
        131_071,
        131_072,
        131_073,
        1 << 20,
        1 << 27,
        1 << 31,
        1 << 40,
    ];
    for &ws in window_sizes {
        diff(&format!("estimateDStreamSize({ws})"), |l| {
            res(l, unsafe {
                l.sym::<FnEstimateFromSize>("ZSTD_estimateDStreamSize")(ws)
            })
        });
    }
    // _fromFrame: real frames, a truncated header, a skippable frame and
    // garbage. (NULL, 0) is in contract: ZSTD_getFrameHeader returns
    // "need more input" without touching src.
    let src = corpus(Corpus::Text, 100_000, 0x20D0);
    let f10 = {
        // windowLog 10 via ZSTD_compress2
        let l = &pair().c;
        let ctx = Ctx::cctx(l);
        let set = l.sym::<FnCCtxSetParameter>("ZSTD_CCtx_setParameter");
        unsafe {
            set(ctx.ptr, ZSTD_c_windowLog, 10);
        }
        let cap = compress_bound(l, src.len()) + 256;
        let mut dst = vec![0u8; cap];
        let n = unsafe {
            l.sym::<FnCompress2>("ZSTD_compress2")(
                ctx.ptr,
                dst.as_mut_ptr() as *mut c_void,
                cap,
                src.as_ptr() as *const c_void,
                src.len(),
            )
        };
        assert!(!is_error(l, n));
        dst.truncate(n);
        dst
    };
    let f27 = c_compress(&src, 19);
    let mut skippable = vec![0u8; 12];
    skippable[..4].copy_from_slice(&ZSTD_MAGIC_SKIPPABLE_START.to_le_bytes());
    skippable[4..8].copy_from_slice(&4u32.to_le_bytes());
    // Hand-built 6-byte frame headers: FHD 0x00 (fcsFlag 0, not singleSegment,
    // no dictID, no checksum) followed by a Window_Descriptor byte encoding
    // `windowLog = 10 + (byte >> 3)` and mantissa `byte & 7`. 0xA8 gives exactly
    // `1 << 31` (the largest accepted), 0xA9 adds a mantissa so windowSize
    // exceeds `windowSizeMax`, and 0xB0 encodes windowLog 32.
    let hdr = |wd: u8| -> Vec<u8> {
        let mut v = Vec::with_capacity(6);
        v.extend_from_slice(&ZSTD_MAGICNUMBER.to_le_bytes());
        v.push(0x00);
        v.push(wd);
        v
    };
    let h_wl10 = hdr(0x00);
    let h_wl31 = hdr(0xA8);
    let h_wl31m = hdr(0xA9);
    let h_wl32 = hdr(0xB0);
    let h_wl41 = hdr(0xFF);
    let frames: &[(&str, &[u8])] = &[
        ("wlog10 frame", &f10),
        ("lvl19 frame", &f27),
        ("first 3 bytes", &f27[..3]),
        ("first 4 bytes", &f27[..4]),
        ("first 5 bytes", &f27[..5]),
        ("skippable", &skippable),
        ("garbage", &[0xAAu8, 0xAA, 0xAA, 0xAA, 0xAA]),
        ("hand header wlog10", &h_wl10),
        ("hand header wlog31", &h_wl31),
        ("hand header wlog31+mantissa", &h_wl31m),
        ("hand header wlog32", &h_wl32),
        ("hand header wd=0xFF", &h_wl41),
    ];
    for &(label, buf) in frames {
        diff(&format!("estimateDStreamSize_fromFrame({label})"), |l| {
            res(l, unsafe {
                l.sym::<FnEstimateFromFrame>("ZSTD_estimateDStreamSize_fromFrame")(
                    buf.as_ptr() as *const c_void,
                    buf.len(),
                )
            })
        });
    }
    diff("estimateDStreamSize_fromFrame(NULL,0)", |l| {
        res(l, unsafe {
            l.sym::<FnEstimateFromFrame>("ZSTD_estimateDStreamSize_fromFrame")(std::ptr::null(), 0)
        })
    });
}

// ===========================================================================
// 9. NULL contracts and the static-CCtx nbWorkers guard
// ===========================================================================

/// The NULL-pointer cases the C **actually checks**. Every one of these has an
/// explicit early `return` in the C source (verified by reading it), so the
/// behaviour is defined and the Rust must match it. Cases guarded only by an
/// `assert()` are excluded — see the module-level notes in the report; with
/// `DEBUGLEVEL=0` the assert is gone and the reference C segfaults, so there is
/// no C behaviour to match.
#[test]
fn null_pointer_contracts_that_the_c_really_checks() {
    covers(&["CFG:102", "ERR:compress/zstd_compress.c:359"]);
    // ZSTD_freeCCtxParams(NULL) -> `if (params == NULL) return 0;`
    diff("freeCCtxParams(NULL)", |l| {
        res(l, unsafe {
            l.sym::<FnCCtxParamsReset>("ZSTD_freeCCtxParams")(std::ptr::null_mut())
        })
    });
    // ZSTD_CCtxParams_init / _reset / _init_advanced all start with
    // `RETURN_ERROR_IF(!cctxParams, GENERIC, "NULL pointer!")`.
    for lvl in [c_int::MIN, -1, 0, 3, 22, c_int::MAX] {
        diff(&format!("CCtxParams_init(NULL, {lvl})"), |l| {
            res(l, unsafe {
                l.sym::<FnCCtxParamsInit>("ZSTD_CCtxParams_init")(std::ptr::null_mut(), lvl)
            })
        });
    }
    diff("CCtxParams_reset(NULL)", |l| {
        res(l, unsafe {
            l.sym::<FnCCtxParamsReset>("ZSTD_CCtxParams_reset")(std::ptr::null_mut())
        })
    });
    let params = unsafe { pair().c.sym::<FnGetParams>("ZSTD_getParams")(3, 0, 0) };
    diff("CCtxParams_init_advanced(NULL, params)", |l| {
        res(l, unsafe {
            l.sym::<FnCCtxParamsInitAdvanced>("ZSTD_CCtxParams_init_advanced")(
                std::ptr::null_mut(),
                params,
            )
        })
    });
    // and with out-of-range cParams, to prove the NULL check runs FIRST
    diff("CCtxParams_init_advanced(NULL, bad params)", |l| {
        let mut p = params;
        p.cParams.windowLog = 40;
        res(l, unsafe {
            l.sym::<FnCCtxParamsInitAdvanced>("ZSTD_CCtxParams_init_advanced")(
                std::ptr::null_mut(),
                p,
            )
        })
    });
    // create/free round-trip and a second free of NULL (CONFIGS row 102)
    diff("createCCtxParams then free then free(NULL)", |l| {
        let f = l.sym::<FnCreateCCtx>("ZSTD_createCCtxParams");
        let ptr = unsafe { f() };
        let nonnull = !ptr.is_null();
        let free = l.sym::<FnFreeCCtx>("ZSTD_freeCCtxParams");
        let a = res(l, unsafe { free(ptr) });
        let b = res(l, unsafe { free(std::ptr::null_mut()) });
        (nonnull, a, b)
    });
}

/// `ZSTD_CCtx_setParameter` special-cases `nbWorkers` *before* the generic
/// switch: `RETURN_ERROR_IF((value != 0) && cctx->staticSize,
/// parameter_unsupported)` at `zstd_compress.c:721`. Only a static CCtx has a
/// non-zero `staticSize`, so this branch is unreachable from a heap context and
/// needs `ZSTD_initStaticCCtx`. With `ZSTD_MULTITHREAD` undefined the generic
/// path at `:868` also rejects non-zero, so the interesting comparison is
/// value 0 (must succeed on both kinds of context) versus non-zero (must fail
/// on both, and both libraries must pick the *same* error).
#[test]
fn nbworkers_on_a_static_cctx() {
    covers(&["CFG:114", "ERR:compress/zstd_compress.c:721,ERR:compress/zstd_compress.c:868"]);
    let need = diff("estimateCCtxSize(3) for static buffer", |l| {
        res(l, unsafe {
            l.sym::<FnEstimateFromLevel>("ZSTD_estimateCCtxSize")(3)
        })
    });
    let need = match need {
        R::Ok(n) => n,
        e => panic!("estimateCCtxSize(3) failed: {e:?}"),
    };
    for &v in &[0, 1, 2, -1, c_int::MAX] {
        diff(&format!("static CCtx setParameter(nbWorkers, {v})"), |l| {
            // 8-byte aligned workspace, as ZSTD_initStaticCCtx requires
            let mut ws = vec![0u64; need / 8 + 2];
            let ptr = unsafe {
                l.sym::<FnInitStaticCCtx>("ZSTD_initStaticCCtx")(
                    ws.as_mut_ptr() as *mut c_void,
                    ws.len() * 8,
                )
            };
            assert!(!ptr.is_null(), "[{}] initStaticCCtx returned NULL", l.tag);
            let set = l.sym::<FnCCtxSetParameter>("ZSTD_CCtx_setParameter");
            let get = l.sym::<FnCCtxGetParameter>("ZSTD_CCtx_getParameter");
            let sr = res(l, unsafe { set(ptr, ZSTD_c_nbWorkers, v) });
            let mut got = SENTINEL;
            let gr = res(l, unsafe { get(ptr, ZSTD_c_nbWorkers, &mut got) });
            // ZSTD_freeCCtx on a static context returns memory_allocation and
            // must NOT free our Vec — compare that too.
            let fr = res(l, unsafe {
                l.sym::<FnFreeCCtx>("ZSTD_freeCCtx")(ptr)
            });
            (sr, gr, got, fr)
        });
    }
    // A heap CCtx must take the generic (non-static) path for the same values.
    for &v in &[0, 1, 2, -1, c_int::MAX] {
        diff(&format!("heap CCtx setParameter(nbWorkers, {v})"), |l| {
            cctx_set_then_get(l, ZSTD_c_nbWorkers, v)
        });
    }
}

// ===========================================================================
// 10. end-to-end: a parameter that is accepted must actually change the frame
// ===========================================================================

/// A parameter surface test that only checks return codes cannot tell "stored"
/// from "silently dropped". This test sets each parameter to a value that the C
/// accepts, compresses the same input through `ZSTD_compress2`, and compares the
/// resulting frame byte for byte — so a parameter the Rust accepts but never
/// applies shows up as a frame difference rather than passing silently.
#[test]
fn accepted_parameters_change_the_frame_identically() {
    covers(&["CFG:12,CFG:13,CFG:14,CFG:15"]);
    let b = &pair().c;
    let src = corpus(Corpus::Mixed, 150_000, 0x20E0);
    let cap = compress_bound(b, src.len()) + 1024;

    // For each parameter, a couple of accepted, behaviour-changing values, on a
    // level-3 base and on a level-19 base (CONFIGS row 15 asks for both: the
    // explicit cParam overrides interact with the level-derived defaults).
    let mut cases: Vec<(String, Vec<(c_int, c_int)>)> = Vec::new();
    for &(name, id) in ALL_CPARAMS {
        let bd = cparam_bounds(b, id);
        let mut vals = vec![bd.lowerBound, bd.upperBound, 0];
        if bd.upperBound as i64 - bd.lowerBound as i64 > 2 {
            vals.push(bd.lowerBound + 1);
        }
        vals.sort_unstable();
        vals.dedup();
        for v in vals {
            cases.push((format!("{name}={v}"), vec![(id, v)]));
            if id != ZSTD_c_compressionLevel {
                cases.push((
                    format!("lvl19+{name}={v}"),
                    vec![(ZSTD_c_compressionLevel, 19), (id, v)],
                ));
            }
        }
    }
    // Explicit cParam value sets from CONFIGS row 15, individually.
    let explicit: &[(c_int, &[c_int])] = &[
        (ZSTD_c_windowLog, &[0, 10, 11, 14, 15, 17, 20, 27, 31]),
        (ZSTD_c_hashLog, &[0, 6, 12, 25, 30]),
        (ZSTD_c_chainLog, &[0, 6, 16, 30]),
        (ZSTD_c_searchLog, &[0, 1, 4, 5, 6, 30]),
        (ZSTD_c_minMatch, &[0, 3, 4, 5, 6, 7]),
        (ZSTD_c_targetLength, &[0, 1, 64, 999, 131_072]),
        (ZSTD_c_strategy, &[0, 1, 9]),
    ];
    for &(id, vals) in explicit {
        for &v in vals {
            for base in [3, 19] {
                cases.push((
                    format!("lvl{base}+explicit {id}={v}"),
                    vec![(ZSTD_c_compressionLevel, base), (id, v)],
                ));
            }
        }
    }
    // CONFIGS row 14: the four multithreading parameters. Every non-zero value
    // is refused (`parameter_unsupported`) and the subsequent compression must be
    // byte-identical to the untouched default output.
    for &id in &[
        ZSTD_c_nbWorkers,
        ZSTD_c_jobSize,
        ZSTD_c_overlapLog,
        ZSTD_c_rsyncable,
    ] {
        for &v in &[0, 1, 2, -1, 9, 1_048_576] {
            cases.push((format!("MT param {id}={v}"), vec![(id, v)]));
        }
    }
    // a few multi-parameter combinations that interact
    cases.push((
        "ldm+wlog+strategy".into(),
        vec![
            (ZSTD_c_enableLongDistanceMatching, ZSTD_ps_enable),
            (ZSTD_c_windowLog, 27),
            (ZSTD_c_strategy, ZSTD_btopt),
            (ZSTD_c_ldmHashLog, 20),
            (ZSTD_c_ldmMinMatch, 32),
        ],
    ));
    cases.push((
        "magicless+checksum+nodictid".into(),
        vec![
            (ZSTD_c_format, ZSTD_f_zstd1_magicless),
            (ZSTD_c_checksumFlag, 1),
            (ZSTD_c_dictIDFlag, 0),
            (ZSTD_c_contentSizeFlag, 0),
        ],
    ));
    cases.push((
        "targetCBlockSize+splitAfterSequences".into(),
        vec![
            (ZSTD_c_targetCBlockSize, 2048),
            (ZSTD_c_splitAfterSequences, ZSTD_ps_enable),
            (ZSTD_c_blockSplitterLevel, 4),
        ],
    ));

    // The high compression levels are ~40x slower per byte, so they get a
    // smaller (but still >1 block after windowLog overrides) input. Keeping the
    // whole 150 KB multi-block corpus for the default level is what makes the
    // block-boundary interactions visible.
    let small = corpus(Corpus::Mixed, 24_000, 0x20E1);
    let cap_small = compress_bound(b, small.len()) + 1024;
    for (label, sets) in &cases {
        let heavy = sets
            .iter()
            .any(|&(id, v)| id == ZSTD_c_compressionLevel && v >= 12);
        let (input, dcap): (&[u8], usize) = if heavy {
            (&small, cap_small)
        } else {
            (&src, cap)
        };
        diff_bytes(&format!("compress2 with {label}"), |l| {
            let ctx = Ctx::cctx(l);
            let set = l.sym::<FnCCtxSetParameter>("ZSTD_CCtx_setParameter");
            let mut rs = Vec::new();
            for &(id, v) in sets {
                rs.push(res(l, unsafe { set(ctx.ptr, id, v) }));
            }
            let snap = snapshot_all(l, ctx.ptr, "ZSTD_CCtx_getParameter");
            let mut dst = vec![0xCDu8; dcap];
            let n = unsafe {
                l.sym::<FnCompress2>("ZSTD_compress2")(
                    ctx.ptr,
                    dst.as_mut_ptr() as *mut c_void,
                    dcap,
                    input.as_ptr() as *const c_void,
                    input.len(),
                )
            };
            let r = res(l, n);
            if let R::Ok(n) = r {
                dst.truncate(n);
            }
            ((rs, snap), r, Blob(dst))
        });
    }
}

// ===========================================================================
// 11. dense sweeps over the individual clamp / silent-raise disciplines
// ===========================================================================

/// `ZSTD_c_compressionLevel` is the *only* parameter that is CLAMPED instead of
/// rejected: `ZSTD_cParam_clampBounds` pulls the value into
/// `[ZSTD_minCLevel(), ZSTD_maxCLevel()]` and the function then returns the
/// clamped level as a `size_t` — but only when it is `>= 0`; a negative level
/// returns a plain `0` because "return type (size_t) cannot represent negative
/// values". `value == 0` is separately rewritten to `ZSTD_CLEVEL_DEFAULT`. A
/// translation that returned the error, or that returned the negative level
/// reinterpreted as a huge `size_t`, fails here. Dense sweep over both clamp
/// edges plus every level in between.
/// Targets `zstd_compress.c:782` and `:645` (`ZSTD_cParam_clampBounds`).
#[test]
fn compression_level_is_clamped_not_rejected() {
    covers(&[
        "CFG:104,CFG:106",
        "ERR:compress/zstd_compress.c:782",
    ]);
    let b = &pair().c;
    let lo = min_clevel(b);
    let hi = max_clevel(b);
    let mut vals: Vec<c_int> = Vec::new();
    for d in -4i64..=4 {
        vals.push((lo as i64 + d).clamp(c_int::MIN as i64, c_int::MAX as i64) as c_int);
        vals.push((hi as i64 + d).clamp(c_int::MIN as i64, c_int::MAX as i64) as c_int);
    }
    vals.extend(-40..=40);
    vals.extend_from_slice(&[c_int::MIN, c_int::MIN + 1, -1_000_000, 1_000_000, c_int::MAX]);
    vals.sort_unstable();
    vals.dedup();
    for v in vals {
        diff(&format!("compressionLevel clamp CCtx {v}"), |l| {
            cctx_set_then_get(l, ZSTD_c_compressionLevel, v)
        });
        diff(&format!("compressionLevel clamp CCtxParams {v}"), |l| {
            cctxparams_set_then_get(l, ZSTD_c_compressionLevel, v)
        });
    }
}

/// `ZSTD_c_targetCBlockSize` is the only parameter that *silently raises* its
/// input: `if (value != 0) { value = MAX(value, ZSTD_TARGETCBLOCKSIZE_MIN);
/// BOUNDCHECK(...); }`. So 0 stays 0, every value in `1..=1339` reads back as
/// 1340, `1340..=131072` round-trips, and `131073` upward is rejected. Dense
/// sweep over both discontinuities — a plain `BOUNDCHECK` translation would
/// reject 1..1339 instead of raising them.
/// Targets `zstd_compress.c:946`.
#[test]
fn target_cblock_size_silently_raises_small_values() {
    covers(&["CFG:106", "ERR:compress/zstd_compress.c:946"]);
    let mut vals: Vec<c_int> = (-4..=1400).collect();
    vals.extend(130_000..=131_080);
    vals.extend_from_slice(&[c_int::MIN, -131_072, 65_536, c_int::MAX]);
    vals.sort_unstable();
    vals.dedup();
    for v in vals {
        diff(&format!("targetCBlockSize CCtx {v}"), |l| {
            cctx_set_then_get(l, ZSTD_c_targetCBlockSize, v)
        });
        diff(&format!("targetCBlockSize CCtxParams {v}"), |l| {
            cctxparams_set_then_get(l, ZSTD_c_targetCBlockSize, v)
        });
    }
}

/// Dense integer sweeps across every remaining bound discontinuity, one
/// parameter at a time: this is a per-value transition table, so an off-by-one
/// in any `BOUNDCHECK` (or a missing `if (value != 0)` guard that would reject
/// the "use default" 0) is caught exactly at the boundary rather than
/// probabilistically. Ranges are chosen to include the boundary +/- 3 and the
/// whole small-integer neighbourhood where the `0 == default` sentinel lives.
#[test]
fn dense_boundary_sweeps_per_parameter() {
    covers(&[
        "CFG:106,CFG:110,CFG:111",
        "ERR:compress/zstd_compress.c:793,ERR:compress/zstd_compress.c:799",
        "ERR:compress/zstd_compress.c:805,ERR:compress/zstd_compress.c:811",
        "ERR:compress/zstd_compress.c:817,ERR:compress/zstd_compress.c:822",
        "ERR:compress/zstd_compress.c:828,ERR:compress/zstd_compress.c:921",
        "ERR:compress/zstd_compress.c:927,ERR:compress/zstd_compress.c:933",
        "ERR:compress/zstd_compress.c:939,ERR:compress/zstd_compress.c:953",
        "ERR:compress/zstd_compress.c:1009",
    ]);
    // (param, id, dense ranges to walk)
    let dense: &[(&str, c_int, &[(i64, i64)])] = &[
        ("windowLog", ZSTD_c_windowLog, &[(-4, 40)]),
        ("hashLog", ZSTD_c_hashLog, &[(-4, 40)]),
        ("chainLog", ZSTD_c_chainLog, &[(-4, 40)]),
        ("searchLog", ZSTD_c_searchLog, &[(-4, 40)]),
        ("minMatch", ZSTD_c_minMatch, &[(-4, 16)]),
        ("targetLength", ZSTD_c_targetLength, &[(-4, 20), (131_060, 131_080)]),
        ("strategy", ZSTD_c_strategy, &[(-4, 16)]),
        ("contentSizeFlag", ZSTD_c_contentSizeFlag, &[(-4, 8)]),
        ("checksumFlag", ZSTD_c_checksumFlag, &[(-4, 8)]),
        ("dictIDFlag", ZSTD_c_dictIDFlag, &[(-4, 8)]),
        ("nbWorkers", ZSTD_c_nbWorkers, &[(-4, 8)]),
        ("jobSize", ZSTD_c_jobSize, &[(-4, 8)]),
        ("overlapLog", ZSTD_c_overlapLog, &[(-12, 12)]),
        ("rsyncable", ZSTD_c_rsyncable, &[(-4, 8)]),
        ("format", ZSTD_c_format, &[(-4, 8)]),
        ("forceMaxWindow", ZSTD_c_forceMaxWindow, &[(-4, 8)]),
        ("forceAttachDict", ZSTD_c_forceAttachDict, &[(-4, 8)]),
        ("literalCompressionMode", ZSTD_c_literalCompressionMode, &[(-4, 8)]),
        (
            "enableLongDistanceMatching",
            ZSTD_c_enableLongDistanceMatching,
            &[(-4, 8)],
        ),
        ("ldmHashLog", ZSTD_c_ldmHashLog, &[(-4, 40)]),
        ("ldmMinMatch", ZSTD_c_ldmMinMatch, &[(-4, 12), (4090, 4100)]),
        ("ldmBucketSizeLog", ZSTD_c_ldmBucketSizeLog, &[(-4, 14)]),
        ("ldmHashRateLog", ZSTD_c_ldmHashRateLog, &[(-4, 32)]),
        ("srcSizeHint", ZSTD_c_srcSizeHint, &[(-8, 8)]),
        ("enableDedicatedDictSearch", ZSTD_c_enableDedicatedDictSearch, &[(-4, 8)]),
        ("stableInBuffer", ZSTD_c_stableInBuffer, &[(-4, 8)]),
        ("stableOutBuffer", ZSTD_c_stableOutBuffer, &[(-4, 8)]),
        ("blockDelimiters", ZSTD_c_blockDelimiters, &[(-4, 8)]),
        ("validateSequences", ZSTD_c_validateSequences, &[(-4, 8)]),
        ("splitAfterSequences", ZSTD_c_splitAfterSequences, &[(-4, 8)]),
        ("useRowMatchFinder", ZSTD_c_useRowMatchFinder, &[(-4, 8)]),
        ("deterministicRefPrefix", ZSTD_c_deterministicRefPrefix, &[(-4, 8)]),
        ("prefetchCDictTables", ZSTD_c_prefetchCDictTables, &[(-4, 8)]),
        ("enableSeqProducerFallback", ZSTD_c_enableSeqProducerFallback, &[(-4, 8)]),
        (
            "maxBlockSize",
            ZSTD_c_maxBlockSize,
            &[(-4, 8), (1018, 1032), (131_060, 131_080)],
        ),
        ("repcodeResolution", ZSTD_c_repcodeResolution, &[(-4, 8)]),
        ("blockSplitterLevel", ZSTD_c_blockSplitterLevel, &[(-4, 12)]),
    ];
    for &(name, id, ranges) in dense {
        for &(a, z) in ranges {
            for v in a..=z {
                let v = v as c_int;
                diff(&format!("dense CCtx {name}={v}"), |l| {
                    cctx_set_then_get(l, id, v)
                });
                diff(&format!("dense CCtxParams {name}={v}"), |l| {
                    cctxparams_set_then_get(l, id, v)
                });
            }
        }
    }
    // and the decoder side
    let dense_d: &[(&str, c_int, &[(i64, i64)])] = &[
        ("d_windowLogMax", ZSTD_d_windowLogMax, &[(-4, 40)]),
        ("d_format", ZSTD_d_format, &[(-4, 8)]),
        ("d_stableOutBuffer", ZSTD_d_stableOutBuffer, &[(-4, 8)]),
        ("d_forceIgnoreChecksum", ZSTD_d_forceIgnoreChecksum, &[(-4, 8)]),
        ("d_refMultipleDDicts", ZSTD_d_refMultipleDDicts, &[(-4, 8)]),
        ("d_disableHuffmanAssembly", ZSTD_d_disableHuffmanAssembly, &[(-4, 8)]),
        (
            "d_maxBlockSize",
            ZSTD_d_maxBlockSize,
            &[(-4, 8), (1018, 1032), (131_060, 131_080)],
        ),
    ];
    for &(name, id, ranges) in dense_d {
        for &(a, z) in ranges {
            for v in a..=z {
                let v = v as c_int;
                diff(&format!("dense DCtx {name}={v}"), |l| {
                    dctx_set_then_get(l, id, v)
                });
            }
        }
    }
}

/// Setting the same parameter twice must *overwrite*, and setting a bad value
/// after a good one must leave the good one in place. This distinguishes
/// "validate then store" from "store then validate", which the single-set sweeps
/// cannot see.
#[test]
fn second_set_overwrites_and_a_failing_second_set_preserves_the_first() {
    covers(&["CFG:106,CFG:113"]);
    let b = &pair().c;
    for &(name, id) in ALL_CPARAMS {
        let bd = cparam_bounds(b, id);
        let good = bd.upperBound;
        let alt = bd.lowerBound;
        let bad = bd.upperBound.saturating_add(1);
        for &(second, tag) in &[(alt, "alt"), (bad, "bad"), (0, "zero")] {
            diff(&format!("{name}: set {good} then {tag}={second}"), |l| {
                let ctx = Ctx::cctx(l);
                let set = l.sym::<FnCCtxSetParameter>("ZSTD_CCtx_setParameter");
                let get = l.sym::<FnCCtxGetParameter>("ZSTD_CCtx_getParameter");
                let r1 = res(l, unsafe { set(ctx.ptr, id, good) });
                let mut v1 = SENTINEL;
                let g1 = res(l, unsafe { get(ctx.ptr, id, &mut v1) });
                let r2 = res(l, unsafe { set(ctx.ptr, id, second) });
                let mut v2 = SENTINEL;
                let g2 = res(l, unsafe { get(ctx.ptr, id, &mut v2) });
                (r1, g1, v1, r2, g2, v2)
            });
        }
    }
}

// ===========================================================================
// 12. end-to-end consequences of the parameter surface
// ===========================================================================

/// An *authorized* mid-stream parameter change must actually take effect: the C
/// sets `cctx->cParamsChanged = 1` and re-derives the cParams at the next block.
/// This compresses the first half of the input, changes each of the eight
/// whitelisted parameters mid-frame, compresses the rest and compares the whole
/// frame byte for byte — so a translation that accepts the set but forgets the
/// `cParamsChanged` flag (or applies it too eagerly) produces a different frame.
#[test]
fn authorized_mid_stream_changes_alter_the_frame() {
    covers(&["CFG:68,CFG:113", "ERR:compress/zstd_compress.c:715"]);
    let src = corpus(Corpus::Mixed, 400_000, 0x20F0);
    let cap = compress_bound(&pair().c, src.len()) + 1024;
    // exactly ZSTD_isUpdateAuthorized's whitelist
    let authorized: &[(&str, c_int, &[c_int])] = &[
        ("compressionLevel", ZSTD_c_compressionLevel, &[1, 9, 19, -5, 0]),
        ("hashLog", ZSTD_c_hashLog, &[6, 16, 22, 0]),
        ("chainLog", ZSTD_c_chainLog, &[6, 16, 24, 0]),
        ("searchLog", ZSTD_c_searchLog, &[1, 4, 8, 0]),
        ("minMatch", ZSTD_c_minMatch, &[3, 4, 6, 7, 0]),
        ("targetLength", ZSTD_c_targetLength, &[0, 16, 999]),
        ("strategy", ZSTD_c_strategy, &[1, 3, 5, 7, 9, 0]),
        ("blockSplitterLevel", ZSTD_c_blockSplitterLevel, &[0, 1, 4, 6]),
    ];
    for &(name, id, vals) in authorized {
        for &v in vals {
            diff_bytes(&format!("mid-stream {name}={v} then finish"), |l| {
                let ctx = Ctx::cctx(l);
                let mut out = vec![0xCDu8; cap];
                // first half through e_continue
                let f = l.sym::<FnCompressStream2>("ZSTD_compressStream2");
                let half = src.len() / 2;
                let mut i = ZSTD_inBuffer {
                    src: src.as_ptr() as *const c_void,
                    size: half,
                    pos: 0,
                };
                let mut o = ZSTD_outBuffer {
                    dst: out.as_mut_ptr() as *mut c_void,
                    size: out.len(),
                    pos: 0,
                };
                let r1 = res(l, unsafe { f(ctx.ptr, &mut o, &mut i, ZSTD_e_continue) });
                let set = l.sym::<FnCCtxSetParameter>("ZSTD_CCtx_setParameter");
                let r2 = res(l, unsafe { set(ctx.ptr, id, v) });
                // rest through e_end
                let mut ipos = i.pos;
                let mut opos = o.pos;
                let mut r3 = R::Ok(0);
                for _ in 0..256 {
                    let mut i2 = ZSTD_inBuffer {
                        src: src.as_ptr() as *const c_void,
                        size: src.len(),
                        pos: ipos,
                    };
                    let mut o2 = ZSTD_outBuffer {
                        dst: out.as_mut_ptr() as *mut c_void,
                        size: out.len(),
                        pos: opos,
                    };
                    let n = unsafe { f(ctx.ptr, &mut o2, &mut i2, ZSTD_e_end) };
                    ipos = i2.pos;
                    opos = o2.pos;
                    r3 = res(l, n);
                    match r3 {
                        R::Ok(0) => break,
                        R::Ok(_) => {}
                        R::Err(..) => break,
                    }
                }
                out.truncate(opos.min(out.len()));
                ((r1, r2), r3, Blob(out))
            });
        }
    }
}

/// `ZSTD_CCtx_reset` end-to-end (CONFIGS row 68): a context that has been used
/// and then reset with `ZSTD_reset_session_and_parameters` must produce
/// byte-identical output to a brand-new context, while `ZSTD_reset_session_only`
/// must keep the parameters (and therefore produce the *parameterised* output).
/// The invalid directives 0 and 4 must change nothing at all.
#[test]
fn reset_then_recompress_matches_a_fresh_context() {
    covers(&["CFG:68,CFG:71"]);
    let src = corpus(Corpus::Text, 120_000, 0x20F1);
    let cap = compress_bound(&pair().c, src.len()) + 1024;
    let compress2 = |l: &Lib, ctx: &Ctx, cap: usize| -> (R, Blob) {
        let mut dst = vec![0xCDu8; cap];
        let n = unsafe {
            l.sym::<FnCompress2>("ZSTD_compress2")(
                ctx.ptr,
                dst.as_mut_ptr() as *mut c_void,
                cap,
                src.as_ptr() as *const c_void,
                src.len(),
            )
        };
        let r = res(l, n);
        if let R::Ok(n) = r {
            dst.truncate(n);
        }
        (r, Blob(dst))
    };
    for &d in &[
        0,
        ZSTD_reset_session_only,
        ZSTD_reset_parameters,
        ZSTD_reset_session_and_parameters,
        4,
    ] {
        diff_bytes(&format!("compress, reset({d}), recompress"), |l| {
            let ctx = Ctx::cctx(l);
            let set = l.sym::<FnCCtxSetParameter>("ZSTD_CCtx_setParameter");
            unsafe {
                set(ctx.ptr, ZSTD_c_compressionLevel, 7);
                set(ctx.ptr, ZSTD_c_checksumFlag, 1);
                set(ctx.ptr, ZSTD_c_windowLog, 18);
            };
            let (r1, _b1) = compress2(l, &ctx, cap);
            let rr = cctx_reset(l, &ctx, d);
            let (r2, b2) = compress2(l, &ctx, cap);
            ((r1, rr), r2, b2)
        });
    }
    // reference: a fresh context at defaults
    diff_bytes("fresh context, default params", |l| {
        let ctx = Ctx::cctx(l);
        let (r, b) = compress2(l, &ctx, cap);
        (r, b)
    });
    // CONFIGS row 68 also asks for the reset *after a completed streaming
    // frame*: the session is already back at zcss_init, so even
    // ZSTD_reset_parameters must succeed there (unlike mid-frame).
    for &d in &[
        0,
        ZSTD_reset_session_only,
        ZSTD_reset_parameters,
        ZSTD_reset_session_and_parameters,
        4,
    ] {
        diff_bytes(&format!("full frame, reset({d}), recompress"), |l| {
            let ctx = Ctx::cctx(l);
            let set = l.sym::<FnCCtxSetParameter>("ZSTD_CCtx_setParameter");
            unsafe {
                set(ctx.ptr, ZSTD_c_compressionLevel, 7);
                set(ctx.ptr, ZSTD_c_checksumFlag, 1);
            };
            let mut out = vec![0u8; cap];
            let (e1, _) = stream_end(l, &ctx, &src, &mut out);
            let rr = cctx_reset(l, &ctx, d);
            let snap = snapshot_all(l, ctx.ptr, "ZSTD_CCtx_getParameter");
            let (r2, b2) = compress2(l, &ctx, cap);
            ((e1, rr, snap), r2, b2)
        });
    }
    // CONFIGS row 71: ZSTD_compressCCtx / ZSTD_compress use a *separate*
    // `cctx->simpleApiParams`, re-initialised from the level on every call, so
    // the sticky `requestedParams` set above must not leak into them — and must
    // still apply to the following ZSTD_compress2 on the same context.
    for lvl in [1, 3, 19] {
        diff_bytes(&format!("compressCCtx({lvl}) then compress2 on same ctx"), |l| {
            let ctx = Ctx::cctx(l);
            let set = l.sym::<FnCCtxSetParameter>("ZSTD_CCtx_setParameter");
            unsafe {
                set(ctx.ptr, ZSTD_c_checksumFlag, 1);
                set(ctx.ptr, ZSTD_c_windowLog, 10);
                set(ctx.ptr, ZSTD_c_strategy, ZSTD_btultra2);
            };
            let mut dst = vec![0xCDu8; cap];
            let n = unsafe {
                l.sym::<FnCompressCCtx>("ZSTD_compressCCtx")(
                    ctx.ptr,
                    dst.as_mut_ptr() as *mut c_void,
                    cap,
                    src.as_ptr() as *const c_void,
                    src.len(),
                    lvl,
                )
            };
            let r1 = res(l, n);
            let simple = match r1 {
                R::Ok(n) => Blob(dst[..n].to_vec()),
                _ => Blob(Vec::new()),
            };
            let (r2, b2) = compress2(l, &ctx, cap);
            ((r1, simple, r2), fnv1a64(&b2.0), b2)
        });
    }
}

// ===========================================================================
// 13. dictionary-size estimators (pure functions of the parameter surface)
// ===========================================================================

/// `ZSTD_estimateCDictSize_advanced` = `sizeof(CDict)` + `HUF_WORKSPACE_SIZE` +
/// `ZSTD_sizeof_matchState(..., enableDedicatedDictSearch = 1, forCCtx = 0)` +
/// (`byRef ? 0 : align(dictSize)`), and `ZSTD_estimateDDictSize` is just
/// `sizeof(ZSTD_DDict) + (byRef ? 0 : dictSize)`. Both are pure size arithmetic
/// over the cParams, so they probe the match-state layout (chain table present or
/// not, hashLog3 only for `minMatch == 3`) without allocating anything. The
/// `dictLoadMethod` argument is an enum consumed by `int`, so out-of-range values
/// are swept too: the C compares `== ZSTD_dlm_byRef`, so anything else behaves
/// like `byCopy`.
#[test]
fn estimate_dict_sizes() {
    covers(&["CFG:163,CFG:166"]);
    let b = &pair().c;
    let getcp = b.sym::<FnGetCParams>("ZSTD_getCParams");
    let dict_sizes: &[usize] = &[0, 1, 7, 8, 9, 63, 64, 4096, 4097, 1 << 20];
    for &ds in dict_sizes {
        for lvl in [-5, 1, 3, 12, 19, 22] {
            diff(&format!("estimateCDictSize({ds}, {lvl})"), |l| {
                res(l, unsafe {
                    l.sym::<FnEstimateCDictSize>("ZSTD_estimateCDictSize")(ds, lvl)
                })
            });
        }
        // dictLoadMethod: the two enumerators plus out-of-range values
        for dlm in [-1, ZSTD_dlm_byCopy, ZSTD_dlm_byRef, 2, 7] {
            diff(&format!("estimateDDictSize({ds}, dlm={dlm})"), |l| {
                res(l, unsafe {
                    l.sym::<FnEstimateDDictSize>("ZSTD_estimateDDictSize")(ds, dlm)
                })
            });
            for lvl in [1, 3, 19] {
                for &strat in ALL_STRATEGIES {
                    let mut cp = unsafe { getcp(lvl, 0, ds) };
                    cp.strategy = strat;
                    diff(
                        &format!("estimateCDictSize_advanced({ds}, lvl{lvl}/s{strat}, dlm={dlm})"),
                        |l| {
                            res(l, unsafe {
                                l.sym::<FnEstimateCDictSizeAdvanced>(
                                    "ZSTD_estimateCDictSize_advanced",
                                )(ds, cp, dlm)
                            })
                        },
                    );
                }
            }
        }
    }
}

// ===========================================================================
// 14. getParams over the whole level range
// ===========================================================================

/// The `ZSTD_getParams` twin of `getcparams_over_the_entire_level_range`: the
/// full `ZSTD_minCLevel() ..= ZSTD_maxCLevel()+2` sweep, packed so `diff_bytes`
/// pinpoints the first differing level. `ZSTD_getParams_internal` memsets the
/// whole `ZSTD_parameters` and then sets **only** `fParams.contentSizeFlag = 1`,
/// so the 3 fParams fields are compared alongside the 7 cParams for every level.
#[test]
fn getparams_over_the_entire_level_range() {
    covers(&["CFG:153"]);
    let b = &pair().c;
    let lo = min_clevel(b);
    let hi = max_clevel(b) + 2;
    for &(hint, ds) in &[
        (0u64, 0usize),
        (CONTENTSIZE_UNKNOWN_U64, 1),
        (256 * 1024 + 1, 500),
    ] {
        diff_bytes(&format!("getParams all levels, hint={hint} dict={ds}"), |l| {
            let f = l.sym::<FnGetParams>("ZSTD_getParams");
            let mut v: Vec<u8> = Vec::with_capacity(((hi - lo + 1) as usize) * 40);
            let mut lvl = lo;
            loop {
                let p = unsafe { f(lvl, hint, ds) };
                push_cparams(&mut v, &p.cParams);
                v.extend_from_slice(&p.fParams.contentSizeFlag.to_le_bytes());
                v.extend_from_slice(&p.fParams.checksumFlag.to_le_bytes());
                v.extend_from_slice(&p.fParams.noDictIDFlag.to_le_bytes());
                if lvl == hi {
                    break;
                }
                lvl += 1;
            }
            Blob(v)
        });
    }
}

// ===========================================================================
// 15. remaining error sites on the parameter surface
// ===========================================================================

/// `ZSTD_CCtx_setParametersUsingCCtxParams` has a *second* `stage_wrong` guard
/// that has nothing to do with the stream stage: `RETURN_ERROR_IF(cctx->cdict,
/// stage_wrong, "Can't override parameters with cdict attached")` at
/// `zstd_compress.c:1184`. Reaching it needs a real CDict, so it is invisible to
/// every other test in this file. `ZSTD_CCtx_setParameter` itself is *not*
/// guarded this way, which is the asymmetry compared here.
#[test]
fn cdict_attached_blocks_setparametersusingcctxparams_only() {
    covers(&["CFG:112", "ERR:compress/zstd_compress.c:1184"]);
    let dict = corpus(Corpus::Text, 8192, 0x2100);
    for lvl in [1, 3, 19] {
        diff(&format!("refCDict(lvl{lvl}) then setParametersUsingCCtxParams"), |l| {
            let create = l
                .sym::<unsafe extern "C" fn(*const c_void, SizeT, c_int) -> *mut c_void>(
                    "ZSTD_createCDict",
                );
            let cd = unsafe { create(dict.as_ptr() as *const c_void, dict.len(), lvl) };
            assert!(!cd.is_null(), "[{}] createCDict returned NULL", l.tag);
            let cdict = Ctx::from_raw(l, cd, "ZSTD_freeCDict");
            let ctx = Ctx::cctx(l);
            let refc = res(l, unsafe {
                l.sym::<FnSetParametersUsingCCtxParams>("ZSTD_CCtx_refCDict")(ctx.ptr, cdict.ptr)
            });
            let p = new_cctx_params(l);
            let sp = l.sym::<FnCCtxSetParameter>("ZSTD_CCtxParams_setParameter");
            unsafe {
                sp(p.ptr, ZSTD_c_compressionLevel, 19);
                sp(p.ptr, ZSTD_c_format, ZSTD_f_zstd1_magicless);
                sp(p.ptr, ZSTD_c_maxBlockSize, 4096);
            };
            let apply = res(l, unsafe {
                l.sym::<FnSetParametersUsingCCtxParams>("ZSTD_CCtx_setParametersUsingCCtxParams")(
                    ctx.ptr, p.ptr,
                )
            });
            // ZSTD_CCtx_setParameter has NO cdict guard, so it must still work
            let set = l.sym::<FnCCtxSetParameter>("ZSTD_CCtx_setParameter");
            let direct = res(l, unsafe { set(ctx.ptr, ZSTD_c_compressionLevel, 9) });
            let snap = snapshot_all(l, ctx.ptr, "ZSTD_CCtx_getParameter");
            // and after dropping the dict (session_and_parameters clears it)
            let rr = cctx_reset(l, &ctx, ZSTD_reset_session_and_parameters);
            let apply2 = res(l, unsafe {
                l.sym::<FnSetParametersUsingCCtxParams>("ZSTD_CCtx_setParametersUsingCCtxParams")(
                    ctx.ptr, p.ptr,
                )
            });
            (refc, apply, direct, snap, rr, apply2)
        });
    }
}

/// `ZSTD_DCtx_setFormat` is a one-line wrapper over
/// `ZSTD_DCtx_setParameter(dctx, ZSTD_d_format, format)`, so it inherits both the
/// `stage_wrong` gate and the `parameter_outOfBound` check — a second entry point
/// into the same rejection sites, worth checking separately because a
/// translation could easily validate in the wrapper instead of forwarding.
#[test]
fn dctx_setformat_forwards_to_setparameter() {
    covers(&["CFG:121", "ERR:decompress/zstd_decompress.c:1916"]);
    for v in -3..=6 {
        diff(&format!("DCtx_setFormat({v})"), |l| {
            let ctx = Ctx::dctx(l);
            let f = l.sym::<FnCCtxParamsInit>("ZSTD_DCtx_setFormat");
            let sr = res(l, unsafe { f(ctx.ptr, v) });
            let get = l.sym::<FnCCtxGetParameter>("ZSTD_DCtx_getParameter");
            let mut got = SENTINEL;
            let gr = res(l, unsafe { get(ctx.ptr, ZSTD_d_format, &mut got) });
            (sr, gr, got)
        });
    }
}

/// `ZSTD_CCtx_setPledgedSrcSize` is *overridden* by `ZSTD_compress2`, which
/// re-pledges `srcSize` inside `ZSTD_CCtx_init_compressStream2` when
/// `endOp == ZSTD_e_end`. So a deliberately wrong pledge followed by
/// `ZSTD_compress2` must still succeed and produce the same frame as no pledge
/// at all — while the same wrong pledge followed by streaming must fail.
#[test]
fn compress2_overrides_the_pledge() {
    covers(&["CFG:45", "ERR:compress/zstd_compress.c:1233"]);
    let src = corpus(Corpus::Text, 30_000, 0x2101);
    let cap = compress_bound(&pair().c, src.len()) + 256;
    for pledge in [0u64, 1, 29_999, 30_000, 30_001, 1 << 32, ZSTD_CONTENTSIZE_UNKNOWN] {
        diff_bytes(&format!("pledge {pledge} then compress2"), |l| {
            let ctx = Ctx::cctx(l);
            let sr = res(l, unsafe {
                l.sym::<FnSetPledgedSrcSize>("ZSTD_CCtx_setPledgedSrcSize")(ctx.ptr, pledge)
            });
            let mut dst = vec![0xCDu8; cap];
            let n = unsafe {
                l.sym::<FnCompress2>("ZSTD_compress2")(
                    ctx.ptr,
                    dst.as_mut_ptr() as *mut c_void,
                    cap,
                    src.as_ptr() as *const c_void,
                    src.len(),
                )
            };
            let r = res(l, n);
            if let R::Ok(n) = r {
                dst.truncate(n);
            }
            (sr, r, Blob(dst))
        });
    }
}

/// Dense sweep of the `dictSize` axis of `ZSTD_getCParamRowSize` with
/// `srcSizeHint == ZSTD_CONTENTSIZE_UNKNOWN`, where the C computes
/// `srcSizeHint + dictSize + 500` on a `U64` that is `(U64)-1` — so the sum
/// **wraps** and `rSize == dictSize + 499`. `dictSize == 1` therefore yields
/// exactly 0 (tableID 3), `dictSize == 0` takes the separate
/// `unknown && dictSize == 0 -> ZSTD_CONTENTSIZE_UNKNOWN` branch (tableID 0),
/// and the 16 KB / 128 KB / 256 KB tableID edges land 499 bytes early. Packed so
/// `diff_bytes` names the first differing dictSize.
#[test]
fn getcparams_row_size_wrap_dense_dictsize_sweep() {
    covers(&["CFG:151,CFG:152"]);
    let interesting: &[usize] = &[
        0,
        1,
        2,
        498,
        499,
        500,
        501,
        16 * 1024 - 500,
        16 * 1024 - 499,
        16 * 1024 - 498,
        128 * 1024 - 500,
        128 * 1024 - 499,
        128 * 1024 - 498,
        256 * 1024 - 500,
        256 * 1024 - 499,
        256 * 1024 - 498,
    ];
    for lvl in [-1000, -1, 0, 1, 3, 19, 22] {
        diff_bytes(&format!("getCParams(UNKNOWN, dense dictSize, lvl {lvl})"), |l| {
            let f = l.sym::<FnGetCParams>("ZSTD_getCParams");
            let mut v = Vec::new();
            // dense 0..=1200 plus the tableID edges
            for ds in 0usize..=1200 {
                push_cparams(&mut v, &unsafe { f(lvl, CONTENTSIZE_UNKNOWN_U64, ds) });
            }
            for &ds in interesting {
                push_cparams(&mut v, &unsafe { f(lvl, CONTENTSIZE_UNKNOWN_U64, ds) });
                // srcSizeHint 0 is remapped to UNKNOWN by the public wrapper, so
                // these two must agree exactly
                push_cparams(&mut v, &unsafe { f(lvl, 0, ds) });
            }
            Blob(v)
        });
    }
}

/// `ZSTD_estimateCCtxSize` / `ZSTD_estimateCStreamSize` over a dense band of
/// levels at both ends of the range. The loop `for (level = MIN(cl, 1);
/// level <= cl; level++)` means a *negative* level runs exactly one iteration
/// while a positive one runs `1..=cl`, and level 0 runs exactly `{0}` (row
/// `ZSTD_CLEVEL_DEFAULT`) — three different shapes that a `for` translation can
/// easily get wrong at the edges.
#[test]
fn estimate_ccxt_and_cstream_size_dense_level_bands() {
    covers(&["CFG:159,CFG:162"]);
    let b = &pair().c;
    let lo = min_clevel(b);
    let mut levels: Vec<c_int> = Vec::new();
    for d in 0..80 {
        levels.push(lo + d);
    }
    levels.extend(-200..=30);
    levels.sort_unstable();
    levels.dedup();
    for name in ["ZSTD_estimateCCtxSize", "ZSTD_estimateCStreamSize"] {
        diff_bytes(&format!("{name} dense level band"), |l| {
            let f = l.sym::<FnEstimateFromLevel>(name);
            let mut v = Vec::with_capacity(levels.len() * 8);
            for &lvl in &levels {
                v.extend_from_slice(&(unsafe { f(lvl) } as u64).to_le_bytes());
            }
            Blob(v)
        });
    }
}

/// `ZSTD_c_srcSizeHint` has bounds `[0, INT_MAX]` *and* an
/// `if (value != 0)` guard, so the only rejected values are the negatives — but
/// the getter reads back `(int)CCtxParams->srcSizeHint`, which is where a
/// translation storing it as an unsigned type would show. Dense sweep at both
/// ends of the `int` range.
#[test]
fn src_size_hint_full_int_range_edges() {
    covers(&["CFG:110", "ERR:compress/zstd_compress.c:953"]);
    let mut vals: Vec<c_int> = (-8..=8).collect();
    for d in 0..8 {
        vals.push(c_int::MAX - d);
        vals.push(c_int::MIN + d);
    }
    vals.extend_from_slice(&[16_384, 131_072, 262_144, 1 << 30, -(1 << 30)]);
    vals.sort_unstable();
    vals.dedup();
    for v in vals {
        diff(&format!("srcSizeHint CCtx {v}"), |l| {
            cctx_set_then_get(l, ZSTD_c_srcSizeHint, v)
        });
        diff(&format!("srcSizeHint CCtxParams {v}"), |l| {
            cctxparams_set_then_get(l, ZSTD_c_srcSizeHint, v)
        });
    }
}

/// CONFIGS row 66: `cctx->inBuffTarget = blockSizeMax + (blockSizeMax ==
/// pledgedSrcSize)` in `ZSTD_CCtx_init_compressStream2`. The `+1` fires *only*
/// when the pledge is exactly one block, specifically to avoid emitting a
/// trailing empty block, so the frame has a different number of blocks for
/// pledge 131072 than for 131071 / 131073. Fed in 1024-byte chunks with
/// `ZSTD_e_continue` and finished with `ZSTD_e_end`, comparing the whole frame.
#[test]
fn pledged_src_size_exactly_one_block_changes_the_block_count() {
    covers(&["CFG:66,CFG:45"]);
    let src = corpus(Corpus::Text, 140_000, 0x2110);
    let cap = compress_bound(&pair().c, src.len()) + 1024;
    for pledge in [131_071usize, 131_072, 131_073, 130_000, 0] {
        let feed = if pledge == 0 { 131_072 } else { pledge };
        diff_bytes(&format!("pledge {pledge}, feed {feed} in 1 KB chunks"), |l| {
            let ctx = Ctx::cctx(l);
            let sr = res(l, unsafe {
                l.sym::<FnSetPledgedSrcSize>("ZSTD_CCtx_setPledgedSrcSize")(ctx.ptr, pledge as u64)
            });
            let f = l.sym::<FnCompressStream2>("ZSTD_compressStream2");
            let mut out = vec![0xCDu8; cap];
            let mut opos = 0usize;
            let mut fed = 0usize;
            let mut last = R::Ok(0);
            while fed < feed {
                let n = (feed - fed).min(1024);
                let chunk = &src[fed..fed + n];
                let mut i = ZSTD_inBuffer {
                    src: chunk.as_ptr() as *const c_void,
                    size: n,
                    pos: 0,
                };
                let mut o = ZSTD_outBuffer {
                    dst: out.as_mut_ptr() as *mut c_void,
                    size: out.len(),
                    pos: opos,
                };
                last = res(l, unsafe { f(ctx.ptr, &mut o, &mut i, ZSTD_e_continue) });
                opos = o.pos;
                fed += i.pos;
                if matches!(last, R::Err(..)) || i.pos == 0 {
                    break;
                }
            }
            let mut end = R::Ok(0);
            for _ in 0..64 {
                let mut i = ZSTD_inBuffer {
                    src: std::ptr::null(),
                    size: 0,
                    pos: 0,
                };
                let mut o = ZSTD_outBuffer {
                    dst: out.as_mut_ptr() as *mut c_void,
                    size: out.len(),
                    pos: opos,
                };
                let n = unsafe { f(ctx.ptr, &mut o, &mut i, ZSTD_e_end) };
                opos = o.pos;
                end = res(l, n);
                match end {
                    R::Ok(0) => break,
                    R::Ok(_) => {}
                    R::Err(..) => break,
                }
            }
            out.truncate(opos.min(out.len()));
            ((sr, last), end, Blob(out))
        });
    }
}

/// CONFIGS row 121: `ZSTD_DCtx_setFormat(ZSTD_f_zstd1_magicless)` followed by a
/// one-shot decode of a magicless frame. `ZSTD_startingInputLength(format)` is 1
/// for magicless versus 5 for zstd1, and the whole skippable-frame branch is
/// gated on `format == ZSTD_f_zstd1`, so the *parameter* decides how many bytes
/// the header parser demands. Both the matching and the mismatching combination
/// are compared, since a translation could ignore the format on the decode side
/// and still round-trip the matching case.
#[test]
fn magicless_format_parameter_end_to_end() {
    covers(&["CFG:121,CFG:120", "ERR:decompress/zstd_decompress.c:1916"]);
    let src = corpus(Corpus::Text, 20_000, 0x2111);
    // Build a magicless frame with the C library.
    let magicless = {
        let l = &pair().c;
        let ctx = Ctx::cctx(l);
        let set = l.sym::<FnCCtxSetParameter>("ZSTD_CCtx_setParameter");
        unsafe {
            set(ctx.ptr, ZSTD_c_format, ZSTD_f_zstd1_magicless);
        };
        let cap = compress_bound(l, src.len()) + 256;
        let mut dst = vec![0u8; cap];
        let n = unsafe {
            l.sym::<FnCompress2>("ZSTD_compress2")(
                ctx.ptr,
                dst.as_mut_ptr() as *mut c_void,
                cap,
                src.as_ptr() as *const c_void,
                src.len(),
            )
        };
        assert!(!is_error(l, n));
        dst.truncate(n);
        dst
    };
    let normal = c_compress(&src, 3);
    assert_eq!(normal.len(), magicless.len() + 4, "magicless must be 4B shorter");

    for (flabel, fmt) in [("zstd1", ZSTD_f_zstd1), ("magicless", ZSTD_f_zstd1_magicless)] {
        for (dlabel, frame) in [("normal", &normal), ("magicless", &magicless)] {
            diff_bytes(
                &format!("DCtx_setFormat({flabel}) one-shot decode of {dlabel}"),
                |l| {
                    let ctx = Ctx::dctx(l);
                    let sr = res(l, unsafe {
                        l.sym::<FnCCtxParamsInit>("ZSTD_DCtx_setFormat")(ctx.ptr, fmt)
                    });
                    let mut out = vec![0xCDu8; src.len() + 64];
                    let n = unsafe {
                        l.sym::<FnDecompressDCtx>("ZSTD_decompressDCtx")(
                            ctx.ptr,
                            out.as_mut_ptr() as *mut c_void,
                            out.len(),
                            frame.as_ptr() as *const c_void,
                            frame.len(),
                        )
                    };
                    let r = res(l, n);
                    if let R::Ok(n) = r {
                        out.truncate(n);
                    }
                    (sr, r, Blob(out))
                },
            );
            // and byte-at-a-time streaming, which is where
            // ZSTD_startingInputLength really bites
            diff_bytes(
                &format!("d_format={flabel} streaming 1 byte at a time of {dlabel}"),
                |l| {
                    let ctx = Ctx::dstream(l);
                    let sr = res(l, unsafe {
                        l.sym::<FnDCtxSetParameter>("ZSTD_DCtx_setParameter")(
                            ctx.ptr,
                            ZSTD_d_format,
                            fmt,
                        )
                    });
                    let g = l.sym::<FnDecompressStream>("ZSTD_decompressStream");
                    let mut out = vec![0xCDu8; src.len() + 64];
                    let mut opos = 0usize;
                    let mut last = R::Ok(0);
                    for k in 0..frame.len() {
                        let mut i = ZSTD_inBuffer {
                            src: frame[k..].as_ptr() as *const c_void,
                            size: 1,
                            pos: 0,
                        };
                        let mut o = ZSTD_outBuffer {
                            dst: out.as_mut_ptr() as *mut c_void,
                            size: out.len(),
                            pos: opos,
                        };
                        last = res(l, unsafe { g(ctx.ptr, &mut o, &mut i) });
                        opos = o.pos;
                        if matches!(last, R::Err(..)) {
                            break;
                        }
                    }
                    out.truncate(opos.min(out.len()));
                    (sr, last, Blob(out))
                },
            );
        }
    }
}

/// `ZSTD_DCtx_setParameter` has one guard that is *not* a bounds check:
/// `ZSTD_d_refMultipleDDicts` on a **static** DCtx returns
/// `parameter_unsupported` ("Static dctx does not support multiple DDicts!")
/// at `zstd_decompress.c:1930`, and only for that one parameter. Reaching it
/// needs `ZSTD_initStaticDCtx`, so it is invisible to every heap-context test.
/// All seven parameters are driven on both a static and a heap DCtx so the
/// asymmetry is pinned rather than assumed.
#[test]
fn refmultipleddicts_is_refused_on_a_static_dctx() {
    covers(&["CFG:174,CFG:311", "ERR:decompress/zstd_decompress.c:1930"]);
    let need = diff("estimateDCtxSize", |l| {
        res(l, unsafe {
            l.sym::<FnEstimateVoid>("ZSTD_estimateDCtxSize")()
        })
    });
    let need = match need {
        R::Ok(n) => n,
        e => panic!("estimateDCtxSize failed: {e:?}"),
    };
    let b = &pair().c;
    for &(name, id) in ALL_DPARAMS {
        let hi = dparam_bounds(b, id).upperBound;
        for &v in &[0, 1, hi] {
            diff(&format!("static DCtx setParameter(d_{name}, {v})"), |l| {
                // 8-byte aligned workspace of exactly the estimated size
                let mut ws = vec![0u64; need / 8 + 1];
                let ptr = unsafe {
                    l.sym::<FnInitStaticCCtx>("ZSTD_initStaticDCtx")(
                        ws.as_mut_ptr() as *mut c_void,
                        ws.len() * 8,
                    )
                };
                assert!(!ptr.is_null(), "[{}] initStaticDCtx returned NULL", l.tag);
                let set = l.sym::<FnDCtxSetParameter>("ZSTD_DCtx_setParameter");
                let get = l.sym::<FnCCtxGetParameter>("ZSTD_DCtx_getParameter");
                let sr = res(l, unsafe { set(ptr, id, v) });
                let mut got = SENTINEL;
                let gr = res(l, unsafe { get(ptr, id, &mut got) });
                // ZSTD_freeDCtx on a static DCtx must report memory_allocation
                // and must not free our Vec.
                let fr = res(l, unsafe { l.sym::<FnFreeCCtx>("ZSTD_freeDCtx")(ptr) });
                (sr, gr, got, fr)
            });
            diff(&format!("heap DCtx setParameter(d_{name}, {v})"), |l| {
                dctx_set_then_get(l, id, v)
            });
        }
    }
}
