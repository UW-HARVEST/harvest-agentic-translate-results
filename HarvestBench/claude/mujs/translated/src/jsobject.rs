//! Translated from jsobject.c — the Object constructor and prototype methods.
#![allow(non_snake_case, non_upper_case_globals)]

use crate::jsrun::*;
use crate::types::*;
use std::os::raw::{c_char, c_int};

macro_rules! cstr {
    ($s:literal) => {
        concat!($s, "\0").as_ptr() as *const c_char
    };
}

unsafe extern "C-unwind" fn jsB_new_Object(J: *mut js_State) {
    if js_isundefined(J, 1) != 0 || js_isnull(J, 1) != 0 {
        crate::jsvalue::js_newobject(J);
    } else {
        js_pushobject(J, js_toobject(J, 1));
    }
}

unsafe extern "C-unwind" fn jsB_Object(J: *mut js_State) {
    if js_isundefined(J, 1) != 0 || js_isnull(J, 1) != 0 {
        crate::jsvalue::js_newobject(J);
    } else {
        js_pushobject(J, js_toobject(J, 1));
    }
}

unsafe extern "C-unwind" fn Op_toString(J: *mut js_State) {
    if js_isundefined(J, 0) != 0 {
        js_pushliteral(J, cstr!("[object Undefined]"));
    } else if js_isnull(J, 0) != 0 {
        js_pushliteral(J, cstr!("[object Null]"));
    } else {
        let self_ = js_toobject(J, 0);
        match (*self_).type_ {
            JS_COBJECT => js_pushliteral(J, cstr!("[object Object]")),
            JS_CARRAY => js_pushliteral(J, cstr!("[object Array]")),
            JS_CFUNCTION => js_pushliteral(J, cstr!("[object Function]")),
            JS_CSCRIPT => js_pushliteral(J, cstr!("[object Function]")),
            JS_CCFUNCTION => js_pushliteral(J, cstr!("[object Function]")),
            JS_CERROR => js_pushliteral(J, cstr!("[object Error]")),
            JS_CBOOLEAN => js_pushliteral(J, cstr!("[object Boolean]")),
            JS_CNUMBER => js_pushliteral(J, cstr!("[object Number]")),
            JS_CSTRING => js_pushliteral(J, cstr!("[object String]")),
            JS_CREGEXP => js_pushliteral(J, cstr!("[object RegExp]")),
            JS_CDATE => js_pushliteral(J, cstr!("[object Date]")),
            JS_CMATH => js_pushliteral(J, cstr!("[object Math]")),
            JS_CJSON => js_pushliteral(J, cstr!("[object JSON]")),
            JS_CARGUMENTS => js_pushliteral(J, cstr!("[object Arguments]")),
            JS_CITERATOR => js_pushliteral(J, cstr!("[object Iterator]")),
            JS_CUSERDATA => {
                js_pushliteral(J, cstr!("[object "));
                js_pushliteral(J, (*self_).u.user.tag);
                crate::jsvalue::js_concat(J);
                js_pushliteral(J, cstr!("]"));
                crate::jsvalue::js_concat(J);
            }
            _ => {}
        }
    }
}

unsafe extern "C-unwind" fn Op_valueOf(J: *mut js_State) {
    js_copy(J, 0);
}

unsafe extern "C-unwind" fn Op_hasOwnProperty(J: *mut js_State) {
    let self_ = js_toobject(J, 0);
    let name = js_tostring(J, 1);
    let ref_: *mut js_Property;
    let mut k: c_int = 0;

    if (*self_).type_ == JS_CSTRING {
        if js_isarrayindex(J, name, &mut k) != 0 && k >= 0 && k < (*self_).u.s.length {
            js_pushboolean(J, 1);
            return;
        }
    }

    if (*self_).type_ == JS_CARRAY && (*self_).u.a.simple != 0 {
        if js_isarrayindex(J, name, &mut k) != 0 && k >= 0 && k < (*self_).u.a.flat_length {
            js_pushboolean(J, 1);
            return;
        }
    }

    ref_ = crate::jsproperty::jsV_getownproperty(J, self_, name);
    js_pushboolean(J, (!ref_.is_null()) as c_int);
}

unsafe extern "C-unwind" fn Op_isPrototypeOf(J: *mut js_State) {
    let self_ = js_toobject(J, 0);
    if js_isobject(J, 1) != 0 {
        let mut V = js_toobject(J, 1);
        loop {
            V = (*V).prototype;
            if V == self_ {
                js_pushboolean(J, 1);
                return;
            }
            if V.is_null() {
                break;
            }
        }
    }
    js_pushboolean(J, 0);
}

unsafe extern "C-unwind" fn Op_propertyIsEnumerable(J: *mut js_State) {
    let self_ = js_toobject(J, 0);
    let name = js_tostring(J, 1);
    let ref_ = crate::jsproperty::jsV_getownproperty(J, self_, name);
    js_pushboolean(J, (!ref_.is_null() && ((*ref_).atts & JS_DONTENUM) == 0) as c_int);
}

unsafe extern "C-unwind" fn O_getPrototypeOf(J: *mut js_State) {
    let obj: *mut js_Object;
    if js_isobject(J, 1) == 0 {
        crate::jserror::js_typeerror(J, cstr!("not an object"));
    }
    obj = js_toobject(J, 1);
    if !(*obj).prototype.is_null() {
        js_pushobject(J, (*obj).prototype);
    } else {
        js_pushnull(J);
    }
}

unsafe extern "C-unwind" fn O_getOwnPropertyDescriptor(J: *mut js_State) {
    let obj: *mut js_Object;
    let ref_: *mut js_Property;
    if js_isobject(J, 1) == 0 {
        crate::jserror::js_typeerror(J, cstr!("not an object"));
    }
    obj = js_toobject(J, 1);
    ref_ = crate::jsproperty::jsV_getproperty(J, obj, js_tostring(J, 2));
    if ref_.is_null() {
        /* TODO: builtin properties (string and array index and length, regexp flags, etc) */
        js_pushundefined(J);
    } else {
        crate::jsvalue::js_newobject(J);
        if (*ref_).getter.is_null() && (*ref_).setter.is_null() {
            js_pushvalue(J, (*ref_).value);
            js_defproperty(J, -2, cstr!("value"), 0);
            js_pushboolean(J, ((*ref_).atts & JS_READONLY == 0) as c_int);
            js_defproperty(J, -2, cstr!("writable"), 0);
        } else {
            if !(*ref_).getter.is_null() {
                js_pushobject(J, (*ref_).getter);
            } else {
                js_pushundefined(J);
            }
            js_defproperty(J, -2, cstr!("get"), 0);
            if !(*ref_).setter.is_null() {
                js_pushobject(J, (*ref_).setter);
            } else {
                js_pushundefined(J);
            }
            js_defproperty(J, -2, cstr!("set"), 0);
        }
        js_pushboolean(J, ((*ref_).atts & JS_DONTENUM == 0) as c_int);
        js_defproperty(J, -2, cstr!("enumerable"), 0);
        js_pushboolean(J, ((*ref_).atts & JS_DONTCONF == 0) as c_int);
        js_defproperty(J, -2, cstr!("configurable"), 0);
    }
}

unsafe fn O_getOwnPropertyNames_walk(J: *mut js_State, ref_: *mut js_Property, mut i: c_int) -> c_int {
    if (*(*ref_).left).level != 0 {
        i = O_getOwnPropertyNames_walk(J, (*ref_).left, i);
    }
    js_pushstring(J, (*ref_).name.as_ptr());
    js_setindex(J, -2, i);
    i += 1;
    if (*(*ref_).right).level != 0 {
        i = O_getOwnPropertyNames_walk(J, (*ref_).right, i);
    }
    i
}

unsafe extern "C-unwind" fn O_getOwnPropertyNames(J: *mut js_State) {
    let obj: *mut js_Object;
    let mut name: [c_char; 32] = [0; 32];
    let mut k: c_int;
    let mut i: c_int;

    if js_isobject(J, 1) == 0 {
        crate::jserror::js_typeerror(J, cstr!("not an object"));
    }
    obj = js_toobject(J, 1);

    crate::jsvalue::js_newarray(J);

    if (*(*obj).properties).level != 0 {
        i = O_getOwnPropertyNames_walk(J, (*obj).properties, 0);
    } else {
        i = 0;
    }

    if (*obj).type_ == JS_CARRAY {
        js_pushliteral(J, cstr!("length"));
        js_setindex(J, -2, i);
        i += 1;
        if (*obj).u.a.simple != 0 {
            k = 0;
            while k < (*obj).u.a.flat_length {
                crate::jsvalue::js_itoa(name.as_mut_ptr(), k);
                js_pushstring(J, name.as_ptr());
                js_setindex(J, -2, i);
                i += 1;
                k += 1;
            }
        }
    }

    if (*obj).type_ == JS_CSTRING {
        js_pushliteral(J, cstr!("length"));
        js_setindex(J, -2, i);
        i += 1;
        k = 0;
        while k < (*obj).u.s.length {
            crate::jsvalue::js_itoa(name.as_mut_ptr(), k);
            js_pushstring(J, name.as_ptr());
            js_setindex(J, -2, i);
            i += 1;
            k += 1;
        }
    }

    if (*obj).type_ == JS_CREGEXP {
        js_pushliteral(J, cstr!("source"));
        js_setindex(J, -2, i);
        i += 1;
        js_pushliteral(J, cstr!("global"));
        js_setindex(J, -2, i);
        i += 1;
        js_pushliteral(J, cstr!("ignoreCase"));
        js_setindex(J, -2, i);
        i += 1;
        js_pushliteral(J, cstr!("multiline"));
        js_setindex(J, -2, i);
        i += 1;
        js_pushliteral(J, cstr!("lastIndex"));
        js_setindex(J, -2, i);
        i += 1;
    }
}

unsafe fn ToPropertyDescriptor(J: *mut js_State, obj: *mut js_Object, name: *const c_char, desc: *mut js_Object) {
    let mut haswritable = 0;
    let mut hasvalue = 0;
    let mut enumerable = 0;
    let mut configurable = 0;
    let mut writable = 0;
    let mut atts = 0;

    js_pushobject(J, obj);
    js_pushobject(J, desc);

    if js_hasproperty(J, -1, cstr!("writable")) != 0 {
        haswritable = 1;
        writable = js_toboolean(J, -1);
        js_pop(J, 1);
    }
    if js_hasproperty(J, -1, cstr!("enumerable")) != 0 {
        enumerable = js_toboolean(J, -1);
        js_pop(J, 1);
    }
    if js_hasproperty(J, -1, cstr!("configurable")) != 0 {
        configurable = js_toboolean(J, -1);
        js_pop(J, 1);
    }
    if js_hasproperty(J, -1, cstr!("value")) != 0 {
        hasvalue = 1;
        js_defproperty(J, -3, name, 0);
    }

    if writable == 0 {
        atts |= JS_READONLY;
    }
    if enumerable == 0 {
        atts |= JS_DONTENUM;
    }
    if configurable == 0 {
        atts |= JS_DONTCONF;
    }

    if js_hasproperty(J, -1, cstr!("get")) != 0 {
        if haswritable != 0 || hasvalue != 0 {
            crate::jserror::js_typeerror(J, cstr!("value/writable and get/set attributes are exclusive"));
        }
    } else {
        js_pushundefined(J);
    }

    if js_hasproperty(J, -2, cstr!("set")) != 0 {
        if haswritable != 0 || hasvalue != 0 {
            crate::jserror::js_typeerror(J, cstr!("value/writable and get/set attributes are exclusive"));
        }
    } else {
        js_pushundefined(J);
    }

    js_defaccessor(J, -4, name, atts);

    js_pop(J, 2);
}

unsafe extern "C-unwind" fn O_defineProperty(J: *mut js_State) {
    if js_isobject(J, 1) == 0 {
        crate::jserror::js_typeerror(J, cstr!("not an object"));
    }
    if js_isobject(J, 3) == 0 {
        crate::jserror::js_typeerror(J, cstr!("not an object"));
    }
    ToPropertyDescriptor(J, js_toobject(J, 1), js_tostring(J, 2), js_toobject(J, 3));
    js_copy(J, 1);
}

unsafe fn O_defineProperties_walk(J: *mut js_State, ref_: *mut js_Property, mut i: c_int) -> c_int {
    if (*(*ref_).left).level != 0 {
        i = O_defineProperties_walk(J, (*ref_).left, i);
    }
    if ((*ref_).atts & JS_DONTENUM) == 0 {
        if (*ref_).value.type_() != JS_TOBJECT {
            crate::jserror::js_typeerror(J, cstr!("not an object"));
        }
        js_pushstring(J, (*ref_).name.as_ptr());
        js_setindex(J, -2, i);
        i += 1;
    }
    if (*(*ref_).right).level != 0 {
        i = O_defineProperties_walk(J, (*ref_).right, i);
    }
    i
}

unsafe fn O_defineProperties_imp(J: *mut js_State, obj: *mut js_Object) {
    let props: *mut js_Object;
    let mut name: *const c_char;
    let mut i: c_int;
    let n: c_int;

    if js_isobject(J, 2) == 0 {
        crate::jserror::js_typeerror(J, cstr!("not an object"));
    }

    props = js_toobject(J, 2);
    if (*(*props).properties).level != 0 {
        crate::jsvalue::js_newarray(J);
        n = O_defineProperties_walk(J, (*props).properties, 0);
        i = 0;
        while i < n {
            js_getindex(J, -1, i);
            name = js_tostring(J, -1);
            if js_hasproperty(J, 2, name) != 0 {
                ToPropertyDescriptor(J, obj, name, js_toobject(J, -1));
                js_pop(J, 1);
            }
            js_pop(J, 1);
            i += 1;
        }
        js_pop(J, 1);
    }
}

unsafe extern "C-unwind" fn O_defineProperties(J: *mut js_State) {
    let obj: *mut js_Object;
    if js_isobject(J, 1) == 0 {
        crate::jserror::js_typeerror(J, cstr!("not an object"));
    }
    obj = js_toobject(J, 1);
    O_defineProperties_imp(J, obj);
    js_copy(J, 1);
}

unsafe extern "C-unwind" fn O_create(J: *mut js_State) {
    let obj: *mut js_Object;
    let mut proto: *mut js_Object = std::ptr::null_mut();

    if js_isobject(J, 1) != 0 {
        proto = js_toobject(J, 1);
    } else if js_isnull(J, 1) != 0 {
        proto = std::ptr::null_mut();
    } else {
        crate::jserror::js_typeerror(J, cstr!("not an object or null"));
    }

    obj = crate::jsproperty::jsV_newobject(J, JS_COBJECT, proto);
    js_pushobject(J, obj);

    if js_isdefined(J, 2) != 0 {
        O_defineProperties_imp(J, obj);
    }
}

unsafe fn O_keys_walk(J: *mut js_State, ref_: *mut js_Property, mut i: c_int) -> c_int {
    if (*(*ref_).left).level != 0 {
        i = O_keys_walk(J, (*ref_).left, i);
    }
    if ((*ref_).atts & JS_DONTENUM) == 0 {
        js_pushstring(J, (*ref_).name.as_ptr());
        js_setindex(J, -2, i);
        i += 1;
    }
    if (*(*ref_).right).level != 0 {
        i = O_keys_walk(J, (*ref_).right, i);
    }
    i
}

unsafe extern "C-unwind" fn O_keys(J: *mut js_State) {
    let obj: *mut js_Object;
    let mut name: [c_char; 32] = [0; 32];
    let mut i: c_int;
    let mut k: c_int;

    if js_isobject(J, 1) == 0 {
        crate::jserror::js_typeerror(J, cstr!("not an object"));
    }
    obj = js_toobject(J, 1);

    crate::jsvalue::js_newarray(J);

    if (*(*obj).properties).level != 0 {
        i = O_keys_walk(J, (*obj).properties, 0);
    } else {
        i = 0;
    }

    if (*obj).type_ == JS_CSTRING {
        k = 0;
        while k < (*obj).u.s.length {
            crate::jsvalue::js_itoa(name.as_mut_ptr(), k);
            js_pushstring(J, name.as_ptr());
            js_setindex(J, -2, i);
            i += 1;
            k += 1;
        }
    }

    if (*obj).type_ == JS_CARRAY && (*obj).u.a.simple != 0 {
        k = 0;
        while k < (*obj).u.a.flat_length {
            crate::jsvalue::js_itoa(name.as_mut_ptr(), k);
            js_pushstring(J, name.as_ptr());
            js_setindex(J, -2, i);
            i += 1;
            k += 1;
        }
    }
}

unsafe extern "C-unwind" fn O_preventExtensions(J: *mut js_State) {
    let obj: *mut js_Object;
    if js_isobject(J, 1) == 0 {
        crate::jserror::js_typeerror(J, cstr!("not an object"));
    }
    obj = js_toobject(J, 1);
    crate::jsrun::jsR_unflattenarray(J, obj);
    (*obj).extensible = 0;
    js_copy(J, 1);
}

unsafe extern "C-unwind" fn O_isExtensible(J: *mut js_State) {
    if js_isobject(J, 1) == 0 {
        crate::jserror::js_typeerror(J, cstr!("not an object"));
    }
    js_pushboolean(J, (*js_toobject(J, 1)).extensible);
}

unsafe fn O_seal_walk(J: *mut js_State, ref_: *mut js_Property) {
    if (*(*ref_).left).level != 0 {
        O_seal_walk(J, (*ref_).left);
    }
    (*ref_).atts |= JS_DONTCONF;
    if (*(*ref_).right).level != 0 {
        O_seal_walk(J, (*ref_).right);
    }
}

unsafe extern "C-unwind" fn O_seal(J: *mut js_State) {
    let obj: *mut js_Object;

    if js_isobject(J, 1) == 0 {
        crate::jserror::js_typeerror(J, cstr!("not an object"));
    }

    obj = js_toobject(J, 1);
    crate::jsrun::jsR_unflattenarray(J, obj);
    (*obj).extensible = 0;

    if (*(*obj).properties).level != 0 {
        O_seal_walk(J, (*obj).properties);
    }

    js_copy(J, 1);
}

unsafe fn O_isSealed_walk(J: *mut js_State, ref_: *mut js_Property) -> c_int {
    if (*(*ref_).left).level != 0 {
        if O_isSealed_walk(J, (*ref_).left) == 0 {
            return 0;
        }
    }
    if ((*ref_).atts & JS_DONTCONF) == 0 {
        return 0;
    }
    if (*(*ref_).right).level != 0 {
        if O_isSealed_walk(J, (*ref_).right) == 0 {
            return 0;
        }
    }
    1
}

unsafe extern "C-unwind" fn O_isSealed(J: *mut js_State) {
    let obj: *mut js_Object;

    if js_isobject(J, 1) == 0 {
        crate::jserror::js_typeerror(J, cstr!("not an object"));
    }

    obj = js_toobject(J, 1);
    if (*obj).extensible != 0 {
        js_pushboolean(J, 0);
        return;
    }

    if (*(*obj).properties).level != 0 {
        js_pushboolean(J, O_isSealed_walk(J, (*obj).properties));
    } else {
        js_pushboolean(J, 1);
    }
}

unsafe fn O_freeze_walk(J: *mut js_State, ref_: *mut js_Property) {
    if (*(*ref_).left).level != 0 {
        O_freeze_walk(J, (*ref_).left);
    }
    (*ref_).atts |= JS_READONLY | JS_DONTCONF;
    if (*(*ref_).right).level != 0 {
        O_freeze_walk(J, (*ref_).right);
    }
}

unsafe extern "C-unwind" fn O_freeze(J: *mut js_State) {
    let obj: *mut js_Object;

    if js_isobject(J, 1) == 0 {
        crate::jserror::js_typeerror(J, cstr!("not an object"));
    }

    obj = js_toobject(J, 1);
    crate::jsrun::jsR_unflattenarray(J, obj);
    (*obj).extensible = 0;

    if (*(*obj).properties).level != 0 {
        O_freeze_walk(J, (*obj).properties);
    }

    js_copy(J, 1);
}

unsafe fn O_isFrozen_walk(J: *mut js_State, ref_: *mut js_Property) -> c_int {
    if (*(*ref_).left).level != 0 {
        if O_isFrozen_walk(J, (*ref_).left) == 0 {
            return 0;
        }
    }
    if ((*ref_).atts & JS_READONLY) == 0 {
        return 0;
    }
    if ((*ref_).atts & JS_DONTCONF) == 0 {
        return 0;
    }
    if (*(*ref_).right).level != 0 {
        if O_isFrozen_walk(J, (*ref_).right) == 0 {
            return 0;
        }
    }
    1
}

unsafe extern "C-unwind" fn O_isFrozen(J: *mut js_State) {
    let obj: *mut js_Object;

    if js_isobject(J, 1) == 0 {
        crate::jserror::js_typeerror(J, cstr!("not an object"));
    }

    obj = js_toobject(J, 1);

    if (*(*obj).properties).level != 0 {
        if O_isFrozen_walk(J, (*obj).properties) == 0 {
            js_pushboolean(J, 0);
            return;
        }
    }

    js_pushboolean(J, ((*obj).extensible == 0) as c_int);
}

#[no_mangle]
pub unsafe extern "C-unwind" fn jsB_initobject(J: *mut js_State) {
    js_pushobject(J, (*J).Object_prototype);
    {
        crate::jsbuiltin::jsB_propf(J, cstr!("Object.prototype.toString"), Some(Op_toString), 0);
        crate::jsbuiltin::jsB_propf(J, cstr!("Object.prototype.toLocaleString"), Some(Op_toString), 0);
        crate::jsbuiltin::jsB_propf(J, cstr!("Object.prototype.valueOf"), Some(Op_valueOf), 0);
        crate::jsbuiltin::jsB_propf(J, cstr!("Object.prototype.hasOwnProperty"), Some(Op_hasOwnProperty), 1);
        crate::jsbuiltin::jsB_propf(J, cstr!("Object.prototype.isPrototypeOf"), Some(Op_isPrototypeOf), 1);
        crate::jsbuiltin::jsB_propf(J, cstr!("Object.prototype.propertyIsEnumerable"), Some(Op_propertyIsEnumerable), 1);
    }
    crate::jsvalue::js_newcconstructor(J, Some(jsB_Object), Some(jsB_new_Object), cstr!("Object"), 1);
    {
        /* ES5 */
        crate::jsbuiltin::jsB_propf(J, cstr!("Object.getPrototypeOf"), Some(O_getPrototypeOf), 1);
        crate::jsbuiltin::jsB_propf(J, cstr!("Object.getOwnPropertyDescriptor"), Some(O_getOwnPropertyDescriptor), 2);
        crate::jsbuiltin::jsB_propf(J, cstr!("Object.getOwnPropertyNames"), Some(O_getOwnPropertyNames), 1);
        crate::jsbuiltin::jsB_propf(J, cstr!("Object.create"), Some(O_create), 2);
        crate::jsbuiltin::jsB_propf(J, cstr!("Object.defineProperty"), Some(O_defineProperty), 3);
        crate::jsbuiltin::jsB_propf(J, cstr!("Object.defineProperties"), Some(O_defineProperties), 2);
        crate::jsbuiltin::jsB_propf(J, cstr!("Object.seal"), Some(O_seal), 1);
        crate::jsbuiltin::jsB_propf(J, cstr!("Object.freeze"), Some(O_freeze), 1);
        crate::jsbuiltin::jsB_propf(J, cstr!("Object.preventExtensions"), Some(O_preventExtensions), 1);
        crate::jsbuiltin::jsB_propf(J, cstr!("Object.isSealed"), Some(O_isSealed), 1);
        crate::jsbuiltin::jsB_propf(J, cstr!("Object.isFrozen"), Some(O_isFrozen), 1);
        crate::jsbuiltin::jsB_propf(J, cstr!("Object.isExtensible"), Some(O_isExtensible), 1);
        crate::jsbuiltin::jsB_propf(J, cstr!("Object.keys"), Some(O_keys), 1);
    }
    js_defglobal(J, cstr!("Object"), JS_DONTENUM);
}
