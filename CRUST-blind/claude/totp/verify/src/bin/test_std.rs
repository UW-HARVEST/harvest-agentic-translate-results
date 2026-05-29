use totp::std as cstd;

#[test]
fn test_size_max() {
    // SIZE_MAX should equal !0 for usize
    assert_eq!(cstd::SIZE_MAX, usize::MAX);
}

#[test]
fn test_memset_basic() {
    let mut buf = [0u8; 10];
    let result = cstd::memset(&mut buf, 0x42, 10);
    for i in 0..10 {
        assert_eq!(result[i], 0x42);
    }
}

#[test]
fn test_memset_partial() {
    let mut buf = [0xAAu8; 10];
    cstd::memset(&mut buf, 0, 5);
    for i in 0..5 {
        assert_eq!(buf[i], 0);
    }
    for i in 5..10 {
        assert_eq!(buf[i], 0xAA);
    }
}

#[test]
fn test_memset_zero_n() {
    let mut buf = [0xFFu8; 5];
    cstd::memset(&mut buf, 0, 0);
    // Nothing should change
    for i in 0..5 {
        assert_eq!(buf[i], 0xFF);
    }
}

#[test]
fn test_memset_truncate_high_byte() {
    // memset truncates int to byte; passing 0x1FF gives 0xFF
    let mut buf = [0u8; 4];
    cstd::memset(&mut buf, 0x1FF, 4);
    for i in 0..4 {
        assert_eq!(buf[i], 0xFF);
    }
}

#[test]
fn test_memcpy_basic() {
    let src = [1u8, 2, 3, 4, 5];
    let mut dst = [0u8; 5];
    cstd::memcpy(&mut dst, &src, 5);
    for i in 0..5 {
        assert_eq!(dst[i], src[i]);
    }
}

#[test]
fn test_memcpy_partial() {
    let src = [1u8, 2, 3, 4, 5];
    let mut dst = [0xFFu8; 5];
    cstd::memcpy(&mut dst, &src, 3);
    assert_eq!(dst[0], 1);
    assert_eq!(dst[1], 2);
    assert_eq!(dst[2], 3);
    assert_eq!(dst[3], 0xFF);
    assert_eq!(dst[4], 0xFF);
}

#[test]
fn test_memcpy_zero_n() {
    let src = [1u8, 2, 3];
    let mut dst = [0u8; 3];
    cstd::memcpy(&mut dst, &src, 0);
    assert_eq!(dst, [0u8; 3]);
}

#[test]
fn test_strlen_empty() {
    assert_eq!(cstd::strlen(""), 0);
}

#[test]
fn test_strlen_basic() {
    assert_eq!(cstd::strlen("hello"), 5);
}

#[test]
fn test_strlen_one_char() {
    assert_eq!(cstd::strlen("a"), 1);
}

#[test]
fn test_strlen_long() {
    assert_eq!(cstd::strlen("The quick brown fox"), 19);
}

#[test]
fn test_strlen_with_embedded_null() {
    // Rust strlen mirrors C: stops at first null byte
    let s = "abc\0def";
    assert_eq!(cstd::strlen(s), 3);
}

fn main() {}
