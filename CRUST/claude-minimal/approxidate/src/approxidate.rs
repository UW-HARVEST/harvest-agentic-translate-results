use chrono::TimeZone as ChronoTimeZone;
use chrono::{Datelike, Local, NaiveDate, Timelike, Utc};

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

#[allow(dead_code)]
const GIT_SPACE: u8 = 0x01;
#[allow(dead_code)]
const GIT_DIGIT: u8 = 0x02;
#[allow(dead_code)]
const GIT_ALPHA: u8 = 0x04;
#[allow(dead_code)]
const GIT_GLOB_SPECIAL: u8 = 0x08;
#[allow(dead_code)]
const GIT_REGEX_SPECIAL: u8 = 0x10;
#[allow(dead_code)]
const GIT_PATHSPEC_MAGIC: u8 = 0x20;
#[allow(dead_code)]
const GIT_CNTRL: u8 = 0x40;
#[allow(dead_code)]
const GIT_PUNCT: u8 = 0x80;

// ----- Internal helpers (not part of the documented API) -----

fn empty_tm() -> Atm {
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

/// Parse an unsigned integer starting at `start` in `bytes`. Returns the value
/// and the index just past the last digit consumed.
fn strtoul(bytes: &[u8], start: usize) -> (u64, usize) {
    let mut i = start;
    let mut num: u64 = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        num = num.wrapping_mul(10).wrapping_add((bytes[i] - b'0') as u64);
        i += 1;
    }
    (num, i)
}

/// Parse a (possibly signed) integer starting at `start` in `bytes`.
fn strtol(bytes: &[u8], start: usize) -> (i64, usize) {
    let mut i = start;
    let mut sign: i64 = 1;
    if i < bytes.len() {
        if bytes[i] == b'-' {
            sign = -1;
            i += 1;
        } else if bytes[i] == b'+' {
            i += 1;
        }
    }
    let mut num: i64 = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        num = num.wrapping_mul(10).wrapping_add((bytes[i] - b'0') as i64);
        i += 1;
    }
    (sign.wrapping_mul(num), i)
}

/// Fill `tm` from a Unix timestamp interpreted as UTC.
fn gmtime(tm: &mut Atm, sec: i64) -> bool {
    if let Some(dt) = Utc.timestamp_opt(sec, 0).single() {
        tm.tm_sec = dt.second() as i32;
        tm.tm_min = dt.minute() as i32;
        tm.tm_hour = dt.hour() as i32;
        tm.tm_mday = dt.day() as i32;
        tm.tm_mon = dt.month() as i32 - 1;
        tm.tm_year = dt.year() - 1900;
        tm.tm_wday = dt.weekday().num_days_from_sunday() as i32;
        tm.tm_yday = dt.ordinal() as i32 - 1;
        tm.tm_isdst = 0;
        true
    } else {
        false
    }
}

/// Fill `tm` from a Unix timestamp interpreted as local time.
fn localtime(tm: &mut Atm, sec: i64) -> bool {
    if let Some(dt) = Local.timestamp_opt(sec, 0).single() {
        tm.tm_sec = dt.second() as i32;
        tm.tm_min = dt.minute() as i32;
        tm.tm_hour = dt.hour() as i32;
        tm.tm_mday = dt.day() as i32;
        tm.tm_mon = dt.month() as i32 - 1;
        tm.tm_year = dt.year() - 1900;
        tm.tm_wday = dt.weekday().num_days_from_sunday() as i32;
        tm.tm_yday = dt.ordinal() as i32 - 1;
        tm.tm_isdst = 0;
        true
    } else {
        false
    }
}

/// Like C's mktime: interprets `tm` as local time and returns the Unix
/// timestamp, normalizing out-of-range fields.
fn mktime(tm: &Atm) -> i64 {
    // Normalize seconds, minutes, hours, days, months.
    let mut sec = tm.tm_sec as i64;
    let mut min = tm.tm_min as i64;
    let mut hour = tm.tm_hour as i64;
    let mut day = tm.tm_mday as i64;
    let mut month = tm.tm_mon as i64; // 0-11 ideally
    let mut year = tm.tm_year as i64 + 1900;

    let extra = sec.div_euclid(60);
    sec = sec.rem_euclid(60);
    min += extra;
    let extra = min.div_euclid(60);
    min = min.rem_euclid(60);
    hour += extra;
    let extra = hour.div_euclid(24);
    hour = hour.rem_euclid(24);
    day += extra;
    let extra = month.div_euclid(12);
    month = month.rem_euclid(12);
    year += extra;

    // year/month are now valid; day may be out-of-range.
    let base = match NaiveDate::from_ymd_opt(year as i32, (month + 1) as u32, 1) {
        Some(d) => d,
        None => return 0,
    };
    let date = match base.checked_add_signed(chrono::Duration::days(day - 1)) {
        Some(d) => d,
        None => return 0,
    };
    let dt = match date.and_hms_opt(hour as u32, min as u32, sec as u32) {
        Some(d) => d,
        None => return 0,
    };
    match Local.from_local_datetime(&dt) {
        chrono::LocalResult::Single(d) => d.timestamp(),
        chrono::LocalResult::Ambiguous(d, _) => d.timestamp(),
        chrono::LocalResult::None => 0,
    }
}

// ----- Static tables (unchanged) -----

pub fn tm_to_time_t(tm: &Atm) -> Option<i64> {
    const MDAYS: [i64; 12] = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];
    let year = tm.tm_year as i64 - 70;
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
            + tm.tm_hour as i64 * 60 * 60
            + tm.tm_min as i64 * 60
            + tm.tm_sec as i64,
    )
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

pub struct TimeZoneEntry {
    pub name: &'static str,
    pub offset: i32,
    pub dst: bool,
}

// Note: type alias to avoid name conflict with chrono::TimeZone trait.
pub type TimeZoneT = TimeZoneEntry;

// Re-export under the original name `TimeZone` if needed by external callers.
pub use TimeZoneEntry as TimeZone;

pub const TIMEZONE_NAMES: [TimeZoneEntry; 44] = [
    TimeZoneEntry { name: "IDLW", offset: -12, dst: false },
    TimeZoneEntry { name: "NT",   offset: -11, dst: false },
    TimeZoneEntry { name: "CAT",  offset: -10, dst: false },
    TimeZoneEntry { name: "HST",  offset: -10, dst: false },
    TimeZoneEntry { name: "HDT",  offset: -10, dst: true  },
    TimeZoneEntry { name: "YST",  offset: -9,  dst: false },
    TimeZoneEntry { name: "YDT",  offset: -9,  dst: true  },
    TimeZoneEntry { name: "PST",  offset: -8,  dst: false },
    TimeZoneEntry { name: "PDT",  offset: -8,  dst: true  },
    TimeZoneEntry { name: "MST",  offset: -7,  dst: false },
    TimeZoneEntry { name: "MDT",  offset: -7,  dst: true  },
    TimeZoneEntry { name: "CST",  offset: -6,  dst: false },
    TimeZoneEntry { name: "CDT",  offset: -6,  dst: true  },
    TimeZoneEntry { name: "EST",  offset: -5,  dst: false },
    TimeZoneEntry { name: "EDT",  offset: -5,  dst: true  },
    TimeZoneEntry { name: "AST",  offset: -3,  dst: false },
    TimeZoneEntry { name: "ADT",  offset: -3,  dst: true  },
    TimeZoneEntry { name: "WAT",  offset: -1,  dst: false },
    TimeZoneEntry { name: "GMT",  offset: 0,   dst: false },
    TimeZoneEntry { name: "UTC",  offset: 0,   dst: false },
    TimeZoneEntry { name: "Z",    offset: 0,   dst: false },
    TimeZoneEntry { name: "WET",  offset: 0,   dst: false },
    TimeZoneEntry { name: "BST",  offset: 0,   dst: true  },
    TimeZoneEntry { name: "CET",  offset: 1,   dst: false },
    TimeZoneEntry { name: "MET",  offset: 1,   dst: false },
    TimeZoneEntry { name: "MEWT", offset: 1,   dst: false },
    TimeZoneEntry { name: "MEST", offset: 1,   dst: true  },
    TimeZoneEntry { name: "CEST", offset: 1,   dst: true  },
    TimeZoneEntry { name: "MESZ", offset: 1,   dst: true  },
    TimeZoneEntry { name: "FWT",  offset: 1,   dst: false },
    TimeZoneEntry { name: "FST",  offset: 1,   dst: true  },
    TimeZoneEntry { name: "EET",  offset: 2,   dst: false },
    TimeZoneEntry { name: "EEST", offset: 2,   dst: true  },
    TimeZoneEntry { name: "WAST", offset: 7,   dst: false },
    TimeZoneEntry { name: "WADT", offset: 7,   dst: true  },
    TimeZoneEntry { name: "CCT",  offset: 8,   dst: false },
    TimeZoneEntry { name: "JST",  offset: 9,   dst: false },
    TimeZoneEntry { name: "EAST", offset: 10,  dst: false },
    TimeZoneEntry { name: "EADT", offset: 10,  dst: true  },
    TimeZoneEntry { name: "GST",  offset: 10,  dst: false },
    TimeZoneEntry { name: "NZT",  offset: 12,  dst: false },
    TimeZoneEntry { name: "NZST", offset: 12,  dst: false },
    TimeZoneEntry { name: "NZDT", offset: 12,  dst: true  },
    TimeZoneEntry { name: "IDLE", offset: 12,  dst: false },
];

pub fn match_string(date: &str, format: &str) -> i32 {
    let dbytes = date.as_bytes();
    let fbytes = format.as_bytes();
    let mut i: usize = 0;
    while i < dbytes.len() {
        let dc = dbytes[i];
        if dc == 0 {
            break;
        }
        let fc = if i < fbytes.len() { fbytes[i] } else { 0 };
        if dc == fc {
            // matches exactly
        } else if dc.to_ascii_uppercase() == fc.to_ascii_uppercase() {
            // matches case-insensitively
        } else if !dc.is_ascii_alphanumeric() {
            break;
        } else {
            return 0;
        }
        i += 1;
    }
    i as i32
}

pub fn skip_alpha(date: &str) -> i32 {
    let bytes = date.as_bytes();
    let mut i: usize = 1;
    while i < bytes.len() && bytes[i].is_ascii_alphabetic() {
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
        if m >= 3 || (m as usize) == tz.name.len() {
            let mut off = tz.offset;
            // "This is bogus, but we like summer"
            if tz.dst {
                off += 1;
            }
            // Only set offset if no better one yet.
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

pub fn is_date(
    year: i32,
    month: i32,
    day: i32,
    _now_tm: &Atm,
    _now: TimeT,
    tm: &mut Atm,
) -> i32 {
    // The C code only ever calls is_date with NULL for now_tm (from
    // match_multi_number).  Mirror that NULL branch here.
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

pub fn match_multi_number(
    num: u64,
    c: char,
    date: &str,
    end: &str,
    tm: &mut Atm,
    now: TimeT,
) -> i32 {
    let dbytes = date.as_bytes();
    // `end` is a tail of `date`, so its starting position within `date` is:
    let mut pos = dbytes.len() - end.as_bytes().len();
    debug_assert!(pos < dbytes.len());

    // Skip the separator character we just looked at.
    pos += 1;

    let (n2_signed, np) = strtol(dbytes, pos);
    let num2: i64 = n2_signed;
    pos = np;

    let mut num3: i64 = -1;
    let mut num4: i64 = 0;

    if pos < dbytes.len()
        && dbytes[pos] as char == c
        && pos + 1 < dbytes.len()
        && dbytes[pos + 1].is_ascii_digit()
    {
        let (n3_signed, np) = strtol(dbytes, pos + 1);
        num3 = n3_signed;
        pos = np;

        if pos < dbytes.len() && dbytes[pos] == b'.' {
            let start = pos + 1;
            let (n4_signed, np) = strtol(dbytes, start);
            num4 = n4_signed;
            pos = np;
            let consumed = pos - start;
            if consumed < 6 {
                num4 *= 10i64.pow((6 - consumed) as u32);
            }
        }
    }

    let dummy = empty_tm();

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
            let mut handled = false;
            let now_eff: TimeT = if now != 0 {
                now
            } else {
                // C uses time(NULL) here; we don't actually use this when
                // is_date receives NULL for now_tm, but we keep the value.
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0)
            };

            if num > 70 {
                // yyyy-mm-dd?
                if is_date(num as i32, num2 as i32, num3 as i32, &dummy, now_eff, tm) != 0 {
                    handled = true;
                } else if is_date(num as i32, num3 as i32, num2 as i32, &dummy, now_eff, tm) != 0 {
                    // yyyy-dd-mm?
                    handled = true;
                }
            }
            // mm/dd/yy[yy] form (preferred when separator isn't '.')
            if !handled
                && c != '.'
                && is_date(num3 as i32, num as i32, num2 as i32, &dummy, now_eff, tm) != 0
            {
                handled = true;
            }
            // dd.mm.yy[yy] / dd/mm/yy[yy]
            if !handled
                && is_date(num3 as i32, num2 as i32, num as i32, &dummy, now_eff, tm) != 0
            {
                handled = true;
            }
            // Funny European mm.dd.yy
            if !handled
                && c == '.'
                && is_date(num3 as i32, num as i32, num2 as i32, &dummy, now_eff, tm) != 0
            {
                handled = true;
            }
            if !handled {
                return 0;
            }
        }
        _ => {}
    }

    pos as i32
}

pub fn nodate(tm: &mut Atm) -> i32 {
    if (tm.tm_year & tm.tm_mon & tm.tm_mday & tm.tm_hour & tm.tm_min & tm.tm_sec) < 0 {
        1
    } else {
        0
    }
}

pub fn match_digit(date: &str, tm: &mut Atm, offset: &mut i32, tm_gmt: &mut i32) -> i32 {
    let bytes = date.as_bytes();
    let (num, end_pos) = strtoul(bytes, 0);

    // Seconds since 1970? More than 8 digits and no other field set yet.
    if num >= 100_000_000 && nodate(tm) != 0 {
        if gmtime(tm, num as i64) {
            *tm_gmt = 1;
            return end_pos as i32;
        }
    }

    // Multi-number formats (num[-.:/]num...)
    if end_pos < bytes.len() {
        let c = bytes[end_pos];
        if c == b':' || c == b'.' || c == b'/' || c == b'-' {
            if end_pos + 1 < bytes.len() && bytes[end_pos + 1].is_ascii_digit() {
                let end_str = std::str::from_utf8(&bytes[end_pos..]).unwrap_or("");
                let m = match_multi_number(num, c as char, date, end_str, tm, 0);
                if m != 0 {
                    return m;
                }
            }
        }
    }

    // Count consecutive digits at the start.
    let mut n: usize = 0;
    while n < bytes.len() && bytes[n].is_ascii_digit() {
        n += 1;
    }

    // Four-digit year or a timezone?
    if n == 4 {
        if num <= 1400 && *offset == -1 {
            let minutes = (num % 100) as i32;
            let hours = (num / 100) as i32;
            *offset = hours * 60 + minutes;
        } else if num > 1900 && num < 2100 {
            tm.tm_year = num as i32 - 1900;
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
        tm.tm_mon = num as i32 - 1;
    }

    n as i32
}

pub fn match_tz(date: &str, offp: &mut i32) -> i32 {
    let bytes = date.as_bytes();
    let (h_raw, end_pos) = strtoul(bytes, 1);
    let mut hour: i64 = h_raw as i64;
    let n = end_pos - 1;
    let mut min: i64 = 0;
    let mut final_pos = end_pos;

    if n == 4 {
        // hhmm
        min = hour % 100;
        hour /= 100;
    } else if n != 2 {
        min = 99; // random crap
    } else if end_pos < bytes.len() && bytes[end_pos] == b':' {
        let (m, np) = strtoul(bytes, end_pos + 1);
        min = m as i64;
        final_pos = np;
        if final_pos - 1 != 5 {
            min = 99;
        }
    } /* otherwise we parsed "hh" */

    if min < 60 && hour < 24 {
        let mut offset = hour * 60 + min;
        if !bytes.is_empty() && bytes[0] == b'-' {
            offset = -offset;
        }
        *offp = offset as i32;
    }

    final_pos as i32
}

pub fn match_object_header_date(date: &str, tv: &mut TimeVal, offset: &mut i32) -> i32 {
    let bytes = date.as_bytes();
    if bytes.is_empty() || bytes[0] < b'0' || bytes[0] > b'9' {
        return -1;
    }
    let (stamp, end_pos) = strtoul(bytes, 0);
    if end_pos >= bytes.len() || bytes[end_pos] != b' ' {
        return -1;
    }
    if end_pos + 1 >= bytes.len()
        || (bytes[end_pos + 1] != b'+' && bytes[end_pos + 1] != b'-')
    {
        return -1;
    }
    let sign_byte = bytes[end_pos + 1];
    let after_sign = end_pos + 2;
    let (ofs_raw, end2) = strtol(bytes, after_sign);
    if end2 - after_sign != 4 {
        return -1;
    }
    if end2 < bytes.len() && bytes[end2] != 0 && bytes[end2] != b'\n' {
        return -1;
    }
    let mut ofs = (ofs_raw / 100) * 60 + (ofs_raw % 100);
    if sign_byte == b'-' {
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
    let mut tm_gmt: i32 = 0;
    *offset = -1;

    let bytes = date.as_bytes();

    if !bytes.is_empty() && bytes[0] == b'@' {
        let rest = &date[1..];
        if match_object_header_date(rest, tv, offset) == 0 {
            return 0;
        }
    }

    let mut pos: usize = 0;
    while pos < bytes.len() {
        let c = bytes[pos];
        if c == 0 || c == b'\n' {
            break;
        }
        let sub = match std::str::from_utf8(&bytes[pos..]) {
            Ok(s) => s,
            Err(_) => break,
        };
        let mut m: i32 = 0;
        if c.is_ascii_alphabetic() {
            m = match_alpha(sub, &mut tm, offset);
        } else if c.is_ascii_digit() {
            m = match_digit(sub, &mut tm, offset, &mut tm_gmt);
        } else if (c == b'-' || c == b'+')
            && pos + 1 < bytes.len()
            && bytes[pos + 1].is_ascii_digit()
        {
            m = match_tz(sub, offset);
        }
        if m == 0 {
            m = 1;
        }
        pos = pos.saturating_add(m as usize);
    }

    tv.tv_usec = tm.tm_usec;

    // mktime uses local timezone
    tv.tv_sec = tm_to_time_t(&tm).unwrap_or(-1);

    if *offset == -1 {
        let temp_time = mktime(&tm);
        if tv.tv_sec > temp_time {
            *offset = ((tv.tv_sec - temp_time) / 60) as i32;
        } else {
            *offset = -(((temp_time - tv.tv_sec) / 60) as i32);
        }
    }

    if *offset == -1 {
        *offset = ((tv.tv_sec - mktime(&tm)) / 60) as i32;
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

    let n = mktime(tm) - sec as i64;
    localtime(tm, n);
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
    localtime(tm, 0);
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

/// Returns the number of bytes consumed from the start of `date`.
pub fn approxidate_alpha(
    date: &str,
    tm: &mut Atm,
    now: &mut Atm,
    num: &mut i32,
    touched: &mut i32,
) -> usize {
    let bytes = date.as_bytes();
    // Determine `end - date` (skip at least one char, then continue while alpha)
    let mut end_idx: usize = 1;
    while end_idx < bytes.len() && bytes[end_idx].is_ascii_alphabetic() {
        end_idx += 1;
    }

    for i in 0..12 {
        let m = match_string(date, MONTH_NAMES[i]);
        if m >= 3 {
            tm.tm_mon = i as i32;
            *touched = 1;
            return end_idx;
        }
    }

    for s in SPECIAL.iter() {
        let len = s.name.len();
        if (match_string(date, s.name) as usize) == len {
            (s.fn_ptr)(tm, now, num);
            *touched = 1;
            return end_idx;
        }
    }

    if *num == 0 {
        for i in 1..11 {
            let len = NUMBER_NAME[i].len();
            if (match_string(date, NUMBER_NAME[i]) as usize) == len {
                *num = i as i32;
                *touched = 1;
                return end_idx;
            }
        }
        if match_string(date, "last") == 4 {
            *num = 1;
            *touched = 1;
        }
        return end_idx;
    }

    for tl in TYPELEN.iter() {
        let len = tl.type_name.len();
        if match_string(date, tl.type_name) >= (len as i32) - 1 {
            update_tm(tm, now, (tl.length as u64) * (*num as u64));
            *num = 0;
            *touched = 1;
            return end_idx;
        }
    }

    for i in 0..7 {
        let m = match_string(date, WEEKDAY_NAMES[i]);
        if m >= 3 {
            let n = *num - 1;
            *num = 0;

            let mut diff = tm.tm_wday - i as i32;
            let mut n = n;
            if diff <= 0 {
                n += 1;
            }
            diff += 7 * n;

            update_tm(tm, now, (diff as u64) * 24 * 60 * 60);
            *touched = 1;
            return end_idx;
        }
    }

    if match_string(date, "months") >= 5 {
        update_tm(tm, now, 0); // fill in date fields if needed
        let mut n = tm.tm_mon - *num;
        *num = 0;
        while n < 0 {
            n += 12;
            tm.tm_year -= 1;
        }
        tm.tm_mon = n;
        *touched = 1;
        return end_idx;
    }

    if match_string(date, "years") >= 4 {
        update_tm(tm, now, 0);
        tm.tm_year -= *num;
        *num = 0;
        *touched = 1;
        return end_idx;
    }

    end_idx
}

/// Returns the number of bytes consumed from the start of `date`.
pub fn approxidate_digit(date: &str, tm: &mut Atm, num: &mut i32, now: TimeT) -> usize {
    let bytes = date.as_bytes();
    let (number, end_pos) = strtoul(bytes, 0);

    if end_pos < bytes.len() {
        let c = bytes[end_pos];
        if c == b':' || c == b'.' || c == b'/' || c == b'-' {
            if end_pos + 1 < bytes.len() && bytes[end_pos + 1].is_ascii_digit() {
                let end_str = std::str::from_utf8(&bytes[end_pos..]).unwrap_or("");
                let m = match_multi_number(number, c as char, date, end_str, tm, now);
                if m != 0 {
                    return m as usize;
                }
            }
        }
    }

    // Accept zero-padding only for small numbers ("Dec 02", never "Dec 0002")
    if bytes.is_empty() || bytes[0] != b'0' || end_pos <= 2 {
        *num = number as i32;
    }
    end_pos
}

pub fn pending_number(tm: &mut Atm, num: &mut i32) {
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
    let mut tm = empty_tm();

    let time_sec = tv.tv_sec;
    localtime(&mut tm, time_sec);
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
    let mut pos: usize = 0;
    while pos < bytes.len() {
        let c = bytes[pos];
        if c == 0 {
            break;
        }
        if c.is_ascii_digit() {
            pending_number(&mut tm, &mut number);
            let sub = std::str::from_utf8(&bytes[pos..]).unwrap_or("");
            let consumed = approxidate_digit(sub, &mut tm, &mut number, time_sec);
            pos += consumed;
            touched = 1;
            continue;
        }
        if c.is_ascii_alphabetic() {
            let sub = std::str::from_utf8(&bytes[pos..]).unwrap_or("");
            let consumed =
                approxidate_alpha(sub, &mut tm, &mut now, &mut number, &mut touched);
            pos += consumed;
            continue;
        }
        pos += 1;
    }

    pending_number(&mut tm, &mut number);
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

    // gettimeofday equivalent: NULL relative_to → use current time.
    if let Ok(d) = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        tv.tv_sec = d.as_secs() as i64;
        tv.tv_usec = d.subsec_micros() as i64;
    }

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
