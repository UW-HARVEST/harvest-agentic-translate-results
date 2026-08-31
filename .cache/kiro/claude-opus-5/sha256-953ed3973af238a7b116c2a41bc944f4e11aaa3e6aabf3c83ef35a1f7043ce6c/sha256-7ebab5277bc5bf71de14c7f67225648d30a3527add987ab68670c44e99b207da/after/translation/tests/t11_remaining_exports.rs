// Level 11: the remaining exported entry points -- builtin registration
// helpers, the error-raising family, try-frame bookkeeping, js_throw,
// js_toregexp / js_RegExp_prototype_exec, js_newfunction, js_pushobject.
mod common;

use common::*;
use libloading::Symbol;
use std::os::raw::{c_char, c_int, c_void};

fn lib_of(side: Side) -> &'static libloading::Library {
    let i = impls();
    match side {
        Side::C => &i.c,
        Side::Rust => &i.rust,
    }
}

macro_rules! sym {
    ($vm:expr, $name:literal, $ty:ty) => {{
        let s: Symbol<$ty> = unsafe { lib_of($vm.side).get(concat!($name, "\0").as_bytes()) }
            .unwrap_or_else(|e| panic!("missing {}: {}", $name, e));
        *s
    }};
}

// ---------------------------------------------------------------------------
// Error constructors and the noreturn error-raising family.
// ---------------------------------------------------------------------------

const NEWERRORS: &[&str] = &[
    "js_newerror",
    "js_newevalerror",
    "js_newrangeerror",
    "js_newreferenceerror",
    "js_newsyntaxerror",
    "js_newtypeerror",
    "js_newurierror",
];

const RAISERS: &[&str] = &[
    "js_error",
    "js_evalerror",
    "js_rangeerror",
    "js_referenceerror",
    "js_syntaxerror",
    "js_typeerror",
    "js_urierror",
];

#[test]
fn new_error_constructors_match() {
    let cs = Session::new(Side::C, 0);
    let rs = Session::new(Side::Rust, 0);
    for name in NEWERRORS {
        let n: &'static str = name;
        for msg in [
            &b"\0"[..],
            &b"plain\0"[..],
            &b"with %s format %d chars\0"[..],
            &b"unicode \xc3\xa9\xe4\xb8\xad\0"[..],
        ] {
            let m: &'static [u8] = msg;
            let f = move |vm: &Vm, j: JsPtr| {
                let cn = format!("{}\0", n);
                let s: Symbol<unsafe extern "C-unwind" fn(JsPtr, *const c_char)> =
                    unsafe { lib_of(vm.side).get(cn.as_bytes()) }.unwrap();
                unsafe { s(j, m.as_ptr() as *const c_char) };
                logln(format!("err={:?}", stack_snapshot(vm, j)));
                unsafe { (vm.getproperty)(j, -1, b"name\0".as_ptr() as *const c_char) };
                logln(format!("name={:?}", stack_snapshot(vm, j)));
                unsafe { (vm.pop)(j, 1) };
                unsafe { (vm.getproperty)(j, -1, b"message\0".as_ptr() as *const c_char) };
                logln(format!("message={:?}", stack_snapshot(vm, j)));
                unsafe { (vm.pop)(j, 1) };
                logln(format!("iserror={}", unsafe { (vm.iserror)(j, -1) }));
            };
            assert_same_protected(&cs, &rs, &format!("{} {:?}", n, msg), f);
        }
    }
}

#[test]
fn error_raisers_match() {
    let cs = Session::new(Side::C, 0);
    let rs = Session::new(Side::Rust, 0);
    for name in RAISERS {
        let n: &'static str = name;
        // plain message, and printf-style formats with each supported conversion
        let variants: Vec<(&'static [u8], i32)> = vec![
            (b"simple message\0", 0),
            (b"num=%d\0", 1),
            (b"str=%s\0", 2),
            (b"chr=%c\0", 3),
            (b"pct=100%%\0", 0),
            (b"two=%s/%d\0", 4),
        ];
        for (fmt, kind) in variants {
            let f = move |vm: &Vm, j: JsPtr| {
                let cn = format!("{}\0", n);
                let raw = unsafe { lib_of(vm.side).get::<*const c_void>(cn.as_bytes()) }.unwrap();
                let addr = *raw as usize;
                unsafe {
                    match kind {
                        0 => {
                            let g: unsafe extern "C-unwind" fn(JsPtr, *const c_char) =
                                std::mem::transmute(addr);
                            g(j, fmt.as_ptr() as *const c_char);
                        }
                        1 => {
                            let g: unsafe extern "C-unwind" fn(JsPtr, *const c_char, c_int) =
                                std::mem::transmute(addr);
                            g(j, fmt.as_ptr() as *const c_char, -42);
                        }
                        2 => {
                            let g: unsafe extern "C-unwind" fn(
                                JsPtr,
                                *const c_char,
                                *const c_char,
                            ) = std::mem::transmute(addr);
                            g(
                                j,
                                fmt.as_ptr() as *const c_char,
                                b"inserted\0".as_ptr() as *const c_char,
                            );
                        }
                        3 => {
                            let g: unsafe extern "C-unwind" fn(JsPtr, *const c_char, c_int) =
                                std::mem::transmute(addr);
                            g(j, fmt.as_ptr() as *const c_char, b'Z' as c_int);
                        }
                        _ => {
                            let g: unsafe extern "C-unwind" fn(
                                JsPtr,
                                *const c_char,
                                *const c_char,
                                c_int,
                            ) = std::mem::transmute(addr);
                            g(
                                j,
                                fmt.as_ptr() as *const c_char,
                                b"abc\0".as_ptr() as *const c_char,
                                7,
                            );
                        }
                    }
                }
                logln("unreachable".to_string());
            };
            assert_same_protected(&cs, &rs, &format!("{} {:?}", n, fmt), f);
        }
    }
}

#[test]
fn js_throw_matches() {
    let cs = Session::new(Side::C, 0);
    let rs = Session::new(Side::Rust, 0);
    let values: Vec<(&'static str, fn(&Vm, JsPtr))> = vec![
        ("number", |vm, j| unsafe { (vm.pushnumber)(j, 7.5) }),
        ("string", |vm, j| unsafe {
            (vm.pushliteral)(j, b"thrown\0".as_ptr() as *const c_char)
        }),
        ("undefined", |vm, j| unsafe { (vm.pushundefined)(j) }),
        ("null", |vm, j| unsafe { (vm.pushnull)(j) }),
        ("object", |vm, j| unsafe { (vm.newobject)(j) }),
        ("error", |vm, j| {
            let s = sym!(vm, "js_newerror", unsafe extern "C-unwind" fn(JsPtr, *const c_char));
            unsafe { s(j, b"thrown error\0".as_ptr() as *const c_char) };
        }),
    ];
    for (label, push) in values {
        let f = move |vm: &Vm, j: JsPtr| {
            let throw = sym!(vm, "js_throw", unsafe extern "C-unwind" fn(JsPtr) -> !);
            push(vm, j);
            unsafe { throw(j) };
        };
        assert_same_protected(&cs, &rs, &format!("js_throw {}", label), f);
    }
}

#[test]
fn try_frame_bookkeeping_matches() {
    let cs = Session::new(Side::C, 0);
    let rs = Session::new(Side::Rust, 0);
    // Balanced savetry/endtry pairs must be pure bookkeeping. We never longjmp
    // out of them, so this is safe on both sides.
    for depth in [1usize, 2, 8, 16, 32] {
        let f = move |vm: &Vm, j: JsPtr| {
            let savetry = sym!(vm, "js_savetry", unsafe extern "C-unwind" fn(JsPtr) -> *mut c_void);
            let endtry = sym!(vm, "js_endtry", unsafe extern "C-unwind" fn(JsPtr));
            for _ in 0..depth {
                let b = unsafe { savetry(j) };
                logln(format!("buf_null={}", b.is_null()));
            }
            for _ in 0..depth {
                unsafe { endtry(j) };
            }
            logln("balanced".to_string());
        };
        assert_same_protected(&cs, &rs, &format!("savetry/endtry depth {}", depth), f);

        let f = move |vm: &Vm, j: JsPtr| {
            let savetrypc = sym!(vm, "js_savetrypc", unsafe extern "C-unwind" fn(JsPtr, *mut c_void) -> *mut c_void);
            let endtry = sym!(vm, "js_endtry", unsafe extern "C-unwind" fn(JsPtr));
            for _ in 0..depth {
                let b = unsafe { savetrypc(j, std::ptr::null_mut()) };
                logln(format!("buf_null={}", b.is_null()));
            }
            for _ in 0..depth {
                unsafe { endtry(j) };
            }
        };
        assert_same_protected(&cs, &rs, &format!("savetrypc/endtry depth {}", depth), f);
    }

    // NOTE: the error paths of js_endtry (underflow) and js_savetry (try stack
    // overflow) both raise a JS error *after* the caller's try frames have been
    // consumed, so the resulting throw has nowhere valid to land. That is
    // undefined behaviour in the C original and cannot be compared; the limits
    // themselves are covered by deeply nested JS `try` statements in
    // t08_structure::recursion_and_limit_behaviour.
}

#[test]
fn toprimitive_and_toregexp_match() {
    let cs = Session::new(Side::C, 0);
    let rs = Session::new(Side::Rust, 0);
    let exprs = [
        "1", "'s'", "true", "null", "undefined", "{}", "[]", "[1]", "new Date(0)",
        "new Number(3)", "new String('x')", "({valueOf:function(){return 9}})",
        "({toString:function(){return 'ts'}})",
        "({valueOf:function(){return {}},toString:function(){return {}}})",
        "/re/g",
    ];
    for e in exprs {
        let setup = format!("var __v = ({});", e);
        assert_eq!(run_script(&cs, &setup), run_script(&rs, &setup));
        for hint in [0i32, 1, 2, 3, -1] {
            let f = move |vm: &Vm, j: JsPtr| {
                let tp = sym!(vm, "js_toprimitive", unsafe extern "C-unwind" fn(JsPtr, c_int, c_int));
                unsafe { (vm.getglobal)(j, b"__v\0".as_ptr() as *const c_char) };
                unsafe { tp(j, -1, hint) };
                logln(format!("prim={:?}", stack_snapshot(vm, j)));
            };
            assert_same_protected(&cs, &rs, &format!("js_toprimitive {} hint={}", e, hint), f);
        }
        let f = move |vm: &Vm, j: JsPtr| {
            let tr = sym!(vm, "js_toregexp", unsafe extern "C-unwind" fn(JsPtr, c_int) -> *mut c_void);
            unsafe { (vm.getglobal)(j, b"__v\0".as_ptr() as *const c_char) };
            let re = unsafe { tr(j, -1) };
            logln(format!("re_null={}", re.is_null()));
        };
        assert_same_protected(&cs, &rs, &format!("js_toregexp {}", e), f);
    }
}

#[test]
fn regexp_prototype_exec_export_matches() {
    let cs = Session::new(Side::C, 0);
    let rs = Session::new(Side::Rust, 0);
    let cases: &[(&str, &[u8])] = &[
        ("/a/", b"abc\0"),
        ("/a/g", b"aaa\0"),
        ("/(a)(b)/", b"xabz\0"),
        ("/z/", b"abc\0"),
        ("/^a/m", b"b\na\0"),
        ("/a/i", b"ABC\0"),
        ("/(a)|(b)/", b"b\0"),
        ("/\\d+/", b"x123y\0"),
        ("/(?:)/", b"\0"),
        ("/a*/", b"\0"),
    ];
    for (re, text) in cases {
        let setup = format!("var __r = {};", re);
        assert_eq!(run_script(&cs, &setup), run_script(&rs, &setup));
        let t: &'static [u8] = text;
        let f = move |vm: &Vm, j: JsPtr| {
            let tr = sym!(vm, "js_toregexp", unsafe extern "C-unwind" fn(JsPtr, c_int) -> *mut c_void);
            let exec = sym!(vm, "js_RegExp_prototype_exec", unsafe extern "C-unwind" fn(JsPtr, *mut c_void, *const c_char));
            unsafe { (vm.getglobal)(j, b"__r\0".as_ptr() as *const c_char) };
            let r = unsafe { tr(j, -1) };
            for round in 0..3 {
                unsafe { exec(j, r, t.as_ptr() as *const c_char) };
                logln(format!("round{}={:?}", round, stack_snapshot(vm, j)));
                unsafe { (vm.pop)(j, 1) };
            }
        };
        assert_same_protected(&cs, &rs, &format!("RegExp_prototype_exec {} {:?}", re, text), f);
    }
}

#[test]
fn loadstring_and_pushobject_match() {
    let cs = Session::new(Side::C, 0);
    let rs = Session::new(Side::Rust, 0);
    for src in ["1+1", "var q=1; q", "(", "throw 1", "function f(){return 3} f()"] {
        let owned: &'static str = Box::leak(format!("{}\0", src).into_boxed_str());
        let f = move |vm: &Vm, j: JsPtr| {
            let ls = sym!(vm, "js_loadstring", unsafe extern "C-unwind" fn(JsPtr, *const c_char, *const c_char));
            unsafe {
                ls(
                    j,
                    b"load.js\0".as_ptr() as *const c_char,
                    owned.as_ptr() as *const c_char,
                )
            };
            logln(format!("loaded={:?}", stack_snapshot(vm, j)));
            unsafe { (vm.pushundefined)(j) };
            let e = unsafe { (vm.pcall)(j, 0) };
            logln(format!("err={} res={:?}", e, stack_snapshot(vm, j)));
        };
        assert_same_protected(&cs, &rs, &format!("js_loadstring {:?}", src), f);
    }

    let f = |vm: &Vm, j: JsPtr| {
        let toobj = sym!(vm, "js_toobject", unsafe extern "C-unwind" fn(JsPtr, c_int) -> *mut c_void);
        let pushobj = sym!(vm, "js_pushobject", unsafe extern "C-unwind" fn(JsPtr, *mut c_void));
        unsafe { (vm.newobject)(j) };
        unsafe { (vm.pushnumber)(j, 11.0) };
        unsafe { (vm.setproperty)(j, -2, b"v\0".as_ptr() as *const c_char) };
        let o = unsafe { toobj(j, -1) };
        unsafe { (vm.pop)(j, 1) };
        unsafe { pushobj(j, o) };
        logln(format!("pushed={:?}", stack_snapshot(vm, j)));
    };
    assert_same_protected(&cs, &rs, "js_pushobject", f);
}

#[test]
fn newfunction_matches() {
    let cs = Session::new(Side::C, 0);
    let rs = Session::new(Side::Rust, 0);
    for src in ["1+1", "return 2", "var z=3; z"] {
        let owned: &'static str = Box::leak(format!("{}\0", src).into_boxed_str());
        for use_env in [false, true] {
            let f = move |vm: &Vm, j: JsPtr| {
                let parse = sym!(vm, "jsP_parse", unsafe extern "C-unwind" fn(JsPtr, *const c_char, *const c_char) -> *mut c_void);
                let compile = sym!(vm, "jsC_compilescript", unsafe extern "C-unwind" fn(JsPtr, *mut c_void, c_int) -> *mut c_void);
                let freeparse = sym!(vm, "jsP_freeparse", unsafe extern "C-unwind" fn(JsPtr));
                let newfun = sym!(vm, "js_newfunction", unsafe extern "C-unwind" fn(JsPtr, *mut c_void, *mut c_void));
                let newenv = sym!(vm, "jsR_newenvironment", unsafe extern "C-unwind" fn(JsPtr, *mut c_void, *mut c_void) -> *mut c_void);
                let toobj = sym!(vm, "js_toobject", unsafe extern "C-unwind" fn(JsPtr, c_int) -> *mut c_void);

                let p = unsafe {
                    parse(
                        j,
                        b"nf.js\0".as_ptr() as *const c_char,
                        owned.as_ptr() as *const c_char,
                    )
                };
                let fun = unsafe { compile(j, p, 0) };
                unsafe { freeparse(j) };
                let env = if use_env {
                    unsafe { (vm.newobject)(j) };
                    let vars = unsafe { toobj(j, -1) };
                    unsafe { (vm.pop)(j, 1) };
                    unsafe { newenv(j, vars, std::ptr::null_mut()) }
                } else {
                    std::ptr::null_mut()
                };
                unsafe { newfun(j, fun, env) };
                logln(format!("fn={:?}", stack_snapshot(vm, j)));
                logln(format!("callable={}", unsafe { (vm.iscallable)(j, -1) }));
                unsafe { (vm.pushundefined)(j) };
                let e = unsafe { (vm.pcall)(j, 0) };
                logln(format!("err={} res={:?}", e, stack_snapshot(vm, j)));
            };
            assert_same_protected(
                &cs,
                &rs,
                &format!("js_newfunction {:?} env={}", src, use_env),
                f,
            );
        }
    }
}

#[test]
fn builtin_registration_helpers_match() {
    let cs = Session::new(Side::C, 0);
    let rs = Session::new(Side::Rust, 0);
    let f = |vm: &Vm, j: JsPtr| {
        let propf = sym!(vm, "jsB_propf", unsafe extern "C-unwind" fn(JsPtr, *const c_char, unsafe extern "C-unwind" fn(JsPtr), c_int));
        let propn = sym!(vm, "jsB_propn", unsafe extern "C-unwind" fn(JsPtr, *const c_char, f64));
        let props = sym!(vm, "jsB_props", unsafe extern "C-unwind" fn(JsPtr, *const c_char, *const c_char));

        unsafe extern "C-unwind" fn dummy(_j: JsPtr) {}

        unsafe { (vm.newobject)(j) };
        unsafe { propf(j, b"Ns.method\0".as_ptr() as *const c_char, dummy, 2) };
        unsafe { propf(j, b"nodot\0".as_ptr() as *const c_char, dummy, 0) };
        unsafe { propn(j, b"NUM\0".as_ptr() as *const c_char, 42.5) };
        unsafe { props(j, b"STR\0".as_ptr() as *const c_char, b"hello\0".as_ptr() as *const c_char) };
        logln(format!("obj={:?}", stack_snapshot(vm, j)));
        for k in [
            &b"method\0"[..],
            &b"Ns.method\0"[..],
            &b"nodot\0"[..],
            &b"NUM\0"[..],
            &b"STR\0"[..],
        ] {
            let p = k.as_ptr() as *const c_char;
            logln(format!("has {:?}={}", k, unsafe {
                (vm.hasproperty)(j, -1, p)
            }));
            if unsafe { (vm.hasproperty)(j, -1, p) } != 0 {
                logln(format!("val={:?}", stack_snapshot(vm, j)));
                unsafe { (vm.pop)(j, 1) };
            }
        }
        unsafe { (vm.pushiterator)(j, -1, 1) };
        loop {
            let k = unsafe { (vm.nextiterator)(j, -1) };
            if k.is_null() {
                break;
            }
            logln(format!("enum={}", unsafe { cstr_to_string(k) }.unwrap_or_default()));
        }
    };
    assert_same_protected(&cs, &rs, "jsB_propf/propn/props", f);
}

#[test]
fn jsB_init_reinitialisation_matches() {
    // Re-running the builtin initialisers on a live state must produce the same
    // observable global object on both implementations.
    for name in [
        "jsB_initobject",
        "jsB_initarray",
        "jsB_initfunction",
        "jsB_initboolean",
        "jsB_initnumber",
        "jsB_initstring",
        "jsB_initregexp",
        "jsB_initdate",
        "jsB_initerror",
        "jsB_initmath",
        "jsB_initjson",
        "jsB_init",
    ] {
        let cs = Session::new(Side::C, 0);
        let rs = Session::new(Side::Rust, 0);
        let n: &'static str = name;
        let f = move |vm: &Vm, j: JsPtr| {
            let cn = format!("{}\0", n);
            let s: Symbol<unsafe extern "C-unwind" fn(JsPtr)> =
                unsafe { lib_of(vm.side).get(cn.as_bytes()) }.unwrap();
            unsafe { s(j) };
            logln("reinitialised".to_string());
        };
        assert_same_protected(&cs, &rs, &format!("{} re-init", n), f);
        // The globals must still behave identically afterwards.
        for src in [
            "typeof Object+typeof Array+typeof Math+typeof JSON",
            "Object.keys({a:1}).join(',')",
            "[3,1,2].sort().join(',')",
            "'ab'.toUpperCase()",
            "JSON.stringify({a:[1,2]})",
            "String(/a/g)",
            "new Date(0).toISOString()",
            "new Error('x').toString()",
            "Math.max(1,2)",
            "(255).toString(16)",
            "Object.getOwnPropertyNames(this).sort().join(',')",
        ] {
            let a = run_script(&cs, src);
            let b = run_script(&rs, src);
            assert_eq!(a, b, "after {} re-init: {}", n, src);
        }
    }
}

// NOTE: jsS_freestrings does not reset J->strings, so calling it and then
// letting js_freestate call it again is a double free in the C original. That
// sequence is undefined behaviour and therefore not a valid differential test;
// jsS_freestrings is exercised through js_freestate in every other test.

#[test]
fn trap_matches() {
    let cs = Session::new(Side::C, 0);
    let rs = Session::new(Side::Rust, 0);
    // js_trap dumps the stack/environment to stdout; make sure both survive and
    // leave the state usable.
    let f = |vm: &Vm, j: JsPtr| {
        let trap = sym!(vm, "js_trap", unsafe extern "C-unwind" fn(JsPtr, c_int));
        unsafe { (vm.pushnumber)(j, 1.0) };
        unsafe { (vm.newobject)(j) };
        unsafe { trap(j, 0) };
        unsafe { trap(j, 7) };
        logln(format!("after trap={:?}", stack_snapshot(vm, j)));
    };
    assert_same_protected(&cs, &rs, "js_trap", f);
    let a = run_script(&cs, "1+1");
    let b = run_script(&rs, "1+1");
    assert_eq!(a, b, "state usable after js_trap");
    // and through the debugger opcode
    let a = run_script(&cs, "debugger; 5");
    let b = run_script(&rs, "debugger; 5");
    assert_eq!(a, b, "debugger opcode");
}
