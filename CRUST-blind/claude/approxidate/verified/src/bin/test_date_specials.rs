#![allow(unused_imports)]
use approxidate::approxidate;

/// Make a tm/now pair where now corresponds to localtime(1577910617) in UTC TZ.
/// Resulting now: 2020-01-01 20:30:17 UTC, wday=3 (Wed).
fn make_tm_now() -> (approxidate::Atm, approxidate::Atm) {
    let now = approxidate::Atm {
        tm_sec: 17, tm_min: 30, tm_hour: 20,
        tm_mday: 1, tm_mon: 0, tm_year: 120,
        tm_wday: 3, tm_yday: 0, tm_isdst: 0, tm_usec: 0,
    };
    // tm starts from now, then has y/m/d cleared.
    let mut tm = approxidate::Atm {
        tm_sec: 17, tm_min: 30, tm_hour: 20,
        tm_mday: 1, tm_mon: 0, tm_year: 120,
        tm_wday: 3, tm_yday: 0, tm_isdst: 0, tm_usec: 0,
    };
    tm.tm_year = -1;
    tm.tm_mon = -1;
    tm.tm_mday = -1;
    (tm, now)
}

#[test]
fn test_date_now_fills_date_from_now() {
    let (mut tm, mut now) = make_tm_now();
    let mut num = 0;
    approxidate::date_now(&mut tm, &mut now, &mut num);
    assert_eq!(tm.tm_year, 120);
    assert_eq!(tm.tm_mon, 0);
    assert_eq!(tm.tm_mday, 1);
    assert_eq!(tm.tm_hour, 20);
    assert_eq!(tm.tm_min, 30);
    assert_eq!(tm.tm_sec, 17);
}

#[test]
fn test_date_yesterday_subtracts_a_day() {
    let (mut tm, mut now) = make_tm_now();
    let mut num = 0;
    approxidate::date_yesterday(&mut tm, &mut now, &mut num);
    // 2020-01-01 -> 2019-12-31
    assert_eq!(tm.tm_year, 119);
    assert_eq!(tm.tm_mon, 11);
    assert_eq!(tm.tm_mday, 31);
    assert_eq!(tm.tm_hour, 20);
    assert_eq!(tm.tm_min, 30);
    assert_eq!(tm.tm_sec, 17);
}

#[test]
fn test_date_midnight_sets_zero_no_shift() {
    let (mut tm, mut now) = make_tm_now();
    let mut num = 0;
    approxidate::date_midnight(&mut tm, &mut now, &mut num);
    // tm_hour=20 < 0? No -> no yesterday-shift; date doesn't get filled by
    // update_tm for midnight (since update_tm isn't called).
    assert_eq!(tm.tm_hour, 0);
    assert_eq!(tm.tm_min, 0);
    assert_eq!(tm.tm_sec, 0);
    assert_eq!(tm.tm_mday, -1);
    assert_eq!(tm.tm_mon, -1);
    assert_eq!(tm.tm_year, -1);
}

#[test]
fn test_date_noon_with_evening_hour_no_shift() {
    let (mut tm, mut now) = make_tm_now();
    let mut num = 0;
    approxidate::date_noon(&mut tm, &mut now, &mut num);
    // tm_hour=20 not < 12 → no yesterday call → no date filling
    assert_eq!(tm.tm_hour, 12);
    assert_eq!(tm.tm_min, 0);
    assert_eq!(tm.tm_sec, 0);
    assert_eq!(tm.tm_mday, -1);
}

#[test]
fn test_date_tea_with_evening_hour_no_shift() {
    let (mut tm, mut now) = make_tm_now();
    let mut num = 0;
    approxidate::date_tea(&mut tm, &mut now, &mut num);
    // tm_hour=20 not < 17 → no yesterday call
    assert_eq!(tm.tm_hour, 17);
    assert_eq!(tm.tm_min, 0);
    assert_eq!(tm.tm_sec, 0);
    assert_eq!(tm.tm_mday, -1);
}

#[test]
fn test_date_noon_with_morning_hour_shifts_to_yesterday() {
    let (mut tm, mut now) = make_tm_now();
    tm.tm_hour = 5;
    let mut num = 0;
    approxidate::date_noon(&mut tm, &mut now, &mut num);
    // tm_hour=5 < 12 → date_yesterday is called → date filled, then hour=12.
    assert_eq!(tm.tm_hour, 12);
    assert_eq!(tm.tm_year, 119);
    assert_eq!(tm.tm_mon, 11);
    assert_eq!(tm.tm_mday, 31);
}

#[test]
fn test_date_tea_with_morning_hour_shifts_to_yesterday() {
    let (mut tm, mut now) = make_tm_now();
    tm.tm_hour = 5;
    let mut num = 0;
    approxidate::date_tea(&mut tm, &mut now, &mut num);
    assert_eq!(tm.tm_hour, 17);
    assert_eq!(tm.tm_year, 119);
    assert_eq!(tm.tm_mon, 11);
    assert_eq!(tm.tm_mday, 31);
}

#[test]
fn test_date_never_resets_to_epoch() {
    let mut tm = approxidate::Atm {
        tm_sec: 0, tm_min: 30, tm_hour: 12, tm_mday: 15, tm_mon: 5,
        tm_year: 99, tm_wday: 0, tm_yday: 0, tm_isdst: 0, tm_usec: 0,
    };
    let mut now = approxidate::Atm {
        tm_sec: 0, tm_min: 0, tm_hour: 0, tm_mday: 0, tm_mon: 0,
        tm_year: 0, tm_wday: 0, tm_yday: 0, tm_isdst: 0, tm_usec: 0,
    };
    let mut num = 0;
    approxidate::date_never(&mut tm, &mut now, &mut num);
    // Localtime of 0 in UTC = 1970-01-01 00:00:00
    assert_eq!(tm.tm_year, 70);
    assert_eq!(tm.tm_mon, 0);
    assert_eq!(tm.tm_mday, 1);
    assert_eq!(tm.tm_hour, 0);
    assert_eq!(tm.tm_min, 0);
    assert_eq!(tm.tm_sec, 0);
}

#[test]
fn test_update_tm_no_offset_returns_same_seconds() {
    let (mut tm, mut now) = make_tm_now();
    let result = approxidate::update_tm(&mut tm, &mut now, 0);
    // update_tm should fill date from now, then mktime gives back 1577910617
    assert_eq!(result, 1577910617);
    assert_eq!(tm.tm_year, 120);
    assert_eq!(tm.tm_mon, 0);
    assert_eq!(tm.tm_mday, 1);
    assert_eq!(tm.tm_hour, 20);
    assert_eq!(tm.tm_min, 30);
    assert_eq!(tm.tm_sec, 17);
}

#[test]
fn test_update_tm_subtract_one_day() {
    let (mut tm, mut now) = make_tm_now();
    let result = approxidate::update_tm(&mut tm, &mut now, 86400);
    assert_eq!(result, 1577824217);
    // 2019-12-31 20:30:17
    assert_eq!(tm.tm_year, 119);
    assert_eq!(tm.tm_mon, 11);
    assert_eq!(tm.tm_mday, 31);
    assert_eq!(tm.tm_hour, 20);
    assert_eq!(tm.tm_min, 30);
    assert_eq!(tm.tm_sec, 17);
}

#[test]
fn test_approxidate_str_yesterday() {
    let mut tv = approxidate::TimeVal { tv_sec: 1577910617, tv_usec: 0 };
    let rc = approxidate::approxidate_str("yesterday", &mut tv);
    assert_eq!(rc, 0);
    assert_eq!(tv.tv_sec, 1577824217);
    assert_eq!(tv.tv_usec, 0);
}

#[test]
fn test_approxidate_str_noon() {
    let mut tv = approxidate::TimeVal { tv_sec: 1577910617, tv_usec: 0 };
    let rc = approxidate::approxidate_str("noon", &mut tv);
    assert_eq!(rc, 0);
    assert_eq!(tv.tv_sec, 1577880000);
}

#[test]
fn test_approxidate_str_empty_returns_minus_one() {
    let mut tv = approxidate::TimeVal { tv_sec: 1577910617, tv_usec: 0 };
    let rc = approxidate::approxidate_str("", &mut tv);
    assert_eq!(rc, -1);
    // tv unchanged when nothing was touched
    assert_eq!(tv.tv_sec, 1577910617);
}

fn main() {}
