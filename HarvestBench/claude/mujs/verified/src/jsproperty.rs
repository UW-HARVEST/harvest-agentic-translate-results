//! Translated from jsproperty.c — object property AA-tree, objects, iterators.
#![allow(non_snake_case, non_upper_case_globals)]

use crate::cutil::*;
use crate::jsrun::{js_free, js_malloc};
use crate::types::*;
use std::os::raw::{c_char, c_int, c_void};

macro_rules! cstr {
    ($s:literal) => {
        concat!($s, "\0").as_ptr() as *const c_char
    };
}

static mut sentinel: js_Property = js_Property {
    left: std::ptr::null_mut(),
    right: std::ptr::null_mut(),
    level: 0,
    atts: 0,
    value: js_Value { t: JsValueT { pad: [0; 15], type_: JS_TUNDEFINED } },
    getter: std::ptr::null_mut(),
    setter: std::ptr::null_mut(),
    name: [0],
};

#[inline]
unsafe fn sen() -> *mut js_Property {
    std::ptr::addr_of_mut!(sentinel)
}

#[inline]
unsafe fn init_sentinel() {
    if sentinel.left.is_null() {
        sentinel.left = sen();
        sentinel.right = sen();
    }
}

unsafe fn newproperty(J: *mut js_State, obj: *mut js_Object, name: *const c_char) -> *mut js_Property {
    let n = strlen(name) + 1;
    let base = std::mem::offset_of!(js_Property, name);
    let node = js_malloc(J, (base + n) as c_int) as *mut js_Property;
    (*node).left = sen();
    (*node).right = sen();
    (*node).level = 1;
    (*node).atts = 0;
    (*node).value.set_type(JS_TUNDEFINED);
    (*node).value.u.number = 0.0;
    (*node).getter = std::ptr::null_mut();
    (*node).setter = std::ptr::null_mut();
    memcpy((*node).name.as_mut_ptr(), name, n);
    (*obj).count += 1;
    (*J).gccounter += 1;
    node
}

unsafe fn lookup(mut node: *mut js_Property, name: *const c_char) -> *mut js_Property {
    while node != sen() {
        let c = strcmp(name, (*node).name.as_ptr());
        if c == 0 {
            return node;
        } else if c < 0 {
            node = (*node).left;
        } else {
            node = (*node).right;
        }
    }
    std::ptr::null_mut()
}

unsafe fn skew(mut node: *mut js_Property) -> *mut js_Property {
    if (*(*node).left).level == (*node).level {
        let temp = node;
        node = (*node).left;
        (*temp).left = (*node).right;
        (*node).right = temp;
    }
    node
}

unsafe fn split(mut node: *mut js_Property) -> *mut js_Property {
    if (*(*(*node).right).right).level == (*node).level {
        let temp = node;
        node = (*node).right;
        (*temp).right = (*node).left;
        (*node).left = temp;
        (*node).level += 1;
    }
    node
}

unsafe fn insert(J: *mut js_State, obj: *mut js_Object, mut node: *mut js_Property, name: *const c_char, result: *mut *mut js_Property) -> *mut js_Property {
    if node != sen() {
        let c = strcmp(name, (*node).name.as_ptr());
        if c < 0 {
            (*node).left = insert(J, obj, (*node).left, name, result);
        } else if c > 0 {
            (*node).right = insert(J, obj, (*node).right, name, result);
        } else {
            *result = node;
            return node;
        }
        node = skew(node);
        node = split(node);
        return node;
    }
    let np = newproperty(J, obj, name);
    *result = np;
    np
}

unsafe fn freeproperty(J: *mut js_State, obj: *mut js_Object, node: *mut js_Property) {
    js_free(J, node as *mut c_void);
    (*obj).count -= 1;
}

unsafe fn unlinkproperty(mut node: *mut js_Property, name: *const c_char, garbage: *mut *mut js_Property) -> *mut js_Property {
    let mut temp: *mut js_Property = std::ptr::null_mut();
    let mut a: *mut js_Property;
    let b: *mut js_Property;
    if node != sen() {
        let c = strcmp(name, (*node).name.as_ptr());
        if c < 0 {
            (*node).left = unlinkproperty((*node).left, name, garbage);
        } else if c > 0 {
            (*node).right = unlinkproperty((*node).right, name, garbage);
        } else {
            *garbage = node;
            if (*node).left == sen() && (*node).right == sen() {
                return sen();
            } else if (*node).left == sen() {
                a = (*node).right;
                while (*a).left != sen() {
                    a = (*a).left;
                }
                b = unlinkproperty((*node).right, (*a).name.as_ptr(), &mut temp);
                (*temp).level = (*node).level;
                (*temp).left = (*node).left;
                (*temp).right = b;
                node = temp;
            } else {
                a = (*node).left;
                while (*a).right != sen() {
                    a = (*a).right;
                }
                b = unlinkproperty((*node).left, (*a).name.as_ptr(), &mut temp);
                (*temp).level = (*node).level;
                (*temp).left = b;
                (*temp).right = (*node).right;
                node = temp;
            }
        }

        if (*(*node).left).level < (*node).level - 1 || (*(*node).right).level < (*node).level - 1 {
            (*node).level -= 1;
            if (*(*node).right).level > (*node).level {
                (*(*node).right).level = (*node).level;
            }
            node = skew(node);
            (*node).right = skew((*node).right);
            (*(*node).right).right = skew((*(*node).right).right);
            node = split(node);
            (*node).right = split((*node).right);
        }
    }
    node
}

unsafe fn deleteproperty(J: *mut js_State, obj: *mut js_Object, mut tree: *mut js_Property, name: *const c_char) -> *mut js_Property {
    let mut garbage: *mut js_Property = sen();
    tree = unlinkproperty(tree, name, &mut garbage);
    if garbage != sen() {
        freeproperty(J, obj, garbage);
    }
    tree
}

#[no_mangle]
pub unsafe extern "C-unwind" fn jsV_newobject(J: *mut js_State, type_: c_int, prototype: *mut js_Object) -> *mut js_Object {
    init_sentinel();
    let obj = js_malloc(J, std::mem::size_of::<js_Object>() as c_int) as *mut js_Object;
    libc::memset(obj as *mut c_void, 0, std::mem::size_of::<js_Object>());
    (*obj).gcmark = 0;
    (*obj).gcnext = (*J).gcobj;
    (*J).gcobj = obj;
    (*J).gccounter += 1;

    (*obj).type_ = type_;
    (*obj).properties = sen();
    (*obj).prototype = prototype;
    (*obj).extensible = 1;
    obj
}

#[no_mangle]
pub unsafe extern "C-unwind" fn jsV_getownproperty(J: *mut js_State, obj: *mut js_Object, name: *const c_char) -> *mut js_Property {
    lookup((*obj).properties, name)
}

#[no_mangle]
pub unsafe extern "C-unwind" fn jsV_getpropertyx(J: *mut js_State, mut obj: *mut js_Object, name: *const c_char, own: *mut c_int) -> *mut js_Property {
    *own = 1;
    loop {
        let rf = lookup((*obj).properties, name);
        if !rf.is_null() {
            return rf;
        }
        obj = (*obj).prototype;
        *own = 0;
        if obj.is_null() {
            break;
        }
    }
    std::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C-unwind" fn jsV_getproperty(J: *mut js_State, mut obj: *mut js_Object, name: *const c_char) -> *mut js_Property {
    loop {
        let rf = lookup((*obj).properties, name);
        if !rf.is_null() {
            return rf;
        }
        obj = (*obj).prototype;
        if obj.is_null() {
            break;
        }
    }
    std::ptr::null_mut()
}

unsafe fn jsV_getenumproperty(J: *mut js_State, mut obj: *mut js_Object, name: *const c_char) -> *mut js_Property {
    loop {
        let rf = lookup((*obj).properties, name);
        if !rf.is_null() && (*rf).atts & JS_DONTENUM == 0 {
            return rf;
        }
        obj = (*obj).prototype;
        if obj.is_null() {
            break;
        }
    }
    std::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C-unwind" fn jsV_setproperty(J: *mut js_State, obj: *mut js_Object, name: *const c_char) -> *mut js_Property {
    let mut result: *mut js_Property = std::ptr::null_mut();

    if (*obj).extensible == 0 {
        result = lookup((*obj).properties, name);
        if (*J).strict != 0 && result.is_null() {
            crate::jserror::js_typeerror(J, cstr!("object is non-extensible"));
        }
        return result;
    }

    (*obj).properties = insert(J, obj, (*obj).properties, name, &mut result);
    result
}

#[no_mangle]
pub unsafe extern "C-unwind" fn jsV_delproperty(J: *mut js_State, obj: *mut js_Object, name: *const c_char) {
    (*obj).properties = deleteproperty(J, obj, (*obj).properties, name);
}

/* Iterators */
unsafe fn itnewnode(J: *mut js_State, name: *const c_char, next: *mut js_Iterator) -> *mut js_Iterator {
    let n = strlen(name) + 1;
    let base = std::mem::offset_of!(js_Iterator, name);
    let node = js_malloc(J, (base + n) as c_int) as *mut js_Iterator;
    (*node).next = next;
    memcpy((*node).name.as_mut_ptr(), name, n);
    node
}

unsafe fn itwalk(J: *mut js_State, mut iter: *mut js_Iterator, prop: *mut js_Property, seen: *mut js_Object) -> *mut js_Iterator {
    if (*prop).right != sen() {
        iter = itwalk(J, iter, (*prop).right, seen);
    }
    if (*prop).atts & JS_DONTENUM == 0 {
        if seen.is_null() || jsV_getenumproperty(J, seen, (*prop).name.as_ptr()).is_null() {
            iter = itnewnode(J, (*prop).name.as_ptr(), iter);
        }
    }
    if (*prop).left != sen() {
        iter = itwalk(J, iter, (*prop).left, seen);
    }
    iter
}

unsafe fn itflatten(J: *mut js_State, obj: *mut js_Object) -> *mut js_Iterator {
    let mut iter: *mut js_Iterator = std::ptr::null_mut();
    if !(*obj).prototype.is_null() {
        iter = itflatten(J, (*obj).prototype);
    }
    if (*obj).properties != sen() {
        iter = itwalk(J, iter, (*obj).properties, (*obj).prototype);
    }
    iter
}

#[no_mangle]
pub unsafe extern "C-unwind" fn jsV_newiterator(J: *mut js_State, obj: *mut js_Object, own: c_int) -> *mut js_Object {
    let io = jsV_newobject(J, JS_CITERATOR, std::ptr::null_mut());
    (*io).u.iter.target = obj;
    (*io).u.iter.i = 0;
    (*io).u.iter.n = 0;
    if own != 0 {
        (*io).u.iter.head = std::ptr::null_mut();
        if (*obj).properties != sen() {
            (*io).u.iter.head = itwalk(J, (*io).u.iter.head, (*obj).properties, std::ptr::null_mut());
        }
    } else {
        (*io).u.iter.head = itflatten(J, obj);
    }
    (*io).u.iter.current = (*io).u.iter.head;

    if (*obj).type_ == JS_CSTRING {
        (*io).u.iter.n = (*obj).u.s.length;
    }
    if (*obj).type_ == JS_CARRAY && (*obj).u.a.simple != 0 {
        (*io).u.iter.n = (*obj).u.a.flat_length;
    }

    io
}

#[no_mangle]
pub unsafe extern "C-unwind" fn jsV_nextiterator(J: *mut js_State, io: *mut js_Object) -> *const c_char {
    if (*io).type_ != JS_CITERATOR {
        crate::jserror::js_typeerror(J, cstr!("not an iterator"));
    }
    if (*io).u.iter.i < (*io).u.iter.n {
        crate::jsvalue::js_itoa((*J).scratch.as_mut_ptr(), (*io).u.iter.i);
        (*io).u.iter.i += 1;
        return (*J).scratch.as_ptr();
    }
    while !(*io).u.iter.current.is_null() {
        let name = (*(*io).u.iter.current).name.as_ptr();
        (*io).u.iter.current = (*(*io).u.iter.current).next;
        if !jsV_getproperty(J, (*io).u.iter.target, name).is_null() {
            return name;
        }
    }
    std::ptr::null()
}

#[no_mangle]
pub unsafe extern "C-unwind" fn jsV_resizearray(J: *mut js_State, obj: *mut js_Object, newlen: c_int) {
    let mut buf: [c_char; 32] = [0; 32];
    let mut k: c_int;
    if newlen < (*obj).u.a.length {
        if (*obj).u.a.length > (*obj).count * 2 {
            let it = jsV_newiterator(J, obj, 1);
            loop {
                let s = jsV_nextiterator(J, it);
                if s.is_null() {
                    break;
                }
                k = crate::jsvalue::jsV_numbertointeger(crate::jsvalue::jsV_stringtonumber(J, s));
                if k >= newlen && strcmp(s, crate::jsvalue::jsV_numbertostring(J, buf.as_mut_ptr(), k as f64)) == 0 {
                    jsV_delproperty(J, obj, s);
                }
            }
        } else {
            k = newlen;
            while k < (*obj).u.a.length {
                jsV_delproperty(J, obj, crate::jsvalue::js_itoa(buf.as_mut_ptr(), k));
                k += 1;
            }
        }
    }
    (*obj).u.a.length = newlen;
}

/// Exposed for jsgc.c which frees property trees; the sentinel has level 0.
#[inline]
pub(crate) unsafe fn property_sentinel() -> *mut js_Property {
    init_sentinel();
    sen()
}
