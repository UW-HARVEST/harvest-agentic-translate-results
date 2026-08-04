use rand::Rng;
use rand_distr::{Distribution, Normal};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Xgcd {
    pub gcd: u64,
    pub a: i64,
    pub b: i64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pair {
    pub first: u64,
    pub second: u64,
}
pub fn gcd(mut x: u64, mut y: u64) -> u64 {
    // Euclidean subtraction algorithm matching the C implementation.
    if x == 0 && y == 0 {
        return 0;
    }
    if x == 0 {
        return y;
    }
    if y == 0 {
        return x;
    }
    while x != y {
        if x > y {
            x -= y;
        } else {
            y -= x;
        }
    }
    x
}
pub fn xgcd(a: u64, b: u64) -> Xgcd {
    // Extended Euclidean algorithm matching the C implementation.
    let mut x = a;
    let mut y = b;
    let mut prev_a: i64 = 1;
    let mut a_coef: i64 = 0;
    let mut prev_b: i64 = 0;
    let mut b_coef: i64 = 1;

    while y != 0 {
        let q = (x / y) as i64;
        let temp_rem = (x % y) as i64;
        x = y;
        y = temp_rem as u64;

        let tmp_a = a_coef;
        a_coef = prev_a - q * a_coef;
        prev_a = tmp_a;

        let tmp_b = b_coef;
        b_coef = prev_b - q * b_coef;
        prev_b = tmp_b;
    }

    let _ = (a, b);
    Xgcd {
        gcd: x,
        a: prev_a,
        b: prev_b,
    }
}
pub fn are_coprime(x: u64, y: u64) -> bool {
    gcd(x, y) == 1
}
pub fn randinverse(value: u64) -> Pair {
    // Mirror the C implementation: pick a random `a` in [2, value-1] coprime to value
    // and return (a, modular inverse of a mod value).
    let mut a = randrange(2, value.saturating_sub(1));
    while !are_coprime(a, value) {
        a = randrange(2, value.saturating_sub(1));
    }
    let result = xgcd(a, value);
    let second = if result.a > 0 {
        result.a as u64
    } else {
        (result.a + value as i64) as u64
    };
    Pair { first: a, second }
}
pub fn randrange(lower: u64, upper: u64) -> u64 {
    // Inclusive range [lower, upper]
    if upper < lower {
        return lower;
    }
    let mut rng = rand::rng();
    let span = upper - lower + 1;
    if span == 0 {
        // span overflowed, pick any u64
        return rng.random::<u64>();
    }
    rng.random_range(0..span) + lower
}
pub fn normal_rand(mean: f64, stddev: f64) -> f64 {
    let dev = if stddev <= 0.0 { f64::EPSILON } else { stddev };
    let normal = match Normal::new(mean, dev) {
        Ok(n) => n,
        Err(_) => return mean,
    };
    let mut rng = rand::rng();
    normal.sample(&mut rng)
}
pub fn max(a: u64, b: u64) -> u64 {
    if a > b {
        a
    } else {
        b
    }
}
pub fn min(a: u64, b: u64) -> u64 {
    if a < b {
        a
    } else {
        b
    }
}
pub fn clamp(min_value: u64, max_value: u64, value: u64) -> u64 {
    max(min_value, min(max_value, value))
}
