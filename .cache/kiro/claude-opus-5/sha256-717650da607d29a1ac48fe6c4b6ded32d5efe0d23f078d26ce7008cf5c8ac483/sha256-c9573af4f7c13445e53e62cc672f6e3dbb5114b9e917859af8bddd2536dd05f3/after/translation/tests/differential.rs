//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md` (rows 1–33). Every test calls **both** the C
//! `.so` and the Rust `.so` through `libloading` and requires bit-identical
//! `uint16_t` results.

// The literals in `cfg_31_named_magnitudes` are deliberately written with more
// decimal digits than `f32` can hold (and deliberately include hand-written
// approximations of pi and e): the point is to make the *compiler* round them,
// which is what a real caller's source looks like. Replacing them with
// `std::f32::consts::PI` would remove the input under test.
#![allow(clippy::excessive_precision, clippy::approx_constant)]

mod common;

use common::{bits_from, mantissa_shapes, Pair, Rng, SEED};

/// `m__shift[j]`, transcribed from the run-length structure of the C table
/// (see the region table in `CONFIGS.md`). Used only to build the
/// "bits the shift discards" mantissa shape — never to predict a result.
fn shift_for(j: u32) -> u32 {
    let e = j & 0xff;
    match e {
        0..=102 => 24,
        103..=112 => 126 - e,
        113..=142 => 13,
        143..=254 => 24,
        255 => 13,
        _ => unreachable!(),
    }
}

const RANDOM_MANTISSAS_PER_EXPONENT: u32 = 1000;

/// Drive one row: every `j` in `js`, each with all six mantissa shapes plus
/// `RANDOM_MANTISSAS_PER_EXPONENT` seeded random mantissas.
fn run_row(pair: &Pair, row: &str, js: impl IntoIterator<Item = u32>) {
    let mut rng = Rng::new(SEED ^ row.len() as u64);
    for j in js {
        let shift = shift_for(j);
        for (k, m) in mantissa_shapes(shift).into_iter().enumerate() {
            pair.check_bits(
                bits_from(j, m),
                &format!("{row} j={j} shape#{k} mantissa={m:#08x}"),
            );
        }
        for i in 0..RANDOM_MANTISSAS_PER_EXPONENT {
            let m = rng.next_u32() & 0x007f_ffff;
            pair.check_bits(
                bits_from(j, m),
                &format!("{row} j={j} random#{i} mantissa={m:#08x}"),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Sanity: both libraries really are two distinct .so files loaded via dlopen.
// ---------------------------------------------------------------------------

#[test]
fn cfg_00_both_libraries_load_and_export_float2half() {
    let pair = Pair::load();
    assert_ne!(
        pair.c_path, pair.rust_path,
        "must load two distinct shared objects"
    );
    assert_ne!(
        pair.c as usize, pair.rust as usize,
        "C and Rust float2half must be different function pointers"
    );
    println!("C   .so: {:?}", pair.c_path);
    println!("Rust.so: {:?}", pair.rust_path);
    // Smoke: 1.0f32 -> binary16 0x3c00.
    assert_eq!(pair.c_of_bits(1.0f32.to_bits()), 0x3c00);
    assert_eq!(pair.rust_of_bits(1.0f32.to_bits()), 0x3c00);
}

// ---------------------------------------------------------------------------
// Rows 1–14: positive sign (j = 0..=255)
// ---------------------------------------------------------------------------

#[test]
fn cfg_01_pos_underflow_run() {
    run_row(&Pair::load(), "row01 +underflow shift24 base0x0000", 0..=102);
}

#[test]
fn cfg_02_pos_subnormal_j103_shift23() {
    run_row(&Pair::load(), "row02 +sub j103 shift23", [103]);
}

#[test]
fn cfg_03_pos_subnormal_j104_shift22() {
    run_row(&Pair::load(), "row03 +sub j104 shift22", [104]);
}

#[test]
fn cfg_04_pos_subnormal_j105_shift21() {
    run_row(&Pair::load(), "row04 +sub j105 shift21", [105]);
}

#[test]
fn cfg_05_pos_subnormal_j106_shift20() {
    run_row(&Pair::load(), "row05 +sub j106 shift20", [106]);
}

#[test]
fn cfg_06_pos_subnormal_j107_shift19() {
    run_row(&Pair::load(), "row06 +sub j107 shift19", [107]);
}

#[test]
fn cfg_07_pos_subnormal_j108_shift18() {
    run_row(&Pair::load(), "row07 +sub j108 shift18", [108]);
}

#[test]
fn cfg_08_pos_subnormal_j109_shift17() {
    run_row(&Pair::load(), "row08 +sub j109 shift17", [109]);
}

#[test]
fn cfg_09_pos_subnormal_j110_shift16() {
    run_row(&Pair::load(), "row09 +sub j110 shift16", [110]);
}

#[test]
fn cfg_10_pos_subnormal_j111_shift15() {
    run_row(&Pair::load(), "row10 +sub j111 shift15", [111]);
}

#[test]
fn cfg_11_pos_subnormal_j112_shift14() {
    run_row(&Pair::load(), "row11 +sub j112 shift14", [112]);
}

#[test]
fn cfg_12_pos_normal_run() {
    run_row(&Pair::load(), "row12 +normal shift13", 113..=142);
}

#[test]
fn cfg_13_pos_overflow_run() {
    run_row(&Pair::load(), "row13 +overflow shift24 base0x7c00", 143..=254);
}

#[test]
fn cfg_14_pos_inf_nan_j255() {
    run_row(&Pair::load(), "row14 +inf/nan j255 shift13", [255]);
}

// ---------------------------------------------------------------------------
// Rows 15–28: negative sign (j = 256..=511)
// ---------------------------------------------------------------------------

#[test]
fn cfg_15_neg_underflow_run() {
    run_row(
        &Pair::load(),
        "row15 -underflow shift24 base0x8000",
        256..=358,
    );
}

#[test]
fn cfg_16_neg_subnormal_j359_shift23() {
    run_row(&Pair::load(), "row16 -sub j359 shift23", [359]);
}

#[test]
fn cfg_17_neg_subnormal_j360_shift22() {
    run_row(&Pair::load(), "row17 -sub j360 shift22", [360]);
}

#[test]
fn cfg_18_neg_subnormal_j361_shift21() {
    run_row(&Pair::load(), "row18 -sub j361 shift21", [361]);
}

#[test]
fn cfg_19_neg_subnormal_j362_shift20() {
    run_row(&Pair::load(), "row19 -sub j362 shift20", [362]);
}

#[test]
fn cfg_20_neg_subnormal_j363_shift19() {
    run_row(&Pair::load(), "row20 -sub j363 shift19", [363]);
}

#[test]
fn cfg_21_neg_subnormal_j364_shift18() {
    run_row(&Pair::load(), "row21 -sub j364 shift18", [364]);
}

#[test]
fn cfg_22_neg_subnormal_j365_shift17() {
    run_row(&Pair::load(), "row22 -sub j365 shift17", [365]);
}

#[test]
fn cfg_23_neg_subnormal_j366_shift16() {
    run_row(&Pair::load(), "row23 -sub j366 shift16", [366]);
}

#[test]
fn cfg_24_neg_subnormal_j367_shift15() {
    run_row(&Pair::load(), "row24 -sub j367 shift15", [367]);
}

#[test]
fn cfg_25_neg_subnormal_j368_shift14() {
    run_row(&Pair::load(), "row25 -sub j368 shift14", [368]);
}

#[test]
fn cfg_26_neg_normal_run() {
    run_row(&Pair::load(), "row26 -normal shift13", 369..=398);
}

#[test]
fn cfg_27_neg_overflow_run() {
    run_row(
        &Pair::load(),
        "row27 -overflow shift24 base0xfc00",
        399..=510,
    );
}

#[test]
fn cfg_28_neg_inf_nan_j511() {
    run_row(&Pair::load(), "row28 -inf/nan j511 shift13", [511]);
}

// ---------------------------------------------------------------------------
// Row 29: the whole sign × exponent cross-product.
// ---------------------------------------------------------------------------

#[test]
fn cfg_29_all_512_exponent_classes_all_shapes() {
    let pair = Pair::load();
    for j in 0..512u32 {
        for (k, m) in mantissa_shapes(shift_for(j)).into_iter().enumerate() {
            pair.check_bits(bits_from(j, m), &format!("row29 j={j} shape#{k}"));
        }
    }
}

// ---------------------------------------------------------------------------
// Row 30: shift-run boundaries and one step either side.
// ---------------------------------------------------------------------------

#[test]
fn cfg_30_run_boundary_exponents() {
    const EDGES: [u32; 32] = [
        101, 102, 103, 104, // underflow -> subnormal
        111, 112, 113, 114, // subnormal -> normal
        141, 142, 143, 144, // normal -> overflow
        253, 254, 255, 0, // overflow -> inf/nan, and wrap to j=0
        357, 358, 359, 360, // negative counterparts
        367, 368, 369, 370, //
        397, 398, 399, 400, //
        509, 510, 511, 256, //
    ];
    run_row(&Pair::load(), "row30 boundaries", EDGES);
}

// ---------------------------------------------------------------------------
// Row 31: named real-world magnitudes, passed as actual float values.
// ---------------------------------------------------------------------------

#[test]
fn cfg_31_named_magnitudes() {
    let pair = Pair::load();
    let mut vals: Vec<f32> = vec![
        0.0,
        -0.0,
        1.0,
        -1.0,
        2.0,
        -2.0,
        0.5,
        -0.5,
        65504.0,   // binary16 max finite
        -65504.0,  //
        65519.996, // largest float rounding below the overflow threshold
        65520.0,   // binary16 overflow threshold
        -65520.0,
        65536.0,
        -65536.0,
        6.103515625e-5,  // binary16 min normal
        -6.103515625e-5, //
        6.0975552e-5,    // just below binary16 min normal
        5.9604645e-8,    // binary16 min subnormal
        -5.9604645e-8,
        2.9802322e-8, // half of min subnormal -> underflows
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        f32::MAX,
        f32::MIN,
        f32::EPSILON,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        -f32::NAN,
        3.14159265,
        -2.718281828,
        1.0e-45, // smallest positive float subnormal region
        -1.0e-45,
        1.0e38,
        -1.0e38,
    ];
    // Also every power of two representable as a float, both signs.
    for e in 0u32..=254 {
        vals.push(f32::from_bits(e << 23));
        vals.push(f32::from_bits((1 << 31) | (e << 23)));
    }
    for v in vals {
        pair.check_value(v, "row31 named magnitudes");
    }
}

// ---------------------------------------------------------------------------
// Row 32: one million uniformly random 32-bit patterns.
// ---------------------------------------------------------------------------

#[test]
fn cfg_32_random_bit_patterns() {
    let pair = Pair::load();
    let mut rng = Rng::new(SEED);
    for i in 0..1_000_000u32 {
        pair.check_bits(rng.next_u32(), &format!("row32 draw#{i}"));
    }
}

// ---------------------------------------------------------------------------
// Row 33: random real-valued floats spread across magnitude decades.
// ---------------------------------------------------------------------------

#[test]
fn cfg_33_random_real_values_across_decades() {
    let pair = Pair::load();
    let mut rng = Rng::new(SEED.rotate_left(17));
    for i in 0..200_000u32 {
        // Uniform mantissa, uniform exponent over the whole float range.
        let e = rng.below(255);
        let m = rng.next_u32() & 0x007f_ffff;
        let s = rng.next_u32() & 1;
        let bits = (s << 31) | (e << 23) | m;
        let x = f32::from_bits(bits);
        pair.check_value(x, &format!("row33 draw#{i}"));

        // And a value built by scaling, which exercises a different set of
        // mantissa patterns than a raw random draw.
        let scale = 2f32.powi(rng.below(90) as i32 - 45);
        let frac = (rng.next_u32() as f32) / (u32::MAX as f32);
        let y = if s == 1 { -frac * scale } else { frac * scale };
        pair.check_value(y, &format!("row33 scaled#{i}"));
    }
}
