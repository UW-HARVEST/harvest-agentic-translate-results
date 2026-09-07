//! Differential test harness: loads BOTH the C `.so` and the Rust `.so` with
//! `libloading` and compares `contrast_ratio` through the FFI boundary on raw
//! `u32` bit patterns. The Rust implementation is never called directly, so the
//! `#[no_mangle] extern "C"` export wrapper and the C struct ABI are tested too.
//!
//! Phase B rows -> `CONFIGS.md`, Phase C rows -> `ERRORS.md`,
//! Phase D symbol parity -> `SYMBOLS.md`.

use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};
use std::process::Command;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Rgb {
    r: u8,
    g: u8,
    b: u8,
}

type ContrastFn = unsafe extern "C" fn(Rgb, Rgb) -> f32;

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate has a parent directory")
        .to_path_buf()
}

/// Build the C shared library exactly as the task description prescribes and
/// return the path to the produced `.so` (its name is derived from the
/// directory name by `CMakeLists.txt`, so it must be globbed).
fn c_library_path() -> PathBuf {
    // Allows re-running the whole suite against a C library built with
    // different optimisation flags (e.g. -O2) without touching c_src/.
    if let Ok(p) = std::env::var("DIFF_C_SO") {
        return PathBuf::from(p);
    }
    let c_src = repo_root().join("c_src");
    let build = c_src.join("build");
    std::fs::create_dir_all(&build).expect("create c_src/build");

    let cmake_ok = Command::new("cmake")
        .current_dir(&build)
        .args(["..", "-DCMAKE_POSITION_INDEPENDENT_CODE=ON"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if cmake_ok {
        let _ = Command::new("cmake")
            .current_dir(&build)
            .args(["--build", "."])
            .output();
    }

    let mut found: Vec<PathBuf> = std::fs::read_dir(&build)
        .expect("c_src/build must exist -- build the C library first")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.extension().map(|e| e == "so").unwrap_or(false)
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("lib"))
                    .unwrap_or(false)
        })
        .collect();
    found.sort();
    assert!(
        !found.is_empty(),
        "no C .so found in {} -- run: cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        build.display()
    );
    found.remove(0)
}

/// Path of the Rust `cdylib`. Prefers the release artifact (the shipped one),
/// falls back to the debug artifact.
fn rust_library_path() -> PathBuf {
    // Allows re-running the suite against the debug-profile cdylib.
    if let Ok(p) = std::env::var("DIFF_RUST_SO") {
        return PathBuf::from(p);
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("target");
    let name = "libcontrast_ratio_lib.so";
    let release = root.join("release").join(name);
    if release.exists() {
        return release;
    }
    let debug = root.join("debug").join(name);
    if debug.exists() {
        return debug;
    }
    // Built by the same `cargo test` invocation? Search deps as a last resort.
    panic!(
        "Rust cdylib not found at {} -- run `cargo build --release` first",
        release.display()
    );
}

struct Pair {
    _c_lib: Library,
    _rust_lib: Library,
    c: ContrastFn,
    rust: ContrastFn,
}

impl Pair {
    fn load() -> Pair {
        unsafe {
            let c_lib = Library::new(c_library_path()).expect("dlopen C .so");
            let rust_lib = Library::new(rust_library_path()).expect("dlopen Rust .so");
            let c: Symbol<ContrastFn> = c_lib
                .get(b"contrast_ratio\0")
                .expect("C .so exports contrast_ratio");
            let rust: Symbol<ContrastFn> = rust_lib
                .get(b"contrast_ratio\0")
                .expect("Rust .so exports contrast_ratio");
            let c = *c;
            let rust = *rust;
            Pair {
                _c_lib: c_lib,
                _rust_lib: rust_lib,
                c,
                rust,
            }
        }
    }

    /// Calls both libraries and asserts bit-for-bit equality.
    #[track_caller]
    fn check(&self, a: Rgb, b: Rgb) -> u32 {
        let cv = unsafe { (self.c)(a, b) }.to_bits();
        let rv = unsafe { (self.rust)(a, b) }.to_bits();
        assert_eq!(
            cv,
            rv,
            "divergence for A={:?} B={:?}: C={:#010x} ({}) Rust={:#010x} ({})",
            a,
            b,
            cv,
            f32::from_bits(cv),
            rv,
            f32::from_bits(rv)
        );
        cv
    }
}

/// Deterministic SplitMix64 so every "randomized" row is reproducible.
struct Rng(u64);
impl Rng {
    fn new(seed: u64) -> Rng {
        Rng(seed)
    }
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn below(&mut self, n: u32) -> u32 {
        (self.next_u64() % n as u64) as u32
    }
    fn byte(&mut self) -> u8 {
        (self.next_u64() >> 33) as u8
    }
    /// Byte in the linear arm (`byte/255 <= 0.04045`) => 0..=10.
    fn lin_byte(&mut self) -> u8 {
        self.below(11) as u8
    }
    /// Byte in the `pow` arm (`byte/255 > 0.04045`) => 11..=255.
    fn pow_byte(&mut self) -> u8 {
        11 + self.below(245) as u8
    }
    fn masked_colour(&mut self, mask: u8) -> Rgb {
        let ch = |rng: &mut Rng, bit: u8| {
            if mask & bit != 0 {
                rng.pow_byte()
            } else {
                rng.lin_byte()
            }
        };
        let r = ch(self, 0b100);
        let g = ch(self, 0b010);
        let b = ch(self, 0b001);
        Rgb { r, g, b }
    }
}

const SEED: u64 = 0x5EED_1234_ABCD_0001;
const SAMPLES_PER_ROW: usize = 256;

// ---------------------------------------------------------------------------
// Phase D -- symbol parity (SYMBOLS.md)
// ---------------------------------------------------------------------------

fn dynamic_symbols(path: &Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", path.to_str().unwrap()])
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed on {}", path.display());
    // Toolchain / runtime symbols that are not part of the library API.
    const IGNORE: &[&str] = &[
        "_init",
        "_fini",
        "_edata",
        "_end",
        "__bss_start",
        "__cxa_finalize",
        "__gmon_start__",
        "_ITM_registerTMCloneTable",
        "_ITM_deregisterTMCloneTable",
        "rust_eh_personality",
        "rust_metadata",
    ];
    let mut syms: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .filter(|s| !IGNORE.contains(&s.as_str()) && !s.starts_with("__rust"))
        .collect();
    syms.sort();
    syms.dedup();
    syms
}

#[test]
fn phase_d_symbol_parity() {
    let c_syms = dynamic_symbols(&c_library_path());
    let r_syms = dynamic_symbols(&rust_library_path());
    let missing: Vec<&String> = c_syms.iter().filter(|s| !r_syms.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing C-exported symbols: {:?}\nC: {:?}\nRust: {:?}",
        missing,
        c_syms,
        r_syms
    );
    assert!(
        c_syms.iter().any(|s| s == "contrast_ratio"),
        "sanity: C .so must export contrast_ratio, got {:?}",
        c_syms
    );
    // The `static` C helpers must not leak into the Rust dynamic symbol table.
    for leaked in ["cbLuminance", "cbContrastRatio"] {
        assert!(
            !r_syms.iter().any(|s| s == leaked),
            "Rust .so must not export the C-static helper {leaked}"
        );
    }
}

// ---------------------------------------------------------------------------
// Phase B -- rows 1..64: full cross product of the per-channel sRGB transfer
// branch masks for colour A and colour B, both argument orders.
// ---------------------------------------------------------------------------

#[test]
fn phase_b_rows_1_to_64_branch_mask_cross_product() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED);
    let mut row = 0;
    for ma in 0u8..8 {
        for mb in 0u8..8 {
            row += 1;
            for _ in 0..SAMPLES_PER_ROW {
                let a = rng.masked_colour(ma);
                let b = rng.masked_colour(mb);
                // Axis 2: both swap directions.
                let ab = p.check(a, b);
                let ba = p.check(b, a);
                assert_eq!(
                    ab, ba,
                    "row {row}: C/Rust agreed but f(A,B) != f(B,A) for {a:?} {b:?}"
                );
            }
        }
    }
    assert_eq!(row, 64);
}

// ---------------------------------------------------------------------------
// Phase B -- row 65: exhaustive grey x grey (65536 pairs). This pins every
// possible `cbLuminance` byte input and both swap directions.
// ---------------------------------------------------------------------------

#[test]
fn phase_b_row_65_exhaustive_grey_pairs() {
    let p = Pair::load();
    for v in 0u8..=255 {
        for w in 0u8..=255 {
            p.check(Rgb { r: v, g: v, b: v }, Rgb { r: w, g: w, b: w });
        }
    }
}

// ---------------------------------------------------------------------------
// Phase B -- rows 66..69: exhaustive single-channel sweeps.
// ---------------------------------------------------------------------------

#[test]
fn phase_b_rows_66_to_69_single_channel_sweeps() {
    let p = Pair::load();
    let mid = Rgb {
        r: 128,
        g: 128,
        b: 128,
    };
    for v in 0u8..=255 {
        p.check(Rgb { r: v, g: 0, b: 0 }, mid); // row 66
        p.check(Rgb { r: 0, g: v, b: 0 }, mid); // row 67
        p.check(Rgb { r: 0, g: 0, b: v }, mid); // row 68
        p.check(mid, Rgb { r: v, g: v, b: 0 }); // row 69
    }
}

// ---------------------------------------------------------------------------
// Phase B -- rows 70..71: boundary sweeps around the 0.04045 branch point and
// near the top of the range.
// ---------------------------------------------------------------------------

#[test]
fn phase_b_rows_70_71_boundary_sweeps() {
    let p = Pair::load();
    for v in 0u8..=21 {
        for w in 0u8..=21 {
            p.check(Rgb { r: v, g: v, b: v }, Rgb { r: w, g: w, b: w }); // row 70
        }
    }
    for v in 234u8..=255 {
        for w in 234u8..=255 {
            p.check(Rgb { r: v, g: v, b: v }, Rgb { r: w, g: w, b: w }); // row 71
        }
    }
    // Mixed-channel straddling of the threshold: every combination of a
    // below-threshold byte (10) and an above-threshold byte (11) per channel.
    for i in 0u8..8 {
        for j in 0u8..8 {
            let pick = |m: u8, bit: u8| if m & bit != 0 { 11u8 } else { 10u8 };
            let a = Rgb {
                r: pick(i, 4),
                g: pick(i, 2),
                b: pick(i, 1),
            };
            let b = Rgb {
                r: pick(j, 4),
                g: pick(j, 2),
                b: pick(j, 1),
            };
            p.check(a, b);
        }
    }
}

// ---------------------------------------------------------------------------
// Phase B -- row 72: A == B for randomized colours (exact self-division).
// ---------------------------------------------------------------------------

#[test]
fn phase_b_row_72_equal_arguments() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 0x72);
    for _ in 0..4096 {
        let a = Rgb {
            r: rng.byte(),
            g: rng.byte(),
            b: rng.byte(),
        };
        let bits = p.check(a, a);
        if a == (Rgb { r: 0, g: 0, b: 0 }) {
            assert!(f32::from_bits(bits).is_nan());
        } else {
            assert_eq!(
                bits,
                1.0f32.to_bits(),
                "self-ratio must be exactly 1.0 for {a:?}, got {:#010x}",
                bits
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Phase B -- row 73: large unbiased randomized sweep over the whole domain.
// ---------------------------------------------------------------------------

#[test]
fn phase_b_row_73_randomized_full_range() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 0x73);
    for _ in 0..200_000 {
        let a = Rgb {
            r: rng.byte(),
            g: rng.byte(),
            b: rng.byte(),
        };
        let b = Rgb {
            r: rng.byte(),
            g: rng.byte(),
            b: rng.byte(),
        };
        p.check(a, b);
    }
}

// ---------------------------------------------------------------------------
// Phase B -- row 74: extreme ratios (smallest non-zero denominator).
// ---------------------------------------------------------------------------

#[test]
fn phase_b_row_74_extreme_ratios() {
    let p = Pair::load();
    let black = Rgb { r: 0, g: 0, b: 0 };
    for minimal in [
        Rgb { r: 1, g: 0, b: 0 },
        Rgb { r: 0, g: 1, b: 0 },
        Rgb { r: 0, g: 0, b: 1 },
    ] {
        let bits = p.check(black, minimal);
        p.check(minimal, black);
        assert!(
            f32::from_bits(bits).is_infinite(),
            "black vs {minimal:?} divides by zero luminance -> inf, got {:#010x}",
            bits
        );
    }
    for v in 1u8..=255 {
        p.check(black, Rgb { r: v, g: v, b: v });
        p.check(Rgb { r: v, g: v, b: v }, black);
    }
}

// ---------------------------------------------------------------------------
// Phase B -- row 75: argument symmetry inside each library.
// ---------------------------------------------------------------------------

#[test]
fn phase_b_row_75_argument_symmetry() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 0x75);
    for _ in 0..50_000 {
        let a = Rgb {
            r: rng.byte(),
            g: rng.byte(),
            b: rng.byte(),
        };
        let b = Rgb {
            r: rng.byte(),
            g: rng.byte(),
            b: rng.byte(),
        };
        let c_ab = unsafe { (p.c)(a, b) }.to_bits();
        let c_ba = unsafe { (p.c)(b, a) }.to_bits();
        let r_ab = unsafe { (p.rust)(a, b) }.to_bits();
        let r_ba = unsafe { (p.rust)(b, a) }.to_bits();
        assert_eq!(c_ab, r_ab, "A={a:?} B={b:?}");
        assert_eq!(c_ba, r_ba, "A={b:?} B={a:?}");
        assert_eq!(c_ab, c_ba, "C is not order-symmetric for {a:?} {b:?}");
        assert_eq!(r_ab, r_ba, "Rust is not order-symmetric for {a:?} {b:?}");
    }
}

// ---------------------------------------------------------------------------
// Phase B -- row 76: register-padding independence. `cb_rgb_255` is 3 bytes
// with align 1 but is passed packed in a register; garbage in the unused
// register bytes must not affect either library.
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy)]
struct RgbPadded {
    r: u8,
    g: u8,
    b: u8,
    pad: u8,
    pad2: u32,
}
type ContrastFnPadded = unsafe extern "C" fn(RgbPadded, RgbPadded) -> f32;

#[test]
fn phase_b_row_76_register_padding_independence() {
    let p = Pair::load();
    // Re-type the same exported symbols with a padded struct: the callee reads
    // only the low 3 bytes, so results must be unchanged.
    let c_pad: ContrastFnPadded = unsafe { std::mem::transmute(p.c) };
    let r_pad: ContrastFnPadded = unsafe { std::mem::transmute(p.rust) };
    let mut rng = Rng::new(SEED ^ 0x76);
    for _ in 0..4096 {
        let (ar, ag, ab) = (rng.byte(), rng.byte(), rng.byte());
        let (br, bg, bb) = (rng.byte(), rng.byte(), rng.byte());
        let baseline_c = unsafe { (p.c)(Rgb { r: ar, g: ag, b: ab }, Rgb { r: br, g: bg, b: bb }) }
            .to_bits();
        let baseline_r =
            unsafe { (p.rust)(Rgb { r: ar, g: ag, b: ab }, Rgb { r: br, g: bg, b: bb }) }.to_bits();
        assert_eq!(baseline_c, baseline_r);
        let pa = RgbPadded {
            r: ar,
            g: ag,
            b: ab,
            pad: rng.byte(),
            pad2: rng.next_u64() as u32,
        };
        let pb = RgbPadded {
            r: br,
            g: bg,
            b: bb,
            pad: rng.byte(),
            pad2: rng.next_u64() as u32,
        };
        let cv = unsafe { c_pad(pa, pb) }.to_bits();
        let rv = unsafe { r_pad(pa, pb) }.to_bits();
        assert_eq!(cv, rv, "padded-call divergence");
        assert_eq!(cv, baseline_c, "C result depended on padding bytes");
        assert_eq!(rv, baseline_r, "Rust result depended on padding bytes");
    }
}

// ---------------------------------------------------------------------------
// Phase C -- ERRORS.md rows 1..10. Every assertion is on raw bit patterns so
// +inf / -inf / NaN / finite are distinguished.
// ---------------------------------------------------------------------------

const BLACK: Rgb = Rgb { r: 0, g: 0, b: 0 };
const WHITE: Rgb = Rgb {
    r: 255,
    g: 255,
    b: 255,
};

#[test]
fn phase_c_row_1_zero_denominator_second_arg() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 1);
    for _ in 0..1024 {
        let mut a = Rgb {
            r: rng.byte(),
            g: rng.byte(),
            b: rng.byte(),
        };
        if a == BLACK {
            a = WHITE;
        }
        let bits = p.check(a, BLACK);
        assert_eq!(
            bits,
            f32::INFINITY.to_bits(),
            "expected +inf for {a:?} vs black, got {:#010x}",
            bits
        );
    }
}

#[test]
fn phase_c_row_2_zero_denominator_first_arg_swap_path() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 2);
    for _ in 0..1024 {
        let mut b = Rgb {
            r: rng.byte(),
            g: rng.byte(),
            b: rng.byte(),
        };
        if b == BLACK {
            b = WHITE;
        }
        let bits = p.check(BLACK, b);
        assert_eq!(
            bits,
            f32::INFINITY.to_bits(),
            "expected +inf for black vs {b:?}, got {:#010x}",
            bits
        );
    }
}

#[test]
fn phase_c_row_3_zero_over_zero_is_nan() {
    let p = Pair::load();
    let c = unsafe { (p.c)(BLACK, BLACK) };
    let r = unsafe { (p.rust)(BLACK, BLACK) };
    assert!(c.is_nan() && r.is_nan(), "C={c} Rust={r}");
    assert_eq!(
        c.to_bits(),
        r.to_bits(),
        "NaN payloads differ: C={:#010x} Rust={:#010x}",
        c.to_bits(),
        r.to_bits()
    );
}

#[test]
fn phase_c_row_4_equal_non_black_is_exactly_one() {
    let p = Pair::load();
    for v in 1u8..=255 {
        let a = Rgb { r: v, g: v, b: v };
        assert_eq!(p.check(a, a), 1.0f32.to_bits());
    }
    assert_eq!(p.check(WHITE, WHITE), 1.0f32.to_bits());
}

#[test]
fn phase_c_row_5_transfer_branch_boundary() {
    let p = Pair::load();
    // 10/255 = 0.039215688 <= 0.04045 -> linear arm; 11/255 = 0.043137256 > -> pow arm.
    for v in [9u8, 10, 11, 12] {
        for w in [9u8, 10, 11, 12] {
            p.check(Rgb { r: v, g: w, b: v }, Rgb { r: w, g: v, b: w });
        }
    }
    // Per-channel independence of the branch decision.
    for v in [10u8, 11] {
        p.check(Rgb { r: v, g: 10, b: 11 }, Rgb { r: 11, g: v, b: 10 });
    }
}

#[test]
fn phase_c_row_6_max_channels() {
    let p = Pair::load();
    assert_eq!(p.check(WHITE, WHITE), 1.0f32.to_bits());
    p.check(WHITE, BLACK);
    p.check(BLACK, WHITE);
    p.check(WHITE, Rgb { r: 255, g: 255, b: 254 });
    p.check(Rgb { r: 254, g: 255, b: 255 }, WHITE);
}

#[test]
fn phase_c_row_7_smallest_non_zero_denominator_is_finite_or_inf() {
    let p = Pair::load();
    for minimal in [
        Rgb { r: 1, g: 0, b: 0 },
        Rgb { r: 0, g: 1, b: 0 },
        Rgb { r: 0, g: 0, b: 1 },
        Rgb { r: 1, g: 1, b: 1 },
    ] {
        // vs white: largest finite-or-infinite ratio.
        let bits = p.check(WHITE, minimal);
        let back = p.check(minimal, WHITE);
        assert_eq!(bits, back);
        assert!(
            !f32::from_bits(bits).is_nan(),
            "unexpected NaN for white vs {minimal:?}"
        );
    }
}

#[test]
fn phase_c_row_8_order_asymmetry_including_degenerates() {
    let p = Pair::load();
    let degenerate = [
        (BLACK, BLACK),
        (BLACK, WHITE),
        (WHITE, BLACK),
        (BLACK, Rgb { r: 0, g: 0, b: 1 }),
        (Rgb { r: 0, g: 0, b: 1 }, BLACK),
    ];
    for (a, b) in degenerate {
        let c_ab = unsafe { (p.c)(a, b) }.to_bits();
        let c_ba = unsafe { (p.c)(b, a) }.to_bits();
        let r_ab = unsafe { (p.rust)(a, b) }.to_bits();
        let r_ba = unsafe { (p.rust)(b, a) }.to_bits();
        assert_eq!(c_ab, r_ab, "{a:?} {b:?}");
        assert_eq!(c_ba, r_ba, "{b:?} {a:?}");
        assert_eq!(c_ab, c_ba, "C order asymmetry for {a:?} {b:?}");
        assert_eq!(r_ab, r_ba, "Rust order asymmetry for {a:?} {b:?}");
    }
}

#[test]
fn phase_c_row_9_padding_bytes_are_ignored() {
    // Same mechanism as Phase B row 76 but pinned to the degenerate inputs.
    let p = Pair::load();
    let c_pad: ContrastFnPadded = unsafe { std::mem::transmute(p.c) };
    let r_pad: ContrastFnPadded = unsafe { std::mem::transmute(p.rust) };
    for (a, b) in [(BLACK, BLACK), (BLACK, WHITE), (WHITE, WHITE)] {
        let expect_c = unsafe { (p.c)(a, b) }.to_bits();
        for junk in [0x0000_0000u32, 0xFFFF_FFFF, 0xDEAD_BEEF] {
            let pa = RgbPadded {
                r: a.r,
                g: a.g,
                b: a.b,
                pad: (junk & 0xFF) as u8,
                pad2: junk,
            };
            let pb = RgbPadded {
                r: b.r,
                g: b.g,
                b: b.b,
                pad: (junk >> 8) as u8,
                pad2: !junk,
            };
            let cv = unsafe { c_pad(pa, pb) }.to_bits();
            let rv = unsafe { r_pad(pa, pb) }.to_bits();
            assert_eq!(cv, rv, "padded degenerate divergence {a:?} {b:?}");
            assert_eq!(cv, expect_c);
        }
    }
}

#[test]
fn phase_c_row_10_out_of_range_channel_values_wrap() {
    // There is no valid-range rejection: the ABI truncates any wider integer to
    // `unsigned char`. Emulate a caller passing 256/-1/300 by taking the low
    // byte, and confirm both libraries agree with the wrapped-byte call.
    let p = Pair::load();
    type WideFn = unsafe extern "C" fn(u32, u32) -> f32;
    let c_wide: WideFn = unsafe { std::mem::transmute(p.c) };
    let r_wide: WideFn = unsafe { std::mem::transmute(p.rust) };
    for raw_a in [0x0000_0100u32, 0xFFFF_FFFF, 0x0000_012Cu32, 0x00FF_FF00] {
        for raw_b in [0x0000_0100u32, 0x0000_0000, 0x1234_5678] {
            let cv = unsafe { c_wide(raw_a, raw_b) }.to_bits();
            let rv = unsafe { r_wide(raw_a, raw_b) }.to_bits();
            assert_eq!(
                cv, rv,
                "wide-int call divergence raw_a={raw_a:#010x} raw_b={raw_b:#010x}"
            );
            let wrapped = |x: u32| Rgb {
                r: (x & 0xFF) as u8,
                g: ((x >> 8) & 0xFF) as u8,
                b: ((x >> 16) & 0xFF) as u8,
            };
            assert_eq!(
                cv,
                unsafe { (p.c)(wrapped(raw_a), wrapped(raw_b)) }.to_bits(),
                "C did not simply truncate to 3 bytes"
            );
        }
    }
}

// Generic FFI boundary checks that every C API has, beyond the table.
#[test]
fn phase_c_generic_boundaries() {
    let p = Pair::load();
    // All 8 corners of the RGB cube against each other (0/255 extremes).
    let corners: Vec<Rgb> = (0..8)
        .map(|i| Rgb {
            r: if i & 4 != 0 { 255 } else { 0 },
            g: if i & 2 != 0 { 255 } else { 0 },
            b: if i & 1 != 0 { 255 } else { 0 },
        })
        .collect();
    for &a in &corners {
        for &b in &corners {
            p.check(a, b);
        }
    }
    // One step past each branch boundary and each channel extreme.
    for v in [0u8, 1, 10, 11, 12, 127, 128, 254, 255] {
        for w in [0u8, 1, 10, 11, 12, 127, 128, 254, 255] {
            p.check(Rgb { r: v, g: w, b: v }, Rgb { r: w, g: v, b: w });
            p.check(Rgb { r: v, g: v, b: w }, Rgb { r: v, g: w, b: w });
        }
    }
    // There are no pointer parameters (struct passed by value), so there is no
    // null-pointer input class; assert that fact structurally.
    assert_eq!(std::mem::size_of::<Rgb>(), 3);
    assert_eq!(std::mem::align_of::<Rgb>(), 1);
}

// ---------------------------------------------------------------------------
// Exhaustive closure of the input domain (superset of CONFIGS.md rows).
//
// The full domain is 256^6 pairs, but `contrast_ratio` factors through
// `cbLuminance`, which has exactly 256^3 = 16777216 distinct inputs. Pairing
// every colour against a fixed reference therefore pins every reachable
// intermediate luminance bit pattern, and hence every reachable branch mask,
// for both libraries. Run against two references (white and black) so both
// swap directions and the zero-denominator path are covered exhaustively.
// ---------------------------------------------------------------------------

#[test]
fn exhaustive_all_16m_colours_against_references() {
    let p = Pair::load();
    for &reference in &[WHITE, BLACK, Rgb { r: 10, g: 11, b: 200 }] {
        for r in 0u8..=255 {
            for g in 0u8..=255 {
                for b in 0u8..=255 {
                    let a = Rgb { r, g, b };
                    let cv = unsafe { (p.c)(a, reference) }.to_bits();
                    let rv = unsafe { (p.rust)(a, reference) }.to_bits();
                    if cv != rv {
                        panic!(
                            "divergence A={a:?} ref={reference:?}: C={cv:#010x} Rust={rv:#010x}"
                        );
                    }
                }
            }
        }
    }
}
