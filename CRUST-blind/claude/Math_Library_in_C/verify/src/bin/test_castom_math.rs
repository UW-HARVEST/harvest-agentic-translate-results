use Math_Library_in_C::castom_math::*;

const TOL: f64 = 1e-6;

fn approx_eq(a: f64, b: f64, tol: f64) -> bool {
    if a.is_nan() && b.is_nan() {
        return true;
    }
    if a.is_infinite() && b.is_infinite() {
        return a.is_sign_positive() == b.is_sign_positive();
    }
    (a - b).abs() <= tol
}

#[test]
fn test_constants() {
    assert!((S21_M_PI - 3.14159265358979323846).abs() < 1e-15);
    assert!((S21_M_E - 2.71828182845904523536).abs() < 1e-15);
    assert_eq!(EPS_10, 1e-10);
    assert_eq!(EPS_6, 1e-06);
    assert!(S21_NAN.is_nan());
    assert!(S21_INFINITY.is_infinite());
    assert!(S21_INFINITY > 0.0);
}

#[test]
fn test_castom_abs() {
    // From C ground truth
    assert_eq!(castom_abs(-1000), 1000);
    assert_eq!(castom_abs(-500), 500);
    assert_eq!(castom_abs(-1), 1);
    assert_eq!(castom_abs(0), 0);
    assert_eq!(castom_abs(1), 1);
    assert_eq!(castom_abs(500), 500);
    assert_eq!(castom_abs(1000), 1000);
    // Sweep
    for i in -1000..=1000 {
        let expected = if i > 0 { i } else { -i };
        assert_eq!(castom_abs(i), expected);
    }
}

#[test]
fn test_castom_fabs() {
    // From C ground truth
    assert!(approx_eq(castom_fabs(15.5), 15.5, TOL));
    assert!(approx_eq(castom_fabs(-15.3), 15.3, TOL));
    assert!(approx_eq(castom_fabs(0.000004), 4e-06, TOL));
    assert!(approx_eq(castom_fabs(-0.000004), 4e-06, TOL));
    assert!(approx_eq(castom_fabs(-1000.0), 1000.0, TOL));
    assert!(approx_eq(castom_fabs(1000.0), 1000.0, TOL));
    // Note: C castom_fabs(0.0) returns -0 because 0.0 > 0.0 is false, so it returns -(0.0) = -0
    // We'll just verify it's zero (positive or negative)
    let v = castom_fabs(0.0);
    assert!(v == 0.0);

    // sweep
    let mut i = -1000.0_f64;
    while i <= 1000.0 {
        let expected = i.abs();
        assert!(approx_eq(castom_fabs(i), expected, TOL),
                "fabs({}) failed", i);
        i += 0.5;
    }
}

#[test]
fn test_castom_floor() {
    // From C ground truth
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
    assert_eq!(castom_floor(-1000000.0), -1000000.0);
    assert_eq!(castom_floor(-0.0000001), -1.0);
    assert_eq!(castom_floor(-1.5), -2.0);
    // -0.0: C returns 0 (because (long long)(-0.0) is 0, -(truncation > x) = -(0.0 > -0.0) = -(false) = 0)
    assert_eq!(castom_floor(-0.0), 0.0);
    // NaN
    assert!(castom_floor(f64::NAN).is_nan());
}

#[test]
fn test_castom_ceil() {
    // From C ground truth
    assert_eq!(castom_ceil(1.000001), 2.0);
    assert_eq!(castom_ceil(8.44), 9.0);
    assert_eq!(castom_ceil(0.1), 1.0);
    assert_eq!(castom_ceil(500.55), 501.0);
    assert_eq!(castom_ceil(1000000.0), 1000000.0);
    assert_eq!(castom_ceil(0.0000001), 1.0);
    assert_eq!(castom_ceil(1.5), 2.0);
    assert_eq!(castom_ceil(0.0), 0.0);
    assert_eq!(castom_ceil(-1.000001), -1.0);
    assert_eq!(castom_ceil(-8.44), -8.0);
    assert_eq!(castom_ceil(-0.1), 0.0);
    assert_eq!(castom_ceil(-500.55), -500.0);
    assert_eq!(castom_ceil(-1000000.0), -1000000.0);
    assert_eq!(castom_ceil(-0.0000001), 0.0);
    assert_eq!(castom_ceil(-1.5), -1.0);
    assert_eq!(castom_ceil(-0.0), 0.0);
    // NaN
    assert!(castom_ceil(f64::NAN).is_nan());
}

#[test]
fn test_castom_trunc() {
    // From C ground truth
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
    assert_eq!(castom_trunc(-1000000.0), -1000000.0);
    assert_eq!(castom_trunc(-0.0000001), 0.0);
    assert_eq!(castom_trunc(-1.5), -1.0);
    assert_eq!(castom_trunc(-0.0), 0.0);
}

#[test]
fn test_castom_sqrt() {
    // From C ground truth
    assert!(approx_eq(castom_sqrt(1.0), 1.0, TOL));
    assert!(approx_eq(castom_sqrt(8.0), 2.8284271247461898, TOL));
    assert!(approx_eq(castom_sqrt(0.1), 0.31622776601683794, TOL));
    assert!(approx_eq(castom_sqrt(500.0), 22.360679774997898, TOL));
    assert!(approx_eq(castom_sqrt(1000000.0), 1000.0, TOL));
    assert!(approx_eq(castom_sqrt(0.0000001), 0.00031622776601683794, TOL));
    assert!(approx_eq(castom_sqrt(1.5), 1.2247448713915889, TOL));
    assert!(approx_eq(castom_sqrt(2.0), 1.4142135623730949, TOL));
    assert_eq!(castom_sqrt(0.0), 0.0);
    assert!(approx_eq(castom_sqrt(4.0), 2.0, TOL));
    assert!(approx_eq(castom_sqrt(9.0), 3.0, TOL));
    assert!(approx_eq(castom_sqrt(16.0), 4.0, TOL));
    assert!(approx_eq(castom_sqrt(25.0), 5.0, TOL));
    assert!(castom_sqrt(-2.0).is_nan());
    assert!(castom_sqrt(-1.0).is_nan());
    assert!(castom_sqrt(f64::NAN).is_nan());
}

#[test]
fn test_castom_exp() {
    // From C ground truth
    assert!(approx_eq(castom_exp(0.0), 1.0, TOL));
    assert!(approx_eq(castom_exp(1.0), 2.718281828458229, TOL));
    assert!(approx_eq(castom_exp(-1.0), 0.36787944117216190, TOL));
    assert!(approx_eq(castom_exp(5.0), 148.41315910257242, TOL));
    assert!(approx_eq(castom_exp(-5.0), 0.0067379469960638930, TOL));
    assert!(approx_eq(castom_exp(0.0000001), 1.000000100000005, TOL));
    assert!(approx_eq(castom_exp(2.0), 7.3890560989258641, TOL));
    assert!(approx_eq(castom_exp(-10.0), 4.5399936624619471e-05, TOL));
    // exp(19) ~ 178482300.96, so larger absolute tol
    assert!(approx_eq(castom_exp(19.0), 178482300.9631872609, 1e-2));
}

#[test]
fn test_castom_log() {
    // From C ground truth
    assert!(approx_eq(castom_log(1.0), 0.0, TOL));
    assert!(approx_eq(castom_log(8.0), 2.0794415416798361917, TOL));
    assert!(approx_eq(castom_log(0.1), -2.3025850929940459, TOL));
    assert!(approx_eq(castom_log(500.0), 6.214608098422192, TOL));
    assert!(approx_eq(castom_log(1000000.0), 13.815510557964275, TOL));
    assert!(approx_eq(castom_log(0.0000001), -16.118095650958320, TOL));
    assert!(approx_eq(castom_log(1.5), 0.40546510810816444, TOL));
    assert!(approx_eq(castom_log(2.0), 0.69314718055994506, TOL));
    assert!(approx_eq(castom_log(S21_M_E), 1.0, TOL));

    // Edge cases
    assert!(castom_log(-1.0).is_nan());
    assert!(castom_log(-0.5).is_nan());
    assert!(castom_log(-1000.0).is_nan());
    let zero_log = castom_log(0.0);
    assert!(zero_log.is_infinite());
    assert!(zero_log < 0.0);
}

#[test]
fn test_castom_fmod() {
    // From C ground truth
    assert!(approx_eq(castom_fmod(10.5, -3.0), 1.5, TOL));
    assert!(approx_eq(castom_fmod(-8.1, 4.0), -0.099999999999999644, TOL));
    assert_eq!(castom_fmod(0.0, 1.4), 0.0);
    assert!(approx_eq(castom_fmod(10.1, 0.051), 0.0020000000000002932, TOL));
    assert!(approx_eq(castom_fmod(100.1, 0.1), 0.099999999999988764, TOL));
    assert!(approx_eq(castom_fmod(1.5, 100.0), 1.5, TOL));
    assert!(approx_eq(castom_fmod(0.0, 1.0), 0.0, TOL));
}

#[test]
fn test_castom_pow() {
    // From C ground truth
    assert!(approx_eq(castom_pow(1.0, 1.0), 1.0, TOL));
    assert!(approx_eq(castom_pow(3.0, 0.0), 1.0, TOL));
    assert!(approx_eq(castom_pow(3.0, 2.0), 9.0, TOL));
    assert!(approx_eq(castom_pow(3.1, 4.0), 92.3521, TOL));
    assert!(approx_eq(castom_pow(3.1, 4.2), 115.80281433591698, TOL));
    assert!(approx_eq(castom_pow(5.1, 0.511), 2.2991555786415872, TOL));
    assert_eq!(castom_pow(0.0, 0.511), 0.0);
    assert!(approx_eq(castom_pow(1.5, -100.0), 2.4596544265798156e-18, 1e-23));
    assert!(approx_eq(castom_pow(-1.5, 8.0), 25.62890625, TOL));
    assert!(approx_eq(castom_pow(-100.0, 2.0), 10000.0, TOL));
    assert_eq!(castom_pow(0.0, 0.0), 1.0);
    assert!(approx_eq(castom_pow(2.0, 10.0), 1024.0, TOL));
    // base < 0 with non-integer exp -> NaN
    assert!(castom_pow(-2.0, 0.5).is_nan());
}

#[test]
fn test_castom_sin() {
    // From C ground truth
    assert!(approx_eq(castom_sin(0.0), 0.0, TOL));
    assert!(approx_eq(castom_sin(S21_M_PI / 6.0), 0.5, TOL));
    assert!(approx_eq(castom_sin(S21_M_PI / 4.0), 0.7071067811865475, TOL));
    assert!(approx_eq(castom_sin(S21_M_PI / 3.0), 0.8660254037844387, TOL));
    assert!(approx_eq(castom_sin(S21_M_PI / 2.0), 1.0, TOL));
    assert!(approx_eq(castom_sin(S21_M_PI), 0.0, TOL));
    assert!(approx_eq(castom_sin(2.0 * S21_M_PI), 0.0, TOL));
    assert!(approx_eq(castom_sin(1.0), 0.84147098480789651, TOL));
    assert!(approx_eq(castom_sin(-1.0), -0.84147098480789651, TOL));
    assert!(approx_eq(castom_sin(2.5), 0.59847214410395668, TOL));

    // sweep
    let mut j = -10.0_f64;
    while j <= 10.0 {
        let expected = j.sin();
        let got = castom_sin(j);
        assert!(approx_eq(got, expected, TOL),
                "sin({}) = {}, expected {}", j, got, expected);
        j += 0.1;
    }
}

#[test]
fn test_castom_cos() {
    // From C ground truth
    assert!(approx_eq(castom_cos(0.0), 1.0, TOL));
    assert!(approx_eq(castom_cos(S21_M_PI / 4.0), 0.7071067811869, TOL));
    assert!(approx_eq(castom_cos(S21_M_PI / 3.0), 0.5, TOL));
    assert!(approx_eq(castom_cos(S21_M_PI / 2.0), 0.0, TOL));
    assert!(approx_eq(castom_cos(S21_M_PI), -1.0, TOL));
    assert!(approx_eq(castom_cos(2.0 * S21_M_PI), 1.0, TOL));
    assert!(approx_eq(castom_cos(1.0), 0.5403023058795628, TOL));
    assert!(approx_eq(castom_cos(-1.0), 0.5403023058795628, TOL));
    assert!(approx_eq(castom_cos(30.0), 0.15425144983159268, TOL));
    assert!(approx_eq(castom_cos(45.0), 0.52532198883233, TOL));

    // sweep
    let mut j = -10.0_f64;
    while j <= 10.0 {
        let expected = j.cos();
        let got = castom_cos(j);
        assert!(approx_eq(got, expected, TOL),
                "cos({}) = {}, expected {}", j, got, expected);
        j += 0.1;
    }
}

#[test]
fn test_castom_tan() {
    // From C ground truth
    assert_eq!(castom_tan(0.0), 0.0);
    // M_PI/6 = exact match in special case
    assert!(approx_eq(castom_tan(S21_M_PI / 6.0), 1.0 / castom_sqrt(3.0), TOL));
    assert_eq!(castom_tan(S21_M_PI / 4.0), 1.0);
    assert!(approx_eq(castom_tan(S21_M_PI / 3.0), castom_sqrt(3.0), TOL));
    // Special case: M_PI returns 0.0 directly
    assert_eq!(castom_tan(S21_M_PI), 0.0);
    assert_eq!(castom_tan(S21_M_PI / 2.0), S21_INFINITY);
    assert_eq!(castom_tan(2.0 * S21_M_PI), 0.0);
    assert_eq!(castom_tan(3.0 * S21_M_PI / 2.0), S21_INFINITY);

    assert!(approx_eq(castom_tan(1.0), 1.5574077246219754, TOL));
    assert!(approx_eq(castom_tan(0.5), 0.5463024898441073, TOL));
    assert!(approx_eq(castom_tan(0.0001), 0.00010000000033333333, TOL));

    // Sweep
    let mut j = -3.0_f64;
    while j < 1.5 {
        // Avoid points near pi/2 where tan blows up
        if (j - 1.5).abs() > 0.1 && (j + 1.5).abs() > 0.1 {
            let expected = j.tan();
            let got = castom_tan(j);
            assert!(approx_eq(got, expected, 1e-3),
                    "tan({}) = {}, expected {}", j, got, expected);
        }
        j += 0.1;
    }
}

#[test]
fn test_castom_asin() {
    // From C ground truth
    assert_eq!(castom_asin(0.0), 0.0);
    assert!(approx_eq(castom_asin(0.5), 0.5235987755976584, TOL));
    assert!(approx_eq(castom_asin(-0.5), -0.5235987755976584, TOL));
    assert!(approx_eq(castom_asin(1.0), S21_M_PI / 2.0, TOL));
    assert!(approx_eq(castom_asin(-1.0), -S21_M_PI / 2.0, TOL));
    assert!(approx_eq(castom_asin(0.3), 0.3046926540153022, TOL));
    assert!(approx_eq(castom_asin(-0.99), -1.4292568534680987, TOL));
    assert!(approx_eq(castom_asin(0.000001), 1.0000000000001666e-06, TOL));

    // Sweep
    let mut j = -0.99_f64;
    while j <= 0.99 {
        let expected = j.asin();
        let got = castom_asin(j);
        assert!(approx_eq(got, expected, TOL),
                "asin({}) = {}, expected {}", j, got, expected);
        j += 0.05;
    }
}

#[test]
fn test_castom_acos() {
    // From C ground truth
    assert!(approx_eq(castom_acos(0.0), S21_M_PI / 2.0, TOL));
    assert!(approx_eq(castom_acos(0.5), 1.0471975511972381, TOL));
    assert!(approx_eq(castom_acos(-0.5), 2.094395102392554994, TOL));
    assert!(approx_eq(castom_acos(1.0), 0.0, TOL));
    assert!(approx_eq(castom_acos(-1.0), S21_M_PI, TOL));
    assert!(approx_eq(castom_acos(0.3), 1.2661036727795943, TOL));
    assert!(approx_eq(castom_acos(0.000001), 1.5707953267948965, TOL));

    // Sweep
    let mut j = -0.99_f64;
    while j <= 0.99 {
        let expected = j.acos();
        let got = castom_acos(j);
        assert!(approx_eq(got, expected, TOL),
                "acos({}) = {}, expected {}", j, got, expected);
        j += 0.05;
    }
}

#[test]
fn test_castom_atan() {
    // From C ground truth
    assert!(approx_eq(castom_atan(0.0), 0.0, TOL));
    assert!(approx_eq(castom_atan(1.0), 0.7853981633964386, TOL));
    assert!(approx_eq(castom_atan(-1.0), -0.7853981633964386, TOL));
    assert!(approx_eq(castom_atan(2.1), 1.1263771168920829, TOL));
    assert!(approx_eq(castom_atan(-50.0), -1.5507989928190461, TOL));
    assert!(approx_eq(castom_atan(0.5), 0.46364760900034500, TOL));
    assert!(approx_eq(castom_atan(100.0), 1.5607966601054387, TOL));

    // Sweep
    let mut j = -10.0_f64;
    while j < 10.0 {
        let expected = j.atan();
        let got = castom_atan(j);
        assert!(approx_eq(got, expected, TOL),
                "atan({}) = {}, expected {}", j, got, expected);
        j += 0.5;
    }
}

#[test]
fn test_castom_factorial() {
    // factorial is declared in header; the Rust impl computes it directly via wrapping_mul.
    // Compute expected values via a simple reference.
    assert_eq!(castom_factorial(0), 1);
    assert_eq!(castom_factorial(1), 1);
    assert_eq!(castom_factorial(2), 2);
    assert_eq!(castom_factorial(3), 6);
    assert_eq!(castom_factorial(4), 24);
    assert_eq!(castom_factorial(5), 120);
    assert_eq!(castom_factorial(6), 720);
    assert_eq!(castom_factorial(10), 3628800);
    assert_eq!(castom_factorial(12), 479001600);
    assert_eq!(castom_factorial(15), 1307674368000);
    assert_eq!(castom_factorial(20), 2432902008176640000);
}

fn main() {}
