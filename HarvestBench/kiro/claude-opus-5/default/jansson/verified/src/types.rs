//! Core types, constants and libc bindings shared by the whole crate.
//!
//! Mirrors `include/jansson.h`, `include/jansson_private.h`, `src/hashtable.h`
//! and `src/strbuffer.h`.

use core::ffi::{c_char, c_int, c_void};

/* ---------------------------------------------------------------- json_type */

pub const JSON_OBJECT: c_int = 0;
pub const JSON_ARRAY: c_int = 1;
pub const JSON_STRING: c_int = 2;
pub const JSON_INTEGER: c_int = 3;
pub const JSON_REAL: c_int = 4;
pub const JSON_TRUE: c_int = 5;
pub const JSON_FALSE: c_int = 6;
pub const JSON_NULL: c_int = 7;

/* ------------------------------------------------------------------ json_t */

#[repr(C)]
pub struct JsonT {
    pub type_: c_int,
    pub refcount: usize,
}

pub type JsonIntT = i64; /* JSON_INTEGER_IS_LONG_LONG == 1 */

/* ------------------------------------------------------------ error codes */

pub const JSON_ERROR_TEXT_LENGTH: usize = 160;
pub const JSON_ERROR_SOURCE_LENGTH: usize = 80;

#[repr(C)]
pub struct JsonErrorT {
    pub line: c_int,
    pub column: c_int,
    pub position: c_int,
    pub source: [c_char; JSON_ERROR_SOURCE_LENGTH],
    pub text: [c_char; JSON_ERROR_TEXT_LENGTH],
}

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

/* ------------------------------------------------------------------- flags */

pub const JSON_VALIDATE_ONLY: usize = 0x1;
pub const JSON_STRICT: usize = 0x2;

pub const JSON_REJECT_DUPLICATES: usize = 0x1;
pub const JSON_DISABLE_EOF_CHECK: usize = 0x2;
pub const JSON_DECODE_ANY: usize = 0x4;
pub const JSON_DECODE_INT_AS_REAL: usize = 0x8;
pub const JSON_ALLOW_NUL: usize = 0x10;

pub const JSON_COMPACT: usize = 0x20;
pub const JSON_ENSURE_ASCII: usize = 0x40;
pub const JSON_SORT_KEYS: usize = 0x80;
pub const JSON_ENCODE_ANY: usize = 0x200;
pub const JSON_ESCAPE_SLASH: usize = 0x400;
pub const JSON_EMBED: usize = 0x10000;

pub const JSON_PARSER_MAX_DEPTH: usize = 2048;

/* --------------------------------------------------------------- hashtable */

#[repr(C)]
pub struct HashtableList {
    pub prev: *mut HashtableList,
    pub next: *mut HashtableList,
}

#[repr(C)]
pub struct HashtablePair {
    pub list: HashtableList,
    pub ordered_list: HashtableList,
    pub hash: usize,
    pub value: *mut JsonT,
    pub key_len: usize,
    pub key: [c_char; 1],
}

#[repr(C)]
pub struct HashtableBucket {
    pub first: *mut HashtableList,
    pub last: *mut HashtableList,
}

#[repr(C)]
pub struct HashtableT {
    pub size: usize,
    pub buckets: *mut HashtableBucket,
    pub order: usize,
    pub list: HashtableList,
    pub ordered_list: HashtableList,
}

pub const PAIR_KEY_OFFSET: usize = core::mem::offset_of!(HashtablePair, key);
pub const PAIR_ORDERED_LIST_OFFSET: usize = core::mem::offset_of!(HashtablePair, ordered_list);

/// `container_of(list_, pair_t, list)`
#[inline]
pub unsafe fn list_to_pair(list: *mut HashtableList) -> *mut HashtablePair {
    (list as *mut u8).sub(core::mem::offset_of!(HashtablePair, list)) as *mut HashtablePair
}

/// `container_of(list_, pair_t, ordered_list)`
#[inline]
pub unsafe fn ordered_list_to_pair(list: *mut HashtableList) -> *mut HashtablePair {
    (list as *mut u8).sub(PAIR_ORDERED_LIST_OFFSET) as *mut HashtablePair
}

/// `container_of(key_, pair_t, key)`
#[inline]
pub unsafe fn key_to_pair(key: *const c_char) -> *mut HashtablePair {
    (key as *mut u8).sub(PAIR_KEY_OFFSET) as *mut HashtablePair
}

/* --------------------------------------------------------------- strbuffer */

#[repr(C)]
pub struct StrbufferT {
    pub value: *mut c_char,
    pub length: usize,
    pub size: usize,
}

/* ------------------------------------------------------- private json types */

#[repr(C)]
pub struct JsonObjectT {
    pub json: JsonT,
    pub hashtable: HashtableT,
}

#[repr(C)]
pub struct JsonArrayT {
    pub json: JsonT,
    pub size: usize,
    pub entries: usize,
    pub table: *mut *mut JsonT,
}

#[repr(C)]
pub struct JsonStringT {
    pub json: JsonT,
    pub value: *mut c_char,
    pub length: usize,
}

#[repr(C)]
pub struct JsonRealT {
    pub json: JsonT,
    pub value: f64,
}

#[repr(C)]
pub struct JsonIntegerT {
    pub json: JsonT,
    pub value: JsonIntT,
}

/// `LOOP_KEY_LEN`: space for "0x", double the sizeof a pointer, and a terminator.
pub const LOOP_KEY_LEN: usize = 2 + (core::mem::size_of::<*mut JsonT>() * 2) + 1;

/* ------------------------------------------------------ inline type helpers */

#[inline]
pub unsafe fn json_typeof(json: *const JsonT) -> c_int {
    (*json).type_
}

#[inline]
pub unsafe fn json_is_object(json: *const JsonT) -> bool {
    !json.is_null() && json_typeof(json) == JSON_OBJECT
}

#[inline]
pub unsafe fn json_is_array(json: *const JsonT) -> bool {
    !json.is_null() && json_typeof(json) == JSON_ARRAY
}

#[inline]
pub unsafe fn json_is_string(json: *const JsonT) -> bool {
    !json.is_null() && json_typeof(json) == JSON_STRING
}

#[inline]
pub unsafe fn json_is_integer(json: *const JsonT) -> bool {
    !json.is_null() && json_typeof(json) == JSON_INTEGER
}

#[inline]
pub unsafe fn json_is_real(json: *const JsonT) -> bool {
    !json.is_null() && json_typeof(json) == JSON_REAL
}

#[inline]
pub unsafe fn json_is_number(json: *const JsonT) -> bool {
    json_is_integer(json) || json_is_real(json)
}

#[inline]
pub unsafe fn json_is_true(json: *const JsonT) -> bool {
    !json.is_null() && json_typeof(json) == JSON_TRUE
}

#[inline]
pub unsafe fn json_is_false(json: *const JsonT) -> bool {
    !json.is_null() && json_typeof(json) == JSON_FALSE
}

#[inline]
pub unsafe fn json_is_boolean(json: *const JsonT) -> bool {
    json_is_true(json) || json_is_false(json)
}

#[inline]
pub unsafe fn json_is_null(json: *const JsonT) -> bool {
    !json.is_null() && json_typeof(json) == JSON_NULL
}

#[inline]
pub unsafe fn json_to_object(json: *const JsonT) -> *mut JsonObjectT {
    json as *mut JsonObjectT
}

#[inline]
pub unsafe fn json_to_array(json: *const JsonT) -> *mut JsonArrayT {
    json as *mut JsonArrayT
}

#[inline]
pub unsafe fn json_to_string(json: *const JsonT) -> *mut JsonStringT {
    json as *mut JsonStringT
}

#[inline]
pub unsafe fn json_to_real(json: *const JsonT) -> *mut JsonRealT {
    json as *mut JsonRealT
}

#[inline]
pub unsafe fn json_to_integer(json: *const JsonT) -> *mut JsonIntegerT {
    json as *mut JsonIntegerT
}

/// The `json_incref()` inline function from jansson.h. `JSON_HAVE_ATOMIC_BUILTINS`
/// and `JSON_HAVE_SYNC_BUILTINS` are not set in jansson_config.h, so this is a
/// plain non-atomic increment.
#[inline]
pub unsafe fn json_incref(json: *mut JsonT) -> *mut JsonT {
    if !json.is_null() && (*json).refcount != usize::MAX {
        (*json).refcount = (*json).refcount.wrapping_add(1);
    }
    json
}

/// The `json_decref()` inline function from jansson.h.
#[inline]
pub unsafe fn json_decref(json: *mut JsonT) {
    if !json.is_null() && (*json).refcount != usize::MAX {
        (*json).refcount = (*json).refcount.wrapping_sub(1);
        if (*json).refcount == 0 {
            crate::value::json_delete(json);
        }
    }
}

/* -------------------------------------------------------------------- libc */

pub const EOF: c_int = -1;

extern "C" {
    pub fn malloc(size: usize) -> *mut c_void;
    pub fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
    pub fn free(ptr: *mut c_void);
    pub fn memcpy(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    pub fn memmove(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    pub fn memcmp(a: *const c_void, b: *const c_void, n: usize) -> c_int;
    pub fn memchr(s: *const c_void, c: c_int, n: usize) -> *mut c_void;
    pub fn memset(s: *mut c_void, c: c_int, n: usize) -> *mut c_void;
    pub fn strlen(s: *const c_char) -> usize;
    pub fn strerror(errnum: c_int) -> *mut c_char;
    pub fn strtod(nptr: *const c_char, endptr: *mut *mut c_char) -> f64;
    pub fn strtoll(nptr: *const c_char, endptr: *mut *mut c_char, base: c_int) -> i64;
    pub fn __errno_location() -> *mut c_int;

    pub fn fopen(path: *const c_char, mode: *const c_char) -> *mut c_void;
    pub fn fclose(stream: *mut c_void) -> c_int;
    pub fn fwrite(ptr: *const c_void, size: usize, n: usize, stream: *mut c_void) -> usize;
    pub fn fgetc(stream: *mut c_void) -> c_int;
    pub fn read(fd: c_int, buf: *mut c_void, count: usize) -> isize;
    pub fn write(fd: c_int, buf: *const c_void, count: usize) -> isize;
    pub fn open(path: *const c_char, flags: c_int, ...) -> c_int;
    pub fn close(fd: c_int) -> c_int;
    pub fn getpid() -> c_int;
    pub fn sched_yield() -> c_int;
    pub fn gettimeofday(tv: *mut Timeval, tz: *mut c_void) -> c_int;

    pub fn snprintf(s: *mut c_char, n: usize, fmt: *const c_char, ...) -> c_int;
    pub fn vsnprintf(
        s: *mut c_char,
        n: usize,
        fmt: *const c_char,
        ap: *mut crate::varargs::VaListTag,
    ) -> c_int;

    /// `stdin` from <stdio.h>, used by `json_loadf()` for the "<stdin>" source name.
    pub static mut stdin: *mut c_void;
}

#[repr(C)]
pub struct Timeval {
    pub tv_sec: i64,
    pub tv_usec: i64,
}

#[inline]
pub unsafe fn errno() -> c_int {
    *__errno_location()
}

#[inline]
pub unsafe fn set_errno(v: c_int) {
    *__errno_location() = v;
}

pub const ERANGE: c_int = 34;
pub const STDIN_FILENO: c_int = 0;
