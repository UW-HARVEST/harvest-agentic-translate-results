//! Translated from jsarray.c — Array constructor and prototype methods.
#![allow(non_snake_case, non_upper_case_globals)]

use crate::cutil::*;
use crate::jsrun::*;
use crate::types::*;
use std::os::raw::{c_char, c_int, c_void};

macro_rules! cstr {
    ($s:literal) => {
        concat!($s, "\0").as_ptr() as *const c_char
    };
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_getlength(J: *mut js_State, idx: c_int) -> c_int {
    let len;
    js_getproperty(J, idx, cstr!("length"));
    len = js_tointeger(J, -1);
    js_pop(J, 1);
    len
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_setlength(J: *mut js_State, idx: c_int, len: c_int) {
    js_pushnumber(J, len as f64);
    js_setproperty(J, if idx < 0 { idx - 1 } else { idx }, cstr!("length"));
}

unsafe extern "C-unwind" fn jsB_new_Array(J: *mut js_State) {
    let mut i;
    let top = js_gettop(J);

    crate::jsvalue::js_newarray(J);

    if top == 2 {
        if js_isnumber(J, 1) != 0 {
            js_copy(J, 1);
            js_setproperty(J, -2, cstr!("length"));
        } else {
            js_copy(J, 1);
            js_setindex(J, -2, 0);
        }
    } else {
        i = 1;
        while i < top {
            js_copy(J, i);
            js_setindex(J, -2, i - 1);
            i += 1;
        }
    }
}

unsafe extern "C-unwind" fn Ap_concat(J: *mut js_State) {
    let mut i;
    let top = js_gettop(J);
    let mut n;
    let mut k;
    let mut len;

    crate::jsvalue::js_newarray(J);
    n = 0;

    i = 0;
    while i < top {
        js_copy(J, i);
        if js_isarray(J, -1) != 0 {
            len = js_getlength(J, -1);
            k = 0;
            while k < len {
                if js_hasindex(J, -1, k) != 0 {
                    js_setindex(J, -3, n);
                    n += 1;
                }
                k += 1;
            }
            js_pop(J, 1);
        } else {
            js_setindex(J, -2, n);
            n += 1;
        }
        i += 1;
    }
}

unsafe fn Ap_join_cycle(J: *mut js_State) -> c_int {
    let needle = js_toobject(J, 0);
    let mut top = (*J).tracetop - 1;
    while top > 0 {
        let stk = (*J).trace[top as usize].stack;
        let fun = (*J).stack.add((stk - 1) as usize);
        if (*fun).type_() != JS_TOBJECT {
            return 0;
        }
        if (*(*fun).u.object).type_ != JS_CCFUNCTION {
            return 0;
        }
        if (*(*fun).u.object).u.c.function == Some(Ap_join as unsafe extern "C-unwind" fn(*mut js_State)) {
            let obj = (*J).stack.add(stk as usize);
            if (*obj).type_() != JS_TOBJECT {
                return 0;
            }
            if (*obj).u.object == needle {
                return 1;
            }
        } else if (*(*fun).u.object).u.c.function == Some(Ap_toString as unsafe extern "C-unwind" fn(*mut js_State)) {
            /* join calls toString which calls join ... */
        } else {
            return 0;
        }
        top -= 1;
    }
    0
}

unsafe extern "C-unwind" fn Ap_join(J: *mut js_State) {
    let mut out: *mut c_char = std::ptr::null_mut();
    let mut r: *const c_char = std::ptr::null();
    let sep: *const c_char;
    let seplen;
    let mut k = 0;
    let mut n = 0;
    let len;
    let mut rlen = 0;

    if Ap_join_cycle(J) != 0 {
        js_pushliteral(J, cstr!(""));
        return;
    }

    len = js_getlength(J, 0);

    if js_isdefined(J, 1) != 0 {
        sep = js_tostring(J, 1);
        seplen = strlen(sep) as c_int;
    } else {
        sep = cstr!(",");
        seplen = 1;
    }

    if len <= 0 {
        js_pushliteral(J, cstr!(""));
        return;
    }

    let out_ptr = std::ptr::addr_of_mut!(out);
    let r_ptr = std::ptr::addr_of_mut!(r);
    let caught = protect(J, || {
        n = 0;
        k = 0;
        while k < len {
            js_getindex(J, 0, k);
            if js_iscoercible(J, -1) != 0 {
                *r_ptr = js_tostring(J, -1);
                rlen = strlen(*r_ptr) as c_int;
            } else {
                rlen = 0;
            }

            if k == 0 {
                *out_ptr = js_malloc(J, rlen + 1) as *mut c_char;
                if rlen > 0 {
                    memcpy(*out_ptr, *r_ptr, rlen as usize);
                    n += rlen;
                }
            } else {
                if n + seplen + rlen > JS_STRLIMIT {
                    crate::jserror::js_rangeerror(J, cstr!("invalid string length"));
                }
                *out_ptr = js_realloc(J, *out_ptr as *mut c_void, n + seplen + rlen + 1) as *mut c_char;
                if seplen > 0 {
                    memcpy((*out_ptr).add(n as usize), sep, seplen as usize);
                    n += seplen;
                }
                if rlen > 0 {
                    memcpy((*out_ptr).add(n as usize), *r_ptr, rlen as usize);
                    n += rlen;
                }
            }

            js_pop(J, 1);
            k += 1;
        }
        js_pushlstring(J, *out_ptr, n);
    });
    if caught {
        js_free(J, out as *mut c_void);
        js_throw(J);
    }
    js_endtry(J);
    js_free(J, out as *mut c_void);
}

unsafe extern "C-unwind" fn Ap_pop(J: *mut js_State) {
    let n = js_getlength(J, 0);
    if n > 0 {
        js_getindex(J, 0, n - 1);
        js_delindex(J, 0, n - 1);
        js_setlength(J, 0, n - 1);
    } else {
        js_setlength(J, 0, 0);
        js_pushundefined(J);
    }
}

unsafe extern "C-unwind" fn Ap_push(J: *mut js_State) {
    let mut i;
    let top = js_gettop(J);
    let mut n;

    n = js_getlength(J, 0);

    i = 1;
    while i < top {
        js_copy(J, i);
        js_setindex(J, 0, n);
        i += 1;
        n += 1;
    }

    js_setlength(J, 0, n);
    js_pushnumber(J, n as f64);
}

unsafe extern "C-unwind" fn Ap_reverse(J: *mut js_State) {
    let len;
    let middle;
    let mut lower;

    len = js_getlength(J, 0);
    middle = len / 2;
    lower = 0;

    while lower != middle {
        let upper = len - lower - 1;
        let haslower = js_hasindex(J, 0, lower);
        let hasupper = js_hasindex(J, 0, upper);
        if haslower != 0 && hasupper != 0 {
            js_setindex(J, 0, lower);
            js_setindex(J, 0, upper);
        } else if hasupper != 0 {
            js_setindex(J, 0, lower);
            js_delindex(J, 0, upper);
        } else if haslower != 0 {
            js_setindex(J, 0, upper);
            js_delindex(J, 0, lower);
        }
        lower += 1;
    }

    js_copy(J, 0);
}

unsafe extern "C-unwind" fn Ap_shift(J: *mut js_State) {
    let mut k;
    let len;

    len = js_getlength(J, 0);

    if len == 0 {
        js_setlength(J, 0, 0);
        js_pushundefined(J);
        return;
    }

    js_getindex(J, 0, 0);

    k = 1;
    while k < len {
        if js_hasindex(J, 0, k) != 0 {
            js_setindex(J, 0, k - 1);
        } else {
            js_delindex(J, 0, k - 1);
        }
        k += 1;
    }

    js_delindex(J, 0, len - 1);
    js_setlength(J, 0, len - 1);
}

unsafe extern "C-unwind" fn Ap_slice(J: *mut js_State) {
    let len;
    let mut s;
    let e;
    let mut n;
    let mut sv;
    let mut ev;

    crate::jsvalue::js_newarray(J);

    len = js_getlength(J, 0);
    sv = js_tointeger(J, 1) as f64;
    ev = if js_isdefined(J, 2) != 0 { js_tointeger(J, 2) as f64 } else { len as f64 };

    if sv < 0.0 {
        sv = sv + len as f64;
    }
    if ev < 0.0 {
        ev = ev + len as f64;
    }

    // jsarray.c:269/270 `s = sv < 0 ? 0 : sv > len ? len : sv;` (likewise e) — the conditional
    // expression has type double and is implicitly converted to `int` on assignment. Provably
    // in range and never NaN: sv/ev come from js_getlength()/js_tointeger(), both of which
    // return `int` (mujs.h:149/212), so sv, ev are integral doubles, and the casting branch is
    // only reached when 0 <= sv <= len <= INT_MAX. Plain `as` is identical to C.
    s = if sv < 0.0 { 0 } else if sv > len as f64 { len } else { sv as c_int };
    e = if ev < 0.0 { 0 } else if ev > len as f64 { len } else { ev as c_int };

    n = 0;
    while s < e {
        if js_hasindex(J, 0, s) != 0 {
            js_setindex(J, -2, n);
        }
        s += 1;
        n += 1;
    }
}

unsafe fn Ap_sort_cmp(J: *mut js_State, idx_a: c_int, idx_b: c_int) -> c_int {
    let obj = (*js_tovalue(J, 0)).u.object;
    if (*obj).u.a.simple != 0 && idx_b < (*obj).u.a.flat_length {
        let val_a = (*obj).u.a.array.add(idx_a as usize);
        let val_b = (*obj).u.a.array.add(idx_b as usize);
        let und_a = ((*val_a).type_() == JS_TUNDEFINED) as c_int;
        let und_b = ((*val_b).type_() == JS_TUNDEFINED) as c_int;
        if und_a != 0 {
            return und_b;
        }
        if und_b != 0 {
            return -1;
        }
        if js_iscallable(J, 1) != 0 {
            let v;
            js_copy(J, 1);
            js_pushundefined(J);
            js_pushvalue(J, *val_a);
            js_pushvalue(J, *val_b);
            js_call(J, 2);
            v = js_tonumber(J, -1);
            js_pop(J, 1);
            if v.is_nan() {
                return 0;
            }
            if v == 0.0 {
                return 0;
            }
            return if v < 0.0 { -1 } else { 1 };
        } else {
            let str_a;
            let str_b;
            let c;
            js_pushvalue(J, *val_a);
            js_pushvalue(J, *val_b);
            str_a = js_tostring(J, -2);
            str_b = js_tostring(J, -1);
            c = strcmp(str_a, str_b);
            js_pop(J, 2);
            return c;
        }
    } else {
        let und_a;
        let und_b;
        let has_a = js_hasindex(J, 0, idx_a);
        let has_b = js_hasindex(J, 0, idx_b);
        if has_a == 0 && has_b == 0 {
            return 0;
        }
        if has_a != 0 && has_b == 0 {
            js_pop(J, 1);
            return -1;
        }
        if has_a == 0 && has_b != 0 {
            js_pop(J, 1);
            return 1;
        }

        und_a = js_isundefined(J, -2);
        und_b = js_isundefined(J, -1);
        if und_a != 0 {
            js_pop(J, 2);
            return und_b;
        }
        if und_b != 0 {
            js_pop(J, 2);
            return -1;
        }

        if js_iscallable(J, 1) != 0 {
            let v;
            js_copy(J, 1);
            js_pushundefined(J);
            js_copy(J, -4);
            js_copy(J, -4);
            js_call(J, 2);
            v = js_tonumber(J, -1);
            js_pop(J, 3);
            if v.is_nan() {
                return 0;
            }
            if v == 0.0 {
                return 0;
            }
            return if v < 0.0 { -1 } else { 1 };
        } else {
            let str_a = js_tostring(J, -2);
            let str_b = js_tostring(J, -1);
            let c = strcmp(str_a, str_b);
            js_pop(J, 2);
            return c;
        }
    }
}

unsafe fn Ap_sort_swap(J: *mut js_State, idx_a: c_int, idx_b: c_int) {
    let obj = (*js_tovalue(J, 0)).u.object;
    if (*obj).u.a.simple != 0 && idx_b < (*obj).u.a.flat_length {
        let tmp = *(*obj).u.a.array.add(idx_a as usize);
        *(*obj).u.a.array.add(idx_a as usize) = *(*obj).u.a.array.add(idx_b as usize);
        *(*obj).u.a.array.add(idx_b as usize) = tmp;
    } else {
        let has_a = js_hasindex(J, 0, idx_a);
        let has_b = js_hasindex(J, 0, idx_b);
        if has_a != 0 && has_b != 0 {
            js_setindex(J, 0, idx_a);
            js_setindex(J, 0, idx_b);
        } else if has_a != 0 && has_b == 0 {
            js_delindex(J, 0, idx_a);
            js_setindex(J, 0, idx_b);
        } else if has_a == 0 && has_b != 0 {
            js_delindex(J, 0, idx_b);
            js_setindex(J, 0, idx_a);
        }
    }
}

unsafe fn Ap_sort_leaf(J: *mut js_State, i: c_int, end: c_int) -> c_int {
    let mut j = i;
    let mut lc = (j << 1) + 1;
    let mut rc = (j << 1) + 2;
    while rc < end {
        if Ap_sort_cmp(J, lc, rc) <= 0 {
            j = rc;
        } else {
            j = lc;
        }
        lc = (j << 1) + 1;
        rc = (j << 1) + 2;
    }
    if lc < end {
        j = lc;
    }
    j
}

unsafe fn Ap_sort_sift(J: *mut js_State, i: c_int, end: c_int) {
    let mut j = Ap_sort_leaf(J, i, end);
    while j > i && Ap_sort_cmp(J, i, j) > 0 {
        j = (j - 1) >> 1;
    }
    while j > i {
        Ap_sort_swap(J, i, j);
        j = (j - 1) >> 1;
    }
}

unsafe fn Ap_sort_heapsort(J: *mut js_State, n: c_int) {
    let mut i;
    i = n / 2 - 1;
    while i >= 0 {
        Ap_sort_sift(J, i, n);
        i -= 1;
    }
    i = n - 1;
    while i > 0 {
        Ap_sort_swap(J, 0, i);
        Ap_sort_sift(J, 0, i);
        i -= 1;
    }
}

unsafe extern "C-unwind" fn Ap_sort(J: *mut js_State) {
    let len;

    len = js_getlength(J, 0);
    if len <= 1 {
        js_copy(J, 0);
        return;
    }

    if js_iscallable(J, 1) == 0 && js_isundefined(J, 1) == 0 {
        crate::jserror::js_typeerror(J, cstr!("comparison function must be a function or undefined"));
    }

    if len >= c_int::MAX {
        crate::jserror::js_rangeerror(J, cstr!("array is too large to sort"));
    }

    Ap_sort_heapsort(J, len);

    js_copy(J, 0);
}

unsafe extern "C-unwind" fn Ap_splice(J: *mut js_State) {
    let top = js_gettop(J);
    let len;
    let mut start;
    let mut del;
    let add;
    let mut k;

    len = js_getlength(J, 0);
    start = js_tointeger(J, 1);
    if start < 0 {
        start = if (len + start) > 0 { len + start } else { 0 };
    } else if start > len {
        start = len;
    }

    if js_isdefined(J, 2) != 0 {
        del = js_tointeger(J, 2);
    } else {
        del = len - start;
    }
    if del > len - start {
        del = len - start;
    }
    if del < 0 {
        del = 0;
    }

    crate::jsvalue::js_newarray(J);

    k = 0;
    while k < del {
        if js_hasindex(J, 0, start + k) != 0 {
            js_setindex(J, -2, k);
        }
        k += 1;
    }
    js_setlength(J, -1, del);

    add = top - 3;
    if add < del {
        k = start;
        while k < len - del {
            if js_hasindex(J, 0, k + del) != 0 {
                js_setindex(J, 0, k + add);
            } else {
                js_delindex(J, 0, k + add);
            }
            k += 1;
        }
        k = len;
        while k > len - del + add {
            js_delindex(J, 0, k - 1);
            k -= 1;
        }
    } else if add > del {
        k = len - del;
        while k > start {
            if js_hasindex(J, 0, k + del - 1) != 0 {
                js_setindex(J, 0, k + add - 1);
            } else {
                js_delindex(J, 0, k + add - 1);
            }
            k -= 1;
        }
    }

    k = 0;
    while k < add {
        js_copy(J, 3 + k);
        js_setindex(J, 0, start + k);
        k += 1;
    }

    js_setlength(J, 0, len - del + add);
}

unsafe extern "C-unwind" fn Ap_unshift(J: *mut js_State) {
    let mut i;
    let top = js_gettop(J);
    let mut k;
    let len;

    len = js_getlength(J, 0);

    k = len;
    while k > 0 {
        let from = k - 1;
        let to = k + top - 2;
        if js_hasindex(J, 0, from) != 0 {
            js_setindex(J, 0, to);
        } else {
            js_delindex(J, 0, to);
        }
        k -= 1;
    }

    i = 1;
    while i < top {
        js_copy(J, i);
        js_setindex(J, 0, i - 1);
        i += 1;
    }

    js_setlength(J, 0, len + top - 1);
    js_pushnumber(J, (len + top - 1) as f64);
}

unsafe extern "C-unwind" fn Ap_toString(J: *mut js_State) {
    if js_iscoercible(J, 0) == 0 {
        crate::jserror::js_typeerror(J, cstr!("'this' is not an object"));
    }
    js_getproperty(J, 0, cstr!("join"));
    if js_iscallable(J, -1) == 0 {
        js_pop(J, 1);
        js_getglobal(J, cstr!("Object"));
        js_getproperty(J, -1, cstr!("prototype"));
        js_rot2pop1(J);
        js_getproperty(J, -1, cstr!("toString"));
        js_rot2pop1(J);
    }
    js_copy(J, 0);
    js_call(J, 0);
}

unsafe extern "C-unwind" fn Ap_indexOf(J: *mut js_State) {
    let mut k;
    let len;
    let mut from;

    len = js_getlength(J, 0);
    from = if js_isdefined(J, 2) != 0 { js_tointeger(J, 2) } else { 0 };
    if from < 0 {
        from = len + from;
    }
    if from < 0 {
        from = 0;
    }

    js_copy(J, 1);
    k = from;
    while k < len {
        if js_hasindex(J, 0, k) != 0 {
            if crate::jsvalue::js_strictequal(J) != 0 {
                js_pushnumber(J, k as f64);
                return;
            }
            js_pop(J, 1);
        }
        k += 1;
    }

    js_pushnumber(J, -1.0);
}

unsafe extern "C-unwind" fn Ap_lastIndexOf(J: *mut js_State) {
    let mut k;
    let len;
    let mut from;

    len = js_getlength(J, 0);
    from = if js_isdefined(J, 2) != 0 { js_tointeger(J, 2) } else { len - 1 };
    if from > len - 1 {
        from = len - 1;
    }
    if from < 0 {
        from = len + from;
    }

    js_copy(J, 1);
    k = from;
    while k >= 0 {
        if js_hasindex(J, 0, k) != 0 {
            if crate::jsvalue::js_strictequal(J) != 0 {
                js_pushnumber(J, k as f64);
                return;
            }
            js_pop(J, 1);
        }
        k -= 1;
    }

    js_pushnumber(J, -1.0);
}

unsafe extern "C-unwind" fn Ap_every(J: *mut js_State) {
    let hasthis = (js_gettop(J) >= 3) as c_int;
    let mut k;
    let len;

    if js_iscallable(J, 1) == 0 {
        crate::jserror::js_typeerror(J, cstr!("callback is not a function"));
    }

    len = js_getlength(J, 0);
    k = 0;
    while k < len {
        if js_hasindex(J, 0, k) != 0 {
            js_copy(J, 1);
            if hasthis != 0 {
                js_copy(J, 2);
            } else {
                js_pushundefined(J);
            }
            js_copy(J, -3);
            js_pushnumber(J, k as f64);
            js_copy(J, 0);
            js_call(J, 3);
            if js_toboolean(J, -1) == 0 {
                return;
            }
            js_pop(J, 2);
        }
        k += 1;
    }

    js_pushboolean(J, 1);
}

unsafe extern "C-unwind" fn Ap_some(J: *mut js_State) {
    let hasthis = (js_gettop(J) >= 3) as c_int;
    let mut k;
    let len;

    if js_iscallable(J, 1) == 0 {
        crate::jserror::js_typeerror(J, cstr!("callback is not a function"));
    }

    len = js_getlength(J, 0);
    k = 0;
    while k < len {
        if js_hasindex(J, 0, k) != 0 {
            js_copy(J, 1);
            if hasthis != 0 {
                js_copy(J, 2);
            } else {
                js_pushundefined(J);
            }
            js_copy(J, -3);
            js_pushnumber(J, k as f64);
            js_copy(J, 0);
            js_call(J, 3);
            if js_toboolean(J, -1) != 0 {
                return;
            }
            js_pop(J, 2);
        }
        k += 1;
    }

    js_pushboolean(J, 0);
}

unsafe extern "C-unwind" fn Ap_forEach(J: *mut js_State) {
    let hasthis = (js_gettop(J) >= 3) as c_int;
    let mut k;
    let len;

    if js_iscallable(J, 1) == 0 {
        crate::jserror::js_typeerror(J, cstr!("callback is not a function"));
    }

    len = js_getlength(J, 0);
    k = 0;
    while k < len {
        if js_hasindex(J, 0, k) != 0 {
            js_copy(J, 1);
            if hasthis != 0 {
                js_copy(J, 2);
            } else {
                js_pushundefined(J);
            }
            js_copy(J, -3);
            js_pushnumber(J, k as f64);
            js_copy(J, 0);
            js_call(J, 3);
            js_pop(J, 2);
        }
        k += 1;
    }

    js_pushundefined(J);
}

unsafe extern "C-unwind" fn Ap_map(J: *mut js_State) {
    let hasthis = (js_gettop(J) >= 3) as c_int;
    let mut k;
    let len;

    if js_iscallable(J, 1) == 0 {
        crate::jserror::js_typeerror(J, cstr!("callback is not a function"));
    }

    crate::jsvalue::js_newarray(J);

    len = js_getlength(J, 0);
    k = 0;
    while k < len {
        if js_hasindex(J, 0, k) != 0 {
            js_copy(J, 1);
            if hasthis != 0 {
                js_copy(J, 2);
            } else {
                js_pushundefined(J);
            }
            js_copy(J, -3);
            js_pushnumber(J, k as f64);
            js_copy(J, 0);
            js_call(J, 3);
            js_setindex(J, -3, k);
            js_pop(J, 1);
        }
        k += 1;
    }
    js_setlength(J, -1, len);
}

unsafe extern "C-unwind" fn Ap_filter(J: *mut js_State) {
    let hasthis = (js_gettop(J) >= 3) as c_int;
    let mut k;
    let mut to;
    let len;

    if js_iscallable(J, 1) == 0 {
        crate::jserror::js_typeerror(J, cstr!("callback is not a function"));
    }

    crate::jsvalue::js_newarray(J);
    to = 0;

    len = js_getlength(J, 0);
    k = 0;
    while k < len {
        if js_hasindex(J, 0, k) != 0 {
            js_copy(J, 1);
            if hasthis != 0 {
                js_copy(J, 2);
            } else {
                js_pushundefined(J);
            }
            js_copy(J, -3);
            js_pushnumber(J, k as f64);
            js_copy(J, 0);
            js_call(J, 3);
            if js_toboolean(J, -1) != 0 {
                js_pop(J, 1);
                js_setindex(J, -2, to);
                to += 1;
            } else {
                js_pop(J, 2);
            }
        }
        k += 1;
    }
}

unsafe extern "C-unwind" fn Ap_reduce(J: *mut js_State) {
    let hasinitial = (js_gettop(J) >= 3) as c_int;
    let mut k;
    let len;

    if js_iscallable(J, 1) == 0 {
        crate::jserror::js_typeerror(J, cstr!("callback is not a function"));
    }

    len = js_getlength(J, 0);
    k = 0;

    if len == 0 && hasinitial == 0 {
        crate::jserror::js_typeerror(J, cstr!("no initial value"));
    }

    if hasinitial != 0 {
        js_copy(J, 2);
    } else {
        while k < len {
            let had = js_hasindex(J, 0, k);
            k += 1;
            if had != 0 {
                break;
            }
        }
        if k == len {
            crate::jserror::js_typeerror(J, cstr!("no initial value"));
        }
    }

    while k < len {
        if js_hasindex(J, 0, k) != 0 {
            js_copy(J, 1);
            js_pushundefined(J);
            js_rot(J, 4);
            js_rot(J, 4);
            js_pushnumber(J, k as f64);
            js_copy(J, 0);
            js_call(J, 4);
        }
        k += 1;
    }
}

unsafe extern "C-unwind" fn Ap_reduceRight(J: *mut js_State) {
    let hasinitial = (js_gettop(J) >= 3) as c_int;
    let mut k;
    let len;

    if js_iscallable(J, 1) == 0 {
        crate::jserror::js_typeerror(J, cstr!("callback is not a function"));
    }

    len = js_getlength(J, 0);
    k = len - 1;

    if len == 0 && hasinitial == 0 {
        crate::jserror::js_typeerror(J, cstr!("no initial value"));
    }

    if hasinitial != 0 {
        js_copy(J, 2);
    } else {
        while k >= 0 {
            let had = js_hasindex(J, 0, k);
            k -= 1;
            if had != 0 {
                break;
            }
        }
        if k < 0 {
            crate::jserror::js_typeerror(J, cstr!("no initial value"));
        }
    }

    while k >= 0 {
        if js_hasindex(J, 0, k) != 0 {
            js_copy(J, 1);
            js_pushundefined(J);
            js_rot(J, 4);
            js_rot(J, 4);
            js_pushnumber(J, k as f64);
            js_copy(J, 0);
            js_call(J, 4);
        }
        k -= 1;
    }
}

unsafe extern "C-unwind" fn A_isArray(J: *mut js_State) {
    if js_isobject(J, 1) != 0 {
        let T = js_toobject(J, 1);
        js_pushboolean(J, ((*T).type_ == JS_CARRAY) as c_int);
    } else {
        js_pushboolean(J, 0);
    }
}

#[no_mangle]
pub unsafe extern "C-unwind" fn jsB_initarray(J: *mut js_State) {
    js_pushobject(J, (*J).Array_prototype);
    {
        crate::jsbuiltin::jsB_propf(J, cstr!("Array.prototype.toString"), Some(Ap_toString), 0);
        crate::jsbuiltin::jsB_propf(J, cstr!("Array.prototype.concat"), Some(Ap_concat), 0);
        crate::jsbuiltin::jsB_propf(J, cstr!("Array.prototype.join"), Some(Ap_join), 1);
        crate::jsbuiltin::jsB_propf(J, cstr!("Array.prototype.pop"), Some(Ap_pop), 0);
        crate::jsbuiltin::jsB_propf(J, cstr!("Array.prototype.push"), Some(Ap_push), 0);
        crate::jsbuiltin::jsB_propf(J, cstr!("Array.prototype.reverse"), Some(Ap_reverse), 0);
        crate::jsbuiltin::jsB_propf(J, cstr!("Array.prototype.shift"), Some(Ap_shift), 0);
        crate::jsbuiltin::jsB_propf(J, cstr!("Array.prototype.slice"), Some(Ap_slice), 2);
        crate::jsbuiltin::jsB_propf(J, cstr!("Array.prototype.sort"), Some(Ap_sort), 1);
        crate::jsbuiltin::jsB_propf(J, cstr!("Array.prototype.splice"), Some(Ap_splice), 2);
        crate::jsbuiltin::jsB_propf(J, cstr!("Array.prototype.unshift"), Some(Ap_unshift), 0);

        crate::jsbuiltin::jsB_propf(J, cstr!("Array.prototype.indexOf"), Some(Ap_indexOf), 1);
        crate::jsbuiltin::jsB_propf(J, cstr!("Array.prototype.lastIndexOf"), Some(Ap_lastIndexOf), 1);
        crate::jsbuiltin::jsB_propf(J, cstr!("Array.prototype.every"), Some(Ap_every), 1);
        crate::jsbuiltin::jsB_propf(J, cstr!("Array.prototype.some"), Some(Ap_some), 1);
        crate::jsbuiltin::jsB_propf(J, cstr!("Array.prototype.forEach"), Some(Ap_forEach), 1);
        crate::jsbuiltin::jsB_propf(J, cstr!("Array.prototype.map"), Some(Ap_map), 1);
        crate::jsbuiltin::jsB_propf(J, cstr!("Array.prototype.filter"), Some(Ap_filter), 1);
        crate::jsbuiltin::jsB_propf(J, cstr!("Array.prototype.reduce"), Some(Ap_reduce), 1);
        crate::jsbuiltin::jsB_propf(J, cstr!("Array.prototype.reduceRight"), Some(Ap_reduceRight), 1);
    }
    crate::jsvalue::js_newcconstructor(J, Some(jsB_new_Array), Some(jsB_new_Array), cstr!("Array"), 0);
    {
        crate::jsbuiltin::jsB_propf(J, cstr!("Array.isArray"), Some(A_isArray), 1);
    }
    js_defglobal(J, cstr!("Array"), JS_DONTENUM);
}
