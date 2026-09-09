//! Phase B: advanced API — ZSTD_CCtx_setParameter / ZSTD_compress2 /
//! ZSTD_DCtx_setParameter across every documented parameter and its bounds.
#![allow(non_snake_case, dead_code)]
mod common;
use common::*;
use std::os::raw::{c_int, c_void};

pub type FnNew = unsafe extern "C" fn() -> *mut c_void;
pub type FnFree = unsafe extern "C" fn(*mut c_void) -> usize;
pub type FnSetParam = unsafe extern "C" fn(*mut c_void, c_int, c_int) -> usize;
pub type FnGetParam = unsafe extern "C" fn(*mut c_void, c_int, *mut c_int) -> usize;
pub type FnCompress2 = unsafe extern "C" fn(*mut c_void, *mut u8, usize, *const u8, usize) -> usize;
pub type FnDecompressDCtx =
    unsafe extern "C" fn(*mut c_void, *mut u8, usize, *const u8, usize) -> usize;
pub type FnBound = unsafe extern "C" fn(usize) -> usize;
pub type FnReset = unsafe extern "C" fn(*mut c_void, c_int) -> usize;
pub type FnPledged = unsafe extern "C" fn(*mut c_void, u64) -> usize;
pub type FnGetBounds = unsafe extern "C" fn(c_int) -> Bounds;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Bounds {
    pub error: usize,
    pub lower_bound: c_int,
    pub upper_bound: c_int,
}

// cParameters
pub const C_COMPRESSIONLEVEL: c_int = 100;
pub const C_WINDOWLOG: c_int = 101;
pub const C_HASHLOG: c_int = 102;
pub const C_CHAINLOG: c_int = 103;
pub const C_SEARCHLOG: c_int = 104;
pub const C_MINMATCH: c_int = 105;
pub const C_TARGETLENGTH: c_int = 106;
pub const C_STRATEGY: c_int = 107;
pub const C_TARGETCBLOCKSIZE: c_int = 130;
pub const C_ENABLELDM: c_int = 160;
pub const C_LDMHASHLOG: c_int = 161;
pub const C_LDMMINMATCH: c_int = 162;
pub const C_LDMBUCKETSIZELOG: c_int = 163;
pub const C_LDMHASHRATELOG: c_int = 164;
pub const C_CONTENTSIZEFLAG: c_int = 200;
pub const C_CHECKSUMFLAG: c_int = 201;
pub const C_DICTIDFLAG: c_int = 202;
pub const C_NBWORKERS: c_int = 400;
pub const C_JOBSIZE: c_int = 401;
pub const C_OVERLAPLOG: c_int = 402;
pub const C_RSYNCABLE: c_int = 500; // experimentalParam1
pub const C_FORMAT: c_int = 10; // experimentalParam2
pub const C_FORCEMAXWINDOW: c_int = 1000;
pub const C_FORCEATTACHDICT: c_int = 1001;
pub const C_LITERALCOMPRESSIONMODE: c_int = 1002;
pub const C_SRCSIZEHINT: c_int = 1004;
pub const C_ENABLEDEDICATEDDICTSEARCH: c_int = 1005;
pub const C_STABLEINBUFFER: c_int = 1006;
pub const C_STABLEOUTBUFFER: c_int = 1007;
pub const C_BLOCKDELIMITERS: c_int = 1008;
pub const C_VALIDATESEQUENCES: c_int = 1009;
pub const C_SPLITAFTERSEQUENCES: c_int = 1010;
pub const C_USEROWMATCHFINDER: c_int = 1011;
pub const C_DETERMINISTICREFPREFIX: c_int = 1012;
pub const C_PREFETCHCDICTTABLES: c_int = 1013;
pub const C_ENABLESEQPRODUCERFALLBACK: c_int = 1014;
pub const C_MAXBLOCKSIZE: c_int = 1015;
pub const C_REPCODERESOLUTION: c_int = 1016;
pub const C_BLOCKSPLITTERLEVEL: c_int = 1017;

pub const ALL_CPARAMS: &[(c_int, &str)] = &[
    (C_COMPRESSIONLEVEL, "compressionLevel"),
    (C_WINDOWLOG, "windowLog"),
    (C_HASHLOG, "hashLog"),
    (C_CHAINLOG, "chainLog"),
    (C_SEARCHLOG, "searchLog"),
    (C_MINMATCH, "minMatch"),
    (C_TARGETLENGTH, "targetLength"),
    (C_STRATEGY, "strategy"),
    (C_TARGETCBLOCKSIZE, "targetCBlockSize"),
    (C_ENABLELDM, "enableLongDistanceMatching"),
    (C_LDMHASHLOG, "ldmHashLog"),
    (C_LDMMINMATCH, "ldmMinMatch"),
    (C_LDMBUCKETSIZELOG, "ldmBucketSizeLog"),
    (C_LDMHASHRATELOG, "ldmHashRateLog"),
    (C_CONTENTSIZEFLAG, "contentSizeFlag"),
    (C_CHECKSUMFLAG, "checksumFlag"),
    (C_DICTIDFLAG, "dictIDFlag"),
    (C_NBWORKERS, "nbWorkers"),
    (C_JOBSIZE, "jobSize"),
    (C_OVERLAPLOG, "overlapLog"),
    (C_RSYNCABLE, "rsyncable"),
    (C_FORMAT, "format"),
    (C_FORCEMAXWINDOW, "forceMaxWindow"),
    (C_FORCEATTACHDICT, "forceAttachDict"),
    (C_LITERALCOMPRESSIONMODE, "literalCompressionMode"),
    (C_SRCSIZEHINT, "srcSizeHint"),
    (C_ENABLEDEDICATEDDICTSEARCH, "enableDedicatedDictSearch"),
    (C_STABLEINBUFFER, "stableInBuffer"),
    (C_STABLEOUTBUFFER, "stableOutBuffer"),
    (C_BLOCKDELIMITERS, "blockDelimiters"),
    (C_VALIDATESEQUENCES, "validateSequences"),
    (C_SPLITAFTERSEQUENCES, "splitAfterSequences"),
    (C_USEROWMATCHFINDER, "useRowMatchFinder"),
    (C_DETERMINISTICREFPREFIX, "deterministicRefPrefix"),
    (C_PREFETCHCDICTTABLES, "prefetchCDictTables"),
    (C_ENABLESEQPRODUCERFALLBACK, "enableSeqProducerFallback"),
    (C_MAXBLOCKSIZE, "maxBlockSize"),
    (C_REPCODERESOLUTION, "repcodeResolution"),
    (C_BLOCKSPLITTERLEVEL, "blockSplitterLevel"),
];

// dParameters
pub const D_WINDOWLOGMAX: c_int = 100;
pub const D_FORMAT: c_int = 1000;
pub const D_STABLEOUTBUFFER: c_int = 1001;
pub const D_FORCEIGNORECHECKSUM: c_int = 1002;
pub const D_REFMULTIPLEDDICTS: c_int = 1003;
pub const D_DISABLEHUFFMANASSEMBLY: c_int = 1004;
pub const D_MAXBLOCKSIZE: c_int = 1005;

pub const ALL_DPARAMS: &[(c_int, &str)] = &[
    (D_WINDOWLOGMAX, "windowLogMax"),
    (D_FORMAT, "format"),
    (D_STABLEOUTBUFFER, "stableOutBuffer"),
    (D_FORCEIGNORECHECKSUM, "forceIgnoreChecksum"),
    (D_REFMULTIPLEDDICTS, "refMultipleDDicts"),
    (D_DISABLEHUFFMANASSEMBLY, "disableHuffmanAssembly"),
    (D_MAXBLOCKSIZE, "maxBlockSize"),
];

pub const RESET_SESSION_ONLY: c_int = 1;
pub const RESET_PARAMETERS: c_int = 2;
pub const RESET_SESSION_AND_PARAMETERS: c_int = 3;

#[test]
fn cfg_cParam_getBounds_all() {
    let (cg, rg) = unsafe { pair::<FnGetBounds>("ZSTD_cParam_getBounds") };
    for &(p, name) in ALL_CPARAMS {
        let c = unsafe { cg(p) };
        let r = unsafe { rg(p) };
        assert_eq!(c, r, "ZSTD_cParam_getBounds({name}={p})");
        assert_eq!(c.error, 0, "C bounds error for {name}");
    }
    // unknown / out-of-range parameter values (C enums accept any int)
    for p in [-1000, -1, 0, 1, 9, 11, 99, 108, 129, 131, 165, 203, 403, 499, 501, 999, 1003, 1018,
        1019, 100000, c_int::MAX, c_int::MIN] {
        let c = unsafe { cg(p) };
        let r = unsafe { rg(p) };
        assert_eq!(c, r, "ZSTD_cParam_getBounds(bogus {p})");
    }
}

#[test]
fn cfg_dParam_getBounds_all() {
    let (cg, rg) = unsafe { pair::<FnGetBounds>("ZSTD_dParam_getBounds") };
    for &(p, name) in ALL_DPARAMS {
        let c = unsafe { cg(p) };
        let r = unsafe { rg(p) };
        assert_eq!(c, r, "ZSTD_dParam_getBounds({name}={p})");
        assert_eq!(c.error, 0, "C bounds error for {name}");
    }
    for p in [-1, 0, 1, 99, 101, 999, 1006, 1007, 100000, c_int::MAX, c_int::MIN] {
        let c = unsafe { cg(p) };
        let r = unsafe { rg(p) };
        assert_eq!(c, r, "ZSTD_dParam_getBounds(bogus {p})");
    }
}

struct CCtxPair {
    c: *mut c_void,
    r: *mut c_void,
    c_set: libloading::Symbol<'static, FnSetParam>,
    r_set: libloading::Symbol<'static, FnSetParam>,
    c_get: libloading::Symbol<'static, FnGetParam>,
    r_get: libloading::Symbol<'static, FnGetParam>,
    c_c2: libloading::Symbol<'static, FnCompress2>,
    r_c2: libloading::Symbol<'static, FnCompress2>,
    c_reset: libloading::Symbol<'static, FnReset>,
    r_reset: libloading::Symbol<'static, FnReset>,
    c_pledge: libloading::Symbol<'static, FnPledged>,
    r_pledge: libloading::Symbol<'static, FnPledged>,
    c_free: libloading::Symbol<'static, FnFree>,
    r_free: libloading::Symbol<'static, FnFree>,
}

impl CCtxPair {
    fn new() -> CCtxPair {
        let (cn, rn) = unsafe { pair::<FnNew>("ZSTD_createCCtx") };
        let (c_set, r_set) = unsafe { pair::<FnSetParam>("ZSTD_CCtx_setParameter") };
        let (c_get, r_get) = unsafe { pair::<FnGetParam>("ZSTD_CCtx_getParameter") };
        let (c_c2, r_c2) = unsafe { pair::<FnCompress2>("ZSTD_compress2") };
        let (c_reset, r_reset) = unsafe { pair::<FnReset>("ZSTD_CCtx_reset") };
        let (c_pledge, r_pledge) = unsafe { pair::<FnPledged>("ZSTD_CCtx_setPledgedSrcSize") };
        let (c_free, r_free) = unsafe { pair::<FnFree>("ZSTD_freeCCtx") };
        let c = unsafe { cn() };
        let r = unsafe { rn() };
        assert!(!c.is_null() && !r.is_null());
        CCtxPair { c, r, c_set, r_set, c_get, r_get, c_c2, r_c2, c_reset, r_reset, c_pledge,
            r_pledge, c_free, r_free }
    }

    fn set(&self, p: c_int, v: c_int, ctx: &str) -> usize {
        let a = unsafe { (self.c_set)(self.c, p, v) };
        let b = unsafe { (self.r_set)(self.r, p, v) };
        assert_eq!(a, b, "{ctx}: setParameter({p},{v}) return");
        // and the read-back value must match
        let mut av = 0;
        let mut bv = 0;
        let ga = unsafe { (self.c_get)(self.c, p, &mut av) };
        let gb = unsafe { (self.r_get)(self.r, p, &mut bv) };
        assert_eq!(ga, gb, "{ctx}: getParameter({p}) return");
        if !is_error(ga) {
            assert_eq!(av, bv, "{ctx}: getParameter({p}) value");
        }
        a
    }

    fn reset(&self, kind: c_int) {
        let a = unsafe { (self.c_reset)(self.c, kind) };
        let b = unsafe { (self.r_reset)(self.r, kind) };
        assert_eq!(a, b, "CCtx_reset({kind})");
    }

    fn pledge(&self, n: u64) -> usize {
        let a = unsafe { (self.c_pledge)(self.c, n) };
        let b = unsafe { (self.r_pledge)(self.r, n) };
        assert_eq!(a, b, "setPledgedSrcSize({n})");
        a
    }

    /// compress2 in both, requiring identical return and bytes
    fn compress2(&self, src: &[u8], cap: usize, ctx: &str) -> (usize, Vec<u8>) {
        let mut cbuf = vec![0xAAu8; cap];
        let mut rbuf = vec![0x55u8; cap];
        let a = unsafe { (self.c_c2)(self.c, cbuf.as_mut_ptr(), cap, src.as_ptr(), src.len()) };
        let b = unsafe { (self.r_c2)(self.r, rbuf.as_mut_ptr(), cap, src.as_ptr(), src.len()) };
        assert_eq!(a, b, "{ctx}: compress2 return (C err={})", err_code(a));
        if !is_error(a) {
            assert_bytes_eq(&format!("{ctx}: compress2 bytes"), &cbuf[..a], &rbuf[..a]);
            cbuf.truncate(a);
        }
        (a, cbuf)
    }
}

impl Drop for CCtxPair {
    fn drop(&mut self) {
        unsafe {
            (self.c_free)(self.c);
            (self.r_free)(self.r);
        }
    }
}

fn dctx_decompress_diff(frame: &[u8], expect: Option<&[u8]>, dparams: &[(c_int, c_int)], ctx: &str) {
    let (cn, rn) = unsafe { pair::<FnNew>("ZSTD_createDCtx") };
    let (cf, rf) = unsafe { pair::<FnFree>("ZSTD_freeDCtx") };
    let (cs, rs) = unsafe { pair::<FnSetParam>("ZSTD_DCtx_setParameter") };
    let (cd, rd) = unsafe { pair::<FnDecompressDCtx>("ZSTD_decompressDCtx") };
    let c = unsafe { cn() };
    let r = unsafe { rn() };
    for &(p, v) in dparams {
        let a = unsafe { cs(c, p, v) };
        let b = unsafe { rs(r, p, v) };
        assert_eq!(a, b, "{ctx}: DCtx_setParameter({p},{v})");
    }
    let cap = expect.map(|e| e.len()).unwrap_or(1 << 20) + 64;
    let mut co = vec![0u8; cap];
    let mut ro = vec![0u8; cap];
    let a = unsafe { cd(c, co.as_mut_ptr(), cap, frame.as_ptr(), frame.len()) };
    let b = unsafe { rd(r, ro.as_mut_ptr(), cap, frame.as_ptr(), frame.len()) };
    assert_eq!(a, b, "{ctx}: decompressDCtx return (C err {})", err_code(a));
    if !is_error(a) {
        assert_bytes_eq(&format!("{ctx}: dec bytes"), &co[..a], &ro[..b]);
        if let Some(e) = expect {
            assert_bytes_eq(&format!("{ctx}: dec vs orig"), &co[..a], e);
        }
    }
    unsafe {
        cf(c);
        rf(r);
    }
}

/// For every cParameter: set it to lower bound, upper bound, mid, default(0-ish)
/// and compress several inputs, comparing bytes.
#[test]
fn cfg_every_cparam_at_bounds() {
    let (cg, _) = unsafe { pair::<FnGetBounds>("ZSTD_cParam_getBounds") };
    let (cb, _) = unsafe { pair::<FnBound>("ZSTD_compressBound") };
    let mut rng = Rng::new(0xBEEF);
    for &(p, name) in ALL_CPARAMS {
        let b = unsafe { cg(p) };
        assert_eq!(b.error, 0);
        let mut vals = vec![b.lower_bound, b.upper_bound];
        if b.upper_bound > b.lower_bound {
            vals.push(b.lower_bound + (b.upper_bound - b.lower_bound) / 2);
            vals.push(b.lower_bound.saturating_add(1));
            vals.push(b.upper_bound - 1);
        }
        vals.dedup();
        for &v in &vals {
            // limit crazy-large values that would allocate GBs
            if (p == C_WINDOWLOG || p == C_LDMHASHLOG || p == C_HASHLOG || p == C_CHAINLOG)
                && v > 27
            {
                continue;
            }
            if p == C_NBWORKERS && v > 4 {
                continue;
            }
            if p == C_JOBSIZE && v > (1 << 22) {
                continue;
            }
            if p == C_SRCSIZEHINT && v > (1 << 24) {
                continue;
            }
            if p == C_TARGETCBLOCKSIZE && v > (1 << 20) {
                continue;
            }
            if p == C_MAXBLOCKSIZE && v > (1 << 18) {
                continue;
            }
            for &(shape, size) in &[
                (Shape::Text, 3000usize),
                (Shape::Random, 5000),
                (Shape::Rle, 200000),
                (Shape::LongMatches, 150000),
            ] {
                let src = gen(shape, size, &mut rng);
                let pair_ctx = CCtxPair::new();
                let ctx = format!("cparam {name}({p})={v} shape={shape:?} size={size}");
                let rc = pair_ctx.set(p, v, &ctx);
                if is_error(rc) {
                    continue;
                }
                let cap = unsafe { cb(src.len()) } + 64;
                let (n, frame) = pair_ctx.compress2(&src, cap, &ctx);
                if !is_error(n) {
                    let dp: Vec<(c_int, c_int)> = if p == C_FORMAT && v == 1 {
                        vec![(D_FORMAT, 1)]
                    } else {
                        vec![(D_WINDOWLOGMAX, 31)]
                    };
                    dctx_decompress_diff(&frame, Some(&src), &dp, &ctx);
                }
            }
        }
    }
}

/// Cross-product of the "shape of output" flags with input shapes.
#[test]
fn cfg_flag_combinations() {
    let (cb, _) = unsafe { pair::<FnBound>("ZSTD_compressBound") };
    let mut rng = Rng::new(0xF00D);
    for &checksum in &[0, 1] {
        for &contentsize in &[0, 1] {
            for &dictid in &[0, 1] {
                for &format in &[0, 1] {
                    for &pledged in &[false, true] {
                        for &shape in &[Shape::Text, Shape::Random, Shape::Rle] {
                            let size = rng.range(1, 20000);
                            let src = gen(shape, size, &mut rng);
                            let p = CCtxPair::new();
                            let ctx = format!(
                                "flags cs={checksum} csz={contentsize} did={dictid} fmt={format} pledge={pledged} shape={shape:?} size={size}"
                            );
                            p.set(C_CHECKSUMFLAG, checksum, &ctx);
                            p.set(C_CONTENTSIZEFLAG, contentsize, &ctx);
                            p.set(C_DICTIDFLAG, dictid, &ctx);
                            p.set(C_FORMAT, format, &ctx);
                            if pledged {
                                p.pledge(src.len() as u64);
                            }
                            let cap = unsafe { cb(src.len()) } + 64;
                            let (n, frame) = p.compress2(&src, cap, &ctx);
                            assert!(!is_error(n), "{ctx}: {}", err_code(n));
                            let dp = if format == 1 {
                                vec![(D_FORMAT, 1)]
                            } else {
                                vec![]
                            };
                            dctx_decompress_diff(&frame, Some(&src), &dp, &ctx);
                        }
                    }
                }
            }
        }
    }
}

/// All 9 strategies x window/hash/chain/search/minMatch/targetLength variation.
#[test]
fn cfg_all_strategies_manual_params() {
    let (cb, _) = unsafe { pair::<FnBound>("ZSTD_compressBound") };
    let mut rng = Rng::new(0x5721);
    for strat in 1..=9 {
        for &(wlog, hlog, clog, slog, mm, tlen) in &[
            (10, 10, 10, 1, 3, 0),
            (17, 17, 17, 4, 4, 32),
            (20, 18, 19, 6, 5, 999),
            (23, 22, 23, 8, 7, 4096),
        ] {
            for &shape in &[Shape::Text, Shape::Random, Shape::LongMatches, Shape::Mixed] {
                let size = rng.range(1000, 200000);
                let src = gen(shape, size, &mut rng);
                let p = CCtxPair::new();
                let ctx = format!(
                    "strategy={strat} w={wlog} h={hlog} c={clog} s={slog} mm={mm} t={tlen} shape={shape:?} size={size}"
                );
                p.set(C_STRATEGY, strat, &ctx);
                p.set(C_WINDOWLOG, wlog, &ctx);
                p.set(C_HASHLOG, hlog, &ctx);
                let e = p.set(C_CHAINLOG, clog, &ctx);
                let _ = e;
                p.set(C_SEARCHLOG, slog, &ctx);
                p.set(C_MINMATCH, mm, &ctx);
                p.set(C_TARGETLENGTH, tlen, &ctx);
                let cap = unsafe { cb(src.len()) } + 64;
                let (n, frame) = p.compress2(&src, cap, &ctx);
                if !is_error(n) {
                    dctx_decompress_diff(&frame, Some(&src), &[(D_WINDOWLOGMAX, 31)], &ctx);
                }
            }
        }
    }
}

/// Long-distance matching parameter matrix.
#[test]
fn cfg_ldm_matrix() {
    let (cb, _) = unsafe { pair::<FnBound>("ZSTD_compressBound") };
    let mut rng = Rng::new(0x1D1D);
    for &enable in &[0, 1] {
        for &(hlog, mm, bslog, hrlog) in
            &[(0, 0, 0, 0), (10, 32, 1, 0), (20, 64, 4, 7), (24, 4096, 8, 1)]
        {
            for &wlog in &[17, 20, 25] {
                for &shape in &[Shape::LongMatches, Shape::Repetitive, Shape::Random] {
                    let size = rng.range(100000, 400000);
                    let src = gen(shape, size, &mut rng);
                    let p = CCtxPair::new();
                    let ctx = format!(
                        "ldm en={enable} h={hlog} mm={mm} b={bslog} r={hrlog} w={wlog} shape={shape:?} size={size}"
                    );
                    p.set(C_ENABLELDM, enable, &ctx);
                    p.set(C_WINDOWLOG, wlog, &ctx);
                    if hlog != 0 {
                        p.set(C_LDMHASHLOG, hlog, &ctx);
                    }
                    if mm != 0 {
                        p.set(C_LDMMINMATCH, mm, &ctx);
                    }
                    if bslog != 0 {
                        p.set(C_LDMBUCKETSIZELOG, bslog, &ctx);
                    }
                    if hrlog != 0 {
                        p.set(C_LDMHASHRATELOG, hrlog, &ctx);
                    }
                    let cap = unsafe { cb(src.len()) } + 64;
                    let (n, frame) = p.compress2(&src, cap, &ctx);
                    if !is_error(n) {
                        dctx_decompress_diff(&frame, Some(&src), &[(D_WINDOWLOGMAX, 31)], &ctx);
                    }
                }
            }
        }
    }
}

/// Block splitter / row match finder / literal compression mode / repcode resolution.
#[test]
fn cfg_misc_experimental_matrix() {
    let (cb, _) = unsafe { pair::<FnBound>("ZSTD_compressBound") };
    let mut rng = Rng::new(0xABCD);
    for &lcm in &[0, 1, 2] {
        for &rowmf in &[0, 1, 2] {
            for &bsl in &[0, 1, 2, 3, 4] {
                for &rcr in &[0, 1, 2] {
                    for &shape in &[Shape::Text, Shape::Mixed, Shape::Random] {
                        let size = rng.range(20000, 200000);
                        let src = gen(shape, size, &mut rng);
                        let p = CCtxPair::new();
                        let ctx = format!(
                            "misc lcm={lcm} rowmf={rowmf} bsl={bsl} rcr={rcr} shape={shape:?} size={size}"
                        );
                        p.set(C_LITERALCOMPRESSIONMODE, lcm, &ctx);
                        p.set(C_USEROWMATCHFINDER, rowmf, &ctx);
                        p.set(C_BLOCKSPLITTERLEVEL, bsl, &ctx);
                        p.set(C_REPCODERESOLUTION, rcr, &ctx);
                        p.set(C_COMPRESSIONLEVEL, [1, 5, 9, 13, 19][rng.below(5)], &ctx);
                        let cap = unsafe { cb(src.len()) } + 64;
                        let (n, frame) = p.compress2(&src, cap, &ctx);
                        if !is_error(n) {
                            dctx_decompress_diff(&frame, Some(&src), &[], &ctx);
                        }
                    }
                }
            }
        }
    }
}

/// targetCBlockSize / maxBlockSize / srcSizeHint / forceMaxWindow / rsyncable /
/// deterministicRefPrefix / splitAfterSequences / prefetchCDictTables
#[test]
fn cfg_more_experimental_matrix() {
    let (cb, _) = unsafe { pair::<FnBound>("ZSTD_compressBound") };
    let mut rng = Rng::new(0xDEAD);
    for &tcbs in &[0, 1340, 65536] {
        for &mbs in &[0, 1024, 131072] {
            for &ssh in &[0, 100, 1 << 20] {
                for &fmw in &[0, 1] {
                    for &rsy in &[0, 1] {
                        for &shape in &[Shape::Text, Shape::Random] {
                            let size = rng.range(50000, 300000);
                            let src = gen(shape, size, &mut rng);
                            let p = CCtxPair::new();
                            let ctx = format!(
                                "more tcbs={tcbs} mbs={mbs} ssh={ssh} fmw={fmw} rsy={rsy} shape={shape:?} size={size}"
                            );
                            p.set(C_TARGETCBLOCKSIZE, tcbs, &ctx);
                            p.set(C_MAXBLOCKSIZE, mbs, &ctx);
                            p.set(C_SRCSIZEHINT, ssh, &ctx);
                            p.set(C_FORCEMAXWINDOW, fmw, &ctx);
                            p.set(C_RSYNCABLE, rsy, &ctx);
                            p.set(C_SPLITAFTERSEQUENCES, (rng.next_u32() % 3) as c_int, &ctx);
                            p.set(C_DETERMINISTICREFPREFIX, (rng.next_u32() % 2) as c_int, &ctx);
                            let cap = unsafe { cb(src.len()) } + 64;
                            let (n, frame) = p.compress2(&src, cap, &ctx);
                            if !is_error(n) {
                                dctx_decompress_diff(
                                    &frame,
                                    Some(&src),
                                    &[(D_MAXBLOCKSIZE, 131072)],
                                    &ctx,
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

/// every dParameter, valid values, on a variety of frames
#[test]
fn cfg_every_dparam() {
    let (cg, _) = unsafe { pair::<FnGetBounds>("ZSTD_dParam_getBounds") };
    let (cb, _) = unsafe { pair::<FnBound>("ZSTD_compressBound") };
    let mut rng = Rng::new(0x0D0D);
    // build a set of frames: plain, with checksum, magicless, big window
    let mut frames: Vec<(String, Vec<u8>, Vec<u8>, bool)> = Vec::new();
    for &(cksum, fmt, wlog) in &[(0, 0, 17), (1, 0, 17), (0, 1, 17), (1, 1, 21), (0, 0, 25)] {
        let src = gen(Shape::Text, 250000, &mut rng);
        let p = CCtxPair::new();
        let ctx = format!("frame cksum={cksum} fmt={fmt} wlog={wlog}");
        p.set(C_CHECKSUMFLAG, cksum, &ctx);
        p.set(C_FORMAT, fmt, &ctx);
        p.set(C_WINDOWLOG, wlog, &ctx);
        let cap = unsafe { cb(src.len()) } + 64;
        let (n, frame) = p.compress2(&src, cap, &ctx);
        assert!(!is_error(n));
        frames.push((ctx, frame, src, fmt == 1));
    }
    for &(dp, name) in ALL_DPARAMS {
        let b = unsafe { cg(dp) };
        let mut vals = vec![b.lower_bound, b.upper_bound];
        if b.upper_bound > b.lower_bound {
            vals.push(b.lower_bound + (b.upper_bound - b.lower_bound) / 2);
        }
        vals.dedup();
        for &v in &vals {
            if dp == D_MAXBLOCKSIZE && v > (1 << 18) {
                continue;
            }
            for (fctx, frame, src, magicless) in &frames {
                let mut params = vec![(dp, v)];
                if *magicless && dp != D_FORMAT {
                    params.push((D_FORMAT, 1));
                }
                if dp != D_WINDOWLOGMAX {
                    params.push((D_WINDOWLOGMAX, 31));
                }
                let ctx = format!("dparam {name}({dp})={v} on {fctx}");
                // note: some combinations legitimately fail in C (e.g. windowLogMax too
                // small, wrong format); dctx_decompress_diff compares returns either way
                let expect: Option<&[u8]> = None;
                let _ = src;
                dctx_decompress_diff(frame, expect, &params, &ctx);
            }
        }
    }
}

#[test]
fn cfg_reset_and_pledged_sequences() {
    let (cb, _) = unsafe { pair::<FnBound>("ZSTD_compressBound") };
    let mut rng = Rng::new(0x9999);
    let p = CCtxPair::new();
    for round in 0..30 {
        let ctx = format!("reset round={round}");
        let kind = [RESET_SESSION_ONLY, RESET_PARAMETERS, RESET_SESSION_AND_PARAMETERS]
            [rng.below(3)];
        p.reset(kind);
        p.set(C_COMPRESSIONLEVEL, [1, 4, 8, 12, 19][rng.below(5)], &ctx);
        p.set(C_CHECKSUMFLAG, (rng.next_u32() % 2) as c_int, &ctx);
        let src = gen(ALL_SHAPES[rng.below(ALL_SHAPES.len())], rng.range(0, 50000), &mut rng);
        if rng.next_u32() % 2 == 0 {
            p.pledge(src.len() as u64);
        }
        let cap = unsafe { cb(src.len()) } + 64;
        let (n, frame) = p.compress2(&src, cap, &ctx);
        if !is_error(n) {
            dctx_decompress_diff(&frame, Some(&src), &[], &ctx);
        }
    }
}
