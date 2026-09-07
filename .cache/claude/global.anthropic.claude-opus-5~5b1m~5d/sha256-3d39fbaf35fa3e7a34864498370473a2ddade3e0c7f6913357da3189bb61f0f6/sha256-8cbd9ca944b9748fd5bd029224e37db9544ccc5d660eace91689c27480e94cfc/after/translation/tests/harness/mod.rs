//! Shared differential-test harness.
//!
//! Loads BOTH shared libraries (the C one built by CMake and the Rust `cdylib`)
//! with `libloading` and calls `dequantize_granule` through the FFI boundary on
//! each, so the `#[no_mangle] extern "C"` wrapper is what gets exercised — never
//! a direct Rust call.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// C type mirrors (layout must match c_src/include/lib.h exactly)
// ---------------------------------------------------------------------------

/// `typedef struct { const uint8_t *buf; int pos, limit; } bs_t;`
#[repr(C)]
#[derive(Clone, Copy)]
pub struct BsT {
    pub buf: *const u8,
    pub pos: i32,
    pub limit: i32,
}

/// `typedef struct { float scf[3*64]; uint8_t total_bands, stereo_bands,
///                   bitalloc[64], scfcod[64]; } L12_scale_info;`
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Sci {
    pub scf: [f32; 3 * 64],
    pub total_bands: u8,
    pub stereo_bands: u8,
    pub bitalloc: [u8; 64],
    pub scfcod: [u8; 64],
}

impl Sci {
    pub fn zeroed() -> Self {
        Sci {
            scf: [0.0; 192],
            total_bands: 0,
            stereo_bands: 0,
            bitalloc: [0; 64],
            scfcod: [0; 64],
        }
    }

    /// Byte at flat index `i` of the `bitalloc`/`scfcod` region — this is what
    /// `sci->bitalloc[i]` reads for `i >= 64` (out of bounds, but still inside
    /// the object).
    pub fn alloc_byte(&self, i: i32) -> u8 {
        if i < 64 {
            self.bitalloc[i as usize]
        } else {
            self.scfcod[(i - 64) as usize]
        }
    }
}

pub type DequantFn =
    unsafe extern "C" fn(*mut f32, *mut BsT, *mut Sci, i32) -> i32;

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

struct Libs {
    c: Library,
    rust: Library,
    c_path: PathBuf,
    rust_path: PathBuf,
}

// SAFETY: the loaded libraries are pure leaf code with no interior state that
// is mutated concurrently; both are kept alive for the whole process.
unsafe impl Send for Libs {}
unsafe impl Sync for Libs {}

static LIBS: OnceLock<Libs> = OnceLock::new();

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn find_c_so() -> PathBuf {
    let build = manifest_dir().join("../c_src/build");
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("so") {
                candidates.push(p);
            }
        }
    }
    assert!(
        !candidates.is_empty(),
        "no .so found in {}. Build it with:\n  cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        build.display()
    );
    candidates.sort();
    candidates.remove(0)
}

fn find_rust_so() -> PathBuf {
    let base = manifest_dir().join("target");
    let name = "libdequantize_granule_lib.so";
    let order: [&str; 2] = if cfg!(debug_assertions) {
        ["debug", "release"]
    } else {
        ["release", "debug"]
    };
    for profile in order {
        let p = base.join(profile).join(name);
        if p.exists() {
            return p;
        }
    }
    panic!(
        "{} not found under {}. Build it with: cargo build --release --offline",
        name,
        base.display()
    );
}

fn libs() -> &'static Libs {
    LIBS.get_or_init(|| {
        let c_path = find_c_so();
        let rust_path = find_rust_so();
        let c = unsafe { Library::new(&c_path) }
            .unwrap_or_else(|e| panic!("dlopen {}: {e}", c_path.display()));
        let rust = unsafe { Library::new(&rust_path) }
            .unwrap_or_else(|e| panic!("dlopen {}: {e}", rust_path.display()));
        Libs {
            c,
            rust,
            c_path,
            rust_path,
        }
    })
}

pub fn c_so_path() -> &'static Path {
    &libs().c_path
}
pub fn rust_so_path() -> &'static Path {
    &libs().rust_path
}

pub fn c_dequantize() -> DequantFn {
    let l = libs();
    let s: Symbol<DequantFn> = unsafe { l.c.get(b"dequantize_granule\0") }
        .expect("C .so does not export dequantize_granule");
    *s
}

pub fn rust_dequantize() -> DequantFn {
    let l = libs();
    let s: Symbol<DequantFn> = unsafe { l.rust.get(b"dequantize_granule\0") }
        .expect("Rust .so does not export dequantize_granule");
    *s
}

// ---------------------------------------------------------------------------
// Deterministic RNG (splitmix64) — fixed seed per test for reproducibility
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    /// Uniform in `0..n` (`n > 0`).
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
    /// Inclusive range.
    pub fn range(&mut self, lo: u32, hi: u32) -> u32 {
        lo + self.below(hi - lo + 1)
    }
    pub fn fill(&mut self, buf: &mut [u8]) {
        for chunk in buf.chunks_mut(8) {
            let v = self.next_u64().to_le_bytes();
            chunk.copy_from_slice(&v[..chunk.len()]);
        }
    }
    pub fn bool(&mut self) -> bool {
        self.next_u32() & 1 == 1
    }
}

// ---------------------------------------------------------------------------
// Safety oracle
//
// The C code performs completely unchecked reads and writes.  Before running a
// case on the real libraries we replay the exact control flow of
// `dequantize_granule`/`get_bits` symbolically to work out
//   * how many `f32` slots will be written (so the output buffer is big enough),
//   * whether any bitstream read would fall outside `buf`.
// A case that would read out of bounds, or read past the `L12_scale_info`
// object, is *skipped* rather than run — such a case has no defined value to
// compare against on either side.
// ---------------------------------------------------------------------------

/// How many bytes `get_bits(bs, n)` would touch, and the byte index it starts
/// at. Returns `None` if no read happens (limit exceeded).
fn gb_read_span(pos_before: i32, n: i32, pos_after: i32, limit: i32) -> Option<(i64, i64)> {
    if pos_after > limit {
        return None;
    }
    let s = (pos_before & 7) as u32;
    let shl0 = (n as u32).wrapping_add(s) as i32;
    let extra: i64 = if shl0 > 0 {
        // body runs while shl0 - 8t > 0  =>  t_max = ceil(shl0/8) - 1
        (((shl0 as i64) + 7) / 8) - 1
    } else {
        0
    };
    let count = 1 + extra.max(0);
    let start = (pos_before >> 3) as i64;
    Some((start, count))
}

/// The `n` that `get_bits` is called with for a grouped (`ba >= 17`) band, plus
/// the `mod` value, reproducing the x86-64 masked shift.
pub fn grouped_params(ba: i32) -> (u32, i32) {
    let m = (2i32.wrapping_shl(((ba - 17) as u32) & 31) as u32).wrapping_add(1);
    let n = m.wrapping_add(2).wrapping_sub(m >> 3) as i32;
    (m, n)
}

pub struct Plan {
    /// Number of `f32` slots that must be addressable in `grbuf`.
    pub grbuf_len: usize,
    /// Final `bs->pos` (for cross-checking against the libraries).
    pub final_pos: i32,
}

/// Returns `None` when the configuration is not comparable (OOB bitstream read,
/// read past the `sci` object, negative `grbuf` index, or an absurd buffer
/// requirement).
pub fn plan(sci: &Sci, group_size: i32, pos0: i32, limit: i32, buf_len: usize) -> Option<Plan> {
    plan_range(sci, group_size, pos0, limit, 0, buf_len as i64)
}

/// Like [`plan`], but the readable byte window relative to `bs->buf` is
/// `lo..hi` — which may start *below* zero when `bs->buf` points into the
/// middle of a larger allocation. That is what makes a negative `bs->pos` with
/// an actual read (and therefore the arithmetic `bs->pos >> 3`) observable.
pub fn plan_range(
    sci: &Sci,
    group_size: i32,
    pos0: i32,
    limit: i32,
    lo: i64,
    hi: i64,
) -> Option<Plan> {
    let mut alloc = [0u8; 128];
    alloc[..64].copy_from_slice(&sci.bitalloc);
    alloc[64..].copy_from_slice(&sci.scfcod);
    plan_core(&alloc, sci.total_bands, group_size, pos0, limit, lo, hi)
}

/// The planner, generalized over the raw `bitalloc`/`scfcod` byte region.
///
/// `alloc` holds the bytes starting at `offsetof(L12_scale_info, bitalloc)`. Its
/// length is how far the C code may legitimately walk: 128 for a bare
/// `L12_scale_info`, or more when the struct is embedded in a padded allocation
/// whose trailing bytes the test controls (which is what makes
/// `total_bands > 64` comparable at all).
pub fn plan_core(
    alloc: &[u8],
    total_bands: u8,
    group_size: i32,
    pos0: i32,
    limit: i32,
    lo: i64,
    hi: i64,
) -> Option<Plan> {
    const GRBUF_CAP: i64 = 1 << 21;
    let mut pos = pos0;
    let mut choff: i32 = 576;
    let mut max_idx: i64 = -1;

    for j in 0..4i32 {
        let mut dst: i64 = group_size.wrapping_mul(j) as i64;
        let mut i = 0i32;
        while i < 2i32.wrapping_mul(total_bands as i32) {
            if i as usize >= alloc.len() {
                // would read past the memory the test controls -> indeterminate
                return None;
            }
            let ba = alloc[i as usize] as i32;
            if ba != 0 {
                let touch = |max_idx: &mut i64| -> bool {
                    if group_size > 0 {
                        let lo = dst;
                        let hi = dst + (group_size as i64) - 1;
                        if lo < 0 || hi >= GRBUF_CAP {
                            return false;
                        }
                        if hi > *max_idx {
                            *max_idx = hi;
                        }
                    }
                    true
                };
                if ba < 17 {
                    if !touch(&mut max_idx) {
                        return None;
                    }
                    let mut k = 0i32;
                    while k < group_size {
                        let before = pos;
                        pos = pos.wrapping_add(ba);
                        if let Some((start, count)) = gb_read_span(before, ba, pos, limit) {
                            if start < lo || start + count > hi {
                                return None;
                            }
                        }
                        k += 1;
                    }
                } else {
                    let (_m, n) = grouped_params(ba);
                    let before = pos;
                    pos = pos.wrapping_add(n);
                    if let Some((start, count)) = gb_read_span(before, n, pos, limit) {
                        if start < lo || start + count > hi {
                            return None;
                        }
                    }
                    if !touch(&mut max_idx) {
                        return None;
                    }
                }
            }
            dst += choff as i64;
            choff = 18 - choff;
            i += 1;
        }
    }
    Some(Plan {
        grbuf_len: (max_idx + 1).max(0) as usize,
        final_pos: pos,
    })
}

// ---------------------------------------------------------------------------
// The differential call
// ---------------------------------------------------------------------------

/// Sentinel written into every `grbuf` slot before the call so that *unwritten*
/// slots are also compared (any stray write shows up as a mismatch).
const SENTINEL: u32 = 0xDEAD_BEEF;

pub struct Outcome {
    pub ret: i32,
    pub pos: i32,
    pub limit: i32,
    pub grbuf: Vec<u32>,
    pub sci_bytes: Vec<u8>,
}

fn sci_bytes(sci: &Sci) -> Vec<u8> {
    let p = sci as *const Sci as *const u8;
    unsafe { std::slice::from_raw_parts(p, std::mem::size_of::<Sci>()) }.to_vec()
}

fn run_one(
    f: DequantFn,
    buf: &[u8],
    origin: usize,
    sci_in: &Sci,
    group_size: i32,
    pos0: i32,
    limit: i32,
    grbuf_len: usize,
) -> Outcome {
    // 64 slots of guard padding on both ends of the *logical* region.
    let mut grbuf: Vec<u32> = vec![SENTINEL; grbuf_len + 64];
    let mut sci = *sci_in;
    let mut bs = BsT {
        buf: unsafe { buf.as_ptr().add(origin) },
        pos: pos0,
        limit,
    };
    let ret = unsafe {
        f(
            grbuf.as_mut_ptr() as *mut f32,
            &mut bs as *mut BsT,
            &mut sci as *mut Sci,
            group_size,
        )
    };
    Outcome {
        ret,
        pos: bs.pos,
        limit: bs.limit,
        grbuf: std::mem::take(&mut grbuf),
        sci_bytes: sci_bytes(&sci),
    }
}

/// Runs one configuration against both `.so`s and asserts byte-identical
/// results. Returns `true` if the case ran, `false` if it was skipped as
/// non-comparable.
#[must_use]
pub fn diff_case(
    label: &str,
    buf: &[u8],
    sci: &Sci,
    group_size: i32,
    pos0: i32,
    limit: i32,
) -> bool {
    diff_case_origin(label, buf, 0, sci, group_size, pos0, limit)
}

/// Like [`diff_case`], but `bs->buf` is set to `buf.as_ptr() + origin`, so a
/// negative `bs->pos` can produce a *real* read that still lands inside the
/// backing allocation. Without this, `bs->pos >> 3` being an arithmetic rather
/// than a logical shift is unobservable.
#[must_use]
pub fn diff_case_origin(
    label: &str,
    buf: &[u8],
    origin: usize,
    sci: &Sci,
    group_size: i32,
    pos0: i32,
    limit: i32,
) -> bool {
    let lo = -(origin as i64);
    let hi = buf.len() as i64 - origin as i64;
    let Some(p) = plan_range(sci, group_size, pos0, limit, lo, hi) else {
        return false;
    };
    let c = run_one(
        c_dequantize(),
        buf,
        origin,
        sci,
        group_size,
        pos0,
        limit,
        p.grbuf_len,
    );
    let r = run_one(
        rust_dequantize(),
        buf,
        origin,
        sci,
        group_size,
        pos0,
        limit,
        p.grbuf_len,
    );

    let ctx = || {
        format!(
            "{label}: group_size={group_size} pos0={pos0} limit={limit} \
             total_bands={} buf_len={} grbuf_len={} \
             bitalloc[..{}]={:?}",
            sci.total_bands,
            buf.len(),
            p.grbuf_len,
            (2 * sci.total_bands as usize).min(64),
            &sci.bitalloc[..(2 * sci.total_bands as usize).min(64)],
        )
    };

    assert_eq!(c.ret, r.ret, "return value mismatch — {}", ctx());
    assert_eq!(c.pos, r.pos, "bs->pos mismatch — {}", ctx());
    assert_eq!(c.limit, r.limit, "bs->limit mismatch — {}", ctx());
    assert_eq!(
        c.pos,
        p.final_pos,
        "oracle disagrees with C on final bs->pos — {}",
        ctx()
    );
    assert_eq!(
        c.sci_bytes.len(),
        r.sci_bytes.len(),
        "L12_scale_info size mismatch"
    );
    if c.sci_bytes != r.sci_bytes {
        let at = c
            .sci_bytes
            .iter()
            .zip(&r.sci_bytes)
            .position(|(a, b)| a != b)
            .unwrap();
        panic!(
            "L12_scale_info differs at byte {at}: C={:#04x} Rust={:#04x} — {}",
            c.sci_bytes[at],
            r.sci_bytes[at],
            ctx()
        );
    }
    if c.grbuf != r.grbuf {
        let at = c
            .grbuf
            .iter()
            .zip(&r.grbuf)
            .position(|(a, b)| a != b)
            .unwrap();
        panic!(
            "grbuf differs at index {at}: C={:#010x} ({}) Rust={:#010x} ({}) — {}",
            c.grbuf[at],
            f32::from_bits(c.grbuf[at]),
            r.grbuf[at],
            f32::from_bits(r.grbuf[at]),
            ctx()
        );
    }
    true
}

/// Like [`diff_case`] but fails if the case was skipped — use it for the rows
/// that must definitely execute.
pub fn diff_case_origin_must_run(
    label: &str,
    buf: &[u8],
    origin: usize,
    sci: &Sci,
    group_size: i32,
    pos0: i32,
    limit: i32,
) {
    assert!(
        diff_case_origin(label, buf, origin, sci, group_size, pos0, limit),
        "{label}: case was skipped by the safety oracle but was expected to run \
         (origin={origin} group_size={group_size} pos0={pos0} limit={limit} total_bands={})",
        sci.total_bands
    );
}

pub fn diff_case_must_run(
    label: &str,
    buf: &[u8],
    sci: &Sci,
    group_size: i32,
    pos0: i32,
    limit: i32,
) {
    assert!(
        diff_case(label, buf, sci, group_size, pos0, limit),
        "{label}: case was skipped by the safety oracle but was expected to run \
         (group_size={group_size} pos0={pos0} limit={limit} total_bands={})",
        sci.total_bands
    );
}

pub const BUF_LEN: usize = 1 << 16;

pub fn random_buf(rng: &mut Rng) -> Vec<u8> {
    let mut b = vec![0u8; BUF_LEN];
    rng.fill(&mut b);
    b
}

/// `bs->limit` that can never be reached, so `get_bits` never bails out early
/// (as long as the reads stay inside `buf`, which the oracle verifies).
pub fn ample_limit(buf_len: usize) -> i32 {
    (buf_len * 8) as i32
}

/// A `limit` that makes every single `get_bits` call bail out with `0` without
/// ever dereferencing `buf` — needed for `ba` values whose field width is so
/// large that a real read would run off any buffer.
pub const NO_READ_LIMIT: i32 = i32::MIN;

/// True when every visited band's `ba` is small enough that reading it from a
/// `BUF_LEN` buffer is plausible.
pub fn all_ba_readable(sci: &Sci) -> bool {
    let n = (2 * sci.total_bands as i32).min(128);
    (0..n).all(|i| {
        let ba = sci.alloc_byte(i) as i32;
        ba <= 24 || {
            let (_m, nn) = grouped_params(ba);
            nn <= 32_768
        }
    })
}

// ---------------------------------------------------------------------------
// Padded-`L12_scale_info` differential calls
//
// `dequantize_granule` indexes `sci->bitalloc[i]` for `i < 2*total_bands`, i.e.
// up to `i == 509` when `total_bands == 255`.  For `i >= 128` that runs off the
// end of a bare `L12_scale_info`.  To make those reads *comparable* rather than
// indeterminate, the struct is placed at the start of a larger allocation whose
// trailing bytes the test fills with a known pattern; both libraries then read
// exactly the same memory, and the whole 0..=255 range of `total_bands` becomes
// verifiable.
// ---------------------------------------------------------------------------

pub const SCI_SIZE: usize = 900;
/// Byte offset of `bitalloc` inside `L12_scale_info` on x86-64 LP64.
pub const BITALLOC_OFF: usize = 770;

/// A `L12_scale_info` followed by `tail_len` bytes of test-controlled padding.
pub struct PaddedSci {
    /// 4-aligned backing store; `blob[0..SCI_SIZE]` is the struct itself.
    words: Vec<u32>,
    len: usize,
}

impl PaddedSci {
    pub fn new(sci: &Sci, tail_len: usize, rng: &mut Rng) -> Self {
        let len = SCI_SIZE + tail_len;
        let words = vec![0u32; (len + 3) / 4];
        let mut me = PaddedSci { words, len };
        // struct bytes
        let src = unsafe {
            std::slice::from_raw_parts(sci as *const Sci as *const u8, SCI_SIZE)
        };
        me.bytes_mut()[..SCI_SIZE].copy_from_slice(src);
        // known tail pattern
        let tail: &mut [u8] = &mut me.bytes_mut()[SCI_SIZE..];
        rng.fill(tail);
        me
    }

    fn bytes_mut(&mut self) -> &mut [u8] {
        let len = self.len;
        unsafe { std::slice::from_raw_parts_mut(self.words.as_mut_ptr() as *mut u8, len) }
    }
    pub fn bytes(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.words.as_ptr() as *const u8, self.len) }
    }
    /// The `bitalloc`-onward region the C code may walk through.
    pub fn alloc_region(&self) -> &[u8] {
        &self.bytes()[BITALLOC_OFF..]
    }
    pub fn total_bands(&self) -> u8 {
        self.bytes()[768]
    }
    pub fn set_total_bands(&mut self, tb: u8) {
        self.bytes_mut()[768] = tb;
    }
    pub fn set_alloc_byte(&mut self, i: usize, v: u8) {
        let off = BITALLOC_OFF + i;
        self.bytes_mut()[off] = v;
    }
    /// A 4-aligned copy of the whole padded blob, for direct raw calls.
    pub fn clone_bytes(&self) -> Vec<u32> {
        self.words.clone()
    }
    fn clone_words(&self) -> (Vec<u32>, usize) {
        (self.words.clone(), self.len)
    }
}

/// Differential call with the struct embedded in a padded allocation.
/// Returns `false` when the oracle deems the case non-comparable.
#[must_use]
pub fn diff_case_padded(
    label: &str,
    buf: &[u8],
    origin: usize,
    padded: &PaddedSci,
    group_size: i32,
    pos0: i32,
    limit: i32,
) -> bool {
    let lo = -(origin as i64);
    let hi = buf.len() as i64 - origin as i64;
    let Some(p) = plan_core(
        padded.alloc_region(),
        padded.total_bands(),
        group_size,
        pos0,
        limit,
        lo,
        hi,
    ) else {
        return false;
    };

    let run = |f: DequantFn| -> (i32, i32, i32, Vec<u32>, Vec<u8>) {
        let (mut words, len) = padded.clone_words();
        let mut grbuf: Vec<u32> = vec![SENTINEL; p.grbuf_len + 64];
        let mut bs = BsT {
            buf: unsafe { buf.as_ptr().add(origin) },
            pos: pos0,
            limit,
        };
        let ret = unsafe {
            f(
                grbuf.as_mut_ptr() as *mut f32,
                &mut bs as *mut BsT,
                words.as_mut_ptr() as *mut Sci,
                group_size,
            )
        };
        let after =
            unsafe { std::slice::from_raw_parts(words.as_ptr() as *const u8, len) }.to_vec();
        (ret, bs.pos, bs.limit, grbuf, after)
    };

    let (cr, cp, cl, cg, cs) = run(c_dequantize());
    let (rr, rp, rl, rg, rs) = run(rust_dequantize());

    let ctx = || {
        format!(
            "{label}: total_bands={} group_size={group_size} pos0={pos0} limit={limit} \
             origin={origin} grbuf_len={}",
            padded.total_bands(),
            p.grbuf_len
        )
    };
    assert_eq!(cr, rr, "return value mismatch — {}", ctx());
    assert_eq!(cp, rp, "bs->pos mismatch — {}", ctx());
    assert_eq!(cl, rl, "bs->limit mismatch — {}", ctx());
    assert_eq!(cp, p.final_pos, "oracle disagrees with C on bs->pos — {}", ctx());
    if cs != rs {
        let at = cs.iter().zip(&rs).position(|(a, b)| a != b).unwrap();
        panic!(
            "padded sci differs at byte {at}: C={:#04x} Rust={:#04x} — {}",
            cs[at], rs[at], ctx()
        );
    }
    if cg != rg {
        let at = cg.iter().zip(&rg).position(|(a, b)| a != b).unwrap();
        panic!(
            "grbuf differs at index {at}: C={:#010x} ({}) Rust={:#010x} ({}) — {}",
            cg[at],
            f32::from_bits(cg[at]),
            rg[at],
            f32::from_bits(rg[at]),
            ctx()
        );
    }
    true
}

pub fn diff_case_padded_must_run(
    label: &str,
    buf: &[u8],
    origin: usize,
    padded: &PaddedSci,
    group_size: i32,
    pos0: i32,
    limit: i32,
) {
    assert!(
        diff_case_padded(label, buf, origin, padded, group_size, pos0, limit),
        "{label}: case skipped by the safety oracle but expected to run \
         (total_bands={} group_size={group_size} pos0={pos0} limit={limit})",
        padded.total_bands()
    );
}
