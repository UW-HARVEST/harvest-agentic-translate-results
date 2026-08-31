// Level 9: exported *internal* entry points that can be reached with opaque
// pointers obtained from other exports (js_tovalue / js_toobject), plus the
// js_Buffer helpers.
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

type JsValuePtr = *mut c_void;
type JsObjectPtr = *mut c_void;

const VALUE_EXPRS: &[&str] = &[
    "undefined",
    "null",
    "true",
    "false",
    "0",
    "-0",
    "1",
    "-1",
    "0.5",
    "NaN",
    "Infinity",
    "-Infinity",
    "1e21",
    "1e-7",
    "2147483648",
    "''",
    "'a'",
    "'abc'",
    "'0'",
    "'1e3'",
    "'  12  '",
    "'\\u00e9\\u4e2d'",
    "'a-very-long-string-that-will-not-fit-in-the-inline-short-string-buffer'",
    "{}",
    "[]",
    "[1,2,3]",
    "({a:1})",
    "(function(){})",
    "/re/gi",
    "new Date(0)",
    "new Number(5)",
    "new String('boxed')",
    "new Boolean(true)",
    "new Error('e')",
    "({valueOf:function(){return 42}})",
    "({toString:function(){return 'ts'}})",
    "({valueOf:function(){return {}},toString:function(){return {}}})",
    "Math",
    "JSON",
];

#[test]
fn jsV_value_conversions_match() {
    let cs = Session::new(Side::C, 0);
    let rs = Session::new(Side::Rust, 0);
    for expr in VALUE_EXPRS {
        let setup = format!("var __v = ({});", expr);
        assert_eq!(
            run_script(&cs, &setup),
            run_script(&rs, &setup),
            "setup {}",
            expr
        );
        for preferred in [0i32, 1, 2, 3, 4, 5, 6] {
            let f = move |vm: &Vm, j: JsPtr| {
                let tovalue = sym!(vm, "js_tovalue", unsafe extern "C-unwind" fn(JsPtr, c_int) -> JsValuePtr);
                let toboolean = sym!(vm, "jsV_toboolean", unsafe extern "C-unwind" fn(JsPtr, JsValuePtr) -> c_int);
                let tonumber = sym!(vm, "jsV_tonumber", unsafe extern "C-unwind" fn(JsPtr, JsValuePtr) -> f64);
                let tointeger = sym!(vm, "jsV_tointeger", unsafe extern "C-unwind" fn(JsPtr, JsValuePtr) -> f64);
                let tostring = sym!(vm, "jsV_tostring", unsafe extern "C-unwind" fn(JsPtr, JsValuePtr) -> *const c_char);
                let toprimitive = sym!(vm, "jsV_toprimitive", unsafe extern "C-unwind" fn(JsPtr, JsValuePtr, c_int));

                unsafe { (vm.getglobal)(j, b"__v\0".as_ptr() as *const c_char) };
                let v = unsafe { tovalue(j, -1) };
                logln(format!("bool={}", unsafe { toboolean(j, v) }));
                let n = unsafe { tonumber(j, v) };
                logln(format!("num={:?}", n.to_bits()));
                let i = unsafe { tointeger(j, v) };
                logln(format!("int={:?}", i.to_bits()));
                logln(format!("str={:?}", unsafe {
                    cstr_to_bytes(tostring(j, v))
                }));
                // toprimitive mutates the value in place
                unsafe { toprimitive(j, v, preferred) };
                logln(format!("prim={:?}", stack_snapshot(vm, j)));
            };
            assert_same_protected(
                &cs,
                &rs,
                &format!("jsV conversions {} preferred={}", expr, preferred),
                f,
            );
        }
    }
}

#[test]
fn jsV_toobject_and_stringtonumber_match() {
    let cs = Session::new(Side::C, 0);
    let rs = Session::new(Side::Rust, 0);
    for expr in VALUE_EXPRS {
        let setup = format!("var __v = ({});", expr);
        assert_eq!(run_script(&cs, &setup), run_script(&rs, &setup));
        let f = |vm: &Vm, j: JsPtr| {
            let tovalue = sym!(vm, "js_tovalue", unsafe extern "C-unwind" fn(JsPtr, c_int) -> JsValuePtr);
            let toobject = sym!(vm, "jsV_toobject", unsafe extern "C-unwind" fn(JsPtr, JsValuePtr) -> JsObjectPtr);
            unsafe { (vm.getglobal)(j, b"__v\0".as_ptr() as *const c_char) };
            let v = unsafe { tovalue(j, -1) };
            let o = unsafe { toobject(j, v) };
            logln(format!("obj_null={}", o.is_null()));
            // reflect it back onto the stack via js_toobject to compare shape
            logln(format!("stack={:?}", stack_snapshot(vm, j)));
        };
        assert_same_protected(&cs, &rs, &format!("jsV_toobject {}", expr), f);
    }

    let strings: &[&[u8]] = &[
        b"\0",
        b"0\0",
        b"1\0",
        b"-1\0",
        b"  12  \0",
        b"1e3\0",
        b"0x10\0",
        b"0X1f\0",
        b"Infinity\0",
        b"-Infinity\0",
        b"+Infinity\0",
        b"NaN\0",
        b"abc\0",
        b".5\0",
        b"5.\0",
        b"1.5e-3\0",
        b"017\0",
        b"08\0",
        b" \t\n\r 42 \t\n\r \0",
        b"1 2\0",
        b"--1\0",
        b"1e999\0",
        b"1e-999\0",
        b"9007199254740993\0",
    ];
    for s in strings {
        let sp: &'static [u8] = s;
        let f = move |vm: &Vm, j: JsPtr| {
            let stn = sym!(vm, "jsV_stringtonumber", unsafe extern "C-unwind" fn(JsPtr, *const c_char) -> f64);
            let d = unsafe { stn(j, sp.as_ptr() as *const c_char) };
            logln(format!("{:?}", d.to_bits()));
        };
        assert_same_protected(&cs, &rs, &format!("jsV_stringtonumber {:?}", s), f);
    }

    // jsV_numbertostring with an explicit 32-byte buffer
    let nums: &[f64] = &[
        0.0, -0.0, 1.0, -1.0, 0.1, 1e21, 1e-7, 1e-6, 1234567890.0, -1234567890.5,
        f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 5e-324, 1.7976931348623157e308,
        2147483647.0, -2147483648.0, 4294967296.0, 1e20, 1e-5, 100.0, 0.000001,
    ];
    for &n in nums {
        let f = move |vm: &Vm, j: JsPtr| {
            let nts = sym!(vm, "jsV_numbertostring", unsafe extern "C-unwind" fn(JsPtr, *mut c_char, f64) -> *const c_char);
            let mut buf = [0u8; 64];
            let p = unsafe { nts(j, buf.as_mut_ptr() as *mut c_char, n) };
            logln(format!("{:?}", unsafe { cstr_to_bytes(p) }));
            logln(format!("inbuf={}", p as usize == buf.as_ptr() as usize));
        };
        assert_same_protected(&cs, &rs, &format!("jsV_numbertostring {}", n), f);
    }
}

#[test]
fn jsV_property_api_matches() {
    let cs = Session::new(Side::C, 0);
    let rs = Session::new(Side::Rust, 0);
    let setups = [
        "var __o = {};",
        "var __o = {a:1,b:2,c:3};",
        "var __o = [1,2,3];",
        "var __o = Object.create({inherited:1});",
        "var __o = (function(){var o={};Object.defineProperty(o,'h',{value:1});return o})();",
        "var __o = Object.freeze({a:1});",
        "var __o = function(){};",
        "var __o = /re/;",
        "var __o = new Date(0);",
    ];
    let names: &[&[u8]] = &[
        b"a\0", b"b\0", b"zz\0", b"length\0", b"inherited\0", b"h\0", b"0\0", b"1\0",
        b"toString\0", b"\0",
    ];
    for setup in setups {
        assert_eq!(run_script(&cs, setup), run_script(&rs, setup), "{}", setup);
        for nm in names {
            let name: &'static [u8] = nm;
            let f = move |vm: &Vm, j: JsPtr| {
                let toobj = sym!(vm, "js_toobject", unsafe extern "C-unwind" fn(JsPtr, c_int) -> JsObjectPtr);
                let getown = sym!(vm, "jsV_getownproperty", unsafe extern "C-unwind" fn(JsPtr, JsObjectPtr, *const c_char) -> *mut c_void);
                let get = sym!(vm, "jsV_getproperty", unsafe extern "C-unwind" fn(JsPtr, JsObjectPtr, *const c_char) -> *mut c_void);
                let getx = sym!(vm, "jsV_getpropertyx", unsafe extern "C-unwind" fn(JsPtr, JsObjectPtr, *const c_char, *mut c_int) -> *mut c_void);
                let set = sym!(vm, "jsV_setproperty", unsafe extern "C-unwind" fn(JsPtr, JsObjectPtr, *const c_char) -> *mut c_void);
                let del = sym!(vm, "jsV_delproperty", unsafe extern "C-unwind" fn(JsPtr, JsObjectPtr, *const c_char));

                unsafe { (vm.getglobal)(j, b"__o\0".as_ptr() as *const c_char) };
                let o = unsafe { toobj(j, -1) };
                let p = name.as_ptr() as *const c_char;
                logln(format!("own_null={}", unsafe { getown(j, o, p) }.is_null()));
                logln(format!("get_null={}", unsafe { get(j, o, p) }.is_null()));
                let mut own: c_int = -5;
                let gx = unsafe { getx(j, o, p, &mut own) };
                logln(format!("getx_null={} own={}", gx.is_null(), own));
                logln(format!("set_null={}", unsafe { set(j, o, p) }.is_null()));
                logln(format!("own_after_set={}", unsafe { getown(j, o, p) }.is_null()));
                unsafe { del(j, o, p) };
                logln(format!("own_after_del={}", unsafe { getown(j, o, p) }.is_null()));
            };
            assert_same_protected(
                &cs,
                &rs,
                &format!("jsV property {} on {}", String::from_utf8_lossy(nm), setup),
                f,
            );
        }
        // Enumeration order via jsV_newiterator / jsV_nextiterator.
        for own in [0i32, 1] {
            let f = move |vm: &Vm, j: JsPtr| {
                let toobj = sym!(vm, "js_toobject", unsafe extern "C-unwind" fn(JsPtr, c_int) -> JsObjectPtr);
                let newit = sym!(vm, "jsV_newiterator", unsafe extern "C-unwind" fn(JsPtr, JsObjectPtr, c_int) -> JsObjectPtr);
                let nextit = sym!(vm, "jsV_nextiterator", unsafe extern "C-unwind" fn(JsPtr, JsObjectPtr) -> *const c_char);
                unsafe { (vm.getglobal)(j, b"__o\0".as_ptr() as *const c_char) };
                let o = unsafe { toobj(j, -1) };
                let it = unsafe { newit(j, o, own) };
                for _ in 0..64 {
                    let k = unsafe { nextit(j, it) };
                    if k.is_null() {
                        break;
                    }
                    logln(unsafe { cstr_to_string(k) }.unwrap_or_default());
                }
            };
            assert_same_protected(
                &cs,
                &rs,
                &format!("jsV iterator own={} on {}", own, setup),
                f,
            );
        }
    }
}

#[test]
fn jsV_resizearray_and_unflatten_match() {
    let cs = Session::new(Side::C, 0);
    let rs = Session::new(Side::Rust, 0);
    for setup in [
        "var __a = [];",
        "var __a = [1,2,3];",
        "var __a = [1,2,3,4,5,6,7,8,9,10];",
        "var __a = (function(){var a=[1,2,3];a.x=1;return a})();",
        "var __a = (function(){var a=[];a[5]=1;return a})();",
    ] {
        assert_eq!(run_script(&cs, setup), run_script(&rs, setup));
        for newlen in [0i32, 1, 3, 5, 20] {
            let f = move |vm: &Vm, j: JsPtr| {
                let toobj = sym!(vm, "js_toobject", unsafe extern "C-unwind" fn(JsPtr, c_int) -> JsObjectPtr);
                let resize = sym!(vm, "jsV_resizearray", unsafe extern "C-unwind" fn(JsPtr, JsObjectPtr, c_int));
                let unflat = sym!(vm, "jsR_unflattenarray", unsafe extern "C-unwind" fn(JsPtr, JsObjectPtr));
                unsafe { (vm.getglobal)(j, b"__a\0".as_ptr() as *const c_char) };
                let o = unsafe { toobj(j, -1) };
                // jsV_resizearray asserts !simple, so unflatten first (this is
                // what the interpreter itself does before resizing).
                unsafe { unflat(j, o) };
                logln(format!("after unflatten: {:?}", stack_snapshot(vm, j)));
                unsafe { resize(j, o, newlen) };
                logln(format!("after resize: {:?}", stack_snapshot(vm, j)));
                logln(format!("len={}", unsafe { (vm.getlength)(j, -1) }));
                // second unflatten must be a no-op
                unsafe { unflat(j, o) };
                unsafe { (vm.pushiterator)(j, -1, 1) };
                loop {
                    let k = unsafe { (vm.nextiterator)(j, -1) };
                    if k.is_null() {
                        break;
                    }
                    logln(format!("key={}", unsafe { cstr_to_string(k) }.unwrap_or_default()));
                }
            };
            assert_same_protected(
                &cs,
                &rs,
                &format!("resizearray({}) on {}", newlen, setup),
                f,
            );
        }
    }
}

#[test]
fn jsV_newmemstring_and_newobject_match() {
    let cs = Session::new(Side::C, 0);
    let rs = Session::new(Side::Rust, 0);
    let strs: &[(&[u8], c_int)] = &[
        (b"\0", 0),
        (b"a\0", 1),
        (b"hello world\0", 11),
        (b"hello world\0", 5),
        (b"\xc3\xa9\xe4\xb8\xad\0", 5),
        (b"\xc3\xa9\xe4\xb8\xad\0", 2),
        (b"0123456789012345678901234567890123456789\0", 40),
    ];
    for (bytes, n) in strs {
        let b: &'static [u8] = bytes;
        let len = *n;
        let f = move |vm: &Vm, j: JsPtr| {
            let nms = sym!(vm, "jsV_newmemstring", unsafe extern "C-unwind" fn(JsPtr, *const c_char, c_int) -> *mut c_void);
            let p = unsafe { nms(j, b.as_ptr() as *const c_char, len) };
            logln(format!("null={}", p.is_null()));
            // Push it as a string through the normal API for comparison.
            unsafe { (vm.pushlstring)(j, b.as_ptr() as *const c_char, len) };
            logln(format!("stack={:?}", stack_snapshot(vm, j)));
        };
        assert_same_protected(
            &cs,
            &rs,
            &format!("jsV_newmemstring {:?} n={}", bytes, n),
            f,
        );
    }

    // jsV_newobject with each class id and a NULL / real prototype.
    for class in 0..16i32 {
        for with_proto in [false, true] {
            let f = move |vm: &Vm, j: JsPtr| {
                let nobj = sym!(vm, "jsV_newobject", unsafe extern "C-unwind" fn(JsPtr, c_int, JsObjectPtr) -> JsObjectPtr);
                let toobj = sym!(vm, "js_toobject", unsafe extern "C-unwind" fn(JsPtr, c_int) -> JsObjectPtr);
                let proto = if with_proto {
                    unsafe { (vm.newobject)(j) };
                    let p = unsafe { toobj(j, -1) };
                    unsafe { (vm.pop)(j, 1) };
                    p
                } else {
                    std::ptr::null_mut()
                };
                let o = unsafe { nobj(j, class, proto) };
                logln(format!("class={} null={}", class, o.is_null()));
            };
            assert_same_protected(
                &cs,
                &rs,
                &format!("jsV_newobject class={} proto={}", class, with_proto),
                f,
            );
        }
    }
}

#[repr(C)]
struct JsBuffer {
    n: c_int,
    m: c_int,
    s: [u8; 64],
}

#[test]
fn buffer_helpers_match() {
    let cs = Session::new(Side::C, 0);
    let rs = Session::new(Side::Rust, 0);
    // js_putc / js_puts / js_putm grow a js_Buffer allocated with js_malloc.
    let f = |vm: &Vm, j: JsPtr| {
        let putc = sym!(vm, "js_putc", unsafe extern "C-unwind" fn(JsPtr, *mut *mut JsBuffer, c_int));
        let puts = sym!(vm, "js_puts", unsafe extern "C-unwind" fn(JsPtr, *mut *mut JsBuffer, *const c_char));
        let putm = sym!(vm, "js_putm", unsafe extern "C-unwind" fn(JsPtr, *mut *mut JsBuffer, *const c_char, *const c_char));
        let free = sym!(vm, "js_free", unsafe extern "C-unwind" fn(JsPtr, *mut c_void));

        let mut sb: *mut JsBuffer = std::ptr::null_mut();
        for i in 0..40 {
            unsafe { putc(j, &mut sb, b'a' as c_int + (i % 26)) };
        }
        unsafe { puts(j, &mut sb, b"hello\0".as_ptr() as *const c_char) };
        let src = b"0123456789\0";
        unsafe {
            putm(
                j,
                &mut sb,
                src.as_ptr() as *const c_char,
                src.as_ptr().add(10) as *const c_char,
            )
        };
        for i in 0..300 {
            unsafe { putc(j, &mut sb, b'0' as c_int + (i % 10)) };
        }
        unsafe { puts(j, &mut sb, b"\0".as_ptr() as *const c_char) };
        unsafe { putm(j, &mut sb, src.as_ptr() as *const c_char, src.as_ptr() as *const c_char) };
        unsafe { putc(j, &mut sb, 0) };
        let b = unsafe { &*sb };
        logln(format!("n={} m={}", b.n, b.m));
        let bytes: Vec<u8> =
            unsafe { std::slice::from_raw_parts((&raw const b.s) as *const u8, b.n as usize) }
                .to_vec();
        logln(format!("bytes={:?}", bytes));
        unsafe { free(j, sb as *mut c_void) };
    };
    assert_same_protected(&cs, &rs, "js_putc/js_puts/js_putm", f);
}

#[test]
fn malloc_realloc_free_match() {
    let cs = Session::new(Side::C, 0);
    let rs = Session::new(Side::Rust, 0);
    let f = |vm: &Vm, j: JsPtr| {
        let malloc = sym!(vm, "js_malloc", unsafe extern "C-unwind" fn(JsPtr, c_int) -> *mut c_void);
        let realloc = sym!(vm, "js_realloc", unsafe extern "C-unwind" fn(JsPtr, *mut c_void, c_int) -> *mut c_void);
        let free = sym!(vm, "js_free", unsafe extern "C-unwind" fn(JsPtr, *mut c_void));
        let strdup = sym!(vm, "js_strdup", unsafe extern "C-unwind" fn(JsPtr, *const c_char) -> *mut c_char);
        for n in [1i32, 8, 64, 1024, 65536] {
            let p = unsafe { malloc(j, n) };
            logln(format!("malloc({}) null={}", n, p.is_null()));
            let q = unsafe { realloc(j, p, n * 2) };
            logln(format!("realloc null={}", q.is_null()));
            unsafe { free(j, q) };
        }
        let s = unsafe { strdup(j, b"duplicate me\0".as_ptr() as *const c_char) };
        logln(format!("strdup={:?}", unsafe { cstr_to_bytes(s) }));
        unsafe { free(j, s as *mut c_void) };
    };
    assert_same_protected(&cs, &rs, "js_malloc/js_realloc/js_free/js_strdup", f);
}

#[test]
fn newenvironment_and_arguments_match() {
    let cs = Session::new(Side::C, 0);
    let rs = Session::new(Side::Rust, 0);
    let f = |vm: &Vm, j: JsPtr| {
        let toobj = sym!(vm, "js_toobject", unsafe extern "C-unwind" fn(JsPtr, c_int) -> JsObjectPtr);
        let newenv = sym!(vm, "jsR_newenvironment", unsafe extern "C-unwind" fn(JsPtr, JsObjectPtr, *mut c_void) -> *mut c_void);
        unsafe { (vm.newobject)(j) };
        let vars = unsafe { toobj(j, -1) };
        let e1 = unsafe { newenv(j, vars, std::ptr::null_mut()) };
        logln(format!("e1 null={}", e1.is_null()));
        let e2 = unsafe { newenv(j, vars, e1) };
        logln(format!("e2 null={}", e2.is_null()));
        unsafe { (vm.pop)(j, 1) };
        // js_newarguments requires an active call frame; exercise it here where
        // BOT > 0 because we are inside a cfunction.
        let newargs = sym!(vm, "js_newarguments", unsafe extern "C-unwind" fn(JsPtr));
        unsafe { newargs(j) };
        logln(format!("arguments={:?}", stack_snapshot(vm, j)));
        logln(format!("len={}", unsafe { (vm.getlength)(j, -1) }));
    };
    assert_same_protected(&cs, &rs, "jsR_newenvironment/js_newarguments", f);
}

#[test]
fn currentfunction_and_data_match() {
    let cs = Session::new(Side::C, 0);
    let rs = Session::new(Side::Rust, 0);
    let f = |vm: &Vm, j: JsPtr| {
        let curfun = sym!(vm, "js_currentfunction", unsafe extern "C-unwind" fn(JsPtr));
        let curdata = sym!(vm, "js_currentfunctiondata", unsafe extern "C-unwind" fn(JsPtr) -> *mut c_void);
        unsafe { curfun(j) };
        logln(format!("fn={:?}", stack_snapshot(vm, j)));
        logln(format!("data_null={}", unsafe { curdata(j) }.is_null()));
    };
    assert_same_protected(&cs, &rs, "js_currentfunction", f);
}

#[test]
fn intern_table_growth_matches() {
    let cs = Session::new(Side::C, 0);
    let rs = Session::new(Side::Rust, 0);
    let f = |vm: &Vm, j: JsPtr| {
        // Intern many strings in a deterministic order and check that repeated
        // interning is stable and that the same set of strings is produced.
        let mut ptrs = Vec::new();
        for i in 0..500 {
            let s = format!("interned-key-{}\0", i * 7919 % 500);
            let p = unsafe { (vm.intern)(j, s.as_ptr() as *const c_char) };
            ptrs.push((s.clone(), p));
            logln(unsafe { cstr_to_string(p) }.unwrap_or_default());
        }
        // second pass must return identical pointers
        for (s, p) in &ptrs {
            let q = unsafe { (vm.intern)(j, s.as_ptr() as *const c_char) };
            logln(format!("stable={}", q == *p));
        }
    };
    assert_same_protected(&cs, &rs, "js_intern growth", f);
}
