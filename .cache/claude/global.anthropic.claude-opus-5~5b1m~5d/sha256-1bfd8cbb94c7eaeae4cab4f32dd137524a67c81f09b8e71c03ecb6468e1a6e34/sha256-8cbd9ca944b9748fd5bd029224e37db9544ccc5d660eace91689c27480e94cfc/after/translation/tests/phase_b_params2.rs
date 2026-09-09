//! Phase B (part 2): parameter equivalences, table-ID selection, row match finder,
//! block splitter, stable buffers, CCtxParams API, getCParams/adjustCParams.
#![allow(non_snake_case, dead_code)]
mod common;
use common::*;
use std::os::raw::{c_int, c_uint, c_ulonglong, c_void};

type FnNew = unsafe extern "C" fn() -> *mut c_void;
type FnFree = unsafe extern "C" fn(*mut c_void) -> usize;
type FnSetParam = unsafe extern "C" fn(*mut c_void, c_int, c_int) -> usize;
type FnGetParam = unsafe extern "C" fn(*mut c_void, c_int, *mut c_int) -> usize;
type FnCompress = unsafe extern "C" fn(*mut u8, usize, *const u8, usize, c_int) -> usize;
type FnCompress2 = unsafe extern "C" fn(*mut c_void, *mut u8, usize, *const u8, usize) -> usize;
type FnDecompressDCtx =
    unsafe extern "C" fn(*mut c_void, *mut u8, usize, *const u8, usize) -> usize;
type FnBound = unsafe extern "C" fn(usize) -> usize;
type FnPledged = unsafe extern "C" fn(*mut c_void, u64) -> usize;
type FnCParamsInit = unsafe extern "C" fn(*mut c_void, c_int) -> usize;
type FnCParamsInitAdv = unsafe extern "C" fn(*mut c_void, Parameters) -> usize;
type FnCParamsReset = unsafe extern "C" fn(*mut c_void) -> usize;
type FnUseParams = unsafe extern "C" fn(*mut c_void, *const c_void) -> usize;
type FnGetCParams = unsafe extern "C" fn(c_int, c_ulonglong, usize) -> CParams;
type FnGetParams = unsafe extern "C" fn(c_int, c_ulonglong, usize) -> Parameters;
type FnAdjust = unsafe extern "C" fn(CParams, c_ulonglong, usize) -> CParams;
type FnCheck = unsafe extern "C" fn(CParams) -> usize;
type FnCycleLog = unsafe extern "C" fn(c_uint, c_int) -> c_uint;
type FnStream2 = unsafe extern "C" fn(*mut c_void, *mut OutBuffer, *mut InBuffer, c_int) -> usize;
type FnStream = unsafe extern "C" fn(*mut c_void, *mut OutBuffer, *mut InBuffer) -> usize;
type FnSimpleArgsC = unsafe extern "C" fn(
    *mut c_void,
    *mut u8,
    usize,
    *mut usize,
    *const u8,
    usize,
    *mut usize,
    c_int,
) -> usize;
type FnSimpleArgsD = unsafe extern "C" fn(
    *mut c_void,
    *mut u8,
    usize,
    *mut usize,
    *const u8,
    usize,
    *mut usize,
) -> usize;
type FnInitCStreamSrcSize = unsafe extern "C" fn(*mut c_void, c_int, c_ulonglong) -> usize;
type FnInitCStreamAdv =
    unsafe extern "C" fn(*mut c_void, *const u8, usize, Parameters, c_ulonglong) -> usize;
type FnInitCStreamDict = unsafe extern "C" fn(*mut c_void, *const u8, usize, c_int) -> usize;
type FnInitCStreamCDict = unsafe extern "C" fn(*mut c_void, *const c_void) -> usize;
type FnInitCStreamCDictAdv =
    unsafe extern "C" fn(*mut c_void, *const c_void, FParams, c_ulonglong) -> usize;
type FnResetCStream = unsafe extern "C" fn(*mut c_void, c_ulonglong) -> usize;
type FnCreateCDict = unsafe extern "C" fn(*const u8, usize, c_int) -> *mut c_void;
type FnFreeDict = unsafe extern "C" fn(*mut c_void) -> usize;
type FnInitDStreamDict = unsafe extern "C" fn(*mut c_void, *const u8, usize) -> usize;
type FnInitDStreamDDict = unsafe extern "C" fn(*mut c_void, *const c_void) -> usize;
type FnCreateDDict = unsafe extern "C" fn(*const u8, usize) -> *mut c_void;
type FnReset = unsafe extern "C" fn(*mut c_void, c_int) -> usize;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CParams {
    pub windowLog: c_uint,
    pub chainLog: c_uint,
    pub hashLog: c_uint,
    pub searchLog: c_uint,
    pub minMatch: c_uint,
    pub targetLength: c_uint,
    pub strategy: c_int,
}
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FParams {
    pub contentSizeFlag: c_int,
    pub checksumFlag: c_int,
    pub noDictIDFlag: c_int,
}
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Parameters {
    pub cParams: CParams,
    pub fParams: FParams,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct InBuffer {
    pub src: *const u8,
    pub size: usize,
    pub pos: usize,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct OutBuffer {
    pub dst: *mut u8,
    pub size: usize,
    pub pos: usize,
}

const C_COMPRESSIONLEVEL: c_int = 100;
const C_WINDOWLOG: c_int = 101;
const C_HASHLOG: c_int = 102;
const C_CHAINLOG: c_int = 103;
const C_STRATEGY: c_int = 107;
const C_TARGETCBLOCKSIZE: c_int = 130;
const C_CHECKSUMFLAG: c_int = 201;
const C_SRCSIZEHINT: c_int = 1004;
const C_STABLEINBUFFER: c_int = 1006;
const C_STABLEOUTBUFFER: c_int = 1007;
const C_USEROWMATCHFINDER: c_int = 1011;
const C_MAXBLOCKSIZE: c_int = 1015;
const C_BLOCKSPLITTERLEVEL: c_int = 1017;
const C_SPLITAFTERSEQUENCES: c_int = 1010;
const D_MAXBLOCKSIZE: c_int = 1005;
const D_WINDOWLOGMAX: c_int = 100;
const E_END: c_int = 2;

/// compress with compress2 in both libs under `params`, compare bytes,
/// then round-trip through decompressDCtx with `dparams`.
fn diff2(params: &[(c_int, c_int)], pledged: Option<u64>, src: &[u8], dparams: &[(c_int, c_int)],
         ctx: &str) -> Option<Vec<u8>> {
    let (cn, rn) = unsafe { pair::<FnNew>("ZSTD_createCCtx") };
    let (cf, rf) = unsafe { pair::<FnFree>("ZSTD_freeCCtx") };
    let (cs, rs) = unsafe { pair::<FnSetParam>("ZSTD_CCtx_setParameter") };
    let (cp, rp) = unsafe { pair::<FnPledged>("ZSTD_CCtx_setPledgedSrcSize") };
    let (c2, r2) = unsafe { pair::<FnCompress2>("ZSTD_compress2") };
    let (cb, _) = unsafe { pair::<FnBound>("ZSTD_compressBound") };

    let cc = unsafe { cn() };
    let rc = unsafe { rn() };
    let mut bad = false;
    for &(p, v) in params {
        let a = unsafe { cs(cc, p, v) };
        let b = unsafe { rs(rc, p, v) };
        assert_eq!(a, b, "{ctx}: setParameter({p},{v})");
        if is_error(a) {
            bad = true;
        }
    }
    if let Some(n) = pledged {
        let a = unsafe { cp(cc, n) };
        let b = unsafe { rp(rc, n) };
        assert_eq!(a, b, "{ctx}: pledged");
        if is_error(a) {
            bad = true;
        }
    }
    let mut out = None;
    if !bad {
        let cap = unsafe { cb(src.len()) } + 64;
        let mut cbuf = vec![0xAAu8; cap];
        let mut rbuf = vec![0x55u8; cap];
        let a = unsafe { c2(cc, cbuf.as_mut_ptr(), cap, src.as_ptr(), src.len()) };
        let b = unsafe { r2(rc, rbuf.as_mut_ptr(), cap, src.as_ptr(), src.len()) };
        assert_eq!(a, b, "{ctx}: compress2 (C err {})", err_code(a));
        if !is_error(a) {
            assert_bytes_eq(&format!("{ctx}: compress2 bytes"), &cbuf[..a], &rbuf[..b]);
            cbuf.truncate(a);
            decompress_diff(&cbuf, Some(src), dparams, ctx);
            out = Some(cbuf);
        }
    }
    unsafe {
        cf(cc);
        rf(rc);
    }
    out
}

fn decompress_diff(frame: &[u8], expect: Option<&[u8]>, dparams: &[(c_int, c_int)], ctx: &str) {
    let (cn, rn) = unsafe { pair::<FnNew>("ZSTD_createDCtx") };
    let (cf, rf) = unsafe { pair::<FnFree>("ZSTD_freeDCtx") };
    let (cs, rs) = unsafe { pair::<FnSetParam>("ZSTD_DCtx_setParameter") };
    let (cd, rd) = unsafe { pair::<FnDecompressDCtx>("ZSTD_decompressDCtx") };
    let cc = unsafe { cn() };
    let rc = unsafe { rn() };
    for &(p, v) in dparams {
        let a = unsafe { cs(cc, p, v) };
        let b = unsafe { rs(rc, p, v) };
        assert_eq!(a, b, "{ctx}: DCtx_setParameter({p},{v})");
    }
    let cap = expect.map(|e| e.len()).unwrap_or(1 << 21) + 64;
    let mut co = vec![0u8; cap];
    let mut ro = vec![0u8; cap];
    let a = unsafe { cd(cc, co.as_mut_ptr(), cap, frame.as_ptr(), frame.len()) };
    let b = unsafe { rd(rc, ro.as_mut_ptr(), cap, frame.as_ptr(), frame.len()) };
    assert_eq!(a, b, "{ctx}: decompressDCtx (C err {})", err_code(a));
    if !is_error(a) {
        assert_bytes_eq(&format!("{ctx}: dec bytes"), &co[..a], &ro[..b]);
        if let Some(e) = expect {
            assert_bytes_eq(&format!("{ctx}: dec vs orig"), &co[..a], e);
        }
    }
    unsafe {
        cf(cc);
        rf(rc);
    }
}

/// Row 14/15: level 0 == 3, level > max clamped, level < min clamped.
#[test]
fn cfg_level_equivalences() {
    let (cc, rc) = unsafe { pair::<FnCompress>("ZSTD_compress") };
    let (cb, _) = unsafe { pair::<FnBound>("ZSTD_compressBound") };
    let mut rng = Rng::new(0x1E4E);
    for &shape in ALL_SHAPES {
        let src = gen(shape, rng.range(0, 60000), &mut rng);
        let cap = unsafe { cb(src.len()) } + 64;
        let mut out: Vec<Vec<u8>> = Vec::new();
        for &lvl in &[0, 3, 100, 22, 1000, -131072, -200000, i32::MIN + 1] {
            let mut cbuf = vec![0u8; cap];
            let mut rbuf = vec![0u8; cap];
            let a = unsafe { cc(cbuf.as_mut_ptr(), cap, src.as_ptr(), src.len(), lvl) };
            let b = unsafe { rc(rbuf.as_mut_ptr(), cap, src.as_ptr(), src.len(), lvl) };
            assert_eq!(a, b, "level {lvl} shape {shape:?}");
            assert!(!is_error(a), "level {lvl}: C err {}", err_code(a));
            assert_bytes_eq(&format!("level {lvl} shape {shape:?}"), &cbuf[..a], &rbuf[..b]);
            cbuf.truncate(a);
            out.push(cbuf);
        }
        // 0 == 3, 100 == 22 == 1000
        assert_eq!(out[0], out[1], "level 0 must equal level 3 ({shape:?})");
        assert_eq!(out[2], out[3], "level 100 must equal level 22 ({shape:?})");
        assert_eq!(out[4], out[3], "level 1000 must equal level 22 ({shape:?})");
        assert_eq!(out[6], out[5], "level -200000 must equal minCLevel ({shape:?})");
    }
}

/// Rows 18-21, 30: row match finder auto/enable/disable across strategies and windowLogs.
#[test]
fn cfg_rowmatchfinder_matrix() {
    let mut rng = Rng::new(0x2202);
    for &strat in &[1, 2, 3, 4, 5, 6, 7] {
        for &rowmf in &[0, 1, 2] {
            for &wlog in &[13, 14, 15, 20] {
                for &clog in &[0, 16, 25] {
                    for &shape in &[Shape::Text, Shape::Random, Shape::Repetitive] {
                        let size = rng.range(1000, 250000);
                        let src = gen(shape, size, &mut rng);
                        let ctx = format!(
                            "rowmf strat={strat} rowmf={rowmf} wlog={wlog} clog={clog} shape={shape:?} size={size}"
                        );
                        let mut p = vec![
                            (C_STRATEGY, strat),
                            (C_USEROWMATCHFINDER, rowmf),
                            (C_WINDOWLOG, wlog),
                        ];
                        if clog != 0 {
                            p.push((C_CHAINLOG, clog));
                        }
                        diff2(&p, None, &src, &[(D_WINDOWLOGMAX, 31)], &ctx);
                    }
                }
            }
        }
    }
}

/// Rows 26-28: window wrap / extDict, and decoding limits.
#[test]
fn cfg_window_wrap() {
    let mut rng = Rng::new(0x3303);
    for &wlog in &[10, 11, 14, 17, 20, 25, 31] {
        for &shape in &[Shape::Text, Shape::Random, Shape::LongMatches] {
            for &size in &[4096usize, 100000, 400000] {
                let src = gen(shape, size, &mut rng);
                let ctx = format!("wrap wlog={wlog} shape={shape:?} size={size}");
                diff2(
                    &[(C_WINDOWLOG, wlog), (C_COMPRESSIONLEVEL, 5)],
                    None,
                    &src,
                    &[(D_WINDOWLOGMAX, 31)],
                    &ctx,
                );
                // and with the default decoder window limit (27): frames with wlog 31
                // must be rejected identically
                if let Some(frame) = diff2(
                    &[(C_WINDOWLOG, wlog), (C_COMPRESSIONLEVEL, 5)],
                    None,
                    &src,
                    &[(D_WINDOWLOGMAX, 31)],
                    &ctx,
                ) {
                    decompress_diff(&frame, None, &[], &format!("{ctx} default-dwlm"));
                }
            }
        }
    }
}

/// Row 53: targetCBlockSize below MIN is silently raised to 1340.
#[test]
fn cfg_targetcblocksize_clamp() {
    let mut rng = Rng::new(0x4404);
    for &shape in &[Shape::Text, Shape::Random, Shape::Mixed] {
        let src = gen(shape, 262144, &mut rng);
        let mut frames = Vec::new();
        for &v in &[1, 100, 1339, 1340] {
            let ctx = format!("tcbs={v} shape={shape:?}");
            let f = diff2(
                &[(C_TARGETCBLOCKSIZE, v), (C_COMPRESSIONLEVEL, 5)],
                None,
                &src,
                &[],
                &ctx,
            )
            .expect("compress ok");
            frames.push(f);
        }
        for i in 0..frames.len() {
            assert_eq!(frames[i], frames[3], "targetCBlockSize clamp mismatch idx {i}");
        }
    }
}

/// Rows 56-59: srcSizeHint selects the cParam table row.
#[test]
fn cfg_srcsizehint_tables() {
    let mut rng = Rng::new(0x5505);
    for &hint in &[0, 1, 10000, 16384, 100000, 131072, 200000, 262144, 1000000, i32::MAX] {
        for &lvl in &[1, 2, 4, 11, 19] {
            for &shape in &[Shape::Text, Shape::Random] {
                let size = rng.range(1000, 150000);
                let src = gen(shape, size, &mut rng);
                let ctx = format!("ssh={hint} lvl={lvl} shape={shape:?} size={size}");
                diff2(
                    &[(C_SRCSIZEHINT, hint), (C_COMPRESSIONLEVEL, lvl)],
                    None,
                    &src,
                    &[],
                    &ctx,
                );
            }
        }
    }
    // pledgedSrcSize also drives the table selection
    for &pledged in &[0u64, 1, 16384, 131072, 262144, 1 << 20] {
        for &lvl in &[2, 4, 11] {
            let src = gen(Shape::Text, pledged as usize, &mut rng);
            let ctx = format!("pledged={pledged} lvl={lvl}");
            diff2(&[(C_COMPRESSIONLEVEL, lvl)], Some(pledged), &src, &[], &ctx);
        }
    }
}

/// Row 61: compress maxBlockSize paired with decoder d_maxBlockSize.
#[test]
fn cfg_maxblocksize_pairs() {
    let mut rng = Rng::new(0x6606);
    for &cmbs in &[0, 1024, 4096, 32768, 131072] {
        for &dmbs in &[0, 1024, 32768, 131072] {
            let src = gen(Shape::Text, 200000, &mut rng);
            let ctx = format!("mbs c={cmbs} d={dmbs}");
            let params: Vec<(c_int, c_int)> = if cmbs == 0 {
                vec![(C_COMPRESSIONLEVEL, 5)]
            } else {
                vec![(C_MAXBLOCKSIZE, cmbs), (C_COMPRESSIONLEVEL, 5)]
            };
            let dp: Vec<(c_int, c_int)> =
                if dmbs == 0 { vec![] } else { vec![(D_MAXBLOCKSIZE, dmbs)] };
            let frame = diff2(&params, None, &src, &[], &ctx);
            if let Some(f) = frame {
                // may legitimately error (block too large for the decoder) — compare returns
                decompress_diff(&f, None, &dp, &format!("{ctx} dec"));
            }
        }
    }
}

/// Rows 62-65: every blockSplitterLevel across strategies.
#[test]
fn cfg_blocksplitter_levels() {
    let mut rng = Rng::new(0x7707);
    for &bsl in &[0, 1, 2, 3, 4, 5, 6] {
        for &strat in &[1, 3, 5, 7, 9] {
            for &shape in &[Shape::Mixed, Shape::Text, Shape::Random, Shape::Sparse] {
                let size = rng.range(100000, 420000);
                let src = gen(shape, size, &mut rng);
                let ctx = format!("bsl={bsl} strat={strat} shape={shape:?} size={size}");
                diff2(
                    &[(C_BLOCKSPLITTERLEVEL, bsl), (C_STRATEGY, strat)],
                    None,
                    &src,
                    &[],
                    &ctx,
                );
            }
        }
    }
}

/// Row 66: splitAfterSequences auto/enable/disable × strategy × windowLog.
#[test]
fn cfg_splitaftersequences() {
    let mut rng = Rng::new(0x8808);
    for &sas in &[0, 1, 2] {
        for &strat in &[5, 7, 8, 9] {
            for &wlog in &[16, 17, 20] {
                let size = rng.range(150000, 400000);
                let src = gen(Shape::Mixed, size, &mut rng);
                let ctx = format!("sas={sas} strat={strat} wlog={wlog} size={size}");
                diff2(
                    &[(C_SPLITAFTERSEQUENCES, sas), (C_STRATEGY, strat), (C_WINDOWLOG, wlog)],
                    None,
                    &src,
                    &[(D_WINDOWLOGMAX, 31)],
                    &ctx,
                );
            }
        }
    }
}

/// Rows 73-76: the ZSTD_CCtx_params object API.
#[test]
fn cfg_cctxparams_api() {
    let (c_cpn, r_cpn) = unsafe { pair::<FnNew>("ZSTD_createCCtxParams") };
    let (c_cpf, r_cpf) = unsafe { pair::<FnFree>("ZSTD_freeCCtxParams") };
    let (c_init, r_init) = unsafe { pair::<FnCParamsInit>("ZSTD_CCtxParams_init") };
    let (c_inita, r_inita) = unsafe { pair::<FnCParamsInitAdv>("ZSTD_CCtxParams_init_advanced") };
    let (c_rst, r_rst) = unsafe { pair::<FnCParamsReset>("ZSTD_CCtxParams_reset") };
    let (c_sp, r_sp) = unsafe { pair::<FnSetParam>("ZSTD_CCtxParams_setParameter") };
    let (c_gp, r_gp) = unsafe { pair::<FnGetParam>("ZSTD_CCtxParams_getParameter") };
    let (c_use, r_use) = unsafe { pair::<FnUseParams>("ZSTD_CCtx_setParametersUsingCCtxParams") };
    let (c_new, r_new) = unsafe { pair::<FnNew>("ZSTD_createCCtx") };
    let (c_free, r_free) = unsafe { pair::<FnFree>("ZSTD_freeCCtx") };
    let (c_c2, r_c2) = unsafe { pair::<FnCompress2>("ZSTD_compress2") };
    let (c_gpar, r_gpar) = unsafe { pair::<FnGetParams>("ZSTD_getParams") };
    let (cb, _) = unsafe { pair::<FnBound>("ZSTD_compressBound") };

    let mut rng = Rng::new(0x9909);
    let cp = unsafe { c_cpn() };
    let rp = unsafe { r_cpn() };
    assert!(!cp.is_null() && !rp.is_null());

    for round in 0..12 {
        let lvl = [1, 3, 6, 12, 19][rng.below(5)];
        let use_advanced = round % 3 == 0;
        let do_reset = round % 4 == 3;
        let ctx = format!("cctxparams round={round} lvl={lvl} adv={use_advanced} reset={do_reset}");
        if use_advanced {
            let cpar = unsafe { c_gpar(lvl, 300000, 0) };
            let rpar = unsafe { r_gpar(lvl, 300000, 0) };
            assert_eq!(cpar, rpar, "{ctx}: getParams");
            let a = unsafe { c_inita(cp, cpar) };
            let b = unsafe { r_inita(rp, rpar) };
            assert_eq!(a, b, "{ctx}: init_advanced");
        } else {
            let a = unsafe { c_init(cp, lvl) };
            let b = unsafe { r_init(rp, lvl) };
            assert_eq!(a, b, "{ctx}: init");
        }
        for (p, v) in [(C_STRATEGY, 7), (C_CHECKSUMFLAG, 1), (C_WINDOWLOG, 18)] {
            let a = unsafe { c_sp(cp, p, v) };
            let b = unsafe { r_sp(rp, p, v) };
            assert_eq!(a, b, "{ctx}: CCtxParams_setParameter({p},{v})");
            let mut av = 0;
            let mut bv = 0;
            let ga = unsafe { c_gp(cp, p, &mut av) };
            let gb = unsafe { r_gp(rp, p, &mut bv) };
            assert_eq!(ga, gb, "{ctx}: getParameter ret");
            assert_eq!(av, bv, "{ctx}: getParameter value");
        }
        if do_reset {
            let a = unsafe { c_rst(cp) };
            let b = unsafe { r_rst(rp) };
            assert_eq!(a, b, "{ctx}: reset");
        }
        let cc = unsafe { c_new() };
        let rc = unsafe { r_new() };
        let a = unsafe { c_use(cc, cp) };
        let b = unsafe { r_use(rc, rp) };
        assert_eq!(a, b, "{ctx}: setParametersUsingCCtxParams");
        let src = gen(ALL_SHAPES[rng.below(ALL_SHAPES.len())], rng.range(0, 120000), &mut rng);
        let cap = unsafe { cb(src.len()) } + 64;
        let mut cbuf = vec![0u8; cap];
        let mut rbuf = vec![0u8; cap];
        let ca = unsafe { c_c2(cc, cbuf.as_mut_ptr(), cap, src.as_ptr(), src.len()) };
        let rb = unsafe { r_c2(rc, rbuf.as_mut_ptr(), cap, src.as_ptr(), src.len()) };
        assert_eq!(ca, rb, "{ctx}: compress2");
        if !is_error(ca) {
            assert_bytes_eq(&ctx, &cbuf[..ca], &rbuf[..rb]);
            cbuf.truncate(ca);
            decompress_diff(&cbuf, Some(&src), &[(D_WINDOWLOGMAX, 31)], &ctx);
        }
        unsafe {
            c_free(cc);
            r_free(rc);
        }
    }
    unsafe {
        c_cpf(cp);
        r_cpf(rp);
    }
}

/// Row 77: getCParams / getParams / adjustCParams / checkCParams / cycleLog sweep.
#[test]
fn cfg_getcparams_sweep() {
    let (c_gc, r_gc) = unsafe { pair::<FnGetCParams>("ZSTD_getCParams") };
    let (c_gp, r_gp) = unsafe { pair::<FnGetParams>("ZSTD_getParams") };
    let (c_adj, r_adj) = unsafe { pair::<FnAdjust>("ZSTD_adjustCParams") };
    let (c_chk, r_chk) = unsafe { pair::<FnCheck>("ZSTD_checkCParams") };
    let (c_cl, r_cl) = unsafe { pair::<FnCycleLog>("ZSTD_cycleLog") };

    let levels = [-131072, -1000, -22, -1, 0, 1, 3, 9, 13, 19, 22, 23, 1000];
    let sizes: [c_ulonglong; 8] = [0, 1, 16384, 131072, 262144, 1000000, 1 << 32, u64::MAX];
    let dicts = [0usize, 1, 1024, 112640, 1 << 20];
    for &lvl in &levels {
        for &sz in &sizes {
            for &d in &dicts {
                let a = unsafe { c_gc(lvl, sz, d) };
                let b = unsafe { r_gc(lvl, sz, d) };
                assert_eq!(a, b, "getCParams({lvl},{sz},{d})");
                let pa = unsafe { c_gp(lvl, sz, d) };
                let pb = unsafe { r_gp(lvl, sz, d) };
                assert_eq!(pa, pb, "getParams({lvl},{sz},{d})");
                let aa = unsafe { c_adj(a, sz, d) };
                let ab = unsafe { r_adj(b, sz, d) };
                assert_eq!(aa, ab, "adjustCParams({lvl},{sz},{d})");
                assert_eq!(
                    unsafe { c_chk(a) },
                    unsafe { r_chk(b) },
                    "checkCParams({lvl},{sz},{d})"
                );
            }
        }
    }
    // checkCParams on deliberately invalid structs
    for bad in [
        CParams { windowLog: 9, chainLog: 10, hashLog: 10, searchLog: 1, minMatch: 4,
                  targetLength: 0, strategy: 1 },
        CParams { windowLog: 32, chainLog: 10, hashLog: 10, searchLog: 1, minMatch: 4,
                  targetLength: 0, strategy: 1 },
        CParams { windowLog: 20, chainLog: 5, hashLog: 10, searchLog: 1, minMatch: 4,
                  targetLength: 0, strategy: 1 },
        CParams { windowLog: 20, chainLog: 10, hashLog: 31, searchLog: 1, minMatch: 4,
                  targetLength: 0, strategy: 1 },
        CParams { windowLog: 20, chainLog: 10, hashLog: 10, searchLog: 0, minMatch: 4,
                  targetLength: 0, strategy: 1 },
        CParams { windowLog: 20, chainLog: 10, hashLog: 10, searchLog: 1, minMatch: 2,
                  targetLength: 0, strategy: 1 },
        CParams { windowLog: 20, chainLog: 10, hashLog: 10, searchLog: 1, minMatch: 8,
                  targetLength: 0, strategy: 1 },
        CParams { windowLog: 20, chainLog: 10, hashLog: 10, searchLog: 1, minMatch: 4,
                  targetLength: 200000, strategy: 1 },
        CParams { windowLog: 20, chainLog: 10, hashLog: 10, searchLog: 1, minMatch: 4,
                  targetLength: 0, strategy: 0 },
        CParams { windowLog: 20, chainLog: 10, hashLog: 10, searchLog: 1, minMatch: 4,
                  targetLength: 0, strategy: 10 },
    ] {
        assert_eq!(unsafe { c_chk(bad) }, unsafe { r_chk(bad) }, "checkCParams({bad:?})");
        for &sz in &[0u64, 1000, 1 << 20] {
            assert_eq!(
                unsafe { c_adj(bad, sz, 0) },
                unsafe { r_adj(bad, sz, 0) },
                "adjustCParams({bad:?},{sz})"
            );
        }
    }
    for hashLog in [0u32, 1, 6, 10, 20, 30, 31] {
        for strat in 0..=10 {
            assert_eq!(
                unsafe { c_cl(hashLog, strat) },
                unsafe { r_cl(hashLog, strat) },
                "cycleLog({hashLog},{strat})"
            );
        }
    }
}

/// Rows 86-88: stableInBuffer / stableOutBuffer valid usage.
#[test]
fn cfg_stable_buffers() {
    let (cn, rn) = unsafe { pair::<FnNew>("ZSTD_createCCtx") };
    let (cf, rf) = unsafe { pair::<FnFree>("ZSTD_freeCCtx") };
    let (cs, rs) = unsafe { pair::<FnSetParam>("ZSTD_CCtx_setParameter") };
    let (c2, r2) = unsafe { pair::<FnStream2>("ZSTD_compressStream2") };
    let (cb, _) = unsafe { pair::<FnBound>("ZSTD_compressBound") };
    let mut rng = Rng::new(0xAAB1);

    for &(sin, sout) in &[(1, 0), (0, 1), (1, 1)] {
        for &out_chunk in &[0usize, 100, 4096] {
            let src = gen(Shape::Text, rng.range(1000, 262144), &mut rng);
            let ctx = format!("stable in={sin} out={sout} outchunk={out_chunk} n={}", src.len());
            let bound = unsafe { cb(src.len()) } + 64;
            let mut outs: [Vec<u8>; 2] = [Vec::new(), Vec::new()];
            let mut rets: [Vec<usize>; 2] = [Vec::new(), Vec::new()];
            for which in 0..2 {
                let z = if which == 0 { unsafe { cn() } } else { unsafe { rn() } };
                let setp = |p, v| {
                    if which == 0 {
                        unsafe { cs(z, p, v) }
                    } else {
                        unsafe { rs(z, p, v) }
                    }
                };
                rets[which].push(setp(C_STABLEINBUFFER, sin));
                rets[which].push(setp(C_STABLEOUTBUFFER, sout));
                rets[which].push(setp(C_COMPRESSIONLEVEL, 6));
                // stableOutBuffer requires one big output buffer
                let osz = if sout == 1 || out_chunk == 0 { bound } else { out_chunk };
                let mut obuf = vec![0u8; osz];
                let mut collected = Vec::new();
                let mut input = InBuffer { src: src.as_ptr(), size: src.len(), pos: 0 };
                let mut out = OutBuffer { dst: obuf.as_mut_ptr(), size: osz, pos: 0 };
                loop {
                    let rc = if which == 0 {
                        unsafe { c2(z, &mut out, &mut input, E_END) }
                    } else {
                        unsafe { r2(z, &mut out, &mut input, E_END) }
                    };
                    rets[which].push(rc);
                    if is_error(rc) {
                        break;
                    }
                    if sout == 1 {
                        // must keep the same buffer/size; drain only at the end
                        if rc == 0 {
                            collected.extend_from_slice(&obuf[..out.pos]);
                            break;
                        }
                    } else {
                        collected.extend_from_slice(&obuf[..out.pos]);
                        out.pos = 0;
                        if rc == 0 {
                            break;
                        }
                    }
                }
                outs[which] = collected;
                if which == 0 {
                    unsafe { cf(z) };
                } else {
                    unsafe { rf(z) };
                }
            }
            assert_eq!(rets[0], rets[1], "{ctx}: return sequence");
            assert_bytes_eq(&ctx, &outs[0], &outs[1]);
            if !outs[0].is_empty() {
                decompress_diff(&outs[0], Some(&src), &[], &ctx);
            }
        }
    }
}

/// Row 99: the *_simpleArgs entry points.
#[test]
fn cfg_simple_args() {
    let (c_cn, r_cn) = unsafe { pair::<FnNew>("ZSTD_createCCtx") };
    let (c_cf, r_cf) = unsafe { pair::<FnFree>("ZSTD_freeCCtx") };
    let (c_ca, r_ca) = unsafe { pair::<FnSimpleArgsC>("ZSTD_compressStream2_simpleArgs") };
    let (c_dn, r_dn) = unsafe { pair::<FnNew>("ZSTD_createDCtx") };
    let (c_df, r_df) = unsafe { pair::<FnFree>("ZSTD_freeDCtx") };
    let (c_da, r_da) = unsafe { pair::<FnSimpleArgsD>("ZSTD_decompressStream_simpleArgs") };
    let (c_sp, r_sp) = unsafe { pair::<FnSetParam>("ZSTD_CCtx_setParameter") };
    let (cb, _) = unsafe { pair::<FnBound>("ZSTD_compressBound") };

    let mut rng = Rng::new(0xBBB2);
    for round in 0..8 {
        let src = gen(ALL_SHAPES[rng.below(ALL_SHAPES.len())], rng.range(0, 150000), &mut rng);
        let in_chunk = rng.range(1, 20000);
        let out_chunk = rng.range(1, 20000);
        let ctx = format!("simpleArgs round={round} n={} in={in_chunk} out={out_chunk}", src.len());
        let mut frames: [Vec<u8>; 2] = [Vec::new(), Vec::new()];
        let mut rets: [Vec<usize>; 2] = [Vec::new(), Vec::new()];
        for which in 0..2 {
            let z = if which == 0 { unsafe { c_cn() } } else { unsafe { r_cn() } };
            let rc = if which == 0 {
                unsafe { c_sp(z, C_COMPRESSIONLEVEL, 6) }
            } else {
                unsafe { r_sp(z, C_COMPRESSIONLEVEL, 6) }
            };
            rets[which].push(rc);
            let mut obuf = vec![0u8; out_chunk];
            let mut collected = Vec::new();
            let mut src_pos = 0usize;
            let mut avail = 0usize;
            let mut prev = 0usize;
            loop {
                if prev == 0 && src_pos == avail && avail < src.len() {
                    avail = (avail + in_chunk).min(src.len());
                }
                let end = if avail == src.len() { 2 } else { 0 };
                let mut dst_pos = 0usize;
                let mut sp = src_pos;
                let rc = if which == 0 {
                    unsafe {
                        c_ca(z, obuf.as_mut_ptr(), out_chunk, &mut dst_pos, src.as_ptr(), avail,
                             &mut sp, end)
                    }
                } else {
                    unsafe {
                        r_ca(z, obuf.as_mut_ptr(), out_chunk, &mut dst_pos, src.as_ptr(), avail,
                             &mut sp, end)
                    }
                };
                rets[which].push(rc);
                rets[which].push(dst_pos);
                rets[which].push(sp);
                assert!(!is_error(rc), "{ctx}: compress simpleArgs err {}", err_code(rc));
                collected.extend_from_slice(&obuf[..dst_pos]);
                prev = if sp < avail { 1 } else { rc };
                src_pos = sp;
                if end == 2 && rc == 0 && sp == src.len() {
                    break;
                }
            }
            frames[which] = collected;
            if which == 0 {
                unsafe { c_cf(z) };
            } else {
                unsafe { r_cf(z) };
            }
        }
        assert_eq!(rets[0], rets[1], "{ctx}: compress return sequence");
        assert_bytes_eq(&ctx, &frames[0], &frames[1]);

        // decompress with simpleArgs
        let frame = &frames[0];
        let mut douts: [Vec<u8>; 2] = [Vec::new(), Vec::new()];
        let mut drets: [Vec<usize>; 2] = [Vec::new(), Vec::new()];
        for which in 0..2 {
            let z = if which == 0 { unsafe { c_dn() } } else { unsafe { r_dn() } };
            let mut obuf = vec![0u8; out_chunk];
            let mut collected = Vec::new();
            let mut ipos = 0usize;
            let mut avail = 0usize;
            loop {
                if avail < frame.len() {
                    avail = (avail + in_chunk).min(frame.len());
                }
                let mut dst_pos = 0usize;
                let mut sp = ipos;
                let rc = if which == 0 {
                    unsafe {
                        c_da(z, obuf.as_mut_ptr(), out_chunk, &mut dst_pos, frame.as_ptr(), avail,
                             &mut sp)
                    }
                } else {
                    unsafe {
                        r_da(z, obuf.as_mut_ptr(), out_chunk, &mut dst_pos, frame.as_ptr(), avail,
                             &mut sp)
                    }
                };
                drets[which].push(rc);
                drets[which].push(dst_pos);
                drets[which].push(sp);
                collected.extend_from_slice(&obuf[..dst_pos]);
                ipos = sp;
                if is_error(rc) || (rc == 0 && ipos == frame.len()) {
                    break;
                }
            }
            douts[which] = collected;
            if which == 0 {
                unsafe { c_df(z) };
            } else {
                unsafe { r_df(z) };
            }
        }
        assert_eq!(drets[0], drets[1], "{ctx}: decompress return sequence");
        assert_bytes_eq(&format!("{ctx} dec"), &douts[0], &douts[1]);
        assert_bytes_eq(&format!("{ctx} dec vs orig"), &douts[0], &src);
    }
}

/// Rows 90-93: the old initCStream_* variants and resetCStream.
#[test]
fn cfg_old_cstream_init_variants() {
    let (c_new, r_new) = unsafe { pair::<FnNew>("ZSTD_createCStream") };
    let (c_free, r_free) = unsafe { pair::<FnFree>("ZSTD_freeCStream") };
    let (c_iss, r_iss) = unsafe { pair::<FnInitCStreamSrcSize>("ZSTD_initCStream_srcSize") };
    let (c_ia, r_ia) = unsafe { pair::<FnInitCStreamAdv>("ZSTD_initCStream_advanced") };
    let (c_id, r_id) = unsafe { pair::<FnInitCStreamDict>("ZSTD_initCStream_usingDict") };
    let (c_ic, r_ic) = unsafe { pair::<FnInitCStreamCDict>("ZSTD_initCStream_usingCDict") };
    let (c_ica, r_ica) =
        unsafe { pair::<FnInitCStreamCDictAdv>("ZSTD_initCStream_usingCDict_advanced") };
    let (c_rs, r_rs) = unsafe { pair::<FnResetCStream>("ZSTD_resetCStream") };
    let (c_cs, r_cs) = unsafe { pair::<FnStream>("ZSTD_compressStream") };
    let (c_end, r_end) =
        unsafe { pair::<unsafe extern "C" fn(*mut c_void, *mut OutBuffer) -> usize>("ZSTD_endStream") };
    let (c_cd, r_cd) = unsafe { pair::<FnCreateCDict>("ZSTD_createCDict") };
    let (c_fd, r_fd) = unsafe { pair::<FnFreeDict>("ZSTD_freeCDict") };
    let (c_gpar, r_gpar) = unsafe { pair::<FnGetParams>("ZSTD_getParams") };

    let mut rng = Rng::new(0xCCC3);
    let dict = gen(Shape::Text, 8192, &mut rng);
    let ccd = unsafe { c_cd(dict.as_ptr(), dict.len(), 6) };
    let rcd = unsafe { r_cd(dict.as_ptr(), dict.len(), 6) };

    let cz = unsafe { c_new() };
    let rz = unsafe { r_new() };
    for mode in 0..6 {
        for round in 0..3 {
            let src = gen(Shape::Text, rng.range(0, 120000), &mut rng);
            let in_chunk = rng.range(1, 30000);
            let out_chunk = rng.range(1, 30000);
            let ctx = format!("oldinit mode={mode} round={round} n={}", src.len());
            let mut outs: [Vec<u8>; 2] = [Vec::new(), Vec::new()];
            let mut rets: [Vec<usize>; 2] = [Vec::new(), Vec::new()];
            for which in 0..2 {
                let z = if which == 0 { cz } else { rz };
                let rc = unsafe {
                    match (mode, which) {
                        (0, 0) => c_iss(z, 7, src.len() as u64),
                        (0, _) => r_iss(z, 7, src.len() as u64),
                        (1, 0) => {
                            let mut p = c_gpar(9, src.len() as u64, 0);
                            p.fParams.checksumFlag = 1;
                            p.fParams.contentSizeFlag = 1;
                            c_ia(z, std::ptr::null(), 0, p, src.len() as u64)
                        }
                        (1, _) => {
                            let mut p = r_gpar(9, src.len() as u64, 0);
                            p.fParams.checksumFlag = 1;
                            p.fParams.contentSizeFlag = 1;
                            r_ia(z, std::ptr::null(), 0, p, src.len() as u64)
                        }
                        (2, 0) => c_id(z, dict.as_ptr(), dict.len(), 6),
                        (2, _) => r_id(z, dict.as_ptr(), dict.len(), 6),
                        (3, 0) => c_ic(z, ccd),
                        (3, _) => r_ic(z, rcd),
                        (4, 0) => c_ica(
                            z,
                            ccd,
                            FParams { contentSizeFlag: 1, checksumFlag: 1, noDictIDFlag: 0 },
                            src.len() as u64,
                        ),
                        (4, _) => r_ica(
                            z,
                            rcd,
                            FParams { contentSizeFlag: 1, checksumFlag: 1, noDictIDFlag: 0 },
                            src.len() as u64,
                        ),
                        (_, 0) => c_rs(z, src.len() as u64),
                        (_, _) => r_rs(z, src.len() as u64),
                    }
                };
                rets[which].push(rc);
                if is_error(rc) {
                    continue;
                }
                let mut input = InBuffer { src: src.as_ptr(), size: 0, pos: 0 };
                let mut obuf = vec![0u8; out_chunk];
                let mut collected = Vec::new();
                let mut avail = 0usize;
                while avail < src.len() || input.pos < input.size {
                    if input.pos == input.size && avail < src.len() {
                        avail = (avail + in_chunk).min(src.len());
                        input.size = avail;
                    }
                    let mut out = OutBuffer { dst: obuf.as_mut_ptr(), size: out_chunk, pos: 0 };
                    let rc = if which == 0 {
                        unsafe { c_cs(z, &mut out, &mut input) }
                    } else {
                        unsafe { r_cs(z, &mut out, &mut input) }
                    };
                    rets[which].push(rc);
                    assert!(!is_error(rc), "{ctx}: compressStream err {}", err_code(rc));
                    collected.extend_from_slice(&obuf[..out.pos]);
                }
                loop {
                    let mut out = OutBuffer { dst: obuf.as_mut_ptr(), size: out_chunk, pos: 0 };
                    let rc = if which == 0 {
                        unsafe { c_end(z, &mut out) }
                    } else {
                        unsafe { r_end(z, &mut out) }
                    };
                    rets[which].push(rc);
                    assert!(!is_error(rc));
                    collected.extend_from_slice(&obuf[..out.pos]);
                    if rc == 0 {
                        break;
                    }
                }
                outs[which] = collected;
            }
            assert_eq!(rets[0], rets[1], "{ctx}: return sequence");
            assert_bytes_eq(&ctx, &outs[0], &outs[1]);
            if !outs[0].is_empty() && mode != 2 && mode != 3 && mode != 4 {
                decompress_diff(&outs[0], Some(&src), &[], &ctx);
            }
        }
    }
    unsafe {
        c_free(cz);
        r_free(rz);
        c_fd(ccd);
        r_fd(rcd);
    }
}

/// Rows 97-98: initDStream_usingDict / _usingDDict / resetDStream / disableHuffmanAssembly.
#[test]
fn cfg_dstream_init_variants() {
    let (c_new, r_new) = unsafe { pair::<FnNew>("ZSTD_createDStream") };
    let (c_free, r_free) = unsafe { pair::<FnFree>("ZSTD_freeDStream") };
    let (c_idd, r_idd) = unsafe { pair::<FnInitDStreamDict>("ZSTD_initDStream_usingDict") };
    let (c_iddd, r_iddd) = unsafe { pair::<FnInitDStreamDDict>("ZSTD_initDStream_usingDDict") };
    let (c_rst, r_rst) = unsafe { pair::<FnResetCStream>("ZSTD_resetDStream") };
    let (c_ds, r_ds) = unsafe { pair::<FnStream>("ZSTD_decompressStream") };
    let (c_cdd, r_cdd) = unsafe { pair::<FnCreateDDict>("ZSTD_createDDict") };
    let (c_fdd, r_fdd) = unsafe { pair::<FnFreeDict>("ZSTD_freeDDict") };
    let (c_sp, r_sp) = unsafe { pair::<FnSetParam>("ZSTD_DCtx_setParameter") };
    let (c_ld, r_ld) = unsafe { pair::<FnInitDStreamDict>("ZSTD_CCtx_loadDictionary") };
    let (c_cn, r_cn) = unsafe { pair::<FnNew>("ZSTD_createCCtx") };
    let (c_cf, r_cf) = unsafe { pair::<FnFree>("ZSTD_freeCCtx") };
    let (c_c2, r_c2) = unsafe { pair::<FnCompress2>("ZSTD_compress2") };
    let (cb, _) = unsafe { pair::<FnBound>("ZSTD_compressBound") };

    let mut rng = Rng::new(0xDDD4);
    let dict = gen(Shape::Text, 8192, &mut rng);
    let src = gen(Shape::Text, 90000, &mut rng);

    // dict-compressed frame produced identically by both libs
    let cap = unsafe { cb(src.len()) } + 64;
    let mut cbuf = vec![0u8; cap];
    let mut rbuf = vec![0u8; cap];
    let cc = unsafe { c_cn() };
    let rc = unsafe { r_cn() };
    assert_eq!(unsafe { c_ld(cc, dict.as_ptr(), dict.len()) },
               unsafe { r_ld(rc, dict.as_ptr(), dict.len()) });
    let a = unsafe { c_c2(cc, cbuf.as_mut_ptr(), cap, src.as_ptr(), src.len()) };
    let b = unsafe { r_c2(rc, rbuf.as_mut_ptr(), cap, src.as_ptr(), src.len()) };
    assert_eq!(a, b);
    assert_bytes_eq("dict frame", &cbuf[..a], &rbuf[..b]);
    let frame = cbuf[..a].to_vec();
    unsafe {
        c_cf(cc);
        r_cf(rc);
    }

    let cdd = unsafe { c_cdd(dict.as_ptr(), dict.len()) };
    let rdd = unsafe { r_cdd(dict.as_ptr(), dict.len()) };
    let cz = unsafe { c_new() };
    let rz = unsafe { r_new() };
    for mode in 0..3 {
        for &dishuf in &[0, 1] {
            for &out_chunk in &[1usize, 1000, 70000] {
                let ctx = format!("dstream mode={mode} dishuf={dishuf} out={out_chunk}");
                let mut outs: [Vec<u8>; 2] = [Vec::new(), Vec::new()];
                let mut rets: [Vec<usize>; 2] = [Vec::new(), Vec::new()];
                for which in 0..2 {
                    let z = if which == 0 { cz } else { rz };
                    let rc = unsafe {
                        match (mode, which) {
                            (0, 0) => c_idd(z, dict.as_ptr(), dict.len()),
                            (0, _) => r_idd(z, dict.as_ptr(), dict.len()),
                            (1, 0) => c_iddd(z, cdd),
                            (1, _) => r_iddd(z, rdd),
                            (_, 0) => {
                                let r = c_rst(z, 0);
                                let _ = c_idd(z, dict.as_ptr(), dict.len());
                                r
                            }
                            (_, _) => {
                                let r = r_rst(z, 0);
                                let _ = r_idd(z, dict.as_ptr(), dict.len());
                                r
                            }
                        }
                    };
                    rets[which].push(rc);
                    let sp = if which == 0 { &c_sp } else { &r_sp };
                    rets[which].push(unsafe { sp(z, 1004, dishuf) });
                    let mut obuf = vec![0u8; out_chunk];
                    let mut input = InBuffer { src: frame.as_ptr(), size: frame.len(), pos: 0 };
                    let mut collected = Vec::new();
                    loop {
                        let mut out =
                            OutBuffer { dst: obuf.as_mut_ptr(), size: out_chunk, pos: 0 };
                        let rc = if which == 0 {
                            unsafe { c_ds(z, &mut out, &mut input) }
                        } else {
                            unsafe { r_ds(z, &mut out, &mut input) }
                        };
                        rets[which].push(rc);
                        collected.extend_from_slice(&obuf[..out.pos]);
                        if is_error(rc) || rc == 0 {
                            break;
                        }
                    }
                    outs[which] = collected;
                }
                assert_eq!(rets[0], rets[1], "{ctx}: return sequence");
                assert_bytes_eq(&ctx, &outs[0], &outs[1]);
            }
        }
    }
    unsafe {
        c_free(cz);
        r_free(rz);
        c_fdd(cdd);
        r_fdd(rdd);
    }
}

/// Row 96: d_stableOutBuffer valid usage (exact-size buffer, known FCS) and the
/// unknown-FCS shape.
#[test]
fn cfg_dstream_stable_out() {
    let (cn, rn) = unsafe { pair::<FnNew>("ZSTD_createDCtx") };
    let (cf, rf) = unsafe { pair::<FnFree>("ZSTD_freeDCtx") };
    let (cs, rs) = unsafe { pair::<FnSetParam>("ZSTD_DCtx_setParameter") };
    let (cd, rd) = unsafe { pair::<FnStream>("ZSTD_decompressStream") };
    let (ccc, rcc) = unsafe { pair::<FnCompress>("ZSTD_compress") };
    let (cb, _) = unsafe { pair::<FnBound>("ZSTD_compressBound") };
    let mut rng = Rng::new(0xEEE5);

    for &known_fcs in &[true, false] {
        for &shape in &[Shape::Text, Shape::Random] {
            let src = gen(shape, rng.range(1000, 200000), &mut rng);
            let frame = if known_fcs {
                let cap = unsafe { cb(src.len()) } + 64;
                let mut b = vec![0u8; cap];
                let mut b2 = vec![0u8; cap];
                let n = unsafe { ccc(b.as_mut_ptr(), cap, src.as_ptr(), src.len(), 5) };
                let n2 = unsafe { rcc(b2.as_mut_ptr(), cap, src.as_ptr(), src.len(), 5) };
                assert_eq!(n, n2);
                assert_bytes_eq("frame", &b[..n], &b2[..n2]);
                b.truncate(n);
                b
            } else {
                // stream without pledged size => no FCS in header
                let f = diff2(&[(C_COMPRESSIONLEVEL, 5)], None, &src, &[], "nofcs");
                f.unwrap()
            };
            let ctx = format!("d_stableOut known={known_fcs} shape={shape:?} n={}", src.len());
            let mut outs: [Vec<u8>; 2] = [Vec::new(), Vec::new()];
            let mut rets: [Vec<usize>; 2] = [Vec::new(), Vec::new()];
            for which in 0..2 {
                let z = if which == 0 { unsafe { cn() } } else { unsafe { rn() } };
                let sp = if which == 0 { &cs } else { &rs };
                rets[which].push(unsafe { sp(z, 1001, 1) });
                let mut obuf = vec![0u8; src.len() + 16];
                let mut input = InBuffer { src: frame.as_ptr(), size: frame.len(), pos: 0 };
                let mut out =
                    OutBuffer { dst: obuf.as_mut_ptr(), size: obuf.len(), pos: 0 };
                loop {
                    let rc = if which == 0 {
                        unsafe { cd(z, &mut out, &mut input) }
                    } else {
                        unsafe { rd(z, &mut out, &mut input) }
                    };
                    rets[which].push(rc);
                    if is_error(rc) || rc == 0 {
                        break;
                    }
                }
                outs[which] = obuf[..out.pos].to_vec();
                if which == 0 {
                    unsafe { cf(z) };
                } else {
                    unsafe { rf(z) };
                }
            }
            assert_eq!(rets[0], rets[1], "{ctx}: return sequence");
            assert_bytes_eq(&ctx, &outs[0], &outs[1]);
        }
    }
}
