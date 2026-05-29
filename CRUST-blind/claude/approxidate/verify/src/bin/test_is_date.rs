#![allow(unused_imports)]
use approxidate::approxidate;

fn fresh_tm() -> approxidate::Atm {
    approxidate::Atm {
        tm_sec: -1, tm_min: -1, tm_hour: -1,
        tm_mday: -1, tm_mon: -1, tm_year: -1,
        tm_wday: 0, tm_yday: 0, tm_isdst: -1,
        tm_usec: 0,
    }
}

fn dummy_now() -> approxidate::Atm {
    approxidate::Atm {
        tm_sec: 0, tm_min: 0, tm_hour: 0,
        tm_mday: 0, tm_mon: 0, tm_year: 0,
        tm_wday: 0, tm_yday: 0, tm_isdst: 0,
        tm_usec: 0,
    }
}

#[test]
fn test_is_date_basic() {
    let now_tm = dummy_now();
    let mut tm = fresh_tm();
    let r = approxidate::is_date(2013, 3, 10, &now_tm, 0, &mut tm);
    assert_eq!(r, 1);
    assert_eq!(tm.tm_year, 113);
    assert_eq!(tm.tm_mon, 2);
    assert_eq!(tm.tm_mday, 10);
}

#[test]
fn test_is_date_2digit_year_high_70_99() {
    // year 95: > 70 && < 100 => tm_year = year (i.e. 95)
    let now_tm = dummy_now();
    let mut tm = fresh_tm();
    let r = approxidate::is_date(95, 6, 15, &now_tm, 0, &mut tm);
    assert_eq!(r, 1);
    assert_eq!(tm.tm_year, 95);
    assert_eq!(tm.tm_mon, 5);
    assert_eq!(tm.tm_mday, 15);
}

#[test]
fn test_is_date_2digit_year_low_lt_38() {
    // year 5: < 38 => tm_year = year + 100 = 105
    let now_tm = dummy_now();
    let mut tm = fresh_tm();
    let r = approxidate::is_date(5, 6, 15, &now_tm, 0, &mut tm);
    assert_eq!(r, 1);
    assert_eq!(tm.tm_year, 105);
    assert_eq!(tm.tm_mon, 5);
    assert_eq!(tm.tm_mday, 15);
}

#[test]
fn test_is_date_year_in_dead_range() {
    // year 50: not [1970,2099], not (70,99], not <38 => 0
    // (also tm.tm_year unchanged but month/day will have been set already in C path)
    let now_tm = dummy_now();
    let mut tm = fresh_tm();
    let r = approxidate::is_date(50, 6, 15, &now_tm, 0, &mut tm);
    assert_eq!(r, 0);
    // C sets r->tm_mon = month-1 and r->tm_mday = day before checking year,
    // and the rust impl also follows this pattern when now_tm is "null".
    assert_eq!(tm.tm_mon, 5);
    assert_eq!(tm.tm_mday, 15);
    // Year unchanged (not set since year fell in dead range)
    assert_eq!(tm.tm_year, -1);
}

#[test]
fn test_is_date_year_minus_one() {
    // year == -1 returns 1 without setting tm.tm_year (when now_tm is null).
    let now_tm = dummy_now();
    let mut tm = fresh_tm();
    let r = approxidate::is_date(-1, 6, 15, &now_tm, 0, &mut tm);
    assert_eq!(r, 1);
    assert_eq!(tm.tm_year, -1);
    assert_eq!(tm.tm_mon, 5);
    assert_eq!(tm.tm_mday, 15);
}

#[test]
fn test_is_date_month_too_big() {
    let now_tm = dummy_now();
    let mut tm = fresh_tm();
    let r = approxidate::is_date(2013, 13, 10, &now_tm, 0, &mut tm);
    assert_eq!(r, 0);
}

#[test]
fn test_is_date_month_zero() {
    let now_tm = dummy_now();
    let mut tm = fresh_tm();
    let r = approxidate::is_date(2013, 0, 10, &now_tm, 0, &mut tm);
    assert_eq!(r, 0);
}

#[test]
fn test_is_date_day_too_big() {
    let now_tm = dummy_now();
    let mut tm = fresh_tm();
    let r = approxidate::is_date(2013, 3, 32, &now_tm, 0, &mut tm);
    assert_eq!(r, 0);
}

#[test]
fn test_is_date_day_zero() {
    let now_tm = dummy_now();
    let mut tm = fresh_tm();
    let r = approxidate::is_date(2013, 3, 0, &now_tm, 0, &mut tm);
    assert_eq!(r, 0);
}

fn main() {}
