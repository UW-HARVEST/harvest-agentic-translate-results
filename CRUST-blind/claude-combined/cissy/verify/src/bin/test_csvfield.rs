use cissy::csvfield::CsvField;

#[test]
fn test_new_initial_state() {
    let f = CsvField::new();
    // Mirrors C csvfield_create: data is allocated but empty (no payload),
    // and the bookkeeping len starts at STR_MEM_PAD (10).
    assert_eq!(f.data, "");
    assert_eq!(f.len, 10);
}

#[test]
fn test_set_basic() {
    let mut f = CsvField::new();
    f.set("hello", 0, 5);
    assert_eq!(f.data, "hello");
    assert_eq!(f.len, 10);
}

#[test]
fn test_set_with_offset() {
    let mut f = CsvField::new();
    f.set("hello world", 6, 5);
    assert_eq!(f.data, "world");
    assert_eq!(f.len, 10);
}

#[test]
fn test_set_partial_with_offset() {
    let mut f = CsvField::new();
    // Force a long allocation first
    f.set("0123456789012345678901234567", 0, 28);
    assert_eq!(f.data, "0123456789012345678901234567");
    assert_eq!(f.len, 39);

    f.set("abcdefghij", 2, 4);
    assert_eq!(f.data, "cdef");
    // len should remain at the larger allocation
    assert_eq!(f.len, 39);
}

#[test]
fn test_set_long_grows_len() {
    let mut f = CsvField::new();
    // 28 chars triggers reallocation: new len = 28 + 1 + STR_MEM_PAD(10) = 39
    f.set("0123456789012345678901234567", 0, 28);
    assert_eq!(f.data, "0123456789012345678901234567");
    assert_eq!(f.len, 39);
}

#[test]
fn test_append_basic() {
    let mut f = CsvField::new();
    f.set("abc", 0, 3);
    f.append("def", 0, 3);
    assert_eq!(f.data, "abcdef");
    assert_eq!(f.len, 10);
}

#[test]
fn test_reset_clears_data() {
    let mut f = CsvField::new();
    f.set("hello", 0, 5);
    f.reset();
    assert_eq!(f.data, "");
    // Allocation length is preserved on reset (only data is cleared in C).
    assert_eq!(f.len, 10);
}

#[test]
fn test_set_after_reset() {
    let mut f = CsvField::new();
    f.set("hello", 0, 5);
    f.reset();
    f.set("world", 0, 5);
    assert_eq!(f.data, "world");
    assert_eq!(f.len, 10);
}

#[test]
fn test_set_with_zero_length() {
    let mut f = CsvField::new();
    f.set("hello", 0, 0);
    assert_eq!(f.data, "");
    assert_eq!(f.len, 10);
}

#[test]
fn test_append_to_empty() {
    let mut f = CsvField::new();
    f.append("hi", 0, 2);
    assert_eq!(f.data, "hi");
    assert_eq!(f.len, 10);
}

fn main() {}
