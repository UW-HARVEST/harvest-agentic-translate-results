//! Translation of jsrun.c

use crate::enums::*;
use crate::jsarray::{js_getlength, js_setlength};
use crate::jsgc::js_gc;
use crate::jsi::*;
use crate::jsintern::js_intern;
use crate::jsproperty::*;
use crate::jsregexp::js_newregexp;
use crate::jsstate::js_loadeval;
use crate::jsstring::js_runeat;
use crate::jsvalue::*;
use crate::utf::{runetochar, UTFmax};

/* Push values on stack */

macro_rules! STACK {
    ($J:expr) => {
        (*$J).stack
    };
}
macro_rules! TOP {
    ($J:expr) => {
        (*$J).top
    };
}
macro_rules! BOT {
    ($J:expr) => {
        (*$J).bot
    };
}
/// STACK[i]
macro_rules! ST {
    ($J:expr, $i:expr) => {
        *(*$J).stack.offset($i as isize)
    };
}
macro_rules! CHECKSTACK {
    ($J:expr, $n:expr) => {
        if TOP!($J) + $n >= JS_STACKSIZE {
            js_stackoverflow($J);
        }
    };
}

unsafe fn js_trystackoverflow(J: *mut js_State) -> ! {
    setvtype(addr_of_mut!(ST!(J, TOP!(J))), JS_TLITSTR);
    ST!(J, TOP!(J)).u.litstr = cs!("exception stack overflow");
    TOP!(J) += 1;
    js_throw(J)
}

unsafe fn js_stackoverflow(J: *mut js_State) -> ! {
    setvtype(addr_of_mut!(ST!(J, TOP!(J))), JS_TLITSTR);
    ST!(J, TOP!(J)).u.litstr = cs!("stack overflow");
    TOP!(J) += 1;
    js_throw(J)
}

unsafe fn js_outofmemory(J: *mut js_State) -> ! {
    setvtype(addr_of_mut!(ST!(J, TOP!(J))), JS_TLITSTR);
    ST!(J, TOP!(J)).u.litstr = cs!("out of memory");
    TOP!(J) += 1;
    js_throw(J)
}

unsafe fn js_runlimit(J: *mut js_State) -> ! {
    setvtype(addr_of_mut!(ST!(J, TOP!(J))), JS_TLITSTR);
    ST!(J, TOP!(J)).u.litstr = cs!("script ran too long");
    TOP!(J) += 1;
    js_throw(J)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_setlimit(J: *mut js_State, runlimit: c_int, memlimit: c_int) {
    (*J).runlimit = runlimit;
    (*J).memlimit = memlimit;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_malloc(J: *mut js_State, size: c_int) -> *mut c_void {
    let mut ptr: *mut c_void;
    if (*J).memlimit > 0 {
        if size >= (*J).memlimit {
            js_outofmemory(J);
        }
        (*J).memlimit -= size;
    }
    ptr = ((*J).alloc.unwrap())((*J).actx, null_mut(), size);
    if ptr.is_null() {
        js_outofmemory(J);
    }
    ptr
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_realloc(
    J: *mut js_State,
    ptr: *mut c_void,
    size: c_int,
) -> *mut c_void {
    let mut ptr = ptr;
    if (*J).memlimit > 0 {
        if size >= (*J).memlimit {
            js_outofmemory(J);
        }
        (*J).memlimit -= size;
    }
    ptr = ((*J).alloc.unwrap())((*J).actx, ptr, size);
    if ptr.is_null() {
        js_outofmemory(J);
    }
    ptr
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_strdup(J: *mut js_State, s: *const c_char) -> *mut c_char {
    let n = strlen(s) + 1;
    let p = js_malloc(J, n as c_int) as *mut c_char;
    memcpy(p as *mut c_void, s as *const c_void, n);
    p
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_free(J: *mut js_State, ptr: *mut c_void) {
    ((*J).alloc.unwrap())((*J).actx, ptr, 0);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsV_newmemstring(
    J: *mut js_State,
    s: *const c_char,
    n: c_int,
) -> *mut js_String {
    let v = js_malloc(J, (OFF_STRING_P as c_int) + n + 1) as *mut js_String;
    memcpy(strp(v) as *mut c_void, s as *const c_void, n as usize);
    *strp(v).offset(n as isize) = 0;
    (*v).gcmark = 0;
    (*v).gcnext = (*J).gcstr;
    (*J).gcstr = v;
    (*J).gccounter += 1;
    v
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_pushvalue(J: *mut js_State, v: js_Value) {
    CHECKSTACK!(J, 1);
    ST!(J, TOP!(J)) = v;
    TOP!(J) += 1;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_pushundefined(J: *mut js_State) {
    CHECKSTACK!(J, 1);
    setvtype(addr_of_mut!(ST!(J, TOP!(J))), JS_TUNDEFINED);
    TOP!(J) += 1;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_pushnull(J: *mut js_State) {
    CHECKSTACK!(J, 1);
    setvtype(addr_of_mut!(ST!(J, TOP!(J))), JS_TNULL);
    TOP!(J) += 1;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_pushboolean(J: *mut js_State, v: c_int) {
    CHECKSTACK!(J, 1);
    setvtype(addr_of_mut!(ST!(J, TOP!(J))), JS_TBOOLEAN);
    ST!(J, TOP!(J)).u.boolean = (v != 0) as c_int;
    TOP!(J) += 1;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_pushnumber(J: *mut js_State, v: f64) {
    CHECKSTACK!(J, 1);
    setvtype(addr_of_mut!(ST!(J, TOP!(J))), JS_TNUMBER);
    ST!(J, TOP!(J)).u.number = v;
    TOP!(J) += 1;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_pushstring(J: *mut js_State, v: *const c_char) {
    let mut v = v;
    let mut n = strlen(v);
    if n > JS_STRLIMIT {
        js_rangeerror!(J, "invalid string length");
    }
    CHECKSTACK!(J, 1);
    if n <= OFF_VALUE_TYPE as usize {
        let mut s = shrstrp(addr_of_mut!(ST!(J, TOP!(J))));
        while n != 0 {
            n -= 1;
            *s = *v;
            s = s.add(1);
            v = v.add(1);
        }
        *s = 0;
        setvtype(addr_of_mut!(ST!(J, TOP!(J))), JS_TSHRSTR);
    } else {
        setvtype(addr_of_mut!(ST!(J, TOP!(J))), JS_TMEMSTR);
        ST!(J, TOP!(J)).u.memstr = jsV_newmemstring(J, v, n as c_int);
    }
    TOP!(J) += 1;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_pushlstring(J: *mut js_State, v: *const c_char, n: c_int) {
    let mut v = v;
    let mut n = n;
    if n as usize > JS_STRLIMIT {
        js_rangeerror!(J, "invalid string length");
    }
    CHECKSTACK!(J, 1);
    if n <= OFF_VALUE_TYPE {
        let mut s = shrstrp(addr_of_mut!(ST!(J, TOP!(J))));
        while n != 0 {
            n -= 1;
            *s = *v;
            s = s.add(1);
            v = v.add(1);
        }
        *s = 0;
        setvtype(addr_of_mut!(ST!(J, TOP!(J))), JS_TSHRSTR);
    } else {
        setvtype(addr_of_mut!(ST!(J, TOP!(J))), JS_TMEMSTR);
        ST!(J, TOP!(J)).u.memstr = jsV_newmemstring(J, v, n);
    }
    TOP!(J) += 1;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_pushliteral(J: *mut js_State, v: *const c_char) {
    CHECKSTACK!(J, 1);
    setvtype(addr_of_mut!(ST!(J, TOP!(J))), JS_TLITSTR);
    ST!(J, TOP!(J)).u.litstr = v;
    TOP!(J) += 1;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_pushobject(J: *mut js_State, v: *mut js_Object) {
    CHECKSTACK!(J, 1);
    setvtype(addr_of_mut!(ST!(J, TOP!(J))), JS_TOBJECT);
    ST!(J, TOP!(J)).u.object = v;
    TOP!(J) += 1;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_pushglobal(J: *mut js_State) {
    js_pushobject(J, (*J).G);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_currentfunction(J: *mut js_State) {
    CHECKSTACK!(J, 1);
    if BOT!(J) > 0 {
        ST!(J, TOP!(J)) = ST!(J, BOT!(J) - 1);
    } else {
        setvtype(addr_of_mut!(ST!(J, TOP!(J))), JS_TUNDEFINED);
    }
    TOP!(J) += 1;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_currentfunctiondata(J: *mut js_State) -> *mut c_void {
    if BOT!(J) > 0 {
        return (*ST!(J, BOT!(J) - 1).u.object).u.c.data;
    }
    null_mut()
}

/* Read values from stack */

static mut UNDEFINED_VALUE: js_Value = js_Value::undefined();

unsafe fn stackidx(J: *mut js_State, idx: c_int) -> *mut js_Value {
    let idx = if idx < 0 {
        TOP!(J) + idx
    } else {
        BOT!(J) + idx
    };
    if idx < 0 || idx >= TOP!(J) {
        return addr_of_mut!(UNDEFINED_VALUE);
    }
    STACK!(J).offset(idx as isize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_tovalue(J: *mut js_State, idx: c_int) -> *mut js_Value {
    stackidx(J, idx)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_isdefined(J: *mut js_State, idx: c_int) -> c_int {
    (vtype(stackidx(J, idx)) != JS_TUNDEFINED) as c_int
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_isundefined(J: *mut js_State, idx: c_int) -> c_int {
    (vtype(stackidx(J, idx)) == JS_TUNDEFINED) as c_int
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_isnull(J: *mut js_State, idx: c_int) -> c_int {
    (vtype(stackidx(J, idx)) == JS_TNULL) as c_int
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_isboolean(J: *mut js_State, idx: c_int) -> c_int {
    (vtype(stackidx(J, idx)) == JS_TBOOLEAN) as c_int
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_isnumber(J: *mut js_State, idx: c_int) -> c_int {
    (vtype(stackidx(J, idx)) == JS_TNUMBER) as c_int
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_isstring(J: *mut js_State, idx: c_int) -> c_int {
    let t = vtype(stackidx(J, idx));
    (t == JS_TSHRSTR || t == JS_TLITSTR || t == JS_TMEMSTR) as c_int
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_isprimitive(J: *mut js_State, idx: c_int) -> c_int {
    (vtype(stackidx(J, idx)) != JS_TOBJECT) as c_int
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_isobject(J: *mut js_State, idx: c_int) -> c_int {
    (vtype(stackidx(J, idx)) == JS_TOBJECT) as c_int
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_iscoercible(J: *mut js_State, idx: c_int) -> c_int {
    let v = stackidx(J, idx);
    (vtype(v) != JS_TUNDEFINED && vtype(v) != JS_TNULL) as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_iscallable(J: *mut js_State, idx: c_int) -> c_int {
    let v = stackidx(J, idx);
    if vtype(v) == JS_TOBJECT {
        return ((*(*v).u.object).type_ == JS_CFUNCTION
            || (*(*v).u.object).type_ == JS_CSCRIPT
            || (*(*v).u.object).type_ == JS_CCFUNCTION) as c_int;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_isarray(J: *mut js_State, idx: c_int) -> c_int {
    let v = stackidx(J, idx);
    (vtype(v) == JS_TOBJECT && (*(*v).u.object).type_ == JS_CARRAY) as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_isregexp(J: *mut js_State, idx: c_int) -> c_int {
    let v = stackidx(J, idx);
    (vtype(v) == JS_TOBJECT && (*(*v).u.object).type_ == JS_CREGEXP) as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_isuserdata(J: *mut js_State, idx: c_int, tag: *const c_char) -> c_int {
    let v = stackidx(J, idx);
    if vtype(v) == JS_TOBJECT && (*(*v).u.object).type_ == JS_CUSERDATA {
        return (strcmp(tag, (*(*v).u.object).u.user.tag) == 0) as c_int;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_iserror(J: *mut js_State, idx: c_int) -> c_int {
    let v = stackidx(J, idx);
    (vtype(v) == JS_TOBJECT && (*(*v).u.object).type_ == JS_CERROR) as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_typeof(J: *mut js_State, idx: c_int) -> *const c_char {
    let v = stackidx(J, idx);
    match vtype(v) {
        JS_TUNDEFINED => cs!("undefined"),
        JS_TNULL => cs!("object"),
        JS_TBOOLEAN => cs!("boolean"),
        JS_TNUMBER => cs!("number"),
        JS_TLITSTR => cs!("string"),
        JS_TMEMSTR => cs!("string"),
        JS_TOBJECT => {
            if (*(*v).u.object).type_ == JS_CFUNCTION || (*(*v).u.object).type_ == JS_CCFUNCTION {
                cs!("function")
            } else {
                cs!("object")
            }
        }
        /* default and JS_TSHRSTR */
        _ => cs!("string"),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_type(J: *mut js_State, idx: c_int) -> c_int {
    let v = stackidx(J, idx);
    match vtype(v) {
        JS_TUNDEFINED => JS_ISUNDEFINED,
        JS_TNULL => JS_ISNULL,
        JS_TBOOLEAN => JS_ISBOOLEAN,
        JS_TNUMBER => JS_ISNUMBER,
        JS_TLITSTR => JS_ISSTRING,
        JS_TMEMSTR => JS_ISSTRING,
        JS_TOBJECT => {
            if (*(*v).u.object).type_ == JS_CFUNCTION || (*(*v).u.object).type_ == JS_CCFUNCTION {
                JS_ISFUNCTION
            } else {
                JS_ISOBJECT
            }
        }
        /* default and JS_TSHRSTR */
        _ => JS_ISSTRING,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_toboolean(J: *mut js_State, idx: c_int) -> c_int {
    jsV_toboolean(J, stackidx(J, idx))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_tonumber(J: *mut js_State, idx: c_int) -> f64 {
    jsV_tonumber(J, stackidx(J, idx))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_tointeger(J: *mut js_State, idx: c_int) -> c_int {
    jsV_numbertointeger(jsV_tonumber(J, stackidx(J, idx)))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_toint32(J: *mut js_State, idx: c_int) -> c_int {
    jsV_numbertoint32(jsV_tonumber(J, stackidx(J, idx)))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_touint32(J: *mut js_State, idx: c_int) -> c_uint {
    jsV_numbertouint32(jsV_tonumber(J, stackidx(J, idx)))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_toint16(J: *mut js_State, idx: c_int) -> c_short {
    jsV_numbertoint16(jsV_tonumber(J, stackidx(J, idx)))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_touint16(J: *mut js_State, idx: c_int) -> c_ushort {
    jsV_numbertouint16(jsV_tonumber(J, stackidx(J, idx)))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_tostring(J: *mut js_State, idx: c_int) -> *const c_char {
    jsV_tostring(J, stackidx(J, idx))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_toobject(J: *mut js_State, idx: c_int) -> *mut js_Object {
    jsV_toobject(J, stackidx(J, idx))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_toprimitive(J: *mut js_State, idx: c_int, hint: c_int) {
    jsV_toprimitive(J, stackidx(J, idx), hint);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_toregexp(J: *mut js_State, idx: c_int) -> *mut js_Regexp {
    let v = stackidx(J, idx);
    if vtype(v) == JS_TOBJECT && (*(*v).u.object).type_ == JS_CREGEXP {
        return addr_of_mut!((*(*v).u.object).u.r);
    }
    js_typeerror!(J, "not a regexp")
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_touserdata(
    J: *mut js_State,
    idx: c_int,
    tag: *const c_char,
) -> *mut c_void {
    let v = stackidx(J, idx);
    if vtype(v) == JS_TOBJECT && (*(*v).u.object).type_ == JS_CUSERDATA {
        if strcmp(tag, (*(*v).u.object).u.user.tag) == 0 {
            return (*(*v).u.object).u.user.data;
        }
    }
    js_typeerror!(J, "not a %s", tag)
}

unsafe fn jsR_tofunction(J: *mut js_State, idx: c_int) -> *mut js_Object {
    let v = stackidx(J, idx);
    if vtype(v) == JS_TUNDEFINED || vtype(v) == JS_TNULL {
        return null_mut();
    }
    if vtype(v) == JS_TOBJECT {
        if (*(*v).u.object).type_ == JS_CFUNCTION || (*(*v).u.object).type_ == JS_CCFUNCTION {
            return (*v).u.object;
        }
    }
    js_typeerror!(J, "not a function")
}

/* Stack manipulation */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_gettop(J: *mut js_State) -> c_int {
    TOP!(J) - BOT!(J)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_pop(J: *mut js_State, n: c_int) {
    TOP!(J) -= n;
    if TOP!(J) < BOT!(J) {
        TOP!(J) = BOT!(J);
        js_error!(J, "stack underflow!");
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_remove(J: *mut js_State, idx: c_int) {
    let mut idx = if idx < 0 {
        TOP!(J) + idx
    } else {
        BOT!(J) + idx
    };
    if idx < BOT!(J) || idx >= TOP!(J) {
        js_error!(J, "stack error!");
    }
    while idx < TOP!(J) - 1 {
        ST!(J, idx) = ST!(J, idx + 1);
        idx += 1;
    }
    TOP!(J) -= 1;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_insert(J: *mut js_State, idx: c_int) {
    js_error!(J, "not implemented yet");
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_replace(J: *mut js_State, idx: c_int) {
    let idx = if idx < 0 {
        TOP!(J) + idx
    } else {
        BOT!(J) + idx
    };
    if idx < BOT!(J) || idx >= TOP!(J) {
        js_error!(J, "stack error!");
    }
    TOP!(J) -= 1;
    ST!(J, idx) = ST!(J, TOP!(J));
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_copy(J: *mut js_State, idx: c_int) {
    CHECKSTACK!(J, 1);
    ST!(J, TOP!(J)) = *stackidx(J, idx);
    TOP!(J) += 1;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_dup(J: *mut js_State) {
    CHECKSTACK!(J, 1);
    ST!(J, TOP!(J)) = ST!(J, TOP!(J) - 1);
    TOP!(J) += 1;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_dup2(J: *mut js_State) {
    CHECKSTACK!(J, 2);
    ST!(J, TOP!(J)) = ST!(J, TOP!(J) - 2);
    ST!(J, TOP!(J) + 1) = ST!(J, TOP!(J) - 1);
    TOP!(J) += 2;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_rot2(J: *mut js_State) {
    /* A B -> B A */
    let tmp = ST!(J, TOP!(J) - 1);
    ST!(J, TOP!(J) - 1) = ST!(J, TOP!(J) - 2);
    ST!(J, TOP!(J) - 2) = tmp;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_rot3(J: *mut js_State) {
    /* A B C -> C A B */
    let tmp = ST!(J, TOP!(J) - 1);
    ST!(J, TOP!(J) - 1) = ST!(J, TOP!(J) - 2);
    ST!(J, TOP!(J) - 2) = ST!(J, TOP!(J) - 3);
    ST!(J, TOP!(J) - 3) = tmp;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_rot4(J: *mut js_State) {
    /* A B C D -> D A B C */
    let tmp = ST!(J, TOP!(J) - 1);
    ST!(J, TOP!(J) - 1) = ST!(J, TOP!(J) - 2);
    ST!(J, TOP!(J) - 2) = ST!(J, TOP!(J) - 3);
    ST!(J, TOP!(J) - 3) = ST!(J, TOP!(J) - 4);
    ST!(J, TOP!(J) - 4) = tmp;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_rot2pop1(J: *mut js_State) {
    /* A B -> B */
    ST!(J, TOP!(J) - 2) = ST!(J, TOP!(J) - 1);
    TOP!(J) -= 1;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_rot3pop2(J: *mut js_State) {
    /* A B C -> C */
    ST!(J, TOP!(J) - 3) = ST!(J, TOP!(J) - 1);
    TOP!(J) -= 2;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_rot(J: *mut js_State, n: c_int) {
    let mut i: c_int;
    let tmp = ST!(J, TOP!(J) - 1);
    i = 1;
    while i < n {
        ST!(J, TOP!(J) - i) = ST!(J, TOP!(J) - i - 1);
        i += 1;
    }
    ST!(J, TOP!(J) - i) = tmp;
}

/* Property access that takes care of attributes and getters/setters */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_isarrayindex(
    J: *mut js_State,
    p: *const c_char,
    idx: *mut c_int,
) -> c_int {
    let mut n: c_int = 0;
    let mut p = p;

    /* check for empty string */
    if *p.add(0) == 0 {
        return 0;
    }

    /* check for '0' and integers with leading zero */
    if *p.add(0) == '0' as c_char {
        if *p.add(1) == 0 {
            *idx = 0;
            return 1;
        }
        return 0;
    }

    while *p != 0 {
        let c = *p as c_int;
        p = p.add(1);
        if c >= '0' as c_int && c <= '9' as c_int {
            if n >= INT_MAX / 10 {
                return 0;
            }
            n = n * 10 + (c - '0' as c_int);
        } else {
            return 0;
        }
    }
    *idx = n;
    1
}

unsafe fn js_pushrune(J: *mut js_State, rune: Rune) {
    let mut buf: [c_char; (UTFmax + 1) as usize] = [0; (UTFmax + 1) as usize];
    if rune >= 0 {
        let n = runetochar(buf.as_mut_ptr(), &rune);
        buf[n as usize] = 0;
        js_pushstring(J, buf.as_ptr());
    } else {
        js_pushundefined(J);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsR_unflattenarray(J: *mut js_State, obj: *mut js_Object) {
    if (*obj).type_ == JS_CARRAY && (*obj).u.a.simple != 0 {
        let mut refp: *mut js_Property;
        let mut i: c_int;
        let mut name: [c_char; 32] = [0; 32];
        if js_try!(J) != 0 {
            (*obj).properties = null_mut();
            js_throw(J);
        }
        i = 0;
        while i < (*obj).u.a.flat_length {
            js_itoa(name.as_mut_ptr(), i);
            refp = jsV_setproperty(J, obj, name.as_ptr());
            (*refp).value = *(*obj).u.a.array.offset(i as isize);
            i += 1;
        }
        js_free(J, (*obj).u.a.array as *mut c_void);
        (*obj).u.a.simple = 0;
        (*obj).u.a.flat_length = 0;
        (*obj).u.a.flat_capacity = 0;
        (*obj).u.a.array = null_mut();
        js_endtry(J);
    }
}

unsafe fn jsR_hasproperty(J: *mut js_State, obj: *mut js_Object, name: *const c_char) -> c_int {
    let refp: *mut js_Property;
    let mut k: c_int = 0;

    if (*obj).type_ == JS_CARRAY {
        if strcmp(name, cs!("length")) == 0 {
            js_pushnumber(J, (*obj).u.a.length as f64);
            return 1;
        }
        if (*obj).u.a.simple != 0 {
            if js_isarrayindex(J, name, &mut k) != 0 {
                if k >= 0 && k < (*obj).u.a.flat_length {
                    js_pushvalue(J, *(*obj).u.a.array.offset(k as isize));
                    return 1;
                }
                return 0;
            }
        }
    } else if (*obj).type_ == JS_CSTRING {
        if strcmp(name, cs!("length")) == 0 {
            js_pushnumber(J, (*obj).u.s.length as f64);
            return 1;
        }
        if js_isarrayindex(J, name, &mut k) != 0 {
            if k >= 0 && k < (*obj).u.s.length {
                js_pushrune(J, js_runeat(J, (*obj).u.s.string, k));
                return 1;
            }
        }
    } else if (*obj).type_ == JS_CREGEXP {
        if strcmp(name, cs!("source")) == 0 {
            js_pushstring(J, (*obj).u.r.source);
            return 1;
        }
        if strcmp(name, cs!("global")) == 0 {
            js_pushboolean(J, (*obj).u.r.flags as c_int & JS_REGEXP_G);
            return 1;
        }
        if strcmp(name, cs!("ignoreCase")) == 0 {
            js_pushboolean(J, (*obj).u.r.flags as c_int & JS_REGEXP_I);
            return 1;
        }
        if strcmp(name, cs!("multiline")) == 0 {
            js_pushboolean(J, (*obj).u.r.flags as c_int & JS_REGEXP_M);
            return 1;
        }
        if strcmp(name, cs!("lastIndex")) == 0 {
            js_pushnumber(J, (*obj).u.r.last as f64);
            return 1;
        }
    } else if (*obj).type_ == JS_CUSERDATA {
        if let Some(has) = (*obj).u.user.has {
            if has(J, (*obj).u.user.data, name) != 0 {
                return 1;
            }
        }
    }

    refp = jsV_getproperty(J, obj, name);
    if !refp.is_null() {
        if !(*refp).getter.is_null() {
            js_pushobject(J, (*refp).getter);
            js_pushobject(J, obj);
            js_call(J, 0);
        } else {
            js_pushvalue(J, (*refp).value);
        }
        return 1;
    }

    0
}

unsafe fn jsR_getproperty(J: *mut js_State, obj: *mut js_Object, name: *const c_char) {
    if jsR_hasproperty(J, obj, name) == 0 {
        js_pushundefined(J);
    }
}

unsafe fn jsR_hasindex(J: *mut js_State, obj: *mut js_Object, k: c_int) -> c_int {
    let mut buf: [c_char; 32] = [0; 32];
    if (*obj).type_ == JS_CARRAY && (*obj).u.a.simple != 0 {
        if k >= 0 && k < (*obj).u.a.flat_length {
            js_pushvalue(J, *(*obj).u.a.array.offset(k as isize));
            return 1;
        }
        return 0;
    }
    jsR_hasproperty(J, obj, js_itoa(buf.as_mut_ptr(), k))
}

unsafe fn jsR_getindex(J: *mut js_State, obj: *mut js_Object, k: c_int) {
    if jsR_hasindex(J, obj, k) == 0 {
        js_pushundefined(J);
    }
}

unsafe fn jsR_setarrayindex(J: *mut js_State, obj: *mut js_Object, k: c_int, value: *mut js_Value) {
    let newlen = k + 1;
    if newlen > JS_ARRAYLIMIT {
        js_rangeerror!(J, "array too large");
    }
    if newlen > (*obj).u.a.flat_length {
        if newlen > (*obj).u.a.flat_capacity {
            let mut newcap = (*obj).u.a.flat_capacity;
            if newcap == 0 {
                newcap = 8;
            }
            while newcap < newlen {
                newcap <<= 1;
            }
            (*obj).u.a.array = js_realloc(
                J,
                (*obj).u.a.array as *mut c_void,
                (newcap as usize * core::mem::size_of::<js_Value>()) as c_int,
            ) as *mut js_Value;
            (*obj).u.a.flat_capacity = newcap;
        }
        (*obj).u.a.flat_length = newlen;
    }
    if newlen > (*obj).u.a.length {
        (*obj).u.a.length = newlen;
    }
    *(*obj).u.a.array.offset(k as isize) = *value;
}

unsafe fn jsR_setproperty(
    J: *mut js_State,
    obj: *mut js_Object,
    name: *const c_char,
    transient: c_int,
) {
    let value = stackidx(J, -1);
    let mut refp: *mut js_Property;
    let mut k: c_int = 0;
    let mut own: c_int = 0;
    let mut readonly = false;

    'body: {
        if (*obj).type_ == JS_CARRAY {
            if strcmp(name, cs!("length")) == 0 {
                let rawlen = jsV_tonumber(J, value);
                let newlen = jsV_numbertointeger(rawlen);
                if newlen as f64 != rawlen || newlen < 0 {
                    js_rangeerror!(J, "invalid array length");
                }
                if newlen > JS_ARRAYLIMIT {
                    js_rangeerror!(J, "array too large");
                }
                if (*obj).u.a.simple != 0 {
                    (*obj).u.a.length = newlen;
                    if newlen <= (*obj).u.a.flat_length {
                        (*obj).u.a.flat_length = newlen;
                    }
                } else {
                    jsV_resizearray(J, obj, newlen);
                }
                return;
            }

            if js_isarrayindex(J, name, &mut k) != 0 {
                if (*obj).u.a.simple != 0 {
                    if k >= 0 && k <= (*obj).u.a.flat_length {
                        jsR_setarrayindex(J, obj, k, value);
                    } else {
                        jsR_unflattenarray(J, obj);
                        if (*obj).u.a.length < k + 1 {
                            (*obj).u.a.length = k + 1;
                        }
                    }
                } else {
                    if (*obj).u.a.length < k + 1 {
                        (*obj).u.a.length = k + 1;
                    }
                }
            }
        } else if (*obj).type_ == JS_CSTRING {
            if strcmp(name, cs!("length")) == 0 {
                readonly = true;
                break 'body;
            }
            if js_isarrayindex(J, name, &mut k) != 0 {
                if k >= 0 && k < (*obj).u.s.length {
                    readonly = true;
                    break 'body;
                }
            }
        } else if (*obj).type_ == JS_CREGEXP {
            if strcmp(name, cs!("source")) == 0 {
                readonly = true;
                break 'body;
            }
            if strcmp(name, cs!("global")) == 0 {
                readonly = true;
                break 'body;
            }
            if strcmp(name, cs!("ignoreCase")) == 0 {
                readonly = true;
                break 'body;
            }
            if strcmp(name, cs!("multiline")) == 0 {
                readonly = true;
                break 'body;
            }
            if strcmp(name, cs!("lastIndex")) == 0 {
                /* C: obj->u.r.last = jsV_tointeger(J, value);
                 * jsV_tointeger yields a double already clamped to the int
                 * range, and the C double -> unsigned short conversion keeps
                 * only the low 16 bits (gcc: cvttsd2si then truncate), while a
                 * Rust `as c_ushort` cast would SATURATE. Truncate through
                 * i64 to reproduce the C wrap-around. */
                (*obj).u.r.last = jsV_tointeger(J, value) as i64 as c_ushort;
                return;
            }
        } else if (*obj).type_ == JS_CUSERDATA {
            if let Some(put) = (*obj).u.user.put {
                if put(J, (*obj).u.user.data, name) != 0 {
                    return;
                }
            }
        }

        /* First try to find a setter in prototype chain */
        refp = jsV_getpropertyx(J, obj, name, &mut own);
        if !refp.is_null() {
            if !(*refp).setter.is_null() {
                js_pushobject(J, (*refp).setter);
                js_pushobject(J, obj);
                js_pushvalue(J, *value);
                js_call(J, 1);
                js_pop(J, 1);
                return;
            } else {
                if (*J).strict != 0 {
                    if !(*refp).getter.is_null() {
                        js_typeerror!(
                            J,
                            "setting property '%s' that only has a getter",
                            name
                        );
                    }
                }
                if ((*refp).atts & JS_READONLY) != 0 {
                    readonly = true;
                    break 'body;
                }
            }
        }

        /* Property not found on this object, so create one */
        if refp.is_null() || own == 0 {
            if transient != 0 {
                if (*J).strict != 0 {
                    js_typeerror!(J, "cannot create property '%s' on transient object", name);
                }
                return;
            }
            refp = jsV_setproperty(J, obj, name);
        }

        if !refp.is_null() {
            if ((*refp).atts & JS_READONLY) == 0 {
                (*refp).value = *value;
            } else {
                readonly = true;
                break 'body;
            }
        }

        return;
    }

    /* readonly: */
    if (*J).strict != 0 {
        js_typeerror!(J, "'%s' is read-only", name);
    }
}

unsafe fn jsR_setindex(J: *mut js_State, obj: *mut js_Object, k: c_int, transient: c_int) {
    let mut buf: [c_char; 32] = [0; 32];
    if (*obj).type_ == JS_CARRAY && (*obj).u.a.simple != 0 && k >= 0 && k <= (*obj).u.a.flat_length {
        jsR_setarrayindex(J, obj, k, stackidx(J, -1));
    } else {
        jsR_setproperty(J, obj, js_itoa(buf.as_mut_ptr(), k), transient);
    }
}

unsafe fn jsR_defproperty(
    J: *mut js_State,
    obj: *mut js_Object,
    name: *const c_char,
    atts: c_int,
    value: *mut js_Value,
    getter: *mut js_Object,
    setter: *mut js_Object,
    throw: c_int,
) {
    let refp: *mut js_Property;
    let mut k: c_int = 0;

    'body: {
        if (*obj).type_ == JS_CARRAY {
            if strcmp(name, cs!("length")) == 0 {
                break 'body;
            }
            if (*obj).u.a.simple != 0 {
                jsR_unflattenarray(J, obj);
            }
        } else if (*obj).type_ == JS_CSTRING {
            if strcmp(name, cs!("length")) == 0 {
                break 'body;
            }
            if js_isarrayindex(J, name, &mut k) != 0 {
                if k >= 0 && k < (*obj).u.s.length {
                    break 'body;
                }
            }
        } else if (*obj).type_ == JS_CREGEXP {
            if strcmp(name, cs!("source")) == 0 {
                break 'body;
            }
            if strcmp(name, cs!("global")) == 0 {
                break 'body;
            }
            if strcmp(name, cs!("ignoreCase")) == 0 {
                break 'body;
            }
            if strcmp(name, cs!("multiline")) == 0 {
                break 'body;
            }
            if strcmp(name, cs!("lastIndex")) == 0 {
                break 'body;
            }
        } else if (*obj).type_ == JS_CUSERDATA {
            if let Some(put) = (*obj).u.user.put {
                if put(J, (*obj).u.user.data, name) != 0 {
                    return;
                }
            }
        }

        refp = jsV_setproperty(J, obj, name);
        if !refp.is_null() {
            if !value.is_null() {
                if ((*refp).atts & JS_READONLY) == 0 {
                    (*refp).value = *value;
                } else if (*J).strict != 0 {
                    js_typeerror!(J, "'%s' is read-only", name);
                }
            }
            if !getter.is_null() {
                if ((*refp).atts & JS_DONTCONF) == 0 {
                    (*refp).getter = getter;
                } else if (*J).strict != 0 {
                    js_typeerror!(J, "'%s' is non-configurable", name);
                }
            }
            if !setter.is_null() {
                if ((*refp).atts & JS_DONTCONF) == 0 {
                    (*refp).setter = setter;
                } else if (*J).strict != 0 {
                    js_typeerror!(J, "'%s' is non-configurable", name);
                }
            }
            (*refp).atts |= atts;
        }

        return;
    }

    /* readonly: */
    if (*J).strict != 0 || throw != 0 {
        js_typeerror!(J, "'%s' is read-only or non-configurable", name);
    }
}

unsafe fn jsR_delproperty(J: *mut js_State, obj: *mut js_Object, name: *const c_char) -> c_int {
    let refp: *mut js_Property;
    let mut k: c_int = 0;

    'body: {
        if (*obj).type_ == JS_CARRAY {
            if strcmp(name, cs!("length")) == 0 {
                break 'body;
            }
            if (*obj).u.a.simple != 0 {
                jsR_unflattenarray(J, obj);
            }
        } else if (*obj).type_ == JS_CSTRING {
            if strcmp(name, cs!("length")) == 0 {
                break 'body;
            }
            if js_isarrayindex(J, name, &mut k) != 0 {
                if k >= 0 && k < (*obj).u.s.length {
                    break 'body;
                }
            }
        } else if (*obj).type_ == JS_CREGEXP {
            if strcmp(name, cs!("source")) == 0 {
                break 'body;
            }
            if strcmp(name, cs!("global")) == 0 {
                break 'body;
            }
            if strcmp(name, cs!("ignoreCase")) == 0 {
                break 'body;
            }
            if strcmp(name, cs!("multiline")) == 0 {
                break 'body;
            }
            if strcmp(name, cs!("lastIndex")) == 0 {
                break 'body;
            }
        } else if (*obj).type_ == JS_CUSERDATA {
            if let Some(del) = (*obj).u.user.delete {
                if del(J, (*obj).u.user.data, name) != 0 {
                    return 1;
                }
            }
        }

        refp = jsV_getownproperty(J, obj, name);
        if !refp.is_null() {
            if ((*refp).atts & JS_DONTCONF) != 0 {
                break 'body;
            }
            jsV_delproperty(J, obj, name);
        }
        return 1;
    }

    /* dontconf: */
    if (*J).strict != 0 {
        js_typeerror!(J, "'%s' is non-configurable", name);
    }
    0
}

unsafe fn jsR_delindex(J: *mut js_State, obj: *mut js_Object, k: c_int) {
    let mut buf: [c_char; 32] = [0; 32];
    /* Allow deleting last element of a simple array without unflattening */
    if (*obj).type_ == JS_CARRAY && (*obj).u.a.simple != 0 && k == (*obj).u.a.flat_length - 1 {
        (*obj).u.a.flat_length = k;
    } else {
        jsR_delproperty(J, obj, js_itoa(buf.as_mut_ptr(), k));
    }
}

/* Registry, global and object property accessors */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_ref(J: *mut js_State) -> *const c_char {
    let v = stackidx(J, -1);
    let s: *const c_char;
    let mut buf: [c_char; 32] = [0; 32];
    match vtype(v) {
        JS_TUNDEFINED => s = cs!("_Undefined"),
        JS_TNULL => s = cs!("_Null"),
        JS_TBOOLEAN => {
            s = if (*v).u.boolean != 0 {
                cs!("_True")
            } else {
                cs!("_False")
            };
        }
        JS_TOBJECT => {
            sprintf(buf.as_mut_ptr(), cs!("%p"), (*v).u.object as *mut c_void);
            s = js_intern(J, buf.as_ptr());
        }
        _ => {
            sprintf(buf.as_mut_ptr(), cs!("%d"), (*J).nextref);
            (*J).nextref += 1;
            s = js_intern(J, buf.as_ptr());
        }
    }
    js_setregistry(J, s);
    s
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_unref(J: *mut js_State, refname: *const c_char) {
    js_delregistry(J, refname);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_getregistry(J: *mut js_State, name: *const c_char) {
    jsR_getproperty(J, (*J).R, name);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_setregistry(J: *mut js_State, name: *const c_char) {
    jsR_setproperty(J, (*J).R, name, 0);
    js_pop(J, 1);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_delregistry(J: *mut js_State, name: *const c_char) {
    jsR_delproperty(J, (*J).R, name);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_getglobal(J: *mut js_State, name: *const c_char) {
    jsR_getproperty(J, (*J).G, name);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_setglobal(J: *mut js_State, name: *const c_char) {
    jsR_setproperty(J, (*J).G, name, 0);
    js_pop(J, 1);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_defglobal(J: *mut js_State, name: *const c_char, atts: c_int) {
    jsR_defproperty(
        J,
        (*J).G,
        name,
        atts,
        stackidx(J, -1),
        null_mut(),
        null_mut(),
        0,
    );
    js_pop(J, 1);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_delglobal(J: *mut js_State, name: *const c_char) {
    jsR_delproperty(J, (*J).G, name);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_getproperty(J: *mut js_State, idx: c_int, name: *const c_char) {
    jsR_getproperty(J, js_toobject(J, idx), name);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_setproperty(J: *mut js_State, idx: c_int, name: *const c_char) {
    /* The C is
     *     jsR_setproperty(J, js_toobject(J, idx), name, !js_isobject(J, idx));
     * and the compiler evaluates the arguments right to left, so the
     * transient flag is computed BEFORE js_toobject() replaces the primitive
     * on the stack with its wrapper object. Keep that order. */
    let transient = (js_isobject(J, idx) == 0) as c_int;
    jsR_setproperty(J, js_toobject(J, idx), name, transient);
    js_pop(J, 1);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_defproperty(
    J: *mut js_State,
    idx: c_int,
    name: *const c_char,
    atts: c_int,
) {
    jsR_defproperty(
        J,
        js_toobject(J, idx),
        name,
        atts,
        stackidx(J, -1),
        null_mut(),
        null_mut(),
        1,
    );
    js_pop(J, 1);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_delproperty(J: *mut js_State, idx: c_int, name: *const c_char) {
    jsR_delproperty(J, js_toobject(J, idx), name);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_defaccessor(
    J: *mut js_State,
    idx: c_int,
    name: *const c_char,
    atts: c_int,
) {
    jsR_defproperty(
        J,
        js_toobject(J, idx),
        name,
        atts,
        null_mut(),
        jsR_tofunction(J, -2),
        jsR_tofunction(J, -1),
        1,
    );
    js_pop(J, 2);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_hasproperty(
    J: *mut js_State,
    idx: c_int,
    name: *const c_char,
) -> c_int {
    jsR_hasproperty(J, js_toobject(J, idx), name)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_getindex(J: *mut js_State, idx: c_int, i: c_int) {
    jsR_getindex(J, js_toobject(J, idx), i);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_hasindex(J: *mut js_State, idx: c_int, i: c_int) -> c_int {
    jsR_hasindex(J, js_toobject(J, idx), i)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_setindex(J: *mut js_State, idx: c_int, i: c_int) {
    /* argument evaluation order, as in js_setproperty above */
    let transient = (js_isobject(J, idx) == 0) as c_int;
    jsR_setindex(J, js_toobject(J, idx), i, transient);
    js_pop(J, 1);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_delindex(J: *mut js_State, idx: c_int, i: c_int) {
    jsR_delindex(J, js_toobject(J, idx), i);
}

/* Iterator */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_pushiterator(J: *mut js_State, idx: c_int, own: c_int) {
    js_pushobject(J, jsV_newiterator(J, js_toobject(J, idx), own));
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_nextiterator(J: *mut js_State, idx: c_int) -> *const c_char {
    jsV_nextiterator(J, js_toobject(J, idx))
}

/* Environment records */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsR_newenvironment(
    J: *mut js_State,
    vars: *mut js_Object,
    outer: *mut js_Environment,
) -> *mut js_Environment {
    let E = js_malloc(J, core::mem::size_of::<js_Environment>() as c_int) as *mut js_Environment;
    (*E).gcmark = 0;
    (*E).gcnext = (*J).gcenv;
    (*J).gcenv = E;
    (*J).gccounter += 1;

    (*E).outer = outer;
    (*E).variables = vars;
    E
}

unsafe fn js_initvar(J: *mut js_State, name: *const c_char, idx: c_int) {
    jsR_defproperty(
        J,
        (*(*J).E).variables,
        name,
        JS_DONTENUM | JS_DONTCONF,
        stackidx(J, idx),
        null_mut(),
        null_mut(),
        0,
    );
}

unsafe fn js_hasvar(J: *mut js_State, name: *const c_char) -> c_int {
    let mut E = (*J).E;
    loop {
        let refp = jsV_getproperty(J, (*E).variables, name);
        if !refp.is_null() {
            if !(*refp).getter.is_null() {
                js_pushobject(J, (*refp).getter);
                js_pushobject(J, (*E).variables);
                js_call(J, 0);
            } else {
                js_pushvalue(J, (*refp).value);
            }
            return 1;
        }
        E = (*E).outer;
        if E.is_null() {
            break;
        }
    }
    0
}

unsafe fn js_setvar(J: *mut js_State, name: *const c_char) {
    let mut E = (*J).E;
    loop {
        let refp = jsV_getproperty(J, (*E).variables, name);
        if !refp.is_null() {
            if !(*refp).setter.is_null() {
                js_pushobject(J, (*refp).setter);
                js_pushobject(J, (*E).variables);
                js_copy(J, -3);
                js_call(J, 1);
                js_pop(J, 1);
                return;
            }
            if ((*refp).atts & JS_READONLY) == 0 {
                (*refp).value = *stackidx(J, -1);
            } else if (*J).strict != 0 {
                js_typeerror!(J, "'%s' is read-only", name);
            }
            return;
        }
        E = (*E).outer;
        if E.is_null() {
            break;
        }
    }
    if (*J).strict != 0 {
        js_referenceerror!(J, "assignment to undeclared variable '%s'", name);
    }
    jsR_setproperty(J, (*J).G, name, 0);
}

unsafe fn js_delvar(J: *mut js_State, name: *const c_char) -> c_int {
    let mut E = (*J).E;
    loop {
        let refp = jsV_getownproperty(J, (*E).variables, name);
        if !refp.is_null() {
            if ((*refp).atts & JS_DONTCONF) != 0 {
                if (*J).strict != 0 {
                    js_typeerror!(J, "'%s' is non-configurable", name);
                }
                return 0;
            }
            jsV_delproperty(J, (*E).variables, name);
            return 1;
        }
        E = (*E).outer;
        if E.is_null() {
            break;
        }
    }
    jsR_delproperty(J, (*J).G, name)
}

/* Function calls */

unsafe fn jsR_savescope(J: *mut js_State, newE: *mut js_Environment) {
    if (*J).envtop + 1 >= JS_ENVLIMIT as c_int {
        js_stackoverflow(J);
    }
    (*J).envstack[(*J).envtop as usize] = (*J).E;
    (*J).envtop += 1;
    (*J).E = newE;
}

unsafe fn jsR_restorescope(J: *mut js_State) {
    (*J).envtop -= 1;
    (*J).E = (*J).envstack[(*J).envtop as usize];
}

unsafe fn jsR_calllwfunction(
    J: *mut js_State,
    n: c_int,
    F: *mut js_Function,
    scope: *mut js_Environment,
) {
    let v: js_Value;
    let mut i: c_int;
    let mut n = n;

    jsR_savescope(J, scope);

    if n > (*F).numparams {
        js_pop(J, n - (*F).numparams);
        n = (*F).numparams;
    }

    i = n;
    while i < (*F).varlen {
        js_pushundefined(J);
        i += 1;
    }

    jsR_run(J, F);
    v = *stackidx(J, -1);
    BOT!(J) -= 1;
    TOP!(J) = BOT!(J); /* clear stack */
    js_pushvalue(J, v);

    jsR_restorescope(J);
}

unsafe fn jsR_callfunction(
    J: *mut js_State,
    n: c_int,
    F: *mut js_Function,
    scope: *mut js_Environment,
) {
    let v: js_Value;
    let mut i: c_int;
    let mut scope = scope;

    scope = jsR_newenvironment(J, jsV_newobject(J, JS_COBJECT, null_mut()), scope);

    jsR_savescope(J, scope);

    if (*F).arguments != 0 {
        js_newarguments(J);
        if (*J).strict == 0 {
            js_currentfunction(J);
            js_defproperty(J, -2, cs!("callee"), JS_DONTENUM);
        }
        js_pushnumber(J, n as f64);
        js_defproperty(J, -2, cs!("length"), JS_DONTENUM);
        i = 0;
        while i < n {
            js_copy(J, i + 1);
            js_setindex(J, -2, i);
            i += 1;
        }
        js_initvar(J, cs!("arguments"), -1);
        js_pop(J, 1);
    }

    i = 0;
    while i < n && i < (*F).numparams {
        js_initvar(J, *(*F).vartab.offset(i as isize), i + 1);
        i += 1;
    }
    js_pop(J, n);

    while i < (*F).varlen {
        js_pushundefined(J);
        js_initvar(J, *(*F).vartab.offset(i as isize), -1);
        js_pop(J, 1);
        i += 1;
    }

    jsR_run(J, F);
    v = *stackidx(J, -1);
    BOT!(J) -= 1;
    TOP!(J) = BOT!(J); /* clear stack */
    js_pushvalue(J, v);

    jsR_restorescope(J);
}

unsafe fn jsR_callscript(
    J: *mut js_State,
    n: c_int,
    F: *mut js_Function,
    scope: *mut js_Environment,
) {
    let v: js_Value;
    let mut i: c_int;

    if !scope.is_null() {
        jsR_savescope(J, scope);
    }

    /* scripts take no arguments */
    js_pop(J, n);

    i = 0;
    while i < (*F).varlen {
        /* Bug 701886: don't redefine existing vars in eval/scripts */
        if js_hasvar(J, *(*F).vartab.offset(i as isize)) == 0 {
            js_pushundefined(J);
            js_initvar(J, *(*F).vartab.offset(i as isize), -1);
            js_pop(J, 1);
        }
        i += 1;
    }

    jsR_run(J, F);
    v = *stackidx(J, -1);
    BOT!(J) -= 1;
    TOP!(J) = BOT!(J); /* clear stack */
    js_pushvalue(J, v);

    if !scope.is_null() {
        jsR_restorescope(J);
    }
}

unsafe fn jsR_callcfunction(J: *mut js_State, n: c_int, min: c_int, F: js_CFunction) {
    let save_top: c_int;
    let mut i: c_int;
    let v: js_Value;

    i = n;
    while i < min {
        js_pushundefined(J);
        i += 1;
    }

    save_top = TOP!(J);
    (F.unwrap())(J);
    if TOP!(J) > save_top {
        v = *stackidx(J, -1);
        BOT!(J) -= 1;
        TOP!(J) = BOT!(J); /* clear stack */
        js_pushvalue(J, v);
    } else {
        BOT!(J) -= 1;
        TOP!(J) = BOT!(J); /* clear stack */
        js_pushundefined(J);
    }
}

unsafe fn jsR_pushtrace(
    J: *mut js_State,
    name: *const c_char,
    file: *const c_char,
    line: c_int,
) {
    if (*J).tracetop + 1 == JS_ENVLIMIT as c_int {
        js_error!(J, "call stack overflow");
    }
    (*J).tracetop += 1;
    let t = (*J).tracetop as usize;
    (*J).trace[t].stack = (*J).bot;
    (*J).trace[t].name = name;
    (*J).trace[t].file = file;
    (*J).trace[t].line = line;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_call(J: *mut js_State, n: c_int) {
    let obj: *mut js_Object;
    let savebot: c_int;

    if n < 0 {
        js_rangeerror!(J, "number of arguments cannot be negative");
    }

    if js_iscallable(J, -n - 2) == 0 {
        js_typeerror!(J, "%s is not callable", js_typeof(J, -n - 2));
    }

    obj = js_toobject(J, -n - 2);

    savebot = BOT!(J);
    BOT!(J) = TOP!(J) - n - 1;

    if (*obj).type_ == JS_CFUNCTION {
        jsR_pushtrace(
            J,
            (*(*obj).u.f.function).name,
            (*(*obj).u.f.function).filename,
            (*(*obj).u.f.function).line,
        );
        if (*(*obj).u.f.function).lightweight != 0 {
            jsR_calllwfunction(J, n, (*obj).u.f.function, (*obj).u.f.scope);
        } else {
            jsR_callfunction(J, n, (*obj).u.f.function, (*obj).u.f.scope);
        }
        (*J).tracetop -= 1;
    } else if (*obj).type_ == JS_CSCRIPT {
        jsR_pushtrace(
            J,
            (*(*obj).u.f.function).name,
            (*(*obj).u.f.function).filename,
            (*(*obj).u.f.function).line,
        );
        jsR_callscript(J, n, (*obj).u.f.function, (*obj).u.f.scope);
        (*J).tracetop -= 1;
    } else if (*obj).type_ == JS_CCFUNCTION {
        jsR_pushtrace(J, (*obj).u.c.name, cs!("native"), 0);
        jsR_callcfunction(J, n, (*obj).u.c.length, (*obj).u.c.function);
        (*J).tracetop -= 1;
    }

    BOT!(J) = savebot;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_construct(J: *mut js_State, n: c_int) {
    let obj: *mut js_Object;
    let prototype: *mut js_Object;
    let newobj: *mut js_Object;

    if js_iscallable(J, -n - 1) == 0 {
        js_typeerror!(J, "%s is not callable", js_typeof(J, -n - 1));
    }

    obj = js_toobject(J, -n - 1);

    /* built-in constructors create their own objects, give them a 'null' this */
    if (*obj).type_ == JS_CCFUNCTION && (*obj).u.c.constructor.is_some() {
        let savebot = BOT!(J);
        js_pushnull(J);
        if n > 0 {
            js_rot(J, n + 1);
        }
        BOT!(J) = TOP!(J) - n - 1;

        jsR_pushtrace(J, (*obj).u.c.name, cs!("native"), 0);
        jsR_callcfunction(J, n, (*obj).u.c.length, (*obj).u.c.constructor);
        (*J).tracetop -= 1;

        BOT!(J) = savebot;
        return;
    }

    /* extract the function object's prototype property */
    js_getproperty(J, -n - 1, cs!("prototype"));
    if js_isobject(J, -1) != 0 {
        prototype = js_toobject(J, -1);
    } else {
        prototype = (*J).Object_prototype;
    }
    js_pop(J, 1);

    /* create a new object with above prototype, and shift it into the 'this' slot */
    newobj = jsV_newobject(J, JS_COBJECT, prototype);
    js_pushobject(J, newobj);
    if n > 0 {
        js_rot(J, n + 1);
    }

    /* and save a copy to return */
    js_pushobject(J, newobj);
    js_rot(J, n + 3);

    /* call the function */
    js_call(J, n);

    /* if result is not an object, return the original object we created */
    if js_isobject(J, -1) == 0 {
        js_pop(J, 1);
    } else {
        js_rot2pop1(J);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_eval(J: *mut js_State) {
    if js_isstring(J, -1) == 0 {
        return;
    }
    js_loadeval(J, cs!("(eval)"), js_tostring(J, -1));
    js_rot2pop1(J);
    js_copy(J, 0); /* copy 'this' */
    js_call(J, 0);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_pconstruct(J: *mut js_State, n: c_int) -> c_int {
    let mut savetop = TOP!(J) - n - 2;
    if js_try!(J) != 0 {
        /* clean up the stack to only hold the error object */
        let savetop = vol!(savetop);
        ST!(J, savetop) = ST!(J, TOP!(J) - 1);
        TOP!(J) = savetop + 1;
        return 1;
    }
    js_construct(J, n);
    js_endtry(J);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_pcall(J: *mut js_State, n: c_int) -> c_int {
    let mut savetop = TOP!(J) - n - 2;
    if js_try!(J) != 0 {
        /* clean up the stack to only hold the error object */
        let savetop = vol!(savetop);
        ST!(J, savetop) = ST!(J, TOP!(J) - 1);
        TOP!(J) = savetop + 1;
        return 1;
    }
    js_call(J, n);
    js_endtry(J);
    0
}

/* Exceptions */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_savetrypc(J: *mut js_State, pc: *mut js_Instruction) -> *mut c_void {
    if (*J).trytop == JS_TRYLIMIT {
        js_trystackoverflow(J);
    }
    let t = (*J).trytop as usize;
    (*J).trybuf[t].E = (*J).E;
    (*J).trybuf[t].envtop = (*J).envtop;
    (*J).trybuf[t].tracetop = (*J).tracetop;
    (*J).trybuf[t].top = (*J).top;
    (*J).trybuf[t].bot = (*J).bot;
    (*J).trybuf[t].strict = (*J).strict;
    (*J).trybuf[t].pc = pc;
    (*J).trytop += 1;
    addr_of_mut!((*J).trybuf[t].buf) as *mut c_void
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_savetry(J: *mut js_State) -> *mut c_void {
    if (*J).trytop == JS_TRYLIMIT {
        js_trystackoverflow(J);
    }
    let t = (*J).trytop as usize;
    (*J).trybuf[t].E = (*J).E;
    (*J).trybuf[t].envtop = (*J).envtop;
    (*J).trybuf[t].tracetop = (*J).tracetop;
    (*J).trybuf[t].top = (*J).top;
    (*J).trybuf[t].bot = (*J).bot;
    (*J).trybuf[t].strict = (*J).strict;
    (*J).trybuf[t].pc = null_mut();
    (*J).trytop += 1;
    addr_of_mut!((*J).trybuf[t].buf) as *mut c_void
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_endtry(J: *mut js_State) {
    if (*J).trytop == 0 {
        js_error!(J, "endtry: exception stack underflow");
    }
    (*J).trytop -= 1;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_throw(J: *mut js_State) -> ! {
    if (*J).trytop > 0 {
        let v = *stackidx(J, -1);
        (*J).trytop -= 1;
        let t = (*J).trytop as usize;
        (*J).E = (*J).trybuf[t].E;
        (*J).envtop = (*J).trybuf[t].envtop;
        (*J).tracetop = (*J).trybuf[t].tracetop;
        (*J).top = (*J).trybuf[t].top;
        (*J).bot = (*J).trybuf[t].bot;
        (*J).strict = (*J).trybuf[t].strict;
        js_pushvalue(J, v);
        longjmp(addr_of_mut!((*J).trybuf[t].buf) as *mut c_void, 1);
    }
    if let Some(panic) = (*J).panic {
        panic(J);
    }
    abort()
}

/* Main interpreter loop */

unsafe fn js_dumpvalue(J: *mut js_State, v: js_Value) {
    match vtype(&v) {
        JS_TUNDEFINED => {
            printf(cs!("undefined"));
        }
        JS_TNULL => {
            printf(cs!("null"));
        }
        JS_TBOOLEAN => {
            printf(if v.u.boolean != 0 {
                cs!("true")
            } else {
                cs!("false")
            });
        }
        JS_TNUMBER => {
            printf(cs!("%.9g"), v.u.number);
        }
        JS_TSHRSTR => {
            printf(cs!("'%s'"), addr_of!(v.u.shrstr) as *const c_char);
        }
        JS_TLITSTR => {
            printf(cs!("'%s'"), v.u.litstr);
        }
        JS_TMEMSTR => {
            printf(cs!("'%s'"), strp(v.u.memstr));
        }
        JS_TOBJECT => {
            if v.u.object == (*J).G {
                printf(cs!("[Global]"));
                return;
            }
            match (*v.u.object).type_ {
                JS_COBJECT => {
                    printf(cs!("[Object %p]"), v.u.object as *mut c_void);
                }
                JS_CARRAY => {
                    printf(cs!("[Array %p]"), v.u.object as *mut c_void);
                }
                JS_CFUNCTION => {
                    printf(
                        cs!("[Function %p, %s, %s:%d]"),
                        v.u.object as *mut c_void,
                        (*(*v.u.object).u.f.function).name,
                        (*(*v.u.object).u.f.function).filename,
                        (*(*v.u.object).u.f.function).line,
                    );
                }
                JS_CSCRIPT => {
                    printf(
                        cs!("[Script %s]"),
                        (*(*v.u.object).u.f.function).filename,
                    );
                }
                JS_CCFUNCTION => {
                    printf(cs!("[CFunction %s]"), (*v.u.object).u.c.name);
                }
                JS_CBOOLEAN => {
                    printf(cs!("[Boolean %d]"), (*v.u.object).u.boolean);
                }
                JS_CNUMBER => {
                    printf(cs!("[Number %g]"), (*v.u.object).u.number);
                }
                JS_CSTRING => {
                    printf(cs!("[String'%s']"), (*v.u.object).u.s.string);
                }
                JS_CERROR => {
                    printf(cs!("[Error]"));
                }
                JS_CARGUMENTS => {
                    printf(cs!("[Arguments %p]"), v.u.object as *mut c_void);
                }
                JS_CITERATOR => {
                    printf(cs!("[Iterator %p]"), v.u.object as *mut c_void);
                }
                JS_CUSERDATA => {
                    printf(
                        cs!("[Userdata %s %p]"),
                        (*v.u.object).u.user.tag,
                        (*v.u.object).u.user.data,
                    );
                }
                _ => {
                    printf(cs!("[Object %p]"), v.u.object as *mut c_void);
                }
            }
        }
        _ => {}
    }
}

unsafe fn js_stacktrace(J: *mut js_State) {
    let mut n: c_int;
    printf(cs!("stack trace:\n"));
    n = (*J).tracetop;
    while n >= 0 {
        let name = (*J).trace[n as usize].name;
        let file = (*J).trace[n as usize].file;
        let line = (*J).trace[n as usize].line;
        if line > 0 {
            if *name != 0 {
                printf(cs!("\tat %s (%s:%d)\n"), name, file, line);
            } else {
                printf(cs!("\tat %s:%d\n"), file, line);
            }
        } else {
            printf(cs!("\tat %s (%s)\n"), name, file);
        }
        n -= 1;
    }
}

unsafe fn js_dumpstack(J: *mut js_State) {
    let mut i: c_int;
    printf(cs!("stack {\n"));
    i = 0;
    while i < TOP!(J) {
        putchar(if i == BOT!(J) {
            '>' as c_int
        } else {
            ' ' as c_int
        });
        printf(cs!("%4d: "), i);
        js_dumpvalue(J, ST!(J, i));
        putchar('\n' as c_int);
        i += 1;
    }
    printf(cs!("}\n"));
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_trap(J: *mut js_State, pc: c_int) {
    js_dumpstack(J);
    js_stacktrace(J);
}

unsafe fn jsR_isindex(J: *mut js_State, idx: c_int, k: *mut c_int) -> c_int {
    let v = stackidx(J, idx);
    if vtype(v) == JS_TNUMBER {
        *k = (*v).u.number as c_int;
        return (*k as f64 == (*v).u.number && *k >= 0) as c_int;
    }
    0
}

unsafe fn jsR_run(J: *mut js_State, F: *mut js_Function) {
    let FT: *mut *mut js_Function = (*F).funtab;
    let VT: *mut *const c_char = if !(*F).vartab.is_null() {
        (*F).vartab.offset(-1)
    } else {
        null_mut()
    };
    let lightweight = (*F).lightweight;
    let pcstart: *mut js_Instruction = (*F).code;
    let mut pc: *mut js_Instruction = (*F).code;
    let mut opcode: c_int;
    let mut offset: c_int;
    let savestrict: c_int;

    let mut str: *const c_char = null();
    let mut obj: *mut js_Object;
    let mut x: f64;
    let mut y: f64;
    let mut ux: c_uint;
    let mut uy: c_uint;
    let mut ix: c_int = 0;
    let mut iy: c_int;
    let mut okay: c_int = 0;
    let mut b: c_int;
    let mut transient: c_int;

    savestrict = (*J).strict;
    (*J).strict = (*F).strict;

    macro_rules! READSTRING {
        () => {
            memcpy(
                addr_of_mut!(str) as *mut c_void,
                pc as *const c_void,
                core::mem::size_of::<*const c_char>(),
            );
            pc = pc.add(core::mem::size_of::<*const c_char>() / core::mem::size_of::<js_Instruction>());
        };
    }
    macro_rules! NEXT {
        () => {{
            let v = *pc;
            pc = pc.add(1);
            v as c_int
        }};
    }

    loop {
        if (*J).runlimit > 0 {
            if (*J).runlimit == 1 {
                js_runlimit(J);
            }
            (*J).runlimit -= 1;
        }

        if (*J).gccounter > (*J).gcthresh {
            js_gc(J, 0);
        }

        (*J).trace[(*J).tracetop as usize].line = NEXT!();

        opcode = NEXT!();

        match opcode {
            OP_POP => js_pop(J, 1),
            OP_DUP => js_dup(J),
            OP_DUP2 => js_dup2(J),
            OP_ROT2 => js_rot2(J),
            OP_ROT3 => js_rot3(J),
            OP_ROT4 => js_rot4(J),

            OP_INTEGER => {
                js_pushnumber(J, (NEXT!() - 32768) as f64);
            }

            OP_NUMBER => {
                let mut xx: f64 = 0.0;
                memcpy(
                    addr_of_mut!(xx) as *mut c_void,
                    pc as *const c_void,
                    core::mem::size_of::<f64>(),
                );
                pc = pc.add(core::mem::size_of::<f64>() / core::mem::size_of::<js_Instruction>());
                js_pushnumber(J, xx);
            }

            OP_STRING => {
                READSTRING!();
                js_pushliteral(J, str);
            }

            OP_CLOSURE => {
                let i = NEXT!();
                js_newfunction(J, *FT.offset(i as isize), (*J).E);
            }
            OP_NEWOBJECT => js_newobject(J),
            OP_NEWARRAY => js_newarray(J),
            OP_NEWREGEXP => {
                READSTRING!();
                let f = NEXT!();
                js_newregexp(J, str, f);
            }

            OP_UNDEF => js_pushundefined(J),
            OP_NULL => js_pushnull(J),
            OP_TRUE => js_pushboolean(J, 1),
            OP_FALSE => js_pushboolean(J, 0),

            OP_THIS => {
                if (*J).strict != 0 {
                    js_copy(J, 0);
                } else {
                    if js_iscoercible(J, 0) != 0 {
                        js_copy(J, 0);
                    } else {
                        js_pushglobal(J);
                    }
                }
            }

            OP_CURRENT => {
                js_currentfunction(J);
            }

            OP_GETLOCAL => {
                if lightweight != 0 {
                    CHECKSTACK!(J, 1);
                    let i = NEXT!();
                    ST!(J, TOP!(J)) = ST!(J, BOT!(J) + i);
                    TOP!(J) += 1;
                } else {
                    let i = NEXT!();
                    str = *VT.offset(i as isize);
                    if js_hasvar(J, str) == 0 {
                        js_referenceerror!(J, "'%s' is not defined", str);
                    }
                }
            }

            OP_SETLOCAL => {
                if lightweight != 0 {
                    let i = NEXT!();
                    ST!(J, BOT!(J) + i) = ST!(J, TOP!(J) - 1);
                } else {
                    let i = NEXT!();
                    js_setvar(J, *VT.offset(i as isize));
                }
            }

            OP_DELLOCAL => {
                if lightweight != 0 {
                    pc = pc.add(1);
                    js_pushboolean(J, 0);
                } else {
                    let i = NEXT!();
                    b = js_delvar(J, *VT.offset(i as isize));
                    js_pushboolean(J, b);
                }
            }

            OP_GETVAR => {
                READSTRING!();
                if js_hasvar(J, str) == 0 {
                    js_referenceerror!(J, "'%s' is not defined", str);
                }
            }

            OP_HASVAR => {
                READSTRING!();
                if js_hasvar(J, str) == 0 {
                    js_pushundefined(J);
                }
            }

            OP_SETVAR => {
                READSTRING!();
                js_setvar(J, str);
            }

            OP_DELVAR => {
                READSTRING!();
                b = js_delvar(J, str);
                js_pushboolean(J, b);
            }

            OP_IN => {
                str = js_tostring(J, -2);
                if js_isobject(J, -1) == 0 {
                    js_typeerror!(J, "operand to 'in' is not an object");
                }
                b = js_hasproperty(J, -1, str);
                js_pop(J, 2 + b);
                js_pushboolean(J, b);
            }

            OP_SKIPARRAY => {
                js_setlength(J, -1, js_getlength(J, -1) + 1);
            }
            OP_INITARRAY => {
                js_setindex(J, -2, js_getlength(J, -2));
            }

            OP_INITPROP => {
                obj = js_toobject(J, -3);
                str = js_tostring(J, -2);
                jsR_setproperty(J, obj, str, 0);
                js_pop(J, 2);
            }

            OP_INITGETTER => {
                obj = js_toobject(J, -3);
                str = js_tostring(J, -2);
                jsR_defproperty(
                    J,
                    obj,
                    str,
                    0,
                    null_mut(),
                    jsR_tofunction(J, -1),
                    null_mut(),
                    0,
                );
                js_pop(J, 2);
            }

            OP_INITSETTER => {
                obj = js_toobject(J, -3);
                str = js_tostring(J, -2);
                jsR_defproperty(
                    J,
                    obj,
                    str,
                    0,
                    null_mut(),
                    null_mut(),
                    jsR_tofunction(J, -1),
                    0,
                );
                js_pop(J, 2);
            }

            OP_GETPROP => {
                if jsR_isindex(J, -1, &mut ix) != 0 {
                    obj = js_toobject(J, -2);
                    jsR_getindex(J, obj, ix);
                } else {
                    str = js_tostring(J, -1);
                    obj = js_toobject(J, -2);
                    jsR_getproperty(J, obj, str);
                }
                js_rot3pop2(J);
            }

            OP_GETPROP_S => {
                READSTRING!();
                obj = js_toobject(J, -1);
                jsR_getproperty(J, obj, str);
                js_rot2pop1(J);
            }

            OP_SETPROP => {
                if jsR_isindex(J, -2, &mut ix) != 0 {
                    obj = js_toobject(J, -3);
                    transient = (js_isobject(J, -3) == 0) as c_int;
                    jsR_setindex(J, obj, ix, transient);
                } else {
                    str = js_tostring(J, -2);
                    obj = js_toobject(J, -3);
                    transient = (js_isobject(J, -3) == 0) as c_int;
                    jsR_setproperty(J, obj, str, transient);
                }
                js_rot3pop2(J);
            }

            OP_SETPROP_S => {
                READSTRING!();
                obj = js_toobject(J, -2);
                transient = (js_isobject(J, -2) == 0) as c_int;
                jsR_setproperty(J, obj, str, transient);
                js_rot2pop1(J);
            }

            OP_DELPROP => {
                str = js_tostring(J, -1);
                obj = js_toobject(J, -2);
                b = jsR_delproperty(J, obj, str);
                js_pop(J, 2);
                js_pushboolean(J, b);
            }

            OP_DELPROP_S => {
                READSTRING!();
                obj = js_toobject(J, -1);
                b = jsR_delproperty(J, obj, str);
                js_pop(J, 1);
                js_pushboolean(J, b);
            }

            OP_ITERATOR => {
                if js_iscoercible(J, -1) != 0 {
                    obj = jsV_newiterator(J, js_toobject(J, -1), 0);
                    js_pop(J, 1);
                    js_pushobject(J, obj);
                }
            }

            OP_NEXTITER => {
                if js_isobject(J, -1) != 0 {
                    obj = js_toobject(J, -1);
                    str = jsV_nextiterator(J, obj);
                    if !str.is_null() {
                        js_pushstring(J, str);
                        js_pushboolean(J, 1);
                    } else {
                        js_pop(J, 1);
                        js_pushboolean(J, 0);
                    }
                } else {
                    js_pop(J, 1);
                    js_pushboolean(J, 0);
                }
            }

            /* Function calls */
            OP_EVAL => {
                js_eval(J);
            }

            OP_CALL => {
                let n = NEXT!();
                js_call(J, n);
            }

            OP_NEW => {
                let n = NEXT!();
                js_construct(J, n);
            }

            /* Unary operators */
            OP_TYPEOF => {
                str = js_typeof(J, -1);
                js_pop(J, 1);
                js_pushliteral(J, str);
            }

            OP_POS => {
                x = js_tonumber(J, -1);
                js_pop(J, 1);
                js_pushnumber(J, x);
            }

            OP_NEG => {
                x = js_tonumber(J, -1);
                js_pop(J, 1);
                js_pushnumber(J, -x);
            }

            OP_BITNOT => {
                ix = js_toint32(J, -1);
                js_pop(J, 1);
                js_pushnumber(J, (!ix) as f64);
            }

            OP_LOGNOT => {
                b = js_toboolean(J, -1);
                js_pop(J, 1);
                js_pushboolean(J, (b == 0) as c_int);
            }

            OP_INC => {
                x = js_tonumber(J, -1);
                js_pop(J, 1);
                js_pushnumber(J, x + 1.0);
            }

            OP_DEC => {
                x = js_tonumber(J, -1);
                js_pop(J, 1);
                js_pushnumber(J, x - 1.0);
            }

            OP_POSTINC => {
                x = js_tonumber(J, -1);
                js_pop(J, 1);
                js_pushnumber(J, x + 1.0);
                js_pushnumber(J, x);
            }

            OP_POSTDEC => {
                x = js_tonumber(J, -1);
                js_pop(J, 1);
                js_pushnumber(J, x - 1.0);
                js_pushnumber(J, x);
            }

            /* Multiplicative operators */
            OP_MUL => {
                x = js_tonumber(J, -2);
                y = js_tonumber(J, -1);
                js_pop(J, 2);
                js_pushnumber(J, x * y);
            }

            OP_DIV => {
                x = js_tonumber(J, -2);
                y = js_tonumber(J, -1);
                js_pop(J, 2);
                js_pushnumber(J, x / y);
            }

            OP_MOD => {
                x = js_tonumber(J, -2);
                y = js_tonumber(J, -1);
                js_pop(J, 2);
                js_pushnumber(J, fmod(x, y));
            }

            /* Additive operators */
            OP_ADD => {
                js_concat(J);
            }

            OP_SUB => {
                x = js_tonumber(J, -2);
                y = js_tonumber(J, -1);
                js_pop(J, 2);
                js_pushnumber(J, x - y);
            }

            /* Shift operators */
            OP_SHL => {
                ix = js_toint32(J, -2);
                uy = js_touint32(J, -1);
                js_pop(J, 2);
                js_pushnumber(J, ix.wrapping_shl(uy & 0x1F) as f64);
            }

            OP_SHR => {
                ix = js_toint32(J, -2);
                uy = js_touint32(J, -1);
                js_pop(J, 2);
                js_pushnumber(J, (ix >> (uy & 0x1F)) as f64);
            }

            OP_USHR => {
                ux = js_touint32(J, -2);
                uy = js_touint32(J, -1);
                js_pop(J, 2);
                js_pushnumber(J, (ux >> (uy & 0x1F)) as f64);
            }

            /* Relational operators */
            OP_LT => {
                b = js_compare(J, &mut okay);
                js_pop(J, 2);
                js_pushboolean(J, (okay != 0 && b < 0) as c_int);
            }
            OP_GT => {
                b = js_compare(J, &mut okay);
                js_pop(J, 2);
                js_pushboolean(J, (okay != 0 && b > 0) as c_int);
            }
            OP_LE => {
                b = js_compare(J, &mut okay);
                js_pop(J, 2);
                js_pushboolean(J, (okay != 0 && b <= 0) as c_int);
            }
            OP_GE => {
                b = js_compare(J, &mut okay);
                js_pop(J, 2);
                js_pushboolean(J, (okay != 0 && b >= 0) as c_int);
            }

            OP_INSTANCEOF => {
                b = js_instanceof(J);
                js_pop(J, 2);
                js_pushboolean(J, b);
            }

            /* Equality */
            OP_EQ => {
                b = js_equal(J);
                js_pop(J, 2);
                js_pushboolean(J, b);
            }
            OP_NE => {
                b = js_equal(J);
                js_pop(J, 2);
                js_pushboolean(J, (b == 0) as c_int);
            }
            OP_STRICTEQ => {
                b = js_strictequal(J);
                js_pop(J, 2);
                js_pushboolean(J, b);
            }
            OP_STRICTNE => {
                b = js_strictequal(J);
                js_pop(J, 2);
                js_pushboolean(J, (b == 0) as c_int);
            }

            OP_JCASE => {
                offset = NEXT!();
                b = js_strictequal(J);
                if b != 0 {
                    js_pop(J, 2);
                    pc = pcstart.offset(offset as isize);
                } else {
                    js_pop(J, 1);
                }
            }

            /* Binary bitwise operators */
            OP_BITAND => {
                ix = js_toint32(J, -2);
                iy = js_toint32(J, -1);
                js_pop(J, 2);
                js_pushnumber(J, (ix & iy) as f64);
            }

            OP_BITXOR => {
                ix = js_toint32(J, -2);
                iy = js_toint32(J, -1);
                js_pop(J, 2);
                js_pushnumber(J, (ix ^ iy) as f64);
            }

            OP_BITOR => {
                ix = js_toint32(J, -2);
                iy = js_toint32(J, -1);
                js_pop(J, 2);
                js_pushnumber(J, (ix | iy) as f64);
            }

            /* Try and Catch */
            OP_THROW => {
                js_throw(J);
            }

            OP_TRY => {
                offset = NEXT!();
                if js_trypc!(J, pc) != 0 {
                    pc = (*J).trybuf[(*J).trytop as usize].pc;
                } else {
                    pc = pcstart.offset(offset as isize);
                }
            }

            OP_ENDTRY => {
                js_endtry(J);
            }

            OP_CATCH => {
                READSTRING!();
                obj = jsV_newobject(J, JS_COBJECT, null_mut());
                js_pushobject(J, obj);
                js_rot2(J);
                js_setproperty(J, -2, str);
                (*J).E = jsR_newenvironment(J, obj, (*J).E);
                js_pop(J, 1);
            }

            OP_ENDCATCH => {
                (*J).E = (*(*J).E).outer;
            }

            /* With */
            OP_WITH => {
                obj = js_toobject(J, -1);
                (*J).E = jsR_newenvironment(J, obj, (*J).E);
                js_pop(J, 1);
            }

            OP_ENDWITH => {
                (*J).E = (*(*J).E).outer;
            }

            /* Branching */
            OP_DEBUGGER => {
                js_trap(J, (pc.offset_from(pcstart) as c_int) - 1);
            }

            OP_JUMP => {
                pc = pcstart.offset(*pc as isize);
            }

            OP_JTRUE => {
                offset = NEXT!();
                b = js_toboolean(J, -1);
                js_pop(J, 1);
                if b != 0 {
                    pc = pcstart.offset(offset as isize);
                }
            }

            OP_JFALSE => {
                offset = NEXT!();
                b = js_toboolean(J, -1);
                js_pop(J, 1);
                if b == 0 {
                    pc = pcstart.offset(offset as isize);
                }
            }

            OP_RETURN => {
                (*J).strict = savestrict;
                return;
            }

            _ => {}
        }
    }
}
