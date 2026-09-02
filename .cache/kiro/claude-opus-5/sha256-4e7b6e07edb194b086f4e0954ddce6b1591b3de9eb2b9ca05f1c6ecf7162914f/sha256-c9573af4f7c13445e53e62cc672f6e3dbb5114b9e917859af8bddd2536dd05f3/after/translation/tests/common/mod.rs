//! Shared differential-test harness.
//!
//! Both libraries are loaded as shared objects through `libloading` and called
//! only through their exported `gaussian_kernel` symbol, exactly as an external
//! C consumer would. The Rust implementation is never called directly, so the
//! `#[unsafe(no_mangle)] extern "C"` wrapper is part of what is under test.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use libloading::{Library, Symbol};

/// `void gaussian_kernel(float *dest, int size, float radius);`
pub type GaussianKernelFn = unsafe extern "C" fn(*mut f32, std::ffi::c_int, f32);

/// Guard value written into every element of the scratch buffer that the
/// function is *not* expected to touch. Chosen as a signalling-NaN-ish bit
/// pattern that no legitimate computation in this library can produce.
pub const GUARD_BITS: u32 = 0xDEAD_BEEF;

/// Number of guard elements appended after the writable region.
pub const PAD: usize = 8;

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO_PATH") {
        return PathBuf::from(p);
    }
    let build = repo_root().join("c_src").join("build");
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            let name = p.file_name().unwrap_or_default().to_string_lossy().to_string();
            if name.starts_with("lib") && name.ends_with(".so") {
                candidates.push(p);
            }
        }
    }
    candidates.sort();
    candidates.into_iter().next().unwrap_or_else(|| {
        panic!(
            "no C shared library found in {}; build it with:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

fn find_rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO_PATH") {
        return PathBuf::from(p);
    }
    let target = Path::new(env!("CARGO_MANIFEST_DIR")).join("target");
    // Prefer the release artifact (the one the task builds), fall back to debug.
    for profile in ["release", "debug"] {
        let p = target.join(profile).join("libgaussian_kernel_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "no Rust shared library found under {}; build it with:\n  cd translation && cargo build --release",
        target.display()
    )
}

/// A loaded shared object plus the resolved entry point.
pub struct Impl {
    _lib: Library,
    pub gaussian_kernel: GaussianKernelFn,
    pub path: PathBuf,
}

impl Impl {
    fn load(path: PathBuf) -> Impl {
        unsafe {
            let lib = Library::new(&path)
                .unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", path.display()));
            let sym: Symbol<GaussianKernelFn> = lib
                .get(b"gaussian_kernel\0")
                .unwrap_or_else(|e| panic!("{} does not export gaussian_kernel: {e}", path.display()));
            let f = *sym;
            Impl { _lib: lib, gaussian_kernel: f, path }
        }
    }
}

pub struct Pair {
    pub c: Impl,
    pub rs: Impl,
}

static PAIR: OnceLock<Pair> = OnceLock::new();

/// Both implementations, loaded once per test process.
pub fn pair() -> &'static Pair {
    PAIR.get_or_init(|| Pair {
        c: Impl::load(find_c_so()),
        rs: Impl::load(find_rust_so()),
    })
}

pub fn c_so_path() -> PathBuf {
    find_c_so()
}
pub fn rust_so_path() -> PathBuf {
    find_rust_so()
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5EED_1234_5EED_1234;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
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
    /// Uniform in `[lo, hi]` inclusive.
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
    /// Uniform `f64` in `[0, 1)`.
    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
    /// Log-uniform magnitude in `[lo, hi]` (both must be > 0).
    pub fn log_uniform(&mut self, lo: f64, hi: f64) -> f32 {
        let t = self.unit();
        let v = (lo.ln() + t * (hi.ln() - lo.ln())).exp();
        v as f32
    }
    /// A completely arbitrary `f32` (any class: normal, subnormal, inf, NaN).
    pub fn any_f32(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }
    /// A random subnormal `f32` with random sign.
    pub fn subnormal_f32(&mut self) -> f32 {
        let mantissa = 1 + (self.next_u32() % 0x007F_FFFF);
        let sign = (self.next_u32() & 1) << 31;
        f32::from_bits(sign | mantissa)
    }
}

// ---------------------------------------------------------------------------
// Differential driver
// ---------------------------------------------------------------------------

/// How many elements `gaussian_kernel` can possibly store into, for a given
/// `size`, derived from the C source:
///   loop 1 stores indices `0 ..= 2*hsize`  (only when `hsize >= 0`)
///   loop 2 stores indices `0 .. size`      (only when `size > 0`)
pub fn touched_len(size: i32) -> usize {
    let hsize = size / 2; // C truncating division
    let l1 = if hsize >= 0 { 2 * (hsize as i64) + 1 } else { 0 };
    let l2 = if size > 0 { size as i64 } else { 0 };
    l1.max(l2) as usize
}

/// Total scratch-buffer length: everything the function may write, plus guards.
pub fn buffer_len(size: i32) -> usize {
    touched_len(size) + PAD
}

/// Run one differential case.
///
/// `fill` produces the identical pre-call buffer contents given to both
/// implementations. Returns `Err(description)` on divergence.
pub fn diff_case(size: i32, radius: f32, fill: &[f32]) -> Result<Vec<f32>, String> {
    let p = pair();
    let mut cbuf = fill.to_vec();
    let mut rbuf = fill.to_vec();

    unsafe {
        (p.c.gaussian_kernel)(cbuf.as_mut_ptr(), size, radius);
        (p.rs.gaussian_kernel)(rbuf.as_mut_ptr(), size, radius);
    }

    for i in 0..cbuf.len() {
        let (cb, rb) = (cbuf[i].to_bits(), rbuf[i].to_bits());
        if cb != rb {
            return Err(format!(
                "divergence at index {i}/{len}: size={size} radius={radius:e} (radius bits=0x{rbits:08X})\n  \
                 C   = {cv:e} (bits 0x{cb:08X})\n  \
                 Rust= {rv:e} (bits 0x{rb:08X})\n  \
                 pre-call fill[{i}] = 0x{fb:08X}\n  \
                 C   buf = {cfull:?}\n  Rust buf = {rfull:?}",
                len = cbuf.len(),
                rbits = radius.to_bits(),
                cv = cbuf[i],
                rv = rbuf[i],
                fb = fill[i].to_bits(),
                cfull = cbuf.iter().map(|v| format!("0x{:08X}", v.to_bits())).collect::<Vec<_>>(),
                rfull = rbuf.iter().map(|v| format!("0x{:08X}", v.to_bits())).collect::<Vec<_>>(),
            ));
        }
    }
    Ok(cbuf)
}

/// `diff_case` with a fresh guard-filled buffer of the derived length.
pub fn diff(size: i32, radius: f32) -> Result<Vec<f32>, String> {
    let fill = vec![f32::from_bits(GUARD_BITS); buffer_len(size)];
    diff_case(size, radius, &fill)
}

/// `diff` that panics with a readable message on divergence.
pub fn expect_match(size: i32, radius: f32) -> Vec<f32> {
    match diff(size, radius) {
        Ok(v) => v,
        Err(e) => panic!("C/Rust divergence:\n{e}"),
    }
}

/// `diff_case` that panics with a readable message on divergence.
pub fn expect_match_fill(size: i32, radius: f32, fill: &[f32]) -> Vec<f32> {
    match diff_case(size, radius, fill) {
        Ok(v) => v,
        Err(e) => panic!("C/Rust divergence:\n{e}"),
    }
}

/// A randomized fill of arbitrary bit patterns (so "untouched" is detectable).
pub fn garbage_fill(rng: &mut Rng, len: usize) -> Vec<f32> {
    (0..len).map(|_| rng.any_f32()).collect()
}

// ---------------------------------------------------------------------------
// Independent reference model of the C, used to assert that the differential
// tests actually reach the branch each row claims to reach (so a row cannot be
// "passing" vacuously).
// ---------------------------------------------------------------------------

pub const SIGMA: f32 = 1.6;
pub const TETHA: f32 = 2.25;

/// `1.0f / expf(sigma*sigma*tetha)`
pub fn s2() -> f32 {
    1.0f32 / (SIGMA * SIGMA * TETHA).exp()
}

/// Reports which branches the C takes for a given input, for coverage asserts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Branches {
    pub iterations: u64,
    pub stores: u64,
    pub clamped_zero: u64,
    pub clamped_from_nan: u64,
    pub kept_positive: u64,
    pub normalised: bool,
}

pub fn branches(size: i32, radius: f32) -> Branches {
    let s2 = s2();
    let rs = SIGMA / radius;
    let hsize = size / 2;
    let mut b = Branches {
        iterations: 0,
        stores: 0,
        clamped_zero: 0,
        clamped_from_nan: 0,
        kept_positive: 0,
        normalised: false,
    };
    let mut sum = 0.0f32;
    let mut r = -hsize;
    while r <= hsize {
        b.iterations += 1;
        b.stores += 1;
        let x = (r as f32) * rs;
        let v = (1.0f32 / (x * x).exp()) - s2;
        if v.is_nan() {
            b.clamped_from_nan += 1;
        }
        let v = if v > 0.0 { v } else { 0.0 };
        if v > 0.0 {
            b.kept_positive += 1;
        } else {
            b.clamped_zero += 1;
        }
        sum += v;
        r += 1;
    }
    b.normalised = sum > 0.0;
    b
}
