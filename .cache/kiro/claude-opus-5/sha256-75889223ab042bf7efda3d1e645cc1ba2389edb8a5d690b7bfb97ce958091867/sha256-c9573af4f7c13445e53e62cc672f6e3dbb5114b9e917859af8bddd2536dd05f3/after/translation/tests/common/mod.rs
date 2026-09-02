//! Shared differential-test harness.
//!
//! Both the C `.so` and the Rust `.so` are loaded with `libloading` and driven
//! **only** through their exported `normalize` symbol — the Rust crate is never
//! linked directly, so the `#[unsafe(no_mangle)] extern "C"` wrapper is part of
//! what is under test.

#![allow(dead_code)]

use std::path::PathBuf;
use std::sync::OnceLock;

use libloading::{Library, Symbol};

/// ABI of the single exported entry point (`c_src/include/lib.h`).
pub type NormalizeFn = unsafe extern "C" fn(*mut f32, *const f32, i32);

// ---------------------------------------------------------------------------
// locating and loading the two shared objects
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `<workdir>/c_src/build/lib<project>.so`. The CMake project name is derived
/// from the parent directory name, so the file is discovered rather than
/// hard-coded.
fn c_so_path() -> PathBuf {
    let dir = manifest_dir().parent().unwrap().join("c_src/build");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e} — build the C library first", dir.display()))
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.extension().map(|x| x == "so").unwrap_or(false)
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("lib"))
                    .unwrap_or(false)
        })
        .collect();
    found.sort();
    assert_eq!(found.len(), 1, "expected exactly one .so in {}, got {found:?}", dir.display());
    found.pop().unwrap()
}

/// The cdylib produced alongside this test binary. `current_exe()` is
/// `target/<profile>/deps/<test>-<hash>`, so the profile directory is two
/// levels up — this keeps debug and release runs pointed at their own artifact.
///
/// `cargo test` does **not** build `crate-type = ["cdylib"]` artifacts, so if
/// the `.so` is missing or older than `src/lib.rs` the harness builds it via a
/// nested `cargo build` before loading it. That keeps a bare `cargo test` from
/// silently testing a stale library.
fn rust_so_path() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir: PathBuf = exe.parent().unwrap().parent().unwrap().to_path_buf();
    let p = profile_dir.join("libnormalize_lib.so");

    let src = manifest_dir().join("src/lib.rs");
    let stale = match (std::fs::metadata(&p), std::fs::metadata(&src)) {
        (Ok(a), Ok(b)) => match (a.modified(), b.modified()) {
            (Ok(a), Ok(b)) => a < b,
            _ => true,
        },
        _ => true,
    };

    if stale {
        let release = profile_dir.file_name().map(|n| n == "release").unwrap_or(false);
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
        let mut cmd = std::process::Command::new(cargo);
        cmd.arg("build").arg("--lib").current_dir(manifest_dir());
        if release {
            cmd.arg("--release");
        }
        let out = cmd.output().expect("spawn nested `cargo build --lib`");
        assert!(
            out.status.success(),
            "nested `cargo build --lib` failed:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }

    assert!(
        p.is_file(),
        "missing Rust cdylib at {} — run `cargo build` (and `cargo build --release`) first",
        p.display()
    );
    p
}

pub struct Libs {
    c: Library,
    r: Library,
    pub c_path: PathBuf,
    pub r_path: PathBuf,
}

impl Libs {
    pub fn c(&self) -> NormalizeFn {
        let s: Symbol<NormalizeFn> =
            unsafe { self.c.get(b"normalize\0") }.expect("`normalize` not exported by the C .so");
        *s
    }
    pub fn rust(&self) -> NormalizeFn {
        let s: Symbol<NormalizeFn> = unsafe { self.r.get(b"normalize\0") }
            .expect("`normalize` not exported by the Rust .so");
        *s
    }
}

static LIBS: OnceLock<Libs> = OnceLock::new();

pub fn libs() -> &'static Libs {
    LIBS.get_or_init(|| {
        let c_path = c_so_path();
        let r_path = rust_so_path();
        let c = unsafe { Library::new(&c_path) }.expect("dlopen C .so");
        let r = unsafe { Library::new(&r_path) }.expect("dlopen Rust .so");
        Libs { c, r, c_path, r_path }
    })
}

// ---------------------------------------------------------------------------
// deterministic PRNG (xorshift64* — fixed seed, reproducible)
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x9E37_79B9_7F4A_7C15;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(if seed == 0 { SEED } else { seed })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    /// Uniform in `0..n` (n > 0).
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
    /// Uniform in `lo..=hi`.
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        lo + (self.next_u64() % ((hi - lo + 1) as u64)) as i32
    }
    pub fn bool_1_in(&mut self, n: usize) -> bool {
        self.below(n) == 0
    }
    /// Uniform in `[-1, 1)`.
    pub fn unit(&mut self) -> f32 {
        let m = (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32; // [0,1)
        if self.next_u64() & 1 == 0 { m } else { -m }
    }
    pub fn sign(&mut self) -> f32 {
        if self.next_u64() & 1 == 0 { 1.0 } else { -1.0 }
    }
}

// ---------------------------------------------------------------------------
// element populations (axis D of CONFIGS.md)
// ---------------------------------------------------------------------------

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Pop {
    /// Uniform in [-1, 1).
    Uniform,
    /// sign * 2^e * m, e in [-60, 60].
    WideExp,
    /// Exact powers of two, sign * 2^e, e in [-20, 20].
    PowersOfTwo,
    /// Random mix of +0.0 and -0.0.
    Zeros,
    /// Denormals only (`sum` will underflow to +0.0).
    Denormal,
    /// ~1e-25 .. 1e-30: non-zero, but every square rounds to +0.0.
    TinyUnderflow,
    /// One or two elements ~1e-22 so that `sum` itself is denormal.
    DenormalSum,
    /// Mix of denormals and normals.
    MixedDenormalNormal,
    /// Near FLT_MAX, so `sum` overflows to +inf.
    NearMax,
    /// Mostly finite, with ±inf sprinkled in.
    InfSprinkle,
    /// Mostly finite, with random-payload NaNs sprinkled in.
    NanSprinkle,
    /// Completely arbitrary 32-bit patterns reinterpreted as f32.
    RandomBits,
}

fn denormal(rng: &mut Rng) -> f32 {
    // Non-zero mantissa, zero exponent field => denormal.
    let m = (rng.next_u32() & 0x007F_FFFF).max(1);
    let s = (rng.next_u32() & 1) << 31;
    f32::from_bits(s | m)
}

fn nan(rng: &mut Rng) -> f32 {
    // Random payload, random sign, random quiet/signalling bit.
    let payload = (rng.next_u32() & 0x007F_FFFF).max(1);
    let s = (rng.next_u32() & 1) << 31;
    f32::from_bits(s | 0x7F80_0000 | payload)
}

pub fn gen_elem(rng: &mut Rng, pop: Pop) -> f32 {
    match pop {
        Pop::Uniform => rng.unit(),
        Pop::WideExp => {
            let e = rng.range_i32(-60, 60);
            rng.sign() * (rng.unit().abs() + 0.5) * 2.0f32.powi(e)
        }
        Pop::PowersOfTwo => rng.sign() * 2.0f32.powi(rng.range_i32(-20, 20)),
        Pop::Zeros => {
            if rng.next_u64() & 1 == 0 {
                0.0
            } else {
                -0.0
            }
        }
        Pop::Denormal => denormal(rng),
        Pop::TinyUnderflow => rng.sign() * (rng.unit().abs() + 0.5) * 2.0f32.powi(rng.range_i32(-95, -85)),
        Pop::DenormalSum => rng.sign() * (rng.unit().abs() + 0.5) * 2.0f32.powi(rng.range_i32(-74, -72)),
        Pop::MixedDenormalNormal => {
            if rng.next_u64() & 1 == 0 {
                denormal(rng)
            } else {
                rng.sign() * (rng.unit().abs() + 0.5) * 2.0f32.powi(rng.range_i32(-10, 10))
            }
        }
        Pop::NearMax => rng.sign() * (rng.unit().abs() + 0.5) * 2.0f32.powi(rng.range_i32(100, 127)),
        Pop::InfSprinkle => {
            if rng.bool_1_in(4) {
                rng.sign() * f32::INFINITY
            } else {
                rng.unit()
            }
        }
        Pop::NanSprinkle => {
            if rng.bool_1_in(4) {
                nan(rng)
            } else {
                rng.unit()
            }
        }
        Pop::RandomBits => f32::from_bits(rng.next_u32()),
    }
}

pub fn fill(rng: &mut Rng, dst: &mut [f32], pop: Pop) {
    for slot in dst.iter_mut() {
        *slot = gen_elem(rng, pop);
    }
}

/// The size sweep of axis A.
pub const SIZES: &[i32] = &[
    1, 2, 3, 4, 5, 6, 7, 8, 9, 11, 15, 16, 17, 23, 31, 32, 33, 64, 65, 127, 128, 1000, 4096,
];

pub fn pick_size(rng: &mut Rng) -> i32 {
    SIZES[rng.below(SIZES.len())]
}

// ---------------------------------------------------------------------------
// pointer-relationship axis (axis B of CONFIGS.md)
// ---------------------------------------------------------------------------

#[derive(Copy, Clone, Debug)]
pub enum Alias {
    /// `dest` and `src` are far apart in the buffer.
    Disjoint,
    /// `dest == src`.
    InPlace,
    /// `dest == src + delta` (delta may be negative).
    Delta(isize),
    /// `dest == src + delta` with `delta` drawn uniformly from `lo..=hi` each
    /// iteration (0 included, which degenerates to the in-place case).
    RandomDelta(i32, i32),
    /// `dest == src + size/2`.
    HalfOverlap,
}

/// Padding, in elements, kept around every live region so that an off-by-one
/// over-write lands inside the compared buffer instead of corrupting the heap.
pub const GUARD: usize = 8;

/// Build a flat scratch buffer and return `(buffer, dest_off, src_off)`.
///
/// `src` always occupies `[src_off, src_off + n)`; `dest` is placed according
/// to `alias`. Everything outside the live regions is a guard band that must be
/// bit-identical after both calls.
pub fn layout(n: usize, alias: Alias) -> (Vec<f32>, usize, usize) {
    let base = GUARD + 8; // headroom for negative deltas
    match alias {
        Alias::Disjoint => {
            let src_off = base;
            let dest_off = base + n + 2 * GUARD;
            let total = dest_off + n + 2 * GUARD;
            (vec![0.0; total], dest_off, src_off)
        }
        Alias::InPlace => {
            let off = base;
            let total = off + n + 2 * GUARD;
            (vec![0.0; total], off, off)
        }
        Alias::Delta(d) => {
            let src_off = base;
            let dest_off = (src_off as isize + d) as usize;
            let total = src_off.max(dest_off) + n + 2 * GUARD;
            (vec![0.0; total], dest_off, src_off)
        }
        Alias::RandomDelta(..) | Alias::HalfOverlap => {
            unreachable!("resolve to Alias::Delta before calling layout")
        }
    }
}

/// Distinct, recognisable sentinel so that "was not written" is observable.
pub fn sentinel(i: usize) -> f32 {
    f32::from_bits(0x4B00_0000u32.wrapping_add(i as u32 * 7 + 1)) // ~8.4e6, all distinct
}

/// Paint guard bands (and, for the disjoint case, the whole `dest` region)
/// with sentinels, so a missing or extra write is visible.
pub fn paint_untouched(buf: &mut [f32], live: &[(usize, usize)]) {
    'outer: for i in 0..buf.len() {
        for &(off, n) in live {
            if i >= off && i < off + n {
                continue 'outer;
            }
        }
        buf[i] = sentinel(i);
    }
}

// ---------------------------------------------------------------------------
// the differential comparison
// ---------------------------------------------------------------------------

fn bits(v: &[f32]) -> Vec<u32> {
    v.iter().map(|x| x.to_bits()).collect()
}

/// Run C and Rust on identical copies of `buf`, returning both result buffers.
pub fn run_both(buf: &[f32], dest_off: usize, src_off: usize, size: i32) -> (Vec<f32>, Vec<f32>) {
    let l = libs();
    let cf = l.c();
    let rf = l.rust();

    let mut bc = buf.to_vec();
    let mut br = buf.to_vec();

    unsafe { cf(bc.as_mut_ptr().add(dest_off), bc.as_ptr().add(src_off), size) };
    unsafe { rf(br.as_mut_ptr().add(dest_off), br.as_ptr().add(src_off), size) };

    (bc, br)
}

/// Assert C and Rust agree bit-for-bit; returns the (identical) C result.
pub fn diff_expect(
    what: &str,
    buf: &[f32],
    dest_off: usize,
    src_off: usize,
    size: i32,
) -> Vec<f32> {
    match diff_once(buf, dest_off, src_off, size) {
        Ok(()) => run_both(buf, dest_off, src_off, size).0,
        Err(e) => panic!("[{what}]\n{e}"),
    }
}

/// Run C and Rust on identical copies of `buf` and compare every element
/// bit-for-bit. Returns `Err(description)` on the first divergence.
pub fn diff_once(buf: &[f32], dest_off: usize, src_off: usize, size: i32) -> Result<(), String> {
    let (bc, br) = run_both(buf, dest_off, src_off, size);
    let (a, b) = (bits(&bc), bits(&br));
    if a == b {
        return Ok(());
    }
    let idx = a.iter().zip(&b).position(|(x, y)| x != y).unwrap();
    let ndiff = a.iter().zip(&b).filter(|(x, y)| x != y).count();
    Err(format!(
        "divergence: size={size} dest_off={dest_off} src_off={src_off}\n  \
         first differing index {idx} (dest-relative {})\n  \
         C    = {:#010x} ({:e})\n  Rust = {:#010x} ({:e})\n  \
         src[{}] = {:#010x} ({:e})\n  {ndiff} of {} elements differ",
        idx as isize - dest_off as isize,
        a[idx],
        f32::from_bits(a[idx]),
        b[idx],
        f32::from_bits(b[idx]),
        idx as isize - src_off as isize,
        buf.get((idx as isize - dest_off as isize + src_off as isize) as usize)
            .map(|v| v.to_bits())
            .unwrap_or(0),
        buf.get((idx as isize - dest_off as isize + src_off as isize) as usize)
            .copied()
            .unwrap_or(0.0),
        a.len(),
    ))
}

/// Property-style driver for one `CONFIGS.md` row: `iters` randomized inputs.
pub fn run_row(name: &str, iters: usize, alias: Alias, pop: Pop, sizes: Option<&[i32]>) {
    run_row_with(name, iters, alias, sizes, |rng, dst| fill(rng, dst, pop), &format!("{pop:?}"));
}

/// Same as [`run_row`] but with a caller-supplied `src` filler, for rows whose
/// population is contrived rather than sampled (e.g. `sum == 1.0` exactly).
pub fn run_row_with(
    name: &str,
    iters: usize,
    alias: Alias,
    sizes: Option<&[i32]>,
    mut filler: impl FnMut(&mut Rng, &mut [f32]),
    pop_label: &str,
) {
    let mut rng = Rng::new(
        SEED ^ name.bytes().fold(0u64, |a, b| a.wrapping_mul(131).wrapping_add(b as u64)),
    );
    for it in 0..iters {
        let n = match sizes {
            Some(s) => s[rng.below(s.len())],
            None => pick_size(&mut rng),
        };
        let n_usize = n.max(0) as usize;
        let alias = match alias {
            Alias::RandomDelta(lo, hi) => Alias::Delta(rng.range_i32(lo, hi) as isize),
            Alias::HalfOverlap => Alias::Delta((n_usize / 2) as isize),
            a => a,
        };
        let (mut buf, dest_off, src_off) = layout(n_usize, alias);
        // Everything outside `src` starts as a distinct sentinel, so "not
        // written" is distinguishable from "written zero".
        paint_untouched(&mut buf, &[(src_off, n_usize)]);
        let mut tmp = vec![0.0f32; n_usize];
        filler(&mut rng, &mut tmp);
        buf[src_off..src_off + n_usize].copy_from_slice(&tmp);

        if let Err(e) = diff_once(&buf, dest_off, src_off, n) {
            panic!("[{name}] iteration {it} (pop={pop_label}, alias={alias:?}, n={n}):\n{e}");
        }
    }
}
