use cissy::range::range::{RangeElement, RangeType};

#[test]
fn test_new_default_state() {
    let e = RangeElement::new();
    assert_eq!(e.start, u32::MAX);
    assert_eq!(e.end, u32::MAX);
    assert_eq!(e.rangetype, RangeType::Empty);
    assert!(e.next.is_none());
}

#[test]
fn test_add_single_to_empty() {
    let list = RangeElement::add_single(5, None);
    assert!(list.is_some());
    let head = list.unwrap();
    assert_eq!(head.start, 5);
    assert_eq!(head.rangetype, RangeType::Single);
    assert!(head.next.is_none());
}

#[test]
fn test_contains_single() {
    let list = RangeElement::add_single(5, None);
    assert_eq!(RangeElement::contains_num(4, &list), false);
    assert_eq!(RangeElement::contains_num(5, &list), true);
    assert_eq!(RangeElement::contains_num(6, &list), false);
}

#[test]
fn test_single_to_string() {
    let list = RangeElement::add_single(5, None);
    let head = list.as_deref().unwrap();
    let mut buf = String::new();
    head.to_string(&mut buf, 1024);
    assert_eq!(buf, "[5]");
}

#[test]
fn test_single_list_to_string() {
    let list = RangeElement::add_single(5, None);
    let mut buf = String::new();
    RangeElement::list_to_string(&mut buf, 1024, &list);
    assert_eq!(buf, "[5]");
}

#[test]
fn test_start_end_contains() {
    let list = RangeElement::add_start_end(5, 10, None);
    assert_eq!(RangeElement::contains_num(4, &list), false);
    assert_eq!(RangeElement::contains_num(5, &list), true);
    assert_eq!(RangeElement::contains_num(7, &list), true);
    assert_eq!(RangeElement::contains_num(10, &list), true);
    assert_eq!(RangeElement::contains_num(12, &list), false);
}

#[test]
fn test_start_end_to_string() {
    let list = RangeElement::add_start_end(5, 10, None);
    let head = list.as_deref().unwrap();
    let mut buf = String::new();
    head.to_string(&mut buf, 1024);
    assert_eq!(buf, "[5-10]");

    let mut buf2 = String::new();
    RangeElement::list_to_string(&mut buf2, 1024, &list);
    assert_eq!(buf2, "[5-10]");
}

#[test]
fn test_start_end_equal_becomes_single() {
    // C: when start == end, rangetype becomes SINGLE
    let list = RangeElement::add_start_end(7, 7, None);
    let head = list.as_deref().unwrap();
    assert_eq!(head.start, 7);
    assert_eq!(head.rangetype, RangeType::Single);
}

#[test]
fn test_combined_list() {
    let mut list = RangeElement::add_start_end(5, 10, None);
    list = RangeElement::add_single(15, list);
    list = RangeElement::add_greater_equal(40, list);

    assert_eq!(RangeElement::contains_num(1, &list), false);
    assert_eq!(RangeElement::contains_num(6, &list), true);
    assert_eq!(RangeElement::contains_num(12, &list), false);
    assert_eq!(RangeElement::contains_num(50, &list), true);
    assert_eq!(RangeElement::contains_num(15, &list), true);
    assert_eq!(RangeElement::contains_num(40, &list), true);
    assert_eq!(RangeElement::contains_num(39, &list), false);

    // first element to_string
    let head = list.as_deref().unwrap();
    let mut buf = String::new();
    head.to_string(&mut buf, 1024);
    assert_eq!(buf, "[5-10]");

    let mut bufl = String::new();
    RangeElement::list_to_string(&mut bufl, 1024, &list);
    assert_eq!(bufl, "[5-10][15][40-]");
}

#[test]
fn test_duplicate_single_then_range() {
    let mut list = RangeElement::add_single(2, None);
    list = RangeElement::add_start_end(2, 4, list);
    let mut buf = String::new();
    RangeElement::list_to_string(&mut buf, 1024, &list);
    assert_eq!(buf, "[2][2-4]");
}

#[test]
fn test_parse_single() {
    let list = RangeElement::parse_int_ranges("5");
    let mut buf = String::new();
    RangeElement::list_to_string(&mut buf, 1024, &list);
    assert_eq!(buf, "[5]");
}

#[test]
fn test_parse_start_end() {
    let list = RangeElement::parse_int_ranges("5-10");
    let mut buf = String::new();
    RangeElement::list_to_string(&mut buf, 1024, &list);
    assert_eq!(buf, "[5-10]");
}

#[test]
fn test_parse_greater_equal() {
    let list = RangeElement::parse_int_ranges("5-");
    let mut buf = String::new();
    RangeElement::list_to_string(&mut buf, 1024, &list);
    assert_eq!(buf, "[5-]");
}

#[test]
fn test_parse_multiple_singles() {
    let list = RangeElement::parse_int_ranges("1,3,5");
    let mut buf = String::new();
    RangeElement::list_to_string(&mut buf, 1024, &list);
    assert_eq!(buf, "[1][3][5]");
}

#[test]
fn test_parse_mixed() {
    let list = RangeElement::parse_int_ranges("1-2,5,7-");
    let mut buf = String::new();
    RangeElement::list_to_string(&mut buf, 1024, &list);
    assert_eq!(buf, "[1-2][5][7-]");
}

#[test]
fn test_parse_2_2_4() {
    let list = RangeElement::parse_int_ranges("2,2-4");
    let mut buf = String::new();
    RangeElement::list_to_string(&mut buf, 1024, &list);
    assert_eq!(buf, "[2][2-4]");
}

#[test]
fn test_greater_equal_element_to_string() {
    let list = RangeElement::add_greater_equal(7, None);
    let head = list.as_deref().unwrap();
    let mut buf = String::new();
    head.to_string(&mut buf, 1024);
    assert_eq!(buf, "[7-]");
}

#[test]
fn test_greater_equal_contains() {
    let list = RangeElement::add_greater_equal(42, None);
    assert_eq!(RangeElement::contains_num(42, &list), true);
    assert_eq!(RangeElement::contains_num(41, &list), false);
    assert_eq!(RangeElement::contains_num(100, &list), true);
}

#[test]
fn test_mixed_list_thoroughly() {
    let mut list = RangeElement::add_single(2, None);
    list = RangeElement::add_start_end(5, 7, list);
    list = RangeElement::add_greater_equal(10, list);

    let mut buf = String::new();
    RangeElement::list_to_string(&mut buf, 1024, &list);
    assert_eq!(buf, "[2][5-7][10-]");

    assert_eq!(RangeElement::contains_num(1, &list), false);
    assert_eq!(RangeElement::contains_num(2, &list), true);
    assert_eq!(RangeElement::contains_num(3, &list), false);
    assert_eq!(RangeElement::contains_num(5, &list), true);
    assert_eq!(RangeElement::contains_num(6, &list), true);
    assert_eq!(RangeElement::contains_num(7, &list), true);
    assert_eq!(RangeElement::contains_num(8, &list), false);
    assert_eq!(RangeElement::contains_num(10, &list), true);
    assert_eq!(RangeElement::contains_num(11, &list), true);
}

#[test]
fn test_empty_to_string() {
    let mut e = RangeElement::new();
    e.rangetype = RangeType::Empty;
    let mut buf = String::new();
    e.to_string(&mut buf, 1024);
    assert_eq!(buf, "[]");
}

fn main() {}
