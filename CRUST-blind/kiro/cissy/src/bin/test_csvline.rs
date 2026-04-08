use cissy::csvline::CsvLine;

#[test]
fn test_new() {
    let cline = CsvLine::new();
    assert_eq!(cline.get_field_count(), 0);
    assert_eq!(cline.fieldsize, 0);
    assert_eq!(cline.eol_str, "\n");
}

#[test]
fn test_add_field() {
    let mut cline = CsvLine::new();
    cline.add_field("hello", 0, 5);
    assert_eq!(cline.get_field_count(), 1);
    assert_eq!(cline.get_field(0), Some("hello"));
}

#[test]
fn test_add_field_with_offset() {
    let mut cline = CsvLine::new();
    cline.add_field("hello,world", 6, 5);
    assert_eq!(cline.get_field(0), Some("world"));
}

#[test]
fn test_add_multiple_fields() {
    let mut cline = CsvLine::new();
    cline.add_field("hello,world", 0, 5);
    cline.add_field("hello,world", 6, 5);
    assert_eq!(cline.get_field_count(), 2);
    assert_eq!(cline.get_field(0), Some("hello"));
    assert_eq!(cline.get_field(1), Some("world"));
}

#[test]
fn test_get_field_out_of_bounds() {
    let mut cline = CsvLine::new();
    cline.add_field("hello", 0, 5);
    assert_eq!(cline.get_field(10), Some(""));
}

#[test]
fn test_append_field() {
    let mut cline = CsvLine::new();
    cline.add_field("hello,world", 0, 5);
    cline.add_field("hello,world", 6, 5);
    cline.append_field(" extra", 0, 6);
    assert_eq!(cline.get_field(1), Some("world extra"));
}

#[test]
fn test_reset() {
    let mut cline = CsvLine::new();
    cline.add_field("hello", 0, 5);
    cline.add_field("world", 0, 5);
    cline.reset();
    assert_eq!(cline.get_field_count(), 0);
    assert_eq!(cline.eol_str, "\n");
}

#[test]
fn test_fieldsize_grows() {
    let mut cline = CsvLine::new();
    // Add 25 fields - should trigger growth from 0 to 10 to 20 to 30
    for _ in 0..25 {
        cline.add_field("hello", 0, 5);
    }
    assert_eq!(cline.get_field_count(), 25);
    assert_eq!(cline.fieldsize, 30);
    assert_eq!(cline.get_field(0), Some("hello"));
    assert_eq!(cline.get_field(24), Some("hello"));
}

#[test]
fn test_reuse_after_reset() {
    let mut cline = CsvLine::new();
    cline.add_field("hello", 0, 5);
    cline.reset();
    cline.add_field("new", 0, 3);
    assert_eq!(cline.get_field_count(), 1);
    assert_eq!(cline.get_field(0), Some("new"));
}

fn main() {}
