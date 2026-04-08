use approxidate::approxidate::*;

fn main() {}

// ---- tm_to_time_t ----

#[test]
fn test_tm_to_time_t_epoch() {
    let tm = Atm {
        tm_sec: 0, tm_min: 0, tm_hour: 0, tm_mday: 1, tm_mon: 0,
        tm_year: 70, tm_wday: 0, tm_yday: 0, tm_isdst: 0, tm_usec: 0,
    };
    assert_eq!(tm_to_time_t(&tm), Some(0));
}

#[test]
fn test_tm_to_time_t_2099_end() {
    let tm = Atm {
        tm_sec: 59, tm_min: 59, tm_hour: 23, tm_mday: 31, tm_mon: 11,
        tm_year: 199, tm_wday: 0, tm_yday: 0, tm_isdst: 0, tm_usec: 0,
    };
    assert_eq!(tm_to_time_t(&tm), Some(4102444799));
}

#[test]
fn test_tm_to_time_t_year_out_of_range() {
    let mut tm = Atm {
        tm_sec: 0, tm_min: 0, tm_hour: 0, tm_mday: 1, tm_mon: 0,
        tm_year: 69, tm_wday: 0, tm_yday: 0, tm_isdst: 0, tm_usec: 0,
    };
    assert_eq!(tm_to_time_t(&tm), None);
    tm.tm_year = 200;
    assert_eq!(tm_to_time_t(&tm), None);
}

#[test]
fn test_tm_to_time_t_month_out_of_range() {
    let mut tm = Atm {
        tm_sec: 0, tm_min: 0, tm_hour: 0, tm_mday: 1, tm_mon: -1,
        tm_year: 70, tm_wday: 0, tm_yday: 0, tm_isdst: 0, tm_usec: 0,
    };
    assert_eq!(tm_to_time_t(&tm), None);
    tm.tm_mon = 12;
    assert_eq!(tm_to_time_t(&tm), None);
}

#[test]
fn test_tm_to_time_t_negative_time() {
    let tm = Atm {
        tm_sec: -1, tm_min: 0, tm_hour: 0, tm_mday: 1, tm_mon: 0,
        tm_year: 70, tm_wday: 0, tm_yday: 0, tm_isdst: 0, tm_usec: 0,
    };
    assert_eq!(tm_to_time_t(&tm), None);
}

#[test]
fn test_tm_to_time_t_leap_year() {
    // Feb 29, 2000
    let tm = Atm {
        tm_sec: 0, tm_min: 0, tm_hour: 0, tm_mday: 29, tm_mon: 1,
        tm_year: 100, tm_wday: 0, tm_yday: 0, tm_isdst: 0, tm_usec: 0,
    };
    assert_eq!(tm_to_time_t(&tm), Some(951782400));
}

// ---- match_string ----

#[test]
fn test_match_string_exact() {
    assert_eq!(match_string("March", "March"), 5);
}

#[test]
fn test_match_string_case_insensitive() {
    assert_eq!(match_string("march", "March"), 5);
    assert_eq!(match_string("MARCH", "March"), 5);
}

#[test]
fn test_match_string_prefix() {
    assert_eq!(match_string("Mar", "March"), 3);
    assert_eq!(match_string("Mar/10", "March"), 3);
}

#[test]
fn test_match_string_no_match() {
    assert_eq!(match_string("April", "March"), 0);
}

#[test]
fn test_match_string_partial_alnum_mismatch() {
    assert_eq!(match_string("Mxy", "March"), 0);
}

// ---- skip_alpha ----

#[test]
fn test_skip_alpha_basic() {
    assert_eq!(skip_alpha("abc123"), 3);
    assert_eq!(skip_alpha("a"), 1);
}

// ---- match_alpha ----

#[test]
fn test_match_alpha_month() {
    let mut tm = Atm {
        tm_sec: -1, tm_min: -1, tm_hour: -1, tm_mday: -1, tm_mon: -1,
        tm_year: -1, tm_wday: 0, tm_yday: 0, tm_isdst: -1, tm_usec: 0,
    };
    let mut offset = -1;
    let m = match_alpha("March", &mut tm, &mut offset);
    assert!(m >= 3);
    assert_eq!(tm.tm_mon, 2); // March = index 2
}

#[test]
fn test_match_alpha_weekday() {
    let mut tm = Atm {
        tm_sec: -1, tm_min: -1, tm_hour: -1, tm_mday: -1, tm_mon: -1,
        tm_year: -1, tm_wday: 0, tm_yday: 0, tm_isdst: -1, tm_usec: 0,
    };
    let mut offset = -1;
    let m = match_alpha("Monday", &mut tm, &mut offset);
    assert!(m >= 3);
    assert_eq!(tm.tm_wday, 1);
}

#[test]
fn test_match_alpha_timezone_utc() {
    let mut tm = Atm {
        tm_sec: -1, tm_min: -1, tm_hour: -1, tm_mday: -1, tm_mon: -1,
        tm_year: -1, tm_wday: 0, tm_yday: 0, tm_isdst: -1, tm_usec: 0,
    };
    let mut offset = -1;
    let m = match_alpha("UTC", &mut tm, &mut offset);
    assert!(m >= 3);
    assert_eq!(offset, 0);
}

#[test]
fn test_match_alpha_timezone_pst() {
    let mut tm = Atm {
        tm_sec: -1, tm_min: -1, tm_hour: -1, tm_mday: -1, tm_mon: -1,
        tm_year: -1, tm_wday: 0, tm_yday: 0, tm_isdst: -1, tm_usec: 0,
    };
    let mut offset = -1;
    match_alpha("PST", &mut tm, &mut offset);
    // PST: offset=-8, dst=false => 60*(-8) = -480
    assert_eq!(offset, -480);
}

#[test]
fn test_match_alpha_timezone_pdt_dst() {
    let mut tm = Atm {
        tm_sec: -1, tm_min: -1, tm_hour: -1, tm_mday: -1, tm_mon: -1,
        tm_year: -1, tm_wday: 0, tm_yday: 0, tm_isdst: -1, tm_usec: 0,
    };
    let mut offset = -1;
    match_alpha("PDT", &mut tm, &mut offset);
    // PDT: offset=-8, dst=true => off = -8+1 = -7 => 60*(-7) = -420
    assert_eq!(offset, -420);
}

#[test]
fn test_match_alpha_pm() {
    let mut tm = Atm {
        tm_sec: 0, tm_min: 30, tm_hour: 1, tm_mday: 10, tm_mon: 2,
        tm_year: 113, tm_wday: 0, tm_yday: 0, tm_isdst: 0, tm_usec: 0,
    };
    let mut offset = -1;
    let m = match_alpha("PM", &mut tm, &mut offset);
    assert_eq!(m, 2);
    assert_eq!(tm.tm_hour, 13); // 1 PM = 13
}

#[test]
fn test_match_alpha_am() {
    let mut tm = Atm {
        tm_sec: 0, tm_min: 30, tm_hour: 13, tm_mday: 10, tm_mon: 2,
        tm_year: 113, tm_wday: 0, tm_yday: 0, tm_isdst: 0, tm_usec: 0,
    };
    let mut offset = -1;
    let m = match_alpha("AM", &mut tm, &mut offset);
    assert_eq!(m, 2);
    assert_eq!(tm.tm_hour, 1); // 13 % 12 = 1
}

// ---- nodate ----

#[test]
fn test_nodate_all_negative() {
    let mut tm = Atm {
        tm_sec: -1, tm_min: -1, tm_hour: -1, tm_mday: -1, tm_mon: -1,
        tm_year: -1, tm_wday: 0, tm_yday: 0, tm_isdst: 0, tm_usec: 0,
    };
    assert_eq!(nodate(&mut tm), 1);
}

#[test]
fn test_nodate_some_set() {
    let mut tm = Atm {
        tm_sec: -1, tm_min: -1, tm_hour: -1, tm_mday: -1, tm_mon: -1,
        tm_year: 113, tm_wday: 0, tm_yday: 0, tm_isdst: 0, tm_usec: 0,
    };
    assert_eq!(nodate(&mut tm), 0);
}

// ---- match_tz ----

#[test]
fn test_match_tz_positive() {
    let mut off = -1;
    match_tz("+0500", &mut off);
    assert_eq!(off, 300);
}

#[test]
fn test_match_tz_negative() {
    let mut off = -1;
    match_tz("-0500", &mut off);
    assert_eq!(off, -300);
}

#[test]
fn test_match_tz_with_colon() {
    let mut off = -1;
    match_tz("+05:30", &mut off);
    assert_eq!(off, 330);
}

#[test]
fn test_match_tz_1400() {
    let mut off = -1;
    match_tz("+1400", &mut off);
    assert_eq!(off, 840);
}

#[test]
fn test_match_tz_0110_neg() {
    let mut off = -1;
    match_tz("-0110", &mut off);
    assert_eq!(off, -70);
}

// ---- match_object_header_date ----

#[test]
fn test_object_header_date_basic() {
    let mut tv = TimeVal { tv_sec: 0, tv_usec: 0 };
    let mut offset = 0;
    let r = match_object_header_date("1362873602 +0000", &mut tv, &mut offset);
    assert_eq!(r, 0);
    assert_eq!(tv.tv_sec, 1362873602);
    assert_eq!(offset, 0);
}

#[test]
fn test_object_header_date_zero() {
    let mut tv = TimeVal { tv_sec: 0, tv_usec: 0 };
    let mut offset = 0;
    let r = match_object_header_date("0 +0000", &mut tv, &mut offset);
    assert_eq!(r, 0);
    assert_eq!(tv.tv_sec, 0);
    assert_eq!(offset, 0);
}

#[test]
fn test_object_header_date_negative_offset() {
    let mut tv = TimeVal { tv_sec: 0, tv_usec: 0 };
    let mut offset = 0;
    let r = match_object_header_date("1362873602 -0500", &mut tv, &mut offset);
    assert_eq!(r, 0);
    assert_eq!(tv.tv_sec, 1362873602);
    assert_eq!(offset, -300);
}

#[test]
fn test_object_header_date_invalid() {
    let mut tv = TimeVal { tv_sec: 0, tv_usec: 0 };
    let mut offset = 0;
    assert_eq!(match_object_header_date("", &mut tv, &mut offset), -1);
    assert_eq!(match_object_header_date("abc", &mut tv, &mut offset), -1);
}
