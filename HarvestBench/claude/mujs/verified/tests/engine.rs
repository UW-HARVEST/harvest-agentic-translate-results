//! Phase B/C — end-to-end differential tests driving the full engine via the
//! public API (js_newstate/js_dostring/js_getglobal/js_tostring) loaded from
//! BOTH .so files. CONFIGS rows 15-21, ERRORS rows 20-25.
mod common;
use common::Libs;
use std::os::raw::{c_char, c_int, c_double, c_void};

type NewstateFn = unsafe extern "C" fn(*mut c_void, *mut c_void, c_int) -> *mut c_void;
type FreestateFn = unsafe extern "C" fn(*mut c_void);
type DostringFn = unsafe extern "C" fn(*mut c_void, *const c_char) -> c_int;
type GetglobalFn = unsafe extern "C" fn(*mut c_void, *const c_char);
type TostringFn = unsafe extern "C" fn(*mut c_void, c_int) -> *const c_char;
type PopFn = unsafe extern "C" fn(*mut c_void, c_int);
type GettopFn = unsafe extern "C" fn(*mut c_void) -> c_int;
type PushnumberFn = unsafe extern "C" fn(*mut c_void, c_double);
type TonumberFn = unsafe extern "C" fn(*mut c_void, c_int) -> c_double;
type PushstringFn = unsafe extern "C" fn(*mut c_void, *const c_char);

const JS_STRICT: c_int = 1;

fn cstr(s: &str) -> Vec<c_char> {
    let mut v: Vec<c_char> = s.bytes().map(|b| b as c_char).collect();
    v.push(0);
    v
}

unsafe fn read_cstr(p: *const c_char) -> String {
    if p.is_null() { return "<null>".to_string(); }
    let mut s = String::new();
    let mut i = 0;
    loop {
        let c = *p.add(i);
        if c == 0 { break; }
        s.push(c as u8 as char);
        i += 1;
        if i > 1_000_000 { break; }
    }
    s
}

/// Run `script` in the given library, then evaluate `expr` (a JS expression),
/// store its String() into global `__result__`, and return that string plus
/// the dostring return code.
struct Engine<'a> {
    newstate: libloading::Symbol<'a, NewstateFn>,
    freestate: libloading::Symbol<'a, FreestateFn>,
    dostring: libloading::Symbol<'a, DostringFn>,
    getglobal: libloading::Symbol<'a, GetglobalFn>,
    tostring: libloading::Symbol<'a, TostringFn>,
    pop: libloading::Symbol<'a, PopFn>,
    gettop: libloading::Symbol<'a, GettopFn>,
    pushnumber: libloading::Symbol<'a, PushnumberFn>,
    tonumber: libloading::Symbol<'a, TonumberFn>,
    pushstring: libloading::Symbol<'a, PushstringFn>,
}

impl<'a> Engine<'a> {
    unsafe fn c(libs: &'a Libs) -> Engine<'a> {
        Engine {
            newstate: libs.c_sym(b"js_newstate"),
            freestate: libs.c_sym(b"js_freestate"),
            dostring: libs.c_sym(b"js_dostring"),
            getglobal: libs.c_sym(b"js_getglobal"),
            tostring: libs.c_sym(b"js_tostring"),
            pop: libs.c_sym(b"js_pop"),
            gettop: libs.c_sym(b"js_gettop"),
            pushnumber: libs.c_sym(b"js_pushnumber"),
            tonumber: libs.c_sym(b"js_tonumber"),
            pushstring: libs.c_sym(b"js_pushstring"),
        }
    }
    unsafe fn r(libs: &'a Libs) -> Engine<'a> {
        Engine {
            newstate: libs.rust_sym(b"js_newstate"),
            freestate: libs.rust_sym(b"js_freestate"),
            dostring: libs.rust_sym(b"js_dostring"),
            getglobal: libs.rust_sym(b"js_getglobal"),
            tostring: libs.rust_sym(b"js_tostring"),
            pop: libs.rust_sym(b"js_pop"),
            gettop: libs.rust_sym(b"js_gettop"),
            pushnumber: libs.rust_sym(b"js_pushnumber"),
            tonumber: libs.rust_sym(b"js_tonumber"),
            pushstring: libs.rust_sym(b"js_pushstring"),
        }
    }

    /// Evaluate an expression; return (dostring_ret, string_result).
    /// The script assigns String(expr) to global __r__.
    unsafe fn eval(&self, flags: c_int, expr: &str) -> (c_int, String) {
        let j = (self.newstate)(std::ptr::null_mut(), std::ptr::null_mut(), flags);
        assert!(!j.is_null(), "js_newstate returned null");
        let top0 = (self.gettop)(j);
        let script = format!("var __r__ = String({});", expr);
        let cs = cstr(&script);
        let ret = (self.dostring)(j, cs.as_ptr());
        let result = if ret == 0 {
            let name = cstr("__r__");
            (self.getglobal)(j, name.as_ptr());
            let sp = (self.tostring)(j, -1);
            let s = read_cstr(sp);
            (self.pop)(j, 1);
            s
        } else {
            String::new()
        };
        // stack should be balanced back to top0
        let _ = (self.gettop)(j);
        let _ = top0;
        (self.freestate)(j);
        (ret, result)
    }

    /// Run a raw script; return dostring return code only (error-path tests).
    unsafe fn run(&self, flags: c_int, script: &str) -> c_int {
        let j = (self.newstate)(std::ptr::null_mut(), std::ptr::null_mut(), flags);
        assert!(!j.is_null());
        let cs = cstr(script);
        let ret = (self.dostring)(j, cs.as_ptr());
        (self.freestate)(j);
        ret
    }

    /// FFI primitive round-trip: push a number, read it back.
    unsafe fn number_roundtrip(&self, flags: c_int, v: c_double) -> c_double {
        let j = (self.newstate)(std::ptr::null_mut(), std::ptr::null_mut(), flags);
        (self.pushnumber)(j, v);
        let out = (self.tonumber)(j, -1);
        (self.pop)(j, 1);
        (self.freestate)(j);
        out
    }

    /// FFI string push + tostring round-trip.
    unsafe fn string_roundtrip(&self, flags: c_int, s: &str) -> String {
        let j = (self.newstate)(std::ptr::null_mut(), std::ptr::null_mut(), flags);
        let cs = cstr(s);
        (self.pushstring)(j, cs.as_ptr());
        let sp = (self.tostring)(j, -1);
        let out = read_cstr(sp);
        (self.pop)(j, 1);
        (self.freestate)(j);
        out
    }
}

type ConvI32Fn = unsafe extern "C" fn(*mut c_void, c_int) -> c_int;
type ConvU32Fn = unsafe extern "C" fn(*mut c_void, c_int) -> u32;
type ConvI16Fn = unsafe extern "C" fn(*mut c_void, c_int) -> i16;
type ConvU16Fn = unsafe extern "C" fn(*mut c_void, c_int) -> u16;

#[test]
fn engine_number_conversions_differential() {
    // ECMAScript ToInt32/ToUint32/ToInt16/ToUint16/ToInteger have specific
    // modular-wrapping semantics that are value-dependent — a classic place
    // for a translation to diverge. Push a number, then convert via the .so.
    let libs = Libs::load();
    unsafe {
        let cnew: libloading::Symbol<NewstateFn> = libs.c_sym(b"js_newstate");
        let rnew: libloading::Symbol<NewstateFn> = libs.rust_sym(b"js_newstate");
        let cfree: libloading::Symbol<FreestateFn> = libs.c_sym(b"js_freestate");
        let rfree: libloading::Symbol<FreestateFn> = libs.rust_sym(b"js_freestate");
        let cpush: libloading::Symbol<PushnumberFn> = libs.c_sym(b"js_pushnumber");
        let rpush: libloading::Symbol<PushnumberFn> = libs.rust_sym(b"js_pushnumber");
        let cpop: libloading::Symbol<PopFn> = libs.c_sym(b"js_pop");
        let rpop: libloading::Symbol<PopFn> = libs.rust_sym(b"js_pop");

        let cj = cnew(std::ptr::null_mut(), std::ptr::null_mut(), 0);
        let rj = rnew(std::ptr::null_mut(), std::ptr::null_mut(), 0);

        let ci32: libloading::Symbol<ConvI32Fn> = libs.c_sym(b"js_toint32");
        let ri32: libloading::Symbol<ConvI32Fn> = libs.rust_sym(b"js_toint32");
        let cu32: libloading::Symbol<ConvU32Fn> = libs.c_sym(b"js_touint32");
        let ru32: libloading::Symbol<ConvU32Fn> = libs.rust_sym(b"js_touint32");
        let ci16: libloading::Symbol<ConvI16Fn> = libs.c_sym(b"js_toint16");
        let ri16: libloading::Symbol<ConvI16Fn> = libs.rust_sym(b"js_toint16");
        let cu16: libloading::Symbol<ConvU16Fn> = libs.c_sym(b"js_touint16");
        let ru16: libloading::Symbol<ConvU16Fn> = libs.rust_sym(b"js_touint16");
        let ci: libloading::Symbol<ConvI32Fn> = libs.c_sym(b"js_tointeger");
        let ri: libloading::Symbol<ConvI32Fn> = libs.rust_sym(b"js_tointeger");

        let mut check = |v: c_double| {
            cpush(cj, v); rpush(rj, v);
            assert_eq!(ci32(cj, -1), ri32(rj, -1), "toint32 v={}", v);
            assert_eq!(cu32(cj, -1), ru32(rj, -1), "touint32 v={}", v);
            assert_eq!(ci16(cj, -1), ri16(rj, -1), "toint16 v={}", v);
            assert_eq!(cu16(cj, -1), ru16(rj, -1), "touint16 v={}", v);
            assert_eq!(ci(cj, -1), ri(rj, -1), "tointeger v={}", v);
            cpop(cj, 1); rpop(rj, 1);
        };
        let boundaries = [
            0.0, -0.0, 1.0, -1.0, 0.5, -0.5, 2147483647.0, 2147483648.0,
            4294967295.0, 4294967296.0, -2147483648.0, -2147483649.0,
            65535.0, 65536.0, 32767.0, 32768.0, -32768.0, -32769.0,
            1e21, -1e21, 3.9, -3.9, f64::INFINITY, f64::NEG_INFINITY,
            f64::NAN, 1e300, 4294967297.5, 123456789.99,
        ];
        for v in boundaries { check(v); }
        let mut seed: u64 = 0x1234_5678_9abc_def0;
        for _ in 0..50_000 {
            seed ^= seed << 13; seed ^= seed >> 7; seed ^= seed << 17;
            // spread across a wide dynamic range
            let m = (seed >> 11) as f64 / (1u64 << 53) as f64 - 0.5;
            let e = ((seed & 0x7f) as i32) - 40;
            check(m * 2f64.powi(e) * 5e9);
        }
        cfree(cj); rfree(rj);
    }
}

fn expressions() -> Vec<&'static str> {
    vec![
        // arithmetic
        "1+2*3", "(1+2)*3", "10/3", "10%3", "2**10", "-5*-5", "0.1+0.2",
        "1e100*1e100", "1/0", "-1/0", "0/0", "Math.PI", "Math.E",
        // Math library
        "Math.sqrt(2)", "Math.pow(2,0.5)", "Math.floor(3.7)", "Math.ceil(3.2)",
        "Math.abs(-42.5)", "Math.max(1,2,3)", "Math.min(4,5,6)", "Math.round(2.5)",
        "Math.sin(1)", "Math.cos(1)", "Math.log(10)", "Math.exp(2)", "Math.atan2(1,1)",
        // Number formatting
        "(255).toString(16)", "(255).toString(2)", "(3.14159).toFixed(2)",
        "(12345.6789).toPrecision(6)", "(1000000).toExponential(3)",
        "(0.000123).toString()", "(1e21).toString()", "(123456789012345680000).toString()",
        "parseInt('0xFF')", "parseInt('101',2)", "parseFloat('3.14abc')",
        "Number('  42  ')", "Number('0x10')", "Number('')", "Number('abc')",
        "(-0).toString()", "(NaN).toString()", "(Infinity).toString()",
        // strings
        "'hello'.length", "'hello'.toUpperCase()", "'WORLD'.toLowerCase()",
        "'a,b,c'.split(',').join('|')", "'  trim  '.trim()",
        "'abcdef'.substring(1,4)", "'abcdef'.slice(-3)", "'abc'.charCodeAt(1)",
        "String.fromCharCode(72,105)", "'foobar'.indexOf('bar')",
        "'aaa'.replace('a','b')", "'a1b2c3'.replace(/[0-9]/g,'#')",
        "'x'.repeat(5)", "'Hello World'.match(/o/g).length",
        // arrays
        "[1,2,3].map(function(x){return x*2}).join(',')",
        "[3,1,2].sort().join(',')", "[1,2,3,4].filter(function(x){return x%2==0}).join(',')",
        "[1,2,3].reduce(function(a,b){return a+b},0)",
        "[1,2,3].reverse().join(',')", "[1,2,3].concat([4,5]).length",
        "['a','b','c'].indexOf('b')", "[1,2,3].slice(1).join(',')",
        "Array(5).length", "[1,[2,[3]]].toString()",
        // JSON
        "JSON.stringify({a:1,b:[2,3],c:'x'})", "JSON.parse('{\"x\":42}').x",
        "JSON.stringify([1,true,null,'s'])", "JSON.stringify({nested:{deep:[1,2]}})",
        "JSON.parse('[1,2,3]').length",
        // regexp via JS
        "/\\d+/.test('abc123')", "'2024-01-15'.match(/(\\d+)-(\\d+)-(\\d+)/)[2]",
        "'one two three'.split(/\\s+/).length", "'aAbBcC'.replace(/[a-z]/g,'_')",
        // booleans / logic
        "true && false", "true || false", "!true", "1<2", "2<=2", "'a'<'b'",
        "1==1", "1==='1'", "null==undefined", "NaN===NaN",
        // encode/decode
        "encodeURIComponent('a b&c')", "decodeURIComponent('a%20b')",
        "encodeURI('http://x.com/a b')", "escape('a b')", "unescape('a%20b')",
        // type coercion
        "typeof 42", "typeof 'x'", "typeof true", "typeof undefined",
        "typeof null", "typeof {}", "typeof [].push",
        "'' + [1,2,3]", "+ '42'", "!!''", "!!'x'",
    ]
}

#[test]
fn engine_expressions_differential() {
    let libs = Libs::load();
    unsafe {
        let ec = Engine::c(&libs);
        let er = Engine::r(&libs);
        for flags in [0, JS_STRICT] {
            for expr in expressions() {
                let (cret, cval) = ec.eval(flags, expr);
                let (rret, rval) = er.eval(flags, expr);
                assert_eq!(cret, rret, "dostring ret differ flags={} expr={:?}", flags, expr);
                assert_eq!(cval, rval, "value differ flags={} expr={:?}", flags, expr);
            }
        }
    }
}

// The JS call stack overflow guard (JS_ENVLIMIT=1024) fires only after ~1024
// nested native jsR_run frames. In debug builds Rust frames are large, so we
// run on a thread with a big stack to reach the same guard the C reaches on the
// process main stack — otherwise the OS stack overflows first (a harness
// artifact, not a behavior difference).
fn on_big_stack<F: FnOnce() + Send + 'static>(f: F) {
    std::thread::Builder::new()
        .stack_size(1024 * 1024 * 1024)
        .spawn(f)
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn engine_error_paths_differential() {
    on_big_stack(|| engine_error_paths_differential_inner());
}

fn engine_error_paths_differential_inner() {
    let libs = Libs::load();
    unsafe {
        let ec = Engine::c(&libs);
        let er = Engine::r(&libs);
        // ERRORS rows 20-25 + generic throws. dostring returns 1 on throw.
        let bad_scripts = [
            "var =",                       // syntax error
            "1 +* 2",                      // syntax error
            "function(){",                 // syntax error
            "nosuchvariable",              // reference error
            "null.x",                      // type error
            "undefined.foo()",             // type error
            "(1).toFixed(999)",            // range error
            "decodeURI('%')",              // URI error
            "decodeURIComponent('%GG')",   // URI error
            "throw new Error('boom')",     // explicit throw
            "throw 'string error'",        // throw primitive
            "[].reduce(function(a,b){return a})", // TypeError reduce empty no init
            "JSON.parse('{invalid}')",     // syntax error in JSON
            "var x = {}; x.y.z",           // type error (undefined prop access)
            "(function f(){return f()})()",// call stack overflow / recursion
        ];
        for s in bad_scripts {
            for flags in [0, JS_STRICT] {
                let cret = ec.run(flags, s);
                let rret = er.run(flags, s);
                assert_eq!(cret, rret, "error ret differ flags={} script={:?}", flags, s);
            }
        }
        // valid scripts should both return 0
        for s in ["var a = 1;", "1+1;", "function f(){return 5} f();"] {
            assert_eq!(ec.run(0, s), 0, "valid script C: {:?}", s);
            assert_eq!(er.run(0, s), 0, "valid script Rust: {:?}", s);
        }
    }
}

#[test]
fn engine_ffi_primitive_roundtrips() {
    let libs = Libs::load();
    unsafe {
        let ec = Engine::c(&libs);
        let er = Engine::r(&libs);
        for v in [0.0, 1.0, -1.0, 3.14159, 1e100, -1e-100, f64::INFINITY,
                  f64::NEG_INFINITY, 123456.789, -0.0] {
            let cv = ec.number_roundtrip(0, v);
            let rv = er.number_roundtrip(0, v);
            assert!((cv.is_nan() && rv.is_nan()) || cv.to_bits() == rv.to_bits(),
                "number roundtrip v={} c={} r={}", v, cv, rv);
        }
        let cn = ec.number_roundtrip(0, f64::NAN);
        let rn = er.number_roundtrip(0, f64::NAN);
        assert_eq!(cn.is_nan(), rn.is_nan(), "NaN roundtrip");

        for s in ["", "hello", "unicode: \u{00e9}\u{4e2d}\u{6587}", "with\ttabs\nnewlines",
                  "quote\"backslash\\", "a".repeat(100).as_str()] {
            let cs = ec.string_roundtrip(0, s);
            let rs = er.string_roundtrip(0, s);
            assert_eq!(cs, rs, "string roundtrip {:?}", s);
        }
    }
}
