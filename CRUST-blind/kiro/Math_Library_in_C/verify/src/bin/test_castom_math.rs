use Math_Library_in_C::castom_math::*;

const EPS: f64 = 1e-06;

fn assert_near(a: f64, b: f64, eps: f64) {
    assert!((a - b).abs() < eps, "expected {b}, got {a}");
}

// ===== abs =====
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
fn test_abs_large_negative() {
    assert_eq!(castom_abs(-2147483647), 2147483647);
}

#[test]
fn test_abs_range() {
    for i in -1000..=1000 {
        assert_eq!(castom_abs(i), i.abs());
    }
}

// ===== fabs =====
#[test]
fn test_fabs_positive() {
    assert_eq!(castom_fabs(15.5), 15.5);
}

#[test]
fn test_fabs_negative() {
    assert_near(castom_fabs(-15.3), 15.3, EPS);
}

#[test]
fn test_fabs_zero() {
    // C returns -0.0 for fabs(0.0) due to else branch negating; Rust does same
    let r = castom_fabs(0.0);
    assert!(r == 0.0); // -0.0 == 0.0 in IEEE754
}

#[test]
fn test_fabs_small_negative() {
    assert_near(castom_fabs(-0.000004), 0.000004, EPS);
}

// ===== floor =====
#[test]
fn test_floor_positive_frac() {
    assert_eq!(castom_floor(1.7), 1.0);
}

#[test]
fn test_floor_negative_frac() {
    assert_eq!(castom_floor(-1.7), -2.0);
}

#[test]
fn test_floor_zero() {
    assert_eq!(castom_floor(0.0), 0.0);
}

#[test]
fn test_floor_positive() {
    assert_eq!(castom_floor(8.44), 8.0);
}

#[test]
fn test_floor_small_negative() {
    assert_eq!(castom_floor(-0.1), -1.0);
}

#[test]
fn test_floor_nan() {
    assert!(castom_floor(f64::NAN).is_nan());
}

// ===== ceil =====
#[test]
fn test_ceil_positive_frac() {
    assert_eq!(castom_ceil(1.7), 2.0);
}

#[test]
fn test_ceil_negative_frac() {
    assert_eq!(castom_ceil(-1.7), -1.0);
}

#[test]
fn test_ceil_zero() {
    assert_eq!(castom_ceil(0.0), 0.0);
}

#[test]
fn test_ceil_positive() {
    assert_eq!(castom_ceil(8.44), 9.0);
}

#[test]
fn test_ceil_small_negative() {
    assert_eq!(castom_ceil(-0.1), 0.0);
}

#[test]
fn test_ceil_nan() {
    assert!(castom_ceil(f64::NAN).is_nan());
}

// ===== trunc =====
#[test]
fn test_trunc_positive() {
    assert_eq!(castom_trunc(1.7), 1.0);
}

#[test]
fn test_trunc_negative() {
    assert_eq!(castom_trunc(-1.7), -1.0);
}

#[test]
fn test_trunc_zero() {
    assert_eq!(castom_trunc(0.0), 0.0);
}

#[test]
fn test_trunc_positive2() {
    assert_eq!(castom_trunc(8.44), 8.0);
}

// ===== exp =====
#[test]
fn test_exp_zero() {
    assert_eq!(castom_exp(0.0), 1.0);
}

#[test]
fn test_exp_one() {
    assert_near(castom_exp(1.0), 2.71828182845822974799, EPS);
}

#[test]
fn test_exp_neg_one() {
    assert_near(castom_exp(-1.0), 0.36787944117216190627, EPS);
}

#[test]
fn test_exp_five() {
    assert_near(castom_exp(5.0), 148.41315910257242184633, EPS);
}

#[test]
fn test_exp_neg_five() {
    assert_near(castom_exp(-5.0), 0.00673794699606389300, EPS);
}

#[test]
fn test_exp_half() {
    assert_near(castom_exp(0.5), 1.64872127069959813293, EPS);
}

// ===== log =====
#[test]
fn test_log_one() {
    assert_eq!(castom_log(1.0), 0.0);
}

#[test]
fn test_log_two() {
    assert_near(castom_log(2.0), 0.69314718055994506418, EPS);
}

#[test]
fn test_log_half() {
    assert_near(castom_log(0.5), -0.69314718055994506418, EPS);
}

#[test]
fn test_log_ten() {
    assert_near(castom_log(10.0), 2.30258509299404590109, EPS);
}

#[test]
fn test_log_zero() {
    assert_eq!(castom_log(0.0), f64::NEG_INFINITY);
}

#[test]
fn test_log_negative() {
    assert!(castom_log(-1.0).is_nan());
}

#[test]
fn test_log_million() {
    assert_near(castom_log(1000000.0), 13.81551055796427540656, EPS);
}

// ===== sqrt =====
#[test]
fn test_sqrt_four() {
    assert_eq!(castom_sqrt(4.0), 2.0);
}

#[test]
fn test_sqrt_two() {
    assert_near(castom_sqrt(2.0), 1.41421356237309492343, EPS);
}

#[test]
fn test_sqrt_zero() {
    assert_eq!(castom_sqrt(0.0), 0.0);
}

#[test]
fn test_sqrt_quarter() {
    assert_eq!(castom_sqrt(0.25), 0.5);
}

#[test]
fn test_sqrt_million() {
    assert_eq!(castom_sqrt(1000000.0), 1000.0);
}

#[test]
fn test_sqrt_negative() {
    assert!(castom_sqrt(-2.0).is_nan());
}

// ===== sin =====
#[test]
fn test_sin_zero() {
    assert_eq!(castom_sin(0.0), 0.0);
}

#[test]
fn test_sin_one() {
    assert_near(castom_sin(1.0), 0.84147098480789651572, EPS);
}

#[test]
fn test_sin_neg_one() {
    assert_near(castom_sin(-1.0), -0.84147098480789651572, EPS);
}

#[test]
fn test_sin_half() {
    assert_near(castom_sin(0.5), 0.47942553860420300145, EPS);
}

#[test]
fn test_sin_pi() {
    assert_near(castom_sin(S21_M_PI), 0.0, EPS);
}

// ===== cos =====
#[test]
fn test_cos_zero() {
    assert_eq!(castom_cos(0.0), 1.0);
}

#[test]
fn test_cos_one() {
    assert_near(castom_cos(1.0), 0.54030230587956282179, EPS);
}

#[test]
fn test_cos_neg_one() {
    assert_near(castom_cos(-1.0), 0.54030230587956282179, EPS);
}

#[test]
fn test_cos_half() {
    assert_near(castom_cos(0.5), 0.87758256188986372891, EPS);
}

#[test]
fn test_cos_pi() {
    assert_near(castom_cos(S21_M_PI), -1.0, EPS);
}

// ===== tan =====
#[test]
fn test_tan_zero() {
    assert_eq!(castom_tan(0.0), 0.0);
}

#[test]
fn test_tan_one() {
    assert_near(castom_tan(1.0), 1.55740772462197543788, EPS);
}

#[test]
fn test_tan_neg_one() {
    assert_near(castom_tan(-1.0), -1.55740772462197543788, EPS);
}

#[test]
fn test_tan_half() {
    assert_near(castom_tan(0.5), 0.54630248984410736339, EPS);
}

#[test]
fn test_tan_45() {
    assert_near(castom_tan(45.0), 1.61977519049884393460, EPS);
}

#[test]
fn test_tan_pi() {
    assert_eq!(castom_tan(S21_M_PI), 0.0);
}

#[test]
fn test_tan_pi_over_4() {
    assert_eq!(castom_tan(S21_M_PI / 4.0), 1.0);
}

#[test]
fn test_tan_pi_over_2() {
    assert_eq!(castom_tan(S21_M_PI / 2.0), f64::INFINITY);
}

// ===== asin =====
#[test]
fn test_asin_zero() {
    assert_eq!(castom_asin(0.0), 0.0);
}

#[test]
fn test_asin_half() {
    assert_near(castom_asin(0.5), 0.52359877559765843609, EPS);
}

#[test]
fn test_asin_neg_half() {
    assert_near(castom_asin(-0.5), -0.52359877559765843609, EPS);
}

#[test]
fn test_asin_one() {
    assert_near(castom_asin(1.0), S21_M_PI / 2.0, EPS);
}

#[test]
fn test_asin_neg_one() {
    assert_near(castom_asin(-1.0), -S21_M_PI / 2.0, EPS);
}

#[test]
fn test_asin_099() {
    assert_near(castom_asin(0.99), 1.42925685346809867903, EPS);
}

#[test]
fn test_asin_out_of_range() {
    assert!(castom_asin(2.0).is_nan());
}

// ===== acos =====
#[test]
fn test_acos_zero() {
    assert_near(castom_acos(0.0), S21_M_PI / 2.0, EPS);
}

#[test]
fn test_acos_half() {
    assert_near(castom_acos(0.5), 1.04719755119723812191, EPS);
}

#[test]
fn test_acos_neg_half() {
    assert_near(castom_acos(-0.5), 2.09439510239255499398, EPS);
}

#[test]
fn test_acos_one() {
    assert_near(castom_acos(1.0), 0.0, EPS);
}

#[test]
fn test_acos_neg_one() {
    assert_near(castom_acos(-1.0), S21_M_PI, EPS);
}

// ===== atan =====
#[test]
fn test_atan_zero() {
    assert_eq!(castom_atan(0.0), 0.0);
}

#[test]
fn test_atan_one() {
    assert_near(castom_atan(1.0), 0.78539816339643858802, EPS);
}

#[test]
fn test_atan_neg_one() {
    assert_near(castom_atan(-1.0), -0.78539816339643858802, EPS);
}

#[test]
fn test_atan_half() {
    assert_near(castom_atan(0.5), 0.46364760900034500323, EPS);
}

#[test]
fn test_atan_large() {
    assert_near(castom_atan(100.0), 1.56079666010543871677, EPS);
}

// ===== pow =====
#[test]
fn test_pow_basic() {
    assert_eq!(castom_pow(2.0, 3.0), 8.0);
}

#[test]
fn test_pow_zero_exp() {
    assert_eq!(castom_pow(3.0, 0.0), 1.0);
}

#[test]
fn test_pow_zero_base() {
    assert_eq!(castom_pow(0.0, 5.0), 0.0);
}

#[test]
fn test_pow_fractional_exp() {
    assert_near(castom_pow(2.0, 0.5), 1.41421356237287136745, EPS);
}

#[test]
fn test_pow_mixed() {
    assert_near(castom_pow(5.1, 4.51), 1552.89286383219678011613, EPS);
}

#[test]
fn test_pow_negative_exp() {
    assert_near(castom_pow(1.5, -100.0), 0.0, EPS);
}

#[test]
fn test_pow_negative_base_even() {
    assert_near(castom_pow(-1.5, 8.0), 25.62890625, EPS);
}

#[test]
fn test_pow_negative_base_squared() {
    assert_eq!(castom_pow(-100.0, 2.0), 10000.0);
}

#[test]
fn test_pow_zero_zero() {
    assert_eq!(castom_pow(0.0, 0.0), 1.0);
}

#[test]
fn test_pow_negative_base_odd() {
    assert_eq!(castom_pow(-2.0, 3.0), -8.0);
}

#[test]
fn test_pow_negative_base_frac_exp() {
    // C returns -NAN for negative base with fractional exponent
    assert!(castom_pow(-2.0, 1.5).is_nan());
}

// ===== fmod =====
#[test]
fn test_fmod_basic() {
    assert_near(castom_fmod(10.5, -3.0), 1.5, EPS);
}

#[test]
fn test_fmod_negative_x() {
    assert_near(castom_fmod(-8.1, 4.0), -0.09999999999999964473, EPS);
}

#[test]
fn test_fmod_zero() {
    assert_eq!(castom_fmod(0.0, 1.0), 0.0);
}

#[test]
fn test_fmod_small_y() {
    assert_near(castom_fmod(1.5, 100.0), 1.5, EPS);
}

// ===== factorial =====
#[test]
fn test_factorial_zero() {
    assert_eq!(castom_factorial(0), 1);
}

#[test]
fn test_factorial_one() {
    assert_eq!(castom_factorial(1), 1);
}

#[test]
fn test_factorial_five() {
    assert_eq!(castom_factorial(5), 120);
}

#[test]
fn test_factorial_ten() {
    assert_eq!(castom_factorial(10), 3628800);
}

#[test]
fn test_factorial_twenty() {
    assert_eq!(castom_factorial(20), 2432902008176640000);
}

fn main() {}
