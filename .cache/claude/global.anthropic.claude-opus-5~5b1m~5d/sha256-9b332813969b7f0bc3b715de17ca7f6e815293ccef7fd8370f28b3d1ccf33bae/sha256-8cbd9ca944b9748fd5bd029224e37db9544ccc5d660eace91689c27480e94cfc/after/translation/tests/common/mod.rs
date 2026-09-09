//! Differential-test harness: loads BOTH the C `libjansson.so` and the Rust
//! `libjansson.so` through `libloading` and exposes typed wrappers so the two
//! can be driven identically and their outputs compared byte-for-byte.
//!
//! Nothing here calls into the Rust crate directly — every call crosses the
//! FFI boundary exactly as an external consumer's would, so the
//! `#[no_mangle]`/`extern "C"` export wrappers are under test too.

#![allow(dead_code)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

use libloading::{Library, Symbol};
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_double, c_int, c_void};
use std::path::PathBuf;

pub type json_ptr = *mut c_void;
pub type json_int_t = i64;

pub const JSON_ERROR_TEXT_LENGTH: usize = 160;
pub const JSON_ERROR_SOURCE_LENGTH: usize = 80;

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
        json_error_t {
            line: -12345,
            column: -12345,
            position: -12345,
            source: [0x7f; JSON_ERROR_SOURCE_LENGTH],
            text: [0x7f; JSON_ERROR_TEXT_LENGTH],
        }
    }
}

/// Snapshot of a `json_error_t` in a form that is cheap to compare and print.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ErrSnap {
    pub line: c_int,
    pub column: c_int,
    pub position: c_int,
    pub source: Vec<u8>,
    pub text: Vec<u8>,
    /// `json_error_code()` == last byte of `text`, reinterpreted as an enum.
    pub code: u8,
    /// The raw 160 bytes of `text`, so trailing-byte differences are caught.
    pub text_raw: Vec<u8>,
}

impl json_error_t {
    pub fn snap(&self) -> ErrSnap {
        let text_bytes: Vec<u8> = self.text.iter().map(|&c| c as u8).collect();
        let source_bytes: Vec<u8> = self.source.iter().map(|&c| c as u8).collect();
        let cstr_of = |b: &[u8]| -> Vec<u8> {
            match b.iter().position(|&c| c == 0) {
                Some(i) => b[..i].to_vec(),
                None => b.to_vec(),
            }
        };
        ErrSnap {
            line: self.line,
            column: self.column,
            position: self.position,
            source: cstr_of(&source_bytes),
            text: cstr_of(&text_bytes),
            code: text_bytes[JSON_ERROR_TEXT_LENGTH - 1],
            text_raw: text_bytes,
        }
    }
}

// ---------------------------------------------------------------------------
// json_error_code enum values (from jansson.h)
// ---------------------------------------------------------------------------
pub const json_error_unknown: u8 = 0;
pub const json_error_out_of_memory: u8 = 1;
pub const json_error_stack_overflow: u8 = 2;
pub const json_error_cannot_open_file: u8 = 3;
pub const json_error_invalid_argument: u8 = 4;
pub const json_error_invalid_utf8: u8 = 5;
pub const json_error_premature_end_of_input: u8 = 6;
pub const json_error_end_of_input_expected: u8 = 7;
pub const json_error_invalid_syntax: u8 = 8;
pub const json_error_invalid_format: u8 = 9;
pub const json_error_wrong_type: u8 = 10;
pub const json_error_null_character: u8 = 11;
pub const json_error_null_value: u8 = 12;
pub const json_error_null_byte_in_key: u8 = 13;
pub const json_error_duplicate_key: u8 = 14;
pub const json_error_numeric_overflow: u8 = 15;
pub const json_error_item_not_found: u8 = 16;
pub const json_error_index_out_of_range: u8 = 17;

// ---------------------------------------------------------------------------
// flags (from jansson.h)
// ---------------------------------------------------------------------------
pub const JSON_REJECT_DUPLICATES: usize = 0x1;
pub const JSON_DISABLE_EOF_CHECK: usize = 0x2;
pub const JSON_DECODE_ANY: usize = 0x4;
pub const JSON_DECODE_INT_AS_REAL: usize = 0x8;
pub const JSON_ALLOW_NUL: usize = 0x10;

pub const JSON_MAX_INDENT: usize = 0x1F;
pub const JSON_COMPACT: usize = 0x20;
pub const JSON_ENSURE_ASCII: usize = 0x40;
pub const JSON_SORT_KEYS: usize = 0x80;
pub const JSON_PRESERVE_ORDER: usize = 0x100;
pub const JSON_ENCODE_ANY: usize = 0x200;
pub const JSON_ESCAPE_SLASH: usize = 0x400;
pub const JSON_EMBED: usize = 0x10000;

pub const JSON_VALIDATE_ONLY: usize = 0x1;
pub const JSON_STRICT: usize = 0x2;

pub fn JSON_INDENT(n: usize) -> usize {
    n & JSON_MAX_INDENT
}
pub fn JSON_REAL_PRECISION(n: usize) -> usize {
    (n & 0x1F) << 11
}

// json_type
pub const JSON_OBJECT: c_int = 0;
pub const JSON_ARRAY: c_int = 1;
pub const JSON_STRING: c_int = 2;
pub const JSON_INTEGER: c_int = 3;
pub const JSON_REAL: c_int = 4;
pub const JSON_TRUE: c_int = 5;
pub const JSON_FALSE: c_int = 6;
pub const JSON_NULL: c_int = 7;

pub const JSON_PARSER_MAX_DEPTH: usize = 2048;

// ---------------------------------------------------------------------------
// `json_t` layout, for reading `type` / `refcount` without calling anything.
// ---------------------------------------------------------------------------
#[repr(C)]
pub struct json_t_hdr {
    pub type_: c_int,
    pub refcount: usize,
}

pub unsafe fn typeof_json(p: json_ptr) -> c_int {
    assert!(!p.is_null(), "typeof_json on NULL");
    (*(p as *const json_t_hdr)).type_
}

pub unsafe fn refcount_json(p: json_ptr) -> usize {
    assert!(!p.is_null(), "refcount_json on NULL");
    (*(p as *const json_t_hdr)).refcount
}

// ---------------------------------------------------------------------------
// Library wrapper
// ---------------------------------------------------------------------------

pub struct Lib {
    pub lib: Library,
    pub name: &'static str,
}

macro_rules! sym {
    ($self:expr, $name:literal, $t:ty) => {{
        let s: Symbol<$t> = $self
            .lib
            .get(concat!($name, "\0").as_bytes())
            .unwrap_or_else(|e| panic!("{}: missing symbol {}: {}", $self.name, $name, e));
        *s
    }};
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p
}

pub fn c_so_path() -> PathBuf {
    workspace_root().join("c_src/build/libjansson.so")
}

pub fn rust_so_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/libjansson.so")
}

impl Lib {
    pub unsafe fn open(path: &PathBuf, name: &'static str) -> Lib {
        let lib = Library::new(path)
            .unwrap_or_else(|e| panic!("failed to load {} ({}): {}", name, path.display(), e));
        Lib { lib, name }
    }

    // ---- construction -------------------------------------------------
    pub unsafe fn json_object(&self) -> json_ptr {
        sym!(self, "json_object", unsafe extern "C" fn() -> json_ptr)()
    }
    pub unsafe fn json_array(&self) -> json_ptr {
        sym!(self, "json_array", unsafe extern "C" fn() -> json_ptr)()
    }
    pub unsafe fn json_string(&self, v: *const c_char) -> json_ptr {
        sym!(self, "json_string", unsafe extern "C" fn(*const c_char) -> json_ptr)(v)
    }
    pub unsafe fn json_stringn(&self, v: *const c_char, len: usize) -> json_ptr {
        sym!(
            self,
            "json_stringn",
            unsafe extern "C" fn(*const c_char, usize) -> json_ptr
        )(v, len)
    }
    pub unsafe fn json_string_nocheck(&self, v: *const c_char) -> json_ptr {
        sym!(
            self,
            "json_string_nocheck",
            unsafe extern "C" fn(*const c_char) -> json_ptr
        )(v)
    }
    pub unsafe fn json_stringn_nocheck(&self, v: *const c_char, len: usize) -> json_ptr {
        sym!(
            self,
            "json_stringn_nocheck",
            unsafe extern "C" fn(*const c_char, usize) -> json_ptr
        )(v, len)
    }
    pub unsafe fn json_integer(&self, v: json_int_t) -> json_ptr {
        sym!(self, "json_integer", unsafe extern "C" fn(json_int_t) -> json_ptr)(v)
    }
    pub unsafe fn json_real(&self, v: c_double) -> json_ptr {
        sym!(self, "json_real", unsafe extern "C" fn(c_double) -> json_ptr)(v)
    }
    pub unsafe fn json_true(&self) -> json_ptr {
        sym!(self, "json_true", unsafe extern "C" fn() -> json_ptr)()
    }
    pub unsafe fn json_false(&self) -> json_ptr {
        sym!(self, "json_false", unsafe extern "C" fn() -> json_ptr)()
    }
    pub unsafe fn json_null(&self) -> json_ptr {
        sym!(self, "json_null", unsafe extern "C" fn() -> json_ptr)()
    }
    pub unsafe fn json_delete(&self, j: json_ptr) {
        sym!(self, "json_delete", unsafe extern "C" fn(json_ptr))(j)
    }

    /// `json_decref` is a static inline in the header; replicate it exactly.
    pub unsafe fn json_decref(&self, j: json_ptr) {
        if j.is_null() {
            return;
        }
        let hdr = j as *mut json_t_hdr;
        if (*hdr).refcount == usize::MAX {
            return;
        }
        (*hdr).refcount -= 1;
        if (*hdr).refcount == 0 {
            self.json_delete(j);
        }
    }

    /// `json_incref` is a static inline in the header; replicate it exactly.
    pub unsafe fn json_incref(&self, j: json_ptr) -> json_ptr {
        if !j.is_null() {
            let hdr = j as *mut json_t_hdr;
            if (*hdr).refcount != usize::MAX {
                (*hdr).refcount += 1;
            }
        }
        j
    }

    // ---- object -------------------------------------------------------
    pub unsafe fn json_object_seed(&self, seed: usize) {
        sym!(self, "json_object_seed", unsafe extern "C" fn(usize))(seed)
    }
    pub unsafe fn json_object_size(&self, o: json_ptr) -> usize {
        sym!(self, "json_object_size", unsafe extern "C" fn(json_ptr) -> usize)(o)
    }
    pub unsafe fn json_object_get(&self, o: json_ptr, k: *const c_char) -> json_ptr {
        sym!(
            self,
            "json_object_get",
            unsafe extern "C" fn(json_ptr, *const c_char) -> json_ptr
        )(o, k)
    }
    pub unsafe fn json_object_getn(&self, o: json_ptr, k: *const c_char, kl: usize) -> json_ptr {
        sym!(
            self,
            "json_object_getn",
            unsafe extern "C" fn(json_ptr, *const c_char, usize) -> json_ptr
        )(o, k, kl)
    }
    pub unsafe fn json_object_set_new(&self, o: json_ptr, k: *const c_char, v: json_ptr) -> c_int {
        sym!(
            self,
            "json_object_set_new",
            unsafe extern "C" fn(json_ptr, *const c_char, json_ptr) -> c_int
        )(o, k, v)
    }
    pub unsafe fn json_object_setn_new(
        &self,
        o: json_ptr,
        k: *const c_char,
        kl: usize,
        v: json_ptr,
    ) -> c_int {
        sym!(
            self,
            "json_object_setn_new",
            unsafe extern "C" fn(json_ptr, *const c_char, usize, json_ptr) -> c_int
        )(o, k, kl, v)
    }
    pub unsafe fn json_object_set_new_nocheck(
        &self,
        o: json_ptr,
        k: *const c_char,
        v: json_ptr,
    ) -> c_int {
        sym!(
            self,
            "json_object_set_new_nocheck",
            unsafe extern "C" fn(json_ptr, *const c_char, json_ptr) -> c_int
        )(o, k, v)
    }
    pub unsafe fn json_object_setn_new_nocheck(
        &self,
        o: json_ptr,
        k: *const c_char,
        kl: usize,
        v: json_ptr,
    ) -> c_int {
        sym!(
            self,
            "json_object_setn_new_nocheck",
            unsafe extern "C" fn(json_ptr, *const c_char, usize, json_ptr) -> c_int
        )(o, k, kl, v)
    }
    pub unsafe fn json_object_del(&self, o: json_ptr, k: *const c_char) -> c_int {
        sym!(
            self,
            "json_object_del",
            unsafe extern "C" fn(json_ptr, *const c_char) -> c_int
        )(o, k)
    }
    pub unsafe fn json_object_deln(&self, o: json_ptr, k: *const c_char, kl: usize) -> c_int {
        sym!(
            self,
            "json_object_deln",
            unsafe extern "C" fn(json_ptr, *const c_char, usize) -> c_int
        )(o, k, kl)
    }
    pub unsafe fn json_object_clear(&self, o: json_ptr) -> c_int {
        sym!(self, "json_object_clear", unsafe extern "C" fn(json_ptr) -> c_int)(o)
    }
    pub unsafe fn json_object_update(&self, o: json_ptr, other: json_ptr) -> c_int {
        sym!(
            self,
            "json_object_update",
            unsafe extern "C" fn(json_ptr, json_ptr) -> c_int
        )(o, other)
    }
    pub unsafe fn json_object_update_existing(&self, o: json_ptr, other: json_ptr) -> c_int {
        sym!(
            self,
            "json_object_update_existing",
            unsafe extern "C" fn(json_ptr, json_ptr) -> c_int
        )(o, other)
    }
    pub unsafe fn json_object_update_missing(&self, o: json_ptr, other: json_ptr) -> c_int {
        sym!(
            self,
            "json_object_update_missing",
            unsafe extern "C" fn(json_ptr, json_ptr) -> c_int
        )(o, other)
    }
    pub unsafe fn json_object_update_recursive(&self, o: json_ptr, other: json_ptr) -> c_int {
        sym!(
            self,
            "json_object_update_recursive",
            unsafe extern "C" fn(json_ptr, json_ptr) -> c_int
        )(o, other)
    }
    pub unsafe fn json_object_iter(&self, o: json_ptr) -> *mut c_void {
        sym!(self, "json_object_iter", unsafe extern "C" fn(json_ptr) -> *mut c_void)(o)
    }
    pub unsafe fn json_object_iter_at(&self, o: json_ptr, k: *const c_char) -> *mut c_void {
        sym!(
            self,
            "json_object_iter_at",
            unsafe extern "C" fn(json_ptr, *const c_char) -> *mut c_void
        )(o, k)
    }
    pub unsafe fn json_object_key_to_iter(&self, k: *const c_char) -> *mut c_void {
        sym!(
            self,
            "json_object_key_to_iter",
            unsafe extern "C" fn(*const c_char) -> *mut c_void
        )(k)
    }
    pub unsafe fn json_object_iter_next(&self, o: json_ptr, it: *mut c_void) -> *mut c_void {
        sym!(
            self,
            "json_object_iter_next",
            unsafe extern "C" fn(json_ptr, *mut c_void) -> *mut c_void
        )(o, it)
    }
    pub unsafe fn json_object_iter_key(&self, it: *mut c_void) -> *const c_char {
        sym!(
            self,
            "json_object_iter_key",
            unsafe extern "C" fn(*mut c_void) -> *const c_char
        )(it)
    }
    pub unsafe fn json_object_iter_key_len(&self, it: *mut c_void) -> usize {
        sym!(
            self,
            "json_object_iter_key_len",
            unsafe extern "C" fn(*mut c_void) -> usize
        )(it)
    }
    pub unsafe fn json_object_iter_value(&self, it: *mut c_void) -> json_ptr {
        sym!(
            self,
            "json_object_iter_value",
            unsafe extern "C" fn(*mut c_void) -> json_ptr
        )(it)
    }
    pub unsafe fn json_object_iter_set_new(
        &self,
        o: json_ptr,
        it: *mut c_void,
        v: json_ptr,
    ) -> c_int {
        sym!(
            self,
            "json_object_iter_set_new",
            unsafe extern "C" fn(json_ptr, *mut c_void, json_ptr) -> c_int
        )(o, it, v)
    }

    // ---- array --------------------------------------------------------
    pub unsafe fn json_array_size(&self, a: json_ptr) -> usize {
        sym!(self, "json_array_size", unsafe extern "C" fn(json_ptr) -> usize)(a)
    }
    pub unsafe fn json_array_get(&self, a: json_ptr, i: usize) -> json_ptr {
        sym!(
            self,
            "json_array_get",
            unsafe extern "C" fn(json_ptr, usize) -> json_ptr
        )(a, i)
    }
    pub unsafe fn json_array_set_new(&self, a: json_ptr, i: usize, v: json_ptr) -> c_int {
        sym!(
            self,
            "json_array_set_new",
            unsafe extern "C" fn(json_ptr, usize, json_ptr) -> c_int
        )(a, i, v)
    }
    pub unsafe fn json_array_append_new(&self, a: json_ptr, v: json_ptr) -> c_int {
        sym!(
            self,
            "json_array_append_new",
            unsafe extern "C" fn(json_ptr, json_ptr) -> c_int
        )(a, v)
    }
    pub unsafe fn json_array_insert_new(&self, a: json_ptr, i: usize, v: json_ptr) -> c_int {
        sym!(
            self,
            "json_array_insert_new",
            unsafe extern "C" fn(json_ptr, usize, json_ptr) -> c_int
        )(a, i, v)
    }
    pub unsafe fn json_array_remove(&self, a: json_ptr, i: usize) -> c_int {
        sym!(
            self,
            "json_array_remove",
            unsafe extern "C" fn(json_ptr, usize) -> c_int
        )(a, i)
    }
    pub unsafe fn json_array_clear(&self, a: json_ptr) -> c_int {
        sym!(self, "json_array_clear", unsafe extern "C" fn(json_ptr) -> c_int)(a)
    }
    pub unsafe fn json_array_extend(&self, a: json_ptr, other: json_ptr) -> c_int {
        sym!(
            self,
            "json_array_extend",
            unsafe extern "C" fn(json_ptr, json_ptr) -> c_int
        )(a, other)
    }

    // ---- scalars ------------------------------------------------------
    pub unsafe fn json_string_value(&self, s: json_ptr) -> *const c_char {
        sym!(
            self,
            "json_string_value",
            unsafe extern "C" fn(json_ptr) -> *const c_char
        )(s)
    }
    pub unsafe fn json_string_length(&self, s: json_ptr) -> usize {
        sym!(self, "json_string_length", unsafe extern "C" fn(json_ptr) -> usize)(s)
    }
    pub unsafe fn json_integer_value(&self, s: json_ptr) -> json_int_t {
        sym!(
            self,
            "json_integer_value",
            unsafe extern "C" fn(json_ptr) -> json_int_t
        )(s)
    }
    pub unsafe fn json_real_value(&self, s: json_ptr) -> c_double {
        sym!(self, "json_real_value", unsafe extern "C" fn(json_ptr) -> c_double)(s)
    }
    pub unsafe fn json_number_value(&self, s: json_ptr) -> c_double {
        sym!(self, "json_number_value", unsafe extern "C" fn(json_ptr) -> c_double)(s)
    }
    pub unsafe fn json_string_set(&self, s: json_ptr, v: *const c_char) -> c_int {
        sym!(
            self,
            "json_string_set",
            unsafe extern "C" fn(json_ptr, *const c_char) -> c_int
        )(s, v)
    }
    pub unsafe fn json_string_setn(&self, s: json_ptr, v: *const c_char, l: usize) -> c_int {
        sym!(
            self,
            "json_string_setn",
            unsafe extern "C" fn(json_ptr, *const c_char, usize) -> c_int
        )(s, v, l)
    }
    pub unsafe fn json_string_set_nocheck(&self, s: json_ptr, v: *const c_char) -> c_int {
        sym!(
            self,
            "json_string_set_nocheck",
            unsafe extern "C" fn(json_ptr, *const c_char) -> c_int
        )(s, v)
    }
    pub unsafe fn json_string_setn_nocheck(
        &self,
        s: json_ptr,
        v: *const c_char,
        l: usize,
    ) -> c_int {
        sym!(
            self,
            "json_string_setn_nocheck",
            unsafe extern "C" fn(json_ptr, *const c_char, usize) -> c_int
        )(s, v, l)
    }
    pub unsafe fn json_integer_set(&self, s: json_ptr, v: json_int_t) -> c_int {
        sym!(
            self,
            "json_integer_set",
            unsafe extern "C" fn(json_ptr, json_int_t) -> c_int
        )(s, v)
    }
    pub unsafe fn json_real_set(&self, s: json_ptr, v: c_double) -> c_int {
        sym!(
            self,
            "json_real_set",
            unsafe extern "C" fn(json_ptr, c_double) -> c_int
        )(s, v)
    }

    // ---- equality / copy ---------------------------------------------
    pub unsafe fn json_equal(&self, a: json_ptr, b: json_ptr) -> c_int {
        sym!(self, "json_equal", unsafe extern "C" fn(json_ptr, json_ptr) -> c_int)(a, b)
    }
    pub unsafe fn json_copy(&self, v: json_ptr) -> json_ptr {
        sym!(self, "json_copy", unsafe extern "C" fn(json_ptr) -> json_ptr)(v)
    }
    pub unsafe fn json_deep_copy(&self, v: json_ptr) -> json_ptr {
        sym!(self, "json_deep_copy", unsafe extern "C" fn(json_ptr) -> json_ptr)(v)
    }

    // ---- load ---------------------------------------------------------
    pub unsafe fn json_loads(
        &self,
        input: *const c_char,
        flags: usize,
        err: *mut json_error_t,
    ) -> json_ptr {
        sym!(
            self,
            "json_loads",
            unsafe extern "C" fn(*const c_char, usize, *mut json_error_t) -> json_ptr
        )(input, flags, err)
    }
    pub unsafe fn json_loadb(
        &self,
        buf: *const c_char,
        buflen: usize,
        flags: usize,
        err: *mut json_error_t,
    ) -> json_ptr {
        sym!(
            self,
            "json_loadb",
            unsafe extern "C" fn(*const c_char, usize, usize, *mut json_error_t) -> json_ptr
        )(buf, buflen, flags, err)
    }
    pub unsafe fn json_loadf(
        &self,
        f: *mut c_void,
        flags: usize,
        err: *mut json_error_t,
    ) -> json_ptr {
        sym!(
            self,
            "json_loadf",
            unsafe extern "C" fn(*mut c_void, usize, *mut json_error_t) -> json_ptr
        )(f, flags, err)
    }
    pub unsafe fn json_loadfd(&self, fd: c_int, flags: usize, err: *mut json_error_t) -> json_ptr {
        sym!(
            self,
            "json_loadfd",
            unsafe extern "C" fn(c_int, usize, *mut json_error_t) -> json_ptr
        )(fd, flags, err)
    }
    pub unsafe fn json_load_file(
        &self,
        path: *const c_char,
        flags: usize,
        err: *mut json_error_t,
    ) -> json_ptr {
        sym!(
            self,
            "json_load_file",
            unsafe extern "C" fn(*const c_char, usize, *mut json_error_t) -> json_ptr
        )(path, flags, err)
    }
    pub unsafe fn json_load_callback(
        &self,
        cb: Option<unsafe extern "C" fn(*mut c_void, usize, *mut c_void) -> usize>,
        data: *mut c_void,
        flags: usize,
        err: *mut json_error_t,
    ) -> json_ptr {
        sym!(
            self,
            "json_load_callback",
            unsafe extern "C" fn(
                Option<unsafe extern "C" fn(*mut c_void, usize, *mut c_void) -> usize>,
                *mut c_void,
                usize,
                *mut json_error_t,
            ) -> json_ptr
        )(cb, data, flags, err)
    }

    // ---- dump ---------------------------------------------------------
    pub unsafe fn json_dumps_raw(&self, j: json_ptr, flags: usize) -> *mut c_char {
        sym!(
            self,
            "json_dumps",
            unsafe extern "C" fn(json_ptr, usize) -> *mut c_char
        )(j, flags)
    }
    pub unsafe fn json_dumpb(
        &self,
        j: json_ptr,
        buf: *mut c_char,
        size: usize,
        flags: usize,
    ) -> usize {
        sym!(
            self,
            "json_dumpb",
            unsafe extern "C" fn(json_ptr, *mut c_char, usize, usize) -> usize
        )(j, buf, size, flags)
    }
    pub unsafe fn json_dumpf(&self, j: json_ptr, f: *mut c_void, flags: usize) -> c_int {
        sym!(
            self,
            "json_dumpf",
            unsafe extern "C" fn(json_ptr, *mut c_void, usize) -> c_int
        )(j, f, flags)
    }
    pub unsafe fn json_dumpfd(&self, j: json_ptr, fd: c_int, flags: usize) -> c_int {
        sym!(
            self,
            "json_dumpfd",
            unsafe extern "C" fn(json_ptr, c_int, usize) -> c_int
        )(j, fd, flags)
    }
    pub unsafe fn json_dump_file(&self, j: json_ptr, path: *const c_char, flags: usize) -> c_int {
        sym!(
            self,
            "json_dump_file",
            unsafe extern "C" fn(json_ptr, *const c_char, usize) -> c_int
        )(j, path, flags)
    }
    pub unsafe fn json_dump_callback(
        &self,
        j: json_ptr,
        cb: Option<unsafe extern "C" fn(*const c_char, usize, *mut c_void) -> c_int>,
        data: *mut c_void,
        flags: usize,
    ) -> c_int {
        sym!(
            self,
            "json_dump_callback",
            unsafe extern "C" fn(
                json_ptr,
                Option<unsafe extern "C" fn(*const c_char, usize, *mut c_void) -> c_int>,
                *mut c_void,
                usize,
            ) -> c_int
        )(j, cb, data, flags)
    }

    /// `json_dumps` returning an owned `Vec<u8>`, freeing via the *same*
    /// library's allocator (`jsonp_free`), which is what a C caller's `free()`
    /// resolves to when the default allocators are in effect.
    pub unsafe fn dumps(&self, j: json_ptr, flags: usize) -> Option<Vec<u8>> {
        let p = self.json_dumps_raw(j, flags);
        if p.is_null() {
            return None;
        }
        let out = CStr::from_ptr(p).to_bytes().to_vec();
        self.jsonp_free(p as *mut c_void);
        Some(out)
    }

    // ---- pack / unpack (variadic) -------------------------------------
    pub unsafe fn json_pack_ex_sym(
        &self,
    ) -> unsafe extern "C" fn(*mut json_error_t, usize, *const c_char, ...) -> json_ptr {
        sym!(
            self,
            "json_pack_ex",
            unsafe extern "C" fn(*mut json_error_t, usize, *const c_char, ...) -> json_ptr
        )
    }
    pub unsafe fn json_pack_sym(
        &self,
    ) -> unsafe extern "C" fn(*const c_char, ...) -> json_ptr {
        sym!(self, "json_pack", unsafe extern "C" fn(*const c_char, ...) -> json_ptr)
    }
    pub unsafe fn json_unpack_sym(&self) -> unsafe extern "C" fn(json_ptr, *const c_char, ...) -> c_int {
        sym!(
            self,
            "json_unpack",
            unsafe extern "C" fn(json_ptr, *const c_char, ...) -> c_int
        )
    }
    pub unsafe fn json_unpack_ex_sym(
        &self,
    ) -> unsafe extern "C" fn(json_ptr, *mut json_error_t, usize, *const c_char, ...) -> c_int {
        sym!(
            self,
            "json_unpack_ex",
            unsafe extern "C" fn(json_ptr, *mut json_error_t, usize, *const c_char, ...) -> c_int
        )
    }
    pub unsafe fn json_sprintf_sym(&self) -> unsafe extern "C" fn(*const c_char, ...) -> json_ptr {
        sym!(self, "json_sprintf", unsafe extern "C" fn(*const c_char, ...) -> json_ptr)
    }

    // ---- allocators ---------------------------------------------------
    pub unsafe fn jsonp_malloc(&self, n: usize) -> *mut c_void {
        sym!(self, "jsonp_malloc", unsafe extern "C" fn(usize) -> *mut c_void)(n)
    }
    pub unsafe fn jsonp_realloc(&self, p: *mut c_void, o: usize, n: usize) -> *mut c_void {
        sym!(
            self,
            "jsonp_realloc",
            unsafe extern "C" fn(*mut c_void, usize, usize) -> *mut c_void
        )(p, o, n)
    }
    pub unsafe fn jsonp_free(&self, p: *mut c_void) {
        sym!(self, "jsonp_free", unsafe extern "C" fn(*mut c_void))(p)
    }
    pub unsafe fn jsonp_strndup(&self, s: *const c_char, n: usize) -> *mut c_char {
        sym!(
            self,
            "jsonp_strndup",
            unsafe extern "C" fn(*const c_char, usize) -> *mut c_char
        )(s, n)
    }
    pub unsafe fn jsonp_stringn_nocheck_own(&self, s: *mut c_char, n: usize) -> json_ptr {
        sym!(
            self,
            "jsonp_stringn_nocheck_own",
            unsafe extern "C" fn(*mut c_char, usize) -> json_ptr
        )(s, n)
    }
    pub unsafe fn json_set_alloc_funcs(
        &self,
        m: Option<unsafe extern "C" fn(usize) -> *mut c_void>,
        f: Option<unsafe extern "C" fn(*mut c_void)>,
    ) {
        sym!(
            self,
            "json_set_alloc_funcs",
            unsafe extern "C" fn(
                Option<unsafe extern "C" fn(usize) -> *mut c_void>,
                Option<unsafe extern "C" fn(*mut c_void)>,
            )
        )(m, f)
    }
    pub unsafe fn json_get_alloc_funcs(
        &self,
        m: *mut Option<unsafe extern "C" fn(usize) -> *mut c_void>,
        f: *mut Option<unsafe extern "C" fn(*mut c_void)>,
    ) {
        sym!(
            self,
            "json_get_alloc_funcs",
            unsafe extern "C" fn(
                *mut Option<unsafe extern "C" fn(usize) -> *mut c_void>,
                *mut Option<unsafe extern "C" fn(*mut c_void)>,
            )
        )(m, f)
    }
    pub unsafe fn json_set_alloc_funcs2(
        &self,
        m: Option<unsafe extern "C" fn(usize) -> *mut c_void>,
        r: Option<unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void>,
        f: Option<unsafe extern "C" fn(*mut c_void)>,
    ) {
        sym!(
            self,
            "json_set_alloc_funcs2",
            unsafe extern "C" fn(
                Option<unsafe extern "C" fn(usize) -> *mut c_void>,
                Option<unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void>,
                Option<unsafe extern "C" fn(*mut c_void)>,
            )
        )(m, r, f)
    }
    pub unsafe fn json_get_alloc_funcs2(
        &self,
        m: *mut usize,
        r: *mut usize,
        f: *mut usize,
    ) {
        sym!(
            self,
            "json_get_alloc_funcs2",
            unsafe extern "C" fn(*mut usize, *mut usize, *mut usize)
        )(m, r, f)
    }

    // ---- version ------------------------------------------------------
    pub unsafe fn jansson_version_str(&self) -> *const c_char {
        sym!(self, "jansson_version_str", unsafe extern "C" fn() -> *const c_char)()
    }
    pub unsafe fn jansson_version_cmp(&self, a: c_int, b: c_int, c: c_int) -> c_int {
        sym!(
            self,
            "jansson_version_cmp",
            unsafe extern "C" fn(c_int, c_int, c_int) -> c_int
        )(a, b, c)
    }

    // ---- utf8 ---------------------------------------------------------
    pub unsafe fn utf8_encode(&self, cp: c_int, buf: *mut c_char, len: *mut usize) -> c_int {
        sym!(
            self,
            "utf8_encode",
            unsafe extern "C" fn(c_int, *mut c_char, *mut usize) -> c_int
        )(cp, buf, len)
    }
    pub unsafe fn utf8_check_first(&self, b: c_char) -> usize {
        sym!(self, "utf8_check_first", unsafe extern "C" fn(c_char) -> usize)(b)
    }
    pub unsafe fn utf8_check_full(&self, b: *const c_char, len: usize, cp: *mut i32) -> c_int {
        sym!(
            self,
            "utf8_check_full",
            unsafe extern "C" fn(*const c_char, usize, *mut i32) -> c_int
        )(b, len, cp)
    }
    pub unsafe fn utf8_iterate(
        &self,
        b: *const c_char,
        len: usize,
        cp: *mut i32,
    ) -> *const c_char {
        sym!(
            self,
            "utf8_iterate",
            unsafe extern "C" fn(*const c_char, usize, *mut i32) -> *const c_char
        )(b, len, cp)
    }
    pub unsafe fn utf8_check_string(&self, s: *const c_char, len: usize) -> c_int {
        sym!(
            self,
            "utf8_check_string",
            unsafe extern "C" fn(*const c_char, usize) -> c_int
        )(s, len)
    }

    // ---- strconv / dtoa ----------------------------------------------
    pub unsafe fn jsonp_strtod(&self, sb: *mut c_void, out: *mut c_double) -> c_int {
        sym!(
            self,
            "jsonp_strtod",
            unsafe extern "C" fn(*mut c_void, *mut c_double) -> c_int
        )(sb, out)
    }
    pub unsafe fn jsonp_dtostr(&self, buf: *mut c_char, size: usize, v: c_double, p: c_int) -> c_int {
        sym!(
            self,
            "jsonp_dtostr",
            unsafe extern "C" fn(*mut c_char, usize, c_double, c_int) -> c_int
        )(buf, size, v, p)
    }
    pub unsafe fn dtoa_sym(
        &self,
    ) -> unsafe extern "C" fn(c_double, c_int, c_int, *mut c_int, *mut c_int, *mut *mut c_char)
        -> *mut c_char {
        sym!(
            self,
            "dtoa",
            unsafe extern "C" fn(
                c_double,
                c_int,
                c_int,
                *mut c_int,
                *mut c_int,
                *mut *mut c_char,
            ) -> *mut c_char
        )
    }
    pub unsafe fn freedtoa(&self, p: *mut c_char) {
        sym!(self, "freedtoa", unsafe extern "C" fn(*mut c_char))(p)
    }

    // ---- strbuffer ----------------------------------------------------
    pub unsafe fn strbuffer_init(&self, sb: *mut c_void) -> c_int {
        sym!(self, "strbuffer_init", unsafe extern "C" fn(*mut c_void) -> c_int)(sb)
    }
    pub unsafe fn strbuffer_close(&self, sb: *mut c_void) {
        sym!(self, "strbuffer_close", unsafe extern "C" fn(*mut c_void))(sb)
    }
    pub unsafe fn strbuffer_clear(&self, sb: *mut c_void) {
        sym!(self, "strbuffer_clear", unsafe extern "C" fn(*mut c_void))(sb)
    }
    pub unsafe fn strbuffer_value(&self, sb: *const c_void) -> *const c_char {
        sym!(
            self,
            "strbuffer_value",
            unsafe extern "C" fn(*const c_void) -> *const c_char
        )(sb)
    }
    pub unsafe fn strbuffer_steal_value(&self, sb: *mut c_void) -> *mut c_char {
        sym!(
            self,
            "strbuffer_steal_value",
            unsafe extern "C" fn(*mut c_void) -> *mut c_char
        )(sb)
    }
    pub unsafe fn strbuffer_append_byte(&self, sb: *mut c_void, b: c_char) -> c_int {
        sym!(
            self,
            "strbuffer_append_byte",
            unsafe extern "C" fn(*mut c_void, c_char) -> c_int
        )(sb, b)
    }
    pub unsafe fn strbuffer_append_bytes(
        &self,
        sb: *mut c_void,
        data: *const c_char,
        size: usize,
    ) -> c_int {
        sym!(
            self,
            "strbuffer_append_bytes",
            unsafe extern "C" fn(*mut c_void, *const c_char, usize) -> c_int
        )(sb, data, size)
    }
    pub unsafe fn strbuffer_pop(&self, sb: *mut c_void) -> c_char {
        sym!(self, "strbuffer_pop", unsafe extern "C" fn(*mut c_void) -> c_char)(sb)
    }

    // ---- hashtable ----------------------------------------------------
    // NOTE: `hashtable_seed` is an exported *global* (`volatile uint32_t`),
    // NOT a function — read it with `lib.get::<*mut u32>(b"hashtable_seed\0")`.
    pub unsafe fn hashtable_init(&self, ht: *mut c_void) -> c_int {
        sym!(self, "hashtable_init", unsafe extern "C" fn(*mut c_void) -> c_int)(ht)
    }
    pub unsafe fn hashtable_close(&self, ht: *mut c_void) {
        sym!(self, "hashtable_close", unsafe extern "C" fn(*mut c_void))(ht)
    }
    pub unsafe fn hashtable_set(
        &self,
        ht: *mut c_void,
        key: *const c_char,
        key_len: usize,
        value: json_ptr,
    ) -> c_int {
        sym!(
            self,
            "hashtable_set",
            unsafe extern "C" fn(*mut c_void, *const c_char, usize, json_ptr) -> c_int
        )(ht, key, key_len, value)
    }
    pub unsafe fn hashtable_get(
        &self,
        ht: *mut c_void,
        key: *const c_char,
        key_len: usize,
    ) -> json_ptr {
        sym!(
            self,
            "hashtable_get",
            unsafe extern "C" fn(*mut c_void, *const c_char, usize) -> json_ptr
        )(ht, key, key_len)
    }
    pub unsafe fn hashtable_del(
        &self,
        ht: *mut c_void,
        key: *const c_char,
        key_len: usize,
    ) -> c_int {
        sym!(
            self,
            "hashtable_del",
            unsafe extern "C" fn(*mut c_void, *const c_char, usize) -> c_int
        )(ht, key, key_len)
    }
    pub unsafe fn hashtable_clear(&self, ht: *mut c_void) {
        sym!(self, "hashtable_clear", unsafe extern "C" fn(*mut c_void))(ht)
    }
    pub unsafe fn hashtable_iter(&self, ht: *mut c_void) -> *mut c_void {
        sym!(self, "hashtable_iter", unsafe extern "C" fn(*mut c_void) -> *mut c_void)(ht)
    }
    pub unsafe fn hashtable_iter_at(
        &self,
        ht: *mut c_void,
        key: *const c_char,
        key_len: usize,
    ) -> *mut c_void {
        sym!(
            self,
            "hashtable_iter_at",
            unsafe extern "C" fn(*mut c_void, *const c_char, usize) -> *mut c_void
        )(ht, key, key_len)
    }
    pub unsafe fn hashtable_iter_next(&self, ht: *mut c_void, it: *mut c_void) -> *mut c_void {
        sym!(
            self,
            "hashtable_iter_next",
            unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void
        )(ht, it)
    }
    pub unsafe fn hashtable_iter_key(&self, it: *mut c_void) -> *const c_char {
        sym!(
            self,
            "hashtable_iter_key",
            unsafe extern "C" fn(*mut c_void) -> *const c_char
        )(it)
    }
    pub unsafe fn hashtable_iter_key_len(&self, it: *mut c_void) -> usize {
        sym!(
            self,
            "hashtable_iter_key_len",
            unsafe extern "C" fn(*mut c_void) -> usize
        )(it)
    }
    pub unsafe fn hashtable_iter_value(&self, it: *mut c_void) -> json_ptr {
        sym!(
            self,
            "hashtable_iter_value",
            unsafe extern "C" fn(*mut c_void) -> json_ptr
        )(it)
    }
    pub unsafe fn hashtable_iter_set(&self, it: *mut c_void, v: json_ptr) {
        sym!(
            self,
            "hashtable_iter_set",
            unsafe extern "C" fn(*mut c_void, json_ptr)
        )(it, v)
    }

    // ---- error --------------------------------------------------------
    pub unsafe fn jsonp_error_init(&self, e: *mut json_error_t, source: *const c_char) {
        sym!(
            self,
            "jsonp_error_init",
            unsafe extern "C" fn(*mut json_error_t, *const c_char)
        )(e, source)
    }
    pub unsafe fn jsonp_error_set_source(&self, e: *mut json_error_t, source: *const c_char) {
        sym!(
            self,
            "jsonp_error_set_source",
            unsafe extern "C" fn(*mut json_error_t, *const c_char)
        )(e, source)
    }
    pub unsafe fn jsonp_error_set_sym(
        &self,
    ) -> unsafe extern "C" fn(*mut json_error_t, c_int, c_int, usize, c_int, *const c_char, ...) {
        sym!(
            self,
            "jsonp_error_set",
            unsafe extern "C" fn(
                *mut json_error_t,
                c_int,
                c_int,
                usize,
                c_int,
                *const c_char,
                ...
            )
        )
    }
    pub unsafe fn jsonp_loop_check(
        &self,
        ht: *mut c_void,
        j: json_ptr,
        key: *mut c_char,
        key_size: usize,
        key_len: *mut usize,
    ) -> c_int {
        sym!(
            self,
            "jsonp_loop_check",
            unsafe extern "C" fn(*mut c_void, json_ptr, *mut c_char, usize, *mut usize) -> c_int
        )(ht, j, key, key_size, key_len)
    }
}

// ---------------------------------------------------------------------------
// Test-global library pair
// ---------------------------------------------------------------------------

/// The fixed hash seed both libraries are pinned to, so that hashtable
/// bucket layout — and hence any order-sensitive behaviour — is identical.
pub const FIXED_SEED: usize = 0x5EED_1234;

pub struct Pair {
    pub c: Lib,
    pub r: Lib,
}

impl Pair {
    pub fn new() -> Pair {
        unsafe {
            let c = Lib::open(&c_so_path(), "C");
            let r = Lib::open(&rust_so_path(), "RUST");
            c.json_object_seed(FIXED_SEED);
            r.json_object_seed(FIXED_SEED);
            Pair { c, r }
        }
    }
}

/// One process-wide pair; loading each `.so` twice would reset global state.
pub fn pair() -> &'static Pair {
    use std::sync::OnceLock;
    static P: OnceLock<Pair> = OnceLock::new();
    P.get_or_init(Pair::new)
}

// `Library` is Send+Sync-ish for our purposes; tests are single-threaded per
// object graph and we never share `json_t*` across libraries.
unsafe impl Send for Pair {}
unsafe impl Sync for Pair {}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

pub fn cs(s: &str) -> CString {
    CString::new(s).unwrap()
}

/// A NUL-terminated byte buffer that may itself contain interior NULs.
pub fn raw_z(bytes: &[u8]) -> Vec<u8> {
    let mut v = bytes.to_vec();
    v.push(0);
    v
}

pub unsafe fn cstr_bytes(p: *const c_char) -> Option<Vec<u8>> {
    if p.is_null() {
        None
    } else {
        Some(CStr::from_ptr(p).to_bytes().to_vec())
    }
}

pub fn show(b: &Option<Vec<u8>>) -> String {
    match b {
        None => "<NULL>".to_string(),
        Some(v) => String::from_utf8_lossy(v).into_owned(),
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) so every property test is reproducible.
// ---------------------------------------------------------------------------
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
        if hi <= lo {
            return lo;
        }
        lo + (self.next_u64() % ((hi - lo) as u64)) as i64
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
    /// A double drawn from a distribution that covers normals, subnormals,
    /// huge/tiny magnitudes and exact integers — but never NaN/Inf, which
    /// jansson rejects.
    pub fn finite_f64(&mut self) -> f64 {
        loop {
            let v = match self.below(6) {
                0 => (self.range(-1000, 1000)) as f64,
                1 => (self.range(-1_000_000, 1_000_000) as f64) / 1000.0,
                2 => f64::from_bits(self.next_u64()),
                3 => (self.range(-300, 300) as f64).exp(),
                4 => (self.next_u64() as f64) / (u64::MAX as f64),
                _ => {
                    let m = self.range(-(1 << 52), 1 << 52) as f64;
                    let e = self.range(-40, 40) as i32;
                    m * 2f64.powi(e)
                }
            };
            if v.is_finite() {
                return v;
            }
        }
    }
    /// A random ASCII-safe string.
    pub fn ascii(&mut self, maxlen: usize) -> String {
        let n = self.below(maxlen + 1);
        (0..n)
            .map(|_| {
                let c = b" !\"#$%&'()*+,-./0123456789:;<=>?@ABCXYZ[\\]^_`abcxyz{|}~\t\n\r";
                c[self.below(c.len())] as char
            })
            .collect()
    }
    /// A random valid UTF-8 string spanning 1/2/3/4-byte sequences.
    pub fn utf8(&mut self, maxchars: usize) -> String {
        let n = self.below(maxchars + 1);
        let mut s = String::new();
        for _ in 0..n {
            let cp: u32 = match self.below(5) {
                0 => self.below(0x80) as u32,
                1 => (0x80 + self.below(0x780)) as u32,
                2 => (0x800 + self.below(0xF800)) as u32,
                3 => (0x1_0000 + self.below(0x10_0000)) as u32,
                _ => {
                    // deliberately include control chars & quote-y bytes
                    let c = b"\x00\x01\x07\x08\x0b\x0c\x1f\"\\/";
                    c[self.below(9)] as u32
                }
            };
            match char::from_u32(cp) {
                Some(c) if c != '\0' => s.push(c),
                _ => s.push('x'),
            }
        }
        s
    }
}

// ---------------------------------------------------------------------------
// ABI mirrors of the private structs the exported internal API takes by
// pointer. Sizes/layouts come from c_src/src/strbuffer.h and
// c_src/src/hashtable.h; the tests assert the Rust side agrees.
// ---------------------------------------------------------------------------

/// `strbuffer_t` — `{ char *value; size_t length; size_t size; }`
#[repr(C)]
#[derive(Clone, Copy)]
pub struct strbuffer_t {
    pub value: *mut c_char,
    pub length: usize,
    pub size: usize,
}

impl strbuffer_t {
    pub fn zeroed() -> strbuffer_t {
        strbuffer_t {
            value: std::ptr::null_mut(),
            length: 0,
            size: 0,
        }
    }
    /// The `length` bytes currently held (independent of NUL placement).
    pub unsafe fn bytes(&self) -> Vec<u8> {
        if self.value.is_null() {
            Vec::new()
        } else {
            std::slice::from_raw_parts(self.value as *const u8, self.length).to_vec()
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct hashtable_list {
    pub prev: *mut c_void,
    pub next: *mut c_void,
}

/// `hashtable_t` — `{ size_t size; bucket *buckets; size_t order;
///                    hashtable_list list; hashtable_list ordered_list; }`
#[repr(C)]
#[derive(Clone, Copy)]
pub struct hashtable_t {
    pub size: usize,
    pub buckets: *mut c_void,
    pub order: usize,
    pub list: hashtable_list,
    pub ordered_list: hashtable_list,
}

impl hashtable_t {
    pub fn zeroed() -> hashtable_t {
        let nil = hashtable_list {
            prev: std::ptr::null_mut(),
            next: std::ptr::null_mut(),
        };
        hashtable_t {
            size: 0,
            buckets: std::ptr::null_mut(),
            order: 0,
            list: nil,
            ordered_list: nil,
        }
    }
}

/// Walk a whole object with the public iterator API and return
/// `(key_bytes, value_dump)` for every entry, in iteration order.
pub unsafe fn object_entries(l: &Lib, obj: json_ptr, flags: usize) -> Vec<(Vec<u8>, Option<Vec<u8>>)> {
    let mut out = Vec::new();
    let mut it = l.json_object_iter(obj);
    while !it.is_null() {
        let kp = l.json_object_iter_key(it);
        let klen = l.json_object_iter_key_len(it);
        let key = if kp.is_null() {
            Vec::new()
        } else {
            std::slice::from_raw_parts(kp as *const u8, klen).to_vec()
        };
        let v = l.json_object_iter_value(it);
        let d = if v.is_null() {
            None
        } else {
            l.dumps(v, flags | JSON_ENCODE_ANY)
        };
        out.push((key, d));
        it = l.json_object_iter_next(obj, it);
    }
    out
}

/// Walk a whole hashtable with the exported `hashtable_iter*` API.
pub unsafe fn hashtable_entries(l: &Lib, ht: *mut c_void) -> Vec<(Vec<u8>, usize)> {
    let mut out = Vec::new();
    let mut it = l.hashtable_iter(ht);
    while !it.is_null() {
        let kp = l.hashtable_iter_key(it);
        let klen = l.hashtable_iter_key_len(it);
        let key = if kp.is_null() {
            Vec::new()
        } else {
            std::slice::from_raw_parts(kp as *const u8, klen).to_vec()
        };
        // The stored value is a `json_t*` from the *same* library; compare its
        // type tag rather than the (necessarily different) pointer value.
        let v = l.hashtable_iter_value(it);
        let tag = if v.is_null() { usize::MAX } else { typeof_json(v) as usize };
        out.push((key, tag));
        it = l.hashtable_iter_next(ht, it);
    }
    out
}

// ---------------------------------------------------------------------------
// Library-independent description of a JSON tree, so the *same* tree can be
// built in both libraries and the results compared.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Null,
    True,
    False,
    Int(json_int_t),
    Real(f64),
    /// Raw bytes, built with `json_stringn` (UTF-8 checked).
    Str(Vec<u8>),
    /// Raw bytes, built with `json_stringn_nocheck` — may contain interior
    /// NULs or invalid UTF-8, which no checked constructor would accept.
    StrNoCheck(Vec<u8>),
    Arr(Vec<Node>),
    /// Insertion-ordered key/value list; duplicate keys are allowed and
    /// exercise the overwrite path.
    Obj(Vec<(Vec<u8>, Node)>),
}

impl Node {
    /// Build this tree inside `l`, returning an owned reference (or NULL if a
    /// constructor legitimately refused the input).
    pub unsafe fn build(&self, l: &Lib) -> json_ptr {
        match self {
            Node::Null => l.json_null(),
            Node::True => l.json_true(),
            Node::False => l.json_false(),
            Node::Int(v) => l.json_integer(*v),
            Node::Real(v) => l.json_real(*v),
            Node::Str(b) => l.json_stringn(b.as_ptr() as *const c_char, b.len()),
            Node::StrNoCheck(b) => l.json_stringn_nocheck(b.as_ptr() as *const c_char, b.len()),
            Node::Arr(items) => {
                let a = l.json_array();
                if a.is_null() {
                    return a;
                }
                for it in items {
                    let v = it.build(l);
                    // `_new` steals the reference even on failure.
                    l.json_array_append_new(a, v);
                }
                a
            }
            Node::Obj(entries) => {
                let o = l.json_object();
                if o.is_null() {
                    return o;
                }
                for (k, v) in entries {
                    let vp = v.build(l);
                    l.json_object_setn_new_nocheck(o, k.as_ptr() as *const c_char, k.len(), vp);
                }
                o
            }
        }
    }
}

/// Build `node` in both libraries, hand both to `f`, then release them.
/// `f` receives `(&Pair, c_json, rust_json)`.
pub unsafe fn with_both<F: FnOnce(&'static Pair, json_ptr, json_ptr)>(node: &Node, f: F) {
    let p = pair();
    let cj = node.build(&p.c);
    let rj = node.build(&p.r);
    assert_eq!(
        cj.is_null(),
        rj.is_null(),
        "constructor NULL-ness diverged for {node:?}"
    );
    f(p, cj, rj);
    p.c.json_decref(cj);
    p.r.json_decref(rj);
}

/// Assert that `json_dumps` agrees byte-for-byte under `flags`.
pub unsafe fn assert_dumps_eq(p: &Pair, cj: json_ptr, rj: json_ptr, flags: usize, ctx: &str) {
    let cd = p.c.dumps(cj, flags);
    let rd = p.r.dumps(rj, flags);
    assert_eq!(
        cd,
        rd,
        "json_dumps(flags=0x{flags:x}) mismatch [{ctx}]:\n  C   = {}\n  RUST= {}",
        show(&cd),
        show(&rd)
    );
}

// ---------------------------------------------------------------------------
// Random tree generation
// ---------------------------------------------------------------------------

impl Rng {
    /// A random key: mostly short ASCII, sometimes non-ASCII UTF-8, sometimes
    /// empty, sometimes long enough to exercise the hashtable's memcmp path.
    pub fn key_bytes(&mut self) -> Vec<u8> {
        match self.below(10) {
            0 => Vec::new(),
            1 => self.utf8(6).into_bytes(),
            2 => vec![b'k'; 1 + self.below(300)],
            3 => format!("dup{}", self.below(3)).into_bytes(),
            _ => format!("k{}", self.below(64)).into_bytes(),
        }
    }

    /// A random UTF-8-valid scalar or container, `depth` levels deep.
    pub fn node(&mut self, depth: usize) -> Node {
        let pick = if depth == 0 { self.below(6) } else { self.below(8) };
        match pick {
            0 => Node::Null,
            1 => {
                if self.bool() {
                    Node::True
                } else {
                    Node::False
                }
            }
            2 => Node::Int(match self.below(6) {
                0 => 0,
                1 => i64::MAX,
                2 => i64::MIN,
                3 => self.range(-1000, 1000),
                4 => self.next_u64() as i64,
                _ => self.range(-(1 << 40), 1 << 40),
            }),
            3 => Node::Real(self.finite_f64()),
            4 | 5 => Node::Str(self.utf8(10).into_bytes()),
            6 => {
                let n = self.below(6);
                Node::Arr((0..n).map(|_| self.node(depth - 1)).collect())
            }
            _ => {
                let n = self.below(6);
                Node::Obj(
                    (0..n)
                        .map(|_| (self.key_bytes(), self.node(depth - 1)))
                        .collect(),
                )
            }
        }
    }

    /// A random *container* root (what `json_dumps` accepts without
    /// `JSON_ENCODE_ANY`).
    pub fn container(&mut self, depth: usize) -> Node {
        if self.bool() {
            let n = self.below(7);
            Node::Arr((0..n).map(|_| self.node(depth)).collect())
        } else {
            let n = self.below(7);
            Node::Obj(
                (0..n)
                    .map(|_| (self.key_bytes(), self.node(depth)))
                    .collect(),
            )
        }
    }
}
