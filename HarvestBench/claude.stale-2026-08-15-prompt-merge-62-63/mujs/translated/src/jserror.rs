//! Translated from jserror.c — error objects and the error constructors.
//! Variadic entry points live in shim.c; here we provide the `rs_*` impls that
//! build the error object and throw, plus the non-variadic `js_new*error`.
#![allow(non_snake_case)]

use crate::cutil::*;
use crate::jsrun::*;
use crate::types::*;
use std::os::raw::{c_char, c_int, c_void};

macro_rules! cstr {
    ($s:literal) => {
        concat!($s, "\0").as_ptr() as *const c_char
    };
}

// Re-export the variadic public/internal error functions from the shim so the
// rest of the crate can call e.g. `crate::jserror::js_typeerror(J, "...")`.
pub use crate::shim::{
    js_error, js_evalerror, js_rangeerror, js_referenceerror, js_syntaxerror, js_typeerror,
    js_urierror, jsC_error, jsP_error_shim, jsP_warning_shim, jsY_error_shim,
};

unsafe fn jsB_stacktrace(J: *mut js_State, skip: c_int) -> c_int {
    let mut buf: [c_char; 256] = [0; 256];
    let mut n = (*J).tracetop - skip;
    if n <= 0 {
        return 0;
    }
    while n > 0 {
        let name = (*J).trace[n as usize].name;
        let file = (*J).trace[n as usize].file;
        let line = (*J).trace[n as usize].line;
        if line > 0 {
            if *name.add(0) != 0 {
                libc::snprintf(buf.as_mut_ptr(), 256, cstr!("\n\tat %s (%s:%d)"), name, file, line);
            } else {
                libc::snprintf(buf.as_mut_ptr(), 256, cstr!("\n\tat %s:%d"), file, line);
            }
        } else {
            libc::snprintf(buf.as_mut_ptr(), 256, cstr!("\n\tat %s (%s)"), name, file);
        }
        js_pushstring(J, buf.as_ptr());
        if n < (*J).tracetop - skip {
            crate::jsvalue::js_concat(J);
        }
        n -= 1;
    }
    1
}

unsafe extern "C-unwind" fn Ep_toString(J: *mut js_State) {
    let mut name: *const c_char = cstr!("Error");
    let mut message: *const c_char = cstr!("");

    if js_isobject(J, -1) == 0 {
        js_typeerror(J, cstr!("not an object"));
    }

    if js_hasproperty(J, 0, cstr!("name")) != 0 {
        name = js_tostring(J, -1);
    }
    if js_hasproperty(J, 0, cstr!("message")) != 0 {
        message = js_tostring(J, -1);
    }

    if *name.add(0) == 0 {
        js_pushstring(J, message);
    } else if *message.add(0) == 0 {
        js_pushstring(J, name);
    } else {
        js_pushstring(J, name);
        js_pushstring(J, cstr!(": "));
        crate::jsvalue::js_concat(J);
        js_pushstring(J, message);
        crate::jsvalue::js_concat(J);
    }
}

unsafe extern "C-unwind" fn Ep_get_stack(J: *mut js_State) {
    Ep_toString(J);
    js_getproperty(J, 0, cstr!("stackTrace"));
    crate::jsvalue::js_concat(J);
}

unsafe fn jsB_ErrorX(J: *mut js_State, prototype: *mut js_Object) -> c_int {
    js_pushobject(J, crate::jsproperty::jsV_newobject(J, JS_CERROR, prototype));
    if js_isdefined(J, 1) != 0 {
        js_pushstring(J, js_tostring(J, 1));
        js_defproperty(J, -2, cstr!("message"), JS_DONTENUM);
    }
    if jsB_stacktrace(J, 1) != 0 {
        js_defproperty(J, -2, cstr!("stackTrace"), JS_DONTENUM);
    }
    1
}

unsafe fn js_newerrorx(J: *mut js_State, message: *const c_char, prototype: *mut js_Object) {
    js_pushobject(J, crate::jsproperty::jsV_newobject(J, JS_CERROR, prototype));
    js_pushstring(J, message);
    js_setproperty(J, -2, cstr!("message"));
    if jsB_stacktrace(J, 0) != 0 {
        js_setproperty(J, -2, cstr!("stackTrace"));
    }
}

/* DERROR expansion: for each (name, Name) we get:
 *   static jsB_Name(J)               -> jsB_ErrorX(J, J->Name_prototype)
 *   void js_newname(J, s)            -> js_newerrorx(J, s, J->Name_prototype)  [public]
 *   void js_name(J, fmt, ...)        -> variadic (in shim.c), rs impl throws
 */

macro_rules! derror_impl {
    ($rs_fn:ident, $new_fn:ident, $jsb_fn:ident, $proto:ident) => {
        // rs_* : called by the shim after formatting; builds error and throws.
        #[no_mangle]
        pub unsafe extern "C-unwind" fn $rs_fn(J: *mut js_State, msg: *const c_char) {
            js_newerrorx(J, msg, (*J).$proto);
            js_throw(J);
        }
        // public non-variadic constructor js_new*error
        #[no_mangle]
        pub unsafe extern "C-unwind" fn $new_fn(J: *mut js_State, s: *const c_char) {
            js_newerrorx(J, s, (*J).$proto);
        }
        // native constructor callback
        unsafe extern "C-unwind" fn $jsb_fn(J: *mut js_State) {
            jsB_ErrorX(J, (*J).$proto);
        }
    };
}

derror_impl!(rs_js_error, js_newerror, jsB_Error, Error_prototype);
derror_impl!(rs_js_evalerror, js_newevalerror, jsB_EvalError, EvalError_prototype);
derror_impl!(rs_js_rangeerror, js_newrangeerror, jsB_RangeError, RangeError_prototype);
derror_impl!(rs_js_referenceerror, js_newreferenceerror, jsB_ReferenceError, ReferenceError_prototype);
derror_impl!(rs_js_syntaxerror, js_newsyntaxerror, jsB_SyntaxError, SyntaxError_prototype);
derror_impl!(rs_js_typeerror, js_newtypeerror, jsB_TypeError, TypeError_prototype);
derror_impl!(rs_js_urierror, js_newurierror, jsB_URIError, URIError_prototype);

#[no_mangle]
pub unsafe extern "C-unwind" fn jsB_initerror(J: *mut js_State) {
    js_pushobject(J, (*J).Error_prototype);
    {
        crate::jsbuiltin::jsB_props(J, cstr!("name"), cstr!("Error"));
        crate::jsbuiltin::jsB_propf(J, cstr!("Error.prototype.toString"), Some(Ep_toString), 0);
        crate::jsbuiltin::jsB_props(J, cstr!("message"), cstr!(""));

        crate::jsvalue::js_newcfunction(J, Some(Ep_get_stack), cstr!("stack"), 0);
        js_pushnull(J);
        js_defaccessor(J, -3, cstr!("stack"), JS_READONLY | JS_DONTENUM | JS_DONTCONF);
    }
    crate::jsvalue::js_newcconstructor(J, Some(jsB_Error), Some(jsB_Error), cstr!("Error"), 1);
    js_defglobal(J, cstr!("Error"), JS_DONTENUM);

    macro_rules! ierror {
        ($proto:ident, $jsb:ident, $name:literal) => {
            js_pushobject(J, (*J).$proto);
            crate::jsbuiltin::jsB_props(J, cstr!("name"), cstr!($name));
            crate::jsvalue::js_newcconstructor(J, Some($jsb), Some($jsb), cstr!($name), 1);
            js_defglobal(J, cstr!($name), JS_DONTENUM);
        };
    }

    ierror!(EvalError_prototype, jsB_EvalError, "EvalError");
    ierror!(RangeError_prototype, jsB_RangeError, "RangeError");
    ierror!(ReferenceError_prototype, jsB_ReferenceError, "ReferenceError");
    ierror!(SyntaxError_prototype, jsB_SyntaxError, "SyntaxError");
    ierror!(TypeError_prototype, jsB_TypeError, "TypeError");
    ierror!(URIError_prototype, jsB_URIError, "URIError");
}
