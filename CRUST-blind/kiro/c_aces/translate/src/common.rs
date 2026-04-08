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
    let mut prev_a: i64 = 1;
    let mut ca: i64 = 0;
    let mut prev_b: i64 = 0;
    let mut cb: i64 = 1;

    while b != 0 {
        let q = (a / b) as i64;
        let temp = (a % b) as i64;
        a = b;
        b = temp as u64;

        let temp = ca;
        ca = prev_a - q * ca;
        prev_a = temp;

        let temp = cb;
        cb = prev_b - q * cb;
        prev_b = temp;
    }

    Xgcd { gcd: a, a: prev_a, b: prev_b }
}
pub fn are_coprime(x: u64, y: u64) -> bool {
    gcd(x, y) == 1
}
pub fn randinverse(value: u64) -> Pair {
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
    let mut rng = rand::rng();
    rng.random_range(lower..=upper)
}
pub fn normal_rand(mean: f64, stddev: f64) -> f64 {
    let mut rng = rand::rng();
    let normal = Normal::new(mean, stddev).unwrap();
    normal.sample(&mut rng)
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
