use gorilla_paper_encode::gorilla::{bitslen, leading_zero64, trailing_zero64};

#[test]
fn test_bitslen_zero() {
    assert_eq!(bitslen(0), 0);
}

#[test]
fn test_bitslen_small_values() {
    assert_eq!(bitslen(1), 1);
    assert_eq!(bitslen(2), 2);
    assert_eq!(bitslen(3), 2);
    assert_eq!(bitslen(4), 3);
    assert_eq!(bitslen(5), 3);
    assert_eq!(bitslen(7), 3);
    assert_eq!(bitslen(8), 4);
    assert_eq!(bitslen(15), 4);
    assert_eq!(bitslen(16), 5);
}

#[test]
fn test_bitslen_byte_boundaries() {
    assert_eq!(bitslen(0xFF), 8);
    assert_eq!(bitslen(0x100), 9);
    assert_eq!(bitslen(0xFFFF), 16);
    assert_eq!(bitslen(0x10000), 17);
}

#[test]
fn test_bitslen_large_values() {
    assert_eq!(bitslen(0xFFFFFFFFu64), 32);
    assert_eq!(bitslen(0x100000000u64), 33);
    assert_eq!(bitslen(0xFFFFFFFFFFFFFFFFu64), 64);
    assert_eq!(bitslen(0x8000000000000000u64), 64);
    assert_eq!(bitslen(0x0080000000000000u64), 56);
    assert_eq!(bitslen(0x40a1f80000000000u64), 63);
}

#[test]
fn test_leading_zero64_zero() {
    assert_eq!(leading_zero64(0), 64);
}

#[test]
fn test_leading_zero64_values() {
    assert_eq!(leading_zero64(1), 63);
    assert_eq!(leading_zero64(2), 62);
    assert_eq!(leading_zero64(3), 62);
    assert_eq!(leading_zero64(8), 60);
    assert_eq!(leading_zero64(0xFF), 56);
    assert_eq!(leading_zero64(0x100), 55);
    assert_eq!(leading_zero64(0xFFFF), 48);
    assert_eq!(leading_zero64(0x10000), 47);
    assert_eq!(leading_zero64(0xFFFFFFFFu64), 32);
    assert_eq!(leading_zero64(0x100000000u64), 31);
    assert_eq!(leading_zero64(0xFFFFFFFFFFFFFFFFu64), 0);
    assert_eq!(leading_zero64(0x8000000000000000u64), 0);
    assert_eq!(leading_zero64(0x0080000000000000u64), 8);
    assert_eq!(leading_zero64(0x40a1f80000000000u64), 1);
}

#[test]
fn test_trailing_zero64_zero() {
    assert_eq!(trailing_zero64(0), 64);
}

#[test]
fn test_trailing_zero64_values() {
    assert_eq!(trailing_zero64(1), 0);
    assert_eq!(trailing_zero64(2), 1);
    assert_eq!(trailing_zero64(3), 0);
    assert_eq!(trailing_zero64(4), 2);
    assert_eq!(trailing_zero64(8), 3);
    assert_eq!(trailing_zero64(0xFF), 0);
    assert_eq!(trailing_zero64(0x100), 8);
    assert_eq!(trailing_zero64(0x10000), 16);
    assert_eq!(trailing_zero64(0x100000000u64), 32);
    assert_eq!(trailing_zero64(0x8000000000000000u64), 63);
    assert_eq!(trailing_zero64(0x0080000000000000u64), 55);
    assert_eq!(trailing_zero64(0x40a1f80000000000u64), 43);
    assert_eq!(trailing_zero64(0xFFFFFFFFFFFFFFFFu64), 0);
}

fn main() {}
