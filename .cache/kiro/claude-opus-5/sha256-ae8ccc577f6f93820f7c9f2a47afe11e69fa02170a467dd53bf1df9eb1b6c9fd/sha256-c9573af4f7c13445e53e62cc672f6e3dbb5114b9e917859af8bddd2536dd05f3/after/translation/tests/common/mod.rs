//! Differential-test harness.
//!
//! Loads BOTH shared objects through `libloading` and exposes every exported
//! symbol as a plain `extern "C"` function pointer.  No Rust function is ever
//! called directly: the Rust side is always reached through
//! `translation/target/release/libjansson.so`, exactly like an external C
//! consumer, so the `#[no_mangle]` wrappers are part of what is verified.
#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};
use std::path::PathBuf;

/* ------------------------------------------------------------------ */
/* ABI types                                                          */
/* ------------------------------------------------------------------ */

#[repr(C)]
#[derive(Clone, Copy)]
pub struct JsonT {
    pub type_: c_int,
    pub refcount: usize,
}

pub type Jt = *mut JsonT;

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

impl JsonError {
    pub fn zeroed() -> JsonError {
        JsonError {
            line: 0,
            column: 0,
            position: 0,
            source: [0; JSON_ERROR_SOURCE_LENGTH],
            text: [0; JSON_ERROR_TEXT_LENGTH],
        }
    }
    /// The `text` field up to (but not including) the trailing code byte.
    pub fn text_str(&self) -> String {
        let b: Vec<u8> = self.text[..JSON_ERROR_TEXT_LENGTH - 1]
            .iter()
            .map(|&c| c as u8)
            .collect();
        let end = b.iter().position(|&c| c == 0).unwrap_or(b.len());
        String::from_utf8_lossy(&b[..end]).into_owned()
    }
    pub fn source_str(&self) -> String {
        let b: Vec<u8> = self.source.iter().map(|&c| c as u8).collect();
        let end = b.iter().position(|&c| c == 0).unwrap_or(b.len());
        String::from_utf8_lossy(&b[..end]).into_owned()
    }
    /// `json_error_code()` from `jansson.h`: the last byte of `text`.
    pub fn code(&self) -> u8 {
        self.text[JSON_ERROR_TEXT_LENGTH - 1] as u8
    }
    /// Whole-struct comparison key: raw bytes of every field.
    pub fn snapshot(&self) -> (c_int, c_int, c_int, Vec<u8>, Vec<u8>) {
        (
            self.line,
            self.column,
            self.position,
            self.source.iter().map(|&c| c as u8).collect(),
            self.text.iter().map(|&c| c as u8).collect(),
        )
    }
}

/// `hashtable_t` — 7 pointer-sized words.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct HashtableT {
    pub size: usize,
    pub buckets: *mut c_void,
    pub order: usize,
    pub list_prev: *mut c_void,
    pub list_next: *mut c_void,
    pub olist_prev: *mut c_void,
    pub olist_next: *mut c_void,
}

impl HashtableT {
    pub fn zeroed() -> HashtableT {
        HashtableT {
            size: 0,
            buckets: std::ptr::null_mut(),
            order: 0,
            list_prev: std::ptr::null_mut(),
            list_next: std::ptr::null_mut(),
            olist_prev: std::ptr::null_mut(),
            olist_next: std::ptr::null_mut(),
        }
    }
}

/// `strbuffer_t`
#[repr(C)]
#[derive(Clone, Copy)]
pub struct StrbufferT {
    pub value: *mut c_char,
    pub length: usize,
    pub size: usize,
}

impl StrbufferT {
    pub fn zeroed() -> StrbufferT {
        StrbufferT {
            value: std::ptr::null_mut(),
            length: 0,
            size: 0,
        }
    }
}

pub type DumpCb = Option<unsafe extern "C" fn(*const c_char, usize, *mut c_void) -> c_int>;
pub type LoadCb = Option<unsafe extern "C" fn(*mut c_void, usize, *mut c_void) -> usize>;
pub type MallocFn = Option<unsafe extern "C" fn(usize) -> *mut c_void>;
pub type ReallocFn = Option<unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void>;
pub type FreeFn = Option<unsafe extern "C" fn(*mut c_void)>;

/* json_type */
pub const JSON_OBJECT: c_int = 0;
pub const JSON_ARRAY: c_int = 1;
pub const JSON_STRING: c_int = 2;
pub const JSON_INTEGER: c_int = 3;
pub const JSON_REAL: c_int = 4;
pub const JSON_TRUE: c_int = 5;
pub const JSON_FALSE: c_int = 6;
pub const JSON_NULL: c_int = 7;

/* error codes */
pub const E_UNKNOWN: u8 = 0;
pub const E_OUT_OF_MEMORY: u8 = 1;
pub const E_STACK_OVERFLOW: u8 = 2;
pub const E_CANNOT_OPEN_FILE: u8 = 3;
pub const E_INVALID_ARGUMENT: u8 = 4;
pub const E_INVALID_UTF8: u8 = 5;
pub const E_PREMATURE_END: u8 = 6;
pub const E_END_OF_INPUT_EXPECTED: u8 = 7;
pub const E_INVALID_SYNTAX: u8 = 8;
pub const E_INVALID_FORMAT: u8 = 9;
pub const E_WRONG_TYPE: u8 = 10;
pub const E_NULL_CHARACTER: u8 = 11;
pub const E_NULL_VALUE: u8 = 12;
pub const E_NULL_BYTE_IN_KEY: u8 = 13;
pub const E_DUPLICATE_KEY: u8 = 14;
pub const E_NUMERIC_OVERFLOW: u8 = 15;
pub const E_ITEM_NOT_FOUND: u8 = 16;
pub const E_INDEX_OUT_OF_RANGE: u8 = 17;

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

/* ------------------------------------------------------------------ */
/* the Api table                                                      */
/* ------------------------------------------------------------------ */

macro_rules! api {
    ( $( $name:ident : $t:ty ),* $(,)? ) => {
        pub struct Api {
            pub tag: &'static str,
            $( pub $name : $t, )*
            /* data symbols */
            pub hashtable_seed: *mut u32,
            pub dtoa_divmax: *mut c_int,
            _lib: &'static libloading::Library,
        }
        impl Api {
            pub fn load(path: &std::path::Path, tag: &'static str) -> Api {
                let lib: &'static libloading::Library = Box::leak(Box::new(unsafe {
                    libloading::Library::new(path)
                }.unwrap_or_else(|e| panic!("dlopen {}: {e}", path.display()))));
                unsafe {
                    Api {
                        tag,
                        $( $name : *lib.get::<$t>(
                                concat!(stringify!($name), "\0").as_bytes()
                            ).unwrap_or_else(|e| panic!("{}: {}: {e}", tag, stringify!($name))), )*
                        hashtable_seed: *lib.get::<*mut u32>(b"hashtable_seed\0").unwrap(),
                        dtoa_divmax: *lib.get::<*mut c_int>(b"dtoa_divmax\0").unwrap(),
                        _lib: lib,
                    }
                }
            }
        }
    }
}

api! {
    /* ---- construction / destruction ---- */
    json_object: unsafe extern "C" fn() -> Jt,
    json_array: unsafe extern "C" fn() -> Jt,
    json_string: unsafe extern "C" fn(*const c_char) -> Jt,
    json_stringn: unsafe extern "C" fn(*const c_char, usize) -> Jt,
    json_string_nocheck: unsafe extern "C" fn(*const c_char) -> Jt,
    json_stringn_nocheck: unsafe extern "C" fn(*const c_char, usize) -> Jt,
    json_integer: unsafe extern "C" fn(i64) -> Jt,
    json_real: unsafe extern "C" fn(f64) -> Jt,
    json_true: unsafe extern "C" fn() -> Jt,
    json_false: unsafe extern "C" fn() -> Jt,
    json_null: unsafe extern "C" fn() -> Jt,
    json_delete: unsafe extern "C" fn(Jt),

    /* ---- object ---- */
    json_object_seed: unsafe extern "C" fn(usize),
    json_object_size: unsafe extern "C" fn(Jt) -> usize,
    json_object_get: unsafe extern "C" fn(Jt, *const c_char) -> Jt,
    json_object_getn: unsafe extern "C" fn(Jt, *const c_char, usize) -> Jt,
    json_object_set_new: unsafe extern "C" fn(Jt, *const c_char, Jt) -> c_int,
    json_object_setn_new: unsafe extern "C" fn(Jt, *const c_char, usize, Jt) -> c_int,
    json_object_set_new_nocheck: unsafe extern "C" fn(Jt, *const c_char, Jt) -> c_int,
    json_object_setn_new_nocheck: unsafe extern "C" fn(Jt, *const c_char, usize, Jt) -> c_int,
    json_object_del: unsafe extern "C" fn(Jt, *const c_char) -> c_int,
    json_object_deln: unsafe extern "C" fn(Jt, *const c_char, usize) -> c_int,
    json_object_clear: unsafe extern "C" fn(Jt) -> c_int,
    json_object_update: unsafe extern "C" fn(Jt, Jt) -> c_int,
    json_object_update_existing: unsafe extern "C" fn(Jt, Jt) -> c_int,
    json_object_update_missing: unsafe extern "C" fn(Jt, Jt) -> c_int,
    json_object_update_recursive: unsafe extern "C" fn(Jt, Jt) -> c_int,
    do_object_update_recursive: unsafe extern "C" fn(Jt, Jt, *mut HashtableT) -> c_int,
    json_object_iter: unsafe extern "C" fn(Jt) -> *mut c_void,
    json_object_iter_at: unsafe extern "C" fn(Jt, *const c_char) -> *mut c_void,
    json_object_key_to_iter: unsafe extern "C" fn(*const c_char) -> *mut c_void,
    json_object_iter_next: unsafe extern "C" fn(Jt, *mut c_void) -> *mut c_void,
    json_object_iter_key: unsafe extern "C" fn(*mut c_void) -> *const c_char,
    json_object_iter_key_len: unsafe extern "C" fn(*mut c_void) -> usize,
    json_object_iter_value: unsafe extern "C" fn(*mut c_void) -> Jt,
    json_object_iter_set_new: unsafe extern "C" fn(Jt, *mut c_void, Jt) -> c_int,

    /* ---- array ---- */
    json_array_size: unsafe extern "C" fn(Jt) -> usize,
    json_array_get: unsafe extern "C" fn(Jt, usize) -> Jt,
    json_array_set_new: unsafe extern "C" fn(Jt, usize, Jt) -> c_int,
    json_array_append_new: unsafe extern "C" fn(Jt, Jt) -> c_int,
    json_array_insert_new: unsafe extern "C" fn(Jt, usize, Jt) -> c_int,
    json_array_remove: unsafe extern "C" fn(Jt, usize) -> c_int,
    json_array_clear: unsafe extern "C" fn(Jt) -> c_int,
    json_array_extend: unsafe extern "C" fn(Jt, Jt) -> c_int,

    /* ---- scalars ---- */
    json_string_value: unsafe extern "C" fn(Jt) -> *const c_char,
    json_string_length: unsafe extern "C" fn(Jt) -> usize,
    json_integer_value: unsafe extern "C" fn(Jt) -> i64,
    json_real_value: unsafe extern "C" fn(Jt) -> f64,
    json_number_value: unsafe extern "C" fn(Jt) -> f64,
    json_string_set: unsafe extern "C" fn(Jt, *const c_char) -> c_int,
    json_string_setn: unsafe extern "C" fn(Jt, *const c_char, usize) -> c_int,
    json_string_set_nocheck: unsafe extern "C" fn(Jt, *const c_char) -> c_int,
    json_string_setn_nocheck: unsafe extern "C" fn(Jt, *const c_char, usize) -> c_int,
    json_integer_set: unsafe extern "C" fn(Jt, i64) -> c_int,
    json_real_set: unsafe extern "C" fn(Jt, f64) -> c_int,

    /* ---- equality / copy ---- */
    json_equal: unsafe extern "C" fn(Jt, Jt) -> c_int,
    json_copy: unsafe extern "C" fn(Jt) -> Jt,
    json_deep_copy: unsafe extern "C" fn(Jt) -> Jt,
    do_deep_copy: unsafe extern "C" fn(Jt, *mut HashtableT) -> Jt,

    /* ---- encoding ---- */
    json_dumps: unsafe extern "C" fn(Jt, usize) -> *mut c_char,
    json_dumpb: unsafe extern "C" fn(Jt, *mut c_char, usize, usize) -> usize,
    json_dumpf: unsafe extern "C" fn(Jt, *mut c_void, usize) -> c_int,
    json_dumpfd: unsafe extern "C" fn(Jt, c_int, usize) -> c_int,
    json_dump_file: unsafe extern "C" fn(Jt, *const c_char, usize) -> c_int,
    json_dump_callback: unsafe extern "C" fn(Jt, DumpCb, *mut c_void, usize) -> c_int,

    /* ---- decoding ---- */
    json_loads: unsafe extern "C" fn(*const c_char, usize, *mut JsonError) -> Jt,
    json_loadb: unsafe extern "C" fn(*const c_char, usize, usize, *mut JsonError) -> Jt,
    json_loadf: unsafe extern "C" fn(*mut c_void, usize, *mut JsonError) -> Jt,
    json_loadfd: unsafe extern "C" fn(c_int, usize, *mut JsonError) -> Jt,
    json_load_file: unsafe extern "C" fn(*const c_char, usize, *mut JsonError) -> Jt,
    json_load_callback: unsafe extern "C" fn(LoadCb, *mut c_void, usize, *mut JsonError) -> Jt,

    /* ---- pack / unpack (variadic) ---- */
    json_pack: unsafe extern "C" fn(*const c_char, ...) -> Jt,
    json_pack_ex: unsafe extern "C" fn(*mut JsonError, usize, *const c_char, ...) -> Jt,
    json_unpack: unsafe extern "C" fn(Jt, *const c_char, ...) -> c_int,
    json_unpack_ex: unsafe extern "C" fn(Jt, *mut JsonError, usize, *const c_char, ...) -> c_int,
    json_sprintf: unsafe extern "C" fn(*const c_char, ...) -> Jt,
    /* v-variants: addresses only, invoked through the C shim */
    json_vpack_ex: unsafe extern "C" fn() -> *mut c_void,
    json_vunpack_ex: unsafe extern "C" fn() -> *mut c_void,
    json_vsprintf: unsafe extern "C" fn() -> *mut c_void,

    /* ---- allocators ---- */
    json_set_alloc_funcs: unsafe extern "C" fn(MallocFn, FreeFn),
    json_get_alloc_funcs: unsafe extern "C" fn(*mut MallocFn, *mut FreeFn),
    json_set_alloc_funcs2: unsafe extern "C" fn(MallocFn, ReallocFn, FreeFn),
    json_get_alloc_funcs2: unsafe extern "C" fn(*mut MallocFn, *mut ReallocFn, *mut FreeFn),

    /* ---- version ---- */
    jansson_version_str: unsafe extern "C" fn() -> *const c_char,
    jansson_version_cmp: unsafe extern "C" fn(c_int, c_int, c_int) -> c_int,

    /* ---- private: memory ---- */
    jsonp_malloc: unsafe extern "C" fn(usize) -> *mut c_void,
    jsonp_realloc: unsafe extern "C" fn(*mut c_void, usize, usize) -> *mut c_void,
    jsonp_free: unsafe extern "C" fn(*mut c_void),
    jsonp_strndup: unsafe extern "C" fn(*const c_char, usize) -> *mut c_char,
    jsonp_stringn_nocheck_own: unsafe extern "C" fn(*const c_char, usize) -> Jt,

    /* ---- private: error ---- */
    jsonp_error_init: unsafe extern "C" fn(*mut JsonError, *const c_char),
    jsonp_error_set_source: unsafe extern "C" fn(*mut JsonError, *const c_char),
    jsonp_error_set: unsafe extern "C" fn(*mut JsonError, c_int, c_int, usize, c_int, *const c_char, ...),
    jsonp_error_vset: unsafe extern "C" fn() -> *mut c_void,

    /* ---- private: strconv ---- */
    jsonp_strtod: unsafe extern "C" fn(*mut StrbufferT, *mut f64) -> c_int,
    jsonp_dtostr: unsafe extern "C" fn(*mut c_char, usize, f64, c_int) -> c_int,

    /* ---- private: loop check ---- */
    jsonp_loop_check: unsafe extern "C" fn(*mut HashtableT, Jt, *mut c_char, usize, *mut usize) -> c_int,

    /* ---- private: strbuffer ---- */
    strbuffer_init: unsafe extern "C" fn(*mut StrbufferT) -> c_int,
    strbuffer_close: unsafe extern "C" fn(*mut StrbufferT),
    strbuffer_clear: unsafe extern "C" fn(*mut StrbufferT),
    strbuffer_value: unsafe extern "C" fn(*const StrbufferT) -> *const c_char,
    strbuffer_steal_value: unsafe extern "C" fn(*mut StrbufferT) -> *mut c_char,
    strbuffer_append_byte: unsafe extern "C" fn(*mut StrbufferT, c_char) -> c_int,
    strbuffer_append_bytes: unsafe extern "C" fn(*mut StrbufferT, *const c_char, usize) -> c_int,
    strbuffer_pop: unsafe extern "C" fn(*mut StrbufferT) -> c_char,

    /* ---- private: utf ---- */
    utf8_encode: unsafe extern "C" fn(i32, *mut c_char, *mut usize) -> c_int,
    utf8_check_first: unsafe extern "C" fn(c_char) -> usize,
    utf8_check_full: unsafe extern "C" fn(*const c_char, usize, *mut i32) -> usize,
    utf8_iterate: unsafe extern "C" fn(*const c_char, usize, *mut i32) -> *const c_char,
    utf8_check_string: unsafe extern "C" fn(*const c_char, usize) -> c_int,

    /* ---- private: hashtable ---- */
    hashtable_init: unsafe extern "C" fn(*mut HashtableT) -> c_int,
    hashtable_close: unsafe extern "C" fn(*mut HashtableT),
    hashtable_set: unsafe extern "C" fn(*mut HashtableT, *const c_char, usize, Jt) -> c_int,
    hashtable_get: unsafe extern "C" fn(*mut HashtableT, *const c_char, usize) -> *mut c_void,
    hashtable_del: unsafe extern "C" fn(*mut HashtableT, *const c_char, usize) -> c_int,
    hashtable_clear: unsafe extern "C" fn(*mut HashtableT),
    hashtable_iter: unsafe extern "C" fn(*mut HashtableT) -> *mut c_void,
    hashtable_iter_at: unsafe extern "C" fn(*mut HashtableT, *const c_char, usize) -> *mut c_void,
    hashtable_iter_next: unsafe extern "C" fn(*mut HashtableT, *mut c_void) -> *mut c_void,
    hashtable_iter_key: unsafe extern "C" fn(*mut c_void) -> *const c_char,
    hashtable_iter_key_len: unsafe extern "C" fn(*mut c_void) -> usize,
    hashtable_iter_value: unsafe extern "C" fn(*mut c_void) -> *mut c_void,
    hashtable_iter_set: unsafe extern "C" fn(*mut c_void, Jt),

    /* ---- private: dtoa ---- */
    dtoa: unsafe extern "C" fn(f64, c_int, c_int, *mut c_int, *mut c_int, *mut *mut c_char) -> *mut c_char,
    dtoa_r: unsafe extern "C" fn(f64, c_int, c_int, *mut c_int, *mut c_int, *mut *mut c_char, *mut c_char, usize) -> *mut c_char,
    freedtoa: unsafe extern "C" fn(*mut c_char),
    gethex: unsafe extern "C" fn(*mut *const c_char, *mut f64, c_int, c_int),
    strtod__unused: unsafe extern "C" fn(*const c_char, *mut *mut c_char) -> f64,
}

/* ------------------------------------------------------------------ */
/* loading                                                            */
/* ------------------------------------------------------------------ */

pub fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn c_so() -> PathBuf {
    let p = workspace_root().join("c_src/build/libjansson.so");
    assert!(
        p.exists(),
        "C shared library missing at {} — build it with:\n  cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

fn rust_so() -> PathBuf {
    let p = workspace_root().join("translation/target/release/libjansson.so");
    assert!(
        p.exists(),
        "Rust shared library missing at {} — build it with:\n  cd translation && cargo build --release",
        p.display()
    );
    p
}

pub struct Pair {
    pub c: &'static Api,
    pub r: &'static Api,
}

static ONCE: std::sync::OnceLock<Pair> = std::sync::OnceLock::new();
unsafe impl Send for Pair {}
unsafe impl Sync for Pair {}
unsafe impl Send for Api {}
unsafe impl Sync for Api {}

/// The fixed hash seed both libraries are pinned to, so any hash-order-visible
/// behaviour is compared apples-to-apples.
pub const TEST_SEED: usize = 0x5eed_1234;

pub fn pair() -> &'static Pair {
    ONCE.get_or_init(|| {
        let c: &'static Api = Box::leak(Box::new(Api::load(&c_so(), "C")));
        let r: &'static Api = Box::leak(Box::new(Api::load(&rust_so(), "Rust")));
        unsafe {
            // Pin both hash seeds before any object is created.
            (c.json_object_seed)(TEST_SEED);
            (r.json_object_seed)(TEST_SEED);
        }
        Pair { c, r }
    })
}

/// Both libraries carry process-global mutable state that is deliberately NOT
/// thread safe in the C original: dtoa's `Balloc` freelist / `dtoa_result` /
/// `p5s` statics (`MULTIPLE_THREADS` is undefined), and `memory.c`'s
/// `do_malloc/do_realloc/do_free` pointers.  Tests inside one binary run on
/// several threads, so every test body takes this lock; otherwise the C side
/// races with itself and produces garbage that no translation could match.
pub fn lock() -> std::sync::MutexGuard<'static, ()> {
    static M: std::sync::Mutex<()> = std::sync::Mutex::new(());
    M.lock().unwrap_or_else(|e| e.into_inner())
}

/// Runs `f` against both libraries and returns `(c_result, rust_result)`.
pub fn both<T, F: Fn(&'static Api) -> T>(f: F) -> (T, T) {
    let p = pair();
    (f(p.c), f(p.r))
}

/// Runs `f` against both libraries and asserts the results are equal.
#[track_caller]
pub fn same<T: PartialEq + std::fmt::Debug, F: Fn(&'static Api) -> T>(what: &str, f: F) {
    let (a, b) = both(f);
    assert_eq!(a, b, "divergence in {what}");
}

/* ------------------------------------------------------------------ */
/* small helpers                                                      */
/* ------------------------------------------------------------------ */

pub fn cstr(s: &str) -> CString {
    CString::new(s).unwrap()
}

/// NUL-terminated byte vector from arbitrary bytes (NULs allowed inside).
pub fn nul_terminated(b: &[u8]) -> Vec<c_char> {
    let mut v: Vec<c_char> = b.iter().map(|&x| x as c_char).collect();
    v.push(0);
    v
}

/// Read a `char*` returned by the library into an owned `Vec<u8>` and free it
/// with that library's own `jsonp_free`.
pub unsafe fn take_cstring(api: &Api, p: *mut c_char) -> Option<Vec<u8>> {
    if p.is_null() {
        return None;
    }
    let out = unsafe { CStr::from_ptr(p) }.to_bytes().to_vec();
    unsafe { (api.jsonp_free)(p as *mut c_void) };
    Some(out)
}

/// `json_dumps` on one library, as owned bytes.
pub unsafe fn dumps(api: &Api, j: Jt, flags: usize) -> Option<Vec<u8>> {
    unsafe { take_cstring(api, (api.json_dumps)(j, flags)) }
}

/// `json_decref` re-implemented on the caller side (it is a static inline in
/// `jansson.h`, so it is not an exported symbol).  Uses the library's own
/// `json_delete`.
pub unsafe fn decref(api: &Api, j: Jt) {
    unsafe {
        if !j.is_null() && (*j).refcount != usize::MAX {
            (*j).refcount -= 1;
            if (*j).refcount == 0 {
                (api.json_delete)(j);
            }
        }
    }
}

pub unsafe fn incref(api: &Api, j: Jt) -> Jt {
    let _ = api;
    unsafe {
        if !j.is_null() && (*j).refcount != usize::MAX {
            (*j).refcount += 1;
        }
        j
    }
}

/* ------------------------------------------------------------------ */
/* minimal libc access (for FILE* based entry points)                 */
/* ------------------------------------------------------------------ */

pub struct Libc {
    pub fopen: unsafe extern "C" fn(*const c_char, *const c_char) -> *mut c_void,
    pub fclose: unsafe extern "C" fn(*mut c_void) -> c_int,
    pub fflush: unsafe extern "C" fn(*mut c_void) -> c_int,
    pub rewind: unsafe extern "C" fn(*mut c_void),
    pub fread: unsafe extern "C" fn(*mut c_void, usize, usize, *mut c_void) -> usize,
    pub malloc: unsafe extern "C" fn(usize) -> *mut c_void,
    pub realloc: unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void,
    pub free: unsafe extern "C" fn(*mut c_void),
    _lib: &'static libloading::Library,
}

unsafe impl Send for Libc {}
unsafe impl Sync for Libc {}

static LIBC: std::sync::OnceLock<Libc> = std::sync::OnceLock::new();

pub fn libc() -> &'static Libc {
    LIBC.get_or_init(|| {
        let lib: &'static libloading::Library = Box::leak(Box::new(unsafe {
            libloading::Library::new("libc.so.6")
        }
        .expect("dlopen libc.so.6")));
        unsafe {
            Libc {
                fopen: *lib.get(b"fopen\0").unwrap(),
                fclose: *lib.get(b"fclose\0").unwrap(),
                fflush: *lib.get(b"fflush\0").unwrap(),
                rewind: *lib.get(b"rewind\0").unwrap(),
                fread: *lib.get(b"fread\0").unwrap(),
                malloc: *lib.get(b"malloc\0").unwrap(),
                realloc: *lib.get(b"realloc\0").unwrap(),
                free: *lib.get(b"free\0").unwrap(),
                _lib: lib,
            }
        }
    })
}

/// A unique temporary path inside the crate's target dir.
pub fn temp_path(tag: &str) -> PathBuf {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static N: AtomicUsize = AtomicUsize::new(0);
    let d = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/difftest-tmp");
    std::fs::create_dir_all(&d).unwrap();
    d.join(format!(
        "{tag}-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ))
}

/* ------------------------------------------------------------------ */
/* C shim: lets the tests call the `va_list` entry points             */
/* (json_vpack_ex / json_vunpack_ex / json_vsprintf) which Rust       */
/* cannot construct a va_list for on stable.                          */
/* ------------------------------------------------------------------ */

const VSHIM_C: &str = r#"
#include <stdarg.h>
#include <stddef.h>

typedef void *(*vpack_t)(void *, size_t, const char *, va_list);
typedef int (*vunpack_t)(void *, void *, size_t, const char *, va_list);
typedef void *(*vsprintf_t)(const char *, va_list);

static void *cvp(vpack_t f, void *e, size_t fl, const char *fmt, ...) {
    va_list ap; void *r;
    va_start(ap, fmt);
    r = f(e, fl, fmt, ap);
    va_end(ap);
    return r;
}
void *shim_vpack_0(vpack_t f, void *e, size_t fl, const char *fmt) {
    return cvp(f, e, fl, fmt);
}
void *shim_vpack_i(vpack_t f, void *e, size_t fl, const char *fmt, int a) {
    return cvp(f, e, fl, fmt, a);
}
void *shim_vpack_s(vpack_t f, void *e, size_t fl, const char *fmt, const char *a) {
    return cvp(f, e, fl, fmt, a);
}
void *shim_vpack_si(vpack_t f, void *e, size_t fl, const char *fmt, const char *a, int b) {
    return cvp(f, e, fl, fmt, a, b);
}
void *shim_vpack_sisi(vpack_t f, void *e, size_t fl, const char *fmt,
                      const char *a, int b, const char *c, int d) {
    return cvp(f, e, fl, fmt, a, b, c, d);
}
void *shim_vpack_d(vpack_t f, void *e, size_t fl, const char *fmt, double a) {
    return cvp(f, e, fl, fmt, a);
}
void *shim_vpack_sd(vpack_t f, void *e, size_t fl, const char *fmt, const char *a, double b) {
    return cvp(f, e, fl, fmt, a, b);
}
void *shim_vpack_p(vpack_t f, void *e, size_t fl, const char *fmt, void *a) {
    return cvp(f, e, fl, fmt, a);
}

static int cvu(vunpack_t f, void *root, void *e, size_t fl, const char *fmt, ...) {
    va_list ap; int r;
    va_start(ap, fmt);
    r = f(root, e, fl, fmt, ap);
    va_end(ap);
    return r;
}
int shim_vunpack_0(vunpack_t f, void *root, void *e, size_t fl, const char *fmt) {
    return cvu(f, root, e, fl, fmt);
}
int shim_vunpack_p(vunpack_t f, void *root, void *e, size_t fl, const char *fmt, void *a) {
    return cvu(f, root, e, fl, fmt, a);
}
int shim_vunpack_pp(vunpack_t f, void *root, void *e, size_t fl, const char *fmt,
                    void *a, void *b) {
    return cvu(f, root, e, fl, fmt, a, b);
}
int shim_vunpack_sp(vunpack_t f, void *root, void *e, size_t fl, const char *fmt,
                    const char *k, void *a) {
    return cvu(f, root, e, fl, fmt, k, a);
}
int shim_vunpack_spsp(vunpack_t f, void *root, void *e, size_t fl, const char *fmt,
                      const char *k1, void *a1, const char *k2, void *a2) {
    return cvu(f, root, e, fl, fmt, k1, a1, k2, a2);
}

static void *cvs(vsprintf_t f, const char *fmt, ...) {
    va_list ap; void *r;
    va_start(ap, fmt);
    r = f(fmt, ap);
    va_end(ap);
    return r;
}
void *shim_vsprintf_0(vsprintf_t f, const char *fmt) { return cvs(f, fmt); }
void *shim_vsprintf_s(vsprintf_t f, const char *fmt, const char *a) { return cvs(f, fmt, a); }
void *shim_vsprintf_i(vsprintf_t f, const char *fmt, int a) { return cvs(f, fmt, a); }
void *shim_vsprintf_si(vsprintf_t f, const char *fmt, const char *a, int b) {
    return cvs(f, fmt, a, b);
}
void *shim_vsprintf_d(vsprintf_t f, const char *fmt, double a) { return cvs(f, fmt, a); }
"#;

type FnAddr = *mut c_void;

pub struct VShim {
    pub vpack_0: unsafe extern "C" fn(FnAddr, *mut JsonError, usize, *const c_char) -> Jt,
    pub vpack_i: unsafe extern "C" fn(FnAddr, *mut JsonError, usize, *const c_char, c_int) -> Jt,
    pub vpack_s:
        unsafe extern "C" fn(FnAddr, *mut JsonError, usize, *const c_char, *const c_char) -> Jt,
    pub vpack_si: unsafe extern "C" fn(
        FnAddr,
        *mut JsonError,
        usize,
        *const c_char,
        *const c_char,
        c_int,
    ) -> Jt,
    pub vpack_sisi: unsafe extern "C" fn(
        FnAddr,
        *mut JsonError,
        usize,
        *const c_char,
        *const c_char,
        c_int,
        *const c_char,
        c_int,
    ) -> Jt,
    pub vpack_d: unsafe extern "C" fn(FnAddr, *mut JsonError, usize, *const c_char, f64) -> Jt,
    pub vpack_sd:
        unsafe extern "C" fn(FnAddr, *mut JsonError, usize, *const c_char, *const c_char, f64) -> Jt,
    pub vpack_p:
        unsafe extern "C" fn(FnAddr, *mut JsonError, usize, *const c_char, *mut c_void) -> Jt,

    pub vunpack_0: unsafe extern "C" fn(FnAddr, Jt, *mut JsonError, usize, *const c_char) -> c_int,
    pub vunpack_p:
        unsafe extern "C" fn(FnAddr, Jt, *mut JsonError, usize, *const c_char, *mut c_void) -> c_int,
    pub vunpack_pp: unsafe extern "C" fn(
        FnAddr,
        Jt,
        *mut JsonError,
        usize,
        *const c_char,
        *mut c_void,
        *mut c_void,
    ) -> c_int,
    pub vunpack_sp: unsafe extern "C" fn(
        FnAddr,
        Jt,
        *mut JsonError,
        usize,
        *const c_char,
        *const c_char,
        *mut c_void,
    ) -> c_int,
    pub vunpack_spsp: unsafe extern "C" fn(
        FnAddr,
        Jt,
        *mut JsonError,
        usize,
        *const c_char,
        *const c_char,
        *mut c_void,
        *const c_char,
        *mut c_void,
    ) -> c_int,

    pub vsprintf_0: unsafe extern "C" fn(FnAddr, *const c_char) -> Jt,
    pub vsprintf_s: unsafe extern "C" fn(FnAddr, *const c_char, *const c_char) -> Jt,
    pub vsprintf_i: unsafe extern "C" fn(FnAddr, *const c_char, c_int) -> Jt,
    pub vsprintf_si: unsafe extern "C" fn(FnAddr, *const c_char, *const c_char, c_int) -> Jt,
    pub vsprintf_d: unsafe extern "C" fn(FnAddr, *const c_char, f64) -> Jt,
    _lib: &'static libloading::Library,
}

unsafe impl Send for VShim {}
unsafe impl Sync for VShim {}

static VSHIM: std::sync::OnceLock<VShim> = std::sync::OnceLock::new();

pub fn vshim() -> &'static VShim {
    VSHIM.get_or_init(|| {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/difftest-tmp");
        std::fs::create_dir_all(&dir).unwrap();
        let src = dir.join("vshim.c");
        let so = dir.join("libvshim.so");
        std::fs::write(&src, VSHIM_C).unwrap();
        let out = std::process::Command::new("cc")
            .args(["-shared", "-fPIC", "-O1", "-o"])
            .arg(&so)
            .arg(&src)
            .output()
            .expect("cc not available — needed to build the va_list shim");
        assert!(
            out.status.success(),
            "cc failed building the va_list shim:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let lib: &'static libloading::Library =
            Box::leak(Box::new(unsafe { libloading::Library::new(&so) }.unwrap()));
        unsafe {
            VShim {
                vpack_0: *lib.get(b"shim_vpack_0\0").unwrap(),
                vpack_i: *lib.get(b"shim_vpack_i\0").unwrap(),
                vpack_s: *lib.get(b"shim_vpack_s\0").unwrap(),
                vpack_si: *lib.get(b"shim_vpack_si\0").unwrap(),
                vpack_sisi: *lib.get(b"shim_vpack_sisi\0").unwrap(),
                vpack_d: *lib.get(b"shim_vpack_d\0").unwrap(),
                vpack_sd: *lib.get(b"shim_vpack_sd\0").unwrap(),
                vpack_p: *lib.get(b"shim_vpack_p\0").unwrap(),
                vunpack_0: *lib.get(b"shim_vunpack_0\0").unwrap(),
                vunpack_p: *lib.get(b"shim_vunpack_p\0").unwrap(),
                vunpack_pp: *lib.get(b"shim_vunpack_pp\0").unwrap(),
                vunpack_sp: *lib.get(b"shim_vunpack_sp\0").unwrap(),
                vunpack_spsp: *lib.get(b"shim_vunpack_spsp\0").unwrap(),
                vsprintf_0: *lib.get(b"shim_vsprintf_0\0").unwrap(),
                vsprintf_s: *lib.get(b"shim_vsprintf_s\0").unwrap(),
                vsprintf_i: *lib.get(b"shim_vsprintf_i\0").unwrap(),
                vsprintf_si: *lib.get(b"shim_vsprintf_si\0").unwrap(),
                vsprintf_d: *lib.get(b"shim_vsprintf_d\0").unwrap(),
                _lib: lib,
            }
        }
    })
}

/* ------------------------------------------------------------------ */
/* deterministic RNG (SplitMix64) — property-style testing            */
/* ------------------------------------------------------------------ */

pub struct Rng(pub u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }
    pub fn range(&mut self, lo: i64, hi: i64) -> i64 {
        lo + (self.next_u64() % ((hi - lo + 1) as u64)) as i64
    }
    pub fn i64(&mut self) -> i64 {
        self.next_u64() as i64
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
    /// A finite `f64` spanning the whole exponent range.
    pub fn finite_f64(&mut self) -> f64 {
        loop {
            let v = f64::from_bits(self.next_u64());
            if v.is_finite() {
                return v;
            }
        }
    }
    /// A "human" double: small magnitude, few significant digits.
    pub fn tame_f64(&mut self) -> f64 {
        let m = self.range(-1_000_000, 1_000_000) as f64;
        let e = self.range(-20, 20) as i32;
        let v = m * 10f64.powi(e);
        if v.is_finite() { v } else { 0.0 }
    }
    pub fn ascii_string(&mut self, maxlen: usize) -> String {
        let n = self.below(maxlen + 1);
        (0..n)
            .map(|_| {
                let c = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_-.";
                c[self.below(c.len())] as char
            })
            .collect()
    }
    /// A string containing every interesting escape / UTF-8 class.
    pub fn spicy_string(&mut self, maxlen: usize) -> String {
        let pool: &[&str] = &[
            "a", "Z", "0", " ", "/", "\\", "\"", "\n", "\r", "\t", "\u{8}", "\u{c}", "\u{1}",
            "\u{1f}", "\u{7f}", "é", "ß", "€", "中", "\u{fffd}", "𝄞", "😀", "\u{10ffff}",
        ];
        let n = self.below(maxlen + 1);
        (0..n).map(|_| pool[self.below(pool.len())]).collect()
    }
    pub fn bytes(&mut self, len: usize) -> Vec<u8> {
        (0..len).map(|_| (self.next_u64() & 0xFF) as u8).collect()
    }
}

/* ------------------------------------------------------------------ */
/* random JSON document generator (text form, always valid)           */
/* ------------------------------------------------------------------ */

fn json_escape(s: &str) -> String {
    let mut out = String::from("\"");
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04X}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Random, syntactically valid JSON text.  `spicy` turns on the full escape /
/// UTF-8 pool for strings and keys.
pub fn random_json(rng: &mut Rng, depth: usize, spicy: bool) -> String {
    let leaf = depth == 0 || rng.below(100) < 40;
    if leaf {
        match rng.below(7) {
            0 => "null".into(),
            1 => "true".into(),
            2 => "false".into(),
            3 => format!("{}", rng.i64()),
            4 => {
                let d = rng.tame_f64();
                let s = format!("{:?}", d);
                if s.contains("inf") || s.contains("NaN") {
                    "0.0".into()
                } else {
                    s
                }
            }
            5 => json_escape(&if spicy {
                rng.spicy_string(8)
            } else {
                rng.ascii_string(8)
            }),
            _ => format!("{}", rng.range(-1000, 1000)),
        }
    } else if rng.bool() {
        let n = rng.below(6);
        let items: Vec<String> = (0..n).map(|_| random_json(rng, depth - 1, spicy)).collect();
        format!("[{}]", items.join(","))
    } else {
        let n = rng.below(6);
        let mut used = std::collections::HashSet::new();
        let mut items = Vec::new();
        for _ in 0..n {
            let k = if spicy {
                rng.spicy_string(6)
            } else {
                rng.ascii_string(6)
            };
            if !used.insert(k.clone()) {
                continue;
            }
            items.push(format!(
                "{}:{}",
                json_escape(&k),
                random_json(rng, depth - 1, spicy)
            ));
        }
        format!("{{{}}}", items.join(","))
    }
}

/// A fixed corpus of valid documents covering the shapes `load.c` special-cases.
pub fn corpus() -> Vec<String> {
    let mut v: Vec<String> = vec![
        "{}".into(),
        "[]".into(),
        "[[]]".into(),
        "[{}]".into(),
        "{\"a\":{}}".into(),
        "{\"a\":[]}".into(),
        "[1]".into(),
        "[0]".into(),
        "[-0]".into(),
        "[1,2,3,4,5,6,7,8,9,10]".into(),
        "[9223372036854775807]".into(),
        "[-9223372036854775808]".into(),
        "[1.0]".into(),
        "[-1.5]".into(),
        "[1e10]".into(),
        "[1E+10]".into(),
        "[1e-10]".into(),
        "[-1.5e-3]".into(),
        "[0.1]".into(),
        "[1e16]".into(),
        "[1e17]".into(),
        "[1e-4]".into(),
        "[1e-5]".into(),
        "[3.141592653589793]".into(),
        "[2.2250738585072014e-308]".into(),
        "[1.7976931348623157e308]".into(),
        "[5e-324]".into(),
        "[true,false,null]".into(),
        "[\"\"]".into(),
        "[\"a\"]".into(),
        "[\"\\\"\\\\\\/\\b\\f\\n\\r\\t\"]".into(),
        "[\"\\u0041\"]".into(),
        "[\"\\u00e9\"]".into(),
        "[\"\\u20ac\"]".into(),
        "[\"\\ud834\\udd1e\"]".into(),
        "[\"é ß € 中 𝄞 😀\"]".into(),
        "[\"a/b\"]".into(),
        "[\"\\u007f\"]".into(),
        "{\"a\":1,\"b\":2,\"c\":3}".into(),
        "{\"c\":1,\"b\":2,\"a\":3}".into(),
        "{\"a\":1,\"aa\":2,\"aaa\":3,\"ab\":4}".into(),
        "{\"z\":1,\"y\":2,\"x\":3,\"w\":4,\"v\":5,\"u\":6,\"t\":7,\"s\":8,\"r\":9}".into(),
        "{\"a\":{\"b\":{\"c\":{\"d\":[1,2,{\"e\":null}]}}}}".into(),
        " \t\r\n [ \t\r\n 1 \t\r\n , \t\r\n 2 \t\r\n ] \t\r\n ".into(),
    ];
    // 1000-element array + 1000-key object (growth + rehash)
    v.push(format!(
        "[{}]",
        (0..1000).map(|i| i.to_string()).collect::<Vec<_>>().join(",")
    ));
    v.push(format!(
        "{{{}}}",
        (0..1000)
            .map(|i| format!("\"k{i}\":{i}"))
            .collect::<Vec<_>>()
            .join(",")
    ));
    // deep nesting at the limit
    v.push(format!("{}{}", "[".repeat(2048), "]".repeat(2048)));
    v
}
