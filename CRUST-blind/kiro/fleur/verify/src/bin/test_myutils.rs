use fleur::myutils::print_bin;

#[test]
fn test_print_bin_zero() {
    assert_eq!(print_bin(0), "0000000000000000000000000000000000000000000000000000000000000000");
}

#[test]
fn test_print_bin_one() {
    assert_eq!(print_bin(1), "0000000000000000000000000000000000000000000000000000000000000001");
}

#[test]
fn test_print_bin_255() {
    assert_eq!(print_bin(255), "0000000000000000000000000000000000000000000000000000000011111111");
}

#[test]
fn test_print_bin_max() {
    assert_eq!(print_bin(u64::MAX), "1111111111111111111111111111111111111111111111111111111111111111");
}

#[test]
fn test_print_bin_42() {
    assert_eq!(print_bin(42), "0000000000000000000000000000000000000000000000000000000000101010");
}

#[test]
fn test_print_bin_length() {
    assert_eq!(print_bin(0).len(), 64);
    assert_eq!(print_bin(u64::MAX).len(), 64);
}

fn main() {}
