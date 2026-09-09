//! Translation of src/jsarray.c
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(unused)]

use crate::jsi::*;

use crate::jsrun::{
    js_call, js_copy, js_delindex, js_free, js_getindex, js_getproperty, js_gettop,
    js_hasindex, js_isarray, js_iscallable, js_iscoercible, js_isdefined, js_isnumber,
    js_isobject, js_isundefined, js_malloc, js_pop, js_pushboolean, js_pushliteral,
    js_pushlstring, js_pushnumber, js_pushobject, js_pushundefined, js_pushvalue, js_realloc,
    js_rot, js_rot2pop1, js_setindex, js_setproperty, js_toboolean,
    js_tointeger, js_tonumber, js_toobject, js_tostring, js_tovalue,
};
use crate::jsrun::js_getglobal;
use crate::jsrun::{js_endtry, js_throw};
use crate::jsvalue::{js_newarray, js_newcconstructor};
use crate::jsvalue::js_strictequal;
use crate::jsbuiltin::jsB_propf;
use crate::jsrun::js_defglobal;

/*
#ifndef JS_HEAPSORT
#define JS_HEAPSORT 0
#endif
*/

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_getlength(J: *mut js_State, idx: c_int) -> c_int { unsafe {
    let len: c_int;
    js_getproperty(J, idx, c"length".as_ptr());
    len = js_tointeger(J, -1);
    js_pop(J, 1);
    len
}}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_setlength(J: *mut js_State, idx: c_int, len: c_int) { unsafe {
    js_pushnumber(J, len as f64);
    js_setproperty(J, if idx < 0 { idx - 1 } else { idx }, c"length".as_ptr());
}}

unsafe extern "C-unwind" fn jsB_new_Array(J: *mut js_State) { unsafe {
    let mut i: c_int;
    let top: c_int = js_gettop(J);

    js_newarray(J);

    if top == 2 {
        if js_isnumber(J, 1) != 0 {
            js_copy(J, 1);
            js_setproperty(J, -2, c"length".as_ptr());
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
}}

unsafe extern "C-unwind" fn Ap_concat(J: *mut js_State) { unsafe {
    let mut i: c_int;
    let top: c_int = js_gettop(J);
    let mut n: c_int;
    let mut k: c_int;
    let mut len: c_int;

    js_newarray(J);
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
}}

/* ugly cycle detection for Array.prototype.join */
#[allow(unpredictable_function_pointer_comparisons)]
unsafe extern "C-unwind" fn Ap_join_cycle(J: *mut js_State) -> c_int { unsafe {
    let needle: *mut js_Object = js_toobject(J, 0);
    let mut top: c_int = (*J).tracetop - 1;
    while top > 0 {
        let stk: c_int = (*J).trace[top as usize].stack;
        let fun: *mut js_Value = &raw mut *(*J).stack.offset((stk - 1) as isize);
        if (*fun).ty() != JS_TOBJECT { return 0; }
        if (*(*fun).object).ty != JS_CCFUNCTION { return 0; }
        if (*(*fun).object).u.c.function == Some(Ap_join as unsafe extern "C-unwind" fn(*mut js_State))
        {
            let obj: *mut js_Value = &raw mut *(*J).stack.offset(stk as isize);
            if (*obj).ty() != JS_TOBJECT { return 0; }
            if (*obj).object == needle {
                return 1;
            }
        }
        else if (*(*fun).object).u.c.function == Some(Ap_toString as unsafe extern "C-unwind" fn(*mut js_State))
        {
            /* join calls toString which calls join which calls toString, etc */
        }
        else
        {
            return 0;
        }
        top -= 1;
    }
    0
}}

#[inline(never)]
unsafe extern "C-unwind" fn Ap_join(J: *mut js_State) { unsafe {
    let mut out: *mut c_char = core::ptr::null_mut(); /* volatile */
    let mut r: *const c_char = core::ptr::null(); /* volatile */
    let sep: *const c_char;
    let seplen: c_int;
    let mut n: c_int;
    let len: c_int;

    if Ap_join_cycle(J) != 0 {
        js_pushliteral(J, c"".as_ptr());
        return;
    }

    len = js_getlength(J, 0);

    if js_isdefined(J, 1) != 0 {
        sep = js_tostring(J, 1);
        seplen = strlen(sep) as c_int;
    } else {
        sep = c",".as_ptr();
        seplen = 1;
    }

    if len <= 0 {
        js_pushliteral(J, c"".as_ptr());
        return;
    }

    /*
     * C idiom:
     *   if (js_try(J)) { js_free(J, out); js_throw(J); }
     *   BODY
     *   js_endtry(J);
     *   js_free(J, out);
     */
    n = 0;
    if crate::except::js_try_run(J, || {
        let mut k: c_int;
        let mut rlen: c_int;
        k = 0;
        while k < len {
            js_getindex(J, 0, k);
            if js_iscoercible(J, -1) != 0 {
                r = js_tostring(J, -1);
                rlen = strlen(r) as c_int;
            } else {
                rlen = 0;
            }

            if k == 0 {
                out = js_malloc(J, (rlen + 1) as c_int) as *mut c_char;
                if rlen > 0 {
                    memcpy(out as *mut c_void, r as *const c_void, rlen as size_t);
                    n += rlen;
                }
            } else {
                if n + seplen + rlen > JS_STRLIMIT {
                    js_rangeerror!(J, c"invalid string length".as_ptr());
                }
                out = js_realloc(J, out as *mut c_void, (n + seplen + rlen + 1) as c_int) as *mut c_char;
                if seplen > 0 {
                    memcpy(out.offset(n as isize) as *mut c_void, sep as *const c_void, seplen as size_t);
                    n += seplen;
                }
                if rlen > 0 {
                    memcpy(out.offset(n as isize) as *mut c_void, r as *const c_void, rlen as size_t);
                    n += rlen;
                }
            }

            js_pop(J, 1);
            k += 1;
        }

        js_pushlstring(J, out, n);
        js_endtry(J);
        js_free(J, out as *mut c_void);
    }) {
        js_free(J, out as *mut c_void);
        js_throw(J);
    }
}}

unsafe extern "C-unwind" fn Ap_pop(J: *mut js_State) { unsafe {
    let n: c_int;

    n = js_getlength(J, 0);

    if n > 0 {
        js_getindex(J, 0, n - 1);
        js_delindex(J, 0, n - 1);
        js_setlength(J, 0, n - 1);
    } else {
        js_setlength(J, 0, 0);
        js_pushundefined(J);
    }
}}

unsafe extern "C-unwind" fn Ap_push(J: *mut js_State) { unsafe {
    let mut i: c_int;
    let top: c_int = js_gettop(J);
    let mut n: c_int;

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
}}

unsafe extern "C-unwind" fn Ap_reverse(J: *mut js_State) { unsafe {
    let len: c_int;
    let middle: c_int;
    let mut lower: c_int;

    len = js_getlength(J, 0);
    middle = len / 2;
    lower = 0;

    while lower != middle {
        let upper: c_int = len - lower - 1;
        let haslower: c_int = js_hasindex(J, 0, lower);
        let hasupper: c_int = js_hasindex(J, 0, upper);
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
}}

unsafe extern "C-unwind" fn Ap_shift(J: *mut js_State) { unsafe {
    let mut k: c_int;
    let len: c_int;

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
}}

unsafe extern "C-unwind" fn Ap_slice(J: *mut js_State) { unsafe {
    let len: c_int;
    let s: c_int;
    let e: c_int;
    let mut n: c_int;
    let mut sv: f64;
    let mut ev: f64;

    js_newarray(J);

    len = js_getlength(J, 0);
    sv = js_tointeger(J, 1) as f64;
    ev = if js_isdefined(J, 2) != 0 { js_tointeger(J, 2) as f64 } else { len as f64 };

    if sv < 0.0 { sv = sv + len as f64; }
    if ev < 0.0 { ev = ev + len as f64; }

    s = if sv < 0.0 { 0 } else if sv > len as f64 { len } else { sv as c_int };
    e = if ev < 0.0 { 0 } else if ev > len as f64 { len } else { ev as c_int };

    let mut s = s;
    n = 0;
    while s < e {
        if js_hasindex(J, 0, s) != 0 {
            js_setindex(J, -2, n);
        }
        s += 1;
        n += 1;
    }
}}

unsafe fn Ap_sort_cmp(J: *mut js_State, idx_a: c_int, idx_b: c_int) -> c_int { unsafe {
    let obj: *mut js_Object = (*js_tovalue(J, 0)).object;
    if (*obj).u.a.simple != 0 && idx_b < (*obj).u.a.flat_length {
        let val_a: *mut js_Value = &raw mut *(*obj).u.a.array.offset(idx_a as isize);
        let val_b: *mut js_Value = &raw mut *(*obj).u.a.array.offset(idx_b as isize);
        let und_a: c_int = ((*val_a).ty() == JS_TUNDEFINED) as c_int;
        let und_b: c_int = ((*val_b).ty() == JS_TUNDEFINED) as c_int;
        if und_a != 0 { return und_b; }
        if und_b != 0 { return -1; }
        if js_iscallable(J, 1) != 0 {
            let v: f64;
            js_copy(J, 1); /* copy function */
            js_pushundefined(J); /* no 'this' binding */
            js_pushvalue(J, *val_a);
            js_pushvalue(J, *val_b);
            js_call(J, 2);
            v = js_tonumber(J, -1);
            js_pop(J, 1);
            if isnan(v) {
                return 0;
            }
            if v == 0.0 {
                return 0;
            }
            return if v < 0.0 { -1 } else { 1 };
        } else {
            let str_a: *const c_char;
            let str_b: *const c_char;
            let c: c_int;
            js_pushvalue(J, *val_a);
            js_pushvalue(J, *val_b);
            str_a = js_tostring(J, -2);
            str_b = js_tostring(J, -1);
            c = strcmp(str_a, str_b);
            js_pop(J, 2);
            return c;
        }
    } else {
        let und_a: c_int;
        let und_b: c_int;
        let has_a: c_int = js_hasindex(J, 0, idx_a);
        let has_b: c_int = js_hasindex(J, 0, idx_b);
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
            let v: f64;
            js_copy(J, 1); /* copy function */
            js_pushundefined(J); /* no 'this' binding */
            js_copy(J, -4);
            js_copy(J, -4);
            js_call(J, 2);
            v = js_tonumber(J, -1);
            js_pop(J, 3);
            if isnan(v) {
                return 0;
            }
            if v == 0.0 {
                return 0;
            }
            return if v < 0.0 { -1 } else { 1 };
        } else {
            let str_a: *const c_char = js_tostring(J, -2);
            let str_b: *const c_char = js_tostring(J, -1);
            let c: c_int = strcmp(str_a, str_b);
            js_pop(J, 2);
            return c;
        }
    }
}}

unsafe fn Ap_sort_swap(J: *mut js_State, idx_a: c_int, idx_b: c_int) { unsafe {
    let obj: *mut js_Object = (*js_tovalue(J, 0)).object;
    if (*obj).u.a.simple != 0 && idx_b < (*obj).u.a.flat_length {
        let tmp: js_Value = *(*obj).u.a.array.offset(idx_a as isize);
        *(*obj).u.a.array.offset(idx_a as isize) = *(*obj).u.a.array.offset(idx_b as isize);
        *(*obj).u.a.array.offset(idx_b as isize) = tmp;
    } else {
        let has_a: c_int = js_hasindex(J, 0, idx_a);
        let has_b: c_int = js_hasindex(J, 0, idx_b);
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
}}

/* A bottom-up/bouncing heapsort implementation */

unsafe fn Ap_sort_leaf(J: *mut js_State, i: c_int, end: c_int) -> c_int { unsafe {
    let mut j: c_int = i;
    let mut lc: c_int = (j << 1) + 1; /* left child */
    let mut rc: c_int = (j << 1) + 2; /* right child */
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
}}

unsafe fn Ap_sort_sift(J: *mut js_State, i: c_int, end: c_int) { unsafe {
    let mut j: c_int = Ap_sort_leaf(J, i, end);
    while j > i && Ap_sort_cmp(J, i, j) > 0 {
        j = (j - 1) >> 1; /* parent */
    }
    while j > i {
        Ap_sort_swap(J, i, j);
        j = (j - 1) >> 1; /* parent */
    }
}}

unsafe fn Ap_sort_heapsort(J: *mut js_State, n: c_int) { unsafe {
    let mut i: c_int;
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
}}

unsafe extern "C-unwind" fn Ap_sort(J: *mut js_State) { unsafe {
    let len: c_int;

    len = js_getlength(J, 0);
    if len <= 1 {
        js_copy(J, 0);
        return;
    }

    if js_iscallable(J, 1) == 0 && js_isundefined(J, 1) == 0 {
        js_typeerror!(J, c"comparison function must be a function or undefined".as_ptr());
    }

    if len >= INT_MAX {
        js_rangeerror!(J, c"array is too large to sort".as_ptr());
    }

    Ap_sort_heapsort(J, len);

    js_copy(J, 0);
}}

unsafe extern "C-unwind" fn Ap_splice(J: *mut js_State) { unsafe {
    let top: c_int = js_gettop(J);
    let len: c_int;
    let mut start: c_int;
    let mut del: c_int;
    let add: c_int;
    let mut k: c_int;

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

    js_newarray(J);

    /* copy deleted items to return array */
    k = 0;
    while k < del {
        if js_hasindex(J, 0, start + k) != 0 {
            js_setindex(J, -2, k);
        }
        k += 1;
    }
    js_setlength(J, -1, del);

    /* shift the tail to resize the hole left by deleted items */
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

    /* copy new items into the hole */
    k = 0;
    while k < add {
        js_copy(J, 3 + k);
        js_setindex(J, 0, start + k);
        k += 1;
    }

    js_setlength(J, 0, len - del + add);
}}

unsafe extern "C-unwind" fn Ap_unshift(J: *mut js_State) { unsafe {
    let mut i: c_int;
    let top: c_int = js_gettop(J);
    let mut k: c_int;
    let len: c_int;

    len = js_getlength(J, 0);

    k = len;
    while k > 0 {
        let from: c_int = k - 1;
        let to: c_int = k + top - 2;
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
}}

#[inline(never)]
unsafe extern "C-unwind" fn Ap_toString(J: *mut js_State) { unsafe {
    if js_iscoercible(J, 0) == 0 {
        js_typeerror!(J, c"'this' is not an object".as_ptr());
    }
    js_getproperty(J, 0, c"join".as_ptr());
    if js_iscallable(J, -1) == 0 {
        js_pop(J, 1);
        /* TODO: call Object.prototype.toString implementation directly */
        js_getglobal(J, c"Object".as_ptr());
        js_getproperty(J, -1, c"prototype".as_ptr());
        js_rot2pop1(J);
        js_getproperty(J, -1, c"toString".as_ptr());
        js_rot2pop1(J);
    }
    js_copy(J, 0);
    js_call(J, 0);
}}

unsafe extern "C-unwind" fn Ap_indexOf(J: *mut js_State) { unsafe {
    let mut k: c_int;
    let len: c_int;
    let mut from: c_int;

    len = js_getlength(J, 0);
    from = if js_isdefined(J, 2) != 0 { js_tointeger(J, 2) } else { 0 };
    if from < 0 { from = len + from; }
    if from < 0 { from = 0; }

    js_copy(J, 1);
    k = from;
    while k < len {
        if js_hasindex(J, 0, k) != 0 {
            if js_strictequal(J) != 0 {
                js_pushnumber(J, k as f64);
                return;
            }
            js_pop(J, 1);
        }
        k += 1;
    }

    js_pushnumber(J, -1 as c_int as f64);
}}

unsafe extern "C-unwind" fn Ap_lastIndexOf(J: *mut js_State) { unsafe {
    let mut k: c_int;
    let len: c_int;
    let mut from: c_int;

    len = js_getlength(J, 0);
    from = if js_isdefined(J, 2) != 0 { js_tointeger(J, 2) } else { len - 1 };
    if from > len - 1 { from = len - 1; }
    if from < 0 { from = len + from; }

    js_copy(J, 1);
    k = from;
    while k >= 0 {
        if js_hasindex(J, 0, k) != 0 {
            if js_strictequal(J) != 0 {
                js_pushnumber(J, k as f64);
                return;
            }
            js_pop(J, 1);
        }
        k -= 1;
    }

    js_pushnumber(J, -1 as c_int as f64);
}}

unsafe extern "C-unwind" fn Ap_every(J: *mut js_State) { unsafe {
    let hasthis: c_int = (js_gettop(J) >= 3) as c_int;
    let mut k: c_int;
    let len: c_int;

    if js_iscallable(J, 1) == 0 {
        js_typeerror!(J, c"callback is not a function".as_ptr());
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
}}

unsafe extern "C-unwind" fn Ap_some(J: *mut js_State) { unsafe {
    let hasthis: c_int = (js_gettop(J) >= 3) as c_int;
    let mut k: c_int;
    let len: c_int;

    if js_iscallable(J, 1) == 0 {
        js_typeerror!(J, c"callback is not a function".as_ptr());
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
}}

unsafe extern "C-unwind" fn Ap_forEach(J: *mut js_State) { unsafe {
    let hasthis: c_int = (js_gettop(J) >= 3) as c_int;
    let mut k: c_int;
    let len: c_int;

    if js_iscallable(J, 1) == 0 {
        js_typeerror!(J, c"callback is not a function".as_ptr());
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
}}

unsafe extern "C-unwind" fn Ap_map(J: *mut js_State) { unsafe {
    let hasthis: c_int = (js_gettop(J) >= 3) as c_int;
    let mut k: c_int;
    let len: c_int;

    if js_iscallable(J, 1) == 0 {
        js_typeerror!(J, c"callback is not a function".as_ptr());
    }

    js_newarray(J);

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
}}

unsafe extern "C-unwind" fn Ap_filter(J: *mut js_State) { unsafe {
    let hasthis: c_int = (js_gettop(J) >= 3) as c_int;
    let mut k: c_int;
    let mut to: c_int;
    let len: c_int;

    if js_iscallable(J, 1) == 0 {
        js_typeerror!(J, c"callback is not a function".as_ptr());
    }

    js_newarray(J);
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
}}

unsafe extern "C-unwind" fn Ap_reduce(J: *mut js_State) { unsafe {
    let hasinitial: c_int = (js_gettop(J) >= 3) as c_int;
    let mut k: c_int;
    let len: c_int;

    if js_iscallable(J, 1) == 0 {
        js_typeerror!(J, c"callback is not a function".as_ptr());
    }

    len = js_getlength(J, 0);
    k = 0;

    if len == 0 && hasinitial == 0 {
        js_typeerror!(J, c"no initial value".as_ptr());
    }

    /* initial value of accumulator */
    if hasinitial != 0 {
        js_copy(J, 2);
    } else {
        while k < len {
            let cur = k;
            k += 1;
            if js_hasindex(J, 0, cur) != 0 {
                break;
            }
        }
        if k == len {
            js_typeerror!(J, c"no initial value".as_ptr());
        }
    }

    while k < len {
        if js_hasindex(J, 0, k) != 0 {
            js_copy(J, 1);
            js_pushundefined(J);
            js_rot(J, 4); /* accumulator on top */
            js_rot(J, 4); /* property on top */
            js_pushnumber(J, k as f64);
            js_copy(J, 0);
            js_call(J, 4); /* calculate new accumulator */
        }
        k += 1;
    }

    /* return accumulator */
}}

unsafe extern "C-unwind" fn Ap_reduceRight(J: *mut js_State) { unsafe {
    let hasinitial: c_int = (js_gettop(J) >= 3) as c_int;
    let mut k: c_int;
    let len: c_int;

    if js_iscallable(J, 1) == 0 {
        js_typeerror!(J, c"callback is not a function".as_ptr());
    }

    len = js_getlength(J, 0);
    k = len - 1;

    if len == 0 && hasinitial == 0 {
        js_typeerror!(J, c"no initial value".as_ptr());
    }

    /* initial value of accumulator */
    if hasinitial != 0 {
        js_copy(J, 2);
    } else {
        while k >= 0 {
            let cur = k;
            k -= 1;
            if js_hasindex(J, 0, cur) != 0 {
                break;
            }
        }
        if k < 0 {
            js_typeerror!(J, c"no initial value".as_ptr());
        }
    }

    while k >= 0 {
        if js_hasindex(J, 0, k) != 0 {
            js_copy(J, 1);
            js_pushundefined(J);
            js_rot(J, 4); /* accumulator on top */
            js_rot(J, 4); /* property on top */
            js_pushnumber(J, k as f64);
            js_copy(J, 0);
            js_call(J, 4); /* calculate new accumulator */
        }
        k -= 1;
    }

    /* return accumulator */
}}

unsafe extern "C-unwind" fn A_isArray(J: *mut js_State) { unsafe {
    if js_isobject(J, 1) != 0 {
        let T: *mut js_Object = js_toobject(J, 1);
        js_pushboolean(J, ((*T).ty == JS_CARRAY) as c_int);
    } else {
        js_pushboolean(J, 0);
    }
}}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsB_initarray(J: *mut js_State) { unsafe {
    js_pushobject(J, (*J).Array_prototype);
    {
        jsB_propf(J, c"Array.prototype.toString".as_ptr(), Some(Ap_toString), 0);
        jsB_propf(J, c"Array.prototype.concat".as_ptr(), Some(Ap_concat), 0); /* 1 */
        jsB_propf(J, c"Array.prototype.join".as_ptr(), Some(Ap_join), 1);
        jsB_propf(J, c"Array.prototype.pop".as_ptr(), Some(Ap_pop), 0);
        jsB_propf(J, c"Array.prototype.push".as_ptr(), Some(Ap_push), 0); /* 1 */
        jsB_propf(J, c"Array.prototype.reverse".as_ptr(), Some(Ap_reverse), 0);
        jsB_propf(J, c"Array.prototype.shift".as_ptr(), Some(Ap_shift), 0);
        jsB_propf(J, c"Array.prototype.slice".as_ptr(), Some(Ap_slice), 2);
        jsB_propf(J, c"Array.prototype.sort".as_ptr(), Some(Ap_sort), 1);
        jsB_propf(J, c"Array.prototype.splice".as_ptr(), Some(Ap_splice), 2);
        jsB_propf(J, c"Array.prototype.unshift".as_ptr(), Some(Ap_unshift), 0); /* 1 */

        /* ES5 */
        jsB_propf(J, c"Array.prototype.indexOf".as_ptr(), Some(Ap_indexOf), 1);
        jsB_propf(J, c"Array.prototype.lastIndexOf".as_ptr(), Some(Ap_lastIndexOf), 1);
        jsB_propf(J, c"Array.prototype.every".as_ptr(), Some(Ap_every), 1);
        jsB_propf(J, c"Array.prototype.some".as_ptr(), Some(Ap_some), 1);
        jsB_propf(J, c"Array.prototype.forEach".as_ptr(), Some(Ap_forEach), 1);
        jsB_propf(J, c"Array.prototype.map".as_ptr(), Some(Ap_map), 1);
        jsB_propf(J, c"Array.prototype.filter".as_ptr(), Some(Ap_filter), 1);
        jsB_propf(J, c"Array.prototype.reduce".as_ptr(), Some(Ap_reduce), 1);
        jsB_propf(J, c"Array.prototype.reduceRight".as_ptr(), Some(Ap_reduceRight), 1);
    }
    js_newcconstructor(J, Some(jsB_new_Array), Some(jsB_new_Array), c"Array".as_ptr(), 0); /* 1 */
    {
        /* ES5 */
        jsB_propf(J, c"Array.isArray".as_ptr(), Some(A_isArray), 1);
    }
    js_defglobal(J, c"Array".as_ptr(), JS_DONTENUM);
}}
