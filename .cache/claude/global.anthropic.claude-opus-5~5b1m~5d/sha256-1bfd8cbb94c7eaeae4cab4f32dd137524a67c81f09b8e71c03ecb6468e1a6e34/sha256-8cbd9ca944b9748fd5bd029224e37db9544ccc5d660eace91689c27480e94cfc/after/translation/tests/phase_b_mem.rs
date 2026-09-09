//! Phase B: memory sizing, static allocation, custom allocators and frame
//! progression.
//!
//! Every test drives BOTH the C `libzstd.so` (ground truth) and the Rust
//! `libzstd.so` and asserts identical return values / byte-identical output.
//!
//! CONFIGS 162-167; ERRORS 23-30, 78, 89, 90, 99-105, 210-224, 248, 260, 261,
//! 267, 273.
#![allow(non_snake_case, dead_code)]
mod common;
use common::*;
use std::os::raw::{c_int, c_uint, c_ulonglong, c_void};

// ------------------------------------------------------------ fn types -----

type FnNew = unsafe extern "C" fn() -> *mut c_void;
type FnFree = unsafe extern "C" fn(*mut c_void) -> usize;
type FnSizeof = unsafe extern "C" fn(*const c_void) -> usize;
type FnSetParam = unsafe extern "C" fn(*mut c_void, c_int, c_int) -> usize;
type FnReset = unsafe extern "C" fn(*mut c_void, c_int) -> usize;
type FnBound = unsafe extern "C" fn(usize) -> usize;
type FnCompress = unsafe extern "C" fn(*mut u8, usize, *const u8, usize, c_int) -> usize;
type FnCompressCCtx =
    unsafe extern "C" fn(*mut c_void, *mut u8, usize, *const u8, usize, c_int) -> usize;
type FnDecompressDCtx =
    unsafe extern "C" fn(*mut c_void, *mut u8, usize, *const u8, usize) -> usize;
type FnDecompress = unsafe extern "C" fn(*mut u8, usize, *const u8, usize) -> usize;
type FnCompressStream2 =
    unsafe extern "C" fn(*mut c_void, *mut OutBuffer, *mut InBuffer, c_int) -> usize;
type FnDecompressStream =
    unsafe extern "C" fn(*mut c_void, *mut OutBuffer, *mut InBuffer) -> usize;
type FnInitCStream = unsafe extern "C" fn(*mut c_void, c_int) -> usize;
type FnInitDStream = unsafe extern "C" fn(*mut c_void) -> usize;
type FnLoadDict = unsafe extern "C" fn(*mut c_void, *const u8, usize) -> usize;

type FnEstimateLevel = unsafe extern "C" fn(c_int) -> usize;
type FnEstimateVoid = unsafe extern "C" fn() -> usize;
type FnEstimateCParams = unsafe extern "C" fn(CParams) -> usize;
type FnEstimateParamsPtr = unsafe extern "C" fn(*const c_void) -> usize;
type FnEstimateSize = unsafe extern "C" fn(usize) -> usize;
type FnEstimateFromFrame = unsafe extern "C" fn(*const u8, usize) -> usize;
type FnEstimateCDict = unsafe extern "C" fn(usize, c_int) -> usize;
type FnEstimateCDictAdv = unsafe extern "C" fn(usize, CParams, c_int) -> usize;
type FnEstimateDDict = unsafe extern "C" fn(usize, c_int) -> usize;
type FnGetCParams = unsafe extern "C" fn(c_int, c_ulonglong, usize) -> CParams;

type FnInitStatic = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
type FnInitStaticCDict = unsafe extern "C" fn(
    *mut c_void,
    usize,
    *const u8,
    usize,
    c_int,
    c_int,
    CParams,
) -> *mut c_void;
type FnInitStaticDDict =
    unsafe extern "C" fn(*mut c_void, usize, *const u8, usize, c_int, c_int) -> *mut c_void;

type FnCreateAdvanced = unsafe extern "C" fn(CustomMem) -> *mut c_void;
type FnCreateCDictAdvanced =
    unsafe extern "C" fn(*const u8, usize, c_int, c_int, CParams, CustomMem) -> *mut c_void;
type FnCreateDDictAdvanced =
    unsafe extern "C" fn(*const u8, usize, c_int, c_int, CustomMem) -> *mut c_void;

type FnRefCDict = unsafe extern "C" fn(*mut c_void, *const c_void) -> usize;
type FnRefDDict = unsafe extern "C" fn(*mut c_void, *const c_void) -> usize;
type FnCompressUsingCDict = unsafe extern "C" fn(
    *mut c_void,
    *mut u8,
    usize,
    *const u8,
    usize,
    *const c_void,
) -> usize;
type FnDecompressUsingDDict = unsafe extern "C" fn(
    *mut c_void,
    *mut u8,
    usize,
    *const u8,
    usize,
    *const c_void,
) -> usize;

type FnFrameProgression = unsafe extern "C" fn(*const c_void) -> FrameProgression;
type FnToFlushNow = unsafe extern "C" fn(*mut c_void) -> usize;
type FnGet1BlockSummary = unsafe extern "C" fn(*const Sequence, usize) -> BlockSummary;

// ------------------------------------------------------------- structs -----

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FrameHeader {
    pub frameContentSize: c_ulonglong,
    pub windowSize: c_ulonglong,
    pub blockSizeMax: c_uint,
    pub frameType: c_int,
    pub headerSize: c_uint,
    pub dictID: c_uint,
    pub checksumFlag: c_uint,
    pub _r1: c_uint,
    pub _r2: c_uint,
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

/// ZSTD_frameProgression (see zstd.h).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FrameProgression {
    pub ingested: u64,
    pub consumed: u64,
    pub produced: u64,
    pub flushed: u64,
    pub currentJobID: c_uint,
    pub nbActiveWorkers: c_uint,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Sequence {
    pub offset: u32,
    pub litLength: u32,
    pub matchLength: u32,
    pub rep: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BlockSummary {
    pub nbSequences: usize,
    pub blockSize: usize,
    pub litSize: usize,
}

pub type AllocFn = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
pub type FreeFn = unsafe extern "C" fn(*mut c_void, *mut c_void);

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CustomMem {
    pub customAlloc: Option<AllocFn>,
    pub customFree: Option<FreeFn>,
    pub opaque: *mut c_void,
}

impl CustomMem {
    fn none() -> CustomMem {
        CustomMem { customAlloc: None, customFree: None, opaque: std::ptr::null_mut() }
    }
}

// ----------------------------------------------------------- constants -----

const C_COMPRESSIONLEVEL: c_int = 100;
const C_WINDOWLOG: c_int = 101;
const C_CHECKSUMFLAG: c_int = 201;
const C_NBWORKERS: c_int = 400;
const D_REFMULTIPLEDDICTS: c_int = 1003;

const RESET_SESSION_ONLY: c_int = 1;
const RESET_PARAMETERS: c_int = 2;
const RESET_SESSION_AND_PARAMETERS: c_int = 3;

const ZSTD_E_CONTINUE: c_int = 0;
const ZSTD_E_FLUSH: c_int = 1;
const ZSTD_E_END: c_int = 2;

const DLM_BY_COPY: c_int = 0;
const DLM_BY_REF: c_int = 1;
const DCT_AUTO: c_int = 0;
const DCT_RAWCONTENT: c_int = 1;
const DCT_FULLDICT: c_int = 2;

const ZSTD_MAGICNUMBER: u32 = 0xFD2FB528;
const ZSTD_MAGIC_DICTIONARY: u32 = 0xEC30A437;

// ------------------------------------------------- custom allocator --------

use std::sync::atomic::{AtomicUsize, Ordering};

static ALLOC_CALLS: AtomicUsize = AtomicUsize::new(0);
static FREE_CALLS: AtomicUsize = AtomicUsize::new(0);
static LIVE_BYTES: AtomicUsize = AtomicUsize::new(0);

const HDR: usize = 16;

/// A real working custom allocator, implemented in Rust and handed to both
/// libraries via ZSTD_customMem.
unsafe extern "C" fn my_alloc(opaque: *mut c_void, size: usize) -> *mut c_void {
    let _ = opaque;
    if size == 0 {
        return std::ptr::null_mut();
    }
    let layout = std::alloc::Layout::from_size_align(size + HDR, 16).unwrap();
    let p = unsafe { std::alloc::alloc(layout) };
    if p.is_null() {
        return std::ptr::null_mut();
    }
    unsafe { (p as *mut usize).write(size) };
    ALLOC_CALLS.fetch_add(1, Ordering::Relaxed);
    LIVE_BYTES.fetch_add(size, Ordering::Relaxed);
    unsafe { p.add(HDR) as *mut c_void }
}

unsafe extern "C" fn my_free(opaque: *mut c_void, addr: *mut c_void) {
    let _ = opaque;
    if addr.is_null() {
        return;
    }
    let base = unsafe { (addr as *mut u8).sub(HDR) };
    let size = unsafe { (base as *mut usize).read() };
    let layout = std::alloc::Layout::from_size_align(size + HDR, 16).unwrap();
    unsafe { std::alloc::dealloc(base, layout) };
    FREE_CALLS.fetch_add(1, Ordering::Relaxed);
    LIVE_BYTES.fetch_sub(size, Ordering::Relaxed);
}

// ------------------------------------------------------------- helpers -----

/// A workspace buffer guaranteed 8-byte aligned (Vec<u64>) with byte access.
struct Wksp(Vec<u64>);
impl Wksp {
    fn new(bytes: usize) -> Wksp {
        Wksp(vec![0u64; bytes / 8 + 2])
    }
    fn ptr(&mut self) -> *mut c_void {
        self.0.as_mut_ptr() as *mut c_void
    }
    /// Pointer offset by `off` bytes (used to break the 8-byte alignment).
    fn ptr_off(&mut self, off: usize) -> *mut c_void {
        unsafe { (self.0.as_mut_ptr() as *mut u8).add(off) as *mut c_void }
    }
    fn capacity(&self) -> usize {
        self.0.len() * 8
    }
}

fn frame_of(src: &[u8], level: c_int) -> Vec<u8> {
    let (cc, rc) = unsafe { pair::<FnCompress>("ZSTD_compress") };
    let (cb, _) = unsafe { pair::<FnBound>("ZSTD_compressBound") };
    let cap = unsafe { cb(src.len()) } + 64;
    let mut co = vec![0u8; cap];
    let mut ro = vec![0u8; cap];
    let a = unsafe { cc(co.as_mut_ptr(), cap, src.as_ptr(), src.len(), level) };
    let b = unsafe { rc(ro.as_mut_ptr(), cap, src.as_ptr(), src.len(), level) };
    assert_eq!(a, b, "ZSTD_compress return");
    assert!(!is_error(a));
    assert_bytes_eq("frame_of", &co[..a], &ro[..a]);
    co.truncate(a);
    co
}

/// A raw-content dictionary (works with dct_auto / dct_rawContent).
fn raw_dict(n: usize, rng: &mut Rng) -> Vec<u8> {
    gen(Shape::Text, n, rng)
}

/// A buffer that starts with ZSTD_MAGIC_DICTIONARY but is otherwise garbage:
/// accepted by dct_rawContent/dct_auto-as-raw but rejected by dct_fullDict.
fn corrupt_dict(n: usize, rng: &mut Rng) -> Vec<u8> {
    let mut v = ZSTD_MAGIC_DICTIONARY.to_le_bytes().to_vec();
    while v.len() < n {
        v.push(rng.byte());
    }
    v.truncate(n.max(4));
    v
}

fn le32(v: u32) -> [u8; 4] {
    v.to_le_bytes()
}

// =========================================================== CONFIG 162 ====

#[test]
fn cfg_estimate_functions() {
    let (c_ec, r_ec) = unsafe { pair::<FnEstimateLevel>("ZSTD_estimateCCtxSize") };
    let (c_ecp, r_ecp) =
        unsafe { pair::<FnEstimateCParams>("ZSTD_estimateCCtxSize_usingCParams") };
    let (c_ecc, r_ecc) =
        unsafe { pair::<FnEstimateParamsPtr>("ZSTD_estimateCCtxSize_usingCCtxParams") };
    let (c_es, r_es) = unsafe { pair::<FnEstimateLevel>("ZSTD_estimateCStreamSize") };
    let (c_esp, r_esp) =
        unsafe { pair::<FnEstimateCParams>("ZSTD_estimateCStreamSize_usingCParams") };
    let (c_esc, r_esc) =
        unsafe { pair::<FnEstimateParamsPtr>("ZSTD_estimateCStreamSize_usingCCtxParams") };
    let (c_ed, r_ed) = unsafe { pair::<FnEstimateVoid>("ZSTD_estimateDCtxSize") };
    let (c_eds, r_eds) = unsafe { pair::<FnEstimateSize>("ZSTD_estimateDStreamSize") };
    let (c_edf, r_edf) =
        unsafe { pair::<FnEstimateFromFrame>("ZSTD_estimateDStreamSize_fromFrame") };
    let (c_ecd, r_ecd) = unsafe { pair::<FnEstimateCDict>("ZSTD_estimateCDictSize") };
    let (c_ecda, r_ecda) =
        unsafe { pair::<FnEstimateCDictAdv>("ZSTD_estimateCDictSize_advanced") };
    let (c_edd, r_edd) = unsafe { pair::<FnEstimateDDict>("ZSTD_estimateDDictSize") };
    let (c_gcp, r_gcp) = unsafe { pair::<FnGetCParams>("ZSTD_getCParams") };
    let (c_cp, r_cp) = unsafe { pair::<FnNew>("ZSTD_createCCtxParams") };
    let (c_cpf, r_cpf) = unsafe { pair::<FnFree>("ZSTD_freeCCtxParams") };
    let (c_cps, r_cps) = unsafe { pair::<FnSetParam>("ZSTD_CCtxParams_setParameter") };
    let mut rng = Rng::new(0x162);

    // ---- ZSTD_estimateDCtxSize (no arguments)
    assert_eq!(unsafe { c_ed() }, unsafe { r_ed() }, "ZSTD_estimateDCtxSize");

    let levels: Vec<c_int> = vec![1, 3, 9, 19, 22];
    let dict_sizes: Vec<usize> = vec![0, 8192, 112640];

    for &lvl in &levels {
        assert_eq!(
            unsafe { c_ec(lvl) },
            unsafe { r_ec(lvl) },
            "ZSTD_estimateCCtxSize({lvl})"
        );
        assert_eq!(
            unsafe { c_es(lvl) },
            unsafe { r_es(lvl) },
            "ZSTD_estimateCStreamSize({lvl})"
        );
        for &ds in &dict_sizes {
            assert_eq!(
                unsafe { c_ecd(ds, lvl) },
                unsafe { r_ecd(ds, lvl) },
                "ZSTD_estimateCDictSize({ds},{lvl})"
            );
        }
        // ---- cParams from ZSTD_getCParams over several estimated src sizes
        for &est in &[0u64, 1, 1000, 131072, 1 << 20, 1 << 30] {
            for &ds in &dict_sizes {
                let cp_c = unsafe { c_gcp(lvl, est, ds) };
                let cp_r = unsafe { r_gcp(lvl, est, ds) };
                assert_eq!(cp_c, cp_r, "ZSTD_getCParams({lvl},{est},{ds})");
                assert_eq!(
                    unsafe { c_ecp(cp_c) },
                    unsafe { r_ecp(cp_c) },
                    "estimateCCtxSize_usingCParams({cp_c:?})"
                );
                assert_eq!(
                    unsafe { c_esp(cp_c) },
                    unsafe { r_esp(cp_c) },
                    "estimateCStreamSize_usingCParams({cp_c:?})"
                );
                for &dlm in &[DLM_BY_COPY, DLM_BY_REF] {
                    assert_eq!(
                        unsafe { c_ecda(ds, cp_c, dlm) },
                        unsafe { r_ecda(ds, cp_c, dlm) },
                        "estimateCDictSize_advanced({ds},{cp_c:?},{dlm})"
                    );
                    assert_eq!(
                        unsafe { c_edd(ds, dlm) },
                        unsafe { r_edd(ds, dlm) },
                        "estimateDDictSize({ds},{dlm})"
                    );
                }
            }
        }
        // ---- the _usingCCtxParams variants
        for &ds in &dict_sizes {
            let cparams = unsafe { c_cp() };
            let rparams = unsafe { r_cp() };
            assert!(!cparams.is_null() && !rparams.is_null());
            assert!(!is_error(unsafe { c_cps(cparams, C_COMPRESSIONLEVEL, lvl) }));
            assert!(!is_error(unsafe { r_cps(rparams, C_COMPRESSIONLEVEL, lvl) }));
            let ctx = format!("usingCCtxParams lvl={lvl} ds={ds}");
            assert_eq!(
                unsafe { c_ecc(cparams as *const c_void) },
                unsafe { r_ecc(rparams as *const c_void) },
                "{ctx}: estimateCCtxSize_usingCCtxParams"
            );
            assert_eq!(
                unsafe { c_esc(cparams as *const c_void) },
                unsafe { r_esc(rparams as *const c_void) },
                "{ctx}: estimateCStreamSize_usingCCtxParams"
            );
            // and with an explicit windowLog on the params object
            for &wl in &[10, 17, 23, 27] {
                assert!(!is_error(unsafe { c_cps(cparams, C_WINDOWLOG, wl) }));
                assert!(!is_error(unsafe { r_cps(rparams, C_WINDOWLOG, wl) }));
                assert_eq!(
                    unsafe { c_ecc(cparams as *const c_void) },
                    unsafe { r_ecc(rparams as *const c_void) },
                    "{ctx} wl={wl}: estimateCCtxSize_usingCCtxParams"
                );
                assert_eq!(
                    unsafe { c_esc(cparams as *const c_void) },
                    unsafe { r_esc(rparams as *const c_void) },
                    "{ctx} wl={wl}: estimateCStreamSize_usingCCtxParams"
                );
            }
            unsafe {
                c_cpf(cparams);
                r_cpf(rparams);
            }
        }
    }

    // ---- ZSTD_estimateDStreamSize over many window sizes
    let mut windows: Vec<usize> = vec![0, 1, 1024, 1 << 17, (1 << 17) + 1, 1 << 20, 1 << 27];
    windows.push(1usize << 30);
    windows.push(1usize << 31);
    for _ in 0..40 {
        windows.push(rng.below(1 << 28));
    }
    for &w in &windows {
        assert_eq!(
            unsafe { c_eds(w) },
            unsafe { r_eds(w) },
            "ZSTD_estimateDStreamSize({w})"
        );
    }

    // ---- ZSTD_estimateDStreamSize_fromFrame on real frames
    for &shape in ALL_SHAPES {
        for &n in &[0usize, 1, 1000, 131072, 300000] {
            let src = gen(shape, n, &mut rng);
            for &lvl in &levels {
                let f = frame_of(&src, lvl);
                let a = unsafe { c_edf(f.as_ptr(), f.len()) };
                let b = unsafe { r_edf(f.as_ptr(), f.len()) };
                assert_eq!(a, b, "estimateDStreamSize_fromFrame shape={shape:?} n={n} lvl={lvl}");
            }
        }
    }
}

// ======================================================= ERRORS 89, 90 ====

#[test]
fn err_estimate_nbworkers() {
    let (c_ecc, r_ecc) =
        unsafe { pair::<FnEstimateParamsPtr>("ZSTD_estimateCCtxSize_usingCCtxParams") };
    let (c_esc, r_esc) =
        unsafe { pair::<FnEstimateParamsPtr>("ZSTD_estimateCStreamSize_usingCCtxParams") };
    let (c_cp, r_cp) = unsafe { pair::<FnNew>("ZSTD_createCCtxParams") };
    let (c_cpf, r_cpf) = unsafe { pair::<FnFree>("ZSTD_freeCCtxParams") };
    let (c_cps, r_cps) = unsafe { pair::<FnSetParam>("ZSTD_CCtxParams_setParameter") };

    // NOTE: this build has no multi-threading support (ZSTD_MULTITHREAD is not
    // defined), so ZSTD_CCtxParams_setParameter(ZSTD_c_nbWorkers, >0) is itself
    // rejected with parameter_unsupported and nbWorkers stays 0.  The
    // `params->nbWorkers > 0` guards inside the two estimate functions are
    // therefore unreachable here.  What is verified is that both libraries
    // reject the parameter identically and that the estimates then still agree.
    for &nb in &[1, 2, 4, 16, 200] {
        let cparams = unsafe { c_cp() };
        let rparams = unsafe { r_cp() };
        assert!(!cparams.is_null() && !rparams.is_null());
        let a = unsafe { c_cps(cparams, C_NBWORKERS, nb) };
        let b = unsafe { r_cps(rparams, C_NBWORKERS, nb) };
        assert_eq!(a, b, "CCtxParams_setParameter(nbWorkers={nb}) (C err={})", err_code(a));
        assert!(is_error(a), "nbWorkers={nb} must be rejected in this non-MT build");

        let x = unsafe { c_ecc(cparams as *const c_void) };
        let y = unsafe { r_ecc(rparams as *const c_void) };
        assert_eq!(x, y, "estimateCCtxSize_usingCCtxParams after nbWorkers={nb}");
        let x = unsafe { c_esc(cparams as *const c_void) };
        let y = unsafe { r_esc(rparams as *const c_void) };
        assert_eq!(x, y, "estimateCStreamSize_usingCCtxParams after nbWorkers={nb}");
        unsafe {
            c_cpf(cparams);
            r_cpf(rparams);
        }
    }
    // note: a NULL ZSTD_CCtx_params* is not covered -- the C implementation
    // dereferences it unconditionally (`params->nbWorkers`), so passing NULL is
    // undefined behaviour rather than an error condition.
}

// ===================================================== ERRORS 260, 261 ====

#[test]
fn err_estimate_dstream_fromframe() {
    let (c_edf, r_edf) =
        unsafe { pair::<FnEstimateFromFrame>("ZSTD_estimateDStreamSize_fromFrame") };
    let mut rng = Rng::new(0x260);

    let cmp = |buf: &[u8], ctx: &str| unsafe {
        let a = c_edf(buf.as_ptr(), buf.len());
        let b = r_edf(buf.as_ptr(), buf.len());
        assert_eq!(a, b, "{ctx}: estimateDStreamSize_fromFrame (C err={})", err_code(a));
        a
    };

    // ERRORS 260: srcSize too small for the header (every truncation)
    let f = frame_of(&gen(Shape::Text, 200000, &mut rng), 5);
    for k in 0..=f.len().min(24) {
        cmp(&f[..k], &format!("trunc {k}"));
    }
    // bad magic / junk
    for round in 0..32 {
        let j = gen(Shape::Random, rng.range(0, 24), &mut rng);
        cmp(&j, &format!("junk {round}"));
    }

    // ERRORS 261: windowSize > 1 << ZSTD_WINDOWLOG_MAX.
    // windowLog = wlCode + 10; windowSize = 2^windowLog + (2^windowLog >> 3)*mantissa
    for wl_code in 0u8..=31 {
        for mantissa in 0u8..=7 {
            let mut v = le32(ZSTD_MAGICNUMBER).to_vec();
            v.push(0x00); // no single segment, no dictID, fcsID 0, no checksum
            v.push((wl_code << 3) | mantissa);
            // one raw last block of 1 byte
            let bh: u32 = 1 | (1 << 3);
            v.push((bh & 0xFF) as u8);
            v.push(((bh >> 8) & 0xFF) as u8);
            v.push(((bh >> 16) & 0xFF) as u8);
            v.push(0x42);
            cmp(&v, &format!("wlCode={wl_code} mantissa={mantissa}"));
        }
    }
    // singleSegment frames whose frameContentSize *is* the window size
    for &fcs in &[
        1u64,
        1 << 20,
        (1u64 << 31) - 1,
        1u64 << 31,
        (1u64 << 31) + 1,
        1u64 << 32,
        1u64 << 40,
        u64::MAX - 2,
    ] {
        let mut v = le32(ZSTD_MAGICNUMBER).to_vec();
        v.push(0x20 | 0xC0); // singleSegment, fcsID = 3 (8-byte FCS)
        v.extend_from_slice(&fcs.to_le_bytes());
        let bh: u32 = 1 | (1 << 3);
        v.push((bh & 0xFF) as u8);
        v.push(((bh >> 8) & 0xFF) as u8);
        v.push(((bh >> 16) & 0xFF) as u8);
        v.push(0x42);
        cmp(&v, &format!("singleSegment fcs={fcs}"));
    }
}

// ====================================================== CONFIGS 163-165 ====

#[test]
fn cfg_init_static() {
    let (c_isc, r_isc) = unsafe { pair::<FnInitStatic>("ZSTD_initStaticCCtx") };
    let (c_ics, r_ics) = unsafe { pair::<FnInitStatic>("ZSTD_initStaticCStream") };
    let (c_isd, r_isd) = unsafe { pair::<FnInitStatic>("ZSTD_initStaticDCtx") };
    let (c_isds, r_isds) = unsafe { pair::<FnInitStatic>("ZSTD_initStaticDStream") };
    let (c_iscd, r_iscd) = unsafe { pair::<FnInitStaticCDict>("ZSTD_initStaticCDict") };
    let (c_isdd, r_isdd) = unsafe { pair::<FnInitStaticDDict>("ZSTD_initStaticDDict") };
    let (c_ec, r_ec) = unsafe { pair::<FnEstimateLevel>("ZSTD_estimateCCtxSize") };
    let (c_es, r_es) = unsafe { pair::<FnEstimateLevel>("ZSTD_estimateCStreamSize") };
    let (c_ed, r_ed) = unsafe { pair::<FnEstimateVoid>("ZSTD_estimateDCtxSize") };
    let (c_eds, r_eds) = unsafe { pair::<FnEstimateSize>("ZSTD_estimateDStreamSize") };
    let (c_ecda, r_ecda) =
        unsafe { pair::<FnEstimateCDictAdv>("ZSTD_estimateCDictSize_advanced") };
    let (c_edd, r_edd) = unsafe { pair::<FnEstimateDDict>("ZSTD_estimateDDictSize") };
    let (c_gcp, _) = unsafe { pair::<FnGetCParams>("ZSTD_getCParams") };
    let (c_cc, r_cc) = unsafe { pair::<FnCompressCCtx>("ZSTD_compressCCtx") };
    let (c_dd, r_dd) = unsafe { pair::<FnDecompressDCtx>("ZSTD_decompressDCtx") };
    let (c_ics2, r_ics2) = unsafe { pair::<FnInitCStream>("ZSTD_initCStream") };
    let (c_ids, r_ids) = unsafe { pair::<FnInitDStream>("ZSTD_initDStream") };
    let (c_cs2, r_cs2) = unsafe { pair::<FnCompressStream2>("ZSTD_compressStream2") };
    let (c_ds, r_ds) = unsafe { pair::<FnDecompressStream>("ZSTD_decompressStream") };
    let (c_cucd, r_cucd) =
        unsafe { pair::<FnCompressUsingCDict>("ZSTD_compress_usingCDict") };
    let (c_dudd, r_dudd) =
        unsafe { pair::<FnDecompressUsingDDict>("ZSTD_decompress_usingDDict") };
    let (c_cb, _) = unsafe { pair::<FnBound>("ZSTD_compressBound") };
    let mut rng = Rng::new(0x163);

    let src = gen(Shape::Text, 65536, &mut rng);
    let cap = unsafe { c_cb(src.len()) } + 64;

    // ------------------------------------------------ static CCtx / DCtx ----
    for &lvl in &[1, 3, 9, 19] {
        let cn = unsafe { c_ec(lvl) };
        let rn = unsafe { r_ec(lvl) };
        assert_eq!(cn, rn, "estimateCCtxSize({lvl})");
        let mut cws = Wksp::new(cn);
        let mut rws = Wksp::new(rn);
        let cctx = unsafe { c_isc(cws.ptr(), cn) };
        let rctx = unsafe { r_isc(rws.ptr(), rn) };
        assert_eq!(cctx.is_null(), rctx.is_null(), "initStaticCCtx({lvl}) null-ness");
        assert!(!cctx.is_null(), "initStaticCCtx({lvl}) should succeed");

        let mut co = vec![0xAAu8; cap];
        let mut ro = vec![0x55u8; cap];
        let a = unsafe { c_cc(cctx, co.as_mut_ptr(), cap, src.as_ptr(), src.len(), lvl) };
        let b = unsafe { r_cc(rctx, ro.as_mut_ptr(), cap, src.as_ptr(), src.len(), lvl) };
        assert_eq!(a, b, "static compressCCtx({lvl}) (C err={})", err_code(a));
        assert!(!is_error(a), "static compressCCtx({lvl}) err {}", err_code(a));
        assert_bytes_eq(&format!("static compressCCtx({lvl}) bytes"), &co[..a], &ro[..a]);

        // static DCtx round trip on that frame
        let dn = unsafe { c_ed() };
        assert_eq!(dn, unsafe { r_ed() });
        let mut cdws = Wksp::new(dn);
        let mut rdws = Wksp::new(dn);
        let dctx = unsafe { c_isd(cdws.ptr(), dn) };
        let rdctx = unsafe { r_isd(rdws.ptr(), dn) };
        assert_eq!(dctx.is_null(), rdctx.is_null(), "initStaticDCtx null-ness");
        assert!(!dctx.is_null());
        let dcap = src.len() + 64;
        let mut cd = vec![0u8; dcap];
        let mut rd = vec![0u8; dcap];
        let x = unsafe { c_dd(dctx, cd.as_mut_ptr(), dcap, co.as_ptr(), a) };
        let y = unsafe { r_dd(rdctx, rd.as_mut_ptr(), dcap, ro.as_ptr(), b) };
        assert_eq!(x, y, "static decompressDCtx (C err={})", err_code(x));
        assert!(!is_error(x), "static decompressDCtx err {}", err_code(x));
        assert_bytes_eq("static decompressDCtx bytes", &cd[..x], &rd[..y]);
        assert_bytes_eq("static round trip vs orig", &cd[..x], &src);
    }

    // -------------------------------------------- static CStream / DStream --
    for &lvl in &[1, 3, 9] {
        let cn = unsafe { c_es(lvl) };
        assert_eq!(cn, unsafe { r_es(lvl) }, "estimateCStreamSize({lvl})");
        let mut cws = Wksp::new(cn);
        let mut rws = Wksp::new(cn);
        let ccs = unsafe { c_ics(cws.ptr(), cn) };
        let rcs = unsafe { r_ics(rws.ptr(), cn) };
        assert_eq!(ccs.is_null(), rcs.is_null(), "initStaticCStream({lvl}) null-ness");
        assert!(!ccs.is_null());
        assert!(!is_error(unsafe { c_ics2(ccs, lvl) }));
        assert!(!is_error(unsafe { r_ics2(rcs, lvl) }));

        let mut out: Vec<Vec<u8>> = Vec::new();
        for which in 0..2 {
            let ctx = if which == 0 { ccs } else { rcs };
            let stream = if which == 0 { &c_cs2 } else { &r_cs2 };
            let mut buf = vec![0u8; cap];
            let mut ob = OutBuffer { dst: buf.as_mut_ptr(), size: cap, pos: 0 };
            let mut fed = 0usize;
            while fed < src.len() {
                let take = (src.len() - fed).min(4096);
                let mut ib =
                    InBuffer { src: unsafe { src.as_ptr().add(fed) }, size: take, pos: 0 };
                while ib.pos < ib.size {
                    let r = unsafe { stream(ctx, &mut ob, &mut ib, ZSTD_E_CONTINUE) };
                    assert!(!is_error(r), "static cstream err {}", err_code(r));
                }
                fed += take;
            }
            loop {
                let mut ib = InBuffer { src: src.as_ptr(), size: 0, pos: 0 };
                let r = unsafe { stream(ctx, &mut ob, &mut ib, ZSTD_E_END) };
                assert!(!is_error(r), "static cstream end err {}", err_code(r));
                if r == 0 {
                    break;
                }
            }
            buf.truncate(ob.pos);
            out.push(buf);
        }
        assert_bytes_eq(&format!("static CStream({lvl}) bytes"), &out[0], &out[1]);

        // static DStream over the produced frame
        let wn = unsafe { c_eds(1 << 23) };
        assert_eq!(wn, unsafe { r_eds(1 << 23) });
        let mut cdws = Wksp::new(wn);
        let mut rdws = Wksp::new(wn);
        let cds = unsafe { c_isds(cdws.ptr(), wn) };
        let rds = unsafe { r_isds(rdws.ptr(), wn) };
        assert_eq!(cds.is_null(), rds.is_null(), "initStaticDStream null-ness");
        assert!(!cds.is_null());
        assert!(!is_error(unsafe { c_ids(cds) }));
        assert!(!is_error(unsafe { r_ids(rds) }));
        let mut dec: Vec<Vec<u8>> = Vec::new();
        for which in 0..2 {
            let ctx = if which == 0 { cds } else { rds };
            let stream = if which == 0 { &c_ds } else { &r_ds };
            let frame = &out[which];
            let mut buf = vec![0u8; src.len() + 64];
            let mut ob =
                OutBuffer { dst: buf.as_mut_ptr(), size: src.len() + 64, pos: 0 };
            let mut ib = InBuffer { src: frame.as_ptr(), size: frame.len(), pos: 0 };
            while ib.pos < ib.size {
                let r = unsafe { stream(ctx, &mut ob, &mut ib) };
                assert!(!is_error(r), "static dstream err {}", err_code(r));
                if r == 0 {
                    break;
                }
            }
            buf.truncate(ob.pos);
            dec.push(buf);
        }
        assert_bytes_eq(&format!("static DStream({lvl}) bytes"), &dec[0], &dec[1]);
        assert_bytes_eq(&format!("static DStream({lvl}) vs orig"), &dec[0], &src);
    }

    // ------------------------------------------------ static CDict / DDict --
    for &dsz in &[0usize, 8192, 112640] {
        let dict = raw_dict(dsz, &mut rng);
        for &lvl in &[3, 9] {
            let cparams = unsafe { c_gcp(lvl, src.len() as u64, dsz) };
            for &dlm in &[DLM_BY_COPY, DLM_BY_REF] {
                let cn = unsafe { c_ecda(dsz, cparams, dlm) };
                assert_eq!(cn, unsafe { r_ecda(dsz, cparams, dlm) }, "estimateCDictSize_advanced");
                let mut cws = Wksp::new(cn);
                let mut rws = Wksp::new(cn);
                let ccd = unsafe {
                    c_iscd(cws.ptr(), cn, dict.as_ptr(), dsz, dlm, DCT_RAWCONTENT, cparams)
                };
                let rcd = unsafe {
                    r_iscd(rws.ptr(), cn, dict.as_ptr(), dsz, dlm, DCT_RAWCONTENT, cparams)
                };
                assert_eq!(
                    ccd.is_null(),
                    rcd.is_null(),
                    "initStaticCDict(dsz={dsz} lvl={lvl} dlm={dlm}) null-ness"
                );
                if ccd.is_null() {
                    continue;
                }
                // use it through a normal CCtx of the *same* library
                let (c_new, r_new) = unsafe { pair::<FnNew>("ZSTD_createCCtx") };
                let (c_free, r_free) = unsafe { pair::<FnFree>("ZSTD_freeCCtx") };
                let cctx = unsafe { c_new() };
                let rctx = unsafe { r_new() };
                let mut co = vec![0xAAu8; cap];
                let mut ro = vec![0x55u8; cap];
                let a = unsafe {
                    c_cucd(cctx, co.as_mut_ptr(), cap, src.as_ptr(), src.len(), ccd)
                };
                let b = unsafe {
                    r_cucd(rctx, ro.as_mut_ptr(), cap, src.as_ptr(), src.len(), rcd)
                };
                let ctx = format!("static CDict dsz={dsz} lvl={lvl} dlm={dlm}");
                assert_eq!(a, b, "{ctx}: compress_usingCDict (C err={})", err_code(a));
                assert!(!is_error(a), "{ctx}: err {}", err_code(a));
                assert_bytes_eq(&format!("{ctx}: bytes"), &co[..a], &ro[..a]);

                // static DDict for the round trip
                let dn = unsafe { c_edd(dsz, dlm) };
                assert_eq!(dn, unsafe { r_edd(dsz, dlm) }, "estimateDDictSize");
                let mut cdws = Wksp::new(dn);
                let mut rdws = Wksp::new(dn);
                let cdd = unsafe {
                    c_isdd(cdws.ptr(), dn, dict.as_ptr(), dsz, dlm, DCT_RAWCONTENT)
                };
                let rdd = unsafe {
                    r_isdd(rdws.ptr(), dn, dict.as_ptr(), dsz, dlm, DCT_RAWCONTENT)
                };
                assert_eq!(cdd.is_null(), rdd.is_null(), "{ctx}: initStaticDDict null-ness");
                assert!(!cdd.is_null(), "{ctx}: initStaticDDict should succeed");
                let (c_dnew, r_dnew) = unsafe { pair::<FnNew>("ZSTD_createDCtx") };
                let (c_dfree, r_dfree) = unsafe { pair::<FnFree>("ZSTD_freeDCtx") };
                let dctx = unsafe { c_dnew() };
                let rdctx = unsafe { r_dnew() };
                let dcap = src.len() + 64;
                let mut cd = vec![0u8; dcap];
                let mut rd = vec![0u8; dcap];
                let x = unsafe {
                    c_dudd(dctx, cd.as_mut_ptr(), dcap, co.as_ptr(), a, cdd)
                };
                let y = unsafe {
                    r_dudd(rdctx, rd.as_mut_ptr(), dcap, ro.as_ptr(), b, rdd)
                };
                assert_eq!(x, y, "{ctx}: decompress_usingDDict (C err={})", err_code(x));
                assert!(!is_error(x), "{ctx}: dec err {}", err_code(x));
                assert_bytes_eq(&format!("{ctx}: dec bytes"), &cd[..x], &rd[..y]);
                assert_bytes_eq(&format!("{ctx}: dec vs orig"), &cd[..x], &src);
                unsafe {
                    c_free(cctx);
                    r_free(rctx);
                    c_dfree(dctx);
                    r_dfree(rdctx);
                }
            }
        }
    }
}

// ================================== ERRORS 24, 26, 102, 212, 221 ==========

#[test]
fn err_init_static_small() {
    let (c_isc, r_isc) = unsafe { pair::<FnInitStatic>("ZSTD_initStaticCCtx") };
    let (c_ics, r_ics) = unsafe { pair::<FnInitStatic>("ZSTD_initStaticCStream") };
    let (c_isd, r_isd) = unsafe { pair::<FnInitStatic>("ZSTD_initStaticDCtx") };
    let (c_isds, r_isds) = unsafe { pair::<FnInitStatic>("ZSTD_initStaticDStream") };
    let (c_iscd, r_iscd) = unsafe { pair::<FnInitStaticCDict>("ZSTD_initStaticCDict") };
    let (c_isdd, r_isdd) = unsafe { pair::<FnInitStaticDDict>("ZSTD_initStaticDDict") };
    let (c_ec, _) = unsafe { pair::<FnEstimateLevel>("ZSTD_estimateCCtxSize") };
    let (c_es, _) = unsafe { pair::<FnEstimateLevel>("ZSTD_estimateCStreamSize") };
    let (c_ed, _) = unsafe { pair::<FnEstimateVoid>("ZSTD_estimateDCtxSize") };
    let (c_eds, _) = unsafe { pair::<FnEstimateSize>("ZSTD_estimateDStreamSize") };
    let (c_ecda, _) = unsafe { pair::<FnEstimateCDictAdv>("ZSTD_estimateCDictSize_advanced") };
    let (c_edd, _) = unsafe { pair::<FnEstimateDDict>("ZSTD_estimateDDictSize") };
    let (c_gcp, _) = unsafe { pair::<FnGetCParams>("ZSTD_getCParams") };
    let mut rng = Rng::new(0x24);

    /// Sizes to probe below (and at) a known-good size: a dense sweep of the very
    /// small values plus a geometric sweep plus exact-1 / exact.
    fn probe_sizes(full: usize) -> Vec<usize> {
        let mut v: Vec<usize> = (0..80).collect();
        let mut s = 80usize;
        while s < full {
            v.push(s);
            s = s + s / 8 + 17;
        }
        for k in [1usize, 2, 3, 4, 8, 16, 64, 1024] {
            if full > k {
                v.push(full - k);
            }
        }
        v.push(full);
        v.push(full + 1);
        v.sort_unstable();
        v.dedup();
        v
    }

    // ---- ERRORS 24 / 26: initStaticCCtx / initStaticCStream
    for &lvl in &[1, 9, 19] {
        let full = unsafe { c_ec(lvl) };
        let mut cws = Wksp::new(full + 64);
        let mut rws = Wksp::new(full + 64);
        for &n in &probe_sizes(full) {
            let a = unsafe { c_isc(cws.ptr(), n) };
            let b = unsafe { r_isc(rws.ptr(), n) };
            assert_eq!(
                a.is_null(),
                b.is_null(),
                "initStaticCCtx(n={n}, full={full}, lvl={lvl}) null-ness"
            );
        }
        let full = unsafe { c_es(lvl) };
        let mut cws = Wksp::new(full + 64);
        let mut rws = Wksp::new(full + 64);
        for &n in &probe_sizes(full) {
            let a = unsafe { c_ics(cws.ptr(), n) };
            let b = unsafe { r_ics(rws.ptr(), n) };
            assert_eq!(
                a.is_null(),
                b.is_null(),
                "initStaticCStream(n={n}, full={full}, lvl={lvl}) null-ness"
            );
        }
    }

    // ---- ERRORS 212: initStaticDCtx / initStaticDStream
    {
        let full = unsafe { c_ed() };
        let mut cws = Wksp::new(full + 64);
        let mut rws = Wksp::new(full + 64);
        for &n in &probe_sizes(full) {
            let a = unsafe { c_isd(cws.ptr(), n) };
            let b = unsafe { r_isd(rws.ptr(), n) };
            assert_eq!(a.is_null(), b.is_null(), "initStaticDCtx(n={n}, full={full}) null-ness");
        }
        let full = unsafe { c_eds(1 << 20) };
        let mut cws = Wksp::new(full + 64);
        let mut rws = Wksp::new(full + 64);
        for &n in &probe_sizes(full) {
            let a = unsafe { c_isds(cws.ptr(), n) };
            let b = unsafe { r_isds(rws.ptr(), n) };
            assert_eq!(a.is_null(), b.is_null(), "initStaticDStream(n={n}, full={full}) null-ness");
        }
    }

    // ---- ERRORS 102: initStaticCDict
    for &dsz in &[0usize, 1024, 8192] {
        let dict = raw_dict(dsz, &mut rng);
        let cparams = unsafe { c_gcp(9, 65536, dsz) };
        for &dlm in &[DLM_BY_COPY, DLM_BY_REF] {
            let full = unsafe { c_ecda(dsz, cparams, dlm) };
            let mut cws = Wksp::new(full + 64);
            let mut rws = Wksp::new(full + 64);
            for &n in &probe_sizes(full) {
                let a = unsafe {
                    c_iscd(cws.ptr(), n, dict.as_ptr(), dsz, dlm, DCT_RAWCONTENT, cparams)
                };
                let b = unsafe {
                    r_iscd(rws.ptr(), n, dict.as_ptr(), dsz, dlm, DCT_RAWCONTENT, cparams)
                };
                assert_eq!(
                    a.is_null(),
                    b.is_null(),
                    "initStaticCDict(n={n}, full={full}, dsz={dsz}, dlm={dlm}) null-ness"
                );
            }
        }
    }

    // ---- ERRORS 221: initStaticDDict
    for &dsz in &[0usize, 1024, 8192] {
        let dict = raw_dict(dsz, &mut rng);
        for &dlm in &[DLM_BY_COPY, DLM_BY_REF] {
            let full = unsafe { c_edd(dsz, dlm) };
            let mut cws = Wksp::new(full + 64);
            let mut rws = Wksp::new(full + 64);
            for &n in &probe_sizes(full) {
                let a =
                    unsafe { c_isdd(cws.ptr(), n, dict.as_ptr(), dsz, dlm, DCT_RAWCONTENT) };
                let b =
                    unsafe { r_isdd(rws.ptr(), n, dict.as_ptr(), dsz, dlm, DCT_RAWCONTENT) };
                assert_eq!(
                    a.is_null(),
                    b.is_null(),
                    "initStaticDDict(n={n}, full={full}, dsz={dsz}, dlm={dlm}) null-ness"
                );
            }
        }
    }
}

// ==================================== ERRORS 25, 101, 211, 220 ============

#[test]
fn err_init_static_misaligned() {
    let (c_isc, r_isc) = unsafe { pair::<FnInitStatic>("ZSTD_initStaticCCtx") };
    let (c_ics, r_ics) = unsafe { pair::<FnInitStatic>("ZSTD_initStaticCStream") };
    let (c_isd, r_isd) = unsafe { pair::<FnInitStatic>("ZSTD_initStaticDCtx") };
    let (c_isds, r_isds) = unsafe { pair::<FnInitStatic>("ZSTD_initStaticDStream") };
    let (c_iscd, r_iscd) = unsafe { pair::<FnInitStaticCDict>("ZSTD_initStaticCDict") };
    let (c_isdd, r_isdd) = unsafe { pair::<FnInitStaticDDict>("ZSTD_initStaticDDict") };
    let (c_ec, _) = unsafe { pair::<FnEstimateLevel>("ZSTD_estimateCCtxSize") };
    let (c_es, _) = unsafe { pair::<FnEstimateLevel>("ZSTD_estimateCStreamSize") };
    let (c_ed, _) = unsafe { pair::<FnEstimateVoid>("ZSTD_estimateDCtxSize") };
    let (c_eds, _) = unsafe { pair::<FnEstimateSize>("ZSTD_estimateDStreamSize") };
    let (c_ecda, _) = unsafe { pair::<FnEstimateCDictAdv>("ZSTD_estimateCDictSize_advanced") };
    let (c_edd, _) = unsafe { pair::<FnEstimateDDict>("ZSTD_estimateDDictSize") };
    let (c_gcp, _) = unsafe { pair::<FnGetCParams>("ZSTD_getCParams") };
    let mut rng = Rng::new(0x25);

    // every non-zero byte offset breaks the mandatory 8-byte alignment
    let offs: [usize; 7] = [1, 2, 3, 4, 5, 6, 7];

    for &lvl in &[1, 9, 19] {
        let full = unsafe { c_ec(lvl) };
        let mut cws = Wksp::new(full + 64);
        let mut rws = Wksp::new(full + 64);
        for &off in &offs {
            let a = unsafe { c_isc(cws.ptr_off(off), full) };
            let b = unsafe { r_isc(rws.ptr_off(off), full) };
            assert_eq!(a.is_null(), b.is_null(), "initStaticCCtx off={off} lvl={lvl}");
            assert!(a.is_null(), "misaligned initStaticCCtx must fail (off={off})");
        }
        let full = unsafe { c_es(lvl) };
        let mut cws = Wksp::new(full + 64);
        let mut rws = Wksp::new(full + 64);
        for &off in &offs {
            let a = unsafe { c_ics(cws.ptr_off(off), full) };
            let b = unsafe { r_ics(rws.ptr_off(off), full) };
            assert_eq!(a.is_null(), b.is_null(), "initStaticCStream off={off} lvl={lvl}");
            assert!(a.is_null());
        }
    }
    {
        let full = unsafe { c_ed() };
        let mut cws = Wksp::new(full + 64);
        let mut rws = Wksp::new(full + 64);
        for &off in &offs {
            let a = unsafe { c_isd(cws.ptr_off(off), full) };
            let b = unsafe { r_isd(rws.ptr_off(off), full) };
            assert_eq!(a.is_null(), b.is_null(), "initStaticDCtx off={off}");
            assert!(a.is_null());
        }
        let full = unsafe { c_eds(1 << 20) };
        let mut cws = Wksp::new(full + 64);
        let mut rws = Wksp::new(full + 64);
        for &off in &offs {
            let a = unsafe { c_isds(cws.ptr_off(off), full) };
            let b = unsafe { r_isds(rws.ptr_off(off), full) };
            assert_eq!(a.is_null(), b.is_null(), "initStaticDStream off={off}");
            assert!(a.is_null());
        }
    }
    for &dsz in &[0usize, 8192] {
        let dict = raw_dict(dsz, &mut rng);
        let cparams = unsafe { c_gcp(9, 65536, dsz) };
        for &dlm in &[DLM_BY_COPY, DLM_BY_REF] {
            let full = unsafe { c_ecda(dsz, cparams, dlm) };
            let mut cws = Wksp::new(full + 64);
            let mut rws = Wksp::new(full + 64);
            for &off in &offs {
                let a = unsafe {
                    c_iscd(cws.ptr_off(off), full, dict.as_ptr(), dsz, dlm, DCT_RAWCONTENT, cparams)
                };
                let b = unsafe {
                    r_iscd(rws.ptr_off(off), full, dict.as_ptr(), dsz, dlm, DCT_RAWCONTENT, cparams)
                };
                assert_eq!(a.is_null(), b.is_null(), "initStaticCDict off={off} dsz={dsz}");
                assert!(a.is_null());
            }
            let full = unsafe { c_edd(dsz, dlm) };
            let mut cws = Wksp::new(full + 64);
            let mut rws = Wksp::new(full + 64);
            for &off in &offs {
                let a = unsafe {
                    c_isdd(cws.ptr_off(off), full, dict.as_ptr(), dsz, dlm, DCT_RAWCONTENT)
                };
                let b = unsafe {
                    r_isdd(rws.ptr_off(off), full, dict.as_ptr(), dsz, dlm, DCT_RAWCONTENT)
                };
                assert_eq!(a.is_null(), b.is_null(), "initStaticDDict off={off} dsz={dsz}");
                assert!(a.is_null());
            }
        }
    }
}

// ======================================================= ERRORS 103, 222 ==

#[test]
fn err_init_static_baddict() {
    let (c_iscd, r_iscd) = unsafe { pair::<FnInitStaticCDict>("ZSTD_initStaticCDict") };
    let (c_isdd, r_isdd) = unsafe { pair::<FnInitStaticDDict>("ZSTD_initStaticDDict") };
    let (c_ecda, _) = unsafe { pair::<FnEstimateCDictAdv>("ZSTD_estimateCDictSize_advanced") };
    let (c_edd, _) = unsafe { pair::<FnEstimateDDict>("ZSTD_estimateDDictSize") };
    let (c_gcp, _) = unsafe { pair::<FnGetCParams>("ZSTD_getCParams") };
    let mut rng = Rng::new(0x103);

    // dictionaries that are invalid for ZSTD_dct_fullDict:
    //  - too small (< 8 bytes)
    //  - wrong magic
    //  - right magic but garbage entropy tables
    let mut dicts: Vec<(String, Vec<u8>)> = Vec::new();
    for n in [0usize, 1, 4, 7] {
        dicts.push((format!("tiny {n}"), gen(Shape::Random, n, &mut rng)));
    }
    dicts.push(("raw content".into(), raw_dict(8192, &mut rng)));
    for n in [8usize, 64, 1024, 8192] {
        dicts.push((format!("magic+garbage {n}"), corrupt_dict(n, &mut rng)));
    }

    for (label, dict) in &dicts {
        let dsz = dict.len();
        let cparams = unsafe { c_gcp(9, 65536, dsz) };
        for &dct in &[DCT_AUTO, DCT_RAWCONTENT, DCT_FULLDICT] {
            for &dlm in &[DLM_BY_COPY, DLM_BY_REF] {
                let full = unsafe { c_ecda(dsz, cparams, dlm) } + 4096;
                let mut cws = Wksp::new(full + 64);
                let mut rws = Wksp::new(full + 64);
                let a = unsafe {
                    c_iscd(cws.ptr(), full, dict.as_ptr(), dsz, dlm, dct, cparams)
                };
                let b = unsafe {
                    r_iscd(rws.ptr(), full, dict.as_ptr(), dsz, dlm, dct, cparams)
                };
                assert_eq!(
                    a.is_null(),
                    b.is_null(),
                    "initStaticCDict({label}, dct={dct}, dlm={dlm}) null-ness"
                );
                if dct == DCT_FULLDICT {
                    assert!(a.is_null(), "fullDict on {label} must fail");
                }

                let full = unsafe { c_edd(dsz, dlm) } + 4096;
                let mut cws = Wksp::new(full + 64);
                let mut rws = Wksp::new(full + 64);
                let a =
                    unsafe { c_isdd(cws.ptr(), full, dict.as_ptr(), dsz, dlm, dct) };
                let b =
                    unsafe { r_isdd(rws.ptr(), full, dict.as_ptr(), dsz, dlm, dct) };
                assert_eq!(
                    a.is_null(),
                    b.is_null(),
                    "initStaticDDict({label}, dct={dct}, dlm={dlm}) null-ness"
                );
                if dct == DCT_FULLDICT {
                    assert!(a.is_null(), "fullDict DDict on {label} must fail");
                }
            }
        }
    }
}

// ============================================================ ERRORS 27 ===

#[test]
fn err_free_static_cctx() {
    let (c_isc, r_isc) = unsafe { pair::<FnInitStatic>("ZSTD_initStaticCCtx") };
    let (c_ics, r_ics) = unsafe { pair::<FnInitStatic>("ZSTD_initStaticCStream") };
    let (c_free, r_free) = unsafe { pair::<FnFree>("ZSTD_freeCCtx") };
    let (c_freecs, r_freecs) = unsafe { pair::<FnFree>("ZSTD_freeCStream") };
    let (c_ec, _) = unsafe { pair::<FnEstimateLevel>("ZSTD_estimateCCtxSize") };
    let (c_es, _) = unsafe { pair::<FnEstimateLevel>("ZSTD_estimateCStreamSize") };

    for &lvl in &[1, 9, 19] {
        let n = unsafe { c_ec(lvl) };
        let mut cws = Wksp::new(n);
        let mut rws = Wksp::new(n);
        let cctx = unsafe { c_isc(cws.ptr(), n) };
        let rctx = unsafe { r_isc(rws.ptr(), n) };
        assert!(!cctx.is_null() && !rctx.is_null());
        // NOTE: each context is freed with its OWN library's free function.
        let a = unsafe { c_free(cctx) };
        let b = unsafe { r_free(rctx) };
        assert_eq!(a, b, "freeCCtx(static, lvl={lvl}) (C err={})", err_code(a));
        assert!(is_error(a), "freeCCtx on a static CCtx must fail");
        // and again, repeatedly: the state must not change
        let a2 = unsafe { c_free(cctx) };
        let b2 = unsafe { r_free(rctx) };
        assert_eq!(a2, b2, "freeCCtx(static) second call");
        assert_eq!(a, a2, "freeCCtx(static) is idempotent");

        let n = unsafe { c_es(lvl) };
        let mut cws = Wksp::new(n);
        let mut rws = Wksp::new(n);
        let ccs = unsafe { c_ics(cws.ptr(), n) };
        let rcs = unsafe { r_ics(rws.ptr(), n) };
        assert!(!ccs.is_null() && !rcs.is_null());
        let a = unsafe { c_freecs(ccs) };
        let b = unsafe { r_freecs(rcs) };
        assert_eq!(a, b, "freeCStream(static, lvl={lvl})");
        assert!(is_error(a), "freeCStream on a static CStream must fail");
    }
}

// =========================================================== ERRORS 213 ===

#[test]
fn err_free_static_dctx() {
    let (c_isd, r_isd) = unsafe { pair::<FnInitStatic>("ZSTD_initStaticDCtx") };
    let (c_isds, r_isds) = unsafe { pair::<FnInitStatic>("ZSTD_initStaticDStream") };
    let (c_free, r_free) = unsafe { pair::<FnFree>("ZSTD_freeDCtx") };
    let (c_freeds, r_freeds) = unsafe { pair::<FnFree>("ZSTD_freeDStream") };
    let (c_ed, _) = unsafe { pair::<FnEstimateVoid>("ZSTD_estimateDCtxSize") };
    let (c_eds, _) = unsafe { pair::<FnEstimateSize>("ZSTD_estimateDStreamSize") };

    let n = unsafe { c_ed() };
    let mut cws = Wksp::new(n);
    let mut rws = Wksp::new(n);
    let cctx = unsafe { c_isd(cws.ptr(), n) };
    let rctx = unsafe { r_isd(rws.ptr(), n) };
    assert!(!cctx.is_null() && !rctx.is_null());
    let a = unsafe { c_free(cctx) };
    let b = unsafe { r_free(rctx) };
    assert_eq!(a, b, "freeDCtx(static) (C err={})", err_code(a));
    assert!(is_error(a), "freeDCtx on a static DCtx must fail");
    let a2 = unsafe { c_free(cctx) };
    let b2 = unsafe { r_free(rctx) };
    assert_eq!(a2, b2, "freeDCtx(static) second call");
    assert_eq!(a, a2, "freeDCtx(static) is idempotent");

    for &w in &[1usize << 17, 1 << 20, 1 << 23] {
        let n = unsafe { c_eds(w) };
        let mut cws = Wksp::new(n);
        let mut rws = Wksp::new(n);
        let cds = unsafe { c_isds(cws.ptr(), n) };
        let rds = unsafe { r_isds(rws.ptr(), n) };
        assert!(!cds.is_null() && !rds.is_null());
        let a = unsafe { c_freeds(cds) };
        let b = unsafe { r_freeds(rds) };
        assert_eq!(a, b, "freeDStream(static, w={w})");
        assert!(is_error(a), "freeDStream on a static DStream must fail");
    }
}

// ============================================================ ERRORS 78 ===

#[test]
fn err_static_loaddict() {
    let (c_isc, r_isc) = unsafe { pair::<FnInitStatic>("ZSTD_initStaticCCtx") };
    let (c_ld, r_ld) = unsafe { pair::<FnLoadDict>("ZSTD_CCtx_loadDictionary") };
    let (c_ec, _) = unsafe { pair::<FnEstimateLevel>("ZSTD_estimateCCtxSize") };
    let mut rng = Rng::new(0x78);

    for &lvl in &[1, 9, 19] {
        for &dsz in &[8usize, 1024, 8192, 112640] {
            let dict = raw_dict(dsz, &mut rng);
            let n = unsafe { c_ec(lvl) };
            let mut cws = Wksp::new(n);
            let mut rws = Wksp::new(n);
            let cctx = unsafe { c_isc(cws.ptr(), n) };
            let rctx = unsafe { r_isc(rws.ptr(), n) };
            assert!(!cctx.is_null() && !rctx.is_null());
            // byCopy (the default for ZSTD_CCtx_loadDictionary) needs to allocate
            let a = unsafe { c_ld(cctx, dict.as_ptr(), dsz) };
            let b = unsafe { r_ld(rctx, dict.as_ptr(), dsz) };
            assert_eq!(a, b, "loadDictionary(static, lvl={lvl}, dsz={dsz}) (C err={})", err_code(a));
            assert!(is_error(a), "byCopy loadDictionary on a static CCtx must fail");
        }
        // a NULL / empty dictionary just clears the dict and succeeds
        let n = unsafe { c_ec(lvl) };
        let mut cws = Wksp::new(n);
        let mut rws = Wksp::new(n);
        let cctx = unsafe { c_isc(cws.ptr(), n) };
        let rctx = unsafe { r_isc(rws.ptr(), n) };
        let a = unsafe { c_ld(cctx, std::ptr::null(), 0) };
        let b = unsafe { r_ld(rctx, std::ptr::null(), 0) };
        assert_eq!(a, b, "loadDictionary(static, NULL) lvl={lvl}");
    }
}

// =========================================================== ERRORS 248 ===

#[test]
fn err_static_refMultipleDDicts() {
    let (c_isd, r_isd) = unsafe { pair::<FnInitStatic>("ZSTD_initStaticDCtx") };
    let (c_isds, r_isds) = unsafe { pair::<FnInitStatic>("ZSTD_initStaticDStream") };
    let (c_sp, r_sp) = unsafe { pair::<FnSetParam>("ZSTD_DCtx_setParameter") };
    let (c_ed, _) = unsafe { pair::<FnEstimateVoid>("ZSTD_estimateDCtxSize") };
    let (c_eds, _) = unsafe { pair::<FnEstimateSize>("ZSTD_estimateDStreamSize") };

    let n = unsafe { c_ed() };
    let mut cws = Wksp::new(n);
    let mut rws = Wksp::new(n);
    let cctx = unsafe { c_isd(cws.ptr(), n) };
    let rctx = unsafe { r_isd(rws.ptr(), n) };
    assert!(!cctx.is_null() && !rctx.is_null());
    // refMultipleDDicts = 1 must be rejected on a static DCtx
    let a = unsafe { c_sp(cctx, D_REFMULTIPLEDDICTS, 1) };
    let b = unsafe { r_sp(rctx, D_REFMULTIPLEDDICTS, 1) };
    assert_eq!(a, b, "static DCtx refMultipleDDicts=1 (C err={})", err_code(a));
    assert!(is_error(a), "refMultipleDDicts=1 on a static DCtx must fail");
    // note: the static-DCtx check in C fires for *any* value, including 0
    // (it is placed after the bounds check but before the assignment), so
    // setting 0 also fails -- both libraries must agree on that.
    let a = unsafe { c_sp(cctx, D_REFMULTIPLEDDICTS, 0) };
    let b = unsafe { r_sp(rctx, D_REFMULTIPLEDDICTS, 0) };
    assert_eq!(a, b, "static DCtx refMultipleDDicts=0 (C err={})", err_code(a));
    // out-of-bounds values are rejected by the bounds check first
    for v in [-1, 2, 100] {
        let a = unsafe { c_sp(cctx, D_REFMULTIPLEDDICTS, v) };
        let b = unsafe { r_sp(rctx, D_REFMULTIPLEDDICTS, v) };
        assert_eq!(a, b, "static DCtx refMultipleDDicts={v} (C err={})", err_code(a));
        assert!(is_error(a));
    }

    // same on a static DStream
    for &w in &[1usize << 17, 1 << 22] {
        let n = unsafe { c_eds(w) };
        let mut cws = Wksp::new(n);
        let mut rws = Wksp::new(n);
        let cds = unsafe { c_isds(cws.ptr(), n) };
        let rds = unsafe { r_isds(rws.ptr(), n) };
        assert!(!cds.is_null() && !rds.is_null());
        let a = unsafe { c_sp(cds, D_REFMULTIPLEDDICTS, 1) };
        let b = unsafe { r_sp(rds, D_REFMULTIPLEDDICTS, 1) };
        assert_eq!(a, b, "static DStream(w={w}) refMultipleDDicts=1");
        assert!(is_error(a));
    }
}

// =========================================================== ERRORS 267 ===

#[test]
fn err_static_dstream_small() {
    let (c_isds, r_isds) = unsafe { pair::<FnInitStatic>("ZSTD_initStaticDStream") };
    let (c_ids, r_ids) = unsafe { pair::<FnInitDStream>("ZSTD_initDStream") };
    let (c_ds, r_ds) = unsafe { pair::<FnDecompressStream>("ZSTD_decompressStream") };
    let (c_ed, _) = unsafe { pair::<FnEstimateVoid>("ZSTD_estimateDCtxSize") };
    let (c_eds, _) = unsafe { pair::<FnEstimateSize>("ZSTD_estimateDStreamSize") };
    let mut rng = Rng::new(0x267);

    let src = gen(Shape::Text, 400000, &mut rng);
    let frame = frame_of(&src, 9);

    // Workspaces that are large enough for the DCtx object itself but too small
    // for the in/out buffers ZSTD_decompressStream needs.
    let dctx_only = unsafe { c_ed() };
    let mut sizes: Vec<usize> = vec![dctx_only];
    for w in [1usize << 10, 1 << 14, 1 << 16] {
        let s = unsafe { c_eds(w) };
        if s > dctx_only {
            sizes.push(s);
        }
    }
    sizes.push(dctx_only + 4096);
    sizes.push(dctx_only + 65536);

    for &n in &sizes {
        let mut cws = Wksp::new(n + 64);
        let mut rws = Wksp::new(n + 64);
        let cds = unsafe { c_isds(cws.ptr(), n) };
        let rds = unsafe { r_isds(rws.ptr(), n) };
        assert_eq!(cds.is_null(), rds.is_null(), "initStaticDStream(n={n}) null-ness");
        if cds.is_null() {
            continue;
        }
        let a = unsafe { c_ids(cds) };
        let b = unsafe { r_ids(rds) };
        assert_eq!(a, b, "initDStream(static n={n})");

        let mut cbuf = vec![0u8; src.len() + 64];
        let mut rbuf = vec![0u8; src.len() + 64];
        let mut cob = OutBuffer { dst: cbuf.as_mut_ptr(), size: cbuf.len(), pos: 0 };
        let mut rob = OutBuffer { dst: rbuf.as_mut_ptr(), size: rbuf.len(), pos: 0 };
        let mut cib = InBuffer { src: frame.as_ptr(), size: frame.len(), pos: 0 };
        let mut rib = InBuffer { src: frame.as_ptr(), size: frame.len(), pos: 0 };
        loop {
            let x = unsafe { c_ds(cds, &mut cob, &mut cib) };
            let y = unsafe { r_ds(rds, &mut rob, &mut rib) };
            assert_eq!(x, y, "decompressStream(static n={n}) (C err={})", err_code(x));
            assert_eq!(cib.pos, rib.pos, "decompressStream(static n={n}) input pos");
            assert_eq!(cob.pos, rob.pos, "decompressStream(static n={n}) output pos");
            if is_error(x) || x == 0 || cib.pos == cib.size {
                break;
            }
        }
        assert_bytes_eq(
            &format!("decompressStream(static n={n}) output"),
            &cbuf[..cob.pos],
            &rbuf[..rob.pos],
        );
    }
}

// =========================================================== CONFIG 166 ====

#[test]
fn cfg_sizeof_functions() {
    let (c_ncc, r_ncc) = unsafe { pair::<FnNew>("ZSTD_createCCtx") };
    let (c_fcc, r_fcc) = unsafe { pair::<FnFree>("ZSTD_freeCCtx") };
    let (c_ndc, r_ndc) = unsafe { pair::<FnNew>("ZSTD_createDCtx") };
    let (c_fdc, r_fdc) = unsafe { pair::<FnFree>("ZSTD_freeDCtx") };
    let (c_ncs, r_ncs) = unsafe { pair::<FnNew>("ZSTD_createCStream") };
    let (c_fcs, r_fcs) = unsafe { pair::<FnFree>("ZSTD_freeCStream") };
    let (c_nds, r_nds) = unsafe { pair::<FnNew>("ZSTD_createDStream") };
    let (c_fds, r_fds) = unsafe { pair::<FnFree>("ZSTD_freeDStream") };
    let (c_scc, r_scc) = unsafe { pair::<FnSizeof>("ZSTD_sizeof_CCtx") };
    let (c_sdc, r_sdc) = unsafe { pair::<FnSizeof>("ZSTD_sizeof_DCtx") };
    let (c_scs, r_scs) = unsafe { pair::<FnSizeof>("ZSTD_sizeof_CStream") };
    let (c_sds, r_sds) = unsafe { pair::<FnSizeof>("ZSTD_sizeof_DStream") };
    let (c_scd, r_scd) = unsafe { pair::<FnSizeof>("ZSTD_sizeof_CDict") };
    let (c_sdd, r_sdd) = unsafe { pair::<FnSizeof>("ZSTD_sizeof_DDict") };
    let (c_cc, r_cc) = unsafe { pair::<FnCompressCCtx>("ZSTD_compressCCtx") };
    let (c_dd, r_dd) = unsafe { pair::<FnDecompressDCtx>("ZSTD_decompressDCtx") };
    let (c_rst, r_rst) = unsafe { pair::<FnReset>("ZSTD_CCtx_reset") };
    let (c_drst, r_drst) = unsafe { pair::<FnReset>("ZSTD_DCtx_reset") };
    let (c_cs2, r_cs2) = unsafe { pair::<FnCompressStream2>("ZSTD_compressStream2") };
    let (c_ds, r_ds) = unsafe { pair::<FnDecompressStream>("ZSTD_decompressStream") };
    let (c_ics, r_ics) = unsafe { pair::<FnInitCStream>("ZSTD_initCStream") };
    let (c_ids, r_ids) = unsafe { pair::<FnInitDStream>("ZSTD_initDStream") };
    let (c_ncd, r_ncd) =
        unsafe { pair::<unsafe extern "C" fn(*const u8, usize, c_int) -> *mut c_void>("ZSTD_createCDict") };
    let (c_fcd, r_fcd) = unsafe { pair::<FnFree>("ZSTD_freeCDict") };
    let (c_ndd, r_ndd) =
        unsafe { pair::<unsafe extern "C" fn(*const u8, usize) -> *mut c_void>("ZSTD_createDDict") };
    let (c_fdd, r_fdd) = unsafe { pair::<FnFree>("ZSTD_freeDDict") };
    let (c_cb, _) = unsafe { pair::<FnBound>("ZSTD_compressBound") };
    let mut rng = Rng::new(0x166);

    let src = gen(Shape::Text, 200000, &mut rng);
    let cap = unsafe { c_cb(src.len()) } + 64;

    // ---- CCtx: after create, after compression, after reset
    for &lvl in &[1, 3, 9, 19] {
        let cctx = unsafe { c_ncc() };
        let rctx = unsafe { r_ncc() };
        assert_eq!(
            unsafe { c_scc(cctx) },
            unsafe { r_scc(rctx) },
            "sizeof_CCtx after create (lvl={lvl})"
        );
        let mut co = vec![0u8; cap];
        let mut ro = vec![0u8; cap];
        let a = unsafe { c_cc(cctx, co.as_mut_ptr(), cap, src.as_ptr(), src.len(), lvl) };
        let b = unsafe { r_cc(rctx, ro.as_mut_ptr(), cap, src.as_ptr(), src.len(), lvl) };
        assert_eq!(a, b);
        assert_eq!(
            unsafe { c_scc(cctx) },
            unsafe { r_scc(rctx) },
            "sizeof_CCtx after compression (lvl={lvl})"
        );
        for &kind in &[RESET_SESSION_ONLY, RESET_PARAMETERS, RESET_SESSION_AND_PARAMETERS] {
            let x = unsafe { c_rst(cctx, kind) };
            let y = unsafe { r_rst(rctx, kind) };
            assert_eq!(x, y, "CCtx_reset({kind})");
            assert_eq!(
                unsafe { c_scc(cctx) },
                unsafe { r_scc(rctx) },
                "sizeof_CCtx after reset({kind}) (lvl={lvl})"
            );
        }

        // ---- DCtx
        let dctx = unsafe { c_ndc() };
        let rdctx = unsafe { r_ndc() };
        assert_eq!(unsafe { c_sdc(dctx) }, unsafe { r_sdc(rdctx) }, "sizeof_DCtx after create");
        let dcap = src.len() + 64;
        let mut cd = vec![0u8; dcap];
        let mut rd = vec![0u8; dcap];
        let x = unsafe { c_dd(dctx, cd.as_mut_ptr(), dcap, co.as_ptr(), a) };
        let y = unsafe { r_dd(rdctx, rd.as_mut_ptr(), dcap, ro.as_ptr(), b) };
        assert_eq!(x, y);
        assert_eq!(
            unsafe { c_sdc(dctx) },
            unsafe { r_sdc(rdctx) },
            "sizeof_DCtx after decompression"
        );
        for &kind in &[RESET_SESSION_ONLY, RESET_PARAMETERS, RESET_SESSION_AND_PARAMETERS] {
            let x = unsafe { c_drst(dctx, kind) };
            let y = unsafe { r_drst(rdctx, kind) };
            assert_eq!(x, y, "DCtx_reset({kind})");
            assert_eq!(
                unsafe { c_sdc(dctx) },
                unsafe { r_sdc(rdctx) },
                "sizeof_DCtx after reset({kind})"
            );
        }
        unsafe {
            c_fcc(cctx);
            r_fcc(rctx);
            c_fdc(dctx);
            r_fdc(rdctx);
        }
    }

    // ---- CStream / DStream
    for &lvl in &[1, 9] {
        let ccs = unsafe { c_ncs() };
        let rcs = unsafe { r_ncs() };
        assert_eq!(
            unsafe { c_scs(ccs) },
            unsafe { r_scs(rcs) },
            "sizeof_CStream after create"
        );
        assert!(!is_error(unsafe { c_ics(ccs, lvl) }));
        assert!(!is_error(unsafe { r_ics(rcs, lvl) }));
        let mut frames: Vec<Vec<u8>> = Vec::new();
        for which in 0..2 {
            let ctx = if which == 0 { ccs } else { rcs };
            let stream = if which == 0 { &c_cs2 } else { &r_cs2 };
            let mut buf = vec![0u8; cap];
            let mut ob = OutBuffer { dst: buf.as_mut_ptr(), size: cap, pos: 0 };
            let mut ib = InBuffer { src: src.as_ptr(), size: src.len(), pos: 0 };
            loop {
                let r = unsafe { stream(ctx, &mut ob, &mut ib, ZSTD_E_END) };
                assert!(!is_error(r));
                if r == 0 {
                    break;
                }
            }
            buf.truncate(ob.pos);
            frames.push(buf);
        }
        assert_bytes_eq("sizeof CStream frames", &frames[0], &frames[1]);
        assert_eq!(
            unsafe { c_scs(ccs) },
            unsafe { r_scs(rcs) },
            "sizeof_CStream after compression"
        );
        let x = unsafe { c_rst(ccs, RESET_SESSION_AND_PARAMETERS) };
        let y = unsafe { r_rst(rcs, RESET_SESSION_AND_PARAMETERS) };
        assert_eq!(x, y);
        assert_eq!(
            unsafe { c_scs(ccs) },
            unsafe { r_scs(rcs) },
            "sizeof_CStream after reset"
        );

        let cdsm = unsafe { c_nds() };
        let rdsm = unsafe { r_nds() };
        assert_eq!(
            unsafe { c_sds(cdsm) },
            unsafe { r_sds(rdsm) },
            "sizeof_DStream after create"
        );
        assert!(!is_error(unsafe { c_ids(cdsm) }));
        assert!(!is_error(unsafe { r_ids(rdsm) }));
        for which in 0..2 {
            let ctx = if which == 0 { cdsm } else { rdsm };
            let stream = if which == 0 { &c_ds } else { &r_ds };
            let frame = &frames[which];
            let mut buf = vec![0u8; src.len() + 64];
            let mut ob = OutBuffer { dst: buf.as_mut_ptr(), size: buf.len(), pos: 0 };
            let mut ib = InBuffer { src: frame.as_ptr(), size: frame.len(), pos: 0 };
            while ib.pos < ib.size {
                let r = unsafe { stream(ctx, &mut ob, &mut ib) };
                assert!(!is_error(r), "dstream err {}", err_code(r));
                if r == 0 {
                    break;
                }
            }
        }
        assert_eq!(
            unsafe { c_sds(cdsm) },
            unsafe { r_sds(rdsm) },
            "sizeof_DStream after decompression"
        );
        let x = unsafe { c_drst(cdsm, RESET_SESSION_AND_PARAMETERS) };
        let y = unsafe { r_drst(rdsm, RESET_SESSION_AND_PARAMETERS) };
        assert_eq!(x, y);
        assert_eq!(
            unsafe { c_sds(cdsm) },
            unsafe { r_sds(rdsm) },
            "sizeof_DStream after reset"
        );
        unsafe {
            c_fcs(ccs);
            r_fcs(rcs);
            c_fds(cdsm);
            r_fds(rdsm);
        }
    }

    // ---- CDict / DDict
    for &dsz in &[0usize, 8192, 112640] {
        let dict = raw_dict(dsz, &mut rng);
        for &lvl in &[1, 9, 19] {
            let ccd = unsafe { c_ncd(dict.as_ptr(), dsz, lvl) };
            let rcd = unsafe { r_ncd(dict.as_ptr(), dsz, lvl) };
            assert_eq!(ccd.is_null(), rcd.is_null(), "createCDict null-ness");
            assert_eq!(
                unsafe { c_scd(ccd) },
                unsafe { r_scd(rcd) },
                "sizeof_CDict(dsz={dsz} lvl={lvl})"
            );
            unsafe {
                c_fcd(ccd);
                r_fcd(rcd);
            }
        }
        let cdd = unsafe { c_ndd(dict.as_ptr(), dsz) };
        let rdd = unsafe { r_ndd(dict.as_ptr(), dsz) };
        assert_eq!(cdd.is_null(), rdd.is_null(), "createDDict null-ness");
        assert_eq!(unsafe { c_sdd(cdd) }, unsafe { r_sdd(rdd) }, "sizeof_DDict(dsz={dsz})");
        unsafe {
            c_fdd(cdd);
            r_fdd(rdd);
        }
    }
}

// ============== ERRORS 28, 29, 104, 105, 214, 215, 223, 224 ===============

#[test]
fn err_null_free_and_sizeof() {
    for name in [
        "ZSTD_freeCCtx",
        "ZSTD_freeCStream",
        "ZSTD_freeDCtx",
        "ZSTD_freeDStream",
        "ZSTD_freeCDict",
        "ZSTD_freeDDict",
        "ZSTD_freeCCtxParams",
    ] {
        let (c, r) = unsafe { pair::<FnFree>(name) };
        let a = unsafe { c(std::ptr::null_mut()) };
        let b = unsafe { r(std::ptr::null_mut()) };
        assert_eq!(a, b, "{name}(NULL)");
        assert_eq!(a, 0, "{name}(NULL) must return 0");
    }
    for name in [
        "ZSTD_sizeof_CCtx",
        "ZSTD_sizeof_CStream",
        "ZSTD_sizeof_DCtx",
        "ZSTD_sizeof_DStream",
        "ZSTD_sizeof_CDict",
        "ZSTD_sizeof_DDict",
    ] {
        let (c, r) = unsafe { pair::<FnSizeof>(name) };
        let a = unsafe { c(std::ptr::null()) };
        let b = unsafe { r(std::ptr::null()) };
        assert_eq!(a, b, "{name}(NULL)");
        assert_eq!(a, 0, "{name}(NULL) must return 0");
    }
    // note: ZSTD_freeThreadPool / ZSTD_createThreadPool are not exported by this
    // (non-multi-threaded) build, so they are not covered here.
}

// =========================================================== CONFIG 167 ====

#[test]
fn cfg_frame_progression() {
    let (c_new, r_new) = unsafe { pair::<FnNew>("ZSTD_createCCtx") };
    let (c_free, r_free) = unsafe { pair::<FnFree>("ZSTD_freeCCtx") };
    let (c_set, r_set) = unsafe { pair::<FnSetParam>("ZSTD_CCtx_setParameter") };
    let (c_cs2, r_cs2) = unsafe { pair::<FnCompressStream2>("ZSTD_compressStream2") };
    let (c_fp, r_fp) = unsafe { pair::<FnFrameProgression>("ZSTD_getFrameProgression") };
    let (c_tf, r_tf) = unsafe { pair::<FnToFlushNow>("ZSTD_toFlushNow") };
    let (c_g1, r_g1) = unsafe { pair::<FnGet1BlockSummary>("ZSTD_get1BlockSummary") };
    let (c_cb, _) = unsafe { pair::<FnBound>("ZSTD_compressBound") };
    let (c_dec, r_dec) = unsafe { pair::<FnDecompress>("ZSTD_decompress") };
    let mut rng = Rng::new(0x167);

    let src = gen(Shape::Text, 400000, &mut rng);
    let cap = unsafe { c_cb(src.len()) } + 1024;

    let cctx = unsafe { c_new() };
    let rctx = unsafe { r_new() };
    assert!(!cctx.is_null() && !rctx.is_null());
    assert!(!is_error(unsafe { c_set(cctx, C_COMPRESSIONLEVEL, 9) }));
    assert!(!is_error(unsafe { r_set(rctx, C_COMPRESSIONLEVEL, 9) }));

    let mut cbuf = vec![0u8; cap];
    let mut rbuf = vec![0u8; cap];
    let mut cob = OutBuffer { dst: cbuf.as_mut_ptr(), size: cap, pos: 0 };
    let mut rob = OutBuffer { dst: rbuf.as_mut_ptr(), size: cap, pos: 0 };

    let check = |label: &str| {
        let a = unsafe { c_fp(cctx as *const c_void) };
        let b = unsafe { r_fp(rctx as *const c_void) };
        assert_eq!(a.ingested, b.ingested, "{label}: progression.ingested");
        assert_eq!(a.consumed, b.consumed, "{label}: progression.consumed");
        assert_eq!(a.produced, b.produced, "{label}: progression.produced");
        assert_eq!(a.flushed, b.flushed, "{label}: progression.flushed");
        assert_eq!(a.currentJobID, b.currentJobID, "{label}: progression.currentJobID");
        assert_eq!(
            a.nbActiveWorkers, b.nbActiveWorkers,
            "{label}: progression.nbActiveWorkers"
        );
        assert_eq!(a, b, "{label}: whole ZSTD_frameProgression struct");
        let x = unsafe { c_tf(cctx) };
        let y = unsafe { r_tf(rctx) };
        assert_eq!(x, y, "{label}: ZSTD_toFlushNow");
    };

    // point 0: before anything happens
    check("start");

    // feed the input in chunks, probing progression at several points
    let mut fed = 0usize;
    let mut probe = 0usize;
    while fed < src.len() {
        let take = (src.len() - fed).min(37_000);
        let mut cib = InBuffer { src: unsafe { src.as_ptr().add(fed) }, size: take, pos: 0 };
        let mut rib = InBuffer { src: unsafe { src.as_ptr().add(fed) }, size: take, pos: 0 };
        while cib.pos < cib.size {
            let x = unsafe { c_cs2(cctx, &mut cob, &mut cib, ZSTD_E_CONTINUE) };
            let y = unsafe { r_cs2(rctx, &mut rob, &mut rib, ZSTD_E_CONTINUE) };
            assert_eq!(x, y, "compressStream2(continue) return");
            assert_eq!(cib.pos, rib.pos, "input pos");
            assert_eq!(cob.pos, rob.pos, "output pos");
            assert!(!is_error(x), "compressStream2 err {}", err_code(x));
        }
        fed += take;
        probe += 1;
        check(&format!("after chunk {probe} (fed={fed})"));
    }

    // flush, then end -- probing again at each point
    loop {
        let mut cib = InBuffer { src: src.as_ptr(), size: 0, pos: 0 };
        let mut rib = InBuffer { src: src.as_ptr(), size: 0, pos: 0 };
        let x = unsafe { c_cs2(cctx, &mut cob, &mut cib, ZSTD_E_FLUSH) };
        let y = unsafe { r_cs2(rctx, &mut rob, &mut rib, ZSTD_E_FLUSH) };
        assert_eq!(x, y, "compressStream2(flush) return");
        assert!(!is_error(x));
        check("during flush");
        if x == 0 {
            break;
        }
    }
    loop {
        let mut cib = InBuffer { src: src.as_ptr(), size: 0, pos: 0 };
        let mut rib = InBuffer { src: src.as_ptr(), size: 0, pos: 0 };
        let x = unsafe { c_cs2(cctx, &mut cob, &mut cib, ZSTD_E_END) };
        let y = unsafe { r_cs2(rctx, &mut rob, &mut rib, ZSTD_E_END) };
        assert_eq!(x, y, "compressStream2(end) return");
        assert!(!is_error(x));
        check("during end");
        if x == 0 {
            break;
        }
    }
    check("after end");

    assert_eq!(cob.pos, rob.pos, "final output size");
    assert_bytes_eq("frame progression frames", &cbuf[..cob.pos], &rbuf[..rob.pos]);
    // round trip for good measure
    let dcap = src.len() + 64;
    let mut cd = vec![0u8; dcap];
    let mut rd = vec![0u8; dcap];
    let x = unsafe { c_dec(cd.as_mut_ptr(), dcap, cbuf.as_ptr(), cob.pos) };
    let y = unsafe { r_dec(rd.as_mut_ptr(), dcap, rbuf.as_ptr(), rob.pos) };
    assert_eq!(x, y, "progression round trip return");
    assert!(!is_error(x));
    assert_bytes_eq("progression round trip", &cd[..x], &src);

    unsafe {
        c_free(cctx);
        r_free(rctx);
    }

    // ---- ZSTD_get1BlockSummary.
    // NOTE: despite living next to the progression API in CONFIGS, this function
    // has nothing to do with streaming state: it is a pure function over a
    // ZSTD_Sequence array that measures the next block described by it.  It is
    // therefore exercised on hand-built sequence arrays.
    let arrays: Vec<Vec<Sequence>> = vec![
        // single delimiter
        vec![Sequence { offset: 0, litLength: 0, matchLength: 0, rep: 0 }],
        // delimiter carrying last literals
        vec![Sequence { offset: 0, litLength: 17, matchLength: 0, rep: 0 }],
        // a few sequences then a delimiter, then a second block
        vec![
            Sequence { offset: 5, litLength: 3, matchLength: 4, rep: 0 },
            Sequence { offset: 7, litLength: 0, matchLength: 9, rep: 0 },
            Sequence { offset: 0, litLength: 11, matchLength: 0, rep: 0 },
            Sequence { offset: 2, litLength: 1, matchLength: 3, rep: 0 },
            Sequence { offset: 0, litLength: 0, matchLength: 0, rep: 0 },
        ],
        // no delimiter at all => must report an error identically
        vec![
            Sequence { offset: 5, litLength: 3, matchLength: 4, rep: 0 },
            Sequence { offset: 7, litLength: 2, matchLength: 9, rep: 0 },
        ],
        // long lengths (> 64 KB)
        vec![
            Sequence { offset: 9, litLength: 70000, matchLength: 80000, rep: 0 },
            Sequence { offset: 0, litLength: 5, matchLength: 0, rep: 0 },
        ],
        vec![],
    ];
    // NOTE: when no block delimiter is found, C sets only `nbSequences` (to an
    // error code) and leaves `blockSize` / `litSize` uninitialized, so only
    // `nbSequences` may be compared in that case.
    let cmp_summary = |a: BlockSummary, b: BlockSummary, ctx: String| {
        assert_eq!(a.nbSequences, b.nbSequences, "{ctx}: nbSequences");
        if !is_error(a.nbSequences) {
            assert_eq!(a.blockSize, b.blockSize, "{ctx}: blockSize");
            assert_eq!(a.litSize, b.litSize, "{ctx}: litSize");
        }
    };
    for (i, arr) in arrays.iter().enumerate() {
        for len in 0..=arr.len() {
            let a = unsafe { c_g1(arr.as_ptr(), len) };
            let b = unsafe { r_g1(arr.as_ptr(), len) };
            cmp_summary(a, b, format!("ZSTD_get1BlockSummary(array {i}, len {len})"));
        }
    }
    // and randomized arrays
    for round in 0..200 {
        let n = rng.range(1, 40);
        let mut arr: Vec<Sequence> = Vec::with_capacity(n);
        for _ in 0..n {
            if rng.below(5) == 0 {
                arr.push(Sequence { offset: 0, litLength: rng.below(50) as u32, matchLength: 0, rep: 0 });
            } else {
                arr.push(Sequence {
                    offset: rng.range(1, 1000) as u32,
                    litLength: rng.below(100) as u32,
                    matchLength: rng.range(3, 200) as u32,
                    rep: 0,
                });
            }
        }
        let a = unsafe { c_g1(arr.as_ptr(), arr.len()) };
        let b = unsafe { r_g1(arr.as_ptr(), arr.len()) };
        cmp_summary(a, b, format!("ZSTD_get1BlockSummary(random {round})"));
    }
}

// ================== ERRORS 23, 30, 99, 210, 216, 273 =====================

#[test]
fn err_custommem_xor() {
    let alloc_only = CustomMem {
        customAlloc: Some(my_alloc),
        customFree: None,
        opaque: std::ptr::null_mut(),
    };
    let free_only = CustomMem {
        customAlloc: None,
        customFree: Some(my_free),
        opaque: std::ptr::null_mut(),
    };
    let both = CustomMem {
        customAlloc: Some(my_alloc),
        customFree: Some(my_free),
        opaque: std::ptr::null_mut(),
    };
    let mut rng = Rng::new(0x23);

    // ---- ERRORS 23 / 210 / 273: the simple create*_advanced entry points
    for name in [
        "ZSTD_createCCtx_advanced",
        "ZSTD_createCStream_advanced",
        "ZSTD_createDCtx_advanced",
        "ZSTD_createDStream_advanced",
        "ZBUFF_createCCtx_advanced",
        "ZBUFF_createDCtx_advanced",
    ] {
        let (c, r) = unsafe { pair::<FnCreateAdvanced>(name) };
        for (label, cm) in [("alloc only", alloc_only), ("free only", free_only)] {
            let a = unsafe { c(cm) };
            let b = unsafe { r(cm) };
            assert!(a.is_null(), "{name} with {label} must return NULL (C)");
            assert!(b.is_null(), "{name} with {label} must return NULL (Rust)");
        }
        // the default (all-NULL) customMem is valid and must produce an object
        let a = unsafe { c(CustomMem::none()) };
        let b = unsafe { r(CustomMem::none()) };
        assert_eq!(a.is_null(), b.is_null(), "{name} default customMem null-ness");
        assert!(!a.is_null(), "{name} with the default customMem should succeed");
        // free with the matching library's free function
        let free_name = match name {
            "ZSTD_createCCtx_advanced" => "ZSTD_freeCCtx",
            "ZSTD_createCStream_advanced" => "ZSTD_freeCStream",
            "ZSTD_createDCtx_advanced" => "ZSTD_freeDCtx",
            "ZSTD_createDStream_advanced" => "ZSTD_freeDStream",
            "ZBUFF_createCCtx_advanced" => "ZBUFF_freeCCtx",
            _ => "ZBUFF_freeDCtx",
        };
        let (cf, rf) = unsafe { pair::<FnFree>(free_name) };
        let x = unsafe { cf(a) };
        let y = unsafe { rf(b) };
        assert_eq!(x, y, "{free_name} return");
    }

    // ---- ERRORS 99: createCDict_advanced (and _advanced2 through the same rule)
    {
        let (c, r) = unsafe { pair::<FnCreateCDictAdvanced>("ZSTD_createCDict_advanced") };
        let (c_gcp, _) = unsafe { pair::<FnGetCParams>("ZSTD_getCParams") };
        let dict = raw_dict(8192, &mut rng);
        let cparams = unsafe { c_gcp(9, 65536, dict.len()) };
        for (label, cm) in [("alloc only", alloc_only), ("free only", free_only)] {
            let a = unsafe {
                c(dict.as_ptr(), dict.len(), DLM_BY_COPY, DCT_RAWCONTENT, cparams, cm)
            };
            let b = unsafe {
                r(dict.as_ptr(), dict.len(), DLM_BY_COPY, DCT_RAWCONTENT, cparams, cm)
            };
            assert!(a.is_null(), "createCDict_advanced with {label} must be NULL (C)");
            assert!(b.is_null(), "createCDict_advanced with {label} must be NULL (Rust)");
        }
    }

    // ---- ERRORS 216: createDDict_advanced
    {
        let (c, r) = unsafe { pair::<FnCreateDDictAdvanced>("ZSTD_createDDict_advanced") };
        let dict = raw_dict(8192, &mut rng);
        for (label, cm) in [("alloc only", alloc_only), ("free only", free_only)] {
            let a = unsafe { c(dict.as_ptr(), dict.len(), DLM_BY_COPY, DCT_RAWCONTENT, cm) };
            let b = unsafe { r(dict.as_ptr(), dict.len(), DLM_BY_COPY, DCT_RAWCONTENT, cm) };
            assert!(a.is_null(), "createDDict_advanced with {label} must be NULL (C)");
            assert!(b.is_null(), "createDDict_advanced with {label} must be NULL (Rust)");
        }
    }

    // ---- ERRORS 30: ZSTD_createCCtxParams.
    // ZSTD_createCCtxParams_advanced() (which holds the XOR check) is `static` in
    // the C source and is NOT exported, so the only reachable entry point is
    // ZSTD_createCCtxParams(), which always passes ZSTD_defaultCMem (both NULL).
    // The condition is therefore unreachable from outside the library; what is
    // verified here is that the reachable path behaves identically.
    {
        let (c, r) = unsafe { pair::<FnNew>("ZSTD_createCCtxParams") };
        let (cf, rf) = unsafe { pair::<FnFree>("ZSTD_freeCCtxParams") };
        let a = unsafe { c() };
        let b = unsafe { r() };
        assert_eq!(a.is_null(), b.is_null(), "createCCtxParams null-ness");
        assert!(!a.is_null());
        assert_eq!(unsafe { cf(a) }, unsafe { rf(b) }, "freeCCtxParams");
    }

    // ---- a WORKING custom allocator pair: full round trip through it
    {
        let (c_cca, r_cca) = unsafe { pair::<FnCreateAdvanced>("ZSTD_createCCtx_advanced") };
        let (c_dca, r_dca) = unsafe { pair::<FnCreateAdvanced>("ZSTD_createDCtx_advanced") };
        let (c_fcc, r_fcc) = unsafe { pair::<FnFree>("ZSTD_freeCCtx") };
        let (c_fdc, r_fdc) = unsafe { pair::<FnFree>("ZSTD_freeDCtx") };
        let (c_cc, r_cc) = unsafe { pair::<FnCompressCCtx>("ZSTD_compressCCtx") };
        let (c_dd, r_dd) = unsafe { pair::<FnDecompressDCtx>("ZSTD_decompressDCtx") };
        let (c_cb, _) = unsafe { pair::<FnBound>("ZSTD_compressBound") };

        let before_alloc = ALLOC_CALLS.load(Ordering::Relaxed);
        for &shape in ALL_SHAPES {
            for &n in &[0usize, 1, 1000, 65536, 200000] {
                let src = gen(shape, n, &mut rng);
                let cap = unsafe { c_cb(src.len()) } + 64;
                for &lvl in &[1, 9, 19] {
                    let cctx = unsafe { c_cca(both) };
                    let rctx = unsafe { r_cca(both) };
                    assert!(!cctx.is_null() && !rctx.is_null(), "custom alloc cctx");
                    let mut co = vec![0xAAu8; cap];
                    let mut ro = vec![0x55u8; cap];
                    let a =
                        unsafe { c_cc(cctx, co.as_mut_ptr(), cap, src.as_ptr(), src.len(), lvl) };
                    let b =
                        unsafe { r_cc(rctx, ro.as_mut_ptr(), cap, src.as_ptr(), src.len(), lvl) };
                    let ctx = format!("customMem shape={shape:?} n={n} lvl={lvl}");
                    assert_eq!(a, b, "{ctx}: compressCCtx (C err={})", err_code(a));
                    assert!(!is_error(a), "{ctx}: err {}", err_code(a));
                    assert_bytes_eq(&format!("{ctx}: bytes"), &co[..a], &ro[..a]);

                    let dctx = unsafe { c_dca(both) };
                    let rdctx = unsafe { r_dca(both) };
                    assert!(!dctx.is_null() && !rdctx.is_null(), "custom alloc dctx");
                    let dcap = src.len() + 64;
                    let mut cd = vec![0u8; dcap];
                    let mut rd = vec![0u8; dcap];
                    let x = unsafe { c_dd(dctx, cd.as_mut_ptr(), dcap, co.as_ptr(), a) };
                    let y = unsafe { r_dd(rdctx, rd.as_mut_ptr(), dcap, ro.as_ptr(), b) };
                    assert_eq!(x, y, "{ctx}: decompressDCtx (C err={})", err_code(x));
                    assert!(!is_error(x));
                    assert_bytes_eq(&format!("{ctx}: dec bytes"), &cd[..x], &rd[..y]);
                    assert_bytes_eq(&format!("{ctx}: dec vs orig"), &cd[..x], &src);
                    unsafe {
                        c_fcc(cctx);
                        r_fcc(rctx);
                        c_fdc(dctx);
                        r_fdc(rdctx);
                    }
                }
            }
        }
        assert!(
            ALLOC_CALLS.load(Ordering::Relaxed) > before_alloc,
            "the custom allocator was never called"
        );
        assert_eq!(
            LIVE_BYTES.load(Ordering::Relaxed),
            0,
            "custom allocator leaked ({} allocs, {} frees)",
            ALLOC_CALLS.load(Ordering::Relaxed),
            FREE_CALLS.load(Ordering::Relaxed)
        );
    }
}
