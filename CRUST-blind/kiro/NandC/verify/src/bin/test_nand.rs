use NandC::nand as m;

#[test]
fn test_nand() {
    assert_eq!(m::nand(false, false), true);
    assert_eq!(m::nand(false, true), true);
    assert_eq!(m::nand(true, false), true);
    assert_eq!(m::nand(true, true), false);
}

#[test]
fn test_not() {
    assert_eq!(m::not(false), true);
    assert_eq!(m::not(true), false);
}

#[test]
fn test_or() {
    assert_eq!(m::or(false, false), false);
    assert_eq!(m::or(false, true), true);
    assert_eq!(m::or(true, false), true);
    assert_eq!(m::or(true, true), true);
}

#[test]
fn test_and() {
    assert_eq!(m::and(false, false), false);
    assert_eq!(m::and(false, true), false);
    assert_eq!(m::and(true, false), false);
    assert_eq!(m::and(true, true), true);
}

#[test]
fn test_xor() {
    assert_eq!(m::xor(false, false), false);
    assert_eq!(m::xor(false, true), true);
    assert_eq!(m::xor(true, false), true);
    assert_eq!(m::xor(true, true), false);
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
        let result = m::add_bit(a, b, c, &mut carry);
        assert_eq!(result, exp_bit, "add_bit({a},{b},{c}) bit");
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
        let result = m::half_sub(a, b, &mut borrow);
        assert_eq!(result, exp_bit, "half_sub({a},{b}) bit");
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
        let result = m::sub_bit(a, b, c, &mut borrow);
        assert_eq!(result, exp_bit, "sub_bit({a},{b},{c}) bit");
        assert_eq!(borrow, exp_borrow, "sub_bit({a},{b},{c}) borrow");
    }
}

#[test]
fn test_add_u4() {
    assert_eq!(m::add_u4(0, 0), 0);
    assert_eq!(m::add_u4(1, 1), 2);
    assert_eq!(m::add_u4(3, 4), 7);
    assert_eq!(m::add_u4(7, 8), 15);
    assert_eq!(m::add_u4(15, 15), 14);
    assert_eq!(m::add_u4(0, 15), 15);
    assert_eq!(m::add_u4(5, 10), 15);
    assert_eq!(m::add_u4(6, 7), 13);
    assert_eq!(m::add_u4(15, 1), 0);
    assert_eq!(m::add_u4(8, 8), 0);
}

#[test]
fn test_sub_u4() {
    assert_eq!(m::sub_u4(0, 0), 0);
    assert_eq!(m::sub_u4(1, 1), 0);
    assert_eq!(m::sub_u4(5, 3), 2);
    assert_eq!(m::sub_u4(0, 1), 15);
    assert_eq!(m::sub_u4(15, 15), 0);
    assert_eq!(m::sub_u4(15, 0), 15);
    assert_eq!(m::sub_u4(8, 9), 15);
    assert_eq!(m::sub_u4(3, 7), 12);
    assert_eq!(m::sub_u4(10, 5), 5);
    assert_eq!(m::sub_u4(0, 15), 1);
}

#[test]
fn test_bll() {
    assert_eq!(m::bll(true), "true");
    assert_eq!(m::bll(false), "false");
}

#[test]
fn test_check_add() {
    assert_eq!(m::check_add(), true);
}

#[test]
fn test_check_sub() {
    assert_eq!(m::check_sub(), true);
}

#[test]
fn test_u4_max_constant() {
    assert_eq!(m::U4_MAX, 0b1111);
}

fn main() {}
