use fleur::myutils::print_bin;

#[test]
fn test_print_bin_zero() {
    let s = print_bin(0);
    assert_eq!(s.len(), 64);
    assert_eq!(
        s,
        "0000000000000000000000000000000000000000000000000000000000000000"
    );
}

#[test]
fn test_print_bin_one() {
    assert_eq!(
        print_bin(1),
        "0000000000000000000000000000000000000000000000000000000000000001"
    );
}

#[test]
fn test_print_bin_255() {
    assert_eq!(
        print_bin(255),
        "0000000000000000000000000000000000000000000000000000000011111111"
    );
}

#[test]
fn test_print_bin_42() {
    assert_eq!(
        print_bin(42),
        "0000000000000000000000000000000000000000000000000000000000101010"
    );
}

#[test]
fn test_print_bin_deadbeef() {
    assert_eq!(
        print_bin(0xDEADBEEF),
        "0000000000000000000000000000000011011110101011011011111011101111"
    );
}

#[test]
fn test_print_bin_all_ones() {
    let s = print_bin(0xFFFFFFFFFFFFFFFFu64);
    assert_eq!(s.len(), 64);
    assert_eq!(
        s,
        "1111111111111111111111111111111111111111111111111111111111111111"
    );
}

#[test]
fn test_print_bin_high_bit_only() {
    let s = print_bin(1u64 << 63);
    assert_eq!(s.len(), 64);
    let mut expected = String::from("1");
    expected.push_str(&"0".repeat(63));
    assert_eq!(s, expected);
}

fn main() {}
