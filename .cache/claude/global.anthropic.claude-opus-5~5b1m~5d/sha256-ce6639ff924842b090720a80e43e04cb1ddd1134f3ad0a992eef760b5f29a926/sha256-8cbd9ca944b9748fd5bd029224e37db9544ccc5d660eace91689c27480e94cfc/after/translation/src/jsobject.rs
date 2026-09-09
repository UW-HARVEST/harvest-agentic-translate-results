//! Translation of jsobject.c

use crate::*;

unsafe extern "C" fn jsB_new_Object(J: *mut js_State) {
    if js_isundefined(J, 1) != 0 || js_isnull(J, 1) != 0 {
        js_newobject(J);
    } else {
        js_pushobject(J, js_toobject(J, 1));
    }
}

unsafe extern "C" fn jsB_Object(J: *mut js_State) {
    if js_isundefined(J, 1) != 0 || js_isnull(J, 1) != 0 {
        js_newobject(J);
    } else {
        js_pushobject(J, js_toobject(J, 1));
    }
}

unsafe extern "C" fn Op_toString(J: *mut js_State) {
    if js_isundefined(J, 0) != 0 {
        js_pushliteral(J, cs!("[object Undefined]"));
    } else if js_isnull(J, 0) != 0 {
        js_pushliteral(J, cs!("[object Null]"));
    } else {
        let selfp: *mut js_Object = js_toobject(J, 0);
        match (*selfp).type_ {
            JS_COBJECT => js_pushliteral(J, cs!("[object Object]")),
            JS_CARRAY => js_pushliteral(J, cs!("[object Array]")),
            JS_CFUNCTION => js_pushliteral(J, cs!("[object Function]")),
            JS_CSCRIPT => js_pushliteral(J, cs!("[object Function]")),
            JS_CCFUNCTION => js_pushliteral(J, cs!("[object Function]")),
            JS_CERROR => js_pushliteral(J, cs!("[object Error]")),
            JS_CBOOLEAN => js_pushliteral(J, cs!("[object Boolean]")),
            JS_CNUMBER => js_pushliteral(J, cs!("[object Number]")),
            JS_CSTRING => js_pushliteral(J, cs!("[object String]")),
            JS_CREGEXP => js_pushliteral(J, cs!("[object RegExp]")),
            JS_CDATE => js_pushliteral(J, cs!("[object Date]")),
            JS_CMATH => js_pushliteral(J, cs!("[object Math]")),
            JS_CJSON => js_pushliteral(J, cs!("[object JSON]")),
            JS_CARGUMENTS => js_pushliteral(J, cs!("[object Arguments]")),
            JS_CITERATOR => js_pushliteral(J, cs!("[object Iterator]")),
            JS_CUSERDATA => {
                js_pushliteral(J, cs!("[object "));
                js_pushliteral(J, (*selfp).u.user.tag);
                js_concat(J);
                js_pushliteral(J, cs!("]"));
                js_concat(J);
            }
            _ => {}
        }
    }
}

unsafe extern "C" fn Op_valueOf(J: *mut js_State) {
    js_copy(J, 0);
}

unsafe extern "C" fn Op_hasOwnProperty(J: *mut js_State) {
    let selfp: *mut js_Object = js_toobject(J, 0);
    let name: *const c_char = js_tostring(J, 1);
    let refp: *mut js_Property;
    let mut k: c_int = 0;

    if (*selfp).type_ == JS_CSTRING {
        if js_isarrayindex(J, name, &mut k) != 0 && k >= 0 && k < (*selfp).u.s.length {
            js_pushboolean(J, 1);
            return;
        }
    }

    if (*selfp).type_ == JS_CARRAY && (*selfp).u.a.simple != 0 {
        if js_isarrayindex(J, name, &mut k) != 0 && k >= 0 && k < (*selfp).u.a.flat_length {
            js_pushboolean(J, 1);
            return;
        }
    }

    refp = jsV_getownproperty(J, selfp, name);
    js_pushboolean(J, (refp != null_mut()) as c_int);
}

unsafe extern "C" fn Op_isPrototypeOf(J: *mut js_State) {
    let selfp: *mut js_Object = js_toobject(J, 0);
    if js_isobject(J, 1) != 0 {
        let mut V: *mut js_Object = js_toobject(J, 1);
        loop {
            V = (*V).prototype;
            if V == selfp {
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

unsafe extern "C" fn Op_propertyIsEnumerable(J: *mut js_State) {
    let selfp: *mut js_Object = js_toobject(J, 0);
    let name: *const c_char = js_tostring(J, 1);
    let refp: *mut js_Property = jsV_getownproperty(J, selfp, name);
    js_pushboolean(
        J,
        (!refp.is_null() && ((*refp).atts & JS_DONTENUM) == 0) as c_int,
    );
}

unsafe extern "C" fn O_getPrototypeOf(J: *mut js_State) {
    let obj: *mut js_Object;
    if js_isobject(J, 1) == 0 {
        js_typeerror!(J, "not an object");
    }
    obj = js_toobject(J, 1);
    if !(*obj).prototype.is_null() {
        js_pushobject(J, (*obj).prototype);
    } else {
        js_pushnull(J);
    }
}

unsafe extern "C" fn O_getOwnPropertyDescriptor(J: *mut js_State) {
    let obj: *mut js_Object;
    let refp: *mut js_Property;
    if js_isobject(J, 1) == 0 {
        js_typeerror!(J, "not an object");
    }
    obj = js_toobject(J, 1);
    refp = jsV_getproperty(J, obj, js_tostring(J, 2));
    if refp.is_null() {
        /* TODO: builtin properties (string and array index and length, regexp flags, etc) */
        js_pushundefined(J);
    } else {
        js_newobject(J);
        if (*refp).getter.is_null() && (*refp).setter.is_null() {
            js_pushvalue(J, (*refp).value);
            js_defproperty(J, -2, cs!("value"), 0);
            js_pushboolean(J, (((*refp).atts & JS_READONLY) == 0) as c_int);
            js_defproperty(J, -2, cs!("writable"), 0);
        } else {
            if !(*refp).getter.is_null() {
                js_pushobject(J, (*refp).getter);
            } else {
                js_pushundefined(J);
            }
            js_defproperty(J, -2, cs!("get"), 0);
            if !(*refp).setter.is_null() {
                js_pushobject(J, (*refp).setter);
            } else {
                js_pushundefined(J);
            }
            js_defproperty(J, -2, cs!("set"), 0);
        }
        js_pushboolean(J, (((*refp).atts & JS_DONTENUM) == 0) as c_int);
        js_defproperty(J, -2, cs!("enumerable"), 0);
        js_pushboolean(J, (((*refp).atts & JS_DONTCONF) == 0) as c_int);
        js_defproperty(J, -2, cs!("configurable"), 0);
    }
}

unsafe fn O_getOwnPropertyNames_walk(
    J: *mut js_State,
    refp: *mut js_Property,
    i: c_int,
) -> c_int {
    let mut i = i;
    if (*(*refp).left).level != 0 {
        i = O_getOwnPropertyNames_walk(J, (*refp).left, i);
    }
    js_pushstring(J, propname(refp) as *const c_char);
    js_setindex(J, -2, i);
    i += 1;
    if (*(*refp).right).level != 0 {
        i = O_getOwnPropertyNames_walk(J, (*refp).right, i);
    }
    i
}

unsafe extern "C" fn O_getOwnPropertyNames(J: *mut js_State) {
    let obj: *mut js_Object;
    let mut name: [c_char; 32] = [0; 32];
    let mut k: c_int;
    let mut i: c_int;

    if js_isobject(J, 1) == 0 {
        js_typeerror!(J, "not an object");
    }
    obj = js_toobject(J, 1);

    js_newarray(J);

    if (*(*obj).properties).level != 0 {
        i = O_getOwnPropertyNames_walk(J, (*obj).properties, 0);
    } else {
        i = 0;
    }

    if (*obj).type_ == JS_CARRAY {
        js_pushliteral(J, cs!("length"));
        js_setindex(J, -2, i);
        i += 1;
        if (*obj).u.a.simple != 0 {
            k = 0;
            while k < (*obj).u.a.flat_length {
                js_itoa(name.as_mut_ptr(), k);
                js_pushstring(J, name.as_ptr() as *const c_char);
                js_setindex(J, -2, i);
                i += 1;
                k += 1;
            }
        }
    }

    if (*obj).type_ == JS_CSTRING {
        js_pushliteral(J, cs!("length"));
        js_setindex(J, -2, i);
        i += 1;
        k = 0;
        while k < (*obj).u.s.length {
            js_itoa(name.as_mut_ptr(), k);
            js_pushstring(J, name.as_ptr() as *const c_char);
            js_setindex(J, -2, i);
            i += 1;
            k += 1;
        }
    }

    if (*obj).type_ == JS_CREGEXP {
        js_pushliteral(J, cs!("source"));
        js_setindex(J, -2, i);
        i += 1;
        js_pushliteral(J, cs!("global"));
        js_setindex(J, -2, i);
        i += 1;
        js_pushliteral(J, cs!("ignoreCase"));
        js_setindex(J, -2, i);
        i += 1;
        js_pushliteral(J, cs!("multiline"));
        js_setindex(J, -2, i);
        i += 1;
        js_pushliteral(J, cs!("lastIndex"));
        js_setindex(J, -2, i);
        i += 1;
    }
}

unsafe fn ToPropertyDescriptor(
    J: *mut js_State,
    obj: *mut js_Object,
    name: *const c_char,
    desc: *mut js_Object,
) {
    let mut haswritable: c_int = 0;
    let mut hasvalue: c_int = 0;
    let mut enumerable: c_int = 0;
    let mut configurable: c_int = 0;
    let mut writable: c_int = 0;
    let mut atts: c_int = 0;

    js_pushobject(J, obj);
    js_pushobject(J, desc);

    if js_hasproperty(J, -1, cs!("writable")) != 0 {
        haswritable = 1;
        writable = js_toboolean(J, -1);
        js_pop(J, 1);
    }
    if js_hasproperty(J, -1, cs!("enumerable")) != 0 {
        enumerable = js_toboolean(J, -1);
        js_pop(J, 1);
    }
    if js_hasproperty(J, -1, cs!("configurable")) != 0 {
        configurable = js_toboolean(J, -1);
        js_pop(J, 1);
    }
    if js_hasproperty(J, -1, cs!("value")) != 0 {
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

    if js_hasproperty(J, -1, cs!("get")) != 0 {
        if haswritable != 0 || hasvalue != 0 {
            js_typeerror!(J, "value/writable and get/set attributes are exclusive");
        }
    } else {
        js_pushundefined(J);
    }

    if js_hasproperty(J, -2, cs!("set")) != 0 {
        if haswritable != 0 || hasvalue != 0 {
            js_typeerror!(J, "value/writable and get/set attributes are exclusive");
        }
    } else {
        js_pushundefined(J);
    }

    js_defaccessor(J, -4, name, atts);

    js_pop(J, 2);
}

unsafe extern "C" fn O_defineProperty(J: *mut js_State) {
    if js_isobject(J, 1) == 0 {
        js_typeerror!(J, "not an object");
    }
    if js_isobject(J, 3) == 0 {
        js_typeerror!(J, "not an object");
    }
    ToPropertyDescriptor(
        J,
        js_toobject(J, 1),
        js_tostring(J, 2),
        js_toobject(J, 3),
    );
    js_copy(J, 1);
}

unsafe fn O_defineProperties_walk(J: *mut js_State, refp: *mut js_Property, i: c_int) -> c_int {
    let mut i = i;
    if (*(*refp).left).level != 0 {
        i = O_defineProperties_walk(J, (*refp).left, i);
    }
    if ((*refp).atts & JS_DONTENUM) == 0 {
        if vtype(addr_of!((*refp).value)) != JS_TOBJECT {
            js_typeerror!(J, "not an object");
        }
        js_pushstring(J, propname(refp) as *const c_char);
        js_setindex(J, -2, i);
        i += 1;
    }
    if (*(*refp).right).level != 0 {
        i = O_defineProperties_walk(J, (*refp).right, i);
    }
    i
}

unsafe fn O_defineProperties_imp(J: *mut js_State, obj: *mut js_Object) {
    let props: *mut js_Object;
    let mut name: *const c_char;
    let mut i: c_int;
    let n: c_int;

    if js_isobject(J, 2) == 0 {
        js_typeerror!(J, "not an object");
    }

    props = js_toobject(J, 2);
    if (*(*props).properties).level != 0 {
        js_newarray(J);
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

unsafe extern "C" fn O_defineProperties(J: *mut js_State) {
    let obj: *mut js_Object;
    if js_isobject(J, 1) == 0 {
        js_typeerror!(J, "not an object");
    }
    obj = js_toobject(J, 1);
    O_defineProperties_imp(J, obj);
    js_copy(J, 1);
}

unsafe extern "C" fn O_create(J: *mut js_State) {
    let obj: *mut js_Object;
    let proto: *mut js_Object;

    if js_isobject(J, 1) != 0 {
        proto = js_toobject(J, 1);
    } else if js_isnull(J, 1) != 0 {
        proto = null_mut();
    } else {
        js_typeerror!(J, "not an object or null");
    }

    obj = jsV_newobject(J, JS_COBJECT, proto);
    js_pushobject(J, obj);

    if js_isdefined(J, 2) != 0 {
        O_defineProperties_imp(J, obj);
    }
}

unsafe fn O_keys_walk(J: *mut js_State, refp: *mut js_Property, i: c_int) -> c_int {
    let mut i = i;
    if (*(*refp).left).level != 0 {
        i = O_keys_walk(J, (*refp).left, i);
    }
    if ((*refp).atts & JS_DONTENUM) == 0 {
        js_pushstring(J, propname(refp) as *const c_char);
        js_setindex(J, -2, i);
        i += 1;
    }
    if (*(*refp).right).level != 0 {
        i = O_keys_walk(J, (*refp).right, i);
    }
    i
}

unsafe extern "C" fn O_keys(J: *mut js_State) {
    let obj: *mut js_Object;
    let mut name: [c_char; 32] = [0; 32];
    let mut i: c_int;
    let mut k: c_int;

    if js_isobject(J, 1) == 0 {
        js_typeerror!(J, "not an object");
    }
    obj = js_toobject(J, 1);

    js_newarray(J);

    if (*(*obj).properties).level != 0 {
        i = O_keys_walk(J, (*obj).properties, 0);
    } else {
        i = 0;
    }

    if (*obj).type_ == JS_CSTRING {
        k = 0;
        while k < (*obj).u.s.length {
            js_itoa(name.as_mut_ptr(), k);
            js_pushstring(J, name.as_ptr() as *const c_char);
            js_setindex(J, -2, i);
            i += 1;
            k += 1;
        }
    }

    if (*obj).type_ == JS_CARRAY && (*obj).u.a.simple != 0 {
        k = 0;
        while k < (*obj).u.a.flat_length {
            js_itoa(name.as_mut_ptr(), k);
            js_pushstring(J, name.as_ptr() as *const c_char);
            js_setindex(J, -2, i);
            i += 1;
            k += 1;
        }
    }
}

unsafe extern "C" fn O_preventExtensions(J: *mut js_State) {
    let obj: *mut js_Object;
    if js_isobject(J, 1) == 0 {
        js_typeerror!(J, "not an object");
    }
    obj = js_toobject(J, 1);
    jsR_unflattenarray(J, obj);
    (*obj).extensible = 0;
    js_copy(J, 1);
}

unsafe extern "C" fn O_isExtensible(J: *mut js_State) {
    if js_isobject(J, 1) == 0 {
        js_typeerror!(J, "not an object");
    }
    js_pushboolean(J, (*js_toobject(J, 1)).extensible);
}

unsafe fn O_seal_walk(J: *mut js_State, refp: *mut js_Property) {
    if (*(*refp).left).level != 0 {
        O_seal_walk(J, (*refp).left);
    }
    (*refp).atts |= JS_DONTCONF;
    if (*(*refp).right).level != 0 {
        O_seal_walk(J, (*refp).right);
    }
}

unsafe extern "C" fn O_seal(J: *mut js_State) {
    let obj: *mut js_Object;

    if js_isobject(J, 1) == 0 {
        js_typeerror!(J, "not an object");
    }

    obj = js_toobject(J, 1);
    jsR_unflattenarray(J, obj);
    (*obj).extensible = 0;

    if (*(*obj).properties).level != 0 {
        O_seal_walk(J, (*obj).properties);
    }

    js_copy(J, 1);
}

unsafe fn O_isSealed_walk(J: *mut js_State, refp: *mut js_Property) -> c_int {
    if (*(*refp).left).level != 0 {
        if O_isSealed_walk(J, (*refp).left) == 0 {
            return 0;
        }
    }
    if ((*refp).atts & JS_DONTCONF) == 0 {
        return 0;
    }
    if (*(*refp).right).level != 0 {
        if O_isSealed_walk(J, (*refp).right) == 0 {
            return 0;
        }
    }
    1
}

unsafe extern "C" fn O_isSealed(J: *mut js_State) {
    let obj: *mut js_Object;

    if js_isobject(J, 1) == 0 {
        js_typeerror!(J, "not an object");
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

unsafe fn O_freeze_walk(J: *mut js_State, refp: *mut js_Property) {
    if (*(*refp).left).level != 0 {
        O_freeze_walk(J, (*refp).left);
    }
    (*refp).atts |= JS_READONLY | JS_DONTCONF;
    if (*(*refp).right).level != 0 {
        O_freeze_walk(J, (*refp).right);
    }
}

unsafe extern "C" fn O_freeze(J: *mut js_State) {
    let obj: *mut js_Object;

    if js_isobject(J, 1) == 0 {
        js_typeerror!(J, "not an object");
    }

    obj = js_toobject(J, 1);
    jsR_unflattenarray(J, obj);
    (*obj).extensible = 0;

    if (*(*obj).properties).level != 0 {
        O_freeze_walk(J, (*obj).properties);
    }

    js_copy(J, 1);
}

unsafe fn O_isFrozen_walk(J: *mut js_State, refp: *mut js_Property) -> c_int {
    if (*(*refp).left).level != 0 {
        if O_isFrozen_walk(J, (*refp).left) == 0 {
            return 0;
        }
    }
    if ((*refp).atts & JS_READONLY) == 0 {
        return 0;
    }
    if ((*refp).atts & JS_DONTCONF) == 0 {
        return 0;
    }
    if (*(*refp).right).level != 0 {
        if O_isFrozen_walk(J, (*refp).right) == 0 {
            return 0;
        }
    }
    1
}

unsafe extern "C" fn O_isFrozen(J: *mut js_State) {
    let obj: *mut js_Object;

    if js_isobject(J, 1) == 0 {
        js_typeerror!(J, "not an object");
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsB_initobject(J: *mut js_State) {
    js_pushobject(J, (*J).Object_prototype);
    {
        jsB_propf(J, cs!("Object.prototype.toString"), Some(Op_toString), 0);
        jsB_propf(
            J,
            cs!("Object.prototype.toLocaleString"),
            Some(Op_toString),
            0,
        );
        jsB_propf(J, cs!("Object.prototype.valueOf"), Some(Op_valueOf), 0);
        jsB_propf(
            J,
            cs!("Object.prototype.hasOwnProperty"),
            Some(Op_hasOwnProperty),
            1,
        );
        jsB_propf(
            J,
            cs!("Object.prototype.isPrototypeOf"),
            Some(Op_isPrototypeOf),
            1,
        );
        jsB_propf(
            J,
            cs!("Object.prototype.propertyIsEnumerable"),
            Some(Op_propertyIsEnumerable),
            1,
        );
    }
    js_newcconstructor(
        J,
        Some(jsB_Object),
        Some(jsB_new_Object),
        cs!("Object"),
        1,
    );
    {
        /* ES5 */
        jsB_propf(J, cs!("Object.getPrototypeOf"), Some(O_getPrototypeOf), 1);
        jsB_propf(
            J,
            cs!("Object.getOwnPropertyDescriptor"),
            Some(O_getOwnPropertyDescriptor),
            2,
        );
        jsB_propf(
            J,
            cs!("Object.getOwnPropertyNames"),
            Some(O_getOwnPropertyNames),
            1,
        );
        jsB_propf(J, cs!("Object.create"), Some(O_create), 2);
        jsB_propf(J, cs!("Object.defineProperty"), Some(O_defineProperty), 3);
        jsB_propf(
            J,
            cs!("Object.defineProperties"),
            Some(O_defineProperties),
            2,
        );
        jsB_propf(J, cs!("Object.seal"), Some(O_seal), 1);
        jsB_propf(J, cs!("Object.freeze"), Some(O_freeze), 1);
        jsB_propf(
            J,
            cs!("Object.preventExtensions"),
            Some(O_preventExtensions),
            1,
        );
        jsB_propf(J, cs!("Object.isSealed"), Some(O_isSealed), 1);
        jsB_propf(J, cs!("Object.isFrozen"), Some(O_isFrozen), 1);
        jsB_propf(J, cs!("Object.isExtensible"), Some(O_isExtensible), 1);
        jsB_propf(J, cs!("Object.keys"), Some(O_keys), 1);
    }
    js_defglobal(J, cs!("Object"), JS_DONTENUM);
}
