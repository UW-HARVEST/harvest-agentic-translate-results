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
pub fn xgcd(mut x: u64, mut y: u64) -> Xgcd {
    let mut prev_a: i64 = 1;
    let mut a: i64 = 0;
    let mut prev_b: i64 = 0;
    let mut b: i64 = 1;

    while y != 0 {
        let q = (x / y) as i64;
        let temp = (x % y) as i64;
        x = y;
        y = temp as u64;

        let temp_a = a;
        a = prev_a - q * a;
        prev_a = temp_a;

        let temp_b = b;
        b = prev_b - q * b;
        prev_b = temp_b;
    }

    Xgcd {
        gcd: x,
        a: prev_a,
        b: prev_b,
    }
}
pub fn are_coprime(x: u64, y: u64) -> bool {
    !(gcd(x, y) > 1)
}
pub fn randinverse(value: u64) -> Pair {
    if value < 3 {
        // Avoid invalid range; fall back to identity-like result
        return Pair {
            first: 1,
            second: 1,
        };
    }
    let mut a = randrange(2, value - 1);
    while !are_coprime(a, value) {
        a = randrange(2, value - 1);
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
    if upper < lower {
        return lower;
    }
    let mut rng = rand::rng();
    rng.random_range(lower..=upper)
}
pub fn normal_rand(mean: f64, stddev: f64) -> f64 {
    // Avoid panics for stddev == 0 by sampling deterministically.
    if stddev == 0.0 {
        return mean;
    }
    let normal = match Normal::new(mean, stddev) {
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
