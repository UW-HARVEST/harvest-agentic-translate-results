#![allow(unused_imports)]
use morton::morton::morton as morton_fn;

#[test]
fn test_morton_basic_zero() {
    assert_eq!(morton_fn(0, 0), 0);
}

#[test]
fn test_morton_basic_lo_one() {
    assert_eq!(morton_fn(0, 1), 1);
}

#[test]
fn test_morton_basic_hi_one() {
    assert_eq!(morton_fn(1, 0), 2);
}

#[test]
fn test_morton_basic_both_one() {
    assert_eq!(morton_fn(1, 1), 3);
}

#[test]
fn test_morton_3_0() {
    // hi=0b0011, lo=0b0000 -> 0b1010 = 0xa
    assert_eq!(morton_fn(0b0011, 0b0000), 0b1010);
}

#[test]
fn test_morton_0_3() {
    // hi=0b0000, lo=0b0011 -> 0b0101 = 0x5
    assert_eq!(morton_fn(0b0000, 0b0011), 0b0101);
}

#[test]
fn test_morton_12_3() {
    // hi=0b1100, lo=0b0011 -> 0b10100101
    assert_eq!(morton_fn(0b1100, 0b0011), 0b10100101);
}

#[test]
fn test_morton_all_ones() {
    assert_eq!(
        morton_fn(0xFFFFFFFFu32, 0xFFFFFFFFu32),
        0xFFFFFFFFFFFFFFFFu64
    );
}

#[test]
fn test_morton_known_value() {
    assert_eq!(
        morton_fn(0x347210d1u32, 0xc6843fadu32),
        0x5a346a180755e653u64
    );
}

#[test]
fn test_morton_hi_only() {
    assert_eq!(morton_fn(0xFFFFFFFFu32, 0u32), 0xAAAAAAAAAAAAAAAAu64);
}

#[test]
fn test_morton_lo_only() {
    assert_eq!(morton_fn(0u32, 0xFFFFFFFFu32), 0x5555555555555555u64);
}

#[test]
fn test_morton_msb_only() {
    assert_eq!(
        morton_fn(0x80000000u32, 0x80000000u32),
        0xC000000000000000u64
    );
}

#[test]
fn test_morton_alternating() {
    assert_eq!(
        morton_fn(0xAAAAAAAAu32, 0x55555555u32),
        0x9999999999999999u64
    );
}

#[test]
fn test_morton_decimal() {
    assert_eq!(morton_fn(12345u32, 67890u32), 0x10A410F86u64);
}

#[test]
fn test_morton_misc() {
    assert_eq!(
        morton_fn(0x12345678u32, 0x9ABCDEF0u32),
        0x434C4F70737C7F80u64
    );
}

fn main() {}
