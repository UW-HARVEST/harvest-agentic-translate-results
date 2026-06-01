use Linear_Algebra_C::utils;

#[test]
fn test_exclusive_or_all_combinations() {
    assert_eq!(utils::exclusive_or(true, true), false);
    assert_eq!(utils::exclusive_or(true, false), true);
    assert_eq!(utils::exclusive_or(false, true), true);
    assert_eq!(utils::exclusive_or(false, false), false);
}

#[test]
fn test_roundn_2_digits() {
    assert_eq!(utils::roundn(3.14159, 2), 3.14);
}

#[test]
fn test_roundn_3_digits() {
    assert_eq!(utils::roundn(3.14159, 3), 3.142);
}

#[test]
fn test_roundn_negative_value() {
    assert_eq!(utils::roundn(-1.2345, 2), -1.23);
}

#[test]
fn test_roundn_4_digits() {
    assert_eq!(utils::roundn(2.71828, 4), 2.7183);
}

fn main() {}
