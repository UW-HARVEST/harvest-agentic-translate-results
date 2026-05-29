extern crate bigint as bigint_crate;
use bigint_crate::bigint::{self, BigInt};

// ----- Construction / conversion -----

#[test]
fn test_zero() {
    let z = BigInt::zero();
    assert!(z.is_zero());
    assert_eq!(z.to_int(), 0);
    assert_eq!(format!("{}", z), "0");
    assert_eq!(z, BigInt::from_int(0));
}

#[test]
fn test_from_int_positive() {
    let n = BigInt::from_int(100);
    assert_eq!(n.to_int(), 100);
    assert_eq!(format!("{}", n), "100");
    assert!(!n.is_zero());
}

#[test]
fn test_from_int_negative() {
    let n = BigInt::from_int(-100);
    assert_eq!(n.to_int(), -100);
    assert_eq!(format!("{}", n), "-100");
}

#[test]
fn test_from_int_zero() {
    let n = BigInt::from_int(0);
    assert_eq!(n.to_int(), 0);
    assert_eq!(format!("{}", n), "0");
    assert!(n.is_zero());
}

#[test]
fn test_from_int_one() {
    let n = BigInt::from_int(1);
    assert_eq!(n.to_int(), 1);
    assert_eq!(format!("{}", n), "1");
}

#[test]
fn test_from_int_minus_one() {
    let n = BigInt::from_int(-1);
    assert_eq!(n.to_int(), -1);
    assert_eq!(format!("{}", n), "-1");
}

#[test]
fn test_from_int_99() {
    let n = BigInt::from_int(99);
    assert_eq!(format!("{}", n), "99");
    assert_eq!(n.to_int(), 99);
}

#[test]
fn test_from_int_big_neg() {
    let n = BigInt::from_int(-987654321);
    assert_eq!(format!("{}", n), "-987654321");
    assert_eq!(n.to_int(), -987654321);
}

#[test]
fn test_to_int() {
    assert_eq!(BigInt::from_str("12345").to_int(), 12345);
    assert_eq!(BigInt::from_str("-42").to_int(), -42);
    assert_eq!(BigInt::from_str("0").to_int(), 0);
    assert_eq!(BigInt::from_str("999999999").to_int(), 999999999);
}

#[test]
fn test_from_str_positive() {
    let n = BigInt::from_str("100");
    assert_eq!(format!("{}", n), "100");
    assert_eq!(n.to_int(), 100);
}

#[test]
fn test_from_str_negative() {
    let n = BigInt::from_str("-123");
    assert_eq!(format!("{}", n), "-123");
    assert_eq!(n.to_int(), -123);
}

#[test]
fn test_from_str_zero() {
    let n = BigInt::from_str("0");
    assert!(n.is_zero());
    assert_eq!(format!("{}", n), "0");
}

#[test]
fn test_from_str_leading_zeros_preserve_format() {
    // C bigint_from_string does not strip leading zeros; bigint_print prints them all.
    let n = BigInt::from_str("00123");
    assert_eq!(format!("{}", n), "00123");
    // But equality strips leading zeros:
    assert_eq!(n, BigInt::from_str("123"));
}

#[test]
fn test_copy_equals_original() {
    let a = BigInt::from_str("123456789012345678901234567890");
    let b = a.copy();
    assert_eq!(a, b);
    assert_eq!(format!("{}", a), format!("{}", b));
}

// ----- is_64_bit -----

#[test]
fn test_is_64_bit_size_lt_10() {
    // sizes < 10 => true
    assert!(BigInt::from_str("0").is_64_bit());
    assert!(BigInt::from_str("1").is_64_bit());
    assert!(BigInt::from_str("12345").is_64_bit());
    assert!(BigInt::from_str("999999999").is_64_bit()); // 9 digits
    assert!(BigInt::from_str("-999999999").is_64_bit());
}

#[test]
fn test_is_64_bit_size_eq_10() {
    // size == 10 => false (matches C)
    assert!(!BigInt::from_str("1234567890").is_64_bit());
    assert!(!BigInt::from_str("9999999999").is_64_bit());
}

#[test]
fn test_is_64_bit_size_gt_10() {
    assert!(!BigInt::from_str("12345678901").is_64_bit());
    assert!(!BigInt::from_str("99999999999999").is_64_bit());
}

// ----- remove_leading_zeros -----

#[test]
fn test_remove_leading_zeros_basic() {
    let mut n = BigInt::from_str("00123");
    n.remove_leading_zeros();
    assert_eq!(format!("{}", n), "123");
    assert_eq!(n.to_int(), 123);
}

#[test]
fn test_remove_leading_zeros_all_zeros() {
    let mut n = BigInt::from_str("0000");
    n.remove_leading_zeros();
    assert_eq!(format!("{}", n), "0");
    assert!(n.is_zero());
}

#[test]
fn test_remove_leading_zeros_no_change() {
    let mut n = BigInt::from_str("12345");
    n.remove_leading_zeros();
    assert_eq!(format!("{}", n), "12345");
}

// ----- is_zero / sign predicates -----

#[test]
fn test_is_zero_predicates() {
    let z = BigInt::from_str("0");
    let zz = BigInt::from_str("000");
    let pos = BigInt::from_str("5");
    let neg = BigInt::from_str("-5");

    assert!(z.is_zero());
    assert!(zz.is_zero());
    assert!(!pos.is_zero());
    assert!(!neg.is_zero());
}

#[test]
fn test_lt_zero() {
    assert!(BigInt::from_int(-5).lt_zero());
    assert!(!BigInt::from_int(5).lt_zero());
    assert!(!BigInt::from_int(0).lt_zero());
}

#[test]
fn test_gt_zero() {
    assert!(!BigInt::from_int(-5).gt_zero());
    assert!(BigInt::from_int(5).gt_zero());
    assert!(!BigInt::from_int(0).gt_zero());
}

#[test]
fn test_le_zero() {
    assert!(BigInt::from_int(0).lezero());
    assert!(BigInt::from_int(-5).lezero());
    assert!(!BigInt::from_int(5).lezero());
}

#[test]
fn test_ge_zero() {
    assert!(BigInt::from_int(0).gezero());
    assert!(!BigInt::from_int(-5).gezero());
    assert!(BigInt::from_int(5).gezero());
}

// ----- abs -----

#[test]
fn test_abs() {
    assert_eq!(BigInt::from_int(-123).abs(), BigInt::from_int(123));
    assert_eq!(BigInt::from_int(123).abs(), BigInt::from_int(123));
    assert_eq!(BigInt::from_int(0).abs(), BigInt::from_int(0));
}

// ----- comparisons -----

#[test]
fn test_eq() {
    assert_eq!(BigInt::from_int(100), BigInt::from_int(100));
    assert_ne!(BigInt::from_int(100), BigInt::from_int(200));
    assert_ne!(BigInt::from_int(-100), BigInt::from_int(100));
    // Leading zeros must compare equal.
    assert_eq!(BigInt::from_str("0123"), BigInt::from_str("123"));
}

#[test]
fn test_gt() {
    assert!(bigint::gt(&BigInt::from_int(200), &BigInt::from_int(100)));
    assert!(!bigint::gt(&BigInt::from_int(100), &BigInt::from_int(200)));
    assert!(!bigint::gt(&BigInt::from_int(100), &BigInt::from_int(100)));
    assert!(!bigint::gt(&BigInt::from_int(-200), &BigInt::from_int(-100)));
    assert!(bigint::gt(&BigInt::from_int(-3), &BigInt::from_int(-5)));
    assert!(!bigint::gt(&BigInt::from_int(-5), &BigInt::from_int(5)));
}

#[test]
fn test_lt() {
    assert!(bigint::lt(&BigInt::from_int(100), &BigInt::from_int(200)));
    assert!(!bigint::lt(&BigInt::from_int(200), &BigInt::from_int(100)));
    assert!(!bigint::lt(&BigInt::from_int(100), &BigInt::from_int(100)));
    assert!(bigint::lt(&BigInt::from_int(-5), &BigInt::from_int(5)));
    assert!(bigint::lt(&BigInt::from_int(-5), &BigInt::from_int(-3)));
}

#[test]
fn test_ge() {
    assert!(bigint::ge(&BigInt::from_int(100), &BigInt::from_int(100)));
    assert!(bigint::ge(&BigInt::from_int(200), &BigInt::from_int(100)));
    assert!(!bigint::ge(&BigInt::from_int(100), &BigInt::from_int(200)));
}

#[test]
fn test_le() {
    assert!(bigint::le(&BigInt::from_int(100), &BigInt::from_int(100)));
    assert!(bigint::le(&BigInt::from_int(100), &BigInt::from_int(200)));
    assert!(!bigint::le(&BigInt::from_int(200), &BigInt::from_int(100)));
}

#[test]
fn test_compare_with_leading_zeros() {
    let a = BigInt::from_str("00100");
    let b = BigInt::from_str("100");
    assert_eq!(a, b);
    assert!(!bigint::gt(&a, &b));
    assert!(!bigint::lt(&a, &b));
    assert!(bigint::le(&a, &b));
    assert!(bigint::ge(&a, &b));
}

// ----- add -----

#[test]
fn test_add_pos_pos() {
    let a = BigInt::from_int(100);
    let b = BigInt::from_int(200);
    assert_eq!(a.add(&b), BigInt::from_int(300));
}

#[test]
fn test_add_neg_pos() {
    let a = BigInt::from_int(-100);
    let b = BigInt::from_int(200);
    assert_eq!(a.add(&b), BigInt::from_int(100));
}

#[test]
fn test_add_pos_neg() {
    let a = BigInt::from_int(100);
    let b = BigInt::from_int(-200);
    assert_eq!(a.add(&b), BigInt::from_int(-100));
}

#[test]
fn test_add_neg_neg() {
    let a = BigInt::from_int(-100);
    let b = BigInt::from_int(-200);
    assert_eq!(a.add(&b), BigInt::from_int(-300));
}

#[test]
fn test_add_carry() {
    // 999 + 1 = 1000
    let a = BigInt::from_int(999);
    let b = BigInt::from_int(1);
    let r = a.add(&b);
    assert_eq!(r, BigInt::from_int(1000));
    assert_eq!(format!("{}", r), "1000");
}

#[test]
fn test_add_zero() {
    let a = BigInt::from_int(123);
    let b = BigInt::from_int(0);
    assert_eq!(a.add(&b), BigInt::from_int(123));
    assert_eq!(b.add(&a), BigInt::from_int(123));
}

#[test]
fn test_add_big() {
    let a = BigInt::from_str("123456789012345678901234567890");
    let b = BigInt::from_str("987654321098765432109876543210");
    let r = a.add(&b);
    assert_eq!(format!("{}", r), "1111111110111111111011111111100");
}

// ----- sub -----

#[test]
fn test_sub_simple() {
    let a = BigInt::from_int(200);
    let b = BigInt::from_int(100);
    assert_eq!(a.sub(&b), BigInt::from_int(100));
}

#[test]
fn test_sub_negative_result() {
    let a = BigInt::from_int(100);
    let b = BigInt::from_int(200);
    assert_eq!(a.sub(&b), BigInt::from_int(-100));
}

#[test]
fn test_sub_neg_neg() {
    // -100 - (-200) = 100
    let a = BigInt::from_int(-100);
    let b = BigInt::from_int(-200);
    // C: bigint_sub: both neg => add(b,a) negated => -(200+100) = -300
    // But mathematically -100 - (-200) = 100. The C does *not* implement the
    // mathematical semantics — it does both-negative => -(b+a). Trust C.
    assert_eq!(a.sub(&b), BigInt::from_int(-300));
}

#[test]
fn test_sub_equal() {
    let a = BigInt::from_int(100);
    let b = BigInt::from_int(100);
    assert_eq!(a.sub(&b), BigInt::from_int(0));
}

#[test]
fn test_sub_with_borrow() {
    // 1000 - 1 = 999
    let a = BigInt::from_int(1000);
    let b = BigInt::from_int(1);
    let r = a.sub(&b);
    assert_eq!(r, BigInt::from_int(999));
}

#[test]
fn test_sub_big() {
    let a = BigInt::from_str("123456789012345678901234567890");
    let b = BigInt::from_str("987654321098765432109876543210");
    let r = a.sub(&b);
    assert_eq!(format!("{}", r), "-864197532086419753208641975320");
}

// ----- mul -----

#[test]
fn test_mul_basic() {
    let a = BigInt::from_int(12);
    let b = BigInt::from_int(13);
    assert_eq!(a.mul(&b), BigInt::from_int(156));
}

#[test]
fn test_mul_neg() {
    let a = BigInt::from_int(-12);
    let b = BigInt::from_int(13);
    assert_eq!(a.mul(&b), BigInt::from_int(-156));
}

#[test]
fn test_mul_zero() {
    let a = BigInt::from_int(0);
    let b = BigInt::from_int(5);
    assert_eq!(a.mul(&b), BigInt::from_int(0));
}

#[test]
fn test_mul_powers() {
    let a = BigInt::from_int(100);
    let b = BigInt::from_int(200);
    assert_eq!(a.mul(&b), BigInt::from_int(20000));
}

#[test]
fn test_mul_big() {
    let a = BigInt::from_str("12345678901234567890");
    let b = BigInt::from_str("98765432109876543210");
    let r = a.mul(&b);
    assert_eq!(format!("{}", r), "1219326311370217952237463801111263526900");
}

// ----- div / mod / divmod -----

#[test]
fn test_div_basic() {
    assert_eq!(BigInt::from_int(200).div(&BigInt::from_int(100)),
               BigInt::from_int(2));
    assert_eq!(BigInt::from_int(200).div(&BigInt::from_int(6)),
               BigInt::from_int(33));
    assert_eq!(BigInt::from_int(7).div(&BigInt::from_int(3)),
               BigInt::from_int(2));
    assert_eq!(BigInt::from_int(1234).div(&BigInt::from_int(5)),
               BigInt::from_int(246));
}

#[test]
fn test_div_negative() {
    assert_eq!(BigInt::from_int(-200).div(&BigInt::from_int(6)),
               BigInt::from_int(-33));
    assert_eq!(BigInt::from_int(-200).div(&BigInt::from_int(-100)),
               BigInt::from_int(2));
}

#[test]
fn test_div_truncation_negative() {
    // C: -7 / 3 = -2 (truncation toward zero)
    assert_eq!(BigInt::from_int(-7).div(&BigInt::from_int(3)),
               BigInt::from_int(-2));
    assert_eq!(BigInt::from_int(7).div(&BigInt::from_int(-3)),
               BigInt::from_int(-2));
}

#[test]
fn test_mod_basic() {
    assert_eq!(BigInt::from_int(200).r#mod(&BigInt::from_int(100)),
               BigInt::from_int(0));
    assert_eq!(BigInt::from_int(201).r#mod(&BigInt::from_int(100)),
               BigInt::from_int(1));
    assert_eq!(BigInt::from_int(7).r#mod(&BigInt::from_int(3)),
               BigInt::from_int(1));
    assert_eq!(BigInt::from_int(200).r#mod(&BigInt::from_int(7)),
               BigInt::from_int(4));
    assert_eq!(BigInt::from_int(1234).r#mod(&BigInt::from_int(5)),
               BigInt::from_int(4));
}

#[test]
fn test_mod_negative() {
    // C: -7 % 3 = -1 (truncating)
    assert_eq!(BigInt::from_int(-7).r#mod(&BigInt::from_int(3)),
               BigInt::from_int(-1));
    // C: 7 % -3 = 1
    assert_eq!(BigInt::from_int(7).r#mod(&BigInt::from_int(-3)),
               BigInt::from_int(1));
}

#[test]
fn test_divmod_basic() {
    let (q, r) = BigInt::from_int(7).divmod(&BigInt::from_int(3));
    assert_eq!(q, BigInt::from_int(2));
    assert_eq!(r, BigInt::from_int(1));
}

#[test]
fn test_divmod_exact() {
    let (q, r) = BigInt::from_int(200).divmod(&BigInt::from_int(100));
    assert_eq!(q, BigInt::from_int(2));
    assert_eq!(r, BigInt::from_int(0));
}

// ----- inc / dec -----

#[test]
fn test_inc() {
    let mut n = BigInt::from_int(99);
    n.inc();
    assert_eq!(n, BigInt::from_int(100));
}

#[test]
fn test_inc_from_neg() {
    let mut n = BigInt::from_int(-1);
    n.inc();
    assert_eq!(n, BigInt::from_int(0));
}

#[test]
fn test_dec() {
    let mut n = BigInt::from_int(100);
    n.dec();
    assert_eq!(n, BigInt::from_int(99));
}

#[test]
fn test_dec_from_zero() {
    let mut n = BigInt::from_int(0);
    n.dec();
    assert_eq!(n, BigInt::from_int(-1));
}

// ----- pow -----

#[test]
fn test_pow_basic() {
    let r = BigInt::from_int(2).pow(&BigInt::from_int(10));
    assert_eq!(r, BigInt::from_int(1024));
}

#[test]
fn test_pow_zero_exp() {
    let r = BigInt::from_int(5).pow(&BigInt::from_int(0));
    assert_eq!(r, BigInt::from_int(1));
}

#[test]
fn test_pow_neg_exp_returns_zero() {
    // C: bigint_pow returns "0" if exponent is negative
    let r = BigInt::from_int(2).pow(&BigInt::from_int(-3));
    assert_eq!(r, BigInt::from_int(0));
}

#[test]
fn test_pow_three_four() {
    let r = BigInt::from_int(3).pow(&BigInt::from_int(4));
    assert_eq!(r, BigInt::from_int(81));
}

#[test]
fn test_pow_two_twenty() {
    let r = BigInt::from_int(2).pow(&BigInt::from_int(20));
    assert_eq!(r, BigInt::from_int(1048576));
}

// ----- fast_pow -----

#[test]
fn test_fast_pow_known_value() {
    let r = BigInt::from_int(745232).fast_pow(
        &BigInt::from_int(67121),
        &BigInt::from_int(1022117),
    );
    assert_eq!(r, BigInt::from_int(97));
}

#[test]
fn test_fast_pow_simple() {
    // 2^10 mod 1000 = 1024 mod 1000 = 24
    let r = BigInt::from_int(2).fast_pow(
        &BigInt::from_int(10),
        &BigInt::from_int(1000),
    );
    assert_eq!(r, BigInt::from_int(24));
}

#[test]
fn test_fast_pow_test3_value() {
    // 2^1234 mod 10007 = 8621
    let r = BigInt::from_int(2).fast_pow(
        &BigInt::from_int(1234),
        &BigInt::from_int(10007),
    );
    assert_eq!(r, BigInt::from_int(8621));
}

#[test]
fn test_fast_pow_seven_256_13() {
    // 7^256 mod 13 = 9
    let r = BigInt::from_int(7).fast_pow(
        &BigInt::from_int(256),
        &BigInt::from_int(13),
    );
    assert_eq!(r, BigInt::from_int(9));
}

#[test]
fn test_fast_pow_neg_exp() {
    // C: returns 0 if exponent negative.  The 64-bit fast path doesn't
    // hit this, so use values forcing slow path.  Here all values are 64-bit
    // so the fast path runs and the negative exponent branch isn't hit;
    // do a value where slow-path is taken: use big modulus so is_64_bit fails.
    let r = BigInt::from_str("123").fast_pow(
        &BigInt::from_str("-1"),
        &BigInt::from_str("99999999999"), // 11 digits, not 64-bit
    );
    assert_eq!(r, BigInt::from_int(0));
}

#[test]
fn test_fast_pow_zero_exp() {
    // a^0 mod m = 1 (for any positive m)
    let r = BigInt::from_str("123").fast_pow(
        &BigInt::from_str("0"),
        &BigInt::from_str("99999999999"),
    );
    assert_eq!(r, BigInt::from_int(1));
}

// ----- modinv -----

#[test]
fn test_modinv_known() {
    // modinv(17, 43) = 38
    let r = BigInt::from_int(17).modinv(&BigInt::from_int(43));
    assert_eq!(r, BigInt::from_int(38));
    // verify (a * inv) mod m == 1
    let prod = BigInt::from_int(17).mul(&r).r#mod(&BigInt::from_int(43));
    assert_eq!(prod, BigInt::from_int(1));
}

#[test]
fn test_modinv_3_11() {
    let r = BigInt::from_int(3).modinv(&BigInt::from_int(11));
    assert_eq!(r, BigInt::from_int(4));
}

#[test]
fn test_modinv_7_26() {
    let r = BigInt::from_int(7).modinv(&BigInt::from_int(26));
    assert_eq!(r, BigInt::from_int(15));
}

#[test]
fn test_modinv_5_11() {
    let r = BigInt::from_int(5).modinv(&BigInt::from_int(11));
    assert_eq!(r, BigInt::from_int(9));
}

#[test]
fn test_modinv_1_5() {
    let r = BigInt::from_int(1).modinv(&BigInt::from_int(5));
    assert_eq!(r, BigInt::from_int(1));
}

#[test]
fn test_modinv_not_coprime() {
    // gcd(4, 8) != 1 so no inverse; C returns 0
    let r = BigInt::from_int(4).modinv(&BigInt::from_int(8));
    assert_eq!(r, BigInt::from_int(0));
}

// ----- sqrt -----

#[test]
fn test_sqrt_perfect_square() {
    assert_eq!(BigInt::from_int(100).sqrt(), BigInt::from_int(10));
    assert_eq!(BigInt::from_int(144).sqrt(), BigInt::from_int(12));
}

#[test]
fn test_sqrt_non_square() {
    // C returns ceil(sqrt) for non-squares due to its binary search
    assert_eq!(BigInt::from_int(101).sqrt(), BigInt::from_int(11));
}

#[test]
fn test_sqrt_zero_one() {
    assert_eq!(BigInt::from_int(0).sqrt(), BigInt::from_int(0));
    assert_eq!(BigInt::from_int(1).sqrt(), BigInt::from_int(1));
}

#[test]
fn test_sqrt_known() {
    assert_eq!(BigInt::from_int(123412341).sqrt(), BigInt::from_int(11110));
}

// ----- is_even / is_odd -----

#[test]
fn test_is_even() {
    assert!(BigInt::from_int(0).is_even());
    assert!(BigInt::from_int(2).is_even());
    assert!(BigInt::from_int(4).is_even());
    assert!(BigInt::from_int(100).is_even());
    assert!(!BigInt::from_int(1).is_even());
    assert!(!BigInt::from_int(5).is_even());
}

#[test]
fn test_is_odd() {
    assert!(!BigInt::from_int(0).is_odd());
    assert!(!BigInt::from_int(4).is_odd());
    assert!(BigInt::from_int(1).is_odd());
    assert!(BigInt::from_int(5).is_odd());
}

// ----- is_prime -----

#[test]
fn test_is_prime_true() {
    assert!(BigInt::from_int(17).is_prime());
    assert!(BigInt::from_int(7).is_prime());
    assert!(BigInt::from_int(999983).is_prime());
}

#[test]
fn test_is_prime_false_composite() {
    assert!(!BigInt::from_int(18).is_prime());
    assert!(!BigInt::from_int(25).is_prime());
    assert!(!BigInt::from_int(121).is_prime());
    assert!(!BigInt::from_int(100).is_prime());
}

#[test]
fn test_is_prime_special_cases() {
    // The C bigint_is_prime returns false for 2 (since 2 is even)
    assert!(!BigInt::from_int(2).is_prime());
    // Returns false for 3 (sum of digits divisible by 3)
    assert!(!BigInt::from_int(3).is_prime());
}

// ----- Negation -----

#[test]
fn test_neg() {
    let a = BigInt::from_int(123);
    let b = -a.clone();
    assert_eq!(b, BigInt::from_int(-123));
    let c = -b;
    assert_eq!(c, BigInt::from_int(123));
}

// ----- delete -----

#[test]
fn test_delete() {
    let mut a = BigInt::from_int(123);
    bigint::delete(&mut a);
    // After delete, the digits buffer is cleared.  When converted to int,
    // the value is zero (sum over empty digit list).
    assert_eq!(a.to_int(), 0);
}

// ----- factorial via repeated mul (matches test1.c) -----

#[allow(dead_code)]
fn factorial(n: i64) -> BigInt {
    let mut result = BigInt::from_int(1);
    let mut i = BigInt::from_int(1);
    let target = BigInt::from_int(n);
    while bigint::le(&i, &target) {
        result = result.mul(&i);
        i.inc();
    }
    result
}

#[test]
fn test_factorial_small() {
    assert_eq!(factorial(5), BigInt::from_int(120));
    assert_eq!(factorial(10), BigInt::from_int(3628800));
    assert_eq!(factorial(12), BigInt::from_int(479001600));
}

#[test]
fn test_factorial_15() {
    assert_eq!(format!("{}", factorial(15)), "1307674368000");
}

#[test]
fn test_factorial_100() {
    let r = factorial(100);
    assert_eq!(
        format!("{}", r),
        "93326215443944152681699238856266700490715968264381621468592963895217599993229915608941463976156518286253697920827223758251185210916864000000000000000000000000"
    );
}

// ----- print (only checks it doesn't panic) -----

#[test]
fn test_print_no_panic() {
    let n = BigInt::from_str("12345");
    n.print();
    let n2 = BigInt::from_str("-12345");
    n2.print();
}

fn main() {}
