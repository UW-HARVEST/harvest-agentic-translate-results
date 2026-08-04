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
fn test_approxidate_digit_simple_number() {
    // "10" -> consumes all digits, result is the empty tail
    let mut tm = fresh_tm();
    let num: i32 = 0;
    let res = approxidate::approxidate_digit("10", &mut tm, &num, 0);
    assert_eq!(res, "");
    // Note: num is &i32 in this signature, so its update is not visible. But tm
    // is unchanged because the simple-number branch only updates num, not tm.
    assert_eq!(tm.tm_year, -1);
    assert_eq!(tm.tm_mon, -1);
    assert_eq!(tm.tm_mday, -1);
    assert_eq!(tm.tm_hour, -1);
}

#[test]
fn test_approxidate_digit_iso_date_eats_full_match() {
    // "10/Mar/2013" -> match_multi_number with c='/' picks up "10/Mar/2013"... but
    // the multi-num path requires digit after '/'. After '10' we see '/', and end[1]='M'
    // which is not a digit. So match_multi_number isn't entered; it falls through to
    // updating num=10, returning the tail starting from end of digits ("/Mar/2013").
    let mut tm = fresh_tm();
    let num: i32 = 0;
    let res = approxidate::approxidate_digit("10/Mar/2013", &mut tm, &num, 0);
    assert_eq!(res, "/Mar/2013");
}

#[test]
fn test_approxidate_digit_time_format() {
    // "12:30:45" -> match_multi_number eats it entirely and sets time
    let mut tm = fresh_tm();
    let num: i32 = 0;
    let res = approxidate::approxidate_digit("12:30:45", &mut tm, &num, 0);
    assert_eq!(res, "");
    assert_eq!(tm.tm_hour, 12);
    assert_eq!(tm.tm_min, 30);
    assert_eq!(tm.tm_sec, 45);
}

#[test]
fn test_approxidate_digit_zero_padded_no_num_update() {
    // "0500" -> 4 digits start with '0' and end-date == 4 (not <=2),
    // so num is NOT updated (stays 0). Result is the empty tail.
    let mut tm = fresh_tm();
    let num: i32 = 0;
    let res = approxidate::approxidate_digit("0500", &mut tm, &num, 0);
    assert_eq!(res, "");
}

fn main() {}
