//! Translation of `c_src/src/jsintern.c`
#![allow(non_snake_case)]

use crate::cstd::*;
use crate::jsi::*;
use crate::jsrun::{js_free, js_malloc, js_realloc};
use core::ptr::{null, null_mut};

/* Dynamically grown string buffer */

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_putc(J: *mut js_State, sbp: *mut *mut js_Buffer, c: c_int) {
    let mut sb: *mut js_Buffer = *sbp;
    if sb.is_null() {
        sb = js_malloc(J, core::mem::size_of::<js_Buffer>() as c_int) as *mut js_Buffer;
        (*sb).n = 0;
        (*sb).m = core::mem::size_of::<[c_char; 64]>() as c_int;
        *sbp = sb;
    } else if (*sb).n == (*sb).m {
        (*sb).m *= 2;
        sb = js_realloc(
            J,
            sb as *mut c_void,
            (*sb).m + JS_BUFFER_S_OFFSET as c_int,
        ) as *mut js_Buffer;
        *sbp = sb;
    }
    /* sb->s[sb->n++] = c; -- writes past the declared array once grown */
    let n = (*sb).n;
    (*sb).n = n + 1;
    *(*sb).s.as_mut_ptr().offset(n as isize) = c as c_char;
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_puts(
    J: *mut js_State,
    sb: *mut *mut js_Buffer,
    mut s: *const c_char,
) {
    while *s != 0 {
        let c = *s;
        s = s.offset(1);
        js_putc(J, sb, c as c_int);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_putm(
    J: *mut js_State,
    sb: *mut *mut js_Buffer,
    mut s: *const c_char,
    e: *const c_char,
) {
    while s < e {
        let c = *s;
        s = s.offset(1);
        js_putc(J, sb, c as c_int);
    }
}

/* Use an AA-tree to quickly look up interned strings. */

/// `static js_StringNode jsS_sentinel = { &jsS_sentinel, &jsS_sentinel, 0, ""};`
static mut STR_SENTINEL: js_StringNode = js_StringNode {
    left: null_mut(),
    right: null_mut(),
    level: 0,
    string: [0],
};

#[inline]
unsafe fn sentinel() -> *mut js_StringNode {
    let p = core::ptr::addr_of_mut!(STR_SENTINEL);
    if (*p).left.is_null() {
        (*p).left = p;
        (*p).right = p;
    }
    p
}

unsafe fn jsS_newstringnode(
    J: *mut js_State,
    string: *const c_char,
    result: *mut *const c_char,
) -> *mut js_StringNode {
    let n: size_t = strlen(string);
    if n > JS_STRLIMIT as size_t {
        js_rangeerror!(J, c"invalid string length".as_ptr());
    }
    let node = js_malloc(J, (JS_STRINGNODE_STRING_OFFSET + n + 1) as c_int) as *mut js_StringNode;
    (*node).right = sentinel();
    (*node).left = (*node).right;
    (*node).level = 1;
    memcpy(
        (*node).string.as_mut_ptr() as *mut c_void,
        string as *const c_void,
        n + 1,
    );
    *result = (*node).string.as_ptr() as *const c_char;
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

unsafe fn jsS_insert(
    J: *mut js_State,
    mut node: *mut js_StringNode,
    string: *const c_char,
    result: *mut *const c_char,
) -> *mut js_StringNode {
    if node != sentinel() {
        let c = strcmp(string, (*node).string.as_ptr());
        if c < 0 {
            (*node).left = jsS_insert(J, (*node).left, string, result);
        } else if c > 0 {
            (*node).right = jsS_insert(J, (*node).right, string, result);
        } else {
            *result = (*node).string.as_ptr() as *const c_char;
            return node;
        }
        node = jsS_skew(node);
        node = jsS_split(node);
        return node;
    }
    jsS_newstringnode(J, string, result)
}

unsafe fn dumpstringnode(node: *mut js_StringNode, level: c_int) {
    let mut i: c_int;
    if (*node).left != sentinel() {
        dumpstringnode((*node).left, level + 1);
    }
    printf(c"%d: ".as_ptr(), (*node).level);
    i = 0;
    while i < level {
        putchar('\t' as c_int);
        i += 1;
    }
    printf(c"'%s'\n".as_ptr(), (*node).string.as_ptr());
    if (*node).right != sentinel() {
        dumpstringnode((*node).right, level + 1);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsS_dumpstrings(J: *mut js_State) {
    let root: *mut js_StringNode = (*J).strings;
    printf(c"interned strings {\n".as_ptr());
    if !root.is_null() && root != sentinel() {
        dumpstringnode(root, 1);
    }
    printf(c"}\n".as_ptr());
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

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsS_freestrings(J: *mut js_State) {
    if !(*J).strings.is_null() && (*J).strings != sentinel() {
        jsS_freestringnode(J, (*J).strings);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_intern(J: *mut js_State, s: *const c_char) -> *const c_char {
    let mut result: *const c_char = null();
    if (*J).strings.is_null() {
        (*J).strings = sentinel();
    }
    (*J).strings = jsS_insert(J, (*J).strings, s, &mut result as *mut *const c_char);
    result
}
