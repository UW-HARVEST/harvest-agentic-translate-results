//! Differential tests for the C → Rust translation of `gaussian_kernel`.
//!
//! BOTH implementations are loaded as shared objects through `libloading` and
//! called only through their exported `gaussian_kernel` symbol — the Rust
//! function is never called directly, so the `#[no_mangle] extern "C"` wrapper
//! and the C ABI are part of what is under test.
//!
//! Every comparison is done on raw `u32` bit patterns of the whole allocation
//! (including guard words before/after the region the kernel may touch), so
//! `+0.0` vs `-0.0`, NaN payloads, and any extra/missing store are all caught.
//!
//! * Phase B rows live in `mod phase_b` (see `CONFIGS.md`).
//! * Phase C rows live in `mod phase_c` (see `ERRORS.md`).
//! * Phase D symbol parity lives in `mod phase_d_symbol_parity` (see `SYMBOLS.md`).

use libloading::{Library, Symbol};
use std::path::PathBuf;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// harness: locate + load both shared objects
// ---------------------------------------------------------------------------

type GaussianKernelFn = unsafe extern "C" fn(*mut f32, i32, f32);

const SYM: &[u8] = b"gaussian_kernel\0";

struct Impls {
    c: Library,
    rs: Library,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `c_src/build/lib<projectname>.so` — the project name is derived from the
/// parent directory name by `c_src/CMakeLists.txt`, so scan for it.
fn find_c_so() -> PathBuf {
    // Optional override, used to re-run the whole suite against a C library
    // built with different compiler flags (e.g. -O2) without touching c_src/.
    if let Some(p) = std::env::var_os("HARVEST_C_SO") {
        let p = PathBuf::from(p);
        assert!(p.is_file(), "HARVEST_C_SO={} is not a file", p.display());
        return p;
    }
    let dir = manifest_dir().join("c_src").join("build");
    let named = dir.join("libtranslated_rust.so");
    if named.is_file() {
        return named;
    }
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for e in entries.flatten() {
            let p = e.path();
            let name = p.file_name().unwrap_or_default().to_string_lossy().to_string();
            if name.starts_with("lib") && name.ends_with(".so") {
                return p;
            }
        }
    }
    panic!(
        "C shared object not found in {}. Build it with:\n  cd c_src && mkdir -p build && cd build \
         && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        dir.display()
    );
}

/// `target/<profile>/libgaussian_kernel_lib.so`, found relative to the test
/// executable (`target/<profile>/deps/<test>`).
fn find_rust_so() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    let deps = exe.parent().expect("deps dir");
    let profile = deps.parent().expect("profile dir");
    for d in [profile, deps] {
        let p = d.join("libgaussian_kernel_lib.so");
        if p.is_file() {
            return p;
        }
    }
    for profile in ["debug", "release"] {
        let p = manifest_dir()
            .join("target")
            .join(profile)
            .join("libgaussian_kernel_lib.so");
        if p.is_file() {
            return p;
        }
    }
    panic!("Rust cdylib libgaussian_kernel_lib.so not found; run `cargo build` first");
}

/// `cargo test` does **not** rebuild a `cdylib` (integration tests never link
/// it), so without this guard the suite would happily test a stale `.so` and
/// report success for a Rust source that no longer matches. Refuse to run if
/// either shared object is older than its sources.
fn assert_not_stale(so: &std::path::Path, sources: &[PathBuf], how_to_build: &str) {
    let so_mtime = std::fs::metadata(so)
        .and_then(|m| m.modified())
        .unwrap_or_else(|e| panic!("stat {}: {e}", so.display()));
    for src in sources {
        if let Ok(src_mtime) = std::fs::metadata(src).and_then(|m| m.modified()) {
            assert!(
                so_mtime >= src_mtime,
                "STALE ARTIFACT: {} is older than {}.\nRebuild it first:\n  {how_to_build}",
                so.display(),
                src.display()
            );
        }
    }
}

fn impls() -> &'static Impls {
    static I: OnceLock<Impls> = OnceLock::new();
    I.get_or_init(|| {
        let c_path = find_c_so();
        let rs_path = find_rust_so();
        let md = manifest_dir();
        assert_not_stale(
            &rs_path,
            &[md.join("src").join("lib.rs"), md.join("Cargo.toml")],
            "cargo build   # (cargo test alone does NOT rebuild the cdylib)",
        );
        assert_not_stale(
            &c_path,
            &[
                md.join("c_src").join("src").join("lib.c"),
                md.join("c_src").join("include").join("lib.h"),
            ],
            "cd c_src/build && cmake --build .",
        );
        unsafe {
            let c = Library::new(&c_path)
                .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", c_path.display()));
            let rs = Library::new(&rs_path)
                .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", rs_path.display()));
            Impls { c, rs }
        }
    })
}

fn kernels() -> (
    Symbol<'static, GaussianKernelFn>,
    Symbol<'static, GaussianKernelFn>,
) {
    let i = impls();
    unsafe {
        let c: Symbol<GaussianKernelFn> =
            i.c.get(SYM).expect("C .so does not export gaussian_kernel");
        let rs: Symbol<GaussianKernelFn> = i
            .rs
            .get(SYM)
            .expect("Rust .so does not export gaussian_kernel");
        (c, rs)
    }
}

// ---------------------------------------------------------------------------
// buffers + comparison
// ---------------------------------------------------------------------------

/// Fill pattern for untouched memory: a value the kernel can never produce.
const SENTINEL: u32 = 0xCAFE_BABE;
/// Guard words appended after the widest region the C code may write.
const GUARD: usize = 4;

/// Number of `float`s the C taps loop actually stores for `size`:
/// `2 * (size / 2) + 1` whenever the loop runs at all (`size >= -1`).
fn written_words(size: i32) -> usize {
    if size >= -1 {
        (2 * (i64::from(size) / 2) + 1) as usize
    } else {
        0
    }
}

fn words_for(size: i32) -> usize {
    written_words(size).max(1) + GUARD
}

/// One invocation into a freshly sentinel-filled allocation.
/// `lead` sentinel words are placed *before* the pointer handed to the library.
fn call_one(
    f: &Symbol<'static, GaussianKernelFn>,
    lead: usize,
    words: usize,
    size: i32,
    radius: f32,
) -> Vec<u32> {
    let mut buf: Vec<u32> = vec![SENTINEL; lead + words];
    unsafe {
        let dest = (buf.as_mut_ptr() as *mut f32).add(lead);
        f(dest, size, radius);
    }
    buf
}

fn describe(size: i32, radius: f32) -> String {
    format!(
        "size={size} radius={radius:e} (bits 0x{:08X})",
        radius.to_bits()
    )
}

fn compare(ctx: &str, size: i32, radius: f32, c: &[u32], r: &[u32]) {
    assert_eq!(c.len(), r.len(), "{ctx}: buffer length mismatch (harness bug)");
    if c == r {
        return;
    }
    let mut msgs = Vec::new();
    for (i, (cv, rv)) in c.iter().zip(r.iter()).enumerate() {
        if cv != rv {
            msgs.push(format!(
                "  word[{i}]: C=0x{cv:08X} ({}) Rust=0x{rv:08X} ({})",
                f32::from_bits(*cv),
                f32::from_bits(*rv)
            ));
            if msgs.len() >= 12 {
                msgs.push("  ...".to_string());
                break;
            }
        }
    }
    panic!(
        "{ctx}: C/Rust divergence for {}\n{}",
        describe(size, radius),
        msgs.join("\n")
    );
}

/// The workhorse: run both `.so`s with the same input and require byte equality.
fn check(ctx: &str, size: i32, radius: f32) {
    check_with_lead(ctx, 0, size, radius);
}

fn check_with_lead(ctx: &str, lead: usize, size: i32, radius: f32) {
    let (cf, rf) = kernels();
    let words = words_for(size);
    let c = call_one(&cf, lead, words, size, radius);
    let r = call_one(&rf, lead, words, size, radius);
    compare(ctx, size, radius, &c, &r);
}

// ---------------------------------------------------------------------------
// deterministic PRNG (SplitMix64)
// ---------------------------------------------------------------------------

const SEED: u64 = 0x5EED_1234_ABCD_9876;

struct Rng(u64);

impl Rng {
    fn new() -> Self {
        Rng(SEED)
    }
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    /// Uniform in [0, 1).
    fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }
    fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.unit()
    }
    fn int_range(&mut self, lo: i32, hi: i32) -> i32 {
        let span = (i64::from(hi) - i64::from(lo) + 1) as u64;
        (i64::from(lo) + (self.next_u64() % span) as i64) as i32
    }
    /// Random radius covering every magnitude class the C branches on.
    fn any_radius(&mut self) -> f32 {
        match self.next_u32() % 8 {
            0 => self.range(0.1, 10.0),
            1 => self.range(-10.0, -0.1),
            2 => self.range(10.0, 1.0e6),
            3 => self.range(1.0e-6, 1.0),
            4 => f32::from_bits(self.next_u32()),
            5 => SPECIAL_RADII[(self.next_u32() as usize) % SPECIAL_RADII.len()],
            6 => 2.0f32.powi(self.int_range(-40, 40)),
            _ => self.range(-1.0e6, 1.0e6),
        }
    }
}

/// Every radius value the C code treats specially (see CONFIGS.md / ERRORS.md).
const SPECIAL_RADII: &[f32] = &[
    0.0,
    -0.0,
    1.6, // == sigma  =>  rs == 1.0
    -1.6,
    1.0,
    -1.0,
    0.5,
    2.0,
    3.0,
    f32::EPSILON,
    f32::MIN_POSITIVE,
    -f32::MIN_POSITIVE,
    1.0e-45, // subnormal
    -1.0e-45,
    1.0e-30,
    1.0e30,
    f32::MAX,
    f32::MIN,
    f32::INFINITY,
    f32::NEG_INFINITY,
    f32::NAN,
    -f32::NAN,
    1.0e38,
    -1.0e38,
];

/// NaN bit patterns (quiet + signalling, both signs) built explicitly.
fn nan_patterns() -> Vec<f32> {
    [
        0x7FC0_0000u32, // canonical quiet NaN
        0xFFC0_0000,    // negative quiet NaN
        0x7F80_0001,    // signalling NaN
        0xFF80_0001,    // negative signalling NaN
        0x7FFF_FFFF,    // NaN, all payload bits set
        0x7FC0_1234,    // NaN with payload
    ]
    .iter()
    .map(|b| f32::from_bits(*b))
    .collect()
}

// ===========================================================================
// Phase B — valid-path differential tests (one test per CONFIGS.md row)
// ===========================================================================

mod phase_b {
    use super::*;

    #[test]
    fn cfg_01_odd_sizes_typical_radius() {
        let mut rng = Rng::new();
        for size in (1..=33).step_by(2) {
            for _ in 0..64 {
                let radius = rng.range(0.1, 10.0);
                check("cfg_01", size, radius);
            }
        }
    }

    #[test]
    fn cfg_02_even_sizes_typical_radius() {
        let mut rng = Rng::new();
        for size in (2..=34).step_by(2) {
            for _ in 0..64 {
                let radius = rng.range(0.1, 10.0);
                check("cfg_02", size, radius);
            }
        }
    }

    #[test]
    fn cfg_03_random_size_random_radius() {
        let mut rng = Rng::new();
        for _ in 0..4000 {
            let size = rng.int_range(-8, 512);
            let radius = rng.any_radius();
            check("cfg_03", size, radius);
        }
    }

    #[test]
    fn cfg_04_radius_equals_sigma() {
        // radius == sigma => rs == 1.0f exactly => x == r
        for size in 0..=33 {
            check("cfg_04", size, 1.6);
            check("cfg_04", size, -1.6);
        }
    }

    #[test]
    fn cfg_05_radius_below_one_tails_clamped() {
        let mut rng = Rng::new();
        for size in 1..=33 {
            for _ in 0..32 {
                let radius = rng.range(1.0e-3, 1.0);
                check("cfg_05", size, radius);
            }
        }
    }

    #[test]
    fn cfg_06_radius_large_no_clamping() {
        let mut rng = Rng::new();
        for size in 1..=33 {
            for _ in 0..32 {
                let radius = rng.range(10.0, 1.0e6);
                check("cfg_06", size, radius);
            }
        }
    }

    #[test]
    fn cfg_07_negative_radius() {
        let mut rng = Rng::new();
        for size in 1..=34 {
            for _ in 0..32 {
                let radius = rng.range(-1.0e6, -0.1);
                check("cfg_07", size, radius);
            }
        }
    }

    #[test]
    fn cfg_08_random_finite_bit_pattern_radius() {
        let mut rng = Rng::new();
        let mut done = 0;
        while done < 3000 {
            let radius = f32::from_bits(rng.next_u32());
            if !radius.is_finite() {
                continue;
            }
            let size = rng.int_range(-4, 64);
            check("cfg_08", size, radius);
            done += 1;
        }
    }

    #[test]
    fn cfg_09_random_any_bit_pattern_radius() {
        let mut rng = Rng::new();
        for _ in 0..3000 {
            let radius = f32::from_bits(rng.next_u32());
            let size = rng.int_range(-4, 64);
            check("cfg_09", size, radius);
        }
    }

    #[test]
    fn cfg_10_size_zero_and_one_all_radii() {
        for &size in &[0i32, 1] {
            for &radius in SPECIAL_RADII {
                check("cfg_10", size, radius);
            }
            for radius in nan_patterns() {
                check("cfg_10/nan", size, radius);
            }
        }
    }

    #[test]
    fn cfg_11_negative_sizes() {
        let mut rng = Rng::new();
        for &size in &[-1i32, -2, -3, -4, -5, -17, i32::MIN + 1, i32::MIN] {
            for &radius in SPECIAL_RADII {
                check("cfg_11", size, radius);
            }
            for _ in 0..32 {
                let radius = rng.any_radius();
                check("cfg_11/rand", size, radius);
            }
        }
    }

    #[test]
    fn cfg_12_guard_regions_preserved() {
        // Explicitly assert (a) C and Rust agree and (b) both leave the guard
        // words untouched -- i.e. exactly 2*(size/2)+1 words are written.
        let (cf, rf) = kernels();
        let mut rng = Rng::new();
        for size in -3..=40 {
            for _ in 0..16 {
                let radius = rng.range(0.2, 8.0);
                let lead = 3;
                let words = words_for(size);
                let c = call_one(&cf, lead, words, size, radius);
                let r = call_one(&rf, lead, words, size, radius);
                compare("cfg_12", size, radius, &c, &r);

                let written = written_words(size);
                for i in 0..lead {
                    assert_eq!(
                        c[i], SENTINEL,
                        "cfg_12: C wrote before dest at word {i} for {}",
                        describe(size, radius)
                    );
                    assert_eq!(
                        r[i], SENTINEL,
                        "cfg_12: Rust wrote before dest at word {i} for {}",
                        describe(size, radius)
                    );
                }
                for i in (lead + written)..c.len() {
                    assert_eq!(
                        c[i], SENTINEL,
                        "cfg_12: C wrote past word {} for {}",
                        i - lead,
                        describe(size, radius)
                    );
                    assert_eq!(
                        r[i], SENTINEL,
                        "cfg_12: Rust wrote past word {} for {}",
                        i - lead,
                        describe(size, radius)
                    );
                }
            }
        }
    }

    #[test]
    fn cfg_13_dest_offset_inside_buffer() {
        let mut rng = Rng::new();
        for lead in 0..8usize {
            for _ in 0..200 {
                let size = rng.int_range(-3, 96);
                let radius = rng.any_radius();
                check_with_lead("cfg_13", lead, size, radius);
            }
        }
    }

    #[test]
    fn cfg_14_unaligned_dest() {
        let (cf, rf) = kernels();
        let mut rng = Rng::new();
        for byte_off in 1..=3usize {
            for size in 1..=9 {
                for _ in 0..16 {
                    let radius = rng.range(0.2, 8.0);
                    let c = call_unaligned(&cf, byte_off, size, radius);
                    let r = call_unaligned(&rf, byte_off, size, radius);
                    assert_eq!(
                        c,
                        r,
                        "cfg_14: unaligned (byte_off={byte_off}) divergence for {}",
                        describe(size, radius)
                    );
                }
            }
        }
    }

    #[test]
    fn cfg_15_repeated_calls_same_buffer() {
        // Three calls in a row into one buffer: catches hidden static state and
        // proves partial-overwrite behaviour matches.
        let (cf, rf) = kernels();
        let mut rng = Rng::new();
        for _ in 0..300 {
            let plan: Vec<(i32, f32)> = (0..3)
                .map(|_| (rng.int_range(-3, 48), rng.any_radius()))
                .collect();
            let words = plan.iter().map(|(s, _)| words_for(*s)).max().unwrap() + GUARD;
            let mut cbuf: Vec<u32> = vec![SENTINEL; words];
            let mut rbuf: Vec<u32> = vec![SENTINEL; words];
            for (size, radius) in plan {
                unsafe {
                    cf(cbuf.as_mut_ptr() as *mut f32, size, radius);
                    rf(rbuf.as_mut_ptr() as *mut f32, size, radius);
                }
                compare("cfg_15", size, radius, &cbuf, &rbuf);
            }
        }
    }

    #[test]
    fn cfg_16_large_sizes() {
        let mut rng = Rng::new();
        for &size in &[255i32, 256, 257, 1023, 1024, 4095, 4096, 65535, 65536, 65537] {
            for _ in 0..8 {
                let radius = rng.range(0.5, 5000.0);
                check("cfg_16", size, radius);
            }
            check("cfg_16/inf", size, f32::INFINITY);
            check("cfg_16/zero", size, 0.0);
            check("cfg_16/sigma", size, 1.6);
        }
    }

    #[test]
    fn cfg_17_all_taps_clamped_no_normalisation() {
        // rs = sigma/radius overflows to +/-inf => centre tap is 0*inf = NaN and
        // every other tap underflows => sum == 0 => normalisation skipped.
        // Requires |radius| < 1.6 / f32::MAX (~4.7e-39), i.e. deep subnormals.
        for &radius in &[
            1.0e-45f32,
            -1.0e-45,
            f32::from_bits(1),
            f32::from_bits(0x8000_0001),
            4.0e-39,
            0.0,
            -0.0,
            f32::NAN,
        ] {
            for size in 0..=17 {
                check("cfg_17", size, radius);
            }
        }
    }

    #[test]
    fn cfg_18_power_of_two_radii() {
        for e in -30..=30 {
            let radius = 2.0f32.powi(e);
            for size in 1..=17 {
                check("cfg_18", size, radius);
                check("cfg_18/neg", size, -radius);
            }
        }
    }

    #[test]
    fn cfg_19_extreme_radii() {
        // Includes both sides of the `rs = sigma / radius` overflow threshold
        // (1.6 / f32::MAX ~= 4.7e-39): just above it `rs` stays finite and the
        // centre tap survives; just below it `rs` becomes +/-inf.
        for &radius in &[
            f32::MIN_POSITIVE,
            -f32::MIN_POSITIVE,
            1.0e-45f32,
            5.0e-39,
            4.7e-39,
            4.0e-39,
            1.0e-39,
            f32::MAX,
            f32::MIN,
            1.0e38,
            -1.0e38,
            f32::EPSILON,
        ] {
            for size in 0..=9 {
                check("cfg_19", size, radius);
            }
        }
    }

    #[test]
    fn cfg_20_size_radius_cross_product() {
        for size in -4..=40 {
            for &radius in SPECIAL_RADII {
                check("cfg_20", size, radius);
            }
        }
    }
}

/// Unaligned variant of `call_one`, operating on a byte buffer.
fn call_unaligned(
    f: &Symbol<'static, GaussianKernelFn>,
    byte_off: usize,
    size: i32,
    radius: f32,
) -> Vec<u8> {
    const PAT: [u8; 4] = [0xBE, 0xBA, 0xFE, 0xCA];
    let total = byte_off + words_for(size) * 4 + 8;
    let mut buf: Vec<u8> = (0..total).map(|i| PAT[i % 4]).collect();
    unsafe {
        let dest = buf.as_mut_ptr().add(byte_off) as *mut f32;
        f(dest, size, radius);
    }
    buf
}

// ===========================================================================
// Phase C — error-path differential tests (one test per ERRORS.md row)
// ===========================================================================

mod phase_c {
    use super::*;

    /// Row 1: negative taps must be clamped to exactly +0.0 by both.
    #[test]
    fn err_01_negative_tap_clamped_to_zero() {
        let (cf, rf) = kernels();
        // radius small => everything but the centre is below s2 => clamped.
        let size = 21;
        let radius = 0.05f32;
        let words = words_for(size);
        let c = call_one(&cf, 0, words, size, radius);
        let r = call_one(&rf, 0, words, size, radius);
        compare("err_01", size, radius, &c, &r);
        // The clamp really fired, and produced +0.0 (0x00000000), not -0.0.
        let zeros = c[..written_words(size)]
            .iter()
            .filter(|w| **w == 0x0000_0000)
            .count();
        assert!(
            zeros > 0,
            "err_01: expected the negative-tap clamp to fire, buffer = {c:08X?}"
        );
        assert!(
            !c[..written_words(size)].contains(&0x8000_0000),
            "err_01: C never stores -0.0"
        );
    }

    /// Row 2: NaN taps are clamped to +0.0 (comiss/jbe semantics), not stored.
    #[test]
    fn err_02_nan_tap_clamped_to_zero() {
        let (cf, rf) = kernels();
        for radius in nan_patterns() {
            for size in [1i32, 2, 5, 8, 33] {
                let words = words_for(size);
                let c = call_one(&cf, 0, words, size, radius);
                let r = call_one(&rf, 0, words, size, radius);
                compare("err_02", size, radius, &c, &r);
                for (i, w) in c[..written_words(size)].iter().enumerate() {
                    assert_eq!(
                        *w, 0x0000_0000,
                        "err_02: C must clamp NaN tap {i} to +0.0 for {}",
                        describe(size, radius)
                    );
                }
            }
        }
    }

    /// Row 3: sum == 0 (all taps clamped) => normalisation skipped, no inf/NaN.
    ///
    /// `sum` can only reach 0 when the centre tap itself is clamped, which needs
    /// `rs = sigma / radius` to be non-finite. That requires `radius` to be NaN
    /// or below `1.6 / f32::MAX` (~4.7e-39) -- note `f32::MIN_POSITIVE`
    /// (1.175e-38) is NOT small enough: it yields a finite `rs = 1.36e38`, so
    /// its centre tap survives (covered by `cfg_19_extreme_radii` instead).
    #[test]
    fn err_03_sum_zero_skips_normalisation() {
        let (cf, rf) = kernels();
        for &radius in &[
            0.0f32,
            -0.0,
            1.0e-45,
            -1.0e-45,
            f32::from_bits(1),
            f32::from_bits(0x8000_0001),
            f32::NAN,
        ] {
            for size in 0..=16 {
                let words = words_for(size);
                let c = call_one(&cf, 0, words, size, radius);
                let r = call_one(&rf, 0, words, size, radius);
                compare("err_03", size, radius, &c, &r);
                for (i, w) in c[..written_words(size)].iter().enumerate() {
                    assert_eq!(
                        *w, 0x0000_0000,
                        "err_03: expected all-zero unnormalised buffer, word {i} for {}",
                        describe(size, radius)
                    );
                }
            }
        }
    }

    /// Row 4: size <= -2 is a complete no-op in both.
    #[test]
    fn err_04_size_le_minus2_is_noop() {
        let (cf, rf) = kernels();
        let mut rng = Rng::new();
        for &size in &[-2i32, -3, -4, -100, -1_000_000, i32::MIN + 1, i32::MIN] {
            for _ in 0..24 {
                let radius = rng.any_radius();
                let words = 8;
                let c = call_one(&cf, 2, words, size, radius);
                let r = call_one(&rf, 2, words, size, radius);
                compare("err_04", size, radius, &c, &r);
                assert!(
                    c.iter().all(|w| *w == SENTINEL),
                    "err_04: C must not write anything for {}",
                    describe(size, radius)
                );
                assert!(
                    r.iter().all(|w| *w == SENTINEL),
                    "err_04: Rust must not write anything for {}",
                    describe(size, radius)
                );
            }
        }
    }

    /// Rows 5 + 6: NULL dest survives iff the taps loop never runs (size <= -2).
    /// The `size >= -1` NULL case is UB (SIGSEGV) in *both* implementations and
    /// is deliberately not executed -- crashing the harness proves nothing.
    #[test]
    fn err_05_null_dest_with_noop_size() {
        let (cf, rf) = kernels();
        for &size in &[-2i32, -7, -12345, i32::MIN] {
            for &radius in &[1.0f32, 0.0, f32::NAN, f32::INFINITY, -3.5] {
                unsafe {
                    cf(std::ptr::null_mut(), size, radius);
                    rf(std::ptr::null_mut(), size, radius);
                }
            }
        }
        // Reaching here means neither implementation dereferenced NULL.
    }

    /// Row 7: size == -1 writes exactly one *unnormalised* element.
    #[test]
    fn err_07_size_minus1_writes_one_unnormalised() {
        let (cf, rf) = kernels();
        for &radius in SPECIAL_RADII {
            let words = words_for(-1);
            let c = call_one(&cf, 1, words, -1, radius);
            let r = call_one(&rf, 1, words, -1, radius);
            compare("err_07", -1, radius, &c, &r);
            assert_eq!(c[0], SENTINEL, "err_07: no write before dest");
            for (i, w) in c.iter().enumerate().skip(2) {
                assert_eq!(*w, SENTINEL, "err_07: only dest[0] may be written (word {i})");
            }
        }
        // With a well-behaved radius the single tap is 1.0 - s2, i.e. NOT 1.0
        // (no normalisation happened because the `r < size` loop never ran).
        let words = words_for(-1);
        let c = call_one(&cf, 0, words, -1, 2.0);
        let one_minus_s2 = f32::from_bits(c[0]);
        assert!(
            one_minus_s2 < 1.0 && one_minus_s2 > 0.99,
            "err_07: expected unnormalised 1.0-s2, got {one_minus_s2}"
        );
    }

    /// Row 8: size == 0 still writes dest[0] (one past a zero-length buffer).
    #[test]
    fn err_08_size_zero_writes_one_past_end() {
        let (cf, rf) = kernels();
        for &radius in SPECIAL_RADII {
            let words = words_for(0);
            let c = call_one(&cf, 1, words, 0, radius);
            let r = call_one(&rf, 1, words, 0, radius);
            compare("err_08", 0, radius, &c, &r);
        }
        let words = words_for(0);
        let c = call_one(&cf, 0, words, 0, 2.0);
        let r = call_one(&rf, 0, words, 0, 2.0);
        assert_ne!(c[0], SENTINEL, "err_08: C writes dest[0] even for size == 0");
        assert_eq!(c[0], r[0], "err_08: Rust must write the same dest[0]");
        assert_eq!(&c[1..], &r[1..], "err_08: nothing else may differ");
    }

    /// Row 9: even size overruns the buffer by exactly one unnormalised float.
    #[test]
    fn err_09_even_size_overruns_by_one() {
        let (cf, rf) = kernels();
        let mut rng = Rng::new();
        for size in (2..=32).step_by(2) {
            for _ in 0..16 {
                let radius = rng.range(0.5, 6.0);
                let words = words_for(size);
                let c = call_one(&cf, 0, words, size, radius);
                let r = call_one(&rf, 0, words, size, radius);
                compare("err_09", size, radius, &c, &r);
                let n = size as usize;
                assert_eq!(written_words(size), n + 1, "harness: even size writes size+1");
                assert_ne!(
                    c[n], SENTINEL,
                    "err_09: C must write the stray element at index {n} for {}",
                    describe(size, radius)
                );
                assert_eq!(
                    c[n + 1], SENTINEL,
                    "err_09: C must not write past index {n}"
                );
                assert_eq!(
                    r[n + 1], SENTINEL,
                    "err_09: Rust must not write past index {n}"
                );
            }
        }
    }

    /// Row 10: radius == 0.0f -- no divide-by-zero guard exists.
    #[test]
    fn err_10_radius_zero() {
        for size in -3..=24 {
            check("err_10", size, 0.0);
        }
    }

    /// Row 11: radius == -0.0f.
    #[test]
    fn err_11_radius_negative_zero() {
        for size in -3..=24 {
            check("err_11", size, -0.0);
        }
    }

    /// Row 12: NaN radius, every NaN encoding.
    #[test]
    fn err_12_radius_nan() {
        for radius in nan_patterns() {
            for size in -3..=24 {
                check("err_12", size, radius);
            }
        }
    }

    /// Row 13: +/-inf radius => flat kernel, all in-range taps == 1/(2*hsize+1).
    #[test]
    fn err_13_radius_infinite() {
        let (cf, rf) = kernels();
        for &radius in &[f32::INFINITY, f32::NEG_INFINITY] {
            for size in -3..=33 {
                let words = words_for(size);
                let c = call_one(&cf, 0, words, size, radius);
                let r = call_one(&rf, 0, words, size, radius);
                compare("err_13", size, radius, &c, &r);
            }
        }
        // Spot-check the documented value for an odd size.
        let words = words_for(9);
        let c = call_one(&cf, 0, words, 9, f32::INFINITY);
        for w in &c[..9] {
            let v = f32::from_bits(*w);
            assert!(
                (v - 1.0 / 9.0).abs() < 1e-6,
                "err_13: expected flat 1/9 kernel, got {v}"
            );
        }
    }

    /// Row 14: subnormal/tiny radius makes sigma/radius overflow to +/-inf.
    #[test]
    fn err_14_radius_subnormal_overflow() {
        for &radius in &[
            f32::MIN_POSITIVE,
            -f32::MIN_POSITIVE,
            1.0e-45f32,
            -1.0e-45,
            f32::from_bits(1),
            f32::from_bits(0x8000_0001),
        ] {
            for size in -3..=24 {
                check("err_14", size, radius);
            }
        }
    }

    /// Row 15: huge radius => rs underflows, kernel flat, no overflow trap.
    #[test]
    fn err_15_radius_huge() {
        for &radius in &[f32::MAX, f32::MIN, 1.0e38f32, -1.0e38, 3.0e38] {
            for size in -3..=24 {
                check("err_15", size, radius);
            }
        }
    }

    /// Row 16: size == i32::MIN (no overflow on `size / 2` or `-hsize`).
    #[test]
    fn err_16_size_int_min() {
        for &radius in SPECIAL_RADII {
            check("err_16", i32::MIN, radius);
            check("err_16", i32::MIN + 1, radius);
        }
    }

    /// Row 17: one step past every interesting size boundary.
    #[test]
    fn err_17_size_boundary_sweep() {
        let mut rng = Rng::new();
        for size in -3..=3 {
            for &radius in SPECIAL_RADII {
                check("err_17", size, radius);
            }
            for _ in 0..64 {
                let radius = rng.any_radius();
                check("err_17/rand", size, radius);
            }
        }
    }

    /// Row 18: arbitrary/garbage `int` values across the FFI boundary (the
    /// stand-in for out-of-range enum values -- the C API has no enum, so `int
    /// size` is the only unconstrained integer domain).
    #[test]
    fn err_18_arbitrary_int_size_values() {
        let mut rng = Rng::new();
        // Random values from the whole i32 domain: only the memory-safe ones
        // (<= a bounded positive size, or <= -2) can be executed; huge positive
        // sizes are clamped into a safe range, which is exactly row 19.
        for _ in 0..2000 {
            let raw = rng.next_u32() as i32;
            let size = if raw > 4096 { raw % 4096 } else { raw };
            let radius = rng.any_radius();
            check("err_18", size, radius);
        }
        for &size in &[
            i32::MIN,
            i32::MIN + 1,
            -0x4000_0000,
            -0x3FFF_FFFF,
            -65_537,
            -2,
            -1,
            0,
            1,
        ] {
            for &radius in SPECIAL_RADII {
                check("err_18/edge", size, radius);
            }
        }
    }

    /// Row 20: unaligned `dest` (no alignment check in the C).
    #[test]
    fn err_20_unaligned_dest() {
        let (cf, rf) = kernels();
        for byte_off in 1..=3usize {
            for &size in &[-2i32, -1, 0, 1, 2, 7, 8, 33] {
                for &radius in &[1.6f32, 0.0, f32::NAN, f32::INFINITY, 0.05, -2.5] {
                    let c = call_unaligned(&cf, byte_off, size, radius);
                    let r = call_unaligned(&rf, byte_off, size, radius);
                    assert_eq!(
                        c,
                        r,
                        "err_20: unaligned (byte_off={byte_off}) divergence for {}",
                        describe(size, radius)
                    );
                }
            }
        }
    }
}

// ===========================================================================
// Phase D — symbol parity, enforced automatically
// ===========================================================================

mod phase_d_symbol_parity {
    use super::*;
    use std::process::Command;

    fn dynamic_defined(path: &std::path::Path) -> Vec<String> {
        let out = Command::new("nm")
            .args(["-D", "--defined-only", path.to_str().unwrap()])
            .output()
            .expect("failed to run nm");
        assert!(out.status.success(), "nm failed on {}", path.display());
        let mut syms: Vec<String> = String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|l| {
                let mut it = l.split_whitespace();
                let (_addr, kind, name) = (it.next()?, it.next()?, it.next()?);
                // Skip loader/CRT-provided weak symbols; keep real definitions.
                if kind == "w" || kind == "V" {
                    return None;
                }
                Some(name.to_string())
            })
            .filter(|n| {
                !matches!(
                    n.as_str(),
                    "_init"
                        | "_fini"
                        | "_edata"
                        | "_end"
                        | "__bss_start"
                        | "_ITM_registerTMCloneTable"
                        | "_ITM_deregisterTMCloneTable"
                        | "__cxa_finalize"
                        | "__gmon_start__"
                )
            })
            .collect();
        syms.sort();
        syms.dedup();
        syms
    }

    #[test]
    fn c_symbols_are_all_exported_by_rust() {
        let c_syms = dynamic_defined(&find_c_so());
        let rust_syms = dynamic_defined(&find_rust_so());
        assert!(
            !c_syms.is_empty(),
            "sanity: the C .so must export at least one symbol"
        );
        let missing: Vec<&String> = c_syms.iter().filter(|s| !rust_syms.contains(s)).collect();
        assert!(
            missing.is_empty(),
            "Rust .so is missing {} C symbol(s): {missing:?}\nC exports: {c_syms:?}",
            missing.len()
        );
        assert!(
            c_syms.iter().any(|s| s == "gaussian_kernel"),
            "expected gaussian_kernel in the C export list, got {c_syms:?}"
        );
    }

    #[test]
    fn rust_so_has_no_unresolved_non_libc_symbols() {
        let path = find_rust_so();
        let out = Command::new("nm")
            .args(["-D", "--undefined-only", path.to_str().unwrap()])
            .output()
            .expect("failed to run nm");
        assert!(out.status.success());
        let text = String::from_utf8_lossy(&out.stdout);
        let suspicious: Vec<&str> = text
            .lines()
            .filter_map(|l| l.split_whitespace().last())
            .filter(|n| {
                // Everything the platform provides: glibc versioned symbols,
                // libgcc unwinder, ELF/CRT weak hooks.
                !n.contains('@')
                    && !n.starts_with("_Unwind_")
                    && !n.starts_with("_ITM_")
                    && !n.starts_with("__gmon_start__")
            })
            .collect();
        assert!(
            suspicious.is_empty(),
            "Rust .so has unresolved non-libc symbols: {suspicious:?}"
        );
    }
}
