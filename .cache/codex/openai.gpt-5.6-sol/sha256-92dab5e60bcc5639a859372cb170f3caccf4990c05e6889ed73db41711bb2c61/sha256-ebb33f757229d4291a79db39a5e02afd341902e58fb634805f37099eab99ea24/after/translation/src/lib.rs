#![allow(clippy::missing_safety_doc)]

use std::ffi::{c_char, c_double, c_int, c_void};

#[repr(C)]
pub struct JsonT {
    pub type_: c_int,
    pub refcount: usize,
}

#[repr(C)]
pub struct JsonError {
    pub line: c_int,
    pub column: c_int,
    pub position: c_int,
    pub source: [c_char; 80],
    pub text: [c_char; 160],
}

#[repr(C)]
pub struct StrBuffer {
    pub value: *mut c_char,
    pub length: usize,
    pub size: usize,
}

#[repr(C)]
pub struct Hashtable {
    _private: [u8; 0],
}

pub type LoadCallback =
    Option<unsafe extern "C" fn(*mut c_void, usize, *mut c_void) -> usize>;
pub type DumpCallback =
    Option<unsafe extern "C" fn(*const c_char, usize, *mut c_void) -> c_int>;
pub type MallocCallback = Option<unsafe extern "C" fn(usize) -> *mut c_void>;
pub type ReallocCallback =
    Option<unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void>;
pub type FreeCallback = Option<unsafe extern "C" fn(*mut c_void)>;

#[unsafe(no_mangle)]
pub static mut dtoa_divmax: c_int = 2;

#[unsafe(no_mangle)]
pub static mut hashtable_seed: u32 = 0;

macro_rules! export_tail_shim {
    ($name:ident($($arg:ident: $ty:ty),* $(,)?) -> $ret:ty) => {
        #[unsafe(naked)]
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name($($arg: $ty),*) -> $ret {
            core::arch::naked_asm!(concat!("jmp rust_impl_", stringify!($name)));
        }
    };
    ($name:ident($($arg:ident: $ty:ty),* $(,)?)) => {
        #[unsafe(naked)]
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name($($arg: $ty),*) {
            core::arch::naked_asm!(concat!("jmp rust_impl_", stringify!($name)));
        }
    };
}

macro_rules! export_variadic_tail_shims {
    ($($name:ident),+ $(,)?) => {
        $(
            // Stable Rust cannot define C-variadic functions. A naked tail jump
            // preserves all integer, vector, stack, and varargs metadata registers.
            #[unsafe(naked)]
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn $name() {
                core::arch::naked_asm!(
                    concat!("jmp rust_impl_", stringify!($name))
                );
            }
        )+
    };
}

export_tail_shim!(do_deep_copy(
    json: *const JsonT,
    parents: *mut Hashtable,
) -> *mut JsonT);
export_tail_shim!(do_object_update_recursive(
    object: *mut JsonT,
    other: *mut JsonT,
    parents: *mut Hashtable,
) -> c_int);
export_tail_shim!(dtoa(
    value: c_double,
    mode: c_int,
    digits: c_int,
    decimal_point: *mut c_int,
    sign: *mut c_int,
    end: *mut *mut c_char,
) -> *mut c_char);
export_tail_shim!(dtoa_r(
    value: c_double,
    mode: c_int,
    digits: c_int,
    decimal_point: *mut c_int,
    sign: *mut c_int,
    end: *mut *mut c_char,
    buffer: *mut c_char,
    buffer_len: usize,
) -> *mut c_char);
export_tail_shim!(freedtoa(value: *mut c_char));
export_tail_shim!(gethex(
    input: *mut *const c_char,
    value: *mut c_void,
    rounding: c_int,
    sign: c_int,
));

export_tail_shim!(hashtable_clear(table: *mut Hashtable));
export_tail_shim!(hashtable_close(table: *mut Hashtable));
export_tail_shim!(hashtable_del(
    table: *mut Hashtable,
    key: *const c_char,
    key_len: usize,
) -> c_int);
export_tail_shim!(hashtable_get(
    table: *mut Hashtable,
    key: *const c_char,
    key_len: usize,
) -> *mut c_void);
export_tail_shim!(hashtable_init(table: *mut Hashtable) -> c_int);
export_tail_shim!(hashtable_iter(table: *mut Hashtable) -> *mut c_void);
export_tail_shim!(hashtable_iter_at(
    table: *mut Hashtable,
    key: *const c_char,
    key_len: usize,
) -> *mut c_void);
export_tail_shim!(hashtable_iter_key(iter: *mut c_void) -> *mut c_void);
export_tail_shim!(hashtable_iter_key_len(iter: *mut c_void) -> usize);
export_tail_shim!(hashtable_iter_next(
    table: *mut Hashtable,
    iter: *mut c_void,
) -> *mut c_void);
export_tail_shim!(hashtable_iter_set(iter: *mut c_void, value: *mut JsonT));
export_tail_shim!(hashtable_iter_value(iter: *mut c_void) -> *mut c_void);
export_tail_shim!(hashtable_set(
    table: *mut Hashtable,
    key: *const c_char,
    key_len: usize,
    value: *mut JsonT,
) -> c_int);

export_tail_shim!(jansson_version_cmp(
    major: c_int,
    minor: c_int,
    micro: c_int,
) -> c_int);
export_tail_shim!(jansson_version_str() -> *const c_char);

export_tail_shim!(json_array() -> *mut JsonT);
export_tail_shim!(json_array_append_new(array: *mut JsonT, value: *mut JsonT) -> c_int);
export_tail_shim!(json_array_clear(array: *mut JsonT) -> c_int);
export_tail_shim!(json_array_extend(array: *mut JsonT, other: *mut JsonT) -> c_int);
export_tail_shim!(json_array_get(array: *const JsonT, index: usize) -> *mut JsonT);
export_tail_shim!(json_array_insert_new(
    array: *mut JsonT,
    index: usize,
    value: *mut JsonT,
) -> c_int);
export_tail_shim!(json_array_remove(array: *mut JsonT, index: usize) -> c_int);
export_tail_shim!(json_array_set_new(
    array: *mut JsonT,
    index: usize,
    value: *mut JsonT,
) -> c_int);
export_tail_shim!(json_array_size(array: *const JsonT) -> usize);

export_tail_shim!(json_copy(value: *mut JsonT) -> *mut JsonT);
export_tail_shim!(json_deep_copy(value: *const JsonT) -> *mut JsonT);
export_tail_shim!(json_delete(value: *mut JsonT));
export_tail_shim!(json_dump_callback(
    value: *const JsonT,
    callback: DumpCallback,
    data: *mut c_void,
    flags: usize,
) -> c_int);
export_tail_shim!(json_dump_file(
    value: *const JsonT,
    path: *const c_char,
    flags: usize,
) -> c_int);
export_tail_shim!(json_dumpb(
    value: *const JsonT,
    buffer: *mut c_char,
    size: usize,
    flags: usize,
) -> usize);
export_tail_shim!(json_dumpf(
    value: *const JsonT,
    output: *mut c_void,
    flags: usize,
) -> c_int);
export_tail_shim!(json_dumpfd(value: *const JsonT, output: c_int, flags: usize) -> c_int);
export_tail_shim!(json_dumps(value: *const JsonT, flags: usize) -> *mut c_char);
export_tail_shim!(json_equal(left: *const JsonT, right: *const JsonT) -> c_int);
export_tail_shim!(json_false() -> *mut JsonT);

export_tail_shim!(json_get_alloc_funcs(
    malloc_fn: *mut MallocCallback,
    free_fn: *mut FreeCallback,
));
export_tail_shim!(json_get_alloc_funcs2(
    malloc_fn: *mut MallocCallback,
    realloc_fn: *mut ReallocCallback,
    free_fn: *mut FreeCallback,
));
export_tail_shim!(json_integer(value: i64) -> *mut JsonT);
export_tail_shim!(json_integer_set(integer: *mut JsonT, value: i64) -> c_int);
export_tail_shim!(json_integer_value(integer: *const JsonT) -> i64);

export_tail_shim!(json_load_callback(
    callback: LoadCallback,
    data: *mut c_void,
    flags: usize,
    error: *mut JsonError,
) -> *mut JsonT);
export_tail_shim!(json_load_file(
    path: *const c_char,
    flags: usize,
    error: *mut JsonError,
) -> *mut JsonT);
export_tail_shim!(json_loadb(
    buffer: *const c_char,
    buffer_len: usize,
    flags: usize,
    error: *mut JsonError,
) -> *mut JsonT);
export_tail_shim!(json_loadf(
    input: *mut c_void,
    flags: usize,
    error: *mut JsonError,
) -> *mut JsonT);
export_tail_shim!(json_loadfd(
    input: c_int,
    flags: usize,
    error: *mut JsonError,
) -> *mut JsonT);
export_tail_shim!(json_loads(
    input: *const c_char,
    flags: usize,
    error: *mut JsonError,
) -> *mut JsonT);
export_tail_shim!(json_null() -> *mut JsonT);
export_tail_shim!(json_number_value(value: *const JsonT) -> c_double);

export_tail_shim!(json_object() -> *mut JsonT);
export_tail_shim!(json_object_clear(object: *mut JsonT) -> c_int);
export_tail_shim!(json_object_del(object: *mut JsonT, key: *const c_char) -> c_int);
export_tail_shim!(json_object_deln(
    object: *mut JsonT,
    key: *const c_char,
    key_len: usize,
) -> c_int);
export_tail_shim!(json_object_get(
    object: *const JsonT,
    key: *const c_char,
) -> *mut JsonT);
export_tail_shim!(json_object_getn(
    object: *const JsonT,
    key: *const c_char,
    key_len: usize,
) -> *mut JsonT);
export_tail_shim!(json_object_iter(object: *mut JsonT) -> *mut c_void);
export_tail_shim!(json_object_iter_at(
    object: *mut JsonT,
    key: *const c_char,
) -> *mut c_void);
export_tail_shim!(json_object_iter_key(iter: *mut c_void) -> *const c_char);
export_tail_shim!(json_object_iter_key_len(iter: *mut c_void) -> usize);
export_tail_shim!(json_object_iter_next(
    object: *mut JsonT,
    iter: *mut c_void,
) -> *mut c_void);
export_tail_shim!(json_object_iter_set_new(
    object: *mut JsonT,
    iter: *mut c_void,
    value: *mut JsonT,
) -> c_int);
export_tail_shim!(json_object_iter_value(iter: *mut c_void) -> *mut JsonT);
export_tail_shim!(json_object_key_to_iter(key: *const c_char) -> *mut c_void);
export_tail_shim!(json_object_seed(seed: usize));
export_tail_shim!(json_object_set_new(
    object: *mut JsonT,
    key: *const c_char,
    value: *mut JsonT,
) -> c_int);
export_tail_shim!(json_object_set_new_nocheck(
    object: *mut JsonT,
    key: *const c_char,
    value: *mut JsonT,
) -> c_int);
export_tail_shim!(json_object_setn_new(
    object: *mut JsonT,
    key: *const c_char,
    key_len: usize,
    value: *mut JsonT,
) -> c_int);
export_tail_shim!(json_object_setn_new_nocheck(
    object: *mut JsonT,
    key: *const c_char,
    key_len: usize,
    value: *mut JsonT,
) -> c_int);
export_tail_shim!(json_object_size(object: *const JsonT) -> usize);
export_tail_shim!(json_object_update(object: *mut JsonT, other: *mut JsonT) -> c_int);
export_tail_shim!(json_object_update_existing(
    object: *mut JsonT,
    other: *mut JsonT,
) -> c_int);
export_tail_shim!(json_object_update_missing(
    object: *mut JsonT,
    other: *mut JsonT,
) -> c_int);
export_tail_shim!(json_object_update_recursive(
    object: *mut JsonT,
    other: *mut JsonT,
) -> c_int);

export_tail_shim!(json_real(value: c_double) -> *mut JsonT);
export_tail_shim!(json_real_set(real: *mut JsonT, value: c_double) -> c_int);
export_tail_shim!(json_real_value(real: *const JsonT) -> c_double);
export_tail_shim!(json_set_alloc_funcs(
    malloc_fn: MallocCallback,
    free_fn: FreeCallback,
));
export_tail_shim!(json_set_alloc_funcs2(
    malloc_fn: MallocCallback,
    realloc_fn: ReallocCallback,
    free_fn: FreeCallback,
));

export_tail_shim!(json_string(value: *const c_char) -> *mut JsonT);
export_tail_shim!(json_string_length(string: *const JsonT) -> usize);
export_tail_shim!(json_string_nocheck(value: *const c_char) -> *mut JsonT);
export_tail_shim!(json_string_set(string: *mut JsonT, value: *const c_char) -> c_int);
export_tail_shim!(json_string_set_nocheck(
    string: *mut JsonT,
    value: *const c_char,
) -> c_int);
export_tail_shim!(json_string_setn(
    string: *mut JsonT,
    value: *const c_char,
    length: usize,
) -> c_int);
export_tail_shim!(json_string_setn_nocheck(
    string: *mut JsonT,
    value: *const c_char,
    length: usize,
) -> c_int);
export_tail_shim!(json_string_value(string: *const JsonT) -> *const c_char);
export_tail_shim!(json_stringn(value: *const c_char, length: usize) -> *mut JsonT);
export_tail_shim!(json_stringn_nocheck(
    value: *const c_char,
    length: usize,
) -> *mut JsonT);
export_tail_shim!(json_true() -> *mut JsonT);

export_tail_shim!(json_vpack_ex(
    error: *mut JsonError,
    flags: usize,
    format: *const c_char,
    args: *mut c_void,
) -> *mut JsonT);
export_tail_shim!(json_vsprintf(
    format: *const c_char,
    args: *mut c_void,
) -> *mut JsonT);
export_tail_shim!(json_vunpack_ex(
    root: *mut JsonT,
    error: *mut JsonError,
    flags: usize,
    format: *const c_char,
    args: *mut c_void,
) -> c_int);

export_tail_shim!(jsonp_dtostr(
    buffer: *mut c_char,
    size: usize,
    value: c_double,
    precision: c_int,
) -> c_int);
export_tail_shim!(jsonp_error_init(error: *mut JsonError, source: *const c_char));
export_tail_shim!(jsonp_error_set_source(
    error: *mut JsonError,
    source: *const c_char,
));
export_tail_shim!(jsonp_error_vset(
    error: *mut JsonError,
    line: c_int,
    column: c_int,
    position: usize,
    code: c_int,
    message: *const c_char,
    args: *mut c_void,
));
export_tail_shim!(jsonp_free(pointer: *mut c_void));
export_tail_shim!(jsonp_loop_check(
    parents: *mut Hashtable,
    value: *const JsonT,
    key: *mut c_char,
    key_size: usize,
    key_len: *mut usize,
) -> c_int);
export_tail_shim!(jsonp_malloc(size: usize) -> *mut c_void);
export_tail_shim!(jsonp_realloc(
    pointer: *mut c_void,
    original_size: usize,
    new_size: usize,
) -> *mut c_void);
export_tail_shim!(jsonp_stringn_nocheck_own(
    value: *const c_char,
    length: usize,
) -> *mut JsonT);
export_tail_shim!(jsonp_strndup(value: *const c_char, length: usize) -> *mut c_char);
export_tail_shim!(jsonp_strtod(buffer: *mut StrBuffer, output: *mut c_double) -> c_int);

export_tail_shim!(strbuffer_append_byte(buffer: *mut StrBuffer, byte: c_char) -> c_int);
export_tail_shim!(strbuffer_append_bytes(
    buffer: *mut StrBuffer,
    data: *const c_char,
    size: usize,
) -> c_int);
export_tail_shim!(strbuffer_clear(buffer: *mut StrBuffer));
export_tail_shim!(strbuffer_close(buffer: *mut StrBuffer));
export_tail_shim!(strbuffer_init(buffer: *mut StrBuffer) -> c_int);
export_tail_shim!(strbuffer_pop(buffer: *mut StrBuffer) -> c_char);
export_tail_shim!(strbuffer_steal_value(buffer: *mut StrBuffer) -> *mut c_char);
export_tail_shim!(strbuffer_value(buffer: *const StrBuffer) -> *const c_char);
export_tail_shim!(strtod__unused(
    input: *const c_char,
    end: *mut *mut c_char,
) -> c_double);

export_tail_shim!(utf8_check_first(byte: c_char) -> usize);
export_tail_shim!(utf8_check_full(
    buffer: *const c_char,
    size: usize,
    codepoint: *mut i32,
) -> usize);
export_tail_shim!(utf8_check_string(buffer: *const c_char, length: usize) -> c_int);
export_tail_shim!(utf8_encode(
    codepoint: i32,
    buffer: *mut c_char,
    size: *mut usize,
) -> c_int);
export_tail_shim!(utf8_iterate(
    buffer: *const c_char,
    size: usize,
    codepoint: *mut i32,
) -> *const c_char);

export_variadic_tail_shims!(
    json_pack,
    json_pack_ex,
    json_sprintf,
    json_unpack,
    json_unpack_ex,
    jsonp_error_set,
);
