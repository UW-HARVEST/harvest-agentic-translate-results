use approxidate::approxidate::*;

#[test]
fn test_tm_to_time_t_epoch() {
    let t = Atm {
        tm_sec: 0, tm_min: 0, tm_hour: 0, tm_mday: 1, tm_mon: 0,
        tm_year: 70, tm_wday: 0, tm_yday: 0, tm_isdst: 0, tm_usec: 0,
    };
    assert_eq!(tm_to_time_t(&t), Some(0));
}

#[test]
fn test_tm_to_time_t_mar10_2013() {
    let t = Atm {
        tm_sec: 2, tm_min: 0, tm_hour: 0, tm_mday: 10, tm_mon: 2,
        tm_year: 113, tm_wday: 0, tm_yday: 0, tm_isdst: 0, tm_usec: 0,
    };
    assert_eq!(tm_to_time_t(&t), Some(1362873602));
}

#[test]
fn test_tm_to_time_t_year_below_1970() {
    let t = Atm {
        tm_sec: 0, tm_min: 0, tm_hour: 0, tm_mday: 1, tm_mon: 0,
        tm_year: 69, tm_wday: 0, tm_yday: 0, tm_isdst: 0, tm_usec: 0,
    };
    assert_eq!(tm_to_time_t(&t), None);
}

#[test]
fn test_tm_to_time_t_year_above_2099() {
    let t = Atm {
        tm_sec: 0, tm_min: 0, tm_hour: 0, tm_mday: 1, tm_mon: 0,
        tm_year: 200, tm_wday: 0, tm_yday: 0, tm_isdst: 0, tm_usec: 0,
    };
    assert_eq!(tm_to_time_t(&t), None);
}

#[test]
fn test_tm_to_time_t_invalid_month() {
    let t = Atm {
        tm_sec: 0, tm_min: 0, tm_hour: 0, tm_mday: 1, tm_mon: 12,
        tm_year: 113, tm_wday: 0, tm_yday: 0, tm_isdst: 0, tm_usec: 0,
    };
    assert_eq!(tm_to_time_t(&t), None);
}

#[test]
fn test_tm_to_time_t_negative_sec() {
    let t = Atm {
        tm_sec: -1, tm_min: 0, tm_hour: 0, tm_mday: 1, tm_mon: 0,
        tm_year: 113, tm_wday: 0, tm_yday: 0, tm_isdst: 0, tm_usec: 0,
    };
    assert_eq!(tm_to_time_t(&t), None);
}

#[test]
fn test_tm_to_time_t_feb29_2000() {
    let t = Atm {
        tm_sec: 0, tm_min: 0, tm_hour: 0, tm_mday: 29, tm_mon: 1,
        tm_year: 100, tm_wday: 0, tm_yday: 0, tm_isdst: 0, tm_usec: 0,
    };
    assert_eq!(tm_to_time_t(&t), Some(951782400));
}

#[test]
fn test_tm_to_time_t_max_2099() {
    let t = Atm {
        tm_sec: 59, tm_min: 59, tm_hour: 23, tm_mday: 31, tm_mon: 11,
        tm_year: 199, tm_wday: 0, tm_yday: 0, tm_isdst: 0, tm_usec: 0,
    };
    assert_eq!(tm_to_time_t(&t), Some(4102444799));
}

#[test]
fn test_match_string_exact() {
    assert_eq!(match_string("January", "January"), 7);
}

#[test]
fn test_match_string_case_insensitive() {
    assert_eq!(match_string("january", "January"), 7);
    assert_eq!(match_string("JANUARY", "January"), 7);
}

#[test]
fn test_match_string_partial_with_separator() {
    // "Jan " matches "January" up to length 3 (then non-alnum stops)
    assert_eq!(match_string("Jan ", "January"), 3);
}

#[test]
fn test_match_string_no_match() {
    assert_eq!(match_string("Foo", "January"), 0);
}

#[test]
fn test_match_string_extra_chars_in_date_alpha() {
    // "Februar2" - 7 chars match "February", then '2' is alnum but mismatches
    assert_eq!(match_string("Februar2", "February"), 0);
}

#[test]
fn test_skip_alpha_basic() {
    assert_eq!(skip_alpha("Foo123"), 3);
}

#[test]
fn test_skip_alpha_single() {
    assert_eq!(skip_alpha("X "), 1);
}

#[test]
fn test_match_alpha_month() {
    let mut tm = Atm::default();
    tm.tm_mon = -1; tm.tm_hour = -1; tm.tm_year = -1; tm.tm_mday = -1;
    tm.tm_min = -1; tm.tm_sec = -1; tm.tm_isdst = -1;
    let mut off: i32 = -1;
    let r = match_alpha("March 10", &mut tm, &mut off);
    assert_eq!(r, 5);
    assert_eq!(tm.tm_mon, 2);
}

#[test]
fn test_match_alpha_weekday() {
    let mut tm = Atm::default();
    tm.tm_mon = -1; tm.tm_hour = -1; tm.tm_year = -1; tm.tm_mday = -1;
    tm.tm_min = -1; tm.tm_sec = -1; tm.tm_isdst = -1;
    let mut off: i32 = -1;
    let r = match_alpha("Mon", &mut tm, &mut off);
    assert_eq!(r, 3);
    assert_eq!(tm.tm_wday, 1);
}

#[test]
fn test_match_alpha_timezone_utc() {
    let mut tm = Atm::default();
    tm.tm_mon = -1; tm.tm_hour = -1; tm.tm_year = -1; tm.tm_mday = -1;
    tm.tm_min = -1; tm.tm_sec = -1; tm.tm_isdst = -1;
    let mut off: i32 = -1;
    let r = match_alpha("UTC", &mut tm, &mut off);
    assert_eq!(r, 3);
    assert_eq!(off, 0);
}

#[test]
fn test_match_alpha_timezone_pdt_with_dst() {
    let mut tm = Atm::default();
    tm.tm_mon = -1; tm.tm_hour = -1; tm.tm_year = -1; tm.tm_mday = -1;
    tm.tm_min = -1; tm.tm_sec = -1; tm.tm_isdst = -1;
    let mut off: i32 = -1;
    let r = match_alpha("PDT", &mut tm, &mut off);
    assert_eq!(r, 3);
    // PDT offset -8, dst +1 -> -7, in minutes = -420
    assert_eq!(off, -420);
}

#[test]
fn test_match_alpha_pm_lowercase_no_match() {
    // "pm" - PM matches case-insensitively
    let mut tm = Atm::default();
    tm.tm_hour = 3; tm.tm_mon = -1; tm.tm_year = -1; tm.tm_mday = -1;
    tm.tm_min = -1; tm.tm_sec = -1; tm.tm_isdst = -1;
    let mut off: i32 = -1;
    let r = match_alpha("PM", &mut tm, &mut off);
    assert_eq!(r, 2);
    assert_eq!(tm.tm_hour, 15);
}

#[test]
fn test_match_alpha_am() {
    let mut tm = Atm::default();
    tm.tm_hour = 13; tm.tm_mon = -1; tm.tm_year = -1; tm.tm_mday = -1;
    tm.tm_min = -1; tm.tm_sec = -1; tm.tm_isdst = -1;
    let mut off: i32 = -1;
    let r = match_alpha("AM", &mut tm, &mut off);
    assert_eq!(r, 2);
    assert_eq!(tm.tm_hour, 1);
}

#[test]
fn test_is_date_valid_2013() {
    let mut tm = Atm::default();
    tm.tm_mon = -1; tm.tm_year = -1; tm.tm_mday = -1;
    let now = Atm::default();
    let r = is_date(2013, 3, 10, &now, 0, &mut tm);
    assert_eq!(r, 1);
    assert_eq!(tm.tm_mon, 2);
    assert_eq!(tm.tm_mday, 10);
    assert_eq!(tm.tm_year, 113);
}

#[test]
fn test_is_date_invalid_month() {
    let mut tm = Atm::default();
    tm.tm_mon = -1; tm.tm_year = -1; tm.tm_mday = -1;
    let now = Atm::default();
    let r = is_date(2013, 13, 10, &now, 0, &mut tm);
    assert_eq!(r, 0);
}

#[test]
fn test_is_date_invalid_day_zero() {
    let mut tm = Atm::default();
    let now = Atm::default();
    let r = is_date(2013, 3, 0, &now, 0, &mut tm);
    assert_eq!(r, 0);
}

#[test]
fn test_is_date_two_digit_year_70_99() {
    let mut tm = Atm::default();
    tm.tm_mon = -1; tm.tm_year = -1; tm.tm_mday = -1;
    let now = Atm::default();
    let r = is_date(85, 1, 1, &now, 0, &mut tm);
    assert_eq!(r, 1);
    assert_eq!(tm.tm_year, 85);
}

#[test]
fn test_is_date_two_digit_year_below_38() {
    let mut tm = Atm::default();
    tm.tm_mon = -1; tm.tm_year = -1; tm.tm_mday = -1;
    let now = Atm::default();
    let r = is_date(20, 1, 1, &now, 0, &mut tm);
    assert_eq!(r, 1);
    assert_eq!(tm.tm_year, 120);
}

#[test]
fn test_is_date_invalid_year_range() {
    let mut tm = Atm::default();
    tm.tm_mon = -1; tm.tm_year = -1; tm.tm_mday = -1;
    let now = Atm::default();
    let r = is_date(50, 1, 1, &now, 0, &mut tm);
    assert_eq!(r, 0);
}

#[test]
fn test_nodate_all_negative() {
    let mut tm = Atm {
        tm_sec: -1, tm_min: -1, tm_hour: -1, tm_mday: -1, tm_mon: -1,
        tm_year: -1, tm_wday: 0, tm_yday: 0, tm_isdst: 0, tm_usec: 0,
    };
    assert_eq!(nodate(&mut tm), 1);
}

#[test]
fn test_nodate_partial() {
    let mut tm = Atm {
        tm_sec: 0, tm_min: -1, tm_hour: -1, tm_mday: -1, tm_mon: -1,
        tm_year: -1, tm_wday: 0, tm_yday: 0, tm_isdst: 0, tm_usec: 0,
    };
    // bitwise AND of (-1 & -1 & -1 & -1 & -1 & 0) = 0; not < 0
    assert_eq!(nodate(&mut tm), 0);
}

#[test]
fn test_match_tz_hhmm() {
    let mut off: i32 = 0;
    let r = match_tz("+0500", &mut off);
    assert_eq!(r, 5);
    assert_eq!(off, 300);
}

#[test]
fn test_match_tz_negative() {
    let mut off: i32 = 0;
    let r = match_tz("-0500", &mut off);
    assert_eq!(r, 5);
    assert_eq!(off, -300);
}

#[test]
fn test_match_tz_hh_only() {
    let mut off: i32 = 999;
    let r = match_tz("+05", &mut off);
    assert_eq!(r, 3);
    assert_eq!(off, 300);
}

#[test]
fn test_match_tz_hh_colon_mm() {
    let mut off: i32 = 0;
    let r = match_tz("+05:30", &mut off);
    assert_eq!(r, 6);
    assert_eq!(off, 330);
}

#[test]
fn test_match_object_header_date_basic() {
    let mut tv = TimeVal::default();
    let mut off: i32 = -1;
    let r = match_object_header_date("1234567890 +0000", &mut tv, &mut off);
    assert_eq!(r, 0);
    assert_eq!(tv.tv_sec, 1234567890);
    assert_eq!(off, 0);
}

#[test]
fn test_match_object_header_date_pos_offset() {
    let mut tv = TimeVal::default();
    let mut off: i32 = -1;
    let r = match_object_header_date("500 +0500", &mut tv, &mut off);
    assert_eq!(r, 0);
    assert_eq!(tv.tv_sec, 500);
    assert_eq!(off, 300);
}

#[test]
fn test_match_object_header_date_neg_offset() {
    let mut tv = TimeVal::default();
    let mut off: i32 = -1;
    let r = match_object_header_date("500 -0500", &mut tv, &mut off);
    assert_eq!(r, 0);
    assert_eq!(tv.tv_sec, 500);
    assert_eq!(off, -300);
}

#[test]
fn test_match_object_header_date_bad_no_offset() {
    let mut tv = TimeVal::default();
    let mut off: i32 = -1;
    let r = match_object_header_date("500", &mut tv, &mut off);
    assert_eq!(r, -1);
}

#[test]
fn test_match_object_header_date_non_digit() {
    let mut tv = TimeVal::default();
    let mut off: i32 = -1;
    let r = match_object_header_date("abc", &mut tv, &mut off);
    assert_eq!(r, -1);
}

#[test]
fn test_parse_date_basic_iso() {
    let mut tv = TimeVal::default();
    let mut off: i32 = -1;
    let r = parse_date_basic("10/Mar/2013:00:00:02 UTC", &mut tv, &mut off);
    assert_eq!(r, 0);
    assert_eq!(tv.tv_sec, 1362873602);
    assert_eq!(tv.tv_usec, 0);
    assert_eq!(off, 0);
}

#[test]
fn test_parse_date_basic_with_microseconds() {
    let mut tv = TimeVal::default();
    let mut off: i32 = -1;
    let r = parse_date_basic("10/Mar/2013:00:00:02.003 UTC", &mut tv, &mut off);
    assert_eq!(r, 0);
    assert_eq!(tv.tv_sec, 1362873602);
    assert_eq!(tv.tv_usec, 3000);
    assert_eq!(off, 0);
}

#[test]
fn test_parse_date_basic_object_header() {
    let mut tv = TimeVal::default();
    let mut off: i32 = -1;
    let r = parse_date_basic("@1234567890 +0000", &mut tv, &mut off);
    assert_eq!(r, 0);
    assert_eq!(tv.tv_sec, 1234567890);
    assert_eq!(tv.tv_usec, 0);
    assert_eq!(off, 0);
}

#[test]
fn test_parse_date_basic_with_tz_offset() {
    let mut tv = TimeVal::default();
    let mut off: i32 = -1;
    let r = parse_date_basic("10 march 2013 04:00:07 -0500", &mut tv, &mut off);
    assert_eq!(r, 0);
    assert_eq!(tv.tv_sec, 1362906007);
    assert_eq!(tv.tv_usec, 0);
    assert_eq!(off, -300);
}

#[test]
fn test_parse_date_basic_garbage() {
    let mut tv = TimeVal::default();
    let mut off: i32 = -1;
    let r = parse_date_basic("garbage", &mut tv, &mut off);
    assert_eq!(r, -1);
}

#[test]
fn test_parse_date_basic_no_year_fails() {
    // "1/1/2014 UTC" without a tz fails because it doesn't have a 4-digit year detection consistent with parse_date_basic
    let mut tv = TimeVal::default();
    let mut off: i32 = -1;
    let r = parse_date_basic("1/1/2014 UTC", &mut tv, &mut off);
    assert_eq!(r, -1);
}

#[test]
fn test_approxidate_main_basic() {
    let mut tv = TimeVal::default();
    let rc = approxidate_main("10/Mar/2013:00:00:02.003 UTC", &mut tv);
    assert_eq!(rc, 0);
    assert_eq!(tv.tv_sec, 1362873602);
    assert_eq!(tv.tv_usec, 3000);
}

#[test]
fn test_approxidate_main_with_offset() {
    let mut tv = TimeVal::default();
    let rc = approxidate_main("10/Mar/2012:00:00:07 +0500", &mut tv);
    assert_eq!(rc, 0);
    assert_eq!(tv.tv_sec, 1331319607);
    assert_eq!(tv.tv_usec, 0);
}

#[test]
fn test_approxidate_main_negative_offset() {
    let mut tv = TimeVal::default();
    let rc = approxidate_main("10/Mar/2012:00:00:07.657891 -0110", &mut tv);
    assert_eq!(rc, 0);
    assert_eq!(tv.tv_sec, 1331341807);
    assert_eq!(tv.tv_usec, 657891);
}

#[test]
fn test_approxidate_main_alpha_date() {
    let mut tv = TimeVal::default();
    let rc = approxidate_main("mar 10 2013 04:00:07 -0500", &mut tv);
    assert_eq!(rc, 0);
    assert_eq!(tv.tv_sec, 1362906007);
    assert_eq!(tv.tv_usec, 0);
}

#[test]
fn test_approxidate_main_full_month() {
    let mut tv = TimeVal::default();
    let rc = approxidate_main("march 10 2013 04:00:07 -0500", &mut tv);
    assert_eq!(rc, 0);
    assert_eq!(tv.tv_sec, 1362906007);
    assert_eq!(tv.tv_usec, 0);
}

#[test]
fn test_approxidate_main_year_first() {
    let mut tv = TimeVal::default();
    let rc = approxidate_main("2013 march 10 04:00:07 -0500", &mut tv);
    assert_eq!(rc, 0);
    assert_eq!(tv.tv_sec, 1362906007);
    assert_eq!(tv.tv_usec, 0);
}

#[test]
fn test_approxidate_main_object_header() {
    let mut tv = TimeVal::default();
    let rc = approxidate_main("@1234567890 +0000", &mut tv);
    assert_eq!(rc, 0);
    assert_eq!(tv.tv_sec, 1234567890);
}

#[test]
fn test_approxidate_main_garbage_fails() {
    let mut tv = TimeVal { tv_sec: 0, tv_usec: 0 };
    let rc = approxidate_main("garbage", &mut tv);
    assert_eq!(rc, -1);
}

#[test]
fn test_approxidate_relative_basic() {
    let mut tv = TimeVal::default();
    let mut rel = TimeVal { tv_sec: 1577910617, tv_usec: 0 };
    let rc = approxidate_relative("1/1/2014", &mut tv, &mut rel);
    assert_eq!(rc, 0);
    assert_eq!(tv.tv_sec, 1388608217);
    assert_eq!(tv.tv_usec, 0);
}

#[test]
fn test_approxidate_relative_yesterday() {
    let mut tv = TimeVal::default();
    let mut rel = TimeVal { tv_sec: 1577910617, tv_usec: 0 };
    let rc = approxidate_relative("yesterday", &mut tv, &mut rel);
    assert_eq!(rc, 0);
    assert_eq!(tv.tv_sec, 1577824217);
    assert_eq!(tv.tv_usec, 0);
}

#[test]
fn test_approxidate_relative_noon() {
    let mut tv = TimeVal::default();
    let mut rel = TimeVal { tv_sec: 1577910617, tv_usec: 0 };
    let rc = approxidate_relative("noon", &mut tv, &mut rel);
    assert_eq!(rc, 0);
    assert_eq!(tv.tv_sec, 1577880000);
    assert_eq!(tv.tv_usec, 0);
}

#[test]
fn test_approxidate_relative_midnight() {
    let mut tv = TimeVal::default();
    let mut rel = TimeVal { tv_sec: 1577910617, tv_usec: 0 };
    let rc = approxidate_relative("midnight", &mut tv, &mut rel);
    assert_eq!(rc, 0);
    assert_eq!(tv.tv_sec, 1577836800);
    assert_eq!(tv.tv_usec, 0);
}

#[test]
fn test_approxidate_relative_tea() {
    let mut tv = TimeVal::default();
    let mut rel = TimeVal { tv_sec: 1577910617, tv_usec: 0 };
    let rc = approxidate_relative("tea", &mut tv, &mut rel);
    assert_eq!(rc, 0);
    assert_eq!(tv.tv_sec, 1577898000);
    assert_eq!(tv.tv_usec, 0);
}

#[test]
fn test_approxidate_relative_never() {
    let mut tv = TimeVal::default();
    let mut rel = TimeVal { tv_sec: 1577910617, tv_usec: 0 };
    let rc = approxidate_relative("never", &mut tv, &mut rel);
    assert_eq!(rc, 0);
    assert_eq!(tv.tv_sec, 0);
    assert_eq!(tv.tv_usec, 0);
}

#[test]
fn test_approxidate_relative_now() {
    let mut tv = TimeVal::default();
    let mut rel = TimeVal { tv_sec: 1577910617, tv_usec: 0 };
    let rc = approxidate_relative("now", &mut tv, &mut rel);
    assert_eq!(rc, 0);
    assert_eq!(tv.tv_sec, 1577910617);
    assert_eq!(tv.tv_usec, 0);
}

#[test]
fn test_approxidate_relative_two_days_ago() {
    let mut tv = TimeVal::default();
    let mut rel = TimeVal { tv_sec: 1577910617, tv_usec: 0 };
    let rc = approxidate_relative("2 days ago", &mut tv, &mut rel);
    assert_eq!(rc, 0);
    assert_eq!(tv.tv_sec, 1577737817);
    assert_eq!(tv.tv_usec, 0);
}

#[test]
fn test_approxidate_relative_three_weeks_ago() {
    let mut tv = TimeVal::default();
    let mut rel = TimeVal { tv_sec: 1577910617, tv_usec: 0 };
    let rc = approxidate_relative("3 weeks ago", &mut tv, &mut rel);
    assert_eq!(rc, 0);
    assert_eq!(tv.tv_sec, 1576096217);
}

#[test]
fn test_approxidate_relative_five_hours_ago() {
    let mut tv = TimeVal::default();
    let mut rel = TimeVal { tv_sec: 1577910617, tv_usec: 0 };
    let rc = approxidate_relative("five hours ago", &mut tv, &mut rel);
    assert_eq!(rc, 0);
    assert_eq!(tv.tv_sec, 1577892617);
}

#[test]
fn test_approxidate_relative_last_week() {
    let mut tv = TimeVal::default();
    let mut rel = TimeVal { tv_sec: 1577910617, tv_usec: 0 };
    let rc = approxidate_relative("last week", &mut tv, &mut rel);
    assert_eq!(rc, 0);
    assert_eq!(tv.tv_sec, 1577305817);
}

#[test]
fn test_approxidate_relative_one_month_ago() {
    let mut tv = TimeVal::default();
    let mut rel = TimeVal { tv_sec: 1577910617, tv_usec: 0 };
    let rc = approxidate_relative("1 month ago", &mut tv, &mut rel);
    assert_eq!(rc, 0);
    assert_eq!(tv.tv_sec, 1575232217);
}

#[test]
fn test_approxidate_relative_one_year_ago() {
    let mut tv = TimeVal::default();
    let mut rel = TimeVal { tv_sec: 1577910617, tv_usec: 0 };
    let rc = approxidate_relative("1 year ago", &mut tv, &mut rel);
    assert_eq!(rc, 0);
    assert_eq!(tv.tv_sec, 1546374617);
}

#[test]
fn test_approxidate_relative_garbage_fails() {
    let mut tv = TimeVal::default();
    let mut rel = TimeVal { tv_sec: 1577910617, tv_usec: 0 };
    let rc = approxidate_relative("garbage", &mut tv, &mut rel);
    assert_eq!(rc, -1);
}

#[test]
fn test_approxidate_relative_time_only_with_tz() {
    let mut tv = TimeVal::default();
    let mut rel = TimeVal { tv_sec: 1577910617, tv_usec: 0 };
    let rc = approxidate_relative("23:11:07.9876 +1400", &mut tv, &mut rel);
    assert_eq!(rc, 0);
    assert_eq!(tv.tv_sec, 1577920267);
    assert_eq!(tv.tv_usec, 987600);
}

#[test]
fn test_approxidate_str_yesterday() {
    let mut tv = TimeVal { tv_sec: 1577910617, tv_usec: 0 };
    let rc = approxidate_str("yesterday", &mut tv);
    assert_eq!(rc, 0);
    assert_eq!(tv.tv_sec, 1577824217);
    assert_eq!(tv.tv_usec, 0);
}

#[test]
fn test_approxidate_str_garbage() {
    let mut tv = TimeVal { tv_sec: 1577910617, tv_usec: 0 };
    let rc = approxidate_str("garbage", &mut tv);
    assert_eq!(rc, -1);
}

#[test]
fn test_pending_number_sets_mday() {
    let mut tm = Atm::default();
    tm.tm_mday = -1; tm.tm_mon = -1; tm.tm_year = -1;
    let n: i32 = 15;
    pending_number(&mut tm, &n);
    assert_eq!(tm.tm_mday, 15);
}

#[test]
fn test_pending_number_sets_year_4digit() {
    let mut tm = Atm::default();
    tm.tm_mday = 1; tm.tm_mon = 0; tm.tm_year = -1;
    let n: i32 = 2020;
    pending_number(&mut tm, &n);
    assert_eq!(tm.tm_year, 120);
}

#[test]
fn test_pending_number_sets_year_2digit_70_99() {
    let mut tm = Atm::default();
    tm.tm_mday = 1; tm.tm_mon = 0; tm.tm_year = -1;
    let n: i32 = 85;
    pending_number(&mut tm, &n);
    assert_eq!(tm.tm_year, 85);
}

#[test]
fn test_pending_number_sets_year_2digit_below_38() {
    let mut tm = Atm::default();
    tm.tm_mday = 1; tm.tm_mon = 0; tm.tm_year = -1;
    let n: i32 = 25;
    pending_number(&mut tm, &n);
    assert_eq!(tm.tm_year, 125);
}

#[test]
fn test_pending_number_zero_does_nothing() {
    let mut tm = Atm::default();
    tm.tm_mday = -1; tm.tm_mon = -1; tm.tm_year = -1;
    let n: i32 = 0;
    pending_number(&mut tm, &n);
    assert_eq!(tm.tm_mday, -1);
    assert_eq!(tm.tm_mon, -1);
    assert_eq!(tm.tm_year, -1);
}

fn main() {}
