// Constants
pub const S21_NAN: f64 = f64::NAN;
pub const EPS_6: f64 = 1e-06;
pub const S21_M_E: f64 = 2.71828182845904523536028747135266250;
pub const S21_INFINITY: f64 = f64::INFINITY;
pub const EPS_10: f64 = 1e-10;
pub const S21_M_PI: f64 = 3.14159265358979323846264338327950288;
// Function Declarations
pub fn castom_floor(x: f64) -> f64 {
    if x >= i64::MAX as f64 || x <= i64::MIN as f64 || x.is_nan() {
        return x;
    }
    let t = x as i64 as f64;
    if t > x { t - 1.0 } else { t }
}
pub fn castom_tan(x: f64) -> f64 {
    castom_sin(x) / castom_cos(x)
}
pub fn castom_atan(x: f64) -> f64 {
    castom_asin(x / castom_sqrt(1.0 + x * x))
}
pub fn castom_ceil(x: f64) -> f64 {
    if x >= i64::MAX as f64 || x <= i64::MIN as f64 || x.is_nan() {
        return x;
    }
    let t = x as i64 as f64;
    if t < x { t + 1.0 } else { t }
}
pub fn castom_sqrt(x: f64) -> f64 {
    if x.is_nan() || x < 0.0 {
        return S21_NAN;
    }
    let mut sqrt = x / 2.0;
    let mut temp = 0.0_f64;
    while sqrt != temp {
        temp = sqrt;
        sqrt = (x / temp + temp) / 2.0;
    }
    sqrt
}
pub fn castom_trunc(x: f64) -> f64 {
    if x > 0.0 { castom_floor(x) } else { castom_ceil(x) }
}
pub fn castom_exp(x: f64) -> f64 {
    x.exp()
}
pub fn castom_factorial(n: u64) -> u64 {
    (1..=n).product()
}
pub fn castom_asin(x: f64) -> f64 {
    if x > 1.0 || x < -1.0 {
        return S21_NAN;
    }
    if x == 1.0 {
        return S21_M_PI / 2.0;
    }
    if x == -1.0 {
        return -S21_M_PI / 2.0;
    }
    let mut term = x;
    let mut sum = term;
    let x2 = x * x;
    let mut k = 1;
    while castom_fabs(term) > EPS_10 {
        term *= x2 * k as f64 / (k + 1) as f64;
        sum += term / (k + 2) as f64;
        k += 2;
    }
    sum
}
pub fn castom_fmod(x: f64, y: f64) -> f64 {
    x - castom_trunc(x / y) * y
}
pub fn castom_acos(x: f64) -> f64 {
    S21_M_PI / 2.0 - castom_asin(x)
}
pub fn castom_pow(base: f64, exp: f64) -> f64 {
    base.powf(exp)
}
pub fn castom_abs(x: i32) -> i32 {
    if x > 0 { x } else { -x }
}
pub fn castom_log(x: f64) -> f64 {
    if x <= 0.0 {
        return if x == 0.0 { f64::NEG_INFINITY } else { S21_NAN };
    }
    let mut a = 0u32;
    let mut c = if x < 1.0 { 1.0 / x } else { x };
    while { c /= S21_M_E; c > 1.0 } {
        a += 1;
    }
    c = 1.0 / (c * S21_M_E - 1.0);
    c = c + c + 1.0;
    let f = c * c;
    c /= 2.0;
    let mut b = 0.0_f64;
    let mut d = 1u32;
    loop {
        let e = b;
        b += 1.0 / (d as f64 * c);
        if b == e {
            break;
        }
        d += 2;
        c *= f;
    }
    if x < 1.0 { -(a as f64 + b) } else { a as f64 + b }
}
pub fn castom_sin(x: f64) -> f64 {
    let x = castom_fmod(x, 2.0 * S21_M_PI);
    let mut sum = 0.0_f64;
    for i in 0..=20 {
        let mut fa = 1.0_f64;
        let mut pow = 1.0_f64;
        for j in 1..=(2 * i + 1) {
            fa *= j as f64;
            pow *= x;
        }
        sum += (if i % 2 != 0 { -1.0 } else { 1.0 } / fa) * pow;
    }
    sum
}
pub fn castom_cos(x: f64) -> f64 {
    let x = castom_fmod(x, 2.0 * S21_M_PI);
    let mut t_s = 0.0_f64;
    let mut last = 1.0_f64;
    let mut k = 1;
    while castom_fabs(last) > EPS_10 {
        t_s += last;
        last *= -x * x / ((2.0 * k as f64 - 1.0) * (2.0 * k as f64));
        k += 1;
    }
    t_s
}
pub fn castom_fabs(x: f64) -> f64 {
    if x > 0.0 { x } else { -x }
}
