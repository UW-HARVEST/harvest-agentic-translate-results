use cissy::range::range::{RangeElement, RangeType};

#[test]
fn test_new() {
    let r = RangeElement::new();
    assert_eq!(r.start, 0);
    assert_eq!(r.end, 0);
    assert_eq!(r.rangetype, RangeType::Empty);
    assert!(r.next.is_none());
}

#[test]
fn test_add_single() {
    let list = RangeElement::add_single(5, None);
    let elem = list.as_ref().unwrap();
    assert_eq!(elem.start, 5);
    assert_eq!(elem.rangetype, RangeType::Single);
    assert!(elem.next.is_none());
}

#[test]
fn test_add_single_chain() {
    let list = RangeElement::add_single(5, None);
    let list = RangeElement::add_single(15, list);
    let head = list.as_ref().unwrap();
    assert_eq!(head.start, 5);
    assert_eq!(head.rangetype, RangeType::Single);
    let second = head.next.as_ref().unwrap();
    assert_eq!(second.start, 15);
    assert_eq!(second.rangetype, RangeType::Single);
    assert!(second.next.is_none());
}

#[test]
fn test_add_start_end() {
    let list = RangeElement::add_start_end(5, 10, None);
    let elem = list.as_ref().unwrap();
    assert_eq!(elem.start, 5);
    assert_eq!(elem.end, 10);
    assert_eq!(elem.rangetype, RangeType::StartEnd);
}

#[test]
fn test_add_start_end_equal_becomes_single() {
    // C: rangeAddStartEnd(3,3,NULL) => rangetype=SINGLE
    let list = RangeElement::add_start_end(3, 3, None);
    let elem = list.as_ref().unwrap();
    assert_eq!(elem.start, 3);
    assert_eq!(elem.rangetype, RangeType::Single);
}

#[test]
fn test_add_greater_equal() {
    let list = RangeElement::add_greater_equal(5, None);
    let elem = list.as_ref().unwrap();
    assert_eq!(elem.start, 5);
    assert_eq!(elem.rangetype, RangeType::GreaterEqual);
}

#[test]
fn test_contains_single() {
    let list = RangeElement::add_single(3, None);
    assert!(!RangeElement::contains_num(4, &list));
    assert!(RangeElement::contains_num(3, &list));
}

#[test]
fn test_contains_start_end() {
    let list = RangeElement::add_start_end(3, 7, None);
    assert!(RangeElement::contains_num(3, &list));
    assert!(RangeElement::contains_num(4, &list));
    assert!(RangeElement::contains_num(7, &list));
    assert!(!RangeElement::contains_num(8, &list));
    assert!(!RangeElement::contains_num(2, &list));
}

#[test]
fn test_contains_greater_equal() {
    let list = RangeElement::add_greater_equal(5, None);
    assert!(RangeElement::contains_num(5, &list));
    assert!(RangeElement::contains_num(100, &list));
    assert!(!RangeElement::contains_num(4, &list));
}

#[test]
fn test_contains_complex_list() {
    // [5-10][15][40-]
    let list = RangeElement::add_start_end(5, 10, None);
    let list = RangeElement::add_single(15, list);
    let list = RangeElement::add_greater_equal(40, list);
    assert!(!RangeElement::contains_num(1, &list));
    assert!(RangeElement::contains_num(6, &list));
    assert!(!RangeElement::contains_num(12, &list));
    assert!(RangeElement::contains_num(50, &list));
    assert!(RangeElement::contains_num(15, &list));
    assert!(RangeElement::contains_num(40, &list));
}

#[test]
fn test_element_to_string_single() {
    let elem = RangeElement {
        start: 3, end: 0, rangetype: RangeType::Single, next: None,
    };
    let mut buf = String::new();
    elem.to_string(&mut buf, 1024);
    assert_eq!(buf, "[3]");
}

#[test]
fn test_element_to_string_start_end() {
    let elem = RangeElement {
        start: 3, end: 7, rangetype: RangeType::StartEnd, next: None,
    };
    let mut buf = String::new();
    elem.to_string(&mut buf, 1024);
    assert_eq!(buf, "[3-7]");
}

#[test]
fn test_element_to_string_greater_equal() {
    let elem = RangeElement {
        start: 5, end: 0, rangetype: RangeType::GreaterEqual, next: None,
    };
    let mut buf = String::new();
    elem.to_string(&mut buf, 1024);
    assert_eq!(buf, "[5-]");
}

#[test]
fn test_element_to_string_empty() {
    let elem = RangeElement::new();
    let mut buf = String::new();
    elem.to_string(&mut buf, 1024);
    assert_eq!(buf, "[]");
}

#[test]
fn test_list_to_string() {
    let list = RangeElement::add_single(1, None);
    let list = RangeElement::add_start_end(3, 5, list);
    let list = RangeElement::add_greater_equal(8, list);
    let mut buf = String::new();
    RangeElement::list_to_string(&mut buf, 1024, &list);
    assert_eq!(buf, "[1][3-5][8-]");
}

#[test]
fn test_list_to_string_single_element() {
    let list = RangeElement::add_single(5, None);
    let mut buf = String::new();
    RangeElement::list_to_string(&mut buf, 1024, &list);
    assert_eq!(buf, "[5]");
}

#[test]
fn test_parse_single() {
    let list = RangeElement::parse_int_ranges("3");
    let elem = list.as_ref().unwrap();
    assert_eq!(elem.start, 3);
    assert_eq!(elem.rangetype, RangeType::Single);
    assert!(elem.next.is_none());
}

#[test]
fn test_parse_two_singles() {
    let list = RangeElement::parse_int_ranges("2,5");
    let e0 = list.as_ref().unwrap();
    assert_eq!(e0.start, 2);
    assert_eq!(e0.rangetype, RangeType::Single);
    let e1 = e0.next.as_ref().unwrap();
    assert_eq!(e1.start, 5);
    assert_eq!(e1.rangetype, RangeType::Single);
    assert!(e1.next.is_none());
}

#[test]
fn test_parse_start_end() {
    let list = RangeElement::parse_int_ranges("3-7");
    let elem = list.as_ref().unwrap();
    assert_eq!(elem.start, 3);
    assert_eq!(elem.end, 7);
    assert_eq!(elem.rangetype, RangeType::StartEnd);
}

#[test]
fn test_parse_greater_equal() {
    let list = RangeElement::parse_int_ranges("5-");
    let elem = list.as_ref().unwrap();
    assert_eq!(elem.start, 5);
    assert_eq!(elem.rangetype, RangeType::GreaterEqual);
}

#[test]
fn test_parse_complex() {
    // "1,3-5,8-" => [1][3-5][8-]
    let list = RangeElement::parse_int_ranges("1,3-5,8-");
    let e0 = list.as_ref().unwrap();
    assert_eq!(e0.start, 1);
    assert_eq!(e0.rangetype, RangeType::Single);
    let e1 = e0.next.as_ref().unwrap();
    assert_eq!(e1.start, 3);
    assert_eq!(e1.end, 5);
    assert_eq!(e1.rangetype, RangeType::StartEnd);
    let e2 = e1.next.as_ref().unwrap();
    assert_eq!(e2.start, 8);
    assert_eq!(e2.rangetype, RangeType::GreaterEqual);
    assert!(e2.next.is_none());
}

#[test]
fn test_parse_equal_start_end_becomes_single() {
    // C: parseIntRanges("5-5") => SINGLE
    let list = RangeElement::parse_int_ranges("5-5");
    let elem = list.as_ref().unwrap();
    assert_eq!(elem.start, 5);
    assert_eq!(elem.rangetype, RangeType::Single);
}

#[test]
fn test_parse_and_list_to_string() {
    let list = RangeElement::parse_int_ranges("2,2-4");
    let mut buf = String::new();
    RangeElement::list_to_string(&mut buf, 1024, &list);
    assert_eq!(buf, "[2][2-4]");
}

fn main() {}
