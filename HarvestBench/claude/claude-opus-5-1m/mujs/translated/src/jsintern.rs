//! Translated from c_src/src/jsintern.c
use crate::jsi::*;
use crate::prelude::*;

/* Dynamically grown string buffer */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_putc(J: *mut js_State, sbp: *mut *mut js_Buffer, c: c_int) {
    let mut sb: *mut js_Buffer = *sbp;
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
            (*sb).m + SOFFSETOF_JS_BUFFER_S,
        ) as *mut js_Buffer;
        *sbp = sb;
    }
    *js_Buffer_s(sb).offset((*sb).n as isize) = c as c_char;
    (*sb).n += 1;
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

/// `static js_StringNode jsS_sentinel = { &jsS_sentinel, &jsS_sentinel, 0, ""};`
/// Rust statics cannot reference themselves, so the self pointers are filled
/// in on first use.
#[inline(always)]
unsafe fn sentinel() -> *mut js_StringNode {
    let p = std::ptr::addr_of_mut!(jsS_sentinel);
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
    let n: usize = strlen(string);
    if n > JS_STRLIMIT as usize {
        js_rangeerror!(J, c"invalid string length".as_ptr());
    }
    let node: *mut js_StringNode = js_malloc(
        J,
        SOFFSETOF_JS_STRINGNODE_STRING + n as c_int + 1,
    ) as *mut js_StringNode;
    (*node).right = sentinel();
    (*node).left = (*node).right;
    (*node).level = 1;
    memcpy(
        js_StringNode_string(node) as *mut c_void,
        string as *const c_void,
        n + 1,
    );
    *result = js_StringNode_string(node) as *const c_char;
    node
}

unsafe fn jsS_skew(node: *mut js_StringNode) -> *mut js_StringNode {
    let mut node = node;
    if (*(*node).left).level == (*node).level {
        let temp: *mut js_StringNode = node;
        node = (*node).left;
        (*temp).left = (*node).right;
        (*node).right = temp;
    }
    node
}

unsafe fn jsS_split(node: *mut js_StringNode) -> *mut js_StringNode {
    let mut node = node;
    if (*(*(*node).right).right).level == (*node).level {
        let temp: *mut js_StringNode = node;
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
        let c: c_int = strcmp(string, js_StringNode_string(node) as *const c_char);
        if c < 0 {
            (*node).left = jsS_insert(J, (*node).left, string, result);
        } else if c > 0 {
            (*node).right = jsS_insert(J, (*node).right, string, result);
        } else {
            *result = js_StringNode_string(node) as *const c_char;
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
    printf(c"'%s'\n".as_ptr(), js_StringNode_string(node));
    if (*node).right != sentinel() {
        dumpstringnode((*node).right, level + 1);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsS_dumpstrings(J: *mut js_State) {
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
