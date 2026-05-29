#![allow(unused_imports)]
use approxidate::approxidate;

fn make_tm(y: i32, m: i32, d: i32) -> approxidate::Atm {
    approxidate::Atm {
        tm_sec: -1, tm_min: -1, tm_hour: -1,
        tm_mday: d, tm_mon: m, tm_year: y,
        tm_wday: 0, tm_yday: 0, tm_isdst: -1,
        tm_usec: 0,
    }
}

#[test]
fn test_pending_number_assigns_mday() {
    // tm_mday=-1 and num<32 -> tm_mday=num
    let mut tm = make_tm(-1, -1, -1);
    let num: i32 = 25;
    approxidate::pending_number(&mut tm, &num);
    assert_eq!(tm.tm_mday, 25);
    assert_eq!(tm.tm_mon, -1);
    assert_eq!(tm.tm_year, -1);
}

#[test]
fn test_pending_number_assigns_month_when_mday_set() {
    // mday=25, mon=-1, num=12 (<13) -> tm_mon = 11
    let mut tm = make_tm(-1, -1, 25);
    let num: i32 = 12;
    approxidate::pending_number(&mut tm, &num);
    assert_eq!(tm.tm_mday, 25);
    assert_eq!(tm.tm_mon, 11);
    assert_eq!(tm.tm_year, -1);
}

#[test]
fn test_pending_number_assigns_4digit_year() {
    let mut tm = make_tm(-1, -1, -1);
    let num: i32 = 2025;
    approxidate::pending_number(&mut tm, &num);
    // 2025 is not <32, not <13. tm_year=-1, > 1969 && < 2100, so year = 125
    assert_eq!(tm.tm_year, 125);
    // mday and mon untouched
    assert_eq!(tm.tm_mday, -1);
    assert_eq!(tm.tm_mon, -1);
}

#[test]
fn test_pending_number_assigns_2digit_year_high() {
    // num=85: not<32, not<13, year<0; >69 && <100 -> tm_year=85
    let mut tm = make_tm(-1, -1, -1);
    let num: i32 = 85;
    approxidate::pending_number(&mut tm, &num);
    assert_eq!(tm.tm_year, 85);
}

#[test]
fn test_pending_number_assigns_2digit_year_low_when_mday_taken() {
    // tm_mday set, tm_mon set; num=25 -> not < 32 first since mday set so mday<0 false
    // Then mon<0? false. Then year<0? true. 25 not >1969, not >69, but <38 -> tm_year=125
    let mut tm = make_tm(-1, 5, 25);
    let num: i32 = 25;
    approxidate::pending_number(&mut tm, &num);
    assert_eq!(tm.tm_year, 125);
}

#[test]
fn test_pending_number_zero_does_nothing() {
    let mut tm = make_tm(-1, -1, -1);
    let num: i32 = 0;
    approxidate::pending_number(&mut tm, &num);
    assert_eq!(tm.tm_year, -1);
    assert_eq!(tm.tm_mon, -1);
    assert_eq!(tm.tm_mday, -1);
}

#[test]
fn test_pending_number_no_op_when_all_set() {
    // year=113, mon=5, mday=25 are all >= 0; num=12 won't match any branch
    let mut tm = make_tm(113, 5, 25);
    let num: i32 = 12;
    approxidate::pending_number(&mut tm, &num);
    assert_eq!(tm.tm_year, 113);
    assert_eq!(tm.tm_mon, 5);
    assert_eq!(tm.tm_mday, 25);
}

fn main() {}
