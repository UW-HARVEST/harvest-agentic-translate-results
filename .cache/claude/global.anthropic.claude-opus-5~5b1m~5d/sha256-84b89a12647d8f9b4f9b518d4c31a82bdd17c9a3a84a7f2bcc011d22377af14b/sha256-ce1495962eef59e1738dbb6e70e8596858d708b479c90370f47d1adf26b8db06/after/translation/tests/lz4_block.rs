//! Differential tests for CONFIGS.md rows 1-34:
//! "lz4 block — one-shot compression" (1-21) and "lz4 block — decompression" (22-34).
//!
//! Every call goes through a `.so` export looked up with libloading, once for the
//! C library and once for the Rust library. C and Rust always get their own
//! destination buffer, pre-filled with the 0xCD sentinel, and both the return
//! value and the *whole* buffer (including a trailing guard region) are compared.
#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::CStr;
use std::os::raw::{c_char, c_int, c_void};

// ---------------------------------------------------------------------------
// Local FFI signature aliases
// ---------------------------------------------------------------------------

type FnCompressFastExtState = unsafe extern "C" fn(
    *mut c_void,
    *const c_char,
    *mut c_char,
    c_int,
    c_int,
    c_int,
) -> c_int;
type FnCompressDestSizeExtState = unsafe extern "C" fn(
    *mut c_void,
    *const c_char,
    *mut c_char,
    *mut c_int,
    c_int,
    c_int,
) -> c_int;
type FnInitStream = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
type FnVersionString = unsafe extern "C" fn() -> *const c_char;
type FnUncompress = unsafe extern "C" fn(*const c_char, *mut c_char, c_int) -> c_int;
type FnUncompressUnknown =
    unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int) -> c_int;

// ---------------------------------------------------------------------------
// Constants mirrored from lz4.c
// ---------------------------------------------------------------------------

const SENT: u8 = 0xCD;
/// Extra bytes appended to every destination buffer; must stay untouched (and,
/// more importantly, must be scribbled *identically* by both implementations).
const GUARD: usize = 64;
const LZ4_64KLIMIT: usize = 65547;
const FASTLOOP_SAFE_DISTANCE: usize = 64;
const LZ4_ACCELERATION_MAX: c_int = 65537;

// ---------------------------------------------------------------------------
// Small utilities
// ---------------------------------------------------------------------------

fn dstbuf(cap: usize) -> Vec<u8> {
    vec![SENT; cap + GUARD]
}

/// A copy of `b` with `GUARD` zero bytes appended, so that any (C or Rust)
/// speculative over-read past the logical end of a compressed block stays
/// inside an allocation we own.
fn padded(b: &[u8]) -> Vec<u8> {
    let mut v = Vec::with_capacity(b.len() + GUARD);
    v.extend_from_slice(b);
    v.resize(b.len() + GUARD, 0);
    v
}

/// 8-byte aligned, sentinel-filled scratch buffer used for caller-allocated
/// `LZ4_stream_t` state. One of these is obtained *separately* per library.
struct State {
    v: Vec<u64>,
    n: usize,
}

impl State {
    fn new(n: usize) -> State {
        let words = n / 8 + 2;
        State {
            v: vec![0xCDCD_CDCD_CDCD_CDCDu64; words],
            n,
        }
    }
    fn ptr(&mut self) -> *mut c_void {
        self.v.as_mut_ptr() as *mut c_void
    }
    /// Raw pointer offset by `off` bytes (for the misalignment tests).
    fn ptr_off(&mut self, off: usize) -> *mut c_void {
        unsafe { (self.v.as_mut_ptr() as *mut u8).add(off) as *mut c_void }
    }
    fn bytes(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.v.as_ptr() as *const u8, self.n) }
    }
}

fn sizeof_state() -> usize {
    let l = libs();
    unsafe {
        let (c, r) = l.sym::<FnVoidToInt>("LZ4_sizeofState");
        let a = c();
        let b = r();
        assert_eq!(a, b, "LZ4_sizeofState mismatch (C={a} Rust={b})");
        assert!(a > 0);
        a as usize
    }
}

fn bound(n: c_int) -> c_int {
    let l = libs();
    unsafe {
        let (c, r) = l.sym::<FnCompressBound>("LZ4_compressBound");
        let a = c(n);
        let b = r(n);
        assert_eq!(a, b, "LZ4_compressBound({n}) mismatch (C={a} Rust={b})");
        a
    }
}

fn ubound(n: usize) -> usize {
    bound(n as c_int) as usize
}

/// `LZ4_initStream` on two freshly allocated states, one per library.
/// Asserts both succeeded and that the zeroed state images are identical.
fn init_states(n: usize) -> (State, State) {
    let l = libs();
    let mut cs = State::new(n);
    let mut rs = State::new(n);
    unsafe {
        let (c, r) = l.sym::<FnInitStream>("LZ4_initStream");
        let cp = cs.ptr();
        let rp = rs.ptr();
        let a = c(cp, n);
        let b = r(rp, n);
        assert_eq!(a, cp, "C LZ4_initStream did not return its buffer");
        assert_eq!(b, rp, "Rust LZ4_initStream did not return its buffer");
    }
    same_full_buffers("LZ4_initStream freshly-initialised state", cs.bytes(), rs.bytes());
    (cs, rs)
}

// ---------------------------------------------------------------------------
// Differential drivers
// ---------------------------------------------------------------------------

/// `LZ4_compress_default` (accel == None) or `LZ4_compress_fast` (accel == Some).
/// Returns `(ret, c_dst, rust_dst)`.
fn diff_compress_raw(
    ctx: &str,
    src: *const c_char,
    src_size: c_int,
    cap: usize,
    accel: Option<c_int>,
) -> (c_int, Vec<u8>, Vec<u8>) {
    let l = libs();
    let mut cb = dstbuf(cap);
    let mut rb = dstbuf(cap);
    unsafe {
        let (cr, rr) = match accel {
            None => {
                let (c, r) = l.sym::<FnCompressDefault>("LZ4_compress_default");
                (
                    c(src, cb.as_mut_ptr() as *mut c_char, src_size, cap as c_int),
                    r(src, rb.as_mut_ptr() as *mut c_char, src_size, cap as c_int),
                )
            }
            Some(a) => {
                let (c, r) = l.sym::<FnCompressFast>("LZ4_compress_fast");
                (
                    c(src, cb.as_mut_ptr() as *mut c_char, src_size, cap as c_int, a),
                    r(src, rb.as_mut_ptr() as *mut c_char, src_size, cap as c_int, a),
                )
            }
        };
        same_int_and_bytes(ctx, cr, rr, &cb, &rb);
        same_full_buffers(ctx, &cb, &rb);
        (cr, cb, rb)
    }
}

/// Cross-check: decompress the C output with the RUST decoder and the Rust
/// output with the C decoder. Catches asymmetric encoder/decoder bugs.
fn cross_roundtrip(ctx: &str, src: &[u8], c_comp: &[u8], r_comp: &[u8]) {
    let l = libs();
    let cap = src.len().max(1);
    let pc = padded(c_comp);
    let pr = padded(r_comp);
    unsafe {
        let (dc, dr) = l.sym::<FnDecompressSafe>("LZ4_decompress_safe");
        let mut o1 = dstbuf(cap);
        let n1 = dr(
            pc.as_ptr() as *const c_char,
            o1.as_mut_ptr() as *mut c_char,
            c_comp.len() as c_int,
            cap as c_int,
        );
        assert_eq!(
            n1, src.len() as c_int,
            "{ctx}: RUST decoder on C-compressed block returned {n1}, want {}",
            src.len()
        );
        assert!(
            o1[..src.len()] == *src,
            "{ctx}: RUST decoder on C-compressed block produced wrong plaintext (first diff {:?})",
            first_diff(&o1[..src.len()], src)
        );
        let mut o2 = dstbuf(cap);
        let n2 = dc(
            pr.as_ptr() as *const c_char,
            o2.as_mut_ptr() as *mut c_char,
            r_comp.len() as c_int,
            cap as c_int,
        );
        assert_eq!(
            n2, src.len() as c_int,
            "{ctx}: C decoder on RUST-compressed block returned {n2}, want {}",
            src.len()
        );
        assert!(
            o2[..src.len()] == *src,
            "{ctx}: C decoder on RUST-compressed block produced wrong plaintext (first diff {:?})",
            first_diff(&o2[..src.len()], src)
        );
    }
}

fn diff_compress(ctx: &str, src: &[u8], cap: usize, accel: Option<c_int>) -> c_int {
    let (ret, cb, rb) =
        diff_compress_raw(ctx, src.as_ptr() as *const c_char, src.len() as c_int, cap, accel);
    if ret > 0 {
        cross_roundtrip(ctx, src, &cb[..ret as usize], &rb[..ret as usize]);
    }
    ret
}

/// `LZ4_compress_fast_extState` / `..._fastReset` with caller-allocated state.
/// Compares the return value, the destination buffer AND the resulting state image.
fn diff_extstate(
    ctx: &str,
    symbol: &str,
    src: &[u8],
    cap: usize,
    accel: c_int,
    cs: &mut State,
    rs: &mut State,
) -> c_int {
    let l = libs();
    let mut cb = dstbuf(cap);
    let mut rb = dstbuf(cap);
    unsafe {
        let (c, r) = l.sym::<FnCompressFastExtState>(symbol);
        let a = c(
            cs.ptr(),
            src.as_ptr() as *const c_char,
            cb.as_mut_ptr() as *mut c_char,
            src.len() as c_int,
            cap as c_int,
            accel,
        );
        let b = r(
            rs.ptr(),
            src.as_ptr() as *const c_char,
            rb.as_mut_ptr() as *mut c_char,
            src.len() as c_int,
            cap as c_int,
            accel,
        );
        same_int_and_bytes(ctx, a, b, &cb, &rb);
        same_full_buffers(ctx, &cb, &rb);
        same_full_buffers(&format!("{ctx} [state image]"), cs.bytes(), rs.bytes());
        if a > 0 {
            cross_roundtrip(ctx, src, &cb[..a as usize], &rb[..a as usize]);
        }
        a
    }
}

/// `LZ4_compress_destSize`. Returns `(ret, *srcSizePtr)`.
fn diff_destsize(ctx: &str, src: &[u8], target: usize) -> (c_int, c_int) {
    let l = libs();
    let mut cb = dstbuf(target);
    let mut rb = dstbuf(target);
    let mut cn = src.len() as c_int;
    let mut rn = src.len() as c_int;
    unsafe {
        let (c, r) = l.sym::<FnCompressDestSize>("LZ4_compress_destSize");
        let a = c(
            src.as_ptr() as *const c_char,
            cb.as_mut_ptr() as *mut c_char,
            &mut cn,
            target as c_int,
        );
        let b = r(
            src.as_ptr() as *const c_char,
            rb.as_mut_ptr() as *mut c_char,
            &mut rn,
            target as c_int,
        );
        same_int_and_bytes(ctx, a, b, &cb, &rb);
        same_full_buffers(ctx, &cb, &rb);
        assert_eq!(cn, rn, "{ctx}: *srcSizePtr mismatch (C={cn} Rust={rn})");
        if a > 0 {
            assert!(
                cn >= 0 && cn as usize <= src.len(),
                "{ctx}: nonsensical *srcSizePtr {cn}"
            );
            assert!(
                a as usize <= target,
                "{ctx}: destSize overflowed target ({a} > {target})"
            );
            cross_roundtrip(ctx, &src[..cn as usize], &cb[..a as usize], &rb[..a as usize]);
        }
        (a, cn)
    }
}

/// `LZ4_compress_destSize_extState`. Returns `(ret, *srcSizePtr)`.
fn diff_destsize_extstate(
    ctx: &str,
    src: &[u8],
    target: usize,
    accel: c_int,
    cs: &mut State,
    rs: &mut State,
) -> (c_int, c_int) {
    let l = libs();
    let mut cb = dstbuf(target);
    let mut rb = dstbuf(target);
    let mut cn = src.len() as c_int;
    let mut rn = src.len() as c_int;
    unsafe {
        let (c, r) = l.sym::<FnCompressDestSizeExtState>("LZ4_compress_destSize_extState");
        let a = c(
            cs.ptr(),
            src.as_ptr() as *const c_char,
            cb.as_mut_ptr() as *mut c_char,
            &mut cn,
            target as c_int,
            accel,
        );
        let b = r(
            rs.ptr(),
            src.as_ptr() as *const c_char,
            rb.as_mut_ptr() as *mut c_char,
            &mut rn,
            target as c_int,
            accel,
        );
        same_int_and_bytes(ctx, a, b, &cb, &rb);
        same_full_buffers(ctx, &cb, &rb);
        same_full_buffers(&format!("{ctx} [state image]"), cs.bytes(), rs.bytes());
        assert_eq!(cn, rn, "{ctx}: *srcSizePtr mismatch (C={cn} Rust={rn})");
        if a > 0 {
            cross_roundtrip(ctx, &src[..cn as usize], &cb[..a as usize], &rb[..a as usize]);
        }
        (a, cn)
    }
}

fn diff_decompress_safe(ctx: &str, comp: &[u8], comp_size: c_int, cap: usize) -> (c_int, Vec<u8>) {
    let l = libs();
    let p = padded(comp);
    let mut cb = dstbuf(cap);
    let mut rb = dstbuf(cap);
    unsafe {
        let (c, r) = l.sym::<FnDecompressSafe>("LZ4_decompress_safe");
        let a = c(
            p.as_ptr() as *const c_char,
            cb.as_mut_ptr() as *mut c_char,
            comp_size,
            cap as c_int,
        );
        let b = r(
            p.as_ptr() as *const c_char,
            rb.as_mut_ptr() as *mut c_char,
            comp_size,
            cap as c_int,
        );
        same_int_and_bytes(ctx, a, b, &cb, &rb);
        same_full_buffers(ctx, &cb, &rb);
        (a, cb)
    }
}

fn diff_decompress_partial(
    ctx: &str,
    comp: &[u8],
    comp_size: c_int,
    target: c_int,
    cap: usize,
) -> (c_int, Vec<u8>) {
    let l = libs();
    let p = padded(comp);
    let mut cb = dstbuf(cap);
    let mut rb = dstbuf(cap);
    unsafe {
        let (c, r) = l.sym::<FnDecompressSafePartial>("LZ4_decompress_safe_partial");
        let a = c(
            p.as_ptr() as *const c_char,
            cb.as_mut_ptr() as *mut c_char,
            comp_size,
            target,
            cap as c_int,
        );
        let b = r(
            p.as_ptr() as *const c_char,
            rb.as_mut_ptr() as *mut c_char,
            comp_size,
            target,
            cap as c_int,
        );
        same_int_and_bytes(ctx, a, b, &cb, &rb);
        same_full_buffers(ctx, &cb, &rb);
        (a, cb)
    }
}

/// `LZ4_decompress_fast`. Only ever called on *well-formed* blocks: it has no
/// input-bounds checking at all, so feeding it corrupt data would read wild.
fn diff_decompress_fast(ctx: &str, comp: &[u8], orig_size: c_int) -> (c_int, Vec<u8>) {
    let l = libs();
    let p = padded(comp);
    let cap = if orig_size > 0 { orig_size as usize } else { 0 };
    let mut cb = dstbuf(cap);
    let mut rb = dstbuf(cap);
    unsafe {
        let (c, r) = l.sym::<FnDecompressFast>("LZ4_decompress_fast");
        let a = c(
            p.as_ptr() as *const c_char,
            cb.as_mut_ptr() as *mut c_char,
            orig_size,
        );
        let b = r(
            p.as_ptr() as *const c_char,
            rb.as_mut_ptr() as *mut c_char,
            orig_size,
        );
        same_int_and_bytes(ctx, a, b, &cb, &rb);
        same_full_buffers(ctx, &cb, &rb);
        (a, cb)
    }
}

// ---------------------------------------------------------------------------
// Hand-rolled LZ4 block builder — gives exact control over token nibbles,
// 255-extension chains and match offsets.
// ---------------------------------------------------------------------------

struct Blk {
    c: Vec<u8>,
    p: Vec<u8>,
}

impl Blk {
    fn new() -> Blk {
        Blk { c: Vec::new(), p: Vec::new() }
    }

    fn push_ext(v: &mut Vec<u8>, mut rem: usize) {
        while rem >= 255 {
            v.push(255);
            rem -= 255;
        }
        v.push(rem as u8);
    }

    /// Append one (literals, match) sequence. `matchlen` counts the *total*
    /// match length (>= MINMATCH == 4); `offset` must be >= 1 and <= the number
    /// of plaintext bytes produced so far (including `lits`).
    fn seq(&mut self, lits: &[u8], offset: usize, matchlen: usize) {
        assert!(matchlen >= 4, "matchlen {matchlen} < MINMATCH");
        assert!(offset >= 1 && offset <= 65535);
        let ll = lits.len();
        let ml = matchlen - 4;
        self.c.push(((ll.min(15) as u8) << 4) | (ml.min(15) as u8));
        if ll >= 15 {
            Self::push_ext(&mut self.c, ll - 15);
        }
        self.c.extend_from_slice(lits);
        self.p.extend_from_slice(lits);
        self.c.push((offset & 0xFF) as u8);
        self.c.push(((offset >> 8) & 0xFF) as u8);
        if ml >= 15 {
            Self::push_ext(&mut self.c, ml - 15);
        }
        assert!(offset <= self.p.len(), "offset {offset} > produced {}", self.p.len());
        for _ in 0..matchlen {
            let b = self.p[self.p.len() - offset];
            self.p.push(b);
        }
    }

    /// Terminating literals-only sequence. Returns `(compressed, plaintext)`.
    fn finish(mut self, lits: &[u8]) -> (Vec<u8>, Vec<u8>) {
        let ll = lits.len();
        self.c.push((ll.min(15) as u8) << 4);
        if ll >= 15 {
            Self::push_ext(&mut self.c, ll - 15);
        }
        self.c.extend_from_slice(lits);
        self.p.extend_from_slice(lits);
        (self.c, self.p)
    }
}

fn rand_lits(rng: &mut Rng, n: usize) -> Vec<u8> {
    (0..n).map(|_| rng.byte()).collect()
}

/// `rand_lits` with a randomly chosen length in `[lo, hi]` (avoids a double
/// mutable borrow of `rng` at the call site).
fn rand_lits_range(rng: &mut Rng, r: (usize, usize)) -> Vec<u8> {
    let n = rng.range(r.0, r.1);
    rand_lits(rng, n)
}

/// Build a random but *well-formed* block. `offsets` is the pool of candidate
/// match offsets (clamped down when not enough plaintext has been produced yet).
///
/// The trailing literal run is forced to >= MFLIMIT == 12 bytes: for a
/// non-final sequence `LZ4_decompress_generic` requires that the literal copy
/// end at least MFLIMIT bytes before `oend`, otherwise it demands that the
/// sequence be the last one and reports an error. Blocks that deliberately
/// break that rule are built explicitly in `row_29`.
fn build_block(
    rng: &mut Rng,
    nseq: usize,
    offsets: &[usize],
    ll: (usize, usize),
    ml: (usize, usize),
    tail: (usize, usize),
) -> (Vec<u8>, Vec<u8>) {
    let mut b = Blk::new();
    for _ in 0..nseq {
        let n = rng.range(ll.0, ll.1);
        let lits = rand_lits(rng, n);
        let avail = b.p.len() + n;
        if avail == 0 {
            continue; // nothing to point a match at yet
        }
        let mut off = offsets[rng.below(offsets.len())];
        if off > avail {
            off = avail;
        }
        if off > 65535 {
            off = 65535;
        }
        if off == 0 {
            off = 1;
        }
        let m = rng.range(ml.0.max(4), ml.1.max(4));
        b.seq(&lits, off, m);
    }
    let t = rng.range(tail.0.max(12), tail.1.max(12));
    let lits = rand_lits(rng, t);
    b.finish(&lits)
}

/// Compress `src` with both libraries (asserting they agree) and return the
/// (identical) compressed block.
fn compress_both(ctx: &str, src: &[u8]) -> Vec<u8> {
    let cap = ubound(src.len());
    let (ret, cb, rb) =
        diff_compress_raw(ctx, src.as_ptr() as *const c_char, src.len() as c_int, cap, None);
    assert!(ret > 0, "{ctx}: compression failed");
    cross_roundtrip(ctx, src, &cb[..ret as usize], &rb[..ret as usize]);
    cb[..ret as usize].to_vec()
}

// ===========================================================================
// Row 1 — LZ4_compress_default with srcSize == 0
// ===========================================================================

#[test]
fn row_01_compress_default_srcsize_zero() {
    let mut rng = Rng::new(1);
    let scratch = vec![0x5Au8; 64];
    for i in 0..400usize {
        let cap = match i % 7 {
            0 => 0,
            1 => 1,
            2 => 2,
            3 => 15,
            4 => 16,
            5 => 17,
            _ => rng.range(0, 600),
        };
        let want = if cap == 0 { 0 } else { 1 };

        // Non-NULL src, srcSize == 0.
        let ret = diff_compress(&format!("row1 default cap={cap}"), &scratch[..0], cap, None);
        assert_eq!(ret, want, "row1: srcSize=0 cap={cap} unexpected return {ret}");

        // NULL src is explicitly supported when srcSize == 0.
        let (ret2, cb, _rb) =
            diff_compress_raw(&format!("row1 NULL src cap={cap}"), std::ptr::null(), 0, cap, None);
        assert_eq!(ret2, want, "row1: NULL src cap={cap} unexpected return {ret2}");
        if ret2 == 1 {
            assert_eq!(cb[0], 0, "row1: empty block must be a single 0x00 token");
            assert_eq!(cb[1], SENT, "row1: empty block wrote more than one byte");
        }

        // Same through LZ4_compress_fast with a random acceleration.
        let a = rng.range(0, 80) as c_int;
        let ret3 = diff_compress(
            &format!("row1 fast cap={cap} accel={a}"),
            &scratch[..0],
            cap,
            Some(a),
        );
        assert_eq!(ret3, want);
    }
}

// ===========================================================================
// Row 2 — srcSize 1 / 12 (== MFLIMIT) / 13 (== LZ4_minLength)
// ===========================================================================

#[test]
fn row_02_compress_default_tiny_srcsizes() {
    let mut rng = Rng::new(2);
    const NS: [usize; 24] = [
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 31, 32, 33,
    ];
    for &n in NS.iter() {
        let b = ubound(n);
        for &sh in ALL_SHAPES.iter() {
            for rep in 0..2 {
                let src = gen(&mut rng, sh, n);
                let caps = [b, b + 9, b.saturating_sub(1), n, 1, 0];
                for &cap in caps.iter() {
                    let ctx = format!("row2 n={n} {sh:?} rep{rep} cap={cap}");
                    let ret = diff_compress(&ctx, &src, cap, None);
                    if cap >= b {
                        assert!(ret > 0, "{ctx}: bound-sized dst must succeed");
                        if n == 0 {
                            assert_eq!(ret, 1);
                        } else if n <= 12 {
                            // < LZ4_minLength: all-literals path.
                            assert!(ret as usize >= n + 1);
                        }
                    }
                }
            }
        }
    }
}

// ===========================================================================
// Row 3 — highly compressible < 4 KB, dstCapacity == bound (byU16 + notLimited)
// ===========================================================================

#[test]
fn row_03_compressible_under_4k_byu16_notlimited() {
    let mut rng = Rng::new(3);
    for _ in 0..1000 {
        let n = rng.range(16, 4095);
        let src = gen_compressible(&mut rng, n);
        let ret = diff_compress(&format!("row3 compressible n={n}"), &src, ubound(n), None);
        assert!(ret > 0);
    }
    for &sh in ALL_SHAPES.iter() {
        for rep in 0..4 {
            let src = gen(&mut rng, sh, 1024);
            diff_compress(&format!("row3 1KB {sh:?} rep{rep}"), &src, ubound(1024), None);
        }
    }
}

// ===========================================================================
// Row 4 — incompressible 1 KB: one long literal run with a 255-byte chain
// ===========================================================================

#[test]
fn row_04_incompressible_long_literal_run() {
    let mut rng = Rng::new(4);
    for _ in 0..600 {
        let n = rng.range(300, 3000);
        let src = gen_incompressible(&mut rng, n);
        let ret = diff_compress(&format!("row4 random n={n}"), &src, ubound(n), None);
        assert!(ret as usize >= n, "row4: random data should not shrink");
    }
    // Exactly 1 KB, repeated, plus sizes that make the lastRun 255-chain change
    // length (15, 15+255, 15+2*255, ...).
    for &n in &[1024usize, 15, 16, 269, 270, 271, 524, 525, 526, 779, 780, 781] {
        for rep in 0..3 {
            let src = gen_incompressible(&mut rng, n);
            let ret = diff_compress(&format!("row4 exact n={n} rep{rep}"), &src, ubound(n), None);
            assert!(ret > 0);
        }
    }
}

// ===========================================================================
// Row 5 — srcSize 65535 and 65546: still byU16
// ===========================================================================

#[test]
fn row_05_byu16_upper_boundary() {
    let mut rng = Rng::new(5);
    for &n in &[65535usize, 65536, 65537, 65545, 65546] {
        for &sh in ALL_SHAPES.iter() {
            assert!(n < LZ4_64KLIMIT || n >= 65547);
            let src = gen(&mut rng, sh, n);
            let b = ubound(n);
            for &cap in &[b, b - 1, b / 2] {
                let ctx = format!("row5 n={n} {sh:?} cap={cap}");
                diff_compress(&ctx, &src, cap, None);
            }
        }
    }
}

// ===========================================================================
// Row 6 — srcSize == LZ4_64Klimit (65547): switches to byU32
// ===========================================================================

#[test]
fn row_06_byu32_pivot_65547() {
    let mut rng = Rng::new(6);
    for &n in &[LZ4_64KLIMIT, LZ4_64KLIMIT + 1, LZ4_64KLIMIT + 2, LZ4_64KLIMIT + 1000] {
        for &sh in ALL_SHAPES.iter() {
            let src = gen(&mut rng, sh, n);
            let b = ubound(n);
            for &cap in &[b, b - 1, b / 2] {
                diff_compress(&format!("row6 n={n} {sh:?} cap={cap}"), &src, cap, None);
            }
        }
    }
    // The pivot triple, all shapes, once more with a fresh rng draw.
    for &n in &[65546usize, 65547, 65548] {
        for &sh in ALL_SHAPES.iter() {
            let src = gen(&mut rng, sh, n);
            let ret = diff_compress(&format!("row6 pivot n={n} {sh:?}"), &src, ubound(n), None);
            assert!(ret > 0);
        }
    }
}

// ===========================================================================
// Row 7 — 256 KB / 1 MB / 4 MB (frame block-size boundaries) → byU32
// ===========================================================================

#[test]
fn row_07_large_inputs_byu32() {
    let mut rng = Rng::new(7);
    for &n in &[262144usize, 262145, 1048576, 4 * 1024 * 1024] {
        for &sh in &[Shape::Compressible, Shape::TextLike, Shape::Incompressible] {
            let src = gen(&mut rng, sh, n);
            let b = ubound(n);
            let ret = diff_compress(&format!("row7 n={n} {sh:?} bound"), &src, b, None);
            assert!(ret > 0);
            diff_compress(&format!("row7 n={n} {sh:?} bound-1"), &src, b - 1, None);
        }
    }
    // A few extra randomly sized large buffers.
    for _ in 0..4 {
        let n = rng.range(200_000, 900_000);
        let src = gen_textlike(&mut rng, n);
        diff_compress(&format!("row7 rand n={n}"), &src, ubound(n), None);
    }
}

// ===========================================================================
// Row 8 — 100 KB of a single repeated byte: offset 1 + 4*255 match-length chain
// ===========================================================================

#[test]
fn row_08_single_repeated_byte_100k() {
    let mut rng = Rng::new(8);
    for _ in 0..8 {
        let b = rng.byte();
        for &n in &[100 * 1024usize, 65536, 200_000] {
            let src = vec![b; n];
            let ret = diff_compress(&format!("row8 byte={b:02x} n={n}"), &src, ubound(n), None);
            assert!(ret > 0 && (ret as usize) < 1000, "row8: expected tiny output, got {ret}");
        }
    }
    // Periods 1..8 over 100 KB — offset == period, huge match lengths.
    for period in 1..=8usize {
        let base = rand_lits(&mut rng, period);
        let src: Vec<u8> = (0..100 * 1024).map(|i| base[i % period]).collect();
        diff_compress(&format!("row8 period={period}"), &src, ubound(src.len()), None);
    }
}

// ===========================================================================
// Row 9 — dstCapacity == bound-1 (limitedOutput, succeeds) and far too small
// ===========================================================================

#[test]
fn row_09_limited_output() {
    let mut rng = Rng::new(9);
    let sizes = interesting_sizes();
    for _ in 0..800 {
        let n = sizes[rng.below(sizes.len())];
        let sh = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
        let src = gen(&mut rng, sh, n);
        let b = ubound(n);
        // bound-1 must still succeed for any input LZ4 can encode.
        let ctx = format!("row9 n={n} {sh:?} bound-1");
        diff_compress(&ctx, &src, b - 1, None);
        // Progressively hostile capacities; 0 must fail.
        for &cap in &[b / 2, b / 4, n / 2, n / 8, 16, 8, 2, 1, 0] {
            let ctx = format!("row9 n={n} {sh:?} cap={cap}");
            let ret = diff_compress(&ctx, &src, cap, None);
            if cap == 0 {
                assert_eq!(ret, 0, "{ctx}: dstCapacity 0 must return 0");
            }
        }
    }
}

// ===========================================================================
// Row 10 — srcSize negative or > LZ4_MAX_INPUT_SIZE → 0
// ===========================================================================

#[test]
fn row_10_invalid_srcsize() {
    let mut rng = Rng::new(10);
    let scratch = vec![0xA5u8; 256];
    let mut bad: Vec<c_int> = vec![
        -1,
        -2,
        -13,
        -1000,
        c_int::MIN,
        c_int::MIN + 1,
        (LZ4_MAX_INPUT_SIZE + 1) as c_int,
        (LZ4_MAX_INPUT_SIZE + 2) as c_int,
        0x7EFF_FFFF,
        c_int::MAX,
    ];
    for _ in 0..40 {
        bad.push(-(rng.range(1, 1 << 30) as c_int));
        bad.push((LZ4_MAX_INPUT_SIZE + rng.range(1, 1 << 20)) as c_int);
    }
    for &ss in bad.iter() {
        for &cap in &[0usize, 1, 16, 1024] {
            let (r1, _, _) = diff_compress_raw(
                &format!("row10 default srcSize={ss} cap={cap}"),
                scratch.as_ptr() as *const c_char,
                ss,
                cap,
                None,
            );
            assert_eq!(r1, 0, "row10: LZ4_compress_default(srcSize={ss}) = {r1}, want 0");
            let (r2, _, _) = diff_compress_raw(
                &format!("row10 fast srcSize={ss} cap={cap}"),
                scratch.as_ptr() as *const c_char,
                ss,
                cap,
                Some(1),
            );
            assert_eq!(r2, 0, "row10: LZ4_compress_fast(srcSize={ss}) = {r2}, want 0");
        }
    }
}

// ===========================================================================
// Row 11 — LZ4_compress_fast acceleration matrix
// ===========================================================================

#[test]
fn row_11_compress_fast_acceleration() {
    let mut rng = Rng::new(11);
    let accels: [c_int; 15] = [
        c_int::MIN,
        -1_000_000,
        -1000,
        -1,
        0,
        1,
        2,
        3,
        8,
        64,
        65536,
        LZ4_ACCELERATION_MAX,
        LZ4_ACCELERATION_MAX + 1,
        1_000_000,
        c_int::MAX,
    ];
    let sizes = [0usize, 1, 13, 100, 1024, 4096, 65546, 65547, 200_000];
    for &n in sizes.iter() {
        for &sh in ALL_SHAPES.iter() {
            let src = gen(&mut rng, sh, n);
            let b = ubound(n);
            let mut baseline_default: Option<Vec<u8>> = None;
            let mut baseline_max: Option<Vec<u8>> = None;
            for &a in accels.iter() {
                for &cap in &[b, b / 2 + 1] {
                    let ctx = format!("row11 n={n} {sh:?} accel={a} cap={cap}");
                    diff_compress(&ctx, &src, cap, Some(a));
                }
                // acceleration < 1 is coerced to LZ4_ACCELERATION_DEFAULT == 1,
                // and anything > LZ4_ACCELERATION_MAX is clamped to it.
                let (ret, cb, _) = diff_compress_raw(
                    &format!("row11 canon n={n} {sh:?} accel={a}"),
                    src.as_ptr() as *const c_char,
                    src.len() as c_int,
                    b,
                    Some(a),
                );
                let out = cb[..ret.max(0) as usize].to_vec();
                if a <= 1 {
                    match &baseline_default {
                        None => baseline_default = Some(out),
                        Some(bl) => assert_eq!(
                            *bl, out,
                            "row11: accel={a} must behave like acceleration 1 (n={n} {sh:?})"
                        ),
                    }
                } else if a >= LZ4_ACCELERATION_MAX {
                    match &baseline_max {
                        None => baseline_max = Some(out),
                        Some(bl) => assert_eq!(
                            *bl, out,
                            "row11: accel={a} must clamp to LZ4_ACCELERATION_MAX (n={n} {sh:?})"
                        ),
                    }
                }
            }
        }
    }
}

// ===========================================================================
// Row 12 — LZ4_sizeofState + LZ4_compress_fast_extState
// ===========================================================================

#[test]
fn row_12_sizeofstate_and_extstate() {
    let n = sizeof_state();
    assert!(n >= 16_000, "unexpected LZ4_sizeofState() == {n}");
    let mut rng = Rng::new(12);

    // Four quadrants: src </>= LZ4_64Klimit  x  dst >=/< bound.
    let sizes = [
        0usize, 1, 13, 1024, 4096, 65546, LZ4_64KLIMIT, LZ4_64KLIMIT + 3, 200_000,
    ];
    for &sz in sizes.iter() {
        for &sh in ALL_SHAPES.iter() {
            let src = gen(&mut rng, sh, sz);
            let b = ubound(sz);
            for &accel in &[1i32, 2, 17] {
                for &cap in &[b, b + 5, b - 1, b / 2, 1, 0] {
                    // Each library gets its OWN state buffer.
                    let mut cs = State::new(n);
                    let mut rs = State::new(n);
                    let ctx =
                        format!("row12 extState n={sz} {sh:?} accel={accel} cap={cap}");
                    diff_extstate(
                        &ctx,
                        "LZ4_compress_fast_extState",
                        &src,
                        cap,
                        accel,
                        &mut cs,
                        &mut rs,
                    );
                }
            }
        }
    }

    // Repeated reuse of one state pair: extState re-runs LZ4_initStream every
    // call, so the state image must be identical after each of them.
    let (mut cs, mut rs) = init_states(n);
    for i in 0..600 {
        let sz = rng.range(0, 6000);
        let sh = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
        let src = gen(&mut rng, sh, sz);
        let b = ubound(sz);
        let cap = if i % 3 == 0 { b } else { b / 2 };
        diff_extstate(
            &format!("row12 reuse i={i} n={sz} {sh:?}"),
            "LZ4_compress_fast_extState",
            &src,
            cap,
            1,
            &mut cs,
            &mut rs,
        );
    }
}

// ===========================================================================
// Row 13 — LZ4_initStream: valid / undersized / misaligned / NULL
// ===========================================================================

#[test]
fn row_13_init_stream() {
    let l = libs();
    let n = sizeof_state();
    let mut rng = Rng::new(13);

    unsafe {
        let (c, r) = l.sym::<FnInitStream>("LZ4_initStream");

        // NULL buffer, many sizes.
        for i in 0..20 {
            let size = if i == 0 { 0 } else { rng.range(0, 4 * n) };
            let a = c(std::ptr::null_mut(), size);
            let b = r(std::ptr::null_mut(), size);
            assert!(a.is_null(), "C initStream(NULL,{size}) returned {a:?}");
            assert!(b.is_null(), "Rust initStream(NULL,{size}) returned {b:?}");
        }

        // Valid aligned buffers, size >= sizeof(LZ4_stream_t).
        for i in 0..25 {
            let size = if i == 0 { n } else { n + rng.range(0, 4096) };
            let mut cs = State::new(size + 64);
            let mut rs = State::new(size + 64);
            let cp = cs.ptr();
            let rp = rs.ptr();
            let a = c(cp, size);
            let b = r(rp, size);
            assert_eq!(a, cp, "row13: C initStream(valid,{size}) should return buffer");
            assert_eq!(b, rp, "row13: Rust initStream(valid,{size}) should return buffer");
            same_full_buffers(
                &format!("row13 valid size={size} state image"),
                cs.bytes(),
                rs.bytes(),
            );
        }

        // Undersized buffers → NULL, and nothing must be written.
        for i in 0..25 {
            let size = match i {
                0 => 0,
                1 => 1,
                2 => n - 1,
                3 => n / 2,
                _ => rng.below(n),
            };
            let mut cs = State::new(n + 64);
            let mut rs = State::new(n + 64);
            let cp = cs.ptr();
            let rp = rs.ptr();
            let a = c(cp, size);
            let b = r(rp, size);
            assert_eq!(
                a.is_null(),
                b.is_null(),
                "row13: undersized size={size}: C null={} Rust null={}",
                a.is_null(),
                b.is_null()
            );
            assert!(a.is_null(), "row13: C initStream(size={size} < {n}) must be NULL");
            same_full_buffers(
                &format!("row13 undersized size={size} state image"),
                cs.bytes(),
                rs.bytes(),
            );
        }

        // Misaligned buffers → NULL (LZ4_ALIGN_TEST is enabled).
        for off in 1..8usize {
            for extra in [0usize, 1, 4096] {
                let size = n + extra;
                let mut cs = State::new(size + 64);
                let mut rs = State::new(size + 64);
                let a = c(cs.ptr_off(off), size);
                let b = r(rs.ptr_off(off), size);
                assert_eq!(
                    a.is_null(),
                    b.is_null(),
                    "row13: misaligned off={off} size={size}: C null={} Rust null={}",
                    a.is_null(),
                    b.is_null()
                );
                assert!(
                    a.is_null(),
                    "row13: C initStream(misaligned by {off}) should be NULL"
                );
                same_full_buffers(
                    &format!("row13 misaligned off={off} size={size} state image"),
                    cs.bytes(),
                    rs.bytes(),
                );
            }
        }
    }
}

// ===========================================================================
// Row 14 — extState_fastReset: noDictIssue (fresh) vs dictSmall (reused)
// ===========================================================================

#[test]
fn row_14_extstate_fastreset_dictsmall() {
    let n = sizeof_state();
    let mut rng = Rng::new(14);
    let sizes = [1usize, 13, 100, 1024, 3000, 4095, 20000, 65546];
    for &sz in sizes.iter() {
        for &sh in ALL_SHAPES.iter() {
            let b = ubound(sz);
            for &cap in &[b, b - 1] {
                // Fresh state → currentOffset == 0 → noDictIssue.
                let (mut cs, mut rs) = init_states(n);
                let src0 = gen(&mut rng, sh, sz);
                diff_extstate(
                    &format!("row14 fresh n={sz} {sh:?} cap={cap}"),
                    "LZ4_compress_fast_extState_fastReset",
                    &src0,
                    cap,
                    1,
                    &mut cs,
                    &mut rs,
                );
                // Reused state → currentOffset != 0 → dictSmall (when byU16).
                for k in 0..3 {
                    let src = gen(&mut rng, sh, sz.min(3000));
                    diff_extstate(
                        &format!("row14 reuse{k} n={sz} {sh:?} cap={cap}"),
                        "LZ4_compress_fast_extState_fastReset",
                        &src,
                        ubound(src.len()),
                        1,
                        &mut cs,
                        &mut rs,
                    );
                }
            }
        }
    }
}

// ===========================================================================
// Row 15 — extState_fastReset: srcSize < 4 KB (table reuse) vs >= 4 KB (reset)
// ===========================================================================

#[test]
fn row_15_extstate_fastreset_4kb_threshold() {
    let n = sizeof_state();
    let mut rng = Rng::new(15);

    for round in 0..12 {
        let (mut cs, mut rs) = init_states(n);
        let sh = ALL_SHAPES[round % ALL_SHAPES.len()];
        // A long run of sub-4 KB blocks: the hash table is re-used and
        // currentOffset accumulates identically in both implementations.
        for i in 0..30 {
            let sz = rng.range(1, 4095);
            let src = gen(&mut rng, sh, sz);
            diff_extstate(
                &format!("row15 round{round} small i={i} n={sz} {sh:?}"),
                "LZ4_compress_fast_extState_fastReset",
                &src,
                ubound(sz),
                1,
                &mut cs,
                &mut rs,
            );
        }
        // Now cross the 4 KB threshold: LZ4_prepareTable does a full MEM_INIT
        // reset and drops currentOffset back to 0.
        for &sz in &[4096usize, 4097, 8192, 4095, 100] {
            let src = gen(&mut rng, sh, sz);
            diff_extstate(
                &format!("row15 round{round} big n={sz} {sh:?}"),
                "LZ4_compress_fast_extState_fastReset",
                &src,
                ubound(sz),
                1,
                &mut cs,
                &mut rs,
            );
        }
        // Interleave exactly at the boundary many times.
        for i in 0..20 {
            let sz = if i % 2 == 0 { 4095 } else { 4096 };
            let src = gen(&mut rng, sh, sz);
            diff_extstate(
                &format!("row15 round{round} alt i={i} n={sz} {sh:?}"),
                "LZ4_compress_fast_extState_fastReset",
                &src,
                ubound(sz),
                1,
                &mut cs,
                &mut rs,
            );
        }
    }
}

// ===========================================================================
// Row 16 — extState_fastReset: forced table reset thresholds
//   byU16 : currentOffset + srcSize >= 0xFFFF
//   byU32 : currentOffset > 1 GB  (see note below)
//   plus tableType byU16 <-> byU32 transitions
// ===========================================================================

#[test]
fn row_16_extstate_fastreset_forced_table_reset() {
    let n = sizeof_state();
    let mut rng = Rng::new(16);

    // byU16: repeatedly compress ~1000-byte blocks. currentOffset grows by
    // srcSize per call until currentOffset + srcSize >= 0xFFFF forces a reset;
    // the cycle then repeats. 200 calls crosses the threshold ~3 times.
    for round in 0..8 {
        let (mut cs, mut rs) = init_states(n);
        let sh = ALL_SHAPES[round % ALL_SHAPES.len()];
        for i in 0..300 {
            let sz = 1000 + rng.below(24);
            let src = gen(&mut rng, sh, sz);
            diff_extstate(
                &format!("row16 byU16 round{round} i={i} n={sz} {sh:?}"),
                "LZ4_compress_fast_extState_fastReset",
                &src,
                ubound(sz),
                1,
                &mut cs,
                &mut rs,
            );
        }
    }

    // Sizes chosen so that currentOffset lands exactly on 0xFFFF - srcSize.
    for &sz in &[65534usize / 4, 21845, 32767, 16383] {
        let (mut cs, mut rs) = init_states(n);
        for i in 0..8 {
            let src = gen_textlike(&mut rng, sz);
            diff_extstate(
                &format!("row16 exact sz={sz} i={i}"),
                "LZ4_compress_fast_extState_fastReset",
                &src,
                ubound(sz),
                1,
                &mut cs,
                &mut rs,
            );
        }
    }

    // tableType transitions: byU16 (small) <-> byU32 (>= LZ4_64Klimit) force a
    // reset via the `cctx->tableType != tableType` arm.
    //
    // NOTE: `currentOffset > 1 GB` (the byU32 arm) is unreachable through
    // _fastReset, because byU32 is only selected when srcSize >= LZ4_64Klimit,
    // which is also >= 4 KB and therefore always resets currentOffset to 0
    // first. It is only reachable through the streaming API (rows 35+).
    for round in 0..3 {
        let (mut cs, mut rs) = init_states(n);
        for i in 0..12 {
            let sz = if i % 2 == 0 { 2000 + rng.below(100) } else { LZ4_64KLIMIT + rng.below(64) };
            let src = gen_textlike(&mut rng, sz);
            diff_extstate(
                &format!("row16 mix round{round} i={i} n={sz}"),
                "LZ4_compress_fast_extState_fastReset",
                &src,
                ubound(sz),
                1,
                &mut cs,
                &mut rs,
            );
        }
    }

    // A byU32 state followed by small byU16 blocks (and vice versa), via
    // extState first so the state carries a byU32 tableType.
    for round in 0..3 {
        let (mut cs, mut rs) = init_states(n);
        let big = gen_textlike(&mut rng, LZ4_64KLIMIT + 10);
        diff_extstate(
            &format!("row16 seed byU32 round{round}"),
            "LZ4_compress_fast_extState",
            &big,
            ubound(big.len()),
            1,
            &mut cs,
            &mut rs,
        );
        for i in 0..10 {
            let sz = rng.range(1, 3999);
            let src = gen_compressible(&mut rng, sz);
            diff_extstate(
                &format!("row16 after-byU32 round{round} i={i} n={sz}"),
                "LZ4_compress_fast_extState_fastReset",
                &src,
                ubound(sz),
                1,
                &mut cs,
                &mut rs,
            );
        }
    }
}

// ===========================================================================
// Row 17 — LZ4_compress_destSize with targetDstSize >= bound
// ===========================================================================

#[test]
fn row_17_destsize_whole_input() {
    let mut rng = Rng::new(17);
    let sizes = interesting_sizes();
    for _ in 0..800 {
        let n = sizes[rng.below(sizes.len())];
        let sh = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
        let src = gen(&mut rng, sh, n);
        let b = ubound(n);
        for &target in &[b, b + 1, b + 1000, 2 * b] {
            let ctx = format!("row17 n={n} {sh:?} target={target}");
            let (ret, consumed) = diff_destsize(&ctx, &src, target);
            assert!(ret > 0, "{ctx}: must succeed");
            assert_eq!(
                consumed as usize, n,
                "{ctx}: whole input should be consumed, got {consumed}"
            );
        }
    }
}

// ===========================================================================
// Row 18 — LZ4_compress_destSize fillOutput, *srcSizePtr reduced
// ===========================================================================

#[test]
fn row_18_destsize_fill_output() {
    let mut rng = Rng::new(18);
    let sizes = [
        13usize, 100, 1024, 4096, 20000, 65535, 65546, LZ4_64KLIMIT, LZ4_64KLIMIT + 5, 300_000,
    ];
    for &n in sizes.iter() {
        for &sh in ALL_SHAPES.iter() {
            let src = gen(&mut rng, sh, n);
            let b = ubound(n);
            for &target in &[b / 2, b / 3, b / 4, b / 8, n / 2 + 1] {
                let ctx = format!("row18 n={n} {sh:?} target={target}");
                let (ret, consumed) = diff_destsize(&ctx, &src, target);
                if ret > 0 {
                    assert!(
                        ret as usize <= target,
                        "{ctx}: output {ret} exceeded target {target}"
                    );
                    assert!(consumed as usize <= n);
                }
            }
        }
    }
    // Randomised sweep across the whole target range.
    for _ in 0..500 {
        let n = rng.range(1, 40000);
        let sh = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
        let src = gen(&mut rng, sh, n);
        let target = rng.range(0, ubound(n));
        diff_destsize(&format!("row18 rand n={n} {sh:?} target={target}"), &src, target);
    }
}

// ===========================================================================
// Row 19 — LZ4_compress_destSize with targetDstSize 0 and 1, and exact fills
// ===========================================================================

#[test]
fn row_19_destsize_tiny_targets() {
    let mut rng = Rng::new(19);
    for _ in 0..700 {
        let n = rng.range(0, 5000);
        let sh = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
        let src = gen(&mut rng, sh, n);

        // targetDstSize == 0 → 0, and *srcSizePtr must be left alone.
        let ctx0 = format!("row19 n={n} {sh:?} target=0");
        let (r0, c0) = diff_destsize(&ctx0, &src, 0);
        assert_eq!(r0, 0, "{ctx0}: targetDstSize 0 must return 0");
        assert_eq!(c0 as usize, n, "{ctx0}: *srcSizePtr must be untouched");

        // targetDstSize == 1 (minimum).
        let ctx1 = format!("row19 n={n} {sh:?} target=1");
        let (r1, _c1) = diff_destsize(&ctx1, &src, 1);
        if n == 0 {
            assert_eq!(r1, 1);
        }

        for &target in &[2usize, 3, 5, 16, 17] {
            diff_destsize(&format!("row19 n={n} {sh:?} target={target}"), &src, target);
        }
    }

    // Incompressible input with targets that land exactly on the lastRun
    // truncation branch (`lastRun -= (lastRun + 256 - RUN_MASK)/256`).
    for _ in 0..500 {
        let n = rng.range(300, 4000);
        let src = gen_incompressible(&mut rng, n);
        for d in 0..12usize {
            let target = 16 + d * 23;
            diff_destsize(&format!("row19 exactfill n={n} target={target}"), &src, target);
        }
    }
}

// ===========================================================================
// Row 20 — LZ4_compress_destSize_extState (acceleration 1 and 10)
// ===========================================================================

#[test]
fn row_20_destsize_extstate() {
    let n = sizeof_state();
    let mut rng = Rng::new(20);
    for &accel in &[1i32, 10] {
        for round in 0..40 {
            let sz = match round % 4 {
                0 => rng.range(1, 500),
                1 => rng.range(1000, 20000),
                2 => LZ4_64KLIMIT + rng.below(200),
                _ => rng.range(20000, 80000),
            };
            // Very repetitive input forces match-length truncation and the
            // `ip <= filledIp` hash-table-clearing branch inside fillOutput.
            let src: Vec<u8> = match round % 3 {
                0 => vec![0xABu8; sz],
                1 => gen_periodic(&mut rng, sz),
                _ => gen_compressible(&mut rng, sz),
            };
            let b = ubound(sz);
            for &target in &[b, b / 2, b / 4, b / 16, 64, 17, 2, 1, 0] {
                let mut cs = State::new(n);
                let mut rs = State::new(n);
                let ctx = format!("row20 accel={accel} sz={sz} target={target}");
                let (ret, consumed) =
                    diff_destsize_extstate(&ctx, &src, target, accel, &mut cs, &mut rs);
                if target == 0 {
                    assert_eq!(ret, 0, "{ctx}: target 0 must return 0");
                    assert_eq!(consumed as usize, sz);
                }
            }
        }
    }
    // Also drive LZ4_compress_destSize on the same repetitive shapes.
    for round in 0..50 {
        let sz = rng.range(5000, 60000);
        let src = if round % 2 == 0 {
            vec![0x11u8; sz]
        } else {
            gen_periodic(&mut rng, sz)
        };
        let b = ubound(sz);
        for &target in &[b / 2, 1000, 200, 40, 1] {
            diff_destsize(&format!("row20 destSize sz={sz} target={target}"), &src, target);
        }
    }
}

// ===========================================================================
// Row 21 — LZ4_compressBound, LZ4_versionNumber, LZ4_versionString
// ===========================================================================

#[test]
fn row_21_bound_and_version() {
    let l = libs();
    let mut rng = Rng::new(21);

    // Explicit boundary values.
    assert!(bound(0) > 0);
    assert!(bound(1) > 0);
    assert!(bound(LZ4_MAX_INPUT_SIZE as c_int) > 0);
    assert_eq!(
        bound((LZ4_MAX_INPUT_SIZE + 1) as c_int),
        0,
        "LZ4_compressBound(LZ4_MAX_INPUT_SIZE+1) must be 0"
    );
    for &n in &[-1i32, -2, -1000, c_int::MIN, c_int::MAX, 0x7EFF_FFFF] {
        assert_eq!(bound(n), 0, "LZ4_compressBound({n}) must be 0");
    }
    // Randomised sweep (values are compared C vs Rust inside `bound`).
    for _ in 0..400 {
        let v = rng.next_u32() as c_int;
        bound(v);
        bound(rng.range(0, LZ4_MAX_INPUT_SIZE + 4096) as c_int);
    }
    for n in 0..600i32 {
        bound(n);
    }
    for d in 0..8i32 {
        bound(LZ4_MAX_INPUT_SIZE as c_int - d);
        bound(LZ4_MAX_INPUT_SIZE as c_int + d);
    }

    unsafe {
        let (c, r) = l.sym::<FnVoidToInt>("LZ4_versionNumber");
        let a = c();
        let b = r();
        assert_eq!(a, b, "LZ4_versionNumber mismatch (C={a} Rust={b})");
        assert!(a > 10000, "unexpected version number {a}");

        let (c, r) = l.sym::<FnVersionString>("LZ4_versionString");
        let pa = c();
        let pb = r();
        assert!(!pa.is_null() && !pb.is_null());
        let sa = CStr::from_ptr(pa);
        let sb = CStr::from_ptr(pb);
        assert_eq!(
            sa.to_bytes(),
            sb.to_bytes(),
            "LZ4_versionString mismatch (C={sa:?} Rust={sb:?})"
        );
    }
}

// ===========================================================================
// Row 22 — LZ4_decompress_safe: exact / larger / smaller dstCapacity
// ===========================================================================

#[test]
fn row_22_decompress_capacities() {
    let mut rng = Rng::new(22);
    let sizes = interesting_sizes();
    for _ in 0..800 {
        let n = sizes[rng.below(sizes.len())].min(70_000);
        let sh = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
        let src = gen(&mut rng, sh, n);
        let comp = compress_both(&format!("row22 compress n={n} {sh:?}"), &src);

        // Exact capacity.
        let (ret, out) =
            diff_decompress_safe(&format!("row22 exact n={n} {sh:?}"), &comp, comp.len() as c_int, n.max(1));
        assert_eq!(ret as usize, n, "row22: exact dstCapacity should decode fully");
        assert!(out[..n] == src[..], "row22: wrong plaintext");

        // Larger capacity.
        for &extra in &[1usize, 7, 64, 1000] {
            let (ret, out) = diff_decompress_safe(
                &format!("row22 larger n={n} extra={extra} {sh:?}"),
                &comp,
                comp.len() as c_int,
                n + extra,
            );
            assert_eq!(ret as usize, n);
            assert!(out[..n] == src[..]);
        }

        // Smaller capacity → negative.
        if n > 0 {
            for &less in &[1usize, 2, 5, 13] {
                if less > n {
                    continue;
                }
                let cap = n - less;
                let (ret, _) = diff_decompress_safe(
                    &format!("row22 smaller n={n} cap={cap} {sh:?}"),
                    &comp,
                    comp.len() as c_int,
                    cap,
                );
                assert!(ret < 0, "row22: undersized dstCapacity {cap} returned {ret}");
            }
            // A random capacity sweep for good measure.
            for _ in 0..4 {
                let cap = rng.below(n + 8);
                diff_decompress_safe(
                    &format!("row22 randcap n={n} cap={cap} {sh:?}"),
                    &comp,
                    comp.len() as c_int,
                    cap,
                );
            }
        }
    }
}

// ===========================================================================
// Row 23 — compressedSize 0, dstCapacity 0 special cases
// ===========================================================================

#[test]
fn row_23_decompress_degenerate_sizes() {
    let mut rng = Rng::new(23);

    // dstCapacity == 0 with src == "\0" and srcSize == 1 → 0.
    let empty_block = [0u8];
    for _ in 0..20 {
        let (ret, _) = diff_decompress_safe("row23 empty block cap0", &empty_block, 1, 0);
        assert_eq!(ret, 0, "row23: dstCapacity 0 + \"\\0\" must return 0");
    }
    // dstCapacity == 0 with anything else → -1.
    for _ in 0..4000 {
        let len = rng.range(1, 12);
        let mut b = rand_lits(&mut rng, len);
        if len == 1 && b[0] == 0 {
            b[0] = 1; // that case is the "empty block" above
        }
        let (ret, _) = diff_decompress_safe(
            &format!("row23 cap0 len={len} {}", hexdump(&b)),
            &b,
            len as c_int,
            0,
        );
        assert_eq!(ret, -1, "row23: dstCapacity 0 with src {} must be -1", hexdump(&b));
    }
    // srcSize == 0 → -1 (for any non-zero dstCapacity).
    for _ in 0..1500 {
        let cap = rng.range(1, 4096);
        let buf = rand_lits(&mut rng, 8);
        let (ret, _) = diff_decompress_safe(&format!("row23 srcSize0 cap={cap}"), &buf, 0, cap);
        assert_eq!(ret, -1, "row23: compressedSize 0 must return -1");
    }
    // srcSize == 0 AND dstCapacity == 0 → -1 (outputSize==0 branch first).
    for _ in 0..10 {
        let buf = rand_lits(&mut rng, 4);
        diff_decompress_safe("row23 both zero", &buf, 0, 0);
    }
    // Negative dstCapacity / negative compressedSize.
    for &cs in &[-1i32, -100, c_int::MIN] {
        for &cap in &[0i32, -1, -100, 64, c_int::MIN] {
            let l = libs();
            let comp = padded(&[0u8, 0, 0, 0]);
            let mut cb = dstbuf(4096);
            let mut rb = dstbuf(4096);
            unsafe {
                let (c, r) = l.sym::<FnDecompressSafe>("LZ4_decompress_safe");
                let a = c(comp.as_ptr() as *const c_char, cb.as_mut_ptr() as *mut c_char, cs, cap);
                let b = r(comp.as_ptr() as *const c_char, rb.as_mut_ptr() as *mut c_char, cs, cap);
                same_int_and_bytes(&format!("row23 neg cs={cs} cap={cap}"), a, b, &cb, &rb);
                same_full_buffers(&format!("row23 neg cs={cs} cap={cap}"), &cb, &rb);
            }
        }
    }
    // NULL src.
    {
        let l = libs();
        let mut cb = dstbuf(64);
        let mut rb = dstbuf(64);
        unsafe {
            let (c, r) = l.sym::<FnDecompressSafe>("LZ4_decompress_safe");
            let a = c(std::ptr::null(), cb.as_mut_ptr() as *mut c_char, 1, 64);
            let b = r(std::ptr::null(), rb.as_mut_ptr() as *mut c_char, 1, 64);
            same_int_and_bytes("row23 NULL src", a, b, &cb, &rb);
            same_full_buffers("row23 NULL src", &cb, &rb);
            assert_eq!(a, -1);
        }
    }
}

// ===========================================================================
// Row 24 — output below and far above FASTLOOP_SAFE_DISTANCE == 64
// ===========================================================================

#[test]
fn row_24_fastloop_safe_distance() {
    let mut rng = Rng::new(24);

    // Outputs strictly below / around 64 bytes: safe loop only.
    for out_len in 5..=140usize {
        for rep in 0..2 {
            // Literals-only block of exactly `out_len` bytes.
            let lits = rand_lits(&mut rng, out_len);
            let (comp, plain) = Blk::new().finish(&lits);
            assert_eq!(plain.len(), out_len);
            let ctx = format!("row24 literals out={out_len} rep{rep}");
            let (ret, out) = diff_decompress_safe(&ctx, &comp, comp.len() as c_int, out_len);
            assert_eq!(ret as usize, out_len, "{ctx}: ret {ret}");
            assert!(out[..out_len] == plain[..]);

            // A block with one match, tuned to end just under/over the limit.
            if out_len >= 24 {
                let head = rand_lits(&mut rng, 8);
                let mut b = Blk::new();
                b.seq(&head, rng.range(1, 8), 4 + rng.below(6));
                let produced = b.p.len();
                if out_len >= produced + 12 {
                    let tail = rand_lits(&mut rng, out_len - produced);
                    let (comp, plain) = b.finish(&tail);
                    let ctx = format!("row24 match out={out_len} rep{rep}");
                    let (ret, out) =
                        diff_decompress_safe(&ctx, &comp, comp.len() as c_int, plain.len());
                    assert_eq!(ret as usize, plain.len(), "{ctx}: ret {ret}");
                    assert!(out[..plain.len()] == plain[..]);
                }
            }
        }
    }
    assert_eq!(FASTLOOP_SAFE_DISTANCE, 64);

    // Outputs far above 64 bytes: fast loop then safe tail.
    for _ in 0..800 {
        let nseq = rng.range(10, 60);
        let (comp, plain) = build_block(
            &mut rng,
            nseq,
            &[1, 2, 3, 4, 5, 6, 7, 8, 12, 16, 20, 64, 200, 1000],
            (0, 30),
            (4, 60),
            (5, 40),
        );
        let ctx = format!("row24 big nseq={nseq} out={}", plain.len());
        let (ret, out) = diff_decompress_safe(&ctx, &comp, comp.len() as c_int, plain.len());
        assert_eq!(ret as usize, plain.len(), "{ctx}: ret {ret}");
        assert!(out[..plain.len()] == plain[..], "{ctx}: plaintext mismatch");
    }

    // Trailing literal runs of exactly LASTLITERALS..MFLIMIT-1 (5..11) bytes:
    // the final match then ends inside `oend - MATCH_SAFEGUARD_DISTANCE`, taking
    // the slow byte-at-a-time copy branch. Legal as long as the *literals* of
    // that last sequence still end >= MFLIMIT before oend, hence the long match.
    for tail_len in 5..12usize {
        for &off in &[1usize, 2, 3, 4, 5, 7, 8, 12, 16, 40] {
            for &head_len in &[16usize, 40, 200] {
                let head = rand_lits(&mut rng, head_len);
                let mut b = Blk::new();
                let ml = rng.range(12 - tail_len + 4, 300);
                b.seq(&head, off.min(head_len), ml);
                let tail = rand_lits(&mut rng, tail_len);
                let (comp, plain) = b.finish(&tail);
                let ctx = format!("row24 shorttail t={tail_len} off={off} ml={ml}");
                let (ret, out) =
                    diff_decompress_safe(&ctx, &comp, comp.len() as c_int, plain.len());
                assert_eq!(ret as usize, plain.len(), "{ctx}: ret {ret}");
                assert!(out[..plain.len()] == plain[..], "{ctx}: plaintext mismatch");
                // Same block with a much larger dst so it goes via the fast loop.
                diff_decompress_safe(&ctx, &comp, comp.len() as c_int, plain.len() + 4096);
            }
        }
    }
}

// ===========================================================================
// Row 25 — match offsets 1, 2 and 4 (LZ4_memcpy_using_offset special cases)
// ===========================================================================

#[test]
fn row_25_offsets_1_2_4() {
    let mut rng = Rng::new(25);
    for &off in &[1usize, 2, 4] {
        for _ in 0..400 {
            // Hand-built blocks with every match at exactly this offset.
            let nseq = rng.range(1, 30);
            let (comp, plain) =
                build_block(&mut rng, nseq, &[off], (0, 25), (4, 300), (5, 40));
            let ctx = format!("row25 built off={off} nseq={nseq} out={}", plain.len());
            let (ret, out) = diff_decompress_safe(&ctx, &comp, comp.len() as c_int, plain.len());
            assert_eq!(ret as usize, plain.len(), "{ctx}: ret {ret}");
            assert!(out[..plain.len()] == plain[..], "{ctx}: plaintext mismatch");
            // Same block, generous and exact capacities plus partial decoding.
            diff_decompress_safe(&ctx, &comp, comp.len() as c_int, plain.len() + 100);
            let t = rng.below(plain.len() + 4) as c_int;
            diff_decompress_partial(&ctx, &comp, comp.len() as c_int, t, plain.len() + 8);
        }
        // Real compressor output for a period-`off` input forces offset==off.
        for _ in 0..12 {
            let base = rand_lits(&mut rng, off);
            let n = rng.range(100, 60000);
            let src: Vec<u8> = (0..n).map(|i| base[i % off]).collect();
            let comp = compress_both(&format!("row25 period {off} n={n}"), &src);
            let (ret, out) = diff_decompress_safe(
                &format!("row25 period {off} decode n={n}"),
                &comp,
                comp.len() as c_int,
                n,
            );
            assert_eq!(ret as usize, n);
            assert!(out[..n] == src[..]);
        }
    }
}

// ===========================================================================
// Row 26 — offsets 3,5,6,7 (<8), 8..15 (<16) and >= 16 (wildCopy32)
// ===========================================================================

#[test]
fn row_26_offsets_3_to_15_and_above() {
    let mut rng = Rng::new(26);
    let groups: [&[usize]; 4] = [
        &[3, 5, 6, 7],
        &[8, 9, 10, 11, 12, 13, 14, 15],
        &[16, 17, 18, 31, 32, 33, 64, 100],
        &[255, 256, 257, 1000, 4096, 32768, 65534, 65535],
    ];
    for (gi, g) in groups.iter().enumerate() {
        for &off in g.iter() {
            for _ in 0..120 {
                let nseq = rng.range(1, 24);
                let (comp, plain) =
                    build_block(&mut rng, nseq, &[off], (0, 40), (4, 200), (5, 40));
                let ctx = format!("row26 g{gi} off={off} nseq={nseq} out={}", plain.len());
                let (ret, out) =
                    diff_decompress_safe(&ctx, &comp, comp.len() as c_int, plain.len());
                assert_eq!(ret as usize, plain.len(), "{ctx}: ret {ret}");
                assert!(out[..plain.len()] == plain[..], "{ctx}: plaintext mismatch");
            }
        }
    }
    // Mixed-offset blocks so consecutive sequences change copy strategy.
    for _ in 0..1000 {
        let all: Vec<usize> = groups.iter().flat_map(|g| g.iter().cloned()).collect();
        let nseq = rng.range(5, 50);
        let (comp, plain) = build_block(&mut rng, nseq, &all, (0, 30), (4, 120), (5, 40));
        let ctx = format!("row26 mixed nseq={nseq} out={}", plain.len());
        let (ret, out) = diff_decompress_safe(&ctx, &comp, comp.len() as c_int, plain.len());
        assert_eq!(ret as usize, plain.len(), "{ctx}: ret {ret}");
        assert!(out[..plain.len()] == plain[..]);
    }
    // Real compressor output with periodic data of period >= 16.
    for &period in &[16usize, 17, 40, 300, 5000] {
        let base = rand_lits(&mut rng, period);
        let n = rng.range(period * 8, 80000);
        let src: Vec<u8> = (0..n).map(|i| base[i % period]).collect();
        let comp = compress_both(&format!("row26 period {period}"), &src);
        let (ret, out) = diff_decompress_safe(
            &format!("row26 period {period} decode"),
            &comp,
            comp.len() as c_int,
            n,
        );
        assert_eq!(ret as usize, n);
        assert!(out[..n] == src[..]);
    }
}

// ===========================================================================
// Row 27 — token nibble == 15 with multi-255 extension bytes
// ===========================================================================

#[test]
fn row_27_token_15_multi255_extensions() {
    let mut rng = Rng::new(27);

    // Literal lengths straddling every 255 boundary.
    for k in 0..8usize {
        for d in 0..3usize {
            let ll = 15 + k * 255 + d;
            let lits = rand_lits(&mut rng, ll);
            let mut b = Blk::new();
            b.seq(&lits, rng.range(1, 16), 4 + rng.below(20));
            let tail = rand_lits_range(&mut rng, (12, 30));
            let (comp, plain) = b.finish(&tail);
            let ctx = format!("row27 ll={ll} out={}", plain.len());
            let (ret, out) = diff_decompress_safe(&ctx, &comp, comp.len() as c_int, plain.len());
            assert_eq!(ret as usize, plain.len(), "{ctx}: ret {ret}");
            assert!(out[..plain.len()] == plain[..], "{ctx}: plaintext mismatch");
        }
    }

    // Match lengths straddling every 255 boundary (token ML nibble == 15).
    for k in 0..8usize {
        for d in 0..3usize {
            let ml = 19 + k * 255 + d; // ml - 4 >= 15
            let head = rand_lits_range(&mut rng, (4, 40));
            let mut b = Blk::new();
            let off = rng.range(1, head.len().max(1));
            b.seq(&head, off, ml);
            let tail = rand_lits_range(&mut rng, (12, 30));
            let (comp, plain) = b.finish(&tail);
            let ctx = format!("row27 ml={ml} off={off} out={}", plain.len());
            let (ret, out) = diff_decompress_safe(&ctx, &comp, comp.len() as c_int, plain.len());
            assert_eq!(ret as usize, plain.len(), "{ctx}: ret {ret}");
            assert!(out[..plain.len()] == plain[..], "{ctx}: plaintext mismatch");
        }
    }

    // Both nibbles == 15 in the same token, plus a final literal run >= 15.
    for _ in 0..1000 {
        let ll = rng.range(15, 1200);
        let ml = rng.range(19, 1500);
        let lits = rand_lits(&mut rng, ll);
        let mut b = Blk::new();
        b.seq(&lits, rng.range(1, ll.min(65535)), ml);
        let tail = rand_lits_range(&mut rng, (15, 900));
        let (comp, plain) = b.finish(&tail);
        let ctx = format!("row27 both ll={ll} ml={ml}");
        let (ret, out) = diff_decompress_safe(&ctx, &comp, comp.len() as c_int, plain.len());
        assert_eq!(ret as usize, plain.len(), "{ctx}: ret {ret}");
        assert!(out[..plain.len()] == plain[..]);
    }

    // Real data: long literal runs from incompressible input and a >100 KB
    // single-byte run (match length chain of 4*255 bytes).
    for _ in 0..20 {
        let n = rng.range(5000, 40000);
        let src = gen_incompressible(&mut rng, n);
        let comp = compress_both("row27 real literals", &src);
        let (ret, out) =
            diff_decompress_safe("row27 real literals decode", &comp, comp.len() as c_int, n);
        assert_eq!(ret as usize, n);
        assert!(out[..n] == src[..]);
    }
    for &n in &[100 * 1024usize, 150_000, 300_000] {
        let src = vec![0x7Eu8; n];
        let comp = compress_both("row27 real long match", &src);
        let (ret, out) =
            diff_decompress_safe("row27 real long match decode", &comp, comp.len() as c_int, n);
        assert_eq!(ret as usize, n);
        assert!(out[..n] == src[..]);
    }
}

// ===========================================================================
// Row 28 — literal length <= 14 shortcut (ip<shortiend && op<=shortoend)
// ===========================================================================

#[test]
fn row_28_two_stage_shortcut() {
    let mut rng = Rng::new(28);

    // Sequences with ll <= 14 and ml in 4..18 (ML nibble != 15) and offset >= 8
    // satisfy both shortcut stages. Interleave failures of stage 2 (offset < 8
    // or ML nibble == 15) so both exits are taken.
    for _ in 0..1500 {
        let nseq = rng.range(2, 40);
        let mut b = Blk::new();
        // Prime the output so large offsets are legal.
        let head = rand_lits_range(&mut rng, (14, 40));
        b.seq(&head, rng.range(8, head.len()), rng.range(4, 18));
        for _ in 0..nseq {
            let ll = rng.below(15); // 0..=14 → shortcut eligible
            let lits = rand_lits(&mut rng, ll);
            let avail = b.p.len() + ll;
            let off = match rng.below(3) {
                0 => rng.range(1, 7.min(avail)),            // stage 2 fails (offset < 8)
                1 => rng.range(8, 64.min(avail).max(8)).min(avail),
                _ => rng.range(8, avail),
            }
            .max(1)
            .min(avail);
            let ml = match rng.below(3) {
                0 => rng.range(4, 18),  // ML nibble 0..13 → shortcut ok
                1 => rng.range(19, 60), // ML nibble == 15 → stage 2 fails
                _ => 18,                // ML nibble == 14
            };
            b.seq(&lits, off, ml);
        }
        let tail = rand_lits_range(&mut rng, (12, 60));
        let (comp, plain) = b.finish(&tail);
        let ctx = format!("row28 nseq={nseq} out={}", plain.len());
        // Exact capacity (drives the safe loop for the last 64 bytes) and a
        // small total output (entirely in the safe loop).
        let (ret, out) = diff_decompress_safe(&ctx, &comp, comp.len() as c_int, plain.len());
        assert_eq!(ret as usize, plain.len(), "{ctx}: ret {ret}");
        assert!(out[..plain.len()] == plain[..], "{ctx}: plaintext mismatch");
        diff_decompress_safe(&ctx, &comp, comp.len() as c_int, plain.len() + 200);
    }

    // Small blocks (< 64 bytes of output) go through the safe loop from the
    // very first token, which is where the shortcut lives.
    for _ in 0..4000 {
        let head = rand_lits_range(&mut rng, (8, 14));
        let mut b = Blk::new();
        let off = rng.range(8, head.len()).max(1).min(head.len());
        b.seq(&head, off, rng.range(4, 18));
        let tail = rand_lits_range(&mut rng, (12, 20));
        let (comp, plain) = b.finish(&tail);
        let ctx = format!("row28 small out={}", plain.len());
        let (ret, out) = diff_decompress_safe(&ctx, &comp, comp.len() as c_int, plain.len());
        assert_eq!(ret as usize, plain.len(), "{ctx}: ret {ret}");
        assert!(out[..plain.len()] == plain[..]);
    }
}

// ===========================================================================
// Row 29 — malformed input fuzzing: C and Rust must return the *identical*
// value (0 / -1 / negative offsets), not merely "both negative".
// ===========================================================================

#[test]
fn row_29_fuzz_malformed_blocks() {
    let mut rng = Rng::new(29);

    // Pool of valid blocks to corrupt.
    let mut pool: Vec<(Vec<u8>, usize)> = Vec::new();
    for _ in 0..120 {
        let n = rng.range(1, 3000);
        let sh = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
        let src = gen(&mut rng, sh, n);
        let comp = compress_both("row29 pool real", &src);
        pool.push((comp, n));
    }
    for _ in 0..120 {
        let nseq = rng.range(1, 25);
        let (comp, plain) = build_block(
            &mut rng,
            nseq,
            &[1, 2, 3, 4, 5, 7, 8, 13, 16, 40, 300, 65535],
            (0, 40),
            (4, 400),
            (5, 40),
        );
        let n = plain.len();
        pool.push((comp, n));
    }

    for iter in 0..2_000_000usize {
        let (base, orig) = &pool[rng.below(pool.len())];
        let mut c2 = base.clone();
        let kind = rng.below(7);
        match kind {
            0 => {
                if !c2.is_empty() {
                    let k = rng.below(c2.len());
                    c2[k] = rng.byte();
                }
            }
            1 => {
                let nb = rng.range(1, 4);
                for _ in 0..nb {
                    if c2.is_empty() {
                        break;
                    }
                    let i = rng.below(c2.len());
                    c2[i] ^= 1u8 << rng.below(8);
                }
            }
            2 => {
                let t = rng.below(c2.len() + 1);
                c2.truncate(t);
            }
            3 => {
                // Corrupt the very first token / offset field.
                if c2.len() >= 4 {
                    let k = rng.below(4);
                    c2[k] = rng.byte();
                }
            }
            4 => {
                // Pure garbage.
                let n = rng.range(0, 48);
                c2 = rand_lits(&mut rng, n);
            }
            5 => {
                // Append junk (declared srcSize longer than the real block).
                let n = rng.range(1, 8);
                c2.extend(rand_lits(&mut rng, n));
            }
            _ => {
                // Zero out a run, which tends to create offset==0 matches.
                if !c2.is_empty() {
                    let k = rng.below(c2.len());
                    let len = rng.range(1, 4).min(c2.len() - k);
                    for j in 0..len {
                        c2[k + j] = 0;
                    }
                }
            }
        }

        // Generous destination so neither implementation can run off the end of
        // its own buffer; both get their own sentinel-filled copy.
        let cap = orig + rng.range(0, 256);
        let ctx = format!("row29 iter={iter} kind={kind} clen={} cap={cap}", c2.len());
        diff_decompress_safe(&ctx, &c2, c2.len() as c_int, cap);

        // Also declare a wrong (larger/smaller) compressedSize.
        if !c2.is_empty() {
            let cs = rng.range(0, c2.len() + 4) as c_int;
            diff_decompress_safe(&format!("{ctx} cs={cs}"), &c2, cs, cap);
        }

        // And partial decoding of the same corrupt bytes.
        let t = rng.below(orig + 8) as c_int;
        diff_decompress_partial(&format!("{ctx} partial t={t}"), &c2, c2.len() as c_int, t, cap);

        // Tight destination buffers as well.
        if iter % 4 == 0 {
            let tight = rng.below(orig + 2);
            diff_decompress_safe(&format!("{ctx} tight={tight}"), &c2, c2.len() as c_int, tight);
        }
    }

    // Explicitly hand-built violations of the end-of-block rules.
    for _ in 0..60_000 {
        // Last literal run shorter than LASTLITERALS == 5.
        let head = rand_lits_range(&mut rng, (8, 40));
        let mut b = Blk::new();
        b.seq(&head, rng.range(1, head.len()), rng.range(4, 40));
        let t = rng.below(5);
        let tail = rand_lits(&mut rng, t);
        let (comp, plain) = b.finish(&tail);
        diff_decompress_safe(
            &format!("row29 short-tail t={t}"),
            &comp,
            comp.len() as c_int,
            plain.len(),
        );

        // Offset pointing before the start of the buffer.
        let lits = rand_lits_range(&mut rng, (4, 20));
        let mut b = Blk::new();
        b.c.push(((lits.len().min(15) as u8) << 4) | 4);
        if lits.len() >= 15 {
            Blk::push_ext(&mut b.c, lits.len() - 15);
        }
        b.c.extend_from_slice(&lits);
        let bad_off = lits.len() + rng.range(1, 5000);
        b.c.push((bad_off & 0xFF) as u8);
        b.c.push(((bad_off >> 8) & 0xFF) as u8);
        let tail = rand_lits(&mut rng, 20);
        b.c.push(0xF0u8 | 0);
        Blk::push_ext(&mut b.c, tail.len() - 15);
        b.c.extend_from_slice(&tail);
        let comp = b.c.clone();
        diff_decompress_safe(
            &format!("row29 bad-offset off={bad_off}"),
            &comp,
            comp.len() as c_int,
            lits.len() + 8 + tail.len() + 64,
        );

        // Truncated length-extension bytes: token nibble 15, then only 255s.
        let mut t2: Vec<u8> = vec![0xF0];
        let k = rng.range(1, 6);
        for _ in 0..k {
            t2.push(255);
        }
        diff_decompress_safe(&format!("row29 trunc-ext k={k}"), &t2, t2.len() as c_int, 4096);
    }
}

// ===========================================================================
// Row 30 — in-place decompression, src at the tail of the dst buffer
// ===========================================================================

#[test]
fn row_30_in_place_decompression() {
    let l = libs();
    let mut rng = Rng::new(30);
    let sizes = interesting_sizes();
    for i in 0..600usize {
        let n = if i < sizes.len() { sizes[i].min(70_000) } else { rng.range(1, 70_000) };
        if n == 0 {
            continue;
        }
        let sh = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
        let src = gen(&mut rng, sh, n);
        let comp = compress_both(&format!("row30 compress n={n} {sh:?}"), &src);
        let margin = (comp.len() >> 8) + 32;
        let bufsize = (n + margin).max(comp.len() + 1);
        let off = bufsize - comp.len();

        let mut cb = vec![SENT; bufsize + GUARD];
        let mut rb = vec![SENT; bufsize + GUARD];
        cb[off..off + comp.len()].copy_from_slice(&comp);
        rb[off..off + comp.len()].copy_from_slice(&comp);

        unsafe {
            let (c, r) = l.sym::<FnDecompressSafe>("LZ4_decompress_safe");
            let cp = cb.as_mut_ptr();
            let rp = rb.as_mut_ptr();
            let a = c(
                cp.add(off) as *const c_char,
                cp as *mut c_char,
                comp.len() as c_int,
                n as c_int,
            );
            let b = r(
                rp.add(off) as *const c_char,
                rp as *mut c_char,
                comp.len() as c_int,
                n as c_int,
            );
            let ctx = format!("row30 in-place n={n} {sh:?} bufsize={bufsize}");
            same_int_and_bytes(&ctx, a, b, &cb, &rb);
            same_full_buffers(&ctx, &cb, &rb);
            assert_eq!(a as usize, n, "{ctx}: in-place decode returned {a}");
            assert!(cb[..n] == src[..], "{ctx}: in-place plaintext mismatch");
        }
    }
}

// ===========================================================================
// Row 31 — LZ4_decompress_safe_partial: targetOutputSize 0 / oversized /
//          dstCapacity < targetOutputSize
// ===========================================================================

#[test]
fn row_31_partial_basic() {
    let mut rng = Rng::new(31);
    let sizes = interesting_sizes();
    for _ in 0..600 {
        let n = sizes[rng.below(sizes.len())].min(70_000);
        let sh = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
        let src = gen(&mut rng, sh, n);
        let comp = compress_both(&format!("row31 compress n={n} {sh:?}"), &src);
        let cl = comp.len() as c_int;

        // targetOutputSize == 0 → 0 bytes produced.
        for &cap in &[0usize, 1, n + 8] {
            let ctx = format!("row31 target0 n={n} cap={cap} {sh:?}");
            let (ret, _) = diff_decompress_partial(&ctx, &comp, cl, 0, cap);
            assert_eq!(ret, 0, "{ctx}: targetOutputSize 0 must return 0");
        }

        // targetOutputSize > decompressed size with exact srcSize.
        for &extra in &[1usize, 5, 64, 1000] {
            let ctx = format!("row31 over n={n} extra={extra} {sh:?}");
            let (ret, out) =
                diff_decompress_partial(&ctx, &comp, cl, (n + extra) as c_int, n + extra);
            assert_eq!(ret as usize, n, "{ctx}: ret {ret}");
            assert!(out[..n] == src[..]);
        }

        // dstCapacity < targetOutputSize → MIN() applied.
        if n > 0 {
            for _ in 0..4 {
                let cap = rng.below(n + 1);
                let target = (cap + rng.range(1, 100)) as c_int;
                let ctx = format!("row31 min n={n} cap={cap} target={target} {sh:?}");
                let (ret, out) = diff_decompress_partial(&ctx, &comp, cl, target, cap);
                assert!(ret >= 0, "{ctx}: partial decode returned {ret}");
                assert!(ret as usize <= cap, "{ctx}: wrote {ret} into cap {cap}");
                if ret > 0 {
                    assert!(out[..ret as usize] == src[..ret as usize], "{ctx}: prefix mismatch");
                }
            }
        }

        // Negative targetOutputSize / dstCapacity.
        for &t in &[-1i32, -1000, c_int::MIN] {
            diff_decompress_partial(
                &format!("row31 negtarget n={n} t={t}"),
                &comp,
                cl,
                t,
                n + 8,
            );
        }
    }
}

// ===========================================================================
// Row 32 — LZ4_decompress_safe_partial stopping inside a literal run / a match
// ===========================================================================

#[test]
fn row_32_partial_mid_literal_and_mid_match() {
    let mut rng = Rng::new(32);
    for _ in 0..600 {
        // Blocks with alternating long literal runs and long matches, so that
        // scanning every target hits both mid-literal and mid-match stops.
        let mut b = Blk::new();
        let head = rand_lits_range(&mut rng, (20, 80));
        b.seq(&head, rng.range(1, head.len()), rng.range(30, 200));
        for _ in 0..rng.range(2, 8) {
            let lits = rand_lits_range(&mut rng, (10, 300));
            let avail = b.p.len() + lits.len();
            let off = rng.range(1, avail.min(65535));
            b.seq(&lits, off, rng.range(20, 400));
        }
        let tail = rand_lits_range(&mut rng, (12, 60));
        let (comp, plain) = b.finish(&tail);
        let cl = comp.len() as c_int;
        let n = plain.len();

        // Sweep every target across the whole plaintext (stride keeps it fast).
        let stride = (n / 60).max(1);
        let mut t = 0usize;
        while t <= n + 16 {
            let ctx = format!("row32 n={n} target={t}");
            let (ret, out) = diff_decompress_partial(&ctx, &comp, cl, t as c_int, n + 32);
            assert!(ret >= 0, "{ctx}: returned {ret}");
            assert!(ret as usize <= n.max(t), "{ctx}: ret {ret}");
            if ret > 0 {
                assert!(
                    out[..ret as usize] == plain[..ret as usize],
                    "{ctx}: prefix mismatch at {:?}",
                    first_diff(&out[..ret as usize], &plain[..ret as usize])
                );
            }
            // Same target with dstCapacity below it.
            if t > 0 {
                let cap = rng.below(t);
                diff_decompress_partial(
                    &format!("{ctx} cap={cap}"),
                    &comp,
                    cl,
                    t as c_int,
                    cap,
                );
            }
            t += stride;
        }
    }

    // Real compressor output, random targets.
    for _ in 0..500 {
        let n = rng.range(100, 40000);
        let sh = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
        let src = gen(&mut rng, sh, n);
        let comp = compress_both("row32 real", &src);
        let cl = comp.len() as c_int;
        for _ in 0..8 {
            let t = rng.below(n + 32);
            let ctx = format!("row32 real n={n} {sh:?} target={t}");
            let (ret, out) = diff_decompress_partial(&ctx, &comp, cl, t as c_int, n + 32);
            assert!(ret >= 0, "{ctx}: returned {ret}");
            if ret > 0 {
                assert!(out[..ret as usize] == src[..ret as usize], "{ctx}: prefix mismatch");
            }
        }
    }
}

// ===========================================================================
// Row 33 — LZ4_decompress_fast with exact originalSize
// ===========================================================================

#[test]
fn row_33_decompress_fast() {
    let mut rng = Rng::new(33);

    // originalSize 0 (the empty block) and 1..11 (< MFLIMIT, literals only).
    let (comp0, plain0) = Blk::new().finish(&[]);
    assert_eq!(comp0, vec![0u8]);
    assert!(plain0.is_empty());
    for _ in 0..10 {
        let (ret, _) = diff_decompress_fast("row33 empty", &comp0, 0);
        assert_eq!(ret, 1, "row33: empty block should consume 1 byte, got {ret}");
    }
    for n in 1..=11usize {
        for _ in 0..5 {
            let lits = rand_lits(&mut rng, n);
            let (comp, plain) = Blk::new().finish(&lits);
            let ctx = format!("row33 tiny n={n}");
            let (ret, out) = diff_decompress_fast(&ctx, &comp, n as c_int);
            assert_eq!(ret as usize, comp.len(), "{ctx}: ret {ret}");
            assert!(out[..n] == plain[..], "{ctx}: plaintext mismatch");
        }
    }

    // Normal blocks produced by the real compressor.
    let sizes = interesting_sizes();
    for _ in 0..600 {
        let n = sizes[rng.below(sizes.len())].min(70_000);
        let sh = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
        let src = gen(&mut rng, sh, n);
        let comp = compress_both(&format!("row33 compress n={n} {sh:?}"), &src);
        let ctx = format!("row33 fast n={n} {sh:?}");
        let (ret, out) = diff_decompress_fast(&ctx, &comp, n as c_int);
        assert_eq!(ret as usize, comp.len(), "{ctx}: ret {ret}, comp {}", comp.len());
        if n > 0 {
            assert!(out[..n] == src[..], "{ctx}: plaintext mismatch");
        }
    }

    // Hand-built blocks with long literal and match lengths, exercising
    // read_long_length_no_check.
    for _ in 0..1000 {
        let nseq = rng.range(1, 20);
        let (comp, plain) = build_block(
            &mut rng,
            nseq,
            &[1, 2, 3, 4, 5, 8, 16, 100, 1000],
            (0, 900),
            (4, 1400),
            (12, 900),
        );
        let ctx = format!("row33 built out={}", plain.len());
        let (ret, out) = diff_decompress_fast(&ctx, &comp, plain.len() as c_int);
        assert_eq!(ret as usize, comp.len(), "{ctx}: ret {ret}");
        assert!(out[..plain.len()] == plain[..], "{ctx}: plaintext mismatch");
    }

    // Wrong (too small / too large) originalSize on a *well-formed* block:
    // decompress_fast detects output overflow and returns -1.
    for _ in 0..600 {
        let n = rng.range(30, 4000);
        let src = gen_textlike(&mut rng, n);
        let comp = compress_both("row33 wrongsize", &src);
        for &d in &[1usize, 2, 7, 13] {
            if d < n {
                diff_decompress_fast(
                    &format!("row33 small orig n={n} d={d}"),
                    &comp,
                    (n - d) as c_int,
                );
            }
        }
    }
}

// ===========================================================================
// Row 34 — legacy LZ4_uncompress / LZ4_uncompress_unknownOutputSize
// ===========================================================================

#[test]
fn row_34_legacy_uncompress_wrappers() {
    let l = libs();
    let mut rng = Rng::new(34);
    let sizes = interesting_sizes();

    for _ in 0..600 {
        let n = sizes[rng.below(sizes.len())].min(70_000);
        let sh = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
        let src = gen(&mut rng, sh, n);
        let comp = compress_both(&format!("row34 compress n={n} {sh:?}"), &src);
        let p = padded(&comp);

        // LZ4_uncompress == LZ4_decompress_fast
        unsafe {
            let (c, r) = l.sym::<FnUncompress>("LZ4_uncompress");
            let mut cb = dstbuf(n);
            let mut rb = dstbuf(n);
            let a = c(p.as_ptr() as *const c_char, cb.as_mut_ptr() as *mut c_char, n as c_int);
            let b = r(p.as_ptr() as *const c_char, rb.as_mut_ptr() as *mut c_char, n as c_int);
            let ctx = format!("row34 LZ4_uncompress n={n} {sh:?}");
            same_int_and_bytes(&ctx, a, b, &cb, &rb);
            same_full_buffers(&ctx, &cb, &rb);
            assert_eq!(a as usize, comp.len(), "{ctx}: ret {a}");
            if n > 0 {
                assert!(cb[..n] == src[..], "{ctx}: plaintext mismatch");
            }
            // Must agree with LZ4_decompress_fast.
            let (fret, fout) = diff_decompress_fast(&ctx, &comp, n as c_int);
            assert_eq!(a, fret, "{ctx}: LZ4_uncompress != LZ4_decompress_fast");
            assert_eq!(&cb[..], &fout[..], "{ctx}: LZ4_uncompress output differs");
        }

        // LZ4_uncompress_unknownOutputSize == LZ4_decompress_safe
        for &cap in &[n, n + 64, n.saturating_sub(1)] {
            unsafe {
                let (c, r) = l.sym::<FnUncompressUnknown>("LZ4_uncompress_unknownOutputSize");
                let mut cb = dstbuf(cap);
                let mut rb = dstbuf(cap);
                let a = c(
                    p.as_ptr() as *const c_char,
                    cb.as_mut_ptr() as *mut c_char,
                    comp.len() as c_int,
                    cap as c_int,
                );
                let b = r(
                    p.as_ptr() as *const c_char,
                    rb.as_mut_ptr() as *mut c_char,
                    comp.len() as c_int,
                    cap as c_int,
                );
                let ctx = format!("row34 unknownOutputSize n={n} cap={cap} {sh:?}");
                same_int_and_bytes(&ctx, a, b, &cb, &rb);
                same_full_buffers(&ctx, &cb, &rb);
                let (sret, sout) =
                    diff_decompress_safe(&ctx, &comp, comp.len() as c_int, cap);
                assert_eq!(a, sret, "{ctx}: legacy wrapper != LZ4_decompress_safe");
                assert_eq!(&cb[..], &sout[..], "{ctx}: legacy wrapper output differs");
            }
        }
    }

    // Legacy safe wrapper on degenerate inputs.
    for _ in 0..2000 {
        let len = rng.range(0, 12);
        let b = rand_lits(&mut rng, len);
        let p = padded(&b);
        let cap = rng.range(0, 512);
        unsafe {
            let (c, r) = l.sym::<FnUncompressUnknown>("LZ4_uncompress_unknownOutputSize");
            let mut cbuf = dstbuf(cap);
            let mut rbuf = dstbuf(cap);
            let a = c(
                p.as_ptr() as *const c_char,
                cbuf.as_mut_ptr() as *mut c_char,
                len as c_int,
                cap as c_int,
            );
            let bb = r(
                p.as_ptr() as *const c_char,
                rbuf.as_mut_ptr() as *mut c_char,
                len as c_int,
                cap as c_int,
            );
            let ctx = format!("row34 degenerate len={len} cap={cap}");
            same_int_and_bytes(&ctx, a, bb, &cbuf, &rbuf);
            same_full_buffers(&ctx, &cbuf, &rbuf);
        }
    }
}
