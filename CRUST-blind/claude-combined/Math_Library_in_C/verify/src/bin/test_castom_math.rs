use Math_Library_in_C::castom_math::*;

fn approx_eq(a: f64, b: f64, eps: f64) -> bool {
    (a - b).abs() <= eps
}

const EPS: f64 = 1e-6;

#[test]
fn test_constants() {
    assert!(S21_NAN.is_nan());
    assert_eq!(EPS_6, 1e-06);
    assert_eq!(EPS_10, 1e-10);
    assert_eq!(S21_INFINITY, f64::INFINITY);
    // S21_M_PI and S21_M_E
    assert!((S21_M_PI - 3.14159265358979323846264338327950288).abs() < 1e-15);
    assert!((S21_M_E - 2.71828182845904523536028747135266250).abs() < 1e-15);
}

#[test]
fn test_abs() {
    assert_eq!(castom_abs(0), 0);
    assert_eq!(castom_abs(5), 5);
    assert_eq!(castom_abs(-5), 5);
    assert_eq!(castom_abs(-1000), 1000);
    assert_eq!(castom_abs(1000), 1000);
    assert_eq!(castom_abs(1), 1);
    assert_eq!(castom_abs(-1), 1);
    for i in -1000i32..=1000i32 {
        let expected = i.abs();
        assert_eq!(castom_abs(i), expected, "abs({}) failed", i);
    }
}

#[test]
fn test_fabs() {
    assert!(approx_eq(castom_fabs(15.5), 15.5, EPS));
    assert!(approx_eq(castom_fabs(-15.3), 15.3, EPS));
    // Note: C's castom_fabs(0.0) returns -0.0 (because x > 0.0 is false when x == 0)
    // both -0.0 and 0.0 compare equal numerically
    assert_eq!(castom_fabs(0.0), 0.0);
    assert!(approx_eq(castom_fabs(0.000004), 0.000004, EPS));
    assert!(approx_eq(castom_fabs(-0.000004), 0.000004, EPS));
    assert!(approx_eq(castom_fabs(-1.0), 1.0, EPS));
    assert!(approx_eq(castom_fabs(123.456), 123.456, EPS));
    assert!(approx_eq(castom_fabs(-123.456), 123.456, EPS));
}

#[test]
fn test_floor() {
    assert_eq!(castom_floor(1.000001), 1.0);
    assert_eq!(castom_floor(8.44), 8.0);
    assert_eq!(castom_floor(0.1), 0.0);
    assert_eq!(castom_floor(500.55), 500.0);
    assert_eq!(castom_floor(1000000.0), 1000000.0);
    assert_eq!(castom_floor(0.0000001), 0.0);
    assert_eq!(castom_floor(1.5), 1.0);
    assert_eq!(castom_floor(0.0), 0.0);
    assert_eq!(castom_floor(-1.000001), -2.0);
    assert_eq!(castom_floor(-8.44), -9.0);
    assert_eq!(castom_floor(-0.1), -1.0);
    assert_eq!(castom_floor(-500.55), -501.0);
    assert_eq!(castom_floor(-1.5), -2.0);
    assert_eq!(castom_floor(2.0), 2.0);
    assert_eq!(castom_floor(-2.0), -2.0);
    assert!(castom_floor(f64::NAN).is_nan());
}

#[test]
fn test_ceil() {
    assert_eq!(castom_ceil(1.000001), 2.0);
    assert_eq!(castom_ceil(8.44), 9.0);
    assert_eq!(castom_ceil(0.1), 1.0);
    assert_eq!(castom_ceil(500.55), 501.0);
    assert_eq!(castom_ceil(1000000.0), 1000000.0);
    assert_eq!(castom_ceil(0.0000001), 1.0);
    assert_eq!(castom_ceil(1.5), 2.0);
    assert_eq!(castom_ceil(0.0), 0.0);
    assert_eq!(castom_ceil(-1.5), -1.0);
    assert_eq!(castom_ceil(-8.44), -8.0);
    assert_eq!(castom_ceil(2.0), 2.0);
    assert_eq!(castom_ceil(-2.0), -2.0);
    assert!(castom_ceil(f64::NAN).is_nan());
}

#[test]
fn test_trunc() {
    assert_eq!(castom_trunc(1.000001), 1.0);
    assert_eq!(castom_trunc(8.44), 8.0);
    assert_eq!(castom_trunc(0.1), 0.0);
    assert_eq!(castom_trunc(500.55), 500.0);
    assert_eq!(castom_trunc(1000000.0), 1000000.0);
    assert_eq!(castom_trunc(0.0000001), 0.0);
    assert_eq!(castom_trunc(1.5), 1.0);
    assert_eq!(castom_trunc(0.0), 0.0);
    assert_eq!(castom_trunc(-1.000001), -1.0);
    assert_eq!(castom_trunc(-8.44), -8.0);
    assert_eq!(castom_trunc(-0.1), 0.0);
    assert_eq!(castom_trunc(-500.55), -500.0);
    assert_eq!(castom_trunc(-1.5), -1.0);
    assert_eq!(castom_trunc(-1000000.0), -1000000.0);
}

#[test]
fn test_sqrt() {
    assert!(approx_eq(castom_sqrt(1.0), 1.0, EPS));
    assert!(approx_eq(castom_sqrt(8.0), 8.0_f64.sqrt(), EPS));
    assert!(approx_eq(castom_sqrt(0.1), 0.31622776601683794118, EPS));
    assert!(approx_eq(castom_sqrt(500.0), 500.0_f64.sqrt(), EPS));
    assert!(approx_eq(castom_sqrt(1000000.0), 1000.0, EPS));
    assert!(approx_eq(castom_sqrt(0.0000001), 0.0000001_f64.sqrt(), EPS));
    assert!(approx_eq(castom_sqrt(1.5), 1.5_f64.sqrt(), EPS));
    assert!(approx_eq(castom_sqrt(2.0), 1.4142135623730949234, EPS));
    // C returns -nan for sqrt(-2). is_nan() catches both +nan and -nan.
    assert!(castom_sqrt(-2.0).is_nan());
    assert!(castom_sqrt(f64::NAN).is_nan());
    assert_eq!(castom_sqrt(0.0), 0.0);
}

#[test]
fn test_exp() {
    assert!(approx_eq(castom_exp(0.0), 1.0, EPS));
    assert!(approx_eq(castom_exp(1.0), 2.718281828458229748, EPS));
    assert!(approx_eq(castom_exp(-5.0), 0.0067379469960638930036, EPS));
    assert!(approx_eq(castom_exp(5.0), 148.41315910257242185, EPS));
    assert!(approx_eq(castom_exp(-15.0), (-15.0_f64).exp(), EPS));
    assert!(approx_eq(castom_exp(0.0000001), 0.0000001_f64.exp(), EPS));
    assert!(approx_eq(castom_exp(-20.0), (-20.0_f64).exp(), EPS));
    assert!(approx_eq(castom_exp(19.0), (19.0_f64).exp(), EPS));
}

#[test]
fn test_log() {
    assert!(approx_eq(castom_log(1.0), 0.0, EPS));
    assert!(approx_eq(castom_log(0.1), -2.3025850929940459011, EPS));
    assert!(approx_eq(castom_log(2.0), 0.69314718055994506418, EPS));
    assert!(approx_eq(castom_log(8.0), 2.0794415416798361917, EPS));
    assert!(approx_eq(castom_log(500.0), (500.0_f64).ln(), EPS));
    assert!(approx_eq(castom_log(1000000.0), (1000000.0_f64).ln(), EPS));
    assert!(approx_eq(castom_log(0.0000001), (0.0000001_f64).ln(), EPS));
    assert!(approx_eq(castom_log(1.5), (1.5_f64).ln(), EPS));
    // C log(0) -> -inf
    assert!(castom_log(0.0).is_infinite() && castom_log(0.0) < 0.0);
    // C log(<0) -> NaN
    assert!(castom_log(-1.0).is_nan());
    assert!(castom_log(-1000.0).is_nan());
}

#[test]
fn test_sin() {
    assert!(approx_eq(castom_sin(0.0), 0.0, EPS));
    assert!(approx_eq(castom_sin(1.0), 0.84147098480789651572, EPS));
    assert!(approx_eq(castom_sin(S21_M_PI / 6.0), 0.5, EPS));
    assert!(approx_eq(castom_sin(S21_M_PI / 4.0), (S21_M_PI / 4.0).sin(), EPS));
    assert!(approx_eq(castom_sin(S21_M_PI / 3.0), (S21_M_PI / 3.0).sin(), EPS));
    assert!(approx_eq(castom_sin(S21_M_PI / 2.0), 1.0, EPS));
    assert!(approx_eq(castom_sin(S21_M_PI), 0.0, EPS));
    assert!(approx_eq(castom_sin(2.0 * S21_M_PI), 0.0, EPS));
    // sin in a range
    let mut j: f64 = -10.0;
    while j <= 10.0 {
        assert!(approx_eq(castom_sin(j), j.sin(), EPS), "sin({}) failed: got {}, expected {}", j, castom_sin(j), j.sin());
        j += 0.5;
    }
}

#[test]
fn test_cos() {
    assert!(approx_eq(castom_cos(0.0), 1.0, EPS));
    assert!(approx_eq(castom_cos(1.0), 0.54030230587956282179, EPS));
    assert!(approx_eq(castom_cos(S21_M_PI / 4.0), (S21_M_PI / 4.0).cos(), EPS));
    assert!(approx_eq(castom_cos(S21_M_PI), -1.0, EPS));
    assert!(approx_eq(castom_cos(0.000001), (0.000001_f64).cos(), EPS));
    let mut j: f64 = -10.0;
    while j <= 10.0 {
        assert!(approx_eq(castom_cos(j), j.cos(), EPS), "cos({}) failed", j);
        j += 0.5;
    }
}

#[test]
fn test_tan() {
    assert!(approx_eq(castom_tan(0.0), 0.0, EPS));
    assert!(approx_eq(castom_tan(1.0), 1.5574077246219754379, EPS));
    // Special branches in tan
    assert!(approx_eq(castom_tan(S21_M_PI / 6.0), 1.0 / castom_sqrt(3.0), EPS));
    assert_eq!(castom_tan(S21_M_PI / 4.0), 1.0);
    assert_eq!(castom_tan(S21_M_PI / 3.0), castom_sqrt(3.0));
    assert_eq!(castom_tan(S21_M_PI / 2.0), S21_INFINITY);
    assert_eq!(castom_tan(S21_M_PI), 0.0);
    assert_eq!(castom_tan(3.0 * S21_M_PI / 2.0), S21_INFINITY);
    assert_eq!(castom_tan(2.0 * S21_M_PI), 0.0);
    assert!(approx_eq(castom_tan(0.0001), (0.0001_f64).tan(), EPS));
    assert!(approx_eq(castom_tan(-5.0), (-5.0_f64).tan(), EPS));
}

#[test]
fn test_asin() {
    assert!(approx_eq(castom_asin(0.0), 0.0, EPS));
    assert!(approx_eq(castom_asin(0.5), 0.52359877559765843609, EPS));
    assert!(approx_eq(castom_asin(1.0), 1.570796326794896558, EPS));
    assert!(approx_eq(castom_asin(-1.0), -1.570796326794896558, EPS));
    assert!(approx_eq(castom_asin(0.3), (0.3_f64).asin(), EPS));
    assert!(approx_eq(castom_asin(0.45), (0.45_f64).asin(), EPS));
    assert!(approx_eq(castom_asin(-0.18), (-0.18_f64).asin(), EPS));
    assert!(approx_eq(castom_asin(-0.99), (-0.99_f64).asin(), EPS));
    assert!(approx_eq(castom_asin(0.000001), (0.000001_f64).asin(), EPS));
    // Outside [-1, 1] produces NaN per C implementation
    assert!(castom_asin(2.0).is_nan());
    assert!(castom_asin(-2.0).is_nan());
}

#[test]
fn test_acos() {
    assert!(approx_eq(castom_acos(0.0), 1.570796326794896558, EPS));
    assert!(approx_eq(castom_acos(0.5), 1.0471975511972381219, EPS));
    assert!(approx_eq(castom_acos(0.3), (0.3_f64).acos(), EPS));
    assert!(approx_eq(castom_acos(-0.99), (-0.99_f64).acos(), EPS));
    assert!(approx_eq(castom_acos(0.255), (0.255_f64).acos(), EPS));
    assert!(approx_eq(castom_acos(0.000001), (0.000001_f64).acos(), EPS));
    // acos(1) = 0
    assert!(approx_eq(castom_acos(1.0), 0.0, EPS));
    // acos(-1) = pi
    assert!(approx_eq(castom_acos(-1.0), S21_M_PI, EPS));
}

#[test]
fn test_atan() {
    assert!(approx_eq(castom_atan(0.0), 0.0, EPS));
    assert!(approx_eq(castom_atan(1.0), 0.78539816339643858802, EPS));
    assert!(approx_eq(castom_atan(-1.0), -0.78539816339643858802, EPS));
    assert!(approx_eq(castom_atan(-50.0), -1.5507989928190461855, EPS));
    assert!(approx_eq(castom_atan(2.1), (2.1_f64).atan(), EPS));
    assert!(approx_eq(castom_atan(0.3), (0.3_f64).atan(), EPS));
    assert!(approx_eq(castom_atan(0.45), (0.45_f64).atan(), EPS));
    assert!(approx_eq(castom_atan(-0.99), (-0.99_f64).atan(), EPS));
    assert!(approx_eq(castom_atan(0.000001), (0.000001_f64).atan(), EPS));
}

#[test]
fn test_fmod() {
    assert!(approx_eq(castom_fmod(10.5, -3.0), 1.5, EPS));
    assert!(approx_eq(castom_fmod(-8.1, 4.0), -0.099999999999999644729, EPS));
    assert!(approx_eq(castom_fmod(0.0, 1.0), 0.0, EPS));
    assert!(approx_eq(castom_fmod(0.0, 1.4), 0.0, EPS));
    assert!(approx_eq(castom_fmod(-0.0, 4.4), 0.0, EPS));
    assert!(approx_eq(castom_fmod(10.1, 0.051), (10.1_f64) % 0.051, EPS));
    assert!(approx_eq(castom_fmod(100.1, 0.1), (100.1_f64) % 0.1, EPS));
    assert!(approx_eq(castom_fmod(1.5, 100.0), 1.5, EPS));
}

#[test]
fn test_pow() {
    assert!(approx_eq(castom_pow(0.0, 0.0), 1.0, EPS));
    assert!(approx_eq(castom_pow(1.0, 1.0), 1.0, EPS));
    assert!(approx_eq(castom_pow(3.0, 0.0), 1.0, EPS));
    assert!(approx_eq(castom_pow(3.0, 2.0), 9.0, EPS));
    assert!(approx_eq(castom_pow(3.1, 4.0), 92.352100000000010585, EPS));
    assert!(approx_eq(castom_pow(-1.5, 8.0), (-1.5_f64).powf(8.0), EPS));
    assert!(approx_eq(castom_pow(-100.0, 2.0), 10000.0, EPS));
    assert!(approx_eq(castom_pow(0.0, 0.5), 0.0, EPS));
    assert!(approx_eq(castom_pow(0.0, 0.511), 0.0, EPS));
    assert!(approx_eq(castom_pow(1.5, -100.0), (1.5_f64).powf(-100.0), 1e-15));
    // Fractional exponent on negative base -> NaN
    assert!(castom_pow(-2.0, 0.5).is_nan());
    // Sign for odd integer exponent of negative base
    assert!(approx_eq(castom_pow(-2.0, 3.0), -8.0, EPS));
    assert!(approx_eq(castom_pow(-2.0, 4.0), 16.0, EPS));
}

#[test]
fn test_factorial() {
    // factorial isn't implemented in C source, only declared. Test the
    // standard mathematical definition that the Rust translation provides.
    assert_eq!(castom_factorial(0), 1);
    assert_eq!(castom_factorial(1), 1);
    assert_eq!(castom_factorial(2), 2);
    assert_eq!(castom_factorial(3), 6);
    assert_eq!(castom_factorial(4), 24);
    assert_eq!(castom_factorial(5), 120);
    assert_eq!(castom_factorial(10), 3628800);
    assert_eq!(castom_factorial(12), 479001600);
    assert_eq!(castom_factorial(20), 2432902008176640000);
}

fn main() {}
