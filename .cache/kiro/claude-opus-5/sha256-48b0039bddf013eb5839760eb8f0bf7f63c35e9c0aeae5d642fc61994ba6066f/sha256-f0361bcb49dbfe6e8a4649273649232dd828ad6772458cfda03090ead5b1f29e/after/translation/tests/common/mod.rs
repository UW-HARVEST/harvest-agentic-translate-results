#![allow(dead_code)]

//! Shared harness: loads BOTH shared objects through `libloading` and exposes
//! them behind an identical call signature, so no Rust function is ever called
//! directly — every invocation crosses the `.so` FFI boundary exactly as an
//! external consumer's would.

use std::ffi::c_int;
use std::path::PathBuf;

use libloading::{Library, Symbol};

pub type SynthPairFn = unsafe extern "C" fn(*mut i16, c_int, *const f32);

/// Number of floats `synth_pair` may legally touch:
/// first block reads `z[0 ..= 14*64]`, then `z += 2` and the second block reads
/// `z[2 ..= 2 + 14*64]`, i.e. indices `0 ..= 898`.
pub const Z_LEN: usize = 2 + 14 * 64 + 1; // 899

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    if let Ok(p) = std::env::var("SYNTH_C_SO") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "SYNTH_C_SO={} does not exist", p.display());
        return p;
    }
    let build = workspace_root().join("c_src/build");
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| panic!("cannot read {}: {e} — build the C lib first", build.display()))
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "so"))
        .collect();
    candidates.sort();
    candidates
        .pop()
        .unwrap_or_else(|| panic!("no .so found in {}", build.display()))
}

fn find_rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("SYNTH_RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "SYNTH_RUST_SO={} does not exist", p.display());
        return p;
    }
    let root = workspace_root().join("translation/target");
    // Prefer the profile the tests were built with, then fall back.
    for profile in ["release", "debug"] {
        let p = root.join(profile).join("libsynth_pair_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "libsynth_pair_lib.so not found under {} — run `cargo build --release` first",
        root.display()
    )
}

/// Both libraries, kept alive for the lifetime of the test.
pub struct Pair {
    _c_lib: Library,
    _rust_lib: Library,
    pub c: SynthPairFn,
    pub rs: SynthPairFn,
}

impl Pair {
    pub fn load() -> Self {
        unsafe {
            let c_path = find_c_so();
            let rs_path = find_rust_so();
            let c_lib = Library::new(&c_path)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", c_path.display()));
            let rust_lib = Library::new(&rs_path)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", rs_path.display()));

            let c_sym: Symbol<SynthPairFn> = c_lib
                .get(b"synth_pair\0")
                .expect("C .so does not export synth_pair");
            let rs_sym: Symbol<SynthPairFn> = rust_lib
                .get(b"synth_pair\0")
                .expect("Rust .so does not export synth_pair");

            let c = *c_sym;
            let rs = *rs_sym;
            Pair { _c_lib: c_lib, _rust_lib: rust_lib, c, rs }
        }
    }
}

/// Deterministic PRNG (SplitMix64) so every randomized row is reproducible.
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
    /// Uniform in `[0, 1)`.
    pub fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }
    /// Uniform in `[-mag, mag]`.
    pub fn sym(&mut self, mag: f32) -> f32 {
        (self.unit() * 2.0 - 1.0) * mag
    }
    /// Arbitrary bit pattern reinterpreted as `f32` (may be NaN/Inf/subnormal).
    pub fn any_f32(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }
    /// Finite value spanning a very wide exponent range.
    pub fn wide_finite(&mut self) -> f32 {
        let sign = if self.next_u32() & 1 == 0 { 1.0f32 } else { -1.0f32 };
        // exponent 10^-30 .. 10^30
        let e = (self.unit() * 60.0 - 30.0) as f64;
        let v = (10.0f64).powf(e) * (self.unit() as f64 + 0.5);
        let out = (sign as f64 * v) as f32;
        if out.is_finite() { out } else { 0.0 }
    }
}

/// A `pcm` scratch buffer with generous headroom on BOTH sides, so negative and
/// wrapped `16*nch` indices land inside the allocation. `base()` returns the
/// pointer the library is handed; `HALF` elements exist before and after it.
pub struct Pcm {
    buf: Vec<i16>,
}

impl Pcm {
    /// Elements available on each side of the handed-out pointer.
    pub const HALF: usize = 1 << 12;

    pub fn new() -> Self {
        Pcm { buf: vec![0x5A5Au16 as i16; Self::HALF * 2 + 1] }
    }
    pub fn base(&mut self) -> *mut i16 {
        unsafe { self.buf.as_mut_ptr().add(Self::HALF) }
    }
    pub fn as_slice(&self) -> &[i16] {
        &self.buf
    }
}

impl Default for Pcm {
    fn default() -> Self {
        Self::new()
    }
}

/// Offsets (relative to `base()`) that `synth_pair` may write, mirroring the C
/// index computation `16 * nch` in wrapping 32-bit `int` arithmetic.
pub fn store_offsets(nch: i32) -> [isize; 2] {
    [0, nch.wrapping_mul(16) as isize]
}

/// Does `nch` keep both stores inside a `Pcm` buffer?
pub fn nch_fits(nch: i32) -> bool {
    let off = nch.wrapping_mul(16) as isize;
    off.unsigned_abs() < Pcm::HALF
}

/// Run one configuration through both `.so`s and compare the FULL buffers.
///
/// Comparing the whole buffer (not just the two expected slots) also catches a
/// Rust version that writes to a *different* offset than the C.
#[track_caller]
pub fn assert_same(pair: &Pair, nch: i32, z: &[f32], what: &str) {
    assert!(z.len() >= Z_LEN, "z too short for {what}");
    assert!(nch_fits(nch), "nch={nch} would write outside the scratch buffer ({what})");

    let mut c_pcm = Pcm::new();
    let mut rs_pcm = Pcm::new();

    unsafe {
        (pair.c)(c_pcm.base(), nch, z.as_ptr());
        (pair.rs)(rs_pcm.base(), nch, z.as_ptr());
    }

    if c_pcm.as_slice() != rs_pcm.as_slice() {
        let mut diffs = Vec::new();
        for (i, (a, b)) in c_pcm.as_slice().iter().zip(rs_pcm.as_slice()).enumerate() {
            if a != b {
                diffs.push(format!(
                    "  offset {:+} : C={} Rust={}",
                    i as isize - Pcm::HALF as isize,
                    a,
                    b
                ));
                if diffs.len() >= 8 {
                    break;
                }
            }
        }
        panic!(
            "DIVERGENCE [{what}] nch={nch}\n{}\nfirst taps: {:?}",
            diffs.join("\n"),
            &z[..8.min(z.len())]
        );
    }
}

/// `z` layout helpers -------------------------------------------------------
/// Index of the i-th tap of the FIRST accumulator: `z[i * 64]`, i in 0..=14.
pub fn tap1(i: usize) -> usize {
    i * 64
}
/// Index of the k-th tap of the SECOND accumulator: `(z+2)[k * 64]`.
pub fn tap2(k: usize) -> usize {
    2 + k * 64
}
