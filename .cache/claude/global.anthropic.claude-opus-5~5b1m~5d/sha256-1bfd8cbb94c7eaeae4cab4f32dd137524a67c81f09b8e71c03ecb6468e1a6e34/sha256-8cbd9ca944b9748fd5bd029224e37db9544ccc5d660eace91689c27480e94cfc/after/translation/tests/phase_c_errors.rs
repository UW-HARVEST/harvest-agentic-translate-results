//! Phase C part 1: compression-side error paths (ERRORS.md rows 1-144).
#![allow(non_snake_case, dead_code)]
mod common;
use common::*;
use std::ffi::CStr;
use std::os::raw::{c_char, c_int, c_uint, c_ulonglong, c_void};

type FnNew = unsafe extern "C" fn() -> *mut c_void;
type FnFree = unsafe extern "C" fn(*mut c_void) -> usize;
type FnFreeVoidOk = unsafe extern "C" fn(*mut c_void) -> usize;
type FnSetParam = unsafe extern "C" fn(*mut c_void, c_int, c_int) -> usize;
type FnGetParam = unsafe extern "C" fn(*mut c_void, c_int, *mut c_int) -> usize;
type FnCompress = unsafe extern "C" fn(*mut u8, usize, *const u8, usize, c_int) -> usize;
type FnCompressCCtx =
    unsafe extern "C" fn(*mut c_void, *mut u8, usize, *const u8, usize, c_int) -> usize;
type FnCompress2 = unsafe extern "C" fn(*mut c_void, *mut u8, usize, *const u8, usize) -> usize;
type FnCompressAdv = unsafe extern "C" fn(
    *mut c_void,
    *mut u8,
    usize,
    *const u8,
    usize,
    *const u8,
    usize,
    Parameters,
) -> usize;
type FnBound = unsafe extern "C" fn(usize) -> usize;
type FnIsError = unsafe extern "C" fn(usize) -> c_uint;
type FnGetErrorCode = unsafe extern "C" fn(usize) -> c_int;
type FnGetErrorName = unsafe extern "C" fn(usize) -> *const c_char;
type FnGetErrorString = unsafe extern "C" fn(c_int) -> *const c_char;
type FnPledged = unsafe extern "C" fn(*mut c_void, u64) -> usize;
type FnReset = unsafe extern "C" fn(*mut c_void, c_int) -> usize;
type FnLoadDict = unsafe extern "C" fn(*mut c_void, *const u8, usize) -> usize;
type FnLoadDictAdv = unsafe extern "C" fn(*mut c_void, *const u8, usize, c_int, c_int) -> usize;
type FnRefDict = unsafe extern "C" fn(*mut c_void, *const c_void) -> usize;
type FnRefPrefixAdv = unsafe extern "C" fn(*mut c_void, *const u8, usize, c_int) -> usize;
type FnCreateCDict = unsafe extern "C" fn(*const u8, usize, c_int) -> *mut c_void;
type FnCreateCDictAdv = unsafe extern "C" fn(
    *const u8,
    usize,
    c_int,
    c_int,
    CParams,
    CustomMem,
) -> *mut c_void;
type FnCreateDDict = unsafe extern "C" fn(*const u8, usize) -> *mut c_void;
type FnCreateDDictAdv =
    unsafe extern "C" fn(*const u8, usize, c_int, c_int, CustomMem) -> *mut c_void;
type FnFreeDict = unsafe extern "C" fn(*mut c_void) -> usize;
type FnDictIDFromBuf = unsafe extern "C" fn(*const u8, usize) -> c_uint;
type FnDictIDFromObj = unsafe extern "C" fn(*const c_void) -> c_uint;
type FnStream2 = unsafe extern "C" fn(*mut c_void, *mut OutBuffer, *mut InBuffer, c_int) -> usize;
type FnStream = unsafe extern "C" fn(*mut c_void, *mut OutBuffer, *mut InBuffer) -> usize;
type FnFlush = unsafe extern "C" fn(*mut c_void, *mut OutBuffer) -> usize;
type FnBegin = unsafe extern "C" fn(*mut c_void, c_int) -> usize;
type FnBeginAdv =
    unsafe extern "C" fn(*mut c_void, *const u8, usize, Parameters, c_ulonglong) -> usize;
type FnBeginCDict = unsafe extern "C" fn(*mut c_void, *const c_void) -> usize;
type FnBeginCDictAdv =
    unsafe extern "C" fn(*mut c_void, *const c_void, FParams, c_ulonglong) -> usize;
type FnChunk = unsafe extern "C" fn(*mut c_void, *mut u8, usize, *const u8, usize) -> usize;
type FnCopyCCtx = unsafe extern "C" fn(*mut c_void, *const c_void, c_ulonglong) -> usize;
type FnCtxSize = unsafe extern "C" fn(*const c_void) -> usize;
type FnWriteLastEmpty = unsafe extern "C" fn(*mut u8, usize) -> usize;
type FnCheckCParams = unsafe extern "C" fn(CParams) -> usize;
type FnSetCParams = unsafe extern "C" fn(*mut c_void, CParams) -> usize;
type FnSetFParams = unsafe extern "C" fn(*mut c_void, FParams) -> usize;
type FnSetParams = unsafe extern "C" fn(*mut c_void, Parameters) -> usize;
type FnGetParams = unsafe extern "C" fn(c_int, c_ulonglong, usize) -> Parameters;
type FnGetCParams = unsafe extern "C" fn(c_int, c_ulonglong, usize) -> CParams;
type FnCParamsInit = unsafe extern "C" fn(*mut c_void, c_int) -> usize;
type FnCParamsInitAdv = unsafe extern "C" fn(*mut c_void, Parameters) -> usize;
type FnCParamsReset = unsafe extern "C" fn(*mut c_void) -> usize;
type FnUseParams = unsafe extern "C" fn(*mut c_void, *const c_void) -> usize;
type FnCompressUsingCDict =
    unsafe extern "C" fn(*mut c_void, *mut u8, usize, *const u8, usize, *const c_void) -> usize;
type FnCompressUsingCDictAdv = unsafe extern "C" fn(
    *mut c_void,
    *mut u8,
    usize,
    *const u8,
    usize,
    *const c_void,
    FParams,
) -> usize;
type FnRefThreadPool = unsafe extern "C" fn(*mut c_void, *mut c_void) -> usize;

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
pub struct CustomMem {
    pub alloc: Option<unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void>,
    pub free: Option<unsafe extern "C" fn(*mut c_void, *mut c_void)>,
    pub opaque: *mut c_void,
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

pub const CPARAMS: &[(c_int, &str)] = &[
    (100, "compressionLevel"),
    (101, "windowLog"),
    (102, "hashLog"),
    (103, "chainLog"),
    (104, "searchLog"),
    (105, "minMatch"),
    (106, "targetLength"),
    (107, "strategy"),
    (130, "targetCBlockSize"),
    (160, "enableLDM"),
    (161, "ldmHashLog"),
    (162, "ldmMinMatch"),
    (163, "ldmBucketSizeLog"),
    (164, "ldmHashRateLog"),
    (200, "contentSizeFlag"),
    (201, "checksumFlag"),
    (202, "dictIDFlag"),
    (400, "nbWorkers"),
    (401, "jobSize"),
    (402, "overlapLog"),
    (500, "rsyncable"),
    (10, "format"),
    (1000, "forceMaxWindow"),
    (1001, "forceAttachDict"),
    (1002, "literalCompressionMode"),
    (1004, "srcSizeHint"),
    (1005, "enableDedicatedDictSearch"),
    (1006, "stableInBuffer"),
    (1007, "stableOutBuffer"),
    (1008, "blockDelimiters"),
    (1009, "validateSequences"),
    (1010, "splitAfterSequences"),
    (1011, "useRowMatchFinder"),
    (1012, "deterministicRefPrefix"),
    (1013, "prefetchCDictTables"),
    (1014, "enableSeqProducerFallback"),
    (1015, "maxBlockSize"),
    (1016, "repcodeResolution"),
    (1017, "blockSplitterLevel"),
];

const C_COMPRESSIONLEVEL: c_int = 100;
const C_WINDOWLOG: c_int = 101;
const C_CHECKSUMFLAG: c_int = 201;
const C_STABLEINBUFFER: c_int = 1006;
const C_STABLEOUTBUFFER: c_int = 1007;
const E_CONTINUE: c_int = 0;
const E_FLUSH: c_int = 1;
const E_END: c_int = 2;

const MAGIC_DICTIONARY: u32 = 0xEC30A437;

fn cstr(p: *const c_char) -> String {
    if p.is_null() {
        return "<null>".into();
    }
    unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned()
}

// ---------------------------------------------------------------- rows 1-6 --

#[test]
fn err_error_api() {
    let (cie, rie) = unsafe { pair::<FnIsError>("ZSTD_isError") };
    let (cgc, rgc) = unsafe { pair::<FnGetErrorCode>("ZSTD_getErrorCode") };
    let (cgn, rgn) = unsafe { pair::<FnGetErrorName>("ZSTD_getErrorName") };
    let (cgs, rgs) = unsafe { pair::<FnGetErrorString>("ZSTD_getErrorString") };
    let (ces, res) = unsafe { pair::<FnGetErrorString>("ERR_getErrorString") };

    let mut codes: Vec<usize> = vec![0, 1, 2, 100, 1000, usize::MAX / 2];
    for c in 0..=130usize {
        codes.push(0usize.wrapping_sub(c));
    }
    codes.push(usize::MAX);
    for &code in &codes {
        assert_eq!(unsafe { cie(code) }, unsafe { rie(code) }, "ZSTD_isError({code})");
        assert_eq!(
            unsafe { cgc(code) },
            unsafe { rgc(code) },
            "ZSTD_getErrorCode({code})"
        );
        assert_eq!(
            cstr(unsafe { cgn(code) }),
            cstr(unsafe { rgn(code) }),
            "ZSTD_getErrorName({code})"
        );
    }
    for e in -5..=130i32 {
        assert_eq!(
            cstr(unsafe { cgs(e) }),
            cstr(unsafe { rgs(e) }),
            "ZSTD_getErrorString({e})"
        );
        assert_eq!(
            cstr(unsafe { ces(e) }),
            cstr(unsafe { res(e) }),
            "ERR_getErrorString({e})"
        );
    }
    for e in [i32::MIN, i32::MIN + 1, -1000, 200, 1000, i32::MAX] {
        assert_eq!(cstr(unsafe { cgs(e) }), cstr(unsafe { rgs(e) }), "getErrorString({e})");
        assert_eq!(cstr(unsafe { ces(e) }), cstr(unsafe { res(e) }), "ERR_getErrorString({e})");
    }
}

// ------------------------------------------------------------------- row 7 --

#[test]
fn err_compressbound_huge() {
    let (cb, rb) = unsafe { pair::<FnBound>("ZSTD_compressBound") };
    for n in [
        0usize,
        1,
        1 << 20,
        0xFF00FF00FF00FF00u64 as usize - 1,
        0xFF00FF00FF00FF00u64 as usize,
        0xFF00FF00FF00FF00u64 as usize + 1,
        usize::MAX - 1,
        usize::MAX,
    ] {
        assert_eq!(unsafe { cb(n) }, unsafe { rb(n) }, "compressBound({n})");
    }
    // ZSTD_compress with a huge (unallocatable) srcSize is not testable, but
    // ZSTD_compressBound's sentinel is what the API exposes.
}

// --------------------------------------------------------------- rows 8-13 --

#[test]
fn err_dst_capacity_compress() {
    let (cc, rc) = unsafe { pair::<FnCompress>("ZSTD_compress") };
    let (ccx, rcx) = unsafe { pair::<FnCompressCCtx>("ZSTD_compressCCtx") };
    let (cn, rn) = unsafe { pair::<FnNew>("ZSTD_createCCtx") };
    let (cf, rf) = unsafe { pair::<FnFree>("ZSTD_freeCCtx") };
    let (cs, rs) = unsafe { pair::<FnSetParam>("ZSTD_CCtx_setParameter") };
    let (c2, r2) = unsafe { pair::<FnCompress2>("ZSTD_compress2") };
    let (cb, _) = unsafe { pair::<FnBound>("ZSTD_compressBound") };
    let mut rng = Rng::new(0xE001);

    for &shape in ALL_SHAPES {
        for &size in &[0usize, 1, 100, 5000, 131072, 200000] {
            let src = gen(shape, size, &mut rng);
            // exact compressed size first
            let full = unsafe { cb(size) } + 64;
            let mut tmp = vec![0u8; full];
            let n = unsafe { cc(tmp.as_mut_ptr(), full, src.as_ptr(), size, 3) };
            assert!(!is_error(n));
            // every interesting capacity around the real size
            let mut caps: Vec<usize> = vec![0, 1, 2, 3, 4, 5, 6, 8, 12, 17, 18];
            for d in 1..=6 {
                if n >= d {
                    caps.push(n - d);
                }
            }
            caps.push(n);
            caps.push(n + 1);
            caps.retain(|&c| c <= full);
            for &cap in &caps {
                let mut a = vec![0u8; cap.max(1)];
                let mut b = vec![0u8; cap.max(1)];
                let x = unsafe { cc(a.as_mut_ptr(), cap, src.as_ptr(), size, 3) };
                let y = unsafe { rc(b.as_mut_ptr(), cap, src.as_ptr(), size, 3) };
                assert_eq!(x, y, "compress(cap={cap}, {shape:?}/{size}) return");
                if !is_error(x) {
                    assert_bytes_eq("compress bytes", &a[..x], &b[..y]);
                }
                // and the same through compressCCtx / compress2 (+ checksum variant)
                let cctx = unsafe { cn() };
                let rctx = unsafe { rn() };
                let x = unsafe { ccx(cctx, a.as_mut_ptr(), cap, src.as_ptr(), size, 3) };
                let y = unsafe { rcx(rctx, b.as_mut_ptr(), cap, src.as_ptr(), size, 3) };
                assert_eq!(x, y, "compressCCtx(cap={cap}) return");
                assert_eq!(unsafe { cs(cctx, C_CHECKSUMFLAG, 1) },
                           unsafe { rs(rctx, C_CHECKSUMFLAG, 1) });
                let x = unsafe { c2(cctx, a.as_mut_ptr(), cap, src.as_ptr(), size) };
                let y = unsafe { r2(rctx, b.as_mut_ptr(), cap, src.as_ptr(), size) };
                assert_eq!(x, y, "compress2(cap={cap}, checksum) return");
                if !is_error(x) {
                    assert_bytes_eq("compress2 bytes", &a[..x], &b[..y]);
                }
                unsafe {
                    cf(cctx);
                    rf(rctx);
                }
            }
        }
    }
}

// ------------------------------------------------------------------ row 15 --

#[test]
fn err_compress_advanced_params() {
    let (ca, ra) = unsafe { pair::<FnCompressAdv>("ZSTD_compress_advanced") };
    let (cn, rn) = unsafe { pair::<FnNew>("ZSTD_createCCtx") };
    let (cf, rf) = unsafe { pair::<FnFree>("ZSTD_freeCCtx") };
    let (cgp, rgp) = unsafe { pair::<FnGetParams>("ZSTD_getParams") };
    let (cb, _) = unsafe { pair::<FnBound>("ZSTD_compressBound") };
    let mut rng = Rng::new(0xE002);
    let src = gen(Shape::Text, 5000, &mut rng);
    let cap = unsafe { cb(src.len()) } + 64;
    let cctx = unsafe { cn() };
    let rctx = unsafe { rn() };

    let base_c = unsafe { cgp(5, src.len() as u64, 0) };
    let base_r = unsafe { rgp(5, src.len() as u64, 0) };
    assert_eq!(base_c, base_r);
    let mutate: &[(&str, fn(&mut Parameters))] = &[
        ("windowLog=9", |p| p.cParams.windowLog = 9),
        ("windowLog=32", |p| p.cParams.windowLog = 32),
        ("chainLog=5", |p| p.cParams.chainLog = 5),
        ("chainLog=31", |p| p.cParams.chainLog = 31),
        ("hashLog=5", |p| p.cParams.hashLog = 5),
        ("hashLog=31", |p| p.cParams.hashLog = 31),
        ("searchLog=0", |p| p.cParams.searchLog = 0),
        ("searchLog=31", |p| p.cParams.searchLog = 31),
        ("minMatch=2", |p| p.cParams.minMatch = 2),
        ("minMatch=8", |p| p.cParams.minMatch = 8),
        ("targetLength=131073", |p| p.cParams.targetLength = 131073),
        ("strategy=0", |p| p.cParams.strategy = 0),
        ("strategy=10", |p| p.cParams.strategy = 10),
        ("strategy=-1", |p| p.cParams.strategy = -1),
    ];
    for (name, m) in mutate {
        let mut p = base_c;
        m(&mut p);
        let mut a = vec![0u8; cap];
        let mut b = vec![0u8; cap];
        let x = unsafe {
            ca(cctx, a.as_mut_ptr(), cap, src.as_ptr(), src.len(), std::ptr::null(), 0, p)
        };
        let y = unsafe {
            ra(rctx, b.as_mut_ptr(), cap, src.as_ptr(), src.len(), std::ptr::null(), 0, p)
        };
        assert_eq!(x, y, "compress_advanced({name})");
        if !is_error(x) {
            assert_bytes_eq(name, &a[..x], &b[..y]);
        }
    }
    unsafe {
        cf(cctx);
        rf(rctx);
    }
}

// ------------------------------------------------------------- rows 16-21 ---

/// Build a "full" dictionary: magic + dictID + (possibly corrupt) tables.
fn corrupt_full_dicts(rng: &mut Rng) -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    // trained dict as the valid baseline
    let trained = train_dict(rng);
    out.push(("valid trained".into(), trained.clone()));
    for n in [0usize, 1, 4, 7, 8, 9, 12, 16, 32, 64, 100] {
        if n <= trained.len() {
            out.push((format!("trained truncated to {n}"), trained[..n].to_vec()));
        }
    }
    for pos in [4, 5, 6, 7, 8, 9, 10, 12, 20, 40, 80] {
        if pos < trained.len() {
            let mut d = trained.clone();
            d[pos] ^= 0xFF;
            out.push((format!("trained byte {pos} flipped"), d));
        }
    }
    // magic + garbage
    for n in [8usize, 16, 64, 300] {
        let mut d = MAGIC_DICTIONARY.to_le_bytes().to_vec();
        d.extend(gen(Shape::Random, n, rng));
        out.push((format!("magic+random{n}"), d));
    }
    // magic + zeros
    for n in [8usize, 64, 300] {
        let mut d = MAGIC_DICTIONARY.to_le_bytes().to_vec();
        d.extend(std::iter::repeat(0u8).take(n));
        out.push((format!("magic+zeros{n}"), d));
    }
    // raw content dicts
    out.push(("raw small".into(), gen(Shape::Text, 5, rng)));
    out.push(("raw 8k".into(), gen(Shape::Text, 8192, rng)));
    out.push(("empty".into(), Vec::new()));
    out
}

fn train_dict(rng: &mut Rng) -> Vec<u8> {
    type FnTrain = unsafe extern "C" fn(*mut u8, usize, *const u8, *const usize, c_uint) -> usize;
    let (ct, rt) = unsafe { pair::<FnTrain>("ZDICT_trainFromBuffer") };
    let nb = 300;
    let mut samples = Vec::new();
    let mut sizes = Vec::new();
    for _ in 0..nb {
        let s = gen(Shape::Text, rng.range(64, 400), rng);
        sizes.push(s.len());
        samples.extend_from_slice(&s);
    }
    let cap = 8192;
    let mut a = vec![0u8; cap];
    let mut b = vec![0u8; cap];
    let x = unsafe { ct(a.as_mut_ptr(), cap, samples.as_ptr(), sizes.as_ptr(), nb as c_uint) };
    let y = unsafe { rt(b.as_mut_ptr(), cap, samples.as_ptr(), sizes.as_ptr(), nb as c_uint) };
    assert_eq!(x, y, "ZDICT_trainFromBuffer");
    assert!(!is_error(x));
    assert_bytes_eq("trained dict", &a[..x], &b[..y]);
    a.truncate(x);
    a
}

#[test]
fn err_dict_fullDict_small() {
    let (cla, rla) = unsafe { pair::<FnLoadDictAdv>("ZSTD_CCtx_loadDictionary_advanced") };
    let (cn, rn) = unsafe { pair::<FnNew>("ZSTD_createCCtx") };
    let (cf, rf) = unsafe { pair::<FnFree>("ZSTD_freeCCtx") };
    let mut rng = Rng::new(0xE003);
    for &n in &[0usize, 1, 2, 4, 7] {
        let d = gen(Shape::Random, n, &mut rng);
        for dlm in 0..2 {
            let cctx = unsafe { cn() };
            let rctx = unsafe { rn() };
            let x = unsafe { cla(cctx, d.as_ptr(), n, dlm, 2) };
            let y = unsafe { rla(rctx, d.as_ptr(), n, dlm, 2) };
            assert_eq!(x, y, "loadDictionary_advanced(fullDict, n={n}, dlm={dlm})");
            unsafe {
                cf(cctx);
                rf(rctx);
            }
        }
    }
}

#[test]
fn err_dict_fullDict_on_raw() {
    let (cla, rla) = unsafe { pair::<FnLoadDictAdv>("ZSTD_CCtx_loadDictionary_advanced") };
    let (cdla, rdla) = unsafe { pair::<FnLoadDictAdv>("ZSTD_DCtx_loadDictionary_advanced") };
    let (cn, rn) = unsafe { pair::<FnNew>("ZSTD_createCCtx") };
    let (cf, rf) = unsafe { pair::<FnFree>("ZSTD_freeCCtx") };
    let (cdn, rdn) = unsafe { pair::<FnNew>("ZSTD_createDCtx") };
    let (cdf, rdf) = unsafe { pair::<FnFree>("ZSTD_freeDCtx") };
    let (cca, rca) = unsafe { pair::<FnCreateCDictAdv>("ZSTD_createCDict_advanced") };
    let (cfd, rfd) = unsafe { pair::<FnFreeDict>("ZSTD_freeCDict") };
    let (cda, rda) = unsafe { pair::<FnCreateDDictAdv>("ZSTD_createDDict_advanced") };
    let (cfdd, rfdd) = unsafe { pair::<FnFreeDict>("ZSTD_freeDDict") };
    let (cgc, rgc) = unsafe { pair::<FnGetCParams>("ZSTD_getCParams") };
    let mut rng = Rng::new(0xE004);
    let nomem = CustomMem { alloc: None, free: None, opaque: std::ptr::null_mut() };

    for (name, d) in corrupt_full_dicts(&mut rng) {
        for dct in 0..3 {
            for dlm in 0..2 {
                let cctx = unsafe { cn() };
                let rctx = unsafe { rn() };
                let x = unsafe { cla(cctx, d.as_ptr(), d.len(), dlm, dct) };
                let y = unsafe { rla(rctx, d.as_ptr(), d.len(), dlm, dct) };
                assert_eq!(x, y, "CCtx_loadDictionary_advanced({name}, dlm={dlm}, dct={dct})");
                unsafe {
                    cf(cctx);
                    rf(rctx);
                }
                let cd = unsafe { cdn() };
                let rd = unsafe { rdn() };
                let x = unsafe { cdla(cd, d.as_ptr(), d.len(), dlm, dct) };
                let y = unsafe { rdla(rd, d.as_ptr(), d.len(), dlm, dct) };
                assert_eq!(x, y, "DCtx_loadDictionary_advanced({name}, dlm={dlm}, dct={dct})");
                unsafe {
                    cdf(cd);
                    rdf(rd);
                }
                let cp = unsafe { cgc(5, 100000, d.len()) };
                let rp = unsafe { rgc(5, 100000, d.len()) };
                assert_eq!(cp, rp);
                let a = unsafe { cca(d.as_ptr(), d.len(), dlm, dct, cp, nomem) };
                let b = unsafe { rca(d.as_ptr(), d.len(), dlm, dct, rp, nomem) };
                assert_eq!(
                    a.is_null(),
                    b.is_null(),
                    "createCDict_advanced({name}, dlm={dlm}, dct={dct}) null-ness"
                );
                unsafe {
                    if !a.is_null() {
                        cfd(a);
                    }
                    if !b.is_null() {
                        rfd(b);
                    }
                }
                let a = unsafe { cda(d.as_ptr(), d.len(), dlm, dct, nomem) };
                let b = unsafe { rda(d.as_ptr(), d.len(), dlm, dct, nomem) };
                assert_eq!(
                    a.is_null(),
                    b.is_null(),
                    "createDDict_advanced({name}, dlm={dlm}, dct={dct}) null-ness"
                );
                unsafe {
                    if !a.is_null() {
                        cfdd(a);
                    }
                    if !b.is_null() {
                        rfdd(b);
                    }
                }
            }
        }
    }
}

#[test]
fn err_dict_corrupted_variants() {
    let (cld, rld) = unsafe { pair::<FnLoadDict>("ZSTD_CCtx_loadDictionary") };
    let (cn, rn) = unsafe { pair::<FnNew>("ZSTD_createCCtx") };
    let (cf, rf) = unsafe { pair::<FnFree>("ZSTD_freeCCtx") };
    let (c2, r2) = unsafe { pair::<FnCompress2>("ZSTD_compress2") };
    let (ccd, rcd) = unsafe { pair::<FnCreateCDict>("ZSTD_createCDict") };
    let (cfd, rfd) = unsafe { pair::<FnFreeDict>("ZSTD_freeCDict") };
    let (cdd, rdd) = unsafe { pair::<FnCreateDDict>("ZSTD_createDDict") };
    let (cfdd, rfdd) = unsafe { pair::<FnFreeDict>("ZSTD_freeDDict") };
    let (cbd, rbd) = unsafe { pair::<FnLoadDict>("ZSTD_decompressBegin_usingDict") };
    let (cdn, rdn) = unsafe { pair::<FnNew>("ZSTD_createDCtx") };
    let (cdf, rdf) = unsafe { pair::<FnFree>("ZSTD_freeDCtx") };
    let (cb, _) = unsafe { pair::<FnBound>("ZSTD_compressBound") };
    let mut rng = Rng::new(0xE005);
    let src = gen(Shape::Text, 3000, &mut rng);
    let cap = unsafe { cb(src.len()) } + 64;

    for (name, d) in corrupt_full_dicts(&mut rng) {
        let cctx = unsafe { cn() };
        let rctx = unsafe { rn() };
        let x = unsafe { cld(cctx, d.as_ptr(), d.len()) };
        let y = unsafe { rld(rctx, d.as_ptr(), d.len()) };
        assert_eq!(x, y, "CCtx_loadDictionary({name})");
        if !is_error(x) {
            let mut a = vec![0u8; cap];
            let mut b = vec![0u8; cap];
            let x = unsafe { c2(cctx, a.as_mut_ptr(), cap, src.as_ptr(), src.len()) };
            let y = unsafe { r2(rctx, b.as_mut_ptr(), cap, src.as_ptr(), src.len()) };
            assert_eq!(x, y, "compress2 after loadDictionary({name})");
            if !is_error(x) {
                assert_bytes_eq(&name, &a[..x], &b[..y]);
            }
        }
        unsafe {
            cf(cctx);
            rf(rctx);
        }
        for &lvl in &[1, 9] {
            let a = unsafe { ccd(d.as_ptr(), d.len(), lvl) };
            let b = unsafe { rcd(d.as_ptr(), d.len(), lvl) };
            assert_eq!(a.is_null(), b.is_null(), "createCDict({name}, lvl={lvl})");
            unsafe {
                if !a.is_null() {
                    cfd(a);
                }
                if !b.is_null() {
                    rfd(b);
                }
            }
        }
        let a = unsafe { cdd(d.as_ptr(), d.len()) };
        let b = unsafe { rdd(d.as_ptr(), d.len()) };
        assert_eq!(a.is_null(), b.is_null(), "createDDict({name})");
        unsafe {
            if !a.is_null() {
                cfdd(a);
            }
            if !b.is_null() {
                rfdd(b);
            }
        }
        let cd = unsafe { cdn() };
        let rd = unsafe { rdn() };
        let x = unsafe { cbd(cd, d.as_ptr(), d.len()) };
        let y = unsafe { rbd(rd, d.as_ptr(), d.len()) };
        assert_eq!(x, y, "decompressBegin_usingDict({name})");
        unsafe {
            cdf(cd);
            rdf(rd);
        }
    }
}

// ------------------------------------------------------- rows 34-73 (params) -

#[test]
fn err_cparam_unknown() {
    let (cn, rn) = unsafe { pair::<FnNew>("ZSTD_createCCtx") };
    let (cf, rf) = unsafe { pair::<FnFree>("ZSTD_freeCCtx") };
    let (cs, rs) = unsafe { pair::<FnSetParam>("ZSTD_CCtx_setParameter") };
    let (cg, rg) = unsafe { pair::<FnGetParam>("ZSTD_CCtx_getParameter") };
    let (cpn, rpn) = unsafe { pair::<FnNew>("ZSTD_createCCtxParams") };
    let (cpf, rpf) = unsafe { pair::<FnFree>("ZSTD_freeCCtxParams") };
    let (cps, rps) = unsafe { pair::<FnSetParam>("ZSTD_CCtxParams_setParameter") };
    let (cpg, rpg) = unsafe { pair::<FnGetParam>("ZSTD_CCtxParams_getParameter") };

    let cctx = unsafe { cn() };
    let rctx = unsafe { rn() };
    let cp = unsafe { cpn() };
    let rp = unsafe { rpn() };
    let bogus: &[c_int] = &[
        i32::MIN, -1000, -1, 0, 1, 2, 9, 11, 12, 99, 108, 109, 129, 131, 159, 165, 199, 203, 299,
        399, 403, 499, 501, 999, 1003, 1018, 1019, 100000, i32::MAX,
    ];
    for &p in bogus {
        for &v in &[0, 1, -1, 12345] {
            assert_eq!(
                unsafe { cs(cctx, p, v) },
                unsafe { rs(rctx, p, v) },
                "CCtx_setParameter({p},{v})"
            );
            assert_eq!(
                unsafe { cps(cp, p, v) },
                unsafe { rps(rp, p, v) },
                "CCtxParams_setParameter({p},{v})"
            );
        }
        let mut a = 0;
        let mut b = 0;
        assert_eq!(
            unsafe { cg(cctx, p, &mut a) },
            unsafe { rg(rctx, p, &mut b) },
            "CCtx_getParameter({p})"
        );
        let mut a = 0;
        let mut b = 0;
        assert_eq!(
            unsafe { cpg(cp, p, &mut a) },
            unsafe { rpg(rp, p, &mut b) },
            "CCtxParams_getParameter({p})"
        );
    }
    unsafe {
        cf(cctx);
        rf(rctx);
        cpf(cp);
        rpf(rp);
    }
}

#[test]
fn err_cparam_out_of_bound() {
    #[repr(C)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct Bounds {
        error: usize,
        lo: c_int,
        hi: c_int,
    }
    type FnGetBounds = unsafe extern "C" fn(c_int) -> Bounds;
    let (cgb, _) = unsafe { pair::<FnGetBounds>("ZSTD_cParam_getBounds") };
    let (cn, rn) = unsafe { pair::<FnNew>("ZSTD_createCCtx") };
    let (cf, rf) = unsafe { pair::<FnFree>("ZSTD_freeCCtx") };
    let (cs, rs) = unsafe { pair::<FnSetParam>("ZSTD_CCtx_setParameter") };
    let (cpn, rpn) = unsafe { pair::<FnNew>("ZSTD_createCCtxParams") };
    let (cpf, rpf) = unsafe { pair::<FnFree>("ZSTD_freeCCtxParams") };
    let (cps, rps) = unsafe { pair::<FnSetParam>("ZSTD_CCtxParams_setParameter") };

    let cctx = unsafe { cn() };
    let rctx = unsafe { rn() };
    let cp = unsafe { cpn() };
    let rp = unsafe { rpn() };
    for &(p, name) in CPARAMS {
        let b = unsafe { cgb(p) };
        assert_eq!(b.error, 0, "{name} bounds");
        let mut vals: Vec<c_int> = vec![
            b.lo.saturating_sub(1),
            b.hi.saturating_add(1),
            i32::MIN,
            i32::MAX,
            -1,
            0,
            1,
        ];
        vals.push(b.lo.saturating_sub(2));
        vals.push(b.hi.saturating_add(2));
        vals.dedup();
        for &v in &vals {
            let x = unsafe { cs(cctx, p, v) };
            let y = unsafe { rs(rctx, p, v) };
            assert_eq!(x, y, "CCtx_setParameter({name}={v})");
            let x = unsafe { cps(cp, p, v) };
            let y = unsafe { rps(rp, p, v) };
            assert_eq!(x, y, "CCtxParams_setParameter({name}={v})");
        }
    }
    unsafe {
        cf(cctx);
        rf(rctx);
        cpf(cp);
        rpf(rp);
    }
}

#[test]
fn err_mt_params_unsupported() {
    let (cn, rn) = unsafe { pair::<FnNew>("ZSTD_createCCtx") };
    let (cf, rf) = unsafe { pair::<FnFree>("ZSTD_freeCCtx") };
    let (cs, rs) = unsafe { pair::<FnSetParam>("ZSTD_CCtx_setParameter") };
    let (cg, rg) = unsafe { pair::<FnGetParam>("ZSTD_CCtx_getParameter") };
    let (cpn, rpn) = unsafe { pair::<FnNew>("ZSTD_createCCtxParams") };
    let (cpf, rpf) = unsafe { pair::<FnFree>("ZSTD_freeCCtxParams") };
    let (cps, rps) = unsafe { pair::<FnSetParam>("ZSTD_CCtxParams_setParameter") };
    let (cpg, rpg) = unsafe { pair::<FnGetParam>("ZSTD_CCtxParams_getParameter") };
    let cctx = unsafe { cn() };
    let rctx = unsafe { rn() };
    let cp = unsafe { cpn() };
    let rp = unsafe { rpn() };
    for p in [400, 401, 402, 500] {
        for v in [0, 1, 2, 4, 1 << 20, -1] {
            assert_eq!(
                unsafe { cs(cctx, p, v) },
                unsafe { rs(rctx, p, v) },
                "setParameter({p},{v})"
            );
            assert_eq!(
                unsafe { cps(cp, p, v) },
                unsafe { rps(rp, p, v) },
                "CCtxParams_setParameter({p},{v})"
            );
        }
        let mut a = 0;
        let mut b = 0;
        assert_eq!(unsafe { cg(cctx, p, &mut a) }, unsafe { rg(rctx, p, &mut b) },
                   "getParameter({p})");
        assert_eq!(a, b);
        let mut a = 0;
        let mut b = 0;
        assert_eq!(unsafe { cpg(cp, p, &mut a) }, unsafe { rpg(rp, p, &mut b) },
                   "CCtxParams_getParameter({p})");
    }
    unsafe {
        cf(cctx);
        rf(rctx);
        cpf(cp);
        rpf(rp);
    }
}

/// Rows 36, 74-77, 80-82, 87: every "must be called before compression starts"
/// entry point, invoked mid-frame.
#[test]
fn err_setparam_stage_wrong() {
    let (cn, rn) = unsafe { pair::<FnNew>("ZSTD_createCCtx") };
    let (cf, rf) = unsafe { pair::<FnFree>("ZSTD_freeCCtx") };
    let (cs, rs) = unsafe { pair::<FnSetParam>("ZSTD_CCtx_setParameter") };
    let (c2, r2) = unsafe { pair::<FnStream2>("ZSTD_compressStream2") };
    let (cpl, rpl) = unsafe { pair::<FnPledged>("ZSTD_CCtx_setPledgedSrcSize") };
    let (cld, rld) = unsafe { pair::<FnLoadDict>("ZSTD_CCtx_loadDictionary") };
    let (clda, rlda) = unsafe { pair::<FnLoadDictAdv>("ZSTD_CCtx_loadDictionary_advanced") };
    let (crc, rrc) = unsafe { pair::<FnRefDict>("ZSTD_CCtx_refCDict") };
    let (crp, rrp) = unsafe { pair::<FnLoadDict>("ZSTD_CCtx_refPrefix") };
    let (crpa, rrpa) = unsafe { pair::<FnRefPrefixAdv>("ZSTD_CCtx_refPrefix_advanced") };
    let (cup, rup) = unsafe { pair::<FnUseParams>("ZSTD_CCtx_setParametersUsingCCtxParams") };
    let (ccp, rcp) = unsafe { pair::<FnSetCParams>("ZSTD_CCtx_setCParams") };
    let (cfp, rfp) = unsafe { pair::<FnSetFParams>("ZSTD_CCtx_setFParams") };
    let (csp, rsp) = unsafe { pair::<FnSetParams>("ZSTD_CCtx_setParams") };
    let (crtp, rrtp) = unsafe { pair::<FnRefThreadPool>("ZSTD_CCtx_refThreadPool") };
    let (cgp, rgp) = unsafe { pair::<FnGetParams>("ZSTD_getParams") };
    let (cpn, rpn) = unsafe { pair::<FnNew>("ZSTD_createCCtxParams") };
    let (cpf, rpf) = unsafe { pair::<FnFree>("ZSTD_freeCCtxParams") };
    let (ccd, rcd) = unsafe { pair::<FnCreateCDict>("ZSTD_createCDict") };
    let (cfd, rfd) = unsafe { pair::<FnFreeDict>("ZSTD_freeCDict") };

    let mut rng = Rng::new(0xE006);
    let dict = gen(Shape::Text, 4096, &mut rng);
    let src = gen(Shape::Text, 200000, &mut rng);
    let cdict = unsafe { ccd(dict.as_ptr(), dict.len(), 3) };
    let rdict = unsafe { rcd(dict.as_ptr(), dict.len(), 3) };
    let cparams = unsafe { cpn() };
    let rparams = unsafe { rpn() };

    let cctx = unsafe { cn() };
    let rctx = unsafe { rn() };
    // start a frame but do not finish it
    let mut obuf = vec![0u8; 4096];
    for which in 0..2 {
        let z = if which == 0 { cctx } else { rctx };
        let mut input = InBuffer { src: src.as_ptr(), size: src.len(), pos: 0 };
        let mut out = OutBuffer { dst: obuf.as_mut_ptr(), size: obuf.len(), pos: 0 };
        let rc = if which == 0 {
            unsafe { c2(z, &mut out, &mut input, E_CONTINUE) }
        } else {
            unsafe { r2(z, &mut out, &mut input, E_CONTINUE) }
        };
        assert!(!is_error(rc));
    }
    let params_c = unsafe { cgp(5, 1000, 0) };
    let params_r = unsafe { rgp(5, 1000, 0) };
    assert_eq!(params_c, params_r);

    // every one of these must fail with the same code mid-frame
    macro_rules! chk {
        ($label:expr, $c:expr, $r:expr) => {{
            let x = unsafe { $c };
            let y = unsafe { $r };
            assert_eq!(x, y, "mid-frame {}", $label);
            assert!(is_error(x), "mid-frame {} should fail in C (got {})", $label, x);
        }};
    }
    chk!("setParameter(windowLog)", cs(cctx, C_WINDOWLOG, 20), rs(rctx, C_WINDOWLOG, 20));
    chk!("setPledgedSrcSize", cpl(cctx, 100), rpl(rctx, 100));
    chk!(
        "loadDictionary",
        cld(cctx, dict.as_ptr(), dict.len()),
        rld(rctx, dict.as_ptr(), dict.len())
    );
    chk!(
        "loadDictionary_advanced",
        clda(cctx, dict.as_ptr(), dict.len(), 0, 0),
        rlda(rctx, dict.as_ptr(), dict.len(), 0, 0)
    );
    chk!("refCDict", crc(cctx, cdict), rrc(rctx, rdict));
    chk!(
        "refPrefix",
        crp(cctx, dict.as_ptr(), dict.len()),
        rrp(rctx, dict.as_ptr(), dict.len())
    );
    chk!(
        "refPrefix_advanced",
        crpa(cctx, dict.as_ptr(), dict.len(), 0),
        rrpa(rctx, dict.as_ptr(), dict.len(), 0)
    );
    chk!("setParametersUsingCCtxParams", cup(cctx, cparams), rup(rctx, rparams));
    chk!("setCParams", ccp(cctx, params_c.cParams), rcp(rctx, params_r.cParams));
    chk!("setFParams", cfp(cctx, params_c.fParams), rfp(rctx, params_r.fParams));
    chk!("setParams", csp(cctx, params_c), rsp(rctx, params_r));
    chk!(
        "refThreadPool",
        crtp(cctx, std::ptr::null_mut()),
        rrtp(rctx, std::ptr::null_mut())
    );

    // a cdict attached to a fresh context also blocks setParametersUsingCCtxParams
    let cctx2 = unsafe { cn() };
    let rctx2 = unsafe { rn() };
    assert_eq!(unsafe { crc(cctx2, cdict) }, unsafe { rrc(rctx2, rdict) });
    let x = unsafe { cup(cctx2, cparams) };
    let y = unsafe { rup(rctx2, rparams) };
    assert_eq!(x, y, "setParametersUsingCCtxParams with cdict attached");

    unsafe {
        cf(cctx);
        rf(rctx);
        cf(cctx2);
        rf(rctx2);
        cpf(cparams);
        rpf(rparams);
        cfd(cdict);
        rfd(rdict);
    }
}

#[test]
fn err_ccctxparams_null() {
    let (ci, ri) = unsafe { pair::<FnCParamsInit>("ZSTD_CCtxParams_init") };
    let (cia, ria) = unsafe { pair::<FnCParamsInitAdv>("ZSTD_CCtxParams_init_advanced") };
    let (cr, rr) = unsafe { pair::<FnCParamsReset>("ZSTD_CCtxParams_reset") };
    let (cgp, rgp) = unsafe { pair::<FnGetParams>("ZSTD_getParams") };
    let (cpn, rpn) = unsafe { pair::<FnNew>("ZSTD_createCCtxParams") };
    let (cpf, rpf) = unsafe { pair::<FnFree>("ZSTD_freeCCtxParams") };

    for lvl in [0, 3, 19] {
        assert_eq!(
            unsafe { ci(std::ptr::null_mut(), lvl) },
            unsafe { ri(std::ptr::null_mut(), lvl) },
            "CCtxParams_init(NULL,{lvl})"
        );
    }
    assert_eq!(
        unsafe { cr(std::ptr::null_mut()) },
        unsafe { rr(std::ptr::null_mut()) },
        "CCtxParams_reset(NULL)"
    );
    let p_c = unsafe { cgp(5, 1000, 0) };
    let p_r = unsafe { rgp(5, 1000, 0) };
    assert_eq!(
        unsafe { cia(std::ptr::null_mut(), p_c) },
        unsafe { ria(std::ptr::null_mut(), p_r) },
        "CCtxParams_init_advanced(NULL)"
    );
    // invalid cParams through init_advanced
    let cp = unsafe { cpn() };
    let rp = unsafe { rpn() };
    for bad in [
        CParams { windowLog: 9, chainLog: 10, hashLog: 10, searchLog: 1, minMatch: 4,
                  targetLength: 0, strategy: 1 },
        CParams { windowLog: 32, chainLog: 10, hashLog: 10, searchLog: 1, minMatch: 4,
                  targetLength: 0, strategy: 1 },
        CParams { windowLog: 20, chainLog: 10, hashLog: 10, searchLog: 1, minMatch: 4,
                  targetLength: 0, strategy: 99 },
    ] {
        let mut p = p_c;
        p.cParams = bad;
        assert_eq!(
            unsafe { cia(cp, p) },
            unsafe { ria(rp, p) },
            "CCtxParams_init_advanced(bad {bad:?})"
        );
    }
    unsafe {
        cpf(cp);
        rpf(rp);
    }
}

#[test]
fn err_reset_parameters_midframe() {
    let (cn, rn) = unsafe { pair::<FnNew>("ZSTD_createCCtx") };
    let (cf, rf) = unsafe { pair::<FnFree>("ZSTD_freeCCtx") };
    let (c2, r2) = unsafe { pair::<FnStream2>("ZSTD_compressStream2") };
    let (crst, rrst) = unsafe { pair::<FnReset>("ZSTD_CCtx_reset") };
    let mut rng = Rng::new(0xE007);
    let src = gen(Shape::Text, 200000, &mut rng);
    for &kind in &[1, 2, 3] {
        let cctx = unsafe { cn() };
        let rctx = unsafe { rn() };
        let mut obuf = vec![0u8; 2048];
        for which in 0..2 {
            let z = if which == 0 { cctx } else { rctx };
            let mut input = InBuffer { src: src.as_ptr(), size: src.len(), pos: 0 };
            let mut out = OutBuffer { dst: obuf.as_mut_ptr(), size: obuf.len(), pos: 0 };
            let rc = if which == 0 {
                unsafe { c2(z, &mut out, &mut input, E_CONTINUE) }
            } else {
                unsafe { r2(z, &mut out, &mut input, E_CONTINUE) }
            };
            assert!(!is_error(rc));
        }
        let x = unsafe { crst(cctx, kind) };
        let y = unsafe { rrst(rctx, kind) };
        assert_eq!(x, y, "CCtx_reset({kind}) mid-frame");
        unsafe {
            cf(cctx);
            rf(rctx);
        }
    }
}

#[test]
fn err_reset_bogus_enum() {
    let (cn, rn) = unsafe { pair::<FnNew>("ZSTD_createCCtx") };
    let (cf, rf) = unsafe { pair::<FnFree>("ZSTD_freeCCtx") };
    let (crst, rrst) = unsafe { pair::<FnReset>("ZSTD_CCtx_reset") };
    let (cdn, rdn) = unsafe { pair::<FnNew>("ZSTD_createDCtx") };
    let (cdf, rdf) = unsafe { pair::<FnFree>("ZSTD_freeDCtx") };
    let (cdr, rdr) = unsafe { pair::<FnReset>("ZSTD_DCtx_reset") };
    let cctx = unsafe { cn() };
    let rctx = unsafe { rn() };
    let cd = unsafe { cdn() };
    let rd = unsafe { rdn() };
    for kind in [i32::MIN, -1, 0, 4, 5, 99, i32::MAX] {
        assert_eq!(
            unsafe { crst(cctx, kind) },
            unsafe { rrst(rctx, kind) },
            "CCtx_reset(bogus {kind})"
        );
        assert_eq!(
            unsafe { cdr(cd, kind) },
            unsafe { rdr(rd, kind) },
            "DCtx_reset(bogus {kind})"
        );
    }
    unsafe {
        cf(cctx);
        rf(rctx);
        cdf(cd);
        rdf(rd);
    }
}

#[test]
fn err_checkcparams() {
    let (cc, rc) = unsafe { pair::<FnCheckCParams>("ZSTD_checkCParams") };
    let (ccp, rcp) = unsafe { pair::<FnSetCParams>("ZSTD_CCtx_setCParams") };
    let (csp, rsp) = unsafe { pair::<FnSetParams>("ZSTD_CCtx_setParams") };
    let (cn, rn) = unsafe { pair::<FnNew>("ZSTD_createCCtx") };
    let (cf, rf) = unsafe { pair::<FnFree>("ZSTD_freeCCtx") };
    let (cgp, rgp) = unsafe { pair::<FnGetParams>("ZSTD_getParams") };
    let cctx = unsafe { cn() };
    let rctx = unsafe { rn() };
    let base = unsafe { cgp(5, 1000, 0) };
    assert_eq!(base, unsafe { rgp(5, 1000, 0) });
    let mut bads = Vec::new();
    for wl in [0u32, 9, 10, 31, 32, 1000, u32::MAX] {
        let mut p = base.cParams;
        p.windowLog = wl;
        bads.push(p);
    }
    for cl in [0u32, 5, 6, 30, 31, u32::MAX] {
        let mut p = base.cParams;
        p.chainLog = cl;
        bads.push(p);
    }
    for hl in [0u32, 5, 6, 30, 31, u32::MAX] {
        let mut p = base.cParams;
        p.hashLog = hl;
        bads.push(p);
    }
    for sl in [0u32, 1, 30, 31, u32::MAX] {
        let mut p = base.cParams;
        p.searchLog = sl;
        bads.push(p);
    }
    for mm in [0u32, 2, 3, 7, 8, u32::MAX] {
        let mut p = base.cParams;
        p.minMatch = mm;
        bads.push(p);
    }
    for tl in [0u32, 131072, 131073, u32::MAX] {
        let mut p = base.cParams;
        p.targetLength = tl;
        bads.push(p);
    }
    for st in [-1, 0, 1, 9, 10, 99, i32::MAX, i32::MIN] {
        let mut p = base.cParams;
        p.strategy = st;
        bads.push(p);
    }
    for p in bads {
        assert_eq!(unsafe { cc(p) }, unsafe { rc(p) }, "checkCParams({p:?})");
        assert_eq!(unsafe { ccp(cctx, p) }, unsafe { rcp(rctx, p) }, "setCParams({p:?})");
        let mut full = base;
        full.cParams = p;
        assert_eq!(unsafe { csp(cctx, full) }, unsafe { rsp(rctx, full) }, "setParams({p:?})");
    }
    unsafe {
        cf(cctx);
        rf(rctx);
    }
}

// ------------------------------------------------------------ rows 88-98 ----

#[test]
fn err_copycctx_stage() {
    let (cn, rn) = unsafe { pair::<FnNew>("ZSTD_createCCtx") };
    let (cf, rf) = unsafe { pair::<FnFree>("ZSTD_freeCCtx") };
    let (cbg, rbg) = unsafe { pair::<FnBegin>("ZSTD_compressBegin") };
    let (ccont, rcont) = unsafe { pair::<FnChunk>("ZSTD_compressContinue") };
    let (ccp, rcp) = unsafe { pair::<FnCopyCCtx>("ZSTD_copyCCtx") };
    let (cb, _) = unsafe { pair::<FnBound>("ZSTD_compressBound") };
    let mut rng = Rng::new(0xE008);
    let src = gen(Shape::Text, 20000, &mut rng);

    // copy from a fresh (created, not begun) context
    let a = unsafe { cn() };
    let b = unsafe { rn() };
    let da = unsafe { cn() };
    let db = unsafe { rn() };
    assert_eq!(
        unsafe { ccp(da, a, src.len() as u64) },
        unsafe { rcp(db, b, src.len() as u64) },
        "copyCCtx from created context"
    );
    // begin, then copy (legal), then compress, then copy again (illegal)
    assert_eq!(unsafe { cbg(a, 5) }, unsafe { rbg(b, 5) });
    assert_eq!(
        unsafe { ccp(da, a, src.len() as u64) },
        unsafe { rcp(db, b, src.len() as u64) },
        "copyCCtx after begin"
    );
    let cap = unsafe { cb(src.len()) } + 64;
    let mut o1 = vec![0u8; cap];
    let mut o2 = vec![0u8; cap];
    let x = unsafe { ccont(a, o1.as_mut_ptr(), cap, src.as_ptr(), src.len()) };
    let y = unsafe { rcont(b, o2.as_mut_ptr(), cap, src.as_ptr(), src.len()) };
    assert_eq!(x, y);
    assert_bytes_eq("continue", &o1[..x], &o2[..y]);
    assert_eq!(
        unsafe { ccp(da, a, src.len() as u64) },
        unsafe { rcp(db, b, src.len() as u64) },
        "copyCCtx after compressContinue (must be stage_wrong)"
    );
    unsafe {
        cf(a);
        rf(b);
        cf(da);
        rf(db);
    }
}

#[test]
fn err_compressbegin_params() {
    let (cn, rn) = unsafe { pair::<FnNew>("ZSTD_createCCtx") };
    let (cf, rf) = unsafe { pair::<FnFree>("ZSTD_freeCCtx") };
    let (cba, rba) = unsafe { pair::<FnBeginAdv>("ZSTD_compressBegin_advanced") };
    let (cgp, rgp) = unsafe { pair::<FnGetParams>("ZSTD_getParams") };
    let cctx = unsafe { cn() };
    let rctx = unsafe { rn() };
    let base = unsafe { cgp(5, 1000, 0) };
    assert_eq!(base, unsafe { rgp(5, 1000, 0) });
    for (name, f) in [
        ("windowLog=9", 0),
        ("windowLog=32", 1),
        ("strategy=0", 2),
        ("strategy=10", 3),
        ("minMatch=2", 4),
    ] {
        let mut p = base;
        match f {
            0 => p.cParams.windowLog = 9,
            1 => p.cParams.windowLog = 32,
            2 => p.cParams.strategy = 0,
            3 => p.cParams.strategy = 10,
            _ => p.cParams.minMatch = 2,
        }
        assert_eq!(
            unsafe { cba(cctx, std::ptr::null(), 0, p, 1000) },
            unsafe { rba(rctx, std::ptr::null(), 0, p, 1000) },
            "compressBegin_advanced({name})"
        );
    }
    unsafe {
        cf(cctx);
        rf(rctx);
    }
}

#[test]
fn err_compresscontinue_stage() {
    let (cn, rn) = unsafe { pair::<FnNew>("ZSTD_createCCtx") };
    let (cf, rf) = unsafe { pair::<FnFree>("ZSTD_freeCCtx") };
    let (ccont, rcont) = unsafe { pair::<FnChunk>("ZSTD_compressContinue") };
    let (cend, rend) = unsafe { pair::<FnChunk>("ZSTD_compressEnd") };
    let (ccblk, rcblk) = unsafe { pair::<FnChunk>("ZSTD_compressBlock") };
    let (cgbs, rgbs) = unsafe { pair::<FnCtxSize>("ZSTD_getBlockSize") };
    let mut rng = Rng::new(0xE009);
    let src = gen(Shape::Text, 5000, &mut rng);
    let mut o = vec![0u8; 1 << 18];

    // fresh contexts: no compressBegin => stage_wrong
    let a = unsafe { cn() };
    let b = unsafe { rn() };
    assert_eq!(
        unsafe { ccont(a, o.as_mut_ptr(), o.len(), src.as_ptr(), src.len()) },
        unsafe { rcont(b, o.as_mut_ptr(), o.len(), src.as_ptr(), src.len()) },
        "compressContinue without begin"
    );
    assert_eq!(
        unsafe { cend(a, o.as_mut_ptr(), o.len(), src.as_ptr(), src.len()) },
        unsafe { rend(b, o.as_mut_ptr(), o.len(), src.as_ptr(), src.len()) },
        "compressEnd without begin"
    );
    assert_eq!(unsafe { cgbs(a) }, unsafe { rgbs(b) }, "getBlockSize on fresh ctx");
    assert_eq!(
        unsafe { ccblk(a, o.as_mut_ptr(), o.len(), src.as_ptr(), src.len()) },
        unsafe { rcblk(b, o.as_mut_ptr(), o.len(), src.as_ptr(), src.len()) },
        "compressBlock without begin"
    );
    unsafe {
        cf(a);
        rf(b);
    }
}

#[test]
fn err_compresscontinue_pledged() {
    let (cn, rn) = unsafe { pair::<FnNew>("ZSTD_createCCtx") };
    let (cf, rf) = unsafe { pair::<FnFree>("ZSTD_freeCCtx") };
    let (cba, rba) = unsafe { pair::<FnBeginAdv>("ZSTD_compressBegin_advanced") };
    let (ccont, rcont) = unsafe { pair::<FnChunk>("ZSTD_compressContinue") };
    let (cend, rend) = unsafe { pair::<FnChunk>("ZSTD_compressEnd") };
    let (cgp, rgp) = unsafe { pair::<FnGetParams>("ZSTD_getParams") };
    let mut rng = Rng::new(0xE00A);
    let src = gen(Shape::Text, 30000, &mut rng);
    let mut o = vec![0u8; 1 << 18];

    for &(pledged, feed) in &[
        (1000u64, 1001usize),
        (1000, 2000),
        (1000, 999),
        (0, 1),
        (30000, 30001),
    ] {
        let a = unsafe { cn() };
        let b = unsafe { rn() };
        let p_c = unsafe { cgp(3, pledged, 0) };
        let p_r = unsafe { rgp(3, pledged, 0) };
        assert_eq!(p_c, p_r);
        assert_eq!(
            unsafe { cba(a, std::ptr::null(), 0, p_c, pledged) },
            unsafe { rba(b, std::ptr::null(), 0, p_r, pledged) }
        );
        let n = feed.min(src.len());
        let x = unsafe { ccont(a, o.as_mut_ptr(), o.len(), src.as_ptr(), n) };
        let y = unsafe { rcont(b, o.as_mut_ptr(), o.len(), src.as_ptr(), n) };
        assert_eq!(x, y, "compressContinue(pledged={pledged}, feed={n})");
        // then finish with 0 bytes: reveals the "less than pledged" error
        let x = unsafe { cend(a, o.as_mut_ptr(), o.len(), src.as_ptr(), 0) };
        let y = unsafe { rend(b, o.as_mut_ptr(), o.len(), src.as_ptr(), 0) };
        assert_eq!(x, y, "compressEnd(pledged={pledged}, fed={n})");
        unsafe {
            cf(a);
            rf(b);
        }
    }
}

#[test]
fn err_block_too_large() {
    let (cn, rn) = unsafe { pair::<FnNew>("ZSTD_createCCtx") };
    let (cf, rf) = unsafe { pair::<FnFree>("ZSTD_freeCCtx") };
    let (cbg, rbg) = unsafe { pair::<FnBegin>("ZSTD_compressBegin") };
    let (cgbs, rgbs) = unsafe { pair::<FnCtxSize>("ZSTD_getBlockSize") };
    let (ccblk, rcblk) = unsafe { pair::<FnChunk>("ZSTD_compressBlock") };
    let (cs, rs) = unsafe { pair::<FnSetParam>("ZSTD_CCtx_setParameter") };
    let mut rng = Rng::new(0xE00B);
    let src = gen(Shape::Text, 300000, &mut rng);
    let mut o = vec![0u8; 1 << 19];

    for &maxbs in &[0, 1024, 65536, 131072] {
        let a = unsafe { cn() };
        let b = unsafe { rn() };
        if maxbs != 0 {
            assert_eq!(unsafe { cs(a, 1015, maxbs) }, unsafe { rs(b, 1015, maxbs) });
        }
        assert_eq!(unsafe { cbg(a, 3) }, unsafe { rbg(b, 3) });
        let bs_a = unsafe { cgbs(a) };
        let bs_b = unsafe { rgbs(b) };
        assert_eq!(bs_a, bs_b, "getBlockSize(maxbs={maxbs})");
        for n in [bs_a, bs_a + 1, bs_a + 2, bs_a * 2, src.len()] {
            if n > src.len() {
                continue;
            }
            let x = unsafe { ccblk(a, o.as_mut_ptr(), o.len(), src.as_ptr(), n) };
            let y = unsafe { rcblk(b, o.as_mut_ptr(), o.len(), src.as_ptr(), n) };
            assert_eq!(x, y, "compressBlock(n={n}, blockSize={bs_a})");
        }
        unsafe {
            cf(a);
            rf(b);
        }
    }
}

#[test]
fn err_null_cdict() {
    let (cn, rn) = unsafe { pair::<FnNew>("ZSTD_createCCtx") };
    let (cf, rf) = unsafe { pair::<FnFree>("ZSTD_freeCCtx") };
    let (cbc, rbc) = unsafe { pair::<FnBeginCDict>("ZSTD_compressBegin_usingCDict") };
    let (cbca, rbca) =
        unsafe { pair::<FnBeginCDictAdv>("ZSTD_compressBegin_usingCDict_advanced") };
    let (cuc, ruc) = unsafe { pair::<FnCompressUsingCDict>("ZSTD_compress_usingCDict") };
    let (cuca, ruca) =
        unsafe { pair::<FnCompressUsingCDictAdv>("ZSTD_compress_usingCDict_advanced") };
    let (crc, rrc) = unsafe { pair::<FnRefDict>("ZSTD_CCtx_refCDict") };
    let (cb, _) = unsafe { pair::<FnBound>("ZSTD_compressBound") };
    let mut rng = Rng::new(0xE00C);
    let src = gen(Shape::Text, 3000, &mut rng);
    let cap = unsafe { cb(src.len()) } + 64;
    let mut o = vec![0u8; cap];
    let a = unsafe { cn() };
    let b = unsafe { rn() };
    let null = std::ptr::null();
    let fp = FParams { contentSizeFlag: 1, checksumFlag: 0, noDictIDFlag: 0 };
    assert_eq!(unsafe { cbc(a, null) }, unsafe { rbc(b, null) }, "compressBegin_usingCDict(NULL)");
    assert_eq!(
        unsafe { cbca(a, null, fp, 0) },
        unsafe { rbca(b, null, fp, 0) },
        "compressBegin_usingCDict_advanced(NULL)"
    );
    assert_eq!(
        unsafe { cuc(a, o.as_mut_ptr(), cap, src.as_ptr(), src.len(), null) },
        unsafe { ruc(b, o.as_mut_ptr(), cap, src.as_ptr(), src.len(), null) },
        "compress_usingCDict(NULL)"
    );
    assert_eq!(
        unsafe { cuca(a, o.as_mut_ptr(), cap, src.as_ptr(), src.len(), null, fp) },
        unsafe { ruca(b, o.as_mut_ptr(), cap, src.as_ptr(), src.len(), null, fp) },
        "compress_usingCDict_advanced(NULL)"
    );
    // refCDict(NULL) is legal: it clears the dictionary
    assert_eq!(unsafe { crc(a, null) }, unsafe { rrc(b, null) }, "refCDict(NULL)");
    unsafe {
        cf(a);
        rf(b);
    }
}

#[test]
fn err_dictid_queries() {
    let (cfd, rfd) = unsafe { pair::<FnDictIDFromBuf>("ZSTD_getDictID_fromDict") };
    let (cff, rff) = unsafe { pair::<FnDictIDFromBuf>("ZSTD_getDictID_fromFrame") };
    let (cfc, rfc) = unsafe { pair::<FnDictIDFromObj>("ZSTD_getDictID_fromCDict") };
    let (cfdd, rfdd) = unsafe { pair::<FnDictIDFromObj>("ZSTD_getDictID_fromDDict") };
    let (ccd, rcd) = unsafe { pair::<FnCreateCDict>("ZSTD_createCDict") };
    let (cfree, rfree) = unsafe { pair::<FnFreeDict>("ZSTD_freeCDict") };
    let (cdd, rdd) = unsafe { pair::<FnCreateDDict>("ZSTD_createDDict") };
    let (cfree2, rfree2) = unsafe { pair::<FnFreeDict>("ZSTD_freeDDict") };
    let (cc, rc) = unsafe { pair::<FnCompress>("ZSTD_compress") };
    let (cb, _) = unsafe { pair::<FnBound>("ZSTD_compressBound") };
    let mut rng = Rng::new(0xE00D);

    assert_eq!(unsafe { cfc(std::ptr::null()) }, unsafe { rfc(std::ptr::null()) },
               "getDictID_fromCDict(NULL)");
    assert_eq!(unsafe { cfdd(std::ptr::null()) }, unsafe { rfdd(std::ptr::null()) },
               "getDictID_fromDDict(NULL)");
    for (name, d) in corrupt_full_dicts(&mut rng) {
        assert_eq!(
            unsafe { cfd(d.as_ptr(), d.len()) },
            unsafe { rfd(d.as_ptr(), d.len()) },
            "getDictID_fromDict({name})"
        );
        let a = unsafe { ccd(d.as_ptr(), d.len(), 3) };
        let b = unsafe { rcd(d.as_ptr(), d.len(), 3) };
        assert_eq!(a.is_null(), b.is_null());
        if !a.is_null() {
            assert_eq!(unsafe { cfc(a) }, unsafe { rfc(b) }, "getDictID_fromCDict({name})");
            unsafe {
                cfree(a);
                rfree(b);
            }
        }
        let a = unsafe { cdd(d.as_ptr(), d.len()) };
        let b = unsafe { rdd(d.as_ptr(), d.len()) };
        assert_eq!(a.is_null(), b.is_null());
        if !a.is_null() {
            assert_eq!(unsafe { cfdd(a) }, unsafe { rfdd(b) }, "getDictID_fromDDict({name})");
            unsafe {
                cfree2(a);
                rfree2(b);
            }
        }
    }
    // fromFrame on: plain frame, truncated frames, garbage
    let src = gen(Shape::Text, 5000, &mut rng);
    let cap = unsafe { cb(src.len()) } + 64;
    let mut f = vec![0u8; cap];
    let n = unsafe { cc(f.as_mut_ptr(), cap, src.as_ptr(), src.len(), 3) };
    let mut f2 = vec![0u8; cap];
    let n2 = unsafe { rc(f2.as_mut_ptr(), cap, src.as_ptr(), src.len(), 3) };
    assert_eq!(n, n2);
    f.truncate(n);
    for k in 0..=f.len().min(20) {
        assert_eq!(
            unsafe { cff(f.as_ptr(), k) },
            unsafe { rff(f.as_ptr(), k) },
            "getDictID_fromFrame(prefix {k})"
        );
    }
    let junk = gen(Shape::Random, 64, &mut rng);
    assert_eq!(
        unsafe { cff(junk.as_ptr(), junk.len()) },
        unsafe { rff(junk.as_ptr(), junk.len()) },
        "getDictID_fromFrame(junk)"
    );
}

#[test]
fn err_loaddict_null() {
    let (cld, rld) = unsafe { pair::<FnLoadDict>("ZSTD_CCtx_loadDictionary") };
    let (cldr, rldr) = unsafe { pair::<FnLoadDict>("ZSTD_CCtx_loadDictionary_byReference") };
    let (cdld, rdld) = unsafe { pair::<FnLoadDict>("ZSTD_DCtx_loadDictionary") };
    let (cn, rn) = unsafe { pair::<FnNew>("ZSTD_createCCtx") };
    let (cf, rf) = unsafe { pair::<FnFree>("ZSTD_freeCCtx") };
    let (cdn, rdn) = unsafe { pair::<FnNew>("ZSTD_createDCtx") };
    let (cdf, rdf) = unsafe { pair::<FnFree>("ZSTD_freeDCtx") };
    let a = unsafe { cn() };
    let b = unsafe { rn() };
    let da = unsafe { cdn() };
    let db = unsafe { rdn() };
    let dummy = [1u8, 2, 3, 4];
    for (name, ptr, len) in [
        ("NULL,0", std::ptr::null(), 0usize),
        ("NULL,100", std::ptr::null(), 100),
        ("ptr,0", dummy.as_ptr(), 0),
    ] {
        assert_eq!(unsafe { cld(a, ptr, len) }, unsafe { rld(b, ptr, len) },
                   "CCtx_loadDictionary({name})");
        assert_eq!(unsafe { cldr(a, ptr, len) }, unsafe { rldr(b, ptr, len) },
                   "CCtx_loadDictionary_byReference({name})");
        assert_eq!(unsafe { cdld(da, ptr, len) }, unsafe { rdld(db, ptr, len) },
                   "DCtx_loadDictionary({name})");
    }
    unsafe {
        cf(a);
        rf(b);
        cdf(da);
        rdf(db);
    }
}

// ------------------------------------------------------- rows 107-114, 118 --

#[test]
fn err_stream_bad_buffers() {
    let (cn, rn) = unsafe { pair::<FnNew>("ZSTD_createCCtx") };
    let (cf, rf) = unsafe { pair::<FnFree>("ZSTD_freeCCtx") };
    let (c2, r2) = unsafe { pair::<FnStream2>("ZSTD_compressStream2") };
    let (ccs, rcs) = unsafe { pair::<FnStream>("ZSTD_compressStream") };
    let (cfl, rfl) = unsafe { pair::<FnFlush>("ZSTD_flushStream") };
    let (cend, rend) = unsafe { pair::<FnFlush>("ZSTD_endStream") };
    let (cdn, rdn) = unsafe { pair::<FnNew>("ZSTD_createDCtx") };
    let (cdf, rdf) = unsafe { pair::<FnFree>("ZSTD_freeDCtx") };
    let (cds, rds) = unsafe { pair::<FnStream>("ZSTD_decompressStream") };
    let mut rng = Rng::new(0xE00E);
    let src = gen(Shape::Text, 5000, &mut rng);
    let mut obuf = vec![0u8; 4096];

    let cases: &[(&str, usize, usize, usize, usize)] = &[
        // (label, out.size, out.pos, in.size, in.pos)
        ("out.pos > out.size", 100, 200, 5000, 0),
        ("out.pos == out.size", 100, 100, 5000, 0),
        ("in.pos > in.size", 4096, 0, 100, 200),
        ("both bad", 100, 200, 100, 200),
        ("in.pos == in.size", 4096, 0, 100, 100),
    ];
    for &(label, osz, opos, isz, ipos) in cases {
        for &mode in &[E_CONTINUE, E_FLUSH, E_END] {
            let a = unsafe { cn() };
            let b = unsafe { rn() };
            let mut oa = OutBuffer { dst: obuf.as_mut_ptr(), size: osz, pos: opos };
            let mut ob = OutBuffer { dst: obuf.as_mut_ptr(), size: osz, pos: opos };
            let mut ia = InBuffer { src: src.as_ptr(), size: isz, pos: ipos };
            let mut ib = InBuffer { src: src.as_ptr(), size: isz, pos: ipos };
            let x = unsafe { c2(a, &mut oa, &mut ia, mode) };
            let y = unsafe { r2(b, &mut ob, &mut ib, mode) };
            assert_eq!(x, y, "compressStream2({label}, mode={mode})");
            assert_eq!(oa.pos, ob.pos, "{label}: out.pos");
            assert_eq!(ia.pos, ib.pos, "{label}: in.pos");
            unsafe {
                cf(a);
                rf(b);
            }
        }
        // old API
        let a = unsafe { cn() };
        let b = unsafe { rn() };
        let mut oa = OutBuffer { dst: obuf.as_mut_ptr(), size: osz, pos: opos };
        let mut ob = OutBuffer { dst: obuf.as_mut_ptr(), size: osz, pos: opos };
        let mut ia = InBuffer { src: src.as_ptr(), size: isz, pos: ipos };
        let mut ib = InBuffer { src: src.as_ptr(), size: isz, pos: ipos };
        assert_eq!(
            unsafe { ccs(a, &mut oa, &mut ia) },
            unsafe { rcs(b, &mut ob, &mut ib) },
            "compressStream({label})"
        );
        let mut oa = OutBuffer { dst: obuf.as_mut_ptr(), size: osz, pos: opos };
        let mut ob = OutBuffer { dst: obuf.as_mut_ptr(), size: osz, pos: opos };
        assert_eq!(unsafe { cfl(a, &mut oa) }, unsafe { rfl(b, &mut ob) }, "flushStream({label})");
        let mut oa = OutBuffer { dst: obuf.as_mut_ptr(), size: osz, pos: opos };
        let mut ob = OutBuffer { dst: obuf.as_mut_ptr(), size: osz, pos: opos };
        assert_eq!(unsafe { cend(a, &mut oa) }, unsafe { rend(b, &mut ob) }, "endStream({label})");
        unsafe {
            cf(a);
            rf(b);
        }
        // decompression side
        let da = unsafe { cdn() };
        let db = unsafe { rdn() };
        let mut oa = OutBuffer { dst: obuf.as_mut_ptr(), size: osz, pos: opos };
        let mut ob = OutBuffer { dst: obuf.as_mut_ptr(), size: osz, pos: opos };
        let mut ia = InBuffer { src: src.as_ptr(), size: isz, pos: ipos };
        let mut ib = InBuffer { src: src.as_ptr(), size: isz, pos: ipos };
        assert_eq!(
            unsafe { cds(da, &mut oa, &mut ia) },
            unsafe { rds(db, &mut ob, &mut ib) },
            "decompressStream({label})"
        );
        unsafe {
            cdf(da);
            rdf(db);
        }
    }
    // NULL dst / NULL src pointers with zero sizes
    let a = unsafe { cn() };
    let b = unsafe { rn() };
    let mut oa = OutBuffer { dst: std::ptr::null_mut(), size: 0, pos: 0 };
    let mut ob = OutBuffer { dst: std::ptr::null_mut(), size: 0, pos: 0 };
    let mut ia = InBuffer { src: std::ptr::null(), size: 0, pos: 0 };
    let mut ib = InBuffer { src: std::ptr::null(), size: 0, pos: 0 };
    assert_eq!(
        unsafe { c2(a, &mut oa, &mut ia, E_END) },
        unsafe { r2(b, &mut ob, &mut ib, E_END) },
        "compressStream2 with NULL buffers"
    );
    unsafe {
        cf(a);
        rf(b);
    }
}

#[test]
fn err_stream_bad_enddirective() {
    let (cn, rn) = unsafe { pair::<FnNew>("ZSTD_createCCtx") };
    let (cf, rf) = unsafe { pair::<FnFree>("ZSTD_freeCCtx") };
    let (c2, r2) = unsafe { pair::<FnStream2>("ZSTD_compressStream2") };
    let mut rng = Rng::new(0xE00F);
    let src = gen(Shape::Text, 2000, &mut rng);
    let mut obuf = vec![0u8; 8192];
    for endop in [3, 4, 99, -1, i32::MIN, i32::MAX] {
        let a = unsafe { cn() };
        let b = unsafe { rn() };
        let mut oa = OutBuffer { dst: obuf.as_mut_ptr(), size: obuf.len(), pos: 0 };
        let mut ob = OutBuffer { dst: obuf.as_mut_ptr(), size: obuf.len(), pos: 0 };
        let mut ia = InBuffer { src: src.as_ptr(), size: src.len(), pos: 0 };
        let mut ib = InBuffer { src: src.as_ptr(), size: src.len(), pos: 0 };
        let x = unsafe { c2(a, &mut oa, &mut ia, endop) };
        let y = unsafe { r2(b, &mut ob, &mut ib, endop) };
        assert_eq!(x, y, "compressStream2(endOp={endop})");
        assert!(is_error(x), "endOp={endop} must be rejected by C");
        unsafe {
            cf(a);
            rf(b);
        }
    }
}

#[test]
fn err_stable_in_violation() {
    let (cn, rn) = unsafe { pair::<FnNew>("ZSTD_createCCtx") };
    let (cf, rf) = unsafe { pair::<FnFree>("ZSTD_freeCCtx") };
    let (cs, rs) = unsafe { pair::<FnSetParam>("ZSTD_CCtx_setParameter") };
    let (c2, r2) = unsafe { pair::<FnStream2>("ZSTD_compressStream2") };
    let mut rng = Rng::new(0xE010);
    let src1 = gen(Shape::Text, 100000, &mut rng);
    let src2 = gen(Shape::Text, 100000, &mut rng);
    let mut obuf = vec![0u8; 1000];

    for violation in 0..3 {
        let a = unsafe { cn() };
        let b = unsafe { rn() };
        assert_eq!(unsafe { cs(a, C_STABLEINBUFFER, 1) }, unsafe { rs(b, C_STABLEINBUFFER, 1) });
        let mut rets: [Vec<usize>; 2] = [Vec::new(), Vec::new()];
        for which in 0..2 {
            let z = if which == 0 { a } else { b };
            let mut input = InBuffer { src: src1.as_ptr(), size: src1.len(), pos: 0 };
            let mut out = OutBuffer { dst: obuf.as_mut_ptr(), size: obuf.len(), pos: 0 };
            let rc = if which == 0 {
                unsafe { c2(z, &mut out, &mut input, E_CONTINUE) }
            } else {
                unsafe { r2(z, &mut out, &mut input, E_CONTINUE) }
            };
            rets[which].push(rc);
            // now violate the stability contract
            match violation {
                0 => input.src = src2.as_ptr(),          // different buffer
                1 => input.pos = input.pos.saturating_add(7), // externally advanced pos
                _ => input.size = input.size / 2,        // shrunk size
            }
            let mut out = OutBuffer { dst: obuf.as_mut_ptr(), size: obuf.len(), pos: 0 };
            let rc = if which == 0 {
                unsafe { c2(z, &mut out, &mut input, E_CONTINUE) }
            } else {
                unsafe { r2(z, &mut out, &mut input, E_CONTINUE) }
            };
            rets[which].push(rc);
        }
        assert_eq!(rets[0], rets[1], "stableInBuffer violation {violation}");
        unsafe {
            cf(a);
            rf(b);
        }
    }
}

#[test]
fn err_stable_out_violation() {
    let (cn, rn) = unsafe { pair::<FnNew>("ZSTD_createCCtx") };
    let (cf, rf) = unsafe { pair::<FnFree>("ZSTD_freeCCtx") };
    let (cs, rs) = unsafe { pair::<FnSetParam>("ZSTD_CCtx_setParameter") };
    let (c2, r2) = unsafe { pair::<FnStream2>("ZSTD_compressStream2") };
    let mut rng = Rng::new(0xE011);
    let src = gen(Shape::Text, 300000, &mut rng);
    let mut obuf = vec![0u8; 400000];

    for violation in 0..2 {
        let a = unsafe { cn() };
        let b = unsafe { rn() };
        assert_eq!(unsafe { cs(a, C_STABLEOUTBUFFER, 1) }, unsafe { rs(b, C_STABLEOUTBUFFER, 1) });
        let mut rets: [Vec<usize>; 2] = [Vec::new(), Vec::new()];
        for which in 0..2 {
            let z = if which == 0 { a } else { b };
            let mut input = InBuffer { src: src.as_ptr(), size: 100000, pos: 0 };
            let mut out = OutBuffer { dst: obuf.as_mut_ptr(), size: 200000, pos: 0 };
            let rc = if which == 0 {
                unsafe { c2(z, &mut out, &mut input, E_CONTINUE) }
            } else {
                unsafe { r2(z, &mut out, &mut input, E_CONTINUE) }
            };
            rets[which].push(rc);
            // violate: change the remaining capacity (size - pos)
            let mut out2 = if violation == 0 {
                OutBuffer { dst: obuf.as_mut_ptr(), size: 150000, pos: out.pos }
            } else {
                OutBuffer { dst: obuf.as_mut_ptr(), size: 200000, pos: out.pos + 13 }
            };
            input.size = src.len();
            let rc = if which == 0 {
                unsafe { c2(z, &mut out2, &mut input, E_END) }
            } else {
                unsafe { r2(z, &mut out2, &mut input, E_END) }
            };
            rets[which].push(rc);
        }
        assert_eq!(rets[0], rets[1], "stableOutBuffer violation {violation}");
        unsafe {
            cf(a);
            rf(b);
        }
    }
}

#[test]
fn err_init_missing() {
    // The old streaming API on a context that never got an init call, plus
    // flush/end before any input.
    let (cn, rn) = unsafe { pair::<FnNew>("ZSTD_createCStream") };
    let (cf, rf) = unsafe { pair::<FnFree>("ZSTD_freeCStream") };
    let (ccs, rcs) = unsafe { pair::<FnStream>("ZSTD_compressStream") };
    let (cfl, rfl) = unsafe { pair::<FnFlush>("ZSTD_flushStream") };
    let (cend, rend) = unsafe { pair::<FnFlush>("ZSTD_endStream") };
    let mut rng = Rng::new(0xE012);
    let src = gen(Shape::Text, 1000, &mut rng);
    let mut obuf = vec![0u8; 4096];

    for order in 0..3 {
        let a = unsafe { cn() };
        let b = unsafe { rn() };
        let mut rets: [Vec<usize>; 2] = [Vec::new(), Vec::new()];
        for which in 0..2 {
            let z = if which == 0 { a } else { b };
            let mut push = |rc: usize| rets[which].push(rc);
            match order {
                0 => {
                    let mut o = OutBuffer { dst: obuf.as_mut_ptr(), size: obuf.len(), pos: 0 };
                    push(if which == 0 {
                        unsafe { cfl(z, &mut o) }
                    } else {
                        unsafe { rfl(z, &mut o) }
                    });
                }
                1 => {
                    let mut o = OutBuffer { dst: obuf.as_mut_ptr(), size: obuf.len(), pos: 0 };
                    push(if which == 0 {
                        unsafe { cend(z, &mut o) }
                    } else {
                        unsafe { rend(z, &mut o) }
                    });
                }
                _ => {
                    let mut o = OutBuffer { dst: obuf.as_mut_ptr(), size: obuf.len(), pos: 0 };
                    let mut i = InBuffer { src: src.as_ptr(), size: src.len(), pos: 0 };
                    push(if which == 0 {
                        unsafe { ccs(z, &mut o, &mut i) }
                    } else {
                        unsafe { rcs(z, &mut o, &mut i) }
                    });
                }
            }
        }
        assert_eq!(rets[0], rets[1], "uninitialised CStream, order {order}");
        unsafe {
            cf(a);
            rf(b);
        }
    }
}

#[test]
fn err_write_last_empty_block() {
    let (cw, rw) = unsafe { pair::<FnWriteLastEmpty>("ZSTD_writeLastEmptyBlock") };
    let mut buf = [0u8; 8];
    for cap in 0..=8usize {
        let x = unsafe { cw(buf.as_mut_ptr(), cap) };
        let mut b2 = [0u8; 8];
        let y = unsafe { rw(b2.as_mut_ptr(), cap) };
        assert_eq!(x, y, "writeLastEmptyBlock(cap={cap})");
        if !is_error(x) {
            assert_bytes_eq("last empty block", &buf[..x], &b2[..y]);
        }
    }
}

#[test]
fn err_pledged_srcsize_wrong() {
    let (cn, rn) = unsafe { pair::<FnNew>("ZSTD_createCCtx") };
    let (cf, rf) = unsafe { pair::<FnFree>("ZSTD_freeCCtx") };
    let (cp, rp) = unsafe { pair::<FnPledged>("ZSTD_CCtx_setPledgedSrcSize") };
    let (c2, r2) = unsafe { pair::<FnStream2>("ZSTD_compressStream2") };
    let mut rng = Rng::new(0xE013);
    let src = gen(Shape::Text, 50000, &mut rng);
    let mut obuf = vec![0u8; 1 << 18];

    for &(pledged, feed) in &[
        (1000u64, 2000usize),
        (2000, 1000),
        (0, 10),
        (50000, 49999),
        (49999, 50000),
    ] {
        let a = unsafe { cn() };
        let b = unsafe { rn() };
        assert_eq!(unsafe { cp(a, pledged) }, unsafe { rp(b, pledged) });
        let mut rets: [Vec<usize>; 2] = [Vec::new(), Vec::new()];
        for which in 0..2 {
            let z = if which == 0 { a } else { b };
            let mut input = InBuffer { src: src.as_ptr(), size: feed.min(src.len()), pos: 0 };
            let mut out = OutBuffer { dst: obuf.as_mut_ptr(), size: obuf.len(), pos: 0 };
            let mut guard = 0;
            loop {
                guard += 1;
                assert!(guard < 1000);
                let rc = if which == 0 {
                    unsafe { c2(z, &mut out, &mut input, E_END) }
                } else {
                    unsafe { r2(z, &mut out, &mut input, E_END) }
                };
                rets[which].push(rc);
                if is_error(rc) || rc == 0 {
                    break;
                }
                out.pos = 0;
            }
        }
        assert_eq!(rets[0], rets[1], "pledged={pledged} feed={feed}");
        unsafe {
            cf(a);
            rf(b);
        }
    }
}

// ------------------------------------------------------------------ FSE/HUF --

#[test]
fn err_fse() {
    type FnNormalize = unsafe extern "C" fn(*mut i16, c_uint, *const c_uint, usize, c_uint, c_uint) -> usize;
    type FnWriteNCount =
        unsafe extern "C" fn(*mut c_void, usize, *const i16, c_uint, c_uint) -> usize;
    type FnReadNCount = unsafe extern "C" fn(*mut i16, *mut c_uint, *mut c_uint, *const c_void, usize) -> usize;
    type FnDecompressWksp = unsafe extern "C" fn(*mut c_void, usize, *const c_void, usize, c_uint, *mut c_void, usize, c_int) -> usize;
    let (cnorm, rnorm) = unsafe { pair::<FnNormalize>("FSE_normalizeCount") };
    let (cwr, rwr) = unsafe { pair::<FnWriteNCount>("FSE_writeNCount") };
    let (crd, rrd) = unsafe { pair::<FnReadNCount>("FSE_readNCount") };
    let (cdec, rdec) = unsafe { pair::<FnDecompressWksp>("FSE_decompress_wksp_bmi2") };
    let mut rng = Rng::new(0xE014);

    // histogram of a small alphabet
    let mut count = [0u32; 256];
    let data = gen(Shape::Text, 4096, &mut rng);
    for &b in &data {
        count[b as usize] += 1;
    }
    let max_sv = 255u32;
    for tableLog in [0u32, 1, 4, 5, 11, 12, 13, 16, 100] {
        for &lowprob in &[0u32, 1] {
            let mut na = vec![0i16; 256];
            let mut nb = vec![0i16; 256];
            let x = unsafe {
                cnorm(na.as_mut_ptr(), tableLog, count.as_ptr(), data.len(), max_sv, lowprob)
            };
            let y = unsafe {
                rnorm(nb.as_mut_ptr(), tableLog, count.as_ptr(), data.len(), max_sv, lowprob)
            };
            assert_eq!(x, y, "FSE_normalizeCount(tableLog={tableLog}, lowprob={lowprob})");
            if !is_error(x) {
                assert_eq!(na, nb, "normalized counts differ");
                // writeNCount with too-small dst
                for cap in [0usize, 1, 2, 3, 8, 512] {
                    let mut a = vec![0u8; cap.max(1)];
                    let mut b = vec![0u8; cap.max(1)];
                    let p = unsafe {
                        cwr(a.as_mut_ptr() as *mut c_void, cap, na.as_ptr(), max_sv, x as c_uint)
                    };
                    let q = unsafe {
                        rwr(b.as_mut_ptr() as *mut c_void, cap, nb.as_ptr(), max_sv, x as c_uint)
                    };
                    assert_eq!(p, q, "FSE_writeNCount(cap={cap})");
                    if !is_error(p) {
                        assert_bytes_eq("NCount bytes", &a[..p], &b[..q]);
                    }
                }
            }
        }
    }
    // readNCount on garbage / truncated buffers
    for n in [0usize, 1, 2, 3, 8, 64] {
        let junk = gen(Shape::Random, n, &mut rng);
        let mut na = vec![0i16; 256];
        let mut nb = vec![0i16; 256];
        let mut msa = 255u32;
        let mut msb = 255u32;
        let mut tla = 12u32;
        let mut tlb = 12u32;
        let x = unsafe {
            crd(na.as_mut_ptr(), &mut msa, &mut tla, junk.as_ptr() as *const c_void, n)
        };
        let y = unsafe {
            rrd(nb.as_mut_ptr(), &mut msb, &mut tlb, junk.as_ptr() as *const c_void, n)
        };
        assert_eq!(x, y, "FSE_readNCount(junk {n})");
        if !is_error(x) {
            assert_eq!((msa, tla), (msb, tlb));
            assert_eq!(na, nb);
        }
    }
    // decompress garbage
    let wksp_size = 4096 * 8;
    for n in [0usize, 1, 2, 16, 256] {
        let junk = gen(Shape::Random, n, &mut rng);
        for maxlog in [0u32, 5, 12, 13] {
            let mut oa = vec![0u8; 4096];
            let mut ob = vec![0u8; 4096];
            let mut wa = vec![0u8; wksp_size];
            let mut wb = vec![0u8; wksp_size];
            for bmi2 in 0..2 {
                let x = unsafe {
                    cdec(oa.as_mut_ptr() as *mut c_void, oa.len(),
                         junk.as_ptr() as *const c_void, n, maxlog,
                         wa.as_mut_ptr() as *mut c_void, wksp_size, bmi2)
                };
                let y = unsafe {
                    rdec(ob.as_mut_ptr() as *mut c_void, ob.len(),
                         junk.as_ptr() as *const c_void, n, maxlog,
                         wb.as_mut_ptr() as *mut c_void, wksp_size, bmi2)
                };
                assert_eq!(x, y, "FSE_decompress_wksp_bmi2(n={n},maxlog={maxlog},bmi2={bmi2})");
                if !is_error(x) {
                    assert_bytes_eq("fse decompressed", &oa[..x], &ob[..y]);
                }
            }
        }
    }
}

#[test]
fn err_huf() {
    type FnReadStats = unsafe extern "C" fn(
        *mut u8, usize, *mut c_uint, *mut c_uint, *mut c_uint, *const c_void, usize,
    ) -> usize;
    type FnReadStatsWksp = unsafe extern "C" fn(
        *mut u8, usize, *mut c_uint, *mut c_uint, *mut c_uint, *const c_void, usize,
        *mut c_void, usize, c_int,
    ) -> usize;
    // HUF_decompress{1,4}X*_DCtx_wksp(HUF_DTable* dctx, dst, dstSize, cSrc, cSrcSize,
    //                                workSpace, wkspSize, flags)
    type FnDecompressDCtx = unsafe extern "C" fn(
        *mut u32, *mut c_void, usize, *const c_void, usize, *mut c_void, usize, c_int,
    ) -> usize;
    let (crs, rrs) = unsafe { pair::<FnReadStats>("HUF_readStats") };
    let (crsw, rrsw) = unsafe { pair::<FnReadStatsWksp>("HUF_readStats_wksp") };
    let (c1, r1) = unsafe { pair::<FnDecompressDCtx>("HUF_decompress1X_DCtx_wksp") };
    let (c11, r11) = unsafe { pair::<FnDecompressDCtx>("HUF_decompress1X1_DCtx_wksp") };
    let (c12, r12) = unsafe { pair::<FnDecompressDCtx>("HUF_decompress1X2_DCtx_wksp") };
    let (c4, r4) = unsafe { pair::<FnDecompressDCtx>("HUF_decompress4X_hufOnly_wksp") };
    let mut rng = Rng::new(0xE015);
    let wksp_size = 8704 * 2;
    // HUF_DTABLE_SIZE(12) u32 entries, first entry encodes the max table log
    let dt_len = 1 + (1usize << 12);

    for n in [0usize, 1, 2, 3, 8, 64, 128, 512] {
        for &shape in &[Shape::Random, Shape::Rle, Shape::Text] {
            let junk = gen(shape, n, &mut rng);
            let mut wa = vec![0u8; wksp_size];
            let mut wb = vec![0u8; wksp_size];
            let mut ha = vec![0u8; 256];
            let mut hb = vec![0u8; 256];
            let mut ranka = [0u32; 16];
            let mut rankb = [0u32; 16];
            let mut nsa = 0u32;
            let mut nsb = 0u32;
            let mut tla = 0u32;
            let mut tlb = 0u32;
            let x = unsafe {
                crs(ha.as_mut_ptr(), 256, ranka.as_mut_ptr(), &mut nsa, &mut tla,
                    junk.as_ptr() as *const c_void, n)
            };
            let y = unsafe {
                rrs(hb.as_mut_ptr(), 256, rankb.as_mut_ptr(), &mut nsb, &mut tlb,
                    junk.as_ptr() as *const c_void, n)
            };
            assert_eq!(x, y, "HUF_readStats(n={n},{shape:?})");
            if !is_error(x) {
                assert_eq!(ha, hb, "HUF_readStats weights");
                assert_eq!(ranka, rankb, "HUF_readStats rankStats");
                assert_eq!((nsa, tla), (nsb, tlb), "HUF_readStats out params");
            }
            for flags in [0, 1, 16, 32, 48] {
                let mut ranka = [0u32; 16];
                let mut rankb = [0u32; 16];
                let mut nsa = 0u32;
                let mut nsb = 0u32;
                let mut tla = 0u32;
                let mut tlb = 0u32;
                let x = unsafe {
                    crsw(ha.as_mut_ptr(), 256, ranka.as_mut_ptr(), &mut nsa, &mut tla,
                         junk.as_ptr() as *const c_void, n,
                         wa.as_mut_ptr() as *mut c_void, wksp_size, flags)
                };
                let y = unsafe {
                    rrsw(hb.as_mut_ptr(), 256, rankb.as_mut_ptr(), &mut nsb, &mut tlb,
                         junk.as_ptr() as *const c_void, n,
                         wb.as_mut_ptr() as *mut c_void, wksp_size, flags)
                };
                assert_eq!(x, y, "HUF_readStats_wksp(n={n},flags={flags})");
                if !is_error(x) {
                    assert_eq!((nsa, tla), (nsb, tlb));
                    assert_eq!(ranka, rankb);
                }
                for (label, cf, rf) in [
                    ("1X", &c1, &r1),
                    ("1X1", &c11, &r11),
                    ("1X2", &c12, &r12),
                    ("4XhufOnly", &c4, &r4),
                ] {
                    let mut dta = vec![0u32; dt_len];
                    let mut dtb = vec![0u32; dt_len];
                    dta[0] = 12 * 0x01000001;
                    dtb[0] = 12 * 0x01000001;
                    let mut oa = vec![0u8; 4096];
                    let mut ob = vec![0u8; 4096];
                    let x = unsafe {
                        cf(dta.as_mut_ptr(), oa.as_mut_ptr() as *mut c_void, oa.len(),
                           junk.as_ptr() as *const c_void, n,
                           wa.as_mut_ptr() as *mut c_void, wksp_size, flags)
                    };
                    let y = unsafe {
                        rf(dtb.as_mut_ptr(), ob.as_mut_ptr() as *mut c_void, ob.len(),
                           junk.as_ptr() as *const c_void, n,
                           wb.as_mut_ptr() as *mut c_void, wksp_size, flags)
                    };
                    assert_eq!(x, y, "HUF_decompress{label}_DCtx_wksp(n={n},flags={flags})");
                    if !is_error(x) {
                        assert_bytes_eq(&format!("huf {label} out"), &oa[..x], &ob[..y]);
                        assert_eq!(dta, dtb, "huf {label} DTable");
                    }
                }
            }
        }
    }
}
