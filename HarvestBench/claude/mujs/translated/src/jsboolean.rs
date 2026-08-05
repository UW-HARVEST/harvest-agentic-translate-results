//! Translated from jsboolean.c — Boolean object.
#![allow(non_snake_case, non_upper_case_globals)]

use crate::jsrun::*;
use crate::jsvalue::*;
use crate::types::*;
use std::os::raw::c_char;

macro_rules! cstr {
    ($s:literal) => {
        concat!($s, "\0").as_ptr() as *const c_char
    };
}

unsafe extern "C-unwind" fn jsB_new_Boolean(J: *mut js_State) {
    js_newboolean(J, js_toboolean(J, 1));
}

unsafe extern "C-unwind" fn jsB_Boolean(J: *mut js_State) {
    js_pushboolean(J, js_toboolean(J, 1));
}

unsafe extern "C-unwind" fn Bp_toString(J: *mut js_State) {
    let self_ = js_toobject(J, 0);
    if (*self_).type_ != JS_CBOOLEAN {
        crate::jserror::js_typeerror(J, cstr!("not a boolean"));
    }
    js_pushliteral(J, if (*self_).u.boolean != 0 { cstr!("true") } else { cstr!("false") });
}

unsafe extern "C-unwind" fn Bp_valueOf(J: *mut js_State) {
    let self_ = js_toobject(J, 0);
    if (*self_).type_ != JS_CBOOLEAN {
        crate::jserror::js_typeerror(J, cstr!("not a boolean"));
    }
    js_pushboolean(J, (*self_).u.boolean);
}

#[no_mangle]
pub unsafe extern "C-unwind" fn jsB_initboolean(J: *mut js_State) {
    (*(*J).Boolean_prototype).u.boolean = 0;

    js_pushobject(J, (*J).Boolean_prototype);
    {
        crate::jsbuiltin::jsB_propf(J, cstr!("Boolean.prototype.toString"), Some(Bp_toString), 0);
        crate::jsbuiltin::jsB_propf(J, cstr!("Boolean.prototype.valueOf"), Some(Bp_valueOf), 0);
    }
    js_newcconstructor(J, Some(jsB_Boolean), Some(jsB_new_Boolean), cstr!("Boolean"), 1);
    js_defglobal(J, cstr!("Boolean"), JS_DONTENUM);
}
