// Translation of c_src/src/jsstate.c
#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals, dead_code)]

use crate::common::*;
use crate::jsbuiltin::jsB_init;
use crate::jscompile::jsC_compilescript;
use crate::jsparse::{jsP_freeparse, jsP_parse};
use crate::jsproperty::jsV_newobject;
use crate::jsrun::*;
use crate::jsvalue::js_newscript;
use crate::types::*;
use std::ffi::{c_char, c_int, c_void};
use std::ptr;

unsafe fn js_ptry(J: *mut js_State) -> c_int {
    unsafe {
        if (*J).trytop == JS_TRYLIMIT as c_int {
            let v = (*J).stack.offset((*J).top as isize);
            (*v).set_ty(JS_TLITSTR);
            (*v).u.litstr = c"exception stack overflow".as_ptr();
            (*J).top += 1;
            return 1;
        }
        0
    }
}

unsafe extern "C-unwind" fn js_defaultalloc(
    _actx: *mut c_void,
    ptr: *mut c_void,
    size: c_int,
) -> *mut c_void {
    unsafe {
        if size == 0 {
            free(ptr);
            return ptr::null_mut();
        }
        realloc(ptr, size as usize)
    }
}

unsafe extern "C-unwind" fn js_defaultreport(_J: *mut js_State, message: *const c_char) {
    unsafe {
        fputs(message, stderr);
        fputc(b'\n' as c_int, stderr);
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
        if js_try(J, || {
            js_loadstring(J, filename, source);
            js_endtry(J);
        })
        .is_err()
        {
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
        if js_ptry(J) != 0 {
            js_pop(J, 1);
            return error;
        }
        match js_try(J, || {
            let s = js_tostring(J, idx);
            js_endtry(J);
            s
        }) {
            Ok(s) => s,
            Err(()) => {
                js_pop(J, 1);
                error
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_trynumber(
    J: *mut js_State,
    idx: c_int,
    error: f64,
) -> f64 {
    unsafe {
        if js_ptry(J) != 0 {
            js_pop(J, 1);
            return error;
        }
        match js_try(J, || {
            let v = js_tonumber(J, idx);
            js_endtry(J);
            v
        }) {
            Ok(v) => v,
            Err(()) => {
                js_pop(J, 1);
                error
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_tryinteger(
    J: *mut js_State,
    idx: c_int,
    error: c_int,
) -> c_int {
    unsafe {
        if js_ptry(J) != 0 {
            js_pop(J, 1);
            return error;
        }
        match js_try(J, || {
            let v = js_tointeger(J, idx);
            js_endtry(J);
            v
        }) {
            Ok(v) => v,
            Err(()) => {
                js_pop(J, 1);
                error
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_tryboolean(
    J: *mut js_State,
    idx: c_int,
    error: c_int,
) -> c_int {
    unsafe {
        if js_ptry(J) != 0 {
            js_pop(J, 1);
            return error;
        }
        match js_try(J, || {
            let v = js_toboolean(J, idx);
            js_endtry(J);
            v
        }) {
            Ok(v) => v,
            Err(()) => {
                js_pop(J, 1);
                error
            }
        }
    }
}

unsafe fn js_loadstringx(
    J: *mut js_State,
    filename: *const c_char,
    source: *const c_char,
    iseval: c_int,
) {
    unsafe {
        if js_try(J, || {
            let P = jsP_parse(J, filename, source);
            let F = jsC_compilescript(
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
                    if (*J).strict != 0 { (*J).E } else { ptr::null_mut() }
                } else {
                    (*J).GE
                },
            );
            js_endtry(J);
        })
        .is_err()
        {
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
        if js_try(J, || {
            js_loadstring(J, c"[string]".as_ptr(), source);
            js_pushundefined(J);
            js_call(J, 0);
            js_pop(J, 1);
            js_endtry(J);
        })
        .is_err()
        {
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
        let old = (*J).panic;
        (*J).panic = panic;
        old
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_report(J: *mut js_State, message: *const c_char) {
    unsafe {
        if let Some(r) = (*J).report {
            r(J, message);
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
        crate::common::install_panic_hook();

        const _: () = assert!(std::mem::size_of::<js_Value>() == 16);
        const _: () = assert!(std::mem::offset_of!(js_ValueT, type_) == 15);

        let alloc = match alloc {
            Some(a) => Some(a),
            None => Some(js_defaultalloc as unsafe extern "C-unwind" fn(*mut c_void, *mut c_void, c_int) -> *mut c_void),
        };

        let J = (alloc.unwrap())(
            actx,
            ptr::null_mut(),
            std::mem::size_of::<js_State>() as c_int,
        ) as *mut js_State;
        if J.is_null() {
            return ptr::null_mut();
        }
        memset(J as *mut c_void, 0, std::mem::size_of::<js_State>());
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
            ptr::null_mut(),
            JS_STACKSIZE * std::mem::size_of::<js_Value>() as c_int,
        ) as *mut js_Value;
        if (*J).stack.is_null() {
            (alloc.unwrap())(actx, J as *mut c_void, 0);
            return ptr::null_mut();
        }

        (*J).gcmark = 1;
        (*J).nextref = 0;
        (*J).gcthresh = 0; /* reaches stability within ~ 2-5 GC cycles */

        if js_try(J, || {
            (*J).R = jsV_newobject(J, JS_COBJECT, ptr::null_mut());
            (*J).G = jsV_newobject(J, JS_COBJECT, ptr::null_mut());
            (*J).E = jsR_newenvironment(J, (*J).G, ptr::null_mut());
            (*J).GE = (*J).E;

            jsB_init(J);

            js_endtry(J);
        })
        .is_err()
        {
            js_freestate(J);
            return ptr::null_mut();
        }
        J
    }
}

use crate::jsgc::js_freestate;
