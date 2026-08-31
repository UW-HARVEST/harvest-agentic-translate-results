//! Translation of jsintern.c

use crate::jsi::*;
use crate::jsrun::{js_free, js_malloc, js_realloc};

/* Dynamically grown string buffer */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_putc(J: *mut js_State, sbp: *mut *mut js_Buffer, c: c_int) {
    let mut sb: *mut js_Buffer = *sbp;
    if sb.is_null() {
        sb = js_malloc(J, core::mem::size_of::<js_Buffer>() as c_int) as *mut js_Buffer;
        (*sb).n = 0;
        (*sb).m = 64; /* sizeof sb->s */
        *sbp = sb;
    } else if (*sb).n == (*sb).m {
        (*sb).m *= 2;
        sb = js_realloc(J, sb as *mut c_void, (*sb).m + OFF_BUFFER_S) as *mut js_Buffer;
        *sbp = sb;
    }
    let n = (*sb).n;
    *(addr_of_mut!((*sb).s) as *mut c_char).offset(n as isize) = c as c_char;
    (*sb).n = n + 1;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_puts(J: *mut js_State, sb: *mut *mut js_Buffer, s: *const c_char) {
    let mut s = s;
    while *s != 0 {
        js_putc(J, sb, *s as c_int);
        s = s.add(1);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_putm(
    J: *mut js_State,
    sb: *mut *mut js_Buffer,
    s: *const c_char,
    e: *const c_char,
) {
    let mut s = s;
    while s < e {
        js_putc(J, sb, *s as c_int);
        s = s.add(1);
    }
}

/* Use an AA-tree to quickly look up interned strings. */

static mut jsS_sentinel: js_StringNode = js_StringNode {
    left: null_mut(),
    right: null_mut(),
    level: 0,
    string: [0],
};

#[inline]
unsafe fn sentinel() -> *mut js_StringNode {
    let p = addr_of_mut!(jsS_sentinel);
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
    let n = strlen(string);
    if n > JS_STRLIMIT {
        js_rangeerror!(J, "invalid string length");
    }
    let node =
        js_malloc(J, (OFF_STRINGNODE_STRING + n + 1) as c_int) as *mut js_StringNode;
    (*node).left = sentinel();
    (*node).right = sentinel();
    (*node).level = 1;
    memcpy(
        nodestring(node) as *mut c_void,
        string as *const c_void,
        n + 1,
    );
    *result = nodestring(node);
    node
}

unsafe fn jsS_skew(node: *mut js_StringNode) -> *mut js_StringNode {
    let mut node = node;
    if (*(*node).left).level == (*node).level {
        let temp = node;
        node = (*node).left;
        (*temp).left = (*node).right;
        (*node).right = temp;
    }
    node
}

unsafe fn jsS_split(node: *mut js_StringNode) -> *mut js_StringNode {
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

unsafe fn jsS_insert(
    J: *mut js_State,
    node: *mut js_StringNode,
    string: *const c_char,
    result: *mut *const c_char,
) -> *mut js_StringNode {
    let mut node = node;
    if node != sentinel() {
        let c = strcmp(string, nodestring(node));
        if c < 0 {
            (*node).left = jsS_insert(J, (*node).left, string, result);
        } else if c > 0 {
            (*node).right = jsS_insert(J, (*node).right, string, result);
        } else {
            *result = nodestring(node);
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
    printf(cs!("%d: "), (*node).level);
    let mut i = 0;
    while i < level {
        putchar('\t' as c_int);
        i += 1;
    }
    printf(cs!("'%s'\n"), nodestring(node));
    if (*node).right != sentinel() {
        dumpstringnode((*node).right, level + 1);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsS_dumpstrings(J: *mut js_State) {
    let root = (*J).strings;
    printf(cs!("interned strings {\n"));
    if !root.is_null() && root != sentinel() {
        dumpstringnode(root, 1);
    }
    printf(cs!("}\n"));
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
pub unsafe extern "C" fn jsS_freestrings(J: *mut js_State) {
    if !(*J).strings.is_null() && (*J).strings != sentinel() {
        jsS_freestringnode(J, (*J).strings);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_intern(J: *mut js_State, s: *const c_char) -> *const c_char {
    let mut result: *const c_char = null();
    if (*J).strings.is_null() {
        (*J).strings = sentinel();
    }
    (*J).strings = jsS_insert(J, (*J).strings, s, &mut result);
    result
}
