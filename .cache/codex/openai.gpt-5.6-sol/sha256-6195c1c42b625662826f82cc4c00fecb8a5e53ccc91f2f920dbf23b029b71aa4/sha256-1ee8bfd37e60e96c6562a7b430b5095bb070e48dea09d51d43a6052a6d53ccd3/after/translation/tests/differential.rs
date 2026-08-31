use libloading::Library;
use std::cell::RefCell;
use std::collections::HashMap;
use std::env;
use std::ffi::{CStr, CString, c_char, c_double, c_int, c_uint, c_void};
use std::path::{Path, PathBuf};
use std::ptr;

type State = c_void;
type Reprog = c_void;

thread_local! {
    static REPORTS: RefCell<HashMap<usize, Vec<Vec<u8>>>> = RefCell::new(HashMap::new());
}

unsafe extern "C" fn report_callback(j: *mut State, message: *const c_char) {
    let message = if message.is_null() {
        b"<null>".to_vec()
    } else {
        // SAFETY: MuJS passes a NUL-terminated message for the duration of the callback.
        unsafe { CStr::from_ptr(message) }.to_bytes().to_vec()
    };
    REPORTS.with(|reports| {
        reports
            .borrow_mut()
            .entry(j as usize)
            .or_default()
            .push(message);
    });
}

struct Shared {
    _path: PathBuf,
    library: Library,
}

impl Shared {
    unsafe fn open(path: PathBuf) -> Self {
        assert!(
            path.is_file(),
            "shared library does not exist: {}",
            path.display()
        );
        Self {
            // SAFETY: The test controls both shared libraries and keeps them loaded.
            library: unsafe { Library::new(&path) }.unwrap(),
            _path: path,
        }
    }

    unsafe fn symbol<T: Copy>(&self, name: &str) -> T {
        // SAFETY: Call sites provide the function signature from the C headers.
        unsafe { *self.library.get::<T>(name.as_bytes()).unwrap() }
    }

    unsafe fn has_symbol(&self, name: &str) -> bool {
        // SAFETY: Symbol lookup does not call the symbol.
        unsafe { self.library.get::<*mut c_void>(name.as_bytes()) }.is_ok()
    }
}

struct Pair {
    _math: Library,
    c: Shared,
    rust: Shared,
}

impl Pair {
    unsafe fn load() -> Self {
        let math = unsafe {
            libloading::os::unix::Library::open(
                Some("libm.so.6"),
                libloading::os::unix::RTLD_NOW | libloading::os::unix::RTLD_GLOBAL,
            )
        }
        .unwrap();
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c = root.join("../c_src/build/libmujs.so");
        let rust = env::var_os("MUJS_RUST_SO")
            .map(PathBuf::from)
            .unwrap_or_else(|| root.join("target/release/libmujs.so"));
        Self {
            _math: math.into(),
            // SAFETY: Paths are checked by Shared::open.
            c: unsafe { Shared::open(c) },
            // SAFETY: Paths are checked by Shared::open.
            rust: unsafe { Shared::open(rust) },
        }
    }
}

unsafe fn new_state(lib: &Shared, flags: c_int) -> *mut State {
    type F = unsafe extern "C" fn(
        Option<unsafe extern "C" fn(*mut c_void, *mut c_void, c_int) -> *mut c_void>,
        *mut c_void,
        c_int,
    ) -> *mut State;
    // SAFETY: Signature is from mujs.h.
    unsafe { lib.symbol::<F>("js_newstate")(None, ptr::null_mut(), flags) }
}

unsafe fn free_state(lib: &Shared, j: *mut State) {
    type F = unsafe extern "C" fn(*mut State);
    // SAFETY: State came from this library; NULL is explicitly accepted by C.
    unsafe { lib.symbol::<F>("js_freestate")(j) };
}

unsafe fn c_string(ptr: *const c_char) -> Vec<u8> {
    if ptr.is_null() {
        Vec::new()
    } else {
        // SAFETY: Callers only pass MuJS-owned NUL-terminated strings.
        unsafe { CStr::from_ptr(ptr) }.to_bytes().to_vec()
    }
}

#[derive(Debug, PartialEq)]
struct ScriptResult {
    status: c_int,
    reports: Vec<Vec<u8>>,
    result: Option<Vec<u8>>,
}

unsafe fn run_script(
    lib: &Shared,
    flags: c_int,
    source: &str,
    limit: Option<(i32, i32)>,
) -> ScriptResult {
    type SetReport =
        unsafe extern "C" fn(*mut State, Option<unsafe extern "C" fn(*mut State, *const c_char)>);
    type SetLimit = unsafe extern "C" fn(*mut State, c_int, c_int);
    type DoString = unsafe extern "C" fn(*mut State, *const c_char) -> c_int;
    type GetGlobal = unsafe extern "C" fn(*mut State, *const c_char);
    type ToString = unsafe extern "C" fn(*mut State, c_int) -> *const c_char;
    type Pop = unsafe extern "C" fn(*mut State, c_int);

    // SAFETY: All calls use signatures from mujs.h and a state from this library.
    let j = unsafe { new_state(lib, flags) };
    assert!(!j.is_null());
    REPORTS.with(|reports| {
        reports.borrow_mut().insert(j as usize, Vec::new());
    });
    unsafe { lib.symbol::<SetReport>("js_setreport")(j, Some(report_callback)) };
    if let Some((run, mem)) = limit {
        unsafe { lib.symbol::<SetLimit>("js_setlimit")(j, run, mem) };
    }
    let source = CString::new(source).unwrap();
    let status = unsafe { lib.symbol::<DoString>("js_dostring")(j, source.as_ptr()) };
    let reports = REPORTS.with(|reports| reports.borrow_mut().remove(&(j as usize)).unwrap());

    let result = if status == 0 {
        let name = c"__result";
        unsafe { lib.symbol::<GetGlobal>("js_getglobal")(j, name.as_ptr()) };
        let value = unsafe { c_string(lib.symbol::<ToString>("js_tostring")(j, -1)) };
        unsafe { lib.symbol::<Pop>("js_pop")(j, 1) };
        Some(value)
    } else {
        None
    };
    unsafe { free_state(lib, j) };
    ScriptResult {
        status,
        reports,
        result,
    }
}

unsafe fn load_error(lib: &Shared, flags: c_int, source: &str) -> (c_int, Vec<u8>) {
    type Load = unsafe extern "C" fn(*mut State, *const c_char, *const c_char) -> c_int;
    type TryRepr = unsafe extern "C" fn(*mut State, c_int, *const c_char) -> *const c_char;
    let j = unsafe { new_state(lib, flags) };
    assert!(!j.is_null());
    let filename = c"test.js";
    let source = CString::new(source).unwrap();
    let status =
        unsafe { lib.symbol::<Load>("js_ploadstring")(j, filename.as_ptr(), source.as_ptr()) };
    let error = if status == 0 {
        Vec::new()
    } else {
        unsafe {
            c_string(lib.symbol::<TryRepr>("js_tryrepr")(
                j,
                -1,
                c"<repr failed>".as_ptr(),
            ))
        }
    };
    unsafe { free_state(lib, j) };
    (status, error)
}

fn assert_script_pair(pair: &Pair, flags: i32, script: &str) {
    // SAFETY: The harness owns both states and all input C strings.
    let c = unsafe { run_script(&pair.c, flags, script, None) };
    // SAFETY: Same operation through the Rust cdylib boundary.
    let rust = unsafe { run_script(&pair.rust, flags, script, None) };
    assert_eq!(c, rust, "script diverged:\n{script}");
}

#[test]
fn all_dynamic_symbols_match() {
    // SAFETY: This test performs lookup only.
    let pair = unsafe { Pair::load() };
    let manifest =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("SYMBOLS.md")).unwrap();
    let mut count = 0;
    for line in manifest.lines() {
        let Some(rest) = line.strip_prefix("| ") else {
            continue;
        };
        let Some((number, rest)) = rest.split_once(" | `") else {
            continue;
        };
        if number.parse::<usize>().is_err() {
            continue;
        }
        let Some((name, status)) = rest.split_once("` | ") else {
            continue;
        };
        assert_eq!(status, "present |");
        // SAFETY: No symbol is called.
        assert!(unsafe { pair.c.has_symbol(name) }, "C missing {name}");
        // SAFETY: No symbol is called.
        assert!(unsafe { pair.rust.has_symbol(name) }, "Rust missing {name}");
        count += 1;
    }
    assert_eq!(count, 237);
}

#[test]
fn randomized_end_to_end_valid_paths_match() {
    // SAFETY: Libraries remain loaded for the test.
    let pair = unsafe { Pair::load() };
    let deterministic = [
        "__result = JSON.stringify([undefined,null,true,false,0,-0,1/0,-1/0,0/0].map(String));",
        "__result = JSON.stringify((function(){var a=[];a[0]='x';a[3]='z';a.length=6;delete a[3];return [a.length,0 in a,3 in a,5 in a,a];})());",
        "__result = JSON.stringify((function(){var p={inherited:7};var o=Object.create(p);Object.defineProperty(o,'hidden',{value:3});o.own=9;var a=[];for(var k in o)a.push(k);a.sort();return [a,o.inherited,o.hidden];})());",
        "__result = JSON.stringify(['AbC\\nabc'.match(/^abc/gim), 'a1b2'.replace(/(\\d)/g,'[$1]'), 'a,b,,c'.split(/,/)]);",
        "__result = JSON.stringify([JSON.parse('{\"a\":[1,true,null]}'), JSON.stringify({z:1,a:'\\u03bb'})]);",
        "__result = JSON.stringify([new Date(0).toISOString(),Date.parse('2000-02-29T12:34:56.789Z'),new Date(NaN).toJSON()]);",
        "__result = JSON.stringify([Math.abs(-3),Math.pow(2,10),Math.round(-0.5),Math.max(1,9,-2),Math.min(1,9,-2)]);",
        "__result = JSON.stringify([(255).toString(16),(1.25).toFixed(3),(12.5).toExponential(2),(12.5).toPrecision(4)]);",
        "__result = JSON.stringify((function(a,b){return [arguments.length,a+b,this.x];}).call({x:4},2,3));",
        "__result = JSON.stringify((function(){function C(x){this.x=x}C.prototype.y=8;var o=new C(7);return [o.x,o.y,o instanceof C,typeof C];})());",
        "__result = JSON.stringify([encodeURI('a b/\\u03bb'),decodeURI('a%20b/%CE%BB'),parseInt('0xff',0),parseFloat('-1.25e2x')]);",
        "__result = JSON.stringify((function(){try{throw new RangeError('x')}catch(e){return [e.name,e.message,typeof e.stack]}})());",
    ];
    for flags in [0, 1, 0x4000_0000] {
        for script in deterministic {
            assert_script_pair(&pair, flags, script);
        }
    }

    let mut seed = 0x7a6d_5c4b_3e2f_1908_u64;
    for iteration in 0..192 {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let a = ((seed >> 32) as i32 % 200_001) - 100_000;
        seed = seed.rotate_left(17).wrapping_add(0x9e3779b97f4a7c15);
        let b = ((seed >> 24) as i32 % 20_001) - 10_000;
        let len = (seed as usize % 12) + 1;
        let values = (0..len)
            .map(|i| {
                let v = a.wrapping_mul((i as i32) + 3).wrapping_add(b);
                if i % 5 == 0 {
                    "null".to_owned()
                } else {
                    v.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join(",");
        let script = format!(
            "__result=JSON.stringify((function(){{var a=[{values}],s=0;a.forEach(function(x,i){{if(x!==null)s+=(x^i)}});return [a.slice(1,-1).reverse(),a.indexOf(null),a.lastIndexOf(null),s,{a},{b},'v{iteration}'.toUpperCase()];}})());"
        );
        assert_script_pair(&pair, iteration & 1, &script);
    }
}

#[derive(Debug, PartialEq)]
struct StackSnapshot {
    top: i32,
    types: Vec<i32>,
    type_names: Vec<Vec<u8>>,
    booleans: Vec<i32>,
    strings: Vec<Vec<u8>>,
}

unsafe fn stack_snapshot(lib: &Shared) -> StackSnapshot {
    type PushVoid = unsafe extern "C" fn(*mut State);
    type PushBool = unsafe extern "C" fn(*mut State, c_int);
    type PushNumber = unsafe extern "C" fn(*mut State, c_double);
    type PushString = unsafe extern "C" fn(*mut State, *const c_char);
    type PushLString = unsafe extern "C" fn(*mut State, *const c_char, c_int);
    type IntAt = unsafe extern "C" fn(*mut State, c_int) -> c_int;
    type StringAt = unsafe extern "C" fn(*mut State, c_int) -> *const c_char;
    type VoidInt = unsafe extern "C" fn(*mut State, c_int);

    let j = unsafe { new_state(lib, 0) };
    unsafe { lib.symbol::<PushVoid>("js_pushundefined")(j) };
    unsafe { lib.symbol::<PushVoid>("js_pushnull")(j) };
    unsafe { lib.symbol::<PushBool>("js_pushboolean")(j, -17) };
    for number in [0.0, -0.0, 1.25, f64::INFINITY, f64::NEG_INFINITY, f64::NAN] {
        unsafe { lib.symbol::<PushNumber>("js_pushnumber")(j, number) };
    }
    unsafe { lib.symbol::<PushString>("js_pushstring")(j, c"short".as_ptr()) };
    unsafe {
        lib.symbol::<PushString>("js_pushstring")(j, c"this is a heap allocated string".as_ptr())
    };
    let embedded = b"a\0b\0";
    unsafe {
        lib.symbol::<PushLString>("js_pushlstring")(
            j,
            embedded.as_ptr().cast(),
            (embedded.len() - 1) as i32,
        )
    };

    let top = unsafe { lib.symbol::<IntAt>("js_gettop")(j, 0) };
    let mut types = Vec::new();
    let mut type_names = Vec::new();
    let mut booleans = Vec::new();
    let mut strings = Vec::new();
    for idx in 0..top {
        types.push(unsafe { lib.symbol::<IntAt>("js_type")(j, idx) });
        type_names.push(unsafe { c_string(lib.symbol::<StringAt>("js_typeof")(j, idx)) });
        booleans.push(unsafe { lib.symbol::<IntAt>("js_toboolean")(j, idx) });
        strings.push(unsafe { c_string(lib.symbol::<StringAt>("js_tostring")(j, idx)) });
    }
    unsafe { lib.symbol::<VoidInt>("js_pop")(j, top) };
    assert_eq!(unsafe { lib.symbol::<IntAt>("js_gettop")(j, 0) }, 0);
    unsafe { free_state(lib, j) };
    StackSnapshot {
        top,
        types,
        type_names,
        booleans,
        strings,
    }
}

#[test]
fn direct_stack_and_value_api_matches() {
    // SAFETY: Each snapshot owns its state.
    let pair = unsafe { Pair::load() };
    let c = unsafe { stack_snapshot(&pair.c) };
    let rust = unsafe { stack_snapshot(&pair.rust) };
    assert_eq!(c, rust);
    assert_eq!(c.top, 12);
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Submatch {
    sp: *const c_char,
    ep: *const c_char,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Resub {
    nsub: c_int,
    sub: [Submatch; 16],
}

impl Default for Resub {
    fn default() -> Self {
        Self {
            nsub: 0,
            sub: [Submatch {
                sp: ptr::null(),
                ep: ptr::null(),
            }; 16],
        }
    }
}

#[derive(Debug, PartialEq)]
struct RegexResult {
    compile_error: Option<Vec<u8>>,
    matches: Vec<(i32, i32, Vec<(isize, isize)>)>,
}

unsafe fn regex_results(
    lib: &Shared,
    pattern: &str,
    cflags: i32,
    inputs: &[(&str, i32)],
) -> RegexResult {
    type Comp = unsafe extern "C" fn(*const c_char, c_int, *mut *const c_char) -> *mut Reprog;
    type Exec = unsafe extern "C" fn(*mut Reprog, *const c_char, *mut Resub, c_int) -> c_int;
    type Free = unsafe extern "C" fn(*mut Reprog);
    let pattern = CString::new(pattern).unwrap();
    let mut error = ptr::null();
    let prog = unsafe { lib.symbol::<Comp>("js_regcomp")(pattern.as_ptr(), cflags, &mut error) };
    if prog.is_null() {
        return RegexResult {
            compile_error: Some(unsafe { c_string(error) }),
            matches: Vec::new(),
        };
    }
    assert!(error.is_null());
    let mut matches = Vec::new();
    for (text, eflags) in inputs {
        let text = CString::new(*text).unwrap();
        let mut sub = Resub::default();
        let rc =
            unsafe { lib.symbol::<Exec>("js_regexec")(prog, text.as_ptr(), &mut sub, *eflags) };
        let offsets = sub.sub[..sub.nsub.clamp(0, 16) as usize]
            .iter()
            .map(|m| {
                if m.sp.is_null() || m.ep.is_null() {
                    (-1, -1)
                } else {
                    (unsafe { m.sp.offset_from(text.as_ptr()) }, unsafe {
                        m.ep.offset_from(text.as_ptr())
                    })
                }
            })
            .collect();
        matches.push((rc, sub.nsub, offsets));
        let rc_without_sub = unsafe {
            lib.symbol::<Exec>("js_regexec")(prog, text.as_ptr(), ptr::null_mut(), *eflags)
        };
        assert_eq!(rc_without_sub, rc);
    }
    unsafe { lib.symbol::<Free>("js_regfree")(prog) };
    unsafe { lib.symbol::<Free>("js_regfree")(ptr::null_mut()) };
    RegexResult {
        compile_error: None,
        matches,
    }
}

#[test]
fn direct_regexp_engine_matches() {
    // SAFETY: Regexp pointers never cross library boundaries.
    let pair = unsafe { Pair::load() };
    let valid = [
        ("", 0),
        ("a+", 0),
        ("^abc$", 0),
        ("^abc$", 2),
        ("(a+)(b?)\\1", 0),
        ("[a-z0-9_]+", 1),
        ("(?=ab)ab|cd", 0),
        ("(?!x).", 0),
        ("\\bword\\B", 0),
        ("\\u03bb+", 0),
    ];
    let inputs = [
        ("", 0),
        ("abc", 0),
        ("ABC", 0),
        ("x\nabc\ny", 0),
        ("abc", 4),
        ("aaabaa", 0),
        ("lambda \u{03bb}\u{03bb}", 0),
    ];
    for (pattern, flags) in valid {
        let c = unsafe { regex_results(&pair.c, pattern, flags, &inputs) };
        let rust = unsafe { regex_results(&pair.rust, pattern, flags, &inputs) };
        assert_eq!(c, rust, "regexp {pattern:?}, flags {flags}");
    }

    let invalid = [
        "\\", "\\x0", "\\x0z", "\\q", "a{999}", "[z-a]", "[abc", "(a", "a)", "\\1", "(a*)*",
    ];
    for pattern in invalid {
        let c = unsafe { regex_results(&pair.c, pattern, 0, &[]) };
        let rust = unsafe { regex_results(&pair.rust, pattern, 0, &[]) };
        assert_eq!(c, rust, "invalid regexp {pattern:?}");
        assert!(
            c.compile_error.is_some(),
            "{pattern:?} unexpectedly compiled"
        );
    }
}

#[derive(Debug, PartialEq)]
struct UtfResult {
    decoded: Vec<(i32, i32)>,
    encoded: Vec<(i32, Vec<u8>)>,
    classes: Vec<(i32, i32, i32, i32, i32, i32)>,
}

unsafe fn utf_results(lib: &Shared) -> UtfResult {
    type Decode = unsafe extern "C" fn(*mut c_int, *const c_char) -> c_int;
    type Encode = unsafe extern "C" fn(*mut c_char, *const c_int) -> c_int;
    type RuneFn = unsafe extern "C" fn(c_int) -> c_int;
    let byte_inputs: &[&[u8]] = &[
        b"\0",
        b"A\0",
        b"\xc2\xa2\0",
        b"\xe2\x82\xac\0",
        b"\xf0\x9f\x98\x80\0",
        b"\x80\0",
        b"\xc0\x80\0",
        b"\xed\xa0\x80\0",
        b"\xf4\x90\x80\x80\0",
        b"\xe2\0\0\0",
    ];
    let mut decoded = Vec::new();
    for input in byte_inputs {
        let mut rune = 0;
        let n = unsafe { lib.symbol::<Decode>("jsU_chartorune")(&mut rune, input.as_ptr().cast()) };
        decoded.push((n, rune));
    }

    let runes = [
        -1, 0, 0x41, 0x7f, 0x80, 0x7ff, 0x800, 0xd7ff, 0xd800, 0xfffd, 0xffff, 0x10000, 0x10ffff,
        0x110000,
    ];
    let mut encoded = Vec::new();
    let mut classes = Vec::new();
    for rune in runes {
        let mut bytes = [0_u8; 8];
        let n = unsafe { lib.symbol::<Encode>("jsU_runetochar")(bytes.as_mut_ptr().cast(), &rune) };
        encoded.push((n, bytes[..n.max(0) as usize].to_vec()));
        classes.push((
            unsafe { lib.symbol::<RuneFn>("jsU_runelen")(rune) },
            unsafe { lib.symbol::<RuneFn>("jsU_isalpharune")(rune) },
            unsafe { lib.symbol::<RuneFn>("jsU_islowerrune")(rune) },
            unsafe { lib.symbol::<RuneFn>("jsU_isupperrune")(rune) },
            unsafe { lib.symbol::<RuneFn>("jsU_tolowerrune")(rune) },
            unsafe { lib.symbol::<RuneFn>("jsU_toupperrune")(rune) },
        ));
    }
    UtfResult {
        decoded,
        encoded,
        classes,
    }
}

#[test]
fn direct_utf_api_matches() {
    // SAFETY: Buffers have enough trailing bytes for the C decoder.
    let pair = unsafe { Pair::load() };
    assert_eq!(unsafe { utf_results(&pair.c) }, unsafe {
        utf_results(&pair.rust)
    });
}

#[derive(Debug, PartialEq)]
struct NumberResult {
    strtod: Vec<(u64, isize)>,
    strtol: Vec<Vec<(u64, isize)>>,
    conversions: Vec<(i32, i32, u32, i16, u16)>,
    array_indices: Vec<(i32, i32)>,
    lex: Vec<(i32, i32, i32, i32)>,
}

unsafe fn number_results(lib: &Shared) -> NumberResult {
    type Parse = unsafe extern "C" fn(*const c_char, *mut *mut c_char) -> c_double;
    type ParseRadix = unsafe extern "C" fn(*const c_char, *mut *mut c_char, c_int) -> c_double;
    type ConvI = unsafe extern "C" fn(c_double) -> c_int;
    type ConvU = unsafe extern "C" fn(c_double) -> c_uint;
    type ConvS = unsafe extern "C" fn(c_double) -> i16;
    type ConvUS = unsafe extern "C" fn(c_double) -> u16;
    type ArrayIndex = unsafe extern "C" fn(*mut State, *const c_char, *mut c_int) -> c_int;
    type CharPred = unsafe extern "C" fn(c_int) -> c_int;

    let strings = [
        "",
        " ",
        "0",
        "-0",
        "1.25",
        ".5",
        "1e309",
        "-1e-999",
        "0x10",
        "Infinity",
        "+Infinity",
        "-Infinity",
        "NaN",
        "12junk",
    ];
    let mut strtod = Vec::new();
    let mut strtol = Vec::new();
    for value in strings {
        let value = CString::new(value).unwrap();
        let mut end = ptr::null_mut();
        let n = unsafe { lib.symbol::<Parse>("js_strtod")(value.as_ptr(), &mut end) };
        strtod.push((n.to_bits(), unsafe {
            end.offset_from(value.as_ptr().cast_mut())
        }));
        let mut by_radix = Vec::new();
        for radix in [2, 8, 10, 16, 36] {
            let mut end = ptr::null_mut();
            let n =
                unsafe { lib.symbol::<ParseRadix>("js_strtol")(value.as_ptr(), &mut end, radix) };
            by_radix.push((n.to_bits(), unsafe {
                end.offset_from(value.as_ptr().cast_mut())
            }));
        }
        strtol.push(by_radix);
    }

    let numbers = [
        f64::NAN,
        f64::NEG_INFINITY,
        -4294967297.0,
        -2147483649.0,
        -65537.0,
        -32769.0,
        -1.9,
        -0.0,
        0.0,
        1.9,
        32767.0,
        32768.0,
        65535.0,
        65536.0,
        2147483647.0,
        2147483648.0,
        4294967295.0,
        4294967296.0,
        f64::INFINITY,
    ];
    let conversions = numbers
        .into_iter()
        .map(|n| {
            (
                unsafe { lib.symbol::<ConvI>("jsV_numbertointeger")(n) },
                unsafe { lib.symbol::<ConvI>("jsV_numbertoint32")(n) },
                unsafe { lib.symbol::<ConvU>("jsV_numbertouint32")(n) },
                unsafe { lib.symbol::<ConvS>("jsV_numbertoint16")(n) },
                unsafe { lib.symbol::<ConvUS>("jsV_numbertouint16")(n) },
            )
        })
        .collect();

    let j = unsafe { new_state(lib, 0) };
    let mut array_indices = Vec::new();
    for value in [
        "",
        "0",
        "00",
        "01",
        "1",
        "-1",
        "2147483647",
        "2147483648",
        "1x",
    ] {
        let value = CString::new(value).unwrap();
        let mut index = -777;
        let valid =
            unsafe { lib.symbol::<ArrayIndex>("js_isarrayindex")(j, value.as_ptr(), &mut index) };
        array_indices.push((valid, index));
    }
    unsafe { free_state(lib, j) };

    let lex = (-2..=260)
        .map(|c| {
            (
                unsafe { lib.symbol::<CharPred>("jsY_ishex")(c) },
                unsafe { lib.symbol::<CharPred>("jsY_tohex")(c) },
                unsafe { lib.symbol::<CharPred>("jsY_iswhite")(c) },
                unsafe { lib.symbol::<CharPred>("jsY_isnewline")(c) },
            )
        })
        .collect();
    NumberResult {
        strtod,
        strtol,
        conversions,
        array_indices,
        lex,
    }
}

#[test]
fn direct_numeric_and_lexer_predicates_match() {
    // SAFETY: Function signatures and buffers follow jsi.h.
    let pair = unsafe { Pair::load() };
    assert_eq!(unsafe { number_results(&pair.c) }, unsafe {
        number_results(&pair.rust)
    });
}

#[test]
fn syntax_and_compile_errors_match_exactly() {
    // SAFETY: Protected loader catches all listed parser/compiler errors.
    let pair = unsafe { Pair::load() };
    let malformed = [
        "",
        "var ;",
        "if (",
        "function f( {",
        "var x = '\\u00zz';",
        "var r = /[/;",
        "return 1;",
        "break;",
        "continue;",
        "throw\n1;",
        "({get x(a){return a}});",
        "({set x(){}});",
        "\"use strict\"; with({}){}",
        "\"use strict\"; delete x;",
        "\"use strict\"; var eval;",
        "\"use strict\"; function f(arguments){}",
    ];
    for flags in [0, 1, i32::MAX] {
        for source in malformed {
            let c = unsafe { load_error(&pair.c, flags, source) };
            let rust = unsafe { load_error(&pair.rust, flags, source) };
            assert_eq!(c, rust, "loader error diverged for {source:?}");
            if !source.is_empty() {
                assert_eq!(c.0, 1, "{source:?} unexpectedly loaded");
            }
        }
    }

    let deep = format!("var x={};", "(".repeat(410) + "1" + &")".repeat(410));
    let c = unsafe { load_error(&pair.c, 0, &deep) };
    let rust = unsafe { load_error(&pair.rust, 0, &deep) };
    assert_eq!(c, rust);
    assert_eq!(c.0, 1);
}

#[test]
fn javascript_error_surface_matches_exactly() {
    // SAFETY: js_dostring protects each error and reports it through the callback.
    let pair = unsafe { Pair::load() };
    let errors = [
        "Array.prototype.sort.call([2,1], 1);",
        "Array.prototype.toString.call(null);",
        "[1].every(1);",
        "[1].some(1);",
        "[1].forEach(1);",
        "[1].map(1);",
        "[1].filter(1);",
        "[].reduce(function(a,b){return a+b});",
        "[].reduceRight(function(a,b){return a+b});",
        "[1].reduce(1);",
        "[1].reduceRight(1);",
        "Boolean.prototype.toString.call(1);",
        "Number.prototype.toString.call('x');",
        "(1).toString(1);",
        "(1).toString(37);",
        "(1).toFixed(-1);",
        "(1).toFixed(21);",
        "(1).toExponential(-1);",
        "(1).toExponential(21);",
        "(1).toPrecision(0);",
        "(1).toPrecision(22);",
        "String.prototype.toString.call(1);",
        "String.prototype.indexOf.call(null,'x');",
        "Date.prototype.getTime.call({});",
        "new Date(NaN).toISOString();",
        "Date.prototype.toJSON.call({toISOString:1});",
        "decodeURI('%');",
        "decodeURI('%xx');",
        "JSON.parse('{');",
        "JSON.parse('{1:2}');",
        "JSON.parse('undefined');",
        "var o={};o.x=o;JSON.stringify(o);",
        "var a=[];a[0]=a;JSON.stringify(a);",
        "new RegExp(/x/,'g');",
        "new RegExp('x','z');",
        "new RegExp('x','gg');",
        "new RegExp('x','ii');",
        "new RegExp('x','mm');",
        "Function.prototype.call.call(1,null);",
        "({}) instanceof ({});",
        "function C(){};C.prototype=1;({}) instanceof C;",
        "\"use strict\";var o={valueOf:function(){return {}},toString:function(){return {}}};Number(o);",
        "Object.create(1);",
        "Object.defineProperty({},'x',{value:1,get:function(){}});",
        "1 in 2;",
        "missing_name + 1;",
        "\"use strict\";missing_name=1;",
        "\"use strict\";var o={};Object.preventExtensions(o);o.x=1;",
        "\"use strict\";var o={};Object.defineProperty(o,'x',{value:1,writable:false});o.x=2;",
        "\"use strict\";var o={};Object.defineProperty(o,'x',{value:1,configurable:false});delete o.x;",
    ];
    for source in errors {
        let c = unsafe { run_script(&pair.c, 0, source, None) };
        let rust = unsafe { run_script(&pair.rust, 0, source, None) };
        assert_eq!(c, rust, "error diverged for {source}");
        assert_eq!(c.status, 1, "{source} unexpectedly succeeded");
        assert!(!c.reports.is_empty(), "{source} produced no report");
    }

    let infinite = "for(;;){}";
    let c = unsafe { run_script(&pair.c, 0, infinite, Some((200, 0))) };
    let rust = unsafe { run_script(&pair.rust, 0, infinite, Some((200, 0))) };
    assert_eq!(c, rust);
    assert_eq!(c.status, 1);

    let recursion = "function f(){f()}f();";
    let c = unsafe { run_script(&pair.c, 0, recursion, None) };
    let rust = unsafe { run_script(&pair.rust, 0, recursion, None) };
    assert_eq!(c, rust);
    assert_eq!(c.status, 1);
}

#[test]
fn context_registry_properties_iterators_and_references_match() {
    // These scripts compose the low-level state APIs through the same exported ABI.
    // SAFETY: Libraries remain loaded for all calls.
    let pair = unsafe { Pair::load() };
    let scripts = [
        "__result=JSON.stringify((function(){var o={a:1,b:2};delete o.a;o.c=3;return [o.a,o.b,o.c,Object.keys(o).sort()]})());",
        "__result=JSON.stringify((function(){var a=[1,2,3];a.length=1;a.length=4;a[3]=9;delete a[0];return [a.length,0 in a,3 in a,a]})());",
        "__result=JSON.stringify((function(){var p={p:1},o=Object.create(p),r=[];o.a=2;for(var k in o)r.push(k);return r.sort()})());",
        "__result=JSON.stringify((function(){var x=3;return function(y){return x+y}})()(4));",
        "__result=JSON.stringify((function(){function C(){};var x=new C;return [x instanceof C,C.prototype.isPrototypeOf(x)]})());",
    ];
    for script in scripts {
        assert_script_pair(&pair, 0, script);
        assert_script_pair(&pair, 1, script);
    }

    type SetContext = unsafe extern "C" fn(*mut State, *mut c_void);
    type GetContext = unsafe extern "C" fn(*mut State) -> *mut c_void;
    type PushNumber = unsafe extern "C" fn(*mut State, f64);
    type SetName = unsafe extern "C" fn(*mut State, *const c_char);
    type GetName = unsafe extern "C" fn(*mut State, *const c_char);
    type ToNumber = unsafe extern "C" fn(*mut State, i32) -> f64;
    type Pop = unsafe extern "C" fn(*mut State, i32);

    for lib in [&pair.c, &pair.rust] {
        let j = unsafe { new_state(lib, 0) };
        let marker = 0x1234usize as *mut c_void;
        unsafe { lib.symbol::<SetContext>("js_setcontext")(j, marker) };
        assert_eq!(
            unsafe { lib.symbol::<GetContext>("js_getcontext")(j) },
            marker
        );

        unsafe { lib.symbol::<PushNumber>("js_pushnumber")(j, 42.5) };
        unsafe { lib.symbol::<SetName>("js_setregistry")(j, c"key".as_ptr()) };
        unsafe { lib.symbol::<GetName>("js_getregistry")(j, c"key".as_ptr()) };
        assert_eq!(
            unsafe { lib.symbol::<ToNumber>("js_tonumber")(j, -1) },
            42.5
        );
        unsafe { lib.symbol::<Pop>("js_pop")(j, 1) };
        unsafe { lib.symbol::<SetName>("js_delregistry")(j, c"key".as_ptr()) };
        unsafe { lib.symbol::<GetName>("js_getregistry")(j, c"key".as_ptr()) };
        assert!(unsafe { lib.symbol::<ToNumber>("js_tonumber")(j, -1) }.is_nan());
        unsafe { free_state(lib, j) };
        unsafe { free_state(lib, ptr::null_mut()) };
    }
}
