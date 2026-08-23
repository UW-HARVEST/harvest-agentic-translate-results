//! Translated from jsintern.c — dynamic string buffer + interned string AA-tree.
#![allow(non_snake_case, non_upper_case_globals)]

use crate::cutil::*;
use crate::jsrun::{js_free, js_malloc, js_realloc};
use crate::types::*;
use std::os::raw::{c_char, c_int, c_void};

macro_rules! cstr {
    ($s:literal) => {
        concat!($s, "\0").as_ptr() as *const c_char
    };
}

/* Dynamically grown string buffer */
#[no_mangle]
pub unsafe extern "C-unwind" fn js_putc(J: *mut js_State, sbp: *mut *mut js_Buffer, c: c_int) {
    let mut sb = *sbp;
    if sb.is_null() {
        sb = js_malloc(J, std::mem::size_of::<js_Buffer>() as c_int) as *mut js_Buffer;
        (*sb).n = 0;
        (*sb).m = std::mem::size_of::<[c_char; 64]>() as c_int;
        *sbp = sb;
    } else if (*sb).n == (*sb).m {
        (*sb).m *= 2;
        let base = std::mem::offset_of!(js_Buffer, s) as c_int;
        sb = js_realloc(J, sb as *mut c_void, (*sb).m + base) as *mut js_Buffer;
        *sbp = sb;
    }
    *(*sb).s.as_mut_ptr().add((*sb).n as usize) = c as c_char;
    (*sb).n += 1;
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_puts(J: *mut js_State, sb: *mut *mut js_Buffer, s: *const c_char) {
    let mut s = s;
    while *s != 0 {
        js_putc(J, sb, *s as c_int);
        s = s.add(1);
    }
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_putm(J: *mut js_State, sb: *mut *mut js_Buffer, s: *const c_char, e: *const c_char) {
    let mut s = s;
    while s < e {
        js_putc(J, sb, *s as c_int);
        s = s.add(1);
    }
}

/* Interned string AA-tree. */
static mut jsS_sentinel: js_StringNode = js_StringNode {
    left: std::ptr::null_mut(),
    right: std::ptr::null_mut(),
    level: 0,
    string: [0],
};

unsafe fn sentinel() -> *mut js_StringNode {
    std::ptr::addr_of_mut!(jsS_sentinel)
}

unsafe fn init_sentinel() {
    if jsS_sentinel.left.is_null() {
        jsS_sentinel.left = sentinel();
        jsS_sentinel.right = sentinel();
    }
}

unsafe fn jsS_newstringnode(J: *mut js_State, string: *const c_char, result: *mut *const c_char) -> *mut js_StringNode {
    let n = strlen(string);
    if n as c_int > JS_STRLIMIT {
        crate::jserror::js_rangeerror(J, cstr!("invalid string length"));
    }
    let base = std::mem::offset_of!(js_StringNode, string);
    let node = js_malloc(J, (base + n + 1) as c_int) as *mut js_StringNode;
    (*node).left = sentinel();
    (*node).right = sentinel();
    (*node).level = 1;
    memcpy((*node).string.as_mut_ptr(), string, n + 1);
    *result = (*node).string.as_ptr();
    node
}

unsafe fn jsS_skew(mut node: *mut js_StringNode) -> *mut js_StringNode {
    if (*(*node).left).level == (*node).level {
        let temp = node;
        node = (*node).left;
        (*temp).left = (*node).right;
        (*node).right = temp;
    }
    node
}

unsafe fn jsS_split(mut node: *mut js_StringNode) -> *mut js_StringNode {
    if (*(*(*node).right).right).level == (*node).level {
        let temp = node;
        node = (*node).right;
        (*temp).right = (*node).left;
        (*node).left = temp;
        (*node).level += 1;
    }
    node
}

unsafe fn jsS_insert(J: *mut js_State, mut node: *mut js_StringNode, string: *const c_char, result: *mut *const c_char) -> *mut js_StringNode {
    if node != sentinel() {
        let c = strcmp(string, (*node).string.as_ptr());
        if c < 0 {
            (*node).left = jsS_insert(J, (*node).left, string, result);
        } else if c > 0 {
            (*node).right = jsS_insert(J, (*node).right, string, result);
        } else {
            *result = (*node).string.as_ptr();
            return node;
        }
        node = jsS_skew(node);
        node = jsS_split(node);
        return node;
    }
    jsS_newstringnode(J, string, result)
}

unsafe fn dumpstringnode(node: *mut js_StringNode, level: c_int) {
    if (*node).left != sentinel() {
        dumpstringnode((*node).left, level + 1);
    }
    libc::printf(cstr!("%d: "), (*node).level);
    let mut i = 0;
    while i < level {
        libc::putchar('\t' as c_int);
        i += 1;
    }
    libc::printf(cstr!("'%s'\n"), (*node).string.as_ptr());
    if (*node).right != sentinel() {
        dumpstringnode((*node).right, level + 1);
    }
}

#[no_mangle]
pub unsafe extern "C-unwind" fn jsS_dumpstrings(J: *mut js_State) {
    init_sentinel();
    let root = (*J).strings;
    libc::printf(cstr!("interned strings {\n"));
    if !root.is_null() && root != sentinel() {
        dumpstringnode(root, 1);
    }
    libc::printf(cstr!("}\n"));
}

unsafe fn jsS_freestringnode(J: *mut js_State, node: *mut js_StringNode) {
    if (*node).left != sentinel() {
        jsS_freestringnode(J, (*node).left);
    }
    if (*node).right != sentinel() {
        jsS_freestringnode(J, (*node).right);
    }
    js_free(J, node as *mut c_void);
}

#[no_mangle]
pub unsafe extern "C-unwind" fn jsS_freestrings(J: *mut js_State) {
    init_sentinel();
    if !(*J).strings.is_null() && (*J).strings != sentinel() {
        jsS_freestringnode(J, (*J).strings);
    }
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_intern(J: *mut js_State, s: *const c_char) -> *const c_char {
    init_sentinel();
    let mut result: *const c_char = std::ptr::null();
    if (*J).strings.is_null() {
        (*J).strings = sentinel();
    }
    (*J).strings = jsS_insert(J, (*J).strings, s, &mut result);
    result
}
