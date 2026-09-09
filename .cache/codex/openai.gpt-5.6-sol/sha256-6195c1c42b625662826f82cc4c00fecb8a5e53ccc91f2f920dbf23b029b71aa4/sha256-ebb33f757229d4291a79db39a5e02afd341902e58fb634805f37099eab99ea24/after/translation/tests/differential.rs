use libloading::Library;
use std::ffi::{CStr, CString, c_char, c_double, c_int, c_uint, c_void};
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::Once;

#[repr(C)]
struct JsState {
    _private: [u8; 0],
}

type NewState = unsafe extern "C" fn(
    Option<unsafe extern "C" fn(*mut c_void, *mut c_void, c_int) -> *mut c_void>,
    *mut c_void,
    c_int,
) -> *mut JsState;
type FreeState = unsafe extern "C" fn(*mut JsState);
type PLoadString = unsafe extern "C" fn(*mut JsState, *const c_char, *const c_char) -> c_int;
type PCall = unsafe extern "C" fn(*mut JsState, c_int) -> c_int;
type PushUndefined = unsafe extern "C" fn(*mut JsState);
type TryRepr = unsafe extern "C" fn(*mut JsState, c_int, *const c_char) -> *const c_char;
type GetTop = unsafe extern "C" fn(*mut JsState) -> c_int;
type Pop = unsafe extern "C" fn(*mut JsState, c_int);
type PushNull = unsafe extern "C" fn(*mut JsState);
type PushBoolean = unsafe extern "C" fn(*mut JsState, c_int);
type PushNumber = unsafe extern "C" fn(*mut JsState, c_double);
type PushString = unsafe extern "C" fn(*mut JsState, *const c_char);
type PushLString = unsafe extern "C" fn(*mut JsState, *const c_char, c_int);
type ToBoolean = unsafe extern "C" fn(*mut JsState, c_int) -> c_int;
type ToNumber = unsafe extern "C" fn(*mut JsState, c_int) -> c_double;
type ToInteger = unsafe extern "C" fn(*mut JsState, c_int) -> c_int;
type ToInt32 = unsafe extern "C" fn(*mut JsState, c_int) -> c_int;
type ToUint32 = unsafe extern "C" fn(*mut JsState, c_int) -> c_uint;
type ToInt16 = unsafe extern "C" fn(*mut JsState, c_int) -> i16;
type ToUint16 = unsafe extern "C" fn(*mut JsState, c_int) -> u16;
type Type = unsafe extern "C" fn(*mut JsState, c_int) -> c_int;
type TypeOf = unsafe extern "C" fn(*mut JsState, c_int) -> *const c_char;
type NewArray = unsafe extern "C" fn(*mut JsState);
type SetIndex = unsafe extern "C" fn(*mut JsState, c_int, c_int);
type GetIndex = unsafe extern "C" fn(*mut JsState, c_int, c_int);
type HasIndex = unsafe extern "C" fn(*mut JsState, c_int, c_int) -> c_int;
type DelIndex = unsafe extern "C" fn(*mut JsState, c_int, c_int);
type GetLength = unsafe extern "C" fn(*mut JsState, c_int) -> c_int;
type SetLength = unsafe extern "C" fn(*mut JsState, c_int, c_int);
type NewObject = unsafe extern "C" fn(*mut JsState);
type SetProperty = unsafe extern "C" fn(*mut JsState, c_int, *const c_char);
type GetProperty = unsafe extern "C" fn(*mut JsState, c_int, *const c_char);
type HasProperty = unsafe extern "C" fn(*mut JsState, c_int, *const c_char) -> c_int;
type DelProperty = unsafe extern "C" fn(*mut JsState, c_int, *const c_char);
type DefProperty = unsafe extern "C" fn(*mut JsState, c_int, *const c_char, c_int);
type SetContext = unsafe extern "C" fn(*mut JsState, *mut c_void);
type GetContext = unsafe extern "C" fn(*mut JsState) -> *mut c_void;
type Gc = unsafe extern "C" fn(*mut JsState, c_int);
type SetLimit = unsafe extern "C" fn(*mut JsState, c_int, c_int);
type NewRegexp = unsafe extern "C" fn(*mut JsState, *const c_char, c_int);
type IsArrayIndex = unsafe extern "C" fn(*mut JsState, *const c_char, *mut c_int) -> c_int;

struct Api {
    _lib: Library,
    newstate: NewState,
    freestate: FreeState,
    ploadstring: PLoadString,
    pcall: PCall,
    pushundefined: PushUndefined,
    tryrepr: TryRepr,
    gettop: GetTop,
    pop: Pop,
    pushnull: PushNull,
    pushboolean: PushBoolean,
    pushnumber: PushNumber,
    pushstring: PushString,
    pushlstring: PushLString,
    toboolean: ToBoolean,
    tonumber: ToNumber,
    tointeger: ToInteger,
    toint32: ToInt32,
    touint32: ToUint32,
    toint16: ToInt16,
    touint16: ToUint16,
    js_type: Type,
    typeof_: TypeOf,
    newarray: NewArray,
    setindex: SetIndex,
    getindex: GetIndex,
    hasindex: HasIndex,
    delindex: DelIndex,
    getlength: GetLength,
    setlength: SetLength,
    newobject: NewObject,
    setproperty: SetProperty,
    getproperty: GetProperty,
    hasproperty: HasProperty,
    delproperty: DelProperty,
    defproperty: DefProperty,
    setcontext: SetContext,
    getcontext: GetContext,
    gc: Gc,
    setlimit: SetLimit,
    newregexp: NewRegexp,
    isarrayindex: IsArrayIndex,
}

unsafe fn load_symbol<T: Copy>(lib: &Library, name: &[u8]) -> T {
    unsafe { *lib.get::<T>(name).unwrap() }
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let lib = unsafe { Library::new(path).unwrap() };
        Self {
            newstate: unsafe { load_symbol(&lib, b"js_newstate\0") },
            freestate: unsafe { load_symbol(&lib, b"js_freestate\0") },
            ploadstring: unsafe { load_symbol(&lib, b"js_ploadstring\0") },
            pcall: unsafe { load_symbol(&lib, b"js_pcall\0") },
            pushundefined: unsafe { load_symbol(&lib, b"js_pushundefined\0") },
            tryrepr: unsafe { load_symbol(&lib, b"js_tryrepr\0") },
            gettop: unsafe { load_symbol(&lib, b"js_gettop\0") },
            pop: unsafe { load_symbol(&lib, b"js_pop\0") },
            pushnull: unsafe { load_symbol(&lib, b"js_pushnull\0") },
            pushboolean: unsafe { load_symbol(&lib, b"js_pushboolean\0") },
            pushnumber: unsafe { load_symbol(&lib, b"js_pushnumber\0") },
            pushstring: unsafe { load_symbol(&lib, b"js_pushstring\0") },
            pushlstring: unsafe { load_symbol(&lib, b"js_pushlstring\0") },
            toboolean: unsafe { load_symbol(&lib, b"js_toboolean\0") },
            tonumber: unsafe { load_symbol(&lib, b"js_tonumber\0") },
            tointeger: unsafe { load_symbol(&lib, b"js_tointeger\0") },
            toint32: unsafe { load_symbol(&lib, b"js_toint32\0") },
            touint32: unsafe { load_symbol(&lib, b"js_touint32\0") },
            toint16: unsafe { load_symbol(&lib, b"js_toint16\0") },
            touint16: unsafe { load_symbol(&lib, b"js_touint16\0") },
            js_type: unsafe { load_symbol(&lib, b"js_type\0") },
            typeof_: unsafe { load_symbol(&lib, b"js_typeof\0") },
            newarray: unsafe { load_symbol(&lib, b"js_newarray\0") },
            setindex: unsafe { load_symbol(&lib, b"js_setindex\0") },
            getindex: unsafe { load_symbol(&lib, b"js_getindex\0") },
            hasindex: unsafe { load_symbol(&lib, b"js_hasindex\0") },
            delindex: unsafe { load_symbol(&lib, b"js_delindex\0") },
            getlength: unsafe { load_symbol(&lib, b"js_getlength\0") },
            setlength: unsafe { load_symbol(&lib, b"js_setlength\0") },
            newobject: unsafe { load_symbol(&lib, b"js_newobject\0") },
            setproperty: unsafe { load_symbol(&lib, b"js_setproperty\0") },
            getproperty: unsafe { load_symbol(&lib, b"js_getproperty\0") },
            hasproperty: unsafe { load_symbol(&lib, b"js_hasproperty\0") },
            delproperty: unsafe { load_symbol(&lib, b"js_delproperty\0") },
            defproperty: unsafe { load_symbol(&lib, b"js_defproperty\0") },
            setcontext: unsafe { load_symbol(&lib, b"js_setcontext\0") },
            getcontext: unsafe { load_symbol(&lib, b"js_getcontext\0") },
            gc: unsafe { load_symbol(&lib, b"js_gc\0") },
            setlimit: unsafe { load_symbol(&lib, b"js_setlimit\0") },
            newregexp: unsafe { load_symbol(&lib, b"js_newregexp\0") },
            isarrayindex: unsafe { load_symbol(&lib, b"js_isarrayindex\0") },
            _lib: lib,
        }
    }

    fn state(&self, flags: c_int) -> *mut JsState {
        let state = unsafe { (self.newstate)(None, ptr::null_mut(), flags) };
        assert!(!state.is_null());
        state
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so() -> PathBuf {
    manifest_dir().join("../c_src/build/libmujs.so")
}

fn rust_so() -> PathBuf {
    manifest_dir().join("target/release/libmujs.so")
}

fn ensure_libm_is_global() {
    static LOAD_LIBM: Once = Once::new();
    LOAD_LIBM.call_once(|| unsafe {
        let library = libloading::os::unix::Library::open(
            Some("libm.so.6"),
            libloading::os::unix::RTLD_NOW | libloading::os::unix::RTLD_GLOBAL,
        )
        .unwrap();
        std::mem::forget(library);
    });
}

fn load_pair() -> (Api, Api) {
    assert!(c_so().is_file(), "build the C shared library first");
    assert!(rust_so().is_file(), "run cargo build --release first");
    ensure_libm_is_global();
    unsafe { (Api::load(&c_so()), Api::load(&rust_so())) }
}

#[derive(Debug, PartialEq, Eq)]
struct EvalResult {
    load_status: c_int,
    call_status: c_int,
    top: c_int,
    repr: Vec<u8>,
}

fn top_repr(api: &Api, state: *mut JsState) -> Vec<u8> {
    let fallback = b"<repr-error>\0";
    let value = unsafe { (api.tryrepr)(state, -1, fallback.as_ptr().cast()) };
    assert!(!value.is_null());
    unsafe { CStr::from_ptr(value).to_bytes().to_vec() }
}

fn eval(api: &Api, flags: c_int, source: &str) -> EvalResult {
    let state = api.state(flags);
    let filename = b"[differential]\0";
    let source = CString::new(source).unwrap();
    let load_status =
        unsafe { (api.ploadstring)(state, filename.as_ptr().cast(), source.as_ptr()) };
    let call_status = if load_status == 0 {
        unsafe { (api.pushundefined)(state) };
        unsafe { (api.pcall)(state, 0) }
    } else {
        -1
    };
    let top = unsafe { (api.gettop)(state) };
    let repr = if top > 0 {
        top_repr(api, state)
    } else {
        Vec::new()
    };
    unsafe { (api.freestate)(state) };
    EvalResult {
        load_status,
        call_status,
        top,
        repr,
    }
}

fn assert_eval_equal(c: &Api, rust: &Api, flags: c_int, source: &str) {
    let c_result = eval(c, flags, source);
    let rust_result = eval(rust, flags, source);
    assert_eq!(c_result, rust_result, "source:\n{source}");
}

#[test]
fn every_c_dynamic_symbol_is_present_in_rust() {
    ensure_libm_is_global();
    let c_lib = unsafe { Library::new(c_so()).unwrap() };
    let rust_lib = unsafe { Library::new(rust_so()).unwrap() };
    let symbols = std::fs::read_to_string(manifest_dir().join("symbols.txt")).unwrap();
    let mut count = 0;
    for symbol in symbols.lines().filter(|line| !line.is_empty()) {
        let name = CString::new(symbol).unwrap();
        unsafe {
            c_lib
                .get::<unsafe extern "C" fn()>(name.as_bytes_with_nul())
                .unwrap();
            rust_lib
                .get::<unsafe extern "C" fn()>(name.as_bytes_with_nul())
                .unwrap();
        }
        count += 1;
    }
    assert_eq!(count, 237);
}

#[test]
fn valid_script_corpus_and_randomized_inputs_match() {
    let (c, rust) = load_pair();
    let corpus = [
        "",
        "undefined",
        "null",
        "true",
        "false",
        "0",
        "-0",
        "NaN",
        "Infinity",
        "-Infinity",
        "'ascii'",
        "'héllö 世界'",
        "'a\\0b'.length",
        "[].length",
        "[1].length",
        "[1,,3].length",
        "({a:1,b:'x',c:true})",
        "(function(a,b){return a+b})(2,3)",
        "(function(){var s=0; for(var i=0;i<20;i++)s+=i; return s})()",
        "(function(){var x=0; while(x<7)++x; return x})()",
        "(function(){switch(3){case 2:return 'a';case 3:return 'b';default:return 'c'}})()",
        "(function(){try{throw 7}catch(e){return e}finally{var x=1}})()",
        "JSON.stringify({a:[1,null,'x'],b:true})",
        "JSON.parse('{\"a\":[1,2,3]}').a.join('-')",
        "/a+/g.exec('caaab')[0]",
        "/^b/im.test('a\\nB\\nc')",
        "'abc'.replace(/b/,'X')",
        "'a,b,c'.split(',').reverse().join(':')",
        "[3,1,2].sort().join(',')",
        "[1,2,3,4].map(function(x){return x*x}).join(',')",
        "[1,2,3,4].filter(function(x){return x%2}).join(',')",
        "[1,2,3,4].reduce(function(a,b){return a+b},0)",
        "Object.keys({z:1,a:2}).sort().join(',')",
        "(function(){var p={x:1};var o=Object.create(p);o.y=2;return o.x+o.y})()",
        "(255).toString(16)",
        "(1.25).toFixed(3)",
        "(1.25).toExponential(2)",
        "(1.25).toPrecision(3)",
        "parseInt('ff',16)",
        "parseFloat('  -12.5e2x')",
        "isNaN('x')",
        "encodeURIComponent('a b/世界')",
        "decodeURIComponent('a%20b%2F%E4%B8%96%E7%95%8C')",
        "new Date(0).toISOString()",
        "Date.parse('2000-02-29T12:34:56.789Z')",
        "(function(){'use strict';return this===undefined})()",
        "(function(){var a=[];a[100]=3;return [a.length,a[100],0 in a].join(':')})()",
        "(function(){var o={};Object.defineProperty(o,'x',{value:3,enumerable:false});return o.x+':'+Object.keys(o).length})()",
        "(function(){function F(){};var x=new F();return x instanceof F})()",
        "(function(){return typeof undefined+','+typeof null+','+typeof 1+','+typeof 'x'+','+typeof function(){}})()",
    ];
    for flags in [0, 1, 0x4000_0000] {
        for source in corpus {
            assert_eval_equal(&c, &rust, flags, source);
        }
    }

    let mut seed = 0x4d595df4d0f33173_u64;
    for iteration in 0..500 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let a = ((seed >> 16) as i32 % 200_001) - 100_000;
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let b = ((seed >> 16) as i32 % 200_001) - 100_000;
        let divisor = if b == 0 { 1 } else { b };
        let source = format!(
            "(function(){{var a={a},b={b},d={divisor};return [a+b,a-b,a*b,a/d,a%d,(a|0)^(b|0),Math.abs(a),Math.max(a,b),String(a),parseInt(String(b),10)].join('|')}})()"
        );
        assert_eval_equal(&c, &rust, iteration & 1, &source);
    }
}

fn direct_snapshot(api: &Api) -> Vec<String> {
    let state = api.state(0x4000_0000);
    let mut out = Vec::new();
    let marker = 0x1234usize as *mut c_void;
    unsafe { (api.setcontext)(state, marker) };
    out.push(format!("ctx={:x}", unsafe { (api.getcontext)(state) } as usize));

    unsafe {
        (api.pushundefined)(state);
        (api.pushnull)(state);
        (api.pushboolean)(state, 0);
        (api.pushboolean)(state, 7);
    }
    for value in [
        -0.0,
        0.0,
        1.5,
        -1.5,
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        2147483648.0,
        -2147483649.0,
    ] {
        unsafe { (api.pushnumber)(state, value) };
    }
    let short = CString::new("short").unwrap();
    unsafe { (api.pushstring)(state, short.as_ptr()) };
    let embedded = b"a\0b";
    unsafe { (api.pushlstring)(state, embedded.as_ptr().cast(), embedded.len() as c_int) };

    let top = unsafe { (api.gettop)(state) };
    out.push(format!("top={top}"));
    for index in 0..top {
        let kind = unsafe { (api.js_type)(state, index) };
        let typeof_ptr = unsafe { (api.typeof_)(state, index) };
        let typeof_bytes = unsafe { CStr::from_ptr(typeof_ptr).to_bytes() };
        let truth = unsafe { (api.toboolean)(state, index) };
        let number = unsafe { (api.tonumber)(state, index) };
        let integer = unsafe { (api.tointeger)(state, index) };
        let i32v = unsafe { (api.toint32)(state, index) };
        let u32v = unsafe { (api.touint32)(state, index) };
        let i16v = unsafe { (api.toint16)(state, index) };
        let u16v = unsafe { (api.touint16)(state, index) };
        out.push(format!(
            "{index}:{kind}:{}:{truth}:{:016x}:{integer}:{i32v}:{u32v}:{i16v}:{u16v}",
            String::from_utf8_lossy(typeof_bytes),
            number.to_bits()
        ));
    }
    for index in [c_int::MIN, -1000, top, 1000, c_int::MAX] {
        out.push(format!(
            "oob:{index}:{}:{}",
            unsafe { (api.js_type)(state, index) },
            unsafe { CStr::from_ptr((api.typeof_)(state, index)).to_string_lossy() }
        ));
    }
    unsafe { (api.pop)(state, top) };

    unsafe { (api.newarray)(state) };
    for (index, value) in [(0, 10.0), (1, 20.0), (5, 60.0)] {
        unsafe {
            (api.pushnumber)(state, value);
            (api.setindex)(state, -2, index);
        }
    }
    out.push(format!("array-length={}", unsafe {
        (api.getlength)(state, -1)
    }));
    for index in [-1, 0, 1, 2, 5, 6] {
        let has = unsafe { (api.hasindex)(state, -1, index) };
        unsafe { (api.getindex)(state, -1, index) };
        out.push(format!("index:{index}:{has}:{:?}", top_repr(api, state)));
        unsafe { (api.pop)(state, 1) };
    }
    unsafe {
        (api.delindex)(state, -1, 1);
        (api.setlength)(state, -1, 3);
    }
    out.push(format!(
        "array-after={}:{:?}",
        unsafe { (api.getlength)(state, -1) },
        top_repr(api, state)
    ));
    unsafe { (api.pop)(state, 1) };

    let x = CString::new("x").unwrap();
    let missing = CString::new("missing").unwrap();
    unsafe {
        (api.newobject)(state);
        (api.pushnumber)(state, 42.0);
        (api.setproperty)(state, -2, x.as_ptr());
    }
    out.push(format!(
        "props={}:{}",
        unsafe { (api.hasproperty)(state, -1, x.as_ptr()) },
        unsafe { (api.hasproperty)(state, -1, missing.as_ptr()) }
    ));
    unsafe { (api.getproperty)(state, -1, x.as_ptr()) };
    out.push(format!("x={:?}", top_repr(api, state)));
    unsafe { (api.pop)(state, 1) };
    for atts in 0..16 {
        let name = CString::new(format!("p{atts}")).unwrap();
        unsafe {
            (api.pushnumber)(state, atts as f64);
            (api.defproperty)(state, -2, name.as_ptr(), atts);
        }
    }
    out.push(format!("object={:?}", top_repr(api, state)));
    out.push(format!("del={}", unsafe {
        (api.delproperty)(state, -1, x.as_ptr());
        (api.hasproperty)(state, -1, x.as_ptr())
    }));

    for text in ["", "0", "00", "1", "2147483647", "2147483648", "-1", "1x"] {
        let text = CString::new(text).unwrap();
        let mut index = -77;
        let valid = unsafe { (api.isarrayindex)(state, text.as_ptr(), &mut index) };
        out.push(format!("array-index:{valid}:{index}"));
    }

    let pattern = CString::new("a+").unwrap();
    for flags in 0..16 {
        unsafe { (api.newregexp)(state, pattern.as_ptr(), flags) };
        out.push(format!("regexp:{flags}:{:?}", top_repr(api, state)));
        unsafe { (api.pop)(state, 1) };
    }
    unsafe {
        (api.gc)(state, 0);
        (api.gc)(state, 1);
        (api.setlimit)(state, 0, 0);
        (api.freestate)(state);
    }
    out
}

#[test]
fn direct_state_stack_property_and_boundary_calls_match() {
    let (c, rust) = load_pair();
    assert_eq!(direct_snapshot(&c), direct_snapshot(&rust));
}

type Strtod = unsafe extern "C" fn(*const c_char, *mut *mut c_char) -> c_double;
type Strtol = unsafe extern "C" fn(*const c_char, *mut *mut c_char, c_int) -> c_double;
type StringToFloat = unsafe extern "C" fn(*const c_char, *mut *mut c_char) -> c_double;
type Itoa = unsafe extern "C" fn(*mut c_char, c_int) -> *const c_char;
type RuneUnary = unsafe extern "C" fn(c_int) -> c_int;
type CharToRune = unsafe extern "C" fn(*mut c_int, *const c_char) -> c_int;
type RuneToChar = unsafe extern "C" fn(*mut c_char, *const c_int) -> c_int;
type UtfLen = unsafe extern "C" fn(*const c_char) -> c_int;
type UtfPtrToIdx = unsafe extern "C" fn(*const c_char, *const c_char) -> c_int;

fn numeric_snapshot(lib: &Library) -> Vec<String> {
    let strtod: Strtod = unsafe { load_symbol(lib, b"js_strtod\0") };
    let strtol: Strtol = unsafe { load_symbol(lib, b"js_strtol\0") };
    let stringtofloat: StringToFloat = unsafe { load_symbol(lib, b"js_stringtofloat\0") };
    let itoa: Itoa = unsafe { load_symbol(lib, b"js_itoa\0") };
    let mut out = Vec::new();
    let samples = [
        "",
        " ",
        "0",
        "-0",
        "+0",
        "1",
        "-1",
        ".5",
        "1.",
        "1.25",
        "1e3",
        "1e-300",
        "1e309",
        "1234567890123456789012345",
        "0x10",
        "Infinity",
        "-Infinity",
        "NaN",
        "12tail",
    ];
    for sample in samples {
        let input = CString::new(sample).unwrap();
        for (name, function) in [("strtod", strtod), ("float", stringtofloat)] {
            let mut end = ptr::null_mut();
            let value = unsafe { function(input.as_ptr(), &mut end) };
            let offset = unsafe { end.offset_from(input.as_ptr().cast_mut()) };
            out.push(format!("{name}:{sample}:{:016x}:{offset}", value.to_bits()));
        }
        for base in [0, 2, 8, 10, 16, 36, -1, 1, 37] {
            let mut end = ptr::null_mut();
            let value = unsafe { strtol(input.as_ptr(), &mut end, base) };
            let offset = unsafe { end.offset_from(input.as_ptr().cast_mut()) };
            out.push(format!(
                "strtol:{base}:{sample}:{:016x}:{offset}",
                value.to_bits()
            ));
        }
    }
    for value in [
        c_int::MIN,
        c_int::MIN + 1,
        -1,
        0,
        1,
        c_int::MAX - 1,
        c_int::MAX,
    ] {
        let mut buffer = [0_i8; 64];
        let result = unsafe { itoa(buffer.as_mut_ptr(), value) };
        out.push(format!("itoa:{value}:{}", unsafe {
            CStr::from_ptr(result).to_string_lossy()
        }));
    }
    out
}

fn utf_snapshot(lib: &Library) -> Vec<String> {
    let chartorune: CharToRune = unsafe { load_symbol(lib, b"jsU_chartorune\0") };
    let runetochar: RuneToChar = unsafe { load_symbol(lib, b"jsU_runetochar\0") };
    let runelen: RuneUnary = unsafe { load_symbol(lib, b"jsU_runelen\0") };
    let lower: RuneUnary = unsafe { load_symbol(lib, b"jsU_tolowerrune\0") };
    let upper: RuneUnary = unsafe { load_symbol(lib, b"jsU_toupperrune\0") };
    let isalpha: RuneUnary = unsafe { load_symbol(lib, b"jsU_isalpharune\0") };
    let islower: RuneUnary = unsafe { load_symbol(lib, b"jsU_islowerrune\0") };
    let isupper: RuneUnary = unsafe { load_symbol(lib, b"jsU_isupperrune\0") };
    let utflen: UtfLen = unsafe { load_symbol(lib, b"js_utflen\0") };
    let utfptrtoidx: UtfPtrToIdx = unsafe { load_symbol(lib, b"js_utfptrtoidx\0") };
    let mut out = Vec::new();
    let byte_samples: &[&[u8]] = &[
        b"",
        b"A",
        "é".as_bytes(),
        "世".as_bytes(),
        "😀".as_bytes(),
        &[0xc0, 0x80],
        &[0x80],
        &[0xe0, 0x80, 0x80],
        &[0xf4, 0x8f, 0xbf, 0xbf],
        &[0xf4, 0x90, 0x80, 0x80],
    ];
    for bytes in byte_samples {
        let input = CString::new(*bytes).unwrap();
        let mut rune = -999;
        let width = unsafe { chartorune(&mut rune, input.as_ptr()) };
        let utf_len = unsafe { utflen(input.as_ptr()) };
        out.push(format!("decode:{bytes:?}:{width}:{rune}:{utf_len}"));
        for offset in 0..=bytes.len() {
            out.push(format!("ptr:{bytes:?}:{offset}:{}", unsafe {
                utfptrtoidx(input.as_ptr(), input.as_ptr().add(offset))
            }));
        }
    }
    for rune in [
        -1, 0, 1, 0x7f, 0x80, 0x7ff, 0x800, 0xd7ff, 0xd800, 0xffff, 0x10000, 0x10ffff, 0x110000,
        0x41, 0x61, 0x391, 0x3b1, 0x10400, 0x10428,
    ] {
        let mut bytes = [0_i8; 8];
        let width = unsafe { runetochar(bytes.as_mut_ptr(), &rune) };
        out.push(format!(
            "rune:{rune}:{width}:{}:{}:{}:{}:{}:{}",
            unsafe { runelen(rune) },
            unsafe { lower(rune) },
            unsafe { upper(rune) },
            unsafe { isalpha(rune) },
            unsafe { islower(rune) },
            unsafe { isupper(rune) }
        ));
        out.push(format!("encoded:{:?}", &bytes[..width.max(0) as usize]));
    }
    out
}

#[test]
fn low_level_numeric_and_utf_functions_match() {
    ensure_libm_is_global();
    let c = unsafe { Library::new(c_so()).unwrap() };
    let rust = unsafe { Library::new(rust_so()).unwrap() };
    assert_eq!(numeric_snapshot(&c), numeric_snapshot(&rust));
    assert_eq!(utf_snapshot(&c), utf_snapshot(&rust));
}

#[repr(C)]
#[derive(Clone, Copy)]
struct ReSubMatch {
    sp: *const c_char,
    ep: *const c_char,
}

#[repr(C)]
struct ReSub {
    nsub: c_int,
    sub: [ReSubMatch; 16],
}

type RegComp = unsafe extern "C" fn(*const c_char, c_int, *mut *const c_char) -> *mut c_void;
type RegExec = unsafe extern "C" fn(*mut c_void, *const c_char, *mut ReSub, c_int) -> c_int;
type RegFree = unsafe extern "C" fn(*mut c_void);

fn regexp_case(lib: &Library, pattern: &str, text: &str, cflags: c_int, eflags: c_int) -> String {
    let regcomp: RegComp = unsafe { load_symbol(lib, b"js_regcomp\0") };
    let regexec: RegExec = unsafe { load_symbol(lib, b"js_regexec\0") };
    let regfree: RegFree = unsafe { load_symbol(lib, b"js_regfree\0") };
    let pattern = CString::new(pattern).unwrap();
    let text = CString::new(text).unwrap();
    let mut error = ptr::null();
    let program = unsafe { regcomp(pattern.as_ptr(), cflags, &mut error) };
    if program.is_null() {
        let message = if error.is_null() {
            "<null>".into()
        } else {
            unsafe { CStr::from_ptr(error).to_string_lossy().into_owned() }
        };
        return format!("compile-error:{message}");
    }
    let null_match = ReSubMatch {
        sp: ptr::null(),
        ep: ptr::null(),
    };
    let mut matches = ReSub {
        nsub: 16,
        sub: [null_match; 16],
    };
    let status = unsafe { regexec(program, text.as_ptr(), &mut matches, eflags) };
    let mut result = format!("status:{status}:nsub:{}", matches.nsub);
    for item in matches.sub.iter().take(matches.nsub.clamp(0, 16) as usize) {
        let start = if item.sp.is_null() {
            -1
        } else {
            unsafe { item.sp.offset_from(text.as_ptr()) as isize }
        };
        let end = if item.ep.is_null() {
            -1
        } else {
            unsafe { item.ep.offset_from(text.as_ptr()) as isize }
        };
        result.push_str(&format!(":{start}-{end}"));
    }
    unsafe { regfree(program) };
    result
}

#[test]
fn low_level_regexp_valid_and_error_paths_match() {
    ensure_libm_is_global();
    let c = unsafe { Library::new(c_so()).unwrap() };
    let rust = unsafe { Library::new(rust_so()).unwrap() };
    let valid = [
        ("", ""),
        ("a", "cat"),
        ("a+", "caaat"),
        ("^a$", "a"),
        ("^a$", "x\na\ny"),
        ("[a-z]+", "123abc456"),
        ("[^0-9]+", "123abc456"),
        ("(a)(b+)", "xxabbb"),
        ("(?:ab)+", "zzabab"),
        ("a(?=b)", "ab"),
        ("a(?!b)", "ac"),
        ("(a)\\1", "aa"),
        ("\\w+\\s+\\d+", "abc 123"),
        ("é+", "zzéé"),
        (".", "😀"),
    ];
    for (pattern, text) in valid {
        for cflags in 0..4 {
            for eflags in [0, 4, 7, 0x4000_0000] {
                assert_eq!(
                    regexp_case(&c, pattern, text, cflags, eflags),
                    regexp_case(&rust, pattern, text, cflags, eflags),
                    "pattern={pattern:?}, text={text:?}, cflags={cflags}, eflags={eflags}"
                );
            }
        }
    }
    let invalid = [
        "\\", "(", ")", "[", "[z-a]", "*", "+", "?", "{1}", "a{3,2}", "(a", "a)", "\\1", "(a)\\2",
        "(?x)", "[\\", "a{999}", "[a-\\d]",
    ];
    for pattern in invalid {
        for flags in [0, 1, 2, 3, 7, c_int::MAX] {
            assert_eq!(
                regexp_case(&c, pattern, "", flags, 0),
                regexp_case(&rust, pattern, "", flags, 0),
                "invalid pattern={pattern:?}, flags={flags}"
            );
        }
    }
}

#[test]
fn explicit_error_and_rejection_corpus_matches_exactly() {
    let (c, rust) = load_pair();
    let errors = [
        "var = 1",
        "if (",
        "function (",
        "({get x(a){}})",
        "({set x(){}})",
        "'use strict'; with({}){}",
        "'use strict'; var implements=1",
        "'use strict'; delete x",
        "'use strict'; function f(a,a){}",
        "return 1",
        "break",
        "continue",
        "throw",
        "/[/",
        "/a/gg",
        "new RegExp('a','gg')",
        "new RegExp('a','x')",
        "(1).toString(1)",
        "(1).toString(37)",
        "(1).toFixed(-1)",
        "(1).toFixed(21)",
        "(1).toExponential(-1)",
        "(1).toExponential(21)",
        "(1).toPrecision(0)",
        "(1).toPrecision(22)",
        "Number.prototype.toString.call({})",
        "Boolean.prototype.valueOf.call(1)",
        "String.prototype.toString.call(1)",
        "Date.prototype.toISOString.call({})",
        "new Date(NaN).toISOString()",
        "[].reduce(function(a,b){return a+b})",
        "[].reduceRight(function(a,b){return a+b})",
        "[1].map(3)",
        "[1].filter(null)",
        "[1].forEach({})",
        "[1].every('x')",
        "[1].some(/x/)",
        "[2,1].sort(3)",
        "decodeURI('%')",
        "decodeURI('%x0')",
        "decodeURIComponent('%A')",
        "JSON.parse('')",
        "JSON.parse('{')",
        "JSON.parse('[1,]')",
        "JSON.parse('{\"a\":}')",
        "(function(){var x={};x.self=x;return JSON.stringify(x)})()",
        "null.x",
        "undefined.x",
        "1()",
        "new 1",
        "'x' in 1",
        "({}) instanceof 1",
        "(function F(){}); F.prototype=1; ({}) instanceof F",
        "notDeclared",
        "'use strict'; notDeclared=1",
        "'use strict'; (function(){return this.x}).call(null)",
        "(function(){'use strict';var o={};Object.defineProperty(o,'x',{value:1,writable:false});o.x=2})()",
        "(function(){'use strict';var o={};Object.preventExtensions(o);o.x=1})()",
        "(function(){'use strict';var o={};Object.defineProperty(o,'x',{value:1,configurable:false});delete o.x})()",
        "(function(){'use strict';var o={get x(){return 1}};o.x=2})()",
        "Object.create(1)",
        "Object.defineProperty(null,'x',{})",
        "Object.getPrototypeOf(null)",
        "Function.prototype.call.call(1)",
        "String.prototype.charAt.call(null,0)",
        "RegExp.prototype.exec.call({},'x')",
        "Error.prototype.toString.call(null)",
        "Array.prototype.push.call(null,1)",
    ];
    for flags in [0, 1, 0x7fff_ffff] {
        for source in errors {
            let c_result = eval(&c, flags, source);
            let rust_result = eval(&rust, flags, source);
            assert_eq!(c_result, rust_result, "source:\n{source}");
            assert!(
                c_result.load_status != 0 || c_result.call_status != 0,
                "expected rejection but source succeeded: {source}"
            );
        }
    }
}

#[test]
fn phase_tables_exist_and_are_source_complete() {
    let symbols = std::fs::read_to_string(manifest_dir().join("SYMBOLS.md")).unwrap();
    let errors = std::fs::read_to_string(manifest_dir().join("ERRORS.md")).unwrap();
    let configs = std::fs::read_to_string(manifest_dir().join("CONFIGS.md")).unwrap();
    assert_eq!(
        symbols
            .lines()
            .filter(|line| line.starts_with("| ") && line.as_bytes()[2].is_ascii_digit())
            .count(),
        237
    );
    assert_eq!(
        errors
            .lines()
            .filter(|line| line.starts_with("| ") && line.as_bytes()[2].is_ascii_digit())
            .count(),
        309
    );
    assert_eq!(
        configs
            .lines()
            .filter(|line| line.starts_with("| ") && line.as_bytes()[2].is_ascii_digit())
            .count(),
        286
    );
    assert!(symbols.contains("Missing from Rust: **0**"));
}
