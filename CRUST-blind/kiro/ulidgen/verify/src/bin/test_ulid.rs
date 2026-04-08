use ulidgen::ulid;

const B32_ALPHABET: &str = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";

#[test]
fn test_ulid_length_constant() {
    assert_eq!(ulid::ULID_LENGTH, 27);
}

#[test]
fn test_generated_ulid_is_26_chars() {
    let mut buf = ['\0'; ulid::ULID_LENGTH];
    ulid::ulidgen_r(&mut buf);
    // First 26 chars should be non-null, position 26 should be null
    for i in 0..26 {
        assert_ne!(buf[i], '\0', "char at index {} should not be null", i);
    }
    assert_eq!(buf[26], '\0');
}

#[test]
fn test_null_terminator_at_position_26() {
    let mut buf = ['X'; ulid::ULID_LENGTH];
    ulid::ulidgen_r(&mut buf);
    assert_eq!(buf[26], '\0');
}

#[test]
fn test_all_chars_in_b32_alphabet() {
    let mut buf = ['\0'; ulid::ULID_LENGTH];
    ulid::ulidgen_r(&mut buf);
    for i in 0..26 {
        assert!(
            B32_ALPHABET.contains(buf[i]),
            "char '{}' at index {} not in B32 alphabet",
            buf[i],
            i
        );
    }
}

#[test]
fn test_b32_alphabet_is_32_chars() {
    assert_eq!(B32_ALPHABET.len(), 32);
}

#[test]
fn test_b32_missing_ilou() {
    assert!(!B32_ALPHABET.contains('I'));
    assert!(!B32_ALPHABET.contains('L'));
    assert!(!B32_ALPHABET.contains('O'));
    assert!(!B32_ALPHABET.contains('U'));
}

#[test]
fn test_same_buffer_monotonic_increment() {
    // When reusing the same buffer, consecutive ULIDs within the same
    // millisecond should be monotonically increasing (increment random part)
    let mut buf = ['\0'; ulid::ULID_LENGTH];
    ulid::ulidgen_r(&mut buf);
    let first: String = buf[..26].iter().collect();

    ulid::ulidgen_r(&mut buf);
    let second: String = buf[..26].iter().collect();

    // Timestamp part should be same (called within same ms)
    assert_eq!(&first[..10], &second[..10], "timestamps should match for rapid calls");
    // Overall ULID should be strictly increasing
    assert!(second > first, "second ULID {} should be > first ULID {}", second, first);
}

#[test]
fn test_monotonic_sequence() {
    // Generate a sequence and verify all are strictly increasing
    let mut buf = ['\0'; ulid::ULID_LENGTH];
    let mut prev = String::new();
    for i in 0..20 {
        ulid::ulidgen_r(&mut buf);
        let curr: String = buf[..26].iter().collect();
        if i > 0 {
            assert!(curr > prev, "ULID[{}] {} should be > ULID[{}] {}", i, curr, i - 1, prev);
        }
        prev = curr;
    }
}

#[test]
fn test_increment_wraps_z_to_0() {
    // Set up a buffer where the last random char is 'Z'
    // When incremented, Z should wrap to '0' and carry to the left
    let mut buf = ['\0'; ulid::ULID_LENGTH];
    ulid::ulidgen_r(&mut buf);

    // Save the timestamp part
    let ts: Vec<char> = buf[..10].to_vec();

    // Set random part to end with 'Z'
    for i in 10..25 {
        buf[i] = 'A';
    }
    buf[25] = 'Z';

    // Call again - since timestamp matches, it should increment
    // Z wraps to '0', carry to position 24: A -> B
    ulid::ulidgen_r(&mut buf);

    // Verify timestamp hasn't changed (should be same ms)
    let new_ts: Vec<char> = buf[..10].to_vec();
    if ts == new_ts {
        // Increment happened: position 25 should be '0', position 24 should be 'B'
        assert_eq!(buf[25], '0', "Z should wrap to 0");
        assert_eq!(buf[24], 'B', "carry should increment A to B");
    }
    // If timestamp changed, the random part was re-randomized, which is also valid
}

#[test]
fn test_increment_carries_multiple() {
    // Set up buffer where multiple trailing chars are 'Z'
    let mut buf = ['\0'; ulid::ULID_LENGTH];
    ulid::ulidgen_r(&mut buf);

    let ts: Vec<char> = buf[..10].to_vec();

    // Set random part: ...AZZZ
    for i in 10..22 {
        buf[i] = 'A';
    }
    buf[22] = 'A';
    buf[23] = 'Z';
    buf[24] = 'Z';
    buf[25] = 'Z';

    ulid::ulidgen_r(&mut buf);

    let new_ts: Vec<char> = buf[..10].to_vec();
    if ts == new_ts {
        // All trailing Z's should wrap to '0', and position 22 should go A -> B
        assert_eq!(buf[25], '0');
        assert_eq!(buf[24], '0');
        assert_eq!(buf[23], '0');
        assert_eq!(buf[22], 'B');
    }
}

#[test]
fn test_separate_buffers_independent() {
    // Two separate buffers should get independent random parts
    let mut buf1 = ['\0'; ulid::ULID_LENGTH];
    let mut buf2 = ['\0'; ulid::ULID_LENGTH];
    ulid::ulidgen_r(&mut buf1);
    ulid::ulidgen_r(&mut buf2);

    let s1: String = buf1[..26].iter().collect();
    let s2: String = buf2[..26].iter().collect();

    // They should be different (random parts are independent)
    assert_ne!(s1, s2, "separate buffers should produce different ULIDs");
}

#[test]
fn test_timestamp_is_10_chars() {
    let mut buf = ['\0'; ulid::ULID_LENGTH];
    ulid::ulidgen_r(&mut buf);
    // First 10 chars are timestamp, all should be valid b32
    for i in 0..10 {
        assert!(B32_ALPHABET.contains(buf[i]), "timestamp char at {} invalid", i);
    }
}

#[test]
fn test_random_part_is_16_chars() {
    let mut buf = ['\0'; ulid::ULID_LENGTH];
    ulid::ulidgen_r(&mut buf);
    // Chars 10..26 are random part, all should be valid b32
    for i in 10..26 {
        assert!(B32_ALPHABET.contains(buf[i]), "random char at {} invalid", i);
    }
}

fn main() {}
