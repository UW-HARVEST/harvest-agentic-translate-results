use fleur::myutils;

#[test]
fn test_print_bin_zero() {
    let s = myutils::print_bin(0);
    assert_eq!(s.len(), 64);
    assert_eq!(
        s,
        "0000000000000000000000000000000000000000000000000000000000000000"
    );
}

#[test]
fn test_print_bin_one() {
    let s = myutils::print_bin(1);
    assert_eq!(s.len(), 64);
    assert_eq!(
        s,
        "0000000000000000000000000000000000000000000000000000000000000001"
    );
}

#[test]
fn test_print_bin_two() {
    let s = myutils::print_bin(2);
    assert_eq!(s.len(), 64);
    assert_eq!(
        s,
        "0000000000000000000000000000000000000000000000000000000000000010"
    );
}

#[test]
fn test_print_bin_max() {
    let s = myutils::print_bin(u64::MAX);
    assert_eq!(s.len(), 64);
    assert_eq!(
        s,
        "1111111111111111111111111111111111111111111111111111111111111111"
    );
}

#[test]
fn test_print_bin_high_bit() {
    let s = myutils::print_bin(0x8000000000000000u64);
    assert_eq!(s.len(), 64);
    assert_eq!(
        s,
        "1000000000000000000000000000000000000000000000000000000000000000"
    );
}

#[test]
fn test_print_bin_deadbeef() {
    let s = myutils::print_bin(0xdeadbeefu64);
    assert_eq!(s.len(), 64);
    assert_eq!(
        s,
        "0000000000000000000000000000000011011110101011011011111011101111"
    );
}

fn main() {}
