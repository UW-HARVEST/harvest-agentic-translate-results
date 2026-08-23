//! Translation of `c_src/src/jsrun.c`
#![allow(non_snake_case)]
#![allow(unused_assignments)]

use crate::cstd::*;
use crate::jsi::*;
use crate::jsarray::{js_getlength, js_setlength};
use crate::jsgc::js_gc;
use crate::jsintern::js_intern;
use crate::jsproperty::*;
use crate::jsvalue::*;
use crate::utf::runetochar;
use core::ptr::{null, null_mut};

/* Push values on stack */

/// `STACK[i]` as an lvalue-ish raw pointer
#[inline]
unsafe fn slot(J: *mut js_State, i: c_int) -> *mut js_Value {
    (*J).stack.offset(i as isize)
}

pub(crate) unsafe fn js_trystackoverflow(J: *mut js_State) -> ! {
    (*slot(J, (*J).top)).set_ty(JS_TLITSTR);
    (*slot(J, (*J).top)).set_litstr(c"exception stack overflow".as_ptr());
    (*J).top += 1;
    js_throw(J)
}

pub(crate) unsafe fn js_stackoverflow(J: *mut js_State) -> ! {
    (*slot(J, (*J).top)).set_ty(JS_TLITSTR);
    (*slot(J, (*J).top)).set_litstr(c"stack overflow".as_ptr());
    (*J).top += 1;
    js_throw(J)
}

pub(crate) unsafe fn js_outofmemory(J: *mut js_State) -> ! {
    (*slot(J, (*J).top)).set_ty(JS_TLITSTR);
    (*slot(J, (*J).top)).set_litstr(c"out of memory".as_ptr());
    (*J).top += 1;
    js_throw(J)
}

pub(crate) unsafe fn js_runlimit(J: *mut js_State) -> ! {
    (*slot(J, (*J).top)).set_ty(JS_TLITSTR);
    (*slot(J, (*J).top)).set_litstr(c"script ran too long".as_ptr());
    (*J).top += 1;
    js_throw(J)
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_setlimit(J: *mut js_State, runlimit: c_int, memlimit: c_int) {
    (*J).runlimit = runlimit;
    (*J).memlimit = memlimit;
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_malloc(J: *mut js_State, size: c_int) -> *mut c_void {
    let mut ptr: *mut c_void;
    if (*J).memlimit > 0 {
        if size >= (*J).memlimit {
            js_outofmemory(J);
        }
        (*J).memlimit -= size;
    }
    ptr = ((*J).alloc.unwrap())((*J).actx, null_mut(), size);
    if ptr.is_null() {
        js_outofmemory(J);
    }
    ptr
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_realloc(
    J: *mut js_State,
    mut ptr: *mut c_void,
    size: c_int,
) -> *mut c_void {
    if (*J).memlimit > 0 {
        if size >= (*J).memlimit {
            js_outofmemory(J);
        }
        (*J).memlimit -= size;
    }
    ptr = ((*J).alloc.unwrap())((*J).actx, ptr, size);
    if ptr.is_null() {
        js_outofmemory(J);
    }
    ptr
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_strdup(J: *mut js_State, s: *const c_char) -> *mut c_char {
    let n = strlen(s) + 1;
    let p = js_malloc(J, n as c_int) as *mut c_char;
    memcpy(p as *mut c_void, s as *const c_void, n);
    p
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_free(J: *mut js_State, ptr: *mut c_void) {
    ((*J).alloc.unwrap())((*J).actx, ptr, 0);
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsV_newmemstring(
    J: *mut js_State,
    s: *const c_char,
    n: c_int,
) -> *mut js_String {
    let v = js_malloc(J, JS_STRING_P_OFFSET as c_int + n + 1) as *mut js_String;
    let p = (*v).p.as_mut_ptr();
    memcpy(p as *mut c_void, s as *const c_void, n as size_t);
    *p.offset(n as isize) = 0;
    (*v).gcmark = 0;
    (*v).gcnext = (*J).gcstr;
    (*J).gcstr = v;
    (*J).gccounter += 1;
    v
}

macro_rules! CHECKSTACK {
    ($J:expr, $n:expr) => {
        if (*$J).top + $n >= JS_STACKSIZE {
            js_stackoverflow($J);
        }
    };
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_pushvalue(J: *mut js_State, v: js_Value) {
    CHECKSTACK!(J, 1);
    *slot(J, (*J).top) = v;
    (*J).top += 1;
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_pushundefined(J: *mut js_State) {
    CHECKSTACK!(J, 1);
    (*slot(J, (*J).top)).set_ty(JS_TUNDEFINED);
    (*J).top += 1;
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_pushnull(J: *mut js_State) {
    CHECKSTACK!(J, 1);
    (*slot(J, (*J).top)).set_ty(JS_TNULL);
    (*J).top += 1;
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_pushboolean(J: *mut js_State, v: c_int) {
    CHECKSTACK!(J, 1);
    (*slot(J, (*J).top)).set_ty(JS_TBOOLEAN);
    (*slot(J, (*J).top)).set_boolean((v != 0) as c_int);
    (*J).top += 1;
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_pushnumber(J: *mut js_State, v: f64) {
    CHECKSTACK!(J, 1);
    (*slot(J, (*J).top)).set_ty(JS_TNUMBER);
    (*slot(J, (*J).top)).set_num(v);
    (*J).top += 1;
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_pushstring(J: *mut js_State, mut v: *const c_char) {
    let mut n = strlen(v);
    if n > JS_STRLIMIT as size_t {
        js_rangeerror!(J, c"invalid string length".as_ptr());
    }
    CHECKSTACK!(J, 1);
    if n <= 15 {
        let mut s = (*slot(J, (*J).top)).shrstr_mut();
        while n != 0 {
            n -= 1;
            *s = *v;
            s = s.offset(1);
            v = v.offset(1);
        }
        *s = 0;
        (*slot(J, (*J).top)).set_ty(JS_TSHRSTR);
    } else {
        (*slot(J, (*J).top)).set_ty(JS_TMEMSTR);
        (*slot(J, (*J).top)).set_memstr(jsV_newmemstring(J, v, n as c_int));
    }
    (*J).top += 1;
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_pushlstring(J: *mut js_State, mut v: *const c_char, n: c_int) {
    if n > JS_STRLIMIT {
        js_rangeerror!(J, c"invalid string length".as_ptr());
    }
    CHECKSTACK!(J, 1);
    if n <= 15 {
        let mut k = n;
        let mut s = (*slot(J, (*J).top)).shrstr_mut();
        loop {
            let old = k;
            k = k.wrapping_sub(1);
            if old == 0 {
                break;
            }
            *s = *v;
            s = s.offset(1);
            v = v.offset(1);
        }
        *s = 0;
        (*slot(J, (*J).top)).set_ty(JS_TSHRSTR);
    } else {
        (*slot(J, (*J).top)).set_ty(JS_TMEMSTR);
        (*slot(J, (*J).top)).set_memstr(jsV_newmemstring(J, v, n));
    }
    (*J).top += 1;
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_pushliteral(J: *mut js_State, v: *const c_char) {
    CHECKSTACK!(J, 1);
    (*slot(J, (*J).top)).set_ty(JS_TLITSTR);
    (*slot(J, (*J).top)).set_litstr(v);
    (*J).top += 1;
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_pushobject(J: *mut js_State, v: *mut js_Object) {
    CHECKSTACK!(J, 1);
    (*slot(J, (*J).top)).set_ty(JS_TOBJECT);
    (*slot(J, (*J).top)).set_object(v);
    (*J).top += 1;
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_pushglobal(J: *mut js_State) {
    js_pushobject(J, (*J).G);
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_currentfunction(J: *mut js_State) {
    CHECKSTACK!(J, 1);
    if (*J).bot > 0 {
        *slot(J, (*J).top) = *slot(J, (*J).bot - 1);
    } else {
        (*slot(J, (*J).top)).set_ty(JS_TUNDEFINED);
    }
    (*J).top += 1;
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_currentfunctiondata(J: *mut js_State) -> *mut c_void {
    if (*J).bot > 0 {
        return (*(*slot(J, (*J).bot - 1)).object()).u.c.data;
    }
    null_mut()
}

/* Read values from stack */

static mut UNDEFINED: js_Value = js_Value::undef();

pub(crate) unsafe fn stackidx(J: *mut js_State, idx: c_int) -> *mut js_Value {
    let idx = if idx < 0 { (*J).top + idx } else { (*J).bot + idx };
    if idx < 0 || idx >= (*J).top {
        return core::ptr::addr_of_mut!(UNDEFINED);
    }
    (*J).stack.offset(idx as isize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_tovalue(J: *mut js_State, idx: c_int) -> *mut js_Value {
    stackidx(J, idx)
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_isdefined(J: *mut js_State, idx: c_int) -> c_int {
    ((*stackidx(J, idx)).ty() != JS_TUNDEFINED) as c_int
}
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_isundefined(J: *mut js_State, idx: c_int) -> c_int {
    ((*stackidx(J, idx)).ty() == JS_TUNDEFINED) as c_int
}
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_isnull(J: *mut js_State, idx: c_int) -> c_int {
    ((*stackidx(J, idx)).ty() == JS_TNULL) as c_int
}
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_isboolean(J: *mut js_State, idx: c_int) -> c_int {
    ((*stackidx(J, idx)).ty() == JS_TBOOLEAN) as c_int
}
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_isnumber(J: *mut js_State, idx: c_int) -> c_int {
    ((*stackidx(J, idx)).ty() == JS_TNUMBER) as c_int
}
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_isstring(J: *mut js_State, idx: c_int) -> c_int {
    let t = (*stackidx(J, idx)).ty();
    (t == JS_TSHRSTR || t == JS_TLITSTR || t == JS_TMEMSTR) as c_int
}
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_isprimitive(J: *mut js_State, idx: c_int) -> c_int {
    ((*stackidx(J, idx)).ty() != JS_TOBJECT) as c_int
}
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_isobject(J: *mut js_State, idx: c_int) -> c_int {
    ((*stackidx(J, idx)).ty() == JS_TOBJECT) as c_int
}
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_iscoercible(J: *mut js_State, idx: c_int) -> c_int {
    let v = stackidx(J, idx);
    ((*v).ty() != JS_TUNDEFINED && (*v).ty() != JS_TNULL) as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_iscallable(J: *mut js_State, idx: c_int) -> c_int {
    let v = stackidx(J, idx);
    if (*v).ty() == JS_TOBJECT {
        let t = (*(*v).object()).type_;
        return (t == JS_CFUNCTION || t == JS_CSCRIPT || t == JS_CCFUNCTION) as c_int;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_isarray(J: *mut js_State, idx: c_int) -> c_int {
    let v = stackidx(J, idx);
    ((*v).ty() == JS_TOBJECT && (*(*v).object()).type_ == JS_CARRAY) as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_isregexp(J: *mut js_State, idx: c_int) -> c_int {
    let v = stackidx(J, idx);
    ((*v).ty() == JS_TOBJECT && (*(*v).object()).type_ == JS_CREGEXP) as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_isuserdata(
    J: *mut js_State,
    idx: c_int,
    tag: *const c_char,
) -> c_int {
    let v = stackidx(J, idx);
    if (*v).ty() == JS_TOBJECT && (*(*v).object()).type_ == JS_CUSERDATA {
        return (strcmp(tag, (*(*v).object()).u.user.tag) == 0) as c_int;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_iserror(J: *mut js_State, idx: c_int) -> c_int {
    let v = stackidx(J, idx);
    ((*v).ty() == JS_TOBJECT && (*(*v).object()).type_ == JS_CERROR) as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_typeof(J: *mut js_State, idx: c_int) -> *const c_char {
    let v = stackidx(J, idx);
    match (*v).ty() {
        JS_TUNDEFINED => c"undefined".as_ptr(),
        JS_TNULL => c"object".as_ptr(),
        JS_TBOOLEAN => c"boolean".as_ptr(),
        JS_TNUMBER => c"number".as_ptr(),
        JS_TLITSTR => c"string".as_ptr(),
        JS_TMEMSTR => c"string".as_ptr(),
        JS_TOBJECT => {
            let t = (*(*v).object()).type_;
            if t == JS_CFUNCTION || t == JS_CCFUNCTION {
                c"function".as_ptr()
            } else {
                c"object".as_ptr()
            }
        }
        /* JS_TSHRSTR and default */
        _ => c"string".as_ptr(),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_type(J: *mut js_State, idx: c_int) -> c_int {
    let v = stackidx(J, idx);
    match (*v).ty() {
        JS_TUNDEFINED => JS_ISUNDEFINED,
        JS_TNULL => JS_ISNULL,
        JS_TBOOLEAN => JS_ISBOOLEAN,
        JS_TNUMBER => JS_ISNUMBER,
        JS_TLITSTR => JS_ISSTRING,
        JS_TMEMSTR => JS_ISSTRING,
        JS_TOBJECT => {
            let t = (*(*v).object()).type_;
            if t == JS_CFUNCTION || t == JS_CCFUNCTION {
                JS_ISFUNCTION
            } else {
                JS_ISOBJECT
            }
        }
        _ => JS_ISSTRING,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_toboolean(J: *mut js_State, idx: c_int) -> c_int {
    jsV_toboolean(J, stackidx(J, idx))
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_tonumber(J: *mut js_State, idx: c_int) -> f64 {
    jsV_tonumber(J, stackidx(J, idx))
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_tointeger(J: *mut js_State, idx: c_int) -> c_int {
    jsV_numbertointeger(jsV_tonumber(J, stackidx(J, idx)))
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_toint32(J: *mut js_State, idx: c_int) -> c_int {
    jsV_numbertoint32(jsV_tonumber(J, stackidx(J, idx)))
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_touint32(J: *mut js_State, idx: c_int) -> c_uint {
    jsV_numbertouint32(jsV_tonumber(J, stackidx(J, idx)))
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_toint16(J: *mut js_State, idx: c_int) -> i16 {
    jsV_numbertoint16(jsV_tonumber(J, stackidx(J, idx)))
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_touint16(J: *mut js_State, idx: c_int) -> u16 {
    jsV_numbertouint16(jsV_tonumber(J, stackidx(J, idx)))
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_tostring(J: *mut js_State, idx: c_int) -> *const c_char {
    jsV_tostring(J, stackidx(J, idx))
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_toobject(J: *mut js_State, idx: c_int) -> *mut js_Object {
    jsV_toobject(J, stackidx(J, idx))
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_toprimitive(J: *mut js_State, idx: c_int, hint: c_int) {
    jsV_toprimitive(J, stackidx(J, idx), hint);
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_toregexp(J: *mut js_State, idx: c_int) -> *mut js_Regexp {
    let v = stackidx(J, idx);
    if (*v).ty() == JS_TOBJECT && (*(*v).object()).type_ == JS_CREGEXP {
        return core::ptr::addr_of_mut!((*(*v).object()).u.r);
    }
    js_typeerror!(J, c"not a regexp".as_ptr());
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_touserdata(
    J: *mut js_State,
    idx: c_int,
    tag: *const c_char,
) -> *mut c_void {
    let v = stackidx(J, idx);
    if (*v).ty() == JS_TOBJECT && (*(*v).object()).type_ == JS_CUSERDATA {
        if strcmp(tag, (*(*v).object()).u.user.tag) == 0 {
            return (*(*v).object()).u.user.data;
        }
    }
    js_typeerror!(J, c"not a %s".as_ptr(), tag);
}

unsafe fn jsR_tofunction(J: *mut js_State, idx: c_int) -> *mut js_Object {
    let v = stackidx(J, idx);
    if (*v).ty() == JS_TUNDEFINED || (*v).ty() == JS_TNULL {
        return null_mut();
    }
    if (*v).ty() == JS_TOBJECT {
        let t = (*(*v).object()).type_;
        if t == JS_CFUNCTION || t == JS_CCFUNCTION {
            return (*v).object();
        }
    }
    js_typeerror!(J, c"not a function".as_ptr());
}

/* Stack manipulation */

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_gettop(J: *mut js_State) -> c_int {
    (*J).top - (*J).bot
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_pop(J: *mut js_State, n: c_int) {
    (*J).top -= n;
    if (*J).top < (*J).bot {
        (*J).top = (*J).bot;
        js_error!(J, c"stack underflow!".as_ptr());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_remove(J: *mut js_State, idx: c_int) {
    let mut idx = if idx < 0 { (*J).top + idx } else { (*J).bot + idx };
    if idx < (*J).bot || idx >= (*J).top {
        js_error!(J, c"stack error!".as_ptr());
    }
    while idx < (*J).top - 1 {
        *slot(J, idx) = *slot(J, idx + 1);
        idx += 1;
    }
    (*J).top -= 1;
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_insert(J: *mut js_State, _idx: c_int) {
    js_error!(J, c"not implemented yet".as_ptr());
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_replace(J: *mut js_State, idx: c_int) {
    let idx = if idx < 0 { (*J).top + idx } else { (*J).bot + idx };
    if idx < (*J).bot || idx >= (*J).top {
        js_error!(J, c"stack error!".as_ptr());
    }
    (*J).top -= 1;
    *slot(J, idx) = *slot(J, (*J).top);
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_copy(J: *mut js_State, idx: c_int) {
    CHECKSTACK!(J, 1);
    *slot(J, (*J).top) = *stackidx(J, idx);
    (*J).top += 1;
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_dup(J: *mut js_State) {
    CHECKSTACK!(J, 1);
    *slot(J, (*J).top) = *slot(J, (*J).top - 1);
    (*J).top += 1;
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_dup2(J: *mut js_State) {
    CHECKSTACK!(J, 2);
    *slot(J, (*J).top) = *slot(J, (*J).top - 2);
    *slot(J, (*J).top + 1) = *slot(J, (*J).top - 1);
    (*J).top += 2;
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_rot2(J: *mut js_State) {
    let t = (*J).top;
    let tmp = *slot(J, t - 1);
    *slot(J, t - 1) = *slot(J, t - 2);
    *slot(J, t - 2) = tmp;
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_rot3(J: *mut js_State) {
    let t = (*J).top;
    let tmp = *slot(J, t - 1);
    *slot(J, t - 1) = *slot(J, t - 2);
    *slot(J, t - 2) = *slot(J, t - 3);
    *slot(J, t - 3) = tmp;
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_rot4(J: *mut js_State) {
    let t = (*J).top;
    let tmp = *slot(J, t - 1);
    *slot(J, t - 1) = *slot(J, t - 2);
    *slot(J, t - 2) = *slot(J, t - 3);
    *slot(J, t - 3) = *slot(J, t - 4);
    *slot(J, t - 4) = tmp;
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_rot2pop1(J: *mut js_State) {
    let t = (*J).top;
    *slot(J, t - 2) = *slot(J, t - 1);
    (*J).top -= 1;
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_rot3pop2(J: *mut js_State) {
    let t = (*J).top;
    *slot(J, t - 3) = *slot(J, t - 1);
    (*J).top -= 2;
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_rot(J: *mut js_State, n: c_int) {
    let t = (*J).top;
    let tmp = *slot(J, t - 1);
    let mut i = 1;
    while i < n {
        *slot(J, t - i) = *slot(J, t - i - 1);
        i += 1;
    }
    *slot(J, t - i) = tmp;
}

/* Property access that takes care of attributes and getters/setters */

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_isarrayindex(
    _J: *mut js_State,
    mut p: *const c_char,
    idx: *mut c_int,
) -> c_int {
    let mut n: c_int = 0;

    /* check for empty string */
    if *p == 0 {
        return 0;
    }

    /* check for '0' and integers with leading zero */
    if *p == b'0' as c_char {
        if *p.offset(1) == 0 {
            *idx = 0;
            return 1;
        }
        return 0;
    }

    while *p != 0 {
        let c = *p as c_int;
        p = p.offset(1);
        if c >= '0' as c_int && c <= '9' as c_int {
            if n >= INT_MAX / 10 {
                return 0;
            }
            n = n * 10 + (c - '0' as c_int);
        } else {
            return 0;
        }
    }
    *idx = n;
    1
}

unsafe fn js_pushrune(J: *mut js_State, rune: Rune) {
    let mut buf = [0 as c_char; UTFmax + 1];
    if rune >= 0 {
        let n = runetochar(buf.as_mut_ptr(), &rune);
        buf[n as usize] = 0;
        js_pushstring(J, buf.as_ptr());
    } else {
        js_pushundefined(J);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsR_unflattenarray(J: *mut js_State, obj: *mut js_Object) {
    if (*obj).type_ == JS_CARRAY && (*obj).u.a.simple != 0 {
        let mut name = [0 as c_char; 32];
        if js_do_try(J, || {
            let mut i = 0;
            while i < (*obj).u.a.flat_length {
                js_itoa(name.as_mut_ptr(), i);
                let r = jsV_setproperty(J, obj, name.as_ptr());
                (*r).value = *(*obj).u.a.array.offset(i as isize);
                i += 1;
            }
            js_free(J, (*obj).u.a.array as *mut c_void);
            (*obj).u.a.simple = 0;
            (*obj).u.a.flat_length = 0;
            (*obj).u.a.flat_capacity = 0;
            (*obj).u.a.array = null_mut();
            js_endtry(J);
        })
        .is_none()
        {
            (*obj).properties = null_mut();
            js_throw(J);
        }
    }
}

pub(crate) unsafe fn jsR_hasproperty(
    J: *mut js_State,
    obj: *mut js_Object,
    name: *const c_char,
) -> c_int {
    let mut k: c_int = 0;

    if (*obj).type_ == JS_CARRAY {
        if streq(name, c"length".as_ptr()) {
            js_pushnumber(J, (*obj).u.a.length as f64);
            return 1;
        }
        if (*obj).u.a.simple != 0 {
            if js_isarrayindex(J, name, &mut k) != 0 {
                if k >= 0 && k < (*obj).u.a.flat_length {
                    js_pushvalue(J, *(*obj).u.a.array.offset(k as isize));
                    return 1;
                }
                return 0;
            }
        }
    } else if (*obj).type_ == JS_CSTRING {
        if streq(name, c"length".as_ptr()) {
            js_pushnumber(J, (*obj).u.s.length as f64);
            return 1;
        }
        if js_isarrayindex(J, name, &mut k) != 0 {
            if k >= 0 && k < (*obj).u.s.length {
                js_pushrune(J, js_runeat(J, (*obj).u.s.string, k));
                return 1;
            }
        }
    } else if (*obj).type_ == JS_CREGEXP {
        if streq(name, c"source".as_ptr()) {
            js_pushstring(J, (*obj).u.r.source);
            return 1;
        }
        if streq(name, c"global".as_ptr()) {
            js_pushboolean(J, (*obj).u.r.flags as c_int & JS_REGEXP_G);
            return 1;
        }
        if streq(name, c"ignoreCase".as_ptr()) {
            js_pushboolean(J, (*obj).u.r.flags as c_int & JS_REGEXP_I);
            return 1;
        }
        if streq(name, c"multiline".as_ptr()) {
            js_pushboolean(J, (*obj).u.r.flags as c_int & JS_REGEXP_M);
            return 1;
        }
        if streq(name, c"lastIndex".as_ptr()) {
            js_pushnumber(J, (*obj).u.r.last as f64);
            return 1;
        }
    } else if (*obj).type_ == JS_CUSERDATA {
        if let Some(has) = (*obj).u.user.has {
            if has(J, (*obj).u.user.data, name) != 0 {
                return 1;
            }
        }
    }

    let refp = jsV_getproperty(J, obj, name);
    if !refp.is_null() {
        if !(*refp).getter.is_null() {
            js_pushobject(J, (*refp).getter);
            js_pushobject(J, obj);
            js_call(J, 0);
        } else {
            js_pushvalue(J, (*refp).value);
        }
        return 1;
    }

    0
}

pub(crate) unsafe fn jsR_getproperty(J: *mut js_State, obj: *mut js_Object, name: *const c_char) {
    if jsR_hasproperty(J, obj, name) == 0 {
        js_pushundefined(J);
    }
}

unsafe fn jsR_hasindex(J: *mut js_State, obj: *mut js_Object, k: c_int) -> c_int {
    let mut buf = [0 as c_char; 32];
    if (*obj).type_ == JS_CARRAY && (*obj).u.a.simple != 0 {
        if k >= 0 && k < (*obj).u.a.flat_length {
            js_pushvalue(J, *(*obj).u.a.array.offset(k as isize));
            return 1;
        }
        return 0;
    }
    jsR_hasproperty(J, obj, js_itoa(buf.as_mut_ptr(), k))
}

unsafe fn jsR_getindex(J: *mut js_State, obj: *mut js_Object, k: c_int) {
    if jsR_hasindex(J, obj, k) == 0 {
        js_pushundefined(J);
    }
}

unsafe fn jsR_setarrayindex(J: *mut js_State, obj: *mut js_Object, k: c_int, value: *mut js_Value) {
    let newlen = k + 1;
    if newlen > JS_ARRAYLIMIT {
        js_rangeerror!(J, c"array too large".as_ptr());
    }
    if newlen > (*obj).u.a.flat_length {
        if newlen > (*obj).u.a.flat_capacity {
            let mut newcap = (*obj).u.a.flat_capacity;
            if newcap == 0 {
                newcap = 8;
            }
            while newcap < newlen {
                newcap <<= 1;
            }
            (*obj).u.a.array = js_realloc(
                J,
                (*obj).u.a.array as *mut c_void,
                newcap * core::mem::size_of::<js_Value>() as c_int,
            ) as *mut js_Value;
            (*obj).u.a.flat_capacity = newcap;
        }
        (*obj).u.a.flat_length = newlen;
    }
    if newlen > (*obj).u.a.length {
        (*obj).u.a.length = newlen;
    }
    *(*obj).u.a.array.offset(k as isize) = *value;
}

pub(crate) unsafe fn jsR_setproperty(
    J: *mut js_State,
    obj: *mut js_Object,
    name: *const c_char,
    transient: c_int,
) {
    let value = stackidx(J, -1);
    let mut refp: *mut js_Property;
    let mut k: c_int = 0;
    let mut own: c_int = 0;

    'readonly: {
        if (*obj).type_ == JS_CARRAY {
            if streq(name, c"length".as_ptr()) {
                let rawlen = jsV_tonumber(J, value);
                let newlen = jsV_numbertointeger(rawlen);
                if (newlen as f64) != rawlen || newlen < 0 {
                    js_rangeerror!(J, c"invalid array length".as_ptr());
                }
                if newlen > JS_ARRAYLIMIT {
                    js_rangeerror!(J, c"array too large".as_ptr());
                }
                if (*obj).u.a.simple != 0 {
                    (*obj).u.a.length = newlen;
                    if newlen <= (*obj).u.a.flat_length {
                        (*obj).u.a.flat_length = newlen;
                    }
                } else {
                    jsV_resizearray(J, obj, newlen);
                }
                return;
            }

            if js_isarrayindex(J, name, &mut k) != 0 {
                if (*obj).u.a.simple != 0 {
                    if k >= 0 && k <= (*obj).u.a.flat_length {
                        jsR_setarrayindex(J, obj, k, value);
                    } else {
                        jsR_unflattenarray(J, obj);
                        if (*obj).u.a.length < k + 1 {
                            (*obj).u.a.length = k + 1;
                        }
                    }
                } else {
                    if (*obj).u.a.length < k + 1 {
                        (*obj).u.a.length = k + 1;
                    }
                }
            }
        } else if (*obj).type_ == JS_CSTRING {
            if streq(name, c"length".as_ptr()) {
                break 'readonly;
            }
            if js_isarrayindex(J, name, &mut k) != 0 {
                if k >= 0 && k < (*obj).u.s.length {
                    break 'readonly;
                }
            }
        } else if (*obj).type_ == JS_CREGEXP {
            if streq(name, c"source".as_ptr()) {
                break 'readonly;
            }
            if streq(name, c"global".as_ptr()) {
                break 'readonly;
            }
            if streq(name, c"ignoreCase".as_ptr()) {
                break 'readonly;
            }
            if streq(name, c"multiline".as_ptr()) {
                break 'readonly;
            }
            if streq(name, c"lastIndex".as_ptr()) {
                (*obj).u.r.last = cvt_u16(jsV_tointeger(J, value));
                return;
            }
        } else if (*obj).type_ == JS_CUSERDATA {
            if let Some(put) = (*obj).u.user.put {
                if put(J, (*obj).u.user.data, name) != 0 {
                    return;
                }
            }
        }

        /* First try to find a setter in prototype chain */
        refp = jsV_getpropertyx(J, obj, name, &mut own);
        if !refp.is_null() {
            if !(*refp).setter.is_null() {
                js_pushobject(J, (*refp).setter);
                js_pushobject(J, obj);
                js_pushvalue(J, *value);
                js_call(J, 1);
                js_pop(J, 1);
                return;
            } else {
                if (*J).strict != 0 {
                    if !(*refp).getter.is_null() {
                        js_typeerror!(
                            J,
                            c"setting property '%s' that only has a getter".as_ptr(),
                            name
                        );
                    }
                }
                if (*refp).atts & JS_READONLY != 0 {
                    break 'readonly;
                }
            }
        }

        /* Property not found on this object, so create one */
        if refp.is_null() || own == 0 {
            if transient != 0 {
                if (*J).strict != 0 {
                    js_typeerror!(
                        J,
                        c"cannot create property '%s' on transient object".as_ptr(),
                        name
                    );
                }
                return;
            }
            refp = jsV_setproperty(J, obj, name);
        }

        if !refp.is_null() {
            if (*refp).atts & JS_READONLY == 0 {
                (*refp).value = *value;
            } else {
                break 'readonly;
            }
        }

        return;
    }

    /* readonly: */
    if (*J).strict != 0 {
        js_typeerror!(J, c"'%s' is read-only".as_ptr(), name);
    }
}

unsafe fn jsR_setindex(J: *mut js_State, obj: *mut js_Object, k: c_int, transient: c_int) {
    let mut buf = [0 as c_char; 32];
    if (*obj).type_ == JS_CARRAY && (*obj).u.a.simple != 0 && k >= 0 && k <= (*obj).u.a.flat_length {
        jsR_setarrayindex(J, obj, k, stackidx(J, -1));
    } else {
        jsR_setproperty(J, obj, js_itoa(buf.as_mut_ptr(), k), transient);
    }
}

pub(crate) unsafe fn jsR_defproperty(
    J: *mut js_State,
    obj: *mut js_Object,
    name: *const c_char,
    atts: c_int,
    value: *mut js_Value,
    getter: *mut js_Object,
    setter: *mut js_Object,
    throw_: c_int,
) {
    let mut k: c_int = 0;

    'readonly: {
        if (*obj).type_ == JS_CARRAY {
            if streq(name, c"length".as_ptr()) {
                break 'readonly;
            }
            if (*obj).u.a.simple != 0 {
                jsR_unflattenarray(J, obj);
            }
        } else if (*obj).type_ == JS_CSTRING {
            if streq(name, c"length".as_ptr()) {
                break 'readonly;
            }
            if js_isarrayindex(J, name, &mut k) != 0 {
                if k >= 0 && k < (*obj).u.s.length {
                    break 'readonly;
                }
            }
        } else if (*obj).type_ == JS_CREGEXP {
            if streq(name, c"source".as_ptr()) {
                break 'readonly;
            }
            if streq(name, c"global".as_ptr()) {
                break 'readonly;
            }
            if streq(name, c"ignoreCase".as_ptr()) {
                break 'readonly;
            }
            if streq(name, c"multiline".as_ptr()) {
                break 'readonly;
            }
            if streq(name, c"lastIndex".as_ptr()) {
                break 'readonly;
            }
        } else if (*obj).type_ == JS_CUSERDATA {
            if let Some(put) = (*obj).u.user.put {
                if put(J, (*obj).u.user.data, name) != 0 {
                    return;
                }
            }
        }

        let refp = jsV_setproperty(J, obj, name);
        if !refp.is_null() {
            if !value.is_null() {
                if (*refp).atts & JS_READONLY == 0 {
                    (*refp).value = *value;
                } else if (*J).strict != 0 {
                    js_typeerror!(J, c"'%s' is read-only".as_ptr(), name);
                }
            }
            if !getter.is_null() {
                if (*refp).atts & JS_DONTCONF == 0 {
                    (*refp).getter = getter;
                } else if (*J).strict != 0 {
                    js_typeerror!(J, c"'%s' is non-configurable".as_ptr(), name);
                }
            }
            if !setter.is_null() {
                if (*refp).atts & JS_DONTCONF == 0 {
                    (*refp).setter = setter;
                } else if (*J).strict != 0 {
                    js_typeerror!(J, c"'%s' is non-configurable".as_ptr(), name);
                }
            }
            (*refp).atts |= atts;
        }

        return;
    }

    /* readonly: */
    if (*J).strict != 0 || throw_ != 0 {
        js_typeerror!(
            J,
            c"'%s' is read-only or non-configurable".as_ptr(),
            name
        );
    }
}

pub(crate) unsafe fn jsR_delproperty(
    J: *mut js_State,
    obj: *mut js_Object,
    name: *const c_char,
) -> c_int {
    let mut k: c_int = 0;

    'dontconf: {
        if (*obj).type_ == JS_CARRAY {
            if streq(name, c"length".as_ptr()) {
                break 'dontconf;
            }
            if (*obj).u.a.simple != 0 {
                jsR_unflattenarray(J, obj);
            }
        } else if (*obj).type_ == JS_CSTRING {
            if streq(name, c"length".as_ptr()) {
                break 'dontconf;
            }
            if js_isarrayindex(J, name, &mut k) != 0 {
                if k >= 0 && k < (*obj).u.s.length {
                    break 'dontconf;
                }
            }
        } else if (*obj).type_ == JS_CREGEXP {
            if streq(name, c"source".as_ptr()) {
                break 'dontconf;
            }
            if streq(name, c"global".as_ptr()) {
                break 'dontconf;
            }
            if streq(name, c"ignoreCase".as_ptr()) {
                break 'dontconf;
            }
            if streq(name, c"multiline".as_ptr()) {
                break 'dontconf;
            }
            if streq(name, c"lastIndex".as_ptr()) {
                break 'dontconf;
            }
        } else if (*obj).type_ == JS_CUSERDATA {
            if let Some(del) = (*obj).u.user.delete {
                if del(J, (*obj).u.user.data, name) != 0 {
                    return 1;
                }
            }
        }

        let refp = jsV_getownproperty(J, obj, name);
        if !refp.is_null() {
            if (*refp).atts & JS_DONTCONF != 0 {
                break 'dontconf;
            }
            jsV_delproperty(J, obj, name);
        }
        return 1;
    }

    /* dontconf: */
    if (*J).strict != 0 {
        js_typeerror!(J, c"'%s' is non-configurable".as_ptr(), name);
    }
    0
}

unsafe fn jsR_delindex(J: *mut js_State, obj: *mut js_Object, k: c_int) {
    let mut buf = [0 as c_char; 32];
    /* Allow deleting last element of a simple array without unflattening */
    if (*obj).type_ == JS_CARRAY && (*obj).u.a.simple != 0 && k == (*obj).u.a.flat_length - 1 {
        (*obj).u.a.flat_length = k;
    } else {
        jsR_delproperty(J, obj, js_itoa(buf.as_mut_ptr(), k));
    }
}

/* Registry, global and object property accessors */

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_ref(J: *mut js_State) -> *const c_char {
    let v = stackidx(J, -1);
    let s: *const c_char;
    let mut buf = [0 as c_char; 32];
    match (*v).ty() {
        JS_TUNDEFINED => s = c"_Undefined".as_ptr(),
        JS_TNULL => s = c"_Null".as_ptr(),
        JS_TBOOLEAN => {
            s = if (*v).boolean() != 0 {
                c"_True".as_ptr()
            } else {
                c"_False".as_ptr()
            }
        }
        JS_TOBJECT => {
            sprintf(buf.as_mut_ptr(), c"%p".as_ptr(), (*v).object() as *mut c_void);
            s = js_intern(J, buf.as_ptr());
        }
        _ => {
            let n = (*J).nextref;
            (*J).nextref += 1;
            sprintf(buf.as_mut_ptr(), c"%d".as_ptr(), n);
            s = js_intern(J, buf.as_ptr());
        }
    }
    js_setregistry(J, s);
    s
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_unref(J: *mut js_State, r: *const c_char) {
    js_delregistry(J, r);
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_getregistry(J: *mut js_State, name: *const c_char) {
    jsR_getproperty(J, (*J).R, name);
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_setregistry(J: *mut js_State, name: *const c_char) {
    jsR_setproperty(J, (*J).R, name, 0);
    js_pop(J, 1);
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_delregistry(J: *mut js_State, name: *const c_char) {
    jsR_delproperty(J, (*J).R, name);
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_getglobal(J: *mut js_State, name: *const c_char) {
    jsR_getproperty(J, (*J).G, name);
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_setglobal(J: *mut js_State, name: *const c_char) {
    jsR_setproperty(J, (*J).G, name, 0);
    js_pop(J, 1);
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_defglobal(J: *mut js_State, name: *const c_char, atts: c_int) {
    jsR_defproperty(
        J,
        (*J).G,
        name,
        atts,
        stackidx(J, -1),
        null_mut(),
        null_mut(),
        0,
    );
    js_pop(J, 1);
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_delglobal(J: *mut js_State, name: *const c_char) {
    jsR_delproperty(J, (*J).G, name);
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_getproperty(J: *mut js_State, idx: c_int, name: *const c_char) {
    jsR_getproperty(J, js_toobject(J, idx), name);
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_setproperty(J: *mut js_State, idx: c_int, name: *const c_char) {
    let obj = js_toobject(J, idx);
    let transient = (js_isobject(J, idx) == 0) as c_int;
    jsR_setproperty(J, obj, name, transient);
    js_pop(J, 1);
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_defproperty(
    J: *mut js_State,
    idx: c_int,
    name: *const c_char,
    atts: c_int,
) {
    jsR_defproperty(
        J,
        js_toobject(J, idx),
        name,
        atts,
        stackidx(J, -1),
        null_mut(),
        null_mut(),
        1,
    );
    js_pop(J, 1);
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_delproperty(J: *mut js_State, idx: c_int, name: *const c_char) {
    jsR_delproperty(J, js_toobject(J, idx), name);
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_defaccessor(
    J: *mut js_State,
    idx: c_int,
    name: *const c_char,
    atts: c_int,
) {
    let obj = js_toobject(J, idx);
    let g = jsR_tofunction(J, -2);
    let s = jsR_tofunction(J, -1);
    jsR_defproperty(J, obj, name, atts, null_mut(), g, s, 1);
    js_pop(J, 2);
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_hasproperty(
    J: *mut js_State,
    idx: c_int,
    name: *const c_char,
) -> c_int {
    jsR_hasproperty(J, js_toobject(J, idx), name)
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_getindex(J: *mut js_State, idx: c_int, i: c_int) {
    jsR_getindex(J, js_toobject(J, idx), i);
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_hasindex(J: *mut js_State, idx: c_int, i: c_int) -> c_int {
    jsR_hasindex(J, js_toobject(J, idx), i)
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_setindex(J: *mut js_State, idx: c_int, i: c_int) {
    let obj = js_toobject(J, idx);
    let transient = (js_isobject(J, idx) == 0) as c_int;
    jsR_setindex(J, obj, i, transient);
    js_pop(J, 1);
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_delindex(J: *mut js_State, idx: c_int, i: c_int) {
    jsR_delindex(J, js_toobject(J, idx), i);
}

/* Iterator */

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_pushiterator(J: *mut js_State, idx: c_int, own: c_int) {
    js_pushobject(J, jsV_newiterator(J, js_toobject(J, idx), own));
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_nextiterator(J: *mut js_State, idx: c_int) -> *const c_char {
    jsV_nextiterator(J, js_toobject(J, idx))
}

/* Environment records */

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsR_newenvironment(
    J: *mut js_State,
    vars: *mut js_Object,
    outer: *mut js_Environment,
) -> *mut js_Environment {
    let E = js_malloc(J, core::mem::size_of::<js_Environment>() as c_int) as *mut js_Environment;
    (*E).gcmark = 0;
    (*E).gcnext = (*J).gcenv;
    (*J).gcenv = E;
    (*J).gccounter += 1;

    (*E).outer = outer;
    (*E).variables = vars;
    E
}

unsafe fn js_initvar(J: *mut js_State, name: *const c_char, idx: c_int) {
    jsR_defproperty(
        J,
        (*(*J).E).variables,
        name,
        JS_DONTENUM | JS_DONTCONF,
        stackidx(J, idx),
        null_mut(),
        null_mut(),
        0,
    );
}

unsafe fn js_hasvar(J: *mut js_State, name: *const c_char) -> c_int {
    let mut E = (*J).E;
    loop {
        let refp = jsV_getproperty(J, (*E).variables, name);
        if !refp.is_null() {
            if !(*refp).getter.is_null() {
                js_pushobject(J, (*refp).getter);
                js_pushobject(J, (*E).variables);
                js_call(J, 0);
            } else {
                js_pushvalue(J, (*refp).value);
            }
            return 1;
        }
        E = (*E).outer;
        if E.is_null() {
            break;
        }
    }
    0
}

unsafe fn js_setvar(J: *mut js_State, name: *const c_char) {
    let mut E = (*J).E;
    loop {
        let refp = jsV_getproperty(J, (*E).variables, name);
        if !refp.is_null() {
            if !(*refp).setter.is_null() {
                js_pushobject(J, (*refp).setter);
                js_pushobject(J, (*E).variables);
                js_copy(J, -3);
                js_call(J, 1);
                js_pop(J, 1);
                return;
            }
            if (*refp).atts & JS_READONLY == 0 {
                (*refp).value = *stackidx(J, -1);
            } else if (*J).strict != 0 {
                js_typeerror!(J, c"'%s' is read-only".as_ptr(), name);
            }
            return;
        }
        E = (*E).outer;
        if E.is_null() {
            break;
        }
    }
    if (*J).strict != 0 {
        js_referenceerror!(
            J,
            c"assignment to undeclared variable '%s'".as_ptr(),
            name
        );
    }
    jsR_setproperty(J, (*J).G, name, 0);
}

unsafe fn js_delvar(J: *mut js_State, name: *const c_char) -> c_int {
    let mut E = (*J).E;
    loop {
        let refp = jsV_getownproperty(J, (*E).variables, name);
        if !refp.is_null() {
            if (*refp).atts & JS_DONTCONF != 0 {
                if (*J).strict != 0 {
                    js_typeerror!(J, c"'%s' is non-configurable".as_ptr(), name);
                }
                return 0;
            }
            jsV_delproperty(J, (*E).variables, name);
            return 1;
        }
        E = (*E).outer;
        if E.is_null() {
            break;
        }
    }
    jsR_delproperty(J, (*J).G, name)
}

/* Function calls */

unsafe fn jsR_savescope(J: *mut js_State, newE: *mut js_Environment) {
    if (*J).envtop as usize + 1 >= JS_ENVLIMIT {
        js_stackoverflow(J);
    }
    (*J).envstack[(*J).envtop as usize] = (*J).E;
    (*J).envtop += 1;
    (*J).E = newE;
}

unsafe fn jsR_restorescope(J: *mut js_State) {
    (*J).envtop -= 1;
    (*J).E = (*J).envstack[(*J).envtop as usize];
}

unsafe fn jsR_calllwfunction(
    J: *mut js_State,
    mut n: c_int,
    F: *mut js_Function,
    scope: *mut js_Environment,
) {
    jsR_savescope(J, scope);

    if n > (*F).numparams {
        js_pop(J, n - (*F).numparams);
        n = (*F).numparams;
    }

    let mut i = n;
    while i < (*F).varlen {
        js_pushundefined(J);
        i += 1;
    }

    jsR_run(J, F);
    let v = *stackidx(J, -1);
    (*J).bot -= 1;
    (*J).top = (*J).bot; /* clear stack */
    js_pushvalue(J, v);

    jsR_restorescope(J);
}

unsafe fn jsR_callfunction(
    J: *mut js_State,
    n: c_int,
    F: *mut js_Function,
    mut scope: *mut js_Environment,
) {
    scope = jsR_newenvironment(J, jsV_newobject(J, JS_COBJECT, null_mut()), scope);

    jsR_savescope(J, scope);

    if (*F).arguments != 0 {
        js_newarguments(J);
        if (*J).strict == 0 {
            js_currentfunction(J);
            js_defproperty(J, -2, c"callee".as_ptr(), JS_DONTENUM);
        }
        js_pushnumber(J, n as f64);
        js_defproperty(J, -2, c"length".as_ptr(), JS_DONTENUM);
        let mut i = 0;
        while i < n {
            js_copy(J, i + 1);
            js_setindex(J, -2, i);
            i += 1;
        }
        js_initvar(J, c"arguments".as_ptr(), -1);
        js_pop(J, 1);
    }

    let mut i = 0;
    while i < n && i < (*F).numparams {
        js_initvar(J, *(*F).vartab.offset(i as isize), i + 1);
        i += 1;
    }
    js_pop(J, n);

    while i < (*F).varlen {
        js_pushundefined(J);
        js_initvar(J, *(*F).vartab.offset(i as isize), -1);
        js_pop(J, 1);
        i += 1;
    }

    jsR_run(J, F);
    let v = *stackidx(J, -1);
    (*J).bot -= 1;
    (*J).top = (*J).bot; /* clear stack */
    js_pushvalue(J, v);

    jsR_restorescope(J);
}

unsafe fn jsR_callscript(
    J: *mut js_State,
    n: c_int,
    F: *mut js_Function,
    scope: *mut js_Environment,
) {
    if !scope.is_null() {
        jsR_savescope(J, scope);
    }

    /* scripts take no arguments */
    js_pop(J, n);

    let mut i = 0;
    while i < (*F).varlen {
        /* Bug 701886: don't redefine existing vars in eval/scripts */
        if js_hasvar(J, *(*F).vartab.offset(i as isize)) == 0 {
            js_pushundefined(J);
            js_initvar(J, *(*F).vartab.offset(i as isize), -1);
            js_pop(J, 1);
        }
        i += 1;
    }

    jsR_run(J, F);
    let v = *stackidx(J, -1);
    (*J).bot -= 1;
    (*J).top = (*J).bot; /* clear stack */
    js_pushvalue(J, v);

    if !scope.is_null() {
        jsR_restorescope(J);
    }
}

unsafe fn jsR_callcfunction(J: *mut js_State, n: c_int, min: c_int, F: js_CFunction) {
    let mut i = n;
    while i < min {
        js_pushundefined(J);
        i += 1;
    }

    let save_top = (*J).top;
    (F.unwrap())(J);
    if (*J).top > save_top {
        let v = *stackidx(J, -1);
        (*J).bot -= 1;
        (*J).top = (*J).bot; /* clear stack */
        js_pushvalue(J, v);
    } else {
        (*J).bot -= 1;
        (*J).top = (*J).bot; /* clear stack */
        js_pushundefined(J);
    }
}

unsafe fn jsR_pushtrace(
    J: *mut js_State,
    name: *const c_char,
    file: *const c_char,
    line: c_int,
) {
    if (*J).tracetop as usize + 1 == JS_ENVLIMIT {
        js_error!(J, c"call stack overflow".as_ptr());
    }
    (*J).tracetop += 1;
    let t = (*J).tracetop as usize;
    (*J).trace[t].stack = (*J).bot;
    (*J).trace[t].name = name;
    (*J).trace[t].file = file;
    (*J).trace[t].line = line;
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_call(J: *mut js_State, n: c_int) {
    if n < 0 {
        js_rangeerror!(J, c"number of arguments cannot be negative".as_ptr());
    }

    if js_iscallable(J, -n - 2) == 0 {
        js_typeerror!(J, c"%s is not callable".as_ptr(), js_typeof(J, -n - 2));
    }

    let obj = js_toobject(J, -n - 2);

    let savebot = (*J).bot;
    (*J).bot = (*J).top - n - 1;

    if (*obj).type_ == JS_CFUNCTION {
        let f = (*obj).u.f.function;
        jsR_pushtrace(J, (*f).name, (*f).filename, (*f).line);
        if (*f).lightweight != 0 {
            jsR_calllwfunction(J, n, f, (*obj).u.f.scope);
        } else {
            jsR_callfunction(J, n, f, (*obj).u.f.scope);
        }
        (*J).tracetop -= 1;
    } else if (*obj).type_ == JS_CSCRIPT {
        let f = (*obj).u.f.function;
        jsR_pushtrace(J, (*f).name, (*f).filename, (*f).line);
        jsR_callscript(J, n, f, (*obj).u.f.scope);
        (*J).tracetop -= 1;
    } else if (*obj).type_ == JS_CCFUNCTION {
        jsR_pushtrace(J, (*obj).u.c.name, c"native".as_ptr(), 0);
        jsR_callcfunction(J, n, (*obj).u.c.length, (*obj).u.c.function);
        (*J).tracetop -= 1;
    }

    (*J).bot = savebot;
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_construct(J: *mut js_State, n: c_int) {
    let prototype: *mut js_Object;

    if js_iscallable(J, -n - 1) == 0 {
        js_typeerror!(J, c"%s is not callable".as_ptr(), js_typeof(J, -n - 1));
    }

    let obj = js_toobject(J, -n - 1);

    /* built-in constructors create their own objects, give them a 'null' this */
    if (*obj).type_ == JS_CCFUNCTION && (*obj).u.c.constructor.is_some() {
        let savebot = (*J).bot;
        js_pushnull(J);
        if n > 0 {
            js_rot(J, n + 1);
        }
        (*J).bot = (*J).top - n - 1;

        jsR_pushtrace(J, (*obj).u.c.name, c"native".as_ptr(), 0);
        jsR_callcfunction(J, n, (*obj).u.c.length, (*obj).u.c.constructor);
        (*J).tracetop -= 1;

        (*J).bot = savebot;
        return;
    }

    /* extract the function object's prototype property */
    js_getproperty(J, -n - 1, c"prototype".as_ptr());
    if js_isobject(J, -1) != 0 {
        prototype = js_toobject(J, -1);
    } else {
        prototype = (*J).Object_prototype;
    }
    js_pop(J, 1);

    /* create a new object with above prototype, and shift it into the 'this' slot */
    let newobj = jsV_newobject(J, JS_COBJECT, prototype);
    js_pushobject(J, newobj);
    if n > 0 {
        js_rot(J, n + 1);
    }

    /* and save a copy to return */
    js_pushobject(J, newobj);
    js_rot(J, n + 3);

    /* call the function */
    js_call(J, n);

    /* if result is not an object, return the original object we created */
    if js_isobject(J, -1) == 0 {
        js_pop(J, 1);
    } else {
        js_rot2pop1(J);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_eval(J: *mut js_State) {
    if js_isstring(J, -1) == 0 {
        return;
    }
    js_loadeval(J, c"(eval)".as_ptr(), js_tostring(J, -1));
    js_rot2pop1(J);
    js_copy(J, 0); /* copy 'this' */
    js_call(J, 0);
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_pconstruct(J: *mut js_State, n: c_int) -> c_int {
    let savetop = (*J).top - n - 2;
    if js_do_try(J, || {
        js_construct(J, n);
        js_endtry(J);
    })
    .is_none()
    {
        /* clean up the stack to only hold the error object */
        *slot(J, savetop) = *slot(J, (*J).top - 1);
        (*J).top = savetop + 1;
        return 1;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_pcall(J: *mut js_State, n: c_int) -> c_int {
    let savetop = (*J).top - n - 2;
    if js_do_try(J, || {
        js_call(J, n);
        js_endtry(J);
    })
    .is_none()
    {
        /* clean up the stack to only hold the error object */
        *slot(J, savetop) = *slot(J, (*J).top - 1);
        (*J).top = savetop + 1;
        return 1;
    }
    0
}

/* Exceptions */

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_savetrypc(
    J: *mut js_State,
    pc: *mut js_Instruction,
) -> *mut c_void {
    if (*J).trytop as usize == JS_TRYLIMIT {
        js_trystackoverflow(J);
    }
    let i = (*J).trytop as usize;
    (*J).trybuf[i].kind = TRY_EXTERNAL;
    (*J).trybuf[i].owner = 0;
    (*J).trybuf[i].E = (*J).E;
    (*J).trybuf[i].envtop = (*J).envtop;
    (*J).trybuf[i].tracetop = (*J).tracetop;
    (*J).trybuf[i].top = (*J).top;
    (*J).trybuf[i].bot = (*J).bot;
    (*J).trybuf[i].strict = (*J).strict;
    (*J).trybuf[i].pc = pc;
    (*J).trytop += 1;
    (*J).trybuf[i].buf.as_mut_ptr() as *mut c_void
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_savetry(J: *mut js_State) -> *mut c_void {
    js_savetrypc(J, null_mut())
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_endtry(J: *mut js_State) {
    if (*J).trytop == 0 {
        js_error!(J, c"endtry: exception stack underflow".as_ptr());
    }
    (*J).trytop -= 1;
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_throw(J: *mut js_State) -> ! {
    if (*J).trytop > 0 {
        let v = *stackidx(J, -1);
        (*J).trytop -= 1;
        let i = (*J).trytop as usize;
        (*J).E = (*J).trybuf[i].E;
        (*J).envtop = (*J).trybuf[i].envtop;
        (*J).tracetop = (*J).trybuf[i].tracetop;
        (*J).top = (*J).trybuf[i].top;
        (*J).bot = (*J).trybuf[i].bot;
        (*J).strict = (*J).trybuf[i].strict;
        js_pushvalue(J, v);
        if (*J).trybuf[i].kind == TRY_EXTERNAL {
            longjmp((*J).trybuf[i].buf.as_mut_ptr() as *mut c_void, 1);
        } else {
            let owner = (*J).trybuf[i].owner;
            std::panic::panic_any(JsThrow(owner));
        }
    }
    if let Some(p) = (*J).panic {
        p(J);
    }
    abort();
}

/* Main interpreter loop */

unsafe fn js_dumpvalue(J: *mut js_State, v: js_Value) {
    match v.ty() {
        JS_TUNDEFINED => {
            printf(c"undefined".as_ptr());
        }
        JS_TNULL => {
            printf(c"null".as_ptr());
        }
        JS_TBOOLEAN => {
            printf(if v.boolean() != 0 {
                c"true".as_ptr()
            } else {
                c"false".as_ptr()
            });
        }
        JS_TNUMBER => {
            printf(c"%.9g".as_ptr(), v.num());
        }
        JS_TSHRSTR => {
            printf(c"'%s'".as_ptr(), v.shrstr());
        }
        JS_TLITSTR => {
            printf(c"'%s'".as_ptr(), v.litstr());
        }
        JS_TMEMSTR => {
            printf(c"'%s'".as_ptr(), (*v.memstr()).p.as_ptr());
        }
        JS_TOBJECT => {
            let o = v.object();
            if o == (*J).G {
                printf(c"[Global]".as_ptr());
                return;
            }
            match (*o).type_ {
                JS_COBJECT => {
                    printf(c"[Object %p]".as_ptr(), o as *mut c_void);
                }
                JS_CARRAY => {
                    printf(c"[Array %p]".as_ptr(), o as *mut c_void);
                }
                JS_CFUNCTION => {
                    printf(
                        c"[Function %p, %s, %s:%d]".as_ptr(),
                        o as *mut c_void,
                        (*(*o).u.f.function).name,
                        (*(*o).u.f.function).filename,
                        (*(*o).u.f.function).line,
                    );
                }
                JS_CSCRIPT => {
                    printf(c"[Script %s]".as_ptr(), (*(*o).u.f.function).filename);
                }
                JS_CCFUNCTION => {
                    printf(c"[CFunction %s]".as_ptr(), (*o).u.c.name);
                }
                JS_CBOOLEAN => {
                    printf(c"[Boolean %d]".as_ptr(), (*o).u.boolean);
                }
                JS_CNUMBER => {
                    printf(c"[Number %g]".as_ptr(), (*o).u.number);
                }
                JS_CSTRING => {
                    printf(c"[String'%s']".as_ptr(), (*o).u.s.string);
                }
                JS_CERROR => {
                    printf(c"[Error]".as_ptr());
                }
                JS_CARGUMENTS => {
                    printf(c"[Arguments %p]".as_ptr(), o as *mut c_void);
                }
                JS_CITERATOR => {
                    printf(c"[Iterator %p]".as_ptr(), o as *mut c_void);
                }
                JS_CUSERDATA => {
                    printf(
                        c"[Userdata %s %p]".as_ptr(),
                        (*o).u.user.tag,
                        (*o).u.user.data,
                    );
                }
                _ => {
                    printf(c"[Object %p]".as_ptr(), o as *mut c_void);
                }
            }
        }
        _ => {}
    }
}

unsafe fn js_stacktrace(J: *mut js_State) {
    printf(c"stack trace:\n".as_ptr());
    let mut n = (*J).tracetop;
    while n >= 0 {
        let name = (*J).trace[n as usize].name;
        let file = (*J).trace[n as usize].file;
        let line = (*J).trace[n as usize].line;
        if line > 0 {
            if *name != 0 {
                printf(c"\tat %s (%s:%d)\n".as_ptr(), name, file, line);
            } else {
                printf(c"\tat %s:%d\n".as_ptr(), file, line);
            }
        } else {
            printf(c"\tat %s (%s)\n".as_ptr(), name, file);
        }
        n -= 1;
    }
}

unsafe fn js_dumpstack(J: *mut js_State) {
    printf(c"stack {\n".as_ptr());
    let mut i = 0;
    while i < (*J).top {
        putchar(if i == (*J).bot { '>' as c_int } else { ' ' as c_int });
        printf(c"%4d: ".as_ptr(), i);
        js_dumpvalue(J, *slot(J, i));
        putchar('\n' as c_int);
        i += 1;
    }
    printf(c"}\n".as_ptr());
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_trap(J: *mut js_State, _pc: c_int) {
    js_dumpstack(J);
    js_stacktrace(J);
}

unsafe fn jsR_isindex(J: *mut js_State, idx: c_int, k: *mut c_int) -> c_int {
    let v = stackidx(J, idx);
    if (*v).ty() == JS_TNUMBER {
        *k = cvt_i32((*v).num());
        return ((*k as f64) == (*v).num() && *k >= 0) as c_int;
    }
    0
}

unsafe fn jsR_run(J: *mut js_State, F: *mut js_Function) {
    let savestrict = (*J).strict;
    (*J).strict = (*F).strict;
    let owner = next_try_id(J);
    let mut pc = (*F).code;
    loop {
        let start = pc;
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            jsR_loop(J, F, start, owner, savestrict)
        }));
        match r {
            Ok(()) => return,
            Err(p) => match p.downcast::<JsThrow>() {
                Ok(b) => {
                    if b.0 == owner {
                        pc = (*J).trybuf[(*J).trytop as usize].pc;
                    } else {
                        std::panic::resume_unwind(b);
                    }
                }
                Err(p) => std::panic::resume_unwind(p),
            },
        }
    }
}

unsafe fn jsR_loop(
    J: *mut js_State,
    F: *mut js_Function,
    startpc: *mut js_Instruction,
    owner: u64,
    savestrict: c_int,
) {
    let FT = (*F).funtab;
    let VT: *mut *const c_char = if !(*F).vartab.is_null() {
        (*F).vartab.offset(-1)
    } else {
        null_mut()
    };
    let lightweight = (*F).lightweight;
    let pcstart = (*F).code;
    let mut pc = startpc;
    let mut offset: c_int;

    let mut str: *const c_char = null();
    let mut obj: *mut js_Object;
    let mut x: f64;
    let mut y: f64;
    let mut ux: c_uint;
    let mut uy: c_uint;
    let mut ix: c_int = 0;
    let mut iy: c_int;
    let mut okay: c_int = 0;
    let mut b: c_int;
    let mut transient: c_int;

    /// `READSTRING()`: read an 8 byte string pointer out of the instruction stream
    macro_rules! READSTRING {
        ($pc:ident, $str:ident) => {{
            $str = ($pc as *const *const c_char).read_unaligned();
            $pc = $pc.offset(4);
        }};
    }

    loop {
        let opcode: c_int;
        if (*J).runlimit > 0 {
            if (*J).runlimit == 1 {
                js_runlimit(J);
            }
            (*J).runlimit -= 1;
        }

        if (*J).gccounter > (*J).gcthresh {
            js_gc(J, 0);
        }

        (*J).trace[(*J).tracetop as usize].line = *pc as c_int;
        pc = pc.offset(1);

        opcode = *pc as c_int;
        pc = pc.offset(1);

        match opcode {
            OP_POP => js_pop(J, 1),
            OP_DUP => js_dup(J),
            OP_DUP2 => js_dup2(J),
            OP_ROT2 => js_rot2(J),
            OP_ROT3 => js_rot3(J),
            OP_ROT4 => js_rot4(J),

            OP_INTEGER => {
                let k = *pc as c_int;
                pc = pc.offset(1);
                js_pushnumber(J, (k - 32768) as f64);
            }

            OP_NUMBER => {
                x = (pc as *const f64).read_unaligned();
                pc = pc.offset(4);
                js_pushnumber(J, x);
            }

            OP_STRING => {
                READSTRING!(pc, str);
                js_pushliteral(J, str);
            }

            OP_CLOSURE => {
                let k = *pc as isize;
                pc = pc.offset(1);
                js_newfunction(J, *FT.offset(k), (*J).E);
            }
            OP_NEWOBJECT => js_newobject(J),
            OP_NEWARRAY => js_newarray(J),
            OP_NEWREGEXP => {
                READSTRING!(pc, str);
                let f = *pc as c_int;
                pc = pc.offset(1);
                js_newregexp(J, str, f);
            }

            OP_UNDEF => js_pushundefined(J),
            OP_NULL => js_pushnull(J),
            OP_TRUE => js_pushboolean(J, 1),
            OP_FALSE => js_pushboolean(J, 0),

            OP_THIS => {
                if (*J).strict != 0 {
                    js_copy(J, 0);
                } else {
                    if js_iscoercible(J, 0) != 0 {
                        js_copy(J, 0);
                    } else {
                        js_pushglobal(J);
                    }
                }
            }

            OP_CURRENT => {
                js_currentfunction(J);
            }

            OP_GETLOCAL => {
                if lightweight != 0 {
                    CHECKSTACK!(J, 1);
                    let k = *pc as c_int;
                    pc = pc.offset(1);
                    *slot(J, (*J).top) = *slot(J, (*J).bot + k);
                    (*J).top += 1;
                } else {
                    let k = *pc as isize;
                    pc = pc.offset(1);
                    str = *VT.offset(k);
                    if js_hasvar(J, str) == 0 {
                        js_referenceerror!(J, c"'%s' is not defined".as_ptr(), str);
                    }
                }
            }

            OP_SETLOCAL => {
                if lightweight != 0 {
                    let k = *pc as c_int;
                    pc = pc.offset(1);
                    *slot(J, (*J).bot + k) = *slot(J, (*J).top - 1);
                } else {
                    let k = *pc as isize;
                    pc = pc.offset(1);
                    js_setvar(J, *VT.offset(k));
                }
            }

            OP_DELLOCAL => {
                if lightweight != 0 {
                    pc = pc.offset(1);
                    js_pushboolean(J, 0);
                } else {
                    let k = *pc as isize;
                    pc = pc.offset(1);
                    b = js_delvar(J, *VT.offset(k));
                    js_pushboolean(J, b);
                }
            }

            OP_GETVAR => {
                READSTRING!(pc, str);
                if js_hasvar(J, str) == 0 {
                    js_referenceerror!(J, c"'%s' is not defined".as_ptr(), str);
                }
            }

            OP_HASVAR => {
                READSTRING!(pc, str);
                if js_hasvar(J, str) == 0 {
                    js_pushundefined(J);
                }
            }

            OP_SETVAR => {
                READSTRING!(pc, str);
                js_setvar(J, str);
            }

            OP_DELVAR => {
                READSTRING!(pc, str);
                b = js_delvar(J, str);
                js_pushboolean(J, b);
            }

            OP_IN => {
                str = js_tostring(J, -2);
                if js_isobject(J, -1) == 0 {
                    js_typeerror!(J, c"operand to 'in' is not an object".as_ptr());
                }
                b = js_hasproperty(J, -1, str);
                js_pop(J, 2 + b);
                js_pushboolean(J, b);
            }

            OP_SKIPARRAY => {
                js_setlength(J, -1, js_getlength(J, -1) + 1);
            }
            OP_INITARRAY => {
                js_setindex(J, -2, js_getlength(J, -2));
            }

            OP_INITPROP => {
                obj = js_toobject(J, -3);
                str = js_tostring(J, -2);
                jsR_setproperty(J, obj, str, 0);
                js_pop(J, 2);
            }

            OP_INITGETTER => {
                obj = js_toobject(J, -3);
                str = js_tostring(J, -2);
                let g = jsR_tofunction(J, -1);
                jsR_defproperty(J, obj, str, 0, null_mut(), g, null_mut(), 0);
                js_pop(J, 2);
            }

            OP_INITSETTER => {
                obj = js_toobject(J, -3);
                str = js_tostring(J, -2);
                let s = jsR_tofunction(J, -1);
                jsR_defproperty(J, obj, str, 0, null_mut(), null_mut(), s, 0);
                js_pop(J, 2);
            }

            OP_GETPROP => {
                if jsR_isindex(J, -1, &mut ix) != 0 {
                    obj = js_toobject(J, -2);
                    jsR_getindex(J, obj, ix);
                } else {
                    str = js_tostring(J, -1);
                    obj = js_toobject(J, -2);
                    jsR_getproperty(J, obj, str);
                }
                js_rot3pop2(J);
            }

            OP_GETPROP_S => {
                READSTRING!(pc, str);
                obj = js_toobject(J, -1);
                jsR_getproperty(J, obj, str);
                js_rot2pop1(J);
            }

            OP_SETPROP => {
                if jsR_isindex(J, -2, &mut ix) != 0 {
                    obj = js_toobject(J, -3);
                    transient = (js_isobject(J, -3) == 0) as c_int;
                    jsR_setindex(J, obj, ix, transient);
                } else {
                    str = js_tostring(J, -2);
                    obj = js_toobject(J, -3);
                    transient = (js_isobject(J, -3) == 0) as c_int;
                    jsR_setproperty(J, obj, str, transient);
                }
                js_rot3pop2(J);
            }

            OP_SETPROP_S => {
                READSTRING!(pc, str);
                obj = js_toobject(J, -2);
                transient = (js_isobject(J, -2) == 0) as c_int;
                jsR_setproperty(J, obj, str, transient);
                js_rot2pop1(J);
            }

            OP_DELPROP => {
                str = js_tostring(J, -1);
                obj = js_toobject(J, -2);
                b = jsR_delproperty(J, obj, str);
                js_pop(J, 2);
                js_pushboolean(J, b);
            }

            OP_DELPROP_S => {
                READSTRING!(pc, str);
                obj = js_toobject(J, -1);
                b = jsR_delproperty(J, obj, str);
                js_pop(J, 1);
                js_pushboolean(J, b);
            }

            OP_ITERATOR => {
                if js_iscoercible(J, -1) != 0 {
                    obj = jsV_newiterator(J, js_toobject(J, -1), 0);
                    js_pop(J, 1);
                    js_pushobject(J, obj);
                }
            }

            OP_NEXTITER => {
                if js_isobject(J, -1) != 0 {
                    obj = js_toobject(J, -1);
                    str = jsV_nextiterator(J, obj);
                    if !str.is_null() {
                        js_pushstring(J, str);
                        js_pushboolean(J, 1);
                    } else {
                        js_pop(J, 1);
                        js_pushboolean(J, 0);
                    }
                } else {
                    js_pop(J, 1);
                    js_pushboolean(J, 0);
                }
            }

            /* Function calls */
            OP_EVAL => {
                js_eval(J);
            }

            OP_CALL => {
                let k = *pc as c_int;
                pc = pc.offset(1);
                js_call(J, k);
            }

            OP_NEW => {
                let k = *pc as c_int;
                pc = pc.offset(1);
                js_construct(J, k);
            }

            /* Unary operators */
            OP_TYPEOF => {
                str = js_typeof(J, -1);
                js_pop(J, 1);
                js_pushliteral(J, str);
            }

            OP_POS => {
                x = js_tonumber(J, -1);
                js_pop(J, 1);
                js_pushnumber(J, x);
            }

            OP_NEG => {
                x = js_tonumber(J, -1);
                js_pop(J, 1);
                js_pushnumber(J, -x);
            }

            OP_BITNOT => {
                ix = js_toint32(J, -1);
                js_pop(J, 1);
                js_pushnumber(J, (!ix) as f64);
            }

            OP_LOGNOT => {
                b = js_toboolean(J, -1);
                js_pop(J, 1);
                js_pushboolean(J, (b == 0) as c_int);
            }

            OP_INC => {
                x = js_tonumber(J, -1);
                js_pop(J, 1);
                js_pushnumber(J, x + 1.0);
            }

            OP_DEC => {
                x = js_tonumber(J, -1);
                js_pop(J, 1);
                js_pushnumber(J, x - 1.0);
            }

            OP_POSTINC => {
                x = js_tonumber(J, -1);
                js_pop(J, 1);
                js_pushnumber(J, x + 1.0);
                js_pushnumber(J, x);
            }

            OP_POSTDEC => {
                x = js_tonumber(J, -1);
                js_pop(J, 1);
                js_pushnumber(J, x - 1.0);
                js_pushnumber(J, x);
            }

            /* Multiplicative operators */
            OP_MUL => {
                x = js_tonumber(J, -2);
                y = js_tonumber(J, -1);
                js_pop(J, 2);
                js_pushnumber(J, x * y);
            }

            OP_DIV => {
                x = js_tonumber(J, -2);
                y = js_tonumber(J, -1);
                js_pop(J, 2);
                js_pushnumber(J, x / y);
            }

            OP_MOD => {
                x = js_tonumber(J, -2);
                y = js_tonumber(J, -1);
                js_pop(J, 2);
                js_pushnumber(J, fmod(x, y));
            }

            /* Additive operators */
            OP_ADD => {
                js_concat(J);
            }

            OP_SUB => {
                x = js_tonumber(J, -2);
                y = js_tonumber(J, -1);
                js_pop(J, 2);
                js_pushnumber(J, x - y);
            }

            /* Shift operators */
            OP_SHL => {
                ix = js_toint32(J, -2);
                uy = js_touint32(J, -1);
                js_pop(J, 2);
                js_pushnumber(J, (((ix as u32) << (uy & 0x1F)) as i32) as f64);
            }

            OP_SHR => {
                ix = js_toint32(J, -2);
                uy = js_touint32(J, -1);
                js_pop(J, 2);
                js_pushnumber(J, (ix >> (uy & 0x1F)) as f64);
            }

            OP_USHR => {
                ux = js_touint32(J, -2);
                uy = js_touint32(J, -1);
                js_pop(J, 2);
                js_pushnumber(J, (ux >> (uy & 0x1F)) as f64);
            }

            /* Relational operators */
            OP_LT => {
                b = js_compare(J, &mut okay);
                js_pop(J, 2);
                js_pushboolean(J, (okay != 0 && b < 0) as c_int);
            }
            OP_GT => {
                b = js_compare(J, &mut okay);
                js_pop(J, 2);
                js_pushboolean(J, (okay != 0 && b > 0) as c_int);
            }
            OP_LE => {
                b = js_compare(J, &mut okay);
                js_pop(J, 2);
                js_pushboolean(J, (okay != 0 && b <= 0) as c_int);
            }
            OP_GE => {
                b = js_compare(J, &mut okay);
                js_pop(J, 2);
                js_pushboolean(J, (okay != 0 && b >= 0) as c_int);
            }

            OP_INSTANCEOF => {
                b = js_instanceof(J);
                js_pop(J, 2);
                js_pushboolean(J, b);
            }

            /* Equality */
            OP_EQ => {
                b = js_equal(J);
                js_pop(J, 2);
                js_pushboolean(J, b);
            }
            OP_NE => {
                b = js_equal(J);
                js_pop(J, 2);
                js_pushboolean(J, (b == 0) as c_int);
            }
            OP_STRICTEQ => {
                b = js_strictequal(J);
                js_pop(J, 2);
                js_pushboolean(J, b);
            }
            OP_STRICTNE => {
                b = js_strictequal(J);
                js_pop(J, 2);
                js_pushboolean(J, (b == 0) as c_int);
            }

            OP_JCASE => {
                offset = *pc as c_int;
                pc = pc.offset(1);
                b = js_strictequal(J);
                if b != 0 {
                    js_pop(J, 2);
                    pc = pcstart.offset(offset as isize);
                } else {
                    js_pop(J, 1);
                }
            }

            /* Binary bitwise operators */
            OP_BITAND => {
                ix = js_toint32(J, -2);
                iy = js_toint32(J, -1);
                js_pop(J, 2);
                js_pushnumber(J, (ix & iy) as f64);
            }

            OP_BITXOR => {
                ix = js_toint32(J, -2);
                iy = js_toint32(J, -1);
                js_pop(J, 2);
                js_pushnumber(J, (ix ^ iy) as f64);
            }

            OP_BITOR => {
                ix = js_toint32(J, -2);
                iy = js_toint32(J, -1);
                js_pop(J, 2);
                js_pushnumber(J, (ix | iy) as f64);
            }

            /* Try and Catch */
            OP_THROW => {
                js_throw(J);
            }

            OP_TRY => {
                offset = *pc as c_int;
                pc = pc.offset(1);
                js_pushtry_internal(J, pc, owner);
                pc = pcstart.offset(offset as isize);
            }

            OP_ENDTRY => {
                js_endtry(J);
            }

            OP_CATCH => {
                READSTRING!(pc, str);
                obj = jsV_newobject(J, JS_COBJECT, null_mut());
                js_pushobject(J, obj);
                js_rot2(J);
                js_setproperty(J, -2, str);
                (*J).E = jsR_newenvironment(J, obj, (*J).E);
                js_pop(J, 1);
            }

            OP_ENDCATCH => {
                (*J).E = (*(*J).E).outer;
            }

            /* With */
            OP_WITH => {
                obj = js_toobject(J, -1);
                (*J).E = jsR_newenvironment(J, obj, (*J).E);
                js_pop(J, 1);
            }

            OP_ENDWITH => {
                (*J).E = (*(*J).E).outer;
            }

            /* Branching */
            OP_DEBUGGER => {
                js_trap(
                    J,
                    (pc.offset_from(pcstart) as c_int) - 1,
                );
            }

            OP_JUMP => {
                pc = pcstart.offset(*pc as isize);
            }

            OP_JTRUE => {
                offset = *pc as c_int;
                pc = pc.offset(1);
                b = js_toboolean(J, -1);
                js_pop(J, 1);
                if b != 0 {
                    pc = pcstart.offset(offset as isize);
                }
            }

            OP_JFALSE => {
                offset = *pc as c_int;
                pc = pc.offset(1);
                b = js_toboolean(J, -1);
                js_pop(J, 1);
                if b == 0 {
                    pc = pcstart.offset(offset as isize);
                }
            }

            OP_RETURN => {
                (*J).strict = savestrict;
                return;
            }

            _ => {}
        }
    }
}

/* forward declarations resolved through other modules */
use crate::jsregexp::js_newregexp;
use crate::jsstate::js_loadeval;
use crate::jsstring::js_runeat;
