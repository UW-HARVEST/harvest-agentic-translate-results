use c_aces::common;

#[test]
fn test_gcd() {
    assert_eq!(common::gcd(1, 1), 1);
    assert_eq!(common::gcd(2, 1), 1);
    assert_eq!(common::gcd(1, 2), 1);
    assert_eq!(common::gcd(2, 2), 2);
    assert_eq!(common::gcd(6, 15), 3);
    assert_eq!(common::gcd(15, 6), 3);
    assert_eq!(common::gcd(7, 15), 1);
    assert_eq!(common::gcd(11112, 44445), 3);
    assert_eq!(common::gcd(12, 8), 4);
    assert_eq!(common::gcd(100, 75), 25);
}

#[test]
fn test_xgcd() {
    let r = common::xgcd(7, 7);
    assert_eq!(r.gcd, 7); assert_eq!(r.a, 0); assert_eq!(r.b, 1);

    let r = common::xgcd(2, 2);
    assert_eq!(r.gcd, 2); assert_eq!(r.a, 0); assert_eq!(r.b, 1);

    let r = common::xgcd(1, 1);
    assert_eq!(r.gcd, 1); assert_eq!(r.a, 0); assert_eq!(r.b, 1);

    let r = common::xgcd(1, 2);
    assert_eq!(r.gcd, 1); assert_eq!(r.a, 1); assert_eq!(r.b, 0);

    let r = common::xgcd(5, 6);
    assert_eq!(r.gcd, 1); assert_eq!(r.a, -1); assert_eq!(r.b, 1);

    let r = common::xgcd(19, 13);
    assert_eq!(r.gcd, 1); assert_eq!(r.a, -2); assert_eq!(r.b, 3);

    let r = common::xgcd(35, 15);
    assert_eq!(r.gcd, 5); assert_eq!(r.a, 1); assert_eq!(r.b, -2);

    let r = common::xgcd(120, 23);
    assert_eq!(r.gcd, 1); assert_eq!(r.a, -9); assert_eq!(r.b, 47);
}

#[test]
fn test_are_coprime() {
    assert!(common::are_coprime(3, 5));
    assert!(!common::are_coprime(4, 6));
    assert!(common::are_coprime(7, 11));
    assert!(!common::are_coprime(12, 8));
    assert!(common::are_coprime(1, 100));
}

#[test]
fn test_max() {
    assert_eq!(common::max(3, 5), 5);
    assert_eq!(common::max(5, 3), 5);
    assert_eq!(common::max(7, 7), 7);
}

#[test]
fn test_min() {
    assert_eq!(common::min(3, 5), 3);
    assert_eq!(common::min(5, 3), 3);
    assert_eq!(common::min(7, 7), 7);
}

#[test]
fn test_clamp() {
    assert_eq!(common::clamp(2, 8, 5), 5);
    assert_eq!(common::clamp(2, 8, 1), 2);
    assert_eq!(common::clamp(2, 8, 10), 8);
    assert_eq!(common::clamp(0, 0, 0), 0);
}

fn main() {}
