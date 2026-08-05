//! Translated from jsstate.c — state lifecycle, protected entry points.
#![allow(non_snake_case, non_upper_case_globals)]

use crate::jsrun::*;
use crate::types::*;
use std::os::raw::{c_char, c_int, c_void};

macro_rules! cstr {
    ($s:literal) => {
        concat!($s, "\0").as_ptr() as *const c_char
    };
}

unsafe fn js_ptry(J: *mut js_State) -> c_int {
    if (*J).trytop == JS_TRYLIMIT as c_int {
        let top = (*J).top as usize;
        (*(*J).stack.add(top)).set_type(JS_TLITSTR);
        (*(*J).stack.add(top)).u.litstr = cstr!("exception stack overflow");
        (*J).top += 1;
        return 1;
    }
    0
}

unsafe extern "C-unwind" fn js_defaultalloc(_actx: *mut c_void, ptr: *mut c_void, size: c_int) -> *mut c_void {
    if size == 0 {
        libc::free(ptr);
        return std::ptr::null_mut();
    }
    libc::realloc(ptr, size as usize)
}

unsafe extern "C-unwind" fn js_defaultreport(_J: *mut js_State, message: *const c_char) {
    libc::fputs(message, unsafe_stderr());
    libc::fputc('\n' as c_int, unsafe_stderr());
}

#[inline]
unsafe fn unsafe_stderr() -> *mut libc::FILE {
    extern "C" {
        // stderr is a macro in C; libc exposes it via a static.
        static mut stderr: *mut libc::FILE;
    }
    stderr
}

unsafe extern "C-unwind" fn js_defaultpanic(J: *mut js_State) {
    js_report(J, cstr!("uncaught exception"));
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_ploadstring(J: *mut js_State, filename: *const c_char, source: *const c_char) -> c_int {
    if js_ptry(J) != 0 {
        return 1;
    }
    let caught = protect(J, || {
        js_loadstring(J, filename, source);
    });
    if caught {
        return 1;
    }
    js_endtry(J);
    0
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_trystring(J: *mut js_State, idx: c_int, error: *const c_char) -> *const c_char {
    if js_ptry(J) != 0 {
        js_pop(J, 1);
        return error;
    }
    let mut s: *const c_char = std::ptr::null();
    let sp = std::ptr::addr_of_mut!(s);
    let caught = protect(J, || {
        *sp = js_tostring(J, idx);
    });
    if caught {
        js_pop(J, 1);
        return error;
    }
    js_endtry(J);
    s
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_trynumber(J: *mut js_State, idx: c_int, error: f64) -> f64 {
    if js_ptry(J) != 0 {
        js_pop(J, 1);
        return error;
    }
    let mut v: f64 = 0.0;
    let vp = std::ptr::addr_of_mut!(v);
    let caught = protect(J, || {
        *vp = js_tonumber(J, idx);
    });
    if caught {
        js_pop(J, 1);
        return error;
    }
    js_endtry(J);
    v
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_tryinteger(J: *mut js_State, idx: c_int, error: c_int) -> c_int {
    if js_ptry(J) != 0 {
        js_pop(J, 1);
        return error;
    }
    let mut v: c_int = 0;
    let vp = std::ptr::addr_of_mut!(v);
    let caught = protect(J, || {
        *vp = js_tointeger(J, idx);
    });
    if caught {
        js_pop(J, 1);
        return error;
    }
    js_endtry(J);
    v
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_tryboolean(J: *mut js_State, idx: c_int, error: c_int) -> c_int {
    if js_ptry(J) != 0 {
        js_pop(J, 1);
        return error;
    }
    let mut v: c_int = 0;
    let vp = std::ptr::addr_of_mut!(v);
    let caught = protect(J, || {
        *vp = js_toboolean(J, idx);
    });
    if caught {
        js_pop(J, 1);
        return error;
    }
    js_endtry(J);
    v
}

unsafe fn js_loadstringx(J: *mut js_State, filename: *const c_char, source: *const c_char, iseval: c_int) {
    let caught = protect(J, || {
        let P = crate::jsparse::jsP_parse(J, filename, source);
        let F = crate::jscompile::jsC_compilescript(J, P, if iseval != 0 { (*J).strict } else { (*J).default_strict });
        crate::jsparse::jsP_freeparse(J);
        let scope = if iseval != 0 {
            if (*J).strict != 0 {
                (*J).E
            } else {
                std::ptr::null_mut()
            }
        } else {
            (*J).GE
        };
        crate::jsvalue::js_newscript(J, F, scope);
    });
    if caught {
        crate::jsparse::jsP_freeparse(J);
        js_throw(J);
    }
    js_endtry(J);
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_loadeval(J: *mut js_State, filename: *const c_char, source: *const c_char) {
    js_loadstringx(J, filename, source, 1);
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_loadstring(J: *mut js_State, filename: *const c_char, source: *const c_char) {
    js_loadstringx(J, filename, source, 0);
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_dostring(J: *mut js_State, source: *const c_char) -> c_int {
    if js_ptry(J) != 0 {
        js_report(J, cstr!("exception stack overflow"));
        js_pop(J, 1);
        return 1;
    }
    let caught = protect(J, || {
        js_loadstring(J, cstr!("[string]"), source);
        js_pushundefined(J);
        js_call(J, 0);
        js_pop(J, 1);
    });
    if caught {
        js_report(J, js_trystring(J, -1, cstr!("Error")));
        js_pop(J, 1);
        return 1;
    }
    js_endtry(J);
    0
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_atpanic(J: *mut js_State, panic: js_Panic) -> js_Panic {
    let old = (*J).panic;
    (*J).panic = panic;
    old
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_report(J: *mut js_State, message: *const c_char) {
    if let Some(r) = (*J).report {
        r(J, message);
    }
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_setreport(J: *mut js_State, report: js_Report) {
    (*J).report = report;
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_setcontext(J: *mut js_State, uctx: *mut c_void) {
    (*J).uctx = uctx;
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_getcontext(J: *mut js_State) -> *mut c_void {
    (*J).uctx
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_newstate(alloc: js_Alloc, actx: *mut c_void, flags: c_int) -> *mut js_State {
    crate::except::install_panic_hook();

    assert!(std::mem::size_of::<js_Value>() == 16);
    assert!(std::mem::offset_of!(js_Value, t.type_) == 15);

    let alloc = if alloc.is_none() { Some(js_defaultalloc as unsafe extern "C-unwind" fn(*mut c_void, *mut c_void, c_int) -> *mut c_void) } else { alloc };

    let J = (alloc.unwrap())(actx, std::ptr::null_mut(), std::mem::size_of::<js_State>() as c_int) as *mut js_State;
    if J.is_null() {
        return std::ptr::null_mut();
    }
    libc::memset(J as *mut c_void, 0, std::mem::size_of::<js_State>());
    (*J).actx = actx;
    (*J).alloc = alloc;

    if flags & JS_STRICT != 0 {
        (*J).default_strict = 1;
        (*J).strict = 1;
    }

    (*J).trace[0].name = cstr!("-top-");
    (*J).trace[0].file = cstr!("native");
    (*J).trace[0].line = 0;

    (*J).report = Some(js_defaultreport);
    (*J).panic = Some(js_defaultpanic);

    (*J).stack = (alloc.unwrap())(actx, std::ptr::null_mut(), (JS_STACKSIZE * std::mem::size_of::<js_Value>()) as c_int) as *mut js_Value;
    if (*J).stack.is_null() {
        (alloc.unwrap())(actx, J as *mut c_void, 0);
        return std::ptr::null_mut();
    }

    (*J).gcmark = 1;
    (*J).nextref = 0;
    (*J).gcthresh = 0;

    let caught = protect(J, || {
        (*J).R = crate::jsproperty::jsV_newobject(J, JS_COBJECT, std::ptr::null_mut());
        (*J).G = crate::jsproperty::jsV_newobject(J, JS_COBJECT, std::ptr::null_mut());
        (*J).E = jsR_newenvironment(J, (*J).G, std::ptr::null_mut());
        (*J).GE = (*J).E;
        crate::jsbuiltin::jsB_init(J);
    });
    if caught {
        crate::jsgc::js_freestate(J);
        return std::ptr::null_mut();
    }
    js_endtry(J);
    J
}
