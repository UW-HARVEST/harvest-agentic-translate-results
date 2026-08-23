//! Core public types and libc bindings shared by the whole crate.
//!
//! Layouts mirror the C declarations in `jansson.h`, `jansson_private.h`,
//! `hashtable.h` and `strbuffer.h` exactly.

#![allow(non_camel_case_types)]

use core::ffi::{c_char, c_int, c_uint, c_void};

pub type json_int_t = i64; /* long long (JSON_INTEGER_IS_LONG_LONG == 1) */

/* ---------------------------------------------------------------- json_type */

pub const JSON_OBJECT: c_int = 0;
pub const JSON_ARRAY: c_int = 1;
pub const JSON_STRING: c_int = 2;
pub const JSON_INTEGER: c_int = 3;
pub const JSON_REAL: c_int = 4;
pub const JSON_TRUE: c_int = 5;
pub const JSON_FALSE: c_int = 6;
pub const JSON_NULL: c_int = 7;

/* -------------------------------------------------------- error code enum */

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
pub const JSON_PRESERVE_ORDER: usize = 0x100;
pub const JSON_ENCODE_ANY: usize = 0x200;
pub const JSON_ESCAPE_SLASH: usize = 0x400;
pub const JSON_EMBED: usize = 0x10000;

pub const JSON_ERROR_TEXT_LENGTH: usize = 160;
pub const JSON_ERROR_SOURCE_LENGTH: usize = 80;

pub const JSON_PARSER_MAX_DEPTH: usize = 2048;

/* `LOOP_KEY_LEN` from jansson_private.h: 2 + sizeof(json_t *) * 2 + 1 */
pub const LOOP_KEY_LEN: usize = 2 + (core::mem::size_of::<*mut json_t>() * 2) + 1;

/* ------------------------------------------------------------------ json_t */

#[repr(C)]
pub struct json_t {
    pub type_: c_int,
    pub refcount: usize, /* volatile size_t */
}

#[repr(C)]
pub struct json_error_t {
    pub line: c_int,
    pub column: c_int,
    pub position: c_int,
    pub source: [c_char; JSON_ERROR_SOURCE_LENGTH],
    pub text: [c_char; JSON_ERROR_TEXT_LENGTH],
}

/* ------------------------------------------------------------- hashtable_t */

#[repr(C)]
#[derive(Clone, Copy)]
pub struct hashtable_list {
    pub prev: *mut hashtable_list,
    pub next: *mut hashtable_list,
}

#[repr(C)]
pub struct hashtable_pair {
    pub list: hashtable_list,
    pub ordered_list: hashtable_list,
    pub hash: usize,
    pub value: *mut json_t,
    pub key_len: usize,
    pub key: [c_char; 1],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct hashtable_bucket {
    pub first: *mut hashtable_list,
    pub last: *mut hashtable_list,
}

#[repr(C)]
pub struct hashtable_t {
    pub size: usize,
    pub buckets: *mut hashtable_bucket,
    pub order: usize,
    pub list: hashtable_list,
    pub ordered_list: hashtable_list,
}

/* offsetof(struct hashtable_pair, key) */
pub const PAIR_KEY_OFFSET: usize = 56;

/* ------------------------------------------------------------ strbuffer_t */

#[repr(C)]
pub struct strbuffer_t {
    pub value: *mut c_char,
    pub length: usize,
    pub size: usize,
}

/* ------------------------------------------------- private value structs */

#[repr(C)]
pub struct json_object_t {
    pub json: json_t,
    pub hashtable: hashtable_t,
}

#[repr(C)]
pub struct json_array_t {
    pub json: json_t,
    pub size: usize,
    pub entries: usize,
    pub table: *mut *mut json_t,
}

#[repr(C)]
pub struct json_string_t {
    pub json: json_t,
    pub value: *mut c_char,
    pub length: usize,
}

#[repr(C)]
pub struct json_real_t {
    pub json: json_t,
    pub value: f64,
}

#[repr(C)]
pub struct json_integer_t {
    pub json: json_t,
    pub value: json_int_t,
}

/* ------------------------------------------------------- callback types */

pub type json_load_callback_t =
    Option<unsafe extern "C" fn(buffer: *mut c_void, buflen: usize, data: *mut c_void) -> usize>;
pub type json_dump_callback_t =
    Option<unsafe extern "C" fn(buffer: *const c_char, size: usize, data: *mut c_void) -> c_int>;

pub type json_malloc_t = Option<unsafe extern "C" fn(usize) -> *mut c_void>;
pub type json_realloc_t = Option<unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void>;
pub type json_free_t = Option<unsafe extern "C" fn(*mut c_void)>;

/* ---------------------------------------------------- inline C helpers */

#[inline]
pub unsafe fn json_typeof(json: *const json_t) -> c_int {
    (*json).type_
}

#[inline]
pub unsafe fn json_is_object(json: *const json_t) -> bool {
    !json.is_null() && json_typeof(json) == JSON_OBJECT
}

#[inline]
pub unsafe fn json_is_array(json: *const json_t) -> bool {
    !json.is_null() && json_typeof(json) == JSON_ARRAY
}

#[inline]
pub unsafe fn json_is_string(json: *const json_t) -> bool {
    !json.is_null() && json_typeof(json) == JSON_STRING
}

#[inline]
pub unsafe fn json_is_integer(json: *const json_t) -> bool {
    !json.is_null() && json_typeof(json) == JSON_INTEGER
}

#[inline]
pub unsafe fn json_is_real(json: *const json_t) -> bool {
    !json.is_null() && json_typeof(json) == JSON_REAL
}

#[inline]
pub unsafe fn json_is_number(json: *const json_t) -> bool {
    json_is_integer(json) || json_is_real(json)
}

#[inline]
pub unsafe fn json_is_true(json: *const json_t) -> bool {
    !json.is_null() && json_typeof(json) == JSON_TRUE
}

#[inline]
pub unsafe fn json_is_false(json: *const json_t) -> bool {
    !json.is_null() && json_typeof(json) == JSON_FALSE
}

#[inline]
pub unsafe fn json_is_boolean(json: *const json_t) -> bool {
    json_is_true(json) || json_is_false(json)
}

#[inline]
pub unsafe fn json_is_null(json: *const json_t) -> bool {
    !json.is_null() && json_typeof(json) == JSON_NULL
}

/// `json_incref` from jansson.h (non-atomic variant: `++json->refcount`).
#[inline]
pub unsafe fn json_incref(json: *mut json_t) -> *mut json_t {
    if !json.is_null() && (*json).refcount != usize::MAX {
        (*json).refcount = (*json).refcount.wrapping_add(1);
    }
    json
}

/// `json_decref` from jansson.h (non-atomic variant: `--json->refcount`).
#[inline]
pub unsafe fn json_decref(json: *mut json_t) {
    if !json.is_null() && (*json).refcount != usize::MAX {
        (*json).refcount = (*json).refcount.wrapping_sub(1);
        if (*json).refcount == 0 {
            crate::value::json_delete(json);
        }
    }
}

/* --------------------------------------------------------------- libc FFI */

pub type FILE = c_void;

extern "C" {
    pub fn malloc(size: usize) -> *mut c_void;
    pub fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
    pub fn free(ptr: *mut c_void);
    pub fn memcpy(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    pub fn memmove(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    pub fn memset(dst: *mut c_void, c: c_int, n: usize) -> *mut c_void;
    pub fn memcmp(a: *const c_void, b: *const c_void, n: usize) -> c_int;
    pub fn memchr(s: *const c_void, c: c_int, n: usize) -> *mut c_void;
    pub fn strlen(s: *const c_char) -> usize;
    pub fn strcmp(a: *const c_char, b: *const c_char) -> c_int;
    pub fn strchr(s: *const c_char, c: c_int) -> *mut c_char;
    pub fn strerror(errnum: c_int) -> *mut c_char;
    pub fn strtod(s: *const c_char, endptr: *mut *mut c_char) -> f64;
    pub fn strtoll(s: *const c_char, endptr: *mut *mut c_char, base: c_int) -> i64;

    pub fn snprintf(s: *mut c_char, n: usize, fmt: *const c_char, ...) -> c_int;
    pub fn sprintf(s: *mut c_char, fmt: *const c_char, ...) -> c_int;

    pub fn fopen(path: *const c_char, mode: *const c_char) -> *mut FILE;
    pub fn fclose(f: *mut FILE) -> c_int;
    pub fn fwrite(ptr: *const c_void, size: usize, n: usize, f: *mut FILE) -> usize;
    pub fn fgetc(f: *mut FILE) -> c_int;
    pub fn write(fd: c_int, buf: *const c_void, n: usize) -> isize;
    pub fn read(fd: c_int, buf: *mut c_void, n: usize) -> isize;
    pub fn open(path: *const c_char, flags: c_int, ...) -> c_int;
    pub fn close(fd: c_int) -> c_int;
    pub fn getpid() -> c_int;
    pub fn sched_yield() -> c_int;
    pub fn gettimeofday(tv: *mut timeval, tz: *mut c_void) -> c_int;
    pub fn qsort(
        base: *mut c_void,
        n: usize,
        size: usize,
        cmp: Option<unsafe extern "C" fn(*const c_void, *const c_void) -> c_int>,
    );

    #[link_name = "__errno_location"]
    pub fn errno_location() -> *mut c_int;

    pub static mut stdin: *mut FILE;
}

#[repr(C)]
pub struct timeval {
    pub tv_sec: i64,
    pub tv_usec: i64,
}

pub const EOF: c_int = -1;
pub const ERANGE: c_int = 34;
pub const O_RDONLY: c_int = 0;
pub const STDIN_FILENO: c_int = 0;

#[inline]
pub unsafe fn set_errno(v: c_int) {
    *errno_location() = v;
}

#[inline]
pub unsafe fn get_errno() -> c_int {
    *errno_location()
}

/* --------------------------------------------------------- va_list (SysV) */

/// x86-64 System V `__va_list_tag`. A C `va_list` decays to a pointer to this.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VaListTag {
    pub gp_offset: c_uint,
    pub fp_offset: c_uint,
    pub overflow_arg_area: *mut c_void,
    pub reg_save_area: *mut c_void,
}

pub type va_list = *mut VaListTag;

extern "C" {
    pub fn vsnprintf(s: *mut c_char, n: usize, fmt: *const c_char, ap: va_list) -> c_int;
}

impl VaListTag {
    /// `va_arg(ap, T)` for INTEGER-class arguments of at most 8 bytes.
    pub unsafe fn arg_gp<T: Copy>(&mut self) -> T {
        debug_assert!(core::mem::size_of::<T>() <= 8);
        if self.gp_offset < 48 {
            let p = (self.reg_save_area as *mut u8).add(self.gp_offset as usize);
            self.gp_offset += 8;
            core::ptr::read_unaligned(p as *const T)
        } else {
            let p = self.overflow_arg_area as *mut u8;
            self.overflow_arg_area = p.add(8) as *mut c_void;
            core::ptr::read_unaligned(p as *const T)
        }
    }

    /// `va_arg(ap, double)`.
    pub unsafe fn arg_double(&mut self) -> f64 {
        if self.fp_offset < 176 {
            let p = (self.reg_save_area as *mut u8).add(self.fp_offset as usize);
            self.fp_offset += 16;
            core::ptr::read_unaligned(p as *const f64)
        } else {
            let p = self.overflow_arg_area as *mut u8;
            self.overflow_arg_area = p.add(8) as *mut c_void;
            core::ptr::read_unaligned(p as *const f64)
        }
    }
}

/* ------------------------------------------------------------ misc helpers */

#[inline]
pub fn max_usize(a: usize, b: usize) -> usize {
    if a > b {
        a
    } else {
        b
    }
}

/// Length of the NUL-terminated prefix of `buf`.
pub fn cstr_len(buf: &[u8]) -> usize {
    match buf.iter().position(|&c| c == 0) {
        Some(i) => i,
        None => buf.len(),
    }
}

/// Emulates `snprintf(dst, dst.len(), "%s", src)`: copies at most
/// `dst.len() - 1` bytes and NUL-terminates. `src` may contain embedded NUL
/// bytes; they are copied verbatim, exactly as C's `%s` would *not* do — so
/// callers must pass already-NUL-trimmed data when that matters.
pub fn copy_trunc(dst: &mut [u8], src: &[u8]) -> usize {
    if dst.is_empty() {
        return src.len();
    }
    let n = core::cmp::min(src.len(), dst.len() - 1);
    dst[..n].copy_from_slice(&src[..n]);
    dst[n] = 0;
    src.len()
}
