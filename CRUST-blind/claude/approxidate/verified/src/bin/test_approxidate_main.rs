#![allow(unused_imports)]
use approxidate::approxidate;

#[test]
fn test_approxidate_main_full_iso_utc() {
    // approxidate_main(date) ~ approxidate(date) in C: parse-able dates produce
    // a deterministic absolute timestamp.
    let mut tv = approxidate::TimeVal { tv_sec: 0, tv_usec: 0 };
    let rc = approxidate::approxidate_main("10/Mar/2013:00:00:02.003 UTC", &mut tv);
    assert_eq!(rc, 0);
    assert_eq!(tv.tv_sec, 1362873602);
    assert_eq!(tv.tv_usec, 3000);
}

#[test]
fn test_approxidate_main_iso_with_tz() {
    let mut tv = approxidate::TimeVal { tv_sec: 0, tv_usec: 0 };
    let rc = approxidate::approxidate_main("10/Mar/2012:00:00:07 +0500", &mut tv);
    assert_eq!(rc, 0);
    assert_eq!(tv.tv_sec, 1331319607);
    assert_eq!(tv.tv_usec, 0);
}

#[test]
fn test_approxidate_main_iso_with_negative_tz() {
    let mut tv = approxidate::TimeVal { tv_sec: 0, tv_usec: 0 };
    let rc = approxidate::approxidate_main("10/Mar/2012:00:00:07.657891 -0110", &mut tv);
    assert_eq!(rc, 0);
    assert_eq!(tv.tv_sec, 1331341807);
    assert_eq!(tv.tv_usec, 657891);
}

#[test]
fn test_approxidate_main_named_month_form() {
    let mut tv = approxidate::TimeVal { tv_sec: 0, tv_usec: 0 };
    let rc = approxidate::approxidate_main("mar 10 2013 00:00:07 UTC", &mut tv);
    assert_eq!(rc, 0);
    assert_eq!(tv.tv_sec, 1362873607);
    assert_eq!(tv.tv_usec, 0);
}

#[test]
fn test_approxidate_main_named_month_in_middle() {
    let mut tv = approxidate::TimeVal { tv_sec: 0, tv_usec: 0 };
    let rc = approxidate::approxidate_main("10 march 2013 04:00:07 -0500", &mut tv);
    assert_eq!(rc, 0);
    assert_eq!(tv.tv_sec, 1362906007);
}

#[test]
fn test_approxidate_main_year_first() {
    let mut tv = approxidate::TimeVal { tv_sec: 0, tv_usec: 0 };
    let rc = approxidate::approxidate_main("2013 march 10 04:00:07 -0500", &mut tv);
    assert_eq!(rc, 0);
    assert_eq!(tv.tv_sec, 1362906007);
}

fn main() {}
