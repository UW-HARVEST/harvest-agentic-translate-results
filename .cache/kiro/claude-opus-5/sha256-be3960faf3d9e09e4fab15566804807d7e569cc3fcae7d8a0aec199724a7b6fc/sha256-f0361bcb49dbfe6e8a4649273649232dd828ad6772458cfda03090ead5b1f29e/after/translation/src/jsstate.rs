//! Translation of src/jsstate.c
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(unused)]

use crate::jsi::*;
use crate::jsvalue::js_newscript;
use core::ptr::{null, null_mut};

use crate::jsrun::{
    js_call, js_endtry, js_pop, js_pushundefined, js_throw, js_toboolean,
    js_tointeger, js_tonumber, js_tostring, jsR_newenvironment,
};
use crate::jsproperty::jsV_newobject;
use crate::jsbuiltin::jsB_init;
use crate::jsgc::js_freestate;
use crate::jsparse::{jsP_freeparse, jsP_parse};
use crate::jscompile::jsC_compilescript;

/* --- static helpers --- */

unsafe fn js_ptry(J: *mut js_State) -> c_int {
    unsafe {
        if (*J).trytop == JS_TRYLIMIT {
            let top = (*J).top as usize;
            (*(*J).stack.add(top)).set_ty(JS_TLITSTR);
            (*(*J).stack.add(top)).litstr = c"exception stack overflow".as_ptr();
            (*J).top += 1;
            return 1;
        }
        0
    }
}

unsafe extern "C-unwind" fn js_defaultalloc(
    actx: *mut c_void,
    ptr: *mut c_void,
    size: c_int,
) -> *mut c_void {
    unsafe {
        if size == 0 {
            free(ptr);
            return null_mut();
        }
        realloc(ptr, size as size_t)
    }
}

unsafe extern "C-unwind" fn js_defaultreport(J: *mut js_State, message: *const c_char) {
    unsafe {
        fputs(message, stderr);
        fputc('\n' as c_int, stderr);
    }
}

unsafe extern "C-unwind" fn js_defaultpanic(J: *mut js_State) {
    unsafe {
        js_report(J, c"uncaught exception".as_ptr());
        /* return to javascript to abort */
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_ploadstring(
    J: *mut js_State,
    filename: *const c_char,
    source: *const c_char,
) -> c_int {
    unsafe {
        if js_ptry(J) != 0 {
            return 1;
        }
        let mut caught = false;
        if crate::except::js_try_run(J, || {
            js_loadstring(J, filename, source);
            js_endtry(J);
        }) {
            return 1;
        }
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_trystring(
    J: *mut js_State,
    idx: c_int,
    error: *const c_char,
) -> *const c_char {
    unsafe {
        let mut s: *const c_char = null();
        if js_ptry(J) != 0 {
            js_pop(J, 1);
            return error;
        }
        if crate::except::js_try_run(J, || {
            s = js_tostring(J, idx);
            js_endtry(J);
        }) {
            js_pop(J, 1);
            return error;
        }
        s
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_trynumber(J: *mut js_State, idx: c_int, error: f64) -> f64 {
    unsafe {
        let mut v: f64 = 0.0;
        if js_ptry(J) != 0 {
            js_pop(J, 1);
            return error;
        }
        if crate::except::js_try_run(J, || {
            v = js_tonumber(J, idx);
            js_endtry(J);
        }) {
            js_pop(J, 1);
            return error;
        }
        v
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_tryinteger(J: *mut js_State, idx: c_int, error: c_int) -> c_int {
    unsafe {
        let mut v: c_int = 0;
        if js_ptry(J) != 0 {
            js_pop(J, 1);
            return error;
        }
        if crate::except::js_try_run(J, || {
            v = js_tointeger(J, idx);
            js_endtry(J);
        }) {
            js_pop(J, 1);
            return error;
        }
        v
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_tryboolean(J: *mut js_State, idx: c_int, error: c_int) -> c_int {
    unsafe {
        let mut v: c_int = 0;
        if js_ptry(J) != 0 {
            js_pop(J, 1);
            return error;
        }
        if crate::except::js_try_run(J, || {
            v = js_toboolean(J, idx);
            js_endtry(J);
        }) {
            js_pop(J, 1);
            return error;
        }
        v
    }
}

unsafe fn js_loadstringx(
    J: *mut js_State,
    filename: *const c_char,
    source: *const c_char,
    iseval: c_int,
) {
    unsafe {
        if crate::except::js_try_run(J, || {
            let P: *mut js_Ast = jsP_parse(J, filename, source);
            let F: *mut js_Function = jsC_compilescript(
                J,
                P,
                if iseval != 0 {
                    (*J).strict
                } else {
                    (*J).default_strict
                },
            );
            jsP_freeparse(J);
            js_newscript(
                J,
                F,
                if iseval != 0 {
                    if (*J).strict != 0 {
                        (*J).E
                    } else {
                        null_mut()
                    }
                } else {
                    (*J).GE
                },
            );
            js_endtry(J);
        }) {
            jsP_freeparse(J);
            js_throw(J);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_loadeval(
    J: *mut js_State,
    filename: *const c_char,
    source: *const c_char,
) {
    unsafe {
        js_loadstringx(J, filename, source, 1);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_loadstring(
    J: *mut js_State,
    filename: *const c_char,
    source: *const c_char,
) {
    unsafe {
        js_loadstringx(J, filename, source, 0);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_dostring(J: *mut js_State, source: *const c_char) -> c_int {
    unsafe {
        if js_ptry(J) != 0 {
            js_report(J, c"exception stack overflow".as_ptr());
            js_pop(J, 1);
            return 1;
        }
        if crate::except::js_try_run(J, || {
            js_loadstring(J, c"[string]".as_ptr(), source);
            js_pushundefined(J);
            js_call(J, 0);
            js_pop(J, 1);
            js_endtry(J);
        }) {
            js_report(J, js_trystring(J, -1, c"Error".as_ptr()));
            js_pop(J, 1);
            return 1;
        }
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_atpanic(J: *mut js_State, panic: js_Panic) -> js_Panic {
    unsafe {
        let old: js_Panic = (*J).panic;
        (*J).panic = panic;
        old
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_report(J: *mut js_State, message: *const c_char) {
    unsafe {
        if (*J).report.is_some() {
            ((*J).report.unwrap())(J, message);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_setreport(J: *mut js_State, report: js_Report) {
    unsafe {
        (*J).report = report;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_setcontext(J: *mut js_State, uctx: *mut c_void) {
    unsafe {
        (*J).uctx = uctx;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_getcontext(J: *mut js_State) -> *mut c_void {
    unsafe { (*J).uctx }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_newstate(
    alloc: js_Alloc,
    actx: *mut c_void,
    flags: c_int,
) -> *mut js_State {
    unsafe {
        crate::except::install_panic_hook();

        let mut alloc = alloc;

        debug_assert!(core::mem::size_of::<js_Value>() == 16);
        debug_assert!(JS_VALUE_TYPEOFF == 15);

        if alloc.is_none() {
            alloc = Some(js_defaultalloc);
        }

        let J = (alloc.unwrap())(actx, null_mut(), core::mem::size_of::<js_State>() as c_int)
            as *mut js_State;
        if J.is_null() {
            return null_mut();
        }
        memset(J as *mut c_void, 0, core::mem::size_of::<js_State>());
        (*J).actx = actx;
        (*J).alloc = alloc;

        if flags & JS_STRICT != 0 {
            (*J).strict = 1;
            (*J).default_strict = 1;
        }

        (*J).trace[0].name = c"-top-".as_ptr();
        (*J).trace[0].file = c"native".as_ptr();
        (*J).trace[0].line = 0;

        (*J).report = Some(js_defaultreport);
        (*J).panic = Some(js_defaultpanic);

        (*J).stack = (alloc.unwrap())(
            actx,
            null_mut(),
            JS_STACKSIZE * core::mem::size_of::<js_Value>() as c_int,
        ) as *mut js_Value;
        if (*J).stack.is_null() {
            (alloc.unwrap())(actx, J as *mut c_void, 0);
            return null_mut();
        }

        (*J).gcmark = 1;
        (*J).nextref = 0;
        (*J).gcthresh = 0; /* reaches stability within ~ 2-5 GC cycles */

        let mut failed = false;
        if crate::except::js_try_run(J, || {
            (*J).R = jsV_newobject(J, JS_COBJECT, null_mut());
            (*J).G = jsV_newobject(J, JS_COBJECT, null_mut());
            (*J).E = jsR_newenvironment(J, (*J).G, null_mut());
            (*J).GE = (*J).E;

            jsB_init(J);

            js_endtry(J);
        }) {
            js_freestate(J);
            failed = true;
        }
        if failed {
            return null_mut();
        }
        J
    }
}
