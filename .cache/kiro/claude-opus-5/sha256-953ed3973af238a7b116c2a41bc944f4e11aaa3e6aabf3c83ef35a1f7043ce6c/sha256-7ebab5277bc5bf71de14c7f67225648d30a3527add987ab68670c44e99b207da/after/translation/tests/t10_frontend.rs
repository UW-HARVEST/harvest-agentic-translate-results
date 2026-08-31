// Level 10: parser / compiler / lexer exports, call & construct entry points,
// cfunction data + userdata callbacks, panic and context hooks.
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

const SOURCES: &[&str] = &[
    "1+1",
    "var x = 1; x",
    "function f(){return 2} f()",
    "'str'",
    "throw new Error('x')",
    "(",
    "var",
    "1+",
    "function(){}",
    "for(;;){break}",
    "with({a:1}) a",
    "try{1}catch(e){2}finally{3}",
    "switch(1){case 1: 'a'}",
    "/re/g",
    "[1,2,3].map(function(x){return x*2}).join(',')",
    "'use strict'; var y=1; y",
    "delete x",
    "eval('1')",
    "arguments",
    "017",
    "0x",
    "'\\u{'",
    "a\nb",
    "{ }",
    "label: for(;;) break label;",
    "/* unterminated",
    "'unterminated",
    "",
    "   ",
    "// only a comment",
];

#[test]
fn lexer_token_stream_matches() {
    let cs = Session::new(Side::C, 0);
    let rs = Session::new(Side::Rust, 0);
    for src in SOURCES {
        let owned: &'static str = Box::leak(format!("{}\0", src).into_boxed_str());
        let f = move |vm: &Vm, j: JsPtr| {
            let initlex = sym!(vm, "jsY_initlex", unsafe extern "C-unwind" fn(JsPtr, *const c_char, *const c_char));
            let lex = sym!(vm, "jsY_lex", unsafe extern "C-unwind" fn(JsPtr) -> c_int);
            let tokstr = sym!(vm, "jsY_tokenstring", unsafe extern "C-unwind" fn(c_int) -> *const c_char);
            unsafe {
                initlex(
                    j,
                    b"lex.js\0".as_ptr() as *const c_char,
                    owned.as_ptr() as *const c_char,
                )
            };
            for _ in 0..200 {
                let t = unsafe { lex(j) };
                logln(format!(
                    "{} {:?}",
                    t,
                    unsafe { cstr_to_string(tokstr(t)) }
                ));
                if t == 0 {
                    break;
                }
            }
        };
        assert_same_protected(&cs, &rs, &format!("jsY_lex on {:?}", src), f);
    }
}

#[test]
fn json_lexer_token_stream_matches() {
    let cs = Session::new(Side::C, 0);
    let rs = Session::new(Side::Rust, 0);
    let texts = [
        "1", "-1", "1.5", "1e3", "\"a\"", "\"\\u00e9\"", "true", "false", "null", "[]",
        "[1,2]", "{}", "{\"a\":1}", "{\"a\":[1,{\"b\":2}]}", " 1 ", "01", "+1", ".5", "1.",
        "'a'", "{a:1}", "[1,]", "\"\\x41\"", "\"\\u12\"", "[", "]", "{", "}", "", "  ",
        "nul", "trues", "1 2", "\"unterminated", "tru", "fals", "nu", "\"\\q\"", "\t\n1",
    ];
    for t in texts {
        let owned: &'static str = Box::leak(format!("{}\0", t).into_boxed_str());
        let f = move |vm: &Vm, j: JsPtr| {
            let initlex = sym!(vm, "jsY_initlex", unsafe extern "C-unwind" fn(JsPtr, *const c_char, *const c_char));
            let lexjson = sym!(vm, "jsY_lexjson", unsafe extern "C-unwind" fn(JsPtr) -> c_int);
            let tokstr = sym!(vm, "jsY_tokenstring", unsafe extern "C-unwind" fn(c_int) -> *const c_char);
            unsafe {
                initlex(
                    j,
                    b"json\0".as_ptr() as *const c_char,
                    owned.as_ptr() as *const c_char,
                )
            };
            for _ in 0..100 {
                let tk = unsafe { lexjson(j) };
                logln(format!("{} {:?}", tk, unsafe { cstr_to_string(tokstr(tk)) }));
                if tk == 0 {
                    break;
                }
            }
        };
        assert_same_protected(&cs, &rs, &format!("jsY_lexjson on {:?}", t), f);
    }
}

#[test]
fn parse_compile_pipeline_matches() {
    let cs = Session::new(Side::C, 0);
    let rs = Session::new(Side::Rust, 0);
    for src in SOURCES {
        for strict in [0i32, 1] {
            let owned: &'static str = Box::leak(format!("{}\0", src).into_boxed_str());
            let f = move |vm: &Vm, j: JsPtr| {
                let parse = sym!(vm, "jsP_parse", unsafe extern "C-unwind" fn(JsPtr, *const c_char, *const c_char) -> *mut c_void);
                let compile = sym!(vm, "jsC_compilescript", unsafe extern "C-unwind" fn(JsPtr, *mut c_void, c_int) -> *mut c_void);
                let freeparse = sym!(vm, "jsP_freeparse", unsafe extern "C-unwind" fn(JsPtr));
                let newscript = sym!(vm, "js_newscript", unsafe extern "C-unwind" fn(JsPtr, *mut c_void, *mut c_void));

                let p = unsafe {
                    parse(
                        j,
                        b"pipe.js\0".as_ptr() as *const c_char,
                        owned.as_ptr() as *const c_char,
                    )
                };
                logln(format!("parse_null={}", p.is_null()));
                let fun = unsafe { compile(j, p, strict) };
                logln(format!("compile_null={}", fun.is_null()));
                unsafe { freeparse(j) };
                unsafe { newscript(j, fun, std::ptr::null_mut()) };
                logln(format!("script={:?}", stack_snapshot(vm, j)));
                unsafe { (vm.pushundefined)(j) };
                let e = unsafe { (vm.pcall)(j, 0) };
                logln(format!(
                    "call_err={} result={:?}",
                    e,
                    stack_snapshot(vm, j)
                ));
            };
            assert_same_protected(
                &cs,
                &rs,
                &format!("parse/compile {:?} strict={}", src, strict),
                f,
            );
        }
    }
}

#[test]
fn parsefunction_and_compilefunction_match() {
    let cs = Session::new(Side::C, 0);
    let rs = Session::new(Side::Rust, 0);
    let cases: &[(&str, &str)] = &[
        ("", "return 1"),
        ("a", "return a"),
        ("a,b", "return a+b"),
        ("a,a", "return a"),
        ("", ""),
        ("", "return"),
        ("a", "'use strict'; return a"),
        ("", "1+"),
        ("1", "return 1"),
        ("a b", "return 1"),
        ("", "var x=1; return x"),
        ("", "function g(){return 2} return g()"),
        ("eval", "return 1"),
        ("arguments", "return 1"),
    ];
    for (params, body) in cases {
        let p: &'static str = Box::leak(format!("{}\0", params).into_boxed_str());
        let b: &'static str = Box::leak(format!("{}\0", body).into_boxed_str());
        for strict in [0i32, 1] {
            let f = move |vm: &Vm, j: JsPtr| {
                let parsefun = sym!(vm, "jsP_parsefunction", unsafe extern "C-unwind" fn(JsPtr, *const c_char, *const c_char, *const c_char) -> *mut c_void);
                let compilefun = sym!(vm, "jsC_compilefunction", unsafe extern "C-unwind" fn(JsPtr, *mut c_void) -> *mut c_void);
                let freeparse = sym!(vm, "jsP_freeparse", unsafe extern "C-unwind" fn(JsPtr));
                let newscript = sym!(vm, "js_newscript", unsafe extern "C-unwind" fn(JsPtr, *mut c_void, *mut c_void));
                let ast = unsafe {
                    parsefun(
                        j,
                        b"fun.js\0".as_ptr() as *const c_char,
                        p.as_ptr() as *const c_char,
                        b.as_ptr() as *const c_char,
                    )
                };
                logln(format!("ast_null={}", ast.is_null()));
                let fun = unsafe { compilefun(j, ast) };
                logln(format!("fun_null={}", fun.is_null()));
                unsafe { freeparse(j) };
                unsafe { newscript(j, fun, std::ptr::null_mut()) };
                logln(format!("script={:?}", stack_snapshot(vm, j)));
                unsafe { (vm.pushundefined)(j) };
                unsafe { (vm.pushnumber)(j, 3.0) };
                unsafe { (vm.pushnumber)(j, 4.0) };
                let e = unsafe { (vm.pcall)(j, 2) };
                logln(format!("call_err={} res={:?}", e, stack_snapshot(vm, j)));
            };
            assert_same_protected(
                &cs,
                &rs,
                &format!("parsefunction({:?},{:?}) strict={}", params, body, strict),
                f,
            );
        }
    }
}

#[test]
fn loadeval_and_eval_match() {
    let cs = Session::new(Side::C, 0);
    let rs = Session::new(Side::Rust, 0);
    for src in SOURCES {
        let owned: &'static str = Box::leak(format!("{}\0", src).into_boxed_str());
        let f = move |vm: &Vm, j: JsPtr| {
            let loadeval = sym!(vm, "js_loadeval", unsafe extern "C-unwind" fn(JsPtr, *const c_char, *const c_char));
            unsafe {
                loadeval(
                    j,
                    b"eval.js\0".as_ptr() as *const c_char,
                    owned.as_ptr() as *const c_char,
                )
            };
            logln(format!("loaded={:?}", stack_snapshot(vm, j)));
            unsafe { (vm.pushundefined)(j) };
            let e = unsafe { (vm.pcall)(j, 0) };
            logln(format!("err={} res={:?}", e, stack_snapshot(vm, j)));
        };
        assert_same_protected(&cs, &rs, &format!("js_loadeval {:?}", src), f);

        // js_eval consumes a string from the stack
        let f = move |vm: &Vm, j: JsPtr| {
            let eval = sym!(vm, "js_eval", unsafe extern "C-unwind" fn(JsPtr));
            unsafe { (vm.pushstring)(j, owned.as_ptr() as *const c_char) };
            unsafe { eval(j) };
            logln(format!("eval={:?}", stack_snapshot(vm, j)));
        };
        assert_same_protected(&cs, &rs, &format!("js_eval {:?}", src), f);
    }
}

#[test]
fn call_and_construct_match() {
    let cs = Session::new(Side::C, 0);
    let rs = Session::new(Side::Rust, 0);
    let setup = "\
        var plain = function(a,b){ return [this===undefined?'u':(this===this?'t':'?'), a, b].join('|') };\
        var ctor = function(a){ this.a = a; };\
        var thrower = function(){ throw new Error('boom') };\
        var retobj = function(){ return {r:1} };\
        var notfn = {};\
        var arrctor = Array;\
        var errctor = Error;";
    assert_eq!(run_script(&cs, setup), run_script(&rs, setup));

    for (name, nargs) in [
        ("plain", 0i32),
        ("plain", 1),
        ("plain", 2),
        ("plain", 3),
        ("ctor", 1),
        ("thrower", 0),
        ("retobj", 0),
        ("notfn", 0),
        ("arrctor", 1),
        ("arrctor", 3),
        ("errctor", 1),
    ] {
        let nm: &'static str = Box::leak(format!("{}\0", name).into_boxed_str());
        // js_call
        let f = move |vm: &Vm, j: JsPtr| {
            unsafe { (vm.getglobal)(j, nm.as_ptr() as *const c_char) };
            unsafe { (vm.pushundefined)(j) };
            for i in 0..nargs {
                unsafe { (vm.pushnumber)(j, i as f64 + 10.0) };
            }
            let e = unsafe { (vm.pcall)(j, nargs) };
            logln(format!("pcall err={} res={:?}", e, stack_snapshot(vm, j)));
        };
        assert_same_protected(&cs, &rs, &format!("pcall {} n={}", name, nargs), f);

        // js_pconstruct
        let f = move |vm: &Vm, j: JsPtr| {
            unsafe { (vm.getglobal)(j, nm.as_ptr() as *const c_char) };
            for i in 0..nargs {
                unsafe { (vm.pushnumber)(j, i as f64 + 10.0) };
            }
            let e = unsafe { (vm.pconstruct)(j, nargs) };
            logln(format!("pconstruct err={} res={:?}", e, stack_snapshot(vm, j)));
        };
        assert_same_protected(&cs, &rs, &format!("pconstruct {} n={}", name, nargs), f);

        // unprotected js_call / js_construct inside our own protected frame
        let f = move |vm: &Vm, j: JsPtr| {
            let call = sym!(vm, "js_call", unsafe extern "C-unwind" fn(JsPtr, c_int));
            unsafe { (vm.getglobal)(j, nm.as_ptr() as *const c_char) };
            unsafe { (vm.pushundefined)(j) };
            for i in 0..nargs {
                unsafe { (vm.pushnumber)(j, i as f64 + 10.0) };
            }
            unsafe { call(j, nargs) };
            logln(format!("call res={:?}", stack_snapshot(vm, j)));
        };
        assert_same_protected(&cs, &rs, &format!("js_call {} n={}", name, nargs), f);

        let f = move |vm: &Vm, j: JsPtr| {
            let construct = sym!(vm, "js_construct", unsafe extern "C-unwind" fn(JsPtr, c_int));
            unsafe { (vm.getglobal)(j, nm.as_ptr() as *const c_char) };
            for i in 0..nargs {
                unsafe { (vm.pushnumber)(j, i as f64 + 10.0) };
            }
            unsafe { construct(j, nargs) };
            logln(format!("construct res={:?}", stack_snapshot(vm, j)));
        };
        assert_same_protected(&cs, &rs, &format!("js_construct {} n={}", name, nargs), f);
    }
}

// ---------------------------------------------------------------------------
// cfunction with data + finalizer
// ---------------------------------------------------------------------------

static mut FINALIZED_C: c_int = 0;
static mut FINALIZED_R: c_int = 0;

unsafe extern "C-unwind" fn fin_c(_j: JsPtr, _p: *mut c_void) {
    unsafe { FINALIZED_C += 1 };
}
unsafe extern "C-unwind" fn fin_r(_j: JsPtr, _p: *mut c_void) {
    unsafe { FINALIZED_R += 1 };
}
unsafe extern "C-unwind" fn datafn_c(j: JsPtr) {
    let vm = current_vm(Side::C);
    emit_data(&vm, j);
}
unsafe extern "C-unwind" fn datafn_r(j: JsPtr) {
    let vm = current_vm(Side::Rust);
    emit_data(&vm, j);
}
fn emit_data(vm: &Vm, j: JsPtr) {
    let i = impls();
    let lib = match vm.side {
        Side::C => &i.c,
        Side::Rust => &i.rust,
    };
    type CurData = unsafe extern "C-unwind" fn(JsPtr) -> *mut c_void;
    let cd: Symbol<CurData> = unsafe { lib.get(b"js_currentfunctiondata\0").unwrap() };
    let d = unsafe { cd(j) };
    unsafe { (vm.pushnumber)(j, d as usize as f64) };
}

#[test]
fn newcfunctionx_data_and_finalize_match() {
    let cs = Session::new(Side::C, 0);
    let rs = Session::new(Side::Rust, 0);
    for s in [&cs, &rs] {
        let vm = &s.vm;
        let j = s.j;
        type NewCFX = unsafe extern "C-unwind" fn(
            JsPtr,
            unsafe extern "C-unwind" fn(JsPtr),
            *const c_char,
            c_int,
            *mut c_void,
            Option<unsafe extern "C-unwind" fn(JsPtr, *mut c_void)>,
        );
        let i = impls();
        let lib = match vm.side {
            Side::C => &i.c,
            Side::Rust => &i.rust,
        };
        let ncfx: Symbol<NewCFX> = unsafe { lib.get(b"js_newcfunctionx\0").unwrap() };
        let (fun, fin): (
            unsafe extern "C-unwind" fn(JsPtr),
            unsafe extern "C-unwind" fn(JsPtr, *mut c_void),
        ) = match vm.side {
            Side::C => (datafn_c, fin_c),
            Side::Rust => (datafn_r, fin_r),
        };
        unsafe {
            ncfx(
                j,
                fun,
                b"withdata\0".as_ptr() as *const c_char,
                0,
                0xDEAD_BEEF_usize as *mut c_void,
                Some(fin),
            )
        };
        unsafe { (vm.setglobal)(j, b"withdata\0".as_ptr() as *const c_char) };
    }
    for src in [
        "withdata()",
        "withdata()===3735928559",
        "withdata.length",
        "withdata.name",
        "typeof withdata",
        "String(withdata)",
    ] {
        let a = run_script(&cs, src);
        let b = run_script(&rs, src);
        assert_eq!(a, b, "cfunctionx script {}", src);
    }
    // The finalizer must run when the state is freed.
    drop(cs);
    drop(rs);
    let (fc, fr) = unsafe { (FINALIZED_C, FINALIZED_R) };
    assert_eq!(fc, fr, "finalizer call count differs (C={}, Rust={})", fc, fr);
    assert!(fc > 0, "finalizer never ran");
}

// ---------------------------------------------------------------------------
// userdata with has / put / delete hooks
// ---------------------------------------------------------------------------

unsafe extern "C-unwind" fn ud_has_c(j: JsPtr, _p: *mut c_void, name: *const c_char) -> c_int {
    ud_has(&current_vm(Side::C), j, name)
}
unsafe extern "C-unwind" fn ud_has_r(j: JsPtr, _p: *mut c_void, name: *const c_char) -> c_int {
    ud_has(&current_vm(Side::Rust), j, name)
}
fn ud_has(vm: &Vm, j: JsPtr, name: *const c_char) -> c_int {
    let n = unsafe { cstr_to_string(name) }.unwrap_or_default();
    if n == "magic" {
        unsafe { (vm.pushnumber)(j, 99.0) };
        1
    } else {
        0
    }
}
unsafe extern "C-unwind" fn ud_put_c(_j: JsPtr, _p: *mut c_void, name: *const c_char) -> c_int {
    (unsafe { cstr_to_string(name) }.unwrap_or_default() == "magic") as c_int
}
unsafe extern "C-unwind" fn ud_put_r(_j: JsPtr, _p: *mut c_void, name: *const c_char) -> c_int {
    (unsafe { cstr_to_string(name) }.unwrap_or_default() == "magic") as c_int
}
unsafe extern "C-unwind" fn ud_del_c(_j: JsPtr, _p: *mut c_void, name: *const c_char) -> c_int {
    (unsafe { cstr_to_string(name) }.unwrap_or_default() == "magic") as c_int
}
unsafe extern "C-unwind" fn ud_del_r(_j: JsPtr, _p: *mut c_void, name: *const c_char) -> c_int {
    (unsafe { cstr_to_string(name) }.unwrap_or_default() == "magic") as c_int
}

#[test]
fn userdatax_hooks_match() {
    let cs = Session::new(Side::C, 0);
    let rs = Session::new(Side::Rust, 0);
    for s in [&cs, &rs] {
        let vm = &s.vm;
        let j = s.j;
        type HasFn = unsafe extern "C-unwind" fn(JsPtr, *mut c_void, *const c_char) -> c_int;
        type NewUDX = unsafe extern "C-unwind" fn(
            JsPtr,
            *const c_char,
            *mut c_void,
            Option<HasFn>,
            Option<HasFn>,
            Option<HasFn>,
            Option<unsafe extern "C-unwind" fn(JsPtr, *mut c_void)>,
        );
        let i = impls();
        let lib = match vm.side {
            Side::C => &i.c,
            Side::Rust => &i.rust,
        };
        let n: Symbol<NewUDX> = unsafe { lib.get(b"js_newuserdatax\0").unwrap() };
        let (h, p, d): (HasFn, HasFn, HasFn) = match vm.side {
            Side::C => (ud_has_c, ud_put_c, ud_del_c),
            Side::Rust => (ud_has_r, ud_put_r, ud_del_r),
        };
        unsafe { (vm.newobject)(j) }; // prototype
        unsafe {
            n(
                j,
                b"HookTag\0".as_ptr() as *const c_char,
                0x42 as *mut c_void,
                Some(h),
                Some(p),
                Some(d),
                None,
            )
        };
        unsafe { (vm.setglobal)(j, b"hooked\0".as_ptr() as *const c_char) };
    }
    for src in [
        "hooked.magic",
        "hooked.other",
        "'magic' in hooked",
        "'other' in hooked",
        "hooked.magic = 1; hooked.magic",
        "hooked.other = 1; hooked.other",
        "delete hooked.magic",
        "delete hooked.other",
        "typeof hooked",
        "String(hooked)",
        "var s=''; for(var k in hooked) s+=k; s",
        "JSON.stringify(hooked)",
        "Object.keys(hooked).join(',')",
    ] {
        let a = run_script(&cs, src);
        let b = run_script(&rs, src);
        assert_eq!(a, b, "userdatax script {}", src);
    }
}

#[test]
fn newobjectx_matches() {
    let cs = Session::new(Side::C, 0);
    let rs = Session::new(Side::Rust, 0);
    let f = |vm: &Vm, j: JsPtr| {
        let nox = sym!(vm, "js_newobjectx", unsafe extern "C-unwind" fn(JsPtr));
        // js_newobjectx uses the value on the stack as prototype
        unsafe { (vm.newobject)(j) };
        unsafe { (vm.pushnumber)(j, 5.0) };
        unsafe { (vm.setproperty)(j, -2, b"p\0".as_ptr() as *const c_char) };
        unsafe { nox(j) };
        logln(format!("obj={:?}", stack_snapshot(vm, j)));
        unsafe { (vm.getproperty)(j, -1, b"p\0".as_ptr() as *const c_char) };
        logln(format!("inherited={:?}", stack_snapshot(vm, j)));
    };
    assert_same_protected(&cs, &rs, "js_newobjectx", f);

    // and with a non-object prototype
    let f = |vm: &Vm, j: JsPtr| {
        let nox = sym!(vm, "js_newobjectx", unsafe extern "C-unwind" fn(JsPtr));
        unsafe { (vm.pushnumber)(j, 1.0) };
        unsafe { nox(j) };
        logln(format!("obj={:?}", stack_snapshot(vm, j)));
    };
    assert_same_protected(&cs, &rs, "js_newobjectx non-object proto", f);
}

#[test]
fn context_and_panic_hooks_match() {
    let cs = Session::new(Side::C, 0);
    let rs = Session::new(Side::Rust, 0);
    let f = |vm: &Vm, j: JsPtr| {
        let setctx = sym!(vm, "js_setcontext", unsafe extern "C-unwind" fn(JsPtr, *mut c_void));
        let getctx = sym!(vm, "js_getcontext", unsafe extern "C-unwind" fn(JsPtr) -> *mut c_void);
        logln(format!("initial_null={}", unsafe { getctx(j) }.is_null()));
        unsafe { setctx(j, 0x1234 as *mut c_void) };
        logln(format!("after={}", unsafe { getctx(j) } as usize));
        unsafe { setctx(j, std::ptr::null_mut()) };
        logln(format!("cleared_null={}", unsafe { getctx(j) }.is_null()));

        let atpanic = sym!(vm, "js_atpanic", unsafe extern "C-unwind" fn(JsPtr, *mut c_void) -> *mut c_void);
        let old = unsafe { atpanic(j, std::ptr::null_mut()) };
        logln(format!("old_panic_null={}", old.is_null()));
        let old2 = unsafe { atpanic(j, old) };
        logln(format!("second_null={}", old2.is_null()));
    };
    assert_same_protected(&cs, &rs, "context/panic hooks", f);
}

#[test]
fn jsC_error_and_syntax_messages_match() {
    // Compile-time diagnostics must be byte-identical, including line numbers.
    let cs = Session::new(Side::C, 0);
    let rs = Session::new(Side::Rust, 0);
    let bad = [
        "(", ")", "var", "var 1", "1+", "function", "function(){}", "if", "else", "{",
        "return", "break", "continue", "case 1:", "default:", "x: break y;",
        "for(var i in) ;", "1 = 2", "++1", "delete 1", "a\n\n\n+", "\n\n\nvar",
        "function f(1){}", "0x", "1e", "'\\u{'", "/[/", "/a", "'a\nb'", "'use strict'; 017",
        "'use strict'; var eval=1", "'use strict'; delete x", "'use strict'; with({}){}",
        "'use strict'; function f(a,a){}", "'use strict'; arguments=1",
        "try{}", "try{}catch(){}", "switch(1){case:}", "do 1", "while", "label:",
        "new", "typeof", "[1,", "{a:", "a.", "a[", "a(", "'\\'", "\"\\\"",
    ];
    for src in bad {
        let a = run_script(&cs, src);
        let b = run_script(&rs, src);
        assert_eq!(a, b, "syntax error message for {:?}", src);
        // also through eval so jsC_error paths inside a running script are hit
        let wrapped = format!(
            "try{{ eval('{}') }}catch(e){{ e.name+': '+e.message }}",
            src.replace('\\', "\\\\").replace('\'', "\\'").replace('\n', "\\n")
        );
        let a = run_script(&cs, &wrapped);
        let b = run_script(&rs, &wrapped);
        assert_eq!(a, b, "eval syntax error for {:?}", src);
    }
}
