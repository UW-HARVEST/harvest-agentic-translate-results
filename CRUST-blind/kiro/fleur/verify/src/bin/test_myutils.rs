use fleur::myutils;

#[test]
fn test_print_bin_zero() {
    assert_eq!(myutils::print_bin(0), "0000000000000000000000000000000000000000000000000000000000000000");
}

#[test]
fn test_print_bin_one() {
    assert_eq!(myutils::print_bin(1), "0000000000000000000000000000000000000000000000000000000000000001");
}

#[test]
fn test_print_bin_max() {
    assert_eq!(myutils::print_bin(u64::MAX), "1111111111111111111111111111111111111111111111111111111111111111");
}

#[test]
fn test_print_bin_255() {
    assert_eq!(myutils::print_bin(255), "0000000000000000000000000000000000000000000000000000000011111111");
}

#[test]
fn test_print_bin_msb() {
    assert_eq!(myutils::print_bin(1u64 << 63), "1000000000000000000000000000000000000000000000000000000000000000");
}

#[test]
fn test_print_bin_length() {
    assert_eq!(myutils::print_bin(42).len(), 64);
}

fn main() {}
