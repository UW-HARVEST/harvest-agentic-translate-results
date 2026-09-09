//! Phase B: the sequence API (generate / merge / compress sequences), the
//! literals-block builders and the seqStore / entropy internals.
//!
//! Every test drives BOTH the C `libzstd.so` (ground truth) and the Rust
//! `libzstd.so` and asserts identical return values / byte-identical output.
//!
//! CONFIGS 146-155; ERRORS 119-144.
#![allow(non_snake_case, dead_code)]
mod common;
use common::*;
use std::os::raw::{c_int, c_uint, c_void};

// ------------------------------------------------------------ fn types -----

type FnNew = unsafe extern "C" fn() -> *mut c_void;
type FnFree = unsafe extern "C" fn(*mut c_void) -> usize;
type FnSetParam = unsafe extern "C" fn(*mut c_void, c_int, c_int) -> usize;
type FnGetParam = unsafe extern "C" fn(*mut c_void, c_int, *mut c_int) -> usize;
type FnBound = unsafe extern "C" fn(usize) -> usize;
type FnCompress = unsafe extern "C" fn(*mut u8, usize, *const u8, usize, c_int) -> usize;
type FnDecompress = unsafe extern "C" fn(*mut u8, usize, *const u8, usize) -> usize;
type FnBegin = unsafe extern "C" fn(*mut c_void, c_int) -> usize;
type FnChunk = unsafe extern "C" fn(*mut c_void, *mut u8, usize, *const u8, usize) -> usize;
type FnBlockSize = unsafe extern "C" fn(*const c_void) -> usize;

type FnGenerateSequences =
    unsafe extern "C" fn(*mut c_void, *mut Sequence, usize, *const u8, usize) -> usize;
type FnMergeDelims = unsafe extern "C" fn(*mut Sequence, usize) -> usize;
type FnCompressSequences = unsafe extern "C" fn(
    *mut c_void,
    *mut u8,
    usize,
    *const Sequence,
    usize,
    *const u8,
    usize,
) -> usize;
type FnCompressSeqAndLit = unsafe extern "C" fn(
    *mut c_void,
    *mut u8,
    usize,
    *const Sequence,
    usize,
    *const u8,
    usize,
    usize,
    usize,
) -> usize;
type FnConvertBlockSequences =
    unsafe extern "C" fn(*mut c_void, *const Sequence, usize, c_int) -> usize;
type FnGet1BlockSummary = unsafe extern "C" fn(*const Sequence, usize) -> BlockSummary;

type FnNoCompressLiterals = unsafe extern "C" fn(*mut u8, usize, *const u8, usize) -> usize;
type FnCompressLiterals = unsafe extern "C" fn(
    *mut u8,
    usize,
    *const u8,
    usize,
    *mut c_void,
    usize,
    *const c_void,
    *mut c_void,
    c_int,
    c_int,
    c_int,
    c_int,
) -> usize;

type FnGetSeqStore = unsafe extern "C" fn(*const c_void) -> *const SeqStore;
type FnResetSeqStore = unsafe extern "C" fn(*mut SeqStore);
type FnSeqToCodes = unsafe extern "C" fn(*const SeqStore) -> c_int;
type FnSelectEncodingType = unsafe extern "C" fn(
    *mut c_int,      // FSE_repeat* repeatMode
    *const c_uint,   // count
    c_uint,          // max
    usize,           // mostFrequent
    usize,           // nbSeq
    c_uint,          // FSELog
    *const u32,      // prevCTable
    *const i16,      // defaultNorm
    u32,             // defaultNormLog
    c_int,           // ZSTD_DefaultPolicy_e
    c_int,           // strategy
) -> c_int;
type FnBuildCTable = unsafe extern "C" fn(
    *mut u8,       // dst
    usize,         // dstCapacity
    *mut u32,      // nextCTable
    u32,           // FSELog
    c_int,         // type
    *mut c_uint,   // count (modified!)
    u32,           // max
    *const u8,     // codeTable
    usize,         // nbSeq
    *const i16,    // defaultNorm
    u32,           // defaultNormLog
    u32,           // defaultMax
    *const u32,    // prevCTable
    usize,         // prevCTableSize
    *mut c_void,   // entropyWorkspace
    usize,         // entropyWorkspaceSize
) -> usize;
type FnEncodeSequences = unsafe extern "C" fn(
    *mut u8,
    usize,
    *const u32,
    *const u8,
    *const u32,
    *const u8,
    *const u32,
    *const u8,
    *const SeqDef,
    usize,
    c_int,
    c_int,
) -> usize;
type FnFseBitCost = unsafe extern "C" fn(*const u32, *const c_uint, c_uint) -> usize;
type FnCrossEntropyCost = unsafe extern "C" fn(*const i16, c_uint, *const c_uint, c_uint) -> usize;
type FnBuildBlockEntropyStats = unsafe extern "C" fn(
    *const SeqStore,
    *const c_void, // prevEntropy
    *mut c_void,   // nextEntropy
    *const c_void, // cctxParams
    *mut c_void,   // entropyMetadata
    *mut c_void,   // workspace
    usize,
) -> usize;
type FnSplitBlock = unsafe extern "C" fn(*const u8, usize, c_int, *mut c_void, usize) -> usize;
type FnCompressSuperBlock =
    unsafe extern "C" fn(*mut c_void, *mut u8, usize, *const u8, usize, c_uint) -> usize;

// ------------------------------------------------------------- structs -----

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

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SeqDef {
    pub offBase: u32,
    pub litLength: u16,
    pub mlBase: u16,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SeqStore {
    pub sequencesStart: *mut SeqDef,
    pub sequences: *mut SeqDef,
    pub litStart: *mut u8,
    pub lit: *mut u8,
    pub llCode: *mut u8,
    pub mlCode: *mut u8,
    pub ofCode: *mut u8,
    pub maxNbSeq: usize,
    pub maxNbLit: usize,
    pub longLengthType: c_int,
    pub longLengthPos: u32,
}

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

// ----------------------------------------------------------- constants -----

const C_COMPRESSIONLEVEL: c_int = 100;
const C_WINDOWLOG: c_int = 101;
const C_MINMATCH: c_int = 105;
const C_TARGETCBLOCKSIZE: c_int = 130;
const C_CONTENTSIZEFLAG: c_int = 200;
const C_CHECKSUMFLAG: c_int = 201;
const C_NBWORKERS: c_int = 400;
const C_BLOCKDELIMITERS: c_int = 1008;
const C_VALIDATESEQUENCES: c_int = 1009;
const C_MAXBLOCKSIZE: c_int = 1015;
const C_REPCODERESOLUTION: c_int = 1016;

const MAX_LL: u32 = 35;
const MAX_ML: u32 = 52;
const MAX_OFF: u32 = 31;
const DEFAULT_MAX_OFF: u32 = 28;
const LL_FSELOG: u32 = 9;
const ML_FSELOG: u32 = 9;
const OFF_FSELOG: u32 = 8;
const LL_DEFAULTNORMLOG: u32 = 6;
const ML_DEFAULTNORMLOG: u32 = 6;
const OF_DEFAULTNORMLOG: u32 = 5;

const LL_DEFAULT_NORM: [i16; 36] = [
    4, 3, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 1, 1, 1, 2, 2, 2, 2, 2, 2, 2, 2, 2, 3, 2, 1, 1, 1, 1, 1,
    -1, -1, -1, -1,
];
const ML_DEFAULT_NORM: [i16; 53] = [
    1, 4, 3, 2, 2, 2, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, -1, -1, -1, -1, -1, -1, -1,
];
const OF_DEFAULT_NORM: [i16; 29] = [
    1, 1, 1, 1, 1, 1, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, -1, -1, -1, -1, -1,
];

// SymbolEncodingType_e
const SET_BASIC: c_int = 0;
const SET_RLE: c_int = 1;
const SET_COMPRESSED: c_int = 2;
const SET_REPEAT: c_int = 3;
// FSE_repeat
const FSE_REPEAT_NONE: c_int = 0;
const FSE_REPEAT_CHECK: c_int = 1;
const FSE_REPEAT_VALID: c_int = 2;

/// HUF_WORKSPACE_SIZE (8 KB + 512) rounded up; also >= ENTROPY_WORKSPACE_SIZE (8920).
const ENTROPY_WKSP_BYTES: usize = 16384;
/// ZSTD_FRAMEHEADERSIZE_MAX. ZSTD_writeFrameHeader refuses to write into a
/// buffer smaller than this, and the callers of it in ZSTD_compressSequences /
/// ZSTD_compressSequencesAndLiterals only `assert()` that it succeeded (so with
/// NDEBUG a smaller dstCapacity would underflow the remaining capacity).
/// Never pass a dstCapacity below this to those two entry points.
const MIN_DST_CAP: usize = 18;
/// ZSTD_SLIPBLOCK_WORKSPACESIZE == 8208.
const SPLITBLOCK_WKSP_BYTES: usize = 16384;
/// sizeof(ZSTD_hufCTables_t) == 2064; sizeof(ZSTD_entropyCTables_t) == 5616;
/// sizeof(ZSTD_entropyCTablesMetadata_t) == 312. Over-allocate (and zero) so any
/// trailing bytes stay identical in both libraries.
const HUFTABLES_BYTES: usize = 4096;
const ENTROPY_TABLES_BYTES: usize = 8192;
const METADATA_BYTES: usize = 1024;
/// FSE_CTABLE_SIZE_U32(9, 52) == 363; allocate generously.
const CTABLE_U32: usize = 1024;

// ------------------------------------------------------------- helpers -----

/// An 8-byte-aligned zeroed byte buffer.
struct Aligned(Vec<u64>);
impl Aligned {
    fn new(bytes: usize) -> Aligned {
        Aligned(vec![0u64; (bytes + 7) / 8])
    }
    fn ptr(&mut self) -> *mut c_void {
        self.0.as_mut_ptr() as *mut c_void
    }
    fn cptr(&self) -> *const c_void {
        self.0.as_ptr() as *const c_void
    }
    fn bytes(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.0.as_ptr() as *const u8, self.0.len() * 8) }
    }
}

/// A pair of CCtx (one per library) sharing the same parameter settings.
struct CctxPair {
    c: *mut c_void,
    r: *mut c_void,
    c_set: libloading::Symbol<'static, FnSetParam>,
    r_set: libloading::Symbol<'static, FnSetParam>,
    c_free: libloading::Symbol<'static, FnFree>,
    r_free: libloading::Symbol<'static, FnFree>,
}

impl CctxPair {
    fn new() -> CctxPair {
        let (cn, rn) = unsafe { pair::<FnNew>("ZSTD_createCCtx") };
        let (c_set, r_set) = unsafe { pair::<FnSetParam>("ZSTD_CCtx_setParameter") };
        let (c_free, r_free) = unsafe { pair::<FnFree>("ZSTD_freeCCtx") };
        let c = unsafe { cn() };
        let r = unsafe { rn() };
        assert!(!c.is_null() && !r.is_null());
        CctxPair { c, r, c_set, r_set, c_free, r_free }
    }
    fn set(&self, p: c_int, v: c_int) -> usize {
        let a = unsafe { (self.c_set)(self.c, p, v) };
        let b = unsafe { (self.r_set)(self.r, p, v) };
        assert_eq!(a, b, "setParameter({p},{v})");
        a
    }
    fn set_ok(&self, p: c_int, v: c_int) {
        let a = self.set(p, v);
        assert!(!is_error(a), "setParameter({p},{v}) failed: {}", err_code(a));
    }
}

impl Drop for CctxPair {
    fn drop(&mut self) {
        unsafe {
            (self.c_free)(self.c);
            (self.r_free)(self.r);
        }
    }
}

/// Generate sequences in both libraries; assert identical count and array.
/// Returns `Some(seqs)` when both succeeded.
fn generate_sequences(p: &CctxPair, src: &[u8], ctx: &str) -> Option<Vec<Sequence>> {
    let (c_sb, r_sb) = unsafe { pair::<FnBound>("ZSTD_sequenceBound") };
    let (c_gs, r_gs) = unsafe { pair::<FnGenerateSequences>("ZSTD_generateSequences") };
    let bound_c = unsafe { c_sb(src.len()) };
    let bound_r = unsafe { r_sb(src.len()) };
    assert_eq!(bound_c, bound_r, "{ctx}: ZSTD_sequenceBound({})", src.len());
    let mut cs = vec![Sequence::default(); bound_c];
    let mut rs = vec![Sequence::default(); bound_c];
    let a = unsafe { c_gs(p.c, cs.as_mut_ptr(), bound_c, src.as_ptr(), src.len()) };
    let b = unsafe { r_gs(p.r, rs.as_mut_ptr(), bound_c, src.as_ptr(), src.len()) };
    assert_eq!(a, b, "{ctx}: ZSTD_generateSequences return (C err={})", err_code(a));
    if is_error(a) {
        return None;
    }
    assert!(a <= bound_c, "{ctx}: count {a} > bound {bound_c}");
    assert_eq!(&cs[..a], &rs[..a], "{ctx}: ZSTD_generateSequences array");
    cs.truncate(a);
    Some(cs)
}

/// Extract the literals buffer implied by `seqs` from `src`, plus the total
/// decompressed size the sequences describe.
fn extract_literals(src: &[u8], seqs: &[Sequence]) -> (Vec<u8>, usize) {
    let mut lit = Vec::new();
    let mut pos = 0usize;
    for s in seqs {
        let ll = s.litLength as usize;
        assert!(pos + ll <= src.len(), "literal run out of range");
        lit.extend_from_slice(&src[pos..pos + ll]);
        pos += ll + s.matchLength as usize;
        assert!(pos <= src.len(), "sequence out of range");
    }
    (lit, pos)
}

fn decompress_both(frame: &[u8], expect: &[u8], ctx: &str) {
    let (cd, rd) = unsafe { pair::<FnDecompress>("ZSTD_decompress") };
    let cap = expect.len() + 64;
    let mut co = vec![0xAAu8; cap];
    let mut ro = vec![0x55u8; cap];
    let a = unsafe { cd(co.as_mut_ptr(), cap, frame.as_ptr(), frame.len()) };
    let b = unsafe { rd(ro.as_mut_ptr(), cap, frame.as_ptr(), frame.len()) };
    assert_eq!(a, b, "{ctx}: round-trip decompress return (C err={})", err_code(a));
    assert!(!is_error(a), "{ctx}: round-trip decompress err {}", err_code(a));
    assert_bytes_eq(&format!("{ctx}: round-trip bytes"), &co[..a], &ro[..b]);
    assert_bytes_eq(&format!("{ctx}: round-trip vs orig"), &co[..a], expect);
}

// =========================================================== CONFIG 146 ====

#[test]
fn cfg_generate_sequences() {
    // ---- ZSTD_sequenceBound
    let (c_sb, r_sb) = unsafe { pair::<FnBound>("ZSTD_sequenceBound") };
    let mut sizes: Vec<usize> = vec![0, 1, 1000, 131072, 300000];
    sizes.extend_from_slice(&[2, 3, 4, 5, 131071, 131073, 262144, 1 << 20, 1 << 24]);
    let mut rng = Rng::new(0x146);
    for _ in 0..40 {
        sizes.push(rng.below(1 << 20));
    }
    for &n in &sizes {
        let a = unsafe { c_sb(n) };
        let b = unsafe { r_sb(n) };
        assert_eq!(a, b, "ZSTD_sequenceBound({n})");
    }

    // ---- ZSTD_generateSequences
    let (c_g1, r_g1) = unsafe { pair::<FnGet1BlockSummary>("ZSTD_get1BlockSummary") };
    for &lvl in &[1, 5, 9, 19] {
        for &shape in &[Shape::Text, Shape::Random, Shape::Repetitive, Shape::Mixed] {
            for &n in &[1000usize, 131072, 300000] {
                for round in 0..2 {
                    let src = gen(shape, n, &mut rng);
                    let p = CctxPair::new();
                    p.set_ok(C_COMPRESSIONLEVEL, lvl);
                    let ctx = format!("genseq lvl={lvl} shape={shape:?} n={n} round={round}");
                    if let Some(seqs) = generate_sequences(&p, &src, &ctx) {
                        // sanity: sequences must exactly cover the source
                        let (_lit, total) = extract_literals(&src, &seqs);
                        assert_eq!(total, src.len(), "{ctx}: sequences cover srcSize");
                        // ZSTD_get1BlockSummary over every block boundary
                        // NOTE: when no delimiter is found C only sets
                        // `nbSequences` (to an error code) and leaves the other
                        // two fields uninitialized, so compare those only on
                        // success.
                        let mut off = 0usize;
                        while off < seqs.len() {
                            let a = unsafe { c_g1(seqs[off..].as_ptr(), seqs.len() - off) };
                            let b = unsafe { r_g1(seqs[off..].as_ptr(), seqs.len() - off) };
                            assert_eq!(
                                a.nbSequences, b.nbSequences,
                                "{ctx}: ZSTD_get1BlockSummary nbSequences at {off}"
                            );
                            if is_error(a.nbSequences) || a.nbSequences == 0 {
                                break;
                            }
                            assert_eq!(
                                a, b,
                                "{ctx}: ZSTD_get1BlockSummary at {off}"
                            );
                            off += a.nbSequences;
                        }
                    }
                }
            }
        }
    }
    // also tiny / empty inputs
    for &n in &[0usize, 1, 2, 3, 7, 64] {
        for &lvl in &[1, 9, 19] {
            let src = gen(Shape::Text, n, &mut rng);
            let p = CctxPair::new();
            p.set_ok(C_COMPRESSIONLEVEL, lvl);
            generate_sequences(&p, &src, &format!("genseq tiny n={n} lvl={lvl}"));
        }
    }
}

// ====================================================== ERRORS 119, 120 ====

#[test]
fn err_generateSequences_targetCBlockSize() {
    let (c_gs, r_gs) = unsafe { pair::<FnGenerateSequences>("ZSTD_generateSequences") };
    let (c_sb, _) = unsafe { pair::<FnBound>("ZSTD_sequenceBound") };
    let mut rng = Rng::new(0x119);
    let src = gen(Shape::Text, 50000, &mut rng);
    let bound = unsafe { c_sb(src.len()) };

    // ERRORS 119: targetCBlockSize != 0
    for &tcbs in &[1340, 2000, 65536, 131072] {
        let p = CctxPair::new();
        p.set_ok(C_TARGETCBLOCKSIZE, tcbs);
        let mut cs = vec![Sequence::default(); bound];
        let mut rs = vec![Sequence::default(); bound];
        let a = unsafe { c_gs(p.c, cs.as_mut_ptr(), bound, src.as_ptr(), src.len()) };
        let b = unsafe { r_gs(p.r, rs.as_mut_ptr(), bound, src.as_ptr(), src.len()) };
        assert_eq!(a, b, "targetCBlockSize={tcbs}: (C err={})", err_code(a));
        assert!(is_error(a), "targetCBlockSize={tcbs} must be rejected");
    }

    // ERRORS 120: nbWorkers != 0.  This build has no multi-threading support, so
    // ZSTD_CCtx_setParameter(ZSTD_c_nbWorkers, >0) is itself rejected; both
    // libraries must reject it identically, and generateSequences then succeeds
    // (nbWorkers stayed 0).  The generateSequences guard is therefore only
    // reachable in an MT build; verified here to behave identically anyway.
    for &nb in &[1, 2, 4] {
        let p = CctxPair::new();
        let rc = p.set(C_NBWORKERS, nb);
        let mut cs = vec![Sequence::default(); bound];
        let mut rs = vec![Sequence::default(); bound];
        let a = unsafe { c_gs(p.c, cs.as_mut_ptr(), bound, src.as_ptr(), src.len()) };
        let b = unsafe { r_gs(p.r, rs.as_mut_ptr(), bound, src.as_ptr(), src.len()) };
        assert_eq!(a, b, "nbWorkers={nb} (setParameter rc={}): (C err={})", rc, err_code(a));
        if !is_error(a) {
            assert_eq!(&cs[..a], &rs[..a], "nbWorkers={nb}: sequence array");
        }
    }
}

// ========================================================== ERRORS 121 ====

#[test]
fn err_generateSequences_capacity() {
    let (c_gs, r_gs) = unsafe { pair::<FnGenerateSequences>("ZSTD_generateSequences") };
    let (c_sb, _) = unsafe { pair::<FnBound>("ZSTD_sequenceBound") };
    let mut rng = Rng::new(0x121);

    for &n in &[1000usize, 50000, 131072, 300000] {
        let src = gen(Shape::Text, n, &mut rng);
        let bound = unsafe { c_sb(src.len()) };
        for &cap in &[0usize, 1, 2, 8, bound / 8, bound / 2, bound - 2, bound - 1] {
            if cap >= bound {
                continue;
            }
            let p = CctxPair::new();
            p.set_ok(C_COMPRESSIONLEVEL, 5);
            let mut cs = vec![Sequence::default(); cap.max(1)];
            let mut rs = vec![Sequence::default(); cap.max(1)];
            let a = unsafe { c_gs(p.c, cs.as_mut_ptr(), cap, src.as_ptr(), src.len()) };
            let b = unsafe { r_gs(p.r, rs.as_mut_ptr(), cap, src.as_ptr(), src.len()) };
            assert_eq!(a, b, "n={n} cap={cap}/{bound}: (C err={})", err_code(a));
        }
        // the exact bound must always be enough
        let p = CctxPair::new();
        p.set_ok(C_COMPRESSIONLEVEL, 5);
        let mut cs = vec![Sequence::default(); bound];
        let mut rs = vec![Sequence::default(); bound];
        let a = unsafe { c_gs(p.c, cs.as_mut_ptr(), bound, src.as_ptr(), src.len()) };
        let b = unsafe { r_gs(p.r, rs.as_mut_ptr(), bound, src.as_ptr(), src.len()) };
        assert_eq!(a, b, "n={n} exact bound: (C err={})", err_code(a));
        assert!(!is_error(a), "n={n}: exact bound should succeed");
    }
}

// ====================================================== CONFIGS 147-150 ====

#[test]
fn cfg_compress_sequences() {
    let (c_md, r_md) = unsafe { pair::<FnMergeDelims>("ZSTD_mergeBlockDelimiters") };
    let (c_cs, r_cs) = unsafe { pair::<FnCompressSequences>("ZSTD_compressSequences") };
    let (c_cb, _) = unsafe { pair::<FnBound>("ZSTD_compressBound") };
    let mut rng = Rng::new(0x147);

    for &shape in &[Shape::Text, Shape::Repetitive, Shape::Mixed, Shape::LongMatches] {
        for &n in &[1000usize, 131072, 300000] {
            let src = gen(shape, n, &mut rng);

            // sequences generated at level 5 (explicit block delimiters)
            let gp = CctxPair::new();
            gp.set_ok(C_COMPRESSIONLEVEL, 5);
            let gctx = format!("cs-gen shape={shape:?} n={n}");
            let seqs = match generate_sequences(&gp, &src, &gctx) {
                Some(s) => s,
                None => continue,
            };
            drop(gp);

            // ---- ZSTD_mergeBlockDelimiters: compare the merged array in both libs
            let mut cm = seqs.clone();
            let mut rm = seqs.clone();
            let a = unsafe { c_md(cm.as_mut_ptr(), cm.len()) };
            let b = unsafe { r_md(rm.as_mut_ptr(), rm.len()) };
            assert_eq!(a, b, "{gctx}: ZSTD_mergeBlockDelimiters return");
            assert_eq!(&cm[..a], &rm[..a], "{gctx}: merged sequence array");
            cm.truncate(a);
            let merged = cm;

            for &delims in &[0, 1] {
                let seq_in: &[Sequence] = if delims == 1 { &seqs } else { &merged };
                for &validate in &[0, 1] {
                    for &rcr in &[0, 1, 2] {
                        for &lvl in &[5, 12] {
                            for &(cks, csz) in &[(0, 1), (1, 1), (1, 0), (0, 0)] {
                                let p = CctxPair::new();
                                p.set_ok(C_COMPRESSIONLEVEL, lvl);
                                p.set_ok(C_BLOCKDELIMITERS, delims);
                                p.set_ok(C_VALIDATESEQUENCES, validate);
                                p.set_ok(C_REPCODERESOLUTION, rcr);
                                p.set_ok(C_CHECKSUMFLAG, cks);
                                p.set_ok(C_CONTENTSIZEFLAG, csz);
                                let cap = unsafe { c_cb(src.len()) } + 1024;
                                let mut co = vec![0xAAu8; cap];
                                let mut ro = vec![0x55u8; cap];
                                let ctx = format!(
                                    "{gctx} delims={delims} val={validate} rcr={rcr} lvl={lvl} cks={cks} csz={csz}"
                                );
                                let x = unsafe {
                                    c_cs(
                                        p.c,
                                        co.as_mut_ptr(),
                                        cap,
                                        seq_in.as_ptr(),
                                        seq_in.len(),
                                        src.as_ptr(),
                                        src.len(),
                                    )
                                };
                                let y = unsafe {
                                    r_cs(
                                        p.r,
                                        ro.as_mut_ptr(),
                                        cap,
                                        seq_in.as_ptr(),
                                        seq_in.len(),
                                        src.as_ptr(),
                                        src.len(),
                                    )
                                };
                                assert_eq!(
                                    x, y,
                                    "{ctx}: compressSequences return (C err={})",
                                    err_code(x)
                                );
                                if is_error(x) {
                                    continue;
                                }
                                assert_bytes_eq(
                                    &format!("{ctx}: compressSequences bytes"),
                                    &co[..x],
                                    &ro[..y],
                                );
                                decompress_both(&co[..x], &src, &ctx);
                            }
                        }
                    }
                }
            }
        }
    }
}

// ================================================ ERRORS 125-132 ==========

/// Hand-crafted invalid sequence arrays.
#[test]
fn err_sequences_invalid() {
    let (c_cs, r_cs) = unsafe { pair::<FnCompressSequences>("ZSTD_compressSequences") };
    let (c_cb, _) = unsafe { pair::<FnBound>("ZSTD_compressBound") };
    let mut rng = Rng::new(0x125);

    // helper: run compressSequences on both libs with a given parameter set
    let run = |params: &[(c_int, c_int)], seqs: &[Sequence], src: &[u8], ctx: &str| {
        let p = CctxPair::new();
        for &(k, v) in params {
            p.set_ok(k, v);
        }
        let cap = unsafe { c_cb(src.len()) } + 1024;
        let mut co = vec![0xAAu8; cap];
        let mut ro = vec![0x55u8; cap];
        let x = unsafe {
            c_cs(p.c, co.as_mut_ptr(), cap, seqs.as_ptr(), seqs.len(), src.as_ptr(), src.len())
        };
        let y = unsafe {
            r_cs(p.r, ro.as_mut_ptr(), cap, seqs.as_ptr(), seqs.len(), src.as_ptr(), src.len())
        };
        assert_eq!(x, y, "{ctx}: compressSequences return (C err={})", err_code(x));
        if !is_error(x) {
            assert_bytes_eq(&format!("{ctx}: bytes"), &co[..x], &ro[..y]);
        }
        x
    };

    let src = gen(Shape::Text, 200000, &mut rng);
    let gp = CctxPair::new();
    gp.set_ok(C_COMPRESSIONLEVEL, 5);
    let seqs = generate_sequences(&gp, &src, "invalid-base").expect("base sequences");
    drop(gp);
    let explicit: Vec<(c_int, c_int)> = vec![(C_COMPRESSIONLEVEL, 5), (C_BLOCKDELIMITERS, 1)];

    // sanity: the untouched array compresses fine
    let rc = run(&explicit, &seqs, &src, "baseline");
    assert!(!is_error(rc), "baseline must succeed, got {}", err_code(rc));

    // ---- ERRORS 125: no block delimiter before the end of the array
    // Truncate the array right before its first delimiter.
    let first_delim = seqs
        .iter()
        .position(|s| s.offset == 0 && s.matchLength == 0)
        .expect("a delimiter exists");
    assert!(first_delim > 0);
    for cut in [first_delim, (first_delim / 2).max(1), 1] {
        let sub = &seqs[..cut];
        if sub.iter().any(|s| s.offset == 0 && s.matchLength == 0) {
            continue;
        }
        let rc = run(&explicit, sub, &src, &format!("no delimiter cut={cut}"));
        assert!(is_error(rc), "missing delimiter must fail (cut={cut})");
    }

    // ---- ERRORS 126: offset == 0 but matchLength != 0
    for &k in &[0usize, 1, 5, 50] {
        if k >= first_delim {
            continue;
        }
        let mut s = seqs.clone();
        s[k].offset = 0;
        s[k].matchLength = 7;
        let rc = run(&explicit, &s, &src, &format!("bad delimiter at {k}"));
        assert!(is_error(rc), "offset==0 with matchLength!=0 must fail (k={k})");
    }

    // ---- ERRORS 127: an explicit block larger than blockSizeMax
    for &mbs in &[1024, 2048, 4096] {
        let params: Vec<(c_int, c_int)> =
            vec![(C_COMPRESSIONLEVEL, 5), (C_BLOCKDELIMITERS, 1), (C_MAXBLOCKSIZE, mbs)];
        let rc = run(&params, &seqs, &src, &format!("block too large mbs={mbs}"));
        assert!(is_error(rc), "block > blockSizeMax must fail (mbs={mbs})");
    }

    // ---- ERRORS 128: sum of sequence lengths beyond srcSize
    for &div in &[2usize, 4, 8] {
        let short = &src[..src.len() / div];
        let rc = run(&explicit, &seqs, short, &format!("sum > srcSize div={div}"));
        assert!(is_error(rc), "lengths beyond srcSize must fail (div={div})");
    }

    // ---- ERRORS 129: more sequences in a block than seqStore.maxNbSeq.
    // windowLog 10 + minMatch 4 => maxNbSeq = min(1024, srcSize)/4.
    {
        let small = gen(Shape::Text, 900, &mut rng);
        let mut many: Vec<Sequence> = Vec::new();
        for _ in 0..300 {
            many.push(Sequence { offset: 1, litLength: 0, matchLength: 3, rep: 0 });
        }
        many.push(Sequence { offset: 0, litLength: 0, matchLength: 0, rep: 0 });
        let params: Vec<(c_int, c_int)> = vec![
            (C_COMPRESSIONLEVEL, 5),
            (C_BLOCKDELIMITERS, 1),
            (C_WINDOWLOG, 10),
            (C_MINMATCH, 4),
            (C_MAXBLOCKSIZE, 1024),
        ];
        let rc = run(&params, &many, &small, "too many sequences");
        assert!(is_error(rc), "too many sequences must fail");
    }

    // ---- ERRORS 130: consumed bytes != declared blockSize.
    // Shrink one match length: the delimiter's litLength then no longer makes the
    // block add up (validation off so the length itself is accepted).
    for &k in &[1usize, 3, 9] {
        if k >= first_delim {
            continue;
        }
        let mut s = seqs.clone();
        if s[k].matchLength <= 4 {
            continue;
        }
        // keep the block sum identical but move bytes past the delimiter
        s[k].matchLength -= 1;
        let rc = run(&explicit, &s, &src, &format!("blocksize mismatch k={k}"));
        assert!(is_error(rc), "block size mismatch must fail (k={k})");
    }

    // ---- ERRORS 131: offset beyond the window (validateSequences = 1)
    let validated: Vec<(c_int, c_int)> = vec![
        (C_COMPRESSIONLEVEL, 5),
        (C_BLOCKDELIMITERS, 1),
        (C_VALIDATESEQUENCES, 1),
        (C_WINDOWLOG, 17),
    ];
    for &k in &[0usize, 2, 11] {
        if k >= first_delim {
            continue;
        }
        let mut s = seqs.clone();
        s[k].offset = 1 << 30;
        let rc = run(&validated, &s, &src, &format!("offset too large k={k}"));
        assert!(is_error(rc), "huge offset must fail (k={k})");
    }
    // offset == 0 with matchLength == 0 in the middle is a *legal* delimiter, but
    // an offset of 0 with a non-zero matchLength is not (covered above).

    // ---- ERRORS 132: matchLength < 3 (validateSequences = 1)
    for &ml in &[0u32, 1, 2] {
        for &k in &[0usize, 4] {
            if k >= first_delim {
                continue;
            }
            let mut s = seqs.clone();
            let delta = s[k].matchLength - ml;
            s[k].matchLength = ml;
            // give the removed bytes to the block delimiter so the block still adds up
            s[first_delim].litLength += delta;
            let rc = run(&validated, &s, &src, &format!("matchLength={ml} k={k}"));
            assert!(is_error(rc), "matchLength {ml} must fail (k={k})");
        }
    }
    // and with minMatch > 3, matchLength == 3 is also rejected
    {
        let params: Vec<(c_int, c_int)> = vec![
            (C_COMPRESSIONLEVEL, 5),
            (C_BLOCKDELIMITERS, 1),
            (C_VALIDATESEQUENCES, 1),
            (C_MINMATCH, 5),
        ];
        let mut s = seqs.clone();
        let delta = s[0].matchLength - 3;
        s[0].matchLength = 3;
        s[first_delim].litLength += delta;
        let rc = run(&params, &s, &src, "matchLength=3 with minMatch=5");
        assert!(is_error(rc), "matchLength 3 with minMatch 5 must fail");
    }
}

// ================================================ ERRORS 122, 123, 124 ====

#[test]
fn err_compressSequences_dst() {
    let (c_cs, r_cs) = unsafe { pair::<FnCompressSequences>("ZSTD_compressSequences") };
    let (c_cb, _) = unsafe { pair::<FnBound>("ZSTD_compressBound") };
    let (c_fhs, _) = unsafe { pair::<FnBound2>("ZSTD_frameHeaderSize") };
    let mut rng = Rng::new(0x122);

    // Sweep dstCapacity from the exact frame-header size up past the successful
    // compressed size. This covers ERRORS 122 (empty frame, dstCapacity < 4),
    // 123 (dstCapacity < blockHeaderSize) and 124 (no room for the checksum).
    // Capacities below the frame header size are not exercised: the C code
    // asserts `frameHeaderSize <= dstCapacity` and, with NDEBUG, would underflow.
    for &(shape, n) in &[
        (Shape::Text, 0usize),
        (Shape::Text, 1),
        (Shape::Text, 3),
        (Shape::Text, 1000),
        (Shape::Repetitive, 20000),
        (Shape::Mixed, 60000),
    ] {
        let src = gen(shape, n, &mut rng);
        let gp = CctxPair::new();
        gp.set_ok(C_COMPRESSIONLEVEL, 5);
        let seqs = match generate_sequences(&gp, &src, "dst-base") {
            Some(s) => s,
            None => continue,
        };
        drop(gp);

        for &cks in &[0, 1] {
            // first a successful run to learn the frame header / total size
            let p = CctxPair::new();
            p.set_ok(C_COMPRESSIONLEVEL, 5);
            p.set_ok(C_BLOCKDELIMITERS, 1);
            p.set_ok(C_CHECKSUMFLAG, cks);
            let cap = unsafe { c_cb(src.len()) } + 1024;
            let mut co = vec![0u8; cap];
            let mut ro = vec![0u8; cap];
            let full = unsafe {
                c_cs(p.c, co.as_mut_ptr(), cap, seqs.as_ptr(), seqs.len(), src.as_ptr(), src.len())
            };
            let full_r = unsafe {
                r_cs(p.r, ro.as_mut_ptr(), cap, seqs.as_ptr(), seqs.len(), src.as_ptr(), src.len())
            };
            assert_eq!(full, full_r, "n={n} cks={cks}: baseline return");
            if is_error(full) {
                continue;
            }
            let fhs = unsafe { c_fhs(co.as_ptr(), full) };
            assert!(!is_error(fhs));

            let lo = fhs.max(MIN_DST_CAP);
            let mut caps: Vec<usize> = (lo..=(lo + 8)).collect();
            for extra in [0usize, 1, 2, 3, 4, 5] {
                if full > extra {
                    caps.push(full - extra);
                }
            }
            caps.push(full);
            caps.push(full + 1);
            caps.push(full / 2 + lo);
            for &c in &caps {
                if c < MIN_DST_CAP {
                    continue;
                }
                let p = CctxPair::new();
                p.set_ok(C_COMPRESSIONLEVEL, 5);
                p.set_ok(C_BLOCKDELIMITERS, 1);
                p.set_ok(C_CHECKSUMFLAG, cks);
                let mut co = vec![0xAAu8; c.max(1)];
                let mut ro = vec![0x55u8; c.max(1)];
                let x = unsafe {
                    c_cs(p.c, co.as_mut_ptr(), c, seqs.as_ptr(), seqs.len(), src.as_ptr(), src.len())
                };
                let y = unsafe {
                    r_cs(p.r, ro.as_mut_ptr(), c, seqs.as_ptr(), seqs.len(), src.as_ptr(), src.len())
                };
                assert_eq!(
                    x, y,
                    "n={n} cks={cks} cap={c} (full={full}, fhs={fhs}): return (C err={})",
                    err_code(x)
                );
                if !is_error(x) {
                    assert_bytes_eq(
                        &format!("n={n} cks={cks} cap={c}: bytes"),
                        &co[..x],
                        &ro[..y],
                    );
                }
            }
        }
    }
}

type FnBound2 = unsafe extern "C" fn(*const u8, usize) -> usize;

// =========================================================== CONFIG 151 ====

#[test]
fn cfg_compress_sequences_and_literals() {
    let (c_sl, r_sl) =
        unsafe { pair::<FnCompressSeqAndLit>("ZSTD_compressSequencesAndLiterals") };
    let (c_cb, _) = unsafe { pair::<FnBound>("ZSTD_compressBound") };
    let mut rng = Rng::new(0x151);

    let mut any_ok = false;
    for &shape in &[Shape::Text, Shape::Repetitive, Shape::LongMatches, Shape::Mixed] {
        for &n in &[1000usize, 131072, 200000] {
            for &lvl in &[3, 5, 9, 12] {
                let src = gen(shape, n, &mut rng);
                let gp = CctxPair::new();
                gp.set_ok(C_COMPRESSIONLEVEL, lvl);
                let gctx = format!("sal shape={shape:?} n={n} lvl={lvl}");
                let seqs = match generate_sequences(&gp, &src, &gctx) {
                    Some(s) => s,
                    None => continue,
                };
                drop(gp);
                let (lit, total) = extract_literals(&src, &seqs);
                assert_eq!(total, src.len());
                // litBufCapacity must be at least litSize + 8 (the encoder may
                // read up to 8 bytes past litSize).
                let mut litbuf = lit.clone();
                litbuf.extend_from_slice(&[0u8; 8]);
                let litcap = litbuf.len();

                for &rcr in &[0, 1, 2] {
                    let p = CctxPair::new();
                    p.set_ok(C_COMPRESSIONLEVEL, lvl);
                    p.set_ok(C_BLOCKDELIMITERS, 1); // explicit delimiters required
                    p.set_ok(C_REPCODERESOLUTION, rcr);
                    let cap = unsafe { c_cb(src.len()) } + 1024;
                    let mut co = vec![0xAAu8; cap];
                    let mut ro = vec![0x55u8; cap];
                    let ctx = format!("{gctx} rcr={rcr}");
                    let x = unsafe {
                        c_sl(
                            p.c,
                            co.as_mut_ptr(),
                            cap,
                            seqs.as_ptr(),
                            seqs.len(),
                            litbuf.as_ptr(),
                            lit.len(),
                            litcap,
                            src.len(),
                        )
                    };
                    let y = unsafe {
                        r_sl(
                            p.r,
                            ro.as_mut_ptr(),
                            cap,
                            seqs.as_ptr(),
                            seqs.len(),
                            litbuf.as_ptr(),
                            lit.len(),
                            litcap,
                            src.len(),
                        )
                    };
                    assert_eq!(
                        x, y,
                        "{ctx}: compressSequencesAndLiterals return (C err={})",
                        err_code(x)
                    );
                    if is_error(x) {
                        continue;
                    }
                    any_ok = true;
                    assert_bytes_eq(&format!("{ctx}: bytes"), &co[..x], &ro[..y]);
                    decompress_both(&co[..x], &src, &ctx);
                }
            }
        }
    }
    assert!(any_ok, "no ZSTD_compressSequencesAndLiterals happy path succeeded");
}

// ====================================================== ERRORS 133-143 ====

#[test]
fn err_seq_and_literals() {
    let (c_sl, r_sl) =
        unsafe { pair::<FnCompressSeqAndLit>("ZSTD_compressSequencesAndLiterals") };
    let (c_cb, _) = unsafe { pair::<FnBound>("ZSTD_compressBound") };
    let mut rng = Rng::new(0x133);

    // ---- build a valid (sequences, literals) pair from a compressible source
    let src = gen(Shape::Text, 150000, &mut rng);
    let gp = CctxPair::new();
    gp.set_ok(C_COMPRESSIONLEVEL, 5);
    let seqs = generate_sequences(&gp, &src, "sal-err-base").expect("base sequences");
    drop(gp);
    let (lit, total) = extract_literals(&src, &seqs);
    assert_eq!(total, src.len());
    let mut litbuf = lit.clone();
    litbuf.extend_from_slice(&[0u8; 8]);

    let run = |params: &[(c_int, c_int)],
               seqs: &[Sequence],
               litbuf: &[u8],
               litSize: usize,
               litCap: usize,
               dstCap: Option<usize>,
               decompressedSize: usize,
               ctx: &str| {
        let p = CctxPair::new();
        for &(k, v) in params {
            p.set_ok(k, v);
        }
        let cap = dstCap.unwrap_or_else(|| unsafe { c_cb(decompressedSize) } + 1024);
        let mut co = vec![0xAAu8; cap.max(1)];
        let mut ro = vec![0x55u8; cap.max(1)];
        let x = unsafe {
            c_sl(
                p.c,
                co.as_mut_ptr(),
                cap,
                seqs.as_ptr(),
                seqs.len(),
                litbuf.as_ptr(),
                litSize,
                litCap,
                decompressedSize,
            )
        };
        let y = unsafe {
            r_sl(
                p.r,
                ro.as_mut_ptr(),
                cap,
                seqs.as_ptr(),
                seqs.len(),
                litbuf.as_ptr(),
                litSize,
                litCap,
                decompressedSize,
            )
        };
        assert_eq!(x, y, "{ctx}: return (C err={})", err_code(x));
        if !is_error(x) {
            assert_bytes_eq(&format!("{ctx}: bytes"), &co[..x], &ro[..y]);
        }
        x
    };

    let base: Vec<(c_int, c_int)> = vec![(C_COMPRESSIONLEVEL, 5), (C_BLOCKDELIMITERS, 1)];

    // sanity
    let rc = run(&base, &seqs, &litbuf, lit.len(), litbuf.len(), None, src.len(), "baseline");
    assert!(!is_error(rc), "baseline must succeed, got {}", err_code(rc));

    // ---- ERRORS 133: litCapacity < litSize
    for &sub in &[1usize, 2, 8, 100] {
        if sub > lit.len() {
            continue;
        }
        let rc = run(
            &base,
            &seqs,
            &litbuf,
            lit.len(),
            lit.len() - sub,
            None,
            src.len(),
            &format!("litCap short by {sub}"),
        );
        assert!(is_error(rc), "litCapacity < litSize must fail");
    }

    // ---- ERRORS 134: blockDelimiters == noBlockDelimiters
    {
        let params: Vec<(c_int, c_int)> = vec![(C_COMPRESSIONLEVEL, 5), (C_BLOCKDELIMITERS, 0)];
        let rc = run(&params, &seqs, &litbuf, lit.len(), litbuf.len(), None, src.len(), "noDelims");
        assert!(is_error(rc), "noBlockDelimiters must fail");
    }

    // ---- ERRORS 135: validateSequences enabled
    {
        let params: Vec<(c_int, c_int)> = vec![
            (C_COMPRESSIONLEVEL, 5),
            (C_BLOCKDELIMITERS, 1),
            (C_VALIDATESEQUENCES, 1),
        ];
        let rc = run(&params, &seqs, &litbuf, lit.len(), litbuf.len(), None, src.len(), "validate");
        assert!(is_error(rc), "validateSequences must fail");
    }

    // ---- ERRORS 136: checksumFlag enabled
    {
        let params: Vec<(c_int, c_int)> =
            vec![(C_COMPRESSIONLEVEL, 5), (C_BLOCKDELIMITERS, 1), (C_CHECKSUMFLAG, 1)];
        let rc = run(&params, &seqs, &litbuf, lit.len(), litbuf.len(), None, src.len(), "checksum");
        assert!(is_error(rc), "checksumFlag must fail");
    }

    // ---- ERRORS 137: inSeqsSize == 0
    {
        let rc = run(&base, &[], &litbuf, lit.len(), litbuf.len(), None, src.len(), "nbSeq=0");
        assert!(is_error(rc), "inSeqsSize == 0 must fail");
    }

    // ---- ERRORS 138: the "empty frame" branch with dstCapacity < 3.
    // Note: this exact branch is NOT reachable through the public entry point in
    // this build.  ZSTD_writeFrameHeader runs first and demands
    // dstCapacity >= ZSTD_FRAMEHEADERSIZE_MAX (18), so by the time the empty
    // frame branch runs there are always >= 18 - headerSize >= 3 bytes left.
    // What *is* reachable is the empty-frame sequence array itself: with
    // decompressedSize == 0 the seqStore is sized for a 1-byte window, giving
    // maxNbSeq == 0, so ZSTD_convertBlockSequences rejects it with
    // externalSequences_invalid.  Both libraries must agree on that, and on
    // every dstCapacity from the minimum upwards.
    {
        let empty = [Sequence { offset: 0, litLength: 0, matchLength: 0, rep: 0 }];
        let zero: [u8; 8] = [0; 8];
        for cap in MIN_DST_CAP..=(MIN_DST_CAP + 12) {
            run(
                &base,
                &empty,
                &zero,
                0,
                8,
                Some(cap),
                0,
                &format!("empty frame dstCap={cap}"),
            );
        }
        // and with a non-zero decompressedSize the same array is still invalid
        for &ds in &[1usize, 8, 1000] {
            run(&base, &empty, &zero, 0, 8, None, ds, &format!("empty seqs ds={ds}"));
        }
    }

    // ---- ERRORS 139: a block's summed litLength > litSize
    for &sub in &[1usize, 5, 1000] {
        if sub >= lit.len() {
            continue;
        }
        let rc = run(
            &base,
            &seqs,
            &litbuf,
            lit.len() - sub,
            litbuf.len(),
            None,
            src.len(),
            &format!("litSize short by {sub}"),
        );
        assert!(is_error(rc), "litLength sum > litSize must fail");
    }

    // ---- ERRORS 140: dstCapacity < blockHeaderSize (sweep small capacities)
    {
        let full = run(&base, &seqs, &litbuf, lit.len(), litbuf.len(), None, src.len(), "full");
        assert!(!is_error(full));
        let (c_fhs, _) = unsafe { pair::<FnBound2>("ZSTD_frameHeaderSize") };
        // recompute the frame to read its header size
        let p = CctxPair::new();
        p.set_ok(C_COMPRESSIONLEVEL, 5);
        p.set_ok(C_BLOCKDELIMITERS, 1);
        let cap = unsafe { c_cb(src.len()) } + 1024;
        let mut co = vec![0u8; cap];
        let n = unsafe {
            c_sl(
                p.c,
                co.as_mut_ptr(),
                cap,
                seqs.as_ptr(),
                seqs.len(),
                litbuf.as_ptr(),
                lit.len(),
                litbuf.len(),
                src.len(),
            )
        };
        assert!(!is_error(n));
        let fhs = unsafe { c_fhs(co.as_ptr(), n) };
        assert!(!is_error(fhs));
        let lo = fhs.max(MIN_DST_CAP);
        let mut caps: Vec<usize> = (lo..=(lo + 6)).collect();
        for e in [1usize, 2, 3, 10, 100] {
            if n > e {
                caps.push(n - e);
            }
        }
        for &c in caps.iter().filter(|&&c| c >= MIN_DST_CAP) {
            run(
                &base,
                &seqs,
                &litbuf,
                lit.len(),
                litbuf.len(),
                Some(c),
                src.len(),
                &format!("dstCap={c} (full={n}, fhs={fhs})"),
            );
        }
    }

    // ---- ERRORS 141: a block is incompressible => cannotProduce_uncompressedBlock
    {
        let rsrc = gen(Shape::Random, 150000, &mut rng);
        let gp = CctxPair::new();
        gp.set_ok(C_COMPRESSIONLEVEL, 5);
        if let Some(rseqs) = generate_sequences(&gp, &rsrc, "incompressible") {
            drop(gp);
            let (rlit, rtotal) = extract_literals(&rsrc, &rseqs);
            assert_eq!(rtotal, rsrc.len());
            let mut rlitbuf = rlit.clone();
            rlitbuf.extend_from_slice(&[0u8; 8]);
            let rc = run(
                &base,
                &rseqs,
                &rlitbuf,
                rlit.len(),
                rlitbuf.len(),
                None,
                rsrc.len(),
                "incompressible block",
            );
            // whichever way it goes, both libraries must agree (asserted in `run`)
            let _ = rc;
        }
    }

    // ---- ERRORS 142: leftover literals after all blocks
    for &extra in &[1usize, 7, 500] {
        if lit.len() + extra + 8 > litbuf.len() + 8 {
            // grow the buffer so litSize+8 <= capacity
        }
        let mut big = lit.clone();
        for _ in 0..extra {
            big.push(rng.byte());
        }
        let mut bigbuf = big.clone();
        bigbuf.extend_from_slice(&[0u8; 8]);
        let rc = run(
            &base,
            &seqs,
            &bigbuf,
            big.len(),
            bigbuf.len(),
            None,
            src.len(),
            &format!("leftover literals +{extra}"),
        );
        assert!(is_error(rc), "leftover literals must fail (+{extra})");
    }

    // ---- ERRORS 143: sequences don't sum to decompressedSize
    for &delta in &[1i64, -1, 1000, -1000] {
        let ds = (src.len() as i64 + delta) as usize;
        let rc = run(
            &base,
            &seqs,
            &litbuf,
            lit.len(),
            litbuf.len(),
            None,
            ds,
            &format!("decompressedSize {ds} (real {})", src.len()),
        );
        assert!(is_error(rc), "wrong decompressedSize must fail (delta={delta})");
    }
}

// ========================================================== ERRORS 144 ====

#[test]
fn err_convertBlockSequences() {
    let (c_cv, r_cv) = unsafe { pair::<FnConvertBlockSequences>("ZSTD_convertBlockSequences") };
    let (c_gss, r_gss) = unsafe { pair::<FnGetSeqStore>("ZSTD_getSeqStore") };
    let (c_cs, r_cs) = unsafe { pair::<FnCompressSequences>("ZSTD_compressSequences") };
    let (c_cb, _) = unsafe { pair::<FnBound>("ZSTD_compressBound") };
    let mut rng = Rng::new(0x144);

    // A freshly created CCtx has seqStore.maxNbSeq == 0, so *any* nbSequences
    // satisfies `nbSequences >= maxNbSeq` and the guard fires before any of the
    // sequence data is touched.
    let seqs = [
        Sequence { offset: 1, litLength: 4, matchLength: 5, rep: 0 },
        Sequence { offset: 0, litLength: 0, matchLength: 0, rep: 0 },
    ];
    for &nb in &[1usize, 2, 100, 100000] {
        for &rcr in &[0, 1] {
            let p = CctxPair::new();
            let a = unsafe { c_cv(p.c, seqs.as_ptr(), nb, rcr) };
            let b = unsafe { r_cv(p.r, seqs.as_ptr(), nb, rcr) };
            assert_eq!(a, b, "fresh cctx nb={nb} rcr={rcr}: (C err={})", err_code(a));
            assert!(is_error(a), "fresh cctx nb={nb} must be rejected");
        }
    }

    // Now with a *real* seqStore: run a compressSequences first so the workspace
    // (and therefore maxNbSeq) is allocated, then call with nbSequences at and
    // above maxNbSeq.
    let src = gen(Shape::Text, 150000, &mut rng);
    let gp = CctxPair::new();
    gp.set_ok(C_COMPRESSIONLEVEL, 5);
    let gseqs = generate_sequences(&gp, &src, "cv-base").expect("sequences");
    drop(gp);

    let p = CctxPair::new();
    p.set_ok(C_COMPRESSIONLEVEL, 5);
    p.set_ok(C_BLOCKDELIMITERS, 1);
    let cap = unsafe { c_cb(src.len()) } + 1024;
    let mut co = vec![0u8; cap];
    let mut ro = vec![0u8; cap];
    let x = unsafe {
        c_cs(p.c, co.as_mut_ptr(), cap, gseqs.as_ptr(), gseqs.len(), src.as_ptr(), src.len())
    };
    let y = unsafe {
        r_cs(p.r, ro.as_mut_ptr(), cap, gseqs.as_ptr(), gseqs.len(), src.as_ptr(), src.len())
    };
    assert_eq!(x, y, "cv-base compressSequences");
    assert!(!is_error(x));

    let cmax = unsafe { (*c_gss(p.c)).maxNbSeq };
    let rmax = unsafe { (*r_gss(p.r)).maxNbSeq };
    assert_eq!(cmax, rmax, "seqStore.maxNbSeq");
    assert!(cmax > 0, "maxNbSeq should be allocated after a compression");

    for &nb in &[cmax, cmax + 1, cmax + 1000, cmax * 2] {
        for &rcr in &[0, 1] {
            let a = unsafe { c_cv(p.c, gseqs.as_ptr(), nb, rcr) };
            let b = unsafe { r_cv(p.r, gseqs.as_ptr(), nb, rcr) };
            assert_eq!(a, b, "nb={nb} >= maxNbSeq={cmax} rcr={rcr}: (C err={})", err_code(a));
            assert!(is_error(a), "nb={nb} >= maxNbSeq must be rejected");
        }
    }
    // ...and a legal call (nbSequences < maxNbSeq, array ends on a delimiter)
    // must produce the same result on both sides.
    {
        let first_delim = gseqs
            .iter()
            .position(|s| s.offset == 0 && s.matchLength == 0)
            .expect("delimiter");
        let nb = first_delim + 1;
        if nb < cmax {
            for &rcr in &[0, 1] {
                let a = unsafe { c_cv(p.c, gseqs.as_ptr(), nb, rcr) };
                let b = unsafe { r_cv(p.r, gseqs.as_ptr(), nb, rcr) };
                assert_eq!(a, b, "legal nb={nb} rcr={rcr}: (C err={})", err_code(a));
                // and the resulting seqStore must match
                let cs = unsafe { *c_gss(p.c) };
                let rs = unsafe { *r_gss(p.r) };
                compare_seqstore(&cs, &rs, &format!("after convertBlockSequences rcr={rcr}"));
            }
        }
    }
}

/// Compare every observable field of two SeqStore instances.
fn compare_seqstore(c: &SeqStore, r: &SeqStore, ctx: &str) {
    assert_eq!(c.maxNbSeq, r.maxNbSeq, "{ctx}: maxNbSeq");
    assert_eq!(c.maxNbLit, r.maxNbLit, "{ctx}: maxNbLit");
    assert_eq!(c.longLengthType, r.longLengthType, "{ctx}: longLengthType");
    assert_eq!(c.longLengthPos, r.longLengthPos, "{ctx}: longLengthPos");
    let cn = unsafe { c.sequences.offset_from(c.sequencesStart) };
    let rn = unsafe { r.sequences.offset_from(r.sequencesStart) };
    assert_eq!(cn, rn, "{ctx}: nbSeq");
    let cl = unsafe { c.lit.offset_from(c.litStart) };
    let rl = unsafe { r.lit.offset_from(r.litStart) };
    assert_eq!(cl, rl, "{ctx}: litSize");
    assert!(cn >= 0 && cl >= 0);
    let cseq = unsafe { std::slice::from_raw_parts(c.sequencesStart, cn as usize) };
    let rseq = unsafe { std::slice::from_raw_parts(r.sequencesStart, rn as usize) };
    assert_eq!(cseq, rseq, "{ctx}: SeqDef array");
    let clit = unsafe { std::slice::from_raw_parts(c.litStart, cl as usize) };
    let rlit = unsafe { std::slice::from_raw_parts(r.litStart, rl as usize) };
    assert_bytes_eq(&format!("{ctx}: literals"), clit, rlit);
}

// =========================================================== CONFIG 154 ====

#[test]
fn cfg_literals_functions() {
    let (c_ncl, r_ncl) = unsafe { pair::<FnNoCompressLiterals>("ZSTD_noCompressLiterals") };
    let (c_rle, r_rle) = unsafe { pair::<FnNoCompressLiterals>("ZSTD_compressRleLiteralsBlock") };
    let (c_cl, r_cl) = unsafe { pair::<FnCompressLiterals>("ZSTD_compressLiterals") };
    let mut rng = Rng::new(0x154);

    for &n in &[0usize, 1, 2, 31, 32, 63, 1024, 4095, 4096, 16384, 131072] {
        for &kind in &[0, 1, 2, 3] {
            let src = match kind {
                0 => gen(Shape::Random, n, &mut rng),
                1 => vec![rng.byte(); n],
                2 => gen(Shape::Text, n, &mut rng),
                _ => gen(Shape::Sparse, n, &mut rng),
            };
            let ctx0 = format!("lits n={n} kind={kind}");

            // ---- ZSTD_noCompressLiterals over a range of dst capacities
            let fl_size = 1 + (n > 31) as usize + (n > 4095) as usize;
            for &cap in &[0usize, 1, fl_size, n + fl_size - 1, n + fl_size, n + fl_size + 8] {
                let cap = if n + fl_size == 0 { cap } else { cap };
                let mut co = vec![0xAAu8; cap.max(1)];
                let mut ro = vec![0x55u8; cap.max(1)];
                let a = unsafe { c_ncl(co.as_mut_ptr(), cap, src.as_ptr(), n) };
                let b = unsafe { r_ncl(ro.as_mut_ptr(), cap, src.as_ptr(), n) };
                assert_eq!(a, b, "{ctx0}: noCompressLiterals cap={cap} (C err={})", err_code(a));
                if !is_error(a) {
                    assert_bytes_eq(
                        &format!("{ctx0}: noCompressLiterals bytes cap={cap}"),
                        &co[..a],
                        &ro[..a],
                    );
                }
            }

            // ---- ZSTD_compressRleLiteralsBlock: only legal for all-identical
            // input with srcSize >= 1 and dstCapacity >= 4 (both are asserted in C).
            if n >= 1 && kind == 1 {
                for &cap in &[4usize, 8, 64] {
                    let mut co = vec![0xAAu8; cap];
                    let mut ro = vec![0x55u8; cap];
                    let a = unsafe { c_rle(co.as_mut_ptr(), cap, src.as_ptr(), n) };
                    let b = unsafe { r_rle(ro.as_mut_ptr(), cap, src.as_ptr(), n) };
                    assert_eq!(a, b, "{ctx0}: rleLiteralsBlock cap={cap}");
                    assert_bytes_eq(
                        &format!("{ctx0}: rleLiteralsBlock bytes cap={cap}"),
                        &co[..a],
                        &ro[..a],
                    );
                }
            }

            // ---- ZSTD_compressLiterals over the full flag matrix
            for strategy in 1..=9 {
                for &disable in &[0, 1] {
                    for &suspect in &[0, 1] {
                        for &bmi2 in &[0, 1] {
                            let mut cws = Aligned::new(ENTROPY_WKSP_BYTES);
                            let mut rws = Aligned::new(ENTROPY_WKSP_BYTES);
                            let cprev = Aligned::new(HUFTABLES_BYTES);
                            let rprev = Aligned::new(HUFTABLES_BYTES);
                            let mut cnext = Aligned::new(HUFTABLES_BYTES);
                            let mut rnext = Aligned::new(HUFTABLES_BYTES);
                            let cap = n + 64;
                            let mut co = vec![0xAAu8; cap];
                            let mut ro = vec![0x55u8; cap];
                            let ctx = format!(
                                "{ctx0} strat={strategy} dis={disable} sus={suspect} bmi2={bmi2}"
                            );
                            let a = unsafe {
                                c_cl(
                                    co.as_mut_ptr(),
                                    cap,
                                    src.as_ptr(),
                                    n,
                                    cws.ptr(),
                                    ENTROPY_WKSP_BYTES,
                                    cprev.cptr(),
                                    cnext.ptr(),
                                    strategy,
                                    disable,
                                    suspect,
                                    bmi2,
                                )
                            };
                            let b = unsafe {
                                r_cl(
                                    ro.as_mut_ptr(),
                                    cap,
                                    src.as_ptr(),
                                    n,
                                    rws.ptr(),
                                    ENTROPY_WKSP_BYTES,
                                    rprev.cptr(),
                                    rnext.ptr(),
                                    strategy,
                                    disable,
                                    suspect,
                                    bmi2,
                                )
                            };
                            assert_eq!(
                                a, b,
                                "{ctx}: compressLiterals return (C err={})",
                                err_code(a)
                            );
                            if !is_error(a) {
                                assert_bytes_eq(
                                    &format!("{ctx}: compressLiterals bytes"),
                                    &co[..a],
                                    &ro[..a],
                                );
                            }
                            assert_bytes_eq(
                                &format!("{ctx}: nextHuf table"),
                                cnext.bytes(),
                                rnext.bytes(),
                            );
                        }
                    }
                }
            }
        }
    }
}

// =========================================================== CONFIG 153 ====

/// Drive the seqStore / entropy internals from a real block compression.
#[test]
fn cfg_seqstore_internals() {
    let (c_new, r_new) = unsafe { pair::<FnNew>("ZSTD_createCCtx") };
    let (c_free, r_free) = unsafe { pair::<FnFree>("ZSTD_freeCCtx") };
    let (c_beg, r_beg) = unsafe { pair::<FnBegin>("ZSTD_compressBegin") };
    let (c_blk, r_blk) = unsafe { pair::<FnChunk>("ZSTD_compressBlock") };
    let (c_gss, r_gss) = unsafe { pair::<FnGetSeqStore>("ZSTD_getSeqStore") };
    let (c_rss, r_rss) = unsafe { pair::<FnResetSeqStore>("ZSTD_resetSeqStore") };
    let (c_stc, r_stc) = unsafe { pair::<FnSeqToCodes>("ZSTD_seqToCodes") };
    let (c_set, r_set) = unsafe { pair::<FnSelectEncodingType>("ZSTD_selectEncodingType") };
    let (c_bct, r_bct) = unsafe { pair::<FnBuildCTable>("ZSTD_buildCTable") };
    let (c_enc, r_enc) = unsafe { pair::<FnEncodeSequences>("ZSTD_encodeSequences") };
    let (c_fbc, r_fbc) = unsafe { pair::<FnFseBitCost>("ZSTD_fseBitCost") };
    let (c_cec, r_cec) = unsafe { pair::<FnCrossEntropyCost>("ZSTD_crossEntropyCost") };
    let (c_bes, r_bes) =
        unsafe { pair::<FnBuildBlockEntropyStats>("ZSTD_buildBlockEntropyStats") };
    let (c_cp, r_cp) = unsafe { pair::<FnNew>("ZSTD_createCCtxParams") };
    let (c_cpf, r_cpf) = unsafe { pair::<FnFree>("ZSTD_freeCCtxParams") };
    let (c_cps, r_cps) = unsafe { pair::<FnSetParam>("ZSTD_CCtxParams_setParameter") };
    let mut rng = Rng::new(0x153);

    for &level in &[3, 19] {
        for &shape in &[Shape::Text, Shape::Mixed, Shape::Repetitive, Shape::LongMatches] {
            let block = gen(shape, 131072, &mut rng);
            let ctx0 = format!("seqstore lvl={level} shape={shape:?}");

            let cctx = unsafe { c_new() };
            let rctx = unsafe { r_new() };
            assert!(!cctx.is_null() && !rctx.is_null());
            assert!(!is_error(unsafe { c_beg(cctx, level) }));
            assert!(!is_error(unsafe { r_beg(rctx, level) }));
            let cap = block.len() + 1024;
            let mut cbuf = vec![0u8; cap];
            let mut rbuf = vec![0u8; cap];
            let a = unsafe { c_blk(cctx, cbuf.as_mut_ptr(), cap, block.as_ptr(), block.len()) };
            let b = unsafe { r_blk(rctx, rbuf.as_mut_ptr(), cap, block.as_ptr(), block.len()) };
            assert_eq!(a, b, "{ctx0}: compressBlock return (C err={})", err_code(a));
            assert!(!is_error(a));
            assert_bytes_eq(&format!("{ctx0}: compressBlock bytes"), &cbuf[..a], &rbuf[..a]);

            // ---- ZSTD_getSeqStore + field-by-field comparison
            let cssp = unsafe { c_gss(cctx) };
            let rssp = unsafe { r_gss(rctx) };
            assert!(!cssp.is_null() && !rssp.is_null());
            let css = unsafe { *cssp };
            let rss = unsafe { *rssp };
            compare_seqstore(&css, &rss, &format!("{ctx0}: after compressBlock"));

            let nb_seq = unsafe { css.sequences.offset_from(css.sequencesStart) } as usize;
            let seqdefs =
                unsafe { std::slice::from_raw_parts(css.sequencesStart, nb_seq) }.to_vec();

            if nb_seq >= 3 {
                // ---- ZSTD_seqToCodes on both sides
                let x = unsafe { c_stc(cssp) };
                let y = unsafe { r_stc(rssp) };
                assert_eq!(x, y, "{ctx0}: ZSTD_seqToCodes return");
                let cll = unsafe { std::slice::from_raw_parts(css.llCode, nb_seq) };
                let rll = unsafe { std::slice::from_raw_parts(rss.llCode, nb_seq) };
                let cml = unsafe { std::slice::from_raw_parts(css.mlCode, nb_seq) };
                let rml = unsafe { std::slice::from_raw_parts(rss.mlCode, nb_seq) };
                let cof = unsafe { std::slice::from_raw_parts(css.ofCode, nb_seq) };
                let rof = unsafe { std::slice::from_raw_parts(rss.ofCode, nb_seq) };
                assert_bytes_eq(&format!("{ctx0}: llCode"), cll, rll);
                assert_bytes_eq(&format!("{ctx0}: mlCode"), cml, rml);
                assert_bytes_eq(&format!("{ctx0}: ofCode"), cof, rof);

                // ---- per-stream histograms, then the FSE table machinery
                for &(name, codes, fselog, dnorm, dnormlog, dmax) in &[
                    (
                        "ll",
                        cll,
                        LL_FSELOG,
                        &LL_DEFAULT_NORM[..],
                        LL_DEFAULTNORMLOG,
                        MAX_LL,
                    ),
                    (
                        "ml",
                        cml,
                        ML_FSELOG,
                        &ML_DEFAULT_NORM[..],
                        ML_DEFAULTNORMLOG,
                        MAX_ML,
                    ),
                    (
                        "of",
                        cof,
                        OFF_FSELOG,
                        &OF_DEFAULT_NORM[..],
                        OF_DEFAULTNORMLOG,
                        DEFAULT_MAX_OFF,
                    ),
                ] {
                    let mut count = vec![0u32; 64];
                    let mut max = 0u32;
                    for &c in codes.iter() {
                        count[c as usize] += 1;
                        max = max.max(c as u32);
                    }
                    // the default-norm tables only describe symbols up to dmax
                    if max > dmax {
                        continue;
                    }
                    let most = *count[..=max as usize].iter().max().unwrap() as usize;
                    let ctx = format!("{ctx0} stream={name} max={max} nbSeq={nb_seq}");

                    // ZSTD_crossEntropyCost against the default normalized table
                    let a = unsafe { c_cec(dnorm.as_ptr(), dnormlog, count.as_ptr(), max) };
                    let b = unsafe { r_cec(dnorm.as_ptr(), dnormlog, count.as_ptr(), max) };
                    assert_eq!(a, b, "{ctx}: ZSTD_crossEntropyCost");

                    // ZSTD_buildCTable(set_basic) => a valid CTable from defaultNorm
                    let mut cbasic = vec![0u32; CTABLE_U32];
                    let mut rbasic = vec![0u32; CTABLE_U32];
                    let mut cws = Aligned::new(ENTROPY_WKSP_BYTES);
                    let mut rws = Aligned::new(ENTROPY_WKSP_BYTES);
                    let mut ccount = count.clone();
                    let mut rcount = count.clone();
                    let mut cdst = vec![0u8; 4096];
                    let mut rdst = vec![0u8; 4096];
                    let a = unsafe {
                        c_bct(
                            cdst.as_mut_ptr(),
                            cdst.len(),
                            cbasic.as_mut_ptr(),
                            fselog,
                            SET_BASIC,
                            ccount.as_mut_ptr(),
                            max,
                            codes.as_ptr(),
                            nb_seq,
                            dnorm.as_ptr(),
                            dnormlog,
                            dmax,
                            std::ptr::null(),
                            0,
                            cws.ptr(),
                            ENTROPY_WKSP_BYTES,
                        )
                    };
                    let b = unsafe {
                        r_bct(
                            rdst.as_mut_ptr(),
                            rdst.len(),
                            rbasic.as_mut_ptr(),
                            fselog,
                            SET_BASIC,
                            rcount.as_mut_ptr(),
                            max,
                            codes.as_ptr(),
                            nb_seq,
                            dnorm.as_ptr(),
                            dnormlog,
                            dmax,
                            std::ptr::null(),
                            0,
                            rws.ptr(),
                            ENTROPY_WKSP_BYTES,
                        )
                    };
                    assert_eq!(a, b, "{ctx}: buildCTable(set_basic) (C err={})", err_code(a));
                    assert!(!is_error(a), "{ctx}: buildCTable(set_basic) err {}", err_code(a));
                    assert_eq!(cbasic, rbasic, "{ctx}: buildCTable(set_basic) table");
                    assert_eq!(ccount, rcount, "{ctx}: buildCTable(set_basic) count");

                    // ZSTD_fseBitCost against that table
                    let a = unsafe { c_fbc(cbasic.as_ptr(), count.as_ptr(), max) };
                    let b = unsafe { r_fbc(rbasic.as_ptr(), count.as_ptr(), max) };
                    assert_eq!(a, b, "{ctx}: ZSTD_fseBitCost (C err={})", err_code(a));

                    // ZSTD_selectEncodingType over every repeatMode / policy / strategy
                    for &rm0 in &[FSE_REPEAT_NONE, FSE_REPEAT_CHECK, FSE_REPEAT_VALID] {
                        for &policy in &[0, 1] {
                            for strategy in 1..=9 {
                                let mut crm = rm0;
                                let mut rrm = rm0;
                                let a = unsafe {
                                    c_set(
                                        &mut crm,
                                        count.as_ptr(),
                                        max,
                                        most,
                                        nb_seq,
                                        fselog,
                                        cbasic.as_ptr(),
                                        dnorm.as_ptr(),
                                        dnormlog,
                                        policy,
                                        strategy,
                                    )
                                };
                                let b = unsafe {
                                    r_set(
                                        &mut rrm,
                                        count.as_ptr(),
                                        max,
                                        most,
                                        nb_seq,
                                        fselog,
                                        rbasic.as_ptr(),
                                        dnorm.as_ptr(),
                                        dnormlog,
                                        policy,
                                        strategy,
                                    )
                                };
                                assert_eq!(
                                    a, b,
                                    "{ctx}: selectEncodingType rm={rm0} pol={policy} strat={strategy}"
                                );
                                assert_eq!(
                                    crm, rrm,
                                    "{ctx}: selectEncodingType repeatMode out (rm={rm0} pol={policy} strat={strategy})"
                                );
                            }
                        }
                    }

                    // ZSTD_buildCTable for every encoding type
                    for &ty in &[SET_RLE, SET_REPEAT, SET_COMPRESSED] {
                        let mut ct = vec![0u32; CTABLE_U32];
                        let mut rt = vec![0u32; CTABLE_U32];
                        let mut ccount = count.clone();
                        let mut rcount = count.clone();
                        let mut cws = Aligned::new(ENTROPY_WKSP_BYTES);
                        let mut rws = Aligned::new(ENTROPY_WKSP_BYTES);
                        let mut cdst = vec![0u8; 4096];
                        let mut rdst = vec![0u8; 4096];
                        let a = unsafe {
                            c_bct(
                                cdst.as_mut_ptr(),
                                cdst.len(),
                                ct.as_mut_ptr(),
                                fselog,
                                ty,
                                ccount.as_mut_ptr(),
                                max,
                                codes.as_ptr(),
                                nb_seq,
                                dnorm.as_ptr(),
                                dnormlog,
                                dmax,
                                cbasic.as_ptr(),
                                CTABLE_U32 * 4,
                                cws.ptr(),
                                ENTROPY_WKSP_BYTES,
                            )
                        };
                        let b = unsafe {
                            r_bct(
                                rdst.as_mut_ptr(),
                                rdst.len(),
                                rt.as_mut_ptr(),
                                fselog,
                                ty,
                                rcount.as_mut_ptr(),
                                max,
                                codes.as_ptr(),
                                nb_seq,
                                dnorm.as_ptr(),
                                dnormlog,
                                dmax,
                                rbasic.as_ptr(),
                                CTABLE_U32 * 4,
                                rws.ptr(),
                                ENTROPY_WKSP_BYTES,
                            )
                        };
                        assert_eq!(a, b, "{ctx}: buildCTable(type={ty}) (C err={})", err_code(a));
                        if !is_error(a) {
                            assert_bytes_eq(
                                &format!("{ctx}: buildCTable(type={ty}) dst"),
                                &cdst[..a],
                                &rdst[..a],
                            );
                            assert_eq!(ct, rt, "{ctx}: buildCTable(type={ty}) table");
                            assert_eq!(ccount, rcount, "{ctx}: buildCTable(type={ty}) count");
                        }
                    }
                }

                // ---- ZSTD_encodeSequences with freshly built (set_compressed) tables
                encode_sequences_case(
                    &format!("{ctx0}"),
                    nb_seq,
                    cll,
                    cml,
                    cof,
                    &seqdefs,
                    &c_bct,
                    &r_bct,
                    &c_enc,
                    &r_enc,
                );

                // ---- ZSTD_buildBlockEntropyStats
                for &lvl in &[level, 1, 12] {
                    let cparams = unsafe { c_cp() };
                    let rparams = unsafe { r_cp() };
                    assert!(!cparams.is_null() && !rparams.is_null());
                    assert!(!is_error(unsafe { c_cps(cparams, C_COMPRESSIONLEVEL, lvl) }));
                    assert!(!is_error(unsafe { r_cps(rparams, C_COMPRESSIONLEVEL, lvl) }));
                    let cprev = Aligned::new(ENTROPY_TABLES_BYTES);
                    let rprev = Aligned::new(ENTROPY_TABLES_BYTES);
                    let mut cnext = Aligned::new(ENTROPY_TABLES_BYTES);
                    let mut rnext = Aligned::new(ENTROPY_TABLES_BYTES);
                    let mut cmeta = Aligned::new(METADATA_BYTES);
                    let mut rmeta = Aligned::new(METADATA_BYTES);
                    let mut cws = Aligned::new(ENTROPY_WKSP_BYTES);
                    let mut rws = Aligned::new(ENTROPY_WKSP_BYTES);
                    let a = unsafe {
                        c_bes(
                            cssp,
                            cprev.cptr(),
                            cnext.ptr(),
                            cparams as *const c_void,
                            cmeta.ptr(),
                            cws.ptr(),
                            ENTROPY_WKSP_BYTES,
                        )
                    };
                    let b = unsafe {
                        r_bes(
                            rssp,
                            rprev.cptr(),
                            rnext.ptr(),
                            rparams as *const c_void,
                            rmeta.ptr(),
                            rws.ptr(),
                            ENTROPY_WKSP_BYTES,
                        )
                    };
                    assert_eq!(
                        a, b,
                        "{ctx0} lvl={lvl}: buildBlockEntropyStats (C err={})",
                        err_code(a)
                    );
                    if !is_error(a) {
                        assert_bytes_eq(
                            &format!("{ctx0} lvl={lvl}: entropy metadata"),
                            cmeta.bytes(),
                            rmeta.bytes(),
                        );
                        assert_bytes_eq(
                            &format!("{ctx0} lvl={lvl}: nextEntropy tables"),
                            cnext.bytes(),
                            rnext.bytes(),
                        );
                    }
                    unsafe {
                        c_cpf(cparams);
                        r_cpf(rparams);
                    }
                }
            }

            // ---- ZSTD_resetSeqStore (done last: it clears the state above)
            unsafe {
                c_rss(cssp as *mut SeqStore);
                r_rss(rssp as *mut SeqStore);
            }
            let css = unsafe { *cssp };
            let rss = unsafe { *rssp };
            compare_seqstore(&css, &rss, &format!("{ctx0}: after resetSeqStore"));
            assert_eq!(
                unsafe { css.sequences.offset_from(css.sequencesStart) },
                0,
                "{ctx0}: resetSeqStore clears sequences"
            );

            unsafe {
                c_free(cctx);
                r_free(rctx);
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn encode_sequences_case(
    ctx0: &str,
    nb_seq: usize,
    llcodes: &[u8],
    mlcodes: &[u8],
    ofcodes: &[u8],
    seqdefs: &[SeqDef],
    c_bct: &libloading::Symbol<'static, FnBuildCTable>,
    r_bct: &libloading::Symbol<'static, FnBuildCTable>,
    c_enc: &libloading::Symbol<'static, FnEncodeSequences>,
    r_enc: &libloading::Symbol<'static, FnEncodeSequences>,
) {
    // Build one set_compressed CTable per stream (exactly what
    // ZSTD_entropyCompressSeqStore_internal does), then encode the sequences.
    let mut tables: Vec<(Vec<u32>, Vec<u32>)> = Vec::new();
    for &(codes, fselog, dnorm, dnormlog, dmax) in &[
        (llcodes, LL_FSELOG, &LL_DEFAULT_NORM[..], LL_DEFAULTNORMLOG, MAX_LL),
        (mlcodes, ML_FSELOG, &ML_DEFAULT_NORM[..], ML_DEFAULTNORMLOG, MAX_ML),
        (ofcodes, OFF_FSELOG, &OF_DEFAULT_NORM[..], OF_DEFAULTNORMLOG, MAX_OFF),
    ] {
        let mut count = vec![0u32; 64];
        let mut max = 0u32;
        for &c in codes.iter() {
            count[c as usize] += 1;
            max = max.max(c as u32);
        }
        let most = *count[..=max as usize].iter().max().unwrap() as usize;
        if most == nb_seq || max == 0 {
            // set_compressed is not valid for a single-symbol alphabet
            return;
        }
        let mut ct = vec![0u32; CTABLE_U32];
        let mut rt = vec![0u32; CTABLE_U32];
        let mut ccount = count.clone();
        let mut rcount = count.clone();
        let mut cws = Aligned::new(ENTROPY_WKSP_BYTES);
        let mut rws = Aligned::new(ENTROPY_WKSP_BYTES);
        let mut cdst = vec![0u8; 4096];
        let mut rdst = vec![0u8; 4096];
        let a = unsafe {
            c_bct(
                cdst.as_mut_ptr(),
                cdst.len(),
                ct.as_mut_ptr(),
                fselog,
                SET_COMPRESSED,
                ccount.as_mut_ptr(),
                max,
                codes.as_ptr(),
                nb_seq,
                dnorm.as_ptr(),
                dnormlog,
                dmax,
                std::ptr::null(),
                0,
                cws.ptr(),
                ENTROPY_WKSP_BYTES,
            )
        };
        let b = unsafe {
            r_bct(
                rdst.as_mut_ptr(),
                rdst.len(),
                rt.as_mut_ptr(),
                fselog,
                SET_COMPRESSED,
                rcount.as_mut_ptr(),
                max,
                codes.as_ptr(),
                nb_seq,
                dnorm.as_ptr(),
                dnormlog,
                dmax,
                std::ptr::null(),
                0,
                rws.ptr(),
                ENTROPY_WKSP_BYTES,
            )
        };
        assert_eq!(a, b, "{ctx0}: encode-prep buildCTable (C err={})", err_code(a));
        if is_error(a) {
            return;
        }
        assert_bytes_eq(&format!("{ctx0}: encode-prep NCount"), &cdst[..a], &rdst[..a]);
        assert_eq!(ct, rt, "{ctx0}: encode-prep table");
        tables.push((ct, rt));
    }

    for &bmi2 in &[0, 1] {
        let cap = 1 << 18;
        let mut co = vec![0xAAu8; cap];
        let mut ro = vec![0x55u8; cap];
        let a = unsafe {
            c_enc(
                co.as_mut_ptr(),
                cap,
                tables[1].0.as_ptr(),
                mlcodes.as_ptr(),
                tables[2].0.as_ptr(),
                ofcodes.as_ptr(),
                tables[0].0.as_ptr(),
                llcodes.as_ptr(),
                seqdefs.as_ptr(),
                nb_seq,
                0,
                bmi2,
            )
        };
        let b = unsafe {
            r_enc(
                ro.as_mut_ptr(),
                cap,
                tables[1].1.as_ptr(),
                mlcodes.as_ptr(),
                tables[2].1.as_ptr(),
                ofcodes.as_ptr(),
                tables[0].1.as_ptr(),
                llcodes.as_ptr(),
                seqdefs.as_ptr(),
                nb_seq,
                0,
                bmi2,
            )
        };
        assert_eq!(a, b, "{ctx0}: encodeSequences bmi2={bmi2} (C err={})", err_code(a));
        if !is_error(a) {
            assert_bytes_eq(
                &format!("{ctx0}: encodeSequences bytes bmi2={bmi2}"),
                &co[..a],
                &ro[..a],
            );
        }
    }
}

// =========================================================== CONFIG 155 ====

#[test]
fn cfg_split_block_direct() {
    let (c_sp, r_sp) = unsafe { pair::<FnSplitBlock>("ZSTD_splitBlock") };
    let (c_cb, _) = unsafe { pair::<FnBound>("ZSTD_compressBound") };
    let mut rng = Rng::new(0x155);

    // ---- ZSTD_splitBlock: only accepts full 128 KB blocks.
    for round in 0..24 {
        // mixed-entropy data: half text, half random, in varying proportions
        let mut block = Vec::with_capacity(131072);
        let cut = rng.range(1024, 131072 - 1024);
        block.extend_from_slice(&gen(Shape::Text, cut, &mut rng));
        block.extend_from_slice(&gen(Shape::Random, 131072 - cut, &mut rng));
        assert_eq!(block.len(), 131072);
        for level in 0..=4 {
            let mut cws = Aligned::new(SPLITBLOCK_WKSP_BYTES);
            let mut rws = Aligned::new(SPLITBLOCK_WKSP_BYTES);
            let a = unsafe {
                c_sp(block.as_ptr(), block.len(), level, cws.ptr(), SPLITBLOCK_WKSP_BYTES)
            };
            let b = unsafe {
                r_sp(block.as_ptr(), block.len(), level, rws.ptr(), SPLITBLOCK_WKSP_BYTES)
            };
            assert_eq!(a, b, "splitBlock round={round} cut={cut} level={level}");
        }
    }
    // homogeneous blocks of every shape
    for &shape in ALL_SHAPES {
        let block = gen(shape, 131072, &mut rng);
        for level in 0..=4 {
            let mut cws = Aligned::new(SPLITBLOCK_WKSP_BYTES);
            let mut rws = Aligned::new(SPLITBLOCK_WKSP_BYTES);
            let a = unsafe {
                c_sp(block.as_ptr(), block.len(), level, cws.ptr(), SPLITBLOCK_WKSP_BYTES)
            };
            let b = unsafe {
                r_sp(block.as_ptr(), block.len(), level, rws.ptr(), SPLITBLOCK_WKSP_BYTES)
            };
            assert_eq!(a, b, "splitBlock shape={shape:?} level={level}");
        }
    }

    // ---- ZSTD_compressSuperBlock.
    // It cannot be called in isolation: it consumes the CCtx's *already built*
    // seqStore plus the block-state entropy tables, so a standalone call would
    // require reproducing internal state.  It is therefore exercised through the
    // public path that uses it, ZSTD_c_targetCBlockSize, comparing the resulting
    // frames byte-for-byte (and round-tripping them).
    let (c_c2, r_c2) = unsafe {
        pair::<unsafe extern "C" fn(*mut c_void, *mut u8, usize, *const u8, usize) -> usize>(
            "ZSTD_compress2",
        )
    };
    let (c_dec, r_dec) = unsafe { pair::<FnDecompress>("ZSTD_decompress") };
    for &tcbs in &[1340, 2000, 8192, 65536, 131072] {
        for &lvl in &[3, 9, 19] {
            for &shape in &[Shape::Text, Shape::Mixed, Shape::Random] {
                let src = gen(shape, 131072 * 2, &mut rng);
                let p = CctxPair::new();
                p.set_ok(C_COMPRESSIONLEVEL, lvl);
                p.set_ok(C_TARGETCBLOCKSIZE, tcbs);
                let cap = unsafe { c_cb(src.len()) } + 1024;
                let mut co = vec![0xAAu8; cap];
                let mut ro = vec![0x55u8; cap];
                let a =
                    unsafe { c_c2(p.c, co.as_mut_ptr(), cap, src.as_ptr(), src.len()) };
                let b =
                    unsafe { r_c2(p.r, ro.as_mut_ptr(), cap, src.as_ptr(), src.len()) };
                let ctx = format!("superblock tcbs={tcbs} lvl={lvl} shape={shape:?}");
                assert_eq!(a, b, "{ctx}: compress2 return (C err={})", err_code(a));
                assert!(!is_error(a));
                assert_bytes_eq(&format!("{ctx}: bytes"), &co[..a], &ro[..a]);
                let dcap = src.len() + 64;
                let mut cd = vec![0u8; dcap];
                let mut rd = vec![0u8; dcap];
                let x = unsafe { c_dec(cd.as_mut_ptr(), dcap, co.as_ptr(), a) };
                let y = unsafe { r_dec(rd.as_mut_ptr(), dcap, ro.as_ptr(), a) };
                assert_eq!(x, y, "{ctx}: decompress return");
                assert!(!is_error(x));
                assert_bytes_eq(&format!("{ctx}: round trip"), &cd[..x], &src);
            }
        }
    }
}
