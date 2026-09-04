//! Phase B — CONFIGS.md rows 82..96: the bufferless streaming API and the
//! block-level API.
//!
//! Every entry point is resolved with `dlsym` in **both** shared libraries and
//! called in lockstep; after each call pair the return value *and* the whole
//! destination buffer are compared byte-for-byte.
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

mod common;
use common::*;
use std::ffi::{c_int, c_uint, c_ulonglong, c_void};
use std::sync::OnceLock;

// ---------------------------------------------------------------- fn types

type FnBegin = unsafe extern "C" fn(*mut c_void, c_int) -> usize;
type FnBeginDict = unsafe extern "C" fn(*mut c_void, *const c_void, usize, c_int) -> usize;
type FnBeginAdv =
    unsafe extern "C" fn(*mut c_void, *const c_void, usize, ZSTD_parameters, c_ulonglong) -> usize;
type FnBeginAdvInt = unsafe extern "C" fn(
    *mut c_void,
    *const c_void,
    usize,
    c_int,
    c_int,
    *const c_void,
    *const c_void,
    c_ulonglong,
) -> usize;
type FnBeginCDict = unsafe extern "C" fn(*mut c_void, *const c_void) -> usize;
type FnBeginCDictAdv =
    unsafe extern "C" fn(*mut c_void, *const c_void, ZSTD_frameParameters, c_ulonglong) -> usize;
/// (ctx, dst, dstCapacity, src, srcSize) -> size_t : compressContinue/End/Block,
/// decompressContinue/Block, compress2 …
type FnBlk = unsafe extern "C" fn(*mut c_void, *mut c_void, usize, *const c_void, usize) -> usize;
type FnCopyCCtx = unsafe extern "C" fn(*mut c_void, *const c_void, c_ulonglong) -> usize;
type FnCtx2Size = unsafe extern "C" fn(*mut c_void) -> usize;
type FnCtxConst2Size = unsafe extern "C" fn(*const c_void) -> usize;
type FnVoidCtx = unsafe extern "C" fn(*mut c_void);
type FnCopyDCtx = unsafe extern "C" fn(*mut c_void, *const c_void);
type FnBeginUsingDict = unsafe extern "C" fn(*mut c_void, *const c_void, usize) -> usize;
type FnBeginUsingDDict = unsafe extern "C" fn(*mut c_void, *const c_void) -> usize;
type FnInsertBlock = unsafe extern "C" fn(*mut c_void, *const c_void, usize) -> usize;
type FnCheckCont = unsafe extern "C" fn(*mut c_void, *const c_void, usize);
type FnNextType = unsafe extern "C" fn(*mut c_void) -> c_int;
type FnGetcBlockSize = unsafe extern "C" fn(*const c_void, usize, *mut BlockProps) -> usize;
type FnWriteLastEmpty = unsafe extern "C" fn(*mut c_void, usize) -> usize;
type FnGetSeqStore = unsafe extern "C" fn(*const c_void) -> *const SeqStoreRaw;
type FnResetSeqStore = unsafe extern "C" fn(*mut SeqStoreRaw);
type FnSplitBlock =
    unsafe extern "C" fn(*const c_void, usize, c_int, *mut c_void, usize) -> usize;
type FnBlockSummary = unsafe extern "C" fn(*const ZSTD_Sequence, usize) -> BlockSummary;
type FnDecompBlkInt =
    unsafe extern "C" fn(*mut c_void, *mut c_void, usize, *const c_void, usize, c_int) -> usize;
type FnDecLits =
    unsafe extern "C" fn(*mut c_void, *const c_void, usize, *mut c_void, usize) -> usize;
type FnDecSeqHdr = unsafe extern "C" fn(*mut c_void, *mut c_int, *const c_void, usize) -> usize;
type FnCreateCDict = unsafe extern "C" fn(*const c_void, usize, c_int) -> *mut c_void;
type FnCreateCDictAdv = unsafe extern "C" fn(
    *const c_void,
    usize,
    c_int,
    c_int,
    ZSTD_compressionParameters,
    ZSTD_customMem,
) -> *mut c_void;
type FnCreateDDict = unsafe extern "C" fn(*const c_void, usize) -> *mut c_void;
type FnCreateDDictAdv =
    unsafe extern "C" fn(*const c_void, usize, c_int, c_int, ZSTD_customMem) -> *mut c_void;
type FnGetParams = unsafe extern "C" fn(c_int, c_ulonglong, usize) -> ZSTD_parameters;
type FnParamsInitAdv = unsafe extern "C" fn(*mut c_void, ZSTD_parameters) -> usize;
type FnWriteSkippable =
    unsafe extern "C" fn(*mut c_void, usize, *const c_void, usize, c_uint) -> usize;
type FnTrain =
    unsafe extern "C" fn(*mut c_void, usize, *const c_void, *const usize, c_uint) -> usize;

// ---------------------------------------------------------------- structs
//
// Layouts taken verbatim from the C headers:
//   blockProperties_t  — c_src/src/common/zstd_internal.h:297
//   SeqStore_t         — c_src/src/compress/zstd_compress_internal.h:98
//   SeqDef             — c_src/src/compress/zstd_compress_internal.h:85
//   BlockSummary       — c_src/src/compress/zstd_compress_internal.h:1528

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
struct BlockProps {
    blockType: c_int,
    lastBlock: c_uint,
    origSize: c_uint,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
struct SeqDef {
    offBase: u32,
    litLength: u16,
    mlBase: u16,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct SeqStoreRaw {
    sequencesStart: *mut SeqDef,
    sequences: *mut SeqDef,
    litStart: *mut u8,
    lit: *mut u8,
    llCode: *mut u8,
    mlCode: *mut u8,
    ofCode: *mut u8,
    maxNbSeq: usize,
    maxNbLit: usize,
    longLengthType: c_int,
    longLengthPos: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
struct BlockSummary {
    nbSequences: usize,
    blockSize: usize,
    litSize: usize,
}

/// `sizeof(ZSTD_compressedBlockState_t)` — verified against the C build
/// (huf 2064 + fse 3552 + rep 3*4 + padding = 5632).
const SIZEOF_CBLOCKSTATE: usize = 5632;
/// `ZSTD_SLIPBLOCK_WORKSPACESIZE` (c_src/src/compress/zstd_preSplit.h)
const SLIPBLOCK_WKSP: usize = 8208;

const ZSTD_dtlm_fast: c_int = 0;
const ZSTD_dtlm_full: c_int = 1;

// nextInputType enum
const ZSTDnit_frameHeader: c_int = 0;
const ZSTDnit_blockHeader: c_int = 1;
const ZSTDnit_block: c_int = 2;
const ZSTDnit_lastBlock: c_int = 3;
const ZSTDnit_checksum: c_int = 4;
const ZSTDnit_skippableFrame: c_int = 5;

// ---------------------------------------------------------------- small helpers

/// A pair of `ZSTD_CDict*` / `ZSTD_DDict*` (one per library).
struct DictPair {
    c: *mut c_void,
    r: *mut c_void,
    free_c: FnFreePtr,
    free_r: FnFreePtr,
}

impl DictPair {
    unsafe fn cdict(dict: &[u8], level: c_int) -> DictPair {
        let (cc, cr) = duo::<FnCreateCDict>("ZSTD_createCDict");
        let (fc, fr) = duo::<FnFreePtr>("ZSTD_freeCDict");
        let p = dict.as_ptr() as *const c_void;
        let c = cc(p, dict.len(), level);
        let r = cr(p, dict.len(), level);
        assert!(!c.is_null() && !r.is_null(), "createCDict returned NULL");
        DictPair { c, r, free_c: fc, free_r: fr }
    }
    unsafe fn cdict_advanced(
        dict: &[u8],
        dlm: c_int,
        dct: c_int,
        cp: ZSTD_compressionParameters,
    ) -> DictPair {
        let (cc, cr) = duo::<FnCreateCDictAdv>("ZSTD_createCDict_advanced");
        let (fc, fr) = duo::<FnFreePtr>("ZSTD_freeCDict");
        let p = dict.as_ptr() as *const c_void;
        let m = ZSTD_customMem::default();
        let c = cc(p, dict.len(), dlm, dct, cp, m);
        let r = cr(p, dict.len(), dlm, dct, cp, m);
        assert!(!c.is_null() && !r.is_null(), "createCDict_advanced returned NULL");
        DictPair { c, r, free_c: fc, free_r: fr }
    }
    unsafe fn ddict(dict: &[u8]) -> DictPair {
        let (cc, cr) = duo::<FnCreateDDict>("ZSTD_createDDict");
        let (fc, fr) = duo::<FnFreePtr>("ZSTD_freeDDict");
        let p = dict.as_ptr() as *const c_void;
        let c = cc(p, dict.len());
        let r = cr(p, dict.len());
        assert!(!c.is_null() && !r.is_null(), "createDDict returned NULL");
        DictPair { c, r, free_c: fc, free_r: fr }
    }
    unsafe fn ddict_advanced(dict: &[u8], dlm: c_int, dct: c_int) -> DictPair {
        let (cc, cr) = duo::<FnCreateDDictAdv>("ZSTD_createDDict_advanced");
        let (fc, fr) = duo::<FnFreePtr>("ZSTD_freeDDict");
        let p = dict.as_ptr() as *const c_void;
        let m = ZSTD_customMem::default();
        let c = cc(p, dict.len(), dlm, dct, m);
        let r = cr(p, dict.len(), dlm, dct, m);
        assert!(!c.is_null() && !r.is_null(), "createDDict_advanced returned NULL");
        DictPair { c, r, free_c: fc, free_r: fr }
    }
}

impl Drop for DictPair {
    fn drop(&mut self) {
        unsafe {
            (self.free_c)(self.c);
            (self.free_r)(self.r);
        }
    }
}

unsafe fn errname(rc: usize) -> String {
    let (f, _) = duo::<FnErrName>("ZSTD_getErrorName");
    cstr(f(rc))
}

unsafe fn bound(n: usize) -> usize {
    let (b, _) = duo::<FnSizeT1>("ZSTD_compressBound");
    b(n)
}

/// A raw-content dictionary and a real trained dictionary, built once.
fn raw_dict(size: usize) -> Vec<u8> {
    // A mixture of text-like and long-range material: makes a useful prefix.
    let mut v = gen_class(4, size / 2, 0xD1C7);
    v.extend_from_slice(&gen_class(5, size - size / 2, 0xD1C8));
    v
}

fn trained_dict() -> &'static Vec<u8> {
    static D: OnceLock<Vec<u8>> = OnceLock::new();
    D.get_or_init(|| unsafe {
        let (train, _) = duo::<FnTrain>("ZDICT_trainFromBuffer");
        const NB: usize = 256;
        const SS: usize = 2048;
        let mut samples = Vec::with_capacity(NB * SS);
        for i in 0..NB {
            samples.extend_from_slice(&gen_class(4, SS, 0x5EED_0000 + i as u64));
        }
        let sizes = [SS; NB];
        let mut dict = vec![0u8; 8 * 1024];
        let n = train(
            dict.as_mut_ptr() as *mut c_void,
            dict.len(),
            samples.as_ptr() as *const c_void,
            sizes.as_ptr(),
            NB as c_uint,
        );
        assert!(!is_err(n), "ZDICT_trainFromBuffer failed");
        dict.truncate(n);
        dict
    })
}

/// Compress with the *C* library (ground-truth frame producer).
unsafe fn c_compress_with(src: &[u8], opts: &[(c_int, c_int)]) -> Vec<u8> {
    let (create, _) = duo::<FnPtr0>("ZSTD_createCCtx");
    let (free, _) = duo::<FnFreePtr>("ZSTD_freeCCtx");
    let (setp, _) = duo::<FnSetParam>("ZSTD_CCtx_setParameter");
    let (c2, _) = duo::<FnBlk>("ZSTD_compress2");
    let cctx = create();
    for &(p, v) in opts {
        let rc = setp(cctx, p, v);
        assert!(!is_err(rc), "ZSTD_CCtx_setParameter({p},{v}) failed");
    }
    let cap = bound(src.len()) + 64;
    let mut dst = vec![0u8; cap];
    let n = c2(
        cctx,
        dst.as_mut_ptr() as *mut c_void,
        cap,
        src.as_ptr() as *const c_void,
        src.len(),
    );
    free(cctx);
    assert!(!is_err(n), "c_compress_with failed");
    dst.truncate(n);
    dst
}

/// Split `n` bytes into 1..6 chunks (possibly containing empty chunks).
fn chunks_for(rng: &mut Rng, n: usize) -> Vec<usize> {
    if n == 0 {
        return vec![0];
    }
    let k = 1 + rng.below(6);
    if k == 1 {
        return vec![n];
    }
    let mut cuts: Vec<usize> = Vec::with_capacity(k - 1);
    for _ in 0..k - 1 {
        cuts.push(rng.below(n + 1));
    }
    cuts.sort_unstable();
    let mut v = Vec::with_capacity(k);
    let mut prev = 0usize;
    for c in cuts {
        v.push(c - prev);
        prev = c;
    }
    v.push(n - prev);
    v
}

/// Run `continue…continue,end` in lockstep over both libraries.
///
/// Returns the produced frame, or `None` when both libraries agreed on an error.
unsafe fn run_session(
    label: &str,
    ctx: &CtxPair,
    cont_name: &str,
    end_name: &str,
    src: &[u8],
    chunks: &[usize],
) -> Option<Vec<u8>> {
    let (cc, cr) = duo::<FnBlk>(cont_name);
    let (ec, er) = duo::<FnBlk>(end_name);
    let mut out: Vec<u8> = Vec::new();
    let mut off = 0usize;
    for (i, &len) in chunks.iter().enumerate() {
        let last = i + 1 == chunks.len();
        let cap = bound(len) + 512;
        let mut dc = vec![0xA5u8; cap];
        let mut dr = vec![0xA5u8; cap];
        let sp = src.as_ptr().add(off) as *const c_void;
        let (fc, fr) = if last { (ec, er) } else { (cc, cr) };
        let rc = fc(ctx.c, dc.as_mut_ptr() as *mut c_void, cap, sp, len);
        let rr = fr(ctx.r, dr.as_mut_ptr() as *mut c_void, cap, sp, len);
        eqv(&format!("{label} chunk{i} ret"), rc, rr);
        eqbuf(&format!("{label} chunk{i} dst"), &dc, &dr);
        if is_err(rc) {
            return None;
        }
        assert!(rc <= cap, "{label}: return {rc} > capacity {cap}");
        out.extend_from_slice(&dc[..rc]);
        off += len;
    }
    Some(out)
}

/// Decompress `frame` with both libraries' one-shot API and check it equals
/// `orig` — proves the bufferless stream we just built is a real frame.
unsafe fn check_roundtrip(label: &str, frame: &[u8], orig: &[u8]) {
    let (dc, dr) = duo::<FnDecompress>("ZSTD_decompress");
    let cap = orig.len() + 16;
    let mut oc = vec![0x5Au8; cap];
    let mut or = vec![0x5Au8; cap];
    let rc = dc(
        oc.as_mut_ptr() as *mut c_void,
        cap,
        frame.as_ptr() as *const c_void,
        frame.len(),
    );
    let rr = dr(
        or.as_mut_ptr() as *mut c_void,
        cap,
        frame.as_ptr() as *const c_void,
        frame.len(),
    );
    eqv(&format!("{label} roundtrip ret"), rc, rr);
    eqbuf(&format!("{label} roundtrip out"), &oc, &or);
    assert!(!is_err(rc), "{label}: frame does not decode");
    eqv(&format!("{label} roundtrip size"), rc, orig.len());
    eqbuf(&format!("{label} roundtrip content"), orig, &oc[..rc]);
}

/// Full bufferless decode of (possibly several) frames.
///
/// `partial_raw`: when set, first probe each block with a partial `srcSize`
/// (only legal for raw blocks — the library must reject it for every other
/// block type, identically in both builds).
unsafe fn bufferless_decode(
    label: &str,
    dctx: &CtxPair,
    frame: &[u8],
    out_cap: usize,
    partial_raw: bool,
) -> Option<Vec<u8>> {
    let (bc, br) = duo::<FnCtx2Size>("ZSTD_decompressBegin");
    let (nc, nr) = duo::<FnCtx2Size>("ZSTD_nextSrcSizeToDecompress");
    let (tc, tr) = duo::<FnNextType>("ZSTD_nextInputType");
    let (kc, kr) = duo::<FnBlk>("ZSTD_decompressContinue");

    let cap = out_cap.max(1);
    let mut oc = vec![0xA5u8; cap];
    let mut or = vec![0xA5u8; cap];
    let mut ip = 0usize;
    let mut op = 0usize;
    let mut frames = 0usize;

    while ip < frame.len() {
        eqv(&format!("{label} f{frames} decompressBegin"), bc(dctx.c), br(dctx.r));
        let mut step = 0usize;
        loop {
            let en_c = nc(dctx.c);
            let en_r = nr(dctx.r);
            eqv(&format!("{label} f{frames}s{step} nextSrcSize"), en_c, en_r);
            let ty_c = tc(dctx.c);
            let ty_r = tr(dctx.r);
            eqv(&format!("{label} f{frames}s{step} nextInputType"), ty_c, ty_r);
            assert!(
                (ZSTDnit_frameHeader..=ZSTDnit_skippableFrame).contains(&ty_c),
                "{label}: bogus nextInputType {ty_c}"
            );
            if ty_c == ZSTDnit_checksum {
                assert_eq!(en_c, 4, "{label}: checksum stage must want 4 bytes");
            }
            if en_c == 0 {
                break;
            }
            assert!(!is_err(en_c), "{label}: nextSrcSizeToDecompress is an error");
            assert!(
                ip + en_c <= frame.len(),
                "{label}: frame wants {en_c} bytes but only {} remain",
                frame.len() - ip
            );
            let sp = frame.as_ptr().add(ip) as *const c_void;
            let mut take = en_c;
            if partial_raw
                && en_c > 1
                && (ty_c == ZSTDnit_block || ty_c == ZSTDnit_lastBlock)
                && cap - op >= en_c
            {
                // probe: legal only for raw (uncompressed) blocks
                let half = en_c / 2;
                let pc = kc(dctx.c, oc.as_mut_ptr().add(op) as *mut c_void, cap - op, sp, half);
                let pr = kr(dctx.r, or.as_mut_ptr().add(op) as *mut c_void, cap - op, sp, half);
                eqv(&format!("{label} f{frames}s{step} partial ret"), pc, pr);
                if !is_err(pc) {
                    eqbuf(
                        &format!("{label} f{frames}s{step} partial out"),
                        &oc[..op + pc],
                        &or[..op + pc],
                    );
                }
                if !is_err(pc) {
                    // raw block, streamed: advance and continue with the rest
                    ip += half;
                    op += pc;
                    step += 1;
                    continue;
                }
                take = en_c;
            }
            let avail = cap - op;
            let rc = kc(dctx.c, oc.as_mut_ptr().add(op) as *mut c_void, avail, sp, take);
            let rr = kr(dctx.r, or.as_mut_ptr().add(op) as *mut c_void, avail, sp, take);
            eqv(&format!("{label} f{frames}s{step} decompressContinue"), rc, rr);
            if is_err(rc) {
                return None;
            }
            // Only the regenerated output is defined; the tail of `dst` is
            // scratch space for the literals section plus wildcopy slop.
            eqbuf(
                &format!("{label} f{frames}s{step} out"),
                &oc[..op + rc],
                &or[..op + rc],
            );
            assert!(rc <= avail, "{label}: produced {rc} > capacity {avail}");
            ip += take;
            op += rc;
            step += 1;
            assert!(step < 200_000, "{label}: bufferless decode does not terminate");
        }
        frames += 1;
        assert!(frames < 64, "{label}: too many frames");
        if step == 0 {
            break; // no progress — avoid an infinite outer loop
        }
    }
    oc.truncate(op);
    Some(oc)
}

// ================================================================ row 82

#[test]
fn row82_compress_begin_continue_end() {
    unsafe {
        let (bc, br) = duo::<FnBegin>("ZSTD_compressBegin");
        let mut rng = Rng::new(0x8200_0001);
        let levels: Vec<c_int> = (-5..=19).collect();
        let sizes: [usize; 9] = [
            0,
            1,
            7,
            128,
            1024,
            8 * 1024,
            64 * 1024,
            128 * 1024 - 1,
            128 * 1024 + 1,
        ];

        // full level × class × size grid on the small ladder
        for &lvl in levels.iter() {
            for class in 0..N_CLASSES {
                for (si, &size) in sizes.iter().enumerate() {
                    if size > 64 * 1024 && (lvl > 9 || class % 3 != si % 3) {
                        continue;
                    }
                    let src = gen_class(class, size, 0x82 ^ (si as u64));
                    let ctx = CtxPair::cctx();
                    let label =
                        format!("row82 lvl={lvl} class={} size={size}", CLASS_NAMES[class]);
                    eqv(&format!("{label} compressBegin"), bc(ctx.c, lvl), br(ctx.r, lvl));
                    let chunks = chunks_for(&mut rng, src.len());
                    if let Some(frame) = run_session(
                        &label,
                        &ctx,
                        "ZSTD_compressContinue",
                        "ZSTD_compressEnd",
                        &src,
                        &chunks,
                    ) {
                        check_roundtrip(&label, &frame, &src);
                    }
                }
            }
        }

        // ultra levels: `ZSTD_compressBegin` pledges an unknown size, so each
        // context costs ~705 MB at level 22 — create and free one pair at a
        // time (the process has a 6 GB RLIMIT_DATA).
        for &lvl in [20, 21, 22].iter() {
            for (i, &size) in [0usize, 1, 1000, 70_000].iter().enumerate() {
                let class = (i + lvl as usize) % N_CLASSES;
                let src = gen_class(class, size, 0x8280);
                let label = format!("row82ultra lvl={lvl} class={} size={size}", CLASS_NAMES[class]);
                let frame = {
                    let ctx = CtxPair::cctx();
                    eqv(&format!("{label} compressBegin"), bc(ctx.c, lvl), br(ctx.r, lvl));
                    let chunks = chunks_for(&mut rng, src.len());
                    run_session(
                        &label,
                        &ctx,
                        "ZSTD_compressContinue",
                        "ZSTD_compressEnd",
                        &src,
                        &chunks,
                    )
                };
                if let Some(frame) = frame {
                    check_roundtrip(&label, &frame, &src);
                }
            }
        }

        // the 128KB boundary and beyond, context reused
        let big: [usize; 5] = [128 * 1024, 128 * 1024 + 1, 200_000, 256 * 1024, 1024 * 1024];
        for &lvl in [1, 3, 6, 9, 19].iter() {
            let ctx = CtxPair::cctx();
            for (i, &size) in big.iter().enumerate() {
                if size > 256 * 1024 && lvl > 9 {
                    continue;
                }
                let class = (i + lvl as usize) % N_CLASSES;
                let src = gen_class(class, size, 0x8282);
                let label = format!("row82big lvl={lvl} class={} size={size}", CLASS_NAMES[class]);
                eqv(&format!("{label} compressBegin"), bc(ctx.c, lvl), br(ctx.r, lvl));
                let chunks = chunks_for(&mut rng, src.len());
                if let Some(frame) = run_session(
                    &label,
                    &ctx,
                    "ZSTD_compressContinue",
                    "ZSTD_compressEnd",
                    &src,
                    &chunks,
                ) {
                    check_roundtrip(&label, &frame, &src);
                }
            }
        }
    }
}

// ================================================================ row 85

#[test]
fn row85_compress_continue_end_public() {
    unsafe {
        let (bc, br) = duo::<FnBegin>("ZSTD_compressBegin");
        let mut rng = Rng::new(0x8500_0001);
        for &lvl in [-3, -1, 0, 1, 3, 6, 11, 16, 19].iter() {
            for class in 0..N_CLASSES {
                for &size in [0usize, 1, 333, 9_000, 70_000, 140_000].iter() {
                    if size > 70_000 && lvl > 16 {
                        continue;
                    }
                    let src = gen_class(class, size, 0x85);
                    let ctx = CtxPair::cctx();
                    let label = format!(
                        "row85 lvl={lvl} class={} size={size}",
                        CLASS_NAMES[class]
                    );
                    eqv(&format!("{label} compressBegin"), bc(ctx.c, lvl), br(ctx.r, lvl));
                    let chunks = chunks_for(&mut rng, src.len());
                    if let Some(frame) = run_session(
                        &label,
                        &ctx,
                        "ZSTD_compressContinue_public",
                        "ZSTD_compressEnd_public",
                        &src,
                        &chunks,
                    ) {
                        check_roundtrip(&label, &frame, &src);
                    }
                }
            }
        }
    }
}

// ================================================================ row 83

#[test]
fn row83_compress_begin_advanced() {
    unsafe {
        let (gpc, gpr) = duo::<FnGetParams>("ZSTD_getParams");
        let (bc, br) = duo::<FnBeginAdv>("ZSTD_compressBegin_advanced");
        let mut rng = Rng::new(0x8300_0001);
        let dict_raw = raw_dict(4096);
        let dict_trained = trained_dict().clone();

        for &lvl in [-5, -1, 0, 1, 3, 8, 13, 19, 22].iter() {
            for &strategy in ALL_STRATEGIES.iter() {
                let size = [0usize, 1, 200, 4096, 40_000, 140_000][rng.below(6)];
                let class = rng.below(N_CLASSES);
                let src = gen_class(class, size, 0x83 ^ (strategy as u64));
                // dict choice: none / raw / trained
                let which = rng.below(3);
                let dict: &[u8] = match which {
                    0 => &[],
                    1 => &dict_raw,
                    _ => &dict_trained,
                };
                let pc = gpc(lvl, src.len() as c_ulonglong, dict.len());
                let pr = gpr(lvl, src.len() as c_ulonglong, dict.len());
                eqv("row83 ZSTD_getParams", pc, pr);
                let mut params = pc;
                params.cParams.strategy = strategy as c_uint;
                params.fParams.contentSizeFlag = (rng.below(2)) as c_int;
                params.fParams.checksumFlag = (rng.below(2)) as c_int;
                params.fParams.noDictIDFlag = (rng.below(2)) as c_int;
                let pledged = if rng.below(2) == 0 {
                    src.len() as c_ulonglong
                } else {
                    ZSTD_CONTENTSIZE_UNKNOWN
                };
                let ctx = CtxPair::cctx();
                let label = format!(
                    "row83 lvl={lvl} strat={strategy} which={which} size={size} class={}",
                    CLASS_NAMES[class]
                );
                let dp = if dict.is_empty() {
                    std::ptr::null()
                } else {
                    dict.as_ptr() as *const c_void
                };
                eqv(
                    &format!("{label} compressBegin_advanced"),
                    bc(ctx.c, dp, dict.len(), params, pledged),
                    br(ctx.r, dp, dict.len(), params, pledged),
                );
                let chunks = chunks_for(&mut rng, src.len());
                if let Some(frame) = run_session(
                    &label,
                    &ctx,
                    "ZSTD_compressContinue",
                    "ZSTD_compressEnd",
                    &src,
                    &chunks,
                ) {
                    // frames built with a dictionary need the dictionary to decode
                    if dict.is_empty() {
                        check_roundtrip(&label, &frame, &src);
                    }
                }
            }
        }
    }
}

#[test]
fn row83b_compress_begin_advanced_internal() {
    unsafe {
        let (gpc, gpr) = duo::<FnGetParams>("ZSTD_getParams");
        let (iac, iar) = duo::<FnParamsInitAdv>("ZSTD_CCtxParams_init_advanced");
        let (spc, spr) = duo::<FnSetParam>("ZSTD_CCtxParams_setParameter");
        let (bc, br) = duo::<FnBeginAdvInt>("ZSTD_compressBegin_advanced_internal");
        let mut rng = Rng::new(0x8300_0002);
        let dict_raw = raw_dict(6000);
        let dict_trained = trained_dict().clone();

        for &lvl in [-2, 0, 1, 4, 9, 14, 19].iter() {
            for &strategy in ALL_STRATEGIES.iter() {
                for &dct in [ZSTD_dct_auto, ZSTD_dct_rawContent, ZSTD_dct_fullDict].iter() {
                    let dtlm = if rng.below(2) == 0 { ZSTD_dtlm_fast } else { ZSTD_dtlm_full };
                    let size = [0usize, 5, 700, 20_000, 132_000][rng.below(5)];
                    let class = rng.below(N_CLASSES);
                    let src = gen_class(class, size, 0x8300 ^ (strategy as u64));
                    // 0 = no dict, 1 = raw bytes, 2 = trained dict, 3 = cdict
                    let which = rng.below(4);
                    let dict: &[u8] = match which {
                        0 => &[],
                        1 => &dict_raw,
                        _ => &dict_trained,
                    };
                    if which == 2 && dct == ZSTD_dct_fullDict {
                        // fine: trained dict really is a full dict
                    }
                    if which == 1 && dct == ZSTD_dct_fullDict {
                        continue; // raw bytes are not a valid full dict (ERRORS.md territory)
                    }
                    let params = CtxPair::cctx_params();
                    let mut p = gpc(lvl, src.len() as c_ulonglong, dict.len());
                    eqv(
                        "row83b ZSTD_getParams",
                        p,
                        gpr(lvl, src.len() as c_ulonglong, dict.len()),
                    );
                    p.cParams.strategy = strategy as c_uint;
                    p.fParams.contentSizeFlag = rng.below(2) as c_int;
                    p.fParams.checksumFlag = rng.below(2) as c_int;
                    p.fParams.noDictIDFlag = rng.below(2) as c_int;
                    eqv(
                        "row83b CCtxParams_init_advanced",
                        iac(params.c, p),
                        iar(params.r, p),
                    );
                    // extra knobs that only reach the bufferless path through
                    // a ZSTD_CCtx_params object
                    let extra: [(c_int, c_int); 3] = [
                        (ZSTD_c_blockSplitterLevel, rng.range(0, 6)),
                        (ZSTD_c_literalCompressionMode, rng.range(0, 2)),
                        (ZSTD_c_targetCBlockSize, [0, 1340, 8192, 131072][rng.below(4)]),
                    ];
                    for &(k, v) in extra.iter() {
                        eqv(
                            &format!("row83b CCtxParams_setParameter {k}={v}"),
                            spc(params.c, k, v),
                            spr(params.r, k, v),
                        );
                    }
                    let cdict = if which == 3 {
                        Some(DictPair::cdict(&dict_trained, if lvl <= 0 { 1 } else { lvl }))
                    } else {
                        None
                    };
                    let (dp, dlen, cdc, cdr) = match &cdict {
                        Some(cd) => (std::ptr::null(), 0usize, cd.c as *const c_void, cd.r as *const c_void),
                        None => (
                            if dict.is_empty() {
                                std::ptr::null()
                            } else {
                                dict.as_ptr() as *const c_void
                            },
                            dict.len(),
                            std::ptr::null(),
                            std::ptr::null(),
                        ),
                    };
                    let pledged = if rng.below(2) == 0 {
                        src.len() as c_ulonglong
                    } else {
                        ZSTD_CONTENTSIZE_UNKNOWN
                    };
                    let ctx = CtxPair::cctx();
                    let label = format!(
                        "row83b lvl={lvl} strat={strategy} dct={dct} dtlm={dtlm} which={which} size={size}"
                    );
                    eqv(
                        &format!("{label} begin_advanced_internal"),
                        bc(ctx.c, dp, dlen, dct, dtlm, cdc, params.c as *const c_void, pledged),
                        br(ctx.r, dp, dlen, dct, dtlm, cdr, params.r as *const c_void, pledged),
                    );
                    let chunks = chunks_for(&mut rng, src.len());
                    if let Some(frame) = run_session(
                        &label,
                        &ctx,
                        "ZSTD_compressContinue",
                        "ZSTD_compressEnd",
                        &src,
                        &chunks,
                    ) {
                        if which == 0 {
                            check_roundtrip(&label, &frame, &src);
                        }
                    }
                }
            }
        }
    }
}

// ================================================================ row 84

#[test]
fn row84_compress_begin_using_dict_and_cdict() {
    unsafe {
        let (bdc, bdr) = duo::<FnBeginDict>("ZSTD_compressBegin_usingDict");
        let (bcc, bcr) = duo::<FnBeginCDict>("ZSTD_compressBegin_usingCDict");
        let (bpc, bpr) = duo::<FnBeginCDict>("ZSTD_compressBegin_usingCDict_deprecated");
        let (bac, bar) = duo::<FnBeginCDictAdv>("ZSTD_compressBegin_usingCDict_advanced");
        let (gcc, gcr) = duo::<
            unsafe extern "C" fn(c_int, c_ulonglong, usize) -> ZSTD_compressionParameters,
        >("ZSTD_getCParams");
        let mut rng = Rng::new(0x8400_0001);

        let dicts: Vec<(&str, Vec<u8>)> = vec![
            ("empty", Vec::new()),
            ("tiny", vec![0x11, 0x22, 0x33]),
            ("raw256", gen_class(3, 256, 0x84)),
            ("raw4k", raw_dict(4096)),
            ("raw64k", raw_dict(64 * 1024)),
            ("trained", trained_dict().clone()),
        ];
        let levels: [c_int; 10] = [-5, -1, 0, 1, 2, 3, 6, 10, 16, 19];

        // --- ZSTD_compressBegin_usingDict -------------------------------
        for (dn, dict) in dicts.iter() {
            for &lvl in levels.iter() {
                let class = rng.below(N_CLASSES);
                let size = [0usize, 3, 512, 12_000, 150_000][rng.below(5)];
                let src = gen_class(class, size, 0x8401);
                let ctx = CtxPair::cctx();
                let label = format!("row84 usingDict {dn} lvl={lvl} size={size}");
                let dp = if dict.is_empty() {
                    std::ptr::null()
                } else {
                    dict.as_ptr() as *const c_void
                };
                eqv(
                    &format!("{label} begin"),
                    bdc(ctx.c, dp, dict.len(), lvl),
                    bdr(ctx.r, dp, dict.len(), lvl),
                );
                let chunks = chunks_for(&mut rng, src.len());
                if let Some(frame) = run_session(
                    &label,
                    &ctx,
                    "ZSTD_compressContinue",
                    "ZSTD_compressEnd",
                    &src,
                    &chunks,
                ) {
                    if dict.is_empty() {
                        check_roundtrip(&label, &frame, &src);
                    }
                }
            }
        }

        // --- CDict variants ---------------------------------------------
        // NULL cdict must be rejected identically by both builds.
        {
            let ctx = CtxPair::cctx();
            eqv(
                "row84 usingCDict(NULL)",
                bcc(ctx.c, std::ptr::null()),
                bcr(ctx.r, std::ptr::null()),
            );
        }

        for (dn, dict) in dicts.iter().filter(|(_, d)| !d.is_empty()) {
            for &lvl in [1, 3, 6, 12, 19].iter() {
                let cd = DictPair::cdict(dict, lvl);
                let class = rng.below(N_CLASSES);
                let size = [0usize, 9, 900, 30_000, 140_000][rng.below(5)];
                let src = gen_class(class, size, 0x8402);
                let chunks = chunks_for(&mut rng, src.len());

                for &(nm, kind) in [("usingCDict", 0), ("usingCDict_deprecated", 1)].iter() {
                    let ctx = CtxPair::cctx();
                    let label = format!("row84 {nm} {dn} lvl={lvl} size={size}");
                    let (a, b) = if kind == 0 {
                        (bcc(ctx.c, cd.c), bcr(ctx.r, cd.r))
                    } else {
                        (bpc(ctx.c, cd.c), bpr(ctx.r, cd.r))
                    };
                    eqv(&format!("{label} begin"), a, b);
                    run_session(
                        &label,
                        &ctx,
                        "ZSTD_compressContinue",
                        "ZSTD_compressEnd",
                        &src,
                        &chunks,
                    );
                }

                // _advanced: explicit fParams + pledgedSrcSize
                for cs in 0..2 {
                    for ck in 0..2 {
                        for nd in 0..2 {
                            let f = ZSTD_frameParameters {
                                contentSizeFlag: cs,
                                checksumFlag: ck,
                                noDictIDFlag: nd,
                            };
                            for &pledged in
                                [src.len() as c_ulonglong, ZSTD_CONTENTSIZE_UNKNOWN].iter()
                            {
                                let ctx = CtxPair::cctx();
                                let label = format!(
                                    "row84 usingCDict_advanced {dn} lvl={lvl} f=({cs},{ck},{nd}) pledged={pledged}"
                                );
                                eqv(
                                    &format!("{label} begin"),
                                    bac(ctx.c, cd.c, f, pledged),
                                    bar(ctx.r, cd.r, f, pledged),
                                );
                                run_session(
                                    &label,
                                    &ctx,
                                    "ZSTD_compressContinue",
                                    "ZSTD_compressEnd",
                                    &src,
                                    &chunks,
                                );
                            }
                        }
                    }
                }
            }
        }

        // --- CDict built with every dictLoadMethod / dictContentType ----
        let dict = trained_dict().clone();
        for &dlm in [ZSTD_dlm_byCopy, ZSTD_dlm_byRef].iter() {
            for &dct in [ZSTD_dct_auto, ZSTD_dct_rawContent, ZSTD_dct_fullDict].iter() {
                for &lvl in [1, 5, 11, 19].iter() {
                    let cp = gcc(lvl, ZSTD_CONTENTSIZE_UNKNOWN, dict.len());
                    eqv(
                        "row84 ZSTD_getCParams",
                        cp,
                        gcr(lvl, ZSTD_CONTENTSIZE_UNKNOWN, dict.len()),
                    );
                    let cd = DictPair::cdict_advanced(&dict, dlm, dct, cp);
                    let class = rng.below(N_CLASSES);
                    let size = [16usize, 4096, 66_000][rng.below(3)];
                    let src = gen_class(class, size, 0x8403);
                    let chunks = chunks_for(&mut rng, src.len());
                    let ctx = CtxPair::cctx();
                    let label =
                        format!("row84 cdict_adv dlm={dlm} dct={dct} lvl={lvl} size={size}");
                    eqv(&format!("{label} begin"), bcc(ctx.c, cd.c), bcr(ctx.r, cd.r));
                    run_session(
                        &label,
                        &ctx,
                        "ZSTD_compressContinue",
                        "ZSTD_compressEnd",
                        &src,
                        &chunks,
                    );
                }
            }
        }
    }
}

// ================================================================ row 86

#[test]
fn row86_copy_cctx() {
    unsafe {
        let (bc, br) = duo::<FnBegin>("ZSTD_compressBegin");
        let (bdc, bdr) = duo::<FnBeginDict>("ZSTD_compressBegin_usingDict");
        let (cpc, cpr) = duo::<FnCopyCCtx>("ZSTD_copyCCtx");
        let mut rng = Rng::new(0x8600_0001);
        let dict = raw_dict(8192);

        for &lvl in [-3, -1, 0, 1, 3, 7, 12, 17].iter() {
            for use_dict in [false, true] {
                for &pledged_kind in [0u32, 1, 2].iter() {
                    let class = rng.below(N_CLASSES);
                    let size = [0usize, 1, 64, 5000, 40_000, 140_000][rng.below(6)];
                    if size > 40_000 && lvl > 17 {
                        continue;
                    }
                    let src = gen_class(class, size, 0x86);
                    let prepared = CtxPair::cctx();
                    let label = format!(
                        "row86 lvl={lvl} dict={use_dict} pledged={pledged_kind} size={size}"
                    );
                    if use_dict {
                        eqv(
                            &format!("{label} begin_usingDict"),
                            bdc(prepared.c, dict.as_ptr() as *const c_void, dict.len(), lvl),
                            bdr(prepared.r, dict.as_ptr() as *const c_void, dict.len(), lvl),
                        );
                    } else {
                        eqv(
                            &format!("{label} begin"),
                            bc(prepared.c, lvl),
                            br(prepared.r, lvl),
                        );
                    }
                    let pledged = match pledged_kind {
                        0 => ZSTD_CONTENTSIZE_UNKNOWN,
                        1 => src.len() as c_ulonglong,
                        _ => 0,
                    };
                    let chunks = chunks_for(&mut rng, src.len());
                    let frame = {
                        let dest = CtxPair::cctx();
                        eqv(
                            &format!("{label} copyCCtx"),
                            cpc(dest.c, prepared.c as *const c_void, pledged),
                            cpr(dest.r, prepared.r as *const c_void, pledged),
                        );
                        run_session(
                            &label,
                            &dest,
                            "ZSTD_compressContinue",
                            "ZSTD_compressEnd",
                            &src,
                            &chunks,
                        )
                    };
                    if let Some(frame) = frame {
                        if !use_dict {
                            check_roundtrip(&label, &frame, &src);
                        }
                    }
                    // copying twice from the same prepared context must work too
                    let dest2 = CtxPair::cctx();
                    eqv(
                        &format!("{label} copyCCtx#2"),
                        cpc(dest2.c, prepared.c as *const c_void, ZSTD_CONTENTSIZE_UNKNOWN),
                        cpr(dest2.r, prepared.r as *const c_void, ZSTD_CONTENTSIZE_UNKNOWN),
                    );
                    run_session(
                        &format!("{label}#2"),
                        &dest2,
                        "ZSTD_compressContinue",
                        "ZSTD_compressEnd",
                        &src,
                        &[src.len()],
                    );
                }
            }
        }
    }
}

// ================================================================ row 87 / 80

#[test]
fn row87_decompress_bufferless() {
    unsafe {
        let (wsk, _) = duo::<FnWriteSkippable>("ZSTD_writeSkippableFrame");
        let mut rng = Rng::new(0x8700_0001);

        let sizes: [usize; 9] =
            [0, 1, 7, 128, 1024, 8 * 1024, 64 * 1024, 128 * 1024 + 1, 200_000];
        for class in 0..N_CLASSES {
            for (si, &size) in sizes.iter().enumerate() {
                for &lvl in [-2, 1, 3, 9, 19].iter() {
                    if size > 64 * 1024 && lvl > 9 && si % 2 == 0 {
                        continue;
                    }
                    let src = gen_class(class, size, 0x87 ^ si as u64);
                    let cs = rng.below(2) as c_int;
                    let ck = rng.below(2) as c_int;
                    let did = rng.below(2) as c_int;
                    let frame = c_compress_with(
                        &src,
                        &[
                            (ZSTD_c_compressionLevel, lvl),
                            (ZSTD_c_contentSizeFlag, cs),
                            (ZSTD_c_checksumFlag, ck),
                            (ZSTD_c_dictIDFlag, did),
                        ],
                    );
                    let dctx = CtxPair::dctx();
                    let label = format!(
                        "row87 class={} size={size} lvl={lvl} cs={cs} ck={ck}",
                        CLASS_NAMES[class]
                    );
                    let partial = rng.below(2) == 0;
                    let out = bufferless_decode(&label, &dctx, &frame, src.len(), partial)
                        .expect("bufferless decode failed");
                    eqbuf(&format!("{label} content"), &src, &out);
                }
            }
        }

        // a skippable frame followed by a real frame
        for &pl in [0usize, 1, 17, 1000].iter() {
            let payload = gen_class(3, pl, 0x8788);
            let src = gen_class(4, 5000, 0x8789);
            let inner = c_compress_with(&src, &[(ZSTD_c_compressionLevel, 5)]);
            let mut frame = vec![0u8; pl + 16];
            let n = wsk(
                frame.as_mut_ptr() as *mut c_void,
                frame.len(),
                payload.as_ptr() as *const c_void,
                pl,
                7,
            );
            assert!(!is_err(n));
            frame.truncate(n);
            frame.extend_from_slice(&inner);
            let dctx = CtxPair::dctx();
            let label = format!("row87 skippable pl={pl}");
            let out = bufferless_decode(&label, &dctx, &frame, src.len(), false)
                .expect("bufferless decode of skippable+frame failed");
            eqbuf(&format!("{label} content"), &src, &out);
        }

        // magicless frames, decoded with ZSTD_d_format = magicless
        let (dsp, dspr) = duo::<FnSetParam>("ZSTD_DCtx_setParameter");
        for &size in [0usize, 100, 9000, 140_000].iter() {
            for &ck in [0, 1].iter() {
                let src = gen_class(5, size, 0x878A);
                let frame = c_compress_with(
                    &src,
                    &[
                        (ZSTD_c_compressionLevel, 4),
                        (ZSTD_c_format, 1),
                        (ZSTD_c_checksumFlag, ck),
                    ],
                );
                let dctx = CtxPair::dctx();
                eqv(
                    "row87 magicless DCtx_setParameter",
                    dsp(dctx.c, ZSTD_d_format, 1),
                    dspr(dctx.r, ZSTD_d_format, 1),
                );
                let label = format!("row87 magicless size={size} ck={ck}");
                let out = bufferless_decode(&label, &dctx, &frame, src.len(), false)
                    .expect("bufferless decode of magicless frame failed");
                eqbuf(&format!("{label} content"), &src, &out);
            }
        }

        // reuse one DCtx across many frames
        let dctx = CtxPair::dctx();
        for i in 0..24usize {
            let class = i % N_CLASSES;
            let size = [1usize, 40, 3000, 70_000][i % 4];
            let src = gen_class(class, size, 0x878B + i as u64);
            let frame = c_compress_with(
                &src,
                &[
                    (ZSTD_c_compressionLevel, (i % 9) as c_int),
                    (ZSTD_c_checksumFlag, (i % 2) as c_int),
                ],
            );
            let label = format!("row87 reuse i={i}");
            let out = bufferless_decode(&label, &dctx, &frame, src.len(), i % 3 == 0)
                .expect("bufferless decode failed");
            eqbuf(&format!("{label} content"), &src, &out);
        }
    }
}

// ================================================================ row 88

#[test]
fn row88_decompress_begin_using_dict() {
    unsafe {
        let (bdc, bdr) = duo::<FnBeginUsingDict>("ZSTD_decompressBegin_usingDict");
        let (bddc, bddr) = duo::<FnBeginUsingDDict>("ZSTD_decompressBegin_usingDDict");
        let (nc, nr) = duo::<FnCtx2Size>("ZSTD_nextSrcSizeToDecompress");
        let (tc, tr) = duo::<FnNextType>("ZSTD_nextInputType");
        let (kc, kr) = duo::<FnBlk>("ZSTD_decompressContinue");
        let (cudc, _) = duo::<
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
        let (create, _) = duo::<FnPtr0>("ZSTD_createCCtx");
        let (free, _) = duo::<FnFreePtr>("ZSTD_freeCCtx");
        let mut rng = Rng::new(0x8800_0001);

        let dicts: Vec<(&str, Vec<u8>)> = vec![
            ("raw4k", raw_dict(4096)),
            ("raw64k", raw_dict(64 * 1024)),
            ("trained", trained_dict().clone()),
        ];

        // A tiny local bufferless decode loop that does NOT call
        // ZSTD_decompressBegin (the dict variants take that role).
        let decode = |label: &str,
                      dctx: &CtxPair,
                      frame: &[u8],
                      out_cap: usize|
         -> Option<Vec<u8>> {
            let cap = out_cap.max(1);
            let mut oc = vec![0xA5u8; cap];
            let mut or = vec![0xA5u8; cap];
            let mut ip = 0usize;
            let mut op = 0usize;
            let mut step = 0usize;
            loop {
                let en_c = nc(dctx.c);
                eqv(&format!("{label}s{step} nextSrcSize"), en_c, nr(dctx.r));
                eqv(&format!("{label}s{step} nextInputType"), tc(dctx.c), tr(dctx.r));
                if en_c == 0 {
                    break;
                }
                assert!(ip + en_c <= frame.len(), "{label}: input exhausted");
                let sp = frame.as_ptr().add(ip) as *const c_void;
                let avail = cap - op;
                let rc = kc(dctx.c, oc.as_mut_ptr().add(op) as *mut c_void, avail, sp, en_c);
                let rr = kr(dctx.r, or.as_mut_ptr().add(op) as *mut c_void, avail, sp, en_c);
                eqv(&format!("{label}s{step} decompressContinue"), rc, rr);
                if is_err(rc) {
                    return None;
                }
                eqbuf(&format!("{label}s{step} out"), &oc[..op + rc], &or[..op + rc]);
                ip += en_c;
                op += rc;
                step += 1;
                assert!(step < 100_000);
            }
            oc.truncate(op);
            Some(oc)
        };

        for (dn, dict) in dicts.iter() {
            for &lvl in [-1, 1, 3, 8, 15, 19].iter() {
                for &size in [0usize, 1, 300, 9000, 140_000].iter() {
                    let class = rng.below(N_CLASSES);
                    let src = gen_class(class, size, 0x88);
                    // build a dictionary-compressed frame with the C library
                    let cctx = create();
                    let cap = bound(src.len()) + 64;
                    let mut frame = vec![0u8; cap];
                    let n = cudc(
                        cctx,
                        frame.as_mut_ptr() as *mut c_void,
                        cap,
                        src.as_ptr() as *const c_void,
                        src.len(),
                        dict.as_ptr() as *const c_void,
                        dict.len(),
                        lvl,
                    );
                    free(cctx);
                    assert!(!is_err(n), "compress_usingDict failed");
                    frame.truncate(n);

                    let label = format!("row88 usingDict {dn} lvl={lvl} size={size} ");
                    let dctx = CtxPair::dctx();
                    eqv(
                        &format!("{label}decompressBegin_usingDict"),
                        bdc(dctx.c, dict.as_ptr() as *const c_void, dict.len()),
                        bdr(dctx.r, dict.as_ptr() as *const c_void, dict.len()),
                    );
                    let out = decode(&label, &dctx, &frame, src.len())
                        .unwrap_or_else(|| panic!("{label}: bufferless decode failed"));
                    eqbuf(&format!("{label}content"), &src, &out);

                    // ... and the same through a DDict
                    for &(nm, dlm, dct) in [
                        ("plain", -1, -1),
                        ("byCopy/auto", ZSTD_dlm_byCopy, ZSTD_dct_auto),
                        ("byRef/auto", ZSTD_dlm_byRef, ZSTD_dct_auto),
                        ("byCopy/raw", ZSTD_dlm_byCopy, ZSTD_dct_rawContent),
                    ]
                    .iter()
                    {
                        let dd = if dlm < 0 {
                            DictPair::ddict(dict)
                        } else {
                            DictPair::ddict_advanced(dict, dlm, dct)
                        };
                        let label2 = format!("{label}ddict={nm} ");
                        let dctx2 = CtxPair::dctx();
                        eqv(
                            &format!("{label2}decompressBegin_usingDDict"),
                            bddc(dctx2.c, dd.c),
                            bddr(dctx2.r, dd.r),
                        );
                        // a trained dictionary forced to `rawContent` loses its
                        // dictID, so the frame legitimately fails to decode —
                        // both builds must agree on that (checked above).
                        let must_work = dct != ZSTD_dct_rawContent || *dn != "trained";
                        match decode(&label2, &dctx2, &frame, src.len()) {
                            Some(out2) => eqbuf(&format!("{label2}content"), &src, &out2),
                            None => assert!(
                                !must_work,
                                "{label2}: bufferless decode failed unexpectedly"
                            ),
                        }
                    }
                }
            }
        }

        // NULL DDict == no dictionary
        {
            let src = gen_class(4, 4000, 0x8899);
            let frame = c_compress_with(&src, &[(ZSTD_c_compressionLevel, 3)]);
            let dctx = CtxPair::dctx();
            eqv(
                "row88 usingDDict(NULL) begin",
                bddc(dctx.c, std::ptr::null()),
                bddr(dctx.r, std::ptr::null()),
            );
            let out = decode("row88 nullddict ", &dctx, &frame, src.len()).unwrap();
            eqbuf("row88 nullddict content", &src, &out);
            let dctx = CtxPair::dctx();
            eqv(
                "row88 usingDict(NULL,0) begin",
                bdc(dctx.c, std::ptr::null(), 0),
                bdr(dctx.r, std::ptr::null(), 0),
            );
            let out = decode("row88 nulldict ", &dctx, &frame, src.len()).unwrap();
            eqbuf("row88 nulldict content", &src, &out);
        }
    }
}

// ================================================================ row 89

#[test]
fn row89_copy_dctx() {
    unsafe {
        let (bc, br) = duo::<FnCtx2Size>("ZSTD_decompressBegin");
        let (bdc, bdr) = duo::<FnBeginUsingDict>("ZSTD_decompressBegin_usingDict");
        let (cpc, cpr) = duo::<FnCopyDCtx>("ZSTD_copyDCtx");
        let (nc, nr) = duo::<FnCtx2Size>("ZSTD_nextSrcSizeToDecompress");
        let (tc, tr) = duo::<FnNextType>("ZSTD_nextInputType");
        let (kc, kr) = duo::<FnBlk>("ZSTD_decompressContinue");
        let (cudc, _) = duo::<
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
        let (create, _) = duo::<FnPtr0>("ZSTD_createCCtx");
        let (free, _) = duo::<FnFreePtr>("ZSTD_freeCCtx");
        let dict = raw_dict(16 * 1024);
        let mut rng = Rng::new(0x8900_0001);

        for &lvl in [1, 3, 6, 12, 19].iter() {
            for &size in [1usize, 500, 30_000, 200_000, 400_000].iter() {
                for &with_dict in [false, true].iter() {
                    for &ck in [0, 1].iter() {
                        let class = rng.below(N_CLASSES);
                        let src = gen_class(class, size, 0x89);
                        let frame = if with_dict {
                            let cctx = create();
                            let cap = bound(src.len()) + 64;
                            let mut f = vec![0u8; cap];
                            let n = cudc(
                                cctx,
                                f.as_mut_ptr() as *mut c_void,
                                cap,
                                src.as_ptr() as *const c_void,
                                src.len(),
                                dict.as_ptr() as *const c_void,
                                dict.len(),
                                lvl,
                            );
                            free(cctx);
                            assert!(!is_err(n));
                            f.truncate(n);
                            f
                        } else {
                            c_compress_with(
                                &src,
                                &[
                                    (ZSTD_c_compressionLevel, lvl),
                                    (ZSTD_c_checksumFlag, ck),
                                ],
                            )
                        };
                        let label = format!(
                            "row89 lvl={lvl} size={size} dict={with_dict} ck={ck} class={}",
                            CLASS_NAMES[class]
                        );

                        let a = CtxPair::dctx();
                        if with_dict {
                            eqv(
                                &format!("{label} begin_usingDict"),
                                bdc(a.c, dict.as_ptr() as *const c_void, dict.len()),
                                bdr(a.r, dict.as_ptr() as *const c_void, dict.len()),
                            );
                        } else {
                            eqv(&format!("{label} begin"), bc(a.c), br(a.r));
                        }
                        // copy the freshly prepared context (the classic use)
                        let b = CtxPair::dctx();
                        cpc(b.c, a.c as *const c_void);
                        cpr(b.r, a.r as *const c_void);

                        // decode using `a` up to a random block boundary, then
                        // hand the state to a third context and finish there,
                        // writing into the *same* output buffer.
                        let cap = src.len().max(1);
                        let mut oc = vec![0xA5u8; cap];
                        let mut or = vec![0xA5u8; cap];
                        let mut ip = 0usize;
                        let mut op = 0usize;
                        let mut step = 0usize;
                        let switch_at = rng.below(6);
                        let mid = CtxPair::dctx();
                        let mut cur_c = a.c;
                        let mut cur_r = a.r;
                        let mut switched = false;
                        loop {
                            let en_c = nc(cur_c);
                            eqv(&format!("{label} s{step} nextSrcSize"), en_c, nr(cur_r));
                            let ty_c = tc(cur_c);
                            eqv(&format!("{label} s{step} nextInputType"), ty_c, tr(cur_r));
                            if en_c == 0 {
                                break;
                            }
                            // `ZSTD_copyDCtx` does not copy `headerBuffer`, so a
                            // hand-over is only meaningful before the frame
                            // header has been (partially) loaded or once we are
                            // at a block boundary.
                            if !switched
                                && step >= switch_at
                                && (step == 0 || ty_c == ZSTDnit_blockHeader)
                            {
                                cpc(mid.c, cur_c as *const c_void);
                                cpr(mid.r, cur_r as *const c_void);
                                cur_c = mid.c;
                                cur_r = mid.r;
                                switched = true;
                                // the copy must report the same expectations
                                eqv(
                                    &format!("{label} s{step} copy nextSrcSize"),
                                    en_c,
                                    nc(cur_c),
                                );
                                eqv(
                                    &format!("{label} s{step} copy nextSrcSize r"),
                                    en_c,
                                    nr(cur_r),
                                );
                                eqv(
                                    &format!("{label} s{step} copy nextInputType"),
                                    tc(cur_c),
                                    tr(cur_r),
                                );
                            }
                            assert!(ip + en_c <= frame.len(), "{label}: input exhausted");
                            let sp = frame.as_ptr().add(ip) as *const c_void;
                            let avail = cap - op;
                            let rc =
                                kc(cur_c, oc.as_mut_ptr().add(op) as *mut c_void, avail, sp, en_c);
                            let rr =
                                kr(cur_r, or.as_mut_ptr().add(op) as *mut c_void, avail, sp, en_c);
                            eqv(&format!("{label} s{step} decompressContinue"), rc, rr);
                            if !is_err(rc) {
                                eqbuf(
                                    &format!("{label} s{step} out"),
                                    &oc[..op + rc],
                                    &or[..op + rc],
                                );
                            }
                            assert!(
                                !is_err(rc),
                                "{label}: decompressContinue failed at step {step} \
                                 (switch_at={switch_at}, switched={switched}): {}",
                                errname(rc)
                            );
                            ip += en_c;
                            op += rc;
                            step += 1;
                            assert!(step < 100_000);
                        }
                        eqv(&format!("{label} total"), op, src.len());
                        eqbuf(&format!("{label} content"), &src, &oc[..op]);

                        // and now decode the whole frame again from the copy `b`
                        let mut oc2 = vec![0xA5u8; cap];
                        let mut or2 = vec![0xA5u8; cap];
                        let mut ip = 0usize;
                        let mut op = 0usize;
                        let mut step = 0usize;
                        loop {
                            let en_c = nc(b.c);
                            eqv(&format!("{label} b s{step} nextSrcSize"), en_c, nr(b.r));
                            if en_c == 0 {
                                break;
                            }
                            let sp = frame.as_ptr().add(ip) as *const c_void;
                            let avail = cap - op;
                            let rc =
                                kc(b.c, oc2.as_mut_ptr().add(op) as *mut c_void, avail, sp, en_c);
                            let rr =
                                kr(b.r, or2.as_mut_ptr().add(op) as *mut c_void, avail, sp, en_c);
                            eqv(&format!("{label} b s{step} decompressContinue"), rc, rr);
                            assert!(
                                !is_err(rc),
                                "{label} b: decompressContinue failed: {}",
                                errname(rc)
                            );
                            eqbuf(
                                &format!("{label} b s{step} out"),
                                &oc2[..op + rc],
                                &or2[..op + rc],
                            );
                            ip += en_c;
                            op += rc;
                            step += 1;
                            assert!(step < 100_000);
                        }
                        eqbuf(&format!("{label} b content"), &src, &oc2[..op]);
                    }
                }
            }
        }
    }
}

// ================================================================ rows 90 / 91

/// How a block-level session starts.
#[derive(Clone, Copy)]
enum Begin {
    Level(c_int),
    Params(ZSTD_parameters),
}

/// One block-level compress/decompress session.
///
/// * `invalidate`: call `ZSTD_invalidateRepCodes` before every other block.
/// * `split_dst`: decode block *i* into region *i* of a sparse output buffer,
///   which forces `ZSTD_checkContinuity` down its "external dictionary" branch.
unsafe fn block_session(
    label: &str,
    how: Begin,
    src: &[u8],
    nblocks: usize,
    deprecated: bool,
    invalidate: bool,
    split_dst: bool,
) {
    let (bc, br) = duo::<FnBegin>("ZSTD_compressBegin");
    let (bac, bar) = duo::<FnBeginAdv>("ZSTD_compressBegin_advanced");
    let (gc, gr) = duo::<FnCtxConst2Size>("ZSTD_getBlockSize");
    let (cbc, cbr) = duo::<FnBlk>(if deprecated {
        "ZSTD_compressBlock_deprecated"
    } else {
        "ZSTD_compressBlock"
    });
    let (dbc, dbr) = duo::<FnBlk>(if deprecated {
        "ZSTD_decompressBlock_deprecated"
    } else {
        "ZSTD_decompressBlock"
    });
    let (dbegc, dbegr) = duo::<FnCtx2Size>("ZSTD_decompressBegin");
    let (ibc, ibr) = duo::<FnInsertBlock>("ZSTD_insertBlock");
    let (ccc, ccr) = duo::<FnCheckCont>("ZSTD_checkContinuity");
    let (ivc, ivr) = duo::<FnVoidCtx>("ZSTD_invalidateRepCodes");

    let cctx = CtxPair::cctx();
    match how {
        Begin::Level(lvl) => eqv(
            &format!("{label} compressBegin"),
            bc(cctx.c, lvl),
            br(cctx.r, lvl),
        ),
        Begin::Params(p) => eqv(
            &format!("{label} compressBegin_advanced"),
            bac(cctx.c, std::ptr::null(), 0, p, ZSTD_CONTENTSIZE_UNKNOWN),
            bar(cctx.r, std::ptr::null(), 0, p, ZSTD_CONTENTSIZE_UNKNOWN),
        ),
    }
    let bs_c = gc(cctx.c as *const c_void);
    let bs_r = gr(cctx.r as *const c_void);
    eqv(&format!("{label} getBlockSize"), bs_c, bs_r);
    assert!(bs_c > 0 && bs_c <= ZSTD_BLOCKSIZE_MAX);

    // slice the input into <= nblocks blocks of <= blockSize bytes
    let n = src.len();
    let per = ((n + nblocks - 1) / nblocks).max(1).min(bs_c);
    let mut blocks: Vec<(usize, usize)> = Vec::new();
    let mut off = 0usize;
    while off < n {
        let len = per.min(n - off);
        blocks.push((off, len));
        off += len;
    }
    if blocks.is_empty() {
        blocks.push((0, 0));
    }

    // ---- compress every block, comparing return value + destination ----
    // (srcOff, srcLen, cSize, compressed body)
    let mut bodies: Vec<(usize, usize, usize, Vec<u8>)> = Vec::new();
    for (i, &(o, len)) in blocks.iter().enumerate() {
        if invalidate && i % 2 == 1 {
            ivc(cctx.c);
            ivr(cctx.r);
        }
        let cap = bound(len) + 64;
        let mut dc = vec![0xA5u8; cap];
        let mut dr = vec![0xA5u8; cap];
        let sp = src.as_ptr().add(o) as *const c_void;
        let rc = cbc(cctx.c, dc.as_mut_ptr() as *mut c_void, cap, sp, len);
        let rr = cbr(cctx.r, dr.as_mut_ptr() as *mut c_void, cap, sp, len);
        eqv(&format!("{label} block{i} compressBlock"), rc, rr);
        eqbuf(&format!("{label} block{i} dst"), &dc, &dr);
        if is_err(rc) {
            return;
        }
        bodies.push((o, len, rc, dc[..rc].to_vec()));
    }

    // ---- decode -------------------------------------------------------
    let dctx = CtxPair::dctx();
    eqv(&format!("{label} decompressBegin"), dbegc(dctx.c), dbegr(dctx.r));
    // a sparse output buffer: block i lands at stride*i
    let stride = if split_dst { bs_c + 4096 } else { 0 };
    let total = if split_dst {
        stride * blocks.len() + n + 4096
    } else {
        n.max(1)
    };
    let mut oc = vec![0xA5u8; total];
    let mut or = vec![0xA5u8; total];
    let mut plain = 0usize; // running offset when !split_dst
    for (i, (o, len, csize, body)) in bodies.iter().enumerate() {
        let (o, len, csize) = (*o, *len, *csize);
        let dst_off = if split_dst { stride * i } else { plain };
        // exercise the two no-op branches of ZSTD_checkContinuity explicitly
        ccc(dctx.c, oc.as_ptr().add(dst_off) as *const c_void, 0);
        ccr(dctx.r, or.as_ptr().add(dst_off) as *const c_void, 0);
        if csize == 0 {
            // incompressible: transmit raw and register it in the history
            oc[dst_off..dst_off + len].copy_from_slice(&src[o..o + len]);
            or[dst_off..dst_off + len].copy_from_slice(&src[o..o + len]);
            let rc = ibc(dctx.c, oc.as_ptr().add(dst_off) as *const c_void, len);
            let rr = ibr(dctx.r, or.as_ptr().add(dst_off) as *const c_void, len);
            eqv(&format!("{label} block{i} insertBlock"), rc, rr);
            eqbuf(
                &format!("{label} block{i} insert out"),
                &oc[dst_off..dst_off + len],
                &or[dst_off..dst_off + len],
            );
        } else {
            let avail = total - dst_off;
            let rc = dbc(
                dctx.c,
                oc.as_mut_ptr().add(dst_off) as *mut c_void,
                avail,
                body.as_ptr() as *const c_void,
                csize,
            );
            let rr = dbr(
                dctx.r,
                or.as_mut_ptr().add(dst_off) as *mut c_void,
                avail,
                body.as_ptr() as *const c_void,
                csize,
            );
            eqv(&format!("{label} block{i} decompressBlock"), rc, rr);
            assert!(
                !is_err(rc),
                "{label} block{i}: decompressBlock failed: {}",
                errname(rc)
            );
            eqbuf(
                &format!("{label} block{i} out"),
                &oc[dst_off..dst_off + rc],
                &or[dst_off..dst_off + rc],
            );
            eqv(&format!("{label} block{i} size"), rc, len);
        }
        eqbuf(
            &format!("{label} block{i} content"),
            &src[o..o + len],
            &oc[dst_off..dst_off + len],
        );
        plain += len;
    }
}

#[test]
fn row90_block_api() {
    unsafe {
        let mut rng = Rng::new(0x9000_0001);
        for &lvl in [-3, -1, 0, 1, 2, 3, 5, 8, 12, 17].iter() {
            for class in 0..N_CLASSES {
                for &size in [1usize, 2, 7, 100, 4096, 60_000, 131_072].iter() {
                    if size >= 60_000 && lvl > 12 {
                        continue;
                    }
                    let src = gen_class(class, size, 0x90 ^ lvl as u64);
                    let nb = 1 + rng.below(3);
                    let dep = rng.below(2) == 0;
                    let label = format!(
                        "row90 lvl={lvl} class={} size={size} nb={nb} dep={dep}",
                        CLASS_NAMES[class]
                    );
                    block_session(&label, Begin::Level(lvl), &src, nb, dep, false, false);
                }
            }
        }
        // exactly one full-size block, every class
        for class in 0..N_CLASSES {
            for &lvl in [1, 3, 9].iter() {
                let src = gen_class(class, ZSTD_BLOCKSIZE_MAX, 0x9001);
                let label = format!("row90 full lvl={lvl} class={}", CLASS_NAMES[class]);
                block_session(&label, Begin::Level(lvl), &src, 1, false, false, false);
            }
        }
        // all 9 strategies (incl. btultra2) through the block API.  The window
        // is derived from the real srcSize here, which keeps the contexts small.
        let (gpc, gpr) = duo::<FnGetParams>("ZSTD_getParams");
        for &strategy in ALL_STRATEGIES.iter() {
            for class in 0..N_CLASSES {
                for &size in [1usize, 300, 9000, 131_072].iter() {
                    for &lvl in [1i32, 12, 22].iter() {
                        let src = gen_class(class, size, 0x9002 ^ strategy as u64);
                        let mut p = gpc(lvl, size as c_ulonglong, 0);
                        eqv("row90 getParams", p, gpr(lvl, size as c_ulonglong, 0));
                        p.cParams.strategy = strategy as c_uint;
                        let nb = 1 + rng.below(3);
                        let label = format!(
                            "row90 strat={strategy} lvl={lvl} class={} size={size} nb={nb}",
                            CLASS_NAMES[class]
                        );
                        block_session(&label, Begin::Params(p), &src, nb, false, false, false);
                    }
                }
            }
        }
    }
}

#[test]
fn row91_insert_block_and_repcodes() {
    unsafe {
        let mut rng = Rng::new(0x9100_0001);
        for &lvl in [-2, 1, 3, 6, 11, 17].iter() {
            for class in 0..N_CLASSES {
                let size = [64usize, 3000, 50_000, 120_000][rng.below(4)];
                let src = gen_class(class, size, 0x91);
                // invalidateRepCodes between blocks
                let label = format!(
                    "row91 invalidate lvl={lvl} class={} size={size}",
                    CLASS_NAMES[class]
                );
                block_session(&label, Begin::Level(lvl), &src, 3, false, true, false);
                // two blocks decoded into two disjoint regions: extDict path
                let label = format!(
                    "row91 split lvl={lvl} class={} size={size}",
                    CLASS_NAMES[class]
                );
                block_session(&label, Begin::Level(lvl), &src, 2, false, false, true);
                let label = format!("row91 split+dep lvl={lvl} class={}", CLASS_NAMES[class]);
                block_session(&label, Begin::Level(lvl), &src, 2, true, true, true);
            }
        }
        // ZSTD_insertBlock on its own: a pure raw-block history
        let (dbeg, dbegr) = duo::<FnCtx2Size>("ZSTD_decompressBegin");
        let (ibc, ibr) = duo::<FnInsertBlock>("ZSTD_insertBlock");
        let (ccc, ccr) = duo::<FnCheckCont>("ZSTD_checkContinuity");
        let data = gen_class(3, 40_000, 0x9102);
        let dctx = CtxPair::dctx();
        eqv("row91 decompressBegin", dbeg(dctx.c), dbegr(dctx.r));
        let mut off = 0usize;
        let mut i = 0usize;
        while off < data.len() {
            let len = (1 + rng.below(9000)).min(data.len() - off);
            let p = data.as_ptr().add(off) as *const c_void;
            eqv(
                &format!("row91 insertBlock{i}"),
                ibc(dctx.c, p, len),
                ibr(dctx.r, p, len),
            );
            // and a no-op continuity check at the same spot
            ccc(dctx.c, p, 0);
            ccr(dctx.r, p, 0);
            off += len;
            i += 1;
        }
    }
}

// ================================================================ row 92

#[test]
fn row92_getc_block_size() {
    unsafe {
        let (fc, fr) = duo::<FnGetcBlockSize>("ZSTD_getcBlockSize");
        let mut rng = Rng::new(0x9200_0001);
        let mut cases: Vec<(u32, usize)> = Vec::new();
        // every block type × lastBlock × a size ladder, and short buffers
        for bt in 0..4u32 {
            for last in 0..2u32 {
                for &sz in [
                    0u32,
                    1,
                    2,
                    3,
                    17,
                    1023,
                    1024,
                    65535,
                    131071,
                    131072,
                    131073,
                    0x1FFFFF,
                ]
                .iter()
                {
                    let hdr = (sz << 3) | (bt << 1) | last;
                    for &avail in [0usize, 1, 2, 3, 4, 100].iter() {
                        cases.push((hdr, avail));
                    }
                }
            }
        }
        for _ in 0..2000 {
            cases.push((rng.next_u32() & 0x00FF_FFFF, 3 + rng.below(4)));
        }
        for (i, &(hdr, avail)) in cases.iter().enumerate() {
            let mut buf = vec![0u8; 8];
            buf[0] = (hdr & 0xFF) as u8;
            buf[1] = ((hdr >> 8) & 0xFF) as u8;
            buf[2] = ((hdr >> 16) & 0xFF) as u8;
            let mut pc = BlockProps { blockType: -7, lastBlock: 0xDEAD, origSize: 0xBEEF };
            let mut pr = pc;
            let rc = fc(buf.as_ptr() as *const c_void, avail, &mut pc);
            let rr = fr(buf.as_ptr() as *const c_void, avail, &mut pr);
            eqv(&format!("row92 case{i} hdr={hdr:#08x} avail={avail} ret"), rc, rr);
            eqv(&format!("row92 case{i} hdr={hdr:#08x} avail={avail} props"), pc, pr);
        }
    }
}

// ================================================================ row 93

#[test]
fn row93_write_last_empty_block() {
    unsafe {
        let (fc, fr) = duo::<FnWriteLastEmpty>("ZSTD_writeLastEmptyBlock");
        for cap in 0usize..12 {
            let mut dc = vec![0xA5u8; 16];
            let mut dr = vec![0xA5u8; 16];
            let rc = fc(dc.as_mut_ptr() as *mut c_void, cap);
            let rr = fr(dr.as_mut_ptr() as *mut c_void, cap);
            eqv(&format!("row93 cap={cap} ret"), rc, rr);
            eqbuf(&format!("row93 cap={cap} dst"), &dc, &dr);
        }
        // the emitted trailer must actually close a frame
        let src = gen_class(4, 5000, 0x93);
        let ctx = CtxPair::cctx();
        let (bc, br) = duo::<FnBegin>("ZSTD_compressBegin");
        eqv("row93 compressBegin", bc(ctx.c, 3), br(ctx.r, 3));
        let (cc, cr) = duo::<FnBlk>("ZSTD_compressContinue");
        let cap = bound(src.len()) + 512;
        let mut dc = vec![0xA5u8; cap];
        let mut dr = vec![0xA5u8; cap];
        let rc = cc(
            ctx.c,
            dc.as_mut_ptr() as *mut c_void,
            cap,
            src.as_ptr() as *const c_void,
            src.len(),
        );
        let rr = cr(
            ctx.r,
            dr.as_mut_ptr() as *mut c_void,
            cap,
            src.as_ptr() as *const c_void,
            src.len(),
        );
        eqv("row93 compressContinue", rc, rr);
        eqbuf("row93 compressContinue dst", &dc, &dr);
        assert!(!is_err(rc));
        let mut frame_c = dc[..rc].to_vec();
        let mut tc = vec![0xA5u8; 8];
        let mut tr = vec![0xA5u8; 8];
        let ec = fc(tc.as_mut_ptr() as *mut c_void, tc.len());
        let er = fr(tr.as_mut_ptr() as *mut c_void, tr.len());
        eqv("row93 trailer ret", ec, er);
        eqbuf("row93 trailer dst", &tc, &tr);
        frame_c.extend_from_slice(&tc[..ec]);
        check_roundtrip("row93 assembled", &frame_c, &src);
    }
}

// ================================================================ row 94

#[test]
fn row94_block_internals() {
    unsafe {
        let (bc, br) = duo::<FnBegin>("ZSTD_compressBegin");
        let (cbc, cbr) = duo::<FnBlk>("ZSTD_compressBlock");
        let (dbegc, dbegr) = duo::<FnCtx2Size>("ZSTD_decompressBegin");
        let (dbc, dbr) = duo::<FnBlk>("ZSTD_decompressBlock_deprecated");
        let (dic, dir) = duo::<FnDecompBlkInt>("ZSTD_decompressBlock_internal");
        let (dlc, dlr) = duo::<FnDecLits>("ZSTD_decodeLiteralsBlock_wrapper");
        let (dsc, dsr) = duo::<FnDecSeqHdr>("ZSTD_decodeSeqHeaders");
        let mut rng = Rng::new(0x9400_0001);

        for &lvl in [-1, 1, 3, 6, 9, 13, 17].iter() {
            for class in 0..N_CLASSES {
                for &size in [1usize, 60, 3000, 40_000, 131_072].iter() {
                    if size >= 40_000 && lvl > 13 {
                        continue;
                    }
                    let src = gen_class(class, size, 0x94 ^ rng.next_u64() % 7);
                    let cctx = CtxPair::cctx();
                    eqv("row94 compressBegin", bc(cctx.c, lvl), br(cctx.r, lvl));
                    let cap = bound(size) + 64;
                    let mut ec = vec![0xA5u8; cap];
                    let mut er = vec![0xA5u8; cap];
                    let rc = cbc(
                        cctx.c,
                        ec.as_mut_ptr() as *mut c_void,
                        cap,
                        src.as_ptr() as *const c_void,
                        size,
                    );
                    let rr = cbr(
                        cctx.r,
                        er.as_mut_ptr() as *mut c_void,
                        cap,
                        src.as_ptr() as *const c_void,
                        size,
                    );
                    eqv("row94 compressBlock", rc, rr);
                    eqbuf("row94 compressBlock dst", &ec, &er);
                    if is_err(rc) || rc == 0 {
                        continue; // incompressible → no compressed body to inspect
                    }
                    let body = ec[..rc].to_vec();
                    let label = format!(
                        "row94 lvl={lvl} class={} size={size} cSize={rc}",
                        CLASS_NAMES[class]
                    );

                    // (a) decode with the public deprecated wrapper
                    let d1 = CtxPair::dctx();
                    eqv(&format!("{label} decompressBegin"), dbegc(d1.c), dbegr(d1.r));
                    // 64 guard bytes past `dstCapacity`: neither build may
                    // write outside the buffer it was handed.
                    const GUARD: usize = 64;
                    let ocap = ZSTD_BLOCKSIZE_MAX;
                    let mut oc = vec![0xA5u8; ocap + GUARD];
                    let mut or = vec![0xA5u8; ocap + GUARD];
                    let a = dbc(
                        d1.c,
                        oc.as_mut_ptr() as *mut c_void,
                        ocap,
                        body.as_ptr() as *const c_void,
                        rc,
                    );
                    let b = dbr(
                        d1.r,
                        or.as_mut_ptr() as *mut c_void,
                        ocap,
                        body.as_ptr() as *const c_void,
                        rc,
                    );
                    eqv(&format!("{label} decompressBlock_deprecated"), a, b);
                    assert!(!is_err(a), "{label}: {}", errname(a));
                    // only [0, a) is defined output — the following bytes are
                    // wildcopy slop / literals scratch
                    eqbuf(
                        &format!("{label} decompressBlock_deprecated out"),
                        &oc[..a],
                        &or[..a],
                    );
                    eqbuf(&format!("{label} content"), &src, &oc[..a]);
                    assert!(
                        oc[ocap..].iter().all(|&b| b == 0xA5),
                        "{label}: C wrote past dstCapacity"
                    );
                    assert!(
                        or[ocap..].iter().all(|&b| b == 0xA5),
                        "{label}: Rust wrote past dstCapacity"
                    );

                    // (b) ZSTD_decompressBlock_internal on the very same block,
                    //     re-using the context that has already seen it: it has
                    //     `isFrameDecompression == 0` and its history already
                    //     points at this very output buffer, which is what the
                    //     function requires (it performs no continuity check of
                    //     its own).
                    for &streaming in [0i32, 1].iter() {
                        let a2 = dic(
                            d1.c,
                            oc.as_mut_ptr() as *mut c_void,
                            ocap,
                            body.as_ptr() as *const c_void,
                            rc,
                            streaming,
                        );
                        let b2 = dir(
                            d1.r,
                            or.as_mut_ptr() as *mut c_void,
                            ocap,
                            body.as_ptr() as *const c_void,
                            rc,
                            streaming,
                        );
                        eqv(&format!("{label} decompressBlock_internal s={streaming}"), a2, b2);
                        assert!(
                            oc[ocap..].iter().all(|&b| b == 0xA5)
                                && or[ocap..].iter().all(|&b| b == 0xA5),
                            "{label}: decompressBlock_internal wrote past dstCapacity"
                        );
                        if !is_err(a2) {
                            eqbuf(
                                &format!("{label} decompressBlock_internal s={streaming} out"),
                                &oc[..a2],
                                &or[..a2],
                            );
                            eqbuf(
                                &format!("{label} decompressBlock_internal s={streaming} content"),
                                &src,
                                &oc[..a2],
                            );
                        }
                    }

                    // (c) literals section + sequence header decoding
                    let d2 = CtxPair::dctx();
                    eqv(&format!("{label} decompressBegin#2"), dbegc(d2.c), dbegr(d2.r));
                    let mut lc = vec![0xA5u8; ocap + GUARD];
                    let mut lr = vec![0xA5u8; ocap + GUARD];
                    let litc = dlc(
                        d2.c,
                        body.as_ptr() as *const c_void,
                        rc,
                        lc.as_mut_ptr() as *mut c_void,
                        ocap,
                    );
                    let litr = dlr(
                        d2.r,
                        body.as_ptr() as *const c_void,
                        rc,
                        lr.as_mut_ptr() as *mut c_void,
                        ocap,
                    );
                    eqv(&format!("{label} decodeLiteralsBlock_wrapper"), litc, litr);
                    eqbuf(
                        &format!("{label} decodeLiteralsBlock_wrapper out"),
                        &lc[..ocap],
                        &lr[..ocap],
                    );
                    assert!(
                        lc[ocap..].iter().all(|&b| b == 0xA5)
                            && lr[ocap..].iter().all(|&b| b == 0xA5),
                        "{label}: decodeLiteralsBlock_wrapper wrote past dstCapacity"
                    );
                    if is_err(litc) || litc > rc {
                        continue;
                    }
                    let mut nc: c_int = -12345;
                    let mut nr: c_int = -12345;
                    let sa = dsc(
                        d2.c,
                        &mut nc,
                        body.as_ptr().add(litc) as *const c_void,
                        rc - litc,
                    );
                    let sb = dsr(
                        d2.r,
                        &mut nr,
                        body.as_ptr().add(litc) as *const c_void,
                        rc - litc,
                    );
                    eqv(&format!("{label} decodeSeqHeaders"), sa, sb);
                    eqv(&format!("{label} decodeSeqHeaders nbSeq"), nc, nr);
                }
            }
        }
    }
}

// ================================================================ row 95

#[test]
fn row95_seq_store() {
    unsafe {
        let (bc, br) = duo::<FnBegin>("ZSTD_compressBegin");
        let (cbc, cbr) = duo::<FnBlk>("ZSTD_compressBlock");
        let (gsc, gsr) = duo::<FnGetSeqStore>("ZSTD_getSeqStore");
        let (rsc, rsr) = duo::<FnResetSeqStore>("ZSTD_resetSeqStore");
        let (rbc, rbr) = duo::<FnVoidCtx>("ZSTD_reset_compressedBlockState");
        let mut rng = Rng::new(0x9500_0001);
        let mut deep = 0usize;

        for &lvl in [-3, -1, 1, 2, 3, 5, 7, 9, 12, 16, 17].iter() {
            for class in 0..N_CLASSES {
                for &size in [1usize, 500, 20_000, 120_000].iter() {
                    if size > 20_000 && lvl > 16 {
                        continue;
                    }
                    let src = gen_class(class, size, 0x95);
                    let cctx = CtxPair::cctx();
                    eqv("row95 compressBegin", bc(cctx.c, lvl), br(cctx.r, lvl));
                    let label = format!(
                        "row95 lvl={lvl} class={} size={size}",
                        CLASS_NAMES[class]
                    );

                    // right after compressBegin only the capacities are
                    // initialised (`sequences`/`lit` are set per block)
                    let s0c = *gsc(cctx.c as *const c_void);
                    let s0r = *gsr(cctx.r as *const c_void);
                    assert!(
                        !s0c.sequencesStart.is_null() && !s0r.sequencesStart.is_null(),
                        "{label}: fresh sequencesStart == NULL"
                    );
                    assert!(
                        !s0c.litStart.is_null() && !s0r.litStart.is_null(),
                        "{label}: fresh litStart == NULL"
                    );
                    eqv(&format!("{label} fresh maxNbSeq"), s0c.maxNbSeq, s0r.maxNbSeq);
                    eqv(&format!("{label} fresh maxNbLit"), s0c.maxNbLit, s0r.maxNbLit);

                    let cap = bound(size) + 64;
                    let mut dc = vec![0xA5u8; cap];
                    let mut dr = vec![0xA5u8; cap];
                    let rc = cbc(
                        cctx.c,
                        dc.as_mut_ptr() as *mut c_void,
                        cap,
                        src.as_ptr() as *const c_void,
                        size,
                    );
                    let rr = cbr(
                        cctx.r,
                        dr.as_mut_ptr() as *mut c_void,
                        cap,
                        src.as_ptr() as *const c_void,
                        size,
                    );
                    eqv(&format!("{label} compressBlock"), rc, rr);
                    eqbuf(&format!("{label} compressBlock dst"), &dc, &dr);
                    if is_err(rc) {
                        continue;
                    }

                    let pc = gsc(cctx.c as *const c_void);
                    let pr = gsr(cctx.r as *const c_void);
                    assert!(!pc.is_null() && !pr.is_null(), "{label}: getSeqStore == NULL");
                    let sc = *pc;
                    let sr = *pr;
                    if cmp_seqstore(&format!("{label} after block"), &sc, &sr) {
                        deep += 1;
                    }

                    // ZSTD_resetSeqStore
                    rsc(pc as *mut SeqStoreRaw);
                    rsr(pr as *mut SeqStoreRaw);
                    let sc2 = *pc;
                    let sr2 = *pr;
                    cmp_seqstore(&format!("{label} after reset"), &sc2, &sr2);
                    if !sc2.sequences.is_null() {
                        eqv(
                            &format!("{label} reset nbSeq"),
                            sc2.sequences as usize - sc2.sequencesStart as usize,
                            0,
                        );
                        eqv(
                            &format!("{label} reset litLen"),
                            sr2.lit as usize - sr2.litStart as usize,
                            0,
                        );
                    }
                    eqv(&format!("{label} reset longLengthType"), sc2.longLengthType, 0);
                    eqv(&format!("{label} reset longLengthType r"), sr2.longLengthType, 0);
                }
            }
        }

        // all 9 strategies produce structurally different seqStores: sweep them
        // through ZSTD_compressBegin_advanced (window sized from srcSize, so the
        // ultra strategies stay cheap).
        let (gpc, gpr) = duo::<FnGetParams>("ZSTD_getParams");
        let (bac, bar) = duo::<FnBeginAdv>("ZSTD_compressBegin_advanced");
        for &strategy in ALL_STRATEGIES.iter() {
            for class in 0..N_CLASSES {
                for &size in [300usize, 9000, 131_072].iter() {
                    for &lvl in [1i32, 12, 22].iter() {
                        let src = gen_class(class, size, 0x9502 ^ strategy as u64);
                        let mut p = gpc(lvl, size as c_ulonglong, 0);
                        eqv("row95 getParams", p, gpr(lvl, size as c_ulonglong, 0));
                        p.cParams.strategy = strategy as c_uint;
                        let cctx = CtxPair::cctx();
                        let label = format!(
                            "row95 strat={strategy} lvl={lvl} class={} size={size}",
                            CLASS_NAMES[class]
                        );
                        eqv(
                            &format!("{label} compressBegin_advanced"),
                            bac(cctx.c, std::ptr::null(), 0, p, ZSTD_CONTENTSIZE_UNKNOWN),
                            bar(cctx.r, std::ptr::null(), 0, p, ZSTD_CONTENTSIZE_UNKNOWN),
                        );
                        let cap = bound(size) + 64;
                        let mut dc = vec![0xA5u8; cap];
                        let mut dr = vec![0xA5u8; cap];
                        let rc = cbc(
                            cctx.c,
                            dc.as_mut_ptr() as *mut c_void,
                            cap,
                            src.as_ptr() as *const c_void,
                            size,
                        );
                        let rr = cbr(
                            cctx.r,
                            dr.as_mut_ptr() as *mut c_void,
                            cap,
                            src.as_ptr() as *const c_void,
                            size,
                        );
                        eqv(&format!("{label} compressBlock"), rc, rr);
                        eqbuf(&format!("{label} compressBlock dst"), &dc, &dr);
                        if is_err(rc) {
                            continue;
                        }
                        let pc = gsc(cctx.c as *const c_void);
                        let pr = gsr(cctx.r as *const c_void);
                        let (sc, sr) = (*pc, *pr);
                        if cmp_seqstore(&format!("{label} after block"), &sc, &sr) {
                            deep += 1;
                        }
                        rsc(pc as *mut SeqStoreRaw);
                        rsr(pr as *mut SeqStoreRaw);
                        cmp_seqstore(&format!("{label} after reset"), &*pc, &*pr);
                    }
                }
            }
        }

        assert!(deep > 50, "row95: only {deep} deep seqStore comparisons — test is vacuous");

        // ZSTD_reset_compressedBlockState over a byte-identical scratch buffer
        for round in 0..16u64 {
            let mut wc: Vec<u64> = vec![0; SIZEOF_CBLOCKSTATE / 8];
            for w in wc.iter_mut() {
                *w = rng.next_u64();
            }
            let mut wr = wc.clone();
            rbc(wc.as_mut_ptr() as *mut c_void);
            rbr(wr.as_mut_ptr() as *mut c_void);
            let bc_ = std::slice::from_raw_parts(wc.as_ptr() as *const u8, SIZEOF_CBLOCKSTATE);
            let br_ = std::slice::from_raw_parts(wr.as_ptr() as *const u8, SIZEOF_CBLOCKSTATE);
            eqbuf(&format!("row95 reset_compressedBlockState round={round}"), bc_, br_);
        }
    }
}

/// Compare everything about a `SeqStore_t` that is meaningful across two
/// separate address spaces: the scalar fields, the number of stored sequences
/// and literals, and the bytes of the sequence / literal arrays themselves.
/// Returns `true` when the deep (array) comparison was possible.
#[track_caller]
unsafe fn cmp_seqstore(label: &str, c: &SeqStoreRaw, r: &SeqStoreRaw) -> bool {
    assert!(
        !c.sequencesStart.is_null() && !r.sequencesStart.is_null(),
        "{label}: sequencesStart == NULL"
    );
    assert!(!c.litStart.is_null() && !r.litStart.is_null(), "{label}: litStart == NULL");
    eqv(&format!("{label} maxNbSeq"), c.maxNbSeq, r.maxNbSeq);
    eqv(&format!("{label} maxNbLit"), c.maxNbLit, r.maxNbLit);
    eqv(&format!("{label} longLengthType"), c.longLengthType, r.longLengthType);
    eqv(&format!("{label} longLengthPos"), c.longLengthPos, r.longLengthPos);
    // `sequences` / `lit` are only positioned when a block really went through
    // the match finder; before that they are still the NULL left by calloc.
    eqv(
        &format!("{label} sequences-set"),
        c.sequences.is_null(),
        r.sequences.is_null(),
    );
    eqv(&format!("{label} lit-set"), c.lit.is_null(), r.lit.is_null());
    if c.sequences.is_null() || c.lit.is_null() {
        return false;
    }
    let sz = std::mem::size_of::<SeqDef>() as isize;
    let nseq_c = (c.sequences as isize - c.sequencesStart as isize) / sz;
    let nseq_r = (r.sequences as isize - r.sequencesStart as isize) / sz;
    eqv(&format!("{label} nbSeq"), nseq_c, nseq_r);
    let nlit_c = c.lit as isize - c.litStart as isize;
    let nlit_r = r.lit as isize - r.litStart as isize;
    eqv(&format!("{label} litLength"), nlit_c, nlit_r);
    assert!(nseq_c >= 0 && nlit_c >= 0, "{label}: negative counts");
    assert!(nseq_c as usize <= c.maxNbSeq + 1, "{label}: nbSeq beyond maxNbSeq");
    assert!(nlit_c as usize <= c.maxNbLit + 64, "{label}: literals beyond maxNbLit");
    let sc = std::slice::from_raw_parts(c.sequencesStart as *const u8, nseq_c as usize * 8);
    let sr = std::slice::from_raw_parts(r.sequencesStart as *const u8, nseq_r as usize * 8);
    eqbuf(&format!("{label} sequences"), sc, sr);
    let lc = std::slice::from_raw_parts(c.litStart as *const u8, nlit_c as usize);
    let lr = std::slice::from_raw_parts(r.litStart as *const u8, nlit_r as usize);
    eqbuf(&format!("{label} literals"), lc, lr);
    true
}

// ================================================================ row 96

/// `ZSTD_get1BlockSummary` only fills `blockSize`/`litSize` on success — in the
/// error path the C leaves them as uninitialised stack slots, so only
/// `nbSequences` is comparable there.
#[track_caller]
fn cmp_summary(label: &str, c: BlockSummary, r: BlockSummary) {
    eqv(&format!("{label} nbSequences"), c.nbSequences, r.nbSequences);
    if !is_err(c.nbSequences) {
        eqv(&format!("{label} blockSize"), c.blockSize, r.blockSize);
        eqv(&format!("{label} litSize"), c.litSize, r.litSize);
    }
}

#[test]
fn row96_split_block_and_block_summary() {
    unsafe {
        let (sc, sr) = duo::<FnSplitBlock>("ZSTD_splitBlock");
        let (bsc, bsr) = duo::<FnBlockSummary>("ZSTD_get1BlockSummary");
        let mut rng = Rng::new(0x9600_0001);

        // ---- ZSTD_splitBlock: level 0..4 × mixed-entropy 128KB blocks ----
        let mut inputs: Vec<(String, Vec<u8>)> = Vec::new();
        for class in 0..N_CLASSES {
            inputs.push((
                format!("uniform-{}", CLASS_NAMES[class]),
                gen_class(class, ZSTD_BLOCKSIZE_MAX, 0x9601),
            ));
        }
        // mixed: two different classes glued at a random boundary
        for i in 0..24usize {
            let a = rng.below(N_CLASSES);
            let b = rng.below(N_CLASSES);
            let cut = 8 * 1024 * (1 + rng.below(15));
            let mut v = gen_class(a, cut, 0x9602 + i as u64);
            v.extend_from_slice(&gen_class(b, ZSTD_BLOCKSIZE_MAX - cut, 0x9603 + i as u64));
            inputs.push((format!("mix-{a}-{b}-at-{cut}"), v));
        }
        for (nm, data) in inputs.iter() {
            assert_eq!(data.len(), ZSTD_BLOCKSIZE_MAX);
            for level in 0..5i32 {
                for &wsz in [SLIPBLOCK_WKSP, SLIPBLOCK_WKSP + 1024].iter() {
                    let mut wc: Vec<u64> = vec![0xA5A5_A5A5_A5A5_A5A5; (wsz + 7) / 8];
                    let mut wr = wc.clone();
                    let a = sc(
                        data.as_ptr() as *const c_void,
                        data.len(),
                        level,
                        wc.as_mut_ptr() as *mut c_void,
                        wsz,
                    );
                    let b = sr(
                        data.as_ptr() as *const c_void,
                        data.len(),
                        level,
                        wr.as_mut_ptr() as *mut c_void,
                        wsz,
                    );
                    eqv(&format!("row96 splitBlock {nm} level={level} wsz={wsz}"), a, b);
                    assert!(a > 0 && a <= data.len(), "row96 splitBlock returned {a}");
                }
            }
        }

        // ---- the same knob through the real compression path -------------
        {
            let (gpc, _) = duo::<FnGetParams>("ZSTD_getParams");
            let (iac, iar) = duo::<FnParamsInitAdv>("ZSTD_CCtxParams_init_advanced");
            let (spc, spr) = duo::<FnSetParam>("ZSTD_CCtxParams_setParameter");
            let (bc, br) = duo::<FnBeginAdvInt>("ZSTD_compressBegin_advanced_internal");
            for splitter in 0..7i32 {
                for &after in [0i32, 1, 2].iter() {
                    for &lvl in [1i32, 5, 12].iter() {
                        let mut src = gen_class(4, 90_000, 0x9604);
                        src.extend_from_slice(&gen_class(3, 60_000, 0x9605));
                        src.extend_from_slice(&gen_class(6, 120_000, 0x9606));
                        let params = CtxPair::cctx_params();
                        let p = gpc(lvl, src.len() as c_ulonglong, 0);
                        eqv("row96 init_advanced", iac(params.c, p), iar(params.r, p));
                        eqv(
                            "row96 set blockSplitterLevel",
                            spc(params.c, ZSTD_c_blockSplitterLevel, splitter),
                            spr(params.r, ZSTD_c_blockSplitterLevel, splitter),
                        );
                        eqv(
                            "row96 set splitAfterSequences",
                            spc(params.c, ZSTD_c_splitAfterSequences, after),
                            spr(params.r, ZSTD_c_splitAfterSequences, after),
                        );
                        let ctx = CtxPair::cctx();
                        let label =
                            format!("row96 path splitter={splitter} after={after} lvl={lvl}");
                        eqv(
                            &format!("{label} begin"),
                            bc(
                                ctx.c,
                                std::ptr::null(),
                                0,
                                ZSTD_dct_auto,
                                ZSTD_dtlm_fast,
                                std::ptr::null(),
                                params.c as *const c_void,
                                src.len() as c_ulonglong,
                            ),
                            br(
                                ctx.r,
                                std::ptr::null(),
                                0,
                                ZSTD_dct_auto,
                                ZSTD_dtlm_fast,
                                std::ptr::null(),
                                params.r as *const c_void,
                                src.len() as c_ulonglong,
                            ),
                        );
                        let chunks = chunks_for(&mut rng, src.len());
                        if let Some(frame) = run_session(
                            &label,
                            &ctx,
                            "ZSTD_compressContinue",
                            "ZSTD_compressEnd",
                            &src,
                            &chunks,
                        ) {
                            check_roundtrip(&label, &frame, &src);
                        }
                    }
                }
            }
        }

        // ---- ZSTD_get1BlockSummary --------------------------------------
        for case in 0..400usize {
            let n = 1 + rng.below(40);
            let mut seqs: Vec<ZSTD_Sequence> = Vec::with_capacity(n + 1);
            // a random number of real sequences, then the end-of-block marker
            let real = rng.below(n);
            for _ in 0..real {
                seqs.push(ZSTD_Sequence {
                    offset: 1 + rng.next_u32() % 4096,
                    litLength: rng.next_u32() % 1000,
                    matchLength: 3 + rng.next_u32() % 500,
                    rep: rng.next_u32() % 4,
                });
            }
            let terminated = case % 5 != 0;
            if terminated {
                let ll = rng.next_u32() % 1000;
                seqs.push(ZSTD_Sequence { offset: 0, litLength: ll, matchLength: 0, rep: 0 });
                // trailing garbage after the terminator must be ignored
                while seqs.len() < n + 1 {
                    seqs.push(ZSTD_Sequence {
                        offset: 1 + rng.next_u32() % 100,
                        litLength: rng.next_u32() % 100,
                        matchLength: 3 + rng.next_u32() % 100,
                        rep: 0,
                    });
                }
            } else {
                while seqs.len() < n {
                    seqs.push(ZSTD_Sequence {
                        offset: 1 + rng.next_u32() % 100,
                        litLength: rng.next_u32() % 100,
                        matchLength: 3 + rng.next_u32() % 100,
                        rep: 0,
                    });
                }
            }
            let a = bsc(seqs.as_ptr(), seqs.len());
            let b = bsr(seqs.as_ptr(), seqs.len());
            let lbl =
                format!("row96 get1BlockSummary case={case} n={} term={terminated}", seqs.len());
            cmp_summary(&lbl, a, b);
            assert_eq!(
                is_err(a.nbSequences),
                !terminated,
                "{lbl}: unexpected error status"
            );
        }
        // and on real sequences produced by the library itself
        let (gsq, _) = duo::<
            unsafe extern "C" fn(*mut c_void, *mut ZSTD_Sequence, usize, *const c_void, usize) -> usize,
        >("ZSTD_generateSequences");
        let (create, _) = duo::<FnPtr0>("ZSTD_createCCtx");
        let (free, _) = duo::<FnFreePtr>("ZSTD_freeCCtx");
        let (setp, _) = duo::<FnSetParam>("ZSTD_CCtx_setParameter");
        let (sbnd, _) = duo::<FnSizeT1>("ZSTD_sequenceBound");
        for class in 0..N_CLASSES {
            for &size in [1000usize, 50_000, 200_000].iter() {
                let src = gen_class(class, size, 0x9607);
                let cctx = create();
                assert!(!is_err(setp(cctx, ZSTD_c_blockDelimiters, 1)));
                let mut seqs = vec![ZSTD_Sequence::default(); sbnd(size)];
                let n = gsq(
                    cctx,
                    seqs.as_mut_ptr(),
                    seqs.len(),
                    src.as_ptr() as *const c_void,
                    size,
                );
                free(cctx);
                if is_err(n) {
                    continue;
                }
                seqs.truncate(n);
                for take in [n, n.min(4), n / 2 + 1] {
                    if take == 0 {
                        continue;
                    }
                    let a = bsc(seqs.as_ptr(), take);
                    let b = bsr(seqs.as_ptr(), take);
                    cmp_summary(
                        &format!("row96 real summary class={class} size={size} take={take}"),
                        a,
                        b,
                    );
                }
            }
        }
    }
}
