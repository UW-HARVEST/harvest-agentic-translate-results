//! Translation of jsdate.c

use crate::*;

/// `js_optnumber(J,I,V)` -- (js_isdefined(J,I) ? js_tonumber(J,I) : V)
macro_rules! js_optnumber {
    ($J:expr, $I:expr, $V:expr) => {
        if js_isdefined($J, $I) != 0 {
            js_tonumber($J, $I)
        } else {
            $V
        }
    };
}

/* C's implicit double -> int conversion (x86-64 cvttsd2si: NaN and
 * out-of-range values yield the "integer indefinite" value INT_MIN). */
#[inline(always)]
fn d2i(x: f64) -> c_int {
    if x >= -2147483648.0 && x < 2147483648.0 {
        x as c_int
    } else {
        INT_MIN
    }
}

unsafe fn Now() -> f64 {
    let mut tv: timeval = timeval {
        tv_sec: 0,
        tv_usec: 0,
    };
    gettimeofday(&mut tv, null_mut());
    floor(tv.tv_sec as f64 * 1000.0 + tv.tv_usec as f64 / 1000.0)
}

unsafe fn LocalTZA() -> f64 {
    static mut once: c_int = 1;
    static mut tza: f64 = 0.0;
    if once != 0 {
        let mut now: c_long = time(null_mut());
        let utc: c_long = mktime(gmtime(&now));
        let loc: c_long = mktime(localtime(&now));
        tza = ((loc - utc) * 1000) as f64;
        once = 0;
    }
    tza
}

unsafe fn DaylightSavingTA(t: f64) -> f64 {
    0.0 /* TODO */
}

/* Helpers from the ECMA 262 specification */

const HoursPerDay: f64 = 24.0;
const MinutesPerHour: f64 = 60.0;
const MinutesPerDay: f64 = HoursPerDay * MinutesPerHour;
const SecondsPerMinute: f64 = 60.0;
const SecondsPerDay: f64 = MinutesPerDay * SecondsPerMinute;
const SecondsPerHour: f64 = MinutesPerHour * SecondsPerMinute;

const msPerSecond: f64 = 1000.0;
const msPerDay: f64 = SecondsPerDay * msPerSecond;
const msPerHour: f64 = SecondsPerHour * msPerSecond;
const msPerMinute: f64 = SecondsPerMinute * msPerSecond;

unsafe fn pmod(x: f64, y: f64) -> f64 {
    let mut x = x;
    x = fmod(x, y);
    if x < 0.0 {
        x += y;
    }
    x
}

unsafe fn Day(t: f64) -> c_int {
    d2i(floor(t / msPerDay))
}

unsafe fn TimeWithinDay(t: f64) -> f64 {
    pmod(t, msPerDay)
}

unsafe fn DaysInYear(y: c_int) -> c_int {
    if y % 4 == 0 && (y % 100 != 0 || (y % 400 == 0)) {
        366
    } else {
        365
    }
}

unsafe fn DayFromYear(y: c_int) -> c_int {
    d2i(365i32.wrapping_mul(y.wrapping_sub(1970)) as f64
        + floor(y.wrapping_sub(1969) as f64 / 4.0)
        - floor(y.wrapping_sub(1901) as f64 / 100.0)
        + floor(y.wrapping_sub(1601) as f64 / 400.0))
}

unsafe fn TimeFromYear(y: c_int) -> f64 {
    DayFromYear(y) as f64 * msPerDay
}

unsafe fn YearFromTime(t: f64) -> c_int {
    let mut y: c_int = d2i(floor(t / (msPerDay * 365.2425)) + 1970.0);
    let t2: f64 = TimeFromYear(y);
    if t2 > t {
        y = y.wrapping_sub(1);
    } else if t2 + msPerDay * DaysInYear(y) as f64 <= t {
        y = y.wrapping_add(1);
    }
    y
}

unsafe fn InLeapYear(t: f64) -> c_int {
    (DaysInYear(YearFromTime(t)) == 366) as c_int
}

unsafe fn DayWithinYear(t: f64) -> c_int {
    Day(t).wrapping_sub(DayFromYear(YearFromTime(t)))
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
    d2i(pmod(Day(t).wrapping_add(4) as f64, 7.0))
}

unsafe fn LocalTime(utc: f64) -> f64 {
    utc + LocalTZA() + DaylightSavingTA(utc)
}

unsafe fn UTC(loc: f64) -> f64 {
    loc - LocalTZA() - DaylightSavingTA(loc - LocalTZA())
}

unsafe fn HourFromTime(t: f64) -> c_int {
    d2i(pmod(floor(t / msPerHour), HoursPerDay))
}

unsafe fn MinFromTime(t: f64) -> c_int {
    d2i(pmod(floor(t / msPerMinute), MinutesPerHour))
}

unsafe fn SecFromTime(t: f64) -> c_int {
    d2i(pmod(floor(t / msPerSecond), SecondsPerMinute))
}

unsafe fn msFromTime(t: f64) -> c_int {
    d2i(pmod(t, msPerSecond))
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
        [
            0.0, 31.0, 59.0, 90.0, 120.0, 151.0, 181.0, 212.0, 243.0, 273.0, 304.0, 334.0,
        ],
        [
            0.0, 31.0, 60.0, 91.0, 121.0, 152.0, 182.0, 213.0, 244.0, 274.0, 305.0, 335.0,
        ],
    ];

    let yd: f64;
    let md: f64;
    let im: c_int;

    let mut y = y;
    let mut m = m;

    y += floor(m / 12.0);
    m = pmod(m, 12.0);

    im = d2i(m);
    if im < 0 || im >= 12 {
        return NAN;
    }

    yd = floor(TimeFromYear(d2i(y)) / msPerDay);
    md = firstDayOfMonth[(DaysInYear(d2i(y)) == 366) as usize][im as usize];

    yd + md + date - 1.0
}

unsafe fn MakeDate(day: f64, time: f64) -> f64 {
    day * msPerDay + time
}

unsafe fn TimeClip(t: f64) -> f64 {
    if !isfinite(t) {
        return NAN;
    }
    if fabs(t) > 8.64e15 {
        return NAN;
    }
    if t < 0.0 {
        -floor(-t)
    } else {
        floor(t)
    }
}

unsafe fn toint(sp: *mut *const c_char, w: c_int, v: *mut c_int) -> c_int {
    let mut s: *const c_char = *sp;
    let mut w = w;
    *v = 0;
    while {
        let __t = w;
        w -= 1;
        __t != 0
    } {
        if *s < '0' as c_char || *s > '9' as c_char {
            return 0;
        }
        *v = *v * 10 + (*s as c_int - '0' as c_int);
        s = s.add(1);
    }
    *sp = s;
    1
}

unsafe fn parseDateTime(s: *const c_char) -> f64 {
    let mut s = s;
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

    if toint(&mut s, 4, &mut y) == 0 {
        return NAN;
    }
    if *s == '-' as c_char {
        s = s.add(1);
        if toint(&mut s, 2, &mut m) == 0 {
            return NAN;
        }
        if *s == '-' as c_char {
            s = s.add(1);
            if toint(&mut s, 2, &mut d) == 0 {
                return NAN;
            }
        }
    }

    if *s == 'T' as c_char {
        s = s.add(1);
        if toint(&mut s, 2, &mut H) == 0 {
            return NAN;
        }
        if *s != ':' as c_char {
            return NAN;
        }
        s = s.add(1);
        if toint(&mut s, 2, &mut M) == 0 {
            return NAN;
        }
        if *s == ':' as c_char {
            s = s.add(1);
            if toint(&mut s, 2, &mut S) == 0 {
                return NAN;
            }
            if *s == '.' as c_char {
                s = s.add(1);
                if toint(&mut s, 3, &mut ms) == 0 {
                    return NAN;
                }
            }
        }
        if *s == 'Z' as c_char {
            s = s.add(1);
            tza = 0;
        } else if *s == '+' as c_char || *s == '-' as c_char {
            let mut tzh: c_int = 0;
            let mut tzm: c_int = 0;
            let tzs: c_int = if *s == '+' as c_char { 1 } else { -1 };
            s = s.add(1);
            if toint(&mut s, 2, &mut tzh) == 0 {
                return NAN;
            }
            if *s == ':' as c_char {
                s = s.add(1);
                if toint(&mut s, 2, &mut tzm) == 0 {
                    return NAN;
                }
            }
            if tzh > 23 || tzm > 59 {
                return NAN;
            }
            tza = d2i(tzs as f64 * (tzh as f64 * msPerHour + tzm as f64 * msPerMinute));
        } else {
            tza = d2i(LocalTZA());
        }
    }

    if *s != 0 {
        return NAN;
    }

    if m < 1 || m > 12 {
        return NAN;
    }
    if d < 1 || d > 31 {
        return NAN;
    }
    if H < 0 || H > 24 {
        return NAN;
    }
    if M < 0 || M > 59 {
        return NAN;
    }
    if S < 0 || S > 59 {
        return NAN;
    }
    if ms < 0 || ms > 999 {
        return NAN;
    }
    if H == 24 && (M != 0 || S != 0 || ms != 0) {
        return NAN;
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
    if !isfinite(t) {
        return cs!("Invalid Date") as *mut c_char;
    }
    sprintf(buf, cs!("%04d-%02d-%02d"), y, m + 1, d);
    buf
}

unsafe fn fmttime(buf: *mut c_char, t: f64, tza: f64) -> *mut c_char {
    let H = HourFromTime(t);
    let M = MinFromTime(t);
    let S = SecFromTime(t);
    let ms = msFromTime(t);
    let tzh = HourFromTime(fabs(tza));
    let tzm = MinFromTime(fabs(tza));
    if !isfinite(t) {
        return cs!("Invalid Date") as *mut c_char;
    }
    if tza == 0.0 {
        sprintf(buf, cs!("%02d:%02d:%02d.%03dZ"), H, M, S, ms);
    } else if tza < 0.0 {
        sprintf(
            buf,
            cs!("%02d:%02d:%02d.%03d-%02d:%02d"),
            H,
            M,
            S,
            ms,
            tzh,
            tzm,
        );
    } else {
        sprintf(
            buf,
            cs!("%02d:%02d:%02d.%03d+%02d:%02d"),
            H,
            M,
            S,
            ms,
            tzh,
            tzm,
        );
    }
    buf
}

unsafe fn fmtdatetime(buf: *mut c_char, t: f64, tza: f64) -> *mut c_char {
    let mut dbuf: [c_char; 20] = [0; 20];
    let mut tbuf: [c_char; 20] = [0; 20];
    if !isfinite(t) {
        return cs!("Invalid Date") as *mut c_char;
    }
    fmtdate(dbuf.as_mut_ptr(), t);
    fmttime(tbuf.as_mut_ptr(), t, tza);
    sprintf(
        buf,
        cs!("%sT%s"),
        dbuf.as_ptr() as *const c_char,
        tbuf.as_ptr() as *const c_char,
    );
    buf
}

/* Date functions */

unsafe fn js_todate(J: *mut js_State, idx: c_int) -> f64 {
    let self_: *mut js_Object = js_toobject(J, idx);
    if (*self_).type_ != JS_CDATE {
        js_typeerror!(J, "not a date");
    }
    (*self_).u.number
}

unsafe fn js_setdate(J: *mut js_State, idx: c_int, t: f64) {
    let self_: *mut js_Object = js_toobject(J, idx);
    if (*self_).type_ != JS_CDATE {
        js_typeerror!(J, "not a date");
    }
    (*self_).u.number = TimeClip(t);
    js_pushnumber(J, (*self_).u.number);
}

unsafe extern "C" fn D_parse(J: *mut js_State) {
    let t = parseDateTime(js_tostring(J, 1));
    js_pushnumber(J, t);
}

unsafe extern "C" fn D_UTC(J: *mut js_State) {
    let m: f64;
    let d: f64;
    let H: f64;
    let M: f64;
    let S: f64;
    let ms: f64;
    let mut t: f64;
    let mut y: f64 = js_tonumber(J, 1);
    if y < 100.0 {
        y += 1900.0;
    }
    m = js_tonumber(J, 2);
    d = js_optnumber!(J, 3, 1.0);
    H = js_optnumber!(J, 4, 0.0);
    M = js_optnumber!(J, 5, 0.0);
    S = js_optnumber!(J, 6, 0.0);
    ms = js_optnumber!(J, 7, 0.0);
    t = MakeDate(MakeDay(y, m, d), MakeTime(H, M, S, ms));
    t = TimeClip(t);
    js_pushnumber(J, t);
}

unsafe extern "C" fn D_now(J: *mut js_State) {
    js_pushnumber(J, Now());
}

unsafe extern "C" fn jsB_Date(J: *mut js_State) {
    let mut buf: [c_char; 64] = [0; 64];
    js_pushstring(
        J,
        fmtdatetime(buf.as_mut_ptr(), LocalTime(Now()), LocalTZA()),
    );
}

unsafe extern "C" fn jsB_new_Date(J: *mut js_State) {
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
        let m: f64;
        let d: f64;
        let H: f64;
        let M: f64;
        let S: f64;
        let ms: f64;
        let mut y = js_tonumber(J, 1);
        if y < 100.0 {
            y += 1900.0;
        }
        m = js_tonumber(J, 2);
        d = js_optnumber!(J, 3, 1.0);
        H = js_optnumber!(J, 4, 0.0);
        M = js_optnumber!(J, 5, 0.0);
        S = js_optnumber!(J, 6, 0.0);
        ms = js_optnumber!(J, 7, 0.0);
        let t0 = MakeDate(MakeDay(y, m, d), MakeTime(H, M, S, ms));
        t = TimeClip(UTC(t0));
    }

    obj = jsV_newobject(J, JS_CDATE, (*J).Date_prototype);
    (*obj).u.number = t;

    js_pushobject(J, obj);
}

unsafe extern "C" fn Dp_valueOf(J: *mut js_State) {
    let t = js_todate(J, 0);
    js_pushnumber(J, t);
}

unsafe extern "C" fn Dp_toString(J: *mut js_State) {
    let mut buf: [c_char; 64] = [0; 64];
    let t = js_todate(J, 0);
    js_pushstring(J, fmtdatetime(buf.as_mut_ptr(), LocalTime(t), LocalTZA()));
}

unsafe extern "C" fn Dp_toDateString(J: *mut js_State) {
    let mut buf: [c_char; 64] = [0; 64];
    let t = js_todate(J, 0);
    js_pushstring(J, fmtdate(buf.as_mut_ptr(), LocalTime(t)));
}

unsafe extern "C" fn Dp_toTimeString(J: *mut js_State) {
    let mut buf: [c_char; 64] = [0; 64];
    let t = js_todate(J, 0);
    js_pushstring(J, fmttime(buf.as_mut_ptr(), LocalTime(t), LocalTZA()));
}

unsafe extern "C" fn Dp_toUTCString(J: *mut js_State) {
    let mut buf: [c_char; 64] = [0; 64];
    let t = js_todate(J, 0);
    js_pushstring(J, fmtdatetime(buf.as_mut_ptr(), t, 0.0));
}

unsafe extern "C" fn Dp_toISOString(J: *mut js_State) {
    let mut buf: [c_char; 64] = [0; 64];
    let t = js_todate(J, 0);
    if !isfinite(t) {
        js_rangeerror!(J, "invalid date");
    }
    js_pushstring(J, fmtdatetime(buf.as_mut_ptr(), t, 0.0));
}

unsafe extern "C" fn Dp_getFullYear(J: *mut js_State) {
    let t = js_todate(J, 0);
    if isnan(t) {
        js_pushnumber(J, NAN);
    } else {
        js_pushnumber(J, YearFromTime(LocalTime(t)) as f64);
    }
}

unsafe extern "C" fn Dp_getMonth(J: *mut js_State) {
    let t = js_todate(J, 0);
    if isnan(t) {
        js_pushnumber(J, NAN);
    } else {
        js_pushnumber(J, MonthFromTime(LocalTime(t)) as f64);
    }
}

unsafe extern "C" fn Dp_getDate(J: *mut js_State) {
    let t = js_todate(J, 0);
    if isnan(t) {
        js_pushnumber(J, NAN);
    } else {
        js_pushnumber(J, DateFromTime(LocalTime(t)) as f64);
    }
}

unsafe extern "C" fn Dp_getDay(J: *mut js_State) {
    let t = js_todate(J, 0);
    if isnan(t) {
        js_pushnumber(J, NAN);
    } else {
        js_pushnumber(J, WeekDay(LocalTime(t)) as f64);
    }
}

unsafe extern "C" fn Dp_getHours(J: *mut js_State) {
    let t = js_todate(J, 0);
    if isnan(t) {
        js_pushnumber(J, NAN);
    } else {
        js_pushnumber(J, HourFromTime(LocalTime(t)) as f64);
    }
}

unsafe extern "C" fn Dp_getMinutes(J: *mut js_State) {
    let t = js_todate(J, 0);
    if isnan(t) {
        js_pushnumber(J, NAN);
    } else {
        js_pushnumber(J, MinFromTime(LocalTime(t)) as f64);
    }
}

unsafe extern "C" fn Dp_getSeconds(J: *mut js_State) {
    let t = js_todate(J, 0);
    if isnan(t) {
        js_pushnumber(J, NAN);
    } else {
        js_pushnumber(J, SecFromTime(LocalTime(t)) as f64);
    }
}

unsafe extern "C" fn Dp_getMilliseconds(J: *mut js_State) {
    let t = js_todate(J, 0);
    if isnan(t) {
        js_pushnumber(J, NAN);
    } else {
        js_pushnumber(J, msFromTime(LocalTime(t)) as f64);
    }
}

unsafe extern "C" fn Dp_getUTCFullYear(J: *mut js_State) {
    let t = js_todate(J, 0);
    if isnan(t) {
        js_pushnumber(J, NAN);
    } else {
        js_pushnumber(J, YearFromTime(t) as f64);
    }
}

unsafe extern "C" fn Dp_getUTCMonth(J: *mut js_State) {
    let t = js_todate(J, 0);
    if isnan(t) {
        js_pushnumber(J, NAN);
    } else {
        js_pushnumber(J, MonthFromTime(t) as f64);
    }
}

unsafe extern "C" fn Dp_getUTCDate(J: *mut js_State) {
    let t = js_todate(J, 0);
    if isnan(t) {
        js_pushnumber(J, NAN);
    } else {
        js_pushnumber(J, DateFromTime(t) as f64);
    }
}

unsafe extern "C" fn Dp_getUTCDay(J: *mut js_State) {
    let t = js_todate(J, 0);
    if isnan(t) {
        js_pushnumber(J, NAN);
    } else {
        js_pushnumber(J, WeekDay(t) as f64);
    }
}

unsafe extern "C" fn Dp_getUTCHours(J: *mut js_State) {
    let t = js_todate(J, 0);
    if isnan(t) {
        js_pushnumber(J, NAN);
    } else {
        js_pushnumber(J, HourFromTime(t) as f64);
    }
}

unsafe extern "C" fn Dp_getUTCMinutes(J: *mut js_State) {
    let t = js_todate(J, 0);
    if isnan(t) {
        js_pushnumber(J, NAN);
    } else {
        js_pushnumber(J, MinFromTime(t) as f64);
    }
}

unsafe extern "C" fn Dp_getUTCSeconds(J: *mut js_State) {
    let t = js_todate(J, 0);
    if isnan(t) {
        js_pushnumber(J, NAN);
    } else {
        js_pushnumber(J, SecFromTime(t) as f64);
    }
}

unsafe extern "C" fn Dp_getUTCMilliseconds(J: *mut js_State) {
    let t = js_todate(J, 0);
    if isnan(t) {
        js_pushnumber(J, NAN);
    } else {
        js_pushnumber(J, msFromTime(t) as f64);
    }
}

unsafe extern "C" fn Dp_getTimezoneOffset(J: *mut js_State) {
    let t = js_todate(J, 0);
    if isnan(t) {
        js_pushnumber(J, NAN);
    } else {
        js_pushnumber(J, (t - LocalTime(t)) / msPerMinute);
    }
}

unsafe extern "C" fn Dp_setTime(J: *mut js_State) {
    js_setdate(J, 0, js_tonumber(J, 1));
}

unsafe extern "C" fn Dp_setMilliseconds(J: *mut js_State) {
    let t = LocalTime(js_todate(J, 0));
    let h = HourFromTime(t) as f64;
    let m = MinFromTime(t) as f64;
    let s = SecFromTime(t) as f64;
    let ms = js_tonumber(J, 1);
    js_setdate(J, 0, UTC(MakeDate(Day(t) as f64, MakeTime(h, m, s, ms))));
}

unsafe extern "C" fn Dp_setSeconds(J: *mut js_State) {
    let t = LocalTime(js_todate(J, 0));
    let h = HourFromTime(t) as f64;
    let m = MinFromTime(t) as f64;
    let s = js_tonumber(J, 1);
    let ms = js_optnumber!(J, 2, msFromTime(t) as f64);
    js_setdate(J, 0, UTC(MakeDate(Day(t) as f64, MakeTime(h, m, s, ms))));
}

unsafe extern "C" fn Dp_setMinutes(J: *mut js_State) {
    let t = LocalTime(js_todate(J, 0));
    let h = HourFromTime(t) as f64;
    let m = js_tonumber(J, 1);
    let s = js_optnumber!(J, 2, SecFromTime(t) as f64);
    let ms = js_optnumber!(J, 3, msFromTime(t) as f64);
    js_setdate(J, 0, UTC(MakeDate(Day(t) as f64, MakeTime(h, m, s, ms))));
}

unsafe extern "C" fn Dp_setHours(J: *mut js_State) {
    let t = LocalTime(js_todate(J, 0));
    let h = js_tonumber(J, 1);
    let m = js_optnumber!(J, 2, MinFromTime(t) as f64);
    let s = js_optnumber!(J, 3, SecFromTime(t) as f64);
    let ms = js_optnumber!(J, 4, msFromTime(t) as f64);
    js_setdate(J, 0, UTC(MakeDate(Day(t) as f64, MakeTime(h, m, s, ms))));
}

unsafe extern "C" fn Dp_setDate(J: *mut js_State) {
    let t = LocalTime(js_todate(J, 0));
    let y = YearFromTime(t) as f64;
    let m = MonthFromTime(t) as f64;
    let d = js_tonumber(J, 1);
    js_setdate(J, 0, UTC(MakeDate(MakeDay(y, m, d), TimeWithinDay(t))));
}

unsafe extern "C" fn Dp_setMonth(J: *mut js_State) {
    let t = LocalTime(js_todate(J, 0));
    let y = YearFromTime(t) as f64;
    let m = js_tonumber(J, 1);
    let d = js_optnumber!(J, 2, DateFromTime(t) as f64);
    js_setdate(J, 0, UTC(MakeDate(MakeDay(y, m, d), TimeWithinDay(t))));
}

unsafe extern "C" fn Dp_setFullYear(J: *mut js_State) {
    let t = LocalTime(js_todate(J, 0));
    let y = js_tonumber(J, 1);
    let m = js_optnumber!(J, 2, MonthFromTime(t) as f64);
    let d = js_optnumber!(J, 3, DateFromTime(t) as f64);
    js_setdate(J, 0, UTC(MakeDate(MakeDay(y, m, d), TimeWithinDay(t))));
}

unsafe extern "C" fn Dp_setUTCMilliseconds(J: *mut js_State) {
    let t = js_todate(J, 0);
    let h = HourFromTime(t) as f64;
    let m = MinFromTime(t) as f64;
    let s = SecFromTime(t) as f64;
    let ms = js_tonumber(J, 1);
    js_setdate(J, 0, MakeDate(Day(t) as f64, MakeTime(h, m, s, ms)));
}

unsafe extern "C" fn Dp_setUTCSeconds(J: *mut js_State) {
    let t = js_todate(J, 0);
    let h = HourFromTime(t) as f64;
    let m = MinFromTime(t) as f64;
    let s = js_tonumber(J, 1);
    let ms = js_optnumber!(J, 2, msFromTime(t) as f64);
    js_setdate(J, 0, MakeDate(Day(t) as f64, MakeTime(h, m, s, ms)));
}

unsafe extern "C" fn Dp_setUTCMinutes(J: *mut js_State) {
    let t = js_todate(J, 0);
    let h = HourFromTime(t) as f64;
    let m = js_tonumber(J, 1);
    let s = js_optnumber!(J, 2, SecFromTime(t) as f64);
    let ms = js_optnumber!(J, 3, msFromTime(t) as f64);
    js_setdate(J, 0, MakeDate(Day(t) as f64, MakeTime(h, m, s, ms)));
}

unsafe extern "C" fn Dp_setUTCHours(J: *mut js_State) {
    let t = js_todate(J, 0);
    let h = js_tonumber(J, 1);
    let m = js_optnumber!(J, 2, HourFromTime(t) as f64);
    let s = js_optnumber!(J, 3, SecFromTime(t) as f64);
    let ms = js_optnumber!(J, 4, msFromTime(t) as f64);
    js_setdate(J, 0, MakeDate(Day(t) as f64, MakeTime(h, m, s, ms)));
}

unsafe extern "C" fn Dp_setUTCDate(J: *mut js_State) {
    let t = js_todate(J, 0);
    let y = YearFromTime(t) as f64;
    let m = MonthFromTime(t) as f64;
    let d = js_tonumber(J, 1);
    js_setdate(J, 0, MakeDate(MakeDay(y, m, d), TimeWithinDay(t)));
}

unsafe extern "C" fn Dp_setUTCMonth(J: *mut js_State) {
    let t = js_todate(J, 0);
    let y = YearFromTime(t) as f64;
    let m = js_tonumber(J, 1);
    let d = js_optnumber!(J, 2, DateFromTime(t) as f64);
    js_setdate(J, 0, MakeDate(MakeDay(y, m, d), TimeWithinDay(t)));
}

unsafe extern "C" fn Dp_setUTCFullYear(J: *mut js_State) {
    let t = js_todate(J, 0);
    let y = js_tonumber(J, 1);
    let m = js_optnumber!(J, 2, MonthFromTime(t) as f64);
    let d = js_optnumber!(J, 3, DateFromTime(t) as f64);
    js_setdate(J, 0, MakeDate(MakeDay(y, m, d), TimeWithinDay(t)));
}

unsafe extern "C" fn Dp_toJSON(J: *mut js_State) {
    js_copy(J, 0);
    js_toprimitive(J, -1, JS_HNUMBER);
    if js_isnumber(J, -1) != 0 && !isfinite(js_tonumber(J, -1)) {
        js_pushnull(J);
        return;
    }
    js_pop(J, 1);

    js_getproperty(J, 0, cs!("toISOString"));
    if js_iscallable(J, -1) == 0 {
        js_typeerror!(J, "this.toISOString is not a function");
    }
    js_copy(J, 0);
    js_call(J, 0);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsB_initdate(J: *mut js_State) {
    (*(*J).Date_prototype).u.number = 0.0;

    js_pushobject(J, (*J).Date_prototype);
    {
        jsB_propf(J, cs!("Date.prototype.valueOf"), Some(Dp_valueOf), 0);
        jsB_propf(J, cs!("Date.prototype.toString"), Some(Dp_toString), 0);
        jsB_propf(
            J,
            cs!("Date.prototype.toDateString"),
            Some(Dp_toDateString),
            0,
        );
        jsB_propf(
            J,
            cs!("Date.prototype.toTimeString"),
            Some(Dp_toTimeString),
            0,
        );
        jsB_propf(J, cs!("Date.prototype.toLocaleString"), Some(Dp_toString), 0);
        jsB_propf(
            J,
            cs!("Date.prototype.toLocaleDateString"),
            Some(Dp_toDateString),
            0,
        );
        jsB_propf(
            J,
            cs!("Date.prototype.toLocaleTimeString"),
            Some(Dp_toTimeString),
            0,
        );
        jsB_propf(J, cs!("Date.prototype.toUTCString"), Some(Dp_toUTCString), 0);

        jsB_propf(J, cs!("Date.prototype.getTime"), Some(Dp_valueOf), 0);
        jsB_propf(J, cs!("Date.prototype.getFullYear"), Some(Dp_getFullYear), 0);
        jsB_propf(
            J,
            cs!("Date.prototype.getUTCFullYear"),
            Some(Dp_getUTCFullYear),
            0,
        );
        jsB_propf(J, cs!("Date.prototype.getMonth"), Some(Dp_getMonth), 0);
        jsB_propf(J, cs!("Date.prototype.getUTCMonth"), Some(Dp_getUTCMonth), 0);
        jsB_propf(J, cs!("Date.prototype.getDate"), Some(Dp_getDate), 0);
        jsB_propf(J, cs!("Date.prototype.getUTCDate"), Some(Dp_getUTCDate), 0);
        jsB_propf(J, cs!("Date.prototype.getDay"), Some(Dp_getDay), 0);
        jsB_propf(J, cs!("Date.prototype.getUTCDay"), Some(Dp_getUTCDay), 0);
        jsB_propf(J, cs!("Date.prototype.getHours"), Some(Dp_getHours), 0);
        jsB_propf(J, cs!("Date.prototype.getUTCHours"), Some(Dp_getUTCHours), 0);
        jsB_propf(J, cs!("Date.prototype.getMinutes"), Some(Dp_getMinutes), 0);
        jsB_propf(
            J,
            cs!("Date.prototype.getUTCMinutes"),
            Some(Dp_getUTCMinutes),
            0,
        );
        jsB_propf(J, cs!("Date.prototype.getSeconds"), Some(Dp_getSeconds), 0);
        jsB_propf(
            J,
            cs!("Date.prototype.getUTCSeconds"),
            Some(Dp_getUTCSeconds),
            0,
        );
        jsB_propf(
            J,
            cs!("Date.prototype.getMilliseconds"),
            Some(Dp_getMilliseconds),
            0,
        );
        jsB_propf(
            J,
            cs!("Date.prototype.getUTCMilliseconds"),
            Some(Dp_getUTCMilliseconds),
            0,
        );
        jsB_propf(
            J,
            cs!("Date.prototype.getTimezoneOffset"),
            Some(Dp_getTimezoneOffset),
            0,
        );

        jsB_propf(J, cs!("Date.prototype.setTime"), Some(Dp_setTime), 1);
        jsB_propf(
            J,
            cs!("Date.prototype.setMilliseconds"),
            Some(Dp_setMilliseconds),
            1,
        );
        jsB_propf(
            J,
            cs!("Date.prototype.setUTCMilliseconds"),
            Some(Dp_setUTCMilliseconds),
            1,
        );
        jsB_propf(J, cs!("Date.prototype.setSeconds"), Some(Dp_setSeconds), 2);
        jsB_propf(
            J,
            cs!("Date.prototype.setUTCSeconds"),
            Some(Dp_setUTCSeconds),
            2,
        );
        jsB_propf(J, cs!("Date.prototype.setMinutes"), Some(Dp_setMinutes), 3);
        jsB_propf(
            J,
            cs!("Date.prototype.setUTCMinutes"),
            Some(Dp_setUTCMinutes),
            3,
        );
        jsB_propf(J, cs!("Date.prototype.setHours"), Some(Dp_setHours), 4);
        jsB_propf(J, cs!("Date.prototype.setUTCHours"), Some(Dp_setUTCHours), 4);
        jsB_propf(J, cs!("Date.prototype.setDate"), Some(Dp_setDate), 1);
        jsB_propf(J, cs!("Date.prototype.setUTCDate"), Some(Dp_setUTCDate), 1);
        jsB_propf(J, cs!("Date.prototype.setMonth"), Some(Dp_setMonth), 2);
        jsB_propf(J, cs!("Date.prototype.setUTCMonth"), Some(Dp_setUTCMonth), 2);
        jsB_propf(J, cs!("Date.prototype.setFullYear"), Some(Dp_setFullYear), 3);
        jsB_propf(
            J,
            cs!("Date.prototype.setUTCFullYear"),
            Some(Dp_setUTCFullYear),
            3,
        );

        /* ES5 */
        jsB_propf(J, cs!("Date.prototype.toISOString"), Some(Dp_toISOString), 0);
        jsB_propf(J, cs!("Date.prototype.toJSON"), Some(Dp_toJSON), 1);
    }
    js_newcconstructor(J, Some(jsB_Date), Some(jsB_new_Date), cs!("Date"), 0); /* 1 */
    {
        jsB_propf(J, cs!("Date.parse"), Some(D_parse), 1);
        jsB_propf(J, cs!("Date.UTC"), Some(D_UTC), 7);

        /* ES5 */
        jsB_propf(J, cs!("Date.now"), Some(D_now), 0);
    }
    js_defglobal(J, cs!("Date"), JS_DONTENUM);
}
