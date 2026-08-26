//! Translated from c_src/src/jsstate.c
use crate::jsi::*;
use crate::prelude::*;

unsafe fn js_ptry(J: *mut js_State) -> c_int {
    if (*J).trytop as usize == JS_TRYLIMIT {
        (*(*J).stack.offset((*J).top as isize)).t.r#type = JS_TLITSTR;
        (*(*J).stack.offset((*J).top as isize)).u.litstr = c"exception stack overflow".as_ptr();
        (*J).top += 1;
        return 1;
    }
    0
}

unsafe extern "C" fn js_defaultalloc(
    actx: *mut c_void,
    ptr: *mut c_void,
    size: c_int,
) -> *mut c_void {
    if size == 0 {
        free(ptr);
        return null_mut();
    }
    realloc(ptr, size as usize)
}

unsafe extern "C" fn js_defaultreport(J: *mut js_State, message: *const c_char) {
    fputs(message, stderr);
    fputc('\n' as c_int, stderr);
}

unsafe extern "C" fn js_defaultpanic(J: *mut js_State) {
    js_report(J, c"uncaught exception".as_ptr());
    /* return to javascript to abort */
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_ploadstring(
    J: *mut js_State,
    filename: *const c_char,
    source: *const c_char,
) -> c_int {
    if js_ptry(J) != 0 {
        return 1;
    }
    if js_try!(J) {
        return 1;
    }
    js_loadstring(J, filename, source);
    js_endtry(J);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_trystring(
    J: *mut js_State,
    idx: c_int,
    error: *const c_char,
) -> *const c_char {
    let s: *const c_char;
    if js_ptry(J) != 0 {
        js_pop(J, 1);
        return error;
    }
    if js_try!(J) {
        js_pop(J, 1);
        return error;
    }
    s = js_tostring(J, idx);
    js_endtry(J);
    s
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_trynumber(J: *mut js_State, idx: c_int, error: f64) -> f64 {
    let v: f64;
    if js_ptry(J) != 0 {
        js_pop(J, 1);
        return error;
    }
    if js_try!(J) {
        js_pop(J, 1);
        return error;
    }
    v = js_tonumber(J, idx);
    js_endtry(J);
    v
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_tryinteger(J: *mut js_State, idx: c_int, error: c_int) -> c_int {
    let v: c_int;
    if js_ptry(J) != 0 {
        js_pop(J, 1);
        return error;
    }
    if js_try!(J) {
        js_pop(J, 1);
        return error;
    }
    v = js_tointeger(J, idx);
    js_endtry(J);
    v
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_tryboolean(J: *mut js_State, idx: c_int, error: c_int) -> c_int {
    let v: c_int;
    if js_ptry(J) != 0 {
        js_pop(J, 1);
        return error;
    }
    if js_try!(J) {
        js_pop(J, 1);
        return error;
    }
    v = js_toboolean(J, idx);
    js_endtry(J);
    v
}

unsafe fn js_loadstringx(
    J: *mut js_State,
    filename: *const c_char,
    source: *const c_char,
    iseval: c_int,
) {
    let P: *mut js_Ast;
    let F: *mut js_Function;

    if js_try!(J) {
        jsP_freeparse(J);
        js_throw(J);
    }

    P = jsP_parse(J, filename, source);
    F = jsC_compilescript(
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
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_loadeval(
    J: *mut js_State,
    filename: *const c_char,
    source: *const c_char,
) {
    js_loadstringx(J, filename, source, 1);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_loadstring(
    J: *mut js_State,
    filename: *const c_char,
    source: *const c_char,
) {
    js_loadstringx(J, filename, source, 0);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_dostring(J: *mut js_State, source: *const c_char) -> c_int {
    if js_ptry(J) != 0 {
        js_report(J, c"exception stack overflow".as_ptr());
        js_pop(J, 1);
        return 1;
    }
    if js_try!(J) {
        js_report(J, js_trystring(J, -1, c"Error".as_ptr()));
        js_pop(J, 1);
        return 1;
    }
    js_loadstring(J, c"[string]".as_ptr(), source);
    js_pushundefined(J);
    js_call(J, 0);
    js_pop(J, 1);
    js_endtry(J);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_atpanic(J: *mut js_State, panic: js_Panic) -> js_Panic {
    let old: js_Panic = (*J).panic;
    (*J).panic = panic;
    old
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_report(J: *mut js_State, message: *const c_char) {
    if let Some(report) = (*J).report {
        report(J, message);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_setreport(J: *mut js_State, report: js_Report) {
    (*J).report = report;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_setcontext(J: *mut js_State, uctx: *mut c_void) {
    (*J).uctx = uctx;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_getcontext(J: *mut js_State) -> *mut c_void {
    (*J).uctx
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_newstate(
    alloc: js_Alloc,
    actx: *mut c_void,
    flags: c_int,
) -> *mut js_State {
    let J: *mut js_State;
    let mut alloc = alloc;

    if alloc.is_none() {
        alloc = Some(js_defaultalloc);
    }

    J = (alloc.unwrap())(actx, null_mut(), std::mem::size_of::<js_State>() as c_int)
        as *mut js_State;
    if J.is_null() {
        return null_mut();
    }
    memset(J as *mut c_void, 0, std::mem::size_of::<js_State>());
    (*J).actx = actx;
    (*J).alloc = alloc;

    if (flags & JS_STRICT) != 0 {
        (*J).default_strict = 1;
        (*J).strict = (*J).default_strict;
    }

    (*J).trace[0].name = c"-top-".as_ptr();
    (*J).trace[0].file = c"native".as_ptr();
    (*J).trace[0].line = 0;

    (*J).report = Some(js_defaultreport);
    (*J).panic = Some(js_defaultpanic);

    (*J).stack = (alloc.unwrap())(
        actx,
        null_mut(),
        JS_STACKSIZE * std::mem::size_of::<js_Value>() as c_int,
    ) as *mut js_Value;
    if (*J).stack.is_null() {
        (alloc.unwrap())(actx, J as *mut c_void, 0);
        return null_mut();
    }

    (*J).gcmark = 1;
    (*J).nextref = 0;
    (*J).gcthresh = 0; /* reaches stability within ~ 2-5 GC cycles */

    if js_try!(J) {
        js_freestate(J);
        return null_mut();
    }

    (*J).R = jsV_newobject(J, JS_COBJECT, null_mut());
    (*J).G = jsV_newobject(J, JS_COBJECT, null_mut());
    (*J).E = jsR_newenvironment(J, (*J).G, null_mut());
    (*J).GE = (*J).E;

    jsB_init(J);

    js_endtry(J);
    J
}
