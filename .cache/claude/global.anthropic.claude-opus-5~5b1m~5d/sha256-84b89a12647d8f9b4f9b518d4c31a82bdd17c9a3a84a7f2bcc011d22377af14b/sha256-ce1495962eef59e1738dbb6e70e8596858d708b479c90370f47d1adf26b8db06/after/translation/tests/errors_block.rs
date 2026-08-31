//! Error-path differential tests for `ERRORS.md` rows 56-159:
//! the whole "## lz4.c (block API)" section (rows 56-116) and the whole
//! "## lz4hc.c (HC block API)" section (rows 117-159).
//!
//! Every call goes through a `.so` export looked up with `libloading`, once for
//! the C reference and once for the Rust translation.  C and Rust always get
//! their *own* destination buffer, pre-filled with a `0xCD` sentinel plus a
//! guard tail, and both the returned integer and the whole buffer (guard tail
//! included) are compared, so a failing call that scribbles differently is
//! caught.
//!
//! Opaque state (`LZ4_stream_t`, `LZ4_streamHC_t`, `LZ4_streamDecode_t`) is
//! always created/initialised by one library *for itself* and released by that
//! same library.  A context is never handed across the boundary.
//!
//! Three distinct error conventions are checked exactly, never as "both
//! failed":
//!   * compression returns `0` on failure;
//!   * `LZ4_decompress_safe*` returns `-1` or the negative parse offset
//!     `(int)(-(ip-src))-1`;
//!   * `LZ4_resetStreamStateHC` inverts the convention (`1` = failure).
//!
//! ## Build note that shapes several rows
//!
//! `c_src/src/lz4.c:268-274` reads
//! ```c
//! #if defined(LZ4_DEBUG) && (LZ4_DEBUG>=1)
//! #  include <assert.h>
//! #else
//! #  ifndef assert
//! #    define assert(condition) ((void)0)
//! #  endif
//! #endif
//! ```
//! and `lz4hc.c` `#include`s `lz4.c`, so in this build (`LZ4_DEBUG` undefined)
//! **every `assert()` in `lz4.c`/`lz4hc.c` is a no-op**.  The "assertion failure
//! in debug" rows therefore cannot abort the process here.  They are still
//! treated as out-of-contract: where the condition leads to a real invalid
//! dereference (a `memmove` to `NULL`, a `(size_t)`-cast negative length, ...)
//! the test documents that and asserts the closest reachable *in-contract*
//! behaviour instead of provoking the crash.
#![allow(non_snake_case)]

mod common;
use common::*;
use std::collections::BTreeMap;
use std::os::raw::{c_char, c_int, c_void};

// ---------------------------------------------------------------------------
// Local FFI signature aliases
// ---------------------------------------------------------------------------

/// `LZ4_initStream(buffer, size)` / `LZ4_initStreamHC(buffer, size)`
type FnInitStream = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
/// `LZ4_compress_fast_extState[_fastReset](state, src, dst, srcSize, dstCap, accel)`
/// and `LZ4_compress_HC_extStateHC[_fastReset](state, src, dst, srcSize, dstCap, level)`
type FnExtState =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
/// `LZ4_compress_destSize_extState(state, src, dst, srcSizePtr, target, accel)`
/// and `LZ4_compress_HC_destSize(state, src, dst, srcSizePtr, target, level)`
type FnDestSizeExtState = unsafe extern "C" fn(
    *mut c_void,
    *const c_char,
    *mut c_char,
    *mut c_int,
    c_int,
    c_int,
) -> c_int;
/// `LZ4_resetStreamState(state, inputBuffer)` / `LZ4_resetStreamStateHC(...)`
type FnResetStreamState = unsafe extern "C" fn(*mut c_void, *mut c_char) -> c_int;
/// `LZ4_loadDict` / `LZ4_loadDictSlow` / `LZ4_loadDictHC`
type FnLoadDict = unsafe extern "C" fn(*mut c_void, *const c_char, c_int) -> c_int;
/// `LZ4_saveDict` / `LZ4_saveDictHC`
type FnSaveDict = unsafe extern "C" fn(*mut c_void, *mut c_char, c_int) -> c_int;
/// `LZ4_setStreamDecode(sd, dictionary, dictSize)`
type FnSetStreamDecode = unsafe extern "C" fn(*mut c_void, *const c_char, c_int) -> c_int;
/// `LZ4_resetStream_fast(ctx)` / `LZ4_resetStream(ctx)`
type FnStreamVoid = unsafe extern "C" fn(*mut c_void);
/// `LZ4_setCompressionLevel(ctx, level)` / `LZ4_resetStreamHC[_fast](ctx, level)`
type FnStreamInt = unsafe extern "C" fn(*mut c_void, c_int);
/// `LZ4_compress_fast_continue(stream, src, dst, srcSize, dstCap, accel)`
type FnFastContinue =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
/// `LZ4_compress_HC_continue(stream, src, dst, srcSize, dstCap)`
type FnHCContinue =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int) -> c_int;
/// `LZ4_compress_HC_continue_destSize(stream, src, dst, srcSizePtr, target)`
type FnHCContinueDestSize =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, *mut c_int, c_int) -> c_int;
/// `LZ4_decompress_safe_continue(sd, src, dst, cSize, dstCap)`
type FnDecSafeContinue =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int) -> c_int;
/// `LZ4_decompress_fast_continue(sd, src, dst, originalSize)`
type FnDecFastContinue =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int) -> c_int;
/// `LZ4_decompress_safe_usingDict(src, dst, cSize, dstCap, dictStart, dictSize)`
type FnDecUsingDict =
    unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int, *const c_char, c_int) -> c_int;
/// `LZ4_decompress_safe_partial_usingDict(src, dst, cSize, target, dstCap, dictStart, dictSize)`
type FnDecPartialUsingDict = unsafe extern "C" fn(
    *const c_char,
    *mut c_char,
    c_int,
    c_int,
    c_int,
    *const c_char,
    c_int,
) -> c_int;
/// `LZ4_uncompress(src, dst, outputSize)` (== `LZ4_decompress_fast`)
type FnUncompress = unsafe extern "C" fn(*const c_char, *mut c_char, c_int) -> c_int;
/// `LZ4_uncompress_unknownOutputSize(src, dst, isize, maxOutputSize)`
type FnUncompressUnknown =
    unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int) -> c_int;
/// `LZ4_compress_HC(src, dst, srcSize, dstCap, level)`
type FnHC = unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
/// `LZ4_createHC(inputBuffer)`
type FnCreateHC = unsafe extern "C" fn(*const c_char) -> *mut c_void;
/// `LZ4_attach_HC_dictionary(working, dict)` / `LZ4_attach_dictionary(working, dict)`
type FnAttach = unsafe extern "C" fn(*mut c_void, *const c_void);
/// `LZ4_compress_withState(state, src, dst, srcSize)` /
/// `LZ4_compressHC_withStateHC(state, src, dst, srcSize)`
type FnWithState4 =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int) -> c_int;
/// `LZ4_compress_limitedOutput_withState(state, src, dst, srcSize, dstSize)` /
/// `LZ4_compressHC_limitedOutput_withStateHC` / `LZ4_compressHC2_withStateHC`
type FnWithState5 =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int) -> c_int;
/// `LZ4_compressHC2_limitedOutput_withStateHC(state, src, dst, srcSize, dstSize, level)`
type FnWithState6 =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;

// ---------------------------------------------------------------------------
// Constants mirrored from lz4.c / lz4hc.c
// ---------------------------------------------------------------------------

const SENT: u8 = 0xCD;
/// Guard bytes appended past the advertised destination capacity.  The decoder
/// legitimately wildcopies up to 32 bytes beyond `cpy`, so this region *is*
/// written; it must be written *identically* by both implementations.
const GUARD: usize = 128;
const LZ4_ACCELERATION_MAX: c_int = 65537;
const FASTLOOP_SAFE_DISTANCE: usize = 64;
/// `LZ4_STREAM_MINSIZE == (1 << LZ4_MEMORY_USAGE) + 32` with the default
/// `LZ4_MEMORY_USAGE == 14`.
const LZ4_STREAM_MINSIZE: usize = (1usize << 14) + 32;
/// `LZ4_STREAMHC_MINSIZE`, lz4hc.h:252.
const LZ4_STREAMHC_MINSIZE: usize = 262200;
/// Every misalignment step worth probing for an 8-byte-aligned state type.
const MISALIGN: [usize; 7] = [1, 2, 3, 4, 5, 6, 7];
/// The full "one step past every documented bound and far beyond" ladder that
/// the task requires for `compressionLevel` and `acceleration`.
const WILD_LEVELS: [c_int; 16] = [
    c_int::MIN,
    -1000,
    -1,
    0,
    1,
    2,
    9,
    10,
    12,
    13,
    100,
    65536,
    65537,
    65538,
    1000000,
    c_int::MAX,
];
/// The extreme `srcSize` / `dstCapacity` ladder.
const WILD_SIZES: [c_int; 6] = [
    0,
    -1,
    c_int::MIN,
    LZ4_MAX_INPUT_SIZE as c_int,
    LZ4_MAX_INPUT_SIZE as c_int + 1,
    c_int::MAX,
];

// ---------------------------------------------------------------------------
// Small utilities
// ---------------------------------------------------------------------------

fn dstbuf(alloc: usize) -> Vec<u8> {
    vec![SENT; alloc + GUARD]
}

/// A copy of `b` with `extra` trailing zero bytes, so speculative over-reads
/// past the logical end of a (possibly corrupt) block stay inside memory we own.
fn padded(b: &[u8], extra: usize) -> Vec<u8> {
    let mut v = Vec::with_capacity(b.len() + extra);
    v.extend_from_slice(b);
    v.resize(b.len() + extra, 0);
    v
}

/// `common::gen`, but guaranteeing a real allocation even for length 0: an empty
/// `Vec`'s pointer is the dangling `0x1`, and the HC optimal parser computes
/// `iend - MFLIMIT`, which would then wrap.
fn gen_real(rng: &mut Rng, shape: Shape, len: usize) -> Vec<u8> {
    let mut v = common::gen(rng, shape, len);
    if v.capacity() == 0 {
        v.reserve(64);
    }
    v
}

/// 8-byte aligned, sentinel-filled scratch memory used for caller-provided
/// state buffers.  One of these is obtained *separately* per library.
struct Scratch {
    v: Vec<u64>,
}

impl Scratch {
    fn new(bytes: usize) -> Scratch {
        Scratch { v: vec![0xCDCD_CDCD_CDCD_CDCDu64; bytes / 8 + 16] }
    }
    fn ptr(&mut self) -> *mut c_void {
        self.v.as_mut_ptr() as *mut c_void
    }
    /// Pointer offset by `off` bytes (used to build deliberately misaligned state).
    fn at(&mut self, off: usize) -> *mut c_void {
        unsafe { (self.v.as_mut_ptr() as *mut u8).add(off) as *mut c_void }
    }
    fn bytes(&self, n: usize) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.v.as_ptr() as *const u8, n) }
    }
}

unsafe fn bound(l: &Pair, n: c_int) -> c_int {
    let (c, r) = l.sym::<FnCompressBound>("LZ4_compressBound");
    let (a, b) = (c(n), r(n));
    assert_eq!(a, b, "LZ4_compressBound({n}) mismatch (C={a} Rust={b})");
    a
}

unsafe fn sizeof_state(l: &Pair) -> usize {
    let (c, r) = l.sym::<FnVoidToInt>("LZ4_sizeofState");
    let (a, b) = (c(), r());
    assert_eq!(a, b, "LZ4_sizeofState mismatch (C={a} Rust={b})");
    a as usize
}

unsafe fn sizeof_state_hc(l: &Pair) -> usize {
    let (c, r) = l.sym::<FnVoidToInt>("LZ4_sizeofStateHC");
    let (a, b) = (c(), r());
    assert_eq!(a, b, "LZ4_sizeofStateHC mismatch (C={a} Rust={b})");
    a as usize
}

/// A per-library pair of opaque handles produced by `name` (a `LZ4_create*`).
struct Handles {
    c: *mut c_void,
    r: *mut c_void,
}

unsafe fn create_pair(l: &Pair, name: &str) -> Handles {
    let (fc, fr) = l.sym::<FnVoidToPtr>(name);
    let h = Handles { c: fc(), r: fr() };
    assert!(!h.c.is_null(), "{name}: C returned NULL");
    assert!(!h.r.is_null(), "{name}: Rust returned NULL");
    h
}

unsafe fn free_pair(l: &Pair, name: &str, h: Handles) {
    let (fc, fr) = l.sym::<FnFreePtr>(name);
    let (a, b) = (fc(h.c), fr(h.r));
    assert_eq!(a, b, "{name}: return mismatch (C={a} Rust={b})");
    assert_eq!(a, 0, "{name}: expected 0, got {a}");
}

// ---------------------------------------------------------------------------
// Differential drivers
// ---------------------------------------------------------------------------

/// `LZ4_compress_default` (`accel == None`) or `LZ4_compress_fast`.
/// `alloc` is the real allocation size; `cap` is the (possibly lying) argument.
unsafe fn diff_compress(
    l: &Pair,
    ctx: &str,
    src: *const c_char,
    ssz: c_int,
    alloc: usize,
    cap: c_int,
    accel: Option<c_int>,
) -> c_int {
    let mut cb = dstbuf(alloc);
    let mut rb = dstbuf(alloc);
    let (cr, rr) = match accel {
        None => {
            let (c, r) = l.sym::<FnCompressDefault>("LZ4_compress_default");
            (
                c(src, cb.as_mut_ptr() as *mut c_char, ssz, cap),
                r(src, rb.as_mut_ptr() as *mut c_char, ssz, cap),
            )
        }
        Some(a) => {
            let (c, r) = l.sym::<FnCompressFast>("LZ4_compress_fast");
            (
                c(src, cb.as_mut_ptr() as *mut c_char, ssz, cap, a),
                r(src, rb.as_mut_ptr() as *mut c_char, ssz, cap, a),
            )
        }
    };
    same_int_and_bytes(ctx, cr, rr, &cb, &rb);
    same_full_buffers(ctx, &cb, &rb);
    cr
}

/// `LZ4_compress_default` / `LZ4_compress_fast` on a slice, returning the bytes.
unsafe fn diff_compress_bytes(
    l: &Pair,
    ctx: &str,
    src: &[u8],
    cap: usize,
    accel: Option<c_int>,
) -> (c_int, Vec<u8>) {
    let mut cb = dstbuf(cap);
    let mut rb = dstbuf(cap);
    let (cr, rr) = match accel {
        None => {
            let (c, r) = l.sym::<FnCompressDefault>("LZ4_compress_default");
            (
                c(
                    src.as_ptr() as *const c_char,
                    cb.as_mut_ptr() as *mut c_char,
                    src.len() as c_int,
                    cap as c_int,
                ),
                r(
                    src.as_ptr() as *const c_char,
                    rb.as_mut_ptr() as *mut c_char,
                    src.len() as c_int,
                    cap as c_int,
                ),
            )
        }
        Some(a) => {
            let (c, r) = l.sym::<FnCompressFast>("LZ4_compress_fast");
            (
                c(
                    src.as_ptr() as *const c_char,
                    cb.as_mut_ptr() as *mut c_char,
                    src.len() as c_int,
                    cap as c_int,
                    a,
                ),
                r(
                    src.as_ptr() as *const c_char,
                    rb.as_mut_ptr() as *mut c_char,
                    src.len() as c_int,
                    cap as c_int,
                    a,
                ),
            )
        }
    };
    same_int_and_bytes(ctx, cr, rr, &cb, &rb);
    same_full_buffers(ctx, &cb, &rb);
    (cr, cb)
}

/// `LZ4_compress_HC` with independent sentinel buffers.
unsafe fn diff_hc(
    l: &Pair,
    ctx: &str,
    src: *const c_char,
    ssz: c_int,
    alloc: usize,
    cap: c_int,
    level: c_int,
) -> (c_int, Vec<u8>) {
    let (fc, fr) = l.sym::<FnHC>("LZ4_compress_HC");
    let mut cb = dstbuf(alloc);
    let mut rb = dstbuf(alloc);
    let a = fc(src, cb.as_mut_ptr() as *mut c_char, ssz, cap, level);
    let b = fr(src, rb.as_mut_ptr() as *mut c_char, ssz, cap, level);
    same_int_and_bytes(ctx, a, b, &cb, &rb);
    same_full_buffers(ctx, &cb, &rb);
    (a, cb)
}

/// `LZ4_decompress_safe`.  `comp` is padded with `pad` zero bytes first.
unsafe fn diff_dec_safe(
    l: &Pair,
    ctx: &str,
    comp: &[u8],
    csize: c_int,
    alloc: usize,
    cap: c_int,
    pad: usize,
) -> c_int {
    let p = padded(comp, pad);
    let mut cb = dstbuf(alloc);
    let mut rb = dstbuf(alloc);
    let (c, r) = l.sym::<FnDecompressSafe>("LZ4_decompress_safe");
    let a = c(p.as_ptr() as *const c_char, cb.as_mut_ptr() as *mut c_char, csize, cap);
    let b = r(p.as_ptr() as *const c_char, rb.as_mut_ptr() as *mut c_char, csize, cap);
    same_int_and_bytes(ctx, a, b, &cb, &rb);
    same_full_buffers(ctx, &cb, &rb);
    a
}

/// `LZ4_decompress_safe_partial`.
unsafe fn diff_dec_partial(
    l: &Pair,
    ctx: &str,
    comp: &[u8],
    csize: c_int,
    target: c_int,
    alloc: usize,
    cap: c_int,
    pad: usize,
) -> c_int {
    let p = padded(comp, pad);
    let mut cb = dstbuf(alloc);
    let mut rb = dstbuf(alloc);
    let (c, r) = l.sym::<FnDecompressSafePartial>("LZ4_decompress_safe_partial");
    let a = c(
        p.as_ptr() as *const c_char,
        cb.as_mut_ptr() as *mut c_char,
        csize,
        target,
        cap,
    );
    let b = r(
        p.as_ptr() as *const c_char,
        rb.as_mut_ptr() as *mut c_char,
        csize,
        target,
        cap,
    );
    same_int_and_bytes(ctx, a, b, &cb, &rb);
    same_full_buffers(ctx, &cb, &rb);
    a
}

/// `LZ4_decompress_fast` **and** the obsolete `LZ4_uncompress`, which lz4.c
/// implements by forwarding straight to it.  `LZ4_decompress_fast` has no input
/// bounds checking at all, so the input is padded generously with zero bytes:
/// every loop iteration produces >= 4 output bytes while consuming a bounded
/// number of input bytes, so `2*originalSize + 512` bytes of padding keep every
/// read inside memory we own.
unsafe fn diff_dec_fast(l: &Pair, ctx: &str, comp: &[u8], orig: c_int) -> c_int {
    assert!(orig >= 0, "{ctx}: negative originalSize is out of contract");
    let alloc = orig as usize;
    let p = padded(comp, 2 * alloc + 512);
    let (c, r) = l.sym::<FnDecompressFast>("LZ4_decompress_fast");
    let mut cb = dstbuf(alloc);
    let mut rb = dstbuf(alloc);
    let a = c(p.as_ptr() as *const c_char, cb.as_mut_ptr() as *mut c_char, orig);
    let b = r(p.as_ptr() as *const c_char, rb.as_mut_ptr() as *mut c_char, orig);
    same_int_and_bytes(ctx, a, b, &cb, &rb);
    same_full_buffers(ctx, &cb, &rb);

    let (uc, ur) = l.sym::<FnUncompress>("LZ4_uncompress");
    let mut cb2 = dstbuf(alloc);
    let mut rb2 = dstbuf(alloc);
    let a2 = uc(p.as_ptr() as *const c_char, cb2.as_mut_ptr() as *mut c_char, orig);
    let b2 = ur(p.as_ptr() as *const c_char, rb2.as_mut_ptr() as *mut c_char, orig);
    same_int_and_bytes(&format!("{ctx} [LZ4_uncompress]"), a2, b2, &cb2, &rb2);
    same_full_buffers(&format!("{ctx} [LZ4_uncompress]"), &cb2, &rb2);
    assert_eq!(
        a, a2,
        "{ctx}: LZ4_uncompress ({a2}) must equal LZ4_decompress_fast ({a})"
    );
    same_full_buffers(&format!("{ctx} [uncompress vs fast]"), &cb, &cb2);
    a
}

/// The obsolete `LZ4_uncompress_unknownOutputSize`, which forwards to
/// `LZ4_decompress_safe`.
unsafe fn diff_uncompress_unknown(
    l: &Pair,
    ctx: &str,
    comp: &[u8],
    csize: c_int,
    alloc: usize,
    cap: c_int,
    pad: usize,
) -> c_int {
    let p = padded(comp, pad);
    let mut cb = dstbuf(alloc);
    let mut rb = dstbuf(alloc);
    let (c, r) = l.sym::<FnUncompressUnknown>("LZ4_uncompress_unknownOutputSize");
    let a = c(p.as_ptr() as *const c_char, cb.as_mut_ptr() as *mut c_char, csize, cap);
    let b = r(p.as_ptr() as *const c_char, rb.as_mut_ptr() as *mut c_char, csize, cap);
    same_int_and_bytes(ctx, a, b, &cb, &rb);
    same_full_buffers(ctx, &cb, &rb);
    a
}

// ---------------------------------------------------------------------------
// Hand-rolled block builder (exact control over tokens / extensions / offsets)
// ---------------------------------------------------------------------------

fn push_ext(v: &mut Vec<u8>, mut rem: usize) {
    while rem >= 255 {
        v.push(255);
        rem -= 255;
    }
    v.push(rem as u8);
}

struct Blk {
    c: Vec<u8>,
    p: Vec<u8>,
}

impl Blk {
    fn new() -> Blk {
        Blk { c: Vec::new(), p: Vec::new() }
    }
    /// One (literals, match) sequence; `matchlen` is the total match length.
    fn seq(&mut self, lits: &[u8], offset: usize, matchlen: usize) {
        assert!(matchlen >= 4);
        assert!((1..=65535).contains(&offset));
        let ll = lits.len();
        let ml = matchlen - 4;
        self.c.push(((ll.min(15) as u8) << 4) | (ml.min(15) as u8));
        if ll >= 15 {
            push_ext(&mut self.c, ll - 15);
        }
        self.c.extend_from_slice(lits);
        self.p.extend_from_slice(lits);
        self.c.push((offset & 0xFF) as u8);
        self.c.push(((offset >> 8) & 0xFF) as u8);
        if ml >= 15 {
            push_ext(&mut self.c, ml - 15);
        }
        assert!(offset <= self.p.len());
        for _ in 0..matchlen {
            let b = self.p[self.p.len() - offset];
            self.p.push(b);
        }
    }
    /// Terminating literals-only sequence -> `(compressed, plaintext)`.
    fn finish(mut self, lits: &[u8]) -> (Vec<u8>, Vec<u8>) {
        let ll = lits.len();
        self.c.push((ll.min(15) as u8) << 4);
        if ll >= 15 {
            push_ext(&mut self.c, ll - 15);
        }
        self.c.extend_from_slice(lits);
        self.p.extend_from_slice(lits);
        (self.c, self.p)
    }
}

fn rand_lits(rng: &mut Rng, n: usize) -> Vec<u8> {
    (0..n).map(|_| rng.byte()).collect()
}

/// A random but well-formed block (trailing literal run >= MFLIMIT).
fn well_formed(rng: &mut Rng, nseq: usize) -> (Vec<u8>, Vec<u8>) {
    let mut b = Blk::new();
    for _ in 0..nseq {
        let n = rng.range(0, 30);
        let lits = rand_lits(rng, n);
        let avail = b.p.len() + n;
        if avail == 0 {
            continue;
        }
        let mut off = [1usize, 2, 3, 4, 5, 8, 13, 16, 40, 300, 65535][rng.below(11)];
        off = off.min(avail).min(65535).max(1);
        let m = rng.range(4, 300);
        b.seq(&lits, off, m);
    }
    let t = rng.range(12, 60);
    let lits = rand_lits(rng, t);
    b.finish(&lits)
}

// ===========================================================================
// Row 56 - LZ4_compressBound / LZ4_COMPRESSBOUND: (unsigned)isize > LZ4_MAX_INPUT_SIZE -> 0
// ===========================================================================

#[test]
fn err_056_compress_bound_over_max_input_size() {
    let l = libs();
    let mut rng = Rng::new(56);
    unsafe {
        // Exactly at / around the boundary.
        let max = LZ4_MAX_INPUT_SIZE as c_int;
        assert_eq!(
            bound(l, max),
            max + max / 255 + 16,
            "LZ4_compressBound(LZ4_MAX_INPUT_SIZE) must still be valid"
        );
        for &n in &[max + 1, max + 2, c_int::MAX, -1, -2, c_int::MIN, -max] {
            assert_eq!(bound(l, n), 0, "LZ4_compressBound({n}) must be 0");
        }
        // Property sweep: every negative value and everything above the cap is 0,
        // everything in range matches the macro exactly.
        for _ in 0..20_000 {
            let n = rng.next_u32() as c_int;
            let got = bound(l, n);
            let want = if (n as u32) > LZ4_MAX_INPUT_SIZE as u32 {
                0
            } else {
                n + n / 255 + 16
            };
            assert_eq!(got, want, "LZ4_compressBound({n}) = {got}, want {want}");
        }
    }
}

// ===========================================================================
// Row 57 - LZ4_compress_generic: (U32)srcSize > (U32)LZ4_MAX_INPUT_SIZE -> 0
// ===========================================================================

#[test]
fn err_057_compress_srcsize_too_large_or_negative() {
    let l = libs();
    let mut rng = Rng::new(57);
    let src = gen_real(&mut rng, Shape::TextLike, 4096);
    unsafe {
        // lz4.c:1360 is the very first statement of LZ4_compress_generic and
        // `src` is not touched before it, so a *lying* oversized/negative
        // srcSize is safe here (verified by reading lz4.c:1358-1362).
        // LZ4_MAX_INPUT_SIZE *itself* is NOT rejected, so it is not used as a
        // lie: it would make the C read 2 GB from a 4 KB buffer.
        for &ssz in &[
            -1,
            -2,
            -4096,
            c_int::MIN,
            LZ4_MAX_INPUT_SIZE as c_int + 1,
            LZ4_MAX_INPUT_SIZE as c_int + 2,
            c_int::MAX,
        ] {
            for &cap in &[0i32, 1, 16, 4096, 8192] {
                let ctx = format!("row57 default ssz={ssz} cap={cap}");
                let r = diff_compress(
                    l,
                    &ctx,
                    src.as_ptr() as *const c_char,
                    ssz,
                    8192,
                    cap,
                    None,
                );
                assert_eq!(r, 0, "{ctx}: expected 0, got {r}");
                let ctx = format!("row57 fast ssz={ssz} cap={cap}");
                let r = diff_compress(
                    l,
                    &ctx,
                    src.as_ptr() as *const c_char,
                    ssz,
                    8192,
                    cap,
                    Some(1),
                );
                assert_eq!(r, 0, "{ctx}: expected 0, got {r}");
            }
        }
        // Randomised sweep over the whole negative half-plane and over the
        // range just above the cap.
        for _ in 0..3000 {
            let ssz = if rng.next_u64() & 1 == 0 {
                -(rng.range(1, 1 << 30) as c_int)
            } else {
                LZ4_MAX_INPUT_SIZE as c_int + rng.range(1, 1 << 24) as c_int
            };
            let cap = rng.range(0, 8192) as c_int;
            let ctx = format!("row57 rand ssz={ssz} cap={cap}");
            let r =
                diff_compress(l, &ctx, src.as_ptr() as *const c_char, ssz, 8192, cap, None);
            assert_eq!(r, 0, "{ctx}: expected 0, got {r}");
        }
    }
}

// ===========================================================================
// Row 58 - srcSize == 0 with outputDirective != notLimited and dstCapacity <= 0 -> 0
// ===========================================================================

#[test]
fn err_058_compress_empty_input_no_room_for_empty_block() {
    let l = libs();
    let mut rng = Rng::new(58);
    let scratch = gen_real(&mut rng, Shape::Degenerate, 64);
    unsafe {
        // LZ4_compress_fast_extState always takes the limitedOutput path when
        // dstCapacity < LZ4_compressBound(0) == 16, so dstCapacity <= 0 hits
        // lz4.c:1362 and returns 0; dstCapacity >= 1 writes the 1-byte empty
        // block and returns 1.
        for &cap in &[0i32, -1, -2, -16, c_int::MIN] {
            for accel in [None, Some(1), Some(65537)] {
                let ctx = format!("row58 cap={cap} accel={accel:?}");
                let r = diff_compress(
                    l,
                    &ctx,
                    scratch.as_ptr() as *const c_char,
                    0,
                    64,
                    cap,
                    accel,
                );
                assert_eq!(r, 0, "{ctx}: expected 0, got {r}");
            }
        }
        for &cap in &[1i32, 2, 15, 16, 17, 1000] {
            let ctx = format!("row58 ok cap={cap}");
            let r =
                diff_compress(l, &ctx, scratch.as_ptr() as *const c_char, 0, 1024, cap, None);
            assert_eq!(r, 1, "{ctx}: expected the 1-byte empty block, got {r}");
        }
    }
}

// ===========================================================================
// Row 59 - LZ4_compress_generic_validated: fillOutput with maxOutputSize < 1 -> 0
// ===========================================================================

#[test]
fn err_059_destsize_fill_output_target_below_one() {
    let l = libs();
    let mut rng = Rng::new(59);
    unsafe {
        let (c, r) = l.sym::<FnCompressDestSize>("LZ4_compress_destSize");
        for &n in &[1usize, 12, 13, 64, 1000, 70_000] {
            let src = gen_real(&mut rng, Shape::TextLike, n);
            for &target in &[0i32, -1, -1000, c_int::MIN] {
                let mut cb = dstbuf(64);
                let mut rb = dstbuf(64);
                let mut cn = n as c_int;
                let mut rn = n as c_int;
                let a = c(
                    src.as_ptr() as *const c_char,
                    cb.as_mut_ptr() as *mut c_char,
                    &mut cn,
                    target,
                );
                let b = r(
                    src.as_ptr() as *const c_char,
                    rb.as_mut_ptr() as *mut c_char,
                    &mut rn,
                    target,
                );
                let ctx = format!("row59 n={n} target={target}");
                same_int_and_bytes(&ctx, a, b, &cb, &rb);
                same_full_buffers(&ctx, &cb, &rb);
                assert_eq!(cn, rn, "{ctx}: *srcSizePtr mismatch (C={cn} Rust={rn})");
                assert_eq!(a, 0, "{ctx}: expected 0, got {a}");
            }
        }
        // ... and the same through LZ4_compress_destSize_extState, where the
        // acceleration axis is also visible.
        let (ec, er) = l.sym::<FnDestSizeExtState>("LZ4_compress_destSize_extState");
        let ss = sizeof_state(l);
        let mut cs = Scratch::new(ss);
        let mut rs = Scratch::new(ss);
        for &n in &[1usize, 13, 4096] {
            let src = gen_real(&mut rng, Shape::Compressible, n);
            for &target in &[0i32, -1, c_int::MIN] {
                for &accel in &[1i32, 0, -5, 65538] {
                    let mut cb = dstbuf(64);
                    let mut rb = dstbuf(64);
                    let mut cn = n as c_int;
                    let mut rn = n as c_int;
                    let a = ec(
                        cs.ptr(),
                        src.as_ptr() as *const c_char,
                        cb.as_mut_ptr() as *mut c_char,
                        &mut cn,
                        target,
                        accel,
                    );
                    let b = er(
                        rs.ptr(),
                        src.as_ptr() as *const c_char,
                        rb.as_mut_ptr() as *mut c_char,
                        &mut rn,
                        target,
                        accel,
                    );
                    let ctx = format!("row59 extState n={n} target={target} accel={accel}");
                    same_int_and_bytes(&ctx, a, b, &cb, &rb);
                    same_full_buffers(&ctx, &cb, &rb);
                    same_full_buffers(
                        &format!("{ctx} [state image]"),
                        cs.bytes(ss),
                        rs.bytes(ss),
                    );
                    assert_eq!(cn, rn, "{ctx}: *srcSizePtr mismatch");
                    assert_eq!(a, 0, "{ctx}: expected 0, got {a}");
                }
            }
        }
    }
}

// ===========================================================================
// Rows 60, 61, 62 - limitedOutput: literal run / match length / final literal
// run do not fit in `dst`  ->  0 (hash table left valid)
// ===========================================================================

/// Sweep every destination capacity from 0 up to (and past) `LZ4_compressBound`
/// for many shapes and sizes.  Below the bound the three `return 0` sites at
/// lz4.c:1113, :1208 and :1305 are the only possible outcomes besides success,
/// so C and Rust must agree on the exact cut-over capacity for every input.
fn limited_output_sweep(seed: u64, sizes: &[usize], accel: Option<c_int>, tag: &str) {
    let l = libs();
    let mut rng = Rng::new(seed);
    unsafe {
        for &n in sizes {
            for sh in ALL_SHAPES {
                let src = gen_real(&mut rng, sh, n);
                let b = bound(l, n as c_int) as usize;
                // Full sweep for small inputs, sampled sweep for large ones.
                let caps: Vec<usize> = if b <= 400 {
                    (0..=b + 2).collect()
                } else {
                    let mut v: Vec<usize> = (0..40).collect();
                    v.extend((0..60).map(|_| rng.below(b + 3)));
                    v.extend((b.saturating_sub(40))..=(b + 2));
                    v
                };
                let mut first_ok = usize::MAX;
                for cap in caps {
                    let ctx = format!("{tag} n={n} shape={sh:?} cap={cap}");
                    let r = diff_compress_bytes(l, &ctx, &src, cap, accel).0;
                    assert!(
                        r == 0 || (r > 0 && r as usize <= cap),
                        "{ctx}: nonsensical return {r}"
                    );
                    if r > 0 && cap < first_ok {
                        first_ok = cap;
                    }
                    if cap >= b {
                        assert!(
                            r > 0,
                            "{ctx}: capacity >= LZ4_compressBound({n}) must always succeed"
                        );
                    }
                }
                assert!(
                    first_ok != usize::MAX,
                    "{tag} n={n} shape={sh:?}: never succeeded, sweep is vacuous"
                );
            }
        }
    }
}

#[test]
fn err_060_limited_output_literal_run_does_not_fit() {
    // Incompressible inputs spend their whole budget on literal runs, so the
    // failing site is lz4.c:1113-1116 (`op + litLength + 8 + litLength/255 > olimit`).
    limited_output_sweep(60, &[13, 20, 64, 200, 1000], None, "row60");
}

#[test]
fn err_061_limited_output_match_length_does_not_fit() {
    // Long-match inputs (periodic / compressible) reach lz4.c:1208-1210.
    limited_output_sweep(61, &[300, 4096, 65546, 65547, 65548], Some(1), "row61");
}

#[test]
fn err_062_limited_output_final_literals_do_not_fit() {
    // Inputs below LZ4_minLength == 13 jump straight to `_last_literals`, so
    // the only reachable failure is lz4.c:1305-1314.
    limited_output_sweep(62, &[1, 2, 3, 4, 5, 6, 7, 8, 12], None, "row62");
    // Explicitly: for srcSize < 13 the block is `1 + srcSize` bytes (plus the
    // 255-extension bytes) and any smaller capacity must return exactly 0.
    let l = libs();
    let mut rng = Rng::new(620);
    unsafe {
        for n in 1..13usize {
            let src = gen_real(&mut rng, Shape::Incompressible, n);
            for cap in 0..=(n + 2) {
                let ctx = format!("row62 exact n={n} cap={cap}");
                let r = diff_compress_bytes(l, &ctx, &src, cap, None).0;
                let want = if cap >= n + 1 { (n + 1) as c_int } else { 0 };
                assert_eq!(r, want, "{ctx}: expected {want}, got {r}");
            }
        }
    }
}

// ===========================================================================
// Rows 63, 64 - LZ4_HEAPMODE ALLOC failure in LZ4_compress_fast /
// LZ4_compress_destSize.  UNREACHABLE in this build.
// ===========================================================================

#[test]
fn err_063_064_heapmode_alloc_failure_unreachable() {
    // c_src/CMakeLists.txt compiles with `LZ4_HEAPMODE=0`, so lz4.c:1457-1458
    // and lz4.c:1509-1510 are `#if (LZ4_HEAPMODE)`-excluded: the state lives on
    // the stack and there is no allocation that can fail.  There is no way to
    // provoke these rows from outside the library (a malloc interposer would be
    // required), so the test pins the *success* side of both branches instead:
    // neither entry point may ever return 0 when the destination is large
    // enough, i.e. the `if (ctxPtr == NULL) return 0` sentinel is never taken.
    let l = libs();
    let mut rng = Rng::new(63);
    unsafe {
        let (dc, dr) = l.sym::<FnCompressDestSize>("LZ4_compress_destSize");
        for &n in &[0usize, 1, 13, 100, 4096, 65547, 200_000] {
            for sh in ALL_SHAPES {
                let src = gen_real(&mut rng, sh, n);
                let b = bound(l, n as c_int) as usize;
                let ctx = format!("row63 n={n} shape={sh:?}");
                let r = diff_compress_bytes(l, &ctx, &src, b, Some(1)).0;
                assert!(r > 0, "{ctx}: LZ4_compress_fast returned {r} with full bound");

                let mut cb = dstbuf(b.max(1));
                let mut rb = dstbuf(b.max(1));
                let mut cn = n as c_int;
                let mut rn = n as c_int;
                let a = dc(
                    src.as_ptr() as *const c_char,
                    cb.as_mut_ptr() as *mut c_char,
                    &mut cn,
                    b.max(1) as c_int,
                );
                let bb = dr(
                    src.as_ptr() as *const c_char,
                    rb.as_mut_ptr() as *mut c_char,
                    &mut rn,
                    b.max(1) as c_int,
                );
                let ctx = format!("row64 n={n} shape={sh:?}");
                same_int_and_bytes(&ctx, a, bb, &cb, &rb);
                same_full_buffers(&ctx, &cb, &rb);
                assert_eq!(cn, rn, "{ctx}: *srcSizePtr mismatch");
                assert!(a > 0, "{ctx}: LZ4_compress_destSize returned {a}");
            }
        }
    }
}

// ===========================================================================
// Row 65 - LZ4_compress_fast_extState[_fastReset]: acceleration clamped to
// [LZ4_ACCELERATION_DEFAULT .. LZ4_ACCELERATION_MAX], never rejected
// ===========================================================================

#[test]
fn err_065_acceleration_clamped_not_rejected() {
    let l = libs();
    let mut rng = Rng::new(65);
    unsafe {
        let ss = sizeof_state(l);
        let (ic, ir) = l.sym::<FnInitStream>("LZ4_initStream");
        for name in ["LZ4_compress_fast_extState", "LZ4_compress_fast_extState_fastReset"] {
            let (fc, fr) = l.sym::<FnExtState>(name);
            for &n in &[13usize, 200, 4096, 65547] {
                for sh in [Shape::TextLike, Shape::Compressible, Shape::Incompressible] {
                    let src = gen_real(&mut rng, sh, n);
                    let cap = bound(l, n as c_int) as usize;
                    // Reference outputs at the two clamp targets.
                    let mut refs: Vec<(c_int, Vec<u8>)> = Vec::new();
                    for &a in &[1i32, LZ4_ACCELERATION_MAX] {
                        let mut cb = dstbuf(cap);
                        let mut rb = dstbuf(cap);
                        let mut csb = Scratch::new(ss);
                        let mut rsb = Scratch::new(ss);
                        assert!(!ic(csb.ptr(), ss).is_null());
                        assert!(!ir(rsb.ptr(), ss).is_null());
                        let x = fc(
                            csb.ptr(),
                            src.as_ptr() as *const c_char,
                            cb.as_mut_ptr() as *mut c_char,
                            n as c_int,
                            cap as c_int,
                            a,
                        );
                        let y = fr(
                            rsb.ptr(),
                            src.as_ptr() as *const c_char,
                            rb.as_mut_ptr() as *mut c_char,
                            n as c_int,
                            cap as c_int,
                            a,
                        );
                        let ctx = format!("row65 {name} ref accel={a} n={n} {sh:?}");
                        same_int_and_bytes(&ctx, x, y, &cb, &rb);
                        same_full_buffers(&ctx, &cb, &rb);
                        refs.push((x, cb));
                    }
                    for &a in &[
                        c_int::MIN,
                        -1000,
                        -1,
                        0,
                        1,
                        2,
                        65536,
                        65537,
                        65538,
                        1000000,
                        c_int::MAX,
                    ] {
                        let mut cb = dstbuf(cap);
                        let mut rb = dstbuf(cap);
                        let mut csb = Scratch::new(ss);
                        let mut rsb = Scratch::new(ss);
                        assert!(!ic(csb.ptr(), ss).is_null());
                        assert!(!ir(rsb.ptr(), ss).is_null());
                        let x = fc(
                            csb.ptr(),
                            src.as_ptr() as *const c_char,
                            cb.as_mut_ptr() as *mut c_char,
                            n as c_int,
                            cap as c_int,
                            a,
                        );
                        let y = fr(
                            rsb.ptr(),
                            src.as_ptr() as *const c_char,
                            rb.as_mut_ptr() as *mut c_char,
                            n as c_int,
                            cap as c_int,
                            a,
                        );
                        let ctx = format!("row65 {name} accel={a} n={n} {sh:?}");
                        same_int_and_bytes(&ctx, x, y, &cb, &rb);
                        same_full_buffers(&ctx, &cb, &rb);
                        assert!(x > 0, "{ctx}: acceleration must never be rejected, got {x}");
                        // Documented clamping equivalences.
                        if a < 1 {
                            assert_eq!(x, refs[0].0, "{ctx}: accel < 1 must behave as accel 1");
                            same_full_buffers(&format!("{ctx} == accel 1"), &refs[0].1, &cb);
                        }
                        if a > LZ4_ACCELERATION_MAX {
                            assert_eq!(
                                x, refs[1].0,
                                "{ctx}: accel > LZ4_ACCELERATION_MAX must behave as the max"
                            );
                            same_full_buffers(
                                &format!("{ctx} == accel LZ4_ACCELERATION_MAX"),
                                &refs[1].1,
                                &cb,
                            );
                        }
                    }
                }
            }
        }
        // LZ4_compress_fast itself forwards the acceleration unchanged.
        for &n in &[13usize, 300, 4096] {
            let src = gen_real(&mut rng, Shape::TextLike, n);
            let cap = bound(l, n as c_int) as usize;
            let r1 = diff_compress_bytes(l, "row65 fast accel=1", &src, cap, Some(1));
            let rmax = diff_compress_bytes(
                l,
                "row65 fast accel=max",
                &src,
                cap,
                Some(LZ4_ACCELERATION_MAX),
            );
            for &a in &[c_int::MIN, -1, 0] {
                let g = diff_compress_bytes(l, &format!("row65 fast accel={a}"), &src, cap, Some(a));
                assert_eq!(g.0, r1.0, "row65 fast accel={a} != accel 1");
                same_full_buffers(&format!("row65 fast accel={a} == 1"), &r1.1, &g.1);
            }
            for &a in &[65538, 1000000, c_int::MAX] {
                let g = diff_compress_bytes(l, &format!("row65 fast accel={a}"), &src, cap, Some(a));
                assert_eq!(g.0, rmax.0, "row65 fast accel={a} != accel max");
                same_full_buffers(&format!("row65 fast accel={a} == max"), &rmax.1, &g.1);
            }
        }
    }
}

// ===========================================================================
// Row 66 - LZ4_createStream: ALLOC failure -> NULL.  UNREACHABLE.
// ===========================================================================

#[test]
fn err_066_create_stream_alloc_failure_unreachable() {
    // lz4.c:1536 `if (lz4s == NULL) return NULL;` can only be reached when
    // malloc fails; that cannot be provoked through the public ABI.  Pin the
    // success side: both libraries hand back a non-NULL, freshly initialised
    // stream, and LZ4_freeStream accepts it with the documented `0`.
    let l = libs();
    unsafe {
        for _ in 0..200 {
            let h = create_pair(l, "LZ4_createStream");
            free_pair(l, "LZ4_freeStream", h);
        }
    }
}

// ===========================================================================
// Rows 67, 68, 69 - LZ4_initStream: NULL buffer / size too small / misaligned
// ===========================================================================

#[test]
fn err_067_068_069_init_stream_null_undersized_misaligned() {
    let l = libs();
    unsafe {
        let ss = sizeof_state(l);
        assert_eq!(
            ss, LZ4_STREAM_MINSIZE,
            "LZ4_sizeofState() != LZ4_STREAM_MINSIZE ({ss} vs {LZ4_STREAM_MINSIZE})"
        );
        let (ic, ir) = l.sym::<FnInitStream>("LZ4_initStream");

        // Row 67: buffer == NULL (lz4.c:1555) -> NULL, for *every* size.
        for &size in &[0usize, 1, 32, ss - 1, ss, ss + 1, usize::MAX] {
            let a = ic(std::ptr::null_mut(), size);
            let b = ir(std::ptr::null_mut(), size);
            assert!(a.is_null(), "row67 C LZ4_initStream(NULL, {size}) = {a:?}");
            assert!(b.is_null(), "row67 Rust LZ4_initStream(NULL, {size}) = {b:?}");
        }

        // Row 68: size < sizeof(LZ4_stream_t) (lz4.c:1556) -> NULL.  The
        // *allocation* stays full size; only the declared `size` lies, so the
        // rejection happens before any write.
        let mut cs = Scratch::new(ss);
        let mut rs = Scratch::new(ss);
        for &size in &[0usize, 1, 2, 7, 8, 32, 1024, ss / 2, ss - 2, ss - 1] {
            let a = ic(cs.ptr(), size);
            let b = ir(rs.ptr(), size);
            assert!(a.is_null(), "row68 C LZ4_initStream(buf, {size}) = {a:?}");
            assert!(b.is_null(), "row68 Rust LZ4_initStream(buf, {size}) = {b:?}");
        }
        // Untouched: the rejection must not have zeroed anything.
        for (i, &x) in cs.bytes(ss).iter().enumerate() {
            assert_eq!(x, SENT, "row68: C scribbled the rejected buffer at {i}");
        }
        for (i, &x) in rs.bytes(ss).iter().enumerate() {
            assert_eq!(x, SENT, "row68: Rust scribbled the rejected buffer at {i}");
        }
        // Exactly sizeof(LZ4_stream_t) and above must succeed.
        for &size in &[ss, ss + 1, ss * 2] {
            let a = ic(cs.ptr(), size);
            let b = ir(rs.ptr(), size);
            assert_eq!(a, cs.ptr(), "row68 C LZ4_initStream(buf, {size}) must succeed");
            assert_eq!(b, rs.ptr(), "row68 Rust LZ4_initStream(buf, {size}) must succeed");
            same_full_buffers(
                &format!("row68 initialised state size={size}"),
                cs.bytes(ss),
                rs.bytes(ss),
            );
        }

        // Row 69: buffer not aligned to LZ4_stream_t_alignment() (lz4.c:1557,
        // active because LZ4_ALIGN_TEST defaults to 1) -> NULL.
        let mut cs2 = Scratch::new(ss + 16);
        let mut rs2 = Scratch::new(ss + 16);
        for &off in &MISALIGN {
            let a = ic(cs2.at(off), ss);
            let b = ir(rs2.at(off), ss);
            assert!(a.is_null(), "row69 C LZ4_initStream(buf+{off}) = {a:?}");
            assert!(b.is_null(), "row69 Rust LZ4_initStream(buf+{off}) = {b:?}");
            // ... and an undersized *and* misaligned buffer is still NULL.
            let a = ic(cs2.at(off), ss - 1);
            let b = ir(rs2.at(off), ss - 1);
            assert!(a.is_null(), "row68+69 C combined off={off}");
            assert!(b.is_null(), "row68+69 Rust combined off={off}");
        }
        // 8-byte multiples are accepted.
        for &off in &[0usize, 8, 16] {
            let a = ic(cs2.at(off), ss);
            let b = ir(rs2.at(off), ss);
            assert_eq!(a, cs2.at(off), "row69 C aligned off={off} must succeed");
            assert_eq!(b, rs2.at(off), "row69 Rust aligned off={off} must succeed");
        }
    }
}

// ===========================================================================
// Row 70 - LZ4_freeStream(NULL) -> 0 (free-on-NULL tolerated)
// ===========================================================================

#[test]
fn err_070_free_stream_null() {
    let l = libs();
    unsafe {
        let (fc, fr) = l.sym::<FnFreePtr>("LZ4_freeStream");
        for _ in 0..100 {
            let a = fc(std::ptr::null_mut());
            let b = fr(std::ptr::null_mut());
            assert_eq!(a, b, "row70: return mismatch (C={a} Rust={b})");
            assert_eq!(a, 0, "row70: LZ4_freeStream(NULL) must be 0, got {a}");
        }
    }
}

// ===========================================================================
// Rows 71, 72 - LZ4_loadDict / LZ4_loadDictSlow: dictSize < HASH_UNIT -> 0,
// dictSize > 64 KB -> silently truncated to the last 64 KB
// ===========================================================================

#[test]
fn err_071_072_load_dict_below_hash_unit_and_oversized() {
    let l = libs();
    let mut rng = Rng::new(71);
    unsafe {
        let dict = gen_real(&mut rng, Shape::TextLike, 200_000);
        for name in ["LZ4_loadDict", "LZ4_loadDictSlow"] {
            let (fc, fr) = l.sym::<FnLoadDict>(name);
            // Row 71: dictSize < HASH_UNIT (8 on 64-bit) -> 0.  Negative sizes
            // are also caught by the same signed comparison at lz4.c:1613 and
            // are safe: nothing is dereferenced before the early return.
            for &ds in &[-1000i32, -1, 0, 1, 2, 3, 4, 5, 6, 7] {
                let sc = create_pair(l, "LZ4_createStream");
                let a = fc(sc.c, dict.as_ptr() as *const c_char, ds);
                let b = fr(sc.r, dict.as_ptr() as *const c_char, ds);
                assert_eq!(a, b, "row71 {name} ds={ds}: mismatch (C={a} Rust={b})");
                assert_eq!(a, 0, "row71 {name} ds={ds}: expected 0, got {a}");
                free_pair(l, "LZ4_freeStream", sc);
            }
            // dictSize == HASH_UNIT is the first accepted value.
            for &ds in &[8i32, 9, 100, 65535, 65536] {
                let sc = create_pair(l, "LZ4_createStream");
                let a = fc(sc.c, dict.as_ptr() as *const c_char, ds);
                let b = fr(sc.r, dict.as_ptr() as *const c_char, ds);
                assert_eq!(a, b, "row71 {name} ds={ds}: mismatch");
                assert_eq!(a, ds, "row71 {name} ds={ds}: expected {ds}, got {a}");
                free_pair(l, "LZ4_freeStream", sc);
            }
            // Row 72: dictSize > 64 KB -> only the last 64 KB kept, and the
            // *returned* size is the truncated one.
            for &ds in &[65537i32, 70_000, 131_072, 200_000] {
                let sc = create_pair(l, "LZ4_createStream");
                let a = fc(sc.c, dict.as_ptr() as *const c_char, ds);
                let b = fr(sc.r, dict.as_ptr() as *const c_char, ds);
                assert_eq!(a, b, "row72 {name} ds={ds}: mismatch (C={a} Rust={b})");
                assert_eq!(a, 65536, "row72 {name} ds={ds}: expected 65536, got {a}");
                free_pair(l, "LZ4_freeStream", sc);
            }
        }
        // The truncation really is "last 64 KB": loading the tail directly must
        // produce byte-identical compression of a following block.
        let (fc, fr) = l.sym::<FnLoadDict>("LZ4_loadDict");
        let (cc, cr) = l.sym::<FnFastContinue>("LZ4_compress_fast_continue");
        let blk = gen_real(&mut rng, Shape::TextLike, 8000);
        let cap = bound(l, 8000) as usize;
        let mut out: Vec<Vec<u8>> = Vec::new();
        for &(ptr_off, ds) in &[(0usize, 200_000i32), (200_000 - 65536, 65536)] {
            let s = create_pair(l, "LZ4_createStream");
            let p = dict.as_ptr().add(ptr_off) as *const c_char;
            assert_eq!(fc(s.c, p, ds), fr(s.r, p, ds), "row72 tail load mismatch");
            let mut cb = dstbuf(cap);
            let mut rb = dstbuf(cap);
            let a = cc(
                s.c,
                blk.as_ptr() as *const c_char,
                cb.as_mut_ptr() as *mut c_char,
                8000,
                cap as c_int,
                1,
            );
            let b = cr(
                s.r,
                blk.as_ptr() as *const c_char,
                rb.as_mut_ptr() as *mut c_char,
                8000,
                cap as c_int,
                1,
            );
            let ctx = format!("row72 continue after load off={ptr_off} ds={ds}");
            same_int_and_bytes(&ctx, a, b, &cb, &rb);
            same_full_buffers(&ctx, &cb, &rb);
            assert!(a > 0, "{ctx}: expected success");
            out.push(cb[..a as usize].to_vec());
            free_pair(l, "LZ4_freeStream", s);
        }
        assert_eq!(
            out[0], out[1],
            "row72: loading 200000 bytes must be equivalent to loading the last 64 KB"
        );
    }
}

// ===========================================================================
// Rows 73, 74, 75 - LZ4_compress_fast_continue: tiny dictionary discarded,
// src overlapping the recorded dictionary, LZ4_renormDictT index rescale
// ===========================================================================

#[test]
fn err_073_074_075_continue_dictionary_sanitization() {
    let l = libs();
    let mut rng = Rng::new(73);
    unsafe {
        let (ldc, ldr) = l.sym::<FnLoadDict>("LZ4_loadDict");
        let (cc, cr) = l.sym::<FnFastContinue>("LZ4_compress_fast_continue");
        let (rfc, rfr) = l.sym::<FnStreamVoid>("LZ4_resetStream_fast");

        // ---- Row 73: dictSize < 4, not prefix mode, inputSize > 0, no dictCtx
        //      -> the dictionary is silently dropped (dictSize = 0).
        let tiny = vec![7u8, 8, 9];
        for &n in &[1usize, 12, 13, 100, 5000] {
            for sh in ALL_SHAPES {
                let blk = gen_real(&mut rng, sh, n);
                let cap = bound(l, n as c_int) as usize;
                let s = create_pair(l, "LZ4_createStream");
                let a0 = ldc(s.c, tiny.as_ptr() as *const c_char, 3);
                let b0 = ldr(s.r, tiny.as_ptr() as *const c_char, 3);
                assert_eq!(a0, b0, "row73 loadDict mismatch");
                assert_eq!(a0, 0, "row73: a 3-byte dictionary must report 0");
                let mut cb = dstbuf(cap);
                let mut rb = dstbuf(cap);
                let a = cc(
                    s.c,
                    blk.as_ptr() as *const c_char,
                    cb.as_mut_ptr() as *mut c_char,
                    n as c_int,
                    cap as c_int,
                    1,
                );
                let b = cr(
                    s.r,
                    blk.as_ptr() as *const c_char,
                    rb.as_mut_ptr() as *mut c_char,
                    n as c_int,
                    cap as c_int,
                    1,
                );
                let ctx = format!("row73 n={n} shape={sh:?}");
                same_int_and_bytes(&ctx, a, b, &cb, &rb);
                same_full_buffers(&ctx, &cb, &rb);
                assert!(a > 0, "{ctx}: expected success, got {a}");
                free_pair(l, "LZ4_freeStream", s);
            }
        }
        // ... and with inputSize == 0 the tolerance branch keeps the history:
        // LZ4_compress_generic returns the 1-byte empty block instead.
        {
            let s = create_pair(l, "LZ4_createStream");
            assert_eq!(
                ldc(s.c, tiny.as_ptr() as *const c_char, 3),
                ldr(s.r, tiny.as_ptr() as *const c_char, 3)
            );
            let mut cb = dstbuf(16);
            let mut rb = dstbuf(16);
            let a = cc(
                s.c,
                tiny.as_ptr() as *const c_char,
                cb.as_mut_ptr() as *mut c_char,
                0,
                16,
                1,
            );
            let b = cr(
                s.r,
                tiny.as_ptr() as *const c_char,
                rb.as_mut_ptr() as *mut c_char,
                0,
                16,
                1,
            );
            same_int_and_bytes("row73 inputSize=0", a, b, &cb, &rb);
            same_full_buffers("row73 inputSize=0", &cb, &rb);
            assert_eq!(a, 1, "row73: empty input must still emit the empty block");
            free_pair(l, "LZ4_freeStream", s);
        }

        // ---- Row 74: src overlaps the recorded dictionary
        //      (sourceEnd > dictionary && sourceEnd < dictEnd)
        //      -> dictionary shrunk to `dictEnd - sourceEnd`, zeroed when < 4.
        let buf = gen_real(&mut rng, Shape::TextLike, 100_000);
        // loadDict(buf, 70000) keeps buf[4464 .. 70000]; a following block that
        // ends inside that window triggers the trim.
        for &n in &[10_000usize, 60_000, 69_990, 69_996, 69_997, 69_998, 69_999] {
            let s = create_pair(l, "LZ4_createStream");
            let a0 = ldc(s.c, buf.as_ptr() as *const c_char, 70_000);
            let b0 = ldr(s.r, buf.as_ptr() as *const c_char, 70_000);
            assert_eq!(a0, b0, "row74 loadDict mismatch");
            assert_eq!(a0, 65536, "row74: expected the 64 KB truncation");
            let cap = bound(l, n as c_int) as usize;
            let mut cb = dstbuf(cap);
            let mut rb = dstbuf(cap);
            let a = cc(
                s.c,
                buf.as_ptr() as *const c_char,
                cb.as_mut_ptr() as *mut c_char,
                n as c_int,
                cap as c_int,
                1,
            );
            let b = cr(
                s.r,
                buf.as_ptr() as *const c_char,
                rb.as_mut_ptr() as *mut c_char,
                n as c_int,
                cap as c_int,
                1,
            );
            let ctx = format!("row74 overlap n={n}");
            same_int_and_bytes(&ctx, a, b, &cb, &rb);
            same_full_buffers(&ctx, &cb, &rb);
            assert!(a > 0, "{ctx}: expected success, got {a}");
            free_pair(l, "LZ4_freeStream", s);
        }

        // ---- Row 75: LZ4_renormDictT, currentOffset + nextSize > 0x80000000.
        // A 3-byte dictionary leaves `tableType == clearedTable` (loadDict
        // returns before lz4.c:1620), so every LZ4_resetStream_fast call takes
        // the `currentOffset += 64 KB` shortcut at lz4.c:913-915 without ever
        // resetting the table.  32766 of them put currentOffset at 0x7FFF0000,
        // exactly one 64 KB block below the rescale threshold.
        for &(k, n, renorm) in &[
            (32766usize, 65536usize, false), // 0x7FFF0000 + 0x10000 == 0x80000000, not `>`
            (32766, 65537, true),            // 0x80000001 > 0x80000000  -> rescale
            (32767, 100, true),              // 0x80000000 + 100         -> rescale
        ] {
            let src = gen_real(&mut rng, Shape::TextLike, n);
            let cap = bound(l, n as c_int) as usize;
            let s = create_pair(l, "LZ4_createStream");
            assert_eq!(
                ldc(s.c, tiny.as_ptr() as *const c_char, 3),
                ldr(s.r, tiny.as_ptr() as *const c_char, 3)
            );
            for _ in 0..k {
                rfc(s.c);
                rfr(s.r);
            }
            let mut cb = dstbuf(cap);
            let mut rb = dstbuf(cap);
            let a = cc(
                s.c,
                src.as_ptr() as *const c_char,
                cb.as_mut_ptr() as *mut c_char,
                n as c_int,
                cap as c_int,
                1,
            );
            let b = cr(
                s.r,
                src.as_ptr() as *const c_char,
                rb.as_mut_ptr() as *mut c_char,
                n as c_int,
                cap as c_int,
                1,
            );
            let ctx = format!("row75 k={k} n={n} renorm={renorm}");
            same_int_and_bytes(&ctx, a, b, &cb, &rb);
            same_full_buffers(&ctx, &cb, &rb);
            assert!(a > 0, "{ctx}: expected success, got {a}");
            // A second block on the same stream must also agree (the rescaled
            // hash table is now in use).
            let mut cb2 = dstbuf(cap);
            let mut rb2 = dstbuf(cap);
            let a2 = cc(
                s.c,
                src.as_ptr() as *const c_char,
                cb2.as_mut_ptr() as *mut c_char,
                n as c_int,
                cap as c_int,
                1,
            );
            let b2 = cr(
                s.r,
                src.as_ptr() as *const c_char,
                rb2.as_mut_ptr() as *mut c_char,
                n as c_int,
                cap as c_int,
                1,
            );
            same_int_and_bytes(&format!("{ctx} second"), a2, b2, &cb2, &rb2);
            same_full_buffers(&format!("{ctx} second"), &cb2, &rb2);
            free_pair(l, "LZ4_freeStream", s);
        }
    }
}

// ===========================================================================
// Rows 76, 77 - LZ4_saveDict: dictSize clamping; safeBuffer == NULL
// ===========================================================================

#[test]
fn err_076_077_save_dict_clamping_and_null_safebuffer() {
    let l = libs();
    let mut rng = Rng::new(76);
    unsafe {
        let (ldc, ldr) = l.sym::<FnLoadDict>("LZ4_loadDict");
        let (sc, sr) = l.sym::<FnSaveDict>("LZ4_saveDict");
        let dict = gen_real(&mut rng, Shape::TextLike, 200_000);

        // Row 76: (U32)dictSize > 64 KB -> 64 KB; then > dict->dictSize -> that.
        // A negative dictSize becomes huge under the (U32) cast and is clamped
        // the same way, so it is safe to pass.
        for &(loaded, want_loaded) in &[(200_000i32, 65536i32), (30_000, 30_000), (8, 8)] {
            for &ds in &[
                c_int::MIN,
                -100_000,
                -1,
                0,
                1,
                3,
                4,
                100,
                65535,
                65536,
                65537,
                100_000,
                c_int::MAX,
            ] {
                let s = create_pair(l, "LZ4_createStream");
                let a0 = ldc(s.c, dict.as_ptr() as *const c_char, loaded);
                let b0 = ldr(s.r, dict.as_ptr() as *const c_char, loaded);
                assert_eq!(a0, b0, "row76 loadDict mismatch");
                assert_eq!(a0, want_loaded, "row76: loadDict({loaded}) = {a0}");
                let mut cb = vec![SENT; 70_000];
                let mut rb = vec![SENT; 70_000];
                let a = sc(s.c, cb.as_mut_ptr() as *mut c_char, ds);
                let b = sr(s.r, rb.as_mut_ptr() as *mut c_char, ds);
                let ctx = format!("row76 loaded={loaded} ds={ds}");
                assert_eq!(a, b, "{ctx}: return mismatch (C={a} Rust={b})");
                let want = {
                    let mut d = if (ds as u32) > 65536 { 65536 } else { ds };
                    if (d as u32) > want_loaded as u32 {
                        d = want_loaded;
                    }
                    d
                };
                assert_eq!(a, want, "{ctx}: expected {want}, got {a}");
                same_full_buffers(&ctx, &cb, &rb);
                if a > 0 {
                    // The saved bytes are the last `a` bytes of the loaded
                    // dictionary region, i.e. of `dict[..loaded]`.
                    let n = a as usize;
                    let src_tail = &dict[(loaded as usize - n)..loaded as usize];
                    assert_eq!(
                        &cb[..n],
                        src_tail,
                        "{ctx}: saved bytes are not the dictionary tail"
                    );
                }
                free_pair(l, "LZ4_freeStream", s);
            }
        }

        // Row 77: `if (safeBuffer == NULL) assert(dictSize == 0);` (lz4.c:1823).
        // ASSERT-ONLY CONTRACT.  With a non-empty dictionary this would run
        // `LZ4_memmove(NULL, ..., dictSize)`; the assert is compiled out in this
        // build, so it would be a hard SIGSEGV rather than an abort.  It is
        // therefore NOT provoked.  The closest reachable in-contract behaviour
        // is the same call once the clamping at lz4.c:1821-1822 has driven
        // dictSize to 0 -- either because the stream holds no dictionary, or
        // because the requested size clamps to the stream's 0-sized one.
        for &ds in &[0i32, -1, 1, 3, 4, 100, 65536, c_int::MAX] {
            let s = create_pair(l, "LZ4_createStream"); // dict->dictSize == 0
            let a = sc(s.c, std::ptr::null_mut(), ds);
            let b = sr(s.r, std::ptr::null_mut(), ds);
            assert_eq!(a, b, "row77 ds={ds}: return mismatch (C={a} Rust={b})");
            assert_eq!(a, 0, "row77 ds={ds}: expected 0, got {a}");
            free_pair(l, "LZ4_freeStream", s);
        }
        // Same after a dictionary that loadDict rejected (dictSize stays 0).
        let tiny = vec![1u8, 2, 3];
        for &ds in &[0i32, 5, 70_000] {
            let s = create_pair(l, "LZ4_createStream");
            assert_eq!(
                ldc(s.c, tiny.as_ptr() as *const c_char, 3),
                ldr(s.r, tiny.as_ptr() as *const c_char, 3)
            );
            let a = sc(s.c, std::ptr::null_mut(), ds);
            let b = sr(s.r, std::ptr::null_mut(), ds);
            assert_eq!(a, b, "row77 rejected-dict ds={ds}: mismatch");
            assert_eq!(a, 0, "row77 rejected-dict ds={ds}: expected 0, got {a}");
            free_pair(l, "LZ4_freeStream", s);
        }
    }
}

// ===========================================================================
// Row 78 - LZ4_freeStreamDecode(NULL) -> 0
// ===========================================================================

#[test]
fn err_078_free_stream_decode_null() {
    let l = libs();
    unsafe {
        let (fc, fr) = l.sym::<FnFreePtr>("LZ4_freeStreamDecode");
        for _ in 0..100 {
            let a = fc(std::ptr::null_mut());
            let b = fr(std::ptr::null_mut());
            assert_eq!(a, b, "row78: return mismatch (C={a} Rust={b})");
            assert_eq!(a, 0, "row78: LZ4_freeStreamDecode(NULL) must be 0, got {a}");
        }
    }
}

// ===========================================================================
// Row 79 - LZ4_setStreamDecode: dictSize != 0 with dictionary == NULL
// ===========================================================================

#[test]
fn err_079_set_stream_decode_null_dictionary() {
    let l = libs();
    let mut rng = Rng::new(79);
    unsafe {
        let (fc, fr) = l.sym::<FnSetStreamDecode>("LZ4_setStreamDecode");
        // In-contract: dictSize == 0 with a NULL dictionary is explicitly
        // allowed ("Loading a size of 0 is allowed"), and the function always
        // returns 1.
        for _ in 0..50 {
            let sd = create_pair(l, "LZ4_createStreamDecode");
            let a = fc(sd.c, std::ptr::null(), 0);
            let b = fr(sd.r, std::ptr::null(), 0);
            assert_eq!(a, b, "row79 NULL/0: mismatch (C={a} Rust={b})");
            assert_eq!(a, 1, "row79: LZ4_setStreamDecode must return 1, got {a}");
            free_pair(l, "LZ4_freeStreamDecode", sd);
        }
        // ASSERT-ONLY CONTRACT (lz4.c:2594 `assert(dictionary != NULL)`), and
        // the assert is a no-op in this build.  The only thing the function
        // does with `dictionary` is the pointer arithmetic
        // `prefixEnd = dictionary + dictSize` -- no dereference -- so the call
        // itself is observable and safe.  The resulting decode state is
        // poisoned, so the stream is freed immediately afterwards and never
        // used to decode.
        for &ds in &[1i32, 2, 100, 65535, 65536, 1_000_000] {
            let sd = create_pair(l, "LZ4_createStreamDecode");
            let a = fc(sd.c, std::ptr::null(), ds);
            let b = fr(sd.r, std::ptr::null(), ds);
            assert_eq!(a, b, "row79 NULL/{ds}: mismatch (C={a} Rust={b})");
            assert_eq!(a, 1, "row79 NULL/{ds}: expected 1, got {a}");
            free_pair(l, "LZ4_freeStreamDecode", sd);
        }
        // A real dictionary of every size also just returns 1.
        let dict = gen_real(&mut rng, Shape::TextLike, 70_000);
        for &ds in &[0i32, 1, 4, 64, 65535, 65536, 70_000] {
            let sd = create_pair(l, "LZ4_createStreamDecode");
            let a = fc(sd.c, dict.as_ptr() as *const c_char, ds);
            let b = fr(sd.r, dict.as_ptr() as *const c_char, ds);
            assert_eq!(a, b, "row79 real/{ds}: mismatch");
            assert_eq!(a, 1, "row79 real/{ds}: expected 1, got {a}");
            free_pair(l, "LZ4_freeStreamDecode", sd);
        }
    }
}

// ===========================================================================
// Rows 80, 81 - LZ4_decoderRingBufferSize: maxBlockSize < 0 / > LZ4_MAX_INPUT_SIZE -> 0
// ===========================================================================

#[test]
fn err_080_081_decoder_ring_buffer_size_out_of_range() {
    let l = libs();
    let mut rng = Rng::new(80);
    unsafe {
        let (fc, fr) = l.sym::<FnCompressBound>("LZ4_decoderRingBufferSize");
        let want = |mbs: c_int| -> c_int {
            if mbs < 0 {
                0
            } else if mbs > LZ4_MAX_INPUT_SIZE as c_int {
                0
            } else {
                65536 + 14 + if mbs < 16 { 16 } else { mbs }
            }
        };
        let mut probes: Vec<c_int> = vec![
            c_int::MIN,
            c_int::MIN + 1,
            -1_000_000,
            -17,
            -16,
            -1,
            0,
            1,
            15,
            16,
            17,
            65535,
            65536,
            LZ4_MAX_INPUT_SIZE as c_int - 1,
            LZ4_MAX_INPUT_SIZE as c_int,
            LZ4_MAX_INPUT_SIZE as c_int + 1,
            LZ4_MAX_INPUT_SIZE as c_int + 2,
            c_int::MAX - 1,
            c_int::MAX,
        ];
        for _ in 0..20_000 {
            probes.push(rng.next_u32() as c_int);
        }
        for &mbs in &probes {
            let a = fc(mbs);
            let b = fr(mbs);
            assert_eq!(a, b, "row80/81 mbs={mbs}: mismatch (C={a} Rust={b})");
            assert_eq!(a, want(mbs), "row80/81 mbs={mbs}: expected {}", want(mbs));
            if mbs < 0 {
                assert_eq!(a, 0, "row80 mbs={mbs}: negative must give 0");
            }
            if mbs > LZ4_MAX_INPUT_SIZE as c_int {
                assert_eq!(a, 0, "row81 mbs={mbs}: over-cap must give 0");
            }
        }
    }
}

// ===========================================================================
// Rows 82, 83 - LZ4_decompress_generic: src == NULL / outputSize < 0 -> -1
// ===========================================================================

#[test]
fn err_082_083_decompress_null_src_or_negative_output() {
    let l = libs();
    let mut rng = Rng::new(82);
    unsafe {
        let (dc, dr) = l.sym::<FnDecompressSafe>("LZ4_decompress_safe");
        let (pc, pr) = l.sym::<FnDecompressSafePartial>("LZ4_decompress_safe_partial");

        // Row 82: src == NULL is rejected at lz4.c:2036 before any read, for
        // *every* compressedSize / dstCapacity combination.
        for &cs in &[0i32, 1, 2, 100, -1, c_int::MIN, c_int::MAX] {
            for &cap in &[0i32, 1, 64, 4096, -1, c_int::MIN, c_int::MAX] {
                let mut cb = dstbuf(4096);
                let mut rb = dstbuf(4096);
                let a = dc(std::ptr::null(), cb.as_mut_ptr() as *mut c_char, cs, cap);
                let b = dr(std::ptr::null(), rb.as_mut_ptr() as *mut c_char, cs, cap);
                let ctx = format!("row82 safe cs={cs} cap={cap}");
                same_int_and_bytes(&ctx, a, b, &cb, &rb);
                same_full_buffers(&ctx, &cb, &rb);
                assert_eq!(a, -1, "{ctx}: expected -1, got {a}");

                let mut cb = dstbuf(4096);
                let mut rb = dstbuf(4096);
                let a = pc(std::ptr::null(), cb.as_mut_ptr() as *mut c_char, cs, cap, cap);
                let b = pr(std::ptr::null(), rb.as_mut_ptr() as *mut c_char, cs, cap, cap);
                let ctx = format!("row82 partial cs={cs} cap={cap}");
                same_int_and_bytes(&ctx, a, b, &cb, &rb);
                same_full_buffers(&ctx, &cb, &rb);
                assert_eq!(a, -1, "{ctx}: expected -1, got {a}");
            }
        }

        // Row 83: outputSize < 0 -> -1, again before any read.  Because the
        // check precedes every dereference, a *lying* oversized compressedSize
        // is safe here (verified at lz4.c:2036).
        let src = gen_real(&mut rng, Shape::TextLike, 200);
        let comp = diff_compress_bytes(l, "row83 seed", &src, bound(l, 200) as usize, None);
        assert!(comp.0 > 0);
        let block = comp.1[..comp.0 as usize].to_vec();
        for &cap in &[-1i32, -2, -100, c_int::MIN, c_int::MIN + 1] {
            for &cs in &[
                0i32,
                1,
                block.len() as c_int,
                -1,
                c_int::MIN,
                LZ4_MAX_INPUT_SIZE as c_int,
                c_int::MAX,
            ] {
                let ctx = format!("row83 safe cs={cs} cap={cap}");
                let r = diff_dec_safe(l, &ctx, &block, cs, 4096, cap, GUARD);
                assert_eq!(r, -1, "{ctx}: expected -1, got {r}");
            }
        }
        // LZ4_decompress_safe_partial computes dstCapacity = MIN(target, cap)
        // first, so a negative *either* side lands in the same branch.
        for &(t, cap) in &[
            (-1i32, 100i32),
            (100, -1),
            (-1, -1),
            (c_int::MIN, 100),
            (100, c_int::MIN),
        ] {
            let ctx = format!("row83 partial t={t} cap={cap}");
            let r = diff_dec_partial(l, &ctx, &block, block.len() as c_int, t, 4096, cap, GUARD);
            assert_eq!(r, -1, "{ctx}: expected -1, got {r}");
        }
    }
}

// ===========================================================================
// Rows 84, 85 - outputSize == 0 (full block needs exactly the 1-byte empty
// block), srcSize == 0 with outputSize != 0
// ===========================================================================

#[test]
fn err_084_085_decompress_zero_sized_edges() {
    let l = libs();
    let mut rng = Rng::new(84);
    unsafe {
        // Row 84, full-block mode: `((srcSize==1) && (*ip==0)) ? 0 : -1`.
        // Note the short-circuit: `*ip` is only read when srcSize == 1.
        for b0 in 0..=255u8 {
            let blk = vec![b0, 0x11, 0x22, 0x33];
            let want = if b0 == 0 { 0 } else { -1 };
            let ctx = format!("row84 cap=0 cs=1 b0={b0:#04x}");
            let r = diff_dec_safe(l, &ctx, &blk, 1, 0, 0, GUARD);
            assert_eq!(r, want, "{ctx}: expected {want}, got {r}");
            // Partial mode returns 0 regardless of the input.
            let ctx = format!("row84 partial cap=0 cs=1 b0={b0:#04x}");
            let r = diff_dec_partial(l, &ctx, &blk, 1, 0, 0, 0, GUARD);
            assert_eq!(r, 0, "{ctx}: partial mode must return 0, got {r}");
        }
        // Any compressedSize other than exactly 1 is -1 in full-block mode.
        let blk = vec![0u8, 0, 0, 0, 0, 0, 0, 0];
        for &cs in &[
            0i32,
            2,
            3,
            8,
            -1,
            c_int::MIN,
            LZ4_MAX_INPUT_SIZE as c_int,
            c_int::MAX,
        ] {
            let ctx = format!("row84 cap=0 cs={cs}");
            let r = diff_dec_safe(l, &ctx, &blk, cs, 0, 0, GUARD);
            assert_eq!(r, -1, "{ctx}: expected -1, got {r}");
            let ctx = format!("row84 partial cap=0 cs={cs}");
            let r = diff_dec_partial(l, &ctx, &blk, cs, 0, 0, 0, GUARD);
            assert_eq!(r, 0, "{ctx}: partial mode must return 0, got {r}");
        }
        // Row 85: srcSize == 0 with outputSize != 0 -> -1 (lz4.c:2069).  Both
        // the fast-loop (cap >= 64) and safe-loop (cap < 64) entries.
        let seed = gen_real(&mut rng, Shape::TextLike, 500);
        let comp = diff_compress_bytes(l, "row85 seed", &seed, bound(l, 500) as usize, None);
        let block = comp.1[..comp.0 as usize].to_vec();
        for cap in [
            1usize, 2, 5, 12, 13, 32, 63, 64, 65, 100, 4096,
        ] {
            let ctx = format!("row85 cs=0 cap={cap}");
            let r = diff_dec_safe(l, &ctx, &block, 0, cap, cap as c_int, GUARD);
            assert_eq!(r, -1, "{ctx}: expected -1, got {r}");
            // Partial mode reaches the same check (its dstCapacity is > 0).
            let ctx = format!("row85 partial cs=0 cap={cap}");
            let r = diff_dec_partial(l, &ctx, &block, 0, cap as c_int, cap, cap as c_int, GUARD);
            assert_eq!(r, -1, "{ctx}: expected -1, got {r}");
        }
        for _ in 0..2000 {
            let cap = rng.range(1, 8192);
            let ctx = format!("row85 rand cap={cap}");
            let r = diff_dec_safe(l, &ctx, &block, 0, cap, cap as c_int, GUARD);
            assert_eq!(r, -1, "{ctx}: expected -1, got {r}");
        }
    }
}

// ===========================================================================
// Rows 86, 87, 88 - read_variable_length: initial_check, mid-length input
// exhaustion, 32-bit accumulator overflow
// ===========================================================================

#[test]
fn err_086_087_088_read_variable_length_errors() {
    let l = libs();
    unsafe {
        // Row 86 (`initial_check` set and `*ip >= ilimit` before the loop,
        // lz4.c:1985-1987).  A lone 0xF0 token: the literal length nibble is
        // RUN_MASK, and `ilimit == iend - RUN_MASK` is already behind `ip`.
        // `ip` stopped one byte in, so `_output_error` yields -(1)-1 == -2.
        for &cap in &[64usize, 65, 128, 4096] {
            let ctx = format!("row86 fastloop cap={cap}");
            let r = diff_dec_safe(l, &ctx, &[0xF0], 1, cap, cap as c_int, GUARD);
            assert_eq!(r, -2, "{ctx}: expected -2, got {r}");
        }
        for &cap in &[13usize, 20, 32, 63] {
            let ctx = format!("row86 safeloop cap={cap}");
            let r = diff_dec_safe(l, &ctx, &[0xF0], 1, cap, cap as c_int, GUARD);
            assert_eq!(r, -2, "{ctx}: expected -2, got {r}");
        }
        // `ilimit == iend - RUN_MASK`, so the initial check fires for every
        // declared srcSize <= 16 and the parse always stops one byte in (-2).
        for n in 1..=16usize {
            let mut blk = vec![0xF0u8];
            blk.extend(std::iter::repeat(0xFFu8).take(n - 1));
            let ctx = format!("row86 n={n}");
            let r = diff_dec_safe(l, &ctx, &blk, n as c_int, 4096, 4096, GUARD);
            assert_eq!(r, -2, "{ctx}: expected -2, got {r}");
        }

        // Row 87 (`*ip > ilimit` after consuming a length byte,
        // lz4.c:1992-1994 / :2003-2005).  20 declared bytes -> ilimit == src+5;
        // the 255-chain walks ip to src+6 before failing, so the encoded
        // offset is -(6)-1 == -7.
        let mut blk = vec![0xF0u8];
        blk.extend(std::iter::repeat(0xFFu8).take(19));
        for &cap in &[64usize, 4096, 20, 13] {
            let ctx = format!("row87 cap={cap}");
            let r = diff_dec_safe(l, &ctx, &blk, 20, cap, cap as c_int, GUARD);
            assert_eq!(r, -7, "{ctx}: expected -7, got {r}");
        }
        // Sweep the declared srcSize.  With an all-0xFF chain the loop stops
        // after reading the byte at index `n - 15`, so `ip == src + n - 14` and
        // the encoded offset is `-(n - 14) - 1 == -(n - 13)`.
        for n in 17..=200usize {
            let mut blk = vec![0xF0u8];
            blk.extend(std::iter::repeat(0xFFu8).take(n - 1));
            let want = -((n - 13) as c_int);
            let ctx = format!("row87 sweep n={n}");
            let r = diff_dec_safe(l, &ctx, &blk, n as c_int, 4096, 4096, GUARD);
            assert_eq!(r, want, "{ctx}: expected {want}, got {r}");
        }

        // Row 88: `(sizeof(length) < 8) && length > (Rvl_t)-1/2` --
        // UNREACHABLE on this target.  `Rvl_t` is `size_t`, which is 8 bytes
        // here, so the guarded expression is compiled out of both libraries
        // (lz4.c:1996-1998, :2007-2009).  Even if it were live, `length` is
        // bounded by `255 * srcSize <= 255 * 2^31`, far below `SIZE_MAX/2`.
        // Pin the fact that a very long *legal* 255-chain is accepted rather
        // than reported as an overflow: 3000 extension bytes give a literal
        // length of 15 + 255*2 + 90, which decodes cleanly.
        assert_eq!(
            std::mem::size_of::<usize>(),
            8,
            "row88 is only unreachable because Rvl_t is 8 bytes wide"
        );
        {
            let lit_len = 15usize + 255 * 2 + 90;
            let mut blk = vec![0xF0u8];
            push_ext(&mut blk, lit_len - 15);
            blk.extend(std::iter::repeat(0x5Au8).take(lit_len));
            let ctx = "row88 long-but-legal 255 chain";
            let r = diff_dec_safe(l, ctx, &blk, blk.len() as c_int, lit_len, lit_len as c_int, GUARD);
            assert_eq!(r, lit_len as c_int, "{ctx}: expected {lit_len}, got {r}");
        }
    }
}

// ---------------------------------------------------------------------------
// Hand-built malformed blocks with a *derived* exact `_output_error` value.
//
// `_output_error` returns `(int)(-((const char*)ip - src)) - 1` (lz4.c:2442),
// so every case below records the byte offset at which the parse must stop.
// Capacities are chosen to select the fast loop (`oend - op >= 64`) or the safe
// loop (`< 64`) deliberately.
// ---------------------------------------------------------------------------

struct BadBlock {
    tag: &'static str,
    bytes: Vec<u8>,
    /// declared `compressedSize`
    csize: c_int,
    /// declared `dstCapacity` (also the real allocation)
    cap: usize,
    /// exact expected return value
    want: c_int,
}

/// Row 92 shape: fast loop, ML nibble == RUN_MASK, all-0xFF match-length chain.
/// `iend - LASTLITERALS + 1 == src + 14` for `srcSize == 18`; the chain stops
/// after reading index 14, so `ip == src + 15` and the result is -16.
fn bad_row92() -> BadBlock {
    let mut b = vec![0x0Fu8, 0x01, 0x00];
    b.extend(std::iter::repeat(0xFFu8).take(15));
    BadBlock { tag: "row92 fastloop long-match rvl_error", bytes: b, csize: 18, cap: 4096, want: -16 }
}

/// Row 94 shape: fast loop, LL == 4, ML == 0 (length 4), offset 100 while only
/// 4 bytes of output exist, so `match < lowPrefix` and `match + 0 < lowPrefix`.
/// `ip == src + 7` -> -8.
fn bad_row94() -> BadBlock {
    let mut b = vec![0x40u8, 0xAA, 0xBB, 0xCC, 0xDD, 100, 0];
    b.extend(std::iter::repeat(0x5Au8).take(11)); // reach srcSize 18
    BadBlock { tag: "row94 fastloop offset outside buffers", bytes: b, csize: 18, cap: 4096, want: -8 }
}

/// Row 99 shape: safe loop, full-block mode, the literal run would overrun
/// `oend`.  Token 0x20 (LL 2, ML 0), 2 literals, dstCapacity 1.  `ip == src+1`.
fn bad_row99_short_dst() -> BadBlock {
    BadBlock {
        tag: "row99 literal run overruns dst",
        bytes: vec![0x20, 0x11, 0x22],
        csize: 3,
        cap: 1,
        want: -2,
    }
}

/// Row 99 shape: `ip + length != iend` -- there is a trailing byte after what
/// claims to be the final literal run, so it was not the last sequence.
fn bad_row99_trailing() -> BadBlock {
    BadBlock {
        tag: "row99 literal run is not the last sequence",
        bytes: vec![0x20, 0x11, 0x22, 0x33],
        csize: 4,
        cap: 64,
        want: -2,
    }
}

/// Row 100 shape: safe loop, `_copy_match` long match length rvl_error.
/// LL 8 / offset 8 / ML nibble 15, chain of 0xFF from index 11; with
/// `srcSize == 17`, `ilimit == src + 13` and the chain stops with `ip == src+14`.
fn bad_row100() -> BadBlock {
    let mut b = vec![0x8Fu8];
    b.extend_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]); // 8 literals
    b.extend_from_slice(&[8, 0]); // offset 8
    b.extend(std::iter::repeat(0xFFu8).take(6));
    BadBlock { tag: "row100 safe-loop long-match rvl_error", bytes: b, csize: 17, cap: 20, want: -15 }
}

/// Row 102 shape: safe loop `safe_match_copy`, offset outside buffers.
/// LL 8 / offset 100 / ML nibble 0. `ip == src + 11` -> -12.
fn bad_row102() -> BadBlock {
    let mut b = vec![0x80u8];
    b.extend_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]);
    b.extend_from_slice(&[100, 0]);
    b.extend(std::iter::repeat(0x5Au8).take(6));
    BadBlock { tag: "row102 safe_match_copy offset outside buffers", bytes: b, csize: 17, cap: 20, want: -12 }
}

/// Row 104 shape: the match copy would end within the last LASTLITERALS bytes.
/// LL 8 / offset 8 / ML nibble 14 (length 18) with dstCapacity 20:
/// `cpy == dst + 26 > oend - 5`. `ip == src + 11` -> -12.
fn bad_row104() -> BadBlock {
    let mut b = vec![0x8Eu8];
    b.extend_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]);
    b.extend_from_slice(&[8, 0]);
    b.extend(std::iter::repeat(0x5Au8).take(6));
    BadBlock { tag: "row104 match ends inside the last 5 bytes", bytes: b, csize: 17, cap: 20, want: -12 }
}

fn all_bad_blocks() -> Vec<BadBlock> {
    vec![
        bad_row92(),
        bad_row94(),
        bad_row99_short_dst(),
        bad_row99_trailing(),
        bad_row100(),
        bad_row102(),
        bad_row104(),
    ]
}

// ===========================================================================
// Rows 89, 90, 91 - fast loop: long literal length rvl_error, and the two
// literal-length pointer-overflow guards
// ===========================================================================

#[test]
fn err_089_090_091_fastloop_literal_length_errors() {
    let l = libs();
    let mut rng = Rng::new(89);
    unsafe {
        // Row 89 (lz4.c:2092-2097): the fast loop is entered whenever
        // `oend - op >= FASTLOOP_SAFE_DISTANCE == 64`, and a 0xF0 token whose
        // length chain cannot be read there jumps to `_output_error`.
        for cap in [FASTLOOP_SAFE_DISTANCE, 65, 100, 1000, 100_000] {
            let ctx = format!("row89 lone 0xF0 cap={cap}");
            let r = diff_dec_safe(l, &ctx, &[0xF0], 1, cap, cap as c_int, GUARD);
            assert_eq!(r, -2, "{ctx}: expected -2, got {r}");
            // A chain that runs out mid-way, still in the fast loop.
            let mut blk = vec![0xF0u8];
            blk.extend(std::iter::repeat(0xFFu8).take(29));
            let ctx = format!("row89 chain cap={cap}");
            let r = diff_dec_safe(l, &ctx, &blk, 30, cap, cap as c_int, GUARD);
            assert_eq!(r, -(30 - 13), "{ctx}: expected {}, got {r}", -(30 - 13));
        }
        // Randomised: a 0xF0 token followed by a random 255-chain, always in the
        // fast loop, must produce the same exact integer in both libraries.
        for i in 0..20_000usize {
            let n = rng.range(1, 120);
            let mut blk = vec![0xF0u8];
            for _ in 1..n {
                blk.push(if rng.below(4) == 0 { rng.byte() } else { 0xFF });
            }
            let cap = rng.range(64, 4096);
            let ctx = format!("row89 rand i={i} n={n} cap={cap}");
            diff_dec_safe(l, &ctx, &blk, n as c_int, cap, cap as c_int, GUARD);
        }

        // Rows 90 and 91 (lz4.c:2099, :2100) are the guards
        // `(uptrval)op + length < (uptrval)op` and the same for `ip`.
        // UNREACHABLE on this (64-bit) target: `read_variable_length` adds at
        // most 255 per consumed input byte and stops at `iend`, so
        // `length <= 255 * srcSize <= 255 * (2^31 - 1) < 2^40`, which cannot
        // wrap a 64-bit pointer.  Provoking a wrap would need ~2^56 input
        // bytes.  Pin the reachable neighbour instead: the largest literal
        // length that the guards let through decodes exactly.
        assert_eq!(std::mem::size_of::<usize>(), 8, "rows 90/91 unreachability assumes 64-bit");
        for lit_len in [15usize, 16, 269, 270, 524, 5000] {
            let mut blk = vec![0xF0u8];
            push_ext(&mut blk, lit_len - 15);
            blk.extend((0..lit_len).map(|_| rng.byte()));
            let ctx = format!("row90/91 legal lit_len={lit_len}");
            let r = diff_dec_safe(
                l,
                &ctx,
                &blk,
                blk.len() as c_int,
                lit_len + 64,
                (lit_len + 64) as c_int,
                GUARD,
            );
            assert_eq!(r, lit_len as c_int, "{ctx}: expected {lit_len}, got {r}");
        }
    }
}

// ===========================================================================
// Rows 92, 93 - fast loop: long match length rvl_error, match-length pointer
// overflow
// ===========================================================================

#[test]
fn err_092_093_fastloop_match_length_errors() {
    let l = libs();
    let mut rng = Rng::new(92);
    unsafe {
        // Row 92 (lz4.c:2126-2132), exact value derived in `bad_row92`.
        let b = bad_row92();
        let r = diff_dec_safe(l, b.tag, &b.bytes, b.csize, b.cap, b.cap as c_int, GUARD);
        assert_eq!(r, b.want, "{}: expected {}, got {r}", b.tag, b.want);
        // Sweep the declared srcSize.  `srcSize >= 18` keeps the fast loop's
        // 16-byte literal shortcut viable (`ip <= iend - 17`).
        // `ilimit == iend - LASTLITERALS + 1`, so the chain (starting at index
        // 3) stops after reading index `n - 4`, giving `ip == src + n - 3` and
        // an encoded offset of `-(n - 2)`.
        for n in 18..=200usize {
            let mut blk = vec![0x0Fu8, 0x01, 0x00];
            blk.extend(std::iter::repeat(0xFFu8).take(n.saturating_sub(3)));
            let ctx = format!("row92 sweep n={n}");
            let r = diff_dec_safe(l, &ctx, &blk, n as c_int, 4096, 4096, GUARD);
            assert_eq!(r, -((n - 2) as c_int), "{ctx}: expected {}, got {r}", -((n as c_int) - 2));
        }
        // Randomised match-length chains.
        for i in 0..20_000usize {
            let n = rng.range(3, 120);
            let mut blk = vec![0x0Fu8, 0x01, 0x00];
            for _ in 3..n {
                blk.push(if rng.below(4) == 0 { rng.byte() } else { 0xFF });
            }
            let cap = rng.range(64, 4096);
            let ctx = format!("row92 rand i={i} n={n} cap={cap}");
            diff_dec_safe(l, &ctx, &blk, n as c_int, cap, cap as c_int, GUARD);
        }

        // Row 93 (lz4.c:2136) is `(uptrval)op + length < (uptrval)op` for the
        // match length -- UNREACHABLE for the same arithmetic reason as rows
        // 90/91.  Pin the reachable neighbour: very long *legal* match lengths.
        for ml in [19usize, 274, 529, 5000] {
            let lits = (0..16usize).map(|_| rng.byte()).collect::<Vec<u8>>();
            let mut blk = Blk::new();
            blk.seq(&lits, 16, ml);
            let (comp, plain) = blk.finish(&(0..20).map(|_| rng.byte()).collect::<Vec<u8>>());
            let ctx = format!("row93 legal ml={ml}");
            let r = diff_dec_safe(
                l,
                &ctx,
                &comp,
                comp.len() as c_int,
                plain.len(),
                plain.len() as c_int,
                GUARD,
            );
            assert_eq!(r, plain.len() as c_int, "{ctx}: expected {}, got {r}", plain.len());
        }
    }
}

// ===========================================================================
// Row 94 - fast loop: offset points outside available history
// ===========================================================================

#[test]
fn err_094_fastloop_offset_outside_buffers() {
    let l = libs();
    let mut rng = Rng::new(94);
    unsafe {
        let b = bad_row94();
        let r = diff_dec_safe(l, b.tag, &b.bytes, b.csize, b.cap, b.cap as c_int, GUARD);
        assert_eq!(r, b.want, "{}: expected {}, got {r}", b.tag, b.want);

        // Property sweep: a first sequence with `ll` literals and an offset
        // strictly greater than `ll` can never be satisfied (dictSize == 0), so
        // it always fails at lz4.c:2162-2164 with `ip == src + 1 + ll + 2`.
        for ll in 0..=14usize {
            for extra in [1usize, 2, 3, 17, 200, 5000, 65535] {
                let off = ll + extra;
                if off > 65535 {
                    continue;
                }
                let mut blk = vec![((ll as u8) << 4) | 0x00];
                blk.extend((0..ll).map(|_| rng.byte()));
                blk.push((off & 0xFF) as u8);
                blk.push(((off >> 8) & 0xFF) as u8);
                // Pad the declared input so the fast loop's 16-byte literal
                // shortcut is taken (`ip <= iend - 17`).
                while blk.len() < 1 + ll + 2 + 20 {
                    blk.push(rng.byte());
                }
                let want = -((1 + ll + 2) as c_int) - 1;
                let ctx = format!("row94 ll={ll} off={off}");
                let r = diff_dec_safe(l, &ctx, &blk, blk.len() as c_int, 4096, 4096, GUARD);
                assert_eq!(r, want, "{ctx}: expected {want}, got {r}");
            }
        }
        // Offset 0 is also "outside": `match == op`, and `checkOffset` sees
        // `match + 0 < lowPrefix` only when `op < dst`, which cannot happen, so
        // offset 0 instead survives to the copy path.  Assert C == Rust for the
        // whole offset ladder including 0.
        for off in [0usize, 1, 2, 4, 8, 15, 16] {
            let ll = 20usize;
            let mut blk = vec![0xF0u8];
            push_ext(&mut blk, ll - 15);
            blk.extend((0..ll).map(|_| rng.byte()));
            blk.push((off & 0xFF) as u8);
            blk.push(((off >> 8) & 0xFF) as u8);
            blk.push(0x00); // ML nibble 0 -> length 4
            let tail: Vec<u8> = (0..20).map(|_| rng.byte()).collect();
            blk.push(0xF0);
            push_ext(&mut blk, tail.len() - 15);
            blk.extend_from_slice(&tail);
            let ctx = format!("row94 offset ladder off={off}");
            diff_dec_safe(l, &ctx, &blk, blk.len() as c_int, 4096, 4096, GUARD);
        }
    }
}

// ===========================================================================
// Row 95 - fast loop, extDict, full-block: op + length > oend - LASTLITERALS.
// UNREACHABLE while LZ4_FAST_DEC_LOOP == 1.
// ===========================================================================

#[test]
fn err_095_fastloop_extdict_end_of_block_rule_unreachable() {
    // Both fast-loop match-length branches jump to `safe_match_copy` as soon as
    // `op + length >= oend - FASTLOOP_SAFE_DISTANCE` (lz4.c:2138-2145), so
    // surviving into the extDict block requires `op + length < oend - 64`,
    // which is incompatible with the row-95 condition
    // `op + length > oend - LASTLITERALS(5)` that follows at lz4.c:2167.
    // The reachable twin is row 103 (`safe_match_copy`), covered separately.
    //
    // What *is* checkable here is that the fast-loop extDict path itself agrees
    // between the two libraries, including right at the `oend - 64` frontier.
    let l = libs();
    let mut rng = Rng::new(95);
    unsafe {
        let (dc, dr) = l.sym::<FnDecUsingDict>("LZ4_decompress_safe_usingDict");
        let dict = gen_real(&mut rng, Shape::TextLike, 40_000);
        for &dsz in &[16i32, 1000, 40_000] {
            for _ in 0..400 {
                // A block that matches into the external dictionary.
                let mut plain: Vec<u8> = Vec::new();
                let start = rng.below(dict.len() - 2000);
                plain.extend_from_slice(&dict[start..start + rng.range(200, 1500)]);
                plain.extend((0..rng.range(20, 200)).map(|_| rng.byte()));
                let cap = plain.len() + 64;
                let comp = {
                    let s = create_pair(l, "LZ4_createStream");
                    let (ldc, ldr) = l.sym::<FnLoadDict>("LZ4_loadDict");
                    let dp = dict.as_ptr().add(dict.len() - dsz as usize) as *const c_char;
                    assert_eq!(ldc(s.c, dp, dsz), ldr(s.r, dp, dsz));
                    let (cc, cr) = l.sym::<FnFastContinue>("LZ4_compress_fast_continue");
                    let b = bound(l, plain.len() as c_int) as usize;
                    let mut cb = dstbuf(b);
                    let mut rb = dstbuf(b);
                    let a = cc(
                        s.c,
                        plain.as_ptr() as *const c_char,
                        cb.as_mut_ptr() as *mut c_char,
                        plain.len() as c_int,
                        b as c_int,
                        1,
                    );
                    let bb = cr(
                        s.r,
                        plain.as_ptr() as *const c_char,
                        rb.as_mut_ptr() as *mut c_char,
                        plain.len() as c_int,
                        b as c_int,
                        1,
                    );
                    same_int_and_bytes("row95 seed compress", a, bb, &cb, &rb);
                    free_pair(l, "LZ4_freeStream", s);
                    assert!(a > 0);
                    cb[..a as usize].to_vec()
                };
                let p = padded(&comp, GUARD);
                let mut cb = dstbuf(cap);
                let mut rb = dstbuf(cap);
                let dp = dict.as_ptr().add(dict.len() - dsz as usize) as *const c_char;
                let a = dc(
                    p.as_ptr() as *const c_char,
                    cb.as_mut_ptr() as *mut c_char,
                    comp.len() as c_int,
                    cap as c_int,
                    dp,
                    dsz,
                );
                let b = dr(
                    p.as_ptr() as *const c_char,
                    rb.as_mut_ptr() as *mut c_char,
                    comp.len() as c_int,
                    cap as c_int,
                    dp,
                    dsz,
                );
                let ctx = format!("row95 extdict dsz={dsz} n={}", plain.len());
                same_int_and_bytes(&ctx, a, b, &cb, &rb);
                same_full_buffers(&ctx, &cb, &rb);
                assert_eq!(a, plain.len() as c_int, "{ctx}: expected a clean decode");
                assert_eq!(&cb[..plain.len()], &plain[..], "{ctx}: wrong plaintext");
            }
        }
    }
}

// ===========================================================================
// Rows 96, 97, 98 - safe loop: long literal length rvl_error and the two
// pointer-overflow guards
// ===========================================================================

#[test]
fn err_096_097_098_safeloop_literal_length_errors() {
    let l = libs();
    let mut rng = Rng::new(96);
    unsafe {
        // Row 96 (lz4.c:2265-2266): identical trigger to row 89 but with
        // `oend - op < FASTLOOP_SAFE_DISTANCE`, so the safe loop handles it.
        for cap in [0usize + 1, 2, 5, 12, 13, 20, 32, 63] {
            let ctx = format!("row96 lone 0xF0 cap={cap}");
            let r = diff_dec_safe(l, &ctx, &[0xF0], 1, cap, cap as c_int, GUARD);
            assert_eq!(r, -2, "{ctx}: expected -2, got {r}");
        }
        for n in 17..=120usize {
            let mut blk = vec![0xF0u8];
            blk.extend(std::iter::repeat(0xFFu8).take(n - 1));
            for cap in [1usize, 13, 40, 63] {
                let ctx = format!("row96 sweep n={n} cap={cap}");
                let r = diff_dec_safe(l, &ctx, &blk, n as c_int, cap, cap as c_int, GUARD);
                assert_eq!(r, -((n - 13) as c_int), "{ctx}: expected {}, got {r}", -((n as c_int) - 13));
            }
        }
        for i in 0..20_000usize {
            let n = rng.range(1, 120);
            let mut blk = vec![0xF0u8];
            for _ in 1..n {
                blk.push(if rng.below(4) == 0 { rng.byte() } else { 0xFF });
            }
            let cap = rng.range(1, 63);
            let ctx = format!("row96 rand i={i} n={n} cap={cap}");
            diff_dec_safe(l, &ctx, &blk, n as c_int, cap, cap as c_int, GUARD);
        }
        // Rows 97 and 98 (lz4.c:2268, :2269) are the safe-loop copies of rows
        // 90/91 and are UNREACHABLE for the same reason (64-bit pointers cannot
        // be wrapped by `length <= 255 * srcSize`).  Pin the reachable
        // neighbour: the longest literal runs that fit in a small dst.
        assert_eq!(std::mem::size_of::<usize>(), 8, "rows 97/98 unreachability assumes 64-bit");
        for lit_len in [15usize, 20, 40, 63] {
            let mut blk = vec![0xF0u8];
            push_ext(&mut blk, lit_len - 15);
            blk.extend((0..lit_len).map(|_| rng.byte()));
            let ctx = format!("row97/98 legal lit_len={lit_len}");
            let r = diff_dec_safe(l, &ctx, &blk, blk.len() as c_int, lit_len, lit_len as c_int, GUARD);
            assert_eq!(r, lit_len as c_int, "{ctx}: expected {lit_len}, got {r}");
        }
    }
}

// ===========================================================================
// Row 99 - safe_literal_copy, full-block: not the last literal run, or the
// output would overflow
// ===========================================================================

#[test]
fn err_099_safe_literal_copy_not_last_run_or_overflow() {
    let l = libs();
    let mut rng = Rng::new(99);
    unsafe {
        for b in [bad_row99_short_dst(), bad_row99_trailing()] {
            let r = diff_dec_safe(l, b.tag, &b.bytes, b.csize, b.cap, b.cap as c_int, GUARD);
            assert_eq!(r, b.want, "{}: expected {}, got {r}", b.tag, b.want);
            // Partial mode must NOT report an error for the same bytes: it is
            // allowed to stop early (lz4.c:2286-2308).
            let r = diff_dec_partial(
                l,
                &format!("{} [partial]", b.tag),
                &b.bytes,
                b.csize,
                b.cap as c_int,
                b.cap,
                b.cap as c_int,
                GUARD,
            );
            assert!(r >= 0, "{} [partial]: expected >= 0, got {r}", b.tag);
        }
        // (a) declared literal length longer than the remaining input.  With a
        // single-byte token (ll <= 14) the parse always stops right after the
        // token, so the encoded offset is exactly -2.
        for ll in 1..=14usize {
            for missing in 1..=ll.min(3) {
                let mut blk = vec![(ll as u8) << 4];
                blk.extend((0..(ll - missing)).map(|_| rng.byte()));
                let ctx = format!("row99 short input ll={ll} missing={missing}");
                let r = diff_dec_safe(l, &ctx, &blk, blk.len() as c_int, 4096, 4096, GUARD);
                assert_eq!(r, -2, "{ctx}: expected -2, got {r}");
            }
        }
        // ... and with a 255-extended literal length the parse stops after the
        // extension bytes, i.e. at `-(1 + extBytes) - 1`.  The declared srcSize
        // must exceed RUN_MASK + 1 == 16, otherwise `read_variable_length`'s
        // initial check fires first (that is row 86).
        for ll in [16usize, 20, 40, 100, 270, 300] {
            for missing in 1..=3usize {
                let mut blk = vec![0xF0u8];
                push_ext(&mut blk, ll - 15);
                let header = blk.len();
                blk.extend((0..(ll - missing)).map(|_| rng.byte()));
                if blk.len() <= 16 {
                    continue;
                }
                let want = -(header as c_int) - 1;
                let ctx = format!("row99 short input ext ll={ll} missing={missing}");
                let r = diff_dec_safe(l, &ctx, &blk, blk.len() as c_int, 4096, 4096, GUARD);
                assert_eq!(r, want, "{ctx}: expected {want}, got {r}");
            }
        }
        // (b) declared literal length longer than the remaining output
        // (`cpy > oend`).  dstCapacity 0 is excluded: it is handled by the
        // `outputSize == 0` special case (row 84) before the loop.
        for ll in 2..=14usize {
            for cap in 1..ll {
                let mut blk = vec![(ll as u8) << 4];
                blk.extend((0..ll).map(|_| rng.byte()));
                let ctx = format!("row99 short dst ll={ll} cap={cap}");
                let r = diff_dec_safe(l, &ctx, &blk, blk.len() as c_int, cap, cap as c_int, GUARD);
                assert_eq!(r, -2, "{ctx}: expected -2, got {r}");
            }
        }
        // (c) trailing junk after a complete block.
        for extra in 1..=8usize {
            let plain: Vec<u8> = (0..40).map(|_| rng.byte()).collect();
            let comp = diff_compress_bytes(
                l,
                "row99 seed",
                &plain,
                bound(l, plain.len() as c_int) as usize,
                None,
            );
            let mut blk = comp.1[..comp.0 as usize].to_vec();
            let real = blk.len();
            blk.extend((0..extra).map(|_| rng.byte()));
            let ctx = format!("row99 trailing junk extra={extra}");
            let r = diff_dec_safe(l, &ctx, &blk, blk.len() as c_int, 4096, 4096, GUARD);
            assert!(r < 0, "{ctx}: expected an error, got {r}");
            // Declaring the true size decodes cleanly.
            let ctx = format!("row99 trailing junk honest real={real}");
            let r = diff_dec_safe(l, &ctx, &blk, real as c_int, 4096, 4096, GUARD);
            assert_eq!(r, plain.len() as c_int, "{ctx}: expected {}, got {r}", plain.len());
        }
    }
}

// ===========================================================================
// Rows 100, 101 - _copy_match: long match length rvl_error, match-length
// pointer overflow
// ===========================================================================

#[test]
fn err_100_101_copy_match_length_errors() {
    let l = libs();
    let mut rng = Rng::new(100);
    unsafe {
        let b = bad_row100();
        let r = diff_dec_safe(l, b.tag, &b.bytes, b.csize, b.cap, b.cap as c_int, GUARD);
        assert_eq!(r, b.want, "{}: expected {}, got {r}", b.tag, b.want);
        // Sweep: LL 8, offset 8, ML nibble 15 with an all-0xFF chain from index
        // 11.  `ilimit == iend - 4`, so the chain stops after reading index
        // `n - 4`, giving `ip == src + n - 3` and `-(n - 2)`.
        // `srcSize >= 17` keeps `ip + 8 <= iend - 8` so the literal run takes
        // the wildcopy branch, and `dstCapacity <= 31` keeps the safe loop's
        // two-stage shortcut disabled (it needs `op <= oend - 32`).
        for n in 17..=120usize {
            let mut blk = vec![0x8Fu8];
            blk.extend_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]);
            blk.extend_from_slice(&[8, 0]);
            blk.extend(std::iter::repeat(0xFFu8).take(n - 11));
            for cap in [20usize, 24, 31] {
                let ctx = format!("row100 sweep n={n} cap={cap}");
                let r = diff_dec_safe(l, &ctx, &blk, n as c_int, cap, cap as c_int, GUARD);
                assert_eq!(r, -((n - 2) as c_int), "{ctx}: expected {}, got {r}", -((n as c_int) - 2));
            }
        }
        for i in 0..20_000usize {
            let n = rng.range(11, 120);
            let mut blk = vec![0x8Fu8];
            blk.extend_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]);
            blk.extend_from_slice(&[8, 0]);
            for _ in 11..n {
                blk.push(if rng.below(4) == 0 { rng.byte() } else { 0xFF });
            }
            let cap = rng.range(13, 31);
            let ctx = format!("row100 rand i={i} n={n} cap={cap}");
            diff_dec_safe(l, &ctx, &blk, n as c_int, cap, cap as c_int, GUARD);
        }
        // Row 101 (lz4.c:2349) -- UNREACHABLE, 64-bit pointer overflow, same
        // argument as rows 90/91/93/97/98.
        assert_eq!(std::mem::size_of::<usize>(), 8, "row101 unreachability assumes 64-bit");
    }
}

// ===========================================================================
// Rows 102, 103 - safe_match_copy: offset outside buffers; extDict full-block
// end-of-block rule
// ===========================================================================

#[test]
fn err_102_103_safe_match_copy_errors() {
    let l = libs();
    let mut rng = Rng::new(102);
    unsafe {
        // Row 102 (lz4.c:2356) with dictSize == 0 (plain LZ4_decompress_safe).
        let b = bad_row102();
        let r = diff_dec_safe(l, b.tag, &b.bytes, b.csize, b.cap, b.cap as c_int, GUARD);
        assert_eq!(r, b.want, "{}: expected {}, got {r}", b.tag, b.want);
        // Property sweep in the safe loop: LL literals then an offset strictly
        // greater than LL.  `ip == src + 1 + ll + 2`.
        for ll in 0..=14usize {
            for extra in [1usize, 2, 5, 100, 65000] {
                let off = ll + extra;
                if off > 65535 {
                    continue;
                }
                let mut blk = vec![((ll as u8) << 4) | 0x00];
                blk.extend((0..ll).map(|_| rng.byte()));
                blk.push((off & 0xFF) as u8);
                blk.push(((off >> 8) & 0xFF) as u8);
                blk.extend((0..8).map(|_| rng.byte()));
                let want = -((1 + ll + 2) as c_int) - 1;
                // `dstCapacity >= ll + MFLIMIT` makes the literal run take the
                // wildcopy branch (so the parse reaches `safe_match_copy`), and
                // `dstCapacity <= 31` keeps the safe loop's two-stage shortcut
                // (`op <= oend - 32`) disabled.
                for cap in [ll + 12, ll + 13, 31] {
                    if cap > 31 {
                        continue;
                    }
                    let ctx = format!("row102 ll={ll} off={off} cap={cap}");
                    let r =
                        diff_dec_safe(l, &ctx, &blk, blk.len() as c_int, cap, cap as c_int, GUARD);
                    assert_eq!(r, want, "{ctx}: expected {want}, got {r}");
                }
            }
        }

        // Row 103 (lz4.c:2359-2362): `dict == usingExtDict`, full-block mode,
        // `op + length > oend - LASTLITERALS`.  Reached through
        // LZ4_decompress_safe_usingDict with a dictionary that is *not*
        // adjacent to `dst` (-> LZ4_decompress_safe_forceExtDict).
        let (dc, dr) = l.sym::<FnDecUsingDict>("LZ4_decompress_safe_usingDict");
        let dict = gen_real(&mut rng, Shape::TextLike, 1000);
        // LL 8 / offset 100 (points into the extDict) / ML nibble 14 -> length
        // 18; with dstCapacity 20, `op + length == dst + 26 > oend - 5`.
        let mut blk = vec![0x8Eu8];
        blk.extend_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]);
        blk.extend_from_slice(&[100, 0]);
        blk.extend(std::iter::repeat(0x5Au8).take(6));
        let p = padded(&blk, GUARD);
        for &dsz in &[1000i32, 200, 92] {
            let cap = 20usize;
            let mut cb = dstbuf(cap);
            let mut rb = dstbuf(cap);
            let dp = dict.as_ptr() as *const c_char;
            let a = dc(
                p.as_ptr() as *const c_char,
                cb.as_mut_ptr() as *mut c_char,
                17,
                cap as c_int,
                dp,
                dsz,
            );
            let b = dr(
                p.as_ptr() as *const c_char,
                rb.as_mut_ptr() as *mut c_char,
                17,
                cap as c_int,
                dp,
                dsz,
            );
            let ctx = format!("row103 extdict end-of-block dsz={dsz}");
            same_int_and_bytes(&ctx, a, b, &cb, &rb);
            same_full_buffers(&ctx, &cb, &rb);
            assert_eq!(a, -12, "{ctx}: expected -12, got {a}");
            // Partial mode clamps instead of failing (lz4.c:2360).
            let (pc, pr) = l.sym::<FnDecPartialUsingDict>("LZ4_decompress_safe_partial_usingDict");
            let mut cb = dstbuf(cap);
            let mut rb = dstbuf(cap);
            let a = pc(
                p.as_ptr() as *const c_char,
                cb.as_mut_ptr() as *mut c_char,
                17,
                cap as c_int,
                cap as c_int,
                dp,
                dsz,
            );
            let b = pr(
                p.as_ptr() as *const c_char,
                rb.as_mut_ptr() as *mut c_char,
                17,
                cap as c_int,
                cap as c_int,
                dp,
                dsz,
            );
            let ctx = format!("row103 partial dsz={dsz}");
            same_int_and_bytes(&ctx, a, b, &cb, &rb);
            same_full_buffers(&ctx, &cb, &rb);
            assert!(a >= 0, "{ctx}: partial mode must clamp, got {a}");
        }
    }
}

// ===========================================================================
// Row 104 - match copy tail: cpy > oend - LASTLITERALS (last 5 bytes of a
// block must be literals)
// ===========================================================================

#[test]
fn err_104_last_five_bytes_must_be_literals() {
    let l = libs();
    let mut rng = Rng::new(104);
    unsafe {
        let b = bad_row104();
        let r = diff_dec_safe(l, b.tag, &b.bytes, b.csize, b.cap, b.cap as c_int, GUARD);
        assert_eq!(r, b.want, "{}: expected {}, got {r}", b.tag, b.want);

        // Property sweep: a well-formed prefix followed by a final literal run
        // shorter than LASTLITERALS == 5.
        //
        // NOTE: such a block is *not* always rejected.  The safe loop's
        // two-stage shortcut (lz4.c:2228-2255) copies 16 literal bytes and 18
        // match bytes and `continue`s without consulting the end-of-block rule
        // whenever `ip < iend - 16 && op <= oend - 32`; that is memory-safe (the
        // `shortoend` guard) but skips the row-104 check.  The contract asserted
        // here is therefore the exact agreement of C and Rust (checked inside
        // `diff_dec_safe`) plus non-vacuousness of the sweep.
        let mut kinds: BTreeMap<c_int, usize> = BTreeMap::new();
        for _ in 0..20_000usize {
            let hl = rng.range(8, 40);
            let head: Vec<u8> = (0..hl).map(|_| rng.byte()).collect();
            let off = rng.range(1, head.len());
            let ml = rng.range(4, 40);
            let mut blk = Blk::new();
            blk.seq(&head, off, ml);
            let t = rng.below(5);
            let tail: Vec<u8> = (0..t).map(|_| rng.byte()).collect();
            let (comp, plain) = blk.finish(&tail);
            let ctx = format!("row104 short tail t={t} hl={hl} off={off} ml={ml}");
            let r = diff_dec_safe(
                l,
                &ctx,
                &comp,
                comp.len() as c_int,
                plain.len(),
                plain.len() as c_int,
                GUARD,
            );
            *kinds.entry(r).or_insert(0) += 1;
        }
        let negs: usize = kinds.iter().filter(|(k, _)| **k < 0).map(|(_, v)| *v).sum();
        let distinct_negs = kinds.keys().filter(|k| **k < 0).count();
        assert!(
            negs > 1000 && distinct_negs >= 3,
            "row104: sweep is too weak ({negs} rejections, {distinct_negs} distinct offsets)"
        );
        // The deterministic families where the shortcut cannot apply: a literal
        // run of >= RUN_MASK bytes disables the shortcut (its first condition is
        // `length != RUN_MASK`), so a short final literal run *must* be caught.
        for t in 0..5usize {
            for hl in [20usize, 25, 30] {
                let head: Vec<u8> = (0..hl).map(|_| rng.byte()).collect();
                let mut blk = Blk::new();
                blk.seq(&head, hl, 300);
                let tail: Vec<u8> = (0..t).map(|_| rng.byte()).collect();
                let (comp, plain) = blk.finish(&tail);
                let ctx = format!("row104 RUN_MASK head hl={hl} t={t}");
                let r = diff_dec_safe(
                    l,
                    &ctx,
                    &comp,
                    comp.len() as c_int,
                    plain.len(),
                    plain.len() as c_int,
                    GUARD,
                );
                assert!(r < 0, "{ctx}: expected an error, got {r}");
            }
        }
        // And a match that reaches exactly `oend - 5` is legal, while one byte
        // further is not.
        for slack in 0..=6usize {
            let head: Vec<u8> = (0..16).map(|_| rng.byte()).collect();
            let mut blk = Blk::new();
            blk.seq(&head, 16, 20);
            let tail: Vec<u8> = (0..(5 + slack)).map(|_| rng.byte()).collect();
            let (comp, plain) = blk.finish(&tail);
            let ctx = format!("row104 exact slack={slack}");
            let r = diff_dec_safe(
                l,
                &ctx,
                &comp,
                comp.len() as c_int,
                plain.len(),
                plain.len() as c_int,
                GUARD,
            );
            assert_eq!(r, plain.len() as c_int, "{ctx}: expected {}, got {r}", plain.len());
        }
    }
}

// ===========================================================================
// Row 105 - the `_output_error` label: (int)(-(ip - src)) - 1
// ===========================================================================

#[test]
fn err_105_output_error_encodes_the_parse_offset() {
    let l = libs();
    unsafe {
        // Every hand-built block above has a hand-derived `ip`, so the exact
        // encoded offset is asserted here in one place.
        for b in all_bad_blocks() {
            let r = diff_dec_safe(l, b.tag, &b.bytes, b.csize, b.cap, b.cap as c_int, GUARD);
            assert_eq!(
                r, b.want,
                "row105 {}: `(int)(-(ip-src))-1` must be {}, got {r}",
                b.tag, b.want
            );
            // The encoding must always denote a position inside the declared
            // input: -1 - offset with 0 <= offset <= srcSize.
            let offset = -(r + 1);
            assert!(
                offset >= 0 && offset <= b.csize,
                "row105 {}: decoded offset {offset} outside [0, {}]",
                b.tag,
                b.csize
            );
        }
        // The parse offset really is the *first* byte not consumed: truncating
        // the input at that offset must give the identical error.
        for b in all_bad_blocks() {
            let offset = -(b.want + 1) as usize;
            if offset == 0 || offset > b.bytes.len() {
                continue;
            }
            let r = diff_dec_safe(
                l,
                &format!("row105 {} truncated-at-{offset}", b.tag),
                &b.bytes[..offset],
                offset as c_int,
                b.cap,
                b.cap as c_int,
                GUARD,
            );
            assert!(r < 0, "row105 {}: truncated input must still fail, got {r}", b.tag);
        }
    }
}

// ===========================================================================
// Rows 106..110 - LZ4_decompress_unsafe_generic (LZ4_decompress_fast*): five
// distinct `-1` sites
// ===========================================================================

#[test]
fn err_106_107_108_109_110_decompress_fast_errors() {
    let l = libs();
    let mut rng = Rng::new(106);
    unsafe {
        // NOTE: LZ4_decompress_fast has no input-bounds checking; `diff_dec_fast`
        // pads the input with `2*originalSize + 512` zero bytes so that every
        // read stays inside memory we own.  A *negative* originalSize is out of
        // contract (there is no check for it and `oend < ostart` would make the
        // literal copy write below `dst`), so it is never passed.

        // Row 106 (lz4.c:1898): literal run longer than the remaining output.
        for ll in 1..=14usize {
            for orig in 0..ll {
                let mut blk = vec![(ll as u8) << 4];
                blk.extend((0..ll).map(|_| rng.byte()));
                let ctx = format!("row106 ll={ll} orig={orig}");
                let r = diff_dec_fast(l, &ctx, &blk, orig as c_int);
                assert_eq!(r, -1, "{ctx}: expected -1, got {r}");
            }
        }
        // ... including long (255-chain) literal lengths.
        for ll in [15usize, 20, 270, 600] {
            let mut blk = vec![0xF0u8];
            push_ext(&mut blk, ll - 15);
            blk.extend((0..ll).map(|_| rng.byte()));
            for orig in [0usize, 1, ll - 1] {
                let ctx = format!("row106 long ll={ll} orig={orig}");
                let r = diff_dec_fast(l, &ctx, &blk, orig as c_int);
                assert_eq!(r, -1, "{ctx}: expected -1, got {r}");
            }
        }
        // Row 107 (lz4.c:1902-1907): the literals leave 0 < oend-op < MFLIMIT.
        for ll in 1..=14usize {
            for rest in 1..12usize {
                let orig = ll + rest;
                let mut blk = vec![(ll as u8) << 4];
                blk.extend((0..ll).map(|_| rng.byte()));
                let ctx = format!("row107 ll={ll} rest={rest}");
                let r = diff_dec_fast(l, &ctx, &blk, orig as c_int);
                assert_eq!(r, -1, "{ctx}: expected -1, got {r}");
            }
        }
        // ... and `op == oend` exactly is *not* an error: it ends the block.
        for ll in 0..=14usize {
            let mut blk = vec![(ll as u8) << 4];
            blk.extend((0..ll).map(|_| rng.byte()));
            let ctx = format!("row107 exact end ll={ll}");
            let r = diff_dec_fast(l, &ctx, &blk, ll as c_int);
            assert_eq!(r, (1 + ll) as c_int, "{ctx}: expected {}, got {r}", 1 + ll);
        }
        // Row 108 (lz4.c:1921): match length longer than the remaining output.
        for orig in [20usize, 30, 64, 100] {
            for ml_ext in [0usize, 255, 510] {
                let mut blk = vec![0x0Fu8, 0x01, 0x00];
                push_ext(&mut blk, ml_ext);
                let total_ml = 15 + ml_ext + 4;
                let ctx = format!("row108 orig={orig} ml={total_ml}");
                let r = diff_dec_fast(l, &ctx, &blk, orig as c_int);
                if total_ml > orig {
                    assert_eq!(r, -1, "{ctx}: expected -1, got {r}");
                }
            }
        }
        // Row 109 (lz4.c:1925-1928): offset before the start of history.
        for orig in [16usize, 20, 40, 100] {
            for off in [1usize, 2, 5, 100, 65535] {
                let blk = vec![0x00u8, (off & 0xFF) as u8, ((off >> 8) & 0xFF) as u8];
                let ctx = format!("row109 orig={orig} off={off}");
                let r = diff_dec_fast(l, &ctx, &blk, orig as c_int);
                // op == dst, prefixSize == 0, dictSize == 0, so *any* nonzero
                // offset is out of range.
                assert_eq!(r, -1, "{ctx}: expected -1, got {r}");
            }
        }
        // Row 110 (lz4.c:1956-1961): the match ends with oend-op < LASTLITERALS.
        for ll in 12..=20usize {
            for ml in 4..=8usize {
                for slack in 1..5usize {
                    let orig = ll + ml + slack;
                    let mut blk = vec![((ll.min(15) as u8) << 4) | ((ml - 4) as u8)];
                    if ll >= 15 {
                        blk[0] = 0xF0 | ((ml - 4) as u8);
                        push_ext(&mut blk, ll - 15);
                    }
                    blk.extend((0..ll).map(|_| rng.byte()));
                    blk.push((ll & 0xFF) as u8);
                    blk.push(((ll >> 8) & 0xFF) as u8);
                    let ctx = format!("row110 ll={ll} ml={ml} slack={slack}");
                    let r = diff_dec_fast(l, &ctx, &blk, orig as c_int);
                    assert_eq!(r, -1, "{ctx}: expected -1, got {r}");
                }
            }
        }
        // ... while a block that respects both restrictions -- every match
        // leaves >= LASTLITERALS bytes and the final literal run lands exactly
        // on `oend` -- decodes cleanly and returns the number of input bytes
        // consumed.
        for hl in [16usize, 20, 30] {
            for ml in [4usize, 20, 300] {
                let head: Vec<u8> = (0..hl).map(|_| rng.byte()).collect();
                let mut b = Blk::new();
                b.seq(&head, hl, ml);
                let tail: Vec<u8> = (0..20).map(|_| rng.byte()).collect();
                let (comp, plain) = b.finish(&tail);
                let ctx = format!("row110 well-formed hl={hl} ml={ml}");
                let r = diff_dec_fast(l, &ctx, &comp, plain.len() as c_int);
                assert_eq!(r, comp.len() as c_int, "{ctx}: expected {}, got {r}", comp.len());
            }
        }
    }
}

// ===========================================================================
// Rows 111, 112 - LZ4_decompress_safe_continue / _fast_continue: an inner
// decode returning <= 0 is forwarded unchanged and the prefix state is not
// advanced
// ===========================================================================

#[test]
fn err_111_112_decompress_continue_forwards_failures() {
    let l = libs();
    let mut rng = Rng::new(111);
    unsafe {
        let (sc, sr) = l.sym::<FnDecSafeContinue>("LZ4_decompress_safe_continue");
        let (fc, fr) = l.sym::<FnDecFastContinue>("LZ4_decompress_fast_continue");

        // Three good blocks laid out contiguously in one destination buffer.
        let blocks: Vec<(Vec<u8>, Vec<u8>)> = (0..3)
            .map(|_| {
                let sz = rng.range(200, 900);
                let plain = gen_real(&mut rng, Shape::TextLike, sz);
                let comp = diff_compress_bytes(
                    l,
                    "row111 seed",
                    &plain,
                    bound(l, plain.len() as c_int) as usize,
                    None,
                );
                assert!(comp.0 > 0);
                (comp.1[..comp.0 as usize].to_vec(), plain)
            })
            .collect();
        let total: usize = blocks.iter().map(|b| b.1.len()).sum();

        // Corrupt blocks whose failure is deterministic and independent of the
        // available history (so it cannot be "rescued" by the stream prefix).
        let mut chain = vec![0xF0u8];
        chain.extend(std::iter::repeat(0xFFu8).take(19));
        let bad: Vec<(Vec<u8>, c_int)> = vec![
            (vec![0xF0], 1),
            (vec![0x20, 0x11, 0x22, 0x33], 4),
            (chain, 20),
        ];

        // Row 111 (lz4.c:2639, :2650, :2657).
        for (bi, (bb, bsz)) in bad.iter().enumerate() {
            let sd = create_pair(l, "LZ4_createStreamDecode");
            let mut cb = dstbuf(total + 256);
            let mut rb = dstbuf(total + 256);
            let mut off = 0usize;
            for (i, (comp, plain)) in blocks.iter().enumerate() {
                let p = padded(comp, GUARD);
                let a = sc(
                    sd.c,
                    p.as_ptr() as *const c_char,
                    cb.as_mut_ptr().add(off) as *mut c_char,
                    comp.len() as c_int,
                    (total + 256 - off) as c_int,
                );
                let b = sr(
                    sd.r,
                    p.as_ptr() as *const c_char,
                    rb.as_mut_ptr().add(off) as *mut c_char,
                    comp.len() as c_int,
                    (total + 256 - off) as c_int,
                );
                let ctx = format!("row111 good bi={bi} i={i}");
                assert_eq!(a, b, "{ctx}: mismatch (C={a} Rust={b})");
                assert_eq!(a, plain.len() as c_int, "{ctx}: expected a clean decode");
                // ... then a corrupt block at the same position: the return
                // value is forwarded unchanged and must be identical.
                let pbad = padded(bb, GUARD);
                let a2 = sc(
                    sd.c,
                    pbad.as_ptr() as *const c_char,
                    cb.as_mut_ptr().add(off + plain.len()) as *mut c_char,
                    *bsz,
                    (total + 256 - off - plain.len()) as c_int,
                );
                let b2 = sr(
                    sd.r,
                    pbad.as_ptr() as *const c_char,
                    rb.as_mut_ptr().add(off + plain.len()) as *mut c_char,
                    *bsz,
                    (total + 256 - off - plain.len()) as c_int,
                );
                let ctx = format!("row111 bad bi={bi} i={i}");
                assert_eq!(a2, b2, "{ctx}: mismatch (C={a2} Rust={b2})");
                assert!(a2 <= 0, "{ctx}: expected <= 0, got {a2}");
                off += plain.len();
            }
            // The state was not advanced by the failures: a contiguous good
            // block still decodes as a continuation, identically in both.
            let (comp, plain) = &blocks[0];
            let p = padded(comp, GUARD);
            let a = sc(
                sd.c,
                p.as_ptr() as *const c_char,
                cb.as_mut_ptr().add(off) as *mut c_char,
                comp.len() as c_int,
                (total + 256 - off) as c_int,
            );
            let b = sr(
                sd.r,
                p.as_ptr() as *const c_char,
                rb.as_mut_ptr().add(off) as *mut c_char,
                comp.len() as c_int,
                (total + 256 - off) as c_int,
            );
            assert_eq!(a, b, "row111 resume bi={bi}: mismatch (C={a} Rust={b})");
            assert_eq!(a, plain.len() as c_int, "row111 resume bi={bi}: expected a clean decode");
            same_full_buffers(&format!("row111 buffers bi={bi}"), &cb, &rb);
            free_pair(l, "LZ4_freeStreamDecode", sd);
        }

        // Row 112 (lz4.c:2685, :2693, :2701).  LZ4_decompress_fast_continue has
        // no input bounds checking, so only *output*-side failures are provoked
        // (a declared originalSize that the block cannot fill), with generous
        // zero padding behind every block.
        for (bi, orig_lie) in [1i32, 7, 11].iter().enumerate() {
            let sd = create_pair(l, "LZ4_createStreamDecode");
            let mut cb = dstbuf(total + 4096);
            let mut rb = dstbuf(total + 4096);
            let mut off = 0usize;
            for (i, (comp, plain)) in blocks.iter().enumerate() {
                let p = padded(comp, 4 * plain.len() + 1024);
                let a = fc(
                    sd.c,
                    p.as_ptr() as *const c_char,
                    cb.as_mut_ptr().add(off) as *mut c_char,
                    plain.len() as c_int,
                );
                let b = fr(
                    sd.r,
                    p.as_ptr() as *const c_char,
                    rb.as_mut_ptr().add(off) as *mut c_char,
                    plain.len() as c_int,
                );
                let ctx = format!("row112 good bi={bi} i={i}");
                assert_eq!(a, b, "{ctx}: mismatch (C={a} Rust={b})");
                assert_eq!(a, comp.len() as c_int, "{ctx}: expected {}, got {a}", comp.len());
                // A too-small declared originalSize makes the inner decode fail.
                let bad_blk = vec![0xF0u8, 0xFF, 0xFF, 0xFF];
                let pbad = padded(&bad_blk, 4096);
                let a2 = fc(
                    sd.c,
                    pbad.as_ptr() as *const c_char,
                    cb.as_mut_ptr().add(off + plain.len()) as *mut c_char,
                    *orig_lie,
                );
                let b2 = fr(
                    sd.r,
                    pbad.as_ptr() as *const c_char,
                    rb.as_mut_ptr().add(off + plain.len()) as *mut c_char,
                    *orig_lie,
                );
                let ctx = format!("row112 bad bi={bi} i={i}");
                assert_eq!(a2, b2, "{ctx}: mismatch (C={a2} Rust={b2})");
                assert_eq!(a2, -1, "{ctx}: expected -1, got {a2}");
                off += plain.len();
            }
            same_full_buffers(&format!("row112 buffers bi={bi}"), &cb, &rb);
            free_pair(l, "LZ4_freeStreamDecode", sd);
        }
    }
}

// ===========================================================================
// Row 113 - LZ4_decompress_safe_partial*: targetOutputSize > dstCapacity is
// not an error, dstCapacity = MIN(targetOutputSize, dstCapacity)
// ===========================================================================

#[test]
fn err_113_partial_target_above_dst_capacity_is_clamped() {
    let l = libs();
    let mut rng = Rng::new(113);
    unsafe {
        for _ in 0..2000 {
            let n = rng.range(1, 3000);
            let sh = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
            let plain = gen_real(&mut rng, sh, n);
            let comp = diff_compress_bytes(
                l,
                "row113 seed",
                &plain,
                bound(l, n as c_int) as usize,
                None,
            );
            assert!(comp.0 > 0);
            let blk = comp.1[..comp.0 as usize].to_vec();
            let cap = rng.range(0, n + 8);
            // target > cap must behave exactly like target == cap.
            let extras = [
                cap as c_int,
                cap as c_int + 1,
                cap as c_int + 17,
                n as c_int,
                n as c_int + 1000,
                LZ4_MAX_INPUT_SIZE as c_int,
                c_int::MAX,
            ];
            let base = diff_dec_partial(
                l,
                "row113 base",
                &blk,
                blk.len() as c_int,
                cap as c_int,
                cap,
                cap as c_int,
                GUARD,
            );
            for &t in &extras {
                if t < cap as c_int {
                    continue;
                }
                let ctx = format!("row113 n={n} cap={cap} target={t}");
                let r = diff_dec_partial(
                    l,
                    &ctx,
                    &blk,
                    blk.len() as c_int,
                    t,
                    cap,
                    cap as c_int,
                    GUARD,
                );
                assert_eq!(
                    r, base,
                    "{ctx}: target > dstCapacity must clamp (expected {base}, got {r})"
                );
            }
            // And target < cap really does limit the output.
            for _ in 0..3 {
                let t = rng.below(cap + 1);
                let ctx = format!("row113 n={n} cap={cap} small target={t}");
                let r = diff_dec_partial(
                    l,
                    &ctx,
                    &blk,
                    blk.len() as c_int,
                    t as c_int,
                    cap,
                    cap as c_int,
                    GUARD,
                );
                assert!(
                    r <= t as c_int,
                    "{ctx}: partial produced {r} bytes for target {t}"
                );
            }
        }
    }
}

// ===========================================================================
// Rows 114, 115, 116 - internal assert-only contracts of
// LZ4_compress_generic_validated / LZ4_put/getIndexOnHash
// ===========================================================================

#[test]
fn err_114_115_116_internal_table_type_contracts() {
    // All three rows are *internal* contracts that the public entry points
    // maintain by construction; they are unreachable from outside the library:
    //
    //  * Row 114 `if (tableType == byU16) assert(inputSize < LZ4_64Klimit)`
    //    (lz4.c:981): every caller picks `byU16` only inside an explicit
    //    `if (inputSize < LZ4_64Klimit)` (lz4.c:1391, :1404, :1425, :1519).
    //  * Row 115 `if (tableType == byPtr) assert(dictDirective == noDict)`
    //    (lz4.c:982): `byPtr` is only selected by the
    //    `(sizeof(void*)==4) && ...` expression, which is statically false on
    //    this 64-bit target, and only on `noDict` paths.
    //  * Row 116 `assert(0)` in LZ4_putIndexOnHash / LZ4_getIndexOnHash for
    //    `clearedTable` / `byPtr` (lz4.c:813, :826, :866): the index helpers are
    //    only called with `byU16`/`byU32`.
    //
    // (And in this build `assert()` is a no-op anyway -- see the module docs.)
    // What is testable from outside is that the *observable* consequence of
    // those contracts holds: the byU16/byU32 table pivot at
    // LZ4_64Klimit == 65547 is placed identically by both libraries, so
    // compressed output matches byte-for-byte right across the boundary.
    let l = libs();
    let mut rng = Rng::new(114);
    unsafe {
        for n in [
            65_540usize, 65_541, 65_542, 65_543, 65_544, 65_545, 65_546, 65_547, 65_548,
            65_549, 65_550, 65_551,
        ] {
            for sh in ALL_SHAPES {
                let src = gen_real(&mut rng, sh, n);
                let b = bound(l, n as c_int) as usize;
                let ctx = format!("row114/115 pivot n={n} shape={sh:?}");
                let r = diff_compress_bytes(l, &ctx, &src, b, None).0;
                assert!(r > 0, "{ctx}: expected success, got {r}");
                // ... and with a tight destination, so the limitedOutput
                // variants of the same table types are covered too.
                let ctx = format!("row114/115 pivot tight n={n} shape={sh:?}");
                let r2 = diff_compress_bytes(l, &ctx, &src, (r as usize) - 1, None).0;
                assert!(r2 == 0 || r2 > 0, "{ctx}: nonsensical {r2}");
                // Row 116: LZ4_resetStream_fast leaves `tableType` in the
                // `clearedTable` state; a following compression must not touch
                // the index helpers with it, and both libraries must agree.
                let ss = sizeof_state(l);
                let (ic, ir) = l.sym::<FnInitStream>("LZ4_initStream");
                let (ec, er) = l.sym::<FnExtState>("LZ4_compress_fast_extState_fastReset");
                let mut cs = Scratch::new(ss);
                let mut rs = Scratch::new(ss);
                assert!(!ic(cs.ptr(), ss).is_null());
                assert!(!ir(rs.ptr(), ss).is_null());
                let mut cb = dstbuf(b);
                let mut rb = dstbuf(b);
                let a = ec(
                    cs.ptr(),
                    src.as_ptr() as *const c_char,
                    cb.as_mut_ptr() as *mut c_char,
                    n as c_int,
                    b as c_int,
                    1,
                );
                let bb = er(
                    rs.ptr(),
                    src.as_ptr() as *const c_char,
                    rb.as_mut_ptr() as *mut c_char,
                    n as c_int,
                    b as c_int,
                    1,
                );
                let ctx = format!("row116 clearedTable n={n} shape={sh:?}");
                same_int_and_bytes(&ctx, a, bb, &cb, &rb);
                same_full_buffers(&ctx, &cb, &rb);
                same_full_buffers(&format!("{ctx} [state]"), cs.bytes(ss), rs.bytes(ss));
            }
        }
    }
}

// ###########################################################################
// ## lz4hc.c (HC block API) -- ERRORS.md rows 117-159
// ###########################################################################

/// `LZ4_compress_HC_continue` on both libraries, own buffers each.
unsafe fn diff_hc_continue(
    l: &Pair,
    ctx: &str,
    h: &Handles,
    src: &[u8],
    ssz: c_int,
    cap: usize,
) -> (c_int, Vec<u8>) {
    let (fc, fr) = l.sym::<FnHCContinue>("LZ4_compress_HC_continue");
    let mut cb = dstbuf(cap);
    let mut rb = dstbuf(cap);
    let a = fc(
        h.c,
        src.as_ptr() as *const c_char,
        cb.as_mut_ptr() as *mut c_char,
        ssz,
        cap as c_int,
    );
    let b = fr(
        h.r,
        src.as_ptr() as *const c_char,
        rb.as_mut_ptr() as *mut c_char,
        ssz,
        cap as c_int,
    );
    same_int_and_bytes(ctx, a, b, &cb, &rb);
    same_full_buffers(ctx, &cb, &rb);
    (a, cb)
}

unsafe fn hc_stream_int(l: &Pair, name: &str, h: &Handles, v: c_int) {
    let (fc, fr) = l.sym::<FnStreamInt>(name);
    fc(h.c, v);
    fr(h.r, v);
}

// ===========================================================================
// Rows 117, 118 - LZ4HC_getCLevelParams: cLevel < 1 -> LZ4HC_CLEVEL_DEFAULT,
// cLevel > LZ4HC_CLEVEL_MAX -> clamped to 12.  Never rejected.
// ===========================================================================

#[test]
fn err_117_118_hc_level_clamped_below_one_and_above_max() {
    let l = libs();
    let mut rng = Rng::new(117);
    unsafe {
        // Keep the inputs small: level 12 is the exhaustive optimal parser.
        for &n in &[0usize, 1, 12, 13, 100, 1000, 4096] {
            for sh in ALL_SHAPES {
                let src = gen_real(&mut rng, sh, n);
                let cap = bound(l, n as c_int) as usize;
                let d9 = diff_hc(
                    l,
                    &format!("row117 ref9 n={n} {sh:?}"),
                    src.as_ptr() as *const c_char,
                    n as c_int,
                    cap,
                    cap as c_int,
                    LZ4HC_CLEVEL_DEFAULT,
                );
                let d12 = diff_hc(
                    l,
                    &format!("row118 ref12 n={n} {sh:?}"),
                    src.as_ptr() as *const c_char,
                    n as c_int,
                    cap,
                    cap as c_int,
                    LZ4HC_CLEVEL_MAX,
                );
                for &lvl in &WILD_LEVELS {
                    let ctx = format!("row117/118 n={n} {sh:?} level={lvl}");
                    let g = diff_hc(
                        l,
                        &ctx,
                        src.as_ptr() as *const c_char,
                        n as c_int,
                        cap,
                        cap as c_int,
                        lvl,
                    );
                    assert!(g.0 > 0, "{ctx}: a level must never be rejected, got {}", g.0);
                    if lvl < 1 {
                        // Row 117: replaced by LZ4HC_CLEVEL_DEFAULT (9).
                        assert_eq!(g.0, d9.0, "{ctx}: level < 1 must behave as level 9");
                        same_full_buffers(&format!("{ctx} == level 9"), &d9.1, &g.1);
                    }
                    if lvl > LZ4HC_CLEVEL_MAX {
                        // Row 118: clamped to LZ4HC_CLEVEL_MAX (12).
                        assert_eq!(g.0, d12.0, "{ctx}: level > 12 must behave as level 12");
                        same_full_buffers(&format!("{ctx} == level 12"), &d12.1, &g.1);
                    }
                }
                // Level 1 is *not* equivalent to level 9: k_clTable[1] is the
                // lz4mid strategy (lz4hc.c:93).  Pin that the clamping is only
                // applied below 1, not below LZ4HC_CLEVEL_MIN.
                let d1 = diff_hc(
                    l,
                    &format!("row117 level1 n={n} {sh:?}"),
                    src.as_ptr() as *const c_char,
                    n as c_int,
                    cap,
                    cap as c_int,
                    1,
                );
                let d2 = diff_hc(
                    l,
                    &format!("row117 level2 n={n} {sh:?}"),
                    src.as_ptr() as *const c_char,
                    n as c_int,
                    cap,
                    cap as c_int,
                    LZ4HC_CLEVEL_MIN,
                );
                assert_eq!(
                    d1.0, d2.0,
                    "row117 n={n} {sh:?}: levels 1 and 2 share k_clTable entries"
                );
                same_full_buffers(
                    &format!("row117 n={n} {sh:?} level 1 == level 2"),
                    &d1.1,
                    &d2.1,
                );
            }
        }
    }
}

// ===========================================================================
// Rows 119, 120 - LZ4_setCompressionLevel: < 1 -> 9, > 12 -> 12
// ===========================================================================

#[test]
fn err_119_120_set_compression_level_clamped() {
    let l = libs();
    let mut rng = Rng::new(119);
    unsafe {
        for &n in &[13usize, 500, 4096] {
            let src = gen_real(&mut rng, Shape::TextLike, n);
            let cap = bound(l, n as c_int) as usize;
            // References produced through the same code path (a fresh stream
            // with an explicit level) so only the clamping differs.
            let mut refs: BTreeMap<c_int, Vec<u8>> = BTreeMap::new();
            for &lvl in &[LZ4HC_CLEVEL_DEFAULT, LZ4HC_CLEVEL_MAX] {
                let h = create_pair(l, "LZ4_createStreamHC");
                hc_stream_int(l, "LZ4_setCompressionLevel", &h, lvl);
                let ctx = format!("row119 ref n={n} lvl={lvl}");
                let (r, cb) = diff_hc_continue(l, &ctx, &h, &src, n as c_int, cap);
                assert!(r > 0, "{ctx}: expected success");
                refs.insert(lvl, cb[..r as usize].to_vec());
                free_pair(l, "LZ4_freeStreamHC", h);
            }
            for &lvl in &WILD_LEVELS {
                let h = create_pair(l, "LZ4_createStreamHC");
                hc_stream_int(l, "LZ4_setCompressionLevel", &h, lvl);
                let ctx = format!("row119/120 n={n} lvl={lvl}");
                let (r, cb) = diff_hc_continue(l, &ctx, &h, &src, n as c_int, cap);
                assert!(r > 0, "{ctx}: level must never be rejected, got {r}");
                if lvl < 1 {
                    assert_eq!(
                        &cb[..r as usize],
                        &refs[&LZ4HC_CLEVEL_DEFAULT][..],
                        "{ctx}: level < 1 must be LZ4HC_CLEVEL_DEFAULT"
                    );
                }
                if lvl > LZ4HC_CLEVEL_MAX {
                    assert_eq!(
                        &cb[..r as usize],
                        &refs[&LZ4HC_CLEVEL_MAX][..],
                        "{ctx}: level > 12 must be LZ4HC_CLEVEL_MAX"
                    );
                }
                free_pair(l, "LZ4_freeStreamHC", h);
            }
            // LZ4_resetStreamHC / _fast route the level through the very same
            // LZ4_setCompressionLevel, so they clamp identically.
            for name in ["LZ4_resetStreamHC", "LZ4_resetStreamHC_fast"] {
                for &lvl in &WILD_LEVELS {
                    let h = create_pair(l, "LZ4_createStreamHC");
                    hc_stream_int(l, name, &h, lvl);
                    let ctx = format!("row119/120 {name} n={n} lvl={lvl}");
                    let (r, cb) = diff_hc_continue(l, &ctx, &h, &src, n as c_int, cap);
                    assert!(r > 0, "{ctx}: expected success, got {r}");
                    if lvl < 1 {
                        assert_eq!(
                            &cb[..r as usize],
                            &refs[&LZ4HC_CLEVEL_DEFAULT][..],
                            "{ctx}: level < 1 must be LZ4HC_CLEVEL_DEFAULT"
                        );
                    }
                    if lvl > LZ4HC_CLEVEL_MAX {
                        assert_eq!(
                            &cb[..r as usize],
                            &refs[&LZ4HC_CLEVEL_MAX][..],
                            "{ctx}: level > 12 must be LZ4HC_CLEVEL_MAX"
                        );
                    }
                    free_pair(l, "LZ4_freeStreamHC", h);
                }
            }
        }
    }
}

// ===========================================================================
// Row 121 - LZ4HC_compress_generic_internal: fillOutput with dstCapacity < 1 -> 0
// ===========================================================================

#[test]
fn err_121_hc_fill_output_dst_capacity_below_one() {
    let l = libs();
    let mut rng = Rng::new(121);
    unsafe {
        let hss = sizeof_state_hc(l);
        let (fc, fr) = l.sym::<FnDestSizeExtState>("LZ4_compress_HC_destSize");
        let (cc, cr) = l.sym::<FnHCContinueDestSize>("LZ4_compress_HC_continue_destSize");
        for &n in &[0usize, 1, 13, 500, 4096] {
            let src = gen_real(&mut rng, Shape::TextLike, n);
            for &lvl in &[1i32, 2, 3, 9, 10, 12, 0, -5, 100] {
                for &target in &[0i32, -1, -1000, c_int::MIN] {
                    // fillOutput is only reachable through the *_destSize entry
                    // points (lz4hc.c:1541, :1732).
                    let mut cs = Scratch::new(hss);
                    let mut rs = Scratch::new(hss);
                    let mut cb = dstbuf(64);
                    let mut rb = dstbuf(64);
                    let mut cn = n as c_int;
                    let mut rn = n as c_int;
                    let a = fc(
                        cs.ptr(),
                        src.as_ptr() as *const c_char,
                        cb.as_mut_ptr() as *mut c_char,
                        &mut cn,
                        target,
                        lvl,
                    );
                    let b = fr(
                        rs.ptr(),
                        src.as_ptr() as *const c_char,
                        rb.as_mut_ptr() as *mut c_char,
                        &mut rn,
                        target,
                        lvl,
                    );
                    let ctx = format!("row121 destSize n={n} lvl={lvl} target={target}");
                    same_int_and_bytes(&ctx, a, b, &cb, &rb);
                    same_full_buffers(&ctx, &cb, &rb);
                    assert_eq!(cn, rn, "{ctx}: *srcSizePtr mismatch (C={cn} Rust={rn})");
                    assert_eq!(a, 0, "{ctx}: expected 0, got {a}");

                    let h = create_pair(l, "LZ4_createStreamHC");
                    hc_stream_int(l, "LZ4_setCompressionLevel", &h, lvl);
                    let mut cb = dstbuf(64);
                    let mut rb = dstbuf(64);
                    let mut cn = n as c_int;
                    let mut rn = n as c_int;
                    let a = cc(
                        h.c,
                        src.as_ptr() as *const c_char,
                        cb.as_mut_ptr() as *mut c_char,
                        &mut cn,
                        target,
                    );
                    let b = cr(
                        h.r,
                        src.as_ptr() as *const c_char,
                        rb.as_mut_ptr() as *mut c_char,
                        &mut rn,
                        target,
                    );
                    let ctx = format!("row121 continue_destSize n={n} lvl={lvl} target={target}");
                    same_int_and_bytes(&ctx, a, b, &cb, &rb);
                    same_full_buffers(&ctx, &cb, &rb);
                    assert_eq!(cn, rn, "{ctx}: *srcSizePtr mismatch");
                    assert_eq!(a, 0, "{ctx}: expected 0, got {a}");
                    free_pair(l, "LZ4_freeStreamHC", h);
                }
            }
        }
    }
}

// ===========================================================================
// Row 122 - LZ4HC_compress_generic_internal:
// (U32)*srcSizePtr > (U32)LZ4_MAX_INPUT_SIZE -> 0
// ===========================================================================

#[test]
fn err_122_hc_srcsize_too_large_or_negative() {
    let l = libs();
    let mut rng = Rng::new(122);
    let src = gen_real(&mut rng, Shape::TextLike, 4096);
    unsafe {
        // lz4hc.c:1389 runs before `src` is touched (only pointer bookkeeping
        // happens in LZ4HC_init_internal), so a lying oversized/negative
        // srcSize is safe.  LZ4_MAX_INPUT_SIZE itself is *not* rejected and is
        // therefore never used as a lie.
        for &ssz in &[
            -1,
            -4096,
            c_int::MIN,
            LZ4_MAX_INPUT_SIZE as c_int + 1,
            LZ4_MAX_INPUT_SIZE as c_int + 2,
            c_int::MAX,
        ] {
            for &lvl in &[1i32, 2, 3, 9, 12, 0, -1, 13] {
                for &cap in &[0i32, 1, 16, 8192] {
                    let ctx = format!("row122 HC ssz={ssz} lvl={lvl} cap={cap}");
                    let (r, _) = diff_hc(
                        l,
                        &ctx,
                        src.as_ptr() as *const c_char,
                        ssz,
                        8192,
                        cap,
                        lvl,
                    );
                    assert_eq!(r, 0, "{ctx}: expected 0, got {r}");
                }
            }
        }
        // Same through the streaming entry point.
        let (cc, cr) = l.sym::<FnHCContinue>("LZ4_compress_HC_continue");
        for &ssz in &[-1, c_int::MIN, LZ4_MAX_INPUT_SIZE as c_int + 1, c_int::MAX] {
            for &lvl in &[2i32, 9, 12] {
                let h = create_pair(l, "LZ4_createStreamHC");
                hc_stream_int(l, "LZ4_setCompressionLevel", &h, lvl);
                let mut cb = dstbuf(8192);
                let mut rb = dstbuf(8192);
                let a = cc(
                    h.c,
                    src.as_ptr() as *const c_char,
                    cb.as_mut_ptr() as *mut c_char,
                    ssz,
                    8192,
                );
                let b = cr(
                    h.r,
                    src.as_ptr() as *const c_char,
                    rb.as_mut_ptr() as *mut c_char,
                    ssz,
                    8192,
                );
                let ctx = format!("row122 continue ssz={ssz} lvl={lvl}");
                same_int_and_bytes(&ctx, a, b, &cb, &rb);
                same_full_buffers(&ctx, &cb, &rb);
                assert_eq!(a, 0, "{ctx}: expected 0, got {a}");
                free_pair(l, "LZ4_freeStreamHC", h);
            }
        }
    }
}

// ===========================================================================
// Row 123 - inner strategy returned <= 0: ctx->dirty is latched and the stream
// must be re-initialised (LZ4_resetStreamHC_fast)
// ===========================================================================

#[test]
fn err_123_hc_dirty_flag_latched_after_failure() {
    let l = libs();
    let mut rng = Rng::new(123);
    unsafe {
        for &lvl in &[1i32, 2, 3, 9, 10, 12] {
            for &n in &[500usize, 4096] {
                let a1 = gen_real(&mut rng, Shape::TextLike, n);
                let a2 = gen_real(&mut rng, Shape::Compressible, n);
                let cap = bound(l, n as c_int) as usize;

                // (a) The reference: a clean stream compressing a2 as its first
                // block.
                let refbytes = {
                    let h = create_pair(l, "LZ4_createStreamHC");
                    hc_stream_int(l, "LZ4_setCompressionLevel", &h, lvl);
                    let ctx = format!("row123 ref lvl={lvl} n={n}");
                    let (r, cb) = diff_hc_continue(l, &ctx, &h, &a2, n as c_int, cap);
                    assert!(r > 0, "{ctx}: expected success");
                    free_pair(l, "LZ4_freeStreamHC", h);
                    cb[..r as usize].to_vec()
                };

                // (b) A stream whose first block failed (dstCapacity 0) latches
                // ctx->dirty; LZ4_resetStreamHC_fast then takes the
                // LZ4_initStreamHC branch (lz4hc.c:1598-1601), so the stream is
                // fully clean again and reproduces the reference exactly.
                let h = create_pair(l, "LZ4_createStreamHC");
                hc_stream_int(l, "LZ4_setCompressionLevel", &h, lvl);
                let ctx = format!("row123 fail lvl={lvl} n={n}");
                let (r, _) = diff_hc_continue(l, &ctx, &h, &a1, n as c_int, 0);
                assert_eq!(r, 0, "{ctx}: expected the failure that latches dirty");
                hc_stream_int(l, "LZ4_resetStreamHC_fast", &h, lvl);
                let ctx = format!("row123 after reset lvl={lvl} n={n}");
                let (r2, cb2) = diff_hc_continue(l, &ctx, &h, &a2, n as c_int, cap);
                assert!(r2 > 0, "{ctx}: expected success");
                assert_eq!(
                    &cb2[..r2 as usize],
                    &refbytes[..],
                    "{ctx}: a dirty stream reset with _fast must behave like a fresh one"
                );
                free_pair(l, "LZ4_freeStreamHC", h);

                // (c) Without the reset the stream is still usable but keeps its
                // (dirty) history; the only contract here is that C and Rust
                // agree exactly, which diff_hc_continue asserts.
                let h = create_pair(l, "LZ4_createStreamHC");
                hc_stream_int(l, "LZ4_setCompressionLevel", &h, lvl);
                let ctx = format!("row123 no-reset fail lvl={lvl} n={n}");
                let (r, _) = diff_hc_continue(l, &ctx, &h, &a1, n as c_int, 0);
                assert_eq!(r, 0, "{ctx}: expected failure");
                for k in 0..3 {
                    let ctx = format!("row123 no-reset continue k={k} lvl={lvl} n={n}");
                    diff_hc_continue(l, &ctx, &h, &a2, n as c_int, cap);
                }
                free_pair(l, "LZ4_freeStreamHC", h);
            }
        }
    }
}

// ===========================================================================
// Rows 124, 125, 126 - LZ4MID_compress input sanitization
// ===========================================================================

#[test]
fn err_124_125_126_lz4mid_size_sanitization() {
    let l = libs();
    let mut rng = Rng::new(124);
    let src = gen_real(&mut rng, Shape::Compressible, 4096);
    unsafe {
        // Rows 124 (`*srcSizePtr < 0`, lz4hc.c:559) and 126
        // (`*srcSizePtr > LZ4_MAX_INPUT_SIZE`, lz4hc.c:561-563) are
        // *unreachable* through the public API: LZ4HC_compress_generic_internal
        // already rejects both at lz4hc.c:1389 with the single unsigned
        // comparison `(U32)*srcSizePtr > (U32)LZ4_MAX_INPUT_SIZE`, which
        // subsumes them.  The externally visible result is the same `0`, and it
        // is asserted here for the lz4mid levels specifically (1 and 2).
        for &lvl in &[1i32, 2] {
            for &ssz in &[-1, -4096, c_int::MIN, LZ4_MAX_INPUT_SIZE as c_int + 1, c_int::MAX] {
                let ctx = format!("row124/126 lvl={lvl} ssz={ssz}");
                let (r, _) = diff_hc(
                    l,
                    &ctx,
                    src.as_ptr() as *const c_char,
                    ssz,
                    8192,
                    8192,
                    lvl,
                );
                assert_eq!(r, 0, "{ctx}: expected 0, got {r}");
            }
        }
        // Row 125 (`maxOutputSize < 0`, lz4hc.c:560) IS reachable: nothing above
        // LZ4MID_compress inspects the sign of dstCapacity when the limit is
        // `limitedOutput`, and `dstCapacity < LZ4_compressBound(srcSize)`
        // selects exactly that (lz4hc.c:1507).
        for &lvl in &[1i32, 2] {
            for &n in &[0usize, 1, 13, 100, 4096] {
                for &cap in &[-1i32, -2, -1000, c_int::MIN] {
                    let s = gen_real(&mut rng, Shape::Compressible, n);
                    let ctx = format!("row125 lvl={lvl} n={n} cap={cap}");
                    let (r, _) = diff_hc(
                        l,
                        &ctx,
                        s.as_ptr() as *const c_char,
                        n as c_int,
                        64,
                        cap,
                        lvl,
                    );
                    assert_eq!(r, 0, "{ctx}: expected 0, got {r}");
                }
            }
        }
        // The same negative-dstCapacity rejection for the hashChain (3..9) and
        // optimal (10..12) strategies, which reach it through their own
        // `_last_literals` / `_dest_overflow` handlers instead.
        for &lvl in &[3i32, 9, 10, 12] {
            for &n in &[0usize, 13, 1000] {
                for &cap in &[-1i32, c_int::MIN] {
                    let s = gen_real(&mut rng, Shape::Compressible, n);
                    let ctx = format!("row125 other-strat lvl={lvl} n={n} cap={cap}");
                    let (r, _) = diff_hc(
                        l,
                        &ctx,
                        s.as_ptr() as *const c_char,
                        n as c_int,
                        64,
                        cap,
                        lvl,
                    );
                    assert_eq!(r, 0, "{ctx}: expected 0, got {r}");
                }
            }
        }
    }
}

// ===========================================================================
// Rows 127, 128 - LZ4MID_compress: src == NULL with *srcSizePtr != 0, and
// dst == NULL with maxOutputSize != 0 (assert-only contracts)
// ===========================================================================

#[test]
fn err_127_128_lz4mid_null_src_and_dst_contracts() {
    let l = libs();
    unsafe {
        // ASSERT-ONLY CONTRACTS (lz4hc.c:557-558).  The asserts are no-ops in
        // this build, so passing `src == NULL` with a nonzero size would let the
        // match finder dereference NULL (a hard SIGSEGV, not an abort) and
        // passing `dst == NULL` with a nonzero capacity would let the literal
        // copy write to NULL.  Neither is provoked.
        //
        // The reachable in-contract side is exactly the sizes the asserts
        // permit: `*srcSizePtr == 0` with `src == NULL`, and
        // `maxOutputSize == 0` with `dst == NULL`.  Both are exercised for the
        // lz4mid levels, where `if (*srcSizePtr < LZ4_minLength) goto
        // _lz4mid_last_literals` (lz4hc.c:565) keeps the parser away from the
        // pointers entirely.
        let (fc, fr) = l.sym::<FnHC>("LZ4_compress_HC");
        for &lvl in &[1i32, 2] {
            // dst == NULL with dstCapacity 0: `oend == op == NULL`, so the
            // last-literals check `op + totalSize > oend` fails immediately.
            let a = fc(std::ptr::null(), std::ptr::null_mut(), 0, 0, lvl);
            let b = fr(std::ptr::null(), std::ptr::null_mut(), 0, 0, lvl);
            assert_eq!(a, b, "row127/128 lvl={lvl}: mismatch (C={a} Rust={b})");
            assert_eq!(a, 0, "row127/128 lvl={lvl}: expected 0, got {a}");
            // src == NULL with srcSize 0 but a real destination: the 1-byte
            // "empty" literal token is emitted.
            let mut cb = dstbuf(64);
            let mut rb = dstbuf(64);
            let a = fc(
                std::ptr::null(),
                cb.as_mut_ptr() as *mut c_char,
                0,
                64,
                lvl,
            );
            let b = fr(
                std::ptr::null(),
                rb.as_mut_ptr() as *mut c_char,
                0,
                64,
                lvl,
            );
            let ctx = format!("row127 NULL src size 0 lvl={lvl}");
            same_int_and_bytes(&ctx, a, b, &cb, &rb);
            same_full_buffers(&ctx, &cb, &rb);
            assert_eq!(a, 1, "{ctx}: expected the 1-byte empty block, got {a}");
        }
        // For levels >= LZ4HC_CLEVEL_OPT_MIN the optimal parser has no
        // `srcSize < LZ4_minLength` guard: `mflimit = iend - MFLIMIT` wraps for
        // `src == NULL`, `while (ip <= mflimit)` becomes true and
        // LZ4HC_FindLongerMatch dereferences NULL.  A NULL src is therefore
        // NEVER passed at those levels; a real zero-length buffer is used
        // instead (see `err_generic_null_src_and_dst`).
    }
}

// ===========================================================================
// Row 129 - LZ4MID_compress `_lz4mid_last_literals`: limitedOutput and the
// final literal run does not fit -> 0
// ===========================================================================

#[test]
fn err_129_lz4mid_last_literals_do_not_fit() {
    let l = libs();
    let mut rng = Rng::new(129);
    unsafe {
        // Inputs below LZ4_minLength == 13 jump straight to the last-literals
        // encoder, so the block is exactly `1 + n` bytes and any smaller
        // capacity must return 0 -- for both lz4mid levels.
        for &lvl in &[1i32, 2] {
            for n in 0..13usize {
                let src = gen_real(&mut rng, Shape::Incompressible, n);
                for cap in 0..=(n + 2) {
                    let ctx = format!("row129 lvl={lvl} n={n} cap={cap}");
                    let (r, _) = diff_hc(
                        l,
                        &ctx,
                        src.as_ptr() as *const c_char,
                        n as c_int,
                        cap.max(1),
                        cap as c_int,
                        lvl,
                    );
                    let want = if cap >= n + 1 { (n + 1) as c_int } else { 0 };
                    assert_eq!(r, want, "{ctx}: expected {want}, got {r}");
                }
            }
            // Incompressible inputs above LZ4_minLength also end in the same
            // handler after the main loop finds nothing.
            for &n in &[13usize, 20, 64, 300, 2000] {
                let src = gen_real(&mut rng, Shape::Incompressible, n);
                let b = bound(l, n as c_int) as usize;
                let mut first_ok = usize::MAX;
                for cap in 0..=b {
                    let ctx = format!("row129 sweep lvl={lvl} n={n} cap={cap}");
                    let (r, _) = diff_hc(
                        l,
                        &ctx,
                        src.as_ptr() as *const c_char,
                        n as c_int,
                        cap.max(1),
                        cap as c_int,
                        lvl,
                    );
                    if r > 0 && cap < first_ok {
                        first_ok = cap;
                    }
                    if cap >= b {
                        assert!(r > 0, "{ctx}: full bound must succeed");
                    }
                }
                assert!(first_ok != usize::MAX, "row129 lvl={lvl} n={n}: vacuous sweep");
            }
        }
    }
}

// ===========================================================================
// Row 130 - LZ4MID_compress `_lz4mid_dest_overflow` with limit != fillOutput -> 0
// ===========================================================================

#[test]
fn err_130_lz4mid_dest_overflow_limited_output() {
    let l = libs();
    let mut rng = Rng::new(130);
    unsafe {
        // Highly compressible inputs make LZ4HC_encodeSequence report a buffer
        // issue in the middle of the main loop, which jumps to
        // `_lz4mid_dest_overflow`; with `limit == limitedOutput` that returns 0
        // (lz4hc.c:770-772).
        for &lvl in &[1i32, 2] {
            for &n in &[64usize, 300, 2000, 20_000] {
                for sh in [Shape::Compressible, Shape::Periodic, Shape::Degenerate, Shape::TextLike]
                {
                    let src = gen_real(&mut rng, sh, n);
                    let b = bound(l, n as c_int) as usize;
                    let mut first_ok = usize::MAX;
                    let mut caps: Vec<usize> = (0..48).collect();
                    caps.extend((0..64).map(|_| rng.below(b + 1)));
                    caps.push(b);
                    for cap in caps {
                        let ctx = format!("row130 lvl={lvl} n={n} {sh:?} cap={cap}");
                        let (r, _) = diff_hc(
                            l,
                            &ctx,
                            src.as_ptr() as *const c_char,
                            n as c_int,
                            cap.max(1),
                            cap as c_int,
                            lvl,
                        );
                        assert!(
                            r == 0 || (r > 0 && r as usize <= cap),
                            "{ctx}: nonsensical return {r}"
                        );
                        if r > 0 && cap < first_ok {
                            first_ok = cap;
                        }
                    }
                    assert!(
                        first_ok != usize::MAX,
                        "row130 lvl={lvl} n={n} {sh:?}: vacuous sweep"
                    );
                }
            }
        }
    }
}

// ===========================================================================
// Rows 131, 132 - LZ4HC_encodeSequence returns 1 (buffer issue) for the
// literals / for the match length
// ===========================================================================

#[test]
fn err_131_132_hc_encode_sequence_buffer_issues() {
    let l = libs();
    let mut rng = Rng::new(131);
    unsafe {
        // `LZ4HC_encodeSequence` returning 1 is not directly observable: every
        // caller turns it into its own `_dest_overflow` handler, which -- for
        // `limit != fillOutput` -- yields 0.  What *is* checkable exactly is the
        // capacity at which each strategy flips from 0 to a successful size, and
        // that must be identical in C and Rust for every level.
        //
        // Row 131 (literals do not fit, lz4hc.c:304-308) dominates for
        // incompressible/text data; row 132 (match length does not fit,
        // lz4hc.c:330-333) dominates for long-match data.
        for &lvl in &[1i32, 3, 6, 9, 10, 12] {
            for (tag, sh) in [
                ("row131", Shape::Incompressible),
                ("row131", Shape::TextLike),
                ("row132", Shape::Periodic),
                ("row132", Shape::Compressible),
            ] {
                for &n in &[64usize, 400, 3000] {
                    let src = gen_real(&mut rng, sh, n);
                    let b = bound(l, n as c_int) as usize;
                    let mut boundary: Option<usize> = None;
                    for cap in 0..=b {
                        let ctx = format!("{tag} lvl={lvl} n={n} {sh:?} cap={cap}");
                        let (r, _) = diff_hc(
                            l,
                            &ctx,
                            src.as_ptr() as *const c_char,
                            n as c_int,
                            cap.max(1),
                            cap as c_int,
                            lvl,
                        );
                        if r > 0 && boundary.is_none() {
                            boundary = Some(cap);
                        }
                        if cap >= b {
                            assert!(r > 0, "{ctx}: full bound must succeed");
                        }
                    }
                    assert!(
                        boundary.is_some(),
                        "{tag} lvl={lvl} n={n} {sh:?}: vacuous sweep"
                    );
                }
            }
        }
    }
}

// ===========================================================================
// Rows 133, 134 - LZ4HC_encodeSequence assert-only contracts:
// 0 < offset <= LZ4_DISTANCE_MAX and matchLength >= MINMATCH
// ===========================================================================

#[test]
fn err_133_134_hc_encode_sequence_assert_contracts() {
    let l = libs();
    let mut rng = Rng::new(133);
    unsafe {
        // ASSERT-ONLY CONTRACTS (lz4hc.c:324-325 and :329).  They are internal
        // invariants of the match finders, not input validation: there is no
        // argument to LZ4_compress_HC* that can make the HC parser propose an
        // offset of 0, an offset above LZ4_DISTANCE_MAX == 65535, or a match
        // shorter than MINMATCH.  With the asserts compiled out, violating them
        // would corrupt the emitted block rather than abort.  They are therefore
        // not provoked.
        //
        // The reachable, externally checkable consequence is that every block
        // the HC encoder emits satisfies those invariants -- which is exactly
        // what a successful round-trip through the *other* library's decoder
        // proves, since the decoder rejects offset-0 / out-of-range matches.
        let (dc, dr) = l.sym::<FnDecompressSafe>("LZ4_decompress_safe");
        for &lvl in &[1i32, 2, 3, 9, 10, 12] {
            for &n in &[13usize, 100, 4096, 70_000] {
                for sh in ALL_SHAPES {
                    let src = gen_real(&mut rng, sh, n);
                    let cap = bound(l, n as c_int) as usize;
                    let ctx = format!("row133/134 lvl={lvl} n={n} {sh:?}");
                    let (r, cb) = diff_hc(
                        l,
                        &ctx,
                        src.as_ptr() as *const c_char,
                        n as c_int,
                        cap,
                        cap as c_int,
                        lvl,
                    );
                    assert!(r > 0, "{ctx}: expected success");
                    let p = padded(&cb[..r as usize], GUARD);
                    let mut o1 = dstbuf(n.max(1));
                    let mut o2 = dstbuf(n.max(1));
                    let x = dc(
                        p.as_ptr() as *const c_char,
                        o1.as_mut_ptr() as *mut c_char,
                        r,
                        n as c_int,
                    );
                    let y = dr(
                        p.as_ptr() as *const c_char,
                        o2.as_mut_ptr() as *mut c_char,
                        r,
                        n as c_int,
                    );
                    assert_eq!(x, n as c_int, "{ctx}: C decoder rejected the HC block ({x})");
                    assert_eq!(y, n as c_int, "{ctx}: Rust decoder rejected the HC block ({y})");
                    assert_eq!(&o1[..n], &src[..], "{ctx}: C decode mismatch");
                    assert_eq!(&o2[..n], &src[..], "{ctx}: Rust decode mismatch");
                }
            }
        }
    }
}

// ===========================================================================
// Rows 135, 136 - LZ4HC_compress_hashChain `_last_literals` / `_dest_overflow`
// ===========================================================================

#[test]
fn err_135_136_hashchain_last_literals_and_dest_overflow() {
    let l = libs();
    let mut rng = Rng::new(135);
    unsafe {
        // Levels 3..9 select the hashChain strategy (k_clTable, lz4hc.c:96-103).
        for &lvl in &[3i32, 4, 5, 6, 7, 8, 9] {
            for &n in &[0usize, 1, 12, 13, 64, 500, 4000] {
                for sh in ALL_SHAPES {
                    let src = gen_real(&mut rng, sh, n);
                    let b = bound(l, n as c_int) as usize;
                    let caps: Vec<usize> = if b <= 300 {
                        (0..=b).collect()
                    } else {
                        let mut v: Vec<usize> = (0..40).collect();
                        v.extend((0..40).map(|_| rng.below(b + 1)));
                        v.push(b);
                        v
                    };
                    let mut first_ok = usize::MAX;
                    for cap in caps {
                        let ctx = format!("row135/136 lvl={lvl} n={n} {sh:?} cap={cap}");
                        let (r, _) = diff_hc(
                            l,
                            &ctx,
                            src.as_ptr() as *const c_char,
                            n as c_int,
                            cap.max(1),
                            cap as c_int,
                            lvl,
                        );
                        assert!(
                            r == 0 || (r > 0 && r as usize <= cap),
                            "{ctx}: nonsensical return {r}"
                        );
                        if r > 0 && cap < first_ok {
                            first_ok = cap;
                        }
                        if cap >= b {
                            assert!(r > 0, "{ctx}: full bound must succeed");
                        }
                    }
                    assert!(
                        first_ok != usize::MAX,
                        "row135/136 lvl={lvl} n={n} {sh:?}: vacuous sweep"
                    );
                }
            }
        }
    }
}

// ===========================================================================
// Row 137 - LZ4HC_compress_optimal: LZ4HC_HEAPMODE == 1 and ALLOC failure -> 0
// ===========================================================================

#[test]
fn err_137_optimal_heapmode_alloc_failure_unreachable() {
    // lz4hc.c:47-48 leaves `LZ4HC_HEAPMODE` at its default of 1, so
    // lz4hc.c:1856-1857 (`if (opt == NULL) goto _return_label;`) *is* compiled
    // in -- but it can only fire when malloc fails, which cannot be provoked
    // through the public ABI.  Pin the success side: with a full
    // LZ4_compressBound destination the optimal parser (levels 10..12) never
    // returns the `retval = 0` sentinel.
    let l = libs();
    let mut rng = Rng::new(137);
    unsafe {
        for &lvl in &[10i32, 11, 12] {
            for &n in &[1usize, 13, 100, 2000] {
                for sh in ALL_SHAPES {
                    let src = gen_real(&mut rng, sh, n);
                    let cap = bound(l, n as c_int) as usize;
                    let ctx = format!("row137 lvl={lvl} n={n} {sh:?}");
                    let (r, _) = diff_hc(
                        l,
                        &ctx,
                        src.as_ptr() as *const c_char,
                        n as c_int,
                        cap,
                        cap as c_int,
                        lvl,
                    );
                    assert!(r > 0, "{ctx}: expected success, got {r}");
                }
            }
        }
    }
}

// ===========================================================================
// Rows 138, 139 - LZ4HC_compress_optimal `_last_literals` / `_dest_overflow`
// ===========================================================================

#[test]
fn err_138_139_optimal_last_literals_and_dest_overflow() {
    let l = libs();
    let mut rng = Rng::new(138);
    unsafe {
        // Levels 10..12 select the optimal parser.  Inputs are kept small
        // because level 12 is the exhaustive (nbSearches == 16384) variant.
        for &lvl in &[10i32, 11, 12] {
            for &n in &[0usize, 1, 12, 13, 64, 400, 1500] {
                for sh in ALL_SHAPES {
                    let src = gen_real(&mut rng, sh, n);
                    let b = bound(l, n as c_int) as usize;
                    let caps: Vec<usize> = if b <= 120 {
                        (0..=b).collect()
                    } else {
                        let mut v: Vec<usize> = (0..30).collect();
                        v.extend((0..30).map(|_| rng.below(b + 1)));
                        v.push(b);
                        v
                    };
                    let mut first_ok = usize::MAX;
                    for cap in caps {
                        let ctx = format!("row138/139 lvl={lvl} n={n} {sh:?} cap={cap}");
                        let (r, _) = diff_hc(
                            l,
                            &ctx,
                            src.as_ptr() as *const c_char,
                            n as c_int,
                            cap.max(1),
                            cap as c_int,
                            lvl,
                        );
                        assert!(
                            r == 0 || (r > 0 && r as usize <= cap),
                            "{ctx}: nonsensical return {r}"
                        );
                        if r > 0 && cap < first_ok {
                            first_ok = cap;
                        }
                        if cap >= b {
                            assert!(r > 0, "{ctx}: full bound must succeed");
                        }
                    }
                    assert!(
                        first_ok != usize::MAX,
                        "row138/139 lvl={lvl} n={n} {sh:?}: vacuous sweep"
                    );
                }
            }
        }
    }
}

// ===========================================================================
// Rows 140, 141 - LZ4_compress_HC_extStateHC[_fastReset]: misaligned state and
// LZ4_initStreamHC failure -> 0
// ===========================================================================

#[test]
fn err_140_141_hc_ext_state_rejections() {
    let l = libs();
    let mut rng = Rng::new(140);
    unsafe {
        let hss = sizeof_state_hc(l);
        let src = gen_real(&mut rng, Shape::TextLike, 2000);
        let cap = bound(l, 2000) as usize;

        // Row 140: LZ4_compress_HC_extStateHC_fastReset checks the alignment
        // itself (lz4hc.c:1503) and returns 0 before touching the state.
        let (fc, fr) = l.sym::<FnExtState>("LZ4_compress_HC_extStateHC_fastReset");
        let mut cs = Scratch::new(hss + 16);
        let mut rs = Scratch::new(hss + 16);
        for &off in &MISALIGN {
            for &lvl in &[1i32, 2, 9, 12, 0, 13] {
                let mut cb = dstbuf(cap);
                let mut rb = dstbuf(cap);
                let a = fc(
                    cs.at(off),
                    src.as_ptr() as *const c_char,
                    cb.as_mut_ptr() as *mut c_char,
                    2000,
                    cap as c_int,
                    lvl,
                );
                let b = fr(
                    rs.at(off),
                    src.as_ptr() as *const c_char,
                    rb.as_mut_ptr() as *mut c_char,
                    2000,
                    cap as c_int,
                    lvl,
                );
                let ctx = format!("row140 fastReset off={off} lvl={lvl}");
                same_int_and_bytes(&ctx, a, b, &cb, &rb);
                same_full_buffers(&ctx, &cb, &rb);
                assert_eq!(a, 0, "{ctx}: expected 0, got {a}");
            }
        }
        // NOTE: `state == NULL` is NOT passed to _fastReset.  LZ4_isAligned(NULL)
        // is true (0 % 8 == 0), so the alignment guard lets it through and
        // LZ4_resetStreamHC_fast would then dereference NULL.  That is a genuine
        // invalid dereference, not a reported error, so it is skipped here.

        // Row 141: LZ4_compress_HC_extStateHC calls LZ4_initStreamHC first
        // (lz4hc.c:1515), so NULL *and* misaligned state both give 0.  The size
        // check inside LZ4_initStreamHC cannot fire from here (the wrapper always
        // passes `sizeof(LZ4_streamHC_t)`), so no undersized buffer is passed --
        // that would be an out-of-bounds MEM_INIT, not a reported error.
        let (ec, er) = l.sym::<FnExtState>("LZ4_compress_HC_extStateHC");
        for &lvl in &[1i32, 2, 9, 12, 0, 13] {
            let mut cb = dstbuf(cap);
            let mut rb = dstbuf(cap);
            let a = ec(
                std::ptr::null_mut(),
                src.as_ptr() as *const c_char,
                cb.as_mut_ptr() as *mut c_char,
                2000,
                cap as c_int,
                lvl,
            );
            let b = er(
                std::ptr::null_mut(),
                src.as_ptr() as *const c_char,
                rb.as_mut_ptr() as *mut c_char,
                2000,
                cap as c_int,
                lvl,
            );
            let ctx = format!("row141 NULL state lvl={lvl}");
            same_int_and_bytes(&ctx, a, b, &cb, &rb);
            same_full_buffers(&ctx, &cb, &rb);
            assert_eq!(a, 0, "{ctx}: expected 0, got {a}");
            for &off in &MISALIGN {
                let mut cb = dstbuf(cap);
                let mut rb = dstbuf(cap);
                let a = ec(
                    cs.at(off),
                    src.as_ptr() as *const c_char,
                    cb.as_mut_ptr() as *mut c_char,
                    2000,
                    cap as c_int,
                    lvl,
                );
                let b = er(
                    rs.at(off),
                    src.as_ptr() as *const c_char,
                    rb.as_mut_ptr() as *mut c_char,
                    2000,
                    cap as c_int,
                    lvl,
                );
                let ctx = format!("row141 misaligned off={off} lvl={lvl}");
                same_int_and_bytes(&ctx, a, b, &cb, &rb);
                same_full_buffers(&ctx, &cb, &rb);
                assert_eq!(a, 0, "{ctx}: expected 0, got {a}");
            }
        }
        // A properly aligned, properly sized state succeeds through both.
        for &lvl in &[1i32, 9, 12] {
            let mut cb = dstbuf(cap);
            let mut rb = dstbuf(cap);
            let a = ec(
                cs.ptr(),
                src.as_ptr() as *const c_char,
                cb.as_mut_ptr() as *mut c_char,
                2000,
                cap as c_int,
                lvl,
            );
            let b = er(
                rs.ptr(),
                src.as_ptr() as *const c_char,
                rb.as_mut_ptr() as *mut c_char,
                2000,
                cap as c_int,
                lvl,
            );
            let ctx = format!("row140/141 aligned lvl={lvl}");
            same_int_and_bytes(&ctx, a, b, &cb, &rb);
            same_full_buffers(&ctx, &cb, &rb);
            assert!(a > 0, "{ctx}: expected success, got {a}");
        }
    }
}

// ===========================================================================
// Row 142 - LZ4_compress_HC: LZ4HC_HEAPMODE == 1 and ALLOC failure -> 0
// ===========================================================================

#[test]
fn err_142_compress_hc_heapmode_alloc_failure_unreachable() {
    // lz4hc.c:1522-1525 is compiled in (LZ4HC_HEAPMODE defaults to 1) but its
    // `if (statePtr == NULL) return 0;` needs a malloc failure, which cannot be
    // provoked through the public ABI.  Pin the success side across every level
    // and strategy.
    let l = libs();
    let mut rng = Rng::new(142);
    unsafe {
        for &lvl in &[1i32, 2, 3, 6, 9, 10, 11, 12] {
            for &n in &[0usize, 1, 13, 1000] {
                let src = gen_real(&mut rng, Shape::TextLike, n);
                let cap = bound(l, n as c_int) as usize;
                let ctx = format!("row142 lvl={lvl} n={n}");
                let (r, _) = diff_hc(
                    l,
                    &ctx,
                    src.as_ptr() as *const c_char,
                    n as c_int,
                    cap,
                    cap as c_int,
                    lvl,
                );
                assert!(r > 0, "{ctx}: expected success, got {r}");
            }
        }
    }
}

// ===========================================================================
// Row 143 - LZ4_compress_HC_destSize: LZ4_initStreamHC returned NULL -> 0
// ===========================================================================

#[test]
fn err_143_hc_destsize_init_failure() {
    let l = libs();
    let mut rng = Rng::new(143);
    unsafe {
        let hss = sizeof_state_hc(l);
        let (fc, fr) = l.sym::<FnDestSizeExtState>("LZ4_compress_HC_destSize");
        let src = gen_real(&mut rng, Shape::TextLike, 2000);
        let mut cs = Scratch::new(hss + 16);
        let mut rs = Scratch::new(hss + 16);
        for &lvl in &[1i32, 9, 12, 0, 13] {
            for &target in &[1i32, 16, 100, 4096] {
                // NULL state.
                let mut cb = dstbuf(4096);
                let mut rb = dstbuf(4096);
                let mut cn = 2000i32;
                let mut rn = 2000i32;
                let a = fc(
                    std::ptr::null_mut(),
                    src.as_ptr() as *const c_char,
                    cb.as_mut_ptr() as *mut c_char,
                    &mut cn,
                    target,
                    lvl,
                );
                let b = fr(
                    std::ptr::null_mut(),
                    src.as_ptr() as *const c_char,
                    rb.as_mut_ptr() as *mut c_char,
                    &mut rn,
                    target,
                    lvl,
                );
                let ctx = format!("row143 NULL state lvl={lvl} target={target}");
                same_int_and_bytes(&ctx, a, b, &cb, &rb);
                same_full_buffers(&ctx, &cb, &rb);
                assert_eq!(a, 0, "{ctx}: expected 0, got {a}");
                assert_eq!(cn, rn, "{ctx}: *sourceSizePtr mismatch (C={cn} Rust={rn})");
                assert_eq!(cn, 2000, "{ctx}: *sourceSizePtr must be untouched, got {cn}");
                // Misaligned state.
                for &off in &MISALIGN {
                    let mut cb = dstbuf(4096);
                    let mut rb = dstbuf(4096);
                    let mut cn = 2000i32;
                    let mut rn = 2000i32;
                    let a = fc(
                        cs.at(off),
                        src.as_ptr() as *const c_char,
                        cb.as_mut_ptr() as *mut c_char,
                        &mut cn,
                        target,
                        lvl,
                    );
                    let b = fr(
                        rs.at(off),
                        src.as_ptr() as *const c_char,
                        rb.as_mut_ptr() as *mut c_char,
                        &mut rn,
                        target,
                        lvl,
                    );
                    let ctx = format!("row143 misaligned off={off} lvl={lvl} target={target}");
                    same_int_and_bytes(&ctx, a, b, &cb, &rb);
                    same_full_buffers(&ctx, &cb, &rb);
                    assert_eq!(a, 0, "{ctx}: expected 0, got {a}");
                    assert_eq!(cn, rn, "{ctx}: *sourceSizePtr mismatch");
                    assert_eq!(cn, 2000, "{ctx}: *sourceSizePtr must be untouched");
                }
                // A valid state succeeds.
                let mut cb = dstbuf(4096);
                let mut rb = dstbuf(4096);
                let mut cn = 2000i32;
                let mut rn = 2000i32;
                let a = fc(
                    cs.ptr(),
                    src.as_ptr() as *const c_char,
                    cb.as_mut_ptr() as *mut c_char,
                    &mut cn,
                    target,
                    lvl,
                );
                let b = fr(
                    rs.ptr(),
                    src.as_ptr() as *const c_char,
                    rb.as_mut_ptr() as *mut c_char,
                    &mut rn,
                    target,
                    lvl,
                );
                let ctx = format!("row143 valid lvl={lvl} target={target}");
                same_int_and_bytes(&ctx, a, b, &cb, &rb);
                same_full_buffers(&ctx, &cb, &rb);
                assert_eq!(cn, rn, "{ctx}: *sourceSizePtr mismatch");
                assert!(a > 0, "{ctx}: expected success, got {a}");
            }
        }
    }
}

// ===========================================================================
// Row 144 - LZ4_createStreamHC: ALLOC_AND_ZERO failure -> NULL.  UNREACHABLE.
// ===========================================================================

#[test]
fn err_144_create_stream_hc_alloc_failure_unreachable() {
    // lz4hc.c:1558 `if (state == NULL) return NULL;` needs a malloc failure.
    // Pin the success side, including the documented side effect that the fresh
    // stream is set to LZ4HC_CLEVEL_DEFAULT (observable by comparing a first
    // block against an explicit level 9).
    let l = libs();
    let mut rng = Rng::new(144);
    unsafe {
        let src = gen_real(&mut rng, Shape::TextLike, 2000);
        let cap = bound(l, 2000) as usize;
        let ref9 = {
            let h = create_pair(l, "LZ4_createStreamHC");
            hc_stream_int(l, "LZ4_setCompressionLevel", &h, LZ4HC_CLEVEL_DEFAULT);
            let (r, cb) = diff_hc_continue(l, "row144 ref9", &h, &src, 2000, cap);
            assert!(r > 0);
            free_pair(l, "LZ4_freeStreamHC", h);
            cb[..r as usize].to_vec()
        };
        for _ in 0..50 {
            let h = create_pair(l, "LZ4_createStreamHC");
            let (r, cb) = diff_hc_continue(l, "row144 default level", &h, &src, 2000, cap);
            assert!(r > 0, "row144: expected success");
            assert_eq!(
                &cb[..r as usize],
                &ref9[..],
                "row144: a fresh LZ4_streamHC_t must default to LZ4HC_CLEVEL_DEFAULT"
            );
            free_pair(l, "LZ4_freeStreamHC", h);
        }
    }
}

// ===========================================================================
// Row 145 - LZ4_freeStreamHC(NULL) -> 0
// ===========================================================================

#[test]
fn err_145_free_stream_hc_null() {
    let l = libs();
    unsafe {
        let (fc, fr) = l.sym::<FnFreePtr>("LZ4_freeStreamHC");
        for _ in 0..100 {
            let a = fc(std::ptr::null_mut());
            let b = fr(std::ptr::null_mut());
            assert_eq!(a, b, "row145: return mismatch (C={a} Rust={b})");
            assert_eq!(a, 0, "row145: LZ4_freeStreamHC(NULL) must be 0, got {a}");
        }
    }
}

// ===========================================================================
// Rows 146, 147, 148 - LZ4_initStreamHC: NULL / size too small / misaligned
// ===========================================================================

#[test]
fn err_146_147_148_init_stream_hc_null_undersized_misaligned() {
    let l = libs();
    unsafe {
        let hss = sizeof_state_hc(l);
        assert_eq!(
            hss, LZ4_STREAMHC_MINSIZE,
            "LZ4_sizeofStateHC() != LZ4_STREAMHC_MINSIZE ({hss} vs {LZ4_STREAMHC_MINSIZE})"
        );
        let (ic, ir) = l.sym::<FnInitStream>("LZ4_initStreamHC");

        // Row 146: buffer == NULL (lz4hc.c:1578) -> NULL for every size.
        for &size in &[0usize, 1, 32, hss - 1, hss, hss + 1, usize::MAX] {
            let a = ic(std::ptr::null_mut(), size);
            let b = ir(std::ptr::null_mut(), size);
            assert!(a.is_null(), "row146 C LZ4_initStreamHC(NULL, {size}) = {a:?}");
            assert!(b.is_null(), "row146 Rust LZ4_initStreamHC(NULL, {size}) = {b:?}");
        }

        // Row 147: size < sizeof(LZ4_streamHC_t) (lz4hc.c:1579) -> NULL.  The
        // allocation stays full size; only the declared size lies.
        let mut cs = Scratch::new(hss + 16);
        let mut rs = Scratch::new(hss + 16);
        for &size in &[0usize, 1, 7, 8, 1024, hss / 2, hss - 2, hss - 1] {
            let a = ic(cs.ptr(), size);
            let b = ir(rs.ptr(), size);
            assert!(a.is_null(), "row147 C LZ4_initStreamHC(buf, {size}) = {a:?}");
            assert!(b.is_null(), "row147 Rust LZ4_initStreamHC(buf, {size}) = {b:?}");
        }
        for (i, &x) in cs.bytes(hss).iter().enumerate() {
            assert_eq!(x, SENT, "row147: C scribbled the rejected buffer at {i}");
        }
        for (i, &x) in rs.bytes(hss).iter().enumerate() {
            assert_eq!(x, SENT, "row147: Rust scribbled the rejected buffer at {i}");
        }

        // Row 148: misaligned buffer (lz4hc.c:1580) -> NULL.
        for &off in &MISALIGN {
            let a = ic(cs.at(off), hss);
            let b = ir(rs.at(off), hss);
            assert!(a.is_null(), "row148 C LZ4_initStreamHC(buf+{off}) = {a:?}");
            assert!(b.is_null(), "row148 Rust LZ4_initStreamHC(buf+{off}) = {b:?}");
            let a = ic(cs.at(off), hss - 1);
            let b = ir(rs.at(off), hss - 1);
            assert!(a.is_null(), "row147+148 C combined off={off}");
            assert!(b.is_null(), "row147+148 Rust combined off={off}");
        }

        // Exactly sizeof(LZ4_streamHC_t), aligned -> success, and the resulting
        // state images are identical.
        for &size in &[hss, hss + 1, hss + 8] {
            let a = ic(cs.ptr(), size);
            let b = ir(rs.ptr(), size);
            assert_eq!(a, cs.ptr(), "row147 C LZ4_initStreamHC(buf, {size}) must succeed");
            assert_eq!(b, rs.ptr(), "row147 Rust LZ4_initStreamHC(buf, {size}) must succeed");
            same_full_buffers(
                &format!("row146/147/148 initialised HC state size={size}"),
                cs.bytes(hss),
                rs.bytes(hss),
            );
        }
        for &off in &[0usize, 8, 16] {
            let a = ic(cs.at(off), hss);
            let b = ir(rs.at(off), hss);
            assert_eq!(a, cs.at(off), "row148 C aligned off={off} must succeed");
            assert_eq!(b, rs.at(off), "row148 Rust aligned off={off} must succeed");
        }
    }
}

// ===========================================================================
// Row 149 - select_searchDict_function: dictCtx == NULL -> NULL search function
// ===========================================================================

#[test]
fn err_149_select_search_dict_null_dictctx_unreachable() {
    // lz4hc.c:516 `if (dictCtx == NULL) return NULL;` is defensive-only:
    // `select_searchDict_function` is called exactly once, from LZ4MID_compress
    // under `(dict == usingDictCtxHc)` (lz4hc.c:546), and the
    // `usingDictCtxHc` directive is only chosen by
    // LZ4HC_compress_generic_noDictCtx/_dictCtx after an explicit
    // `ctx->dictCtx == NULL` test (lz4hc.c:1436-1483).  So `dictCtx` is never
    // NULL at that call site and the NULL search function is never returned.
    //
    // The externally reachable neighbour is LZ4_attach_HC_dictionary(ws, NULL),
    // which stores `dictCtx = NULL` and must make the stream behave exactly like
    // one with no dictionary attached at all -- for the lz4mid levels in
    // particular, since those are the ones that would consult the search
    // function.
    let l = libs();
    let mut rng = Rng::new(149);
    unsafe {
        let (ac, ar) = l.sym::<FnAttach>("LZ4_attach_HC_dictionary");
        for &lvl in &[1i32, 2, 3, 9, 12] {
            for &n in &[100usize, 2000, 8000] {
                let src = gen_real(&mut rng, Shape::TextLike, n);
                let cap = bound(l, n as c_int) as usize;
                let plain = {
                    let h = create_pair(l, "LZ4_createStreamHC");
                    hc_stream_int(l, "LZ4_setCompressionLevel", &h, lvl);
                    let ctx = format!("row149 no-attach lvl={lvl} n={n}");
                    let (r, cb) = diff_hc_continue(l, &ctx, &h, &src, n as c_int, cap);
                    assert!(r > 0, "{ctx}: expected success");
                    free_pair(l, "LZ4_freeStreamHC", h);
                    cb[..r as usize].to_vec()
                };
                let h = create_pair(l, "LZ4_createStreamHC");
                hc_stream_int(l, "LZ4_setCompressionLevel", &h, lvl);
                ac(h.c, std::ptr::null());
                ar(h.r, std::ptr::null());
                let ctx = format!("row149 attach-NULL lvl={lvl} n={n}");
                let (r, cb) = diff_hc_continue(l, &ctx, &h, &src, n as c_int, cap);
                assert!(r > 0, "{ctx}: expected success");
                assert_eq!(
                    &cb[..r as usize],
                    &plain[..],
                    "{ctx}: attaching a NULL dictionary must be a no-op"
                );
                free_pair(l, "LZ4_freeStreamHC", h);
            }
        }
    }
}

// ===========================================================================
// Rows 150, 151, 152 - LZ4_loadDictHC: > 64 KB truncation, negative dictSize
// (assert-only), dictSize < LZ4HC_HASHSIZE
// ===========================================================================

#[test]
fn err_150_151_152_load_dict_hc_size_handling() {
    let l = libs();
    let mut rng = Rng::new(150);
    unsafe {
        let (fc, fr) = l.sym::<FnLoadDict>("LZ4_loadDictHC");
        let dict = gen_real(&mut rng, Shape::TextLike, 200_000);

        // Row 150: dictSize > 64 KB -> only the last 64 KB is used and 65536 is
        // returned.
        for &lvl in &[1i32, 2, 3, 9, 12] {
            for &ds in &[65537i32, 70_000, 131_072, 200_000] {
                let h = create_pair(l, "LZ4_createStreamHC");
                hc_stream_int(l, "LZ4_setCompressionLevel", &h, lvl);
                let a = fc(h.c, dict.as_ptr() as *const c_char, ds);
                let b = fr(h.r, dict.as_ptr() as *const c_char, ds);
                assert_eq!(a, b, "row150 lvl={lvl} ds={ds}: mismatch (C={a} Rust={b})");
                assert_eq!(a, 65536, "row150 lvl={lvl} ds={ds}: expected 65536, got {a}");
                free_pair(l, "LZ4_freeStreamHC", h);
            }
            // Truncation really is "last 64 KB": loading the tail directly gives
            // byte-identical compression of a following block.
            let blk = gen_real(&mut rng, Shape::TextLike, 5000);
            let cap = bound(l, 5000) as usize;
            let mut outs: Vec<Vec<u8>> = Vec::new();
            for &(off, ds) in &[(0usize, 200_000i32), (200_000 - 65536, 65536)] {
                let h = create_pair(l, "LZ4_createStreamHC");
                hc_stream_int(l, "LZ4_setCompressionLevel", &h, lvl);
                let p = dict.as_ptr().add(off) as *const c_char;
                assert_eq!(fc(h.c, p, ds), fr(h.r, p, ds), "row150 tail load mismatch");
                let ctx = format!("row150 tail lvl={lvl} off={off}");
                let (r, cb) = diff_hc_continue(l, &ctx, &h, &blk, 5000, cap);
                assert!(r > 0, "{ctx}: expected success");
                outs.push(cb[..r as usize].to_vec());
                free_pair(l, "LZ4_freeStreamHC", h);
            }
            assert_eq!(
                outs[0], outs[1],
                "row150 lvl={lvl}: loading 200000 bytes must equal loading the last 64 KB"
            );
        }

        // Row 152: dictSize < LZ4HC_HASHSIZE (4) in non-lz4mid strategies means
        // no chain insertion is performed, but it is *not* an error and the
        // (clamped) dictSize is returned unchanged.
        for &lvl in &[3i32, 6, 9, 10, 12] {
            for &ds in &[0i32, 1, 2, 3, 4, 5] {
                let h = create_pair(l, "LZ4_createStreamHC");
                hc_stream_int(l, "LZ4_setCompressionLevel", &h, lvl);
                let a = fc(h.c, dict.as_ptr() as *const c_char, ds);
                let b = fr(h.r, dict.as_ptr() as *const c_char, ds);
                assert_eq!(a, b, "row152 lvl={lvl} ds={ds}: mismatch (C={a} Rust={b})");
                assert_eq!(a, ds, "row152 lvl={lvl} ds={ds}: expected {ds}, got {a}");
                // A following block still compresses identically in both.
                let blk = gen_real(&mut rng, Shape::TextLike, 800);
                let cap = bound(l, 800) as usize;
                let ctx = format!("row152 lvl={lvl} ds={ds} follow-up");
                let (r, _) = diff_hc_continue(l, &ctx, &h, &blk, 800, cap);
                assert!(r > 0, "{ctx}: expected success");
                free_pair(l, "LZ4_freeStreamHC", h);
            }
        }
        // The lz4mid levels take the LZ4MID_fillHTable branch instead; same
        // "not an error" contract.
        for &lvl in &[1i32, 2] {
            for &ds in &[0i32, 1, 2, 3, 4, 5, 100] {
                let h = create_pair(l, "LZ4_createStreamHC");
                hc_stream_int(l, "LZ4_setCompressionLevel", &h, lvl);
                let a = fc(h.c, dict.as_ptr() as *const c_char, ds);
                let b = fr(h.r, dict.as_ptr() as *const c_char, ds);
                assert_eq!(a, b, "row152 mid lvl={lvl} ds={ds}: mismatch");
                assert_eq!(a, ds, "row152 mid lvl={lvl} ds={ds}: expected {ds}, got {a}");
                free_pair(l, "LZ4_freeStreamHC", h);
            }
        }

        // Row 151: `assert(dictSize >= 0)` (lz4hc.c:1632) -- ASSERT-ONLY.
        // With the assert compiled out, a negative dictSize is handled as
        // follows: the `> 64 KB` truncation does not apply, LZ4HC_init_internal
        // only records pointers, `ctxPtr->end = dictionary + dictSize` is pure
        // (out-of-range but never dereferenced) pointer arithmetic, and
        // `if (dictSize >= LZ4HC_HASHSIZE)` is false, so nothing is read.  That
        // is observable and safe for the hashChain/optimal strategies and is
        // asserted here.
        //
        // It is NOT exercised for the lz4mid levels (1 and 2): there the same
        // value reaches `LZ4MID_fillHTable(ctxPtr, dictionary, (size_t)dictSize)`
        // (lz4hc.c:1647), where the `(size_t)` cast turns it into a huge length
        // and the table fill reads wild memory.  The stream is freed immediately
        // after each probe and never used to compress.
        for &lvl in &[3i32, 6, 9, 10, 12] {
            for &ds in &[-1i32, -2, -4, -1000, -65536] {
                let h = create_pair(l, "LZ4_createStreamHC");
                hc_stream_int(l, "LZ4_setCompressionLevel", &h, lvl);
                let p = dict.as_ptr().add(100_000) as *const c_char;
                let a = fc(h.c, p, ds);
                let b = fr(h.r, p, ds);
                assert_eq!(a, b, "row151 lvl={lvl} ds={ds}: mismatch (C={a} Rust={b})");
                assert_eq!(a, ds, "row151 lvl={lvl} ds={ds}: expected {ds}, got {a}");
                free_pair(l, "LZ4_freeStreamHC", h);
            }
        }
    }
}

// ===========================================================================
// Rows 153, 154 - LZ4_compressHC_continue_generic: history index overflow
// (> 2 GB) and src overlapping the recorded extDict
// ===========================================================================

#[test]
fn err_153_154_hc_continue_history_overflow_and_overlap() {
    let l = libs();
    let mut rng = Rng::new(153);
    unsafe {
        // Each failing LZ4_compress_HC_continue call still advances
        // `ctx->end += *srcSizePtr` (lz4hc.c:1391) before the strategy runs, and
        // the next (non-contiguous) call folds that into `dictLimit` through
        // LZ4HC_setExternalDict.  So `dictLimit` grows by `n` per round and
        // crosses the 2 GB re-anchor threshold (lz4hc.c:1694) after ~2 GB / n
        // rounds -- without ever compressing 2 GB.
        //
        // Reusing the *same* source pointer every round also makes
        // `sourceEnd > dictBegin && src < dictEnd` true, which is exactly row
        // 154's overlap trim (lz4hc.c:1705-1716); the trimmed extDict then falls
        // below LZ4HC_HASHSIZE and is invalidated entirely.
        let n = 1024 * 1024usize;
        let src = gen_real(&mut rng, Shape::Compressible, n);
        let h = create_pair(l, "LZ4_createStreamHC");
        // Level 2 (lz4mid) keeps every failing round cheap: the first match
        // found makes LZ4HC_encodeSequence report a buffer issue immediately.
        hc_stream_int(l, "LZ4_resetStreamHC_fast", &h, LZ4HC_CLEVEL_MIN);
        let rounds = 2 * 1024 / 1 + 60; // 2 GB / 1 MB, plus slack
        for i in 0..rounds {
            let ctx = format!("row153/154 round {i}");
            let (r, _) = diff_hc_continue(l, &ctx, &h, &src, n as c_int, 0);
            assert_eq!(r, 0, "{ctx}: expected failure with dstCapacity 0");
        }
        // After the re-anchor a normal block must still compress identically,
        // and the resulting state must be identical as observed through
        // LZ4_saveDictHC.
        let small = gen_real(&mut rng, Shape::TextLike, 20_000);
        let cap = bound(l, 20_000) as usize;
        let (r, _) = diff_hc_continue(l, "row153 after re-anchor", &h, &small, 20_000, cap);
        assert!(r > 0, "row153: expected success after the re-anchor, got {r}");
        {
            let (svc, svr) = l.sym::<FnSaveDict>("LZ4_saveDictHC");
            let mut cb = vec![SENT; 65_536 + GUARD];
            let mut rb = vec![SENT; 65_536 + GUARD];
            let a = svc(h.c, cb.as_mut_ptr() as *mut c_char, 65_536);
            let b = svr(h.r, rb.as_mut_ptr() as *mut c_char, 65_536);
            assert_eq!(a, b, "row153: saveDictHC return mismatch (C={a} Rust={b})");
            same_full_buffers("row153 saveDictHC content", &cb, &rb);
        }
        free_pair(l, "LZ4_freeStreamHC", h);

        // Row 154 on its own, without the 2 GB machinery: a ring-buffer style
        // reuse where each new block overlaps the previously recorded one.
        for &lvl in &[1i32, 2, 3, 9, 12] {
            let buf = gen_real(&mut rng, Shape::TextLike, 100_000);
            let h = create_pair(l, "LZ4_createStreamHC");
            hc_stream_int(l, "LZ4_setCompressionLevel", &h, lvl);
            let mut pos = 0usize;
            for i in 0..20usize {
                let len = 5000usize;
                let cap = bound(l, len as c_int) as usize;
                let mut cb = dstbuf(cap);
                let mut rb = dstbuf(cap);
                let (cc, cr) = l.sym::<FnHCContinue>("LZ4_compress_HC_continue");
                let p = buf.as_ptr().add(pos) as *const c_char;
                let a = cc(h.c, p, cb.as_mut_ptr() as *mut c_char, len as c_int, cap as c_int);
                let b = cr(h.r, p, rb.as_mut_ptr() as *mut c_char, len as c_int, cap as c_int);
                let ctx = format!("row154 overlap lvl={lvl} i={i} pos={pos}");
                same_int_and_bytes(&ctx, a, b, &cb, &rb);
                same_full_buffers(&ctx, &cb, &rb);
                assert!(a > 0, "{ctx}: expected success, got {a}");
                // Step *backwards* by less than a block so the next src overlaps
                // the extDict that was just recorded.
                pos = if pos + 2500 + len <= buf.len() { pos + 2500 } else { 0 };
            }
            free_pair(l, "LZ4_freeStreamHC", h);
        }
    }
}

// ===========================================================================
// Rows 155, 156 - LZ4_saveDictHC: clamping; safeBuffer == NULL
// ===========================================================================

#[test]
fn err_155_156_save_dict_hc_clamping_and_null_safebuffer() {
    let l = libs();
    let mut rng = Rng::new(155);
    unsafe {
        let (svc, svr) = l.sym::<FnSaveDict>("LZ4_saveDictHC");
        // Row 155: dictSize > 64 KB -> 64 KB; < 4 -> 0; > prefixSize ->
        // prefixSize.  The prefix is the block just compressed.
        for &prefix in &[0usize, 3, 4, 100, 5000, 70_000] {
            let src = gen_real(&mut rng, Shape::TextLike, prefix.max(1));
            for &ds in &[
                c_int::MIN,
                -1,
                0,
                1,
                3,
                4,
                5,
                100,
                65535,
                65536,
                65537,
                100_000,
                c_int::MAX,
            ] {
                let h = create_pair(l, "LZ4_createStreamHC");
                hc_stream_int(l, "LZ4_setCompressionLevel", &h, LZ4HC_CLEVEL_DEFAULT);
                if prefix > 0 {
                    let cap = bound(l, prefix as c_int) as usize;
                    let ctx = format!("row155 seed prefix={prefix}");
                    let (r, _) = diff_hc_continue(l, &ctx, &h, &src, prefix as c_int, cap);
                    assert!(r > 0, "{ctx}: expected success");
                }
                let mut cb = vec![SENT; 70_000 + GUARD];
                let mut rb = vec![SENT; 70_000 + GUARD];
                let a = svc(h.c, cb.as_mut_ptr() as *mut c_char, ds);
                let b = svr(h.r, rb.as_mut_ptr() as *mut c_char, ds);
                let ctx = format!("row155 prefix={prefix} ds={ds}");
                assert_eq!(a, b, "{ctx}: return mismatch (C={a} Rust={b})");
                // lz4hc.c:1748-1750, applied in order, on *signed* dictSize.
                let want = {
                    let mut d = if ds > 65536 { 65536 } else { ds };
                    if d < 4 {
                        d = 0;
                    }
                    if d > prefix as c_int {
                        d = prefix as c_int;
                    }
                    d
                };
                assert_eq!(a, want, "{ctx}: expected {want}, got {a}");
                same_full_buffers(&ctx, &cb, &rb);
                if a > 0 {
                    let k = a as usize;
                    assert_eq!(
                        &cb[..k],
                        &src[prefix - k..prefix],
                        "{ctx}: saved bytes are not the prefix tail"
                    );
                }
                free_pair(l, "LZ4_freeStreamHC", h);
            }
        }

        // Row 156: `if (safeBuffer == NULL) assert(dictSize == 0);`
        // (lz4hc.c:1751) -- ASSERT-ONLY CONTRACT.  With a nonzero clamped
        // dictSize this would `LZ4_memmove` to NULL, so it is not provoked.  The
        // reachable in-contract case is a NULL safeBuffer once the clamping has
        // already forced dictSize to 0: either because the requested size is
        // < 4, or because the stream has no prefix at all.
        for &ds in &[c_int::MIN, -1, 0, 1, 2, 3] {
            for &prefix in &[0usize, 100, 5000] {
                let src = gen_real(&mut rng, Shape::TextLike, prefix.max(1));
                let h = create_pair(l, "LZ4_createStreamHC");
                if prefix > 0 {
                    let cap = bound(l, prefix as c_int) as usize;
                    let (r, _) = diff_hc_continue(
                        l,
                        &format!("row156 seed prefix={prefix}"),
                        &h,
                        &src,
                        prefix as c_int,
                        cap,
                    );
                    assert!(r > 0);
                }
                let a = svc(h.c, std::ptr::null_mut(), ds);
                let b = svr(h.r, std::ptr::null_mut(), ds);
                let ctx = format!("row156 ds={ds} prefix={prefix}");
                assert_eq!(a, b, "{ctx}: return mismatch (C={a} Rust={b})");
                assert_eq!(a, 0, "{ctx}: expected 0, got {a}");
                free_pair(l, "LZ4_freeStreamHC", h);
            }
        }
        // ... and with no prefix, *any* requested size clamps to 0, so a NULL
        // safeBuffer stays in contract for the whole ladder.
        for &ds in &[4i32, 100, 65536, 100_000, c_int::MAX] {
            let h = create_pair(l, "LZ4_createStreamHC");
            let a = svc(h.c, std::ptr::null_mut(), ds);
            let b = svr(h.r, std::ptr::null_mut(), ds);
            let ctx = format!("row156 no-prefix ds={ds}");
            assert_eq!(a, b, "{ctx}: return mismatch (C={a} Rust={b})");
            assert_eq!(a, 0, "{ctx}: expected 0, got {a}");
            free_pair(l, "LZ4_freeStreamHC", h);
        }
    }
}

// ===========================================================================
// Row 157 - LZ4_resetStreamStateHC: INVERTED convention (1 = failure)
// ===========================================================================

#[test]
fn err_157_reset_stream_state_hc_inverted_convention() {
    let l = libs();
    let mut rng = Rng::new(157);
    unsafe {
        let hss = sizeof_state_hc(l);
        let (fc, fr) = l.sym::<FnResetStreamState>("LZ4_resetStreamStateHC");
        let mut inbuf = gen_real(&mut rng, Shape::TextLike, 64);
        let inptr = inbuf.as_mut_ptr() as *mut c_char;

        // Failure: LZ4_initStreamHC returns NULL -> 1 (note the inversion).
        let a = fc(std::ptr::null_mut(), inptr);
        let b = fr(std::ptr::null_mut(), inptr);
        assert_eq!(a, b, "row157 NULL state: mismatch (C={a} Rust={b})");
        assert_eq!(a, 1, "row157: NULL state must return 1 (failure), got {a}");
        // ... including with a NULL inputBuffer, which is only stored.
        let a = fc(std::ptr::null_mut(), std::ptr::null_mut());
        let b = fr(std::ptr::null_mut(), std::ptr::null_mut());
        assert_eq!(a, b, "row157 NULL/NULL: mismatch (C={a} Rust={b})");
        assert_eq!(a, 1, "row157: NULL state must return 1, got {a}");

        let mut cs = Scratch::new(hss + 16);
        let mut rs = Scratch::new(hss + 16);
        for &off in &MISALIGN {
            let a = fc(cs.at(off), inptr);
            let b = fr(rs.at(off), inptr);
            assert_eq!(a, b, "row157 misaligned off={off}: mismatch (C={a} Rust={b})");
            assert_eq!(a, 1, "row157: misaligned state must return 1, got {a}");
        }
        // Success: 0.  (NOTE: an *undersized* buffer is not probed -- the
        // wrapper always passes `sizeof(*hc4)` to LZ4_initStreamHC, so a short
        // allocation would be an out-of-bounds MEM_INIT rather than a reported
        // error.)
        for &off in &[0usize, 8, 16] {
            let a = fc(cs.at(off), inptr);
            let b = fr(rs.at(off), inptr);
            assert_eq!(a, b, "row157 aligned off={off}: mismatch (C={a} Rust={b})");
            assert_eq!(a, 0, "row157: aligned state must return 0 (success), got {a}");
        }
        same_full_buffers("row157 reset HC state image", cs.bytes(hss), rs.bytes(hss));

        // For contrast, LZ4_resetStreamState (the non-HC twin) has *no*
        // validation at all: it only MEM_INITs, always returns 0, and NULL or
        // undersized buffers would be an invalid write rather than an error.  A
        // large-but-misaligned buffer is the only out-of-contract input it
        // tolerates, and both libraries must handle it identically.
        let ss = sizeof_state(l);
        let (rc2, rr2) = l.sym::<FnResetStreamState>("LZ4_resetStreamState");
        let mut cs2 = Scratch::new(ss + 16);
        let mut rs2 = Scratch::new(ss + 16);
        for &off in &[0usize, 1, 4, 8] {
            let a = rc2(cs2.at(off), inptr);
            let b = rr2(rs2.at(off), inptr);
            assert_eq!(a, b, "row157 resetStreamState off={off}: mismatch (C={a} Rust={b})");
            assert_eq!(a, 0, "row157: LZ4_resetStreamState always returns 0, got {a}");
        }
        same_full_buffers("row157 resetStreamState image", cs2.bytes(ss), rs2.bytes(ss));
    }
}

// ===========================================================================
// Row 158 - LZ4_createHC: LZ4_createStreamHC returned NULL -> NULL.  UNREACHABLE.
// ===========================================================================

#[test]
fn err_158_create_hc_alloc_failure_unreachable() {
    // lz4hc.c:2161-2162 needs a malloc failure inside LZ4_createStreamHC.  Pin
    // the success side: a non-NULL handle for every inputBuffer argument
    // (including NULL, which is only recorded by LZ4HC_init_internal), usable
    // with the obsolete continue wrappers and released by LZ4_freeHC.
    let l = libs();
    let mut rng = Rng::new(158);
    unsafe {
        let (cc, cr) = l.sym::<FnCreateHC>("LZ4_createHC");
        let (fc, fr) = l.sym::<FnFreePtr>("LZ4_freeHC");
        let mut inbuf = gen_real(&mut rng, Shape::TextLike, 64);
        for pass in 0..2 {
            let p: *const c_char = if pass == 0 {
                std::ptr::null()
            } else {
                inbuf.as_mut_ptr() as *const c_char
            };
            for _ in 0..20 {
                let a = cc(p);
                let b = cr(p);
                assert!(!a.is_null(), "row158: C LZ4_createHC returned NULL");
                assert!(!b.is_null(), "row158: Rust LZ4_createHC returned NULL");
                let x = fc(a);
                let y = fr(b);
                assert_eq!(x, y, "row158: LZ4_freeHC mismatch (C={x} Rust={y})");
                assert_eq!(x, 0, "row158: LZ4_freeHC must return 0, got {x}");
            }
        }
    }
}

// ===========================================================================
// Row 159 - LZ4_freeHC(NULL) -> 0
// ===========================================================================

#[test]
fn err_159_free_hc_null() {
    let l = libs();
    unsafe {
        let (fc, fr) = l.sym::<FnFreePtr>("LZ4_freeHC");
        for _ in 0..100 {
            let a = fc(std::ptr::null_mut());
            let b = fr(std::ptr::null_mut());
            assert_eq!(a, b, "row159: return mismatch (C={a} Rust={b})");
            assert_eq!(a, 0, "row159: LZ4_freeHC(NULL) must be 0, got {a}");
        }
    }
}

// ###########################################################################
// ## Generic boundary cases (beyond the enumerated ERRORS.md rows)
// ###########################################################################

// ===========================================================================
// NULL src with srcSize 0, NULL dst with dstCapacity 0, and free-on-NULL, for
// every entry point that tolerates it
// ===========================================================================

#[test]
fn err_generic_null_src_and_null_dst_tolerated_entry_points() {
    let l = libs();
    let mut rng = Rng::new(1000);
    unsafe {
        let real = gen_real(&mut rng, Shape::Degenerate, 64);

        // ---- LZ4_compress_default / LZ4_compress_fast ----
        // "src == NULL supported if srcSize == 0" (lz4.c:1361).
        for accel in [None, Some(1), Some(0), Some(-1), Some(65538)] {
            for &cap in &[0i32, 1, 2, 16, 17, 1000] {
                let ctx = format!("generic NULL src default cap={cap} accel={accel:?}");
                let r = diff_compress(l, &ctx, std::ptr::null(), 0, 1024, cap, accel);
                assert_eq!(
                    r,
                    if cap >= 1 { 1 } else { 0 },
                    "{ctx}: unexpected {r}"
                );
            }
            // NULL dst with dstCapacity 0: the `dstCapacity <= 0` guard at
            // lz4.c:1362 fires before `dst[0] = 0`, so dst is never touched.
            let (cd, rd) = l.sym::<FnCompressDefault>("LZ4_compress_default");
            let (cf, rf) = l.sym::<FnCompressFast>("LZ4_compress_fast");
            let (a, b) = match accel {
                None => (
                    cd(real.as_ptr() as *const c_char, std::ptr::null_mut(), 0, 0),
                    rd(real.as_ptr() as *const c_char, std::ptr::null_mut(), 0, 0),
                ),
                Some(x) => (
                    cf(real.as_ptr() as *const c_char, std::ptr::null_mut(), 0, 0, x),
                    rf(real.as_ptr() as *const c_char, std::ptr::null_mut(), 0, 0, x),
                ),
            };
            assert_eq!(a, b, "generic NULL dst accel={accel:?}: mismatch (C={a} Rust={b})");
            assert_eq!(a, 0, "generic NULL dst accel={accel:?}: expected 0, got {a}");
            // Both NULL, both zero.
            let (a, b) = match accel {
                None => (
                    cd(std::ptr::null(), std::ptr::null_mut(), 0, 0),
                    rd(std::ptr::null(), std::ptr::null_mut(), 0, 0),
                ),
                Some(x) => (
                    cf(std::ptr::null(), std::ptr::null_mut(), 0, 0, x),
                    rf(std::ptr::null(), std::ptr::null_mut(), 0, 0, x),
                ),
            };
            assert_eq!(a, b, "generic NULL/NULL accel={accel:?}: mismatch (C={a} Rust={b})");
            assert_eq!(a, 0, "generic NULL/NULL accel={accel:?}: expected 0, got {a}");
        }

        // ---- LZ4_compress_HC ----
        let (hc, hr) = l.sym::<FnHC>("LZ4_compress_HC");
        for &lvl in &[1i32, 2, 3, 6, 9, 10, 11, 12, 0, -1, 13] {
            // NULL dst with dstCapacity 0 -- safe at *every* level: with
            // srcSize 0 the optimal parser's `while (ip <= mflimit)` is false
            // for a real (non-NULL) src, and the last-literals encoder then
            // finds `op + 1 > oend` with `op == oend == NULL`.
            let a = hc(real.as_ptr() as *const c_char, std::ptr::null_mut(), 0, 0, lvl);
            let b = hr(real.as_ptr() as *const c_char, std::ptr::null_mut(), 0, 0, lvl);
            assert_eq!(a, b, "generic HC NULL dst lvl={lvl}: mismatch (C={a} Rust={b})");
            assert_eq!(a, 0, "generic HC NULL dst lvl={lvl}: expected 0, got {a}");
            // A real zero-length source with a real destination emits the
            // 1-byte empty block at every level.
            let mut cb = dstbuf(64);
            let mut rb = dstbuf(64);
            let a = hc(real.as_ptr() as *const c_char, cb.as_mut_ptr() as *mut c_char, 0, 64, lvl);
            let b = hr(real.as_ptr() as *const c_char, rb.as_mut_ptr() as *mut c_char, 0, 64, lvl);
            let ctx = format!("generic HC empty src lvl={lvl}");
            same_int_and_bytes(&ctx, a, b, &cb, &rb);
            same_full_buffers(&ctx, &cb, &rb);
            assert_eq!(a, 1, "{ctx}: expected the 1-byte empty block, got {a}");
        }
        // NULL src with srcSize 0 is only exercised for the lz4mid (1, 2) and
        // hashChain (3..9) strategies: both start with
        // `if (srcSize < LZ4_minLength) goto _last_literals`, so the NULL is
        // never dereferenced.  SKIPPED for levels >= LZ4HC_CLEVEL_OPT_MIN (10):
        // LZ4HC_compress_optimal has no such guard, `mflimit = (BYTE*)0 - 12`
        // wraps to a huge address, `while (ip <= mflimit)` becomes true and
        // LZ4HC_FindLongerMatch would dereference NULL.
        for &lvl in &[1i32, 2, 3, 4, 5, 6, 7, 8, 9, 0, -1] {
            let mut cb = dstbuf(64);
            let mut rb = dstbuf(64);
            let a = hc(std::ptr::null(), cb.as_mut_ptr() as *mut c_char, 0, 64, lvl);
            let b = hr(std::ptr::null(), rb.as_mut_ptr() as *mut c_char, 0, 64, lvl);
            let ctx = format!("generic HC NULL src lvl={lvl}");
            same_int_and_bytes(&ctx, a, b, &cb, &rb);
            same_full_buffers(&ctx, &cb, &rb);
            assert_eq!(a, 1, "{ctx}: expected the 1-byte empty block, got {a}");
            let a = hc(std::ptr::null(), std::ptr::null_mut(), 0, 0, lvl);
            let b = hr(std::ptr::null(), std::ptr::null_mut(), 0, 0, lvl);
            assert_eq!(a, b, "generic HC NULL/NULL lvl={lvl}: mismatch (C={a} Rust={b})");
            assert_eq!(a, 0, "generic HC NULL/NULL lvl={lvl}: expected 0, got {a}");
        }

        // ---- LZ4_decompress_safe ----
        let (dc, dr) = l.sym::<FnDecompressSafe>("LZ4_decompress_safe");
        // NULL src -> -1 for any size pair (checked before any read).
        for &cs in &[0i32, 1, 100] {
            for &cap in &[0i32, 1, 64] {
                let a = dc(std::ptr::null(), std::ptr::null_mut(), cs, cap);
                let b = dr(std::ptr::null(), std::ptr::null_mut(), cs, cap);
                let ctx = format!("generic dec NULL/NULL cs={cs} cap={cap}");
                assert_eq!(a, b, "{ctx}: mismatch (C={a} Rust={b})");
                assert_eq!(a, -1, "{ctx}: expected -1, got {a}");
            }
        }
        // NULL dst with dstCapacity 0: the `outputSize == 0` special case
        // (lz4.c:2062-2068) answers without touching dst.
        for &(b0, cs, want) in &[
            (0u8, 1i32, 0i32),
            (1u8, 1i32, -1i32),
            (0u8, 2i32, -1i32),
            (0u8, 0i32, -1i32),
        ] {
            let blk = vec![b0, 0, 0, 0];
            let a = dc(blk.as_ptr() as *const c_char, std::ptr::null_mut(), cs, 0);
            let b = dr(blk.as_ptr() as *const c_char, std::ptr::null_mut(), cs, 0);
            let ctx = format!("generic dec NULL dst b0={b0} cs={cs}");
            assert_eq!(a, b, "{ctx}: mismatch (C={a} Rust={b})");
            assert_eq!(a, want, "{ctx}: expected {want}, got {a}");
        }

        // ---- Pure functions ----
        assert_eq!(bound(l, 0), 16, "LZ4_compressBound(0) must be 16");
        let ss = sizeof_state(l);
        let hss = sizeof_state_hc(l);
        assert_eq!(ss, LZ4_STREAM_MINSIZE);
        assert_eq!(hss, LZ4_STREAMHC_MINSIZE);
        {
            let (c, r) = l.sym::<FnVoidToInt>("LZ4_sizeofStreamState");
            let (a, b) = (c(), r());
            assert_eq!(a, b, "LZ4_sizeofStreamState mismatch (C={a} Rust={b})");
            assert_eq!(a as usize, ss, "LZ4_sizeofStreamState != LZ4_sizeofState");
        }
        {
            let (c, r) = l.sym::<FnVoidToInt>("LZ4_sizeofStreamStateHC");
            let (a, b) = (c(), r());
            assert_eq!(a, b, "LZ4_sizeofStreamStateHC mismatch (C={a} Rust={b})");
            assert_eq!(a as usize, hss, "LZ4_sizeofStreamStateHC != LZ4_sizeofStateHC");
        }

        // ---- free-on-NULL, all four allocators ----
        for name in [
            "LZ4_freeStream",
            "LZ4_freeStreamHC",
            "LZ4_freeStreamDecode",
            "LZ4_freeHC",
        ] {
            let (fc, fr) = l.sym::<FnFreePtr>(name);
            let a = fc(std::ptr::null_mut());
            let b = fr(std::ptr::null_mut());
            assert_eq!(a, b, "{name}(NULL): mismatch (C={a} Rust={b})");
            assert_eq!(a, 0, "{name}(NULL): expected 0, got {a}");
        }
    }
}

// ===========================================================================
// Extreme srcSize / dstCapacity arguments: 0, -1, i32::MIN, LZ4_MAX_INPUT_SIZE,
// LZ4_MAX_INPUT_SIZE + 1, i32::MAX
// ===========================================================================

#[test]
fn err_generic_extreme_size_arguments() {
    let l = libs();
    let mut rng = Rng::new(1001);
    unsafe {
        // ---- LZ4_compressBound and LZ4_decoderRingBufferSize: pure ----
        let (rbc, rbr) = l.sym::<FnCompressBound>("LZ4_decoderRingBufferSize");
        for &v in &WILD_SIZES {
            let (a, b) = (bound(l, v), bound(l, v));
            assert_eq!(a, b);
            let want = if (v as u32) > LZ4_MAX_INPUT_SIZE as u32 {
                0
            } else {
                v + v / 255 + 16
            };
            assert_eq!(a, want, "LZ4_compressBound({v}): expected {want}, got {a}");
            let (x, y) = (rbc(v), rbr(v));
            assert_eq!(x, y, "LZ4_decoderRingBufferSize({v}): mismatch (C={x} Rust={y})");
            let want = if v < 0 || v > LZ4_MAX_INPUT_SIZE as c_int {
                0
            } else {
                65536 + 14 + if v < 16 { 16 } else { v }
            };
            assert_eq!(x, want, "LZ4_decoderRingBufferSize({v}): expected {want}, got {x}");
        }

        // ---- srcSize ----
        // The compressors validate srcSize *before* dereferencing src
        // (lz4.c:1360, lz4hc.c:1389), so a lying value is safe -- except for
        // LZ4_MAX_INPUT_SIZE itself, which is a *legal* size and would make the
        // library read 2 GB from a 4 KB buffer.  That value is therefore skipped
        // for every function that would read the source.
        let src = gen_real(&mut rng, Shape::TextLike, 4096);
        for &ssz in &WILD_SIZES {
            if ssz == LZ4_MAX_INPUT_SIZE as c_int {
                continue; // legal size, would read 2 GB -- skipped by design
            }
            let alloc = 8192usize;
            for &cap in &[0i32, 1, 16, 8192] {
                let want = if ssz == 0 {
                    if cap >= 1 { 1 } else { 0 }
                } else {
                    0
                };
                let ctx = format!("generic srcSize default ssz={ssz} cap={cap}");
                let r = diff_compress(l, &ctx, src.as_ptr() as *const c_char, ssz, alloc, cap, None);
                assert_eq!(r, want, "{ctx}: expected {want}, got {r}");
                let ctx = format!("generic srcSize fast ssz={ssz} cap={cap}");
                let r =
                    diff_compress(l, &ctx, src.as_ptr() as *const c_char, ssz, alloc, cap, Some(1));
                assert_eq!(r, want, "{ctx}: expected {want}, got {r}");
                for &lvl in &[1i32, 9, 12] {
                    let ctx = format!("generic srcSize HC ssz={ssz} cap={cap} lvl={lvl}");
                    let (r, _) =
                        diff_hc(l, &ctx, src.as_ptr() as *const c_char, ssz, alloc, cap, lvl);
                    assert_eq!(r, want, "{ctx}: expected {want}, got {r}");
                }
            }
        }

        // ---- dstCapacity ----
        // A lying *oversized* dstCapacity is safe: the amount actually written is
        // bounded by LZ4_compressBound(srcSize), and the real allocation covers
        // that.  A negative dstCapacity only makes `olimit` unreachable, so every
        // limitedOutput check fails and 0 is returned.
        for &n in &[0usize, 1, 13, 500, 4096] {
            let s = gen_real(&mut rng, Shape::TextLike, n);
            let b = bound(l, n as c_int) as usize;
            for &cap in &WILD_SIZES {
                let ctx = format!("generic dstCapacity default n={n} cap={cap}");
                let r = diff_compress(
                    l,
                    &ctx,
                    s.as_ptr() as *const c_char,
                    n as c_int,
                    b.max(1),
                    cap,
                    None,
                );
                if cap <= 0 {
                    assert_eq!(r, 0, "{ctx}: non-positive dstCapacity must give 0, got {r}");
                } else if cap as usize >= b {
                    assert!(r > 0, "{ctx}: dstCapacity >= bound must succeed, got {r}");
                    assert!(r as usize <= b, "{ctx}: wrote {r} > bound {b}");
                }
                let ctx = format!("generic dstCapacity fast n={n} cap={cap}");
                let r = diff_compress(
                    l,
                    &ctx,
                    s.as_ptr() as *const c_char,
                    n as c_int,
                    b.max(1),
                    cap,
                    Some(1),
                );
                if cap <= 0 {
                    assert_eq!(r, 0, "{ctx}: non-positive dstCapacity must give 0, got {r}");
                }
                for &lvl in &[1i32, 9, 12] {
                    let ctx = format!("generic dstCapacity HC n={n} cap={cap} lvl={lvl}");
                    let (r, _) = diff_hc(
                        l,
                        &ctx,
                        s.as_ptr() as *const c_char,
                        n as c_int,
                        b.max(1),
                        cap,
                        lvl,
                    );
                    if cap <= 0 {
                        assert_eq!(r, 0, "{ctx}: non-positive dstCapacity must give 0, got {r}");
                    } else if cap as usize >= b {
                        assert!(r > 0, "{ctx}: dstCapacity >= bound must succeed, got {r}");
                    }
                }
            }
        }

        // ---- LZ4_decompress_safe / _partial ----
        let plain = gen_real(&mut rng, Shape::TextLike, 700);
        let comp = diff_compress_bytes(
            l,
            "generic dec seed",
            &plain,
            bound(l, 700) as usize,
            None,
        );
        assert!(comp.0 > 0);
        let blk = comp.1[..comp.0 as usize].to_vec();
        for &cs in &WILD_SIZES {
            // A lying oversized compressedSize with a *positive* dstCapacity is
            // UNSAFE: `iend = src + srcSize` and the parser would read far past
            // the block.  It is only combined with dstCapacity <= 0, where the
            // decision is taken before any read (lz4.c:2036, :2062).
            for &cap in &[0i32, -1, c_int::MIN] {
                let ctx = format!("generic dec cs={cs} cap={cap}");
                let r = diff_dec_safe(l, &ctx, &blk, cs, 4096, cap, GUARD);
                let want = if cap < 0 {
                    -1
                } else if cs == 1 && blk[0] == 0 {
                    0
                } else {
                    -1
                };
                assert_eq!(r, want, "{ctx}: expected {want}, got {r}");
                let ctx = format!("generic dec partial cs={cs} cap={cap}");
                let r = diff_dec_partial(l, &ctx, &blk, cs, cap, 4096, cap, GUARD);
                assert!(r <= 0, "{ctx}: expected <= 0, got {r}");
            }
            // Honest and small-negative compressedSizes with a real dst.
            if cs == 0 || cs == -1 {
                let ctx = format!("generic dec real dst cs={cs}");
                let r = diff_dec_safe(l, &ctx, &blk, cs, 4096, 4096, GUARD);
                assert!(r < 0, "{ctx}: expected an error, got {r}");
            }
        }
        // Oversized dstCapacity with an honest compressedSize: safe, because the
        // decoder writes only what the block encodes.
        for &cap in &[
            plain.len() as c_int,
            plain.len() as c_int + 1,
            LZ4_MAX_INPUT_SIZE as c_int,
            c_int::MAX,
        ] {
            let ctx = format!("generic dec huge cap={cap}");
            let r = diff_dec_safe(l, &ctx, &blk, blk.len() as c_int, plain.len() + 4096, cap, GUARD);
            assert_eq!(r, plain.len() as c_int, "{ctx}: expected {}, got {r}", plain.len());
            let ctx = format!("generic dec partial huge cap={cap}");
            let r = diff_dec_partial(
                l,
                &ctx,
                &blk,
                blk.len() as c_int,
                cap,
                plain.len() + 4096,
                cap,
                GUARD,
            );
            assert_eq!(r, plain.len() as c_int, "{ctx}: expected {}, got {r}", plain.len());
        }
        // ... and every extreme targetOutputSize for _partial.
        for &t in &WILD_SIZES {
            let ctx = format!("generic dec partial target={t}");
            let r = diff_dec_partial(
                l,
                &ctx,
                &blk,
                blk.len() as c_int,
                t,
                plain.len() + 4096,
                plain.len() as c_int,
                GUARD,
            );
            if t < 0 {
                assert_eq!(r, -1, "{ctx}: negative target must give -1, got {r}");
            } else {
                assert!(
                    r >= 0 && r <= plain.len() as c_int,
                    "{ctx}: nonsensical {r}"
                );
            }
        }
    }
}

// ===========================================================================
// compressionLevel / acceleration far past every documented bound
// ===========================================================================

#[test]
fn err_generic_level_and_acceleration_far_out_of_range() {
    let l = libs();
    let mut rng = Rng::new(1002);
    unsafe {
        // ERRORS.md row 5 records that LZ4F_ERROR_compressionLevel_invalid is
        // never produced: levels are CLAMPED, not rejected.  The same holds for
        // acceleration (lz4.h:233-234).  Assert the clamping is byte-identical.
        for &n in &[13usize, 300, 4096] {
            for sh in [Shape::TextLike, Shape::Compressible, Shape::Incompressible] {
                let src = gen_real(&mut rng, sh, n);
                let cap = bound(l, n as c_int) as usize;

                // --- HC compressionLevel ---
                let r9 = diff_hc(
                    l,
                    "clamp ref9",
                    src.as_ptr() as *const c_char,
                    n as c_int,
                    cap,
                    cap as c_int,
                    LZ4HC_CLEVEL_DEFAULT,
                );
                let r12 = diff_hc(
                    l,
                    "clamp ref12",
                    src.as_ptr() as *const c_char,
                    n as c_int,
                    cap,
                    cap as c_int,
                    LZ4HC_CLEVEL_MAX,
                );
                for &lvl in &WILD_LEVELS {
                    let ctx = format!("clamp HC n={n} {sh:?} lvl={lvl}");
                    let g = diff_hc(
                        l,
                        &ctx,
                        src.as_ptr() as *const c_char,
                        n as c_int,
                        cap,
                        cap as c_int,
                        lvl,
                    );
                    assert!(g.0 > 0, "{ctx}: never rejected, got {}", g.0);
                    if lvl <= 0 {
                        assert_eq!(g.0, r9.0, "{ctx}: level <= 0 == level 9");
                        same_full_buffers(&format!("{ctx} == 9"), &r9.1, &g.1);
                    }
                    if lvl >= LZ4HC_CLEVEL_MAX {
                        assert_eq!(g.0, r12.0, "{ctx}: level >= 12 == level 12");
                        same_full_buffers(&format!("{ctx} == 12"), &r12.1, &g.1);
                    }
                }
                // The obsolete level-taking wrappers clamp the same way.
                let (h2c, h2r) = l.sym::<FnCompressFast>("LZ4_compressHC2_limitedOutput");
                for &lvl in &WILD_LEVELS {
                    let mut cb = dstbuf(cap);
                    let mut rb = dstbuf(cap);
                    let a = h2c(
                        src.as_ptr() as *const c_char,
                        cb.as_mut_ptr() as *mut c_char,
                        n as c_int,
                        cap as c_int,
                        lvl,
                    );
                    let b = h2r(
                        src.as_ptr() as *const c_char,
                        rb.as_mut_ptr() as *mut c_char,
                        n as c_int,
                        cap as c_int,
                        lvl,
                    );
                    let ctx = format!("clamp LZ4_compressHC2_limitedOutput n={n} {sh:?} lvl={lvl}");
                    same_int_and_bytes(&ctx, a, b, &cb, &rb);
                    same_full_buffers(&ctx, &cb, &rb);
                    assert!(a > 0, "{ctx}: never rejected, got {a}");
                    if lvl <= 0 {
                        assert_eq!(a, r9.0, "{ctx}: level <= 0 == level 9");
                        same_full_buffers(&format!("{ctx} == 9"), &r9.1, &cb);
                    }
                    if lvl >= LZ4HC_CLEVEL_MAX {
                        assert_eq!(a, r12.0, "{ctx}: level >= 12 == level 12");
                        same_full_buffers(&format!("{ctx} == 12"), &r12.1, &cb);
                    }
                }

                // --- acceleration ---
                let a1 = diff_compress_bytes(l, "clamp accel 1", &src, cap, Some(1));
                let amax = diff_compress_bytes(
                    l,
                    "clamp accel max",
                    &src,
                    cap,
                    Some(LZ4_ACCELERATION_MAX),
                );
                for &acc in &WILD_LEVELS {
                    let ctx = format!("clamp accel n={n} {sh:?} accel={acc}");
                    let g = diff_compress_bytes(l, &ctx, &src, cap, Some(acc));
                    assert!(g.0 > 0, "{ctx}: never rejected, got {}", g.0);
                    if acc < 1 {
                        assert_eq!(g.0, a1.0, "{ctx}: accel < 1 == accel 1");
                        same_full_buffers(&format!("{ctx} == 1"), &a1.1, &g.1);
                    }
                    if acc > LZ4_ACCELERATION_MAX {
                        assert_eq!(g.0, amax.0, "{ctx}: accel > 65537 == accel 65537");
                        same_full_buffers(&format!("{ctx} == 65537"), &amax.1, &g.1);
                    }
                }
                // 65537 is the last distinct value; 65536 must differ from it
                // (otherwise the clamp boundary would be untested).
                let a65536 = diff_compress_bytes(l, "clamp accel 65536", &src, cap, Some(65536));
                let a65537 = diff_compress_bytes(l, "clamp accel 65537", &src, cap, Some(65537));
                assert_eq!(
                    a65537.0, amax.0,
                    "clamp: accel 65537 is LZ4_ACCELERATION_MAX"
                );
                assert!(a65536.0 > 0 && a65537.0 > 0);
            }
        }
    }
}

// ===========================================================================
// Undersized and misaligned caller-provided state buffers
// ===========================================================================

#[test]
fn err_generic_undersized_and_misaligned_state_buffers() {
    let l = libs();
    let mut rng = Rng::new(1003);
    unsafe {
        let ss = sizeof_state(l);
        let hss = sizeof_state_hc(l);
        let src = gen_real(&mut rng, Shape::TextLike, 2000);
        let mut inbuf = gen_real(&mut rng, Shape::Degenerate, 64);
        let inptr = inbuf.as_mut_ptr() as *mut c_char;

        // ---- LZ4_initStream / LZ4_initStreamHC: the only two functions that
        //      report *all three* problems (NULL, too small, misaligned).
        for (name, sz) in [("LZ4_initStream", ss), ("LZ4_initStreamHC", hss)] {
            let (ic, ir) = l.sym::<FnInitStream>(name);
            let mut cs = Scratch::new(sz + 16);
            let mut rs = Scratch::new(sz + 16);
            for &(p_null, size, off) in &[
                (true, sz, 0usize),
                (false, 0, 0),
                (false, sz - 1, 0),
                (false, sz, 1),
                (false, sz, 4),
                (false, sz - 1, 3),
            ] {
                let (a, b) = if p_null {
                    (ic(std::ptr::null_mut(), size), ir(std::ptr::null_mut(), size))
                } else {
                    (ic(cs.at(off), size), ir(rs.at(off), size))
                };
                let ctx = format!("{name} null={p_null} size={size} off={off}");
                assert!(a.is_null(), "{ctx}: C must return NULL, got {a:?}");
                assert!(b.is_null(), "{ctx}: Rust must return NULL, got {b:?}");
            }
        }

        // ---- LZ4_compress_fast_extState: validates via LZ4_initStream, but the
        //      returned NULL is then used as the context.  With srcSize == 0
        //      LZ4_compress_generic answers before touching the context
        //      (lz4.c:1361-1372), so the rejection is observable without an
        //      invalid dereference.  A nonzero srcSize with a rejected state
        //      WOULD dereference NULL, so it is never attempted.
        {
            let (fc, fr) = l.sym::<FnExtState>("LZ4_compress_fast_extState");
            let mut cs = Scratch::new(ss + 16);
            let mut rs = Scratch::new(ss + 16);
            for &cap in &[0i32, 1, 16, 64] {
                for &off in &[1usize, 4, 7] {
                    let mut cb = dstbuf(64);
                    let mut rb = dstbuf(64);
                    let a = fc(
                        cs.at(off),
                        src.as_ptr() as *const c_char,
                        cb.as_mut_ptr() as *mut c_char,
                        0,
                        cap,
                        1,
                    );
                    let b = fr(
                        rs.at(off),
                        src.as_ptr() as *const c_char,
                        rb.as_mut_ptr() as *mut c_char,
                        0,
                        cap,
                        1,
                    );
                    let ctx = format!("extState misaligned off={off} cap={cap}");
                    same_int_and_bytes(&ctx, a, b, &cb, &rb);
                    same_full_buffers(&ctx, &cb, &rb);
                    assert_eq!(
                        a,
                        if cap >= 1 { 1 } else { 0 },
                        "{ctx}: unexpected {a}"
                    );
                }
                let mut cb = dstbuf(64);
                let mut rb = dstbuf(64);
                let a = fc(
                    std::ptr::null_mut(),
                    src.as_ptr() as *const c_char,
                    cb.as_mut_ptr() as *mut c_char,
                    0,
                    cap,
                    1,
                );
                let b = fr(
                    std::ptr::null_mut(),
                    src.as_ptr() as *const c_char,
                    rb.as_mut_ptr() as *mut c_char,
                    0,
                    cap,
                    1,
                );
                let ctx = format!("extState NULL state cap={cap}");
                same_int_and_bytes(&ctx, a, b, &cb, &rb);
                same_full_buffers(&ctx, &cb, &rb);
                assert_eq!(a, if cap >= 1 { 1 } else { 0 }, "{ctx}: unexpected {a}");
            }
        }

        // ---- LZ4_compress_destSize_extState: same structure (it calls
        //      LZ4_initStream and ignores the NULL), same srcSize == 0 restriction.
        {
            let (fc, fr) = l.sym::<FnDestSizeExtState>("LZ4_compress_destSize_extState");
            let mut cs = Scratch::new(ss + 16);
            let mut rs = Scratch::new(ss + 16);
            for &target in &[0i32, 1, 16, 64] {
                for off in [0usize, 1, 4] {
                    let mut cb = dstbuf(64);
                    let mut rb = dstbuf(64);
                    let mut cn = 0i32;
                    let mut rn = 0i32;
                    let (a, b) = if off == 0 {
                        (
                            fc(
                                std::ptr::null_mut(),
                                src.as_ptr() as *const c_char,
                                cb.as_mut_ptr() as *mut c_char,
                                &mut cn,
                                target,
                                1,
                            ),
                            fr(
                                std::ptr::null_mut(),
                                src.as_ptr() as *const c_char,
                                rb.as_mut_ptr() as *mut c_char,
                                &mut rn,
                                target,
                                1,
                            ),
                        )
                    } else {
                        (
                            fc(
                                cs.at(off),
                                src.as_ptr() as *const c_char,
                                cb.as_mut_ptr() as *mut c_char,
                                &mut cn,
                                target,
                                1,
                            ),
                            fr(
                                rs.at(off),
                                src.as_ptr() as *const c_char,
                                rb.as_mut_ptr() as *mut c_char,
                                &mut rn,
                                target,
                                1,
                            ),
                        )
                    };
                    let ctx = format!("destSize_extState off={off} target={target}");
                    same_int_and_bytes(&ctx, a, b, &cb, &rb);
                    same_full_buffers(&ctx, &cb, &rb);
                    assert_eq!(cn, rn, "{ctx}: *srcSizePtr mismatch (C={cn} Rust={rn})");
                    assert_eq!(a, if target >= 1 { 1 } else { 0 }, "{ctx}: unexpected {a}");
                }
            }
        }

        // ---- LZ4_compress_fast_extState_fastReset: performs NO validation at
        //      all ("state is presumed correctly initialized", lz4.c:1409-1417).
        //      There is therefore no sentinel to assert: a NULL state would have
        //      LZ4_prepareTable dereference it and an undersized buffer would be
        //      an out-of-bounds MEM_INIT.  Neither is attempted.  What is pinned
        //      here is the in-contract path: a correctly initialised, correctly
        //      aligned, correctly sized state.
        {
            let (ic, ir) = l.sym::<FnInitStream>("LZ4_initStream");
            let (fc, fr) = l.sym::<FnExtState>("LZ4_compress_fast_extState_fastReset");
            let mut cs = Scratch::new(ss);
            let mut rs = Scratch::new(ss);
            assert!(!ic(cs.ptr(), ss).is_null());
            assert!(!ir(rs.ptr(), ss).is_null());
            let cap = bound(l, 2000) as usize;
            for k in 0..4 {
                let mut cb = dstbuf(cap);
                let mut rb = dstbuf(cap);
                let a = fc(
                    cs.ptr(),
                    src.as_ptr() as *const c_char,
                    cb.as_mut_ptr() as *mut c_char,
                    2000,
                    cap as c_int,
                    1,
                );
                let b = fr(
                    rs.ptr(),
                    src.as_ptr() as *const c_char,
                    rb.as_mut_ptr() as *mut c_char,
                    2000,
                    cap as c_int,
                    1,
                );
                let ctx = format!("fastReset in-contract k={k}");
                same_int_and_bytes(&ctx, a, b, &cb, &rb);
                same_full_buffers(&ctx, &cb, &rb);
                same_full_buffers(&format!("{ctx} [state]"), cs.bytes(ss), rs.bytes(ss));
                assert!(a > 0, "{ctx}: expected success, got {a}");
            }
        }

        // ---- LZ4_compress_HC_extStateHC / _fastReset ----
        // extStateHC validates through LZ4_initStreamHC (-> 0 for NULL and for
        // misaligned).  _fastReset only checks the alignment (-> 0); NULL passes
        // its LZ4_isAligned test (0 % 8 == 0) and would then be dereferenced, so
        // NULL is not passed to it.
        {
            let cap = bound(l, 2000) as usize;
            let mut cs = Scratch::new(hss + 16);
            let mut rs = Scratch::new(hss + 16);
            let (ec, er) = l.sym::<FnExtState>("LZ4_compress_HC_extStateHC");
            let (rc2, rr2) = l.sym::<FnExtState>("LZ4_compress_HC_extStateHC_fastReset");
            for &off in &MISALIGN {
                for (name, f) in [("extStateHC", 0usize), ("extStateHC_fastReset", 1usize)] {
                    let mut cb = dstbuf(cap);
                    let mut rb = dstbuf(cap);
                    let (a, b) = if f == 0 {
                        (
                            ec(
                                cs.at(off),
                                src.as_ptr() as *const c_char,
                                cb.as_mut_ptr() as *mut c_char,
                                2000,
                                cap as c_int,
                                9,
                            ),
                            er(
                                rs.at(off),
                                src.as_ptr() as *const c_char,
                                rb.as_mut_ptr() as *mut c_char,
                                2000,
                                cap as c_int,
                                9,
                            ),
                        )
                    } else {
                        (
                            rc2(
                                cs.at(off),
                                src.as_ptr() as *const c_char,
                                cb.as_mut_ptr() as *mut c_char,
                                2000,
                                cap as c_int,
                                9,
                            ),
                            rr2(
                                rs.at(off),
                                src.as_ptr() as *const c_char,
                                rb.as_mut_ptr() as *mut c_char,
                                2000,
                                cap as c_int,
                                9,
                            ),
                        )
                    };
                    let ctx = format!("HC {name} misaligned off={off}");
                    same_int_and_bytes(&ctx, a, b, &cb, &rb);
                    same_full_buffers(&ctx, &cb, &rb);
                    assert_eq!(a, 0, "{ctx}: expected 0, got {a}");
                }
            }
            let mut cb = dstbuf(cap);
            let mut rb = dstbuf(cap);
            let a = ec(
                std::ptr::null_mut(),
                src.as_ptr() as *const c_char,
                cb.as_mut_ptr() as *mut c_char,
                2000,
                cap as c_int,
                9,
            );
            let b = er(
                std::ptr::null_mut(),
                src.as_ptr() as *const c_char,
                rb.as_mut_ptr() as *mut c_char,
                2000,
                cap as c_int,
                9,
            );
            same_int_and_bytes("HC extStateHC NULL", a, b, &cb, &rb);
            assert_eq!(a, 0, "HC extStateHC NULL: expected 0, got {a}");
        }

        // ---- LZ4_compress_HC_destSize (validates through LZ4_initStreamHC) ----
        {
            let (fc, fr) = l.sym::<FnDestSizeExtState>("LZ4_compress_HC_destSize");
            let mut cs = Scratch::new(hss + 16);
            let mut rs = Scratch::new(hss + 16);
            for &off in &MISALIGN {
                let mut cb = dstbuf(4096);
                let mut rb = dstbuf(4096);
                let mut cn = 2000i32;
                let mut rn = 2000i32;
                let a = fc(
                    cs.at(off),
                    src.as_ptr() as *const c_char,
                    cb.as_mut_ptr() as *mut c_char,
                    &mut cn,
                    4096,
                    9,
                );
                let b = fr(
                    rs.at(off),
                    src.as_ptr() as *const c_char,
                    rb.as_mut_ptr() as *mut c_char,
                    &mut rn,
                    4096,
                    9,
                );
                let ctx = format!("HC destSize misaligned off={off}");
                same_int_and_bytes(&ctx, a, b, &cb, &rb);
                same_full_buffers(&ctx, &cb, &rb);
                assert_eq!(cn, rn, "{ctx}: *sourceSizePtr mismatch");
                assert_eq!(a, 0, "{ctx}: expected 0, got {a}");
            }
        }

        // ---- LZ4_resetStreamState / LZ4_resetStreamStateHC ----
        // The HC variant reports failures with the INVERTED convention
        // (1 = failure).  The non-HC variant performs no validation at all and
        // always returns 0, so only a large-but-misaligned buffer is probed.
        {
            let (hc, hr) = l.sym::<FnResetStreamState>("LZ4_resetStreamStateHC");
            let mut cs = Scratch::new(hss + 16);
            let mut rs = Scratch::new(hss + 16);
            let a = hc(std::ptr::null_mut(), inptr);
            let b = hr(std::ptr::null_mut(), inptr);
            assert_eq!(a, b, "resetStreamStateHC NULL: mismatch (C={a} Rust={b})");
            assert_eq!(a, 1, "resetStreamStateHC NULL: expected 1, got {a}");
            for &off in &MISALIGN {
                let a = hc(cs.at(off), inptr);
                let b = hr(rs.at(off), inptr);
                assert_eq!(a, b, "resetStreamStateHC off={off}: mismatch (C={a} Rust={b})");
                assert_eq!(a, 1, "resetStreamStateHC off={off}: expected 1, got {a}");
            }
            let a = hc(cs.ptr(), inptr);
            let b = hr(rs.ptr(), inptr);
            assert_eq!(a, b, "resetStreamStateHC valid: mismatch");
            assert_eq!(a, 0, "resetStreamStateHC valid: expected 0, got {a}");

            let (nc, nr) = l.sym::<FnResetStreamState>("LZ4_resetStreamState");
            let mut cs2 = Scratch::new(ss + 16);
            let mut rs2 = Scratch::new(ss + 16);
            for &off in &[0usize, 1, 4, 8] {
                let a = nc(cs2.at(off), inptr);
                let b = nr(rs2.at(off), inptr);
                assert_eq!(a, b, "resetStreamState off={off}: mismatch (C={a} Rust={b})");
                assert_eq!(a, 0, "resetStreamState off={off}: expected 0, got {a}");
            }
        }

        // ---- The deprecated *_withState* wrappers ----
        // The lz4 ones forward to LZ4_compress_fast_extState, so an invalid
        // state is only safe with srcSize == 0 (see above).  The HC ones forward
        // to LZ4_compress_HC_extStateHC, which validates properly and returns 0
        // for any srcSize.
        {
            let cap = bound(l, 2000) as usize;
            let mut cs = Scratch::new(hss + 16);
            let mut rs = Scratch::new(hss + 16);

            let (w4c, w4r) = l.sym::<FnWithState4>("LZ4_compress_withState");
            for st in [0usize, 1, 4] {
                let mut cb = dstbuf(64);
                let mut rb = dstbuf(64);
                let (a, b) = if st == 0 {
                    (
                        w4c(std::ptr::null_mut(), src.as_ptr() as *const c_char, cb.as_mut_ptr() as *mut c_char, 0),
                        w4r(std::ptr::null_mut(), src.as_ptr() as *const c_char, rb.as_mut_ptr() as *mut c_char, 0),
                    )
                } else {
                    (
                        w4c(cs.at(st), src.as_ptr() as *const c_char, cb.as_mut_ptr() as *mut c_char, 0),
                        w4r(rs.at(st), src.as_ptr() as *const c_char, rb.as_mut_ptr() as *mut c_char, 0),
                    )
                };
                let ctx = format!("LZ4_compress_withState bad state st={st}");
                same_int_and_bytes(&ctx, a, b, &cb, &rb);
                same_full_buffers(&ctx, &cb, &rb);
                assert_eq!(a, 1, "{ctx}: bound(0)==16 > 0, so the empty block is written");
            }
            let (w5c, w5r) = l.sym::<FnWithState5>("LZ4_compress_limitedOutput_withState");
            for st in [0usize, 1, 4] {
                for &ds in &[0i32, 1, 16] {
                    let mut cb = dstbuf(64);
                    let mut rb = dstbuf(64);
                    let (a, b) = if st == 0 {
                        (
                            w5c(std::ptr::null_mut(), src.as_ptr() as *const c_char, cb.as_mut_ptr() as *mut c_char, 0, ds),
                            w5r(std::ptr::null_mut(), src.as_ptr() as *const c_char, rb.as_mut_ptr() as *mut c_char, 0, ds),
                        )
                    } else {
                        (
                            w5c(cs.at(st), src.as_ptr() as *const c_char, cb.as_mut_ptr() as *mut c_char, 0, ds),
                            w5r(rs.at(st), src.as_ptr() as *const c_char, rb.as_mut_ptr() as *mut c_char, 0, ds),
                        )
                    };
                    let ctx = format!("LZ4_compress_limitedOutput_withState st={st} ds={ds}");
                    same_int_and_bytes(&ctx, a, b, &cb, &rb);
                    same_full_buffers(&ctx, &cb, &rb);
                    assert_eq!(a, if ds >= 1 { 1 } else { 0 }, "{ctx}: unexpected {a}");
                }
            }
            // HC wrappers: exact `0` sentinel from both, for a full-size input.
            let (hw4c, hw4r) = l.sym::<FnWithState4>("LZ4_compressHC_withStateHC");
            for st in [0usize, 1, 4, 7] {
                let mut cb = dstbuf(cap);
                let mut rb = dstbuf(cap);
                let (a, b) = if st == 0 {
                    (
                        hw4c(std::ptr::null_mut(), src.as_ptr() as *const c_char, cb.as_mut_ptr() as *mut c_char, 2000),
                        hw4r(std::ptr::null_mut(), src.as_ptr() as *const c_char, rb.as_mut_ptr() as *mut c_char, 2000),
                    )
                } else {
                    (
                        hw4c(cs.at(st), src.as_ptr() as *const c_char, cb.as_mut_ptr() as *mut c_char, 2000),
                        hw4r(rs.at(st), src.as_ptr() as *const c_char, rb.as_mut_ptr() as *mut c_char, 2000),
                    )
                };
                let ctx = format!("LZ4_compressHC_withStateHC st={st}");
                same_int_and_bytes(&ctx, a, b, &cb, &rb);
                same_full_buffers(&ctx, &cb, &rb);
                assert_eq!(a, 0, "{ctx}: expected 0, got {a}");
            }
            for name in [
                "LZ4_compressHC_limitedOutput_withStateHC",
                "LZ4_compressHC2_withStateHC",
            ] {
                let (fc5, fr5) = l.sym::<FnWithState5>(name);
                for st in [0usize, 1, 4, 7] {
                    let mut cb = dstbuf(cap);
                    let mut rb = dstbuf(cap);
                    // 5th argument is maxDstSize for the first name and cLevel
                    // for the second; both give 0 for an invalid state.
                    let arg = if name.contains("HC2") { 9 } else { cap as c_int };
                    let (a, b) = if st == 0 {
                        (
                            fc5(std::ptr::null_mut(), src.as_ptr() as *const c_char, cb.as_mut_ptr() as *mut c_char, 2000, arg),
                            fr5(std::ptr::null_mut(), src.as_ptr() as *const c_char, rb.as_mut_ptr() as *mut c_char, 2000, arg),
                        )
                    } else {
                        (
                            fc5(cs.at(st), src.as_ptr() as *const c_char, cb.as_mut_ptr() as *mut c_char, 2000, arg),
                            fr5(rs.at(st), src.as_ptr() as *const c_char, rb.as_mut_ptr() as *mut c_char, 2000, arg),
                        )
                    };
                    let ctx = format!("{name} st={st}");
                    same_int_and_bytes(&ctx, a, b, &cb, &rb);
                    same_full_buffers(&ctx, &cb, &rb);
                    assert_eq!(a, 0, "{ctx}: expected 0, got {a}");
                }
            }
            let (fc6, fr6) =
                l.sym::<FnWithState6>("LZ4_compressHC2_limitedOutput_withStateHC");
            for st in [0usize, 1, 4, 7] {
                for &lvl in &[1i32, 9, 12, 0, 13] {
                    let mut cb = dstbuf(cap);
                    let mut rb = dstbuf(cap);
                    let (a, b) = if st == 0 {
                        (
                            fc6(std::ptr::null_mut(), src.as_ptr() as *const c_char, cb.as_mut_ptr() as *mut c_char, 2000, cap as c_int, lvl),
                            fr6(std::ptr::null_mut(), src.as_ptr() as *const c_char, rb.as_mut_ptr() as *mut c_char, 2000, cap as c_int, lvl),
                        )
                    } else {
                        (
                            fc6(cs.at(st), src.as_ptr() as *const c_char, cb.as_mut_ptr() as *mut c_char, 2000, cap as c_int, lvl),
                            fr6(rs.at(st), src.as_ptr() as *const c_char, rb.as_mut_ptr() as *mut c_char, 2000, cap as c_int, lvl),
                        )
                    };
                    let ctx = format!("LZ4_compressHC2_limitedOutput_withStateHC st={st} lvl={lvl}");
                    same_int_and_bytes(&ctx, a, b, &cb, &rb);
                    same_full_buffers(&ctx, &cb, &rb);
                    assert_eq!(a, 0, "{ctx}: expected 0, got {a}");
                }
            }
        }
    }
}

// ===========================================================================
// Large randomized corruption fuzz over every decoder entry point
// ===========================================================================

#[test]
fn err_generic_fuzz_decoder_return_values() {
    let l = libs();
    let mut rng = Rng::new(2024);
    unsafe {
        // Pool of (block, plaintext length) pairs: real compressor output plus
        // hand-built blocks with unusual token/extension shapes.
        let mut pool: Vec<(Vec<u8>, usize)> = Vec::new();
        for _ in 0..80 {
            let n = rng.range(1, 2500);
            let sh = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
            let src = gen_real(&mut rng, sh, n);
            let c = diff_compress_bytes(l, "fuzz pool", &src, bound(l, n as c_int) as usize, None);
            assert!(c.0 > 0);
            pool.push((c.1[..c.0 as usize].to_vec(), n));
        }
        for _ in 0..40 {
            let n = rng.range(1, 800);
            let sh = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
            let src = gen_real(&mut rng, sh, n);
            let lvl = [1i32, 2, 6, 9, 12][rng.below(5)];
            let (r, cb) = diff_hc(
                l,
                "fuzz pool HC",
                src.as_ptr() as *const c_char,
                n as c_int,
                bound(l, n as c_int) as usize,
                bound(l, n as c_int),
                lvl,
            );
            assert!(r > 0);
            pool.push((cb[..r as usize].to_vec(), n));
        }
        for _ in 0..80 {
            let nseq = rng.range(1, 20);
            let (comp, plain) = well_formed(&mut rng, nseq);
            pool.push((comp, plain.len()));
        }

        let mut safe_hist: BTreeMap<c_int, usize> = BTreeMap::new();
        let mut partial_hist: BTreeMap<c_int, usize> = BTreeMap::new();
        let mut fast_hist: BTreeMap<c_int, usize> = BTreeMap::new();
        let mut kind_counts = [0usize; 8];

        const ITERS: usize = 400_000;
        for iter in 0..ITERS {
            let (base, orig) = pool[rng.below(pool.len())].clone();
            let mut b = base.clone();
            let kind = rng.below(8);
            kind_counts[kind] += 1;
            match kind {
                0 => {
                    // Single random byte replacement.
                    if !b.is_empty() {
                        let k = rng.below(b.len());
                        b[k] = rng.byte();
                    }
                }
                1 => {
                    // A handful of single-bit flips.
                    for _ in 0..rng.range(1, 5) {
                        if b.is_empty() {
                            break;
                        }
                        let i = rng.below(b.len());
                        let s = rng.below(8);
                        b[i] ^= 1u8 << s;
                    }
                }
                2 => {
                    // Truncation (input exhausted mid-sequence / mid-length).
                    let t = rng.below(b.len() + 1);
                    b.truncate(t);
                }
                3 => {
                    // Zeroed run: manufactures offset-0 matches and 0 tokens.
                    if !b.is_empty() {
                        let k = rng.below(b.len());
                        let n = rng.range(1, 6).min(b.len() - k);
                        for j in 0..n {
                            b[k + j] = 0;
                        }
                    }
                }
                4 => {
                    // Oversized length token: force a nibble to RUN_MASK and
                    // splice in a 255-extension chain.
                    if !b.is_empty() {
                        let k = rng.below(b.len());
                        b[k] |= if rng.below(2) == 0 { 0xF0 } else { 0x0F };
                        let mut ext: Vec<u8> = Vec::new();
                        for _ in 0..rng.range(1, 6) {
                            ext.push(255);
                        }
                        if rng.below(2) == 0 {
                            ext.push(rng.byte());
                        }
                        let at = (k + 1).min(b.len());
                        for (j, e) in ext.into_iter().enumerate() {
                            b.insert(at + j, e);
                        }
                    }
                }
                5 => {
                    // Violated last-literals rule: chop the final literal run
                    // down to fewer than LASTLITERALS bytes.
                    let cut = rng.range(1, 5).min(b.len());
                    let n = b.len() - cut;
                    b.truncate(n);
                }
                6 => {
                    // Violated last-match rule: extend a match offset far past
                    // what the produced output can support.
                    if b.len() >= 4 {
                        let k = rng.below(b.len() - 2);
                        let off = rng.range(1, 65535);
                        b[k] = (off & 0xFF) as u8;
                        b[k + 1] = ((off >> 8) & 0xFF) as u8;
                    }
                }
                _ => {
                    // Pure garbage of a random length.
                    let n = rng.range(0, 64);
                    b = (0..n).map(|_| rng.byte()).collect();
                }
            }

            // --- LZ4_decompress_safe, and the obsolete
            //     LZ4_uncompress_unknownOutputSize which forwards to it ---
            let cap = orig + rng.range(0, 200);
            let csize = if rng.below(4) == 0 {
                rng.range(0, b.len() + 4) as c_int
            } else {
                b.len() as c_int
            };
            let ctx = format!("fuzz iter={iter} kind={kind} clen={} cap={cap} cs={csize}", b.len());
            let rs = diff_dec_safe(l, &ctx, &b, csize, cap, cap as c_int, GUARD + 64);
            *safe_hist.entry(rs).or_insert(0) += 1;
            let ru = diff_uncompress_unknown(
                l,
                &format!("{ctx} [unknownOutputSize]"),
                &b,
                csize,
                cap,
                cap as c_int,
                GUARD + 64,
            );
            assert_eq!(
                rs, ru,
                "{ctx}: LZ4_uncompress_unknownOutputSize ({ru}) must equal LZ4_decompress_safe ({rs})"
            );

            // --- LZ4_decompress_safe_partial ---
            let target = rng.below(orig + 8) as c_int;
            let rp = diff_dec_partial(
                l,
                &format!("{ctx} [partial t={target}]"),
                &b,
                csize,
                target,
                cap,
                cap as c_int,
                GUARD + 64,
            );
            *partial_hist.entry(rp).or_insert(0) += 1;

            // --- Tight destination buffers ---
            if iter % 3 == 0 {
                let tight = rng.below(orig + 2);
                let r = diff_dec_safe(
                    l,
                    &format!("{ctx} [tight={tight}]"),
                    &b,
                    csize,
                    tight,
                    tight as c_int,
                    GUARD + 64,
                );
                *safe_hist.entry(r).or_insert(0) += 1;
            }

            // --- LZ4_decompress_fast + LZ4_uncompress ---
            // `diff_dec_fast` pads the input with `2*orig + 512` zero bytes; the
            // decoder consumes a bounded number of input bytes per >= 4 output
            // bytes produced, so every read stays inside that padding.  Only
            // non-negative originalSize values are used (there is no check for a
            // negative one in LZ4_decompress_unsafe_generic).
            if iter % 2 == 0 {
                let o = if rng.below(4) == 0 {
                    rng.below(orig + 4) as c_int
                } else {
                    orig as c_int
                };
                let r = diff_dec_fast(l, &format!("{ctx} [fast o={o}]"), &b, o);
                *fast_hist.entry(r).or_insert(0) += 1;
            }
        }

        // --- Report the observed distributions so the fuzz is demonstrably
        //     non-vacuous. ---
        let summarize = |name: &str, h: &BTreeMap<c_int, usize>| {
            let total: usize = h.values().sum();
            let distinct = h.len();
            let ok = h.iter().filter(|(k, _)| **k > 0).map(|(_, v)| *v).sum::<usize>();
            let minus1 = *h.get(&-1).unwrap_or(&0);
            let zero = *h.get(&0).unwrap_or(&0);
            let neg_off = h
                .iter()
                .filter(|(k, _)| **k < -1)
                .map(|(_, v)| *v)
                .sum::<usize>();
            let distinct_neg_off = h.keys().filter(|k| **k < -1).count();
            let lo = h.keys().next().copied().unwrap_or(0);
            let hi = h.keys().next_back().copied().unwrap_or(0);
            println!(
                "[fuzz] {name}: {total} calls, {distinct} distinct return values in [{lo}, {hi}]\n\
                           success(>0)={ok}  zero={zero}  minus_one={minus1}  \
                 negative_offsets={neg_off} ({distinct_neg_off} distinct)"
            );
            (total, distinct, ok, minus1, neg_off, distinct_neg_off)
        };
        let (t1, d1, ok1, m1, no1, dno1) = summarize("LZ4_decompress_safe", &safe_hist);
        let (t2, d2, _ok2, _m2, _no2, _dno2) = summarize("LZ4_decompress_safe_partial", &partial_hist);
        let (t3, d3, ok3, m3, _no3, _dno3) = summarize("LZ4_decompress_fast", &fast_hist);
        println!("[fuzz] mutation kinds: {kind_counts:?}");
        // Top negative offsets actually observed, for the record.
        let mut top: Vec<(c_int, usize)> =
            safe_hist.iter().filter(|(k, _)| **k < -1).map(|(k, v)| (*k, *v)).collect();
        top.sort_by(|a, b| b.1.cmp(&a.1));
        top.truncate(12);
        println!("[fuzz] most frequent LZ4_decompress_safe negative offsets: {top:?}");

        // Non-vacuousness: all three conventions must have been observed many
        // times, and the negative-offset encoding must span many positions.
        assert!(t1 > 400_000, "fuzz: only {t1} LZ4_decompress_safe calls");
        assert!(t2 > 300_000, "fuzz: only {t2} partial calls");
        assert!(t3 > 150_000, "fuzz: only {t3} LZ4_decompress_fast calls");
        assert!(d1 > 200, "fuzz: only {d1} distinct LZ4_decompress_safe values");
        assert!(d2 > 50, "fuzz: only {d2} distinct partial values");
        assert!(d3 >= 2, "fuzz: only {d3} distinct LZ4_decompress_fast values");
        assert!(ok1 > 1000, "fuzz: only {ok1} successful decodes");
        assert!(m1 > 100, "fuzz: only {m1} plain -1 rejections");
        assert!(no1 > 10_000, "fuzz: only {no1} negative-offset rejections");
        assert!(
            dno1 > 100,
            "fuzz: only {dno1} distinct negative offsets -- parse positions barely vary"
        );
        assert!(ok3 > 500, "fuzz: only {ok3} successful LZ4_decompress_fast decodes");
        assert!(m3 > 1000, "fuzz: only {m3} LZ4_decompress_fast rejections");
    }
}
