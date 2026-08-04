use NandC::nand;

#[test]
fn test_nand_truth_table() {
    assert_eq!(nand::nand(false, false), true);
    assert_eq!(nand::nand(false, true), true);
    assert_eq!(nand::nand(true, false), true);
    assert_eq!(nand::nand(true, true), false);
}

#[test]
fn test_not() {
    assert_eq!(nand::not(false), true);
    assert_eq!(nand::not(true), false);
}

#[test]
fn test_or() {
    assert_eq!(nand::or(false, false), false);
    assert_eq!(nand::or(false, true), true);
    assert_eq!(nand::or(true, false), true);
    assert_eq!(nand::or(true, true), true);
}

#[test]
fn test_and() {
    assert_eq!(nand::and(false, false), false);
    assert_eq!(nand::and(false, true), false);
    assert_eq!(nand::and(true, false), false);
    assert_eq!(nand::and(true, true), true);
}

#[test]
fn test_xor() {
    assert_eq!(nand::xor(false, false), false);
    assert_eq!(nand::xor(false, true), true);
    assert_eq!(nand::xor(true, false), true);
    assert_eq!(nand::xor(true, true), false);
}

#[test]
fn test_add_bit() {
    // Expected from running C: (a, b, carry) -> (bit, carry_out)
    let cases: [(bool, bool, bool, bool, bool); 8] = [
        (false, false, false, false, false),
        (false, false, true, true, false),
        (false, true, false, true, false),
        (false, true, true, false, true),
        (true, false, false, true, false),
        (true, false, true, false, true),
        (true, true, false, false, true),
        (true, true, true, true, true),
    ];
    for (a, b, c, expected_bit, expected_carry) in cases.iter() {
        let mut carry_out = false;
        let bit = nand::add_bit(*a, *b, *c, &mut carry_out);
        assert_eq!(bit, *expected_bit, "add_bit({},{},{}) bit", a, b, c);
        assert_eq!(carry_out, *expected_carry, "add_bit({},{},{}) carry", a, b, c);
    }
}

#[test]
fn test_half_sub() {
    let cases: [(bool, bool, bool, bool); 4] = [
        (false, false, false, false),
        (false, true, true, true),
        (true, false, true, false),
        (true, true, false, false),
    ];
    for (a, b, expected_bit, expected_carry) in cases.iter() {
        let mut carry_out = false;
        let bit = nand::half_sub(*a, *b, &mut carry_out);
        assert_eq!(bit, *expected_bit, "half_sub({},{}) bit", a, b);
        assert_eq!(carry_out, *expected_carry, "half_sub({},{}) carry", a, b);
    }
}

#[test]
fn test_sub_bit() {
    let cases: [(bool, bool, bool, bool, bool); 8] = [
        (false, false, false, false, false),
        (false, false, true, true, true),
        (false, true, false, true, true),
        (false, true, true, false, true),
        (true, false, false, true, false),
        (true, false, true, false, false),
        (true, true, false, false, false),
        (true, true, true, true, true),
    ];
    for (a, b, c, expected_bit, expected_carry) in cases.iter() {
        let mut carry_out = false;
        let bit = nand::sub_bit(*a, *b, *c, &mut carry_out);
        assert_eq!(bit, *expected_bit, "sub_bit({},{},{}) bit", a, b, c);
        assert_eq!(carry_out, *expected_carry, "sub_bit({},{},{}) carry", a, b, c);
    }
}

#[test]
fn test_bll() {
    assert_eq!(nand::bll(true), "true");
    assert_eq!(nand::bll(false), "false");
}

#[test]
fn test_print_add_bit_does_not_panic() {
    // Just make sure it doesn't panic.
    nand::print_add_bit(true, false, true);
    nand::print_add_bit(false, false, false);
}

#[test]
fn test_constants_and_types() {
    assert_eq!(nand::U4_MAX, 0b1111);
    let v: nand::U4 = 5;
    assert_eq!(v, 5u8);
}

#[test]
fn test_add_u4_all_pairs() {
    // verify against (a+b) & 0b1111 (matches C ground truth from test.c)
    for a in 0u8..=15 {
        for b in 0u8..=15 {
            let expected = (a.wrapping_add(b)) & 0b1111;
            assert_eq!(nand::add_u4(a, b), expected, "add_u4({},{})", a, b);
        }
    }
}

#[test]
fn test_add_u4_specific() {
    // Ground truth selected pairs
    assert_eq!(nand::add_u4(0, 0), 0);
    assert_eq!(nand::add_u4(0, 15), 15);
    assert_eq!(nand::add_u4(1, 15), 0);  // overflow wraps
    assert_eq!(nand::add_u4(7, 8), 15);
    assert_eq!(nand::add_u4(8, 8), 0);
    assert_eq!(nand::add_u4(15, 15), 14);
    assert_eq!(nand::add_u4(5, 11), 0);
    assert_eq!(nand::add_u4(3, 5), 8);
}

#[test]
fn test_sub_u4_all_pairs() {
    for a in 0u8..=15 {
        for b in 0u8..=15 {
            let expected = (a.wrapping_sub(b)) & 0b1111;
            assert_eq!(nand::sub_u4(a, b), expected, "sub_u4({},{})", a, b);
        }
    }
}

#[test]
fn test_sub_u4_specific() {
    assert_eq!(nand::sub_u4(0, 0), 0);
    assert_eq!(nand::sub_u4(15, 15), 0);
    assert_eq!(nand::sub_u4(15, 0), 15);
    assert_eq!(nand::sub_u4(0, 1), 15);  // underflow wraps
    assert_eq!(nand::sub_u4(8, 3), 5);
    assert_eq!(nand::sub_u4(0, 15), 1);
    assert_eq!(nand::sub_u4(5, 7), 14);
}

#[test]
fn test_check_add() {
    assert_eq!(nand::check_add(), true);
}

#[test]
fn test_check_sub() {
    assert_eq!(nand::check_sub(), true);
}

fn main() {}
