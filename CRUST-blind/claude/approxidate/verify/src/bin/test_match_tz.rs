#![allow(unused_imports)]
use approxidate::approxidate;

#[test]
fn test_match_tz_plus_4digit() {
    // +0500 -> hour=5, min=0, n=4 → offset = 300
    let mut off = -1;
    let r = approxidate::match_tz("+0500", &mut off);
    assert_eq!(r, 5);
    assert_eq!(off, 300);
}

#[test]
fn test_match_tz_minus_4digit() {
    let mut off = -1;
    let r = approxidate::match_tz("-0530", &mut off);
    assert_eq!(r, 5);
    assert_eq!(off, -330);
}

#[test]
fn test_match_tz_plus_zero() {
    let mut off = -1;
    let r = approxidate::match_tz("+0000", &mut off);
    assert_eq!(r, 5);
    assert_eq!(off, 0);
}

#[test]
fn test_match_tz_minus_1400() {
    let mut off = -1;
    let r = approxidate::match_tz("-1400", &mut off);
    assert_eq!(r, 5);
    assert_eq!(off, -840);
}

#[test]
fn test_match_tz_colon_form() {
    // +05:30 -> n=2, then colon -> min=30, end_pos=6, end_pos-1==5 → keep
    let mut off = -1;
    let r = approxidate::match_tz("+05:30 foo", &mut off);
    assert_eq!(r, 6);
    assert_eq!(off, 330);
}

#[test]
fn test_match_tz_2digit_only() {
    // +05 -> n=2, no colon, min=0, hour=5 → offset=300
    let mut off = -1;
    let r = approxidate::match_tz("+05", &mut off);
    assert_eq!(r, 3);
    assert_eq!(off, 300);
}

#[test]
fn test_match_tz_1digit_random_crap() {
    // +8 -> n=1, min=99, not updated, returns 2
    let mut off = -1;
    let r = approxidate::match_tz("+8", &mut off);
    assert_eq!(r, 2);
    assert_eq!(off, -1);
}

#[test]
fn test_match_tz_5digit_random_crap() {
    // +12345 -> n=5, min=99 → no update
    let mut off = -1;
    let r = approxidate::match_tz("+12345", &mut off);
    assert_eq!(r, 6);
    assert_eq!(off, -1);
}

fn main() {}
