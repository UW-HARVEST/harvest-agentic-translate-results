//! Phase C — the generic boundaries every C API has: NULL pointers, zero and
//! oversized lengths, and values one step past a documented range.
//!
//! Only entry points whose C implementation *documents or implements* NULL /
//! zero handling are probed; the ones that dereference unconditionally are
//! listed in `CONFIGS.md` ("C preconditions that are UB when violated") and
//! excluded, because a precondition violation is not a translation difference.
mod common;
use common::*;
use std::ffi::{c_char, c_int, c_uint, c_ulonglong, c_void};

type FnU64FromBuf = unsafe extern "C" fn(*const c_void, usize) -> c_ulonglong;
type FnUFromBuf = unsafe extern "C" fn(*const c_void, usize) -> c_uint;
type FnFromBuf = unsafe extern "C" fn(*const c_void, usize) -> usize;
type FnVoidPtr = unsafe extern "C" fn(*const c_void) -> usize;

#[track_caller]
fn eqcode(what: &str, c: usize, r: usize) {
    unsafe {
        let (gcc, gcr) = duo::<unsafe extern "C" fn(usize) -> c_uint>("ZSTD_getErrorCode");
        let (nc, nr) = duo::<FnErrName>("ZSTD_getErrorName");
        assert_eq!(
            c,
            r,
            "{what}: C={c:#x} (code {} = {}), Rust={r:#x} (code {} = {})",
            gcc(c),
            cstr(nc(c)),
            gcr(r),
            cstr(nr(r))
        );
    }
}

// ------------------------------------------------------------------ free(NULL)

/// Every `ZSTD_free*` / `ZBUFF_free*` / `ZSTDMT_freeCCtx` / `POOL_free` is
/// documented as "compatible with free on NULL".
#[test]
fn null_free_functions() {
    unsafe {
        for n in [
            "ZSTD_freeCCtx",
            "ZSTD_freeDCtx",
            "ZSTD_freeCStream",
            "ZSTD_freeDStream",
            "ZSTD_freeCDict",
            "ZSTD_freeDDict",
            "ZSTD_freeCCtxParams",
            "ZBUFF_freeCCtx",
            "ZBUFF_freeDCtx",
            "ZSTDMT_freeCCtx",
        ] {
            let (a, b) = duo::<FnFreePtr>(n);
            eqcode(
                &format!("{n}(NULL)"),
                a(std::ptr::null_mut()),
                b(std::ptr::null_mut()),
            );
        }
        let (a, b) = duo::<unsafe extern "C" fn(*mut c_void)>("POOL_free");
        a(std::ptr::null_mut());
        b(std::ptr::null_mut());
        let (a, b) = duo::<unsafe extern "C" fn(*mut c_void)>("POOL_joinJobs");
        a(std::ptr::null_mut());
        b(std::ptr::null_mut());
    }
}

// ------------------------------------------------------------------ sizeof(NULL)

/// The `ZSTD_sizeof_*` family documents "supports sizeof NULL".
#[test]
fn null_sizeof_functions() {
    unsafe {
        for n in [
            "ZSTD_sizeof_CCtx",
            "ZSTD_sizeof_DCtx",
            "ZSTD_sizeof_CStream",
            "ZSTD_sizeof_DStream",
            "ZSTD_sizeof_CDict",
            "ZSTD_sizeof_DDict",
            "ZSTDMT_sizeof_CCtx",
            "POOL_sizeof",
        ] {
            let (a, b) = duo::<FnVoidPtr>(n);
            eqv(&format!("{n}(NULL)"), a(std::ptr::null()), b(std::ptr::null()));
        }
    }
}

// ------------------------------------------------------------------ (src, srcSize) family

/// Every `f(const void* src, size_t srcSize)` entry point, with
/// `(NULL, 0)`, `(NULL, n)`, `(valid, 0)` and oversized `srcSize`.
#[test]
fn null_src_size_family() {
    unsafe {
        let buf = gen_class(4, 4096, 1);
        let bp = buf.as_ptr() as *const c_void;

        // returning size_t (error-coded)
        for n in [
            "ZSTD_findFrameCompressedSize",
            "ZSTD_frameHeaderSize",
            "ZSTD_decompressionMargin",
            "ZSTD_estimateDStreamSize_fromFrame",
        ] {
            let (a, b) = duo::<FnFromBuf>(n);
            eqcode(&format!("{n}(NULL,0)"), a(std::ptr::null(), 0), b(std::ptr::null(), 0));
            eqcode(&format!("{n}(valid,0)"), a(bp, 0), b(bp, 0));
            for l in [1usize, 2, 3, 4, 5, 8, 17, 4096] {
                eqcode(&format!("{n}(valid,{l})"), a(bp, l), b(bp, l));
            }
        }
        // returning unsigned long long
        for n in [
            "ZSTD_getFrameContentSize",
            "ZSTD_getDecompressedSize",
            "ZSTD_findDecompressedSize",
            "ZSTD_decompressBound",
        ] {
            let (a, b) = duo::<FnU64FromBuf>(n);
            eqv(&format!("{n}(NULL,0)"), a(std::ptr::null(), 0), b(std::ptr::null(), 0));
            eqv(&format!("{n}(valid,0)"), a(bp, 0), b(bp, 0));
            for l in [1usize, 3, 4, 5, 8, 17, 4096] {
                eqv(&format!("{n}(valid,{l})"), a(bp, l), b(bp, l));
            }
        }
        // returning unsigned
        for n in [
            "ZSTD_isFrame",
            "ZSTD_isSkippableFrame",
            "ZSTD_getDictID_fromFrame",
            "ZSTD_getDictID_fromDict",
        ] {
            let (a, b) = duo::<FnUFromBuf>(n);
            eqv(&format!("{n}(NULL,0)"), a(std::ptr::null(), 0), b(std::ptr::null(), 0));
            eqv(&format!("{n}(valid,0)"), a(bp, 0), b(bp, 0));
            for l in [1usize, 3, 4, 5, 8, 17, 4096] {
                eqv(&format!("{n}(valid,{l})"), a(bp, l), b(bp, l));
            }
        }
        // ZDICT_getDictID / ZDICT_getDictHeaderSize
        {
            let (a, b) = duo::<FnUFromBuf>("ZDICT_getDictID");
            eqv("ZDICT_getDictID(NULL,0)", a(std::ptr::null(), 0), b(std::ptr::null(), 0));
            for l in [0usize, 1, 4, 8, 4096] {
                eqv(&format!("ZDICT_getDictID(valid,{l})"), a(bp, l), b(bp, l));
            }
            let (a, b) = duo::<FnFromBuf>("ZDICT_getDictHeaderSize");
            eqcode(
                "ZDICT_getDictHeaderSize(NULL,0)",
                a(std::ptr::null(), 0),
                b(std::ptr::null(), 0),
            );
            for l in [0usize, 1, 4, 8, 4096] {
                eqcode(&format!("ZDICT_getDictHeaderSize(valid,{l})"), a(bp, l), b(bp, l));
            }
        }
        // ZSTD_getFrameHeader{,_advanced}: NULL out-param is UB in the C
        // (`zfhPtr->...` is written unconditionally), so only the (src,srcSize)
        // axis is probed here.
        {
            let (a, b) =
                duo::<unsafe extern "C" fn(*mut ZSTD_frameHeader, *const c_void, usize) -> usize>(
                    "ZSTD_getFrameHeader",
                );
            for (p, l) in [
                (std::ptr::null(), 0usize),
                (bp, 0),
                (bp, 1),
                (bp, 4),
                (bp, 5),
                (bp, 6),
                (bp, 18),
                (bp, 4096),
            ] {
                let mut hc = ZSTD_frameHeader::default();
                let mut hr = ZSTD_frameHeader::default();
                let x = a(&mut hc, p, l);
                let y = b(&mut hr, p, l);
                eqcode(&format!("ZSTD_getFrameHeader(_,{l})"), x, y);
                eqv(&format!("ZSTD_getFrameHeader(_,{l}) out"), hc, hr);
            }
        }
    }
}

// ------------------------------------------------------------------ dst/src matrix

/// The `(dst, dstCapacity, src, srcSize)` one-shot entry points, with every
/// combination of NULL / zero / oversized.
#[test]
fn null_dst_src_matrix() {
    unsafe {
        let src = gen_class(4, 4096, 2);
        let frame = c_compress(&src, 3);
        let mut oc = vec![0x44u8; 8192];
        let mut or_ = vec![0x44u8; 8192];

        // ZSTD_compress / ZSTD_decompress
        {
            let (cc, cr) = duo::<FnCompress>("ZSTD_compress");
            let (dc, dr) = duo::<FnDecompress>("ZSTD_decompress");
            // Legal boundary combinations only:
            //   * `src == NULL` is only meaningful with `srcSize == 0` (the C
            //     memcpy's / reads `src` unconditionally once srcSize > 0).
            //   * `dst == NULL` is only meaningful with `dstCapacity == 0` for
            //     the *compressors*; the decompressors additionally have an
            //     explicit `dstBuffer_null` check for `dst == NULL &&
            //     dstCapacity > 0`, which is exercised separately below and in
            //     tests/phase_c_decompress.rs::err_dst_too_small.
            // (dst_valid, dstCapacity, src_valid, srcSize)
            let cases: Vec<(bool, usize, bool, usize)> = vec![
                (false, 0, false, 0),
                (false, 0, true, 0),
                (false, 0, true, 4096),
                (true, 0, false, 0),
                (true, 0, true, 0),
                (true, 0, true, 4096),
                (true, 1, true, 0),
                (true, 1, true, 4096),
                (true, 8192, false, 0),
                (true, 8192, true, 0),
                (true, 8192, true, 1),
                (true, 8192, true, 4096),
            ];
            for (dvalid, dcap, svalid, ssz) in cases {
                for lvl in [1, 3, 19] {
                    let dp: *mut c_void = if dvalid {
                        oc.as_mut_ptr() as *mut c_void
                    } else {
                        std::ptr::null_mut()
                    };
                    let dp2: *mut c_void = if dvalid {
                        or_.as_mut_ptr() as *mut c_void
                    } else {
                        std::ptr::null_mut()
                    };
                    let sp: *const c_void = if svalid {
                        src.as_ptr() as *const c_void
                    } else {
                        std::ptr::null()
                    };
                    let x = cc(dp, dcap, sp, ssz, lvl);
                    let y = cr(dp2, dcap, sp, ssz, lvl);
                    eqcode(
                        &format!("ZSTD_compress(dst_valid={dvalid},cap={dcap},src_valid={svalid},n={ssz},lvl={lvl})"),
                        x,
                        y,
                    );
                    if !is_err(x) {
                        eqbuf("ZSTD_compress dst", &oc[..x.min(oc.len())], &or_[..y.min(or_.len())]);
                    }
                }
                let dp: *mut c_void = if dvalid {
                    oc.as_mut_ptr() as *mut c_void
                } else {
                    std::ptr::null_mut()
                };
                let dp2: *mut c_void = if dvalid {
                    or_.as_mut_ptr() as *mut c_void
                } else {
                    std::ptr::null_mut()
                };
                let sp: *const c_void = if svalid {
                    frame.as_ptr() as *const c_void
                } else {
                    std::ptr::null()
                };
                let ssz2 = ssz.min(frame.len());
                let x = dc(dp, dcap, sp, ssz2);
                let y = dr(dp2, dcap, sp, ssz2);
                eqcode(
                    &format!("ZSTD_decompress(dst_valid={dvalid},cap={dcap},src_valid={svalid},n={ssz2})"),
                    x,
                    y,
                );
            }
        }

        // ZSTD_compressCCtx / ZSTD_decompressDCtx / ZSTD_compress2
        {
            let cctx = CtxPair::cctx();
            let dctx = CtxPair::dctx();
            let (cc, cr) = duo::<FnCompressCCtx>("ZSTD_compressCCtx");
            let (dc, dr) = duo::<FnDecompressDCtx>("ZSTD_decompressDCtx");
            let (c2c, c2r) =
                duo::<FnDecompressDCtx>("ZSTD_compress2");
            for (dcap, ssz) in [(0usize, 0usize), (0, 4096), (1, 4096), (8192, 0), (8192, 4096)] {
                eqcode(
                    &format!("ZSTD_compressCCtx(cap={dcap},n={ssz})"),
                    cc(
                        cctx.c,
                        oc.as_mut_ptr() as *mut c_void,
                        dcap,
                        src.as_ptr() as *const c_void,
                        ssz,
                        3,
                    ),
                    cr(
                        cctx.r,
                        or_.as_mut_ptr() as *mut c_void,
                        dcap,
                        src.as_ptr() as *const c_void,
                        ssz,
                        3,
                    ),
                );
                eqcode(
                    &format!("ZSTD_compress2(cap={dcap},n={ssz})"),
                    c2c(
                        cctx.c,
                        oc.as_mut_ptr() as *mut c_void,
                        dcap,
                        src.as_ptr() as *const c_void,
                        ssz,
                    ),
                    c2r(
                        cctx.r,
                        or_.as_mut_ptr() as *mut c_void,
                        dcap,
                        src.as_ptr() as *const c_void,
                        ssz,
                    ),
                );
                eqcode(
                    &format!("ZSTD_decompressDCtx(cap={dcap},n={ssz})"),
                    dc(
                        dctx.c,
                        oc.as_mut_ptr() as *mut c_void,
                        dcap,
                        frame.as_ptr() as *const c_void,
                        ssz.min(frame.len()),
                    ),
                    dr(
                        dctx.r,
                        or_.as_mut_ptr() as *mut c_void,
                        dcap,
                        frame.as_ptr() as *const c_void,
                        ssz.min(frame.len()),
                    ),
                );
            }
            // NULL dst with non-zero capacity -> ZSTD_error_dstBuffer_null
            eqcode(
                "ZSTD_decompressDCtx(dst=NULL,cap=100)",
                dc(
                    dctx.c,
                    std::ptr::null_mut(),
                    100,
                    frame.as_ptr() as *const c_void,
                    frame.len(),
                ),
                dr(
                    dctx.r,
                    std::ptr::null_mut(),
                    100,
                    frame.as_ptr() as *const c_void,
                    frame.len(),
                ),
            );
        }
    }
}

// ------------------------------------------------------------------ NULL dict pointers

#[test]
fn null_dictionary_pointers() {
    unsafe {
        let src = gen_class(4, 4096, 3);
        let frame = c_compress(&src, 3);
        let mut oc = vec![0u8; 8192];
        let mut or_ = vec![0u8; 8192];

        // (dict = NULL, dictSize = 0) must be a no-op == "no dictionary";
        // (dict = NULL, dictSize > 0) must be rejected identically.
        let (clc, clr) = duo::<
            unsafe extern "C" fn(*mut c_void, *const c_void, usize) -> usize,
        >("ZSTD_CCtx_loadDictionary");
        let (clbc, clbr) = duo::<
            unsafe extern "C" fn(*mut c_void, *const c_void, usize) -> usize,
        >("ZSTD_CCtx_loadDictionary_byReference");
        let (rpc, rpr) = duo::<
            unsafe extern "C" fn(*mut c_void, *const c_void, usize) -> usize,
        >("ZSTD_CCtx_refPrefix");
        let (dlc, dlr) = duo::<
            unsafe extern "C" fn(*mut c_void, *const c_void, usize) -> usize,
        >("ZSTD_DCtx_loadDictionary");
        let (dlbc, dlbr) = duo::<
            unsafe extern "C" fn(*mut c_void, *const c_void, usize) -> usize,
        >("ZSTD_DCtx_loadDictionary_byReference");
        let (dpc, dpr) = duo::<
            unsafe extern "C" fn(*mut c_void, *const c_void, usize) -> usize,
        >("ZSTD_DCtx_refPrefix");
        for ds in [0usize, 1, 7, 8, 100] {
            let cctx = CtxPair::cctx();
            let dctx = CtxPair::dctx();
            eqcode(
                &format!("CCtx_loadDictionary(NULL,{ds})"),
                clc(cctx.c, std::ptr::null(), ds),
                clr(cctx.r, std::ptr::null(), ds),
            );
            eqcode(
                &format!("CCtx_loadDictionary_byReference(NULL,{ds})"),
                clbc(cctx.c, std::ptr::null(), ds),
                clbr(cctx.r, std::ptr::null(), ds),
            );
            eqcode(
                &format!("CCtx_refPrefix(NULL,{ds})"),
                rpc(cctx.c, std::ptr::null(), ds),
                rpr(cctx.r, std::ptr::null(), ds),
            );
            eqcode(
                &format!("DCtx_loadDictionary(NULL,{ds})"),
                dlc(dctx.c, std::ptr::null(), ds),
                dlr(dctx.r, std::ptr::null(), ds),
            );
            eqcode(
                &format!("DCtx_loadDictionary_byReference(NULL,{ds})"),
                dlbc(dctx.c, std::ptr::null(), ds),
                dlbr(dctx.r, std::ptr::null(), ds),
            );
            eqcode(
                &format!("DCtx_refPrefix(NULL,{ds})"),
                dpc(dctx.c, std::ptr::null(), ds),
                dpr(dctx.r, std::ptr::null(), ds),
            );
        }

        // ZSTD_CCtx_refCDict(NULL) / ZSTD_DCtx_refDDict(NULL) mean "clear"
        {
            let cctx = CtxPair::cctx();
            let dctx = CtxPair::dctx();
            let (rc, rr) =
                duo::<unsafe extern "C" fn(*mut c_void, *const c_void) -> usize>("ZSTD_CCtx_refCDict");
            eqcode(
                "CCtx_refCDict(NULL)",
                rc(cctx.c, std::ptr::null()),
                rr(cctx.r, std::ptr::null()),
            );
            let (rc, rr) =
                duo::<unsafe extern "C" fn(*mut c_void, *const c_void) -> usize>("ZSTD_DCtx_refDDict");
            eqcode(
                "DCtx_refDDict(NULL)",
                rc(dctx.c, std::ptr::null()),
                rr(dctx.r, std::ptr::null()),
            );
        }

        // ZSTD_createCDict / ZSTD_createDDict with NULL / tiny dictionaries
        {
            let (ccc, ccr) = duo::<
                unsafe extern "C" fn(*const c_void, usize, c_int) -> *mut c_void,
            >("ZSTD_createCDict");
            let (fcc, fcr) = duo::<FnFreePtr>("ZSTD_freeCDict");
            let (dcc, dcr) =
                duo::<unsafe extern "C" fn(*const c_void, usize) -> *mut c_void>("ZSTD_createDDict");
            let (fdc, fdr) = duo::<FnFreePtr>("ZSTD_freeDDict");
            for ds in [0usize, 1, 4, 7, 8] {
                let a = ccc(std::ptr::null(), ds, 3);
                let b = ccr(std::ptr::null(), ds, 3);
                eqv(
                    &format!("createCDict(NULL,{ds}) null?"),
                    a.is_null(),
                    b.is_null(),
                );
                if !a.is_null() {
                    fcc(a);
                    fcr(b);
                }
                let a = dcc(std::ptr::null(), ds);
                let b = dcr(std::ptr::null(), ds);
                eqv(
                    &format!("createDDict(NULL,{ds}) null?"),
                    a.is_null(),
                    b.is_null(),
                );
                if !a.is_null() {
                    fdc(a);
                    fdr(b);
                }
            }
        }

        // one-shot *_usingDict / *_usingDDict with NULL dictionary
        {
            let cctx = CtxPair::cctx();
            let dctx = CtxPair::dctx();
            let (cuc, cur) = duo::<
                unsafe extern "C" fn(
                    *mut c_void,
                    *mut c_void,
                    usize,
                    *const c_void,
                    usize,
                    *const c_void,
                    usize,
                    c_int,
                ) -> usize,
            >("ZSTD_compress_usingDict");
            let (duc, dur) = duo::<
                unsafe extern "C" fn(
                    *mut c_void,
                    *mut c_void,
                    usize,
                    *const c_void,
                    usize,
                    *const c_void,
                    usize,
                ) -> usize,
            >("ZSTD_decompress_usingDict");
            for ds in [0usize, 1, 8, 100] {
                eqcode(
                    &format!("compress_usingDict(NULL,{ds})"),
                    cuc(
                        cctx.c,
                        oc.as_mut_ptr() as *mut c_void,
                        oc.len(),
                        src.as_ptr() as *const c_void,
                        src.len(),
                        std::ptr::null(),
                        ds,
                        3,
                    ),
                    cur(
                        cctx.r,
                        or_.as_mut_ptr() as *mut c_void,
                        or_.len(),
                        src.as_ptr() as *const c_void,
                        src.len(),
                        std::ptr::null(),
                        ds,
                        3,
                    ),
                );
                eqcode(
                    &format!("decompress_usingDict(NULL,{ds})"),
                    duc(
                        dctx.c,
                        oc.as_mut_ptr() as *mut c_void,
                        oc.len(),
                        frame.as_ptr() as *const c_void,
                        frame.len(),
                        std::ptr::null(),
                        ds,
                    ),
                    dur(
                        dctx.r,
                        or_.as_mut_ptr() as *mut c_void,
                        or_.len(),
                        frame.as_ptr() as *const c_void,
                        frame.len(),
                        std::ptr::null(),
                        ds,
                    ),
                );
            }
            // ZSTD_compress_usingCDict / ZSTD_decompress_usingDDict with a NULL
            // dict object: the C checks `cdict` and returns dictionary_wrong /
            // treats NULL DDict as "no dictionary".
            let (ccc, ccr) = duo::<
                unsafe extern "C" fn(
                    *mut c_void,
                    *mut c_void,
                    usize,
                    *const c_void,
                    usize,
                    *const c_void,
                ) -> usize,
            >("ZSTD_compress_usingCDict");
            eqcode(
                "compress_usingCDict(NULL cdict)",
                ccc(
                    cctx.c,
                    oc.as_mut_ptr() as *mut c_void,
                    oc.len(),
                    src.as_ptr() as *const c_void,
                    src.len(),
                    std::ptr::null(),
                ),
                ccr(
                    cctx.r,
                    or_.as_mut_ptr() as *mut c_void,
                    or_.len(),
                    src.as_ptr() as *const c_void,
                    src.len(),
                    std::ptr::null(),
                ),
            );
            let (ddc, ddr) = duo::<
                unsafe extern "C" fn(
                    *mut c_void,
                    *mut c_void,
                    usize,
                    *const c_void,
                    usize,
                    *const c_void,
                ) -> usize,
            >("ZSTD_decompress_usingDDict");
            eqcode(
                "decompress_usingDDict(NULL ddict)",
                ddc(
                    dctx.c,
                    oc.as_mut_ptr() as *mut c_void,
                    oc.len(),
                    frame.as_ptr() as *const c_void,
                    frame.len(),
                    std::ptr::null(),
                ),
                ddr(
                    dctx.r,
                    or_.as_mut_ptr() as *mut c_void,
                    or_.len(),
                    frame.as_ptr() as *const c_void,
                    frame.len(),
                    std::ptr::null(),
                ),
            );
            // NOTE: `ZSTD_DDict_dictContent(NULL)` / `ZSTD_DDict_dictSize(NULL)`
            // guard only with `assert(ddict != NULL)`, which this build compiles
            // to `((void)0)` (DEBUGLEVEL 0), so they dereference NULL and
            // segfault the C reference. Precondition violation, not
            // differentiable; excluded (see CONFIGS.md "C preconditions").
            // The dictID accessors DO check (`if (ddict==NULL) return 0;`,
            // zstd_ddict.c:242 / zstd_compress.c:5816), so they are tested:
            let (idc, idr) =
                duo::<unsafe extern "C" fn(*const c_void) -> c_uint>("ZSTD_getDictID_fromDDict");
            eqv(
                "getDictID_fromDDict(NULL)",
                idc(std::ptr::null()),
                idr(std::ptr::null()),
            );
            let (icc, icr) =
                duo::<unsafe extern "C" fn(*const c_void) -> c_uint>("ZSTD_getDictID_fromCDict");
            eqv(
                "getDictID_fromCDict(NULL)",
                icc(std::ptr::null()),
                icr(std::ptr::null()),
            );
        }
    }
}

// ------------------------------------------------------------------ oversized lengths

#[test]
fn oversized_lengths() {
    unsafe {
        let src = gen_class(4, 1024, 4);
        let mut oc = vec![0u8; 4096];
        let mut or_ = vec![0u8; 4096];

        // NOTE: `ZSTD_compress(dst, cap, src, srcSize, lvl)` with a `srcSize`
        // larger than the buffer `src` actually points at is a caller
        // precondition violation: the C reads `MIN(srcSize, blockSize)` bytes
        // out of `src` *before* any size validation, so it performs a large
        // out-of-bounds read (observed as an intermittent SIGSEGV depending on
        // heap layout). There is no `srcSize` guard to reach that way, so the
        // over-range axis is probed on the *pure* size helpers below instead
        // (`ZSTD_compressBound`, `ZSTD_sequenceBound`,
        // `ZSTD_CCtx_setPledgedSrcSize`, `ZSTD_decodingBufferSize_min`,
        // `ZSTD_estimateDStreamSize`, `ZSTD_writeSkippableFrame`), which DO
        // validate their arguments and never dereference `src`.
        // ZSTD_compressBound with huge inputs
        let (bc, br) = duo::<FnSizeT1>("ZSTD_compressBound");
        for s in [usize::MAX, usize::MAX - 1, 1usize << 63, 1usize << 62] {
            eqv(&format!("compressBound({s})"), bc(s), br(s));
        }
        // ZSTD_setPledgedSrcSize with values beyond the frame-header capacity
        let cctx = CtxPair::cctx();
        let (pc, pr) = duo::<unsafe extern "C" fn(*mut c_void, c_ulonglong) -> usize>(
            "ZSTD_CCtx_setPledgedSrcSize",
        );
        for v in [
            0u64,
            1,
            ZSTD_CONTENTSIZE_UNKNOWN,
            ZSTD_CONTENTSIZE_ERROR,
            u64::MAX - 2,
            1u64 << 40,
        ] {
            eqcode(
                &format!("setPledgedSrcSize({v})"),
                pc(cctx.c, v),
                pr(cctx.r, v),
            );
        }
        // ZSTD_decodingBufferSize_min with huge window / content sizes
        let (dc, dr) = duo::<unsafe extern "C" fn(c_ulonglong, c_ulonglong) -> usize>(
            "ZSTD_decodingBufferSize_min",
        );
        for w in [0u64, 1, 1 << 20, 1u64 << 40, u64::MAX] {
            for f in [0u64, 1, 1 << 20, u64::MAX, ZSTD_CONTENTSIZE_UNKNOWN] {
                eqcode(&format!("decodingBufferSize_min({w},{f})"), dc(w, f), dr(w, f));
            }
        }
        // ZSTD_estimateDStreamSize with huge windows
        let (ec, er) = duo::<FnSizeT1>("ZSTD_estimateDStreamSize");
        for w in [0usize, 1, 1 << 27, 1 << 31, usize::MAX / 2, usize::MAX] {
            eqcode(&format!("estimateDStreamSize({w})"), ec(w), er(w));
        }
        // ZSTD_sequenceBound with huge inputs
        let (sc, sr) = duo::<FnSizeT1>("ZSTD_sequenceBound");
        for s in [usize::MAX, usize::MAX / 2, 1usize << 60] {
            eqv(&format!("sequenceBound({s})"), sc(s), sr(s));
        }
        // ZSTD_writeSkippableFrame with oversized payloads
        let (wc, wr) = duo::<
            unsafe extern "C" fn(*mut c_void, usize, *const c_void, usize, c_uint) -> usize,
        >("ZSTD_writeSkippableFrame");
        for (cap, n) in [
            (8usize, 0usize),
            (7, 0),
            (0, 0),
            (8, usize::MAX),
            (8, 1 << 40),
            (usize::MAX, 0),
        ] {
            let mut a = vec![0xD8u8; oc.len()];
            let mut b = vec![0xD8u8; or_.len()];
            let x = wc(
                a.as_mut_ptr() as *mut c_void,
                cap.min(a.len()),
                src.as_ptr() as *const c_void,
                n,
                0,
            );
            let y = wr(
                b.as_mut_ptr() as *mut c_void,
                cap.min(b.len()),
                src.as_ptr() as *const c_void,
                n,
                0,
            );
            eqcode(&format!("writeSkippableFrame(cap={cap},n={n})"), x, y);
            eqbuf(&format!("writeSkippableFrame(cap={cap},n={n}) dst"), &a, &b);
        }
        // ZSTD_writeLastEmptyBlock with a capacity one byte short
        let (lc, lr) = duo::<unsafe extern "C" fn(*mut c_void, usize) -> usize>(
            "ZSTD_writeLastEmptyBlock",
        );
        for cap in [0usize, 1, 2, 3, 4, 100] {
            // fresh, identically pre-filled buffers so the comparison covers
            // exactly what this call writes
            let mut a = vec![0xD7u8; cap.max(1)];
            let mut b = vec![0xD7u8; cap.max(1)];
            let x = lc(a.as_mut_ptr() as *mut c_void, cap);
            let y = lr(b.as_mut_ptr() as *mut c_void, cap);
            eqcode(&format!("writeLastEmptyBlock(cap={cap})"), x, y);
            eqbuf(&format!("writeLastEmptyBlock(cap={cap}) dst"), &a, &b);
        }
        // ZSTD_readSkippableFrame with a NULL magicVariant out-param (documented
        // as optional) and truncated inputs
        let (rc, rr) = duo::<
            unsafe extern "C" fn(*mut c_void, usize, *mut c_uint, *const c_void, usize) -> usize,
        >("ZSTD_readSkippableFrame");
        let mut sk = vec![0u8; 64];
        let n = wc(
            sk.as_mut_ptr() as *mut c_void,
            sk.len(),
            src.as_ptr() as *const c_void,
            16,
            3,
        );
        assert!(!is_err(n));
        for l in [0usize, 1, 4, 7, 8, 9, n - 1, n] {
            for vnull in [false, true] {
                let mut vc: c_uint = 0xEE;
                let mut vr: c_uint = 0xEE;
                let pvc = if vnull { std::ptr::null_mut() } else { &mut vc };
                let pvr = if vnull { std::ptr::null_mut() } else { &mut vr };
                let mut a = vec![0xD9u8; 256];
                let mut b = vec![0xD9u8; 256];
                let x = rc(
                    a.as_mut_ptr() as *mut c_void,
                    a.len(),
                    pvc,
                    sk.as_ptr() as *const c_void,
                    l,
                );
                let y = rr(
                    b.as_mut_ptr() as *mut c_void,
                    b.len(),
                    pvr,
                    sk.as_ptr() as *const c_void,
                    l,
                );
                eqcode(&format!("readSkippableFrame(len={l},vnull={vnull})"), x, y);
                eqbuf(&format!("readSkippableFrame(len={l},vnull={vnull}) dst"), &a, &b);
                if !vnull {
                    eqv(&format!("readSkippableFrame(len={l}) variant"), vc, vr);
                }
            }
        }
    }
}

// ------------------------------------------------------------------ NULL out-params

#[test]
fn null_out_params() {
    unsafe {
        // ZSTD_CCtx_getParameter / ZSTD_DCtx_getParameter with a NULL value
        // pointer: the C writes `*value` only after the switch validates the
        // parameter, so a NULL pointer with an INVALID parameter is well
        // defined (it returns the error before dereferencing). Both libraries
        // must agree.
        let cctx = CtxPair::cctx();
        let dctx = CtxPair::dctx();
        let (gpc, gpr) = duo::<FnGetParam>("ZSTD_CCtx_getParameter");
        let (dgc, dgr) = duo::<FnGetParam>("ZSTD_DCtx_getParameter");
        let mut rng = Rng::new(0xE001);
        for _ in 0..500 {
            let p = rng.next_u32() as c_int;
            if ALL_CPARAMS.iter().any(|(_, q)| *q == p) {
                continue;
            }
            eqcode(
                &format!("CCtx_getParameter(bogus {p}, NULL)"),
                gpc(cctx.c, p, std::ptr::null_mut()),
                gpr(cctx.r, p, std::ptr::null_mut()),
            );
        }
        for _ in 0..500 {
            let p = rng.next_u32() as c_int;
            if ALL_DPARAMS.iter().any(|(_, q)| *q == p) {
                continue;
            }
            eqcode(
                &format!("DCtx_getParameter(bogus {p}, NULL)"),
                dgc(dctx.c, p, std::ptr::null_mut()),
                dgr(dctx.r, p, std::ptr::null_mut()),
            );
        }
    }
}
