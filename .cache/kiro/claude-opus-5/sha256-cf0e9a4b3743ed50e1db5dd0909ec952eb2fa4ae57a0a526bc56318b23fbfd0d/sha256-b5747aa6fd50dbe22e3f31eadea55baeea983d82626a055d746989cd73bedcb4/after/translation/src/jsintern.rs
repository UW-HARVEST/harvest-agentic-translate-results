// Translation of c_src/src/jsintern.c
#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals, dead_code)]

use crate::common::*;
use crate::jsrun::*;
use crate::types::*;
use crate::js_rangeerror;
use std::ffi::{c_char, c_int, c_void};
use std::ptr;

/* Dynamically grown string buffer */

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_putc(J: *mut js_State, sbp: *mut *mut js_Buffer, c: c_int) {
    unsafe {
        let mut sb = *sbp;
        if sb.is_null() {
            sb = js_malloc(J, std::mem::size_of::<js_Buffer>() as c_int) as *mut js_Buffer;
            (*sb).n = 0;
            (*sb).m = 64; /* sizeof sb->s */
            *sbp = sb;
        } else if (*sb).n == (*sb).m {
            (*sb).m *= 2;
            sb = js_realloc(
                J,
                sb as *mut c_void,
                (*sb).m + std::mem::offset_of!(js_Buffer, s) as c_int,
            ) as *mut js_Buffer;
            *sbp = sb;
        }
        let s = (&raw mut (*sb).s) as *mut c_char;
        *s.offset((*sb).n as isize) = c as c_char;
        (*sb).n += 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_puts(
    J: *mut js_State,
    sb: *mut *mut js_Buffer,
    s: *const c_char,
) {
    unsafe {
        let mut s = s;
        while *s != 0 {
            js_putc(J, sb, *s as c_int);
            s = s.add(1);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_putm(
    J: *mut js_State,
    sb: *mut *mut js_Buffer,
    s: *const c_char,
    e: *const c_char,
) {
    unsafe {
        let mut s = s;
        while s < e {
            js_putc(J, sb, *s as c_int);
            s = s.add(1);
        }
    }
}

/* Use an AA-tree to quickly look up interned strings. */

static mut JSS_SENTINEL: js_StringNode = js_StringNode {
    left: ptr::null_mut(),
    right: ptr::null_mut(),
    level: 0,
    string: [0],
};

#[inline]
unsafe fn jsS_sentinel() -> *mut js_StringNode {
    unsafe {
        let p = &raw mut JSS_SENTINEL;
        if (*p).left.is_null() {
            (*p).left = p;
            (*p).right = p;
        }
        p
    }
}

unsafe fn jsS_newstringnode(
    J: *mut js_State,
    string: *const c_char,
    result: *mut *const c_char,
) -> *mut js_StringNode {
    unsafe {
        let n = strlen(string);
        if n > JS_STRLIMIT {
            js_rangeerror!(J, c"invalid string length");
        }
        let node = js_malloc(
            J,
            std::mem::offset_of!(js_StringNode, string) as c_int + n as c_int + 1,
        ) as *mut js_StringNode;
        (*node).left = jsS_sentinel();
        (*node).right = jsS_sentinel();
        (*node).level = 1;
        memcpy(
            (&raw mut (*node).string) as *mut c_void,
            string as *const c_void,
            n + 1,
        );
        *result = (&raw const (*node).string) as *const c_char;
        node
    }
}

unsafe fn jsS_skew(node: *mut js_StringNode) -> *mut js_StringNode {
    unsafe {
        let mut node = node;
        if (*(*node).left).level == (*node).level {
            let temp = node;
            node = (*node).left;
            (*temp).left = (*node).right;
            (*node).right = temp;
        }
        node
    }
}

unsafe fn jsS_split(node: *mut js_StringNode) -> *mut js_StringNode {
    unsafe {
        let mut node = node;
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

unsafe fn jsS_insert(
    J: *mut js_State,
    node: *mut js_StringNode,
    string: *const c_char,
    result: *mut *const c_char,
) -> *mut js_StringNode {
    unsafe {
        if node != jsS_sentinel() {
            let mut node = node;
            let c = strcmp(string, (&raw const (*node).string) as *const c_char);
            if c < 0 {
                (*node).left = jsS_insert(J, (*node).left, string, result);
            } else if c > 0 {
                (*node).right = jsS_insert(J, (*node).right, string, result);
            } else {
                *result = (&raw const (*node).string) as *const c_char;
                return node;
            }
            node = jsS_skew(node);
            node = jsS_split(node);
            return node;
        }
        jsS_newstringnode(J, string, result)
    }
}

unsafe fn dumpstringnode(node: *mut js_StringNode, level: c_int) {
    unsafe {
        if (*node).left != jsS_sentinel() {
            dumpstringnode((*node).left, level + 1);
        }
        printf(c"%d: ".as_ptr(), (*node).level);
        let mut i = 0;
        while i < level {
            putchar(b'\t' as c_int);
            i += 1;
        }
        printf(
            c"'%s'\n".as_ptr(),
            (&raw const (*node).string) as *const c_char,
        );
        if (*node).right != jsS_sentinel() {
            dumpstringnode((*node).right, level + 1);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsS_dumpstrings(J: *mut js_State) {
    unsafe {
        let root = (*J).strings;
        printf(c"interned strings {\n".as_ptr());
        if !root.is_null() && root != jsS_sentinel() {
            dumpstringnode(root, 1);
        }
        printf(c"}\n".as_ptr());
    }
}

unsafe fn jsS_freestringnode(J: *mut js_State, node: *mut js_StringNode) {
    unsafe {
        if (*node).left != jsS_sentinel() {
            jsS_freestringnode(J, (*node).left);
        }
        if (*node).right != jsS_sentinel() {
            jsS_freestringnode(J, (*node).right);
        }
        js_free(J, node as *mut c_void);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsS_freestrings(J: *mut js_State) {
    unsafe {
        if !(*J).strings.is_null() && (*J).strings != jsS_sentinel() {
            jsS_freestringnode(J, (*J).strings);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_intern(J: *mut js_State, s: *const c_char) -> *const c_char {
    unsafe {
        let mut result: *const c_char = ptr::null();
        if (*J).strings.is_null() {
            (*J).strings = jsS_sentinel();
        }
        (*J).strings = jsS_insert(J, (*J).strings, s, &mut result);
        result
    }
}
