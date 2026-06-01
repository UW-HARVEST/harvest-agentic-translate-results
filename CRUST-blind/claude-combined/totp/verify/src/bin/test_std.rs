use totp::std;

#[test]
fn test_size_max() {
    assert_eq!(std::SIZE_MAX, !0usize);
}

#[test]
fn test_memset_basic() {
    let mut buf = [9u8; 10];
    std::memset(&mut buf, 0, 5);
    assert_eq!(buf, [0, 0, 0, 0, 0, 9, 9, 9, 9, 9]);
}

#[test]
fn test_memset_full() {
    let mut buf = [0u8; 8];
    std::memset(&mut buf, 0xAB, 8);
    assert_eq!(buf, [0xAB; 8]);
}

#[test]
fn test_memset_zero_n() {
    let mut buf = [42u8; 4];
    std::memset(&mut buf, 0, 0);
    assert_eq!(buf, [42u8; 4]);
}

#[test]
fn test_memset_returns_input() {
    let mut buf = [0u8; 3];
    let r = std::memset(&mut buf, 7, 3);
    assert_eq!(r, &mut [7u8, 7, 7]);
}

#[test]
fn test_memset_cast() {
    // C casts int -> uint8_t, so high bits are dropped.
    let mut buf = [0u8; 4];
    std::memset(&mut buf, 0x1FF, 4);
    assert_eq!(buf, [0xFF, 0xFF, 0xFF, 0xFF]);
}

#[test]
fn test_memcpy_basic() {
    let src = [1u8, 2, 3, 4, 5];
    let mut dst = [0u8; 5];
    std::memcpy(&mut dst, &src, 5);
    assert_eq!(dst, [1, 2, 3, 4, 5]);
}

#[test]
fn test_memcpy_partial() {
    let src = [10u8, 20, 30, 40, 50];
    let mut dst = [9u8; 5];
    std::memcpy(&mut dst, &src, 3);
    assert_eq!(dst, [10, 20, 30, 9, 9]);
}

#[test]
fn test_memcpy_zero() {
    let src = [1u8, 2, 3];
    let mut dst = [0u8; 3];
    std::memcpy(&mut dst, &src, 0);
    assert_eq!(dst, [0, 0, 0]);
}

#[test]
fn test_strlen_basic() {
    assert_eq!(std::strlen("hello"), 5);
}

#[test]
fn test_strlen_empty() {
    assert_eq!(std::strlen(""), 0);
}

#[test]
fn test_strlen_longer() {
    assert_eq!(std::strlen("The quick brown fox"), 19);
}

fn main() {}
