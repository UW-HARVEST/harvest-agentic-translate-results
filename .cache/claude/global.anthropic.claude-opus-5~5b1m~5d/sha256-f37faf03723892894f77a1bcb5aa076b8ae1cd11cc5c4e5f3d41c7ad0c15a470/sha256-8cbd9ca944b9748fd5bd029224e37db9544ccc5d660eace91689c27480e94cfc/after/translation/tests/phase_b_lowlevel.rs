// Phase B -- valid-path differential tests for the LOW-LEVEL entry points.
// One test per CONFIGS.md row 1..=25. Every row uses many randomized inputs
// from a fixed seed.
mod common;
use common::*;

const N: usize = 4000;

// ===========================================================================
// safe_double_to_int -- CONFIGS rows 1..9
// ===========================================================================

#[test]
fn cfg_01_sdti_in_range_positive_integral() {
    let mut r = Rng::new(SEED ^ 1);
    cmp_sdti("cfg01", 0.0);
    cmp_sdti("cfg01", 1.0);
    cmp_sdti("cfg01", 2147483647.0);
    for _ in 0..N {
        let v = r.range_i32(0, i32::MAX) as f64;
        cmp_sdti("cfg01", v);
    }
}

#[test]
fn cfg_02_sdti_in_range_negative_integral() {
    let mut r = Rng::new(SEED ^ 2);
    cmp_sdti("cfg02", -0.0);
    cmp_sdti("cfg02", -1.0);
    cmp_sdti("cfg02", -2147483648.0);
    for _ in 0..N {
        let v = r.range_i32(i32::MIN, 0) as f64;
        cmp_sdti("cfg02", v);
    }
}

#[test]
fn cfg_03_sdti_in_range_fractional_truncation() {
    let mut r = Rng::new(SEED ^ 3);
    for _ in 0..N {
        // +-[0,1)
        let f = r.unit();
        cmp_sdti("cfg03", f);
        cmp_sdti("cfg03", -f);
        // +-[1, 1e9) with a fractional part
        let g = r.range_f64(1.0, 1e9);
        cmp_sdti("cfg03", g);
        cmp_sdti("cfg03", -g);
        // integral +- a fraction, straddling the truncation direction
        let n = r.range_i32(-2_000_000_000, 2_000_000_000) as f64;
        cmp_sdti("cfg03", n + 0.5);
        cmp_sdti("cfg03", n - 0.5);
        cmp_sdti("cfg03", n + 0.9999999);
    }
}

#[test]
fn cfg_04_sdti_magnitude_below_one() {
    let mut r = Rng::new(SEED ^ 4);
    for v in [
        0.0,
        -0.0,
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
        5e-324,
        -5e-324,
        f64::MIN_POSITIVE / 2.0,
        0.5,
        -0.5,
        0.9999999999999999,
        -0.9999999999999999,
        1e-300,
        -1e-300,
    ] {
        cmp_sdti("cfg04", v);
    }
    for _ in 0..N {
        // random subnormals: exponent field zero, random mantissa
        let bits = r.next_u64() & 0x800F_FFFF_FFFF_FFFF;
        cmp_sdti("cfg04", f64::from_bits(bits));
        // random tiny normals
        cmp_sdti("cfg04", r.unit() * 1e-8);
    }
}

#[test]
fn cfg_05_sdti_exact_boundaries_and_ulps() {
    let max = i32::MAX as f64; // 2147483647.0 exactly
    let min = i32::MIN as f64; // -2147483648.0 exactly
    let mut cases = vec![
        max,
        min,
        -max,
        -min,
        max - 1.0,
        min + 1.0,
        max + 1.0,
        min - 1.0,
        max + 2.0,
        min - 2.0,
    ];
    // one ULP either side of each boundary
    for base in [max, min, -max, -min] {
        cases.push(next_after(base, f64::INFINITY));
        cases.push(next_after(base, f64::NEG_INFINITY));
        cases.push(next_after(next_after(base, f64::INFINITY), f64::INFINITY));
        cases.push(next_after(
            next_after(base, f64::NEG_INFINITY),
            f64::NEG_INFINITY,
        ));
    }
    for v in cases {
        cmp_sdti("cfg05", v);
    }
}

#[test]
fn cfg_06_sdti_out_of_range_high() {
    let mut r = Rng::new(SEED ^ 6);
    for v in [
        2147483648.0,
        2147483648.5,
        1e10,
        1e15,
        1e300,
        f64::MAX,
        f64::INFINITY,
    ] {
        cmp_sdti("cfg06", v);
    }
    for _ in 0..N {
        cmp_sdti("cfg06", r.range_f64(2147483648.0, 1e300));
        cmp_sdti("cfg06", r.range_f64(2147483647.0, 4294967296.0));
    }
}

#[test]
fn cfg_07_sdti_out_of_range_low() {
    let mut r = Rng::new(SEED ^ 7);
    for v in [
        -2147483649.0,
        -2147483648.5,
        -1e10,
        -1e15,
        -1e300,
        f64::MIN,
        f64::NEG_INFINITY,
    ] {
        cmp_sdti("cfg07", v);
    }
    for _ in 0..N {
        cmp_sdti("cfg07", -r.range_f64(2147483649.0, 1e300));
        cmp_sdti("cfg07", -r.range_f64(2147483648.0, 4294967296.0));
    }
}

#[test]
fn cfg_08_sdti_nan_bit_patterns() {
    let mut r = Rng::new(SEED ^ 8);
    for v in [
        f64::NAN,
        -f64::NAN,
        f64::from_bits(0x7FF8_0000_0000_0001), // quiet, payload 1
        f64::from_bits(0xFFF8_0000_0000_0001), // negative quiet
        f64::from_bits(0x7FF0_0000_0000_0001), // signalling
        f64::from_bits(0xFFF0_0000_0000_0001), // negative signalling
        f64::from_bits(0x7FFF_FFFF_FFFF_FFFF),
        f64::from_bits(0xFFFF_FFFF_FFFF_FFFF),
    ] {
        assert!(v.is_nan());
        cmp_sdti("cfg08", v);
    }
    for _ in 0..N {
        // exponent all ones, non-zero mantissa => NaN with random payload
        let mantissa = (r.next_u64() & 0x000F_FFFF_FFFF_FFFF) | 1;
        let sign = (r.next_u64() & 1) << 63;
        let bits = sign | 0x7FF0_0000_0000_0000 | mantissa;
        let v = f64::from_bits(bits);
        assert!(v.is_nan());
        cmp_sdti("cfg08", v);
    }
}

#[test]
fn cfg_09_sdti_fully_random_bit_patterns() {
    let mut r = Rng::new(SEED ^ 9);
    for _ in 0..(N * 8) {
        cmp_sdti("cfg09", r.any_f64());
    }
}

// ===========================================================================
// process_with_fallthrough -- CONFIGS rows 10..17
// ===========================================================================

fn pwf_sweep_code(row: &str, code: i32, seed: u64) {
    let mut r = Rng::new(seed);
    for base in [
        0,
        1,
        -1,
        50,
        -50,
        149,
        150,
        i32::MAX,
        i32::MAX - 1,
        i32::MAX - 9,
        i32::MAX - 10,
        i32::MAX - 149,
        i32::MAX - 150,
        i32::MIN,
        i32::MIN + 1,
        i32::MIN + 149,
        i32::MIN + 150,
    ] {
        cmp_pwf(row, code, base);
    }
    for _ in 0..N {
        cmp_pwf(row, code, r.next_i32());
        cmp_pwf(row, code, r.range_i32(-1000, 1000));
        cmp_pwf(row, code, i32::MAX - r.range_i32(0, 300));
        cmp_pwf(row, code, i32::MIN + r.range_i32(0, 300));
    }
}

#[test]
fn cfg_10_pwf_code_5() {
    pwf_sweep_code("cfg10", 5, SEED ^ 10);
}

#[test]
fn cfg_11_pwf_code_4() {
    pwf_sweep_code("cfg11", 4, SEED ^ 11);
}

#[test]
fn cfg_12_pwf_code_3() {
    pwf_sweep_code("cfg12", 3, SEED ^ 12);
}

#[test]
fn cfg_13_pwf_code_2() {
    pwf_sweep_code("cfg13", 2, SEED ^ 13);
}

#[test]
fn cfg_14_pwf_code_1() {
    pwf_sweep_code("cfg14", 1, SEED ^ 14);
}

#[test]
fn cfg_15_pwf_code_0_discards_base() {
    pwf_sweep_code("cfg15", 0, SEED ^ 15);
}

#[test]
fn cfg_16_pwf_code_default_random() {
    let mut r = Rng::new(SEED ^ 16);
    let mut done = 0usize;
    while done < N * 4 {
        let code = r.next_i32();
        if (0..=5).contains(&code) {
            continue;
        }
        cmp_pwf("cfg16", code, r.next_i32());
        done += 1;
    }
}

#[test]
fn cfg_17_pwf_full_cross_product() {
    let codes: Vec<i32> = (-2..=8)
        .chain([i32::MIN, i32::MAX, i32::MIN + 1, i32::MAX - 1, 100, -100])
        .collect();
    let bases = [
        i32::MIN,
        i32::MIN + 1,
        i32::MIN + 149,
        -1,
        0,
        1,
        149,
        i32::MAX - 149,
        i32::MAX - 1,
        i32::MAX,
    ];
    for &code in &codes {
        for &base in &bases {
            cmp_pwf("cfg17", code, base);
        }
    }
}

// ===========================================================================
// copy_data_block -- CONFIGS rows 18..22
// ===========================================================================

fn block_bytes(id: i32, value: f64, label: &[u8], pad: u8) -> [u8; DATABLOCK_SIZE] {
    let mut raw = [pad; DATABLOCK_SIZE];
    raw[0..4].copy_from_slice(&id.to_ne_bytes());
    raw[8..16].copy_from_slice(&value.to_ne_bytes());
    let n = label.len().min(20);
    raw[16..16 + n].copy_from_slice(&label[..n]);
    for i in (16 + n)..36 {
        raw[i] = 0;
    }
    raw
}

#[test]
fn cfg_18_cdb_well_formed_random_blocks() {
    let mut r = Rng::new(SEED ^ 18);
    for _ in 0..N {
        let id = r.next_i32();
        let value = r.range_f64(-1e12, 1e12);
        let len = (r.next_u64() % 20) as usize;
        let mut label = vec![0u8; len];
        for byte in label.iter_mut() {
            *byte = r.range_i32(1, 126) as u8;
        }
        let raw = block_bytes(id, value, &label, 0);
        cmp_cdb("cfg18", &raw, 0);
    }
}

#[test]
fn cfg_19_cdb_special_double_values() {
    let specials = [
        f64::NAN,
        -f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        0.0,
        -0.0,
        f64::MIN_POSITIVE,
        5e-324,
        f64::MAX,
        f64::MIN,
    ];
    for &v in &specials {
        for &id in &[i32::MIN, -1, 0, 1, i32::MAX] {
            let raw = block_bytes(id, v, b"Source", 0);
            cmp_cdb("cfg19", &raw, 0);
            cmp_cdb("cfg19", &raw, 0xAA);
        }
    }
}

#[test]
fn cfg_20_cdb_unterminated_and_uniform_labels() {
    // 20 non-zero bytes -> no NUL terminator anywhere in `label`
    let raw = block_bytes(42, 1.5, &[b'X'; 20], 0);
    cmp_cdb("cfg20", &raw, 0);
    cmp_cdb("cfg20", &raw, 0xAA);

    cmp_cdb("cfg20", &[0x00; DATABLOCK_SIZE], 0xAA);
    cmp_cdb("cfg20", &[0xFF; DATABLOCK_SIZE], 0x00);
    cmp_cdb("cfg20", &[0xFF; DATABLOCK_SIZE], 0xFF);
    cmp_cdb("cfg20", &[0x80; DATABLOCK_SIZE], 0x7F);
}

#[test]
fn cfg_21_cdb_fully_random_40_byte_images_incl_padding() {
    let mut r = Rng::new(SEED ^ 21);
    for _ in 0..N {
        let mut raw = [0u8; DATABLOCK_SIZE];
        for byte in raw.iter_mut() {
            *byte = r.next_u32() as u8;
        }
        cmp_cdb("cfg21", &raw, 0);
    }
}

#[test]
fn cfg_22_cdb_prefilled_destination() {
    let mut r = Rng::new(SEED ^ 22);
    for fill in [0x00u8, 0x01, 0x55, 0xAA, 0xFF, 0x7F, 0x80] {
        for _ in 0..200 {
            let mut raw = [0u8; DATABLOCK_SIZE];
            for byte in raw.iter_mut() {
                *byte = r.next_u32() as u8;
            }
            cmp_cdb("cfg22", &raw, fill);
        }
    }
}

// ===========================================================================
// handle_pointer_operations -- CONFIGS rows 23..25
// ===========================================================================

#[test]
fn cfg_23_hpo_small_values() {
    let mut r = Rng::new(SEED ^ 23);
    for v in -200..=200 {
        cmp_hpo("cfg23", v);
    }
    for _ in 0..N {
        cmp_hpo("cfg23", r.range_i32(-1_000_000, 1_000_000));
    }
}

#[test]
fn cfg_24_hpo_full_range_random() {
    let mut r = Rng::new(SEED ^ 24);
    for _ in 0..(N * 4) {
        cmp_hpo("cfg24", r.next_i32());
    }
}

#[test]
fn cfg_25_hpo_boundaries() {
    for v in [
        i32::MIN,
        i32::MIN + 1,
        i32::MIN + 50,
        i32::MIN / 2 - 1,
        i32::MIN / 2,
        -51,
        -50,
        -1,
        0,
        1,
        50,
        i32::MAX / 2 - 1,
        i32::MAX / 2,
        i32::MAX / 2 + 1,
        1 << 30,
        (1 << 30) + 1,
        i32::MAX - 1,
        i32::MAX,
    ] {
        cmp_hpo("cfg25", v);
    }
    for k in 0..31 {
        cmp_hpo("cfg25", 1i32 << k);
        cmp_hpo("cfg25", -(1i32 << k));
        cmp_hpo("cfg25", (1i32 << k) - 1);
    }
}

// ---------------------------------------------------------------------------
// `nextafter` without pulling in libm bindings.
// ---------------------------------------------------------------------------
fn next_after(x: f64, toward: f64) -> f64 {
    if x.is_nan() || toward.is_nan() {
        return f64::NAN;
    }
    if x == toward {
        return toward;
    }
    if x == 0.0 {
        return if toward > 0.0 { 5e-324 } else { -5e-324 };
    }
    let bits = x.to_bits();
    let going_up = toward > x;
    let next = if (x > 0.0) == going_up { bits + 1 } else { bits - 1 };
    f64::from_bits(next)
}
