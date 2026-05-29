#![allow(unused_imports)]
use approxidate::approxidate;

fn rel(sec: i64) -> approxidate::TimeVal {
    approxidate::TimeVal { tv_sec: sec, tv_usec: 0 }
}

#[test]
fn test_approxidate_relative_full_date() {
    let mut tv = approxidate::TimeVal { tv_sec: 0, tv_usec: 0 };
    let mut r = rel(1577910617);
    let rc = approxidate::approxidate_relative("10/Mar/2013:00:00:02 UTC", &mut tv, &mut r);
    assert_eq!(rc, 0);
    assert_eq!(tv.tv_sec, 1362873602);
    assert_eq!(tv.tv_usec, 0);
}

#[test]
fn test_approxidate_relative_yesterday() {
    let mut tv = approxidate::TimeVal { tv_sec: 0, tv_usec: 0 };
    let mut r = rel(1577910617);
    let rc = approxidate::approxidate_relative("yesterday", &mut tv, &mut r);
    assert_eq!(rc, 0);
    assert_eq!(tv.tv_sec, 1577824217);
}

#[test]
fn test_approxidate_relative_noon() {
    let mut tv = approxidate::TimeVal { tv_sec: 0, tv_usec: 0 };
    let mut r = rel(1577910617);
    let rc = approxidate::approxidate_relative("noon", &mut tv, &mut r);
    assert_eq!(rc, 0);
    assert_eq!(tv.tv_sec, 1577880000);
}

#[test]
fn test_approxidate_relative_midnight() {
    let mut tv = approxidate::TimeVal { tv_sec: 0, tv_usec: 0 };
    let mut r = rel(1577910617);
    let rc = approxidate::approxidate_relative("midnight", &mut tv, &mut r);
    assert_eq!(rc, 0);
    assert_eq!(tv.tv_sec, 1577836800);
}

#[test]
fn test_approxidate_relative_tea() {
    let mut tv = approxidate::TimeVal { tv_sec: 0, tv_usec: 0 };
    let mut r = rel(1577910617);
    let rc = approxidate::approxidate_relative("tea", &mut tv, &mut r);
    assert_eq!(rc, 0);
    assert_eq!(tv.tv_sec, 1577898000);
}

#[test]
fn test_approxidate_relative_now() {
    let mut tv = approxidate::TimeVal { tv_sec: 0, tv_usec: 0 };
    let mut r = rel(1577910617);
    let rc = approxidate::approxidate_relative("now", &mut tv, &mut r);
    assert_eq!(rc, 0);
    assert_eq!(tv.tv_sec, 1577910617);
}

#[test]
fn test_approxidate_relative_2_days_ago() {
    let mut tv = approxidate::TimeVal { tv_sec: 0, tv_usec: 0 };
    let mut r = rel(1577910617);
    let rc = approxidate::approxidate_relative("2 days ago", &mut tv, &mut r);
    assert_eq!(rc, 0);
    assert_eq!(tv.tv_sec, 1577737817);
}

#[test]
fn test_approxidate_relative_1_week_ago() {
    let mut tv = approxidate::TimeVal { tv_sec: 0, tv_usec: 0 };
    let mut r = rel(1577910617);
    let rc = approxidate::approxidate_relative("1 week ago", &mut tv, &mut r);
    assert_eq!(rc, 0);
    assert_eq!(tv.tv_sec, 1577305817);
}

#[test]
fn test_approxidate_relative_1_month_ago() {
    let mut tv = approxidate::TimeVal { tv_sec: 0, tv_usec: 0 };
    let mut r = rel(1577910617);
    let rc = approxidate::approxidate_relative("1 month ago", &mut tv, &mut r);
    assert_eq!(rc, 0);
    assert_eq!(tv.tv_sec, 1575232217);
}

#[test]
fn test_approxidate_relative_1_year_ago() {
    let mut tv = approxidate::TimeVal { tv_sec: 0, tv_usec: 0 };
    let mut r = rel(1577910617);
    let rc = approxidate::approxidate_relative("1 year ago", &mut tv, &mut r);
    assert_eq!(rc, 0);
    assert_eq!(tv.tv_sec, 1546374617);
}

#[test]
fn test_approxidate_relative_empty_returns_minus_one() {
    let mut tv = approxidate::TimeVal { tv_sec: 0, tv_usec: 0 };
    let mut r = rel(1577910617);
    let rc = approxidate::approxidate_relative("", &mut tv, &mut r);
    assert_eq!(rc, -1);
    // tv is set to relative_to since parse_date_basic failed
    assert_eq!(tv.tv_sec, 1577910617);
}

#[test]
fn test_approxidate_relative_one_one_2014() {
    let mut tv = approxidate::TimeVal { tv_sec: 0, tv_usec: 0 };
    let mut r = rel(1577910617);
    let rc = approxidate::approxidate_relative("1/1/2014", &mut tv, &mut r);
    assert_eq!(rc, 0);
    assert_eq!(tv.tv_sec, 1388608217);
}

#[test]
fn test_approxidate_relative_iso_with_usec_utc() {
    let mut tv = approxidate::TimeVal { tv_sec: 0, tv_usec: 0 };
    let mut r = rel(1577910617);
    let rc = approxidate::approxidate_relative("10/Mar/2013:00:00:02.003 UTC", &mut tv, &mut r);
    assert_eq!(rc, 0);
    assert_eq!(tv.tv_sec, 1362873602);
    assert_eq!(tv.tv_usec, 3000);
}

#[test]
fn test_approxidate_relative_iso_with_negative_tz() {
    let mut tv = approxidate::TimeVal { tv_sec: 0, tv_usec: 0 };
    let mut r = rel(1577910617);
    let rc = approxidate::approxidate_relative("10/Mar/2012:00:00:07.657891 -0110", &mut tv, &mut r);
    assert_eq!(rc, 0);
    assert_eq!(tv.tv_sec, 1331341807);
    assert_eq!(tv.tv_usec, 657891);
}

#[test]
fn test_approxidate_relative_iso_plus_1400() {
    let mut tv = approxidate::TimeVal { tv_sec: 0, tv_usec: 0 };
    let mut r = rel(1577910617);
    let rc = approxidate::approxidate_relative("10/Mar/2012:00:00:07.657891 +1400", &mut tv, &mut r);
    assert_eq!(rc, 0);
    assert_eq!(tv.tv_sec, 1331287207);
    assert_eq!(tv.tv_usec, 657891);
}

fn main() {}
