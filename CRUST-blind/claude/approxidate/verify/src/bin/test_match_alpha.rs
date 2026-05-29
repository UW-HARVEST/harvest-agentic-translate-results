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
fn test_match_alpha_full_january() {
    let mut tm = fresh_tm();
    let mut offset = -1;
    let r = approxidate::match_alpha("January", &mut tm, &mut offset);
    assert_eq!(r, 7);
    assert_eq!(tm.tm_mon, 0);
    assert_eq!(offset, -1);
}

#[test]
fn test_match_alpha_short_jan() {
    let mut tm = fresh_tm();
    let mut offset = -1;
    let r = approxidate::match_alpha("Jan 5", &mut tm, &mut offset);
    assert_eq!(r, 3);
    assert_eq!(tm.tm_mon, 0);
}

#[test]
fn test_match_alpha_weekday_long() {
    let mut tm = fresh_tm();
    let mut offset = -1;
    let r = approxidate::match_alpha("Mondays", &mut tm, &mut offset);
    assert_eq!(r, 7);
    assert_eq!(tm.tm_wday, 1);
}

#[test]
fn test_match_alpha_weekday_short() {
    let mut tm = fresh_tm();
    let mut offset = -1;
    let r = approxidate::match_alpha("Mon 5", &mut tm, &mut offset);
    assert_eq!(r, 3);
    assert_eq!(tm.tm_wday, 1);
}

#[test]
fn test_match_alpha_utc() {
    let mut tm = fresh_tm();
    let mut offset = -1;
    let r = approxidate::match_alpha("UTC", &mut tm, &mut offset);
    assert_eq!(r, 3);
    assert_eq!(offset, 0);
}

#[test]
fn test_match_alpha_z_zulu() {
    // 'Z' has length 1; match_string returns 1, which equals the string length,
    // so tz match still applies even though < 3.
    let mut tm = fresh_tm();
    let mut offset = -1;
    let r = approxidate::match_alpha("Z", &mut tm, &mut offset);
    assert_eq!(r, 1);
    assert_eq!(offset, 0);
}

#[test]
fn test_match_alpha_pst() {
    let mut tm = fresh_tm();
    let mut offset = -1;
    let r = approxidate::match_alpha("PST", &mut tm, &mut offset);
    assert_eq!(r, 3);
    // -8 hours = -480 minutes
    assert_eq!(offset, -480);
}

#[test]
fn test_match_alpha_pdt_includes_dst_bonus() {
    let mut tm = fresh_tm();
    let mut offset = -1;
    let r = approxidate::match_alpha("PDT", &mut tm, &mut offset);
    assert_eq!(r, 3);
    // PDT offset -8, dst=1 => -8 + 1 = -7 hours = -420
    assert_eq!(offset, -420);
}

#[test]
fn test_match_alpha_pm_with_existing_hour() {
    let mut tm = fresh_tm();
    tm.tm_hour = 5;
    let mut offset = -1;
    let r = approxidate::match_alpha("PM", &mut tm, &mut offset);
    assert_eq!(r, 2);
    // (5 % 12) + 12 = 17
    assert_eq!(tm.tm_hour, 17);
}

#[test]
fn test_match_alpha_am_with_hour_13() {
    let mut tm = fresh_tm();
    tm.tm_hour = 13;
    let mut offset = -1;
    let r = approxidate::match_alpha("AM", &mut tm, &mut offset);
    assert_eq!(r, 2);
    // 13 % 12 = 1
    assert_eq!(tm.tm_hour, 1);
}

#[test]
fn test_match_alpha_garbage_skip_alpha() {
    let mut tm = fresh_tm();
    let mut offset = -1;
    let r = approxidate::match_alpha("garbage", &mut tm, &mut offset);
    assert_eq!(r, 7);
    assert_eq!(tm.tm_mon, -1);
    assert_eq!(tm.tm_wday, 0);
    assert_eq!(offset, -1);
}

#[test]
fn test_match_alpha_bst_dst_only() {
    // BST: offset 0, dst 1 => offset = 1*60 = 60
    let mut tm = fresh_tm();
    let mut offset = -1;
    let r = approxidate::match_alpha("BST", &mut tm, &mut offset);
    assert_eq!(r, 3);
    assert_eq!(offset, 60);
}

#[test]
fn test_match_alpha_offset_already_set() {
    // If offset != -1, it should not be overwritten by tz lookup.
    let mut tm = fresh_tm();
    let mut offset = 123;
    let r = approxidate::match_alpha("UTC", &mut tm, &mut offset);
    assert_eq!(r, 3);
    assert_eq!(offset, 123);
}

fn main() {}
