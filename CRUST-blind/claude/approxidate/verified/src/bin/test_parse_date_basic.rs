#![allow(unused_imports)]
use approxidate::approxidate;

#[test]
fn test_parse_date_basic_full_iso_with_usec() {
    let mut tv = approxidate::TimeVal { tv_sec: 0, tv_usec: 0 };
    let mut off = -1;
    let rc = approxidate::parse_date_basic("10/Mar/2013:00:00:02.003 UTC", &mut tv, &mut off);
    assert_eq!(rc, 0);
    assert_eq!(tv.tv_sec, 1362873602);
    assert_eq!(tv.tv_usec, 3000);
    assert_eq!(off, 0);
}

#[test]
fn test_parse_date_basic_iso_no_usec() {
    let mut tv = approxidate::TimeVal { tv_sec: 0, tv_usec: 0 };
    let mut off = -1;
    let rc = approxidate::parse_date_basic("10/Mar/2013:00:00:02 UTC", &mut tv, &mut off);
    assert_eq!(rc, 0);
    assert_eq!(tv.tv_sec, 1362873602);
    assert_eq!(tv.tv_usec, 0);
    assert_eq!(off, 0);
}

#[test]
fn test_parse_date_basic_with_tz_offset_plus_5() {
    let mut tv = approxidate::TimeVal { tv_sec: 0, tv_usec: 0 };
    let mut off = -1;
    let rc = approxidate::parse_date_basic("10/Mar/2012:00:00:07 +0500", &mut tv, &mut off);
    assert_eq!(rc, 0);
    assert_eq!(tv.tv_sec, 1331319607);
    assert_eq!(tv.tv_usec, 0);
    assert_eq!(off, 300);
}

#[test]
fn test_parse_date_basic_with_usec_and_tz() {
    let mut tv = approxidate::TimeVal { tv_sec: 0, tv_usec: 0 };
    let mut off = -1;
    let rc = approxidate::parse_date_basic("10/Mar/2012:00:00:07.657891 +0500", &mut tv, &mut off);
    assert_eq!(rc, 0);
    assert_eq!(tv.tv_sec, 1331319607);
    assert_eq!(tv.tv_usec, 657891);
    assert_eq!(off, 300);
}

#[test]
fn test_parse_date_basic_garbage() {
    let mut tv = approxidate::TimeVal { tv_sec: 0, tv_usec: 0 };
    let mut off = -1;
    let rc = approxidate::parse_date_basic("garbage", &mut tv, &mut off);
    assert_eq!(rc, -1);
    assert_eq!(tv.tv_sec, -1);
}

#[test]
fn test_parse_date_basic_at_object_header() {
    let mut tv = approxidate::TimeVal { tv_sec: 0, tv_usec: 0 };
    let mut off = -1;
    let rc = approxidate::parse_date_basic("@1234567890 +0500", &mut tv, &mut off);
    assert_eq!(rc, 0);
    assert_eq!(tv.tv_sec, 1234567890);
    assert_eq!(off, 300);
}

#[test]
fn test_parse_date_basic_at_object_header_negative() {
    let mut tv = approxidate::TimeVal { tv_sec: 0, tv_usec: 0 };
    let mut off = -1;
    let rc = approxidate::parse_date_basic("@1234567890 -0530", &mut tv, &mut off);
    assert_eq!(rc, 0);
    assert_eq!(tv.tv_sec, 1234567890);
    assert_eq!(off, -330);
}

#[test]
fn test_parse_date_basic_named_month() {
    let mut tv = approxidate::TimeVal { tv_sec: 0, tv_usec: 0 };
    let mut off = -1;
    let rc = approxidate::parse_date_basic("Mar 10 2013 04:00:07 -0500", &mut tv, &mut off);
    assert_eq!(rc, 0);
    assert_eq!(tv.tv_sec, 1362906007);
    assert_eq!(tv.tv_usec, 0);
    assert_eq!(off, -300);
}

fn main() {}
