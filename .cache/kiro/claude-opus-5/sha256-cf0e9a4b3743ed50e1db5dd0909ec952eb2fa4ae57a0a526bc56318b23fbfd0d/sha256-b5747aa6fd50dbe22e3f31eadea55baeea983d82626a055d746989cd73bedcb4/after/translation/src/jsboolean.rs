#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals, dead_code)]
use crate::common::*;
use crate::jsbuiltin::jsB_propf;
use crate::jsrun::{
    js_defglobal, js_pushboolean, js_pushliteral, js_pushobject, js_toboolean, js_toobject,
};
use crate::jsvalue::{js_newboolean, js_newcconstructor};
use crate::types::*;
use crate::js_typeerror;
use std::ffi::c_int;

unsafe extern "C-unwind" fn jsB_new_Boolean(J: *mut js_State) {
    unsafe {
        js_newboolean(J, js_toboolean(J, 1));
    }
}

unsafe extern "C-unwind" fn jsB_Boolean(J: *mut js_State) {
    unsafe {
        js_pushboolean(J, js_toboolean(J, 1));
    }
}

unsafe extern "C-unwind" fn Bp_toString(J: *mut js_State) {
    unsafe {
        let self_ = js_toobject(J, 0);
        if (*self_).type_ as c_int != JS_CBOOLEAN {
            js_typeerror!(J, c"not a boolean");
        }
        js_pushliteral(
            J,
            if (*self_).u.boolean != 0 {
                c"true".as_ptr()
            } else {
                c"false".as_ptr()
            },
        );
    }
}

unsafe extern "C-unwind" fn Bp_valueOf(J: *mut js_State) {
    unsafe {
        let self_ = js_toobject(J, 0);
        if (*self_).type_ as c_int != JS_CBOOLEAN {
            js_typeerror!(J, c"not a boolean");
        }
        js_pushboolean(J, (*self_).u.boolean);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsB_initboolean(J: *mut js_State) {
    unsafe {
        (*(*J).Boolean_prototype).u.boolean = 0;

        js_pushobject(J, (*J).Boolean_prototype);
        {
            jsB_propf(J, c"Boolean.prototype.toString".as_ptr(), Some(Bp_toString), 0);
            jsB_propf(J, c"Boolean.prototype.valueOf".as_ptr(), Some(Bp_valueOf), 0);
        }
        js_newcconstructor(
            J,
            Some(jsB_Boolean),
            Some(jsB_new_Boolean),
            c"Boolean".as_ptr(),
            1,
        );
        js_defglobal(J, c"Boolean".as_ptr(), JS_DONTENUM);
    }
}
