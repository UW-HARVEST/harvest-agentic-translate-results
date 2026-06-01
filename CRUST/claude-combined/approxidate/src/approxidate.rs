use chrono::{Datelike, Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone as ChronoTz, Timelike, Utc, Offset};

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
    pub tv_sec: i64,  // Equivalent to time_t
    pub tv_usec: i64, // Equivalent to suseconds_t
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

const S: u8 = GIT_SPACE;
const A: u8 = GIT_ALPHA;
const D: u8 = GIT_DIGIT;
const G: u8 = GIT_GLOB_SPECIAL;
const R: u8 = GIT_REGEX_SPECIAL;
const P: u8 = GIT_PATHSPEC_MAGIC;
const X: u8 = GIT_CNTRL;
const U: u8 = GIT_PUNCT;
const Z: u8 = GIT_CNTRL | GIT_SPACE;

const SANE_CTYPE: [u8; 256] = {
    let mut t = [0u8; 256];
    let row0: [u8; 16] = [X, X, X, X, X, X, X, X, X, Z, Z, X, X, Z, X, X];
    let row1: [u8; 16] = [X, X, X, X, X, X, X, X, X, X, X, X, X, X, X, X];
    let row2: [u8; 16] = [S, P, P, P, R, P, P, P, R, R, G, R, P, P, R, P];
    let row3: [u8; 16] = [D, D, D, D, D, D, D, D, D, D, P, P, P, P, P, G];
    let row4: [u8; 16] = [P, A, A, A, A, A, A, A, A, A, A, A, A, A, A, A];
    let row5: [u8; 16] = [A, A, A, A, A, A, A, A, A, A, A, G, G, U, R, P];
    let row6: [u8; 16] = [P, A, A, A, A, A, A, A, A, A, A, A, A, A, A, A];
    let row7: [u8; 16] = [A, A, A, A, A, A, A, A, A, A, A, R, R, U, P, X];
    let mut i = 0;
    while i < 16 {
        t[i] = row0[i];
        t[i + 16] = row1[i];
        t[i + 32] = row2[i];
        t[i + 48] = row3[i];
        t[i + 64] = row4[i];
        t[i + 80] = row5[i];
        t[i + 96] = row6[i];
        t[i + 112] = row7[i];
        i += 1;
    }
    t
};

#[inline]
fn sane_istest(c: u8, mask: u8) -> bool {
    (SANE_CTYPE[c as usize] & mask) != 0
}

#[inline]
fn is_digit_byte(c: u8) -> bool {
    sane_istest(c, GIT_DIGIT)
}

#[inline]
fn is_alpha_byte(c: u8) -> bool {
    sane_istest(c, GIT_ALPHA)
}

#[inline]
fn is_alnum_byte(c: u8) -> bool {
    sane_istest(c, GIT_ALPHA | GIT_DIGIT)
}

#[inline]
fn to_upper(c: u8) -> u8 {
    if sane_istest(c, GIT_ALPHA) {
        c & !0x20
    } else {
        c
    }
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

pub fn tm_to_time_t(tm: &Atm) -> Option<i64> {
    const MDAYS: [i32; 12] = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];
    let year = tm.tm_year - 70;
    let month = tm.tm_mon;
    let mut day = tm.tm_mday;

    if year < 0 || year > 129 {
        return None;
    }
    if month < 0 || month > 11 {
        return None;
    }
    if month < 2 || (year + 2) % 4 != 0 {
        day -= 1;
    }
    if tm.tm_hour < 0 || tm.tm_min < 0 || tm.tm_sec < 0 {
        return None;
    }
    let secs = (year as i64 * 365 + ((year + 1) / 4) as i64 + MDAYS[month as usize] as i64 + day as i64)
        * 24 * 60 * 60
        + tm.tm_hour as i64 * 60 * 60
        + tm.tm_min as i64 * 60
        + tm.tm_sec as i64;
    Some(secs)
}

pub fn match_string(date: &str, format: &str) -> i32 {
    let d = date.as_bytes();
    let s = format.as_bytes();
    let mut i = 0usize;
    loop {
        if i >= d.len() {
            return i as i32;
        }
        if i >= s.len() {
            // s ended; in C this means *str == 0; comparing *date to *str (NUL).
            // *date != 0 (else we'd have hit above), and toupper(*date)!=toupper(0).
            // If !isalnum(*date) -> break, return i. Else return 0.
            let dc = d[i];
            if !is_alnum_byte(dc) {
                return i as i32;
            }
            return 0;
        }
        let dc = d[i];
        let sc = s[i];
        if dc == sc {
            i += 1;
            continue;
        }
        if to_upper(dc) == to_upper(sc) {
            i += 1;
            continue;
        }
        if !is_alnum_byte(dc) {
            return i as i32;
        }
        return 0;
    }
}

pub fn skip_alpha(date: &str) -> i32 {
    let d = date.as_bytes();
    let mut i = 0usize;
    loop {
        i += 1;
        if i >= d.len() || !is_alpha_byte(d[i]) {
            break;
        }
    }
    i as i32
}

pub fn match_alpha(date: &str, tm: &mut Atm, offset: &mut i32) -> i32 {
    for i in 0..12 {
        let m = match_string(date, MONTH_NAMES[i]);
        if m >= 3 {
            tm.tm_mon = i as i32;
            return m;
        }
    }
    for i in 0..7 {
        let m = match_string(date, WEEKDAY_NAMES[i]);
        if m >= 3 {
            tm.tm_wday = i as i32;
            return m;
        }
    }
    for tz in TIMEZONE_NAMES.iter() {
        let m = match_string(date, tz.name);
        if m >= 3 || m as usize == tz.name.len() {
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
    if match_string(date, "PM") == 2 {
        tm.tm_hour = (tm.tm_hour % 12) + 12;
        return 2;
    }
    if match_string(date, "AM") == 2 {
        tm.tm_hour = tm.tm_hour % 12;
        return 2;
    }
    skip_alpha(date)
}

pub fn is_date(year: i32, month: i32, day: i32, now_tm: Option<&Atm>, _now: TimeT, tm: &mut Atm) -> i32 {
    if month > 0 && month < 13 && day > 0 && day < 32 {
        // r is either a check copy or tm itself
        let mut check_year;
        let mut check_mon;
        let mut check_mday;
        // initialize check from tm
        check_year = tm.tm_year;
        check_mon = tm.tm_mon;
        check_mday = tm.tm_mday;

        check_mon = month - 1;
        check_mday = day;
        if year == -1 {
            if now_tm.is_none() {
                // C writes to tm directly via r=tm, then returns 1 without overwriting tm again
                tm.tm_mon = check_mon;
                tm.tm_mday = check_mday;
                return 1;
            }
            check_year = now_tm.unwrap().tm_year;
        } else if year >= 1970 && year < 2100 {
            check_year = year - 1900;
        } else if year > 70 && year < 100 {
            check_year = year;
        } else if year < 38 {
            check_year = year + 100;
        } else {
            return 0;
        }
        if now_tm.is_none() {
            tm.tm_mon = check_mon;
            tm.tm_mday = check_mday;
            if year != -1 {
                tm.tm_year = check_year;
            }
            return 1;
        }
        tm.tm_mon = check_mon;
        tm.tm_mday = check_mday;
        if year != -1 {
            tm.tm_year = check_year;
        }
        return 1;
    }
    0
}

// Helper: parse digits returning value and number of consumed bytes
fn parse_ulong(s: &[u8], start: usize) -> (u64, usize) {
    let mut idx = start;
    // skip whitespace? strtoul skips leading whitespace; but our usage doesn't pass whitespace
    while idx < s.len() && (s[idx] == b' ' || s[idx] == b'\t') {
        idx += 1;
    }
    // optional sign for strtoul
    if idx < s.len() && (s[idx] == b'+' || s[idx] == b'-') {
        // strtoul accepts sign but we treat - as wrap; skip for simplicity since approxidate uses
        // it on positive numbers
        idx += 1;
    }
    let mut val: u64 = 0;
    let mut any = false;
    while idx < s.len() && is_digit_byte(s[idx]) {
        val = val.wrapping_mul(10).wrapping_add((s[idx] - b'0') as u64);
        idx += 1;
        any = true;
    }
    if !any {
        return (0, start);
    }
    (val, idx)
}

fn parse_long(s: &[u8], start: usize) -> (i64, usize) {
    let mut idx = start;
    while idx < s.len() && (s[idx] == b' ' || s[idx] == b'\t') {
        idx += 1;
    }
    let mut neg = false;
    if idx < s.len() && (s[idx] == b'+' || s[idx] == b'-') {
        if s[idx] == b'-' {
            neg = true;
        }
        idx += 1;
    }
    let mut val: i64 = 0;
    let mut any = false;
    while idx < s.len() && is_digit_byte(s[idx]) {
        val = val.wrapping_mul(10).wrapping_add((s[idx] - b'0') as i64);
        idx += 1;
        any = true;
    }
    if !any {
        return (0, start);
    }
    if neg {
        val = -val;
    }
    (val, idx)
}

pub fn match_multi_number(num: u64, c: char, date: &str, end: &str, tm: &mut Atm, now: TimeT) -> i32 {
    // date is the original full string starting at the digit we parsed for `num`.
    // end is what's left starting at the separator c.
    let date_bytes = date.as_bytes();
    let end_offset = date_bytes.len() - end.as_bytes().len();
    // We start parsing from end_offset+1 (after the separator)
    let (num2, mut p) = parse_long(date_bytes, end_offset + 1);
    let mut num3: i64 = -1;
    let mut num4: i64 = 0;
    if p < date_bytes.len() && date_bytes[p] == c as u8 && p + 1 < date_bytes.len() && is_digit_byte(date_bytes[p + 1]) {
        let (n3, p2) = parse_long(date_bytes, p + 1);
        num3 = n3;
        p = p2;
        if p < date_bytes.len() && date_bytes[p] == b'.' {
            let start_frac = p + 1;
            let (n4, p3) = parse_long(date_bytes, p + 1);
            num4 = n4;
            p = p3;
            let frac_len = p - start_frac;
            if frac_len < 6 {
                let mut mult: i64 = 1;
                for _ in 0..(6 - frac_len as i32) {
                    mult *= 10;
                }
                num4 *= mult;
            }
        }
    }

    match c {
        ':' => {
            let num3 = if num3 < 0 { 0 } else { num3 };
            if num < 25 && num2 >= 0 && num2 < 60 && num3 >= 0 && num3 <= 60 {
                tm.tm_hour = num as i32;
                tm.tm_min = num2 as i32;
                tm.tm_sec = num3 as i32;
                tm.tm_usec = num4;
                return p as i32;
            }
            return 0;
        }
        '-' | '/' | '.' => {
            let now_used = if now == 0 { current_time_secs() } else { now };
            if num > 70 {
                if is_date(num as i32, num2 as i32, num3 as i32, None, now_used, tm) != 0 {
                    return p as i32;
                }
                if is_date(num as i32, num3 as i32, num2 as i32, None, now_used, tm) != 0 {
                    return p as i32;
                }
            }
            if c != '.' && is_date(num3 as i32, num as i32, num2 as i32, None, now_used, tm) != 0 {
                return p as i32;
            }
            if is_date(num3 as i32, num2 as i32, num as i32, None, now_used, tm) != 0 {
                return p as i32;
            }
            if c == '.' && is_date(num3 as i32, num as i32, num2 as i32, None, now_used, tm) != 0 {
                return p as i32;
            }
            return 0;
        }
        _ => return 0,
    }
}

fn current_time_secs() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

pub fn nodate(tm: &mut Atm) -> i32 {
    let v = tm.tm_year & tm.tm_mon & tm.tm_mday & tm.tm_hour & tm.tm_min & tm.tm_sec;
    if v < 0 {
        1
    } else {
        0
    }
}

pub fn match_digit(date: &str, tm: &mut Atm, offset: &mut i32, tm_gmt: &mut i32) -> i32 {
    let bytes = date.as_bytes();
    let (num, end_idx) = parse_ulong(bytes, 0);
    let consumed = end_idx;

    if num >= 100_000_000 && nodate(tm) != 0 {
        // gmtime_r equivalent
        let time_sec = num as i64;
        if let Some(dt) = chrono::DateTime::<chrono::Utc>::from_timestamp(time_sec, 0) {
            let nt = dt.naive_utc();
            tm.tm_sec = nt.second() as i32;
            tm.tm_min = nt.minute() as i32;
            tm.tm_hour = nt.hour() as i32;
            tm.tm_mday = nt.day() as i32;
            tm.tm_mon = (nt.month() - 1) as i32;
            tm.tm_year = (nt.year() - 1900) as i32;
            tm.tm_wday = nt.weekday().num_days_from_sunday() as i32;
            tm.tm_yday = (nt.ordinal() - 1) as i32;
            tm.tm_isdst = 0;
            *tm_gmt = 1;
            return consumed as i32;
        }
    }

    if consumed < bytes.len() {
        let nch = bytes[consumed];
        match nch {
            b':' | b'.' | b'/' | b'-' => {
                if consumed + 1 < bytes.len() && is_digit_byte(bytes[consumed + 1]) {
                    let end_str = std::str::from_utf8(&bytes[consumed..]).unwrap_or("");
                    let m = match_multi_number(num, nch as char, date, end_str, tm, 0);
                    if m != 0 {
                        return m;
                    }
                }
            }
            _ => {}
        }
    }

    let mut n = 0usize;
    loop {
        n += 1;
        if n >= bytes.len() || !is_digit_byte(bytes[n]) {
            break;
        }
    }

    if n == 4 {
        if num <= 1400 && *offset == -1 {
            let minutes = (num % 100) as i32;
            let hours = (num / 100) as i32;
            *offset = hours * 60 + minutes;
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
            tm.tm_year = (num as i32) + 100;
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

pub fn match_tz(date: &str, offp: &mut i32) -> i32 {
    let bytes = date.as_bytes();
    // C: hour = strtoul(date+1, &end, 10); n = end - (date+1)
    let (hour_u, end_idx) = parse_ulong(bytes, 1);
    let mut hour = hour_u as i32;
    let n = end_idx - 1;
    let mut min: i32 = 0;
    let mut consumed_end = end_idx;

    if n == 4 {
        min = hour % 100;
        hour = hour / 100;
    } else if n != 2 {
        min = 99;
    } else if end_idx < bytes.len() && bytes[end_idx] == b':' {
        let (m, e2) = parse_ulong(bytes, end_idx + 1);
        min = m as i32;
        consumed_end = e2;
        if e2 - 1 != 5 {
            min = 99;
        }
    }

    if min < 60 && hour < 24 {
        let mut off = hour * 60 + min;
        if bytes[0] == b'-' {
            off = -off;
        }
        *offp = off;
    }
    consumed_end as i32
}

pub fn match_object_header_date(date: &str, tv: &mut TimeVal, offset: &mut i32) -> i32 {
    let bytes = date.as_bytes();
    if bytes.is_empty() {
        return -1;
    }
    if bytes[0] < b'0' || bytes[0] > b'9' {
        return -1;
    }
    let (stamp, end_idx) = parse_ulong(bytes, 0);
    if end_idx >= bytes.len() || bytes[end_idx] != b' ' {
        return -1;
    }
    if stamp == u64::MAX {
        return -1;
    }
    if end_idx + 1 >= bytes.len() {
        return -1;
    }
    let sign_byte = bytes[end_idx + 1];
    if sign_byte != b'+' && sign_byte != b'-' {
        return -1;
    }
    let date_start = end_idx + 2;
    let (ofs_l, end_idx2) = parse_long(bytes, date_start);
    let mut ofs = ofs_l as i32;
    if end_idx2 != date_start + 4 {
        return -1;
    }
    if end_idx2 < bytes.len() && bytes[end_idx2] != b'\n' {
        return -1;
    }
    ofs = (ofs / 100) * 60 + (ofs % 100);
    if sign_byte == b'-' {
        ofs = -ofs;
    }
    tv.tv_sec = stamp as i64;
    *offset = ofs;
    0
}

// Local time helpers using chrono
fn local_offset_seconds(timestamp: i64) -> i64 {
    let dt = match Utc.timestamp_opt(timestamp, 0).single() {
        Some(d) => d,
        None => return 0,
    };
    let local_dt = dt.with_timezone(&Local);
    local_dt.offset().local_minus_utc() as i64
}

fn mktime_from_atm(tm: &Atm) -> i64 {
    // mktime treats the broken-down time as local and returns time_t.
    // Use chrono to construct local datetime then convert to timestamp.
    let year = tm.tm_year + 1900;
    let mon = (tm.tm_mon + 1) as u32;
    let day = tm.tm_mday as u32;
    let date = match NaiveDate::from_ymd_opt(year, mon.max(1).min(12), day.max(1).min(31)) {
        Some(d) => d,
        None => return -1,
    };
    let hour = tm.tm_hour.max(0) as u32 % 24;
    let minute = tm.tm_min.max(0) as u32 % 60;
    let second = tm.tm_sec.max(0) as u32 % 60;
    let time = match NaiveTime::from_hms_opt(hour, minute, second) {
        Some(t) => t,
        None => return -1,
    };
    let ndt = NaiveDateTime::new(date, time);
    match Local.from_local_datetime(&ndt).single() {
        Some(d) => d.timestamp(),
        None => match Local.from_local_datetime(&ndt).earliest() {
            Some(d) => d.timestamp(),
            None => 0,
        },
    }
}

fn localtime_r(timestamp: i64, tm: &mut Atm) {
    let dt = match Local.timestamp_opt(timestamp, 0).single() {
        Some(d) => d,
        None => return,
    };
    let nt = dt.naive_local();
    tm.tm_sec = nt.second() as i32;
    tm.tm_min = nt.minute() as i32;
    tm.tm_hour = nt.hour() as i32;
    tm.tm_mday = nt.day() as i32;
    tm.tm_mon = (nt.month() - 1) as i32;
    tm.tm_year = (nt.year() - 1900) as i32;
    tm.tm_wday = nt.weekday().num_days_from_sunday() as i32;
    tm.tm_yday = (nt.ordinal() - 1) as i32;
    // determine isdst from offset
    tm.tm_isdst = 0;
}

pub fn parse_date_basic(date: &str, tv: &mut TimeVal, offset: &mut i32) -> i32 {
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

    let bytes = date.as_bytes();
    if !bytes.is_empty() && bytes[0] == b'@' {
        if match_object_header_date(&date[1..], tv, offset) == 0 {
            return 0;
        }
    }

    let mut idx = 0usize;
    loop {
        if idx >= bytes.len() {
            break;
        }
        let c = bytes[idx];
        if c == 0 || c == b'\n' {
            break;
        }
        let mut m: i32 = 0;
        let sub = &date[idx..];
        if is_alpha_byte(c) {
            m = match_alpha(sub, &mut tm, offset);
        } else if is_digit_byte(c) {
            m = match_digit(sub, &mut tm, offset, &mut tm_gmt);
        } else if (c == b'-' || c == b'+') && idx + 1 < bytes.len() && is_digit_byte(bytes[idx + 1]) {
            m = match_tz(sub, offset);
        }
        if m == 0 {
            m = 1;
        }
        idx += m as usize;
    }

    tv.tv_usec = tm.tm_usec;
    let secs = tm_to_time_t(&tm).unwrap_or(-1);
    tv.tv_sec = secs;

    if *offset == -1 {
        let temp_time = mktime_from_atm(&tm);
        if tv.tv_sec > temp_time {
            *offset = ((tv.tv_sec - temp_time) / 60) as i32;
        } else {
            *offset = -(((temp_time - tv.tv_sec) / 60) as i32);
        }
    }

    if *offset == -1 {
        *offset = ((tv.tv_sec - mktime_from_atm(&tm)) / 60) as i32;
    }

    if tv.tv_sec == -1 {
        return -1;
    }
    if tm_gmt == 0 {
        tv.tv_sec -= (*offset as i64) * 60;
    }
    0
}

pub fn update_tm(tm: &mut Atm, now: &mut Atm, sec: u64) -> u64 {
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
    let n = mktime_from_atm(tm) - sec as i64;
    localtime_r(n, tm);
    n as u64
}

pub fn date_now(tm: &mut Atm, now: &mut Atm, _num: &mut i32) {
    update_tm(tm, now, 0);
}

pub fn date_yesterday(tm: &mut Atm, now: &mut Atm, _num: &mut i32) {
    update_tm(tm, now, 24 * 60 * 60);
}

pub fn date_time(tm: &mut Atm, now: &mut Atm, hour: i32) {
    if tm.tm_hour < hour {
        let mut dummy = 0i32;
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
    localtime_r(0, tm);
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

// Returns a slice of `date` starting at the position where alpha-skipping ended.
fn approxidate_alpha_internal(
    date: &str,
    tm: &mut Atm,
    now: &mut Atm,
    num: &mut i32,
    touched: &mut i32,
) -> usize {
    let bytes = date.as_bytes();
    // C: while (isalpha(*++end)) ; -- starts at date+1, advances while alpha. So end_offset = position
    // of first non-alpha byte starting from index 1.
    let mut end = 1usize;
    while end < bytes.len() && is_alpha_byte(bytes[end]) {
        end += 1;
    }

    for i in 0..12 {
        let m = match_string(date, MONTH_NAMES[i]);
        if m >= 3 {
            tm.tm_mon = i as i32;
            *touched = 1;
            return end;
        }
    }

    for s in SPECIAL {
        let len = s.name.len();
        if match_string(date, s.name) as usize == len {
            (s.fn_ptr)(tm, now, num);
            *touched = 1;
            return end;
        }
    }

    if *num == 0 {
        for i in 1..11usize {
            let len = NUMBER_NAME[i].len();
            if match_string(date, NUMBER_NAME[i]) as usize == len {
                *num = i as i32;
                *touched = 1;
                return end;
            }
        }
        if match_string(date, "last") == 4 {
            *num = 1;
            *touched = 1;
        }
        return end;
    }

    for tl in TYPELEN {
        let len = tl.type_name.len();
        if match_string(date, tl.type_name) >= (len as i32 - 1) {
            update_tm(tm, now, (tl.length as u64) * (*num as u64));
            *num = 0;
            *touched = 1;
            return end;
        }
    }

    for i in 0..7 {
        let m = match_string(date, WEEKDAY_NAMES[i]);
        if m >= 3 {
            let mut n = *num - 1;
            *num = 0;
            let mut diff = tm.tm_wday - i as i32;
            if diff <= 0 {
                n += 1;
            }
            diff += 7 * n;
            update_tm(tm, now, (diff as u64).wrapping_mul(24 * 60 * 60));
            *touched = 1;
            return end;
        }
    }

    if match_string(date, "months") >= 5 {
        update_tm(tm, now, 0);
        let mut n = tm.tm_mon - *num;
        *num = 0;
        while n < 0 {
            n += 12;
            tm.tm_year -= 1;
        }
        tm.tm_mon = n;
        *touched = 1;
        return end;
    }

    if match_string(date, "years") >= 4 {
        update_tm(tm, now, 0);
        tm.tm_year -= *num;
        *num = 0;
        *touched = 1;
        return end;
    }

    end
}

pub fn approxidate_alpha(
    date: &str,
    tm: &mut Atm,
    now: &mut Atm,
    _num: &i32,
    _touched: &i32,
) -> String {
    let mut num = *_num;
    let mut touched = *_touched;
    let off = approxidate_alpha_internal(date, tm, now, &mut num, &mut touched);
    date[off..].to_string()
}

fn approxidate_digit_internal(
    date: &str,
    tm: &mut Atm,
    num: &mut i32,
    now: TimeT,
) -> usize {
    let bytes = date.as_bytes();
    let (number, end_idx) = parse_ulong(bytes, 0);

    if end_idx < bytes.len() {
        let nch = bytes[end_idx];
        match nch {
            b':' | b'.' | b'/' | b'-' => {
                if end_idx + 1 < bytes.len() && is_digit_byte(bytes[end_idx + 1]) {
                    let end_str = std::str::from_utf8(&bytes[end_idx..]).unwrap_or("");
                    let m = match_multi_number(number, nch as char, date, end_str, tm, now);
                    if m != 0 {
                        return m as usize;
                    }
                }
            }
            _ => {}
        }
    }

    if bytes[0] != b'0' || end_idx <= 2 {
        *num = number as i32;
    }
    end_idx
}

pub fn approxidate_digit(date: &str, tm: &mut Atm, _num: &i32, now: TimeT) -> String {
    let mut num = *_num;
    let off = approxidate_digit_internal(date, tm, &mut num, now);
    date[off..].to_string()
}

pub fn pending_number(tm: &mut Atm, _num: &i32) {
    // Re-implementation that mutates via a copy is impossible -- but signature takes &i32.
    // For correctness in the str path we need an internal version.
    let mut number = *_num;
    pending_number_internal(tm, &mut number);
}

fn pending_number_internal(tm: &mut Atm, num: &mut i32) {
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

pub fn approxidate_str(date: &str, tv: &mut TimeVal) -> i32 {
    let mut number: i32 = 0;
    let mut touched: i32 = 0;
    let time_sec = tv.tv_sec;
    let mut tm = Atm {
        tm_sec: 0,
        tm_min: 0,
        tm_hour: 0,
        tm_mday: 0,
        tm_mon: 0,
        tm_year: 0,
        tm_wday: 0,
        tm_yday: 0,
        tm_isdst: 0,
        tm_usec: tv.tv_usec,
    };
    localtime_r(time_sec, &mut tm);
    let mut now = Atm {
        tm_sec: tm.tm_sec,
        tm_min: tm.tm_min,
        tm_hour: tm.tm_hour,
        tm_mday: tm.tm_mday,
        tm_mon: tm.tm_mon,
        tm_year: tm.tm_year,
        tm_wday: tm.tm_wday,
        tm_yday: tm.tm_yday,
        tm_isdst: tm.tm_isdst,
        tm_usec: tm.tm_usec,
    };
    tm.tm_year = -1;
    tm.tm_mon = -1;
    tm.tm_mday = -1;
    tm.tm_usec = tv.tv_usec;

    let bytes = date.as_bytes();
    let mut idx = 0usize;
    while idx < bytes.len() {
        let c = bytes[idx];
        if c == 0 {
            break;
        }
        idx += 1;
        if is_digit_byte(c) {
            pending_number_internal(&mut tm, &mut number);
            let sub = &date[idx - 1..];
            let consumed = approxidate_digit_internal(sub, &mut tm, &mut number, time_sec);
            idx = (idx - 1) + consumed;
            touched = 1;
            continue;
        }
        if is_alpha_byte(c) {
            let sub = &date[idx - 1..];
            let consumed = approxidate_alpha_internal(sub, &mut tm, &mut now, &mut number, &mut touched);
            idx = (idx - 1) + consumed;
        }
    }

    pending_number_internal(&mut tm, &mut number);
    if touched == 0 {
        return -1;
    }
    tv.tv_usec = tm.tm_usec;
    tv.tv_sec = update_tm(&mut tm, &mut now, 0) as i64;
    0
}

pub fn approxidate_main(date: &str, tv: &mut TimeVal) -> i32 {
    let mut dummy = TimeVal { tv_sec: 0, tv_usec: 0 };
    // Use approximate_relative behavior with relative_to=None
    approxidate_relative_inner(date, tv, None, &mut dummy)
}

pub fn approxidate(date: &str, tv: &mut TimeVal) -> i32 {
    approxidate_main(date, tv)
}

fn approxidate_relative_inner(
    date: &str,
    tv: &mut TimeVal,
    relative_to: Option<&TimeVal>,
    _scratch: &mut TimeVal,
) -> i32 {
    let mut offset: i32 = 0;
    if parse_date_basic(date, tv, &mut offset) == 0 {
        return 0;
    }
    if let Some(rel) = relative_to {
        *tv = *rel;
    } else {
        use std::time::{SystemTime, UNIX_EPOCH};
        let d = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or(std::time::Duration::ZERO);
        tv.tv_sec = d.as_secs() as i64;
        tv.tv_usec = d.subsec_micros() as i64;
    }
    if approxidate_str(date, tv) == 0 {
        return 0;
    }
    -1
}

pub fn approxidate_relative(date: &str, tv: &mut TimeVal, relative_to: &mut TimeVal) -> i32 {
    let rel_copy = *relative_to;
    let mut scratch = TimeVal { tv_sec: 0, tv_usec: 0 };
    approxidate_relative_inner(date, tv, Some(&rel_copy), &mut scratch)
}
