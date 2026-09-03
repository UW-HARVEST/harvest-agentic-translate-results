//! Shared differential-test harness.
//!
//! Both the C shared library and the Rust `cdylib` are loaded with
//! `libloading` (i.e. `dlopen` with `RTLD_LOCAL`, so the two symbol sets do
//! not interpose on each other) and every call goes through the exported
//! `extern "C"` entry points — never through Rust-internal functions.

#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

use std::ffi::{c_char, c_int, c_void, CString};
use std::path::PathBuf;

/* ------------------------------------------------------------------ */
/* ABI mirrors                                                        */
/* ------------------------------------------------------------------ */

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CJson {
    pub next: *mut CJson,
    pub prev: *mut CJson,
    pub child: *mut CJson,
    pub type_: c_int,
    pub valuestring: *mut c_char,
    pub valueint: c_int,
    pub valuedouble: f64,
    pub string: *mut c_char,
}

pub type MallocFn = unsafe extern "C" fn(usize) -> *mut c_void;
pub type FreeFn = unsafe extern "C" fn(*mut c_void);

#[repr(C)]
pub struct CJsonHooks {
    pub malloc_fn: Option<MallocFn>,
    pub free_fn: Option<FreeFn>,
}

pub const cJSON_Invalid: c_int = 0;
pub const cJSON_False: c_int = 1 << 0;
pub const cJSON_True: c_int = 1 << 1;
pub const cJSON_NULL: c_int = 1 << 2;
pub const cJSON_Number: c_int = 1 << 3;
pub const cJSON_String: c_int = 1 << 4;
pub const cJSON_Array: c_int = 1 << 5;
pub const cJSON_Object: c_int = 1 << 6;
pub const cJSON_Raw: c_int = 1 << 7;
pub const cJSON_IsReference: c_int = 256;
pub const cJSON_StringIsConst: c_int = 512;

/* ------------------------------------------------------------------ */
/* symbol table                                                       */
/* ------------------------------------------------------------------ */

macro_rules! declare_api {
    ( $( $field:ident : $ty:ty ),* $(,)? ) => {
        pub struct Api {
            pub name: &'static str,
            $( pub $field: $ty, )*
        }

        impl Api {
            fn load_from(lib: &'static libloading::Library, name: &'static str) -> Api {
                unsafe {
                    Api {
                        name,
                        $( $field: {
                            let sym: libloading::Symbol<$ty> = lib
                                .get(concat!(stringify!($field), "\0").as_bytes())
                                .unwrap_or_else(|e| panic!(
                                    "{}: missing symbol {}: {}", name, stringify!($field), e));
                            *sym
                        }, )*
                    }
                }
            }
        }
    };
}

declare_api! {
    cJSON_Version: unsafe extern "C" fn() -> *const c_char,
    cJSON_InitHooks: unsafe extern "C" fn(*mut CJsonHooks),

    cJSON_Parse: unsafe extern "C" fn(*const c_char) -> *mut CJson,
    cJSON_ParseWithLength: unsafe extern "C" fn(*const c_char, usize) -> *mut CJson,
    cJSON_ParseWithOpts:
        unsafe extern "C" fn(*const c_char, *mut *const c_char, c_int) -> *mut CJson,
    cJSON_ParseWithLengthOpts:
        unsafe extern "C" fn(*const c_char, usize, *mut *const c_char, c_int) -> *mut CJson,

    cJSON_Print: unsafe extern "C" fn(*const CJson) -> *mut c_char,
    cJSON_PrintUnformatted: unsafe extern "C" fn(*const CJson) -> *mut c_char,
    cJSON_PrintBuffered: unsafe extern "C" fn(*const CJson, c_int, c_int) -> *mut c_char,
    cJSON_PrintPreallocated:
        unsafe extern "C" fn(*mut CJson, *mut c_char, c_int, c_int) -> c_int,
    cJSON_Delete: unsafe extern "C" fn(*mut CJson),

    cJSON_GetArraySize: unsafe extern "C" fn(*const CJson) -> c_int,
    cJSON_GetArrayItem: unsafe extern "C" fn(*const CJson, c_int) -> *mut CJson,
    cJSON_GetObjectItem: unsafe extern "C" fn(*const CJson, *const c_char) -> *mut CJson,
    cJSON_GetObjectItemCaseSensitive:
        unsafe extern "C" fn(*const CJson, *const c_char) -> *mut CJson,
    cJSON_HasObjectItem: unsafe extern "C" fn(*const CJson, *const c_char) -> c_int,
    cJSON_GetErrorPtr: unsafe extern "C" fn() -> *const c_char,
    cJSON_GetStringValue: unsafe extern "C" fn(*const CJson) -> *mut c_char,
    cJSON_GetNumberValue: unsafe extern "C" fn(*const CJson) -> f64,

    cJSON_IsInvalid: unsafe extern "C" fn(*const CJson) -> c_int,
    cJSON_IsFalse: unsafe extern "C" fn(*const CJson) -> c_int,
    cJSON_IsTrue: unsafe extern "C" fn(*const CJson) -> c_int,
    cJSON_IsBool: unsafe extern "C" fn(*const CJson) -> c_int,
    cJSON_IsNull: unsafe extern "C" fn(*const CJson) -> c_int,
    cJSON_IsNumber: unsafe extern "C" fn(*const CJson) -> c_int,
    cJSON_IsString: unsafe extern "C" fn(*const CJson) -> c_int,
    cJSON_IsArray: unsafe extern "C" fn(*const CJson) -> c_int,
    cJSON_IsObject: unsafe extern "C" fn(*const CJson) -> c_int,
    cJSON_IsRaw: unsafe extern "C" fn(*const CJson) -> c_int,

    cJSON_CreateNull: unsafe extern "C" fn() -> *mut CJson,
    cJSON_CreateTrue: unsafe extern "C" fn() -> *mut CJson,
    cJSON_CreateFalse: unsafe extern "C" fn() -> *mut CJson,
    cJSON_CreateBool: unsafe extern "C" fn(c_int) -> *mut CJson,
    cJSON_CreateNumber: unsafe extern "C" fn(f64) -> *mut CJson,
    cJSON_CreateString: unsafe extern "C" fn(*const c_char) -> *mut CJson,
    cJSON_CreateRaw: unsafe extern "C" fn(*const c_char) -> *mut CJson,
    cJSON_CreateArray: unsafe extern "C" fn() -> *mut CJson,
    cJSON_CreateObject: unsafe extern "C" fn() -> *mut CJson,
    cJSON_CreateStringReference: unsafe extern "C" fn(*const c_char) -> *mut CJson,
    cJSON_CreateObjectReference: unsafe extern "C" fn(*const CJson) -> *mut CJson,
    cJSON_CreateArrayReference: unsafe extern "C" fn(*const CJson) -> *mut CJson,
    cJSON_CreateIntArray: unsafe extern "C" fn(*const c_int, c_int) -> *mut CJson,
    cJSON_CreateFloatArray: unsafe extern "C" fn(*const f32, c_int) -> *mut CJson,
    cJSON_CreateDoubleArray: unsafe extern "C" fn(*const f64, c_int) -> *mut CJson,
    cJSON_CreateStringArray: unsafe extern "C" fn(*const *const c_char, c_int) -> *mut CJson,

    cJSON_AddItemToArray: unsafe extern "C" fn(*mut CJson, *mut CJson) -> c_int,
    cJSON_AddItemToObject:
        unsafe extern "C" fn(*mut CJson, *const c_char, *mut CJson) -> c_int,
    cJSON_AddItemToObjectCS:
        unsafe extern "C" fn(*mut CJson, *const c_char, *mut CJson) -> c_int,
    cJSON_AddItemReferenceToArray: unsafe extern "C" fn(*mut CJson, *mut CJson) -> c_int,
    cJSON_AddItemReferenceToObject:
        unsafe extern "C" fn(*mut CJson, *const c_char, *mut CJson) -> c_int,

    cJSON_DetachItemViaPointer: unsafe extern "C" fn(*mut CJson, *mut CJson) -> *mut CJson,
    cJSON_DetachItemFromArray: unsafe extern "C" fn(*mut CJson, c_int) -> *mut CJson,
    cJSON_DeleteItemFromArray: unsafe extern "C" fn(*mut CJson, c_int),
    cJSON_DetachItemFromObject: unsafe extern "C" fn(*mut CJson, *const c_char) -> *mut CJson,
    cJSON_DetachItemFromObjectCaseSensitive:
        unsafe extern "C" fn(*mut CJson, *const c_char) -> *mut CJson,
    cJSON_DeleteItemFromObject: unsafe extern "C" fn(*mut CJson, *const c_char),
    cJSON_DeleteItemFromObjectCaseSensitive: unsafe extern "C" fn(*mut CJson, *const c_char),

    cJSON_InsertItemInArray: unsafe extern "C" fn(*mut CJson, c_int, *mut CJson) -> c_int,
    cJSON_ReplaceItemViaPointer:
        unsafe extern "C" fn(*mut CJson, *mut CJson, *mut CJson) -> c_int,
    cJSON_ReplaceItemInArray: unsafe extern "C" fn(*mut CJson, c_int, *mut CJson) -> c_int,
    cJSON_ReplaceItemInObject:
        unsafe extern "C" fn(*mut CJson, *const c_char, *mut CJson) -> c_int,
    cJSON_ReplaceItemInObjectCaseSensitive:
        unsafe extern "C" fn(*mut CJson, *const c_char, *mut CJson) -> c_int,

    cJSON_Duplicate: unsafe extern "C" fn(*const CJson, c_int) -> *mut CJson,
    cJSON_Compare: unsafe extern "C" fn(*const CJson, *const CJson, c_int) -> c_int,
    cJSON_Minify: unsafe extern "C" fn(*mut c_char),

    cJSON_AddNullToObject: unsafe extern "C" fn(*mut CJson, *const c_char) -> *mut CJson,
    cJSON_AddTrueToObject: unsafe extern "C" fn(*mut CJson, *const c_char) -> *mut CJson,
    cJSON_AddFalseToObject: unsafe extern "C" fn(*mut CJson, *const c_char) -> *mut CJson,
    cJSON_AddBoolToObject:
        unsafe extern "C" fn(*mut CJson, *const c_char, c_int) -> *mut CJson,
    cJSON_AddNumberToObject:
        unsafe extern "C" fn(*mut CJson, *const c_char, f64) -> *mut CJson,
    cJSON_AddStringToObject:
        unsafe extern "C" fn(*mut CJson, *const c_char, *const c_char) -> *mut CJson,
    cJSON_AddRawToObject:
        unsafe extern "C" fn(*mut CJson, *const c_char, *const c_char) -> *mut CJson,
    cJSON_AddObjectToObject: unsafe extern "C" fn(*mut CJson, *const c_char) -> *mut CJson,
    cJSON_AddArrayToObject: unsafe extern "C" fn(*mut CJson, *const c_char) -> *mut CJson,

    cJSON_SetNumberHelper: unsafe extern "C" fn(*mut CJson, f64) -> f64,
    cJSON_SetValuestring: unsafe extern "C" fn(*mut CJson, *const c_char) -> *mut c_char,

    cJSON_malloc: unsafe extern "C" fn(usize) -> *mut c_void,
    cJSON_free: unsafe extern "C" fn(*mut c_void),
}

/* ------------------------------------------------------------------ */
/* library discovery                                                  */
/* ------------------------------------------------------------------ */

pub fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

pub fn c_lib_path() -> PathBuf {
    let base = workspace_root().join("c_src/build");
    for cand in [
        "libcjson.so.1.7.19",
        "libcjson.so.1",
        "libcjson.so",
    ] {
        let p = base.join(cand);
        if p.exists() {
            return p;
        }
    }
    panic!(
        "C shared library not found under {}. Build it with:\n  cd c_src && mkdir -p build && \
         cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        base.display()
    )
}

pub fn c_driver_lib_path() -> PathBuf {
    let p = workspace_root().join("c_src/build/libcJSON_test.so");
    assert!(p.exists(), "missing {}", p.display());
    p
}

pub fn rust_lib_path() -> PathBuf {
    let target = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
    // Prefer the profile this test binary was compiled with.
    let preferred = if cfg!(debug_assertions) { "debug" } else { "release" };
    let other = if cfg!(debug_assertions) { "release" } else { "debug" };
    for p in [
        target.join(preferred).join("libcJSON_test.so"),
        target.join(other).join("libcJSON_test.so"),
    ] {
        if p.exists() {
            return p;
        }
    }
    panic!("Rust cdylib libcJSON_test.so not found under {}", target.display())
}

fn leak_lib(path: &PathBuf) -> &'static libloading::Library {
    let lib = unsafe { libloading::Library::new(path) }
        .unwrap_or_else(|e| panic!("dlopen({}) failed: {}", path.display(), e));
    Box::leak(Box::new(lib))
}

/// The two implementations under test.
pub struct Pair {
    pub c: &'static Api,
    pub r: &'static Api,
}

static INIT: std::sync::OnceLock<Pair> = std::sync::OnceLock::new();

pub fn pair() -> &'static Pair {
    INIT.get_or_init(|| {
        let c = Api::load_from(leak_lib(&c_lib_path()), "C");
        let r = Api::load_from(leak_lib(&rust_lib_path()), "Rust");
        Pair {
            c: Box::leak(Box::new(c)),
            r: Box::leak(Box::new(r)),
        }
    })
}

/// `driver` lives in a separate C `.so` (`test.c`) but in the *same* Rust
/// `cdylib`.
pub type DriverFn = unsafe extern "C" fn(
    *const *const c_char,
    *const [c_int; 3],
    *const c_int,
    *const Record,
) -> c_int;

#[repr(C)]
pub struct Record {
    pub precision: *const c_char,
    pub lat: f64,
    pub lon: f64,
    pub address: *const c_char,
    pub city: *const c_char,
    pub state: *const c_char,
    pub zip: *const c_char,
    pub country: *const c_char,
}

pub fn c_driver() -> DriverFn {
    let lib = leak_lib(&c_driver_lib_path());
    unsafe {
        let s: libloading::Symbol<DriverFn> = lib.get(b"driver\0").unwrap();
        *s
    }
}

pub fn rust_driver() -> DriverFn {
    let lib = leak_lib(&rust_lib_path());
    unsafe {
        let s: libloading::Symbol<DriverFn> = lib.get(b"driver\0").unwrap();
        *s
    }
}

/* ------------------------------------------------------------------ */
/* global serialization                                               */
/* ------------------------------------------------------------------ */

/// Both libraries keep **process-global** mutable state (`global_error` for
/// `cJSON_GetErrorPtr`, and `global_hooks` for `cJSON_InitHooks`). Cargo runs
/// integration tests as parallel threads inside one process, so every test
/// must hold this lock while it touches either library.
pub fn lock() -> std::sync::MutexGuard<'static, ()> {
    static M: std::sync::Mutex<()> = std::sync::Mutex::new(());
    match M.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/* ------------------------------------------------------------------ */
/* small helpers                                                      */
/* ------------------------------------------------------------------ */

/// Read a NUL-terminated string produced by one of the libraries.
pub unsafe fn take_cstr(p: *const c_char) -> Option<Vec<u8>> {
    if p.is_null() {
        None
    } else {
        Some(std::ffi::CStr::from_ptr(p).to_bytes().to_vec())
    }
}

/// Read the printed string and release it with that library's `cJSON_free`.
pub unsafe fn take_printed(api: &Api, p: *mut c_char) -> Option<Vec<u8>> {
    let out = take_cstr(p);
    if !p.is_null() {
        (api.cJSON_free)(p as *mut c_void);
    }
    out
}

pub fn cs(s: &str) -> CString {
    CString::new(s).unwrap()
}

/// Bytes that may contain arbitrary non-NUL values.
pub fn cbytes(b: &[u8]) -> Vec<c_char> {
    let mut v: Vec<c_char> = b.iter().map(|&x| x as c_char).collect();
    v.push(0);
    v
}

pub fn show(b: &Option<Vec<u8>>) -> String {
    match b {
        None => "<NULL>".to_string(),
        Some(v) => String::from_utf8_lossy(v).into_owned(),
    }
}

/* ------------------------------------------------------------------ */
/* deterministic RNG (xorshift64*), fixed seed per test              */
/* ------------------------------------------------------------------ */

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed | 1)
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % (n as u64)) as usize
    }
    pub fn i32(&mut self) -> i32 {
        self.next_u64() as i32
    }
    pub fn f64(&mut self) -> f64 {
        f64::from_bits(self.next_u64())
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
}

/* ------------------------------------------------------------------ */
/* random JSON generation                                             */
/* ------------------------------------------------------------------ */

const NUMBER_POOL: &[&str] = &[
    "0",
    "-0",
    "1",
    "-1",
    "42",
    "2147483647",
    "2147483648",
    "-2147483648",
    "-2147483649",
    "4294967296",
    "0.5",
    "-0.5",
    "3.141592653589793",
    "1e10",
    "1e-10",
    "1E+3",
    "-1.7976931348623157e308",
    "1.7976931348623157e308",
    "1e309",
    "-1e309",
    "5e-324",
    "1e-320",
    "0.1",
    "0.3",
    "123456789012345678901234567890",
    "1.0000000000000002",
    "9007199254740993",
    "100000000000000000000",
    "-0.0",
    "12345678901234567",
];

const STRING_POOL: &[&str] = &[
    "",
    "a",
    "hello world",
    "with \\\"quotes\\\"",
    "tab\\there",
    "nl\\nhere",
    "cr\\rhere",
    "bs\\\\here",
    "bell\\bhere",
    "ff\\fhere",
    "slash\\/here",
    "\\u0000",
    "\\u0001\\u001f",
    "\\u0041\\u00e9\\u20ac",
    "\\ud83d\\ude00",
    "\\uD800\\uDC00",
    "\\uffff",
    "caf\u{e9}",
    "\u{4e2d}\u{6587}",
    "0123456789012345678901234567890123456789012345678901234567890123456789",
    " leading and trailing ",
    "line1\\nline2\\nline3",
];

const KEY_POOL: &[&str] = &[
    "a", "A", "b", "key", "Key", "KEY", "", "long key with spaces", "0", "\\u0041", "z",
];

fn gen_value(rng: &mut Rng, out: &mut String, depth: usize, max_depth: usize) {
    let choice = if depth >= max_depth {
        rng.below(5)
    } else {
        rng.below(7)
    };
    match choice {
        0 => out.push_str("null"),
        1 => out.push_str("true"),
        2 => out.push_str("false"),
        3 => out.push_str(NUMBER_POOL[rng.below(NUMBER_POOL.len())]),
        4 => {
            out.push('"');
            out.push_str(STRING_POOL[rng.below(STRING_POOL.len())]);
            out.push('"');
        }
        5 => {
            let n = rng.below(5);
            out.push('[');
            for i in 0..n {
                if i > 0 {
                    out.push(',');
                }
                gen_value(rng, out, depth + 1, max_depth);
            }
            out.push(']');
        }
        _ => {
            let n = rng.below(5);
            out.push('{');
            for i in 0..n {
                if i > 0 {
                    out.push(',');
                }
                out.push('"');
                out.push_str(KEY_POOL[rng.below(KEY_POOL.len())]);
                out.push_str("\":");
                gen_value(rng, out, depth + 1, max_depth);
            }
            out.push('}');
        }
    }
}

/// A randomized but valid JSON document.
pub fn random_json(rng: &mut Rng, max_depth: usize) -> String {
    let mut s = String::new();
    gen_value(rng, &mut s, 0, max_depth);
    s
}

const WS: &[&str] = &["", " ", "\t", "\n", "\r", "  \n\t ", "\u{1}"];

/// Sprinkle whitespace (bytes <= 32) around structural characters.
pub fn sprinkle_ws(rng: &mut Rng, json: &str) -> String {
    let mut out = String::new();
    let mut in_string = false;
    let mut escaped = false;
    for ch in json.chars() {
        if in_string {
            out.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        if ch == '"' {
            out.push_str(WS[rng.below(WS.len())]);
            out.push(ch);
            in_string = true;
            continue;
        }
        if matches!(ch, '[' | ']' | '{' | '}' | ',' | ':') {
            out.push_str(WS[rng.below(WS.len())]);
            out.push(ch);
            out.push_str(WS[rng.below(WS.len())]);
        } else {
            out.push(ch);
        }
    }
    out
}

/* ------------------------------------------------------------------ */
/* structural comparison of two parsed trees                          */
/* ------------------------------------------------------------------ */

/// A fully-materialised, comparable snapshot of a `cJSON` tree.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Snapshot {
    pub type_: c_int,
    pub valuestring: Option<Vec<u8>>,
    pub valueint: c_int,
    /// raw bits so that NaN payload / -0.0 differences are caught
    pub valuedouble_bits: u64,
    pub key: Option<Vec<u8>>,
    pub children: Vec<Snapshot>,
    /// `prev` of the first child must point at the last child (cJSON invariant)
    pub child_prev_is_last: Option<bool>,
    pub next_is_null_at_end: bool,
}

pub unsafe fn snapshot(item: *const CJson) -> Option<Snapshot> {
    if item.is_null() {
        return None;
    }
    let it = &*item;
    let mut children = Vec::new();
    // A reference item shares its child list with the referenced node; still
    // walk it, the linked list is well-formed.
    let mut c = it.child;
    let mut last: *mut CJson = std::ptr::null_mut();
    let mut guard = 0;
    while !c.is_null() {
        children.push(snapshot(c).unwrap());
        last = c;
        c = (*c).next;
        guard += 1;
        assert!(guard < 2_000_000, "cycle in child list");
    }
    let child_prev_is_last = if it.child.is_null() {
        None
    } else {
        Some((*it.child).prev == last)
    };
    Some(Snapshot {
        type_: it.type_,
        valuestring: take_cstr(it.valuestring),
        valueint: it.valueint,
        valuedouble_bits: it.valuedouble.to_bits(),
        key: take_cstr(it.string),
        children,
        child_prev_is_last,
        next_is_null_at_end: true,
    })
}

/* ------------------------------------------------------------------ */
/* stdout capture (for the `driver` differential test)                */
/* ------------------------------------------------------------------ */

pub fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    use std::io::{Read, Seek};
    unsafe {
        let mut tmp = std::env::temp_dir();
        tmp.push(format!(
            "cjson_driver_{}_{}.out",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path = cs(tmp.to_str().unwrap());

        libc::fflush(std::ptr::null_mut());
        let saved = libc::dup(1);
        assert!(saved >= 0);
        let fd = libc::open(
            path.as_ptr(),
            libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC,
            0o600 as libc::c_int,
        );
        assert!(fd >= 0, "open temp file failed");
        assert!(libc::dup2(fd, 1) >= 0);

        f();

        libc::fflush(std::ptr::null_mut());
        libc::dup2(saved, 1);
        libc::close(saved);
        libc::close(fd);

        let mut file = std::fs::File::open(tmp.clone()).unwrap();
        file.seek(std::io::SeekFrom::Start(0)).unwrap();
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).unwrap();
        let _ = std::fs::remove_file(&tmp);
        buf
    }
}
