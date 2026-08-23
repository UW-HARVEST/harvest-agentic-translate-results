//! Translated from jsfunction.c — Function constructor and prototype methods.
#![allow(non_snake_case, non_upper_case_globals)]

use crate::jsrun::*;
use crate::types::*;
use std::os::raw::{c_char, c_int};

macro_rules! cstr {
    ($s:literal) => {
        concat!($s, "\0").as_ptr() as *const c_char
    };
}

unsafe extern "C-unwind" fn jsB_Function(J: *mut js_State) {
    let top = js_gettop(J);
    let mut sb: *mut js_Buffer = std::ptr::null_mut();
    let mut fun: *mut js_Function = std::ptr::null_mut();

    let sb_ptr = std::ptr::addr_of_mut!(sb);
    let fun_ptr = std::ptr::addr_of_mut!(fun);
    let caught = crate::jsrun::protect(J, || {
        /* p1, p2, ..., pn */
        if top > 2 {
            let mut i = 1;
            while i < top - 1 {
                if i > 1 {
                    crate::jsintern::js_putc(J, sb_ptr, ',' as c_int);
                }
                crate::jsintern::js_puts(J, sb_ptr, js_tostring(J, i));
                i += 1;
            }
            crate::jsintern::js_putc(J, sb_ptr, ')' as c_int);
            crate::jsintern::js_putc(J, sb_ptr, 0);
        }

        /* body */
        let body: *const c_char = if js_isdefined(J, top - 1) != 0 {
            js_tostring(J, top - 1)
        } else {
            cstr!("")
        };

        let parse = crate::jsparse::jsP_parsefunction(
            J,
            cstr!("[string]"),
            if (*sb_ptr).is_null() {
                std::ptr::null()
            } else {
                (**sb_ptr).s.as_ptr()
            },
            body,
        );
        *fun_ptr = crate::jscompile::jsC_compilefunction(J, parse);
    });
    if caught {
        js_free(J, sb as *mut _);
        crate::jsparse::jsP_freeparse(J);
        js_throw(J);
    }
    js_endtry(J);
    js_free(J, sb as *mut _);
    crate::jsparse::jsP_freeparse(J);

    crate::jsvalue::js_newfunction(J, fun, (*J).GE);
}

unsafe extern "C-unwind" fn jsB_Function_prototype(J: *mut js_State) {
    js_pushundefined(J);
}

unsafe extern "C-unwind" fn Fp_toString(J: *mut js_State) {
    let self_ = js_toobject(J, 0);
    let mut sb: *mut js_Buffer = std::ptr::null_mut();

    if js_iscallable(J, 0) == 0 {
        crate::jserror::js_typeerror(J, cstr!("not a function"));
    }

    if (*self_).type_ == JS_CFUNCTION || (*self_).type_ == JS_CSCRIPT {
        let F: *mut js_Function = (*self_).u.f.function;

        let sb_ptr = std::ptr::addr_of_mut!(sb);
        let caught = crate::jsrun::protect(J, || {
            crate::jsintern::js_puts(J, sb_ptr, cstr!("function "));
            crate::jsintern::js_puts(J, sb_ptr, (*F).name);
            crate::jsintern::js_putc(J, sb_ptr, '(' as c_int);
            let mut i = 0;
            while i < (*F).numparams {
                if i > 0 {
                    crate::jsintern::js_putc(J, sb_ptr, ',' as c_int);
                }
                crate::jsintern::js_puts(J, sb_ptr, *(*F).vartab.add(i as usize));
                i += 1;
            }
            crate::jsintern::js_puts(J, sb_ptr, cstr!(") { [byte code] }"));
            crate::jsintern::js_putc(J, sb_ptr, 0);

            js_pushstring(J, (*sb).s.as_ptr());
        });
        if caught {
            js_free(J, sb as *mut _);
            js_throw(J);
        }
        js_endtry(J);
        js_free(J, sb as *mut _);
    } else if (*self_).type_ == JS_CCFUNCTION {
        let sb_ptr = std::ptr::addr_of_mut!(sb);
        let caught = crate::jsrun::protect(J, || {
            crate::jsintern::js_puts(J, sb_ptr, cstr!("function "));
            crate::jsintern::js_puts(J, sb_ptr, (*self_).u.c.name);
            crate::jsintern::js_puts(J, sb_ptr, cstr!("() { [native code] }"));
            crate::jsintern::js_putc(J, sb_ptr, 0);

            js_pushstring(J, (*sb).s.as_ptr());
        });
        if caught {
            js_free(J, sb as *mut _);
            js_throw(J);
        }
        js_endtry(J);
        js_free(J, sb as *mut _);
    } else {
        js_pushliteral(J, cstr!("function () { }"));
    }
}

unsafe extern "C-unwind" fn Fp_apply(J: *mut js_State) {
    let mut i: c_int;
    let n: c_int;

    if js_iscallable(J, 0) == 0 {
        crate::jserror::js_typeerror(J, cstr!("not a function"));
    }

    js_copy(J, 0);
    js_copy(J, 1);

    if js_isnull(J, 2) != 0 || js_isundefined(J, 2) != 0 {
        n = 0;
    } else {
        let mut m = crate::jsarray::js_getlength(J, 2);
        if m < 0 {
            m = 0;
        }
        n = m;
        i = 0;
        while i < n {
            js_getindex(J, 2, i);
            i += 1;
        }
    }

    js_call(J, n);
}

unsafe extern "C-unwind" fn Fp_call(J: *mut js_State) {
    let top = js_gettop(J);

    if js_iscallable(J, 0) == 0 {
        crate::jserror::js_typeerror(J, cstr!("not a function"));
    }

    let mut i = 0;
    while i < top {
        js_copy(J, i);
        i += 1;
    }

    js_call(J, top - 2);
}

unsafe extern "C-unwind" fn callbound(J: *mut js_State) {
    let top = js_gettop(J);
    let mut i: c_int;
    let fun: c_int;
    let args: c_int;
    let mut n: c_int;

    fun = js_gettop(J);
    js_currentfunction(J);
    js_getproperty(J, fun, cstr!("__TargetFunction__"));
    js_getproperty(J, fun, cstr!("__BoundThis__"));

    args = js_gettop(J);
    js_getproperty(J, fun, cstr!("__BoundArguments__"));
    n = crate::jsarray::js_getlength(J, args);
    if n < 0 {
        n = 0;
    }
    i = 0;
    while i < n {
        js_getindex(J, args, i);
        i += 1;
    }
    js_remove(J, args);

    i = 1;
    while i < top {
        js_copy(J, i);
        i += 1;
    }

    js_call(J, n + top - 1);
}

unsafe extern "C-unwind" fn constructbound(J: *mut js_State) {
    let top = js_gettop(J);
    let mut i: c_int;
    let fun: c_int;
    let args: c_int;
    let mut n: c_int;

    fun = js_gettop(J);
    js_currentfunction(J);
    js_getproperty(J, fun, cstr!("__TargetFunction__"));

    args = js_gettop(J);
    js_getproperty(J, fun, cstr!("__BoundArguments__"));
    n = crate::jsarray::js_getlength(J, args);
    if n < 0 {
        n = 0;
    }
    i = 0;
    while i < n {
        js_getindex(J, args, i);
        i += 1;
    }
    js_remove(J, args);

    i = 1;
    while i < top {
        js_copy(J, i);
        i += 1;
    }

    js_construct(J, n + top - 1);
}

unsafe extern "C-unwind" fn Fp_bind(J: *mut js_State) {
    let top = js_gettop(J);
    let mut n: c_int;

    if js_iscallable(J, 0) == 0 {
        crate::jserror::js_typeerror(J, cstr!("not a function"));
    }

    n = crate::jsarray::js_getlength(J, 0);
    if n > top - 2 {
        n -= top - 2;
    } else {
        n = 0;
    }

    /* Reuse target function's prototype for HasInstance check. */
    js_getproperty(J, 0, cstr!("prototype"));
    crate::jsvalue::js_newcconstructor(J, Some(callbound), Some(constructbound), cstr!("[bind]"), n);

    /* target function */
    js_copy(J, 0);
    js_defproperty(J, -2, cstr!("__TargetFunction__"), JS_READONLY | JS_DONTENUM | JS_DONTCONF);

    /* bound this */
    js_copy(J, 1);
    js_defproperty(J, -2, cstr!("__BoundThis__"), JS_READONLY | JS_DONTENUM | JS_DONTCONF);

    /* bound arguments */
    crate::jsvalue::js_newarray(J);
    let mut i = 2;
    while i < top {
        js_copy(J, i);
        js_setindex(J, -2, i - 2);
        i += 1;
    }
    js_defproperty(J, -2, cstr!("__BoundArguments__"), JS_READONLY | JS_DONTENUM | JS_DONTCONF);
}

#[no_mangle]
pub unsafe extern "C-unwind" fn jsB_initfunction(J: *mut js_State) {
    (*(*J).Function_prototype).u.c.name = cstr!("Function.prototype");
    (*(*J).Function_prototype).u.c.function = Some(jsB_Function_prototype);
    (*(*J).Function_prototype).u.c.constructor = None;
    (*(*J).Function_prototype).u.c.length = 0;

    js_pushobject(J, (*J).Function_prototype);
    {
        crate::jsbuiltin::jsB_propf(J, cstr!("Function.prototype.toString"), Some(Fp_toString), 2);
        crate::jsbuiltin::jsB_propf(J, cstr!("Function.prototype.apply"), Some(Fp_apply), 2);
        crate::jsbuiltin::jsB_propf(J, cstr!("Function.prototype.call"), Some(Fp_call), 1);
        crate::jsbuiltin::jsB_propf(J, cstr!("Function.prototype.bind"), Some(Fp_bind), 1);
    }
    crate::jsvalue::js_newcconstructor(J, Some(jsB_Function), Some(jsB_Function), cstr!("Function"), 1);
    js_defglobal(J, cstr!("Function"), JS_DONTENUM);
}
