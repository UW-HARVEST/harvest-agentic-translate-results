use cissy::range::range::{RangeElement, RangeType};

#[test]
fn test_new_defaults() {
    let e = RangeElement::new();
    assert_eq!(e.start, u32::MAX);
    assert_eq!(e.end, u32::MAX);
    assert_eq!(e.rangetype, RangeType::Empty);
    assert!(e.next.is_none());
}

#[test]
fn test_add_single_creates_list() {
    let l = RangeElement::add_single(5, None);
    let inner = l.as_ref().expect("list created");
    assert_eq!(inner.start, 5);
    assert_eq!(inner.rangetype, RangeType::Single);
    assert!(inner.next.is_none());
}

#[test]
fn test_add_start_end_creates_list() {
    let l = RangeElement::add_start_end(5, 10, None);
    let inner = l.as_ref().unwrap();
    assert_eq!(inner.start, 5);
    assert_eq!(inner.end, 10);
    assert_eq!(inner.rangetype, RangeType::StartEnd);
}

#[test]
fn test_add_start_end_single_when_equal() {
    // Per C: when start == end, the rangetype becomes Single.
    let l = RangeElement::add_start_end(7, 7, None);
    let inner = l.as_ref().unwrap();
    assert_eq!(inner.start, 7);
    assert_eq!(inner.rangetype, RangeType::Single);
    let mut buf = String::new();
    let s = RangeElement::list_to_string(&mut buf, 1024, &l);
    assert_eq!(s, "[7]");
}

#[test]
fn test_add_greater_equal_creates_list() {
    let l = RangeElement::add_greater_equal(40, None);
    let inner = l.as_ref().unwrap();
    assert_eq!(inner.start, 40);
    assert_eq!(inner.rangetype, RangeType::GreaterEqual);
}

#[test]
fn test_contains_num_single() {
    let l = RangeElement::add_single(5, None);
    assert_eq!(RangeElement::contains_num(4, &l), false);
    assert_eq!(RangeElement::contains_num(5, &l), true);
    assert_eq!(RangeElement::contains_num(6, &l), false);
}

#[test]
fn test_contains_num_start_end() {
    let l = RangeElement::add_start_end(5, 10, None);
    assert_eq!(RangeElement::contains_num(4, &l), false);
    assert_eq!(RangeElement::contains_num(5, &l), true);
    assert_eq!(RangeElement::contains_num(7, &l), true);
    assert_eq!(RangeElement::contains_num(10, &l), true);
    assert_eq!(RangeElement::contains_num(11, &l), false);
}

#[test]
fn test_contains_num_greater_equal() {
    let l = RangeElement::add_greater_equal(40, None);
    assert_eq!(RangeElement::contains_num(39, &l), false);
    assert_eq!(RangeElement::contains_num(40, &l), true);
    assert_eq!(RangeElement::contains_num(1000, &l), true);
}

#[test]
fn test_contains_num_combined() {
    let l = RangeElement::add_start_end(5, 10, None);
    let l = RangeElement::add_single(15, l);
    let l = RangeElement::add_greater_equal(40, l);
    assert_eq!(RangeElement::contains_num(1, &l), false);
    assert_eq!(RangeElement::contains_num(6, &l), true);
    assert_eq!(RangeElement::contains_num(12, &l), false);
    assert_eq!(RangeElement::contains_num(15, &l), true);
    assert_eq!(RangeElement::contains_num(50, &l), true);
}

#[test]
fn test_to_string_single() {
    let l = RangeElement::add_single(5, None);
    let mut buf = String::new();
    let s = l.as_ref().unwrap().to_string(&mut buf, 1024);
    assert_eq!(s, "[5]");
}

#[test]
fn test_to_string_start_end() {
    let l = RangeElement::add_start_end(5, 10, None);
    let mut buf = String::new();
    let s = l.as_ref().unwrap().to_string(&mut buf, 1024);
    assert_eq!(s, "[5-10]");
}

#[test]
fn test_to_string_greater_equal() {
    let l = RangeElement::add_greater_equal(7, None);
    let mut buf = String::new();
    let s = l.as_ref().unwrap().to_string(&mut buf, 1024);
    assert_eq!(s, "[7-]");
}

#[test]
fn test_to_string_empty() {
    let e = RangeElement::new();
    let mut buf = String::new();
    let s = e.to_string(&mut buf, 1024);
    assert_eq!(s, "[]");
}

#[test]
fn test_list_to_string_combined() {
    let l = RangeElement::add_start_end(5, 10, None);
    let l = RangeElement::add_single(15, l);
    let l = RangeElement::add_greater_equal(40, l);
    let mut buf = String::new();
    let s = RangeElement::list_to_string(&mut buf, 1024, &l);
    assert_eq!(s, "[5-10][15][40-]");
}

#[test]
fn test_list_to_string_duplicate() {
    let l = RangeElement::add_single(2, None);
    let l = RangeElement::add_start_end(2, 4, l);
    let mut buf = String::new();
    let s = RangeElement::list_to_string(&mut buf, 1024, &l);
    assert_eq!(s, "[2][2-4]");
}

#[test]
fn test_parse_int_ranges_simple_single() {
    let l = RangeElement::parse_int_ranges("3");
    let mut buf = String::new();
    let s = RangeElement::list_to_string(&mut buf, 1024, &l);
    assert_eq!(s, "[3]");
}

#[test]
fn test_parse_int_ranges_greater_equal() {
    let l = RangeElement::parse_int_ranges("3-");
    let mut buf = String::new();
    let s = RangeElement::list_to_string(&mut buf, 1024, &l);
    assert_eq!(s, "[3-]");
}

#[test]
fn test_parse_int_ranges_multiple_singles() {
    let l = RangeElement::parse_int_ranges("1,2,3");
    let mut buf = String::new();
    let s = RangeElement::list_to_string(&mut buf, 1024, &l);
    assert_eq!(s, "[1][2][3]");
}

#[test]
fn test_parse_int_ranges_combined() {
    let l = RangeElement::parse_int_ranges("1-3,5,7-");
    let mut buf = String::new();
    let s = RangeElement::list_to_string(&mut buf, 1024, &l);
    assert_eq!(s, "[1-3][5][7-]");
    assert_eq!(RangeElement::contains_num(0, &l), false);
    assert_eq!(RangeElement::contains_num(1, &l), true);
    assert_eq!(RangeElement::contains_num(2, &l), true);
    assert_eq!(RangeElement::contains_num(3, &l), true);
    assert_eq!(RangeElement::contains_num(4, &l), false);
    assert_eq!(RangeElement::contains_num(5, &l), true);
    assert_eq!(RangeElement::contains_num(6, &l), false);
    assert_eq!(RangeElement::contains_num(7, &l), true);
    assert_eq!(RangeElement::contains_num(100, &l), true);
}

#[test]
fn test_parse_int_ranges_2_2_to_4() {
    let l = RangeElement::parse_int_ranges("2,2-4");
    let mut buf = String::new();
    let s = RangeElement::list_to_string(&mut buf, 1024, &l);
    assert_eq!(s, "[2][2-4]");
}

#[test]
fn test_chained_adds() {
    let l = RangeElement::add_single(5, None);
    let l = RangeElement::add_single(10, l);
    let l = RangeElement::add_start_end(20, 25, l);
    let mut buf = String::new();
    let s = RangeElement::list_to_string(&mut buf, 1024, &l);
    assert_eq!(s, "[5][10][20-25]");
}

fn main() {}
