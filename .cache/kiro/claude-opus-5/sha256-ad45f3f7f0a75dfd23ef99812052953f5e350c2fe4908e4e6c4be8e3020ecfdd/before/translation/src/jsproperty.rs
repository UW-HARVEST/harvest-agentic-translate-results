//! Translation of src/jsproperty.c
//!
//! Use an AA-tree to quickly look up properties in objects.

use crate::jsi::*;

use crate::jsrun::{js_free, js_malloc};
use crate::jsvalue::{js_itoa, jsV_numbertointeger, jsV_numbertostring, jsV_stringtonumber};

/*
	Use an AA-tree to quickly look up properties in objects:

	The level of every leaf node is one.
	The level of every left child is one less than its parent.
	The level of every right child is equal or one less than its parent.
	The level of every right grandchild is less than its grandparent.
	Every node of level greater than one has two children.

	A link where the child's level is equal to that of its parent is called a horizontal link.
	Individual right horizontal links are allowed, but consecutive ones are forbidden.
	Left horizontal links are forbidden.

	skew() fixes left horizontal links.
	split() fixes consecutive right horizontal links.
*/

/* static js_Property sentinel = { &sentinel, &sentinel, 0, 0, ... } */
static mut SENTINEL: js_Property = js_Property {
    left: core::ptr::null_mut(),
    right: core::ptr::null_mut(),
    level: 0,
    atts: 0,
    value: js_Value::undef(),
    getter: core::ptr::null_mut(),
    setter: core::ptr::null_mut(),
    name: [0],
};

/// Return the address of the file-static sentinel, self-linking `left`/`right`
/// on first use. `sentinel.level == 0` is used as the null test.
#[inline]
unsafe fn sentinel() -> *mut js_Property {
    unsafe {
        let s = &raw mut SENTINEL;
        if (*s).left.is_null() {
            (*s).left = s;
            (*s).right = s;
        }
        s
    }
}

unsafe fn newproperty(J: *mut js_State, obj: *mut js_Object, name: *const c_char) -> *mut js_Property {
    unsafe {
        let n = (strlen(name) as c_int) + 1;
        let node =
            js_malloc(J, JS_PROPERTY_NAMEOFF + n) as *mut js_Property;
        (*node).left = sentinel();
        (*node).right = sentinel();
        (*node).level = 1;
        (*node).atts = 0;
        (*node).value.set_ty(JS_TUNDEFINED);
        (*node).value.number = 0.0;
        (*node).getter = core::ptr::null_mut();
        (*node).setter = core::ptr::null_mut();
        memcpy(
            (&raw mut (*node).name) as *mut c_void,
            name as *const c_void,
            n as size_t,
        );
        (*obj).count += 1;
        (*J).gccounter += 1;
        node
    }
}

unsafe fn lookup(mut node: *mut js_Property, name: *const c_char) -> *mut js_Property {
    unsafe {
        while node != sentinel() {
            let c = strcmp(name, (&raw const (*node).name) as *const c_char);
            if c == 0 {
                return node;
            } else if c < 0 {
                node = (*node).left;
            } else {
                node = (*node).right;
            }
        }
        core::ptr::null_mut()
    }
}

unsafe fn skew(mut node: *mut js_Property) -> *mut js_Property {
    unsafe {
        if (*(*node).left).level == (*node).level {
            let temp = node;
            node = (*node).left;
            (*temp).left = (*node).right;
            (*node).right = temp;
        }
        node
    }
}

unsafe fn split(mut node: *mut js_Property) -> *mut js_Property {
    unsafe {
        if (*(*(*node).right).right).level == (*node).level {
            let temp = node;
            node = (*node).right;
            (*temp).right = (*node).left;
            (*node).left = temp;
            (*node).level += 1;
        }
        node
    }
}

unsafe fn insert(
    J: *mut js_State,
    obj: *mut js_Object,
    mut node: *mut js_Property,
    name: *const c_char,
    result: *mut *mut js_Property,
) -> *mut js_Property {
    unsafe {
        if node != sentinel() {
            let c = strcmp(name, (&raw const (*node).name) as *const c_char);
            if c < 0 {
                (*node).left = insert(J, obj, (*node).left, name, result);
            } else if c > 0 {
                (*node).right = insert(J, obj, (*node).right, name, result);
            } else {
                *result = node;
                return *result;
            }
            node = skew(node);
            node = split(node);
            return node;
        }
        *result = newproperty(J, obj, name);
        *result
    }
}

unsafe fn freeproperty(J: *mut js_State, obj: *mut js_Object, node: *mut js_Property) {
    unsafe {
        js_free(J, node as *mut c_void);
        (*obj).count -= 1;
    }
}

unsafe fn unlinkproperty(
    mut node: *mut js_Property,
    name: *const c_char,
    garbage: *mut *mut js_Property,
) -> *mut js_Property {
    unsafe {
        let mut temp: *mut js_Property = core::ptr::null_mut();
        let mut a: *mut js_Property;
        let b: *mut js_Property;
        if node != sentinel() {
            let c = strcmp(name, (&raw const (*node).name) as *const c_char);
            if c < 0 {
                (*node).left = unlinkproperty((*node).left, name, garbage);
            } else if c > 0 {
                (*node).right = unlinkproperty((*node).right, name, garbage);
            } else {
                *garbage = node;
                if (*node).left == sentinel() && (*node).right == sentinel() {
                    return sentinel();
                } else if (*node).left == sentinel() {
                    a = (*node).right;
                    while (*a).left != sentinel() {
                        a = (*a).left;
                    }
                    b = unlinkproperty((*node).right, (&raw const (*a).name) as *const c_char, &raw mut temp);
                    (*temp).level = (*node).level;
                    (*temp).left = (*node).left;
                    (*temp).right = b;
                    node = temp;
                } else {
                    a = (*node).left;
                    while (*a).right != sentinel() {
                        a = (*a).right;
                    }
                    b = unlinkproperty((*node).left, (&raw const (*a).name) as *const c_char, &raw mut temp);
                    (*temp).level = (*node).level;
                    (*temp).left = b;
                    (*temp).right = (*node).right;
                    node = temp;
                }
            }

            if (*(*node).left).level < (*node).level - 1 || (*(*node).right).level < (*node).level - 1
            {
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
}

unsafe fn deleteproperty(
    J: *mut js_State,
    obj: *mut js_Object,
    mut tree: *mut js_Property,
    name: *const c_char,
) -> *mut js_Property {
    unsafe {
        let mut garbage: *mut js_Property = sentinel();
        tree = unlinkproperty(tree, name, &raw mut garbage);
        if garbage != sentinel() {
            freeproperty(J, obj, garbage);
        }
        tree
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsV_newobject(
    J: *mut js_State,
    ty: c_int,
    prototype: *mut js_Object,
) -> *mut js_Object {
    unsafe {
        let obj = js_malloc(J, core::mem::size_of::<js_Object>() as c_int) as *mut js_Object;
        memset(obj as *mut c_void, 0, core::mem::size_of::<js_Object>() as size_t);
        (*obj).gcmark = 0;
        (*obj).gcnext = (*J).gcobj;
        (*J).gcobj = obj;
        (*J).gccounter += 1;

        (*obj).ty = ty;
        (*obj).properties = sentinel();
        (*obj).prototype = prototype;
        (*obj).extensible = 1;
        obj
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsV_getownproperty(
    J: *mut js_State,
    obj: *mut js_Object,
    name: *const c_char,
) -> *mut js_Property {
    unsafe { lookup((*obj).properties, name) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsV_getpropertyx(
    J: *mut js_State,
    mut obj: *mut js_Object,
    name: *const c_char,
    own: *mut c_int,
) -> *mut js_Property {
    unsafe {
        *own = 1;
        loop {
            let refp = lookup((*obj).properties, name);
            if !refp.is_null() {
                return refp;
            }
            obj = (*obj).prototype;
            *own = 0;
            if obj.is_null() {
                break;
            }
        }
        core::ptr::null_mut()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsV_getproperty(
    J: *mut js_State,
    mut obj: *mut js_Object,
    name: *const c_char,
) -> *mut js_Property {
    unsafe {
        loop {
            let refp = lookup((*obj).properties, name);
            if !refp.is_null() {
                return refp;
            }
            obj = (*obj).prototype;
            if obj.is_null() {
                break;
            }
        }
        core::ptr::null_mut()
    }
}

unsafe fn jsV_getenumproperty(
    J: *mut js_State,
    mut obj: *mut js_Object,
    name: *const c_char,
) -> *mut js_Property {
    unsafe {
        loop {
            let refp = lookup((*obj).properties, name);
            if !refp.is_null() && ((*refp).atts & JS_DONTENUM) == 0 {
                return refp;
            }
            obj = (*obj).prototype;
            if obj.is_null() {
                break;
            }
        }
        core::ptr::null_mut()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsV_setproperty(
    J: *mut js_State,
    obj: *mut js_Object,
    name: *const c_char,
) -> *mut js_Property {
    unsafe {
        let mut result: *mut js_Property;

        if (*obj).extensible == 0 {
            result = lookup((*obj).properties, name);
            if (*J).strict != 0 && result.is_null() {
                js_typeerror!(J, c"object is non-extensible".as_ptr());
            }
            return result;
        }

        result = core::ptr::null_mut();
        (*obj).properties = insert(J, obj, (*obj).properties, name, &raw mut result);

        result
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsV_delproperty(
    J: *mut js_State,
    obj: *mut js_Object,
    name: *const c_char,
) {
    unsafe {
        (*obj).properties = deleteproperty(J, obj, (*obj).properties, name);
    }
}

/* Flatten hierarchy of enumerable properties into an iterator object */

unsafe fn itnewnode(
    J: *mut js_State,
    name: *const c_char,
    next: *mut js_Iterator,
) -> *mut js_Iterator {
    unsafe {
        let n = (strlen(name) as c_int) + 1;
        let node = js_malloc(J, JS_ITERATOR_NAMEOFF + n) as *mut js_Iterator;
        (*node).next = next;
        memcpy(
            (&raw mut (*node).name) as *mut c_void,
            name as *const c_void,
            n as size_t,
        );
        node
    }
}

unsafe fn itwalk(
    J: *mut js_State,
    mut iter: *mut js_Iterator,
    prop: *mut js_Property,
    seen: *mut js_Object,
) -> *mut js_Iterator {
    unsafe {
        if (*prop).right != sentinel() {
            iter = itwalk(J, iter, (*prop).right, seen);
        }
        if ((*prop).atts & JS_DONTENUM) == 0 {
            if seen.is_null()
                || jsV_getenumproperty(J, seen, (&raw const (*prop).name) as *const c_char).is_null()
            {
                iter = itnewnode(J, (&raw const (*prop).name) as *const c_char, iter);
            }
        }
        if (*prop).left != sentinel() {
            iter = itwalk(J, iter, (*prop).left, seen);
        }
        iter
    }
}

unsafe fn itflatten(J: *mut js_State, obj: *mut js_Object) -> *mut js_Iterator {
    unsafe {
        let mut iter: *mut js_Iterator = core::ptr::null_mut();
        if !(*obj).prototype.is_null() {
            iter = itflatten(J, (*obj).prototype);
        }
        if (*obj).properties != sentinel() {
            iter = itwalk(J, iter, (*obj).properties, (*obj).prototype);
        }
        iter
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsV_newiterator(
    J: *mut js_State,
    obj: *mut js_Object,
    own: c_int,
) -> *mut js_Object {
    unsafe {
        let io = jsV_newobject(J, JS_CITERATOR, core::ptr::null_mut());
        (*io).u.iter.target = obj;
        (*io).u.iter.i = 0;
        (*io).u.iter.n = 0;
        if own != 0 {
            (*io).u.iter.head = core::ptr::null_mut();
            if (*obj).properties != sentinel() {
                (*io).u.iter.head =
                    itwalk(J, (*io).u.iter.head, (*obj).properties, core::ptr::null_mut());
            }
        } else {
            (*io).u.iter.head = itflatten(J, obj);
        }
        (*io).u.iter.current = (*io).u.iter.head;

        if (*obj).ty == JS_CSTRING {
            (*io).u.iter.n = (*obj).u.s.length;
        }

        if (*obj).ty == JS_CARRAY && (*obj).u.a.simple != 0 {
            (*io).u.iter.n = (*obj).u.a.flat_length;
        }

        io
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsV_nextiterator(
    J: *mut js_State,
    io: *mut js_Object,
) -> *const c_char {
    unsafe {
        if (*io).ty != JS_CITERATOR {
            js_typeerror!(J, c"not an iterator".as_ptr());
        }
        if (*io).u.iter.i < (*io).u.iter.n {
            js_itoa((&raw mut (*J).scratch) as *mut c_char, (*io).u.iter.i);
            (*io).u.iter.i += 1;
            return (&raw const (*J).scratch) as *const c_char;
        }
        while !(*io).u.iter.current.is_null() {
            let name = (&raw const (*(*io).u.iter.current).name) as *const c_char;
            (*io).u.iter.current = (*(*io).u.iter.current).next;
            if !jsV_getproperty(J, (*io).u.iter.target, name).is_null() {
                return name;
            }
        }
        core::ptr::null()
    }
}

/* Walk all the properties and delete them one by one for arrays */

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsV_resizearray(J: *mut js_State, obj: *mut js_Object, newlen: c_int) {
    unsafe {
        let mut buf: [c_char; 32] = [0; 32];
        let mut s: *const c_char;
        let mut k: c_int;
        /* assert(!obj->u.a.simple); */
        if newlen < (*obj).u.a.length {
            if (*obj).u.a.length > (*obj).count * 2 {
                let it = jsV_newiterator(J, obj, 1);
                loop {
                    s = jsV_nextiterator(J, it);
                    if s.is_null() {
                        break;
                    }
                    k = jsV_numbertointeger(jsV_stringtonumber(J, s));
                    if k >= newlen
                        && strcmp(s, jsV_numbertostring(J, buf.as_mut_ptr(), k as f64)) == 0
                    {
                        jsV_delproperty(J, obj, s);
                    }
                }
            } else {
                k = newlen;
                while k < (*obj).u.a.length {
                    jsV_delproperty(J, obj, js_itoa(buf.as_mut_ptr(), k));
                    k += 1;
                }
            }
        }
        (*obj).u.a.length = newlen;
    }
}
