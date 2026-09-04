//! Phase C — error paths that ABORT, panic or otherwise terminate the process
//! (uncaught js_throw, stack under/overflow, unimplemented API, allocator
//! failure). Each case runs in a fresh child process for BOTH libraries and the
//! exit code, signal and output must match exactly.
#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::{c_char, c_int, c_void};

/* ---------------------------------------------------------------- cases */

fn c_throw_no_try(a: &Api, J: JS) {
    unsafe {
        (a.js_newerror)(J, cs("boom").as_ptr());
        (a.js_throw)(J);
    }
}

fn c_error_no_try(a: &Api, J: JS) {
    unsafe {
        /* js_pop underflow raises js_error with no enclosing try */
        (a.js_pop)(J, 100);
    }
}

fn c_pop_negative(a: &Api, J: JS) {
    unsafe {
        (a.js_pushnumber)(J, 1.0);
        (a.js_pop)(J, -5);
        println!("top={}", (a.js_gettop)(J));
    }
}

fn c_stack_overflow(a: &Api, J: JS) {
    unsafe {
        for i in 0..8000 {
            (a.js_pushnumber)(J, i as f64);
        }
        println!("pushed all");
    }
}

fn c_insert_notimpl(a: &Api, J: JS) {
    unsafe {
        (a.js_pushnumber)(J, 1.0);
        (a.js_pushnumber)(J, 2.0);
        (a.js_insert)(J, -2);
        println!("inserted");
    }
}

fn c_copy_oob(a: &Api, J: JS) {
    unsafe {
        (a.js_copy)(J, 1000);
        (a.js_copy)(J, -1000);
        println!("top={} undef={}", (a.js_gettop)(J), (a.js_isundefined)(J, -1));
    }
}

fn c_remove_oob(a: &Api, J: JS) {
    unsafe {
        (a.js_pushnumber)(J, 1.0);
        (a.js_remove)(J, 900);
        println!("removed top={}", (a.js_gettop)(J));
    }
}

fn c_replace_oob(a: &Api, J: JS) {
    unsafe {
        (a.js_pushnumber)(J, 1.0);
        (a.js_replace)(J, 900);
        println!("replaced top={}", (a.js_gettop)(J));
    }
}

fn c_rot_underflow(a: &Api, J: JS) {
    unsafe {
        (a.js_rot)(J, 5);
        println!("rot done top={}", (a.js_gettop)(J));
    }
}

fn c_endtry_underflow(a: &Api, J: JS) {
    unsafe {
        (a.js_endtry)(J);
        println!("endtry done");
    }
}

fn c_getglobal_uncaught(a: &Api, J: JS) {
    unsafe {
        /* js_getproperty on a primitive: throws with no try installed */
        (a.js_pushundefined)(J);
        (a.js_getproperty)(J, -1, cs("x").as_ptr());
        println!("got");
    }
}

fn c_gc_report_fresh(a: &Api, J: JS) {
    unsafe {
        (a.js_gc)(J, 1);
        println!("gc done");
    }
}

fn c_dostring_syntax_error(a: &Api, J: JS) {
    unsafe {
        let rc = (a.js_dostring)(J, cs("var 1").as_ptr());
        println!("dostring={}", rc);
        let rc = (a.js_dostring)(J, cs("throw 1").as_ptr());
        println!("dostring2={}", rc);
    }
}

fn c_report_default(a: &Api, J: JS) {
    unsafe {
        /* the default report handler writes to stderr */
        (a.js_report)(J, cs("a report message").as_ptr());
        (a.js_setreport)(J, None);
        (a.js_report)(J, cs("second message").as_ptr());
        println!("reported");
    }
}

/* allocator that always fails: js_newstate must handle it */
unsafe extern "C" fn null_alloc(_ctx: *mut c_void, _p: *mut c_void, _n: c_int) -> *mut c_void {
    std::ptr::null_mut()
}

/* allocator that only serves the first N requests */
static mut BUDGET: c_int = 3;
unsafe extern "C" fn budget_alloc(_ctx: *mut c_void, p: *mut c_void, n: c_int) -> *mut c_void {
    extern "C" {
        fn realloc(p: *mut c_void, n: usize) -> *mut c_void;
        fn free(p: *mut c_void);
    }
    if n == 0 {
        free(p);
        return std::ptr::null_mut();
    }
    if BUDGET <= 0 {
        return std::ptr::null_mut();
    }
    BUDGET -= 1;
    realloc(p, n as usize)
}

fn c_nostate_null_alloc(a: &Api, _J: JS) {
    unsafe {
        let J = (a.js_newstate)(Some(null_alloc), std::ptr::null_mut(), 0);
        println!("newstate_null_alloc_is_null={}", J.is_null());
    }
}

fn c_nostate_budget_alloc(a: &Api, _J: JS) {
    unsafe {
        BUDGET = 3;
        let J = (a.js_newstate)(Some(budget_alloc), std::ptr::null_mut(), 0);
        println!("newstate_budget_is_null={}", J.is_null());
    }
}

fn c_nostate_flags(a: &Api, _J: JS) {
    unsafe {
        for f in [0, 1, 2, -1, 0x7fffffff] {
            let J = (a.js_newstate)(None, std::ptr::null_mut(), f);
            println!("flags={} null={}", f, J.is_null());
            if !J.is_null() {
                (a.js_freestate)(J);
            }
        }
    }
}

fn c_memlimit_tiny(a: &Api, J: JS) {
    unsafe {
        (a.js_setlimit)(J, 0, 1);
        let rc = (a.js_dostring)(J, cs("var a = []; for (var i=0;i<1000;i++) a[i]=i;").as_ptr());
        println!("dostring={}", rc);
    }
}

fn c_runlimit_one(a: &Api, J: JS) {
    unsafe {
        (a.js_setlimit)(J, 1, 0);
        let rc = (a.js_dostring)(J, cs("var i=0; while(i<10) i++;").as_ptr());
        println!("dostring={}", rc);
    }
}

fn c_deep_recursion(a: &Api, J: JS) {
    unsafe {
        let rc = (a.js_dostring)(J, cs("function f(n){ return n<=0 ? 0 : 1+f(n-1) } f(2000)").as_ptr());
        println!("dostring={}", rc);
    }
}

fn c_deep_try(a: &Api, J: JS) {
    unsafe {
        let rc = (a.js_dostring)(
            J,
            cs("function f(n){ try { return n<=0 ? 0 : 1+f(n-1) } catch(e) { throw e } } f(200)")
                .as_ptr(),
        );
        println!("dostring={}", rc);
    }
}

fn c_array_limit(a: &Api, J: JS) {
    unsafe {
        let rc = (a.js_dostring)(J, cs("var a=[]; a.length = 1<<27; a.length").as_ptr());
        println!("dostring={}", rc);
        let rc = (a.js_dostring)(J, cs("var a=[]; a[1<<27]=1;").as_ptr());
        println!("dostring2={}", rc);
    }
}

fn c_uncaught_typeerror_in_ctor(a: &Api, J: JS) {
    unsafe {
        (a.js_pushnumber)(J, 1.0);
        (a.js_construct)(J, 0);
        println!("constructed");
    }
}

fn c_call_noncallable(a: &Api, J: JS) {
    unsafe {
        (a.js_pushnumber)(J, 1.0);
        (a.js_pushundefined)(J);
        (a.js_call)(J, 0);
        println!("called");
    }
}

fn c_concat_uncaught(a: &Api, J: JS) {
    unsafe {
        /* js_concat on values whose toString throws */
        let rc = (a.js_dostring)(
            J,
            cs("var o = { toString: function(){ throw new Error('ts') } }; o + ''").as_ptr(),
        );
        println!("dostring={}", rc);
    }
}

fn c_setlength_negative(a: &Api, J: JS) {
    unsafe {
        (a.js_newarray)(J);
        (a.js_setlength)(J, -1, -1);
        println!("len={}", (a.js_getlength)(J, -1));
    }
}

static CASES: &[(&str, fn(&Api, JS))] = &[
    ("throw_no_try", c_throw_no_try),
    ("error_no_try", c_error_no_try),
    ("pop_negative", c_pop_negative),
    ("stack_overflow", c_stack_overflow),
    ("insert_notimpl", c_insert_notimpl),
    ("copy_oob", c_copy_oob),
    ("remove_oob", c_remove_oob),
    ("replace_oob", c_replace_oob),
    ("rot_underflow", c_rot_underflow),
    ("endtry_underflow", c_endtry_underflow),
    ("getglobal_uncaught", c_getglobal_uncaught),
    ("gc_report_fresh", c_gc_report_fresh),
    ("dostring_syntax_error", c_dostring_syntax_error),
    ("report_default", c_report_default),
    ("nostate_null_alloc", c_nostate_null_alloc),
    ("nostate_budget_alloc", c_nostate_budget_alloc),
    ("nostate_flags", c_nostate_flags),
    ("memlimit_tiny", c_memlimit_tiny),
    ("runlimit_one", c_runlimit_one),
    ("deep_recursion", c_deep_recursion),
    ("deep_try", c_deep_try),
    ("array_limit", c_array_limit),
    ("uncaught_typeerror_in_ctor", c_uncaught_typeerror_in_ctor),
    ("call_noncallable", c_call_noncallable),
    ("concat_uncaught", c_concat_uncaught),
    ("setlength_negative", c_setlength_negative),
];

/* The child entry point: dispatches one case for one library, then exits. */
#[test]
fn isolated_child() {
    let _ = isolated_child_main(CASES);
}

#[test]
fn all_isolated_cases_match() {
    if std::env::var("MUJS_CASE").is_ok() {
        return; /* we are the child */
    }
    for (name, _) in CASES {
        for flags in [0, JS_STRICT] {
            diff_isolated(name, flags);
        }
    }
}

/* keep the c_char import used */
const _: *const c_char = std::ptr::null();
