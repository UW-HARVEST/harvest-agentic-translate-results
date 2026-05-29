#![allow(unused_imports)]
use approxidate::approxidate;

fn make_tm(hour: i32) -> approxidate::Atm {
    approxidate::Atm {
        tm_sec: 0, tm_min: 0, tm_hour: hour, tm_mday: 0, tm_mon: 0,
        tm_year: 0, tm_wday: 0, tm_yday: 0, tm_isdst: 0, tm_usec: 0,
    }
}

fn empty_now() -> approxidate::Atm { make_tm(0) }

#[test]
fn test_date_pm_no_num_morning_hour() {
    // hour=5, num=0 -> hour = (5%12)+12 = 17
    let mut tm = make_tm(5);
    let mut now = empty_now();
    let mut num = 0;
    approxidate::date_pm(&mut tm, &mut now, &mut num);
    assert_eq!(tm.tm_hour, 17);
    assert_eq!(num, 0);
}

#[test]
fn test_date_pm_no_num_afternoon_hour() {
    // hour=13, num=0 -> (13%12)+12 = 1+12 = 13
    let mut tm = make_tm(13);
    let mut now = empty_now();
    let mut num = 0;
    approxidate::date_pm(&mut tm, &mut now, &mut num);
    assert_eq!(tm.tm_hour, 13);
}

#[test]
fn test_date_pm_no_num_noon() {
    // hour=12, num=0 -> (12%12)+12 = 12
    let mut tm = make_tm(12);
    let mut now = empty_now();
    let mut num = 0;
    approxidate::date_pm(&mut tm, &mut now, &mut num);
    assert_eq!(tm.tm_hour, 12);
}

#[test]
fn test_date_pm_with_num() {
    // num=7, hour=0 -> hour=7 then PM -> (7%12)+12 = 19
    let mut tm = make_tm(0);
    let mut now = empty_now();
    let mut num = 7;
    approxidate::date_pm(&mut tm, &mut now, &mut num);
    assert_eq!(tm.tm_hour, 19);
    assert_eq!(num, 0);
    assert_eq!(tm.tm_min, 0);
    assert_eq!(tm.tm_sec, 0);
}

#[test]
fn test_date_am_no_num_morning() {
    let mut tm = make_tm(5);
    let mut now = empty_now();
    let mut num = 0;
    approxidate::date_am(&mut tm, &mut now, &mut num);
    assert_eq!(tm.tm_hour, 5);
}

#[test]
fn test_date_am_no_num_pm_hour() {
    // hour=13, num=0 -> 13%12 = 1
    let mut tm = make_tm(13);
    let mut now = empty_now();
    let mut num = 0;
    approxidate::date_am(&mut tm, &mut now, &mut num);
    assert_eq!(tm.tm_hour, 1);
}

#[test]
fn test_date_am_with_num_7() {
    // num=7 -> hour=7, AM -> 7%12 = 7
    let mut tm = make_tm(0);
    let mut now = empty_now();
    let mut num = 7;
    approxidate::date_am(&mut tm, &mut now, &mut num);
    assert_eq!(tm.tm_hour, 7);
    assert_eq!(num, 0);
}

#[test]
fn test_date_am_with_num_12() {
    // num=12 -> hour=12, AM -> 12%12 = 0
    let mut tm = make_tm(0);
    let mut now = empty_now();
    let mut num = 12;
    approxidate::date_am(&mut tm, &mut now, &mut num);
    assert_eq!(tm.tm_hour, 0);
    assert_eq!(num, 0);
}

fn main() {}
