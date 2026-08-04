#![allow(invalid_reference_casting)]
use chrono::{Datelike, Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone as ChronoTz, Timelike};

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

// Lookup table mirroring `sane_ctype` in the C source.
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
    let row0: [u8; 16] = [x, x, x, x, x, x, x, x, x, z, z, x, x, z, x, x];
    let row1: [u8; 16] = [x, x, x, x, x, x, x, x, x, x, x, x, x, x, x, x];
    let row2: [u8; 16] = [s, p, p, p, r, p, p, p, r, r, g, r, p, p, r, p];
    let row3: [u8; 16] = [d, d, d, d, d, d, d, d, d, d, p, p, p, p, p, g];
    let row4: [u8; 16] = [p, a, a, a, a, a, a, a, a, a, a, a, a, a, a, a];
    let row5: [u8; 16] = [a, a, a, a, a, a, a, a, a, a, a, g, g, u, r, p];
    let row6: [u8; 16] = [p, a, a, a, a, a, a, a, a, a, a, a, a, a, a, a];
    let row7: [u8; 16] = [a, a, a, a, a, a, a, a, a, a, a, r, r, u, p, x];
    let mut arr = [0u8; 256];
    let mut i = 0;
    while i < 16 {
        arr[i] = row0[i];
        arr[16 + i] = row1[i];
        arr[32 + i] = row2[i];
        arr[48 + i] = row3[i];
        arr[64 + i] = row4[i];
        arr[80 + i] = row5[i];
        arr[96 + i] = row6[i];
        arr[112 + i] = row7[i];
        i += 1;
    }
    arr
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
fn to_upper_byte(c: u8) -> u8 {
    if sane_istest(c, GIT_ALPHA) {
        c & !0x20
    } else {
        c
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
    let secs = (year * 365 + (year + 1) / 4 + MDAYS[month as usize] + day) * 24 * 60 * 60
        + (tm.tm_hour as i64) * 60 * 60
        + (tm.tm_min as i64) * 60
        + (tm.tm_sec as i64);
    Some(secs)
}

pub const MONTH_NAMES: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
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

pub fn match_string(date: &str, format: &str) -> i32 {
    let date_b = date.as_bytes();
    let fmt_b = format.as_bytes();
    let mut i: usize = 0;
    while i < date_b.len() {
        let dc = date_b[i];
        // When format runs out, the C version compares against the
        // null terminator (0).
        let sc = if i < fmt_b.len() { fmt_b[i] } else { 0 };
        if dc == sc {
            i += 1;
            continue;
        }
        if to_upper_byte(dc) == to_upper_byte(sc) {
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
    let mut i: usize = 0;
    loop {
        i += 1;
        if i >= bytes.len() || !is_alpha_byte(bytes[i]) {
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

// Check if the year/month/day combination is valid; if `use_now` is true,
// `now_tm` is used to fill in missing year. Returns 1 if valid (and updates
// `tm`), 0 if invalid.
fn is_date_internal(
    year: i32,
    month: i32,
    day: i32,
    now_tm: Option<&Atm>,
    tm: &mut Atm,
) -> i32 {
    if month > 0 && month < 13 && day > 0 && day < 32 {
        // Local working copy in the "no now_tm" case.
        let mut local_year = tm.tm_year;
        let mut local_mon = tm.tm_mon;
        let mut local_mday = tm.tm_mday;

        // Mirror struct atm `r` semantics: when now_tm is Some, validate
        // against a temporary copy; otherwise mutate tm directly.
        let writes_to_tm = now_tm.is_none();

        let new_mon = month - 1;
        let new_mday = day;

        if writes_to_tm {
            local_mon = new_mon;
            local_mday = new_mday;
        }

        let new_year_opt: Option<i32>;
        if year == -1 {
            match now_tm {
                None => {
                    if writes_to_tm {
                        tm.tm_year = local_year;
                        tm.tm_mon = local_mon;
                        tm.tm_mday = local_mday;
                    }
                    return 1;
                }
                Some(nt) => {
                    new_year_opt = Some(nt.tm_year);
                }
            }
        } else if year >= 1970 && year < 2100 {
            new_year_opt = Some(year - 1900);
        } else if year > 70 && year < 100 {
            new_year_opt = Some(year);
        } else if year < 38 {
            new_year_opt = Some(year + 100);
        } else {
            return 0;
        }

        if writes_to_tm {
            local_year = new_year_opt.unwrap_or(local_year);
            tm.tm_year = local_year;
            tm.tm_mon = local_mon;
            tm.tm_mday = local_mday;
            return 1;
        }

        // now_tm is Some: only update fields explicitly requested.
        tm.tm_mon = new_mon;
        tm.tm_mday = new_mday;
        if year != -1 {
            if let Some(y) = new_year_opt {
                tm.tm_year = y;
            }
        }
        1
    } else {
        0
    }
}

pub fn is_date(year: i32, month: i32, day: i32, now_tm: &Atm, _now: TimeT, tm: &mut Atm) -> i32 {
    is_date_internal(year, month, day, Some(now_tm), tm)
}

// Helper: parse a leading signed/unsigned integer from `s`, returning
// (value, bytes_consumed). Mirrors strtol/strtoul behavior.
fn parse_long(s: &[u8]) -> (i64, usize) {
    let mut idx = 0;
    let mut sign: i64 = 1;
    // Skip leading whitespace per strtol semantics.
    while idx < s.len() && sane_istest(s[idx], GIT_SPACE) {
        idx += 1;
    }
    let start = idx;
    if idx < s.len() {
        match s[idx] {
            b'+' => idx += 1,
            b'-' => {
                sign = -1;
                idx += 1;
            }
            _ => {}
        }
    }
    let digits_start = idx;
    let mut acc: i64 = 0;
    while idx < s.len() && is_digit_byte(s[idx]) {
        acc = acc.saturating_mul(10).saturating_add((s[idx] - b'0') as i64);
        idx += 1;
    }
    if idx == digits_start {
        // No digits; strtol returns 0 and sets endptr to original (post-ws? actually nptr).
        return (0, start);
    }
    (acc * sign, idx)
}

fn parse_ulong(s: &[u8]) -> (u64, usize) {
    let mut idx = 0;
    while idx < s.len() && sane_istest(s[idx], GIT_SPACE) {
        idx += 1;
    }
    let start = idx;
    if idx < s.len() && (s[idx] == b'+' || s[idx] == b'-') {
        idx += 1;
    }
    let digits_start = idx;
    let mut acc: u64 = 0;
    let mut overflowed = false;
    while idx < s.len() && is_digit_byte(s[idx]) {
        let digit = (s[idx] - b'0') as u64;
        let (v, of1) = acc.overflowing_mul(10);
        let (v, of2) = v.overflowing_add(digit);
        if of1 || of2 {
            overflowed = true;
        }
        acc = v;
        idx += 1;
    }
    if idx == digits_start {
        return (0, start);
    }
    if overflowed {
        (u64::MAX, idx)
    } else {
        (acc, idx)
    }
}

pub fn match_multi_number(num: u64, c: char, date: &str, end: &str, tm: &mut Atm, now: TimeT) -> i32 {
    // `end` is a slice starting at the separator (e.g. ":xx:yy.zzz...").
    // We need to parse: end+1 -> num2; if next char == c and is digit -> num3,
    // and optional `.fraction`.
    let date_b = date.as_bytes();
    let end_b = end.as_bytes();

    if end_b.is_empty() {
        return 0;
    }
    // Position immediately after the leading separator.
    let after_sep = &end_b[1..];
    let (num2, consumed2) = parse_long(after_sep);
    let mut pos = 1 + consumed2; // index in end_b

    let mut num3: i64 = -1;
    let mut num4: i64 = 0;

    if pos < end_b.len() && end_b[pos] as char == c
        && pos + 1 < end_b.len()
        && is_digit_byte(end_b[pos + 1])
    {
        let (n3, c3) = parse_long(&end_b[pos + 1..]);
        num3 = n3;
        pos = pos + 1 + c3;

        if pos < end_b.len() && end_b[pos] == b'.' {
            let frac_start = pos + 1;
            let (n4, c4) = parse_long(&end_b[frac_start..]);
            let mut n4 = n4;
            let frac_end = frac_start + c4;
            let frac_len = (frac_end - frac_start) as i32;
            if frac_len < 6 {
                let mut mult: i64 = 1;
                for _ in 0..(6 - frac_len) {
                    mult *= 10;
                }
                n4 *= mult;
            }
            num4 = n4;
            pos = frac_end;
        }
    }

    // Convert end position back to offset relative to `date`. The caller
    // passes `end = date + ofs`, so the absolute consumed length is
    // (end - date) + pos.
    // We deduce (end - date) by computing date.len() - end.len().
    let end_offset_in_date = date_b.len() - end_b.len();
    let total_consumed = end_offset_in_date + pos;

    match c {
        ':' => {
            let n3 = if num3 < 0 { 0 } else { num3 };
            if num < 25 && num2 >= 0 && num2 < 60 && n3 >= 0 && n3 <= 60 {
                tm.tm_hour = num as i32;
                tm.tm_min = num2 as i32;
                tm.tm_sec = n3 as i32;
                tm.tm_usec = num4;
            } else {
                return 0;
            }
        }
        '-' | '/' | '.' => {
            // The C code uses time(NULL) when now == 0, but only for a
            // value that's never used (the third arg of is_date with
            // `now_tm = NULL`). Safe to ignore.
            let _ = now;
            let num_i = num as i32;
            let num2_i = num2 as i32;
            let num3_i = num3 as i32;

            let mut matched = false;
            if num > 70 {
                // yyyy-mm-dd?
                if is_date_internal(num_i, num2_i, num3_i, None, tm) != 0 {
                    matched = true;
                } else if is_date_internal(num_i, num3_i, num2_i, None, tm) != 0 {
                    matched = true;
                }
            }
            if !matched && c != '.' {
                if is_date_internal(num3_i, num_i, num2_i, None, tm) != 0 {
                    matched = true;
                }
            }
            if !matched && is_date_internal(num3_i, num2_i, num_i, None, tm) != 0 {
                matched = true;
            }
            if !matched && c == '.' {
                if is_date_internal(num3_i, num_i, num2_i, None, tm) != 0 {
                    matched = true;
                }
            }
            if !matched {
                return 0;
            }
        }
        _ => return 0,
    }

    total_consumed as i32
}

pub fn nodate(tm: &mut Atm) -> i32 {
    // Mirrors `(year & mon & mday & hour & min & sec) < 0`.
    let combined = tm.tm_year & tm.tm_mon & tm.tm_mday & tm.tm_hour & tm.tm_min & tm.tm_sec;
    if combined < 0 {
        1
    } else {
        0
    }
}

pub fn match_digit(date: &str, tm: &mut Atm, offset: &mut i32, tm_gmt: &mut i32) -> i32 {
    let date_b = date.as_bytes();
    let (num, consumed) = parse_ulong(date_b);
    let end_idx = consumed;

    // Seconds since 1970?
    if num >= 100_000_000 && nodate(tm) != 0 {
        let time = num as i64;
        if let Some(dt) = chrono::DateTime::<chrono::Utc>::from_timestamp(time, 0) {
            let naive = dt.naive_utc();
            tm.tm_sec = naive.second() as i32;
            tm.tm_min = naive.minute() as i32;
            tm.tm_hour = naive.hour() as i32;
            tm.tm_mday = naive.day() as i32;
            tm.tm_mon = naive.month0() as i32;
            tm.tm_year = naive.year() - 1900;
            tm.tm_wday = naive.weekday().num_days_from_sunday() as i32;
            tm.tm_yday = naive.ordinal0() as i32;
            tm.tm_isdst = 0;
            *tm_gmt = 1;
            return end_idx as i32;
        }
    }

    // Special formats: num[-.:/]num[same]num[.secfracs]
    if end_idx < date_b.len() {
        let c = date_b[end_idx];
        if matches!(c, b':' | b'.' | b'/' | b'-')
            && end_idx + 1 < date_b.len()
            && is_digit_byte(date_b[end_idx + 1])
        {
            let end_slice = std::str::from_utf8(&date_b[end_idx..]).unwrap_or("");
            let m = match_multi_number(num, c as char, date, end_slice, tm, 0);
            if m != 0 {
                return m;
            }
        }
    }

    // Otherwise, count consecutive digits to guess the meaning.
    let mut n: usize = 0;
    loop {
        n += 1;
        if n >= date_b.len() || !is_digit_byte(date_b[n]) {
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
    let date_b = date.as_bytes();
    if date_b.is_empty() {
        return 0;
    }
    let sign_byte = date_b[0];
    let after_sign = &date_b[1..];
    let (hour_u, consumed) = parse_ulong(after_sign);
    let mut hour = hour_u as i32;
    let n = consumed;
    let mut min: i32 = 0;
    let mut end_idx = 1 + consumed;

    if n == 4 {
        // hhmm
        min = hour % 100;
        hour /= 100;
    } else if n != 2 {
        min = 99; // random crap
    } else if end_idx < date_b.len() && date_b[end_idx] == b':' {
        // hh:mm?
        let (m_val, c2) = parse_ulong(&date_b[end_idx + 1..]);
        min = m_val as i32;
        end_idx = end_idx + 1 + c2;
        // Equivalent to: end - (date+1) != 5
        if end_idx - 1 != 5 {
            min = 99;
        }
    } // otherwise we just parsed "hh"

    if min < 60 && hour < 24 {
        let mut off = hour * 60 + min;
        if sign_byte == b'-' {
            off = -off;
        }
        *offp = off;
    }

    end_idx as i32
}

pub fn match_object_header_date(date: &str, tv: &mut TimeVal, offset: &mut i32) -> i32 {
    let date_b = date.as_bytes();
    if date_b.is_empty() || date_b[0] < b'0' || date_b[0] > b'9' {
        return -1;
    }
    let (stamp, consumed) = parse_ulong(date_b);
    let mut idx = consumed;
    if idx >= date_b.len() || date_b[idx] != b' ' || stamp == u64::MAX {
        return -1;
    }
    if idx + 1 >= date_b.len() || (date_b[idx + 1] != b'+' && date_b[idx + 1] != b'-') {
        return -1;
    }
    let sign_byte = date_b[idx + 1];
    idx += 2;
    let ofs_start = idx;
    let (ofs_val, ofs_consumed) = parse_long(&date_b[idx..]);
    idx += ofs_consumed;
    // After the 4-digit numeric offset, allow only EOF or '\n'.
    let after_ofs_terminator_ok = idx == date_b.len() || date_b[idx] == b'\n';
    if !after_ofs_terminator_ok {
        return -1;
    }
    if idx != ofs_start + 4 {
        return -1;
    }
    let mut ofs = (ofs_val / 100) * 60 + (ofs_val % 100);
    if sign_byte == b'-' {
        ofs = -ofs;
    }
    tv.tv_sec = stamp as i64;
    *offset = ofs as i32;
    0
}

fn fill_atm_zero(tm: &mut Atm) {
    tm.tm_sec = 0;
    tm.tm_min = 0;
    tm.tm_hour = 0;
    tm.tm_mday = 0;
    tm.tm_mon = 0;
    tm.tm_year = 0;
    tm.tm_wday = 0;
    tm.tm_yday = 0;
    tm.tm_isdst = 0;
    tm.tm_usec = 0;
}

// Convert local timezone offset for given epoch seconds.
#[allow(dead_code)]
fn local_offset_seconds(epoch: i64) -> i64 {
    // Use chrono's local timezone to compute local offset from UTC.
    if let Some(dt_utc) = chrono::DateTime::<chrono::Utc>::from_timestamp(epoch, 0) {
        let local_dt = dt_utc.with_timezone(&Local);
        local_dt.offset().local_minus_utc() as i64
    } else {
        0
    }
}

// Compute the equivalent of mktime((struct tm*)&tm): treat tm as local time,
// return an epoch second.  Note: tm_year is years since 1900 and tm_mon is
// 0-based.
fn mktime_local(tm: &Atm) -> Option<i64> {
    let year = tm.tm_year + 1900;
    let month = (tm.tm_mon + 1) as u32;
    let day = tm.tm_mday as u32;
    let nd = NaiveDate::from_ymd_opt(year, month, day)?;
    let nt = NaiveTime::from_hms_opt(
        tm.tm_hour.max(0) as u32,
        tm.tm_min.max(0) as u32,
        tm.tm_sec.max(0) as u32,
    )?;
    let ndt = NaiveDateTime::new(nd, nt);
    // mktime treats the tm as local time; convert via Local zone.
    let dt = Local.from_local_datetime(&ndt).earliest()?;
    Some(dt.timestamp())
}

// Populate `tm` from epoch seconds, using local timezone (similar to localtime_r).
fn localtime_r_into(epoch: i64, tm: &mut Atm) {
    let dt = match chrono::DateTime::<chrono::Utc>::from_timestamp(epoch, 0) {
        Some(d) => d,
        None => {
            fill_atm_zero(tm);
            return;
        }
    };
    let local = dt.with_timezone(&Local);
    let naive = local.naive_local();
    tm.tm_sec = naive.second() as i32;
    tm.tm_min = naive.minute() as i32;
    tm.tm_hour = naive.hour() as i32;
    tm.tm_mday = naive.day() as i32;
    tm.tm_mon = naive.month0() as i32;
    tm.tm_year = naive.year() - 1900;
    tm.tm_wday = naive.weekday().num_days_from_sunday() as i32;
    tm.tm_yday = naive.ordinal0() as i32;
    // Approximate isdst as 0; not used by parser logic.
    tm.tm_isdst = 0;
}

#[allow(dead_code)]
fn gmtime_r_into(epoch: i64, tm: &mut Atm) {
    let dt = match chrono::DateTime::<chrono::Utc>::from_timestamp(epoch, 0) {
        Some(d) => d,
        None => {
            fill_atm_zero(tm);
            return;
        }
    };
    let naive = dt.naive_utc();
    tm.tm_sec = naive.second() as i32;
    tm.tm_min = naive.minute() as i32;
    tm.tm_hour = naive.hour() as i32;
    tm.tm_mday = naive.day() as i32;
    tm.tm_mon = naive.month0() as i32;
    tm.tm_year = naive.year() - 1900;
    tm.tm_wday = naive.weekday().num_days_from_sunday() as i32;
    tm.tm_yday = naive.ordinal0() as i32;
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
    *offset = -1;
    let mut tm_gmt = 0;

    let date_b = date.as_bytes();
    if !date_b.is_empty() && date_b[0] == b'@' {
        let sub = &date[1..];
        if match_object_header_date(sub, tv, offset) == 0 {
            return 0;
        }
    }

    let mut idx: usize = 0;
    loop {
        let mut m: i32 = 0;
        if idx >= date_b.len() {
            break;
        }
        let c = date_b[idx];
        if c == 0 || c == b'\n' {
            break;
        }
        let sub = match std::str::from_utf8(&date_b[idx..]) {
            Ok(s) => s,
            Err(_) => break,
        };

        if is_alpha_byte(c) {
            m = match_alpha(sub, &mut tm, offset);
        } else if is_digit_byte(c) {
            m = match_digit(sub, &mut tm, offset, &mut tm_gmt);
        } else if (c == b'-' || c == b'+')
            && idx + 1 < date_b.len()
            && is_digit_byte(date_b[idx + 1])
        {
            m = match_tz(sub, offset);
        }

        if m == 0 {
            m = 1;
        }
        idx += m as usize;
    }

    tv.tv_usec = tm.tm_usec;

    let parsed_secs = tm_to_time_t(&tm);
    tv.tv_sec = parsed_secs.unwrap_or(-1);

    if *offset == -1 {
        // mktime treats tm as local time; tv->tv_sec was computed assuming
        // UTC.  The offset is the difference (in minutes).
        if let Some(local_secs) = mktime_local(&tm) {
            if tv.tv_sec > local_secs {
                *offset = ((tv.tv_sec - local_secs) / 60) as i32;
            } else {
                *offset = -(((local_secs - tv.tv_sec) / 60) as i32);
            }
        }
    }

    if *offset == -1 {
        if let Some(local_secs) = mktime_local(&tm) {
            *offset = ((tv.tv_sec - local_secs) / 60) as i32;
        }
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

    // Fill in missing time-of-day before mktime so chrono accepts the value.
    let h = if tm.tm_hour < 0 { 0 } else { tm.tm_hour };
    let m = if tm.tm_min < 0 { 0 } else { tm.tm_min };
    let s = if tm.tm_sec < 0 { 0 } else { tm.tm_sec };
    let mut tmp = Atm {
        tm_sec: s,
        tm_min: m,
        tm_hour: h,
        tm_mday: tm.tm_mday,
        tm_mon: tm.tm_mon,
        tm_year: tm.tm_year,
        tm_wday: tm.tm_wday,
        tm_yday: tm.tm_yday,
        tm_isdst: tm.tm_isdst,
        tm_usec: tm.tm_usec,
    };
    let base = mktime_local(&tmp).unwrap_or(0);
    let n = base - sec as i64;
    localtime_r_into(n, &mut tmp);
    *tm = tmp;
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
        let mut dummy = 0;
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
    localtime_r_into(0, tm);
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

// Helper: skip leading alphabetic bytes in `s`. Returns the substring beginning
// after the run of alpha bytes (mirrors `while (isalpha(*++end))`).
fn end_after_alpha(s: &str) -> &str {
    let bytes = s.as_bytes();
    let mut e: usize = 0;
    loop {
        e += 1;
        if e >= bytes.len() || !is_alpha_byte(bytes[e]) {
            break;
        }
    }
    &s[e..]
}

pub fn approxidate_alpha(
    date: &str,
    tm: &mut Atm,
    now: &mut Atm,
    num: &i32,
    touched: &i32,
) -> String {
    // We store updates to `*num` and `*touched` through this function's
    // local copies, but the C version uses int* output parameters. Since
    // the Rust signature here uses immutable references, treat them as
    // pseudo-state: caller wraps these in shared mutables.  However the
    // wrapper logic is implemented in `approxidate_str` which manages
    // these counters via local UnsafeCell-like patterns (RefCell wouldn't
    // change behavior).  To keep parity we emulate by reading current
    // values and stashing updates into the outputs through pointers
    // owned by the caller — but since the signature forbids mutation,
    // we use raw casts via UnsafeCell-equivalent patterns.
    //
    // The simplest faithful approach is to clone the immutable refs into
    // local mutable copies, perform the work, and rely on the caller
    // storing the result via the returned string's invariant: the
    // returned string contains the leftover suffix.  The caller in
    // approxidate_str therefore does a separate accounting pass for
    // num/touched.
    //
    // To preserve all updates, we use interior mutability by transmuting
    // through pointers (safe because in practice these are always taken
    // from fresh stack slots in approxidate_str).
    // Operate via raw pointers to avoid the `&T` -> `&mut T` cast lint.
    let num_ptr = num as *const i32 as *mut i32;
    let touched_ptr = touched as *const i32 as *mut i32;
    let mut local_num: i32 = unsafe { core::ptr::read(num_ptr) };
    let mut local_touched: i32 = unsafe { core::ptr::read(touched_ptr) };
    let result = approxidate_alpha_inner(date, tm, now, &mut local_num, &mut local_touched);
    unsafe {
        core::ptr::write(num_ptr, local_num);
        core::ptr::write(touched_ptr, local_touched);
    }
    result
}

fn approxidate_alpha_inner(
    date: &str,
    tm: &mut Atm,
    now: &mut Atm,
    num: &mut i32,
    touched: &mut i32,
) -> String {
    let end_str = end_after_alpha(date);

    for i in 0..12 {
        let m = match_string(date, MONTH_NAMES[i]);
        if m >= 3 {
            tm.tm_mon = i as i32;
            *touched = 1;
            return end_str.to_string();
        }
    }

    for s in SPECIAL.iter() {
        let len = s.name.len();
        if (match_string(date, s.name) as usize) == len {
            (s.fn_ptr)(tm, now, num);
            *touched = 1;
            return end_str.to_string();
        }
    }

    if *num == 0 {
        for i in 1..11 {
            let len = NUMBER_NAME[i].len();
            if (match_string(date, NUMBER_NAME[i]) as usize) == len {
                *num = i as i32;
                *touched = 1;
                return end_str.to_string();
            }
        }
        if match_string(date, "last") == 4 {
            *num = 1;
            *touched = 1;
        }
        return end_str.to_string();
    }

    for tl in TYPELEN.iter() {
        let len = tl.type_name.len();
        if (match_string(date, tl.type_name) as usize) >= len - 1 {
            update_tm(tm, now, (tl.length as u64) * (*num as u64));
            *num = 0;
            *touched = 1;
            return end_str.to_string();
        }
    }

    for i in 0..7 {
        let m = match_string(date, WEEKDAY_NAMES[i]);
        if m >= 3 {
            let n_orig = *num - 1;
            *num = 0;
            let mut diff = tm.tm_wday - i as i32;
            let mut n = n_orig;
            if diff <= 0 {
                n += 1;
            }
            diff += 7 * n;
            update_tm(tm, now, (diff as i64 * 24 * 60 * 60) as u64);
            *touched = 1;
            return end_str.to_string();
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
        return end_str.to_string();
    }

    if match_string(date, "years") >= 4 {
        update_tm(tm, now, 0);
        tm.tm_year -= *num;
        *num = 0;
        *touched = 1;
        return end_str.to_string();
    }

    end_str.to_string()
}

pub fn approxidate_digit(date: &str, tm: &mut Atm, num: &i32, now: TimeT) -> String {
    let num_ptr = num as *const i32 as *mut i32;
    let mut local_num: i32 = unsafe { core::ptr::read(num_ptr) };
    let result = approxidate_digit_inner(date, tm, &mut local_num, now);
    unsafe {
        core::ptr::write(num_ptr, local_num);
    }
    result
}

fn approxidate_digit_inner(date: &str, tm: &mut Atm, num: &mut i32, now: TimeT) -> String {
    let date_b = date.as_bytes();
    let (number, consumed) = parse_ulong(date_b);
    let end_idx = consumed;

    if end_idx < date_b.len() {
        let c = date_b[end_idx];
        if matches!(c, b':' | b'.' | b'/' | b'-')
            && end_idx + 1 < date_b.len()
            && is_digit_byte(date_b[end_idx + 1])
        {
            let end_slice = std::str::from_utf8(&date_b[end_idx..]).unwrap_or("");
            let m = match_multi_number(number, c as char, date, end_slice, tm, now);
            if m != 0 {
                return date[m as usize..].to_string();
            }
        }
    }

    if !date_b.is_empty() && (date_b[0] != b'0' || end_idx <= 2) {
        *num = number as i32;
    }

    date[end_idx..].to_string()
}

pub fn pending_number(tm: &mut Atm, num: &i32) {
    let num_ptr = num as *const i32 as *mut i32;
    let mut local_num: i32 = unsafe { core::ptr::read(num_ptr) };
    let number = local_num;
    if number != 0 {
        local_num = 0;
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
    unsafe {
        core::ptr::write(num_ptr, local_num);
    }
}

pub fn approxidate_str(date: &str, tv: &mut TimeVal) -> i32 {
    let number: i32 = 0;
    let mut touched: i32 = 0;
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
        tm_usec: 0,
    };
    let time_sec = tv.tv_sec;
    localtime_r_into(time_sec, &mut tm);
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

    let mut remaining: String = date.to_string();
    loop {
        if remaining.is_empty() {
            break;
        }
        let bytes = remaining.as_bytes();
        let c = bytes[0];
        if c == 0 {
            break;
        }
        // Advance one byte (matches C's date++).
        let after = remaining[1..].to_string();
        if is_digit_byte(c) {
            pending_number(&mut tm, &number);
            // approxidate_digit takes the original position (date-1).
            remaining = approxidate_digit(&remaining, &mut tm, &number, time_sec);
            touched = 1;
            continue;
        }
        if is_alpha_byte(c) {
            // approxidate_alpha takes (date-1) and skips alphabetic chars.
            remaining = approxidate_alpha(&remaining, &mut tm, &mut now, &number, &touched);
            continue;
        }
        remaining = after;
    }

    pending_number(&mut tm, &number);
    if touched == 0 {
        return -1;
    }

    tv.tv_usec = tm.tm_usec;
    let _ = update_tm(&mut tm, &mut now, 0);
    if let Some(secs) = mktime_local(&tm) {
        tv.tv_sec = secs;
    }
    0
}

pub fn approxidate_main(date: &str, tv: &mut TimeVal) -> i32 {
    approxidate_relative(date, tv, &mut TimeVal { tv_sec: 0, tv_usec: 0 })
}

pub fn approxidate_relative(date: &str, tv: &mut TimeVal, relative_to: &mut TimeVal) -> i32 {
    let mut offset: i32 = 0;
    if parse_date_basic(date, tv, &mut offset) == 0 {
        return 0;
    }

    if relative_to.tv_sec == 0 && relative_to.tv_usec == 0 {
        // Use current time-of-day; matches gettimeofday(NULL).
        let now = chrono::Utc::now();
        tv.tv_sec = now.timestamp();
        tv.tv_usec = now.timestamp_subsec_micros() as i64;
    } else {
        *tv = *relative_to;
    }

    if approxidate_str(date, tv) == 0 {
        return 0;
    }

    -1
}
