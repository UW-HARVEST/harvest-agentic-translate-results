use Linear_Algebra_C::utils;

#[test]
fn test_exclusive_or_all_combinations() {
    assert_eq!(utils::exclusive_or(false, false), false);
    assert_eq!(utils::exclusive_or(false, true), true);
    assert_eq!(utils::exclusive_or(true, false), true);
    assert_eq!(utils::exclusive_or(true, true), false);
}

#[test]
fn test_roundn_basic() {
    // C ground truth values from running the C code:
    // roundn(3.14159, 2) = 3.14
    // roundn(3.14159, 4) = 3.1416
    // roundn(-2.789, 1) = -2.8
    // roundn(0.5, 1) = 0.5
    assert_eq!(utils::roundn(3.14159, 2), 3.14);
    assert_eq!(utils::roundn(3.14159, 4), 3.1416);
    assert_eq!(utils::roundn(-2.789, 1), -2.8);
    assert_eq!(utils::roundn(0.5, 1), 0.5);
}

#[test]
fn test_roundn_more_digits() {
    // Test rounding to 3 decimal places
    assert_eq!(utils::roundn(5.385164807134504, 3), 5.385);
    assert_eq!(utils::roundn(13.747727084867520, 3), 13.748);
}

#[test]
fn test_custom_assert_passes() {
    // custom_assert(non-zero) is a no-op (does not exit)
    utils::custom_assert(1);
    utils::custom_assert(42);
    utils::custom_assert(-1);
}

#[test]
fn test_print_call_stack_runs() {
    // Just ensure it runs without panicking
    utils::print_call_stack();
}

fn main() {}
