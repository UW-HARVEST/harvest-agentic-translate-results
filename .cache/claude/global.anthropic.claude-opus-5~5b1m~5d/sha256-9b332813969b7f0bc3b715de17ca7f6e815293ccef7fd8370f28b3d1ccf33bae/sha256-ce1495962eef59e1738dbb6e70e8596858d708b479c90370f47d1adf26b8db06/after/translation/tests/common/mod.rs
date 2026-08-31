//! Shared differential-test harness.
//!
//! Loads BOTH shared objects (the C `libjansson.so` produced by CMake and the
//! Rust `libjansson.so` produced by `cargo build --release`) through
//! `libloading` and exposes every exported symbol as a function pointer, so
//! every call in every test crosses a real FFI boundary exactly as an external
//! consumer's would.

#![allow(dead_code)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use libloading::Library;
use std::ffi::{c_char, c_double, c_int, c_void, CStr, CString};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// C types mirrored from jansson.h / jansson_private.h
// ---------------------------------------------------------------------------

pub type size_t = usize;
pub type ssize_t = isize;
pub type json_int_t = i64;

#[repr(C)]
pub struct FILE {
    _p: [u8; 0],
}

/// `json_t` — the public value header.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct json_t {
    pub type_: c_int,
    pub refcount: size_t,
}

pub const JSON_OBJECT: c_int = 0;
pub const JSON_ARRAY: c_int = 1;
pub const JSON_STRING: c_int = 2;
pub const JSON_INTEGER: c_int = 3;
pub const JSON_REAL: c_int = 4;
pub const JSON_TRUE: c_int = 5;
pub const JSON_FALSE: c_int = 6;
pub const JSON_NULL: c_int = 7;

pub const JSON_ERROR_TEXT_LENGTH: usize = 160;
pub const JSON_ERROR_SOURCE_LENGTH: usize = 80;

/// `json_error_t`
#[repr(C)]
#[derive(Clone, Copy)]
pub struct json_error_t {
    pub line: c_int,
    pub column: c_int,
    pub position: c_int,
    pub source: [c_char; JSON_ERROR_SOURCE_LENGTH],
    pub text: [c_char; JSON_ERROR_TEXT_LENGTH],
}

impl Default for json_error_t {
    fn default() -> Self {
        // C callers routinely hand in an uninitialised struct; zeroing gives a
        // deterministic starting point so that a divergence in which bytes the
        // library writes is visible.
        json_error_t {
            line: 0,
            column: 0,
            position: 0,
            source: [0; JSON_ERROR_SOURCE_LENGTH],
            text: [0; JSON_ERROR_TEXT_LENGTH],
        }
    }
}

impl json_error_t {
    pub fn new() -> Self {
        Self::default()
    }

    /// Pre-fill every byte with a sentinel so that "did not touch" is
    /// distinguishable from "wrote a NUL".
    pub fn poisoned() -> Self {
        json_error_t {
            line: -12345,
            column: -12345,
            position: -12345,
            source: [0x7f; JSON_ERROR_SOURCE_LENGTH],
            text: [0x7f; JSON_ERROR_TEXT_LENGTH],
        }
    }

    pub fn text_str(&self) -> String {
        cstr_lossy(self.text.as_ptr())
    }

    pub fn source_str(&self) -> String {
        cstr_lossy(self.source.as_ptr())
    }

    /// `json_error_code()` from jansson.h — the last byte of `text`.
    pub fn code(&self) -> c_int {
        self.text[JSON_ERROR_TEXT_LENGTH - 1] as u8 as c_int
    }

    /// Everything an equality check should compare, as a printable tuple.
    pub fn snapshot(&self) -> (c_int, c_int, c_int, String, String, c_int) {
        (
            self.line,
            self.column,
            self.position,
            self.source_str(),
            self.text_str(),
            self.code(),
        )
    }

    /// The full raw byte image, for byte-identical comparison.
    pub fn raw(&self) -> Vec<u8> {
        let mut v = Vec::with_capacity(12 + JSON_ERROR_SOURCE_LENGTH + JSON_ERROR_TEXT_LENGTH);
        v.extend_from_slice(&self.line.to_ne_bytes());
        v.extend_from_slice(&self.column.to_ne_bytes());
        v.extend_from_slice(&self.position.to_ne_bytes());
        v.extend(self.source.iter().map(|&c| c as u8));
        v.extend(self.text.iter().map(|&c| c as u8));
        v
    }
}

/// `enum json_error_code`
pub const JSON_ERROR_UNKNOWN: c_int = 0;
pub const JSON_ERROR_OUT_OF_MEMORY: c_int = 1;
pub const JSON_ERROR_STACK_OVERFLOW: c_int = 2;
pub const JSON_ERROR_CANNOT_OPEN_FILE: c_int = 3;
pub const JSON_ERROR_INVALID_ARGUMENT: c_int = 4;
pub const JSON_ERROR_INVALID_UTF8: c_int = 5;
pub const JSON_ERROR_PREMATURE_END_OF_INPUT: c_int = 6;
pub const JSON_ERROR_END_OF_INPUT_EXPECTED: c_int = 7;
pub const JSON_ERROR_INVALID_SYNTAX: c_int = 8;
pub const JSON_ERROR_INVALID_FORMAT: c_int = 9;
pub const JSON_ERROR_WRONG_TYPE: c_int = 10;
pub const JSON_ERROR_NULL_CHARACTER: c_int = 11;
pub const JSON_ERROR_NULL_VALUE: c_int = 12;
pub const JSON_ERROR_NULL_BYTE_IN_KEY: c_int = 13;
pub const JSON_ERROR_DUPLICATE_KEY: c_int = 14;
pub const JSON_ERROR_NUMERIC_OVERFLOW: c_int = 15;
pub const JSON_ERROR_ITEM_NOT_FOUND: c_int = 16;
pub const JSON_ERROR_INDEX_OUT_OF_RANGE: c_int = 17;

// decoding flags
pub const JSON_REJECT_DUPLICATES: size_t = 0x1;
pub const JSON_DISABLE_EOF_CHECK: size_t = 0x2;
pub const JSON_DECODE_ANY: size_t = 0x4;
pub const JSON_DECODE_INT_AS_REAL: size_t = 0x8;
pub const JSON_ALLOW_NUL: size_t = 0x10;

// encoding flags
pub const JSON_MAX_INDENT: size_t = 0x1F;
pub const JSON_COMPACT: size_t = 0x20;
pub const JSON_ENSURE_ASCII: size_t = 0x40;
pub const JSON_SORT_KEYS: size_t = 0x80;
pub const JSON_PRESERVE_ORDER: size_t = 0x100;
pub const JSON_ENCODE_ANY: size_t = 0x200;
pub const JSON_ESCAPE_SLASH: size_t = 0x400;
pub const JSON_EMBED: size_t = 0x10000;

pub fn json_indent(n: size_t) -> size_t {
    n & JSON_MAX_INDENT
}
pub fn json_real_precision(n: size_t) -> size_t {
    (n & 0x1F) << 11
}

// pack/unpack flags
pub const JSON_VALIDATE_ONLY: size_t = 0x1;
pub const JSON_STRICT: size_t = 0x2;

pub const JSON_PARSER_MAX_DEPTH: usize = 2048;

/// `hashtable_t` from src/hashtable.h — size matters because tests allocate it.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct hashtable_list {
    pub prev: *mut hashtable_list,
    pub next: *mut hashtable_list,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct hashtable_t {
    pub size: size_t,
    pub buckets: *mut c_void,
    pub order: size_t,
    pub list: hashtable_list,
    pub ordered_list: hashtable_list,
}

impl hashtable_t {
    pub fn zeroed() -> Self {
        unsafe { std::mem::zeroed() }
    }
}

/// `strbuffer_t` from src/strbuffer.h
#[repr(C)]
#[derive(Clone, Copy)]
pub struct strbuffer_t {
    pub value: *mut c_char,
    pub length: size_t,
    pub size: size_t,
}

impl strbuffer_t {
    pub fn zeroed() -> Self {
        unsafe { std::mem::zeroed() }
    }
}

pub type json_load_callback_t =
    Option<unsafe extern "C" fn(*mut c_void, size_t, *mut c_void) -> size_t>;
pub type json_dump_callback_t =
    Option<unsafe extern "C" fn(*const c_char, size_t, *mut c_void) -> c_int>;
pub type json_malloc_t = Option<unsafe extern "C" fn(size_t) -> *mut c_void>;
pub type json_realloc_t = Option<unsafe extern "C" fn(*mut c_void, size_t) -> *mut c_void>;
pub type json_free_t = Option<unsafe extern "C" fn(*mut c_void)>;

// ---------------------------------------------------------------------------
// The API surface: one function pointer per exported symbol
// ---------------------------------------------------------------------------

macro_rules! declare_api {
    ( $( $field:ident : $ty:ty , )* ) => {
        /// Every symbol exported by one jansson shared object.
        pub struct Api {
            /// "C" or "Rust" — used in assertion messages.
            pub which: &'static str,
            $( pub $field : $ty, )*
        }

        impl Api {
            fn load(lib: &'static Library, which: &'static str) -> Api {
                Api {
                    which,
                    $( $field : unsafe {
                        *lib.get::<$ty>(concat!(stringify!($field), "\0").as_bytes())
                            .unwrap_or_else(|e| panic!(
                                "{} lib is missing symbol `{}`: {e}", which, stringify!($field)))
                    }, )*
                }
            }
        }
    };
}

declare_api! {
    // ---- version.c ----
    jansson_version_str: unsafe extern "C" fn() -> *const c_char,
    jansson_version_cmp: unsafe extern "C" fn(c_int, c_int, c_int) -> c_int,

    // ---- memory.c ----
    jsonp_malloc: unsafe extern "C" fn(size_t) -> *mut c_void,
    jsonp_realloc: unsafe extern "C" fn(*mut c_void, size_t, size_t) -> *mut c_void,
    jsonp_free: unsafe extern "C" fn(*mut c_void),
    jsonp_strndup: unsafe extern "C" fn(*const c_char, size_t) -> *mut c_char,
    json_set_alloc_funcs: unsafe extern "C" fn(json_malloc_t, json_free_t),
    json_get_alloc_funcs: unsafe extern "C" fn(*mut json_malloc_t, *mut json_free_t),
    json_set_alloc_funcs2: unsafe extern "C" fn(json_malloc_t, json_realloc_t, json_free_t),
    json_get_alloc_funcs2:
        unsafe extern "C" fn(*mut json_malloc_t, *mut json_realloc_t, *mut json_free_t),

    // ---- error.c ----
    jsonp_error_init: unsafe extern "C" fn(*mut json_error_t, *const c_char),
    jsonp_error_set_source: unsafe extern "C" fn(*mut json_error_t, *const c_char),
    jsonp_error_set: unsafe extern "C" fn(
        *mut json_error_t, c_int, c_int, size_t, c_int, *const c_char, ...),
    jsonp_error_vset: unsafe extern "C" fn(
        *mut json_error_t, c_int, c_int, size_t, c_int, *const c_char, *mut c_void),

    // ---- utf.c ----
    utf8_encode: unsafe extern "C" fn(i32, *mut c_char, *mut size_t) -> c_int,
    utf8_check_first: unsafe extern "C" fn(c_char) -> size_t,
    utf8_check_full: unsafe extern "C" fn(*const c_char, size_t, *mut i32) -> size_t,
    utf8_iterate: unsafe extern "C" fn(*const c_char, size_t, *mut i32) -> *const c_char,
    utf8_check_string: unsafe extern "C" fn(*const c_char, size_t) -> c_int,

    // ---- strbuffer.c ----
    strbuffer_init: unsafe extern "C" fn(*mut strbuffer_t) -> c_int,
    strbuffer_close: unsafe extern "C" fn(*mut strbuffer_t),
    strbuffer_clear: unsafe extern "C" fn(*mut strbuffer_t),
    strbuffer_value: unsafe extern "C" fn(*const strbuffer_t) -> *const c_char,
    strbuffer_steal_value: unsafe extern "C" fn(*mut strbuffer_t) -> *mut c_char,
    strbuffer_append_byte: unsafe extern "C" fn(*mut strbuffer_t, c_char) -> c_int,
    strbuffer_append_bytes: unsafe extern "C" fn(*mut strbuffer_t, *const c_char, size_t) -> c_int,
    strbuffer_pop: unsafe extern "C" fn(*mut strbuffer_t) -> c_char,

    // ---- hashtable_seed.c ----
    // NOTE: `hashtable_seed` is a `volatile uint32_t` DATA symbol, not a
    // function (nm shows it as `B`), so it is accessed via
    // `Api::hashtable_seed()` below rather than declared here.
    json_object_seed: unsafe extern "C" fn(size_t),

    // ---- hashtable.c ----
    hashtable_init: unsafe extern "C" fn(*mut hashtable_t) -> c_int,
    hashtable_close: unsafe extern "C" fn(*mut hashtable_t),
    hashtable_set: unsafe extern "C" fn(*mut hashtable_t, *const c_char, size_t, *mut json_t) -> c_int,
    hashtable_get: unsafe extern "C" fn(*mut hashtable_t, *const c_char, size_t) -> *mut c_void,
    hashtable_del: unsafe extern "C" fn(*mut hashtable_t, *const c_char, size_t) -> c_int,
    hashtable_clear: unsafe extern "C" fn(*mut hashtable_t),
    hashtable_iter: unsafe extern "C" fn(*mut hashtable_t) -> *mut c_void,
    hashtable_iter_at: unsafe extern "C" fn(*mut hashtable_t, *const c_char, size_t) -> *mut c_void,
    hashtable_iter_next: unsafe extern "C" fn(*mut hashtable_t, *mut c_void) -> *mut c_void,
    hashtable_iter_key: unsafe extern "C" fn(*mut c_void) -> *mut c_void,
    hashtable_iter_key_len: unsafe extern "C" fn(*mut c_void) -> size_t,
    hashtable_iter_value: unsafe extern "C" fn(*mut c_void) -> *mut c_void,
    hashtable_iter_set: unsafe extern "C" fn(*mut c_void, *mut json_t),

    // ---- value.c: construction ----
    json_object: unsafe extern "C" fn() -> *mut json_t,
    json_array: unsafe extern "C" fn() -> *mut json_t,
    json_string: unsafe extern "C" fn(*const c_char) -> *mut json_t,
    json_stringn: unsafe extern "C" fn(*const c_char, size_t) -> *mut json_t,
    json_string_nocheck: unsafe extern "C" fn(*const c_char) -> *mut json_t,
    json_stringn_nocheck: unsafe extern "C" fn(*const c_char, size_t) -> *mut json_t,
    jsonp_stringn_nocheck_own: unsafe extern "C" fn(*const c_char, size_t) -> *mut json_t,
    json_integer: unsafe extern "C" fn(json_int_t) -> *mut json_t,
    json_real: unsafe extern "C" fn(c_double) -> *mut json_t,
    json_true: unsafe extern "C" fn() -> *mut json_t,
    json_false: unsafe extern "C" fn() -> *mut json_t,
    json_null: unsafe extern "C" fn() -> *mut json_t,
    json_delete: unsafe extern "C" fn(*mut json_t),

    // ---- value.c: object ----
    json_object_size: unsafe extern "C" fn(*const json_t) -> size_t,
    json_object_get: unsafe extern "C" fn(*const json_t, *const c_char) -> *mut json_t,
    json_object_getn: unsafe extern "C" fn(*const json_t, *const c_char, size_t) -> *mut json_t,
    json_object_set_new: unsafe extern "C" fn(*mut json_t, *const c_char, *mut json_t) -> c_int,
    json_object_setn_new:
        unsafe extern "C" fn(*mut json_t, *const c_char, size_t, *mut json_t) -> c_int,
    json_object_set_new_nocheck:
        unsafe extern "C" fn(*mut json_t, *const c_char, *mut json_t) -> c_int,
    json_object_setn_new_nocheck:
        unsafe extern "C" fn(*mut json_t, *const c_char, size_t, *mut json_t) -> c_int,
    json_object_del: unsafe extern "C" fn(*mut json_t, *const c_char) -> c_int,
    json_object_deln: unsafe extern "C" fn(*mut json_t, *const c_char, size_t) -> c_int,
    json_object_clear: unsafe extern "C" fn(*mut json_t) -> c_int,
    json_object_update: unsafe extern "C" fn(*mut json_t, *mut json_t) -> c_int,
    json_object_update_existing: unsafe extern "C" fn(*mut json_t, *mut json_t) -> c_int,
    json_object_update_missing: unsafe extern "C" fn(*mut json_t, *mut json_t) -> c_int,
    json_object_update_recursive: unsafe extern "C" fn(*mut json_t, *mut json_t) -> c_int,
    do_object_update_recursive:
        unsafe extern "C" fn(*mut json_t, *mut json_t, *mut hashtable_t) -> c_int,
    json_object_iter: unsafe extern "C" fn(*mut json_t) -> *mut c_void,
    json_object_iter_at: unsafe extern "C" fn(*mut json_t, *const c_char) -> *mut c_void,
    json_object_key_to_iter: unsafe extern "C" fn(*const c_char) -> *mut c_void,
    json_object_iter_next: unsafe extern "C" fn(*mut json_t, *mut c_void) -> *mut c_void,
    json_object_iter_key: unsafe extern "C" fn(*mut c_void) -> *const c_char,
    json_object_iter_key_len: unsafe extern "C" fn(*mut c_void) -> size_t,
    json_object_iter_value: unsafe extern "C" fn(*mut c_void) -> *mut json_t,
    json_object_iter_set_new: unsafe extern "C" fn(*mut json_t, *mut c_void, *mut json_t) -> c_int,

    // ---- value.c: array ----
    json_array_size: unsafe extern "C" fn(*const json_t) -> size_t,
    json_array_get: unsafe extern "C" fn(*const json_t, size_t) -> *mut json_t,
    json_array_set_new: unsafe extern "C" fn(*mut json_t, size_t, *mut json_t) -> c_int,
    json_array_append_new: unsafe extern "C" fn(*mut json_t, *mut json_t) -> c_int,
    json_array_insert_new: unsafe extern "C" fn(*mut json_t, size_t, *mut json_t) -> c_int,
    json_array_remove: unsafe extern "C" fn(*mut json_t, size_t) -> c_int,
    json_array_clear: unsafe extern "C" fn(*mut json_t) -> c_int,
    json_array_extend: unsafe extern "C" fn(*mut json_t, *mut json_t) -> c_int,

    // ---- value.c: scalars ----
    json_string_value: unsafe extern "C" fn(*const json_t) -> *const c_char,
    json_string_length: unsafe extern "C" fn(*const json_t) -> size_t,
    json_integer_value: unsafe extern "C" fn(*const json_t) -> json_int_t,
    json_real_value: unsafe extern "C" fn(*const json_t) -> c_double,
    json_number_value: unsafe extern "C" fn(*const json_t) -> c_double,
    json_string_set: unsafe extern "C" fn(*mut json_t, *const c_char) -> c_int,
    json_string_setn: unsafe extern "C" fn(*mut json_t, *const c_char, size_t) -> c_int,
    json_string_set_nocheck: unsafe extern "C" fn(*mut json_t, *const c_char) -> c_int,
    json_string_setn_nocheck: unsafe extern "C" fn(*mut json_t, *const c_char, size_t) -> c_int,
    json_integer_set: unsafe extern "C" fn(*mut json_t, json_int_t) -> c_int,
    json_real_set: unsafe extern "C" fn(*mut json_t, c_double) -> c_int,

    // ---- value.c: equality / copying / loop check ----
    json_equal: unsafe extern "C" fn(*const json_t, *const json_t) -> c_int,
    json_copy: unsafe extern "C" fn(*mut json_t) -> *mut json_t,
    json_deep_copy: unsafe extern "C" fn(*const json_t) -> *mut json_t,
    do_deep_copy: unsafe extern "C" fn(*const json_t, *mut hashtable_t) -> *mut json_t,
    jsonp_loop_check: unsafe extern "C" fn(
        *mut hashtable_t, *const json_t, *mut c_char, size_t, *mut size_t) -> c_int,

    // ---- strconv.c ----
    jsonp_strtod: unsafe extern "C" fn(*mut strbuffer_t, *mut c_double) -> c_int,
    jsonp_dtostr: unsafe extern "C" fn(*mut c_char, size_t, c_double, c_int) -> c_int,

    // ---- dump.c ----
    json_dumps: unsafe extern "C" fn(*const json_t, size_t) -> *mut c_char,
    json_dumpb: unsafe extern "C" fn(*const json_t, *mut c_char, size_t, size_t) -> size_t,
    json_dumpf: unsafe extern "C" fn(*const json_t, *mut FILE, size_t) -> c_int,
    json_dumpfd: unsafe extern "C" fn(*const json_t, c_int, size_t) -> c_int,
    json_dump_file: unsafe extern "C" fn(*const json_t, *const c_char, size_t) -> c_int,
    json_dump_callback:
        unsafe extern "C" fn(*const json_t, json_dump_callback_t, *mut c_void, size_t) -> c_int,

    // ---- load.c ----
    json_loads: unsafe extern "C" fn(*const c_char, size_t, *mut json_error_t) -> *mut json_t,
    json_loadb:
        unsafe extern "C" fn(*const c_char, size_t, size_t, *mut json_error_t) -> *mut json_t,
    json_loadf: unsafe extern "C" fn(*mut FILE, size_t, *mut json_error_t) -> *mut json_t,
    json_loadfd: unsafe extern "C" fn(c_int, size_t, *mut json_error_t) -> *mut json_t,
    json_load_file: unsafe extern "C" fn(*const c_char, size_t, *mut json_error_t) -> *mut json_t,
    json_load_callback: unsafe extern "C" fn(
        json_load_callback_t, *mut c_void, size_t, *mut json_error_t) -> *mut json_t,

    // ---- pack_unpack.c ----
    json_pack: unsafe extern "C" fn(*const c_char, ...) -> *mut json_t,
    json_pack_ex: unsafe extern "C" fn(*mut json_error_t, size_t, *const c_char, ...) -> *mut json_t,
    json_vpack_ex:
        unsafe extern "C" fn(*mut json_error_t, size_t, *const c_char, *mut c_void) -> *mut json_t,
    json_unpack: unsafe extern "C" fn(*mut json_t, *const c_char, ...) -> c_int,
    json_unpack_ex:
        unsafe extern "C" fn(*mut json_t, *mut json_error_t, size_t, *const c_char, ...) -> c_int,
    json_vunpack_ex: unsafe extern "C" fn(
        *mut json_t, *mut json_error_t, size_t, *const c_char, *mut c_void) -> c_int,
    json_sprintf: unsafe extern "C" fn(*const c_char, ...) -> *mut json_t,
    json_vsprintf: unsafe extern "C" fn(*const c_char, *mut c_void) -> *mut json_t,

    // ---- dtoa.c ----
    dtoa: unsafe extern "C" fn(
        c_double, c_int, c_int, *mut c_int, *mut c_int, *mut *mut c_char) -> *mut c_char,
    dtoa_r: unsafe extern "C" fn(
        c_double, c_int, c_int, *mut c_int, *mut c_int, *mut *mut c_char,
        *mut c_char, size_t) -> *mut c_char,
    freedtoa: unsafe extern "C" fn(*mut c_char),
    // `gethex` returns VOID in this dtoa variant (it stores the parsed value
    // through `rvp` and advances `*sp`); it does not return a status code.
    gethex: unsafe extern "C" fn(*mut *const c_char, *mut c_void, c_int, c_int),
    strtod__unused: unsafe extern "C" fn(*const c_char, *mut *mut c_char) -> c_double,
}

impl Api {
    fn lib(&self) -> &'static Library {
        if self.which == "C" {
            c_library()
        } else {
            rust_library()
        }
    }

    /// `dtoa_divmax` is exported as *data* (`nm` reports `D`), not a function.
    pub fn dtoa_divmax(&self) -> c_int {
        unsafe { **self.lib().get::<*mut c_int>(b"dtoa_divmax\0").unwrap() }
    }

    /// `hashtable_seed` is exported as *data* — `volatile uint32_t`
    /// (`nm` reports `B`). Reading it is how tests confirm that
    /// `json_object_seed` actually installed the seed, and that both libraries
    /// ended up with the SAME seed (without which no object dump could be
    /// compared byte-for-byte).
    pub fn hashtable_seed(&self) -> u32 {
        unsafe { **self.lib().get::<*mut u32>(b"hashtable_seed\0").unwrap() }
    }
}

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

fn repo_root() -> std::path::PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ has a parent")
        .to_path_buf()
}

fn c_library() -> &'static Library {
    static L: OnceLock<Library> = OnceLock::new();
    L.get_or_init(|| {
        let p = repo_root().join("c_src/build/libjansson.so");
        assert!(
            p.exists(),
            "C shared library not found at {}. Build it with:\n  \
             cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            p.display()
        );
        unsafe { Library::new(&p) }.expect("failed to dlopen the C libjansson.so")
    })
}

fn rust_library() -> &'static Library {
    static L: OnceLock<Library> = OnceLock::new();
    L.get_or_init(|| {
        let p = repo_root().join("translation/target/release/libjansson.so");
        assert!(
            p.exists(),
            "Rust shared library not found at {}. Build it with:\n  \
             cd translation && cargo build --release",
            p.display()
        );
        assert_not_stale(&p);
        unsafe { Library::new(&p) }.expect("failed to dlopen the Rust libjansson.so")
    })
}

/// Refuse to run against a stale `.so`.
///
/// This guard matters more than it looks: `cargo test --test <name>` does NOT
/// rebuild the `cdylib`, because the integration test does not *link* it — it
/// `dlopen`s it at run time, so cargo sees no dependency. Without this check a
/// source fix could appear to pass (or a regression appear to be absent) while
/// the tests were actually exercising an older library.
fn assert_not_stale(so: &std::path::Path) {
    let so_time = match so.metadata().and_then(|m| m.modified()) {
        Ok(t) => t,
        Err(_) => return,
    };
    let src = repo_root().join("translation/src");
    let mut newest: Option<(std::time::SystemTime, std::path::PathBuf)> = None;
    if let Ok(entries) = std::fs::read_dir(&src) {
        for e in entries.flatten() {
            if e.path().extension().and_then(|s| s.to_str()) != Some("rs") {
                continue;
            }
            if let Ok(t) = e.metadata().and_then(|m| m.modified()) {
                if newest.as_ref().map_or(true, |(bt, _)| t > *bt) {
                    newest = Some((t, e.path()));
                }
            }
        }
    }
    if let Some((t, path)) = newest {
        assert!(
            t <= so_time,
            "STALE Rust .so — {} is newer than {}.\n\
             `cargo test --test <name>` does not rebuild the cdylib. Run:\n  \
             cd translation && cargo build --release\n\
             (or use ../run_tests.sh, which rebuilds both libraries first).",
            path.display(),
            so.display()
        );
    }
}

// ---------------------------------------------------------------------------
// va_list trampolines (see tests/vashim.c)
// ---------------------------------------------------------------------------

/// The four `va_list`-taking entry points cannot be called from stable Rust
/// directly, so a small C shim turns a variadic call into a real `va_list` and
/// forwards it to whichever implementation's function pointer we hand it.
pub struct VaShim {
    pub vpack_ex: unsafe extern "C" fn(*mut c_void, *mut json_error_t, size_t, *const c_char, ...)
        -> *mut json_t,
    pub vunpack_ex: unsafe extern "C" fn(
        *mut c_void,
        *mut json_t,
        *mut json_error_t,
        size_t,
        *const c_char,
        ...
    ) -> c_int,
    pub vsprintf: unsafe extern "C" fn(*mut c_void, *const c_char, ...) -> *mut json_t,
    pub error_vset: unsafe extern "C" fn(
        *mut c_void,
        *mut json_error_t,
        c_int,
        c_int,
        size_t,
        c_int,
        *const c_char,
        ...
    ),
}

pub fn vashim() -> &'static VaShim {
    static L: OnceLock<Library> = OnceLock::new();
    static S: OnceLock<VaShim> = OnceLock::new();
    let lib = L.get_or_init(|| {
        let p = repo_root().join(".work/libvashim.so");
        assert!(
            p.exists(),
            "va_list shim not built. Build it with:\n  \
             cc -shared -fPIC -O1 -o .work/libvashim.so translation/tests/vashim.c"
        );
        unsafe { Library::new(&p) }.expect("failed to dlopen libvashim.so")
    });
    S.get_or_init(|| unsafe {
        VaShim {
            vpack_ex: *lib.get(b"shim_vpack_ex\0").unwrap(),
            vunpack_ex: *lib.get(b"shim_vunpack_ex\0").unwrap(),
            vsprintf: *lib.get(b"shim_vsprintf\0").unwrap(),
            error_vset: *lib.get(b"shim_error_vset\0").unwrap(),
        }
    })
}

/// The `json_vpack_ex` function pointer of a given library, as a `void *` to
/// hand to the shim.
pub fn sym_addr(which: &str, name: &[u8]) -> *mut c_void {
    let lib = if which == "C" { c_library() } else { rust_library() };
    let mut owned = name.to_vec();
    owned.push(0);
    unsafe { *lib.get::<*mut c_void>(&owned).unwrap() }
}

/// The C implementation's API (the ground truth).
pub fn capi() -> &'static Api {
    static A: OnceLock<Api> = OnceLock::new();
    A.get_or_init(|| Api::load(c_library(), "C"))
}

/// The Rust implementation's API (under test).
pub fn rapi() -> &'static Api {
    static A: OnceLock<Api> = OnceLock::new();
    A.get_or_init(|| Api::load(rust_library(), "Rust"))
}

/// Both APIs, with a deterministic hash seed already installed in each so that
/// hashtable bucket order — and therefore object iteration order — is
/// reproducible and directly comparable between the two libraries.
///
/// jansson seeds its hash function from `/dev/urandom`/time-of-day on first
/// use, so without this every run (and each library within a run) would order
/// object keys differently and no dump could be compared byte-for-byte.
pub fn both() -> (&'static Api, &'static Api) {
    static SEEDED: OnceLock<()> = OnceLock::new();
    SEEDED.get_or_init(|| unsafe {
        (capi().json_object_seed)(FIXED_SEED);
        (rapi().json_object_seed)(FIXED_SEED);
    });
    (capi(), rapi())
}

/// Fixed, non-zero seed. Non-zero matters: `json_object_seed(0)` asks jansson
/// to generate a *random* seed instead of using the value given.
pub const FIXED_SEED: size_t = 0x5eed_1234;

// ---------------------------------------------------------------------------
// Small helpers
// ---------------------------------------------------------------------------

pub fn cs(s: &str) -> CString {
    CString::new(s).expect("test string contains an interior NUL; use cs_bytes")
}

/// A NUL-terminated buffer that may contain interior NUL bytes.
pub fn cs_bytes(b: &[u8]) -> Vec<c_char> {
    let mut v: Vec<c_char> = b.iter().map(|&x| x as c_char).collect();
    v.push(0);
    v
}

pub fn cstr_lossy(p: *const c_char) -> String {
    if p.is_null() {
        return "<NULL>".to_string();
    }
    unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned()
}

/// Read a NUL-terminated C string as raw bytes (no UTF-8 assumption). `None`
/// for a NULL pointer, so "returned NULL" and "returned empty" stay distinct.
pub fn cbytes(p: *const c_char) -> Option<Vec<u8>> {
    if p.is_null() {
        None
    } else {
        Some(unsafe { CStr::from_ptr(p) }.to_bytes().to_vec())
    }
}

/// `json_incref` — a `static inline` in jansson.h, so it is not exported and
/// has to be reproduced here exactly.
pub unsafe fn incref(json: *mut json_t) -> *mut json_t {
    if !json.is_null() && (*json).refcount != usize::MAX {
        (*json).refcount += 1;
    }
    json
}

/// `json_decref` — likewise `static inline` in jansson.h. Calls the *matching*
/// library's `json_delete`, which is why it takes the api.
pub unsafe fn decref(api: &Api, json: *mut json_t) {
    if !json.is_null() && (*json).refcount != usize::MAX {
        (*json).refcount -= 1;
        if (*json).refcount == 0 {
            (api.json_delete)(json);
        }
    }
}

pub unsafe fn typeof_(json: *const json_t) -> c_int {
    (*json).type_
}

/// Free a `char *` that jansson allocated, using that library's allocator.
pub unsafe fn jfree(api: &Api, p: *mut c_void) {
    if !p.is_null() {
        (api.jsonp_free)(p);
    }
}

// ---------------------------------------------------------------------------
// Serialisation of process-global library state
// ---------------------------------------------------------------------------

/// Guard for library state that is PROCESS-GLOBAL and, in the C, not
/// thread-safe. Cargo runs the `#[test]` functions of one binary on several
/// threads, so any test touching such state must hold this lock for its whole
/// body.
///
/// Two distinct pieces of state need it:
///
/// 1. **dtoa's `Balloc`/`Bfree` freelist.** `dtoa.c` is compiled WITHOUT
///    `MULTIPLE_THREADS` defined (see the `#ifdef MULTIPLE_THREADS` block that
///    makes `MTa`/`MTb`/`MTd` expand to nothing), so `freelist[]` is a plain
///    global with no locking. Concurrent `dtoa`/`dtoa_r`/`freedtoa` calls
///    corrupt it and abort with `free(): invalid pointer`. This is a property
///    of the C library, not of the translation.
///
/// 2. **The allocator function pointers** (`do_malloc`/`do_realloc`/`do_free`
///    in memory.c). `json_set_alloc_funcs*` mutates globals, so a test that
///    installs a failing allocator would break any test allocating concurrently.
///
/// Call it as the first statement of such a test and bind the guard so it lives
/// for the whole function:
///
/// ```ignore
/// let _g = global_state_lock();
/// ```
pub fn global_state_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<std::sync::Mutex<()>> = OnceLock::new();
    // Recover from a poisoned lock: a panicking test has already reported its
    // own failure, and blocking every later test behind it would only hide
    // additional divergences.
    match LOCK.get_or_init(|| std::sync::Mutex::new(())).lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (no external rand dependency)
// ---------------------------------------------------------------------------

/// SplitMix64 — small, fast, and identical across runs for a given seed, so
/// every property-style test is reproducible.
pub struct Rng(pub u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    /// Uniform in `[0, n)`.
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }

    pub fn range(&mut self, lo: i64, hi: i64) -> i64 {
        if hi <= lo {
            return lo;
        }
        lo + (self.next_u64() % (hi - lo) as u64) as i64
    }

    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }

    pub fn choice<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len())]
    }

    /// A `json_int_t` drawn from a distribution that emphasises the boundary
    /// values the C code special-cases, not just "some number in the middle".
    pub fn json_int(&mut self) -> json_int_t {
        match self.below(10) {
            0 => 0,
            1 => 1,
            2 => -1,
            3 => i32::MAX as i64,
            4 => i32::MIN as i64,
            5 => i64::MAX,
            6 => i64::MIN,
            7 => self.range(-1000, 1000),
            8 => self.next_u32() as i64,
            _ => self.next_u64() as i64,
        }
    }

    /// A `double` biased towards the values dtoa/strtod treat specially.
    pub fn real(&mut self) -> f64 {
        match self.below(14) {
            0 => 0.0,
            1 => -0.0,
            2 => 1.0,
            3 => -1.0,
            4 => 0.1,
            5 => 1.0 / 3.0,
            6 => f64::MIN_POSITIVE,
            7 => f64::MAX,
            8 => 5e-324,               // smallest subnormal
            9 => 1e308,
            10 => 1e-308,
            11 => 2.2250738585072011e-308, // the classic strtod bug value
            12 => (self.next_u32() as f64) / 7.0,
            _ => {
                // A random finite bit pattern — the widest possible coverage.
                loop {
                    let b = self.next_u64();
                    let d = f64::from_bits(b);
                    if d.is_finite() {
                        return d;
                    }
                }
            }
        }
    }

    /// A random string biased towards bytes that the encoder/parser branch on.
    pub fn ascii_string(&mut self, maxlen: usize) -> String {
        const POOL: &[u8] = b"abcXYZ019 _-.:/\\\"'\t\n\r{}[],~`!@#$%^&*()+=|<>?;";
        let n = self.below(maxlen + 1);
        (0..n).map(|_| *self.choice(POOL) as char).collect()
    }

    /// A random *valid* UTF-8 string spanning 1-, 2-, 3- and 4-byte sequences
    /// plus the control characters the dumper has to escape.
    pub fn utf8_string(&mut self, maxlen: usize) -> String {
        let n = self.below(maxlen + 1);
        let mut s = String::new();
        for _ in 0..n {
            let c = match self.below(8) {
                0 => self.below(0x20) as u32,           // control chars
                1 | 2 => 0x20 + self.below(0x5f) as u32, // printable ASCII
                3 => 0x80 + self.below(0x780) as u32,   // 2-byte
                4 => 0x800 + self.below(0xf000) as u32, // 3-byte
                5 => 0x10000 + self.below(0xf_0000) as u32, // 4-byte
                6 => *self.choice(&[0x22u32, 0x5c, 0x2f, 0x08, 0x0c, 0x0a, 0x0d, 0x09]),
                _ => *self.choice(&[0x7f_u32, 0x80, 0x7ff, 0x800, 0xffff, 0x10000, 0x10ffff]),
            };
            if let Some(ch) = char::from_u32(c) {
                s.push(ch);
            }
        }
        s
    }
}

// ---------------------------------------------------------------------------
// Assertion helper
// ---------------------------------------------------------------------------

/// Assert two values match, printing the configuration that produced them.
#[macro_export]
macro_rules! diff_eq {
    ($c:expr, $r:expr, $($ctx:tt)*) => {{
        let cv = $c;
        let rv = $r;
        if cv != rv {
            panic!(
                "C/Rust divergence [{}]\n  C    = {:?}\n  Rust = {:?}",
                format!($($ctx)*), cv, rv
            );
        }
    }};
}
