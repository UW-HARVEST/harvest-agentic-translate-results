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
fn test_match_digit_two_digit_day() {
    // "10" -> num=10, n=2 → 1<=10<32 → tm_mday=10
    let mut tm = fresh_tm();
    let mut offset = -1;
    let mut tm_gmt = 0;
    let r = approxidate::match_digit("10", &mut tm, &mut offset, &mut tm_gmt);
    assert_eq!(r, 2);
    assert_eq!(tm.tm_mday, 10);
    assert_eq!(tm.tm_year, -1);
    assert_eq!(tm.tm_mon, -1);
    assert_eq!(offset, -1);
    assert_eq!(tm_gmt, 0);
}

#[test]
fn test_match_digit_year_2013() {
    let mut tm = fresh_tm();
    let mut offset = -1;
    let mut tm_gmt = 0;
    let r = approxidate::match_digit("2013", &mut tm, &mut offset, &mut tm_gmt);
    assert_eq!(r, 4);
    assert_eq!(tm.tm_year, 113);
    assert_eq!(offset, -1);
}

#[test]
fn test_match_digit_4digit_offset_low() {
    // 0500: <= 1400, offset == -1 → hours=5, min=0, offset = 300
    let mut tm = fresh_tm();
    let mut offset = -1;
    let mut tm_gmt = 0;
    let r = approxidate::match_digit("0500", &mut tm, &mut offset, &mut tm_gmt);
    assert_eq!(r, 4);
    assert_eq!(offset, 300);
    assert_eq!(tm.tm_year, -1);
}

#[test]
fn test_match_digit_4digit_no_action() {
    // 1500: not <= 1400, not in (1900, 2100) → no change to year nor offset
    let mut tm = fresh_tm();
    let mut offset = -1;
    let mut tm_gmt = 0;
    let r = approxidate::match_digit("1500", &mut tm, &mut offset, &mut tm_gmt);
    assert_eq!(r, 4);
    assert_eq!(offset, -1);
    assert_eq!(tm.tm_year, -1);
}

#[test]
fn test_match_digit_8digit_no_special_action() {
    // 20130610 → not >= 100000000 (only > 100M triggers). n=8, > 2 → return n.
    let mut tm = fresh_tm();
    let mut offset = -1;
    let mut tm_gmt = 0;
    let r = approxidate::match_digit("20130610", &mut tm, &mut offset, &mut tm_gmt);
    assert_eq!(r, 8);
    assert_eq!(tm.tm_year, -1);
    assert_eq!(tm.tm_mon, -1);
    assert_eq!(tm.tm_mday, -1);
}

#[test]
fn test_match_digit_unix_timestamp() {
    // 1234567890 (>= 100000000), nodate true, gmtime applies
    // gmtime(1234567890) = 2009-02-13 23:31:30 UTC
    let mut tm = fresh_tm();
    let mut offset = -1;
    let mut tm_gmt = 0;
    let r = approxidate::match_digit("1234567890", &mut tm, &mut offset, &mut tm_gmt);
    assert_eq!(r, 10);
    assert_eq!(tm.tm_year, 109);
    assert_eq!(tm.tm_mon, 1);
    assert_eq!(tm.tm_mday, 13);
    assert_eq!(tm.tm_hour, 23);
    assert_eq!(tm.tm_min, 31);
    assert_eq!(tm.tm_sec, 30);
    assert_eq!(tm_gmt, 1);
}

#[test]
fn test_match_digit_two_digit_year_high() {
    // "85": > 70 → tm_year = 85 (after exhausting other matches)
    // Actually n=2, num<32 → tm_mday=85? No: 85 >= 32 → skip mday.
    // Then n==2 && tm_year<0: num<10 false, num>=70 true → tm_year=85.
    let mut tm = fresh_tm();
    let mut offset = -1;
    let mut tm_gmt = 0;
    let r = approxidate::match_digit("85", &mut tm, &mut offset, &mut tm_gmt);
    assert_eq!(r, 2);
    assert_eq!(tm.tm_year, 85);
}

#[test]
fn test_match_digit_two_digit_month_when_mday_set() {
    // num=12 with no prior mday -> mday=12 (precedence)
    let mut tm = fresh_tm();
    let mut offset = -1;
    let mut tm_gmt = 0;
    let r = approxidate::match_digit("12", &mut tm, &mut offset, &mut tm_gmt);
    assert_eq!(r, 2);
    assert_eq!(tm.tm_mday, 12);
    assert_eq!(tm.tm_mon, -1);
}

#[test]
fn test_match_digit_two_digit_year_low_with_mday_set() {
    // tm_mday already set, num=05 (< 10), n=2 → tm_year = 05+100 = 105
    let mut tm = fresh_tm();
    tm.tm_mday = 10;
    let mut offset = -1;
    let mut tm_gmt = 0;
    let r = approxidate::match_digit("05", &mut tm, &mut offset, &mut tm_gmt);
    assert_eq!(r, 2);
    assert_eq!(tm.tm_year, 105);
}

#[test]
fn test_match_digit_one_digit() {
    // "5": num=5, n=1, num<32 → tm_mday=5
    let mut tm = fresh_tm();
    let mut offset = -1;
    let mut tm_gmt = 0;
    let r = approxidate::match_digit("5", &mut tm, &mut offset, &mut tm_gmt);
    assert_eq!(r, 1);
    assert_eq!(tm.tm_mday, 5);
}

#[test]
fn test_match_digit_with_colon_time() {
    // "12:30:45" -> match_multi_number for ':' -> sets time
    let mut tm = fresh_tm();
    let mut offset = -1;
    let mut tm_gmt = 0;
    let r = approxidate::match_digit("12:30:45", &mut tm, &mut offset, &mut tm_gmt);
    assert_eq!(r, 8);
    assert_eq!(tm.tm_hour, 12);
    assert_eq!(tm.tm_min, 30);
    assert_eq!(tm.tm_sec, 45);
}

#[test]
fn test_match_digit_iso_date() {
    // "2013-03-10" → multi-number with '-' → sets date (yyyy-mm-dd)
    let mut tm = fresh_tm();
    let mut offset = -1;
    let mut tm_gmt = 0;
    let r = approxidate::match_digit("2013-03-10", &mut tm, &mut offset, &mut tm_gmt);
    assert_eq!(r, 10);
    assert_eq!(tm.tm_year, 113);
    assert_eq!(tm.tm_mon, 2);
    assert_eq!(tm.tm_mday, 10);
}

fn main() {}
