use approxidate::approxidate::*;

fn main() {}

// ---- approxidate_main (equivalent to C's approxidate) ----

#[test]
fn test_approxidate_main_utc_with_usec() {
    let mut tv = TimeVal { tv_sec: 0, tv_usec: 0 };
    let r = approxidate_main("10/Mar/2013:00:00:02.003 UTC", &mut tv);
    assert_eq!(r, 0);
    assert_eq!(tv.tv_sec, 1362873602);
    assert_eq!(tv.tv_usec, 3000);
}

#[test]
fn test_approxidate_main_utc_no_usec() {
    let mut tv = TimeVal { tv_sec: 0, tv_usec: 0 };
    let r = approxidate_main("10/Mar/2013:00:00:07 UTC", &mut tv);
    assert_eq!(r, 0);
    assert_eq!(tv.tv_sec, 1362873607);
    assert_eq!(tv.tv_usec, 0);
}

#[test]
fn test_approxidate_main_positive_offset() {
    let mut tv = TimeVal { tv_sec: 0, tv_usec: 0 };
    let r = approxidate_main("10/Mar/2012:00:00:07 +0500", &mut tv);
    assert_eq!(r, 0);
    assert_eq!(tv.tv_sec, 1331319607);
}

#[test]
fn test_approxidate_main_usec_positive_offset() {
    let mut tv = TimeVal { tv_sec: 0, tv_usec: 0 };
    let r = approxidate_main("10/Mar/2012:00:00:07.657891 +0500", &mut tv);
    assert_eq!(r, 0);
    assert_eq!(tv.tv_sec, 1331319607);
    assert_eq!(tv.tv_usec, 657891);
}

#[test]
fn test_approxidate_main_usec_large_offset() {
    let mut tv = TimeVal { tv_sec: 0, tv_usec: 0 };
    let r = approxidate_main("10/Mar/2012:00:00:07.657891 +1400", &mut tv);
    assert_eq!(r, 0);
    assert_eq!(tv.tv_sec, 1331287207);
    assert_eq!(tv.tv_usec, 657891);
}

#[test]
fn test_approxidate_main_usec_neg_offset() {
    let mut tv = TimeVal { tv_sec: 0, tv_usec: 0 };
    let r = approxidate_main("10/Mar/2012:00:00:07.657891 -0110", &mut tv);
    assert_eq!(r, 0);
    assert_eq!(tv.tv_sec, 1331341807);
    assert_eq!(tv.tv_usec, 657891);
}

#[test]
fn test_approxidate_main_month_day_year() {
    let mut tv = TimeVal { tv_sec: 0, tv_usec: 0 };
    let r = approxidate_main("mar 10 2013 00:00:07 UTC", &mut tv);
    assert_eq!(r, 0);
    assert_eq!(tv.tv_sec, 1362873607);
}

#[test]
fn test_approxidate_main_full_month() {
    let mut tv = TimeVal { tv_sec: 0, tv_usec: 0 };
    let r = approxidate_main("march 10 2013 04:00:07 -0500", &mut tv);
    assert_eq!(r, 0);
    assert_eq!(tv.tv_sec, 1362906007);
}

#[test]
fn test_approxidate_main_object_header() {
    let mut tv = TimeVal { tv_sec: 0, tv_usec: 0 };
    let r = approxidate_main("@1362873602 +0000", &mut tv);
    assert_eq!(r, 0);
    assert_eq!(tv.tv_sec, 1362873602);
}

#[test]
fn test_approxidate_main_object_header_zero() {
    let mut tv = TimeVal { tv_sec: 0, tv_usec: 0 };
    let r = approxidate_main("@0 +0000", &mut tv);
    assert_eq!(r, 0);
    assert_eq!(tv.tv_sec, 0);
}

#[test]
fn test_approxidate_main_epoch_number() {
    let mut tv = TimeVal { tv_sec: 0, tv_usec: 0 };
    let r = approxidate_main("1362873602", &mut tv);
    assert_eq!(r, 0);
    assert_eq!(tv.tv_sec, 1362873602);
}

#[test]
fn test_approxidate_main_empty_returns_neg1() {
    let mut tv = TimeVal { tv_sec: 0, tv_usec: 0 };
    let r = approxidate_main("", &mut tv);
    assert_eq!(r, -1);
}

#[test]
fn test_approxidate_main_not_a_date() {
    let mut tv = TimeVal { tv_sec: 0, tv_usec: 0 };
    let r = approxidate_main("not-a-date-at-all", &mut tv);
    assert_eq!(r, -1);
}

// ---- approxidate_relative ----

#[test]
fn test_approxidate_relative_exact_date() {
    let mut tv = TimeVal { tv_sec: 0, tv_usec: 0 };
    let mut rel = TimeVal { tv_sec: 1577910617, tv_usec: 0 };
    let r = approxidate_relative("1/1/2014", &mut tv, &mut rel);
    assert_eq!(r, 0);
    assert_eq!(tv.tv_sec, 1388608217);
}

#[test]
fn test_approxidate_relative_yesterday() {
    let mut tv = TimeVal { tv_sec: 0, tv_usec: 0 };
    let mut rel = TimeVal { tv_sec: 1362873600, tv_usec: 0 };
    let r = approxidate_relative("yesterday", &mut tv, &mut rel);
    assert_eq!(r, 0);
    assert_eq!(tv.tv_sec, 1362787200);
}

#[test]
fn test_approxidate_relative_now() {
    let mut tv = TimeVal { tv_sec: 0, tv_usec: 0 };
    let mut rel = TimeVal { tv_sec: 1362873600, tv_usec: 500000 };
    let r = approxidate_relative("now", &mut tv, &mut rel);
    assert_eq!(r, 0);
    assert_eq!(tv.tv_sec, 1362873600);
}

#[test]
fn test_approxidate_relative_empty_fails() {
    let mut tv = TimeVal { tv_sec: 0, tv_usec: 0 };
    let mut rel = TimeVal { tv_sec: 1362873600, tv_usec: 0 };
    let r = approxidate_relative("", &mut tv, &mut rel);
    assert_eq!(r, -1);
}

// ---- approxidate_str ----

#[test]
fn test_approxidate_str_yesterday() {
    let mut tv = TimeVal { tv_sec: 1362873600, tv_usec: 0 };
    let r = approxidate_str("yesterday", &mut tv);
    assert_eq!(r, 0);
    assert_eq!(tv.tv_sec, 1362787200);
}

#[test]
fn test_approxidate_str_empty_fails() {
    let mut tv = TimeVal { tv_sec: 1362873600, tv_usec: 0 };
    let r = approxidate_str("", &mut tv);
    assert_eq!(r, -1);
}

// ---- is_date ----

#[test]
fn test_is_date_valid() {
    let null_now = Atm {
        tm_sec: 0, tm_min: 0, tm_hour: 0, tm_mday: 0, tm_mon: 0,
        tm_year: i32::MIN, tm_wday: 0, tm_yday: 0, tm_isdst: 0, tm_usec: 0,
    };
    let mut tm = Atm {
        tm_sec: -1, tm_min: -1, tm_hour: -1, tm_mday: -1, tm_mon: -1,
        tm_year: -1, tm_wday: 0, tm_yday: 0, tm_isdst: -1, tm_usec: 0,
    };
    assert_eq!(is_date(2013, 3, 10, &null_now, 0, &mut tm), 1);
    assert_eq!(tm.tm_mon, 2); // March = 2 (0-indexed)
    assert_eq!(tm.tm_mday, 10);
    assert_eq!(tm.tm_year, 113); // 2013 - 1900
}

#[test]
fn test_is_date_invalid_month() {
    let null_now = Atm {
        tm_sec: 0, tm_min: 0, tm_hour: 0, tm_mday: 0, tm_mon: 0,
        tm_year: i32::MIN, tm_wday: 0, tm_yday: 0, tm_isdst: 0, tm_usec: 0,
    };
    let mut tm = Atm {
        tm_sec: -1, tm_min: -1, tm_hour: -1, tm_mday: -1, tm_mon: -1,
        tm_year: -1, tm_wday: 0, tm_yday: 0, tm_isdst: -1, tm_usec: 0,
    };
    assert_eq!(is_date(2013, 0, 10, &null_now, 0, &mut tm), 0);
    assert_eq!(is_date(2013, 13, 10, &null_now, 0, &mut tm), 0);
}

#[test]
fn test_is_date_invalid_day() {
    let null_now = Atm {
        tm_sec: 0, tm_min: 0, tm_hour: 0, tm_mday: 0, tm_mon: 0,
        tm_year: i32::MIN, tm_wday: 0, tm_yday: 0, tm_isdst: 0, tm_usec: 0,
    };
    let mut tm = Atm {
        tm_sec: -1, tm_min: -1, tm_hour: -1, tm_mday: -1, tm_mon: -1,
        tm_year: -1, tm_wday: 0, tm_yday: 0, tm_isdst: -1, tm_usec: 0,
    };
    assert_eq!(is_date(2013, 3, 0, &null_now, 0, &mut tm), 0);
    assert_eq!(is_date(2013, 3, 32, &null_now, 0, &mut tm), 0);
}

#[test]
fn test_is_date_two_digit_year() {
    let null_now = Atm {
        tm_sec: 0, tm_min: 0, tm_hour: 0, tm_mday: 0, tm_mon: 0,
        tm_year: i32::MIN, tm_wday: 0, tm_yday: 0, tm_isdst: 0, tm_usec: 0,
    };
    let mut tm = Atm {
        tm_sec: -1, tm_min: -1, tm_hour: -1, tm_mday: -1, tm_mon: -1,
        tm_year: -1, tm_wday: 0, tm_yday: 0, tm_isdst: -1, tm_usec: 0,
    };
    // year 99 => tm_year = 99
    assert_eq!(is_date(99, 3, 10, &null_now, 0, &mut tm), 1);
    assert_eq!(tm.tm_year, 99);
}

#[test]
fn test_is_date_small_year() {
    let null_now = Atm {
        tm_sec: 0, tm_min: 0, tm_hour: 0, tm_mday: 0, tm_mon: 0,
        tm_year: i32::MIN, tm_wday: 0, tm_yday: 0, tm_isdst: 0, tm_usec: 0,
    };
    let mut tm = Atm {
        tm_sec: -1, tm_min: -1, tm_hour: -1, tm_mday: -1, tm_mon: -1,
        tm_year: -1, tm_wday: 0, tm_yday: 0, tm_isdst: -1, tm_usec: 0,
    };
    // year 5 (< 38) => tm_year = 105
    assert_eq!(is_date(5, 3, 10, &null_now, 0, &mut tm), 1);
    assert_eq!(tm.tm_year, 105);
}

#[test]
fn test_is_date_no_year() {
    let null_now = Atm {
        tm_sec: 0, tm_min: 0, tm_hour: 0, tm_mday: 0, tm_mon: 0,
        tm_year: i32::MIN, tm_wday: 0, tm_yday: 0, tm_isdst: 0, tm_usec: 0,
    };
    let mut tm = Atm {
        tm_sec: -1, tm_min: -1, tm_hour: -1, tm_mday: -1, tm_mon: -1,
        tm_year: -1, tm_wday: 0, tm_yday: 0, tm_isdst: -1, tm_usec: 0,
    };
    // year -1 with no now_tm => return 1, don't set year
    assert_eq!(is_date(-1, 3, 10, &null_now, 0, &mut tm), 1);
    assert_eq!(tm.tm_year, -1); // unchanged
}

// ---- match_digit ----

#[test]
fn test_match_digit_epoch_timestamp() {
    let mut tm = Atm {
        tm_sec: -1, tm_min: -1, tm_hour: -1, tm_mday: -1, tm_mon: -1,
        tm_year: -1, tm_wday: 0, tm_yday: 0, tm_isdst: -1, tm_usec: 0,
    };
    let mut offset = -1;
    let mut tm_gmt = 0;
    let m = match_digit("1362873602", &mut tm, &mut offset, &mut tm_gmt);
    assert_eq!(m, 10);
    assert_eq!(tm_gmt, 1);
    assert_eq!(tm.tm_year, 113); // 2013 - 1900
}

#[test]
fn test_match_digit_four_digit_year() {
    let mut tm = Atm {
        tm_sec: 0, tm_min: 0, tm_hour: 0, tm_mday: 10, tm_mon: 2,
        tm_year: -1, tm_wday: 0, tm_yday: 0, tm_isdst: 0, tm_usec: 0,
    };
    let mut offset = -1;
    let mut tm_gmt = 0;
    let m = match_digit("2013", &mut tm, &mut offset, &mut tm_gmt);
    assert_eq!(m, 4);
    assert_eq!(tm.tm_year, 113);
}

#[test]
fn test_match_digit_timezone_offset() {
    let mut tm = Atm {
        tm_sec: 0, tm_min: 0, tm_hour: 0, tm_mday: 10, tm_mon: 2,
        tm_year: 113, tm_wday: 0, tm_yday: 0, tm_isdst: 0, tm_usec: 0,
    };
    let mut offset = -1;
    let mut tm_gmt = 0;
    let m = match_digit("0500", &mut tm, &mut offset, &mut tm_gmt);
    assert_eq!(m, 4);
    assert_eq!(offset, 300);
}

#[test]
fn test_match_digit_day() {
    let mut tm = Atm {
        tm_sec: 0, tm_min: 0, tm_hour: 0, tm_mday: -1, tm_mon: 2,
        tm_year: 113, tm_wday: 0, tm_yday: 0, tm_isdst: 0, tm_usec: 0,
    };
    let mut offset = 0;
    let mut tm_gmt = 0;
    let m = match_digit("10", &mut tm, &mut offset, &mut tm_gmt);
    assert_eq!(m, 2);
    assert_eq!(tm.tm_mday, 10);
}

// ---- match_multi_number ----

#[test]
fn test_match_multi_number_time() {
    let mut tm = Atm {
        tm_sec: -1, tm_min: -1, tm_hour: -1, tm_mday: -1, tm_mon: -1,
        tm_year: -1, tm_wday: 0, tm_yday: 0, tm_isdst: -1, tm_usec: 0,
    };
    let date = "04:00:07";
    let m = match_multi_number(4, ':', date, &date[2..], &mut tm, 0);
    assert!(m > 0);
    assert_eq!(tm.tm_hour, 4);
    assert_eq!(tm.tm_min, 0);
    assert_eq!(tm.tm_sec, 7);
}

#[test]
fn test_match_multi_number_time_with_usec() {
    let mut tm = Atm {
        tm_sec: -1, tm_min: -1, tm_hour: -1, tm_mday: -1, tm_mon: -1,
        tm_year: -1, tm_wday: 0, tm_yday: 0, tm_isdst: -1, tm_usec: 0,
    };
    let date = "00:00:07.657891";
    let m = match_multi_number(0, ':', date, &date[2..], &mut tm, 0);
    assert!(m > 0);
    assert_eq!(tm.tm_hour, 0);
    assert_eq!(tm.tm_min, 0);
    assert_eq!(tm.tm_sec, 7);
    assert_eq!(tm.tm_usec, 657891);
}

#[test]
fn test_match_multi_number_date_slash() {
    let mut tm = Atm {
        tm_sec: -1, tm_min: -1, tm_hour: -1, tm_mday: -1, tm_mon: -1,
        tm_year: -1, tm_wday: 0, tm_yday: 0, tm_isdst: -1, tm_usec: 0,
    };
    let date = "1/1/2014";
    let m = match_multi_number(1, '/', date, &date[1..], &mut tm, 0);
    assert!(m > 0);
    assert_eq!(tm.tm_mon, 0); // January
    assert_eq!(tm.tm_mday, 1);
    assert_eq!(tm.tm_year, 114); // 2014 - 1900
}

// ---- pending_number ----

#[test]
fn test_pending_number_sets_mday() {
    let mut tm = Atm {
        tm_sec: -1, tm_min: -1, tm_hour: -1, tm_mday: -1, tm_mon: 2,
        tm_year: 113, tm_wday: 0, tm_yday: 0, tm_isdst: -1, tm_usec: 0,
    };
    pending_number(&mut tm, &15);
    assert_eq!(tm.tm_mday, 15);
}

#[test]
fn test_pending_number_sets_month() {
    let mut tm = Atm {
        tm_sec: -1, tm_min: -1, tm_hour: -1, tm_mday: 10, tm_mon: -1,
        tm_year: 113, tm_wday: 0, tm_yday: 0, tm_isdst: -1, tm_usec: 0,
    };
    pending_number(&mut tm, &3);
    assert_eq!(tm.tm_mon, 2); // 3 - 1 = 2
}

#[test]
fn test_pending_number_sets_year() {
    let mut tm = Atm {
        tm_sec: -1, tm_min: -1, tm_hour: -1, tm_mday: 10, tm_mon: 2,
        tm_year: -1, tm_wday: 0, tm_yday: 0, tm_isdst: -1, tm_usec: 0,
    };
    pending_number(&mut tm, &2013);
    assert_eq!(tm.tm_year, 113);
}

#[test]
fn test_pending_number_zero_noop() {
    let mut tm = Atm {
        tm_sec: -1, tm_min: -1, tm_hour: -1, tm_mday: -1, tm_mon: -1,
        tm_year: -1, tm_wday: 0, tm_yday: 0, tm_isdst: -1, tm_usec: 0,
    };
    pending_number(&mut tm, &0);
    assert_eq!(tm.tm_mday, -1); // unchanged
}
