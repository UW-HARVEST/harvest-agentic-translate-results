//! Translation of jsmath.c

use crate::*;

unsafe fn jsM_round(x: f64) -> f64 {
    if isnan(x) {
        return x;
    }
    if isinf(x) {
        return x;
    }
    if x == 0.0 {
        return x;
    }
    if x > 0.0 && x < 0.5 {
        return 0.0;
    }
    if x < 0.0 && x >= -0.5 {
        /* C source says `return -0;` -- the integer constant 0 negated,
         * which converts to +0.0, not -0.0. Keep the same behaviour. */
        return 0.0;
    }
    floor(x + 0.5)
}

unsafe extern "C" fn Math_abs(J: *mut js_State) {
    js_pushnumber(J, fabs(js_tonumber(J, 1)));
}

unsafe extern "C" fn Math_acos(J: *mut js_State) {
    js_pushnumber(J, acos(js_tonumber(J, 1)));
}

unsafe extern "C" fn Math_asin(J: *mut js_State) {
    js_pushnumber(J, asin(js_tonumber(J, 1)));
}

unsafe extern "C" fn Math_atan(J: *mut js_State) {
    js_pushnumber(J, atan(js_tonumber(J, 1)));
}

unsafe extern "C" fn Math_atan2(J: *mut js_State) {
    let y = js_tonumber(J, 1);
    let x = js_tonumber(J, 2);
    js_pushnumber(J, atan2(y, x));
}

unsafe extern "C" fn Math_ceil(J: *mut js_State) {
    js_pushnumber(J, ceil(js_tonumber(J, 1)));
}

unsafe extern "C" fn Math_cos(J: *mut js_State) {
    js_pushnumber(J, cos(js_tonumber(J, 1)));
}

unsafe extern "C" fn Math_exp(J: *mut js_State) {
    js_pushnumber(J, exp(js_tonumber(J, 1)));
}

unsafe extern "C" fn Math_floor(J: *mut js_State) {
    js_pushnumber(J, floor(js_tonumber(J, 1)));
}

unsafe extern "C" fn Math_log(J: *mut js_State) {
    js_pushnumber(J, log(js_tonumber(J, 1)));
}

unsafe extern "C" fn Math_pow(J: *mut js_State) {
    let x = js_tonumber(J, 1);
    let y = js_tonumber(J, 2);
    if !isfinite(y) && fabs(x) == 1.0 {
        js_pushnumber(J, NAN);
    } else {
        js_pushnumber(J, pow(x, y));
    }
}

unsafe extern "C" fn Math_random(J: *mut js_State) {
    /* Lehmer generator with a=48271 and m=2^31-1 */
    /* Park & Miller (1988). Random Number Generators: Good ones are hard to find. */
    (*J).seed = (((*J).seed as u64).wrapping_mul(48271) % 0x7fffffff) as c_uint;
    js_pushnumber(J, (*J).seed as f64 / 0x7fffffff as f64);
}

unsafe fn Math_init_random(J: *mut js_State) {
    /* Pick initial seed by scrambling current time with Xorshift. */
    /* Marsaglia (2003). Xorshift RNGs. */
    (*J).seed = (time(null_mut()).wrapping_add(123)) as c_uint;
    (*J).seed ^= (*J).seed.wrapping_shl(13);
    (*J).seed ^= (*J).seed.wrapping_shr(17);
    (*J).seed ^= (*J).seed.wrapping_shl(5);
    (*J).seed %= 0x7fffffff;
}

unsafe extern "C" fn Math_round(J: *mut js_State) {
    let x = js_tonumber(J, 1);
    js_pushnumber(J, jsM_round(x));
}

unsafe extern "C" fn Math_sin(J: *mut js_State) {
    js_pushnumber(J, sin(js_tonumber(J, 1)));
}

unsafe extern "C" fn Math_sqrt(J: *mut js_State) {
    js_pushnumber(J, sqrt(js_tonumber(J, 1)));
}

unsafe extern "C" fn Math_tan(J: *mut js_State) {
    js_pushnumber(J, tan(js_tonumber(J, 1)));
}

unsafe extern "C" fn Math_max(J: *mut js_State) {
    let mut i: c_int;
    let n: c_int = js_gettop(J);
    let mut x: f64 = -INFINITY;
    i = 1;
    while i < n {
        let y = js_tonumber(J, i);
        if isnan(y) {
            x = y;
            break;
        }
        if signbit(x) == signbit(y) {
            x = if x > y { x } else { y };
        } else if signbit(x) {
            x = y;
        }
        i += 1;
    }
    js_pushnumber(J, x);
}

unsafe extern "C" fn Math_min(J: *mut js_State) {
    let mut i: c_int;
    let n: c_int = js_gettop(J);
    let mut x: f64 = INFINITY;
    i = 1;
    while i < n {
        let y = js_tonumber(J, i);
        if isnan(y) {
            x = y;
            break;
        }
        if signbit(x) == signbit(y) {
            x = if x < y { x } else { y };
        } else if signbit(y) {
            x = y;
        }
        i += 1;
    }
    js_pushnumber(J, x);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsB_initmath(J: *mut js_State) {
    Math_init_random(J);
    js_pushobject(J, jsV_newobject(J, JS_CMATH, (*J).Object_prototype));
    {
        jsB_propn(J, cs!("E"), 2.7182818284590452354);
        jsB_propn(J, cs!("LN10"), 2.302585092994046);
        jsB_propn(J, cs!("LN2"), 0.6931471805599453);
        jsB_propn(J, cs!("LOG2E"), 1.4426950408889634);
        jsB_propn(J, cs!("LOG10E"), 0.4342944819032518);
        jsB_propn(J, cs!("PI"), 3.1415926535897932);
        jsB_propn(J, cs!("SQRT1_2"), 0.7071067811865476);
        jsB_propn(J, cs!("SQRT2"), 1.4142135623730951);

        jsB_propf(J, cs!("Math.abs"), Some(Math_abs), 1);
        jsB_propf(J, cs!("Math.acos"), Some(Math_acos), 1);
        jsB_propf(J, cs!("Math.asin"), Some(Math_asin), 1);
        jsB_propf(J, cs!("Math.atan"), Some(Math_atan), 1);
        jsB_propf(J, cs!("Math.atan2"), Some(Math_atan2), 2);
        jsB_propf(J, cs!("Math.ceil"), Some(Math_ceil), 1);
        jsB_propf(J, cs!("Math.cos"), Some(Math_cos), 1);
        jsB_propf(J, cs!("Math.exp"), Some(Math_exp), 1);
        jsB_propf(J, cs!("Math.floor"), Some(Math_floor), 1);
        jsB_propf(J, cs!("Math.log"), Some(Math_log), 1);
        jsB_propf(J, cs!("Math.max"), Some(Math_max), 0); /* 2 */
        jsB_propf(J, cs!("Math.min"), Some(Math_min), 0); /* 2 */
        jsB_propf(J, cs!("Math.pow"), Some(Math_pow), 2);
        jsB_propf(J, cs!("Math.random"), Some(Math_random), 0);
        jsB_propf(J, cs!("Math.round"), Some(Math_round), 1);
        jsB_propf(J, cs!("Math.sin"), Some(Math_sin), 1);
        jsB_propf(J, cs!("Math.sqrt"), Some(Math_sqrt), 1);
        jsB_propf(J, cs!("Math.tan"), Some(Math_tan), 1);
    }
    js_defglobal(J, cs!("Math"), JS_DONTENUM);
}
