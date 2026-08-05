//! Translated from jsdate.c — the Date object and its prototype methods.
#![allow(non_snake_case, non_upper_case_globals)]

use crate::jsrun::*;
use crate::types::*;
use std::os::raw::{c_char, c_int};

macro_rules! cstr {
    ($s:literal) => {
        concat!($s, "\0").as_ptr() as *const c_char
    };
}

// #define js_optnumber(J,I,V) (js_isdefined(J,I) ? js_tonumber(J,I) : V)
unsafe fn js_optnumber(J: *mut js_State, i: c_int, v: f64) -> f64 {
    if crate::jsrun::js_isdefined(J, i) != 0 {
        crate::jsrun::js_tonumber(J, i)
    } else {
        v
    }
}

unsafe fn Now() -> f64 {
    let mut tv: libc::timeval = std::mem::zeroed();
    libc::gettimeofday(&mut tv, std::ptr::null_mut());
    crate::cutil::floor(tv.tv_sec as f64 * 1000.0 + tv.tv_usec as f64 / 1000.0)
}

unsafe fn LocalTZA() -> f64 {
    static mut ONCE: c_int = 1;
    static mut TZA: f64 = 0.0;
    if ONCE != 0 {
        let now = libc::time(std::ptr::null_mut());
        let utc = libc::mktime(libc::gmtime(&now));
        let loc = libc::mktime(libc::localtime(&now));
        TZA = (loc - utc) as f64 * 1000.0;
        ONCE = 0;
    }
    TZA
}

unsafe fn DaylightSavingTA(_t: f64) -> f64 {
    0.0 /* TODO */
}

/* Helpers from the ECMA 262 specification */

const HoursPerDay: f64 = 24.0;
const MinutesPerHour: f64 = 60.0;
const SecondsPerMinute: f64 = 60.0;
const MinutesPerDay: f64 = HoursPerDay * MinutesPerHour;
const SecondsPerHour: f64 = MinutesPerHour * SecondsPerMinute;
const SecondsPerDay: f64 = MinutesPerDay * SecondsPerMinute;

const msPerSecond: f64 = 1000.0;
const msPerDay: f64 = SecondsPerDay * msPerSecond;
const msPerHour: f64 = SecondsPerHour * msPerSecond;
const msPerMinute: f64 = SecondsPerMinute * msPerSecond;

unsafe fn pmod(x: f64, y: f64) -> f64 {
    let mut x = crate::cutil::fmod(x, y);
    if x < 0.0 {
        x += y;
    }
    x
}

unsafe fn Day(t: f64) -> c_int {
    crate::cutil::floor(t / msPerDay) as c_int
}

unsafe fn TimeWithinDay(t: f64) -> f64 {
    pmod(t, msPerDay)
}

unsafe fn DaysInYear(y: c_int) -> c_int {
    if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) {
        366
    } else {
        365
    }
}

unsafe fn DayFromYear(y: c_int) -> c_int {
    ((365 * (y - 1970)) as f64
        + crate::cutil::floor((y - 1969) as f64 / 4.0)
        - crate::cutil::floor((y - 1901) as f64 / 100.0)
        + crate::cutil::floor((y - 1601) as f64 / 400.0)) as c_int
}

unsafe fn TimeFromYear(y: c_int) -> f64 {
    DayFromYear(y) as f64 * msPerDay
}

unsafe fn YearFromTime(t: f64) -> c_int {
    let mut y = (crate::cutil::floor(t / (msPerDay * 365.2425)) + 1970.0) as c_int;
    let t2 = TimeFromYear(y);
    if t2 > t {
        y -= 1;
    } else if t2 + msPerDay * DaysInYear(y) as f64 <= t {
        y += 1;
    }
    y
}

unsafe fn InLeapYear(t: f64) -> c_int {
    (DaysInYear(YearFromTime(t)) == 366) as c_int
}

unsafe fn DayWithinYear(t: f64) -> c_int {
    Day(t) - DayFromYear(YearFromTime(t))
}

unsafe fn MonthFromTime(t: f64) -> c_int {
    let day = DayWithinYear(t);
    let leap = InLeapYear(t);
    if day < 31 {
        return 0;
    }
    if day < 59 + leap {
        return 1;
    }
    if day < 90 + leap {
        return 2;
    }
    if day < 120 + leap {
        return 3;
    }
    if day < 151 + leap {
        return 4;
    }
    if day < 181 + leap {
        return 5;
    }
    if day < 212 + leap {
        return 6;
    }
    if day < 243 + leap {
        return 7;
    }
    if day < 273 + leap {
        return 8;
    }
    if day < 304 + leap {
        return 9;
    }
    if day < 334 + leap {
        return 10;
    }
    11
}

unsafe fn DateFromTime(t: f64) -> c_int {
    let day = DayWithinYear(t);
    let leap = InLeapYear(t);
    match MonthFromTime(t) {
        0 => day + 1,
        1 => day - 30,
        2 => day - 58 - leap,
        3 => day - 89 - leap,
        4 => day - 119 - leap,
        5 => day - 150 - leap,
        6 => day - 180 - leap,
        7 => day - 211 - leap,
        8 => day - 242 - leap,
        9 => day - 272 - leap,
        10 => day - 303 - leap,
        _ => day - 333 - leap,
    }
}

unsafe fn WeekDay(t: f64) -> c_int {
    pmod((Day(t) + 4) as f64, 7.0) as c_int
}

unsafe fn LocalTime(utc: f64) -> f64 {
    utc + LocalTZA() + DaylightSavingTA(utc)
}

unsafe fn UTC(loc: f64) -> f64 {
    loc - LocalTZA() - DaylightSavingTA(loc - LocalTZA())
}

unsafe fn HourFromTime(t: f64) -> c_int {
    pmod(crate::cutil::floor(t / msPerHour), HoursPerDay) as c_int
}

unsafe fn MinFromTime(t: f64) -> c_int {
    pmod(crate::cutil::floor(t / msPerMinute), MinutesPerHour) as c_int
}

unsafe fn SecFromTime(t: f64) -> c_int {
    pmod(crate::cutil::floor(t / msPerSecond), SecondsPerMinute) as c_int
}

unsafe fn msFromTime(t: f64) -> c_int {
    pmod(t, msPerSecond) as c_int
}

unsafe fn MakeTime(hour: f64, min: f64, sec: f64, ms: f64) -> f64 {
    ((hour * MinutesPerHour + min) * SecondsPerMinute + sec) * msPerSecond + ms
}

unsafe fn MakeDay(y: f64, m: f64, date: f64) -> f64 {
    /*
     * The following array contains the day of year for the first day of
     * each month, where index 0 is January, and day 0 is January 1.
     */
    static firstDayOfMonth: [[f64; 12]; 2] = [
        [0.0, 31.0, 59.0, 90.0, 120.0, 151.0, 181.0, 212.0, 243.0, 273.0, 304.0, 334.0],
        [0.0, 31.0, 60.0, 91.0, 121.0, 152.0, 182.0, 213.0, 244.0, 274.0, 305.0, 335.0],
    ];

    let mut y = y;
    let mut m = m;

    y += crate::cutil::floor(m / 12.0);
    m = pmod(m, 12.0);

    let im = m as c_int;
    if im < 0 || im >= 12 {
        return f64::NAN;
    }

    let yd = crate::cutil::floor(TimeFromYear(y as c_int) / msPerDay);
    let md = firstDayOfMonth[(DaysInYear(y as c_int) == 366) as usize][im as usize];

    yd + md + date - 1.0
}

unsafe fn MakeDate(day: f64, time: f64) -> f64 {
    day * msPerDay + time
}

unsafe fn TimeClip(t: f64) -> f64 {
    if !t.is_finite() {
        return f64::NAN;
    }
    if crate::cutil::fabs(t) > 8.64e15 {
        return f64::NAN;
    }
    if t < 0.0 {
        -crate::cutil::floor(-t)
    } else {
        crate::cutil::floor(t)
    }
}

unsafe fn toint(sp: *mut *const c_char, w: c_int, v: *mut c_int) -> c_int {
    let mut s = *sp;
    *v = 0;
    let mut w = w;
    while w != 0 {
        w -= 1;
        if *s < b'0' as c_char || *s > b'9' as c_char {
            return 0;
        }
        *v = *v * 10 + (*s as c_int - '0' as c_int);
        s = s.add(1);
    }
    *sp = s;
    1
}

unsafe fn parseDateTime(s: *const c_char) -> f64 {
    let mut y: c_int = 1970;
    let mut m: c_int = 1;
    let mut d: c_int = 1;
    let mut H: c_int = 0;
    let mut M: c_int = 0;
    let mut S: c_int = 0;
    let mut ms: c_int = 0;
    let mut tza: c_int = 0;
    let t: f64;

    /* Parse ISO 8601 formatted date and time: */
    /* YYYY("-"MM("-"DD)?)?("T"HH":"mm(":"ss("."sss)?)?("Z"|[+-]HH(":"mm)?)?)? */

    let mut s = s;
    if toint(&mut s, 4, &mut y) == 0 {
        return f64::NAN;
    }
    if *s == b'-' as c_char {
        s = s.add(1);
        if toint(&mut s, 2, &mut m) == 0 {
            return f64::NAN;
        }
        if *s == b'-' as c_char {
            s = s.add(1);
            if toint(&mut s, 2, &mut d) == 0 {
                return f64::NAN;
            }
        }
    }

    if *s == b'T' as c_char {
        s = s.add(1);
        if toint(&mut s, 2, &mut H) == 0 {
            return f64::NAN;
        }
        if *s != b':' as c_char {
            return f64::NAN;
        }
        s = s.add(1);
        if toint(&mut s, 2, &mut M) == 0 {
            return f64::NAN;
        }
        if *s == b':' as c_char {
            s = s.add(1);
            if toint(&mut s, 2, &mut S) == 0 {
                return f64::NAN;
            }
            if *s == b'.' as c_char {
                s = s.add(1);
                if toint(&mut s, 3, &mut ms) == 0 {
                    return f64::NAN;
                }
            }
        }
        if *s == b'Z' as c_char {
            s = s.add(1);
            tza = 0;
        } else if *s == b'+' as c_char || *s == b'-' as c_char {
            let mut tzh: c_int = 0;
            let mut tzm: c_int = 0;
            let tzs: c_int = if *s == b'+' as c_char { 1 } else { -1 };
            s = s.add(1);
            if toint(&mut s, 2, &mut tzh) == 0 {
                return f64::NAN;
            }
            if *s == b':' as c_char {
                s = s.add(1);
                if toint(&mut s, 2, &mut tzm) == 0 {
                    return f64::NAN;
                }
            }
            if tzh > 23 || tzm > 59 {
                return f64::NAN;
            }
            tza = (tzs as f64 * (tzh as f64 * msPerHour + tzm as f64 * msPerMinute)) as c_int;
        } else {
            tza = LocalTZA() as c_int;
        }
    }

    if *s != 0 {
        return f64::NAN;
    }

    if m < 1 || m > 12 {
        return f64::NAN;
    }
    if d < 1 || d > 31 {
        return f64::NAN;
    }
    if H < 0 || H > 24 {
        return f64::NAN;
    }
    if M < 0 || M > 59 {
        return f64::NAN;
    }
    if S < 0 || S > 59 {
        return f64::NAN;
    }
    if ms < 0 || ms > 999 {
        return f64::NAN;
    }
    if H == 24 && (M != 0 || S != 0 || ms != 0) {
        return f64::NAN;
    }

    /* TODO: DaylightSavingTA on local times */
    t = MakeDate(
        MakeDay(y as f64, (m - 1) as f64, d as f64),
        MakeTime(H as f64, M as f64, S as f64, ms as f64),
    );
    t - tza as f64
}

/* date formatting */

unsafe fn fmtdate(buf: *mut c_char, t: f64) -> *mut c_char {
    let y = YearFromTime(t);
    let m = MonthFromTime(t);
    let d = DateFromTime(t);
    if !t.is_finite() {
        return cstr!("Invalid Date") as *mut c_char;
    }
    libc::sprintf(buf, cstr!("%04d-%02d-%02d"), y, m + 1, d);
    buf
}

unsafe fn fmttime(buf: *mut c_char, t: f64, tza: f64) -> *mut c_char {
    let H = HourFromTime(t);
    let M = MinFromTime(t);
    let S = SecFromTime(t);
    let ms = msFromTime(t);
    let tzh = HourFromTime(crate::cutil::fabs(tza));
    let tzm = MinFromTime(crate::cutil::fabs(tza));
    if !t.is_finite() {
        return cstr!("Invalid Date") as *mut c_char;
    }
    if tza == 0.0 {
        libc::sprintf(buf, cstr!("%02d:%02d:%02d.%03dZ"), H, M, S, ms);
    } else if tza < 0.0 {
        libc::sprintf(buf, cstr!("%02d:%02d:%02d.%03d-%02d:%02d"), H, M, S, ms, tzh, tzm);
    } else {
        libc::sprintf(buf, cstr!("%02d:%02d:%02d.%03d+%02d:%02d"), H, M, S, ms, tzh, tzm);
    }
    buf
}

unsafe fn fmtdatetime(buf: *mut c_char, t: f64, tza: f64) -> *mut c_char {
    let mut dbuf: [c_char; 20] = [0; 20];
    let mut tbuf: [c_char; 20] = [0; 20];
    if !t.is_finite() {
        return cstr!("Invalid Date") as *mut c_char;
    }
    fmtdate(dbuf.as_mut_ptr(), t);
    fmttime(tbuf.as_mut_ptr(), t, tza);
    libc::sprintf(buf, cstr!("%sT%s"), dbuf.as_ptr(), tbuf.as_ptr());
    buf
}

/* Date functions */

unsafe fn js_todate(J: *mut js_State, idx: c_int) -> f64 {
    let self_ = js_toobject(J, idx);
    if (*self_).type_ != JS_CDATE {
        crate::jserror::js_typeerror(J, cstr!("not a date"));
    }
    (*self_).u.number
}

unsafe fn js_setdate(J: *mut js_State, idx: c_int, t: f64) {
    let self_ = js_toobject(J, idx);
    if (*self_).type_ != JS_CDATE {
        crate::jserror::js_typeerror(J, cstr!("not a date"));
    }
    (*self_).u.number = TimeClip(t);
    js_pushnumber(J, (*self_).u.number);
}

unsafe extern "C-unwind" fn D_parse(J: *mut js_State) {
    let t = parseDateTime(js_tostring(J, 1));
    js_pushnumber(J, t);
}

unsafe extern "C-unwind" fn D_UTC(J: *mut js_State) {
    let mut y: f64;
    let m: f64;
    let d: f64;
    let H: f64;
    let M: f64;
    let S: f64;
    let ms: f64;
    let mut t: f64;
    y = js_tonumber(J, 1);
    if y < 100.0 {
        y += 1900.0;
    }
    m = js_tonumber(J, 2);
    d = js_optnumber(J, 3, 1.0);
    H = js_optnumber(J, 4, 0.0);
    M = js_optnumber(J, 5, 0.0);
    S = js_optnumber(J, 6, 0.0);
    ms = js_optnumber(J, 7, 0.0);
    t = MakeDate(MakeDay(y, m, d), MakeTime(H, M, S, ms));
    t = TimeClip(t);
    js_pushnumber(J, t);
}

unsafe extern "C-unwind" fn D_now(J: *mut js_State) {
    js_pushnumber(J, Now());
}

unsafe extern "C-unwind" fn jsB_Date(J: *mut js_State) {
    let mut buf: [c_char; 64] = [0; 64];
    js_pushstring(J, fmtdatetime(buf.as_mut_ptr(), LocalTime(Now()), LocalTZA()));
}

unsafe extern "C-unwind" fn jsB_new_Date(J: *mut js_State) {
    let top = js_gettop(J);
    let obj: *mut js_Object;
    let t: f64;

    if top == 1 {
        t = Now();
    } else if top == 2 {
        js_toprimitive(J, 1, JS_HNONE);
        if js_isstring(J, 1) != 0 {
            t = parseDateTime(js_tostring(J, 1));
        } else {
            t = TimeClip(js_tonumber(J, 1));
        }
    } else {
        let mut y: f64;
        let m: f64;
        let d: f64;
        let H: f64;
        let M: f64;
        let S: f64;
        let ms: f64;
        y = js_tonumber(J, 1);
        if y < 100.0 {
            y += 1900.0;
        }
        m = js_tonumber(J, 2);
        d = js_optnumber(J, 3, 1.0);
        H = js_optnumber(J, 4, 0.0);
        M = js_optnumber(J, 5, 0.0);
        S = js_optnumber(J, 6, 0.0);
        ms = js_optnumber(J, 7, 0.0);
        let tt = MakeDate(MakeDay(y, m, d), MakeTime(H, M, S, ms));
        t = TimeClip(UTC(tt));
    }

    obj = crate::jsproperty::jsV_newobject(J, JS_CDATE, (*J).Date_prototype);
    (*obj).u.number = t;

    js_pushobject(J, obj);
}

unsafe extern "C-unwind" fn Dp_valueOf(J: *mut js_State) {
    let t = js_todate(J, 0);
    js_pushnumber(J, t);
}

unsafe extern "C-unwind" fn Dp_toString(J: *mut js_State) {
    let mut buf: [c_char; 64] = [0; 64];
    let t = js_todate(J, 0);
    js_pushstring(J, fmtdatetime(buf.as_mut_ptr(), LocalTime(t), LocalTZA()));
}

unsafe extern "C-unwind" fn Dp_toDateString(J: *mut js_State) {
    let mut buf: [c_char; 64] = [0; 64];
    let t = js_todate(J, 0);
    js_pushstring(J, fmtdate(buf.as_mut_ptr(), LocalTime(t)));
}

unsafe extern "C-unwind" fn Dp_toTimeString(J: *mut js_State) {
    let mut buf: [c_char; 64] = [0; 64];
    let t = js_todate(J, 0);
    js_pushstring(J, fmttime(buf.as_mut_ptr(), LocalTime(t), LocalTZA()));
}

unsafe extern "C-unwind" fn Dp_toUTCString(J: *mut js_State) {
    let mut buf: [c_char; 64] = [0; 64];
    let t = js_todate(J, 0);
    js_pushstring(J, fmtdatetime(buf.as_mut_ptr(), t, 0.0));
}

unsafe extern "C-unwind" fn Dp_toISOString(J: *mut js_State) {
    let mut buf: [c_char; 64] = [0; 64];
    let t = js_todate(J, 0);
    if !t.is_finite() {
        crate::jserror::js_rangeerror(J, cstr!("invalid date"));
    }
    js_pushstring(J, fmtdatetime(buf.as_mut_ptr(), t, 0.0));
}

unsafe extern "C-unwind" fn Dp_getFullYear(J: *mut js_State) {
    let t = js_todate(J, 0);
    if t.is_nan() {
        js_pushnumber(J, f64::NAN);
    } else {
        js_pushnumber(J, YearFromTime(LocalTime(t)) as f64);
    }
}

unsafe extern "C-unwind" fn Dp_getMonth(J: *mut js_State) {
    let t = js_todate(J, 0);
    if t.is_nan() {
        js_pushnumber(J, f64::NAN);
    } else {
        js_pushnumber(J, MonthFromTime(LocalTime(t)) as f64);
    }
}

unsafe extern "C-unwind" fn Dp_getDate(J: *mut js_State) {
    let t = js_todate(J, 0);
    if t.is_nan() {
        js_pushnumber(J, f64::NAN);
    } else {
        js_pushnumber(J, DateFromTime(LocalTime(t)) as f64);
    }
}

unsafe extern "C-unwind" fn Dp_getDay(J: *mut js_State) {
    let t = js_todate(J, 0);
    if t.is_nan() {
        js_pushnumber(J, f64::NAN);
    } else {
        js_pushnumber(J, WeekDay(LocalTime(t)) as f64);
    }
}

unsafe extern "C-unwind" fn Dp_getHours(J: *mut js_State) {
    let t = js_todate(J, 0);
    if t.is_nan() {
        js_pushnumber(J, f64::NAN);
    } else {
        js_pushnumber(J, HourFromTime(LocalTime(t)) as f64);
    }
}

unsafe extern "C-unwind" fn Dp_getMinutes(J: *mut js_State) {
    let t = js_todate(J, 0);
    if t.is_nan() {
        js_pushnumber(J, f64::NAN);
    } else {
        js_pushnumber(J, MinFromTime(LocalTime(t)) as f64);
    }
}

unsafe extern "C-unwind" fn Dp_getSeconds(J: *mut js_State) {
    let t = js_todate(J, 0);
    if t.is_nan() {
        js_pushnumber(J, f64::NAN);
    } else {
        js_pushnumber(J, SecFromTime(LocalTime(t)) as f64);
    }
}

unsafe extern "C-unwind" fn Dp_getMilliseconds(J: *mut js_State) {
    let t = js_todate(J, 0);
    if t.is_nan() {
        js_pushnumber(J, f64::NAN);
    } else {
        js_pushnumber(J, msFromTime(LocalTime(t)) as f64);
    }
}

unsafe extern "C-unwind" fn Dp_getUTCFullYear(J: *mut js_State) {
    let t = js_todate(J, 0);
    if t.is_nan() {
        js_pushnumber(J, f64::NAN);
    } else {
        js_pushnumber(J, YearFromTime(t) as f64);
    }
}

unsafe extern "C-unwind" fn Dp_getUTCMonth(J: *mut js_State) {
    let t = js_todate(J, 0);
    if t.is_nan() {
        js_pushnumber(J, f64::NAN);
    } else {
        js_pushnumber(J, MonthFromTime(t) as f64);
    }
}

unsafe extern "C-unwind" fn Dp_getUTCDate(J: *mut js_State) {
    let t = js_todate(J, 0);
    if t.is_nan() {
        js_pushnumber(J, f64::NAN);
    } else {
        js_pushnumber(J, DateFromTime(t) as f64);
    }
}

unsafe extern "C-unwind" fn Dp_getUTCDay(J: *mut js_State) {
    let t = js_todate(J, 0);
    if t.is_nan() {
        js_pushnumber(J, f64::NAN);
    } else {
        js_pushnumber(J, WeekDay(t) as f64);
    }
}

unsafe extern "C-unwind" fn Dp_getUTCHours(J: *mut js_State) {
    let t = js_todate(J, 0);
    if t.is_nan() {
        js_pushnumber(J, f64::NAN);
    } else {
        js_pushnumber(J, HourFromTime(t) as f64);
    }
}

unsafe extern "C-unwind" fn Dp_getUTCMinutes(J: *mut js_State) {
    let t = js_todate(J, 0);
    if t.is_nan() {
        js_pushnumber(J, f64::NAN);
    } else {
        js_pushnumber(J, MinFromTime(t) as f64);
    }
}

unsafe extern "C-unwind" fn Dp_getUTCSeconds(J: *mut js_State) {
    let t = js_todate(J, 0);
    if t.is_nan() {
        js_pushnumber(J, f64::NAN);
    } else {
        js_pushnumber(J, SecFromTime(t) as f64);
    }
}

unsafe extern "C-unwind" fn Dp_getUTCMilliseconds(J: *mut js_State) {
    let t = js_todate(J, 0);
    if t.is_nan() {
        js_pushnumber(J, f64::NAN);
    } else {
        js_pushnumber(J, msFromTime(t) as f64);
    }
}

unsafe extern "C-unwind" fn Dp_getTimezoneOffset(J: *mut js_State) {
    let t = js_todate(J, 0);
    if t.is_nan() {
        js_pushnumber(J, f64::NAN);
    } else {
        js_pushnumber(J, (t - LocalTime(t)) / msPerMinute);
    }
}

unsafe extern "C-unwind" fn Dp_setTime(J: *mut js_State) {
    js_setdate(J, 0, js_tonumber(J, 1));
}

unsafe extern "C-unwind" fn Dp_setMilliseconds(J: *mut js_State) {
    let t = LocalTime(js_todate(J, 0));
    let h = HourFromTime(t) as f64;
    let m = MinFromTime(t) as f64;
    let s = SecFromTime(t) as f64;
    let ms = js_tonumber(J, 1);
    js_setdate(J, 0, UTC(MakeDate(Day(t) as f64, MakeTime(h, m, s, ms))));
}

unsafe extern "C-unwind" fn Dp_setSeconds(J: *mut js_State) {
    let t = LocalTime(js_todate(J, 0));
    let h = HourFromTime(t) as f64;
    let m = MinFromTime(t) as f64;
    let s = js_tonumber(J, 1);
    let ms = js_optnumber(J, 2, msFromTime(t) as f64);
    js_setdate(J, 0, UTC(MakeDate(Day(t) as f64, MakeTime(h, m, s, ms))));
}

unsafe extern "C-unwind" fn Dp_setMinutes(J: *mut js_State) {
    let t = LocalTime(js_todate(J, 0));
    let h = HourFromTime(t) as f64;
    let m = js_tonumber(J, 1);
    let s = js_optnumber(J, 2, SecFromTime(t) as f64);
    let ms = js_optnumber(J, 3, msFromTime(t) as f64);
    js_setdate(J, 0, UTC(MakeDate(Day(t) as f64, MakeTime(h, m, s, ms))));
}

unsafe extern "C-unwind" fn Dp_setHours(J: *mut js_State) {
    let t = LocalTime(js_todate(J, 0));
    let h = js_tonumber(J, 1);
    let m = js_optnumber(J, 2, MinFromTime(t) as f64);
    let s = js_optnumber(J, 3, SecFromTime(t) as f64);
    let ms = js_optnumber(J, 4, msFromTime(t) as f64);
    js_setdate(J, 0, UTC(MakeDate(Day(t) as f64, MakeTime(h, m, s, ms))));
}

unsafe extern "C-unwind" fn Dp_setDate(J: *mut js_State) {
    let t = LocalTime(js_todate(J, 0));
    let y = YearFromTime(t) as f64;
    let m = MonthFromTime(t) as f64;
    let d = js_tonumber(J, 1);
    js_setdate(J, 0, UTC(MakeDate(MakeDay(y, m, d), TimeWithinDay(t))));
}

unsafe extern "C-unwind" fn Dp_setMonth(J: *mut js_State) {
    let t = LocalTime(js_todate(J, 0));
    let y = YearFromTime(t) as f64;
    let m = js_tonumber(J, 1);
    let d = js_optnumber(J, 2, DateFromTime(t) as f64);
    js_setdate(J, 0, UTC(MakeDate(MakeDay(y, m, d), TimeWithinDay(t))));
}

unsafe extern "C-unwind" fn Dp_setFullYear(J: *mut js_State) {
    let t = LocalTime(js_todate(J, 0));
    let y = js_tonumber(J, 1);
    let m = js_optnumber(J, 2, MonthFromTime(t) as f64);
    let d = js_optnumber(J, 3, DateFromTime(t) as f64);
    js_setdate(J, 0, UTC(MakeDate(MakeDay(y, m, d), TimeWithinDay(t))));
}

unsafe extern "C-unwind" fn Dp_setUTCMilliseconds(J: *mut js_State) {
    let t = js_todate(J, 0);
    let h = HourFromTime(t) as f64;
    let m = MinFromTime(t) as f64;
    let s = SecFromTime(t) as f64;
    let ms = js_tonumber(J, 1);
    js_setdate(J, 0, MakeDate(Day(t) as f64, MakeTime(h, m, s, ms)));
}

unsafe extern "C-unwind" fn Dp_setUTCSeconds(J: *mut js_State) {
    let t = js_todate(J, 0);
    let h = HourFromTime(t) as f64;
    let m = MinFromTime(t) as f64;
    let s = js_tonumber(J, 1);
    let ms = js_optnumber(J, 2, msFromTime(t) as f64);
    js_setdate(J, 0, MakeDate(Day(t) as f64, MakeTime(h, m, s, ms)));
}

unsafe extern "C-unwind" fn Dp_setUTCMinutes(J: *mut js_State) {
    let t = js_todate(J, 0);
    let h = HourFromTime(t) as f64;
    let m = js_tonumber(J, 1);
    let s = js_optnumber(J, 2, SecFromTime(t) as f64);
    let ms = js_optnumber(J, 3, msFromTime(t) as f64);
    js_setdate(J, 0, MakeDate(Day(t) as f64, MakeTime(h, m, s, ms)));
}

unsafe extern "C-unwind" fn Dp_setUTCHours(J: *mut js_State) {
    let t = js_todate(J, 0);
    let h = js_tonumber(J, 1);
    let m = js_optnumber(J, 2, HourFromTime(t) as f64);
    let s = js_optnumber(J, 3, SecFromTime(t) as f64);
    let ms = js_optnumber(J, 4, msFromTime(t) as f64);
    js_setdate(J, 0, MakeDate(Day(t) as f64, MakeTime(h, m, s, ms)));
}

unsafe extern "C-unwind" fn Dp_setUTCDate(J: *mut js_State) {
    let t = js_todate(J, 0);
    let y = YearFromTime(t) as f64;
    let m = MonthFromTime(t) as f64;
    let d = js_tonumber(J, 1);
    js_setdate(J, 0, MakeDate(MakeDay(y, m, d), TimeWithinDay(t)));
}

unsafe extern "C-unwind" fn Dp_setUTCMonth(J: *mut js_State) {
    let t = js_todate(J, 0);
    let y = YearFromTime(t) as f64;
    let m = js_tonumber(J, 1);
    let d = js_optnumber(J, 2, DateFromTime(t) as f64);
    js_setdate(J, 0, MakeDate(MakeDay(y, m, d), TimeWithinDay(t)));
}

unsafe extern "C-unwind" fn Dp_setUTCFullYear(J: *mut js_State) {
    let t = js_todate(J, 0);
    let y = js_tonumber(J, 1);
    let m = js_optnumber(J, 2, MonthFromTime(t) as f64);
    let d = js_optnumber(J, 3, DateFromTime(t) as f64);
    js_setdate(J, 0, MakeDate(MakeDay(y, m, d), TimeWithinDay(t)));
}

unsafe extern "C-unwind" fn Dp_toJSON(J: *mut js_State) {
    js_copy(J, 0);
    js_toprimitive(J, -1, JS_HNUMBER);
    if js_isnumber(J, -1) != 0 && !js_tonumber(J, -1).is_finite() {
        js_pushnull(J);
        return;
    }
    js_pop(J, 1);

    js_getproperty(J, 0, cstr!("toISOString"));
    if js_iscallable(J, -1) == 0 {
        crate::jserror::js_typeerror(J, cstr!("this.toISOString is not a function"));
    }
    js_copy(J, 0);
    js_call(J, 0);
}

#[no_mangle]
pub unsafe extern "C-unwind" fn jsB_initdate(J: *mut js_State) {
    (*(*J).Date_prototype).u.number = 0.0;

    js_pushobject(J, (*J).Date_prototype);
    {
        crate::jsbuiltin::jsB_propf(J, cstr!("Date.prototype.valueOf"), Some(Dp_valueOf), 0);
        crate::jsbuiltin::jsB_propf(J, cstr!("Date.prototype.toString"), Some(Dp_toString), 0);
        crate::jsbuiltin::jsB_propf(J, cstr!("Date.prototype.toDateString"), Some(Dp_toDateString), 0);
        crate::jsbuiltin::jsB_propf(J, cstr!("Date.prototype.toTimeString"), Some(Dp_toTimeString), 0);
        crate::jsbuiltin::jsB_propf(J, cstr!("Date.prototype.toLocaleString"), Some(Dp_toString), 0);
        crate::jsbuiltin::jsB_propf(J, cstr!("Date.prototype.toLocaleDateString"), Some(Dp_toDateString), 0);
        crate::jsbuiltin::jsB_propf(J, cstr!("Date.prototype.toLocaleTimeString"), Some(Dp_toTimeString), 0);
        crate::jsbuiltin::jsB_propf(J, cstr!("Date.prototype.toUTCString"), Some(Dp_toUTCString), 0);

        crate::jsbuiltin::jsB_propf(J, cstr!("Date.prototype.getTime"), Some(Dp_valueOf), 0);
        crate::jsbuiltin::jsB_propf(J, cstr!("Date.prototype.getFullYear"), Some(Dp_getFullYear), 0);
        crate::jsbuiltin::jsB_propf(J, cstr!("Date.prototype.getUTCFullYear"), Some(Dp_getUTCFullYear), 0);
        crate::jsbuiltin::jsB_propf(J, cstr!("Date.prototype.getMonth"), Some(Dp_getMonth), 0);
        crate::jsbuiltin::jsB_propf(J, cstr!("Date.prototype.getUTCMonth"), Some(Dp_getUTCMonth), 0);
        crate::jsbuiltin::jsB_propf(J, cstr!("Date.prototype.getDate"), Some(Dp_getDate), 0);
        crate::jsbuiltin::jsB_propf(J, cstr!("Date.prototype.getUTCDate"), Some(Dp_getUTCDate), 0);
        crate::jsbuiltin::jsB_propf(J, cstr!("Date.prototype.getDay"), Some(Dp_getDay), 0);
        crate::jsbuiltin::jsB_propf(J, cstr!("Date.prototype.getUTCDay"), Some(Dp_getUTCDay), 0);
        crate::jsbuiltin::jsB_propf(J, cstr!("Date.prototype.getHours"), Some(Dp_getHours), 0);
        crate::jsbuiltin::jsB_propf(J, cstr!("Date.prototype.getUTCHours"), Some(Dp_getUTCHours), 0);
        crate::jsbuiltin::jsB_propf(J, cstr!("Date.prototype.getMinutes"), Some(Dp_getMinutes), 0);
        crate::jsbuiltin::jsB_propf(J, cstr!("Date.prototype.getUTCMinutes"), Some(Dp_getUTCMinutes), 0);
        crate::jsbuiltin::jsB_propf(J, cstr!("Date.prototype.getSeconds"), Some(Dp_getSeconds), 0);
        crate::jsbuiltin::jsB_propf(J, cstr!("Date.prototype.getUTCSeconds"), Some(Dp_getUTCSeconds), 0);
        crate::jsbuiltin::jsB_propf(J, cstr!("Date.prototype.getMilliseconds"), Some(Dp_getMilliseconds), 0);
        crate::jsbuiltin::jsB_propf(J, cstr!("Date.prototype.getUTCMilliseconds"), Some(Dp_getUTCMilliseconds), 0);
        crate::jsbuiltin::jsB_propf(J, cstr!("Date.prototype.getTimezoneOffset"), Some(Dp_getTimezoneOffset), 0);

        crate::jsbuiltin::jsB_propf(J, cstr!("Date.prototype.setTime"), Some(Dp_setTime), 1);
        crate::jsbuiltin::jsB_propf(J, cstr!("Date.prototype.setMilliseconds"), Some(Dp_setMilliseconds), 1);
        crate::jsbuiltin::jsB_propf(J, cstr!("Date.prototype.setUTCMilliseconds"), Some(Dp_setUTCMilliseconds), 1);
        crate::jsbuiltin::jsB_propf(J, cstr!("Date.prototype.setSeconds"), Some(Dp_setSeconds), 2);
        crate::jsbuiltin::jsB_propf(J, cstr!("Date.prototype.setUTCSeconds"), Some(Dp_setUTCSeconds), 2);
        crate::jsbuiltin::jsB_propf(J, cstr!("Date.prototype.setMinutes"), Some(Dp_setMinutes), 3);
        crate::jsbuiltin::jsB_propf(J, cstr!("Date.prototype.setUTCMinutes"), Some(Dp_setUTCMinutes), 3);
        crate::jsbuiltin::jsB_propf(J, cstr!("Date.prototype.setHours"), Some(Dp_setHours), 4);
        crate::jsbuiltin::jsB_propf(J, cstr!("Date.prototype.setUTCHours"), Some(Dp_setUTCHours), 4);
        crate::jsbuiltin::jsB_propf(J, cstr!("Date.prototype.setDate"), Some(Dp_setDate), 1);
        crate::jsbuiltin::jsB_propf(J, cstr!("Date.prototype.setUTCDate"), Some(Dp_setUTCDate), 1);
        crate::jsbuiltin::jsB_propf(J, cstr!("Date.prototype.setMonth"), Some(Dp_setMonth), 2);
        crate::jsbuiltin::jsB_propf(J, cstr!("Date.prototype.setUTCMonth"), Some(Dp_setUTCMonth), 2);
        crate::jsbuiltin::jsB_propf(J, cstr!("Date.prototype.setFullYear"), Some(Dp_setFullYear), 3);
        crate::jsbuiltin::jsB_propf(J, cstr!("Date.prototype.setUTCFullYear"), Some(Dp_setUTCFullYear), 3);

        /* ES5 */
        crate::jsbuiltin::jsB_propf(J, cstr!("Date.prototype.toISOString"), Some(Dp_toISOString), 0);
        crate::jsbuiltin::jsB_propf(J, cstr!("Date.prototype.toJSON"), Some(Dp_toJSON), 1);
    }
    crate::jsvalue::js_newcconstructor(J, Some(jsB_Date), Some(jsB_new_Date), cstr!("Date"), 0); /* 1 */
    {
        crate::jsbuiltin::jsB_propf(J, cstr!("Date.parse"), Some(D_parse), 1);
        crate::jsbuiltin::jsB_propf(J, cstr!("Date.UTC"), Some(D_UTC), 7);

        /* ES5 */
        crate::jsbuiltin::jsB_propf(J, cstr!("Date.now"), Some(D_now), 0);
    }
    js_defglobal(J, cstr!("Date"), JS_DONTENUM);
}
