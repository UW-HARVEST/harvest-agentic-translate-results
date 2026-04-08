use NandC::nand::*;

#[test]
fn test_nand() {
    assert_eq!(nand(false, false), true);
    assert_eq!(nand(false, true), true);
    assert_eq!(nand(true, false), true);
    assert_eq!(nand(true, true), false);
}

#[test]
fn test_not() {
    assert_eq!(not(false), true);
    assert_eq!(not(true), false);
}

#[test]
fn test_or() {
    assert_eq!(or(false, false), false);
    assert_eq!(or(false, true), true);
    assert_eq!(or(true, false), true);
    assert_eq!(or(true, true), true);
}

#[test]
fn test_and() {
    assert_eq!(and(false, false), false);
    assert_eq!(and(false, true), false);
    assert_eq!(and(true, false), false);
    assert_eq!(and(true, true), true);
}

#[test]
fn test_xor() {
    assert_eq!(xor(false, false), false);
    assert_eq!(xor(false, true), true);
    assert_eq!(xor(true, false), true);
    assert_eq!(xor(true, true), false);
}

#[test]
fn test_add_bit() {
    let cases: [(bool, bool, bool, bool, bool); 8] = [
        (false, false, false, false, false),
        (false, false, true,  true,  false),
        (false, true,  false, true,  false),
        (false, true,  true,  false, true),
        (true,  false, false, true,  false),
        (true,  false, true,  false, true),
        (true,  true,  false, false, true),
        (true,  true,  true,  true,  true),
    ];
    for (a, b, c, exp_bit, exp_carry) in cases {
        let mut carry = false;
        let bit = add_bit(a, b, c, &mut carry);
        assert_eq!(bit, exp_bit, "add_bit({a},{b},{c}) bit");
        assert_eq!(carry, exp_carry, "add_bit({a},{b},{c}) carry");
    }
}

#[test]
fn test_half_sub() {
    let cases: [(bool, bool, bool, bool); 4] = [
        (false, false, false, false),
        (false, true,  true,  true),
        (true,  false, true,  false),
        (true,  true,  false, false),
    ];
    for (a, b, exp_bit, exp_borrow) in cases {
        let mut borrow = false;
        let bit = half_sub(a, b, &mut borrow);
        assert_eq!(bit, exp_bit, "half_sub({a},{b}) bit");
        assert_eq!(borrow, exp_borrow, "half_sub({a},{b}) borrow");
    }
}

#[test]
fn test_sub_bit() {
    let cases: [(bool, bool, bool, bool, bool); 8] = [
        (false, false, false, false, false),
        (false, false, true,  true,  true),
        (false, true,  false, true,  true),
        (false, true,  true,  false, true),
        (true,  false, false, true,  false),
        (true,  false, true,  false, false),
        (true,  true,  false, false, false),
        (true,  true,  true,  true,  true),
    ];
    for (a, b, c, exp_bit, exp_borrow) in cases {
        let mut borrow = false;
        let bit = sub_bit(a, b, c, &mut borrow);
        assert_eq!(bit, exp_bit, "sub_bit({a},{b},{c}) bit");
        assert_eq!(borrow, exp_borrow, "sub_bit({a},{b},{c}) borrow");
    }
}

#[test]
fn test_bll() {
    assert_eq!(bll(false), "false");
    assert_eq!(bll(true), "true");
}

#[test]
fn test_add_u4() {
    assert_eq!(add_u4(0, 0), 0);
    assert_eq!(add_u4(5, 3), 8);
    assert_eq!(add_u4(15, 1), 0);  // wraps
    assert_eq!(add_u4(7, 8), 15);
    assert_eq!(add_u4(15, 15), 14); // wraps
    // exhaustive check against wrapping arithmetic
    for a in 0..=U4_MAX {
        for b in 0..=U4_MAX {
            assert_eq!(add_u4(a, b), (a.wrapping_add(b)) & 0b1111,
                "add_u4({a},{b})");
        }
    }
}

#[test]
fn test_sub_u4() {
    assert_eq!(sub_u4(0, 0), 0);
    assert_eq!(sub_u4(5, 3), 2);
    assert_eq!(sub_u4(3, 5), 14);  // wraps
    assert_eq!(sub_u4(15, 1), 14);
    assert_eq!(sub_u4(0, 1), 15);  // wraps
    // exhaustive check against wrapping arithmetic
    for a in 0..=U4_MAX {
        for b in 0..=U4_MAX {
            assert_eq!(sub_u4(a, b), (a.wrapping_sub(b)) & 0b1111,
                "sub_u4({a},{b})");
        }
    }
}

#[test]
fn test_check_add() {
    assert!(check_add());
}

#[test]
fn test_check_sub() {
    assert!(check_sub());
}

#[test]
fn test_u4_max_constant() {
    assert_eq!(U4_MAX, 15);
}

fn main() {}
