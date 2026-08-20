//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Every test loads BOTH the C `.so` and the Rust `.so` through `libloading`
//! and compares the results of the exported `tritanopia` symbol byte-for-byte.
//! Rust code is never called directly.

mod common;

use common::*;

// --- Axis 2: cbRemoveGammaRGB per-channel branch (linear <=10 / pow >=11) ---
// Rows 1-8: the full 2^3 cross-product of the per-channel branch.

const LIN: (u8, u8) = (0, 10); // c = v/255 <= 0.0392156877 -> NOT > 0.04045
const POW: (u8, u8) = (11, 255); // c = v/255 >= 0.0431372561 -> > 0.04045

#[test]
fn row01_remove_gamma_lin_lin_lin() {
    // Sub-domain is only 11^3 = 1331 inputs -> enumerate exhaustively.
    let n = check_all("row01 (lin,lin,lin)", cuboid(LIN, LIN, LIN));
    assert_eq!(n, 11 * 11 * 11);
}

#[test]
fn row02_remove_gamma_lin_lin_pow() {
    check_all(
        "row02 (lin,lin,pow)",
        random_in_ranges(SEED ^ 2, SAMPLES, LIN, LIN, POW),
    );
}

#[test]
fn row03_remove_gamma_lin_pow_lin() {
    check_all(
        "row03 (lin,pow,lin)",
        random_in_ranges(SEED ^ 3, SAMPLES, LIN, POW, LIN),
    );
}

#[test]
fn row04_remove_gamma_lin_pow_pow() {
    check_all(
        "row04 (lin,pow,pow)",
        random_in_ranges(SEED ^ 4, SAMPLES, LIN, POW, POW),
    );
}

#[test]
fn row05_remove_gamma_pow_lin_lin() {
    check_all(
        "row05 (pow,lin,lin)",
        random_in_ranges(SEED ^ 5, SAMPLES, POW, LIN, LIN),
    );
}

#[test]
fn row06_remove_gamma_pow_lin_pow() {
    check_all(
        "row06 (pow,lin,pow)",
        random_in_ranges(SEED ^ 6, SAMPLES, POW, LIN, POW),
    );
}

#[test]
fn row07_remove_gamma_pow_pow_lin() {
    check_all(
        "row07 (pow,pow,lin)",
        random_in_ranges(SEED ^ 7, SAMPLES, POW, POW, LIN),
    );
}

#[test]
fn row08_remove_gamma_pow_pow_pow() {
    check_all(
        "row08 (pow,pow,pow)",
        random_in_ranges(SEED ^ 8, SAMPLES, POW, POW, POW),
    );
}

/// Row 9: the exact Axis-2 threshold. `10` must take the linear branch and
/// `11` the `pow` branch; a `>=` comparison or an `f32` comparison would flip
/// one of these. All 8 combinations of {10,11}^3 are enumerated.
#[test]
fn row09_remove_gamma_exact_threshold() {
    let n = check_all("row09 threshold {10,11}^3", cuboid((10, 11), (10, 11), (10, 11)));
    assert_eq!(n, 8);
}

// --- Axis 3: cbApplyGammaRGB per-output-channel branch ---

/// Row 10: R output <= 0.0031308 (linear branch, includes all negatives).
/// Driven by making B dominate G, since R_out = R + 0.127*G - 0.127*B.
#[test]
fn row10_apply_gamma_r_linear() {
    check_all(
        "row10 applyGamma R linear",
        random_in_ranges(SEED ^ 10, SAMPLES, (0, 8), (0, 8), (128, 255)),
    );
}

/// Row 11: R output > 0.0031308 (pow branch).
#[test]
fn row11_apply_gamma_r_pow() {
    check_all(
        "row11 applyGamma R pow",
        random_in_ranges(SEED ^ 11, SAMPLES, (32, 255), (0, 255), (0, 255)),
    );
}

/// Row 12: G output <= 0.0031308 (linear branch).
/// G_out = -4.486e-11*R + 0.8739*G + 0.1261*B, so G and B must both be tiny.
#[test]
fn row12_apply_gamma_g_linear() {
    check_all(
        "row12 applyGamma G linear",
        random_in_ranges(SEED ^ 12, SAMPLES, (0, 255), (0, 4), (0, 4)),
    );
}

/// Row 13: G output > 0.0031308 (pow branch).
#[test]
fn row13_apply_gamma_g_pow() {
    check_all(
        "row13 applyGamma G pow",
        random_in_ranges(SEED ^ 13, SAMPLES, (0, 255), (64, 255), (0, 255)),
    );
}

/// Row 14: B output <= 0.0031308 (linear branch).
#[test]
fn row14_apply_gamma_b_linear() {
    check_all(
        "row14 applyGamma B linear",
        random_in_ranges(SEED ^ 14, SAMPLES, (0, 255), (0, 4), (0, 4)),
    );
}

/// Row 15: B output > 0.0031308 (pow branch).
#[test]
fn row15_apply_gamma_b_pow() {
    check_all(
        "row15 applyGamma B pow",
        random_in_ranges(SEED ^ 15, SAMPLES, (0, 255), (0, 255), (64, 255)),
    );
}

// --- Axis 4: cbDenorm narrowing-cast range class ---

/// Row 16: R denorm argument < 0 -> the C UB wraparound (ERRORS E1).
#[test]
fn row16_denorm_r_negative() {
    check_all(
        "row16 denorm R < 0",
        random_in_ranges(SEED ^ 16, SAMPLES, (0, 16), (0, 16), (200, 255)),
    );
}

/// Row 17: R denorm argument > 255 -> the C UB wraparound (ERRORS E2).
#[test]
fn row17_denorm_r_over_255() {
    check_all(
        "row17 denorm R > 255",
        random_in_ranges(SEED ^ 17, SAMPLES, (240, 255), (240, 255), (0, 16)),
    );
}

/// Row 18: R denorm argument inside 0..=255 (the ordinary path).
#[test]
fn row18_denorm_r_in_range() {
    let mut rng = Rng::new(SEED ^ 18);
    let inputs = (0..SAMPLES).map(move |_| {
        let v = rng.range_u8(0, 255);
        Rgb::new(v, v, v) // grayscale keeps R_out in range
    });
    check_all("row18 denorm R in range", inputs);
}

/// Rows 19+20: G and B denorm arguments hit their exact maximum of `255.5`.
/// `trunc(255.5) == 255`; a rounding cast would produce `256 -> 0`.
#[test]
fn row19_20_denorm_g_b_upper_boundary() {
    let n = check_all(
        "row19/20 G,B denorm == 255.5",
        (0u16..=255).map(|r| Rgb::new(r as u8, 255, 255)),
    );
    assert_eq!(n, 256);
}

// --- Axis 6: value shapes ---

/// Row 21: all 8 vertices of the {0,255}^3 cube.
#[test]
fn row21_extreme_vertices() {
    let n = check_all("row21 vertices", cuboid((0, 0), (0, 0), (0, 0)).chain(
        [
            Rgb::new(0, 0, 255),
            Rgb::new(0, 255, 0),
            Rgb::new(0, 255, 255),
            Rgb::new(255, 0, 0),
            Rgb::new(255, 0, 255),
            Rgb::new(255, 255, 0),
            Rgb::new(255, 255, 255),
        ]
        .into_iter(),
    ));
    assert_eq!(n, 8);
}

/// Row 22: grayscale R=G=B, all 256 values.
#[test]
fn row22_grayscale() {
    let n = check_all("row22 grayscale", (0u16..=255).map(|v| Rgb::new(v as u8, v as u8, v as u8)));
    assert_eq!(n, 256);
}

/// Row 23: exactly one channel non-zero, all 256 values, all 3 positions.
/// `(255,0,0)` is the only shape that isolates the sign and magnitude of the
/// two near-zero matrix cross-terms `-4.486E-11` and `+3.1113E-10` (Axis 5).
#[test]
fn row23_single_channel_only() {
    let inputs = (0u16..=255)
        .map(|v| Rgb::new(v as u8, 0, 0))
        .chain((0u16..=255).map(|v| Rgb::new(0, v as u8, 0)))
        .chain((0u16..=255).map(|v| Rgb::new(0, 0, v as u8)));
    let n = check_all("row23 single channel", inputs);
    assert_eq!(n, 768);
}

/// Row 27: near-extremes, thresholds and midpoints crossed with each other.
#[test]
fn row27_boundary_value_cube() {
    const V: [u8; 7] = [1, 10, 11, 127, 128, 254, 255];
    let inputs = V
        .iter()
        .flat_map(|&r| V.iter().flat_map(move |&g| V.iter().map(move |&b| Rgb::new(r, g, b))));
    let n = check_all("row27 boundary cube", inputs);
    assert_eq!(n, 343);
}

/// Row 28: uniform random over the whole cube (property-style cross-check).
#[test]
fn row28_uniform_random_full_cube() {
    check_all(
        "row28 uniform random",
        random_in_ranges(SEED ^ 28, 200_000, (0, 255), (0, 255), (0, 255)),
    );
}

// --- Axis 7: ABI (rows 24/25) ---

/// Row 24 / ERRORS E14: `cb_rgb_255` is 3 bytes and is passed in the low 3
/// bytes of `RDI`; the upper 5 bytes are unspecified. Driving them with
/// garbage must not change either implementation's answer.
#[test]
fn row24_upper_arg_register_garbage() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 24);
    for _ in 0..20_000 {
        let (r, g, b) = (rng.range_u8(0, 255), rng.range_u8(0, 255), rng.range_u8(0, 255));
        let rgb_bits = (r as u64) | ((g as u64) << 8) | ((b as u64) << 16);
        let garbage = rng.next_u64() & !0x00FF_FFFFu64;
        let dirty = rgb_bits | garbage;

        let c_clean = unsafe { (p.c_raw)(rgb_bits) } & 0xFF_FFFF;
        let c_dirty = unsafe { (p.c_raw)(dirty) } & 0xFF_FFFF;
        let r_clean = unsafe { (p.rust_raw)(rgb_bits) } & 0xFF_FFFF;
        let r_dirty = unsafe { (p.rust_raw)(dirty) } & 0xFF_FFFF;

        assert_eq!(
            c_clean, c_dirty,
            "C changed answer due to upper arg-register garbage {dirty:#018x}"
        );
        assert_eq!(
            r_clean, r_dirty,
            "Rust changed answer due to upper arg-register garbage {dirty:#018x}"
        );
        assert_eq!(
            c_dirty, r_dirty,
            "C/Rust diverge with dirty arg register {dirty:#018x} for ({r},{g},{b})"
        );
    }
}

/// Row 25 / ERRORS E15: struct layout and return-register contract.
#[test]
fn row25_struct_by_value_abi() {
    assert_eq!(std::mem::size_of::<Rgb>(), 3, "sizeof(cb_rgb_255) must be 3");
    assert_eq!(std::mem::align_of::<Rgb>(), 1, "alignof(cb_rgb_255) must be 1");

    // Only the low 3 bytes of RAX are meaningful; the struct-typed call and
    // the raw u64 call must agree on exactly those bytes.
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 25);
    for _ in 0..20_000 {
        let i = Rgb::new(rng.range_u8(0, 255), rng.range_u8(0, 255), rng.range_u8(0, 255));
        let bits = (i.r as u64) | ((i.g as u64) << 8) | ((i.b as u64) << 16);

        let c_struct = p.call_c(i);
        let r_struct = p.call_rust(i);
        let c_bits = unsafe { (p.c_raw)(bits) } & 0xFF_FFFF;
        let r_bits = unsafe { (p.rust_raw)(bits) } & 0xFF_FFFF;

        let pack = |v: Rgb| (v.r as u64) | ((v.g as u64) << 8) | ((v.b as u64) << 16);
        assert_eq!(pack(c_struct), c_bits, "C struct-return vs register-return mismatch");
        assert_eq!(pack(r_struct), r_bits, "Rust struct-return vs register-return mismatch");
        assert_eq!(c_struct, r_struct, "C/Rust diverge for ({},{},{})", i.r, i.g, i.b);
    }
}

/// Sanity: the two libraries really are two distinct objects, and the Rust one
/// really did resolve its `#[no_mangle]` export (guards against a test that
/// accidentally compares a library with itself).
#[test]
fn row00_harness_sanity() {
    let p = Pair::load();
    let (c_addr, rust_addr) = p.raw_addrs();
    assert_ne!(
        c_addr, rust_addr,
        "C and Rust `tritanopia` resolved to the SAME address - the test would be vacuous"
    );
    // A known-good value pair from the C implementation.
    assert_eq!(p.call_c(Rgb::new(0, 0, 0)), Rgb::new(0, 0, 0));
    assert_eq!(p.call_c(Rgb::new(255, 255, 255)), Rgb::new(255, 255, 255));
    assert_same(&p, Rgb::new(0, 0, 0));
    assert_same(&p, Rgb::new(255, 255, 255));
}
