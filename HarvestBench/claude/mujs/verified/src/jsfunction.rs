//! Translated from c_src/src/jsfunction.c
use crate::jsi::*;
use crate::prelude::*;

unsafe extern "C" fn jsB_Function(J: *mut js_State) {
    let mut i: c_int;
    let top: c_int = js_gettop(J);
    let mut sb: *mut js_Buffer = null_mut();
    let body: *const c_char;
    let parse: *mut js_Ast;
    let fun: *mut js_Function;

    if js_try!(J) {
        js_free(J, sb as *mut c_void);
        jsP_freeparse(J);
        js_throw(J);
    }

    /* p1, p2, ..., pn */
    if top > 2 {
        i = 1;
        while i < top - 1 {
            if i > 1 {
                js_putc(J, &mut sb, ',' as c_int);
            }
            js_puts(J, &mut sb, js_tostring(J, i));
            i += 1;
        }
        js_putc(J, &mut sb, ')' as c_int);
        js_putc(J, &mut sb, 0);
    }

    /* body */
    body = if js_isdefined(J, top - 1) != 0 {
        js_tostring(J, top - 1)
    } else {
        c"".as_ptr()
    };

    parse = jsP_parsefunction(
        J,
        c"[string]".as_ptr(),
        if !sb.is_null() {
            js_Buffer_s(sb) as *const c_char
        } else {
            null()
        },
        body,
    );
    fun = jsC_compilefunction(J, parse);

    js_endtry(J);
    js_free(J, sb as *mut c_void);
    jsP_freeparse(J);

    js_newfunction(J, fun, (*J).GE);
}

unsafe extern "C" fn jsB_Function_prototype(J: *mut js_State) {
    js_pushundefined(J);
}

unsafe extern "C" fn Fp_toString(J: *mut js_State) {
    let self_: *mut js_Object = js_toobject(J, 0);
    let mut sb: *mut js_Buffer = null_mut();
    let mut i: c_int;

    if js_iscallable(J, 0) == 0 {
        js_typeerror!(J, c"not a function".as_ptr());
    }

    if (*self_).r#type == JS_CFUNCTION || (*self_).r#type == JS_CSCRIPT {
        let F: *mut js_Function = (*self_).u.f.function;

        if js_try!(J) {
            js_free(J, sb as *mut c_void);
            js_throw(J);
        }

        js_puts(J, &mut sb, c"function ".as_ptr());
        js_puts(J, &mut sb, (*F).name);
        js_putc(J, &mut sb, '(' as c_int);
        i = 0;
        while i < (*F).numparams {
            if i > 0 {
                js_putc(J, &mut sb, ',' as c_int);
            }
            js_puts(J, &mut sb, *(*F).vartab.add(i as usize));
            i += 1;
        }
        js_puts(J, &mut sb, c") { [byte code] }".as_ptr());
        js_putc(J, &mut sb, 0);

        js_pushstring(J, js_Buffer_s(sb) as *const c_char);
        js_endtry(J);
        js_free(J, sb as *mut c_void);
    } else if (*self_).r#type == JS_CCFUNCTION {
        if js_try!(J) {
            js_free(J, sb as *mut c_void);
            js_throw(J);
        }

        js_puts(J, &mut sb, c"function ".as_ptr());
        js_puts(J, &mut sb, (*self_).u.c.name);
        js_puts(J, &mut sb, c"() { [native code] }".as_ptr());
        js_putc(J, &mut sb, 0);

        js_pushstring(J, js_Buffer_s(sb) as *const c_char);
        js_endtry(J);
        js_free(J, sb as *mut c_void);
    } else {
        js_pushliteral(J, c"function () { }".as_ptr());
    }
}

unsafe extern "C" fn Fp_apply(J: *mut js_State) {
    let mut i: c_int;
    let mut n: c_int;

    if js_iscallable(J, 0) == 0 {
        js_typeerror!(J, c"not a function".as_ptr());
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
        i = 0;
        while i < n {
            js_getindex(J, 2, i);
            i += 1;
        }
    }

    js_call(J, n);
}

unsafe extern "C" fn Fp_call(J: *mut js_State) {
    let mut i: c_int;
    let top: c_int = js_gettop(J);

    if js_iscallable(J, 0) == 0 {
        js_typeerror!(J, c"not a function".as_ptr());
    }

    i = 0;
    while i < top {
        js_copy(J, i);
        i += 1;
    }

    js_call(J, top - 2);
}

unsafe extern "C" fn callbound(J: *mut js_State) {
    let top: c_int = js_gettop(J);
    let mut i: c_int;
    let fun: c_int;
    let args: c_int;
    let mut n: c_int;

    fun = js_gettop(J);
    js_currentfunction(J);
    js_getproperty(J, fun, c"__TargetFunction__".as_ptr());
    js_getproperty(J, fun, c"__BoundThis__".as_ptr());

    args = js_gettop(J);
    js_getproperty(J, fun, c"__BoundArguments__".as_ptr());
    n = js_getlength(J, args);
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

unsafe extern "C" fn constructbound(J: *mut js_State) {
    let top: c_int = js_gettop(J);
    let mut i: c_int;
    let fun: c_int;
    let args: c_int;
    let mut n: c_int;

    fun = js_gettop(J);
    js_currentfunction(J);
    js_getproperty(J, fun, c"__TargetFunction__".as_ptr());

    args = js_gettop(J);
    js_getproperty(J, fun, c"__BoundArguments__".as_ptr());
    n = js_getlength(J, args);
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

unsafe extern "C" fn Fp_bind(J: *mut js_State) {
    let mut i: c_int;
    let top: c_int = js_gettop(J);
    let mut n: c_int;

    if js_iscallable(J, 0) == 0 {
        js_typeerror!(J, c"not a function".as_ptr());
    }

    n = js_getlength(J, 0);
    if n > top - 2 {
        n -= top - 2;
    } else {
        n = 0;
    }

    /* Reuse target function's prototype for HasInstance check. */
    js_getproperty(J, 0, c"prototype".as_ptr());
    js_newcconstructor(
        J,
        Some(callbound),
        Some(constructbound),
        c"[bind]".as_ptr(),
        n,
    );

    /* target function */
    js_copy(J, 0);
    js_defproperty(
        J,
        -2,
        c"__TargetFunction__".as_ptr(),
        JS_READONLY | JS_DONTENUM | JS_DONTCONF,
    );

    /* bound this */
    js_copy(J, 1);
    js_defproperty(
        J,
        -2,
        c"__BoundThis__".as_ptr(),
        JS_READONLY | JS_DONTENUM | JS_DONTCONF,
    );

    /* bound arguments */
    js_newarray(J);
    i = 2;
    while i < top {
        js_copy(J, i);
        js_setindex(J, -2, i - 2);
        i += 1;
    }
    js_defproperty(
        J,
        -2,
        c"__BoundArguments__".as_ptr(),
        JS_READONLY | JS_DONTENUM | JS_DONTCONF,
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsB_initfunction(J: *mut js_State) {
    (*(*J).Function_prototype).u.c.name = c"Function.prototype".as_ptr();
    (*(*J).Function_prototype).u.c.function = Some(jsB_Function_prototype);
    (*(*J).Function_prototype).u.c.constructor = None;
    (*(*J).Function_prototype).u.c.length = 0;

    js_pushobject(J, (*J).Function_prototype);
    {
        jsB_propf(
            J,
            c"Function.prototype.toString".as_ptr(),
            Some(Fp_toString),
            2,
        );
        jsB_propf(J, c"Function.prototype.apply".as_ptr(), Some(Fp_apply), 2);
        jsB_propf(J, c"Function.prototype.call".as_ptr(), Some(Fp_call), 1);
        jsB_propf(J, c"Function.prototype.bind".as_ptr(), Some(Fp_bind), 1);
    }
    js_newcconstructor(
        J,
        Some(jsB_Function),
        Some(jsB_Function),
        c"Function".as_ptr(),
        1,
    );
    js_defglobal(J, c"Function".as_ptr(), JS_DONTENUM);
}
