use cissy::csvfield::CsvField;

#[test]
fn test_new_field_initial_state() {
    // C: create() sets len = STR_MEM_PAD = 10, mallocs empty data
    let f = CsvField::new();
    assert_eq!(f.len, 10);
    assert_eq!(f.data, "");
}

#[test]
fn test_reset_clears_data() {
    let mut f = CsvField::new();
    f.set("hello", 0, 5);
    f.reset();
    assert_eq!(f.data, "");
}

#[test]
fn test_set_short_no_realloc() {
    // After set("hello"): 5+1=6 <= 10, len stays 10
    let mut f = CsvField::new();
    f.set("hello", 0, 5);
    assert_eq!(f.data, "hello");
    assert_eq!(f.len, 10);
}

#[test]
fn test_set_short_then_short() {
    let mut f = CsvField::new();
    f.set("hello", 0, 5);
    f.set("123", 0, 3);
    assert_eq!(f.data, "123");
    assert_eq!(f.len, 10);
}

#[test]
fn test_set_long_triggers_realloc() {
    // 27 chars: 27+1=28 > 10, len = 27+1+10 = 38
    let mut f = CsvField::new();
    f.set("123456789012345678901234567", 0, 27);
    assert_eq!(f.data, "123456789012345678901234567");
    assert_eq!(f.len, 38);
}

#[test]
fn test_set_after_long_keeps_capacity() {
    let mut f = CsvField::new();
    f.set("123456789012345678901234567", 0, 27);
    f.set("abc", 0, 3);
    // 3+1=4 <= 38, no realloc
    assert_eq!(f.data, "abc");
    assert_eq!(f.len, 38);
}

#[test]
fn test_append_within_capacity() {
    let mut f = CsvField::new();
    f.set("123456789012345678901234567", 0, 27);
    f.set("abc", 0, 3);
    f.append("DEF", 0, 3);
    // origflen=3, 3+3+1=7 <= 38, no realloc
    assert_eq!(f.data, "abcDEF");
    assert_eq!(f.len, 38);
}

#[test]
fn test_set_with_offset() {
    let mut f = CsvField::new();
    f.set("ABCDEFGH", 2, 4);
    assert_eq!(f.data, "CDEF");
}

#[test]
fn test_append_with_offset() {
    let mut f = CsvField::new();
    f.set("abc", 0, 3);
    f.append("1234567890", 3, 5);
    assert_eq!(f.data, "abc45678");
}

#[test]
fn test_append_to_empty_field() {
    let mut f = CsvField::new();
    f.append("hello", 0, 5);
    assert_eq!(f.data, "hello");
}

#[test]
fn test_set_zero_length() {
    let mut f = CsvField::new();
    f.set("hello", 0, 0);
    assert_eq!(f.data, "");
}

fn main() {}
