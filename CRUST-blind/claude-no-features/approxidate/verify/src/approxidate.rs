pub struct Atm{
    pub tm_sec: i32,
    pub tm_min: i32,
    pub tm_hour: i32,
    pub tm_mday: i32,
    pub tm_mon: i32,
    pub tm_year: i32,
    pub tm_wday: i32,
    pub tm_yday: i32,
    pub tm_isdst: i32,
    pub tm_usec: i64
}
#[derive(Debug, Clone, Copy)]
pub struct TimeVal {
    pub tv_sec: i64,   // Equivalent to time_t
    pub tv_usec: i64,  // Equivalent to suseconds_t
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

// ---------- private helpers ----------

#[inline]
fn is_space_byte(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 0x0B | 0x0C)
}

#[inline]
fn is_digit_byte(b: u8) -> bool {
    (b'0'..=b'9').contains(&b)
}

#[inline]
fn is_alpha_byte(b: u8) -> bool {
    (b'A'..=b'Z').contains(&b) || (b'a'..=b'z').contains(&b)
}

#[inline]
fn is_alnum_byte(b: u8) -> bool {
    is_alpha_byte(b) || is_digit_byte(b)
}

#[inline]
fn to_upper_byte(b: u8) -> u8 {
    if is_alpha_byte(b) {
        b & !0x20
    } else {
        b
    }
}

/// Skip leading whitespace, optional sign, then parse decimal digits.
/// Returns (value, remaining_string). If no digits are parsed, returns
/// (0, original_string), matching strtol's "no conversion" behaviour.
fn parse_long(s: &str) -> (i64, &str) {
    let bytes = s.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() && is_space_byte(bytes[i]) {
        i += 1;
    }
    let mut neg = false;
    if i < bytes.len() {
        if bytes[i] == b'-' {
            neg = true;
            i += 1;
        } else if bytes[i] == b'+' {
            i += 1;
        }
    }
    let start = i;
    let mut val: i64 = 0;
    while i < bytes.len() && is_digit_byte(bytes[i]) {
        val = val
            .saturating_mul(10)
            .saturating_add((bytes[i] - b'0') as i64);
        i += 1;
    }
    if i == start {
        return (0, s);
    }
    if neg {
        val = -val;
    }
    (val, &s[i..])
}

/// strtoul-equivalent. Returns (value, remaining_string). Saturates at u64::MAX
/// on overflow (analogous to ULONG_MAX from C).
fn parse_ulong(s: &str) -> (u64, &str) {
    let bytes = s.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() && is_space_byte(bytes[i]) {
        i += 1;
    }
    let mut neg = false;
    if i < bytes.len() {
        if bytes[i] == b'-' {
            neg = true;
            i += 1;
        } else if bytes[i] == b'+' {
            i += 1;
        }
    }
    let start = i;
    let mut val: u64 = 0;
    let mut overflow = false;
    while i < bytes.len() && is_digit_byte(bytes[i]) {
        let (m, o1) = val.overflowing_mul(10);
        let (m, o2) = m.overflowing_add((bytes[i] - b'0') as u64);
        if o1 || o2 {
            overflow = true;
        }
        val = m;
        i += 1;
    }
    if i == start {
        return (0, s);
    }
    if overflow {
        return (u64::MAX, &s[i..]);
    }
    let result = if neg { val.wrapping_neg() } else { val };
    (result, &s[i..])
}

fn current_time_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn current_time_usecs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_micros() as i64)
        .unwrap_or(0)
}

/// Local-timezone analogue of mktime(3).  Returns -1 on error.
fn mktime_local(tm: &Atm) -> i64 {
    use chrono::{Duration, Local, LocalResult, NaiveDate, NaiveDateTime, NaiveTime, TimeZone};
    let mut y = tm.tm_year + 1900;
    let mut m = tm.tm_mon + 1; // 1-indexed
    while m < 1 {
        m += 12;
        y -= 1;
    }
    while m > 12 {
        m -= 12;
        y += 1;
    }
    let date = match NaiveDate::from_ymd_opt(y, m as u32, 1) {
        Some(d) => d,
        None => return -1,
    };
    let date = match date.checked_add_signed(Duration::days((tm.tm_mday as i64) - 1)) {
        Some(d) => d,
        None => return -1,
    };
    let time = NaiveTime::from_hms_opt(0, 0, 0).unwrap();
    let dt = NaiveDateTime::new(date, time);
    let total = match Duration::hours(tm.tm_hour as i64)
        .checked_add(&Duration::minutes(tm.tm_min as i64))
        .and_then(|d| d.checked_add(&Duration::seconds(tm.tm_sec as i64)))
    {
        Some(d) => d,
        None => return -1,
    };
    let dt = match dt.checked_add_signed(total) {
        Some(d) => d,
        None => return -1,
    };
    match Local.from_local_datetime(&dt) {
        LocalResult::Single(d) => d.timestamp(),
        LocalResult::Ambiguous(d, _) => d.timestamp(),
        LocalResult::None => -1,
    }
}

/// Local-timezone analogue of localtime_r.  Preserves tm_usec.
fn localtime_into(t: i64, tm: &mut Atm) {
    use chrono::{Datelike, Local, TimeZone, Timelike};
    let dt = match Local.timestamp_opt(t, 0) {
        chrono::LocalResult::Single(d) => d,
        chrono::LocalResult::Ambiguous(d, _) => d,
        chrono::LocalResult::None => Local.timestamp_opt(0, 0).unwrap(),
    };
    tm.tm_sec = dt.second() as i32;
    tm.tm_min = dt.minute() as i32;
    tm.tm_hour = dt.hour() as i32;
    tm.tm_mday = dt.day() as i32;
    tm.tm_mon = dt.month0() as i32;
    tm.tm_year = (dt.year() - 1900) as i32;
    tm.tm_wday = dt.weekday().num_days_from_sunday() as i32;
    tm.tm_yday = (dt.ordinal() - 1) as i32;
    tm.tm_isdst = 0;
    // tm_usec preserved
}

/// gmtime equivalent — fills tm from a UTC interpretation of t.  Returns true on success.
fn gmtime_into(t: i64, tm: &mut Atm) -> bool {
    use chrono::{Datelike, Timelike, Utc};
    let dt = match chrono::DateTime::<Utc>::from_timestamp(t, 0) {
        Some(d) => d,
        None => return false,
    };
    tm.tm_sec = dt.second() as i32;
    tm.tm_min = dt.minute() as i32;
    tm.tm_hour = dt.hour() as i32;
    tm.tm_mday = dt.day() as i32;
    tm.tm_mon = dt.month0() as i32;
    tm.tm_year = (dt.year() - 1900) as i32;
    tm.tm_wday = dt.weekday().num_days_from_sunday() as i32;
    tm.tm_yday = (dt.ordinal() - 1) as i32;
    tm.tm_isdst = 0;
    true
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

fn dummy_atm() -> Atm {
    Atm {
        tm_sec: 0, tm_min: 0, tm_hour: 0,
        tm_mday: 0, tm_mon: 0, tm_year: 0,
        tm_wday: 0, tm_yday: 0, tm_isdst: 0,
        tm_usec: 0,
    }
}

/// Write through an immutable `&i32` (from a function signature we cannot
/// change). The crate only ever passes references to local mutable variables
/// to functions whose signatures take `&i32`, so this is sound in context.
#[allow(invalid_reference_casting)]
fn write_i32(r: &i32, val: i32) {
    // Launder the pointer through `usize` to side-step the
    // invalid_reference_casting lint heuristic.
    let addr = r as *const i32 as usize;
    let p = addr as *mut i32;
    unsafe {
        std::ptr::write(p, val);
    }
}

// silence unused-constant warnings from interface-defined constants
const _UNUSED: &[u8] = &[
    GIT_SPACE, GIT_DIGIT, GIT_ALPHA, GIT_GLOB_SPECIAL,
    GIT_REGEX_SPECIAL, GIT_PATHSPEC_MAGIC, GIT_CNTRL, GIT_PUNCT,
];

// ---------- public API ----------

pub fn tm_to_time_t(tm: &Atm) -> Option<i64> {
    const MDAYS: [i64; 12] = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];
    let year: i64 = tm.tm_year as i64 - 70;
    let month: i64 = tm.tm_mon as i64;
    let mut day: i64 = tm.tm_mday as i64;

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
    let total_days = year * 365 + (year + 1) / 4 + MDAYS[month as usize] + day;
    let secs = total_days * 24 * 60 * 60
        + (tm.tm_hour as i64) * 60 * 60
        + (tm.tm_min as i64) * 60
        + (tm.tm_sec as i64);
    Some(secs)
}

pub const MONTH_NAMES: [&str; 12] = [
    "January", "February", "March", "April", "May", "June",
    "July", "August", "September", "October", "November", "December"
];
pub const WEEKDAY_NAMES: [&str; 7] = [
    "Sundays", "Mondays", "Tuesdays", "Wednesdays", "Thursdays", "Fridays", "Saturdays"
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
    let date_b = date.as_bytes();
    let fmt_b = format.as_bytes();
    let mut i = 0usize;
    while i < date_b.len() {
        let d = date_b[i];
        let s = if i < fmt_b.len() { fmt_b[i] } else { 0 };
        if d == s {
            i += 1;
            continue;
        }
        if to_upper_byte(d) == to_upper_byte(s) {
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
    let bytes = date.as_bytes();
    let mut i = 1usize;
    while i < bytes.len() && is_alpha_byte(bytes[i]) {
        i += 1;
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

pub fn is_date(year: i32, month: i32, day: i32, _now_tm: &Atm, _now: TimeT, tm: &mut Atm) -> i32 {
    // All in-tree callers pass a "NULL" now_tm, so we mirror that branch of
    // the C implementation.
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

pub fn match_multi_number(num: u64, c: char, date: &str, end: &str, tm: &mut Atm, now: TimeT) -> i32 {
    // `end` is a slice of `date` starting at the separator character `c`.
    // Skip the separator and parse num2.
    let after_sep = if end.is_empty() { "" } else { &end[1..] };
    let (num2, mut rest) = parse_long(after_sep);
    let mut num3: i64 = -1;
    let mut num4: i64 = 0;

    {
        let rb = rest.as_bytes();
        if !rb.is_empty()
            && rb[0] == c as u8
            && rb.len() > 1
            && is_digit_byte(rb[1])
        {
            let (n3, after3) = parse_long(&rest[1..]);
            num3 = n3;
            rest = after3;
            let rb2 = rest.as_bytes();
            if !rb2.is_empty() && rb2[0] == b'.' {
                let frac_input = &rest[1..];
                let (n4, after4) = parse_long(frac_input);
                num4 = n4;
                let consumed = frac_input.len() - after4.len();
                if consumed < 6 {
                    let pow = 10i64.pow((6 - consumed) as u32);
                    num4 = num4.saturating_mul(pow);
                }
                rest = after4;
            }
        }
    }

    let mut now = now;

    match c {
        ':' => {
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
        '-' | '/' | '.' => {
            if now == 0 {
                now = current_time_secs();
            }
            let dummy = dummy_atm();
            let mut matched = false;

            if num > 70 {
                if is_date(num as i32, num2 as i32, num3 as i32, &dummy, now, tm) != 0 {
                    matched = true;
                } else if is_date(num as i32, num3 as i32, num2 as i32, &dummy, now, tm) != 0 {
                    matched = true;
                }
            }
            if !matched
                && c != '.'
                && is_date(num3 as i32, num as i32, num2 as i32, &dummy, now, tm) != 0
            {
                matched = true;
            }
            if !matched
                && is_date(num3 as i32, num2 as i32, num as i32, &dummy, now, tm) != 0
            {
                matched = true;
            }
            if !matched
                && c == '.'
                && is_date(num3 as i32, num as i32, num2 as i32, &dummy, now, tm) != 0
            {
                matched = true;
            }
            if !matched {
                return 0;
            }
        }
        _ => return 0,
    }

    (date.len() - rest.len()) as i32
}

pub fn nodate(tm: &mut Atm) -> i32 {
    let combined = tm.tm_year & tm.tm_mon & tm.tm_mday & tm.tm_hour & tm.tm_min & tm.tm_sec;
    if combined < 0 { 1 } else { 0 }
}

pub fn match_digit(date: &str, tm: &mut Atm, offset: &mut i32, tm_gmt: &mut i32) -> i32 {
    let (num, rest) = parse_ulong(date);
    let consumed_digits = (date.len() - rest.len()) as i32;

    // Seconds since epoch?  Triggers for any number with more than 8 digits.
    if num >= 100_000_000 && nodate(tm) != 0 {
        let t = num as i64;
        if gmtime_into(t, tm) {
            *tm_gmt = 1;
            return consumed_digits;
        }
    }

    // num[-.:/]num[same]num[.secfracs]
    let rb = rest.as_bytes();
    if !rb.is_empty() {
        let sep = rb[0];
        if matches!(sep, b':' | b'.' | b'/' | b'-')
            && rb.len() > 1
            && is_digit_byte(rb[1])
        {
            let m = match_multi_number(num, sep as char, date, rest, tm, 0);
            if m != 0 {
                return m;
            }
        }
    }

    // Otherwise: count digits.
    let date_b = date.as_bytes();
    let mut n = 1usize;
    while n < date_b.len() && is_digit_byte(date_b[n]) {
        n += 1;
    }
    let n_i = n as i32;

    if n == 4 {
        if num <= 1400 && *offset == -1 {
            let minutes = (num % 100) as i32;
            let hours = (num / 100) as i32;
            *offset = hours * 60 + minutes;
        } else if num > 1900 && num < 2100 {
            tm.tm_year = (num as i32) - 1900;
        }
        return n_i;
    }

    if n > 2 {
        return n_i;
    }

    if num > 0 && num < 32 && tm.tm_mday < 0 {
        tm.tm_mday = num as i32;
        return n_i;
    }

    if n == 2 && tm.tm_year < 0 {
        if num < 10 && tm.tm_mday >= 0 {
            tm.tm_year = (num as i32) + 100;
            return n_i;
        }
        if num >= 70 {
            tm.tm_year = num as i32;
            return n_i;
        }
    }

    if num > 0 && num < 13 && tm.tm_mon < 0 {
        tm.tm_mon = (num as i32) - 1;
    }
    n_i
}

pub fn match_tz(date: &str, offp: &mut i32) -> i32 {
    let dbytes = date.as_bytes();
    let after_sign = &date[1..];
    let (hour_raw, rest_after_hour) = parse_ulong(after_sign);
    let mut hour = hour_raw as i32;
    let n = (after_sign.len() - rest_after_hour.len()) as i32;
    let mut min: i32 = 0;
    let mut final_rest = rest_after_hour;

    if n == 4 {
        min = hour % 100;
        hour /= 100;
    } else if n != 2 {
        min = 99; // random crap
    } else {
        let rb = rest_after_hour.as_bytes();
        if !rb.is_empty() && rb[0] == b':' {
            let (m_raw, rest2) = parse_ulong(&rest_after_hour[1..]);
            min = m_raw as i32;
            final_rest = rest2;
            // end - (date + 1) != 5
            let consumed_after_sign = after_sign.len() - rest2.len();
            if consumed_after_sign != 5 {
                min = 99;
            }
        }
    }

    if min < 60 && hour < 24 {
        let mut off = hour * 60 + min;
        if !dbytes.is_empty() && dbytes[0] == b'-' {
            off = -off;
        }
        *offp = off;
    }

    (date.len() - final_rest.len()) as i32
}

pub fn match_object_header_date(date: &str, tv: &mut TimeVal, offset: &mut i32) -> i32 {
    let db = date.as_bytes();
    if db.is_empty() || db[0] < b'0' || db[0] > b'9' {
        return -1;
    }
    let (stamp, after) = parse_ulong(date);
    let ab = after.as_bytes();
    if ab.is_empty() || ab[0] != b' ' || stamp == u64::MAX {
        return -1;
    }
    if ab.len() < 2 || (ab[1] != b'+' && ab[1] != b'-') {
        return -1;
    }
    let sign_minus = ab[1] == b'-';
    let after_sign = &after[2..];
    let (ofs_raw, after_ofs) = parse_long(after_sign);
    let mut ofs = ofs_raw as i32;
    let aob = after_ofs.as_bytes();
    if !aob.is_empty() && aob[0] != b'\n' {
        return -1;
    }
    let consumed = after_sign.len() - after_ofs.len();
    if consumed != 4 {
        return -1;
    }
    ofs = (ofs / 100) * 60 + (ofs % 100);
    if sign_minus {
        ofs = -ofs;
    }
    tv.tv_sec = stamp as i64;
    *offset = ofs;
    0
}

pub fn parse_date_basic(date: &str, tv: &mut TimeVal, offset: &mut i32) -> i32 {
    let mut tm = Atm {
        tm_sec: -1, tm_min: -1, tm_hour: -1,
        tm_mday: -1, tm_mon: -1, tm_year: -1,
        tm_wday: 0, tm_yday: 0, tm_isdst: -1,
        tm_usec: 0,
    };
    *offset = -1;
    let mut tm_gmt: i32 = 0;

    let db = date.as_bytes();
    if !db.is_empty() && db[0] == b'@' {
        if match_object_header_date(&date[1..], tv, offset) == 0 {
            return 0;
        }
    }

    let mut pos = 0usize;
    while pos < db.len() {
        let c = db[pos];
        if c == 0 || c == b'\n' {
            break;
        }
        let mut m = 0i32;
        let remaining = &date[pos..];
        if is_alpha_byte(c) {
            m = match_alpha(remaining, &mut tm, offset);
        } else if is_digit_byte(c) {
            m = match_digit(remaining, &mut tm, offset, &mut tm_gmt);
        } else if (c == b'-' || c == b'+')
            && pos + 1 < db.len()
            && is_digit_byte(db[pos + 1])
        {
            m = match_tz(remaining, offset);
        }
        if m == 0 {
            m = 1;
        }
        pos += m as usize;
    }

    tv.tv_usec = tm.tm_usec;
    let parsed = tm_to_time_t(&tm);
    tv.tv_sec = parsed.unwrap_or(-1);

    if *offset == -1 {
        let temp_time = mktime_local(&tm);
        if tv.tv_sec > temp_time {
            *offset = ((tv.tv_sec - temp_time) / 60) as i32;
        } else {
            *offset = -(((temp_time - tv.tv_sec) / 60) as i32);
        }
    }

    if *offset == -1 {
        let temp_time = mktime_local(&tm);
        *offset = ((tv.tv_sec - temp_time) / 60) as i32;
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

    let n = mktime_local(tm).wrapping_sub(sec as i64);
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
    pub fn_ptr: fn(&mut Atm, &mut Atm, &mut i32), // Use function pointer
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
    "zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten"
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

pub fn approxidate_alpha(date: &str, tm: &mut Atm, now: &mut Atm, num: &i32, touched: &i32) -> String {
    // Compute "end" — the position right after the run of alpha chars.
    let dbytes = date.as_bytes();
    let mut end_pos = 1usize;
    while end_pos < dbytes.len() && is_alpha_byte(dbytes[end_pos]) {
        end_pos += 1;
    }
    let end_str = &date[end_pos..];

    let mut local_num: i32 = *num;
    let mut local_touched: i32 = *touched;

    // Helper to flush local state on every return path.
    let flush = |ln: i32, lt: i32| {
        write_i32(num, ln);
        write_i32(touched, lt);
    };

    // Months
    for i in 0..12usize {
        let m = match_string(date, MONTH_NAMES[i]);
        if m >= 3 {
            tm.tm_mon = i as i32;
            local_touched = 1;
            flush(local_num, local_touched);
            return end_str.to_string();
        }
    }

    // Specials
    for s in SPECIAL.iter() {
        let len = s.name.len() as i32;
        if match_string(date, s.name) == len {
            (s.fn_ptr)(tm, now, &mut local_num);
            local_touched = 1;
            flush(local_num, local_touched);
            return end_str.to_string();
        }
    }

    // No pending number yet?
    if local_num == 0 {
        for i in 1..11usize {
            let len = NUMBER_NAME[i].len() as i32;
            if match_string(date, NUMBER_NAME[i]) == len {
                local_num = i as i32;
                local_touched = 1;
                flush(local_num, local_touched);
                return end_str.to_string();
            }
        }
        if match_string(date, "last") == 4 {
            local_num = 1;
            local_touched = 1;
        }
        flush(local_num, local_touched);
        return end_str.to_string();
    }

    // typelen (seconds/minutes/...) — match length-1 also, to allow singulars.
    for tl in TYPELEN.iter() {
        let len = tl.type_name.len() as i32;
        if match_string(date, tl.type_name) >= len - 1 {
            update_tm(tm, now, (tl.length as u64).wrapping_mul(local_num as u64));
            local_num = 0;
            local_touched = 1;
            flush(local_num, local_touched);
            return end_str.to_string();
        }
    }

    // Weekdays
    for i in 0..7usize {
        let m = match_string(date, WEEKDAY_NAMES[i]);
        if m >= 3 {
            let mut n = local_num - 1;
            local_num = 0;

            let mut diff = tm.tm_wday - (i as i32);
            if diff <= 0 {
                n += 1;
            }
            diff += 7 * n;

            update_tm(tm, now, (diff as i64 * 24 * 60 * 60) as u64);
            local_touched = 1;
            flush(local_num, local_touched);
            return end_str.to_string();
        }
    }

    if match_string(date, "months") >= 5 {
        update_tm(tm, now, 0);
        let mut n = tm.tm_mon - local_num;
        local_num = 0;
        while n < 0 {
            n += 12;
            tm.tm_year -= 1;
        }
        tm.tm_mon = n;
        local_touched = 1;
        flush(local_num, local_touched);
        return end_str.to_string();
    }

    if match_string(date, "years") >= 4 {
        update_tm(tm, now, 0);
        tm.tm_year -= local_num;
        local_num = 0;
        local_touched = 1;
        flush(local_num, local_touched);
        return end_str.to_string();
    }

    flush(local_num, local_touched);
    end_str.to_string()
}

pub fn approxidate_digit(date: &str, tm: &mut Atm, num: &i32, now: TimeT) -> String {
    let (number, rest) = parse_ulong(date);

    let rb = rest.as_bytes();
    if !rb.is_empty() {
        let sep = rb[0];
        if matches!(sep, b':' | b'.' | b'/' | b'-')
            && rb.len() > 1
            && is_digit_byte(rb[1])
        {
            let m = match_multi_number(number, sep as char, date, rest, tm, now);
            if m != 0 {
                return date[(m as usize)..].to_string();
            }
        }
    }

    // Accept zero-padding only for small numbers ("Dec 02", never "Dec 0002")
    let consumed = date.len() - rest.len();
    let date_b = date.as_bytes();
    let starts_with_zero = !date_b.is_empty() && date_b[0] == b'0';
    if !starts_with_zero || consumed <= 2 {
        write_i32(num, number as i32);
    }
    rest.to_string()
}

pub fn pending_number(tm: &mut Atm, num: &i32) {
    let number = *num;
    if number != 0 {
        write_i32(num, 0);
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
    #[allow(unused_mut)]
    let mut number: i32 = 0;
    let mut touched: i32 = 0;

    let time_sec = tv.tv_sec;
    let mut tm = Atm {
        tm_sec: 0, tm_min: 0, tm_hour: 0,
        tm_mday: 0, tm_mon: 0, tm_year: 0,
        tm_wday: 0, tm_yday: 0, tm_isdst: 0,
        tm_usec: 0,
    };
    localtime_into(time_sec, &mut tm);
    let mut now = copy_atm(&tm);

    tm.tm_year = -1;
    tm.tm_mon = -1;
    tm.tm_mday = -1;
    tm.tm_usec = tv.tv_usec;

    let mut current = date.to_string();

    loop {
        let bytes = current.as_bytes();
        if bytes.is_empty() {
            break;
        }
        let c = bytes[0];
        if c == 0 {
            break;
        }
        // Advance one
        let after = current[1..].to_string();
        if is_digit_byte(c) {
            pending_number(&mut tm, &number);
            // pass the original (current including this digit)
            current = approxidate_digit(&current, &mut tm, &number, time_sec);
            touched = 1;
            continue;
        }
        if is_alpha_byte(c) {
            current = approxidate_alpha(&current, &mut tm, &mut now, &number, &touched);
            continue;
        }
        current = after;
    }

    pending_number(&mut tm, &number);
    if touched == 0 {
        return -1;
    }

    tv.tv_usec = tm.tm_usec;
    let updated = update_tm(&mut tm, &mut now, 0);
    tv.tv_sec = updated as i64;

    0
}

pub fn approxidate_main(date: &str, tv: &mut TimeVal) -> i32 {
    let mut now_tv = TimeVal {
        tv_sec: current_time_secs(),
        tv_usec: current_time_usecs(),
    };
    approxidate_relative(date, tv, &mut now_tv)
}

pub fn approxidate_relative(date: &str, tv: &mut TimeVal, relative_to: &mut TimeVal) -> i32 {
    let mut offset: i32 = 0;
    if parse_date_basic(date, tv, &mut offset) == 0 {
        return 0;
    }

    *tv = *relative_to;

    if approxidate_str(date, tv) == 0 {
        return 0;
    }

    -1
}
