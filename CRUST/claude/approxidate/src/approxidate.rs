use chrono::{Datelike, Timelike};

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

const SANE_CTYPE: [u8; 256] = build_sane_ctype();

const fn build_sane_ctype() -> [u8; 256] {
    let s = GIT_SPACE;
    let a = GIT_ALPHA;
    let d = GIT_DIGIT;
    let g = GIT_GLOB_SPECIAL;
    let r = GIT_REGEX_SPECIAL;
    let p = GIT_PATHSPEC_MAGIC;
    let x = GIT_CNTRL;
    let u = GIT_PUNCT;
    let z = GIT_CNTRL | GIT_SPACE;
    let mut t = [0u8; 256];
    // 0..15
    let row0: [u8; 16] = [x, x, x, x, x, x, x, x, x, z, z, x, x, z, x, x];
    // 16..31
    let row1: [u8; 16] = [x, x, x, x, x, x, x, x, x, x, x, x, x, x, x, x];
    // 32..47
    let row2: [u8; 16] = [s, p, p, p, r, p, p, p, r, r, g, r, p, p, r, p];
    // 48..63
    let row3: [u8; 16] = [d, d, d, d, d, d, d, d, d, d, p, p, p, p, p, g];
    // 64..79
    let row4: [u8; 16] = [p, a, a, a, a, a, a, a, a, a, a, a, a, a, a, a];
    // 80..95
    let row5: [u8; 16] = [a, a, a, a, a, a, a, a, a, a, a, g, g, u, r, p];
    // 96..111
    let row6: [u8; 16] = [p, a, a, a, a, a, a, a, a, a, a, a, a, a, a, a];
    // 112..127
    let row7: [u8; 16] = [a, a, a, a, a, a, a, a, a, a, a, r, r, u, p, x];
    let mut i = 0;
    while i < 16 { t[i] = row0[i]; i += 1; }
    while i < 32 { t[i] = row1[i - 16]; i += 1; }
    while i < 48 { t[i] = row2[i - 32]; i += 1; }
    while i < 64 { t[i] = row3[i - 48]; i += 1; }
    while i < 80 { t[i] = row4[i - 64]; i += 1; }
    while i < 96 { t[i] = row5[i - 80]; i += 1; }
    while i < 112 { t[i] = row6[i - 96]; i += 1; }
    while i < 128 { t[i] = row7[i - 112]; i += 1; }
    t
}

fn sane_istest(c: u8, mask: u8) -> bool {
    (SANE_CTYPE[c as usize] & mask) != 0
}

fn is_digit_byte(c: u8) -> bool {
    sane_istest(c, GIT_DIGIT)
}

fn is_alpha_byte(c: u8) -> bool {
    sane_istest(c, GIT_ALPHA)
}

fn is_alnum_byte(c: u8) -> bool {
    sane_istest(c, GIT_ALPHA | GIT_DIGIT)
}

fn sane_case(c: u8) -> u8 {
    if is_alpha_byte(c) {
        c & !0x20
    } else {
        c
    }
}

fn first_byte(s: &str) -> u8 {
    s.as_bytes().first().copied().unwrap_or(0)
}

/// Parse a leading non-negative decimal integer from `s`.
/// Returns (value, number_of_chars_consumed). If no digit, returns (0, 0).
fn parse_u64(s: &str) -> (u64, usize) {
    let bytes = s.as_bytes();
    let mut i = 0;
    let mut v: u64 = 0;
    let mut overflow = false;
    while i < bytes.len() && is_digit_byte(bytes[i]) {
        let d = (bytes[i] - b'0') as u64;
        match v.checked_mul(10).and_then(|m| m.checked_add(d)) {
            Some(nv) => v = nv,
            None => {
                overflow = true;
                v = u64::MAX;
            }
        }
        i += 1;
    }
    if overflow { (u64::MAX, i) } else { (v, i) }
}

/// Parse a leading possibly-signed decimal integer from `s`.
/// Returns (value, chars_consumed).
fn parse_i64(s: &str) -> (i64, usize) {
    let bytes = s.as_bytes();
    let mut idx = 0;
    let mut neg = false;
    if !bytes.is_empty() {
        if bytes[0] == b'-' { neg = true; idx = 1; }
        else if bytes[0] == b'+' { idx = 1; }
    }
    let (v, c) = parse_u64(&s[idx..]);
    if c == 0 {
        return (0, 0);
    }
    let mut sv = v as i64;
    if neg { sv = -sv; }
    (sv, idx + c)
}

pub fn tm_to_time_t(tm: &Atm) -> Option<i64> {
    let mdays: [i64; 12] = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];
    let year = tm.tm_year - 70;
    let month = tm.tm_mon;
    let mut day = tm.tm_mday;
    if year < 0 || year > 129 { return None; }
    if month < 0 || month > 11 { return None; }
    if month < 2 || (year + 2) % 4 != 0 {
        day -= 1;
    }
    if tm.tm_hour < 0 || tm.tm_min < 0 || tm.tm_sec < 0 { return None; }
    let y = year as i64;
    let m = month as usize;
    Some((y * 365 + (y + 1) / 4 + mdays[m] + day as i64) * 24 * 60 * 60
        + tm.tm_hour as i64 * 60 * 60
        + tm.tm_min as i64 * 60
        + tm.tm_sec as i64)
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
    let db = date.as_bytes();
    let fb = format.as_bytes();
    let mut i = 0;
    while i < db.len() {
        let dc = db[i];
        if dc == 0 { break; }
        let fc = if i < fb.len() { fb[i] } else { 0 };
        if dc == fc {
            i += 1;
            continue;
        }
        if sane_case(dc) == sane_case(fc) {
            i += 1;
            continue;
        }
        if !is_alnum_byte(dc) {
            break;
        }
        return 0;
    }
    i as i32
}

pub fn skip_alpha(date: &str) -> i32 {
    let bytes = date.as_bytes();
    let mut i = 0usize;
    loop {
        i += 1;
        if i >= bytes.len() { break; }
        if !is_alpha_byte(bytes[i]) { break; }
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
            if tz.dst { off += 1; }
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
    // In all C call sites, now_tm is NULL, so we follow that branch.
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
    // `end` is a suffix of `date` (the chars after the first number was parsed).
    // We need to compute end - date offsets.
    let date_len = date.len();
    let end_len = end.len();
    if end_len > date_len { return 0; }
    let end_offset = date_len - end_len;
    // Skip the separator char (one byte) and parse num2.
    if end_len < 1 { return 0; }
    let after_sep = &date[end_offset + 1..];
    let (num2, c2) = parse_i64(after_sep);
    let mut cursor = end_offset + 1 + c2;
    let mut num3: i64 = -1;
    let mut num4: i64 = 0;
    let bytes = date.as_bytes();
    if cursor < bytes.len() && bytes[cursor] == c as u8 && cursor + 1 < bytes.len() && is_digit_byte(bytes[cursor + 1]) {
        let after_sep2 = &date[cursor + 1..];
        let (n3, c3) = parse_i64(after_sep2);
        num3 = n3;
        cursor = cursor + 1 + c3;
        if cursor < bytes.len() && bytes[cursor] == b'.' {
            let frac_start = cursor + 1;
            let after_dot = &date[frac_start..];
            let (n4, c4) = parse_i64(after_dot);
            num4 = n4;
            cursor = frac_start + c4;
            let frac_len = c4;
            if frac_len < 6 {
                let pow = 10i64.pow((6 - frac_len) as u32);
                num4 *= pow;
            }
        }
    }

    match c {
        ':' => {
            if num3 < 0 { num3 = 0; }
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
            let now_t: TimeT = if now != 0 { now } else { current_time_secs() };
            let dummy = make_dummy_atm();
            // yyyy-mm-dd?
            if num > 70 {
                let mut t1 = clone_atm(tm);
                if is_date(num as i32, num2 as i32, num3 as i32, &dummy, now_t, &mut t1) != 0 {
                    *tm = t1;
                    return (cursor) as i32;
                }
                let mut t2 = clone_atm(tm);
                if is_date(num as i32, num3 as i32, num2 as i32, &dummy, now_t, &mut t2) != 0 {
                    *tm = t2;
                    return (cursor) as i32;
                }
            }
            if c != '.' {
                let mut t = clone_atm(tm);
                if is_date(num3 as i32, num as i32, num2 as i32, &dummy, now_t, &mut t) != 0 {
                    *tm = t;
                    return (cursor) as i32;
                }
            }
            let mut t = clone_atm(tm);
            if is_date(num3 as i32, num2 as i32, num as i32, &dummy, now_t, &mut t) != 0 {
                *tm = t;
                return (cursor) as i32;
            }
            if c == '.' {
                let mut t = clone_atm(tm);
                if is_date(num3 as i32, num as i32, num2 as i32, &dummy, now_t, &mut t) != 0 {
                    *tm = t;
                    return (cursor) as i32;
                }
            }
            return 0;
        }
        _ => return 0,
    }
    cursor as i32
}

fn clone_atm(tm: &Atm) -> Atm {
    Atm {
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
    }
}

fn make_dummy_atm() -> Atm {
    Atm {
        tm_sec: 0, tm_min: 0, tm_hour: 0, tm_mday: 0, tm_mon: 0,
        tm_year: 0, tm_wday: 0, tm_yday: 0, tm_isdst: 0, tm_usec: 0,
    }
}

fn current_time_secs() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn current_time_with_usec() -> (i64, i64) {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| (d.as_secs() as i64, d.subsec_micros() as i64))
        .unwrap_or((0, 0))
}

pub fn nodate(tm: &mut Atm) -> i32 {
    let result = tm.tm_year & tm.tm_mon & tm.tm_mday & tm.tm_hour & tm.tm_min & tm.tm_sec;
    if result < 0 { 1 } else { 0 }
}

pub fn match_digit(date: &str, tm: &mut Atm, offset: &mut i32, tm_gmt: &mut i32) -> i32 {
    let (num, num_len) = parse_u64(date);
    let end_offset = num_len;
    let bytes = date.as_bytes();

    // 8+ digit timestamps -> seconds since 1970, only if no date fields set.
    if num >= 100000000 && nodate(tm) != 0 {
        if let Some(dt) = chrono::DateTime::<chrono::Utc>::from_timestamp(num as i64, 0) {
            tm.tm_sec = dt.second() as i32;
            tm.tm_min = dt.minute() as i32;
            tm.tm_hour = dt.hour() as i32;
            tm.tm_mday = dt.day() as i32;
            tm.tm_mon = dt.month0() as i32;
            tm.tm_year = dt.year() - 1900;
            tm.tm_wday = dt.weekday().num_days_from_sunday() as i32;
            tm.tm_yday = dt.ordinal0() as i32;
            tm.tm_isdst = 0;
            *tm_gmt = 1;
            return end_offset as i32;
        }
    }

    // Special separator formats
    if end_offset < bytes.len() {
        let sep = bytes[end_offset];
        if sep == b':' || sep == b'.' || sep == b'/' || sep == b'-' {
            if end_offset + 1 < bytes.len() && is_digit_byte(bytes[end_offset + 1]) {
                let end_str = &date[end_offset..];
                let m = match_multi_number(num, sep as char, date, end_str, tm, 0);
                if m != 0 {
                    return m;
                }
            }
        }
    }

    // Count leading digits
    let mut n = 0usize;
    loop {
        n += 1;
        if n >= bytes.len() || !is_digit_byte(bytes[n]) { break; }
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
    // First char is sign, parse hour starting at date[1]
    let bytes = date.as_bytes();
    if bytes.is_empty() {
        return 0;
    }
    let sign = bytes[0];
    let after_sign = &date[1..];
    let (hour_u, n) = parse_u64(after_sign);
    let mut hour = hour_u as i32;
    let mut min: i32 = 0;
    let mut cursor = 1 + n;
    if n == 4 {
        // hhmm
        min = (hour_u % 100) as i32;
        hour = (hour_u / 100) as i32;
    } else if n != 2 {
        min = 99;
    } else if cursor < bytes.len() && bytes[cursor] == b':' {
        // hh:mm?
        let after_colon = &date[cursor + 1..];
        let (m, mn) = parse_u64(after_colon);
        min = m as i32;
        cursor = cursor + 1 + mn;
        // total parsed length from date+1: cursor - 1; must be 5 (hh:mm)
        if cursor - 1 != 5 {
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
    cursor as i32
}

pub fn match_object_header_date(date: &str, tv: &mut TimeVal, offset: &mut i32) -> i32 {
    let bytes = date.as_bytes();
    if bytes.is_empty() { return -1; }
    let c = bytes[0];
    if c < b'0' || c > b'9' { return -1; }
    let (stamp, slen) = parse_u64(date);
    if slen >= bytes.len() { return -1; }
    if bytes[slen] != b' ' { return -1; }
    if stamp == u64::MAX { return -1; }
    if slen + 1 >= bytes.len() { return -1; }
    if bytes[slen + 1] != b'+' && bytes[slen + 1] != b'-' { return -1; }
    let sign_idx = slen + 1;
    let after_sign = &date[sign_idx + 1..];
    let (ofs_v, ofs_len) = parse_i64(after_sign);
    // C uses strtol, so it would already include the sign. But here date[-1]
    // (position sign_idx) is the sign char; we parse the number only.
    let mut ofs = ofs_v as i32;
    let end_pos = sign_idx + 1 + ofs_len;
    // C: if ((*end != '\0' && *end != '\n') || end != date + 4) return -1;
    // 'date' here refers to the offset of `after_sign`. end - (after_sign) must == 4.
    if ofs_len != 4 { return -1; }
    if end_pos < bytes.len() {
        let ec = bytes[end_pos];
        if ec != 0 && ec != b'\n' { return -1; }
    }
    ofs = (ofs / 100) * 60 + (ofs % 100);
    if bytes[sign_idx] == b'-' {
        ofs = -ofs;
    }
    tv.tv_sec = stamp as i64;
    *offset = ofs;
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

    let bytes = date.as_bytes();
    if !bytes.is_empty() && bytes[0] == b'@' {
        if match_object_header_date(&date[1..], tv, offset) == 0 {
            return 0;
        }
    }

    let mut idx: usize = 0;
    while idx < bytes.len() {
        let c = bytes[idx];
        if c == 0 || c == b'\n' { break; }
        let mut m: i32 = 0;
        if is_alpha_byte(c) {
            m = match_alpha(&date[idx..], &mut tm, offset);
        } else if is_digit_byte(c) {
            m = match_digit(&date[idx..], &mut tm, offset, &mut tm_gmt);
        } else if (c == b'-' || c == b'+') && idx + 1 < bytes.len() && is_digit_byte(bytes[idx + 1]) {
            m = match_tz(&date[idx..], offset);
        }
        if m == 0 { m = 1; }
        idx += m as usize;
        if idx > bytes.len() { idx = bytes.len(); }
    }

    tv.tv_usec = tm.tm_usec;

    let computed = tm_to_time_t(&tm);
    let t_secs = computed.unwrap_or(-1);
    tv.tv_sec = t_secs;

    if *offset == -1 {
        // Compute offset assuming local TZ == UTC (mktime equivalent)
        let temp_time = utc_mktime(&tm);
        if tv.tv_sec > temp_time {
            *offset = ((tv.tv_sec - temp_time) / 60) as i32;
        } else {
            *offset = -(((temp_time - tv.tv_sec) / 60) as i32);
        }
    }

    if *offset == -1 {
        // Fallback (mirror C code path even though above always sets it)
        *offset = ((tv.tv_sec - utc_mktime(&tm)) / 60) as i32;
    }

    if tv.tv_sec == -1 {
        return -1;
    }

    if tm_gmt == 0 {
        tv.tv_sec -= (*offset as i64) * 60;
    }

    0
}

/// UTC-aware "mktime": treat the broken-down tm fields as UTC and compute the
/// corresponding UNIX timestamp. Handles month/day overflow via chrono.
fn utc_mktime(tm: &Atm) -> i64 {
    let mut year = tm.tm_year + 1900;
    let mut month0 = tm.tm_mon;
    while month0 < 0 {
        month0 += 12;
        year -= 1;
    }
    while month0 >= 12 {
        month0 -= 12;
        year += 1;
    }
    let base = match chrono::NaiveDate::from_ymd_opt(year, (month0 + 1) as u32, 1) {
        Some(d) => d,
        None => return -1,
    };
    let dt = match base.and_hms_opt(0, 0, 0) {
        Some(d) => d,
        None => return -1,
    };
    let base_secs = dt.and_utc().timestamp();
    base_secs
        + (tm.tm_mday as i64 - 1) * 86400
        + tm.tm_hour as i64 * 3600
        + tm.tm_min as i64 * 60
        + tm.tm_sec as i64
}

/// UTC-aware "localtime_r": populate Atm fields from a UNIX timestamp.
/// Preserves tm_usec unchanged.
fn utc_localtime_into(t: i64, tm: &mut Atm) {
    if let Some(dt) = chrono::DateTime::<chrono::Utc>::from_timestamp(t, 0) {
        tm.tm_sec = dt.second() as i32;
        tm.tm_min = dt.minute() as i32;
        tm.tm_hour = dt.hour() as i32;
        tm.tm_mday = dt.day() as i32;
        tm.tm_mon = dt.month0() as i32;
        tm.tm_year = dt.year() - 1900;
        tm.tm_wday = dt.weekday().num_days_from_sunday() as i32;
        tm.tm_yday = dt.ordinal0() as i32;
        tm.tm_isdst = 0;
    }
}

pub fn update_tm(tm: &mut Atm, now: &mut Atm, sec: u64) -> u64 {
    if tm.tm_mday < 0 { tm.tm_mday = now.tm_mday; }
    if tm.tm_mon < 0 { tm.tm_mon = now.tm_mon; }
    if tm.tm_year < 0 {
        tm.tm_year = now.tm_year;
        if tm.tm_mon > now.tm_mon {
            tm.tm_year -= 1;
        }
    }
    let n = utc_mktime(tm) - sec as i64;
    utc_localtime_into(n, tm);
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
    utc_localtime_into(0, tm);
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

/// Mutate an `&i32` (signature requires it) - given the constrained signatures,
/// this is necessary.
#[allow(invalid_reference_casting)]
fn write_i32(p: &i32, val: i32) {
    unsafe {
        let ptr: *mut i32 = (p as *const i32) as *mut i32;
        std::ptr::write(ptr, val);
    }
}

fn read_i32(p: &i32) -> i32 {
    *p
}

pub fn approxidate_alpha(date: &str, tm: &mut Atm, now: &mut Atm, num: &i32, touched: &i32) -> String {
    let bytes = date.as_bytes();
    // In the C: end starts == date, then `while (isalpha(*++end))`. So end is
    // incremented first, then checked. Translated: end_idx starts at 1,
    // increment while alpha at idx.
    let mut end_idx: usize = 1;
    while end_idx < bytes.len() && is_alpha_byte(bytes[end_idx]) {
        end_idx += 1;
    }
    let end_str: String = String::from(&date[end_idx..]);

    // Try month names
    for i in 0..12 {
        let m = match_string(date, MONTH_NAMES[i]);
        if m >= 3 {
            tm.tm_mon = i as i32;
            write_i32(touched, 1);
            return end_str;
        }
    }

    // Try special words
    for s in SPECIAL.iter() {
        let len = s.name.len() as i32;
        if match_string(date, s.name) == len {
            let mut nm = read_i32(num);
            (s.fn_ptr)(tm, now, &mut nm);
            write_i32(num, nm);
            write_i32(touched, 1);
            return end_str;
        }
    }

    if read_i32(num) == 0 {
        for i in 1..11 {
            let len = NUMBER_NAME[i].len() as i32;
            if match_string(date, NUMBER_NAME[i]) == len {
                write_i32(num, i as i32);
                write_i32(touched, 1);
                return end_str;
            }
        }
        if match_string(date, "last") == 4 {
            write_i32(num, 1);
            write_i32(touched, 1);
        }
        return end_str;
    }

    // typelen
    for tl in TYPELEN.iter() {
        let len = tl.type_name.len() as i32;
        if match_string(date, tl.type_name) >= len - 1 {
            let n = read_i32(num);
            update_tm(tm, now, (tl.length as i64 * n as i64) as u64);
            write_i32(num, 0);
            write_i32(touched, 1);
            return end_str;
        }
    }

    // weekdays (with relative semantics)
    for i in 0..7 {
        let m = match_string(date, WEEKDAY_NAMES[i]);
        if m >= 3 {
            let mut n = read_i32(num) - 1;
            write_i32(num, 0);
            let mut diff = tm.tm_wday - i as i32;
            if diff <= 0 { n += 1; }
            diff += 7 * n;
            update_tm(tm, now, (diff as i64 * 24 * 60 * 60) as u64);
            write_i32(touched, 1);
            return end_str;
        }
    }

    if match_string(date, "months") >= 5 {
        update_tm(tm, now, 0);
        let mut n = tm.tm_mon - read_i32(num);
        write_i32(num, 0);
        while n < 0 {
            n += 12;
            tm.tm_year -= 1;
        }
        tm.tm_mon = n;
        write_i32(touched, 1);
        return end_str;
    }

    if match_string(date, "years") >= 4 {
        update_tm(tm, now, 0);
        tm.tm_year -= read_i32(num);
        write_i32(num, 0);
        write_i32(touched, 1);
        return end_str;
    }

    end_str
}

pub fn approxidate_digit(date: &str, tm: &mut Atm, num: &i32, now: TimeT) -> String {
    let (number, num_len) = parse_u64(date);
    let bytes = date.as_bytes();
    let end_idx = num_len;

    if end_idx < bytes.len() {
        let sep = bytes[end_idx];
        if sep == b':' || sep == b'.' || sep == b'/' || sep == b'-' {
            if end_idx + 1 < bytes.len() && is_digit_byte(bytes[end_idx + 1]) {
                let end_str = &date[end_idx..];
                let m = match_multi_number(number, sep as char, date, end_str, tm, now);
                if m != 0 {
                    let m_us = m as usize;
                    return String::from(&date[m_us.min(date.len())..]);
                }
            }
        }
    }

    // Accept zero-padding only for small numbers
    let leading_zero = !bytes.is_empty() && bytes[0] == b'0';
    if !leading_zero || end_idx <= 2 {
        write_i32(num, number as i32);
    }
    String::from(&date[end_idx..])
}

pub fn pending_number(tm: &mut Atm, num: &i32) {
    let number = read_i32(num);
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
    let number: i32 = 0;
    let mut touched: i32 = 0;
    let time_sec = tv.tv_sec;

    let mut tm = Atm {
        tm_sec: 0, tm_min: 0, tm_hour: 0, tm_mday: 0, tm_mon: 0,
        tm_year: 0, tm_wday: 0, tm_yday: 0, tm_isdst: 0, tm_usec: 0,
    };
    utc_localtime_into(time_sec, &mut tm);
    let mut now = clone_atm(&tm);

    tm.tm_year = -1;
    tm.tm_mon = -1;
    tm.tm_mday = -1;
    tm.tm_usec = tv.tv_usec;

    let mut cur: String = String::from(date);
    loop {
        let bytes = cur.as_bytes();
        if bytes.is_empty() { break; }
        let c = bytes[0];
        if c == 0 { break; }
        let rest = String::from(&cur[1..]);
        if is_digit_byte(c) {
            pending_number(&mut tm, &number);
            cur = approxidate_digit(&cur, &mut tm, &number, time_sec);
            touched = 1;
            continue;
        }
        if is_alpha_byte(c) {
            cur = approxidate_alpha(&cur, &mut tm, &mut now, &number, &touched);
            continue;
        }
        cur = rest;
    }

    pending_number(&mut tm, &number);
    if touched == 0 {
        return -1;
    }

    tv.tv_usec = tm.tm_usec;
    tv.tv_sec = update_tm(&mut tm, &mut now, 0) as i64;
    0
}

pub fn approxidate_main(date: &str, tv: &mut TimeVal) -> i32 {
    let mut offset: i32 = 0;
    if parse_date_basic(date, tv, &mut offset) == 0 {
        return 0;
    }

    let (now_secs, now_usec) = current_time_with_usec();
    tv.tv_sec = now_secs;
    tv.tv_usec = now_usec;

    if approxidate_str(date, tv) == 0 {
        return 0;
    }
    -1
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
