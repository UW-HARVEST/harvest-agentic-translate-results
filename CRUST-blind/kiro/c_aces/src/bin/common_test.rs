use c_aces::common::{are_coprime, clamp, gcd, max, min, xgcd, Xgcd};

#[test]
fn test_gcd_basic() {
    assert_eq!(gcd(1, 1), 1);
    assert_eq!(gcd(2, 1), 1);
    assert_eq!(gcd(1, 2), 1);
    assert_eq!(gcd(2, 2), 2);
}

#[test]
fn test_gcd_larger() {
    assert_eq!(gcd(6, 15), 3);
    assert_eq!(gcd(15, 6), 3);
    assert_eq!(gcd(7, 15), 1);
    assert_eq!(gcd(15, 7), 1);
    assert_eq!(gcd(11112, 44445), 3);
}

#[test]
fn test_xgcd() {
    let r = xgcd(7, 7);
    assert_eq!(r, Xgcd { gcd: 7, a: 0, b: 1 });

    let r = xgcd(2, 2);
    assert_eq!(r, Xgcd { gcd: 2, a: 0, b: 1 });

    let r = xgcd(1, 1);
    assert_eq!(r, Xgcd { gcd: 1, a: 0, b: 1 });

    let r = xgcd(1, 2);
    assert_eq!(r, Xgcd { gcd: 1, a: 1, b: 0 });

    let r = xgcd(5, 6);
    assert_eq!(r, Xgcd { gcd: 1, a: -1, b: 1 });

    let r = xgcd(19, 13);
    assert_eq!(r, Xgcd { gcd: 1, a: -2, b: 3 });
}

#[test]
fn test_are_coprime() {
    assert!(are_coprime(7, 15));
    assert!(are_coprime(1, 2));
    assert!(are_coprime(5, 6));
    assert!(!are_coprime(6, 15));
    assert!(!are_coprime(4, 6));
}

#[test]
fn test_min_max() {
    assert_eq!(min(3, 5), 3);
    assert_eq!(min(5, 3), 3);
    assert_eq!(min(3, 3), 3);
    assert_eq!(max(3, 5), 5);
    assert_eq!(max(5, 3), 5);
    assert_eq!(max(3, 3), 3);
}

#[test]
fn test_clamp() {
    assert_eq!(clamp(2, 8, 5), 5);
    assert_eq!(clamp(2, 8, 1), 2);
    assert_eq!(clamp(2, 8, 10), 8);
    assert_eq!(clamp(2, 8, 2), 2);
    assert_eq!(clamp(2, 8, 8), 8);
    assert_eq!(clamp(0, 0, 0), 0);
}

fn main() {}
