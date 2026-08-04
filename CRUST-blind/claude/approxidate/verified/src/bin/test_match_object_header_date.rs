#![allow(unused_imports)]
use approxidate::approxidate;

#[test]
fn test_match_object_header_date_basic() {
    let mut tv = approxidate::TimeVal { tv_sec: 0, tv_usec: 0 };
    let mut off = -1;
    let rc = approxidate::match_object_header_date("1234567890 +0500", &mut tv, &mut off);
    assert_eq!(rc, 0);
    assert_eq!(tv.tv_sec, 1234567890);
    assert_eq!(tv.tv_usec, 0);
    assert_eq!(off, 300);
}

#[test]
fn test_match_object_header_date_negative_offset() {
    let mut tv = approxidate::TimeVal { tv_sec: 0, tv_usec: 0 };
    let mut off = -1;
    let rc = approxidate::match_object_header_date("1234567890 -0530", &mut tv, &mut off);
    assert_eq!(rc, 0);
    assert_eq!(tv.tv_sec, 1234567890);
    assert_eq!(off, -330);
}

#[test]
fn test_match_object_header_date_not_digit() {
    let mut tv = approxidate::TimeVal { tv_sec: 0, tv_usec: 0 };
    let mut off = -1;
    let rc = approxidate::match_object_header_date("abc", &mut tv, &mut off);
    assert_eq!(rc, -1);
    assert_eq!(tv.tv_sec, 0);
    assert_eq!(off, -1);
}

#[test]
fn test_match_object_header_date_short_offset_invalid() {
    // " +50" is only 3 digits, should fail end != date+4 check
    let mut tv = approxidate::TimeVal { tv_sec: 0, tv_usec: 0 };
    let mut off = -1;
    let rc = approxidate::match_object_header_date("1234567890 +50", &mut tv, &mut off);
    assert_eq!(rc, -1);
}

#[test]
fn test_match_object_header_date_no_space() {
    let mut tv = approxidate::TimeVal { tv_sec: 0, tv_usec: 0 };
    let mut off = -1;
    let rc = approxidate::match_object_header_date("1234567890+0500", &mut tv, &mut off);
    assert_eq!(rc, -1);
}

#[test]
fn test_match_object_header_date_extra_space_after_sign() {
    let mut tv = approxidate::TimeVal { tv_sec: 0, tv_usec: 0 };
    let mut off = -1;
    let rc = approxidate::match_object_header_date("1234567890 + 0500", &mut tv, &mut off);
    assert_eq!(rc, -1);
}

#[test]
fn test_match_object_header_date_short_stamp() {
    let mut tv = approxidate::TimeVal { tv_sec: 0, tv_usec: 0 };
    let mut off = -1;
    let rc = approxidate::match_object_header_date("1234 +0500", &mut tv, &mut off);
    assert_eq!(rc, 0);
    assert_eq!(tv.tv_sec, 1234);
    assert_eq!(off, 300);
}

fn main() {}
