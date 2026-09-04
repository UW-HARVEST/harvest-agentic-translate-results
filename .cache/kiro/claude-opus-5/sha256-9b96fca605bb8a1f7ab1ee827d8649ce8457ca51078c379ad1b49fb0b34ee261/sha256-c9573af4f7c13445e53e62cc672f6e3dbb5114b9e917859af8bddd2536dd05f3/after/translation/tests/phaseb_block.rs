//! Phase B — differential tests for the LOW-LEVEL BLOCK API and the exported
//! block / entropy internals of zstd.
//!
//! Every entry point is reached through `dlsym` on BOTH the C `libzstd.so` and
//! the Rust `libzstd.so` (via the shared harness `fnpair!`). No Rust function is
//! ever called directly. Return values are compared exactly, output buffers are
//! pre-filled with 0xAA identically on both sides and compared in full, and on
//! error we assert BOTH `ZSTD_isError` and that `ZSTD_getErrorCode` matches.

mod common;
use common::*;
use std::os::raw::{c_char, c_int, c_uint, c_ulonglong, c_void};

// --------------------------------------------------------------- fn aliases --

type FnCreate = unsafe extern "C" fn() -> *mut c_void;
type FnFree = unsafe extern "C" fn(*mut c_void) -> size_t;
type FnVoid = unsafe extern "C" fn() -> size_t;
type FnRetInt = unsafe extern "C" fn() -> c_int;
type FnRetStr = unsafe extern "C" fn() -> *const c_char;

type FnBegin = unsafe extern "C" fn(*mut c_void, c_int) -> size_t;
type FnBeginDict = unsafe extern "C" fn(*mut c_void, *const c_void, size_t, c_int) -> size_t;
type FnBeginAdvanced =
    unsafe extern "C" fn(*mut c_void, *const c_void, size_t, ZSTD_parameters, c_ulonglong) -> size_t;
type FnCopyCCtx = unsafe extern "C" fn(*mut c_void, *const c_void, c_ulonglong) -> size_t;

// continue/end/block: (ctx, dst, dstCap, src, srcSize)
type FnCont = unsafe extern "C" fn(*mut c_void, *mut c_void, size_t, *const c_void, size_t) -> size_t;
type FnCCtxCompress2 =
    unsafe extern "C" fn(*mut c_void, *mut c_void, size_t, *const c_void, size_t) -> size_t;

type FnDBegin = unsafe extern "C" fn(*mut c_void) -> size_t;
type FnDBeginDict = unsafe extern "C" fn(*mut c_void, *const c_void, size_t) -> size_t;
type FnDBeginDDict = unsafe extern "C" fn(*mut c_void, *const c_void) -> size_t;
type FnNextSrc = unsafe extern "C" fn(*mut c_void) -> size_t;
type FnNextInput = unsafe extern "C" fn(*mut c_void) -> c_int;
type FnCopyDCtx = unsafe extern "C" fn(*mut c_void, *const c_void);
type FnInsertBlock = unsafe extern "C" fn(*mut c_void, *const c_void, size_t) -> size_t;
type FnCheckCont = unsafe extern "C" fn(*mut c_void, *const c_void, size_t);

type FnCreateDDict = unsafe extern "C" fn(*const c_void, size_t) -> *mut c_void;

type FnDecMargin = unsafe extern "C" fn(*const c_void, size_t) -> size_t;
type FnDecBufMin = unsafe extern "C" fn(c_ulonglong, c_ulonglong) -> size_t;
type FnCycleLog = unsafe extern "C" fn(c_uint, c_int) -> c_uint;
type FnGetBlockSize = unsafe extern "C" fn(*const c_void) -> size_t;
type FnGetcBlockSize = unsafe extern "C" fn(*const c_void, size_t, *mut BlockProperties) -> size_t;
type FnWriteLastEmpty = unsafe extern "C" fn(*mut c_void, size_t) -> size_t;
type FnGetCParams = unsafe extern "C" fn(c_int, c_ulonglong, size_t) -> ZSTD_compressionParameters;
type FnGetCParamsFromCCtxParams =
    unsafe extern "C" fn(*const c_void, c_ulonglong, size_t, c_int) -> ZSTD_compressionParameters;
type FnCreateCCtxParams = unsafe extern "C" fn() -> *mut c_void;
type FnCCtxParamsSet = unsafe extern "C" fn(*mut c_void, c_int, c_int) -> size_t;

type FnLit = unsafe extern "C" fn(*mut c_void, size_t, *const c_void, size_t) -> size_t;

type FnFseBitCost = unsafe extern "C" fn(*const c_void, *const c_uint, c_uint) -> size_t;
type FnCrossEntropy = unsafe extern "C" fn(*const i16, c_uint, *const c_uint, c_uint) -> size_t;

// buildFSETable(dt, normCounter, maxSymbolValue, baseValue, nbAddBits, tableLog, wksp, wkspSize, bmi2)
type FnBuildFSETable = unsafe extern "C" fn(
    *mut c_void,
    *const i16,
    c_uint,
    *const c_uint,
    *const u8,
    c_uint,
    *mut c_void,
    size_t,
    c_int,
);

/// Mirrors `blockProperties_t` from zstd_internal.h: three unsigned-ish fields.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct BlockProperties {
    block_type: c_int,
    last_block: c_uint,
    orig_size: c_uint,
}

const FILL: u8 = 0xAA;

fn is_err(f: FnIsError, rc: size_t) -> bool {
    unsafe { f(rc) != 0 }
}

struct Err {
    is_error: (FnIsError, FnIsError),
    err_code: (FnGetErrorCode, FnGetErrorCode),
}
fn err_api() -> Err {
    Err {
        is_error: fnpair!("ZSTD_isError", FnIsError),
        err_code: fnpair!("ZSTD_getErrorCode", FnGetErrorCode),
    }
}

#[track_caller]
fn assert_ret(e: &Err, ctx: &str, c: size_t, r: size_t) {
    let ce = unsafe { (e.is_error.0)(c) != 0 };
    let re = unsafe { (e.is_error.1)(r) != 0 };
    assert_eq!(ce, re, "{ctx}: isError differs (C={c:#x} R={r:#x})");
    if ce {
        let cc = unsafe { (e.err_code.0)(c) };
        let rc = unsafe { (e.err_code.1)(r) };
        assert_eq!(cc, rc, "{ctx}: error code differs (C={cc} R={rc})");
    } else {
        assert_eq!(c, r, "{ctx}: return value differs (C={c} R={r})");
    }
}

const LENS: [usize; 9] = [0, 1, 3, 64, 1024, 65536, 131072, 131073, 300000];

fn getcparams_pair() -> (FnGetCParams, FnGetCParams) {
    fnpair!("ZSTD_getCParams", FnGetCParams)
}
unsafe fn gcparams(
    f: &(FnGetCParams, FnGetCParams),
    level: c_int,
    srcsize: u64,
) -> ZSTD_compressionParameters {
    let c = (f.0)(level, srcsize as c_ulonglong, 0);
    let r = (f.1)(level, srcsize as c_ulonglong, 0);
    assert_eq!(c, r, "getCParams disagree level={level} srcsize={srcsize}");
    c
}

fn u32_bytes(v: &[u32]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u8, std::mem::size_of_val(v)) }
}

// ============================================================= SECTION A ====

#[test]
fn bufferless_compress() {
    let create: (FnCreate, FnCreate) = fnpair!("ZSTD_createCCtx", FnCreate);
    let free: (FnFree, FnFree) = fnpair!("ZSTD_freeCCtx", FnFree);
    let begin: (FnBegin, FnBegin) = fnpair!("ZSTD_compressBegin", FnBegin);
    let begin_dict: (FnBeginDict, FnBeginDict) = fnpair!("ZSTD_compressBegin_usingDict", FnBeginDict);
    let begin_adv: (FnBeginAdvanced, FnBeginAdvanced) =
        fnpair!("ZSTD_compressBegin_advanced", FnBeginAdvanced);
    let cont: (FnCont, FnCont) = fnpair!("ZSTD_compressContinue", FnCont);
    let cont_pub: (FnCont, FnCont) = fnpair!("ZSTD_compressContinue_public", FnCont);
    let end: (FnCont, FnCont) = fnpair!("ZSTD_compressEnd", FnCont);
    let end_pub: (FnCont, FnCont) = fnpair!("ZSTD_compressEnd_public", FnCont);
    let decompress: (FnDecompress, FnDecompress) = fnpair!("ZSTD_decompress", FnDecompress);
    let bound: (FnSizeSize, FnSizeSize) = fnpair!("ZSTD_compressBound", FnSizeSize);
    let gcp = getcparams_pair();
    let e = err_api();

    let mut rng = Rng::new(0xB10C_C0DE_0001);

    #[derive(Clone, Copy)]
    enum BeginKind {
        Plain(c_int),
        Dict(c_int),
        Advanced,
    }

    for &shape in ALL_SHAPES.iter() {
        for &len in LENS.iter() {
            if len >= 131072 && !matches!(shape, Shape::Text | Shape::Random | Shape::Mixed) {
                continue;
            }
            let src = gen(shape, len, &mut rng);

            let begins = [
                BeginKind::Plain(3),
                BeginKind::Plain(1),
                BeginKind::Plain(19),
                BeginKind::Dict(5),
                BeginKind::Advanced,
            ];
            for (bi, &bk) in begins.iter().enumerate() {
                let dict = gen(Shape::Text, 512, &mut rng);
                let use_public = (bi & 1) == 1;
                let ctx0 = format!("compress shape={shape:?} len={len} begin#{bi}");

                unsafe {
                    let cc = (create.0)();
                    let rc = (create.1)();
                    assert!(!cc.is_null() && !rc.is_null(), "{ctx0}: createCCtx null");

                    let (bc, br) = match bk {
                        BeginKind::Plain(l) => ((begin.0)(cc, l), (begin.1)(rc, l)),
                        BeginKind::Dict(l) => (
                            (begin_dict.0)(cc, dict.as_ptr() as *const c_void, dict.len(), l),
                            (begin_dict.1)(rc, dict.as_ptr() as *const c_void, dict.len(), l),
                        ),
                        BeginKind::Advanced => {
                            let params = ZSTD_parameters {
                                cParams: gcparams(&gcp, 6, len as u64),
                                fParams: ZSTD_frameParameters {
                                    contentSizeFlag: 1,
                                    checksumFlag: (bi as c_int) & 1,
                                    noDictIDFlag: 0,
                                },
                            };
                            (
                                (begin_adv.0)(cc, std::ptr::null(), 0, params, ZSTD_CONTENTSIZE_UNKNOWN),
                                (begin_adv.1)(rc, std::ptr::null(), 0, params, ZSTD_CONTENTSIZE_UNKNOWN),
                            )
                        }
                    };
                    assert_ret(&e, &format!("{ctx0}: begin"), bc, br);

                    let cap = (bound.0)(src.len()).max(64);
                    assert_eq!(cap, (bound.1)(src.len()), "{ctx0}: bound differs");
                    let mut out_c = vec![FILL; cap];
                    let mut out_r = vec![FILL; cap];
                    let mut off_c = 0usize;
                    let mut off_r = 0usize;

                    // deterministic chunk plan
                    let mut pos = 0usize;
                    let mut chunks: Vec<(usize, usize)> = Vec::new();
                    while pos < src.len() {
                        let remaining = src.len() - pos;
                        let step = (1 + rng.below(remaining.min(40_000).max(1))).min(remaining);
                        chunks.push((pos, step));
                        pos += step;
                    }

                    let mut broke = false;
                    for (si, &(p, n)) in chunks.iter().enumerate() {
                        let sptr = src.as_ptr().add(p) as *const c_void;
                        let (wc, wr) = if use_public {
                            (
                                (cont_pub.0)(cc, out_c.as_mut_ptr().add(off_c) as *mut c_void, cap - off_c, sptr, n),
                                (cont_pub.1)(rc, out_r.as_mut_ptr().add(off_r) as *mut c_void, cap - off_r, sptr, n),
                            )
                        } else {
                            (
                                (cont.0)(cc, out_c.as_mut_ptr().add(off_c) as *mut c_void, cap - off_c, sptr, n),
                                (cont.1)(rc, out_r.as_mut_ptr().add(off_r) as *mut c_void, cap - off_r, sptr, n),
                            )
                        };
                        assert_ret(&e, &format!("{ctx0}: continue step {si}"), wc, wr);
                        if is_err(e.is_error.0, wc) {
                            broke = true;
                            break;
                        }
                        off_c += wc;
                        off_r += wr;
                    }

                    let (ec, er) = if use_public {
                        (
                            (end_pub.0)(cc, out_c.as_mut_ptr().add(off_c) as *mut c_void, cap - off_c, std::ptr::null(), 0),
                            (end_pub.1)(rc, out_r.as_mut_ptr().add(off_r) as *mut c_void, cap - off_r, std::ptr::null(), 0),
                        )
                    } else {
                        (
                            (end.0)(cc, out_c.as_mut_ptr().add(off_c) as *mut c_void, cap - off_c, std::ptr::null(), 0),
                            (end.1)(rc, out_r.as_mut_ptr().add(off_r) as *mut c_void, cap - off_r, std::ptr::null(), 0),
                        )
                    };
                    assert_ret(&e, &format!("{ctx0}: end"), ec, er);

                    if !broke && !is_err(e.is_error.0, ec) {
                        off_c += ec;
                        off_r += er;
                        assert_eq!(off_c, off_r, "{ctx0}: total frame length differs");
                        assert_bytes_eq(&format!("{ctx0}: frame bytes"), &out_c[..off_c], &out_r[..off_r]);

                        let dcap = src.len().max(1);
                        let mut d_c = vec![0u8; dcap];
                        let mut d_r = vec![0u8; dcap];
                        let dc = (decompress.0)(d_c.as_mut_ptr() as *mut c_void, dcap, out_c.as_ptr() as *const c_void, off_c);
                        let dr = (decompress.1)(d_r.as_mut_ptr() as *mut c_void, dcap, out_r.as_ptr() as *const c_void, off_r);
                        assert_ret(&e, &format!("{ctx0}: decompress"), dc, dr);
                        if !is_err(e.is_error.0, dc) {
                            assert_eq!(dc, src.len(), "{ctx0}: decompressed size wrong");
                            assert_bytes_eq(&format!("{ctx0}: roundtrip C"), &src, &d_c[..dc]);
                            assert_bytes_eq(&format!("{ctx0}: roundtrip R"), &src, &d_r[..dr]);
                        }
                    }

                    (free.0)(cc);
                    (free.1)(rc);
                }
            }
        }
    }
}

#[test]
fn copy_cctx_midstream() {
    let create: (FnCreate, FnCreate) = fnpair!("ZSTD_createCCtx", FnCreate);
    let free: (FnFree, FnFree) = fnpair!("ZSTD_freeCCtx", FnFree);
    let begin: (FnBegin, FnBegin) = fnpair!("ZSTD_compressBegin", FnBegin);
    let copy: (FnCopyCCtx, FnCopyCCtx) = fnpair!("ZSTD_copyCCtx", FnCopyCCtx);
    let cont: (FnCont, FnCont) = fnpair!("ZSTD_compressContinue", FnCont);
    let end: (FnCont, FnCont) = fnpair!("ZSTD_compressEnd", FnCont);
    let bound: (FnSizeSize, FnSizeSize) = fnpair!("ZSTD_compressBound", FnSizeSize);
    let decompress: (FnDecompress, FnDecompress) = fnpair!("ZSTD_decompress", FnDecompress);
    let e = err_api();

    let mut rng = Rng::new(0xB10C_C0DE_0002);

    for &shape in ALL_SHAPES.iter() {
        for &len in [1usize, 64, 1024, 65536, 131073].iter() {
            let src = gen(shape, len, &mut rng);
            let ctx0 = format!("copyCCtx shape={shape:?} len={len}");
            unsafe {
                let pc = (create.0)();
                let pr = (create.1)();
                let cc = (create.0)();
                let rc = (create.1)();
                assert!(!pc.is_null() && !pr.is_null() && !cc.is_null() && !rc.is_null());

                let bc = (begin.0)(pc, 7);
                let br = (begin.1)(pr, 7);
                assert_ret(&e, &format!("{ctx0}: begin prepared"), bc, br);

                // copyCCtx requires a known pledged size to match; pass exact len.
                let ycc = (copy.0)(cc, pc, len as c_ulonglong);
                let ycr = (copy.1)(rc, pr, len as c_ulonglong);
                assert_ret(&e, &format!("{ctx0}: copyCCtx"), ycc, ycr);

                let cap = (bound.0)(src.len()).max(64);
                let mut out_c = vec![FILL; cap];
                let mut out_r = vec![FILL; cap];
                let wc = (cont.0)(cc, out_c.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, src.len());
                let wr = (cont.1)(rc, out_r.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, src.len());
                assert_ret(&e, &format!("{ctx0}: continue-on-copy"), wc, wr);

                if !is_err(e.is_error.0, wc) {
                    let mut off_c = wc;
                    let mut off_r = wr;
                    let ec = (end.0)(cc, out_c.as_mut_ptr().add(off_c) as *mut c_void, cap - off_c, std::ptr::null(), 0);
                    let er = (end.1)(rc, out_r.as_mut_ptr().add(off_r) as *mut c_void, cap - off_r, std::ptr::null(), 0);
                    assert_ret(&e, &format!("{ctx0}: end-on-copy"), ec, er);
                    off_c += ec;
                    off_r += er;

                    assert_eq!(off_c, off_r, "{ctx0}: total len differs");
                    assert_bytes_eq(&format!("{ctx0}: bytes"), &out_c[..off_c], &out_r[..off_r]);

                    let mut d_c = vec![0u8; len.max(1)];
                    let mut d_r = vec![0u8; len.max(1)];
                    let dc = (decompress.0)(d_c.as_mut_ptr() as *mut c_void, len.max(1), out_c.as_ptr() as *const c_void, off_c);
                    let dr = (decompress.1)(d_r.as_mut_ptr() as *mut c_void, len.max(1), out_r.as_ptr() as *const c_void, off_r);
                    assert_ret(&e, &format!("{ctx0}: rt decompress"), dc, dr);
                    if !is_err(e.is_error.0, dc) {
                        assert_bytes_eq(&format!("{ctx0}: rt C"), &src, &d_c[..dc]);
                        assert_bytes_eq(&format!("{ctx0}: rt R"), &src, &d_r[..dr]);
                    }
                }

                (free.0)(pc);
                (free.1)(pr);
                (free.0)(cc);
                (free.1)(rc);
            }
        }
    }
}

#[test]
fn misc_helpers() {
    let e = err_api();
    unsafe {
        for name in ["ZSTD_CStreamInSize", "ZSTD_CStreamOutSize", "ZSTD_DStreamInSize", "ZSTD_DStreamOutSize"] {
            let (c, r): (FnVoid, FnVoid) = {
                let (c, r) = common::pair::<FnVoid>(name);
                (*c, *r)
            };
            assert_eq!(c(), r(), "{name} differs");
        }
        for name in ["ZSTD_minCLevel", "ZSTD_maxCLevel", "ZSTD_defaultCLevel"] {
            let (c, r): (FnRetInt, FnRetInt) = {
                let (c, r) = common::pair::<FnRetInt>(name);
                (*c, *r)
            };
            assert_eq!(c(), r(), "{name} differs");
        }
        {
            let (c, r): (FnRetStr, FnRetStr) = fnpair!("ZSTD_versionString", FnRetStr);
            assert_eq!(cstr(c()), cstr(r()), "ZSTD_versionString differs");
        }
        {
            let (c, r): (FnCycleLog, FnCycleLog) = fnpair!("ZSTD_cycleLog", FnCycleLog);
            for strat in 1..=9i32 {
                for hl in [1u32, 5, 10, 15, 20, 27, 30] {
                    assert_eq!(c(hl, strat), r(hl, strat), "cycleLog hashLog={hl} strat={strat}");
                }
            }
        }
        {
            let (cn, rn): (FnErrName, FnErrName) = fnpair!("ZSTD_getErrorName", FnErrName);
            let (cs, rs): (FnErrName, FnErrName) = fnpair!("ZSTD_getErrorString", FnErrName);
            let (cc, rc) = e.is_error;
            let (ce, re) = e.err_code;
            for code in 0..80usize {
                assert_eq!(cc(code), rc(code), "isError({code}) differs");
                assert_eq!(ce(code), re(code), "getErrorCode({code}) differs");
                assert_eq!(cstr(cn(code)), cstr(rn(code)), "getErrorName({code}) differs");
                let ecode = ce(code) as size_t;
                assert_eq!(cstr(cs(ecode)), cstr(rs(ecode)), "getErrorString({ecode}) differs");
            }
        }
    }
}

// ============================================================= SECTION B ====

#[test]
fn bufferless_decompress() {
    let create_c: (FnCreate, FnCreate) = fnpair!("ZSTD_createCCtx", FnCreate);
    let free_c: (FnFree, FnFree) = fnpair!("ZSTD_freeCCtx", FnFree);
    let set_param: (FnSetParam, FnSetParam) = fnpair!("ZSTD_CCtx_setParameter", FnSetParam);
    let compress2: (FnCCtxCompress2, FnCCtxCompress2) = fnpair!("ZSTD_compress2", FnCCtxCompress2);
    let bound: (FnSizeSize, FnSizeSize) = fnpair!("ZSTD_compressBound", FnSizeSize);

    let create_d: (FnCreate, FnCreate) = fnpair!("ZSTD_createDCtx", FnCreate);
    let free_d: (FnFree, FnFree) = fnpair!("ZSTD_freeDCtx", FnFree);
    let dbegin: (FnDBegin, FnDBegin) = fnpair!("ZSTD_decompressBegin", FnDBegin);
    let next_src: (FnNextSrc, FnNextSrc) = fnpair!("ZSTD_nextSrcSizeToDecompress", FnNextSrc);
    let next_input: (FnNextInput, FnNextInput) = fnpair!("ZSTD_nextInputType", FnNextInput);
    let dcont: (FnCont, FnCont) = fnpair!("ZSTD_decompressContinue", FnCont);
    let e = err_api();

    let mut rng = Rng::new(0xB10C_C0DE_0003);

    let matrix: [(c_int, c_int, c_int, c_int); 6] = [
        (3, 0, 1, 0),
        (5, 1, 1, 0),
        (1, 0, 0, 0),
        (9, 1, 0, 18),
        (12, 0, 1, 20),
        (19, 1, 1, 0),
    ];

    for &shape in ALL_SHAPES.iter() {
        for &len in [1usize, 64, 1024, 65536, 131072, 131073, 300000].iter() {
            if len >= 131072 && !matches!(shape, Shape::Text | Shape::Random | Shape::Mixed | Shape::LongRange) {
                continue;
            }
            let src = gen(shape, len, &mut rng);
            for (mi, &(level, csum, csize, wlog)) in matrix.iter().enumerate() {
                let ctx0 = format!("dstream shape={shape:?} len={len} m#{mi}");
                unsafe {
                    // produce one canonical frame with the C library and drive BOTH
                    // decoders on identical bytes.
                    let cctx = (create_c.0)();
                    assert!(!cctx.is_null());
                    let _ = (set_param.0)(cctx, ZSTD_c_compressionLevel, level);
                    let _ = (set_param.0)(cctx, ZSTD_c_checksumFlag, csum);
                    let _ = (set_param.0)(cctx, ZSTD_c_contentSizeFlag, csize);
                    if wlog != 0 {
                        let _ = (set_param.0)(cctx, ZSTD_c_windowLog, wlog);
                    }
                    let cap = (bound.0)(src.len()).max(64);
                    let mut frame = vec![0u8; cap];
                    let fsz = (compress2.0)(cctx, frame.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, src.len());
                    assert!(!is_err(e.is_error.0, fsz), "{ctx0}: compress2 failed");
                    frame.truncate(fsz);
                    (free_c.0)(cctx);

                    let dc = (create_d.0)();
                    let dr = (create_d.1)();
                    assert!(!dc.is_null() && !dr.is_null());
                    let bc = (dbegin.0)(dc);
                    let br = (dbegin.1)(dr);
                    assert_ret(&e, &format!("{ctx0}: decompressBegin"), bc, br);

                    let outcap = src.len().max(1);
                    let mut out_c = vec![FILL; outcap];
                    let mut out_r = vec![FILL; outcap];
                    let mut ipos = 0usize;
                    let mut opos_c = 0usize;
                    let mut opos_r = 0usize;
                    let mut step = 0usize;

                    loop {
                        let nc = (next_src.0)(dc);
                        let nr = (next_src.1)(dr);
                        assert_ret(&e, &format!("{ctx0}: nextSrcSize step {step}"), nc, nr);
                        let tc = (next_input.0)(dc);
                        let tr = (next_input.1)(dr);
                        assert_eq!(tc, tr, "{ctx0}: nextInputType step {step} differs (C={tc} R={tr})");
                        let n = nc;
                        if n == 0 {
                            break;
                        }
                        assert!(ipos + n <= frame.len(), "{ctx0}: past frame n={n} ipos={ipos} flen={}", frame.len());
                        let sptr = frame.as_ptr().add(ipos) as *const c_void;
                        let wc = (dcont.0)(dc, out_c.as_mut_ptr().add(opos_c) as *mut c_void, outcap - opos_c, sptr, n);
                        let wr = (dcont.1)(dr, out_r.as_mut_ptr().add(opos_r) as *mut c_void, outcap - opos_r, sptr, n);
                        assert_ret(&e, &format!("{ctx0}: decompressContinue step {step}"), wc, wr);
                        if is_err(e.is_error.0, wc) {
                            break;
                        }
                        opos_c += wc;
                        opos_r += wr;
                        ipos += n;
                        step += 1;
                        assert!(step < 1_000_000, "{ctx0}: runaway");
                    }

                    assert_eq!(opos_c, opos_r, "{ctx0}: output length differs");
                    assert_eq!(opos_c, src.len(), "{ctx0}: reconstructed size wrong");
                    assert_bytes_eq(&format!("{ctx0}: reconstructed"), &out_c[..opos_c], &src);
                    assert_bytes_eq(&format!("{ctx0}: C==R"), &out_c[..opos_c], &out_r[..opos_r]);

                    (free_d.0)(dc);
                    (free_d.1)(dr);
                }
            }
        }
    }
}

#[test]
fn copy_dctx_and_insert() {
    let create_c: (FnCreate, FnCreate) = fnpair!("ZSTD_createCCtx", FnCreate);
    let free_c: (FnFree, FnFree) = fnpair!("ZSTD_freeCCtx", FnFree);
    let compress2: (FnCCtxCompress2, FnCCtxCompress2) = fnpair!("ZSTD_compress2", FnCCtxCompress2);
    let set_param: (FnSetParam, FnSetParam) = fnpair!("ZSTD_CCtx_setParameter", FnSetParam);
    let bound: (FnSizeSize, FnSizeSize) = fnpair!("ZSTD_compressBound", FnSizeSize);

    let create_d: (FnCreate, FnCreate) = fnpair!("ZSTD_createDCtx", FnCreate);
    let free_d: (FnFree, FnFree) = fnpair!("ZSTD_freeDCtx", FnFree);
    let dbegin: (FnDBegin, FnDBegin) = fnpair!("ZSTD_decompressBegin", FnDBegin);
    let dbegin_dict: (FnDBeginDict, FnDBeginDict) = fnpair!("ZSTD_decompressBegin_usingDict", FnDBeginDict);
    let dbegin_ddict: (FnDBeginDDict, FnDBeginDDict) = fnpair!("ZSTD_decompressBegin_usingDDict", FnDBeginDDict);
    let create_ddict: (FnCreateDDict, FnCreateDDict) = fnpair!("ZSTD_createDDict", FnCreateDDict);
    let free_ddict: (FnFree, FnFree) = fnpair!("ZSTD_freeDDict", FnFree);
    let next_src: (FnNextSrc, FnNextSrc) = fnpair!("ZSTD_nextSrcSizeToDecompress", FnNextSrc);
    let dcont: (FnCont, FnCont) = fnpair!("ZSTD_decompressContinue", FnCont);
    let copy_d: (FnCopyDCtx, FnCopyDCtx) = fnpair!("ZSTD_copyDCtx", FnCopyDCtx);
    let insert: (FnInsertBlock, FnInsertBlock) = fnpair!("ZSTD_insertBlock", FnInsertBlock);
    let check_cont: (FnCheckCont, FnCheckCont) = fnpair!("ZSTD_checkContinuity", FnCheckCont);
    let dec_margin: (FnDecMargin, FnDecMargin) = fnpair!("ZSTD_decompressionMargin", FnDecMargin);
    let dec_bufmin: (FnDecBufMin, FnDecBufMin) = fnpair!("ZSTD_decodingBufferSize_min", FnDecBufMin);
    let e = err_api();

    let mut rng = Rng::new(0xB10C_C0DE_0004);

    unsafe {
        for &ws in [1u64 << 10, 1 << 17, 1 << 20, 1 << 23].iter() {
            for &fcs in [0u64, 1, 1024, 1 << 20, ZSTD_CONTENTSIZE_UNKNOWN].iter() {
                let c = (dec_bufmin.0)(ws, fcs);
                let r = (dec_bufmin.1)(ws, fcs);
                assert_ret(&e, &format!("decodingBufferSize_min ws={ws} fcs={fcs}"), c, r);
            }
        }
    }

    for &shape in [Shape::Text, Shape::Random, Shape::Mixed, Shape::Repetitive].iter() {
        for &len in [1024usize, 65536, 131072, 300000].iter() {
            let src = gen(shape, len, &mut rng);
            let ctx0 = format!("copyDCtx shape={shape:?} len={len}");
            unsafe {
                let cctx = (create_c.0)();
                let _ = (set_param.0)(cctx, ZSTD_c_compressionLevel, 5);
                let cap = (bound.0)(src.len()).max(64);
                let mut frame = vec![0u8; cap];
                let fsz = (compress2.0)(cctx, frame.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, src.len());
                assert!(!is_err(e.is_error.0, fsz), "{ctx0}: compress2 failed");
                frame.truncate(fsz);
                (free_c.0)(cctx);

                let mc = (dec_margin.0)(frame.as_ptr() as *const c_void, frame.len());
                let mr = (dec_margin.1)(frame.as_ptr() as *const c_void, frame.len());
                assert_ret(&e, &format!("{ctx0}: decompressionMargin"), mc, mr);

                let prep_c = (create_d.0)();
                let prep_r = (create_d.1)();
                let dc = (create_d.0)();
                let dr = (create_d.1)();
                let bc = (dbegin.0)(prep_c);
                let br = (dbegin.1)(prep_r);
                assert_ret(&e, &format!("{ctx0}: dbegin"), bc, br);

                let outcap = src.len().max(1);
                let mut out_c = vec![FILL; outcap];
                let mut out_r = vec![FILL; outcap];
                let mut ipos = 0usize;
                let mut opos_c = 0usize;
                let mut opos_r = 0usize;
                let mut step = 0usize;
                let mut copied = false;
                let mut act_c = prep_c;
                let mut act_r = prep_r;

                loop {
                    let nc = (next_src.0)(act_c);
                    let nr = (next_src.1)(act_r);
                    assert_ret(&e, &format!("{ctx0}: nextSrc step {step}"), nc, nr);
                    let n = nc;
                    if n == 0 {
                        break;
                    }
                    if step == 2 && !copied {
                        (copy_d.0)(dc, act_c);
                        (copy_d.1)(dr, act_r);
                        act_c = dc;
                        act_r = dr;
                        copied = true;
                    }
                    let sptr = frame.as_ptr().add(ipos) as *const c_void;
                    let wc = (dcont.0)(act_c, out_c.as_mut_ptr().add(opos_c) as *mut c_void, outcap - opos_c, sptr, n);
                    let wr = (dcont.1)(act_r, out_r.as_mut_ptr().add(opos_r) as *mut c_void, outcap - opos_r, sptr, n);
                    assert_ret(&e, &format!("{ctx0}: dcont step {step}"), wc, wr);
                    if is_err(e.is_error.0, wc) {
                        break;
                    }
                    opos_c += wc;
                    opos_r += wr;
                    ipos += n;
                    step += 1;
                    assert!(step < 1_000_000);
                }
                assert_bytes_eq(&format!("{ctx0}: reconstructed"), &out_c[..opos_c], &src);
                assert_bytes_eq(&format!("{ctx0}: C==R"), &out_c[..opos_c], &out_r[..opos_r]);

                // checkContinuity — no return; call identically on both.
                (check_cont.0)(prep_c, out_c.as_ptr() as *const c_void, opos_c.min(64));
                (check_cont.1)(prep_r, out_r.as_ptr() as *const c_void, opos_r.min(64));

                (free_d.0)(prep_c);
                (free_d.1)(prep_r);
                (free_d.0)(dc);
                (free_d.1)(dr);

                // decompressBegin_usingDict / usingDDict
                let dict = gen(Shape::Text, 1024, &mut rng);
                let d2c = (create_d.0)();
                let d2r = (create_d.1)();
                let rc1 = (dbegin_dict.0)(d2c, dict.as_ptr() as *const c_void, dict.len());
                let rr1 = (dbegin_dict.1)(d2r, dict.as_ptr() as *const c_void, dict.len());
                assert_ret(&e, &format!("{ctx0}: decompressBegin_usingDict"), rc1, rr1);
                let ddc = (create_ddict.0)(dict.as_ptr() as *const c_void, dict.len());
                let ddr = (create_ddict.1)(dict.as_ptr() as *const c_void, dict.len());
                let rc2 = (dbegin_ddict.0)(d2c, ddc);
                let rr2 = (dbegin_ddict.1)(d2r, ddr);
                assert_ret(&e, &format!("{ctx0}: decompressBegin_usingDDict"), rc2, rr2);
                (free_ddict.0)(ddc);
                (free_ddict.1)(ddr);
                (free_d.0)(d2c);
                (free_d.1)(d2r);

                // insertBlock — returns blockSize; call identically on fresh dctxs.
                let d3c = (create_d.0)();
                let d3r = (create_d.1)();
                let _ = (dbegin.0)(d3c);
                let _ = (dbegin.1)(d3r);
                let blk = gen(Shape::Random, 128, &mut rng);
                let ic = (insert.0)(d3c, blk.as_ptr() as *const c_void, blk.len());
                let ir = (insert.1)(d3r, blk.as_ptr() as *const c_void, blk.len());
                assert_ret(&e, &format!("{ctx0}: insertBlock"), ic, ir);
                (free_d.0)(d3c);
                (free_d.1)(d3r);
            }
        }
    }
}

// ============================================================= SECTION C ====

#[test]
fn raw_single_block() {
    let create_c: (FnCreate, FnCreate) = fnpair!("ZSTD_createCCtx", FnCreate);
    let free_c: (FnFree, FnFree) = fnpair!("ZSTD_freeCCtx", FnFree);
    let begin: (FnBegin, FnBegin) = fnpair!("ZSTD_compressBegin", FnBegin);
    let cblock: (FnCont, FnCont) = fnpair!("ZSTD_compressBlock", FnCont);
    let get_block_size: (FnGetBlockSize, FnGetBlockSize) = fnpair!("ZSTD_getBlockSize", FnGetBlockSize);
    let bound: (FnSizeSize, FnSizeSize) = fnpair!("ZSTD_compressBound", FnSizeSize);

    let create_d: (FnCreate, FnCreate) = fnpair!("ZSTD_createDCtx", FnCreate);
    let free_d: (FnFree, FnFree) = fnpair!("ZSTD_freeDCtx", FnFree);
    let dbegin: (FnDBegin, FnDBegin) = fnpair!("ZSTD_decompressBegin", FnDBegin);
    let dblock: (FnCont, FnCont) = fnpair!("ZSTD_decompressBlock", FnCont);
    let dblock_dep: (FnCont, FnCont) = fnpair!("ZSTD_decompressBlock_deprecated", FnCont);
    let e = err_api();

    let mut rng = Rng::new(0xB10C_C0DE_0005);

    let block_size;
    unsafe {
        let cc = (create_c.0)();
        let rc = (create_c.1)();
        let _ = (begin.0)(cc, 3);
        let _ = (begin.1)(rc, 3);
        let bc = (get_block_size.0)(cc);
        let br = (get_block_size.1)(rc);
        assert_ret(&e, "getBlockSize", bc, br);
        block_size = bc;
        (free_c.0)(cc);
        (free_c.1)(rc);
    }
    assert!(block_size > 0, "block size positive: {block_size}");

    let sizes = [0usize, 1, 3, 64, 1024, 65536, block_size - 1, block_size, block_size + 1];

    for &shape in ALL_SHAPES.iter() {
        for &n in sizes.iter() {
            let src = gen(shape, n, &mut rng);
            let ctx0 = format!("compressBlock shape={shape:?} n={n}");
            unsafe {
                let cc = (create_c.0)();
                let rc = (create_c.1)();
                let bc = (begin.0)(cc, 3);
                let br = (begin.1)(rc, 3);
                assert_ret(&e, &format!("{ctx0}: begin"), bc, br);

                let cap = (bound.0)(n).max(64);
                let mut out_c = vec![FILL; cap];
                let mut out_r = vec![FILL; cap];
                let wc = (cblock.0)(cc, out_c.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, n);
                let wr = (cblock.1)(rc, out_r.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, n);
                assert_ret(&e, &format!("{ctx0}: compressBlock"), wc, wr);
                assert_bytes_eq(&format!("{ctx0}: compressBlock full buf"), &out_c, &out_r);

                (free_c.0)(cc);
                (free_c.1)(rc);

                // raw blocks return 0 => "store raw"; only round-trip real compressed blocks.
                if !is_err(e.is_error.0, wc) && wc > 0 {
                    let dcap = n.max(1);
                    let dc = (create_d.0)();
                    let dr = (create_d.1)();
                    let _ = (dbegin.0)(dc);
                    let _ = (dbegin.1)(dr);
                    let mut d_c = vec![FILL; dcap];
                    let mut d_r = vec![FILL; dcap];
                    let rc1 = (dblock.0)(dc, d_c.as_mut_ptr() as *mut c_void, dcap, out_c.as_ptr() as *const c_void, wc);
                    let rr1 = (dblock.1)(dr, d_r.as_mut_ptr() as *mut c_void, dcap, out_r.as_ptr() as *const c_void, wr);
                    assert_ret(&e, &format!("{ctx0}: decompressBlock"), rc1, rr1);
                    if !is_err(e.is_error.0, rc1) {
                        assert_bytes_eq(&format!("{ctx0}: block roundtrip"), &d_c[..rc1], &src);
                        assert_bytes_eq(&format!("{ctx0}: block C==R"), &d_c[..rc1], &d_r[..rr1]);
                    }
                    (free_d.0)(dc);
                    (free_d.1)(dr);

                    let dc2 = (create_d.0)();
                    let dr2 = (create_d.1)();
                    let _ = (dbegin.0)(dc2);
                    let _ = (dbegin.1)(dr2);
                    let mut d2c = vec![FILL; dcap];
                    let mut d2r = vec![FILL; dcap];
                    let rc2 = (dblock_dep.0)(dc2, d2c.as_mut_ptr() as *mut c_void, dcap, out_c.as_ptr() as *const c_void, wc);
                    let rr2 = (dblock_dep.1)(dr2, d2r.as_mut_ptr() as *mut c_void, dcap, out_r.as_ptr() as *const c_void, wr);
                    assert_ret(&e, &format!("{ctx0}: decompressBlock_deprecated"), rc2, rr2);
                    assert_bytes_eq(&format!("{ctx0}: dep buf C==R"), &d2c, &d2r);
                    (free_d.0)(dc2);
                    (free_d.1)(dr2);
                }
            }
        }
    }
}

#[test]
fn block_probing() {
    let create_c: (FnCreate, FnCreate) = fnpair!("ZSTD_createCCtx", FnCreate);
    let free_c: (FnFree, FnFree) = fnpair!("ZSTD_freeCCtx", FnFree);
    let compress2: (FnCCtxCompress2, FnCCtxCompress2) = fnpair!("ZSTD_compress2", FnCCtxCompress2);
    let set_param: (FnSetParam, FnSetParam) = fnpair!("ZSTD_CCtx_setParameter", FnSetParam);
    let bound: (FnSizeSize, FnSizeSize) = fnpair!("ZSTD_compressBound", FnSizeSize);
    let getc_block: (FnGetcBlockSize, FnGetcBlockSize) = fnpair!("ZSTD_getcBlockSize", FnGetcBlockSize);
    let write_last: (FnWriteLastEmpty, FnWriteLastEmpty) = fnpair!("ZSTD_writeLastEmptyBlock", FnWriteLastEmpty);
    let e = err_api();

    let mut rng = Rng::new(0xB10C_C0DE_0006);

    unsafe {
        for cap in [0usize, 1, 2, 3, 4, 8, 16] {
            let mut b_c = vec![FILL; cap.max(1)];
            let mut b_r = vec![FILL; cap.max(1)];
            let c = (write_last.0)(b_c.as_mut_ptr() as *mut c_void, cap);
            let r = (write_last.1)(b_r.as_mut_ptr() as *mut c_void, cap);
            assert_ret(&e, &format!("writeLastEmptyBlock cap={cap}"), c, r);
            assert_bytes_eq(&format!("writeLastEmptyBlock buf cap={cap}"), &b_c, &b_r);
        }
    }

    for &shape in [Shape::Text, Shape::Random, Shape::Mixed].iter() {
        for &len in [1024usize, 65536, 200000].iter() {
            let src = gen(shape, len, &mut rng);
            let ctx0 = format!("getcBlockSize shape={shape:?} len={len}");
            unsafe {
                let cctx = (create_c.0)();
                let _ = (set_param.0)(cctx, ZSTD_c_compressionLevel, 3);
                let cap = (bound.0)(len).max(64);
                let mut frame = vec![0u8; cap];
                let fsz = (compress2.0)(cctx, frame.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, src.len());
                assert!(!is_err(e.is_error.0, fsz), "{ctx0}: compress2");
                frame.truncate(fsz);
                (free_c.0)(cctx);

                // getcBlockSize reads a 3-byte block header. Feed many windows; both
                // libs must agree on return AND on the filled BlockProperties.
                let mut i = 0usize;
                while i + 3 <= frame.len() {
                    let mut bp_c = BlockProperties::default();
                    let mut bp_r = BlockProperties::default();
                    let avail = frame.len() - i;
                    let c = (getc_block.0)(frame.as_ptr().add(i) as *const c_void, avail, &mut bp_c);
                    let r = (getc_block.1)(frame.as_ptr().add(i) as *const c_void, avail, &mut bp_r);
                    assert_ret(&e, &format!("{ctx0}: getcBlockSize @ {i}"), c, r);
                    assert_eq!(bp_c, bp_r, "{ctx0}: BlockProperties @ {i} differ (C={bp_c:?} R={bp_r:?})");
                    i += if !is_err(e.is_error.0, c) { 3 + c.max(1) } else { 37 };
                }
            }
        }
    }
}

// ============================================================= SECTION D ====

#[test]
fn entropy_literals() {
    let no_compress: (FnLit, FnLit) = fnpair!("ZSTD_noCompressLiterals", FnLit);
    let rle: (FnLit, FnLit) = fnpair!("ZSTD_compressRleLiteralsBlock", FnLit);
    let e = err_api();

    let mut rng = Rng::new(0xB10C_C0DE_0007);

    // ---- ZSTD_noCompressLiterals: header + raw copy; returns dstSize_tooSmall
    // when srcSize + flSize > dstCapacity. Valid for any srcSize incl. 0, but the
    // `src` pointer must be readable, so we always back it with a real allocation.
    for &shape in ALL_SHAPES.iter() {
        for &n in [0usize, 1, 3, 64, 1024, 65536, 131072].iter() {
            let src = {
                let mut v = gen(shape, n, &mut rng);
                if v.is_empty() {
                    v.push(0);
                } // keep a valid, readable pointer even when n==0
                v
            };
            let ctx0 = format!("noCompress shape={shape:?} n={n}");
            unsafe {
                for &cap in [n + 16, n + 5, n + 1, n /* may be too small */].iter() {
                    let mut d_c = vec![FILL; cap.max(1)];
                    let mut d_r = vec![FILL; cap.max(1)];
                    let c = (no_compress.0)(d_c.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, n);
                    let r = (no_compress.1)(d_r.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, n);
                    assert_ret(&e, &format!("{ctx0}: cap={cap}"), c, r);
                    assert_bytes_eq(&format!("{ctx0}: buf cap={cap}"), &d_c, &d_r);
                }
            }
        }
    }

    // ---- ZSTD_compressRleLiteralsBlock: documented preconditions are
    // `dstCapacity >= 4`, `srcSize >= 1`, and all source bytes identical. Violating
    // them is UB in the C (it reads src[0] and writes ostart[flSize] unguarded), so
    // we only feed valid RLE inputs and always give it >= 4 bytes of dst.
    for &n in [1usize, 3, 31, 32, 64, 1024, 4095, 4096, 65536, 131072].iter() {
        let byte = (rng.next_u32() & 0xFF) as u8;
        let src = vec![byte; n];
        let ctx0 = format!("rle byte={byte:#x} n={n}");
        unsafe {
            for &cap in [16usize, 8, 5, 4].iter() {
                let mut d_c = vec![FILL; cap];
                let mut d_r = vec![FILL; cap];
                let c = (rle.0)(d_c.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, n);
                let r = (rle.1)(d_r.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, n);
                assert_ret(&e, &format!("{ctx0}: cap={cap}"), c, r);
                assert_bytes_eq(&format!("{ctx0}: buf cap={cap}"), &d_c, &d_r);
            }
        }
    }
}

#[test]
fn entropy_costs() {
    let fse_bit_cost: (FnFseBitCost, FnFseBitCost) = fnpair!("ZSTD_fseBitCost", FnFseBitCost);
    let cross_entropy: (FnCrossEntropy, FnCrossEntropy) = fnpair!("ZSTD_crossEntropyCost", FnCrossEntropy);

    let mut rng = Rng::new(0xB10C_C0DE_0008);

    // ZSTD_crossEntropyCost(norm, accuracyLog, count, max):
    // C preconditions (asserts, compiled out in release => UB if violated):
    //   * accuracyLog <= 8
    //   * for every symbol, (normAcc << (8-accuracyLog)) in (0,256), where
    //     normAcc = (norm[s]==-1) ? 1 : norm[s]. Hence each norm[s] in [1, 2^acc-1].
    // We therefore keep accuracyLog in 5..=8, use >=2 symbols, and cap every entry
    // strictly below the total so no single symbol takes the whole mass.
    unsafe {
        for trial in 0..200usize {
            let acc_log: c_uint = 5 + (rng.below(4) as c_uint); // 5..=8
            let total = 1i32 << acc_log; // <= 256
            // need at least 2 symbols so each entry can stay < total
            let max: c_uint = 1 + (rng.below((total as usize - 1).min(31)) as c_uint);
            let nsym = (max + 1) as usize;
            let mut norm = vec![0i16; nsym];
            let mut remaining = total;
            for s in 0..nsym {
                let left = (nsym - 1 - s) as i32; // symbols still to fill after this one
                if s == nsym - 1 {
                    // last symbol takes the rest; guaranteed in [1, total-1] because
                    // earlier symbols each took >= 1 and we never let it reach total.
                    norm[s] = remaining as i16;
                } else {
                    // reserve >=1 for each remaining symbol; also keep this entry <= total-1
                    let hi = (remaining - left).min(total - 1).max(1);
                    let v = 1 + rng.below(hi as usize) as i32;
                    norm[s] = v as i16;
                    remaining -= v;
                }
            }
            // validity: every entry in [1, total-1]
            if norm.iter().any(|&v| v < 1 || (v as i32) >= total) {
                continue;
            }
            let mut count = vec![0u32; nsym];
            for c in count.iter_mut() {
                *c = rng.below(1000) as u32;
            }
            let c = (cross_entropy.0)(norm.as_ptr(), acc_log, count.as_ptr(), max);
            let r = (cross_entropy.1)(norm.as_ptr(), acc_log, count.as_ptr(), max);
            assert_eq!(c, r, "crossEntropyCost trial {trial} differs (max={max} acc={acc_log})");
        }
    }

    // ZSTD_fseBitCost(ctable, count, max): build a valid FSE_CTable via the exported
    // FSE_buildCTable_wksp so the ctable is well-formed, then diff the cost.
    type FnFseBuildCTable =
        unsafe extern "C" fn(*mut c_void, *const i16, c_uint, c_uint, *mut c_void, size_t) -> size_t;
    let build_ct: (FnFseBuildCTable, FnFseBuildCTable) = fnpair!("FSE_buildCTable_wksp", FnFseBuildCTable);
    let fse_is_error: (FnIsError, FnIsError) = fnpair!("FSE_isError", FnIsError);
    unsafe {
        for trial in 0..80usize {
            let max: c_uint = 1 + (rng.below(20) as c_uint);
            // tableLog must be >= FSE_MIN_TABLELOG (5) AND large enough for maxSymbol:
            // FSE requires tableLog >= highbit(maxSymbol) + 2. Pick accordingly so the
            // build never errors (an errored/unbuilt ctable fed to fseBitCost is UB).
            let min_for_sym = (31 - max.leading_zeros()) + 2; // highbit(max)+2
            let lo = 5.max(min_for_sym);
            let acc_log: c_uint = lo + (rng.below(3) as c_uint); // lo..=lo+2, capped below
            let acc_log = acc_log.min(12);
            let total = 1i32 << acc_log;
            let nsym = (max + 1) as usize;
            let mut norm = vec![0i16; nsym];
            let mut remaining = total;
            for s in 0..nsym {
                if s == nsym - 1 {
                    norm[s] = remaining as i16;
                } else {
                    let left = (nsym - 1 - s) as i32;
                    let hi = (remaining - left).max(1);
                    let v = 1 + rng.below(hi as usize) as i32;
                    norm[s] = v as i16;
                    remaining -= v;
                }
            }
            if norm.iter().any(|&v| v < 1) {
                continue;
            }
            let ct_u32 = 1 + (1usize << acc_log) + max as usize + 2;
            let mut ct_c = vec![0u32; ct_u32 + 8];
            let mut ct_r = vec![0u32; ct_u32 + 8];
            let mut wksp = vec![0u8; 8192];
            let bc = (build_ct.0)(ct_c.as_mut_ptr() as *mut c_void, norm.as_ptr(), max, acc_log, wksp.as_mut_ptr() as *mut c_void, wksp.len());
            let brr = (build_ct.1)(ct_r.as_mut_ptr() as *mut c_void, norm.as_ptr(), max, acc_log, wksp.as_mut_ptr() as *mut c_void, wksp.len());
            assert_eq!(bc, brr, "FSE_buildCTable_wksp return differs trial {trial}");
            // On a build error the ctable is not usable; skip the cost step (both agree).
            if (fse_is_error.0)(bc) != 0 {
                assert_ne!((fse_is_error.1)(brr), 0, "FSE_isError disagree trial {trial}");
                continue;
            }
            assert_bytes_eq(&format!("fseBitCost ctable trial {trial}"), u32_bytes(&ct_c), u32_bytes(&ct_r));

            let mut count = vec![0u32; nsym];
            for c in count.iter_mut() {
                *c = rng.below(500) as u32;
            }
            let c = (fse_bit_cost.0)(ct_c.as_ptr() as *const c_void, count.as_ptr(), max);
            let r = (fse_bit_cost.1)(ct_r.as_ptr() as *const c_void, count.as_ptr(), max);
            assert_eq!(c, r, "fseBitCost trial {trial} differs");
        }
    }
}

#[test]
fn entropy_fsetable() {
    let build_fse: (FnBuildFSETable, FnBuildFSETable) = fnpair!("ZSTD_buildFSETable", FnBuildFSETable);

    let mut rng = Rng::new(0xB10C_C0DE_0009);

    // ZSTD_seqSymbol is 8 bytes; decode table has (1 + (1<<tableLog)) entries.
    unsafe {
        for trial in 0..120usize {
            let table_log: c_uint = 5 + (rng.below(4) as c_uint); // 5..=8
            let max_sym: c_uint = 1 + (rng.below(20) as c_uint);
            let total = 1i32 << table_log;
            let nsym = (max_sym + 1) as usize;

            let mut norm = vec![0i16; nsym];
            let mut remaining = total;
            for s in 0..nsym {
                if s == nsym - 1 {
                    norm[s] = remaining as i16;
                } else {
                    let left = (nsym - 1 - s) as i32;
                    let hi = (remaining - left).max(1);
                    let v = 1 + rng.below(hi as usize) as i32;
                    norm[s] = v as i16;
                    remaining -= v;
                }
            }
            if norm.iter().any(|&v| v < 1) {
                continue;
            }

            let base: Vec<u32> = (0..nsym).map(|s| (s as u32) * 3).collect();
            let addbits: Vec<u8> = (0..nsym).map(|s| (s % 16) as u8).collect();

            let entries = 1 + (1usize << table_log);
            let mut dt_c = vec![0u8; entries * 8 + 64];
            let mut dt_r = vec![0u8; entries * 8 + 64];
            let mut wk_c = vec![0u64; 4096];
            let mut wk_r = vec![0u64; 4096];

            for &bmi2 in [0i32, 1].iter() {
                for b in dt_c.iter_mut() {
                    *b = FILL;
                }
                for b in dt_r.iter_mut() {
                    *b = FILL;
                }
                (build_fse.0)(
                    dt_c.as_mut_ptr() as *mut c_void, norm.as_ptr(), max_sym,
                    base.as_ptr(), addbits.as_ptr(), table_log,
                    wk_c.as_mut_ptr() as *mut c_void, wk_c.len() * 8, bmi2,
                );
                (build_fse.1)(
                    dt_r.as_mut_ptr() as *mut c_void, norm.as_ptr(), max_sym,
                    base.as_ptr(), addbits.as_ptr(), table_log,
                    wk_r.as_mut_ptr() as *mut c_void, wk_r.len() * 8, bmi2,
                );
                assert_bytes_eq(
                    &format!("buildFSETable trial {trial} bmi2={bmi2} log={table_log} max={max_sym}"),
                    &dt_c[..entries * 8],
                    &dt_r[..entries * 8],
                );
            }
        }
    }
}

#[test]
fn cparams_and_scalars() {
    let create_params: (FnCreateCCtxParams, FnCreateCCtxParams) = fnpair!("ZSTD_createCCtxParams", FnCreateCCtxParams);
    let free_params: (FnFree, FnFree) = fnpair!("ZSTD_freeCCtxParams", FnFree);
    let set_p: (FnCCtxParamsSet, FnCCtxParamsSet) = fnpair!("ZSTD_CCtxParams_setParameter", FnCCtxParamsSet);
    let from_ccp: (FnGetCParamsFromCCtxParams, FnGetCParamsFromCCtxParams) =
        fnpair!("ZSTD_getCParamsFromCCtxParams", FnGetCParamsFromCCtxParams);
    let get_cparams: (FnGetCParams, FnGetCParams) = fnpair!("ZSTD_getCParams", FnGetCParams);

    let mut rng = Rng::new(0xB10C_C0DE_000A);

    unsafe {
        for level in -5..=22i32 {
            for &ss in [0u64, 1, 1024, 1 << 16, 1 << 20, 1 << 27].iter() {
                let c = (get_cparams.0)(level, ss as c_ulonglong, 0);
                let r = (get_cparams.1)(level, ss as c_ulonglong, 0);
                assert_eq!(c, r, "getCParams level={level} ss={ss} differs");
            }
        }
    }

    unsafe {
        for trial in 0..60usize {
            let pc = (create_params.0)();
            let pr = (create_params.1)();
            assert!(!pc.is_null() && !pr.is_null(), "createCCtxParams null");
            let level = rng.range(1, 19);
            let _ = (set_p.0)(pc, ZSTD_c_compressionLevel, level);
            let _ = (set_p.1)(pr, ZSTD_c_compressionLevel, level);
            if rng.below(2) == 0 {
                let wl = rng.range(10, 24);
                let _ = (set_p.0)(pc, ZSTD_c_windowLog, wl);
                let _ = (set_p.1)(pr, ZSTD_c_windowLog, wl);
            }
            // ZSTD_CParamMode_e values 0..=3
            for &mode in [0i32, 1, 2, 3].iter() {
                for &ss in [0u64, 1024, 1 << 20, ZSTD_CONTENTSIZE_UNKNOWN].iter() {
                    for &dsz in [0usize, 256, 4096].iter() {
                        let c = (from_ccp.0)(pc, ss as c_ulonglong, dsz, mode);
                        let r = (from_ccp.1)(pr, ss as c_ulonglong, dsz, mode);
                        assert_eq!(c, r, "getCParamsFromCCtxParams trial {trial} mode={mode} ss={ss} dsz={dsz} differ");
                    }
                }
            }
            (free_params.0)(pc);
            (free_params.1)(pr);
        }
    }
}

/// Section D — internals whose arguments are OPAQUE internal structs that cannot
/// be safely constructed from a test (fabricating that memory would be UB and the
/// comparison meaningless). Each is verified `dlsym`-able from BOTH libraries; the
/// blocking struct/reason is documented. We take the address only — never call.
#[test]
fn dlsym_only_internals() {
    type Opaque = unsafe extern "C" fn();
    let blocked: &[(&str, &str)] = &[
        ("ZSTD_buildCTable",
         "needs FSE_CTable* + SymbolEncodingType_e + internal count/code tables sized to seqStore layout"),
        ("ZSTD_seqToCodes",
         "operates on `const SeqStore_t*` (opaque internal seqStore_t) — cannot construct safely"),
        ("ZSTD_selectEncodingType",
         "needs FSE_repeat*, previous FSE_CTable*, and internal default-norm tables"),
        ("ZSTD_encodeSequences",
         "needs three built FSE_CTable* plus a `SeqDef*` array from the internal seqStore"),
        ("ZSTD_compressLiterals",
         "takes internal ZSTD_hufCTables_t / ZSTD_strategy / entropy workspace + flags (opaque HUF state)"),
        ("ZSTD_buildBlockEntropyStats",
         "needs `const SeqStore_t*`, `const ZSTD_entropyCTables_t*`, `ZSTD_entropyCTablesMetadata_t*` (all opaque)"),
        ("ZSTD_resetSeqStore",
         "takes `SeqStore_t*` (opaque internal seqStore_t); layout not part of public ABI"),
        ("ZSTD_getSeqStore",
         "returns a `SeqStore_t*` into ZSTD_CCtx internals we cannot interpret"),
        ("ZSTD_reset_compressedBlockState",
         "takes `ZSTD_compressedBlockState_t*` (opaque internal block state)"),
        ("ZSTD_invalidateRepCodes",
         "mutates opaque internal repcode state of a ZSTD_CCtx; no observable public output to diff"),
        ("ZSTD_loadCEntropy",
         "takes `ZSTD_compressedBlockState_t*` + workspace + dict; opaque compressed-block-state layout"),
        ("ZSTD_loadDEntropy",
         "takes `ZSTD_entropyDTables_t*` (opaque internal decode-entropy tables)"),
        ("ZSTD_selectBlockCompressor",
         "returns an internal fn-ptr `ZSTD_BlockCompressor_f`; not meaningfully comparable across builds"),
        ("ZSTD_convertBlockSequences",
         "takes `ZSTD_CCtx*` and writes into opaque internal seqStore; no directly-diffable output buffer"),
        ("ZSTD_get1BlockSummary",
         "returns `BlockSummary` but consumes block-delimited `ZSTD_Sequence` layout with internal semantics"),
        ("ZSTD_splitBlock",
         "requires a CCtx-derived pre-split workspace + strict 128KB precondition + internal state"),
        ("ZSTD_compressSuperBlock",
         "takes `ZSTD_CCtx*` with a fully-populated internal seqStore and block state — not constructible alone"),
        ("ZSTD_getCParamsFromCDict",
         "takes `const ZSTD_CDict*`; a direct diff would require identical opaque CDict internals"),
        ("ZSTD_decompressBlock_internal",
         "takes internal `streaming_operation` enum and a DCtx primed mid-frame; unsafe to reach directly"),
        ("ZSTD_decodeLiteralsBlock_wrapper",
         "writes into ZSTD_DCtx internal literalsBuffer; needs a DCtx primed to the exact literals-block state"),
        ("ZSTD_decodeSeqHeaders",
         "takes `ZSTD_DCtx*` + `int* nbSeqPtr` and mutates opaque DCtx FSE entropy tables mid-block"),
    ];

    for (name, reason) in blocked {
        // fnpair! requires a string literal; use the underlying `pair` for dynamic names.
        let (c, r) = common::pair::<Opaque>(name);
        // Take the addresses to prove both are loadable; never invoke.
        let _ = (*c, *r);
        eprintln!("dlsym-verified only: {name} — blocked by: {reason}");
    }
}
