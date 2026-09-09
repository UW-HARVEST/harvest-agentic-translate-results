//! Translation of src/jserror.c

use crate::jsi::*;

use crate::jsbuiltin::{jsB_propf, jsB_props};
use crate::jsproperty::jsV_newobject;
use crate::jsrun::{
    js_defaccessor, js_defglobal, js_defproperty, js_getproperty, js_hasproperty, js_isdefined,
    js_isobject, js_pushnull, js_pushobject, js_pushstring, js_setproperty, js_tostring,
};
use crate::jsvalue::{js_concat, js_newcconstructor, js_newcfunction};

/* #define QQ(X) #X ; #define Q(X) QQ(X) — the stringifications are inlined as
   literals below. */

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
                        core::mem::size_of::<[c_char; 256]>() as size_t,
                        c"\n\tat %s (%s:%d)".as_ptr(),
                        name,
                        file,
                        line,
                    );
                } else {
                    snprintf(
                        buf.as_mut_ptr(),
                        core::mem::size_of::<[c_char; 256]>() as size_t,
                        c"\n\tat %s:%d".as_ptr(),
                        file,
                        line,
                    );
                }
            } else {
                snprintf(
                    buf.as_mut_ptr(),
                    core::mem::size_of::<[c_char; 256]>() as size_t,
                    c"\n\tat %s (%s)".as_ptr(),
                    name,
                    file,
                );
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

/* file-static in the C code */
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

/* Helper used by the crate-wide error macros. */
pub unsafe fn js_error_throw(J: *mut js_State, msg: *const c_char, prototype: *mut js_Object) -> ! {
    unsafe {
        js_newerrorx(J, msg, prototype);
        crate::jsrun::js_throw(J)
    }
}

/*
	The C source uses a DERROR(name, Name) macro that expands, for each error
	class, to:
		static void jsB_<Name>(js_State *J)         -> a js_CFunction
		void js_new<name>(js_State *J, const char *s) -> EXPORTED
		void js_<name>(js_State *J, const char *fmt, ...) -> EXPORTED, VARIADIC

	The variadic entry points live in vararg.rs as naked trampolines that call
	the `*_v` implementations defined here. The non-variadic `js_new<name>`
	functions are exported below. The `jsB_<Name>` callbacks are static.
*/

macro_rules! derror {
    ($JB:ident, $NEW:ident, $V:ident, $PROTO:ident) => {
        unsafe extern "C-unwind" fn $JB(J: *mut js_State) {
            unsafe {
                jsB_ErrorX(J, (*J).$PROTO);
            }
        }

        #[unsafe(no_mangle)]
        pub unsafe extern "C-unwind" fn $NEW(J: *mut js_State, s: *const c_char) {
            unsafe {
                js_newerrorx(J, s, (*J).$PROTO);
            }
        }

        pub unsafe extern "C-unwind" fn $V(
            J: *mut js_State,
            fmt: *const c_char,
            ap: *mut c_void,
        ) -> ! {
            unsafe {
                let mut buf: [c_char; 256] = [0; 256];
                crate::vararg::vsnprintf(buf.as_mut_ptr(), 256, fmt, ap);
                js_newerrorx(J, buf.as_ptr(), (*J).$PROTO);
                crate::jsrun::js_throw(J)
            }
        }
    };
}

derror!(jsB_Error, js_newerror, js_error_v, Error_prototype);
derror!(jsB_EvalError, js_newevalerror, js_evalerror_v, EvalError_prototype);
derror!(jsB_RangeError, js_newrangeerror, js_rangeerror_v, RangeError_prototype);
derror!(
    jsB_ReferenceError,
    js_newreferenceerror,
    js_referenceerror_v,
    ReferenceError_prototype
);
derror!(jsB_SyntaxError, js_newsyntaxerror, js_syntaxerror_v, SyntaxError_prototype);
derror!(jsB_TypeError, js_newtypeerror, js_typeerror_v, TypeError_prototype);
derror!(jsB_URIError, js_newurierror, js_urierror_v, URIError_prototype);

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsB_initerror(J: *mut js_State) {
    unsafe {
        js_pushobject(J, (*J).Error_prototype);
        {
            jsB_props(J, c"name".as_ptr(), c"Error".as_ptr());
            jsB_propf(J, c"Error.prototype.toString".as_ptr(), Some(Ep_toString), 0);
            jsB_props(J, c"message".as_ptr(), c"".as_ptr());

            js_newcfunction(J, Some(Ep_get_stack), c"stack".as_ptr(), 0);
            js_pushnull(J);
            js_defaccessor(J, -3, c"stack".as_ptr(), JS_READONLY | JS_DONTENUM | JS_DONTCONF);
        }
        js_newcconstructor(J, Some(jsB_Error), Some(jsB_Error), c"Error".as_ptr(), 1);
        js_defglobal(J, c"Error".as_ptr(), JS_DONTENUM);

        /* IERROR(NAME): Q(NAME) stringifies to the literal class name. */
        js_pushobject(J, (*J).EvalError_prototype);
        jsB_props(J, c"name".as_ptr(), c"EvalError".as_ptr());
        js_newcconstructor(J, Some(jsB_EvalError), Some(jsB_EvalError), c"EvalError".as_ptr(), 1);
        js_defglobal(J, c"EvalError".as_ptr(), JS_DONTENUM);

        js_pushobject(J, (*J).RangeError_prototype);
        jsB_props(J, c"name".as_ptr(), c"RangeError".as_ptr());
        js_newcconstructor(J, Some(jsB_RangeError), Some(jsB_RangeError), c"RangeError".as_ptr(), 1);
        js_defglobal(J, c"RangeError".as_ptr(), JS_DONTENUM);

        js_pushobject(J, (*J).ReferenceError_prototype);
        jsB_props(J, c"name".as_ptr(), c"ReferenceError".as_ptr());
        js_newcconstructor(
            J,
            Some(jsB_ReferenceError),
            Some(jsB_ReferenceError),
            c"ReferenceError".as_ptr(),
            1,
        );
        js_defglobal(J, c"ReferenceError".as_ptr(), JS_DONTENUM);

        js_pushobject(J, (*J).SyntaxError_prototype);
        jsB_props(J, c"name".as_ptr(), c"SyntaxError".as_ptr());
        js_newcconstructor(J, Some(jsB_SyntaxError), Some(jsB_SyntaxError), c"SyntaxError".as_ptr(), 1);
        js_defglobal(J, c"SyntaxError".as_ptr(), JS_DONTENUM);

        js_pushobject(J, (*J).TypeError_prototype);
        jsB_props(J, c"name".as_ptr(), c"TypeError".as_ptr());
        js_newcconstructor(J, Some(jsB_TypeError), Some(jsB_TypeError), c"TypeError".as_ptr(), 1);
        js_defglobal(J, c"TypeError".as_ptr(), JS_DONTENUM);

        js_pushobject(J, (*J).URIError_prototype);
        jsB_props(J, c"name".as_ptr(), c"URIError".as_ptr());
        js_newcconstructor(J, Some(jsB_URIError), Some(jsB_URIError), c"URIError".as_ptr(), 1);
        js_defglobal(J, c"URIError".as_ptr(), JS_DONTENUM);
    }
}
