//! Differential-test harness: loads BOTH the C `.so` and the Rust `.so` via
//! `libloading` and exposes every exported symbol as a function pointer so the
//! two implementations can be driven identically through the FFI boundary.
//!
//! Rust functions are NEVER called directly — everything goes through
//! `dlsym` on `target/<profile>/libjansson.so`, exactly like an external
//! consumer, which also exercises the `#[no_mangle]` export wrappers.
#![allow(dead_code)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_uint, c_void, CString};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// C types
// ---------------------------------------------------------------------------

pub const JSON_OBJECT: c_int = 0;
pub const JSON_ARRAY: c_int = 1;
pub const JSON_STRING: c_int = 2;
pub const JSON_INTEGER: c_int = 3;
pub const JSON_REAL: c_int = 4;
pub const JSON_TRUE: c_int = 5;
pub const JSON_FALSE: c_int = 6;
pub const JSON_NULL: c_int = 7;

// decode flags
pub const JSON_REJECT_DUPLICATES: usize = 0x1;
pub const JSON_DISABLE_EOF_CHECK: usize = 0x2;
pub const JSON_DECODE_ANY: usize = 0x4;
pub const JSON_DECODE_INT_AS_REAL: usize = 0x8;
pub const JSON_ALLOW_NUL: usize = 0x10;

// encode flags
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

// pack/unpack flags
pub const JSON_VALIDATE_ONLY: usize = 0x1;
pub const JSON_STRICT: usize = 0x2;

pub const JSON_ERROR_TEXT_LENGTH: usize = 160;
pub const JSON_ERROR_SOURCE_LENGTH: usize = 80;

// json_error_code
pub const json_error_unknown: c_int = 0;
pub const json_error_out_of_memory: c_int = 1;
pub const json_error_stack_overflow: c_int = 2;
pub const json_error_cannot_open_file: c_int = 3;
pub const json_error_invalid_argument: c_int = 4;
pub const json_error_invalid_utf8: c_int = 5;
pub const json_error_premature_end_of_input: c_int = 6;
pub const json_error_end_of_input_expected: c_int = 7;
pub const json_error_invalid_syntax: c_int = 8;
pub const json_error_invalid_format: c_int = 9;
pub const json_error_wrong_type: c_int = 10;
pub const json_error_null_character: c_int = 11;
pub const json_error_null_value: c_int = 12;
pub const json_error_null_byte_in_key: c_int = 13;
pub const json_error_duplicate_key: c_int = 14;
pub const json_error_numeric_overflow: c_int = 15;
pub const json_error_item_not_found: c_int = 16;
pub const json_error_index_out_of_range: c_int = 17;

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct json_t {
    pub type_: c_int,
    pub refcount: usize,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct json_error_t {
    pub line: c_int,
    pub column: c_int,
    pub position: c_int,
    pub source: [c_char; JSON_ERROR_SOURCE_LENGTH],
    pub text: [c_char; JSON_ERROR_TEXT_LENGTH],
}

impl json_error_t {
    pub fn new() -> json_error_t {
        json_error_t {
            line: 0,
            column: 0,
            position: 0,
            source: [0; JSON_ERROR_SOURCE_LENGTH],
            text: [0; JSON_ERROR_TEXT_LENGTH],
        }
    }
    /// The `enum json_error_code` byte the C stores in `text[159]`.
    pub fn code(&self) -> c_int {
        self.text[JSON_ERROR_TEXT_LENGTH - 1] as c_int
    }
    pub fn text_str(&self) -> String {
        cstr_to_string_lossy(self.text.as_ptr())
    }
    pub fn source_str(&self) -> String {
        cstr_to_string_lossy(self.source.as_ptr())
    }
    /// Everything an external caller can observe, in one comparable value.
    pub fn snapshot(&self) -> (c_int, c_int, c_int, String, String, i32) {
        (
            self.line,
            self.column,
            self.position,
            self.source_str(),
            self.text_str(),
            self.code(),
        )
    }
    /// Raw bytes of the whole 252-byte struct (byte-for-byte comparison).
    pub fn raw(&self) -> Vec<u8> {
        let p = self as *const json_error_t as *const u8;
        unsafe { std::slice::from_raw_parts(p, std::mem::size_of::<json_error_t>()) }.to_vec()
    }
}

impl std::fmt::Debug for json_error_t {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "json_error_t {{ line: {}, column: {}, position: {}, source: {:?}, text: {:?}, code: {} }}",
            self.line, self.column, self.position, self.source_str(), self.text_str(), self.code()
        )
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct hashtable_list {
    pub prev: *mut hashtable_list,
    pub next: *mut hashtable_list,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct hashtable_t {
    pub size: usize,
    pub buckets: *mut c_void,
    pub order: usize,
    pub list: hashtable_list,
    pub ordered_list: hashtable_list,
}

impl hashtable_t {
    pub fn zeroed() -> hashtable_t {
        unsafe { std::mem::zeroed() }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
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
}

/// `dtoa.c`'s `U` union — a double aliased with two 32-bit words.
#[repr(C)]
#[derive(Copy, Clone)]
pub union U {
    pub d: f64,
    pub L: [u32; 2],
}

impl U {
    pub fn zero() -> U {
        U { d: 0.0 }
    }
    pub fn bits(&self) -> u64 {
        unsafe { self.d.to_bits() }
    }
}

/// x86-64 SysV `va_list`. Passing `*mut VaListTag` matches the C ABI, where
/// `va_list` is an array of one such struct and therefore decays to a pointer.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct VaListTag {
    pub gp_offset: c_uint,
    pub fp_offset: c_uint,
    pub overflow_arg_area: *mut c_void,
    pub reg_save_area: *mut c_void,
}

/// Builds a real `va_list` whose arguments all live in the overflow area, so a
/// flat array of 8-byte slots is consumed in order. `gp_offset = 48` and
/// `fp_offset = 176` are past the register save area, which forces every
/// `va_arg` (integer *and* SSE) down the `overflow_arg_area` path.
pub struct VaArgs {
    words: Vec<u64>,
    tag: VaListTag,
}

impl VaArgs {
    pub fn new() -> VaArgs {
        VaArgs {
            words: Vec::new(),
            tag: VaListTag {
                gp_offset: 48,
                fp_offset: 176,
                overflow_arg_area: std::ptr::null_mut(),
                reg_save_area: std::ptr::null_mut(),
            },
        }
    }
    pub fn int(mut self, v: c_int) -> Self {
        self.words.push(v as u32 as u64);
        self
    }
    pub fn i64(mut self, v: i64) -> Self {
        self.words.push(v as u64);
        self
    }
    pub fn usize(mut self, v: usize) -> Self {
        self.words.push(v as u64);
        self
    }
    pub fn f64(mut self, v: f64) -> Self {
        self.words.push(v.to_bits());
        self
    }
    pub fn ptr<T>(mut self, v: *const T) -> Self {
        self.words.push(v as usize as u64);
        self
    }
    pub fn ptr_mut<T>(mut self, v: *mut T) -> Self {
        self.words.push(v as usize as u64);
        self
    }
    /// Finalise: returns a `va_list` pointer valid while `self` is alive.
    /// Extra trailing slots guard against a format string reading too far.
    pub fn build(&mut self) -> *mut VaListTag {
        for _ in 0..8 {
            self.words.push(0);
        }
        self.tag.overflow_arg_area = self.words.as_mut_ptr() as *mut c_void;
        self.tag.reg_save_area = std::ptr::null_mut();
        &mut self.tag as *mut VaListTag
    }
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

pub fn cstr_to_string_lossy(p: *const c_char) -> String {
    if p.is_null() {
        return String::from("<null>");
    }
    let mut v = Vec::new();
    let mut i = 0isize;
    unsafe {
        while *p.offset(i) != 0 {
            v.push(*p.offset(i) as u8);
            i += 1;
            if i > 1 << 22 {
                break;
            }
        }
    }
    String::from_utf8_lossy(&v).into_owned()
}

pub fn cstr_bytes(p: *const c_char) -> Vec<u8> {
    if p.is_null() {
        return Vec::new();
    }
    let mut v = Vec::new();
    let mut i = 0isize;
    unsafe {
        while *p.offset(i) != 0 {
            v.push(*p.offset(i) as u8);
            i += 1;
            if i > 1 << 24 {
                break;
            }
        }
    }
    v
}

pub fn cs(s: &str) -> CString {
    CString::new(s.as_bytes()).expect("no interior NUL")
}

/// A NUL-terminated byte buffer that may contain interior NULs.
pub fn cbuf(bytes: &[u8]) -> Vec<u8> {
    let mut v = bytes.to_vec();
    v.push(0);
    v
}

// ---------------------------------------------------------------------------
// Function pointer types
// ---------------------------------------------------------------------------

pub type FnVoidToPtr = unsafe extern "C" fn() -> *mut json_t;
pub type FnStrToPtr = unsafe extern "C" fn(*const c_char) -> *mut json_t;
pub type FnStrLenToPtr = unsafe extern "C" fn(*const c_char, usize) -> *mut json_t;
pub type FnI64ToPtr = unsafe extern "C" fn(i64) -> *mut json_t;
pub type FnF64ToPtr = unsafe extern "C" fn(f64) -> *mut json_t;
pub type FnPtrToVoid = unsafe extern "C" fn(*mut json_t);
pub type FnPtrToUsize = unsafe extern "C" fn(*const json_t) -> usize;
pub type FnPtrToInt = unsafe extern "C" fn(*const json_t) -> c_int;
pub type FnPtrToI64 = unsafe extern "C" fn(*const json_t) -> i64;
pub type FnPtrToF64 = unsafe extern "C" fn(*const json_t) -> f64;
pub type FnPtrToCStr = unsafe extern "C" fn(*const json_t) -> *const c_char;
pub type FnMutPtrToInt = unsafe extern "C" fn(*mut json_t) -> c_int;

pub type FnDumpCallback = unsafe extern "C" fn(*const c_char, usize, *mut c_void) -> c_int;
pub type FnLoadCallback = unsafe extern "C" fn(*mut c_void, usize, *mut c_void) -> usize;

macro_rules! define_lib {
    ( $( $name:ident : $ty:ty ),* $(,)? ) => {
        /// One loaded shared object with every exported function resolved.
        pub struct Lib {
            pub which: &'static str,
            pub path: PathBuf,
            $( pub $name: $ty, )*
            _lib: Library,
        }

        impl Lib {
            pub fn open(which: &'static str, path: PathBuf) -> Lib {
                unsafe {
                    let lib = Library::new(&path)
                        .unwrap_or_else(|e| panic!("cannot dlopen {}: {}", path.display(), e));
                    $(
                        let $name: $ty = {
                            let mut nm: Vec<u8> = stringify!($name).as_bytes().to_vec();
                            nm.push(0);
                            let s: Symbol<$ty> = lib.get(&nm).unwrap_or_else(|e| {
                                panic!("{}: missing symbol {}: {}", which, stringify!($name), e)
                            });
                            *s
                        };
                    )*
                    Lib { which, path, $( $name, )* _lib: lib }
                }
            }

            /// Read a `static`/`extern` data symbol.
            pub unsafe fn data<T: Copy>(&self, name: &str) -> T {
                let mut nm = name.as_bytes().to_vec();
                nm.push(0);
                let s: Symbol<*mut T> = self
                    ._lib
                    .get(&nm)
                    .unwrap_or_else(|e| panic!("{}: missing data symbol {}: {}", self.which, name, e));
                **s
            }

            pub unsafe fn data_ptr<T>(&self, name: &str) -> *mut T {
                let mut nm = name.as_bytes().to_vec();
                nm.push(0);
                let s: Symbol<*mut T> = self
                    ._lib
                    .get(&nm)
                    .unwrap_or_else(|e| panic!("{}: missing data symbol {}: {}", self.which, name, e));
                *s
            }

            pub fn has_symbol(&self, name: &str) -> bool {
                let mut nm = name.as_bytes().to_vec();
                nm.push(0);
                unsafe { self._lib.get::<*mut c_void>(&nm).is_ok() }
            }
        }
    };
}

define_lib! {
    // ---- value.c: construction / refcounting -------------------------------
    json_object: FnVoidToPtr,
    json_array: FnVoidToPtr,
    json_string: FnStrToPtr,
    json_stringn: FnStrLenToPtr,
    json_string_nocheck: FnStrToPtr,
    json_stringn_nocheck: FnStrLenToPtr,
    json_integer: FnI64ToPtr,
    json_real: FnF64ToPtr,
    json_true: FnVoidToPtr,
    json_false: FnVoidToPtr,
    json_null: FnVoidToPtr,
    json_delete: FnPtrToVoid,

    // ---- object -----------------------------------------------------------
    json_object_seed: unsafe extern "C" fn(usize),
    json_object_size: FnPtrToUsize,
    json_object_get: unsafe extern "C" fn(*const json_t, *const c_char) -> *mut json_t,
    json_object_getn: unsafe extern "C" fn(*const json_t, *const c_char, usize) -> *mut json_t,
    json_object_set_new: unsafe extern "C" fn(*mut json_t, *const c_char, *mut json_t) -> c_int,
    json_object_setn_new:
        unsafe extern "C" fn(*mut json_t, *const c_char, usize, *mut json_t) -> c_int,
    json_object_set_new_nocheck:
        unsafe extern "C" fn(*mut json_t, *const c_char, *mut json_t) -> c_int,
    json_object_setn_new_nocheck:
        unsafe extern "C" fn(*mut json_t, *const c_char, usize, *mut json_t) -> c_int,
    json_object_del: unsafe extern "C" fn(*mut json_t, *const c_char) -> c_int,
    json_object_deln: unsafe extern "C" fn(*mut json_t, *const c_char, usize) -> c_int,
    json_object_clear: FnMutPtrToInt,
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
    json_object_iter_key_len: unsafe extern "C" fn(*mut c_void) -> usize,
    json_object_iter_value: unsafe extern "C" fn(*mut c_void) -> *mut json_t,
    json_object_iter_set_new:
        unsafe extern "C" fn(*mut json_t, *mut c_void, *mut json_t) -> c_int,

    // ---- array ------------------------------------------------------------
    json_array_size: FnPtrToUsize,
    json_array_get: unsafe extern "C" fn(*const json_t, usize) -> *mut json_t,
    json_array_set_new: unsafe extern "C" fn(*mut json_t, usize, *mut json_t) -> c_int,
    json_array_append_new: unsafe extern "C" fn(*mut json_t, *mut json_t) -> c_int,
    json_array_insert_new: unsafe extern "C" fn(*mut json_t, usize, *mut json_t) -> c_int,
    json_array_remove: unsafe extern "C" fn(*mut json_t, usize) -> c_int,
    json_array_clear: FnMutPtrToInt,
    json_array_extend: unsafe extern "C" fn(*mut json_t, *mut json_t) -> c_int,

    // ---- scalars ----------------------------------------------------------
    json_string_value: FnPtrToCStr,
    json_string_length: FnPtrToUsize,
    json_integer_value: FnPtrToI64,
    json_real_value: FnPtrToF64,
    json_number_value: FnPtrToF64,
    json_string_set: unsafe extern "C" fn(*mut json_t, *const c_char) -> c_int,
    json_string_setn: unsafe extern "C" fn(*mut json_t, *const c_char, usize) -> c_int,
    json_string_set_nocheck: unsafe extern "C" fn(*mut json_t, *const c_char) -> c_int,
    json_string_setn_nocheck: unsafe extern "C" fn(*mut json_t, *const c_char, usize) -> c_int,
    json_integer_set: unsafe extern "C" fn(*mut json_t, i64) -> c_int,
    json_real_set: unsafe extern "C" fn(*mut json_t, f64) -> c_int,

    // ---- equality / copying ----------------------------------------------
    json_equal: unsafe extern "C" fn(*const json_t, *const json_t) -> c_int,
    json_copy: unsafe extern "C" fn(*mut json_t) -> *mut json_t,
    json_deep_copy: unsafe extern "C" fn(*const json_t) -> *mut json_t,
    do_deep_copy: unsafe extern "C" fn(*const json_t, *mut hashtable_t) -> *mut json_t,

    // ---- pack / unpack / sprintf -----------------------------------------
    json_pack: unsafe extern "C" fn(*const c_char, ...) -> *mut json_t,
    json_pack_ex: unsafe extern "C" fn(*mut json_error_t, usize, *const c_char, ...) -> *mut json_t,
    json_vpack_ex:
        unsafe extern "C" fn(*mut json_error_t, usize, *const c_char, *mut VaListTag) -> *mut json_t,
    json_unpack: unsafe extern "C" fn(*mut json_t, *const c_char, ...) -> c_int,
    json_unpack_ex:
        unsafe extern "C" fn(*mut json_t, *mut json_error_t, usize, *const c_char, ...) -> c_int,
    json_vunpack_ex: unsafe extern "C" fn(
        *mut json_t,
        *mut json_error_t,
        usize,
        *const c_char,
        *mut VaListTag,
    ) -> c_int,
    json_sprintf: unsafe extern "C" fn(*const c_char, ...) -> *mut json_t,
    json_vsprintf: unsafe extern "C" fn(*const c_char, *mut VaListTag) -> *mut json_t,

    // ---- load.c -----------------------------------------------------------
    json_loads: unsafe extern "C" fn(*const c_char, usize, *mut json_error_t) -> *mut json_t,
    json_loadb:
        unsafe extern "C" fn(*const c_char, usize, usize, *mut json_error_t) -> *mut json_t,
    json_loadf: unsafe extern "C" fn(*mut c_void, usize, *mut json_error_t) -> *mut json_t,
    json_loadfd: unsafe extern "C" fn(c_int, usize, *mut json_error_t) -> *mut json_t,
    json_load_file: unsafe extern "C" fn(*const c_char, usize, *mut json_error_t) -> *mut json_t,
    json_load_callback: unsafe extern "C" fn(
        Option<FnLoadCallback>,
        *mut c_void,
        usize,
        *mut json_error_t,
    ) -> *mut json_t,

    // ---- dump.c -----------------------------------------------------------
    json_dumps: unsafe extern "C" fn(*const json_t, usize) -> *mut c_char,
    json_dumpb: unsafe extern "C" fn(*const json_t, *mut c_char, usize, usize) -> usize,
    json_dumpf: unsafe extern "C" fn(*const json_t, *mut c_void, usize) -> c_int,
    json_dumpfd: unsafe extern "C" fn(*const json_t, c_int, usize) -> c_int,
    json_dump_file: unsafe extern "C" fn(*const json_t, *const c_char, usize) -> c_int,
    json_dump_callback: unsafe extern "C" fn(
        *const json_t,
        Option<FnDumpCallback>,
        *mut c_void,
        usize,
    ) -> c_int,

    // ---- memory.c ---------------------------------------------------------
    json_set_alloc_funcs: unsafe extern "C" fn(
        Option<unsafe extern "C" fn(usize) -> *mut c_void>,
        Option<unsafe extern "C" fn(*mut c_void)>,
    ),
    json_get_alloc_funcs: unsafe extern "C" fn(*mut *mut c_void, *mut *mut c_void),
    json_set_alloc_funcs2: unsafe extern "C" fn(
        Option<unsafe extern "C" fn(usize) -> *mut c_void>,
        Option<unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void>,
        Option<unsafe extern "C" fn(*mut c_void)>,
    ),
    json_get_alloc_funcs2:
        unsafe extern "C" fn(*mut *mut c_void, *mut *mut c_void, *mut *mut c_void),
    jsonp_malloc: unsafe extern "C" fn(usize) -> *mut c_void,
    jsonp_free: unsafe extern "C" fn(*mut c_void),
    jsonp_realloc: unsafe extern "C" fn(*mut c_void, usize, usize) -> *mut c_void,
    jsonp_strndup: unsafe extern "C" fn(*const c_char, usize) -> *mut c_char,

    // ---- version.c --------------------------------------------------------
    jansson_version_str: unsafe extern "C" fn() -> *const c_char,
    jansson_version_cmp: unsafe extern "C" fn(c_int, c_int, c_int) -> c_int,

    // ---- error.c ----------------------------------------------------------
    jsonp_error_init: unsafe extern "C" fn(*mut json_error_t, *const c_char),
    jsonp_error_set_source: unsafe extern "C" fn(*mut json_error_t, *const c_char),
    jsonp_error_set:
        unsafe extern "C" fn(*mut json_error_t, c_int, c_int, usize, c_int, *const c_char, ...),
    jsonp_error_vset: unsafe extern "C" fn(
        *mut json_error_t,
        c_int,
        c_int,
        usize,
        c_int,
        *const c_char,
        *mut VaListTag,
    ),

    // ---- strbuffer.c ------------------------------------------------------
    strbuffer_init: unsafe extern "C" fn(*mut strbuffer_t) -> c_int,
    strbuffer_close: unsafe extern "C" fn(*mut strbuffer_t),
    strbuffer_clear: unsafe extern "C" fn(*mut strbuffer_t),
    strbuffer_value: unsafe extern "C" fn(*const strbuffer_t) -> *const c_char,
    strbuffer_steal_value: unsafe extern "C" fn(*mut strbuffer_t) -> *mut c_char,
    strbuffer_append_byte: unsafe extern "C" fn(*mut strbuffer_t, c_char) -> c_int,
    strbuffer_append_bytes: unsafe extern "C" fn(*mut strbuffer_t, *const c_char, usize) -> c_int,
    strbuffer_pop: unsafe extern "C" fn(*mut strbuffer_t) -> c_char,

    // ---- utf.c ------------------------------------------------------------
    utf8_encode: unsafe extern "C" fn(i32, *mut c_char, *mut usize) -> c_int,
    utf8_check_first: unsafe extern "C" fn(c_char) -> usize,
    utf8_check_full: unsafe extern "C" fn(*const c_char, usize, *mut i32) -> usize,
    utf8_iterate: unsafe extern "C" fn(*const c_char, usize, *mut i32) -> *const c_char,
    utf8_check_string: unsafe extern "C" fn(*const c_char, usize) -> c_int,

    // ---- hashtable.c ------------------------------------------------------
    hashtable_init: unsafe extern "C" fn(*mut hashtable_t) -> c_int,
    hashtable_close: unsafe extern "C" fn(*mut hashtable_t),
    hashtable_set:
        unsafe extern "C" fn(*mut hashtable_t, *const c_char, usize, *mut json_t) -> c_int,
    hashtable_get: unsafe extern "C" fn(*mut hashtable_t, *const c_char, usize) -> *mut c_void,
    hashtable_del: unsafe extern "C" fn(*mut hashtable_t, *const c_char, usize) -> c_int,
    hashtable_clear: unsafe extern "C" fn(*mut hashtable_t),
    hashtable_iter: unsafe extern "C" fn(*mut hashtable_t) -> *mut c_void,
    hashtable_iter_at:
        unsafe extern "C" fn(*mut hashtable_t, *const c_char, usize) -> *mut c_void,
    hashtable_iter_next: unsafe extern "C" fn(*mut hashtable_t, *mut c_void) -> *mut c_void,
    hashtable_iter_key: unsafe extern "C" fn(*mut c_void) -> *mut c_char,
    hashtable_iter_key_len: unsafe extern "C" fn(*mut c_void) -> usize,
    hashtable_iter_value: unsafe extern "C" fn(*mut c_void) -> *mut c_void,
    hashtable_iter_set: unsafe extern "C" fn(*mut c_void, *mut json_t),

    // ---- strconv.c --------------------------------------------------------
    jsonp_strtod: unsafe extern "C" fn(*mut strbuffer_t, *mut f64) -> c_int,
    jsonp_dtostr: unsafe extern "C" fn(*mut c_char, usize, f64, c_int) -> c_int,

    // ---- value.c private --------------------------------------------------
    jsonp_stringn_nocheck_own: FnStrLenToPtr,
    jsonp_loop_check:
        unsafe extern "C" fn(*mut hashtable_t, *const json_t, *mut c_char, usize, *mut usize) -> c_int,

    // ---- dtoa.c -----------------------------------------------------------
    dtoa: unsafe extern "C" fn(
        f64, c_int, c_int, *mut c_int, *mut c_int, *mut *mut c_char,
    ) -> *mut c_char,
    dtoa_r: unsafe extern "C" fn(
        f64, c_int, c_int, *mut c_int, *mut c_int, *mut *mut c_char, *mut c_char, usize,
    ) -> *mut c_char,
    freedtoa: unsafe extern "C" fn(*mut c_char),
    gethex: unsafe extern "C" fn(*mut *const c_char, *mut U, c_int, c_int),
    strtod__unused: unsafe extern "C" fn(*const c_char, *mut *mut c_char) -> f64,
}

// ---------------------------------------------------------------------------
// Locating the two shared objects
// ---------------------------------------------------------------------------

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_so_path() -> PathBuf {
    manifest_dir().join("c_src/build/libjansson.so")
}

pub fn rust_so_path() -> PathBuf {
    // current_exe() == target/<profile>/deps/<test>-<hash>
    if let Ok(exe) = std::env::current_exe() {
        if let Some(deps) = exe.parent() {
            if let Some(profile) = deps.parent() {
                let p = profile.join("libjansson.so");
                if p.exists() {
                    return p;
                }
            }
        }
    }
    for p in ["target/debug/libjansson.so", "target/release/libjansson.so"] {
        let p = manifest_dir().join(p);
        if p.exists() {
            return p;
        }
    }
    panic!("cannot find the Rust libjansson.so — run `cargo build` first");
}

/// The deterministic hashtable seed used by every test. Both libraries are
/// seeded with the same value so that bucket assignment, and therefore object
/// iteration order and `JSON_SORT_KEYS`-free dump order, is identical.
pub const TEST_SEED: usize = 0x5eed_1234;

/// Both loaded libraries, seeded identically.
pub struct Duo {
    pub c: Lib,
    pub rs: Lib,
}

impl Duo {
    pub fn load() -> Duo {
        let c = Lib::open("C", c_so_path());
        let rs = Lib::open("RUST", rust_so_path());
        // Seed BEFORE anything creates an object (json_object() auto-seeds).
        unsafe {
            (c.json_object_seed)(TEST_SEED);
            (rs.json_object_seed)(TEST_SEED);
            let cs: u32 = c.data("hashtable_seed");
            let rss: u32 = rs.data("hashtable_seed");
            assert_eq!(cs, rss, "hashtable_seed differs after identical seeding");
            assert_ne!(cs, 0, "hashtable_seed must be non-zero after seeding");
        }
        Duo { c, rs }
    }

    pub fn both(&self) -> [&Lib; 2] {
        [&self.c, &self.rs]
    }
}

/// Process-wide singleton: `dlopen`ing the same object twice per test binary is
/// wasteful and the libraries hold mutable global state (allocator hooks, seed).
pub fn duo() -> &'static Duo {
    use std::sync::OnceLock;
    static D: OnceLock<Duo> = OnceLock::new();
    D.get_or_init(Duo::load)
}

unsafe impl Send for Duo {}
unsafe impl Sync for Duo {}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) — fixed seed, reproducible
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(if seed == 0 { 0x9E3779B97F4A7C15 } else { seed })
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
    /// Uniform in `0..n`.
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }
    pub fn range_i64(&mut self, lo: i64, hi: i64) -> i64 {
        if hi <= lo {
            return lo;
        }
        lo + (self.next_u64() % ((hi - lo) as u64)) as i64
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
    /// A finite `f64` drawn from a wide range of magnitudes.
    pub fn finite_f64(&mut self) -> f64 {
        loop {
            let bits = self.next_u64();
            let v = f64::from_bits(bits);
            if v.is_finite() {
                return v;
            }
        }
    }
    /// A "reasonable" f64: mantissa * 10^exp with exp in -30..30.
    pub fn tame_f64(&mut self) -> f64 {
        let m = (self.next_u64() % 2_000_000_000) as f64 - 1_000_000_000.0;
        let e = self.range_i64(-30, 30) as i32;
        let v = m * 10f64.powi(e);
        if v.is_finite() {
            v
        } else {
            0.0
        }
    }
    pub fn ascii_string(&mut self, len: usize) -> Vec<u8> {
        (0..len)
            .map(|_| {
                let c = b" !#$%&'()*+,-./0123456789:;<=>?@ABCXYZ[]^_`abcxyz{|}~";
                c[self.below(c.len())]
            })
            .collect()
    }
    pub fn random_bytes(&mut self, len: usize) -> Vec<u8> {
        (0..len).map(|_| (self.next_u32() & 0xFF) as u8).collect()
    }
    /// A valid UTF-8 string mixing 1-, 2-, 3- and 4-byte sequences.
    pub fn utf8_string(&mut self, chars: usize) -> Vec<u8> {
        let mut out = Vec::new();
        for _ in 0..chars {
            let cp: u32 = match self.below(4) {
                0 => 0x20 + self.below(0x5F) as u32,
                1 => 0x80 + self.below(0x780) as u32,
                2 => loop {
                    let c = 0x800 + self.below(0xF800) as u32;
                    if !(0xD800..=0xDFFF).contains(&c) {
                        break c;
                    }
                },
                _ => 0x10000 + self.below(0x100000) as u32,
            };
            let mut b = [0u8; 4];
            out.extend_from_slice(char::from_u32(cp).unwrap().encode_utf8(&mut b).as_bytes());
        }
        out
    }
}

// ---------------------------------------------------------------------------
// Assertion helpers
// ---------------------------------------------------------------------------

#[track_caller]
pub fn eq<T: PartialEq + std::fmt::Debug>(what: &str, cv: T, rv: T) {
    assert_eq!(cv, rv, "C vs RUST divergence in {}", what);
}

#[track_caller]
pub fn eq_bytes(what: &str, cv: &[u8], rv: &[u8]) {
    if cv != rv {
        panic!(
            "C vs RUST divergence in {}\n  C   ({:>4} bytes): {:?}\n  RUST({:>4} bytes): {:?}",
            what,
            cv.len(),
            String::from_utf8_lossy(cv),
            rv.len(),
            String::from_utf8_lossy(rv),
        );
    }
}

#[track_caller]
pub fn eq_err(what: &str, ce: &json_error_t, re: &json_error_t) {
    if ce.raw() != re.raw() {
        panic!(
            "C vs RUST json_error_t divergence in {}\n  C   : {:?}\n  RUST: {:?}",
            what, ce, re
        );
    }
}

/// `json_dumps` on both libraries; returns the two byte strings (freed).
pub fn dumps_both(d: &Duo, cj: *mut json_t, rj: *mut json_t, flags: usize) -> (Option<Vec<u8>>, Option<Vec<u8>>) {
    unsafe {
        let cp = (d.c.json_dumps)(cj, flags);
        let rp = (d.rs.json_dumps)(rj, flags);
        let cv = if cp.is_null() { None } else { Some(cstr_bytes(cp)) };
        let rv = if rp.is_null() { None } else { Some(cstr_bytes(rp)) };
        if !cp.is_null() {
            (d.c.jsonp_free)(cp as *mut c_void);
        }
        if !rp.is_null() {
            (d.rs.jsonp_free)(rp as *mut c_void);
        }
        (cv, rv)
    }
}

/// Fully-observable snapshot of a value tree, produced only through public
/// getters, so it can be compared between the two libraries.
pub fn describe(l: &Lib, j: *const json_t) -> String {
    let mut s = String::new();
    describe_into(l, j, &mut s, 0);
    s
}

fn describe_into(l: &Lib, j: *const json_t, out: &mut String, depth: usize) {
    if j.is_null() {
        out.push_str("NULL");
        return;
    }
    if depth > 64 {
        out.push_str("<deep>");
        return;
    }
    let ty = unsafe { (*j).type_ };
    let rc = unsafe { (*j).refcount };
    match ty {
        JSON_OBJECT => unsafe {
            out.push_str(&format!("obj(rc={},n={}){{", rc, (l.json_object_size)(j)));
            let mut it = (l.json_object_iter)(j as *mut json_t);
            while !it.is_null() {
                let k = (l.json_object_iter_key)(it);
                let kl = (l.json_object_iter_key_len)(it);
                let kb = std::slice::from_raw_parts(k as *const u8, kl);
                out.push_str(&format!("{:?}=", String::from_utf8_lossy(kb)));
                describe_into(l, (l.json_object_iter_value)(it), out, depth + 1);
                out.push(',');
                it = (l.json_object_iter_next)(j as *mut json_t, it);
            }
            out.push('}');
        },
        JSON_ARRAY => unsafe {
            let n = (l.json_array_size)(j);
            out.push_str(&format!("arr(rc={},n={})[", rc, n));
            for i in 0..n {
                describe_into(l, (l.json_array_get)(j, i), out, depth + 1);
                out.push(',');
            }
            out.push(']');
        },
        JSON_STRING => unsafe {
            let p = (l.json_string_value)(j);
            let n = (l.json_string_length)(j);
            let b = if p.is_null() {
                &[][..]
            } else {
                std::slice::from_raw_parts(p as *const u8, n)
            };
            out.push_str(&format!("str(rc={},{})={:?}", rc, n, b));
        },
        JSON_INTEGER => unsafe {
            out.push_str(&format!("int(rc={})={}", rc, (l.json_integer_value)(j)));
        },
        JSON_REAL => unsafe {
            out.push_str(&format!(
                "real(rc={})={:#018x}",
                rc,
                (l.json_real_value)(j).to_bits()
            ));
        },
        JSON_TRUE => out.push_str(&format!("true(rc={})", rc)),
        JSON_FALSE => out.push_str(&format!("false(rc={})", rc)),
        JSON_NULL => out.push_str(&format!("null(rc={})", rc)),
        other => out.push_str(&format!("<type {}>(rc={})", other, rc)),
    }
}

/// Drop a value on its own library (handles the immortal singletons).
pub fn decref(l: &Lib, j: *mut json_t) {
    if j.is_null() {
        return;
    }
    unsafe {
        if (*j).refcount == usize::MAX {
            return;
        }
        (*j).refcount -= 1;
        if (*j).refcount == 0 {
            (l.json_delete)(j);
        }
    }
}

pub fn incref(j: *mut json_t) -> *mut json_t {
    if !j.is_null() {
        unsafe {
            if (*j).refcount != usize::MAX {
                (*j).refcount += 1;
            }
        }
    }
    j
}
