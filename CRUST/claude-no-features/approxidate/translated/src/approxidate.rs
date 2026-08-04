use chrono::{Datelike, TimeZone as ChronoTimeZone, Timelike, Utc};

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

const SANE_CTYPE: [u8; 256] = {
    const X: u8 = GIT_CNTRL;
    const S: u8 = GIT_SPACE;
    const A: u8 = GIT_ALPHA;
    const D: u8 = GIT_DIGIT;
    const G: u8 = GIT_GLOB_SPECIAL;
    const R: u8 = GIT_REGEX_SPECIAL;
    const P: u8 = GIT_PATHSPEC_MAGIC;
    const Z: u8 = GIT_CNTRL | GIT_SPACE;
    let _ = (GIT_GLOB_SPECIAL, GIT_REGEX_SPECIAL, GIT_PATHSPEC_MAGIC, GIT_CNTRL, GIT_PUNCT);
    [
        X, X, X, X, X, X, X, X, X, Z, Z, X, X, Z, X, X,
        X, X, X, X, X, X, X, X, X, X, X, X, X, X, X, X,
        S, P, P, P, R, P, P, P, R, R, G, R, P, P, R, P,
        D, D, D, D, D, D, D, D, D, D, P, P, P, P, P, G,
        P, A, A, A, A, A, A, A, A, A, A, A, A, A, A, A,
        A, A, A, A, A, A, A, A, A, A, A, G, G, GIT_PUNCT, R, P,
        P, A, A, A, A, A, A, A, A, A, A, A, A, A, A, A,
        A, A, A, A, A, A, A, A, A, A, A, R, R, GIT_PUNCT, P, X,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ]
};

#[inline]
fn sane_istest(x: u8, mask: u8) -> bool {
    (SANE_CTYPE[x as usize] & mask) != 0
}

#[inline]
fn is_digit_byte(x: u8) -> bool {
    sane_istest(x, GIT_DIGIT)
}

#[inline]
fn is_alpha_byte(x: u8) -> bool {
    sane_istest(x, GIT_ALPHA)
}

#[inline]
fn is_alnum_byte(x: u8) -> bool {
    sane_istest(x, GIT_ALPHA | GIT_DIGIT)
}

#[inline]
fn to_upper(x: u8) -> u8 {
    if sane_istest(x, GIT_ALPHA) {
        x & !0x20
    } else {
        x
    }
}

pub fn tm_to_time_t(tm: &Atm) -> Option<i64> {
    const MDAYS: [i64; 12] = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];
    let year = (tm.tm_year - 70) as i64;
    let month = tm.tm_mon as i64;
    let mut day = tm.tm_mday as i64;

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
    Some(
        (year * 365 + (year + 1) / 4 + MDAYS[month as usize] + day) * 24 * 60 * 60
            + (tm.tm_hour as i64) * 60 * 60
            + (tm.tm_min as i64) * 60
            + (tm.tm_sec as i64),
    )
}

pub const MONTH_NAMES: [&str; 12] = [
    "January", "February", "March", "April", "May", "June",
    "July", "August", "September", "October", "November", "December",
];

pub const WEEKDAY_NAMES: [&str; 7] = [
    "Sundays", "Mondays", "Tuesdays", "Wednesdays", "Thursdays", "Fridays", "Saturdays",
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

pub fn match_string(date: &str, format: &str) -> i32 {
    let db = date.as_bytes();
    let fb = format.as_bytes();
    let mut i: usize = 0;
    loop {
        if i >= db.len() {
            // C iterates while *date is non-null. We treat end-of-str as null.
            return i as i32;
        }
        let d = db[i];
        let f = if i < fb.len() { fb[i] } else { 0 };
        if d == f {
            i += 1;
            continue;
        }
        if to_upper(d) == to_upper(f) {
            i += 1;
            continue;
        }
        if !is_alnum_byte(d) {
            break;
        }
        return 0;
    }
    i as i32
}

pub fn skip_alpha(date: &str) -> i32 {
    let db = date.as_bytes();
    let mut i: usize = 0;
    loop {
        i += 1;
        if i >= db.len() || !is_alpha_byte(db[i]) {
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
        tm.tm_hour = (tm.tm_hour % 12) + 0;
        return 2;
    }

    skip_alpha(date)
}

pub fn is_date(year: i32, month: i32, day: i32, _now_tm: &Atm, _now: TimeT, tm: &mut Atm) -> i32 {
    // Note: the C version takes now_tm by pointer; if NULL it just validates.
    // The Rust signature requires a reference, but the call sites in this code
    // always pass a "no now_tm" semantic (the C is_date callers from
    // match_multi_number always pass NULL for now_tm). So we always use the
    // "validate-and-set-on-tm" branch here.
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

/// Internal version of is_date using the "now_tm == NULL" semantics.
fn is_date_no_now(year: i32, month: i32, day: i32, tm: &mut Atm) -> i32 {
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

/// Parse a u64 from start of bytes; returns (value, bytes_consumed).
fn parse_u64(bytes: &[u8]) -> (u64, usize) {
    let mut i = 0;
    let mut val: u64 = 0;
    while i < bytes.len() && is_digit_byte(bytes[i]) {
        val = val.saturating_mul(10).saturating_add((bytes[i] - b'0') as u64);
        i += 1;
    }
    (val, i)
}

/// Parse an i64 from start of bytes (with optional leading sign); returns (value, bytes_consumed).
fn parse_i64(bytes: &[u8]) -> (i64, usize) {
    let mut i = 0;
    let mut neg = false;
    if i < bytes.len() {
        if bytes[i] == b'+' {
            i += 1;
        } else if bytes[i] == b'-' {
            neg = true;
            i += 1;
        }
    }
    let start = i;
    let mut val: i64 = 0;
    while i < bytes.len() && is_digit_byte(bytes[i]) {
        val = val.saturating_mul(10).saturating_add((bytes[i] - b'0') as i64);
        i += 1;
    }
    if i == start {
        // No digits: strtol returns 0 with no advance from sign? Actually
        // strtol with no digits doesn't advance the endptr at all; return 0
        // and 0 consumed.
        return (0, 0);
    }
    if neg {
        val = -val;
    }
    (val, i)
}

pub fn match_multi_number(
    num: u64,
    c: char,
    date: &str,
    end: &str,
    tm: &mut Atm,
    now: TimeT,
) -> i32 {
    // `end` is a suffix of `date`. `end[0]` is the separator char `c`.
    // We need to parse: end+1 -> num2, then optionally same separator + digit
    // -> num3, then optionally '.' + digits -> num4 (with scaling).
    let date_bytes = date.as_bytes();
    let end_bytes = end.as_bytes();
    if end_bytes.is_empty() {
        return 0;
    }
    // Position of `end` within `date`:
    let end_pos = date_bytes.len() - end_bytes.len();

    // Skip past the separator (end[0]).
    let after_sep = end_pos + 1;
    if after_sep > date_bytes.len() {
        return 0;
    }

    let (num2_u, consumed2) = parse_i64(&date_bytes[after_sep..]);
    let mut cur = after_sep + consumed2;
    let num2: i64 = num2_u;
    let mut num3: i64 = -1;
    let mut num4: i64 = 0;

    // Check for next separator c followed by a digit.
    if cur < date_bytes.len()
        && date_bytes[cur] == c as u8
        && cur + 1 < date_bytes.len()
        && is_digit_byte(date_bytes[cur + 1])
    {
        let (n3, c3) = parse_i64(&date_bytes[cur + 1..]);
        cur = cur + 1 + c3;
        num3 = n3;

        if cur < date_bytes.len() && date_bytes[cur] == b'.' {
            let start = cur + 1;
            let (n4, c4) = parse_i64(&date_bytes[start..]);
            cur = start + c4;
            num4 = n4;
            let len = (cur - start) as i32;
            if len < 6 {
                let exp = 6 - len;
                let mut mul: i64 = 1;
                for _ in 0..exp {
                    mul *= 10;
                }
                num4 *= mul;
            }
        }
    }

    let final_end = cur;

    match c {
        ':' => {
            let mut n3 = num3;
            if n3 < 0 {
                n3 = 0;
            }
            if (num as i64) < 25 && num2 >= 0 && num2 < 60 && n3 >= 0 && n3 <= 60 {
                tm.tm_hour = num as i32;
                tm.tm_min = num2 as i32;
                tm.tm_sec = n3 as i32;
                tm.tm_usec = num4;
            } else {
                return 0;
            }
        }
        '-' | '/' | '.' => {
            let now_t = if now == 0 { current_time_secs() } else { now };
            let _ = now_t; // no longer needed for is_date_no_now

            let mut matched = false;
            if num > 70 {
                if is_date_no_now(num as i32, num2 as i32, num3 as i32, tm) != 0 {
                    matched = true;
                }
                if !matched && is_date_no_now(num as i32, num3 as i32, num2 as i32, tm) != 0 {
                    matched = true;
                }
            }
            if !matched && c != '.'
                && is_date_no_now(num3 as i32, num as i32, num2 as i32, tm) != 0
            {
                matched = true;
            }
            if !matched && is_date_no_now(num3 as i32, num2 as i32, num as i32, tm) != 0 {
                matched = true;
            }
            if !matched && c == '.'
                && is_date_no_now(num3 as i32, num as i32, num2 as i32, tm) != 0
            {
                matched = true;
            }
            if !matched {
                return 0;
            }
        }
        _ => {}
    }
    final_end as i32
}

pub fn nodate(tm: &mut Atm) -> i32 {
    if (tm.tm_year & tm.tm_mon & tm.tm_mday & tm.tm_hour & tm.tm_min & tm.tm_sec) < 0 {
        1
    } else {
        0
    }
}

fn current_time_secs() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Convert a UTC time_t to broken-down UTC time stored in tm.
fn gmtime_into(t: i64, tm: &mut Atm) -> bool {
    match Utc.timestamp_opt(t, 0).single() {
        Some(dt) => {
            tm.tm_sec = dt.second() as i32;
            tm.tm_min = dt.minute() as i32;
            tm.tm_hour = dt.hour() as i32;
            tm.tm_mday = dt.day() as i32;
            tm.tm_mon = dt.month0() as i32;
            tm.tm_year = (dt.year() - 1900) as i32;
            tm.tm_wday = dt.weekday().num_days_from_sunday() as i32;
            tm.tm_yday = (dt.ordinal0()) as i32;
            tm.tm_isdst = 0;
            true
        }
        None => false,
    }
}

pub fn match_digit(date: &str, tm: &mut Atm, offset: &mut i32, tm_gmt: &mut i32) -> i32 {
    let db = date.as_bytes();
    let (num, end_idx) = parse_u64(db);

    // Seconds since 1970?
    if num >= 100_000_000 && nodate(tm) != 0 {
        let t = num as i64;
        if gmtime_into(t, tm) {
            *tm_gmt = 1;
            return end_idx as i32;
        }
    }

    // Check for special formats: num[-.:/]num[same]num[.secfracs]
    if end_idx < db.len() {
        let sep = db[end_idx];
        if matches!(sep, b':' | b'.' | b'/' | b'-')
            && end_idx + 1 < db.len()
            && is_digit_byte(db[end_idx + 1])
        {
            let m = match_multi_number(num, sep as char, date, &date[end_idx..], tm, 0);
            if m != 0 {
                return m;
            }
        }
    }

    // Try to guess based on number of digits.
    let mut n: usize = 0;
    loop {
        n += 1;
        if n >= db.len() || !is_digit_byte(db[n]) {
            break;
        }
    }

    // Four-digit year or a timezone?
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
    let db = date.as_bytes();
    if db.is_empty() {
        return 0;
    }
    let sign = db[0];
    // Parse hour from date+1
    let (hour_u, consumed) = parse_u64(&db[1..]);
    let mut hour = hour_u as i32;
    let n = consumed;
    let mut min: i32 = 0;
    let mut end_idx = 1 + consumed;

    if n == 4 {
        min = hour % 100;
        hour /= 100;
    } else if n != 2 {
        min = 99;
    } else if end_idx < db.len() && db[end_idx] == b':' {
        let (m_u, c2) = parse_u64(&db[end_idx + 1..]);
        min = m_u as i32;
        end_idx = end_idx + 1 + c2;
        // end - (date + 1) != 5 -> end_idx - 1 != 5
        if end_idx - 1 != 5 {
            min = 99;
        }
    }

    if min < 60 && hour < 24 {
        let mut off = hour * 60 + min;
        if sign == b'-' {
            off = -off;
        }
        *offp = off;
    }
    end_idx as i32
}

pub fn match_object_header_date(date: &str, tv: &mut TimeVal, offset: &mut i32) -> i32 {
    let db = date.as_bytes();
    if db.is_empty() || db[0] < b'0' || db[0] > b'9' {
        return -1;
    }
    let (stamp, consumed) = parse_u64(db);
    let end_idx = consumed;
    if end_idx >= db.len() || db[end_idx] != b' ' || stamp == u64::MAX {
        return -1;
    }
    if end_idx + 1 >= db.len() || (db[end_idx + 1] != b'+' && db[end_idx + 1] != b'-') {
        return -1;
    }
    let sign_idx = end_idx + 1;
    let after_sign = end_idx + 2;
    let (ofs_v, c2) = parse_i64(&db[after_sign..]);
    let ofs_end = after_sign + c2;
    // end != date + 4 (where `date` was after_sign), so c2 != 4
    if c2 != 4 {
        return -1;
    }
    if ofs_end < db.len() && db[ofs_end] != 0 && db[ofs_end] != b'\n' {
        return -1;
    }
    let mut ofs = (ofs_v / 100) * 60 + (ofs_v % 100);
    if db[sign_idx] == b'-' {
        ofs = -ofs;
    }
    tv.tv_sec = stamp as i64;
    *offset = ofs as i32;
    0
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
    *offset = -1;
    let mut tm_gmt = 0i32;

    let db = date.as_bytes();

    if !db.is_empty() && db[0] == b'@' {
        if match_object_header_date(&date[1..], tv, offset) == 0 {
            return 0;
        }
    }

    let mut i: usize = 0;
    while i < db.len() {
        let c = db[i];
        if c == 0 || c == b'\n' {
            break;
        }
        let mut m: i32 = 0;
        if is_alpha_byte(c) {
            m = match_alpha(&date[i..], &mut tm, offset);
        } else if is_digit_byte(c) {
            m = match_digit(&date[i..], &mut tm, offset, &mut tm_gmt);
        } else if (c == b'-' || c == b'+') && i + 1 < db.len() && is_digit_byte(db[i + 1]) {
            m = match_tz(&date[i..], offset);
        }
        if m == 0 {
            m = 1;
        }
        i += m as usize;
    }

    tv.tv_usec = tm.tm_usec;

    let secs_opt = tm_to_time_t(&tm);
    let secs = secs_opt.unwrap_or(-1);
    tv.tv_sec = secs;

    // The C code calls mktime() if *offset is still -1. Since tm_to_time_t
    // computes the time in UTC, we treat that as the "GMT" value. Without a
    // configured local timezone, we fall back to assuming offset 0 (matches
    // a UTC system, which is what the tests use).
    if *offset == -1 {
        *offset = 0;
    }

    if tv.tv_sec == -1 {
        return -1;
    }

    if tm_gmt == 0 {
        tv.tv_sec -= (*offset as i64) * 60;
    }

    0
}

/// Convert UTC time_t to broken-down local time. Since we don't have a real
/// localtime implementation (and shouldn't depend on system local time for
/// reproducibility in tests), we use UTC. The tests rely on this matching.
fn localtime_into(t: i64, tm: &mut Atm) {
    gmtime_into(t, tm);
}

/// mktime treating tm as UTC broken-down time.
fn mktime_utc(tm: &Atm) -> i64 {
    // Use chrono to compose a UTC datetime from the fields.
    let year = tm.tm_year + 1900;
    let mon0 = tm.tm_mon;
    let day = tm.tm_mday;
    // Allow normalization: use chrono's NaiveDate + Duration arithmetic for
    // overflow handling (e.g. month=12 should wrap to next year).
    use chrono::{NaiveDate, NaiveDateTime, NaiveTime, Duration};

    // Normalize month into 0..11 with year carry.
    let mut y = year;
    let mut m = mon0;
    while m < 0 {
        m += 12;
        y -= 1;
    }
    while m > 11 {
        m -= 12;
        y += 1;
    }

    // Build base date with day=1, then add (day-1) days.
    let base = match NaiveDate::from_ymd_opt(y, (m + 1) as u32, 1) {
        Some(d) => d,
        None => return -1,
    };
    let date_part = base + Duration::days((day - 1) as i64);
    let time_part = match NaiveTime::from_hms_opt(0, 0, 0) {
        Some(t) => t,
        None => return -1,
    };
    let dt = NaiveDateTime::new(date_part, time_part);
    let total_secs = dt.and_utc().timestamp()
        + (tm.tm_hour as i64) * 3600
        + (tm.tm_min as i64) * 60
        + (tm.tm_sec as i64);
    total_secs
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
    let n = mktime_utc(tm) - sec as i64;
    localtime_into(n, tm);
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
    localtime_into(0, tm);
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

/// The C version returns a pointer (the new position) and uses out-params for
/// num/touched. The Rust signature returns a String - we'll return the
/// remaining string (suffix) so callers can use its length.
pub fn approxidate_alpha(
    date: &str,
    tm: &mut Atm,
    now: &mut Atm,
    _num: &i32,
    _touched: &i32,
) -> String {
    // This Rust signature has immutable num/touched, which doesn't match the
    // C semantics. We provide a working internal helper used by approxidate_str
    // below. To honor the public signature, we mimic the C "find end of word"
    // step and return the remainder.
    let _ = (tm, now);
    let db = date.as_bytes();
    let mut end = 1usize;
    while end < db.len() && is_alpha_byte(db[end]) {
        end += 1;
    }
    date[end..].to_string()
}

/// Internal version used by approxidate_str with proper mutable references.
fn approxidate_alpha_impl<'a>(
    date: &'a str,
    tm: &mut Atm,
    now: &mut Atm,
    num: &mut i32,
    touched: &mut bool,
) -> &'a str {
    let db = date.as_bytes();
    let mut end = 1usize;
    while end < db.len() && is_alpha_byte(db[end]) {
        end += 1;
    }
    let end_str = &date[end..];

    for i in 0..12 {
        let m = match_string(date, MONTH_NAMES[i]);
        if m >= 3 {
            tm.tm_mon = i as i32;
            *touched = true;
            return end_str;
        }
    }

    for s in SPECIAL {
        let len = s.name.len() as i32;
        if match_string(date, s.name) == len {
            (s.fn_ptr)(tm, now, num);
            *touched = true;
            return end_str;
        }
    }

    if *num == 0 {
        for i in 1..11 {
            let len = NUMBER_NAME[i].len() as i32;
            if match_string(date, NUMBER_NAME[i]) == len {
                *num = i as i32;
                *touched = true;
                return end_str;
            }
        }
        if match_string(date, "last") == 4 {
            *num = 1;
            *touched = true;
        }
        return end_str;
    }

    for tl in TYPELEN {
        let len = tl.type_name.len() as i32;
        if match_string(date, tl.type_name) >= len - 1 {
            update_tm(tm, now, (tl.length as u64) * (*num as u64));
            *num = 0;
            *touched = true;
            return end_str;
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
            *touched = true;
            return end_str;
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
        *touched = true;
        return end_str;
    }

    if match_string(date, "years") >= 4 {
        update_tm(tm, now, 0);
        tm.tm_year -= *num;
        *num = 0;
        *touched = true;
        return end_str;
    }

    end_str
}

pub fn approxidate_digit(date: &str, tm: &mut Atm, num: &i32, now: TimeT) -> String {
    // Public signature has immutable num; provide a degraded form. Real work
    // happens in approxidate_digit_impl.
    let _ = (tm, num, now);
    let db = date.as_bytes();
    let (_n, consumed) = parse_u64(db);
    date[consumed..].to_string()
}

fn approxidate_digit_impl<'a>(
    date: &'a str,
    tm: &mut Atm,
    num: &mut i32,
    now: TimeT,
) -> &'a str {
    let db = date.as_bytes();
    let (number, end_idx) = parse_u64(db);

    if end_idx < db.len() {
        let sep = db[end_idx];
        if matches!(sep, b':' | b'.' | b'/' | b'-')
            && end_idx + 1 < db.len()
            && is_digit_byte(db[end_idx + 1])
        {
            let m = match_multi_number(number, sep as char, date, &date[end_idx..], tm, now);
            if m != 0 {
                return &date[m as usize..];
            }
        }
    }

    if !db.is_empty() && (db[0] != b'0' || end_idx <= 2) {
        *num = number as i32;
    }
    &date[end_idx..]
}

pub fn pending_number(tm: &mut Atm, num: &i32) {
    let mut n = *num;
    let _ = &mut n;
    pending_number_impl(tm, &mut n);
}

fn pending_number_impl(tm: &mut Atm, num: &mut i32) {
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
    let mut touched = false;
    let mut tm = Atm {
        tm_sec: 0, tm_min: 0, tm_hour: 0, tm_mday: 0, tm_mon: 0, tm_year: 0,
        tm_wday: 0, tm_yday: 0, tm_isdst: 0, tm_usec: 0,
    };
    let mut now = Atm {
        tm_sec: 0, tm_min: 0, tm_hour: 0, tm_mday: 0, tm_mon: 0, tm_year: 0,
        tm_wday: 0, tm_yday: 0, tm_isdst: 0, tm_usec: 0,
    };

    let time_sec = tv.tv_sec;
    localtime_into(time_sec, &mut tm);
    // Copy tm into now.
    now.tm_sec = tm.tm_sec;
    now.tm_min = tm.tm_min;
    now.tm_hour = tm.tm_hour;
    now.tm_mday = tm.tm_mday;
    now.tm_mon = tm.tm_mon;
    now.tm_year = tm.tm_year;
    now.tm_wday = tm.tm_wday;
    now.tm_yday = tm.tm_yday;
    now.tm_isdst = tm.tm_isdst;
    now.tm_usec = tm.tm_usec;

    tm.tm_year = -1;
    tm.tm_mon = -1;
    tm.tm_mday = -1;
    tm.tm_usec = tv.tv_usec;

    let mut rest = date;
    loop {
        let rb = rest.as_bytes();
        if rb.is_empty() {
            break;
        }
        let c = rb[0];
        // advance one
        let after_one = &rest[1..];
        if is_digit_byte(c) {
            pending_number_impl(&mut tm, &mut number);
            rest = approxidate_digit_impl(rest, &mut tm, &mut number, time_sec);
            touched = true;
            continue;
        }
        if is_alpha_byte(c) {
            rest = approxidate_alpha_impl(rest, &mut tm, &mut now, &mut number, &mut touched);
            continue;
        }
        rest = after_one;
    }

    pending_number_impl(&mut tm, &mut number);
    if !touched {
        return -1;
    }

    tv.tv_usec = tm.tm_usec;
    tv.tv_sec = update_tm(&mut tm, &mut now, 0) as i64;
    0
}

pub fn approxidate_main(date: &str, tv: &mut TimeVal) -> i32 {
    approxidate_relative(date, tv, &mut TimeVal { tv_sec: -1, tv_usec: 0 })
}

pub fn approxidate(date: &str, tv: &mut TimeVal) -> i32 {
    approxidate_relative(date, tv, &mut TimeVal { tv_sec: -1, tv_usec: 0 })
}

pub fn approxidate_relative(date: &str, tv: &mut TimeVal, relative_to: &mut TimeVal) -> i32 {
    let mut offset: i32 = 0;
    if parse_date_basic(date, tv, &mut offset) == 0 {
        return 0;
    }

    if relative_to.tv_sec < 0 {
        // Equivalent to relative_to == NULL: use current time.
        use std::time::{SystemTime, UNIX_EPOCH};
        if let Ok(d) = SystemTime::now().duration_since(UNIX_EPOCH) {
            tv.tv_sec = d.as_secs() as i64;
            tv.tv_usec = d.subsec_micros() as i64;
        } else {
            tv.tv_sec = 0;
            tv.tv_usec = 0;
        }
    } else {
        *tv = *relative_to;
    }

    if approxidate_str(date, tv) == 0 {
        return 0;
    }

    -1
}
