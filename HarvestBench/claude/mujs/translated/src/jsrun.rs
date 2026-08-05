//! Translated from jsrun.c — memory, stack, calls, exceptions, interpreter loop.
#![allow(non_snake_case, non_upper_case_globals)]

use crate::cutil::*;
use crate::types::*;
use std::os::raw::{c_char, c_int, c_uint, c_void};
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};

/* Push values on stack (macros in C) */
#[inline]
unsafe fn STACK(J: *mut js_State) -> *mut js_Value {
    (*J).stack
}
#[inline]
unsafe fn TOP(J: *mut js_State) -> c_int {
    (*J).top
}

macro_rules! cstr {
    ($s:literal) => {
        concat!($s, "\0").as_ptr() as *const c_char
    };
}

pub(crate) unsafe fn js_trystackoverflow(J: *mut js_State) {
    let top = (*J).top as usize;
    (*(*J).stack.add(top)).set_type(JS_TLITSTR);
    (*(*J).stack.add(top)).u.litstr = cstr!("exception stack overflow");
    (*J).top += 1;
    js_throw(J);
}

unsafe fn js_stackoverflow(J: *mut js_State) {
    let top = (*J).top as usize;
    (*(*J).stack.add(top)).set_type(JS_TLITSTR);
    (*(*J).stack.add(top)).u.litstr = cstr!("stack overflow");
    (*J).top += 1;
    js_throw(J);
}

unsafe fn js_outofmemory(J: *mut js_State) {
    let top = (*J).top as usize;
    (*(*J).stack.add(top)).set_type(JS_TLITSTR);
    (*(*J).stack.add(top)).u.litstr = cstr!("out of memory");
    (*J).top += 1;
    js_throw(J);
}

unsafe fn js_runlimit_err(J: *mut js_State) {
    let top = (*J).top as usize;
    (*(*J).stack.add(top)).set_type(JS_TLITSTR);
    (*(*J).stack.add(top)).u.litstr = cstr!("script ran too long");
    (*J).top += 1;
    js_throw(J);
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_setlimit(J: *mut js_State, runlimit: c_int, memlimit: c_int) {
    (*J).runlimit = runlimit;
    (*J).memlimit = memlimit;
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_malloc(J: *mut js_State, size: c_int) -> *mut c_void {
    let mut ptr: *mut c_void;
    if (*J).memlimit > 0 {
        if size >= (*J).memlimit {
            js_outofmemory(J);
        }
        (*J).memlimit -= size;
    }
    ptr = ((*J).alloc.unwrap())((*J).actx, std::ptr::null_mut(), size);
    if ptr.is_null() {
        js_outofmemory(J);
    }
    ptr
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_realloc(J: *mut js_State, ptr: *mut c_void, size: c_int) -> *mut c_void {
    let mut ptr = ptr;
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

#[no_mangle]
pub unsafe extern "C-unwind" fn js_strdup(J: *mut js_State, s: *const c_char) -> *mut c_char {
    let n = (strlen(s) + 1) as c_int;
    let p = js_malloc(J, n) as *mut c_char;
    memcpy(p, s, n as usize);
    p
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_free(J: *mut js_State, ptr: *mut c_void) {
    ((*J).alloc.unwrap())((*J).actx, ptr, 0);
}

#[no_mangle]
pub unsafe extern "C-unwind" fn jsV_newmemstring(J: *mut js_State, s: *const c_char, n: c_int) -> *mut js_String {
    // soffsetof(js_String, p) + n + 1
    let base = std::mem::offset_of!(js_String, p) as c_int;
    let v = js_malloc(J, base + n + 1) as *mut js_String;
    let p = (*v).p.as_mut_ptr();
    memcpy(p, s, n as usize);
    *p.add(n as usize) = 0;
    (*v).gcmark = 0;
    (*v).gcnext = (*J).gcstr;
    (*J).gcstr = v;
    (*J).gccounter += 1;
    v
}

macro_rules! checkstack {
    ($J:expr, $n:expr) => {
        if (*$J).top + $n >= JS_STACKSIZE as c_int {
            js_stackoverflow($J);
        }
    };
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_pushvalue(J: *mut js_State, v: js_Value) {
    checkstack!(J, 1);
    *(*J).stack.add((*J).top as usize) = v;
    (*J).top += 1;
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_pushundefined(J: *mut js_State) {
    checkstack!(J, 1);
    (*(*J).stack.add((*J).top as usize)).set_type(JS_TUNDEFINED);
    (*J).top += 1;
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_pushnull(J: *mut js_State) {
    checkstack!(J, 1);
    (*(*J).stack.add((*J).top as usize)).set_type(JS_TNULL);
    (*J).top += 1;
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_pushboolean(J: *mut js_State, v: c_int) {
    checkstack!(J, 1);
    let s = (*J).stack.add((*J).top as usize);
    (*s).set_type(JS_TBOOLEAN);
    (*s).u.boolean = if v != 0 { 1 } else { 0 };
    (*J).top += 1;
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_pushnumber(J: *mut js_State, v: f64) {
    checkstack!(J, 1);
    let s = (*J).stack.add((*J).top as usize);
    (*s).set_type(JS_TNUMBER);
    (*s).u.number = v;
    (*J).top += 1;
}

const SHRLEN: c_int = 15; // soffsetof(js_Value, t.type)

#[no_mangle]
pub unsafe extern "C-unwind" fn js_pushstring(J: *mut js_State, v: *const c_char) {
    let mut n = strlen(v) as c_int;
    if n > JS_STRLIMIT {
        crate::jserror::js_rangeerror(J, cstr!("invalid string length"));
    }
    checkstack!(J, 1);
    let slot = (*J).stack.add((*J).top as usize);
    if n <= SHRLEN {
        let mut s = (*slot).u.shrstr.as_mut_ptr();
        let mut vv = v;
        while n > 0 {
            *s = *vv;
            s = s.add(1);
            vv = vv.add(1);
            n -= 1;
        }
        *s = 0;
        (*slot).set_type(JS_TSHRSTR);
    } else {
        (*slot).set_type(JS_TMEMSTR);
        (*slot).u.memstr = jsV_newmemstring(J, v, n);
    }
    (*J).top += 1;
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_pushlstring(J: *mut js_State, v: *const c_char, n: c_int) {
    let mut n = n;
    if n > JS_STRLIMIT {
        crate::jserror::js_rangeerror(J, cstr!("invalid string length"));
    }
    checkstack!(J, 1);
    let slot = (*J).stack.add((*J).top as usize);
    if n <= SHRLEN {
        let mut s = (*slot).u.shrstr.as_mut_ptr();
        let mut vv = v;
        while n > 0 {
            *s = *vv;
            s = s.add(1);
            vv = vv.add(1);
            n -= 1;
        }
        *s = 0;
        (*slot).set_type(JS_TSHRSTR);
    } else {
        (*slot).set_type(JS_TMEMSTR);
        (*slot).u.memstr = jsV_newmemstring(J, v, n);
    }
    (*J).top += 1;
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_pushliteral(J: *mut js_State, v: *const c_char) {
    checkstack!(J, 1);
    let slot = (*J).stack.add((*J).top as usize);
    (*slot).set_type(JS_TLITSTR);
    (*slot).u.litstr = v;
    (*J).top += 1;
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_pushobject(J: *mut js_State, v: *mut js_Object) {
    checkstack!(J, 1);
    let slot = (*J).stack.add((*J).top as usize);
    (*slot).set_type(JS_TOBJECT);
    (*slot).u.object = v;
    (*J).top += 1;
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_pushglobal(J: *mut js_State) {
    js_pushobject(J, (*J).G);
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_currentfunction(J: *mut js_State) {
    checkstack!(J, 1);
    let top = (*J).top as usize;
    if (*J).bot > 0 {
        *(*J).stack.add(top) = *(*J).stack.add(((*J).bot - 1) as usize);
    } else {
        (*(*J).stack.add(top)).set_type(JS_TUNDEFINED);
    }
    (*J).top += 1;
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_currentfunctiondata(J: *mut js_State) -> *mut c_void {
    if (*J).bot > 0 {
        let o = (*(*J).stack.add(((*J).bot - 1) as usize)).u.object;
        return (*o).u.c.data;
    }
    std::ptr::null_mut()
}

/* Read values from stack */
static mut UNDEFINED: js_Value = js_Value { t: JsValueT { pad: [0; 15], type_: JS_TUNDEFINED } };

pub(crate) unsafe fn stackidx(J: *mut js_State, idx: c_int) -> *mut js_Value {
    let idx = if idx < 0 { (*J).top + idx } else { (*J).bot + idx };
    if idx < 0 || idx >= (*J).top {
        return std::ptr::addr_of_mut!(UNDEFINED);
    }
    (*J).stack.add(idx as usize)
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_tovalue(J: *mut js_State, idx: c_int) -> *mut js_Value {
    stackidx(J, idx)
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_isdefined(J: *mut js_State, idx: c_int) -> c_int {
    ((*stackidx(J, idx)).type_() != JS_TUNDEFINED) as c_int
}
#[no_mangle]
pub unsafe extern "C-unwind" fn js_isundefined(J: *mut js_State, idx: c_int) -> c_int {
    ((*stackidx(J, idx)).type_() == JS_TUNDEFINED) as c_int
}
#[no_mangle]
pub unsafe extern "C-unwind" fn js_isnull(J: *mut js_State, idx: c_int) -> c_int {
    ((*stackidx(J, idx)).type_() == JS_TNULL) as c_int
}
#[no_mangle]
pub unsafe extern "C-unwind" fn js_isboolean(J: *mut js_State, idx: c_int) -> c_int {
    ((*stackidx(J, idx)).type_() == JS_TBOOLEAN) as c_int
}
#[no_mangle]
pub unsafe extern "C-unwind" fn js_isnumber(J: *mut js_State, idx: c_int) -> c_int {
    ((*stackidx(J, idx)).type_() == JS_TNUMBER) as c_int
}
#[no_mangle]
pub unsafe extern "C-unwind" fn js_isstring(J: *mut js_State, idx: c_int) -> c_int {
    let t = (*stackidx(J, idx)).type_();
    (t == JS_TSHRSTR || t == JS_TLITSTR || t == JS_TMEMSTR) as c_int
}
#[no_mangle]
pub unsafe extern "C-unwind" fn js_isprimitive(J: *mut js_State, idx: c_int) -> c_int {
    ((*stackidx(J, idx)).type_() != JS_TOBJECT) as c_int
}
#[no_mangle]
pub unsafe extern "C-unwind" fn js_isobject(J: *mut js_State, idx: c_int) -> c_int {
    ((*stackidx(J, idx)).type_() == JS_TOBJECT) as c_int
}
#[no_mangle]
pub unsafe extern "C-unwind" fn js_iscoercible(J: *mut js_State, idx: c_int) -> c_int {
    let v = stackidx(J, idx);
    ((*v).type_() != JS_TUNDEFINED && (*v).type_() != JS_TNULL) as c_int
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_iscallable(J: *mut js_State, idx: c_int) -> c_int {
    let v = stackidx(J, idx);
    if (*v).type_() == JS_TOBJECT {
        let t = (*(*v).u.object).type_;
        return (t == JS_CFUNCTION || t == JS_CSCRIPT || t == JS_CCFUNCTION) as c_int;
    }
    0
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_isarray(J: *mut js_State, idx: c_int) -> c_int {
    let v = stackidx(J, idx);
    ((*v).type_() == JS_TOBJECT && (*(*v).u.object).type_ == JS_CARRAY) as c_int
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_isregexp(J: *mut js_State, idx: c_int) -> c_int {
    let v = stackidx(J, idx);
    ((*v).type_() == JS_TOBJECT && (*(*v).u.object).type_ == JS_CREGEXP) as c_int
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_isuserdata(J: *mut js_State, idx: c_int, tag: *const c_char) -> c_int {
    let v = stackidx(J, idx);
    if (*v).type_() == JS_TOBJECT && (*(*v).u.object).type_ == JS_CUSERDATA {
        return (strcmp(tag, (*(*v).u.object).u.user.tag) == 0) as c_int;
    }
    0
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_iserror(J: *mut js_State, idx: c_int) -> c_int {
    let v = stackidx(J, idx);
    ((*v).type_() == JS_TOBJECT && (*(*v).u.object).type_ == JS_CERROR) as c_int
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_typeof(J: *mut js_State, idx: c_int) -> *const c_char {
    let v = stackidx(J, idx);
    match (*v).type_() {
        JS_TUNDEFINED => cstr!("undefined"),
        JS_TNULL => cstr!("object"),
        JS_TBOOLEAN => cstr!("boolean"),
        JS_TNUMBER => cstr!("number"),
        JS_TLITSTR | JS_TMEMSTR | JS_TSHRSTR => cstr!("string"),
        JS_TOBJECT => {
            let t = (*(*v).u.object).type_;
            if t == JS_CFUNCTION || t == JS_CCFUNCTION {
                cstr!("function")
            } else {
                cstr!("object")
            }
        }
        _ => cstr!("string"),
    }
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_type(J: *mut js_State, idx: c_int) -> c_int {
    let v = stackidx(J, idx);
    match (*v).type_() {
        JS_TUNDEFINED => JS_ISUNDEFINED,
        JS_TNULL => JS_ISNULL,
        JS_TBOOLEAN => JS_ISBOOLEAN,
        JS_TNUMBER => JS_ISNUMBER,
        JS_TLITSTR | JS_TMEMSTR | JS_TSHRSTR => JS_ISSTRING,
        JS_TOBJECT => {
            let t = (*(*v).u.object).type_;
            if t == JS_CFUNCTION || t == JS_CCFUNCTION {
                JS_ISFUNCTION
            } else {
                JS_ISOBJECT
            }
        }
        _ => JS_ISSTRING,
    }
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_toboolean(J: *mut js_State, idx: c_int) -> c_int {
    crate::jsvalue::jsV_toboolean(J, stackidx(J, idx))
}
#[no_mangle]
pub unsafe extern "C-unwind" fn js_tonumber(J: *mut js_State, idx: c_int) -> f64 {
    crate::jsvalue::jsV_tonumber(J, stackidx(J, idx))
}
#[no_mangle]
pub unsafe extern "C-unwind" fn js_tointeger(J: *mut js_State, idx: c_int) -> c_int {
    crate::jsvalue::jsV_numbertointeger(crate::jsvalue::jsV_tonumber(J, stackidx(J, idx)))
}
#[no_mangle]
pub unsafe extern "C-unwind" fn js_toint32(J: *mut js_State, idx: c_int) -> c_int {
    crate::jsvalue::jsV_numbertoint32(crate::jsvalue::jsV_tonumber(J, stackidx(J, idx)))
}
#[no_mangle]
pub unsafe extern "C-unwind" fn js_touint32(J: *mut js_State, idx: c_int) -> c_uint {
    crate::jsvalue::jsV_numbertouint32(crate::jsvalue::jsV_tonumber(J, stackidx(J, idx)))
}
#[no_mangle]
pub unsafe extern "C-unwind" fn js_toint16(J: *mut js_State, idx: c_int) -> i16 {
    crate::jsvalue::jsV_numbertoint16(crate::jsvalue::jsV_tonumber(J, stackidx(J, idx)))
}
#[no_mangle]
pub unsafe extern "C-unwind" fn js_touint16(J: *mut js_State, idx: c_int) -> u16 {
    crate::jsvalue::jsV_numbertouint16(crate::jsvalue::jsV_tonumber(J, stackidx(J, idx)))
}
#[no_mangle]
pub unsafe extern "C-unwind" fn js_tostring(J: *mut js_State, idx: c_int) -> *const c_char {
    crate::jsvalue::jsV_tostring(J, stackidx(J, idx))
}
#[no_mangle]
pub unsafe extern "C-unwind" fn js_toobject(J: *mut js_State, idx: c_int) -> *mut js_Object {
    crate::jsvalue::jsV_toobject(J, stackidx(J, idx))
}
#[no_mangle]
pub unsafe extern "C-unwind" fn js_toprimitive(J: *mut js_State, idx: c_int, hint: c_int) {
    crate::jsvalue::jsV_toprimitive(J, stackidx(J, idx), hint);
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_toregexp(J: *mut js_State, idx: c_int) -> *mut js_Regexp {
    let v = stackidx(J, idx);
    if (*v).type_() == JS_TOBJECT && (*(*v).u.object).type_ == JS_CREGEXP {
        return std::ptr::addr_of_mut!((*(*v).u.object).u.r);
    }
    crate::jserror::js_typeerror(J, cstr!("not a regexp"));
    unreachable!()
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_touserdata(J: *mut js_State, idx: c_int, tag: *const c_char) -> *mut c_void {
    let v = stackidx(J, idx);
    if (*v).type_() == JS_TOBJECT && (*(*v).u.object).type_ == JS_CUSERDATA {
        if strcmp(tag, (*(*v).u.object).u.user.tag) == 0 {
            return (*(*v).u.object).u.user.data;
        }
    }
    crate::jserror::js_typeerror(J, cstr!("not a %s"), tag);
    unreachable!()
}

unsafe fn jsR_tofunction(J: *mut js_State, idx: c_int) -> *mut js_Object {
    let v = stackidx(J, idx);
    if (*v).type_() == JS_TUNDEFINED || (*v).type_() == JS_TNULL {
        return std::ptr::null_mut();
    }
    if (*v).type_() == JS_TOBJECT {
        let t = (*(*v).u.object).type_;
        if t == JS_CFUNCTION || t == JS_CCFUNCTION {
            return (*v).u.object;
        }
    }
    crate::jserror::js_typeerror(J, cstr!("not a function"));
    unreachable!()
}

/* Stack manipulation */
#[no_mangle]
pub unsafe extern "C-unwind" fn js_gettop(J: *mut js_State) -> c_int {
    (*J).top - (*J).bot
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_pop(J: *mut js_State, n: c_int) {
    (*J).top -= n;
    if (*J).top < (*J).bot {
        (*J).top = (*J).bot;
        crate::jserror::js_error(J, cstr!("stack underflow!"));
    }
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_remove(J: *mut js_State, idx: c_int) {
    let mut idx = if idx < 0 { (*J).top + idx } else { (*J).bot + idx };
    if idx < (*J).bot || idx >= (*J).top {
        crate::jserror::js_error(J, cstr!("stack error!"));
    }
    while idx < (*J).top - 1 {
        *(*J).stack.add(idx as usize) = *(*J).stack.add((idx + 1) as usize);
        idx += 1;
    }
    (*J).top -= 1;
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_insert(J: *mut js_State, _idx: c_int) {
    crate::jserror::js_error(J, cstr!("not implemented yet"));
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_replace(J: *mut js_State, idx: c_int) {
    let idx = if idx < 0 { (*J).top + idx } else { (*J).bot + idx };
    if idx < (*J).bot || idx >= (*J).top {
        crate::jserror::js_error(J, cstr!("stack error!"));
    }
    (*J).top -= 1;
    *(*J).stack.add(idx as usize) = *(*J).stack.add((*J).top as usize);
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_copy(J: *mut js_State, idx: c_int) {
    checkstack!(J, 1);
    *(*J).stack.add((*J).top as usize) = *stackidx(J, idx);
    (*J).top += 1;
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_dup(J: *mut js_State) {
    checkstack!(J, 1);
    *(*J).stack.add((*J).top as usize) = *(*J).stack.add(((*J).top - 1) as usize);
    (*J).top += 1;
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_dup2(J: *mut js_State) {
    checkstack!(J, 2);
    let top = (*J).top;
    *(*J).stack.add(top as usize) = *(*J).stack.add((top - 2) as usize);
    *(*J).stack.add((top + 1) as usize) = *(*J).stack.add((top - 1) as usize);
    (*J).top += 2;
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_rot2(J: *mut js_State) {
    let top = (*J).top;
    let tmp = *(*J).stack.add((top - 1) as usize);
    *(*J).stack.add((top - 1) as usize) = *(*J).stack.add((top - 2) as usize);
    *(*J).stack.add((top - 2) as usize) = tmp;
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_rot3(J: *mut js_State) {
    let top = (*J).top;
    let tmp = *(*J).stack.add((top - 1) as usize);
    *(*J).stack.add((top - 1) as usize) = *(*J).stack.add((top - 2) as usize);
    *(*J).stack.add((top - 2) as usize) = *(*J).stack.add((top - 3) as usize);
    *(*J).stack.add((top - 3) as usize) = tmp;
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_rot4(J: *mut js_State) {
    let top = (*J).top;
    let tmp = *(*J).stack.add((top - 1) as usize);
    *(*J).stack.add((top - 1) as usize) = *(*J).stack.add((top - 2) as usize);
    *(*J).stack.add((top - 2) as usize) = *(*J).stack.add((top - 3) as usize);
    *(*J).stack.add((top - 3) as usize) = *(*J).stack.add((top - 4) as usize);
    *(*J).stack.add((top - 4) as usize) = tmp;
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_rot2pop1(J: *mut js_State) {
    let top = (*J).top;
    *(*J).stack.add((top - 2) as usize) = *(*J).stack.add((top - 1) as usize);
    (*J).top -= 1;
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_rot3pop2(J: *mut js_State) {
    let top = (*J).top;
    *(*J).stack.add((top - 3) as usize) = *(*J).stack.add((top - 1) as usize);
    (*J).top -= 2;
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_rot(J: *mut js_State, n: c_int) {
    let top = (*J).top;
    let tmp = *(*J).stack.add((top - 1) as usize);
    let mut i = 1;
    while i < n {
        *(*J).stack.add((top - i) as usize) = *(*J).stack.add((top - i - 1) as usize);
        i += 1;
    }
    *(*J).stack.add((top - i) as usize) = tmp;
}

/* Property access */
#[no_mangle]
pub unsafe extern "C-unwind" fn js_isarrayindex(J: *mut js_State, p: *const c_char, idx: *mut c_int) -> c_int {
    let mut n: c_int = 0;
    let mut p = p;

    if *p.add(0) == 0 {
        return 0;
    }
    if *p.add(0) == b'0' as c_char {
        if *p.add(1) == 0 {
            *idx = 0;
            return 1;
        }
        return 0;
    }
    while *p != 0 {
        let c = *p as c_int;
        p = p.add(1);
        if c >= '0' as c_int && c <= '9' as c_int {
            if n >= c_int::MAX / 10 {
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
    let mut buf: [c_char; (UTFmax_usize + 1)] = [0; UTFmax_usize + 1];
    if rune >= 0 {
        let mut r = rune;
        let n = crate::utf::runetochar(buf.as_mut_ptr(), &mut r);
        buf[n as usize] = 0;
        js_pushstring(J, buf.as_ptr());
    } else {
        js_pushundefined(J);
    }
}
const UTFmax_usize: usize = 4;

#[no_mangle]
pub unsafe extern "C-unwind" fn jsR_unflattenarray(J: *mut js_State, obj: *mut js_Object) {
    if (*obj).type_ == JS_CARRAY && (*obj).u.a.simple != 0 {
        let mut name: [c_char; 32] = [0; 32];
        let caught = protect(J, || {
            let mut i = 0;
            while i < (*obj).u.a.flat_length {
                crate::jsvalue::js_itoa(name.as_mut_ptr(), i);
                let rf = crate::jsproperty::jsV_setproperty(J, obj, name.as_ptr());
                (*rf).value = *(*obj).u.a.array.add(i as usize);
                i += 1;
            }
            js_free(J, (*obj).u.a.array as *mut c_void);
            (*obj).u.a.simple = 0;
            (*obj).u.a.flat_length = 0;
            (*obj).u.a.flat_capacity = 0;
            (*obj).u.a.array = std::ptr::null_mut();
        });
        if caught {
            (*obj).properties = std::ptr::null_mut();
            js_throw(J);
        } else {
            js_endtry(J);
        }
    }
}

unsafe fn jsR_hasproperty(J: *mut js_State, obj: *mut js_Object, name: *const c_char) -> c_int {
    let mut k: c_int = 0;

    if (*obj).type_ == JS_CARRAY {
        if strcmp(name, cstr!("length")) == 0 {
            js_pushnumber(J, (*obj).u.a.length as f64);
            return 1;
        }
        if (*obj).u.a.simple != 0 {
            if js_isarrayindex(J, name, &mut k) != 0 {
                if k >= 0 && k < (*obj).u.a.flat_length {
                    js_pushvalue(J, *(*obj).u.a.array.add(k as usize));
                    return 1;
                }
                return 0;
            }
        }
    } else if (*obj).type_ == JS_CSTRING {
        if strcmp(name, cstr!("length")) == 0 {
            js_pushnumber(J, (*obj).u.s.length as f64);
            return 1;
        }
        if js_isarrayindex(J, name, &mut k) != 0 {
            if k >= 0 && k < (*obj).u.s.length {
                js_pushrune(J, crate::jsstring::js_runeat(J, (*obj).u.s.string, k));
                return 1;
            }
        }
    } else if (*obj).type_ == JS_CREGEXP {
        if strcmp(name, cstr!("source")) == 0 {
            js_pushstring(J, (*obj).u.r.source);
            return 1;
        }
        if strcmp(name, cstr!("global")) == 0 {
            js_pushboolean(J, (*obj).u.r.flags as c_int & JS_REGEXP_G);
            return 1;
        }
        if strcmp(name, cstr!("ignoreCase")) == 0 {
            js_pushboolean(J, (*obj).u.r.flags as c_int & JS_REGEXP_I);
            return 1;
        }
        if strcmp(name, cstr!("multiline")) == 0 {
            js_pushboolean(J, (*obj).u.r.flags as c_int & JS_REGEXP_M);
            return 1;
        }
        if strcmp(name, cstr!("lastIndex")) == 0 {
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

    let rf = crate::jsproperty::jsV_getproperty(J, obj, name);
    if !rf.is_null() {
        if !(*rf).getter.is_null() {
            js_pushobject(J, (*rf).getter);
            js_pushobject(J, obj);
            js_call(J, 0);
        } else {
            js_pushvalue(J, (*rf).value);
        }
        return 1;
    }
    0
}

unsafe fn jsR_getproperty(J: *mut js_State, obj: *mut js_Object, name: *const c_char) {
    if jsR_hasproperty(J, obj, name) == 0 {
        js_pushundefined(J);
    }
}

unsafe fn jsR_hasindex(J: *mut js_State, obj: *mut js_Object, k: c_int) -> c_int {
    let mut buf: [c_char; 32] = [0; 32];
    if (*obj).type_ == JS_CARRAY && (*obj).u.a.simple != 0 {
        if k >= 0 && k < (*obj).u.a.flat_length {
            js_pushvalue(J, *(*obj).u.a.array.add(k as usize));
            return 1;
        }
        return 0;
    }
    jsR_hasproperty(J, obj, crate::jsvalue::js_itoa(buf.as_mut_ptr(), k))
}

unsafe fn jsR_getindex(J: *mut js_State, obj: *mut js_Object, k: c_int) {
    if jsR_hasindex(J, obj, k) == 0 {
        js_pushundefined(J);
    }
}

unsafe fn jsR_setarrayindex(J: *mut js_State, obj: *mut js_Object, k: c_int, value: *mut js_Value) {
    let newlen = k + 1;
    if newlen > JS_ARRAYLIMIT {
        crate::jserror::js_rangeerror(J, cstr!("array too large"));
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
                newcap * std::mem::size_of::<js_Value>() as c_int,
            ) as *mut js_Value;
            (*obj).u.a.flat_capacity = newcap;
        }
        (*obj).u.a.flat_length = newlen;
    }
    if newlen > (*obj).u.a.length {
        (*obj).u.a.length = newlen;
    }
    *(*obj).u.a.array.add(k as usize) = *value;
}

pub(crate) unsafe fn jsR_setproperty(J: *mut js_State, obj: *mut js_Object, name: *const c_char, transient: c_int) {
    let value = stackidx(J, -1);
    let mut rf: *mut js_Property;
    let mut k: c_int = 0;
    let mut own: c_int = 0;

    'outer: {
        if (*obj).type_ == JS_CARRAY {
            if strcmp(name, cstr!("length")) == 0 {
                let rawlen = crate::jsvalue::jsV_tonumber(J, value);
                let newlen = crate::jsvalue::jsV_numbertointeger(rawlen);
                if newlen as f64 != rawlen || newlen < 0 {
                    crate::jserror::js_rangeerror(J, cstr!("invalid array length"));
                }
                if newlen > JS_ARRAYLIMIT {
                    crate::jserror::js_rangeerror(J, cstr!("array too large"));
                }
                if (*obj).u.a.simple != 0 {
                    (*obj).u.a.length = newlen;
                    if newlen <= (*obj).u.a.flat_length {
                        (*obj).u.a.flat_length = newlen;
                    }
                } else {
                    crate::jsproperty::jsV_resizearray(J, obj, newlen);
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
            if strcmp(name, cstr!("length")) == 0 {
                break 'outer;
            }
            if js_isarrayindex(J, name, &mut k) != 0 {
                if k >= 0 && k < (*obj).u.s.length {
                    break 'outer;
                }
            }
        } else if (*obj).type_ == JS_CREGEXP {
            if strcmp(name, cstr!("source")) == 0 {
                break 'outer;
            }
            if strcmp(name, cstr!("global")) == 0 {
                break 'outer;
            }
            if strcmp(name, cstr!("ignoreCase")) == 0 {
                break 'outer;
            }
            if strcmp(name, cstr!("multiline")) == 0 {
                break 'outer;
            }
            if strcmp(name, cstr!("lastIndex")) == 0 {
                (*obj).u.r.last = crate::jsvalue::jsV_tointeger(J, value) as u16;
                return;
            }
        } else if (*obj).type_ == JS_CUSERDATA {
            if let Some(put) = (*obj).u.user.put {
                if put(J, (*obj).u.user.data, name) != 0 {
                    return;
                }
            }
        }

        rf = crate::jsproperty::jsV_getpropertyx(J, obj, name, &mut own);
        if !rf.is_null() {
            if !(*rf).setter.is_null() {
                js_pushobject(J, (*rf).setter);
                js_pushobject(J, obj);
                js_pushvalue(J, *value);
                js_call(J, 1);
                js_pop(J, 1);
                return;
            } else {
                if (*J).strict != 0 {
                    if !(*rf).getter.is_null() {
                        crate::jserror::js_typeerror(
                            J,
                            cstr!("setting property '%s' that only has a getter"),
                            name,
                        );
                    }
                }
                if (*rf).atts & JS_READONLY != 0 {
                    break 'outer;
                }
            }
        }

        if rf.is_null() || own == 0 {
            if transient != 0 {
                if (*J).strict != 0 {
                    crate::jserror::js_typeerror(
                        J,
                        cstr!("cannot create property '%s' on transient object"),
                        name,
                    );
                }
                return;
            }
            rf = crate::jsproperty::jsV_setproperty(J, obj, name);
        }

        if !rf.is_null() {
            if (*rf).atts & JS_READONLY == 0 {
                (*rf).value = *value;
            } else {
                break 'outer;
            }
        }
        return;
    }
    // readonly:
    if (*J).strict != 0 {
        crate::jserror::js_typeerror(J, cstr!("'%s' is read-only"), name);
    }
}

unsafe fn jsR_setindex(J: *mut js_State, obj: *mut js_Object, k: c_int, transient: c_int) {
    let mut buf: [c_char; 32] = [0; 32];
    if (*obj).type_ == JS_CARRAY && (*obj).u.a.simple != 0 && k >= 0 && k <= (*obj).u.a.flat_length {
        jsR_setarrayindex(J, obj, k, stackidx(J, -1));
    } else {
        jsR_setproperty(J, obj, crate::jsvalue::js_itoa(buf.as_mut_ptr(), k), transient);
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
    throw: c_int,
) {
    let mut rf: *mut js_Property;
    let mut k: c_int = 0;

    'outer: {
        if (*obj).type_ == JS_CARRAY {
            if strcmp(name, cstr!("length")) == 0 {
                break 'outer;
            }
            if (*obj).u.a.simple != 0 {
                jsR_unflattenarray(J, obj);
            }
        } else if (*obj).type_ == JS_CSTRING {
            if strcmp(name, cstr!("length")) == 0 {
                break 'outer;
            }
            if js_isarrayindex(J, name, &mut k) != 0 {
                if k >= 0 && k < (*obj).u.s.length {
                    break 'outer;
                }
            }
        } else if (*obj).type_ == JS_CREGEXP {
            if strcmp(name, cstr!("source")) == 0 {
                break 'outer;
            }
            if strcmp(name, cstr!("global")) == 0 {
                break 'outer;
            }
            if strcmp(name, cstr!("ignoreCase")) == 0 {
                break 'outer;
            }
            if strcmp(name, cstr!("multiline")) == 0 {
                break 'outer;
            }
            if strcmp(name, cstr!("lastIndex")) == 0 {
                break 'outer;
            }
        } else if (*obj).type_ == JS_CUSERDATA {
            if let Some(put) = (*obj).u.user.put {
                if put(J, (*obj).u.user.data, name) != 0 {
                    return;
                }
            }
        }

        rf = crate::jsproperty::jsV_setproperty(J, obj, name);
        if !rf.is_null() {
            if !value.is_null() {
                if (*rf).atts & JS_READONLY == 0 {
                    (*rf).value = *value;
                } else if (*J).strict != 0 {
                    crate::jserror::js_typeerror(J, cstr!("'%s' is read-only"), name);
                }
            }
            if !getter.is_null() {
                if (*rf).atts & JS_DONTCONF == 0 {
                    (*rf).getter = getter;
                } else if (*J).strict != 0 {
                    crate::jserror::js_typeerror(J, cstr!("'%s' is non-configurable"), name);
                }
            }
            if !setter.is_null() {
                if (*rf).atts & JS_DONTCONF == 0 {
                    (*rf).setter = setter;
                } else if (*J).strict != 0 {
                    crate::jserror::js_typeerror(J, cstr!("'%s' is non-configurable"), name);
                }
            }
            (*rf).atts |= atts;
        }
        return;
    }
    // readonly:
    if (*J).strict != 0 || throw != 0 {
        crate::jserror::js_typeerror(J, cstr!("'%s' is read-only or non-configurable"), name);
    }
}

unsafe fn jsR_delproperty(J: *mut js_State, obj: *mut js_Object, name: *const c_char) -> c_int {
    let mut k: c_int = 0;

    'outer: {
        if (*obj).type_ == JS_CARRAY {
            if strcmp(name, cstr!("length")) == 0 {
                break 'outer;
            }
            if (*obj).u.a.simple != 0 {
                jsR_unflattenarray(J, obj);
            }
        } else if (*obj).type_ == JS_CSTRING {
            if strcmp(name, cstr!("length")) == 0 {
                break 'outer;
            }
            if js_isarrayindex(J, name, &mut k) != 0 {
                if k >= 0 && k < (*obj).u.s.length {
                    break 'outer;
                }
            }
        } else if (*obj).type_ == JS_CREGEXP {
            if strcmp(name, cstr!("source")) == 0 {
                break 'outer;
            }
            if strcmp(name, cstr!("global")) == 0 {
                break 'outer;
            }
            if strcmp(name, cstr!("ignoreCase")) == 0 {
                break 'outer;
            }
            if strcmp(name, cstr!("multiline")) == 0 {
                break 'outer;
            }
            if strcmp(name, cstr!("lastIndex")) == 0 {
                break 'outer;
            }
        } else if (*obj).type_ == JS_CUSERDATA {
            if let Some(del) = (*obj).u.user.delete {
                if del(J, (*obj).u.user.data, name) != 0 {
                    return 1;
                }
            }
        }

        let rf = crate::jsproperty::jsV_getownproperty(J, obj, name);
        if !rf.is_null() {
            if (*rf).atts & JS_DONTCONF != 0 {
                break 'outer;
            }
            crate::jsproperty::jsV_delproperty(J, obj, name);
        }
        return 1;
    }
    // dontconf:
    if (*J).strict != 0 {
        crate::jserror::js_typeerror(J, cstr!("'%s' is non-configurable"), name);
    }
    0
}

unsafe fn jsR_delindex(J: *mut js_State, obj: *mut js_Object, k: c_int) {
    let mut buf: [c_char; 32] = [0; 32];
    if (*obj).type_ == JS_CARRAY && (*obj).u.a.simple != 0 && k == (*obj).u.a.flat_length - 1 {
        (*obj).u.a.flat_length = k;
    } else {
        jsR_delproperty(J, obj, crate::jsvalue::js_itoa(buf.as_mut_ptr(), k));
    }
}

/* Registry, global and object property accessors */
#[no_mangle]
pub unsafe extern "C-unwind" fn js_ref(J: *mut js_State) -> *const c_char {
    let v = stackidx(J, -1);
    let s: *const c_char;
    let mut buf: [c_char; 32] = [0; 32];
    match (*v).type_() {
        JS_TUNDEFINED => s = cstr!("_Undefined"),
        JS_TNULL => s = cstr!("_Null"),
        JS_TBOOLEAN => {
            s = if (*v).u.boolean != 0 { cstr!("_True") } else { cstr!("_False") };
        }
        JS_TOBJECT => {
            libc::sprintf(buf.as_mut_ptr(), cstr!("%p"), (*v).u.object as *mut c_void);
            s = crate::jsintern::js_intern(J, buf.as_ptr());
        }
        _ => {
            libc::sprintf(buf.as_mut_ptr(), cstr!("%d"), (*J).nextref);
            (*J).nextref += 1;
            s = crate::jsintern::js_intern(J, buf.as_ptr());
        }
    }
    js_setregistry(J, s);
    s
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_unref(J: *mut js_State, ref_: *const c_char) {
    js_delregistry(J, ref_);
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_getregistry(J: *mut js_State, name: *const c_char) {
    jsR_getproperty(J, (*J).R, name);
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_setregistry(J: *mut js_State, name: *const c_char) {
    jsR_setproperty(J, (*J).R, name, 0);
    js_pop(J, 1);
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_delregistry(J: *mut js_State, name: *const c_char) {
    jsR_delproperty(J, (*J).R, name);
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_getglobal(J: *mut js_State, name: *const c_char) {
    jsR_getproperty(J, (*J).G, name);
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_setglobal(J: *mut js_State, name: *const c_char) {
    jsR_setproperty(J, (*J).G, name, 0);
    js_pop(J, 1);
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_defglobal(J: *mut js_State, name: *const c_char, atts: c_int) {
    jsR_defproperty(J, (*J).G, name, atts, stackidx(J, -1), std::ptr::null_mut(), std::ptr::null_mut(), 0);
    js_pop(J, 1);
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_delglobal(J: *mut js_State, name: *const c_char) {
    jsR_delproperty(J, (*J).G, name);
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_getproperty(J: *mut js_State, idx: c_int, name: *const c_char) {
    jsR_getproperty(J, js_toobject(J, idx), name);
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_setproperty(J: *mut js_State, idx: c_int, name: *const c_char) {
    jsR_setproperty(J, js_toobject(J, idx), name, (js_isobject(J, idx) == 0) as c_int);
    js_pop(J, 1);
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_defproperty(J: *mut js_State, idx: c_int, name: *const c_char, atts: c_int) {
    jsR_defproperty(J, js_toobject(J, idx), name, atts, stackidx(J, -1), std::ptr::null_mut(), std::ptr::null_mut(), 1);
    js_pop(J, 1);
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_delproperty(J: *mut js_State, idx: c_int, name: *const c_char) {
    jsR_delproperty(J, js_toobject(J, idx), name);
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_defaccessor(J: *mut js_State, idx: c_int, name: *const c_char, atts: c_int) {
    jsR_defproperty(J, js_toobject(J, idx), name, atts, std::ptr::null_mut(), jsR_tofunction(J, -2), jsR_tofunction(J, -1), 1);
    js_pop(J, 2);
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_hasproperty(J: *mut js_State, idx: c_int, name: *const c_char) -> c_int {
    jsR_hasproperty(J, js_toobject(J, idx), name)
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_getindex(J: *mut js_State, idx: c_int, i: c_int) {
    jsR_getindex(J, js_toobject(J, idx), i);
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_hasindex(J: *mut js_State, idx: c_int, i: c_int) -> c_int {
    jsR_hasindex(J, js_toobject(J, idx), i)
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_setindex(J: *mut js_State, idx: c_int, i: c_int) {
    jsR_setindex(J, js_toobject(J, idx), i, (js_isobject(J, idx) == 0) as c_int);
    js_pop(J, 1);
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_delindex(J: *mut js_State, idx: c_int, i: c_int) {
    jsR_delindex(J, js_toobject(J, idx), i);
}

/* Iterator */
#[no_mangle]
pub unsafe extern "C-unwind" fn js_pushiterator(J: *mut js_State, idx: c_int, own: c_int) {
    js_pushobject(J, crate::jsproperty::jsV_newiterator(J, js_toobject(J, idx), own));
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_nextiterator(J: *mut js_State, idx: c_int) -> *const c_char {
    crate::jsproperty::jsV_nextiterator(J, js_toobject(J, idx))
}

/* Environment records */
#[no_mangle]
pub unsafe extern "C-unwind" fn jsR_newenvironment(J: *mut js_State, vars: *mut js_Object, outer: *mut js_Environment) -> *mut js_Environment {
    let E = js_malloc(J, std::mem::size_of::<js_Environment>() as c_int) as *mut js_Environment;
    (*E).gcmark = 0;
    (*E).gcnext = (*J).gcenv;
    (*J).gcenv = E;
    (*J).gccounter += 1;
    (*E).outer = outer;
    (*E).variables = vars;
    E
}

unsafe fn js_initvar(J: *mut js_State, name: *const c_char, idx: c_int) {
    jsR_defproperty(J, (*(*J).E).variables, name, JS_DONTENUM | JS_DONTCONF, stackidx(J, idx), std::ptr::null_mut(), std::ptr::null_mut(), 0);
}

unsafe fn js_hasvar(J: *mut js_State, name: *const c_char) -> c_int {
    let mut E = (*J).E;
    loop {
        let rf = crate::jsproperty::jsV_getproperty(J, (*E).variables, name);
        if !rf.is_null() {
            if !(*rf).getter.is_null() {
                js_pushobject(J, (*rf).getter);
                js_pushobject(J, (*E).variables);
                js_call(J, 0);
            } else {
                js_pushvalue(J, (*rf).value);
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
        let rf = crate::jsproperty::jsV_getproperty(J, (*E).variables, name);
        if !rf.is_null() {
            if !(*rf).setter.is_null() {
                js_pushobject(J, (*rf).setter);
                js_pushobject(J, (*E).variables);
                js_copy(J, -3);
                js_call(J, 1);
                js_pop(J, 1);
                return;
            }
            if (*rf).atts & JS_READONLY == 0 {
                (*rf).value = *stackidx(J, -1);
            } else if (*J).strict != 0 {
                crate::jserror::js_typeerror(J, cstr!("'%s' is read-only"), name);
            }
            return;
        }
        E = (*E).outer;
        if E.is_null() {
            break;
        }
    }
    if (*J).strict != 0 {
        crate::jserror::js_referenceerror(J, cstr!("assignment to undeclared variable '%s'"), name);
    }
    jsR_setproperty(J, (*J).G, name, 0);
}

unsafe fn js_delvar(J: *mut js_State, name: *const c_char) -> c_int {
    let mut E = (*J).E;
    loop {
        let rf = crate::jsproperty::jsV_getownproperty(J, (*E).variables, name);
        if !rf.is_null() {
            if (*rf).atts & JS_DONTCONF != 0 {
                if (*J).strict != 0 {
                    crate::jserror::js_typeerror(J, cstr!("'%s' is non-configurable"), name);
                }
                return 0;
            }
            crate::jsproperty::jsV_delproperty(J, (*E).variables, name);
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
    if (*J).envtop + 1 >= JS_ENVLIMIT as c_int {
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

unsafe fn jsR_calllwfunction(J: *mut js_State, mut n: c_int, F: *mut js_Function, scope: *mut js_Environment) {
    let v: js_Value;
    let mut i: c_int;

    jsR_savescope(J, scope);

    if n > (*F).numparams {
        js_pop(J, n - (*F).numparams);
        n = (*F).numparams;
    }

    i = n;
    while i < (*F).varlen {
        js_pushundefined(J);
        i += 1;
    }

    jsR_run(J, F);
    v = *stackidx(J, -1);
    (*J).bot -= 1;
    (*J).top = (*J).bot;
    js_pushvalue(J, v);

    jsR_restorescope(J);
}

unsafe fn jsR_callfunction(J: *mut js_State, n: c_int, F: *mut js_Function, scope: *mut js_Environment) {
    let v: js_Value;
    let mut i: c_int;

    let scope = jsR_newenvironment(J, crate::jsproperty::jsV_newobject(J, JS_COBJECT, std::ptr::null_mut()), scope);

    jsR_savescope(J, scope);

    if (*F).arguments != 0 {
        crate::jsvalue::js_newarguments(J);
        if (*J).strict == 0 {
            js_currentfunction(J);
            js_defproperty(J, -2, cstr!("callee"), JS_DONTENUM);
        }
        js_pushnumber(J, n as f64);
        js_defproperty(J, -2, cstr!("length"), JS_DONTENUM);
        i = 0;
        while i < n {
            js_copy(J, i + 1);
            js_setindex(J, -2, i);
            i += 1;
        }
        js_initvar(J, cstr!("arguments"), -1);
        js_pop(J, 1);
    }

    i = 0;
    while i < n && i < (*F).numparams {
        js_initvar(J, *(*F).vartab.add(i as usize), i + 1);
        i += 1;
    }
    js_pop(J, n);

    while i < (*F).varlen {
        js_pushundefined(J);
        js_initvar(J, *(*F).vartab.add(i as usize), -1);
        js_pop(J, 1);
        i += 1;
    }

    jsR_run(J, F);
    v = *stackidx(J, -1);
    (*J).bot -= 1;
    (*J).top = (*J).bot;
    js_pushvalue(J, v);

    jsR_restorescope(J);
}

unsafe fn jsR_callscript(J: *mut js_State, n: c_int, F: *mut js_Function, scope: *mut js_Environment) {
    let v: js_Value;
    let mut i: c_int;

    if !scope.is_null() {
        jsR_savescope(J, scope);
    }

    js_pop(J, n);

    i = 0;
    while i < (*F).varlen {
        if js_hasvar(J, *(*F).vartab.add(i as usize)) == 0 {
            js_pushundefined(J);
            js_initvar(J, *(*F).vartab.add(i as usize), -1);
            js_pop(J, 1);
        }
        i += 1;
    }

    jsR_run(J, F);
    v = *stackidx(J, -1);
    (*J).bot -= 1;
    (*J).top = (*J).bot;
    js_pushvalue(J, v);

    if !scope.is_null() {
        jsR_restorescope(J);
    }
}

unsafe fn jsR_callcfunction(J: *mut js_State, n: c_int, min: c_int, F: js_CFunction) {
    let save_top: c_int;
    let mut i: c_int;
    let v: js_Value;

    i = n;
    while i < min {
        js_pushundefined(J);
        i += 1;
    }

    save_top = (*J).top;
    (F.unwrap())(J);
    if (*J).top > save_top {
        v = *stackidx(J, -1);
        (*J).bot -= 1;
        (*J).top = (*J).bot;
        js_pushvalue(J, v);
    } else {
        (*J).bot -= 1;
        (*J).top = (*J).bot;
        js_pushundefined(J);
    }
}

unsafe fn jsR_pushtrace(J: *mut js_State, name: *const c_char, file: *const c_char, line: c_int) {
    if (*J).tracetop + 1 == JS_ENVLIMIT as c_int {
        crate::jserror::js_error(J, cstr!("call stack overflow"));
    }
    (*J).tracetop += 1;
    let t = (*J).tracetop as usize;
    (*J).trace[t].stack = (*J).bot;
    (*J).trace[t].name = name;
    (*J).trace[t].file = file;
    (*J).trace[t].line = line;
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_call(J: *mut js_State, n: c_int) {
    let obj: *mut js_Object;
    let savebot: c_int;

    if n < 0 {
        crate::jserror::js_rangeerror(J, cstr!("number of arguments cannot be negative"));
    }

    if js_iscallable(J, -n - 2) == 0 {
        crate::jserror::js_typeerror(J, cstr!("%s is not callable"), js_typeof(J, -n - 2));
    }

    obj = js_toobject(J, -n - 2);

    savebot = (*J).bot;
    (*J).bot = (*J).top - n - 1;

    if (*obj).type_ == JS_CFUNCTION {
        jsR_pushtrace(J, (*(*obj).u.f.function).name, (*(*obj).u.f.function).filename, (*(*obj).u.f.function).line);
        if (*(*obj).u.f.function).lightweight != 0 {
            jsR_calllwfunction(J, n, (*obj).u.f.function, (*obj).u.f.scope);
        } else {
            jsR_callfunction(J, n, (*obj).u.f.function, (*obj).u.f.scope);
        }
        (*J).tracetop -= 1;
    } else if (*obj).type_ == JS_CSCRIPT {
        jsR_pushtrace(J, (*(*obj).u.f.function).name, (*(*obj).u.f.function).filename, (*(*obj).u.f.function).line);
        jsR_callscript(J, n, (*obj).u.f.function, (*obj).u.f.scope);
        (*J).tracetop -= 1;
    } else if (*obj).type_ == JS_CCFUNCTION {
        jsR_pushtrace(J, (*obj).u.c.name, cstr!("native"), 0);
        jsR_callcfunction(J, n, (*obj).u.c.length, (*obj).u.c.function);
        (*J).tracetop -= 1;
    }

    (*J).bot = savebot;
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_construct(J: *mut js_State, n: c_int) {
    let obj: *mut js_Object;
    let prototype: *mut js_Object;
    let newobj: *mut js_Object;

    if js_iscallable(J, -n - 1) == 0 {
        crate::jserror::js_typeerror(J, cstr!("%s is not callable"), js_typeof(J, -n - 1));
    }

    obj = js_toobject(J, -n - 1);

    if (*obj).type_ == JS_CCFUNCTION && (*obj).u.c.constructor.is_some() {
        let savebot = (*J).bot;
        js_pushnull(J);
        if n > 0 {
            js_rot(J, n + 1);
        }
        (*J).bot = (*J).top - n - 1;

        jsR_pushtrace(J, (*obj).u.c.name, cstr!("native"), 0);
        jsR_callcfunction(J, n, (*obj).u.c.length, (*obj).u.c.constructor);
        (*J).tracetop -= 1;

        (*J).bot = savebot;
        return;
    }

    js_getproperty(J, -n - 1, cstr!("prototype"));
    if js_isobject(J, -1) != 0 {
        prototype = js_toobject(J, -1);
    } else {
        prototype = (*J).Object_prototype;
    }
    js_pop(J, 1);

    newobj = crate::jsproperty::jsV_newobject(J, JS_COBJECT, prototype);
    js_pushobject(J, newobj);
    if n > 0 {
        js_rot(J, n + 1);
    }

    js_pushobject(J, newobj);
    js_rot(J, n + 3);

    js_call(J, n);

    if js_isobject(J, -1) == 0 {
        js_pop(J, 1);
    } else {
        js_rot2pop1(J);
    }
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_eval(J: *mut js_State) {
    if js_isstring(J, -1) == 0 {
        return;
    }
    crate::jsstate::js_loadeval(J, cstr!("(eval)"), js_tostring(J, -1));
    js_rot2pop1(J);
    js_copy(J, 0);
    js_call(J, 0);
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_pconstruct(J: *mut js_State, n: c_int) -> c_int {
    let savetop = (*J).top - n - 2;
    let caught = protect(J, || {
        js_construct(J, n);
    });
    if caught {
        *(*J).stack.add(savetop as usize) = *(*J).stack.add(((*J).top - 1) as usize);
        (*J).top = savetop + 1;
        return 1;
    }
    js_endtry(J);
    0
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_pcall(J: *mut js_State, n: c_int) -> c_int {
    let savetop = (*J).top - n - 2;
    let caught = protect(J, || {
        js_call(J, n);
    });
    if caught {
        *(*J).stack.add(savetop as usize) = *(*J).stack.add(((*J).top - 1) as usize);
        (*J).top = savetop + 1;
        return 1;
    }
    js_endtry(J);
    0
}

/* Exceptions */
#[no_mangle]
pub unsafe extern "C-unwind" fn js_savetrypc(J: *mut js_State, pc: *mut js_Instruction) -> *mut c_void {
    if (*J).trytop == JS_TRYLIMIT as c_int {
        js_trystackoverflow(J);
    }
    let i = (*J).trytop as usize;
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

#[no_mangle]
pub unsafe extern "C-unwind" fn js_savetry(J: *mut js_State) -> *mut c_void {
    if (*J).trytop == JS_TRYLIMIT as c_int {
        js_trystackoverflow(J);
    }
    let i = (*J).trytop as usize;
    (*J).trybuf[i].E = (*J).E;
    (*J).trybuf[i].envtop = (*J).envtop;
    (*J).trybuf[i].tracetop = (*J).tracetop;
    (*J).trybuf[i].top = (*J).top;
    (*J).trybuf[i].bot = (*J).bot;
    (*J).trybuf[i].strict = (*J).strict;
    (*J).trybuf[i].pc = std::ptr::null_mut();
    (*J).trytop += 1;
    (*J).trybuf[i].buf.as_mut_ptr() as *mut c_void
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_endtry(J: *mut js_State) {
    if (*J).trytop == 0 {
        crate::jserror::js_error(J, cstr!("endtry: exception stack underflow"));
    }
    (*J).trytop -= 1;
}

#[no_mangle]
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
        std::panic::panic_any(JsThrow { target: i as c_int });
    }
    if let Some(panic) = (*J).panic {
        panic(J);
    }
    std::process::abort();
}

/// Local protected-try helper (models `if (js_try(J)) {handler}` / `js_endtry`).
pub(crate) unsafe fn protect<F: FnOnce()>(J: *mut js_State, body: F) -> bool {
    if (*J).trytop == JS_TRYLIMIT as c_int {
        js_trystackoverflow(J);
    }
    let i = (*J).trytop as usize;
    (*J).trybuf[i].E = (*J).E;
    (*J).trybuf[i].envtop = (*J).envtop;
    (*J).trybuf[i].tracetop = (*J).tracetop;
    (*J).trybuf[i].top = (*J).top;
    (*J).trybuf[i].bot = (*J).bot;
    (*J).trybuf[i].strict = (*J).strict;
    (*J).trybuf[i].pc = std::ptr::null_mut();
    (*J).trytop += 1;
    let idx = i as c_int;
    match catch_unwind(AssertUnwindSafe(body)) {
        Ok(()) => false,
        Err(payload) => {
            if let Some(thr) = payload.downcast_ref::<JsThrow>() {
                if thr.target == idx {
                    return true;
                }
            }
            resume_unwind(payload);
        }
    }
}

/* Main interpreter loop */
unsafe fn js_dumpvalue(J: *mut js_State, v: js_Value) {
    // Debug printing (js_trap). Faithful but rarely used.
    match v.type_() {
        JS_TUNDEFINED => {
            libc::printf(cstr!("undefined"));
        }
        JS_TNULL => {
            libc::printf(cstr!("null"));
        }
        JS_TBOOLEAN => {
            libc::printf(if v.u.boolean != 0 { cstr!("true") } else { cstr!("false") });
        }
        JS_TNUMBER => {
            libc::printf(cstr!("%.9g"), v.u.number);
        }
        JS_TSHRSTR => {
            libc::printf(cstr!("'%s'"), v.u.shrstr.as_ptr());
        }
        JS_TLITSTR => {
            libc::printf(cstr!("'%s'"), v.u.litstr);
        }
        JS_TMEMSTR => {
            libc::printf(cstr!("'%s'"), (*v.u.memstr).p.as_ptr());
        }
        JS_TOBJECT => {
            if v.u.object == (*J).G {
                libc::printf(cstr!("[Global]"));
                return;
            }
            let obj = v.u.object;
            match (*obj).type_ {
                JS_COBJECT => { libc::printf(cstr!("[Object %p]"), obj as *mut c_void); }
                JS_CARRAY => { libc::printf(cstr!("[Array %p]"), obj as *mut c_void); }
                JS_CFUNCTION => {
                    libc::printf(cstr!("[Function %p, %s, %s:%d]"), obj as *mut c_void,
                        (*(*obj).u.f.function).name, (*(*obj).u.f.function).filename, (*(*obj).u.f.function).line);
                }
                JS_CSCRIPT => { libc::printf(cstr!("[Script %s]"), (*(*obj).u.f.function).filename); }
                JS_CCFUNCTION => { libc::printf(cstr!("[CFunction %s]"), (*obj).u.c.name); }
                JS_CBOOLEAN => { libc::printf(cstr!("[Boolean %d]"), (*obj).u.boolean); }
                JS_CNUMBER => { libc::printf(cstr!("[Number %g]"), (*obj).u.number); }
                JS_CSTRING => { libc::printf(cstr!("[String'%s']"), (*obj).u.s.string); }
                JS_CERROR => { libc::printf(cstr!("[Error]")); }
                JS_CARGUMENTS => { libc::printf(cstr!("[Arguments %p]"), obj as *mut c_void); }
                JS_CITERATOR => { libc::printf(cstr!("[Iterator %p]"), obj as *mut c_void); }
                JS_CUSERDATA => { libc::printf(cstr!("[Userdata %s %p]"), (*obj).u.user.tag, (*obj).u.user.data); }
                _ => { libc::printf(cstr!("[Object %p]"), obj as *mut c_void); }
            }
        }
        _ => {}
    }
}

unsafe fn js_stacktrace(J: *mut js_State) {
    libc::printf(cstr!("stack trace:\n"));
    let mut n = (*J).tracetop;
    while n >= 0 {
        let name = (*J).trace[n as usize].name;
        let file = (*J).trace[n as usize].file;
        let line = (*J).trace[n as usize].line;
        if line > 0 {
            if *name.add(0) != 0 {
                libc::printf(cstr!("\tat %s (%s:%d)\n"), name, file, line);
            } else {
                libc::printf(cstr!("\tat %s:%d\n"), file, line);
            }
        } else {
            libc::printf(cstr!("\tat %s (%s)\n"), name, file);
        }
        n -= 1;
    }
}

unsafe fn js_dumpstack(J: *mut js_State) {
    libc::printf(cstr!("stack {\n"));
    let mut i = 0;
    while i < (*J).top {
        libc::putchar(if i == (*J).bot { '>' as c_int } else { ' ' as c_int });
        libc::printf(cstr!("%4d: "), i);
        js_dumpvalue(J, *(*J).stack.add(i as usize));
        libc::putchar('\n' as c_int);
        i += 1;
    }
    libc::printf(cstr!("}\n"));
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_trap(J: *mut js_State, _pc: c_int) {
    js_dumpstack(J);
    js_stacktrace(J);
}

unsafe fn jsR_isindex(J: *mut js_State, idx: c_int, k: *mut c_int) -> c_int {
    let v = stackidx(J, idx);
    if (*v).type_() == JS_TNUMBER {
        *k = (*v).u.number as c_int;
        return (*k as f64 == (*v).u.number && *k >= 0) as c_int;
    }
    0
}

#[inline]
unsafe fn read_pc_usize(pc: *mut js_Instruction, i: &mut isize) -> js_Instruction {
    let v = *pc.offset(*i);
    *i += 1;
    v
}

unsafe fn jsR_run(J: *mut js_State, F: *mut js_Function) {
    let FT = (*F).funtab;
    let VT = if !(*F).vartab.is_null() { (*F).vartab.offset(-1) } else { std::ptr::null() };
    let lightweight = (*F).lightweight;
    let pcstart = (*F).code;
    let savestrict: c_int = (*J).strict;
    (*J).strict = (*F).strict;

    // Number of try-frames active on entry; OP_TRY frames pushed by this
    // invocation have index >= entry_trytop and are "ours" to catch.
    let entry_trytop = (*J).trytop;

    let mut pc = (*F).code;
    loop {
        let res = catch_unwind(AssertUnwindSafe(|| jsR_loop(J, F, pc, pcstart, FT, VT, lightweight)));
        match res {
            Ok(_) => {
                (*J).strict = savestrict;
                return;
            }
            Err(payload) => {
                if let Some(thr) = payload.downcast_ref::<JsThrow>() {
                    // js_throw already restored state and set trytop to the
                    // handler frame index. If that frame is one of ours (an
                    // OP_TRY in this function), resume at its handler pc.
                    if thr.target >= entry_trytop {
                        pc = (*J).trybuf[thr.target as usize].pc;
                        continue;
                    }
                }
                resume_unwind(payload);
            }
        }
    }
}

/// Runs the dispatch loop starting at `startpc`. Returns normally on OP_RETURN.
/// Throws (panics) on exception; the caller's catch_unwind resumes at handlers.
unsafe fn jsR_loop(
    J: *mut js_State,
    _F: *mut js_Function,
    startpc: *mut js_Instruction,
    pcstart: *mut js_Instruction,
    FT: *mut *mut js_Function,
    VT: *const *const c_char,
    lightweight: c_int,
) {
    let mut pc = startpc;
    let mut opcode: c_int;
    let mut offset: c_int;

    let mut str: *const c_char = std::ptr::null();
    let mut obj: *mut js_Object;
    let mut xd: f64;
    let mut yd: f64;
    let mut ux: c_uint;
    let mut uy: c_uint;
    let mut ix: c_int = 0;
    let mut iy: c_int;
    let mut okay: c_int = 0;
    let mut b: c_int;
    let mut transient: c_int;

    macro_rules! READSTRING {
        () => {{
            let mut sp: *const c_char = std::ptr::null();
            libc::memcpy(
                &mut sp as *mut *const c_char as *mut c_void,
                pc as *const c_void,
                std::mem::size_of::<*const c_char>(),
            );
            pc = pc.add(std::mem::size_of::<*const c_char>() / std::mem::size_of::<js_Instruction>());
            str = sp;
        }};
    }

    loop {
        if (*J).runlimit > 0 {
            if (*J).runlimit == 1 {
                js_runlimit_err(J);
            }
            (*J).runlimit -= 1;
        }

        if (*J).gccounter > (*J).gcthresh {
            crate::jsgc::js_gc(J, 0);
        }

        (*J).trace[(*J).tracetop as usize].line = *pc as c_int;
        pc = pc.add(1);

        opcode = *pc as c_int;
        pc = pc.add(1);

        match opcode {
            OP_POP => js_pop(J, 1),
            OP_DUP => js_dup(J),
            OP_DUP2 => js_dup2(J),
            OP_ROT2 => js_rot2(J),
            OP_ROT3 => js_rot3(J),
            OP_ROT4 => js_rot4(J),

            OP_INTEGER => {
                js_pushnumber(J, (*pc as c_int - 32768) as f64);
                pc = pc.add(1);
            }
            OP_NUMBER => {
                let mut xx: f64 = 0.0;
                libc::memcpy(&mut xx as *mut f64 as *mut c_void, pc as *const c_void, std::mem::size_of::<f64>());
                pc = pc.add(std::mem::size_of::<f64>() / std::mem::size_of::<js_Instruction>());
                js_pushnumber(J, xx);
            }
            OP_STRING => {
                READSTRING!();
                js_pushliteral(J, str);
            }
            OP_CLOSURE => {
                crate::jsvalue::js_newfunction(J, *FT.add(*pc as usize), (*J).E);
                pc = pc.add(1);
            }
            OP_NEWOBJECT => crate::jsvalue::js_newobject(J),
            OP_NEWARRAY => crate::jsvalue::js_newarray(J),
            OP_NEWREGEXP => {
                READSTRING!();
                crate::jsregexp::js_newregexp(J, str, *pc as c_int);
                pc = pc.add(1);
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
            OP_CURRENT => js_currentfunction(J),

            OP_GETLOCAL => {
                if lightweight != 0 {
                    checkstack!(J, 1);
                    *(*J).stack.add((*J).top as usize) = *(*J).stack.add(((*J).bot + *pc as c_int) as usize);
                    (*J).top += 1;
                    pc = pc.add(1);
                } else {
                    str = *VT.add(*pc as usize);
                    pc = pc.add(1);
                    if js_hasvar(J, str) == 0 {
                        crate::jserror::js_referenceerror(J, cstr!("'%s' is not defined"), str);
                    }
                }
            }
            OP_SETLOCAL => {
                if lightweight != 0 {
                    *(*J).stack.add(((*J).bot + *pc as c_int) as usize) = *(*J).stack.add(((*J).top - 1) as usize);
                    pc = pc.add(1);
                } else {
                    js_setvar(J, *VT.add(*pc as usize));
                    pc = pc.add(1);
                }
            }
            OP_DELLOCAL => {
                if lightweight != 0 {
                    pc = pc.add(1);
                    js_pushboolean(J, 0);
                } else {
                    b = js_delvar(J, *VT.add(*pc as usize));
                    pc = pc.add(1);
                    js_pushboolean(J, b);
                }
            }

            OP_GETVAR => {
                READSTRING!();
                if js_hasvar(J, str) == 0 {
                    crate::jserror::js_referenceerror(J, cstr!("'%s' is not defined"), str);
                }
            }
            OP_HASVAR => {
                READSTRING!();
                if js_hasvar(J, str) == 0 {
                    js_pushundefined(J);
                }
            }
            OP_SETVAR => {
                READSTRING!();
                js_setvar(J, str);
            }
            OP_DELVAR => {
                READSTRING!();
                b = js_delvar(J, str);
                js_pushboolean(J, b);
            }

            OP_IN => {
                str = js_tostring(J, -2);
                if js_isobject(J, -1) == 0 {
                    crate::jserror::js_typeerror(J, cstr!("operand to 'in' is not an object"));
                }
                b = js_hasproperty(J, -1, str);
                js_pop(J, 2 + b);
                js_pushboolean(J, b);
            }

            OP_SKIPARRAY => {
                crate::jsarray::js_setlength(J, -1, crate::jsarray::js_getlength(J, -1) + 1);
            }
            OP_INITARRAY => {
                js_setindex(J, -2, crate::jsarray::js_getlength(J, -2));
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
                jsR_defproperty(J, obj, str, 0, std::ptr::null_mut(), jsR_tofunction(J, -1), std::ptr::null_mut(), 0);
                js_pop(J, 2);
            }
            OP_INITSETTER => {
                obj = js_toobject(J, -3);
                str = js_tostring(J, -2);
                jsR_defproperty(J, obj, str, 0, std::ptr::null_mut(), std::ptr::null_mut(), jsR_tofunction(J, -1), 0);
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
                READSTRING!();
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
                READSTRING!();
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
                READSTRING!();
                obj = js_toobject(J, -1);
                b = jsR_delproperty(J, obj, str);
                js_pop(J, 1);
                js_pushboolean(J, b);
            }

            OP_ITERATOR => {
                if js_iscoercible(J, -1) != 0 {
                    obj = crate::jsproperty::jsV_newiterator(J, js_toobject(J, -1), 0);
                    js_pop(J, 1);
                    js_pushobject(J, obj);
                }
            }
            OP_NEXTITER => {
                if js_isobject(J, -1) != 0 {
                    obj = js_toobject(J, -1);
                    str = crate::jsproperty::jsV_nextiterator(J, obj);
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

            OP_EVAL => js_eval(J),
            OP_CALL => {
                js_call(J, *pc as c_int);
                pc = pc.add(1);
            }
            OP_NEW => {
                js_construct(J, *pc as c_int);
                pc = pc.add(1);
            }

            OP_TYPEOF => {
                str = js_typeof(J, -1);
                js_pop(J, 1);
                js_pushliteral(J, str);
            }
            OP_POS => {
                xd = js_tonumber(J, -1);
                js_pop(J, 1);
                js_pushnumber(J, xd);
            }
            OP_NEG => {
                xd = js_tonumber(J, -1);
                js_pop(J, 1);
                js_pushnumber(J, -xd);
            }
            OP_BITNOT => {
                ix = js_toint32(J, -1);
                js_pop(J, 1);
                js_pushnumber(J, !ix as f64);
            }
            OP_LOGNOT => {
                b = js_toboolean(J, -1);
                js_pop(J, 1);
                js_pushboolean(J, (b == 0) as c_int);
            }
            OP_INC => {
                xd = js_tonumber(J, -1);
                js_pop(J, 1);
                js_pushnumber(J, xd + 1.0);
            }
            OP_DEC => {
                xd = js_tonumber(J, -1);
                js_pop(J, 1);
                js_pushnumber(J, xd - 1.0);
            }
            OP_POSTINC => {
                xd = js_tonumber(J, -1);
                js_pop(J, 1);
                js_pushnumber(J, xd + 1.0);
                js_pushnumber(J, xd);
            }
            OP_POSTDEC => {
                xd = js_tonumber(J, -1);
                js_pop(J, 1);
                js_pushnumber(J, xd - 1.0);
                js_pushnumber(J, xd);
            }

            OP_MUL => {
                xd = js_tonumber(J, -2);
                yd = js_tonumber(J, -1);
                js_pop(J, 2);
                js_pushnumber(J, xd * yd);
            }
            OP_DIV => {
                xd = js_tonumber(J, -2);
                yd = js_tonumber(J, -1);
                js_pop(J, 2);
                js_pushnumber(J, xd / yd);
            }
            OP_MOD => {
                xd = js_tonumber(J, -2);
                yd = js_tonumber(J, -1);
                js_pop(J, 2);
                js_pushnumber(J, xd % yd);
            }
            OP_ADD => crate::jsvalue::js_concat(J),
            OP_SUB => {
                xd = js_tonumber(J, -2);
                yd = js_tonumber(J, -1);
                js_pop(J, 2);
                js_pushnumber(J, xd - yd);
            }

            OP_SHL => {
                ix = js_toint32(J, -2);
                uy = js_touint32(J, -1);
                js_pop(J, 2);
                js_pushnumber(J, (ix.wrapping_shl(uy & 0x1F)) as f64);
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

            OP_LT => {
                b = crate::jsvalue::js_compare(J, &mut okay);
                js_pop(J, 2);
                js_pushboolean(J, (okay != 0 && b < 0) as c_int);
            }
            OP_GT => {
                b = crate::jsvalue::js_compare(J, &mut okay);
                js_pop(J, 2);
                js_pushboolean(J, (okay != 0 && b > 0) as c_int);
            }
            OP_LE => {
                b = crate::jsvalue::js_compare(J, &mut okay);
                js_pop(J, 2);
                js_pushboolean(J, (okay != 0 && b <= 0) as c_int);
            }
            OP_GE => {
                b = crate::jsvalue::js_compare(J, &mut okay);
                js_pop(J, 2);
                js_pushboolean(J, (okay != 0 && b >= 0) as c_int);
            }

            OP_INSTANCEOF => {
                b = crate::jsvalue::js_instanceof(J);
                js_pop(J, 2);
                js_pushboolean(J, b);
            }

            OP_EQ => {
                b = crate::jsvalue::js_equal(J);
                js_pop(J, 2);
                js_pushboolean(J, b);
            }
            OP_NE => {
                b = crate::jsvalue::js_equal(J);
                js_pop(J, 2);
                js_pushboolean(J, (b == 0) as c_int);
            }
            OP_STRICTEQ => {
                b = crate::jsvalue::js_strictequal(J);
                js_pop(J, 2);
                js_pushboolean(J, b);
            }
            OP_STRICTNE => {
                b = crate::jsvalue::js_strictequal(J);
                js_pop(J, 2);
                js_pushboolean(J, (b == 0) as c_int);
            }
            OP_JCASE => {
                offset = *pc as c_int;
                pc = pc.add(1);
                b = crate::jsvalue::js_strictequal(J);
                if b != 0 {
                    js_pop(J, 2);
                    pc = pcstart.add(offset as usize);
                } else {
                    js_pop(J, 1);
                }
            }

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

            OP_THROW => js_throw(J),

            OP_TRY => {
                offset = *pc as c_int;
                pc = pc.add(1);
                // js_trypc(J, pc): push a handler frame recording pc = handler
                // location (the address right after the OP_TRY arg). On throw
                // targeting this frame, jsR_run resumes at trybuf[].pc which we
                // set to the code right here; but the C sets pc to the handler
                // path (fall-through) on catch, and to pcstart+offset on the
                // normal (setjmp==0) path. Match that exactly:
                js_savetrypc(J, pc);
                pc = pcstart.add(offset as usize);
            }
            OP_ENDTRY => js_endtry(J),

            OP_CATCH => {
                READSTRING!();
                obj = crate::jsproperty::jsV_newobject(J, JS_COBJECT, std::ptr::null_mut());
                js_pushobject(J, obj);
                js_rot2(J);
                js_setproperty(J, -2, str);
                (*J).E = jsR_newenvironment(J, obj, (*J).E);
                js_pop(J, 1);
            }
            OP_ENDCATCH => {
                (*J).E = (*(*J).E).outer;
            }

            OP_WITH => {
                obj = js_toobject(J, -1);
                (*J).E = jsR_newenvironment(J, obj, (*J).E);
                js_pop(J, 1);
            }
            OP_ENDWITH => {
                (*J).E = (*(*J).E).outer;
            }

            OP_DEBUGGER => {
                js_trap(J, (pc.offset_from(pcstart) as c_int) - 1);
            }
            OP_JUMP => {
                pc = pcstart.add(*pc as usize);
            }
            OP_JTRUE => {
                offset = *pc as c_int;
                pc = pc.add(1);
                b = js_toboolean(J, -1);
                js_pop(J, 1);
                if b != 0 {
                    pc = pcstart.add(offset as usize);
                }
            }
            OP_JFALSE => {
                offset = *pc as c_int;
                pc = pc.add(1);
                b = js_toboolean(J, -1);
                js_pop(J, 1);
                if b == 0 {
                    pc = pcstart.add(offset as usize);
                }
            }
            OP_RETURN => {
                return;
            }
            _ => {}
        }
    }
}
