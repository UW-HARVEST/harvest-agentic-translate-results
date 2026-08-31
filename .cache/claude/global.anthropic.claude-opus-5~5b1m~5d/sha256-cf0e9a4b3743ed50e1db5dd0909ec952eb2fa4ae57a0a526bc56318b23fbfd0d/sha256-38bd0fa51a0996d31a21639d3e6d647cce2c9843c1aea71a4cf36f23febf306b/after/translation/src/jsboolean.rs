//! Translation of jsboolean.c

use crate::*;

unsafe extern "C" fn jsB_new_Boolean(J: *mut js_State) {
    js_newboolean(J, js_toboolean(J, 1));
}

unsafe extern "C" fn jsB_Boolean(J: *mut js_State) {
    js_pushboolean(J, js_toboolean(J, 1));
}

unsafe extern "C" fn Bp_toString(J: *mut js_State) {
    let self_: *mut js_Object = js_toobject(J, 0);
    if (*self_).type_ != JS_CBOOLEAN {
        js_typeerror!(J, "not a boolean");
    }
    js_pushliteral(
        J,
        if (*self_).u.boolean != 0 {
            cs!("true")
        } else {
            cs!("false")
        },
    );
}

unsafe extern "C" fn Bp_valueOf(J: *mut js_State) {
    let self_: *mut js_Object = js_toobject(J, 0);
    if (*self_).type_ != JS_CBOOLEAN {
        js_typeerror!(J, "not a boolean");
    }
    js_pushboolean(J, (*self_).u.boolean);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsB_initboolean(J: *mut js_State) {
    (*(*J).Boolean_prototype).u.boolean = 0;

    js_pushobject(J, (*J).Boolean_prototype);
    {
        jsB_propf(J, cs!("Boolean.prototype.toString"), Some(Bp_toString), 0);
        jsB_propf(J, cs!("Boolean.prototype.valueOf"), Some(Bp_valueOf), 0);
    }
    js_newcconstructor(
        J,
        Some(jsB_Boolean),
        Some(jsB_new_Boolean),
        cs!("Boolean"),
        1,
    );
    js_defglobal(J, cs!("Boolean"), JS_DONTENUM);
}
