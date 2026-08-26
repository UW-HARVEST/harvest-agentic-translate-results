//! Differential test harness: loads BOTH the C `.so` and the Rust `.so` with
//! `libloading` and compares `dequantize_granule` through the FFI boundary.
//!
//! Nothing here calls the Rust implementation directly -- every invocation goes
//! through `dlsym("dequantize_granule")` on the built cdylib, so the
//! `#[no_mangle] extern "C"` wrapper is under test too.
//!
//! * Phase B rows (`CONFIGS.md`) -> `phase_b_*` tests
//! * Phase C rows (`ERRORS.md`)  -> `phase_c_*` tests

#![allow(non_camel_case_types)]

use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::path::PathBuf;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// FFI types -- must match c_src/include/lib.h exactly.
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy)]
struct bs_t {
    buf: *const u8,
    pos: c_int,
    limit: c_int,
}

#[repr(C)]
#[derive(Clone)]
struct L12_scale_info {
    scf: [f32; 3 * 64],
    total_bands: u8,
    stereo_bands: u8,
    bitalloc: [u8; 64],
    scfcod: [u8; 64],
}


type DequantFn = unsafe extern "C" fn(*mut f32, *mut bs_t, *mut L12_scale_info, c_int) -> c_int;

// ---------------------------------------------------------------------------
// Arena geometry
// ---------------------------------------------------------------------------

/// Bytes of slack before `buf` (only reachable via a negative `bs->pos`, which
/// the harness normally re-bases away -- kept as a safety net).
const BUF_PAD: usize = 4096;
/// Readable bytes at/after the first byte `get_bits` touches.
const BUF_LEN: usize = 1 << 18;
const BUF_TOTAL: usize = BUF_PAD * 2 + BUF_LEN;
/// Largest bit count that keeps every accepted `get_bits` read inside the arena.
const MAX_BITS: i32 = ((BUF_LEN - 16) * 8) as i32;

/// `f32` slack before `grbuf`. C never writes below `grbuf` (`2*total_bands` is
/// always even so `choff` re-enters every granule at +576), but the pad is
/// compared anyway so that a Rust-only underflow write would be caught.
const GR_PAD: usize = 8192;
/// `f32` slack at/after `grbuf`. C's worst reachable element offset is
/// `3*group_size + 5148 + group_size - 1` (group_size <= 32 => 5275).
const GR_LEN: usize = 16384;
const GR_TOTAL: usize = GR_PAD * 2 + GR_LEN;

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    // `DIFF_C_SO` lets the same suite be pointed at a C `.so` built with
    // different optimisation levels (a robustness check for the UB-dependent
    // behaviour this library relies on: masked shift counts, signed overflow).
    if let Some(p) = std::env::var_os("DIFF_C_SO") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "DIFF_C_SO points at a missing file: {p:?}");
        return p;
    }
    let p = manifest_dir().join("c_src/build/libtranslated_rust.so");
    assert!(
        p.exists(),
        "C shared library not built: {p:?}\n\
         run: cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
    );
    p
}

fn rust_so_path() -> PathBuf {
    let name = "libdequantize_granule_lib.so";
    // Prefer the profile this test binary itself was built with.
    let primary = if cfg!(debug_assertions) { "debug" } else { "release" };
    let mut cands = vec![manifest_dir().join("target").join(primary).join(name)];
    for other in ["debug", "release"] {
        cands.push(manifest_dir().join("target").join(other).join(name));
    }
    for c in &cands {
        if c.exists() {
            // Guard against testing a stale cdylib: `cargo test` alone does not
            // rebuild the cdylib artifact, only `cargo build` does.
            let src = manifest_dir().join("src/lib.rs");
            if let (Ok(a), Ok(b)) = (c.metadata(), src.metadata()) {
                if let (Ok(ta), Ok(tb)) = (a.modified(), b.modified()) {
                    assert!(
                        ta >= tb,
                        "STALE Rust cdylib {c:?} is older than src/lib.rs -- \
                         run `cargo build` (or ./run_verification.sh) first"
                    );
                }
            }
            return c.clone();
        }
    }
    panic!(
        "Rust cdylib not found, tried {cands:?} -- run `cargo build` first \
         (`cargo test` does not emit the cdylib artifact)"
    );
}

struct Impls {
    c: DequantFn,
    rust: DequantFn,
}

fn impls() -> &'static Impls {
    static IMPLS: OnceLock<Impls> = OnceLock::new();
    IMPLS.get_or_init(|| {
        let load = |p: PathBuf| -> DequantFn {
            let lib = unsafe { Library::new(&p) }.unwrap_or_else(|e| panic!("dlopen {p:?}: {e}"));
            let lib: &'static Library = Box::leak(Box::new(lib));
            let sym: Symbol<'static, DequantFn> = unsafe { lib.get(b"dequantize_granule\0") }
                .unwrap_or_else(|e| panic!("dlsym dequantize_granule in {p:?}: {e}"));
            *sym
        };
        Impls {
            c: load(c_so_path()),
            rust: load(rust_so_path()),
        }
    })
}

// ---------------------------------------------------------------------------
// Deterministic RNG (xorshift64*) -- fixed seed => reproducible runs.
// ---------------------------------------------------------------------------

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed | 1)
    }
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    fn u8(&mut self) -> u8 {
        (self.next_u64() >> 33) as u8
    }
    /// Uniform in `lo..=hi`.
    fn range(&mut self, lo: i64, hi: i64) -> i64 {
        debug_assert!(hi >= lo);
        let span = (hi - lo + 1) as u64;
        lo + (self.next_u64() % span) as i64
    }
    fn pick<T: Copy>(&mut self, xs: &[T]) -> T {
        xs[(self.next_u64() % xs.len() as u64) as usize]
    }
    fn fill(&mut self, dst: &mut [u8]) {
        for b in dst.iter_mut() {
            *b = self.u8();
        }
    }
}

// ---------------------------------------------------------------------------
// The `L12_scale_info` arena
//
// It is held as a raw `Vec<u32>` (4-byte aligned, like the struct) instead of a
// typed value on purpose: C reads `bitalloc[i]` for `i` up to 509, which runs
// through `scfcod`, through the struct's 2 tail PADDING bytes, and on past the
// struct. A `#[derive(Clone)]` does **not** copy padding, so a typed clone
// would feed C and Rust different bytes. A byte-exact arena removes that.
// ---------------------------------------------------------------------------

/// `sizeof(L12_scale_info)` == 900, plus slack for the OOB `bitalloc` reads.
const SCI_ARENA_BYTES: usize = 2180;
const SCI_WORDS: usize = SCI_ARENA_BYTES / 4;

const OFF_TOTAL_BANDS: usize = 768;
const OFF_STEREO_BANDS: usize = 769;
const OFF_BITALLOC: usize = 770;

/// Byte-exact image of an `L12_scale_info` (+ trailing slack).
type Sci = Vec<u32>;

fn sci_get(v: &Sci, off: usize) -> u8 {
    assert!(off < SCI_ARENA_BYTES);
    unsafe { *(v.as_ptr() as *const u8).add(off) }
}

fn sci_set(v: &mut Sci, off: usize, b: u8) {
    assert!(off < SCI_ARENA_BYTES);
    unsafe { *(v.as_mut_ptr() as *mut u8).add(off) = b }
}

fn sci_tb(v: &Sci) -> u8 {
    sci_get(v, OFF_TOTAL_BANDS)
}

/// `bitalloc[i]` read exactly the way C does it: unchecked, spilling past the
/// 64-byte array into `scfcod`, the padding, and beyond.
fn sci_ba(v: &Sci, i: usize) -> u8 {
    sci_get(v, OFF_BITALLOC + i)
}

fn sci_set_ba(v: &mut Sci, i: usize, b: u8) {
    sci_set(v, OFF_BITALLOC + i, b)
}

/// Fully randomized arena (scf, stereo_bands, bitalloc, scfcod, padding, slack).
fn make_sci(rng: &mut Rng, total_bands: u8) -> Sci {
    let mut v: Sci = (0..SCI_WORDS).map(|_| rng.u32()).collect();
    sci_set(&mut v, OFF_TOTAL_BANDS, total_bands);
    v
}

/// Overwrite `n` bytes starting at `bitalloc` (spilling into `scfcod` /
/// padding / slack when `n > 64`, exactly the region C's unchecked
/// `bitalloc[i]` reaches).
fn fill_bitalloc_span(rng: &mut Rng, sci: &mut Sci, classes: &[(u8, u8)], n: usize) {
    assert!(OFF_BITALLOC + n <= SCI_ARENA_BYTES);
    for i in 0..n {
        let (lo, hi) = rng.pick(classes);
        let b = rng.range(lo as i64, hi as i64) as u8;
        sci_set_ba(sci, i, b);
    }
}

// ---------------------------------------------------------------------------
// Case description + runner
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum LimitSpec {
    /// Use this exact `bs->limit`.
    Abs(i32),
    /// `bs->limit = pos.wrapping_add(bits)`.
    RelBits(i32),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum BufSpec {
    /// `bs->buf` is re-based so that the first byte `get_bits` touches is
    /// `arena[BUF_PAD]`, whatever `bs->pos` is. Lets huge / negative `pos`
    /// values be exercised without leaving the arena.
    Rebased,
    /// `bs->buf = arena.as_ptr() + BUF_PAD` verbatim (`pos>>3` applies on top).
    Raw,
    /// `bs->buf = NULL`.
    Null,
}

#[derive(Clone)]
struct Case {
    label: String,
    buf: Vec<u8>,
    sci: Sci,
    pos: i32,
    limit: LimitSpec,
    group_size: i32,
    buf_spec: BufSpec,
    null_grbuf: bool,
}

impl Case {
    fn limit_value(&self) -> i32 {
        match self.limit {
            LimitSpec::Abs(l) => l,
            LimitSpec::RelBits(b) => self.pos.wrapping_add(b),
        }
    }
    /// Index of `arena[BUF_PAD]` expressed as an offset from `bs->buf`.
    fn buf_base_index(&self) -> i64 {
        match self.buf_spec {
            BufSpec::Rebased => BUF_PAD as i64 - ((self.pos >> 3) as i64),
            BufSpec::Raw => BUF_PAD as i64,
            BufSpec::Null => 0,
        }
    }
}

#[derive(PartialEq, Eq)]
struct Outcome {
    ret: c_int,
    gr: Vec<u32>,
    buf: Vec<u8>,
    sci_after: Sci,
    pos: i32,
    limit: i32,
    buf_ptr_unchanged: bool,
}

fn fresh_gr() -> Vec<u32> {
    // Unique sentinel per slot: any stray write shows up in the diff.
    (0..GR_TOTAL as u32).map(|i| 0xDEAD_0000u32 ^ i).collect()
}

// ---------------------------------------------------------------------------
// Observability model
//
// `get_bits`'s accept/reject decisions and the byte offsets it dereferences are
// FULLY determined by (bitalloc, total_bands, group_size, pos, limit) -- they do
// not depend on the reservoir CONTENTS. So the harness can predict, exactly,
// every address both libraries will touch, and decline to run a case whose
// (identically wild in C and in Rust) accesses would leave the arenas and take
// the test process down with them. Nothing about the comparison is relaxed --
// a case either runs fully or is reported as skipped.
// ---------------------------------------------------------------------------

#[derive(Default, Debug)]
struct Reach {
    /// Inclusive arena index range dereferenced by `get_bits` (None = no reads).
    read: Option<(i64, i64)>,
    /// Inclusive `gr` arena index range written (None = no writes).
    write: Option<(i64, i64)>,
    /// A read was accepted while `bs->buf` is NULL.
    null_deref: bool,
}

fn reach(case: &Case) -> Reach {
    let limit = case.limit_value() as i64;
    let base = case.buf_base_index();
    let gs = case.group_size;
    let tb = sci_tb(&case.sci) as i32;

    let mut r = Reach::default();
    let mut pos: i32 = case.pos;
    let mut choff: i32 = 576;

    let note_read = |pos_before: i32, n: i32, r: &mut Reach| {
        let s = (pos_before & 7) as i64;
        let p0 = base + ((pos_before >> 3) as i64);
        // bytes read = ceil((n + s) / 8); n >= 1 and s <= 7 here, so no overflow
        let shl0 = n as i64 + s;
        let bytes = (shl0 + 7) / 8;
        let (lo, hi) = (p0, p0 + bytes - 1);
        r.read = Some(match r.read {
            None => (lo, hi),
            Some((a, b)) => (a.min(lo), b.max(hi)),
        });
        if matches!(case.buf_spec, BufSpec::Null) {
            r.null_deref = true;
        }
    };
    let note_write = |lo: i64, hi: i64, r: &mut Reach| {
        r.write = Some(match r.write {
            None => (lo, hi),
            Some((a, b)) => (a.min(lo), b.max(hi)),
        });
    };

    for j in 0..4i32 {
        let mut off: i64 = GR_PAD as i64 + (gs.wrapping_mul(j) as i64);
        for i in 0..(2 * tb) {
            let ba = sci_ba(&case.sci, i as usize) as i32;
            if ba != 0 {
                if ba < 17 {
                    let mut k = 0i32;
                    while k < gs {
                        let before = pos;
                        pos = pos.wrapping_add(ba);
                        if (pos as i64) <= limit {
                            note_read(before, ba, &mut r);
                        }
                        k += 1;
                    }
                } else {
                    let m = 2u32.wrapping_shl((ba - 17) as u32).wrapping_add(1);
                    let n = m.wrapping_add(2).wrapping_sub(m >> 3) as i32;
                    let before = pos;
                    pos = pos.wrapping_add(n);
                    if (pos as i64) <= limit {
                        note_read(before, n, &mut r);
                    }
                }
                if gs > 0 {
                    note_write(off, off + gs as i64 - 1, &mut r);
                }
            }
            off += choff as i64;
            choff = 18 - choff;
        }
    }
    r
}

/// Can this case be observed in-process, or would it take the harness down?
fn observable(case: &Case) -> bool {
    let r = reach(case);
    if r.null_deref {
        return false;
    }
    if let Some((lo, hi)) = r.read {
        if lo < 0 || hi >= BUF_TOTAL as i64 {
            return false;
        }
    }
    if let Some((lo, hi)) = r.write {
        if case.null_grbuf {
            return false;
        }
        if lo < 0 || hi >= GR_TOTAL as i64 {
            return false;
        }
    }
    true
}

fn run_one(f: DequantFn, case: &Case) -> Outcome {
    let mut gr = fresh_gr();
    let buf = case.buf.clone();
    let mut sci = case.sci.clone();
    assert_eq!(sci.len(), SCI_WORDS);

    let base = buf.as_ptr().wrapping_add(BUF_PAD);
    let bufptr: *const u8 = match case.buf_spec {
        BufSpec::Rebased => base.wrapping_offset(-((case.pos >> 3) as isize)),
        BufSpec::Raw => base,
        BufSpec::Null => std::ptr::null(),
    };

    let mut bs = bs_t {
        buf: bufptr,
        pos: case.pos,
        limit: case.limit_value(),
    };
    let grptr: *mut f32 = if case.null_grbuf {
        std::ptr::null_mut()
    } else {
        (gr.as_mut_ptr() as *mut f32).wrapping_add(GR_PAD)
    };
    let sciptr = sci.as_mut_ptr() as *mut L12_scale_info;

    let ret = unsafe { f(grptr, &mut bs, sciptr, case.group_size) };

    Outcome {
        ret,
        gr,
        buf,
        sci_after: sci,
        pos: bs.pos,
        limit: bs.limit,
        buf_ptr_unchanged: bs.buf == bufptr,
    }
}

/// Run the case through both `.so`s and assert byte-identical results.
/// Returns `false` (and does nothing) if the case is not observable in-process.
#[track_caller]
fn assert_same(case: &Case) -> bool {
    if !observable(case) {
        if std::env::var_os("DIFF_TRACE").is_some() {
            eprintln!("SKIP (unobservable) {} :: {:?}", case.label, reach(case));
        }
        return false;
    }
    if std::env::var_os("DIFF_TRACE").is_some() {
        eprintln!("CASE {}", case.label);
    }
    let i = impls();
    let c = run_one(i.c, case);
    let r = run_one(i.rust, case);

    let ctx = || {
        let ba: Vec<u8> = (0..std::cmp::max(64, 2 * sci_tb(&case.sci) as usize))
            .map(|i| sci_ba(&case.sci, i))
            .collect();
        format!(
            "{}\n  group_size={} pos={} limit={:?}(={}) buf={:?} null_grbuf={}\n  \
             total_bands={} stereo_bands={} reach={:?}\n  bitalloc[..]={:?}",
            case.label,
            case.group_size,
            case.pos,
            case.limit,
            case.limit_value(),
            case.buf_spec,
            case.null_grbuf,
            sci_tb(&case.sci),
            sci_get(&case.sci, OFF_STEREO_BANDS),
            reach(case),
            ba,
        )
    };

    assert_eq!(c.ret, r.ret, "return value mismatch\n{}", ctx());
    assert_eq!(
        c.pos,
        r.pos,
        "bs->pos mismatch (C={} rust={})\n{}",
        c.pos,
        r.pos,
        ctx()
    );
    assert_eq!(c.limit, r.limit, "bs->limit mismatch\n{}", ctx());
    assert_eq!(
        c.buf_ptr_unchanged,
        r.buf_ptr_unchanged,
        "bs->buf was mutated by exactly one impl\n{}",
        ctx()
    );
    assert!(c.buf == r.buf, "bs->buf contents mismatch\n{}", ctx());
    assert!(
        c.sci_after == r.sci_after,
        "L12_scale_info mutated differently\n{}",
        ctx()
    );

    if c.gr != r.gr {
        let mut diffs = Vec::new();
        for k in 0..GR_TOTAL {
            if c.gr[k] != r.gr[k] {
                diffs.push(format!(
                    "  grbuf[{}] C=0x{:08x} ({}) rust=0x{:08x} ({})",
                    k as isize - GR_PAD as isize,
                    c.gr[k],
                    f32::from_bits(c.gr[k]),
                    r.gr[k],
                    f32::from_bits(r.gr[k]),
                ));
                if diffs.len() == 12 {
                    break;
                }
            }
        }
        panic!("grbuf mismatch\n{}\n{}", ctx(), diffs.join("\n"));
    }
    true
}

// ---------------------------------------------------------------------------
// Per-test coverage tally
//
// `assert_same` returns false for cases whose accesses are (identically) wild in
// both libraries and therefore cannot be observed in-process. Every test counts
// them so that a row can never silently degrade into "ran nothing".
// ---------------------------------------------------------------------------

#[derive(Default)]
struct Tally {
    ran: usize,
    skipped: usize,
}

impl Tally {
    #[track_caller]
    fn run(&mut self, case: &Case) {
        if assert_same(case) {
            self.ran += 1;
        } else {
            self.skipped += 1;
        }
    }

    /// Run the differential comparison, then pin the documented C behaviour.
    #[track_caller]
    fn run_with(&mut self, case: &Case, extra: impl FnOnce(&Outcome)) {
        if assert_same(case) {
            self.ran += 1;
            let c = run_one(impls().c, case);
            extra(&c);
        } else {
            self.skipped += 1;
        }
    }

    #[track_caller]
    fn finish(&self, name: &str, min_ran: usize) {
        eprintln!("[{name}] observable cases ran={} skipped={}", self.ran, self.skipped);
        assert!(
            self.ran >= min_ran,
            "{name}: only {} observable cases ran (expected >= {min_ran}); skipped {}",
            self.ran,
            self.skipped
        );
    }
}

// ---------------------------------------------------------------------------
// Case builders
// ---------------------------------------------------------------------------

/// How to populate the bit-reservoir bytes.
#[derive(Clone, Copy)]
enum Bytes {
    Zeros,
    Ones,
    Aa55,
    Random,
}

fn make_buf(rng: &mut Rng, kind: Bytes) -> Vec<u8> {
    let mut v = vec![0u8; BUF_TOTAL];
    match kind {
        Bytes::Zeros => {}
        Bytes::Ones => v.fill(0xFF),
        Bytes::Aa55 => {
            for (i, b) in v.iter_mut().enumerate() {
                *b = if i % 2 == 0 { 0xAA } else { 0x55 };
            }
        }
        Bytes::Random => rng.fill(&mut v),
    }
    v
}

/// Test-side model of how many bits the granule consumes if nothing is
/// rejected. Used only to *construct* reservoir limits, never to check outputs.
fn consumed_bits(sci: &Sci, group_size: i32) -> i64 {
    let mut bits = 0i64;
    for _j in 0..4 {
        for i in 0..(2 * sci_tb(sci) as usize) {
            let ba = sci_ba(sci, i) as i64;
            if ba == 0 {
                continue;
            }
            if ba < 17 {
                if group_size > 0 {
                    bits += ba * group_size as i64;
                }
            } else {
                let m = 2u32
                    .wrapping_shl((ba as u32).wrapping_sub(17))
                    .wrapping_add(1);
                bits += m.wrapping_add(2).wrapping_sub(m >> 3) as i32 as i64;
            }
        }
    }
    bits
}

const LOW: &[(u8, u8)] = &[(1, 16)];
const LOW_NARROW: &[(u8, u8)] = &[(1, 8)];
const LOW_WIDE: &[(u8, u8)] = &[(9, 16)];
const HIGH_SMALL: &[(u8, u8)] = &[(17, 24)];
const MIXED: &[(u8, u8)] = &[(0, 0), (1, 16), (17, 32)];
const ANY: &[(u8, u8)] = &[(0, 255)];

fn base_case(
    label: String,
    rng: &mut Rng,
    total_bands: u8,
    classes: &[(u8, u8)],
    group_size: i32,
    bytes: Bytes,
) -> Case {
    let mut sci = make_sci(rng, total_bands);
    let span = std::cmp::max(64, 2 * total_bands as usize);
    fill_bitalloc_span(rng, &mut sci, classes, span);
    Case {
        label,
        buf: make_buf(rng, bytes),
        sci,
        pos: 0,
        limit: LimitSpec::RelBits(MAX_BITS),
        group_size,
        buf_spec: BufSpec::Raw,
        null_grbuf: false,
    }
}

// ===========================================================================
// Layout / symbol sanity
// ===========================================================================

#[test]
fn layout_matches_c() {
    use std::mem::{align_of, offset_of, size_of};
    assert_eq!(size_of::<bs_t>(), 16);
    assert_eq!(offset_of!(bs_t, buf), 0);
    assert_eq!(offset_of!(bs_t, pos), 8);
    assert_eq!(offset_of!(bs_t, limit), 12);

    assert_eq!(size_of::<L12_scale_info>(), 900);
    assert_eq!(align_of::<L12_scale_info>(), 4);
    assert_eq!(offset_of!(L12_scale_info, scf), 0);
    assert_eq!(offset_of!(L12_scale_info, total_bands), 768);
    assert_eq!(offset_of!(L12_scale_info, stereo_bands), 769);
    assert_eq!(offset_of!(L12_scale_info, bitalloc), 770);
    assert_eq!(offset_of!(L12_scale_info, scfcod), 834);
    // bitalloc[509] (the highest index C can reach) must stay inside the arena.
    assert_eq!(OFF_BITALLOC, offset_of!(L12_scale_info, bitalloc));
    assert_eq!(OFF_TOTAL_BANDS, offset_of!(L12_scale_info, total_bands));
    assert_eq!(OFF_STEREO_BANDS, offset_of!(L12_scale_info, stereo_bands));
    assert!(size_of::<L12_scale_info>() <= SCI_ARENA_BYTES);
    assert!(OFF_BITALLOC + 510 <= SCI_ARENA_BYTES);
    assert_eq!(SCI_ARENA_BYTES % 4, 0);
}

#[test]
fn both_libraries_export_dequantize_granule() {
    let i = impls();
    assert!(i.c as usize != 0);
    assert!(i.rust as usize != 0);
    assert!(
        i.c as usize != i.rust as usize,
        "both symbols resolved to the same address -- only one .so got loaded"
    );
}

// ===========================================================================
// Phase B -- CONFIGS.md rows 1..25
// ===========================================================================

/// Row 1 -- empty: total_bands == 0, random (ignored) bitalloc.
#[test]
fn phase_b_row01_empty_total_bands() {
    let mut t = Tally::default();
    let mut rng = Rng::new(0x5EED_1234);
    for &gs in &[0i32, 1, 4, 12, 18, 32] {
        for it in 0..60 {
            let c = base_case(
                format!("row01 empty gs={gs} it={it}"),
                &mut rng,
                0,
                ANY,
                gs,
                Bytes::Random,
            );
            t.run(&c);
        }
    }
    t.finish("phase_b_row01_empty_total_bands", 1);
}

/// Row 2 -- all bands skipped (bitalloc all zero).
#[test]
fn phase_b_row02_all_skip() {
    let mut t = Tally::default();
    let mut rng = Rng::new(0x5EED_1234 ^ 2);
    for &tb in &[1u8, 2, 8, 32] {
        for &gs in &[1i32, 4, 12, 18] {
            for it in 0..30 {
                let mut c = base_case(
                    format!("row02 all-skip tb={tb} gs={gs} it={it}"),
                    &mut rng,
                    tb,
                    &[(0, 0)],
                    gs,
                    Bytes::Random,
                );
                // also zero the OOB region so nothing sneaks in
                fill_bitalloc_span(&mut rng, &mut c.sci, &[(0, 0)], 512);
                t.run(&c);
            }
        }
    }
    t.finish("phase_b_row02_all_skip", 1);
}

/// Row 3 -- single narrow band (1..=8 bits => get_bits never enters its loop).
#[test]
fn phase_b_row03_single_narrow_band() {
    let mut t = Tally::default();
    let mut rng = Rng::new(0x5EED_1234 ^ 3);
    for &gs in &[1i32, 2, 3, 4, 12] {
        for it in 0..60 {
            let mut c = base_case(
                format!("row03 narrow gs={gs} it={it}"),
                &mut rng,
                1,
                LOW_NARROW,
                gs,
                Bytes::Random,
            );
            c.limit = LimitSpec::Abs(i32::MAX);
            t.run(&c);
        }
    }
    t.finish("phase_b_row03_single_narrow_band", 1);
}

/// Row 4 -- single wide band (9..=16 bits => one get_bits loop iteration).
#[test]
fn phase_b_row04_single_wide_band() {
    let mut t = Tally::default();
    let mut rng = Rng::new(0x5EED_1234 ^ 4);
    for &gs in &[1i32, 4, 12, 18] {
        for it in 0..60 {
            let mut c = base_case(
                format!("row04 wide gs={gs} it={it}"),
                &mut rng,
                1,
                LOW_WIDE,
                gs,
                Bytes::Random,
            );
            c.limit = LimitSpec::Abs(i32::MAX);
            t.run(&c);
        }
    }
    t.finish("phase_b_row04_single_wide_band", 1);
}

/// Row 5 -- many low bands, limit == INT_MAX (never rejects).
#[test]
fn phase_b_row05_many_low_bands() {
    let mut t = Tally::default();
    let mut rng = Rng::new(0x5EED_1234 ^ 5);
    for &tb in &[2u8, 8, 31, 32] {
        for &gs in &[1i32, 4, 12] {
            for it in 0..25 {
                let mut c = base_case(
                    format!("row05 low tb={tb} gs={gs} it={it}"),
                    &mut rng,
                    tb,
                    LOW,
                    gs,
                    Bytes::Random,
                );
                c.limit = LimitSpec::Abs(i32::MAX);
                t.run(&c);
            }
        }
    }
    t.finish("phase_b_row05_many_low_bands", 1);
}

/// Row 6 -- unaligned bit positions (all `s = pos & 7` values).
#[test]
fn phase_b_row06_unaligned_start() {
    let mut t = Tally::default();
    let mut rng = Rng::new(0x5EED_1234 ^ 6);
    for &tb in &[2u8, 8, 32] {
        for &gs in &[1i32, 4, 12] {
            for it in 0..25 {
                let mut c = base_case(
                    format!("row06 unaligned tb={tb} gs={gs} it={it}"),
                    &mut rng,
                    tb,
                    LOW,
                    gs,
                    Bytes::Random,
                );
                c.pos = if it % 2 == 0 {
                    rng.range(1, 7) as i32
                } else {
                    rng.range(8, 63) as i32
                };
                c.limit = LimitSpec::Abs(i32::MAX);
                t.run(&c);
            }
        }
    }
    t.finish("phase_b_row06_unaligned_start", 1);
}

/// Row 7 -- grouped ("high") bands only: the `ba >= 17` mod/code path.
#[test]
fn phase_b_row07_high_bands_only() {
    let mut t = Tally::default();
    let mut rng = Rng::new(0x5EED_1234 ^ 7);
    for &tb in &[1u8, 2, 8] {
        for &gs in &[1i32, 3, 12] {
            for it in 0..40 {
                let c = base_case(
                    format!("row07 high tb={tb} gs={gs} it={it}"),
                    &mut rng,
                    tb,
                    HIGH_SMALL,
                    gs,
                    Bytes::Random,
                );
                t.run(&c);
            }
        }
    }
    t.finish("phase_b_row07_high_bands_only", 1);
}

/// Row 8 -- exhaustive `s` sweep: pos = 0..=15.
#[test]
fn phase_b_row08_s_sweep() {
    let mut t = Tally::default();
    let mut rng = Rng::new(0x5EED_1234 ^ 8);
    for pos in 0..=15i32 {
        for it in 0..40 {
            let mut c = base_case(
                format!("row08 s-sweep pos={pos} it={it}"),
                &mut rng,
                4,
                MIXED,
                12,
                Bytes::Random,
            );
            c.pos = pos;
            t.run(&c);
        }
    }
    t.finish("phase_b_row08_s_sweep", 1);
}

/// Row 9 -- mixed skip / low / high bands.
#[test]
fn phase_b_row09_mixed_bands() {
    let mut t = Tally::default();
    let mut rng = Rng::new(0x5EED_1234 ^ 9);
    for &tb in &[2u8, 8, 32] {
        for &gs in &[1i32, 4, 12, 18] {
            for it in 0..25 {
                let c = base_case(
                    format!("row09 mixed tb={tb} gs={gs} it={it}"),
                    &mut rng,
                    tb,
                    MIXED,
                    gs,
                    Bytes::Random,
                );
                t.run(&c);
            }
        }
    }
    t.finish("phase_b_row09_mixed_bands", 1);
}

/// Row 10 -- full 0..=255 bitalloc opcode range (mod overflow, n >= 32, ...).
#[test]
fn phase_b_row10_full_opcode_range() {
    let mut t = Tally::default();
    let mut rng = Rng::new(0x5EED_1234 ^ 10);
    for &tb in &[1u8, 8, 32] {
        for &gs in &[1i32, 4, 12] {
            for it in 0..30 {
                let c = base_case(
                    format!("row10 any tb={tb} gs={gs} it={it}"),
                    &mut rng,
                    tb,
                    ANY,
                    gs,
                    Bytes::Random,
                );
                t.run(&c);
                // Companion with an empty reservoir: every read is rejected, so
                // no memory is touched and the case is ALWAYS observable. This
                // still exercises the full `mod`/`n` arithmetic and the wrapping
                // `bs->pos` walk for the whole 0..=255 opcode domain.
                let mut starved = c.clone();
                starved.label = format!("row10 any-starved tb={tb} gs={gs} it={it}");
                starved.limit = LimitSpec::Abs(0);
                t.run(&starved);
            }
        }
    }
    t.finish("phase_b_row10_full_opcode_range", 1);
}

/// Row 11 -- `mod` boundary sweep on a single band.
#[test]
fn phase_b_row11_mod_boundaries() {
    let mut t = Tally::default();
    let mut rng = Rng::new(0x5EED_1234 ^ 11);
    for &ba in &[16u8, 17, 18, 31, 32, 33, 47, 48, 49, 255] {
        for &gs in &[1i32, 2, 12] {
            for it in 0..25 {
                let mut c = base_case(
                    format!("row11 ba={ba} gs={gs} it={it}"),
                    &mut rng,
                    1,
                    &[(0, 0)],
                    gs,
                    Bytes::Random,
                );
                sci_set_ba(&mut c.sci, 0, ba);
                sci_set_ba(&mut c.sci, 1, ba);
                t.run(&c);
            }
        }
    }
    t.finish("phase_b_row11_mod_boundaries", 1);
}

/// Row 12 -- reservoir sized to exactly fit (boundary: pos+n == limit).
#[test]
fn phase_b_row12_exact_fit_reservoir() {
    let mut t = Tally::default();
    let mut rng = Rng::new(0x5EED_1234 ^ 12);
    for &tb in &[1u8, 2, 8, 32] {
        for &gs in &[1i32, 4, 12] {
            for it in 0..25 {
                let mut c = base_case(
                    format!("row12 exact tb={tb} gs={gs} it={it}"),
                    &mut rng,
                    tb,
                    LOW,
                    gs,
                    Bytes::Random,
                );
                c.limit = LimitSpec::Abs(consumed_bits(&c.sci, gs) as i32);
                t.run(&c);
            }
        }
    }
    t.finish("phase_b_row12_exact_fit_reservoir", 1);
}

/// Row 13 -- one bit short of exact fit (last field rejects).
#[test]
fn phase_b_row13_one_bit_short() {
    let mut t = Tally::default();
    let mut rng = Rng::new(0x5EED_1234 ^ 13);
    for &tb in &[1u8, 2, 8, 32] {
        for &gs in &[1i32, 4, 12] {
            for it in 0..25 {
                let mut c = base_case(
                    format!("row13 short tb={tb} gs={gs} it={it}"),
                    &mut rng,
                    tb,
                    LOW,
                    gs,
                    Bytes::Random,
                );
                c.limit = LimitSpec::Abs(consumed_bits(&c.sci, gs) as i32 - 1);
                t.run(&c);
            }
        }
    }
    t.finish("phase_b_row13_one_bit_short", 1);
}

/// Row 14 -- exhaustion part-way through the granule.
#[test]
fn phase_b_row14_mid_stream_exhaustion() {
    let mut t = Tally::default();
    let mut rng = Rng::new(0x5EED_1234 ^ 14);
    for &tb in &[2u8, 8, 32] {
        for &gs in &[1i32, 4, 12] {
            for it in 0..40 {
                let mut c = base_case(
                    format!("row14 mid tb={tb} gs={gs} it={it}"),
                    &mut rng,
                    tb,
                    MIXED,
                    gs,
                    Bytes::Random,
                );
                let total = consumed_bits(&c.sci, gs).max(1);
                c.limit = LimitSpec::Abs(rng.range(0, total) as i32);
                t.run(&c);
            }
        }
    }
    t.finish("phase_b_row14_mid_stream_exhaustion", 1);
}

/// Row 15 -- limit == 0 with nonzero bitalloc: every get_bits rejects.
#[test]
fn phase_b_row15_limit_zero() {
    let mut t = Tally::default();
    let mut rng = Rng::new(0x5EED_1234 ^ 15);
    for &tb in &[1u8, 8, 32] {
        for &gs in &[1i32, 4, 12] {
            for it in 0..25 {
                let mut c = base_case(
                    format!("row15 limit0 tb={tb} gs={gs} it={it}"),
                    &mut rng,
                    tb,
                    MIXED,
                    gs,
                    Bytes::Random,
                );
                c.limit = LimitSpec::Abs(0);
                t.run(&c);
            }
        }
    }
    t.finish("phase_b_row15_limit_zero", 1);
}

/// Row 16 -- negative limit.
#[test]
fn phase_b_row16_negative_limit() {
    let mut t = Tally::default();
    let mut rng = Rng::new(0x5EED_1234 ^ 16);
    for &lim in &[-1i32, -12345, i32::MIN] {
        for &tb in &[1u8, 8, 32] {
            for it in 0..25 {
                let mut c = base_case(
                    format!("row16 lim={lim} tb={tb} it={it}"),
                    &mut rng,
                    tb,
                    MIXED,
                    12,
                    Bytes::Random,
                );
                c.limit = LimitSpec::Abs(lim);
                t.run(&c);
            }
        }
    }
    t.finish("phase_b_row16_negative_limit", 1);
}

/// Row 17 -- reservoir byte-content shapes.
#[test]
fn phase_b_row17_buffer_shapes() {
    let mut t = Tally::default();
    let mut rng = Rng::new(0x5EED_1234 ^ 17);
    for kind in [Bytes::Zeros, Bytes::Ones, Bytes::Aa55, Bytes::Random] {
        for classes in [LOW, HIGH_SMALL, MIXED] {
            for it in 0..30 {
                let mut c = base_case(
                    format!("row17 buf-shape it={it}"),
                    &mut rng,
                    8,
                    classes,
                    12,
                    kind,
                );
                c.pos = rng.range(0, 15) as i32;
                t.run(&c);
            }
        }
    }
    t.finish("phase_b_row17_buffer_shapes", 1);
}

/// Row 18 -- group_size shape sweep.
#[test]
fn phase_b_row18_group_size_sweep() {
    let mut t = Tally::default();
    let mut rng = Rng::new(0x5EED_1234 ^ 18);
    for &gs in &[1i32, 2, 3, 4, 5, 8, 12, 16, 18, 32] {
        for it in 0..40 {
            let c = base_case(
                format!("row18 gs={gs} it={it}"),
                &mut rng,
                8,
                MIXED,
                gs,
                Bytes::Random,
            );
            t.run(&c);
        }
    }
    t.finish("phase_b_row18_group_size_sweep", 1);
}

/// Row 19 -- negative group_size: no writes, but `ba>=17` still eats one field.
#[test]
fn phase_b_row19_negative_group_size() {
    let mut t = Tally::default();
    let mut rng = Rng::new(0x5EED_1234 ^ 19);
    for &gs in &[-1i32, -7, -1000, i32::MIN] {
        for &tb in &[1u8, 8, 32] {
            for it in 0..25 {
                let c = base_case(
                    format!("row19 gs={gs} tb={tb} it={it}"),
                    &mut rng,
                    tb,
                    MIXED,
                    gs,
                    Bytes::Random,
                );
                t.run(&c);
            }
        }
    }
    t.finish("phase_b_row19_negative_group_size", 1);
}

/// Row 20 -- total_bands > 32 => `bitalloc[i]` runs into scfcod and past the struct.
#[test]
fn phase_b_row20_bitalloc_oob_index() {
    let mut t = Tally::default();
    let mut rng = Rng::new(0x5EED_1234 ^ 20);
    for &tb in &[33u8, 64, 128, 255] {
        for &gs in &[1i32, 4, 12] {
            for it in 0..20 {
                let c = base_case(
                    format!("row20 oob tb={tb} gs={gs} it={it}"),
                    &mut rng,
                    tb,
                    MIXED,
                    gs,
                    Bytes::Random,
                );
                t.run(&c);
            }
        }
    }
    t.finish("phase_b_row20_bitalloc_oob_index", 1);
}

/// Row 21 -- `bs->pos += n` signed overflow makes the limit check pass.
///
/// Two constructions, both observable in-process (see `reach`/`observable`):
///  (a) `pos = INT_MAX`, `limit = INT_MAX + n` (wrapped, negative): call #1
///      wraps and is accepted (`pos_new == limit`), calls #2..#4 reject.
///  (b) `pos = INT_MAX - 3n`, `limit = INT_MAX`: calls #1..#3 are ordinary,
///      call #4 -- the LAST one -- wraps and is accepted. `p` is derived from
///      the pre-increment `pos`, so the read itself stays in the arena.
#[test]
fn phase_b_row21_pos_overflow() {
    let mut t = Tally::default();
    let mut rng = Rng::new(0x5EED_1234 ^ 21);
    for ba in 1u8..=16 {
        for it in 0..8 {
            // (a)
            let mut a = base_case(
                format!("row21a pos=INT_MAX ba={ba} it={it}"),
                &mut rng,
                1,
                &[(0, 0)],
                1,
                Bytes::Random,
            );
            sci_set_ba(&mut a.sci, 0, ba);
            sci_set_ba(&mut a.sci, 1, 0);
            a.pos = i32::MAX;
            a.buf_spec = BufSpec::Rebased;
            a.limit = LimitSpec::Abs(i32::MAX.wrapping_add(ba as i32));
            t.run(&a);

            // (b)
            let mut b = a.clone();
            b.label = format!("row21b pos=INT_MAX-3n ba={ba} it={it}");
            b.pos = i32::MAX - 3 * ba as i32;
            b.limit = LimitSpec::Abs(i32::MAX);
            t.run(&b);
        }
    }
    t.finish("phase_b_row21_pos_overflow", 16);
}

/// Row 22 -- negative `bs->pos`: `p = buf + (pos>>3)` walks backwards.
#[test]
fn phase_b_row22_negative_pos() {
    let mut t = Tally::default();
    let mut rng = Rng::new(0x5EED_1234 ^ 22);
    for &pos in &[-1i32, -2, -7, -8, -9, -64, -1000, -32768] {
        for it in 0..30 {
            let mut c = base_case(
                format!("row22 pos={pos} it={it}"),
                &mut rng,
                8,
                MIXED,
                12,
                Bytes::Random,
            );
            c.pos = pos;
            c.buf_spec = BufSpec::Rebased;
            t.run(&c);
        }
    }
    t.finish("phase_b_row22_negative_pos", 1);
}

/// Row 23 -- huge group_size: `group_size * 4` overflows (no writes: tb == 0).
#[test]
fn phase_b_row23_huge_group_size() {
    let mut t = Tally::default();
    let mut rng = Rng::new(0x5EED_1234 ^ 23);
    for &gs in &[0x4000_0000i32, i32::MAX, 0x7FFF_FFF0, 0x2000_0001, 1 << 20] {
        for it in 0..20 {
            let c = base_case(
                format!("row23 gs={gs} it={it}"),
                &mut rng,
                0,
                ANY,
                gs,
                Bytes::Random,
            );
            t.run(&c);
        }
    }
    t.finish("phase_b_row23_huge_group_size", 1);
}

/// Row 24 -- full `choff` walk: total_bands == 255, all low bitalloc.
#[test]
fn phase_b_row24_full_choff_walk() {
    let mut t = Tally::default();
    let mut rng = Rng::new(0x5EED_1234 ^ 24);
    for &tb in &[128u8, 200, 255] {
        for &gs in &[1i32, 4] {
            for it in 0..12 {
                let mut c = base_case(
                    format!("row24 walk tb={tb} gs={gs} it={it}"),
                    &mut rng,
                    tb,
                    LOW,
                    gs,
                    Bytes::Random,
                );
                c.limit = LimitSpec::Abs(i32::MAX);
                c.pos = rng.range(0, 15) as i32;
                t.run(&c);
            }
        }
    }
    t.finish("phase_b_row24_full_choff_walk", 1);
}

/// Row 25 -- grand fuzz: every argument and both structs randomized together.
#[test]
fn phase_b_row25_grand_fuzz() {
    let mut t = Tally::default();
    let mut rng = Rng::new(0x5EED_1234 ^ 25);
    let gs_pool: [i32; 14] = [-8, -1, 0, 1, 2, 3, 4, 5, 8, 12, 16, 18, 24, 32];
    for it in 0..3000 {
        let tb = rng.range(0, 255) as u8;
        let gs = rng.pick(&gs_pool);
        let classes: &[(u8, u8)] = rng.pick(&[LOW, LOW_NARROW, LOW_WIDE, HIGH_SMALL, MIXED, ANY]);
        let bytes = rng.pick(&[Bytes::Zeros, Bytes::Ones, Bytes::Aa55, Bytes::Random]);
        let mut c = base_case(
            format!("row25 fuzz it={it} tb={tb} gs={gs}"),
            &mut rng,
            tb,
            classes,
            gs,
            bytes,
        );
        c.pos = match rng.range(0, 3) {
            0 => 0,
            1 => rng.range(1, 63) as i32,
            2 => rng.range(-4096, -1) as i32,
            _ => rng.range(0, 1 << 16) as i32,
        };
        c.buf_spec = if c.pos < 0 {
            BufSpec::Rebased
        } else {
            BufSpec::Raw
        };
        c.limit = match rng.range(0, 4) {
            0 => LimitSpec::RelBits(MAX_BITS),
            1 => LimitSpec::Abs(consumed_bits(&c.sci, gs).min(MAX_BITS as i64) as i32),
            2 => LimitSpec::Abs(rng.range(0, consumed_bits(&c.sci, gs).min(MAX_BITS as i64).max(1)) as i32),
            3 => LimitSpec::Abs(0),
            _ => LimitSpec::Abs(-(rng.range(1, 1000) as i32)),
        };
        t.run(&c);
        // Always-observable companion (nothing is read: every get_bits rejects).
        let mut starved = c.clone();
        starved.label = format!("row25 fuzz-starved it={it} tb={tb} gs={gs}");
        starved.limit = LimitSpec::Abs(i32::MIN);
        t.run(&starved);
    }
    t.finish("phase_b_row25_grand_fuzz", 1);
}

// ===========================================================================
// Phase C -- ERRORS.md rows E1..E24
// ===========================================================================

fn half_of(ba: u8) -> i32 {
    (1i32 << (ba as i32 - 1)) - 1
}

fn gr_at(o: &Outcome, idx: isize) -> f32 {
    f32::from_bits(o.gr[(GR_PAD as isize + idx) as usize])
}

/// One band active (`bitalloc[0] = ba`, `bitalloc[1] = 0`), nothing else.
fn one_band_case(label: String, rng: &mut Rng, ba: u8, gs: i32, limit: LimitSpec) -> Case {
    let mut c = base_case(label, rng, 1, &[(0, 0)], gs, Bytes::Random);
    sci_set_ba(&mut c.sci, 0, ba);
    sci_set_ba(&mut c.sci, 1, 0);
    c.limit = limit;
    c
}

/// E1 -- reservoir exhausted: get_bits returns 0 but still advances `bs->pos`.
#[test]
fn phase_c_e01_reject_returns_zero_but_advances_pos() {
    let mut t = Tally::default();
    let mut rng = Rng::new(0xE001);
    for ba in 1u8..=16 {
        for &gs in &[1i32, 4, 12] {
            let c = one_band_case(
                format!("E1 ba={ba} gs={gs}"),
                &mut rng,
                ba,
                gs,
                LimitSpec::Abs(0),
            );
            let half = half_of(ba);
            t.run_with(&c, |o| {
                // every one of the 4*gs get_bits calls rejected, yet pos moved
                assert_eq!(o.pos, 4 * gs * ba as i32, "E1 pos must advance on reject");
                for j in 0..4i32 {
                    for k in 0..gs {
                        assert_eq!(
                            gr_at(o, (gs * j + k) as isize),
                            -(half as f32),
                            "E1 rejected field must be 0-half"
                        );
                    }
                }
            });
        }
    }
    t.finish("phase_c_e01_reject_returns_zero_but_advances_pos", 1);
}

/// E2 -- boundary `pos + n == limit` is NOT rejected (`>` not `>=`).
#[test]
fn phase_c_e02_exact_limit_not_rejected() {
    let mut t = Tally::default();
    let mut rng = Rng::new(0xE002);
    for &tb in &[1u8, 2, 8, 32] {
        for &gs in &[1i32, 4, 12] {
            // all-ones reservoir => an accepted read can never coincide with the
            // rejected value (0 - half), so "did it reject?" is decidable.
            let mut c = base_case(
                format!("E2 tb={tb} gs={gs}"),
                &mut rng,
                tb,
                LOW,
                gs,
                Bytes::Ones,
            );
            let need = consumed_bits(&c.sci, gs) as i32;
            c.limit = LimitSpec::Abs(need);
            assert!(observable(&c));
            let exact = run_one(impls().c, &c);
            assert_eq!(exact.pos, need, "E2: pos+n == limit must be accepted");
            // Same case but with an empty reservoir: everything rejects.
            let mut starved = c.clone();
            starved.limit = LimitSpec::Abs(0);
            let starved_o = run_one(impls().c, &starved);
            assert_ne!(
                exact.gr, starved_o.gr,
                "E2: exact-fit run must actually read bits (differs from all-rejected run)"
            );
            t.run(&c);
            t.run(&starved);
        }
    }
    t.finish("phase_c_e02_exact_limit_not_rejected", 1);
}

/// E3 -- one bit short: the final field rejects, `pos` is unchanged from E2.
#[test]
fn phase_c_e03_one_bit_short_rejects() {
    let mut t = Tally::default();
    let mut rng = Rng::new(0xE003);
    for &tb in &[1u8, 2, 8, 32] {
        for &gs in &[1i32, 4, 12] {
            let mut fit = base_case(
                format!("E3 tb={tb} gs={gs}"),
                &mut rng,
                tb,
                LOW,
                gs,
                Bytes::Ones,
            );
            let need = consumed_bits(&fit.sci, gs) as i32;
            if need == 0 {
                continue;
            }
            fit.limit = LimitSpec::Abs(need);
            let mut short = fit.clone();
            short.limit = LimitSpec::Abs(need - 1);
            assert!(observable(&fit) && observable(&short));
            let a = run_one(impls().c, &fit);
            let b = run_one(impls().c, &short);
            assert_eq!(a.pos, b.pos, "E3: pos advances identically on reject");
            assert_ne!(a.gr, b.gr, "E3: the last field must actually be rejected");
            // ...and the rejected field is exactly `0 - half` of the last band.
            let i_last = 2 * tb as i32 - 1;
            let o_last = 18 * (i_last / 2) + if i_last % 2 == 1 { 576 } else { 0 };
            let idx = (gs * 3 + o_last + gs - 1) as isize;
            let ba_last = sci_ba(&fit.sci, i_last as usize);
            assert_eq!(
                gr_at(&b, idx),
                -(half_of(ba_last) as f32),
                "E3: last field must yield 0-half"
            );
            t.run(&fit);
            t.run(&short);
        }
    }
    t.finish("phase_c_e03_one_bit_short_rejects", 1);
}

/// E4 -- negative limit: every get_bits rejects.
#[test]
fn phase_c_e04_negative_limit_rejects_everything() {
    let mut t = Tally::default();
    let mut rng = Rng::new(0xE004);
    for &lim in &[-1i32, -1000, i32::MIN] {
        for ba in [1u8, 5, 16] {
            let c = one_band_case(
                format!("E4 lim={lim} ba={ba}"),
                &mut rng,
                ba,
                4,
                LimitSpec::Abs(lim),
            );
            let half = half_of(ba);
            t.run_with(&c, |o| {
                for j in 0..4i32 {
                    for k in 0..4i32 {
                        assert_eq!(gr_at(o, (4 * j + k) as isize), -(half as f32));
                    }
                }
            });
        }
    }
    t.finish("phase_c_e04_negative_limit_rejects_everything", 1);
}

/// E5 -- limit == INT_MAX never rejects.
#[test]
fn phase_c_e05_intmax_limit_never_rejects() {
    let mut t = Tally::default();
    let mut rng = Rng::new(0xE005);
    for &tb in &[1u8, 8, 32] {
        for &gs in &[1i32, 4, 12] {
            let mut c = base_case(
                format!("E5 tb={tb} gs={gs}"),
                &mut rng,
                tb,
                LOW,
                gs,
                Bytes::Random,
            );
            c.limit = LimitSpec::Abs(i32::MAX);
            let need = consumed_bits(&c.sci, gs) as i32;
            t.run_with(&c, |o| {
                assert_eq!(o.pos, need, "E5: all reads accepted");
            });
        }
    }
    t.finish("phase_c_e05_intmax_limit_never_rejects", 1);
}

/// E6 -- negative `bs->pos`: `p = buf + (pos >> 3)` (arithmetic shift).
/// A rebased `pos = -8k` reads the same byte stream as `pos = 0`.
#[test]
fn phase_c_e06_negative_pos_reads_below_buf() {
    let mut t = Tally::default();
    let mut rng = Rng::new(0xE006);
    for &pos in &[-8i32, -64, -800, -4096] {
        let mut c = base_case(format!("E6 pos={pos}"), &mut rng, 8, LOW, 12, Bytes::Random);
        c.pos = pos;
        c.buf_spec = BufSpec::Rebased;
        assert!(observable(&c));
        let neg = run_one(impls().c, &c);

        let mut zero = c.clone();
        zero.pos = 0;
        zero.buf_spec = BufSpec::Raw;
        assert!(observable(&zero));
        let z = run_one(impls().c, &zero);
        assert_eq!(
            neg.gr, z.gr,
            "E6: pos={pos} rebased must read the same bytes as pos=0"
        );
        t.run(&c);
    }
    // ...and a non-multiple-of-8 negative pos (s != 0, p still below buf).
    for &pos in &[-1i32, -3, -7, -9, -63] {
        let mut c = base_case(format!("E6 pos={pos}"), &mut rng, 8, MIXED, 12, Bytes::Random);
        c.pos = pos;
        c.buf_spec = BufSpec::Rebased;
        t.run(&c);
    }
    t.finish("phase_c_e06_negative_pos_reads_below_buf", 1);
}

/// E7 -- `bs->pos += n` signed overflow ⇒ the limit check spuriously passes.
#[test]
fn phase_c_e07_pos_overflow_passes_limit_check() {
    let mut t = Tally::default();
    let mut rng = Rng::new(0xE007);
    for ba in [1u8, 4, 9, 16] {
        let half = half_of(ba);

        // (a) pos = INT_MAX: call #1 already wraps. limit == the wrapped value,
        //     so #1 is accepted and #2..#4 reject.
        let pos = i32::MAX;
        let mut a = one_band_case(
            format!("E7a pos=INT_MAX ba={ba}"),
            &mut rng,
            ba,
            1,
            LimitSpec::Abs(pos.wrapping_add(ba as i32)),
        );
        a.pos = pos;
        a.buf_spec = BufSpec::Rebased;
        t.run_with(&a, |o| {
            // A *negative* wrapped pos is <= limit, so call #1 was ACCEPTED
            // even though the reservoir is nominally long exhausted.
            assert_eq!(o.pos, pos.wrapping_add(4 * ba as i32));
            assert!(o.pos < 0, "E7a: pos must have wrapped negative");
            // Calls #2..#4 exceed the limit and yield 0 - half.
            for j in 1..4isize {
                assert_eq!(gr_at(o, j), -(half as f32), "E7a later calls must reject");
            }
        });

        // (b) pos = INT_MAX - 3n, limit = INT_MAX: the 4th (last) call wraps and
        //     is accepted, so NO field is ever rejected despite pos > limit.
        let mut b = a.clone();
        b.label = format!("E7b pos=INT_MAX-3n ba={ba}");
        b.pos = i32::MAX - 3 * ba as i32;
        b.limit = LimitSpec::Abs(i32::MAX);
        b.buf = make_buf(&mut rng, Bytes::Ones);
        t.run_with(&b, |o| {
            assert_eq!(o.pos, (i32::MAX - 3 * ba as i32).wrapping_add(4 * ba as i32));
            assert!(o.pos < 0, "E7b: pos must have wrapped negative");
            // 0xFF reservoir => an accepted read yields (2^ba - 1) - half > 0,
            // so a rejected one (0 - half <= 0) is distinguishable. All four
            // granules must show the accepted value.
            for j in 0..4isize {
                assert_eq!(
                    gr_at(o, j),
                    ((1i32 << ba) - 1 - half) as f32,
                    "E7b: overflowing call must still be ACCEPTED"
                );
            }
        });
    }
    t.finish("phase_c_e07_pos_overflow_passes_limit_check", 1);
}

/// E8 -- single-byte path: `n + s <= 8`, the `while` body never runs.
/// (`n <= 0` is unreachable through the public API: `n` is either `ba >= 1`
/// or `mod + 2 - (mod>>3) >= 3`.)
#[test]
fn phase_c_e08_single_byte_path() {
    let mut t = Tally::default();
    let mut rng = Rng::new(0xE008);
    for ba in 1u8..=8 {
        for s in 0..=(8 - ba as i32) {
            let mut c = one_band_case(
                format!("E8 ba={ba} s={s}"),
                &mut rng,
                ba,
                1,
                LimitSpec::Abs(i32::MAX),
            );
            c.pos = s;
            c.buf_spec = BufSpec::Raw;
            let half = half_of(ba);
            t.run_with(&c, |o| {
                let b = c.buf[BUF_PAD] as u32;
                let sm = (s & 7) as u32;
                let expect = (((b & (255u32 >> sm)) >> (8 - (ba as u32 + sm))) as i32 - half) as f32;
                assert_eq!(gr_at(o, 0), expect, "E8 single-byte extraction");
            });
        }
    }
    t.finish("phase_c_e08_single_byte_path", 1);
}

/// E9 -- accepted reads with `n >= 32` (shift counts UB-masked to `& 31`).
#[test]
fn phase_c_e09_large_n_shift_masking() {
    let mut t = Tally::default();
    let mut rng = Rng::new(0xE009);
    // ba in 17..=37 => n = mod+2-(mod>>3) up to 1_835_011 bits, still <= MAX_BITS
    // so the read is ACCEPTED and the >=32 shift counts really happen.
    for ba in 17u8..=37 {
        for &gs in &[1i32, 3, 12] {
            let c = one_band_case(
                format!("E9 ba={ba} gs={gs}"),
                &mut rng,
                ba,
                gs,
                LimitSpec::RelBits(MAX_BITS),
            );
            let m = 2u32.wrapping_shl(ba as u32 - 17).wrapping_add(1);
            let n = m.wrapping_add(2).wrapping_sub(m >> 3) as i32;
            t.run_with(&c, |o| {
                if n <= MAX_BITS {
                    assert!(o.pos >= n, "E9 ba={ba}: first {n}-bit read must be accepted");
                }
            });
        }
    }
    t.finish("phase_c_e09_large_n_shift_masking", 1);
}

/// E10 -- `bitalloc[i] == 0`: no bits consumed, nothing written.
#[test]
fn phase_c_e10_zero_bitalloc_skips_band() {
    let mut t = Tally::default();
    let mut rng = Rng::new(0xE010);
    for &tb in &[1u8, 2, 8, 32] {
        for &gs in &[1i32, 4, 18] {
            let mut c = base_case(
                format!("E10 tb={tb} gs={gs}"),
                &mut rng,
                tb,
                &[(0, 0)],
                gs,
                Bytes::Random,
            );
            fill_bitalloc_span(&mut rng, &mut c.sci, &[(0, 0)], 512);
            c.limit = LimitSpec::Abs(i32::MAX);
            let pristine = fresh_gr();
            t.run_with(&c, |o| {
                assert_eq!(o.pos, 0, "E10: no bits consumed");
                assert!(o.gr == pristine, "E10: nothing may be written");
                assert_eq!(o.ret, gs * 4);
            });
        }
    }
    t.finish("phase_c_e10_zero_bitalloc_skips_band", 1);
}

/// E11 -- `total_bands == 0`: loop never runs.
#[test]
fn phase_c_e11_zero_total_bands() {
    let mut t = Tally::default();
    let mut rng = Rng::new(0xE011);
    for &gs in &[0i32, 1, 4, 12, 18, 32] {
        let mut c = base_case(format!("E11 gs={gs}"), &mut rng, 0, ANY, gs, Bytes::Random);
        c.limit = LimitSpec::Abs(i32::MAX);
        let pristine = fresh_gr();
        t.run_with(&c, |o| {
            assert_eq!(o.pos, 0);
            assert!(o.gr == pristine);
            assert_eq!(o.ret, gs * 4);
        });
    }
    t.finish("phase_c_e11_zero_total_bands", 1);
}

/// E12 -- `group_size <= 0`: no writes, but `ba >= 17` bands still consume one field.
#[test]
fn phase_c_e12_non_positive_group_size() {
    let mut t = Tally::default();
    let mut rng = Rng::new(0xE012);
    for &gs in &[0i32, -1, -7, i32::MIN] {
        // low-only: nothing is consumed at all
        let mut lo = base_case(format!("E12 low gs={gs}"), &mut rng, 8, LOW, gs, Bytes::Random);
        lo.limit = LimitSpec::Abs(i32::MAX);
        let pristine = fresh_gr();
        t.run_with(&lo, |o| {
            assert_eq!(o.pos, 0, "E12: ba<17 consumes nothing when group_size<=0");
            assert!(o.gr == pristine);
            assert_eq!(o.ret, gs.wrapping_mul(4));
        });

        // high-only: one get_bits per band still happens, before the k-loop
        let hi = base_case(
            format!("E12 high gs={gs}"),
            &mut rng,
            8,
            HIGH_SMALL,
            gs,
            Bytes::Random,
        );
        let want = consumed_bits(&hi.sci, gs) as i32;
        assert!(want > 0);
        let pristine2 = fresh_gr();
        t.run_with(&hi, |o| {
            assert_eq!(o.pos, want, "E12: ba>=17 consumes one field even with no writes");
            assert!(o.gr == pristine2, "E12: still no writes");
        });
    }
    t.finish("phase_c_e12_non_positive_group_size", 1);
}

/// E13 -- `total_bands > 32`: `bitalloc[i]` reads `scfcod` / past the struct.
#[test]
fn phase_c_e13_bitalloc_index_out_of_bounds() {
    let mut t = Tally::default();
    let mut rng = Rng::new(0xE013);
    for &tb in &[33u8, 40, 64, 100, 255] {
        // (a) whole reachable span zeroed -> nothing written
        let mut zeroed = base_case(
            format!("E13 zeroed tb={tb}"),
            &mut rng,
            tb,
            &[(0, 0)],
            4,
            Bytes::Random,
        );
        fill_bitalloc_span(&mut rng, &mut zeroed.sci, &[(0, 0)], 2 * tb as usize);
        zeroed.limit = LimitSpec::Abs(i32::MAX);
        let pristine = fresh_gr();
        t.run_with(&zeroed, |o| {
            assert_eq!(o.pos, 0);
            assert!(o.gr == pristine, "E13a: zeroed OOB span must write nothing");
        });

        // (b) ONLY the out-of-array part is non-zero -> C must still act on it
        let mut oob = zeroed.clone();
        oob.label = format!("E13 oob-only tb={tb}");
        let span = 2 * tb as usize;
        assert!(span > 64);
        fill_bitalloc_span(&mut rng, &mut oob.sci, &[(0, 0)], 64); // in-array: all zero
        for i in 64..span {
            // write low bitallocs into indices 64.. (scfcod / padding / slack)
            sci_set_ba(&mut oob.sci, i, 1 + (i as u8 % 16));
        }
        t.run_with(&oob, |o| {
            assert!(o.pos > 0, "E13b: OOB bitalloc values must be honoured");
            assert!(o.gr != pristine, "E13b: OOB bitalloc values must produce writes");
        });
    }
    t.finish("phase_c_e13_bitalloc_index_out_of_bounds", 1);
}

/// E14 -- `2 << (ba - 17)` with a shift count >= 32 (masked to `& 31`).
#[test]
fn phase_c_e14_mod_shift_count_over_31() {
    let mut t = Tally::default();
    let mut rng = Rng::new(0xE014);
    for ba in [49u8, 50, 63, 64, 80, 100, 127, 128, 200, 254, 255] {
        for &gs in &[1i32, 4, 12] {
            let c = one_band_case(
                format!("E14 ba={ba} gs={gs}"),
                &mut rng,
                ba,
                gs,
                LimitSpec::RelBits(MAX_BITS),
            );
            t.run(&c);
            // and with an empty reservoir (code == 0)
            let mut starved = c.clone();
            starved.limit = LimitSpec::Abs(0);
            let m = 2u32.wrapping_shl(ba as u32 - 17).wrapping_add(1);
            let expect = (0u32.wrapping_sub(m / 2) as i32) as f32;
            t.run_with(&starved, |o| {
                for k in 0..gs {
                    assert_eq!(gr_at(o, k as isize), expect, "E14 ba={ba}");
                }
            });
        }
    }
    t.finish("phase_c_e14_mod_shift_count_over_31", 1);
}

/// E15 -- `ba == 48` ⇒ `2 << 31 == 0` ⇒ `mod == 1` ⇒ every value is 0.0f
/// (and `mod` is never 0, so `% mod` / `/= mod` never divide by zero).
#[test]
fn phase_c_e15_mod_equals_one() {
    let mut t = Tally::default();
    let mut rng = Rng::new(0xE015);
    for &gs in &[1i32, 2, 4, 12, 18] {
        for &lim in &[LimitSpec::Abs(0), LimitSpec::RelBits(MAX_BITS), LimitSpec::Abs(i32::MAX)] {
            let c = one_band_case(format!("E15 gs={gs} lim={lim:?}"), &mut rng, 48, gs, lim);
            t.run_with(&c, |o| {
                for j in 0..4i32 {
                    for k in 0..gs {
                        assert_eq!(
                            gr_at(o, (gs * j + k) as isize).to_bits(),
                            0u32,
                            "E15: mod==1 => dst[k] == +0.0f"
                        );
                    }
                }
            });
        }
    }
    t.finish("phase_c_e15_mod_equals_one", 1);
}

/// E16 -- boundary `ba == 16`: largest value on the `ba < 17` branch.
#[test]
fn phase_c_e16_ba_16_boundary() {
    let mut t = Tally::default();
    let mut rng = Rng::new(0xE016);
    for &gs in &[1i32, 4, 12] {
        let c = one_band_case(format!("E16 gs={gs}"), &mut rng, 16, gs, LimitSpec::Abs(i32::MAX));
        t.run_with(&c, |o| {
            assert_eq!(o.pos, 4 * gs * 16);
            for j in 0..4i32 {
                for k in 0..gs {
                    let v = gr_at(o, (gs * j + k) as isize);
                    assert!((-32767.0..=32768.0).contains(&v), "E16 half=32767, got {v}");
                }
            }
        });
    }
    t.finish("phase_c_e16_ba_16_boundary", 1);
}

/// E17 -- boundary `ba == 17`: smallest value on the `else` branch (mod 3, n 5).
#[test]
fn phase_c_e17_ba_17_boundary() {
    let mut t = Tally::default();
    let mut rng = Rng::new(0xE017);
    for &gs in &[1i32, 2, 3, 4, 12] {
        let c = one_band_case(format!("E17 gs={gs}"), &mut rng, 17, gs, LimitSpec::Abs(i32::MAX));
        t.run_with(&c, |o| {
            assert_eq!(o.pos, 4 * 5, "E17: one 5-bit read per granule");
            for j in 0..4i32 {
                for k in 0..gs {
                    let v = gr_at(o, (gs * j + k) as isize);
                    assert!(v == -1.0 || v == 0.0 || v == 1.0, "E17 mod=3, got {v}");
                }
            }
        });
    }
    t.finish("phase_c_e17_ba_17_boundary", 1);
}

/// E18 -- boundary `ba == 1`: `half == 0`, raw 1-bit codes.
#[test]
fn phase_c_e18_ba_1_boundary() {
    let mut t = Tally::default();
    let mut rng = Rng::new(0xE018);
    for &gs in &[1i32, 4, 12] {
        let c = one_band_case(format!("E18 gs={gs}"), &mut rng, 1, gs, LimitSpec::Abs(i32::MAX));
        t.run_with(&c, |o| {
            assert_eq!(o.pos, 4 * gs);
            for j in 0..4i32 {
                for k in 0..gs {
                    let v = gr_at(o, (gs * j + k) as isize);
                    assert!(v == 0.0 || v == 1.0, "E18 half=0, got {v}");
                }
            }
        });
    }
    t.finish("phase_c_e18_ba_1_boundary", 1);
}

/// E19 -- `ba == 255`: mod = 32769, n = 28675.
#[test]
fn phase_c_e19_ba_255() {
    let mut t = Tally::default();
    let mut rng = Rng::new(0xE019);
    for &gs in &[1i32, 4, 12] {
        // small reservoir -> rejected -> code == 0 -> every value is -16384
        let starved = one_band_case(format!("E19 starved gs={gs}"), &mut rng, 255, gs, LimitSpec::Abs(0));
        t.run_with(&starved, |o| {
            assert_eq!(o.pos, 4 * 28675, "E19: pos advances by n even when rejected");
            for j in 0..4i32 {
                for k in 0..gs {
                    assert_eq!(gr_at(o, (gs * j + k) as isize), -16384.0, "E19 -mod/2");
                }
            }
        });
        // large reservoir -> the 28675-bit read is accepted
        let fed = one_band_case(
            format!("E19 fed gs={gs}"),
            &mut rng,
            255,
            gs,
            LimitSpec::RelBits(MAX_BITS),
        );
        t.run(&fed);
    }
    t.finish("phase_c_e19_ba_255", 1);
}

/// E20 -- `choff` drift writes far past `grbuf` with no bounds check.
#[test]
fn phase_c_e20_choff_drift_writes_past_grbuf() {
    let mut t = Tally::default();
    let mut rng = Rng::new(0xE020);
    for &tb in &[2u8, 8, 64, 255] {
        let mut c = base_case(format!("E20 tb={tb}"), &mut rng, tb, LOW, 1, Bytes::Random);
        c.limit = LimitSpec::Abs(i32::MAX);
        let pristine = fresh_gr();
        t.run_with(&c, |o| {
            // band i=1 is written at grbuf + 576 (choff), far outside a 576-float granule
            assert_ne!(o.gr[GR_PAD + 576], pristine[GR_PAD + 576], "E20: write at +576");
            // nothing below grbuf is ever touched (2*total_bands is even)
            assert_eq!(&o.gr[..GR_PAD], &pristine[..GR_PAD], "E20: no underflow writes");
        });
    }
    t.finish("phase_c_e20_choff_drift_writes_past_grbuf", 1);
}

/// E21 -- `group_size * 4` (and `group_size * j`) signed overflow.
#[test]
fn phase_c_e21_return_value_overflow() {
    let mut t = Tally::default();
    let mut rng = Rng::new(0xE021);
    for &gs in &[
        0x4000_0000i32,
        0x4000_0001,
        i32::MAX,
        i32::MAX - 1,
        0x7FFF_FFF0,
        0x2000_0000,
        i32::MIN,
        i32::MIN + 1,
    ] {
        let c = base_case(format!("E21 gs={gs}"), &mut rng, 0, ANY, gs, Bytes::Random);
        let want = gs.wrapping_mul(4);
        t.run_with(&c, |o| assert_eq!(o.ret, want, "E21 group_size*4 must wrap"));
    }
    t.finish("phase_c_e21_return_value_overflow", 1);
}

/// E22 -- no null checks: the reachable-without-deref NULL cases must agree.
#[test]
fn phase_c_e22_null_grbuf_without_writes() {
    let mut t = Tally::default();
    let mut rng = Rng::new(0xE022);
    // (a) total_bands == 0 -> grbuf is never dereferenced
    for &gs in &[0i32, 1, 12, 0x4000_0000] {
        let mut c = base_case(format!("E22a gs={gs}"), &mut rng, 0, ANY, gs, Bytes::Random);
        c.null_grbuf = true;
        t.run_with(&c, |o| assert_eq!(o.ret, gs.wrapping_mul(4)));
    }
    // (b) every band skipped -> still no writes
    for &tb in &[1u8, 8, 64] {
        let mut c = base_case(format!("E22b tb={tb}"), &mut rng, tb, &[(0, 0)], 12, Bytes::Random);
        fill_bitalloc_span(&mut rng, &mut c.sci, &[(0, 0)], 512);
        c.null_grbuf = true;
        t.run_with(&c, |o| {
            assert_eq!(o.ret, 48);
            assert_eq!(o.pos, 0);
        });
    }
    // (c) group_size <= 0 with low bands -> no writes either
    for &gs in &[0i32, -1, i32::MIN] {
        let mut c = base_case(format!("E22c gs={gs}"), &mut rng, 8, LOW, gs, Bytes::Random);
        c.null_grbuf = true;
        t.run_with(&c, |o| assert_eq!(o.ret, gs.wrapping_mul(4)));
    }
    t.finish("phase_c_e22_null_grbuf_without_writes", 1);
}

/// E23 -- `bs->buf == NULL` is never dereferenced when the limit check rejects first.
#[test]
fn phase_c_e23_null_buf_with_rejecting_limit() {
    let mut t = Tally::default();
    let mut rng = Rng::new(0xE023);
    for &lim in &[-1i32, i32::MIN, 0] {
        for ba in [1u8, 7, 16, 17, 48, 255] {
            let mut c = one_band_case(
                format!("E23 lim={lim} ba={ba}"),
                &mut rng,
                ba,
                4,
                LimitSpec::Abs(lim),
            );
            c.buf_spec = BufSpec::Null;
            let expect = if ba < 17 {
                -(half_of(ba) as f32)
            } else {
                let m = 2u32.wrapping_shl(ba as u32 - 17).wrapping_add(1);
                (0u32.wrapping_sub(m / 2) as i32) as f32
            };
            t.run_with(&c, |o| {
                for j in 0..4i32 {
                    for k in 0..4i32 {
                        assert_eq!(gr_at(o, (4 * j + k) as isize), expect, "E23 ba={ba}");
                    }
                }
            });
        }
    }
    // limit == 0 with a 0-length first read is impossible (n >= 1), so buf is
    // never touched for lim < 1 either -- covered above.
    t.finish("phase_c_e23_null_buf_with_rejecting_limit", 1);
}

/// E24 -- sweep the whole `uint8_t` bitalloc "opcode" domain, 0..=255.
#[test]
fn phase_c_e24_full_bitalloc_domain_sweep() {
    let mut t = Tally::default();
    let mut rng = Rng::new(0xE024);
    for ba in 0u8..=255 {
        for &gs in &[1i32, 4, 12] {
            // bounded reservoir keeps the huge `n` values in-bounds
            let c = one_band_case(
                format!("E24 ba={ba} gs={gs}"),
                &mut rng,
                ba,
                gs,
                LimitSpec::RelBits(MAX_BITS),
            );
            t.run(&c);
            let mut starved = c.clone();
            starved.limit = LimitSpec::Abs(0);
            t.run(&starved);
        }
    }
    // ...and with every band set to the same value, across all four granules.
    for ba in 0u8..=255 {
        let mut c = base_case(format!("E24 all ba={ba}"), &mut rng, 8, &[(0, 0)], 4, Bytes::Random);
        fill_bitalloc_span(&mut rng, &mut c.sci, &[(ba, ba)], 512);
        c.limit = LimitSpec::RelBits(MAX_BITS);
        t.run(&c);
    }
    t.finish("phase_c_e24_full_bitalloc_domain_sweep", 1);
}

