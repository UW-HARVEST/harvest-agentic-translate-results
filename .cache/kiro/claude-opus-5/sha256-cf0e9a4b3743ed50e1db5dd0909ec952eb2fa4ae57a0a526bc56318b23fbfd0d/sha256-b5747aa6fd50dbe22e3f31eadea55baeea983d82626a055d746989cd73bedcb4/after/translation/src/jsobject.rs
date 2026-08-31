// Translation of c_src/src/jsobject.c
#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals, dead_code)]

use crate::common::*;
use crate::jsproperty::{jsV_getownproperty, jsV_getproperty, jsV_newobject};
use crate::jsrun::*;
use crate::jsbuiltin::jsB_propf;
use crate::jsvalue::{js_concat, js_itoa, js_newarray, js_newcconstructor, js_newobject};
use crate::types::*;
use crate::js_typeerror;
use std::ffi::{c_char, c_int};

unsafe extern "C-unwind" fn jsB_new_Object(J: *mut js_State) {
    unsafe {
        if js_isundefined(J, 1) != 0 || js_isnull(J, 1) != 0 {
            js_newobject(J);
        } else {
            js_pushobject(J, js_toobject(J, 1));
        }
    }
}

unsafe extern "C-unwind" fn jsB_Object(J: *mut js_State) {
    unsafe {
        if js_isundefined(J, 1) != 0 || js_isnull(J, 1) != 0 {
            js_newobject(J);
        } else {
            js_pushobject(J, js_toobject(J, 1));
        }
    }
}

unsafe extern "C-unwind" fn Op_toString(J: *mut js_State) {
    unsafe {
        if js_isundefined(J, 0) != 0 {
            js_pushliteral(J, c"[object Undefined]".as_ptr());
        } else if js_isnull(J, 0) != 0 {
            js_pushliteral(J, c"[object Null]".as_ptr());
        } else {
            let self_: *mut js_Object = js_toobject(J, 0);
            match (*self_).type_ {
                JS_COBJECT => js_pushliteral(J, c"[object Object]".as_ptr()),
                JS_CARRAY => js_pushliteral(J, c"[object Array]".as_ptr()),
                JS_CFUNCTION => js_pushliteral(J, c"[object Function]".as_ptr()),
                JS_CSCRIPT => js_pushliteral(J, c"[object Function]".as_ptr()),
                JS_CCFUNCTION => js_pushliteral(J, c"[object Function]".as_ptr()),
                JS_CERROR => js_pushliteral(J, c"[object Error]".as_ptr()),
                JS_CBOOLEAN => js_pushliteral(J, c"[object Boolean]".as_ptr()),
                JS_CNUMBER => js_pushliteral(J, c"[object Number]".as_ptr()),
                JS_CSTRING => js_pushliteral(J, c"[object String]".as_ptr()),
                JS_CREGEXP => js_pushliteral(J, c"[object RegExp]".as_ptr()),
                JS_CDATE => js_pushliteral(J, c"[object Date]".as_ptr()),
                JS_CMATH => js_pushliteral(J, c"[object Math]".as_ptr()),
                JS_CJSON => js_pushliteral(J, c"[object JSON]".as_ptr()),
                JS_CARGUMENTS => js_pushliteral(J, c"[object Arguments]".as_ptr()),
                JS_CITERATOR => js_pushliteral(J, c"[object Iterator]".as_ptr()),
                JS_CUSERDATA => {
                    js_pushliteral(J, c"[object ".as_ptr());
                    js_pushliteral(J, (*self_).u.user.tag);
                    js_concat(J);
                    js_pushliteral(J, c"]".as_ptr());
                    js_concat(J);
                }
                _ => {}
            }
        }
    }
}

unsafe extern "C-unwind" fn Op_valueOf(J: *mut js_State) {
    unsafe {
        js_copy(J, 0);
    }
}

unsafe extern "C-unwind" fn Op_hasOwnProperty(J: *mut js_State) {
    unsafe {
        let self_: *mut js_Object = js_toobject(J, 0);
        let name: *const c_char = js_tostring(J, 1);
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

        ref_ = jsV_getownproperty(J, self_, name);
        js_pushboolean(J, (ref_ != std::ptr::null_mut()) as c_int);
    }
}

unsafe extern "C-unwind" fn Op_isPrototypeOf(J: *mut js_State) {
    unsafe {
        let self_: *mut js_Object = js_toobject(J, 0);
        if js_isobject(J, 1) != 0 {
            let mut V: *mut js_Object = js_toobject(J, 1);
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
}

unsafe extern "C-unwind" fn Op_propertyIsEnumerable(J: *mut js_State) {
    unsafe {
        let self_: *mut js_Object = js_toobject(J, 0);
        let name: *const c_char = js_tostring(J, 1);
        let ref_: *mut js_Property = jsV_getownproperty(J, self_, name);
        js_pushboolean(
            J,
            (!ref_.is_null() && ((*ref_).atts & JS_DONTENUM) == 0) as c_int,
        );
    }
}

unsafe extern "C-unwind" fn O_getPrototypeOf(J: *mut js_State) {
    unsafe {
        let obj: *mut js_Object;
        if js_isobject(J, 1) == 0 {
            js_typeerror!(J, c"not an object");
        }
        obj = js_toobject(J, 1);
        if !(*obj).prototype.is_null() {
            js_pushobject(J, (*obj).prototype);
        } else {
            js_pushnull(J);
        }
    }
}

unsafe extern "C-unwind" fn O_getOwnPropertyDescriptor(J: *mut js_State) {
    unsafe {
        let obj: *mut js_Object;
        let ref_: *mut js_Property;
        if js_isobject(J, 1) == 0 {
            js_typeerror!(J, c"not an object");
        }
        obj = js_toobject(J, 1);
        ref_ = jsV_getproperty(J, obj, js_tostring(J, 2));
        if ref_.is_null() {
            /* TODO: builtin properties (string and array index and length, regexp flags, etc) */
            js_pushundefined(J);
        } else {
            js_newobject(J);
            if (*ref_).getter.is_null() && (*ref_).setter.is_null() {
                js_pushvalue(J, (*ref_).value);
                js_defproperty(J, -2, c"value".as_ptr(), 0);
                js_pushboolean(J, ((*ref_).atts & JS_READONLY == 0) as c_int);
                js_defproperty(J, -2, c"writable".as_ptr(), 0);
            } else {
                if !(*ref_).getter.is_null() {
                    js_pushobject(J, (*ref_).getter);
                } else {
                    js_pushundefined(J);
                }
                js_defproperty(J, -2, c"get".as_ptr(), 0);
                if !(*ref_).setter.is_null() {
                    js_pushobject(J, (*ref_).setter);
                } else {
                    js_pushundefined(J);
                }
                js_defproperty(J, -2, c"set".as_ptr(), 0);
            }
            js_pushboolean(J, ((*ref_).atts & JS_DONTENUM == 0) as c_int);
            js_defproperty(J, -2, c"enumerable".as_ptr(), 0);
            js_pushboolean(J, ((*ref_).atts & JS_DONTCONF == 0) as c_int);
            js_defproperty(J, -2, c"configurable".as_ptr(), 0);
        }
    }
}

unsafe fn O_getOwnPropertyNames_walk(J: *mut js_State, ref_: *mut js_Property, mut i: c_int) -> c_int {
    unsafe {
        if (*(*ref_).left).level != 0 {
            i = O_getOwnPropertyNames_walk(J, (*ref_).left, i);
        }
        js_pushstring(J, (&raw const (*ref_).name) as *const c_char);
        js_setindex(J, -2, i);
        i += 1;
        if (*(*ref_).right).level != 0 {
            i = O_getOwnPropertyNames_walk(J, (*ref_).right, i);
        }
        i
    }
}

unsafe extern "C-unwind" fn O_getOwnPropertyNames(J: *mut js_State) {
    unsafe {
        let obj: *mut js_Object;
        let mut name: [c_char; 32] = [0; 32];
        let mut k: c_int;

        if js_isobject(J, 1) == 0 {
            js_typeerror!(J, c"not an object");
        }
        obj = js_toobject(J, 1);

        js_newarray(J);

        let mut i = if (*(*obj).properties).level != 0 {
            O_getOwnPropertyNames_walk(J, (*obj).properties, 0)
        } else {
            0
        };

        if (*obj).type_ == JS_CARRAY {
            js_pushliteral(J, c"length".as_ptr());
            js_setindex(J, -2, i);
            i += 1;
            if (*obj).u.a.simple != 0 {
                k = 0;
                while k < (*obj).u.a.flat_length {
                    js_itoa(name.as_mut_ptr(), k);
                    js_pushstring(J, name.as_ptr());
                    js_setindex(J, -2, i);
                    i += 1;
                    k += 1;
                }
            }
        }

        if (*obj).type_ == JS_CSTRING {
            js_pushliteral(J, c"length".as_ptr());
            js_setindex(J, -2, i);
            i += 1;
            k = 0;
            while k < (*obj).u.s.length {
                js_itoa(name.as_mut_ptr(), k);
                js_pushstring(J, name.as_ptr());
                js_setindex(J, -2, i);
                i += 1;
                k += 1;
            }
        }

        if (*obj).type_ == JS_CREGEXP {
            js_pushliteral(J, c"source".as_ptr());
            js_setindex(J, -2, i);
            i += 1;
            js_pushliteral(J, c"global".as_ptr());
            js_setindex(J, -2, i);
            i += 1;
            js_pushliteral(J, c"ignoreCase".as_ptr());
            js_setindex(J, -2, i);
            i += 1;
            js_pushliteral(J, c"multiline".as_ptr());
            js_setindex(J, -2, i);
            i += 1;
            js_pushliteral(J, c"lastIndex".as_ptr());
            js_setindex(J, -2, i);
            i += 1;
        }
    }
}

unsafe fn ToPropertyDescriptor(
    J: *mut js_State,
    obj: *mut js_Object,
    name: *const c_char,
    desc: *mut js_Object,
) {
    unsafe {
        let mut haswritable: c_int = 0;
        let mut hasvalue: c_int = 0;
        let mut enumerable: c_int = 0;
        let mut configurable: c_int = 0;
        let mut writable: c_int = 0;
        let mut atts: c_int = 0;

        js_pushobject(J, obj);
        js_pushobject(J, desc);

        if js_hasproperty(J, -1, c"writable".as_ptr()) != 0 {
            haswritable = 1;
            writable = js_toboolean(J, -1);
            js_pop(J, 1);
        }
        if js_hasproperty(J, -1, c"enumerable".as_ptr()) != 0 {
            enumerable = js_toboolean(J, -1);
            js_pop(J, 1);
        }
        if js_hasproperty(J, -1, c"configurable".as_ptr()) != 0 {
            configurable = js_toboolean(J, -1);
            js_pop(J, 1);
        }
        if js_hasproperty(J, -1, c"value".as_ptr()) != 0 {
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

        if js_hasproperty(J, -1, c"get".as_ptr()) != 0 {
            if haswritable != 0 || hasvalue != 0 {
                js_typeerror!(J, c"value/writable and get/set attributes are exclusive");
            }
        } else {
            js_pushundefined(J);
        }

        if js_hasproperty(J, -2, c"set".as_ptr()) != 0 {
            if haswritable != 0 || hasvalue != 0 {
                js_typeerror!(J, c"value/writable and get/set attributes are exclusive");
            }
        } else {
            js_pushundefined(J);
        }

        js_defaccessor(J, -4, name, atts);

        js_pop(J, 2);
    }
}

unsafe extern "C-unwind" fn O_defineProperty(J: *mut js_State) {
    unsafe {
        if js_isobject(J, 1) == 0 {
            js_typeerror!(J, c"not an object");
        }
        if js_isobject(J, 3) == 0 {
            js_typeerror!(J, c"not an object");
        }
        ToPropertyDescriptor(J, js_toobject(J, 1), js_tostring(J, 2), js_toobject(J, 3));
        js_copy(J, 1);
    }
}

unsafe fn O_defineProperties_walk(J: *mut js_State, ref_: *mut js_Property, mut i: c_int) -> c_int {
    unsafe {
        if (*(*ref_).left).level != 0 {
            i = O_defineProperties_walk(J, (*ref_).left, i);
        }
        if ((*ref_).atts & JS_DONTENUM) == 0 {
            if (*ref_).value.t.type_ as c_int != JS_TOBJECT {
                js_typeerror!(J, c"not an object");
            }
            js_pushstring(J, (&raw const (*ref_).name) as *const c_char);
            js_setindex(J, -2, i);
            i += 1;
        }
        if (*(*ref_).right).level != 0 {
            i = O_defineProperties_walk(J, (*ref_).right, i);
        }
        i
    }
}

unsafe fn O_defineProperties_imp(J: *mut js_State, obj: *mut js_Object) {
    unsafe {
        let props: *mut js_Object;
        let mut i: c_int;
        let n: c_int;

        if js_isobject(J, 2) == 0 {
            js_typeerror!(J, c"not an object");
        }

        props = js_toobject(J, 2);
        if (*(*props).properties).level != 0 {
            js_newarray(J);
            n = O_defineProperties_walk(J, (*props).properties, 0);
            i = 0;
            while i < n {
                js_getindex(J, -1, i);
                let name = js_tostring(J, -1);
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
}

unsafe extern "C-unwind" fn O_defineProperties(J: *mut js_State) {
    unsafe {
        let obj: *mut js_Object;
        if js_isobject(J, 1) == 0 {
            js_typeerror!(J, c"not an object");
        }
        obj = js_toobject(J, 1);
        O_defineProperties_imp(J, obj);
        js_copy(J, 1);
    }
}

unsafe extern "C-unwind" fn O_create(J: *mut js_State) {
    unsafe {
        let obj: *mut js_Object;
        let proto: *mut js_Object;

        if js_isobject(J, 1) != 0 {
            proto = js_toobject(J, 1);
        } else if js_isnull(J, 1) != 0 {
            proto = std::ptr::null_mut();
        } else {
            js_typeerror!(J, c"not an object or null");
        }

        obj = jsV_newobject(J, JS_COBJECT, proto);
        js_pushobject(J, obj);

        if js_isdefined(J, 2) != 0 {
            O_defineProperties_imp(J, obj);
        }
    }
}

unsafe fn O_keys_walk(J: *mut js_State, ref_: *mut js_Property, mut i: c_int) -> c_int {
    unsafe {
        if (*(*ref_).left).level != 0 {
            i = O_keys_walk(J, (*ref_).left, i);
        }
        if ((*ref_).atts & JS_DONTENUM) == 0 {
            js_pushstring(J, (&raw const (*ref_).name) as *const c_char);
            js_setindex(J, -2, i);
            i += 1;
        }
        if (*(*ref_).right).level != 0 {
            i = O_keys_walk(J, (*ref_).right, i);
        }
        i
    }
}

unsafe extern "C-unwind" fn O_keys(J: *mut js_State) {
    unsafe {
        let obj: *mut js_Object;
        let mut name: [c_char; 32] = [0; 32];
        let mut k: c_int;

        if js_isobject(J, 1) == 0 {
            js_typeerror!(J, c"not an object");
        }
        obj = js_toobject(J, 1);

        js_newarray(J);

        let mut i = if (*(*obj).properties).level != 0 {
            O_keys_walk(J, (*obj).properties, 0)
        } else {
            0
        };

        if (*obj).type_ == JS_CSTRING {
            k = 0;
            while k < (*obj).u.s.length {
                js_itoa(name.as_mut_ptr(), k);
                js_pushstring(J, name.as_ptr());
                js_setindex(J, -2, i);
                i += 1;
                k += 1;
            }
        }

        if (*obj).type_ == JS_CARRAY && (*obj).u.a.simple != 0 {
            k = 0;
            while k < (*obj).u.a.flat_length {
                js_itoa(name.as_mut_ptr(), k);
                js_pushstring(J, name.as_ptr());
                js_setindex(J, -2, i);
                i += 1;
                k += 1;
            }
        }
    }
}

unsafe extern "C-unwind" fn O_preventExtensions(J: *mut js_State) {
    unsafe {
        let obj: *mut js_Object;
        if js_isobject(J, 1) == 0 {
            js_typeerror!(J, c"not an object");
        }
        obj = js_toobject(J, 1);
        jsR_unflattenarray(J, obj);
        (*obj).extensible = 0;
        js_copy(J, 1);
    }
}

unsafe extern "C-unwind" fn O_isExtensible(J: *mut js_State) {
    unsafe {
        if js_isobject(J, 1) == 0 {
            js_typeerror!(J, c"not an object");
        }
        js_pushboolean(J, (*js_toobject(J, 1)).extensible);
    }
}

unsafe fn O_seal_walk(J: *mut js_State, ref_: *mut js_Property) {
    unsafe {
        if (*(*ref_).left).level != 0 {
            O_seal_walk(J, (*ref_).left);
        }
        (*ref_).atts |= JS_DONTCONF;
        if (*(*ref_).right).level != 0 {
            O_seal_walk(J, (*ref_).right);
        }
    }
}

unsafe extern "C-unwind" fn O_seal(J: *mut js_State) {
    unsafe {
        let obj: *mut js_Object;

        if js_isobject(J, 1) == 0 {
            js_typeerror!(J, c"not an object");
        }

        obj = js_toobject(J, 1);
        jsR_unflattenarray(J, obj);
        (*obj).extensible = 0;

        if (*(*obj).properties).level != 0 {
            O_seal_walk(J, (*obj).properties);
        }

        js_copy(J, 1);
    }
}

unsafe fn O_isSealed_walk(J: *mut js_State, ref_: *mut js_Property) -> c_int {
    unsafe {
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
}

unsafe extern "C-unwind" fn O_isSealed(J: *mut js_State) {
    unsafe {
        let obj: *mut js_Object;

        if js_isobject(J, 1) == 0 {
            js_typeerror!(J, c"not an object");
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
}

unsafe fn O_freeze_walk(J: *mut js_State, ref_: *mut js_Property) {
    unsafe {
        if (*(*ref_).left).level != 0 {
            O_freeze_walk(J, (*ref_).left);
        }
        (*ref_).atts |= JS_READONLY | JS_DONTCONF;
        if (*(*ref_).right).level != 0 {
            O_freeze_walk(J, (*ref_).right);
        }
    }
}

unsafe extern "C-unwind" fn O_freeze(J: *mut js_State) {
    unsafe {
        let obj: *mut js_Object;

        if js_isobject(J, 1) == 0 {
            js_typeerror!(J, c"not an object");
        }

        obj = js_toobject(J, 1);
        jsR_unflattenarray(J, obj);
        (*obj).extensible = 0;

        if (*(*obj).properties).level != 0 {
            O_freeze_walk(J, (*obj).properties);
        }

        js_copy(J, 1);
    }
}

unsafe fn O_isFrozen_walk(J: *mut js_State, ref_: *mut js_Property) -> c_int {
    unsafe {
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
}

unsafe extern "C-unwind" fn O_isFrozen(J: *mut js_State) {
    unsafe {
        let obj: *mut js_Object;

        if js_isobject(J, 1) == 0 {
            js_typeerror!(J, c"not an object");
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
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsB_initobject(J: *mut js_State) {
    unsafe {
        js_pushobject(J, (*J).Object_prototype);
        {
            jsB_propf(J, c"Object.prototype.toString".as_ptr(), Some(Op_toString), 0);
            jsB_propf(J, c"Object.prototype.toLocaleString".as_ptr(), Some(Op_toString), 0);
            jsB_propf(J, c"Object.prototype.valueOf".as_ptr(), Some(Op_valueOf), 0);
            jsB_propf(J, c"Object.prototype.hasOwnProperty".as_ptr(), Some(Op_hasOwnProperty), 1);
            jsB_propf(J, c"Object.prototype.isPrototypeOf".as_ptr(), Some(Op_isPrototypeOf), 1);
            jsB_propf(J, c"Object.prototype.propertyIsEnumerable".as_ptr(), Some(Op_propertyIsEnumerable), 1);
        }
        js_newcconstructor(J, Some(jsB_Object), Some(jsB_new_Object), c"Object".as_ptr(), 1);
        {
            /* ES5 */
            jsB_propf(J, c"Object.getPrototypeOf".as_ptr(), Some(O_getPrototypeOf), 1);
            jsB_propf(J, c"Object.getOwnPropertyDescriptor".as_ptr(), Some(O_getOwnPropertyDescriptor), 2);
            jsB_propf(J, c"Object.getOwnPropertyNames".as_ptr(), Some(O_getOwnPropertyNames), 1);
            jsB_propf(J, c"Object.create".as_ptr(), Some(O_create), 2);
            jsB_propf(J, c"Object.defineProperty".as_ptr(), Some(O_defineProperty), 3);
            jsB_propf(J, c"Object.defineProperties".as_ptr(), Some(O_defineProperties), 2);
            jsB_propf(J, c"Object.seal".as_ptr(), Some(O_seal), 1);
            jsB_propf(J, c"Object.freeze".as_ptr(), Some(O_freeze), 1);
            jsB_propf(J, c"Object.preventExtensions".as_ptr(), Some(O_preventExtensions), 1);
            jsB_propf(J, c"Object.isSealed".as_ptr(), Some(O_isSealed), 1);
            jsB_propf(J, c"Object.isFrozen".as_ptr(), Some(O_isFrozen), 1);
            jsB_propf(J, c"Object.isExtensible".as_ptr(), Some(O_isExtensible), 1);
            jsB_propf(J, c"Object.keys".as_ptr(), Some(O_keys), 1);
        }
        js_defglobal(J, c"Object".as_ptr(), JS_DONTENUM);
    }
}
