#[allow(unused_imports)]
use NandC::nand;

#[test]
fn test_constants_and_types() {
    assert_eq!(nand::U4_MAX, 0b1111);
    assert_eq!(nand::U4_MAX, 15u8);
    let _x: nand::U4 = 0u8;
}

#[test]
fn test_nand_truth_table() {
    assert_eq!(nand::nand(false, false), true);
    assert_eq!(nand::nand(false, true), true);
    assert_eq!(nand::nand(true, false), true);
    assert_eq!(nand::nand(true, true), false);
}

#[test]
fn test_not_truth_table() {
    assert_eq!(nand::not(false), true);
    assert_eq!(nand::not(true), false);
}

#[test]
fn test_or_truth_table() {
    assert_eq!(nand::or(false, false), false);
    assert_eq!(nand::or(false, true), true);
    assert_eq!(nand::or(true, false), true);
    assert_eq!(nand::or(true, true), true);
}

#[test]
fn test_and_truth_table() {
    assert_eq!(nand::and(false, false), false);
    assert_eq!(nand::and(false, true), false);
    assert_eq!(nand::and(true, false), false);
    assert_eq!(nand::and(true, true), true);
}

#[test]
fn test_xor_truth_table() {
    assert_eq!(nand::xor(false, false), false);
    assert_eq!(nand::xor(false, true), true);
    assert_eq!(nand::xor(true, false), true);
    assert_eq!(nand::xor(true, true), false);
}

#[test]
fn test_add_bit_all_combinations() {
    // Format: (a, b, carry) -> (bit, out_carry)
    let cases: [((bool, bool, bool), (bool, bool)); 8] = [
        ((false, false, false), (false, false)),
        ((false, false, true),  (true,  false)),
        ((false, true,  false), (true,  false)),
        ((false, true,  true),  (false, true)),
        ((true,  false, false), (true,  false)),
        ((true,  false, true),  (false, true)),
        ((true,  true,  false), (false, true)),
        ((true,  true,  true),  (true,  true)),
    ];
    for ((a, b, c), (exp_bit, exp_carry)) in cases.iter().copied() {
        let mut carry_out = false;
        let bit = nand::add_bit(a, b, c, &mut carry_out);
        assert_eq!(bit, exp_bit, "add_bit bit for ({},{},{})", a, b, c);
        assert_eq!(carry_out, exp_carry, "add_bit carry for ({},{},{})", a, b, c);
    }
}

#[test]
fn test_half_sub_all_combinations() {
    // (a, b) -> (bit, carry)
    let cases: [((bool, bool), (bool, bool)); 4] = [
        ((false, false), (false, false)),
        ((false, true),  (true,  true)),
        ((true,  false), (true,  false)),
        ((true,  true),  (false, false)),
    ];
    for ((a, b), (exp_bit, exp_carry)) in cases.iter().copied() {
        let mut carry_out = true; // intentionally non-default
        let bit = nand::half_sub(a, b, &mut carry_out);
        assert_eq!(bit, exp_bit, "half_sub bit for ({},{})", a, b);
        assert_eq!(carry_out, exp_carry, "half_sub carry for ({},{})", a, b);
    }
}

#[test]
fn test_sub_bit_all_combinations() {
    // (a, b, carry) -> (bit, carry_out) - matches C ground truth output
    let cases: [((bool, bool, bool), (bool, bool)); 8] = [
        ((false, false, false), (false, false)),
        ((false, false, true),  (true,  true)),
        ((false, true,  false), (true,  true)),
        ((false, true,  true),  (false, true)),
        ((true,  false, false), (true,  false)),
        ((true,  false, true),  (false, false)),
        ((true,  true,  false), (false, false)),
        ((true,  true,  true),  (true,  true)),
    ];
    for ((a, b, c), (exp_bit, exp_carry)) in cases.iter().copied() {
        let mut carry_out = false;
        let bit = nand::sub_bit(a, b, c, &mut carry_out);
        assert_eq!(bit, exp_bit, "sub_bit bit for ({},{},{})", a, b, c);
        assert_eq!(carry_out, exp_carry, "sub_bit carry for ({},{},{})", a, b, c);
    }
}

#[test]
fn test_bll() {
    assert_eq!(nand::bll(true), "true");
    assert_eq!(nand::bll(false), "false");
}

#[test]
fn test_add_u4_all_pairs() {
    // Verify that add_u4 matches (a + b) & 0xF for every nibble pair.
    for a in 0u8..=15u8 {
        for b in 0u8..=15u8 {
            let expected: u8 = (a.wrapping_add(b)) & 0b1111;
            let got = nand::add_u4(a, b);
            assert_eq!(got, expected, "add_u4({}, {}) = {}, expected {}", a, b, got, expected);
        }
    }
}

#[test]
fn test_add_u4_specific_values() {
    // Spot checks aligned with C ground truth
    assert_eq!(nand::add_u4(0, 0), 0);
    assert_eq!(nand::add_u4(1, 1), 2);
    assert_eq!(nand::add_u4(7, 8), 15);
    assert_eq!(nand::add_u4(8, 8), 0);
    assert_eq!(nand::add_u4(15, 15), 14);
    assert_eq!(nand::add_u4(10, 7), 1);
    assert_eq!(nand::add_u4(14, 14), 12);
    assert_eq!(nand::add_u4(15, 1), 0);
    assert_eq!(nand::add_u4(9, 6), 15);
    assert_eq!(nand::add_u4(15, 14), 13);
}

#[test]
fn test_sub_u4_all_pairs() {
    for a in 0u8..=15u8 {
        for b in 0u8..=15u8 {
            let expected: u8 = (a.wrapping_sub(b)) & 0b1111;
            let got = nand::sub_u4(a, b);
            assert_eq!(got, expected, "sub_u4({}, {}) = {}, expected {}", a, b, got, expected);
        }
    }
}

#[test]
fn test_sub_u4_specific_values() {
    // Spot checks aligned with C ground truth
    assert_eq!(nand::sub_u4(0, 0), 0);
    assert_eq!(nand::sub_u4(0, 1), 15);
    assert_eq!(nand::sub_u4(0, 15), 1);
    assert_eq!(nand::sub_u4(1, 1), 0);
    assert_eq!(nand::sub_u4(7, 7), 0);
    assert_eq!(nand::sub_u4(5, 7), 14);
    assert_eq!(nand::sub_u4(5, 10), 11);
    assert_eq!(nand::sub_u4(15, 15), 0);
    assert_eq!(nand::sub_u4(15, 0), 15);
    assert_eq!(nand::sub_u4(7, 15), 8);
}

#[test]
fn test_check_add_returns_true() {
    // The loop iterates a, b in 0..U4_MAX (exclusive). For all such pairs,
    // add_u4 should match wrapping add. Both C and Rust use the same loop
    // bounds, so this should return true.
    assert_eq!(nand::check_add(), true);
}

#[test]
fn test_check_sub_returns_true() {
    assert_eq!(nand::check_sub(), true);
}

#[test]
fn test_print_add_bit_does_not_panic() {
    // Ensure print_add_bit runs to completion for every input combination.
    for a in [false, true] {
        for b in [false, true] {
            for c in [false, true] {
                nand::print_add_bit(a, b, c);
            }
        }
    }
}

fn main() {}
