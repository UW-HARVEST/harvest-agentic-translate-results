use bigint::bigint::*;

fn main() {}

// --- from_int / to_int ---

#[test]
fn test_from_int_zero() {
    let a = BigInt::from_int(0);
    assert_eq!(a.to_int(), 0);
}

#[test]
fn test_from_int_negative() {
    let a = BigInt::from_int(-42);
    assert_eq!(a.to_int(), -42);
}

#[test]
fn test_from_int_large() {
    let a = BigInt::from_int(999999999);
    assert_eq!(a.to_int(), 999999999);
}

// --- from_str ---

#[test]
fn test_from_str_zero() {
    assert_eq!(BigInt::from_str("0").to_int(), 0);
}

#[test]
fn test_from_str_negative() {
    assert_eq!(BigInt::from_str("-12345").to_int(), -12345);
}

#[test]
fn test_from_int_eq_from_str() {
    assert_eq!(BigInt::from_int(100), BigInt::from_str("100"));
}

// --- copy ---

#[test]
fn test_copy() {
    let a = BigInt::from_str("123456789012345678901234567890");
    let b = a.copy();
    assert_eq!(a, b);
}

// --- is_zero, lt_zero, gt_zero, lezero, gezero ---

#[test]
fn test_zero_predicates() {
    let z = BigInt::from_int(0);
    assert!(z.is_zero());
    assert!(!z.lt_zero());
    assert!(!z.gt_zero());
    assert!(z.lezero());
    assert!(z.gezero());
}

#[test]
fn test_positive_predicates() {
    let p = BigInt::from_int(5);
    assert!(!p.is_zero());
    assert!(!p.lt_zero());
    assert!(p.gt_zero());
    assert!(!p.lezero());
    assert!(p.gezero());
}

#[test]
fn test_negative_predicates() {
    let n = BigInt::from_int(-5);
    assert!(!n.is_zero());
    assert!(n.lt_zero());
    assert!(!n.gt_zero());
    assert!(n.lezero());
    assert!(!n.gezero());
}

// --- abs ---

#[test]
fn test_abs() {
    assert_eq!(BigInt::from_int(-42).abs().to_int(), 42);
    assert_eq!(BigInt::from_int(42).abs().to_int(), 42);
}

// --- comparisons (free functions) ---

#[test]
fn test_cmp_equal() {
    let x = BigInt::from_int(100);
    let y = BigInt::from_int(100);
    assert_eq!(x, y);
    assert!(le(&x, &y));
    assert!(ge(&x, &y));
    assert!(!lt(&x, &y));
    assert!(!gt(&x, &y));
}

#[test]
fn test_cmp_less() {
    let x = BigInt::from_int(100);
    let y = BigInt::from_int(101);
    assert_ne!(x, y);
    assert!(le(&x, &y));
    assert!(!ge(&x, &y));
    assert!(lt(&x, &y));
    assert!(!gt(&x, &y));
}

#[test]
fn test_cmp_greater() {
    let x = BigInt::from_int(101);
    let y = BigInt::from_int(100);
    assert_ne!(x, y);
    assert!(!le(&x, &y));
    assert!(ge(&x, &y));
    assert!(!lt(&x, &y));
    assert!(gt(&x, &y));
}

#[test]
fn test_cmp_negative_vs_positive() {
    let a = BigInt::from_int(-5);
    let b = BigInt::from_int(3);
    assert!(!gt(&a, &b));
    assert!(lt(&a, &b));
    assert!(!ge(&a, &b));
    assert!(le(&a, &b));
}

// --- add ---

#[test]
fn test_add_positive() {
    assert_eq!(BigInt::from_int(100).add(&BigInt::from_int(200)).to_int(), 300);
}

#[test]
fn test_add_neg_pos() {
    assert_eq!(BigInt::from_int(-100).add(&BigInt::from_int(200)).to_int(), 100);
}

#[test]
fn test_add_pos_neg() {
    assert_eq!(BigInt::from_int(100).add(&BigInt::from_int(-200)).to_int(), -100);
}

#[test]
fn test_add_both_neg() {
    assert_eq!(BigInt::from_int(-100).add(&BigInt::from_int(-200)).to_int(), -300);
}

#[test]
fn test_add_zeros() {
    assert_eq!(BigInt::from_int(0).add(&BigInt::from_int(0)).to_int(), 0);
}

// --- sub ---

#[test]
fn test_sub_positive() {
    assert_eq!(BigInt::from_int(200).sub(&BigInt::from_int(100)).to_int(), 100);
}

#[test]
fn test_sub_result_negative() {
    assert_eq!(BigInt::from_int(100).sub(&BigInt::from_int(200)).to_int(), -100);
}

#[test]
fn test_sub_both_negative() {
    // C bug: -100 - (-200) = -(100+200) = -300
    assert_eq!(BigInt::from_int(-100).sub(&BigInt::from_int(-200)).to_int(), -300);
}

#[test]
fn test_sub_neg_pos() {
    // -100 - 200 = -300
    assert_eq!(BigInt::from_int(-100).sub(&BigInt::from_int(200)).to_int(), -300);
}

#[test]
fn test_sub_pos_neg() {
    // 100 - (-200) = 300
    assert_eq!(BigInt::from_int(100).sub(&BigInt::from_int(-200)).to_int(), 300);
}

#[test]
fn test_sub_both_neg_small() {
    // -5 - (-3) = -8 in C, -3 - (-5) = -8 in C
    assert_eq!(BigInt::from_int(-5).sub(&BigInt::from_int(-3)).to_int(), -8);
    assert_eq!(BigInt::from_int(-3).sub(&BigInt::from_int(-5)).to_int(), -8);
}

// --- mul ---

#[test]
fn test_mul() {
    assert_eq!(BigInt::from_int(123).mul(&BigInt::from_int(456)).to_int(), 56088);
}

#[test]
fn test_mul_negative() {
    assert_eq!(BigInt::from_int(-123).mul(&BigInt::from_int(456)).to_int(), -56088);
}

#[test]
fn test_mul_zero() {
    assert_eq!(BigInt::from_int(0).mul(&BigInt::from_int(456)).to_int(), 0);
}

// --- div ---

#[test]
fn test_div() {
    assert_eq!(BigInt::from_int(200).div(&BigInt::from_int(100)).to_int(), 2);
}

#[test]
fn test_div_truncate() {
    assert_eq!(BigInt::from_int(7).div(&BigInt::from_int(3)).to_int(), 2);
}

#[test]
fn test_div_negative() {
    assert_eq!(BigInt::from_int(-7).div(&BigInt::from_int(3)).to_int(), -2);
}

// --- mod ---

#[test]
fn test_mod() {
    assert_eq!(BigInt::from_int(7).r#mod(&BigInt::from_int(3)).to_int(), 1);
    assert_eq!(BigInt::from_int(201).r#mod(&BigInt::from_int(100)).to_int(), 1);
    assert_eq!(BigInt::from_int(200).r#mod(&BigInt::from_int(100)).to_int(), 0);
}

// --- divmod ---

#[test]
fn test_divmod() {
    let (q, r) = BigInt::from_int(17).divmod(&BigInt::from_int(5));
    assert_eq!(q.to_int(), 3);
    assert_eq!(r.to_int(), 2);
}

// --- pow ---

#[test]
fn test_pow() {
    assert_eq!(BigInt::from_int(2).pow(&BigInt::from_int(10)).to_int(), 1024);
    assert_eq!(BigInt::from_int(2).pow(&BigInt::from_int(0)).to_int(), 1);
    assert_eq!(BigInt::from_int(5).pow(&BigInt::from_int(3)).to_int(), 125);
}

// --- fast_pow ---

#[test]
fn test_fast_pow() {
    assert_eq!(BigInt::from_int(2).fast_pow(&BigInt::from_int(10), &BigInt::from_int(1000)).to_int(), 24);
    assert_eq!(BigInt::from_int(745232).fast_pow(&BigInt::from_int(67121), &BigInt::from_int(1022117)).to_int(), 97);
    assert_eq!(BigInt::from_int(3).fast_pow(&BigInt::from_int(0), &BigInt::from_int(7)).to_int(), 1);
}

// --- modinv ---

#[test]
fn test_modinv() {
    assert_eq!(BigInt::from_int(17).modinv(&BigInt::from_int(43)).to_int(), 38);
    assert_eq!(BigInt::from_int(3).modinv(&BigInt::from_int(11)).to_int(), 4);
}

// --- sqrt ---

#[test]
fn test_sqrt() {
    assert_eq!(BigInt::from_int(100).sqrt().to_int(), 10);
    assert_eq!(BigInt::from_int(101).sqrt().to_int(), 11);
    assert_eq!(BigInt::from_int(0).sqrt().to_int(), 0);
    assert_eq!(BigInt::from_int(1).sqrt().to_int(), 1);
    assert_eq!(BigInt::from_int(49).sqrt().to_int(), 7);
}

// --- is_even / is_odd ---

#[test]
fn test_even_odd() {
    assert!(BigInt::from_int(4).is_even());
    assert!(!BigInt::from_int(4).is_odd());
    assert!(!BigInt::from_int(7).is_even());
    assert!(BigInt::from_int(7).is_odd());
    assert!(BigInt::from_int(0).is_even());
    assert!(!BigInt::from_int(0).is_odd());
}

// --- is_prime ---

#[test]
fn test_is_prime() {
    assert!(!BigInt::from_int(2).is_prime());  // C: even check catches 2
    assert!(!BigInt::from_int(3).is_prime());  // C: digit sum div by 3
    assert!(!BigInt::from_int(4).is_prime());
    assert!(BigInt::from_int(17).is_prime());
    assert!(!BigInt::from_int(18).is_prime());
    assert!(BigInt::from_int(97).is_prime());
    assert!(!BigInt::from_int(99).is_prime());
}

// --- inc / dec ---

#[test]
fn test_inc_dec() {
    let mut a = BigInt::from_int(5);
    a.inc();
    assert_eq!(a.to_int(), 6);

    let mut b = BigInt::from_int(5);
    b.dec();
    assert_eq!(b.to_int(), 4);

    let mut c = BigInt::from_int(0);
    c.dec();
    assert_eq!(c.to_int(), -1);
}

// --- is_64_bit ---

#[test]
fn test_is_64_bit() {
    assert!(BigInt::from_int(123456789).is_64_bit());
    assert!(!BigInt::from_str("12345678901234567890").is_64_bit());
}

// --- zero ---

#[test]
fn test_zero() {
    let z = BigInt::zero();
    assert!(z.is_zero());
    assert_eq!(z.to_int(), 0);
}

// --- delete ---

#[test]
fn test_delete() {
    let mut a = BigInt::from_int(42);
    delete(&mut a);
}

// --- big number arithmetic ---

#[test]
fn test_big_add() {
    let a = BigInt::from_str("123456789012345678901234567890");
    let b = BigInt::from_str("987654321098765432109876543210");
    let c = a.add(&b);
    assert_eq!(c, BigInt::from_str("1111111110111111111011111111100"));
}

#[test]
fn test_big_sub() {
    let a = BigInt::from_str("123456789012345678901234567890");
    let b = BigInt::from_str("987654321098765432109876543210");
    let c = a.sub(&b);
    assert_eq!(c, BigInt::from_str("-864197532086419753208641975320"));
}

#[test]
fn test_big_mul() {
    let a = BigInt::from_str("123456789012345678901234567890");
    let b = BigInt::from_str("987654321098765432109876543210");
    let c = a.mul(&b);
    assert_eq!(c, BigInt::from_str("121932631137021795226185032733622923332237463801111263526900"));
}

#[test]
fn test_big_div() {
    let a = BigInt::from_str("123456789012345678901234567890");
    let b = BigInt::from_str("987654321098765432109876543210");
    let c = a.div(&b);
    assert_eq!(c, BigInt::from_int(0));
}

#[test]
fn test_big_mod() {
    let a = BigInt::from_str("123456789012345678901234567890");
    let b = BigInt::from_str("987654321098765432109876543210");
    let c = a.r#mod(&b);
    assert_eq!(c, BigInt::from_str("123456789012345678901234567890"));
}

#[test]
fn test_big_sqrt() {
    let a = BigInt::from_str("123412341");
    let c = a.sqrt();
    assert_eq!(c, BigInt::from_int(11110));
}

#[test]
fn test_big_fast_pow() {
    let a = BigInt::from_str("2");
    let b = BigInt::from_str("1234");
    let m = BigInt::from_str("10007");
    let c = a.fast_pow(&b, &m);
    assert_eq!(c, BigInt::from_int(8621));
}

// --- factorial (integration) ---

#[test]
fn test_factorial_100() {
    let mut result = BigInt::from_int(1);
    let mut i = BigInt::from_int(1);
    let n = BigInt::from_int(100);
    while le(&i, &n) {
        result = result.mul(&i);
        i.inc();
    }
    assert_eq!(result, BigInt::from_str("93326215443944152681699238856266700490715968264381621468592963895217599993229915608941463976156518286253697920827223758251185210916864000000000000000000000000"));
}
