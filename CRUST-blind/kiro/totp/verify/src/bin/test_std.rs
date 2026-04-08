use totp::std::{memset, memcpy, strlen, SIZE_MAX};

#[test]
fn test_size_max() {
    assert_eq!(SIZE_MAX, usize::MAX);
}

#[test]
fn test_memset_basic() {
    let mut buf = [0u8; 8];
    memset(&mut buf, 0xAA, 4);
    assert_eq!(buf, [0xAA, 0xAA, 0xAA, 0xAA, 0, 0, 0, 0]);
}

#[test]
fn test_memset_zero() {
    let mut buf = [0xFFu8; 4];
    memset(&mut buf, 0, 4);
    assert_eq!(buf, [0, 0, 0, 0]);
}

#[test]
fn test_memset_zero_len() {
    let mut buf = [1u8; 4];
    memset(&mut buf, 0xFF, 0);
    assert_eq!(buf, [1, 1, 1, 1]);
}

#[test]
fn test_memcpy_basic() {
    let src = [1u8, 2, 3, 4, 5];
    let mut dst = [0u8; 5];
    memcpy(&mut dst, &src, 5);
    assert_eq!(dst, [1, 2, 3, 4, 5]);
}

#[test]
fn test_memcpy_partial() {
    let src = [0xAA, 0xBB, 0xCC, 0xDD];
    let mut dst = [0u8; 4];
    memcpy(&mut dst, &src, 2);
    assert_eq!(dst, [0xAA, 0xBB, 0, 0]);
}

#[test]
fn test_memcpy_zero_len() {
    let src = [1u8; 4];
    let mut dst = [0u8; 4];
    memcpy(&mut dst, &src, 0);
    assert_eq!(dst, [0, 0, 0, 0]);
}

#[test]
fn test_strlen_basic() {
    assert_eq!(strlen("hello"), 5);
}

#[test]
fn test_strlen_empty() {
    assert_eq!(strlen(""), 0);
}

#[test]
fn test_strlen_one() {
    assert_eq!(strlen("x"), 1);
}

fn main() {}
