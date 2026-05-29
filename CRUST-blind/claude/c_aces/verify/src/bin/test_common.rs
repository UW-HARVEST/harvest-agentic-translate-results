#![allow(unused_imports)]
use c_aces::common::{
    are_coprime, clamp, gcd, max, min, normal_rand, randinverse, randrange, xgcd, Pair, Xgcd,
};

#[test]
fn test_gcd_basic() {
    assert_eq!(gcd(1, 1), 1);
    assert_eq!(gcd(2, 1), 1);
    assert_eq!(gcd(1, 2), 1);
    assert_eq!(gcd(2, 2), 2);
    assert_eq!(gcd(6, 15), 3);
    assert_eq!(gcd(15, 6), 3);
    assert_eq!(gcd(7, 15), 1);
    assert_eq!(gcd(15, 7), 1);
    assert_eq!(gcd(11112, 44445), 3);
}

#[test]
fn test_gcd_more() {
    assert_eq!(gcd(48, 18), 6);
    assert_eq!(gcd(100, 75), 25);
    assert_eq!(gcd(17, 23), 1);
}

#[test]
fn test_xgcd_equal() {
    let r: Xgcd = xgcd(7, 7);
    assert_eq!(r.gcd, 7);
    assert_eq!(r.a, 0);
    assert_eq!(r.b, 1);

    let r = xgcd(2, 2);
    assert_eq!(r.gcd, 2);
    assert_eq!(r.a, 0);
    assert_eq!(r.b, 1);

    let r = xgcd(1, 1);
    assert_eq!(r.gcd, 1);
    assert_eq!(r.a, 0);
    assert_eq!(r.b, 1);
}

#[test]
fn test_xgcd_one_two() {
    let r = xgcd(1, 2);
    assert_eq!(r.gcd, 1);
    assert_eq!(r.a, 1);
    assert_eq!(r.b, 0);
}

#[test]
fn test_xgcd_misc() {
    let r = xgcd(5, 6);
    assert_eq!(r.gcd, 1);
    assert_eq!(r.a, -1);
    assert_eq!(r.b, 1);

    let r = xgcd(19, 13);
    assert_eq!(r.gcd, 1);
    assert_eq!(r.a, -2);
    assert_eq!(r.b, 3);

    let r = xgcd(48, 18);
    assert_eq!(r.gcd, 6);
    assert_eq!(r.a, -1);
    assert_eq!(r.b, 3);

    let r = xgcd(17, 23);
    assert_eq!(r.gcd, 1);
    assert_eq!(r.a, -4);
    assert_eq!(r.b, 3);

    let r = xgcd(100, 75);
    assert_eq!(r.gcd, 25);
    assert_eq!(r.a, 1);
    assert_eq!(r.b, -1);

    let r = xgcd(7, 5);
    assert_eq!(r.gcd, 1);
    assert_eq!(r.a, -2);
    assert_eq!(r.b, 3);
}

#[test]
fn test_are_coprime() {
    assert_eq!(are_coprime(7, 9), true);
    assert_eq!(are_coprime(6, 9), false);
    assert_eq!(are_coprime(17, 23), true);
    assert_eq!(are_coprime(48, 18), false);
}

#[test]
fn test_max_min_clamp() {
    assert_eq!(max(3, 5), 5);
    assert_eq!(max(5, 3), 5);
    assert_eq!(max(7, 7), 7);

    assert_eq!(min(3, 5), 3);
    assert_eq!(min(5, 3), 3);
    assert_eq!(min(7, 7), 7);

    // clamp(min, max, value): clamps value into [min, max]
    assert_eq!(clamp(2, 10, 5), 5);
    assert_eq!(clamp(2, 10, 1), 2);
    assert_eq!(clamp(2, 10, 11), 10);
    assert_eq!(clamp(2, 10, 2), 2);
    assert_eq!(clamp(2, 10, 10), 10);
}

#[test]
fn test_randrange_within_bounds() {
    for _ in 0..200 {
        let v = randrange(5, 10);
        assert!(v >= 5 && v <= 10);
    }
    for _ in 0..50 {
        let v = randrange(0, 0);
        assert_eq!(v, 0);
    }
}

#[test]
fn test_normal_rand_finite() {
    for _ in 0..50 {
        let v = normal_rand(0.0, 1.0);
        assert!(v.is_finite());
    }
    // stddev=0 yields mean
    let v = normal_rand(42.5, 0.0);
    assert_eq!(v, 42.5);
}

#[test]
fn test_randinverse_correct() {
    // randinverse(value) returns (a, a_inv) such that a * a_inv mod value == 1
    let value = 97u64;
    for _ in 0..30 {
        let p: Pair = randinverse(value);
        let prod = (p.first * p.second) % value;
        assert_eq!(prod, 1, "{} * {} mod {} != 1", p.first, p.second, value);
        assert!(p.first >= 2 && p.first < value);
    }

    let value = 13u64;
    for _ in 0..30 {
        let p = randinverse(value);
        let prod = (p.first * p.second) % value;
        assert_eq!(prod, 1);
    }
}

fn main() {}
