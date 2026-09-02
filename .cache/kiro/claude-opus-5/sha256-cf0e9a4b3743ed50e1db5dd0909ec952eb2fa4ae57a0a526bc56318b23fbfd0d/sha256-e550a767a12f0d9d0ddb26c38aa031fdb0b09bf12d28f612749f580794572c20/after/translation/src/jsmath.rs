//! Translation of src/jsmath.c
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(unused)]

use crate::jsi::*;

use crate::jsbuiltin::{jsB_propf, jsB_propn};
use crate::jsproperty::jsV_newobject;
use crate::jsrun::{
    js_defglobal, js_gettop, js_pushnumber, js_pushobject, js_tonumber,
};

unsafe fn jsM_round(x: f64) -> f64 {
    unsafe {
        if isnan(x) { return x; }
        if isinf(x) { return x; }
        if x == 0.0 { return x; }
        if x > 0.0 && x < 0.5 { return 0.0; }
        if x < 0.0 && x >= -0.5 { return -0.0; }
        floor(x + 0.5)
    }
}

unsafe extern "C-unwind" fn Math_abs(J: *mut js_State) {
    unsafe {
        js_pushnumber(J, fabs(js_tonumber(J, 1)));
    }
}

unsafe extern "C-unwind" fn Math_acos(J: *mut js_State) {
    unsafe {
        js_pushnumber(J, acos(js_tonumber(J, 1)));
    }
}

unsafe extern "C-unwind" fn Math_asin(J: *mut js_State) {
    unsafe {
        js_pushnumber(J, asin(js_tonumber(J, 1)));
    }
}

unsafe extern "C-unwind" fn Math_atan(J: *mut js_State) {
    unsafe {
        js_pushnumber(J, atan(js_tonumber(J, 1)));
    }
}

unsafe extern "C-unwind" fn Math_atan2(J: *mut js_State) {
    unsafe {
        let y = js_tonumber(J, 1);
        let x = js_tonumber(J, 2);
        js_pushnumber(J, atan2(y, x));
    }
}

unsafe extern "C-unwind" fn Math_ceil(J: *mut js_State) {
    unsafe {
        js_pushnumber(J, ceil(js_tonumber(J, 1)));
    }
}

unsafe extern "C-unwind" fn Math_cos(J: *mut js_State) {
    unsafe {
        js_pushnumber(J, cos(js_tonumber(J, 1)));
    }
}

unsafe extern "C-unwind" fn Math_exp(J: *mut js_State) {
    unsafe {
        js_pushnumber(J, exp(js_tonumber(J, 1)));
    }
}

unsafe extern "C-unwind" fn Math_floor(J: *mut js_State) {
    unsafe {
        js_pushnumber(J, floor(js_tonumber(J, 1)));
    }
}

unsafe extern "C-unwind" fn Math_log(J: *mut js_State) {
    unsafe {
        js_pushnumber(J, log(js_tonumber(J, 1)));
    }
}

unsafe extern "C-unwind" fn Math_pow(J: *mut js_State) {
    unsafe {
        let x = js_tonumber(J, 1);
        let y = js_tonumber(J, 2);
        if !isfinite(y) && fabs(x) == 1.0 {
            js_pushnumber(J, NAN);
        } else {
            js_pushnumber(J, pow(x, y));
        }
    }
}

unsafe extern "C-unwind" fn Math_random(J: *mut js_State) {
    unsafe {
        /* Lehmer generator with a=48271 and m=2^31-1 */
        /* Park & Miller (1988). Random Number Generators: Good ones are hard to find. */
        (*J).seed = ((((*J).seed as u64).wrapping_mul(48271)) % 0x7fffffff) as c_uint;
        js_pushnumber(J, (*J).seed as f64 / 0x7fffffff as f64);
    }
}

unsafe fn Math_init_random(J: *mut js_State) {
    unsafe {
        /* Pick initial seed by scrambling current time with Xorshift. */
        /* Marsaglia (2003). Xorshift RNGs. */
        (*J).seed = (time(core::ptr::null_mut()) as c_uint).wrapping_add(123);
        (*J).seed ^= (*J).seed << 13;
        (*J).seed ^= (*J).seed >> 17;
        (*J).seed ^= (*J).seed << 5;
        (*J).seed %= 0x7fffffff;
    }
}

unsafe extern "C-unwind" fn Math_round(J: *mut js_State) {
    unsafe {
        let x = js_tonumber(J, 1);
        js_pushnumber(J, jsM_round(x));
    }
}

unsafe extern "C-unwind" fn Math_sin(J: *mut js_State) {
    unsafe {
        js_pushnumber(J, sin(js_tonumber(J, 1)));
    }
}

unsafe extern "C-unwind" fn Math_sqrt(J: *mut js_State) {
    unsafe {
        js_pushnumber(J, sqrt(js_tonumber(J, 1)));
    }
}

unsafe extern "C-unwind" fn Math_tan(J: *mut js_State) {
    unsafe {
        js_pushnumber(J, tan(js_tonumber(J, 1)));
    }
}

unsafe extern "C-unwind" fn Math_max(J: *mut js_State) {
    unsafe {
        let n = js_gettop(J);
        let mut x = -INFINITY;
        let mut i = 1;
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
}

unsafe extern "C-unwind" fn Math_min(J: *mut js_State) {
    unsafe {
        let n = js_gettop(J);
        let mut x = INFINITY;
        let mut i = 1;
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
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsB_initmath(J: *mut js_State) {
    unsafe {
        Math_init_random(J);
        js_pushobject(J, jsV_newobject(J, JS_CMATH, (*J).Object_prototype));
        {
            jsB_propn(J, c"E".as_ptr(), 2.7182818284590452354);
            jsB_propn(J, c"LN10".as_ptr(), 2.302585092994046);
            jsB_propn(J, c"LN2".as_ptr(), 0.6931471805599453);
            jsB_propn(J, c"LOG2E".as_ptr(), 1.4426950408889634);
            jsB_propn(J, c"LOG10E".as_ptr(), 0.4342944819032518);
            jsB_propn(J, c"PI".as_ptr(), 3.1415926535897932);
            jsB_propn(J, c"SQRT1_2".as_ptr(), 0.7071067811865476);
            jsB_propn(J, c"SQRT2".as_ptr(), 1.4142135623730951);

            jsB_propf(J, c"Math.abs".as_ptr(), Some(Math_abs), 1);
            jsB_propf(J, c"Math.acos".as_ptr(), Some(Math_acos), 1);
            jsB_propf(J, c"Math.asin".as_ptr(), Some(Math_asin), 1);
            jsB_propf(J, c"Math.atan".as_ptr(), Some(Math_atan), 1);
            jsB_propf(J, c"Math.atan2".as_ptr(), Some(Math_atan2), 2);
            jsB_propf(J, c"Math.ceil".as_ptr(), Some(Math_ceil), 1);
            jsB_propf(J, c"Math.cos".as_ptr(), Some(Math_cos), 1);
            jsB_propf(J, c"Math.exp".as_ptr(), Some(Math_exp), 1);
            jsB_propf(J, c"Math.floor".as_ptr(), Some(Math_floor), 1);
            jsB_propf(J, c"Math.log".as_ptr(), Some(Math_log), 1);
            jsB_propf(J, c"Math.max".as_ptr(), Some(Math_max), 0); /* 2 */
            jsB_propf(J, c"Math.min".as_ptr(), Some(Math_min), 0); /* 2 */
            jsB_propf(J, c"Math.pow".as_ptr(), Some(Math_pow), 2);
            jsB_propf(J, c"Math.random".as_ptr(), Some(Math_random), 0);
            jsB_propf(J, c"Math.round".as_ptr(), Some(Math_round), 1);
            jsB_propf(J, c"Math.sin".as_ptr(), Some(Math_sin), 1);
            jsB_propf(J, c"Math.sqrt".as_ptr(), Some(Math_sqrt), 1);
            jsB_propf(J, c"Math.tan".as_ptr(), Some(Math_tan), 1);
        }
        js_defglobal(J, c"Math".as_ptr(), JS_DONTENUM);
    }
}
