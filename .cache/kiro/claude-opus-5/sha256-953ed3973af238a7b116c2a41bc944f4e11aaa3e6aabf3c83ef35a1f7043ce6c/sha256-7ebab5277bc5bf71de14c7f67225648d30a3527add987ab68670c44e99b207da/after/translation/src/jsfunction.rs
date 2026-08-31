#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals, dead_code)]
use crate::common::*;
use crate::jsbuiltin::jsB_propf;
use crate::jscompile::jsC_compilefunction;
use crate::jsintern::{js_putc, js_puts};
use crate::jsparse::{jsP_freeparse, jsP_parsefunction};
use crate::jsarray::js_getlength;
use crate::jsrun::{
    js_call, js_construct, js_copy, js_currentfunction, js_defglobal, js_defproperty, js_endtry,
    js_free, js_getindex, js_getproperty, js_gettop, js_iscallable, js_isdefined,
    js_isnull, js_isundefined, js_pushliteral, js_pushobject, js_pushstring,
    js_pushundefined, js_remove, js_setindex, js_throw, js_tostring, js_toobject,
};
use crate::jsvalue::{js_newarray, js_newcconstructor, js_newfunction};
use crate::types::*;
use crate::js_typeerror;
use std::ffi::{c_char, c_int, c_void};

unsafe extern "C-unwind" fn jsB_Function(J: *mut js_State) {
    unsafe {
        let top = js_gettop(J);
        let mut sb: *mut js_Buffer = std::ptr::null_mut();

        if js_try(J, || {
            /* p1, p2, ..., pn */
            if top > 2 {
                let mut i = 1;
                while i < top - 1 {
                    if i > 1 {
                        js_putc(J, &raw mut sb, b',' as c_int);
                    }
                    js_puts(J, &raw mut sb, js_tostring(J, i));
                    i += 1;
                }
                js_putc(J, &raw mut sb, b')' as c_int);
                js_putc(J, &raw mut sb, 0);
            }

            /* body */
            let body: *const c_char = if js_isdefined(J, top - 1) != 0 {
                js_tostring(J, top - 1)
            } else {
                c"".as_ptr()
            };

            let parse = jsP_parsefunction(
                J,
                c"[string]".as_ptr(),
                if !sb.is_null() {
                    (&raw mut (*sb).s) as *const c_char
                } else {
                    std::ptr::null()
                },
                body,
            );
            let fun = jsC_compilefunction(J, parse);

            js_endtry(J);
            js_free(J, sb as *mut c_void);
            jsP_freeparse(J);

            js_newfunction(J, fun, (*J).GE);
        })
        .is_err()
        {
            js_free(J, sb as *mut c_void);
            jsP_freeparse(J);
            js_throw(J);
        }
    }
}

unsafe extern "C-unwind" fn jsB_Function_prototype(J: *mut js_State) {
    unsafe {
        js_pushundefined(J);
    }
}

unsafe extern "C-unwind" fn Fp_toString(J: *mut js_State) {
    unsafe {
        let self_ = js_toobject(J, 0);
        let mut sb: *mut js_Buffer = std::ptr::null_mut();

        if js_iscallable(J, 0) == 0 {
            js_typeerror!(J, c"not a function");
        }

        if (*self_).type_ == JS_CFUNCTION || (*self_).type_ == JS_CSCRIPT {
            let f = (*self_).u.f.function;

            if js_try(J, || {
                js_puts(J, &raw mut sb, c"function ".as_ptr());
                js_puts(J, &raw mut sb, (*f).name);
                js_putc(J, &raw mut sb, b'(' as c_int);
                let mut i = 0;
                while i < (*f).numparams {
                    if i > 0 {
                        js_putc(J, &raw mut sb, b',' as c_int);
                    }
                    js_puts(J, &raw mut sb, *(*f).vartab.offset(i as isize));
                    i += 1;
                }
                js_puts(J, &raw mut sb, c") { [byte code] }".as_ptr());
                js_putc(J, &raw mut sb, 0);

                js_pushstring(J, (&raw mut (*sb).s) as *const c_char);
                js_endtry(J);
                js_free(J, sb as *mut c_void);
            })
            .is_err()
            {
                js_free(J, sb as *mut c_void);
                js_throw(J);
            }
        } else if (*self_).type_ == JS_CCFUNCTION {
            if js_try(J, || {
                js_puts(J, &raw mut sb, c"function ".as_ptr());
                js_puts(J, &raw mut sb, (*self_).u.c.name);
                js_puts(J, &raw mut sb, c"() { [native code] }".as_ptr());
                js_putc(J, &raw mut sb, 0);

                js_pushstring(J, (&raw mut (*sb).s) as *const c_char);
                js_endtry(J);
                js_free(J, sb as *mut c_void);
            })
            .is_err()
            {
                js_free(J, sb as *mut c_void);
                js_throw(J);
            }
        } else {
            js_pushliteral(J, c"function () { }".as_ptr());
        }
    }
}

unsafe extern "C-unwind" fn Fp_apply(J: *mut js_State) {
    unsafe {
        let mut n: c_int;

        if js_iscallable(J, 0) == 0 {
            js_typeerror!(J, c"not a function");
        }

        js_copy(J, 0);
        js_copy(J, 1);

        if js_isnull(J, 2) != 0 || js_isundefined(J, 2) != 0 {
            n = 0;
        } else {
            n = js_getlength(J, 2);
            if n < 0 {
                n = 0;
            }
            let mut i = 0;
            while i < n {
                js_getindex(J, 2, i);
                i += 1;
            }
        }

        js_call(J, n);
    }
}

unsafe extern "C-unwind" fn Fp_call(J: *mut js_State) {
    unsafe {
        let top = js_gettop(J);

        if js_iscallable(J, 0) == 0 {
            js_typeerror!(J, c"not a function");
        }

        let mut i = 0;
        while i < top {
            js_copy(J, i);
            i += 1;
        }

        js_call(J, top - 2);
    }
}

unsafe extern "C-unwind" fn callbound(J: *mut js_State) {
    unsafe {
        let top = js_gettop(J);
        let mut n: c_int;

        let fun = js_gettop(J);
        js_currentfunction(J);
        js_getproperty(J, fun, c"__TargetFunction__".as_ptr());
        js_getproperty(J, fun, c"__BoundThis__".as_ptr());

        let args = js_gettop(J);
        js_getproperty(J, fun, c"__BoundArguments__".as_ptr());
        n = js_getlength(J, args);
        if n < 0 {
            n = 0;
        }
        let mut i = 0;
        while i < n {
            js_getindex(J, args, i);
            i += 1;
        }
        js_remove(J, args);

        let mut i = 1;
        while i < top {
            js_copy(J, i);
            i += 1;
        }

        js_call(J, n + top - 1);
    }
}

unsafe extern "C-unwind" fn constructbound(J: *mut js_State) {
    unsafe {
        let top = js_gettop(J);
        let mut n: c_int;

        let fun = js_gettop(J);
        js_currentfunction(J);
        js_getproperty(J, fun, c"__TargetFunction__".as_ptr());

        let args = js_gettop(J);
        js_getproperty(J, fun, c"__BoundArguments__".as_ptr());
        n = js_getlength(J, args);
        if n < 0 {
            n = 0;
        }
        let mut i = 0;
        while i < n {
            js_getindex(J, args, i);
            i += 1;
        }
        js_remove(J, args);

        let mut i = 1;
        while i < top {
            js_copy(J, i);
            i += 1;
        }

        js_construct(J, n + top - 1);
    }
}

unsafe extern "C-unwind" fn Fp_bind(J: *mut js_State) {
    unsafe {
        let top = js_gettop(J);
        let mut n: c_int;

        if js_iscallable(J, 0) == 0 {
            js_typeerror!(J, c"not a function");
        }

        n = js_getlength(J, 0);
        if n > top - 2 {
            n -= top - 2;
        } else {
            n = 0;
        }

        /* Reuse target function's prototype for HasInstance check. */
        js_getproperty(J, 0, c"prototype".as_ptr());
        js_newcconstructor(J, Some(callbound), Some(constructbound), c"[bind]".as_ptr(), n);

        /* target function */
        js_copy(J, 0);
        js_defproperty(J, -2, c"__TargetFunction__".as_ptr(), JS_READONLY | JS_DONTENUM | JS_DONTCONF);

        /* bound this */
        js_copy(J, 1);
        js_defproperty(J, -2, c"__BoundThis__".as_ptr(), JS_READONLY | JS_DONTENUM | JS_DONTCONF);

        /* bound arguments */
        js_newarray(J);
        let mut i = 2;
        while i < top {
            js_copy(J, i);
            js_setindex(J, -2, i - 2);
            i += 1;
        }
        js_defproperty(J, -2, c"__BoundArguments__".as_ptr(), JS_READONLY | JS_DONTENUM | JS_DONTCONF);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsB_initfunction(J: *mut js_State) {
    unsafe {
        (*(*J).Function_prototype).u.c.name = c"Function.prototype".as_ptr();
        (*(*J).Function_prototype).u.c.function = Some(jsB_Function_prototype);
        (*(*J).Function_prototype).u.c.constructor = None;
        (*(*J).Function_prototype).u.c.length = 0;

        js_pushobject(J, (*J).Function_prototype);
        {
            jsB_propf(J, c"Function.prototype.toString".as_ptr(), Some(Fp_toString), 2);
            jsB_propf(J, c"Function.prototype.apply".as_ptr(), Some(Fp_apply), 2);
            jsB_propf(J, c"Function.prototype.call".as_ptr(), Some(Fp_call), 1);
            jsB_propf(J, c"Function.prototype.bind".as_ptr(), Some(Fp_bind), 1);
        }
        js_newcconstructor(J, Some(jsB_Function), Some(jsB_Function), c"Function".as_ptr(), 1);
        js_defglobal(J, c"Function".as_ptr(), JS_DONTENUM);
    }
}
