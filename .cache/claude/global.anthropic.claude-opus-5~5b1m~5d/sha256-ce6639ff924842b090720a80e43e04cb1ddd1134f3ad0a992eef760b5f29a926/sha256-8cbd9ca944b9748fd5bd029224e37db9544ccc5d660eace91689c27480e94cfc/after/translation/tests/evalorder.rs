//! Argument-evaluation-order probes.
//!
//! Several public entry points in `c_src/src/jsrun.c` pass `js_toobject(J, idx)`
//! — which BOXES a primitive stack slot in place — together with other
//! arguments that observe that same slot (`!js_isobject(J, idx)`,
//! `stackidx(J, -1)`, `jsR_tofunction(J, -1)`). The C compiler's evaluation
//! order is therefore observable, and the Rust translation must reproduce it.

mod common;
use common::*;
use std::ffi::{c_char, c_int, c_void};
use std::ptr::null_mut;

extern "C" {
    fn setvbuf(f: *mut c_void, buf: *mut c_char, mode: c_int, size: usize) -> c_int;
    static mut stdout: *mut c_void;
}

static mut CUR: *const Api = std::ptr::null();

unsafe fn unbuffered() {
    setvbuf(stdout, null_mut(), 2 /* _IONBF */, 0);
}

/* set a property on a primitive `this` (transient object) */
unsafe extern "C" fn cf_setprop(J: JS) {
    let api = &*CUR;
    (api.js_pushstring)(J, cs("abc").as_ptr());
    (api.js_pushnumber)(J, 1.0);
    (api.js_setproperty)(J, -2, cs("zzz").as_ptr());
    p_int("after_setproperty_top", (api.js_gettop)(J));
    (api.js_pushundefined)(J);
}

unsafe extern "C" fn cf_setindex(J: JS) {
    let api = &*CUR;
    (api.js_pushstring)(J, cs("abc").as_ptr());
    (api.js_pushnumber)(J, 7.0);
    (api.js_setindex)(J, -2, 0);
    p_int("after_setindex_top", (api.js_gettop)(J));
    (api.js_pushundefined)(J);
}

fn run_cfun(tag: &str, f: unsafe extern "C" fn(JS), flags: c_int) {
    diff(tag, move |api| unsafe {
        unbuffered();
        CUR = api as *const Api;
        let J = newstate(api, flags);
        (api.js_newcfunction)(J, Some(f), cs("probe").as_ptr(), 0);
        (api.js_setglobal)(J, cs("probe").as_ptr());
        let rc = (api.js_dostring)(J, cs("try { probe() } catch (e) { }").as_ptr());
        p_int("rc_caught", rc);
        let rc2 = (api.js_dostring)(J, cs("probe()").as_ptr());
        p_int("rc_raw", rc2);
        (api.js_freestate)(J);
    });
}

#[test]
fn evalorder_setproperty_transient() {
    run_cfun("evalorder_setproperty_nonstrict", cf_setprop, 0);
    run_cfun("evalorder_setproperty_strict", cf_setprop, JS_STRICT);
}

#[test]
fn evalorder_setindex_transient() {
    run_cfun("evalorder_setindex_nonstrict", cf_setindex, 0);
    run_cfun("evalorder_setindex_strict", cf_setindex, JS_STRICT);
}

/// `js_defproperty(J, idx, name, atts)` → `jsR_defproperty(J, js_toobject(J,
/// idx), name, atts, stackidx(J, -1), ...)`: when `idx` designates the same
/// slot as -1, the order in which the slot is boxed and captured is observable.
#[test]
fn evalorder_defproperty_same_slot() {
    diff("evalorder_defproperty", |api| unsafe {
        unbuffered();
        let J = newstate(api, 0);
        /* idx == -1: the value being defined IS the target slot */
        (api.js_pushnumber)(J, 5.0);
        (api.js_defproperty)(J, -1, cs("p").as_ptr(), 0);
        p_int("top", (api.js_gettop)(J));
        p_str("typeof", (api.js_typeof)(J, -1));
        (api.js_freestate)(J);
    });
    diff("evalorder_defproperty_primitive", |api| unsafe {
        unbuffered();
        let J = newstate(api, 0);
        (api.js_pushnumber)(J, 42.0); /* primitive target */
        (api.js_pushnumber)(J, 5.0); /* value */
        (api.js_defproperty)(J, -2, cs("p").as_ptr(), 0);
        p_int("top", (api.js_gettop)(J));
        p_str("typeof", (api.js_typeof)(J, -1));
        (api.js_getproperty)(J, -1, cs("p").as_ptr());
        p_str("p", (api.js_tostring)(J, -1));
        (api.js_freestate)(J);
    });
}

/// `js_defaccessor` → `jsR_defproperty(J, js_toobject(J, idx), name, atts,
/// NULL, jsR_tofunction(J, -2), jsR_tofunction(J, -1), 1)`. Both
/// `jsR_tofunction` (throws "not a function") and `js_toobject` (throws for
/// undefined/null) can throw: which message wins reveals the order.
#[test]
fn evalorder_defaccessor_throw_order() {
    diff("evalorder_defaccessor_bad_getter_and_target", |api| unsafe {
        unbuffered();
        let J = newstate(api, 0);
        (api.js_pushundefined)(J); /* non-coercible target */
        (api.js_pushnumber)(J, 1.0); /* getter: not a function */
        (api.js_pushnumber)(J, 2.0); /* setter: not a function */
        let rc = (api.js_ploadstring)(J, cs("x").as_ptr(), cs("1").as_ptr());
        p_int("preload_rc", rc);
        (api.js_pop)(J, 1);
        /* this throws; both libraries must throw the SAME error */
        (api.js_defaccessor)(J, -3, cs("acc").as_ptr(), 0);
        p_line("no throw");
        (api.js_freestate)(J);
    });
    diff("evalorder_defaccessor_ok", |api| unsafe {
        unbuffered();
        let J = newstate(api, 0);
        (api.js_dostring)(J, cs("var g = function(){ return 11 }; var s = function(v){}").as_ptr());
        (api.js_newobject)(J);
        (api.js_getglobal)(J, cs("g").as_ptr());
        (api.js_getglobal)(J, cs("s").as_ptr());
        (api.js_defaccessor)(J, -3, cs("acc").as_ptr(), 0);
        p_int("top", (api.js_gettop)(J));
        (api.js_getproperty)(J, -1, cs("acc").as_ptr());
        p_str("acc", (api.js_tostring)(J, -1));
        (api.js_freestate)(J);
    });
}
