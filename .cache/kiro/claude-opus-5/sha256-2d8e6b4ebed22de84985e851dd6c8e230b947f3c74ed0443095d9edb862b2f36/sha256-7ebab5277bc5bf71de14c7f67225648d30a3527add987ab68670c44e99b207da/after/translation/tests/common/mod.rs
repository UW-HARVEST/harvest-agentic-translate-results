//! Shared harness: loads the C and Rust shared objects side by side and
//! exposes them through `libloading` so that *both* implementations are
//! exercised strictly through their exported C ABI.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_double, c_int, c_long, c_longlong, c_uint, c_void, CStr, CString};
use std::path::PathBuf;
use std::sync::OnceLock;

// ---------------------------------------------------------------- json types

pub const JSON_OBJECT: c_int = 0;
pub const JSON_ARRAY: c_int = 1;
pub const JSON_STRING: c_int = 2;
pub const JSON_INTEGER: c_int = 3;
pub const JSON_REAL: c_int = 4;
pub const JSON_TRUE: c_int = 5;
pub const JSON_FALSE: c_int = 6;
pub const JSON_NULL: c_int = 7;

pub type JsonInt = c_longlong;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JsonT {
    pub type_: c_int,
    pub refcount: usize,
}

pub const JSON_ERROR_TEXT_LENGTH: usize = 160;
pub const JSON_ERROR_SOURCE_LENGTH: usize = 80;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct JsonError {
    pub line: c_int,
    pub column: c_int,
    pub position: c_int,
    pub source: [c_char; JSON_ERROR_SOURCE_LENGTH],
    pub text: [c_char; JSON_ERROR_TEXT_LENGTH],
}

impl Default for JsonError {
    fn default() -> Self {
        JsonError {
            line: 0,
            column: 0,
            position: 0,
            source: [0; JSON_ERROR_SOURCE_LENGTH],
            text: [0; JSON_ERROR_TEXT_LENGTH],
        }
    }
}

impl JsonError {
    pub fn source_str(&self) -> String {
        cbuf_to_string(&self.source)
    }
    pub fn text_str(&self) -> String {
        cbuf_to_string(&self.text)
    }
    /// Full comparable representation of the struct including all trailing
    /// bytes of both fixed buffers (byte-for-byte).
    pub fn raw(&self) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(&self.line.to_ne_bytes());
        v.extend_from_slice(&self.column.to_ne_bytes());
        v.extend_from_slice(&self.position.to_ne_bytes());
        v.extend(self.source.iter().map(|&c| c as u8));
        v.extend(self.text.iter().map(|&c| c as u8));
        v
    }
}

impl std::fmt::Debug for JsonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JsonError")
            .field("line", &self.line)
            .field("column", &self.column)
            .field("position", &self.position)
            .field("source", &self.source_str())
            .field("text", &self.text_str())
            .finish()
    }
}

pub fn cbuf_to_string(buf: &[c_char]) -> String {
    let bytes: Vec<u8> = buf.iter().map(|&c| c as u8).collect();
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}

// strbuffer_t
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct StrbufferT {
    pub value: *mut c_char,
    pub length: usize,
    pub size: usize,
}

// hashtable list / bucket / table
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct HashtableList {
    pub prev: *mut HashtableList,
    pub next: *mut HashtableList,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct HashtableBucket {
    pub first: *mut HashtableList,
    pub last: *mut HashtableList,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct HashtableT {
    pub size: usize,
    pub buckets: *mut HashtableBucket,
    pub order: usize,
    pub list: HashtableList,
    pub ordered_list: HashtableList,
}

impl Default for HashtableT {
    fn default() -> Self {
        // Zeroed is fine: hashtable_init overwrites everything it uses.
        unsafe { std::mem::zeroed() }
    }
}

// ------------------------------------------------------------------ loading

pub struct Lib {
    pub lib: Library,
    pub name: &'static str,
}

impl Lib {
    pub fn sym<T>(&self, name: &str) -> Symbol<'_, T> {
        unsafe {
            self.lib
                .get(CString::new(name).unwrap().as_bytes_with_nul())
                .unwrap_or_else(|e| panic!("{}: missing symbol `{}`: {}", self.name, name, e))
        }
    }

    pub fn try_sym<T>(&self, name: &str) -> Option<Symbol<'_, T>> {
        unsafe {
            self.lib
                .get(CString::new(name).unwrap().as_bytes_with_nul())
                .ok()
        }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_so_path() -> PathBuf {
    manifest_dir()
        .parent()
        .unwrap()
        .join("c_src/build/libjansson.so")
}

pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("JANSSON_RUST_SO") {
        return PathBuf::from(p);
    }
    let rel = manifest_dir().join("target/release/libjansson.so");
    if rel.exists() {
        return rel;
    }
    manifest_dir().join("target/debug/libjansson.so")
}

static LIBS: OnceLock<(usize, usize)> = OnceLock::new();

/// The crate is `crate-type = ["cdylib"]` only, so an integration test target
/// has no dependency edge on the shared object and `cargo test` will NOT
/// rebuild it. Testing a stale `.so` would silently pass, so refuse to run if
/// any Rust source is newer than the library under test. Use `./verify.sh`
/// (or `cargo build --release`) before `cargo test`.
fn assert_so_is_fresh(so: &std::path::Path) {
    let so_mtime = match std::fs::metadata(so).and_then(|m| m.modified()) {
        Ok(t) => t,
        Err(e) => panic!("cannot stat {:?}: {} -- run `cargo build --release` first", so, e),
    };
    let src = manifest_dir().join("src");
    let mut newest: Option<(std::path::PathBuf, std::time::SystemTime)> = None;
    let mut stack = vec![src];
    while let Some(dir) = stack.pop() {
        let rd = match std::fs::read_dir(&dir) {
            Ok(rd) => rd,
            Err(_) => continue,
        };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().and_then(|s| s.to_str()) == Some("rs") {
                if let Ok(t) = e.metadata().and_then(|m| m.modified()) {
                    if newest.as_ref().map_or(true, |(_, n)| t > *n) {
                        newest = Some((p, t));
                    }
                }
            }
        }
    }
    if let Some((p, t)) = newest {
        assert!(
            t <= so_mtime,
            "{:?} is newer than {:?}.\n\
             The crate is cdylib-only, so `cargo test` does not rebuild the .so.\n\
             Run `cargo build --release` (or ./verify.sh) before testing.",
            p,
            so
        );
    }
}

/// Both libraries are loaded once per test process and leaked, so the returned
/// references live for the whole run. `RTLD_LOCAL` (libloading's default) keeps
/// the two symbol namespaces separate.
pub fn libs() -> (&'static Lib, &'static Lib) {
    let (c, r) = *LIBS.get_or_init(|| {
        assert_so_is_fresh(&rust_so_path());
        let c = Box::leak(Box::new(Lib {
            lib: unsafe { Library::new(c_so_path()) }
                .unwrap_or_else(|e| panic!("loading C .so {:?}: {}", c_so_path(), e)),
            name: "C",
        }));
        let r = Box::leak(Box::new(Lib {
            lib: unsafe { Library::new(rust_so_path()) }
                .unwrap_or_else(|e| panic!("loading Rust .so {:?}: {}", rust_so_path(), e)),
            name: "Rust",
        }));
        (c as *const Lib as usize, r as *const Lib as usize)
    });
    unsafe { (&*(c as *const Lib), &*(r as *const Lib)) }
}

// --------------------------------------------------------------- signatures

pub type FnUtf8Encode = unsafe extern "C" fn(i32, *mut c_char, *mut usize) -> c_int;
pub type FnUtf8CheckFirst = unsafe extern "C" fn(c_char) -> usize;
pub type FnUtf8CheckFull = unsafe extern "C" fn(*const c_char, usize, *mut i32) -> usize;
pub type FnUtf8Iterate = unsafe extern "C" fn(*const c_char, usize, *mut i32) -> *const c_char;
pub type FnUtf8CheckString = unsafe extern "C" fn(*const c_char, usize) -> c_int;

pub type FnMalloc = unsafe extern "C" fn(usize) -> *mut c_void;
pub type FnRealloc = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
pub type FnFree = unsafe extern "C" fn(*mut c_void);
pub type FnStrndup = unsafe extern "C" fn(*const c_char, usize) -> *mut c_char;

pub type FnStrbufferInit = unsafe extern "C" fn(*mut StrbufferT) -> c_int;
pub type FnStrbufferClose = unsafe extern "C" fn(*mut StrbufferT);
pub type FnStrbufferClear = unsafe extern "C" fn(*mut StrbufferT);
pub type FnStrbufferValue = unsafe extern "C" fn(*const StrbufferT) -> *const c_char;
pub type FnStrbufferStealValue = unsafe extern "C" fn(*mut StrbufferT) -> *mut c_char;
pub type FnStrbufferAppendByte = unsafe extern "C" fn(*mut StrbufferT, c_char) -> c_int;
pub type FnStrbufferAppendBytes = unsafe extern "C" fn(*mut StrbufferT, *const c_char, usize) -> c_int;
pub type FnStrbufferPop = unsafe extern "C" fn(*mut StrbufferT) -> c_char;

pub type FnHtInit = unsafe extern "C" fn(*mut HashtableT) -> c_int;
pub type FnHtClose = unsafe extern "C" fn(*mut HashtableT);
pub type FnHtClear = unsafe extern "C" fn(*mut HashtableT);
pub type FnHtSet = unsafe extern "C" fn(*mut HashtableT, *const c_char, usize, *mut JsonT) -> c_int;
pub type FnHtGet = unsafe extern "C" fn(*mut HashtableT, *const c_char, usize) -> *mut c_void;
pub type FnHtDel = unsafe extern "C" fn(*mut HashtableT, *const c_char, usize) -> c_int;
pub type FnHtIter = unsafe extern "C" fn(*mut HashtableT) -> *mut c_void;
pub type FnHtIterAt = unsafe extern "C" fn(*mut HashtableT, *const c_char, usize) -> *mut c_void;
pub type FnHtIterNext = unsafe extern "C" fn(*mut HashtableT, *mut c_void) -> *mut c_void;
pub type FnHtIterKey = unsafe extern "C" fn(*mut c_void) -> *mut c_void;
pub type FnHtIterKeyLen = unsafe extern "C" fn(*mut c_void) -> usize;
pub type FnHtIterValue = unsafe extern "C" fn(*mut c_void) -> *mut c_void;
pub type FnHtIterSet = unsafe extern "C" fn(*mut c_void, *mut JsonT);

pub type FnDtoaR = unsafe extern "C" fn(
    c_double,
    c_int,
    c_int,
    *mut c_int,
    *mut c_int,
    *mut *mut c_char,
    *mut c_char,
    usize,
) -> *mut c_char;
pub type FnDtoa = unsafe extern "C" fn(
    c_double,
    c_int,
    c_int,
    *mut c_int,
    *mut c_int,
    *mut *mut c_char,
) -> *mut c_char;
pub type FnFreedtoa = unsafe extern "C" fn(*mut c_char);
pub type FnGethex = unsafe extern "C" fn(*mut *const c_char, *mut c_void, c_int, c_int) -> c_int;
pub type FnStrtod = unsafe extern "C" fn(*const c_char, *mut *mut c_char) -> c_double;

pub type FnJsonpStrtod = unsafe extern "C" fn(*mut StrbufferT, *mut c_double) -> c_int;
pub type FnJsonpDtostr = unsafe extern "C" fn(*mut c_char, usize, c_double, c_int) -> c_int;

pub type FnNew0 = unsafe extern "C" fn() -> *mut JsonT;
pub type FnJsonString = unsafe extern "C" fn(*const c_char) -> *mut JsonT;
pub type FnJsonStringn = unsafe extern "C" fn(*const c_char, usize) -> *mut JsonT;
pub type FnJsonStringValue = unsafe extern "C" fn(*const JsonT) -> *const c_char;
pub type FnJsonStringLength = unsafe extern "C" fn(*const JsonT) -> usize;
pub type FnJsonStringSet = unsafe extern "C" fn(*mut JsonT, *const c_char) -> c_int;
pub type FnJsonStringSetn = unsafe extern "C" fn(*mut JsonT, *const c_char, usize) -> c_int;
pub type FnJsonInteger = unsafe extern "C" fn(JsonInt) -> *mut JsonT;
pub type FnJsonIntegerValue = unsafe extern "C" fn(*const JsonT) -> JsonInt;
pub type FnJsonIntegerSet = unsafe extern "C" fn(*mut JsonT, JsonInt) -> c_int;
pub type FnJsonReal = unsafe extern "C" fn(c_double) -> *mut JsonT;
pub type FnJsonRealValue = unsafe extern "C" fn(*const JsonT) -> c_double;
pub type FnJsonRealSet = unsafe extern "C" fn(*mut JsonT, c_double) -> c_int;
pub type FnJsonNumberValue = unsafe extern "C" fn(*const JsonT) -> c_double;
pub type FnJsonDelete = unsafe extern "C" fn(*mut JsonT);
pub type FnJsonEqual = unsafe extern "C" fn(*const JsonT, *const JsonT) -> c_int;
pub type FnJsonCopy = unsafe extern "C" fn(*mut JsonT) -> *mut JsonT;

pub type FnJsonArraySize = unsafe extern "C" fn(*const JsonT) -> usize;
pub type FnJsonArrayGet = unsafe extern "C" fn(*const JsonT, usize) -> *mut JsonT;
pub type FnJsonArraySetNew = unsafe extern "C" fn(*mut JsonT, usize, *mut JsonT) -> c_int;
pub type FnJsonArrayAppendNew = unsafe extern "C" fn(*mut JsonT, *mut JsonT) -> c_int;
pub type FnJsonArrayInsertNew = unsafe extern "C" fn(*mut JsonT, usize, *mut JsonT) -> c_int;
pub type FnJsonArrayRemove = unsafe extern "C" fn(*mut JsonT, usize) -> c_int;
pub type FnJsonArrayClear = unsafe extern "C" fn(*mut JsonT) -> c_int;
pub type FnJsonArrayExtend = unsafe extern "C" fn(*mut JsonT, *mut JsonT) -> c_int;

pub type FnJsonObjectSize = unsafe extern "C" fn(*const JsonT) -> usize;
pub type FnJsonObjectGet = unsafe extern "C" fn(*const JsonT, *const c_char) -> *mut JsonT;
pub type FnJsonObjectGetn = unsafe extern "C" fn(*const JsonT, *const c_char, usize) -> *mut JsonT;
pub type FnJsonObjectSetNew = unsafe extern "C" fn(*mut JsonT, *const c_char, *mut JsonT) -> c_int;
pub type FnJsonObjectSetnNew =
    unsafe extern "C" fn(*mut JsonT, *const c_char, usize, *mut JsonT) -> c_int;
pub type FnJsonObjectDel = unsafe extern "C" fn(*mut JsonT, *const c_char) -> c_int;
pub type FnJsonObjectDeln = unsafe extern "C" fn(*mut JsonT, *const c_char, usize) -> c_int;
pub type FnJsonObjectClear = unsafe extern "C" fn(*mut JsonT) -> c_int;
pub type FnJsonObjectUpdate = unsafe extern "C" fn(*mut JsonT, *mut JsonT) -> c_int;
pub type FnJsonObjectIter = unsafe extern "C" fn(*mut JsonT) -> *mut c_void;
pub type FnJsonObjectIterAt = unsafe extern "C" fn(*mut JsonT, *const c_char) -> *mut c_void;
pub type FnJsonObjectIterNext = unsafe extern "C" fn(*mut JsonT, *mut c_void) -> *mut c_void;
pub type FnJsonObjectIterKey = unsafe extern "C" fn(*mut c_void) -> *const c_char;
pub type FnJsonObjectIterKeyLen = unsafe extern "C" fn(*mut c_void) -> usize;
pub type FnJsonObjectIterValue = unsafe extern "C" fn(*mut c_void) -> *mut JsonT;
pub type FnJsonObjectIterSetNew = unsafe extern "C" fn(*mut JsonT, *mut c_void, *mut JsonT) -> c_int;
pub type FnJsonObjectKeyToIter = unsafe extern "C" fn(*const c_char) -> *mut c_void;
pub type FnJsonObjectSeed = unsafe extern "C" fn(usize);

pub type FnJsonDumps = unsafe extern "C" fn(*const JsonT, usize) -> *mut c_char;
pub type FnJsonDumpb = unsafe extern "C" fn(*const JsonT, *mut c_char, usize, usize) -> usize;
pub type FnJsonDumpCallback =
    unsafe extern "C" fn(*const JsonT, *mut c_void, *mut c_void, usize) -> c_int;
pub type FnJsonDumpFile = unsafe extern "C" fn(*const JsonT, *const c_char, usize) -> c_int;
pub type FnJsonDumpfd = unsafe extern "C" fn(*const JsonT, c_int, usize) -> c_int;

pub type FnJsonLoads = unsafe extern "C" fn(*const c_char, usize, *mut JsonError) -> *mut JsonT;
pub type FnJsonLoadb =
    unsafe extern "C" fn(*const c_char, usize, usize, *mut JsonError) -> *mut JsonT;
pub type FnJsonLoadFile = unsafe extern "C" fn(*const c_char, usize, *mut JsonError) -> *mut JsonT;
pub type FnJsonLoadfd = unsafe extern "C" fn(c_int, usize, *mut JsonError) -> *mut JsonT;
pub type FnJsonLoadCallback =
    unsafe extern "C" fn(*mut c_void, *mut c_void, usize, *mut JsonError) -> *mut JsonT;

pub type FnVersionStr = unsafe extern "C" fn() -> *const c_char;
pub type FnVersionCmp = unsafe extern "C" fn(c_int, c_int, c_int) -> c_int;

pub type FnErrorInit = unsafe extern "C" fn(*mut JsonError);

// ------------------------------------------------------------------ helpers

pub unsafe fn cstr(p: *const c_char) -> Option<String> {
    if p.is_null() {
        None
    } else {
        Some(CStr::from_ptr(p).to_string_lossy().into_owned())
    }
}

pub unsafe fn cbytes(p: *const c_char) -> Option<Vec<u8>> {
    if p.is_null() {
        None
    } else {
        Some(CStr::from_ptr(p).to_bytes().to_vec())
    }
}

/// `json_dumps` the value on the given library and return the raw bytes.
pub unsafe fn dump(l: &Lib, v: *const JsonT, flags: usize) -> Option<Vec<u8>> {
    let f: Symbol<FnJsonDumps> = l.sym("json_dumps");
    let free: Symbol<FnFree> = l.sym("jsonp_free");
    let p = f(v, flags);
    if p.is_null() {
        return None;
    }
    let out = CStr::from_ptr(p).to_bytes().to_vec();
    free(p as *mut c_void);
    Some(out)
}

pub fn cs(s: &str) -> CString {
    CString::new(s).unwrap()
}

/// A byte string that may contain interior NULs is handled by keeping the
/// Vec alive and using its pointer/length.
pub struct Raw(pub Vec<u8>);
impl Raw {
    pub fn new(b: &[u8]) -> Self {
        Raw(b.to_vec())
    }
    pub fn ptr(&self) -> *const c_char {
        self.0.as_ptr() as *const c_char
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

// Keep the unused-import warnings quiet for the wide type surface above.
const _: Option<c_long> = None;
const _: Option<c_uint> = None;
