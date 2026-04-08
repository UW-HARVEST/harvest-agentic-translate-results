use cissy::csvline::CsvLine;

#[test]
fn test_new() {
    let cl = CsvLine::new();
    assert_eq!(cl.fieldsize, 0);
    assert_eq!(cl.current_idx, 0);
    assert_eq!(cl.eol_str, "\n");
    assert!(cl.field.is_empty());
}

#[test]
fn test_add_field_grows_by_10() {
    let mut cl = CsvLine::new();
    cl.add_field("hello", 0, 5);
    assert_eq!(cl.fieldsize, 10);
    assert_eq!(cl.current_idx, 1);
    assert_eq!(cl.get_field(0), Some("hello"));
}

#[test]
fn test_add_two_fields() {
    let mut cl = CsvLine::new();
    cl.add_field("hello", 0, 5);
    cl.add_field("world", 0, 5);
    assert_eq!(cl.fieldsize, 10);
    assert_eq!(cl.current_idx, 2);
    assert_eq!(cl.get_field(0), Some("hello"));
    assert_eq!(cl.get_field(1), Some("world"));
}

#[test]
fn test_get_field_count() {
    let mut cl = CsvLine::new();
    assert_eq!(cl.get_field_count(), 0);
    cl.add_field("a", 0, 1);
    assert_eq!(cl.get_field_count(), 1);
    cl.add_field("b", 0, 1);
    assert_eq!(cl.get_field_count(), 2);
}

#[test]
fn test_get_field_out_of_range() {
    let mut cl = CsvLine::new();
    cl.add_field("hello", 0, 5);
    cl.add_field("world", 0, 5);
    // C returns "" for out of range; Rust returns None
    assert_eq!(cl.get_field(2), None);
    assert_eq!(cl.get_field(100), None);
}

#[test]
fn test_fieldsize_grows_at_boundary() {
    let mut cl = CsvLine::new();
    for _ in 0..10 {
        cl.add_field("hello", 0, 5);
    }
    assert_eq!(cl.fieldsize, 10);
    assert_eq!(cl.current_idx, 10);
    cl.add_field("hello", 0, 5);
    assert_eq!(cl.fieldsize, 20);
    assert_eq!(cl.current_idx, 11);
}

#[test]
fn test_add_25_fields() {
    let mut cl = CsvLine::new();
    for _ in 0..25 {
        cl.add_field("hello", 0, 5);
    }
    assert_eq!(cl.fieldsize, 30);
    assert_eq!(cl.current_idx, 25);
    assert_eq!(cl.get_field(0), Some("hello"));
    assert_eq!(cl.get_field(24), Some("hello"));
}

#[test]
fn test_reset() {
    let mut cl = CsvLine::new();
    cl.add_field("hello", 0, 5);
    cl.add_field("world", 0, 5);
    cl.reset();
    assert_eq!(cl.current_idx, 0);
    assert_eq!(cl.fieldsize, 10); // fieldsize doesn't shrink
    assert_eq!(cl.eol_str, "\n");
}

#[test]
fn test_append_field() {
    let mut cl = CsvLine::new();
    cl.add_field("foo", 0, 3);
    cl.append_field("bar", 0, 3);
    assert_eq!(cl.get_field(0), Some("foobar"));
    assert_eq!(cl.get_field_count(), 1);
}

fn main() {}
