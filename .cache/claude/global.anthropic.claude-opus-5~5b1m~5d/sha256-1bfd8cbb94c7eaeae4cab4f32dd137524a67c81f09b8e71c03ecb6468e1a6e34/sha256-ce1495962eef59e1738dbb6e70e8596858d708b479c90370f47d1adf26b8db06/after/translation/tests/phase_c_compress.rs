//! Phase C — ERRORS.md rows covered by `phase_c_compress`.
//!
//! Every rejection site on the **compression** side is reached by *constructing*
//! the invalid input/state and asserting that the C and the Rust `.so` return
//! the SAME `ZSTD_ErrorCode` (not merely "both failed"), plus identical `dst`
//! bytes and identical `in.pos` / `out.pos`.
//!
//! Sections of `ERRORS.md` covered here:
//!   * `compress/zstd_compress.c`            (dstSize_tooSmall, srcSize_wrong,
//!     stage_wrong, init_missing, memory_allocation, workSpace_tooSmall,
//!     cannotProduce_uncompressedBlock, externalSequences_invalid,
//!     sequenceProducer_failed, stabilityCondition_notRespected, GENERIC,
//!     dictionary_corrupted / dictionary_wrong, and every `ZSTD_create*` /
//!     `ZSTD_initStatic*` `return NULL`)
//!   * `compress/zstd_cwksp.h`
//!   * `compress/zstd_compress_internal.h`   (ZSTD_noCompressBlock / ZSTD_rleCompressBlock)
//!   * `compress/zstd_compress_literals.c`
//!   * `compress/zstd_compress_sequences.c`
//!   * `compress/zstd_compress_superblock.c`
//!   * `compress/zstd_ldm.c`
//!
//! `parameter_outOfBound` / `parameter_unsupported` /
//! `parameter_combination_unsupported` and the out-of-range enum values live in
//! `phase_c_params.rs`; the whole decoder lives in `phase_c_decompress.rs`.
//!
//! NOTE (`noForwardProgress_destFull` / `noForwardProgress_inputEmpty` /
//! `dstBuffer_null` / `dstBuffer_wrong`): grepping `c_src/src` shows these four
//! codes are produced **only** by `decompress/zstd_decompress.c`
//! (L2359/L2360/L903/L916/L2049). The compressor has no zero-progress bail-out
//! at all: `ZSTD_compressStream2` returns "bytes still to flush" forever without
//! ever counting stalled calls. The zero-progress loops are still driven here
//! (`err_zero_progress_loops`) to prove that both libraries loop identically and
//! neither invents an error; the four codes themselves are covered by
//! `phase_c_decompress.rs`.
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(dead_code)]
mod common;
use common::*;
use std::ffi::{c_int, c_uint, c_ulonglong, c_void};
use std::ptr;

// ---------------------------------------------------------------- fn types

type FnCompress2 = unsafe extern "C" fn(*mut c_void, *mut c_void, usize, *const c_void, usize) -> usize;
type FnCompressUsingDict = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    usize,
    *const c_void,
    usize,
    *const c_void,
    usize,
    c_int,
) -> usize;
type FnCompressUsingCDict =
    unsafe extern "C" fn(*mut c_void, *mut c_void, usize, *const c_void, usize, *const c_void) -> usize;
type FnCompressUsingCDictAdv = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    usize,
    *const c_void,
    usize,
    *const c_void,
    ZSTD_frameParameters,
) -> usize;
type FnStream1 = unsafe extern "C" fn(*mut c_void, *mut ZSTD_outBuffer, *mut ZSTD_inBuffer) -> usize;
type FnFlush = unsafe extern "C" fn(*mut c_void, *mut ZSTD_outBuffer) -> usize;
type FnInitCStream = unsafe extern "C" fn(*mut c_void, c_int) -> usize;
type FnSimpleArgs = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    usize,
    *mut usize,
    *const c_void,
    usize,
    *mut usize,
    c_int,
) -> usize;
type FnBufferless = unsafe extern "C" fn(*mut c_void, *mut c_void, usize, *const c_void, usize) -> usize;
type FnCompressBegin = unsafe extern "C" fn(*mut c_void, c_int) -> usize;
type FnCompressBeginAdvanced = unsafe extern "C" fn(
    *mut c_void,
    *const c_void,
    usize,
    ZSTD_parameters,
    c_ulonglong,
) -> usize;
type FnCompressBeginUsingCDict = unsafe extern "C" fn(*mut c_void, *const c_void) -> usize;
type FnCopyCCtx = unsafe extern "C" fn(*mut c_void, *const c_void, c_ulonglong) -> usize;
type FnPledged = unsafe extern "C" fn(*mut c_void, c_ulonglong) -> usize;
type FnInitStatic = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
type FnInitStaticCDict = unsafe extern "C" fn(
    *mut c_void,
    usize,
    *const c_void,
    usize,
    c_int,
    c_int,
    ZSTD_compressionParameters,
) -> *const c_void;
type FnCreateCCtxAdvanced = unsafe extern "C" fn(ZSTD_customMem) -> *mut c_void;
type FnCreateCDictAdvanced = unsafe extern "C" fn(
    *const c_void,
    usize,
    c_int,
    c_int,
    ZSTD_compressionParameters,
    ZSTD_customMem,
) -> *mut c_void;
type FnCreateCDictAdvanced2 = unsafe extern "C" fn(
    *const c_void,
    usize,
    c_int,
    c_int,
    *const c_void,
    ZSTD_customMem,
) -> *mut c_void;
type FnLoadDictAdvanced =
    unsafe extern "C" fn(*mut c_void, *const c_void, usize, c_int, c_int) -> usize;
type FnLoadDict = unsafe extern "C" fn(*mut c_void, *const c_void, usize) -> usize;
type FnGetCParams =
    unsafe extern "C" fn(c_int, c_ulonglong, usize) -> ZSTD_compressionParameters;
type FnGetParams = unsafe extern "C" fn(c_int, c_ulonglong, usize) -> ZSTD_parameters;
type FnEstimateFromParams = unsafe extern "C" fn(*const c_void) -> usize;
type FnWriteSkippable =
    unsafe extern "C" fn(*mut c_void, usize, *const c_void, usize, c_uint) -> usize;
type FnWriteLastEmpty = unsafe extern "C" fn(*mut c_void, usize) -> usize;
type FnGenSeq =
    unsafe extern "C" fn(*mut c_void, *mut ZSTD_Sequence, usize, *const c_void, usize) -> usize;
type FnMerge = unsafe extern "C" fn(*mut ZSTD_Sequence, usize) -> usize;
type FnCompressSeq = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    usize,
    *const ZSTD_Sequence,
    usize,
    *const c_void,
    usize,
) -> usize;
type FnCompressSeqLit = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    usize,
    *const ZSTD_Sequence,
    usize,
    *const c_void,
    usize,
    usize,
    usize,
) -> usize;
type FnSeqProducer = unsafe extern "C" fn(
    *mut c_void,
    *mut ZSTD_Sequence,
    usize,
    *const c_void,
    usize,
    *const c_void,
    usize,
    c_int,
    usize,
) -> usize;
type FnRegisterSeqProd =
    unsafe extern "C" fn(*mut c_void, *mut c_void, Option<FnSeqProducer>);
type FnNoCompressLiterals =
    unsafe extern "C" fn(*mut c_void, usize, *const c_void, usize) -> usize;
type FnGetSeqStore = unsafe extern "C" fn(*const c_void) -> *const c_void;
type FnGetBlockSize = unsafe extern "C" fn(*const c_void) -> usize;

// ---------------------------------------------------------------- eqcode

/// Compare the *error code*, not just "both failed".
#[track_caller]
fn eqcode(what: &str, c: usize, r: usize) {
    unsafe {
        let (gcc, gcr) = duo::<unsafe extern "C" fn(usize) -> c_uint>("ZSTD_getErrorCode");
        let (nc, nr) = duo::<FnErrName>("ZSTD_getErrorName");
        if c != r {
            panic!(
                "{what}: C returned {c:#x} (code {} = {}), Rust returned {r:#x} (code {} = {})",
                gcc(c),
                cstr(nc(c)),
                gcr(r),
                cstr(nr(r))
            );
        }
        assert_eq!(gcc(c), gcr(r), "{what}: error code mismatch");
        assert_eq!(cstr(nc(c)), cstr(nr(r)), "{what}: error name mismatch");
    }
}

/// Assert that C produced exactly `code` (by name) — pins the row down to the
/// intended rejection site instead of "some error".
#[track_caller]
fn expect_code(what: &str, n: usize, code: &str) {
    unsafe {
        let (nc, _) = duo::<FnErrName>("ZSTD_getErrorName");
        let got = cstr(nc(n));
        assert_eq!(got, code, "{what}: expected C error `{code}`, got `{got}` ({n:#x})");
    }
}

fn errname(n: usize) -> String {
    unsafe {
        let (nc, _) = duo::<FnErrName>("ZSTD_getErrorName");
        cstr(nc(n))
    }
}

// ---------------------------------------------------------------- misc helpers

/// The destination capacities that matter for a frame whose real size is `exact`.
fn caps_for(exact: usize) -> Vec<usize> {
    let mut v = vec![0usize, 1, 2, 3, 4, 5, 8, 12, 17, 18, 19];
    if exact > 0 {
        v.push(exact - 1);
    }
    v.push(exact);
    if exact > 2 {
        v.push(exact / 2);
    }
    v.retain(|&c| c <= exact.max(19));
    v.sort();
    v.dedup();
    v
}

/// 8-byte aligned scratch workspace, with an adjustable *misalignment*.
struct Ws {
    buf: Vec<u64>,
    off: usize,
    len: usize,
}

impl Ws {
    fn new(len: usize, off: usize) -> Ws {
        Ws { buf: vec![0u64; len / 8 + 4], off, len }
    }
    fn ptr(&mut self) -> *mut c_void {
        unsafe { (self.buf.as_mut_ptr() as *mut u8).add(self.off) as *mut c_void }
    }
    fn bytes(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(
                (self.buf.as_ptr() as *const u8).add(self.off),
                self.len.min(self.buf.len() * 8 - self.off),
            )
        }
    }
}

// ------------------------------------------------------------------ row 81

#[test]
fn err_compressbound_srcsize_wrong() {
    unsafe {
        let (bc, br) = duo::<FnSizeT1>("ZSTD_compressBound");
        // ZSTD_COMPRESSBOUND() yields 0 when srcSize > ZSTD_MAX_INPUT_SIZE.
        let mut cases: Vec<usize> = vec![
            0,
            1,
            128 * 1024,
            1 << 30,
            usize::MAX,
            usize::MAX - 1,
            usize::MAX / 2,
            (usize::MAX / 8) * 7,
        ];
        let mut rng = Rng::new(0xC100);
        for _ in 0..400 {
            cases.push(rng.next_u64() as usize);
        }
        let mut saw_error = false;
        for s in cases {
            let a = bc(s);
            let b = br(s);
            eqcode(&format!("compressBound({s})"), a, b);
            if is_err(a) {
                expect_code(&format!("compressBound({s})"), a, "Src size is incorrect");
                saw_error = true;
            }
        }
        assert!(saw_error, "row 81 (compressBound srcSize_wrong) never triggered");
    }
}

// ------------------------------------------------------------------ rows 91, 92

#[test]
fn err_cctxparams_init_null_generic() {
    unsafe {
        let (ic, ir) = duo::<unsafe extern "C" fn(*mut c_void, c_int) -> usize>("ZSTD_CCtxParams_init");
        let (ac, ar) = duo::<unsafe extern "C" fn(*mut c_void, ZSTD_parameters) -> usize>(
            "ZSTD_CCtxParams_init_advanced",
        );
        let (gp, _) = duo::<FnGetParams>("ZSTD_getParams");
        for lvl in [-131072, -5, 0, 1, 3, 19, 22, 99] {
            let a = ic(ptr::null_mut(), lvl);
            let b = ir(ptr::null_mut(), lvl);
            eqcode(&format!("CCtxParams_init(NULL,{lvl})"), a, b);
            expect_code(&format!("CCtxParams_init(NULL,{lvl})"), a, "Error (generic)");
        }
        for lvl in [1, 3, 19] {
            for &ss in &[0u64, 1024, 1 << 20] {
                let p = gp(lvl, ss, 0);
                let a = ac(ptr::null_mut(), p);
                let b = ar(ptr::null_mut(), p);
                eqcode(&format!("CCtxParams_init_advanced(NULL,{lvl},{ss})"), a, b);
                expect_code(
                    &format!("CCtxParams_init_advanced(NULL,{lvl},{ss})"),
                    a,
                    "Error (generic)",
                );
            }
        }
        // An *invalid* ZSTD_parameters on a real object must be rejected too
        // (ZSTD_checkCParams forwarding, not a NULL check).
        let po = CtxPair::cctx_params();
        for bad in [
            ZSTD_parameters {
                cParams: ZSTD_compressionParameters {
                    windowLog: 9,
                    chainLog: 16,
                    hashLog: 17,
                    searchLog: 1,
                    minMatch: 4,
                    targetLength: 0,
                    strategy: 4,
                },
                fParams: ZSTD_frameParameters::default(),
            },
            ZSTD_parameters {
                cParams: ZSTD_compressionParameters {
                    windowLog: 40,
                    chainLog: 16,
                    hashLog: 17,
                    searchLog: 1,
                    minMatch: 4,
                    targetLength: 0,
                    strategy: 4,
                },
                fParams: ZSTD_frameParameters::default(),
            },
            ZSTD_parameters {
                cParams: ZSTD_compressionParameters {
                    windowLog: 20,
                    chainLog: 16,
                    hashLog: 17,
                    searchLog: 1,
                    minMatch: 2,
                    targetLength: 0,
                    strategy: 4,
                },
                fParams: ZSTD_frameParameters::default(),
            },
        ] {
            let a = ac(po.c, bad);
            let b = ar(po.r, bad);
            eqcode(&format!("CCtxParams_init_advanced(bad {bad:?})"), a, b);
        }
    }
}

// ------------------------------------------------------------------ rows 82, 83, 89, 90

#[test]
fn err_create_advanced_half_custom_mem_null() {
    unsafe {
        let (cc, cr) = duo::<FnCreateCCtxAdvanced>("ZSTD_createCCtx_advanced");
        let (sc, sr) = duo::<FnCreateCCtxAdvanced>("ZSTD_createCStream_advanced");
        let (fcc, fcr) = duo::<FnFreePtr>("ZSTD_freeCCtx");
        let (fsc, fsr) = duo::<FnFreePtr>("ZSTD_freeCStream");
        let (dc, dr) = duo::<FnCreateCDictAdvanced>("ZSTD_createCDict_advanced");
        let (fdc, fdr) = duo::<FnFreePtr>("ZSTD_freeCDict");
        let (gc, _) = duo::<FnGetCParams>("ZSTD_getCParams");

        // (!alloc) ^ (!free)  -> NULL
        let half_a = ZSTD_customMem {
            customAlloc: Some(alloc_never),
            customFree: None,
            opaque: ptr::null_mut(),
        };
        let half_b = ZSTD_customMem {
            customAlloc: None,
            customFree: Some(free_noop),
            opaque: ptr::null_mut(),
        };
        // both set, but the allocator always fails -> also NULL, from a
        // different `return NULL;` site
        let failing = ZSTD_customMem {
            customAlloc: Some(alloc_never),
            customFree: Some(free_noop),
            opaque: ptr::null_mut(),
        };
        let dict = gen_class(4, 4096, 1);
        let cp = gc(3, 0, dict.len());

        for (tag, m) in [("halfA", half_a), ("halfB", half_b), ("failing", failing)] {
            let a = cc(m);
            let b = cr(m);
            eqv(&format!("createCCtx_advanced({tag}) NULL?"), a.is_null(), b.is_null());
            assert!(a.is_null(), "createCCtx_advanced({tag}) should be NULL in C");
            if !a.is_null() {
                fcc(a);
                fcr(b);
            }

            let a = sc(m);
            let b = sr(m);
            eqv(&format!("createCStream_advanced({tag}) NULL?"), a.is_null(), b.is_null());
            assert!(a.is_null(), "createCStream_advanced({tag}) should be NULL in C");
            if !a.is_null() {
                fsc(a);
                fsr(b);
            }

            for dlm in [ZSTD_dlm_byCopy, ZSTD_dlm_byRef] {
                for dct in [ZSTD_dct_auto, ZSTD_dct_rawContent, ZSTD_dct_fullDict] {
                    let a = dc(
                        dict.as_ptr() as *const c_void,
                        dict.len(),
                        dlm,
                        dct,
                        cp,
                        m,
                    );
                    let b = dr(
                        dict.as_ptr() as *const c_void,
                        dict.len(),
                        dlm,
                        dct,
                        cp,
                        m,
                    );
                    eqv(
                        &format!("createCDict_advanced({tag},{dlm},{dct}) NULL?"),
                        a.is_null(),
                        b.is_null(),
                    );
                    assert!(a.is_null(), "createCDict_advanced({tag}) should be NULL in C");
                    if !a.is_null() {
                        fdc(a);
                        fdr(b);
                    }
                }
            }
        }
    }
}

unsafe extern "C" fn alloc_never(_opaque: *mut c_void, _size: usize) -> *mut c_void {
    ptr::null_mut()
}

unsafe extern "C" fn free_noop(_opaque: *mut c_void, _p: *mut c_void) {}

// A budgeted allocator: hands out `LIMIT` successful allocations, then fails.
// The budget lives in the `opaque` word so each library gets its own counter.
#[repr(C)]
struct Budget {
    remaining: c_int,
    live: usize,
}

unsafe extern "C" fn alloc_budget(opaque: *mut c_void, size: usize) -> *mut c_void {
    let b = &mut *(opaque as *mut Budget);
    if b.remaining <= 0 {
        return ptr::null_mut();
    }
    b.remaining -= 1;
    let layout =
        std::alloc::Layout::from_size_align(size.max(1) + 16, 16).unwrap();
    // MUST be zeroed: `ZSTD_customMalloc` hands the block straight to
    // `ZSTD_cwksp_create`, and comparing two libraries that were each fed
    // *uninitialised* memory is not a differential test - the observable
    // compressed output would depend on this process' allocation history.
    let p = std::alloc::alloc_zeroed(layout);
    if p.is_null() {
        return ptr::null_mut();
    }
    // stash the size just before the returned pointer so free can rebuild it
    *(p as *mut usize) = size.max(1) + 16;
    b.live += 1;
    p.add(16) as *mut c_void
}

unsafe extern "C" fn free_budget(opaque: *mut c_void, p: *mut c_void) {
    if p.is_null() {
        return;
    }
    let b = &mut *(opaque as *mut Budget);
    let base = (p as *mut u8).sub(16);
    let total = *(base as *mut usize);
    let layout = std::alloc::Layout::from_size_align(total, 16).unwrap();
    std::alloc::dealloc(base, layout);
    b.live -= 1;
}

// ------------------------------------------------------------------ rows 110, 113, 122-125, 171, 172-176, 238

#[test]
fn err_allocation_failure_paths() {
    unsafe {
        let (cc, cr) = duo::<FnCreateCCtxAdvanced>("ZSTD_createCCtx_advanced");
        let (fcc, fcr) = duo::<FnFreePtr>("ZSTD_freeCCtx");
        let (c2c, c2r) = duo::<FnCompress2>("ZSTD_compress2");
        let (ldc, ldr) = duo::<FnLoadDict>("ZSTD_CCtx_loadDictionary");
        let (spc, spr) = duo::<FnSetParam>("ZSTD_CCtx_setParameter");
        let (rsc, rsr) = duo::<FnReset>("ZSTD_CCtx_reset");
        let (bd, _) = duo::<FnSizeT1>("ZSTD_compressBound");
        let (dc, dr) = duo::<FnCreateCDictAdvanced>("ZSTD_createCDict_advanced");
        let (d2c, d2r) = duo::<FnCreateCDictAdvanced2>("ZSTD_createCDict_advanced2");
        let (fdc, fdr) = duo::<FnFreePtr>("ZSTD_freeCDict");
        let (gc, _) = duo::<FnGetCParams>("ZSTD_getCParams");

        let dict = gen_class(4, 8192, 7);
        let src = gen_class(4, 20_000, 8);
        let cp = gc(5, 0, dict.len());
        let po = CtxPair::cctx_params();

        for limit in 0..12 {
            // ---- ZSTD_createCDict_advanced / _advanced2 under a budget
            let mut bc = Budget { remaining: limit, live: 0 };
            let mut br = Budget { remaining: limit, live: 0 };
            let mc = ZSTD_customMem {
                customAlloc: Some(alloc_budget),
                customFree: Some(free_budget),
                opaque: &mut bc as *mut Budget as *mut c_void,
            };
            let mr = ZSTD_customMem {
                customAlloc: Some(alloc_budget),
                customFree: Some(free_budget),
                opaque: &mut br as *mut Budget as *mut c_void,
            };
            let a = dc(dict.as_ptr() as *const c_void, dict.len(), ZSTD_dlm_byCopy, ZSTD_dct_auto, cp, mc);
            let b = dr(dict.as_ptr() as *const c_void, dict.len(), ZSTD_dlm_byCopy, ZSTD_dct_auto, cp, mr);
            eqv(&format!("createCDict_advanced(limit={limit}) NULL?"), a.is_null(), b.is_null());
            if !a.is_null() {
                fdc(a);
            }
            if !b.is_null() {
                fdr(b);
            }
            assert_eq!(bc.live, 0, "C leaked {} blocks at limit={limit}", bc.live);
            assert_eq!(br.live, 0, "Rust leaked {} blocks at limit={limit}", br.live);

            let mut bc = Budget { remaining: limit, live: 0 };
            let mut br = Budget { remaining: limit, live: 0 };
            let mc = ZSTD_customMem {
                customAlloc: Some(alloc_budget),
                customFree: Some(free_budget),
                opaque: &mut bc as *mut Budget as *mut c_void,
            };
            let mr = ZSTD_customMem {
                customAlloc: Some(alloc_budget),
                customFree: Some(free_budget),
                opaque: &mut br as *mut Budget as *mut c_void,
            };
            let a = d2c(
                dict.as_ptr() as *const c_void,
                dict.len(),
                ZSTD_dlm_byCopy,
                ZSTD_dct_auto,
                po.c,
                mc,
            );
            let b = d2r(
                dict.as_ptr() as *const c_void,
                dict.len(),
                ZSTD_dlm_byCopy,
                ZSTD_dct_auto,
                po.r,
                mr,
            );
            eqv(&format!("createCDict_advanced2(limit={limit}) NULL?"), a.is_null(), b.is_null());
            if !a.is_null() {
                fdc(a);
            }
            if !b.is_null() {
                fdr(b);
            }

            // ---- a CCtx whose *internal* allocations fail
            let mut bc = Budget { remaining: limit, live: 0 };
            let mut br = Budget { remaining: limit, live: 0 };
            let mc = ZSTD_customMem {
                customAlloc: Some(alloc_budget),
                customFree: Some(free_budget),
                opaque: &mut bc as *mut Budget as *mut c_void,
            };
            let mr = ZSTD_customMem {
                customAlloc: Some(alloc_budget),
                customFree: Some(free_budget),
                opaque: &mut br as *mut Budget as *mut c_void,
            };
            let xc = cc(mc);
            let xr = cr(mr);
            eqv(&format!("createCCtx_advanced(limit={limit}) NULL?"), xc.is_null(), xr.is_null());
            if xc.is_null() {
                assert!(xr.is_null());
                continue;
            }
            // loadDictionary needs to allocate an internal CDict
            let a = ldc(xc, dict.as_ptr() as *const c_void, dict.len());
            let b = ldr(xr, dict.as_ptr() as *const c_void, dict.len());
            eqcode(&format!("loadDictionary(limit={limit})"), a, b);

            eqcode(
                &format!("setParameter(limit={limit})"),
                spc(xc, ZSTD_c_compressionLevel, 5),
                spr(xr, ZSTD_c_compressionLevel, 5),
            );
            let cap = bd(src.len()) + 64;
            let mut oc = vec![0x5Au8; cap];
            let mut or_ = vec![0x5Au8; cap];
            let a = c2c(
                xc,
                oc.as_mut_ptr() as *mut c_void,
                cap,
                src.as_ptr() as *const c_void,
                src.len(),
            );
            let b = c2r(
                xr,
                or_.as_mut_ptr() as *mut c_void,
                cap,
                src.as_ptr() as *const c_void,
                src.len(),
            );
            eqcode(&format!("compress2(limit={limit})"), a, b);
            eqbuf(&format!("compress2(limit={limit}) dst"), &oc, &or_);

            // and again after a full reset (exercises ZSTD_resetCCtx_internal)
            eqcode(
                &format!("reset(limit={limit})"),
                rsc(xc, ZSTD_reset_session_and_parameters),
                rsr(xr, ZSTD_reset_session_and_parameters),
            );
            let a = c2c(
                xc,
                oc.as_mut_ptr() as *mut c_void,
                cap,
                src.as_ptr() as *const c_void,
                src.len(),
            );
            let b = c2r(
                xr,
                or_.as_mut_ptr() as *mut c_void,
                cap,
                src.as_ptr() as *const c_void,
                src.len(),
            );
            eqcode(&format!("compress2 after reset(limit={limit})"), a, b);
            eqbuf(&format!("compress2 after reset(limit={limit}) dst"), &oc, &or_);

            fcc(xc);
            fcr(xr);
            assert_eq!(bc.live, 0, "C leaked at limit={limit}");
            assert_eq!(br.live, 0, "Rust leaked at limit={limit}");
        }
    }
}

// ------------------------------------------------------------------ rows 84-88, 112, 177-180, 231-238

#[test]
fn err_static_init_and_workspaces() {
    unsafe {
        let (icc, icr) = duo::<FnInitStatic>("ZSTD_initStaticCCtx");
        let (isc, isr) = duo::<FnInitStatic>("ZSTD_initStaticCStream");
        let (idc, idr) = duo::<FnInitStaticCDict>("ZSTD_initStaticCDict");
        let (ecc, _) = duo::<unsafe extern "C" fn(c_int) -> usize>("ZSTD_estimateCCtxSize");
        let (esc, _) = duo::<unsafe extern "C" fn(c_int) -> usize>("ZSTD_estimateCStreamSize");
        let (edc, _) = duo::<unsafe extern "C" fn(usize, c_int) -> usize>("ZSTD_estimateCDictSize");
        let (fcc, fcr) = duo::<FnFreePtr>("ZSTD_freeCCtx");
        let (fsc, fsr) = duo::<FnFreePtr>("ZSTD_freeCStream");
        let (fdc, fdr) = duo::<FnFreePtr>("ZSTD_freeCDict");
        let (ldc, ldr) = duo::<FnLoadDictAdvanced>("ZSTD_CCtx_loadDictionary_advanced");
        let (gc, _) = duo::<FnGetCParams>("ZSTD_getCParams");
        let (c2c, c2r) = duo::<FnCompress2>("ZSTD_compress2");
        let (bd, _) = duo::<FnSizeT1>("ZSTD_compressBound");

        // ---- NULL workspace, size 0, sub-`sizeof(ZSTD_CCtx)`, misaligned
        for &n in &[0usize, 1, 7, 8, 64, 1024, 4096] {
            let a = icc(ptr::null_mut(), n);
            let b = icr(ptr::null_mut(), n);
            eqv(&format!("initStaticCCtx(NULL,{n}) NULL?"), a.is_null(), b.is_null());
            assert!(a.is_null(), "initStaticCCtx(NULL,{n}) must be NULL in C");
            let a = isc(ptr::null_mut(), n);
            let b = isr(ptr::null_mut(), n);
            eqv(&format!("initStaticCStream(NULL,{n}) NULL?"), a.is_null(), b.is_null());
            assert!(a.is_null(), "initStaticCStream(NULL,{n}) must be NULL in C");
        }

        let lvl = 5;
        let need_cctx = ecc(lvl);
        let need_cs = esc(lvl);
        let mut sizes: Vec<usize> = vec![0, 1, 7, 8, 16, 64, 256, 1024, 4096, 65536];
        for base in [need_cctx, need_cs] {
            sizes.push(base);
            sizes.push(base - 1);
            sizes.push(base / 2);
            sizes.push(base + 64);
        }
        sizes.sort();
        sizes.dedup();

        let mut nulls_cctx = 0usize;
        let mut oks_cctx = 0usize;
        for &n in &sizes {
            for off in 0..8usize {
                let mut wc = Ws::new(n + 16, off);
                let mut wr = Ws::new(n + 16, off);
                let a = icc(wc.ptr(), n);
                let b = icr(wr.ptr(), n);
                eqv(
                    &format!("initStaticCCtx(n={n},off={off}) NULL?"),
                    a.is_null(),
                    b.is_null(),
                );
                if a.is_null() {
                    nulls_cctx += 1;
                } else {
                    oks_cctx += 1;
                    // a static CCtx must refuse to be freed and refuse a dictionary
                    let x = fcc(a);
                    let y = fcr(b);
                    eqcode(&format!("freeCCtx(static n={n},off={off})"), x, y);
                    expect_code(
                        &format!("freeCCtx(static n={n})"),
                        x,
                        "Allocation error : not enough memory",
                    );
                    let d = gen_class(4, 512, 1);
                    let x = ldc(a, d.as_ptr() as *const c_void, d.len(), ZSTD_dlm_byCopy, ZSTD_dct_auto);
                    let y = ldr(b, d.as_ptr() as *const c_void, d.len(), ZSTD_dlm_byCopy, ZSTD_dct_auto);
                    eqcode(&format!("loadDictionary(static n={n},off={off})"), x, y);
                    expect_code(
                        &format!("loadDictionary(static n={n})"),
                        x,
                        "Allocation error : not enough memory",
                    );
                    // and a compression on the (possibly barely-sufficient)
                    // workspace must agree, error or not
                    let s = gen_class(4, 4000, 2);
                    let cap = bd(s.len());
                    let mut oc = vec![0x11u8; cap];
                    let mut or_ = vec![0x11u8; cap];
                    let x = c2c(a, oc.as_mut_ptr() as *mut c_void, cap, s.as_ptr() as *const c_void, s.len());
                    let y = c2r(b, or_.as_mut_ptr() as *mut c_void, cap, s.as_ptr() as *const c_void, s.len());
                    eqcode(&format!("compress2(static n={n},off={off})"), x, y);
                    eqbuf(&format!("compress2(static n={n},off={off}) dst"), &oc, &or_);
                }

                let mut wc = Ws::new(n + 16, off);
                let mut wr = Ws::new(n + 16, off);
                let a = isc(wc.ptr(), n);
                let b = isr(wr.ptr(), n);
                eqv(
                    &format!("initStaticCStream(n={n},off={off}) NULL?"),
                    a.is_null(),
                    b.is_null(),
                );
                if !a.is_null() {
                    let x = fsc(a);
                    let y = fsr(b);
                    eqcode(&format!("freeCStream(static n={n},off={off})"), x, y);
                }
            }
        }
        assert!(nulls_cctx > 0 && oks_cctx > 0, "static CCtx grid degenerate");

        // ---- ZSTD_initStaticCDict
        let dict = gen_class(4, 4096, 3);
        let cp = gc(3, 0, dict.len());
        let need_cd = edc(dict.len(), 3);
        let mut dsizes: Vec<usize> = vec![0, 1, 8, 64, 1024];
        dsizes.push(need_cd);
        dsizes.push(need_cd - 1);
        dsizes.push(need_cd / 2);
        dsizes.push(need_cd + 64);
        dsizes.sort();
        dsizes.dedup();
        for &n in &dsizes {
            for off in [0usize, 1, 4, 7] {
                for dlm in [ZSTD_dlm_byCopy, ZSTD_dlm_byRef] {
                    for dct in [ZSTD_dct_auto, ZSTD_dct_rawContent, ZSTD_dct_fullDict] {
                        let mut wc = Ws::new(n + 16, off);
                        let mut wr = Ws::new(n + 16, off);
                        let a = idc(
                            wc.ptr(),
                            n,
                            dict.as_ptr() as *const c_void,
                            dict.len(),
                            dlm,
                            dct,
                            cp,
                        );
                        let b = idr(
                            wr.ptr(),
                            n,
                            dict.as_ptr() as *const c_void,
                            dict.len(),
                            dlm,
                            dct,
                            cp,
                        );
                        let tag = format!("initStaticCDict(n={n},off={off},dlm={dlm},dct={dct})");
                        eqv(&format!("{tag} NULL?"), a.is_null(), b.is_null());
                        if !a.is_null() {
                            let x = fdc(a as *mut c_void);
                            let y = fdr(b as *mut c_void);
                            eqcode(&format!("{tag} freeCDict"), x, y);
                        }
                        // NULL workspace
                        let a = idc(
                            ptr::null_mut(),
                            n,
                            dict.as_ptr() as *const c_void,
                            dict.len(),
                            dlm,
                            dct,
                            cp,
                        );
                        let b = idr(
                            ptr::null_mut(),
                            n,
                            dict.as_ptr() as *const c_void,
                            dict.len(),
                            dlm,
                            dct,
                            cp,
                        );
                        eqv(&format!("{tag} NULL ws"), a.is_null(), b.is_null());
                        assert!(a.is_null(), "{tag}: NULL workspace must give NULL");
                    }
                }
            }
        }
    }
}

// ------------------------------------------------------------------ rows 143, 144, 168, 169, 192,
// 220, 221, 222, 223, 226, 228, 229, 127, 128, 139, 142, 4487
//
// Every one-shot compressor with dstCapacity = 0, 1, ..., exact-1 and with
// dst == NULL. Truncating at every offset walks the whole
// ZSTD_compress_frameChunk -> ZSTD_compressBlock_internal ->
// ZSTD_entropyCompressSeqStore -> {ZSTD_compressLiterals, ZSTD_buildCTable,
// ZSTD_encodeSequences} chain and the ZSTD_noCompressBlock /
// ZSTD_rleCompressBlock / ZSTD_writeEpilogue fallbacks.

#[test]
fn err_oneshot_dst_too_small() {
    unsafe {
        let (cc, cr) = duo::<FnCompress>("ZSTD_compress");
        let (ccc, ccr) = duo::<FnCompressCCtx>("ZSTD_compressCCtx");
        let (c2c, c2r) = duo::<FnCompress2>("ZSTD_compress2");
        let (udc, udr) = duo::<FnCompressUsingDict>("ZSTD_compress_usingDict");
        let (cdc, cdr) = duo::<FnCompressUsingCDict>("ZSTD_compress_usingCDict");
        let (cac, car) = duo::<FnCompressUsingCDictAdv>("ZSTD_compress_usingCDict_advanced");
        let (mkc, mkr) = duo::<unsafe extern "C" fn(*const c_void, usize, c_int) -> *mut c_void>(
            "ZSTD_createCDict",
        );
        let (fdc, fdr) = duo::<FnFreePtr>("ZSTD_freeCDict");
        let (spc, spr) = duo::<FnSetParam>("ZSTD_CCtx_setParameter");
        let (rsc, rsr) = duo::<FnReset>("ZSTD_CCtx_reset");
        let (bd, _) = duo::<FnSizeT1>("ZSTD_compressBound");

        let cctx = CtxPair::cctx();
        let dict = gen_class(4, 4096, 11);
        let kc = mkc(dict.as_ptr() as *const c_void, dict.len(), 5);
        let kr = mkr(dict.as_ptr() as *const c_void, dict.len(), 5);
        assert!(!kc.is_null() && !kr.is_null());

        let mut saw_dst = false;
        for cls in 0..N_CLASSES {
            for &sz in &[0usize, 1, 3, 6, 7, 20, 128, 1200, 9000, 70_000, 140_000] {
                let src = gen_class(cls, sz, 0xC200 ^ sz as u64);
                let sp = if sz == 0 { ptr::null() } else { src.as_ptr() as *const c_void };
                for &lvl in &[1i32, 3, 9, 19] {
                    // reference size with a generous buffer
                    let full = bd(sz) + 64;
                    let mut probe = vec![0u8; full];
                    let exact = cc(
                        probe.as_mut_ptr() as *mut c_void,
                        full,
                        sp,
                        sz,
                        lvl,
                    );
                    assert!(!is_err(exact), "reference compress failed");
                    for cap in caps_for(exact) {
                        let mut oc = vec![0xC7u8; cap.max(1)];
                        let mut or_ = vec![0xC7u8; cap.max(1)];
                        let tag = format!("cls={cls} sz={sz} lvl={lvl} cap={cap} exact={exact}");

                        let a = cc(oc.as_mut_ptr() as *mut c_void, cap, sp, sz, lvl);
                        let b = cr(or_.as_mut_ptr() as *mut c_void, cap, sp, sz, lvl);
                        eqcode(&format!("ZSTD_compress {tag}"), a, b);
                        eqbuf(&format!("ZSTD_compress {tag} dst"), &oc, &or_);
                        if is_err(a) {
                            saw_dst = true;
                        }

                        let mut oc = vec![0xC7u8; cap.max(1)];
                        let mut or_ = vec![0xC7u8; cap.max(1)];
                        let a = ccc(cctx.c, oc.as_mut_ptr() as *mut c_void, cap, sp, sz, lvl);
                        let b = ccr(cctx.r, or_.as_mut_ptr() as *mut c_void, cap, sp, sz, lvl);
                        eqcode(&format!("ZSTD_compressCCtx {tag}"), a, b);
                        eqbuf(&format!("ZSTD_compressCCtx {tag} dst"), &oc, &or_);

                        let mut oc = vec![0xC7u8; cap.max(1)];
                        let mut or_ = vec![0xC7u8; cap.max(1)];
                        eqcode(
                            &format!("reset {tag}"),
                            rsc(cctx.c, ZSTD_reset_session_and_parameters),
                            rsr(cctx.r, ZSTD_reset_session_and_parameters),
                        );
                        eqcode(
                            &format!("setLevel {tag}"),
                            spc(cctx.c, ZSTD_c_compressionLevel, lvl),
                            spr(cctx.r, ZSTD_c_compressionLevel, lvl),
                        );
                        let a = c2c(cctx.c, oc.as_mut_ptr() as *mut c_void, cap, sp, sz);
                        let b = c2r(cctx.r, or_.as_mut_ptr() as *mut c_void, cap, sp, sz);
                        eqcode(&format!("ZSTD_compress2 {tag}"), a, b);
                        eqbuf(&format!("ZSTD_compress2 {tag} dst"), &oc, &or_);

                        let mut oc = vec![0xC7u8; cap.max(1)];
                        let mut or_ = vec![0xC7u8; cap.max(1)];
                        let a = udc(
                            cctx.c,
                            oc.as_mut_ptr() as *mut c_void,
                            cap,
                            sp,
                            sz,
                            dict.as_ptr() as *const c_void,
                            dict.len(),
                            lvl,
                        );
                        let b = udr(
                            cctx.r,
                            or_.as_mut_ptr() as *mut c_void,
                            cap,
                            sp,
                            sz,
                            dict.as_ptr() as *const c_void,
                            dict.len(),
                            lvl,
                        );
                        eqcode(&format!("ZSTD_compress_usingDict {tag}"), a, b);
                        eqbuf(&format!("ZSTD_compress_usingDict {tag} dst"), &oc, &or_);

                        let mut oc = vec![0xC7u8; cap.max(1)];
                        let mut or_ = vec![0xC7u8; cap.max(1)];
                        let a = cdc(cctx.c, oc.as_mut_ptr() as *mut c_void, cap, sp, sz, kc);
                        let b = cdr(cctx.r, or_.as_mut_ptr() as *mut c_void, cap, sp, sz, kr);
                        eqcode(&format!("ZSTD_compress_usingCDict {tag}"), a, b);
                        eqbuf(&format!("ZSTD_compress_usingCDict {tag} dst"), &oc, &or_);

                        for fp in [
                            ZSTD_frameParameters { contentSizeFlag: 1, checksumFlag: 1, noDictIDFlag: 0 },
                            ZSTD_frameParameters { contentSizeFlag: 0, checksumFlag: 0, noDictIDFlag: 1 },
                        ] {
                            let mut oc = vec![0xC7u8; cap.max(1)];
                            let mut or_ = vec![0xC7u8; cap.max(1)];
                            let a = cac(cctx.c, oc.as_mut_ptr() as *mut c_void, cap, sp, sz, kc, fp);
                            let b = car(cctx.r, or_.as_mut_ptr() as *mut c_void, cap, sp, sz, kr, fp);
                            eqcode(&format!("ZSTD_compress_usingCDict_advanced {tag} {fp:?}"), a, b);
                            eqbuf(
                                &format!("ZSTD_compress_usingCDict_advanced {tag} {fp:?} dst"),
                                &oc,
                                &or_,
                            );
                        }
                    }
                    // dst == NULL. ZSTD_writeFrameHeader bails out with
                    // dstSize_tooSmall for any dstCapacity < ZSTD_FRAMEHEADERSIZE_MAX
                    // (18), so nothing is ever written through the NULL pointer.
                    for cap in [0usize, 1, 2, 3, 8, 17] {
                        let a = cc(ptr::null_mut(), cap, sp, sz, lvl);
                        let b = cr(ptr::null_mut(), cap, sp, sz, lvl);
                        eqcode(&format!("ZSTD_compress(dst=NULL,cap={cap}) cls={cls} sz={sz}"), a, b);
                        assert!(
                            is_err(a),
                            "ZSTD_compress(dst=NULL,cap={cap}) unexpectedly succeeded"
                        );
                        let a = ccc(cctx.c, ptr::null_mut(), cap, sp, sz, lvl);
                        let b = ccr(cctx.r, ptr::null_mut(), cap, sp, sz, lvl);
                        eqcode(
                            &format!("ZSTD_compressCCtx(dst=NULL,cap={cap}) cls={cls} sz={sz}"),
                            a,
                            b,
                        );
                        eqcode(
                            &format!("reset null {cls} {sz}"),
                            rsc(cctx.c, ZSTD_reset_session_and_parameters),
                            rsr(cctx.r, ZSTD_reset_session_and_parameters),
                        );
                        eqcode(
                            &format!("setLevel null {cls} {sz}"),
                            spc(cctx.c, ZSTD_c_compressionLevel, lvl),
                            spr(cctx.r, ZSTD_c_compressionLevel, lvl),
                        );
                        let a = c2c(cctx.c, ptr::null_mut(), cap, sp, sz);
                        let b = c2r(cctx.r, ptr::null_mut(), cap, sp, sz);
                        eqcode(
                            &format!("ZSTD_compress2(dst=NULL,cap={cap}) cls={cls} sz={sz}"),
                            a,
                            b,
                        );
                    }
                }
            }
        }
        assert!(saw_dst, "no dstSize_tooSmall produced by the one-shot grid");
        fdc(kc);
        fdr(kr);
    }
}

// ------------------------------------------------------------------ rows 187, 188, 192
//
// `ZSTD_compressStream2` argument validation plus the stableOutBuffer
// dstSize_tooSmall shortcut, and the same through
// `ZSTD_compressStream2_simpleArgs`.

#[test]
fn err_streaming_dst_and_src_bounds() {
    unsafe {
        let (s2c, s2r) = duo::<FnStream2>("ZSTD_compressStream2");
        let (sac, sar) = duo::<FnSimpleArgs>("ZSTD_compressStream2_simpleArgs");
        let (spc, spr) = duo::<FnSetParam>("ZSTD_CCtx_setParameter");
        let (rsc, rsr) = duo::<FnReset>("ZSTD_CCtx_reset");
        let (bd, _) = duo::<FnSizeT1>("ZSTD_compressBound");
        let cctx = CtxPair::cctx();

        let src = gen_class(4, 5000, 0xC300);

        // ---- output->pos > output->size  => dstSize_tooSmall (row 187)
        // ---- input->pos  > input->size   => srcSize_wrong    (row 188)
        for (opos, osize, ipos, isize_, want) in [
            (1usize, 0usize, 0usize, src.len(), "Destination buffer is too small"),
            (5, 4, 0, src.len(), "Destination buffer is too small"),
            (usize::MAX, 8, 0, src.len(), "Destination buffer is too small"),
            (0, 4096, 1, 0, "Src size is incorrect"),
            (0, 4096, src.len() + 1, src.len(), "Src size is incorrect"),
            (0, 4096, usize::MAX, src.len(), "Src size is incorrect"),
            // both wrong: dstSize_tooSmall is checked first
            (9, 8, 9, 8, "Destination buffer is too small"),
        ] {
            for endop in [ZSTD_e_continue, ZSTD_e_flush, ZSTD_e_end] {
                let mut oc = vec![0x2Bu8; osize.max(1) + 8];
                let mut or_ = vec![0x2Bu8; osize.max(1) + 8];
                let mut obc = ZSTD_outBuffer { dst: oc.as_mut_ptr() as *mut c_void, size: osize, pos: opos };
                let mut obr = ZSTD_outBuffer { dst: or_.as_mut_ptr() as *mut c_void, size: osize, pos: opos };
                let mut ibc = ZSTD_inBuffer { src: src.as_ptr() as *const c_void, size: isize_, pos: ipos };
                let mut ibr = ibc;
                let a = s2c(cctx.c, &mut obc, &mut ibc, endop);
                let b = s2r(cctx.r, &mut obr, &mut ibr, endop);
                let tag = format!("compressStream2(opos={opos},osize={osize},ipos={ipos},isize={isize_},op={endop})");
                eqcode(&tag, a, b);
                expect_code(&tag, a, want);
                eqv(&format!("{tag} in.pos"), ibc.pos, ibr.pos);
                eqv(&format!("{tag} out.pos"), obc.pos, obr.pos);
                eqbuf(&format!("{tag} dst"), &oc, &or_);
            }
        }

        // ---- same via simpleArgs
        for (dpos, dcap, spos, ssz) in [
            (1usize, 0usize, 0usize, src.len()),
            (0, 4096, 1, 0),
            (7, 4, 7, 4),
        ] {
            for endop in [ZSTD_e_continue, ZSTD_e_end] {
                let mut oc = vec![0x2Cu8; dcap.max(1) + 8];
                let mut or_ = vec![0x2Cu8; dcap.max(1) + 8];
                let mut dp_c = dpos;
                let mut dp_r = dpos;
                let mut sp_c = spos;
                let mut sp_r = spos;
                let a = sac(
                    cctx.c,
                    oc.as_mut_ptr() as *mut c_void,
                    dcap,
                    &mut dp_c,
                    src.as_ptr() as *const c_void,
                    ssz,
                    &mut sp_c,
                    endop,
                );
                let b = sar(
                    cctx.r,
                    or_.as_mut_ptr() as *mut c_void,
                    dcap,
                    &mut dp_r,
                    src.as_ptr() as *const c_void,
                    ssz,
                    &mut sp_r,
                    endop,
                );
                let tag = format!("simpleArgs(dpos={dpos},dcap={dcap},spos={spos},ssz={ssz},op={endop})");
                eqcode(&tag, a, b);
                eqv(&format!("{tag} dstPos"), dp_c, dp_r);
                eqv(&format!("{tag} srcPos"), sp_c, sp_r);
                eqbuf(&format!("{tag} dst"), &oc, &or_);
            }
        }

        // ---- stableOutBuffer = 1 with an output smaller than the frame.
        // ZSTD_compressStream2 takes the ZSTD_compressEnd_public shortcut and
        // must report dstSize_tooSmall instead of buffering (row 192's sibling).
        for cls in 0..N_CLASSES {
            for &sz in &[0usize, 1, 7, 100, 5000, 70_000] {
                let s = gen_class(cls, sz, 0xC301 ^ sz as u64);
                let sp = if sz == 0 { ptr::null() } else { s.as_ptr() as *const c_void };
                let full = bd(sz) + 64;
                let mut probe = vec![0u8; full];
                let mut ppos = 0usize;
                let mut spos = 0usize;
                eqcode("reset probe", rsc(cctx.c, ZSTD_reset_session_and_parameters), rsr(cctx.r, ZSTD_reset_session_and_parameters));
                let _ = sac(
                    cctx.c,
                    probe.as_mut_ptr() as *mut c_void,
                    full,
                    &mut ppos,
                    sp,
                    sz,
                    &mut spos,
                    ZSTD_e_end,
                );
                let exact = ppos;
                for cap in caps_for(exact) {
                    for stable_in in [0, 1] {
                        // fresh context: see the `stableIn_notConsumed` note in
                        // err_stability_condition_not_respected
                        let cctx = CtxPair::cctx();
                        let mut oc = vec![0x3Du8; cap.max(1)];
                        let mut or_ = vec![0x3Du8; cap.max(1)];
                        eqcode(
                            "reset stableOut",
                            rsc(cctx.c, ZSTD_reset_session_and_parameters),
                            rsr(cctx.r, ZSTD_reset_session_and_parameters),
                        );
                        for (p, v) in [
                            (ZSTD_c_stableOutBuffer, 1),
                            (ZSTD_c_stableInBuffer, stable_in),
                        ] {
                            eqcode(
                                &format!("set({p},{v})"),
                                spc(cctx.c, p, v),
                                spr(cctx.r, p, v),
                            );
                        }
                        let mut obc = ZSTD_outBuffer { dst: oc.as_mut_ptr() as *mut c_void, size: cap, pos: 0 };
                        let mut obr = ZSTD_outBuffer { dst: or_.as_mut_ptr() as *mut c_void, size: cap, pos: 0 };
                        let mut ibc = ZSTD_inBuffer { src: sp, size: sz, pos: 0 };
                        let mut ibr = ibc;
                        let a = s2c(cctx.c, &mut obc, &mut ibc, ZSTD_e_end);
                        let b = s2r(cctx.r, &mut obr, &mut ibr, ZSTD_e_end);
                        let tag = format!(
                            "stableOut cls={cls} sz={sz} cap={cap} exact={exact} stableIn={stable_in}"
                        );
                        eqcode(&tag, a, b);
                        eqv(&format!("{tag} in.pos"), ibc.pos, ibr.pos);
                        eqv(&format!("{tag} out.pos"), obc.pos, obr.pos);
                        eqbuf(&format!("{tag} dst"), &oc, &or_);
                    }
                }
            }
        }
    }
}

// ------------------------------------------------------------------ rows 95, 107, 108, 109, 111,
// 114, 115, 116, 117, 126, 149, 151, 167, 182
//
// Everything that must be rejected because the context is in the wrong stage.

#[test]
fn err_stage_wrong_and_init_missing() {
    unsafe {
        let (ctc, ctr) = duo::<FnBufferless>("ZSTD_compressContinue");
        let (cec, cer) = duo::<FnBufferless>("ZSTD_compressEnd");
        let (cbc, cbr) = duo::<FnBufferless>("ZSTD_compressBlock");
        let (cpc, cpr) = duo::<FnBufferless>("ZSTD_compressContinue_public");
        let (epc, epr) = duo::<FnBufferless>("ZSTD_compressEnd_public");
        let (bgc, bgr) = duo::<FnCompressBegin>("ZSTD_compressBegin");
        let (s2c, s2r) = duo::<FnStream2>("ZSTD_compressStream2");
        let (csc, csr) = duo::<FnStream1>("ZSTD_compressStream");
        let (flc, flr) = duo::<FnFlush>("ZSTD_flushStream");
        let (enc, enr) = duo::<FnFlush>("ZSTD_endStream");
        let (icc, icr) = duo::<FnInitCStream>("ZSTD_initCStream");
        let (spc, spr) = duo::<FnSetParam>("ZSTD_CCtx_setParameter");
        let (plc, plr) = duo::<FnPledged>("ZSTD_CCtx_setPledgedSrcSize");
        let (rsc, rsr) = duo::<FnReset>("ZSTD_CCtx_reset");
        let (ldc, ldr) = duo::<FnLoadDict>("ZSTD_CCtx_loadDictionary");
        let (rcc, rcr) = duo::<unsafe extern "C" fn(*mut c_void, *const c_void) -> usize>("ZSTD_CCtx_refCDict");
        let (rpc, rpr) = duo::<FnLoadDict>("ZSTD_CCtx_refPrefix");
        let (rtc, rtr) = duo::<unsafe extern "C" fn(*mut c_void, *mut c_void) -> usize>("ZSTD_CCtx_refThreadPool");
        let (upc, upr) = duo::<unsafe extern "C" fn(*mut c_void, *const c_void) -> usize>(
            "ZSTD_CCtx_setParametersUsingCCtxParams",
        );
        let (ccc, ccr) = duo::<FnCopyCCtx>("ZSTD_copyCCtx");
        let (mkc, mkr) = duo::<unsafe extern "C" fn(*const c_void, usize, c_int) -> *mut c_void>("ZSTD_createCDict");
        let (fdc, fdr) = duo::<FnFreePtr>("ZSTD_freeCDict");
        let (gbc, gbr) = duo::<FnGetBlockSize>("ZSTD_getBlockSize");

        let dict = gen_class(4, 2048, 21);
        let kc = mkc(dict.as_ptr() as *const c_void, dict.len(), 3);
        let kr = mkr(dict.as_ptr() as *const c_void, dict.len(), 3);
        let po = CtxPair::cctx_params();
        let src = gen_class(4, 40_000, 22);

        // ---- bufferless entry points on a brand-new (ZSTDcs_created) context
        {
            let cctx = CtxPair::cctx();
            for &sz in &[0usize, 1, 100, 5000] {
                let sp = if sz == 0 { ptr::null() } else { src.as_ptr() as *const c_void };
                for (name, fc, fr) in [
                    ("compressContinue", ctc, ctr),
                    ("compressContinue_public", cpc, cpr),
                    ("compressEnd", cec, cer),
                    ("compressEnd_public", epc, epr),
                    ("compressBlock", cbc, cbr),
                ] {
                    let mut oc = vec![0x4Eu8; 1 << 17];
                    let mut or_ = vec![0x4Eu8; 1 << 17];
                    let a = fc(cctx.c, oc.as_mut_ptr() as *mut c_void, oc.len(), sp, sz);
                    let b = fr(cctx.r, or_.as_mut_ptr() as *mut c_void, or_.len(), sp, sz);
                    let tag = format!("{name} on fresh cctx (sz={sz})");
                    eqcode(&tag, a, b);
                    assert!(is_err(a), "{tag} unexpectedly succeeded (C={a:#x})");
                    eqbuf(&format!("{tag} dst"), &oc, &or_);
                }
                // ZSTD_getBlockSize on a never-initialised context
                eqv(&format!("getBlockSize fresh sz={sz}"), gbc(cctx.c), gbr(cctx.r));
            }
        }

        // ---- ZSTD_copyCCtx from contexts in every stage (row 126)
        {
            for stage in 0..4 {
                let sc = CtxPair::cctx();
                let dstc = CtxPair::cctx();
                let mut oc = vec![0u8; 1 << 17];
                let mut or_ = vec![0u8; 1 << 17];
                match stage {
                    0 => {} // ZSTDcs_created
                    1 => {
                        eqcode("copy begin", bgc(sc.c, 3), bgr(sc.r, 3));
                    }
                    2 => {
                        eqcode("copy begin", bgc(sc.c, 3), bgr(sc.r, 3));
                        let a = ctc(
                            sc.c,
                            oc.as_mut_ptr() as *mut c_void,
                            oc.len(),
                            src.as_ptr() as *const c_void,
                            1000,
                        );
                        let b = ctr(
                            sc.r,
                            or_.as_mut_ptr() as *mut c_void,
                            or_.len(),
                            src.as_ptr() as *const c_void,
                            1000,
                        );
                        eqcode("copy continue", a, b);
                    }
                    _ => {
                        eqcode("copy begin", bgc(sc.c, 3), bgr(sc.r, 3));
                        let a = cec(
                            sc.c,
                            oc.as_mut_ptr() as *mut c_void,
                            oc.len(),
                            src.as_ptr() as *const c_void,
                            1000,
                        );
                        let b = cer(
                            sc.r,
                            or_.as_mut_ptr() as *mut c_void,
                            or_.len(),
                            src.as_ptr() as *const c_void,
                            1000,
                        );
                        eqcode("copy end", a, b);
                    }
                }
                for pledged in [0u64, 1000, ZSTD_CONTENTSIZE_UNKNOWN] {
                    let a = ccc(dstc.c, sc.c, pledged);
                    let b = ccr(dstc.r, sc.r, pledged);
                    let tag = format!("copyCCtx(stage={stage},pledged={pledged})");
                    eqcode(&tag, a, b);
                    if stage == 2 {
                        expect_code(&tag, a, "Operation not authorized at current processing stage");
                    }
                }
            }
        }

        // ---- setters that require zcss_init, called mid-stream
        {
            let cctx = CtxPair::cctx();
            let mut oc = vec![0u8; 4096];
            let mut or_ = vec![0u8; 4096];
            let mut obc = ZSTD_outBuffer { dst: oc.as_mut_ptr() as *mut c_void, size: 64, pos: 0 };
            let mut obr = ZSTD_outBuffer { dst: or_.as_mut_ptr() as *mut c_void, size: 64, pos: 0 };
            let mut ibc = ZSTD_inBuffer { src: src.as_ptr() as *const c_void, size: src.len(), pos: 0 };
            let mut ibr = ibc;
            let a = s2c(cctx.c, &mut obc, &mut ibc, ZSTD_e_continue);
            let b = s2r(cctx.r, &mut obr, &mut ibr, ZSTD_e_continue);
            eqcode("mid-stream priming", a, b);
            assert!(!is_err(a), "priming call failed: {}", errname(a));
            eqv("mid-stream priming in.pos", ibc.pos, ibr.pos);
            eqv("mid-stream priming out.pos", obc.pos, obr.pos);

            // ZSTD_isUpdateAuthorized() lets exactly these through mid-stream;
            // every *other* parameter must be rejected with stage_wrong.
            const AUTHORIZED: [c_int; 8] = [
                ZSTD_c_compressionLevel,
                ZSTD_c_hashLog,
                ZSTD_c_chainLog,
                ZSTD_c_searchLog,
                ZSTD_c_minMatch,
                ZSTD_c_targetLength,
                ZSTD_c_strategy,
                ZSTD_c_blockSplitterLevel,
            ];
            let mut n_stage_wrong = 0usize;
            for (name, p) in ALL_CPARAMS {
                for v in [0, 1] {
                    let a = spc(cctx.c, *p, v);
                    let b = spr(cctx.r, *p, v);
                    let tag = format!("CCtx_setParameter({name}={v}) mid-stream");
                    eqcode(&tag, a, b);
                    if !AUTHORIZED.contains(p) {
                        expect_code(&tag, a, "Operation not authorized at current processing stage");
                        n_stage_wrong += 1;
                    }
                }
            }
            assert!(n_stage_wrong > 30, "row 95 barely exercised");
            for pledged in [0u64, 12345, ZSTD_CONTENTSIZE_UNKNOWN] {
                let a = plc(cctx.c, pledged);
                let b = plr(cctx.r, pledged);
                let tag = format!("setPledgedSrcSize({pledged}) mid-stream");
                eqcode(&tag, a, b);
                expect_code(&tag, a, "Operation not authorized at current processing stage");
            }
            let a = ldc(cctx.c, dict.as_ptr() as *const c_void, dict.len());
            let b = ldr(cctx.r, dict.as_ptr() as *const c_void, dict.len());
            eqcode("loadDictionary mid-stream", a, b);
            expect_code(
                "loadDictionary mid-stream",
                a,
                "Operation not authorized at current processing stage",
            );
            let a = rcc(cctx.c, kc);
            let b = rcr(cctx.r, kr);
            eqcode("refCDict mid-stream", a, b);
            expect_code(
                "refCDict mid-stream",
                a,
                "Operation not authorized at current processing stage",
            );
            let a = rpc(cctx.c, dict.as_ptr() as *const c_void, dict.len());
            let b = rpr(cctx.r, dict.as_ptr() as *const c_void, dict.len());
            eqcode("refPrefix mid-stream", a, b);
            expect_code(
                "refPrefix mid-stream",
                a,
                "Operation not authorized at current processing stage",
            );
            let a = rtc(cctx.c, ptr::null_mut());
            let b = rtr(cctx.r, ptr::null_mut());
            eqcode("refThreadPool mid-stream", a, b);
            expect_code(
                "refThreadPool mid-stream",
                a,
                "Operation not authorized at current processing stage",
            );
            let a = upc(cctx.c, po.c);
            let b = upr(cctx.r, po.r);
            eqcode("setParametersUsingCCtxParams mid-stream", a, b);
            expect_code(
                "setParametersUsingCCtxParams mid-stream",
                a,
                "Operation not authorized at current processing stage",
            );
            // ZSTD_CCtx_reset(ZSTD_reset_parameters) mid-stream is also stage_wrong;
            // reset_session_only / session_and_parameters must succeed.
            let a = rsc(cctx.c, ZSTD_reset_parameters);
            let b = rsr(cctx.r, ZSTD_reset_parameters);
            eqcode("reset_parameters mid-stream", a, b);
            expect_code(
                "reset_parameters mid-stream",
                a,
                "Operation not authorized at current processing stage",
            );
            eqcode(
                "reset_session_only mid-stream",
                rsc(cctx.c, ZSTD_reset_session_only),
                rsr(cctx.r, ZSTD_reset_session_only),
            );
        }

        // ---- setParametersUsingCCtxParams while a CDict is referenced (row 108)
        {
            let cctx = CtxPair::cctx();
            eqcode("refCDict", rcc(cctx.c, kc), rcr(cctx.r, kr));
            let a = upc(cctx.c, po.c);
            let b = upr(cctx.r, po.r);
            eqcode("setParametersUsingCCtxParams with cdict", a, b);
            expect_code(
                "setParametersUsingCCtxParams with cdict",
                a,
                "Operation not authorized at current processing stage",
            );
        }

        // ---- legacy streaming API out of order
        {
            let cs = CtxPair::cstream();
            let mut oc = vec![0x5Bu8; 1 << 17];
            let mut or_ = vec![0x5Bu8; 1 << 17];

            // flushStream / endStream / compressStream *before* initCStream.
            // The transparent-init path of ZSTD_compressStream2 means these do
            // not error out; both libraries must agree on whatever they do.
            for round in 0..2 {
                let mut obc = ZSTD_outBuffer { dst: oc.as_mut_ptr() as *mut c_void, size: oc.len(), pos: 0 };
                let mut obr = ZSTD_outBuffer { dst: or_.as_mut_ptr() as *mut c_void, size: or_.len(), pos: 0 };
                let a = flc(cs.c, &mut obc);
                let b = flr(cs.r, &mut obr);
                eqcode(&format!("flushStream before init round={round}"), a, b);
                eqv(&format!("flushStream before init out.pos round={round}"), obc.pos, obr.pos);
                let a = enc(cs.c, &mut obc);
                let b = enr(cs.r, &mut obr);
                eqcode(&format!("endStream before init round={round}"), a, b);
                eqv(&format!("endStream before init out.pos round={round}"), obc.pos, obr.pos);
                eqbuf(&format!("before-init dst round={round}"), &oc, &or_);
            }

            // after ZSTD_endStream completed the frame the session is reset;
            // further flush/end calls must behave identically
            eqcode("initCStream", icc(cs.c, 3), icr(cs.r, 3));
            let mut obc = ZSTD_outBuffer { dst: oc.as_mut_ptr() as *mut c_void, size: oc.len(), pos: 0 };
            let mut obr = ZSTD_outBuffer { dst: or_.as_mut_ptr() as *mut c_void, size: or_.len(), pos: 0 };
            let mut ibc = ZSTD_inBuffer { src: src.as_ptr() as *const c_void, size: 3000, pos: 0 };
            let mut ibr = ibc;
            eqcode("compressStream", csc(cs.c, &mut obc, &mut ibc), csr(cs.r, &mut obr, &mut ibr));
            eqv("compressStream in.pos", ibc.pos, ibr.pos);
            for k in 0..4 {
                let a = enc(cs.c, &mut obc);
                let b = enr(cs.r, &mut obr);
                eqcode(&format!("endStream #{k}"), a, b);
                eqv(&format!("endStream #{k} out.pos"), obc.pos, obr.pos);
                let a = flc(cs.c, &mut obc);
                let b = flr(cs.r, &mut obr);
                eqcode(&format!("flushStream after end #{k}"), a, b);
                eqv(&format!("flushStream after end #{k} out.pos"), obc.pos, obr.pos);
                // and compressStream after the frame ended
                let mut ic2 = ZSTD_inBuffer { src: src.as_ptr() as *const c_void, size: 500, pos: 0 };
                let mut ir2 = ic2;
                let a = csc(cs.c, &mut obc, &mut ic2);
                let b = csr(cs.r, &mut obr, &mut ir2);
                eqcode(&format!("compressStream after end #{k}"), a, b);
                eqv(&format!("compressStream after end #{k} in.pos"), ic2.pos, ir2.pos);
            }
            eqbuf("post-end dst", &oc, &or_);
        }

        // ---- compressStream2 after an error: the context must stay usable in
        // exactly the same way in both libraries
        {
            let cctx = CtxPair::cctx();
            let mut oc = vec![0x6Cu8; 1 << 16];
            let mut or_ = vec![0x6Cu8; 1 << 16];
            // provoke srcSize_wrong via input->pos > input->size
            let mut obc = ZSTD_outBuffer { dst: oc.as_mut_ptr() as *mut c_void, size: oc.len(), pos: 0 };
            let mut obr = ZSTD_outBuffer { dst: or_.as_mut_ptr() as *mut c_void, size: or_.len(), pos: 0 };
            let mut ibc = ZSTD_inBuffer { src: src.as_ptr() as *const c_void, size: 10, pos: 99 };
            let mut ibr = ibc;
            let a = s2c(cctx.c, &mut obc, &mut ibc, ZSTD_e_end);
            let b = s2r(cctx.r, &mut obr, &mut ibr, ZSTD_e_end);
            eqcode("provoke srcSize_wrong", a, b);
            for k in 0..3 {
                let mut ibc = ZSTD_inBuffer { src: src.as_ptr() as *const c_void, size: 4000, pos: 0 };
                let mut ibr = ibc;
                let a = s2c(cctx.c, &mut obc, &mut ibc, ZSTD_e_end);
                let b = s2r(cctx.r, &mut obr, &mut ibr, ZSTD_e_end);
                eqcode(&format!("compressStream2 after error #{k}"), a, b);
                eqv(&format!("after error #{k} in.pos"), ibc.pos, ibr.pos);
                eqv(&format!("after error #{k} out.pos"), obc.pos, obr.pos);
            }
            eqbuf("after-error dst", &oc, &or_);
        }

        fdc(kc);
        fdr(kr);
    }
}

// ------------------------------------------------------------------ rows 150, 170
//
// pledgedSrcSize disagreeing with the data actually supplied.

#[test]
fn err_pledged_srcsize_wrong() {
    unsafe {
        let (bac, bar) = duo::<FnCompressBeginAdvanced>("ZSTD_compressBegin_advanced");
        let (ctc, ctr) = duo::<FnBufferless>("ZSTD_compressContinue");
        let (cec, cer) = duo::<FnBufferless>("ZSTD_compressEnd");
        let (s2c, s2r) = duo::<FnStream2>("ZSTD_compressStream2");
        let (plc, plr) = duo::<FnPledged>("ZSTD_CCtx_setPledgedSrcSize");
        let (rsc, rsr) = duo::<FnReset>("ZSTD_CCtx_reset");
        let (spc, spr) = duo::<FnSetParam>("ZSTD_CCtx_setParameter");
        let (c2c, c2r) = duo::<FnCompress2>("ZSTD_compress2");
        let (gp, _) = duo::<FnGetParams>("ZSTD_getParams");
        let (bd, _) = duo::<FnSizeT1>("ZSTD_compressBound");

        let src = gen_class(4, 30_000, 31);
        let mut saw_over = false;
        let mut saw_under = false;

        // ---- bufferless: pledge N, then feed more than N  (row 150)
        for &pledged in &[0u64, 1, 100, 1000, 4096, 29_999, 30_000] {
            for &chunk in &[1usize, 100, 1000, 5000, 30_000] {
                let cctx = CtxPair::cctx();
                let params = gp(3, pledged, 0);
                eqcode(
                    &format!("beginAdvanced(pledged={pledged})"),
                    bac(cctx.c, ptr::null(), 0, params, pledged),
                    bar(cctx.r, ptr::null(), 0, params, pledged),
                );
                // Every ZSTD_compressContinue() call emits at least one block
                // header, so with chunk == 1 the output is ~4x the input.
                let room = src.len() * 8 + 65536;
                let mut oc = vec![0x7Au8; room];
                let mut or_ = vec![0x7Au8; room];
                let mut off = 0usize;
                let mut poc = 0usize;
                let mut por = 0usize;
                let mut stop = false;
                while off < src.len() && !stop {
                    let n = chunk.min(src.len() - off);
                    let a = ctc(
                        cctx.c,
                        oc.as_mut_ptr().add(poc) as *mut c_void,
                        oc.len() - poc,
                        src.as_ptr().add(off) as *const c_void,
                        n,
                    );
                    let b = ctr(
                        cctx.r,
                        or_.as_mut_ptr().add(por) as *mut c_void,
                        or_.len() - por,
                        src.as_ptr().add(off) as *const c_void,
                        n,
                    );
                    let tag = format!("compressContinue(pledged={pledged},chunk={chunk},off={off})");
                    eqcode(&tag, a, b);
                    if is_err(a) {
                        expect_code(&tag, a, "Src size is incorrect");
                        saw_over = true;
                        stop = true;
                        break;
                    }
                    poc += a;
                    por += b;
                    off += n;
                }
                if stop {
                    continue;
                }
                // ---- compressEnd having supplied *less* than pledged (row 170)
                let a = cec(
                    cctx.c,
                    oc.as_mut_ptr().add(poc) as *mut c_void,
                    oc.len() - poc,
                    ptr::null(),
                    0,
                );
                let b = cer(
                    cctx.r,
                    or_.as_mut_ptr().add(por) as *mut c_void,
                    or_.len() - por,
                    ptr::null(),
                    0,
                );
                let tag = format!("compressEnd(pledged={pledged},chunk={chunk})");
                eqcode(&tag, a, b);
                if is_err(a) && errname(a) == "Src size is incorrect" {
                    saw_under = true;
                }
                eqbuf(&format!("{tag} dst"), &oc[..poc.max(por)], &or_[..poc.max(por)]);
            }
        }
        assert!(saw_over, "row 150 (compressContinue srcSize_wrong) never triggered");

        // ---- bufferless: pledge N, supply strictly LESS than N, then
        // compressEnd  (row 170: ZSTD_compressEnd_public srcSize_wrong)
        for &pledged in &[1u64, 100, 1000, 30_000] {
            for &fed in &[0usize, 1, 50, 999] {
                if fed as u64 >= pledged {
                    continue;
                }
                let cctx = CtxPair::cctx();
                let params = gp(3, pledged, 0);
                eqcode(
                    &format!("beginAdvanced(under pledged={pledged})"),
                    bac(cctx.c, ptr::null(), 0, params, pledged),
                    bar(cctx.r, ptr::null(), 0, params, pledged),
                );
                let room = 65536usize;
                let mut oc = vec![0xD4u8; room];
                let mut or_ = vec![0xD4u8; room];
                let mut poc = 0usize;
                let mut por = 0usize;
                if fed > 0 {
                    let a = ctc(
                        cctx.c,
                        oc.as_mut_ptr() as *mut c_void,
                        room,
                        src.as_ptr() as *const c_void,
                        fed,
                    );
                    let b = ctr(
                        cctx.r,
                        or_.as_mut_ptr() as *mut c_void,
                        room,
                        src.as_ptr() as *const c_void,
                        fed,
                    );
                    eqcode(&format!("continue(under pledged={pledged},fed={fed})"), a, b);
                    assert!(!is_err(a));
                    poc = a;
                    por = b;
                }
                let a = cec(
                    cctx.c,
                    oc.as_mut_ptr().add(poc) as *mut c_void,
                    room - poc,
                    ptr::null(),
                    0,
                );
                let b = cer(
                    cctx.r,
                    or_.as_mut_ptr().add(por) as *mut c_void,
                    room - por,
                    ptr::null(),
                    0,
                );
                let tag = format!("compressEnd(under pledged={pledged},fed={fed})");
                eqcode(&tag, a, b);
                expect_code(&tag, a, "Src size is incorrect");
                saw_under = true;
                eqbuf(&format!("{tag} dst"), &oc, &or_);
            }
        }
        assert!(saw_under, "row 170 (compressEnd srcSize_wrong) never triggered");

        // ---- streaming: pledge N then supply != N through compressStream2
        let cctx = CtxPair::cctx();
        for &pledged in &[0u64, 1, 1000, 29_999, 30_000, 30_001, 1 << 40, ZSTD_CONTENTSIZE_UNKNOWN] {
            for &supply in &[0usize, 1, 1000, 30_000] {
                for cs in [0, 1] {
                    eqcode(
                        "reset stream pledged",
                        rsc(cctx.c, ZSTD_reset_session_and_parameters),
                        rsr(cctx.r, ZSTD_reset_session_and_parameters),
                    );
                    eqcode(
                        "set checksum",
                        spc(cctx.c, ZSTD_c_checksumFlag, cs),
                        spr(cctx.r, ZSTD_c_checksumFlag, cs),
                    );
                    eqcode(
                        &format!("setPledgedSrcSize({pledged})"),
                        plc(cctx.c, pledged),
                        plr(cctx.r, pledged),
                    );
                    let mut oc = vec![0x8Bu8; bd(30_000) + 4096];
                    let mut or_ = vec![0x8Bu8; bd(30_000) + 4096];
                    let mut obc = ZSTD_outBuffer { dst: oc.as_mut_ptr() as *mut c_void, size: oc.len(), pos: 0 };
                    let mut obr = ZSTD_outBuffer { dst: or_.as_mut_ptr() as *mut c_void, size: or_.len(), pos: 0 };
                    let sp = if supply == 0 { ptr::null() } else { src.as_ptr() as *const c_void };
                    let mut ibc = ZSTD_inBuffer { src: sp, size: supply, pos: 0 };
                    let mut ibr = ibc;
                    // feed in two halves with e_continue, then e_end
                    let half = supply / 2;
                    ibc.size = half;
                    ibr.size = half;
                    let a = s2c(cctx.c, &mut obc, &mut ibc, ZSTD_e_continue);
                    let b = s2r(cctx.r, &mut obr, &mut ibr, ZSTD_e_continue);
                    let tag = format!("stream pledged={pledged} supply={supply} cs={cs}");
                    eqcode(&format!("{tag} continue"), a, b);
                    ibc.size = supply;
                    ibr.size = supply;
                    let a = s2c(cctx.c, &mut obc, &mut ibc, ZSTD_e_end);
                    let b = s2r(cctx.r, &mut obr, &mut ibr, ZSTD_e_end);
                    eqcode(&format!("{tag} end"), a, b);
                    eqv(&format!("{tag} in.pos"), ibc.pos, ibr.pos);
                    eqv(&format!("{tag} out.pos"), obc.pos, obr.pos);
                    eqbuf(&format!("{tag} dst"), &oc, &or_);
                }
            }
        }

        // ---- one-shot ZSTD_compress2 with a mismatching pledge
        for &pledged in &[0u64, 1, 999, 30_000, 30_001, ZSTD_CONTENTSIZE_UNKNOWN] {
            for &sz in &[0usize, 1, 1000, 30_000] {
                eqcode(
                    "reset compress2 pledged",
                    rsc(cctx.c, ZSTD_reset_session_and_parameters),
                    rsr(cctx.r, ZSTD_reset_session_and_parameters),
                );
                eqcode(
                    &format!("pledge {pledged}"),
                    plc(cctx.c, pledged),
                    plr(cctx.r, pledged),
                );
                let cap = bd(sz) + 64;
                let mut oc = vec![0x9Cu8; cap];
                let mut or_ = vec![0x9Cu8; cap];
                let sp = if sz == 0 { ptr::null() } else { src.as_ptr() as *const c_void };
                let a = c2c(cctx.c, oc.as_mut_ptr() as *mut c_void, cap, sp, sz);
                let b = c2r(cctx.r, or_.as_mut_ptr() as *mut c_void, cap, sp, sz);
                let tag = format!("compress2 pledged={pledged} sz={sz}");
                eqcode(&tag, a, b);
                eqbuf(&format!("{tag} dst"), &oc, &or_);
            }
        }
    }
}

// ------------------------------------------------------------------ rows 183, 184, 190, 191
//
// `ZSTD_c_stableInBuffer` / `ZSTD_c_stableOutBuffer` violations.

#[test]
fn err_stability_condition_not_respected() {
    unsafe {
        let (s2c, s2r) = duo::<FnStream2>("ZSTD_compressStream2");
        let (spc, spr) = duo::<FnSetParam>("ZSTD_CCtx_setParameter");
        let (rsc, rsr) = duo::<FnReset>("ZSTD_CCtx_reset");
        let (bd, _) = duo::<FnSizeT1>("ZSTD_compressBound");

        // UPSTREAM C PRECONDITION (avoided, not re-litigated): a `ZSTD_CCtx`
        // must NOT be reused after a `ZSTD_c_stableInBuffer` session that did
        // not consume all of its input. `ZSTD_compressStream_generic()` parks
        // the leftover in `zcs->stableIn_notConsumed` (zstd_compress.c:6185) and
        // `ZSTD_CCtx_reset()` does not clear that field (it only touches
        // `streamStage` / `pledgedSrcSizePlusOne`, zstd_compress.c:1368-1381).
        // The next session then executes `input->pos -= stableIn_notConsumed;
        // ip -= stableIn_notConsumed;` on a fresh `pos == 0`, which underflows
        // and reads *before* the caller's buffer. Because the two libraries call
        // in sequence, the C's own allocations can change that out-of-bounds
        // region between the two calls, so the compressed output diverges purely
        // from heap history. Every stable-buffer session below therefore uses a
        // brand-new CCtx.

        // Two *distinct* buffers with identical content: switching between them
        // changes `input->src` without changing the data.
        let base = gen_class(4, 200_000, 41);
        let copy = base.clone();
        let big = bd(base.len()) + 4096;

        let mut saw_in = 0usize;
        let mut saw_out = 0usize;

        // ---- (a) stableInBuffer with a MOVING src pointer.
        // First call uses a small chunk so the library actually stays mid-frame.
        for &first in &[100usize, 5000, 70_000, 140_000] {
            for &endop1 in &[ZSTD_e_continue, ZSTD_e_flush] {
                for variant in 0..5 {
                    let cctx = CtxPair::cctx();
                    // UPSTREAM C PRECONDITION (avoided, not re-litigated):
                    // when `ZSTD_c_stableInBuffer` is on, `endOp == ZSTD_e_continue`
                    // and the total input is still below ZSTD_BLOCKSIZE_MAX,
                    // `ZSTD_compressStream2` takes the "pretend the input was
                    // consumed" shortcut and remembers the amount in
                    // `cctx->stableIn_notConsumed` *without* initialising the
                    // frame. A following call that SHRINKS `input` then reaches
                    // `ZSTD_compressStream_generic`, which does
                    // `input->pos -= zcs->stableIn_notConsumed;
                    //  ip -= zcs->stableIn_notConsumed;`
                    // (zstd_compress.c:6120-6122). With the smaller `pos` this
                    // underflows and `ip` ends up *before* the caller's buffer,
                    // so the C reads out of bounds (its `assert(input->pos >=
                    // stableIn_notConsumed)` is compiled out — DEBUGLEVEL is
                    // unset). The Rust port transliterates the same arithmetic,
                    // so both read the same out-of-bounds bytes and the result
                    // depends on heap history. Those two shapes are therefore
                    // only exercised from a session whose first call really did
                    // initialise the frame (`stableIn_notConsumed == 0`).
                    let primed = endop1 == ZSTD_e_flush || first >= ZSTD_BLOCKSIZE_MAX;
                    if (variant == 2 || variant == 3) && !primed {
                        continue;
                    }
                    // one fresh session per variant: no state (and no output
                    // position) is carried across violations
                    eqcode(
                        "reset stableIn",
                        rsc(cctx.c, ZSTD_reset_session_and_parameters),
                        rsr(cctx.r, ZSTD_reset_session_and_parameters),
                    );
                    eqcode(
                        "set stableIn",
                        spc(cctx.c, ZSTD_c_stableInBuffer, 1),
                        spr(cctx.r, ZSTD_c_stableInBuffer, 1),
                    );
                    let mut oc = vec![0xA1u8; big];
                    let mut or_ = vec![0xA1u8; big];
                    let mut obc =
                        ZSTD_outBuffer { dst: oc.as_mut_ptr() as *mut c_void, size: big, pos: 0 };
                    let mut obr =
                        ZSTD_outBuffer { dst: or_.as_mut_ptr() as *mut c_void, size: big, pos: 0 };
                    let mut ibc =
                        ZSTD_inBuffer { src: base.as_ptr() as *const c_void, size: first, pos: 0 };
                    let mut ibr = ibc;
                    let a = s2c(cctx.c, &mut obc, &mut ibc, endop1);
                    let b = s2r(cctx.r, &mut obr, &mut ibr, endop1);
                    let tag = format!("stableIn first={first} op1={endop1} v={variant}");
                    eqcode(&format!("{tag} call1"), a, b);
                    eqv(&format!("{tag} call1 in.pos"), ibc.pos, ibr.pos);
                    eqv(&format!("{tag} call1 out.pos"), obc.pos, obr.pos);

                    // now hand it a *different* buffer
                    // -> stabilityCondition_notRespected
                    let (what, newsrc, newsize, newpos) = match variant {
                        0 => ("moved", copy.as_ptr() as *const c_void, first, ibc.pos),
                        1 => ("shifted", base.as_ptr().add(1) as *const c_void, first, ibc.pos),
                        2 => (
                            "shrunk",
                            base.as_ptr() as *const c_void,
                            first / 2,
                            ibc.pos.min(first / 2),
                        ),
                        3 => ("pos-reset", base.as_ptr() as *const c_void, first, 0),
                        _ => ("null", ptr::null(), 0, 0),
                    };
                    let mut i2c = ZSTD_inBuffer { src: newsrc, size: newsize, pos: newpos };
                    let mut i2r = i2c;
                    let a = s2c(cctx.c, &mut obc, &mut i2c, ZSTD_e_end);
                    let b = s2r(cctx.r, &mut obr, &mut i2r, ZSTD_e_end);
                    let t2 = format!("{tag} {what}");
                    eqcode(&t2, a, b);
                    eqv(&format!("{t2} in.pos"), i2c.pos, i2r.pos);
                    eqv(&format!("{t2} out.pos"), obc.pos, obr.pos);
                    eqbuf(&format!("{t2} dst"), &oc, &or_);
                    if is_err(a)
                        && errname(a) == "pledged buffer stability condition is not respected"
                    {
                        saw_in += 1;
                    }
                }
            }
        }
        assert!(saw_in > 0, "rows 183/190/191 (stableInBuffer) never triggered");

        // ---- (b) stableOutBuffer whose remaining size CHANGES between calls
        for &osize1 in &[70_000usize, 100_000, 150_000, 260_000] {
            let cctx = CtxPair::cctx();
            eqcode(
                "reset stableOut",
                rsc(cctx.c, ZSTD_reset_session_and_parameters),
                rsr(cctx.r, ZSTD_reset_session_and_parameters),
            );
            eqcode(
                "set stableOut",
                spc(cctx.c, ZSTD_c_stableOutBuffer, 1),
                spr(cctx.r, ZSTD_c_stableOutBuffer, 1),
            );
            let mut oc = vec![0xB2u8; big];
            let mut or_ = vec![0xB2u8; big];
            let mut obc = ZSTD_outBuffer { dst: oc.as_mut_ptr() as *mut c_void, size: osize1, pos: 0 };
            let mut obr = ZSTD_outBuffer { dst: or_.as_mut_ptr() as *mut c_void, size: osize1, pos: 0 };
            let mut ibc = ZSTD_inBuffer { src: base.as_ptr() as *const c_void, size: 150_000, pos: 0 };
            let mut ibr = ibc;
            let a = s2c(cctx.c, &mut obc, &mut ibc, ZSTD_e_continue);
            let b = s2r(cctx.r, &mut obr, &mut ibr, ZSTD_e_continue);
            let tag = format!("stableOut osize1={osize1}");
            eqcode(&format!("{tag} call1"), a, b);
            eqv(&format!("{tag} call1 out.pos"), obc.pos, obr.pos);
            if is_err(a) {
                continue;
            }
            for (what, dsize, dpos) in [
                ("grown", osize1 + 1024, obc.pos),
                ("shrunk", osize1.saturating_sub(8), obc.pos.min(osize1.saturating_sub(8))),
                ("pos-moved", osize1, obc.pos + 1),
                ("zero", 0usize, 0usize),
            ] {
                let mut o2c = ZSTD_outBuffer { dst: oc.as_mut_ptr() as *mut c_void, size: dsize, pos: dpos };
                let mut o2r = ZSTD_outBuffer { dst: or_.as_mut_ptr() as *mut c_void, size: dsize, pos: dpos };
                let mut i2c = ZSTD_inBuffer { src: base.as_ptr() as *const c_void, size: 150_000, pos: ibc.pos };
                let mut i2r = i2c;
                let a = s2c(cctx.c, &mut o2c, &mut i2c, ZSTD_e_end);
                let b = s2r(cctx.r, &mut o2r, &mut i2r, ZSTD_e_end);
                let t2 = format!("{tag} {what} dsize={dsize} dpos={dpos}");
                eqcode(&t2, a, b);
                eqv(&format!("{t2} in.pos"), i2c.pos, i2r.pos);
                eqv(&format!("{t2} out.pos"), o2c.pos, o2r.pos);
                if is_err(a) && errname(a) == "pledged buffer stability condition is not respected" {
                    saw_out += 1;
                }
            }
            eqbuf(&format!("{tag} dst"), &oc, &or_);
        }
        assert!(saw_out > 0, "row 184 (stableOutBuffer) never triggered");

        // ---- (c) stableOutBuffer smaller than the frame => dstSize_tooSmall
        for cls in 0..N_CLASSES {
            for &sz in &[7usize, 5000, 200_000] {
                let s = gen_class(cls, sz, 0xA300 ^ sz as u64);
                for &osz in &[0usize, 1, 3, 12, 17, 40] {
                    let cctx = CtxPair::cctx();
                    eqcode(
                        "reset stableOut small",
                        rsc(cctx.c, ZSTD_reset_session_and_parameters),
                        rsr(cctx.r, ZSTD_reset_session_and_parameters),
                    );
                    eqcode(
                        "set stableOut small",
                        spc(cctx.c, ZSTD_c_stableOutBuffer, 1),
                        spr(cctx.r, ZSTD_c_stableOutBuffer, 1),
                    );
                    let mut oc = vec![0xC3u8; osz.max(1)];
                    let mut or_ = vec![0xC3u8; osz.max(1)];
                    let mut obc = ZSTD_outBuffer { dst: oc.as_mut_ptr() as *mut c_void, size: osz, pos: 0 };
                    let mut obr = ZSTD_outBuffer { dst: or_.as_mut_ptr() as *mut c_void, size: osz, pos: 0 };
                    let mut ibc = ZSTD_inBuffer { src: s.as_ptr() as *const c_void, size: sz, pos: 0 };
                    let mut ibr = ibc;
                    let a = s2c(cctx.c, &mut obc, &mut ibc, ZSTD_e_end);
                    let b = s2r(cctx.r, &mut obr, &mut ibr, ZSTD_e_end);
                    let tag = format!("stableOut-small cls={cls} sz={sz} osz={osz}");
                    eqcode(&tag, a, b);
                    eqv(&format!("{tag} in.pos"), ibc.pos, ibr.pos);
                    eqv(&format!("{tag} out.pos"), obc.pos, obr.pos);
                    eqbuf(&format!("{tag} dst"), &oc, &or_);
                }
            }
        }
    }
}

// ------------------------------------------------------------------ zero-progress loops
//
// The compressor has NO no-forward-progress bail-out (see the module header):
// both libraries must spin identically forever instead of inventing an error.

#[test]
fn err_zero_progress_loops() {
    unsafe {
        let (s2c, s2r) = duo::<FnStream2>("ZSTD_compressStream2");
        let (flc, flr) = duo::<FnFlush>("ZSTD_flushStream");
        let (enc, enr) = duo::<FnFlush>("ZSTD_endStream");
        let (spc, spr) = duo::<FnSetParam>("ZSTD_CCtx_setParameter");
        let (rsc, rsr) = duo::<FnReset>("ZSTD_CCtx_reset");
        let cctx = CtxPair::cctx();
        let src = gen_class(4, 300_000, 51);

        // ---- (a) out.size == out.pos with data still pending
        for stable_out in [0, 1] {
            eqcode(
                "reset zero-progress out",
                rsc(cctx.c, ZSTD_reset_session_and_parameters),
                rsr(cctx.r, ZSTD_reset_session_and_parameters),
            );
            eqcode(
                "set stableOut",
                spc(cctx.c, ZSTD_c_stableOutBuffer, stable_out),
                spr(cctx.r, ZSTD_c_stableOutBuffer, stable_out),
            );
            let mut oc = vec![0xE1u8; 512];
            let mut or_ = vec![0xE1u8; 512];
            let mut obc = ZSTD_outBuffer { dst: oc.as_mut_ptr() as *mut c_void, size: 512, pos: 0 };
            let mut obr = ZSTD_outBuffer { dst: or_.as_mut_ptr() as *mut c_void, size: 512, pos: 0 };
            let mut ibc = ZSTD_inBuffer { src: src.as_ptr() as *const c_void, size: src.len(), pos: 0 };
            let mut ibr = ibc;
            for k in 0..40 {
                let a = s2c(cctx.c, &mut obc, &mut ibc, ZSTD_e_end);
                let b = s2r(cctx.r, &mut obr, &mut ibr, ZSTD_e_end);
                let tag = format!("zero-progress out (stableOut={stable_out}) iter={k}");
                eqcode(&tag, a, b);
                eqv(&format!("{tag} in.pos"), ibc.pos, ibr.pos);
                eqv(&format!("{tag} out.pos"), obc.pos, obr.pos);
                if is_err(a) {
                    break;
                }
                // deliberately do NOT drain the output: out.pos stays == out.size
                if obc.pos == obc.size && a != 0 {
                    // still pending, but nowhere to put it: keep hammering
                    continue;
                }
                if a == 0 {
                    break;
                }
            }
            eqbuf(&format!("zero-progress out dst stableOut={stable_out}"), &oc, &or_);
        }

        // ---- (b) in.size == in.pos with ZSTD_e_continue, repeated
        eqcode(
            "reset zero-progress in",
            rsc(cctx.c, ZSTD_reset_session_and_parameters),
            rsr(cctx.r, ZSTD_reset_session_and_parameters),
        );
        let mut oc = vec![0xE2u8; 1 << 20];
        let mut or_ = vec![0xE2u8; 1 << 20];
        let mut obc = ZSTD_outBuffer { dst: oc.as_mut_ptr() as *mut c_void, size: oc.len(), pos: 0 };
        let mut obr = ZSTD_outBuffer { dst: or_.as_mut_ptr() as *mut c_void, size: or_.len(), pos: 0 };
        for k in 0..60 {
            let mut ibc = ZSTD_inBuffer { src: src.as_ptr() as *const c_void, size: 0, pos: 0 };
            let mut ibr = ibc;
            let a = s2c(cctx.c, &mut obc, &mut ibc, ZSTD_e_continue);
            let b = s2r(cctx.r, &mut obr, &mut ibr, ZSTD_e_continue);
            let tag = format!("zero-progress in iter={k}");
            eqcode(&tag, a, b);
            eqv(&format!("{tag} in.pos"), ibc.pos, ibr.pos);
            eqv(&format!("{tag} out.pos"), obc.pos, obr.pos);
        }
        // also with a NULL src and size 0
        for k in 0..20 {
            let mut ibc = ZSTD_inBuffer { src: ptr::null(), size: 0, pos: 0 };
            let mut ibr = ibc;
            let a = s2c(cctx.c, &mut obc, &mut ibc, ZSTD_e_continue);
            let b = s2r(cctx.r, &mut obr, &mut ibr, ZSTD_e_continue);
            eqcode(&format!("zero-progress null-in iter={k}"), a, b);
            eqv(&format!("zero-progress null-in iter={k} out.pos"), obc.pos, obr.pos);
        }
        eqbuf("zero-progress in dst", &oc, &or_);

        // ---- (c) legacy flushStream / endStream into a zero-sized output
        {
            let cs = CtxPair::cstream();
            let (icc, icr) = duo::<FnInitCStream>("ZSTD_initCStream");
            let (csc, csr) = duo::<FnStream1>("ZSTD_compressStream");
            eqcode("initCStream", icc(cs.c, 3), icr(cs.r, 3));
            let mut oc = vec![0xE3u8; 1 << 18];
            let mut or_ = vec![0xE3u8; 1 << 18];
            let mut obc = ZSTD_outBuffer { dst: oc.as_mut_ptr() as *mut c_void, size: 0, pos: 0 };
            let mut obr = ZSTD_outBuffer { dst: or_.as_mut_ptr() as *mut c_void, size: 0, pos: 0 };
            let mut ibc = ZSTD_inBuffer { src: src.as_ptr() as *const c_void, size: src.len(), pos: 0 };
            let mut ibr = ibc;
            for k in 0..30 {
                let a = csc(cs.c, &mut obc, &mut ibc);
                let b = csr(cs.r, &mut obr, &mut ibr);
                eqcode(&format!("compressStream out.size=0 iter={k}"), a, b);
                eqv(&format!("compressStream out.size=0 iter={k} in.pos"), ibc.pos, ibr.pos);
                eqv(&format!("compressStream out.size=0 iter={k} out.pos"), obc.pos, obr.pos);
                let a = flc(cs.c, &mut obc);
                let b = flr(cs.r, &mut obr);
                eqcode(&format!("flushStream out.size=0 iter={k}"), a, b);
                let a = enc(cs.c, &mut obc);
                let b = enr(cs.r, &mut obr);
                eqcode(&format!("endStream out.size=0 iter={k}"), a, b);
                eqv(&format!("endStream out.size=0 iter={k} out.pos"), obc.pos, obr.pos);
            }
            eqbuf("zero-progress legacy dst", &oc, &or_);
        }
    }
}

// ------------------------------------------------------------------ rows 145, 146, 148
// plus ZSTD_getBlockSize / ZSTD_getSeqStore

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SeqDefRaw {
    offBase: c_uint,
    litLength: u16,
    mlBase: u16,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct SeqStoreRaw {
    sequencesStart: *mut SeqDefRaw,
    sequences: *mut SeqDefRaw,
    litStart: *mut u8,
    lit: *mut u8,
    llCode: *mut u8,
    mlCode: *mut u8,
    ofCode: *mut u8,
    maxNbSeq: usize,
    maxNbLit: usize,
    longLengthType: c_int,
    longLengthPos: c_uint,
}

#[test]
fn err_write_helpers_dst_too_small() {
    unsafe {
        let (wsc, wsr) = duo::<FnWriteSkippable>("ZSTD_writeSkippableFrame");
        let (wlc, wlr) = duo::<FnWriteLastEmpty>("ZSTD_writeLastEmptyBlock");
        let (gsc, gsr) = duo::<FnGetSeqStore>("ZSTD_getSeqStore");
        let (gbc, gbr) = duo::<FnGetBlockSize>("ZSTD_getBlockSize");
        let (bgc, bgr) = duo::<FnCompressBegin>("ZSTD_compressBegin");
        let (spc, spr) = duo::<FnSetParam>("ZSTD_CCtx_setParameter");
        let (rsc, rsr) = duo::<FnReset>("ZSTD_CCtx_reset");
        let (c2c, c2r) = duo::<FnCompress2>("ZSTD_compress2");
        let (bd, _) = duo::<FnSizeT1>("ZSTD_compressBound");

        // ---- ZSTD_writeSkippableFrame (row 145 dstSize_tooSmall)
        let mut saw_small = false;
        for &sz in &[0usize, 1, 7, 8, 100, 4096] {
            let payload = gen_class(3, sz, sz as u64);
            let ps = if sz == 0 { ptr::null() } else { payload.as_ptr() as *const c_void };
            for cap in 0..(sz + 10) {
                let mut oc = vec![0xF1u8; sz + 16];
                let mut or_ = vec![0xF1u8; sz + 16];
                for mv in [0u32, 7, 15] {
                    let a = wsc(oc.as_mut_ptr() as *mut c_void, cap, ps, sz, mv);
                    let b = wsr(or_.as_mut_ptr() as *mut c_void, cap, ps, sz, mv);
                    let tag = format!("writeSkippableFrame(sz={sz},cap={cap},mv={mv})");
                    eqcode(&tag, a, b);
                    eqbuf(&format!("{tag} dst"), &oc, &or_);
                    if is_err(a) {
                        expect_code(&tag, a, "Destination buffer is too small");
                        saw_small = true;
                    }
                }
            }
            // dst == NULL, capacity too small: the check fires before any write
            for cap in 0..(sz + 8).min(8) {
                let a = wsc(ptr::null_mut(), cap, ps, sz, 0);
                let b = wsr(ptr::null_mut(), cap, ps, sz, 0);
                eqcode(&format!("writeSkippableFrame(dst=NULL,sz={sz},cap={cap})"), a, b);
            }
        }
        assert!(saw_small, "row 145 never triggered");

        // ---- row 146: srcSize > 0xFFFFFFFF.
        // The dstCapacity check comes first, so it must be satisfied; nothing is
        // written and `src` is never dereferenced before the srcSize check.
        for &sz in &[0x1_0000_0000usize, 0x1_0000_0001, usize::MAX / 2] {
            let cap = sz.wrapping_add(8);
            let mut oc = vec![0xF2u8; 64];
            let mut or_ = vec![0xF2u8; 64];
            let a = wsc(oc.as_mut_ptr() as *mut c_void, cap, ptr::null(), sz, 0);
            let b = wsr(or_.as_mut_ptr() as *mut c_void, cap, ptr::null(), sz, 0);
            let tag = format!("writeSkippableFrame(srcSize={sz})");
            eqcode(&tag, a, b);
            expect_code(&tag, a, "Src size is incorrect");
            eqbuf(&format!("{tag} dst"), &oc, &or_);
        }

        // ---- ZSTD_writeLastEmptyBlock (row 148)
        let mut saw_le = false;
        for cap in 0..8usize {
            let mut oc = vec![0xF3u8; 16];
            let mut or_ = vec![0xF3u8; 16];
            let a = wlc(oc.as_mut_ptr() as *mut c_void, cap);
            let b = wlr(or_.as_mut_ptr() as *mut c_void, cap);
            let tag = format!("writeLastEmptyBlock(cap={cap})");
            eqcode(&tag, a, b);
            eqbuf(&format!("{tag} dst"), &oc, &or_);
            if is_err(a) {
                expect_code(&tag, a, "Destination buffer is too small");
                saw_le = true;
            }
            let a = wlc(ptr::null_mut(), cap.min(2));
            let b = wlr(ptr::null_mut(), cap.min(2));
            eqcode(&format!("writeLastEmptyBlock(dst=NULL,cap={})", cap.min(2)), a, b);
        }
        assert!(saw_le, "row 148 never triggered");

        // ---- ZSTD_getBlockSize / ZSTD_getSeqStore across the whole state space
        let cctx = CtxPair::cctx();
        let data = gen_class(4, 60_000, 61);
        for &(p, v) in &[
            (ZSTD_c_compressionLevel, 1),
            (ZSTD_c_compressionLevel, 19),
            (ZSTD_c_windowLog, 10),
            (ZSTD_c_maxBlockSize, 1024),
            (ZSTD_c_maxBlockSize, 131072),
        ] {
            eqcode(
                "reset getBlockSize",
                rsc(cctx.c, ZSTD_reset_session_and_parameters),
                rsr(cctx.r, ZSTD_reset_session_and_parameters),
            );
            eqcode(&format!("set({p},{v})"), spc(cctx.c, p, v), spr(cctx.r, p, v));
            eqv(&format!("getBlockSize before compress ({p},{v})"), gbc(cctx.c), gbr(cctx.r));
            let cap = bd(data.len()) + 64;
            let mut oc = vec![0u8; cap];
            let mut or_ = vec![0u8; cap];
            let a = c2c(
                cctx.c,
                oc.as_mut_ptr() as *mut c_void,
                cap,
                data.as_ptr() as *const c_void,
                data.len(),
            );
            let b = c2r(
                cctx.r,
                or_.as_mut_ptr() as *mut c_void,
                cap,
                data.as_ptr() as *const c_void,
                data.len(),
            );
            eqcode(&format!("compress2 for getBlockSize ({p},{v})"), a, b);
            eqv(&format!("getBlockSize after compress ({p},{v})"), gbc(cctx.c), gbr(cctx.r));
            // ZSTD_getSeqStore returns an interior pointer; compare the derived
            // offsets and the scalar fields, which are fully specified.
            let sc = &*(gsc(cctx.c) as *const SeqStoreRaw);
            let sr = &*(gsr(cctx.r) as *const SeqStoreRaw);
            eqv(
                &format!("getSeqStore nbSeq ({p},{v})"),
                sc.sequences.offset_from(sc.sequencesStart),
                sr.sequences.offset_from(sr.sequencesStart),
            );
            eqv(
                &format!("getSeqStore nbLit ({p},{v})"),
                sc.lit.offset_from(sc.litStart),
                sr.lit.offset_from(sr.litStart),
            );
            eqv(&format!("getSeqStore maxNbSeq ({p},{v})"), sc.maxNbSeq, sr.maxNbSeq);
            eqv(&format!("getSeqStore maxNbLit ({p},{v})"), sc.maxNbLit, sr.maxNbLit);
            eqv(
                &format!("getSeqStore longLengthType ({p},{v})"),
                sc.longLengthType,
                sr.longLengthType,
            );
            eqv(
                &format!("getSeqStore longLengthPos ({p},{v})"),
                sc.longLengthPos,
                sr.longLengthPos,
            );
        }
        // and on a bufferless session
        {
            let bctx = CtxPair::cctx();
            eqcode("compressBegin", bgc(bctx.c, 5), bgr(bctx.r, 5));
            eqv("getBlockSize after begin", gbc(bctx.c), gbr(bctx.r));
            let sc = &*(gsc(bctx.c) as *const SeqStoreRaw);
            let sr = &*(gsr(bctx.r) as *const SeqStoreRaw);
            eqv("getSeqStore maxNbSeq after begin", sc.maxNbSeq, sr.maxNbSeq);
            eqv("getSeqStore maxNbLit after begin", sc.maxNbLit, sr.maxNbLit);
        }
    }
}

// ------------------------------------------------------------------ rows 135, 138, 140, 141

#[test]
fn err_generate_sequences() {
    unsafe {
        let (gsc, gsr) = duo::<FnGenSeq>("ZSTD_generateSequences");
        let (bnd, _) = duo::<FnSizeT1>("ZSTD_sequenceBound");
        let (spc, spr) = duo::<FnSetParam>("ZSTD_CCtx_setParameter");
        let (rsc, rsr) = duo::<FnReset>("ZSTD_CCtx_reset");
        let cctx = CtxPair::cctx();

        // ---- rows 140/141: a block too small to compress while collecting
        // sequences => sequenceProducer_failed ("Uncompressible block").
        // MIN_CBLOCK_SIZE+ZSTD_blockHeaderSize+1+1 == 7, so srcSize in 1..6.
        let mut saw_uncompressible = 0usize;
        for split in [ZSTD_ps_auto, ZSTD_ps_enable, ZSTD_ps_disable] {
            for bsl in [0, 1, 6] {
                for cls in 0..N_CLASSES {
                    for sz in 0..8usize {
                        let src = gen_class(cls, sz, 0xC400 ^ sz as u64);
                        eqcode(
                            "reset genseq",
                            rsc(cctx.c, ZSTD_reset_session_and_parameters),
                            rsr(cctx.r, ZSTD_reset_session_and_parameters),
                        );
                        for (p, v) in [
                            (ZSTD_c_splitAfterSequences, split),
                            (ZSTD_c_blockSplitterLevel, bsl),
                        ] {
                            eqcode(
                                &format!("genseq set({p},{v})"),
                                spc(cctx.c, p, v),
                                spr(cctx.r, p, v),
                            );
                        }
                        let cap = bnd(sz).max(1);
                        let mut sc = vec![ZSTD_Sequence::default(); cap];
                        let mut sr = vec![ZSTD_Sequence::default(); cap];
                        let sp = if sz == 0 { ptr::null() } else { src.as_ptr() as *const c_void };
                        let a = gsc(cctx.c, sc.as_mut_ptr(), cap, sp, sz);
                        let b = gsr(cctx.r, sr.as_mut_ptr(), cap, sp, sz);
                        let tag = format!("generateSequences split={split} bsl={bsl} cls={cls} sz={sz}");
                        eqcode(&tag, a, b);
                        eqv(&format!("{tag} seqs"), &sc[..], &sr[..]);
                        if is_err(a)
                            && errname(a) == "Block-level external sequence producer returned an error code"
                        {
                            saw_uncompressible += 1;
                        }
                    }
                }
            }
        }
        assert!(
            saw_uncompressible > 0,
            "rows 140/141 (sequenceProducer_failed, uncompressible block) never triggered"
        );

        // ---- row 135: the collector's output array is too small
        let mut saw_small = 0usize;
        for cls in 0..N_CLASSES {
            for &sz in &[100usize, 5000, 60_000, 300_000] {
                let src = gen_class(cls, sz, 0xC401 ^ sz as u64);
                eqcode(
                    "reset genseq small",
                    rsc(cctx.c, ZSTD_reset_session_and_parameters),
                    rsr(cctx.r, ZSTD_reset_session_and_parameters),
                );
                let full = bnd(sz).max(1);
                for cap in [0usize, 1, 2, 8, full / 4, full / 2, full - 1, full] {
                    let mut sc = vec![ZSTD_Sequence::default(); cap.max(1)];
                    let mut sr = vec![ZSTD_Sequence::default(); cap.max(1)];
                    let a = gsc(
                        cctx.c,
                        sc.as_mut_ptr(),
                        cap,
                        src.as_ptr() as *const c_void,
                        sz,
                    );
                    let b = gsr(
                        cctx.r,
                        sr.as_mut_ptr(),
                        cap,
                        src.as_ptr() as *const c_void,
                        sz,
                    );
                    let tag = format!("generateSequences cls={cls} sz={sz} cap={cap}");
                    eqcode(&tag, a, b);
                    eqv(&format!("{tag} seqs"), &sc[..], &sr[..]);
                    if is_err(a) && errname(a) == "Destination buffer is too small" {
                        saw_small += 1;
                    }
                }
            }
        }
        assert!(saw_small > 0, "row 135 (copyBlockSequences dstSize_tooSmall) never triggered");

        // ---- row 138: the internal scratch allocation fails.
        // `dstCapacity = ZSTD_compressBound(srcSize)`; for srcSize beyond
        // ZSTD_MAX_INPUT_SIZE that is the (huge) error sentinel, so the
        // ZSTD_customMalloc() cannot succeed. `src` is never dereferenced.
        for &sz in &[usize::MAX, usize::MAX / 2, 1usize << 62] {
            let mut sc = vec![ZSTD_Sequence::default(); 8];
            let mut sr = vec![ZSTD_Sequence::default(); 8];
            eqcode(
                "reset genseq huge",
                rsc(cctx.c, ZSTD_reset_session_and_parameters),
                rsr(cctx.r, ZSTD_reset_session_and_parameters),
            );
            let a = gsc(cctx.c, sc.as_mut_ptr(), 8, ptr::null(), sz);
            let b = gsr(cctx.r, sr.as_mut_ptr(), 8, ptr::null(), sz);
            let tag = format!("generateSequences(srcSize={sz})");
            eqcode(&tag, a, b);
            expect_code(&tag, a, "Allocation error : not enough memory");
            eqv(&format!("{tag} seqs"), &sc[..], &sr[..]);
        }
    }
}

// ------------------------------------------------------------------ rows 193-206, 209-217, 219
//
// Malformed `ZSTD_Sequence` arrays through `ZSTD_compressSequences` and
// `ZSTD_compressSequencesAndLiterals`.

/// A synthetic, *structurally valid* parse of `srcSize` bytes:
/// `n` sequences of (litLength=`ll`, matchLength=`ml`, offset=1) followed by the
/// mandatory `{offset:0, matchLength:0, litLength:rest}` block delimiter.
fn synth_seqs(n: usize, ll: u32, ml: u32, src_size: usize) -> Vec<ZSTD_Sequence> {
    let covered = n * (ll as usize + ml as usize);
    assert!(covered <= src_size);
    let mut v: Vec<ZSTD_Sequence> = (0..n)
        .map(|_| ZSTD_Sequence { offset: 1, litLength: ll, matchLength: ml, rep: 0 })
        .collect();
    v.push(ZSTD_Sequence {
        offset: 0,
        litLength: (src_size - covered) as c_uint,
        matchLength: 0,
        rep: 0,
    });
    v
}

fn seq_sum(s: &[ZSTD_Sequence]) -> usize {
    s.iter().map(|x| x.litLength as usize + x.matchLength as usize).sum()
}

#[test]
fn err_compress_sequences_malformed() {
    unsafe {
        let (csc, csr) = duo::<FnCompressSeq>("ZSTD_compressSequences");
        let (spc, spr) = duo::<FnSetParam>("ZSTD_CCtx_setParameter");
        let (rsc, rsr) = duo::<FnReset>("ZSTD_CCtx_reset");
        let (bd, _) = duo::<FnSizeT1>("ZSTD_compressBound");
        let cctx = CtxPair::cctx();

        // src is padded so that ZSTD_wildcopy()'s 8-byte overread inside
        // ZSTD_storeSeq() always stays inside the allocation.
        const PAD: usize = 64;
        let src_size = 4096usize;
        let mut src = gen_class(4, src_size, 71);
        src.extend_from_slice(&[0u8; PAD]);
        let sp = src.as_ptr() as *const c_void;

        let base = synth_seqs(400, 4, 6, src_size); // 400*10 = 4000, delim ll=96
        assert_eq!(seq_sum(&base), src_size);

        // Mutations. Each entry is (name, sequences, srcSize, expected-C-error
        // or "" when the outcome is parameter-dependent).
        let mut cases: Vec<(String, Vec<ZSTD_Sequence>, usize)> = Vec::new();
        cases.push(("valid".into(), base.clone(), src_size));
        // offset == 0 on a NON-terminal sequence but matchLength != 0 (row 199)
        {
            let mut s = base.clone();
            s[10].offset = 0;
            cases.push(("delim-with-matchlength".into(), s, src_size));
        }
        // no terminating delimiter at all (row 200)
        {
            let mut s = base.clone();
            s.pop();
            cases.push(("no-delimiter".into(), s, src_size));
        }
        {
            cases.push(("empty-seqs".into(), Vec::new(), src_size));
        }
        // sum > blockSizeMax (row 201) -- inflate one litLength a lot
        {
            let mut s = base.clone();
            s[0].litLength = 300_000;
            cases.push(("block-too-large".into(), s, src_size));
        }
        // sum > srcSize but <= blockSizeMax (row 202)
        {
            let mut s = base.clone();
            s[0].litLength += 1000;
            cases.push(("frame-longer-than-src".into(), s, src_size));
        }
        // offset far beyond the window (row 193, only with validateSequences)
        {
            let mut s = base.clone();
            s[5].offset = 0x4000_0000;
            cases.push(("offset-too-large".into(), s, src_size));
        }
        // matchLength below ZSTD_MINMATCH_MIN (row 194), sum preserved by
        // moving the slack into the terminating delimiter
        for ml in [0u32, 1, 2, 3] {
            let mut s = base.clone();
            let d = 6 - ml;
            s[7].matchLength = ml;
            let last = s.len() - 1;
            s[last].litLength += d;
            if ml == 0 {
                // matchLength == 0 with offset != 0 is also the row-199 shape
                cases.push((format!("matchlength-{ml}"), s, src_size));
            } else {
                cases.push((format!("matchlength-{ml}"), s, src_size));
            }
        }
        // more sequences in one block than seqStore.maxNbSeq (rows 195/198):
        // blockSizeMax is forced down to 1024, so maxNbSeq == 1024/4 == 256
        {
            let s = synth_seqs(341, 0, 3, 1024);
            cases.push(("too-many-seqs".into(), s, 1024));
        }
        // sum < srcSize: the frame ends before the source does
        {
            let mut s = base.clone();
            let last = s.len() - 1;
            s[last].litLength = 0;
            cases.push(("frame-shorter-than-src".into(), s, src_size));
        }
        // a lone delimiter for a non-empty source
        {
            cases.push((
                "lone-delimiter".into(),
                vec![ZSTD_Sequence { offset: 0, litLength: 0, matchLength: 0, rep: 0 }],
                src_size,
            ));
        }
        // empty source with sequences
        {
            cases.push(("empty-src".into(), base.clone(), 0));
        }

        let mut n_ext = 0usize;
        for (name, seqs, ssz) in &cases {
            for delim in [0, 1] {
                for vs in [0, 1] {
                    for rr_ in [ZSTD_ps_auto, ZSTD_ps_enable, ZSTD_ps_disable] {
                        for mbs in [0, 1024] {
                            eqcode(
                                "reset seqs",
                                rsc(cctx.c, ZSTD_reset_session_and_parameters),
                                rsr(cctx.r, ZSTD_reset_session_and_parameters),
                            );
                            for (p, v) in [
                                (ZSTD_c_blockDelimiters, delim),
                                (ZSTD_c_validateSequences, vs),
                                (ZSTD_c_repcodeResolution, rr_),
                                (ZSTD_c_maxBlockSize, mbs),
                                (ZSTD_c_windowLog, 17),
                            ] {
                                eqcode(
                                    &format!("seqs set({p},{v})"),
                                    spc(cctx.c, p, v),
                                    spr(cctx.r, p, v),
                                );
                            }
                            let cap = bd(*ssz) + 4096;
                            let mut oc = vec![0x4Au8; cap];
                            let mut or_ = vec![0x4Au8; cap];
                            let sptr = if *ssz == 0 { ptr::null() } else { sp };
                            let a = csc(
                                cctx.c,
                                oc.as_mut_ptr() as *mut c_void,
                                cap,
                                seqs.as_ptr(),
                                seqs.len(),
                                sptr,
                                *ssz,
                            );
                            let b = csr(
                                cctx.r,
                                or_.as_mut_ptr() as *mut c_void,
                                cap,
                                seqs.as_ptr(),
                                seqs.len(),
                                sptr,
                                *ssz,
                            );
                            let tag = format!(
                                "compressSequences[{name}] delim={delim} vs={vs} rr={rr_} mbs={mbs} ssz={ssz}"
                            );
                            eqcode(&tag, a, b);
                            eqbuf(&format!("{tag} dst"), &oc, &or_);
                            if is_err(a) && errname(a) == "External sequences are not valid" {
                                n_ext += 1;
                            }
                        }
                    }
                }
            }
        }
        assert!(
            n_ext > 200,
            "externalSequences_invalid barely triggered by compressSequences ({n_ext})"
        );
    }
}

// ------------------------------------------------------------------ rows 203, 204, 209-217, 219
//
// `ZSTD_compressSequencesAndLiterals`: the literals-only variant has its own
// rejection set (workSpace_tooSmall, frameParameter_unsupported,
// cannotProduce_uncompressedBlock, dstSize_tooSmall on the 3-byte block header).

#[test]
fn err_compress_sequences_and_literals() {
    unsafe {
        let (clc, clr) = duo::<FnCompressSeqLit>("ZSTD_compressSequencesAndLiterals");
        let (spc, spr) = duo::<FnSetParam>("ZSTD_CCtx_setParameter");
        let (rsc, rsr) = duo::<FnReset>("ZSTD_CCtx_reset");
        let (bd, _) = duo::<FnSizeT1>("ZSTD_compressBound");
        let cctx = CtxPair::cctx();

        let src_size = 4096usize;
        let base = synth_seqs(400, 4, 6, src_size); // literals: 400*4 + 96 = 1696
        let lit_size = base.iter().map(|s| s.litLength as usize).sum::<usize>();
        assert_eq!(lit_size, 400 * 4 + 96);
        // The literal buffers are oversized (and padded) so that every case can
        // point at them and the documented 8-byte overread stays in bounds.
        const LITBUF: usize = 8192;
        let mut lits = gen_class(4, LITBUF, 81);
        lits.extend_from_slice(&[0u8; 64]);
        // An *incompressible* literal set, so the compressed block cannot fit
        // inside a reduced blockSizeMax and the C must report
        // cannotProduce_uncompressedBlock.
        let mut rnd = gen_class(3, LITBUF, 82);
        rnd.extend_from_slice(&[0u8; 64]);

        struct Case {
            name: &'static str,
            seqs: Vec<ZSTD_Sequence>,
            src_size: usize,
            lit_size: usize,
        }
        let mut cases: Vec<Case> = Vec::new();
        cases.push(Case { name: "valid", seqs: base.clone(), src_size, lit_size });
        cases.push(Case { name: "nbSeq0", seqs: Vec::new(), src_size, lit_size });
        {
            let mut s = base.clone();
            s.pop(); // no terminating delimiter (rows 207/208)
            cases.push(Case { name: "no-delimiter", seqs: s, src_size, lit_size });
        }
        {
            // sequences claim more literals than the buffer holds (row 211)
            cases.push(Case { name: "lit-too-few", seqs: base.clone(), src_size, lit_size: 8 });
        }
        {
            // literals left over at the end (row 214)
            cases.push(Case {
                name: "lit-left-over",
                seqs: base.clone(),
                src_size,
                lit_size: lit_size + 100,
            });
        }
        {
            // `remaining != 0`: pledge more decompressed bytes than the
            // sequences describe (row 215)
            cases.push(Case {
                name: "remaining",
                seqs: base.clone(),
                src_size: src_size + 500,
                lit_size,
            });
        }
        {
            // the "empty frame" special case: one delimiter with litLength == 0
            cases.push(Case {
                name: "empty-frame",
                seqs: vec![ZSTD_Sequence { offset: 0, litLength: 0, matchLength: 0, rep: 0 }],
                src_size: 0,
                lit_size: 0,
            });
        }
        {
            // more sequences than seqStore.maxNbSeq (row 206)
            cases.push(Case {
                name: "too-many-seqs",
                seqs: synth_seqs(341, 0, 3, 1024),
                src_size: 1024,
                lit_size: 0,
            });
        }
        {
            // few sequences but a big, incompressible literal payload: with
            // ZSTD_c_maxBlockSize reduced to 1024 the compressed block exceeds
            // blockSizeMax, so `compressedSeqsSize` is forced to 0 and the C
            // must answer cannotProduce_uncompressedBlock (row 213).
            cases.push(Case {
                name: "block-over-blocksizemax",
                seqs: synth_seqs(10, 200, 6, 2060),
                src_size: 2060,
                lit_size: 2000,
            });
        }

        let mut seen: Vec<String> = Vec::new();
        for c in &cases {
            for delim in [0, 1] {
                for vs in [0, 1] {
                    for cks in [0, 1] {
                        for mbs in [0, 1024] {
                            for incompressible in [false, true] {
                                let litbuf: &Vec<u8> = if incompressible { &rnd } else { &lits };
                                // litCapacity variants: below litSize triggers
                                // workSpace_tooSmall (row 216)
                                for &litcap in
                                    &[c.lit_size + 8, c.lit_size, c.lit_size / 2, 0usize]
                                {
                                    eqcode(
                                        "reset csal",
                                        rsc(cctx.c, ZSTD_reset_session_and_parameters),
                                        rsr(cctx.r, ZSTD_reset_session_and_parameters),
                                    );
                                    for (p, v) in [
                                        (ZSTD_c_blockDelimiters, delim),
                                        (ZSTD_c_validateSequences, vs),
                                        (ZSTD_c_checksumFlag, cks),
                                        (ZSTD_c_maxBlockSize, mbs),
                                        (ZSTD_c_windowLog, 17),
                                    ] {
                                        eqcode(
                                            &format!("csal set({p},{v})"),
                                            spc(cctx.c, p, v),
                                            spr(cctx.r, p, v),
                                        );
                                    }
                                    let cap = bd(c.src_size) + 4096;
                                    let mut oc = vec![0x6Eu8; cap];
                                    let mut or_ = vec![0x6Eu8; cap];
                                    let lp = litbuf.as_ptr() as *const c_void;
                                    let a = clc(
                                        cctx.c,
                                        oc.as_mut_ptr() as *mut c_void,
                                        cap,
                                        c.seqs.as_ptr(),
                                        c.seqs.len(),
                                        lp,
                                        c.lit_size,
                                        litcap,
                                        c.src_size,
                                    );
                                    let b = clr(
                                        cctx.r,
                                        or_.as_mut_ptr() as *mut c_void,
                                        cap,
                                        c.seqs.as_ptr(),
                                        c.seqs.len(),
                                        lp,
                                        c.lit_size,
                                        litcap,
                                        c.src_size,
                                    );
                                    let tag = format!(
                                        "csal[{}] delim={delim} vs={vs} cks={cks} mbs={mbs} rnd={incompressible} litcap={litcap}",
                                        c.name
                                    );
                                    eqcode(&tag, a, b);
                                    eqbuf(&format!("{tag} dst"), &oc, &or_);
                                    if is_err(a) {
                                        let n = errname(a);
                                        if !seen.contains(&n) {
                                            seen.push(n);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        for want in [
            "workSpace buffer is not large enough",
            "Unsupported parameter",
            "External sequences are not valid",
            "This mode cannot generate an uncompressed block",
            "Unsupported frame parameter",
        ] {
            assert!(
                seen.iter().any(|s| s == want),
                "compressSequencesAndLiterals never produced `{want}`; saw {seen:?}"
            );
        }

        // ---- rows 203/204/210/212: too-small destinations.
        // UPSTREAM C OUT-OF-BOUNDS WRITE (see CONFIGS.md X2): with a dstCapacity
        // smaller than the frame needs, ZSTD_compressSequences* writes *below*
        // `dst` before returning dstSize_tooSmall (measured at ~70 bytes for
        // dstCapacity == 10). The Rust port reproduces the same pointer
        // arithmetic, so both scribble the same bytes in the same place. To keep
        // the row differential without corrupting the test heap, both
        // destinations get a 64 KiB canary guard band on each side and the WHOLE
        // padded region is compared.
        const GUARD: usize = 64 * 1024;
        let mut saw_small = 0usize;
        for c in &cases {
            for delim in [0, 1] {
                for tcap in [0usize, 1, 2, 3, 4, 6, 12, 17, 18, 40, 300] {
                    eqcode(
                        "reset csal small",
                        rsc(cctx.c, ZSTD_reset_session_and_parameters),
                        rsr(cctx.r, ZSTD_reset_session_and_parameters),
                    );
                    for (p, v) in [
                        (ZSTD_c_blockDelimiters, delim),
                        (ZSTD_c_windowLog, 17),
                    ] {
                        eqcode(
                            &format!("csal small set({p},{v})"),
                            spc(cctx.c, p, v),
                            spr(cctx.r, p, v),
                        );
                    }
                    let mut q1 = vec![0xAAu8; GUARD + tcap + GUARD];
                    let mut q2 = vec![0xAAu8; GUARD + tcap + GUARD];
                    let a = clc(
                        cctx.c,
                        q1.as_mut_ptr().add(GUARD) as *mut c_void,
                        tcap,
                        c.seqs.as_ptr(),
                        c.seqs.len(),
                        lits.as_ptr() as *const c_void,
                        c.lit_size,
                        c.lit_size + 8,
                        c.src_size,
                    );
                    let b = clr(
                        cctx.r,
                        q2.as_mut_ptr().add(GUARD) as *mut c_void,
                        tcap,
                        c.seqs.as_ptr(),
                        c.seqs.len(),
                        lits.as_ptr() as *const c_void,
                        c.lit_size,
                        c.lit_size + 8,
                        c.src_size,
                    );
                    let tag = format!("csal-small[{}] delim={delim} tcap={tcap}", c.name);
                    eqcode(&tag, a, b);
                    eqbuf(&format!("{tag} padded dst"), &q1, &q2);
                    if is_err(a) && errname(a) == "Destination buffer is too small" {
                        saw_small += 1;
                    }
                }
            }
        }
        assert!(saw_small > 0, "rows 203/204/210/212 never triggered");
    }
}

// ------------------------------------------------------------------ rows 129, 130, 131, 134,
// 193, 194, 197 (through the block-level external sequence producer)

#[repr(C)]
struct SpState {
    mode: c_int,
    calls: c_int,
}

/// A user sequence producer covering every failure shape the contract in
/// `zstd.h` (~L2820) describes.
unsafe extern "C" fn seq_producer(
    state: *mut c_void,
    out: *mut ZSTD_Sequence,
    cap: usize,
    _src: *const c_void,
    src_size: usize,
    _dict: *const c_void,
    _dict_size: usize,
    _level: c_int,
    _window: usize,
) -> usize {
    let st = &mut *(state as *mut SpState);
    st.calls += 1;
    let put = |i: usize, off: c_uint, ll: c_uint, ml: c_uint| {
        *out.add(i) = ZSTD_Sequence { offset: off, litLength: ll, matchLength: ml, rep: 0 };
    };
    match st.mode {
        // ZSTD_SEQUENCE_PRODUCER_ERROR (row 129)
        0 => usize::MAX,
        // one past capacity: also "greater than outSeqsCapacity" (row 129)
        1 => cap + 1,
        // zero sequences for a non-empty block (row 130)
        2 => 0,
        // exactly `cap` sequences, none of which is a block delimiter (row 131)
        3 => {
            if cap == 0 {
                return 0;
            }
            for i in 0..cap {
                put(i, 1, 0, 4);
            }
            cap
        }
        // a *valid* parse: everything is literals, terminated by a delimiter
        4 => {
            if cap == 0 {
                return 0;
            }
            put(0, 0, src_size as c_uint, 0);
            1
        }
        // the parse claims more bytes than the block holds (row 134)
        5 => {
            if cap == 0 {
                return 0;
            }
            put(0, 0, (src_size + 1000) as c_uint, 0);
            1
        }
        // offset far beyond the window (row 193 with ZSTD_c_validateSequences)
        6 => {
            if cap < 2 || src_size < 4 {
                return 0;
            }
            put(0, 1 << 29, 0, src_size as c_uint);
            put(1, 0, 0, 0);
            2
        }
        // matchLength below ZSTD_MINMATCH_MIN (row 194)
        7 => {
            if cap < 2 || src_size < 4 {
                return 0;
            }
            put(0, 1, 0, 1);
            put(1, 0, (src_size - 1) as c_uint, 0);
            2
        }
        // a parse that covers only half the block: `ip != iend` (row 197)
        8 => {
            if cap == 0 {
                return 0;
            }
            put(0, 0, (src_size / 2) as c_uint, 0);
            1
        }
        // a delimiter whose matchLength is non-zero (row 199 shape)
        9 => {
            if cap < 2 || src_size < 8 {
                return 0;
            }
            put(0, 0, (src_size - 4) as c_uint, 4);
            put(1, 0, 0, 0);
            2
        }
        // many tiny sequences: exceeds seqStore.maxNbSeq (rows 195/198)
        _ => {
            let n = (src_size / 3).min(cap.saturating_sub(1));
            if n == 0 || cap == 0 {
                return 0;
            }
            for i in 0..n {
                put(i, 1, 0, 3);
            }
            put(n, 0, (src_size - n * 3) as c_uint, 0);
            n + 1
        }
    }
}

#[test]
fn err_sequence_producer_failed() {
    unsafe {
        let (rgc, rgr) = duo::<FnRegisterSeqProd>("ZSTD_registerSequenceProducer");
        let (rpc, rpr) = duo::<
            unsafe extern "C" fn(*mut c_void, *mut c_void, Option<FnSeqProducer>),
        >("ZSTD_CCtxParams_registerSequenceProducer");
        let (c2c, c2r) = duo::<FnCompress2>("ZSTD_compress2");
        let (s2c, s2r) = duo::<FnStream2>("ZSTD_compressStream2");
        let (spc, spr) = duo::<FnSetParam>("ZSTD_CCtx_setParameter");
        let (rsc, rsr) = duo::<FnReset>("ZSTD_CCtx_reset");
        let (bd, _) = duo::<FnSizeT1>("ZSTD_compressBound");
        let cctx = CtxPair::cctx();

        let mut seen: Vec<String> = Vec::new();
        for mode in 0..11 {
            for fallback in [0, 1] {
                for vs in [0, 1] {
                    for mbs in [0, 1024] {
                        for &sz in &[0usize, 1, 7, 200, 5000, 140_000] {
                            let src = gen_class(4, sz, 0xC500 ^ sz as u64);
                            let sp =
                                if sz == 0 { ptr::null() } else { src.as_ptr() as *const c_void };
                            let mut stc = SpState { mode, calls: 0 };
                            let mut str_ = SpState { mode, calls: 0 };
                            eqcode(
                                "reset seqprod",
                                rsc(cctx.c, ZSTD_reset_session_and_parameters),
                                rsr(cctx.r, ZSTD_reset_session_and_parameters),
                            );
                            for (p, v) in [
                                (ZSTD_c_enableSeqProducerFallback, fallback),
                                (ZSTD_c_validateSequences, vs),
                                (ZSTD_c_maxBlockSize, mbs),
                                (ZSTD_c_windowLog, 17),
                            ] {
                                eqcode(
                                    &format!("seqprod set({p},{v})"),
                                    spc(cctx.c, p, v),
                                    spr(cctx.r, p, v),
                                );
                            }
                            rgc(cctx.c, &mut stc as *mut SpState as *mut c_void, Some(seq_producer));
                            rgr(cctx.r, &mut str_ as *mut SpState as *mut c_void, Some(seq_producer));
                            let cap = bd(sz) + 4096;
                            let mut oc = vec![0x7Fu8; cap];
                            let mut or_ = vec![0x7Fu8; cap];
                            let a = c2c(cctx.c, oc.as_mut_ptr() as *mut c_void, cap, sp, sz);
                            let b = c2r(cctx.r, or_.as_mut_ptr() as *mut c_void, cap, sp, sz);
                            let tag = format!(
                                "seqprod mode={mode} fb={fallback} vs={vs} mbs={mbs} sz={sz}"
                            );
                            eqcode(&tag, a, b);
                            eqbuf(&format!("{tag} dst"), &oc, &or_);
                            eqv(&format!("{tag} producer calls"), stc.calls, str_.calls);
                            if is_err(a) {
                                let n = errname(a);
                                if !seen.contains(&n) {
                                    seen.push(n);
                                }
                            }

                            // and the same through streaming
                            let mut stc = SpState { mode, calls: 0 };
                            let mut str_ = SpState { mode, calls: 0 };
                            eqcode(
                                "reset seqprod stream",
                                rsc(cctx.c, ZSTD_reset_session_and_parameters),
                                rsr(cctx.r, ZSTD_reset_session_and_parameters),
                            );
                            for (p, v) in [
                                (ZSTD_c_enableSeqProducerFallback, fallback),
                                (ZSTD_c_validateSequences, vs),
                                (ZSTD_c_maxBlockSize, mbs),
                                (ZSTD_c_windowLog, 17),
                            ] {
                                eqcode(
                                    &format!("seqprod stream set({p},{v})"),
                                    spc(cctx.c, p, v),
                                    spr(cctx.r, p, v),
                                );
                            }
                            rgc(cctx.c, &mut stc as *mut SpState as *mut c_void, Some(seq_producer));
                            rgr(cctx.r, &mut str_ as *mut SpState as *mut c_void, Some(seq_producer));
                            let mut oc = vec![0x8Fu8; cap];
                            let mut or_ = vec![0x8Fu8; cap];
                            let mut obc = ZSTD_outBuffer {
                                dst: oc.as_mut_ptr() as *mut c_void,
                                size: cap,
                                pos: 0,
                            };
                            let mut obr = ZSTD_outBuffer {
                                dst: or_.as_mut_ptr() as *mut c_void,
                                size: cap,
                                pos: 0,
                            };
                            let mut ibc = ZSTD_inBuffer { src: sp, size: sz, pos: 0 };
                            let mut ibr = ibc;
                            let a = s2c(cctx.c, &mut obc, &mut ibc, ZSTD_e_end);
                            let b = s2r(cctx.r, &mut obr, &mut ibr, ZSTD_e_end);
                            eqcode(&format!("{tag} stream"), a, b);
                            eqv(&format!("{tag} stream in.pos"), ibc.pos, ibr.pos);
                            eqv(&format!("{tag} stream out.pos"), obc.pos, obr.pos);
                            eqbuf(&format!("{tag} stream dst"), &oc, &or_);
                            eqv(&format!("{tag} stream producer calls"), stc.calls, str_.calls);
                            if is_err(a) {
                                let n = errname(a);
                                if !seen.contains(&n) {
                                    seen.push(n);
                                }
                            }
                            // unregister so the next iteration starts clean
                            rgc(cctx.c, ptr::null_mut(), None);
                            rgr(cctx.r, ptr::null_mut(), None);
                        }
                    }
                }
            }
        }
        for want in [
            "Block-level external sequence producer returned an error code",
            "External sequences are not valid",
        ] {
            assert!(
                seen.iter().any(|s| s == want),
                "sequence producer never produced `{want}`; saw {seen:?}"
            );
        }

        // ---- ZSTD_CCtxParams_registerSequenceProducer + size estimation
        {
            let po = CtxPair::cctx_params();
            let (ec, er) = duo::<FnEstimateFromParams>("ZSTD_estimateCCtxSize_usingCCtxParams");
            let (sc, sr) = duo::<FnEstimateFromParams>("ZSTD_estimateCStreamSize_usingCCtxParams");
            let (setp_c, setp_r) = duo::<FnSetParam>("ZSTD_CCtxParams_setParameter");
            let mut st = SpState { mode: 4, calls: 0 };
            for reg in [false, true] {
                if reg {
                    rpc(po.c, &mut st as *mut SpState as *mut c_void, Some(seq_producer));
                    rpr(po.r, &mut st as *mut SpState as *mut c_void, Some(seq_producer));
                } else {
                    rpc(po.c, ptr::null_mut(), None);
                    rpr(po.r, ptr::null_mut(), None);
                }
                for lvl in [1, 3, 19] {
                    eqcode(
                        &format!("params level {lvl}"),
                        setp_c(po.c, ZSTD_c_compressionLevel, lvl),
                        setp_r(po.r, ZSTD_c_compressionLevel, lvl),
                    );
                    eqcode(&format!("estimateCCtxSize reg={reg} lvl={lvl}"), ec(po.c), er(po.r));
                    eqcode(&format!("estimateCStreamSize reg={reg} lvl={lvl}"), sc(po.c), sr(po.r));
                }
            }
            rpc(po.c, ptr::null_mut(), None);
            rpr(po.r, ptr::null_mut(), None);
        }
    }
}

// ------------------------------------------------------------------ rows 152-166, 181
//
// The COMPRESSION-side dictionary loader: `ZSTD_loadCEntropy` /
// `ZSTD_loadZstdDictionary` reached through every entry point that takes a
// dictionary, driven by systematically corrupted zstd dictionaries.

#[test]
fn err_dictionary_corrupted_on_load() {
    unsafe {
        let (ldc, ldr) = duo::<FnLoadDictAdvanced>("ZSTD_CCtx_loadDictionary_advanced");
        let (mkc, mkr) = duo::<FnCreateCDictAdvanced>("ZSTD_createCDict_advanced");
        let (fdc, fdr) = duo::<FnFreePtr>("ZSTD_freeCDict");
        let (bdc, bdr) = duo::<
            unsafe extern "C" fn(*mut c_void, *const c_void, usize, c_int) -> usize,
        >("ZSTD_compressBegin_usingDict");
        let (bcc, bcr) = duo::<FnCompressBeginUsingCDict>("ZSTD_compressBegin_usingCDict");
        let (c2c, c2r) = duo::<FnCompress2>("ZSTD_compress2");
        let (rsc, rsr) = duo::<FnReset>("ZSTD_CCtx_reset");
        let (udc, udr) = duo::<FnCompressUsingDict>("ZSTD_compress_usingDict");
        let (gc, _) = duo::<FnGetCParams>("ZSTD_getCParams");
        let (bd, _) = duo::<FnSizeT1>("ZSTD_compressBound");
        let (train, _) = duo::<
            unsafe extern "C" fn(*mut c_void, usize, *const c_void, *const usize, c_uint) -> usize,
        >("ZDICT_trainFromBuffer");

        // a real trained dictionary as the base for corruption
        let nb = 64usize;
        let each = 2048usize;
        let mut corpus = Vec::new();
        let mut sizes = Vec::new();
        for k in 0..nb {
            corpus.extend_from_slice(&gen_class(4, each, k as u64));
            sizes.push(each);
        }
        let mut dictbuf = vec![0u8; 24 * 1024];
        let dn = train(
            dictbuf.as_mut_ptr() as *mut c_void,
            dictbuf.len(),
            corpus.as_ptr() as *const c_void,
            sizes.as_ptr(),
            nb as c_uint,
        );
        assert!(!is_err(dn), "ZDICT_trainFromBuffer failed");
        let good: Vec<u8> = dictbuf[..dn].to_vec();
        assert_eq!(
            u32::from_le_bytes([good[0], good[1], good[2], good[3]]),
            ZSTD_MAGIC_DICTIONARY
        );

        // Build the corruption set.
        let mut dicts: Vec<(String, Vec<u8>)> = Vec::new();
        dicts.push(("good".into(), good.clone()));
        // every truncation length in the header region, plus a coarse sweep
        for n in 0..40usize {
            dicts.push((format!("trunc{n}"), good[..n.min(good.len())].to_vec()));
        }
        for n in [64usize, 100, 200, 500, 1000, good.len() / 2, good.len() - 13, good.len() - 1] {
            if n < good.len() {
                dicts.push((format!("trunc{n}"), good[..n].to_vec()));
            }
        }
        // single-byte corruptions all over the entropy-table region
        for off in [8usize, 9, 10, 12, 16, 20, 24, 30, 40, 60, 90, 120, 160, 200] {
            if off >= good.len() {
                continue;
            }
            for x in [0x00u8, 0xFF, 0x55] {
                let mut d = good.clone();
                d[off] ^= x;
                d[off] = d[off].wrapping_add(if x == 0 { 1 } else { 0 });
                dicts.push((format!("flip@{off}^{x:#x}"), d));
            }
        }
        // the three repcodes live in the 12 bytes right before the content;
        // zero them / blow them up (rows 163, 164)
        {
            // locate them by re-deriving the header size: load the good dict and
            // ask the C for its dictID-independent layout is not exported, so
            // instead corrupt candidate positions across the whole tail.
            for back in [13usize, 17, 21] {
                if good.len() < back + 4 {
                    continue;
                }
                let at = good.len() - back;
                let mut d = good.clone();
                d[at..at + 4].copy_from_slice(&0u32.to_le_bytes());
                dicts.push((format!("rep0@-{back}"), d));
                let mut d = good.clone();
                d[at..at + 4].copy_from_slice(&0xFFFF_FFFFu32.to_le_bytes());
                dicts.push((format!("repbig@-{back}"), d));
            }
        }
        // a plausible-looking but bogus dictionary: right magic, garbage body
        for n in [8usize, 12, 20, 64, 256, 4096] {
            let mut d = vec![0u8; n];
            if n >= 4 {
                d[..4].copy_from_slice(&ZSTD_MAGIC_DICTIONARY.to_le_bytes());
            }
            let body = gen_class(3, n.saturating_sub(4), n as u64);
            d[4.min(n)..].copy_from_slice(&body[..n - 4.min(n)]);
            dicts.push((format!("magic+garbage{n}"), d));
        }
        // all-zero body after the magic
        for n in [8usize, 20, 64, 4096] {
            let mut d = vec![0u8; n];
            d[..4].copy_from_slice(&ZSTD_MAGIC_DICTIONARY.to_le_bytes());
            dicts.push((format!("magic+zeros{n}"), d));
        }
        // tiny / empty dictionaries (rows 165/166 with dct_fullDict)
        for n in 0..9usize {
            dicts.push((format!("tiny{n}"), gen_class(3, n, n as u64)));
        }

        let src = gen_class(4, 20_000, 91);
        let cp = gc(5, 0, good.len());
        let cctx = CtxPair::cctx();
        let mut seen: Vec<String> = Vec::new();

        for (name, d) in &dicts {
            let (dp, ds) = if d.is_empty() {
                (ptr::null(), 0usize)
            } else {
                (d.as_ptr() as *const c_void, d.len())
            };
            for dct in [ZSTD_dct_auto, ZSTD_dct_rawContent, ZSTD_dct_fullDict] {
                for dlm in [ZSTD_dlm_byCopy, ZSTD_dlm_byRef] {
                    // (a) ZSTD_CCtx_loadDictionary_advanced + compress2
                    eqcode(
                        "reset dict",
                        rsc(cctx.c, ZSTD_reset_session_and_parameters),
                        rsr(cctx.r, ZSTD_reset_session_and_parameters),
                    );
                    let a = ldc(cctx.c, dp, ds, dlm, dct);
                    let b = ldr(cctx.r, dp, ds, dlm, dct);
                    let tag = format!("loadDictionary[{name}] dct={dct} dlm={dlm}");
                    eqcode(&tag, a, b);
                    let cap = bd(src.len()) + 64;
                    let mut oc = vec![0x9Au8; cap];
                    let mut or_ = vec![0x9Au8; cap];
                    let x = c2c(
                        cctx.c,
                        oc.as_mut_ptr() as *mut c_void,
                        cap,
                        src.as_ptr() as *const c_void,
                        src.len(),
                    );
                    let y = c2r(
                        cctx.r,
                        or_.as_mut_ptr() as *mut c_void,
                        cap,
                        src.as_ptr() as *const c_void,
                        src.len(),
                    );
                    eqcode(&format!("{tag} compress2"), x, y);
                    eqbuf(&format!("{tag} compress2 dst"), &oc, &or_);
                    if is_err(x) {
                        let n = errname(x);
                        if !seen.contains(&n) {
                            seen.push(n);
                        }
                    }

                    // (b) ZSTD_createCDict_advanced
                    let kc = mkc(dp, ds, dlm, dct, cp, ZSTD_customMem::default());
                    let kr = mkr(dp, ds, dlm, dct, cp, ZSTD_customMem::default());
                    eqv(&format!("{tag} createCDict NULL?"), kc.is_null(), kr.is_null());
                    if !kc.is_null() {
                        let bctx = CtxPair::cctx();
                        let x = bcc(bctx.c, kc);
                        let y = bcr(bctx.r, kr);
                        eqcode(&format!("{tag} compressBegin_usingCDict"), x, y);
                        fdc(kc);
                        fdr(kr);
                    }
                }
                // (b2) ZSTD_CCtx_refPrefix_advanced feeds `dictContentType`
                // straight into ZSTD_compress_insertDictionary, so it is the
                // path on which `dictionary_wrong` (rows 165/166) is observable:
                // the loadDictionary/CDict paths turn every dictionary error
                // into `memory_allocation` at ZSTD_initLocalDict (row 110)
                // because ZSTD_createCDict_advanced2 only reports NULL.
                {
                    let (rfc, rfr) = duo::<
                        unsafe extern "C" fn(*mut c_void, *const c_void, usize, c_int) -> usize,
                    >("ZSTD_CCtx_refPrefix_advanced");
                    eqcode(
                        "reset refPrefix",
                        rsc(cctx.c, ZSTD_reset_session_and_parameters),
                        rsr(cctx.r, ZSTD_reset_session_and_parameters),
                    );
                    let x = rfc(cctx.c, dp, ds, dct);
                    let y = rfr(cctx.r, dp, ds, dct);
                    eqcode(&format!("refPrefix_advanced[{name}] dct={dct}"), x, y);
                    let cap = bd(src.len()) + 64;
                    let mut oc = vec![0xBCu8; cap];
                    let mut or_ = vec![0xBCu8; cap];
                    let x = c2c(
                        cctx.c,
                        oc.as_mut_ptr() as *mut c_void,
                        cap,
                        src.as_ptr() as *const c_void,
                        src.len(),
                    );
                    let y = c2r(
                        cctx.r,
                        or_.as_mut_ptr() as *mut c_void,
                        cap,
                        src.as_ptr() as *const c_void,
                        src.len(),
                    );
                    eqcode(&format!("refPrefix_advanced[{name}] dct={dct} compress2"), x, y);
                    eqbuf(&format!("refPrefix_advanced[{name}] dct={dct} dst"), &oc, &or_);
                    if is_err(x) {
                        let n = errname(x);
                        if !seen.contains(&n) {
                            seen.push(n);
                        }
                    }
                }
                // (c) ZSTD_compressBegin_usingDict / ZSTD_compress_usingDict
                // (dct_auto only, they take no dictContentType)
                if dct == ZSTD_dct_auto {
                    let bctx = CtxPair::cctx();
                    let x = bdc(bctx.c, dp, ds, 5);
                    let y = bdr(bctx.r, dp, ds, 5);
                    eqcode(&format!("compressBegin_usingDict[{name}]"), x, y);
                    if is_err(x) {
                        let n = errname(x);
                        if !seen.contains(&n) {
                            seen.push(n);
                        }
                    }
                    let cap = bd(src.len()) + 64;
                    let mut oc = vec![0xABu8; cap];
                    let mut or_ = vec![0xABu8; cap];
                    let uctx = CtxPair::cctx();
                    let x = udc(
                        uctx.c,
                        oc.as_mut_ptr() as *mut c_void,
                        cap,
                        src.as_ptr() as *const c_void,
                        src.len(),
                        dp,
                        ds,
                        5,
                    );
                    let y = udr(
                        uctx.r,
                        or_.as_mut_ptr() as *mut c_void,
                        cap,
                        src.as_ptr() as *const c_void,
                        src.len(),
                        dp,
                        ds,
                        5,
                    );
                    eqcode(&format!("compress_usingDict[{name}]"), x, y);
                    eqbuf(&format!("compress_usingDict[{name}] dst"), &oc, &or_);
                }
            }
        }
        for want in [
            "Dictionary is corrupted",
            "Dictionary mismatch",
            "Allocation error : not enough memory",
        ] {
            assert!(
                seen.iter().any(|s| s == want),
                "dictionary loading never produced `{want}`; saw {seen:?}"
            );
        }

        // ---- row 181: ZSTD_compressBegin_usingCDict*(cctx, NULL)
        {
            let (b1c, b1r) = duo::<FnCompressBeginUsingCDict>("ZSTD_compressBegin_usingCDict");
            let (b2c, b2r) = duo::<FnCompressBeginUsingCDict>("ZSTD_compressBegin_usingCDict_deprecated");
            let (b3c, b3r) = duo::<
                unsafe extern "C" fn(
                    *mut c_void,
                    *const c_void,
                    ZSTD_frameParameters,
                    c_ulonglong,
                ) -> usize,
            >("ZSTD_compressBegin_usingCDict_advanced");
            let bctx = CtxPair::cctx();
            let x = b1c(bctx.c, ptr::null());
            let y = b1r(bctx.r, ptr::null());
            eqcode("compressBegin_usingCDict(NULL)", x, y);
            expect_code("compressBegin_usingCDict(NULL)", x, "Dictionary mismatch");
            let x = b2c(bctx.c, ptr::null());
            let y = b2r(bctx.r, ptr::null());
            eqcode("compressBegin_usingCDict_deprecated(NULL)", x, y);
            expect_code("compressBegin_usingCDict_deprecated(NULL)", x, "Dictionary mismatch");
            for fp in [
                ZSTD_frameParameters { contentSizeFlag: 1, checksumFlag: 0, noDictIDFlag: 0 },
                ZSTD_frameParameters { contentSizeFlag: 0, checksumFlag: 1, noDictIDFlag: 1 },
            ] {
                for pledged in [0u64, 1000, ZSTD_CONTENTSIZE_UNKNOWN] {
                    let x = b3c(bctx.c, ptr::null(), fp, pledged);
                    let y = b3r(bctx.r, ptr::null(), fp, pledged);
                    eqcode(&format!("compressBegin_usingCDict_advanced(NULL,{pledged})"), x, y);
                    expect_code(
                        &format!("compressBegin_usingCDict_advanced(NULL,{pledged})"),
                        x,
                        "Dictionary mismatch",
                    );
                }
            }
            // and ZSTD_CCtx_refCDict(NULL) must simply clear the dictionary
            let (rcc, rcr) = duo::<unsafe extern "C" fn(*mut c_void, *const c_void) -> usize>(
                "ZSTD_CCtx_refCDict",
            );
            eqcode("refCDict(NULL)", rcc(bctx.c, ptr::null()), rcr(bctx.r, ptr::null()));
        }
    }
}

// ------------------------------------------------------------------ rows 220-229
//
// The block-writing internals, called directly with destinations that are one
// byte too small at each step.

const HUF_WORKSPACE_SIZE: usize = (8 << 10) + 512;
const LLFSELog: u32 = 9;
const MLFSELog: u32 = 9;
const OffFSELog: u32 = 8;
const MaxLL: u32 = 35;
const MaxML: u32 = 52;
const MaxOff: u32 = 31;
const set_basic: c_int = 0;
const set_rle: c_int = 1;
const set_compressed: c_int = 2;
const set_repeat: c_int = 3;

const fn fse_ctable_size_u32(tableLog: u32, maxSymbolValue: u32) -> usize {
    1 + (1usize << (tableLog - 1)) + ((maxSymbolValue as usize + 1) * 2)
}

#[repr(C)]
#[derive(Clone, Copy)]
struct ZSTD_hufCTables_t {
    CTable: [u64; 257],
    repeatMode: c_int,
}

type FnCompressLiterals = unsafe extern "C" fn(
    *mut c_void,
    usize,
    *const c_void,
    usize,
    *mut c_void,
    usize,
    *const ZSTD_hufCTables_t,
    *mut ZSTD_hufCTables_t,
    c_int,
    c_int,
    c_int,
    c_int,
) -> usize;
type FnZstdBuildCTable = unsafe extern "C" fn(
    *mut c_void,
    usize,
    *mut u32,
    u32,
    c_int,
    *mut c_uint,
    u32,
    *const u8,
    usize,
    *const i16,
    u32,
    u32,
    *const u32,
    usize,
    *mut c_void,
    usize,
) -> usize;
type FnEncodeSequences = unsafe extern "C" fn(
    *mut c_void,
    usize,
    *const u32,
    *const u8,
    *const u32,
    *const u8,
    *const u32,
    *const u8,
    *const SeqDefRaw,
    usize,
    c_int,
    c_int,
) -> usize;
type FnFseBitCost = unsafe extern "C" fn(*const u32, *const c_uint, c_uint) -> usize;
type FnBuildCTableRle = unsafe extern "C" fn(*mut u32, u8) -> usize;
type FnNormalizeCount =
    unsafe extern "C" fn(*mut i16, c_uint, *const c_uint, usize, c_uint, c_uint) -> usize;
type FnBuildCTableWksp =
    unsafe extern "C" fn(*mut u32, *const i16, c_uint, c_uint, *mut c_void, usize) -> usize;

#[test]
fn err_block_internals_dst_too_small() {
    unsafe {
        let (ncc, ncr) = duo::<FnNoCompressLiterals>("ZSTD_noCompressLiterals");
        let (rlc, rlr) = duo::<FnNoCompressLiterals>("ZSTD_compressRleLiteralsBlock");
        let (clc, clr) = duo::<FnCompressLiterals>("ZSTD_compressLiterals");
        let (btc, btr) = duo::<FnZstdBuildCTable>("ZSTD_buildCTable");
        let (esc, esr) = duo::<FnEncodeSequences>("ZSTD_encodeSequences");
        let (fbc, fbr) = duo::<FnFseBitCost>("ZSTD_fseBitCost");
        let (rlec, _) = duo::<FnBuildCTableRle>("FSE_buildCTable_rle");
        let (normc, _) = duo::<FnNormalizeCount>("FSE_normalizeCount");
        let (bwc, _) = duo::<FnBuildCTableWksp>("FSE_buildCTable_wksp");

        // ---- rows 222/223 (+ the ZSTD_compressRleLiteralsBlock 4-byte guard)
        let mut saw_lit = 0usize;
        for cls in 0..N_CLASSES {
            for &sz in &[0usize, 1, 2, 5, 31, 32, 4095, 4096, 8192] {
                let src = gen_class(cls, sz, 0xC600 ^ sz as u64);
                let sp = if sz == 0 { ptr::null() } else { src.as_ptr() as *const c_void };
                let fl = 1 + (sz > 31) as usize + (sz > 4095) as usize;
                let mut caps: Vec<usize> = vec![0, 1, 2, 3, 4, 5];
                caps.push(sz + fl);
                if sz + fl > 0 {
                    caps.push(sz + fl - 1);
                }
                caps.push(sz + fl + 1);
                caps.sort();
                caps.dedup();
                for cap in caps {
                    let mut oc = vec![0xD1u8; cap + 16];
                    let mut or_ = vec![0xD1u8; cap + 16];
                    let a = ncc(oc.as_mut_ptr() as *mut c_void, cap, sp, sz);
                    let b = ncr(or_.as_mut_ptr() as *mut c_void, cap, sp, sz);
                    let tag = format!("noCompressLiterals cls={cls} sz={sz} cap={cap}");
                    eqcode(&tag, a, b);
                    eqbuf(&format!("{tag} dst"), &oc, &or_);
                    if is_err(a) {
                        expect_code(&tag, a, "Destination buffer is too small");
                        saw_lit += 1;
                    }
                    // ZSTD_compressRleLiteralsBlock's documented preconditions
                    // are "all bytes in src are identical" and dstCapacity >= 4;
                    // it also dereferences src[0] unconditionally, so srcSize
                    // must be > 0. Only class 0/1 data qualifies.
                    if cls <= 1 && cap >= 4 && sz > 0 {
                        let mut oc = vec![0xD2u8; cap + 16];
                        let mut or_ = vec![0xD2u8; cap + 16];
                        let a = rlc(oc.as_mut_ptr() as *mut c_void, cap, sp, sz);
                        let b = rlr(or_.as_mut_ptr() as *mut c_void, cap, sp, sz);
                        let tag = format!("rleLiterals cls={cls} sz={sz} cap={cap}");
                        eqcode(&tag, a, b);
                        eqbuf(&format!("{tag} dst"), &oc, &or_);
                    }
                }

                // ---- ZSTD_compressLiterals (row 223)
                for strategy in [1i32, 4, 7, 9] {
                    for disable in [0, 1] {
                        for suspect in [0, 1] {
                            for cap in [0usize, 1, 2, 3, 4, 5, 6, 8, 16, sz + 8] {
                                let mut wc = vec![0u64; HUF_WORKSPACE_SIZE / 8];
                                let mut wr = vec![0u64; HUF_WORKSPACE_SIZE / 8];
                                let prev = ZSTD_hufCTables_t { CTable: [0u64; 257], repeatMode: 0 };
                                let mut nxc = prev;
                                let mut nxr = prev;
                                let mut oc = vec![0xD3u8; cap + 16];
                                let mut or_ = vec![0xD3u8; cap + 16];
                                let a = clc(
                                    oc.as_mut_ptr() as *mut c_void,
                                    cap,
                                    sp,
                                    sz,
                                    wc.as_mut_ptr() as *mut c_void,
                                    HUF_WORKSPACE_SIZE,
                                    &prev,
                                    &mut nxc,
                                    strategy,
                                    disable,
                                    suspect,
                                    0,
                                );
                                let b = clr(
                                    or_.as_mut_ptr() as *mut c_void,
                                    cap,
                                    sp,
                                    sz,
                                    wr.as_mut_ptr() as *mut c_void,
                                    HUF_WORKSPACE_SIZE,
                                    &prev,
                                    &mut nxr,
                                    strategy,
                                    disable,
                                    suspect,
                                    0,
                                );
                                let tag = format!(
                                    "compressLiterals cls={cls} sz={sz} cap={cap} strat={strategy} dis={disable} sus={suspect}"
                                );
                                eqcode(&tag, a, b);
                                eqbuf(&format!("{tag} dst"), &oc, &or_);
                                if is_err(a) {
                                    expect_code(&tag, a, "Destination buffer is too small");
                                    saw_lit += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
        assert!(saw_lit > 0, "rows 222/223 never triggered");

        // ---- rows 226/227: ZSTD_buildCTable with dstCapacity == 0 (set_rle) and
        // with an out-of-range `type` (the `default:` GENERIC branch).
        {
            let ll_u32 = fse_ctable_size_u32(LLFSELog, MaxLL);
            let mut counts: Vec<c_uint> = vec![0; MaxLL as usize + 2];
            for i in 0..=MaxLL as usize {
                counts[i] = (i as c_uint % 7) + 1;
            }
            let codes: Vec<u8> = (0..64u8).map(|i| i % (MaxLL as u8 + 1)).collect();
            let norm: Vec<i16> = {
                let mut v = vec![0i16; MaxLL as usize + 2];
                v[0] = 4;
                for i in 1..=MaxLL as usize {
                    v[i] = 1;
                }
                v
            };
            let mut saw_rle = 0usize;
            let mut saw_generic = 0usize;
            for ty in [set_basic, set_rle, set_compressed, set_repeat, 4, 5, -1, 99] {
                for cap in [0usize, 1, 2, 4, 64] {
                    for nbseq in [2usize, 8, 64] {
                        let mut ctc = vec![0u32; ll_u32 + 64];
                        let mut ctr = vec![0u32; ll_u32 + 64];
                        let prevc = vec![0u32; ll_u32 + 64];
                        let mut wc = vec![0u64; HUF_WORKSPACE_SIZE / 8];
                        let mut wr = vec![0u64; HUF_WORKSPACE_SIZE / 8];
                        let mut cc = counts.clone();
                        let mut cr = counts.clone();
                        let mut oc = vec![0xD4u8; cap + 16];
                        let mut or_ = vec![0xD4u8; cap + 16];
                        let a = btc(
                            oc.as_mut_ptr() as *mut c_void,
                            cap,
                            ctc.as_mut_ptr(),
                            LLFSELog,
                            ty,
                            cc.as_mut_ptr(),
                            MaxLL,
                            codes.as_ptr(),
                            nbseq,
                            norm.as_ptr(),
                            5,
                            MaxLL,
                            prevc.as_ptr(),
                            (ll_u32 + 64) * 4,
                            wc.as_mut_ptr() as *mut c_void,
                            HUF_WORKSPACE_SIZE,
                        );
                        let b = btr(
                            or_.as_mut_ptr() as *mut c_void,
                            cap,
                            ctr.as_mut_ptr(),
                            LLFSELog,
                            ty,
                            cr.as_mut_ptr(),
                            MaxLL,
                            codes.as_ptr(),
                            nbseq,
                            norm.as_ptr(),
                            5,
                            MaxLL,
                            prevc.as_ptr(),
                            (ll_u32 + 64) * 4,
                            wr.as_mut_ptr() as *mut c_void,
                            HUF_WORKSPACE_SIZE,
                        );
                        let tag = format!("buildCTable ty={ty} cap={cap} nbseq={nbseq}");
                        eqcode(&tag, a, b);
                        eqbuf(&format!("{tag} dst"), &oc, &or_);
                        eqv(&format!("{tag} counts"), &cc[..], &cr[..]);
                        eqv(&format!("{tag} ctable"), &ctc[..], &ctr[..]);
                        if is_err(a) {
                            match errname(a).as_str() {
                                "Destination buffer is too small" => saw_rle += 1,
                                "Error (generic)" => saw_generic += 1,
                                _ => {}
                            }
                        }
                    }
                }
            }
            assert!(saw_rle > 0, "row 226 (buildCTable set_rle dstSize_tooSmall) never triggered");
            assert!(saw_generic > 0, "row 227 (buildCTable GENERIC) never triggered");
        }

        // ---- rows 228/229: ZSTD_encodeSequences into a too-small destination.
        // Minimal but valid CTables: FSE_buildCTable_rle() gives a single-symbol
        // table, and every code in the three code tables is that symbol.
        {
            let n_u32 = fse_ctable_size_u32(MLFSELog, MaxML) + 64;
            let mut llct = vec![0u32; n_u32];
            let mut mlct = vec![0u32; n_u32];
            let mut ofct = vec![0u32; n_u32];
            assert_eq!(rlec(llct.as_mut_ptr(), 0), 0);
            assert_eq!(rlec(mlct.as_mut_ptr(), 0), 0);
            assert_eq!(rlec(ofct.as_mut_ptr(), 0), 0);
            let mut saw_init = 0usize;
            let mut saw_close = 0usize;
            for &nbseq in &[1usize, 2, 8, 64, 512, 4096] {
                let codes = vec![0u8; nbseq];
                let seqs: Vec<SeqDefRaw> = (0..nbseq)
                    .map(|_| SeqDefRaw { offBase: 1, litLength: 0, mlBase: 0 })
                    .collect();
                for cap in [0usize, 1, 2, 3, 4, 7, 8, 9, 12, 16, 32, 64, 1024] {
                    for longoff in [0, 1] {
                        let mut oc = vec![0xD5u8; cap + 32];
                        let mut or_ = vec![0xD5u8; cap + 32];
                        let a = esc(
                            oc.as_mut_ptr() as *mut c_void,
                            cap,
                            mlct.as_ptr(),
                            codes.as_ptr(),
                            ofct.as_ptr(),
                            codes.as_ptr(),
                            llct.as_ptr(),
                            codes.as_ptr(),
                            seqs.as_ptr(),
                            nbseq,
                            longoff,
                            0,
                        );
                        let b = esr(
                            or_.as_mut_ptr() as *mut c_void,
                            cap,
                            mlct.as_ptr(),
                            codes.as_ptr(),
                            ofct.as_ptr(),
                            codes.as_ptr(),
                            llct.as_ptr(),
                            codes.as_ptr(),
                            seqs.as_ptr(),
                            nbseq,
                            longoff,
                            0,
                        );
                        let tag = format!("encodeSequences nbseq={nbseq} cap={cap} lo={longoff}");
                        eqcode(&tag, a, b);
                        eqbuf(&format!("{tag} dst"), &oc, &or_);
                        if is_err(a) {
                            expect_code(&tag, a, "Destination buffer is too small");
                            if cap < 8 {
                                saw_init += 1;
                            } else {
                                saw_close += 1;
                            }
                        }
                    }
                }
            }
            assert!(saw_init > 0, "row 228 (BIT_initCStream) never triggered");
            assert!(saw_close > 0, "row 229 (BIT_closeCStream == 0) never triggered");
        }

        // ---- rows 224/225: ZSTD_fseBitCost GENERIC
        {
            let n_u32 = fse_ctable_size_u32(6, 8) + 64;
            // (a) the repeat table's maxSymbolValue is below `max`
            let mut ct = vec![0u32; n_u32];
            assert_eq!(rlec(ct.as_mut_ptr(), 3), 0);
            let counts: Vec<c_uint> = (0..64).map(|i| (i as c_uint % 5) + 1).collect();
            let mut saw_maxsym = 0usize;
            for max in 0..12u32 {
                let a = fbc(ct.as_ptr(), counts.as_ptr(), max);
                let b = fbr(ct.as_ptr(), counts.as_ptr(), max);
                let tag = format!("fseBitCost(rle sym=3, max={max})");
                eqcode(&tag, a, b);
                if is_err(a) && errname(a) == "Error (generic)" {
                    saw_maxsym += 1;
                }
            }
            assert!(saw_maxsym > 0, "row 224 (fseBitCost maxSymbolValue) never triggered");

            // (b) a symbol whose normalized probability is 0 but whose count is not
            let maxsym = 5u32;
            let mut hist: Vec<c_uint> = vec![10, 20, 0, 30, 15, 25];
            let total: usize = hist.iter().map(|&x| x as usize).sum();
            let mut norm = vec![0i16; maxsym as usize + 2];
            let nres = normc(
                norm.as_mut_ptr(),
                5,
                hist.as_ptr(),
                total,
                maxsym,
                0,
            );
            assert!(!is_err(nres), "FSE_normalizeCount failed: {}", errname(nres));
            assert_eq!(norm[2], 0, "symbol 2 must normalize to probability 0");
            let mut ct2 = vec![0u32; fse_ctable_size_u32(5, maxsym) + 64];
            let mut wksp = vec![0u64; HUF_WORKSPACE_SIZE / 8];
            let bres = bwc(
                ct2.as_mut_ptr(),
                norm.as_ptr(),
                maxsym,
                5,
                wksp.as_mut_ptr() as *mut c_void,
                HUF_WORKSPACE_SIZE,
            );
            assert!(!is_err(bres), "FSE_buildCTable_wksp failed");
            // now claim symbol 2 does occur
            hist[2] = 7;
            let a = fbc(ct2.as_ptr(), hist.as_ptr(), maxsym);
            let b = fbr(ct2.as_ptr(), hist.as_ptr(), maxsym);
            eqcode("fseBitCost(zero-prob symbol)", a, b);
            expect_code("fseBitCost(zero-prob symbol)", a, "Error (generic)");
            // and the same table with the honest histogram must NOT error
            hist[2] = 0;
            let a = fbc(ct2.as_ptr(), hist.as_ptr(), maxsym);
            let b = fbr(ct2.as_ptr(), hist.as_ptr(), maxsym);
            eqcode("fseBitCost(honest histogram)", a, b);
            assert!(!is_err(a), "honest fseBitCost should succeed");
        }
    }
}

// ------------------------------------------------------------------ rows 128, 142, 230
//
// The super-block path (`ZSTD_c_targetCBlockSize`) with destinations that are
// too small, which is where `zstd_compress_superblock.c:181` and
// `ZSTD_compressBlock_targetCBlockSize_body` reject.

#[test]
fn err_superblock_dst_too_small() {
    unsafe {
        let (c2c, c2r) = duo::<FnCompress2>("ZSTD_compress2");
        let (spc, spr) = duo::<FnSetParam>("ZSTD_CCtx_setParameter");
        let (rsc, rsr) = duo::<FnReset>("ZSTD_CCtx_reset");
        let (bd, _) = duo::<FnSizeT1>("ZSTD_compressBound");
        let cctx = CtxPair::cctx();

        let mut saw = 0usize;
        let mut oob = 0usize;
        for tcbs in [1340, 2048, 4096, 65536, 131072] {
            for cls in 0..N_CLASSES {
                for &sz in &[7usize, 1300, 70_000, 300_000] {
                    let src = gen_class(cls, sz, 0xC700 ^ sz as u64);
                    // reference size
                    let full = bd(sz) + 4096;
                    let mut probe = vec![0u8; full];
                    eqcode(
                        "reset superblock probe",
                        rsc(cctx.c, ZSTD_reset_session_and_parameters),
                        rsr(cctx.r, ZSTD_reset_session_and_parameters),
                    );
                    for (p, v) in [(ZSTD_c_targetCBlockSize, tcbs), (ZSTD_c_compressionLevel, 3)] {
                        eqcode(
                            &format!("superblock set({p},{v})"),
                            spc(cctx.c, p, v),
                            spr(cctx.r, p, v),
                        );
                    }
                    let exact = c2c(
                        cctx.c,
                        probe.as_mut_ptr() as *mut c_void,
                        full,
                        src.as_ptr() as *const c_void,
                        sz,
                    );
                    assert!(!is_err(exact), "superblock reference failed: {}", errname(exact));
                    let mut caps = caps_for(exact);
                    for extra in [exact * 3 / 4, exact * 9 / 10] {
                        caps.push(extra);
                    }
                    caps.sort();
                    caps.dedup();
                    for cap in caps {
                        eqcode(
                            "reset superblock",
                            rsc(cctx.c, ZSTD_reset_session_and_parameters),
                            rsr(cctx.r, ZSTD_reset_session_and_parameters),
                        );
                        for (p, v) in
                            [(ZSTD_c_targetCBlockSize, tcbs), (ZSTD_c_compressionLevel, 3)]
                        {
                            eqcode(
                                &format!("superblock set({p},{v})"),
                                spc(cctx.c, p, v),
                                spr(cctx.r, p, v),
                            );
                        }
                        // UPSTREAM C OUT-OF-BOUNDS WRITE (avoided, not
                        // re-litigated): with ZSTD_c_targetCBlockSize set, the
                        // super-block writer is explicitly documented as "not
                        // bound by the standard ZSTD_compressBound()"
                        // (zstd_compress.c:4470-4479) and, for some
                        // (targetCBlockSize, srcSize, dstCapacity) triples, it
                        // writes *past* `dst + dstCapacity` before any of the
                        // dstSize_tooSmall guards fires. Reproduced with
                        // targetCBlockSize=1340, class-4 input of 1300 bytes and
                        // dstCapacity == 792: an unguarded heap buffer is
                        // corrupted and the process dies on the next allocation.
                        // The Rust port performs the same pointer arithmetic, so
                        // both write the same bytes to the same offsets; the row
                        // stays differential by giving both destinations a 64 KiB
                        // canary guard band on each side and comparing the whole
                        // padded region.
                        const GUARD: usize = 64 * 1024;
                        let mut oc = vec![0xE7u8; GUARD + cap + GUARD];
                        let mut or_ = vec![0xE7u8; GUARD + cap + GUARD];
                        let a = c2c(
                            cctx.c,
                            oc.as_mut_ptr().add(GUARD) as *mut c_void,
                            cap,
                            src.as_ptr() as *const c_void,
                            sz,
                        );
                        let b = c2r(
                            cctx.r,
                            or_.as_mut_ptr().add(GUARD) as *mut c_void,
                            cap,
                            src.as_ptr() as *const c_void,
                            sz,
                        );
                        let tag = format!("superblock tcbs={tcbs} cls={cls} sz={sz} cap={cap}");
                        eqcode(&tag, a, b);
                        eqbuf(&format!("{tag} padded dst"), &oc, &or_);
                        if is_err(a) {
                            saw += 1;
                        }
                        // evidence for the out-of-bounds write described above
                        if oc[..GUARD].iter().any(|&x| x != 0xE7)
                            || oc[GUARD + cap..].iter().any(|&x| x != 0xE7)
                        {
                            oob += 1;
                        }
                    }
                }
            }
        }
        assert!(saw > 0, "rows 128/142/230 never triggered");
        // Both libraries scribbled outside `dst` in exactly the same places
        // (the eqbuf above compares the whole padded region); the counter is
        // kept so the upstream defect stays visible instead of silently
        // disappearing into a guard band.
        assert!(
            oob > 0,
            "the documented super-block out-of-bounds write did not reproduce; \
             if upstream fixed it, drop this assertion"
        );
    }
}

// ------------------------------------------------------------------ rows 118, 119
//
// `params->nbWorkers > 0` => GENERIC. In this single-threaded build
// `ZSTD_CCtxParams_setParameter(ZSTD_c_nbWorkers, n!=0)` is rejected with
// parameter_unsupported, and no `ZSTDMT_CCtxParam_setNbWorkers` is exported, so
// the only way to reach the check is to hand the estimators a
// `ZSTD_CCtx_params` whose `nbWorkers` field is non-zero. The struct is
// re-declared here and the layout is *verified* byte-for-byte against a
// library-allocated, library-initialised object before it is used.

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
struct LdmParamsRaw {
    enableLdm: c_int,
    hashLog: c_uint,
    bucketSizeLog: c_uint,
    minMatchLength: c_uint,
    hashRateLog: c_uint,
    windowLog: c_uint,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct CCtxParamsRaw {
    format: c_int,
    cParams: ZSTD_compressionParameters,
    fParams: ZSTD_frameParameters,
    compressionLevel: c_int,
    forceWindow: c_int,
    targetCBlockSize: usize,
    srcSizeHint: c_int,
    attachDictPref: c_int,
    literalCompressionMode: c_int,
    nbWorkers: c_int,
    jobSize: usize,
    overlapLog: c_int,
    rsyncable: c_int,
    ldmParams: LdmParamsRaw,
    enableDedicatedDictSearch: c_int,
    inBufferMode: c_int,
    outBufferMode: c_int,
    blockDelimiters: c_int,
    validateSequences: c_int,
    postBlockSplitter: c_int,
    preBlockSplitter_level: c_int,
    maxBlockSize: usize,
    useRowMatchFinder: c_int,
    deterministicRefPrefix: c_int,
    customMem: ZSTD_customMem,
    prefetchCDictTables: c_int,
    enableMatchFinderFallback: c_int,
    extSeqProdState: *mut c_void,
    extSeqProdFunc: Option<FnSeqProducer>,
    searchForExternalRepcodes: c_int,
}

#[test]
fn err_estimate_size_nbworkers_generic() {
    unsafe {
        let (ic, ir) = duo::<unsafe extern "C" fn(*mut c_void, c_int) -> usize>("ZSTD_CCtxParams_init");
        let (ec, er) = duo::<FnEstimateFromParams>("ZSTD_estimateCCtxSize_usingCCtxParams");
        let (sc, sr) = duo::<FnEstimateFromParams>("ZSTD_estimateCStreamSize_usingCCtxParams");
        let (setp_c, setp_r) = duo::<FnSetParam>("ZSTD_CCtxParams_setParameter");

        // --- layout check: our struct, initialised by the library, must be
        // byte-identical to a library-allocated object of the same size.
        let n = std::mem::size_of::<CCtxParamsRaw>();
        let reference = CtxPair::cctx_params();
        for (init, obj, who) in [(ic, reference.c, "C"), (ir, reference.r, "Rust")] {
            let mut mine = vec![0u8; n];
            let r = init(mine.as_mut_ptr() as *mut c_void, 3);
            assert!(!is_err(r), "{who}: CCtxParams_init on our struct failed");
            let r = init(obj, 3);
            assert!(!is_err(r), "{who}: CCtxParams_init on the library object failed");
            let theirs = std::slice::from_raw_parts(obj as *const u8, n);
            assert_eq!(
                &mine[..],
                theirs,
                "{who}: the locally declared ZSTD_CCtx_params layout does not match \
                 the library's (size {n}); the nbWorkers probe below would be invalid"
            );
        }

        // --- rows 118/119
        for nb in [1i32, 2, 4, 16, i32::MAX] {
            for ldm in [false, true] {
                for lvl in [1, 3, 19] {
                    let mut pc = vec![0u8; n];
                    let mut pr = vec![0u8; n];
                    assert!(!is_err(ic(pc.as_mut_ptr() as *mut c_void, lvl)));
                    assert!(!is_err(ir(pr.as_mut_ptr() as *mut c_void, lvl)));
                    if ldm {
                        // ZSTD_c_ldmMinMatch MUST be set whenever LDM is enabled
                        // on a bare params object: ZSTD_ldm_getMaxNbSeq() divides
                        // by minMatchLength and both libraries die with SIGFPE at
                        // 0 (CONFIGS.md X1).
                        for (p, v) in [
                            (ZSTD_c_enableLongDistanceMatching, ZSTD_ps_enable),
                            (ZSTD_c_ldmMinMatch, 64),
                            (ZSTD_c_ldmHashLog, 20),
                            (ZSTD_c_ldmBucketSizeLog, 3),
                            (ZSTD_c_ldmHashRateLog, 7),
                        ] {
                            eqcode(
                                &format!("raw params set({p},{v})"),
                                setp_c(pc.as_mut_ptr() as *mut c_void, p, v),
                                setp_r(pr.as_mut_ptr() as *mut c_void, p, v),
                            );
                        }
                    }
                    // nbWorkers is only reachable by writing the field directly
                    (*(pc.as_mut_ptr() as *mut CCtxParamsRaw)).nbWorkers = nb;
                    (*(pr.as_mut_ptr() as *mut CCtxParamsRaw)).nbWorkers = nb;
                    let a = ec(pc.as_ptr() as *const c_void);
                    let b = er(pr.as_ptr() as *const c_void);
                    let tag = format!("estimateCCtxSize_usingCCtxParams(nbWorkers={nb},ldm={ldm},lvl={lvl})");
                    eqcode(&tag, a, b);
                    expect_code(&tag, a, "Error (generic)");
                    let a = sc(pc.as_ptr() as *const c_void);
                    let b = sr(pr.as_ptr() as *const c_void);
                    let tag = format!("estimateCStreamSize_usingCCtxParams(nbWorkers={nb},ldm={ldm},lvl={lvl})");
                    eqcode(&tag, a, b);
                    expect_code(&tag, a, "Error (generic)");
                    // and nbWorkers == 0 must succeed identically
                    (*(pc.as_mut_ptr() as *mut CCtxParamsRaw)).nbWorkers = 0;
                    (*(pr.as_mut_ptr() as *mut CCtxParamsRaw)).nbWorkers = 0;
                    eqcode(
                        &format!("estimateCCtxSize(nbWorkers=0,ldm={ldm},lvl={lvl})"),
                        ec(pc.as_ptr() as *const c_void),
                        er(pr.as_ptr() as *const c_void),
                    );
                    eqcode(
                        &format!("estimateCStreamSize(nbWorkers=0,ldm={ldm},lvl={lvl})"),
                        sc(pc.as_ptr() as *const c_void),
                        sr(pr.as_ptr() as *const c_void),
                    );
                    // negative nbWorkers must NOT trip the check
                    (*(pc.as_mut_ptr() as *mut CCtxParamsRaw)).nbWorkers = -1;
                    (*(pr.as_mut_ptr() as *mut CCtxParamsRaw)).nbWorkers = -1;
                    eqcode(
                        &format!("estimateCCtxSize(nbWorkers=-1,ldm={ldm},lvl={lvl})"),
                        ec(pc.as_ptr() as *const c_void),
                        er(pr.as_ptr() as *const c_void),
                    );
                }
            }
        }
    }
}

// ------------------------------------------------------------------ rows 203, 205
//
// `ZSTD_compressSequences` with a destination too small for the empty-frame
// block header (row 203) and for the frame checksum (row 205).

#[test]
fn err_compress_sequences_dst_too_small() {
    unsafe {
        let (csc, csr) = duo::<FnCompressSeq>("ZSTD_compressSequences");
        let (spc, spr) = duo::<FnSetParam>("ZSTD_CCtx_setParameter");
        let (rsc, rsr) = duo::<FnReset>("ZSTD_CCtx_reset");
        let (bd, _) = duo::<FnSizeT1>("ZSTD_compressBound");
        let cctx = CtxPair::cctx();

        const PAD: usize = 64;
        // UPSTREAM C OUT-OF-BOUNDS WRITE (CONFIGS.md X2): with a too-small
        // dstCapacity, ZSTD_compressSequences writes ~70 bytes BELOW `dst`
        // before returning dstSize_tooSmall. Both libraries do the same, so the
        // row stays differential behind a 64 KiB canary guard band on each side.
        const GUARD: usize = 64 * 1024;

        let mut saw_empty = 0usize;
        let mut saw_checksum = 0usize;
        for &ssz in &[0usize, 40, 4096] {
            let mut src = gen_class(4, ssz, 101);
            src.extend_from_slice(&[0u8; PAD]);
            let sptr = if ssz == 0 { ptr::null() } else { src.as_ptr() as *const c_void };
            let seqs: Vec<ZSTD_Sequence> = if ssz == 0 {
                vec![ZSTD_Sequence { offset: 0, litLength: 0, matchLength: 0, rep: 0 }]
            } else if ssz < 64 {
                synth_seqs(0, 0, 0, ssz)
            } else {
                synth_seqs(ssz / 16, 4, 12, ssz)
            };
            // reference size with checksum on
            eqcode(
                "reset cs probe",
                rsc(cctx.c, ZSTD_reset_session_and_parameters),
                rsr(cctx.r, ZSTD_reset_session_and_parameters),
            );
            for (p, v) in [(ZSTD_c_blockDelimiters, 1), (ZSTD_c_checksumFlag, 1)] {
                eqcode(
                    &format!("cs probe set({p},{v})"),
                    spc(cctx.c, p, v),
                    spr(cctx.r, p, v),
                );
            }
            let full = bd(ssz) + 4096;
            let mut probe = vec![0u8; full];
            let exact = csc(
                cctx.c,
                probe.as_mut_ptr() as *mut c_void,
                full,
                seqs.as_ptr(),
                seqs.len(),
                sptr,
                ssz,
            );
            assert!(!is_err(exact), "cs reference failed: {}", errname(exact));

            for cks in [0, 1] {
                let mut caps: Vec<usize> = vec![0, 1, 2, 3, 4, 5, 8, 12, 17, 18, 19, 20];
                for k in 1..=4usize {
                    if exact >= k {
                        caps.push(exact - k);
                    }
                }
                caps.push(exact);
                caps.sort();
                caps.dedup();
                for cap in caps {
                    eqcode(
                        "reset cs",
                        rsc(cctx.c, ZSTD_reset_session_and_parameters),
                        rsr(cctx.r, ZSTD_reset_session_and_parameters),
                    );
                    for (p, v) in [(ZSTD_c_blockDelimiters, 1), (ZSTD_c_checksumFlag, cks)] {
                        eqcode(
                            &format!("cs set({p},{v})"),
                            spc(cctx.c, p, v),
                            spr(cctx.r, p, v),
                        );
                    }
                    let mut q1 = vec![0xABu8; GUARD + cap + GUARD];
                    let mut q2 = vec![0xABu8; GUARD + cap + GUARD];
                    let a = csc(
                        cctx.c,
                        q1.as_mut_ptr().add(GUARD) as *mut c_void,
                        cap,
                        seqs.as_ptr(),
                        seqs.len(),
                        sptr,
                        ssz,
                    );
                    let b = csr(
                        cctx.r,
                        q2.as_mut_ptr().add(GUARD) as *mut c_void,
                        cap,
                        seqs.as_ptr(),
                        seqs.len(),
                        sptr,
                        ssz,
                    );
                    let tag = format!("compressSequences ssz={ssz} cks={cks} cap={cap} exact={exact}");
                    eqcode(&tag, a, b);
                    eqbuf(&format!("{tag} padded dst"), &q1, &q2);
                    if is_err(a) && errname(a) == "Destination buffer is too small" {
                        if ssz == 0 {
                            saw_empty += 1;
                        }
                        if cks == 1 && cap >= 18 {
                            saw_checksum += 1;
                        }
                    }
                }
            }
        }
        // Row 203 (`dstCapacity<4` for the empty-frame block header inside
        // ZSTD_compressSequences_internal) is UNREACHABLE from this entry point:
        // ZSTD_writeFrameHeader runs first and already demands
        // dstCapacity >= ZSTD_FRAMEHEADERSIZE_MAX (18), after which the
        // remaining capacity is always >= 4. The counter is kept to document
        // that the shape was exercised.
        let _ = saw_empty;
        assert!(saw_checksum > 0, "row 205 (no room for checksum) never triggered");
    }
}
