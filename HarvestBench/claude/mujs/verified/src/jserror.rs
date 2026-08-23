//! Translated from c_src/src/jserror.c
use crate::jsi::*;
use crate::prelude::*;

unsafe fn jsB_stacktrace(J: *mut js_State, skip: c_int) -> c_int {
    let mut buf: [c_char; 256] = [0; 256];
    let mut n: c_int = (*J).tracetop - skip;
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

unsafe extern "C" fn Ep_toString(J: *mut js_State) {
    let mut name: *const c_char = c"Error".as_ptr();
    let mut message: *const c_char = c"".as_ptr();

    if js_isobject(J, -1) == 0 {
        js_typeerror!(J, c"not an object".as_ptr());
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

unsafe extern "C" fn Ep_get_stack(J: *mut js_State) {
    Ep_toString(J);
    js_getproperty(J, 0, c"stackTrace".as_ptr());
    js_concat(J);
}

unsafe fn jsB_ErrorX(J: *mut js_State, prototype: *mut js_Object) -> c_int {
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

unsafe fn js_newerrorx(J: *mut js_State, message: *const c_char, prototype: *mut js_Object) {
    js_pushobject(J, jsV_newobject(J, JS_CERROR, prototype));
    js_pushstring(J, message);
    js_setproperty(J, -2, c"message".as_ptr());
    if jsB_stacktrace(J, 0) != 0 {
        js_setproperty(J, -2, c"stackTrace".as_ptr());
    }
}

macro_rules! DERROR {
    ($jsB:ident, $new:ident, $str:ident, $va:ident, $proto:ident) => {
        unsafe extern "C" fn $jsB(J: *mut js_State) {
            jsB_ErrorX(J, (*J).$proto);
        }
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $new(J: *mut js_State, s: *const c_char) {
            js_newerrorx(J, s, (*J).$proto);
        }
        /// `js_xxxerror(J, message)` -- the non-variadic core.
        pub unsafe fn $str(J: *mut js_State, buf: *const c_char) -> ! {
            js_newerrorx(J, buf, (*J).$proto);
            js_throw(J)
        }
        /// Target of the naked variadic trampoline in lib.rs (referenced with a
        /// `sym` operand, so it needs no stable linker name).
        pub unsafe extern "C" fn $va(
            J: *mut js_State,
            fmt: *const c_char,
            ap: *mut VaListTag,
        ) -> ! {
            let mut buf: [c_char; 256] = [0; 256];
            vsnprintf(buf.as_mut_ptr(), 256, fmt, ap);
            $str(J, buf.as_ptr())
        }
    };
}

DERROR!(jsB_Error, js_newerror, js_error_str, js_error_va, Error_prototype);
DERROR!(
    jsB_EvalError,
    js_newevalerror,
    js_evalerror_str,
    js_evalerror_va,
    EvalError_prototype
);
DERROR!(
    jsB_RangeError,
    js_newrangeerror,
    js_rangeerror_str,
    js_rangeerror_va,
    RangeError_prototype
);
DERROR!(
    jsB_ReferenceError,
    js_newreferenceerror,
    js_referenceerror_str,
    js_referenceerror_va,
    ReferenceError_prototype
);
DERROR!(
    jsB_SyntaxError,
    js_newsyntaxerror,
    js_syntaxerror_str,
    js_syntaxerror_va,
    SyntaxError_prototype
);
DERROR!(
    jsB_TypeError,
    js_newtypeerror,
    js_typeerror_str,
    js_typeerror_va,
    TypeError_prototype
);
DERROR!(
    jsB_URIError,
    js_newurierror,
    js_urierror_str,
    js_urierror_va,
    URIError_prototype
);

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsB_initerror(J: *mut js_State) {
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

    macro_rules! IERROR {
        ($proto:ident, $jsB:ident, $name:expr) => {
            js_pushobject(J, (*J).$proto);
            jsB_props(J, c"name".as_ptr(), $name);
            js_newcconstructor(J, Some($jsB), Some($jsB), $name, 1);
            js_defglobal(J, $name, JS_DONTENUM);
        };
    }

    IERROR!(EvalError_prototype, jsB_EvalError, c"EvalError".as_ptr());
    IERROR!(RangeError_prototype, jsB_RangeError, c"RangeError".as_ptr());
    IERROR!(
        ReferenceError_prototype,
        jsB_ReferenceError,
        c"ReferenceError".as_ptr()
    );
    IERROR!(
        SyntaxError_prototype,
        jsB_SyntaxError,
        c"SyntaxError".as_ptr()
    );
    IERROR!(TypeError_prototype, jsB_TypeError, c"TypeError".as_ptr());
    IERROR!(URIError_prototype, jsB_URIError, c"URIError".as_ptr());
}
