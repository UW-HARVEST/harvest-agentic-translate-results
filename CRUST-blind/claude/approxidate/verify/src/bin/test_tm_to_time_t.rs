#![allow(unused_imports)]
use approxidate::approxidate;

fn make_atm(year: i32, mon: i32, mday: i32, hour: i32, min: i32, sec: i32) -> approxidate::Atm {
    approxidate::Atm {
        tm_sec: sec, tm_min: min, tm_hour: hour, tm_mday: mday, tm_mon: mon,
        tm_year: year, tm_wday: 0, tm_yday: 0, tm_isdst: -1, tm_usec: 0,
    }
}

#[test]
fn test_tm_to_time_t_2013_03_10_000007() {
    // 2013-03-10 00:00:07 UTC; tm_year=113, tm_mon=2, tm_mday=10
    let tm = make_atm(113, 2, 10, 0, 0, 7);
    assert_eq!(approxidate::tm_to_time_t(&tm), Some(1362873607));
}

#[test]
fn test_tm_to_time_t_epoch() {
    // 1970-01-01 00:00:00; tm_year=70, tm_mon=0, tm_mday=1
    let tm = make_atm(70, 0, 1, 0, 0, 0);
    assert_eq!(approxidate::tm_to_time_t(&tm), Some(0));
}

#[test]
fn test_tm_to_time_t_year_2000_jan() {
    // Y2K Jan 1 12:00:00; tm_year=100, tm_mon=0, tm_mday=1
    let tm = make_atm(100, 0, 1, 12, 0, 0);
    assert_eq!(approxidate::tm_to_time_t(&tm), Some(946728000));
}

#[test]
fn test_tm_to_time_t_year_too_big() {
    // year - 70 = 130 > 129 -> -1
    let tm = make_atm(200, 0, 1, 0, 0, 0);
    assert_eq!(approxidate::tm_to_time_t(&tm), Some(-1));
}

#[test]
fn test_tm_to_time_t_negative_month() {
    let tm = make_atm(113, -1, 1, 0, 0, 0);
    assert_eq!(approxidate::tm_to_time_t(&tm), Some(-1));
}

#[test]
fn test_tm_to_time_t_month_too_big() {
    let tm = make_atm(113, 12, 1, 0, 0, 0);
    assert_eq!(approxidate::tm_to_time_t(&tm), Some(-1));
}

#[test]
fn test_tm_to_time_t_2013_12_31() {
    // 2013-12-31 23:59:59 UTC
    let tm = make_atm(113, 11, 31, 23, 59, 59);
    assert_eq!(approxidate::tm_to_time_t(&tm), Some(1388534399));
}

#[test]
fn test_tm_to_time_t_negative_year() {
    // year - 70 < 0 -> -1
    let tm = make_atm(0, 0, 1, 0, 0, 0);
    assert_eq!(approxidate::tm_to_time_t(&tm), Some(-1));
}

#[test]
fn test_tm_to_time_t_negative_hour() {
    let tm = make_atm(113, 2, 10, -1, 0, 0);
    assert_eq!(approxidate::tm_to_time_t(&tm), Some(-1));
}

fn main() {}
