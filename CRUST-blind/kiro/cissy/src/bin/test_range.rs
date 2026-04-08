use cissy::range::range::{RangeElement, RangeType};

// --- add_single ---

#[test]
fn test_add_single_new_list() {
    let list = RangeElement::add_single(5, None);
    let e = list.as_ref().unwrap();
    assert_eq!(e.start, 5);
    assert_eq!(e.rangetype, RangeType::Single);
    assert!(e.next.is_none());
}

#[test]
fn test_add_single_to_existing() {
    let list = RangeElement::add_single(5, None);
    let list = RangeElement::add_single(10, list);
    let e = list.as_ref().unwrap();
    assert_eq!(e.start, 5);
    let e2 = e.next.as_ref().unwrap();
    assert_eq!(e2.start, 10);
    assert_eq!(e2.rangetype, RangeType::Single);
}

// --- add_start_end ---

#[test]
fn test_add_start_end() {
    let list = RangeElement::add_start_end(5, 10, None);
    let e = list.as_ref().unwrap();
    assert_eq!(e.start, 5);
    assert_eq!(e.end, 10);
    assert_eq!(e.rangetype, RangeType::StartEnd);
}

#[test]
fn test_add_start_end_equal_becomes_single() {
    let list = RangeElement::add_start_end(5, 5, None);
    let e = list.as_ref().unwrap();
    assert_eq!(e.start, 5);
    assert_eq!(e.rangetype, RangeType::Single);
}

// --- add_greater_equal ---

#[test]
fn test_add_greater_equal() {
    let list = RangeElement::add_greater_equal(3, None);
    let e = list.as_ref().unwrap();
    assert_eq!(e.start, 3);
    assert_eq!(e.rangetype, RangeType::GreaterEqual);
}

// --- contains_num ---

#[test]
fn test_contains_single() {
    let list = RangeElement::add_single(5, None);
    assert!(!RangeElement::contains_num(4, &list));
    assert!(RangeElement::contains_num(5, &list));
    assert!(!RangeElement::contains_num(6, &list));
}

#[test]
fn test_contains_start_end() {
    let list = RangeElement::add_start_end(5, 10, None);
    assert!(!RangeElement::contains_num(4, &list));
    assert!(RangeElement::contains_num(5, &list));
    assert!(RangeElement::contains_num(7, &list));
    assert!(RangeElement::contains_num(10, &list));
    assert!(!RangeElement::contains_num(12, &list));
}

#[test]
fn test_contains_greater_equal() {
    let list = RangeElement::add_greater_equal(1, None);
    assert!(!RangeElement::contains_num(0, &list));
    assert!(RangeElement::contains_num(1, &list));
    assert!(RangeElement::contains_num(100, &list));
}

#[test]
fn test_contains_mixed_list() {
    let list = RangeElement::add_start_end(5, 10, None);
    let list = RangeElement::add_single(15, list);
    let list = RangeElement::add_greater_equal(40, list);
    assert!(!RangeElement::contains_num(1, &list));
    assert!(RangeElement::contains_num(6, &list));
    assert!(!RangeElement::contains_num(12, &list));
    assert!(RangeElement::contains_num(15, &list));
    assert!(RangeElement::contains_num(50, &list));
}

#[test]
fn test_contains_empty_list() {
    let list: Option<Box<RangeElement>> = None;
    assert!(!RangeElement::contains_num(5, &list));
}

// --- to_string ---

#[test]
fn test_to_string_single() {
    let list = RangeElement::add_single(5, None);
    let e = list.as_ref().unwrap();
    let mut buf = String::new();
    e.to_string(&mut buf, 1024);
    assert_eq!(buf, "[5]");
}

#[test]
fn test_to_string_start_end() {
    let list = RangeElement::add_start_end(5, 10, None);
    let e = list.as_ref().unwrap();
    let mut buf = String::new();
    e.to_string(&mut buf, 1024);
    assert_eq!(buf, "[5-10]");
}

#[test]
fn test_to_string_greater_equal() {
    let list = RangeElement::add_greater_equal(3, None);
    let e = list.as_ref().unwrap();
    let mut buf = String::new();
    e.to_string(&mut buf, 1024);
    assert_eq!(buf, "[3-]");
}

#[test]
fn test_to_string_empty() {
    let e = RangeElement::new();
    let mut buf = String::new();
    e.to_string(&mut buf, 1024);
    assert_eq!(buf, "[]");
}

// --- list_to_string ---

#[test]
fn test_list_to_string_single() {
    let list = RangeElement::add_single(5, None);
    let mut buf = String::new();
    let result = RangeElement::list_to_string(&mut buf, 1024, &list);
    assert_eq!(result, "[5]");
}

#[test]
fn test_list_to_string_mixed() {
    let list = RangeElement::add_start_end(5, 10, None);
    let list = RangeElement::add_single(15, list);
    let list = RangeElement::add_greater_equal(40, list);
    let mut buf = String::new();
    let result = RangeElement::list_to_string(&mut buf, 1024, &list);
    assert_eq!(result, "[5-10][15][40-]");
}

#[test]
fn test_list_to_string_two_elements() {
    let list = RangeElement::add_single(2, None);
    let list = RangeElement::add_start_end(2, 4, list);
    let mut buf = String::new();
    let result = RangeElement::list_to_string(&mut buf, 1024, &list);
    assert_eq!(result, "[2][2-4]");
}

// --- parse_int_ranges ---

#[test]
fn test_parse_single() {
    let list = RangeElement::parse_int_ranges("1");
    let mut buf = String::new();
    assert_eq!(RangeElement::list_to_string(&mut buf, 1024, &list), "[1]");
}

#[test]
fn test_parse_multiple_singles() {
    let list = RangeElement::parse_int_ranges("1,2,3");
    let mut buf = String::new();
    assert_eq!(RangeElement::list_to_string(&mut buf, 1024, &list), "[1][2][3]");
}

#[test]
fn test_parse_range() {
    let list = RangeElement::parse_int_ranges("1-5");
    let mut buf = String::new();
    assert_eq!(RangeElement::list_to_string(&mut buf, 1024, &list), "[1-5]");
}

#[test]
fn test_parse_greater_equal() {
    let list = RangeElement::parse_int_ranges("3-");
    let mut buf = String::new();
    assert_eq!(RangeElement::list_to_string(&mut buf, 1024, &list), "[3-]");
}

#[test]
fn test_parse_mixed() {
    let list = RangeElement::parse_int_ranges("1,3-5,7-");
    let mut buf = String::new();
    assert_eq!(RangeElement::list_to_string(&mut buf, 1024, &list), "[1][3-5][7-]");
}

#[test]
fn test_parse_same_start_end_becomes_single() {
    let list = RangeElement::parse_int_ranges("3-3");
    let mut buf = String::new();
    assert_eq!(RangeElement::list_to_string(&mut buf, 1024, &list), "[3]");
}

#[test]
fn test_parse_complex() {
    let list = RangeElement::parse_int_ranges("2,2-4");
    let mut buf = String::new();
    assert_eq!(RangeElement::list_to_string(&mut buf, 1024, &list), "[2][2-4]");
}

fn main() {}
