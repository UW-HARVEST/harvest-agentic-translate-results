#![allow(unused_imports)]
use approxidate::approxidate;

fn make_tm(y: i32, m: i32, d: i32, h: i32, mn: i32, s: i32) -> approxidate::Atm {
    approxidate::Atm {
        tm_sec: s, tm_min: mn, tm_hour: h, tm_mday: d, tm_mon: m,
        tm_year: y, tm_wday: 0, tm_yday: 0, tm_isdst: 0, tm_usec: 0,
    }
}

#[test]
fn test_nodate_all_negative() {
    // All -1: AND of -1 bitwise = -1 < 0 => 1
    let mut tm = make_tm(-1, -1, -1, -1, -1, -1);
    assert_eq!(approxidate::nodate(&mut tm), 1);
}

#[test]
fn test_nodate_one_zero_year() {
    // tm_year=0 (any non-negative), AND result becomes 0, not < 0 => 0
    let mut tm = make_tm(0, -1, -1, -1, -1, -1);
    assert_eq!(approxidate::nodate(&mut tm), 0);
}

#[test]
fn test_nodate_one_zero_sec() {
    let mut tm = make_tm(-1, -1, -1, -1, -1, 0);
    assert_eq!(approxidate::nodate(&mut tm), 0);
}

#[test]
fn test_nodate_all_zero() {
    let mut tm = make_tm(0, 0, 0, 0, 0, 0);
    assert_eq!(approxidate::nodate(&mut tm), 0);
}

fn main() {}
