//! Differential test harness: loads BOTH the C `.so` and the Rust `.so` with
//! `libloading` and compares `contrast_ratio` bit-for-bit across the whole
//! configuration surface described in `CONFIGS.md` / `ERRORS.md`.
//!
//! Nothing in this file calls the Rust crate directly — the Rust side is always
//! reached through `dlopen`/`dlsym` on the produced `cdylib`, exactly as an
//! external C consumer would, so the `#[no_mangle] extern "C"` wrapper is under
//! test too.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use libloading::{Library, Symbol};

// ---------------------------------------------------------------------------
// ABI mirror
// ---------------------------------------------------------------------------

/// Mirror of the C `cb_rgb_255` from `c_src/include/lib.h`.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CbRgb255 {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl CbRgb255 {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}

type ContrastRatioFn = unsafe extern "C" fn(CbRgb255, CbRgb255) -> f32;
/// Raw-register view of the same symbol, used to inject garbage into the
/// undefined upper bits of the argument registers (ERRORS.md row E7).
type ContrastRatioRawFn = unsafe extern "C" fn(u64, u64) -> f32;

// ---------------------------------------------------------------------------
// Library discovery
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    let build = workspace_root().join("c_src").join("build");
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&build) {
        for entry in entries.flatten() {
            let p = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("lib") && name.ends_with(".so") {
                candidates.push(p);
            }
        }
    }
    candidates.sort();
    candidates.into_iter().next().unwrap_or_else(|| {
        panic!(
            "no C shared object found in {}. Build it with:\n  cd c_src && mkdir -p build && cd build \\\n    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

/// Builds the `cdylib` from the CURRENT sources and returns its path.
///
/// This is not optional plumbing: `cargo test` does **not** rebuild a
/// `crate-type = ["cdylib"]` artifact as part of the test build, so simply
/// looking for `target/<profile>/libcontrast_ratio_lib.so` can silently load a
/// stale object left behind by an earlier `cargo build`. That would make every
/// differential assertion vacuous. We therefore invoke `cargo build --lib` into
/// a dedicated `CARGO_TARGET_DIR` (avoiding any lock contention with the
/// `cargo test` that is running us) and assert the result is newer than every
/// source file.
fn build_rust_so() -> PathBuf {
    static SO: OnceLock<PathBuf> = OnceLock::new();
    SO.get_or_init(|| {
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
        let target_dir = manifest.join("target").join("harness");
        let release = !cfg!(debug_assertions);

        let mut cmd = std::process::Command::new(
            std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()),
        );
        cmd.current_dir(manifest)
            .env("CARGO_TARGET_DIR", &target_dir)
            .args(["build", "--lib"]);
        if release {
            cmd.arg("--release");
        }
        // Propagate the feature selection the test binary itself was built with.
        // `Cargo.toml` currently declares no `[features]`, so this is normally a
        // no-op; the env hook keeps the harness correct if features are added.
        if let Ok(feats) = std::env::var("HARNESS_FEATURES") {
            cmd.arg("--no-default-features");
            if !feats.trim().is_empty() {
                cmd.args(["--features", feats.trim()]);
            }
        }

        let out = cmd.output().expect("spawn cargo build --lib for the harness");
        assert!(
            out.status.success(),
            "harness cargo build failed:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );

        let so = target_dir
            .join(if release { "release" } else { "debug" })
            .join("libcontrast_ratio_lib.so");
        assert!(
            so.is_file(),
            "harness build produced no {}",
            so.display()
        );

        // Freshness check: the object must be at least as new as every source
        // file, otherwise we are about to test something other than the code.
        let so_mtime = std::fs::metadata(&so).unwrap().modified().unwrap();
        for src in ["src/lib.rs", "Cargo.toml"] {
            let p = manifest.join(src);
            let m = std::fs::metadata(&p).unwrap().modified().unwrap();
            assert!(
                so_mtime >= m,
                "{} is older than {} -- stale artifact",
                so.display(),
                p.display()
            );
        }
        so
    })
    .clone()
}

/// Both libraries plus the resolved symbols. `Library` must outlive the
/// symbols, so they are re-resolved on every accessor call.
struct Pair {
    c_lib: Library,
    rust_lib: Library,
    c_path: PathBuf,
    rust_path: PathBuf,
}

impl Pair {
    fn load() -> Self {
        let c_path = find_c_so();
        let rust_path = build_rust_so();
        // SAFETY: both paths point at shared objects we just built ourselves;
        // loading them runs their (empty) initializers.
        let c_lib = unsafe { Library::new(&c_path) }
            .unwrap_or_else(|e| panic!("dlopen {}: {e}", c_path.display()));
        let rust_lib = unsafe { Library::new(&rust_path) }
            .unwrap_or_else(|e| panic!("dlopen {}: {e}", rust_path.display()));
        Self {
            c_lib,
            rust_lib,
            c_path,
            rust_path,
        }
    }

    fn c(&self) -> Symbol<'_, ContrastRatioFn> {
        unsafe { self.c_lib.get(b"contrast_ratio\0") }
            .unwrap_or_else(|e| panic!("dlsym contrast_ratio in {}: {e}", self.c_path.display()))
    }

    fn rust(&self) -> Symbol<'_, ContrastRatioFn> {
        unsafe { self.rust_lib.get(b"contrast_ratio\0") }
            .unwrap_or_else(|e| panic!("dlsym contrast_ratio in {}: {e}", self.rust_path.display()))
    }

    fn c_raw(&self) -> Symbol<'_, ContrastRatioRawFn> {
        unsafe { self.c_lib.get(b"contrast_ratio\0") }.expect("dlsym C raw")
    }

    fn rust_raw(&self) -> Symbol<'_, ContrastRatioRawFn> {
        unsafe { self.rust_lib.get(b"contrast_ratio\0") }.expect("dlsym Rust raw")
    }
}

// ---------------------------------------------------------------------------
// Comparison
// ---------------------------------------------------------------------------

/// Bit-exact equality with an explicit NaN rule.
///
/// The C returns the hardware result of `0.0f/0.0f` (SSE "real indefinite",
/// `0xFFC00000`). We require the bit patterns to be equal; if both sides are
/// NaN we additionally report the payloads so a mismatch is diagnosable.
#[track_caller]
fn assert_same(a: CbRgb255, b: CbRgb255, c_val: f32, rust_val: f32, row: &str) {
    let cb = c_val.to_bits();
    let rb = rust_val.to_bits();
    if cb == rb {
        return;
    }
    panic!(
        "[{row}] divergence for A={{{},{},{}}} B={{{},{},{}}}\n  C    = {c_val:e} (bits 0x{cb:08X}, nan={})\n  Rust = {rust_val:e} (bits 0x{rb:08X}, nan={})",
        a.r,
        a.g,
        a.b,
        b.r,
        b.g,
        b.b,
        c_val.is_nan(),
        rust_val.is_nan(),
    );
}

#[track_caller]
fn check(pair: &Pair, a: CbRgb255, b: CbRgb255, row: &str) {
    let c = pair.c();
    let rust = pair.rust();
    // SAFETY: signature matches `float contrast_ratio(cb_rgb_255, cb_rgb_255)`.
    let cv = unsafe { c(a, b) };
    let rv = unsafe { rust(a, b) };
    assert_same(a, b, cv, rv, row);
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (fixed seed -> reproducible)
// ---------------------------------------------------------------------------

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }
    fn next_u64(&mut self) -> u64 {
        // SplitMix64
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn u8(&mut self) -> u8 {
        (self.next_u64() >> 32) as u8
    }
    /// Uniform in `lo..=hi`.
    fn range_u8(&mut self, lo: u8, hi: u8) -> u8 {
        debug_assert!(lo <= hi);
        let span = (hi - lo) as u64 + 1;
        lo + ((self.next_u64() >> 16) % span) as u8
    }
    fn color(&mut self) -> CbRgb255 {
        CbRgb255::new(self.u8(), self.u8(), self.u8())
    }
    fn color_in(&mut self, lo: u8, hi: u8) -> CbRgb255 {
        CbRgb255::new(
            self.range_u8(lo, hi),
            self.range_u8(lo, hi),
            self.range_u8(lo, hi),
        )
    }
}

const SEED: u64 = 0x5EED_C0FF_EE00_0001;

/// Channel-branch arms from CONFIGS.md: `n <= 10` -> linear, `n >= 11` -> pow.
const LINEAR_HI: u8 = 10;
const POW_LO: u8 = 11;

// ===========================================================================
// Phase D — symbol parity
// ===========================================================================

#[test]
fn d01_symbol_parity() {
    let pair = Pair::load();
    // Every dynamic symbol the C .so defines must be resolvable in the Rust .so.
    let out = std::process::Command::new("nm")
        .args(["-D", "--defined-only", pair.c_path.to_str().unwrap()])
        .output()
        .expect("run nm on the C .so");
    assert!(out.status.success(), "nm failed on the C .so");
    let text = String::from_utf8_lossy(&out.stdout);

    let mut c_syms: Vec<String> = Vec::new();
    for line in text.lines() {
        let mut it = line.split_whitespace();
        let (Some(_addr), Some(kind), Some(name)) = (it.next(), it.next(), it.next()) else {
            continue;
        };
        // Skip absolute/ABI-tag symbols, which are not callable API.
        if kind.eq_ignore_ascii_case("a") {
            continue;
        }
        if name.starts_with("_ITM_") || name == "__gmon_start__" {
            continue;
        }
        c_syms.push(name.to_string());
    }
    c_syms.sort();
    c_syms.dedup();
    assert!(
        c_syms.iter().any(|s| s == "contrast_ratio"),
        "C .so must export contrast_ratio; got {c_syms:?}"
    );

    let mut missing = Vec::new();
    for name in &c_syms {
        let mut bytes = name.as_bytes().to_vec();
        bytes.push(0);
        // SAFETY: we only resolve the address, never call an unknown signature.
        let found = unsafe { pair.rust_lib.get::<*const ()>(&bytes) }.is_ok();
        if !found {
            missing.push(name.clone());
        }
    }
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but missing from the Rust .so: {missing:?}"
    );

    // The `static` C helpers must stay private on both sides.
    for hidden in ["cbLuminance", "cbContrastRatio"] {
        let mut bytes = hidden.as_bytes().to_vec();
        bytes.push(0);
        let in_c = unsafe { pair.c_lib.get::<*const ()>(&bytes) }.is_ok();
        let in_rust = unsafe { pair.rust_lib.get::<*const ()>(&bytes) }.is_ok();
        assert!(!in_c, "{hidden} is static in C and must not be dynamic");
        assert_eq!(
            in_c, in_rust,
            "{hidden} visibility must match between C and Rust"
        );
    }
}

// ===========================================================================
// Phase B — valid-path differential tests (CONFIGS.md rows)
// ===========================================================================

/// C01 — exhaustive grayscale cross product (256 x 256 = 65 536 pairs).
#[test]
fn c01_exhaustive_grayscale_pairs() {
    let pair = Pair::load();
    for n in 0u16..=255 {
        for m in 0u16..=255 {
            let a = CbRgb255::new(n as u8, n as u8, n as u8);
            let b = CbRgb255::new(m as u8, m as u8, m as u8);
            check(&pair, a, b, "C01");
        }
    }
}

/// C02 — all 64 per-channel branch-arm combinations, randomized inside each arm.
#[test]
fn c02_all_channel_branch_combinations() {
    let pair = Pair::load();
    let mut rng = Rng::new(SEED ^ 0x02);
    for mask in 0u32..64 {
        for _ in 0..512 {
            let mut ch = [0u8; 6];
            for (i, slot) in ch.iter_mut().enumerate() {
                *slot = if mask & (1 << i) != 0 {
                    rng.range_u8(POW_LO, 255) // pow arm
                } else {
                    rng.range_u8(0, LINEAR_HI) // linear arm
                };
            }
            let a = CbRgb255::new(ch[0], ch[1], ch[2]);
            let b = CbRgb255::new(ch[3], ch[4], ch[5]);
            check(&pair, a, b, "C02");
        }
    }
}

/// C03 — uniform random over the full 6-channel domain.
#[test]
fn c03_uniform_random_full_domain() {
    let pair = Pair::load();
    let mut rng = Rng::new(SEED ^ 0x03);
    for _ in 0..200_000 {
        let a = rng.color();
        let b = rng.color();
        check(&pair, a, b, "C03");
    }
}

/// C04 — swap path: `LumA < LumB` (the `if (High < Low)` body runs).
/// C05 — no-swap path: `LumA >= LumB`.
///
/// `contrast_ratio` is symmetric (it returns `max(LumA,LumB)/min(LumA,LumB)`),
/// so the two paths cannot be told apart from the return value alone. To
/// classify a pair using ONLY C outputs, probe each color against white:
/// `ratio(x, white) == Lum(white)/Lum(x)` is monotone *decreasing* in `Lum(x)`,
/// so `probe(A) > probe(B)` iff `LumA < LumB` iff the swap branch is taken.
#[test]
fn c04_c05_both_swap_directions() {
    let pair = Pair::load();
    let c = pair.c();
    let white = CbRgb255::new(255, 255, 255);
    let probe = |x: CbRgb255| -> f32 { unsafe { c(x, white) } };

    let mut rng = Rng::new(SEED ^ 0x45);
    let mut swapped = 0usize;
    let mut not_swapped = 0usize;
    for _ in 0..100_000 {
        let a = rng.color();
        let b = rng.color();
        check(&pair, a, b, "C04/C05");
        // The symmetry itself is a behaviour to match, not just C-side trivia.
        check(&pair, b, a, "C04/C05-reversed");
        let (pa, pb) = (probe(a), probe(b));
        assert_eq!(
            unsafe { c(a, b) }.to_bits(),
            unsafe { c(b, a) }.to_bits(),
            "C's own result must be order-independent for {a:?}/{b:?}"
        );
        if pa > pb {
            swapped += 1; // LumA < LumB -> swap branch taken
        } else {
            not_swapped += 1; // LumA >= LumB -> swap branch skipped
        }
    }
    assert!(
        swapped > 10_000,
        "swap path under-exercised ({swapped} of 100000)"
    );
    assert!(
        not_swapped > 10_000,
        "no-swap path under-exercised ({not_swapped} of 100000)"
    );

    // Deterministically forced cases for each branch, exhaustively over
    // grayscale so `LumA < LumB` and `LumA > LumB` are guaranteed hit.
    for n in 0u16..=255 {
        for m in 0u16..=255 {
            if n == m {
                continue;
            }
            let a = CbRgb255::new(n as u8, n as u8, n as u8);
            let b = CbRgb255::new(m as u8, m as u8, m as u8);
            // n < m -> LumA < LumB -> swap; n > m -> no swap.
            check(&pair, a, b, if n < m { "C04" } else { "C05" });
        }
    }
}

/// C06 — identical colors: exhaustive grayscale + random identical pairs.
#[test]
fn c06_identical_colors() {
    let pair = Pair::load();
    let c = pair.c();
    for n in 0u16..=255 {
        let a = CbRgb255::new(n as u8, n as u8, n as u8);
        check(&pair, a, a, "C06");
    }
    let mut rng = Rng::new(SEED ^ 0x06);
    for _ in 0..4096 {
        let a = rng.color();
        check(&pair, a, a, "C06");
        if a != CbRgb255::new(0, 0, 0) {
            // ERRORS.md row E4: identical non-black colors give exactly 1.0.
            let v = unsafe { c(a, a) };
            assert_eq!(
                v.to_bits(),
                1.0f32.to_bits(),
                "E4: C returned {v} for identical color {a:?}"
            );
        }
    }
}

/// C07 — both colors confined to the linear arm (`0..=10`), exhaustive-ish.
#[test]
fn c07_linear_arm_only() {
    let pair = Pair::load();
    // Exhaustive over the 11^3 = 1331 colors in the box, crossed against a
    // deterministic sample of the same box, plus random draws.
    let colors: Vec<CbRgb255> = (0..=LINEAR_HI)
        .flat_map(|r| {
            (0..=LINEAR_HI).flat_map(move |g| (0..=LINEAR_HI).map(move |b| CbRgb255::new(r, g, b)))
        })
        .collect();
    assert_eq!(colors.len(), 1331);
    // Every color against a fixed spread of partners (keeps the run bounded but
    // still touches every value in every slot).
    let partners = [
        CbRgb255::new(0, 0, 0),
        CbRgb255::new(1, 0, 0),
        CbRgb255::new(0, 1, 0),
        CbRgb255::new(0, 0, 1),
        CbRgb255::new(10, 10, 10),
        CbRgb255::new(10, 0, 5),
        CbRgb255::new(3, 7, 10),
    ];
    for a in &colors {
        for b in &partners {
            check(&pair, *a, *b, "C07");
            check(&pair, *b, *a, "C07");
        }
    }
    let mut rng = Rng::new(SEED ^ 0x07);
    for _ in 0..50_000 {
        let a = rng.color_in(0, LINEAR_HI);
        let b = rng.color_in(0, LINEAR_HI);
        check(&pair, a, b, "C07");
    }
}

/// C08 — both colors confined to the pow arm (`11..=255`).
#[test]
fn c08_pow_arm_only() {
    let pair = Pair::load();
    let mut rng = Rng::new(SEED ^ 0x08);
    for _ in 0..50_000 {
        let a = rng.color_in(POW_LO, 255);
        let b = rng.color_in(POW_LO, 255);
        check(&pair, a, b, "C08");
    }
}

/// C09 — boundary values only: full cross product of {0,1,10,11,254,255}^3.
#[test]
fn c09_boundary_value_cross_product() {
    let pair = Pair::load();
    const BOUND: [u8; 6] = [0, 1, 10, 11, 254, 255];
    let colors: Vec<CbRgb255> = BOUND
        .iter()
        .flat_map(|&r| {
            BOUND
                .iter()
                .flat_map(move |&g| BOUND.iter().map(move |&b| CbRgb255::new(r, g, b)))
        })
        .collect();
    assert_eq!(colors.len(), 216);
    for a in &colors {
        for b in &colors {
            check(&pair, *a, *b, "C09");
        }
    }
}

/// C10 — sweep one channel over all 256 values with the other five pinned.
#[test]
fn c10_single_channel_sweeps() {
    let pair = Pair::load();
    let mut rng = Rng::new(SEED ^ 0x10);
    for slot in 0..6 {
        for _ in 0..64 {
            let base = [
                rng.u8(),
                rng.u8(),
                rng.u8(),
                rng.u8(),
                rng.u8(),
                rng.u8(),
            ];
            for v in 0u16..=255 {
                let mut ch = base;
                ch[slot] = v as u8;
                let a = CbRgb255::new(ch[0], ch[1], ch[2]);
                let b = CbRgb255::new(ch[3], ch[4], ch[5]);
                check(&pair, a, b, "C10");
            }
        }
    }
}

/// C14 — extremes, both orders.
#[test]
fn c14_extremes() {
    let pair = Pair::load();
    let black = CbRgb255::new(0, 0, 0);
    let white = CbRgb255::new(255, 255, 255);
    check(&pair, black, white, "C14");
    check(&pair, white, black, "C14");
    check(&pair, white, white, "C14");
    check(&pair, black, black, "C14");
}

/// C15 — single-channel-only colors, isolating each luminance weight.
#[test]
fn c15_single_channel_colors() {
    let pair = Pair::load();
    let build: [fn(u8) -> CbRgb255; 3] = [
        |n| CbRgb255::new(n, 0, 0),
        |n| CbRgb255::new(0, n, 0),
        |n| CbRgb255::new(0, 0, n),
    ];
    for fa in build.iter() {
        for fb in build.iter() {
            for n in 0u16..=255 {
                for m in (0u16..=255).step_by(7) {
                    check(&pair, fa(n as u8), fb(m as u8), "C15");
                }
            }
        }
    }
}

/// C16 / E7 — garbage in the undefined upper bits of the argument registers.
#[test]
fn c16_e7_abi_upper_bit_garbage() {
    let pair = Pair::load();
    let c_raw = pair.c_raw();
    let rust_raw = pair.rust_raw();
    let c = pair.c();
    let mut rng = Rng::new(SEED ^ 0x16);

    for _ in 0..20_000 {
        let a = rng.color();
        let b = rng.color();
        let pack = |col: CbRgb255, garbage: u64| -> u64 {
            let clean = col.r as u64 | ((col.g as u64) << 8) | ((col.b as u64) << 16);
            clean | (garbage & !0x00FF_FFFFu64)
        };
        let ga = rng.next_u64();
        let gb = rng.next_u64();
        // SAFETY: a 3-byte struct is SysV class INTEGER, passed in the low bytes
        // of one integer register; the upper bits are unspecified padding.
        let cv = unsafe { c_raw(pack(a, ga), pack(b, gb)) };
        let rv = unsafe { rust_raw(pack(a, ga), pack(b, gb)) };
        assert_same(a, b, cv, rv, "C16/E7");

        // And the garbage must not change the value at all.
        let clean = unsafe { c(a, b) };
        assert_eq!(
            cv.to_bits(),
            clean.to_bits(),
            "E7: upper-bit garbage changed the C result for {a:?}/{b:?}"
        );
        assert_eq!(
            rv.to_bits(),
            clean.to_bits(),
            "E7: upper-bit garbage changed the Rust result for {a:?}/{b:?}"
        );
    }
}

// ===========================================================================
// Phase C — error-path differential tests (ERRORS.md rows)
// ===========================================================================

/// E1 — `B` is black, `A` is not: divide by zero on the no-swap path.
#[test]
fn e01_divide_by_zero_no_swap() {
    let pair = Pair::load();
    let c = pair.c();
    let black = CbRgb255::new(0, 0, 0);
    let mut rng = Rng::new(SEED ^ 0xE1);
    let mut checked = 0;
    for _ in 0..20_000 {
        let a = rng.color();
        if a == black {
            continue;
        }
        check(&pair, a, black, "E1");
        let v = unsafe { c(a, black) };
        assert_eq!(
            v.to_bits(),
            f32::INFINITY.to_bits(),
            "E1: expected +inf for A={a:?} B=black, got {v}"
        );
        checked += 1;
    }
    assert!(checked > 1000);

    // Also every non-black grayscale value, exhaustively.
    for n in 1u16..=255 {
        let a = CbRgb255::new(n as u8, n as u8, n as u8);
        check(&pair, a, black, "E1");
    }
    // And the minimum non-black colors in each channel slot.
    for a in [
        CbRgb255::new(1, 0, 0),
        CbRgb255::new(0, 1, 0),
        CbRgb255::new(0, 0, 1),
    ] {
        check(&pair, a, black, "E1");
        let v = unsafe { c(a, black) };
        assert_eq!(v.to_bits(), f32::INFINITY.to_bits(), "E1: {a:?}");
    }
}

/// E2 — `A` is black, `B` is not: divide by zero via the swap path.
#[test]
fn e02_divide_by_zero_swap_path() {
    let pair = Pair::load();
    let c = pair.c();
    let black = CbRgb255::new(0, 0, 0);
    let mut rng = Rng::new(SEED ^ 0xE2);
    let mut checked = 0;
    for _ in 0..20_000 {
        let b = rng.color();
        if b == black {
            continue;
        }
        check(&pair, black, b, "E2");
        let v = unsafe { c(black, b) };
        assert_eq!(
            v.to_bits(),
            f32::INFINITY.to_bits(),
            "E2: expected +inf for A=black B={b:?}, got {v}"
        );
        checked += 1;
    }
    assert!(checked > 1000);
    for n in 1u16..=255 {
        let b = CbRgb255::new(n as u8, n as u8, n as u8);
        check(&pair, black, b, "E2");
    }
    for b in [
        CbRgb255::new(1, 0, 0),
        CbRgb255::new(0, 1, 0),
        CbRgb255::new(0, 0, 1),
    ] {
        check(&pair, black, b, "E2");
        let v = unsafe { c(black, b) };
        assert_eq!(v.to_bits(), f32::INFINITY.to_bits(), "E2: {b:?}");
    }
}

/// E3 — both black: `0.0 / 0.0`. Must be the SAME NaN bit pattern, not merely
/// "both are NaN".
#[test]
fn e03_zero_over_zero_nan() {
    let pair = Pair::load();
    let c = pair.c();
    let rust = pair.rust();
    let black = CbRgb255::new(0, 0, 0);
    let cv = unsafe { c(black, black) };
    let rv = unsafe { rust(black, black) };
    assert!(cv.is_nan(), "E3: C should return NaN, got {cv}");
    assert!(rv.is_nan(), "E3: Rust should return NaN, got {rv}");
    assert_eq!(
        cv.to_bits(),
        rv.to_bits(),
        "E3: NaN bit patterns differ: C=0x{:08X} Rust=0x{:08X}",
        cv.to_bits(),
        rv.to_bits()
    );
    assert_eq!(
        cv.is_sign_negative(),
        rv.is_sign_negative(),
        "E3: NaN sign differs"
    );
    check(&pair, black, black, "E3");
}

/// E5 — the `> 0.04045` branch boundary: 10 (linear) vs 11 (pow), in every
/// channel slot, against every partner value.
#[test]
fn e05_transfer_function_branch_boundary() {
    let pair = Pair::load();
    for slot in 0..6usize {
        for &v in &[0u8, 1, 9, 10, 11, 12, 254, 255] {
            for &other in &[0u8, 1, 10, 11, 128, 254, 255] {
                let mut ch = [other; 6];
                ch[slot] = v;
                let a = CbRgb255::new(ch[0], ch[1], ch[2]);
                let b = CbRgb255::new(ch[3], ch[4], ch[5]);
                check(&pair, a, b, "E5");
            }
        }
    }
    // Confirm the boundary really is between 10 and 11 in the C, so the row is
    // testing what ERRORS.md claims.
    let c = pair.c();
    let ref_b = CbRgb255::new(255, 255, 255);
    let lum = |n: u8| -> f32 {
        let v = unsafe { c(CbRgb255::new(n, n, n), ref_b) };
        v
    };
    // Ratio is monotone in n over this range; just assert distinctness of the
    // two sides of the boundary (i.e. the branch is observable).
    assert_ne!(lum(10).to_bits(), lum(11).to_bits(), "E5 boundary");
}

/// E6 / E8 — the not-applicable rows, asserted structurally rather than
/// skipped: the exported signature has no pointer and no length parameter, so
/// null-pointer and zero/oversized-length inputs are unrepresentable. We verify
/// the C header really declares by-value structs by round-tripping every byte
/// value through each channel (a pointer-taking ABI would fault instead).
#[test]
fn e06_e08_no_pointer_or_length_parameters() {
    let pair = Pair::load();
    let header = std::fs::read_to_string(
        workspace_root().join("c_src").join("include").join("lib.h"),
    )
    .expect("read lib.h");
    let decl = header
        .lines()
        .find(|l| l.contains("contrast_ratio"))
        .expect("contrast_ratio declaration in lib.h");
    assert!(
        !decl.contains('*'),
        "E6/E8 assume no pointer parameters, but the declaration is: {decl}"
    );
    assert!(
        decl.contains("cb_rgb_255 A") && decl.contains("cb_rgb_255 B"),
        "unexpected declaration: {decl}"
    );
    assert!(
        !header.contains("size_t") && !header.contains("len"),
        "E6 assumes no length parameter, but lib.h mentions one"
    );
    // Structural claim confirmed; exercise by-value passing across all bytes.
    for n in 0u16..=255 {
        check(&pair, CbRgb255::new(n as u8, 0, 0), CbRgb255::new(0, 0, n as u8), "E6/E8");
    }
}

/// Out-of-range "enum" style inputs. The C declares no enum, so the closest
/// real analogue across FFI is a channel byte outside any meaningful range —
/// every `u8` is meaningful here — plus the raw-register injection already
/// covered by E7. This test drives every one of the 256 values through every
/// channel slot to prove there is no unhandled value.
#[test]
fn e09_every_channel_value_is_accepted() {
    let pair = Pair::load();
    for slot in 0..6usize {
        for v in 0u16..=255 {
            let mut ch = [7u8; 6];
            ch[slot] = v as u8;
            let a = CbRgb255::new(ch[0], ch[1], ch[2]);
            let b = CbRgb255::new(ch[3], ch[4], ch[5]);
            check(&pair, a, b, "E9");
            let mut ch2 = [200u8; 6];
            ch2[slot] = v as u8;
            let a2 = CbRgb255::new(ch2[0], ch2[1], ch2[2]);
            let b2 = CbRgb255::new(ch2[3], ch2[4], ch2[5]);
            check(&pair, a2, b2, "E9");
            let mut ch3 = [0u8; 6];
            ch3[slot] = v as u8;
            let a3 = CbRgb255::new(ch3[0], ch3[1], ch3[2]);
            let b3 = CbRgb255::new(ch3[3], ch3[4], ch3[5]);
            check(&pair, a3, b3, "E9");
        }
    }
}

// ===========================================================================
// Whole-domain exhaustion
// ===========================================================================

/// Exhaustive over the entire 2^24 color space against fixed references.
///
/// The input domain of `contrast_ratio` is `256^6`, which is not enumerable,
/// but the computation factors as `f(Lum(A), Lum(B))` with
/// `f(x, y) = max(x, y) / min(x, y)`. Enumerating all `2^24` colors against a
/// fixed reference therefore pins down every reachable `Lum` value (and hence
/// every `pow` argument) on both sides; `f` itself is a single `<` plus a single
/// `divss`, covered exhaustively over grayscale by C01 and in both branch
/// directions by C04/C05.
///
/// Two references are used so a hypothetical `Lum` mismatch cannot be masked by
/// the ratio: white (max luminance, always the numerator) and a dark non-black
/// color sitting exactly on the `pow` branch boundary.
#[test]
fn z01_exhaustive_all_16m_colors() {
    let pair = Pair::load();
    let c = pair.c();
    let rust = pair.rust();
    let refs = [
        (CbRgb255::new(255, 255, 255), "Z01/white"),
        (CbRgb255::new(11, 11, 11), "Z01/dark"),
    ];

    for (r, row) in refs {
        for packed in 0u32..(1u32 << 24) {
            let col = CbRgb255::new(
                (packed & 0xFF) as u8,
                ((packed >> 8) & 0xFF) as u8,
                ((packed >> 16) & 0xFF) as u8,
            );
            let cv = unsafe { c(col, r) };
            let rv = unsafe { rust(col, r) };
            if cv.to_bits() != rv.to_bits() {
                assert_same(col, r, cv, rv, row);
            }
        }
    }
}

/// Sanity/negative-control: the harness must really have TWO distinct libraries
/// loaded, and the Rust symbol must come from the Rust cdylib (not from the C
/// one via symbol interposition). Prints both resolved paths for the log.
#[test]
fn z02_harness_loads_two_distinct_libraries() {
    let pair = Pair::load();
    println!("C    .so: {}", pair.c_path.display());
    println!("Rust .so: {}", pair.rust_path.display());
    assert_ne!(pair.c_path, pair.rust_path);
    assert!(pair.c_path.to_string_lossy().contains("c_src/build"));
    assert!(pair.rust_path.to_string_lossy().contains("target/harness"));
    assert!(pair
        .rust_path
        .file_name()
        .unwrap()
        .to_string_lossy()
        .starts_with("libcontrast_ratio_lib"));
    let ca = *pair.c() as usize;
    let ra = *pair.rust() as usize;
    println!("C fn @ {ca:#x}, Rust fn @ {ra:#x}");
    assert_ne!(ca, ra, "both symbols resolved to the same address");
}
