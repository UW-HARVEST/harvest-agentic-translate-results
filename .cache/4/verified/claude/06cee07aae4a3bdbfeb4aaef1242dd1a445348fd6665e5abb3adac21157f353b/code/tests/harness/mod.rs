//! Shared differential-test harness.
//!
//! Loads BOTH shared libraries through `libloading` and calls `synth_pair`
//! across the FFI boundary in exactly the same way an external C consumer
//! would.  The Rust implementation is never called directly -- only through the
//! `#[no_mangle] extern "C"` export in the freshly built cdylib.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

/// `void synth_pair(mp3d_sample_t *pcm, int nch, const float *z);`
pub type SynthPairFn = unsafe extern "C" fn(*mut i16, c_int, *const f32);

/// Highest index read from `z`: the second chain advances `z` by 2 and then
/// reads `z[14 * 64]`, i.e. absolute index `2 + 896 == 898`.
pub const Z_MAX_INDEX: usize = 2 + 14 * 64;
/// Exact number of `float`s the C code may touch.  Buffers are allocated at
/// exactly this length so that a read past the end is a real out-of-bounds
/// access rather than a silently-tolerated one.
pub const Z_LEN: usize = Z_MAX_INDEX + 1; // 899

// ---------------------------------------------------------------------------
// The two accumulation chains, transcribed from c_src/src/lib.c
// ---------------------------------------------------------------------------

/// Chain 0 -> `pcm[0]`:
/// ```text
/// a  = (z[14*64] - z[0])      * 29
/// a += (z[1*64]  + z[13*64])  * 213
/// a += (z[12*64] - z[2*64])   * 459
/// a += (z[3*64]  + z[11*64])  * 2037
/// a += (z[10*64] - z[4*64])   * 5153
/// a += (z[5*64]  + z[9*64])   * 6574
/// a += (z[8*64]  - z[6*64])   * 37489
/// a +=  z[7*64]               * 75038
/// ```
/// `(absolute z index, effective signed coefficient)`
pub const CHAIN0: [(usize, f32); 15] = [
    (0 * 64, -29.0),
    (1 * 64, 213.0),
    (2 * 64, -459.0),
    (3 * 64, 2037.0),
    (4 * 64, -5153.0),
    (5 * 64, 6574.0),
    (6 * 64, -37489.0),
    (7 * 64, 75038.0),
    (8 * 64, 37489.0),
    (9 * 64, 6574.0),
    (10 * 64, 5153.0),
    (11 * 64, 2037.0),
    (12 * 64, 459.0),
    (13 * 64, 213.0),
    (14 * 64, 29.0),
];

/// Chain 1 -> `pcm[16 * nch]`, evaluated after `z += 2`, and touching only the
/// **even** multiples of 64:
/// ```text
/// a  = z[14*64] * 104
/// a += z[12*64] * 1567
/// a += z[10*64] * 9727
/// a += z[8*64]  * 64019
/// a += z[6*64]  * -9975
/// a += z[4*64]  * -45
/// a += z[2*64]  * 146
/// a += z[0*64]  * -5
/// ```
/// `(absolute z index, effective signed coefficient)`
pub const CHAIN1: [(usize, f32); 8] = [
    (2 + 0 * 64, -5.0),
    (2 + 2 * 64, 146.0),
    (2 + 4 * 64, -45.0),
    (2 + 6 * 64, -9975.0),
    (2 + 8 * 64, 64019.0),
    (2 + 10 * 64, 9727.0),
    (2 + 12 * 64, 1567.0),
    (2 + 14 * 64, 104.0),
];

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Chain {
    /// Accumulator stored to `pcm[0]`.
    Lo,
    /// Accumulator stored to `pcm[16 * nch]`.
    Hi,
}

impl Chain {
    pub fn taps(self) -> &'static [(usize, f32)] {
        match self {
            Chain::Lo => &CHAIN0,
            Chain::Hi => &CHAIN1,
        }
    }
}

/// Every index of `z` the C code reads, sorted.  Exactly 23 of the 899 slots.
pub fn read_indices() -> Vec<usize> {
    let mut v: Vec<usize> = CHAIN0.iter().map(|&(i, _)| i).collect();
    v.extend(CHAIN1.iter().map(|&(i, _)| i));
    v.sort_unstable();
    v
}

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Build (if necessary) and return the path of the C shared library.
pub fn c_so_path() -> PathBuf {
    let c_src = manifest_dir().join("c_src");
    let build = c_src.join("build");
    // The CMake project name is derived from the parent directory name.
    let project = manifest_dir()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();
    let so = build.join(format!("lib{project}.so"));
    if so.exists() {
        return so;
    }
    std::fs::create_dir_all(&build).expect("create c_src/build");
    let st = Command::new("cmake")
        .current_dir(&build)
        .args(["..", "-DCMAKE_POSITION_INDEPENDENT_CODE=ON"])
        .status()
        .expect("run cmake");
    assert!(st.success(), "cmake configure failed");
    let st = Command::new("cmake")
        .current_dir(&build)
        .args(["--build", "."])
        .status()
        .expect("run cmake --build");
    assert!(st.success(), "cmake build failed");
    assert!(so.exists(), "C shared library not produced at {so:?}");
    so
}

/// Build the Rust cdylib into a dedicated target directory (so it does not
/// contend with the `cargo test` lock) and return its path.
pub fn rust_so_path() -> PathBuf {
    let profile = std::env::var("SP_RUST_PROFILE").unwrap_or_else(|_| "release".to_string());
    let target_dir = manifest_dir().join("target").join("ffi-cdylib");
    let out = target_dir.join(&profile).join("libsynth_pair_lib.so");

    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let mut cmd = Command::new(cargo);
    cmd.current_dir(manifest_dir())
        .env("CARGO_TARGET_DIR", &target_dir)
        .env_remove("RUSTFLAGS")
        .args(["build", "--offline", "--lib"]);
    if profile == "release" {
        cmd.arg("--release");
    }
    // Feature selection is propagated so the combination under test is the one
    // that actually gets loaded.  (The crate declares no [features] today.)
    if let Ok(feats) = std::env::var("SP_FEATURES") {
        cmd.arg("--no-default-features");
        if !feats.is_empty() {
            cmd.args(["--features", &feats]);
        }
    }
    let st = cmd.status().expect("run nested cargo build");
    assert!(st.success(), "nested cargo build of the cdylib failed");
    assert!(out.exists(), "Rust cdylib not produced at {out:?}");
    out
}

struct Libs {
    _c_lib: Library,
    _rust_lib: Library,
    c: SynthPairFn,
    rust: SynthPairFn,
    c_path: PathBuf,
    rust_path: PathBuf,
}

// Plain `extern "C"` pointers into two libraries kept loaded for the whole
// process lifetime.
unsafe impl Send for Libs {}
unsafe impl Sync for Libs {}

fn load(path: &Path) -> (Library, SynthPairFn) {
    let lib = unsafe { Library::new(path) }.unwrap_or_else(|e| panic!("dlopen {path:?}: {e}"));
    let f = {
        let sym: Symbol<SynthPairFn> = unsafe { lib.get(b"synth_pair\0") }
            .unwrap_or_else(|e| panic!("dlsym synth_pair in {path:?}: {e}"));
        *sym
    };
    (lib, f)
}

fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| {
        let c_path = c_so_path();
        let rust_path = rust_so_path();
        let (c_lib, c) = load(&c_path);
        let (rust_lib, rust) = load(&rust_path);
        Libs {
            _c_lib: c_lib,
            _rust_lib: rust_lib,
            c,
            rust,
            c_path,
            rust_path,
        }
    })
}

pub fn c_synth_pair() -> SynthPairFn {
    libs().c
}

pub fn rust_synth_pair() -> SynthPairFn {
    libs().rust
}

pub fn c_library_path() -> &'static Path {
    &libs().c_path
}

pub fn rust_library_path() -> &'static Path {
    &libs().rust_path
}

/// Which implementation to invoke -- used by the crash-parity child process.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Impl {
    C,
    Rust,
}

pub fn impl_fn(which: Impl) -> SynthPairFn {
    match which {
        Impl::C => c_synth_pair(),
        Impl::Rust => rust_synth_pair(),
    }
}

// ---------------------------------------------------------------------------
// PCM buffer model
// ---------------------------------------------------------------------------

pub const PCM_POISON: i16 = 0x5A5Au16 as i16;

/// A PCM scratch buffer plus the index handed to the library as `pcm`.  A
/// non-zero base lets us pass negative (or negatively-wrapped) `nch` values
/// without leaving the allocation.
#[derive(Clone, Debug)]
pub struct PcmBuf {
    pub data: Vec<i16>,
    pub base: usize,
}

/// The offset the C code stores its second sample at: `16 * nch` computed in
/// `int` (wrapping, exactly as gcc lowers it).
pub fn second_store_offset(nch: c_int) -> isize {
    16i32.wrapping_mul(nch) as isize
}

impl PcmBuf {
    /// Buffer large enough for `pcm[0]` and `pcm[16 * nch]`, prefilled with a
    /// recognisable poison value so stray writes are visible.
    pub fn for_nch(nch: c_int) -> Self {
        let off = second_store_offset(nch);
        assert!(
            off.unsigned_abs() <= 1 << 16,
            "offset {off} for nch={nch} is a wild pointer; use the subprocess \
             crash-parity test instead of an in-process buffer"
        );
        let pad = 8usize;
        let base = if off < 0 {
            off.unsigned_abs() + pad
        } else {
            pad
        };
        let span = (base as isize + off) as usize;
        let len = (base + pad).max(span + pad);
        PcmBuf {
            data: vec![PCM_POISON; len],
            base,
        }
    }

    pub fn ptr(&mut self) -> *mut i16 {
        unsafe { self.data.as_mut_ptr().add(self.base) }
    }

    /// Indices of the buffer that differ from the poison fill.
    pub fn touched(&self) -> Vec<usize> {
        self.data
            .iter()
            .enumerate()
            .filter(|&(_, &v)| v != PCM_POISON)
            .map(|(i, _)| i)
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Differential driver
// ---------------------------------------------------------------------------

/// Run one differential call: identical inputs to both libraries, returning the
/// two resulting PCM buffers.
pub fn run_pair(nch: c_int, z: &[f32]) -> (PcmBuf, PcmBuf) {
    let mut c_buf = PcmBuf::for_nch(nch);
    let mut r_buf = c_buf.clone();

    let c = c_synth_pair();
    let r = rust_synth_pair();
    unsafe {
        c(c_buf.ptr(), nch, z.as_ptr());
        r(r_buf.ptr(), nch, z.as_ptr());
    }
    (c_buf, r_buf)
}

/// Assert byte-identical PCM output for one configuration.
#[track_caller]
pub fn assert_same(label: &str, nch: c_int, z: &[f32]) -> PcmBuf {
    assert!(
        z.len() >= Z_LEN,
        "z must have at least {Z_LEN} elements, got {}",
        z.len()
    );
    let (c_buf, r_buf) = run_pair(nch, z);
    if c_buf.data != r_buf.data {
        report_mismatch(label, nch, z, &c_buf, &r_buf);
    }
    c_buf
}

/// Same as [`assert_same`] but for a raw `z` pointer (used for alignment and
/// short-buffer rows).  The caller guarantees 899 readable floats.
#[track_caller]
pub unsafe fn assert_same_ptr(label: &str, nch: c_int, z: *const f32) -> PcmBuf {
    let mut c_buf = PcmBuf::for_nch(nch);
    let mut r_buf = c_buf.clone();
    let c = c_synth_pair();
    let r = rust_synth_pair();
    unsafe {
        c(c_buf.ptr(), nch, z);
        r(r_buf.ptr(), nch, z);
    }
    if c_buf.data != r_buf.data {
        let view = unsafe { std::slice::from_raw_parts(z, Z_LEN) };
        report_mismatch(label, nch, view, &c_buf, &r_buf);
    }
    c_buf
}

#[track_caller]
fn report_mismatch(label: &str, nch: c_int, z: &[f32], c_buf: &PcmBuf, r_buf: &PcmBuf) -> ! {
    let diffs: Vec<String> = c_buf
        .data
        .iter()
        .zip(r_buf.data.iter())
        .enumerate()
        .filter(|(_, (a, b))| a != b)
        .map(|(i, (a, b))| {
            let rel = i as isize - c_buf.base as isize;
            format!("  buf[{i}] (pcm[{rel}]): C={a} RUST={b}")
        })
        .collect();
    panic!(
        "{label}: PCM mismatch\n  nch={nch}  base={}  second-store offset={}\n\
         {}\n  chain0 taps: {}\n  chain1 taps: {}",
        c_buf.base,
        second_store_offset(nch),
        diffs.join("\n"),
        describe(z, &CHAIN0),
        describe(z, &CHAIN1),
    );
}

fn describe(z: &[f32], taps: &[(usize, f32)]) -> String {
    taps.iter()
        .map(|&(i, w)| format!("z[{i}]={:e}(*{w})", z[i]))
        .collect::<Vec<_>>()
        .join(" ")
}

// ---------------------------------------------------------------------------
// z-buffer builders
// ---------------------------------------------------------------------------

/// A `z` buffer of exactly the readable length whose *unread* slots are
/// poisoned with `NaN`.  Any indexing mistake in the translation turns into an
/// immediate divergence, because a `NaN` tap collapses the accumulator to `0`
/// while the C result does not.
pub fn poisoned_z() -> Vec<f32> {
    vec![f32::NAN; Z_LEN]
}

/// Fill every read tap using `f`, leaving unread slots `NaN`-poisoned.
pub fn z_from(mut f: impl FnMut(usize) -> f32) -> Vec<f32> {
    let mut z = poisoned_z();
    for idx in read_indices() {
        z[idx] = f(idx);
    }
    z
}

/// All read taps set to `v`.
pub fn z_const(v: f32) -> Vec<f32> {
    z_from(|_| v)
}

/// All read taps zero.
pub fn z_zero() -> Vec<f32> {
    z_const(0.0)
}

/// All read taps zero except `index`, set to `v`.
pub fn z_single(index: usize, v: f32) -> Vec<f32> {
    assert!(read_indices().contains(&index), "index {index} is not read");
    let mut z = z_zero();
    z[index] = v;
    z
}

// ---------------------------------------------------------------------------
// Accumulator targeting
// ---------------------------------------------------------------------------

/// Step `v` by `d` ULPs (towards +inf for positive `d`).
pub fn nudge(v: f32, d: i32) -> f32 {
    if d == 0 {
        return v;
    }
    let bits = v.to_bits() as i32;
    // Map to a monotone ordering so stepping works across zero.
    let ord = if bits < 0 { i32::MIN - bits } else { bits };
    let ord = ord.wrapping_add(d);
    let bits = if ord < 0 { i32::MIN - ord } else { ord };
    f32::from_bits(bits as u32)
}

/// With a single tap active, the chain's accumulator is *exactly*
/// `fl(v * coefficient)` (every other term contributes an exact `0`, and
/// `0 + x == x`).  Find a tap value that lands the accumulator exactly on
/// `target`.
pub fn find_single_tap_exact(chain: Chain, target: f32) -> Option<(usize, f32)> {
    for &(idx, w) in chain.taps() {
        let base = (target as f64 / w as f64) as f32;
        for d in -64i32..=64 {
            let v = nudge(base, d);
            if v.is_finite() && v * w == target && (target != 0.0 || v == 0.0) {
                return Some((idx, v));
            }
        }
    }
    None
}

/// `z` buffer whose `chain` accumulator is exactly `target`.
#[track_caller]
pub fn z_for_exact(chain: Chain, target: f32) -> Vec<f32> {
    let (idx, v) = find_single_tap_exact(chain, target)
        .unwrap_or_else(|| panic!("no single-tap value reaches {target:e} exactly on {chain:?}"));
    z_single(idx, v)
}

/// Accumulator value produced by a single active tap.
pub fn single_tap_accumulator(chain: Chain, idx: usize, v: f32) -> f32 {
    let w = chain
        .taps()
        .iter()
        .find(|&&(i, _)| i == idx)
        .expect("tap belongs to chain")
        .1;
    v * w
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) -- fixed seeds keep failures reproducible
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
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

    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 0
    }

    /// Uniform in `[0, 1)`.
    pub fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }

    /// Uniform in `[-m, m]`.
    pub fn sym(&mut self, m: f32) -> f32 {
        (self.unit() * 2.0 - 1.0) * m
    }

    /// Any bit pattern -- includes `NaN`, infinities and subnormals.
    pub fn any_bits_f32(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }

    /// Random sign, log-uniform magnitude in `[10^lo, 10^hi]`.
    pub fn log_uniform(&mut self, lo: f32, hi: f32) -> f32 {
        let e = lo + self.unit() * (hi - lo);
        let m = 10f32.powf(e);
        if self.bool() { m } else { -m }
    }

    /// A random subnormal `f32`.
    pub fn subnormal(&mut self) -> f32 {
        let mant = self.next_u32() & 0x007F_FFFF;
        let sign = (self.next_u32() & 1) << 31;
        f32::from_bits(sign | mant)
    }

    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }

    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len())]
    }
}
