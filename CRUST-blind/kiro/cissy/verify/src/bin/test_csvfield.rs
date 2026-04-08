use cissy::csvfield::CsvField;

#[test]
fn test_new() {
    let f = CsvField::new();
    assert_eq!(f.data, "");
    assert_eq!(f.len, 10);
}

#[test]
fn test_set_basic() {
    let mut f = CsvField::new();
    f.set("hello", 0, 5);
    assert_eq!(f.data, "hello");
}

#[test]
fn test_set_with_offset() {
    let mut f = CsvField::new();
    f.set("hello world", 6, 5);
    assert_eq!(f.data, "world");
}

#[test]
fn test_set_overwrites() {
    let mut f = CsvField::new();
    f.set("hello", 0, 5);
    f.set("123", 0, 3);
    assert_eq!(f.data, "123");
}

#[test]
fn test_set_zero_length() {
    let mut f = CsvField::new();
    f.set("test", 0, 0);
    assert_eq!(f.data, "");
}

#[test]
fn test_append_basic() {
    let mut f = CsvField::new();
    f.set("abc", 0, 3);
    f.append("def", 0, 3);
    assert_eq!(f.data, "abcdef");
}

#[test]
fn test_append_with_offset() {
    let mut f = CsvField::new();
    f.set("xyz", 0, 3);
    f.append("hello", 2, 3);
    assert_eq!(f.data, "xyzllo");
}

#[test]
fn test_set_long_string() {
    let mut f = CsvField::new();
    let long = "123456789012345678901234567";
    f.set(long, 0, long.len());
    assert_eq!(f.data, long);
}

#[test]
fn test_reset() {
    let mut f = CsvField::new();
    f.set("hello", 0, 5);
    f.reset();
    assert_eq!(f.data, "");
}

#[test]
fn test_set_after_reset() {
    let mut f = CsvField::new();
    f.set("hello", 0, 5);
    f.reset();
    f.set("world", 0, 5);
    assert_eq!(f.data, "world");
}

#[test]
fn test_set_append_sequence() {
    let mut f = CsvField::new();
    f.set("123", 0, 3);
    f.append("123", 0, 3);
    assert_eq!(f.data, "123123");
}

fn main() {}
