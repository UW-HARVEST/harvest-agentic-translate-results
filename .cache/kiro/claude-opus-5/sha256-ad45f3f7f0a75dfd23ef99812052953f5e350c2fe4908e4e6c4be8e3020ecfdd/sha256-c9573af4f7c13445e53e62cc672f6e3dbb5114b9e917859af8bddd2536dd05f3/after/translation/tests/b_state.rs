//! Phase B rows 35-40, 77-80: state construction options, limits, GC,
//! report/panic hooks and nested try frames.
mod common;
use common::*;
use std::os::raw::{c_char, c_int, c_void};

unsafe extern "C" {
    fn malloc(n: usize) -> *mut c_void;
    fn realloc(p: *mut c_void, n: usize) -> *mut c_void;
    fn free(p: *mut c_void);
}

#[test]
fn row35_newstate_default() {
    let p = pair();
    for _ in 0..20 {
        let jc = unsafe { (p.c.js_newstate)(None, std::ptr::null_mut(), 0) };
        let jr = unsafe { (p.r.js_newstate)(None, std::ptr::null_mut(), 0) };
        assert!(!jc.is_null() && !jr.is_null());
        assert_eq!(unsafe { (p.c.js_gettop)(jc) }, unsafe {
            (p.r.js_gettop)(jr)
        });
        unsafe { (p.c.js_freestate)(jc) };
        unsafe { (p.r.js_freestate)(jr) };
    }
    // context round-trip
    let jc = unsafe { (p.c.js_newstate)(None, std::ptr::null_mut(), 0) };
    let jr = unsafe { (p.r.js_newstate)(None, std::ptr::null_mut(), 0) };
    let tag = 0x1234usize as *mut c_void;
    unsafe { (p.c.js_setcontext)(jc, tag) };
    unsafe { (p.r.js_setcontext)(jr, tag) };
    assert_eq!(unsafe { (p.c.js_getcontext)(jc) }, unsafe {
        (p.r.js_getcontext)(jr)
    });
    unsafe { (p.c.js_freestate)(jc) };
    unsafe { (p.r.js_freestate)(jr) };
}

#[test]
fn row36_newstate_strict() {
    // JS_STRICT makes global code strict: octal literals, `with`, undeclared
    // assignment, delete of a variable, duplicate params ... all change.
    let strict_sensitive = [
        "x = 1; x",
        "var o = {}; with (o) { 1 }",
        "010",
        "var x; delete x",
        "function f(a,a){return a} f(1,2)",
        "'use strict'; x = 1",
        "(function(){ return this })()",
        "(function(){ 'use strict'; return this })()",
        "var o = {get a(){return 1}}; o.a = 2; o.a",
        "Object.defineProperty({}, 'a', {value:1, writable:false}).a = 2",
        "arguments",
        "function f(){ return arguments.callee } f()",
        "eval('var y = 3'); typeof y",
        "typeof undeclaredThing",
    ];
    for s in strict_sensitive {
        diff_eval(s, 0);
        diff_eval(s, JS_STRICT);
    }
}

/* --- custom allocator --- */

#[repr(C)]
struct Ctx {
    live: i64,
    calls: i64,
}

unsafe extern "C-unwind" fn counting_alloc(ctx: *mut c_void, ptr: *mut c_void, n: c_int) -> *mut c_void {
    unsafe {
        let c = ctx as *mut Ctx;
        if !c.is_null() {
            (*c).calls += 1;
        }
        if n == 0 {
            if !c.is_null() {
                (*c).live -= 1;
            }
            free(ptr);
            return std::ptr::null_mut();
        }
        if ptr.is_null() && !c.is_null() {
            (*c).live += 1;
        }
        realloc(ptr, n as usize)
    }
}

#[test]
fn row37_newstate_custom_allocator() {
    let p = pair();
    let mut cc = Ctx { live: 0, calls: 0 };
    let mut cr = Ctx { live: 0, calls: 0 };
    let jc = unsafe {
        (p.c.js_newstate)(
            Some(counting_alloc),
            &mut cc as *mut Ctx as *mut c_void,
            0,
        )
    };
    let jr = unsafe {
        (p.r.js_newstate)(
            Some(counting_alloc),
            &mut cr as *mut Ctx as *mut c_void,
            0,
        )
    };
    assert!(!jc.is_null() && !jr.is_null());
    for src in [
        "1+1",
        "var a = []; for (var i=0;i<200;++i) a.push('x'+i); a.length",
        "JSON.stringify({a:[1,2,3]})",
    ] {
        let fname = cs("[string]");
        let csrc = cs(src);
        let ra = unsafe { (p.c.js_ploadstring)(jc, fname.as_ptr(), csrc.as_ptr()) };
        let rb = unsafe { (p.r.js_ploadstring)(jr, fname.as_ptr(), csrc.as_ptr()) };
        assert_eq!(ra, rb, "ploadstring {src:?}");
        unsafe { (p.c.js_pushundefined)(jc) };
        unsafe { (p.r.js_pushundefined)(jr) };
        let ca = unsafe { (p.c.js_pcall)(jc, 0) };
        let cb = unsafe { (p.r.js_pcall)(jr, 0) };
        assert_eq!(ca, cb, "pcall {src:?}");
        let fb = cs("<err>");
        let sa = unsafe { rstr((p.c.js_trystring)(jc, -1, fb.as_ptr())) };
        let sb = unsafe { rstr((p.r.js_trystring)(jr, -1, fb.as_ptr())) };
        assert_eq!(sa, sb, "result {src:?}");
        unsafe { (p.c.js_pop)(jc, 1) };
        unsafe { (p.r.js_pop)(jr, 1) };
    }
    unsafe { (p.c.js_freestate)(jc) };
    unsafe { (p.r.js_freestate)(jr) };
    assert!(cc.calls > 0 && cr.calls > 0, "allocator must be used");
}

/// An allocator that fails after N successful allocations, so `js_newstate`'s
/// out-of-memory paths (return NULL) are taken identically.
static mut FAIL_AFTER_C: i64 = 0;
static mut FAIL_AFTER_R: i64 = 0;

unsafe extern "C-unwind" fn failing_alloc_c(_c: *mut c_void, ptr: *mut c_void, n: c_int) -> *mut c_void {
    unsafe {
        if n == 0 {
            free(ptr);
            return std::ptr::null_mut();
        }
        if FAIL_AFTER_C <= 0 {
            return std::ptr::null_mut();
        }
        FAIL_AFTER_C -= 1;
        realloc(ptr, n as usize)
    }
}

unsafe extern "C-unwind" fn failing_alloc_r(_c: *mut c_void, ptr: *mut c_void, n: c_int) -> *mut c_void {
    unsafe {
        if n == 0 {
            free(ptr);
            return std::ptr::null_mut();
        }
        if FAIL_AFTER_R <= 0 {
            return std::ptr::null_mut();
        }
        FAIL_AFTER_R -= 1;
        realloc(ptr, n as usize)
    }
}

#[test]
fn row37b_newstate_allocation_failure() {
    let p = pair();
    for budget in [0i64, 1, 2] {
        unsafe { FAIL_AFTER_C = budget };
        unsafe { FAIL_AFTER_R = budget };
        let jc = unsafe {
            (p.c.js_newstate)(Some(failing_alloc_c), std::ptr::null_mut(), 0)
        };
        let jr = unsafe {
            (p.r.js_newstate)(Some(failing_alloc_r), std::ptr::null_mut(), 0)
        };
        assert_eq!(
            jc.is_null(),
            jr.is_null(),
            "js_newstate NULL-return parity with budget {budget}"
        );
        if !jc.is_null() {
            unsafe { FAIL_AFTER_C = i64::MAX };
            unsafe { (p.c.js_freestate)(jc) };
        }
        if !jr.is_null() {
            unsafe { FAIL_AFTER_R = i64::MAX };
            unsafe { (p.r.js_freestate)(jr) };
        }
    }
}

/* --- limits --- */

unsafe fn eval_with_limits(
    api: &Api,
    src: &str,
    runlimit: c_int,
    memlimit: c_int,
) -> (c_int, String) {
    unsafe {
        let j = (api.js_newstate)(None, std::ptr::null_mut(), 0);
        assert!(!j.is_null());
        (api.js_setreport)(j, Some(report_cb));
        (api.js_setlimit)(j, runlimit, memlimit);
        let fname = cs("[string]");
        let csrc = cs(src);
        let mut rc = (api.js_ploadstring)(j, fname.as_ptr(), csrc.as_ptr());
        if rc != 0 {
            rc = 1;
        } else {
            (api.js_pushundefined)(j);
            if (api.js_pcall)(j, 0) != 0 {
                rc = 2;
            }
        }
        let fb = cs("<err>");
        let s = rstr((api.js_trystring)(j, -1, fb.as_ptr()));
        (api.js_freestate)(j);
        (rc, s)
    }
}

#[test]
fn row38_runlimit() {
    let p = pair();
    let srcs = [
        "1+1",
        "var i=0; while(i<1000) ++i; i",
        "var i=0; for(;;) ++i;",
        "function f(){return f()} f()",
        "var a=[]; for (var i=0;i<100;++i) a[i]=i; a.length",
    ];
    for src in srcs {
        for limit in [1, 2, 3, 10, 100, 1000, 10000] {
            let a = unsafe { eval_with_limits(&p.c, src, limit, 0) };
            let b = unsafe { eval_with_limits(&p.r, src, limit, 0) };
            assert_eq!(a, b, "runlimit={limit} src={src:?}");
        }
    }
}

#[test]
fn row39_memlimit() {
    let p = pair();
    let srcs = [
        "1+1",
        "var a=[]; for (var i=0;i<5000;++i) a.push({x:i}); a.length",
        "var s=''; for (var i=0;i<2000;++i) s += 'abcdefgh'; s.length",
        "new Array(100000).join('x').length",
    ];
    for src in srcs {
        for limit in [1, 64, 4096, 65536, 1 << 20] {
            let a = unsafe { eval_with_limits(&p.c, src, 0, limit) };
            let b = unsafe { eval_with_limits(&p.r, src, 0, limit) };
            assert_eq!(a, b, "memlimit={limit} src={src:?}");
        }
    }
}

#[test]
fn row40_gc() {
    let p = pair();
    // GC with report=0 must not change observable results.
    let srcs = [
        "var a=[]; for (var i=0;i<500;++i) a.push({x:i}); a=null; 'done'",
        "(function(){ var o={}; for (var i=0;i<100;++i) o['k'+i]={}; return 1 })()",
        "var r=/a(b)c/g; r.exec('abc'); 'ok'",
    ];
    for src in srcs {
        for gcs in [0, 1, 3] {
            let mut outs = Vec::new();
            for api in [&p.c, &p.r] {
                unsafe {
                    let j = (api.js_newstate)(None, std::ptr::null_mut(), 0);
                    (api.js_setreport)(j, Some(report_cb));
                    let fname = cs("[string]");
                    let csrc = cs(src);
                    let mut rc = (api.js_ploadstring)(j, fname.as_ptr(), csrc.as_ptr());
                    if rc == 0 {
                        (api.js_pushundefined)(j);
                        if (api.js_pcall)(j, 0) != 0 {
                            rc = 2;
                        }
                    } else {
                        rc = 1;
                    }
                    for _ in 0..gcs {
                        (api.js_gc)(j, 0);
                    }
                    let fb = cs("<err>");
                    let s = rstr((api.js_trystring)(j, -1, fb.as_ptr()));
                    let top = (api.js_gettop)(j);
                    (api.js_freestate)(j);
                    outs.push((rc, s, top));
                }
            }
            assert_eq!(outs[0], outs[1], "gc x{gcs} src={src:?}");
        }
    }
}

#[test]
fn row77_report_hook() {
    let p = pair();
    // js_dostring routes uncaught errors through the report callback.
    let srcs = [
        "throw new Error('boom')",
        "throw 'plain string'",
        "throw 42",
        "throw null",
        "throw undefined",
        "throw {toString:function(){return 'weird'}}",
        "throw {toString:function(){throw 'nested'}}",
        "undefinedFunction()",
        "var x = ;",
        "null.x",
        "(void 0).x = 1",
        "1()",
        "new 1",
    ];
    for src in srcs {
        let mut outs = Vec::new();
        for api in [&p.c, &p.r] {
            let _ = take_reports();
            unsafe {
                let j = (api.js_newstate)(None, std::ptr::null_mut(), 0);
                (api.js_setreport)(j, Some(report_cb));
                let csrc = cs(src);
                let rc = (api.js_dostring)(j, csrc.as_ptr());
                let top = (api.js_gettop)(j);
                (api.js_freestate)(j);
                outs.push((rc, take_reports(), top));
            }
        }
        assert_eq!(outs[0], outs[1], "report hook for {src:?}");
    }
}

#[test]
fn row77b_default_report_and_atpanic() {
    let p = pair();
    // Without js_setreport the default report writes to stderr; check the
    // return code and stack state still agree.
    for src in ["throw new TypeError('x')", "1+1", "syntax ~ error"] {
        let mut outs = Vec::new();
        for api in [&p.c, &p.r] {
            unsafe {
                let j = (api.js_newstate)(None, std::ptr::null_mut(), 0);
                let csrc = cs(src);
                let rc = (api.js_dostring)(j, csrc.as_ptr());
                let top = (api.js_gettop)(j);
                (api.js_freestate)(j);
                outs.push((rc, top));
            }
        }
        assert_eq!(outs[0], outs[1], "default report for {src:?}");
    }
    // js_report can be called directly.
    let mut outs = Vec::new();
    for api in [&p.c, &p.r] {
        let _ = take_reports();
        unsafe {
            let j = (api.js_newstate)(None, std::ptr::null_mut(), 0);
            (api.js_setreport)(j, Some(report_cb));
            for m in ["", "hello", "with %s format", "\u{e9}"] {
                let cm = cs(m);
                (api.js_report)(j, cm.as_ptr());
            }
            (api.js_freestate)(j);
            outs.push(take_reports());
        }
    }
    assert_eq!(outs[0], outs[1], "js_report passthrough");
}

#[test]
fn row80_nested_try_depth() {
    // JS_TRYLIMIT is 64; deep try nesting must overflow at the same depth.
    for depth in [1usize, 2, 8, 32, 60, 63, 64, 65, 70, 200] {
        let mut src = String::new();
        for _ in 0..depth {
            src.push_str("try{");
        }
        src.push_str("throw 1");
        for i in 0..depth {
            src.push_str(&format!("}}catch(e{i}){{throw e{i}+1}}"));
        }
        // wrap so the final throw is caught and observable
        let wrapped = format!("try{{{src}}}catch(e){{'caught:'+e}}");
        diff_eval_both_modes(&wrapped);
    }
    // deep recursion (environment stack) and deep function nesting
    diff_eval_both_modes("function f(n){ return n<=0 ? 0 : 1+f(n-1) } f(100)");
    diff_eval_both_modes("function f(n){ return n<=0 ? 0 : 1+f(n-1) } f(100000)");
    diff_eval_both_modes(
        "function f(n){ try { return n<=0 ? 0 : 1+f(n-1) } catch(e) { throw e } } f(200)",
    );
}

#[test]
fn row79_scopes_and_closures() {
    let srcs = [
        "var a=1; function f(){ var a=2; return function(){ return a } } f()()",
        "var o={x:5}; with(o){ (function(){ return x })() }",
        "(function(){ var r=[]; for (var i=0;i<3;++i) r.push(function(){return i}); return r[0]()+r[1]()+r[2]() })()",
        "try { throw 1 } catch(e) { var g = function(){ return e } } typeof e + ':' + g()",
        "function f(){ return typeof arguments } f()",
        "function f(a,b){ arguments[0]=9; return a } f(1,2)",
        "function f(){ function g(){ return 1 } return g() } f()",
        "var x = 1; (function(){ x = 2 })(); x",
        "(function(){ if (true) { function h(){return 'h'} } return h() })()",
        "with({a:1}){ with({a:2}){ a } }",
        "label: for(var i=0;i<3;++i){ for(var j=0;j<3;++j){ if(j==1) continue label; } } i+','+j",
        "outer: { break outer; } 'after'",
        "var s=''; for (var k in {a:1,b:2}) s+=k; s",
        "var s=''; for (var k in [10,20]) s+=k+':'+[10,20][k]+';'; s",
        "(function(){ return eval('1+1') })()",
        "var e=eval; e('2+2')",
        "(function(){ var x=1; return eval('x+1') })()",
        "new Function('a','b','return a+b')(3,4)",
        "Function('return 7')()",
    ];
    for s in srcs {
        diff_eval_both_modes(s);
    }
}
