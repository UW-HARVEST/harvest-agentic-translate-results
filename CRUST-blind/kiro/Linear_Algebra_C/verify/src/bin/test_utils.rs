use Linear_Algebra_C::utils;

#[test]
fn test_exclusive_or() {
    assert_eq!(utils::exclusive_or(true, true), false);
    assert_eq!(utils::exclusive_or(true, false), true);
    assert_eq!(utils::exclusive_or(false, true), true);
    assert_eq!(utils::exclusive_or(false, false), false);
}

#[test]
fn test_roundn() {
    assert_eq!(utils::roundn(3.14159, 3), 3.142);
    assert_eq!(utils::roundn(2.71828, 2), 2.72);
    assert_eq!(utils::roundn(5.385165, 3), 5.385);
}

fn main() {}
