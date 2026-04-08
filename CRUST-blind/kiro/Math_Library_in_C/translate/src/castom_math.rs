// Constants
pub const S21_NAN: f64 = f64::NAN;
pub const EPS_6: f64 = 1e-06;
pub const S21_M_E: f64 = 2.71828182845904523536028747135266250;
pub const S21_INFINITY: f64 = f64::INFINITY;
pub const EPS_10: f64 = 1e-10;
pub const S21_M_PI: f64 = 3.14159265358979323846264338327950288;

// Function Declarations
pub fn castom_abs(x: i32) -> i32 {
    if x > 0 { x } else { -x }
}

pub fn castom_fabs(x: f64) -> f64 {
    if x > 0.0 { x } else { -x }
}

pub fn castom_trunc(x: f64) -> f64 {
    if x > 0.0 { castom_floor(x) } else { castom_ceil(x) }
}

pub fn castom_floor(x: f64) -> f64 {
    if x >= i64::MAX as f64 || x <= i64::MIN as f64 || x != x {
        return x;
    }
    let truncation = x as i64 as f64;
    if truncation > x { truncation - 1.0 } else { truncation }
}

pub fn castom_ceil(x: f64) -> f64 {
    if x >= i64::MAX as f64 || x <= i64::MIN as f64 || x != x {
        return x;
    }
    let truncation = x as i64 as f64;
    if truncation < x { truncation + 1.0 } else { truncation }
}

pub fn castom_fmod(x: f64, y: f64) -> f64 {
    x - castom_trunc(x / y) * y
}

pub fn castom_exp(x: f64) -> f64 {
    let mut sum: f64 = 1.0;
    let mut cur: f64 = 1.0;
    let mut n: u32 = 1;
    while castom_fabs(cur) > EPS_10 {
        cur *= x / n as f64;
        sum += cur;
        n += 1;
    }
    sum
}

pub fn castom_log(x: f64) -> f64 {
    if x > 0.0 {
        let mut a: u32 = 0;
        let mut c = if x < 1.0 { 1.0 / x } else { x };
        loop {
            c /= S21_M_E;
            if c <= 1.0 { break; }
            a += 1;
        }
        c = 1.0 / (c * S21_M_E - 1.0);
        c = c + c + 1.0;
        let f = c * c;
        c /= 2.0;
        let mut b: f64 = 0.0;
        let mut d: u32 = 1;
        loop {
            let e = b;
            b += 1.0 / (d as f64 * c);
            if b == e { break; }
            d += 2;
            c *= f;
        }
        if x < 1.0 { -(a as f64 + b) } else { a as f64 + b }
    } else {
        // x == 0 => -inf, x < 0 => NaN
        if x == 0.0 { f64::NEG_INFINITY } else { f64::NAN }
    }
}

pub fn castom_sin(x: f64) -> f64 {
    let x = castom_fmod(x, 2.0 * S21_M_PI);
    let mut sum: f64 = 0.0;
    for i in 0..=20 {
        let mut fa: f64 = 1.0;
        let mut pow: f64 = 1.0;
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
    let mut t_s: f64 = 0.0;
    let mut last: f64 = 1.0;
    let mut k = 1;
    while castom_fabs(last) > EPS_10 {
        t_s += last;
        last *= -x * x / (2.0 * k as f64 - 1.0) / (2.0 * k as f64);
        k += 1;
    }
    t_s
}

pub fn castom_asin(x: f64) -> f64 {
    if x == 1.0 {
        return S21_M_PI / 2.0;
    } else if x == -1.0 {
        return -S21_M_PI / 2.0;
    } else if x > -1.0 && x < 1.0 {
        let mut term: f64 = x;
        let mut sum: f64 = term;
        let x2 = x * x;
        let mut k = 1i64;
        while castom_fabs(term) > EPS_10 {
            term *= x2 * k as f64 / (k + 1) as f64;
            sum += term / (k + 2) as f64;
            k += 2;
        }
        return sum;
    }
    f64::NAN
}

pub fn castom_acos(x: f64) -> f64 {
    S21_M_PI / 2.0 - castom_asin(x)
}

pub fn castom_atan(x: f64) -> f64 {
    castom_asin(x / castom_sqrt(1.0 + x * x))
}

pub fn castom_sqrt(x: f64) -> f64 {
    if x != x || x < 0.0 {
        return f64::NAN;
    }
    let mut sqrt = x / 2.0;
    let mut temp = 0.0f64;
    while sqrt != temp {
        temp = sqrt;
        sqrt = (x / temp + temp) / 2.0;
    }
    sqrt
}

pub fn castom_tan(x: f64) -> f64 {
    if x == 0.0 {
        0.0
    } else if x == S21_M_PI / 6.0 {
        1.0 / castom_sqrt(3.0)
    } else if x == S21_M_PI / 4.0 {
        1.0
    } else if x == S21_M_PI / 3.0 {
        castom_sqrt(3.0)
    } else if x == S21_M_PI / 2.0 {
        S21_INFINITY
    } else if x == S21_M_PI {
        0.0
    } else if x == 3.0 * S21_M_PI / 2.0 {
        S21_INFINITY
    } else if x == 2.0 * S21_M_PI {
        0.0
    } else {
        castom_sin(x) / castom_cos(x)
    }
}

pub fn castom_pow(base: f64, exp: f64) -> f64 {
    if base == 0.0 && exp != 0.0 {
        0.0
    } else if exp < 0.0 {
        castom_pow(1.0 / base, -exp)
    } else if base < 0.0 && exp != (exp as i64) as f64 {
        f64::NAN
    } else if exp == 0.0 {
        1.0
    } else {
        let mut flag: f64 = 1.0;
        let mut base = base;
        if base < 0.0 && (exp as i64) % 2 != 0 {
            flag = -1.0;
        }
        if base < 0.0 { base = -base; }
        let div = exp as i64;
        let mut integer_part: f64 = 1.0;
        for _ in 0..div {
            integer_part *= base;
        }
        integer_part * castom_exp((exp - div as f64) * castom_log(base)) * flag
    }
}

pub fn castom_factorial(n: u64) -> u64 {
    if n == 0 { 1 } else { (1..=n).product() }
}
