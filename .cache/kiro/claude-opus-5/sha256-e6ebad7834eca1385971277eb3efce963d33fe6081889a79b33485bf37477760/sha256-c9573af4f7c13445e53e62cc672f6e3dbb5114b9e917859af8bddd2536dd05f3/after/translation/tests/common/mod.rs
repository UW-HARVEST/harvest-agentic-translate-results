//! Shared differential-testing harness.
//!
//! Both the C shared object and the Rust shared object are loaded with
//! `libloading` and driven exclusively through their exported
//! `dequantize_granule` symbol. The Rust crate is NEVER linked or called
//! directly, so the `#[no_mangle] extern "C"` wrapper is under test too.
//!
//! # Why some configurations force `limit == i32::MIN`
//!
//! The grouped path calls `get_bits(bs, mod + 2 - (mod >> 3))`. For
//! `k = (ba - 17) & 31 >= 25` that `n` exceeds 58 million, and for `k == 30`
//! (`ba == 47, 79, ...`) it is `0x70000003 == 1_879_048_195`. `get_bits` does
//! `bs->pos += n` *unconditionally*, so after two such calls `bs->pos`
//! overflows `int`. Once it wraps negative the `> bs->limit` guard stops
//! firing and the C dereferences `bs->buf + (pos >> 3)` hundreds of megabytes
//! out of bounds — a hard segfault in the C itself, not a translation
//! difference.
//!
//! Setting `bs->limit = INT_MIN` makes `pos > limit` true for every `pos`
//! except exactly `INT_MIN`, so `get_bits` always takes its early-out and never
//! dereferences the buffer. That lets us exercise the *entire* `ba == 0..=255`
//! domain (all 32 shift residues, the `2 << 30` signed overflow, the
//! `mod == 1` case, the unsigned `code % mod - mod / 2` wraparound) while
//! keeping the C well-behaved enough to run.
//!
//! Configurations that *do* exercise the buffer-reading path cap the reachable
//! `k` (see [`kcap_for`]) so `bs->pos` provably never overflows.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// ABI mirrors of the C types
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BsT {
    pub buf: *const u8,
    pub pos: c_int,
    pub limit: c_int,
}

/// `sizeof(L12_scale_info)` on LP64: 768 (`scf`) + 1 + 1 + 64 + 64 = 898,
/// rounded up to the 4-byte alignment of `float` => 900.
pub const SCI_SIZE: usize = 900;
pub const OFF_TOTAL_BANDS: usize = 768;
pub const OFF_STEREO_BANDS: usize = 769;
pub const OFF_BITALLOC: usize = 770;
pub const OFF_SCFCOD: usize = 834;

/// The C indexes `sci->bitalloc[i]` for `i` up to `2*255 - 1 = 509`, i.e. up to
/// byte offset `770 + 509 = 1279` — 380 bytes past the end of the struct. We
/// therefore back `sci` with a larger, fully-initialised allocation so the
/// out-of-bounds reads are deterministic and identical for both libraries.
pub const SCI_ALLOC: usize = 2048;

pub type DequantFn = unsafe extern "C" fn(*mut f32, *mut BsT, *mut u8, c_int) -> c_int;

// ---------------------------------------------------------------------------
// `n` as a function of the grouped-path shift residue k = (ba - 17) & 31
//   mod = ((2 << k) as int) + 1   (wrapping, as unsigned)
//   n   = mod + 2 - (mod >> 3)    (unsigned, then to int)
// ---------------------------------------------------------------------------

/// `mod` for shift residue `k`, reproducing the C's masked shift and signed
/// overflow exactly.
pub fn grouped_mod(k: u32) -> u32 {
    (2i32.wrapping_shl(k).wrapping_add(1)) as u32
}

/// `n` (the bit count requested from `get_bits`) for shift residue `k`.
pub fn grouped_n(k: u32) -> i64 {
    let m = grouped_mod(k);
    (m.wrapping_add(2).wrapping_sub(m >> 3) as i32) as i64
}

/// `k` for a given `bitalloc` value (only meaningful for `ba >= 17`).
pub fn k_of(ba: u8) -> u32 {
    ((ba as u32).wrapping_sub(17)) & 31
}

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

pub struct Impl {
    pub name: &'static str,
    pub dequantize_granule: DequantFn,
    _lib: Library,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn find_c_so() -> PathBuf {
    if let Some(p) = std::env::var_os("C_SO") {
        return PathBuf::from(p);
    }
    let dir = manifest_dir().join("../c_src/build");
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().map(|x| x == "so").unwrap_or(false) {
                found.push(p);
            }
        }
    }
    found.sort();
    assert!(
        !found.is_empty(),
        "no C .so found in {:?}. Build it first:\n  cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        dir
    );
    found.remove(0)
}

pub fn find_rust_so() -> PathBuf {
    // `RUST_SO=...` lets the suite be pointed at a specific build, e.g. the
    // debug `cdylib` (which has overflow checks enabled, so any non-wrapping
    // arithmetic in the translation would panic instead of wrapping).
    if let Some(p) = std::env::var_os("RUST_SO") {
        return PathBuf::from(p);
    }
    let base = manifest_dir().join("target");
    for profile in ["release", "debug"] {
        let p = base.join(profile).join("libdequantize_granule_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!("no Rust .so found under {:?}. Build it: cargo build --release", base);
}

fn load(name: &'static str, path: &Path) -> Impl {
    unsafe {
        let lib =
            Library::new(path).unwrap_or_else(|e| panic!("failed to dlopen {:?}: {e}", path));
        let sym: Symbol<DequantFn> = lib
            .get(b"dequantize_granule\0")
            .unwrap_or_else(|e| panic!("{:?} does not export dequantize_granule: {e}", path));
        let f = *sym;
        Impl { name, dequantize_granule: f, _lib: lib }
    }
}

/// Loads the C and Rust shared objects (in that order).
pub fn load_impls() -> (Impl, Impl) {
    let c = load("C", &find_c_so());
    let r = load("Rust", &find_rust_so());
    (c, r)
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seeds keep every run reproducible
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed.wrapping_mul(0x2545_F491_4F6C_DD1D) ^ 0x9E37_79B9_7F4A_7C15)
    }
    #[inline]
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    #[inline]
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    #[inline]
    pub fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }
    /// Uniform in `lo..=hi`.
    pub fn range_u8(&mut self, lo: u8, hi: u8) -> u8 {
        let span = (hi - lo) as u32 + 1;
        lo + (self.next_u32() % span) as u8
    }
    /// Fast whole-slice fill, 8 bytes per PRNG step.
    pub fn fill(&mut self, dst: &mut [u8]) {
        let mut it = dst.chunks_exact_mut(8);
        for c in &mut it {
            c.copy_from_slice(&self.next_u64().to_le_bytes());
        }
        let rem = it.into_remainder();
        if !rem.is_empty() {
            let v = self.next_u64().to_le_bytes();
            rem.copy_from_slice(&v[..rem.len()]);
        }
    }
}

// ---------------------------------------------------------------------------
// Test-case description
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BaMode {
    /// Every byte the same value (used verbatim — never k-capped).
    Const(u8),
    /// Uniform random in `lo..=hi`, k-capped when the buffer is readable.
    Range(u8, u8),
    /// ~50 % zeros, otherwise uniform in `1..=255`; k-capped.
    Sparse,
    /// Cycle a fixed list of boundary values (used verbatim — never k-capped).
    BoundaryMix,
    /// `0` for `bitalloc` indices `< n`, then `v`. Used to prove that the C
    /// really does read `sci->bitalloc[i]` past the end of the 64-byte array
    /// (and past the end of the struct) for large `total_bands`.
    ZeroBelowThenConst(u16, u8),
}

pub const BOUNDARY_BA: [u8; 13] = [0, 1, 2, 15, 16, 17, 18, 46, 47, 48, 49, 254, 255];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BufMode {
    Random,
    Zeros,
    Ones,
    /// `0x5A` in the headroom before `bs->buf`, `0xFF` from `bs->buf` onwards.
    /// Lets a test distinguish "read before the buffer" from "read at the
    /// buffer" from "never read at all".
    Split,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LimitMode {
    /// `limit = v`
    Abs(c_int),
    /// `limit = pos + v`
    RelPos(c_int),
    /// Largest limit for which every read provably stays inside the bitstream
    /// allocation (`max byte index <= limit/8 + 2`).
    Huge,
}

/// `limit = INT_MIN` — `get_bits` always takes its early-out, never reads.
pub const NO_READ: LimitMode = LimitMode::Abs(c_int::MIN);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PosMode {
    Abs(c_int),
    /// Uniform random in `0..=v`.
    RandomUpTo(c_int),
    /// `rep % 8` — sweeps all byte alignments.
    RepMod8,
}

#[derive(Clone, Copy, Debug)]
pub struct Case {
    pub id: u32,
    pub group_size: c_int,
    pub total_bands: u8,
    pub ba: BaMode,
    pub pos: PosMode,
    pub limit: LimitMode,
    pub buf: BufMode,
    pub iters: u32,
    pub null_grbuf: bool,
    pub null_bs: bool,
}

impl Case {
    pub const fn new(id: u32, group_size: c_int, total_bands: u8, ba: BaMode) -> Self {
        Case {
            id,
            group_size,
            total_bands,
            ba,
            pos: PosMode::Abs(0),
            limit: LimitMode::Huge,
            buf: BufMode::Random,
            iters: 16,
            null_grbuf: false,
            null_bs: false,
        }
    }
    pub const fn pos(mut self, p: PosMode) -> Self {
        self.pos = p;
        self
    }
    pub const fn at(mut self, p: c_int) -> Self {
        self.pos = PosMode::Abs(p);
        self
    }
    pub const fn limit(mut self, l: LimitMode) -> Self {
        self.limit = l;
        self
    }
    pub const fn buf(mut self, b: BufMode) -> Self {
        self.buf = b;
        self
    }
    pub const fn iters(mut self, n: u32) -> Self {
        self.iters = n;
        self
    }
    pub const fn null_grbuf(mut self) -> Self {
        self.null_grbuf = true;
        self
    }
    pub const fn null_bs(mut self) -> Self {
        self.null_bs = true;
        self
    }

    /// Highest `bs->pos` the fixture can start from.
    fn pos_max(&self) -> i64 {
        match self.pos {
            PosMode::Abs(p) => p.max(0) as i64,
            PosMode::RandomUpTo(v) => v.max(0) as i64,
            PosMode::RepMod8 => 7,
        }
    }

    /// Lowest `bs->pos` the fixture can start from.
    fn pos_min(&self) -> i64 {
        match self.pos {
            PosMode::Abs(p) => p.min(0) as i64,
            PosMode::RandomUpTo(_) | PosMode::RepMod8 => 0,
        }
    }

    /// Highest `bs->limit` the fixture can produce.
    fn limit_max(&self) -> i64 {
        match self.limit {
            LimitMode::Abs(v) => v as i64,
            LimitMode::RelPos(v) => self.pos_max() + v as i64,
            LimitMode::Huge => HUGE_LIMIT as i64,
        }
    }

    /// `true` when `get_bits` can never dereference `bs->buf`.
    fn never_reads(&self) -> bool {
        self.limit == NO_READ
    }
}

/// Largest grouped-path shift residue `k` that keeps `bs->pos` from overflowing
/// `int` for this case.
///
/// The worst case is every one of the `4 * 2 * total_bands` band visits taking
/// the grouped path and consuming `grouped_n(k)` bits.
///
/// `grouped_n` is *not* monotonic in `k` (`k == 31` gives `n == 3` because
/// `2 << 31` masks to zero), but [`kcap_remap`] maps residues into the whole
/// range `0..=kcap`, so the cap must be a safe *prefix*: stop at the first
/// unsafe `k` rather than taking the maximum safe one.
pub fn kcap_for(case: &Case) -> u32 {
    if case.never_reads() {
        return 31;
    }
    let calls = 8i64 * case.total_bands as i64;
    if calls == 0 {
        return 31;
    }
    let budget = i32::MAX as i64 - case.pos_max();
    let mut best = 0u32;
    for k in 0..=31u32 {
        let n = grouped_n(k);
        if n > 0 && n.saturating_mul(calls) <= budget {
            best = k;
        } else {
            break;
        }
    }
    best
}

/// Remaps a `bitalloc` value so its shift residue is `<= kcap`, while keeping
/// its magnitude (so high `ba` values are still exercised).
fn kcap_remap(ba: u8, kcap: u32) -> u8 {
    if ba < 17 {
        return ba;
    }
    let off = (ba - 17) as u32; // 0..=238
    let k = off & 31;
    if k <= kcap {
        return ba;
    }
    let base = off & !31u32; // 32 * m
    let k2 = k % (kcap + 1);
    let mut v = 17u32 + base + k2;
    while v > 255 {
        v -= 32;
    }
    v as u8
}

/// Panics if a case that reads the bitstream could overflow `bs->pos` or read
/// outside the fixture's bitstream allocation.
fn assert_case_safe(case: &Case) {
    if case.never_reads() {
        return;
    }

    // `get_bits` reads bytes `[pos>>3, pos>>3 + ceil((n+s)/8)]` and only when
    // `pos + n <= limit`, so the touched byte range is bounded below by
    // `pos_min/8` and above by `limit_max/8 + 2`.
    let lo = case.pos_min() / 8 - 2;
    assert!(
        lo >= -(BIT_PREFIX as i64) + 8,
        "case {}: bs->pos as low as {} would read {} bytes before bs->buf, \
         but only {} bytes of headroom exist",
        case.id,
        case.pos_min(),
        -lo,
        BIT_PREFIX
    );
    let hi = case.limit_max().max(0) / 8 + 2;
    assert!(
        hi <= BIT_USABLE as i64 - 8,
        "case {}: bs->limit as high as {} would read up to byte {}, \
         past the {}-byte bitstream",
        case.id,
        case.limit_max(),
        hi,
        BIT_USABLE
    );

    let kcap = kcap_for(case);
    let max_k = match case.ba {
        BaMode::Const(v) => {
            if v >= 17 {
                k_of(v)
            } else {
                0
            }
        }
        BaMode::BoundaryMix => BOUNDARY_BA
            .iter()
            .filter(|&&v| v >= 17)
            .map(|&v| k_of(v))
            .max()
            .unwrap_or(0),
        BaMode::ZeroBelowThenConst(_, v) => {
            if v >= 17 {
                k_of(v)
            } else {
                0
            }
        }
        BaMode::Range(_, _) | BaMode::Sparse => kcap,
    };
    let calls = 8i64 * case.total_bands as i64;
    let total = case.pos_max() + grouped_n(max_k).max(0) * calls;
    assert!(
        total <= i32::MAX as i64,
        "case {} would overflow bs->pos (max_k={} n={} calls={} total={}). \
         Use `.limit(NO_READ)` for this configuration.",
        case.id,
        max_k,
        grouped_n(max_k),
        calls,
        total
    );
}

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// Bytes of headroom in front of `bs->buf` so that a negative `bs->pos` reads
/// initialised memory instead of faulting.
pub const BIT_PREFIX: usize = 1 << 16;
/// Usable bitstream length starting at `bs->buf`.
pub const BIT_USABLE: usize = 1 << 20;

/// `bs->limit` for [`LimitMode::Huge`].
///
/// `get_bits` reads bytes `[pos>>3, pos>>3 + ceil((n+s)/8)]` and only after
/// checking `pos + n <= limit`, so the highest byte touched is bounded by
/// `limit/8 + 2`. Leaving 16 bytes of slack keeps every read in bounds.
pub const HUGE_LIMIT: c_int = (8 * (BIT_USABLE - 16)) as c_int;

/// Byte pattern pre-filled into `grbuf` so untouched regions are compared too.
pub const GRBUF_FILL: u8 = 0xA5;

pub struct Fixture {
    pub bits: Vec<u8>,
    /// `sci` backing allocation, 8-byte aligned via `Vec<u64>`.
    pub sci: Vec<u64>,
    pub grbuf: Vec<f32>,
}

/// Number of `f32` slots `grbuf` needs.
///
/// `dst` starts at `grbuf + group_size*j` (`j <= 3`) and then accumulates
/// `choff` values alternating `576, -558`, whose partial sums peak at
/// `18*(m/2) + 576 <= 18*255 + 576 = 5166` for `m <= 510` iterations. Writes
/// span `dst[0..group_size]`, so `4*group_size + 5166` suffices; 6000 of slack.
pub fn grbuf_len(group_size: c_int) -> usize {
    let g = group_size.max(0) as usize;
    4 * g + 6000
}

impl Fixture {
    pub fn new(group_size: c_int) -> Self {
        Fixture {
            bits: vec![0u8; BIT_PREFIX + BIT_USABLE],
            sci: vec![0u64; SCI_ALLOC / 8],
            grbuf: vec![0.0f32; grbuf_len(group_size)],
        }
    }

    /// Like [`Fixture::new`] but skips the (possibly enormous) `grbuf`
    /// allocation for cases that pass `grbuf == NULL`. That is what makes
    /// `group_size == 0x7FFF_FFFF` testable at all.
    pub fn new_for(case: &Case) -> Self {
        if case.null_grbuf {
            Fixture {
                bits: vec![0u8; BIT_PREFIX + BIT_USABLE],
                sci: vec![0u64; SCI_ALLOC / 8],
                grbuf: vec![0.0f32; 1],
            }
        } else {
            Fixture::new(case.group_size)
        }
    }

    pub fn sci_bytes(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.sci.as_mut_ptr() as *mut u8, SCI_ALLOC) }
    }

    pub fn sci_bytes_ref(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.sci.as_ptr() as *const u8, SCI_ALLOC) }
    }

    pub fn grbuf_bytes(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(self.grbuf.as_ptr() as *const u8, self.grbuf.len() * 4)
        }
    }

    /// (Re)initialises every byte of every buffer for one repetition.
    /// Deterministic in `(case.id, rep)` so both implementations see the exact
    /// same input bytes.
    pub fn prepare(&mut self, case: &Case, rep: u32) {
        let seed = ((case.id as u64) << 32) | rep as u64;

        // --- bitstream ---
        match case.buf {
            BufMode::Random => Rng::new(0xC0FF_EE00 ^ seed).fill(&mut self.bits),
            BufMode::Zeros => self.bits.iter_mut().for_each(|b| *b = 0x00),
            BufMode::Ones => self.bits.iter_mut().for_each(|b| *b = 0xFF),
            BufMode::Split => {
                self.bits[..BIT_PREFIX].iter_mut().for_each(|b| *b = 0x5A);
                self.bits[BIT_PREFIX..].iter_mut().for_each(|b| *b = 0xFF);
            }
        }

        // --- sci ---
        let ba_mode = case.ba;
        let tb = case.total_bands;
        let kcap = kcap_for(case);
        {
            let mut r2 = Rng::new(0xBEEF_0000 ^ seed);
            let sci = self.sci_bytes();
            // Randomise the whole allocation first: this covers `scf`,
            // `stereo_bands`, the struct's trailing padding and the bytes past
            // the end of the struct that the OOB `bitalloc[i]` read touches.
            r2.fill(sci);
            sci[OFF_TOTAL_BANDS] = tb;
            // `bitalloc` *and* everything the OOB read can reach gets the value
            // distribution under test, so `i >= 64` exercises real code paths.
            for idx in 0..(SCI_ALLOC - OFF_BITALLOC) {
                let v = match ba_mode {
                    BaMode::Const(v) => v,
                    BaMode::Range(lo, hi) => kcap_remap(r2.range_u8(lo, hi), kcap),
                    BaMode::Sparse => {
                        if r2.next_u32() & 1 == 0 {
                            0
                        } else {
                            kcap_remap(r2.range_u8(1, 255), kcap)
                        }
                    }
                    BaMode::BoundaryMix => BOUNDARY_BA[idx % BOUNDARY_BA.len()],
                    BaMode::ZeroBelowThenConst(n, v) => {
                        if idx < n as usize {
                            0
                        } else {
                            v
                        }
                    }
                };
                sci[OFF_BITALLOC + idx] = v;
            }
        }

        // --- grbuf ---
        let n = self.grbuf.len() * 4;
        let gb =
            unsafe { std::slice::from_raw_parts_mut(self.grbuf.as_mut_ptr() as *mut u8, n) };
        gb.iter_mut().for_each(|b| *b = GRBUF_FILL);
    }

    pub fn bs_for(&self, case: &Case, rep: u32) -> BsT {
        let mut rng = Rng::new(0xDEAD_0000 ^ ((case.id as u64) << 32) ^ rep as u64);
        let pos = match case.pos {
            PosMode::Abs(p) => p,
            PosMode::RandomUpTo(v) => (rng.next_u32() % (v as u32 + 1)) as c_int,
            PosMode::RepMod8 => (rep % 8) as c_int,
        };
        let limit = match case.limit {
            LimitMode::Abs(v) => v,
            LimitMode::RelPos(v) => pos.wrapping_add(v),
            LimitMode::Huge => HUGE_LIMIT,
        };
        BsT { buf: unsafe { self.bits.as_ptr().add(BIT_PREFIX) }, pos, limit }
    }
}

// ---------------------------------------------------------------------------
// Differential runner
// ---------------------------------------------------------------------------

/// Everything observable after one call.
#[derive(Clone)]
pub struct Outcome {
    pub ret: c_int,
    pub grbuf: Vec<u8>,
    pub pos: c_int,
    pub limit: c_int,
    pub sci: Vec<u8>,
}

impl Outcome {
    /// `grbuf` reinterpreted as `f32`s.
    pub fn floats(&self) -> Vec<f32> {
        self.grbuf
            .chunks_exact(4)
            .map(|c| f32::from_le_bytes(c.try_into().unwrap()))
            .collect()
    }
    /// Number of `f32` slots that no longer hold the [`GRBUF_FILL`] pattern.
    pub fn written_slots(&self) -> usize {
        self.grbuf
            .chunks_exact(4)
            .filter(|c| c.iter().any(|&b| b != GRBUF_FILL))
            .count()
    }
    /// `true` if `grbuf` was not touched at all.
    pub fn grbuf_untouched(&self) -> bool {
        self.grbuf.iter().all(|&b| b == GRBUF_FILL)
    }
    /// Indices of the `f32` slots that were written, in order.
    pub fn written_indices(&self) -> Vec<usize> {
        self.grbuf
            .chunks_exact(4)
            .enumerate()
            .filter(|(_, c)| c.iter().any(|&b| b != GRBUF_FILL))
            .map(|(i, _)| i)
            .collect()
    }
}

fn run_one(imp: &Impl, fx: &mut Fixture, case: &Case, rep: u32) -> Outcome {
    fx.prepare(case, rep);
    let mut bs = fx.bs_for(case, rep);

    let grbuf_ptr =
        if case.null_grbuf { std::ptr::null_mut() } else { fx.grbuf.as_mut_ptr() };
    let bs_ptr =
        if case.null_bs { std::ptr::null_mut() } else { &mut bs as *mut BsT };
    let sci_ptr = fx.sci.as_mut_ptr() as *mut u8;

    let ret = unsafe { (imp.dequantize_granule)(grbuf_ptr, bs_ptr, sci_ptr, case.group_size) };

    Outcome {
        ret,
        grbuf: fx.grbuf_bytes().to_vec(),
        pos: bs.pos,
        limit: bs.limit,
        sci: fx.sci_bytes_ref().to_vec(),
    }
}

fn first_diff(a: &[u8], b: &[u8]) -> Option<usize> {
    a.iter().zip(b.iter()).position(|(x, y)| x != y)
}

/// Runs one [`Case`] against both implementations for `case.iters` randomized
/// repetitions and asserts byte-identical results.
pub fn check_case(c: &Impl, r: &Impl, case: &Case) {
    assert_case_safe(case);
    if std::env::var_os("DIFF_TRACE").is_some() {
        eprintln!("[case {}] {:?} kcap={}", case.id, case, kcap_for(case));
    }
    let mut fx = Fixture::new_for(case);
    for rep in 0..case.iters {
        if std::env::var_os("DIFF_TRACE").is_some() {
            eprintln!("  [case {} rep {}]", case.id, rep);
        }
        let oc = run_one(c, &mut fx, case, rep);
        let or = run_one(r, &mut fx, case, rep);

        assert_eq!(
            oc.ret, or.ret,
            "case {} rep {}: return value differs (C={} Rust={}) cfg={:?}",
            case.id, rep, oc.ret, or.ret, case
        );

        assert_eq!(oc.grbuf.len(), or.grbuf.len());
        if let Some(i) = first_diff(&oc.grbuf, &or.grbuf) {
            let fi = i / 4 * 4;
            panic!(
                "case {} rep {}: grbuf differs at byte {} (float index {})\n  \
                 C    = {:02x?} ({:?})\n  Rust = {:02x?} ({:?})\n  cfg = {:?}",
                case.id,
                rep,
                i,
                i / 4,
                &oc.grbuf[fi..fi + 4],
                f32::from_le_bytes(oc.grbuf[fi..fi + 4].try_into().unwrap()),
                &or.grbuf[fi..fi + 4],
                f32::from_le_bytes(or.grbuf[fi..fi + 4].try_into().unwrap()),
                case
            );
        }

        assert_eq!(
            oc.pos, or.pos,
            "case {} rep {}: bs->pos differs (C={} Rust={}) cfg={:?}",
            case.id, rep, oc.pos, or.pos, case
        );
        assert_eq!(
            oc.limit, or.limit,
            "case {} rep {}: bs->limit differs cfg={:?}",
            case.id, rep, case
        );

        if let Some(i) = first_diff(&oc.sci, &or.sci) {
            panic!(
                "case {} rep {}: sci bytes differ at offset {} (C={:#04x} Rust={:#04x}) cfg={:?}",
                case.id, rep, i, oc.sci[i], or.sci[i], case
            );
        }
    }
}

pub fn check_cases(cases: &[Case]) {
    let (c, r) = load_impls();
    for case in cases {
        check_case(&c, &r, case);
    }
}

/// Runs `case` (repetition `rep`) on one implementation with a fresh fixture.
pub fn run(imp: &Impl, case: &Case, rep: u32) -> Outcome {
    assert_case_safe(case);
    let mut fx = Fixture::new_for(case);
    run_one(imp, &mut fx, case, rep)
}

/// Runs `case` on both implementations, asserts they agree byte-for-byte, and
/// returns the (identical) outcome so the caller can assert the *absolute*
/// expected behaviour on top of the differential check.
pub fn run_both(c: &Impl, r: &Impl, case: &Case, rep: u32) -> Outcome {
    let oc = run(c, case, rep);
    let or = run(r, case, rep);
    assert_eq!(
        oc.ret, or.ret,
        "case {} rep {}: return value differs (C={} Rust={}) cfg={:?}",
        case.id, rep, oc.ret, or.ret, case
    );
    assert_eq!(
        oc.pos, or.pos,
        "case {} rep {}: bs->pos differs (C={} Rust={}) cfg={:?}",
        case.id, rep, oc.pos, or.pos, case
    );
    assert_eq!(oc.limit, or.limit, "case {} rep {}: bs->limit differs", case.id, rep);
    if let Some(i) = first_diff(&oc.grbuf, &or.grbuf) {
        let fi = i / 4 * 4;
        panic!(
            "case {} rep {}: grbuf differs at float {} (C={:?} Rust={:?}) cfg={:?}",
            case.id,
            rep,
            i / 4,
            f32::from_le_bytes(oc.grbuf[fi..fi + 4].try_into().unwrap()),
            f32::from_le_bytes(or.grbuf[fi..fi + 4].try_into().unwrap()),
            case
        );
    }
    if let Some(i) = first_diff(&oc.sci, &or.sci) {
        panic!("case {} rep {}: sci bytes differ at offset {}", case.id, rep, i);
    }
    oc
}
