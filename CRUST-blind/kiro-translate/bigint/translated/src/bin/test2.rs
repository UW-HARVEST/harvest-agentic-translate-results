use bigint::bigint::*;

fn main() {}

#[test]
fn test_from_int() {
    let x = BigInt::from_int(100);
    let y = BigInt::from_str("100");
    assert_eq!(x, y);
}

#[test]
fn test_comparison_equal() {
    let x = BigInt::from_int(100);
    let y = BigInt::from_int(100);
    assert_eq!(x, y);
    assert!(le(&x, &y));
    assert!(ge(&x, &y));
    assert!(!lt(&x, &y));
    assert!(!gt(&x, &y));
}

#[test]
fn test_comparison_less() {
    let x = BigInt::from_int(100);
    let y = BigInt::from_int(101);
    assert_ne!(x, y);
    assert!(le(&x, &y));
    assert!(!ge(&x, &y));
    assert!(lt(&x, &y));
    assert!(!gt(&x, &y));
}

#[test]
fn test_comparison_greater() {
    let x = BigInt::from_int(101);
    let y = BigInt::from_int(100);
    assert_ne!(x, y);
    assert!(!le(&x, &y));
    assert!(ge(&x, &y));
    assert!(!lt(&x, &y));
    assert!(gt(&x, &y));
}

#[test]
fn test_add() {
    let x = BigInt::from_int(100);
    let y = BigInt::from_int(200);
    let z = x.add(&y);
    assert_eq!(z, BigInt::from_int(300));
}

#[test]
fn test_sub() {
    let x = BigInt::from_int(200);
    let y = BigInt::from_int(100);
    let z = x.sub(&y);
    assert_eq!(z, BigInt::from_int(100));
}

#[test]
fn test_mul() {
    let x = BigInt::from_int(100);
    let y = BigInt::from_int(200);
    let z = x.mul(&y);
    assert_eq!(z, BigInt::from_int(20000));
}

#[test]
fn test_div() {
    let x = BigInt::from_int(200);
    let y = BigInt::from_int(100);
    let z = x.div(&y);
    assert_eq!(z, BigInt::from_int(2));
}

#[test]
fn test_mod_zero() {
    let x = BigInt::from_int(200);
    let y = BigInt::from_int(100);
    let z = x.r#mod(&y);
    assert_eq!(z, BigInt::from_int(0));
}

#[test]
fn test_mod_one() {
    let x = BigInt::from_int(201);
    let y = BigInt::from_int(100);
    let z = x.r#mod(&y);
    assert_eq!(z, BigInt::from_int(1));
}

#[test]
fn test_pow() {
    let x = BigInt::from_int(2);
    let y = BigInt::from_int(10);
    let z = x.pow(&y);
    assert_eq!(z, BigInt::from_int(1024));
}

#[test]
fn test_modinv() {
    let a = BigInt::from_int(17);
    let m = BigInt::from_int(43);
    let z = a.modinv(&m);
    let t = a.mul(&z);
    let r = t.r#mod(&m);
    assert_eq!(r, BigInt::from_int(1));
}

#[test]
fn test_sqrt() {
    let x = BigInt::from_int(100);
    let z = x.sqrt();
    assert_eq!(z, BigInt::from_int(10));

    let x = BigInt::from_int(101);
    let z = x.sqrt();
    assert_eq!(z, BigInt::from_int(11));
}

#[test]
fn test_is_prime() {
    let x = BigInt::from_int(17);
    assert!(x.is_prime());
    let x = BigInt::from_int(18);
    assert!(!x.is_prime());
}

#[test]
fn test_fast_pow() {
    let x = BigInt::from_int(745232);
    let y = BigInt::from_int(67121);
    let z = BigInt::from_int(1022117);
    let result = x.fast_pow(&y, &z);
    assert_eq!(result, BigInt::from_int(97));
}
