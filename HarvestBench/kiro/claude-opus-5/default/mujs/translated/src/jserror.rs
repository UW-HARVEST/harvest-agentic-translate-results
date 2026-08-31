// Translation of c_src/src/jserror.c
#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals, dead_code)]

use crate::common::*;
use crate::jsbuiltin::{jsB_propf, jsB_props};
use crate::jsproperty::jsV_newobject;
use crate::jsrun::*;
use crate::jsvalue::*;
use crate::types::*;
use crate::js_typeerror;
use std::ffi::{c_char, c_int};

unsafe fn jsB_stacktrace(J: *mut js_State, skip: c_int) -> c_int {
    unsafe {
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
                if *name != 0 {
                    snprintf(
                        buf.as_mut_ptr(),
                        256,
                        c"\n\tat %s (%s:%d)".as_ptr(),
                        name,
                        file,
                        line,
                    );
                } else {
                    snprintf(buf.as_mut_ptr(), 256, c"\n\tat %s:%d".as_ptr(), file, line);
                }
            } else {
                snprintf(buf.as_mut_ptr(), 256, c"\n\tat %s (%s)".as_ptr(), name, file);
            }
            js_pushstring(J, buf.as_ptr());
            if n < (*J).tracetop - skip {
                js_concat(J);
            }
            n -= 1;
        }
        1
    }
}

unsafe extern "C-unwind" fn Ep_toString(J: *mut js_State) {
    unsafe {
        let mut name: *const c_char = c"Error".as_ptr();
        let mut message: *const c_char = c"".as_ptr();

        if js_isobject(J, -1) == 0 {
            js_typeerror!(J, c"not an object");
        }

        if js_hasproperty(J, 0, c"name".as_ptr()) != 0 {
            name = js_tostring(J, -1);
        }
        if js_hasproperty(J, 0, c"message".as_ptr()) != 0 {
            message = js_tostring(J, -1);
        }

        if *name == 0 {
            js_pushstring(J, message);
        } else if *message == 0 {
            js_pushstring(J, name);
        } else {
            js_pushstring(J, name);
            js_pushstring(J, c": ".as_ptr());
            js_concat(J);
            js_pushstring(J, message);
            js_concat(J);
        }
    }
}

unsafe extern "C-unwind" fn Ep_get_stack(J: *mut js_State) {
    unsafe {
        Ep_toString(J);
        js_getproperty(J, 0, c"stackTrace".as_ptr());
        js_concat(J);
    }
}

unsafe fn jsB_ErrorX(J: *mut js_State, prototype: *mut js_Object) -> c_int {
    unsafe {
        js_pushobject(J, jsV_newobject(J, JS_CERROR, prototype));
        if js_isdefined(J, 1) != 0 {
            js_pushstring(J, js_tostring(J, 1));
            js_defproperty(J, -2, c"message".as_ptr(), JS_DONTENUM);
        }
        if jsB_stacktrace(J, 1) != 0 {
            js_defproperty(J, -2, c"stackTrace".as_ptr(), JS_DONTENUM);
        }
        1
    }
}

unsafe fn js_newerrorx(J: *mut js_State, message: *const c_char, prototype: *mut js_Object) {
    unsafe {
        js_pushobject(J, jsV_newobject(J, JS_CERROR, prototype));
        js_pushstring(J, message);
        js_setproperty(J, -2, c"message".as_ptr());
        if jsB_stacktrace(J, 0) != 0 {
            js_setproperty(J, -2, c"stackTrace".as_ptr());
        }
    }
}

/*
 * The C original defines these with a macro; the variadic entry points take
 * their extra arguments in the same registers the C ABI would use for
 * `...` arguments (4 integer + 4 floating point), and forward them to
 * snprintf, which reproduces vsnprintf's behaviour exactly.
 */
macro_rules! derror {
    ($jsB:ident, $newfn:ident, $throwfn:ident, $rtfn:ident, $proto:ident) => {
        pub unsafe extern "C-unwind" fn $jsB(J: *mut js_State) {
            unsafe {
                jsB_ErrorX(J, (*J).$proto);
            }
        }

        #[unsafe(no_mangle)]
        pub unsafe extern "C-unwind" fn $newfn(J: *mut js_State, s: *const c_char) {
            unsafe {
                js_newerrorx(J, s, (*J).$proto);
            }
        }

        /// Internal entry point: `msg` is already formatted.
        pub unsafe fn $rtfn(J: *mut js_State, msg: *const c_char) -> ! {
            unsafe {
                js_newerrorx(J, msg, (*J).$proto);
                js_throw(J)
            }
        }

        #[unsafe(no_mangle)]
        pub unsafe extern "C-unwind" fn $throwfn(
            J: *mut js_State,
            fmt: *const c_char,
            a1: usize,
            a2: usize,
            a3: usize,
            a4: usize,
            d1: f64,
            d2: f64,
            d3: f64,
            d4: f64,
        ) -> ! {
            unsafe {
                let mut buf: [c_char; 256] = [0; 256];
                snprintf(buf.as_mut_ptr(), 256, fmt, a1, a2, a3, a4, d1, d2, d3, d4);
                js_newerrorx(J, buf.as_ptr(), (*J).$proto);
                js_throw(J)
            }
        }
    };
}

derror!(jsB_Error, js_newerror, js_error, rt_error, Error_prototype);
derror!(
    jsB_EvalError,
    js_newevalerror,
    js_evalerror,
    rt_evalerror,
    EvalError_prototype
);
derror!(
    jsB_RangeError,
    js_newrangeerror,
    js_rangeerror,
    rt_rangeerror,
    RangeError_prototype
);
derror!(
    jsB_ReferenceError,
    js_newreferenceerror,
    js_referenceerror,
    rt_referenceerror,
    ReferenceError_prototype
);
derror!(
    jsB_SyntaxError,
    js_newsyntaxerror,
    js_syntaxerror,
    rt_syntaxerror,
    SyntaxError_prototype
);
derror!(
    jsB_TypeError,
    js_newtypeerror,
    js_typeerror,
    rt_typeerror,
    TypeError_prototype
);
derror!(
    jsB_URIError,
    js_newurierror,
    js_urierror,
    rt_urierror,
    URIError_prototype
);

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsB_initerror(J: *mut js_State) {
    unsafe {
        js_pushobject(J, (*J).Error_prototype);
        {
            jsB_props(J, c"name".as_ptr(), c"Error".as_ptr());
            jsB_propf(
                J,
                c"Error.prototype.toString".as_ptr(),
                Some(Ep_toString),
                0,
            );
            jsB_props(J, c"message".as_ptr(), c"".as_ptr());

            js_newcfunction(J, Some(Ep_get_stack), c"stack".as_ptr(), 0);
            js_pushnull(J);
            js_defaccessor(
                J,
                -3,
                c"stack".as_ptr(),
                JS_READONLY | JS_DONTENUM | JS_DONTCONF,
            );
        }
        js_newcconstructor(
            J,
            Some(jsB_Error),
            Some(jsB_Error),
            c"Error".as_ptr(),
            1,
        );
        js_defglobal(J, c"Error".as_ptr(), JS_DONTENUM);

        macro_rules! ierror {
            ($proto:ident, $ctor:ident, $name:literal) => {
                js_pushobject(J, (*J).$proto);
                jsB_props(J, c"name".as_ptr(), $name.as_ptr());
                js_newcconstructor(J, Some($ctor), Some($ctor), $name.as_ptr(), 1);
                js_defglobal(J, $name.as_ptr(), JS_DONTENUM);
            };
        }

        ierror!(EvalError_prototype, jsB_EvalError, c"EvalError");
        ierror!(RangeError_prototype, jsB_RangeError, c"RangeError");
        ierror!(
            ReferenceError_prototype,
            jsB_ReferenceError,
            c"ReferenceError"
        );
        ierror!(SyntaxError_prototype, jsB_SyntaxError, c"SyntaxError");
        ierror!(TypeError_prototype, jsB_TypeError, c"TypeError");
        ierror!(URIError_prototype, jsB_URIError, c"URIError");
    }
}
