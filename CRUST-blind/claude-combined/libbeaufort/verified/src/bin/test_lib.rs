#[test]
fn test_constant_beaufort_alpha() {
    assert_eq!(
        libbeaufort::BEAUFORT_ALPHA,
        b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz"
    );
    assert_eq!(libbeaufort::BEAUFORT_ALPHA.len(), 62);
}

#[test]
fn test_constant_beaufort_version() {
    assert_eq!(libbeaufort::BEAUFORT_VERSION, "1");
}

#[test]
fn test_ssize_empty_string() {
    assert_eq!(libbeaufort::ssize(""), 0);
}

#[test]
fn test_ssize_normal_string() {
    assert_eq!(libbeaufort::ssize("Hello"), 5);
    assert_eq!(libbeaufort::ssize("Hello, World!"), 13);
}

#[test]
fn test_ssize_string_with_embedded_null() {
    // C `ssize` stops at the first '\0' byte.
    let s = "abc\0def";
    assert_eq!(libbeaufort::ssize(s), 3);
}

#[test]
fn test_ssize_long_alphabet() {
    let alpha = std::str::from_utf8(libbeaufort::BEAUFORT_ALPHA).unwrap();
    assert_eq!(libbeaufort::ssize(alpha), 62);
}

fn main() {}
