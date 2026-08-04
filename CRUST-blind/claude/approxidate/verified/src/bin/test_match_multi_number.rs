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

#[test]
fn test_match_multi_number_hms() {
    // "12:30:45" with num=12 and c=':' starting after first colon
    let mut tm = fresh_tm();
    let r = approxidate::match_multi_number(12, ':', "12:30:45", ":30:45", &mut tm, 0);
    assert_eq!(r, 8);
    assert_eq!(tm.tm_hour, 12);
    assert_eq!(tm.tm_min, 30);
    assert_eq!(tm.tm_sec, 45);
    assert_eq!(tm.tm_usec, 0);
}

#[test]
fn test_match_multi_number_hm_only() {
    // No third number; sec defaults to 0
    let mut tm = fresh_tm();
    let r = approxidate::match_multi_number(12, ':', "12:30", ":30", &mut tm, 0);
    assert_eq!(r, 5);
    assert_eq!(tm.tm_hour, 12);
    assert_eq!(tm.tm_min, 30);
    assert_eq!(tm.tm_sec, 0);
}

#[test]
fn test_match_multi_number_with_micro() {
    let mut tm = fresh_tm();
    let r = approxidate::match_multi_number(12, ':', "12:30:45.123", ":30:45.123", &mut tm, 0);
    assert_eq!(r, 12);
    assert_eq!(tm.tm_sec, 45);
    // 0.123 sec -> 123000 microseconds (frac_len=3, mult 10^3 = 1000)
    assert_eq!(tm.tm_usec, 123000);
}

#[test]
fn test_match_multi_number_with_full_micro() {
    let mut tm = fresh_tm();
    let r = approxidate::match_multi_number(12, ':', "12:30:45.123456", ":30:45.123456", &mut tm, 0);
    assert_eq!(r, 15);
    assert_eq!(tm.tm_usec, 123456);
}

#[test]
fn test_match_multi_number_iso_date() {
    let mut tm = fresh_tm();
    let r = approxidate::match_multi_number(2013, '-', "2013-03-10", "-03-10", &mut tm, 0);
    assert_eq!(r, 10);
    assert_eq!(tm.tm_year, 113);
    assert_eq!(tm.tm_mon, 2);
    assert_eq!(tm.tm_mday, 10);
}

#[test]
fn test_match_multi_number_us_dash() {
    let mut tm = fresh_tm();
    // "10-03-2013" with num=10 → mm-dd-yy form (since c != '.', try US format first)
    let r = approxidate::match_multi_number(10, '-', "10-03-2013", "-03-2013", &mut tm, 0);
    assert_eq!(r, 10);
    assert_eq!(tm.tm_year, 113);
    assert_eq!(tm.tm_mon, 9);
    assert_eq!(tm.tm_mday, 3);
}

#[test]
fn test_match_multi_number_us_slash() {
    let mut tm = fresh_tm();
    let r = approxidate::match_multi_number(10, '/', "10/3/2013", "/3/2013", &mut tm, 0);
    assert_eq!(r, 9);
    assert_eq!(tm.tm_year, 113);
    assert_eq!(tm.tm_mon, 9);
    assert_eq!(tm.tm_mday, 3);
}

#[test]
fn test_match_multi_number_european_dot() {
    let mut tm = fresh_tm();
    // "10.3.2013" with num=10 → dd.mm.yyyy form
    let r = approxidate::match_multi_number(10, '.', "10.3.2013", ".3.2013", &mut tm, 0);
    assert_eq!(r, 9);
    assert_eq!(tm.tm_year, 113);
    assert_eq!(tm.tm_mon, 2);
    assert_eq!(tm.tm_mday, 10);
}

#[test]
fn test_match_multi_number_invalid_time_returns_zero() {
    // "25:99:99" - num=25 not <25 → 0
    let mut tm = fresh_tm();
    let r = approxidate::match_multi_number(25, ':', "25:99:99", ":99:99", &mut tm, 0);
    assert_eq!(r, 0);
}

#[test]
fn test_match_multi_number_min_60_returns_zero() {
    let mut tm = fresh_tm();
    let r = approxidate::match_multi_number(12, ':', "12:60", ":60", &mut tm, 0);
    assert_eq!(r, 0);
}

fn main() {}
