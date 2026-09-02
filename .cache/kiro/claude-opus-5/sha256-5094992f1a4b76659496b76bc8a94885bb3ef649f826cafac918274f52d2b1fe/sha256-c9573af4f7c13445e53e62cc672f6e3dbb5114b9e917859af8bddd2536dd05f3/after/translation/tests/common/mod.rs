//! Differential-test harness: loads BOTH the C `libjansson.so` and the Rust
//! `libjansson.so` through `libloading` and exposes matching typed wrappers.
//!
//! Nothing here calls a Rust function directly — every call goes through the
//! `.so` export table, exactly as an external C consumer would.
#![allow(dead_code, non_snake_case, non_camel_case_types)]

use libloading::{Library, Symbol};
use std::ffi::{CStr, CString, c_char, c_double, c_int, c_longlong, c_void};
use std::path::PathBuf;

pub const JSON_ERROR_TEXT_LENGTH: usize = 160;
pub const JSON_ERROR_SOURCE_LENGTH: usize = 80;

/* ------------------------------------------------------------------ */
/* C-compatible structs                                               */
/* ------------------------------------------------------------------ */

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct JsonT {
    pub type_: c_int,
    pub refcount: usize,
}

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
        // Deliberately fill with a non-zero sentinel so we can see exactly
        // which bytes each library writes.
        JsonError {
            line: 0x5a5a_5a5a,
            column: 0x5a5a_5a5a,
            position: 0x5a5a_5a5a,
            source: [0x5a; JSON_ERROR_SOURCE_LENGTH],
            text: [0x5a; JSON_ERROR_TEXT_LENGTH],
        }
    }
}

impl JsonError {
    /// A fully-zeroed error struct (matches a caller that memsets to 0).
    pub fn zeroed() -> Self {
        JsonError {
            line: 0,
            column: 0,
            position: 0,
            source: [0; JSON_ERROR_SOURCE_LENGTH],
            text: [0; JSON_ERROR_TEXT_LENGTH],
        }
    }
    pub fn text_str(&self) -> String {
        let bytes: Vec<u8> = self.text.iter().map(|&c| c as u8).collect();
        let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
        String::from_utf8_lossy(&bytes[..end]).into_owned()
    }
    pub fn source_str(&self) -> String {
        let bytes: Vec<u8> = self.source.iter().map(|&c| c as u8).collect();
        let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
        String::from_utf8_lossy(&bytes[..end]).into_owned()
    }
    /// `json_error_code()` from jansson.h: `text[JSON_ERROR_TEXT_LENGTH - 1]`.
    pub fn code(&self) -> i32 {
        self.text[JSON_ERROR_TEXT_LENGTH - 1] as u8 as i32
    }
    /// Everything an external observer can see, as a comparable tuple.
    pub fn snapshot(&self) -> (c_int, c_int, c_int, String, String, i32, Vec<u8>) {
        (
            self.line,
            self.column,
            self.position,
            self.source_str(),
            self.text_str(),
            self.code(),
            self.text.iter().map(|&c| c as u8).collect(),
        )
    }
}

/// `strbuffer_t` from src/strbuffer.h
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct StrBuffer {
    pub value: *mut c_char,
    pub length: usize,
    pub size: usize,
}

/// `struct hashtable_list`
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct HtList {
    pub prev: *mut c_void,
    pub next: *mut c_void,
}

/// `hashtable_t` from src/hashtable.h
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Hashtable {
    pub size: usize,
    pub buckets: *mut c_void,
    pub order: usize,
    pub list: HtList,
    pub ordered_list: HtList,
}

/// `struct hashtable_pair` — needed to emulate the `hashtable_key_to_iter`
/// / `json_object_key_to_iter` macro (`container_of(key, pair_t, key)`).
#[repr(C)]
pub struct HtPair {
    pub list: HtList,
    pub ordered_list: HtList,
    pub hash: usize,
    pub value: *mut JsonT,
    pub key_len: usize,
    pub key: [c_char; 1],
}

/// `union { double d; ULong L[2]; } U` from dtoa.c
#[repr(C)]
#[derive(Clone, Copy)]
pub union U {
    pub d: f64,
    pub L: [u32; 2],
}

/* json_type */
pub const JSON_OBJECT: c_int = 0;
pub const JSON_ARRAY: c_int = 1;
pub const JSON_STRING: c_int = 2;
pub const JSON_INTEGER: c_int = 3;
pub const JSON_REAL: c_int = 4;
pub const JSON_TRUE: c_int = 5;
pub const JSON_FALSE: c_int = 6;
pub const JSON_NULL: c_int = 7;

/* decoder flags */
pub const JSON_REJECT_DUPLICATES: usize = 0x1;
pub const JSON_DISABLE_EOF_CHECK: usize = 0x2;
pub const JSON_DECODE_ANY: usize = 0x4;
pub const JSON_DECODE_INT_AS_REAL: usize = 0x8;
pub const JSON_ALLOW_NUL: usize = 0x10;

/* encoder flags */
pub const JSON_MAX_INDENT: usize = 0x1F;
pub const JSON_COMPACT: usize = 0x20;
pub const JSON_ENSURE_ASCII: usize = 0x40;
pub const JSON_SORT_KEYS: usize = 0x80;
pub const JSON_PRESERVE_ORDER: usize = 0x100;
pub const JSON_ENCODE_ANY: usize = 0x200;
pub const JSON_ESCAPE_SLASH: usize = 0x400;
pub const JSON_EMBED: usize = 0x10000;
pub fn json_indent(n: usize) -> usize {
    n & JSON_MAX_INDENT
}
pub fn json_real_precision(n: usize) -> usize {
    (n & 0x1F) << 11
}

/* pack/unpack flags */
pub const JSON_VALIDATE_ONLY: usize = 0x1;
pub const JSON_STRICT: usize = 0x2;

/* json_error_code */
pub const E_UNKNOWN: i32 = 0;
pub const E_OUT_OF_MEMORY: i32 = 1;
pub const E_STACK_OVERFLOW: i32 = 2;
pub const E_CANNOT_OPEN_FILE: i32 = 3;
pub const E_INVALID_ARGUMENT: i32 = 4;
pub const E_INVALID_UTF8: i32 = 5;
pub const E_PREMATURE_END_OF_INPUT: i32 = 6;
pub const E_END_OF_INPUT_EXPECTED: i32 = 7;
pub const E_INVALID_SYNTAX: i32 = 8;
pub const E_INVALID_FORMAT: i32 = 9;
pub const E_WRONG_TYPE: i32 = 10;
pub const E_NULL_CHARACTER: i32 = 11;
pub const E_NULL_VALUE: i32 = 12;
pub const E_NULL_BYTE_IN_KEY: i32 = 13;
pub const E_DUPLICATE_KEY: i32 = 14;
pub const E_NUMERIC_OVERFLOW: i32 = 15;
pub const E_ITEM_NOT_FOUND: i32 = 16;
pub const E_INDEX_OUT_OF_RANGE: i32 = 17;

/* ------------------------------------------------------------------ */
/* Function-pointer types                                             */
/* ------------------------------------------------------------------ */

pub type DumpCb = unsafe extern "C" fn(*const c_char, usize, *mut c_void) -> c_int;
pub type LoadCb = unsafe extern "C" fn(*mut c_void, usize, *mut c_void) -> usize;
pub type MallocFn = unsafe extern "C" fn(usize) -> *mut c_void;
pub type ReallocFn = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
pub type FreeFn = unsafe extern "C" fn(*mut c_void);

macro_rules! api_struct {
    ( $( $name:ident : $ty:ty ),* $(,)? ) => {
        pub struct Api {
            _lib: &'static Library,
            pub tag: &'static str,
            $( pub $name : $ty , )*
        }
        impl Api {
            unsafe fn load(lib: &'static Library, tag: &'static str) -> Api {
                unsafe {
                    Api {
                        _lib: lib,
                        tag,
                        $( $name : {
                            let s: Symbol<$ty> = lib
                                .get(concat!(stringify!($name), "\0").as_bytes())
                                .unwrap_or_else(|e| panic!("{}: missing symbol {}: {e}", tag, stringify!($name)));
                            *s
                        }, )*
                    }
                }
            }
        }
    };
}

api_struct! {
    /* --- version --- */
    jansson_version_str: unsafe extern "C" fn() -> *const c_char,
    jansson_version_cmp: unsafe extern "C" fn(c_int, c_int, c_int) -> c_int,

    /* --- memory --- */
    jsonp_malloc: unsafe extern "C" fn(usize) -> *mut c_void,
    jsonp_free: unsafe extern "C" fn(*mut c_void),
    jsonp_realloc: unsafe extern "C" fn(*mut c_void, usize, usize) -> *mut c_void,
    jsonp_strndup: unsafe extern "C" fn(*const c_char, usize) -> *mut c_char,
    json_set_alloc_funcs: unsafe extern "C" fn(Option<MallocFn>, Option<FreeFn>),
    json_get_alloc_funcs: unsafe extern "C" fn(*mut Option<MallocFn>, *mut Option<FreeFn>),
    json_set_alloc_funcs2: unsafe extern "C" fn(Option<MallocFn>, Option<ReallocFn>, Option<FreeFn>),
    json_get_alloc_funcs2: unsafe extern "C" fn(*mut Option<MallocFn>, *mut Option<ReallocFn>, *mut Option<FreeFn>),

    /* --- utf --- */
    utf8_encode: unsafe extern "C" fn(i32, *mut c_char, *mut usize) -> c_int,
    utf8_check_first: unsafe extern "C" fn(c_char) -> usize,
    utf8_check_full: unsafe extern "C" fn(*const c_char, usize, *mut i32) -> usize,
    utf8_iterate: unsafe extern "C" fn(*const c_char, usize, *mut i32) -> *const c_char,
    utf8_check_string: unsafe extern "C" fn(*const c_char, usize) -> c_int,

    /* --- strbuffer --- */
    strbuffer_init: unsafe extern "C" fn(*mut StrBuffer) -> c_int,
    strbuffer_close: unsafe extern "C" fn(*mut StrBuffer),
    strbuffer_clear: unsafe extern "C" fn(*mut StrBuffer),
    strbuffer_value: unsafe extern "C" fn(*const StrBuffer) -> *const c_char,
    strbuffer_steal_value: unsafe extern "C" fn(*mut StrBuffer) -> *mut c_char,
    strbuffer_append_byte: unsafe extern "C" fn(*mut StrBuffer, c_char) -> c_int,
    strbuffer_append_bytes: unsafe extern "C" fn(*mut StrBuffer, *const c_char, usize) -> c_int,
    strbuffer_pop: unsafe extern "C" fn(*mut StrBuffer) -> c_char,

    /* --- hashtable --- */
    hashtable_init: unsafe extern "C" fn(*mut Hashtable) -> c_int,
    hashtable_close: unsafe extern "C" fn(*mut Hashtable),
    hashtable_set: unsafe extern "C" fn(*mut Hashtable, *const c_char, usize, *mut JsonT) -> c_int,
    hashtable_get: unsafe extern "C" fn(*mut Hashtable, *const c_char, usize) -> *mut c_void,
    hashtable_del: unsafe extern "C" fn(*mut Hashtable, *const c_char, usize) -> c_int,
    hashtable_clear: unsafe extern "C" fn(*mut Hashtable),
    hashtable_iter: unsafe extern "C" fn(*mut Hashtable) -> *mut c_void,
    hashtable_iter_at: unsafe extern "C" fn(*mut Hashtable, *const c_char, usize) -> *mut c_void,
    hashtable_iter_next: unsafe extern "C" fn(*mut Hashtable, *mut c_void) -> *mut c_void,
    hashtable_iter_key: unsafe extern "C" fn(*mut c_void) -> *mut c_char,
    hashtable_iter_key_len: unsafe extern "C" fn(*mut c_void) -> usize,
    hashtable_iter_value: unsafe extern "C" fn(*mut c_void) -> *mut c_void,
    hashtable_iter_set: unsafe extern "C" fn(*mut c_void, *mut JsonT),

    /* --- error --- */
    jsonp_error_init: unsafe extern "C" fn(*mut JsonError, *const c_char),
    jsonp_error_set_source: unsafe extern "C" fn(*mut JsonError, *const c_char),
    jsonp_error_set: unsafe extern "C" fn(*mut JsonError, c_int, c_int, usize, c_int, *const c_char, ...),

    /* --- strconv / dtoa --- */
    jsonp_dtostr: unsafe extern "C" fn(*mut c_char, usize, f64, c_int) -> c_int,
    jsonp_strtod: unsafe extern "C" fn(*mut StrBuffer, *mut f64) -> c_int,
    dtoa_r: unsafe extern "C" fn(f64, c_int, c_int, *mut c_int, *mut c_int, *mut *mut c_char, *mut c_char, usize) -> *mut c_char,
    dtoa: unsafe extern "C" fn(f64, c_int, c_int, *mut c_int, *mut c_int, *mut *mut c_char) -> *mut c_char,
    freedtoa: unsafe extern "C" fn(*mut c_char),
    gethex: unsafe extern "C" fn(*mut *const c_char, *mut U, c_int, c_int),
    strtod__unused: unsafe extern "C" fn(*const c_char, *mut *mut c_char) -> f64,

    /* --- value: object --- */
    json_object: unsafe extern "C" fn() -> *mut JsonT,
    json_object_seed: unsafe extern "C" fn(usize),
    json_object_size: unsafe extern "C" fn(*const JsonT) -> usize,
    json_object_get: unsafe extern "C" fn(*const JsonT, *const c_char) -> *mut JsonT,
    json_object_getn: unsafe extern "C" fn(*const JsonT, *const c_char, usize) -> *mut JsonT,
    json_object_set_new: unsafe extern "C" fn(*mut JsonT, *const c_char, *mut JsonT) -> c_int,
    json_object_setn_new: unsafe extern "C" fn(*mut JsonT, *const c_char, usize, *mut JsonT) -> c_int,
    json_object_set_new_nocheck: unsafe extern "C" fn(*mut JsonT, *const c_char, *mut JsonT) -> c_int,
    json_object_setn_new_nocheck: unsafe extern "C" fn(*mut JsonT, *const c_char, usize, *mut JsonT) -> c_int,
    json_object_del: unsafe extern "C" fn(*mut JsonT, *const c_char) -> c_int,
    json_object_deln: unsafe extern "C" fn(*mut JsonT, *const c_char, usize) -> c_int,
    json_object_clear: unsafe extern "C" fn(*mut JsonT) -> c_int,
    json_object_update: unsafe extern "C" fn(*mut JsonT, *mut JsonT) -> c_int,
    json_object_update_existing: unsafe extern "C" fn(*mut JsonT, *mut JsonT) -> c_int,
    json_object_update_missing: unsafe extern "C" fn(*mut JsonT, *mut JsonT) -> c_int,
    json_object_update_recursive: unsafe extern "C" fn(*mut JsonT, *mut JsonT) -> c_int,
    do_object_update_recursive: unsafe extern "C" fn(*mut JsonT, *mut JsonT, *mut Hashtable) -> c_int,
    json_object_iter: unsafe extern "C" fn(*mut JsonT) -> *mut c_void,
    json_object_iter_at: unsafe extern "C" fn(*mut JsonT, *const c_char) -> *mut c_void,
    json_object_key_to_iter: unsafe extern "C" fn(*const c_char) -> *mut c_void,
    json_object_iter_next: unsafe extern "C" fn(*mut JsonT, *mut c_void) -> *mut c_void,
    json_object_iter_key: unsafe extern "C" fn(*mut c_void) -> *const c_char,
    json_object_iter_key_len: unsafe extern "C" fn(*mut c_void) -> usize,
    json_object_iter_value: unsafe extern "C" fn(*mut c_void) -> *mut JsonT,
    json_object_iter_set_new: unsafe extern "C" fn(*mut JsonT, *mut c_void, *mut JsonT) -> c_int,

    /* --- value: array --- */
    json_array: unsafe extern "C" fn() -> *mut JsonT,
    json_array_size: unsafe extern "C" fn(*const JsonT) -> usize,
    json_array_get: unsafe extern "C" fn(*const JsonT, usize) -> *mut JsonT,
    json_array_set_new: unsafe extern "C" fn(*mut JsonT, usize, *mut JsonT) -> c_int,
    json_array_append_new: unsafe extern "C" fn(*mut JsonT, *mut JsonT) -> c_int,
    json_array_insert_new: unsafe extern "C" fn(*mut JsonT, usize, *mut JsonT) -> c_int,
    json_array_remove: unsafe extern "C" fn(*mut JsonT, usize) -> c_int,
    json_array_clear: unsafe extern "C" fn(*mut JsonT) -> c_int,
    json_array_extend: unsafe extern "C" fn(*mut JsonT, *mut JsonT) -> c_int,

    /* --- value: scalars --- */
    json_string: unsafe extern "C" fn(*const c_char) -> *mut JsonT,
    json_stringn: unsafe extern "C" fn(*const c_char, usize) -> *mut JsonT,
    json_string_nocheck: unsafe extern "C" fn(*const c_char) -> *mut JsonT,
    json_stringn_nocheck: unsafe extern "C" fn(*const c_char, usize) -> *mut JsonT,
    jsonp_stringn_nocheck_own: unsafe extern "C" fn(*const c_char, usize) -> *mut JsonT,
    json_string_value: unsafe extern "C" fn(*const JsonT) -> *const c_char,
    json_string_length: unsafe extern "C" fn(*const JsonT) -> usize,
    json_string_set: unsafe extern "C" fn(*mut JsonT, *const c_char) -> c_int,
    json_string_setn: unsafe extern "C" fn(*mut JsonT, *const c_char, usize) -> c_int,
    json_string_set_nocheck: unsafe extern "C" fn(*mut JsonT, *const c_char) -> c_int,
    json_string_setn_nocheck: unsafe extern "C" fn(*mut JsonT, *const c_char, usize) -> c_int,
    json_integer: unsafe extern "C" fn(c_longlong) -> *mut JsonT,
    json_integer_value: unsafe extern "C" fn(*const JsonT) -> c_longlong,
    json_integer_set: unsafe extern "C" fn(*mut JsonT, c_longlong) -> c_int,
    json_real: unsafe extern "C" fn(c_double) -> *mut JsonT,
    json_real_value: unsafe extern "C" fn(*const JsonT) -> c_double,
    json_real_set: unsafe extern "C" fn(*mut JsonT, c_double) -> c_int,
    json_number_value: unsafe extern "C" fn(*const JsonT) -> c_double,
    json_true: unsafe extern "C" fn() -> *mut JsonT,
    json_false: unsafe extern "C" fn() -> *mut JsonT,
    json_null: unsafe extern "C" fn() -> *mut JsonT,
    json_delete: unsafe extern "C" fn(*mut JsonT),
    json_equal: unsafe extern "C" fn(*const JsonT, *const JsonT) -> c_int,
    json_copy: unsafe extern "C" fn(*mut JsonT) -> *mut JsonT,
    json_deep_copy: unsafe extern "C" fn(*const JsonT) -> *mut JsonT,
    do_deep_copy: unsafe extern "C" fn(*const JsonT, *mut Hashtable) -> *mut JsonT,
    jsonp_loop_check: unsafe extern "C" fn(*mut Hashtable, *const JsonT, *mut c_char, usize, *mut usize) -> c_int,

    /* --- dump --- */
    json_dumps: unsafe extern "C" fn(*const JsonT, usize) -> *mut c_char,
    json_dumpb: unsafe extern "C" fn(*const JsonT, *mut c_char, usize, usize) -> usize,
    json_dumpf: unsafe extern "C" fn(*const JsonT, *mut c_void, usize) -> c_int,
    json_dumpfd: unsafe extern "C" fn(*const JsonT, c_int, usize) -> c_int,
    json_dump_file: unsafe extern "C" fn(*const JsonT, *const c_char, usize) -> c_int,
    json_dump_callback: unsafe extern "C" fn(*const JsonT, Option<DumpCb>, *mut c_void, usize) -> c_int,

    /* --- load --- */
    json_loads: unsafe extern "C" fn(*const c_char, usize, *mut JsonError) -> *mut JsonT,
    json_loadb: unsafe extern "C" fn(*const c_char, usize, usize, *mut JsonError) -> *mut JsonT,
    json_loadf: unsafe extern "C" fn(*mut c_void, usize, *mut JsonError) -> *mut JsonT,
    json_loadfd: unsafe extern "C" fn(c_int, usize, *mut JsonError) -> *mut JsonT,
    json_load_file: unsafe extern "C" fn(*const c_char, usize, *mut JsonError) -> *mut JsonT,
    json_load_callback: unsafe extern "C" fn(Option<LoadCb>, *mut c_void, usize, *mut JsonError) -> *mut JsonT,

    /* --- pack / unpack / sprintf --- */
    json_pack: unsafe extern "C" fn(*const c_char, ...) -> *mut JsonT,
    json_pack_ex: unsafe extern "C" fn(*mut JsonError, usize, *const c_char, ...) -> *mut JsonT,
    json_unpack: unsafe extern "C" fn(*mut JsonT, *const c_char, ...) -> c_int,
    json_unpack_ex: unsafe extern "C" fn(*mut JsonT, *mut JsonError, usize, *const c_char, ...) -> c_int,
    json_sprintf: unsafe extern "C" fn(*const c_char, ...) -> *mut JsonT,
}

impl Api {
    /// `hashtable_seed` is an exported *variable*, not a function.
    pub fn hashtable_seed(&self) -> u32 {
        unsafe {
            let s: Symbol<*mut u32> = self._lib.get(b"hashtable_seed\0").unwrap();
            **s
        }
    }
    /// `dtoa_divmax` is an exported *variable*.
    pub fn dtoa_divmax(&self) -> c_int {
        unsafe {
            let s: Symbol<*mut c_int> = self._lib.get(b"dtoa_divmax\0").unwrap();
            **s
        }
    }
}

/* ------------------------------------------------------------------ */
/* Library loading                                                    */
/* ------------------------------------------------------------------ */

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("JANSSON_RUST_SO") {
        return PathBuf::from(p);
    }
    let root = workspace_root().join("translation").join("target");
    // Prefer whichever profile dir the running test binary lives in.
    let exe = std::env::current_exe().unwrap();
    for anc in exe.ancestors() {
        let cand = anc.join("libjansson.so");
        if cand.exists() {
            return cand;
        }
    }
    for prof in ["release", "debug"] {
        let cand = root.join(prof).join("libjansson.so");
        if cand.exists() {
            return cand;
        }
    }
    panic!("cannot locate Rust libjansson.so");
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("JANSSON_C_SO") {
        return PathBuf::from(p);
    }
    workspace_root()
        .join("c_src")
        .join("build")
        .join("libjansson.so")
}

static INIT: std::sync::OnceLock<(Api, Api)> = std::sync::OnceLock::new();

fn pair() -> &'static (Api, Api) {
    INIT.get_or_init(|| unsafe {
        let cpath = c_so_path();
        let rpath = rust_so_path();
        let clib: &'static Library = Box::leak(Box::new(
            Library::new(&cpath).unwrap_or_else(|e| panic!("load {cpath:?}: {e}")),
        ));
        let rlib: &'static Library = Box::leak(Box::new(
            Library::new(&rpath).unwrap_or_else(|e| panic!("load {rpath:?}: {e}")),
        ));
        let c = Api::load(clib, "C");
        let r = Api::load(rlib, "RUST");
        // Pin the hash seed in BOTH libraries so object iteration order is
        // deterministic and comparable. Must happen before any json_object().
        (c.json_object_seed)(0x5eed_1234);
        (r.json_object_seed)(0x5eed_1234);
        (c, r)
    })
}

pub fn c() -> &'static Api {
    &pair().0
}
pub fn r() -> &'static Api {
    &pair().1
}
/// Both implementations, in a fixed order, for `for (name, api) in both()`.
pub fn both() -> [&'static Api; 2] {
    [c(), r()]
}

/* ------------------------------------------------------------------ */
/* Helpers                                                            */
/* ------------------------------------------------------------------ */

pub fn cs(s: &str) -> CString {
    CString::new(s.as_bytes().to_vec()).unwrap_or_else(|_| {
        // contains a NUL — build manually with the NUL preserved is impossible
        // for CString; caller should use raw bytes instead.
        panic!("cs() called with embedded NUL: {s:?}")
    })
}

/// NUL-terminated byte vector, allowing interior NULs.
pub fn cbytes(b: &[u8]) -> Vec<u8> {
    let mut v = b.to_vec();
    v.push(0);
    v
}

pub unsafe fn from_cstr(p: *const c_char) -> Option<String> {
    if p.is_null() {
        None
    } else {
        Some(unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned())
    }
}

pub unsafe fn from_cstr_bytes(p: *const c_char) -> Option<Vec<u8>> {
    if p.is_null() {
        None
    } else {
        Some(unsafe { CStr::from_ptr(p) }.to_bytes().to_vec())
    }
}

/// `json_dumps` result as owned bytes; frees with the *same* library's free.
pub unsafe fn dumps(api: &Api, json: *const JsonT, flags: usize) -> Option<Vec<u8>> {
    unsafe {
        let p = (api.json_dumps)(json, flags);
        if p.is_null() {
            return None;
        }
        let out = CStr::from_ptr(p).to_bytes().to_vec();
        (api.jsonp_free)(p as *mut c_void);
        Some(out)
    }
}

/// The `json_incref` inline from jansson.h, replicated (it is a static inline
/// in the header, so it is not an exported symbol in either library).
pub unsafe fn incref(json: *mut JsonT) -> *mut JsonT {
    unsafe {
        if !json.is_null() && (*json).refcount != usize::MAX {
            (*json).refcount += 1;
        }
        json
    }
}

/// The `json_decref` inline from jansson.h, replicated.
pub unsafe fn decref(api: &Api, json: *mut JsonT) {
    unsafe {
        if !json.is_null() && (*json).refcount != usize::MAX {
            (*json).refcount -= 1;
            if (*json).refcount == 0 {
                (api.json_delete)(json);
            }
        }
    }
}

/// Observable shape of a json_t tree, produced *only* through public getters,
/// so it can be compared across the two libraries.
pub unsafe fn shape(api: &Api, json: *const JsonT) -> String {
    unsafe { shape_d(api, json, 0) }
}

unsafe fn shape_d(api: &Api, json: *const JsonT, depth: u32) -> String {
    unsafe {
        if json.is_null() {
            return "<null-ptr>".into();
        }
        if depth > 64 {
            return "<too-deep>".into();
        }
        let t = (*json).type_;
        match t {
            JSON_NULL => "null".into(),
            JSON_TRUE => "true".into(),
            JSON_FALSE => "false".into(),
            JSON_INTEGER => format!("i:{}", (api.json_integer_value)(json)),
            JSON_REAL => format!("r:{:?}", (api.json_real_value)(json).to_bits()),
            JSON_STRING => {
                let p = (api.json_string_value)(json);
                let n = (api.json_string_length)(json);
                let bytes = if p.is_null() {
                    Vec::new()
                } else {
                    std::slice::from_raw_parts(p as *const u8, n).to_vec()
                };
                format!("s:{n}:{bytes:02x?}")
            }
            JSON_ARRAY => {
                let n = (api.json_array_size)(json);
                let mut s = format!("a{n}[");
                for i in 0..n {
                    s.push_str(&shape_d(api, (api.json_array_get)(json, i), depth + 1));
                    s.push(',');
                }
                s.push(']');
                s
            }
            JSON_OBJECT => {
                let n = (api.json_object_size)(json);
                let mut s = format!("o{n}{{");
                let mut it = (api.json_object_iter)(json as *mut JsonT);
                while !it.is_null() {
                    let kp = (api.json_object_iter_key)(it);
                    let kl = (api.json_object_iter_key_len)(it);
                    let kb = if kp.is_null() {
                        Vec::new()
                    } else {
                        std::slice::from_raw_parts(kp as *const u8, kl).to_vec()
                    };
                    s.push_str(&format!("{kb:02x?}="));
                    s.push_str(&shape_d(
                        api,
                        (api.json_object_iter_value)(it),
                        depth + 1,
                    ));
                    s.push(';');
                    it = (api.json_object_iter_next)(json as *mut JsonT, it);
                }
                s.push('}');
                s
            }
            other => format!("<type {other}>"),
        }
    }
}

/* ------------------------------------------------------------------ */
/* Serialization for the (deliberately) thread-unsafe dtoa globals    */
/* ------------------------------------------------------------------ */

/// `dtoa.c` is compiled without `MULTIPLE_THREADS`, so `freelist`, `p5s` and
/// `dtoa_result` are plain globals in BOTH implementations. Any test that can
/// reach them (float formatting/parsing, i.e. `dtoa*`, `gethex`,
/// `strtod__unused`, `jsonp_dtostr`, `jsonp_strtod`, and any `json_dumps` /
/// `json_loads` involving reals) must hold this lock, otherwise the harness
/// itself — not the translation — is at fault for the corruption.
pub static DTOA_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub fn dtoa_guard() -> std::sync::MutexGuard<'static, ()> {
    DTOA_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/* ------------------------------------------------------------------ */
/* Deterministic PRNG (xorshift64*) — fixed seed, reproducible        */
/* ------------------------------------------------------------------ */

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 { 0 } else { (self.next_u64() % n as u64) as usize }
    }
    pub fn range_i64(&mut self, lo: i64, hi: i64) -> i64 {
        if hi <= lo {
            return lo;
        }
        let span = (hi as i128 - lo as i128) as u128;
        (lo as i128 + (self.next_u64() as u128 % span) as i128) as i64
    }
    pub fn f64_finite(&mut self) -> f64 {
        loop {
            let v = f64::from_bits(self.next_u64());
            if v.is_finite() {
                return v;
            }
        }
    }
    pub fn f64_smallish(&mut self) -> f64 {
        let m = (self.next_u64() % 2_000_000_001) as f64 - 1_000_000_000.0;
        let e = (self.next_u64() % 41) as i32 - 20;
        m * 10f64.powi(e)
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
    /// Random ASCII identifier-ish key.
    pub fn key(&mut self, maxlen: usize) -> String {
        const AL: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_-";
        let n = 1 + self.below(maxlen.max(1));
        (0..n)
            .map(|_| AL[self.below(AL.len())] as char)
            .collect()
    }
    /// Random valid-UTF-8 string mixing 1/2/3/4-byte sequences.
    pub fn utf8(&mut self, maxchars: usize) -> String {
        let n = self.below(maxchars + 1);
        let mut s = String::new();
        for _ in 0..n {
            let cp = match self.below(4) {
                // Deliberately excludes U+0000: `cs()` cannot carry an interior
                // NUL. NUL handling is covered separately by the byte-level
                // tests that use `cbytes()` / explicit `key_len`s.
                0 => 1 + self.below(0x7F) as u32,
                1 => 0x80 + self.below(0x780) as u32,
                2 => 0x800 + self.below(0xF800) as u32,
                _ => 0x1_0000 + self.below(0x10_0000) as u32,
            };
            if let Some(ch) = char::from_u32(cp) {
                s.push(ch);
            } else {
                s.push('?');
            }
        }
        s
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| (self.next_u64() & 0xFF) as u8).collect()
    }
}

/* ------------------------------------------------------------------ */
/* Random JSON document generator (as text, so both libs parse it)    */
/* ------------------------------------------------------------------ */

pub fn escape_json(s: &str) -> String {
    let mut out = String::from("\"");
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04X}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Generates a random *valid* JSON document. `depth` bounds nesting.
pub fn gen_json(rng: &mut Rng, depth: u32) -> String {
    let choice = if depth == 0 { rng.below(6) } else { rng.below(8) };
    match choice {
        0 => "null".into(),
        1 => if rng.bool() { "true" } else { "false" }.into(),
        2 => format!("{}", rng.range_i64(-1_000_000, 1_000_000)),
        3 => {
            let v = rng.f64_smallish();
            let s = format!("{:e}", v);
            if s.contains("NaN") || s.contains("inf") {
                "0.5".into()
            } else {
                s
            }
        }
        4 => escape_json(&rng.utf8(12)),
        5 => escape_json(&rng.key(10)),
        6 => {
            let n = rng.below(6);
            let items: Vec<String> = (0..n).map(|_| gen_json(rng, depth - 1)).collect();
            format!("[{}]", items.join(","))
        }
        _ => {
            let n = rng.below(6);
            let mut seen = std::collections::BTreeSet::new();
            let mut items = Vec::new();
            for _ in 0..n {
                let k = rng.key(8);
                if !seen.insert(k.clone()) {
                    continue;
                }
                items.push(format!("{}:{}", escape_json(&k), gen_json(rng, depth - 1)));
            }
            format!("{{{}}}", items.join(","))
        }
    }
}

/// Asserts the two byte slices are identical, with a readable diff.
#[track_caller]
pub fn assert_bytes_eq(what: &str, cv: &Option<Vec<u8>>, rv: &Option<Vec<u8>>) {
    if cv != rv {
        panic!(
            "{what}\n  C   = {}\n  RUST= {}",
            fmt_opt(cv),
            fmt_opt(rv)
        );
    }
}

pub fn fmt_opt(v: &Option<Vec<u8>>) -> String {
    match v {
        None => "<NULL>".into(),
        Some(b) => format!("{:?} ({} bytes)", String::from_utf8_lossy(b), b.len()),
    }
}
