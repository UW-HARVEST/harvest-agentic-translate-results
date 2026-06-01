use c_aces::common::{are_coprime, clamp, gcd, max, min, xgcd, Pair, Xgcd};

#[test]
fn test_gcd() {
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
fn test_xgcd_equal() {
    let r = xgcd(7, 7);
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
fn test_xgcd_unit() {
    let r = xgcd(1, 2);
    assert_eq!(r.gcd, 1);
    assert_eq!(r.a, 1);
    assert_eq!(r.b, 0);
}

#[test]
fn test_xgcd_mixed() {
    let r = xgcd(5, 6);
    assert_eq!(r.gcd, 1);
    assert_eq!(r.a, -1);
    assert_eq!(r.b, 1);

    let r = xgcd(19, 13);
    assert_eq!(r.gcd, 1);
    assert_eq!(r.a, -2);
    assert_eq!(r.b, 3);

    // gcd(35,15)=5, a=1, b=-2
    let r = xgcd(35, 15);
    assert_eq!(r.gcd, 5);
    assert_eq!(r.a, 1);
    assert_eq!(r.b, -2);

    // xgcd with x=0
    let r = xgcd(0, 5);
    assert_eq!(r.gcd, 5);
    assert_eq!(r.a, 0);
    assert_eq!(r.b, 1);
}

#[test]
fn test_xgcd_struct_layout() {
    let r = Xgcd { gcd: 1, a: 2, b: 3 };
    assert_eq!(r.gcd, 1);
    assert_eq!(r.a, 2);
    assert_eq!(r.b, 3);
}

#[test]
fn test_pair_struct() {
    let p = Pair {
        first: 5,
        second: 10,
    };
    assert_eq!(p.first, 5);
    assert_eq!(p.second, 10);
}

#[test]
fn test_are_coprime() {
    assert_eq!(are_coprime(7, 15), true);
    assert_eq!(are_coprime(6, 15), false);
    assert_eq!(are_coprime(11, 13), true);
    assert_eq!(are_coprime(2, 4), false);
    assert_eq!(are_coprime(1, 1), true);
}

#[test]
fn test_max_min() {
    assert_eq!(max(2, 3), 3);
    assert_eq!(max(3, 2), 3);
    assert_eq!(max(5, 5), 5);
    assert_eq!(min(2, 3), 2);
    assert_eq!(min(3, 2), 2);
    assert_eq!(min(5, 5), 5);
}

#[test]
fn test_clamp() {
    assert_eq!(clamp(0, 10, 5), 5);
    assert_eq!(clamp(0, 10, 15), 10);
    assert_eq!(clamp(0, 10, 0), 0);
    assert_eq!(clamp(5, 10, 3), 5);
    assert_eq!(clamp(5, 10, 5), 5);
    assert_eq!(clamp(5, 10, 10), 10);
}

#[test]
fn test_randrange_in_range() {
    use c_aces::common::randrange;
    for _ in 0..100 {
        let v = randrange(5, 10);
        assert!(v >= 5 && v <= 10);
    }
    // single value range
    for _ in 0..10 {
        let v = randrange(7, 7);
        assert_eq!(v, 7);
    }
}

#[test]
fn test_randinverse_correctness() {
    use c_aces::common::randinverse;
    // For prime modulus, randinverse should produce a*b ≡ 1 mod prime
    for _ in 0..20 {
        let p = 97u64;
        let pair = randinverse(p);
        let product = (pair.first * pair.second) % p;
        assert_eq!(product, 1);
        assert!(pair.first >= 2 && pair.first <= p - 1);
    }
}

#[test]
fn test_normal_rand_finite() {
    use c_aces::common::normal_rand;
    // Just check it returns finite values for reasonable parameters.
    for _ in 0..50 {
        let v = normal_rand(0.0, 1.0);
        assert!(v.is_finite());
    }
}

fn main() {}
