// Constants
pub const S21_NAN: f64 = f64::NAN;
pub const EPS_6: f64 = 1e-06;
pub const S21_M_E: f64 = 2.71828182845904523536028747135266250;
pub const S21_INFINITY: f64 = f64::INFINITY;
pub const EPS_10: f64 = 1e-10;
pub const S21_M_PI: f64 = 3.14159265358979323846264338327950288;

// Internal C-equivalent constants for boundary checks
const LLONG_MAX_F: f64 = 9223372036854775807.0; // 2^63 - 1, but as f64 this rounds to 2^63
const LLONG_MIN_F: f64 = -9223372036854775808.0;

// Function Declarations
pub fn castom_floor(x: f64) -> f64 {
    if x >= LLONG_MAX_F || x <= LLONG_MIN_F || x.is_nan() {
        return x;
    }
    let truncation: f64 = (x as i64) as f64;
    truncation - if truncation > x { 1.0 } else { 0.0 }
}

pub fn castom_tan(x: f64) -> f64 {
    if x == 0.0 {
        0.0
    } else if S21_M_PI / 6.0 == x {
        1.0 / castom_sqrt(3.0)
    } else if S21_M_PI / 4.0 == x {
        1.0
    } else if S21_M_PI / 3.0 == x {
        castom_sqrt(3.0)
    } else if S21_M_PI / 2.0 == x {
        S21_INFINITY
    } else if S21_M_PI == x {
        0.0
    } else if 3.0 * S21_M_PI / 2.0 == x {
        S21_INFINITY
    } else if 2.0 * S21_M_PI == x {
        0.0
    } else {
        castom_sin(x) / castom_cos(x)
    }
}

pub fn castom_atan(x: f64) -> f64 {
    castom_asin(x / castom_sqrt(1.0 + x * x))
}

pub fn castom_ceil(x: f64) -> f64 {
    if x >= LLONG_MAX_F || x <= LLONG_MIN_F || x.is_nan() {
        return x;
    }
    let truncation: f64 = (x as i64) as f64;
    truncation + if truncation < x { 1.0 } else { 0.0 }
}

pub fn castom_sqrt(x: f64) -> f64 {
    if x.is_nan() || x < 0.0 {
        return S21_NAN;
    }
    if x == 0.0 {
        return 0.0;
    }
    let mut sqrt = x / 2.0;
    let mut temp: f64 = 0.0;
    while sqrt != temp {
        temp = sqrt;
        sqrt = (x / temp + temp) / 2.0;
    }
    sqrt
}

pub fn castom_trunc(x: f64) -> f64 {
    if x > 0.0 {
        castom_floor(x)
    } else {
        castom_ceil(x)
    }
}

pub fn castom_exp(x: f64) -> f64 {
    let mut sum: f64 = 1.0;
    let mut cur: f64 = 1.0;
    let mut n: u32 = 1;

    while castom_fabs(cur) > EPS_10 {
        cur *= x / (n as f64);
        sum += cur;
        n += 1;
        // Safety guard against runaway loops on pathological inputs (won't trigger
        // for the C-tested input range; mirrors the natural termination of the C loop)
        if n > 100_000 {
            break;
        }
    }

    sum
}

pub fn castom_factorial(n: u64) -> u64 {
    let mut result: u64 = 1;
    let mut i: u64 = 2;
    while i <= n {
        result = result.wrapping_mul(i);
        i += 1;
    }
    result
}

pub fn castom_asin(x: f64) -> f64 {
    let mut term: f64 = x;
    let mut sum: f64 = S21_NAN;
    let mut xv = x;
    if -1.0 < xv && xv < 1.0 {
        sum = term;
        xv *= xv;
        let mut k: i32 = 1;
        while castom_fabs(term) > EPS_10 {
            term *= xv * (k as f64) / ((k + 1) as f64);
            sum += term / ((k + 2) as f64);
            k += 2;
        }
    } else if x == 1.0 {
        sum = S21_M_PI / 2.0;
    } else if x == -1.0 {
        sum = -S21_M_PI / 2.0;
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
    if base == 0.0 && exp != 0.0 {
        0.0
    } else if exp < 0.0 {
        castom_pow(1.0 / base, -exp)
    } else if base < 0.0 && exp != (exp as i64) as f64 {
        -S21_NAN
    } else if exp == 0.0 {
        1.0
    } else {
        let mut flag: f64 = 1.0;
        let mut b = base;
        if b < 0.0 && (exp as i64) % 2 != 0 {
            flag = -1.0;
        }
        if b < 0.0 {
            b = -b;
        }
        let div: i64 = exp as i64;
        let mut integer_part: f64 = 1.0;
        for _ in 0..div {
            integer_part *= b;
        }
        integer_part * castom_exp((exp - div as f64) * castom_log(b)) * flag
    }
}

pub fn castom_abs(x: i32) -> i32 {
    if x > 0 {
        x
    } else {
        -x
    }
}

pub fn castom_log(x: f64) -> f64 {
    let mut a: u32 = 0;
    let mut b: f64 = 0.0;
    if x > 0.0 {
        let mut c: f64 = if x < 1.0 { 1.0 / x } else { x };
        // for (c = ...; (c /= S21_M_E) > 1; ++a) {}
        loop {
            c /= S21_M_E;
            if c > 1.0 {
                a += 1;
            } else {
                break;
            }
        }
        // c = 1 / (c * S21_M_E - 1); c = c + c + 1; f = c * c; b = 0;
        c = 1.0 / (c * S21_M_E - 1.0);
        c = c + c + 1.0;
        let f = c * c;
        b = 0.0;
        // for (d = 1, c /= 2; e = b, b += 1 / (d * c), b - e;) d += 2, c *= f;
        let mut d: u32 = 1;
        c /= 2.0;
        loop {
            let e = b;
            b += 1.0 / ((d as f64) * c);
            if b - e == 0.0 {
                break;
            }
            d += 2;
            c *= f;
        }
    } else {
        // b = (x == 0) / 0.0;
        // If x == 0, this is 1.0 / 0.0 = INFINITY, then negated below to give -INFINITY.
        // If x < 0, this is 0.0 / 0.0 = NaN.
        b = if x == 0.0 { 1.0 } else { 0.0 } / 0.0;
    }
    if x < 1.0 {
        -((a as f64) + b)
    } else {
        (a as f64) + b
    }
}

pub fn castom_sin(x: f64) -> f64 {
    let x = castom_fmod(x, 2.0 * S21_M_PI);
    let mut sum: f64 = 0.0;
    for i in 0..=20i32 {
        let mut fa: f64 = 1.0;
        let mut pw: f64 = 1.0;
        let upper = 2 * i + 1;
        for j in 1..=upper {
            fa *= j as f64;
            pw *= x;
        }
        let sign: f64 = if i % 2 != 0 { -1.0 } else { 1.0 };
        sum += (sign / fa) * pw;
    }
    sum
}

pub fn castom_cos(x: f64) -> f64 {
    let x = castom_fmod(x, 2.0 * S21_M_PI);
    let mut t_s: f64 = 0.0;
    let mut last: f64 = 1.0;
    let mut k: i32 = 1;
    while castom_fabs(last) > EPS_10 {
        t_s += last;
        last *= -x * x / (2.0 * k as f64 - 1.0) / (2.0 * k as f64);
        k += 1;
        if k > 100_000 {
            break;
        }
    }
    t_s
}

pub fn castom_fabs(x: f64) -> f64 {
    if x > 0.0 {
        x
    } else {
        -x
    }
}
