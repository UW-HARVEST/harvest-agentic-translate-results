use Math_Library_in_C::castom_math::*;

const TOL: f64 = EPS_6;

fn approx_eq(a: f64, b: f64) -> bool {
    (a - b).abs() < TOL
}

// ==================== abs ====================

#[test]
fn test_abs_positive() {
    assert_eq!(castom_abs(5), 5);
}

#[test]
fn test_abs_negative() {
    assert_eq!(castom_abs(-5), 5);
}

#[test]
fn test_abs_zero() {
    assert_eq!(castom_abs(0), 0);
}

#[test]
fn test_abs_range() {
    for i in -1000..=1000 {
        assert_eq!(castom_abs(i), i.abs());
    }
}

// ==================== fabs ====================

#[test]
fn test_fabs_positive() {
    assert!(approx_eq(castom_fabs(15.5), 15.5));
}

#[test]
fn test_fabs_negative() {
    assert!(approx_eq(castom_fabs(-15.3), 15.3));
}

#[test]
fn test_fabs_zero() {
    assert!(approx_eq(castom_fabs(0.0), 0.0));
}

#[test]
fn test_fabs_small() {
    assert!(approx_eq(castom_fabs(0.000004), 0.000004));
    assert!(approx_eq(castom_fabs(-0.000004), 0.000004));
}

// ==================== floor ====================

#[test]
fn test_floor_positive() {
    assert_eq!(castom_floor(2.5), 2.0);
    assert_eq!(castom_floor(1.000001), 1.0);
    assert_eq!(castom_floor(8.44), 8.0);
    assert_eq!(castom_floor(0.1), 0.0);
    assert_eq!(castom_floor(1000000.0), 1000000.0);
    assert_eq!(castom_floor(0.0), 0.0);
}

#[test]
fn test_floor_negative() {
    assert_eq!(castom_floor(-2.5), -3.0);
    assert_eq!(castom_floor(-1.000001), -2.0);
    assert_eq!(castom_floor(-8.44), -9.0);
    assert_eq!(castom_floor(-0.1), -1.0);
    assert_eq!(castom_floor(-500.55), -501.0);
    assert_eq!(castom_floor(-1.5), -2.0);
}

// ==================== ceil ====================

#[test]
fn test_ceil_positive() {
    assert_eq!(castom_ceil(2.5), 3.0);
    assert_eq!(castom_ceil(1.000001), 2.0);
    assert_eq!(castom_ceil(8.44), 9.0);
    assert_eq!(castom_ceil(0.1), 1.0);
    assert_eq!(castom_ceil(1000000.0), 1000000.0);
    assert_eq!(castom_ceil(0.0), 0.0);
}

#[test]
fn test_ceil_negative() {
    assert_eq!(castom_ceil(-2.5), -2.0);
    assert_eq!(castom_ceil(-8.44), -8.0);
    assert_eq!(castom_ceil(-0.1), 0.0);
    assert_eq!(castom_ceil(-500.55), -500.0);
}

// ==================== trunc ====================

#[test]
fn test_trunc_positive() {
    assert_eq!(castom_trunc(2.5), 2.0);
    assert_eq!(castom_trunc(1.000001), 1.0);
    assert_eq!(castom_trunc(8.44), 8.0);
    assert_eq!(castom_trunc(0.1), 0.0);
    assert_eq!(castom_trunc(0.0), 0.0);
}

#[test]
fn test_trunc_negative() {
    assert_eq!(castom_trunc(-2.5), -2.0);
    assert_eq!(castom_trunc(-1.000001), -1.0);
    assert_eq!(castom_trunc(-8.44), -8.0);
    assert_eq!(castom_trunc(-0.1), 0.0);
    assert_eq!(castom_trunc(-500.55), -500.0);
    assert_eq!(castom_trunc(-1.5), -1.0);
}

// ==================== fmod ====================

#[test]
fn test_fmod_basic() {
    assert!(approx_eq(castom_fmod(10.5, -3.0), 1.5));
    assert!(approx_eq(castom_fmod(-8.1, 4.0), -8.1_f64 % 4.0));
    assert!(approx_eq(castom_fmod(0.0, 1.0), 0.0));
    assert!(approx_eq(castom_fmod(1.5, 100.0), 1.5));
}

// ==================== exp ====================

#[test]
fn test_exp_zero() {
    assert!(approx_eq(castom_exp(0.0), 1.0));
}

#[test]
fn test_exp_one() {
    assert!(approx_eq(castom_exp(1.0), std::f64::consts::E));
}

#[test]
fn test_exp_positive() {
    assert!(approx_eq(castom_exp(5.0), 5.0_f64.exp()));
    assert!(approx_eq(castom_exp(19.0), 19.0_f64.exp()));
}

#[test]
fn test_exp_negative() {
    assert!(approx_eq(castom_exp(-5.0), (-5.0_f64).exp()));
    assert!(approx_eq(castom_exp(-15.0), (-15.0_f64).exp()));
    assert!(approx_eq(castom_exp(-20.0), (-20.0_f64).exp()));
}

#[test]
fn test_exp_small() {
    assert!(approx_eq(castom_exp(0.0000001), 0.0000001_f64.exp()));
}

// ==================== log ====================

#[test]
fn test_log_one() {
    assert!(approx_eq(castom_log(1.0), 0.0));
}

#[test]
fn test_log_positive() {
    assert!(approx_eq(castom_log(8.0), 8.0_f64.ln()));
    assert!(approx_eq(castom_log(0.1), 0.1_f64.ln()));
    assert!(approx_eq(castom_log(500.0), 500.0_f64.ln()));
    assert!(approx_eq(castom_log(1000000.0), 1000000.0_f64.ln()));
    assert!(approx_eq(castom_log(0.0000001), 0.0000001_f64.ln()));
    assert!(approx_eq(castom_log(1.5), 1.5_f64.ln()));
    assert!(approx_eq(castom_log(2.0), 2.0_f64.ln()));
}

#[test]
fn test_log_zero() {
    assert_eq!(castom_log(0.0), f64::NEG_INFINITY);
}

#[test]
fn test_log_negative_is_nan() {
    assert!(castom_log(-1.0).is_nan());
    assert!(castom_log(-100.0).is_nan());
}

// ==================== sqrt ====================

#[test]
fn test_sqrt_basic() {
    assert!(approx_eq(castom_sqrt(1.0), 1.0));
    assert!(approx_eq(castom_sqrt(4.0), 2.0));
    assert!(approx_eq(castom_sqrt(8.0), 8.0_f64.sqrt()));
    assert!(approx_eq(castom_sqrt(0.1), 0.1_f64.sqrt()));
    assert!(approx_eq(castom_sqrt(500.0), 500.0_f64.sqrt()));
    assert!(approx_eq(castom_sqrt(1000000.0), 1000.0));
    assert!(approx_eq(castom_sqrt(2.0), 2.0_f64.sqrt()));
}

#[test]
fn test_sqrt_zero() {
    assert_eq!(castom_sqrt(0.0), 0.0);
}

#[test]
fn test_sqrt_negative_is_nan() {
    assert!(castom_sqrt(-2.0).is_nan());
    assert!(castom_sqrt(-1.0).is_nan());
}

// ==================== sin ====================

#[test]
fn test_sin_zero() {
    assert!(approx_eq(castom_sin(0.0), 0.0));
}

#[test]
fn test_sin_special_angles() {
    assert!(approx_eq(castom_sin(S21_M_PI / 6.0), 0.5));
    assert!(approx_eq(castom_sin(S21_M_PI / 4.0), std::f64::consts::FRAC_1_SQRT_2));
    assert!(approx_eq(castom_sin(S21_M_PI / 2.0), 1.0));
    assert!(approx_eq(castom_sin(S21_M_PI), 0.0));
    assert!(approx_eq(castom_sin(2.0 * S21_M_PI), 0.0));
}

#[test]
fn test_sin_range() {
    let mut j = -10.0;
    while j <= 10.0 {
        assert!(approx_eq(castom_sin(j), j.sin()), "sin({}) failed: got {} expected {}", j, castom_sin(j), j.sin());
        j += 0.5;
    }
}

// ==================== cos ====================

#[test]
fn test_cos_zero() {
    assert!(approx_eq(castom_cos(0.0), 1.0));
}

#[test]
fn test_cos_special() {
    assert!(approx_eq(castom_cos(S21_M_PI / 3.0), 0.5));
    assert!(approx_eq(castom_cos(S21_M_PI), -1.0));
}

#[test]
fn test_cos_range() {
    let mut j = -10.0;
    while j <= 10.0 {
        assert!(approx_eq(castom_cos(j), j.cos()), "cos({}) failed: got {} expected {}", j, castom_cos(j), j.cos());
        j += 0.5;
    }
}

// ==================== tan ====================

#[test]
fn test_tan_zero() {
    assert_eq!(castom_tan(0.0), 0.0);
}

#[test]
fn test_tan_special_angles() {
    assert!(approx_eq(castom_tan(S21_M_PI / 4.0), 1.0));
    assert!(approx_eq(castom_tan(S21_M_PI), 0.0));
    assert!(approx_eq(castom_tan(2.0 * S21_M_PI), 0.0));
}

#[test]
fn test_tan_pi_over_2_is_inf() {
    assert_eq!(castom_tan(S21_M_PI / 2.0), S21_INFINITY);
}

#[test]
fn test_tan_range() {
    let mut j: f64 = -10.0;
    while j <= 10.0 {
        let expected = j.tan();
        let got = castom_tan(j);
        // Skip near-singularity points where tan diverges
        if expected.abs() < 1000.0 {
            assert!(approx_eq(got, expected), "tan({}) failed: got {} expected {}", j, got, expected);
        }
        j += 0.1;
    }
}

// ==================== asin ====================

#[test]
fn test_asin_boundaries() {
    assert!(approx_eq(castom_asin(1.0), S21_M_PI / 2.0));
    assert!(approx_eq(castom_asin(-1.0), -S21_M_PI / 2.0));
}

#[test]
fn test_asin_zero() {
    assert!(approx_eq(castom_asin(0.0), 0.0));
}

#[test]
fn test_asin_out_of_range_is_nan() {
    assert!(castom_asin(2.0).is_nan());
    assert!(castom_asin(-2.0).is_nan());
    assert!(castom_asin(5.0).is_nan());
}

#[test]
fn test_asin_range() {
    let mut j = -0.99;
    while j <= 0.99 {
        assert!(approx_eq(castom_asin(j), j.asin()), "asin({}) failed: got {} expected {}", j, castom_asin(j), j.asin());
        j += 0.1;
    }
}

// ==================== acos ====================

#[test]
fn test_acos_zero() {
    assert!(approx_eq(castom_acos(0.0), S21_M_PI / 2.0));
}

#[test]
fn test_acos_range() {
    let mut j = -0.99;
    while j <= 0.99 {
        assert!(approx_eq(castom_acos(j), j.acos()), "acos({}) failed: got {} expected {}", j, castom_acos(j), j.acos());
        j += 0.1;
    }
}

#[test]
fn test_acos_out_of_range_is_nan() {
    assert!(castom_acos(2.0).is_nan());
    assert!(castom_acos(-2.0).is_nan());
}

// ==================== atan ====================

#[test]
fn test_atan_zero() {
    assert!(approx_eq(castom_atan(0.0), 0.0));
}

#[test]
fn test_atan_one() {
    assert!(approx_eq(castom_atan(1.0), 1.0_f64.atan()));
}

#[test]
fn test_atan_range() {
    let mut j = -100.0;
    while j < 100.0 {
        assert!(approx_eq(castom_atan(j), j.atan()), "atan({}) failed: got {} expected {}", j, castom_atan(j), j.atan());
        j += 5.0;
    }
}

// ==================== pow ====================

#[test]
fn test_pow_zero_exp() {
    assert_eq!(castom_pow(0.0, 0.0), 1.0);
    assert_eq!(castom_pow(3.0, 0.0), 1.0);
    assert_eq!(castom_pow(2.0, 0.0), 1.0);
}

#[test]
fn test_pow_zero_base() {
    assert_eq!(castom_pow(0.0, 0.5), 0.0);
    assert_eq!(castom_pow(0.0, 5.0), 0.0);
}

#[test]
fn test_pow_basic() {
    assert!(approx_eq(castom_pow(1.0, 1.0), 1.0));
    assert!(approx_eq(castom_pow(3.0, 2.0), 9.0));
    assert!(approx_eq(castom_pow(3.1, 4.0), 3.1_f64.powi(4)));
    assert!(approx_eq(castom_pow(3.1, 4.2), 3.1_f64.powf(4.2)));
}

#[test]
fn test_pow_negative_base_integer_exp() {
    assert!(approx_eq(castom_pow(-1.5, 8.0), (-1.5_f64).powi(8)));
    assert!(approx_eq(castom_pow(-100.0, 2.0), 10000.0));
    assert!(approx_eq(castom_pow(-2.0, 3.0), -8.0));
}

#[test]
fn test_pow_negative_base_fractional_exp_is_nan() {
    assert!(castom_pow(-2.0, 0.5).is_nan());
}

#[test]
fn test_pow_negative_exp() {
    assert!(approx_eq(castom_pow(1.5, -100.0), 1.5_f64.powf(-100.0)));
}

// ==================== factorial ====================

#[test]
fn test_factorial_zero() {
    assert_eq!(castom_factorial(0), 1);
}

#[test]
fn test_factorial_one() {
    assert_eq!(castom_factorial(1), 1);
}

#[test]
fn test_factorial_small() {
    assert_eq!(castom_factorial(2), 2);
    assert_eq!(castom_factorial(3), 6);
    assert_eq!(castom_factorial(4), 24);
    assert_eq!(castom_factorial(5), 120);
    assert_eq!(castom_factorial(10), 3628800);
}

#[test]
fn test_factorial_large() {
    assert_eq!(castom_factorial(20), 2432902008176640000);
}

fn main() {}
