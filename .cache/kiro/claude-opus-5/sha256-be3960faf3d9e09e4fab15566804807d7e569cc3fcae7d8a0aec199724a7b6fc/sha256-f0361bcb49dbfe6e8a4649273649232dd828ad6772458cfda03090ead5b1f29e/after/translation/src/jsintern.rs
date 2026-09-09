//! Translation of src/jsintern.c
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(unused)]

use crate::jsi::*;
use crate::jsrun::{js_free, js_malloc, js_realloc};
use core::ptr::null_mut;

/* Dynamically grown string buffer */

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_putc(J: *mut js_State, sbp: *mut *mut js_Buffer, c: c_int) {
    unsafe {
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
                (*sb).m + JS_BUFFER_SOFF,
            ) as *mut js_Buffer;
            *sbp = sb;
        }
        *sbs(sb).offset((*sb).n as isize) = c as c_char;
        (*sb).n += 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_puts(J: *mut js_State, sb: *mut *mut js_Buffer, mut s: *const c_char) {
    unsafe {
        while *s != 0 {
            js_putc(J, sb, *s as c_int);
            s = s.offset(1);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_putm(
    J: *mut js_State,
    sb: *mut *mut js_Buffer,
    mut s: *const c_char,
    e: *const c_char,
) {
    unsafe {
        while s < e {
            js_putc(J, sb, *s as c_int);
            s = s.offset(1);
        }
    }
}

/* Use an AA-tree to quickly look up interned strings. */

/* struct js_StringNode is declared in crate::jsi (fields left, right, level, string). */

static mut jsS_sentinel: js_StringNode = js_StringNode {
    left: null_mut(),
    right: null_mut(),
    level: 0,
    string: [0],
};

/* The C file declares a self-referential file-scope sentinel:
 *   static js_StringNode jsS_sentinel = { &jsS_sentinel, &jsS_sentinel, 0, "" };
 * Rust cannot express the self reference in a `static` initialiser, so we lazily
 * fix up left/right to point at the sentinel itself on first use and return its
 * address.  jsS_skew/jsS_split dereference node->left->level and
 * node->right->right->level, so the sentinel's left and right MUST point at the
 * sentinel itself. */
unsafe fn sentinel() -> *mut js_StringNode {
    unsafe {
        let s = &raw mut jsS_sentinel;
        if (*s).left.is_null() {
            (*s).left = s;
            (*s).right = s;
        }
        s
    }
}

unsafe fn jsS_newstringnode(
    J: *mut js_State,
    string: *const c_char,
    result: *mut *const c_char,
) -> *mut js_StringNode {
    unsafe {
        let n: size_t = strlen(string);
        if n as c_int > JS_STRLIMIT {
            js_rangeerror!(J, c"invalid string length".as_ptr());
        }
        let node: *mut js_StringNode =
            js_malloc(J, JS_STRINGNODE_STROFF + n as c_int + 1) as *mut js_StringNode;
        (*node).left = sentinel();
        (*node).right = sentinel();
        (*node).level = 1;
        let nodestr = (&raw mut (*node).string) as *mut c_char;
        memcpy(nodestr as *mut c_void, string as *const c_void, n + 1);
        *result = nodestr;
        node
    }
}

unsafe fn jsS_skew(mut node: *mut js_StringNode) -> *mut js_StringNode {
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

unsafe fn jsS_split(mut node: *mut js_StringNode) -> *mut js_StringNode {
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

unsafe fn jsS_insert(
    J: *mut js_State,
    mut node: *mut js_StringNode,
    string: *const c_char,
    result: *mut *const c_char,
) -> *mut js_StringNode {
    unsafe {
        if node != sentinel() {
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
        printf(c"'%s'\n".as_ptr(), (&raw const (*node).string) as *const c_char);
        if (*node).right != sentinel() {
            dumpstringnode((*node).right, level + 1);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsS_dumpstrings(J: *mut js_State) {
    unsafe {
        let root: *mut js_StringNode = (*J).strings;
        printf(c"interned strings {\n".as_ptr());
        if !root.is_null() && root != sentinel() {
            dumpstringnode(root, 1);
        }
        printf(c"}\n".as_ptr());
    }
}

unsafe fn jsS_freestringnode(J: *mut js_State, node: *mut js_StringNode) {
    unsafe {
        if (*node).left != sentinel() {
            jsS_freestringnode(J, (*node).left);
        }
        if (*node).right != sentinel() {
            jsS_freestringnode(J, (*node).right);
        }
        js_free(J, node as *mut c_void);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsS_freestrings(J: *mut js_State) {
    unsafe {
        if !(*J).strings.is_null() && (*J).strings != sentinel() {
            jsS_freestringnode(J, (*J).strings);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_intern(J: *mut js_State, s: *const c_char) -> *const c_char {
    unsafe {
        let mut result: *const c_char = core::ptr::null();
        if (*J).strings.is_null() {
            (*J).strings = sentinel();
        }
        (*J).strings = jsS_insert(J, (*J).strings, s, &raw mut result);
        result
    }
}
