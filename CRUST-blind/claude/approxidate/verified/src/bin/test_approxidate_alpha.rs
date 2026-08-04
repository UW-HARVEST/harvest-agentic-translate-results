#![allow(unused_imports)]
use approxidate::approxidate;

/// Build a tm/now pair representing 2020-01-01 12:30:17 -08:00 = 1577910617 UTC
/// in localtime. Since the test harness runs in UTC, localtime_r(1577910617)
/// produces tm_year=120, tm_mon=0, tm_mday=1, tm_hour=20, tm_min=30, tm_sec=17,
/// tm_wday=3 (Wed).
fn make_tm_now() -> (approxidate::Atm, approxidate::Atm) {
    let now = approxidate::Atm {
        tm_sec: 17, tm_min: 30, tm_hour: 20,
        tm_mday: 1, tm_mon: 0, tm_year: 120,
        tm_wday: 3, tm_yday: 0, tm_isdst: 0,
        tm_usec: 0,
    };
    let mut tm = approxidate::Atm {
        tm_sec: 17, tm_min: 30, tm_hour: 20,
        tm_mday: 1, tm_mon: 0, tm_year: 120,
        tm_wday: 3, tm_yday: 0, tm_isdst: 0,
        tm_usec: 0,
    };
    tm.tm_year = -1;
    tm.tm_mon = -1;
    tm.tm_mday = -1;
    (tm, now)
}

#[test]
fn test_approxidate_alpha_january_sets_month() {
    let (mut tm, mut now) = make_tm_now();
    let num: i32 = 0;
    let touched: i32 = 0;
    let res = approxidate::approxidate_alpha("January", &mut tm, &mut now, &num, &touched);
    assert_eq!(res, "");
    // Month should be set even though num/touched aren't propagated
    assert_eq!(tm.tm_mon, 0);
}

#[test]
fn test_approxidate_alpha_january_with_remainder() {
    let (mut tm, mut now) = make_tm_now();
    let num: i32 = 0;
    let touched: i32 = 0;
    let res = approxidate::approxidate_alpha("January 5", &mut tm, &mut now, &num, &touched);
    // After "January", end_idx points at ' ' so the remainder is " 5"
    assert_eq!(res, " 5");
    assert_eq!(tm.tm_mon, 0);
}

#[test]
fn test_approxidate_alpha_noon_sets_hour() {
    // noon -> date_time(tm, now, 12). tm_hour=20 >= 12, so no yesterday-shift.
    // Then tm_hour=12, tm_min=0, tm_sec=0.
    let (mut tm, mut now) = make_tm_now();
    let num: i32 = 0;
    let touched: i32 = 0;
    let res = approxidate::approxidate_alpha("noon", &mut tm, &mut now, &num, &touched);
    assert_eq!(res, "");
    assert_eq!(tm.tm_hour, 12);
    assert_eq!(tm.tm_min, 0);
    assert_eq!(tm.tm_sec, 0);
}

#[test]
fn test_approxidate_alpha_garbage_does_not_set_anything() {
    let (mut tm, mut now) = make_tm_now();
    let num: i32 = 0;
    let touched: i32 = 0;
    // "garbage abc": skip past alpha sequence, returns " abc"
    let res = approxidate::approxidate_alpha("garbage abc", &mut tm, &mut now, &num, &touched);
    assert_eq!(res, " abc");
    // tm fields shouldn't be set by garbage
    assert_eq!(tm.tm_mon, -1);
    assert_eq!(tm.tm_year, -1);
    assert_eq!(tm.tm_mday, -1);
}

fn main() {}
