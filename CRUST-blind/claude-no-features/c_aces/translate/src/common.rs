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
pub fn xgcd(mut a: u64, mut b: u64) -> Xgcd {
    // Mirrors C: parameters named (x,y) in the body
    let mut x = a;
    let mut y = b;
    let _ = (&mut a, &mut b);

    let mut prev_a: i64 = 1;
    let mut a_coef: i64 = 0;
    let mut prev_b: i64 = 0;
    let mut b_coef: i64 = 1;

    while y != 0 {
        let q = (x / y) as i64;
        let rem = (x % y) as i64;
        x = y;
        y = rem as u64;

        let temp = a_coef;
        a_coef = prev_a.wrapping_sub(q.wrapping_mul(a_coef));
        prev_a = temp;

        let temp = b_coef;
        b_coef = prev_b.wrapping_sub(q.wrapping_mul(b_coef));
        prev_b = temp;
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

    Pair {
        first: a,
        second,
    }
}
pub fn randrange(lower: u64, upper: u64) -> u64 {
    // C: rand() % (upper - lower + 1) + lower
    // We use Rust's rand instead.
    let mut rng = rand::rng();
    let range = upper.wrapping_sub(lower).wrapping_add(1);
    if range == 0 {
        // Avoid div-by-zero; return lower
        return lower;
    }
    let r: u64 = rng.random();
    (r % range).wrapping_add(lower)
}
pub fn normal_rand(mean: f64, stddev: f64) -> f64 {
    // Mirrors Box-Muller method in C
    let normal = Normal::new(mean, stddev).unwrap_or_else(|_| Normal::new(0.0, 1.0).unwrap());
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
