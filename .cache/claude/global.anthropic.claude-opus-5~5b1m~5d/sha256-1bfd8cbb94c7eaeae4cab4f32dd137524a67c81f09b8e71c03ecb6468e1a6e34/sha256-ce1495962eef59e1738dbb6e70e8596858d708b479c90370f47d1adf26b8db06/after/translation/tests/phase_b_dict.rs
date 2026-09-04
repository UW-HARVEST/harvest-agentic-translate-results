//! Phase B — CONFIGS.md rows 103..119: the dictionary API (`Group 8 — dictionaries`).
//!
//! Every call goes through `dlsym` into *both* shared libraries.  `ZSTD_CDict` /
//! `ZSTD_DDict` objects are always created in matching pairs and each object is
//! only ever handed back to the library that produced it.
mod common;
use common::*;
use std::ffi::{c_int, c_uint, c_void};
use std::sync::OnceLock;

// ---------------------------------------------------------------- fn types

type FnCreateCDict = unsafe extern "C" fn(*const c_void, usize, c_int) -> *mut c_void;
type FnCreateCDictAdv = unsafe extern "C" fn(
    *const c_void,
    usize,
    c_int,
    c_int,
    ZSTD_compressionParameters,
    ZSTD_customMem,
) -> *mut c_void;
type FnCreateCDictAdv2 = unsafe extern "C" fn(
    *const c_void,
    usize,
    c_int,
    c_int,
    *const c_void,
    ZSTD_customMem,
) -> *mut c_void;
type FnCreateDDict = unsafe extern "C" fn(*const c_void, usize) -> *mut c_void;
type FnCreateDDictAdv =
    unsafe extern "C" fn(*const c_void, usize, c_int, c_int, ZSTD_customMem) -> *mut c_void;

type FnCompressUsingCDict = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    usize,
    *const c_void,
    usize,
    *const c_void,
) -> usize;
type FnCompressUsingCDictAdv = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    usize,
    *const c_void,
    usize,
    *const c_void,
    ZSTD_frameParameters,
) -> usize;
type FnDecompressUsingDDict = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    usize,
    *const c_void,
    usize,
    *const c_void,
) -> usize;
type FnDecompressUsingDict = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    usize,
    *const c_void,
    usize,
    *const c_void,
    usize,
) -> usize;

type FnRefObj = unsafe extern "C" fn(*mut c_void, *const c_void) -> usize;
type FnLoadDict = unsafe extern "C" fn(*mut c_void, *const c_void, usize) -> usize;
type FnLoadDictAdv = unsafe extern "C" fn(*mut c_void, *const c_void, usize, c_int, c_int) -> usize;
type FnRefPrefixAdv = unsafe extern "C" fn(*mut c_void, *const c_void, usize, c_int) -> usize;

type FnSizeofObj = unsafe extern "C" fn(*const c_void) -> usize;
type FnDictIDObj = unsafe extern "C" fn(*const c_void) -> c_uint;
type FnDictIDBuf = unsafe extern "C" fn(*const c_void, usize) -> c_uint;
type FnCParamsFromCDict = unsafe extern "C" fn(*const c_void) -> ZSTD_compressionParameters;
type FnDDictContent = unsafe extern "C" fn(*const c_void) -> *const c_void;
type FnCopyDDictParams = unsafe extern "C" fn(*mut c_void, *const c_void);

type FnLoadCEntropy =
    unsafe extern "C" fn(*mut c_void, *mut c_void, *const c_void, usize) -> usize;
type FnLoadDEntropy = unsafe extern "C" fn(*mut c_void, *const c_void, usize) -> usize;
type FnDDSLoadDict = unsafe extern "C" fn(*mut c_void, *const u8);

type FnEstCDictAdv =
    unsafe extern "C" fn(usize, ZSTD_compressionParameters, c_int) -> usize;
type FnEstDDict = unsafe extern "C" fn(usize, c_int) -> usize;

type FnCompress2 =
    unsafe extern "C" fn(*mut c_void, *mut c_void, usize, *const c_void, usize) -> usize;
type FnGetCParams = unsafe extern "C" fn(c_int, u64, usize) -> ZSTD_compressionParameters;
type FnTrain =
    unsafe extern "C" fn(*mut c_void, usize, *const c_void, *const usize, c_uint) -> usize;
type FnNextSrc = unsafe extern "C" fn(*mut c_void) -> usize;
type FnDecompressContinue =
    unsafe extern "C" fn(*mut c_void, *mut c_void, usize, *const c_void, usize) -> usize;
type FnBegin = unsafe extern "C" fn(*mut c_void) -> usize;

// ---------------------------------------------------------------- object pairs

/// A CDict/DDict created twice: once by the C library, once by the Rust one.
struct DictObj {
    c: *mut c_void,
    r: *mut c_void,
    free_c: FnFreePtr,
    free_r: FnFreePtr,
}

impl DictObj {
    #[track_caller]
    unsafe fn make(what: &str, c: *mut c_void, r: *mut c_void, free: (FnFreePtr, FnFreePtr)) -> DictObj {
        assert_eq!(
            c.is_null(),
            r.is_null(),
            "{what}: creation NULL-ness differs (C null={}, Rust null={})",
            c.is_null(),
            r.is_null()
        );
        DictObj { c, r, free_c: free.0, free_r: free.1 }
    }
    fn ok(&self) -> bool {
        !self.c.is_null()
    }
}

impl Drop for DictObj {
    fn drop(&mut self) {
        unsafe {
            let a = (self.free_c)(self.c);
            let b = (self.free_r)(self.r);
            assert_eq!(a, b, "free(CDict/DDict) return mismatch");
        }
    }
}

unsafe fn free_cdict() -> (FnFreePtr, FnFreePtr) {
    duo::<FnFreePtr>("ZSTD_freeCDict")
}
unsafe fn free_ddict() -> (FnFreePtr, FnFreePtr) {
    duo::<FnFreePtr>("ZSTD_freeDDict")
}

unsafe fn cdict_pair(what: &str, dict: &[u8], level: c_int) -> DictObj {
    let (fc, fr) = duo::<FnCreateCDict>("ZSTD_createCDict");
    let c = fc(dict.as_ptr() as *const c_void, dict.len(), level);
    let r = fr(dict.as_ptr() as *const c_void, dict.len(), level);
    DictObj::make(what, c, r, free_cdict())
}

unsafe fn cdict_byref_pair(what: &str, dict: &[u8], level: c_int) -> DictObj {
    let (fc, fr) = duo::<FnCreateCDict>("ZSTD_createCDict_byReference");
    let c = fc(dict.as_ptr() as *const c_void, dict.len(), level);
    let r = fr(dict.as_ptr() as *const c_void, dict.len(), level);
    DictObj::make(what, c, r, free_cdict())
}

unsafe fn cdict_adv_pair(
    what: &str,
    dict: &[u8],
    dlm: c_int,
    dct: c_int,
    cp: ZSTD_compressionParameters,
) -> DictObj {
    let (fc, fr) = duo::<FnCreateCDictAdv>("ZSTD_createCDict_advanced");
    let cm = ZSTD_customMem::default();
    let c = fc(dict.as_ptr() as *const c_void, dict.len(), dlm, dct, cp, cm);
    let r = fr(dict.as_ptr() as *const c_void, dict.len(), dlm, dct, cp, cm);
    DictObj::make(what, c, r, free_cdict())
}

unsafe fn cdict_adv2_pair(
    what: &str,
    dict: &[u8],
    dlm: c_int,
    dct: c_int,
    params: &CtxPair,
) -> DictObj {
    let (fc, fr) = duo::<FnCreateCDictAdv2>("ZSTD_createCDict_advanced2");
    let cm = ZSTD_customMem::default();
    let c = fc(dict.as_ptr() as *const c_void, dict.len(), dlm, dct, params.c, cm);
    let r = fr(dict.as_ptr() as *const c_void, dict.len(), dlm, dct, params.r, cm);
    DictObj::make(what, c, r, free_cdict())
}

unsafe fn ddict_pair(what: &str, dict: &[u8]) -> DictObj {
    let (fc, fr) = duo::<FnCreateDDict>("ZSTD_createDDict");
    let c = fc(dict.as_ptr() as *const c_void, dict.len());
    let r = fr(dict.as_ptr() as *const c_void, dict.len());
    DictObj::make(what, c, r, free_ddict())
}

unsafe fn ddict_byref_pair(what: &str, dict: &[u8]) -> DictObj {
    let (fc, fr) = duo::<FnCreateDDict>("ZSTD_createDDict_byReference");
    let c = fc(dict.as_ptr() as *const c_void, dict.len());
    let r = fr(dict.as_ptr() as *const c_void, dict.len());
    DictObj::make(what, c, r, free_ddict())
}

unsafe fn ddict_adv_pair(what: &str, dict: &[u8], dlm: c_int, dct: c_int) -> DictObj {
    let (fc, fr) = duo::<FnCreateDDictAdv>("ZSTD_createDDict_advanced");
    let cm = ZSTD_customMem::default();
    let c = fc(dict.as_ptr() as *const c_void, dict.len(), dlm, dct, cm);
    let r = fr(dict.as_ptr() as *const c_void, dict.len(), dlm, dct, cm);
    DictObj::make(what, c, r, free_ddict())
}

/// `ZSTD_sizeof_CDict` / `ZSTD_getDictID_fromCDict` / `ZSTD_getCParamsFromCDict`
/// on a pair of CDicts.
#[track_caller]
unsafe fn check_cdict_getters(what: &str, d: &DictObj) {
    if !d.ok() {
        return;
    }
    let (sc, sr) = duo::<FnSizeofObj>("ZSTD_sizeof_CDict");
    eqv(&format!("{what} sizeof_CDict"), sc(d.c), sr(d.r));
    let (ic, ir) = duo::<FnDictIDObj>("ZSTD_getDictID_fromCDict");
    eqv(&format!("{what} getDictID_fromCDict"), ic(d.c), ir(d.r));
    let (pc, pr) = duo::<FnCParamsFromCDict>("ZSTD_getCParamsFromCDict");
    eqv(&format!("{what} getCParamsFromCDict"), pc(d.c), pr(d.r));
}

#[track_caller]
unsafe fn check_ddict_getters(what: &str, d: &DictObj) {
    if !d.ok() {
        return;
    }
    let (sc, sr) = duo::<FnSizeofObj>("ZSTD_sizeof_DDict");
    eqv(&format!("{what} sizeof_DDict"), sc(d.c), sr(d.r));
    let (ic, ir) = duo::<FnDictIDObj>("ZSTD_getDictID_fromDDict");
    eqv(&format!("{what} getDictID_fromDDict"), ic(d.c), ir(d.r));
    let (zc, zr) = duo::<FnSizeofObj>("ZSTD_DDict_dictSize");
    let nc = zc(d.c);
    let nr = zr(d.r);
    eqv(&format!("{what} DDict_dictSize"), nc, nr);
    let (cc, cr) = duo::<FnDDictContent>("ZSTD_DDict_dictContent");
    let pc = cc(d.c);
    let pr = cr(d.r);
    assert_eq!(
        pc.is_null(),
        pr.is_null(),
        "{what} DDict_dictContent NULL-ness differs"
    );
    if !pc.is_null() && nc > 0 {
        // compare the *content bytes*, never the pointers
        let a = std::slice::from_raw_parts(pc as *const u8, nc);
        let b = std::slice::from_raw_parts(pr as *const u8, nr);
        eqbuf(&format!("{what} DDict_dictContent bytes"), a, b);
    }
}

// ---------------------------------------------------------------- fixtures

fn corpus(seed: u64, size: usize) -> Vec<u8> {
    gen_class(4, size, seed)
}

/// Train a real dictionary through the **C** `.so` (test fixture only).
unsafe fn train(cap: usize, seed: u64, nb: usize, each: usize) -> Vec<u8> {
    let base = corpus(seed ^ 0xABCD, 4096);
    let mut rng = Rng::new(seed);
    let mut samples: Vec<u8> = Vec::new();
    let mut sizes: Vec<usize> = Vec::new();
    for _ in 0..nb {
        let off = rng.below(base.len() - 512);
        let take = 256 + rng.below(each.max(257) - 256);
        let take = take.min(base.len() - off);
        samples.extend_from_slice(&base[off..off + take]);
        let extra = rng.below(24);
        let tail = rng.bytes(extra);
        samples.extend_from_slice(&tail);
        sizes.push(take + extra);
    }
    let (tc, _) = duo::<FnTrain>("ZDICT_trainFromBuffer");
    let mut dict = vec![0u8; cap];
    let n = tc(
        dict.as_mut_ptr() as *mut c_void,
        cap,
        samples.as_ptr() as *const c_void,
        sizes.as_ptr(),
        nb as c_uint,
    );
    assert!(!is_err(n), "fixture: ZDICT_trainFromBuffer failed ({n:#x})");
    assert!(n >= 8, "fixture: trained dictionary is absurdly small: {n}");
    dict.truncate(n);
    // sanity: it must look like a zstd dictionary
    assert_eq!(
        u32::from_le_bytes([dict[0], dict[1], dict[2], dict[3]]),
        ZSTD_MAGIC_DICTIONARY,
        "fixture: trained dict has no dictionary magic"
    );
    dict
}

static TRAINED_BIG: OnceLock<Vec<u8>> = OnceLock::new();
static TRAINED_SMALL: OnceLock<Vec<u8>> = OnceLock::new();

fn trained_big() -> &'static [u8] {
    TRAINED_BIG.get_or_init(|| unsafe { train(16 * 1024, 0xD1C7, 96, 1024) })
}
fn trained_small() -> &'static [u8] {
    TRAINED_SMALL.get_or_init(|| unsafe { train(2 * 1024, 0x5EED, 64, 700) })
}

fn with_dict_id(dict: &[u8], id: u32) -> Vec<u8> {
    let mut d = dict.to_vec();
    d[4..8].copy_from_slice(&id.to_le_bytes());
    d
}

fn bad_magic(dict: &[u8]) -> Vec<u8> {
    let mut d = dict.to_vec();
    d[0..4].copy_from_slice(&0xDEADBEEFu32.to_le_bytes());
    d
}

/// The dictionary-shape axis of CONFIGS.md: none / tiny / raw / trained /
/// wrong-magic / truncated-trained.
fn dict_shapes() -> Vec<(String, Vec<u8>)> {
    let mut v: Vec<(String, Vec<u8>)> = Vec::new();
    v.push(("none".into(), Vec::new()));
    v.push(("tiny3".into(), gen_class(3, 3, 1)));
    v.push(("tiny7".into(), gen_class(3, 7, 2)));
    v.push(("raw16".into(), gen_class(3, 16, 3)));
    v.push(("raw1k".into(), gen_class(3, 1024, 4)));
    v.push(("raw10k".into(), gen_class(3, 10_000, 5)));
    v.push(("text2k".into(), gen_class(4, 2048, 6)));
    v.push(("zeros1k".into(), gen_class(0, 1024, 7)));
    v.push(("trainedS".into(), trained_small().to_vec()));
    v.push(("trainedB".into(), trained_big().to_vec()));
    v.push(("badmagic".into(), bad_magic(trained_big())));
    let t = trained_big();
    v.push(("trainedTrunc".into(), t[..t.len() / 2].to_vec()));
    v
}

/// The subset used where the row only needs "a raw dict and a real dict".
fn dict_shapes_small() -> Vec<(String, Vec<u8>)> {
    vec![
        ("none".into(), Vec::new()),
        ("tiny7".into(), gen_class(3, 7, 2)),
        ("raw1k".into(), gen_class(3, 1024, 4)),
        ("text2k".into(), gen_class(4, 2048, 6)),
        ("trainedB".into(), trained_big().to_vec()),
        ("badmagic".into(), bad_magic(trained_big())),
    ]
}

// ---------------------------------------------------------------- helpers

unsafe fn compress_bound(n: usize) -> usize {
    duo::<FnSizeT1>("ZSTD_compressBound").0(n)
}

unsafe fn get_cparams(level: c_int, src: u64, dict: usize) -> ZSTD_compressionParameters {
    duo::<FnGetCParams>("ZSTD_getCParams").0(level, src, dict)
}

/// Symmetric round-trip check: the C frame is decoded by the C library with the
/// same dictionary settings, likewise for Rust; returns and payloads must match
/// each other, and — when decoding succeeds — the original input.
#[track_caller]
unsafe fn rt_check(what: &str, cf: &[u8], rf: &[u8], dict: &[u8], dct: c_int, orig: &[u8]) {
    let dctx = CtxPair::dctx();
    let (lc, lr) = duo::<FnLoadDictAdv>("ZSTD_DCtx_loadDictionary_advanced");
    let a = lc(dctx.c, dict.as_ptr() as *const c_void, dict.len(), ZSTD_dlm_byCopy, dct);
    let b = lr(dctx.r, dict.as_ptr() as *const c_void, dict.len(), ZSTD_dlm_byCopy, dct);
    eqv(&format!("{what} DCtx_loadDictionary_advanced"), a, b);
    let (dc, dr) = duo::<FnDecompressDCtx>("ZSTD_decompressDCtx");
    // pass 1: dstCapacity == exactly the decompressed size.  No slack means the
    // whole destination buffer is defined output and can be compared in full.
    let cap = orig.len();
    let mut oc = vec![0xC3u8; cap];
    let mut or_ = vec![0xC3u8; cap];
    let x = dc(dctx.c, oc.as_mut_ptr() as *mut c_void, cap, cf.as_ptr() as *const c_void, cf.len());
    let y = dr(dctx.r, or_.as_mut_ptr() as *mut c_void, cap, rf.as_ptr() as *const c_void, rf.len());
    eqv(&format!("{what} roundtrip(exact) ret"), x, y);
    eqbuf(&format!("{what} roundtrip(exact) dst"), &oc, &or_);
    if is_err(x) {
        return;
    }
    assert_eq!(x, orig.len(), "{what}: roundtrip produced {x} bytes, want {}", orig.len());
    eqbuf(&format!("{what} roundtrip payload"), orig, &oc);
    // pass 2: oversized destination (exercises the wildcopy path).  Everything
    // past the produced size is scratch the API does not define, so only the
    // produced bytes are compared.
    let cap2 = orig.len() + 64;
    let mut pc = vec![0x1Du8; cap2];
    let mut pr = vec![0x1Du8; cap2];
    let x2 = dc(dctx.c, pc.as_mut_ptr() as *mut c_void, cap2, cf.as_ptr() as *const c_void, cf.len());
    let y2 = dr(dctx.r, pr.as_mut_ptr() as *mut c_void, cap2, rf.as_ptr() as *const c_void, rf.len());
    eqv(&format!("{what} roundtrip(padded) ret"), x2, y2);
    cmp_decode_bufs(&format!("{what} roundtrip(padded)"), x2, &pc, &pr);
}

/// Compare two decompression destination buffers: the whole buffer on the error
/// path, only the *produced* bytes on success — `ZSTD_decompress*` is allowed to
/// clobber the unused tail of an oversized destination (`ZSTD_wildcopy`), and
/// those bytes are not part of the API contract.
#[track_caller]
fn cmp_decode_bufs(what: &str, ret: usize, c: &[u8], r: &[u8]) {
    if is_err(ret) {
        eqbuf(&format!("{what} dst (error path)"), c, r);
        return;
    }
    assert!(ret <= c.len(), "{what}: return {ret} exceeds buffer {}", c.len());
    eqbuf(&format!("{what} dst"), &c[..ret], &r[..ret]);
}

/// `ZSTD_compress2` on both cctxs of a pair; compares return + destination.
#[track_caller]
unsafe fn compress2_pair(what: &str, cctx: &CtxPair, src: &[u8]) -> (usize, Vec<u8>, Vec<u8>) {
    let cap = compress_bound(src.len()) + 64;
    let mut oc = vec![0x5Au8; cap];
    let mut or_ = vec![0x5Au8; cap];
    let (fc, fr) = duo::<FnCompress2>("ZSTD_compress2");
    let a = fc(cctx.c, oc.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, src.len());
    let b = fr(cctx.r, or_.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, src.len());
    eqv(&format!("{what} compress2 ret"), a, b);
    eqbuf(&format!("{what} compress2 dst"), &oc, &or_);
    if !is_err(a) {
        oc.truncate(a);
        or_.truncate(a);
    } else {
        oc.clear();
        or_.clear();
    }
    (a, oc, or_)
}

/// Drive `ZSTD_compressStream2` to completion with `ZSTD_e_end`.
unsafe fn stream_end(f: FnStream2, ctx: *mut c_void, src: &[u8], outcap: usize) -> (usize, Vec<u8>) {
    let mut dst = vec![0x33u8; outcap];
    let mut inb = ZSTD_inBuffer { src: src.as_ptr() as *const c_void, size: src.len(), pos: 0 };
    let mut outb = ZSTD_outBuffer { dst: dst.as_mut_ptr() as *mut c_void, size: outcap, pos: 0 };
    let mut last;
    let mut guard = 0;
    loop {
        last = f(ctx, &mut outb, &mut inb, ZSTD_e_end);
        if is_err(last) || last == 0 {
            break;
        }
        guard += 1;
        if guard > 10_000 || outb.pos == outb.size {
            break;
        }
    }
    let n = outb.pos;
    dst.truncate(if is_err(last) { 0 } else { n });
    (last, dst)
}

/// Bufferless decode loop: `ZSTD_nextSrcSizeToDecompress` + `ZSTD_decompressContinue`.
unsafe fn bufferless_decode(
    next: FnNextSrc,
    cont: FnDecompressContinue,
    dctx: *mut c_void,
    frame: &[u8],
    outcap: usize,
) -> (usize, Vec<u8>) {
    let mut out = vec![0x77u8; outcap];
    let mut ip = 0usize;
    let mut op = 0usize;
    loop {
        let n = next(dctx);
        if is_err(n) {
            out.truncate(op);
            return (n, out);
        }
        if n == 0 {
            break;
        }
        if ip + n > frame.len() {
            out.truncate(op);
            return (usize::MAX - 200, out); // "ran out of input" sentinel
        }
        let r = cont(
            dctx,
            out.as_mut_ptr().add(op) as *mut c_void,
            outcap - op,
            frame.as_ptr().add(ip) as *const c_void,
            n,
        );
        if is_err(r) {
            out.truncate(op);
            return (r, out);
        }
        ip += n;
        op += r;
    }
    out.truncate(op);
    (op, out)
}

/// Compress with the **C** library only: ground-truth frames for the decode rows.
unsafe fn c_frame_with_dict(src: &[u8], level: c_int, dict: &[u8], params: &[(c_int, c_int)]) -> Vec<u8> {
    let (create, _) = duo::<FnPtr0>("ZSTD_createCCtx");
    let (free, _) = duo::<FnFreePtr>("ZSTD_freeCCtx");
    let (setp, _) = duo::<FnSetParam>("ZSTD_CCtx_setParameter");
    let (ld, _) = duo::<FnLoadDict>("ZSTD_CCtx_loadDictionary");
    let (c2, _) = duo::<FnCompress2>("ZSTD_compress2");
    let cctx = create();
    assert!(!cctx.is_null());
    assert!(!is_err(setp(cctx, ZSTD_c_compressionLevel, level)));
    for &(p, v) in params {
        assert!(!is_err(setp(cctx, p, v)), "c_frame_with_dict: setParameter({p},{v})");
    }
    if !dict.is_empty() {
        let n = ld(cctx, dict.as_ptr() as *const c_void, dict.len());
        assert!(!is_err(n), "c_frame_with_dict: loadDictionary failed");
    }
    let cap = compress_bound(src.len()) + 64;
    let mut dst = vec![0u8; cap];
    let n = c2(cctx, dst.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, src.len());
    free(cctx);
    assert!(!is_err(n), "c_frame_with_dict: compress2 failed");
    dst.truncate(n);
    dst
}

const LEVELS: [c_int; 9] = [-5, -1, 0, 1, 3, 6, 12, 19, 22];

/// `ZSTD_createCDict(dict, 0, level)` has no source-size hint, so it reserves the
/// *maximum* window for the level: 80 MB at level 19 and 640 MB at level 22 (see
/// `ZSTD_estimateCDictSize`).  Any dictSize > 0 clamps the window and costs well
/// under 1 MB.  `cargo test` runs the 20 tests of this file in parallel inside one
/// process that has a 6 GB `RLIMIT_DATA`, so the empty-dictionary + high-level
/// configurations are serialized instead of being dropped.
static BIG_CDICT: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn maybe_big(dict_len: usize, level: c_int) -> Option<std::sync::MutexGuard<'static, ()>> {
    if dict_len == 0 && level >= 10 {
        Some(BIG_CDICT.lock().unwrap_or_else(|e| e.into_inner()))
    } else {
        None
    }
}

/// The CONFIGS.md input-size ladder, minus the very large entries at the
/// expensive levels (btultra2 on 1MB would blow the time budget).
fn pick_size(rng: &mut Rng, level: c_int) -> usize {
    const SMALL: [usize; 9] = [0, 1, 7, 128, 1024, 8 * 1024, 64 * 1024, 128 * 1024 - 1, 128 * 1024];
    const BIG: [usize; 4] = [128 * 1024 + 1, 200_000, 256 * 1024, 1024 * 1024];
    if level >= 17 || rng.below(3) == 0 {
        SMALL[rng.below(SMALL.len())]
    } else {
        let all = rng.below(SMALL.len() + BIG.len());
        if all < SMALL.len() {
            SMALL[all]
        } else {
            BIG[all - SMALL.len()]
        }
    }
}

// ------------------------------------------------------------------ row 103

#[test]
fn row103_createCDict_compress_usingCDict() {
    unsafe {
        let (cuc, cur) = duo::<FnCompressUsingCDict>("ZSTD_compress_usingCDict");
        let (duc, dur) = duo::<FnDecompressUsingDDict>("ZSTD_decompress_usingDDict");
        let mut rng = Rng::new(103);
        for (dn, dict) in dict_shapes() {
            for &lvl in &LEVELS {
                let _big = maybe_big(dict.len(), lvl);
                let what = format!("row103 dict={dn} lvl={lvl}");
                let cd = cdict_pair(&what, &dict, lvl);
                check_cdict_getters(&what, &cd);
                if !cd.ok() {
                    continue;
                }
                let dd = ddict_pair(&what, &dict);
                check_ddict_getters(&what, &dd);
                for i in 0..4 {
                    let cls = rng.below(N_CLASSES);
                    let sz = pick_size(&mut rng, lvl);
                    let src = gen_class(cls, sz, rng.next_u64());
                    let cctx = CtxPair::cctx();
                    let cap = compress_bound(sz) + 64;
                    let mut oc = vec![0x5Au8; cap];
                    let mut or_ = vec![0x5Au8; cap];
                    let a = cuc(
                        cctx.c,
                        oc.as_mut_ptr() as *mut c_void,
                        cap,
                        src.as_ptr() as *const c_void,
                        sz,
                        cd.c,
                    );
                    let b = cur(
                        cctx.r,
                        or_.as_mut_ptr() as *mut c_void,
                        cap,
                        src.as_ptr() as *const c_void,
                        sz,
                        cd.r,
                    );
                    let tag = format!("{what} i={i} cls={cls} sz={sz}");
                    eqv(&format!("{tag} compress_usingCDict ret"), a, b);
                    eqbuf(&format!("{tag} compress_usingCDict dst"), &oc, &or_);
                    if is_err(a) || !dd.ok() {
                        continue;
                    }
                    // round-trip through ZSTD_decompress_usingDDict
                    let dctx = CtxPair::dctx();
                    let ocap = sz + 64;
                    let mut pc = vec![0xA5u8; ocap];
                    let mut pr = vec![0xA5u8; ocap];
                    let x = duc(
                        dctx.c,
                        pc.as_mut_ptr() as *mut c_void,
                        ocap,
                        oc.as_ptr() as *const c_void,
                        a,
                        dd.c,
                    );
                    let y = dur(
                        dctx.r,
                        pr.as_mut_ptr() as *mut c_void,
                        ocap,
                        or_.as_ptr() as *const c_void,
                        b,
                        dd.r,
                    );
                    eqv(&format!("{tag} decompress_usingDDict ret"), x, y);
                    cmp_decode_bufs(&format!("{tag} decompress_usingDDict"), x, &pc, &pr);
                    assert!(!is_err(x), "{tag}: usingDDict round-trip failed");
                    assert_eq!(x, sz, "{tag}: wrong decompressed size");
                    eqbuf(&format!("{tag} payload"), &src, &pc[..x]);
                }
            }
        }
    }
}

// ------------------------------------------------------------------ row 104

#[test]
fn row104_createCDict_byReference() {
    unsafe {
        let (cuc, cur) = duo::<FnCompressUsingCDict>("ZSTD_compress_usingCDict");
        let mut rng = Rng::new(104);
        for (dn, dict) in dict_shapes() {
            for &lvl in &LEVELS {
                let _big = maybe_big(dict.len(), lvl);
                let what = format!("row104 dict={dn} lvl={lvl}");
                let byval = cdict_pair(&format!("{what} byCopy"), &dict, lvl);
                let byref = cdict_byref_pair(&format!("{what} byRef"), &dict, lvl);
                check_cdict_getters(&format!("{what} byRef"), &byref);
                if !byref.ok() {
                    continue;
                }
                // byRef and byCopy must agree on dictID and cParams
                let (ic, _) = duo::<FnDictIDObj>("ZSTD_getDictID_fromCDict");
                if byval.ok() {
                    eqv(&format!("{what} dictID byCopy vs byRef"), ic(byval.c), ic(byref.c));
                }
                for i in 0..3 {
                    let cls = rng.below(N_CLASSES);
                    let sz = pick_size(&mut rng, lvl);
                    let src = gen_class(cls, sz, rng.next_u64());
                    let cctx = CtxPair::cctx();
                    let cap = compress_bound(sz) + 64;
                    let mut oc = vec![0x5Au8; cap];
                    let mut or_ = vec![0x5Au8; cap];
                    let a = cuc(cctx.c, oc.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, sz, byref.c);
                    let b = cur(cctx.r, or_.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, sz, byref.r);
                    let tag = format!("{what} i={i} cls={cls} sz={sz}");
                    eqv(&format!("{tag} compress_usingCDict(byRef) ret"), a, b);
                    eqbuf(&format!("{tag} compress_usingCDict(byRef) dst"), &oc, &or_);
                    if is_err(a) {
                        continue;
                    }
                    // byRef must produce exactly the same frame as byCopy
                    if byval.ok() {
                        let cctx2 = CtxPair::cctx();
                        let mut o2 = vec![0x5Au8; cap];
                        let a2 = cuc(cctx2.c, o2.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, sz, byval.c);
                        assert!(!is_err(a2), "{tag}: byCopy compression failed");
                        eqbuf(&format!("{tag} byCopy vs byRef frame"), &o2[..a2], &oc[..a]);
                    }
                    rt_check(&tag, &oc[..a], &or_[..b], &dict, ZSTD_dct_auto, &src);
                }
            }
        }
    }
}

// ------------------------------------------------------------------ row 105

#[test]
fn row105_createCDict_advanced_grid() {
    unsafe {
        let (cuc, cur) = duo::<FnCompressUsingCDict>("ZSTD_compress_usingCDict");
        let mut rng = Rng::new(105);
        let cparam_grid: Vec<ZSTD_compressionParameters> = vec![
            get_cparams(1, 0, 0),
            get_cparams(3, 0, 1024),
            get_cparams(9, 100_000, 1024),
            get_cparams(19, 200_000, 1024),
            ZSTD_compressionParameters {
                windowLog: 17,
                chainLog: 16,
                hashLog: 17,
                searchLog: 4,
                minMatch: 5,
                targetLength: 64,
                strategy: 5,
            },
            ZSTD_compressionParameters {
                windowLog: 10,
                chainLog: 10,
                hashLog: 12,
                searchLog: 1,
                minMatch: 7,
                targetLength: 0,
                strategy: 1,
            },
            ZSTD_compressionParameters {
                windowLog: 20,
                chainLog: 20,
                hashLog: 22,
                searchLog: 6,
                minMatch: 3,
                targetLength: 999,
                strategy: 9,
            },
        ];
        for (dn, dict) in dict_shapes_small() {
            for dlm in [ZSTD_dlm_byCopy, ZSTD_dlm_byRef] {
                for dct in [ZSTD_dct_auto, ZSTD_dct_rawContent, ZSTD_dct_fullDict] {
                    for (ci, cp) in cparam_grid.iter().enumerate() {
                        let _big = maybe_big(dict.len(), 22);
                        let what = format!("row105 dict={dn} dlm={dlm} dct={dct} cp#{ci}");
                        let cd = cdict_adv_pair(&what, &dict, dlm, dct, *cp);
                        check_cdict_getters(&what, &cd);
                        if !cd.ok() {
                            continue;
                        }
                        for i in 0..2 {
                            let cls = rng.below(N_CLASSES);
                            let sz = [0usize, 5, 700, 20_000, 90_000, 200_000][rng.below(6)];
                            let src = gen_class(cls, sz, rng.next_u64());
                            let cctx = CtxPair::cctx();
                            let cap = compress_bound(sz) + 64;
                            let mut oc = vec![0x5Au8; cap];
                            let mut or_ = vec![0x5Au8; cap];
                            let a = cuc(cctx.c, oc.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, sz, cd.c);
                            let b = cur(cctx.r, or_.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, sz, cd.r);
                            let tag = format!("{what} i={i} cls={cls} sz={sz}");
                            eqv(&format!("{tag} ret"), a, b);
                            eqbuf(&format!("{tag} dst"), &oc, &or_);
                            if is_err(a) {
                                continue;
                            }
                            rt_check(&tag, &oc[..a], &or_[..b], &dict, dct, &src);
                        }
                    }
                }
            }
        }
    }
}

// ------------------------------------------------------------------ row 106

#[test]
fn row106_createCDict_advanced2_grid() {
    unsafe {
        let (cuc, cur) = duo::<FnCompressUsingCDict>("ZSTD_compress_usingCDict");
        let (psc, psr) = duo::<FnSetParam>("ZSTD_CCtxParams_setParameter");
        let mut rng = Rng::new(106);
        // (label, param script) — exercises the compressionLevel fallback plus
        // explicit cParams overrides and the dedicated-dict-search switch.
        let scripts: Vec<(&str, Vec<(c_int, c_int)>)> = vec![
            ("empty", vec![]),
            ("lvl1", vec![(ZSTD_c_compressionLevel, 1)]),
            ("lvl0", vec![(ZSTD_c_compressionLevel, 0)]),
            ("lvl-3", vec![(ZSTD_c_compressionLevel, -3)]),
            ("lvl19", vec![(ZSTD_c_compressionLevel, 19)]),
            (
                "lvl6+strategy9",
                vec![(ZSTD_c_compressionLevel, 6), (ZSTD_c_strategy, 9)],
            ),
            (
                "dds+greedy",
                vec![
                    (ZSTD_c_compressionLevel, 6),
                    (ZSTD_c_enableDedicatedDictSearch, 1),
                    (ZSTD_c_strategy, 3),
                ],
            ),
            (
                "dds+lazy2",
                vec![
                    (ZSTD_c_compressionLevel, 12),
                    (ZSTD_c_enableDedicatedDictSearch, 1),
                    (ZSTD_c_strategy, 5),
                ],
            ),
            (
                "dds+btopt(unsupported)",
                vec![
                    (ZSTD_c_compressionLevel, 17),
                    (ZSTD_c_enableDedicatedDictSearch, 1),
                    (ZSTD_c_strategy, 7),
                ],
            ),
            (
                "rowmf+windowLog",
                vec![
                    (ZSTD_c_compressionLevel, 5),
                    (ZSTD_c_useRowMatchFinder, ZSTD_ps_enable),
                    (ZSTD_c_windowLog, 18),
                    (ZSTD_c_hashLog, 19),
                ],
            ),
        ];
        for (dn, dict) in dict_shapes_small() {
            for dlm in [ZSTD_dlm_byCopy, ZSTD_dlm_byRef] {
                for dct in [ZSTD_dct_auto, ZSTD_dct_rawContent, ZSTD_dct_fullDict] {
                    for (sn, script) in &scripts {
                        let _big = maybe_big(dict.len(), 22);
                        let params = CtxPair::cctx_params();
                        for &(p, v) in script {
                            let a = psc(params.c, p, v);
                            let b = psr(params.r, p, v);
                            eqv(&format!("row106 CCtxParams_setParameter({p},{v})"), a, b);
                            assert!(!is_err(a), "row106 setParameter({p},{v}) rejected");
                        }
                        let what = format!("row106 dict={dn} dlm={dlm} dct={dct} {sn}");
                        let cd = cdict_adv2_pair(&what, &dict, dlm, dct, &params);
                        check_cdict_getters(&what, &cd);
                        if !cd.ok() {
                            continue;
                        }
                        for i in 0..2 {
                            let cls = rng.below(N_CLASSES);
                            let sz = [0usize, 9, 1_100, 33_000, 130_000, 262_144][rng.below(6)];
                            let src = gen_class(cls, sz, rng.next_u64());
                            let cctx = CtxPair::cctx();
                            let cap = compress_bound(sz) + 64;
                            let mut oc = vec![0x5Au8; cap];
                            let mut or_ = vec![0x5Au8; cap];
                            let a = cuc(cctx.c, oc.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, sz, cd.c);
                            let b = cur(cctx.r, or_.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, sz, cd.r);
                            let tag = format!("{what} i={i} cls={cls} sz={sz}");
                            eqv(&format!("{tag} ret"), a, b);
                            eqbuf(&format!("{tag} dst"), &oc, &or_);
                            if is_err(a) {
                                continue;
                            }
                            rt_check(&tag, &oc[..a], &or_[..b], &dict, dct, &src);
                        }
                    }
                }
            }
        }
    }
}

// ------------------------------------------------------------------ row 107

#[test]
fn row107_compress_usingCDict_advanced_fparams() {
    unsafe {
        let (fc, fr) = duo::<FnCompressUsingCDictAdv>("ZSTD_compress_usingCDict_advanced");
        let (idc, idr) = duo::<FnDictIDBuf>("ZSTD_getDictID_fromFrame");
        let mut rng = Rng::new(107);
        for (dn, dict) in dict_shapes_small() {
            for &lvl in &[-5, 1, 6, 12, 19] {
                let _big = maybe_big(dict.len(), lvl);
                let what = format!("row107 dict={dn} lvl={lvl}");
                let cd = cdict_pair(&what, &dict, lvl);
                if !cd.ok() {
                    continue;
                }
                for cs in 0..2 {
                    for ck in 0..2 {
                        for nd in 0..2 {
                            let fp = ZSTD_frameParameters {
                                contentSizeFlag: cs,
                                checksumFlag: ck,
                                noDictIDFlag: nd,
                            };
                            let cls = rng.below(N_CLASSES);
                            let sz = [0usize, 3, 800, 17_000, 100_000][rng.below(5)];
                            let src = gen_class(cls, sz, rng.next_u64());
                            let cctx = CtxPair::cctx();
                            let cap = compress_bound(sz) + 64;
                            let mut oc = vec![0x5Au8; cap];
                            let mut or_ = vec![0x5Au8; cap];
                            let a = fc(
                                cctx.c,
                                oc.as_mut_ptr() as *mut c_void,
                                cap,
                                src.as_ptr() as *const c_void,
                                sz,
                                cd.c,
                                fp,
                            );
                            let b = fr(
                                cctx.r,
                                or_.as_mut_ptr() as *mut c_void,
                                cap,
                                src.as_ptr() as *const c_void,
                                sz,
                                cd.r,
                                fp,
                            );
                            let tag = format!("{what} cs={cs} ck={ck} nd={nd} cls={cls} sz={sz}");
                            eqv(&format!("{tag} ret"), a, b);
                            eqbuf(&format!("{tag} dst"), &oc, &or_);
                            if is_err(a) {
                                continue;
                            }
                            eqv(
                                &format!("{tag} getDictID_fromFrame"),
                                idc(oc.as_ptr() as *const c_void, a),
                                idr(or_.as_ptr() as *const c_void, b),
                            );
                            rt_check(&tag, &oc[..a], &or_[..b], &dict, ZSTD_dct_auto, &src);
                        }
                    }
                }
            }
        }
    }
}

// ------------------------------------------------------------------ row 108

#[test]
fn row108_refCDict_forceAttachDict_strategies() {
    unsafe {
        let (rc, rr) = duo::<FnRefObj>("ZSTD_CCtx_refCDict");
        let (sc, sr) = duo::<FnSetParam>("ZSTD_CCtx_setParameter");
        let (s2c, s2r) = duo::<FnStream2>("ZSTD_compressStream2");
        let dict = trained_big().to_vec();
        let mut rng = Rng::new(108);
        for strat in ALL_STRATEGIES {
            let cp = ZSTD_compressionParameters { strategy: strat as c_uint, ..get_cparams(6, 0, dict.len()) };
            let cd = cdict_adv_pair(
                &format!("row108 strat={strat}"),
                &dict,
                ZSTD_dlm_byCopy,
                ZSTD_dct_auto,
                cp,
            );
            assert!(cd.ok(), "row108: CDict creation failed for strategy {strat}");
            for attach in 0..4 {
                for &sz in &[7usize, 200, 150_000, 262_144] {
                    let cls = rng.below(N_CLASSES);
                    let src = gen_class(cls, sz, rng.next_u64());
                    let cctx = CtxPair::cctx();
                    let a0 = sc(cctx.c, ZSTD_c_forceAttachDict, attach);
                    let b0 = sr(cctx.r, ZSTD_c_forceAttachDict, attach);
                    eqv("row108 setParameter(forceAttachDict)", a0, b0);
                    assert!(!is_err(a0), "row108: forceAttachDict={attach} rejected");
                    let a1 = rc(cctx.c, cd.c);
                    let b1 = rr(cctx.r, cd.r);
                    eqv("row108 CCtx_refCDict", a1, b1);
                    assert!(!is_err(a1), "row108: refCDict failed");
                    let cap = compress_bound(sz) + 64;
                    let (ea, fa) = stream_end(s2c, cctx.c, &src, cap);
                    let (eb, fb) = stream_end(s2r, cctx.r, &src, cap);
                    let tag = format!("row108 strat={strat} attach={attach} sz={sz} cls={cls}");
                    eqv(&format!("{tag} compressStream2 final ret"), ea, eb);
                    eqbuf(&format!("{tag} frame"), &fa, &fb);
                    assert!(!is_err(ea), "{tag}: compressStream2 failed");
                    rt_check(&tag, &fa, &fb, &dict, ZSTD_dct_auto, &src);
                }
            }
            // referencing a NULL CDict returns to no-dictionary mode
            let cctx = CtxPair::cctx();
            let a = rc(cctx.c, std::ptr::null());
            let b = rr(cctx.r, std::ptr::null());
            eqv("row108 refCDict(NULL)", a, b);
            let src = gen_class(4, 5_000, 108);
            let (_, oc, or_) = compress2_pair("row108 refCDict(NULL)", &cctx, &src);
            rt_check("row108 nodict", &oc, &or_, &[], ZSTD_dct_auto, &src);
        }
    }
}

// ------------------------------------------------------------------ row 109

#[test]
fn row109_cctx_loadDictionary_repeated() {
    unsafe {
        let (lc, lr) = duo::<FnLoadDict>("ZSTD_CCtx_loadDictionary");
        let (sc, sr) = duo::<FnSetParam>("ZSTD_CCtx_setParameter");
        let (rsc, rsr) = duo::<FnReset>("ZSTD_CCtx_reset");
        let shapes = dict_shapes();
        let mut rng = Rng::new(109);
        for (dn, dict) in &shapes {
            for &lvl in &LEVELS {
                let cctx = CtxPair::cctx();
                let a = sc(cctx.c, ZSTD_c_compressionLevel, lvl);
                let b = sr(cctx.r, ZSTD_c_compressionLevel, lvl);
                eqv("row109 setParameter(level)", a, b);
                let what = format!("row109 dict={dn} lvl={lvl}");
                // repeated loads: the last one must win
                let first = &shapes[rng.below(shapes.len())].1;
                let x = lc(cctx.c, first.as_ptr() as *const c_void, first.len());
                let y = lr(cctx.r, first.as_ptr() as *const c_void, first.len());
                eqv(&format!("{what} loadDictionary(first)"), x, y);
                let x = lc(cctx.c, dict.as_ptr() as *const c_void, dict.len());
                let y = lr(cctx.r, dict.as_ptr() as *const c_void, dict.len());
                eqv(&format!("{what} loadDictionary(second)"), x, y);
                if is_err(x) {
                    continue;
                }
                for i in 0..3 {
                    let cls = rng.below(N_CLASSES);
                    let sz = pick_size(&mut rng, lvl);
                    let src = gen_class(cls, sz, rng.next_u64());
                    let tag = format!("{what} i={i} cls={cls} sz={sz}");
                    let (n, oc, or_) = compress2_pair(&tag, &cctx, &src);
                    if is_err(n) {
                        continue;
                    }
                    rt_check(&tag, &oc, &or_, dict, ZSTD_dct_auto, &src);
                }
                // NULL dict clears the dictionary
                let x = lc(cctx.c, std::ptr::null(), 0);
                let y = lr(cctx.r, std::ptr::null(), 0);
                eqv(&format!("{what} loadDictionary(NULL)"), x, y);
                let a = rsc(cctx.c, ZSTD_reset_session_only);
                let b = rsr(cctx.r, ZSTD_reset_session_only);
                eqv(&format!("{what} reset"), a, b);
                let src = gen_class(5, 4_000, 1090 + lvl as u64);
                let (n, oc, or_) = compress2_pair(&format!("{what} cleared"), &cctx, &src);
                if !is_err(n) {
                    rt_check(&format!("{what} cleared"), &oc, &or_, &[], ZSTD_dct_auto, &src);
                }
            }
        }
    }
}

// ------------------------------------------------------------------ row 110

#[test]
fn row110_cctx_loadDictionary_byref_advanced() {
    unsafe {
        let (bc, br) = duo::<FnLoadDict>("ZSTD_CCtx_loadDictionary_byReference");
        let (ac, ar) = duo::<FnLoadDictAdv>("ZSTD_CCtx_loadDictionary_advanced");
        let (sc, sr) = duo::<FnSetParam>("ZSTD_CCtx_setParameter");
        let mut rng = Rng::new(110);
        for (dn, dict) in dict_shapes() {
            // byReference entry point
            for &lvl in &[1, 7, 19] {
                let cctx = CtxPair::cctx();
                eqv(
                    "row110 setParameter(level)",
                    sc(cctx.c, ZSTD_c_compressionLevel, lvl),
                    sr(cctx.r, ZSTD_c_compressionLevel, lvl),
                );
                let what = format!("row110 byRef dict={dn} lvl={lvl}");
                let x = bc(cctx.c, dict.as_ptr() as *const c_void, dict.len());
                let y = br(cctx.r, dict.as_ptr() as *const c_void, dict.len());
                eqv(&format!("{what} loadDictionary_byReference"), x, y);
                if is_err(x) {
                    continue;
                }
                for i in 0..2 {
                    let cls = rng.below(N_CLASSES);
                    let sz = pick_size(&mut rng, lvl);
                    let src = gen_class(cls, sz, rng.next_u64());
                    let tag = format!("{what} i={i} cls={cls} sz={sz}");
                    let (n, oc, or_) = compress2_pair(&tag, &cctx, &src);
                    if !is_err(n) {
                        rt_check(&tag, &oc, &or_, &dict, ZSTD_dct_auto, &src);
                    }
                }
            }
            // _advanced: full dictLoadMethod × dictContentType grid
            for dlm in [ZSTD_dlm_byCopy, ZSTD_dlm_byRef] {
                for dct in [ZSTD_dct_auto, ZSTD_dct_rawContent, ZSTD_dct_fullDict] {
                    for &lvl in &[1, 9] {
                        let cctx = CtxPair::cctx();
                        eqv(
                            "row110 setParameter(level)",
                            sc(cctx.c, ZSTD_c_compressionLevel, lvl),
                            sr(cctx.r, ZSTD_c_compressionLevel, lvl),
                        );
                        let what = format!("row110 adv dict={dn} dlm={dlm} dct={dct} lvl={lvl}");
                        let x = ac(cctx.c, dict.as_ptr() as *const c_void, dict.len(), dlm, dct);
                        let y = ar(cctx.r, dict.as_ptr() as *const c_void, dict.len(), dlm, dct);
                        eqv(&format!("{what} loadDictionary_advanced"), x, y);
                        if is_err(x) {
                            continue;
                        }
                        for i in 0..2 {
                            let cls = rng.below(N_CLASSES);
                            let sz = pick_size(&mut rng, lvl);
                            let src = gen_class(cls, sz, rng.next_u64());
                            let tag = format!("{what} i={i} cls={cls} sz={sz}");
                            let (n, oc, or_) = compress2_pair(&tag, &cctx, &src);
                            if !is_err(n) {
                                rt_check(&tag, &oc, &or_, &dict, dct, &src);
                            }
                        }
                    }
                }
            }
        }
    }
}

// ------------------------------------------------------------------ row 111

#[test]
fn row111_cctx_refPrefix_deterministic() {
    unsafe {
        let (pc, pr) = duo::<FnLoadDict>("ZSTD_CCtx_refPrefix");
        let (apc, apr) = duo::<FnRefPrefixAdv>("ZSTD_CCtx_refPrefix_advanced");
        let (dpc, dpr) = duo::<FnLoadDict>("ZSTD_DCtx_refPrefix");
        let (dapc, dapr) = duo::<FnRefPrefixAdv>("ZSTD_DCtx_refPrefix_advanced");
        let (sc, sr) = duo::<FnSetParam>("ZSTD_CCtx_setParameter");
        let (dc, dr) = duo::<FnDecompressDCtx>("ZSTD_decompressDCtx");
        let mut rng = Rng::new(111);
        let trained = trained_big().to_vec();
        for det in [0, 1] {
            for &lvl in &[1, 6, 19] {
                for pi in 0..8usize {
                    // prefix ∈ {random, previous frame content, trained dict, empty}
                    let payload_cls = rng.below(N_CLASSES);
                    let sz = [1usize, 60, 3_000, 40_000, 150_000][rng.below(5)];
                    let src = gen_class(payload_cls, sz, rng.next_u64());
                    let prefix: Vec<u8> = match pi {
                        0 => Vec::new(),
                        1 => gen_class(3, 1 + rng.below(4096), rng.next_u64()),
                        2 => src.clone(), // "previous frame content"
                        3 => {
                            let n = src.len().min(2048);
                            src[..n].to_vec()
                        }
                        4 => trained.clone(),
                        _ => gen_class(4, 8192, rng.next_u64()),
                    };
                    for adv in [false, true] {
                        let dct = if adv {
                            [ZSTD_dct_auto, ZSTD_dct_rawContent, ZSTD_dct_fullDict][rng.below(3)]
                        } else {
                            ZSTD_dct_rawContent // refPrefix() implies rawContent
                        };
                        let cctx = CtxPair::cctx();
                        eqv(
                            "row111 setParameter(level)",
                            sc(cctx.c, ZSTD_c_compressionLevel, lvl),
                            sr(cctx.r, ZSTD_c_compressionLevel, lvl),
                        );
                        let a0 = sc(cctx.c, ZSTD_c_deterministicRefPrefix, det);
                        let b0 = sr(cctx.r, ZSTD_c_deterministicRefPrefix, det);
                        eqv("row111 setParameter(deterministicRefPrefix)", a0, b0);
                        assert!(!is_err(a0), "row111: deterministicRefPrefix={det} rejected");
                        let what = format!(
                            "row111 det={det} lvl={lvl} pi={pi} adv={adv} dct={dct} sz={sz} cls={payload_cls}"
                        );
                        let (x, y) = if adv {
                            (
                                apc(cctx.c, prefix.as_ptr() as *const c_void, prefix.len(), dct),
                                apr(cctx.r, prefix.as_ptr() as *const c_void, prefix.len(), dct),
                            )
                        } else {
                            (
                                pc(cctx.c, prefix.as_ptr() as *const c_void, prefix.len()),
                                pr(cctx.r, prefix.as_ptr() as *const c_void, prefix.len()),
                            )
                        };
                        eqv(&format!("{what} CCtx_refPrefix"), x, y);
                        if is_err(x) {
                            continue;
                        }
                        let (n, oc, or_) = compress2_pair(&what, &cctx, &src);
                        if is_err(n) {
                            continue;
                        }
                        // decode with the mirror-image prefix
                        let dctx = CtxPair::dctx();
                        let (u, v) = if adv {
                            (
                                dapc(dctx.c, prefix.as_ptr() as *const c_void, prefix.len(), dct),
                                dapr(dctx.r, prefix.as_ptr() as *const c_void, prefix.len(), dct),
                            )
                        } else {
                            (
                                dpc(dctx.c, prefix.as_ptr() as *const c_void, prefix.len()),
                                dpr(dctx.r, prefix.as_ptr() as *const c_void, prefix.len()),
                            )
                        };
                        eqv(&format!("{what} DCtx_refPrefix"), u, v);
                        let cap = sz + 64;
                        let mut pcb = vec![0xB7u8; cap];
                        let mut prb = vec![0xB7u8; cap];
                        let e = dc(dctx.c, pcb.as_mut_ptr() as *mut c_void, cap, oc.as_ptr() as *const c_void, oc.len());
                        let f = dr(dctx.r, prb.as_mut_ptr() as *mut c_void, cap, or_.as_ptr() as *const c_void, or_.len());
                        eqv(&format!("{what} decode ret"), e, f);
                        cmp_decode_bufs(&format!("{what} decode"), e, &pcb, &prb);
                        if !is_err(e) {
                            assert_eq!(e, sz, "{what}: wrong decompressed size");
                            eqbuf(&format!("{what} payload"), &src, &pcb[..e]);
                        }
                    }
                }
            }
        }
    }
}

// ------------------------------------------------------------------ row 112

#[test]
fn row112_createDDict_variants() {
    unsafe {
        let (duc, dur) = duo::<FnDecompressUsingDDict>("ZSTD_decompress_usingDDict");
        let mut rng = Rng::new(112);
        for (dn, dict) in dict_shapes() {
            let plain = ddict_pair(&format!("row112 plain dict={dn}"), &dict);
            check_ddict_getters(&format!("row112 plain dict={dn}"), &plain);
            let byref = ddict_byref_pair(&format!("row112 byRef dict={dn}"), &dict);
            check_ddict_getters(&format!("row112 byRef dict={dn}"), &byref);
            for dlm in [ZSTD_dlm_byCopy, ZSTD_dlm_byRef] {
                for dct in [ZSTD_dct_auto, ZSTD_dct_rawContent, ZSTD_dct_fullDict] {
                    let what = format!("row112 adv dict={dn} dlm={dlm} dct={dct}");
                    let dd = ddict_adv_pair(&what, &dict, dlm, dct);
                    check_ddict_getters(&what, &dd);
                    if !dd.ok() {
                        continue;
                    }
                    // build a frame with the C library using the same dict view,
                    // then decode it with each library's own DDict
                    let cls = rng.below(N_CLASSES);
                    let sz = [0usize, 8, 1_000, 30_000, 200_000][rng.below(5)];
                    let src = gen_class(cls, sz, rng.next_u64());
                    let frame = c_frame_with_dict(&src, 5, &dict, &[]);
                    let cap = sz + 64;
                    let dctx = CtxPair::dctx();
                    let mut oc = vec![0x9Cu8; cap];
                    let mut or2 = vec![0x9Cu8; cap];
                    let x = duc(
                        dctx.c,
                        oc.as_mut_ptr() as *mut c_void,
                        cap,
                        frame.as_ptr() as *const c_void,
                        frame.len(),
                        dd.c,
                    );
                    let y = dur(
                        dctx.r,
                        or2.as_mut_ptr() as *mut c_void,
                        cap,
                        frame.as_ptr() as *const c_void,
                        frame.len(),
                        dd.r,
                    );
                    let tag = format!("{what} cls={cls} sz={sz}");
                    eqv(&format!("{tag} decompress_usingDDict ret"), x, y);
                    cmp_decode_bufs(&format!("{tag} decompress_usingDDict"), x, &oc, &or2);
                    if !is_err(x) {
                        assert_eq!(x, sz, "{tag}: wrong decompressed size");
                        eqbuf(&format!("{tag} payload"), &src, &oc[..x]);
                    }
                }
            }
        }
    }
}

// ------------------------------------------------------------------ row 113

#[test]
fn row113_refDDict_multiple_ddicts() {
    unsafe {
        let (sc, sr) = duo::<FnSetParam>("ZSTD_DCtx_setParameter");
        let (rc, rr) = duo::<FnRefObj>("ZSTD_DCtx_refDDict");
        let (dsc, dsr) = duo::<FnDStream>("ZSTD_decompressStream");
        let (rsc, rsr) = duo::<FnReset>("ZSTD_DCtx_reset");
        let base = trained_big();
        // four distinct dictionaries with four distinct dictIDs
        let dicts: Vec<Vec<u8>> = (0..4)
            .map(|i| {
                let mut d = if i % 2 == 0 {
                    trained_big().to_vec()
                } else {
                    trained_small().to_vec()
                };
                let id = 11 + 11 * (i as u32);
                d[4..8].copy_from_slice(&id.to_le_bytes());
                d
            })
            .collect();
        assert!(!base.is_empty());
        let mut rng = Rng::new(113);
        // ground-truth frames, one per dictionary
        let payloads: Vec<Vec<u8>> = (0..4)
            .map(|i| gen_class(4 + i % 3, 2_000 + 3_000 * i, 1130 + i as u64))
            .collect();
        let frames: Vec<Vec<u8>> = (0..4)
            .map(|i| c_frame_with_dict(&payloads[i], 6, &dicts[i], &[]))
            .collect();
        for multi in [0, 1] {
          for byref in [false, true] {
            for nb in 1..=4usize {
                let dctx = CtxPair::dctx();
                let a = sc(dctx.c, ZSTD_d_refMultipleDDicts, multi);
                let b = sr(dctx.r, ZSTD_d_refMultipleDDicts, multi);
                eqv("row113 setParameter(refMultipleDDicts)", a, b);
                assert!(!is_err(a), "row113: refMultipleDDicts={multi} rejected");
                let objs: Vec<DictObj> = (0..nb)
                    .map(|i| {
                        if byref {
                            ddict_byref_pair(&format!("row113 ddict#{i} byRef"), &dicts[i])
                        } else {
                            ddict_pair(&format!("row113 ddict#{i}"), &dicts[i])
                        }
                    })
                    .collect();
                for (i, o) in objs.iter().enumerate() {
                    assert!(o.ok(), "row113: DDict #{i} creation failed");
                    let x = rc(dctx.c, o.c);
                    let y = rr(dctx.r, o.r);
                    eqv(&format!("row113 refDDict #{i} (multi={multi} nb={nb})"), x, y);
                    assert!(!is_err(x), "row113: refDDict #{i} failed");
                }
                for fi in 0..4usize {
                    let a = rsc(dctx.c, ZSTD_reset_session_only);
                    let b = rsr(dctx.r, ZSTD_reset_session_only);
                    eqv("row113 DCtx_reset(session_only)", a, b);
                    let frame = &frames[fi];
                    let want = &payloads[fi];
                    let cap = want.len() + 64;
                    let mut oc = vec![0x11u8; cap];
                    let mut or_ = vec![0x11u8; cap];
                    let mut inc = ZSTD_inBuffer {
                        src: frame.as_ptr() as *const c_void,
                        size: frame.len(),
                        pos: 0,
                    };
                    let mut inr = inc;
                    let mut outc = ZSTD_outBuffer {
                        dst: oc.as_mut_ptr() as *mut c_void,
                        size: cap,
                        pos: 0,
                    };
                    let mut outr = ZSTD_outBuffer {
                        dst: or_.as_mut_ptr() as *mut c_void,
                        size: cap,
                        pos: 0,
                    };
                    let mut ra;
                    loop {
                        ra = dsc(dctx.c, &mut outc, &mut inc);
                        if is_err(ra) || ra == 0 || inc.pos == inc.size {
                            break;
                        }
                    }
                    let mut rb;
                    loop {
                        rb = dsr(dctx.r, &mut outr, &mut inr);
                        if is_err(rb) || rb == 0 || inr.pos == inr.size {
                            break;
                        }
                    }
                    let tag = format!("row113 multi={multi} byref={byref} nb={nb} frame={fi}");
                    eqv(&format!("{tag} decompressStream ret"), ra, rb);
                    eqv(&format!("{tag} out.pos"), outc.pos, outr.pos);
                    eqv(&format!("{tag} in.pos"), inc.pos, inr.pos);
                    if is_err(ra) {
                        eqbuf(&format!("{tag} dst (error path)"), &oc, &or_);
                    } else {
                        eqbuf(&format!("{tag} dst"), &oc[..outc.pos], &or_[..outr.pos]);
                    }
                    // With multiple DDicts referenced the right one must be found;
                    // with a single DDict only the last one referenced can work.
                    let expect_ok = if multi == 1 { fi < nb } else { fi == nb - 1 };
                    if expect_ok {
                        assert!(!is_err(ra), "{tag}: expected success, got error {ra:#x}");
                        assert_eq!(outc.pos, want.len(), "{tag}: short output");
                        eqbuf(&format!("{tag} payload"), want, &oc[..outc.pos]);
                    }
                    let _ = &mut rng;
                }
                // returning to no-dictionary mode
                let x = rc(dctx.c, std::ptr::null());
                let y = rr(dctx.r, std::ptr::null());
                eqv("row113 refDDict(NULL)", x, y);
            }
          }
        }
    }
}

// ------------------------------------------------------------------ row 114

#[test]
fn row114_dctx_loadDictionary_and_refPrefix() {
    unsafe {
        let (lc, lr) = duo::<FnLoadDict>("ZSTD_DCtx_loadDictionary");
        let (bc, br) = duo::<FnLoadDict>("ZSTD_DCtx_loadDictionary_byReference");
        let (ac, ar) = duo::<FnLoadDictAdv>("ZSTD_DCtx_loadDictionary_advanced");
        let (pc, pr) = duo::<FnLoadDict>("ZSTD_DCtx_refPrefix");
        let (apc, apr) = duo::<FnRefPrefixAdv>("ZSTD_DCtx_refPrefix_advanced");
        let (dc, dr) = duo::<FnDecompressDCtx>("ZSTD_decompressDCtx");
        let (uc, ur) = duo::<FnDecompressUsingDict>("ZSTD_decompress_usingDict");
        let mut rng = Rng::new(114);
        for (dn, dict) in dict_shapes() {
          for it in 0..3usize {
            let cls = rng.below(N_CLASSES);
            let sz = [0usize, 5, 1_200, 20_000, 150_000][rng.below(5)];
            let lvl = [1, 4, 9, 19][rng.below(4)];
            let src = gen_class(cls, sz, rng.next_u64());
            let frame = c_frame_with_dict(&src, lvl, &dict, &[]);
            let cap = sz + 64;
            // 0: loadDictionary, 1: _byReference, 2..: _advanced grid,
            // then refPrefix / refPrefix_advanced
            let mut modes: Vec<(String, u8, c_int, c_int)> = vec![
                ("load".into(), 0, ZSTD_dlm_byCopy, ZSTD_dct_auto),
                ("loadByRef".into(), 1, ZSTD_dlm_byRef, ZSTD_dct_auto),
            ];
            for dlm in [ZSTD_dlm_byCopy, ZSTD_dlm_byRef] {
                for dct in [ZSTD_dct_auto, ZSTD_dct_rawContent, ZSTD_dct_fullDict] {
                    modes.push((format!("loadAdv dlm={dlm} dct={dct}"), 2, dlm, dct));
                }
            }
            modes.push(("refPrefix".into(), 3, ZSTD_dlm_byCopy, ZSTD_dct_rawContent));
            for dct in [ZSTD_dct_auto, ZSTD_dct_rawContent, ZSTD_dct_fullDict] {
                modes.push((format!("refPrefixAdv dct={dct}"), 4, ZSTD_dlm_byCopy, dct));
            }
            for (mn, kind, dlm, dct) in modes {
                let dctx = CtxPair::dctx();
                let what = format!("row114 dict={dn} it={it} lvl={lvl} {mn} cls={cls} sz={sz}");
                let (x, y) = match kind {
                    0 => (
                        lc(dctx.c, dict.as_ptr() as *const c_void, dict.len()),
                        lr(dctx.r, dict.as_ptr() as *const c_void, dict.len()),
                    ),
                    1 => (
                        bc(dctx.c, dict.as_ptr() as *const c_void, dict.len()),
                        br(dctx.r, dict.as_ptr() as *const c_void, dict.len()),
                    ),
                    2 => (
                        ac(dctx.c, dict.as_ptr() as *const c_void, dict.len(), dlm, dct),
                        ar(dctx.r, dict.as_ptr() as *const c_void, dict.len(), dlm, dct),
                    ),
                    3 => (
                        pc(dctx.c, dict.as_ptr() as *const c_void, dict.len()),
                        pr(dctx.r, dict.as_ptr() as *const c_void, dict.len()),
                    ),
                    _ => (
                        apc(dctx.c, dict.as_ptr() as *const c_void, dict.len(), dct),
                        apr(dctx.r, dict.as_ptr() as *const c_void, dict.len(), dct),
                    ),
                };
                eqv(&format!("{what} load/ref status"), x, y);
                if is_err(x) {
                    continue;
                }
                let mut oc = vec![0x2Bu8; cap];
                let mut or_ = vec![0x2Bu8; cap];
                let a = dc(dctx.c, oc.as_mut_ptr() as *mut c_void, cap, frame.as_ptr() as *const c_void, frame.len());
                let b = dr(dctx.r, or_.as_mut_ptr() as *mut c_void, cap, frame.as_ptr() as *const c_void, frame.len());
                eqv(&format!("{what} decompressDCtx ret"), a, b);
                cmp_decode_bufs(&format!("{what} decompressDCtx"), a, &oc, &or_);
                if !is_err(a) {
                    assert_eq!(a, sz, "{what}: wrong size");
                    eqbuf(&format!("{what} payload"), &src, &oc[..a]);
                }
                if kind == 0 {
                    // cross-check the one-shot dictionary entry point
                    let d2 = CtxPair::dctx();
                    let mut uu = vec![0x6Eu8; cap];
                    let mut vv = vec![0x6Eu8; cap];
                    let e = uc(
                        d2.c,
                        uu.as_mut_ptr() as *mut c_void,
                        cap,
                        frame.as_ptr() as *const c_void,
                        frame.len(),
                        dict.as_ptr() as *const c_void,
                        dict.len(),
                    );
                    let f = ur(
                        d2.r,
                        vv.as_mut_ptr() as *mut c_void,
                        cap,
                        frame.as_ptr() as *const c_void,
                        frame.len(),
                        dict.as_ptr() as *const c_void,
                        dict.len(),
                    );
                    eqv(&format!("{what} decompress_usingDict ret"), e, f);
                    cmp_decode_bufs(&format!("{what} decompress_usingDict"), e, &uu, &vv);
                    eqv(&format!("{what} usingDict vs loadDictionary ret"), a, e);
                    if !is_err(e) {
                        eqbuf(&format!("{what} usingDict payload"), &src, &uu[..e]);
                    }
                }
            }
          }
        }
    }
}

// ------------------------------------------------------------------ row 115

#[test]
fn row115_getDictID_all_sources() {
    unsafe {
        let (fdc, fdr) = duo::<FnDictIDBuf>("ZSTD_getDictID_fromDict");
        let (ffc, ffr) = duo::<FnDictIDBuf>("ZSTD_getDictID_fromFrame");
        let (icc, icr) = duo::<FnDictIDObj>("ZSTD_getDictID_fromCDict");
        let (idc, idr) = duo::<FnDictIDObj>("ZSTD_getDictID_fromDDict");
        let (sc, sr) = duo::<FnSetParam>("ZSTD_CCtx_setParameter");
        let (rc, rr) = duo::<FnRefObj>("ZSTD_CCtx_refCDict");
        // every DID field width: 0 (absent), 1, 2 and 4 bytes
        let ids: [u32; 9] = [0, 1, 2, 255, 256, 257, 65_535, 65_536, 0xFFFF_FFFF];
        let mut cases: Vec<(String, Vec<u8>)> = Vec::new();
        for id in ids {
            cases.push((format!("trained,id={id}"), with_dict_id(trained_big(), id)));
        }
        cases.push(("raw1k".into(), gen_class(3, 1024, 4)));
        cases.push(("tiny7".into(), gen_class(3, 7, 2)));
        cases.push(("empty".into(), Vec::new()));
        cases.push(("badmagic".into(), bad_magic(trained_big())));
        let src = gen_class(4, 20_000, 115);
        for (cn, dict) in &cases {
            // fromDict, including every truncation of the 8-byte header
            for cut in [0usize, 1, 3, 4, 5, 7, 8, usize::MAX] {
                let n = if cut == usize::MAX { dict.len() } else { cut.min(dict.len()) };
                eqv(
                    &format!("row115 {cn} getDictID_fromDict(len={n})"),
                    fdc(dict.as_ptr() as *const c_void, n),
                    fdr(dict.as_ptr() as *const c_void, n),
                );
            }
            for &dflag in &[0, 1] {
                let cd = cdict_pair(&format!("row115 {cn}"), dict, 5);
                let dd = ddict_pair(&format!("row115 {cn}"), dict);
                if cd.ok() {
                    eqv(&format!("row115 {cn} fromCDict"), icc(cd.c), icr(cd.r));
                }
                if dd.ok() {
                    eqv(&format!("row115 {cn} fromDDict"), idc(dd.c), idr(dd.r));
                }
                if !cd.ok() {
                    continue;
                }
                let cctx = CtxPair::cctx();
                let a = sc(cctx.c, ZSTD_c_dictIDFlag, dflag);
                let b = sr(cctx.r, ZSTD_c_dictIDFlag, dflag);
                eqv("row115 setParameter(dictIDFlag)", a, b);
                let x = rc(cctx.c, cd.c);
                let y = rr(cctx.r, cd.r);
                eqv("row115 refCDict", x, y);
                let tag = format!("row115 {cn} dictIDFlag={dflag}");
                let (n, oc, or_) = compress2_pair(&tag, &cctx, &src);
                if is_err(n) {
                    continue;
                }
                let fa = ffc(oc.as_ptr() as *const c_void, oc.len());
                let fb = ffr(or_.as_ptr() as *const c_void, or_.len());
                eqv(&format!("{tag} getDictID_fromFrame"), fa, fb);
                // truncated frames too
                for cut in [0usize, 1, 4, 5, 6, 8] {
                    let m = cut.min(oc.len());
                    eqv(
                        &format!("{tag} getDictID_fromFrame(len={m})"),
                        ffc(oc.as_ptr() as *const c_void, m),
                        ffr(or_.as_ptr() as *const c_void, m),
                    );
                }
                if dflag == 0 {
                    assert_eq!(fa, 0, "{tag}: dictIDFlag=0 must not write a DID");
                }
                rt_check(&tag, &oc, &or_, dict, ZSTD_dct_auto, &src);
            }
        }
    }
}

// ------------------------------------------------------------------ row 116

#[test]
fn row116_ddict_content_size_and_copyParams() {
    unsafe {
        let (cpc, cpr) = duo::<FnCopyDDictParams>("ZSTD_copyDDictParameters");
        let (bc, br) = duo::<FnBegin>("ZSTD_decompressBegin");
        let (nc, nr) = duo::<FnNextSrc>("ZSTD_nextSrcSizeToDecompress");
        let (kc, kr) = duo::<FnDecompressContinue>("ZSTD_decompressContinue");
        let mut rng = Rng::new(116);
        for (dn, dict) in dict_shapes() {
            let mut variants: Vec<(String, DictObj)> = Vec::new();
            variants.push(("plain".into(), ddict_pair(&format!("row116 {dn} plain"), &dict)));
            variants.push(("byRef".into(), ddict_byref_pair(&format!("row116 {dn} byRef"), &dict)));
            for dlm in [ZSTD_dlm_byCopy, ZSTD_dlm_byRef] {
                for dct in [ZSTD_dct_auto, ZSTD_dct_rawContent, ZSTD_dct_fullDict] {
                    variants.push((
                        format!("adv dlm={dlm} dct={dct}"),
                        ddict_adv_pair(&format!("row116 {dn} adv{dlm}{dct}"), &dict, dlm, dct),
                    ));
                }
            }
            let cls = rng.below(N_CLASSES);
            let sz = [0usize, 7, 900, 16_000][rng.below(4)];
            let src = gen_class(cls, sz, rng.next_u64());
            let frame = c_frame_with_dict(&src, 3, &dict, &[]);
            for (vn, dd) in &variants {
                let what = format!("row116 dict={dn} {vn}");
                check_ddict_getters(&what, dd);
                if !dd.ok() {
                    continue;
                }
                // ZSTD_copyDDictParameters + bufferless decode
                let dctx = CtxPair::dctx();
                eqv(&format!("{what} decompressBegin"), bc(dctx.c), br(dctx.r));
                cpc(dctx.c, dd.c);
                cpr(dctx.r, dd.r);
                let cap = sz + 64;
                let (ra, oa) = bufferless_decode(nc, kc, dctx.c, &frame, cap);
                let (rb, ob) = bufferless_decode(nr, kr, dctx.r, &frame, cap);
                eqv(&format!("{what} bufferless decode ret"), ra, rb);
                eqbuf(&format!("{what} bufferless decode dst"), &oa, &ob);
                if !is_err(ra) && ra != usize::MAX - 200 {
                    assert_eq!(ra, sz, "{what}: bufferless decode short");
                    eqbuf(&format!("{what} payload"), &src, &oa);
                }
            }
        }
    }
}

// ------------------------------------------------------------------ row 117

#[test]
fn row117_getCParamsFromCDict() {
    unsafe {
        let (pc, pr) = duo::<FnCParamsFromCDict>("ZSTD_getCParamsFromCDict");
        for (dn, dict) in dict_shapes() {
            for &lvl in &[-5, -1, 0, 1, 3, 6, 9, 12, 15, 17, 19, 20, 22] {
                let _big = maybe_big(dict.len(), lvl);
                let a = cdict_pair(&format!("row117 {dn} lvl={lvl}"), &dict, lvl);
                if a.ok() {
                    eqv(&format!("row117 createCDict {dn} lvl={lvl}"), pc(a.c), pr(a.r));
                }
                let b = cdict_byref_pair(&format!("row117 byref {dn} lvl={lvl}"), &dict, lvl);
                if b.ok() {
                    eqv(&format!("row117 byReference {dn} lvl={lvl}"), pc(b.c), pr(b.r));
                }
            }
            for dlm in [ZSTD_dlm_byCopy, ZSTD_dlm_byRef] {
                for dct in [ZSTD_dct_auto, ZSTD_dct_rawContent, ZSTD_dct_fullDict] {
                    for &lvl in &[1, 8, 19] {
                        let _big = maybe_big(dict.len(), lvl);
                        let cp = get_cparams(lvl, 0, dict.len());
                        let d = cdict_adv_pair(
                            &format!("row117 adv {dn} {dlm} {dct} {lvl}"),
                            &dict,
                            dlm,
                            dct,
                            cp,
                        );
                        if d.ok() {
                            let got = pc(d.c);
                            eqv(&format!("row117 adv {dn} {dlm} {dct} {lvl}"), got, pr(d.r));
                        }
                    }
                }
            }
        }
    }
}

// ------------------------------------------------------------------ row 118

/// `sizeof(ZSTD_compressedBlockState_t)` in the C build (probed with `offsetof`
/// on the real headers); the buffer is deliberately oversized.
const CBLOCK_STATE_SIZE: usize = 5632;
/// `ENTROPY_WORKSPACE_SIZE`
const ENTROPY_WKSP_SIZE: usize = 8920;
/// `sizeof(ZSTD_entropyDTables_t)`
const DTABLES_SIZE: usize = 27_292;

fn zeroed_u64(bytes: usize) -> Vec<u64> {
    vec![0u64; (bytes + 7) / 8 + 8]
}

fn as_bytes(v: &[u64], n: usize) -> &[u8] {
    unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u8, n) }
}

#[test]
fn row118_loadCEntropy_loadDEntropy() {
    unsafe {
        let (cc, cr) = duo::<FnLoadCEntropy>("ZSTD_loadCEntropy");
        let (dc, dr) = duo::<FnLoadDEntropy>("ZSTD_loadDEntropy");
        let mut inputs: Vec<(String, Vec<u8>)> = Vec::new();
        for id in [0u32, 1, 300, 70_000, 0xFFFF_FFFF] {
            inputs.push((format!("trainedB id={id}"), with_dict_id(trained_big(), id)));
            inputs.push((format!("trainedS id={id}"), with_dict_id(trained_small(), id)));
        }
        // truncated payloads (still >= 8 bytes, as the contract requires)
        let t = trained_big().to_vec();
        for frac in [8usize, 9, 16, 64, 256, 1024] {
            if frac < t.len() {
                inputs.push((format!("trainedB trunc={frac}"), t[..frac].to_vec()));
            }
        }
        // a dict whose entropy section has been perturbed
        let mut broken = t.clone();
        for i in 8..24.min(broken.len()) {
            broken[i] ^= 0x5A;
        }
        inputs.push(("trainedB perturbed".into(), broken));
        for (n, dict) in &inputs {
            // --- ZSTD_loadCEntropy
            let mut bsc = zeroed_u64(CBLOCK_STATE_SIZE);
            let mut bsr = zeroed_u64(CBLOCK_STATE_SIZE);
            let mut wc = zeroed_u64(ENTROPY_WKSP_SIZE);
            let mut wr = zeroed_u64(ENTROPY_WKSP_SIZE);
            let a = cc(
                bsc.as_mut_ptr() as *mut c_void,
                wc.as_mut_ptr() as *mut c_void,
                dict.as_ptr() as *const c_void,
                dict.len(),
            );
            let b = cr(
                bsr.as_mut_ptr() as *mut c_void,
                wr.as_mut_ptr() as *mut c_void,
                dict.as_ptr() as *const c_void,
                dict.len(),
            );
            eqv(&format!("row118 loadCEntropy({n}) ret"), a, b);
            eqbuf(
                &format!("row118 loadCEntropy({n}) compressedBlockState"),
                as_bytes(&bsc, CBLOCK_STATE_SIZE),
                as_bytes(&bsr, CBLOCK_STATE_SIZE),
            );
            // --- ZSTD_loadDEntropy
            let mut dtc = zeroed_u64(DTABLES_SIZE);
            let mut dtr = zeroed_u64(DTABLES_SIZE);
            let x = dc(
                dtc.as_mut_ptr() as *mut c_void,
                dict.as_ptr() as *const c_void,
                dict.len(),
            );
            let y = dr(
                dtr.as_mut_ptr() as *mut c_void,
                dict.as_ptr() as *const c_void,
                dict.len(),
            );
            eqv(&format!("row118 loadDEntropy({n}) ret"), x, y);
            eqbuf(
                &format!("row118 loadDEntropy({n}) entropyDTables"),
                as_bytes(&dtc, DTABLES_SIZE),
                as_bytes(&dtr, DTABLES_SIZE),
            );
        }
    }
}

// ------------------------------------------------------------------ row 119

#[test]
fn row119_dedicatedDictSearch_via_cdict() {
    unsafe {
        let (psc, psr) = duo::<FnSetParam>("ZSTD_CCtxParams_setParameter");
        let (rc, rr) = duo::<FnRefObj>("ZSTD_CCtx_refCDict");
        let (sc, sr) = duo::<FnSetParam>("ZSTD_CCtx_setParameter");
        let dicts: Vec<(String, Vec<u8>)> = vec![
            ("trainedB".into(), trained_big().to_vec()),
            ("raw10k".into(), gen_class(3, 10_000, 5)),
            ("text8k".into(), gen_class(4, 8192, 9)),
        ];
        let mut rng = Rng::new(119);
        for (dn, dict) in &dicts {
            for dds in [0, 1] {
                // greedy(3), lazy(4), lazy2(5) are the DDS-capable strategies;
                // btlazy2(6) exercises the "unsupported → fall back" branch.
                for strat in [3, 4, 5, 6] {
                    for &lvl in &[6, 12] {
                        let params = CtxPair::cctx_params();
                        for (p, v) in [
                            (ZSTD_c_compressionLevel, lvl),
                            (ZSTD_c_enableDedicatedDictSearch, dds),
                            (ZSTD_c_strategy, strat),
                        ] {
                            let a = psc(params.c, p, v);
                            let b = psr(params.r, p, v);
                            eqv(&format!("row119 CCtxParams_setParameter({p},{v})"), a, b);
                            assert!(!is_err(a), "row119 setParameter({p},{v}) rejected");
                        }
                        let what = format!("row119 dict={dn} dds={dds} strat={strat} lvl={lvl}");
                        let cd = cdict_adv2_pair(&what, dict, ZSTD_dlm_byCopy, ZSTD_dct_auto, &params);
                        check_cdict_getters(&what, &cd);
                        assert!(cd.ok(), "{what}: CDict creation failed");
                        for attach in [0, 1, 2, 3] {
                            let cls = rng.below(N_CLASSES);
                            let sz = [30usize, 4_000, 120_000][rng.below(3)];
                            let src = gen_class(cls, sz, rng.next_u64());
                            let cctx = CtxPair::cctx();
                            let a = sc(cctx.c, ZSTD_c_forceAttachDict, attach);
                            let b = sr(cctx.r, ZSTD_c_forceAttachDict, attach);
                            eqv("row119 setParameter(forceAttachDict)", a, b);
                            let x = rc(cctx.c, cd.c);
                            let y = rr(cctx.r, cd.r);
                            eqv(&format!("{what} refCDict"), x, y);
                            assert!(!is_err(x), "{what}: refCDict failed");
                            let tag = format!("{what} attach={attach} cls={cls} sz={sz}");
                            let (n, oc, or_) = compress2_pair(&tag, &cctx, &src);
                            if is_err(n) {
                                continue;
                            }
                            rt_check(&tag, &oc, &or_, dict, ZSTD_dct_auto, &src);
                        }
                    }
                }
            }
        }
    }
}

// ---- direct call of ZSTD_dedicatedDictSearch_lazy_loadDictionary ----------
//
// The function only reads `window.base`, `nextToUpdate`, `hashTable`,
// `chainTable` and `cParams`, so a hand-built `ZSTD_MatchState_t` is enough.
// The layout below mirrors `compress/zstd_compress_internal.h` exactly; the
// static assertions below pin the offsets that the C build reports.

#[repr(C)]
#[derive(Clone, Copy)]
struct TWindow {
    next_src: *const u8,
    base: *const u8,
    dict_base: *const u8,
    dict_limit: u32,
    low_limit: u32,
    nb_overflow_corrections: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct TMatchState {
    window: TWindow,
    loaded_dict_end: u32,
    next_to_update: u32,
    hash_log3: u32,
    row_hash_log: u32,
    tag_table: *mut u8,
    hash_cache: [u32; 8],
    hash_salt: u64,
    hash_salt_entropy: u32,
    hash_table: *mut u32,
    hash_table3: *mut u32,
    chain_table: *mut u32,
    force_non_contiguous: c_int,
    dedicated_dict_search: c_int,
    opt: [u64; 13], // optState_t: 104 bytes, alignment 8
    dict_match_state: *const c_void,
    c_params: ZSTD_compressionParameters,
    ldm_seq_store: *const c_void,
    prefetch_cdict_tables: c_int,
    lazy_skipping: c_int,
}

const _: () = assert!(std::mem::size_of::<TWindow>() == 40);
const _: () = assert!(std::mem::size_of::<TMatchState>() == 304);

#[test]
fn row119b_dedicatedDictSearch_lazy_loadDictionary_direct() {
    unsafe {
        // verify the field offsets the C build reported
        let probe = TMatchState {
            window: TWindow {
                next_src: std::ptr::null(),
                base: std::ptr::null(),
                dict_base: std::ptr::null(),
                dict_limit: 0,
                low_limit: 0,
                nb_overflow_corrections: 0,
            },
            loaded_dict_end: 0,
            next_to_update: 0,
            hash_log3: 0,
            row_hash_log: 0,
            tag_table: std::ptr::null_mut(),
            hash_cache: [0; 8],
            hash_salt: 0,
            hash_salt_entropy: 0,
            hash_table: std::ptr::null_mut(),
            hash_table3: std::ptr::null_mut(),
            chain_table: std::ptr::null_mut(),
            force_non_contiguous: 0,
            dedicated_dict_search: 0,
            opt: [0; 13],
            dict_match_state: std::ptr::null(),
            c_params: ZSTD_compressionParameters::default(),
            ldm_seq_store: std::ptr::null(),
            prefetch_cdict_tables: 0,
            lazy_skipping: 0,
        };
        let b = &probe as *const TMatchState as usize;
        assert_eq!(&probe.window.base as *const _ as usize - b, 8, "window.base offset");
        assert_eq!(&probe.next_to_update as *const _ as usize - b, 44, "nextToUpdate offset");
        assert_eq!(&probe.hash_table as *const _ as usize - b, 112, "hashTable offset");
        assert_eq!(&probe.chain_table as *const _ as usize - b, 128, "chainTable offset");
        assert_eq!(&probe.opt as *const _ as usize - b, 144, "opt offset");
        assert_eq!(&probe.c_params as *const _ as usize - b, 256, "cParams offset");

        let (fc, fr) = duo::<FnDDSLoadDict>("ZSTD_dedicatedDictSearch_lazy_loadDictionary");
        let mut rng = Rng::new(1190);
        // hashLog must exceed chainLog and chainLog <= 24 (DDS invariants).
        for &(hash_log, chain_log, search_log, min_match) in &[
            (12u32, 10u32, 1u32, 3u32),
            (14, 11, 2, 4),
            (16, 13, 4, 5),
            (17, 14, 6, 6),
            (13, 12, 3, 7),
            (15, 14, 8, 3),
            (18, 12, 2, 4),
            (11, 10, 7, 5),
            (19, 16, 5, 6),
        ] {
            for &cls in &[0usize, 1, 2, 3, 4, 5, 6, 7] {
                for &n in &[2048usize, 20_000, 70_000, 150_000] {
                    let data = gen_class(cls, n, rng.next_u64());
                    let target = (n - 8) as u32;
                    let ht_len = 1usize << hash_log;
                    let ct_len = 1usize << chain_log;
                    let mut htc = vec![0u32; ht_len];
                    let mut htr = vec![0u32; ht_len];
                    let mut ctc = vec![0u32; ct_len];
                    let mut ctr = vec![0u32; ct_len];
                    let cp = ZSTD_compressionParameters {
                        windowLog: 20,
                        chainLog: chain_log,
                        hashLog: hash_log,
                        searchLog: search_log,
                        minMatch: min_match,
                        targetLength: 0,
                        strategy: 4,
                    };
                    let mk = |ht: *mut u32, ct: *mut u32| TMatchState {
                        window: TWindow {
                            next_src: data.as_ptr().add(n),
                            base: data.as_ptr(),
                            dict_base: data.as_ptr(),
                            dict_limit: 2,
                            low_limit: 2,
                            nb_overflow_corrections: 0,
                        },
                        loaded_dict_end: 0,
                        next_to_update: 2,
                        hash_log3: 0,
                        row_hash_log: 0,
                        tag_table: std::ptr::null_mut(),
                        hash_cache: [0; 8],
                        hash_salt: 0,
                        hash_salt_entropy: 0,
                        hash_table: ht,
                        hash_table3: std::ptr::null_mut(),
                        chain_table: ct,
                        force_non_contiguous: 0,
                        dedicated_dict_search: 1,
                        opt: [0; 13],
                        dict_match_state: std::ptr::null(),
                        c_params: cp,
                        ldm_seq_store: std::ptr::null(),
                        prefetch_cdict_tables: 0,
                        lazy_skipping: 0,
                    };
                    let mut msc = mk(htc.as_mut_ptr(), ctc.as_mut_ptr());
                    let mut msr = mk(htr.as_mut_ptr(), ctr.as_mut_ptr());
                    let ip = data.as_ptr().add(target as usize);
                    fc(&mut msc as *mut TMatchState as *mut c_void, ip);
                    fr(&mut msr as *mut TMatchState as *mut c_void, ip);
                    let tag = format!(
                        "row119b hl={hash_log} cl={chain_log} sl={search_log} mm={min_match} cls={cls} n={n}"
                    );
                    eqv(&format!("{tag} nextToUpdate"), msc.next_to_update, msr.next_to_update);
                    assert_eq!(msc.next_to_update, target, "{tag}: nextToUpdate not advanced");
                    let bc = std::slice::from_raw_parts(htc.as_ptr() as *const u8, ht_len * 4);
                    let brr = std::slice::from_raw_parts(htr.as_ptr() as *const u8, ht_len * 4);
                    eqbuf(&format!("{tag} hashTable"), bc, brr);
                    let cc2 = std::slice::from_raw_parts(ctc.as_ptr() as *const u8, ct_len * 4);
                    let cr2 = std::slice::from_raw_parts(ctr.as_ptr() as *const u8, ct_len * 4);
                    eqbuf(&format!("{tag} chainTable"), cc2, cr2);
                    // non-vacuity: the DDS structure really was filled in
                    assert!(
                        htc.iter().any(|&v| v != 0),
                        "{tag}: hashTable left empty — the test would be vacuous"
                    );
                }
            }
        }
    }
}

// ------------------------------------------------------------------ extras

#[test]
fn dict_size_estimates_and_sizeof() {
    unsafe {
        let (ec, er) = duo::<FnEstCDictAdv>("ZSTD_estimateCDictSize_advanced");
        let (dc, dr) = duo::<FnEstDDict>("ZSTD_estimateDDictSize");
        let sizes = [0usize, 1, 7, 8, 100, 1024, 10_000, 100_000, 1 << 20];
        for &ds in &sizes {
            for dlm in [ZSTD_dlm_byCopy, ZSTD_dlm_byRef] {
                eqv(
                    &format!("estimateDDictSize({ds},{dlm})"),
                    dc(ds, dlm),
                    dr(ds, dlm),
                );
                for &lvl in &[-5, 1, 3, 9, 19, 22] {
                    let cp = get_cparams(lvl, 0, ds);
                    eqv(
                        &format!("estimateCDictSize_advanced({ds},lvl={lvl},{dlm})"),
                        ec(ds, cp, dlm),
                        er(ds, cp, dlm),
                    );
                }
            }
        }
        // sizeof_CDict / sizeof_DDict must agree for every construction path
        for (dn, dict) in dict_shapes() {
            for &lvl in &[1, 6, 19] {
                let _big = maybe_big(dict.len(), lvl);
                let cd = cdict_pair(&format!("sizeof {dn} lvl={lvl}"), &dict, lvl);
                check_cdict_getters(&format!("sizeof {dn} lvl={lvl}"), &cd);
            }
            let dd = ddict_pair(&format!("sizeof ddict {dn}"), &dict);
            check_ddict_getters(&format!("sizeof ddict {dn}"), &dd);
        }
    }
}

#[test]
fn prefetchCDictTables_axis() {
    unsafe {
        let (sc, sr) = duo::<FnSetParam>("ZSTD_CCtx_setParameter");
        let (rc, rr) = duo::<FnRefObj>("ZSTD_CCtx_refCDict");
        let dict = trained_big().to_vec();
        let mut rng = Rng::new(47);
        for pf in [ZSTD_ps_auto, ZSTD_ps_enable, ZSTD_ps_disable] {
            for attach in [0, 1, 2, 3] {
                for &lvl in &[1, 6, 12, 19] {
                    let cd = cdict_pair(&format!("prefetch lvl={lvl}"), &dict, lvl);
                    assert!(cd.ok());
                    let cls = rng.below(N_CLASSES);
                    let sz = [50usize, 9_000, 200_000][rng.below(3)];
                    let src = gen_class(cls, sz, rng.next_u64());
                    let cctx = CtxPair::cctx();
                    let a = sc(cctx.c, ZSTD_c_prefetchCDictTables, pf);
                    let b = sr(cctx.r, ZSTD_c_prefetchCDictTables, pf);
                    eqv("prefetchCDictTables setParameter", a, b);
                    assert!(!is_err(a), "prefetchCDictTables={pf} rejected");
                    let a = sc(cctx.c, ZSTD_c_forceAttachDict, attach);
                    let b = sr(cctx.r, ZSTD_c_forceAttachDict, attach);
                    eqv("forceAttachDict setParameter", a, b);
                    let x = rc(cctx.c, cd.c);
                    let y = rr(cctx.r, cd.r);
                    eqv("prefetch refCDict", x, y);
                    let tag = format!("prefetch pf={pf} attach={attach} lvl={lvl} cls={cls} sz={sz}");
                    let (n, oc, or_) = compress2_pair(&tag, &cctx, &src);
                    assert!(!is_err(n), "{tag}: compression failed");
                    rt_check(&tag, &oc, &or_, &dict, ZSTD_dct_auto, &src);
                }
            }
        }
    }
}
