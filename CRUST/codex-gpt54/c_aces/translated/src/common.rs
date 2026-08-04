use rand::Rng;
use std::f64::consts::PI;

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
    if x == 0 || y == 0 {
        return x.max(y);
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
pub fn xgcd(mut a: u64, mut b: u64) -> Xgcd {
    let mut prev_a = 1_i64;
    let mut curr_a = 0_i64;
    let mut prev_b = 0_i64;
    let mut curr_b = 1_i64;

    while b != 0 {
        let q = (a / b) as i64;
        let temp = (a % b) as i64;
        a = b;
        b = temp as u64;

        let old_a = curr_a;
        curr_a = prev_a - (q * curr_a);
        prev_a = old_a;

        let old_b = curr_b;
        curr_b = prev_b - (q * curr_b);
        prev_b = old_b;
    }

    Xgcd {
        gcd: a,
        a: prev_a,
        b: prev_b,
    }
}
pub fn are_coprime(x: u64, y: u64) -> bool {
    gcd(x, y) <= 1
}
pub fn randinverse(value: u64) -> Pair {
    let mut a = randrange(2, value - 1);
    while !are_coprime(a, value) {
        a = randrange(2, value - 1);
    }

    let result = xgcd(a, value);
    Pair {
        first: a,
        second: if result.a > 0 {
            result.a as u64
        } else {
            (result.a + value as i64) as u64
        },
    }
}
pub fn randrange(lower: u64, upper: u64) -> u64 {
    rand::rng().random_range(lower..=upper)
}
pub fn normal_rand(mean: f64, stddev: f64) -> f64 {
    let mut rng = rand::rng();

    let mut u = 0.0_f64;
    while u == 0.0 {
        u = rng.random::<f64>();
    }

    let r = (-2.0 * u.ln()).sqrt();

    let mut theta = 0.0_f64;
    while theta == 0.0 {
        theta = 2.0 * PI * rng.random::<f64>();
    }

    (r * theta.cos()) * stddev + mean
}
pub fn max(a: u64, b: u64) -> u64 {
    if a > b { a } else { b }
}
pub fn min(a: u64, b: u64) -> u64 {
    if a < b { a } else { b }
}
pub fn clamp(min_value: u64, max_value: u64, value: u64) -> u64 {
    max(min_value, min(max_value, value))
}
