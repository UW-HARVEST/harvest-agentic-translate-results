use rand::Rng;

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
    // Note: variables x, y in C correspond to a, b parameters here.
    let mut prev_a: i64 = 1;
    let mut cur_a: i64 = 0;
    let mut prev_b: i64 = 0;
    let mut cur_b: i64 = 1;

    while b != 0 {
        let q = (a / b) as i64;
        let r = a % b;
        a = b;
        b = r;

        let temp_a = cur_a;
        cur_a = prev_a - q * cur_a;
        prev_a = temp_a;

        let temp_b = cur_b;
        cur_b = prev_b - q * cur_b;
        prev_b = temp_b;
    }

    Xgcd {
        gcd: a,
        a: prev_a,
        b: prev_b,
    }
}
pub fn are_coprime(x: u64, y: u64) -> bool {
    !(gcd(x, y) > 1)
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
    // C: rand() % (upper - lower + 1) + lower (inclusive on both ends)
    let range = upper - lower + 1;
    rng.random_range(0..range) + lower
}
pub fn normal_rand(mean: f64, stddev: f64) -> f64 {
    const PI: f64 = 3.14159265;
    let mut rng = rand::rng();
    let mut u: f64 = 0.0;
    while u == 0.0 {
        u = rng.random::<f64>();
    }
    let r = (-2.0 * u.ln()).sqrt();
    let mut theta: f64 = 0.0;
    while theta == 0.0 {
        theta = 2.0 * PI * rng.random::<f64>();
    }
    let x = r * theta.cos();
    x * stddev + mean
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
