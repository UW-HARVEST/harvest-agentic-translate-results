//! Translation of src/jsrun.c
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

use crate::jsi::*;
use core::ptr::{null, null_mut};
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};

use crate::except::{is_js_throw, raise};
use crate::jsarray::{js_getlength, js_setlength};
use crate::jsgc::js_gc;
use crate::jsintern::js_intern;
use crate::jsproperty::{
    jsV_delproperty, jsV_getownproperty, jsV_getproperty, jsV_getpropertyx, jsV_newiterator,
    jsV_newobject, jsV_nextiterator, jsV_resizearray, jsV_setproperty,
};
use crate::jsstate::js_loadeval;
use crate::jsstring::js_runeat;
use crate::jsvalue::{
    js_compare, js_concat, js_equal, js_instanceof, js_itoa, js_newarguments, js_newarray,
    js_newfunction, js_newobject, js_strictequal,
    jsV_toboolean, jsV_tointeger, jsV_tonumber, jsV_toobject, jsV_toprimitive, jsV_tostring,
    jsV_numbertoint16, jsV_numbertoint32, jsV_numbertointeger, jsV_numbertouint16,
    jsV_numbertouint32,
};
use crate::jsregexp::js_newregexp;
use crate::utf::jsU_runetochar;

/* ------------------------------------------------------------------ */
/* Errors that need the value stack                                    */
/* ------------------------------------------------------------------ */

unsafe fn js_trystackoverflow(J: *mut js_State) -> ! {
    unsafe {
        let t = (*J).top as usize;
        (*(*J).stack.add(t)).set_ty(JS_TLITSTR);
        (*(*J).stack.add(t)).litstr = c"exception stack overflow".as_ptr();
        (*J).top += 1;
        js_throw(J)
    }
}

unsafe fn js_stackoverflow(J: *mut js_State) -> ! {
    unsafe {
        let t = (*J).top as usize;
        (*(*J).stack.add(t)).set_ty(JS_TLITSTR);
        (*(*J).stack.add(t)).litstr = c"stack overflow".as_ptr();
        (*J).top += 1;
        js_throw(J)
    }
}

unsafe fn js_outofmemory(J: *mut js_State) -> ! {
    unsafe {
        let t = (*J).top as usize;
        (*(*J).stack.add(t)).set_ty(JS_TLITSTR);
        (*(*J).stack.add(t)).litstr = c"out of memory".as_ptr();
        (*J).top += 1;
        js_throw(J)
    }
}

unsafe fn js_runlimit(J: *mut js_State) -> ! {
    unsafe {
        let t = (*J).top as usize;
        (*(*J).stack.add(t)).set_ty(JS_TLITSTR);
        (*(*J).stack.add(t)).litstr = c"script ran too long".as_ptr();
        (*J).top += 1;
        js_throw(J)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_setlimit(J: *mut js_State, runlimit: c_int, memlimit: c_int) {
    unsafe {
        (*J).runlimit = runlimit;
        (*J).memlimit = memlimit;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_malloc(J: *mut js_State, size: c_int) -> *mut c_void {
    unsafe {
        let ptr: *mut c_void;
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
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_realloc(
    J: *mut js_State,
    ptr: *mut c_void,
    size: c_int,
) -> *mut c_void {
    unsafe {
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
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_strdup(J: *mut js_State, s: *const c_char) -> *mut c_char {
    unsafe {
        let n = (strlen(s) + 1) as c_int;
        let p = js_malloc(J, n) as *mut c_char;
        memcpy(p as *mut c_void, s as *const c_void, n as size_t);
        p
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_free(J: *mut js_State, ptr: *mut c_void) {
    unsafe {
        ((*J).alloc.unwrap())((*J).actx, ptr, 0);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsV_newmemstring(
    J: *mut js_State,
    s: *const c_char,
    n: c_int,
) -> *mut js_String {
    unsafe {
        let v = js_malloc(J, JS_STRING_POFF + n + 1) as *mut js_String;
        let p = (&raw mut (*v).p) as *mut c_char;
        memcpy(p as *mut c_void, s as *const c_void, n as size_t);
        *p.offset(n as isize) = 0;
        (*v).gcmark = 0;
        (*v).gcnext = (*J).gcstr;
        (*J).gcstr = v;
        (*J).gccounter += 1;
        v
    }
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
    unsafe {
        CHECKSTACK!(J, 1);
        *(*J).stack.add((*J).top as usize) = v;
        (*J).top += 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_pushundefined(J: *mut js_State) {
    unsafe {
        CHECKSTACK!(J, 1);
        (*(*J).stack.add((*J).top as usize)).set_ty(JS_TUNDEFINED);
        (*J).top += 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_pushnull(J: *mut js_State) {
    unsafe {
        CHECKSTACK!(J, 1);
        (*(*J).stack.add((*J).top as usize)).set_ty(JS_TNULL);
        (*J).top += 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_pushboolean(J: *mut js_State, v: c_int) {
    unsafe {
        CHECKSTACK!(J, 1);
        let s = (*J).stack.add((*J).top as usize);
        (*s).set_ty(JS_TBOOLEAN);
        (*s).boolean = (v != 0) as c_int;
        (*J).top += 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_pushnumber(J: *mut js_State, v: f64) {
    unsafe {
        CHECKSTACK!(J, 1);
        let s = (*J).stack.add((*J).top as usize);
        (*s).set_ty(JS_TNUMBER);
        (*s).number = v;
        (*J).top += 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_pushstring(J: *mut js_State, v: *const c_char) {
    unsafe {
        let mut v = v;
        let mut n = strlen(v);
        if n > JS_STRLIMIT as size_t {
            js_rangeerror!(J, c"invalid string length".as_ptr());
        }
        CHECKSTACK!(J, 1);
        let slot = (*J).stack.add((*J).top as usize);
        if n <= JS_VALUE_TYPEOFF as size_t {
            let mut s = (&raw mut (*slot).shrstr) as *mut c_char;
            while n != 0 {
                n -= 1;
                *s = *v;
                s = s.add(1);
                v = v.add(1);
            }
            *s = 0;
            (*slot).set_ty(JS_TSHRSTR);
        } else {
            (*slot).set_ty(JS_TMEMSTR);
            (*slot).memstr = jsV_newmemstring(J, v, n as c_int);
        }
        (*J).top += 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_pushlstring(J: *mut js_State, v: *const c_char, n: c_int) {
    unsafe {
        let mut v = v;
        let mut n = n;
        if n > JS_STRLIMIT {
            js_rangeerror!(J, c"invalid string length".as_ptr());
        }
        CHECKSTACK!(J, 1);
        let slot = (*J).stack.add((*J).top as usize);
        if n <= JS_VALUE_TYPEOFF {
            let mut s = (&raw mut (*slot).shrstr) as *mut c_char;
            while n != 0 {
                n -= 1;
                *s = *v;
                s = s.add(1);
                v = v.add(1);
            }
            *s = 0;
            (*slot).set_ty(JS_TSHRSTR);
        } else {
            (*slot).set_ty(JS_TMEMSTR);
            (*slot).memstr = jsV_newmemstring(J, v, n);
        }
        (*J).top += 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_pushliteral(J: *mut js_State, v: *const c_char) {
    unsafe {
        CHECKSTACK!(J, 1);
        let s = (*J).stack.add((*J).top as usize);
        (*s).set_ty(JS_TLITSTR);
        (*s).litstr = v;
        (*J).top += 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_pushobject(J: *mut js_State, v: *mut js_Object) {
    unsafe {
        CHECKSTACK!(J, 1);
        let s = (*J).stack.add((*J).top as usize);
        (*s).set_ty(JS_TOBJECT);
        (*s).object = v;
        (*J).top += 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_pushglobal(J: *mut js_State) {
    unsafe {
        js_pushobject(J, (*J).G);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_currentfunction(J: *mut js_State) {
    unsafe {
        CHECKSTACK!(J, 1);
        if (*J).bot > 0 {
            *(*J).stack.add((*J).top as usize) = *(*J).stack.add(((*J).bot - 1) as usize);
        } else {
            (*(*J).stack.add((*J).top as usize)).set_ty(JS_TUNDEFINED);
        }
        (*J).top += 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_currentfunctiondata(J: *mut js_State) -> *mut c_void {
    unsafe {
        if (*J).bot > 0 {
            return (*(*(*J).stack.add(((*J).bot - 1) as usize)).object).u.c.data;
        }
        null_mut()
    }
}

/* Read values from stack */

static mut STACKIDX_UNDEFINED: js_Value = js_Value::undef();

pub(crate) unsafe fn stackidx(J: *mut js_State, idx: c_int) -> *mut js_Value {
    unsafe {
        let idx = if idx < 0 { (*J).top + idx } else { (*J).bot + idx };
        if idx < 0 || idx >= (*J).top {
            return &raw mut STACKIDX_UNDEFINED;
        }
        (*J).stack.offset(idx as isize)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_tovalue(J: *mut js_State, idx: c_int) -> *mut js_Value {
    unsafe { stackidx(J, idx) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_isdefined(J: *mut js_State, idx: c_int) -> c_int {
    unsafe { ((*stackidx(J, idx)).ty() != JS_TUNDEFINED) as c_int }
}
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_isundefined(J: *mut js_State, idx: c_int) -> c_int {
    unsafe { ((*stackidx(J, idx)).ty() == JS_TUNDEFINED) as c_int }
}
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_isnull(J: *mut js_State, idx: c_int) -> c_int {
    unsafe { ((*stackidx(J, idx)).ty() == JS_TNULL) as c_int }
}
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_isboolean(J: *mut js_State, idx: c_int) -> c_int {
    unsafe { ((*stackidx(J, idx)).ty() == JS_TBOOLEAN) as c_int }
}
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_isnumber(J: *mut js_State, idx: c_int) -> c_int {
    unsafe { ((*stackidx(J, idx)).ty() == JS_TNUMBER) as c_int }
}
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_isstring(J: *mut js_State, idx: c_int) -> c_int {
    unsafe {
        let t = (*stackidx(J, idx)).ty();
        (t == JS_TSHRSTR || t == JS_TLITSTR || t == JS_TMEMSTR) as c_int
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_isprimitive(J: *mut js_State, idx: c_int) -> c_int {
    unsafe { ((*stackidx(J, idx)).ty() != JS_TOBJECT) as c_int }
}
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_isobject(J: *mut js_State, idx: c_int) -> c_int {
    unsafe { ((*stackidx(J, idx)).ty() == JS_TOBJECT) as c_int }
}
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_iscoercible(J: *mut js_State, idx: c_int) -> c_int {
    unsafe {
        let v = stackidx(J, idx);
        ((*v).ty() != JS_TUNDEFINED && (*v).ty() != JS_TNULL) as c_int
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_iscallable(J: *mut js_State, idx: c_int) -> c_int {
    unsafe {
        let v = stackidx(J, idx);
        if (*v).ty() == JS_TOBJECT {
            let t = (*(*v).object).ty;
            return (t == JS_CFUNCTION || t == JS_CSCRIPT || t == JS_CCFUNCTION) as c_int;
        }
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_isarray(J: *mut js_State, idx: c_int) -> c_int {
    unsafe {
        let v = stackidx(J, idx);
        ((*v).ty() == JS_TOBJECT && (*(*v).object).ty == JS_CARRAY) as c_int
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_isregexp(J: *mut js_State, idx: c_int) -> c_int {
    unsafe {
        let v = stackidx(J, idx);
        ((*v).ty() == JS_TOBJECT && (*(*v).object).ty == JS_CREGEXP) as c_int
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_isuserdata(
    J: *mut js_State,
    idx: c_int,
    tag: *const c_char,
) -> c_int {
    unsafe {
        let v = stackidx(J, idx);
        if (*v).ty() == JS_TOBJECT && (*(*v).object).ty == JS_CUSERDATA {
            return (strcmp(tag, (*(*v).object).u.user.tag) == 0) as c_int;
        }
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_iserror(J: *mut js_State, idx: c_int) -> c_int {
    unsafe {
        let v = stackidx(J, idx);
        ((*v).ty() == JS_TOBJECT && (*(*v).object).ty == JS_CERROR) as c_int
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_typeof(J: *mut js_State, idx: c_int) -> *const c_char {
    unsafe {
        let v = stackidx(J, idx);
        match (*v).ty() {
            JS_TUNDEFINED => c"undefined".as_ptr(),
            JS_TNULL => c"object".as_ptr(),
            JS_TBOOLEAN => c"boolean".as_ptr(),
            JS_TNUMBER => c"number".as_ptr(),
            JS_TLITSTR => c"string".as_ptr(),
            JS_TMEMSTR => c"string".as_ptr(),
            JS_TOBJECT => {
                if (*(*v).object).ty == JS_CFUNCTION || (*(*v).object).ty == JS_CCFUNCTION {
                    c"function".as_ptr()
                } else {
                    c"object".as_ptr()
                }
            }
            /* default and JS_TSHRSTR */
            _ => c"string".as_ptr(),
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_type(J: *mut js_State, idx: c_int) -> c_int {
    unsafe {
        let v = stackidx(J, idx);
        match (*v).ty() {
            JS_TUNDEFINED => JS_ISUNDEFINED,
            JS_TNULL => JS_ISNULL,
            JS_TBOOLEAN => JS_ISBOOLEAN,
            JS_TNUMBER => JS_ISNUMBER,
            JS_TLITSTR => JS_ISSTRING,
            JS_TMEMSTR => JS_ISSTRING,
            JS_TOBJECT => {
                if (*(*v).object).ty == JS_CFUNCTION || (*(*v).object).ty == JS_CCFUNCTION {
                    JS_ISFUNCTION
                } else {
                    JS_ISOBJECT
                }
            }
            /* default and JS_TSHRSTR */
            _ => JS_ISSTRING,
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_toboolean(J: *mut js_State, idx: c_int) -> c_int {
    unsafe { jsV_toboolean(J, stackidx(J, idx)) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_tonumber(J: *mut js_State, idx: c_int) -> f64 {
    unsafe { jsV_tonumber(J, stackidx(J, idx)) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_tointeger(J: *mut js_State, idx: c_int) -> c_int {
    unsafe { jsV_numbertointeger(jsV_tonumber(J, stackidx(J, idx))) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_toint32(J: *mut js_State, idx: c_int) -> c_int {
    unsafe { jsV_numbertoint32(jsV_tonumber(J, stackidx(J, idx))) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_touint32(J: *mut js_State, idx: c_int) -> c_uint {
    unsafe { jsV_numbertouint32(jsV_tonumber(J, stackidx(J, idx))) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_toint16(J: *mut js_State, idx: c_int) -> c_short {
    unsafe { jsV_numbertoint16(jsV_tonumber(J, stackidx(J, idx))) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_touint16(J: *mut js_State, idx: c_int) -> c_ushort {
    unsafe { jsV_numbertouint16(jsV_tonumber(J, stackidx(J, idx))) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_tostring(J: *mut js_State, idx: c_int) -> *const c_char {
    unsafe { jsV_tostring(J, stackidx(J, idx)) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_toobject(J: *mut js_State, idx: c_int) -> *mut js_Object {
    unsafe { jsV_toobject(J, stackidx(J, idx)) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_toprimitive(J: *mut js_State, idx: c_int, hint: c_int) {
    unsafe { jsV_toprimitive(J, stackidx(J, idx), hint) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_toregexp(J: *mut js_State, idx: c_int) -> *mut js_Regexp {
    unsafe {
        let v = stackidx(J, idx);
        if (*v).ty() == JS_TOBJECT && (*(*v).object).ty == JS_CREGEXP {
            return &raw mut (*(*v).object).u.r;
        }
        js_typeerror!(J, c"not a regexp".as_ptr())
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_touserdata(
    J: *mut js_State,
    idx: c_int,
    tag: *const c_char,
) -> *mut c_void {
    unsafe {
        let v = stackidx(J, idx);
        if (*v).ty() == JS_TOBJECT && (*(*v).object).ty == JS_CUSERDATA {
            if strcmp(tag, (*(*v).object).u.user.tag) == 0 {
                return (*(*v).object).u.user.data;
            }
        }
        js_typeerror!(J, c"not a %s".as_ptr(), tag)
    }
}

unsafe fn jsR_tofunction(J: *mut js_State, idx: c_int) -> *mut js_Object {
    unsafe {
        let v = stackidx(J, idx);
        if (*v).ty() == JS_TUNDEFINED || (*v).ty() == JS_TNULL {
            return null_mut();
        }
        if (*v).ty() == JS_TOBJECT {
            if (*(*v).object).ty == JS_CFUNCTION || (*(*v).object).ty == JS_CCFUNCTION {
                return (*v).object;
            }
        }
        js_typeerror!(J, c"not a function".as_ptr())
    }
}

/* Stack manipulation */

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_gettop(J: *mut js_State) -> c_int {
    unsafe { (*J).top - (*J).bot }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_pop(J: *mut js_State, n: c_int) {
    unsafe {
        (*J).top -= n;
        if (*J).top < (*J).bot {
            (*J).top = (*J).bot;
            js_error!(J, c"stack underflow!".as_ptr());
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_remove(J: *mut js_State, idx: c_int) {
    unsafe {
        let mut idx = if idx < 0 { (*J).top + idx } else { (*J).bot + idx };
        if idx < (*J).bot || idx >= (*J).top {
            js_error!(J, c"stack error!".as_ptr());
        }
        while idx < (*J).top - 1 {
            *(*J).stack.offset(idx as isize) = *(*J).stack.offset((idx + 1) as isize);
            idx += 1;
        }
        (*J).top -= 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_insert(J: *mut js_State, idx: c_int) {
    unsafe {
        js_error!(J, c"not implemented yet".as_ptr());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_replace(J: *mut js_State, idx: c_int) {
    unsafe {
        let idx = if idx < 0 { (*J).top + idx } else { (*J).bot + idx };
        if idx < (*J).bot || idx >= (*J).top {
            js_error!(J, c"stack error!".as_ptr());
        }
        (*J).top -= 1;
        *(*J).stack.offset(idx as isize) = *(*J).stack.offset((*J).top as isize);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_copy(J: *mut js_State, idx: c_int) {
    unsafe {
        CHECKSTACK!(J, 1);
        *(*J).stack.add((*J).top as usize) = *stackidx(J, idx);
        (*J).top += 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_dup(J: *mut js_State) {
    unsafe {
        CHECKSTACK!(J, 1);
        *(*J).stack.add((*J).top as usize) = *(*J).stack.add(((*J).top - 1) as usize);
        (*J).top += 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_dup2(J: *mut js_State) {
    unsafe {
        CHECKSTACK!(J, 2);
        let top = (*J).top;
        *(*J).stack.offset(top as isize) = *(*J).stack.offset((top - 2) as isize);
        *(*J).stack.offset((top + 1) as isize) = *(*J).stack.offset((top - 1) as isize);
        (*J).top += 2;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_rot2(J: *mut js_State) {
    unsafe {
        /* A B -> B A */
        let top = (*J).top;
        let tmp = *(*J).stack.offset((top - 1) as isize);
        *(*J).stack.offset((top - 1) as isize) = *(*J).stack.offset((top - 2) as isize);
        *(*J).stack.offset((top - 2) as isize) = tmp;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_rot3(J: *mut js_State) {
    unsafe {
        /* A B C -> C A B */
        let top = (*J).top;
        let tmp = *(*J).stack.offset((top - 1) as isize);
        *(*J).stack.offset((top - 1) as isize) = *(*J).stack.offset((top - 2) as isize);
        *(*J).stack.offset((top - 2) as isize) = *(*J).stack.offset((top - 3) as isize);
        *(*J).stack.offset((top - 3) as isize) = tmp;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_rot4(J: *mut js_State) {
    unsafe {
        /* A B C D -> D A B C */
        let top = (*J).top;
        let tmp = *(*J).stack.offset((top - 1) as isize);
        *(*J).stack.offset((top - 1) as isize) = *(*J).stack.offset((top - 2) as isize);
        *(*J).stack.offset((top - 2) as isize) = *(*J).stack.offset((top - 3) as isize);
        *(*J).stack.offset((top - 3) as isize) = *(*J).stack.offset((top - 4) as isize);
        *(*J).stack.offset((top - 4) as isize) = tmp;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_rot2pop1(J: *mut js_State) {
    unsafe {
        /* A B -> B */
        let top = (*J).top;
        *(*J).stack.offset((top - 2) as isize) = *(*J).stack.offset((top - 1) as isize);
        (*J).top -= 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_rot3pop2(J: *mut js_State) {
    unsafe {
        /* A B C -> C */
        let top = (*J).top;
        *(*J).stack.offset((top - 3) as isize) = *(*J).stack.offset((top - 1) as isize);
        (*J).top -= 2;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_rot(J: *mut js_State, n: c_int) {
    unsafe {
        let top = (*J).top;
        let tmp = *(*J).stack.offset((top - 1) as isize);
        let mut i = 1;
        while i < n {
            *(*J).stack.offset((top - i) as isize) = *(*J).stack.offset((top - i - 1) as isize);
            i += 1;
        }
        *(*J).stack.offset((top - i) as isize) = tmp;
    }
}

/* Property access that takes care of attributes and getters/setters */

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_isarrayindex(
    J: *mut js_State,
    p: *const c_char,
    idx: *mut c_int,
) -> c_int {
    unsafe {
        let mut p = p;
        let mut n: c_int = 0;

        /* check for empty string */
        if *p.add(0) == 0 {
            return 0;
        }

        /* check for '0' and integers with leading zero */
        if *p.add(0) == '0' as c_char {
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
}

unsafe fn js_pushrune(J: *mut js_State, rune: Rune) {
    unsafe {
        let mut buf: [c_char; (UTFmax + 1) as usize] = [0; (UTFmax + 1) as usize];
        if rune >= 0 {
            let n = jsU_runetochar(buf.as_mut_ptr(), &rune);
            buf[n as usize] = 0;
            js_pushstring(J, buf.as_ptr());
        } else {
            js_pushundefined(J);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsR_unflattenarray(J: *mut js_State, obj: *mut js_Object) {
    unsafe {
        if (*obj).ty == JS_CARRAY && (*obj).u.a.simple != 0 {
            let mut name: [c_char; 32] = [0; 32];
            if crate::except::js_try_run(J, || {
                let mut i: c_int = 0;
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
            }) {
                (*obj).properties = null_mut();
                js_throw(J);
            }
        }
    }
}

unsafe fn jsR_hasproperty(J: *mut js_State, obj: *mut js_Object, name: *const c_char) -> c_int {
    unsafe {
        let mut k: c_int = 0;

        if (*obj).ty == JS_CARRAY {
            if strcmp(name, c"length".as_ptr()) == 0 {
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
        } else if (*obj).ty == JS_CSTRING {
            if strcmp(name, c"length".as_ptr()) == 0 {
                js_pushnumber(J, (*obj).u.s.length as f64);
                return 1;
            }
            if js_isarrayindex(J, name, &mut k) != 0 {
                if k >= 0 && k < (*obj).u.s.length {
                    js_pushrune(J, js_runeat(J, (*obj).u.s.string, k));
                    return 1;
                }
            }
        } else if (*obj).ty == JS_CREGEXP {
            if strcmp(name, c"source".as_ptr()) == 0 {
                js_pushstring(J, (*obj).u.r.source);
                return 1;
            }
            if strcmp(name, c"global".as_ptr()) == 0 {
                js_pushboolean(J, (*obj).u.r.flags as c_int & JS_REGEXP_G);
                return 1;
            }
            if strcmp(name, c"ignoreCase".as_ptr()) == 0 {
                js_pushboolean(J, (*obj).u.r.flags as c_int & JS_REGEXP_I);
                return 1;
            }
            if strcmp(name, c"multiline".as_ptr()) == 0 {
                js_pushboolean(J, (*obj).u.r.flags as c_int & JS_REGEXP_M);
                return 1;
            }
            if strcmp(name, c"lastIndex".as_ptr()) == 0 {
                js_pushnumber(J, (*obj).u.r.last as f64);
                return 1;
            }
        } else if (*obj).ty == JS_CUSERDATA {
            if (*obj).u.user.has.is_some()
                && ((*obj).u.user.has.unwrap())(J, (*obj).u.user.data, name) != 0
            {
                return 1;
            }
        }

        let r = jsV_getproperty(J, obj, name);
        if !r.is_null() {
            if !(*r).getter.is_null() {
                js_pushobject(J, (*r).getter);
                js_pushobject(J, obj);
                js_call(J, 0);
            } else {
                js_pushvalue(J, (*r).value);
            }
            return 1;
        }

        0
    }
}

unsafe fn jsR_getproperty(J: *mut js_State, obj: *mut js_Object, name: *const c_char) {
    unsafe {
        if jsR_hasproperty(J, obj, name) == 0 {
            js_pushundefined(J);
        }
    }
}

unsafe fn jsR_hasindex(J: *mut js_State, obj: *mut js_Object, k: c_int) -> c_int {
    unsafe {
        let mut buf: [c_char; 32] = [0; 32];
        if (*obj).ty == JS_CARRAY && (*obj).u.a.simple != 0 {
            if k >= 0 && k < (*obj).u.a.flat_length {
                js_pushvalue(J, *(*obj).u.a.array.offset(k as isize));
                return 1;
            }
            return 0;
        }
        jsR_hasproperty(J, obj, js_itoa(buf.as_mut_ptr(), k))
    }
}

unsafe fn jsR_getindex(J: *mut js_State, obj: *mut js_Object, k: c_int) {
    unsafe {
        if jsR_hasindex(J, obj, k) == 0 {
            js_pushundefined(J);
        }
    }
}

unsafe fn jsR_setarrayindex(J: *mut js_State, obj: *mut js_Object, k: c_int, value: *mut js_Value) {
    unsafe {
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
}

unsafe fn jsR_setproperty(
    J: *mut js_State,
    obj: *mut js_Object,
    name: *const c_char,
    transient: c_int,
) {
    unsafe {
        let value = stackidx(J, -1);
        let mut r: *mut js_Property;
        let mut k: c_int = 0;
        let mut own: c_int = 0;

        'readonly: {
            if (*obj).ty == JS_CARRAY {
                if strcmp(name, c"length".as_ptr()) == 0 {
                    let rawlen = jsV_tonumber(J, value);
                    let newlen = jsV_numbertointeger(rawlen);
                    if newlen as f64 != rawlen || newlen < 0 {
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
            } else if (*obj).ty == JS_CSTRING {
                if strcmp(name, c"length".as_ptr()) == 0 {
                    break 'readonly;
                }
                if js_isarrayindex(J, name, &mut k) != 0 {
                    if k >= 0 && k < (*obj).u.s.length {
                        break 'readonly;
                    }
                }
            } else if (*obj).ty == JS_CREGEXP {
                if strcmp(name, c"source".as_ptr()) == 0 {
                    break 'readonly;
                }
                if strcmp(name, c"global".as_ptr()) == 0 {
                    break 'readonly;
                }
                if strcmp(name, c"ignoreCase".as_ptr()) == 0 {
                    break 'readonly;
                }
                if strcmp(name, c"multiline".as_ptr()) == 0 {
                    break 'readonly;
                }
                if strcmp(name, c"lastIndex".as_ptr()) == 0 {
                    (*obj).u.r.last = d2i(jsV_tointeger(J, value)) as c_ushort;
                    return;
                }
            } else if (*obj).ty == JS_CUSERDATA {
                if (*obj).u.user.put.is_some()
                    && ((*obj).u.user.put.unwrap())(J, (*obj).u.user.data, name) != 0
                {
                    return;
                }
            }

            /* First try to find a setter in prototype chain */
            r = jsV_getpropertyx(J, obj, name, &mut own);
            if !r.is_null() {
                if !(*r).setter.is_null() {
                    js_pushobject(J, (*r).setter);
                    js_pushobject(J, obj);
                    js_pushvalue(J, *value);
                    js_call(J, 1);
                    js_pop(J, 1);
                    return;
                } else {
                    if (*J).strict != 0 {
                        if !(*r).getter.is_null() {
                            js_typeerror!(
                                J,
                                c"setting property '%s' that only has a getter".as_ptr(),
                                name
                            );
                        }
                    }
                    if (*r).atts & JS_READONLY != 0 {
                        break 'readonly;
                    }
                }
            }

            /* Property not found on this object, so create one */
            if r.is_null() || own == 0 {
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
                r = jsV_setproperty(J, obj, name);
            }

            if !r.is_null() {
                if (*r).atts & JS_READONLY == 0 {
                    (*r).value = *value;
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
}

unsafe fn jsR_setindex(J: *mut js_State, obj: *mut js_Object, k: c_int, transient: c_int) {
    unsafe {
        let mut buf: [c_char; 32] = [0; 32];
        if (*obj).ty == JS_CARRAY
            && (*obj).u.a.simple != 0
            && k >= 0
            && k <= (*obj).u.a.flat_length
        {
            jsR_setarrayindex(J, obj, k, stackidx(J, -1));
        } else {
            jsR_setproperty(J, obj, js_itoa(buf.as_mut_ptr(), k), transient);
        }
    }
}

unsafe fn jsR_defproperty(
    J: *mut js_State,
    obj: *mut js_Object,
    name: *const c_char,
    atts: c_int,
    value: *mut js_Value,
    getter: *mut js_Object,
    setter: *mut js_Object,
    throw: c_int,
) {
    unsafe {
        let mut k: c_int = 0;

        'readonly: {
            if (*obj).ty == JS_CARRAY {
                if strcmp(name, c"length".as_ptr()) == 0 {
                    break 'readonly;
                }
                if (*obj).u.a.simple != 0 {
                    jsR_unflattenarray(J, obj);
                }
            } else if (*obj).ty == JS_CSTRING {
                if strcmp(name, c"length".as_ptr()) == 0 {
                    break 'readonly;
                }
                if js_isarrayindex(J, name, &mut k) != 0 {
                    if k >= 0 && k < (*obj).u.s.length {
                        break 'readonly;
                    }
                }
            } else if (*obj).ty == JS_CREGEXP {
                if strcmp(name, c"source".as_ptr()) == 0 {
                    break 'readonly;
                }
                if strcmp(name, c"global".as_ptr()) == 0 {
                    break 'readonly;
                }
                if strcmp(name, c"ignoreCase".as_ptr()) == 0 {
                    break 'readonly;
                }
                if strcmp(name, c"multiline".as_ptr()) == 0 {
                    break 'readonly;
                }
                if strcmp(name, c"lastIndex".as_ptr()) == 0 {
                    break 'readonly;
                }
            } else if (*obj).ty == JS_CUSERDATA {
                if (*obj).u.user.put.is_some()
                    && ((*obj).u.user.put.unwrap())(J, (*obj).u.user.data, name) != 0
                {
                    return;
                }
            }

            let r = jsV_setproperty(J, obj, name);
            if !r.is_null() {
                if !value.is_null() {
                    if (*r).atts & JS_READONLY == 0 {
                        (*r).value = *value;
                    } else if (*J).strict != 0 {
                        js_typeerror!(J, c"'%s' is read-only".as_ptr(), name);
                    }
                }
                if !getter.is_null() {
                    if (*r).atts & JS_DONTCONF == 0 {
                        (*r).getter = getter;
                    } else if (*J).strict != 0 {
                        js_typeerror!(J, c"'%s' is non-configurable".as_ptr(), name);
                    }
                }
                if !setter.is_null() {
                    if (*r).atts & JS_DONTCONF == 0 {
                        (*r).setter = setter;
                    } else if (*J).strict != 0 {
                        js_typeerror!(J, c"'%s' is non-configurable".as_ptr(), name);
                    }
                }
                (*r).atts |= atts;
            }

            return;
        }

        /* readonly: */
        if (*J).strict != 0 || throw != 0 {
            js_typeerror!(
                J,
                c"'%s' is read-only or non-configurable".as_ptr(),
                name
            );
        }
    }
}

unsafe fn jsR_delproperty(J: *mut js_State, obj: *mut js_Object, name: *const c_char) -> c_int {
    unsafe {
        let mut k: c_int = 0;

        'dontconf: {
            if (*obj).ty == JS_CARRAY {
                if strcmp(name, c"length".as_ptr()) == 0 {
                    break 'dontconf;
                }
                if (*obj).u.a.simple != 0 {
                    jsR_unflattenarray(J, obj);
                }
            } else if (*obj).ty == JS_CSTRING {
                if strcmp(name, c"length".as_ptr()) == 0 {
                    break 'dontconf;
                }
                if js_isarrayindex(J, name, &mut k) != 0 {
                    if k >= 0 && k < (*obj).u.s.length {
                        break 'dontconf;
                    }
                }
            } else if (*obj).ty == JS_CREGEXP {
                if strcmp(name, c"source".as_ptr()) == 0 {
                    break 'dontconf;
                }
                if strcmp(name, c"global".as_ptr()) == 0 {
                    break 'dontconf;
                }
                if strcmp(name, c"ignoreCase".as_ptr()) == 0 {
                    break 'dontconf;
                }
                if strcmp(name, c"multiline".as_ptr()) == 0 {
                    break 'dontconf;
                }
                if strcmp(name, c"lastIndex".as_ptr()) == 0 {
                    break 'dontconf;
                }
            } else if (*obj).ty == JS_CUSERDATA {
                if (*obj).u.user.delete.is_some()
                    && ((*obj).u.user.delete.unwrap())(J, (*obj).u.user.data, name) != 0
                {
                    return 1;
                }
            }

            let r = jsV_getownproperty(J, obj, name);
            if !r.is_null() {
                if (*r).atts & JS_DONTCONF != 0 {
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
}

unsafe fn jsR_delindex(J: *mut js_State, obj: *mut js_Object, k: c_int) {
    unsafe {
        let mut buf: [c_char; 32] = [0; 32];
        /* Allow deleting last element of a simple array without unflattening */
        if (*obj).ty == JS_CARRAY && (*obj).u.a.simple != 0 && k == (*obj).u.a.flat_length - 1 {
            (*obj).u.a.flat_length = k;
        } else {
            jsR_delproperty(J, obj, js_itoa(buf.as_mut_ptr(), k));
        }
    }
}

/* Registry, global and object property accessors */

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_ref(J: *mut js_State) -> *const c_char {
    unsafe {
        let v = stackidx(J, -1);
        let s: *const c_char;
        let mut buf: [c_char; 32] = [0; 32];
        match (*v).ty() {
            JS_TUNDEFINED => s = c"_Undefined".as_ptr(),
            JS_TNULL => s = c"_Null".as_ptr(),
            JS_TBOOLEAN => {
                s = if (*v).boolean != 0 {
                    c"_True".as_ptr()
                } else {
                    c"_False".as_ptr()
                };
            }
            JS_TOBJECT => {
                sprintf(buf.as_mut_ptr(), c"%p".as_ptr(), (*v).object as *mut c_void);
                s = js_intern(J, buf.as_ptr());
            }
            _ => {
                sprintf(buf.as_mut_ptr(), c"%d".as_ptr(), (*J).nextref);
                (*J).nextref += 1;
                s = js_intern(J, buf.as_ptr());
            }
        }
        js_setregistry(J, s);
        s
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_unref(J: *mut js_State, r: *const c_char) {
    unsafe {
        js_delregistry(J, r);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_getregistry(J: *mut js_State, name: *const c_char) {
    unsafe {
        jsR_getproperty(J, (*J).R, name);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_setregistry(J: *mut js_State, name: *const c_char) {
    unsafe {
        jsR_setproperty(J, (*J).R, name, 0);
        js_pop(J, 1);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_delregistry(J: *mut js_State, name: *const c_char) {
    unsafe {
        jsR_delproperty(J, (*J).R, name);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_getglobal(J: *mut js_State, name: *const c_char) {
    unsafe {
        jsR_getproperty(J, (*J).G, name);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_setglobal(J: *mut js_State, name: *const c_char) {
    unsafe {
        jsR_setproperty(J, (*J).G, name, 0);
        js_pop(J, 1);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_defglobal(J: *mut js_State, name: *const c_char, atts: c_int) {
    unsafe {
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
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_delglobal(J: *mut js_State, name: *const c_char) {
    unsafe {
        jsR_delproperty(J, (*J).G, name);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_getproperty(
    J: *mut js_State,
    idx: c_int,
    name: *const c_char,
) {
    unsafe {
        jsR_getproperty(J, js_toobject(J, idx), name);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_setproperty(
    J: *mut js_State,
    idx: c_int,
    name: *const c_char,
) {
    unsafe {
        let t = (js_isobject(J, idx) == 0) as c_int;
        jsR_setproperty(J, js_toobject(J, idx), name, t);
        js_pop(J, 1);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_defproperty(
    J: *mut js_State,
    idx: c_int,
    name: *const c_char,
    atts: c_int,
) {
    unsafe {
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
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_delproperty(
    J: *mut js_State,
    idx: c_int,
    name: *const c_char,
) {
    unsafe {
        jsR_delproperty(J, js_toobject(J, idx), name);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_defaccessor(
    J: *mut js_State,
    idx: c_int,
    name: *const c_char,
    atts: c_int,
) {
    unsafe {
        let obj = js_toobject(J, idx);
        let g = jsR_tofunction(J, -2);
        let s = jsR_tofunction(J, -1);
        jsR_defproperty(J, obj, name, atts, null_mut(), g, s, 1);
        js_pop(J, 2);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_hasproperty(
    J: *mut js_State,
    idx: c_int,
    name: *const c_char,
) -> c_int {
    unsafe { jsR_hasproperty(J, js_toobject(J, idx), name) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_getindex(J: *mut js_State, idx: c_int, i: c_int) {
    unsafe {
        jsR_getindex(J, js_toobject(J, idx), i);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_hasindex(J: *mut js_State, idx: c_int, i: c_int) -> c_int {
    unsafe { jsR_hasindex(J, js_toobject(J, idx), i) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_setindex(J: *mut js_State, idx: c_int, i: c_int) {
    unsafe {
        let t = (js_isobject(J, idx) == 0) as c_int;
        jsR_setindex(J, js_toobject(J, idx), i, t);
        js_pop(J, 1);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_delindex(J: *mut js_State, idx: c_int, i: c_int) {
    unsafe {
        jsR_delindex(J, js_toobject(J, idx), i);
    }
}

/* Iterator */

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_pushiterator(J: *mut js_State, idx: c_int, own: c_int) {
    unsafe {
        js_pushobject(J, jsV_newiterator(J, js_toobject(J, idx), own));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_nextiterator(J: *mut js_State, idx: c_int) -> *const c_char {
    unsafe { jsV_nextiterator(J, js_toobject(J, idx)) }
}

/* Environment records */

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsR_newenvironment(
    J: *mut js_State,
    vars: *mut js_Object,
    outer: *mut js_Environment,
) -> *mut js_Environment {
    unsafe {
        let E = js_malloc(J, core::mem::size_of::<js_Environment>() as c_int) as *mut js_Environment;
        (*E).gcmark = 0;
        (*E).gcnext = (*J).gcenv;
        (*J).gcenv = E;
        (*J).gccounter += 1;

        (*E).outer = outer;
        (*E).variables = vars;
        E
    }
}

unsafe fn js_initvar(J: *mut js_State, name: *const c_char, idx: c_int) {
    unsafe {
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
}

unsafe fn js_hasvar(J: *mut js_State, name: *const c_char) -> c_int {
    unsafe {
        let mut E = (*J).E;
        loop {
            let r = jsV_getproperty(J, (*E).variables, name);
            if !r.is_null() {
                if !(*r).getter.is_null() {
                    js_pushobject(J, (*r).getter);
                    js_pushobject(J, (*E).variables);
                    js_call(J, 0);
                } else {
                    js_pushvalue(J, (*r).value);
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
}

unsafe fn js_setvar(J: *mut js_State, name: *const c_char) {
    unsafe {
        let mut E = (*J).E;
        loop {
            let r = jsV_getproperty(J, (*E).variables, name);
            if !r.is_null() {
                if !(*r).setter.is_null() {
                    js_pushobject(J, (*r).setter);
                    js_pushobject(J, (*E).variables);
                    js_copy(J, -3);
                    js_call(J, 1);
                    js_pop(J, 1);
                    return;
                }
                if (*r).atts & JS_READONLY == 0 {
                    (*r).value = *stackidx(J, -1);
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
}

unsafe fn js_delvar(J: *mut js_State, name: *const c_char) -> c_int {
    unsafe {
        let mut E = (*J).E;
        loop {
            let r = jsV_getownproperty(J, (*E).variables, name);
            if !r.is_null() {
                if (*r).atts & JS_DONTCONF != 0 {
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
}

/* Function calls */

unsafe fn jsR_savescope(J: *mut js_State, newE: *mut js_Environment) {
    unsafe {
        if (*J).envtop + 1 >= JS_ENVLIMIT as c_int {
            js_stackoverflow(J);
        }
        (*J).envstack[(*J).envtop as usize] = (*J).E;
        (*J).envtop += 1;
        (*J).E = newE;
    }
}

unsafe fn jsR_restorescope(J: *mut js_State) {
    unsafe {
        (*J).envtop -= 1;
        (*J).E = (*J).envstack[(*J).envtop as usize];
    }
}

unsafe fn jsR_calllwfunction(
    J: *mut js_State,
    n: c_int,
    F: *mut js_Function,
    scope: *mut js_Environment,
) {
    unsafe {
        let v: js_Value;
        let mut i: c_int;
        let mut n = n;

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
        (*J).top = (*J).bot; /* clear stack */
        js_pushvalue(J, v);

        jsR_restorescope(J);
    }
}

unsafe fn jsR_callfunction(
    J: *mut js_State,
    n: c_int,
    F: *mut js_Function,
    scope: *mut js_Environment,
) {
    unsafe {
        let v: js_Value;
        let mut i: c_int;

        let scope = jsR_newenvironment(J, jsV_newobject(J, JS_COBJECT, null_mut()), scope);

        jsR_savescope(J, scope);

        if (*F).arguments != 0 {
            js_newarguments(J);
            if (*J).strict == 0 {
                js_currentfunction(J);
                js_defproperty(J, -2, c"callee".as_ptr(), JS_DONTENUM);
            }
            js_pushnumber(J, n as f64);
            js_defproperty(J, -2, c"length".as_ptr(), JS_DONTENUM);
            i = 0;
            while i < n {
                js_copy(J, i + 1);
                js_setindex(J, -2, i);
                i += 1;
            }
            js_initvar(J, c"arguments".as_ptr(), -1);
            js_pop(J, 1);
        }

        i = 0;
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
        v = *stackidx(J, -1);
        (*J).bot -= 1;
        (*J).top = (*J).bot; /* clear stack */
        js_pushvalue(J, v);

        jsR_restorescope(J);
    }
}

unsafe fn jsR_callscript(
    J: *mut js_State,
    n: c_int,
    F: *mut js_Function,
    scope: *mut js_Environment,
) {
    unsafe {
        let v: js_Value;
        let mut i: c_int;

        if !scope.is_null() {
            jsR_savescope(J, scope);
        }

        /* scripts take no arguments */
        js_pop(J, n);

        i = 0;
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
        v = *stackidx(J, -1);
        (*J).bot -= 1;
        (*J).top = (*J).bot; /* clear stack */
        js_pushvalue(J, v);

        if !scope.is_null() {
            jsR_restorescope(J);
        }
    }
}

unsafe fn jsR_callcfunction(J: *mut js_State, n: c_int, min: c_int, F: js_CFunction) {
    unsafe {
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
            (*J).top = (*J).bot; /* clear stack */
            js_pushvalue(J, v);
        } else {
            (*J).bot -= 1;
            (*J).top = (*J).bot; /* clear stack */
            js_pushundefined(J);
        }
    }
}

unsafe fn jsR_pushtrace(
    J: *mut js_State,
    name: *const c_char,
    file: *const c_char,
    line: c_int,
) {
    unsafe {
        if (*J).tracetop + 1 == JS_ENVLIMIT as c_int {
            js_error!(J, c"call stack overflow".as_ptr());
        }
        (*J).tracetop += 1;
        let t = (*J).tracetop as usize;
        (*J).trace[t].stack = (*J).bot;
        (*J).trace[t].name = name;
        (*J).trace[t].file = file;
        (*J).trace[t].line = line;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_call(J: *mut js_State, n: c_int) {
    unsafe {
        let obj: *mut js_Object;
        let savebot: c_int;

        if n < 0 {
            js_rangeerror!(J, c"number of arguments cannot be negative".as_ptr());
        }

        if js_iscallable(J, -n - 2) == 0 {
            js_typeerror!(J, c"%s is not callable".as_ptr(), js_typeof(J, -n - 2));
        }

        obj = js_toobject(J, -n - 2);

        savebot = (*J).bot;
        (*J).bot = (*J).top - n - 1;

        if (*obj).ty == JS_CFUNCTION {
            jsR_pushtrace(
                J,
                (*(*obj).u.f.function).name,
                (*(*obj).u.f.function).filename,
                (*(*obj).u.f.function).line,
            );
            if (*(*obj).u.f.function).lightweight != 0 {
                jsR_calllwfunction(J, n, (*obj).u.f.function, (*obj).u.f.scope);
            } else {
                jsR_callfunction(J, n, (*obj).u.f.function, (*obj).u.f.scope);
            }
            (*J).tracetop -= 1;
        } else if (*obj).ty == JS_CSCRIPT {
            jsR_pushtrace(
                J,
                (*(*obj).u.f.function).name,
                (*(*obj).u.f.function).filename,
                (*(*obj).u.f.function).line,
            );
            jsR_callscript(J, n, (*obj).u.f.function, (*obj).u.f.scope);
            (*J).tracetop -= 1;
        } else if (*obj).ty == JS_CCFUNCTION {
            jsR_pushtrace(J, (*obj).u.c.name, c"native".as_ptr(), 0);
            jsR_callcfunction(J, n, (*obj).u.c.length, (*obj).u.c.function);
            (*J).tracetop -= 1;
        }

        (*J).bot = savebot;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_construct(J: *mut js_State, n: c_int) {
    unsafe {
        let obj: *mut js_Object;
        let prototype: *mut js_Object;
        let newobj: *mut js_Object;

        if js_iscallable(J, -n - 1) == 0 {
            js_typeerror!(J, c"%s is not callable".as_ptr(), js_typeof(J, -n - 1));
        }

        obj = js_toobject(J, -n - 1);

        /* built-in constructors create their own objects, give them a 'null' this */
        if (*obj).ty == JS_CCFUNCTION && (*obj).u.c.constructor.is_some() {
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
        newobj = jsV_newobject(J, JS_COBJECT, prototype);
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
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_eval(J: *mut js_State) {
    unsafe {
        if js_isstring(J, -1) == 0 {
            return;
        }
        js_loadeval(J, c"(eval)".as_ptr(), js_tostring(J, -1));
        js_rot2pop1(J);
        js_copy(J, 0); /* copy 'this' */
        js_call(J, 0);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_pconstruct(J: *mut js_State, n: c_int) -> c_int {
    unsafe {
        let savetop = (*J).top - n - 2;
        if crate::except::js_try_run(J, || {
            js_construct(J, n);
            js_endtry(J);
        }) {
            /* clean up the stack to only hold the error object */
            *(*J).stack.offset(savetop as isize) = *(*J).stack.offset(((*J).top - 1) as isize);
            (*J).top = savetop + 1;
            return 1;
        }
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_pcall(J: *mut js_State, n: c_int) -> c_int {
    unsafe {
        let savetop = (*J).top - n - 2;
        if crate::except::js_try_run(J, || {
            js_call(J, n);
            js_endtry(J);
        }) {
            /* clean up the stack to only hold the error object */
            *(*J).stack.offset(savetop as isize) = *(*J).stack.offset(((*J).top - 1) as isize);
            (*J).top = savetop + 1;
            return 1;
        }
        0
    }
}

/* Exceptions */

/// Push a try frame; returns its index in `J->trybuf`.
pub(crate) unsafe fn js_pushtry(
    J: *mut js_State,
    pc: *mut js_Instruction,
    kind: c_int,
) -> c_int {
    unsafe {
        if (*J).trytop == JS_TRYLIMIT {
            js_trystackoverflow(J);
        }
        let t = (*J).trytop as usize;
        (*J).trybuf[t].E = (*J).E;
        (*J).trybuf[t].envtop = (*J).envtop;
        (*J).trybuf[t].tracetop = (*J).tracetop;
        (*J).trybuf[t].top = (*J).top;
        (*J).trybuf[t].bot = (*J).bot;
        (*J).trybuf[t].strict = (*J).strict;
        (*J).trybuf[t].pc = pc;
        (*J).trybuf[t].kind = kind;
        (*J).trytop += 1;
        t as c_int
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_savetrypc(
    J: *mut js_State,
    pc: *mut js_Instruction,
) -> *mut c_void {
    unsafe {
        let t = js_pushtry(J, pc, TRY_EXTERNAL);
        (&raw mut (*J).trybuf[t as usize].buf) as *mut c_void
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_savetry(J: *mut js_State) -> *mut c_void {
    unsafe {
        let t = js_pushtry(J, null_mut(), TRY_EXTERNAL);
        (&raw mut (*J).trybuf[t as usize].buf) as *mut c_void
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_endtry(J: *mut js_State) {
    unsafe {
        if (*J).trytop == 0 {
            js_error!(J, c"endtry: exception stack underflow".as_ptr());
        }
        (*J).trytop -= 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_throw(J: *mut js_State) -> ! {
    unsafe {
        if (*J).trytop > 0 {
            let v = *stackidx(J, -1);
            (*J).trytop -= 1;
            let t = (*J).trytop as usize;
            (*J).E = (*J).trybuf[t].E;
            (*J).envtop = (*J).trybuf[t].envtop;
            (*J).tracetop = (*J).trybuf[t].tracetop;
            (*J).top = (*J).trybuf[t].top;
            (*J).bot = (*J).trybuf[t].bot;
            (*J).strict = (*J).trybuf[t].strict;
            js_pushvalue(J, v);
            (*J).throwtarget = t as c_int;
            if (*J).trybuf[t].kind == TRY_EXTERNAL {
                longjmp((&raw mut (*J).trybuf[t].buf) as *mut c_void, 1);
            }
            raise();
        }
        if (*J).panic.is_some() {
            ((*J).panic.unwrap())(J);
        }
        abort()
    }
}

/* Main interpreter loop */

unsafe fn js_dumpvalue(J: *mut js_State, v: js_Value) {
    unsafe {
        match v.ty() {
            JS_TUNDEFINED => {
                printf(c"undefined".as_ptr());
            }
            JS_TNULL => {
                printf(c"null".as_ptr());
            }
            JS_TBOOLEAN => {
                printf(if v.boolean != 0 {
                    c"true".as_ptr()
                } else {
                    c"false".as_ptr()
                });
            }
            JS_TNUMBER => {
                printf(c"%.9g".as_ptr(), v.number);
            }
            JS_TSHRSTR => {
                printf(c"'%s'".as_ptr(), (&raw const v.shrstr) as *const c_char);
            }
            JS_TLITSTR => {
                printf(c"'%s'".as_ptr(), v.litstr);
            }
            JS_TMEMSTR => {
                printf(c"'%s'".as_ptr(), (&raw const (*v.memstr).p) as *const c_char);
            }
            JS_TOBJECT => {
                if v.object == (*J).G {
                    printf(c"[Global]".as_ptr());
                    return;
                }
                match (*v.object).ty {
                    JS_COBJECT => {
                        printf(c"[Object %p]".as_ptr(), v.object as *mut c_void);
                    }
                    JS_CARRAY => {
                        printf(c"[Array %p]".as_ptr(), v.object as *mut c_void);
                    }
                    JS_CFUNCTION => {
                        printf(
                            c"[Function %p, %s, %s:%d]".as_ptr(),
                            v.object as *mut c_void,
                            (*(*v.object).u.f.function).name,
                            (*(*v.object).u.f.function).filename,
                            (*(*v.object).u.f.function).line,
                        );
                    }
                    JS_CSCRIPT => {
                        printf(
                            c"[Script %s]".as_ptr(),
                            (*(*v.object).u.f.function).filename,
                        );
                    }
                    JS_CCFUNCTION => {
                        printf(c"[CFunction %s]".as_ptr(), (*v.object).u.c.name);
                    }
                    JS_CBOOLEAN => {
                        printf(c"[Boolean %d]".as_ptr(), (*v.object).u.boolean);
                    }
                    JS_CNUMBER => {
                        printf(c"[Number %g]".as_ptr(), (*v.object).u.number);
                    }
                    JS_CSTRING => {
                        printf(c"[String'%s']".as_ptr(), (*v.object).u.s.string);
                    }
                    JS_CERROR => {
                        printf(c"[Error]".as_ptr());
                    }
                    JS_CARGUMENTS => {
                        printf(c"[Arguments %p]".as_ptr(), v.object as *mut c_void);
                    }
                    JS_CITERATOR => {
                        printf(c"[Iterator %p]".as_ptr(), v.object as *mut c_void);
                    }
                    JS_CUSERDATA => {
                        printf(
                            c"[Userdata %s %p]".as_ptr(),
                            (*v.object).u.user.tag,
                            (*v.object).u.user.data,
                        );
                    }
                    _ => {
                        printf(c"[Object %p]".as_ptr(), v.object as *mut c_void);
                    }
                }
            }
            _ => {}
        }
    }
}

unsafe fn js_stacktrace(J: *mut js_State) {
    unsafe {
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
}

unsafe fn js_dumpstack(J: *mut js_State) {
    unsafe {
        printf(c"stack {\n".as_ptr());
        let mut i = 0;
        while i < (*J).top {
            putchar(if i == (*J).bot { '>' as c_int } else { ' ' as c_int });
            printf(c"%4d: ".as_ptr(), i);
            js_dumpvalue(J, *(*J).stack.offset(i as isize));
            putchar('\n' as c_int);
            i += 1;
        }
        printf(c"}\n".as_ptr());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_trap(J: *mut js_State, pc: c_int) {
    unsafe {
        js_dumpstack(J);
        js_stacktrace(J);
    }
}

unsafe fn jsR_isindex(J: *mut js_State, idx: c_int, k: *mut c_int) -> c_int {
    unsafe {
        let v = stackidx(J, idx);
        if (*v).ty() == JS_TNUMBER {
            *k = d2i((*v).number);
            return ((*k) as f64 == (*v).number && *k >= 0) as c_int;
        }
        0
    }
}

struct RunCtx {
    F: *mut js_Function,
    FT: *mut *mut js_Function,
    VT: *mut *const c_char,
    lightweight: c_int,
    pcstart: *mut js_Instruction,
    savestrict: c_int,
}

unsafe fn jsR_run(J: *mut js_State, F: *mut js_Function) {
    unsafe {
        let ctx = RunCtx {
            F: F,
            FT: (*F).funtab,
            VT: if !(*F).vartab.is_null() {
                (*F).vartab.offset(-1)
            } else {
                null_mut()
            },
            lightweight: (*F).lightweight,
            pcstart: (*F).code,
            savestrict: (*J).strict,
        };

        let mut pc: *mut js_Instruction = (*F).code;
        (*J).strict = (*F).strict;

        let entry_trytop = (*J).trytop;

        loop {
            let pcp: *mut *mut js_Instruction = &mut pc;
            let ctxp: *const RunCtx = &ctx;
            let r = catch_unwind(AssertUnwindSafe(|| jsR_runloop(J, ctxp, pcp)));
            match r {
                Ok(()) => return,
                Err(p) => {
                    if is_js_throw(&p) && (*J).throwtarget >= entry_trytop {
                        /* an OP_TRY frame established by this jsR_run invocation */
                        pc = (*J).trybuf[(*J).throwtarget as usize].pc;
                        continue;
                    }
                    resume_unwind(p)
                }
            }
        }
    }
}

unsafe fn jsR_runloop(
    J: *mut js_State,
    ctxp: *const RunCtx,
    pcp: *mut *mut js_Instruction,
) {
    unsafe {
        let FT = (*ctxp).FT;
        let VT = (*ctxp).VT;
        let lightweight = (*ctxp).lightweight;
        let pcstart = (*ctxp).pcstart;
        let savestrict = (*ctxp).savestrict;

        let mut pc: *mut js_Instruction = *pcp;

        macro_rules! SYNC {
            () => {
                *pcp = pc;
            };
        }
        macro_rules! FETCH {
            () => {{
                let v = *pc;
                pc = pc.add(1);
                v
            }};
        }
        macro_rules! READSTRING {
            () => {{
                let mut s: *const c_char = null();
                memcpy(
                    (&raw mut s) as *mut c_void,
                    pc as *const c_void,
                    core::mem::size_of::<*const c_char>(),
                );
                pc = pc.add(
                    core::mem::size_of::<*const c_char>()
                        / core::mem::size_of::<js_Instruction>(),
                );
                s
            }};
        }

        let mut offset: c_int;

        let mut str: *const c_char;
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

        loop {
            if (*J).runlimit > 0 {
                if (*J).runlimit == 1 {
                    SYNC!();
                    js_runlimit(J);
                }
                (*J).runlimit -= 1;
            }

            if (*J).gccounter > (*J).gcthresh {
                SYNC!();
                js_gc(J, 0);
            }

            (*J).trace[(*J).tracetop as usize].line = FETCH!() as c_int;

            let opcode = FETCH!() as c_int;
            SYNC!();

            if opcode == OP_POP {
                js_pop(J, 1);
            } else if opcode == OP_DUP {
                js_dup(J);
            } else if opcode == OP_DUP2 {
                js_dup2(J);
            } else if opcode == OP_ROT2 {
                js_rot2(J);
            } else if opcode == OP_ROT3 {
                js_rot3(J);
            } else if opcode == OP_ROT4 {
                js_rot4(J);
            } else if opcode == OP_INTEGER {
                let k = FETCH!() as c_int;
                SYNC!();
                js_pushnumber(J, (k - 32768) as f64);
            } else if opcode == OP_NUMBER {
                let mut xx: f64 = 0.0;
                memcpy(
                    (&raw mut xx) as *mut c_void,
                    pc as *const c_void,
                    core::mem::size_of::<f64>(),
                );
                pc = pc.add(core::mem::size_of::<f64>() / core::mem::size_of::<js_Instruction>());
                SYNC!();
                js_pushnumber(J, xx);
            } else if opcode == OP_STRING {
                str = READSTRING!();
                SYNC!();
                js_pushliteral(J, str);
            } else if opcode == OP_CLOSURE {
                let k = FETCH!() as usize;
                SYNC!();
                js_newfunction(J, *FT.add(k), (*J).E);
            } else if opcode == OP_NEWOBJECT {
                js_newobject(J);
            } else if opcode == OP_NEWARRAY {
                js_newarray(J);
            } else if opcode == OP_NEWREGEXP {
                str = READSTRING!();
                let f = FETCH!() as c_int;
                SYNC!();
                js_newregexp(J, str, f);
            } else if opcode == OP_UNDEF {
                js_pushundefined(J);
            } else if opcode == OP_NULL {
                js_pushnull(J);
            } else if opcode == OP_TRUE {
                js_pushboolean(J, 1);
            } else if opcode == OP_FALSE {
                js_pushboolean(J, 0);
            } else if opcode == OP_THIS {
                if (*J).strict != 0 {
                    js_copy(J, 0);
                } else {
                    if js_iscoercible(J, 0) != 0 {
                        js_copy(J, 0);
                    } else {
                        js_pushglobal(J);
                    }
                }
            } else if opcode == OP_CURRENT {
                js_currentfunction(J);
            } else if opcode == OP_GETLOCAL {
                if lightweight != 0 {
                    CHECKSTACK!(J, 1);
                    let k = FETCH!() as c_int;
                    SYNC!();
                    *(*J).stack.offset((*J).top as isize) =
                        *(*J).stack.offset(((*J).bot + k) as isize);
                    (*J).top += 1;
                } else {
                    let k = FETCH!() as usize;
                    SYNC!();
                    str = *VT.add(k);
                    if js_hasvar(J, str) == 0 {
                        js_referenceerror!(J, c"'%s' is not defined".as_ptr(), str);
                    }
                }
            } else if opcode == OP_SETLOCAL {
                if lightweight != 0 {
                    let k = FETCH!() as c_int;
                    SYNC!();
                    *(*J).stack.offset(((*J).bot + k) as isize) =
                        *(*J).stack.offset(((*J).top - 1) as isize);
                } else {
                    let k = FETCH!() as usize;
                    SYNC!();
                    js_setvar(J, *VT.add(k));
                }
            } else if opcode == OP_DELLOCAL {
                if lightweight != 0 {
                    pc = pc.add(1);
                    SYNC!();
                    js_pushboolean(J, 0);
                } else {
                    let k = FETCH!() as usize;
                    SYNC!();
                    b = js_delvar(J, *VT.add(k));
                    js_pushboolean(J, b);
                }
            } else if opcode == OP_GETVAR {
                str = READSTRING!();
                SYNC!();
                if js_hasvar(J, str) == 0 {
                    js_referenceerror!(J, c"'%s' is not defined".as_ptr(), str);
                }
            } else if opcode == OP_HASVAR {
                str = READSTRING!();
                SYNC!();
                if js_hasvar(J, str) == 0 {
                    js_pushundefined(J);
                }
            } else if opcode == OP_SETVAR {
                str = READSTRING!();
                SYNC!();
                js_setvar(J, str);
            } else if opcode == OP_DELVAR {
                str = READSTRING!();
                SYNC!();
                b = js_delvar(J, str);
                js_pushboolean(J, b);
            } else if opcode == OP_IN {
                str = js_tostring(J, -2);
                if js_isobject(J, -1) == 0 {
                    js_typeerror!(J, c"operand to 'in' is not an object".as_ptr());
                }
                b = js_hasproperty(J, -1, str);
                js_pop(J, 2 + b);
                js_pushboolean(J, b);
            } else if opcode == OP_SKIPARRAY {
                js_setlength(J, -1, js_getlength(J, -1) + 1);
            } else if opcode == OP_INITARRAY {
                js_setindex(J, -2, js_getlength(J, -2));
            } else if opcode == OP_INITPROP {
                obj = js_toobject(J, -3);
                str = js_tostring(J, -2);
                jsR_setproperty(J, obj, str, 0);
                js_pop(J, 2);
            } else if opcode == OP_INITGETTER {
                obj = js_toobject(J, -3);
                str = js_tostring(J, -2);
                jsR_defproperty(
                    J,
                    obj,
                    str,
                    0,
                    null_mut(),
                    jsR_tofunction(J, -1),
                    null_mut(),
                    0,
                );
                js_pop(J, 2);
            } else if opcode == OP_INITSETTER {
                obj = js_toobject(J, -3);
                str = js_tostring(J, -2);
                jsR_defproperty(
                    J,
                    obj,
                    str,
                    0,
                    null_mut(),
                    null_mut(),
                    jsR_tofunction(J, -1),
                    0,
                );
                js_pop(J, 2);
            } else if opcode == OP_GETPROP {
                if jsR_isindex(J, -1, &mut ix) != 0 {
                    obj = js_toobject(J, -2);
                    jsR_getindex(J, obj, ix);
                } else {
                    str = js_tostring(J, -1);
                    obj = js_toobject(J, -2);
                    jsR_getproperty(J, obj, str);
                }
                js_rot3pop2(J);
            } else if opcode == OP_GETPROP_S {
                str = READSTRING!();
                SYNC!();
                obj = js_toobject(J, -1);
                jsR_getproperty(J, obj, str);
                js_rot2pop1(J);
            } else if opcode == OP_SETPROP {
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
            } else if opcode == OP_SETPROP_S {
                str = READSTRING!();
                SYNC!();
                obj = js_toobject(J, -2);
                transient = (js_isobject(J, -2) == 0) as c_int;
                jsR_setproperty(J, obj, str, transient);
                js_rot2pop1(J);
            } else if opcode == OP_DELPROP {
                str = js_tostring(J, -1);
                obj = js_toobject(J, -2);
                b = jsR_delproperty(J, obj, str);
                js_pop(J, 2);
                js_pushboolean(J, b);
            } else if opcode == OP_DELPROP_S {
                str = READSTRING!();
                SYNC!();
                obj = js_toobject(J, -1);
                b = jsR_delproperty(J, obj, str);
                js_pop(J, 1);
                js_pushboolean(J, b);
            } else if opcode == OP_ITERATOR {
                if js_iscoercible(J, -1) != 0 {
                    obj = jsV_newiterator(J, js_toobject(J, -1), 0);
                    js_pop(J, 1);
                    js_pushobject(J, obj);
                }
            } else if opcode == OP_NEXTITER {
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
            } else if opcode == OP_EVAL {
                js_eval(J);
            } else if opcode == OP_CALL {
                let k = FETCH!() as c_int;
                SYNC!();
                js_call(J, k);
            } else if opcode == OP_NEW {
                let k = FETCH!() as c_int;
                SYNC!();
                js_construct(J, k);
            } else if opcode == OP_TYPEOF {
                str = js_typeof(J, -1);
                js_pop(J, 1);
                js_pushliteral(J, str);
            } else if opcode == OP_POS {
                x = js_tonumber(J, -1);
                js_pop(J, 1);
                js_pushnumber(J, x);
            } else if opcode == OP_NEG {
                x = js_tonumber(J, -1);
                js_pop(J, 1);
                js_pushnumber(J, -x);
            } else if opcode == OP_BITNOT {
                ix = js_toint32(J, -1);
                js_pop(J, 1);
                js_pushnumber(J, (!ix) as f64);
            } else if opcode == OP_LOGNOT {
                b = js_toboolean(J, -1);
                js_pop(J, 1);
                js_pushboolean(J, (b == 0) as c_int);
            } else if opcode == OP_INC {
                x = js_tonumber(J, -1);
                js_pop(J, 1);
                js_pushnumber(J, x + 1.0);
            } else if opcode == OP_DEC {
                x = js_tonumber(J, -1);
                js_pop(J, 1);
                js_pushnumber(J, x - 1.0);
            } else if opcode == OP_POSTINC {
                x = js_tonumber(J, -1);
                js_pop(J, 1);
                js_pushnumber(J, x + 1.0);
                js_pushnumber(J, x);
            } else if opcode == OP_POSTDEC {
                x = js_tonumber(J, -1);
                js_pop(J, 1);
                js_pushnumber(J, x - 1.0);
                js_pushnumber(J, x);
            } else if opcode == OP_MUL {
                x = js_tonumber(J, -2);
                y = js_tonumber(J, -1);
                js_pop(J, 2);
                js_pushnumber(J, x * y);
            } else if opcode == OP_DIV {
                x = js_tonumber(J, -2);
                y = js_tonumber(J, -1);
                js_pop(J, 2);
                js_pushnumber(J, x / y);
            } else if opcode == OP_MOD {
                x = js_tonumber(J, -2);
                y = js_tonumber(J, -1);
                js_pop(J, 2);
                js_pushnumber(J, fmod(x, y));
            } else if opcode == OP_ADD {
                js_concat(J);
            } else if opcode == OP_SUB {
                x = js_tonumber(J, -2);
                y = js_tonumber(J, -1);
                js_pop(J, 2);
                js_pushnumber(J, x - y);
            } else if opcode == OP_SHL {
                ix = js_toint32(J, -2);
                uy = js_touint32(J, -1);
                js_pop(J, 2);
                js_pushnumber(J, (ix.wrapping_shl(uy & 0x1F)) as f64);
            } else if opcode == OP_SHR {
                ix = js_toint32(J, -2);
                uy = js_touint32(J, -1);
                js_pop(J, 2);
                js_pushnumber(J, (ix >> (uy & 0x1F)) as f64);
            } else if opcode == OP_USHR {
                ux = js_touint32(J, -2);
                uy = js_touint32(J, -1);
                js_pop(J, 2);
                js_pushnumber(J, (ux >> (uy & 0x1F)) as f64);
            } else if opcode == OP_LT {
                b = js_compare(J, &mut okay);
                js_pop(J, 2);
                js_pushboolean(J, (okay != 0 && b < 0) as c_int);
            } else if opcode == OP_GT {
                b = js_compare(J, &mut okay);
                js_pop(J, 2);
                js_pushboolean(J, (okay != 0 && b > 0) as c_int);
            } else if opcode == OP_LE {
                b = js_compare(J, &mut okay);
                js_pop(J, 2);
                js_pushboolean(J, (okay != 0 && b <= 0) as c_int);
            } else if opcode == OP_GE {
                b = js_compare(J, &mut okay);
                js_pop(J, 2);
                js_pushboolean(J, (okay != 0 && b >= 0) as c_int);
            } else if opcode == OP_INSTANCEOF {
                b = js_instanceof(J);
                js_pop(J, 2);
                js_pushboolean(J, b);
            } else if opcode == OP_EQ {
                b = js_equal(J);
                js_pop(J, 2);
                js_pushboolean(J, b);
            } else if opcode == OP_NE {
                b = js_equal(J);
                js_pop(J, 2);
                js_pushboolean(J, (b == 0) as c_int);
            } else if opcode == OP_STRICTEQ {
                b = js_strictequal(J);
                js_pop(J, 2);
                js_pushboolean(J, b);
            } else if opcode == OP_STRICTNE {
                b = js_strictequal(J);
                js_pop(J, 2);
                js_pushboolean(J, (b == 0) as c_int);
            } else if opcode == OP_JCASE {
                offset = FETCH!() as c_int;
                SYNC!();
                b = js_strictequal(J);
                if b != 0 {
                    js_pop(J, 2);
                    pc = pcstart.offset(offset as isize);
                    SYNC!();
                } else {
                    js_pop(J, 1);
                }
            } else if opcode == OP_BITAND {
                ix = js_toint32(J, -2);
                iy = js_toint32(J, -1);
                js_pop(J, 2);
                js_pushnumber(J, (ix & iy) as f64);
            } else if opcode == OP_BITXOR {
                ix = js_toint32(J, -2);
                iy = js_toint32(J, -1);
                js_pop(J, 2);
                js_pushnumber(J, (ix ^ iy) as f64);
            } else if opcode == OP_BITOR {
                ix = js_toint32(J, -2);
                iy = js_toint32(J, -1);
                js_pop(J, 2);
                js_pushnumber(J, (ix | iy) as f64);
            } else if opcode == OP_THROW {
                js_throw(J);
            } else if opcode == OP_TRY {
                offset = FETCH!() as c_int;
                SYNC!();
                js_pushtry(J, pc, TRY_RUST);
                pc = pcstart.offset(offset as isize);
                SYNC!();
            } else if opcode == OP_ENDTRY {
                js_endtry(J);
            } else if opcode == OP_CATCH {
                str = READSTRING!();
                SYNC!();
                obj = jsV_newobject(J, JS_COBJECT, null_mut());
                js_pushobject(J, obj);
                js_rot2(J);
                js_setproperty(J, -2, str);
                (*J).E = jsR_newenvironment(J, obj, (*J).E);
                js_pop(J, 1);
            } else if opcode == OP_ENDCATCH {
                (*J).E = (*(*J).E).outer;
            } else if opcode == OP_WITH {
                obj = js_toobject(J, -1);
                (*J).E = jsR_newenvironment(J, obj, (*J).E);
                js_pop(J, 1);
            } else if opcode == OP_ENDWITH {
                (*J).E = (*(*J).E).outer;
            } else if opcode == OP_DEBUGGER {
                js_trap(J, (pc.offset_from(pcstart) as c_int) - 1);
            } else if opcode == OP_JUMP {
                pc = pcstart.offset(*pc as isize);
                SYNC!();
            } else if opcode == OP_JTRUE {
                offset = FETCH!() as c_int;
                SYNC!();
                b = js_toboolean(J, -1);
                js_pop(J, 1);
                if b != 0 {
                    pc = pcstart.offset(offset as isize);
                    SYNC!();
                }
            } else if opcode == OP_JFALSE {
                offset = FETCH!() as c_int;
                SYNC!();
                b = js_toboolean(J, -1);
                js_pop(J, 1);
                if b == 0 {
                    pc = pcstart.offset(offset as isize);
                    SYNC!();
                }
            } else if opcode == OP_RETURN {
                (*J).strict = savestrict;
                SYNC!();
                return;
            }

            SYNC!();
        }
    }
}
