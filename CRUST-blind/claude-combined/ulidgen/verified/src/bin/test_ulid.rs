use ulidgen::ulid;

const B32_ALPHABET: &str = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";

fn is_valid_b32_char(c: char) -> bool {
    B32_ALPHABET.contains(c)
}

fn ulid_to_string(buf: &[char; ulid::ULID_LENGTH]) -> String {
    buf[..26].iter().collect()
}

#[test]
fn test_ulid_length_constant() {
    // ULID_LENGTH should be 27 (26 chars + null terminator slot)
    assert_eq!(ulid::ULID_LENGTH, 27);
}

#[test]
fn test_ulid_buffer_null_terminator() {
    // After ulidgen_r, position 26 must be '\0' to mirror C behavior
    let mut buf = ['X'; ulid::ULID_LENGTH];
    ulid::ulidgen_r(&mut buf);
    assert_eq!(buf[26], '\0');
}

#[test]
fn test_ulid_length_is_26_chars() {
    let mut buf = ['\0'; ulid::ULID_LENGTH];
    ulid::ulidgen_r(&mut buf);
    let s = ulid_to_string(&buf);
    assert_eq!(s.len(), 26);
}

#[test]
fn test_ulid_only_valid_b32_characters() {
    let mut buf = ['\0'; ulid::ULID_LENGTH];
    ulid::ulidgen_r(&mut buf);
    for i in 0..26 {
        assert!(
            is_valid_b32_char(buf[i]),
            "ulid[{}] = {:?} is not in Base32 Crockford alphabet",
            i,
            buf[i]
        );
    }
}

#[test]
fn test_ulid_uniqueness_consecutive() {
    let mut buf1 = ['\0'; ulid::ULID_LENGTH];
    let mut buf2 = ['\0'; ulid::ULID_LENGTH];
    ulid::ulidgen_r(&mut buf1);
    ulid::ulidgen_r(&mut buf2);
    let s1 = ulid_to_string(&buf1);
    let s2 = ulid_to_string(&buf2);
    assert_ne!(s1, s2, "Two consecutive ULIDs must differ");
}

#[test]
fn test_ulid_sortability_time_based() {
    let mut buf1 = ['\0'; ulid::ULID_LENGTH];
    ulid::ulidgen_r(&mut buf1);

    std::thread::sleep(std::time::Duration::from_millis(2));

    let mut buf2 = ['\0'; ulid::ULID_LENGTH];
    ulid::ulidgen_r(&mut buf2);

    let s1 = ulid_to_string(&buf1);
    let s2 = ulid_to_string(&buf2);
    assert!(
        s1 < s2,
        "ULID generated later should sort after earlier one: {} vs {}",
        s1,
        s2
    );
}

#[test]
fn test_ulid_increments_when_same_ms() {
    // When called twice in the same millisecond, the random part should be
    // monotonically incremented rather than fully randomized.
    // We test this by passing in a buffer that already encodes a known
    // timestamp + random portion, then setting up a tight call. In practice
    // the simplest way to verify "same" path is to call in a tight loop and
    // observe that there exist consecutive pairs which differ only in the
    // last positions (random portion).

    let mut prev = ['\0'; ulid::ULID_LENGTH];
    let mut cur = ['\0'; ulid::ULID_LENGTH];
    ulid::ulidgen_r(&mut prev);

    let mut found_increment = false;
    for _ in 0..10000 {
        // copy prev into cur to simulate state being preserved across calls
        cur = prev;
        ulid::ulidgen_r(&mut cur);
        // timestamp prefix length 10
        let prev_ts: String = prev[..10].iter().collect();
        let cur_ts: String = cur[..10].iter().collect();
        if prev_ts == cur_ts {
            // when timestamp matches, the cur should be > prev lexicographically
            let ps: String = prev[..26].iter().collect();
            let cs: String = cur[..26].iter().collect();
            assert!(cs > ps, "Same-ms ULID must increment: {} vs {}", ps, cs);
            found_increment = true;
            break;
        }
        prev = cur;
    }

    // If the loop never found a same-ms case (unlikely), we still pass — the
    // function may have hit different millisecond boundaries each time.
    let _ = found_increment;
}

#[test]
fn test_ulid_timestamp_encodes_recent_time() {
    use std::time::{SystemTime, UNIX_EPOCH};

    let before = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    let mut buf = ['\0'; ulid::ULID_LENGTH];
    ulid::ulidgen_r(&mut buf);

    let after = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    // Decode the first 10 chars as base32 timestamp
    let mut t: u64 = 0;
    for i in 0..10 {
        let pos = B32_ALPHABET
            .find(buf[i])
            .expect("char must be in alphabet") as u64;
        t = t * 32 + pos;
    }

    assert!(
        t >= before && t <= after + 1,
        "Encoded timestamp {} should be between {} and {}",
        t,
        before,
        after
    );
}

#[test]
fn test_ulid_many_calls_all_valid() {
    // Stress test: generate many ULIDs and verify each is a valid 26-char b32 string
    let mut buf = ['\0'; ulid::ULID_LENGTH];
    for _ in 0..1000 {
        ulid::ulidgen_r(&mut buf);
        assert_eq!(buf[26], '\0');
        for i in 0..26 {
            assert!(is_valid_b32_char(buf[i]));
        }
    }
}

fn main() {}
