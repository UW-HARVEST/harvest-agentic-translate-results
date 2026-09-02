//! Shared differential-test harness.
//!
//! Loads BOTH shared objects with `libloading` and calls `synth_pair` only
//! through their exported C symbols. The Rust implementation is never called
//! directly, so the `#[no_mangle] extern "C"` wrapper is under test too.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::path::{Path, PathBuf};

pub type SynthPairFn = unsafe extern "C" fn(*mut i16, c_int, *const f32);

/// Number of `f32`s a `z` buffer needs: block 1 reads up to `z[14*64] = z[896]`
/// and block 2 (after `z += 2`) up to `z[2 + 14*64] = z[898]`.
pub const Z_LEN: usize = 900;
/// Highest tap index used by either block.
pub const N_TAPS: usize = 15;

// ---------------------------------------------------------------------------
// .so discovery
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn find_so(dir: &Path, pred: &dyn Fn(&str) -> bool) -> Option<PathBuf> {
    let mut hits: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension().map(|e| e == "so").unwrap_or(false)
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| pred(n))
                    .unwrap_or(false)
        })
        .collect();
    hits.sort();
    hits.pop()
}

/// The C shared object produced by `c_src/build`.
pub fn c_so_path() -> PathBuf {
    let build = workspace_root().join("c_src/build");
    find_so(&build, &|_| true).unwrap_or_else(|| {
        panic!(
            "no .so found in {}; build it with:\n  cd c_src && mkdir -p build && cd build \\\n    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

/// The Rust `cdylib`. Located next to the currently running test binary
/// (`target/<profile>/deps/<test>` -> `target/<profile>/`), so the same profile
/// that `cargo test` used is the one that gets verified.
pub fn rust_so_path() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    // .../target/<profile>/deps/<testbin>
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("test binary must live in target/<profile>/deps");

    let want = |n: &str| n.starts_with("libsynth_pair_lib") && n.ends_with(".so");
    if let Some(p) = find_so(profile_dir, &want) {
        return p;
    }
    panic!(
        "libsynth_pair_lib.so not found in {} -- `cargo test` does not emit the \
         cdylib on its own. Run `cargo build` (same profile) first, or use \
         ./run_all.sh which does it for you.",
        profile_dir.display()
    );
}

// ---------------------------------------------------------------------------
// Harness
// ---------------------------------------------------------------------------

pub struct Harness {
    // Fields are dropped in declaration order; keep the libraries alive for as
    // long as the raw function pointers are used.
    c_fn: SynthPairFn,
    r_fn: SynthPairFn,
    _c_lib: Library,
    _r_lib: Library,
}

impl Harness {
    pub fn load() -> Harness {
        let c_path = c_so_path();
        let r_path = rust_so_path();
        unsafe {
            let c_lib = Library::new(&c_path)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", c_path.display()));
            let r_lib = Library::new(&r_path)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", r_path.display()));
            let c_sym: Symbol<SynthPairFn> = c_lib
                .get(b"synth_pair\0")
                .unwrap_or_else(|e| panic!("C synth_pair: {e}"));
            let r_sym: Symbol<SynthPairFn> = r_lib
                .get(b"synth_pair\0")
                .unwrap_or_else(|e| panic!("Rust synth_pair (missing #[no_mangle] export?): {e}"));
            let c_fn = *c_sym;
            let r_fn = *r_sym;
            Harness {
                c_fn,
                r_fn,
                _c_lib: c_lib,
                _r_lib: r_lib,
            }
        }
    }

    /// Raw call into the C `.so` — for cases (aliasing, faults) where the
    /// buffers cannot be modelled as two independent slices.
    ///
    /// # Safety
    /// Same contract as the C function.
    pub unsafe fn call_raw_c(&self, pcm: *mut i16, nch: c_int, z: *const f32) {
        unsafe { (self.c_fn)(pcm, nch, z) }
    }

    /// Raw call into the Rust `.so`, via its exported `synth_pair` symbol.
    ///
    /// # Safety
    /// Same contract as the C function.
    pub unsafe fn call_raw_rust(&self, pcm: *mut i16, nch: c_int, z: *const f32) {
        unsafe { (self.r_fn)(pcm, nch, z) }
    }

    /// Run one case against both `.so`s and return `(c_pcm, rust_pcm)`, each the
    /// **entire** pcm buffer so that out-of-range / clobbering writes show up.
    ///
    /// `pcm_prefill` is the initial buffer contents; `pcm_index` is the element
    /// inside it whose address is handed to `synth_pair` (allowing negative
    /// `nch` to write backwards while staying inside the allocation).
    pub fn call_both(
        &self,
        z: &[f32],
        nch: c_int,
        pcm_prefill: &[i16],
        pcm_index: usize,
    ) -> (Vec<i16>, Vec<i16>) {
        assert!(z.len() >= 899, "z buffer too short for the taps actually read");
        let mut c_pcm = pcm_prefill.to_vec();
        let mut r_pcm = pcm_prefill.to_vec();
        unsafe {
            (self.c_fn)(c_pcm.as_mut_ptr().add(pcm_index), nch, z.as_ptr());
            (self.r_fn)(r_pcm.as_mut_ptr().add(pcm_index), nch, z.as_ptr());
        }
        (c_pcm, r_pcm)
    }

    /// `call_both` + byte-for-byte assertion, with a diagnostic that names the
    /// row of `CONFIGS.md` / `ERRORS.md` being checked.
    pub fn assert_same(
        &self,
        label: &str,
        z: &[f32],
        nch: c_int,
        pcm_prefill: &[i16],
        pcm_index: usize,
    ) -> Vec<i16> {
        let (c_pcm, r_pcm) = self.call_both(z, nch, pcm_prefill, pcm_index);
        if c_pcm != r_pcm {
            let diff: Vec<String> = c_pcm
                .iter()
                .zip(r_pcm.iter())
                .enumerate()
                .filter(|(_, (a, b))| a != b)
                .map(|(i, (a, b))| format!("pcm[{i}]: C={a} Rust={b}"))
                .collect();
            panic!(
                "DIVERGENCE [{label}] nch={nch} pcm_index={pcm_index}\n  {}\n  taps1={:?}\n  taps2={:?}",
                diff.join("\n  "),
                tap_bits1(z),
                tap_bits2(z),
            );
        }
        c_pcm
    }
}

/// Block-1 taps (`z[i*64]`) as raw bits, for failure diagnostics.
pub fn tap_bits1(z: &[f32]) -> Vec<String> {
    (0..N_TAPS)
        .map(|i| format!("{:08x}", z[i * 64].to_bits()))
        .collect()
}

/// Block-2 taps (`z[2 + i*64]`) as raw bits, for failure diagnostics.
pub fn tap_bits2(z: &[f32]) -> Vec<String> {
    (0..N_TAPS)
        .map(|i| format!("{:08x}", z[2 + i * 64].to_bits()))
        .collect()
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64) — fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5EED_1234_ABCD_0001;

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
    /// Uniform in `0..n`.
    pub fn below(&mut self, n: u64) -> u64 {
        if n == 0 { 0 } else { self.next_u64() % n }
    }
    /// Uniform in `[-1.0, 1.0]`.
    pub fn unit(&mut self) -> f32 {
        let x = (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32;
        2.0 * x - 1.0
    }
    /// Uniform in `[-scale, scale]`.
    pub fn scaled(&mut self, scale: f32) -> f32 {
        self.unit() * scale
    }
    /// Any `f32` bit pattern at all (includes `NaN` payloads, `±Inf`,
    /// subnormals, huge exponents).
    pub fn any_f32(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }
    /// A random subnormal `f32` (`|v| < 2^-126`).
    pub fn subnormal(&mut self) -> f32 {
        let mant = self.next_u32() & 0x007F_FFFF;
        let sign = (self.next_u32() & 1) << 31;
        f32::from_bits(sign | mant.max(1))
    }
    /// Draw from a mix of the value classes the C code distinguishes.
    pub fn mixed(&mut self) -> f32 {
        match self.below(10) {
            0 => 0.0,
            1 => -0.0,
            2 => self.subnormal(),
            3 => self.scaled(1e-2),
            4 => self.scaled(0.5),
            5 => self.scaled(1e6),
            6 => self.scaled(1e35),
            7 => {
                if self.next_u64() & 1 == 0 {
                    f32::INFINITY
                } else {
                    f32::NEG_INFINITY
                }
            }
            8 => f32::from_bits(0x7FC0_0000 | (self.next_u32() & 0x003F_FFFF)),
            _ => self.any_f32(),
        }
    }
}

// ---------------------------------------------------------------------------
// z-buffer construction
// ---------------------------------------------------------------------------

/// A `z` buffer of `Z_LEN` zeros.
pub fn z_zeros() -> Vec<f32> {
    vec![0.0f32; Z_LEN]
}

/// Set block-1 tap `i` (`z[i*64]`).
pub fn set_tap1(z: &mut [f32], i: usize, v: f32) {
    z[i * 64] = v;
}

/// Set block-2 tap `i` (`z[2 + i*64]`, i.e. after the C's `z += 2`).
pub fn set_tap2(z: &mut [f32], i: usize, v: f32) {
    z[2 + i * 64] = v;
}

/// Fill every byte of the buffer that is *not* a tap with adversarial garbage,
/// to prove the 64-float stride is honoured identically (CONFIGS row C12).
pub fn poison_filler(z: &mut [f32], rng: &mut Rng) {
    let mut is_tap = [false; Z_LEN];
    for i in 0..N_TAPS {
        is_tap[i * 64] = true;
        is_tap[2 + i * 64] = true;
    }
    for i in 0..Z_LEN {
        if !is_tap[i] {
            z[i] = match rng.below(4) {
                0 => f32::NAN,
                1 => f32::INFINITY,
                2 => f32::NEG_INFINITY,
                _ => rng.scaled(1e30),
            };
        }
    }
}

// ---------------------------------------------------------------------------
// Driving the low-level `mp3d_scale_pcm` with an exact accumulator value
// ---------------------------------------------------------------------------

/// Coefficient of the **last** term of block 1 (`a += z[7*64] * 75038`).
pub const A1_LAST_COEF: f32 = 75038.0;
/// Coefficient of the **last** term of block 2 (`a += z[0*64] * -5`).
pub const A2_LAST_COEF: f32 = -5.0;

/// `(tap index, effective coefficient)` for block 1.
///
/// If exactly ONE block-1 tap is non-zero, every other term of the accumulation
/// contributes `+0.0`, and `x + 0.0 == x` exactly for all finite/infinite `x`.
/// So the accumulator collapses to `fl(v * coef)`, where `coef` is the signed
/// coefficient that tap carries in the C source:
///
/// ```text
/// a  = (z[14] - z[ 0]) * 29     ->  +29  at tap 14,  -29  at tap 0
/// a += (z[ 1] + z[13]) * 213    ->  +213 at taps 1, 13
/// a += (z[12] - z[ 2]) * 459    ->  +459 at tap 12,  -459 at tap 2
/// a += (z[ 3] + z[11]) * 2037   ->  +2037 at taps 3, 11
/// a += (z[10] - z[ 4]) * 5153   ->  +5153 at tap 10, -5153 at tap 4
/// a += (z[ 5] + z[ 9]) * 6574   ->  +6574 at taps 5, 9
/// a += (z[ 8] - z[ 6]) * 37489  ->  +37489 at tap 8, -37489 at tap 6
/// a +=  z[ 7]          * 75038  ->  +75038 at tap 7
/// ```
///
/// Having 15 different coefficients matters: for a single coefficient `c`, one
/// ULP of `v` moves `v * c` by between 1 and 2 ULPs of the product, so a single
/// coefficient can only reach ~50-100% of the targets on the result grid.
/// Trying all 15 makes every finite target reachable in practice.
pub const BLOCK1_TAP_COEFS: [(usize, f32); 15] = [
    (7, 75038.0),
    (8, 37489.0),
    (6, -37489.0),
    (5, 6574.0),
    (9, 6574.0),
    (10, 5153.0),
    (4, -5153.0),
    (3, 2037.0),
    (11, 2037.0),
    (12, 459.0),
    (2, -459.0),
    (1, 213.0),
    (13, 213.0),
    (14, 29.0),
    (0, -29.0),
];

/// `(tap index, effective coefficient)` for block 2 (indices are relative to
/// the `z += 2` shift, i.e. slot `z[2 + i*64]`). Block 2 reads only even slots.
///
/// ```text
/// a  = z[14] * 104
/// a += z[12] * 1567
/// a += z[10] * 9727
/// a += z[ 8] * 64019
/// a += z[ 6] * -9975
/// a += z[ 4] * -45
/// a += z[ 2] * 146
/// a += z[ 0] * -5
/// ```
pub const BLOCK2_TAP_COEFS: [(usize, f32); 8] = [
    (0, -5.0),
    (2, 146.0),
    (4, -45.0),
    (6, -9975.0),
    (8, 64019.0),
    (10, 9727.0),
    (12, 1567.0),
    (14, 104.0),
];

fn neighbours(v: f32) -> Vec<f32> {
    let mut out = vec![v];
    let mut up = v;
    let mut down = v;
    for _ in 0..40 {
        up = next_up(up);
        down = next_down(down);
        out.push(up);
        out.push(down);
    }
    out
}

fn next_up(v: f32) -> f32 {
    if v.is_nan() || v == f32::INFINITY {
        return v;
    }
    let b = v.to_bits();
    if v == 0.0 {
        f32::from_bits(1)
    } else if v > 0.0 {
        f32::from_bits(b + 1)
    } else {
        f32::from_bits(b - 1)
    }
}

fn next_down(v: f32) -> f32 {
    if v.is_nan() || v == f32::NEG_INFINITY {
        return v;
    }
    let b = v.to_bits();
    if v == 0.0 {
        f32::from_bits(0x8000_0001)
    } else if v > 0.0 {
        f32::from_bits(b - 1)
    } else {
        f32::from_bits(b + 1)
    }
}

fn same_bits(a: f32, b: f32) -> bool {
    a.to_bits() == b.to_bits()
}

/// Find a `(tap, value)` pair such that setting **only** that block-1 tap makes
/// block 1's accumulator bit-identical to `target`.
///
/// `-0.0` is the one unreachable target (the trailing `+ 0.0` terms turn it into
/// `+0.0`); it is covered by the hand-built all-`-0.0` sign pattern in
/// `cfg_c2b_negative_zero_accumulator` instead.
pub fn solve_a1(target: f32) -> Option<(usize, f32)> {
    solve_single_tap(target, &BLOCK1_TAP_COEFS)
}

/// Same, for block 2's accumulator.
pub fn solve_a2(target: f32) -> Option<(usize, f32)> {
    solve_single_tap(target, &BLOCK2_TAP_COEFS)
}

fn solve_single_tap(target: f32, table: &[(usize, f32)]) -> Option<(usize, f32)> {
    if target.is_nan() {
        // Any NaN operand yields a NaN accumulator; take the last-term tap so
        // no other rounding is involved.
        return Some((table[0].0, f32::NAN));
    }
    if target.is_infinite() {
        let (tap, coef) = table[0];
        let v = if (target > 0.0) == (coef > 0.0) {
            f32::INFINITY
        } else {
            f32::NEG_INFINITY
        };
        return Some((tap, v));
    }
    if target == 0.0 && target.is_sign_negative() {
        return None; // -0.0 is unreachable via a single tap; see doc comment.
    }
    for &(tap, coef) in table {
        let guess = target / coef;
        if let Some(v) = neighbours(guess)
            .into_iter()
            .find(|&v| v.is_finite() && same_bits(v * coef, target))
        {
            return Some((tap, v));
        }
    }
    None
}

/// Build a `z` buffer that makes block 1's accumulator exactly `a1` and block
/// 2's exactly `a2`.
///
/// The two blocks read disjoint slots (`i*64` vs `2 + i*64`, never congruent
/// mod 64), so they can be targeted independently — itself worth asserting
/// (CONFIGS row C21).
pub fn z_for_accumulators(a1: f32, a2: f32) -> Option<Vec<f32>> {
    let (t1, v1) = solve_a1(a1)?;
    let (t2, v2) = solve_a2(a2)?;
    let mut z = z_zeros();
    set_tap1(&mut z, t1, v1);
    set_tap2(&mut z, t2, v2);
    Some(z)
}

/// Like `z_for_accumulators` but panics with a clear message, for the named
/// `ERRORS.md` rows where the exact accumulator value is the point of the test.
pub fn z_for_accumulators_exact(a1: f32, a2: f32) -> Vec<f32> {
    z_for_accumulators(a1, a2).unwrap_or_else(|| {
        panic!(
            "no single-tap solution for accumulators ({a1} [{:08x}], {a2} [{:08x}])",
            a1.to_bits(),
            a2.to_bits()
        )
    })
}

/// Reference model of the C's `mp3d_scale_pcm`, used only to *document*
/// expectations in the error-path tests. Assertions always compare C vs Rust;
/// this is a cross-check, never the source of truth.
pub fn expected_scale_pcm(sample: f32) -> i16 {
    if sample >= 32766.5 {
        return 32767;
    }
    if sample <= -32767.5 {
        return -32768;
    }
    let s = (sample + 0.5f32) as i32 as i16;
    s.wrapping_sub(i16::from(s < 0))
}
