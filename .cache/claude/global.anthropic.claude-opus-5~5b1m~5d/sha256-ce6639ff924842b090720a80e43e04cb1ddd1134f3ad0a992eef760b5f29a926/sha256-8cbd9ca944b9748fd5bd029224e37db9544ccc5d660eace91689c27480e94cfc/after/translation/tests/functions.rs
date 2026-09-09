//! CONFIGS.md rows 46-50, 59-63, 79-84.
//!
//! Function objects (C functions, C constructors, userdata), the script entry
//! points (`js_ploadstring`/`js_pcall`, `js_loadstring`/`js_eval`,
//! `js_loadeval`, `js_call`/`js_construct`), the exception machinery
//! (`js_savetry`/`js_savetrypc`/`js_endtry`/`js_throw`), the debug dump
//! (`js_trap`), the builtin initialisers (`jsB_init*`, `jsB_prop*`) and the
//! low level function/environment constructors (`js_newarguments`,
//! `js_newfunction`, `js_newscript`, `jsR_newenvironment`).
//!
//! All C callbacks reach the API through the `CUR` raw pointer that every
//! `diff` closure installs as its first statement (the closure and the
//! callbacks run in the same forked, single threaded child).

#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::{c_char, c_int, c_void, CStr};
use std::ptr::{addr_of_mut, null, null_mut};

/* ------------------------------------------------------------------------- */
/* plumbing                                                                  */
/* ------------------------------------------------------------------------- */

/// `'static` NUL terminated C string from a literal.  Needed because several
/// mujs entry points (`js_newcfunction` name, userdata `tag`) store the pointer
/// without copying it, so it has to outlive the `js_State`.
macro_rules! cstr {
    ($l:literal) => {
        concat!($l, "\0").as_ptr() as *const c_char
    };
}

static mut CUR: *const Api = null();

unsafe fn api() -> &'static Api {
    &*CUR
}

extern "C" {
    static mut stdout: *mut c_void;
    fn setvbuf(f: *mut c_void, buf: *mut c_char, mode: c_int, size: usize) -> c_int;
}

/// The harness redirects the child's stdout to a file, which makes it block
/// buffered: a library that aborts (uncaught exception -> js_defaultpanic ->
/// abort) would throw away everything printed before the abort, and stderr
/// (always unbuffered) would float to the front of the file.  Turning stdout
/// off buffering keeps the two streams in true chronological order and keeps
/// the output of the tests that deliberately end in abort().
unsafe fn unbuffer() {
    setvbuf(stdout, null_mut(), 2 /* _IONBF */, 0);
}

/// First statement of every `diff` closure.
unsafe fn enter(api: &Api) {
    CUR = api as *const Api;
    unbuffer();
}

/* Every conversion helper in mujs writes the coerced value back into the stack
 * slot it was given (`jsV_toprimitive` takes a `js_Value*`, `js_torepr` calls
 * `js_replace`).  So a "read only" dump has to work on a copy of the value,
 * otherwise it destroys what the test is about to use next. */

unsafe fn p_num_at(a: &Api, J: JS, tag: &str, idx: c_int) {
    (a.js_copy)(J, idx);
    p_num(tag, (a.js_trynumber)(J, -1, -98765.0));
    (a.js_pop)(J, 1);
}

unsafe fn p_str_at(a: &Api, J: JS, tag: &str, idx: c_int) {
    (a.js_copy)(J, idx);
    p_str(tag, (a.js_trystring)(J, -1, cstr!("<throw>")));
    (a.js_pop)(J, 1);
}

unsafe fn p_repr_at(a: &Api, J: JS, tag: &str, idx: c_int) {
    (a.js_copy)(J, idx);
    p_str(tag, (a.js_tryrepr)(J, -1, cstr!("<throw>")));
    (a.js_pop)(J, 1);
}

/// Everything we can print about the value at `idx` without ever printing an
/// address, without letting a conversion escape as an exception, and without
/// modifying the stack.
unsafe fn show_at(a: &Api, J: JS, tag: &str, idx: c_int) {
    p_str(&format!("{}.typeof", tag), (a.js_typeof)(J, idx));
    p_int(&format!("{}.type", tag), (a.js_type)(J, idx));
    p_int(&format!("{}.iscallable", tag), (a.js_iscallable)(J, idx));
    p_int(&format!("{}.isobject", tag), (a.js_isobject)(J, idx));
    p_int(&format!("{}.iserror", tag), (a.js_iserror)(J, idx));
    p_num_at(a, J, &format!("{}.num", tag), idx);
    p_str_at(a, J, &format!("{}.str", tag), idx);
    p_repr_at(a, J, &format!("{}.repr", tag), idx);
}

unsafe fn show(a: &Api, J: JS, tag: &str) {
    show_at(a, J, tag, -1);
}

/// `js_ploadstring` + `js_pcall`, printing both return codes and the result.
unsafe fn run(a: &Api, J: JS, tag: &str, src: &str) {
    let file = cs(tag);
    let s = cs(src);
    let top0 = (a.js_gettop)(J);
    let rc = (a.js_ploadstring)(J, file.as_ptr(), s.as_ptr());
    p_int(&format!("{}.load", tag), rc);
    if rc != 0 {
        p_str(&format!("{}.loaderr", tag), (a.js_trystring)(J, -1, cstr!("<throw>")));
        (a.js_pop)(J, 1);
        p_int(&format!("{}.dtop", tag), (a.js_gettop)(J) - top0);
        return;
    }
    p_str(&format!("{}.script.typeof", tag), (a.js_typeof)(J, -1));
    p_int(&format!("{}.script.iscallable", tag), (a.js_iscallable)(J, -1));
    (a.js_pushundefined)(J);
    let rc2 = (a.js_pcall)(J, 0);
    p_int(&format!("{}.call", tag), rc2);
    show(a, J, &format!("{}.res", tag));
    (a.js_pop)(J, 1);
    p_int(&format!("{}.dtop", tag), (a.js_gettop)(J) - top0);
}

/// Enumerate the object at the top of the stack (leaves the stack unchanged).
unsafe fn dump_props(a: &Api, J: JS, tag: &str, own: c_int) {
    (a.js_pushiterator)(J, -1, own);
    let mut n = 0;
    loop {
        let name = (a.js_nextiterator)(J, -1);
        if name.is_null() {
            break;
        }
        p_str(&format!("{}.own{}.k{}", tag, own, n), name);
        n += 1;
        if n > 400 {
            p_line("TOO MANY");
            break;
        }
    }
    p_int(&format!("{}.own{}.count", tag, own), n);
    (a.js_pop)(J, 1);
}


/* ------------------------------------------------------------------------- */
/* row 46 — js_newcfunction                                                  */
/* ------------------------------------------------------------------------- */

unsafe extern "C" fn cf46(J: JS) {
    let a = api();
    let top = (a.js_gettop)(J);
    p_int("cf46.top", top);
    p_str("cf46.this.typeof", (a.js_typeof)(J, 0));
    p_str("cf46.this.str", (a.js_trystring)(J, 0, cstr!("<throw>")));
    let mut sum = 0.0f64;
    let mut i: c_int = 1;
    while i < top {
        p_str(&format!("cf46.a{}.typeof", i), (a.js_typeof)(J, i));
        p_str(&format!("cf46.a{}.str", i), (a.js_tostring)(J, i));
        let n = (a.js_tonumber)(J, i);
        p_num(&format!("cf46.a{}.num", i), n);
        if n == n {
            sum += n;
        }
        i += 1;
    }
    p_num("cf46.sum", sum);
    (a.js_pushnumber)(J, sum * 10.0 + top as f64);
}

#[test]
fn cfg46_newcfunction() {
    diff("cfg46_newcfunction", |api| unsafe {
        enter(api);
        let J = newstate(api, 0);

        /* declared length 2 */
        (api.js_newcfunction)(J, Some(cf46), cstr!("f46"), 2);
        show(api, J, "f46");
        (api.js_getproperty)(J, -1, cstr!("length"));
        p_num("f46.length", (api.js_tonumber)(J, -1));
        (api.js_pop)(J, 1);
        (api.js_setglobal)(J, cstr!("f46"));

        /* n = 0 (padded to the declared length with undefined) */
        p_line("-- n=0 --");
        (api.js_getglobal)(J, cstr!("f46"));
        (api.js_pushundefined)(J);
        (api.js_call)(J, 0);
        show(api, J, "r0");
        (api.js_pop)(J, 1);

        /* n = 1 */
        p_line("-- n=1 --");
        (api.js_getglobal)(J, cstr!("f46"));
        (api.js_pushundefined)(J);
        (api.js_pushnumber)(J, 5.5);
        (api.js_call)(J, 1);
        show(api, J, "r1");
        (api.js_pop)(J, 1);

        /* n = 5, i.e. more args than the declared length */
        p_line("-- n=5 --");
        (api.js_getglobal)(J, cstr!("f46"));
        (api.js_pushundefined)(J);
        (api.js_pushnumber)(J, 1.0);
        (api.js_pushstring)(J, cstr!("two"));
        (api.js_pushboolean)(J, 1);
        (api.js_pushnull)(J);
        (api.js_newarray)(J);
        (api.js_call)(J, 5);
        show(api, J, "r5");
        (api.js_pop)(J, 1);

        /* as a method: getglobal(obj) + getproperty(m) + rot2 so `this` is real */
        p_line("-- method --");
        (api.js_newobject)(J);
        (api.js_getglobal)(J, cstr!("f46"));
        (api.js_defproperty)(J, -2, cstr!("m"), 0);
        (api.js_pushnumber)(J, 42.0);
        (api.js_defproperty)(J, -2, cstr!("v"), 0);
        (api.js_setglobal)(J, cstr!("o46"));

        (api.js_getglobal)(J, cstr!("o46"));
        (api.js_getproperty)(J, -1, cstr!("m"));
        (api.js_rot2)(J); /* [fun, this] */
        (api.js_pushnumber)(J, 3.0);
        (api.js_call)(J, 1);
        show(api, J, "rm");
        (api.js_pop)(J, 1);

        /* same thing from script land, so `this` is bound by the interpreter */
        run(api, J, "m46", "o46.m(7, 8)");

        p_int("top", (api.js_gettop)(J));
        (api.js_freestate)(J);
    });
}

/* ------------------------------------------------------------------------- */
/* row 47 — js_newcfunctionx (data + finalize)                               */
/* ------------------------------------------------------------------------- */

static mut D47A: [u8; 12] = *b"data-47-A\0\0\0";
static mut D47B: [u8; 12] = *b"data-47-B\0\0\0";
static mut FIN47: c_int = 0;

unsafe extern "C" fn fin47(_J: JS, data: *mut c_void) {
    FIN47 += 1;
    p_int("fin47.n", FIN47);
    p_str("fin47.data", data as *const c_char);
}

unsafe extern "C" fn cf47(J: JS) {
    let a = api();
    let d = (a.js_currentfunctiondata)(J);
    p_ptr_nonnull("cf47.data", d);
    p_str("cf47.data.str", d as *const c_char);
    (a.js_currentfunction)(J);
    p_str("cf47.fun.typeof", (a.js_typeof)(J, -1));
    p_int("cf47.fun.iscallable", (a.js_iscallable)(J, -1));
    /* Function.prototype.toString spells out the C function's name */
    p_str_at(a, J, "cf47.fun.str", -1);
    p_repr_at(a, J, "cf47.fun.repr", -1);
    (a.js_getproperty)(J, -1, cstr!("name"));
    p_str("cf47.fun.nameprop", (a.js_trystring)(J, -1, cstr!("<throw>")));
    (a.js_pop)(J, 1);
    (a.js_getproperty)(J, -1, cstr!("length"));
    p_num("cf47.fun.length", (a.js_tonumber)(J, -1));
    (a.js_pop)(J, 2);
    (a.js_pushstring)(J, cstr!("cf47-ret"));
}

#[test]
fn cfg47_newcfunctionx() {
    diff("cfg47_newcfunctionx", |api| unsafe {
        enter(api);
        FIN47 = 0;
        let J = newstate(api, 0);

        /* outside any call js_currentfunctiondata is NULL and
         * js_currentfunction pushes undefined */
        p_ptr_nonnull("outer.data", (api.js_currentfunctiondata)(J));
        (api.js_currentfunction)(J);
        p_str("outer.fun.typeof", (api.js_typeof)(J, -1));
        (api.js_pop)(J, 1);

        p_line("-- A: dropped then gc'd --");
        (api.js_newcfunctionx)(
            J,
            Some(cf47),
            cstr!("f47A"),
            1,
            addr_of_mut!(D47A) as *mut c_void,
            Some(fin47),
        );
        show(api, J, "A");
        (api.js_pushundefined)(J);
        (api.js_pushnumber)(J, 1.0);
        (api.js_call)(J, 1);
        show(api, J, "Ares");
        (api.js_pop)(J, 1);

        /* call it a second time, then drop the only reference and collect */
        (api.js_newcfunctionx)(
            J,
            Some(cf47),
            cstr!("f47A"),
            1,
            addr_of_mut!(D47A) as *mut c_void,
            Some(fin47),
        );
        (api.js_pushundefined)(J);
        (api.js_call)(J, 0);
        (api.js_pop)(J, 1);
        p_int("before.gc.top", (api.js_gettop)(J));
        p_line("-- gc(0) --");
        (api.js_gc)(J, 0);
        p_int("fin47.after.gc", FIN47);

        p_line("-- B: kept in a global --");
        (api.js_newcfunctionx)(
            J,
            Some(cf47),
            cstr!("f47B"),
            3,
            addr_of_mut!(D47B) as *mut c_void,
            Some(fin47),
        );
        (api.js_setglobal)(J, cstr!("f47B"));
        (api.js_getglobal)(J, cstr!("f47B"));
        (api.js_pushundefined)(J);
        (api.js_pushnumber)(J, 9.0);
        (api.js_call)(J, 1);
        show(api, J, "Bres");
        (api.js_pop)(J, 1);
        p_line("-- gc(1) --");
        (api.js_gc)(J, 1);
        p_int("fin47.after.gc2", FIN47);

        /* a data-less / finalize-less cfunctionx behaves like js_newcfunction */
        p_line("-- C: NULL data + NULL finalize --");
        (api.js_newcfunctionx)(J, Some(cf47), cstr!("f47C"), 0, null_mut(), None);
        (api.js_pushundefined)(J);
        (api.js_call)(J, 0);
        show(api, J, "Cres");
        (api.js_pop)(J, 1);

        p_line("-- freestate --");
        (api.js_freestate)(J);
        p_int("fin47.after.freestate", FIN47);
    });
}

/* ------------------------------------------------------------------------- */
/* row 48 — js_newcconstructor                                              */
/* ------------------------------------------------------------------------- */

unsafe extern "C" fn cf48_call(J: JS) {
    let a = api();
    p_line("cf48_call");
    p_int("cf48_call.top", (a.js_gettop)(J));
    p_str("cf48_call.this.typeof", (a.js_typeof)(J, 0));
    p_repr_at(a, J, "cf48_call.this.repr", 0);
    let mut i: c_int = 1;
    while i < (a.js_gettop)(J) {
        p_str_at(a, J, &format!("cf48_call.a{}", i), i);
        i += 1;
    }
    (a.js_pushstring)(J, cstr!("from-call"));
}

unsafe extern "C" fn cf48_ctor(J: JS) {
    let a = api();
    p_line("cf48_ctor");
    p_int("cf48_ctor.top", (a.js_gettop)(J));
    p_str("cf48_ctor.this.typeof", (a.js_typeof)(J, 0));
    p_repr_at(a, J, "cf48_ctor.this.repr", 0);
    let n = (a.js_gettop)(J);
    (a.js_newobject)(J);
    (a.js_pushnumber)(J, n as f64);
    (a.js_setproperty)(J, -2, cstr!("argc"));
    (a.js_pushstring)(J, cstr!("ctor"));
    (a.js_setproperty)(J, -2, cstr!("kind"));
}

#[test]
fn cfg48_newcconstructor() {
    diff("cfg48_newcconstructor", |api| unsafe {
        enter(api);
        let J = newstate(api, 0);

        /* js_newcconstructor consumes a prototype object from the stack */
        (api.js_newobject)(J);
        (api.js_newcconstructor)(J, Some(cf48_call), Some(cf48_ctor), cstr!("C48"), 2);
        show(api, J, "C48");
        (api.js_getproperty)(J, -1, cstr!("length"));
        p_num("C48.length", (api.js_tonumber)(J, -1));
        (api.js_pop)(J, 1);
        (api.js_getproperty)(J, -1, cstr!("prototype"));
        p_str("C48.prototype.typeof", (api.js_typeof)(J, -1));
        (api.js_getproperty)(J, -1, cstr!("constructor"));
        p_str("C48.prototype.constructor.typeof", (api.js_typeof)(J, -1));
        (api.js_pop)(J, 2);
        (api.js_setglobal)(J, cstr!("C48"));

        p_line("-- js_call n=0 --");
        (api.js_getglobal)(J, cstr!("C48"));
        (api.js_pushundefined)(J);
        (api.js_call)(J, 0);
        show(api, J, "call0");
        (api.js_pop)(J, 1);

        p_line("-- js_call n=2 with a real this --");
        (api.js_newobject)(J);
        (api.js_getglobal)(J, cstr!("C48"));
        (api.js_rot2)(J);
        (api.js_pushnumber)(J, 11.0);
        (api.js_pushstring)(J, cstr!("xx"));
        (api.js_call)(J, 2);
        show(api, J, "call2");
        (api.js_pop)(J, 1);

        p_line("-- js_construct n=0 --");
        (api.js_getglobal)(J, cstr!("C48"));
        (api.js_construct)(J, 0);
        show(api, J, "ctor0");
        (api.js_getproperty)(J, -1, cstr!("argc"));
        p_num("ctor0.argc", (api.js_trynumber)(J, -1, -1.0));
        (api.js_pop)(J, 2);

        p_line("-- js_construct n=2 --");
        (api.js_getglobal)(J, cstr!("C48"));
        (api.js_pushnumber)(J, 1.0);
        (api.js_pushnumber)(J, 2.0);
        (api.js_construct)(J, 2);
        show(api, J, "ctor2");
        (api.js_pop)(J, 1);

        p_line("-- js_pconstruct n=1 --");
        (api.js_getglobal)(J, cstr!("C48"));
        (api.js_pushstring)(J, cstr!("z"));
        let rc = (api.js_pconstruct)(J, 1);
        p_int("pctor1.rc", rc);
        show(api, J, "pctor1");
        (api.js_pop)(J, 1);

        p_line("-- new C48() from script --");
        run(api, J, "s48", "var q = new C48(4,5); q.kind + '/' + q.argc");

        /* NULL constructor: js_construct takes the generic path and passes a
         * freshly created object as `this` */
        p_line("-- ccon = NULL --");
        (api.js_newobject)(J);
        (api.js_newcconstructor)(J, Some(cf48_call), None, cstr!("C48b"), 1);
        (api.js_setglobal)(J, cstr!("C48b"));
        (api.js_getglobal)(J, cstr!("C48b"));
        (api.js_construct)(J, 0);
        show(api, J, "b.ctor0");
        (api.js_pop)(J, 1);
        (api.js_getglobal)(J, cstr!("C48b"));
        (api.js_pushnumber)(J, 6.0);
        let rc = (api.js_pconstruct)(J, 1);
        p_int("b.pctor1.rc", rc);
        show(api, J, "b.pctor1");
        (api.js_pop)(J, 1);

        /* NULL cfun, non-NULL ccon: plain call must fail */
        p_line("-- cfun = NULL --");
        (api.js_newobject)(J);
        (api.js_newcconstructor)(J, None, Some(cf48_ctor), cstr!("C48c"), 0);
        (api.js_setglobal)(J, cstr!("C48c"));
        (api.js_getglobal)(J, cstr!("C48c"));
        (api.js_construct)(J, 0);
        show(api, J, "c.ctor0");
        (api.js_pop)(J, 1);

        p_int("top", (api.js_gettop)(J));
        (api.js_freestate)(J);
    });
}

/* ------------------------------------------------------------------------- */
/* row 49 — js_newuserdata                                                  */
/* ------------------------------------------------------------------------- */

static mut D49A: [u8; 12] = *b"ud-49-A\0\0\0\0\0";
static mut D49B: [u8; 12] = *b"ud-49-B\0\0\0\0\0";
static mut FIN49: c_int = 0;

unsafe extern "C" fn fin49(_J: JS, data: *mut c_void) {
    FIN49 += 1;
    p_int("fin49.n", FIN49);
    p_str("fin49.data", data as *const c_char);
}

/// Reads argument 1 with the *right* tag, then with a wrong tag (which throws).
unsafe extern "C" fn cf49_bad(J: JS) {
    let a = api();
    p_int("cf49.is.right", (a.js_isuserdata)(J, 1, cstr!("tag49")));
    p_int("cf49.is.wrong", (a.js_isuserdata)(J, 1, cstr!("nope49")));
    let ok = (a.js_touserdata)(J, 1, cstr!("tag49"));
    p_ptr_nonnull("cf49.to.right", ok);
    p_str("cf49.to.right.str", ok as *const c_char);
    let bad = (a.js_touserdata)(J, 1, cstr!("nope49"));
    /* not reached: js_touserdata throws a TypeError on a tag mismatch */
    p_ptr_nonnull("cf49.to.wrong", bad);
    p_line("cf49 UNREACHABLE");
}

#[test]
fn cfg49_newuserdata() {
    diff("cfg49_newuserdata", |api| unsafe {
        enter(api);
        FIN49 = 0;
        let J = newstate(api, 0);

        /* js_newuserdatax consumes the prototype from the stack */
        (api.js_newobject)(J);
        (api.js_pushnumber)(J, 123.0);
        (api.js_defproperty)(J, -2, cstr!("protoprop"), 0);
        (api.js_newuserdata)(
            J,
            cstr!("tag49"),
            addr_of_mut!(D49A) as *mut c_void,
            Some(fin49),
        );
        show(api, J, "ud");
        p_int("ud.is.right", (api.js_isuserdata)(J, -1, cstr!("tag49")));
        p_int("ud.is.wrong", (api.js_isuserdata)(J, -1, cstr!("nope49")));
        p_int("ud.is.prefix", (api.js_isuserdata)(J, -1, cstr!("tag4")));
        let p = (api.js_touserdata)(J, -1, cstr!("tag49"));
        p_ptr_nonnull("ud.to.right", p);
        p_str("ud.to.right.str", p as *const c_char);
        /* the prototype survived */
        (api.js_getproperty)(J, -1, cstr!("protoprop"));
        p_num("ud.protoprop", (api.js_trynumber)(J, -1, -1.0));
        (api.js_pop)(J, 1);
        (api.js_setglobal)(J, cstr!("ud"));

        /* non-userdata values */
        p_line("-- non userdata --");
        (api.js_pushnumber)(J, 1.0);
        p_int("num.is", (api.js_isuserdata)(J, -1, cstr!("tag49")));
        (api.js_pop)(J, 1);
        (api.js_newobject)(J);
        p_int("obj.is", (api.js_isuserdata)(J, -1, cstr!("tag49")));
        (api.js_pop)(J, 1);
        (api.js_pushundefined)(J);
        p_int("undef.is", (api.js_isuserdata)(J, -1, cstr!("tag49")));
        (api.js_pop)(J, 1);

        /* the throwing js_touserdata, caught by js_pcall's own setjmp */
        p_line("-- wrong tag through js_pcall --");
        (api.js_newcfunction)(J, Some(cf49_bad), cstr!("cf49"), 1);
        (api.js_pushundefined)(J);
        (api.js_getglobal)(J, cstr!("ud"));
        let rc = (api.js_pcall)(J, 1);
        p_int("pcall.rc", rc);
        show(api, J, "pcall.res");
        (api.js_pop)(J, 1);

        /* wrong tag on a plain number, also through js_pcall */
        p_line("-- wrong tag on a number through js_pcall --");
        (api.js_newcfunction)(J, Some(cf49_bad), cstr!("cf49"), 1);
        (api.js_pushundefined)(J);
        (api.js_pushnumber)(J, 5.0);
        let rc = (api.js_pcall)(J, 1);
        p_int("pcall2.rc", rc);
        show(api, J, "pcall2.res");
        (api.js_pop)(J, 1);

        /* finalize on gc: drop the only reference, then collect */
        p_line("-- gc after delglobal --");
        (api.js_delglobal)(J, cstr!("ud"));
        (api.js_gc)(J, 0);
        p_int("fin49.after.gc", FIN49);

        /* finalize on freestate */
        p_line("-- B kept until freestate --");
        (api.js_pushundefined)(J); /* no prototype */
        (api.js_newuserdata)(
            J,
            cstr!("tag49b"),
            addr_of_mut!(D49B) as *mut c_void,
            Some(fin49),
        );
        show(api, J, "udb");
        (api.js_setglobal)(J, cstr!("udb"));

        /* a userdata with no finalizer at all */
        (api.js_pushnull)(J);
        (api.js_newuserdata)(J, cstr!("tag49c"), null_mut(), None);
        show(api, J, "udc");
        p_ptr_nonnull("udc.data", (api.js_touserdata)(J, -1, cstr!("tag49c")));
        (api.js_pop)(J, 1);
        (api.js_gc)(J, 0);
        p_int("fin49.after.gc2", FIN49);

        p_line("-- freestate --");
        (api.js_freestate)(J);
        p_int("fin49.after.freestate", FIN49);
    });
}

/* ------------------------------------------------------------------------- */
/* row 50 — js_newuserdatax (has / put / delete)                             */
/* ------------------------------------------------------------------------- */

static mut D50: [u8; 8] = *b"ud-50\0\0\0";
static mut FIN50: c_int = 0;

unsafe extern "C" fn fin50(_J: JS, data: *mut c_void) {
    FIN50 += 1;
    p_int("fin50.n", FIN50);
    p_str("fin50.data", data as *const c_char);
}

unsafe fn name_is(name: *const c_char, want: &str) -> bool {
    CStr::from_ptr(name).to_bytes() == want.as_bytes()
}

unsafe extern "C" fn has50(J: JS, data: *mut c_void, name: *const c_char) -> c_int {
    let a = api();
    p_str("has50.name", name);
    p_str("has50.data", data as *const c_char);
    if name_is(name, "alpha") {
        (a.js_pushnumber)(J, 111.0);
        p_line("has50 -> 1");
        return 1;
    }
    p_line("has50 -> 0");
    0
}

unsafe extern "C" fn put50(J: JS, data: *mut c_void, name: *const c_char) -> c_int {
    let a = api();
    p_str("put50.name", name);
    p_str("put50.data", data as *const c_char);
    p_str_at(a, J, "put50.value", -1);
    if name_is(name, "wprop") {
        p_line("put50 -> 1");
        return 1;
    }
    p_line("put50 -> 0");
    0
}

unsafe extern "C" fn del50(_J: JS, data: *mut c_void, name: *const c_char) -> c_int {
    p_str("del50.name", name);
    p_str("del50.data", data as *const c_char);
    if name_is(name, "dprop") {
        p_line("del50 -> 1");
        return 1;
    }
    p_line("del50 -> 0");
    0
}

unsafe fn seq50(
    a: &Api,
    J: JS,
    label: &str,
    has: Option<HasProp>,
    put: Option<PutProp>,
    del: Option<DelProp>,
) {
    p_line(&format!("== {} ==", label));
    (a.js_newobject)(J); /* prototype */
    (a.js_pushnumber)(J, 7.0);
    (a.js_defproperty)(J, -2, cstr!("inherited"), 0);
    (a.js_newuserdatax)(
        J,
        cstr!("tag50"),
        addr_of_mut!(D50) as *mut c_void,
        has,
        put,
        del,
        Some(fin50),
    );
    /* a plain own property on the userdata object itself */
    (a.js_pushnumber)(J, 5.0);
    (a.js_defproperty)(J, -2, cstr!("own"), 0);
    show(a, J, &format!("{}.ud", label));
    dump_props(a, J, &format!("{}.ud", label), 1);
    dump_props(a, J, &format!("{}.ud", label), 0);

    for nm in ["alpha", "own", "inherited", "other", "wprop", "dprop"].iter() {
        let cn = cs(nm);
        let n = cn.as_ptr();
        p_line(&format!("-- {}.{} --", label, nm));

        let rc = (a.js_hasproperty)(J, -1, n);
        p_int(&format!("{}.{}.has", label, nm), rc);
        if rc != 0 {
            show_at(a, J, &format!("{}.{}.hasval", label, nm), -1);
            (a.js_pop)(J, 1);
        }

        (a.js_getproperty)(J, -1, n);
        show_at(a, J, &format!("{}.{}.get", label, nm), -1);
        (a.js_pop)(J, 1);

        (a.js_pushstring)(J, cstr!("SET"));
        (a.js_setproperty)(J, -2, n);
        (a.js_getproperty)(J, -1, n);
        show_at(a, J, &format!("{}.{}.get2", label, nm), -1);
        (a.js_pop)(J, 1);

        (a.js_delproperty)(J, -1, n);
        let rc = (a.js_hasproperty)(J, -1, n);
        p_int(&format!("{}.{}.has3", label, nm), rc);
        if rc != 0 {
            show_at(a, J, &format!("{}.{}.has3val", label, nm), -1);
            (a.js_pop)(J, 1);
        }
    }

    /* index access goes through the same machinery via jsR_hasindex */
    let rc = (a.js_hasindex)(J, -1, 0);
    p_int(&format!("{}.hasindex0", label), rc);
    if rc != 0 {
        show_at(a, J, &format!("{}.index0", label), -1);
        (a.js_pop)(J, 1);
    }

    (a.js_pop)(J, 1); /* the userdata */
    (a.js_gc)(J, 0);
    p_int(&format!("{}.fin50", label), FIN50);
}

#[test]
fn cfg50_newuserdatax() {
    diff("cfg50_newuserdatax", |api| unsafe {
        enter(api);
        FIN50 = 0;
        let J = newstate(api, 0);
        seq50(api, J, "all", Some(has50), Some(put50), Some(del50));
        seq50(api, J, "nohas", None, Some(put50), Some(del50));
        seq50(api, J, "noput", Some(has50), None, Some(del50));
        seq50(api, J, "nodel", Some(has50), Some(put50), None);
        seq50(api, J, "none", None, None, None);
        p_int("top", (api.js_gettop)(J));
        (api.js_freestate)(J);
        p_int("fin50.after.freestate", FIN50);
    });
}

/* ------------------------------------------------------------------------- */
/* row 59 — js_ploadstring + js_pcall                                        */
/* ------------------------------------------------------------------------- */

#[test]
fn cfg59_ploadstring_pcall() {
    diff("cfg59_ploadstring_pcall", |api| unsafe {
        enter(api);
        let J = newstate(api, 0);

        run(api, J, "ok59", "1 + 2");
        run(api, J, "ok59b", "var a = 3; a * a");
        run(api, J, "ok59c", "'x' + 'y'");
        run(api, J, "syn59", "1 +* 2");
        run(api, J, "syn59b", "function (");
        run(api, J, "syn59c", "var 1x = 3;");
        run(api, J, "thr59", "throw new Error('boom')");
        run(api, J, "thr59b", "null.x");
        run(api, J, "thr59c", "undefinedFunction59()");
        run(api, J, "thr59d", "throw 42");

        p_line("-- pcall n larger than the script's arity --");
        let f = cs("argy59");
        let s = cs("7 * 6");
        p_int("argy.load", (api.js_ploadstring)(J, f.as_ptr(), s.as_ptr()));
        (api.js_pushundefined)(J);
        (api.js_pushnumber)(J, 1.0);
        (api.js_pushnumber)(J, 2.0);
        (api.js_pushnumber)(J, 3.0);
        let rc = (api.js_pcall)(J, 3);
        p_int("argy.rc", rc);
        show(api, J, "argy.res");
        (api.js_pop)(J, 1);
        p_int("argy.top", (api.js_gettop)(J));

        /* n smaller than what is on the stack: js_call looks at the wrong slot
         * and both libraries must raise the same "not callable" TypeError */
        p_line("-- pcall n smaller than pushed args --");
        let f = cs("small59");
        let s = cs("1");
        p_int("small.load", (api.js_ploadstring)(J, f.as_ptr(), s.as_ptr()));
        (api.js_pushundefined)(J);
        (api.js_pushnumber)(J, 10.0);
        (api.js_pushnumber)(J, 11.0);
        let rc = (api.js_pcall)(J, 0);
        p_int("small.rc", rc);
        show(api, J, "small.res");
        (api.js_pop)(J, 1);
        p_int("small.top", (api.js_gettop)(J));

        /* n larger than pushed args, with enough padding below so that
         * js_pcall's savetop stays inside the stack */
        p_line("-- pcall n larger than pushed args --");
        (api.js_pushnumber)(J, 100.0);
        (api.js_pushnumber)(J, 101.0);
        (api.js_pushnumber)(J, 102.0);
        (api.js_pushnumber)(J, 103.0);
        let f = cs("big59");
        let s = cs("2");
        p_int("big.load", (api.js_ploadstring)(J, f.as_ptr(), s.as_ptr()));
        (api.js_pushundefined)(J);
        (api.js_pushnumber)(J, 12.0);
        let rc = (api.js_pcall)(J, 3);
        p_int("big.rc", rc);
        show(api, J, "big.res");
        p_int("big.top", (api.js_gettop)(J));
        (api.js_pop)(J, (api.js_gettop)(J));

        p_int("top", (api.js_gettop)(J));
        (api.js_freestate)(J);
    });
}

/* ------------------------------------------------------------------------- */
/* row 60 — js_loadstring + js_eval                                          */
/* ------------------------------------------------------------------------- */

#[test]
fn cfg60_loadstring_eval() {
    diff("cfg60_loadstring_eval", |api| unsafe {
        enter(api);
        let J = newstate(api, 0);

        p_line("-- js_loadstring valid --");
        let f = cs("l60");
        let s = cs("var v60 = 4; v60 + 1");
        (api.js_loadstring)(J, f.as_ptr(), s.as_ptr());
        show(api, J, "script");
        (api.js_pushundefined)(J);
        (api.js_call)(J, 0);
        show(api, J, "res");
        (api.js_pop)(J, 1);
        (api.js_getglobal)(J, cstr!("v60"));
        show(api, J, "v60");
        (api.js_pop)(J, 1);

        p_line("-- js_loadstring empty source --");
        let f = cs("empty60");
        let s = cs("");
        (api.js_loadstring)(J, f.as_ptr(), s.as_ptr());
        show(api, J, "empty.script");
        (api.js_pushundefined)(J);
        (api.js_call)(J, 0);
        show(api, J, "empty.res");
        (api.js_pop)(J, 1);

        p_line("-- js_loadstring comments only --");
        let f = cs("cmt60");
        let s = cs("// just a line comment\n/* and a block\n   comment */\n");
        (api.js_loadstring)(J, f.as_ptr(), s.as_ptr());
        show(api, J, "cmt.script");
        (api.js_pushundefined)(J);
        (api.js_call)(J, 0);
        show(api, J, "cmt.res");
        (api.js_pop)(J, 1);

        p_line("-- js_loadstring whitespace only --");
        let f = cs("ws60");
        let s = cs("   \t\n  ");
        (api.js_loadstring)(J, f.as_ptr(), s.as_ptr());
        (api.js_pushundefined)(J);
        (api.js_call)(J, 0);
        show(api, J, "ws.res");
        (api.js_pop)(J, 1);

        /* js_eval copies stack index 0 as `this`, so keep a value there */
        p_line("-- js_eval --");
        (api.js_pushundefined)(J);
        p_int("eval.top0", (api.js_gettop)(J));
        (api.js_pushstring)(J, cstr!("3 * 4"));
        (api.js_eval)(J);
        show(api, J, "eval1");
        (api.js_pop)(J, 1);

        (api.js_pushstring)(J, cstr!("var ev = 'inner'; ev + '!'"));
        (api.js_eval)(J);
        show(api, J, "eval2");
        (api.js_pop)(J, 1);
        (api.js_getglobal)(J, cstr!("ev"));
        show(api, J, "ev");
        (api.js_pop)(J, 1);

        /* js_eval on a non-string is a no-op: the value stays on the stack */
        (api.js_pushnumber)(J, 99.0);
        (api.js_eval)(J);
        show(api, J, "eval.nonstring");
        (api.js_pop)(J, 1);

        (api.js_pushstring)(J, cstr!(""));
        (api.js_eval)(J);
        show(api, J, "eval.empty");
        (api.js_pop)(J, 1);

        p_int("eval.top1", (api.js_gettop)(J));
        (api.js_pop)(J, (api.js_gettop)(J));

        /* js_eval from script land, via the interpreter's OP_EVAL */
        run(api, J, "e60", "eval('1+1')");
        run(api, J, "e60b", "var g60 = 5; eval('g60 * 3')");

        p_int("top", (api.js_gettop)(J));
        (api.js_freestate)(J);
    });
}

/* ------------------------------------------------------------------------- */
/* row 61 — js_loadeval                                                      */
/* ------------------------------------------------------------------------- */

unsafe fn loadeval_run(a: &Api, J: JS, tag: &str, src: &str) {
    let f = cs(tag);
    let s = cs(src);
    let top0 = (a.js_gettop)(J);
    (a.js_loadeval)(J, f.as_ptr(), s.as_ptr());
    show(a, J, &format!("{}.script", tag));
    (a.js_pushundefined)(J);
    let rc = (a.js_pcall)(J, 0);
    p_int(&format!("{}.rc", tag), rc);
    show(a, J, &format!("{}.res", tag));
    (a.js_pop)(J, 1);
    p_int(&format!("{}.dtop", tag), (a.js_gettop)(J) - top0);
}

#[test]
fn cfg61_loadeval() {
    diff("cfg61_loadeval", |api| unsafe {
        enter(api);

        for strict in [0, JS_STRICT] {
            p_line(&format!("======== flags={} ========", strict));
            let J = newstate(api, strict);

            loadeval_run(api, J, "ev61a", "var x = 1; x + 1");
            /* in eval scope, non-strict, the var lands in the global object */
            (api.js_getglobal)(J, cstr!("x"));
            show(api, J, "global.x");
            (api.js_pop)(J, 1);

            loadeval_run(api, J, "ev61b", "function dbl(a) { return a * 2; } dbl(21)");
            (api.js_getglobal)(J, cstr!("dbl"));
            show(api, J, "global.dbl");
            if (api.js_iscallable)(J, -1) != 0 {
                (api.js_pushundefined)(J);
                (api.js_pushnumber)(J, 4.0);
                let rc = (api.js_pcall)(J, 1);
                p_int("dbl.rc", rc);
                show(api, J, "dbl.res");
                (api.js_pop)(J, 1);
            } else {
                (api.js_pop)(J, 1);
            }

            loadeval_run(
                api,
                J,
                "ev61c",
                "function a61(){return 'a'} function b61(){return a61()+'b'} b61()",
            );
            loadeval_run(api, J, "ev61d", "");
            loadeval_run(api, J, "ev61e", "// comment only");
            loadeval_run(api, J, "ev61f", "var q = 1, r = 2; q + r");
            loadeval_run(api, J, "ev61g", "'use strict'; var s61 = 3; s61");
            loadeval_run(api, J, "ev61h", "1; 2; 3");
            loadeval_run(api, J, "ev61i", "if (1) { 'yes' } else { 'no' }");

            p_int("top", (api.js_gettop)(J));
            (api.js_freestate)(J);
        }
    });
}

/* ------------------------------------------------------------------------- */
/* row 62 — js_call / js_pcall                                               */
/* ------------------------------------------------------------------------- */

unsafe extern "C" fn cf62(J: JS) {
    let a = api();
    let top = (a.js_gettop)(J);
    p_int("cf62.top", top);
    (a.js_pushstring)(J, cstr!("cf62("));
    let mut i: c_int = 1;
    while i < top {
        if i > 1 {
            (a.js_pushstring)(J, cstr!(","));
            (a.js_concat)(J);
        }
        (a.js_pushstring)(J, (a.js_trystring)(J, i, cstr!("<throw>")));
        (a.js_concat)(J);
        i += 1;
    }
    (a.js_pushstring)(J, cstr!(")"));
    (a.js_concat)(J);
}

/// `[fun, this, args...]` is already on the stack; call it both ways.
unsafe fn call62(a: &Api, J: JS, tag: &str, n: c_int, protected: c_int) {
    if protected != 0 {
        let rc = (a.js_pcall)(J, n);
        p_int(&format!("{}.rc", tag), rc);
    } else {
        (a.js_call)(J, n);
    }
    show(a, J, &format!("{}.res", tag));
    (a.js_pop)(J, 1);
    p_int(&format!("{}.top", tag), (a.js_gettop)(J));
}

#[test]
fn cfg62_call_pcall() {
    diff("cfg62_call_pcall", |api| unsafe {
        enter(api);
        let J = newstate(api, 0);

        let setup = cs(concat!(
            "function jf62(a, b) { return [a, b, typeof this].join('|'); }\n",
            "var o62 = { v: 9, m: function (a) { return this.v + '/' + a; } };\n",
            "var bm62 = o62.m.bind(o62);\n",
            "var notcallable62 = 17;\n"
        ));
        p_int("setup.rc", (api.js_dostring)(J, setup.as_ptr()));

        (api.js_newcfunction)(J, Some(cf62), cstr!("cf62"), 2);
        (api.js_setglobal)(J, cstr!("cf62"));

        for name in ["jf62", "cf62", "bm62", "notcallable62"].iter() {
            let cn = cs(name);
            p_line(&format!("======== {} ========", name));
            (api.js_getglobal)(J, cn.as_ptr());
            show(api, J, &format!("{}.fun", name));
            (api.js_pop)(J, 1);

            for n in [0, 1, 5].iter() {
                /* js_pcall, undefined this */
                p_line(&format!("-- {} pcall n={} --", name, n));
                (api.js_getglobal)(J, cn.as_ptr());
                (api.js_pushundefined)(J);
                for k in 0..*n {
                    (api.js_pushnumber)(J, (k + 1) as f64);
                }
                call62(api, J, &format!("{}.p{}", name, n), *n, 1);

                /* js_call, only when the value is actually callable, so that
                 * the child does not abort in the middle of the test */
                (api.js_getglobal)(J, cn.as_ptr());
                let ok = (api.js_iscallable)(J, -1);
                if ok != 0 {
                    p_line(&format!("-- {} call n={} --", name, n));
                    (api.js_pushundefined)(J);
                    for k in 0..*n {
                        (api.js_pushnumber)(J, (k + 1) as f64);
                    }
                    call62(api, J, &format!("{}.c{}", name, n), *n, 0);
                } else {
                    (api.js_pop)(J, 1);
                }
            }

            /* method style: real `this` */
            p_line(&format!("-- {} method --", name));
            (api.js_getglobal)(J, cstr!("o62"));
            (api.js_getglobal)(J, cn.as_ptr());
            (api.js_rot2)(J);
            (api.js_pushnumber)(J, 4.0);
            call62(api, J, &format!("{}.m", name), 1, 1);
        }

        /* o62.m through js_getproperty, as a genuine bound method */
        p_line("======== o62.m ========");
        (api.js_getglobal)(J, cstr!("o62"));
        (api.js_getproperty)(J, -1, cstr!("m"));
        (api.js_rot2)(J);
        (api.js_pushstring)(J, cstr!("mm"));
        call62(api, J, "o62.m", 1, 1);

        /* negative n */
        p_line("======== negative n ========");
        (api.js_getglobal)(J, cstr!("jf62"));
        (api.js_pushundefined)(J);
        let rc = (api.js_pcall)(J, -1);
        p_int("neg.rc", rc);
        show(api, J, "neg.res");
        (api.js_pop)(J, 1);
        p_int("neg.top", (api.js_gettop)(J));
        (api.js_pop)(J, (api.js_gettop)(J));

        /* from script land */
        run(api, J, "s62", "jf62(1,2) + ';' + cf62('a','b') + ';' + bm62('z')");

        p_int("top", (api.js_gettop)(J));
        (api.js_freestate)(J);
    });
}

/* ------------------------------------------------------------------------- */
/* row 63 — js_construct / js_pconstruct                                     */
/* ------------------------------------------------------------------------- */

unsafe fn construct63(a: &Api, J: JS, tag: &str, name: &str, args: &[f64], strs: &[&str], prot: c_int) {
    let cn = cs(name);
    p_line(&format!("-- {} --", tag));
    let top0 = (a.js_gettop)(J);
    /* js_construct's stack layout is [fun, args...] -- there is no `this`
     * slot -- but js_pconstruct recovers with `savetop = TOP - n - 2`, i.e.
     * the js_pcall layout (c_src/src/jsrun.c:1400).  A throwing
     * js_pconstruct therefore writes one slot *below* the constructor, so
     * push a filler to keep that write inside the stack. */
    if prot != 0 {
        (a.js_pushundefined)(J);
    }
    (a.js_getglobal)(J, cn.as_ptr());
    for v in args.iter() {
        (a.js_pushnumber)(J, *v);
    }
    let mut keep: Vec<std::ffi::CString> = Vec::new();
    for s in strs.iter() {
        let c = cs(s);
        (a.js_pushstring)(J, c.as_ptr());
        keep.push(c);
    }
    let n = (args.len() + strs.len()) as c_int;
    if prot != 0 {
        let rc = (a.js_pconstruct)(J, n);
        p_int(&format!("{}.rc", tag), rc);
    } else {
        (a.js_construct)(J, n);
    }
    show(a, J, &format!("{}.res", tag));
    for k in ["x", "y", "length", "message", "name", "source", "global"].iter() {
        let ck = cs(k);
        if (a.js_hasproperty)(J, -1, ck.as_ptr()) != 0 {
            show_at(a, J, &format!("{}.res.{}", tag, k), -1);
            (a.js_pop)(J, 1);
        }
    }
    p_int(&format!("{}.dtop", tag), (a.js_gettop)(J) - top0);
    (a.js_pop)(J, (a.js_gettop)(J) - top0);
    p_int(&format!("{}.top", tag), (a.js_gettop)(J));
}

#[test]
fn cfg63_construct_pconstruct() {
    diff("cfg63_construct_pconstruct", |api| unsafe {
        enter(api);
        let J = newstate(api, 0);

        let setup = cs(concat!(
            "function Point63(x, y) { this.x = x; this.y = y; }\n",
            "Point63.prototype.tag = 'point';\n",
            "function Ret63(x) { this.x = x; return { x: 'replaced' }; }\n",
            "function RetPrim63(x) { this.x = x; return 5; }\n",
            "var notctor63 = 'nope';\n"
        ));
        p_int("setup.rc", (api.js_dostring)(J, setup.as_ptr()));

        for prot in [0, 1] {
            p_line(&format!("======== protected={} ========", prot));

            construct63(api, J, &format!("Point63.0.p{}", prot), "Point63", &[], &[], prot);
            construct63(api, J, &format!("Point63.1.p{}", prot), "Point63", &[3.0], &[], prot);
            construct63(
                api,
                J,
                &format!("Point63.2.p{}", prot),
                "Point63",
                &[3.0, 4.0],
                &[],
                prot,
            );
            construct63(api, J, &format!("Ret63.p{}", prot), "Ret63", &[1.0], &[], prot);
            construct63(
                api,
                J,
                &format!("RetPrim63.p{}", prot),
                "RetPrim63",
                &[1.0],
                &[],
                prot,
            );

            construct63(api, J, &format!("Object.0.p{}", prot), "Object", &[], &[], prot);
            construct63(api, J, &format!("Object.1.p{}", prot), "Object", &[7.0], &[], prot);
            construct63(api, J, &format!("Array.0.p{}", prot), "Array", &[], &[], prot);
            construct63(api, J, &format!("Array.1.p{}", prot), "Array", &[5.0], &[], prot);
            construct63(
                api,
                J,
                &format!("Array.2.p{}", prot),
                "Array",
                &[5.0, 6.0],
                &[],
                prot,
            );
            construct63(api, J, &format!("Error.0.p{}", prot), "Error", &[], &[], prot);
            construct63(
                api,
                J,
                &format!("Error.1.p{}", prot),
                "Error",
                &[],
                &["oops"],
                prot,
            );
            construct63(
                api,
                J,
                &format!("TypeError.1.p{}", prot),
                "TypeError",
                &[],
                &["te"],
                prot,
            );
            construct63(
                api,
                J,
                &format!("RegExp.1.p{}", prot),
                "RegExp",
                &[],
                &["a+b"],
                prot,
            );
            construct63(
                api,
                J,
                &format!("RegExp.2.p{}", prot),
                "RegExp",
                &[],
                &["a+b", "gi"],
                prot,
            );
            construct63(
                api,
                J,
                &format!("Boolean.1.p{}", prot),
                "Boolean",
                &[1.0],
                &[],
                prot,
            );
            construct63(
                api,
                J,
                &format!("Number.1.p{}", prot),
                "Number",
                &[2.5],
                &[],
                prot,
            );
            construct63(
                api,
                J,
                &format!("String.1.p{}", prot),
                "String",
                &[],
                &["hey"],
                prot,
            );

            if prot != 0 {
                /* non-constructor: identical TypeError required.  Only through
                 * js_pconstruct, so the child survives to the end. */
                construct63(
                    api,
                    J,
                    &format!("notctor63.0.p{}", prot),
                    "notctor63",
                    &[],
                    &[],
                    prot,
                );
                construct63(
                    api,
                    J,
                    &format!("notctor63.2.p{}", prot),
                    "notctor63",
                    &[1.0, 2.0],
                    &[],
                    prot,
                );
                construct63(
                    api,
                    J,
                    &format!("undefined63.p{}", prot),
                    "thereIsNoSuchGlobal63",
                    &[],
                    &[],
                    prot,
                );
                /* Math is an object but not callable */
                construct63(api, J, &format!("Math.p{}", prot), "Math", &[], &[], prot);
            }
        }

        /* a C function without a constructor slot: generic construct path */
        p_line("======== cfunction ========");
        (api.js_newcfunction)(J, Some(cf62), cstr!("cf63"), 1);
        (api.js_setglobal)(J, cstr!("cf63"));
        construct63(api, J, "cf63.p1", "cf63", &[1.0], &[], 1);

        run(api, J, "s63", "var p = new Point63(1,2); p.x + ',' + p.y + ',' + p.tag");
        run(api, J, "s63b", "String(new Array(3).length)");

        p_int("top", (api.js_gettop)(J));
        (api.js_freestate)(J);
    });
}

/* ------------------------------------------------------------------------- */
/* row 79 — js_savetry / js_endtry / js_throw                                */
/* ------------------------------------------------------------------------- */

unsafe extern "C" fn cf79_throw(J: JS) {
    let a = api();
    p_line("cf79_throw enter");
    p_int("cf79.top", (a.js_gettop)(J));
    (a.js_newerror)(J, cstr!("thrown-from-c"));
    p_str("cf79.err.typeof", (a.js_typeof)(J, -1));
    p_int("cf79.err.iserror", (a.js_iserror)(J, -1));
    (a.js_throw)(J);
    p_line("cf79_throw UNREACHABLE");
}

unsafe extern "C" fn cf79_throwstr(J: JS) {
    let a = api();
    p_line("cf79_throwstr enter");
    (a.js_pushstring)(J, cstr!("a-plain-string"));
    (a.js_throw)(J);
    p_line("cf79_throwstr UNREACHABLE");
}

unsafe extern "C" fn cf79_nested(J: JS) {
    let a = api();
    p_line("cf79_nested enter");
    /* an inner js_pcall catches the inner throw, then we throw again */
    (a.js_newcfunction)(J, Some(cf79_throw), cstr!("inner79"), 0);
    (a.js_pushundefined)(J);
    let rc = (a.js_pcall)(J, 0);
    p_int("cf79_nested.inner.rc", rc);
    p_str("cf79_nested.inner.err", (a.js_trystring)(J, -1, cstr!("<throw>")));
    (a.js_pop)(J, 1);
    (a.js_newtypeerror)(J, cstr!("outer-79"));
    (a.js_throw)(J);
}

#[test]
fn cfg79_savetry_endtry_throw() {
    /* (a) try / catch / finally through the script entry points */
    diff("cfg79_scripts", |api| unsafe {
        enter(api);
        let J = newstate(api, 0);
        run(api, J, "t79a", "try { 1 } catch (e) { 2 }");
        run(api, J, "t79b", "try { throw 3 } catch (e) { e }");
        run(api, J, "t79c", "try { throw new Error('x') } catch (e) { e.message } finally { }");
        run(api, J, "t79d", "try { 1 } finally { 2 }");
        run(api, J, "t79e", "(function () { try { return 1 } finally { return 2 } })()");
        run(
            api,
            J,
            "t79f",
            "(function () { try { throw 1 } catch (e) { return 'c' + e } finally { } })()",
        );
        run(api, J, "t79g", "try { throw 1 } finally { }");
        run(
            api,
            J,
            "t79h",
            "try { try { throw 'i' } finally { } } catch (e) { 'outer:' + e }",
        );
        run(
            api,
            J,
            "t79i",
            "var s=''; try { throw 1 } catch(e) { s+='c' } finally { s+='f' } s",
        );
        run(
            api,
            J,
            "t79j",
            "var s=''; for (var i=0;i<3;++i) { try { if (i==1) continue; s+=i } finally { s+='F' } } s",
        );
        run(api, J, "t79k", "try { null.x } catch (e) { e.name }");

        /* the uncaught case also goes through js_report */
        let src = cs("try { throw new Error('reported') } finally { }");
        p_int("dostring.rc", (api.js_dostring)(J, src.as_ptr()));
        let src = cs("try { 1 } catch (e) { 2 } finally { 3 }");
        p_int("dostring.rc2", (api.js_dostring)(J, src.as_ptr()));

        p_int("top", (api.js_gettop)(J));
        (api.js_freestate)(J);
    });

    /* (b) balanced js_savetry / js_endtry pairs.  The returned jmp_buf is
     * never handed to setjmp (impossible from Rust), so nothing may throw
     * while these frames are live. */
    diff("cfg79_balanced", |api| unsafe {
        enter(api);
        let J = newstate(api, 0);
        (api.js_pushnumber)(J, 1.0);
        (api.js_pushnumber)(J, 2.0);
        p_int("top.before", (api.js_gettop)(J));
        let b = (api.js_savetry)(J);
        p_ptr_nonnull("savetry.buf", b);
        p_int("top.insidetry", (api.js_gettop)(J));
        (api.js_pushnumber)(J, 3.0);
        p_int("top.insidetry2", (api.js_gettop)(J));
        (api.js_endtry)(J);
        p_int("top.afterendtry", (api.js_gettop)(J));
        show(api, J, "topvalue");

        /* nested balanced pairs */
        for i in 0..8 {
            let b = (api.js_savetry)(J);
            p_ptr_nonnull(&format!("nest{}.buf", i), b);
            p_int(&format!("nest{}.top", i), (api.js_gettop)(J));
        }
        for i in 0..8 {
            (api.js_endtry)(J);
            p_int(&format!("unnest{}.top", i), (api.js_gettop)(J));
        }
        p_int("top.final", (api.js_gettop)(J));

        /* js_savetry does not disturb the ability to run scripts */
        let b = (api.js_savetry)(J);
        p_ptr_nonnull("around.buf", b);
        run(api, J, "in79", "6 * 7");
        (api.js_endtry)(J);
        p_int("top.after", (api.js_gettop)(J));

        (api.js_pop)(J, (api.js_gettop)(J));
        (api.js_freestate)(J);
    });

    /* (d) js_throw from a C callback, caught by js_pcall's own setjmp */
    diff("cfg79_throw_from_c", |api| unsafe {
        enter(api);
        let J = newstate(api, 0);

        (api.js_newcfunction)(J, Some(cf79_throw), cstr!("thrower79"), 0);
        (api.js_pushundefined)(J);
        let rc = (api.js_pcall)(J, 0);
        p_int("pcall.rc", rc);
        show(api, J, "pcall.res");
        (api.js_pop)(J, 1);
        p_int("pcall.top", (api.js_gettop)(J));

        (api.js_newcfunction)(J, Some(cf79_throwstr), cstr!("thrower79s"), 0);
        (api.js_pushundefined)(J);
        (api.js_pushnumber)(J, 1.0);
        (api.js_pushnumber)(J, 2.0);
        let rc = (api.js_pcall)(J, 2);
        p_int("pcall2.rc", rc);
        show(api, J, "pcall2.res");
        (api.js_pop)(J, 1);
        p_int("pcall2.top", (api.js_gettop)(J));

        (api.js_newcfunction)(J, Some(cf79_nested), cstr!("thrower79n"), 0);
        (api.js_pushundefined)(J);
        let rc = (api.js_pcall)(J, 0);
        p_int("pcall3.rc", rc);
        show(api, J, "pcall3.res");
        (api.js_pop)(J, 1);
        p_int("pcall3.top", (api.js_gettop)(J));

        /* the same C thrower, caught by a JS try/catch this time */
        (api.js_newcfunction)(J, Some(cf79_throw), cstr!("thrower79"), 0);
        (api.js_setglobal)(J, cstr!("thrower79"));
        run(api, J, "c79", "try { thrower79() } catch (e) { 'caught:' + e.message }");
        run(api, J, "c79b", "try { thrower79() } catch (e) { e.name } finally { }");

        /* js_pconstruct also installs a try frame.  It unwinds to
         * `TOP - n - 2`, one slot below js_construct's [fun, args...] layout,
         * so a filler is needed to keep that write inside the stack. */
        (api.js_pushundefined)(J);
        (api.js_getglobal)(J, cstr!("thrower79"));
        let rc = (api.js_pconstruct)(J, 0);
        p_int("pconstruct.rc", rc);
        show(api, J, "pconstruct.res");
        p_int("pconstruct.top", (api.js_gettop)(J));
        (api.js_pop)(J, (api.js_gettop)(J));

        p_int("top", (api.js_gettop)(J));
        (api.js_freestate)(J);
    });

    /* (c) exhaust the try stack.  JS_TRYLIMIT is 64; the 65th js_savetry calls
     * js_trystackoverflow which throws -- and the throw longjmps into a
     * jmp_buf that was never initialised by setjmp.  Isolated in its own
     * diff() call because the child is expected to die. */
    diff("cfg79_exhaust", |api| unsafe {
        enter(api);
        let J = newstate(api, 0);
        let mut i = 0;
        while i < 70 {
            let b = (api.js_savetry)(J);
            p_int("savetry.i", i);
            p_ptr_nonnull("savetry.buf", b);
            i += 1;
        }
        p_line("survived 70 savetry calls");
        p_int("top", (api.js_gettop)(J));
        (api.js_freestate)(J);
    });
}

/* ------------------------------------------------------------------------- */
/* row 80 — js_savetrypc                                                     */
/* ------------------------------------------------------------------------- */

#[test]
fn cfg80_savetrypc() {
    diff("cfg80_balanced", |api| unsafe {
        enter(api);
        let J = newstate(api, 0);
        (api.js_pushstring)(J, cstr!("keep-me"));
        p_int("top.before", (api.js_gettop)(J));
        let b = (api.js_savetrypc)(J, null_mut());
        p_ptr_nonnull("savetrypc.buf", b);
        p_int("top.inside", (api.js_gettop)(J));
        (api.js_pushnumber)(J, 5.0);
        (api.js_endtry)(J);
        p_int("top.after", (api.js_gettop)(J));
        show(api, J, "top.value");
        (api.js_pop)(J, 1);
        show(api, J, "bottom.value");

        /* interleave savetry and savetrypc */
        for i in 0..6 {
            let b = if i % 2 == 0 {
                (api.js_savetrypc)(J, null_mut())
            } else {
                (api.js_savetry)(J)
            };
            p_ptr_nonnull(&format!("mix{}.buf", i), b);
            p_int(&format!("mix{}.top", i), (api.js_gettop)(J));
        }
        for i in 0..6 {
            (api.js_endtry)(J);
            p_int(&format!("unmix{}.top", i), (api.js_gettop)(J));
        }
        p_int("top.final", (api.js_gettop)(J));

        /* scripts still run with a savetrypc frame live */
        let b = (api.js_savetrypc)(J, null_mut());
        p_ptr_nonnull("around.buf", b);
        run(api, J, "in80", "'ok' + 80");
        (api.js_endtry)(J);
        p_int("top.end", (api.js_gettop)(J));

        (api.js_pop)(J, (api.js_gettop)(J));
        (api.js_freestate)(J);
    });

    /* endtry with an empty try stack raises "endtry: exception stack
     * underflow" through js_error -> js_throw -> panic; keep it out of the
     * other closures. */
    diff("cfg80_underflow", |api| unsafe {
        enter(api);
        let J = newstate(api, 0);
        p_int("top", (api.js_gettop)(J));
        (api.js_endtry)(J);
        p_line("survived endtry underflow");
        p_int("top2", (api.js_gettop)(J));
        (api.js_freestate)(J);
    });

    diff("cfg80_exhaust", |api| unsafe {
        enter(api);
        let J = newstate(api, 0);
        let mut i = 0;
        while i < 70 {
            let b = (api.js_savetrypc)(J, null_mut());
            p_int("savetrypc.i", i);
            p_ptr_nonnull("savetrypc.buf", b);
            i += 1;
        }
        p_line("survived 70 savetrypc calls");
        p_int("top", (api.js_gettop)(J));
        (api.js_freestate)(J);
    });
}

/* ------------------------------------------------------------------------- */
/* row 81 — js_trap                                                          */
/* ------------------------------------------------------------------------- */
/*
 * js_trap -> js_dumpstack + js_stacktrace.  js_dumpvalue prints "%p" for
 * every JS_TOBJECT that is not the global object *except* for
 *   JS_CSCRIPT     -> "[Script <filename>]"
 *   JS_CCFUNCTION  -> "[CFunction <name>]"
 *   JS_CERROR      -> "[Error]"
 *   JS_CBOOLEAN/JS_CNUMBER/JS_CSTRING -> value only
 * (see c_src/src/jsrun.c:1486-1525).  Addresses can never be byte-identical
 * between the two libraries, so this test only ever traps with primitives,
 * the global object, script objects, C function objects and wrapper objects
 * on the value stack -- never plain objects, arrays, JS functions, arguments,
 * iterators or userdata.  That is verified empirically: the test passes only
 * because no "%p" branch is reached.
 */

unsafe extern "C" fn cf81_trap(J: JS) {
    let a = api();
    p_line("cf81 enter");
    (a.js_trap)(J, 0);
    (a.js_trap)(J, 3);
    (a.js_pushnumber)(J, 1.0);
}

#[test]
fn cfg81_trap() {
    diff("cfg81_fresh", |api| unsafe {
        enter(api);
        let J = newstate(api, 0);
        p_line("-- empty stack --");
        (api.js_trap)(J, 0);
        (api.js_trap)(J, 1);
        (api.js_trap)(J, 2);
        (api.js_trap)(J, 7);
        (api.js_trap)(J, -1);

        p_line("-- primitives --");
        (api.js_pushundefined)(J);
        (api.js_pushnull)(J);
        (api.js_pushboolean)(J, 1);
        (api.js_pushboolean)(J, 0);
        (api.js_pushnumber)(J, 42.0);
        (api.js_pushnumber)(J, -0.5);
        (api.js_pushnumber)(J, 1.0 / 3.0);
        (api.js_pushnumber)(J, f64::INFINITY);
        (api.js_pushnumber)(J, f64::NAN);
        (api.js_pushstring)(J, cstr!("short")); /* shrstr */
        (api.js_pushliteral)(J, cstr!("a literal string")); /* litstr */
        (api.js_pushstring)(J, cstr!("a fairly long heap allocated string")); /* memstr */
        (api.js_pushglobal)(J); /* [Global] */
        (api.js_trap)(J, 0);
        p_int("top", (api.js_gettop)(J));
        (api.js_trap)(J, 5);

        p_line("-- wrapper objects (address free branches) --");
        (api.js_newboolean)(J, 1);
        (api.js_newnumber)(J, 2.5);
        (api.js_newstring)(J, cstr!("wrapped"));
        (api.js_newerror)(J, cstr!("an error"));
        (api.js_trap)(J, 0);

        (api.js_pop)(J, (api.js_gettop)(J));
        p_line("-- after popping everything --");
        (api.js_trap)(J, 0);
        (api.js_freestate)(J);
    });

    diff("cfg81_script_object", |api| unsafe {
        enter(api);
        let J = newstate(api, 0);
        /* a JS_CSCRIPT prints its filename, no address */
        let f = cs("trap81.js");
        let s = cs("1 + 1");
        (api.js_loadstring)(J, f.as_ptr(), s.as_ptr());
        (api.js_pushnumber)(J, 9.0);
        (api.js_trap)(J, 0);
        (api.js_pop)(J, 2);
        (api.js_freestate)(J);
    });

    diff("cfg81_in_cfunction", |api| unsafe {
        enter(api);
        let J = newstate(api, 0);
        /* inside a C function the stack holds the JS_CCFUNCTION object (name
         * only), the `this` value and the arguments -- all address free, and
         * BOT is non-zero so the '>' marker moves */
        (api.js_newcfunction)(J, Some(cf81_trap), cstr!("cf81"), 1);
        (api.js_pushundefined)(J);
        (api.js_pushnumber)(J, 11.0);
        (api.js_pushstring)(J, cstr!("arg81"));
        let rc = (api.js_pcall)(J, 2);
        p_int("rc", rc);
        show(api, J, "res");
        (api.js_pop)(J, 1);
        (api.js_freestate)(J);
    });

    diff("cfg81_in_script", |api| unsafe {
        enter(api);
        let J = newstate(api, 0);
        /* the `debugger` statement compiles to OP_DEBUGGER -> js_trap.  Inside
         * a top level script the value stack only holds the script object and
         * `this`, so the dump stays address free. */
        let src = cs("var d81 = 1; debugger; d81 + 1");
        p_int("rc", (api.js_dostring)(J, src.as_ptr()));
        let src = cs("debugger;");
        p_int("rc2", (api.js_dostring)(J, src.as_ptr()));
        let src = cs("var s81 = 'str'; var n81 = 7; debugger; n81");
        p_int("rc3", (api.js_dostring)(J, src.as_ptr()));
        p_int("top", (api.js_gettop)(J));
        (api.js_freestate)(J);
    });
}

/* ------------------------------------------------------------------------- */
/* row 82 — jsB_init / jsB_init* / jsB_propf / jsB_propn / jsB_props          */
/* ------------------------------------------------------------------------- */

unsafe extern "C" fn cf82(J: JS) {
    let a = api();
    p_int("cf82.top", (a.js_gettop)(J));
    (a.js_pushstring)(J, cstr!("cf82-called"));
}

/// Probe a fixed list of globals so the effect of an init function is visible
/// even though every builtin global is JS_DONTENUM (and therefore invisible to
/// js_pushiterator).
unsafe fn probe_globals(a: &Api, J: JS, tag: &str) {
    for name in [
        "Object", "Array", "Function", "Boolean", "Number", "String", "RegExp", "Date", "Error",
        "EvalError", "RangeError", "ReferenceError", "SyntaxError", "TypeError", "URIError",
        "Math", "JSON", "NaN", "Infinity", "undefined", "parseInt", "parseFloat", "isNaN",
        "isFinite", "decodeURI", "decodeURIComponent", "encodeURI", "encodeURIComponent",
    ]
    .iter()
    {
        let cn = cs(name);
        let rc = (a.js_hasproperty)(J, -1, cn.as_ptr());
        p_int(&format!("{}.{}.has", tag, name), rc);
        if rc != 0 {
            (a.js_pop)(J, 1);
        }
        (a.js_getglobal)(J, cn.as_ptr());
        p_str(&format!("{}.{}.typeof", tag, name), (a.js_typeof)(J, -1));
        p_repr_at(a, J, &format!("{}.{}.repr", tag, name), -1);
        (a.js_pop)(J, 1);
    }
}

#[test]
fn cfg82_builtins() {
    /* js_newstate already calls jsB_init (c_src/src/jsstate.c:229), so this
     * runs jsB_init a *second* time on an initialised state. */
    diff("cfg82_reinit", |api| unsafe {
        enter(api);
        let J = newstate(api, 0);

        (api.js_pushglobal)(J);
        dump_props(api, J, "before", 1);
        dump_props(api, J, "before", 0);
        probe_globals(api, J, "before");
        (api.js_pop)(J, 1);

        /* enumerable globals, so the iterator has something to report */
        (api.js_pushnumber)(J, 1.0);
        (api.js_defglobal)(J, cstr!("enum_a"), 0);
        (api.js_pushstring)(J, cstr!("two"));
        (api.js_defglobal)(J, cstr!("enum_b"), 0);
        (api.js_pushnumber)(J, 3.0);
        (api.js_defglobal)(J, cstr!("enum_c"), JS_DONTENUM);

        p_line("-- jsB_init again --");
        (api.jsB_init)(J);

        (api.js_pushglobal)(J);
        dump_props(api, J, "after", 1);
        dump_props(api, J, "after", 0);
        probe_globals(api, J, "after");
        (api.js_pop)(J, 1);

        run(api, J, "s82", "typeof Object + '/' + typeof Math.sin + '/' + [1,2].join('-')");
        run(api, J, "s82b", "String(new Error('e').name) + '/' + JSON.stringify({a:1})");
        p_int("top", (api.js_gettop)(J));
        (api.js_freestate)(J);
    });

    /* each jsB_init* individually, a second time, on its own fresh state */
    diff("cfg82_each", |api| unsafe {
        enter(api);
        let inits: [(&str, unsafe extern "C" fn(JS)); 11] = [
            ("object", api.jsB_initobject),
            ("array", api.jsB_initarray),
            ("function", api.jsB_initfunction),
            ("boolean", api.jsB_initboolean),
            ("number", api.jsB_initnumber),
            ("string", api.jsB_initstring),
            ("regexp", api.jsB_initregexp),
            ("error", api.jsB_initerror),
            ("math", api.jsB_initmath),
            ("json", api.jsB_initjson),
            ("date", api.jsB_initdate),
        ];
        for (name, f) in inits.iter() {
            p_line(&format!("======== jsB_init{} ========", name));
            let J = newstate(api, 0);
            p_int(&format!("{}.top0", name), (api.js_gettop)(J));
            (*f)(J);
            p_int(&format!("{}.top1", name), (api.js_gettop)(J));
            (api.js_pushglobal)(J);
            dump_props(api, J, name, 1);
            probe_globals(api, J, name);
            (api.js_pop)(J, 1);
            run(api, J, &format!("s82_{}", name), "typeof Object + typeof Array");
            (api.js_gc)(J, 1);
            p_int(&format!("{}.top2", name), (api.js_gettop)(J));
            (api.js_freestate)(J);
        }
    });

    /* jsB_propf / jsB_propn / jsB_props on a pushed object */
    diff("cfg82_props", |api| unsafe {
        enter(api);
        let J = newstate(api, 0);
        (api.js_newobject)(J);
        (api.jsB_propf)(J, cstr!("Thing.prototype.doit"), Some(cf82), 2);
        (api.jsB_propf)(J, cstr!("bare"), Some(cf82), 0);
        (api.jsB_propn)(J, cstr!("num"), 42.5);
        (api.jsB_propn)(J, cstr!("neg"), -0.0);
        (api.jsB_props)(J, cstr!("str"), cstr!("a string value"));
        (api.jsB_props)(J, cstr!("empty"), cstr!(""));
        p_int("top", (api.js_gettop)(J));

        /* every jsB_prop* uses JS_DONTENUM, so the iterator sees nothing */
        dump_props(api, J, "obj", 1);
        dump_props(api, J, "obj", 0);

        for name in ["doit", "Thing.prototype.doit", "bare", "num", "neg", "str", "empty"].iter() {
            let cn = cs(name);
            let rc = (api.js_hasproperty)(J, -1, cn.as_ptr());
            p_int(&format!("obj.{}.has", name), rc);
            if rc != 0 {
                (api.js_pop)(J, 1);
            }
            (api.js_getproperty)(J, -1, cn.as_ptr());
            show_at(api, J, &format!("obj.{}", name), -1);
            (api.js_pop)(J, 1);
        }

        /* call the function property as a method */
        (api.js_getproperty)(J, -1, cstr!("doit"));
        (api.js_copy)(J, -2);
        (api.js_pushnumber)(J, 1.0);
        let rc = (api.js_pcall)(J, 1);
        p_int("doit.rc", rc);
        show(api, J, "doit.res");
        (api.js_pop)(J, 1);

        /* readonly semantics of jsB_propn (JS_READONLY) */
        (api.js_pushnumber)(J, 999.0);
        (api.js_setproperty)(J, -2, cstr!("num"));
        (api.js_getproperty)(J, -1, cstr!("num"));
        show_at(api, J, "num.after.set", -1);
        (api.js_pop)(J, 1);

        (api.js_setglobal)(J, cstr!("thing82"));
        run(api, J, "s82p", "thing82.num + '/' + thing82.str + '/' + thing82.doit(5)");
        run(
            api,
            J,
            "s82q",
            "var ks = []; for (var k in thing82) ks.push(k); ks.length + ':' + ks.join(',')",
        );
        p_int("endtop", (api.js_gettop)(J));
        (api.js_freestate)(J);
    });
}

/* ------------------------------------------------------------------------- */
/* row 83 — js_newarguments / js_newfunction / js_newscript                  */
/* ------------------------------------------------------------------------- */

const JS_COBJECT: c_int = 0;

#[test]
fn cfg83_newarguments_newfunction_newscript() {
    diff("cfg83_newarguments", |api| unsafe {
        enter(api);
        let J = newstate(api, 0);
        (api.js_newarguments)(J);
        p_str("args.typeof", (api.js_typeof)(J, -1));
        p_int("args.type", (api.js_type)(J, -1));
        p_int("args.isobject", (api.js_isobject)(J, -1));
        p_int("args.isarray", (api.js_isarray)(J, -1));
        p_int("args.iscallable", (api.js_iscallable)(J, -1));
        p_repr_at(api, J, "args.repr", -1);
        p_str_at(api, J, "args.str", -1);
        dump_props(api, J, "args", 1);
        dump_props(api, J, "args", 0);
        p_int("args.getlength", (api.js_getlength)(J, -1));
        /* the Object prototype is reachable */
        (api.js_getproperty)(J, -1, cstr!("toString"));
        p_str("args.toString.typeof", (api.js_typeof)(J, -1));
        (api.js_pop)(J, 1);
        (api.js_getproperty)(J, -1, cstr!("hasOwnProperty"));
        p_str("args.hasOwnProperty.typeof", (api.js_typeof)(J, -1));
        (api.js_pop)(J, 1);

        /* fill it in the way jsR_callfunction does */
        (api.js_pushnumber)(J, 2.0);
        (api.js_defproperty)(J, -2, cstr!("length"), JS_DONTENUM);
        (api.js_pushstring)(J, cstr!("zero"));
        (api.js_setindex)(J, -2, 0);
        (api.js_pushstring)(J, cstr!("one"));
        (api.js_setindex)(J, -2, 1);
        dump_props(api, J, "args2", 1);
        p_int("args2.getlength", (api.js_getlength)(J, -1));
        (api.js_getindex)(J, -1, 0);
        show_at(api, J, "args2.0", -1);
        (api.js_pop)(J, 1);
        p_repr_at(api, J, "args2.repr", -1);

        (api.js_setglobal)(J, cstr!("a83"));
        run(api, J, "s83", "a83.length + ':' + a83[0] + ',' + a83[1]");
        p_int("top", (api.js_gettop)(J));
        (api.js_freestate)(J);
    });

    diff("cfg83_newfunction", |api| unsafe {
        enter(api);
        let J = newstate(api, 0);

        /* low level pipeline: parse -> compile -> freeparse */
        let file = cs("low83");
        let src = cs("var q = 41; q + 1");
        let ast = (api.jsP_parse)(J, file.as_ptr(), src.as_ptr());
        p_ptr_nonnull("ast", ast);
        let fun = (api.jsC_compilescript)(J, ast, 0);
        p_ptr_nonnull("fun", fun);
        (api.jsP_freeparse)(J);

        /* a scope whose variables object is a bare JS_COBJECT with no outer */
        let vars = (api.jsV_newobject)(J, JS_COBJECT, null_mut());
        p_ptr_nonnull("vars", vars);
        (api.js_pushobject)(J, vars); /* keep it rooted */
        let env = (api.jsR_newenvironment)(J, vars, null_mut());
        p_ptr_nonnull("env", env);

        p_line("-- js_newfunction --");
        (api.js_newfunction)(J, fun, env);
        p_str("f.typeof", (api.js_typeof)(J, -1));
        p_int("f.type", (api.js_type)(J, -1));
        p_int("f.iscallable", (api.js_iscallable)(J, -1));
        p_repr_at(api, J, "f.repr", -1);
        p_str_at(api, J, "f.str", -1);
        (api.js_getproperty)(J, -1, cstr!("length"));
        p_num("f.length", (api.js_trynumber)(J, -1, -1.0));
        (api.js_pop)(J, 1);
        (api.js_getproperty)(J, -1, cstr!("prototype"));
        p_str("f.prototype.typeof", (api.js_typeof)(J, -1));
        (api.js_getproperty)(J, -1, cstr!("constructor"));
        p_int("f.prototype.constructor.iscallable", (api.js_iscallable)(J, -1));
        (api.js_pop)(J, 2);
        dump_props(api, J, "f", 1);

        /* call it */
        (api.js_copy)(J, -1);
        (api.js_pushundefined)(J);
        let rc = (api.js_pcall)(J, 0);
        p_int("f.call.rc", rc);
        show(api, J, "f.call.res");
        (api.js_pop)(J, 1);

        /* call it again: a fresh environment is created per call, so the
         * result must be identical */
        (api.js_copy)(J, -1);
        (api.js_pushundefined)(J);
        let rc = (api.js_pcall)(J, 0);
        p_int("f.call2.rc", rc);
        show(api, J, "f.call2.res");
        (api.js_pop)(J, 1);

        /* the variables object of the scope is untouched by the call */
        (api.js_copy)(J, -2);
        dump_props(api, J, "vars.after", 1);
        (api.js_pop)(J, 1);

        p_line("-- js_newscript --");
        let file = cs("low83s");
        let src2 = cs("var s = 7; s * 6");
        let ast2 = (api.jsP_parse)(J, file.as_ptr(), src2.as_ptr());
        let fun2 = (api.jsC_compilescript)(J, ast2, 0);
        (api.jsP_freeparse)(J);
        (api.js_newscript)(J, fun2, env);
        p_str("sc.typeof", (api.js_typeof)(J, -1));
        p_int("sc.type", (api.js_type)(J, -1));
        p_int("sc.iscallable", (api.js_iscallable)(J, -1));
        p_repr_at(api, J, "sc.repr", -1);
        dump_props(api, J, "sc", 1);
        (api.js_copy)(J, -1);
        (api.js_pushundefined)(J);
        let rc = (api.js_pcall)(J, 0);
        p_int("sc.call.rc", rc);
        show(api, J, "sc.call.res");
        (api.js_pop)(J, 1);

        /* a script writes its vars straight into the scope's variables object */
        (api.js_copy)(J, -3);
        dump_props(api, J, "vars.afterscript", 1);
        (api.js_getproperty)(J, -1, cstr!("s"));
        show_at(api, J, "vars.s", -1);
        (api.js_pop)(J, 2);

        /* js_newscript with a NULL scope uses the current environment */
        let file = cs("low83n");
        let src3 = cs("var nullscope83 = 5; nullscope83 + 1");
        let ast3 = (api.jsP_parse)(J, file.as_ptr(), src3.as_ptr());
        let fun3 = (api.jsC_compilescript)(J, ast3, 0);
        (api.jsP_freeparse)(J);
        (api.js_newscript)(J, fun3, null_mut());
        (api.js_pushundefined)(J);
        let rc = (api.js_pcall)(J, 0);
        p_int("sc2.call.rc", rc);
        show(api, J, "sc2.call.res");
        (api.js_pop)(J, 1);
        (api.js_getglobal)(J, cstr!("nullscope83"));
        show_at(api, J, "global.nullscope83", -1);
        (api.js_pop)(J, 1);

        (api.js_gc)(J, 1);
        p_int("top", (api.js_gettop)(J));
        (api.js_pop)(J, (api.js_gettop)(J));
        (api.js_freestate)(J);
    });
}

/* ------------------------------------------------------------------------- */
/* row 84 — jsR_newenvironment                                               */
/* ------------------------------------------------------------------------- */

#[test]
fn cfg84_newenvironment() {
    diff("cfg84_newenvironment", |api| unsafe {
        enter(api);
        let J = newstate(api, 0);

        /* outer scope: aa = 10, cc = 'outer' */
        let v1 = (api.jsV_newobject)(J, JS_COBJECT, null_mut());
        (api.js_pushobject)(J, v1);
        (api.js_pushnumber)(J, 10.0);
        (api.js_setproperty)(J, -2, cstr!("aa"));
        (api.js_pushstring)(J, cstr!("outer"));
        (api.js_setproperty)(J, -2, cstr!("cc"));
        let e1 = (api.jsR_newenvironment)(J, v1, null_mut());
        p_ptr_nonnull("e1", e1);
        dump_props(api, J, "v1", 1);

        /* inner scope: bb = 32, cc = 'inner' (shadows the outer cc) */
        let v2 = (api.jsV_newobject)(J, JS_COBJECT, null_mut());
        (api.js_pushobject)(J, v2);
        (api.js_pushnumber)(J, 32.0);
        (api.js_setproperty)(J, -2, cstr!("bb"));
        (api.js_pushstring)(J, cstr!("inner"));
        (api.js_setproperty)(J, -2, cstr!("cc"));
        let e2 = (api.jsR_newenvironment)(J, v2, e1);
        p_ptr_nonnull("e2", e2);
        dump_props(api, J, "v2", 1);

        /* a function compiled against the chained scope */
        let file = cs("env84");
        let src = cs("aa + bb");
        let ast = (api.jsP_parse)(J, file.as_ptr(), src.as_ptr());
        let fun = (api.jsC_compilescript)(J, ast, 0);
        (api.jsP_freeparse)(J);
        (api.js_newfunction)(J, fun, e2);
        (api.js_copy)(J, -1);
        (api.js_pushundefined)(J);
        let rc = (api.js_pcall)(J, 0);
        p_int("sum.rc", rc);
        show(api, J, "sum.res");
        (api.js_pop)(J, 1);

        /* shadowing: the inner cc must win */
        let file = cs("env84b");
        let src = cs("cc");
        let ast = (api.jsP_parse)(J, file.as_ptr(), src.as_ptr());
        let fun2 = (api.jsC_compilescript)(J, ast, 0);
        (api.jsP_freeparse)(J);
        (api.js_newfunction)(J, fun2, e2);
        (api.js_copy)(J, -1);
        (api.js_pushundefined)(J);
        let rc = (api.js_pcall)(J, 0);
        p_int("shadow.rc", rc);
        show(api, J, "shadow.res");
        (api.js_pop)(J, 1);

        /* only the outer scope: a function bound to e1 cannot see bb */
        let file = cs("env84c");
        let src = cs("aa + cc");
        let ast = (api.jsP_parse)(J, file.as_ptr(), src.as_ptr());
        let fun3 = (api.jsC_compilescript)(J, ast, 0);
        (api.jsP_freeparse)(J);
        (api.js_newfunction)(J, fun3, e1);
        (api.js_copy)(J, -1);
        (api.js_pushundefined)(J);
        let rc = (api.js_pcall)(J, 0);
        p_int("outeronly.rc", rc);
        show(api, J, "outeronly.res");
        (api.js_pop)(J, 1);

        let file = cs("env84d");
        let src = cs("bb");
        let ast = (api.jsP_parse)(J, file.as_ptr(), src.as_ptr());
        let fun4 = (api.jsC_compilescript)(J, ast, 0);
        (api.jsP_freeparse)(J);
        (api.js_newfunction)(J, fun4, e1);
        (api.js_copy)(J, -1);
        (api.js_pushundefined)(J);
        let rc = (api.js_pcall)(J, 0);
        p_int("missing.rc", rc);
        show(api, J, "missing.res");
        (api.js_pop)(J, 1);

        /* a name that is nowhere in the chain: ReferenceError, because
         * js_hasvar walks the environment chain only */
        let file = cs("env84e");
        let src = cs("zz84");
        let ast = (api.jsP_parse)(J, file.as_ptr(), src.as_ptr());
        let fun5 = (api.jsC_compilescript)(J, ast, 0);
        (api.jsP_freeparse)(J);
        (api.js_newfunction)(J, fun5, e2);
        (api.js_copy)(J, -1);
        (api.js_pushundefined)(J);
        let rc = (api.js_pcall)(J, 0);
        p_int("unknown.rc", rc);
        show(api, J, "unknown.res");
        (api.js_pop)(J, 1);

        /* assignment goes to the environment that owns the name */
        let file = cs("env84f");
        let src = cs("bb = bb + 1; aa = aa + 100; bb + '/' + aa");
        let ast = (api.jsP_parse)(J, file.as_ptr(), src.as_ptr());
        let fun6 = (api.jsC_compilescript)(J, ast, 0);
        (api.jsP_freeparse)(J);
        (api.js_newfunction)(J, fun6, e2);
        (api.js_copy)(J, -1);
        (api.js_pushundefined)(J);
        let rc = (api.js_pcall)(J, 0);
        p_int("assign.rc", rc);
        show(api, J, "assign.res");
        (api.js_pop)(J, 1);

        /* read the variables objects back */
        (api.js_pushobject)(J, v1);
        dump_props(api, J, "v1.after", 1);
        (api.js_getproperty)(J, -1, cstr!("aa"));
        show_at(api, J, "v1.aa", -1);
        (api.js_pop)(J, 2);
        (api.js_pushobject)(J, v2);
        dump_props(api, J, "v2.after", 1);
        (api.js_getproperty)(J, -1, cstr!("bb"));
        show_at(api, J, "v2.bb", -1);
        (api.js_pop)(J, 2);

        /* a var declaration inside a js_newscript body lands in the scope's
         * own variables object */
        let file = cs("env84g");
        let src = cs("var newvar84 = 'v'; newvar84 + cc");
        let ast = (api.jsP_parse)(J, file.as_ptr(), src.as_ptr());
        let fun7 = (api.jsC_compilescript)(J, ast, 0);
        (api.jsP_freeparse)(J);
        (api.js_newscript)(J, fun7, e2);
        (api.js_pushundefined)(J);
        let rc = (api.js_pcall)(J, 0);
        p_int("script.rc", rc);
        show(api, J, "script.res");
        (api.js_pop)(J, 1);
        (api.js_pushobject)(J, v2);
        dump_props(api, J, "v2.afterscript", 1);
        /* js_initvar uses JS_DONTENUM, so the iterator cannot see it */
        let rc = (api.js_hasproperty)(J, -1, cstr!("newvar84"));
        p_int("v2.newvar84.has", rc);
        if rc != 0 {
            (api.js_pop)(J, 1);
        }
        (api.js_getproperty)(J, -1, cstr!("newvar84"));
        show_at(api, J, "v2.newvar84", -1);
        (api.js_pop)(J, 2);

        /* an environment with the global object as its variables object */
        (api.js_pushglobal)(J);
        let g = (api.js_toobject)(J, -1);
        (api.js_pop)(J, 1);
        let e3 = (api.jsR_newenvironment)(J, g, null_mut());
        p_ptr_nonnull("e3", e3);
        (api.js_pushnumber)(J, 5.0);
        (api.js_setglobal)(J, cstr!("gv84"));
        let file = cs("env84h");
        let src = cs("gv84 * 2");
        let ast = (api.jsP_parse)(J, file.as_ptr(), src.as_ptr());
        let fun8 = (api.jsC_compilescript)(J, ast, 0);
        (api.jsP_freeparse)(J);
        (api.js_newfunction)(J, fun8, e3);
        (api.js_pushundefined)(J);
        let rc = (api.js_pcall)(J, 0);
        p_int("globalenv.rc", rc);
        show(api, J, "globalenv.res");
        (api.js_pop)(J, 1);

        (api.js_gc)(J, 1);
        p_int("top", (api.js_gettop)(J));
        (api.js_pop)(J, (api.js_gettop)(J));
        (api.js_freestate)(J);
    });
}
