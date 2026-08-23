//! Translated from jsgc.c — mark & sweep garbage collector.
#![allow(non_snake_case, non_upper_case_globals)]

use crate::jsrun::js_free;
use crate::types::*;
use std::os::raw::{c_char, c_int, c_uint, c_void};

macro_rules! cstr {
    ($s:literal) => {
        concat!($s, "\0").as_ptr() as *const c_char
    };
}

unsafe fn jsG_freeenvironment(J: *mut js_State, env: *mut js_Environment) {
    js_free(J, env as *mut c_void);
}

unsafe fn jsG_freefunction(J: *mut js_State, fun: *mut js_Function) {
    js_free(J, (*fun).funtab as *mut c_void);
    js_free(J, (*fun).vartab as *mut c_void);
    js_free(J, (*fun).code as *mut c_void);
    js_free(J, fun as *mut c_void);
}

unsafe fn jsG_freeproperty(J: *mut js_State, node: *mut js_Property) {
    if (*(*node).left).level != 0 {
        jsG_freeproperty(J, (*node).left);
    }
    if (*(*node).right).level != 0 {
        jsG_freeproperty(J, (*node).right);
    }
    js_free(J, node as *mut c_void);
}

unsafe fn jsG_freeiterator(J: *mut js_State, mut node: *mut js_Iterator) {
    while !node.is_null() {
        let next = (*node).next;
        js_free(J, node as *mut c_void);
        node = next;
    }
}

unsafe fn jsG_freeobject(J: *mut js_State, obj: *mut js_Object) {
    if (*(*obj).properties).level != 0 {
        jsG_freeproperty(J, (*obj).properties);
    }
    if (*obj).type_ == JS_CREGEXP {
        js_free(J, (*obj).u.r.source as *mut c_void);
        crate::regexp::js_regfreex((*J).alloc, (*J).actx, (*obj).u.r.prog as *mut crate::regexp::Reprog);
    }
    if (*obj).type_ == JS_CSTRING {
        if (*obj).u.s.string != (*obj).u.s.shrstr.as_mut_ptr() {
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

unsafe fn jsG_markobject(J: *mut js_State, mark: c_int, obj: *mut js_Object) {
    (*obj).gcmark = mark;
    (*obj).gcroot = (*J).gcroot;
    (*J).gcroot = obj;
}

unsafe fn jsG_markfunction(J: *mut js_State, mark: c_int, fun: *mut js_Function) {
    (*fun).gcmark = mark;
    let mut i = 0;
    while i < (*fun).funlen {
        if (*(*(*fun).funtab.add(i as usize))).gcmark != mark {
            jsG_markfunction(J, mark, *(*fun).funtab.add(i as usize));
        }
        i += 1;
    }
}

unsafe fn jsG_markenvironment(J: *mut js_State, mark: c_int, mut env: *mut js_Environment) {
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

unsafe fn jsG_markproperty(J: *mut js_State, mark: c_int, node: *mut js_Property) {
    if (*(*node).left).level != 0 {
        jsG_markproperty(J, mark, (*node).left);
    }
    if (*(*node).right).level != 0 {
        jsG_markproperty(J, mark, (*node).right);
    }

    if (*node).value.type_() == JS_TMEMSTR && (*(*node).value.u.memstr).gcmark != mark as c_char {
        (*(*node).value.u.memstr).gcmark = mark as c_char;
    }
    if (*node).value.type_() == JS_TOBJECT && (*(*node).value.u.object).gcmark != mark {
        jsG_markobject(J, mark, (*node).value.u.object);
    }
    if !(*node).getter.is_null() && (*(*node).getter).gcmark != mark {
        jsG_markobject(J, mark, (*node).getter);
    }
    if !(*node).setter.is_null() && (*(*node).setter).gcmark != mark {
        jsG_markobject(J, mark, (*node).setter);
    }
}

unsafe fn jsG_scanobject(J: *mut js_State, mark: c_int, obj: *mut js_Object) {
    if (*(*obj).properties).level != 0 {
        jsG_markproperty(J, mark, (*obj).properties);
    }
    if !(*obj).prototype.is_null() && (*(*obj).prototype).gcmark != mark {
        jsG_markobject(J, mark, (*obj).prototype);
    }
    if (*obj).type_ == JS_CARRAY && (*obj).u.a.simple != 0 {
        let mut i = 0;
        while i < (*obj).u.a.flat_length {
            let v = (*obj).u.a.array.add(i as usize);
            if (*v).type_() == JS_TMEMSTR && (*(*v).u.memstr).gcmark != mark as c_char {
                (*(*v).u.memstr).gcmark = mark as c_char;
            }
            if (*v).type_() == JS_TOBJECT && (*(*v).u.object).gcmark != mark {
                jsG_markobject(J, mark, (*v).u.object);
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

unsafe fn jsG_markstack(J: *mut js_State, mark: c_int) {
    let mut v = (*J).stack;
    let mut n = (*J).top;
    while n != 0 {
        if (*v).type_() == JS_TMEMSTR && (*(*v).u.memstr).gcmark != mark as c_char {
            (*(*v).u.memstr).gcmark = mark as c_char;
        }
        if (*v).type_() == JS_TOBJECT && (*(*v).u.object).gcmark != mark {
            jsG_markobject(J, mark, (*v).u.object);
        }
        v = v.add(1);
        n -= 1;
    }
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_gc(J: *mut js_State, report: c_int) {
    let mut fun: *mut js_Function;
    let mut nextfun: *mut js_Function;
    let mut prevnextfun: *mut *mut js_Function;
    let mut obj: *mut js_Object;
    let mut nextobj: *mut js_Object;
    let mut prevnextobj: *mut *mut js_Object;
    let mut str: *mut js_String;
    let mut nextstr: *mut js_String;
    let mut prevnextstr: *mut *mut js_String;
    let mut env: *mut js_Environment;
    let mut nextenv: *mut js_Environment;
    let mut prevnextenv: *mut *mut js_Environment;
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
    let mark: c_int;
    let mut i: c_int;

    (*J).gcmark = if (*J).gcmark == 1 { 2 } else { 1 };
    mark = (*J).gcmark;

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
    i = 0;
    while i < (*J).envtop {
        jsG_markenvironment(J, mark, (*J).envstack[i as usize]);
        i += 1;
    }

    loop {
        obj = (*J).gcroot;
        if obj.is_null() {
            break;
        }
        (*J).gcroot = (*obj).gcroot;
        (*obj).gcroot = std::ptr::null_mut();
        jsG_scanobject(J, mark, obj);
    }

    prevnextenv = std::ptr::addr_of_mut!((*J).gcenv);
    env = (*J).gcenv;
    while !env.is_null() {
        nextenv = (*env).gcnext;
        if (*env).gcmark != mark {
            *prevnextenv = nextenv;
            jsG_freeenvironment(J, env);
            genv += 1;
        } else {
            prevnextenv = std::ptr::addr_of_mut!((*env).gcnext);
        }
        nenv += 1;
        env = nextenv;
    }

    prevnextfun = std::ptr::addr_of_mut!((*J).gcfun);
    fun = (*J).gcfun;
    while !fun.is_null() {
        nextfun = (*fun).gcnext;
        if (*fun).gcmark != mark {
            *prevnextfun = nextfun;
            jsG_freefunction(J, fun);
            gfun += 1;
        } else {
            prevnextfun = std::ptr::addr_of_mut!((*fun).gcnext);
        }
        nfun += 1;
        fun = nextfun;
    }

    prevnextobj = std::ptr::addr_of_mut!((*J).gcobj);
    obj = (*J).gcobj;
    while !obj.is_null() {
        nprop = nprop.wrapping_add((*obj).count as c_uint);
        nextobj = (*obj).gcnext;
        if (*obj).gcmark != mark {
            gprop = gprop.wrapping_add((*obj).count as c_uint);
            *prevnextobj = nextobj;
            jsG_freeobject(J, obj);
            gobj += 1;
        } else {
            prevnextobj = std::ptr::addr_of_mut!((*obj).gcnext);
        }
        nobj += 1;
        obj = nextobj;
    }

    prevnextstr = std::ptr::addr_of_mut!((*J).gcstr);
    str = (*J).gcstr;
    while !str.is_null() {
        nextstr = (*str).gcnext;
        if (*str).gcmark != mark as c_char {
            *prevnextstr = nextstr;
            js_free(J, str as *mut c_void);
            gstr += 1;
        } else {
            prevnextstr = std::ptr::addr_of_mut!((*str).gcnext);
        }
        nstr += 1;
        str = nextstr;
    }

    let ntot = nenv + nfun + nobj + nstr + nprop;
    let gtot = genv + gfun + gobj + gstr + gprop;
    let remaining = ntot.wrapping_sub(gtot);

    (*J).gccounter = remaining;
    // jsgc.c:250 `J->gcthresh = remaining * JS_GCFACTOR;` — implicit double -> `unsigned int`
    // (js_State.gcthresh, jsi.h:266); remaining * 5.0 can exceed UINT_MAX, where C keeps the
    // low 32 bits of (long)d instead of saturating.
    (*J).gcthresh = crate::cutil::d2u(remaining as f64 * JS_GCFACTOR);

    if report != 0 {
        let mut buf: [c_char; 256] = [0; 256];
        libc::snprintf(
            buf.as_mut_ptr(),
            256,
            cstr!("garbage collected (%d%%): %d/%d envs, %d/%d funs, %d/%d objs, %d/%d props, %d/%d strs"),
            (100u32.wrapping_mul(gtot) / ntot) as c_int,
            genv as c_int, nenv as c_int, gfun as c_int, nfun as c_int,
            gobj as c_int, nobj as c_int, gprop as c_int, nprop as c_int,
            gstr as c_int, nstr as c_int,
        );
        crate::jsstate::js_report(J, buf.as_ptr());
    }
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_freestate(J: *mut js_State) {
    let mut fun: *mut js_Function;
    let mut nextfun: *mut js_Function;
    let mut obj: *mut js_Object;
    let mut nextobj: *mut js_Object;
    let mut env: *mut js_Environment;
    let mut nextenv: *mut js_Environment;
    let mut str: *mut js_String;
    let mut nextstr: *mut js_String;

    if J.is_null() {
        return;
    }

    env = (*J).gcenv;
    while !env.is_null() {
        nextenv = (*env).gcnext;
        jsG_freeenvironment(J, env);
        env = nextenv;
    }
    fun = (*J).gcfun;
    while !fun.is_null() {
        nextfun = (*fun).gcnext;
        jsG_freefunction(J, fun);
        fun = nextfun;
    }
    obj = (*J).gcobj;
    while !obj.is_null() {
        nextobj = (*obj).gcnext;
        jsG_freeobject(J, obj);
        obj = nextobj;
    }
    str = (*J).gcstr;
    while !str.is_null() {
        nextstr = (*str).gcnext;
        js_free(J, str as *mut c_void);
        str = nextstr;
    }

    crate::jsintern::jsS_freestrings(J);

    js_free(J, (*J).lexbuf.text as *mut c_void);
    ((*J).alloc.unwrap())((*J).actx, (*J).stack as *mut c_void, 0);
    ((*J).alloc.unwrap())((*J).actx, J as *mut c_void, 0);
}
