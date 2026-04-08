use ulidgen::ulid;

const B32_CHARS: &str = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";

fn ulid_string(buf: &[char; ulid::ULID_LENGTH]) -> String {
    buf[..26].iter().collect()
}

#[test]
fn test_ulid_length_is_26() {
    let mut buf = ['\0'; ulid::ULID_LENGTH];
    ulid::ulidgen_r(&mut buf);
    let s = ulid_string(&buf);
    assert_eq!(s.len(), 26);
}

#[test]
fn test_ulid_null_terminator() {
    let mut buf = ['\0'; ulid::ULID_LENGTH];
    ulid::ulidgen_r(&mut buf);
    assert_eq!(buf[26], '\0');
}

#[test]
fn test_ulid_valid_b32_chars() {
    let mut buf = ['\0'; ulid::ULID_LENGTH];
    ulid::ulidgen_r(&mut buf);
    let s = ulid_string(&buf);
    for c in s.chars() {
        assert!(B32_CHARS.contains(c), "Invalid char: {}", c);
    }
}

#[test]
fn test_ulid_uniqueness() {
    let mut buf = ['\0'; ulid::ULID_LENGTH];
    ulid::ulidgen_r(&mut buf);
    let s1 = ulid_string(&buf);
    ulid::ulidgen_r(&mut buf);
    let s2 = ulid_string(&buf);
    assert_ne!(s1, s2);
}

#[test]
fn test_ulid_sortability_with_delay() {
    let mut buf = ['\0'; ulid::ULID_LENGTH];
    ulid::ulidgen_r(&mut buf);
    let s1 = ulid_string(&buf);
    std::thread::sleep(std::time::Duration::from_millis(2));
    ulid::ulidgen_r(&mut buf);
    let s2 = ulid_string(&buf);
    assert!(s1 < s2, "Expected {} < {}", s1, s2);
}

#[test]
fn test_ulid_consecutive_share_timestamp() {
    let mut buf = ['\0'; ulid::ULID_LENGTH];
    ulid::ulidgen_r(&mut buf);
    let s1 = ulid_string(&buf);
    ulid::ulidgen_r(&mut buf);
    let s2 = ulid_string(&buf);
    // Same millisecond: first 10 chars (timestamp) should match
    assert_eq!(&s1[..10], &s2[..10]);
}

#[test]
fn test_ulid_consecutive_increment_random() {
    let mut buf = ['\0'; ulid::ULID_LENGTH];
    ulid::ulidgen_r(&mut buf);
    let s1 = ulid_string(&buf);
    ulid::ulidgen_r(&mut buf);
    let s2 = ulid_string(&buf);
    // Same timestamp means random part incremented, so s2 > s1
    assert!(s2 > s1);
}

#[test]
fn test_ulid_length_constant() {
    assert_eq!(ulid::ULID_LENGTH, 27);
}

#[test]
fn test_ulid_multiple_all_valid() {
    let mut buf = ['\0'; ulid::ULID_LENGTH];
    for _ in 0..10 {
        ulid::ulidgen_r(&mut buf);
        let s = ulid_string(&buf);
        assert_eq!(s.len(), 26);
        for c in s.chars() {
            assert!(B32_CHARS.contains(c));
        }
    }
}

fn main() {}
