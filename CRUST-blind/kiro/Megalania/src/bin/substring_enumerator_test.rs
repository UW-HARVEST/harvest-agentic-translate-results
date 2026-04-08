use Megalania::substring_enumerator::SubstringEnumerator;

const MIN_SUBSTRING: usize = 2;
const MAX_SUBSTRING: usize = 273;

fn main() {}

#[test]
fn substring_enumerator_hello_hello_test() {
    let data: &[u8] = b"hello hello";
    let expected_substrings: [u32; 11] = [0, 0, 0, 0, 0, 0, 4, 3, 2, 1, 0];

    let enumerator = SubstringEnumerator::new(data, MIN_SUBSTRING, MAX_SUBSTRING);

    for i in 0..data.len() {
        let mut count = 0u32;
        enumerator.for_each(i, |offset, length| {
            count += 1;
            assert!(offset + length <= data.len(), "Substring exceeds string bounds!");
            assert!(i + length <= data.len(), "Substring exceeds string bounds!");
            for j in 0..length {
                assert_eq!(data[i + j], data[offset + j], "Substring does not match!");
            }
        });
        assert_eq!(count, expected_substrings[i], "Unexpected number of substrings at position {i}!");
    }
}

#[test]
fn substring_enumerator_hello_hello_max_substring_test() {
    let data: &[u8] = b"hello hello";
    let expected_substrings: [u32; 11] = [0, 0, 0, 0, 0, 0, 2, 2, 2, 1, 0];

    let enumerator = SubstringEnumerator::new(data, MIN_SUBSTRING, 3);

    for i in 0..data.len() {
        let mut count = 0u32;
        enumerator.for_each(i, |offset, length| {
            count += 1;
            assert!(offset + length <= data.len(), "Substring exceeds string bounds!");
            assert!(i + length <= data.len(), "Substring exceeds string bounds!");
            for j in 0..length {
                assert_eq!(data[i + j], data[offset + j], "Substring does not match!");
            }
        });
        assert_eq!(count, expected_substrings[i], "Unexpected number of substrings at position {i}!");
    }
}

#[test]
fn substring_enumerator_aa_bb_cc_test() {
    let data: &[u8] = b"aa bb cc";

    let enumerator = SubstringEnumerator::new(data, MIN_SUBSTRING, MAX_SUBSTRING);

    for i in 0..data.len() {
        enumerator.for_each(i, |_offset, _length| {
            panic!("Got a substring when we expect no substrings!");
        });
    }
}
