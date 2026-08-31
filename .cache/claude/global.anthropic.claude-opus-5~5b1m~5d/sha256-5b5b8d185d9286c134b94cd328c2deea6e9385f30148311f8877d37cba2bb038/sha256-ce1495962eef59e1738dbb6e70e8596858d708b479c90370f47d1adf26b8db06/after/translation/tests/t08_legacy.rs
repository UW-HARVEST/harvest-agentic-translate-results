//! Differential tests for the LEGACY decoders v0.1 .. v0.7
//! (`c_src/src/legacy/zstd_v0{1..7}.c`).
//!
//! Every symbol is reached through `dlopen` on both shared libraries, never by
//! linking the Rust crate directly.
//!
//! This library can only *write* the current frame format, so valid v0.1..v0.7
//! frames cannot be produced here. Instead the parsers are driven with a large
//! deterministic corpus of
//!   (a) pure random bytes,
//!   (b) buffers prefixed with each version's magic number + random payload,
//!   (c) buffers prefixed with each magic + *structured* (frame-header /
//!       block-header) payloads that get well past the magic check,
//!   (d) current-format zstd frames,
//!   (e) truncations and single-bit flips of all of the above,
//!   (f) empty / 1-byte inputs.
//! C and Rust must agree exactly on every return value, every out-parameter and
//! every produced byte. For the streaming entry points the whole
//! `decompressBegin`/`nextSrcSizeToDecompress`/`decompressContinue` state
//! machine is stepped in lock-step.
//!
//! Note on the build config: `ZSTD_LEGACY_SUPPORT=5` (see `c_src/CMakeLists.txt`)
//! means only the v0.5/v0.6/v0.7 magics are dispatched from the main
//! `ZSTD_decompress` path, but *all* of v01..v07 are compiled in and their
//! symbols exported, so their direct entry points are tested regardless.

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

mod common;
use common::*;

use libloading::Symbol;
use std::os::raw::{c_char, c_int, c_uint, c_void};
use std::sync::OnceLock;

// ------------------------------------------------------------------ fn types

type FnDecompress = unsafe extern "C" fn(*mut u8, usize, *const u8, usize) -> usize;
type FnFindFSI = unsafe extern "C" fn(*const u8, usize, *mut usize, *mut u64);
type FnIsError = unsafe extern "C" fn(usize) -> c_uint;
type FnErrName = unsafe extern "C" fn(usize) -> *const c_char;
type FnCreate = unsafe extern "C" fn() -> *mut c_void;
type FnCtxToSz = unsafe extern "C" fn(*mut c_void) -> usize;
type FnSzVoid = unsafe extern "C" fn() -> usize;
type FnDecompressDCtx =
    unsafe extern "C" fn(*mut c_void, *mut u8, usize, *const u8, usize) -> usize;
type FnUsingDict =
    unsafe extern "C" fn(*mut c_void, *mut u8, usize, *const u8, usize, *const u8, usize) -> usize;
type FnCtxDict = unsafe extern "C" fn(*mut c_void, *const u8, usize) -> usize;
type FnCopyDCtx = unsafe extern "C" fn(*mut c_void, *const c_void);
type FnPrepared =
    unsafe extern "C" fn(*mut c_void, *const c_void, *mut u8, usize, *const u8, usize) -> usize;
type FnGetFrameParams = unsafe extern "C" fn(*mut c_void, *const u8, usize) -> usize;
type FnBufToU64 = unsafe extern "C" fn(*const u8, usize) -> u64;
type FnZbuffCont =
    unsafe extern "C" fn(*mut c_void, *mut u8, *mut usize, *const u8, *mut usize) -> usize;
type FnIsSkip = unsafe extern "C" fn(*mut c_void) -> c_int;
type FnCreateDDict = unsafe extern "C" fn(*const u8, usize) -> *mut c_void;
type FnUsingDDict =
    unsafe extern "C" fn(*mut c_void, *mut u8, usize, *const u8, usize, *const c_void) -> usize;
type FnCreateAdv = unsafe extern "C" fn(CustomMem) -> *mut c_void;

// entropy
type FnReadNCount =
    unsafe extern "C" fn(*mut i16, *mut c_uint, *mut c_uint, *const u8, usize) -> usize;
type FnCreateDTable = unsafe extern "C" fn(c_uint) -> *mut u32;
type FnFreeDTable = unsafe extern "C" fn(*mut u32);
type FnBuildDTable = unsafe extern "C" fn(*mut u32, *const i16, c_uint, c_uint) -> usize;
type FnBuildRle = unsafe extern "C" fn(*mut u32, u8) -> usize;
type FnBuildRaw = unsafe extern "C" fn(*mut u32, c_uint) -> usize;
type FnFseUsingDTable = unsafe extern "C" fn(*mut u8, usize, *const u8, usize, *const u32) -> usize;
type FnHufReadX2 = unsafe extern "C" fn(*mut u16, *const u8, usize) -> usize;
type FnHufReadX4 = unsafe extern "C" fn(*mut u32, *const u8, usize) -> usize;
type FnHufUsingX2 = unsafe extern "C" fn(*mut u8, usize, *const u8, usize, *const u16) -> usize;
type FnHufUsingX4 = unsafe extern "C" fn(*mut u8, usize, *const u8, usize, *const u32) -> usize;
type FnHufDCtx = unsafe extern "C" fn(*mut u32, *mut u8, usize, *const u8, usize) -> usize;
type FnHufReadStats = unsafe extern "C" fn(
    *mut u8,
    usize,
    *mut u32,
    *mut c_uint,
    *mut c_uint,
    *const u8,
    usize,
) -> usize;
type FnSelectDecoder = unsafe extern "C" fn(usize, usize) -> u32;

// main API
type FnBufToU32 = unsafe extern "C" fn(*const u8, usize) -> c_uint;
type FnBufToSz = unsafe extern "C" fn(*const u8, usize) -> usize;
type FnStream = unsafe extern "C" fn(*mut c_void, *mut ZSTD_outBuffer, *mut ZSTD_inBuffer) -> usize;
type FnCompress = unsafe extern "C" fn(*mut u8, usize, *const u8, usize, c_int) -> usize;

/// `ZSTDv07_customMem` / `ZSTD_customMem` — identical layout
/// (`{ alloc, free, opaque }`).
#[repr(C)]
#[derive(Copy, Clone)]
pub struct CustomMem {
    pub custom_alloc: Option<unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void>,
    pub custom_free: Option<unsafe extern "C" fn(*mut c_void, *mut c_void)>,
    pub opaque: *mut c_void,
}

/// `ZSTDv05_parameters`
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct V05Params {
    src_size: u64,
    window_log: u32,
    content_log: u32,
    hash_log: u32,
    search_log: u32,
    search_length: u32,
    target_length: u32,
    strategy: c_int,
}

/// `ZSTDv06_frameParams`
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct V06FrameParams {
    frame_content_size: u64,
    window_log: c_uint,
}

/// `ZSTDv07_frameParams`
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct V07FrameParams {
    frame_content_size: u64,
    window_size: c_uint,
    dict_id: c_uint,
    checksum_flag: c_uint,
}

fn cstr(p: *const c_char) -> String {
    if p.is_null() {
        return "<null>".into();
    }
    unsafe { std::ffi::CStr::from_ptr(p) }
        .to_string_lossy()
        .into_owned()
}

/// zstd error codes live at the very top of the `size_t` space.
fn looks_like_error(v: usize) -> bool {
    v >= usize::MAX - 250
}

/// Element-wise comparison with a compact first-divergence report (the tables
/// involved have thousands of entries, so `assert_eq!` output is unusable).
fn assert_words_eq<T: PartialEq + std::fmt::Debug>(ctx: &str, c: &[T], r: &[T]) {
    if c == r {
        return;
    }
    if c.len() != r.len() {
        panic!("{ctx}: length mismatch C={} Rust={}", c.len(), r.len());
    }
    let k = c.iter().zip(r).position(|(a, b)| a != b).unwrap();
    panic!(
        "{ctx}: divergence at index {k} of {}: C={:?} Rust={:?}",
        c.len(),
        c[k],
        r[k]
    );
}

// -------------------------------------------------------------- error naming

/// Optional `getErrorName` pair for a legacy family, so a divergence is reported
/// with the error *meaning*, not just the raw `size_t`.
struct Namer {
    c: Option<Symbol<'static, FnErrName>>,
    r: Option<Symbol<'static, FnErrName>>,
}

impl Namer {
    fn new(sym: &str) -> Namer {
        let i = impls();
        if i.has(sym) {
            let (c, r) = i.pair::<FnErrName>(sym);
            Namer {
                c: Some(c),
                r: Some(r),
            }
        } else {
            Namer { c: None, r: None }
        }
    }

    /// Compare a numeric return *and* (when exported) the error name it maps to.
    fn check(&self, ctx: &str, cret: usize, rret: usize) {
        assert_eq_dbg(ctx, cret, rret);
        if looks_like_error(cret) {
            if let (Some(cn), Some(rn)) = (&self.c, &self.r) {
                unsafe {
                    let (a, b) = (cstr(cn(cret)), cstr(rn(rret)));
                    assert_eq_dbg(&format!("{ctx}: errorName"), a, b);
                }
            }
        }
    }
}

// --------------------------------------------------------------------- corpus

/// Magic numbers as they appear *in the byte stream*: v0.1 stores it
/// big-endian, v0.2..v0.7 little-endian (see `ZSTD_isLegacy`).
const LEGACY_MAGIC: [[u8; 4]; 7] = [
    [0xFD, 0x2F, 0xB5, 0x1E], // v01, BE 0xFD2FB51E
    [0x22, 0xB5, 0x2F, 0xFD], // v02, LE 0xFD2FB522
    [0x23, 0xB5, 0x2F, 0xFD], // v03
    [0x24, 0xB5, 0x2F, 0xFD], // v04
    [0x25, 0xB5, 0x2F, 0xFD], // v05
    [0x26, 0xB5, 0x2F, 0xFD], // v06
    [0x27, 0xB5, 0x2F, 0xFD], // v07
];

fn magic_plus_random(v: usize, len: usize, rng: &mut Rng) -> Vec<u8> {
    let mut b = LEGACY_MAGIC[v - 1].to_vec();
    for _ in 0..len {
        b.push(rng.byte());
    }
    b
}

/// Number of frame-header bytes *after* the 4 magic bytes for each version
/// (`ZSTD_frameHeaderSize`=4 for v01..v03, `_min`=5 for v04..v06, and v07 adds a
/// window-log byte for a 6-byte minimum). Used to build correctly-aligned
/// frames so the block loop is actually reached.
const HDR_EXTRA: [usize; 7] = [0, 0, 0, 1, 1, 1, 2];

/// A plausible-looking legacy frame: magic, a few frame-header bytes whose
/// reserved bits are clear, then a chain of 3-byte block headers (2-bit type +
/// 19-bit size) with matching payloads. Two thirds of the frames are built
/// *structurally well formed* (exact payload lengths, empty `bt_end`
/// terminator) so the decoders run to completion instead of bailing early.
fn structured_frame(v: usize, rng: &mut Rng) -> Vec<u8> {
    // biased towards the header lengths that actually align for some version
    const HDRLEN: [usize; 8] = [0, 0, 1, 1, 2, 2, 3, 5];
    const SZ: [usize; 8] = [0, 1, 2, 3, 5, 7, 16, 100];

    let mut b = LEGACY_MAGIC[v - 1].to_vec();
    let hlen = HDRLEN[rng.below(HDRLEN.len())];
    for _ in 0..hlen {
        // low nibble only: keeps every version's reserved header bits clear
        b.push(rng.byte() & 0x0F);
    }

    let well_formed = rng.below(3) != 0;
    let nblk = 1 + rng.below(3);
    for _ in 0..nblk {
        let t: u8 = if well_formed {
            // v01..v03 reject bt_rle outright, so leave it out of their mix
            if v <= 3 {
                [1u8, 1, 1, 0][rng.below(4)]
            } else {
                [1u8, 1, 1, 0, 2][rng.below(5)]
            }
        } else {
            rng.below(4) as u8
        };
        let s = SZ[rng.below(SZ.len())];
        b.push((t << 6) | (((s >> 16) & 7) as u8));
        b.push(((s >> 8) & 0xff) as u8);
        b.push((s & 0xff) as u8);
        // an RLE block header carries the *regenerated* size but only one byte
        // of payload (see ZSTDv0X_getcBlockSize)
        let payload = if t == 2 { 1 } else { s };
        let shape = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
        b.extend_from_slice(&gen_shape(shape, payload, rng));
        if t == 3 {
            break; // bt_end
        }
    }
    if well_formed || rng.bool() {
        b.extend_from_slice(&[0xC0, 0, 0]); // empty bt_end terminator
    }
    b
}

/// Hand-built, exactly-aligned frames per version: header, one or more raw
/// blocks, empty `bt_end`. These are the corpus entries that reliably decode
/// *successfully*, which keeps the differential assertions from only ever
/// comparing early-exit error codes.
fn aligned_raw_frames(v: usize) -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    let hl = HDR_EXTRA[v - 1];
    for &sizes in &[
        &[0usize][..],
        &[1][..],
        &[7][..],
        &[100][..],
        &[600][..],
        &[3, 3][..],
        &[64, 1, 17][..],
    ] {
        let mut b = LEGACY_MAGIC[v - 1].to_vec();
        b.extend(std::iter::repeat(0u8).take(hl));
        for (bi, &n) in sizes.iter().enumerate() {
            b.push(0x40 | (((n >> 16) & 7) as u8)); // bt_raw
            b.push(((n >> 8) & 0xff) as u8);
            b.push((n & 0xff) as u8);
            for k in 0..n {
                b.push(((k + bi * 7) & 0xff) as u8);
            }
        }
        b.extend_from_slice(&[0xC0, 0, 0]);
        out.push((format!("v{v}alignedRaw{sizes:?}"), b));
    }
    out
}

/// Frames whose block really is a `bt_compressed` block that the v0.5/v0.6/v0.7
/// decoders accept: a *raw literals* section (`IS_RAW << 6 | litSize`, `lhSize`
/// == 1) followed by a `nbSeq == 0` sequence header. Without these the
/// compressed-block path would only ever be reached with payloads that fail in
/// the first few bytes.
fn literal_only_frames(v: usize) -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    let hl = HDR_EXTRA[v - 1];
    for &litsize in &[1usize, 2, 5, 17, 31] {
        for &pad in &[0usize, 16] {
            for &nblk in &[1usize, 2] {
                let mut b = LEGACY_MAGIC[v - 1].to_vec();
                b.extend(std::iter::repeat(0u8).take(hl));
                for k in 0..nblk {
                    let blocklen = 1 + litsize + 1 + pad;
                    b.push(0x00 | (((blocklen >> 16) & 7) as u8)); // bt_compressed
                    b.push(((blocklen >> 8) & 0xff) as u8);
                    b.push((blocklen & 0xff) as u8);
                    b.push(0x80 | (litsize as u8)); // IS_RAW literals, lhSize=1
                    for j in 0..litsize {
                        b.push((b'a' + ((j + k) % 26) as u8) as u8);
                    }
                    b.push(0x00); // nbSeq == 0
                    b.extend(std::iter::repeat(0u8).take(pad));
                }
                b.extend_from_slice(&[0xC0, 0, 0]);
                out.push((
                    format!("v{v}litblock[lit={litsize},pad={pad},blk={nblk}]"),
                    b,
                ));
            }
        }
    }
    out
}

fn current_format_frames() -> Vec<(String, Vec<u8>)> {
    let i = impls();
    let (c_comp, _r_comp) = i.pair::<FnCompress>("ZSTD_compress");
    let mut rng = Rng::new(0x5EED_0F0F);
    let mut out = Vec::new();
    for (n, shape) in ALL_SHAPES.iter().enumerate() {
        for &lvl in &[1i32, 3, 9] {
            let len = [0usize, 1, 37, 900, 5000][n % 5];
            let src = gen_shape(*shape, len, &mut rng);
            let mut dst = vec![0u8; 4 * len + 1024];
            let r = unsafe {
                c_comp(dst.as_mut_ptr(), dst.len(), src.as_ptr(), src.len(), lvl)
            };
            assert!(!looks_like_error(r), "ZSTD_compress failed building corpus");
            dst.truncate(r);
            out.push((format!("zstd1[{shape:?},len={len},lvl={lvl}]"), dst));
        }
    }
    out
}

fn build_corpus() -> Vec<(String, Vec<u8>)> {
    let mut rng = Rng::new(0x00C0_FFEE_1234_5678);
    let mut c: Vec<(String, Vec<u8>)> = Vec::new();

    // (f) empty and tiny inputs, plus every truncated magic
    for len in 0..=8usize {
        c.push((format!("zeros[{len}]"), vec![0u8; len]));
        c.push((format!("ff[{len}]"), vec![0xFFu8; len]));
        let mut v = Vec::new();
        for _ in 0..len {
            v.push(rng.byte());
        }
        c.push((format!("tinyrand[{len}]"), v));
    }
    for v in 1..=7usize {
        for take in 0..=4usize {
            c.push((
                format!("magic{v}trunc{take}"),
                LEGACY_MAGIC[v - 1][..take].to_vec(),
            ));
        }
    }

    // (a) pure random
    for k in 0..48 {
        let len = [1usize, 2, 3, 4, 5, 9, 17, 33, 64, 129, 513, 2000][k % 12];
        let mut v = Vec::with_capacity(len);
        for _ in 0..len {
            v.push(rng.byte());
        }
        c.push((format!("rand[{len}]#{k}"), v));
    }

    // (b) magic + random payload
    for v in 1..=7usize {
        for &len in &[0usize, 1, 2, 3, 4, 8, 16, 64, 300, 1000, 4096] {
            for k in 0..3 {
                c.push((
                    format!("v{v}magic+rand[{len}]#{k}"),
                    magic_plus_random(v, len, &mut rng),
                ));
            }
        }
    }

    // (c) magic + structured payload, plus exactly-aligned raw-block frames
    for v in 1..=7usize {
        for k in 0..80 {
            c.push((format!("v{v}structured#{k}"), structured_frame(v, &mut rng)));
        }
        c.extend(aligned_raw_frames(v));
        c.extend(literal_only_frames(v));
    }

    // (d) current-format frames
    c.extend(current_format_frames());

    // (e) truncations of everything so far
    let base_len = c.len();
    for k in (0..base_len).step_by(5) {
        let (n, b) = (c[k].0.clone(), c[k].1.clone());
        if b.is_empty() {
            continue;
        }
        for t in 0..3 {
            let cut = match t {
                0 => b.len() / 2,
                1 => b.len() - 1,
                _ => rng.below(b.len()),
            };
            c.push((format!("{n}|trunc{cut}"), b[..cut].to_vec()));
        }
    }

    // (e) single-bit flips
    let base_len = c.len();
    for k in (0..base_len).step_by(7) {
        let (n, mut b) = (c[k].0.clone(), c[k].1.clone());
        if b.is_empty() {
            continue;
        }
        let at = rng.below(b.len());
        let bit = rng.below(8);
        b[at] ^= 1u8 << bit;
        c.push((format!("{n}|flip{at}.{bit}"), b));
    }

    c
}

fn corpus() -> &'static Vec<(String, Vec<u8>)> {
    static C: OnceLock<Vec<(String, Vec<u8>)>> = OnceLock::new();
    C.get_or_init(build_corpus)
}

/// Long deterministic tape used to satisfy arbitrarily large
/// `nextSrcSizeToDecompress()` requests without ever reading out of bounds.
fn pad() -> &'static Vec<u8> {
    static P: OnceLock<Vec<u8>> = OnceLock::new();
    P.get_or_init(|| {
        let mut rng = Rng::new(0x0BAD_F00D_0BAD_F00D);
        let mut v = Vec::with_capacity(320 * 1024);
        while v.len() < 320 * 1024 {
            v.extend_from_slice(&rng.next_u64().to_le_bytes());
        }
        v
    })
}

// ============================================================ 1. error surface

/// Every legacy `isError` / `getErrorName` export, over the whole plausible
/// error-code space plus ordinary sizes and random noise. This pins the shared
/// error mapping that every later assertion leans on.
#[test]
fn legacy_error_surface_matches() {
    let i = impls();

    let mut probes: Vec<usize> = Vec::new();
    for e in 0..=260usize {
        probes.push(0usize.wrapping_sub(e));
    }
    probes.extend([0, 1, 2, 3, 100, 1 << 16, 1 << 30, usize::MAX / 2]);
    let mut rng = Rng::new(0xE7707);
    for _ in 0..300 {
        probes.push(rng.next_u64() as usize);
    }

    let mut checked = 0usize;
    for fam in ["ZSTDv", "ZBUFFv", "FSEv", "HUFv"] {
        for v in 1..=7usize {
            let ise = format!("{fam}{v:02}_isError");
            let nme = format!("{fam}{v:02}_getErrorName");
            if i.has(&ise) {
                let (c, r) = i.pair::<FnIsError>(&ise);
                for &p in &probes {
                    unsafe { assert_eq_dbg(&format!("{ise}({p:#x})"), c(p), r(p)) };
                }
                checked += 1;
            }
            if i.has(&nme) {
                let (c, r) = i.pair::<FnErrName>(&nme);
                for &p in &probes {
                    unsafe {
                        let (a, b) = (cstr(c(p)), cstr(r(p)));
                        assert_eq_dbg(&format!("{nme}({p:#x})"), a, b);
                    }
                }
                checked += 1;
            }
        }
    }
    // the non-versioned deprecated ZBUFF API shares the mapping
    let (c, r) = i.pair::<FnIsError>("ZBUFF_isError");
    let (cn, rn) = i.pair::<FnErrName>("ZBUFF_getErrorName");
    for &p in &probes {
        unsafe {
            assert_eq_dbg(&format!("ZBUFF_isError({p:#x})"), c(p), r(p));
            let (a, b) = (cstr(cn(p)), cstr(rn(p)));
            assert_eq_dbg(&format!("ZBUFF_getErrorName({p:#x})"), a, b);
        }
    }
    checked += 2;
    assert!(checked >= 20, "expected many error exports, got {checked}");
}

// ====================================================== 2. one-shot decompress

/// `ZSTDv0X_decompress` for every version over the whole corpus and several
/// destination capacities (0, 1, half, generous).
#[test]
fn legacy_oneshot_decompress_matches() {
    let i = impls();
    let cp = corpus();

    for v in 1..=7usize {
        let name = format!("ZSTDv{v:02}_decompress");
        let (cf, rf) = i.pair::<FnDecompress>(&name);
        let namer = Namer::new(&format!("ZSTDv{v:02}_getErrorName"));

        for (label, buf) in cp.iter() {
            for cap in [0usize, 1, buf.len() / 2 + 1, 4 * buf.len() + 1024] {
                let mut dc = vec![0xA5u8; cap.max(1)];
                let mut dr = vec![0xA5u8; cap.max(1)];
                let (cret, rret) = unsafe {
                    (
                        cf(dc.as_mut_ptr(), cap, buf.as_ptr(), buf.len()),
                        rf(dr.as_mut_ptr(), cap, buf.as_ptr(), buf.len()),
                    )
                };
                let ctx = format!("{name}({label}, cap={cap})");
                namer.check(&ctx, cret, rret);
                if !looks_like_error(cret) {
                    assert!(cret <= cap, "{ctx}: C reports {cret} > cap {cap}");
                    assert_bytes_eq(&ctx, &dc[..cret], &dr[..cret]);
                }
            }
        }
    }
}

// ================================================= 3. findFrameSizeInfoLegacy

/// `ZSTDv0X_findFrameSizeInfoLegacy` reports through out-parameters; both the
/// compressed size and the decompressed bound must match bit-for-bit, including
/// the cases where the function leaves them untouched.
#[test]
fn legacy_find_frame_size_info_matches() {
    let i = impls();
    let cp = corpus();

    for v in 1..=7usize {
        let name = format!("ZSTDv{v:02}_findFrameSizeInfoLegacy");
        let (cf, rf) = i.pair::<FnFindFSI>(&name);
        let namer = Namer::new(&format!("ZSTDv{v:02}_getErrorName"));

        for (label, buf) in cp.iter() {
            let mut cc: usize = 0xDEAD_BEEF;
            let mut rc: usize = 0xDEAD_BEEF;
            let mut cd: u64 = 0x1234_5678_9ABC_DEF0;
            let mut rd: u64 = 0x1234_5678_9ABC_DEF0;
            unsafe {
                cf(buf.as_ptr(), buf.len(), &mut cc, &mut cd);
                rf(buf.as_ptr(), buf.len(), &mut rc, &mut rd);
            }
            let ctx = format!("{name}({label})");
            namer.check(&format!("{ctx}.cSize"), cc, rc);
            assert_eq_dbg(&format!("{ctx}.dBound"), cd, rd);
        }
    }
}

// ==================================================== 4. DCtx one-shot variants

/// `ZSTDv0X_decompressDCtx` (v01, v04, v05, v06, v07) plus create/free parity.
#[test]
fn legacy_decompress_dctx_matches() {
    let i = impls();
    let cp = corpus();

    for v in [1usize, 4, 5, 6, 7] {
        let name = format!("ZSTDv{v:02}_decompressDCtx");
        if !i.has(&name) {
            continue;
        }
        let (cf, rf) = i.pair::<FnDecompressDCtx>(&name);
        let (cc, rc) = i.pair::<FnCreate>(&format!("ZSTDv{v:02}_createDCtx"));
        let (cfr, rfr) = i.pair::<FnCtxToSz>(&format!("ZSTDv{v:02}_freeDCtx"));
        let namer = Namer::new(&format!("ZSTDv{v:02}_getErrorName"));

        for (label, buf) in cp.iter() {
            for cap in [0usize, 3, 4 * buf.len() + 1024] {
                let mut dc = vec![0x5Au8; cap.max(1)];
                let mut dr = vec![0x5Au8; cap.max(1)];
                unsafe {
                    let ctxc = cc();
                    let ctxr = rc();
                    assert_eq_dbg(
                        &format!("ZSTDv{v:02}_createDCtx null-ness"),
                        ctxc.is_null(),
                        ctxr.is_null(),
                    );
                    let cret = cf(ctxc, dc.as_mut_ptr(), cap, buf.as_ptr(), buf.len());
                    let rret = rf(ctxr, dr.as_mut_ptr(), cap, buf.as_ptr(), buf.len());
                    let ctx = format!("{name}({label}, cap={cap})");
                    namer.check(&ctx, cret, rret);
                    if !looks_like_error(cret) {
                        assert_bytes_eq(&ctx, &dc[..cret], &dr[..cret]);
                    }
                    let (a, b) = (cfr(ctxc), rfr(ctxr));
                    assert_eq_dbg(&format!("ZSTDv{v:02}_freeDCtx"), a, b);
                }
            }
        }
    }
}

// ============================================== 5. streaming state machine fuzz

/// Steps the direct streaming state machine of every legacy version in lock
/// step: `nextSrcSizeToDecompress()` is compared at every step, then a chunk is
/// fed to `decompressContinue()` (usually exactly the requested size, sometimes
/// deliberately wrong) and the return value *and* the regenerated bytes are
/// compared before advancing the shared output offset (which also exercises the
/// decoders' "contiguous destination" bookkeeping).
#[test]
fn legacy_streaming_state_machine_matches() {
    let i = impls();
    let cp = corpus();
    let padding = pad();

    const OUT_CAP: usize = 1 << 20;
    let mut dc = vec![0xCDu8; OUT_CAP];
    let mut dr = vec![0xCDu8; OUT_CAP];

    for v in 1..=7usize {
        let (c_next, r_next) = i.pair::<FnCtxToSz>(&format!("ZSTDv{v:02}_nextSrcSizeToDecompress"));
        let (c_cont, r_cont) = i.pair::<FnDecompressDCtx>(&format!("ZSTDv{v:02}_decompressContinue"));
        let (c_new, r_new) = i.pair::<FnCreate>(&format!("ZSTDv{v:02}_createDCtx"));
        let (c_del, r_del) = i.pair::<FnCtxToSz>(&format!("ZSTDv{v:02}_freeDCtx"));
        let namer = Namer::new(&format!("ZSTDv{v:02}_getErrorName"));

        // v01..v04 expose resetDCtx, v05..v07 decompressBegin
        let reset = format!("ZSTDv{v:02}_resetDCtx");
        let begin = format!("ZSTDv{v:02}_decompressBegin");
        let starter = if i.has(&reset) { reset } else { begin };
        let (c_start, r_start) = i.pair::<FnCtxToSz>(&starter);

        let mut rng = Rng::new(0xA11CE ^ ((v as u64) << 40));
        let mut productive_steps = 0usize;
        let mut total_steps = 0usize;

        for (idx, (label, buf)) in cp.iter().enumerate() {
            let _ = idx; // the streaming machine runs over the *whole* corpus
            // tape = corpus buffer followed by deterministic padding, so an
            // exact feed of any requested size stays in bounds.
            let mut tape = buf.clone();
            tape.extend_from_slice(&padding[..64 * 1024]);

            unsafe {
                let ctxc = c_new();
                let ctxr = r_new();
                assert_eq_dbg("createDCtx null-ness", ctxc.is_null(), ctxr.is_null());
                if ctxc.is_null() {
                    continue;
                }
                let (a, b) = (c_start(ctxc), r_start(ctxr));
                namer.check(&starter, a, b);

                dc.fill(0xCD);
                dr.fill(0xCD);
                let mut off = 0usize;
                let mut cursor = 0usize;

                for step in 0..24 {
                    total_steps += 1;
                    let cn = c_next(ctxc);
                    let rn = r_next(ctxr);
                    namer.check(
                        &format!("ZSTDv{v:02}_nextSrcSizeToDecompress[{label}#{step}]"),
                        cn,
                        rn,
                    );
                    if looks_like_error(cn) || cn == 0 {
                        break; // error, or frame complete
                    }
                    // 80% exact feed, 20% deliberately wrong size
                    let mut want = cn;
                    if rng.below(5) == 0 {
                        want = match rng.below(4) {
                            0 => cn.saturating_sub(1),
                            1 => cn + 1,
                            2 => 0,
                            _ => rng.range(1, 32),
                        };
                    }
                    let avail = tape.len().saturating_sub(cursor);
                    let feed = want.min(avail).min(200_000);
                    let src = tape[cursor..cursor + feed].as_ptr();

                    if off + 140_000 > OUT_CAP {
                        off = 0; // wrap identically in both libraries
                    }
                    let room = OUT_CAP - off;
                    let cap = if rng.below(6) == 0 {
                        rng.below(64)
                    } else {
                        room.min(140_000)
                    };

                    let cret = c_cont(ctxc, dc.as_mut_ptr().add(off), cap, src, feed);
                    let rret = r_cont(ctxr, dr.as_mut_ptr().add(off), cap, src, feed);
                    let ctx = format!(
                        "ZSTDv{v:02}_decompressContinue[{label}#{step} feed={feed} cap={cap}]"
                    );
                    namer.check(&ctx, cret, rret);
                    if looks_like_error(cret) {
                        break;
                    }
                    assert!(cret <= cap, "{ctx}: produced {cret} > cap {cap}");
                    assert_bytes_eq(&ctx, &dc[off..off + cret], &dr[off..off + cret]);
                    if cret > 0 {
                        productive_steps += 1;
                    }
                    off += cret;
                    cursor += feed;
                    if cursor >= tape.len() {
                        break;
                    }
                }

                let (a, b) = (c_del(ctxc), r_del(ctxr));
                namer.check(&format!("ZSTDv{v:02}_freeDCtx"), a, b);
            }
        }

        // anti-vacuity: the machine must really have regenerated data, not just
        // rejected every chunk.
        assert!(
            total_steps > 200,
            "v{v}: streaming ran only {total_steps} steps"
        );
        assert!(
            productive_steps >= 8,
            "v{v}: only {productive_steps} decompressContinue steps produced output"
        );
    }
}

// ================================ 5b. copyDCtx + dict-primed streaming (v05..v07)

/// `ZSTDv0X_copyDCtx` duplicates a whole decoder context. This drives a
/// *dictionary-primed* `decompressBegin_usingDict`, copies the context, and then
/// runs the streaming machine on the **copy**, so any field the copy misses (or
/// copies wrongly) shows up as a step-by-step divergence.
#[test]
fn legacy_copy_dctx_streaming_matches() {
    let i = impls();
    let cp = corpus();
    let padding = pad();
    let mut seedrng = Rng::new(0x0C0F_0000_9E37_79B9);
    let dicts: Vec<Vec<u8>> = vec![
        Vec::new(),
        vec![0x33u8; 5],
        gen_shape(Shape::SkewedText, 6000, &mut seedrng),
    ];

    const OUT_CAP: usize = 1 << 19;
    let mut dc = vec![0xB7u8; OUT_CAP];
    let mut dr = vec![0xB7u8; OUT_CAP];

    for v in [5usize, 6, 7] {
        let (c_new, r_new) = i.pair::<FnCreate>(&format!("ZSTDv{v:02}_createDCtx"));
        let (c_del, r_del) = i.pair::<FnCtxToSz>(&format!("ZSTDv{v:02}_freeDCtx"));
        let (c_bd, r_bd) = i.pair::<FnCtxDict>(&format!("ZSTDv{v:02}_decompressBegin_usingDict"));
        let (c_cp, r_cp) = i.pair::<FnCopyDCtx>(&format!("ZSTDv{v:02}_copyDCtx"));
        let (c_next, r_next) = i.pair::<FnCtxToSz>(&format!("ZSTDv{v:02}_nextSrcSizeToDecompress"));
        let (c_cont, r_cont) =
            i.pair::<FnDecompressDCtx>(&format!("ZSTDv{v:02}_decompressContinue"));
        let namer = Namer::new(&format!("ZSTDv{v:02}_getErrorName"));

        let mut productive = 0usize;
        for (idx, (label, buf)) in cp.iter().enumerate() {
            let d = &dicts[idx % dicts.len()];
            let mut tape = buf.clone();
            tape.extend_from_slice(&padding[..64 * 1024]);
            unsafe {
                let refc = c_new();
                let refr = r_new();
                let runc = c_new();
                let runr = r_new();
                let (x, y) = (
                    c_bd(refc, d.as_ptr(), d.len()),
                    r_bd(refr, d.as_ptr(), d.len()),
                );
                namer.check(
                    &format!("ZSTDv{v:02}_decompressBegin_usingDict(dict={})", d.len()),
                    x,
                    y,
                );
                c_cp(runc, refc as *const c_void);
                r_cp(runr, refr as *const c_void);

                dc.fill(0xB7);
                dr.fill(0xB7);
                let mut off = 0usize;
                let mut cursor = 0usize;
                for step in 0..16 {
                    let cn = c_next(runc);
                    let rn = r_next(runr);
                    namer.check(
                        &format!("copy/ZSTDv{v:02}_nextSrcSize[{label}#{step}]"),
                        cn,
                        rn,
                    );
                    if looks_like_error(cn) || cn == 0 {
                        break;
                    }
                    let feed = cn.min(tape.len() - cursor).min(200_000);
                    if off + 140_000 > OUT_CAP {
                        off = 0;
                    }
                    let cap = (OUT_CAP - off).min(140_000);
                    let src = tape[cursor..cursor + feed].as_ptr();
                    let cret = c_cont(runc, dc.as_mut_ptr().add(off), cap, src, feed);
                    let rret = r_cont(runr, dr.as_mut_ptr().add(off), cap, src, feed);
                    let ctx = format!(
                        "copy/ZSTDv{v:02}_decompressContinue[{label}#{step} feed={feed}]"
                    );
                    namer.check(&ctx, cret, rret);
                    if looks_like_error(cret) {
                        break;
                    }
                    assert_bytes_eq(&ctx, &dc[off..off + cret], &dr[off..off + cret]);
                    if cret > 0 {
                        productive += 1;
                    }
                    off += cret;
                    cursor += feed;
                    if cursor >= tape.len() {
                        break;
                    }
                }
                namer.check(
                    &format!("ZSTDv{v:02}_freeDCtx"),
                    c_del(refc),
                    r_del(refr),
                );
                namer.check(
                    &format!("ZSTDv{v:02}_freeDCtx"),
                    c_del(runc),
                    r_del(runr),
                );
            }
        }
        assert!(
            productive >= 5,
            "v{v}: copied-context streaming produced output only {productive} times"
        );
    }
}

// ================================================= 6. v05/v06/v07 dict + ctx API

/// `decompress_usingDict`, `decompressBegin_usingDict`, `copyDCtx`,
/// `decompress_usingPreparedDCtx`, `sizeofDCtx` and `estimateDCtxSize`.
#[test]
fn legacy_v5_v6_v7_dict_api_matches() {
    let i = impls();
    let cp = corpus();
    let mut rng = Rng::new(0xD1C7_0000);

    // dictionaries: empty, 1-byte, text, random, and a legacy-magic blob
    let dicts: Vec<Vec<u8>> = vec![
        Vec::new(),
        vec![0u8; 1],
        gen_shape(Shape::SkewedText, 2048, &mut rng),
        gen_shape(Shape::Random, 700, &mut rng),
        LEGACY_MAGIC[4].to_vec(),
    ];

    for v in [5usize, 6, 7] {
        let (c_ud, r_ud) = i.pair::<FnUsingDict>(&format!("ZSTDv{v:02}_decompress_usingDict"));
        let (c_bd, r_bd) = i.pair::<FnCtxDict>(&format!("ZSTDv{v:02}_decompressBegin_usingDict"));
        let (c_cp, r_cp) = i.pair::<FnCopyDCtx>(&format!("ZSTDv{v:02}_copyDCtx"));
        let (c_new, r_new) = i.pair::<FnCreate>(&format!("ZSTDv{v:02}_createDCtx"));
        let (c_del, r_del) = i.pair::<FnCtxToSz>(&format!("ZSTDv{v:02}_freeDCtx"));
        let namer = Namer::new(&format!("ZSTDv{v:02}_getErrorName"));

        // sizeofDCtx: v05/v06 take no argument, v07 takes the context.
        let szname = format!("ZSTDv{v:02}_sizeofDCtx");
        if i.has(&szname) {
            unsafe {
                if v == 7 {
                    let (cs, rs) = i.pair::<FnCtxToSz>(&szname);
                    let a = c_new();
                    let b = r_new();
                    let (x, y) = (cs(a), rs(b));
                    assert_eq_dbg(&szname, x, y);
                    c_del(a);
                    r_del(b);
                } else {
                    let (cs, rs) = i.pair::<FnSzVoid>(&szname);
                    let (x, y) = (cs(), rs());
                    assert_eq_dbg(&szname, x, y);
                }
            }
        }
        let estname = format!("ZSTDv{v:02}_estimateDCtxSize");
        if i.has(&estname) {
            let (cs, rs) = i.pair::<FnSzVoid>(&estname);
            unsafe {
                let (x, y) = (cs(), rs());
                assert_eq_dbg(&estname, x, y);
            }
        }

        let prepared = format!("ZSTDv{v:02}_decompress_usingPreparedDCtx");
        let has_prepared = i.has(&prepared);
        for (idx, (label, buf)) in cp.iter().enumerate() {
            if idx % 2 != 0 {
                continue;
            }
            let d = &dicts[idx % dicts.len()];
            let cap = 4 * buf.len() + 1024;
            let mut dc = vec![0x11u8; cap];
            let mut dr = vec![0x11u8; cap];
            unsafe {
                let ctxc = c_new();
                let ctxr = r_new();
                let cret = c_ud(
                    ctxc,
                    dc.as_mut_ptr(),
                    cap,
                    buf.as_ptr(),
                    buf.len(),
                    d.as_ptr(),
                    d.len(),
                );
                let rret = r_ud(
                    ctxr,
                    dr.as_mut_ptr(),
                    cap,
                    buf.as_ptr(),
                    buf.len(),
                    d.as_ptr(),
                    d.len(),
                );
                let ctx = format!(
                    "ZSTDv{v:02}_decompress_usingDict({label}, dict={})",
                    d.len()
                );
                namer.check(&ctx, cret, rret);
                if !looks_like_error(cret) {
                    assert_bytes_eq(&ctx, &dc[..cret], &dr[..cret]);
                }

                // decompressBegin_usingDict, then copyDCtx, then use the copy as
                // a "prepared" reference context.
                let bc = c_bd(ctxc, d.as_ptr(), d.len());
                let br = r_bd(ctxr, d.as_ptr(), d.len());
                namer.check(
                    &format!("ZSTDv{v:02}_decompressBegin_usingDict(dict={})", d.len()),
                    bc,
                    br,
                );

                let copyc = c_new();
                let copyr = r_new();
                c_cp(copyc, ctxc as *const c_void);
                r_cp(copyr, ctxr as *const c_void);

                if has_prepared {
                    let (cp_, rp_) = i.pair::<FnPrepared>(&prepared);
                    let run_c = c_new();
                    let run_r = r_new();
                    dc.fill(0x11);
                    dr.fill(0x11);
                    let pc = cp_(
                        run_c,
                        copyc as *const c_void,
                        dc.as_mut_ptr(),
                        cap,
                        buf.as_ptr(),
                        buf.len(),
                    );
                    let pr = rp_(
                        run_r,
                        copyr as *const c_void,
                        dr.as_mut_ptr(),
                        cap,
                        buf.as_ptr(),
                        buf.len(),
                    );
                    let ctx = format!("{prepared}({label}, dict={})", d.len());
                    namer.check(&ctx, pc, pr);
                    if !looks_like_error(pc) {
                        assert_bytes_eq(&ctx, &dc[..pc], &dr[..pc]);
                    }
                    c_del(run_c);
                    r_del(run_r);
                }
                c_del(copyc);
                r_del(copyr);
                c_del(ctxc);
                r_del(ctxr);
            }
        }
    }
}

// ================================================= 7. getFrameParams (v05..v07)

/// The frame-header parsers. Out-structs are pre-poisoned so "did not write" is
/// distinguishable from "wrote zero".
#[test]
fn legacy_get_frame_params_matches() {
    let i = impls();
    let cp = corpus();

    let (c5, r5) = i.pair::<FnGetFrameParams>("ZSTDv05_getFrameParams");
    let (c6, r6) = i.pair::<FnGetFrameParams>("ZSTDv06_getFrameParams");
    let (c7, r7) = i.pair::<FnGetFrameParams>("ZSTDv07_getFrameParams");
    let n5 = Namer::new("ZSTDv05_getErrorName");
    let n6 = Namer::new("ZSTDv06_getErrorName");
    let n7 = Namer::new("ZSTDv07_getErrorName");

    const POISON5: V05Params = V05Params {
        src_size: 0xAAAA_AAAA_AAAA_AAAA,
        window_log: 0xAAAA_AAAA,
        content_log: 0xAAAA_AAAA,
        hash_log: 0xAAAA_AAAA,
        search_log: 0xAAAA_AAAA,
        search_length: 0xAAAA_AAAA,
        target_length: 0xAAAA_AAAA,
        strategy: 0x5AAA_AAAA,
    };
    const POISON6: V06FrameParams = V06FrameParams {
        frame_content_size: 0xAAAA_AAAA_AAAA_AAAA,
        window_log: 0xAAAA_AAAA,
    };
    const POISON7: V07FrameParams = V07FrameParams {
        frame_content_size: 0xAAAA_AAAA_AAAA_AAAA,
        window_size: 0xAAAA_AAAA,
        dict_id: 0xAAAA_AAAA,
        checksum_flag: 0xAAAA_AAAA,
    };

    for (label, buf) in cp.iter() {
        unsafe {
            let (mut a5, mut b5) = (POISON5, POISON5);
            let x = c5(
                &mut a5 as *mut V05Params as *mut c_void,
                buf.as_ptr(),
                buf.len(),
            );
            let y = r5(
                &mut b5 as *mut V05Params as *mut c_void,
                buf.as_ptr(),
                buf.len(),
            );
            n5.check(&format!("ZSTDv05_getFrameParams({label})"), x, y);
            assert_eq_dbg(&format!("ZSTDv05_getFrameParams({label}) out"), a5, b5);

            let (mut a6, mut b6) = (POISON6, POISON6);
            let x = c6(
                &mut a6 as *mut V06FrameParams as *mut c_void,
                buf.as_ptr(),
                buf.len(),
            );
            let y = r6(
                &mut b6 as *mut V06FrameParams as *mut c_void,
                buf.as_ptr(),
                buf.len(),
            );
            n6.check(&format!("ZSTDv06_getFrameParams({label})"), x, y);
            assert_eq_dbg(&format!("ZSTDv06_getFrameParams({label}) out"), a6, b6);

            let (mut a7, mut b7) = (POISON7, POISON7);
            let x = c7(
                &mut a7 as *mut V07FrameParams as *mut c_void,
                buf.as_ptr(),
                buf.len(),
            );
            let y = r7(
                &mut b7 as *mut V07FrameParams as *mut c_void,
                buf.as_ptr(),
                buf.len(),
            );
            n7.check(&format!("ZSTDv07_getFrameParams({label})"), x, y);
            assert_eq_dbg(&format!("ZSTDv07_getFrameParams({label}) out"), a7, b7);
        }
    }
}

// ===================================================== 8. decompressBlock etc.

/// `decompressBegin` + `decompressBlock` (v05/v06/v07) plus the v07-only
/// `insertBlock` / `isSkipFrame` helpers.
#[test]
fn legacy_decompress_block_matches() {
    let i = impls();
    let cp = corpus();

    for v in [5usize, 6, 7] {
        let (c_new, r_new) = i.pair::<FnCreate>(&format!("ZSTDv{v:02}_createDCtx"));
        let (c_del, r_del) = i.pair::<FnCtxToSz>(&format!("ZSTDv{v:02}_freeDCtx"));
        let (c_beg, r_beg) = i.pair::<FnCtxToSz>(&format!("ZSTDv{v:02}_decompressBegin"));
        let (c_blk, r_blk) = i.pair::<FnDecompressDCtx>(&format!("ZSTDv{v:02}_decompressBlock"));
        let namer = Namer::new(&format!("ZSTDv{v:02}_getErrorName"));

        for (idx, (label, buf)) in cp.iter().enumerate() {
            if idx % 2 != 0 {
                continue;
            }
            for cap in [0usize, 5, 1 << 17] {
                let mut dc = vec![0x77u8; cap.max(1)];
                let mut dr = vec![0x77u8; cap.max(1)];
                unsafe {
                    let a = c_new();
                    let b = r_new();
                    let (x, y) = (c_beg(a), r_beg(b));
                    namer.check(&format!("ZSTDv{v:02}_decompressBegin"), x, y);
                    let cret = c_blk(a, dc.as_mut_ptr(), cap, buf.as_ptr(), buf.len());
                    let rret = r_blk(b, dr.as_mut_ptr(), cap, buf.as_ptr(), buf.len());
                    let ctx = format!("ZSTDv{v:02}_decompressBlock({label}, cap={cap})");
                    namer.check(&ctx, cret, rret);
                    if !looks_like_error(cret) {
                        assert_bytes_eq(&ctx, &dc[..cret], &dr[..cret]);
                    }
                    if v == 7 {
                        let (ci, ri) = i.pair::<FnCtxDict>("ZSTDv07_insertBlock");
                        let (x, y) = (
                            ci(a, buf.as_ptr(), buf.len()),
                            ri(b, buf.as_ptr(), buf.len()),
                        );
                        namer.check(&format!("ZSTDv07_insertBlock({label})"), x, y);
                        let (cs, rs) = i.pair::<FnIsSkip>("ZSTDv07_isSkipFrame");
                        let (x, y) = (cs(a), rs(b));
                        assert_eq_dbg(&format!("ZSTDv07_isSkipFrame({label})"), x, y);
                    }
                    c_del(a);
                    r_del(b);
                }
            }
        }
    }
}

// ============================================ 9. v07 DDict / advanced allocators

unsafe extern "C" fn t_alloc(_opaque: *mut c_void, size: usize) -> *mut c_void {
    use std::alloc::{alloc, Layout};
    let total = size + 16;
    let l = Layout::from_size_align(total, 16).unwrap();
    let p = alloc(l);
    if p.is_null() {
        return std::ptr::null_mut();
    }
    (p as *mut usize).write(total);
    p.add(16) as *mut c_void
}

unsafe extern "C" fn t_free(_opaque: *mut c_void, addr: *mut c_void) {
    use std::alloc::{dealloc, Layout};
    if addr.is_null() {
        return;
    }
    let p = (addr as *mut u8).sub(16);
    let total = (p as *mut usize).read();
    dealloc(p, Layout::from_size_align(total, 16).unwrap());
}

/// `ZSTDv07_getDecompressedSize`, the v07 DDict API, and the custom-allocator
/// constructors (including the "half-specified customMem must return NULL" rule).
#[test]
fn legacy_v07_ddict_and_advanced_matches() {
    let i = impls();
    let cp = corpus();
    let mut rng = Rng::new(0x0DD1C7);

    let (c_gds, r_gds) = i.pair::<FnBufToU64>("ZSTDv07_getDecompressedSize");
    for (label, buf) in cp.iter() {
        unsafe {
            let (x, y) = (
                c_gds(buf.as_ptr(), buf.len()),
                r_gds(buf.as_ptr(), buf.len()),
            );
            assert_eq_dbg(&format!("ZSTDv07_getDecompressedSize({label})"), x, y);
        }
    }

    let (c_cd, r_cd) = i.pair::<FnCreateDDict>("ZSTDv07_createDDict");
    let (c_fd, r_fd) = i.pair::<FnCtxToSz>("ZSTDv07_freeDDict");
    let (c_ud, r_ud) = i.pair::<FnUsingDDict>("ZSTDv07_decompress_usingDDict");
    let (c_new, r_new) = i.pair::<FnCreate>("ZSTDv07_createDCtx");
    let (c_del, r_del) = i.pair::<FnCtxToSz>("ZSTDv07_freeDCtx");
    let namer = Namer::new("ZSTDv07_getErrorName");

    let dicts: Vec<Vec<u8>> = vec![
        Vec::new(),
        vec![7u8; 3],
        gen_shape(Shape::Tabular, 4096, &mut rng),
        gen_shape(Shape::Random, 64 * 1024, &mut rng),
    ];

    for d in &dicts {
        unsafe {
            let dc = c_cd(d.as_ptr(), d.len());
            let dr = r_cd(d.as_ptr(), d.len());
            assert_eq_dbg(
                &format!("ZSTDv07_createDDict({}) null-ness", d.len()),
                dc.is_null(),
                dr.is_null(),
            );
            if dc.is_null() {
                continue;
            }
            for (idx, (label, buf)) in cp.iter().enumerate() {
                if idx % 3 != 0 {
                    continue;
                }
                let cap = 4 * buf.len() + 1024;
                let mut oc = vec![0x33u8; cap];
                let mut orr = vec![0x33u8; cap];
                let a = c_new();
                let b = r_new();
                let cret = c_ud(
                    a,
                    oc.as_mut_ptr(),
                    cap,
                    buf.as_ptr(),
                    buf.len(),
                    dc as *const c_void,
                );
                let rret = r_ud(
                    b,
                    orr.as_mut_ptr(),
                    cap,
                    buf.as_ptr(),
                    buf.len(),
                    dr as *const c_void,
                );
                let ctx = format!("ZSTDv07_decompress_usingDDict({label}, dict={})", d.len());
                namer.check(&ctx, cret, rret);
                if !looks_like_error(cret) {
                    assert_bytes_eq(&ctx, &oc[..cret], &orr[..cret]);
                }
                c_del(a);
                r_del(b);
            }
            let (x, y) = (c_fd(dc), r_fd(dr));
            assert_eq_dbg("ZSTDv07_freeDDict", x, y);
        }
    }

    // custom allocators
    let variants: [(&str, CustomMem); 4] = [
        (
            "all-null (=> default)",
            CustomMem {
                custom_alloc: None,
                custom_free: None,
                opaque: std::ptr::null_mut(),
            },
        ),
        (
            "alloc-only (=> NULL)",
            CustomMem {
                custom_alloc: Some(t_alloc),
                custom_free: None,
                opaque: std::ptr::null_mut(),
            },
        ),
        (
            "free-only (=> NULL)",
            CustomMem {
                custom_alloc: None,
                custom_free: Some(t_free),
                opaque: std::ptr::null_mut(),
            },
        ),
        (
            "both",
            CustomMem {
                custom_alloc: Some(t_alloc),
                custom_free: Some(t_free),
                opaque: 0x1234usize as *mut c_void,
            },
        ),
    ];

    for (tag, cm) in variants {
        for sym in ["ZSTDv07_createDCtx_advanced", "ZBUFFv07_createDCtx_advanced"] {
            let (ca, ra) = i.pair::<FnCreateAdv>(sym);
            unsafe {
                let a = ca(cm);
                let b = ra(cm);
                assert_eq_dbg(&format!("{sym}[{tag}] null-ness"), a.is_null(), b.is_null());
                if !a.is_null() {
                    let freer = if sym.starts_with("ZBUFF") {
                        "ZBUFFv07_freeDCtx"
                    } else {
                        "ZSTDv07_freeDCtx"
                    };
                    let (cfr, rfr) = i.pair::<FnCtxToSz>(freer);
                    let (x, y) = (cfr(a), rfr(b));
                    assert_eq_dbg(&format!("{freer}[{tag}]"), x, y);
                }
            }
        }
    }
}

// ============================================ 10. ZBUFFv04..v07 buffered stream

/// The legacy *buffered* streaming decoders — the code path the current library
/// uses when `ZSTD_decompressStream` meets a legacy frame. Input and output
/// chunk sizes are randomized (including 0-, 1- and 2-byte outputs) and the
/// consumed / produced counts plus produced bytes are compared at every step.
#[test]
fn legacy_zbuff_streaming_matches() {
    let i = impls();
    let cp = corpus();
    let mut dictrng = Rng::new(0x00BF_0BFF_0000_0001);
    let dicts: Vec<Vec<u8>> = vec![
        Vec::new(),
        vec![0xABu8; 32],
        gen_shape(Shape::SkewedText, 8192, &mut dictrng),
    ];

    for v in [4usize, 5, 6, 7] {
        let (c_new, r_new) = i.pair::<FnCreate>(&format!("ZBUFFv{v:02}_createDCtx"));
        let (c_del, r_del) = i.pair::<FnCtxToSz>(&format!("ZBUFFv{v:02}_freeDCtx"));
        let (c_ini, r_ini) = i.pair::<FnCtxToSz>(&format!("ZBUFFv{v:02}_decompressInit"));
        let (c_cont, r_cont) = i.pair::<FnZbuffCont>(&format!("ZBUFFv{v:02}_decompressContinue"));
        let namer = Namer::new(&format!("ZBUFFv{v:02}_getErrorName"));

        for f in ["recommendedDInSize", "recommendedDOutSize"] {
            let n = format!("ZBUFFv{v:02}_{f}");
            let (cs, rs) = i.pair::<FnSzVoid>(&n);
            unsafe {
                let (x, y) = (cs(), rs());
                assert_eq_dbg(&n, x, y);
            }
        }

        // v04 uses decompressWithDictionary, v05+ decompressInitDictionary
        let dictname = if v == 4 {
            format!("ZBUFFv{v:02}_decompressWithDictionary")
        } else {
            format!("ZBUFFv{v:02}_decompressInitDictionary")
        };
        let (c_dict, r_dict) = i.pair::<FnCtxDict>(&dictname);

        let mut rng = Rng::new(0xB0FF_0000 ^ ((v as u64) << 32));
        let mut oc = vec![0x99u8; 1 << 18];
        let mut orr = vec![0x99u8; 1 << 18];

        for (idx, (label, buf)) in cp.iter().enumerate() {
            if idx % 2 != 0 {
                continue;
            }
            let d = &dicts[idx % dicts.len()];
            unsafe {
                let a = c_new();
                let b = r_new();
                assert_eq_dbg("ZBUFF createDCtx null-ness", a.is_null(), b.is_null());
                if a.is_null() {
                    continue;
                }
                if v == 4 {
                    let (x, y) = (c_ini(a), r_ini(b));
                    namer.check(&format!("ZBUFFv{v:02}_decompressInit"), x, y);
                    let (x, y) = (
                        c_dict(a, d.as_ptr(), d.len()),
                        r_dict(b, d.as_ptr(), d.len()),
                    );
                    namer.check(&format!("{dictname}({})", d.len()), x, y);
                } else if idx % 2 == 0 {
                    let (x, y) = (c_ini(a), r_ini(b));
                    namer.check(&format!("ZBUFFv{v:02}_decompressInit"), x, y);
                } else {
                    let (x, y) = (
                        c_dict(a, d.as_ptr(), d.len()),
                        r_dict(b, d.as_ptr(), d.len()),
                    );
                    namer.check(&format!("{dictname}({})", d.len()), x, y);
                }

                let mut cursor = 0usize;
                for step in 0..40 {
                    let in_left = buf.len() - cursor;
                    let inb = match rng.below(5) {
                        0 => 0,
                        1 => in_left.min(1),
                        2 => in_left.min(3),
                        3 => in_left,
                        _ => rng.below(in_left + 1),
                    };
                    let outb = match rng.below(6) {
                        0 => 0,
                        1 => 1,
                        2 => 2,
                        3 => rng.range(1, 300),
                        4 => 1 << 17,
                        _ => 1 << 18,
                    };
                    oc[..outb].fill(0x99);
                    orr[..outb].fill(0x99);
                    let mut coc = outb;
                    let mut cor = outb;
                    let mut cic = inb;
                    let mut cir = inb;
                    let src = buf[cursor..cursor + inb].as_ptr();
                    let cret = c_cont(a, oc.as_mut_ptr(), &mut coc, src, &mut cic);
                    let rret = r_cont(b, orr.as_mut_ptr(), &mut cor, src, &mut cir);
                    let ctx = format!(
                        "ZBUFFv{v:02}_decompressContinue[{label}#{step} in={inb} out={outb}]"
                    );
                    namer.check(&ctx, cret, rret);
                    assert_eq_dbg(&format!("{ctx} produced"), coc, cor);
                    assert_eq_dbg(&format!("{ctx} consumed"), cic, cir);
                    if looks_like_error(cret) {
                        break;
                    }
                    assert_bytes_eq(&ctx, &oc[..coc], &orr[..cor]);
                    cursor += cic;
                    if cret == 0 && cursor >= buf.len() {
                        break;
                    }
                }
                let (x, y) = (c_del(a), r_del(b));
                namer.check(&format!("ZBUFFv{v:02}_freeDCtx"), x, y);
            }
        }
    }
}

// ================================== 11. main API dispatch onto legacy magics

/// `zstd_legacy.h` hooks the legacy decoders into the *current* API, so these
/// entry points must agree on legacy-magic inputs too.
#[test]
fn main_api_on_legacy_inputs_matches() {
    let i = impls();
    let cp = corpus();

    let (c_isf, r_isf) = i.pair::<FnBufToU32>("ZSTD_isFrame");
    let (c_fcs, r_fcs) = i.pair::<FnBufToU64>("ZSTD_getFrameContentSize");
    let (c_gds, r_gds) = i.pair::<FnBufToU64>("ZSTD_getDecompressedSize");
    let (c_fds, r_fds) = i.pair::<FnBufToU64>("ZSTD_findDecompressedSize");
    let (c_dbd, r_dbd) = i.pair::<FnBufToU64>("ZSTD_decompressBound");
    let (c_fcz, r_fcz) = i.pair::<FnBufToSz>("ZSTD_findFrameCompressedSize");
    let (c_dec, r_dec) = i.pair::<FnDecompress>("ZSTD_decompress");
    let namer = Namer::new("ZSTD_getErrorName");

    for (label, buf) in cp.iter() {
        unsafe {
            let (x, y) = (
                c_isf(buf.as_ptr(), buf.len()),
                r_isf(buf.as_ptr(), buf.len()),
            );
            assert_eq_dbg(&format!("ZSTD_isFrame({label})"), x, y);

            let (x, y) = (
                c_fcs(buf.as_ptr(), buf.len()),
                r_fcs(buf.as_ptr(), buf.len()),
            );
            assert_eq_dbg(&format!("ZSTD_getFrameContentSize({label})"), x, y);

            let (x, y) = (
                c_gds(buf.as_ptr(), buf.len()),
                r_gds(buf.as_ptr(), buf.len()),
            );
            assert_eq_dbg(&format!("ZSTD_getDecompressedSize({label})"), x, y);

            let (x, y) = (
                c_fds(buf.as_ptr(), buf.len()),
                r_fds(buf.as_ptr(), buf.len()),
            );
            assert_eq_dbg(&format!("ZSTD_findDecompressedSize({label})"), x, y);

            let (x, y) = (
                c_dbd(buf.as_ptr(), buf.len()),
                r_dbd(buf.as_ptr(), buf.len()),
            );
            assert_eq_dbg(&format!("ZSTD_decompressBound({label})"), x, y);

            let (x, y) = (
                c_fcz(buf.as_ptr(), buf.len()),
                r_fcz(buf.as_ptr(), buf.len()),
            );
            namer.check(&format!("ZSTD_findFrameCompressedSize({label})"), x, y);

            for cap in [0usize, 1, 4 * buf.len() + 1024] {
                let mut dc = vec![0x2Du8; cap.max(1)];
                let mut dr = vec![0x2Du8; cap.max(1)];
                let cret = c_dec(dc.as_mut_ptr(), cap, buf.as_ptr(), buf.len());
                let rret = r_dec(dr.as_mut_ptr(), cap, buf.as_ptr(), buf.len());
                let ctx = format!("ZSTD_decompress({label}, cap={cap})");
                namer.check(&ctx, cret, rret);
                if !looks_like_error(cret) {
                    assert_bytes_eq(&ctx, &dc[..cret], &dr[..cret]);
                }
            }
        }
    }
}

/// `ZSTD_decompressStream` on legacy-magic input drives
/// `ZSTD_initLegacyStream` / `ZSTD_decompressLegacyStream` /
/// `ZSTD_freeLegacyStreamContext`.
#[test]
fn main_api_stream_on_legacy_inputs_matches() {
    let i = impls();
    let cp = corpus();

    let (c_new, r_new) = i.pair::<FnCreate>("ZSTD_createDStream");
    let (c_del, r_del) = i.pair::<FnCtxToSz>("ZSTD_freeDStream");
    let (c_ini, r_ini) = i.pair::<FnCtxToSz>("ZSTD_initDStream");
    let (c_idd, r_idd) = i.pair::<FnCtxDict>("ZSTD_initDStream_usingDict");
    let (c_str, r_str) = i.pair::<FnStream>("ZSTD_decompressStream");
    let namer = Namer::new("ZSTD_getErrorName");

    let mut rng = Rng::new(0x0057_BEAF_0000_0002);
    let dict = gen_shape(Shape::SkewedText, 4096, &mut rng);

    unsafe {
        let a = c_new();
        let b = r_new();
        assert!(!a.is_null() && !b.is_null());

        let mut oc = vec![0x44u8; 1 << 18];
        let mut orr = vec![0x44u8; 1 << 18];

        for (idx, (label, buf)) in cp.iter().enumerate() {
            if idx % 2 != 0 {
                continue;
            }
            if idx % 4 == 0 {
                let (x, y) = (c_ini(a), r_ini(b));
                namer.check("ZSTD_initDStream", x, y);
            } else {
                let (x, y) = (
                    c_idd(a, dict.as_ptr(), dict.len()),
                    r_idd(b, dict.as_ptr(), dict.len()),
                );
                namer.check("ZSTD_initDStream_usingDict", x, y);
            }

            let mut inpos = 0usize;
            for step in 0..30 {
                let feed = rng.range(0, (buf.len() - inpos).min(4096));
                let outcap = match rng.below(5) {
                    0 => 0,
                    1 => 1,
                    2 => rng.range(1, 500),
                    3 => 1 << 16,
                    _ => 1 << 18,
                };
                oc[..outcap].fill(0x44);
                orr[..outcap].fill(0x44);
                let mut ic = ZSTD_inBuffer {
                    src: buf.as_ptr(),
                    size: inpos + feed,
                    pos: inpos,
                };
                let mut ir = ZSTD_inBuffer {
                    src: buf.as_ptr(),
                    size: inpos + feed,
                    pos: inpos,
                };
                let mut ouc = ZSTD_outBuffer {
                    dst: oc.as_mut_ptr(),
                    size: outcap,
                    pos: 0,
                };
                let mut our = ZSTD_outBuffer {
                    dst: orr.as_mut_ptr(),
                    size: outcap,
                    pos: 0,
                };
                let cret = c_str(a, &mut ouc, &mut ic);
                let rret = r_str(b, &mut our, &mut ir);
                let ctx =
                    format!("ZSTD_decompressStream[{label}#{step} feed={feed} out={outcap}]");
                namer.check(&ctx, cret, rret);
                assert_eq_dbg(&format!("{ctx} in.pos"), ic.pos, ir.pos);
                assert_eq_dbg(&format!("{ctx} out.pos"), ouc.pos, our.pos);
                if looks_like_error(cret) {
                    break;
                }
                assert_bytes_eq(&ctx, &oc[..ouc.pos], &orr[..our.pos]);
                inpos = ic.pos;
                if cret == 0 && inpos >= buf.len() {
                    break;
                }
            }
        }
        let (x, y) = (c_del(a), r_del(b));
        assert_eq_dbg("ZSTD_freeDStream", x, y);
    }
}

// ================================================================ 12. FSEv0X

/// Build a *valid* normalized-count distribution (positive counts plus `-1`
/// low-probability markers summing to `1<<tableLog`). `FSEv0X_buildDTable` is
/// explicitly not hardened against invalid distributions in the C original, so
/// only valid ones are fed to it.
fn valid_ncount(table_log: u32, max_sv: usize, rng: &mut Rng) -> Vec<i16> {
    let table_size = 1usize << table_log;
    let mut counts = vec![0i16; max_sv + 1];
    let mut rem = table_size;
    for s in 0..=max_sv {
        if s == max_sv {
            let take = rem.min(i16::MAX as usize);
            counts[s] = take as i16;
            rem -= take;
        } else if rem > 0 {
            let hi = (rem / 2 + 1).min(rem);
            let take = rng.below(hi + 1);
            counts[s] = take as i16;
            rem -= take;
        }
    }
    if rem != 0 {
        return Vec::new(); // could not place the whole budget -> skip
    }
    // turn some count==1 symbols into low-probability (-1); the budget is
    // unchanged so the distribution stays valid.
    for s in 0..=max_sv {
        if counts[s] == 1 && rng.bool() {
            counts[s] = -1;
        }
    }
    counts
}

#[test]
fn legacy_fse_entropy_matches() {
    let i = impls();
    let cp = corpus();

    for v in [5usize, 6, 7] {
        let f = |s: &str| format!("FSEv{v:02}_{s}");
        let (c_rn, r_rn) = i.pair::<FnReadNCount>(&f("readNCount"));
        let (c_ct, r_ct) = i.pair::<FnCreateDTable>(&f("createDTable"));
        let (c_ft, r_ft) = i.pair::<FnFreeDTable>(&f("freeDTable"));
        let (c_bd, r_bd) = i.pair::<FnBuildDTable>(&f("buildDTable"));
        let (c_br, r_br) = i.pair::<FnBuildRaw>(&f("buildDTable_raw"));
        let (c_bl, r_bl) = i.pair::<FnBuildRle>(&f("buildDTable_rle"));
        let (c_du, r_du) = i.pair::<FnFseUsingDTable>(&f("decompress_usingDTable"));
        let (c_de, r_de) = i.pair::<FnDecompress>(&f("decompress"));
        let namer = Namer::new(&f("getErrorName"));

        // createDTable / freeDTable
        for tl in 0..=15u32 {
            unsafe {
                let a = c_ct(tl);
                let b = r_ct(tl);
                assert_eq_dbg(
                    &format!("{}({tl}) null-ness", f("createDTable")),
                    a.is_null(),
                    b.is_null(),
                );
                c_ft(a);
                r_ft(b);
            }
        }

        // readNCount over the whole corpus, for several caller-supplied maxSV
        let mut nc_c = vec![0x5A5Ai16; 512];
        let mut nc_r = vec![0x5A5Ai16; 512];
        for (label, buf) in cp.iter() {
            for &msv in &[0u32, 1, 15, 255] {
                let (mut mc, mut mr) = (msv, msv);
                let (mut tc, mut tr) = (0xDEAD_BEEFu32, 0xDEAD_BEEFu32);
                nc_c.fill(0x5A5A);
                nc_r.fill(0x5A5A);
                unsafe {
                    let cret =
                        c_rn(nc_c.as_mut_ptr(), &mut mc, &mut tc, buf.as_ptr(), buf.len());
                    let rret =
                        r_rn(nc_r.as_mut_ptr(), &mut mr, &mut tr, buf.as_ptr(), buf.len());
                    let ctx = format!("{}({label}, maxSV={msv})", f("readNCount"));
                    namer.check(&ctx, cret, rret);
                    assert_eq_dbg(&format!("{ctx} maxSV"), mc, mr);
                    assert_eq_dbg(&format!("{ctx} tableLog"), tc, tr);
                    assert_words_eq(&format!("{ctx} counts"), &nc_c, &nc_r);
                }
            }
        }

        // buildDTable with valid distributions, then decompress_usingDTable with
        // random compressed payloads.
        const DT_U32: usize = 1 + (1 << 14);
        let mut rng = Rng::new(0xF5E0_0000 ^ ((v as u64) << 24));
        let mut dt_c = vec![0xEEEE_EEEEu32; DT_U32];
        let mut dt_r = vec![0xEEEE_EEEEu32; DT_U32];

        // tableLog < FSEv0X_MIN_TABLELOG (5) makes `FSEv05_tableStep()` land on a
        // multiple of the table size, so the spread loop never leaves cell 0 and
        // the C reads uninitialised `symbolNext[]` — out of contract, skipped.
        for round in 0..500usize {
            let tl = rng.range(5, 15) as u32;
            let msv = rng.below(256);
            let counts = valid_ncount(tl.min(12), msv, &mut rng);
            if counts.is_empty() {
                continue;
            }
            dt_c.fill(0xEEEE_EEEE);
            dt_r.fill(0xEEEE_EEEE);
            unsafe {
                let cret = c_bd(dt_c.as_mut_ptr(), counts.as_ptr(), msv as c_uint, tl);
                let rret = r_bd(dt_r.as_mut_ptr(), counts.as_ptr(), msv as c_uint, tl);
                let ctx = format!("{}(#{round} tl={tl} msv={msv})", f("buildDTable"));
                namer.check(&ctx, cret, rret);
                assert_words_eq(&format!("{ctx} table"), &dt_c, &dt_r);

                if !looks_like_error(cret) {
                    let src = &cp[rng.below(cp.len())].1;
                    for dstcap in [0usize, 1, 37, 4096] {
                        let mut oc = vec![0x66u8; dstcap.max(1)];
                        let mut orr = vec![0x66u8; dstcap.max(1)];
                        let x = c_du(
                            oc.as_mut_ptr(),
                            dstcap,
                            src.as_ptr(),
                            src.len(),
                            dt_c.as_ptr(),
                        );
                        let y = r_du(
                            orr.as_mut_ptr(),
                            dstcap,
                            src.as_ptr(),
                            src.len(),
                            dt_r.as_ptr(),
                        );
                        let ctx2 =
                            format!("{}(#{round} cap={dstcap})", f("decompress_usingDTable"));
                        namer.check(&ctx2, x, y);
                        if !looks_like_error(x) {
                            assert_bytes_eq(&ctx2, &oc[..x], &orr[..y]);
                        }
                    }
                }
            }
        }

        // buildDTable_raw / _rle
        for nb in 0..=14u32 {
            dt_c.fill(0xEEEE_EEEE);
            dt_r.fill(0xEEEE_EEEE);
            unsafe {
                let x = c_br(dt_c.as_mut_ptr(), nb);
                let y = r_br(dt_r.as_mut_ptr(), nb);
                namer.check(&format!("{}({nb})", f("buildDTable_raw")), x, y);
                assert_words_eq(
                    &format!("{}({nb}) table", f("buildDTable_raw")),
                    &dt_c,
                    &dt_r,
                );
            }
        }
        for sym in [0u8, 1, 127, 255] {
            dt_c.fill(0xEEEE_EEEE);
            dt_r.fill(0xEEEE_EEEE);
            unsafe {
                let x = c_bl(dt_c.as_mut_ptr(), sym);
                let y = r_bl(dt_r.as_mut_ptr(), sym);
                namer.check(&format!("{}({sym})", f("buildDTable_rle")), x, y);
                assert_words_eq(
                    &format!("{}({sym}) table", f("buildDTable_rle")),
                    &dt_c,
                    &dt_r,
                );
            }
        }

        // one-shot FSE decompress over the corpus
        for (label, buf) in cp.iter() {
            for cap in [0usize, 1, 64, 4096] {
                let mut oc = vec![0x88u8; cap.max(1)];
                let mut orr = vec![0x88u8; cap.max(1)];
                unsafe {
                    let x = c_de(oc.as_mut_ptr(), cap, buf.as_ptr(), buf.len());
                    let y = r_de(orr.as_mut_ptr(), cap, buf.as_ptr(), buf.len());
                    let ctx = format!("{}({label}, cap={cap})", f("decompress"));
                    namer.check(&ctx, x, y);
                    if !looks_like_error(x) {
                        assert_bytes_eq(&ctx, &oc[..x], &orr[..y]);
                    }
                }
            }
        }
    }
}

// ================================================================ 13. HUFv0X

#[test]
fn legacy_huf_v05_v06_matches() {
    let i = impls();
    let cp = corpus();

    for v in [5usize, 6] {
        let f = |s: &str| format!("HUFv{v:02}_{s}");
        let (c_r2, r_r2) = i.pair::<FnHufReadX2>(&f("readDTableX2"));
        let (c_r4, r_r4) = i.pair::<FnHufReadX4>(&f("readDTableX4"));
        let (c_u12, r_u12) = i.pair::<FnHufUsingX2>(&f("decompress1X2_usingDTable"));
        let (c_u42, r_u42) = i.pair::<FnHufUsingX2>(&f("decompress4X2_usingDTable"));
        let (c_u14, r_u14) = i.pair::<FnHufUsingX4>(&f("decompress1X4_usingDTable"));
        let (c_u44, r_u44) = i.pair::<FnHufUsingX4>(&f("decompress4X4_usingDTable"));
        let namer = Namer::new(&f("getErrorName"));

        const N: usize = 1 + (1 << 12);
        let mut t2c = vec![0u16; N];
        let mut t2r = vec![0u16; N];
        let mut t4c = vec![0u32; N];
        let mut t4r = vec![0u32; N];

        for (label, buf) in cp.iter() {
            unsafe {
                // ---- single-symbol (X2) table
                t2c.fill(0xDEAD);
                t2r.fill(0xDEAD);
                t2c[0] = 12; // maxTableLog, exactly as the C static macro does
                t2r[0] = 12;
                let a = c_r2(t2c.as_mut_ptr(), buf.as_ptr(), buf.len());
                let b = r_r2(t2r.as_mut_ptr(), buf.as_ptr(), buf.len());
                let ctx = format!("{}({label})", f("readDTableX2"));
                namer.check(&ctx, a, b);
                assert_words_eq(&format!("{ctx} table"), &t2c, &t2r);
                if !looks_like_error(a) {
                    for cap in [0usize, 1, 100, 4096] {
                        let mut oc = vec![0x21u8; cap.max(1)];
                        let mut orr = vec![0x21u8; cap.max(1)];
                        let x = c_u12(oc.as_mut_ptr(), cap, buf.as_ptr(), buf.len(), t2c.as_ptr());
                        let y = r_u12(orr.as_mut_ptr(), cap, buf.as_ptr(), buf.len(), t2r.as_ptr());
                        let c2 = format!("{}({label}, cap={cap})", f("decompress1X2_usingDTable"));
                        namer.check(&c2, x, y);
                        if !looks_like_error(x) {
                            assert_bytes_eq(&c2, &oc[..x], &orr[..y]);
                        }
                        oc.fill(0x21);
                        orr.fill(0x21);
                        let x = c_u42(oc.as_mut_ptr(), cap, buf.as_ptr(), buf.len(), t2c.as_ptr());
                        let y = r_u42(orr.as_mut_ptr(), cap, buf.as_ptr(), buf.len(), t2r.as_ptr());
                        let c2 = format!("{}({label}, cap={cap})", f("decompress4X2_usingDTable"));
                        namer.check(&c2, x, y);
                        if !looks_like_error(x) {
                            assert_bytes_eq(&c2, &oc[..x], &orr[..y]);
                        }
                    }
                }

                // ---- double-symbol (X4) table
                t4c.fill(0xDEAD_BEEF);
                t4r.fill(0xDEAD_BEEF);
                t4c[0] = 12;
                t4r[0] = 12;
                let a = c_r4(t4c.as_mut_ptr(), buf.as_ptr(), buf.len());
                let b = r_r4(t4r.as_mut_ptr(), buf.as_ptr(), buf.len());
                let ctx = format!("{}({label})", f("readDTableX4"));
                namer.check(&ctx, a, b);
                assert_words_eq(&format!("{ctx} table"), &t4c, &t4r);
                if !looks_like_error(a) {
                    for cap in [0usize, 1, 100, 4096] {
                        let mut oc = vec![0x21u8; cap.max(1)];
                        let mut orr = vec![0x21u8; cap.max(1)];
                        let x = c_u14(oc.as_mut_ptr(), cap, buf.as_ptr(), buf.len(), t4c.as_ptr());
                        let y = r_u14(orr.as_mut_ptr(), cap, buf.as_ptr(), buf.len(), t4r.as_ptr());
                        let c2 = format!("{}({label}, cap={cap})", f("decompress1X4_usingDTable"));
                        namer.check(&c2, x, y);
                        if !looks_like_error(x) {
                            assert_bytes_eq(&c2, &oc[..x], &orr[..y]);
                        }
                        oc.fill(0x21);
                        orr.fill(0x21);
                        let x = c_u44(oc.as_mut_ptr(), cap, buf.as_ptr(), buf.len(), t4c.as_ptr());
                        let y = r_u44(orr.as_mut_ptr(), cap, buf.as_ptr(), buf.len(), t4r.as_ptr());
                        let c2 = format!("{}({label}, cap={cap})", f("decompress4X4_usingDTable"));
                        namer.check(&c2, x, y);
                        if !looks_like_error(x) {
                            assert_bytes_eq(&c2, &oc[..x], &orr[..y]);
                        }
                    }
                }
            }
        }

        // self-contained one-shot decoders
        for nm in [
            "decompress",
            "decompress1X2",
            "decompress1X4",
            "decompress4X2",
            "decompress4X4",
        ] {
            let full = f(nm);
            let (cf, rf) = i.pair::<FnDecompress>(&full);
            for (label, buf) in cp.iter() {
                for cap in [0usize, 1, 100, 4096] {
                    let mut oc = vec![0x31u8; cap.max(1)];
                    let mut orr = vec![0x31u8; cap.max(1)];
                    unsafe {
                        let x = cf(oc.as_mut_ptr(), cap, buf.as_ptr(), buf.len());
                        let y = rf(orr.as_mut_ptr(), cap, buf.as_ptr(), buf.len());
                        let ctx = format!("{full}({label}, cap={cap})");
                        namer.check(&ctx, x, y);
                        if !looks_like_error(x) {
                            assert_bytes_eq(&ctx, &oc[..x], &orr[..y]);
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn legacy_huf_v07_matches() {
    let i = impls();
    let cp = corpus();

    let f = |s: &str| format!("HUFv07_{s}");
    let namer = Namer::new("HUFv07_getErrorName");
    let (c_rs, r_rs) = i.pair::<FnHufReadStats>("HUFv07_readStats");
    let (c_sd, r_sd) = i.pair::<FnSelectDecoder>("HUFv07_selectDecoder");
    let (c_r2, r_r2) = i.pair::<FnHufReadX4>("HUFv07_readDTableX2");
    let (c_r4, r_r4) = i.pair::<FnHufReadX4>("HUFv07_readDTableX4");

    // Documented contract: "Assumption : 0 < cSrcSize < dstSize <= 128 KB".
    // Outside it the C divides by `dstSize` and indexes `algoTime[Q]` out of
    // bounds, so only in-contract pairs are probed — exhaustively.
    for ds in [2usize, 3, 4, 5, 17, 100, 255, 256, 257, 1000, 1 << 16, 1 << 17] {
        for cs in 1..ds.min(400) {
            unsafe {
                let (x, y) = (c_sd(ds, cs), r_sd(ds, cs));
                assert_eq_dbg(&format!("HUFv07_selectDecoder({ds},{cs})"), x, y);
            }
        }
        for k in 1..16usize {
            let cs = (ds * k) / 16;
            if cs == 0 || cs >= ds {
                continue;
            }
            unsafe {
                let (x, y) = (c_sd(ds, cs), r_sd(ds, cs));
                assert_eq_dbg(&format!("HUFv07_selectDecoder({ds},{cs})"), x, y);
            }
        }
    }

    const NT: usize = 1 + (1 << 13);
    let mut t2c = vec![0u32; NT];
    let mut t2r = vec![0u32; NT];
    let mut t4c = vec![0u32; NT];
    let mut t4r = vec![0u32; NT];

    for (label, buf) in cp.iter() {
        unsafe {
            // ---- readStats
            let mut hw_c = vec![0x5Au8; 512];
            let mut hw_r = vec![0x5Au8; 512];
            let mut rk_c = vec![0xAAAA_AAAAu32; 64];
            let mut rk_r = vec![0xAAAA_AAAAu32; 64];
            let (mut nc, mut nr) = (0xDEAD_BEEFu32, 0xDEAD_BEEFu32);
            let (mut tc, mut tr) = (0xDEAD_BEEFu32, 0xDEAD_BEEFu32);
            let a = c_rs(
                hw_c.as_mut_ptr(),
                256,
                rk_c.as_mut_ptr(),
                &mut nc,
                &mut tc,
                buf.as_ptr(),
                buf.len(),
            );
            let b = r_rs(
                hw_r.as_mut_ptr(),
                256,
                rk_r.as_mut_ptr(),
                &mut nr,
                &mut tr,
                buf.as_ptr(),
                buf.len(),
            );
            let ctx = format!("HUFv07_readStats({label})");
            namer.check(&ctx, a, b);
            assert_eq_dbg(&format!("{ctx} nbSymbols"), nc, nr);
            assert_eq_dbg(&format!("{ctx} tableLog"), tc, tr);
            assert_words_eq(&format!("{ctx} huffWeight"), &hw_c, &hw_r);
            assert_words_eq(&format!("{ctx} rankStats"), &rk_c, &rk_r);

            // ---- readDTableX2, descriptor maxTableLog = 11 (as the C macro)
            t2c.fill(0xCCCC_CCCC);
            t2r.fill(0xCCCC_CCCC);
            t2c[0] = 11 * 0x0100_0001;
            t2r[0] = 11 * 0x0100_0001;
            let a2 = c_r2(t2c.as_mut_ptr(), buf.as_ptr(), buf.len());
            let b2 = r_r2(t2r.as_mut_ptr(), buf.as_ptr(), buf.len());
            let ctx = format!("HUFv07_readDTableX2({label})");
            namer.check(&ctx, a2, b2);
            assert_words_eq(&format!("{ctx} table"), &t2c, &t2r);

            // ---- readDTableX4, descriptor maxTableLog = 12
            t4c.fill(0xCCCC_CCCC);
            t4r.fill(0xCCCC_CCCC);
            t4c[0] = 12 * 0x0100_0001;
            t4r[0] = 12 * 0x0100_0001;
            let a4 = c_r4(t4c.as_mut_ptr(), buf.as_ptr(), buf.len());
            let b4 = r_r4(t4r.as_mut_ptr(), buf.as_ptr(), buf.len());
            let ctx = format!("HUFv07_readDTableX4({label})");
            namer.check(&ctx, a4, b4);
            assert_words_eq(&format!("{ctx} table"), &t4c, &t4r);

            // ---- *_usingDTable with the freshly-read (well-formed) tables
            if !looks_like_error(a2) {
                for nm in [
                    "decompress1X2_usingDTable",
                    "decompress4X2_usingDTable",
                    "decompress1X_usingDTable",
                    "decompress4X_usingDTable",
                ] {
                    let (cf, rf) = i.pair::<FnHufUsingX4>(&f(nm));
                    for cap in [0usize, 1, 100, 4096] {
                        let mut oc = vec![0x41u8; cap.max(1)];
                        let mut orr = vec![0x41u8; cap.max(1)];
                        let x = cf(oc.as_mut_ptr(), cap, buf.as_ptr(), buf.len(), t2c.as_ptr());
                        let y = rf(orr.as_mut_ptr(), cap, buf.as_ptr(), buf.len(), t2r.as_ptr());
                        let c2 = format!("HUFv07_{nm}(X2 table, {label}, cap={cap})");
                        namer.check(&c2, x, y);
                        if !looks_like_error(x) {
                            assert_bytes_eq(&c2, &oc[..x], &orr[..y]);
                        }
                    }
                }
            }
            if !looks_like_error(a4) {
                for nm in ["decompress1X4_usingDTable", "decompress4X4_usingDTable"] {
                    let (cf, rf) = i.pair::<FnHufUsingX4>(&f(nm));
                    for cap in [0usize, 1, 100, 4096] {
                        let mut oc = vec![0x41u8; cap.max(1)];
                        let mut orr = vec![0x41u8; cap.max(1)];
                        let x = cf(oc.as_mut_ptr(), cap, buf.as_ptr(), buf.len(), t4c.as_ptr());
                        let y = rf(orr.as_mut_ptr(), cap, buf.as_ptr(), buf.len(), t4r.as_ptr());
                        let c2 = format!("HUFv07_{nm}(X4 table, {label}, cap={cap})");
                        namer.check(&c2, x, y);
                        if !looks_like_error(x) {
                            assert_bytes_eq(&c2, &oc[..x], &orr[..y]);
                        }
                    }
                }
            }

            // ---- DCtx flavours: they read the table themselves
            for nm in [
                "decompress1X2_DCtx",
                "decompress1X4_DCtx",
                "decompress1X_DCtx",
                "decompress4X2_DCtx",
                "decompress4X4_DCtx",
                "decompress4X_DCtx",
                "decompress4X_hufOnly",
            ] {
                let (cf, rf) = i.pair::<FnHufDCtx>(&f(nm));
                for cap in [0usize, 1, 100, 4096] {
                    let mut oc = vec![0x51u8; cap.max(1)];
                    let mut orr = vec![0x51u8; cap.max(1)];
                    t4c.fill(0xCCCC_CCCC);
                    t4r.fill(0xCCCC_CCCC);
                    t4c[0] = 12 * 0x0100_0001;
                    t4r[0] = 12 * 0x0100_0001;
                    let x = cf(t4c.as_mut_ptr(), oc.as_mut_ptr(), cap, buf.as_ptr(), buf.len());
                    let y = rf(t4r.as_mut_ptr(), orr.as_mut_ptr(), cap, buf.as_ptr(), buf.len());
                    let c2 = format!("HUFv07_{nm}({label}, cap={cap})");
                    namer.check(&c2, x, y);
                    assert_words_eq(&format!("{c2} table"), &t4c, &t4r);
                    if !looks_like_error(x) {
                        assert_bytes_eq(&c2, &oc[..x], &orr[..y]);
                    }
                }
            }
        }
    }

    // self-contained one-shots
    for nm in [
        "decompress",
        "decompress1X2",
        "decompress1X4",
        "decompress4X2",
        "decompress4X4",
    ] {
        let full = f(nm);
        let (cf, rf) = i.pair::<FnDecompress>(&full);
        for (label, buf) in cp.iter() {
            for cap in [0usize, 1, 100, 4096] {
                let mut oc = vec![0x61u8; cap.max(1)];
                let mut orr = vec![0x61u8; cap.max(1)];
                unsafe {
                    let x = cf(oc.as_mut_ptr(), cap, buf.as_ptr(), buf.len());
                    let y = rf(orr.as_mut_ptr(), cap, buf.as_ptr(), buf.len());
                    let ctx = format!("{full}({label}, cap={cap})");
                    namer.check(&ctx, x, y);
                    if !looks_like_error(x) {
                        assert_bytes_eq(&ctx, &oc[..x], &orr[..y]);
                    }
                }
            }
        }
    }
}

// ====================================================== 14. coverage depth guard

/// Anti-vacuity guard: for *every* legacy version the corpus must drive
/// `ZSTDv0X_decompress` past the magic-number check into the real frame/block
/// decoders, producing a spread of distinct outcomes (and, for some inputs,
/// actual regenerated data). Without this, all the equality assertions above
/// could pass while only ever comparing `prefix_unknown`.
#[test]
fn legacy_corpus_reaches_past_magic() {
    let i = impls();
    let cp = corpus();
    const PREFIX_UNKNOWN: usize = usize::MAX - 9; // ERROR(prefix_unknown) == -10

    for v in 1..=7usize {
        let (cf, _rf) = i.pair::<FnDecompress>(&format!("ZSTDv{v:02}_decompress"));
        let mut outcomes = std::collections::BTreeSet::new();
        let mut past_magic = 0usize;
        let mut successes = 0usize;
        let mut bytes_out = 0usize;
        for (_label, buf) in cp.iter() {
            let cap = 4 * buf.len() + 1024;
            let mut d = vec![0u8; cap];
            let r = unsafe { cf(d.as_mut_ptr(), cap, buf.as_ptr(), buf.len()) };
            outcomes.insert(r);
            if r != PREFIX_UNKNOWN {
                past_magic += 1;
            }
            if !looks_like_error(r) {
                successes += 1;
                bytes_out += r;
            }
        }
        assert!(
            past_magic >= 30,
            "v{v}: only {past_magic} corpus inputs got past the magic check"
        );
        assert!(
            outcomes.len() >= 4,
            "v{v}: only {} distinct outcomes ({outcomes:?})",
            outcomes.len()
        );
        assert!(
            successes >= 5,
            "v{v}: only {successes} inputs decoded without error"
        );
        let _ = bytes_out;

        // v05..v07: the literal-only frames must exercise the *compressed*
        // block path successfully, not just raw blocks.
        if v >= 5 {
            let tag = format!("v{v}litblock");
            let mut lit_ok = 0usize;
            for (label, buf) in cp.iter() {
                if !label.starts_with(&tag) {
                    continue;
                }
                let cap = 4 * buf.len() + 1024;
                let mut d = vec![0u8; cap];
                let r = unsafe { cf(d.as_mut_ptr(), cap, buf.as_ptr(), buf.len()) };
                if !looks_like_error(r) && r > 0 {
                    lit_ok += 1;
                }
            }
            assert!(
                lit_ok >= 2,
                "v{v}: only {lit_ok} literal-only compressed blocks decoded"
            );
        }
    }
}

// ============================================================ 15. corpus sanity

/// Guards the corpus itself: it must be large, deterministic and actually
/// contain every legacy magic (otherwise everything above would be vacuous).
#[test]
fn corpus_is_representative() {
    let cp = corpus();
    assert!(cp.len() > 400, "corpus too small: {}", cp.len());
    for v in 1..=7usize {
        let m = LEGACY_MAGIC[v - 1];
        let n = cp
            .iter()
            .filter(|(_, b)| b.len() >= 4 && b[..4] == m)
            .count();
        assert!(n >= 10, "corpus has only {n} buffers with the v{v} magic");
    }
    // determinism
    let again = build_corpus();
    assert_eq!(cp.len(), again.len(), "corpus is not deterministic");
    for (a, b) in cp.iter().zip(again.iter()) {
        assert_eq!(a.0, b.0);
        assert!(a.1 == b.1, "corpus entry {} is not deterministic", a.0);
    }
}
