//! Phase C: the DICTIONARY compression / decompression surface.
//!
//! Everything is exercised through `dlopen`'d exports of BOTH libraries, never
//! by linking the Rust crate. For every configuration we require
//!
//!   * identical numeric returns *and* identical `ZSTD_getErrorCode`,
//!   * byte-identical compressed frames,
//!   * identical dictID reporting (`fromDict` / `fromCDict` / `fromDDict` /
//!     `fromFrame`),
//!   * correct cross-library round trips (C frame decoded by Rust and vice
//!     versa, with the same dictionary),
//!   * identical rejection of the wrong dictionary / corrupted dictionaries.
//!
//! Symbols covered: ZSTD_compress_usingDict, ZSTD_decompress_usingDict,
//! ZSTD_createCDict{,_advanced,_advanced2,_byReference}, ZSTD_freeCDict,
//! ZSTD_initStaticCDict, ZSTD_sizeof_CDict, ZSTD_estimateCDictSize{,_advanced},
//! ZSTD_compress_usingCDict{,_advanced}, ZSTD_createDDict{,_advanced,
//! _byReference}, ZSTD_freeDDict, ZSTD_initStaticDDict, ZSTD_sizeof_DDict,
//! ZSTD_estimateDDictSize, ZSTD_decompress_usingDDict,
//! ZSTD_getDictID_from{Dict,CDict,DDict,Frame},
//! ZSTD_CCtx_loadDictionary{,_byReference,_advanced}, ZSTD_CCtx_refCDict,
//! ZSTD_CCtx_refPrefix{,_advanced}, ZSTD_CCtx_refThreadPool,
//! ZSTD_DCtx_loadDictionary{,_byReference,_advanced}, ZSTD_DCtx_refDDict,
//! ZSTD_DCtx_refPrefix{,_advanced}, ZSTD_DCtx_setMaxWindowSize,
//! ZSTD_getCParams, ZDICT_trainFromBuffer, ZDICT_getDictID,
//! ZDICT_getDictHeaderSize.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

mod common;
use common::*;

use libloading::Symbol;
use std::ffi::c_void;
use std::sync::OnceLock;

// ------------------------------------------------------------------ FFI types

type CCtx = *mut c_void;
type DCtx = *mut c_void;
type CDict = *mut c_void;
type DDict = *mut c_void;
type CCtxParams = *mut c_void;

/// `ZSTD_compressionParameters` — 6 unsigned + ZSTD_strategy (an int).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
struct CParams {
    window_log: u32,
    chain_log: u32,
    hash_log: u32,
    search_log: u32,
    min_match: u32,
    target_length: u32,
    strategy: i32,
}

/// `ZSTD_frameParameters` — { int contentSizeFlag; int checksumFlag; int noDictIDFlag; }
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
struct FParams {
    content_size_flag: i32,
    checksum_flag: i32,
    no_dict_id_flag: i32,
}

/// `ZSTD_customMem` — { alloc; free; opaque } ; all-NULL == ZSTD_defaultCMem.
#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct CustomMem {
    alloc: *const c_void,
    free: *const c_void,
    opaque: *mut c_void,
}
impl CustomMem {
    fn default_cmem() -> Self {
        CustomMem {
            alloc: std::ptr::null(),
            free: std::ptr::null(),
            opaque: std::ptr::null_mut(),
        }
    }
}

type Fn_createCCtx = unsafe extern "C" fn() -> CCtx;
type Fn_freeCCtx = unsafe extern "C" fn(CCtx) -> usize;
type Fn_createDCtx = unsafe extern "C" fn() -> DCtx;
type Fn_freeDCtx = unsafe extern "C" fn(DCtx) -> usize;
type Fn_reset = unsafe extern "C" fn(CCtx, i32) -> usize;
type Fn_dReset = unsafe extern "C" fn(DCtx, i32) -> usize;
type Fn_setParam = unsafe extern "C" fn(CCtx, i32, i32) -> usize;
type Fn_compress2 = unsafe extern "C" fn(CCtx, *mut u8, usize, *const u8, usize) -> usize;
type Fn_decompressDCtx = unsafe extern "C" fn(DCtx, *mut u8, usize, *const u8, usize) -> usize;
type Fn_bound = unsafe extern "C" fn(usize) -> usize;
type Fn_getErrorCode = unsafe extern "C" fn(usize) -> i32;
type Fn_isError = unsafe extern "C" fn(usize) -> u32;

type Fn_compress_usingDict =
    unsafe extern "C" fn(CCtx, *mut u8, usize, *const u8, usize, *const u8, usize, i32) -> usize;
type Fn_decompress_usingDict =
    unsafe extern "C" fn(DCtx, *mut u8, usize, *const u8, usize, *const u8, usize) -> usize;

type Fn_createCDict = unsafe extern "C" fn(*const u8, usize, i32) -> CDict;
type Fn_createCDict_advanced =
    unsafe extern "C" fn(*const u8, usize, i32, i32, CParams, CustomMem) -> CDict;
type Fn_createCDict_advanced2 =
    unsafe extern "C" fn(*const u8, usize, i32, i32, CCtxParams, CustomMem) -> CDict;
type Fn_freeCDict = unsafe extern "C" fn(CDict) -> usize;
type Fn_initStaticCDict =
    unsafe extern "C" fn(*mut u8, usize, *const u8, usize, i32, i32, CParams) -> CDict;
type Fn_sizeof_CDict = unsafe extern "C" fn(CDict) -> usize;
type Fn_estimateCDictSize = unsafe extern "C" fn(usize, i32) -> usize;
type Fn_estimateCDictSize_advanced = unsafe extern "C" fn(usize, CParams, i32) -> usize;
type Fn_compress_usingCDict =
    unsafe extern "C" fn(CCtx, *mut u8, usize, *const u8, usize, CDict) -> usize;
type Fn_compress_usingCDict_advanced =
    unsafe extern "C" fn(CCtx, *mut u8, usize, *const u8, usize, CDict, FParams) -> usize;

type Fn_createDDict = unsafe extern "C" fn(*const u8, usize) -> DDict;
type Fn_createDDict_advanced = unsafe extern "C" fn(*const u8, usize, i32, i32, CustomMem) -> DDict;
type Fn_freeDDict = unsafe extern "C" fn(DDict) -> usize;
type Fn_initStaticDDict =
    unsafe extern "C" fn(*mut u8, usize, *const u8, usize, i32, i32) -> DDict;
type Fn_sizeof_DDict = unsafe extern "C" fn(DDict) -> usize;
type Fn_estimateDDictSize = unsafe extern "C" fn(usize, i32) -> usize;
type Fn_decompress_usingDDict =
    unsafe extern "C" fn(DCtx, *mut u8, usize, *const u8, usize, DDict) -> usize;

type Fn_dictID_fromDict = unsafe extern "C" fn(*const u8, usize) -> u32;
type Fn_dictID_fromCDict = unsafe extern "C" fn(CDict) -> u32;
type Fn_dictID_fromDDict = unsafe extern "C" fn(DDict) -> u32;

type Fn_loadDict = unsafe extern "C" fn(CCtx, *const u8, usize) -> usize;
type Fn_loadDict_advanced = unsafe extern "C" fn(CCtx, *const u8, usize, i32, i32) -> usize;
type Fn_refCDict = unsafe extern "C" fn(CCtx, CDict) -> usize;
type Fn_refPrefix = unsafe extern "C" fn(CCtx, *const u8, usize) -> usize;
type Fn_refPrefix_advanced = unsafe extern "C" fn(CCtx, *const u8, usize, i32) -> usize;
type Fn_dLoadDict = unsafe extern "C" fn(DCtx, *const u8, usize) -> usize;
type Fn_dLoadDict_advanced = unsafe extern "C" fn(DCtx, *const u8, usize, i32, i32) -> usize;
type Fn_refDDict = unsafe extern "C" fn(DCtx, DDict) -> usize;
type Fn_dRefPrefix = unsafe extern "C" fn(DCtx, *const u8, usize) -> usize;
type Fn_dRefPrefix_advanced = unsafe extern "C" fn(DCtx, *const u8, usize, i32) -> usize;
type Fn_setMaxWindowSize = unsafe extern "C" fn(DCtx, usize) -> usize;
type Fn_refThreadPool = unsafe extern "C" fn(CCtx, *mut c_void) -> usize;

type Fn_getCParams = unsafe extern "C" fn(i32, u64, usize) -> CParams;

type Fn_createCCtxParams = unsafe extern "C" fn() -> CCtxParams;
type Fn_freeCCtxParams = unsafe extern "C" fn(CCtxParams) -> usize;
type Fn_cctxParamsInit = unsafe extern "C" fn(CCtxParams, i32) -> usize;
type Fn_cctxParamsSet = unsafe extern "C" fn(CCtxParams, i32, i32) -> usize;

type Fn_compressStream2 =
    unsafe extern "C" fn(CCtx, *mut ZSTD_outBuffer, *mut ZSTD_inBuffer, i32) -> usize;
type Fn_decompressStream =
    unsafe extern "C" fn(DCtx, *mut ZSTD_outBuffer, *mut ZSTD_inBuffer) -> usize;

type Fn_train = unsafe extern "C" fn(*mut u8, usize, *const u8, *const usize, u32) -> usize;
type Fn_zdictGetDictID = unsafe extern "C" fn(*const u8, usize) -> u32;
type Fn_zdictHeaderSize = unsafe extern "C" fn(*const u8, usize) -> usize;

// -------------------------------------------------------------------- helpers

/// zstd's own convention: any `size_t` above `-ZSTD_error_maxCode` is an error.
fn is_err(v: usize) -> bool {
    v > usize::MAX - 200
}

/// Compares a numeric result *and* — when it denotes an error — the decoded
/// `ZSTD_ErrorCode`, so "both failed" is never enough to pass.
struct ErrCmp {
    c: Symbol<'static, Fn_getErrorCode>,
    r: Symbol<'static, Fn_getErrorCode>,
}

impl ErrCmp {
    fn new() -> Self {
        let (c, r) = impls().pair::<Fn_getErrorCode>("ZSTD_getErrorCode");
        ErrCmp { c, r }
    }
    /// Returns `true` when the (identical) result is an error.
    fn check(&self, tag: &str, a: usize, b: usize) -> bool {
        assert_eq_dbg(tag, a, b);
        if is_err(a) {
            unsafe {
                assert_eq_dbg(&format!("{tag} / ZSTD_getErrorCode"), (self.c)(a), (self.r)(b));
            }
            true
        } else {
            false
        }
    }
    fn code(&self, v: usize) -> i32 {
        unsafe { (self.c)(v) }
    }
}

/// Log-like, highly dictionary-friendly payload — the trained dictionaries in
/// the corpus below are built from the very same generator, so the dictionary
/// actually changes the compressed output (which is what we want to diff).
fn gen_logish_range(rng: &mut Rng, lo: usize, hi: usize) -> Vec<u8> {
    let n = rng.range(lo, hi);
    gen_logish(rng, n)
}

fn gen_logish(rng: &mut Rng, len: usize) -> Vec<u8> {
    const TOK: [&str; 8] = [
        "alpha", "bravo", "charlie", "delta", "echo", "foxtrot", "golf", "hotel",
    ];
    let mut v = Vec::with_capacity(len + 64);
    while v.len() < len {
        v.extend_from_slice(TOK[rng.below(TOK.len())].as_bytes());
        v.push(b'=');
        v.extend_from_slice(format!("{}", rng.below(1000)).as_bytes());
        v.push(b';');
        if rng.below(9) == 0 {
            v.push(b'\n');
        }
    }
    v.truncate(len);
    v
}

/// Concatenated training samples plus their sizes, for `ZDICT_trainFromBuffer`.
fn make_samples(seed: u64, n: usize) -> (Vec<u8>, Vec<usize>) {
    let mut rng = Rng::new(seed);
    let mut buf = Vec::new();
    let mut sizes = Vec::with_capacity(n);
    for _ in 0..n {
        let one = gen_logish_range(&mut rng, 48, 200);
        sizes.push(one.len());
        buf.extend_from_slice(&one);
    }
    (buf, sizes)
}

fn train_with(train: Fn_train, seed: u64, n: usize, cap: usize) -> (usize, Vec<u8>) {
    let (buf, sizes) = make_samples(seed, n);
    let mut dict = vec![0u8; cap];
    let n = unsafe {
        train(
            dict.as_mut_ptr(),
            cap,
            buf.as_ptr(),
            sizes.as_ptr(),
            sizes.len() as u32,
        )
    };
    (n, dict)
}

/// One named dictionary buffer. `bytes` is `'static` (leaked into the corpus)
/// so `ZSTD_dlm_byRef` dictionaries stay valid for the whole test run.
struct DictSpec {
    name: &'static str,
    bytes: Vec<u8>,
}

impl DictSpec {
    fn ptr(&self) -> *const u8 {
        self.bytes.as_ptr()
    }
    fn len(&self) -> usize {
        self.bytes.len()
    }
}

/// The full dictionary corpus: empty, tiny, raw, real trained, hand-built
/// magic-header, corrupted magic and bit-flipped trained dictionaries.
fn dict_corpus() -> &'static Vec<DictSpec> {
    static C: OnceLock<Vec<DictSpec>> = OnceLock::new();
    C.get_or_init(|| {
        let i = impls();
        let (c_train_sym, _) = i.pair::<Fn_train>("ZDICT_trainFromBuffer");
        let c_train: Fn_train = *c_train_sym;
        let mut v: Vec<DictSpec> = Vec::new();
        let mut rng = Rng::new(0xD1C7_0001);

        // 0-byte dictionary (non-NULL pointer, zero length)
        v.push(DictSpec {
            name: "empty",
            bytes: Vec::new(),
        });
        // tiny dictionaries: 1..=8 bytes exercise the `dictSize < 8` cutoffs
        for n in 1..=8usize {
            let mut b = Vec::with_capacity(n);
            for _ in 0..n {
                b.push(rng.byte());
            }
            v.push(DictSpec {
                name: match n {
                    1 => "tiny1",
                    2 => "tiny2",
                    3 => "tiny3",
                    4 => "tiny4",
                    5 => "tiny5",
                    6 => "tiny6",
                    7 => "tiny7",
                    _ => "tiny8",
                },
                bytes: b,
            });
        }
        // raw random content (incompressible) at several sizes
        v.push(DictSpec {
            name: "raw-rand-64",
            bytes: gen_shape(Shape::Random, 64, &mut rng),
        });
        v.push(DictSpec {
            name: "raw-rand-1k",
            bytes: gen_shape(Shape::Random, 1024, &mut rng),
        });
        v.push(DictSpec {
            name: "raw-rand-100k",
            bytes: gen_shape(Shape::Random, 100_000, &mut rng),
        });
        // raw *textual* content — a plain prefix-style dictionary
        v.push(DictSpec {
            name: "raw-text-1k",
            bytes: gen_logish(&mut rng, 1024),
        });
        v.push(DictSpec {
            name: "raw-text-100k",
            bytes: gen_logish(&mut rng, 100_000),
        });

        // real trained dictionaries (small + ~100KB)
        let (n_small, d_small) = train_with(c_train, 0xAB01, 900, 8 * 1024);
        assert!(!is_err(n_small), "ZDICT_trainFromBuffer(small) failed: {n_small:#x}");
        let trained_small = d_small[..n_small].to_vec();
        v.push(DictSpec {
            name: "trained-8k",
            bytes: trained_small.clone(),
        });

        let (n_big, d_big) = train_with(c_train, 0xAB02, 6000, 112_640);
        assert!(!is_err(n_big), "ZDICT_trainFromBuffer(big) failed: {n_big:#x}");
        let trained_big = d_big[..n_big].to_vec();
        v.push(DictSpec {
            name: "trained-100k",
            bytes: trained_big.clone(),
        });

        // an independently trained dictionary — used as the "wrong" dictionary
        let (n_alt, d_alt) = train_with(c_train, 0xAB03, 900, 8 * 1024);
        assert!(!is_err(n_alt));
        v.push(DictSpec {
            name: "trained-alt-8k",
            bytes: d_alt[..n_alt].to_vec(),
        });

        // same trained dictionary, dictID field zeroed: a conformant full dict
        // that reports dictID == 0
        {
            let mut b = trained_small.clone();
            b[4..8].copy_from_slice(&0u32.to_le_bytes());
            v.push(DictSpec {
                name: "trained-id0",
                bytes: b,
            });
        }
        // hand-built ZSTD_MAGIC_DICTIONARY header followed by garbage: the magic
        // is right so dct_auto/fullDict must take the "full dict" path and fail
        // to parse the entropy tables.
        {
            let mut b = Vec::new();
            b.extend_from_slice(&ZSTD_MAGIC_DICTIONARY.to_le_bytes());
            b.extend_from_slice(&0x1234_5678u32.to_le_bytes());
            b.extend_from_slice(&gen_shape(Shape::Random, 1024, &mut rng));
            v.push(DictSpec {
                name: "magic-garbage",
                bytes: b,
            });
        }
        // trained dictionary with a corrupted magic number -> must be treated as
        // raw content under dct_auto and refused under dct_fullDict
        {
            let mut b = trained_small.clone();
            b[0] ^= 0x01;
            v.push(DictSpec {
                name: "magic-corrupt",
                bytes: b,
            });
        }
        // trained dictionary with bit flips inside the entropy tables
        for (idx, off) in [(0usize, 9usize), (1, 17), (2, 33)] {
            let mut b = trained_small.clone();
            b[off] ^= 1 << (idx + 1);
            v.push(DictSpec {
                name: match idx {
                    0 => "trained-bitflip-9",
                    1 => "trained-bitflip-17",
                    _ => "trained-bitflip-33",
                },
                bytes: b,
            });
        }
        v
    })
}

fn spec(name: &str) -> &'static DictSpec {
    dict_corpus()
        .iter()
        .find(|d| d.name == name)
        .expect("dict spec exists")
}

/// A handful of randomized inputs per configuration row.
fn inputs(rng: &mut Rng, n: usize) -> Vec<Vec<u8>> {
    let mut v = Vec::with_capacity(n);
    for k in 0..n {
        let len = match k % 6 {
            0 => 0,
            1 => rng.range(1, 40),
            2 => rng.range(40, 600),
            3 => rng.range(600, 9_000),
            4 => rng.range(9_000, 60_000),
            _ => rng.range(130_000, 150_000),
        };
        if rng.bool() {
            v.push(gen_logish(rng, len));
        } else {
            let s = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
            v.push(gen_shape(s, len, rng));
        }
    }
    v
}

const ALL_DCT: [i32; 3] = [ZSTD_dct_auto, ZSTD_dct_rawContent, ZSTD_dct_fullDict];
const ALL_DLM: [i32; 2] = [ZSTD_dlm_byCopy, ZSTD_dlm_byRef];

// ============================================================== 1. dict builder

/// The trained dictionaries the rest of this file relies on must themselves be
/// byte-identical between the two libraries, otherwise every downstream
/// comparison would be meaningless.
#[test]
fn zdict_train_and_query_match() {
    let i = impls();
    let (c_tr, r_tr) = i.pair::<Fn_train>("ZDICT_trainFromBuffer");
    let (c_id, r_id) = i.pair::<Fn_zdictGetDictID>("ZDICT_getDictID");
    let (c_hs, r_hs) = i.pair::<Fn_zdictHeaderSize>("ZDICT_getDictHeaderSize");
    let (c_ie, r_ie) = i.pair::<Fn_isError>("ZDICT_isError");
    let ec = ErrCmp::new();

    for &(seed, nsamples, cap) in &[
        (0xAB01u64, 900usize, 8 * 1024usize),
        (0xAB03, 900, 8 * 1024),
        (0x77aa, 400, 4 * 1024),
        (0x77ab, 200, 1024),
        (0x77ac, 40, 1024), // too few samples: exercises the failure path too
        (0x77ad, 1200, 32 * 1024),
    ] {
        let (buf, sizes) = make_samples(seed, nsamples);
        let mut cb = vec![0u8; cap];
        let mut rb = vec![0u8; cap];
        let a = unsafe {
            c_tr(
                cb.as_mut_ptr(),
                cap,
                buf.as_ptr(),
                sizes.as_ptr(),
                sizes.len() as u32,
            )
        };
        let b = unsafe {
            r_tr(
                rb.as_mut_ptr(),
                cap,
                buf.as_ptr(),
                sizes.as_ptr(),
                sizes.len() as u32,
            )
        };
        let tag = format!("ZDICT_trainFromBuffer(seed={seed:#x}, n={nsamples}, cap={cap})");
        if ec.check(&tag, a, b) {
            continue;
        }
        assert_bytes_eq(&tag, &cb[..a], &rb[..b]);

        // dictID / header size / isError parity on the produced dictionary
        unsafe {
            assert_eq_dbg(
                &format!("{tag} / ZDICT_getDictID"),
                c_id(cb.as_ptr(), a),
                r_id(rb.as_ptr(), b),
            );
            let (h1, h2) = (c_hs(cb.as_ptr(), a), r_hs(rb.as_ptr(), b));
            ec.check(&format!("{tag} / ZDICT_getDictHeaderSize"), h1, h2);
            assert_eq_dbg(&format!("{tag} / ZDICT_isError"), c_ie(a), r_ie(b));
        }
    }

    // header size / dictID over the whole corpus, including non-dictionaries
    for d in dict_corpus() {
        unsafe {
            let (h1, h2) = (c_hs(d.ptr(), d.len()), r_hs(d.ptr(), d.len()));
            ec.check(&format!("ZDICT_getDictHeaderSize({})", d.name), h1, h2);
            assert_eq_dbg(
                &format!("ZDICT_getDictID({})", d.name),
                c_id(d.ptr(), d.len()),
                r_id(d.ptr(), d.len()),
            );
        }
    }
}

// ================================================================ 2. dict IDs

/// `ZSTD_getDictID_from{Dict,CDict,DDict,Frame}` must agree everywhere,
/// including for empty / raw / corrupted dictionaries and for NULL handles.
#[test]
fn dict_id_queries_match() {
    let i = impls();
    let (c_fd, r_fd) = i.pair::<Fn_dictID_fromDict>("ZSTD_getDictID_fromDict");
    let (c_fc, r_fc) = i.pair::<Fn_dictID_fromCDict>("ZSTD_getDictID_fromCDict");
    let (c_fdd, r_fdd) = i.pair::<Fn_dictID_fromDDict>("ZSTD_getDictID_fromDDict");
    let (c_ff, r_ff) = i.pair::<Fn_dictID_fromDict>("ZSTD_getDictID_fromFrame");
    let (c_ccd, r_ccd) = i.pair::<Fn_createCDict_advanced>("ZSTD_createCDict_advanced");
    let (c_fcd, r_fcd) = i.pair::<Fn_freeCDict>("ZSTD_freeCDict");
    let (c_cdd, r_cdd) = i.pair::<Fn_createDDict_advanced>("ZSTD_createDDict_advanced");
    let (c_fdd_free, r_fdd_free) = i.pair::<Fn_freeDDict>("ZSTD_freeDDict");
    let (c_gcp, _) = i.pair::<Fn_getCParams>("ZSTD_getCParams");
    let (c_cud, r_cud) = i.pair::<Fn_compress_usingDict>("ZSTD_compress_usingDict");
    let (c_new, r_new) = i.pair::<Fn_createCCtx>("ZSTD_createCCtx");
    let (c_free, r_free) = i.pair::<Fn_freeCCtx>("ZSTD_freeCCtx");
    let (c_bound, _) = i.pair::<Fn_bound>("ZSTD_compressBound");

    // ---- fromDict, incl. every truncation down to 8 bytes and the empty case
    for d in dict_corpus() {
        let mut lens: Vec<usize> = vec![0, d.len()];
        for l in 8..=d.len().min(24) {
            lens.push(l);
        }
        if d.len() > 40 {
            lens.push(d.len() / 2);
        }
        for l in lens {
            unsafe {
                assert_eq_dbg(
                    &format!("ZSTD_getDictID_fromDict({}, {l})", d.name),
                    c_fd(d.ptr(), l),
                    r_fd(d.ptr(), l),
                );
            }
        }
        // dictSize < 8 short-circuits before dereferencing `dict`
        for l in 0..8usize {
            unsafe {
                assert_eq_dbg(
                    &format!("ZSTD_getDictID_fromDict(NULL, {l})"),
                    c_fd(std::ptr::null(), l),
                    r_fd(std::ptr::null(), l),
                );
            }
        }
    }

    // ---- NULL handles
    unsafe {
        assert_eq_dbg(
            "ZSTD_getDictID_fromCDict(NULL)",
            c_fc(std::ptr::null_mut()),
            r_fc(std::ptr::null_mut()),
        );
        assert_eq_dbg(
            "ZSTD_getDictID_fromDDict(NULL)",
            c_fdd(std::ptr::null_mut()),
            r_fdd(std::ptr::null_mut()),
        );
    }

    // ---- fromCDict / fromDDict across content types and load methods
    for d in dict_corpus() {
        for &dct in &ALL_DCT {
            for &dlm in &ALL_DLM {
                let cp = unsafe { c_gcp(3, ZSTD_CONTENTSIZE_UNKNOWN, d.len()) };
                let cm = CustomMem::default_cmem();
                let (cc, rc) = unsafe {
                    (
                        c_ccd(d.ptr(), d.len(), dlm, dct, cp, cm),
                        r_ccd(d.ptr(), d.len(), dlm, dct, cp, cm),
                    )
                };
                let tag = format!("CDict({}, dct={dct}, dlm={dlm})", d.name);
                assert_eq_dbg(&format!("{tag} / created?"), cc.is_null(), rc.is_null());
                if !cc.is_null() {
                    unsafe {
                        assert_eq_dbg(&format!("{tag} / dictID"), c_fc(cc), r_fc(rc));
                    }
                }
                unsafe {
                    c_fcd(cc);
                    r_fcd(rc);
                }

                let (cd, rd) = unsafe {
                    (
                        c_cdd(d.ptr(), d.len(), dlm, dct, cm),
                        r_cdd(d.ptr(), d.len(), dlm, dct, cm),
                    )
                };
                let tag = format!("DDict({}, dct={dct}, dlm={dlm})", d.name);
                assert_eq_dbg(&format!("{tag} / created?"), cd.is_null(), rd.is_null());
                if !cd.is_null() {
                    unsafe {
                        assert_eq_dbg(&format!("{tag} / dictID"), c_fdd(cd), r_fdd(rd));
                    }
                }
                unsafe {
                    c_fdd_free(cd);
                    r_fdd_free(rd);
                }
            }
        }
    }

    // ---- fromFrame, over frames produced with each dictionary and over every
    //      truncation of the header region (dictID lives in the frame header).
    let (cc, rc) = unsafe { (c_new(), r_new()) };
    let mut rng = Rng::new(0x1D_1D_1D);
    for d in dict_corpus() {
        let src = gen_logish(&mut rng, 4096);
        let cap = unsafe { c_bound(src.len()) };
        let mut cb = vec![0u8; cap];
        let mut rb = vec![0u8; cap];
        let ec = ErrCmp::new();
        let a = unsafe {
            c_cud(
                cc,
                cb.as_mut_ptr(),
                cap,
                src.as_ptr(),
                src.len(),
                d.ptr(),
                d.len(),
                3,
            )
        };
        let b = unsafe {
            r_cud(
                rc,
                rb.as_mut_ptr(),
                cap,
                src.as_ptr(),
                src.len(),
                d.ptr(),
                d.len(),
                3,
            )
        };
        let tag = format!("compress_usingDict({})", d.name);
        if ec.check(&tag, a, b) {
            continue;
        }
        assert_bytes_eq(&tag, &cb[..a], &rb[..b]);
        for cut in 0..=a.min(24) {
            unsafe {
                assert_eq_dbg(
                    &format!("{tag} / getDictID_fromFrame(cut={cut})"),
                    c_ff(cb.as_ptr(), cut),
                    r_ff(rb.as_ptr(), cut),
                );
            }
        }
    }
    unsafe {
        c_free(cc);
        r_free(rc);
    }
}

// ============================================================ 3. size estimates

/// `ZSTD_estimateCDictSize{,_advanced}`, `ZSTD_estimateDDictSize`,
/// `ZSTD_sizeof_CDict`, `ZSTD_sizeof_DDict`. These leak internal struct sizes
/// and cwksp layout, so any layout divergence shows up here first.
#[test]
fn dict_size_estimates_match() {
    let i = impls();
    let (c_ec, r_ec) = i.pair::<Fn_estimateCDictSize>("ZSTD_estimateCDictSize");
    let (c_eca, r_eca) = i.pair::<Fn_estimateCDictSize_advanced>("ZSTD_estimateCDictSize_advanced");
    let (c_ed, r_ed) = i.pair::<Fn_estimateDDictSize>("ZSTD_estimateDDictSize");
    let (c_gcp, r_gcp) = i.pair::<Fn_getCParams>("ZSTD_getCParams");
    let (c_sc, r_sc) = i.pair::<Fn_sizeof_CDict>("ZSTD_sizeof_CDict");
    let (c_sd, r_sd) = i.pair::<Fn_sizeof_DDict>("ZSTD_sizeof_DDict");
    let (c_cc, r_cc) = i.pair::<Fn_createCDict>("ZSTD_createCDict");
    let (c_cfree, r_cfree) = i.pair::<Fn_freeCDict>("ZSTD_freeCDict");
    let (c_cd, r_cd) = i.pair::<Fn_createDDict_advanced>("ZSTD_createDDict_advanced");
    let (c_dfree, r_dfree) = i.pair::<Fn_freeDDict>("ZSTD_freeDDict");

    let dict_sizes = [
        0usize, 1, 7, 8, 9, 64, 100, 1023, 1024, 4096, 65_536, 100_000, 1 << 20,
    ];
    let levels = [-131_072i32, -1000, -5, -1, 0, 1, 3, 5, 9, 12, 17, 19, 22];

    for &ds in &dict_sizes {
        for &lvl in &levels {
            unsafe {
                assert_eq_dbg(
                    &format!("ZSTD_estimateCDictSize({ds}, {lvl})"),
                    c_ec(ds, lvl),
                    r_ec(ds, lvl),
                );
            }
            // ZSTD_getCParams itself is part of the dictionary surface
            let (p1, p2) = unsafe {
                (
                    c_gcp(lvl, ZSTD_CONTENTSIZE_UNKNOWN, ds),
                    r_gcp(lvl, ZSTD_CONTENTSIZE_UNKNOWN, ds),
                )
            };
            assert_eq_dbg(&format!("ZSTD_getCParams({lvl}, unknown, {ds})"), p1, p2);
            for &sz in &[0u64, 1, 1000, 1 << 20, ZSTD_CONTENTSIZE_UNKNOWN] {
                let (q1, q2) = unsafe { (c_gcp(lvl, sz, ds), r_gcp(lvl, sz, ds)) };
                assert_eq_dbg(&format!("ZSTD_getCParams({lvl}, {sz}, {ds})"), q1, q2);
            }
            for &dlm in &ALL_DLM {
                unsafe {
                    assert_eq_dbg(
                        &format!("ZSTD_estimateCDictSize_advanced({ds}, lvl={lvl}, dlm={dlm})"),
                        c_eca(ds, p1, dlm),
                        r_eca(ds, p1, dlm),
                    );
                }
            }
        }
        for &dlm in &ALL_DLM {
            unsafe {
                assert_eq_dbg(
                    &format!("ZSTD_estimateDDictSize({ds}, {dlm})"),
                    c_ed(ds, dlm),
                    r_ed(ds, dlm),
                );
            }
        }
    }

    // explicit cParams over every strategy, so ZSTD_sizeof_matchState is
    // exercised for each match-finder table layout
    for &s in &ALL_STRATEGIES {
        for &wl in &[10u32, 15, 20, 24] {
            let cp = CParams {
                window_log: wl,
                chain_log: wl.min(28),
                hash_log: wl.min(24),
                search_log: 4,
                min_match: if s == ZSTD_fast { 5 } else { 4 },
                target_length: 32,
                strategy: s,
            };
            for &ds in &[0usize, 1024, 100_000] {
                for &dlm in &ALL_DLM {
                    unsafe {
                        assert_eq_dbg(
                            &format!(
                                "ZSTD_estimateCDictSize_advanced(ds={ds}, strat={s}, wl={wl}, dlm={dlm})"
                            ),
                            c_eca(ds, cp, dlm),
                            r_eca(ds, cp, dlm),
                        );
                    }
                }
            }
        }
    }

    // sizeof on NULL and on live dictionaries
    unsafe {
        assert_eq_dbg(
            "ZSTD_sizeof_CDict(NULL)",
            c_sc(std::ptr::null_mut()),
            r_sc(std::ptr::null_mut()),
        );
        assert_eq_dbg(
            "ZSTD_sizeof_DDict(NULL)",
            c_sd(std::ptr::null_mut()),
            r_sd(std::ptr::null_mut()),
        );
    }
    for d in dict_corpus() {
        for &lvl in &[1i32, 3, 9, 19] {
            let (a, b) = unsafe { (c_cc(d.ptr(), d.len(), lvl), r_cc(d.ptr(), d.len(), lvl)) };
            let tag = format!("createCDict({}, lvl={lvl})", d.name);
            assert_eq_dbg(&format!("{tag} / created?"), a.is_null(), b.is_null());
            if !a.is_null() {
                unsafe {
                    assert_eq_dbg(&format!("{tag} / ZSTD_sizeof_CDict"), c_sc(a), r_sc(b));
                }
            }
            unsafe {
                c_cfree(a);
                r_cfree(b);
            }
        }
        for &dlm in &ALL_DLM {
            let cm = CustomMem::default_cmem();
            let (a, b) = unsafe {
                (
                    c_cd(d.ptr(), d.len(), dlm, ZSTD_dct_auto, cm),
                    r_cd(d.ptr(), d.len(), dlm, ZSTD_dct_auto, cm),
                )
            };
            let tag = format!("createDDict_advanced({}, dlm={dlm})", d.name);
            assert_eq_dbg(&format!("{tag} / created?"), a.is_null(), b.is_null());
            if !a.is_null() {
                unsafe {
                    assert_eq_dbg(&format!("{tag} / ZSTD_sizeof_DDict"), c_sd(a), r_sd(b));
                }
            }
            unsafe {
                c_dfree(a);
                r_dfree(b);
            }
        }
    }
}

// ==================================================== 4. one-shot usingDict API

/// `ZSTD_compress_usingDict` / `ZSTD_decompress_usingDict` over the whole
/// corpus, every dictionary size class, negative-to-max levels and many
/// randomized inputs. Frames must be byte identical and cross-decodable.
#[test]
fn compress_decompress_usingDict_match() {
    let i = impls();
    let (c_new, r_new) = i.pair::<Fn_createCCtx>("ZSTD_createCCtx");
    let (c_free, r_free) = i.pair::<Fn_freeCCtx>("ZSTD_freeCCtx");
    let (cd_new, rd_new) = i.pair::<Fn_createDCtx>("ZSTD_createDCtx");
    let (cd_free, rd_free) = i.pair::<Fn_freeDCtx>("ZSTD_freeDCtx");
    let (c_cud, r_cud) = i.pair::<Fn_compress_usingDict>("ZSTD_compress_usingDict");
    let (c_dud, r_dud) = i.pair::<Fn_decompress_usingDict>("ZSTD_decompress_usingDict");
    let (c_bound, _) = i.pair::<Fn_bound>("ZSTD_compressBound");
    let (c_ff, r_ff) = i.pair::<Fn_dictID_fromDict>("ZSTD_getDictID_fromFrame");
    let ec = ErrCmp::new();

    let (cc, rc) = unsafe { (c_new(), r_new()) };
    let (cd, rd) = unsafe { (cd_new(), rd_new()) };
    let mut rng = Rng::new(0x0DDC_0FFE);

    for d in dict_corpus() {
        let levels: &[i32] = if d.len() > 50_000 {
            &[-3, 1, 3, 9, 19]
        } else {
            &[-131_072, -5, -1, 0, 1, 3, 6, 9, 12, 19, 22]
        };
        for &lvl in levels {
            for src in inputs(&mut rng, 4) {
                let cap = unsafe { c_bound(src.len()) } + 64;
                let mut cb = vec![0xA5u8; cap];
                let mut rb = vec![0x5Au8; cap];
                let a = unsafe {
                    c_cud(
                        cc,
                        cb.as_mut_ptr(),
                        cap,
                        src.as_ptr(),
                        src.len(),
                        d.ptr(),
                        d.len(),
                        lvl,
                    )
                };
                let b = unsafe {
                    r_cud(
                        rc,
                        rb.as_mut_ptr(),
                        cap,
                        src.as_ptr(),
                        src.len(),
                        d.ptr(),
                        d.len(),
                        lvl,
                    )
                };
                let tag = format!(
                    "compress_usingDict(dict={}[{}], lvl={lvl}, srcSize={})",
                    d.name,
                    d.len(),
                    src.len()
                );
                if ec.check(&tag, a, b) {
                    continue;
                }
                assert_bytes_eq(&tag, &cb[..a], &rb[..b]);
                unsafe {
                    assert_eq_dbg(
                        &format!("{tag} / dictID_fromFrame"),
                        c_ff(cb.as_ptr(), a),
                        r_ff(rb.as_ptr(), b),
                    );
                }

                // cross round trip: Rust decodes the C frame and vice versa
                let mut d1 = vec![0u8; src.len() + 8];
                let mut d2 = vec![0u8; src.len() + 8];
                let n1 = unsafe {
                    r_dud(
                        rd,
                        d1.as_mut_ptr(),
                        d1.len(),
                        cb.as_ptr(),
                        a,
                        d.ptr(),
                        d.len(),
                    )
                };
                let n2 = unsafe {
                    c_dud(
                        cd,
                        d2.as_mut_ptr(),
                        d2.len(),
                        rb.as_ptr(),
                        b,
                        d.ptr(),
                        d.len(),
                    )
                };
                assert_eq_dbg(&format!("{tag} / rust decodes C frame"), n1, src.len());
                assert_eq_dbg(&format!("{tag} / C decodes rust frame"), n2, src.len());
                assert_bytes_eq(&format!("{tag} / payload"), &src, &d1[..n1]);
                assert_bytes_eq(&format!("{tag} / payload"), &src, &d2[..n2]);
            }
        }
    }

    unsafe {
        c_free(cc);
        r_free(rc);
        cd_free(cd);
        rd_free(rd);
    }
}

// ============================================================== 5. CDict / DDict

/// The digested-dictionary object path: creation through all four constructors,
/// `ZSTD_compress_usingCDict{,_advanced}`, and decoding via a matching DDict.
#[test]
fn cdict_ddict_compress_match() {
    let i = impls();
    let (c_new, r_new) = i.pair::<Fn_createCCtx>("ZSTD_createCCtx");
    let (c_free, r_free) = i.pair::<Fn_freeCCtx>("ZSTD_freeCCtx");
    let (cd_new, rd_new) = i.pair::<Fn_createDCtx>("ZSTD_createDCtx");
    let (cd_free, rd_free) = i.pair::<Fn_freeDCtx>("ZSTD_freeDCtx");
    let (c_ccd, r_ccd) = i.pair::<Fn_createCDict_advanced>("ZSTD_createCDict_advanced");
    let (c_cbr, r_cbr) = i.pair::<Fn_createCDict>("ZSTD_createCDict_byReference");
    let (c_cpl, r_cpl) = i.pair::<Fn_createCDict>("ZSTD_createCDict");
    let (c_cfree, r_cfree) = i.pair::<Fn_freeCDict>("ZSTD_freeCDict");
    let (c_cuc, r_cuc) = i.pair::<Fn_compress_usingCDict>("ZSTD_compress_usingCDict");
    let (c_cuca, r_cuca) =
        i.pair::<Fn_compress_usingCDict_advanced>("ZSTD_compress_usingCDict_advanced");
    let (c_dda, r_dda) = i.pair::<Fn_createDDict_advanced>("ZSTD_createDDict_advanced");
    let (c_dfree, r_dfree) = i.pair::<Fn_freeDDict>("ZSTD_freeDDict");
    let (c_dud, r_dud) = i.pair::<Fn_decompress_usingDDict>("ZSTD_decompress_usingDDict");
    let (c_gcp, _) = i.pair::<Fn_getCParams>("ZSTD_getCParams");
    let (c_sc, r_sc) = i.pair::<Fn_sizeof_CDict>("ZSTD_sizeof_CDict");
    let (c_fc, r_fc) = i.pair::<Fn_dictID_fromCDict>("ZSTD_getDictID_fromCDict");
    let (c_bound, _) = i.pair::<Fn_bound>("ZSTD_compressBound");
    let ec = ErrCmp::new();

    let (cc, rc) = unsafe { (c_new(), r_new()) };
    let (cd, rd) = unsafe { (cd_new(), rd_new()) };
    let mut rng = Rng::new(0xCD1C_7000);
    let cm = CustomMem::default_cmem();

    for d in dict_corpus() {
        for &dct in &ALL_DCT {
            for &dlm in &ALL_DLM {
                for &lvl in &[1i32, 9] {
                    let cp = unsafe { c_gcp(lvl, ZSTD_CONTENTSIZE_UNKNOWN, d.len()) };
                    let (ccd, rcd) = unsafe {
                        (
                            c_ccd(d.ptr(), d.len(), dlm, dct, cp, cm),
                            r_ccd(d.ptr(), d.len(), dlm, dct, cp, cm),
                        )
                    };
                    let tag = format!(
                        "createCDict_advanced({}, dct={dct}, dlm={dlm}, lvl={lvl})",
                        d.name
                    );
                    assert_eq_dbg(&format!("{tag} / created?"), ccd.is_null(), rcd.is_null());
                    if ccd.is_null() {
                        unsafe {
                            c_cfree(ccd);
                            r_cfree(rcd);
                        }
                        continue;
                    }
                    unsafe {
                        assert_eq_dbg(&format!("{tag} / sizeof"), c_sc(ccd), r_sc(rcd));
                        assert_eq_dbg(&format!("{tag} / dictID"), c_fc(ccd), r_fc(rcd));
                    }

                    // matching DDict for the round trip
                    let (cdd, rdd) = unsafe {
                        (
                            c_dda(d.ptr(), d.len(), dlm, dct, cm),
                            r_dda(d.ptr(), d.len(), dlm, dct, cm),
                        )
                    };
                    assert_eq_dbg(&format!("{tag} / DDict created?"), cdd.is_null(), rdd.is_null());

                    for src in inputs(&mut rng, 2) {
                        let cap = unsafe { c_bound(src.len()) } + 64;
                        let mut cb = vec![0u8; cap];
                        let mut rb = vec![0u8; cap];
                        let a = unsafe {
                            c_cuc(cc, cb.as_mut_ptr(), cap, src.as_ptr(), src.len(), ccd)
                        };
                        let b = unsafe {
                            r_cuc(rc, rb.as_mut_ptr(), cap, src.as_ptr(), src.len(), rcd)
                        };
                        let t2 = format!("{tag} / usingCDict srcSize={}", src.len());
                        if !ec.check(&t2, a, b) {
                            assert_bytes_eq(&t2, &cb[..a], &rb[..b]);
                            if !cdd.is_null() {
                                let mut o1 = vec![0u8; src.len() + 8];
                                let mut o2 = vec![0u8; src.len() + 8];
                                let n1 = unsafe {
                                    r_dud(rd, o1.as_mut_ptr(), o1.len(), cb.as_ptr(), a, rdd)
                                };
                                let n2 = unsafe {
                                    c_dud(cd, o2.as_mut_ptr(), o2.len(), rb.as_ptr(), b, cdd)
                                };
                                assert_eq_dbg(&format!("{t2} / rust decodes C"), n1, src.len());
                                assert_eq_dbg(&format!("{t2} / C decodes rust"), n2, src.len());
                                assert_bytes_eq(&format!("{t2} / payload"), &src, &o1[..n1]);
                                assert_bytes_eq(&format!("{t2} / payload"), &src, &o2[..n2]);
                            }
                        }

                        // ZSTD_compress_usingCDict_advanced with an fParams sweep
                        for &(csf, ckf, ndf) in
                            &[(1i32, 0i32, 0i32), (0, 1, 0), (1, 1, 1), (0, 0, 1)]
                        {
                            let fp = FParams {
                                content_size_flag: csf,
                                checksum_flag: ckf,
                                no_dict_id_flag: ndf,
                            };
                            let mut cb2 = vec![0u8; cap];
                            let mut rb2 = vec![0u8; cap];
                            let a2 = unsafe {
                                c_cuca(
                                    cc,
                                    cb2.as_mut_ptr(),
                                    cap,
                                    src.as_ptr(),
                                    src.len(),
                                    ccd,
                                    fp,
                                )
                            };
                            let b2 = unsafe {
                                r_cuca(
                                    rc,
                                    rb2.as_mut_ptr(),
                                    cap,
                                    src.as_ptr(),
                                    src.len(),
                                    rcd,
                                    fp,
                                )
                            };
                            let t3 = format!("{tag} / usingCDict_advanced fp={fp:?}");
                            if !ec.check(&t3, a2, b2) {
                                assert_bytes_eq(&t3, &cb2[..a2], &rb2[..b2]);
                            }
                        }
                    }

                    unsafe {
                        c_dfree(cdd);
                        r_dfree(rdd);
                        c_cfree(ccd);
                        r_cfree(rcd);
                    }
                }
            }
        }

        // ZSTD_createCDict / ZSTD_createCDict_byReference
        let (cpl_c, cpl_r): (Fn_createCDict, Fn_createCDict) = (*c_cpl, *r_cpl);
        let (cbr_c, cbr_r): (Fn_createCDict, Fn_createCDict) = (*c_cbr, *r_cbr);
        for &lvl in &[-5i32, 1, 3, 19] {
            for (name, cf, rf) in [
                ("ZSTD_createCDict", cpl_c, cpl_r),
                ("ZSTD_createCDict_byReference", cbr_c, cbr_r),
            ] {
                let (a, b) = unsafe { (cf(d.ptr(), d.len(), lvl), rf(d.ptr(), d.len(), lvl)) };
                let tag = format!("{name}({}, lvl={lvl})", d.name);
                assert_eq_dbg(&format!("{tag} / created?"), a.is_null(), b.is_null());
                if !a.is_null() {
                    unsafe {
                        assert_eq_dbg(&format!("{tag} / sizeof"), c_sc(a), r_sc(b));
                        assert_eq_dbg(&format!("{tag} / dictID"), c_fc(a), r_fc(b));
                    }
                    for src in inputs(&mut rng, 2) {
                        let cap = unsafe { c_bound(src.len()) } + 64;
                        let mut cb = vec![0u8; cap];
                        let mut rb = vec![0u8; cap];
                        let x = unsafe {
                            c_cuc(cc, cb.as_mut_ptr(), cap, src.as_ptr(), src.len(), a)
                        };
                        let y = unsafe {
                            r_cuc(rc, rb.as_mut_ptr(), cap, src.as_ptr(), src.len(), b)
                        };
                        let t2 = format!("{tag} / usingCDict srcSize={}", src.len());
                        if !ec.check(&t2, x, y) {
                            assert_bytes_eq(&t2, &cb[..x], &rb[..y]);
                        }
                    }
                }
                unsafe {
                    c_cfree(a);
                    r_cfree(b);
                }
            }
        }
    }

    unsafe {
        c_free(cc);
        r_free(rc);
        cd_free(cd);
        rd_free(rd);
    }
}

/// `ZSTD_createCDict_advanced2` drives the CDict through a `ZSTD_CCtx_params`
/// object, which is the only way to reach dedicated dict search / row hashing /
/// forceAttachDict at CDict *build* time.
#[test]
fn create_cdict_advanced2_matches() {
    let i = impls();
    let (c_new, r_new) = i.pair::<Fn_createCCtx>("ZSTD_createCCtx");
    let (c_free, r_free) = i.pair::<Fn_freeCCtx>("ZSTD_freeCCtx");
    let (c_pnew, r_pnew) = i.pair::<Fn_createCCtxParams>("ZSTD_createCCtxParams");
    let (c_pfree, r_pfree) = i.pair::<Fn_freeCCtxParams>("ZSTD_freeCCtxParams");
    let (c_pinit, r_pinit) = i.pair::<Fn_cctxParamsInit>("ZSTD_CCtxParams_init");
    let (c_pset, r_pset) = i.pair::<Fn_cctxParamsSet>("ZSTD_CCtxParams_setParameter");
    let (c_ca2, r_ca2) = i.pair::<Fn_createCDict_advanced2>("ZSTD_createCDict_advanced2");
    let (c_cfree, r_cfree) = i.pair::<Fn_freeCDict>("ZSTD_freeCDict");
    let (c_cuc, r_cuc) = i.pair::<Fn_compress_usingCDict>("ZSTD_compress_usingCDict");
    let (c_sc, r_sc) = i.pair::<Fn_sizeof_CDict>("ZSTD_sizeof_CDict");
    let (c_fc, r_fc) = i.pair::<Fn_dictID_fromCDict>("ZSTD_getDictID_fromCDict");
    let (c_bound, _) = i.pair::<Fn_bound>("ZSTD_compressBound");
    let ec = ErrCmp::new();

    let (cc, rc) = unsafe { (c_new(), r_new()) };
    let (cp, rp) = unsafe { (c_pnew(), r_pnew()) };
    let cm = CustomMem::default_cmem();
    let mut rng = Rng::new(0xAD02_0001);

    // parameter rows that materially change the CDict layout
    let mut rows: Vec<Vec<(i32, i32)>> = Vec::new();
    for &dds in &[0, 1] {
        for &lvl in &[1i32, 9, 19] {
            rows.push(vec![
                (ZSTD_c_enableDedicatedDictSearch, dds),
                (ZSTD_c_compressionLevel, lvl),
            ]);
        }
    }
    for &s in &ALL_STRATEGIES {
        rows.push(vec![
            (ZSTD_c_strategy, s),
            (ZSTD_c_compressionLevel, 5),
            (ZSTD_c_enableDedicatedDictSearch, 1),
        ]);
    }
    for &rmf in &[ZSTD_ps_auto, ZSTD_ps_enable, ZSTD_ps_disable] {
        rows.push(vec![
            (ZSTD_c_useRowMatchFinder, rmf),
            (ZSTD_c_strategy, ZSTD_greedy),
        ]);
    }
    for &fad in &[
        ZSTD_dictDefaultAttach,
        ZSTD_dictForceAttach,
        ZSTD_dictForceCopy,
        ZSTD_dictForceLoad,
    ] {
        rows.push(vec![(ZSTD_c_forceAttachDict, fad)]);
    }
    for &wl in &[10i32, 15, 20] {
        rows.push(vec![(ZSTD_c_windowLog, wl)]);
    }

    let specs = [
        spec("trained-8k"),
        spec("trained-100k"),
        spec("raw-text-1k"),
        spec("empty"),
        spec("tiny5"),
    ];

    for row in &rows {
        for d in specs {
            for &dct in &ALL_DCT {
                for &dlm in &ALL_DLM {
                    unsafe {
                        c_pinit(cp, 3);
                        r_pinit(rp, 3);
                    }
                    let mut bad = false;
                    for &(id, v) in row {
                        let (a, b) = unsafe { (c_pset(cp, id, v), r_pset(rp, id, v)) };
                        if ec.check(&format!("CCtxParams_setParameter({id},{v})"), a, b) {
                            bad = true;
                        }
                    }
                    if bad {
                        continue;
                    }
                    let (a, b) = unsafe {
                        (
                            c_ca2(d.ptr(), d.len(), dlm, dct, cp, cm),
                            r_ca2(d.ptr(), d.len(), dlm, dct, rp, cm),
                        )
                    };
                    let tag = format!(
                        "createCDict_advanced2({}, dct={dct}, dlm={dlm}, row={row:?})",
                        d.name
                    );
                    assert_eq_dbg(&format!("{tag} / created?"), a.is_null(), b.is_null());
                    if !a.is_null() {
                        unsafe {
                            assert_eq_dbg(&format!("{tag} / sizeof"), c_sc(a), r_sc(b));
                            assert_eq_dbg(&format!("{tag} / dictID"), c_fc(a), r_fc(b));
                        }
                        let src = gen_logish_range(&mut rng, 200, 30_000);
                        let cap = unsafe { c_bound(src.len()) } + 64;
                        let mut cb = vec![0u8; cap];
                        let mut rb = vec![0u8; cap];
                        let x = unsafe {
                            c_cuc(cc, cb.as_mut_ptr(), cap, src.as_ptr(), src.len(), a)
                        };
                        let y = unsafe {
                            r_cuc(rc, rb.as_mut_ptr(), cap, src.as_ptr(), src.len(), b)
                        };
                        if !ec.check(&format!("{tag} / usingCDict"), x, y) {
                            assert_bytes_eq(&format!("{tag} / usingCDict"), &cb[..x], &rb[..y]);
                        }
                    }
                    unsafe {
                        c_cfree(a);
                        r_cfree(b);
                    }
                }
            }
        }
    }

    unsafe {
        c_pfree(cp);
        r_pfree(rp);
        c_free(cc);
        r_free(rc);
    }
}

// ==================================================== 6. cctx dictionary setters

/// `ZSTD_CCtx_loadDictionary{,_byReference,_advanced}` and `ZSTD_CCtx_refCDict`
/// crossed with every dictionary-related compression parameter:
/// forceAttachDict, enableDedicatedDictSearch, prefetchCDictTables,
/// dictIDFlag, all nine strategies and a level sweep.
#[test]
fn cctx_load_dictionary_and_ref_cdict_match() {
    let i = impls();
    let (c_new, r_new) = i.pair::<Fn_createCCtx>("ZSTD_createCCtx");
    let (c_free, r_free) = i.pair::<Fn_freeCCtx>("ZSTD_freeCCtx");
    let (c_rst, r_rst) = i.pair::<Fn_reset>("ZSTD_CCtx_reset");
    let (c_set, r_set) = i.pair::<Fn_setParam>("ZSTD_CCtx_setParameter");
    let (c_c2, r_c2) = i.pair::<Fn_compress2>("ZSTD_compress2");
    let (c_ld, r_ld) = i.pair::<Fn_loadDict>("ZSTD_CCtx_loadDictionary");
    let (c_ldr, r_ldr) = i.pair::<Fn_loadDict>("ZSTD_CCtx_loadDictionary_byReference");
    let (c_lda, r_lda) = i.pair::<Fn_loadDict_advanced>("ZSTD_CCtx_loadDictionary_advanced");
    let (c_rcd, r_rcd) = i.pair::<Fn_refCDict>("ZSTD_CCtx_refCDict");
    let (c_ccd, r_ccd) = i.pair::<Fn_createCDict_advanced>("ZSTD_createCDict_advanced");
    let (c_cfree, r_cfree) = i.pair::<Fn_freeCDict>("ZSTD_freeCDict");
    let (c_gcp, _) = i.pair::<Fn_getCParams>("ZSTD_getCParams");
    let (cd_new, rd_new) = i.pair::<Fn_createDCtx>("ZSTD_createDCtx");
    let (cd_free, rd_free) = i.pair::<Fn_freeDCtx>("ZSTD_freeDCtx");
    let (c_dud, r_dud) = i.pair::<Fn_decompress_usingDict>("ZSTD_decompress_usingDict");
    let (c_bound, _) = i.pair::<Fn_bound>("ZSTD_compressBound");
    let ec = ErrCmp::new();

    // ------- parameter rows
    let mut rows: Vec<(&'static str, Vec<(i32, i32)>)> = Vec::new();
    for &fad in &[
        ZSTD_dictDefaultAttach,
        ZSTD_dictForceAttach,
        ZSTD_dictForceCopy,
        ZSTD_dictForceLoad,
    ] {
        for &lvl in &[1i32, 5, 12, 19] {
            rows.push((
                "forceAttachDict",
                vec![
                    (ZSTD_c_forceAttachDict, fad),
                    (ZSTD_c_compressionLevel, lvl),
                ],
            ));
        }
    }
    for &dds in &[0, 1] {
        for &lvl in &[1i32, 9, 19] {
            rows.push((
                "enableDedicatedDictSearch",
                vec![
                    (ZSTD_c_enableDedicatedDictSearch, dds),
                    (ZSTD_c_compressionLevel, lvl),
                ],
            ));
        }
    }
    for &pf in &[ZSTD_ps_auto, ZSTD_ps_enable, ZSTD_ps_disable] {
        rows.push((
            "prefetchCDictTables",
            vec![(ZSTD_c_prefetchCDictTables, pf)],
        ));
    }
    for &di in &[0, 1] {
        rows.push(("dictIDFlag", vec![(ZSTD_c_dictIDFlag, di)]));
    }
    for &s in &ALL_STRATEGIES {
        rows.push((
            "strategy",
            vec![(ZSTD_c_strategy, s), (ZSTD_c_compressionLevel, 6)],
        ));
    }
    for &lvl in &[-131_072i32, -1000, -5, -1, 0, 1, 3, 9, 15, 19, 22] {
        rows.push(("level", vec![(ZSTD_c_compressionLevel, lvl)]));
    }
    for &rmf in &[ZSTD_ps_auto, ZSTD_ps_enable, ZSTD_ps_disable] {
        rows.push((
            "useRowMatchFinder",
            vec![(ZSTD_c_useRowMatchFinder, rmf), (ZSTD_c_strategy, ZSTD_lazy2)],
        ));
    }
    for &(ck, cs) in &[(0, 0), (1, 0), (0, 1), (1, 1)] {
        rows.push((
            "frame-flags",
            vec![(ZSTD_c_checksumFlag, ck), (ZSTD_c_contentSizeFlag, cs)],
        ));
    }
    rows.push((
        "ldm+dict",
        vec![
            (ZSTD_c_enableLongDistanceMatching, 1),
            (ZSTD_c_windowLog, 20),
        ],
    ));

    let (cc, rc) = unsafe { (c_new(), r_new()) };
    let (cdx, rdx) = unsafe { (cd_new(), rd_new()) };
    let mut rng = Rng::new(0x10AD_D1C7);
    let corpus = dict_corpus();
    let cm = CustomMem::default_cmem();

    // 0 = loadDictionary, 1 = loadDictionary_byReference,
    // 2 = loadDictionary_advanced, 3 = refCDict
    for row in &rows {
        for method in 0..4 {
            for trial in 0..3 {
                let d = &corpus[rng.below(corpus.len())];
                let dct = ALL_DCT[rng.below(3)];
                let dlm = ALL_DLM[rng.below(2)];
                let src = if trial == 0 {
                    gen_logish_range(&mut rng, 0, 2_000)
                } else if trial == 1 {
                    gen_logish_range(&mut rng, 2_000, 40_000)
                } else {
                    let s = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
                    { let n = rng.range(100_000, 160_000); gen_shape(s, n, &mut rng) }
                };

                unsafe {
                    c_rst(cc, ZSTD_reset_session_and_parameters);
                    r_rst(rc, ZSTD_reset_session_and_parameters);
                }
                let mut bad = false;
                for &(id, v) in &row.1 {
                    let (a, b) = unsafe { (c_set(cc, id, v), r_set(rc, id, v)) };
                    if ec.check(&format!("[{}] setParameter({id},{v})", row.0), a, b) {
                        bad = true;
                    }
                }
                if bad {
                    continue;
                }

                let mut ccd: CDict = std::ptr::null_mut();
                let mut rcd: CDict = std::ptr::null_mut();
                let tag = format!(
                    "[{}] method={method} dict={} dct={dct} dlm={dlm} srcSize={} params={:?}",
                    row.0,
                    d.name,
                    src.len(),
                    row.1
                );
                let (a, b) = unsafe {
                    match method {
                        0 => (c_ld(cc, d.ptr(), d.len()), r_ld(rc, d.ptr(), d.len())),
                        1 => (c_ldr(cc, d.ptr(), d.len()), r_ldr(rc, d.ptr(), d.len())),
                        2 => (
                            c_lda(cc, d.ptr(), d.len(), dlm, dct),
                            r_lda(rc, d.ptr(), d.len(), dlm, dct),
                        ),
                        _ => {
                            let cp = c_gcp(3, ZSTD_CONTENTSIZE_UNKNOWN, d.len());
                            ccd = c_ccd(d.ptr(), d.len(), dlm, dct, cp, cm);
                            rcd = r_ccd(d.ptr(), d.len(), dlm, dct, cp, cm);
                            assert_eq_dbg(
                                &format!("{tag} / CDict created?"),
                                ccd.is_null(),
                                rcd.is_null(),
                            );
                            (c_rcd(cc, ccd), r_rcd(rc, rcd))
                        }
                    }
                };
                let load_failed = ec.check(&format!("{tag} / load"), a, b);

                if !load_failed {
                    let cap = unsafe { c_bound(src.len()) } + 64;
                    let mut cb = vec![0u8; cap];
                    let mut rb = vec![0u8; cap];
                    let x = unsafe { c_c2(cc, cb.as_mut_ptr(), cap, src.as_ptr(), src.len()) };
                    let y = unsafe { r_c2(rc, rb.as_mut_ptr(), cap, src.as_ptr(), src.len()) };
                    if !ec.check(&format!("{tag} / compress2"), x, y) {
                        assert_bytes_eq(&format!("{tag} / frame"), &cb[..x], &rb[..y]);
                        // decode with the raw dictionary buffer through both libs
                        let mut o1 = vec![0u8; src.len() + 8];
                        let mut o2 = vec![0u8; src.len() + 8];
                        let n1 = unsafe {
                            r_dud(rdx, o1.as_mut_ptr(), o1.len(), cb.as_ptr(), x, d.ptr(), d.len())
                        };
                        let n2 = unsafe {
                            c_dud(cdx, o2.as_mut_ptr(), o2.len(), rb.as_ptr(), y, d.ptr(), d.len())
                        };
                        assert_eq_dbg(&format!("{tag} / decode rc"), n1, n2);
                        if !is_err(n1) {
                            assert_eq_dbg(&format!("{tag} / decode len"), n1, src.len());
                            assert_bytes_eq(&format!("{tag} / payload"), &src, &o1[..n1]);
                            assert_bytes_eq(&format!("{tag} / payload"), &src, &o2[..n2]);
                        }
                    }
                }
                unsafe {
                    // the cctx still references the cdict: clear before freeing
                    c_rst(cc, ZSTD_reset_session_and_parameters);
                    r_rst(rc, ZSTD_reset_session_and_parameters);
                    c_cfree(ccd);
                    r_cfree(rcd);
                }
            }
        }
    }

    unsafe {
        c_free(cc);
        r_free(rc);
        cd_free(cdx);
        rd_free(rdx);
    }
}

// ================================================================= 7. refPrefix

/// `ZSTD_CCtx_refPrefix{,_advanced}` / `ZSTD_DCtx_refPrefix{,_advanced}`,
/// including `ZSTD_c_deterministicRefPrefix` and the single-use semantics
/// (the prefix must apply to exactly one frame).
#[test]
fn ref_prefix_match() {
    let i = impls();
    let (c_new, r_new) = i.pair::<Fn_createCCtx>("ZSTD_createCCtx");
    let (c_free, r_free) = i.pair::<Fn_freeCCtx>("ZSTD_freeCCtx");
    let (c_rst, r_rst) = i.pair::<Fn_reset>("ZSTD_CCtx_reset");
    let (c_set, r_set) = i.pair::<Fn_setParam>("ZSTD_CCtx_setParameter");
    let (c_c2, r_c2) = i.pair::<Fn_compress2>("ZSTD_compress2");
    let (c_rp, r_rp) = i.pair::<Fn_refPrefix>("ZSTD_CCtx_refPrefix");
    let (c_rpa, r_rpa) = i.pair::<Fn_refPrefix_advanced>("ZSTD_CCtx_refPrefix_advanced");
    let (cd_new, rd_new) = i.pair::<Fn_createDCtx>("ZSTD_createDCtx");
    let (cd_free, rd_free) = i.pair::<Fn_freeDCtx>("ZSTD_freeDCtx");
    let (cd_rst, rd_rst) = i.pair::<Fn_dReset>("ZSTD_DCtx_reset");
    let (c_drp, r_drp) = i.pair::<Fn_dRefPrefix>("ZSTD_DCtx_refPrefix");
    let (c_drpa, r_drpa) = i.pair::<Fn_dRefPrefix_advanced>("ZSTD_DCtx_refPrefix_advanced");
    let (c_dec, r_dec) = i.pair::<Fn_decompressDCtx>("ZSTD_decompressDCtx");
    let (c_bound, _) = i.pair::<Fn_bound>("ZSTD_compressBound");
    let ec = ErrCmp::new();

    let (cc, rc) = unsafe { (c_new(), r_new()) };
    let (cd, rd) = unsafe { (cd_new(), rd_new()) };
    let mut rng = Rng::new(0x9EFF_1200);
    let corpus = dict_corpus();

    for d in corpus {
        for &advanced in &[false, true] {
            for &dct in &ALL_DCT {
                if !advanced && dct != ZSTD_dct_rawContent {
                    continue; // plain refPrefix is always rawContent
                }
                for &det in &[0i32, 1] {
                    for &lvl in &[1i32, 5, 12, 19] {
                        for trial in 0..2 {
                            let src = if trial == 0 {
                                gen_logish_range(&mut rng, 1, 3_000)
                            } else {
                                gen_logish_range(&mut rng, 3_000, 50_000)
                            };
                            unsafe {
                                c_rst(cc, ZSTD_reset_session_and_parameters);
                                r_rst(rc, ZSTD_reset_session_and_parameters);
                                let (a, b) = (
                                    c_set(cc, ZSTD_c_deterministicRefPrefix, det),
                                    r_set(rc, ZSTD_c_deterministicRefPrefix, det),
                                );
                                ec.check("setParameter(deterministicRefPrefix)", a, b);
                                let (a, b) = (
                                    c_set(cc, ZSTD_c_compressionLevel, lvl),
                                    r_set(rc, ZSTD_c_compressionLevel, lvl),
                                );
                                ec.check("setParameter(compressionLevel)", a, b);
                            }
                            let (a, b) = unsafe {
                                if advanced {
                                    (
                                        c_rpa(cc, d.ptr(), d.len(), dct),
                                        r_rpa(rc, d.ptr(), d.len(), dct),
                                    )
                                } else {
                                    (c_rp(cc, d.ptr(), d.len()), r_rp(rc, d.ptr(), d.len()))
                                }
                            };
                            let tag = format!(
                                "refPrefix(adv={advanced}, {}, dct={dct}, det={det}, lvl={lvl}, srcSize={})",
                                d.name,
                                src.len()
                            );
                            if ec.check(&format!("{tag} / refPrefix"), a, b) {
                                continue;
                            }

                            let cap = unsafe { c_bound(src.len()) } + 64;
                            let mut cb = vec![0u8; cap];
                            let mut rb = vec![0u8; cap];
                            let x =
                                unsafe { c_c2(cc, cb.as_mut_ptr(), cap, src.as_ptr(), src.len()) };
                            let y =
                                unsafe { r_c2(rc, rb.as_mut_ptr(), cap, src.as_ptr(), src.len()) };
                            if ec.check(&format!("{tag} / frame1"), x, y) {
                                continue;
                            }
                            assert_bytes_eq(&format!("{tag} / frame1"), &cb[..x], &rb[..y]);

                            // decode with the prefix on the dctx (cross library)
                            let mut o1 = vec![0u8; src.len() + 8];
                            let mut o2 = vec![0u8; src.len() + 8];
                            unsafe {
                                cd_rst(cd, ZSTD_reset_session_and_parameters);
                                rd_rst(rd, ZSTD_reset_session_and_parameters);
                                if advanced {
                                    let (p, q) = (
                                        c_drpa(cd, d.ptr(), d.len(), dct),
                                        r_drpa(rd, d.ptr(), d.len(), dct),
                                    );
                                    ec.check(&format!("{tag} / DCtx_refPrefix_advanced"), p, q);
                                } else {
                                    let (p, q) =
                                        (c_drp(cd, d.ptr(), d.len()), r_drp(rd, d.ptr(), d.len()));
                                    ec.check(&format!("{tag} / DCtx_refPrefix"), p, q);
                                }
                            }
                            let n2 = unsafe {
                                c_dec(cd, o2.as_mut_ptr(), o2.len(), rb.as_ptr(), y)
                            };
                            // reset + prefix again for the other library's dctx
                            unsafe {
                                rd_rst(rd, ZSTD_reset_session_and_parameters);
                                if advanced {
                                    r_drpa(rd, d.ptr(), d.len(), dct);
                                } else {
                                    r_drp(rd, d.ptr(), d.len());
                                }
                            }
                            let n1 = unsafe {
                                r_dec(rd, o1.as_mut_ptr(), o1.len(), cb.as_ptr(), x)
                            };
                            assert_eq_dbg(&format!("{tag} / decode rc"), n1, n2);
                            if !is_err(n1) {
                                assert_eq_dbg(&format!("{tag} / decode len"), n1, src.len());
                                assert_bytes_eq(&format!("{tag} / payload"), &src, &o1[..n1]);
                                assert_bytes_eq(&format!("{tag} / payload"), &src, &o2[..n2]);
                            }

                            // ---- single-use: the SAME cctx, no new refPrefix.
                            // The prefix must have been consumed by frame 1, so
                            // frame 2 must be a plain dictionary-less frame.
                            let src2 = gen_logish_range(&mut rng, 100, 4_000);
                            let cap2 = unsafe { c_bound(src2.len()) } + 64;
                            let mut cb2 = vec![0u8; cap2];
                            let mut rb2 = vec![0u8; cap2];
                            let x2 = unsafe {
                                c_c2(cc, cb2.as_mut_ptr(), cap2, src2.as_ptr(), src2.len())
                            };
                            let y2 = unsafe {
                                r_c2(rc, rb2.as_mut_ptr(), cap2, src2.as_ptr(), src2.len())
                            };
                            if !ec.check(&format!("{tag} / frame2 (prefix consumed)"), x2, y2) {
                                assert_bytes_eq(
                                    &format!("{tag} / frame2 (prefix consumed)"),
                                    &cb2[..x2],
                                    &rb2[..y2],
                                );
                                // frame 2 must decode without any prefix
                                let mut p1 = vec![0u8; src2.len() + 8];
                                let mut p2 = vec![0u8; src2.len() + 8];
                                unsafe {
                                    cd_rst(cd, ZSTD_reset_session_and_parameters);
                                    rd_rst(rd, ZSTD_reset_session_and_parameters);
                                }
                                let m1 = unsafe {
                                    r_dec(rd, p1.as_mut_ptr(), p1.len(), cb2.as_ptr(), x2)
                                };
                                let m2 = unsafe {
                                    c_dec(cd, p2.as_mut_ptr(), p2.len(), rb2.as_ptr(), y2)
                                };
                                assert_eq_dbg(&format!("{tag} / frame2 decode"), m1, m2);
                                if !is_err(m1) {
                                    assert_bytes_eq(
                                        &format!("{tag} / frame2 payload"),
                                        &src2,
                                        &p1[..m1],
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    unsafe {
        c_free(cc);
        r_free(rc);
        cd_free(cd);
        rd_free(rd);
    }
}

// =================================================== 8. dctx dictionary setters

/// `ZSTD_DCtx_loadDictionary{,_byReference,_advanced}`, `ZSTD_DCtx_refDDict`
/// and `ZSTD_decompress_usingDDict` must all decode the *other* library's
/// dictionary frames identically, and reject identically.
#[test]
fn dctx_dictionary_setters_match() {
    let i = impls();
    let (c_new, r_new) = i.pair::<Fn_createCCtx>("ZSTD_createCCtx");
    let (c_free, r_free) = i.pair::<Fn_freeCCtx>("ZSTD_freeCCtx");
    let (c_cud, r_cud) = i.pair::<Fn_compress_usingDict>("ZSTD_compress_usingDict");
    let (cd_new, rd_new) = i.pair::<Fn_createDCtx>("ZSTD_createDCtx");
    let (cd_free, rd_free) = i.pair::<Fn_freeDCtx>("ZSTD_freeDCtx");
    let (cd_rst, rd_rst) = i.pair::<Fn_dReset>("ZSTD_DCtx_reset");
    let (c_dl, r_dl) = i.pair::<Fn_dLoadDict>("ZSTD_DCtx_loadDictionary");
    let (c_dlr, r_dlr) = i.pair::<Fn_dLoadDict>("ZSTD_DCtx_loadDictionary_byReference");
    let (c_dla, r_dla) = i.pair::<Fn_dLoadDict_advanced>("ZSTD_DCtx_loadDictionary_advanced");
    let (c_rdd, r_rdd) = i.pair::<Fn_refDDict>("ZSTD_DCtx_refDDict");
    let (c_dda, r_dda) = i.pair::<Fn_createDDict_advanced>("ZSTD_createDDict_advanced");
    let (c_dbr, r_dbr) = i.pair::<Fn_createDDict>("ZSTD_createDDict_byReference");
    let (c_dpl, r_dpl) = i.pair::<Fn_createDDict>("ZSTD_createDDict");
    let (c_dfree, r_dfree) = i.pair::<Fn_freeDDict>("ZSTD_freeDDict");
    let (c_dud, r_dud) = i.pair::<Fn_decompress_usingDDict>("ZSTD_decompress_usingDDict");
    let (c_dec, r_dec) = i.pair::<Fn_decompressDCtx>("ZSTD_decompressDCtx");
    let (c_bound, _) = i.pair::<Fn_bound>("ZSTD_compressBound");
    let ec = ErrCmp::new();

    let (cc, rc) = unsafe { (c_new(), r_new()) };
    let (cd, rd) = unsafe { (cd_new(), rd_new()) };
    let cm = CustomMem::default_cmem();
    let mut rng = Rng::new(0xDCD1_C700);

    for d in dict_corpus() {
        for &lvl in &[1i32, 9] {
            let src = gen_logish_range(&mut rng, 500, 40_000);
            let cap = unsafe { c_bound(src.len()) } + 64;
            let mut frame = vec![0u8; cap];
            let n = unsafe {
                c_cud(
                    cc,
                    frame.as_mut_ptr(),
                    cap,
                    src.as_ptr(),
                    src.len(),
                    d.ptr(),
                    d.len(),
                    lvl,
                )
            };
            // the same frame from the Rust compressor must be byte identical;
            // the decode matrix below then runs against this single frame.
            {
                let mut rframe = vec![0u8; cap];
                let m = unsafe {
                    r_cud(
                        rc,
                        rframe.as_mut_ptr(),
                        cap,
                        src.as_ptr(),
                        src.len(),
                        d.ptr(),
                        d.len(),
                        lvl,
                    )
                };
                let tag = format!("compress_usingDict({}, lvl={lvl})", d.name);
                if !ec.check(&tag, n, m) {
                    assert_bytes_eq(&tag, &frame[..n], &rframe[..m]);
                }
            }
            if is_err(n) {
                continue;
            }

            // --- loadDictionary variants
            for method in 0..3 {
                for &dct in &ALL_DCT {
                    for &dlm in &ALL_DLM {
                        unsafe {
                            cd_rst(cd, ZSTD_reset_session_and_parameters);
                            rd_rst(rd, ZSTD_reset_session_and_parameters);
                        }
                        let tag = format!(
                            "DCtx_loadDictionary(m={method}, {}, dct={dct}, dlm={dlm}, lvl={lvl})",
                            d.name
                        );
                        let (a, b) = unsafe {
                            match method {
                                0 => (c_dl(cd, d.ptr(), d.len()), r_dl(rd, d.ptr(), d.len())),
                                1 => (c_dlr(cd, d.ptr(), d.len()), r_dlr(rd, d.ptr(), d.len())),
                                _ => (
                                    c_dla(cd, d.ptr(), d.len(), dlm, dct),
                                    r_dla(rd, d.ptr(), d.len(), dlm, dct),
                                ),
                            }
                        };
                        if ec.check(&format!("{tag} / load"), a, b) {
                            continue;
                        }
                        let mut o1 = vec![0u8; src.len() + 8];
                        let mut o2 = vec![0u8; src.len() + 8];
                        let n1 = unsafe {
                            c_dec(cd, o1.as_mut_ptr(), o1.len(), frame.as_ptr(), n)
                        };
                        let n2 = unsafe {
                            r_dec(rd, o2.as_mut_ptr(), o2.len(), frame.as_ptr(), n)
                        };
                        if !ec.check(&format!("{tag} / decode"), n1, n2) {
                            assert_bytes_eq(&format!("{tag} / payload"), &o1[..n1], &o2[..n2]);
                        }
                    }
                }
            }

            // --- DDict constructors + refDDict + decompress_usingDDict
            for ctor in 0..3 {
                for &dct in &ALL_DCT {
                    for &dlm in &ALL_DLM {
                        if ctor != 2 && (dct != ZSTD_dct_auto) {
                            continue;
                        }
                        if ctor == 0 && dlm != ZSTD_dlm_byCopy {
                            continue;
                        }
                        if ctor == 1 && dlm != ZSTD_dlm_byRef {
                            continue;
                        }
                        let (cdd, rdd) = unsafe {
                            match ctor {
                                0 => (c_dpl(d.ptr(), d.len()), r_dpl(d.ptr(), d.len())),
                                1 => (c_dbr(d.ptr(), d.len()), r_dbr(d.ptr(), d.len())),
                                _ => (
                                    c_dda(d.ptr(), d.len(), dlm, dct, cm),
                                    r_dda(d.ptr(), d.len(), dlm, dct, cm),
                                ),
                            }
                        };
                        let tag =
                            format!("DDict(ctor={ctor}, {}, dct={dct}, dlm={dlm})", d.name);
                        assert_eq_dbg(
                            &format!("{tag} / created?"),
                            cdd.is_null(),
                            rdd.is_null(),
                        );
                        if cdd.is_null() {
                            unsafe {
                                c_dfree(cdd);
                                r_dfree(rdd);
                            }
                            continue;
                        }

                        let mut o1 = vec![0u8; src.len() + 8];
                        let mut o2 = vec![0u8; src.len() + 8];
                        unsafe {
                            cd_rst(cd, ZSTD_reset_session_and_parameters);
                            rd_rst(rd, ZSTD_reset_session_and_parameters);
                        }
                        let n1 = unsafe {
                            c_dud(cd, o1.as_mut_ptr(), o1.len(), frame.as_ptr(), n, cdd)
                        };
                        let n2 = unsafe {
                            r_dud(rd, o2.as_mut_ptr(), o2.len(), frame.as_ptr(), n, rdd)
                        };
                        if !ec.check(&format!("{tag} / decompress_usingDDict"), n1, n2) {
                            assert_bytes_eq(&format!("{tag} / payload"), &o1[..n1], &o2[..n2]);
                        }

                        // refDDict + ZSTD_decompressDCtx
                        unsafe {
                            cd_rst(cd, ZSTD_reset_session_and_parameters);
                            rd_rst(rd, ZSTD_reset_session_and_parameters);
                            let (p, q) = (c_rdd(cd, cdd), r_rdd(rd, rdd));
                            ec.check(&format!("{tag} / refDDict"), p, q);
                        }
                        let mut o3 = vec![0u8; src.len() + 8];
                        let mut o4 = vec![0u8; src.len() + 8];
                        let n3 = unsafe {
                            c_dec(cd, o3.as_mut_ptr(), o3.len(), frame.as_ptr(), n)
                        };
                        let n4 = unsafe {
                            r_dec(rd, o4.as_mut_ptr(), o4.len(), frame.as_ptr(), n)
                        };
                        if !ec.check(&format!("{tag} / refDDict decode"), n3, n4) {
                            assert_bytes_eq(&format!("{tag} / payload2"), &o3[..n3], &o4[..n4]);
                        }
                        unsafe {
                            // drop the reference before freeing the ddict
                            cd_rst(cd, ZSTD_reset_session_and_parameters);
                            rd_rst(rd, ZSTD_reset_session_and_parameters);
                            c_rdd(cd, std::ptr::null_mut());
                            r_rdd(rd, std::ptr::null_mut());
                            c_dfree(cdd);
                            r_dfree(rdd);
                        }
                    }
                }
            }
        }
    }

    // NULL ddict / NULL dictionary are documented no-ops
    unsafe {
        cd_rst(cd, ZSTD_reset_session_and_parameters);
        rd_rst(rd, ZSTD_reset_session_and_parameters);
        let (a, b) = (
            c_rdd(cd, std::ptr::null_mut()),
            r_rdd(rd, std::ptr::null_mut()),
        );
        ec.check("ZSTD_DCtx_refDDict(NULL)", a, b);
        let (a, b) = (c_dl(cd, std::ptr::null(), 0), r_dl(rd, std::ptr::null(), 0));
        ec.check("ZSTD_DCtx_loadDictionary(NULL, 0)", a, b);
    }

    unsafe {
        c_free(cc);
        r_free(rc);
        cd_free(cd);
        rd_free(rd);
    }
}

// ============================================================ 9. the wrong dict

/// Decompressing with the *wrong* dictionary (or none at all) must fail with
/// `ZSTD_error_dictionary_wrong` in both libraries.
#[test]
fn wrong_dictionary_is_rejected_identically() {
    let i = impls();
    let (c_new, r_new) = i.pair::<Fn_createCCtx>("ZSTD_createCCtx");
    let (c_free, r_free) = i.pair::<Fn_freeCCtx>("ZSTD_freeCCtx");
    let (cd_new, rd_new) = i.pair::<Fn_createDCtx>("ZSTD_createDCtx");
    let (cd_free, rd_free) = i.pair::<Fn_freeDCtx>("ZSTD_freeDCtx");
    let (c_cud, _r_cud) = i.pair::<Fn_compress_usingDict>("ZSTD_compress_usingDict");
    let (c_dud, r_dud) = i.pair::<Fn_decompress_usingDict>("ZSTD_decompress_usingDict");
    let (c_dec, r_dec) = i.pair::<Fn_decompressDCtx>("ZSTD_decompressDCtx");
    let (c_fd, _) = i.pair::<Fn_dictID_fromDict>("ZSTD_getDictID_fromDict");
    let (c_bound, _) = i.pair::<Fn_bound>("ZSTD_compressBound");
    let ec = ErrCmp::new();

    let (cc, rc) = unsafe { (c_new(), r_new()) };
    let (cd, rd) = unsafe { (cd_new(), rd_new()) };
    let mut rng = Rng::new(0x_BAD_D1C7);

    // Full, *loadable* dictionaries carrying a nonzero dictID — those are the
    // only ones for which the decoder can detect a mismatch. (A corrupted
    // dictionary would be rejected as `dictionary_corrupted` instead.)
    let (c_dda, _) = i.pair::<Fn_createDDict>("ZSTD_createDDict");
    let (c_dfree, _) = i.pair::<Fn_freeDDict>("ZSTD_freeDDict");
    let full: Vec<&DictSpec> = dict_corpus()
        .iter()
        .filter(|d| unsafe {
            if c_fd(d.ptr(), d.len()) == 0 {
                return false;
            }
            let h = c_dda(d.ptr(), d.len());
            let ok = !h.is_null();
            c_dfree(h);
            ok
        })
        .collect();
    assert!(full.len() >= 2, "need >= 2 loadable full dictionaries with a dictID");
    // distinct dictIDs are what makes the mismatch detectable
    for a in &full {
        for b in &full {
            if std::ptr::eq(*a, *b) {
                continue;
            }
            unsafe {
                assert!(
                    c_fd(a.ptr(), a.len()) != c_fd(b.ptr(), b.len()),
                    "corpus dictionaries {} and {} collide on dictID",
                    a.name,
                    b.name
                );
            }
        }
    }

    for good in &full {
        for &lvl in &[1i32, 9, 19] {
            let src = gen_logish_range(&mut rng, 400, 20_000);
            let cap = unsafe { c_bound(src.len()) } + 64;
            let mut frame = vec![0u8; cap];
            let n = unsafe {
                c_cud(
                    cc,
                    frame.as_mut_ptr(),
                    cap,
                    src.as_ptr(),
                    src.len(),
                    good.ptr(),
                    good.len(),
                    lvl,
                )
            };
            assert!(!is_err(n), "compress with {} failed: {n:#x}", good.name);
            // sanity: the reference implementation round-trips with the right dict
            {
                let mut o = vec![0u8; src.len() + 8];
                let ok = unsafe {
                    c_dud(
                        cd,
                        o.as_mut_ptr(),
                        o.len(),
                        frame.as_ptr(),
                        n,
                        good.ptr(),
                        good.len(),
                    )
                };
                assert_eq_dbg("self round trip", ok, src.len());
            }

            let mut probes: Vec<(&str, *const u8, usize)> = Vec::new();
            for other in &full {
                if std::ptr::eq(*other, *good) {
                    continue;
                }
                probes.push((other.name, other.ptr(), other.len()));
            }
            probes.push(("<none>", std::ptr::null(), 0));
            probes.push((
                "raw-text-1k",
                spec("raw-text-1k").ptr(),
                spec("raw-text-1k").len(),
            ));

            for (name, p, l) in probes {
                let mut o1 = vec![0u8; src.len() + 8];
                let mut o2 = vec![0u8; src.len() + 8];
                let a = unsafe { c_dud(cd, o1.as_mut_ptr(), o1.len(), frame.as_ptr(), n, p, l) };
                let b = unsafe { r_dud(rd, o2.as_mut_ptr(), o2.len(), frame.as_ptr(), n, p, l) };
                let tag = format!(
                    "decompress(dict={} frame, using dict={name}, lvl={lvl})",
                    good.name
                );
                let errored = ec.check(&tag, a, b);
                assert!(errored, "{tag}: the wrong dictionary must be rejected");
                assert_eq_dbg(
                    &format!("{tag} / must be dictionary_wrong"),
                    ec.code(a),
                    ZSTD_error_dictionary_wrong,
                );
            }

            // and with no dictionary at all through the plain DCtx entry point
            let mut o1 = vec![0u8; src.len() + 8];
            let mut o2 = vec![0u8; src.len() + 8];
            let a = unsafe { c_dec(cd, o1.as_mut_ptr(), o1.len(), frame.as_ptr(), n) };
            let b = unsafe { r_dec(rd, o2.as_mut_ptr(), o2.len(), frame.as_ptr(), n) };
            let tag = format!("decompressDCtx(dict={} frame, no dict)", good.name);
            assert!(ec.check(&tag, a, b));
            assert_eq_dbg(
                &format!("{tag} / must be dictionary_wrong"),
                ec.code(a),
                ZSTD_error_dictionary_wrong,
            );
        }
    }

    // raw-content dictionaries carry dictID 0: the decoder cannot detect a
    // mismatch, so both libraries must produce the SAME (possibly corrupt)
    // outcome rather than the same error.
    let a_raw = spec("raw-text-1k");
    let b_raw = spec("raw-rand-1k");
    for &lvl in &[1i32, 9] {
        let src = gen_logish(&mut rng, 8_000);
        let cap = unsafe { c_bound(src.len()) } + 64;
        let mut frame = vec![0u8; cap];
        let n = unsafe {
            c_cud(
                cc,
                frame.as_mut_ptr(),
                cap,
                src.as_ptr(),
                src.len(),
                a_raw.ptr(),
                a_raw.len(),
                lvl,
            )
        };
        assert!(!is_err(n));
        let mut o1 = vec![0u8; src.len() + 8];
        let mut o2 = vec![0u8; src.len() + 8];
        let x = unsafe {
            c_dud(
                cd,
                o1.as_mut_ptr(),
                o1.len(),
                frame.as_ptr(),
                n,
                b_raw.ptr(),
                b_raw.len(),
            )
        };
        let y = unsafe {
            r_dud(
                rd,
                o2.as_mut_ptr(),
                o2.len(),
                frame.as_ptr(),
                n,
                b_raw.ptr(),
                b_raw.len(),
            )
        };
        let tag = format!("raw-dict mismatch lvl={lvl}");
        if !ec.check(&tag, x, y) {
            assert_bytes_eq(&format!("{tag} / payload"), &o1[..x], &o2[..y]);
        }
    }

    unsafe {
        c_free(cc);
        r_free(rc);
        cd_free(cd);
        rd_free(rd);
    }
}

// ============================================================= 10. error paths

/// The dictionary error surface: NULL/0 combinations, `dct_fullDict` refusals,
/// corrupted dictionaries, out-of-range enum values across FFI, wrong-stage
/// calls and `ZSTD_DCtx_setMaxWindowSize` bounds.
#[test]
fn dictionary_error_paths_match() {
    let i = impls();
    let (c_new, r_new) = i.pair::<Fn_createCCtx>("ZSTD_createCCtx");
    let (c_free, r_free) = i.pair::<Fn_freeCCtx>("ZSTD_freeCCtx");
    let (c_rst, r_rst) = i.pair::<Fn_reset>("ZSTD_CCtx_reset");
    let (cd_new, rd_new) = i.pair::<Fn_createDCtx>("ZSTD_createDCtx");
    let (cd_free, rd_free) = i.pair::<Fn_freeDCtx>("ZSTD_freeDCtx");
    let (cd_rst, rd_rst) = i.pair::<Fn_dReset>("ZSTD_DCtx_reset");
    let (c_ld, r_ld) = i.pair::<Fn_loadDict>("ZSTD_CCtx_loadDictionary");
    let (c_lda, r_lda) = i.pair::<Fn_loadDict_advanced>("ZSTD_CCtx_loadDictionary_advanced");
    let (c_dla, r_dla) = i.pair::<Fn_dLoadDict_advanced>("ZSTD_DCtx_loadDictionary_advanced");
    let (c_dl, r_dl) = i.pair::<Fn_dLoadDict>("ZSTD_DCtx_loadDictionary");
    let (c_rp, r_rp) = i.pair::<Fn_refPrefix>("ZSTD_CCtx_refPrefix");
    let (c_rpa, r_rpa) = i.pair::<Fn_refPrefix_advanced>("ZSTD_CCtx_refPrefix_advanced");
    let (c_drpa, r_drpa) = i.pair::<Fn_dRefPrefix_advanced>("ZSTD_DCtx_refPrefix_advanced");
    let (c_ccd, r_ccd) = i.pair::<Fn_createCDict_advanced>("ZSTD_createCDict_advanced");
    let (c_cfree, r_cfree) = i.pair::<Fn_freeCDict>("ZSTD_freeCDict");
    let (c_dda, r_dda) = i.pair::<Fn_createDDict_advanced>("ZSTD_createDDict_advanced");
    let (c_dfree, r_dfree) = i.pair::<Fn_freeDDict>("ZSTD_freeDDict");
    let (c_gcp, _) = i.pair::<Fn_getCParams>("ZSTD_getCParams");
    let (c_mws, r_mws) = i.pair::<Fn_setMaxWindowSize>("ZSTD_DCtx_setMaxWindowSize");
    let (c_tp, r_tp) = i.pair::<Fn_refThreadPool>("ZSTD_CCtx_refThreadPool");
    let (c_rcd, r_rcd) = i.pair::<Fn_refCDict>("ZSTD_CCtx_refCDict");
    let (c_c2, r_c2) = i.pair::<Fn_compress2>("ZSTD_compress2");
    let (c_cs2, r_cs2) = i.pair::<Fn_compressStream2>("ZSTD_compressStream2");
    let (c_ds, r_ds) = i.pair::<Fn_decompressStream>("ZSTD_decompressStream");
    let (c_cud, r_cud) = i.pair::<Fn_compress_usingDict>("ZSTD_compress_usingDict");
    let (c_dud, r_dud) = i.pair::<Fn_decompress_usingDict>("ZSTD_decompress_usingDict");
    let (c_bound, _) = i.pair::<Fn_bound>("ZSTD_compressBound");
    let ec = ErrCmp::new();

    let (cc, rc) = unsafe { (c_new(), r_new()) };
    let (cd, rd) = unsafe { (cd_new(), rd_new()) };
    let cm = CustomMem::default_cmem();
    let mut rng = Rng::new(0xE770_0001);
    let sample = gen_logish(&mut rng, 4096);
    let dummy = spec("trained-8k");

    // ---- NULL dict with a nonzero size / nonzero dict with size 0.
    // Both are documented as "no dictionary" and must not error.
    for &(p, l, name) in &[
        (std::ptr::null::<u8>(), 0usize, "NULL,0"),
        (std::ptr::null::<u8>(), 1usize, "NULL,1"),
        (std::ptr::null::<u8>(), 4096usize, "NULL,4096"),
    ] {
        unsafe {
            c_rst(cc, ZSTD_reset_session_and_parameters);
            r_rst(rc, ZSTD_reset_session_and_parameters);
            cd_rst(cd, ZSTD_reset_session_and_parameters);
            rd_rst(rd, ZSTD_reset_session_and_parameters);
            let (a, b) = (c_ld(cc, p, l), r_ld(rc, p, l));
            ec.check(&format!("ZSTD_CCtx_loadDictionary({name})"), a, b);
            let (a, b) = (c_dl(cd, p, l), r_dl(rd, p, l));
            ec.check(&format!("ZSTD_DCtx_loadDictionary({name})"), a, b);
            let (a, b) = (c_rp(cc, p, l), r_rp(rc, p, l));
            ec.check(&format!("ZSTD_CCtx_refPrefix({name})"), a, b);
            for &dct in &ALL_DCT {
                for &dlm in &ALL_DLM {
                    let (a, b) = (c_lda(cc, p, l, dlm, dct), r_lda(rc, p, l, dlm, dct));
                    ec.check(
                        &format!("ZSTD_CCtx_loadDictionary_advanced({name},{dlm},{dct})"),
                        a,
                        b,
                    );
                    let (a, b) = (c_dla(cd, p, l, dlm, dct), r_dla(rd, p, l, dlm, dct));
                    ec.check(
                        &format!("ZSTD_DCtx_loadDictionary_advanced({name},{dlm},{dct})"),
                        a,
                        b,
                    );
                    // creation with a NULL buffer: both must agree on null-ness
                    let (x, y) = (c_ccd(p, l, dlm, dct, c_gcp(3, ZSTD_CONTENTSIZE_UNKNOWN, l), cm),
                                  r_ccd(p, l, dlm, dct, c_gcp(3, ZSTD_CONTENTSIZE_UNKNOWN, l), cm));
                    assert_eq_dbg(
                        &format!("ZSTD_createCDict_advanced({name},{dlm},{dct}) created?"),
                        x.is_null(),
                        y.is_null(),
                    );
                    c_cfree(x);
                    r_cfree(y);
                    let (x, y) = (c_dda(p, l, dlm, dct, cm), r_dda(p, l, dlm, dct, cm));
                    assert_eq_dbg(
                        &format!("ZSTD_createDDict_advanced({name},{dlm},{dct}) created?"),
                        x.is_null(),
                        y.is_null(),
                    );
                    c_dfree(x);
                    r_dfree(y);
                }
            }
        }
    }
    // nonzero pointer, zero size
    unsafe {
        c_rst(cc, ZSTD_reset_session_and_parameters);
        r_rst(rc, ZSTD_reset_session_and_parameters);
        let (a, b) = (
            c_ld(cc, dummy.ptr(), 0),
            r_ld(rc, dummy.ptr(), 0),
        );
        ec.check("ZSTD_CCtx_loadDictionary(dict, 0)", a, b);
    }

    // ---- out-of-range dct / dlm enum values crossing the FFI boundary
    let odd_enums = [-1000i32, -1, 3, 4, 7, 100, i32::MAX, i32::MIN];
    for &e in &odd_enums {
        unsafe {
            c_rst(cc, ZSTD_reset_session_and_parameters);
            r_rst(rc, ZSTD_reset_session_and_parameters);
            cd_rst(cd, ZSTD_reset_session_and_parameters);
            rd_rst(rd, ZSTD_reset_session_and_parameters);
            // dct out of range
            let (a, b) = (
                c_lda(cc, dummy.ptr(), dummy.len(), ZSTD_dlm_byCopy, e),
                r_lda(rc, dummy.ptr(), dummy.len(), ZSTD_dlm_byCopy, e),
            );
            ec.check(&format!("CCtx_loadDictionary_advanced(dct={e})"), a, b);
            let (a, b) = (
                c_dla(cd, dummy.ptr(), dummy.len(), ZSTD_dlm_byCopy, e),
                r_dla(rd, dummy.ptr(), dummy.len(), ZSTD_dlm_byCopy, e),
            );
            ec.check(&format!("DCtx_loadDictionary_advanced(dct={e})"), a, b);
            let (a, b) = (
                c_rpa(cc, dummy.ptr(), dummy.len(), e),
                r_rpa(rc, dummy.ptr(), dummy.len(), e),
            );
            ec.check(&format!("CCtx_refPrefix_advanced(dct={e})"), a, b);
            let (a, b) = (
                c_drpa(cd, dummy.ptr(), dummy.len(), e),
                r_drpa(rd, dummy.ptr(), dummy.len(), e),
            );
            ec.check(&format!("DCtx_refPrefix_advanced(dct={e})"), a, b);
            // dlm out of range (with a valid dct)
            let (a, b) = (
                c_lda(cc, dummy.ptr(), dummy.len(), e, ZSTD_dct_auto),
                r_lda(rc, dummy.ptr(), dummy.len(), e, ZSTD_dct_auto),
            );
            ec.check(&format!("CCtx_loadDictionary_advanced(dlm={e})"), a, b);
            let (x, y) = (
                c_dda(dummy.ptr(), dummy.len(), e, ZSTD_dct_auto, cm),
                r_dda(dummy.ptr(), dummy.len(), e, ZSTD_dct_auto, cm),
            );
            assert_eq_dbg(
                &format!("createDDict_advanced(dlm={e}) created?"),
                x.is_null(),
                y.is_null(),
            );
            c_dfree(x);
            r_dfree(y);
            let cp = c_gcp(3, ZSTD_CONTENTSIZE_UNKNOWN, dummy.len());
            let (x, y) = (
                c_ccd(dummy.ptr(), dummy.len(), e, ZSTD_dct_auto, cp, cm),
                r_ccd(dummy.ptr(), dummy.len(), e, ZSTD_dct_auto, cp, cm),
            );
            assert_eq_dbg(
                &format!("createCDict_advanced(dlm={e}) created?"),
                x.is_null(),
                y.is_null(),
            );
            c_cfree(x);
            r_cfree(y);
        }
        // and again reaching all the way into compression, so the *effect* of
        // the out-of-range value is compared, not only the setter's return
        unsafe {
            c_rst(cc, ZSTD_reset_session_and_parameters);
            r_rst(rc, ZSTD_reset_session_and_parameters);
            let (a, b) = (
                c_lda(cc, dummy.ptr(), dummy.len(), ZSTD_dlm_byCopy, e),
                r_lda(rc, dummy.ptr(), dummy.len(), ZSTD_dlm_byCopy, e),
            );
            if !ec.check(&format!("load(dct={e})"), a, b) {
                let cap = c_bound(sample.len()) + 64;
                let mut cb = vec![0u8; cap];
                let mut rb = vec![0u8; cap];
                let x = c_c2(cc, cb.as_mut_ptr(), cap, sample.as_ptr(), sample.len());
                let y = r_c2(rc, rb.as_mut_ptr(), cap, sample.as_ptr(), sample.len());
                if !ec.check(&format!("compress2 after load(dct={e})"), x, y) {
                    assert_bytes_eq(&format!("frame after load(dct={e})"), &cb[..x], &rb[..y]);
                }
            }
        }
    }

    // ---- dct_fullDict refusals + corrupted dictionaries
    for d in dict_corpus() {
        for &dlm in &ALL_DLM {
            unsafe {
                c_rst(cc, ZSTD_reset_session_and_parameters);
                r_rst(rc, ZSTD_reset_session_and_parameters);
                cd_rst(cd, ZSTD_reset_session_and_parameters);
                rd_rst(rd, ZSTD_reset_session_and_parameters);
            }
            // compression side: full-dict refusal surfaces at compress time
            let (a, b) = unsafe {
                (
                    c_lda(cc, d.ptr(), d.len(), dlm, ZSTD_dct_fullDict),
                    r_lda(rc, d.ptr(), d.len(), dlm, ZSTD_dct_fullDict),
                )
            };
            let tag = format!("fullDict({}, dlm={dlm})", d.name);
            if !ec.check(&format!("{tag} / load"), a, b) {
                let cap = unsafe { c_bound(sample.len()) } + 64;
                let mut cb = vec![0u8; cap];
                let mut rb = vec![0u8; cap];
                let x = unsafe {
                    c_c2(cc, cb.as_mut_ptr(), cap, sample.as_ptr(), sample.len())
                };
                let y = unsafe {
                    r_c2(rc, rb.as_mut_ptr(), cap, sample.as_ptr(), sample.len())
                };
                if !ec.check(&format!("{tag} / compress2"), x, y) {
                    assert_bytes_eq(&format!("{tag} / frame"), &cb[..x], &rb[..y]);
                }
            }
            // decompression side: refused at load time
            let (a, b) = unsafe {
                (
                    c_dla(cd, d.ptr(), d.len(), dlm, ZSTD_dct_fullDict),
                    r_dla(rd, d.ptr(), d.len(), dlm, ZSTD_dct_fullDict),
                )
            };
            ec.check(&format!("{tag} / DCtx load"), a, b);
            // and via the DDict constructor
            let (x, y) = unsafe {
                (
                    c_dda(d.ptr(), d.len(), dlm, ZSTD_dct_fullDict, cm),
                    r_dda(d.ptr(), d.len(), dlm, ZSTD_dct_fullDict, cm),
                )
            };
            assert_eq_dbg(
                &format!("{tag} / DDict created?"),
                x.is_null(),
                y.is_null(),
            );
            unsafe {
                c_dfree(x);
                r_dfree(y);
            }
        }
    }

    // corrupted trained dictionaries must be rejected as dictionary_corrupted
    // by the *decompressor* under dct_auto (magic still intact).
    for name in [
        "trained-bitflip-9",
        "trained-bitflip-17",
        "trained-bitflip-33",
        "magic-garbage",
    ] {
        let d = spec(name);
        unsafe {
            cd_rst(cd, ZSTD_reset_session_and_parameters);
            rd_rst(rd, ZSTD_reset_session_and_parameters);
        }
        let (a, b) = unsafe {
            (
                c_dla(cd, d.ptr(), d.len(), ZSTD_dlm_byCopy, ZSTD_dct_auto),
                r_dla(rd, d.ptr(), d.len(), ZSTD_dlm_byCopy, ZSTD_dct_auto),
            )
        };
        let tag = format!("corrupted dict {name} (dct_auto, DCtx)");
        ec.check(&tag, a, b);
        // ZSTD_decompress_usingDict takes the same path
        let mut o1 = vec![0u8; 64];
        let mut o2 = vec![0u8; 64];
        let junk = [0u8; 32];
        let (x, y) = unsafe {
            (
                c_dud(cd, o1.as_mut_ptr(), o1.len(), junk.as_ptr(), junk.len(), d.ptr(), d.len()),
                r_dud(rd, o2.as_mut_ptr(), o2.len(), junk.as_ptr(), junk.len(), d.ptr(), d.len()),
            )
        };
        ec.check(&format!("{tag} / decompress_usingDict"), x, y);
        // ... and the compressor
        let mut cb = vec![0u8; unsafe { c_bound(sample.len() ) } + 64];
        let cap = cb.len();
        let mut rb = vec![0u8; cap];
        let (x, y) = unsafe {
            (
                c_cud(cc, cb.as_mut_ptr(), cap, sample.as_ptr(), sample.len(), d.ptr(), d.len(), 3),
                r_cud(rc, rb.as_mut_ptr(), cap, sample.as_ptr(), sample.len(), d.ptr(), d.len(), 3),
            )
        };
        if !ec.check(&format!("{tag} / compress_usingDict"), x, y) {
            assert_bytes_eq(&format!("{tag} / frame"), &cb[..x], &rb[..y]);
        }
    }

    // ---- wrong stage: load / ref after compression has begun
    {
        let src = gen_logish(&mut rng, 200_000);
        let mut cout = vec![0u8; 4096];
        let mut rout = vec![0u8; 4096];
        unsafe {
            c_rst(cc, ZSTD_reset_session_and_parameters);
            r_rst(rc, ZSTD_reset_session_and_parameters);
        }
        let mut ci = ZSTD_inBuffer {
            src: src.as_ptr(),
            size: src.len(),
            pos: 0,
        };
        let mut ri = ci;
        let mut co = ZSTD_outBuffer {
            dst: cout.as_mut_ptr(),
            size: cout.len(),
            pos: 0,
        };
        let mut ro = ZSTD_outBuffer {
            dst: rout.as_mut_ptr(),
            size: rout.len(),
            pos: 0,
        };
        let (a, b) = unsafe {
            (
                c_cs2(cc, &mut co, &mut ci, ZSTD_e_continue),
                r_cs2(rc, &mut ro, &mut ri, ZSTD_e_continue),
            )
        };
        ec.check("compressStream2(e_continue)", a, b);
        assert_eq_dbg("stream input pos", ci.pos, ri.pos);

        unsafe {
            let (a, b) = (
                c_ld(cc, dummy.ptr(), dummy.len()),
                r_ld(rc, dummy.ptr(), dummy.len()),
            );
            assert!(ec.check("loadDictionary in wrong stage", a, b));
            assert_eq_dbg(
                "loadDictionary wrong stage code",
                ec.code(a),
                ZSTD_error_stage_wrong,
            );
            let (a, b) = (
                c_lda(cc, dummy.ptr(), dummy.len(), ZSTD_dlm_byRef, ZSTD_dct_auto),
                r_lda(rc, dummy.ptr(), dummy.len(), ZSTD_dlm_byRef, ZSTD_dct_auto),
            );
            assert!(ec.check("loadDictionary_advanced in wrong stage", a, b));
            let (a, b) = (
                c_rp(cc, dummy.ptr(), dummy.len()),
                r_rp(rc, dummy.ptr(), dummy.len()),
            );
            assert!(ec.check("refPrefix in wrong stage", a, b));
            let (a, b) = (
                c_tp(cc, std::ptr::null_mut()),
                r_tp(rc, std::ptr::null_mut()),
            );
            assert!(ec.check("refThreadPool in wrong stage", a, b));
            assert_eq_dbg(
                "refThreadPool wrong stage code",
                ec.code(a),
                ZSTD_error_stage_wrong,
            );
            let cp = c_gcp(3, ZSTD_CONTENTSIZE_UNKNOWN, dummy.len());
            let x = c_ccd(dummy.ptr(), dummy.len(), ZSTD_dlm_byRef, ZSTD_dct_auto, cp, cm);
            let y = r_ccd(dummy.ptr(), dummy.len(), ZSTD_dlm_byRef, ZSTD_dct_auto, cp, cm);
            let (a, b) = (c_rcd(cc, x), r_rcd(rc, y));
            assert!(ec.check("refCDict in wrong stage", a, b));
            c_cfree(x);
            r_cfree(y);
            c_rst(cc, ZSTD_reset_session_and_parameters);
            r_rst(rc, ZSTD_reset_session_and_parameters);
        }
    }

    // in-init stage: refThreadPool(NULL) is legal
    unsafe {
        c_rst(cc, ZSTD_reset_session_and_parameters);
        r_rst(rc, ZSTD_reset_session_and_parameters);
        let (a, b) = (
            c_tp(cc, std::ptr::null_mut()),
            r_tp(rc, std::ptr::null_mut()),
        );
        ec.check("ZSTD_CCtx_refThreadPool(NULL)", a, b);
    }

    // ---- DCtx wrong stage: partially decode a frame, then try to load a dict
    {
        let src = gen_logish(&mut rng, 300_000);
        let cap = unsafe { c_bound(src.len()) } + 64;
        let mut frame = vec![0u8; cap];
        let n = unsafe {
            c_cud(
                cc,
                frame.as_mut_ptr(),
                cap,
                src.as_ptr(),
                src.len(),
                std::ptr::null(),
                0,
                3,
            )
        };
        assert!(!is_err(n));
        let mut co = vec![0u8; 1024];
        let mut ro = vec![0u8; 1024];
        unsafe {
            cd_rst(cd, ZSTD_reset_session_and_parameters);
            rd_rst(rd, ZSTD_reset_session_and_parameters);
        }
        let mut ci = ZSTD_inBuffer {
            src: frame.as_ptr(),
            size: n,
            pos: 0,
        };
        let mut ri = ci;
        let mut cob = ZSTD_outBuffer {
            dst: co.as_mut_ptr(),
            size: co.len(),
            pos: 0,
        };
        let mut rob = ZSTD_outBuffer {
            dst: ro.as_mut_ptr(),
            size: ro.len(),
            pos: 0,
        };
        let (a, b) = unsafe { (c_ds(cd, &mut cob, &mut ci), r_ds(rd, &mut rob, &mut ri)) };
        ec.check("decompressStream(partial)", a, b);
        assert_eq_dbg("dstream out pos", cob.pos, rob.pos);
        unsafe {
            let (a, b) = (
                c_dl(cd, dummy.ptr(), dummy.len()),
                r_dl(rd, dummy.ptr(), dummy.len()),
            );
            assert!(ec.check("DCtx_loadDictionary in wrong stage", a, b));
            assert_eq_dbg(
                "DCtx_loadDictionary wrong stage code",
                ec.code(a),
                ZSTD_error_stage_wrong,
            );
            let (a, b) = (c_mws(cd, 1 << 20), r_mws(rd, 1 << 20));
            assert!(ec.check("setMaxWindowSize in wrong stage", a, b));
            cd_rst(cd, ZSTD_reset_session_and_parameters);
            rd_rst(rd, ZSTD_reset_session_and_parameters);
        }
    }

    // ---- ZSTD_DCtx_setMaxWindowSize bounds
    for &w in &[
        0usize,
        1,
        (1 << 10) - 1,
        1 << 10,
        (1 << 10) + 1,
        1 << 20,
        1 << 27,
        (1usize << 31) - 1,
        1usize << 31,
        (1usize << 31) + 1,
        usize::MAX / 2,
        usize::MAX,
    ] {
        unsafe {
            cd_rst(cd, ZSTD_reset_session_and_parameters);
            rd_rst(rd, ZSTD_reset_session_and_parameters);
            let (a, b) = (c_mws(cd, w), r_mws(rd, w));
            ec.check(&format!("ZSTD_DCtx_setMaxWindowSize({w})"), a, b);
        }
    }

    // maxWindowSize actually restricting a frame, with a dictionary in play
    {
        let src = gen_logish(&mut rng, 300_000);
        let cap = unsafe { c_bound(src.len()) } + 64;
        let mut frame = vec![0u8; cap];
        let n = unsafe {
            c_cud(
                cc,
                frame.as_mut_ptr(),
                cap,
                src.as_ptr(),
                src.len(),
                dummy.ptr(),
                dummy.len(),
                19,
            )
        };
        assert!(!is_err(n));
        for &w in &[1usize << 10, 1 << 15, 1 << 18, 1 << 20, 1 << 27] {
            let mut o1 = vec![0u8; 4096];
            let mut o2 = vec![0u8; 4096];
            unsafe {
                cd_rst(cd, ZSTD_reset_session_and_parameters);
                rd_rst(rd, ZSTD_reset_session_and_parameters);
                c_mws(cd, w);
                r_mws(rd, w);
                c_dl(cd, dummy.ptr(), dummy.len());
                r_dl(rd, dummy.ptr(), dummy.len());
            }
            let mut ci = ZSTD_inBuffer {
                src: frame.as_ptr(),
                size: n,
                pos: 0,
            };
            let mut ri = ci;
            let mut cob = ZSTD_outBuffer {
                dst: o1.as_mut_ptr(),
                size: o1.len(),
                pos: 0,
            };
            let mut rob = ZSTD_outBuffer {
                dst: o2.as_mut_ptr(),
                size: o2.len(),
                pos: 0,
            };
            let (a, b) = unsafe { (c_ds(cd, &mut cob, &mut ci), r_ds(rd, &mut rob, &mut ri)) };
            let tag = format!("decompressStream with maxWindowSize={w}");
            ec.check(&tag, a, b);
            assert_eq_dbg(&format!("{tag} / out pos"), cob.pos, rob.pos);
            assert_eq_dbg(&format!("{tag} / in pos"), ci.pos, ri.pos);
        }
        unsafe {
            cd_rst(cd, ZSTD_reset_session_and_parameters);
            rd_rst(rd, ZSTD_reset_session_and_parameters);
        }
    }

    unsafe {
        c_free(cc);
        r_free(rc);
        cd_free(cd);
        rd_free(rd);
    }
}

// ============================================================ 11. static dicts

/// `ZSTD_initStaticCDict` / `ZSTD_initStaticDDict`: exact-size, oversized and
/// undersized workspaces (must return NULL identically), plus real use of the
/// resulting static dictionaries.
#[test]
fn static_cdict_ddict_match() {
    let i = impls();
    let (c_new, r_new) = i.pair::<Fn_createCCtx>("ZSTD_createCCtx");
    let (c_free, r_free) = i.pair::<Fn_freeCCtx>("ZSTD_freeCCtx");
    let (cd_new, rd_new) = i.pair::<Fn_createDCtx>("ZSTD_createDCtx");
    let (cd_free, rd_free) = i.pair::<Fn_freeDCtx>("ZSTD_freeDCtx");
    let (c_isc, r_isc) = i.pair::<Fn_initStaticCDict>("ZSTD_initStaticCDict");
    let (c_isd, r_isd) = i.pair::<Fn_initStaticDDict>("ZSTD_initStaticDDict");
    let (c_eca, _) = i.pair::<Fn_estimateCDictSize_advanced>("ZSTD_estimateCDictSize_advanced");
    let (c_ed, _) = i.pair::<Fn_estimateDDictSize>("ZSTD_estimateDDictSize");
    let (c_gcp, _) = i.pair::<Fn_getCParams>("ZSTD_getCParams");
    let (c_sc, r_sc) = i.pair::<Fn_sizeof_CDict>("ZSTD_sizeof_CDict");
    let (c_sd, r_sd) = i.pair::<Fn_sizeof_DDict>("ZSTD_sizeof_DDict");
    let (c_fc, r_fc) = i.pair::<Fn_dictID_fromCDict>("ZSTD_getDictID_fromCDict");
    let (c_fdd, r_fdd) = i.pair::<Fn_dictID_fromDDict>("ZSTD_getDictID_fromDDict");
    let (c_cuc, r_cuc) = i.pair::<Fn_compress_usingCDict>("ZSTD_compress_usingCDict");
    let (c_dud, r_dud) = i.pair::<Fn_decompress_usingDDict>("ZSTD_decompress_usingDDict");
    let (c_cud_sym, _) = i.pair::<Fn_compress_usingDict>("ZSTD_compress_usingDict");
    let c_cud: Fn_compress_usingDict = *c_cud_sym;
    let (c_bound, _) = i.pair::<Fn_bound>("ZSTD_compressBound");
    let ec = ErrCmp::new();

    let (cc, rc) = unsafe { (c_new(), r_new()) };
    let (cd, rd) = unsafe { (cd_new(), rd_new()) };
    let mut rng = Rng::new(0x57A7_1C00);

    let specs = [
        spec("empty"),
        spec("tiny5"),
        spec("raw-text-1k"),
        spec("trained-8k"),
        spec("trained-100k"),
        spec("magic-garbage"),
    ];

    for d in specs {
        for &lvl in &[1i32, 6, 19] {
            let cp = unsafe { c_gcp(lvl, ZSTD_CONTENTSIZE_UNKNOWN, d.len()) };
            for &dct in &ALL_DCT {
                for &dlm in &ALL_DLM {
                    let need = unsafe { c_eca(d.len(), cp, dlm) };
                    // exact, generous, and several undersized workspaces
                    let sizes = [
                        need + 4096,
                        need,
                        need.saturating_sub(1),
                        need / 2,
                        need / 8,
                        64,
                        0,
                    ];
                    for &ws in &sizes {
                        // Vec<u64> guarantees the 8-byte alignment the API requires
                        let mut cbuf = vec![0u64; ws / 8 + 2];
                        let mut rbuf = vec![0u64; ws / 8 + 2];
                        let (a, b) = unsafe {
                            (
                                c_isc(
                                    cbuf.as_mut_ptr() as *mut u8,
                                    ws,
                                    d.ptr(),
                                    d.len(),
                                    dlm,
                                    dct,
                                    cp,
                                ),
                                r_isc(
                                    rbuf.as_mut_ptr() as *mut u8,
                                    ws,
                                    d.ptr(),
                                    d.len(),
                                    dlm,
                                    dct,
                                    cp,
                                ),
                            )
                        };
                        let tag = format!(
                            "initStaticCDict({}, lvl={lvl}, dct={dct}, dlm={dlm}, ws={ws}/{need})",
                            d.name
                        );
                        assert_eq_dbg(&format!("{tag} / created?"), a.is_null(), b.is_null());
                        if a.is_null() {
                            continue;
                        }
                        unsafe {
                            assert_eq_dbg(&format!("{tag} / sizeof"), c_sc(a), r_sc(b));
                            assert_eq_dbg(&format!("{tag} / dictID"), c_fc(a), r_fc(b));
                        }
                        let src = gen_logish_range(&mut rng, 100, 20_000);
                        let cap = unsafe { c_bound(src.len()) } + 64;
                        let mut co = vec![0u8; cap];
                        let mut ro = vec![0u8; cap];
                        let x = unsafe {
                            c_cuc(cc, co.as_mut_ptr(), cap, src.as_ptr(), src.len(), a)
                        };
                        let y = unsafe {
                            r_cuc(rc, ro.as_mut_ptr(), cap, src.as_ptr(), src.len(), b)
                        };
                        if !ec.check(&format!("{tag} / compress_usingCDict"), x, y) {
                            assert_bytes_eq(&format!("{tag} / frame"), &co[..x], &ro[..y]);
                        }
                        // static dictionaries live in `cbuf`/`rbuf`; nothing to free
                    }
                }
            }
        }

        // ---- static DDict
        for &dct in &ALL_DCT {
            for &dlm in &ALL_DLM {
                let need = unsafe { c_ed(d.len(), dlm) };
                for &ws in &[need + 512, need, need.saturating_sub(1), need / 2, 8, 0] {
                    let mut cbuf = vec![0u64; ws / 8 + 2];
                    let mut rbuf = vec![0u64; ws / 8 + 2];
                    // ZSTD_initStaticDDict asserts dict != NULL: always pass the
                    // real (possibly zero-length) buffer.
                    let (a, b) = unsafe {
                        (
                            c_isd(
                                cbuf.as_mut_ptr() as *mut u8,
                                ws,
                                d.ptr(),
                                d.len(),
                                dlm,
                                dct,
                            ),
                            r_isd(
                                rbuf.as_mut_ptr() as *mut u8,
                                ws,
                                d.ptr(),
                                d.len(),
                                dlm,
                                dct,
                            ),
                        )
                    };
                    let tag = format!(
                        "initStaticDDict({}, dct={dct}, dlm={dlm}, ws={ws}/{need})",
                        d.name
                    );
                    assert_eq_dbg(&format!("{tag} / created?"), a.is_null(), b.is_null());
                    if a.is_null() {
                        continue;
                    }
                    unsafe {
                        assert_eq_dbg(&format!("{tag} / sizeof"), c_sd(a), r_sd(b));
                        assert_eq_dbg(&format!("{tag} / dictID"), c_fdd(a), r_fdd(b));
                    }
                    // round trip a frame built with the same raw dictionary
                    let src = gen_logish(&mut rng, 3_000);
                    let cap = unsafe { c_bound(src.len()) } + 64;
                    let mut frame = vec![0u8; cap];
                    let n = unsafe {
                        c_cud(
                            cc,
                            frame.as_mut_ptr(),
                            cap,
                            src.as_ptr(),
                            src.len(),
                            d.ptr(),
                            d.len(),
                            3,
                        )
                    };
                    if is_err(n) {
                        continue;
                    }
                    let mut o1 = vec![0u8; src.len() + 8];
                    let mut o2 = vec![0u8; src.len() + 8];
                    let x = unsafe {
                        c_dud(cd, o1.as_mut_ptr(), o1.len(), frame.as_ptr(), n, a)
                    };
                    let y = unsafe {
                        r_dud(rd, o2.as_mut_ptr(), o2.len(), frame.as_ptr(), n, b)
                    };
                    if !ec.check(&format!("{tag} / decompress_usingDDict"), x, y) {
                        assert_bytes_eq(&format!("{tag} / payload"), &o1[..x], &o2[..y]);
                    }
                }
            }
        }
    }

    unsafe {
        c_free(cc);
        r_free(rc);
        cd_free(cd);
        rd_free(rd);
    }
}

// ================================================== 12. attach / copy / load

/// The dictionary *attachment* decision is the most configuration-sensitive
/// part of dictionary compression: `ZSTD_c_forceAttachDict` picks between
/// attaching the CDict tables, copying them or re-loading the dictionary, and
/// `ZSTD_c_enableDedicatedDictSearch` / `ZSTD_c_prefetchCDictTables` change the
/// table layout that gets attached. This test walks the full cross product for
/// several dictionary sizes and levels and requires byte-identical frames.
#[test]
fn dict_attach_matrix_matches() {
    let i = impls();
    let (c_new, r_new) = i.pair::<Fn_createCCtx>("ZSTD_createCCtx");
    let (c_free, r_free) = i.pair::<Fn_freeCCtx>("ZSTD_freeCCtx");
    let (c_rst, r_rst) = i.pair::<Fn_reset>("ZSTD_CCtx_reset");
    let (c_set, r_set) = i.pair::<Fn_setParam>("ZSTD_CCtx_setParameter");
    let (c_c2, r_c2) = i.pair::<Fn_compress2>("ZSTD_compress2");
    let (c_rcd, r_rcd) = i.pair::<Fn_refCDict>("ZSTD_CCtx_refCDict");
    let (c_ld, r_ld) = i.pair::<Fn_loadDict>("ZSTD_CCtx_loadDictionary");
    let (c_ca2, r_ca2) = i.pair::<Fn_createCDict_advanced2>("ZSTD_createCDict_advanced2");
    let (c_cfree, r_cfree) = i.pair::<Fn_freeCDict>("ZSTD_freeCDict");
    let (c_pnew, r_pnew) = i.pair::<Fn_createCCtxParams>("ZSTD_createCCtxParams");
    let (c_pfree, r_pfree) = i.pair::<Fn_freeCCtxParams>("ZSTD_freeCCtxParams");
    let (c_pinit, r_pinit) = i.pair::<Fn_cctxParamsInit>("ZSTD_CCtxParams_init");
    let (c_pset, r_pset) = i.pair::<Fn_cctxParamsSet>("ZSTD_CCtxParams_setParameter");
    let (cd_new, rd_new) = i.pair::<Fn_createDCtx>("ZSTD_createDCtx");
    let (cd_free, rd_free) = i.pair::<Fn_freeDCtx>("ZSTD_freeDCtx");
    let (c_dud, r_dud) = i.pair::<Fn_decompress_usingDict>("ZSTD_decompress_usingDict");
    let (c_bound, _) = i.pair::<Fn_bound>("ZSTD_compressBound");
    let ec = ErrCmp::new();

    let (cc, rc) = unsafe { (c_new(), r_new()) };
    let (cdx, rdx) = unsafe { (cd_new(), rd_new()) };
    let (cp, rp) = unsafe { (c_pnew(), r_pnew()) };
    let cm = CustomMem::default_cmem();
    let mut rng = Rng::new(0xA77A_C400);

    let specs = [
        spec("empty"),
        spec("tiny8"),
        spec("raw-text-1k"),
        spec("trained-8k"),
        spec("raw-text-100k"),
        spec("trained-100k"),
    ];
    // fixed input set so every row sees identical data
    let srcs: Vec<Vec<u8>> = vec![
        Vec::new(),
        gen_logish_range(&mut rng, 1, 30),
        gen_logish_range(&mut rng, 200, 900),
        gen_logish_range(&mut rng, 5_000, 20_000),
        gen_logish_range(&mut rng, 135_000, 145_000),
    ];

    for d in specs {
        for &fad in &[
            ZSTD_dictDefaultAttach,
            ZSTD_dictForceAttach,
            ZSTD_dictForceCopy,
            ZSTD_dictForceLoad,
        ] {
            for &dds in &[0i32, 1] {
                for &pf in &[ZSTD_ps_auto, ZSTD_ps_enable, ZSTD_ps_disable] {
                    for &lvl in &[1i32, 5, 9, 13, 19] {
                        // build the CDict through a params object so
                        // enableDedicatedDictSearch is honoured at build time
                        unsafe {
                            c_pinit(cp, lvl);
                            r_pinit(rp, lvl);
                            let (a, b) = (
                                c_pset(cp, ZSTD_c_enableDedicatedDictSearch, dds),
                                r_pset(rp, ZSTD_c_enableDedicatedDictSearch, dds),
                            );
                            ec.check("params enableDedicatedDictSearch", a, b);
                        }
                        let (ccd, rcd) = unsafe {
                            (
                                c_ca2(d.ptr(), d.len(), ZSTD_dlm_byCopy, ZSTD_dct_auto, cp, cm),
                                r_ca2(d.ptr(), d.len(), ZSTD_dlm_byCopy, ZSTD_dct_auto, rp, cm),
                            )
                        };
                        let base = format!(
                            "attach[{}] fad={fad} dds={dds} prefetch={pf} lvl={lvl}",
                            d.name
                        );
                        assert_eq_dbg(
                            &format!("{base} / CDict created?"),
                            ccd.is_null(),
                            rcd.is_null(),
                        );

                        // refCDict path and loadDictionary path
                        for method in 0..2 {
                            if method == 1 && ccd.is_null() {
                                continue;
                            }
                            for (k, src) in srcs.iter().enumerate() {
                                unsafe {
                                    c_rst(cc, ZSTD_reset_session_and_parameters);
                                    r_rst(rc, ZSTD_reset_session_and_parameters);
                                }
                                if !apply_cparams(
                                    &ec,
                                    (*c_set, *r_set),
                                    cc,
                                    rc,
                                    &[
                                        (ZSTD_c_compressionLevel, lvl),
                                        (ZSTD_c_forceAttachDict, fad),
                                        (ZSTD_c_enableDedicatedDictSearch, dds),
                                        (ZSTD_c_prefetchCDictTables, pf),
                                    ],
                                ) {
                                    continue;
                                }
                                let (a, b) = unsafe {
                                    if method == 0 {
                                        (
                                            c_ld(cc, d.ptr(), d.len()),
                                            r_ld(rc, d.ptr(), d.len()),
                                        )
                                    } else {
                                        (c_rcd(cc, ccd), r_rcd(rc, rcd))
                                    }
                                };
                                let tag = format!("{base} method={method} src#{k}");
                                if ec.check(&format!("{tag} / load"), a, b) {
                                    continue;
                                }
                                let cap = unsafe { c_bound(src.len()) } + 64;
                                let mut cb = vec![0u8; cap];
                                let mut rb = vec![0u8; cap];
                                let x = unsafe {
                                    c_c2(cc, cb.as_mut_ptr(), cap, src.as_ptr(), src.len())
                                };
                                let y = unsafe {
                                    r_c2(rc, rb.as_mut_ptr(), cap, src.as_ptr(), src.len())
                                };
                                if ec.check(&tag, x, y) {
                                    continue;
                                }
                                assert_bytes_eq(&tag, &cb[..x], &rb[..y]);
                                let mut o1 = vec![0u8; src.len() + 8];
                                let mut o2 = vec![0u8; src.len() + 8];
                                let n1 = unsafe {
                                    r_dud(
                                        rdx,
                                        o1.as_mut_ptr(),
                                        o1.len(),
                                        cb.as_ptr(),
                                        x,
                                        d.ptr(),
                                        d.len(),
                                    )
                                };
                                let n2 = unsafe {
                                    c_dud(
                                        cdx,
                                        o2.as_mut_ptr(),
                                        o2.len(),
                                        rb.as_ptr(),
                                        y,
                                        d.ptr(),
                                        d.len(),
                                    )
                                };
                                assert_eq_dbg(&format!("{tag} / decode rc"), n1, n2);
                                if !is_err(n1) {
                                    assert_eq_dbg(&format!("{tag} / decode len"), n1, src.len());
                                    assert_bytes_eq(&format!("{tag} / payload"), src, &o1[..n1]);
                                }
                            }
                        }
                        unsafe {
                            c_rst(cc, ZSTD_reset_session_and_parameters);
                            r_rst(rc, ZSTD_reset_session_and_parameters);
                            c_cfree(ccd);
                            r_cfree(rcd);
                        }
                    }
                }
            }
        }
    }

    unsafe {
        c_pfree(cp);
        r_pfree(rp);
        c_free(cc);
        r_free(rc);
        cd_free(cdx);
        rd_free(rdx);
    }
}

/// Applies a parameter list to both cctxs; returns `false` if any value was
/// (identically) rejected.
fn apply_cparams(
    ec: &ErrCmp,
    set: (Fn_setParam, Fn_setParam),
    cc: CCtx,
    rc: CCtx,
    params: &[(i32, i32)],
) -> bool {
    let mut ok = true;
    for &(id, v) in params {
        let (a, b) = unsafe { (set.0(cc, id, v), set.1(rc, id, v)) };
        if ec.check(&format!("setParameter({id},{v})"), a, b) {
            ok = false;
        }
    }
    ok
}

// ============================================== 13. streaming + dictionaries

/// Streams `src` through `ZSTD_compressStream2` with bounded input and output
/// windows, returning the frame plus the full trace of return values so that
/// the *stepwise* behaviour (not just the final frame) is compared.
fn stream_compress(
    f: Fn_compressStream2,
    ctx: CCtx,
    src: &[u8],
    in_chunk: usize,
    out_chunk: usize,
) -> (Vec<u8>, Vec<usize>) {
    let mut out: Vec<u8> = Vec::new();
    let mut buf = vec![0u8; out_chunk.max(1)];
    let mut trace: Vec<usize> = Vec::new();
    let mut consumed = 0usize;
    loop {
        let end = (consumed + in_chunk.max(1)).min(src.len());
        let last = end == src.len();
        let mut inb = ZSTD_inBuffer {
            src: unsafe { src.as_ptr().add(consumed) },
            size: end - consumed,
            pos: 0,
        };
        loop {
            let mut ob = ZSTD_outBuffer {
                dst: buf.as_mut_ptr(),
                size: buf.len(),
                pos: 0,
            };
            let mode = if last { ZSTD_e_end } else { ZSTD_e_continue };
            let r = unsafe { f(ctx, &mut ob, &mut inb, mode) };
            trace.push(r);
            out.extend_from_slice(&buf[..ob.pos]);
            if is_err(r) {
                return (out, trace);
            }
            if last {
                if r == 0 {
                    return (out, trace);
                }
            } else if inb.pos == inb.size {
                break;
            }
        }
        consumed = end;
    }
}

/// Mirror of `stream_compress` for `ZSTD_decompressStream`.
fn stream_decompress(
    f: Fn_decompressStream,
    ctx: DCtx,
    frame: &[u8],
    in_chunk: usize,
    out_chunk: usize,
) -> (Vec<u8>, Vec<usize>) {
    let mut out: Vec<u8> = Vec::new();
    let mut buf = vec![0u8; out_chunk.max(1)];
    let mut trace: Vec<usize> = Vec::new();
    let mut consumed = 0usize;
    loop {
        let end = (consumed + in_chunk.max(1)).min(frame.len());
        let mut inb = ZSTD_inBuffer {
            src: unsafe { frame.as_ptr().add(consumed) },
            size: end - consumed,
            pos: 0,
        };
        loop {
            let mut ob = ZSTD_outBuffer {
                dst: buf.as_mut_ptr(),
                size: buf.len(),
                pos: 0,
            };
            let r = unsafe { f(ctx, &mut ob, &mut inb) };
            trace.push(r);
            out.extend_from_slice(&buf[..ob.pos]);
            if is_err(r) {
                return (out, trace);
            }
            if r == 0 {
                return (out, trace); // frame complete
            }
            if inb.pos == inb.size {
                break;
            }
        }
        consumed = end;
        if consumed == frame.len() {
            return (out, trace);
        }
    }
}

/// The streaming API with dictionaries: `ZSTD_CCtx_loadDictionary`,
/// `ZSTD_CCtx_refCDict` and `ZSTD_CCtx_refPrefix` driven through
/// `ZSTD_compressStream2`, decoded through `ZSTD_decompressStream` with
/// `ZSTD_DCtx_loadDictionary` / `ZSTD_DCtx_refDDict` / `ZSTD_DCtx_refPrefix`.
/// Two frames are produced per configuration so the persistence of a loaded
/// dictionary and the single-use nature of a prefix are both observed.
#[test]
fn streaming_dictionary_matches() {
    let i = impls();
    let (c_new, r_new) = i.pair::<Fn_createCCtx>("ZSTD_createCCtx");
    let (c_free, r_free) = i.pair::<Fn_freeCCtx>("ZSTD_freeCCtx");
    let (c_rst, r_rst) = i.pair::<Fn_reset>("ZSTD_CCtx_reset");
    let (c_set, r_set) = i.pair::<Fn_setParam>("ZSTD_CCtx_setParameter");
    let (c_cs2_s, r_cs2_s) = i.pair::<Fn_compressStream2>("ZSTD_compressStream2");
    let (c_ld, r_ld) = i.pair::<Fn_loadDict>("ZSTD_CCtx_loadDictionary");
    let (c_rcd, r_rcd) = i.pair::<Fn_refCDict>("ZSTD_CCtx_refCDict");
    let (c_rp, r_rp) = i.pair::<Fn_refPrefix>("ZSTD_CCtx_refPrefix");
    let (c_ccd, r_ccd) = i.pair::<Fn_createCDict>("ZSTD_createCDict");
    let (c_cfree, r_cfree) = i.pair::<Fn_freeCDict>("ZSTD_freeCDict");
    let (cd_new, rd_new) = i.pair::<Fn_createDCtx>("ZSTD_createDCtx");
    let (cd_free, rd_free) = i.pair::<Fn_freeDCtx>("ZSTD_freeDCtx");
    let (cd_rst, rd_rst) = i.pair::<Fn_dReset>("ZSTD_DCtx_reset");
    let (c_ds_s, r_ds_s) = i.pair::<Fn_decompressStream>("ZSTD_decompressStream");
    let (c_dl, r_dl) = i.pair::<Fn_dLoadDict>("ZSTD_DCtx_loadDictionary");
    let (c_rdd, r_rdd) = i.pair::<Fn_refDDict>("ZSTD_DCtx_refDDict");
    let (c_drp, r_drp) = i.pair::<Fn_dRefPrefix>("ZSTD_DCtx_refPrefix");
    let (c_dda, r_dda) = i.pair::<Fn_createDDict>("ZSTD_createDDict");
    let (c_dfree, r_dfree) = i.pair::<Fn_freeDDict>("ZSTD_freeDDict");
    let ec = ErrCmp::new();

    let c_cs2: Fn_compressStream2 = *c_cs2_s;
    let r_cs2: Fn_compressStream2 = *r_cs2_s;
    let c_ds: Fn_decompressStream = *c_ds_s;
    let r_ds: Fn_decompressStream = *r_ds_s;

    let (cc, rc) = unsafe { (c_new(), r_new()) };
    let (cd, rd) = unsafe { (cd_new(), rd_new()) };
    let mut rng = Rng::new(0x57_9EAA_11);

    let specs = [
        spec("empty"),
        spec("tiny5"),
        spec("raw-text-1k"),
        spec("trained-8k"),
        spec("trained-100k"),
    ];

    for d in specs {
        // 0 = loadDictionary, 1 = refCDict, 2 = refPrefix
        for method in 0..3 {
            for &(in_chunk, out_chunk) in
                &[(1usize, 1usize), (7, 3), (1000, 128), (1 << 20, 1 << 20)]
            {
                for &lvl in &[1i32, 9] {
                    for &ck in &[0i32, 1] {
                        let src1 = gen_logish_range(&mut rng, 0, 30_000);
                        let src2 = gen_logish_range(&mut rng, 0, 6_000);

                        let (ccd, rcd) = if method == 1 {
                            unsafe {
                                (
                                    c_ccd(d.ptr(), d.len(), lvl),
                                    r_ccd(d.ptr(), d.len(), lvl),
                                )
                            }
                        } else {
                            (std::ptr::null_mut(), std::ptr::null_mut())
                        };
                        if method == 1 {
                            assert_eq_dbg(
                                "streaming CDict created?",
                                ccd.is_null(),
                                rcd.is_null(),
                            );
                        }

                        unsafe {
                            c_rst(cc, ZSTD_reset_session_and_parameters);
                            r_rst(rc, ZSTD_reset_session_and_parameters);
                        }
                        if !apply_cparams(
                            &ec,
                            (*c_set, *r_set),
                            cc,
                            rc,
                            &[(ZSTD_c_compressionLevel, lvl), (ZSTD_c_checksumFlag, ck)],
                        ) {
                            continue;
                        }
                        let (a, b) = unsafe {
                            match method {
                                0 => (c_ld(cc, d.ptr(), d.len()), r_ld(rc, d.ptr(), d.len())),
                                1 => (c_rcd(cc, ccd), r_rcd(rc, rcd)),
                                _ => (c_rp(cc, d.ptr(), d.len()), r_rp(rc, d.ptr(), d.len())),
                            }
                        };
                        let tag = format!(
                            "stream[{}] m={method} in={in_chunk} out={out_chunk} lvl={lvl} ck={ck}",
                            d.name
                        );
                        if ec.check(&format!("{tag} / set dict"), a, b) {
                            unsafe {
                                c_cfree(ccd);
                                r_cfree(rcd);
                            }
                            continue;
                        }

                        // ---- frame 1
                        let (cf1, ct1) = stream_compress(c_cs2, cc, &src1, in_chunk, out_chunk);
                        let (rf1, rt1) = stream_compress(r_cs2, rc, &src1, in_chunk, out_chunk);
                        assert_eq_dbg(&format!("{tag} / frame1 trace len"), ct1.len(), rt1.len());
                        for (k, (x, y)) in ct1.iter().zip(&rt1).enumerate() {
                            let t = format!("{tag} / frame1 step {k}");
                            ec.check(&t, *x, *y);
                        }
                        assert_bytes_eq(&format!("{tag} / frame1"), &cf1, &rf1);

                        // ---- frame 2 on the SAME contexts (no dict re-set):
                        // loaded dictionaries persist, a prefix does not.
                        let (cf2, ct2) = stream_compress(c_cs2, cc, &src2, in_chunk, out_chunk);
                        let (rf2, rt2) = stream_compress(r_cs2, rc, &src2, in_chunk, out_chunk);
                        assert_eq_dbg(&format!("{tag} / frame2 trace len"), ct2.len(), rt2.len());
                        for (k, (x, y)) in ct2.iter().zip(&rt2).enumerate() {
                            ec.check(&format!("{tag} / frame2 step {k}"), *x, *y);
                        }
                        assert_bytes_eq(&format!("{tag} / frame2"), &cf2, &rf2);

                        // ---- decode frame 1 with the matching dctx setup,
                        //      cross library (Rust decodes C's frame and back).
                        let (cdd, rdd) = if method == 1 {
                            unsafe { (c_dda(d.ptr(), d.len()), r_dda(d.ptr(), d.len())) }
                        } else {
                            (std::ptr::null_mut(), std::ptr::null_mut())
                        };
                        for (which, frame) in [("C", &cf1), ("Rust", &rf1)] {
                            for lib in 0..2 {
                                unsafe {
                                    cd_rst(cd, ZSTD_reset_session_and_parameters);
                                    rd_rst(rd, ZSTD_reset_session_and_parameters);
                                    let (p, q) = match method {
                                        0 => (
                                            c_dl(cd, d.ptr(), d.len()),
                                            r_dl(rd, d.ptr(), d.len()),
                                        ),
                                        1 => (c_rdd(cd, cdd), r_rdd(rd, rdd)),
                                        _ => (
                                            c_drp(cd, d.ptr(), d.len()),
                                            r_drp(rd, d.ptr(), d.len()),
                                        ),
                                    };
                                    ec.check(&format!("{tag} / dctx set dict"), p, q);
                                }
                                let (out, _) = if lib == 0 {
                                    stream_decompress(c_ds, cd, frame, in_chunk, out_chunk)
                                } else {
                                    stream_decompress(r_ds, rd, frame, in_chunk, out_chunk)
                                };
                                let t = format!("{tag} / decode {which} frame by lib{lib}");
                                assert_bytes_eq(&t, &src1, &out);
                            }
                        }
                        unsafe {
                            cd_rst(cd, ZSTD_reset_session_and_parameters);
                            rd_rst(rd, ZSTD_reset_session_and_parameters);
                            c_rdd(cd, std::ptr::null_mut());
                            r_rdd(rd, std::ptr::null_mut());
                            c_dfree(cdd);
                            r_dfree(rdd);
                            c_rst(cc, ZSTD_reset_session_and_parameters);
                            r_rst(rc, ZSTD_reset_session_and_parameters);
                            c_cfree(ccd);
                            r_cfree(rcd);
                        }
                    }
                }
            }
        }
    }

    unsafe {
        c_free(cc);
        r_free(rc);
        cd_free(cd);
        rd_free(rd);
    }
}

// ================================= 14. dictionary entry points of legacy APIs

type Fn_beginUsingDict = unsafe extern "C" fn(CCtx, *const u8, usize, i32) -> usize;
type Fn_beginUsingCDict = unsafe extern "C" fn(CCtx, CDict) -> usize;
type Fn_beginUsingCDictAdv = unsafe extern "C" fn(CCtx, CDict, FParams, u64) -> usize;
type Fn_continueEnd = unsafe extern "C" fn(CCtx, *mut u8, usize, *const u8, usize) -> usize;
type Fn_dBeginUsingDict = unsafe extern "C" fn(DCtx, *const u8, usize) -> usize;
type Fn_dBeginUsingDDict = unsafe extern "C" fn(DCtx, DDict) -> usize;
type Fn_getCParamsFromCDict = unsafe extern "C" fn(CDict) -> CParams;

/// The remaining dictionary entry points: the buffer-less
/// `ZSTD_compressBegin_usingDict` / `ZSTD_compressBegin_usingCDict{,_advanced}`
/// family (driven with `ZSTD_compressContinue` / `ZSTD_compressEnd`), the
/// `ZSTD_initCStream_using{Dict,CDict{,_advanced}}` and
/// `ZSTD_initDStream_using{Dict,DDict}` shims, the
/// `ZSTD_decompressBegin_using{Dict,DDict}` initialisers and
/// `ZSTD_getCParamsFromCDict`.
#[test]
fn dict_begin_and_init_stream_apis_match() {
    let i = impls();
    let (c_new, r_new) = i.pair::<Fn_createCCtx>("ZSTD_createCCtx");
    let (c_free, r_free) = i.pair::<Fn_freeCCtx>("ZSTD_freeCCtx");
    let (c_rst, r_rst) = i.pair::<Fn_reset>("ZSTD_CCtx_reset");
    let (cd_new, rd_new) = i.pair::<Fn_createDCtx>("ZSTD_createDCtx");
    let (cd_free, rd_free) = i.pair::<Fn_freeDCtx>("ZSTD_freeDCtx");
    let (cd_rst, rd_rst) = i.pair::<Fn_dReset>("ZSTD_DCtx_reset");
    let (c_bud, r_bud) = i.pair::<Fn_beginUsingDict>("ZSTD_compressBegin_usingDict");
    let (c_buc, r_buc) = i.pair::<Fn_beginUsingCDict>("ZSTD_compressBegin_usingCDict");
    let (c_buca, r_buca) =
        i.pair::<Fn_beginUsingCDictAdv>("ZSTD_compressBegin_usingCDict_advanced");
    let (c_cont, r_cont) = i.pair::<Fn_continueEnd>("ZSTD_compressContinue");
    let (c_end, r_end) = i.pair::<Fn_continueEnd>("ZSTD_compressEnd");
    let (c_dbud, r_dbud) = i.pair::<Fn_dBeginUsingDict>("ZSTD_decompressBegin_usingDict");
    let (c_dbudd, r_dbudd) = i.pair::<Fn_dBeginUsingDDict>("ZSTD_decompressBegin_usingDDict");
    let (c_icd, r_icd) = i.pair::<Fn_beginUsingDict>("ZSTD_initCStream_usingDict");
    let (c_icc, r_icc) = i.pair::<Fn_beginUsingCDict>("ZSTD_initCStream_usingCDict");
    let (c_icca, r_icca) =
        i.pair::<Fn_beginUsingCDictAdv>("ZSTD_initCStream_usingCDict_advanced");
    let (c_idd, r_idd) = i.pair::<Fn_dBeginUsingDict>("ZSTD_initDStream_usingDict");
    let (c_iddd, r_iddd) = i.pair::<Fn_dBeginUsingDDict>("ZSTD_initDStream_usingDDict");
    let (c_gcpc, r_gcpc) = i.pair::<Fn_getCParamsFromCDict>("ZSTD_getCParamsFromCDict");
    let (c_ccd, r_ccd) = i.pair::<Fn_createCDict>("ZSTD_createCDict");
    let (c_cfree, r_cfree) = i.pair::<Fn_freeCDict>("ZSTD_freeCDict");
    let (c_dda, r_dda) = i.pair::<Fn_createDDict>("ZSTD_createDDict");
    let (c_dfree, r_dfree) = i.pair::<Fn_freeDDict>("ZSTD_freeDDict");
    let (c_dud, r_dud) = i.pair::<Fn_decompress_usingDict>("ZSTD_decompress_usingDict");
    let (c_cs2_s, r_cs2_s) = i.pair::<Fn_compressStream2>("ZSTD_compressStream2");
    let (c_ds_s, r_ds_s) = i.pair::<Fn_decompressStream>("ZSTD_decompressStream");
    let (c_bound, _) = i.pair::<Fn_bound>("ZSTD_compressBound");
    let ec = ErrCmp::new();

    let c_cs2: Fn_compressStream2 = *c_cs2_s;
    let r_cs2: Fn_compressStream2 = *r_cs2_s;
    let c_ds: Fn_decompressStream = *c_ds_s;
    let r_ds: Fn_decompressStream = *r_ds_s;

    let (cc, rc) = unsafe { (c_new(), r_new()) };
    let (cd, rd) = unsafe { (cd_new(), rd_new()) };
    let mut rng = Rng::new(0xB_E91_0001);

    let specs = [
        spec("empty"),
        spec("tiny5"),
        spec("raw-text-1k"),
        spec("trained-8k"),
        spec("trained-100k"),
        spec("magic-corrupt"),
    ];

    for d in specs {
        for &lvl in &[-3i32, 1, 3, 9, 19] {
            let (ccd, rcd) = unsafe {
                (
                    c_ccd(d.ptr(), d.len(), lvl),
                    r_ccd(d.ptr(), d.len(), lvl),
                )
            };
            let (cdd, rdd) = unsafe { (c_dda(d.ptr(), d.len()), r_dda(d.ptr(), d.len())) };
            let base = format!("legacy-dict[{}] lvl={lvl}", d.name);
            assert_eq_dbg(&format!("{base} / CDict?"), ccd.is_null(), rcd.is_null());
            assert_eq_dbg(&format!("{base} / DDict?"), cdd.is_null(), rdd.is_null());

            if !ccd.is_null() {
                unsafe {
                    assert_eq_dbg(
                        &format!("{base} / ZSTD_getCParamsFromCDict"),
                        c_gcpc(ccd),
                        r_gcpc(rcd),
                    );
                }
            }

            for src in inputs(&mut rng, 3) {
                let cap = unsafe { c_bound(src.len()) } + 256;

                // ---- buffer-less: compressBegin_usingDict + continue/end
                for &split in &[0usize, 1, 3] {
                    unsafe {
                        c_rst(cc, ZSTD_reset_session_and_parameters);
                        r_rst(rc, ZSTD_reset_session_and_parameters);
                    }
                    let (a, b) = unsafe {
                        (
                            c_bud(cc, d.ptr(), d.len(), lvl),
                            r_bud(rc, d.ptr(), d.len(), lvl),
                        )
                    };
                    let tag = format!("{base} / compressBegin_usingDict split={split} n={}", src.len());
                    if ec.check(&tag, a, b) {
                        continue;
                    }
                    let mut cb = vec![0u8; cap];
                    let mut rb = vec![0u8; cap];
                    let mut cn = 0usize;
                    let mut rn = 0usize;
                    let mut failed = false;
                    let cut = if split == 0 || src.is_empty() {
                        src.len()
                    } else {
                        src.len() * split / 4
                    };
                    if cut > 0 {
                        let x = unsafe {
                            c_cont(cc, cb.as_mut_ptr(), cap, src.as_ptr(), cut)
                        };
                        let y = unsafe {
                            r_cont(rc, rb.as_mut_ptr(), cap, src.as_ptr(), cut)
                        };
                        if ec.check(&format!("{tag} / compressContinue"), x, y) {
                            failed = true;
                        } else {
                            cn = x;
                            rn = y;
                        }
                    }
                    if !failed {
                        let x = unsafe {
                            c_end(
                                cc,
                                cb.as_mut_ptr().add(cn),
                                cap - cn,
                                src.as_ptr().add(cut),
                                src.len() - cut,
                            )
                        };
                        let y = unsafe {
                            r_end(
                                rc,
                                rb.as_mut_ptr().add(rn),
                                cap - rn,
                                src.as_ptr().add(cut),
                                src.len() - cut,
                            )
                        };
                        if !ec.check(&format!("{tag} / compressEnd"), x, y) {
                            cn += x;
                            rn += y;
                            assert_bytes_eq(&format!("{tag} / frame"), &cb[..cn], &rb[..rn]);
                            let mut o1 = vec![0u8; src.len() + 8];
                            let mut o2 = vec![0u8; src.len() + 8];
                            let n1 = unsafe {
                                r_dud(
                                    rd,
                                    o1.as_mut_ptr(),
                                    o1.len(),
                                    cb.as_ptr(),
                                    cn,
                                    d.ptr(),
                                    d.len(),
                                )
                            };
                            let n2 = unsafe {
                                c_dud(
                                    cd,
                                    o2.as_mut_ptr(),
                                    o2.len(),
                                    rb.as_ptr(),
                                    rn,
                                    d.ptr(),
                                    d.len(),
                                )
                            };
                            assert_eq_dbg(&format!("{tag} / decode rc"), n1, n2);
                            if !is_err(n1) {
                                assert_eq_dbg(&format!("{tag} / decode len"), n1, src.len());
                                assert_bytes_eq(&format!("{tag} / payload"), &src, &o1[..n1]);
                            }
                        }
                    }
                }

                // ---- compressBegin_usingCDict{,_advanced}
                if !ccd.is_null() {
                    for adv in 0..2 {
                        for &(cs, ck, nd) in
                            &[(1i32, 0i32, 0i32), (0, 1, 1), (1, 1, 0), (0, 0, 1)]
                        {
                            unsafe {
                                c_rst(cc, ZSTD_reset_session_and_parameters);
                                r_rst(rc, ZSTD_reset_session_and_parameters);
                            }
                            let fp = FParams {
                                content_size_flag: cs,
                                checksum_flag: ck,
                                no_dict_id_flag: nd,
                            };
                            let (a, b) = unsafe {
                                if adv == 0 {
                                    (c_buc(cc, ccd), r_buc(rc, rcd))
                                } else {
                                    (
                                        c_buca(cc, ccd, fp, src.len() as u64),
                                        r_buca(rc, rcd, fp, src.len() as u64),
                                    )
                                }
                            };
                            let tag = format!(
                                "{base} / compressBegin_usingCDict adv={adv} fp={fp:?} n={}",
                                src.len()
                            );
                            if ec.check(&tag, a, b) {
                                continue;
                            }
                            let mut cb = vec![0u8; cap];
                            let mut rb = vec![0u8; cap];
                            let x = unsafe {
                                c_end(cc, cb.as_mut_ptr(), cap, src.as_ptr(), src.len())
                            };
                            let y = unsafe {
                                r_end(rc, rb.as_mut_ptr(), cap, src.as_ptr(), src.len())
                            };
                            if !ec.check(&format!("{tag} / compressEnd"), x, y) {
                                assert_bytes_eq(&format!("{tag} / frame"), &cb[..x], &rb[..y]);
                            }
                            if adv == 0 {
                                break; // fParams are unused for the plain variant
                            }
                        }
                    }
                    // NULL cdict is documented to fail
                    unsafe {
                        c_rst(cc, ZSTD_reset_session_and_parameters);
                        r_rst(rc, ZSTD_reset_session_and_parameters);
                        let (a, b) = (
                            c_buc(cc, std::ptr::null_mut()),
                            r_buc(rc, std::ptr::null_mut()),
                        );
                        ec.check("compressBegin_usingCDict(NULL)", a, b);
                    }
                }

                // ---- initCStream_using{Dict,CDict,CDict_advanced} + streaming
                for mode in 0..3 {
                    if mode > 0 && ccd.is_null() {
                        continue;
                    }
                    unsafe {
                        c_rst(cc, ZSTD_reset_session_and_parameters);
                        r_rst(rc, ZSTD_reset_session_and_parameters);
                    }
                    let fp = FParams {
                        content_size_flag: 1,
                        checksum_flag: 1,
                        no_dict_id_flag: 0,
                    };
                    let (a, b) = unsafe {
                        match mode {
                            0 => (
                                c_icd(cc, d.ptr(), d.len(), lvl),
                                r_icd(rc, d.ptr(), d.len(), lvl),
                            ),
                            1 => (c_icc(cc, ccd), r_icc(rc, rcd)),
                            _ => (
                                c_icca(cc, ccd, fp, src.len() as u64),
                                r_icca(rc, rcd, fp, src.len() as u64),
                            ),
                        }
                    };
                    let tag = format!("{base} / initCStream mode={mode} n={}", src.len());
                    if ec.check(&tag, a, b) {
                        continue;
                    }
                    let (cf, ct) = stream_compress(c_cs2, cc, &src, 4096, 1024);
                    let (rf, rt) = stream_compress(r_cs2, rc, &src, 4096, 1024);
                    assert_eq_dbg(&format!("{tag} / trace len"), ct.len(), rt.len());
                    for (k, (x, y)) in ct.iter().zip(&rt).enumerate() {
                        ec.check(&format!("{tag} / step {k}"), *x, *y);
                    }
                    assert_bytes_eq(&format!("{tag} / frame"), &cf, &rf);
                    if is_err(*ct.last().unwrap_or(&0)) {
                        continue;
                    }

                    // ---- initDStream_using{Dict,DDict} + streaming decode
                    for dmode in 0..2 {
                        if dmode == 1 && cdd.is_null() {
                            continue;
                        }
                        unsafe {
                            cd_rst(cd, ZSTD_reset_session_and_parameters);
                            rd_rst(rd, ZSTD_reset_session_and_parameters);
                        }
                        let (p, q) = unsafe {
                            if dmode == 0 {
                                (
                                    c_idd(cd, d.ptr(), d.len()),
                                    r_idd(rd, d.ptr(), d.len()),
                                )
                            } else {
                                (c_iddd(cd, cdd), r_iddd(rd, rdd))
                            }
                        };
                        let dt = format!("{tag} / initDStream dmode={dmode}");
                        if ec.check(&dt, p, q) {
                            continue;
                        }
                        let (o1, _) = stream_decompress(c_ds, cd, &cf, 512, 4096);
                        // re-init before the second decode
                        unsafe {
                            rd_rst(rd, ZSTD_reset_session_and_parameters);
                            if dmode == 0 {
                                r_idd(rd, d.ptr(), d.len());
                            } else {
                                r_iddd(rd, rdd);
                            }
                        }
                        let (o2, _) = stream_decompress(r_ds, rd, &cf, 512, 4096);
                        assert_bytes_eq(&format!("{dt} / decoded"), &o1, &o2);
                        assert_bytes_eq(&format!("{dt} / payload"), &src, &o1);
                    }
                }

                // ---- decompressBegin_using{Dict,DDict} return parity
                unsafe {
                    cd_rst(cd, ZSTD_reset_session_and_parameters);
                    rd_rst(rd, ZSTD_reset_session_and_parameters);
                    let (a, b) = (
                        c_dbud(cd, d.ptr(), d.len()),
                        r_dbud(rd, d.ptr(), d.len()),
                    );
                    ec.check(&format!("{base} / decompressBegin_usingDict"), a, b);
                    let (a, b) = (c_dbudd(cd, cdd), r_dbudd(rd, rdd));
                    ec.check(&format!("{base} / decompressBegin_usingDDict"), a, b);
                    let (a, b) = (
                        c_dbudd(cd, std::ptr::null_mut()),
                        r_dbudd(rd, std::ptr::null_mut()),
                    );
                    ec.check(&format!("{base} / decompressBegin_usingDDict(NULL)"), a, b);
                }
            }

            unsafe {
                c_rst(cc, ZSTD_reset_session_and_parameters);
                r_rst(rc, ZSTD_reset_session_and_parameters);
                cd_rst(cd, ZSTD_reset_session_and_parameters);
                rd_rst(rd, ZSTD_reset_session_and_parameters);
                c_cfree(ccd);
                r_cfree(rcd);
                c_dfree(cdd);
                r_dfree(rdd);
            }
        }
    }

    unsafe {
        c_free(cc);
        r_free(rc);
        cd_free(cd);
        rd_free(rd);
    }
}
