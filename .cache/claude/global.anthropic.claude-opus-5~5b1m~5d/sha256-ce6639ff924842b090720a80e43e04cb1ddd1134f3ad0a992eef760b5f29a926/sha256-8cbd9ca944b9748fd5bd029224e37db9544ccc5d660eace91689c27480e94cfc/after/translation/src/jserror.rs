//! Translation of jserror.c

use crate::jsbuiltin::{jsB_propf, jsB_props};
use crate::jsi::*;
use crate::jsproperty::jsV_newobject;
use crate::jsrun::*;
use crate::jsvalue::{js_concat, js_newcconstructor, js_newcfunction};

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
            if *name != 0 {
                snprintf(
                    buf.as_mut_ptr(),
                    256,
                    cs!("\n\tat %s (%s:%d)"),
                    name,
                    file,
                    line,
                );
            } else {
                snprintf(buf.as_mut_ptr(), 256, cs!("\n\tat %s:%d"), file, line);
            }
        } else {
            snprintf(buf.as_mut_ptr(), 256, cs!("\n\tat %s (%s)"), name, file);
        }
        js_pushstring(J, buf.as_ptr());
        if n < (*J).tracetop - skip {
            js_concat(J);
        }
        n -= 1;
    }
    1
}

unsafe extern "C" fn Ep_toString(J: *mut js_State) {
    let mut name: *const c_char = cs!("Error");
    let mut message: *const c_char = cs!("");

    if js_isobject(J, -1) == 0 {
        js_typeerror!(J, "not an object");
    }

    if js_hasproperty(J, 0, cs!("name")) != 0 {
        name = js_tostring(J, -1);
    }
    if js_hasproperty(J, 0, cs!("message")) != 0 {
        message = js_tostring(J, -1);
    }

    if *name == 0 {
        js_pushstring(J, message);
    } else if *message == 0 {
        js_pushstring(J, name);
    } else {
        js_pushstring(J, name);
        js_pushstring(J, cs!(": "));
        js_concat(J);
        js_pushstring(J, message);
        js_concat(J);
    }
}

unsafe extern "C" fn Ep_get_stack(J: *mut js_State) {
    Ep_toString(J);
    js_getproperty(J, 0, cs!("stackTrace"));
    js_concat(J);
}

unsafe fn jsB_ErrorX(J: *mut js_State, prototype: *mut js_Object) -> c_int {
    js_pushobject(J, jsV_newobject(J, JS_CERROR, prototype));
    if js_isdefined(J, 1) != 0 {
        js_pushstring(J, js_tostring(J, 1));
        js_defproperty(J, -2, cs!("message"), JS_DONTENUM);
    }
    if jsB_stacktrace(J, 1) != 0 {
        js_defproperty(J, -2, cs!("stackTrace"), JS_DONTENUM);
    }
    1
}

pub unsafe fn js_newerrorx(J: *mut js_State, message: *const c_char, prototype: *mut js_Object) {
    js_pushobject(J, jsV_newobject(J, JS_CERROR, prototype));
    js_pushstring(J, message);
    js_setproperty(J, -2, cs!("message"));
    if jsB_stacktrace(J, 0) != 0 {
        js_setproperty(J, -2, cs!("stackTrace"));
    }
}

macro_rules! derror {
    ($ctor:ident, $newname:ident, $strname:ident, $proto:ident) => {
        unsafe extern "C" fn $ctor(J: *mut js_State) {
            jsB_ErrorX(J, (*J).$proto);
        }
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $newname(J: *mut js_State, s: *const c_char) {
            js_newerrorx(J, s, (*J).$proto);
        }
        /// Non-variadic core of the matching `js_*error` function; the public
        /// variadic entry point lives in `varargs.rs`.
        pub unsafe fn $strname(J: *mut js_State, msg: *const c_char) -> ! {
            js_newerrorx(J, msg, (*J).$proto);
            js_throw(J)
        }
    };
}

derror!(jsB_Error, js_newerror, js_error_str, Error_prototype);
derror!(
    jsB_EvalError,
    js_newevalerror,
    js_evalerror_str,
    EvalError_prototype
);
derror!(
    jsB_RangeError,
    js_newrangeerror,
    js_rangeerror_str,
    RangeError_prototype
);
derror!(
    jsB_ReferenceError,
    js_newreferenceerror,
    js_referenceerror_str,
    ReferenceError_prototype
);
derror!(
    jsB_SyntaxError,
    js_newsyntaxerror,
    js_syntaxerror_str,
    SyntaxError_prototype
);
derror!(
    jsB_TypeError,
    js_newtypeerror,
    js_typeerror_str,
    TypeError_prototype
);
derror!(
    jsB_URIError,
    js_newurierror,
    js_urierror_str,
    URIError_prototype
);

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsB_initerror(J: *mut js_State) {
    js_pushobject(J, (*J).Error_prototype);
    {
        jsB_props(J, cs!("name"), cs!("Error"));
        jsB_propf(J, cs!("Error.prototype.toString"), Some(Ep_toString), 0);
        jsB_props(J, cs!("message"), cs!(""));

        js_newcfunction(J, Some(Ep_get_stack), cs!("stack"), 0);
        js_pushnull(J);
        js_defaccessor(
            J,
            -3,
            cs!("stack"),
            JS_READONLY | JS_DONTENUM | JS_DONTCONF,
        );
    }
    js_newcconstructor(J, Some(jsB_Error), Some(jsB_Error), cs!("Error"), 1);
    js_defglobal(J, cs!("Error"), JS_DONTENUM);

    macro_rules! ierror {
        ($ctor:ident, $name:expr, $proto:ident) => {
            js_pushobject(J, (*J).$proto);
            jsB_props(J, cs!("name"), cs!($name));
            js_newcconstructor(J, Some($ctor), Some($ctor), cs!($name), 1);
            js_defglobal(J, cs!($name), JS_DONTENUM);
        };
    }

    ierror!(jsB_EvalError, "EvalError", EvalError_prototype);
    ierror!(jsB_RangeError, "RangeError", RangeError_prototype);
    ierror!(
        jsB_ReferenceError,
        "ReferenceError",
        ReferenceError_prototype
    );
    ierror!(jsB_SyntaxError, "SyntaxError", SyntaxError_prototype);
    ierror!(jsB_TypeError, "TypeError", TypeError_prototype);
    ierror!(jsB_URIError, "URIError", URIError_prototype);
}
