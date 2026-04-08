use cissy::csvfield::CsvField;

#[test]
fn test_new() {
    let f = CsvField::new();
    assert_eq!(f.data, "");
    assert_eq!(f.len, 10);
}

#[test]
fn test_set_within_len() {
    let mut f = CsvField::new();
    f.set("hello world", 0, 5);
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
fn test_set_triggers_grow() {
    let mut f = CsvField::new();
    f.set("abcdefghijklmnopqrstuvwxyz", 0, 26);
    assert_eq!(f.data, "abcdefghijklmnopqrstuvwxyz");
    assert_eq!(f.len, 37); // 26+1+10
}

#[test]
fn test_set_no_shrink() {
    let mut f = CsvField::new();
    f.set("abcdefghijklmnopqrstuvwxyz", 0, 26);
    assert_eq!(f.len, 37);
    f.set("XY", 0, 2);
    assert_eq!(f.data, "XY");
    assert_eq!(f.len, 37);
}

#[test]
fn test_set_then_append_no_grow() {
    // C: set("abc",0,3) => data="abc", len=10. append("def",0,3) => data="abcdef", origflen=3, 3+3+1=7<=10, no grow
    let mut f = CsvField::new();
    f.set("abc", 0, 3);
    f.append("def", 0, 3);
    assert_eq!(f.data, "abcdef");
    assert_eq!(f.len, 10);
}

#[test]
fn test_append_triggers_grow() {
    // C: set("abc",0,3) => len=10. append 8 bytes: origflen=3, 3+8+1=12>10, grow: len=10+8+10=28
    let mut f = CsvField::new();
    f.set("abc", 0, 3);
    f.append("12345678", 0, 8);
    assert_eq!(f.data, "abc12345678");
    assert_eq!(f.len, 28);
}

#[test]
fn test_set_with_offset_partial() {
    let mut f = CsvField::new();
    f.set("abc", 1, 2);
    assert_eq!(f.data, "bc");
    assert_eq!(f.len, 10);
}

#[test]
fn test_reset() {
    let mut f = CsvField::new();
    f.set("hello", 0, 5);
    f.reset();
    assert_eq!(f.data, "");
}

fn main() {}
