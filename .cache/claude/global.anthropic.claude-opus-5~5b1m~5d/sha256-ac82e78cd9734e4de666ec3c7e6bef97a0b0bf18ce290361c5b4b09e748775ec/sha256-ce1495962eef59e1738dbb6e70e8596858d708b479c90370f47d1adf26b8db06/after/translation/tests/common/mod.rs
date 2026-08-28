//! Shared harness for the C-vs-Rust differential tests.
//!
//! Both libraries are loaded through `libloading` (i.e. `dlopen`/`dlsym`), so
//! the Rust side is exercised strictly through its exported `#[no_mangle]`
//! `extern "C"` surface — never by calling Rust functions directly.

#![allow(dead_code)]

use std::ffi::c_int;
use std::path::PathBuf;
use std::sync::OnceLock;

use libloading::{Library, Symbol};

/// `void synth_pair(mp3d_sample_t *pcm, int nch, const float *z);`
pub type SynthPairFn = unsafe extern "C" fn(*mut i16, c_int, *const f32);

/// Number of `float`s the C code may legally read from `z`.
///
/// Lane 0 reads `z[k * 64]` for every `k in 0..=14` (max index `896`).
/// Lane 1 reads `z[2 + k * 64]` for the **even** `k in {0,2,4,6,8,10,12,14}`
/// (max index `898`).
/// Hence the minimum legal extent is `899` floats.
pub const Z_MIN_LEN: usize = 14 * 64 + 2 + 1; // 899

/// Indices of `z` that lane 0 actually reads: `z[k * 64]` for every `k in 0..=14`.
pub const LANE0_TAPS: [usize; 15] = [
    0, 64, 128, 192, 256, 320, 384, 448, 512, 576, 640, 704, 768, 832, 896,
];
/// Indices of `z` that lane 1 actually reads: after `z += 2` it uses only the
/// **even** multipliers `k in {0,2,4,6,8,10,12,14}`, so 8 taps at `2 + k * 64`.
pub const LANE1_TAPS: [usize; 8] = [2, 130, 258, 386, 514, 642, 770, 898];

/// All 23 distinct indices read by `synth_pair`, ascending.
pub fn all_taps() -> Vec<usize> {
    let mut v: Vec<usize> = LANE0_TAPS.iter().chain(LANE1_TAPS.iter()).copied().collect();
    v.sort_unstable();
    v
}

/// Indices in `0..Z_MIN_LEN` that `synth_pair` never reads.
pub fn unread_indices() -> Vec<usize> {
    let taps = all_taps();
    (0..Z_MIN_LEN).filter(|i| !taps.contains(i)).collect()
}

/// Lane-0 term list in **source order**: `(z index, weight, subtract?)`.
///
/// Mirrors `lib.c:15-22`; the pairs that appear inside one C statement are
/// adjacent here.
pub const LANE0_TERMS: &[(usize, f32, bool)] = &[
    (896, 29.0, false),
    (0, 29.0, true),
    (64, 213.0, false),
    (832, 213.0, false),
    (768, 459.0, false),
    (128, 459.0, true),
    (192, 2037.0, false),
    (704, 2037.0, false),
    (640, 5153.0, false),
    (256, 5153.0, true),
    (320, 6574.0, false),
    (576, 6574.0, false),
    (512, 37489.0, false),
    (384, 37489.0, true),
    (448, 75038.0, false),
];

/// Lane-1 term list in source order: `(z index, weight)` — mirrors `lib.c:25-32`.
pub const LANE1_TERMS: &[(usize, f32)] = &[
    (898, 104.0),
    (770, 1567.0),
    (642, 9727.0),
    (514, 64019.0),
    (386, -9975.0),
    (258, -45.0),
    (130, 146.0),
    (2, -5.0),
];

// ---------------------------------------------------------------------------
// Reference model — used ONLY to construct inputs that hit a chosen
// accumulator value. Outputs are always judged by the C/Rust differential.
// ---------------------------------------------------------------------------

/// Lane-0 accumulator, evaluated in exactly the C's operation order.
pub fn model_lane0(z: &[f32]) -> f32 {
    let mut a: f32 = (z[896] - z[0]) * 29.0;
    a += (z[64] + z[832]) * 213.0;
    a += (z[768] - z[128]) * 459.0;
    a += (z[192] + z[704]) * 2037.0;
    a += (z[640] - z[256]) * 5153.0;
    a += (z[320] + z[576]) * 6574.0;
    a += (z[512] - z[384]) * 37489.0;
    a += z[448] * 75038.0;
    a
}

/// Lane-1 accumulator, evaluated in exactly the C's operation order.
pub fn model_lane1(z: &[f32]) -> f32 {
    let mut a: f32 = z[898] * 104.0;
    a += z[770] * 1567.0;
    a += z[642] * 9727.0;
    a += z[514] * 64019.0;
    a += z[386] * -9975.0;
    a += z[258] * -45.0;
    a += z[130] * 146.0;
    a += z[2] * -5.0;
    a
}

fn nudge(x: f32, steps: i32) -> f32 {
    // Step `x` by `steps` representable f32 values (monotone in the ordered
    // bit representation for a fixed sign).
    let mut bits = x.to_bits() as i64;
    if x.is_sign_negative() {
        bits -= steps as i64;
    } else {
        bits += steps as i64;
    }
    f32::from_bits(bits as u32)
}

/// Builds a `z` buffer whose **lane-0** accumulator is bit-exactly `target`.
///
/// Uses only two taps: `z[512]` (weight `37489`, coarse) and `z[448]`
/// (weight `75038`, the last term, fine correction). Returns `None` if the
/// search fails, which the callers assert against.
pub fn z_for_lane0_exact(target: f32) -> Option<Vec<f32>> {
    if !target.is_finite() {
        // Infinities are reachable directly with a huge coarse tap.
        let mut z = zeros_z();
        z[512] = if target > 0.0 { f32::MAX } else { -f32::MAX };
        z[448] = if target > 0.0 { f32::MAX } else { -f32::MAX };
        if target.is_nan() {
            z[512] = f32::MAX;
            z[448] = -f32::MAX; // inf + -inf -> NaN
        }
        return if model_lane0(&z).to_bits() == target.to_bits()
            || (target.is_nan() && model_lane0(&z).is_nan())
        {
            Some(z)
        } else {
            None
        };
    }
    if target == 0.0 {
        return Some(zeros_z());
    }

    let coarse = target / 37489.0;
    for cs in -24i32..=24 {
        let mut z = zeros_z();
        z[512] = nudge(coarse, cs);
        let a_prev = model_lane0(&z);
        if !a_prev.is_finite() {
            continue;
        }
        if a_prev.to_bits() == target.to_bits() {
            return Some(z);
        }
        let residual = target - a_prev;
        if residual == 0.0 {
            continue;
        }
        let fine = residual / 75038.0;
        for fs in -160i32..=160 {
            z[448] = nudge(fine, fs);
            if model_lane0(&z).to_bits() == target.to_bits() {
                return Some(z);
            }
        }
    }
    None
}

/// Builds a `z` buffer whose **lane-1** accumulator is bit-exactly `target`
/// while leaving lane 0 at `0.0`.
///
/// Uses `z[514]` (weight `64019`, coarse) and `z[2]` (weight `-5`, the last
/// term, fine correction). Neither index is a lane-0 tap, so lane 0 stays 0.
pub fn z_for_lane1_exact(target: f32) -> Option<Vec<f32>> {
    if !target.is_finite() {
        let mut z = zeros_z();
        if target.is_nan() {
            z[514] = f32::MAX;
            z[2] = f32::MAX; // 64019*MAX = inf, then inf + (-5*MAX = -inf) -> NaN
            return if model_lane1(&z).is_nan() { Some(z) } else { None };
        }
        z[514] = if target > 0.0 { f32::MAX } else { -f32::MAX };
        return if model_lane1(&z).to_bits() == target.to_bits() {
            Some(z)
        } else {
            None
        };
    }
    if target == 0.0 {
        return Some(zeros_z());
    }

    let coarse = target / 64019.0;
    for cs in -24i32..=24 {
        let mut z = zeros_z();
        z[514] = nudge(coarse, cs);
        let a_prev = model_lane1(&z);
        if !a_prev.is_finite() {
            continue;
        }
        if a_prev.to_bits() == target.to_bits() {
            return Some(z);
        }
        let residual = target - a_prev;
        if residual == 0.0 {
            continue;
        }
        let fine = residual / -5.0;
        for fs in -160i32..=160 {
            z[2] = nudge(fine, fs);
            if model_lane1(&z).to_bits() == target.to_bits() {
                return Some(z);
            }
        }
    }
    None
}

/// One f32 ULP below `x` (for `x > 0`).
pub fn prev_f32(x: f32) -> f32 {
    nudge(x, -1)
}
/// One f32 ULP above `x` (in magnitude direction of `x`'s sign for negatives:
/// `next_f32(-32767.5)` is the value *closer to zero*).
pub fn next_f32(x: f32) -> f32 {
    nudge(x, 1)
}

pub struct Impl {
    pub name: &'static str,
    #[allow(unused)]
    lib: Library,
    pub synth_pair: SynthPairFn,
}

impl Impl {
    unsafe fn open(name: &'static str, path: &PathBuf) -> Impl {
        let lib = unsafe {
            Library::new(path)
                .unwrap_or_else(|e| panic!("failed to dlopen {} ({}): {e}", name, path.display()))
        };
        let sym: Symbol<SynthPairFn> = unsafe {
            lib.get(b"synth_pair\0")
                .unwrap_or_else(|e| panic!("`synth_pair` missing from {} .so: {e}", name))
        };
        let synth_pair = *sym;
        Impl {
            name,
            lib,
            synth_pair,
        }
    }
}

pub struct Pair {
    pub c: Impl,
    pub rust: Impl,
}

static PAIR: OnceLock<Pair> = OnceLock::new();

pub fn c_so_path() -> PathBuf {
    PathBuf::from(env!("C_SO_PATH"))
}

fn mtime(p: &std::path::Path) -> Option<std::time::SystemTime> {
    std::fs::metadata(p).ok()?.modified().ok()
}

pub fn rust_so_path() -> PathBuf {
    let dir = PathBuf::from(env!("RUST_SO_DIR"));
    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs");
    let candidates = ["libsynth_pair_lib.so", "synth_pair_lib.so"];
    for c in candidates {
        let p = dir.join(c);
        if p.is_file() {
            // Never silently test a stale artifact: `cargo test` does not
            // rebuild a cdylib-only lib target, so an old `.so` left behind by
            // a previous `cargo build` would be loaded instead of the current
            // source.
            match (mtime(&p), mtime(&src)) {
                (Some(so), Some(s)) if so < s => panic!(
                    "STALE Rust cdylib: {} is older than {}.\n\
                     Run `cargo build{}` (or use ./run_all_feature_combos.sh) \
                     before `cargo test`.",
                    p.display(),
                    src.display(),
                    if cfg!(debug_assertions) { "" } else { " --release" }
                ),
                _ => {}
            }
            return p;
        }
    }
    // `cargo test` alone does not emit the cdylib artifact for a cdylib-only
    // package; build.rs compiles an identical one as a fallback.
    let fallback = PathBuf::from(env!("FALLBACK_RUST_SO"));
    if fallback.is_file() {
        return fallback;
    }
    panic!(
        "could not find the Rust cdylib in {} nor at {}. Run `cargo build` for \
         the same profile before `cargo test`.\nContents: {:?}",
        dir.display(),
        fallback.display(),
        std::fs::read_dir(&dir)
            .map(|rd| rd.filter_map(|e| e.ok().map(|e| e.file_name())).collect::<Vec<_>>())
            .unwrap_or_default()
    );
}

pub fn pair() -> &'static Pair {
    PAIR.get_or_init(|| unsafe {
        Pair {
            c: Impl::open("C", &c_so_path()),
            rust: Impl::open("Rust", &rust_so_path()),
        }
    })
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seeds for reproducibility.
// ---------------------------------------------------------------------------

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
    /// Uniform in `[0, n)`.
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % (n as u64)) as usize
    }
    /// Uniform in `[0, 1)`.
    pub fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }
    /// Uniform in `[-1, 1)`.
    pub fn signed_unit(&mut self) -> f32 {
        self.unit() * 2.0 - 1.0
    }
    /// A completely arbitrary 32-bit pattern reinterpreted as `f32`
    /// (NaNs of every payload, infinities, subnormals, huge values...).
    pub fn any_bits_f32(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }
    /// Random sign times a random magnitude spread across many binades.
    pub fn wide_exponent_f32(&mut self, min_exp: i32, max_exp: i32) -> f32 {
        let span = (max_exp - min_exp + 1) as usize;
        let e = min_exp + self.below(span) as i32;
        let m = 1.0f32 + self.unit();
        let sign = if self.next_u64() & 1 == 0 { 1.0 } else { -1.0 };
        sign * m * (2.0f32).powi(e)
    }
}

/// Interesting exact `f32` values, including every guard boundary from the C.
pub const BOUNDARY_POOL: &[f32] = &[
    0.0,
    -0.0,
    1.0,
    -1.0,
    0.5,
    -0.5,
    f32::MIN_POSITIVE,
    -f32::MIN_POSITIVE,
    1e-40, // subnormal
    -1e-40,
    f32::EPSILON,
    -f32::EPSILON,
    32766.5,
    -32766.5,
    32767.5,
    -32767.5,
    32768.0,
    -32768.0,
    f32::MAX,
    f32::MIN,
    f32::INFINITY,
    f32::NEG_INFINITY,
    f32::NAN,
    -f32::NAN,
    1.0 / 3.0,
    -1.0 / 3.0,
    75038.0,
    -75038.0,
];

// ---------------------------------------------------------------------------
// Differential driver
// ---------------------------------------------------------------------------

/// Runs `synth_pair` from both `.so`s over identical inputs and compares the
/// **entire** `pcm` buffer bit-for-bit.
///
/// * `pcm_len`      – total elements in the `pcm` allocation.
/// * `pcm_offset`   – element index inside that allocation handed to the callee.
/// * `pcm_fill`     – value pre-filled into every element (so we can also prove
///                    that untouched elements are left alone by *both*).
/// * `z`            – the input buffer (>= [`Z_MIN_LEN`] elements from `z_offset`).
/// * `z_offset`     – element index inside `z` handed to the callee.
///
/// Returns the (identical) resulting buffer.
pub fn diff_call(
    ctx: &str,
    pcm_len: usize,
    pcm_offset: usize,
    pcm_fill: i16,
    nch: c_int,
    z: &[f32],
    z_offset: usize,
) -> Vec<i16> {
    let p = pair();

    assert!(
        z.len() >= z_offset + Z_MIN_LEN,
        "{ctx}: z too short ({} < {})",
        z.len(),
        z_offset + Z_MIN_LEN
    );

    // Harness self-check. Both stores must land inside the allocation, or the
    // callee corrupts memory *outside* `out_c` / `out_r` and the comparison
    // becomes meaningless (the two Vecs are at different addresses, so the
    // out-of-bounds writes hit different things and report a fake divergence).
    // The lane-1 index is computed with the C's wrapping `int` arithmetic so
    // the deliberate-overflow rows are checked against their real destination.
    let lane1 = pcm_offset as isize + 16i32.wrapping_mul(nch) as isize;
    assert!(
        pcm_offset < pcm_len,
        "{ctx}: TEST BUG — pcm_offset {pcm_offset} outside a {pcm_len}-element buffer"
    );
    assert!(
        lane1 >= 0 && (lane1 as usize) < pcm_len,
        "{ctx}: TEST BUG — the lane-1 store lands at index {lane1}, outside the \
         {pcm_len}-element pcm buffer (nch={nch}, pcm_offset={pcm_offset}). \
         Give the buffer more headroom/tail instead of corrupting memory."
    );

    let mut out_c = vec![pcm_fill; pcm_len];
    let mut out_r = vec![pcm_fill; pcm_len];

    unsafe {
        (p.c.synth_pair)(out_c.as_mut_ptr().add(pcm_offset), nch, z.as_ptr().add(z_offset));
        (p.rust.synth_pair)(out_r.as_mut_ptr().add(pcm_offset), nch, z.as_ptr().add(z_offset));
    }

    if out_c != out_r {
        let first = out_c
            .iter()
            .zip(out_r.iter())
            .position(|(a, b)| a != b)
            .unwrap();
        panic!(
            "{ctx}: DIVERGENCE at pcm[{first}] (nch={nch}, pcm_offset={pcm_offset})\n\
             C    = {}\n\
             Rust = {}\n\
             live z taps (index: bits/value):\n{}",
            out_c[first],
            out_r[first],
            dump_taps(z, z_offset),
        );
    }

    out_c
}

/// Convenience wrapper for the common "tight buffers" case.
pub fn diff_simple(ctx: &str, nch: c_int, z: &[f32]) -> Vec<i16> {
    // Room for pcm[0] and pcm[16*nch] for nch in 0..=8, plus slack to detect
    // stray writes.
    let pcm_len = 16 * 8 + 8;
    diff_call(ctx, pcm_len, 0, 0x5A5A_u16 as i16, nch, z, 0)
}

pub fn dump_taps(z: &[f32], z_offset: usize) -> String {
    let mut s = String::new();
    for i in all_taps() {
        let v = z[z_offset + i];
        s.push_str(&format!("  z[{i:3}] = 0x{:08x} ({v:e})\n", v.to_bits()));
    }
    s
}

/// A `z` buffer of exactly the minimum legal extent, all zeros.
pub fn zeros_z() -> Vec<f32> {
    vec![0.0f32; Z_MIN_LEN]
}

// ---------------------------------------------------------------------------
// Aliasing driver (E23): `pcm` points *into* the `z` buffer.
// ---------------------------------------------------------------------------

/// Calls `synth_pair` with `pcm` aliasing the `z` allocation at float index
/// `alias_at` and compares the resulting **whole buffer** (as raw bits).
///
/// The C signature has no `restrict`, so this is a legal (if unusual) call and
/// both implementations must agree, including on how lane 0's store perturbs
/// the floats lane 1 subsequently reads.
pub fn diff_call_aliased(ctx: &str, nch: c_int, z: &[f32], alias_at: usize) -> Vec<u32> {
    let p = pair();
    assert!(z.len() >= Z_MIN_LEN);
    // Harness self-check: both i16 stores must stay inside the float buffer.
    let last_i16 = alias_at as isize * 2 + 16i32.wrapping_mul(nch) as isize;
    assert!(
        alias_at * 2 < z.len() * 2 && last_i16 >= 0 && (last_i16 as usize) < z.len() * 2,
        "{ctx}: TEST BUG — aliased store at i16 index {last_i16} outside the \
         {}-element (i16) buffer (nch={nch}, alias_at={alias_at})",
        z.len() * 2
    );

    let run = |f: SynthPairFn| -> Vec<u32> {
        let mut buf = z.to_vec();
        let base = buf.as_mut_ptr();
        unsafe {
            let pcm = base.add(alias_at) as *mut i16;
            f(pcm, nch, base as *const f32);
        }
        buf.iter().map(|v| v.to_bits()).collect()
    };

    let bits_c = run(p.c.synth_pair);
    let bits_r = run(p.rust.synth_pair);
    if bits_c != bits_r {
        let i = bits_c
            .iter()
            .zip(bits_r.iter())
            .position(|(a, b)| a != b)
            .unwrap();
        panic!(
            "{ctx}: ALIASED DIVERGENCE at z[{i}] (nch={nch}, alias_at={alias_at}): \
             C=0x{:08x} Rust=0x{:08x}",
            bits_c[i], bits_r[i]
        );
    }
    bits_c
}

// ---------------------------------------------------------------------------
// Sub-process helper (E21): observe fatal-signal behaviour without taking the
// test runner down with it.
// ---------------------------------------------------------------------------

pub const CHILD_ENV: &str = "HARVEST_DIFF_CHILD";

/// Re-executes the current test binary with `HARVEST_DIFF_CHILD=<mode>` so the
/// child performs the fatal operation, and reports `(signal, exit_code)`.
pub fn run_child(test_name: &str, mode: &str) -> (Option<i32>, Option<i32>) {
    use std::os::unix::process::ExitStatusExt;

    let exe = std::env::current_exe().expect("current_exe");
    let out = std::process::Command::new(exe)
        .arg(test_name)
        .arg("--exact")
        .arg("--nocapture")
        .arg("--test-threads=1")
        .env(CHILD_ENV, mode)
        .output()
        .expect("spawn child test process");
    (out.status.signal(), out.status.code())
}

pub fn child_mode() -> Option<String> {
    std::env::var(CHILD_ENV).ok()
}

// ---------------------------------------------------------------------------
// Reference replay of the C `mp3d_scale_pcm`.
//
// Used (a) to cross-check the value the two `.so`s agree on and (b) as the
// baseline in `tests/equivalence.rs`. It is itself validated against the real C
// `.so` over all 2^32 single-tap inputs by
// `exhaustive_reference_model_matches_the_c_so`.
// ---------------------------------------------------------------------------

/// ```c
/// static int16_t mp3d_scale_pcm(float sample) {
///     if (sample >= 32766.5)  return (int16_t)32767;
///     if (sample <= -32767.5) return (int16_t)-32768;
///     int16_t s = (int16_t)(sample + .5f);
///     s -= (s < 0);
///     return s;
/// }
/// ```
pub fn c_scale_pcm_reference(sample: f32) -> i16 {
    if sample as f64 >= 32766.5 {
        return 32767;
    }
    if sample as f64 <= -32767.5 {
        return -32768;
    }
    let s = (sample + 0.5f32) as i32 as i16;
    s.wrapping_sub((s < 0) as i16)
}

/// Whether the test binary was built with optimisations.
///
/// The exhaustive sweeps use this (not `cfg!(debug_assertions)`) to pick a
/// stride, because `[profile.dev]` intentionally disables debug assertions so
/// the cdylib keeps the C's unchecked pointer semantics (see `ERRORS.md` E21).
pub fn optimized() -> bool {
    env!("HARVEST_OPT_LEVEL") != "0"
}
