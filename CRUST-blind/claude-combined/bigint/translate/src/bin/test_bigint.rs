extern crate bigint as bigint_crate;
#[allow(unused_imports)]
use bigint_crate::bigint::{ge, gt, le, lt, BigInt};

#[test]
fn test_zero() {
    let z = BigInt::zero();
    assert!(z.is_zero());
    assert_eq!(z.to_int(), 0);
    assert!(!z.lt_zero());
    assert!(!z.gt_zero());
    assert!(z.lezero());
    assert!(z.gezero());
}

#[test]
fn test_from_int_to_int() {
    let a = BigInt::from_int(0);
    assert_eq!(a.to_int(), 0);
    let a = BigInt::from_int(123);
    assert_eq!(a.to_int(), 123);
    let a = BigInt::from_int(-456);
    assert_eq!(a.to_int(), -456);
    let a = BigInt::from_int(1_000_000_000);
    assert_eq!(a.to_int(), 1_000_000_000);
}

#[test]
fn test_from_str() {
    let a = BigInt::from_str("123");
    assert_eq!(a.to_int(), 123);
    let b = BigInt::from_str("-789");
    assert_eq!(b.to_int(), -789);
    let c = BigInt::from_str("0");
    assert!(c.is_zero());
}

#[test]
fn test_from_int_equals_from_str() {
    let a = BigInt::from_int(100);
    let b = BigInt::from_str("100");
    assert_eq!(a, b);
}

#[test]
fn test_remove_leading_zeros() {
    let mut a = BigInt::from_str("000123");
    a.remove_leading_zeros();
    assert_eq!(a.to_int(), 123);
    // After removing leading zeros, internal size is 3.
    let mut z = BigInt::from_str("0000");
    z.remove_leading_zeros();
    assert!(z.is_zero());
    assert_eq!(z.to_int(), 0);
}

#[test]
fn test_copy() {
    let a = BigInt::from_int(123);
    let b = a.copy();
    assert_eq!(a, b);
    assert_eq!(b.to_int(), 123);
}

#[test]
fn test_is_zero_lt_gt_zero() {
    let z = BigInt::from_int(0);
    assert!(z.is_zero());
    assert!(!z.lt_zero());
    assert!(!z.gt_zero());
    assert!(z.lezero());
    assert!(z.gezero());

    let p = BigInt::from_int(5);
    assert!(!p.is_zero());
    assert!(!p.lt_zero());
    assert!(p.gt_zero());
    assert!(!p.lezero());
    assert!(p.gezero());

    let n = BigInt::from_int(-5);
    assert!(!n.is_zero());
    assert!(n.lt_zero());
    assert!(!n.gt_zero());
    assert!(n.lezero());
    assert!(!n.gezero());
}

#[test]
fn test_abs() {
    let a = BigInt::from_int(-123);
    let b = a.abs();
    assert_eq!(b.to_int(), 123);
    assert!(!b.lt_zero());

    let c = BigInt::from_int(456);
    assert_eq!(c.abs().to_int(), 456);
}

#[test]
fn test_eq() {
    let a = BigInt::from_int(100);
    let b = BigInt::from_int(100);
    assert_eq!(a, b);
    let c = BigInt::from_int(101);
    assert!(a != c);
}

#[test]
fn test_compare_same_sign() {
    let a = BigInt::from_int(100);
    let b = BigInt::from_int(101);
    assert!(lt(&a, &b));
    assert!(le(&a, &b));
    assert!(!gt(&a, &b));
    assert!(!ge(&a, &b));

    assert!(gt(&b, &a));
    assert!(ge(&b, &a));
    assert!(!lt(&b, &a));
    assert!(!le(&b, &a));

    let c = BigInt::from_int(100);
    assert!(le(&a, &c));
    assert!(ge(&a, &c));
    assert!(!lt(&a, &c));
    assert!(!gt(&a, &c));
}

#[test]
fn test_add() {
    // 100 + 200 = 300
    let z = BigInt::from_int(100).add(&BigInt::from_int(200));
    assert_eq!(z, BigInt::from_int(300));
    // 0 + 5 = 5
    let z = BigInt::from_int(0).add(&BigInt::from_int(5));
    assert_eq!(z, BigInt::from_int(5));
    // -50 + 30 = -20
    let z = BigInt::from_int(-50).add(&BigInt::from_int(30));
    assert_eq!(z, BigInt::from_int(-20));
    // -30 + 50 = 20
    let z = BigInt::from_int(-30).add(&BigInt::from_int(50));
    assert_eq!(z, BigInt::from_int(20));
}

#[test]
fn test_sub() {
    // 200 - 100 = 100
    let z = BigInt::from_int(200).sub(&BigInt::from_int(100));
    assert_eq!(z, BigInt::from_int(100));
    // 0 - 5 = -5
    let z = BigInt::from_int(0).sub(&BigInt::from_int(5));
    assert_eq!(z, BigInt::from_int(-5));
    // -50 - 30 = -80
    let z = BigInt::from_int(-50).sub(&BigInt::from_int(30));
    assert_eq!(z, BigInt::from_int(-80));
    // -50 - (-30) = -80 (matches C buggy semantics: both negative -> add abs and set negative=true)
    let z = BigInt::from_int(-50).sub(&BigInt::from_int(-30));
    assert_eq!(z, BigInt::from_int(-80));
    // 50 - (-30) = 80
    let z = BigInt::from_int(50).sub(&BigInt::from_int(-30));
    assert_eq!(z, BigInt::from_int(80));
}

#[test]
fn test_inc_dec() {
    let mut a = BigInt::from_int(99);
    a.inc();
    assert_eq!(a, BigInt::from_int(100));

    let mut a = BigInt::from_int(100);
    a.dec();
    assert_eq!(a, BigInt::from_int(99));

    let mut a = BigInt::from_int(-1);
    a.inc();
    assert_eq!(a, BigInt::from_int(0));
}

#[test]
fn test_mul() {
    // small (fits in 64-bit)
    let z = BigInt::from_int(100).mul(&BigInt::from_int(200));
    assert_eq!(z, BigInt::from_int(20000));
    let z = BigInt::from_int(0).mul(&BigInt::from_int(123));
    assert_eq!(z, BigInt::from_int(0));
    let z = BigInt::from_int(-3).mul(&BigInt::from_int(7));
    assert_eq!(z, BigInt::from_int(-21));

    // big * big — fall through to schoolbook path
    let a = BigInt::from_str("123456789012345678901234567890");
    let b = BigInt::from_str("987654321098765432109876543210");
    let c = a.mul(&b);
    let expected = BigInt::from_str("121932631137021795226185032733622923332237463801111263526900");
    assert_eq!(c, expected);
}

#[test]
fn test_div() {
    let z = BigInt::from_int(200).div(&BigInt::from_int(100));
    assert_eq!(z, BigInt::from_int(2));
    let z = BigInt::from_int(-10).div(&BigInt::from_int(3));
    assert_eq!(z, BigInt::from_int(-3));
}

#[test]
fn test_mod() {
    let z = BigInt::from_int(200).r#mod(&BigInt::from_int(100));
    assert_eq!(z, BigInt::from_int(0));
    let z = BigInt::from_int(201).r#mod(&BigInt::from_int(100));
    assert_eq!(z, BigInt::from_int(1));
    let z = BigInt::from_int(-10).r#mod(&BigInt::from_int(3));
    assert_eq!(z, BigInt::from_int(-1));
}

#[test]
fn test_divmod() {
    let (q, r) = BigInt::from_int(201).divmod(&BigInt::from_int(100));
    assert_eq!(q, BigInt::from_int(2));
    assert_eq!(r, BigInt::from_int(1));
}

#[test]
fn test_pow() {
    let z = BigInt::from_int(2).pow(&BigInt::from_int(10));
    assert_eq!(z, BigInt::from_int(1024));

    let z = BigInt::from_int(3).pow(&BigInt::from_int(5));
    assert_eq!(z, BigInt::from_int(243));

    // 7^0 = 1
    let z = BigInt::from_int(7).pow(&BigInt::from_int(0));
    assert_eq!(z, BigInt::from_int(1));

    // 5^0 = 1
    let z = BigInt::from_int(5).pow(&BigInt::from_int(0));
    assert_eq!(z, BigInt::from_int(1));

    // Negative exponent -> 0
    let z = BigInt::from_int(5).pow(&BigInt::from_int(-1));
    assert_eq!(z, BigInt::from_int(0));
}

#[test]
fn test_fast_pow() {
    let z = BigInt::from_int(745232).fast_pow(&BigInt::from_int(67121), &BigInt::from_int(1022117));
    assert_eq!(z, BigInt::from_int(97));

    let z = BigInt::from_str("2").fast_pow(&BigInt::from_str("1234"), &BigInt::from_str("10007"));
    assert_eq!(z, BigInt::from_int(8621));
}

#[test]
fn test_modinv() {
    // inverse of 17 mod 43 satisfies 17*x ≡ 1 (mod 43)
    let inv = BigInt::from_int(17).modinv(&BigInt::from_int(43));
    assert_eq!(inv, BigInt::from_int(38));
    // verify
    let prod = BigInt::from_int(17).mul(&inv);
    let r = prod.r#mod(&BigInt::from_int(43));
    assert_eq!(r, BigInt::from_int(1));
}

#[test]
fn test_sqrt() {
    let z = BigInt::from_int(100).sqrt();
    assert_eq!(z, BigInt::from_int(10));

    let z = BigInt::from_int(101).sqrt();
    assert_eq!(z, BigInt::from_int(11));

    let z = BigInt::from_int(0).sqrt();
    assert_eq!(z, BigInt::from_int(0));

    let z = BigInt::from_int(1).sqrt();
    assert_eq!(z, BigInt::from_int(1));

    let z = BigInt::from_int(123412341).sqrt();
    assert_eq!(z, BigInt::from_int(11110));

    let z = BigInt::from_int(10000).sqrt();
    assert_eq!(z, BigInt::from_int(100));
}

#[test]
fn test_is_even_odd() {
    assert!(BigInt::from_int(10).is_even());
    assert!(!BigInt::from_int(10).is_odd());
    assert!(!BigInt::from_int(11).is_even());
    assert!(BigInt::from_int(11).is_odd());
    assert!(BigInt::from_int(0).is_even());
    assert!(!BigInt::from_int(0).is_odd());
}

#[test]
fn test_is_prime_small() {
    let primes = [2i64, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53];
    let prime_results = [false, false, true, true, true, true, true, true, true, true,
                         true, true, true, true, true, true];
    // Note: C's bigint_is_prime returns false for 2 and 3 due to short-circuiting bugs.
    for (p, expected) in primes.iter().zip(prime_results.iter()) {
        let n = BigInt::from_int(*p);
        let got = n.is_prime();
        assert_eq!(got, *expected, "is_prime({}) expected {}", p, expected);
    }

    // composites
    assert!(!BigInt::from_int(4).is_prime());
    assert!(!BigInt::from_int(15).is_prime());
    assert!(!BigInt::from_int(18).is_prime());
    assert!(!BigInt::from_int(25).is_prime());
}

#[test]
fn test_is_64_bit() {
    // size 1..=9 is 64-bit; 10+ is not.
    let a = BigInt::from_int(123);
    assert!(a.is_64_bit());
    let a = BigInt::from_str("123456789"); // size 9
    assert!(a.is_64_bit());
    let a = BigInt::from_str("1234567890"); // size 10
    assert!(!a.is_64_bit());
    let a = BigInt::from_str("12345678901"); // size 11
    assert!(!a.is_64_bit());
}

#[test]
fn test_zero_compare_buggy() {
    // The C code's bigint_gt has a known bug: if signs differ but absolute values
    // are equal, it returns false. We must match this behavior.
    let a = BigInt::from_int(5);
    let b = BigInt::from_int(-5);
    assert!(!gt(&a, &b)); // 5 > -5 -> false (matches C bug)
    assert!(lt(&a, &b)); // 5 < -5 -> true (matches C bug)
}

#[test]
fn test_compare_zero_to_zero() {
    let a = BigInt::from_int(0);
    let b = BigInt::from_int(0);
    assert_eq!(a, b);
    assert!(!lt(&a, &b));
    assert!(!gt(&a, &b));
    assert!(le(&a, &b));
    assert!(ge(&a, &b));
}

#[test]
fn test_neg() {
    let a = BigInt::from_int(123);
    let n = -a;
    assert_eq!(n, BigInt::from_int(-123));

    let a = BigInt::from_int(-456);
    let n = -a;
    // The Neg impl flips negative, so -(-456) = 456.
    assert_eq!(n, BigInt::from_int(456));
}

#[test]
fn test_display() {
    let a = BigInt::from_int(123);
    assert_eq!(format!("{}", a), "123");
    let a = BigInt::from_int(-456);
    assert_eq!(format!("{}", a), "-456");
    let a = BigInt::from_int(0);
    assert_eq!(format!("{}", a), "0");

    let a = BigInt::from_str("123456789012345678901234567890");
    assert_eq!(format!("{}", a), "123456789012345678901234567890");
}

#[test]
fn test_factorial_100() {
    // Compute 100! and check exact value.
    let n = BigInt::from_int(100);
    let mut result = BigInt::from_int(1);
    let mut i = BigInt::from_int(1);
    while le(&i, &n) {
        result = result.mul(&i);
        i.inc();
    }
    let expected = BigInt::from_str(
        "93326215443944152681699238856266700490715968264381621468592963895217599993229915608941463976156518286253697920827223758251185210916864000000000000000000000000",
    );
    assert_eq!(result, expected);
}

#[test]
fn test_big_add_sub() {
    let a = BigInt::from_str("123456789012345678901234567890");
    let b = BigInt::from_str("987654321098765432109876543210");
    let s = a.add(&b);
    assert_eq!(s, BigInt::from_str("1111111110111111111011111111100"));

    let d = b.sub(&a);
    assert_eq!(d, BigInt::from_str("864197532086419753208641975320"));
}

#[test]
fn test_delete_noop() {
    // delete is a no-op in Rust; should not panic and the value remains valid.
    let mut a = BigInt::from_int(42);
    bigint_crate::bigint::delete(&mut a);
    // a is still usable in Rust (Rust doesn't have manual free).
    assert_eq!(a.to_int(), 42);
}

fn main() {}
