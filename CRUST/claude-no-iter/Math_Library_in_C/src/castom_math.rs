// Constants
pub const S21_NAN: f64 = f64::NAN;
pub const EPS_6: f64 = 1e-06;
pub const S21_M_E: f64 = 2.71828182845904523536028747135266250;
pub const S21_INFINITY: f64 = f64::INFINITY;
pub const EPS_10: f64 = 1e-10;
pub const S21_M_PI: f64 = 3.14159265358979323846264338327950288;

// Function Declarations
pub fn castom_abs(x: i32) -> i32 {
    if x > 0 {
        x
    } else {
        -x
    }
}

pub fn castom_fabs(x: f64) -> f64 {
    if x > 0.0 {
        x
    } else {
        -x
    }
}

pub fn castom_floor(x: f64) -> f64 {
    // Mirror C: if x >= LLONG_MAX, x <= LLONG_MIN, or NaN, return as-is.
    if x.is_nan() || x >= i64::MAX as f64 || x <= i64::MIN as f64 {
        return x;
    }
    let truncation = (x as i64) as f64;
    if truncation > x {
        truncation - 1.0
    } else {
        truncation
    }
}

pub fn castom_ceil(x: f64) -> f64 {
    if x.is_nan() || x >= i64::MAX as f64 || x <= i64::MIN as f64 {
        return x;
    }
    let truncation = (x as i64) as f64;
    if truncation < x {
        truncation + 1.0
    } else {
        truncation
    }
}

pub fn castom_trunc(x: f64) -> f64 {
    if x > 0.0 {
        castom_floor(x)
    } else {
        castom_ceil(x)
    }
}

pub fn castom_fmod(x: f64, y: f64) -> f64 {
    x - castom_trunc(x / y) * y
}

pub fn castom_sqrt(x: f64) -> f64 {
    if x.is_nan() || x < 0.0 {
        return S21_NAN;
    }
    if x == 0.0 {
        return 0.0;
    }
    let mut sqrt_v = x / 2.0;
    let mut temp = 0.0;
    while sqrt_v != temp {
        temp = sqrt_v;
        sqrt_v = (x / temp + temp) / 2.0;
    }
    sqrt_v
}

pub fn castom_exp(x: f64) -> f64 {
    if x.is_nan() {
        return x;
    }
    if x == 0.0 {
        return 1.0;
    }

    // Range reduction: exp(x) = 2^k * exp(r) where r = x - k * ln(2).
    // Use a hi/lo split of ln(2) for better precision when k is large.
    let ln2: f64 = 0.6931471805599453;
    let ln2_hi: f64 = 0.6931471803691238;
    let ln2_lo: f64 = 1.9082149292705877e-10;

    let k = (x / ln2).round();
    let kf = k;
    let r = (x - kf * ln2_hi) - kf * ln2_lo;

    // Compute exp(r) via Taylor series with Kahan summation; |r| <= ln(2)/2.
    let mut sum: f64 = 1.0;
    let mut c: f64 = 0.0;
    let mut term: f64 = 1.0;
    let mut n: u32 = 1;
    while castom_fabs(term) > 1e-20 {
        term *= r / (n as f64);
        let y = term - c;
        let t = sum + y;
        c = (t - sum) - y;
        sum = t;
        n = n.wrapping_add(1);
        if n > 200 {
            break;
        }
    }

    // Multiply by 2^k via bit manipulation.
    let k_int = k as i32;
    sum * pow2_int(k_int)
}

/// Compute 2^k for an integer k.
fn pow2_int(k: i32) -> f64 {
    if k > 1023 {
        return f64::INFINITY;
    }
    if k < -1074 {
        return 0.0;
    }
    if k >= -1022 {
        let bits: u64 = ((k + 1023) as u64) << 52;
        f64::from_bits(bits)
    } else {
        // Subnormal range: 2^k for k < -1022.
        let shift = (-1022 - k) as u32;
        if shift > 52 {
            return 0.0;
        }
        let bits: u64 = 1u64 << (52 - shift);
        f64::from_bits(bits)
    }
}

pub fn castom_factorial(n: u64) -> u64 {
    let mut result: u64 = 1;
    let mut i: u64 = 2;
    while i <= n {
        result = result.saturating_mul(i);
        i += 1;
    }
    result
}

pub fn castom_asin(x: f64) -> f64 {
    let mut sum = S21_NAN;
    if -1.0 < x && x < 1.0 {
        let mut term = x;
        sum = term;
        let xx = x * x;
        let mut k: i32 = 1;
        while castom_fabs(term) > EPS_10 {
            term *= xx * (k as f64) / ((k + 1) as f64);
            sum += term / ((k + 2) as f64);
            k += 2;
            if k > 1_000_000 {
                break;
            }
        }
    } else if x == 1.0 {
        sum = S21_M_PI / 2.0;
    } else if x == -1.0 {
        sum = -S21_M_PI / 2.0;
    }
    sum
}

pub fn castom_acos(x: f64) -> f64 {
    S21_M_PI / 2.0 - castom_asin(x)
}

pub fn castom_atan(x: f64) -> f64 {
    castom_asin(x / castom_sqrt(1.0 + x * x))
}

pub fn castom_log(x: f64) -> f64 {
    let mut a: u32 = 0;
    let b: f64;
    if x > 0.0 {
        let mut c: f64 = if x < 1.0 { 1.0 / x } else { x };
        loop {
            c /= S21_M_E;
            if c > 1.0 {
                a += 1;
            } else {
                break;
            }
        }
        c = 1.0 / (c * S21_M_E - 1.0);
        c = c + c + 1.0;
        let f = c * c;
        let mut b_val: f64 = 0.0;
        let mut d: u32 = 1;
        c /= 2.0;
        loop {
            let e = b_val;
            b_val += 1.0 / ((d as f64) * c);
            if b_val - e == 0.0 {
                break;
            }
            d = d.wrapping_add(2);
            c *= f;
            if d > 10_000_000 {
                break;
            }
        }
        b = b_val;
    } else {
        // For x == 0: 1.0/0.0 = inf; for x < 0 or NaN: 0.0/0.0 = NaN
        b = if x == 0.0 { 1.0 / 0.0 } else { 0.0 / 0.0 };
    }
    if x < 1.0 {
        -((a as f64) + b)
    } else {
        (a as f64) + b
    }
}

pub fn castom_pow(base: f64, exp: f64) -> f64 {
    if base == 0.0 && exp != 0.0 {
        return 0.0;
    }
    if exp < 0.0 {
        return castom_pow(1.0 / base, -exp);
    }
    // Check whether exp is integer-valued (mirroring C's `exp != (long)exp`)
    let exp_as_long = exp as i64;
    if base < 0.0 && exp != (exp_as_long as f64) {
        return -S21_NAN;
    }
    if exp == 0.0 {
        return 1.0;
    }

    let mut flag: f64 = 1.0;
    let mut base_v = base;
    if base_v < 0.0 && exp_as_long % 2 != 0 {
        flag = -1.0;
    }
    if base_v < 0.0 {
        base_v = -base_v;
    }
    let div = exp_as_long;
    let mut integer_part: f64 = 1.0;
    let mut i: i64 = 0;
    while i < div {
        integer_part *= base_v;
        i += 1;
    }
    integer_part * castom_exp((exp - (div as f64)) * castom_log(base_v)) * flag
}

pub fn castom_sin(x: f64) -> f64 {
    let x = castom_fmod(x, 2.0 * S21_M_PI);
    let mut sum: f64 = 0.0;
    for i in 0..=20 {
        let mut fa: f64 = 1.0;
        let mut pow: f64 = 1.0;
        let upper = 2 * i + 1;
        let mut j = 1;
        while j <= upper {
            fa *= j as f64;
            pow *= x;
            j += 1;
        }
        let sign = if i % 2 != 0 { -1.0 } else { 1.0 };
        sum += (sign / fa) * pow;
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
        last *= -x * x / (2.0 * (k as f64) - 1.0) / (2.0 * (k as f64));
        k += 1;
        if k > 1_000_000 {
            break;
        }
    }
    t_s
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
