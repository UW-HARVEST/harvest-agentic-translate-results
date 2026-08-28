//! Differential-test harness.
//!
//! BOTH the C shared library and the Rust shared library are loaded with
//! `libloading` and driven exclusively through their exported
//! `dequantize_granule` symbol.  No Rust function is ever called directly, so
//! the `#[no_mangle] extern "C"` wrapper is part of what is under test.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// C ABI mirror of `c_src/include/lib.h`
// ---------------------------------------------------------------------------

/// `typedef struct { const uint8_t *buf; int pos, limit; } bs_t;`
#[repr(C)]
#[derive(Clone, Copy)]
pub struct BsT {
    pub buf: *const u8,
    pub pos: c_int,
    pub limit: c_int,
}

/// `typedef struct { float scf[3*64]; uint8_t total_bands, stereo_bands,
///                   bitalloc[64], scfcod[64]; } L12_scale_info;`
///
/// Only used to *check* the byte offsets the harness hard-codes below.
#[repr(C)]
pub struct L12ScaleInfo {
    pub scf: [f32; 3 * 64],
    pub total_bands: u8,
    pub stereo_bands: u8,
    pub bitalloc: [u8; 64],
    pub scfcod: [u8; 64],
}

pub const OFF_SCF: usize = 0;
pub const OFF_TOTAL_BANDS: usize = 768;
pub const OFF_STEREO_BANDS: usize = 769;
pub const OFF_BITALLOC: usize = 770;
pub const OFF_SCFCOD: usize = 834;
pub const SIZEOF_SCI: usize = 900;

/// The C code indexes `sci->bitalloc[i]` for `i < 2 * total_bands`, i.e. up to
/// `i == 509` for `total_bands == 255`.  That is byte 770+509 = 1279 relative
/// to the struct base — far past the 900-byte struct.  The harness therefore
/// hands both libraries a pointer into a 2 KiB region whose *every* byte is
/// initialised identically, so the out-of-bounds reads are reproducible.
pub const SCI_REGION: usize = 2048;

/// Bit-stream backing store handed to `bs->buf`.
pub const BUF_LEN: usize = 4096;

/// Largest `bs->limit` the harness ever uses when reads are allowed.
///
/// `get_bits` only dereferences when `pos + n <= limit`; the highest byte it
/// can then touch is `(limit + 7)/8 - 1`.  With `limit == 8*(BUF_LEN-2)` that
/// is byte 4093 of a 4096-byte buffer, so every allowed read is in bounds.
pub const MAX_LIMIT: i32 = 8 * (BUF_LEN as i32 - 2); // 32752

/// `grbuf` size.  `dst` walks `+576, -558, +576, …` (net +18 per band pair)
/// starting at `grbuf + group_size*3`; for `total_bands == 255` and
/// `group_size == 32` the last written slot is index 5275.
pub const GRBUF_LEN: usize = 8192;

/// Upper bound on `|group_size|` used by the *randomised* rows.  Kept small so
/// the fuzz rows stay fast; the dedicated large-stride row uses `GROUP_CLAMP`.
pub const MAX_GROUP: i32 = 32;

/// Hard cap `sanitize` enforces on `|group_size|`.
///
/// 576 is the real MPEG granule size and the largest stride that still keeps
/// every write inside `GRBUF_LEN`: for `total_bands == 255` the last written
/// slot is `3*576 + 5148 + 575 == 7451 < 8192`.
pub const GROUP_CLAMP: i32 = 576;

pub type DequantFn =
    unsafe extern "C" fn(*mut f32, *mut BsT, *mut u8, c_int) -> c_int;

// ---------------------------------------------------------------------------
// Loading the two shared objects
// ---------------------------------------------------------------------------

pub struct Lib {
    pub name: &'static str,
    pub path: PathBuf,
    _lib: Library,
    pub dequantize_granule: DequantFn,
}

impl Lib {
    fn open(name: &'static str, path: PathBuf) -> Lib {
        let lib = unsafe { Library::new(&path) }
            .unwrap_or_else(|e| panic!("could not dlopen {} ({}): {e}", name, path.display()));
        let f = unsafe {
            let sym: Symbol<DequantFn> = lib
                .get(b"dequantize_granule\0")
                .unwrap_or_else(|e| panic!("{name}: missing symbol dequantize_granule: {e}"));
            *sym
        };
        Lib { name, path, _lib: lib, dequantize_granule: f }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `c_src/build/lib<dirname>.so` — the basename depends on the checkout
/// directory (see `CMakeLists.txt`), so it is discovered by scanning.
pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    let build = manifest_dir().join("../c_src/build");
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            let name = p.file_name().unwrap_or_default().to_string_lossy().to_string();
            if name.starts_with("lib") && name.ends_with(".so") {
                found.push(p);
            }
        }
    }
    found.sort();
    assert_eq!(
        found.len(),
        1,
        "expected exactly one lib*.so in {} (found {found:?}); build the C library first:\n  \
         cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        build.display()
    );
    found.pop().unwrap()
}

/// `target/{debug,release}/libdequantize_granule_lib.so`, resolved relative to
/// the running test binary so `cargo test` and `cargo test --release` both work.
pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    const SO: &str = "libdequantize_granule_lib.so";
    let exe = std::env::current_exe().expect("current_exe");
    let mut dir: Option<&Path> = exe.parent();
    while let Some(d) = dir {
        let cand = d.join(SO);
        if cand.exists() {
            return cand;
        }
        dir = d.parent();
    }
    panic!("could not locate {SO} near {}", exe.display());
}

pub fn c_lib() -> &'static Lib {
    static L: OnceLock<Lib> = OnceLock::new();
    L.get_or_init(|| Lib::open("C", c_so_path()))
}

pub fn rust_lib() -> &'static Lib {
    static L: OnceLock<Lib> = OnceLock::new();
    L.get_or_init(|| Lib::open("Rust", rust_so_path()))
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seed => reproducible test corpus
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
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
    pub fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }
    /// Uniform in `0..n` (n > 0).
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
    /// Uniform in `lo..=hi`.
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
    pub fn fill(&mut self, out: &mut [u8]) {
        for b in out.iter_mut() {
            *b = self.next_u8();
        }
    }
    pub fn pick<T: Copy>(&mut self, xs: &[T]) -> T {
        xs[(self.next_u64() % xs.len() as u64) as usize]
    }
}

// ---------------------------------------------------------------------------
// The `n` that `dequantize_granule` passes to `get_bits` for a given `bitalloc`
// (a faithful mirror of the C expression, used only by the harness to keep
// test cases memory-safe and to build exact-boundary `limit` values)
// ---------------------------------------------------------------------------

pub fn n_for_ba(ba: i32) -> i32 {
    assert!(ba != 0);
    if ba < 17 {
        ba
    } else {
        // `2 << (ba - 17)`: 32-bit shift, count masked to 5 bits on x86-64.
        let m = (2i32.wrapping_shl((ba - 17) as u32) as u32).wrapping_add(1);
        m.wrapping_add(2).wrapping_sub(m >> 3) as i32
    }
}

/// Every `bitalloc` value whose `get_bits` width is small enough that a read is
/// actually performed with `limit <= MAX_LIMIT` (the rest always underrun).
pub fn narrow_bas() -> Vec<u8> {
    (1u32..=255).filter(|&b| n_for_ba(b as i32) <= MAX_LIMIT).map(|b| b as u8).collect()
}

/// `bitalloc` values whose width is so large that they always trip the
/// underrun guard for any buffer-safe `limit`.
pub fn wide_bas() -> Vec<u8> {
    (1u32..=255).filter(|&b| n_for_ba(b as i32) > MAX_LIMIT).map(|b| b as u8).collect()
}

// ---------------------------------------------------------------------------
// A test case + its observable outcome
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct Case {
    /// `BUF_LEN` bytes handed to `bs->buf`.
    pub bits: Vec<u8>,
    pub pos: i32,
    pub limit: i32,
    /// `SCI_REGION` bytes; the `L12_scale_info` occupies the first 900.
    pub sci: Vec<u8>,
    pub group_size: i32,
    /// Seed for the `grbuf` pre-fill pattern (untouched slots must survive).
    pub grbuf_seed: u32,
    pub grbuf_null: bool,
    pub bs_null: bool,
}

impl Case {
    pub fn new(rng: &mut Rng) -> Case {
        let mut bits = vec![0u8; BUF_LEN];
        rng.fill(&mut bits);
        let mut sci = vec![0u8; SCI_REGION];
        rng.fill(&mut sci);
        sci[OFF_TOTAL_BANDS] = 0;
        Case {
            bits,
            pos: 0,
            limit: MAX_LIMIT,
            sci,
            group_size: 4,
            grbuf_seed: rng.next_u32(),
            grbuf_null: false,
            bs_null: false,
        }
    }

    pub fn total_bands(&self) -> u8 {
        self.sci[OFF_TOTAL_BANDS]
    }
    pub fn set_total_bands(&mut self, t: u8) {
        self.sci[OFF_TOTAL_BANDS] = t;
    }
    /// Writes through the *effective* bit-allocation window, which extends past
    /// the declared 64-byte array exactly as the C indexing does.
    pub fn set_ba(&mut self, i: usize, v: u8) {
        self.sci[OFF_BITALLOC + i] = v;
    }
    pub fn ba(&self, i: usize) -> u8 {
        self.sci[OFF_BITALLOC + i]
    }
    pub fn set_bitalloc_all(&mut self, v: u8) {
        let n = 2 * self.total_bands() as usize;
        for i in 0..n {
            self.set_ba(i, v);
        }
    }

    /// Total number of bits `dequantize_granule` will try to consume — used to
    /// keep `bs->pos` from overflowing (which in C means reading unmapped
    /// memory, i.e. a crash in *both* implementations rather than a
    /// comparable result).
    fn bits_consumed_bound(&self) -> i64 {
        let nb = 2 * self.total_bands() as usize;
        let gs = if self.group_size > 0 { self.group_size as i64 } else { 1 };
        let mut total: i64 = 0;
        for i in 0..nb {
            let ba = self.ba(i) as i32;
            if ba == 0 {
                continue;
            }
            let n = n_for_ba(ba) as i64;
            total += n * if ba < 17 { gs } else { 1 };
        }
        total * 4 // the `j` loop
    }

    /// Force the case into the region where the C code is memory-safe, so that
    /// any divergence we observe is a translation bug and not shared UB.
    ///
    /// * `pos >= 0` and `limit <= MAX_LIMIT` keeps every *allowed* read inside
    ///   `bits`.
    /// * a bounded total bit consumption keeps `pos` from wrapping to a
    ///   negative value (which would make the guard pass and read far before
    ///   the buffer).
    /// * outside that region we pin `limit = INT_MIN`, which makes the guard
    ///   `pos + n > limit` fire for every read.  The call then performs **no**
    ///   dereference at all, yet `bs->pos` still advances by exactly `n`, so
    ///   the width computation is still fully observable.
    pub fn sanitize(&mut self) {
        self.sanitize_n(1);
    }

    /// As [`Case::sanitize`], for a case that will be driven `ncalls` times over
    /// the same `bs_t` (so the bit consumption bound scales accordingly).
    pub fn sanitize_n(&mut self, ncalls: i64) {
        self.group_size = self.group_size.clamp(-GROUP_CLAMP, GROUP_CLAMP);
        let bound = self.bits_consumed_bound().saturating_mul(ncalls);
        if self.pos < 0 || self.pos > (1 << 29) || bound > (1 << 29) {
            self.pos = self.pos.clamp(-(1 << 29), 1 << 29);
            self.limit = i32::MIN;
        } else if self.limit > MAX_LIMIT {
            self.limit = MAX_LIMIT;
        }
    }
}

#[derive(PartialEq, Eq)]
pub struct Outcome {
    pub ret: i32,
    pub pos: i32,
    pub limit: i32,
    /// `grbuf` compared by raw bit pattern, so `-0.0`/NaN payloads are exact.
    pub grbuf: Vec<u32>,
    /// The libraries must not write to these.
    pub bits: Vec<u8>,
    pub sci: Vec<u8>,
}

fn grbuf_pattern(seed: u32) -> Vec<f32> {
    (0..GRBUF_LEN)
        .map(|i| f32::from_bits(seed ^ (i as u32).wrapping_mul(0x0100_1001)))
        .collect()
}

pub fn run(lib: &Lib, case: &Case) -> Outcome {
    // 8-byte aligned scratch for the `L12_scale_info` region.
    let mut sci_words = vec![0u64; SCI_REGION / 8];
    let sci_ptr = sci_words.as_mut_ptr() as *mut u8;
    unsafe { std::ptr::copy_nonoverlapping(case.sci.as_ptr(), sci_ptr, SCI_REGION) };

    let mut bits = case.bits.clone();
    let mut grbuf = grbuf_pattern(case.grbuf_seed);

    let mut bs = BsT { buf: bits.as_ptr(), pos: case.pos, limit: case.limit };

    let grbuf_arg = if case.grbuf_null { std::ptr::null_mut() } else { grbuf.as_mut_ptr() };
    let bs_arg = if case.bs_null { std::ptr::null_mut() } else { &mut bs as *mut BsT };
    // `sci` is dereferenced unconditionally by the C code (`sci->total_bands`),
    // so a null `sci` is shared UB (SIGSEGV in both) and never passed.
    let ret = unsafe { (lib.dequantize_granule)(grbuf_arg, bs_arg, sci_ptr, case.group_size) };

    if !case.bs_null {
        assert_eq!(bs.buf, bits.as_ptr(), "{}: bs->buf must not be modified", lib.name);
    }
    let out_sci = unsafe { std::slice::from_raw_parts(sci_ptr, SCI_REGION).to_vec() };
    let out_grbuf: Vec<u32> = grbuf.iter().map(|f| f.to_bits()).collect();
    let (pos, limit) = if case.bs_null { (case.pos, case.limit) } else { (bs.pos, bs.limit) };

    // keep the backing stores alive until here
    std::hint::black_box(&mut bits);
    std::hint::black_box(&mut grbuf);
    std::hint::black_box(&mut sci_words);

    Outcome { ret, pos, limit, grbuf: out_grbuf, bits, sci: out_sci }
}

/// Runs `case` through BOTH `.so`s and asserts every observable is identical.
pub fn assert_same(case: &Case, ctx: &str) {
    let c = run(c_lib(), case);
    let r = run(rust_lib(), case);

    let hdr = || {
        format!(
            "{ctx}\n  total_bands={} group_size={} pos={} limit={} \
             bitalloc[0..8]={:?} grbuf_null={} bs_null={}",
            case.total_bands(),
            case.group_size,
            case.pos,
            case.limit,
            &case.sci[OFF_BITALLOC..OFF_BITALLOC + 8],
            case.grbuf_null,
            case.bs_null,
        )
    };

    assert_eq!(c.ret, r.ret, "return value differs\n{}", hdr());
    assert_eq!(c.pos, r.pos, "bs->pos differs\n{}", hdr());
    assert_eq!(c.limit, r.limit, "bs->limit differs\n{}", hdr());
    assert!(c.bits == r.bits, "bit-stream buffer was modified differently\n{}", hdr());
    assert!(c.sci == r.sci, "L12_scale_info region was modified differently\n{}", hdr());
    // the C code only ever *reads* these two, so neither library may touch them
    assert!(c.bits == case.bits, "C modified the bit-stream buffer\n{}", hdr());
    assert!(r.bits == case.bits, "Rust modified the bit-stream buffer\n{}", hdr());
    assert!(c.sci == case.sci, "C modified the L12_scale_info region\n{}", hdr());
    assert!(r.sci == case.sci, "Rust modified the L12_scale_info region\n{}", hdr());

    if c.grbuf != r.grbuf {
        let mut diffs = Vec::new();
        for i in 0..GRBUF_LEN {
            if c.grbuf[i] != r.grbuf[i] {
                diffs.push(format!(
                    "    grbuf[{i}]: C=0x{:08x} ({}) Rust=0x{:08x} ({})",
                    c.grbuf[i],
                    f32::from_bits(c.grbuf[i]),
                    r.grbuf[i],
                    f32::from_bits(r.grbuf[i]),
                ));
                if diffs.len() == 12 {
                    break;
                }
            }
        }
        panic!("grbuf differs in {} slot(s)\n{}\n{}",
            (0..GRBUF_LEN).filter(|&i| c.grbuf[i] != r.grbuf[i]).count(),
            hdr(), diffs.join("\n"));
    }
}

/// Convenience: run a closure that builds a case, `sanitize` it and compare.
pub fn check(case: &mut Case, ctx: &str) {
    case.sanitize();
    assert_same(case, ctx);
}

// ---------------------------------------------------------------------------
// Sequential driving: several `dequantize_granule` calls that share one `bs_t`
// and one `grbuf` (real consumers decode granule after granule from the same
// bit reader, so `bs->pos` carries over and later calls underrun).
// ---------------------------------------------------------------------------

pub fn run_seq(lib: &Lib, case: &Case, ncalls: usize) -> (Vec<i32>, Outcome) {
    let mut sci_words = vec![0u64; SCI_REGION / 8];
    let sci_ptr = sci_words.as_mut_ptr() as *mut u8;
    unsafe { std::ptr::copy_nonoverlapping(case.sci.as_ptr(), sci_ptr, SCI_REGION) };

    let mut bits = case.bits.clone();
    let mut grbuf = grbuf_pattern(case.grbuf_seed);
    let mut bs = BsT { buf: bits.as_ptr(), pos: case.pos, limit: case.limit };

    let mut rets = Vec::with_capacity(ncalls);
    for _ in 0..ncalls {
        rets.push(unsafe {
            (lib.dequantize_granule)(
                grbuf.as_mut_ptr(),
                &mut bs as *mut BsT,
                sci_ptr,
                case.group_size,
            )
        });
    }

    let out_sci = unsafe { std::slice::from_raw_parts(sci_ptr, SCI_REGION).to_vec() };
    let out_grbuf: Vec<u32> = grbuf.iter().map(|f| f.to_bits()).collect();
    let out = Outcome {
        ret: *rets.last().unwrap(),
        pos: bs.pos,
        limit: bs.limit,
        grbuf: out_grbuf,
        bits: bits.clone(),
        sci: out_sci,
    };
    std::hint::black_box(&mut bits);
    std::hint::black_box(&mut grbuf);
    std::hint::black_box(&mut sci_words);
    (rets, out)
}

pub fn assert_same_seq(case: &Case, ncalls: usize, ctx: &str) {
    let (cr, co) = run_seq(c_lib(), case, ncalls);
    let (rr, ro) = run_seq(rust_lib(), case, ncalls);
    assert_eq!(cr, rr, "per-call return values differ ({ctx})");
    assert_eq!(co.pos, ro.pos, "final bs->pos differs ({ctx})");
    assert_eq!(co.limit, ro.limit, "final bs->limit differs ({ctx})");
    assert!(co.bits == ro.bits, "bit-stream modified differently ({ctx})");
    assert!(co.sci == ro.sci, "sci region modified differently ({ctx})");
    if co.grbuf != ro.grbuf {
        let i = (0..GRBUF_LEN).find(|&i| co.grbuf[i] != ro.grbuf[i]).unwrap();
        panic!(
            "grbuf differs after {ncalls} chained calls ({ctx}): first at [{i}] \
             C=0x{:08x} Rust=0x{:08x}",
            co.grbuf[i], ro.grbuf[i]
        );
    }
}

/// Indices of `grbuf` slots a call actually wrote (relative to the pre-fill
/// pattern).  Used to compare the `dst`/`choff` walk itself, not just values.
pub fn touched(out: &Outcome, seed: u32) -> Vec<usize> {
    let base = grbuf_pattern(seed);
    (0..GRBUF_LEN).filter(|&i| out.grbuf[i] != base[i].to_bits()).collect()
}
