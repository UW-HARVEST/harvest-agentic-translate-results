use chrono::{Datelike, TimeZone as ChronoTimeZone, Timelike};

pub struct Atm {
    pub tm_sec: i32,
    pub tm_min: i32,
    pub tm_hour: i32,
    pub tm_mday: i32,
    pub tm_mon: i32,
    pub tm_year: i32,
    pub tm_wday: i32,
    pub tm_yday: i32,
    pub tm_isdst: i32,
    pub tm_usec: i64,
}
#[derive(Debug, Clone, Copy)]
pub struct TimeVal {
    pub tv_sec: i64,
    pub tv_usec: i64,
}
pub type TimeT = i64;
const GIT_SPACE: u8 = 0x01;
const GIT_DIGIT: u8 = 0x02;
const GIT_ALPHA: u8 = 0x04;
const GIT_GLOB_SPECIAL: u8 = 0x08;
const GIT_REGEX_SPECIAL: u8 = 0x10;
const GIT_PATHSPEC_MAGIC: u8 = 0x20;
const GIT_CNTRL: u8 = 0x40;
const GIT_PUNCT: u8 = 0x80;

// ---------- Helpers ----------

#[inline]
fn is_digit_byte(c: u8) -> bool {
    c >= b'0' && c <= b'9'
}
#[inline]
fn is_alpha_byte(c: u8) -> bool {
    (c >= b'A' && c <= b'Z') || (c >= b'a' && c <= b'z')
}
#[inline]
fn is_alnum_byte(c: u8) -> bool {
    is_alpha_byte(c) || is_digit_byte(c)
}
#[inline]
fn to_upper(c: u8) -> u8 {
    if is_alpha_byte(c) {
        c & !0x20
    } else {
        c
    }
}

fn new_atm() -> Atm {
    Atm {
        tm_sec: 0,
        tm_min: 0,
        tm_hour: 0,
        tm_mday: 0,
        tm_mon: 0,
        tm_year: 0,
        tm_wday: 0,
        tm_yday: 0,
        tm_isdst: 0,
        tm_usec: 0,
    }
}

fn copy_atm(src: &Atm) -> Atm {
    Atm {
        tm_sec: src.tm_sec,
        tm_min: src.tm_min,
        tm_hour: src.tm_hour,
        tm_mday: src.tm_mday,
        tm_mon: src.tm_mon,
        tm_year: src.tm_year,
        tm_wday: src.tm_wday,
        tm_yday: src.tm_yday,
        tm_isdst: src.tm_isdst,
        tm_usec: src.tm_usec,
    }
}

// time_t mktime(struct tm *) - local time -> seconds since epoch
fn mktime_local(tm: &Atm) -> i64 {
    let year = tm.tm_year + 1900;
    let mon = tm.tm_mon + 1;
    let local = chrono::Local;
    let dt = local.with_ymd_and_hms(
        year,
        mon as u32,
        tm.tm_mday as u32,
        tm.tm_hour as u32,
        tm.tm_min as u32,
        tm.tm_sec as u32,
    );
    match dt {
        chrono::LocalResult::Single(t) => t.timestamp(),
        chrono::LocalResult::Ambiguous(t, _) => t.timestamp(),
        chrono::LocalResult::None => -1,
    }
}

// localtime_r equivalent: fills date/time fields from time_t (preserves tm_usec)
fn localtime_r(time_sec: i64, tm: &mut Atm) {
    let local = chrono::Local;
    let dt_opt = local.timestamp_opt(time_sec, 0);
    if let chrono::LocalResult::Single(t) = dt_opt {
        let t: chrono::DateTime<chrono::Local> = t;
        tm.tm_sec = t.second() as i32;
        tm.tm_min = t.minute() as i32;
        tm.tm_hour = t.hour() as i32;
        tm.tm_mday = t.day() as i32;
        tm.tm_mon = t.month0() as i32;
        tm.tm_year = (t.year() - 1900) as i32;
        tm.tm_wday = t.weekday().num_days_from_sunday() as i32;
        tm.tm_yday = t.ordinal0() as i32;
        tm.tm_isdst = -1;
    }
}

// gmtime_r equivalent: fills fields from time_t in UTC. Returns true on success.
fn gmtime_r(time_sec: i64, tm: &mut Atm) -> bool {
    let utc = chrono::Utc;
    let dt_opt = utc.timestamp_opt(time_sec, 0);
    if let chrono::LocalResult::Single(t) = dt_opt {
        let t: chrono::DateTime<chrono::Utc> = t;
        tm.tm_sec = t.second() as i32;
        tm.tm_min = t.minute() as i32;
        tm.tm_hour = t.hour() as i32;
        tm.tm_mday = t.day() as i32;
        tm.tm_mon = t.month0() as i32;
        tm.tm_year = (t.year() - 1900) as i32;
        tm.tm_wday = t.weekday().num_days_from_sunday() as i32;
        tm.tm_yday = t.ordinal0() as i32;
        tm.tm_isdst = 0;
        true
    } else {
        false
    }
}

// strtoul/strtol: parse leading digits as a number; return (value, consumed_bytes).
// Skips no leading whitespace (mirrors typical use here, though strtol does skip ws,
// none of our call sites depend on that).
fn parse_ulong(s: &[u8]) -> (u64, usize) {
    let mut i = 0;
    // skip leading whitespace (strtoul behavior)
    while i < s.len() && (s[i] == b' ' || s[i] == b'\t' || s[i] == b'\n' || s[i] == b'\r') {
        i += 1;
    }
    let start = i;
    let mut val: u64 = 0;
    while i < s.len() && is_digit_byte(s[i]) {
        val = val.saturating_mul(10).saturating_add((s[i] - b'0') as u64);
        i += 1;
    }
    if i == start {
        // no digits parsed, but consumed any leading whitespace
        // strtoul returns 0 with end == nptr in that case
        return (0, 0);
    }
    (val, i)
}

fn parse_long(s: &[u8]) -> (i64, usize) {
    let mut i = 0;
    while i < s.len() && (s[i] == b' ' || s[i] == b'\t' || s[i] == b'\n' || s[i] == b'\r') {
        i += 1;
    }
    let mut neg = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        neg = s[i] == b'-';
        i += 1;
    }
    let start_digits = i;
    let mut val: i64 = 0;
    while i < s.len() && is_digit_byte(s[i]) {
        val = val.saturating_mul(10).saturating_add((s[i] - b'0') as i64);
        i += 1;
    }
    if i == start_digits {
        return (0, 0);
    }
    if neg {
        val = -val;
    }
    (val, i)
}

pub fn tm_to_time_t(tm: &Atm) -> Option<i64> {
    let mdays: [i32; 12] = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];
    let year = tm.tm_year - 70;
    let month = tm.tm_mon;
    let mut day = tm.tm_mday;
    if year < 0 || year > 129 {
        return None;
    }
    if month < 0 || month > 11 {
        return None;
    }
    if month < 2 || ((year + 2).rem_euclid(4)) != 0 {
        day -= 1;
    }
    if tm.tm_hour < 0 || tm.tm_min < 0 || tm.tm_sec < 0 {
        return None;
    }
    Some(
        (year as i64 * 365
            + ((year as i64 + 1) / 4)
            + mdays[month as usize] as i64
            + day as i64)
            * 24
            * 60
            * 60
            + tm.tm_hour as i64 * 60 * 60
            + tm.tm_min as i64 * 60
            + tm.tm_sec as i64,
    )
}

// internal version that returns -1 on failure to mirror C usage
fn tm_to_time_t_or_neg(tm: &Atm) -> i64 {
    tm_to_time_t(tm).unwrap_or(-1)
}

pub const MONTH_NAMES: [&str; 12] = [
    "January", "February", "March", "April", "May", "June", "July", "August", "September",
    "October", "November", "December",
];
pub const WEEKDAY_NAMES: [&str; 7] = [
    "Sundays",
    "Mondays",
    "Tuesdays",
    "Wednesdays",
    "Thursdays",
    "Fridays",
    "Saturdays",
];

pub struct TimeZone {
    pub name: &'static str,
    pub offset: i32,
    pub dst: bool,
}

pub const TIMEZONE_NAMES: [TimeZone; 44] = [
    TimeZone { name: "IDLW", offset: -12, dst: false },
    TimeZone { name: "NT", offset: -11, dst: false },
    TimeZone { name: "CAT", offset: -10, dst: false },
    TimeZone { name: "HST", offset: -10, dst: false },
    TimeZone { name: "HDT", offset: -10, dst: true },
    TimeZone { name: "YST", offset: -9, dst: false },
    TimeZone { name: "YDT", offset: -9, dst: true },
    TimeZone { name: "PST", offset: -8, dst: false },
    TimeZone { name: "PDT", offset: -8, dst: true },
    TimeZone { name: "MST", offset: -7, dst: false },
    TimeZone { name: "MDT", offset: -7, dst: true },
    TimeZone { name: "CST", offset: -6, dst: false },
    TimeZone { name: "CDT", offset: -6, dst: true },
    TimeZone { name: "EST", offset: -5, dst: false },
    TimeZone { name: "EDT", offset: -5, dst: true },
    TimeZone { name: "AST", offset: -3, dst: false },
    TimeZone { name: "ADT", offset: -3, dst: true },
    TimeZone { name: "WAT", offset: -1, dst: false },
    TimeZone { name: "GMT", offset: 0, dst: false },
    TimeZone { name: "UTC", offset: 0, dst: false },
    TimeZone { name: "Z", offset: 0, dst: false },
    TimeZone { name: "WET", offset: 0, dst: false },
    TimeZone { name: "BST", offset: 0, dst: true },
    TimeZone { name: "CET", offset: 1, dst: false },
    TimeZone { name: "MET", offset: 1, dst: false },
    TimeZone { name: "MEWT", offset: 1, dst: false },
    TimeZone { name: "MEST", offset: 1, dst: true },
    TimeZone { name: "CEST", offset: 1, dst: true },
    TimeZone { name: "MESZ", offset: 1, dst: true },
    TimeZone { name: "FWT", offset: 1, dst: false },
    TimeZone { name: "FST", offset: 1, dst: true },
    TimeZone { name: "EET", offset: 2, dst: false },
    TimeZone { name: "EEST", offset: 2, dst: true },
    TimeZone { name: "WAST", offset: 7, dst: false },
    TimeZone { name: "WADT", offset: 7, dst: true },
    TimeZone { name: "CCT", offset: 8, dst: false },
    TimeZone { name: "JST", offset: 9, dst: false },
    TimeZone { name: "EAST", offset: 10, dst: false },
    TimeZone { name: "EADT", offset: 10, dst: true },
    TimeZone { name: "GST", offset: 10, dst: false },
    TimeZone { name: "NZT", offset: 12, dst: false },
    TimeZone { name: "NZST", offset: 12, dst: false },
    TimeZone { name: "NZDT", offset: 12, dst: true },
    TimeZone { name: "IDLE", offset: 12, dst: false },
];

// Compares date prefix to format (str). Returns number of bytes matched (case-insensitive).
fn match_string_b(date: &[u8], fmt: &[u8]) -> i32 {
    let mut i = 0usize;
    let mut di = 0usize;
    let mut si = 0usize;
    while di < date.len() {
        let dc = date[di];
        let sc = if si < fmt.len() { fmt[si] } else { 0 };
        if dc == sc {
            // continue
        } else if to_upper(dc) == to_upper(sc) {
            // continue
        } else if !is_alnum_byte(dc) {
            break;
        } else {
            return 0;
        }
        di += 1;
        si += 1;
        i += 1;
    }
    i as i32
}

pub fn match_string(date: &str, format: &str) -> i32 {
    match_string_b(date.as_bytes(), format.as_bytes())
}

fn skip_alpha_b(date: &[u8]) -> i32 {
    let mut i: usize = 0;
    loop {
        i += 1;
        if i >= date.len() || !is_alpha_byte(date[i]) {
            break;
        }
    }
    i as i32
}

pub fn skip_alpha(date: &str) -> i32 {
    skip_alpha_b(date.as_bytes())
}

fn match_alpha_b(date: &[u8], tm: &mut Atm, offset: &mut i32) -> i32 {
    for i in 0..12 {
        let m = match_string_b(date, MONTH_NAMES[i].as_bytes());
        if m >= 3 {
            tm.tm_mon = i as i32;
            return m;
        }
    }
    for i in 0..7 {
        let m = match_string_b(date, WEEKDAY_NAMES[i].as_bytes());
        if m >= 3 {
            tm.tm_wday = i as i32;
            return m;
        }
    }
    for tz in TIMEZONE_NAMES.iter() {
        let m = match_string_b(date, tz.name.as_bytes());
        if m >= 3 || (m as usize) == tz.name.len() {
            let mut off = tz.offset;
            if tz.dst {
                off += 1;
            }
            if *offset == -1 {
                *offset = 60 * off;
            }
            return m;
        }
    }
    if match_string_b(date, b"PM") == 2 {
        tm.tm_hour = (tm.tm_hour % 12) + 12;
        return 2;
    }
    if match_string_b(date, b"AM") == 2 {
        tm.tm_hour = (tm.tm_hour % 12) + 0;
        return 2;
    }
    skip_alpha_b(date)
}

pub fn match_alpha(date: &str, tm: &mut Atm, offset: &mut i32) -> i32 {
    match_alpha_b(date.as_bytes(), tm, offset)
}

fn is_date_internal(year: i32, month: i32, day: i32, tm: &mut Atm) -> i32 {
    if month > 0 && month < 13 && day > 0 && day < 32 {
        tm.tm_mon = month - 1;
        tm.tm_mday = day;
        if year == -1 {
            return 1;
        } else if year >= 1970 && year < 2100 {
            tm.tm_year = year - 1900;
        } else if year > 70 && year < 100 {
            tm.tm_year = year;
        } else if year < 38 {
            tm.tm_year = year + 100;
        } else {
            return 0;
        }
        return 1;
    }
    0
}

pub fn is_date(year: i32, month: i32, day: i32, _now_tm: &Atm, _now: TimeT, tm: &mut Atm) -> i32 {
    is_date_internal(year, month, day, tm)
}

// Returns total bytes consumed from `date` (not from `end`).
// `end_offset_in_date` is the position of `end` within `date`.
fn match_multi_number_b(
    num: u64,
    c: u8,
    date: &[u8],
    end_offset_in_date: usize,
    tm: &mut Atm,
    now: TimeT,
) -> i32 {
    // We are at end_offset_in_date which currently points to the separator char `c`.
    let mut p = end_offset_in_date;
    // num2 = strtol(end+1, &end, 10)
    let (num2, consumed2) = parse_long(&date[p + 1..]);
    p = p + 1 + consumed2;

    let mut num3: i64 = -1;
    let mut num4: i64 = 0;

    if p < date.len() && date[p] == c && (p + 1) < date.len() && is_digit_byte(date[p + 1]) {
        let (n3, c3) = parse_long(&date[p + 1..]);
        num3 = n3;
        p = p + 1 + c3;
        if p < date.len() && date[p] == b'.' {
            let start = p + 1;
            let (n4, c4) = parse_long(&date[start..]);
            num4 = n4;
            p = start + c4;
            let length = p - start;
            if length < 6 {
                num4 *= 10i64.pow((6 - length) as u32);
            }
        }
    }

    match c {
        b':' => {
            if num3 < 0 {
                num3 = 0;
            }
            if num < 25 && num2 >= 0 && num2 < 60 && num3 >= 0 && num3 <= 60 {
                tm.tm_hour = num as i32;
                tm.tm_min = num2 as i32;
                tm.tm_sec = num3 as i32;
                tm.tm_usec = num4;
            } else {
                return 0;
            }
        }
        b'-' | b'/' | b'.' => {
            let _ = now; // not needed; we don't compute now lazily
            if num > 70 {
                if is_date_internal(num as i32, num2 as i32, num3 as i32, tm) != 0 {
                    return p as i32;
                }
                if is_date_internal(num as i32, num3 as i32, num2 as i32, tm) != 0 {
                    return p as i32;
                }
            }
            if c != b'.'
                && is_date_internal(num3 as i32, num as i32, num2 as i32, tm) != 0
            {
                return p as i32;
            }
            if is_date_internal(num3 as i32, num2 as i32, num as i32, tm) != 0 {
                return p as i32;
            }
            if c == b'.'
                && is_date_internal(num3 as i32, num as i32, num2 as i32, tm) != 0
            {
                return p as i32;
            }
            return 0;
        }
        _ => return 0,
    }
    p as i32
}

pub fn match_multi_number(
    num: u64,
    c: char,
    date: &str,
    end: &str,
    tm: &mut Atm,
    now: TimeT,
) -> i32 {
    let date_b = date.as_bytes();
    let end_b = end.as_bytes();
    // Compute end_offset = date.len() - end.len() if end is a suffix of date.
    let end_offset = if end_b.len() <= date_b.len() {
        date_b.len() - end_b.len()
    } else {
        0
    };
    let cb = c as u32;
    if cb > 0xFF {
        return 0;
    }
    match_multi_number_b(num, cb as u8, date_b, end_offset, tm, now)
}

fn nodate_internal(tm: &Atm) -> bool {
    (tm.tm_year & tm.tm_mon & tm.tm_mday & tm.tm_hour & tm.tm_min & tm.tm_sec) < 0
}

pub fn nodate(tm: &mut Atm) -> i32 {
    if nodate_internal(tm) {
        1
    } else {
        0
    }
}

fn match_digit_b(date: &[u8], tm: &mut Atm, offset: &mut i32, tm_gmt: &mut i32) -> i32 {
    let (num, consumed) = parse_ulong(date);
    // unix timestamps with 9+ digits
    if num >= 100_000_000 && nodate_internal(tm) {
        let time = num as i64;
        if gmtime_r(time, tm) {
            *tm_gmt = 1;
            return consumed as i32;
        }
    }

    // Check for special formats: num[-.:/]num[same]num[.secfracs]
    if consumed < date.len() {
        let c = date[consumed];
        if c == b':' || c == b'.' || c == b'/' || c == b'-' {
            if consumed + 1 < date.len() && is_digit_byte(date[consumed + 1]) {
                let m = match_multi_number_b(num, c, date, consumed, tm, 0);
                if m != 0 {
                    return m;
                }
            }
        }
    }

    // None of the special formats? Try to guess based on number of digits.
    let mut n: usize = 0;
    loop {
        n += 1;
        if n >= date.len() || !is_digit_byte(date[n]) {
            break;
        }
    }

    // 4-digit year or timezone
    if n == 4 {
        if num <= 1400 && *offset == -1 {
            let minutes = num % 100;
            let hours = num / 100;
            *offset = (hours * 60 + minutes) as i32;
        } else if num > 1900 && num < 2100 {
            tm.tm_year = (num as i32) - 1900;
        }
        return n as i32;
    }

    if n > 2 {
        return n as i32;
    }

    if num > 0 && num < 32 && tm.tm_mday < 0 {
        tm.tm_mday = num as i32;
        return n as i32;
    }

    if n == 2 && tm.tm_year < 0 {
        if num < 10 && tm.tm_mday >= 0 {
            tm.tm_year = num as i32 + 100;
            return n as i32;
        }
        if num >= 70 {
            tm.tm_year = num as i32;
            return n as i32;
        }
    }

    if num > 0 && num < 13 && tm.tm_mon < 0 {
        tm.tm_mon = (num as i32) - 1;
    }

    n as i32
}

pub fn match_digit(date: &str, tm: &mut Atm, offset: &mut i32, tm_gmt: &mut i32) -> i32 {
    match_digit_b(date.as_bytes(), tm, offset, tm_gmt)
}

fn match_tz_b(date: &[u8], offp: &mut i32) -> i32 {
    // date[0] is '+' or '-'
    let (hour_l, mut consumed) = parse_ulong(&date[1..]);
    let mut hour = hour_l as i32;
    let mut n = consumed;
    let mut min: i32 = 0;

    if n == 4 {
        min = (hour % 100) as i32;
        hour = hour / 100;
    } else if n != 2 {
        min = 99;
    } else if (1 + n) < date.len() && date[1 + n] == b':' {
        // hh:mm?
        let (m, mc) = parse_ulong(&date[1 + n + 1..]);
        min = m as i32;
        consumed = n + 1 + mc;
        if consumed != 5 {
            min = 99;
        }
        n = consumed;
    }

    if min < 60 && hour < 24 {
        let mut tz_offset = hour * 60 + min;
        if date[0] == b'-' {
            tz_offset = -tz_offset;
        }
        *offp = tz_offset;
    }
    // n bytes consumed after '+' / '-', total = 1 + n
    let _ = hour_l;
    (1 + n) as i32
}

pub fn match_tz(date: &str, offp: &mut i32) -> i32 {
    match_tz_b(date.as_bytes(), offp)
}

fn match_object_header_date_b(date: &[u8], tv: &mut TimeVal, offset: &mut i32) -> i32 {
    if date.is_empty() || date[0] < b'0' || date[0] > b'9' {
        return -1;
    }
    let (stamp, consumed) = parse_ulong(date);
    let end = consumed;
    if end >= date.len() || date[end] != b' ' || stamp == u64::MAX {
        return -1;
    }
    if (end + 1) >= date.len() || (date[end + 1] != b'+' && date[end + 1] != b'-') {
        return -1;
    }
    let new_start = end + 2;
    let (ofs_val, ofs_consumed) = parse_long(&date[new_start..]);
    let ofs_end = new_start + ofs_consumed;
    let trailing_ok = if ofs_end >= date.len() {
        true
    } else {
        date[ofs_end] == 0 || date[ofs_end] == b'\n'
    };
    if !trailing_ok || ofs_consumed != 4 {
        return -1;
    }
    let mut ofs = ((ofs_val / 100) * 60 + (ofs_val % 100)) as i32;
    if date[new_start - 1] == b'-' {
        ofs = -ofs;
    }
    tv.tv_sec = stamp as i64;
    *offset = ofs;
    0
}

pub fn match_object_header_date(date: &str, tv: &mut TimeVal, offset: &mut i32) -> i32 {
    match_object_header_date_b(date.as_bytes(), tv, offset)
}

fn parse_date_basic_b(date: &[u8], tv: &mut TimeVal, offset: &mut i32) -> i32 {
    let mut tm = Atm {
        tm_sec: -1,
        tm_min: -1,
        tm_hour: -1,
        tm_mday: -1,
        tm_mon: -1,
        tm_year: -1,
        tm_wday: 0,
        tm_yday: 0,
        tm_isdst: -1,
        tm_usec: 0,
    };
    let mut tm_gmt: i32 = 0;
    *offset = -1;

    if !date.is_empty() && date[0] == b'@' {
        if match_object_header_date_b(&date[1..], tv, offset) == 0 {
            return 0;
        }
    }

    let mut i: usize = 0;
    while i < date.len() {
        let c = date[i];
        if c == 0 || c == b'\n' {
            break;
        }
        let mut m: i32 = 0;
        if is_alpha_byte(c) {
            m = match_alpha_b(&date[i..], &mut tm, offset);
        } else if is_digit_byte(c) {
            m = match_digit_b(&date[i..], &mut tm, offset, &mut tm_gmt);
        } else if (c == b'-' || c == b'+')
            && (i + 1) < date.len()
            && is_digit_byte(date[i + 1])
        {
            m = match_tz_b(&date[i..], offset);
        }
        if m == 0 {
            m = 1;
        }
        i += m as usize;
    }

    tv.tv_usec = tm.tm_usec;

    let computed = tm_to_time_t_or_neg(&tm);
    tv.tv_sec = computed;

    if *offset == -1 {
        let temp_time = mktime_local(&tm);
        if tv.tv_sec > temp_time {
            *offset = ((tv.tv_sec - temp_time) / 60) as i32;
        } else {
            *offset = -(((temp_time - tv.tv_sec) / 60) as i32);
        }
    }

    if *offset == -1 {
        *offset = ((tv.tv_sec - mktime_local(&tm)) / 60) as i32;
    }

    if tv.tv_sec == -1 {
        return -1;
    }

    if tm_gmt == 0 {
        tv.tv_sec -= (*offset as i64) * 60;
    }

    0
}

pub fn parse_date_basic(date: &str, tv: &mut TimeVal, offset: &mut i32) -> i32 {
    parse_date_basic_b(date.as_bytes(), tv, offset)
}

fn update_tm_internal(tm: &mut Atm, now: &Atm, sec: u64) -> i64 {
    if tm.tm_mday < 0 {
        tm.tm_mday = now.tm_mday;
    }
    if tm.tm_mon < 0 {
        tm.tm_mon = now.tm_mon;
    }
    if tm.tm_year < 0 {
        tm.tm_year = now.tm_year;
        if tm.tm_mon > now.tm_mon {
            tm.tm_year -= 1;
        }
    }
    let n = mktime_local(tm) - sec as i64;
    localtime_r(n, tm);
    n
}

pub fn update_tm(tm: &mut Atm, now: &mut Atm, sec: u64) -> u64 {
    update_tm_internal(tm, now, sec) as u64
}

pub fn date_now(tm: &mut Atm, now: &mut Atm, _num: &mut i32) {
    update_tm_internal(tm, now, 0);
}

pub fn date_yesterday(tm: &mut Atm, now: &mut Atm, _num: &mut i32) {
    update_tm_internal(tm, now, 24 * 60 * 60);
}

pub fn date_time(tm: &mut Atm, now: &mut Atm, hour: i32) {
    if tm.tm_hour < hour {
        let mut dummy: i32 = 0;
        date_yesterday(tm, now, &mut dummy);
    }
    tm.tm_hour = hour;
    tm.tm_min = 0;
    tm.tm_sec = 0;
}

pub fn date_midnight(tm: &mut Atm, now: &mut Atm, _num: &mut i32) {
    date_time(tm, now, 0);
}

pub fn date_noon(tm: &mut Atm, now: &mut Atm, _num: &mut i32) {
    date_time(tm, now, 12);
}

pub fn date_tea(tm: &mut Atm, now: &mut Atm, _num: &mut i32) {
    date_time(tm, now, 17);
}

pub fn date_pm(tm: &mut Atm, _now: &mut Atm, num: &mut i32) {
    let n = *num;
    *num = 0;
    let mut hour = tm.tm_hour;
    if n != 0 {
        hour = n;
        tm.tm_min = 0;
        tm.tm_sec = 0;
    }
    tm.tm_hour = (hour % 12) + 12;
}

pub fn date_am(tm: &mut Atm, _now: &mut Atm, num: &mut i32) {
    let n = *num;
    *num = 0;
    let mut hour = tm.tm_hour;
    if n != 0 {
        hour = n;
        tm.tm_min = 0;
        tm.tm_sec = 0;
    }
    tm.tm_hour = hour % 12;
}

pub fn date_never(tm: &mut Atm, _now: &mut Atm, _num: &mut i32) {
    let n: i64 = 0;
    localtime_r(n, tm);
}

pub struct Special {
    pub name: &'static str,
    pub fn_ptr: fn(&mut Atm, &mut Atm, &mut i32),
}
pub static SPECIAL: &[Special] = &[
    Special { name: "yesterday", fn_ptr: date_yesterday },
    Special { name: "noon", fn_ptr: date_noon },
    Special { name: "midnight", fn_ptr: date_midnight },
    Special { name: "tea", fn_ptr: date_tea },
    Special { name: "PM", fn_ptr: date_pm },
    Special { name: "AM", fn_ptr: date_am },
    Special { name: "never", fn_ptr: date_never },
    Special { name: "now", fn_ptr: date_now },
];

pub const NUMBER_NAME: [&str; 11] = [
    "zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten",
];

pub struct TypeLen {
    pub type_name: &'static str,
    pub length: i32,
}
pub const TYPELEN: &[TypeLen] = &[
    TypeLen { type_name: "seconds", length: 1 },
    TypeLen { type_name: "minutes", length: 60 },
    TypeLen { type_name: "hours", length: 60 * 60 },
    TypeLen { type_name: "days", length: 24 * 60 * 60 },
    TypeLen { type_name: "weeks", length: 7 * 24 * 60 * 60 },
];

// Returns (consumed_bytes_from_date, touched_set)
fn approxidate_alpha_b(
    date: &[u8],
    tm: &mut Atm,
    now: &mut Atm,
    num: &mut i32,
    touched: &mut bool,
) -> usize {
    // Find end of alpha sequence: while isalpha(*++end)
    let mut end = 1usize;
    while end < date.len() && is_alpha_byte(date[end]) {
        end += 1;
    }

    for i in 0..12 {
        let m = match_string_b(date, MONTH_NAMES[i].as_bytes());
        if m >= 3 {
            tm.tm_mon = i as i32;
            *touched = true;
            return end;
        }
    }

    for s in SPECIAL.iter() {
        let len = s.name.len() as i32;
        if match_string_b(date, s.name.as_bytes()) == len {
            (s.fn_ptr)(tm, now, num);
            *touched = true;
            return end;
        }
    }

    if *num == 0 {
        for i in 1..11 {
            let len = NUMBER_NAME[i].len() as i32;
            if match_string_b(date, NUMBER_NAME[i].as_bytes()) == len {
                *num = i as i32;
                *touched = true;
                return end;
            }
        }
        if match_string_b(date, b"last") == 4 {
            *num = 1;
            *touched = true;
        }
        return end;
    }

    for tl in TYPELEN.iter() {
        let len = tl.type_name.len() as i32;
        if match_string_b(date, tl.type_name.as_bytes()) >= len - 1 {
            update_tm_internal(tm, now, (tl.length * *num) as u64);
            *num = 0;
            *touched = true;
            return end;
        }
    }

    for i in 0..7 {
        let m = match_string_b(date, WEEKDAY_NAMES[i].as_bytes());
        if m >= 3 {
            let mut n = *num - 1;
            *num = 0;
            let mut diff = tm.tm_wday - i as i32;
            if diff <= 0 {
                n += 1;
            }
            diff += 7 * n;
            update_tm_internal(tm, now, (diff * 24 * 60 * 60) as u64);
            *touched = true;
            return end;
        }
    }

    if match_string_b(date, b"months") >= 5 {
        update_tm_internal(tm, now, 0);
        let mut n = tm.tm_mon - *num;
        *num = 0;
        while n < 0 {
            n += 12;
            tm.tm_year -= 1;
        }
        tm.tm_mon = n;
        *touched = true;
        return end;
    }

    if match_string_b(date, b"years") >= 4 {
        update_tm_internal(tm, now, 0);
        tm.tm_year -= *num;
        *num = 0;
        *touched = true;
        return end;
    }

    end
}

pub fn approxidate_alpha(
    date: &str,
    tm: &mut Atm,
    now: &mut Atm,
    num: &i32,
    touched: &i32,
) -> String {
    // Public-API wrapper — input refs are immutable so we mirror the work locally.
    let mut n_local = *num;
    let mut t_bool = *touched != 0;
    let consumed = approxidate_alpha_b(date.as_bytes(), tm, now, &mut n_local, &mut t_bool);
    String::from_utf8_lossy(&date.as_bytes()[consumed..]).to_string()
}

// Returns consumed bytes from date.
fn approxidate_digit_b(date: &[u8], tm: &mut Atm, num: &mut i32, now: TimeT) -> usize {
    let (number, consumed) = parse_ulong(date);
    let end = consumed;
    if end < date.len() {
        let c = date[end];
        if c == b':' || c == b'.' || c == b'/' || c == b'-' {
            if (end + 1) < date.len() && is_digit_byte(date[end + 1]) {
                let m = match_multi_number_b(number, c, date, end, tm, now);
                if m != 0 {
                    return m as usize;
                }
            }
        }
    }
    // Accept zero-padding only for small numbers
    if !date.is_empty() && (date[0] != b'0' || end <= 2) {
        *num = number as i32;
    }
    end
}

pub fn approxidate_digit(date: &str, tm: &mut Atm, num: &i32, now: TimeT) -> String {
    let mut n_local = *num;
    let consumed = approxidate_digit_b(date.as_bytes(), tm, &mut n_local, now);
    String::from_utf8_lossy(&date.as_bytes()[consumed..]).to_string()
}

fn pending_number_b(tm: &mut Atm, num: &mut i32) {
    let number = *num;
    if number != 0 {
        *num = 0;
        if tm.tm_mday < 0 && number < 32 {
            tm.tm_mday = number;
        } else if tm.tm_mon < 0 && number < 13 {
            tm.tm_mon = number - 1;
        } else if tm.tm_year < 0 {
            if number > 1969 && number < 2100 {
                tm.tm_year = number - 1900;
            } else if number > 69 && number < 100 {
                tm.tm_year = number;
            } else if number < 38 {
                tm.tm_year = 100 + number;
            }
        }
    }
}

pub fn pending_number(tm: &mut Atm, num: &i32) {
    let mut n_local = *num;
    pending_number_b(tm, &mut n_local);
}

fn approxidate_str_b(date: &[u8], tv: &mut TimeVal) -> i32 {
    let mut number: i32 = 0;
    let mut touched: bool = false;
    let mut tm = new_atm();
    let time_sec = tv.tv_sec;
    localtime_r(time_sec, &mut tm);
    let mut now = copy_atm(&tm);

    tm.tm_year = -1;
    tm.tm_mon = -1;
    tm.tm_mday = -1;
    tm.tm_usec = tv.tv_usec;

    let mut i: usize = 0;
    while i < date.len() {
        let c = date[i];
        if c == 0 {
            break;
        }
        i += 1;
        if is_digit_byte(c) {
            pending_number_b(&mut tm, &mut number);
            let consumed = approxidate_digit_b(&date[i - 1..], &mut tm, &mut number, time_sec);
            i = (i - 1) + consumed;
            touched = true;
            continue;
        }
        if is_alpha_byte(c) {
            let consumed = approxidate_alpha_b(
                &date[i - 1..],
                &mut tm,
                &mut now,
                &mut number,
                &mut touched,
            );
            i = (i - 1) + consumed;
        }
    }

    pending_number_b(&mut tm, &mut number);
    if !touched {
        return -1;
    }

    tv.tv_usec = tm.tm_usec;
    tv.tv_sec = update_tm_internal(&mut tm, &now, 0);
    0
}

pub fn approxidate_str(date: &str, tv: &mut TimeVal) -> i32 {
    approxidate_str_b(date.as_bytes(), tv)
}

pub fn approxidate_main(date: &str, tv: &mut TimeVal) -> i32 {
    let mut offset: i32 = 0;
    if parse_date_basic_b(date.as_bytes(), tv, &mut offset) == 0 {
        return 0;
    }
    // gettimeofday
    let now = chrono::Utc::now();
    tv.tv_sec = now.timestamp();
    tv.tv_usec = now.timestamp_subsec_micros() as i64;

    if approxidate_str_b(date.as_bytes(), tv) == 0 {
        return 0;
    }
    -1
}

pub fn approxidate_relative(date: &str, tv: &mut TimeVal, relative_to: &mut TimeVal) -> i32 {
    let mut offset: i32 = 0;
    if parse_date_basic_b(date.as_bytes(), tv, &mut offset) == 0 {
        return 0;
    }
    *tv = *relative_to;
    if approxidate_str_b(date.as_bytes(), tv) == 0 {
        return 0;
    }
    -1
}

// Suppress unused-warnings for ctype constants; they're part of the public spec.
#[allow(dead_code)]
const _UNUSED_CTYPE: [u8; 8] = [
    GIT_SPACE,
    GIT_DIGIT,
    GIT_ALPHA,
    GIT_GLOB_SPECIAL,
    GIT_REGEX_SPECIAL,
    GIT_PATHSPEC_MAGIC,
    GIT_CNTRL,
    GIT_PUNCT,
];
