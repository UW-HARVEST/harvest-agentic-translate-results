use approxidate::approxidate::*;

fn main() {}

// ---- parse_date_basic ----

#[test]
fn test_parse_date_basic_utc() {
    let mut tv = TimeVal { tv_sec: 0, tv_usec: 0 };
    let mut offset = 0;
    let r = parse_date_basic("10/Mar/2013:00:00:02.003 UTC", &mut tv, &mut offset);
    assert_eq!(r, 0);
    assert_eq!(tv.tv_sec, 1362873602);
    assert_eq!(tv.tv_usec, 3000);
}

#[test]
fn test_parse_date_basic_utc_no_usec() {
    let mut tv = TimeVal { tv_sec: 0, tv_usec: 0 };
    let mut offset = 0;
    let r = parse_date_basic("10/Mar/2013:00:00:02 UTC", &mut tv, &mut offset);
    assert_eq!(r, 0);
    assert_eq!(tv.tv_sec, 1362873602);
    assert_eq!(tv.tv_usec, 0);
}

#[test]
fn test_parse_date_basic_positive_offset() {
    let mut tv = TimeVal { tv_sec: 0, tv_usec: 0 };
    let mut offset = 0;
    let r = parse_date_basic("10/Mar/2012:00:00:07 +0500", &mut tv, &mut offset);
    assert_eq!(r, 0);
    assert_eq!(tv.tv_sec, 1331319607);
}

#[test]
fn test_parse_date_basic_usec_with_offset() {
    let mut tv = TimeVal { tv_sec: 0, tv_usec: 0 };
    let mut offset = 0;
    let r = parse_date_basic("10/Mar/2012:00:00:07.657891 +0500", &mut tv, &mut offset);
    assert_eq!(r, 0);
    assert_eq!(tv.tv_sec, 1331319607);
    assert_eq!(tv.tv_usec, 657891);
}

#[test]
fn test_parse_date_basic_large_positive_offset() {
    let mut tv = TimeVal { tv_sec: 0, tv_usec: 0 };
    let mut offset = 0;
    let r = parse_date_basic("10/Mar/2012:00:00:07.657891 +1400", &mut tv, &mut offset);
    assert_eq!(r, 0);
    assert_eq!(tv.tv_sec, 1331287207);
    assert_eq!(tv.tv_usec, 657891);
}

#[test]
fn test_parse_date_basic_negative_offset() {
    let mut tv = TimeVal { tv_sec: 0, tv_usec: 0 };
    let mut offset = 0;
    let r = parse_date_basic("10/Mar/2012:00:00:07.657891 -0110", &mut tv, &mut offset);
    assert_eq!(r, 0);
    assert_eq!(tv.tv_sec, 1331341807);
    assert_eq!(tv.tv_usec, 657891);
}

#[test]
fn test_parse_date_basic_month_first() {
    let mut tv = TimeVal { tv_sec: 0, tv_usec: 0 };
    let mut offset = 0;
    let r = parse_date_basic("mar 10 2013 00:00:07 UTC", &mut tv, &mut offset);
    assert_eq!(r, 0);
    assert_eq!(tv.tv_sec, 1362873607);
}

#[test]
fn test_parse_date_basic_with_neg_tz_offset() {
    let mut tv = TimeVal { tv_sec: 0, tv_usec: 0 };
    let mut offset = 0;
    let r = parse_date_basic("mar 10 2013 04:00:07 -0500", &mut tv, &mut offset);
    assert_eq!(r, 0);
    assert_eq!(tv.tv_sec, 1362906007);
}

#[test]
fn test_parse_date_basic_full_month_name() {
    let mut tv = TimeVal { tv_sec: 0, tv_usec: 0 };
    let mut offset = 0;
    let r = parse_date_basic("march 10 2013 04:00:07 -0500", &mut tv, &mut offset);
    assert_eq!(r, 0);
    assert_eq!(tv.tv_sec, 1362906007);
}

#[test]
fn test_parse_date_basic_day_month_year() {
    let mut tv = TimeVal { tv_sec: 0, tv_usec: 0 };
    let mut offset = 0;
    let r = parse_date_basic("10 march 2013 04:00:07 -0500", &mut tv, &mut offset);
    assert_eq!(r, 0);
    assert_eq!(tv.tv_sec, 1362906007);
}

#[test]
fn test_parse_date_basic_year_day_month() {
    let mut tv = TimeVal { tv_sec: 0, tv_usec: 0 };
    let mut offset = 0;
    let r = parse_date_basic("2013 10 march 04:00:07 -0500", &mut tv, &mut offset);
    assert_eq!(r, 0);
    assert_eq!(tv.tv_sec, 1362906007);
}

#[test]
fn test_parse_date_basic_year_month_day() {
    let mut tv = TimeVal { tv_sec: 0, tv_usec: 0 };
    let mut offset = 0;
    let r = parse_date_basic("2013 march 10 04:00:07 -0500", &mut tv, &mut offset);
    assert_eq!(r, 0);
    assert_eq!(tv.tv_sec, 1362906007);
}

#[test]
fn test_parse_date_basic_object_header() {
    let mut tv = TimeVal { tv_sec: 0, tv_usec: 0 };
    let mut offset = 0;
    let r = parse_date_basic("@1362873602 +0000", &mut tv, &mut offset);
    assert_eq!(r, 0);
    assert_eq!(tv.tv_sec, 1362873602);
    assert_eq!(offset, 0);
}

#[test]
fn test_parse_date_basic_object_header_zero() {
    let mut tv = TimeVal { tv_sec: 0, tv_usec: 0 };
    let mut offset = 0;
    let r = parse_date_basic("@0 +0000", &mut tv, &mut offset);
    assert_eq!(r, 0);
    assert_eq!(tv.tv_sec, 0);
}

#[test]
fn test_parse_date_basic_object_header_neg_offset() {
    let mut tv = TimeVal { tv_sec: 0, tv_usec: 0 };
    let mut offset = 0;
    let r = parse_date_basic("@1362873602 -0500", &mut tv, &mut offset);
    assert_eq!(r, 0);
    assert_eq!(tv.tv_sec, 1362873602);
    assert_eq!(offset, -300);
}

#[test]
fn test_parse_date_basic_epoch_number() {
    let mut tv = TimeVal { tv_sec: 0, tv_usec: 0 };
    let mut offset = 0;
    let r = parse_date_basic("1362873602", &mut tv, &mut offset);
    assert_eq!(r, 0);
    assert_eq!(tv.tv_sec, 1362873602);
}

#[test]
fn test_parse_date_basic_gmt() {
    let mut tv = TimeVal { tv_sec: 0, tv_usec: 0 };
    let mut offset = 0;
    let r = parse_date_basic("10/Mar/2013:00:00:02 GMT", &mut tv, &mut offset);
    assert_eq!(r, 0);
    assert_eq!(tv.tv_sec, 1362873602);
}

#[test]
fn test_parse_date_basic_pst() {
    let mut tv = TimeVal { tv_sec: 0, tv_usec: 0 };
    let mut offset = 0;
    let r = parse_date_basic("10/Mar/2013:00:00:02 PST", &mut tv, &mut offset);
    assert_eq!(r, 0);
    assert_eq!(tv.tv_sec, 1362902402);
}

#[test]
fn test_parse_date_basic_est() {
    let mut tv = TimeVal { tv_sec: 0, tv_usec: 0 };
    let mut offset = 0;
    let r = parse_date_basic("10/Mar/2013:00:00:02 EST", &mut tv, &mut offset);
    assert_eq!(r, 0);
    assert_eq!(tv.tv_sec, 1362891602);
}

#[test]
fn test_parse_date_basic_jst() {
    let mut tv = TimeVal { tv_sec: 0, tv_usec: 0 };
    let mut offset = 0;
    let r = parse_date_basic("10/Mar/2013:00:00:02 JST", &mut tv, &mut offset);
    assert_eq!(r, 0);
    assert_eq!(tv.tv_sec, 1362841202);
}

#[test]
fn test_parse_date_basic_z_timezone() {
    let mut tv = TimeVal { tv_sec: 0, tv_usec: 0 };
    let mut offset = 0;
    let r = parse_date_basic("10/Mar/2013:00:00:02 Z", &mut tv, &mut offset);
    assert_eq!(r, 0);
    assert_eq!(tv.tv_sec, 1362873602);
}

#[test]
fn test_parse_date_basic_pdt_dst() {
    let mut tv = TimeVal { tv_sec: 0, tv_usec: 0 };
    let mut offset = 0;
    let r = parse_date_basic("10/Mar/2013:00:00:02 PDT", &mut tv, &mut offset);
    assert_eq!(r, 0);
    assert_eq!(tv.tv_sec, 1362898802);
}

#[test]
fn test_parse_date_basic_bst_dst() {
    let mut tv = TimeVal { tv_sec: 0, tv_usec: 0 };
    let mut offset = 0;
    let r = parse_date_basic("10/Mar/2013:00:00:02 BST", &mut tv, &mut offset);
    assert_eq!(r, 0);
    assert_eq!(tv.tv_sec, 1362870002);
}

#[test]
fn test_parse_date_basic_nzdt_dst() {
    let mut tv = TimeVal { tv_sec: 0, tv_usec: 0 };
    let mut offset = 0;
    let r = parse_date_basic("10/Mar/2013:00:00:02 NZDT", &mut tv, &mut offset);
    assert_eq!(r, 0);
    assert_eq!(tv.tv_sec, 1362826802);
}

#[test]
fn test_parse_date_basic_epoch_start() {
    let mut tv = TimeVal { tv_sec: 0, tv_usec: 0 };
    let mut offset = 0;
    let r = parse_date_basic("1/Jan/1970:00:00:00 UTC", &mut tv, &mut offset);
    assert_eq!(r, 0);
    assert_eq!(tv.tv_sec, 0);
}

#[test]
fn test_parse_date_basic_2099_end() {
    let mut tv = TimeVal { tv_sec: 0, tv_usec: 0 };
    let mut offset = 0;
    let r = parse_date_basic("31/Dec/2099:23:59:59 UTC", &mut tv, &mut offset);
    assert_eq!(r, 0);
    assert_eq!(tv.tv_sec, 4102444799);
}

#[test]
fn test_parse_date_basic_leap_year_2000() {
    let mut tv = TimeVal { tv_sec: 0, tv_usec: 0 };
    let mut offset = 0;
    let r = parse_date_basic("29/Feb/2000:00:00:00 UTC", &mut tv, &mut offset);
    assert_eq!(r, 0);
    assert_eq!(tv.tv_sec, 951782400);
}

#[test]
fn test_parse_date_basic_leap_year_2004() {
    let mut tv = TimeVal { tv_sec: 0, tv_usec: 0 };
    let mut offset = 0;
    let r = parse_date_basic("29/Feb/2004:00:00:00 UTC", &mut tv, &mut offset);
    assert_eq!(r, 0);
    assert_eq!(tv.tv_sec, 1078012800);
}

#[test]
fn test_parse_date_basic_iso_ish() {
    let mut tv = TimeVal { tv_sec: 0, tv_usec: 0 };
    let mut offset = 0;
    let r = parse_date_basic("2013-03-10T00:00:02 UTC", &mut tv, &mut offset);
    assert_eq!(r, 0);
    assert_eq!(tv.tv_sec, 1362873602);
}

#[test]
fn test_parse_date_basic_pm() {
    let mut tv = TimeVal { tv_sec: 0, tv_usec: 0 };
    let mut offset = 0;
    let r = parse_date_basic("10/Mar/2013:01:30:00 PM UTC", &mut tv, &mut offset);
    assert_eq!(r, 0);
    assert_eq!(tv.tv_sec, 1362922200);
}

#[test]
fn test_parse_date_basic_am() {
    let mut tv = TimeVal { tv_sec: 0, tv_usec: 0 };
    let mut offset = 0;
    let r = parse_date_basic("10/Mar/2013:01:30:00 AM UTC", &mut tv, &mut offset);
    assert_eq!(r, 0);
    assert_eq!(tv.tv_sec, 1362879000);
}

#[test]
fn test_parse_date_basic_neg_0500_offset() {
    let mut tv = TimeVal { tv_sec: 0, tv_usec: 0 };
    let mut offset = 0;
    let r = parse_date_basic("10/Mar/2013:00:00:02 -0500", &mut tv, &mut offset);
    assert_eq!(r, 0);
    assert_eq!(tv.tv_sec, 1362891602);
}

#[test]
fn test_parse_date_basic_pos_0530_offset() {
    let mut tv = TimeVal { tv_sec: 0, tv_usec: 0 };
    let mut offset = 0;
    let r = parse_date_basic("10/Mar/2013:00:00:02 +0530", &mut tv, &mut offset);
    assert_eq!(r, 0);
    assert_eq!(tv.tv_sec, 1362853802);
}

// ---- usec precision ----

#[test]
fn test_usec_1_digit() {
    let mut tv = TimeVal { tv_sec: 0, tv_usec: 0 };
    let mut offset = 0;
    parse_date_basic("10/Mar/2013:00:00:02.1 UTC", &mut tv, &mut offset);
    assert_eq!(tv.tv_usec, 100000);
}

#[test]
fn test_usec_2_digits() {
    let mut tv = TimeVal { tv_sec: 0, tv_usec: 0 };
    let mut offset = 0;
    parse_date_basic("10/Mar/2013:00:00:02.12 UTC", &mut tv, &mut offset);
    assert_eq!(tv.tv_usec, 120000);
}

#[test]
fn test_usec_3_digits() {
    let mut tv = TimeVal { tv_sec: 0, tv_usec: 0 };
    let mut offset = 0;
    parse_date_basic("10/Mar/2013:00:00:02.123 UTC", &mut tv, &mut offset);
    assert_eq!(tv.tv_usec, 123000);
}

#[test]
fn test_usec_4_digits() {
    let mut tv = TimeVal { tv_sec: 0, tv_usec: 0 };
    let mut offset = 0;
    parse_date_basic("10/Mar/2013:00:00:02.1234 UTC", &mut tv, &mut offset);
    assert_eq!(tv.tv_usec, 123400);
}

#[test]
fn test_usec_5_digits() {
    let mut tv = TimeVal { tv_sec: 0, tv_usec: 0 };
    let mut offset = 0;
    parse_date_basic("10/Mar/2013:00:00:02.12345 UTC", &mut tv, &mut offset);
    assert_eq!(tv.tv_usec, 123450);
}

#[test]
fn test_usec_6_digits() {
    let mut tv = TimeVal { tv_sec: 0, tv_usec: 0 };
    let mut offset = 0;
    parse_date_basic("10/Mar/2013:00:00:02.123456 UTC", &mut tv, &mut offset);
    assert_eq!(tv.tv_usec, 123456);
}
