use cissy::csvline::CsvLine;

#[test]
fn test_new_initial_state() {
    let line = CsvLine::new();
    assert_eq!(line.current_idx, 0);
    assert_eq!(line.fieldsize, 0);
    assert_eq!(line.eol_str, "\n");
    assert_eq!(line.get_field_count(), 0);
}

#[test]
fn test_add_one_field_grows_to_10() {
    // Mirror C: size goes 0 -> 10 on first add
    let mut line = CsvLine::new();
    line.add_field("abc", 0, 3);
    assert_eq!(line.current_idx, 1);
    assert_eq!(line.fieldsize, 10);
    assert_eq!(line.get_field_count(), 1);
    assert_eq!(line.get_field(0), Some("abc"));
}

#[test]
fn test_add_10_then_11_grows_to_20() {
    let mut line = CsvLine::new();
    for _ in 0..10 {
        line.add_field("XX", 0, 2);
    }
    assert_eq!(line.current_idx, 10);
    assert_eq!(line.fieldsize, 10);
    line.add_field("YYY", 0, 3);
    assert_eq!(line.current_idx, 11);
    assert_eq!(line.fieldsize, 20);
}

#[test]
fn test_get_field_in_range() {
    let mut line = CsvLine::new();
    line.add_field("first", 0, 5);
    line.add_field("second", 0, 6);
    line.add_field("third", 0, 5);
    assert_eq!(line.get_field(0), Some("first"));
    assert_eq!(line.get_field(1), Some("second"));
    assert_eq!(line.get_field(2), Some("third"));
}

#[test]
fn test_get_field_out_of_range_returns_empty() {
    // Mirror C: csvline_getField returns "" for idx >= currentIdx
    let mut line = CsvLine::new();
    line.add_field("only", 0, 4);
    assert_eq!(line.get_field(1), Some(""));
    assert_eq!(line.get_field(100), Some(""));
}

#[test]
fn test_get_field_at_idx_eq_count() {
    // idx == current_idx should return ""
    let mut line = CsvLine::new();
    line.add_field("a", 0, 1);
    line.add_field("b", 0, 1);
    assert_eq!(line.get_field(2), Some(""));
}

#[test]
fn test_add_field_with_offset_and_length() {
    let mut line = CsvLine::new();
    line.add_field("0123456789", 2, 4);
    assert_eq!(line.get_field(0), Some("2345"));
}

#[test]
fn test_append_field_extends_last() {
    let mut line = CsvLine::new();
    line.add_field("YYY", 0, 3);
    line.append_field("+post", 0, 5);
    assert_eq!(line.get_field(0), Some("YYY+post"));
    assert_eq!(line.current_idx, 1);
}

#[test]
fn test_reset_clears_index_and_eol() {
    let mut line = CsvLine::new();
    line.add_field("a", 0, 1);
    line.add_field("b", 0, 1);
    assert_eq!(line.current_idx, 2);
    line.reset();
    assert_eq!(line.current_idx, 0);
    assert_eq!(line.eol_str, "\n");
    assert_eq!(line.get_field_count(), 0);
}

#[test]
fn test_reset_preserves_capacity() {
    // After reset, fieldsize/Vec capacity is preserved (matches C behavior of
    // not freeing field slots).
    let mut line = CsvLine::new();
    for _ in 0..5 {
        line.add_field("x", 0, 1);
    }
    assert_eq!(line.fieldsize, 10);
    line.reset();
    assert_eq!(line.fieldsize, 10);
    assert_eq!(line.field.len(), 10);
}

#[test]
fn test_reset_then_add_again() {
    // After reset(), fields are emptied but capacity remains; subsequent
    // add_field should reuse existing slots.
    let mut line = CsvLine::new();
    line.add_field("hello", 0, 5);
    line.reset();
    line.add_field("world", 0, 5);
    assert_eq!(line.current_idx, 1);
    assert_eq!(line.get_field(0), Some("world"));
    // No new growth needed
    assert_eq!(line.fieldsize, 10);
}

#[test]
fn test_25_fields_grows_three_times() {
    // 25 fields: size goes 0->10->20->30
    let mut line = CsvLine::new();
    for _ in 0..25 {
        line.add_field("hello", 0, 5);
    }
    assert_eq!(line.current_idx, 25);
    assert_eq!(line.fieldsize, 30);
    assert_eq!(line.get_field(0), Some("hello"));
    assert_eq!(line.get_field(24), Some("hello"));
    // out of range
    assert_eq!(line.get_field(25), Some(""));
}

#[test]
fn test_field_count_matches_current_idx() {
    let mut line = CsvLine::new();
    line.add_field("a", 0, 1);
    line.add_field("bb", 0, 2);
    line.add_field("ccc", 0, 3);
    assert_eq!(line.get_field_count(), 3);
    assert_eq!(line.get_field_count(), line.current_idx);
}

fn main() {}
