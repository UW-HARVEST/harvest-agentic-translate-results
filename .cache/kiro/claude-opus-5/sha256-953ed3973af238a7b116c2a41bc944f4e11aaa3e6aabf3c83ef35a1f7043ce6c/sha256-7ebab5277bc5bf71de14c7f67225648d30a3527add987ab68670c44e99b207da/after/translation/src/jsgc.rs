// Translation of c_src/src/jsgc.c
#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals, dead_code)]

use crate::common::*;
use crate::jsintern::jsS_freestrings;
use crate::jsregexp::js_regfreex;
use crate::jsrun::*;
use crate::jsstate::js_report;
use crate::types::*;
use std::ffi::{c_char, c_int, c_uint, c_void};
use std::ptr;

unsafe fn jsG_freeenvironment(J: *mut js_State, env: *mut js_Environment) {
    unsafe {
        js_free(J, env as *mut c_void);
    }
}

unsafe fn jsG_freefunction(J: *mut js_State, fun: *mut js_Function) {
    unsafe {
        js_free(J, (*fun).funtab as *mut c_void);
        js_free(J, (*fun).vartab as *mut c_void);
        js_free(J, (*fun).code as *mut c_void);
        js_free(J, fun as *mut c_void);
    }
}

unsafe fn jsG_freeproperty(J: *mut js_State, node: *mut js_Property) {
    unsafe {
        if (*(*node).left).level != 0 {
            jsG_freeproperty(J, (*node).left);
        }
        if (*(*node).right).level != 0 {
            jsG_freeproperty(J, (*node).right);
        }
        js_free(J, node as *mut c_void);
    }
}

unsafe fn jsG_freeiterator(J: *mut js_State, node: *mut js_Iterator) {
    unsafe {
        let mut node = node;
        while !node.is_null() {
            let next = (*node).next;
            js_free(J, node as *mut c_void);
            node = next;
        }
    }
}

unsafe fn jsG_freeobject(J: *mut js_State, obj: *mut js_Object) {
    unsafe {
        if (*(*obj).properties).level != 0 {
            jsG_freeproperty(J, (*obj).properties);
        }
        if (*obj).type_ == JS_CREGEXP {
            js_free(J, (*obj).u.r.source as *mut c_void);
            js_regfreex((*J).alloc, (*J).actx, (*obj).u.r.prog as *mut crate::regexp::Reprog);
        }
        if (*obj).type_ == JS_CSTRING {
            if (*obj).u.s.string != (&raw mut (*obj).u.s.shrstr) as *mut c_char {
                js_free(J, (*obj).u.s.string as *mut c_void);
            }
        }
        if (*obj).type_ == JS_CARRAY && (*obj).u.a.simple != 0 {
            js_free(J, (*obj).u.a.array as *mut c_void);
        }
        if (*obj).type_ == JS_CITERATOR {
            jsG_freeiterator(J, (*obj).u.iter.head);
        }
        if (*obj).type_ == JS_CUSERDATA {
            if let Some(f) = (*obj).u.user.finalize {
                f(J, (*obj).u.user.data);
            }
        }
        if (*obj).type_ == JS_CCFUNCTION {
            if let Some(f) = (*obj).u.c.finalize {
                f(J, (*obj).u.c.data);
            }
        }
        js_free(J, obj as *mut c_void);
    }
}

/* Mark and add object to scan queue */
unsafe fn jsG_markobject(J: *mut js_State, mark: c_int, obj: *mut js_Object) {
    unsafe {
        (*obj).gcmark = mark;
        (*obj).gcroot = (*J).gcroot;
        (*J).gcroot = obj;
    }
}

unsafe fn jsG_markfunction(J: *mut js_State, mark: c_int, fun: *mut js_Function) {
    unsafe {
        (*fun).gcmark = mark;
        let mut i = 0;
        while i < (*fun).funlen {
            if (**(*fun).funtab.offset(i as isize)).gcmark != mark {
                jsG_markfunction(J, mark, *(*fun).funtab.offset(i as isize));
            }
            i += 1;
        }
    }
}

unsafe fn jsG_markenvironment(J: *mut js_State, mark: c_int, env: *mut js_Environment) {
    unsafe {
        let mut env = env;
        loop {
            (*env).gcmark = mark;
            if (*(*env).variables).gcmark != mark {
                jsG_markobject(J, mark, (*env).variables);
            }
            env = (*env).outer;
            if env.is_null() || (*env).gcmark == mark {
                break;
            }
        }
    }
}

unsafe fn jsG_markproperty(J: *mut js_State, mark: c_int, node: *mut js_Property) {
    unsafe {
        if (*(*node).left).level != 0 {
            jsG_markproperty(J, mark, (*node).left);
        }
        if (*(*node).right).level != 0 {
            jsG_markproperty(J, mark, (*node).right);
        }

        if (*node).value.ty() == JS_TMEMSTR && (*(*node).value.memstr()).gcmark as c_int != mark {
            (*(*node).value.memstr()).gcmark = mark as c_char;
        }
        if (*node).value.ty() == JS_TOBJECT && (*(*node).value.object()).gcmark != mark {
            jsG_markobject(J, mark, (*node).value.object());
        }
        if !(*node).getter.is_null() && (*(*node).getter).gcmark != mark {
            jsG_markobject(J, mark, (*node).getter);
        }
        if !(*node).setter.is_null() && (*(*node).setter).gcmark != mark {
            jsG_markobject(J, mark, (*node).setter);
        }
    }
}

/* Mark everything the object can reach. */
unsafe fn jsG_scanobject(J: *mut js_State, mark: c_int, obj: *mut js_Object) {
    unsafe {
        if (*(*obj).properties).level != 0 {
            jsG_markproperty(J, mark, (*obj).properties);
        }
        if !(*obj).prototype.is_null() && (*(*obj).prototype).gcmark != mark {
            jsG_markobject(J, mark, (*obj).prototype);
        }
        if (*obj).type_ == JS_CARRAY && (*obj).u.a.simple != 0 {
            let mut i = 0;
            while i < (*obj).u.a.flat_length {
                let v = (*obj).u.a.array.offset(i as isize);
                if (*v).ty() == JS_TMEMSTR && (*(*v).memstr()).gcmark as c_int != mark {
                    (*(*v).memstr()).gcmark = mark as c_char;
                }
                if (*v).ty() == JS_TOBJECT && (*(*v).object()).gcmark != mark {
                    jsG_markobject(J, mark, (*v).object());
                }
                i += 1;
            }
        }
        if (*obj).type_ == JS_CITERATOR && (*(*obj).u.iter.target).gcmark != mark {
            jsG_markobject(J, mark, (*obj).u.iter.target);
        }
        if (*obj).type_ == JS_CFUNCTION || (*obj).type_ == JS_CSCRIPT {
            if !(*obj).u.f.scope.is_null() && (*(*obj).u.f.scope).gcmark != mark {
                jsG_markenvironment(J, mark, (*obj).u.f.scope);
            }
            if !(*obj).u.f.function.is_null() && (*(*obj).u.f.function).gcmark != mark {
                jsG_markfunction(J, mark, (*obj).u.f.function);
            }
        }
    }
}

unsafe fn jsG_markstack(J: *mut js_State, mark: c_int) {
    unsafe {
        let mut v = (*J).stack;
        let mut n = (*J).top;
        while n != 0 {
            n -= 1;
            if (*v).ty() == JS_TMEMSTR && (*(*v).memstr()).gcmark as c_int != mark {
                (*(*v).memstr()).gcmark = mark as c_char;
            }
            if (*v).ty() == JS_TOBJECT && (*(*v).object()).gcmark != mark {
                jsG_markobject(J, mark, (*v).object());
            }
            v = v.add(1);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_gc(J: *mut js_State, report: c_int) {
    unsafe {
        let mut nextfun: *mut js_Function;
        let mut nextobj: *mut js_Object;
        let mut nextstr: *mut js_String;
        let mut nextenv: *mut js_Environment;
        let mut nenv: c_uint = 0;
        let mut nfun: c_uint = 0;
        let mut nobj: c_uint = 0;
        let mut nstr: c_uint = 0;
        let mut nprop: c_uint = 0;
        let mut genv: c_uint = 0;
        let mut gfun: c_uint = 0;
        let mut gobj: c_uint = 0;
        let mut gstr: c_uint = 0;
        let mut gprop: c_uint = 0;

        (*J).gcmark = if (*J).gcmark == 1 { 2 } else { 1 };
        let mark = (*J).gcmark;

        /* Add initial roots. */

        jsG_markobject(J, mark, (*J).Object_prototype);
        jsG_markobject(J, mark, (*J).Array_prototype);
        jsG_markobject(J, mark, (*J).Function_prototype);
        jsG_markobject(J, mark, (*J).Boolean_prototype);
        jsG_markobject(J, mark, (*J).Number_prototype);
        jsG_markobject(J, mark, (*J).String_prototype);
        jsG_markobject(J, mark, (*J).RegExp_prototype);
        jsG_markobject(J, mark, (*J).Date_prototype);

        jsG_markobject(J, mark, (*J).Error_prototype);
        jsG_markobject(J, mark, (*J).EvalError_prototype);
        jsG_markobject(J, mark, (*J).RangeError_prototype);
        jsG_markobject(J, mark, (*J).ReferenceError_prototype);
        jsG_markobject(J, mark, (*J).SyntaxError_prototype);
        jsG_markobject(J, mark, (*J).TypeError_prototype);
        jsG_markobject(J, mark, (*J).URIError_prototype);

        jsG_markobject(J, mark, (*J).R);
        jsG_markobject(J, mark, (*J).G);

        jsG_markstack(J, mark);

        jsG_markenvironment(J, mark, (*J).E);
        jsG_markenvironment(J, mark, (*J).GE);
        let mut i = 0;
        while i < (*J).envtop {
            jsG_markenvironment(J, mark, (*J).envstack[i as usize]);
            i += 1;
        }

        /* Scan objects until none remain. */

        loop {
            let obj = (*J).gcroot;
            if obj.is_null() {
                break;
            }
            (*J).gcroot = (*obj).gcroot;
            (*obj).gcroot = ptr::null_mut();
            jsG_scanobject(J, mark, obj);
        }

        /* Free everything not marked. */

        let mut prevnextenv = &raw mut (*J).gcenv;
        let mut env = (*J).gcenv;
        while !env.is_null() {
            nextenv = (*env).gcnext;
            if (*env).gcmark != mark {
                *prevnextenv = nextenv;
                jsG_freeenvironment(J, env);
                genv += 1;
            } else {
                prevnextenv = &raw mut (*env).gcnext;
            }
            nenv += 1;
            env = nextenv;
        }

        let mut prevnextfun = &raw mut (*J).gcfun;
        let mut fun = (*J).gcfun;
        while !fun.is_null() {
            nextfun = (*fun).gcnext;
            if (*fun).gcmark != mark {
                *prevnextfun = nextfun;
                jsG_freefunction(J, fun);
                gfun += 1;
            } else {
                prevnextfun = &raw mut (*fun).gcnext;
            }
            nfun += 1;
            fun = nextfun;
        }

        let mut prevnextobj = &raw mut (*J).gcobj;
        let mut obj = (*J).gcobj;
        while !obj.is_null() {
            nprop += (*obj).count as c_uint;
            nextobj = (*obj).gcnext;
            if (*obj).gcmark != mark {
                gprop += (*obj).count as c_uint;
                *prevnextobj = nextobj;
                jsG_freeobject(J, obj);
                gobj += 1;
            } else {
                prevnextobj = &raw mut (*obj).gcnext;
            }
            nobj += 1;
            obj = nextobj;
        }

        let mut prevnextstr = &raw mut (*J).gcstr;
        let mut str = (*J).gcstr;
        while !str.is_null() {
            nextstr = (*str).gcnext;
            if (*str).gcmark as c_int != mark {
                *prevnextstr = nextstr;
                js_free(J, str as *mut c_void);
                gstr += 1;
            } else {
                prevnextstr = &raw mut (*str).gcnext;
            }
            nstr += 1;
            str = nextstr;
        }

        let ntot: c_uint = nenv + nfun + nobj + nstr + nprop;
        let gtot: c_uint = genv + gfun + gobj + gstr + gprop;
        let remaining: c_uint = ntot.wrapping_sub(gtot);

        (*J).gccounter = remaining;
        // C: `J->gcthresh = remaining * JS_GCFACTOR;` -- double to unsigned
        // int, which on x86-64 truncates through a 64-bit signed conversion
        // rather than saturating the way Rust's `as` would.
        (*J).gcthresh = d2l(remaining as f64 * JS_GCFACTOR) as c_uint;

        if report != 0 {
            let mut buf: [c_char; 256] = [0; 256];
            let pct = if ntot != 0 { 100 * gtot / ntot } else { 0 };
            snprintf(
                buf.as_mut_ptr(),
                256,
                c"garbage collected (%d%%): %d/%d envs, %d/%d funs, %d/%d objs, %d/%d props, %d/%d strs"
                    .as_ptr(),
                pct,
                genv,
                nenv,
                gfun,
                nfun,
                gobj,
                nobj,
                gprop,
                nprop,
                gstr,
                nstr,
            );
            js_report(J, buf.as_ptr());
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_freestate(J: *mut js_State) {
    unsafe {
        if J.is_null() {
            return;
        }

        let mut env = (*J).gcenv;
        while !env.is_null() {
            let nextenv = (*env).gcnext;
            jsG_freeenvironment(J, env);
            env = nextenv;
        }
        let mut fun = (*J).gcfun;
        while !fun.is_null() {
            let nextfun = (*fun).gcnext;
            jsG_freefunction(J, fun);
            fun = nextfun;
        }
        let mut obj = (*J).gcobj;
        while !obj.is_null() {
            let nextobj = (*obj).gcnext;
            jsG_freeobject(J, obj);
            obj = nextobj;
        }
        let mut str = (*J).gcstr;
        while !str.is_null() {
            let nextstr = (*str).gcnext;
            js_free(J, str as *mut c_void);
            str = nextstr;
        }

        jsS_freestrings(J);

        js_free(J, (*J).lexbuf.text as *mut c_void);
        ((*J).alloc.unwrap())((*J).actx, (*J).stack as *mut c_void, 0);
        ((*J).alloc.unwrap())((*J).actx, J as *mut c_void, 0);
    }
}
