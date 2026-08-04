use cissy::csvline::CsvLine;

#[test]
fn test_new_initial_state() {
    let l = CsvLine::new();
    assert_eq!(l.fieldsize, 0);
    assert_eq!(l.current_idx, 0);
    assert_eq!(l.eol_str, "\n");
    assert_eq!(l.field.len(), 0);
}

#[test]
fn test_get_field_count_empty() {
    let l = CsvLine::new();
    assert_eq!(l.get_field_count(), 0);
}

#[test]
fn test_add_field_grows_to_ten() {
    let mut l = CsvLine::new();
    l.add_field("abc", 0, 3);
    // The C code grows in blocks of 10 entries on first add.
    assert_eq!(l.fieldsize, 10);
    assert_eq!(l.current_idx, 1);
    assert_eq!(l.get_field_count(), 1);
    assert_eq!(l.get_field(0).unwrap(), "abc");
}

#[test]
fn test_multiple_fields() {
    let mut l = CsvLine::new();
    l.add_field("abc", 0, 3);
    l.add_field("def", 0, 3);
    assert_eq!(l.fieldsize, 10);
    assert_eq!(l.current_idx, 2);
    assert_eq!(l.get_field_count(), 2);
    assert_eq!(l.get_field(0).unwrap(), "abc");
    assert_eq!(l.get_field(1).unwrap(), "def");
    // Out-of-range returns the empty string sentinel.
    assert_eq!(l.get_field(2).unwrap(), "");
}

#[test]
fn test_get_field_out_of_range() {
    let mut l = CsvLine::new();
    l.add_field("abc", 0, 3);
    assert_eq!(l.get_field(99).unwrap(), "");
}

#[test]
fn test_append_field_to_last() {
    let mut l = CsvLine::new();
    l.add_field("abc", 0, 3);
    l.add_field("def", 0, 3);
    l.append_field("X", 0, 1);
    assert_eq!(l.get_field_count(), 2);
    assert_eq!(l.get_field(0).unwrap(), "abc");
    assert_eq!(l.get_field(1).unwrap(), "defX");
}

#[test]
fn test_grow_to_twenty() {
    let mut l = CsvLine::new();
    for _ in 0..12 {
        l.add_field("x", 0, 1);
    }
    // Adding 12 entries triggered a second growth from 10 to 20.
    assert_eq!(l.fieldsize, 20);
    assert_eq!(l.current_idx, 12);
}

#[test]
fn test_grow_to_thirty() {
    let mut l = CsvLine::new();
    for _ in 0..25 {
        l.add_field("x", 0, 1);
    }
    assert_eq!(l.fieldsize, 30);
    assert_eq!(l.current_idx, 25);
}

#[test]
fn test_reset() {
    let mut l = CsvLine::new();
    for _ in 0..12 {
        l.add_field("x", 0, 1);
    }
    l.reset();
    // After reset, fieldsize is preserved but the index is zeroed.
    assert_eq!(l.fieldsize, 20);
    assert_eq!(l.current_idx, 0);
    assert_eq!(l.eol_str, "\n");
    // Existing fields are reset to empty data.
    for f in l.field.iter() {
        assert_eq!(f.data, "");
    }
}

#[test]
fn test_get_field_after_reset() {
    let mut l = CsvLine::new();
    l.add_field("abc", 0, 3);
    l.reset();
    // current_idx is now 0, so out-of-range returns empty.
    assert_eq!(l.get_field(0).unwrap(), "");
}

#[test]
fn test_add_field_with_offset() {
    let mut l = CsvLine::new();
    l.add_field("hello world", 6, 5);
    assert_eq!(l.get_field(0).unwrap(), "world");
}

fn main() {}
