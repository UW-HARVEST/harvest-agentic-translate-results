//! Phase B — valid-path differential tests, one test per row of `CONFIGS.md`.
//!
//! Both implementations are reached only through `dlsym("pow43")` on their
//! respective shared objects; results are compared bit-for-bit.

mod harness;

use harness::*;

const N: usize = 4000;

// --- Row 1 ----------------------------------------------------------------
#[test]
fn cfg_row01_path_a_negative_table_half() {
    // exhaustive: only 16 inputs reach g_pow43[0..=15]
    let n = assert_all_bit_eq(DOMAIN_MIN..=-1, "row 1");
    assert_eq!(n, 16);
}

// --- Row 2 ----------------------------------------------------------------
#[test]
fn cfg_row02_path_a_zero() {
    assert_bit_eq(0, "row 2");
    let (c, r) = both(0);
    // g_pow43[16] == 0 -- verify the sign of zero survives identically.
    assert_eq!(c.to_bits(), r.to_bits(), "row 2: signed zero mismatch");
    assert_eq!(c, 0.0, "row 2: C did not return the zero table entry");
    assert_eq!(r, 0.0, "row 2: Rust did not return the zero table entry");
}

// --- Row 3 ----------------------------------------------------------------
#[test]
fn cfg_row03_path_a_positive() {
    let n = assert_all_bit_eq(1..=128, "row 3");
    assert_eq!(n, 128);
}

// --- Rows 4, 5 ------------------------------------------------------------
#[test]
fn cfg_row04_path_a_boundary_domain_min() {
    assert_bit_eq(DOMAIN_MIN, "row 4");
}

#[test]
fn cfg_row05_path_a_boundary_128() {
    assert_bit_eq(128, "row 5");
    assert_bit_eq(BRANCH_A - 1, "row 5");
}

// --- Rows 6, 7 (path B, sign selected by bit 2 of x) ----------------------
#[test]
fn cfg_row06_path_b_sign_zero() {
    let xs = sample_where(6, BRANCH_A, BRANCH_B - 1, N, |x| {
        // after `x <<= 3`, sign = (2 * (x<<3)) & 64 == (16*x) & 64 == 0
        (x.wrapping_shl(3).wrapping_mul(2) & 64) == 0
    });
    assert_all_bit_eq(xs, "row 6");
}

#[test]
fn cfg_row07_path_b_sign_64() {
    let xs = sample_where(7, BRANCH_A, BRANCH_B - 1, N, |x| {
        (x.wrapping_shl(3).wrapping_mul(2) & 64) == 64
    });
    assert_all_bit_eq(xs, "row 7");
}

// --- Rows 8, 9 ------------------------------------------------------------
#[test]
fn cfg_row08_path_b_boundary_129() {
    assert_bit_eq(BRANCH_A, "row 8");
}

#[test]
fn cfg_row09_path_b_boundary_1023() {
    assert_bit_eq(BRANCH_B - 1, "row 9");
}

// --- Rows 10, 11 (frac shape on path B) ----------------------------------
#[test]
fn cfg_row10_path_b_frac_zero() {
    // (x<<3) & 63 == 0  &&  sign == 0  =>  numerator 0  =>  frac == 0
    let xs: Vec<i32> = (BRANCH_A..BRANCH_B)
        .filter(|x| {
            let s = x.wrapping_shl(3);
            (s & 63) == 0 && (s.wrapping_mul(2) & 64) == 0
        })
        .collect();
    assert!(!xs.is_empty(), "row 10: no frac==0 inputs on path B");
    assert_all_bit_eq(xs, "row 10");
}

#[test]
fn cfg_row11_path_b_frac_negative() {
    let xs: Vec<i32> = (BRANCH_A..BRANCH_B)
        .filter(|x| {
            let s = x.wrapping_shl(3);
            let sign = s.wrapping_mul(2) & 64;
            sign == 64 && ((s & 63) - sign) < 0
        })
        .collect();
    assert!(!xs.is_empty(), "row 11: no negative-frac inputs on path B");
    assert_all_bit_eq(xs, "row 11");
}

// --- Row 12 ---------------------------------------------------------------
#[test]
fn cfg_row12_path_b_exhaustive() {
    let n = assert_all_bit_eq(BRANCH_A..BRANCH_B, "row 12");
    assert_eq!(n, 895);
}

// --- Rows 13, 14 (path C, sign selected by bit 5 of x) -------------------
#[test]
fn cfg_row13_path_c_sign_zero() {
    let xs = sample_where(13, BRANCH_B, DOMAIN_MAX, N, |x| (x.wrapping_mul(2) & 64) == 0);
    assert_all_bit_eq(xs, "row 13");
}

#[test]
fn cfg_row14_path_c_sign_64() {
    let xs = sample_where(14, BRANCH_B, DOMAIN_MAX, N, |x| (x.wrapping_mul(2) & 64) == 64);
    assert_all_bit_eq(xs, "row 14");
}

// --- Row 15 ---------------------------------------------------------------
#[test]
fn cfg_row15_path_c_boundary_1024() {
    assert_bit_eq(BRANCH_B, "row 15");
}

// --- Rows 16..19 (frac shape on path C) ----------------------------------
#[test]
fn cfg_row16_path_c_frac_zero_multiples_of_64() {
    let xs = sample_where(16, BRANCH_B, DOMAIN_MAX, 512, |x| (x & 63) == 0);
    assert_all_bit_eq(xs, "row 16");
}

#[test]
fn cfg_row17_path_c_x_mod_64_eq_63() {
    let xs = sample_where(17, BRANCH_B, DOMAIN_MAX, 512, |x| (x & 63) == 63);
    assert_all_bit_eq(xs, "row 17");
}

#[test]
fn cfg_row18_path_c_x_mod_64_eq_32() {
    let xs = sample_where(18, BRANCH_B, DOMAIN_MAX, 512, |x| (x & 63) == 32);
    assert_all_bit_eq(xs, "row 18");
}

#[test]
fn cfg_row19_path_c_x_mod_64_eq_31() {
    let xs = sample_where(19, BRANCH_B, DOMAIN_MAX, 512, |x| (x & 63) == 31);
    assert_all_bit_eq(xs, "row 19");
}

// --- Row 20 ---------------------------------------------------------------
#[test]
fn cfg_row20_path_c_boundary_domain_max() {
    assert_bit_eq(DOMAIN_MAX, "row 20");
}

// --- Row 21 ---------------------------------------------------------------
#[test]
fn cfg_row21_path_c_exhaustive() {
    let n = assert_all_bit_eq(BRANCH_B..=DOMAIN_MAX, "row 21");
    assert_eq!(n, (DOMAIN_MAX - BRANCH_B + 1) as usize);
}

// --- Row 22 ---------------------------------------------------------------
#[test]
fn cfg_row22_full_domain_exhaustive() {
    let n = assert_all_bit_eq(DOMAIN_MIN..=DOMAIN_MAX, "row 22");
    assert_eq!(n, 8240, "row 22: expected the full [-16, 8223] domain");
}

// --- Row 23 ---------------------------------------------------------------
#[test]
fn cfg_row23_branch_boundaries_agree_on_both_sides() {
    for x in [128, 129, 1023, 1024] {
        assert_bit_eq(x, "row 23");
    }
    // The two sides of a boundary take different code paths; make sure the
    // *pair* is consistent between implementations, not just each value.
    let (c128, r128) = both(128);
    let (c129, r129) = both(129);
    assert_eq!((c128.to_bits(), c129.to_bits()), (r128.to_bits(), r129.to_bits()));
    let (c1023, r1023) = both(1023);
    let (c1024, r1024) = both(1024);
    assert_eq!((c1023.to_bits(), c1024.to_bits()), (r1023.to_bits(), r1024.to_bits()));
}

// --- Randomized sweep across the whole domain (property style) ------------
#[test]
fn cfg_randomized_whole_domain() {
    let xs = sample_where(99, DOMAIN_MIN, DOMAIN_MAX, 50_000, |_| true);
    assert_all_bit_eq(xs, "randomized whole domain");
}
