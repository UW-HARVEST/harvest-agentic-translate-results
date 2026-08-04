use ulidgen::ulid;

const B32_ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

fn is_valid_b32_char(c: char) -> bool {
    B32_ALPHABET.iter().any(|&b| b as char == c)
}

#[test]
fn test_ulid_length_constant() {
    // The ULID buffer in C is char[27] (26 chars + null terminator).
    assert_eq!(ulid::ULID_LENGTH, 27);
}

#[test]
fn test_ulid_null_terminator() {
    let mut buf: [char; ulid::ULID_LENGTH] = ['\0'; ulid::ULID_LENGTH];
    ulid::ulidgen_r(&mut buf);
    // C: ulid[26] = 0;
    assert_eq!(buf[26], '\0');
}

#[test]
fn test_ulid_all_chars_valid_b32() {
    let mut buf: [char; ulid::ULID_LENGTH] = ['\0'; ulid::ULID_LENGTH];
    ulid::ulidgen_r(&mut buf);
    // All 26 chars must be from the Crockford base32 alphabet.
    for i in 0..26 {
        assert!(
            is_valid_b32_char(buf[i]),
            "Char at index {} is '{}' (0x{:x}), not in base32 alphabet",
            i,
            buf[i],
            buf[i] as u32
        );
    }
}

#[test]
fn test_ulid_string_length_26() {
    let mut buf: [char; ulid::ULID_LENGTH] = ['\0'; ulid::ULID_LENGTH];
    ulid::ulidgen_r(&mut buf);
    let s: String = buf.iter().take_while(|&&c| c != '\0').collect();
    // C test: assert(strlen(ulid) == 26)
    assert_eq!(s.len(), 26);
}

#[test]
fn test_ulid_uniqueness() {
    let mut a: [char; ulid::ULID_LENGTH] = ['\0'; ulid::ULID_LENGTH];
    let mut b: [char; ulid::ULID_LENGTH] = ['\0'; ulid::ULID_LENGTH];
    ulid::ulidgen_r(&mut a);
    ulid::ulidgen_r(&mut b);
    let sa: String = a.iter().take_while(|&&c| c != '\0').collect();
    let sb: String = b.iter().take_while(|&&c| c != '\0').collect();
    // C test asserts the strings differ; even with same timestamp the random
    // part is incremented in place so they should never match.
    assert_ne!(sa, sb);
}

#[test]
fn test_ulid_sortability() {
    let mut a: [char; ulid::ULID_LENGTH] = ['\0'; ulid::ULID_LENGTH];
    let mut b: [char; ulid::ULID_LENGTH] = ['\0'; ulid::ULID_LENGTH];
    ulid::ulidgen_r(&mut a);
    // Sleep > 1ms so the millisecond timestamp differs.
    std::thread::sleep(std::time::Duration::from_millis(5));
    ulid::ulidgen_r(&mut b);
    let sa: String = a.iter().take_while(|&&c| c != '\0').collect();
    let sb: String = b.iter().take_while(|&&c| c != '\0').collect();
    // ULIDs created later must sort after earlier ones (timestamp prefix).
    assert!(sa < sb, "Expected {} < {}", sa, sb);
}

#[test]
fn test_ulid_timestamp_prefix_matches_current_time() {
    // Compute the expected timestamp portion using the same formula as C:
    //   t = tv_sec*1000 + tv_nsec/1_000_000
    // Then iterate i from 9 down to 0, taking t%32 each time.
    let mut buf: [char; ulid::ULID_LENGTH] = ['\0'; ulid::ULID_LENGTH];
    let before = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap();
    let before_ms = before.as_secs() * 1000 + (before.subsec_nanos() as u64) / 1_000_000;
    ulid::ulidgen_r(&mut buf);
    let after = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap();
    let after_ms = after.as_secs() * 1000 + (after.subsec_nanos() as u64) / 1_000_000;

    // Decode the 10-char timestamp prefix back to a ms value.
    let mut decoded: u64 = 0;
    for i in 0..10 {
        let c = buf[i];
        let pos = B32_ALPHABET
            .iter()
            .position(|&b| b as char == c)
            .expect("char must be in alphabet");
        decoded = decoded * 32 + pos as u64;
    }

    // Decoded timestamp must lie within [before_ms, after_ms].
    assert!(
        decoded >= before_ms && decoded <= after_ms,
        "Decoded ts {} not in [{}, {}]",
        decoded,
        before_ms,
        after_ms
    );
}

#[test]
fn test_ulid_increment_random_part_when_same_timestamp() {
    // When the timestamp portion is unchanged from the previous call (the
    // buffer is reused), the random portion is *incremented in place*, not
    // re-randomized. Set up a buffer with a known random part and observe.
    let mut buf: [char; ulid::ULID_LENGTH] = ['\0'; ulid::ULID_LENGTH];

    // First call to populate timestamp + random part.
    ulid::ulidgen_r(&mut buf);
    let first_random: String = buf[10..26].iter().collect();
    let first_ts: String = buf[0..10].iter().collect();

    // Immediately call again (likely same millisecond).  If timestamp didn't
    // change, the last char of the random part should be incremented by one
    // base32 step (unless 'Z'/carry, which would be exceedingly rare for a
    // freshly random buffer).
    ulid::ulidgen_r(&mut buf);
    let second_ts: String = buf[0..10].iter().collect();
    let second_random: String = buf[10..26].iter().collect();

    if first_ts == second_ts {
        // Random part must differ
        assert_ne!(first_random, second_random);
    } else {
        // Timestamp changed (a millisecond rolled over) — both parts may differ.
        // Just ensure timestamp moved forward lexicographically.
        assert!(second_ts >= first_ts);
    }
}

#[test]
fn test_many_ulids_all_unique_and_sorted_reused_buffer() {
    // Mirror the C `ulidgen` main(): reuse the same buffer across calls so
    // the same-timestamp branch triggers in-place increments, giving a
    // strictly monotonic sequence.
    let n = 100;
    let mut buf: [char; ulid::ULID_LENGTH] = ['\0'; ulid::ULID_LENGTH];
    let mut prev: Option<String> = None;
    let mut all = Vec::with_capacity(n);
    for _ in 0..n {
        ulid::ulidgen_r(&mut buf);
        let s: String = buf.iter().take_while(|&&c| c != '\0').collect();
        assert_eq!(s.len(), 26);
        for c in s.chars() {
            assert!(is_valid_b32_char(c));
        }
        if let Some(p) = &prev {
            assert!(p < &s, "ULIDs not strictly sorted: {} >= {}", p, s);
        }
        prev = Some(s.clone());
        all.push(s);
    }
    // All unique.
    let mut sorted = all.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(sorted.len(), n);
}

#[test]
fn test_fresh_buffers_have_monotonic_timestamp_prefix() {
    // With fresh buffers each call the random part is fully regenerated, so
    // only the 10-char timestamp prefix is guaranteed monotonic.
    let n = 50;
    let mut prev_ts: Option<String> = None;
    for _ in 0..n {
        let mut buf: [char; ulid::ULID_LENGTH] = ['\0'; ulid::ULID_LENGTH];
        ulid::ulidgen_r(&mut buf);
        let ts: String = buf[0..10].iter().collect();
        if let Some(p) = &prev_ts {
            assert!(p <= &ts, "timestamp went backwards: {} > {}", p, ts);
        }
        prev_ts = Some(ts);
    }
}

fn main() {}
