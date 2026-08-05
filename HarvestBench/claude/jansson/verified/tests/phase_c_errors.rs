//! Phase C — error-path differential tests (ERRORS.md rows 1-97).
//! Each test constructs the exact invalid input and asserts C and Rust return
//! the SAME sentinel / error code.
mod common;

use common::*;
use std::os::raw::{c_char, c_double, c_int, c_void};

const JSON_REJECT_DUPLICATES: usize = 0x1;

// --- json_error_t layout (jansson.h) ---
// int line; int column; int position; char source[80]; char text[160];
#[repr(C)]
struct JsonError {
    line: c_int,
    column: c_int,
    position: c_int,
    source: [u8; 80],
    text: [u8; 160],
}
impl JsonError {
    fn zeroed() -> Self {
        // Safety: all-zero is a valid value for this POD.
        unsafe { std::mem::zeroed() }
    }
    /// json_error_code = (enum)text[159]
    fn code(&self) -> i8 {
        self.text[159] as i8
    }
}

type FnConstruct1Ptr = unsafe extern "C" fn(*const c_char) -> *mut c_void;
type FnConstruct1PtrLen = unsafe extern "C" fn(*const c_char, usize) -> *mut c_void;
type FnReal = unsafe extern "C" fn(c_double) -> *mut c_void;
type FnObject = unsafe extern "C" fn() -> *mut c_void;
type FnArray = unsafe extern "C" fn() -> *mut c_void;
type FnInteger = unsafe extern "C" fn(JsonInt) -> *mut c_void;
type FnRetInt = unsafe extern "C" fn(*mut c_void) -> c_int;
type FnLoadsErr =
    unsafe extern "C" fn(*const c_char, usize, *mut JsonError) -> *mut c_void;

/// Run a closure that returns some comparable value under both libs and assert equality.
fn both<T: PartialEq + std::fmt::Debug, F: Fn(&libloading::Library) -> T>(label: &str, f: F) {
    let l = libs();
    let c = f(&l.c);
    let r = f(&l.r);
    assert_eq!(c, r, "[{label}] C={c:?} Rust={r:?}");
}

// ---------- value.c rows 1-57 ----------

#[test]
fn value_null_and_type_rejections() {
    // Rows: string/real/getters returning NULL/0/-1 on bad input.
    both("json_string(NULL)", |lib| unsafe {
        let f: libloading::Symbol<FnConstruct1Ptr> = sym(lib, b"json_string");
        f(std::ptr::null()).is_null()
    });
    both("json_string_nocheck(NULL)", |lib| unsafe {
        let f: libloading::Symbol<FnConstruct1Ptr> = sym(lib, b"json_string_nocheck");
        f(std::ptr::null()).is_null()
    });
    both("json_stringn_nocheck(NULL)", |lib| unsafe {
        let f: libloading::Symbol<FnConstruct1PtrLen> = sym(lib, b"json_stringn_nocheck");
        f(std::ptr::null(), 5).is_null()
    });
    // invalid UTF-8: 0xFF byte
    both("json_string(bad utf8)", |lib| unsafe {
        let f: libloading::Symbol<FnConstruct1Ptr> = sym(lib, b"json_string");
        f(b"\xff\xfe\0".as_ptr() as *const c_char).is_null()
    });
    both("json_stringn(bad utf8)", |lib| unsafe {
        let f: libloading::Symbol<FnConstruct1PtrLen> = sym(lib, b"json_stringn");
        f(b"\xff\xfe".as_ptr() as *const c_char, 2).is_null()
    });
    // json_real NaN/Inf
    both("json_real(NaN)", |lib| unsafe {
        let f: libloading::Symbol<FnReal> = sym(lib, b"json_real");
        f(f64::NAN).is_null()
    });
    both("json_real(Inf)", |lib| unsafe {
        let f: libloading::Symbol<FnReal> = sym(lib, b"json_real");
        f(f64::INFINITY).is_null()
    });
    both("json_real(-Inf)", |lib| unsafe {
        let f: libloading::Symbol<FnReal> = sym(lib, b"json_real");
        f(f64::NEG_INFINITY).is_null()
    });
}

#[test]
fn getters_wrong_type() {
    // Getters on wrong type: json_integer_value(real)=0, json_string_value(int)=NULL, etc.
    both("integer_value(real)", |lib| unsafe {
        let real: libloading::Symbol<FnReal> = sym(lib, b"json_real");
        let get: libloading::Symbol<FnPtrToInt> = sym(lib, b"json_integer_value");
        let del: libloading::Symbol<FnDelete> = sym(lib, b"json_delete");
        let v = real(1.5);
        let g = get(v);
        del(v);
        g
    });
    both("real_value(int)", |lib| unsafe {
        let int: libloading::Symbol<FnInteger> = sym(lib, b"json_integer");
        let get: libloading::Symbol<FnPtrToDouble> = sym(lib, b"json_real_value");
        let del: libloading::Symbol<FnDelete> = sym(lib, b"json_delete");
        let v = int(3);
        let g = get(v).to_bits();
        del(v);
        g
    });
    both("string_value(int)", |lib| unsafe {
        let int: libloading::Symbol<FnInteger> = sym(lib, b"json_integer");
        let get: libloading::Symbol<unsafe extern "C" fn(*const c_void) -> *const c_char> =
            sym(lib, b"json_string_value");
        let del: libloading::Symbol<FnDelete> = sym(lib, b"json_delete");
        let v = int(3);
        let g = get(v).is_null();
        del(v);
        g
    });
    both("string_length(int)", |lib| unsafe {
        let int: libloading::Symbol<FnInteger> = sym(lib, b"json_integer");
        let get: libloading::Symbol<FnPtrToSize> = sym(lib, b"json_string_length");
        let del: libloading::Symbol<FnDelete> = sym(lib, b"json_delete");
        let v = int(3);
        let g = get(v);
        del(v);
        g
    });
    // integer_set / real_set on wrong type
    both("integer_set(real)", |lib| unsafe {
        let real: libloading::Symbol<FnReal> = sym(lib, b"json_real");
        let set: libloading::Symbol<unsafe extern "C" fn(*mut c_void, JsonInt) -> c_int> =
            sym(lib, b"json_integer_set");
        let del: libloading::Symbol<FnDelete> = sym(lib, b"json_delete");
        let v = real(1.5);
        let g = set(v, 5);
        del(v);
        g
    });
    both("real_set(int)", |lib| unsafe {
        let int: libloading::Symbol<FnInteger> = sym(lib, b"json_integer");
        let set: libloading::Symbol<unsafe extern "C" fn(*mut c_void, c_double) -> c_int> =
            sym(lib, b"json_real_set");
        let del: libloading::Symbol<FnDelete> = sym(lib, b"json_delete");
        let v = int(3);
        let g = set(v, 2.5);
        del(v);
        g
    });
    both("real_set(NaN)", |lib| unsafe {
        let real: libloading::Symbol<FnReal> = sym(lib, b"json_real");
        let set: libloading::Symbol<unsafe extern "C" fn(*mut c_void, c_double) -> c_int> =
            sym(lib, b"json_real_set");
        let del: libloading::Symbol<FnDelete> = sym(lib, b"json_delete");
        let v = real(1.0);
        let g = set(v, f64::NAN);
        del(v);
        g
    });
}

#[test]
fn object_null_key_and_type() {
    // json_object_get(NULL key), on non-object, del missing key, etc.
    both("object_get(int, NULL key)", |lib| unsafe {
        let int: libloading::Symbol<FnInteger> = sym(lib, b"json_integer");
        let get: libloading::Symbol<unsafe extern "C" fn(*const c_void, *const c_char) -> *mut c_void> =
            sym(lib, b"json_object_get");
        let del: libloading::Symbol<FnDelete> = sym(lib, b"json_delete");
        let v = int(1);
        let g = get(v, std::ptr::null());
        let r = g.is_null();
        del(v);
        r
    });
    both("object_get(int, key)", |lib| unsafe {
        let int: libloading::Symbol<FnInteger> = sym(lib, b"json_integer");
        let get: libloading::Symbol<unsafe extern "C" fn(*const c_void, *const c_char) -> *mut c_void> =
            sym(lib, b"json_object_get");
        let del: libloading::Symbol<FnDelete> = sym(lib, b"json_delete");
        let v = int(1);
        let g = get(v, b"x\0".as_ptr() as *const c_char).is_null();
        del(v);
        g
    });
    both("object_del missing", |lib| unsafe {
        let obj: libloading::Symbol<FnObject> = sym(lib, b"json_object");
        let del_key: libloading::Symbol<unsafe extern "C" fn(*mut c_void, *const c_char) -> c_int> =
            sym(lib, b"json_object_del");
        let del: libloading::Symbol<FnDelete> = sym(lib, b"json_delete");
        let v = obj();
        let g = del_key(v, b"nope\0".as_ptr() as *const c_char);
        del(v);
        g
    });
    both("object_del(NULL key)", |lib| unsafe {
        let obj: libloading::Symbol<FnObject> = sym(lib, b"json_object");
        let del_key: libloading::Symbol<unsafe extern "C" fn(*mut c_void, *const c_char) -> c_int> =
            sym(lib, b"json_object_del");
        let del: libloading::Symbol<FnDelete> = sym(lib, b"json_delete");
        let v = obj();
        let g = del_key(v, std::ptr::null());
        del(v);
        g
    });
    both("object_clear(array)", |lib| unsafe {
        let arr: libloading::Symbol<FnArray> = sym(lib, b"json_array");
        let clear: libloading::Symbol<FnRetInt> = sym(lib, b"json_object_clear");
        let del: libloading::Symbol<FnDelete> = sym(lib, b"json_delete");
        let v = arr();
        let g = clear(v);
        del(v);
        g
    });
    both("object_size(array)", |lib| unsafe {
        let arr: libloading::Symbol<FnArray> = sym(lib, b"json_array");
        let size: libloading::Symbol<FnPtrToSize> = sym(lib, b"json_object_size");
        let del: libloading::Symbol<FnDelete> = sym(lib, b"json_delete");
        let v = arr();
        let g = size(v);
        del(v);
        g
    });
    // set_new with NULL key: returns -1, and value is decref'd (using an int value)
    both("object_set_new(NULL key)", |lib| unsafe {
        let obj: libloading::Symbol<FnObject> = sym(lib, b"json_object");
        let int: libloading::Symbol<FnInteger> = sym(lib, b"json_integer");
        let set_new: libloading::Symbol<unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_void) -> c_int> =
            sym(lib, b"json_object_set_new");
        let del: libloading::Symbol<FnDelete> = sym(lib, b"json_delete");
        let o = obj();
        let g = set_new(o, std::ptr::null(), int(5));
        del(o);
        g
    });
    // set_new with invalid utf8 key
    both("object_set_new(bad utf8 key)", |lib| unsafe {
        let obj: libloading::Symbol<FnObject> = sym(lib, b"json_object");
        let int: libloading::Symbol<FnInteger> = sym(lib, b"json_integer");
        let set_new: libloading::Symbol<unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_void) -> c_int> =
            sym(lib, b"json_object_set_new");
        let del: libloading::Symbol<FnDelete> = sym(lib, b"json_delete");
        let o = obj();
        let g = set_new(o, b"\xff\0".as_ptr() as *const c_char, int(5));
        del(o);
        g
    });
    // update with non-object other
    both("object_update(obj, array)", |lib| unsafe {
        let obj: libloading::Symbol<FnObject> = sym(lib, b"json_object");
        let arr: libloading::Symbol<FnArray> = sym(lib, b"json_array");
        let upd: libloading::Symbol<unsafe extern "C" fn(*mut c_void, *mut c_void) -> c_int> =
            sym(lib, b"json_object_update");
        let del: libloading::Symbol<FnDelete> = sym(lib, b"json_delete");
        let o = obj();
        let a = arr();
        let g = upd(o, a);
        del(o);
        del(a);
        g
    });
    // iter on non-object
    both("object_iter(int)", |lib| unsafe {
        let int: libloading::Symbol<FnInteger> = sym(lib, b"json_integer");
        let iter: libloading::Symbol<unsafe extern "C" fn(*mut c_void) -> *mut c_void> =
            sym(lib, b"json_object_iter");
        let del: libloading::Symbol<FnDelete> = sym(lib, b"json_delete");
        let v = int(1);
        let g = iter(v).is_null();
        del(v);
        g
    });
    // iter_key / iter_value / iter_key_len on NULL
    both("iter_key(NULL)", |lib| unsafe {
        let f: libloading::Symbol<unsafe extern "C" fn(*mut c_void) -> *const c_char> =
            sym(lib, b"json_object_iter_key");
        f(std::ptr::null_mut()).is_null()
    });
    both("iter_value(NULL)", |lib| unsafe {
        let f: libloading::Symbol<unsafe extern "C" fn(*mut c_void) -> *mut c_void> =
            sym(lib, b"json_object_iter_value");
        f(std::ptr::null_mut()).is_null()
    });
    both("iter_key_len(NULL)", |lib| unsafe {
        let f: libloading::Symbol<FnPtrToSize> = sym(lib, b"json_object_iter_key_len");
        f(std::ptr::null_mut())
    });
    both("key_to_iter(NULL)", |lib| unsafe {
        let f: libloading::Symbol<unsafe extern "C" fn(*const c_char) -> *mut c_void> =
            sym(lib, b"json_object_key_to_iter");
        f(std::ptr::null()).is_null()
    });
}

#[test]
fn array_index_and_type() {
    // json_array_get out of range, set/remove out of range, on non-array, extend.
    both("array_get oob", |lib| unsafe {
        let arr: libloading::Symbol<FnArray> = sym(lib, b"json_array");
        let get: libloading::Symbol<unsafe extern "C" fn(*const c_void, usize) -> *mut c_void> =
            sym(lib, b"json_array_get");
        let del: libloading::Symbol<FnDelete> = sym(lib, b"json_delete");
        let v = arr();
        let g = get(v, 100).is_null();
        del(v);
        g
    });
    both("array_get(int)", |lib| unsafe {
        let int: libloading::Symbol<FnInteger> = sym(lib, b"json_integer");
        let get: libloading::Symbol<unsafe extern "C" fn(*const c_void, usize) -> *mut c_void> =
            sym(lib, b"json_array_get");
        let del: libloading::Symbol<FnDelete> = sym(lib, b"json_delete");
        let v = int(1);
        let g = get(v, 0).is_null();
        del(v);
        g
    });
    both("array_set oob", |lib| unsafe {
        let arr: libloading::Symbol<FnArray> = sym(lib, b"json_array");
        let int: libloading::Symbol<FnInteger> = sym(lib, b"json_integer");
        let set: libloading::Symbol<unsafe extern "C" fn(*mut c_void, usize, *mut c_void) -> c_int> =
            sym(lib, b"json_array_set_new");
        let del: libloading::Symbol<FnDelete> = sym(lib, b"json_delete");
        let v = arr();
        let g = set(v, 5, int(1));
        del(v);
        g
    });
    both("array_insert oob", |lib| unsafe {
        let arr: libloading::Symbol<FnArray> = sym(lib, b"json_array");
        let int: libloading::Symbol<FnInteger> = sym(lib, b"json_integer");
        let ins: libloading::Symbol<unsafe extern "C" fn(*mut c_void, usize, *mut c_void) -> c_int> =
            sym(lib, b"json_array_insert_new");
        let del: libloading::Symbol<FnDelete> = sym(lib, b"json_delete");
        let v = arr();
        let g = ins(v, 5, int(1));
        del(v);
        g
    });
    both("array_remove oob", |lib| unsafe {
        let arr: libloading::Symbol<FnArray> = sym(lib, b"json_array");
        let rm: libloading::Symbol<unsafe extern "C" fn(*mut c_void, usize) -> c_int> =
            sym(lib, b"json_array_remove");
        let del: libloading::Symbol<FnDelete> = sym(lib, b"json_delete");
        let v = arr();
        let g = rm(v, 0);
        del(v);
        g
    });
    both("array_clear(int)", |lib| unsafe {
        let int: libloading::Symbol<FnInteger> = sym(lib, b"json_integer");
        let clr: libloading::Symbol<FnRetInt> = sym(lib, b"json_array_clear");
        let del: libloading::Symbol<FnDelete> = sym(lib, b"json_delete");
        let v = int(1);
        let g = clr(v);
        del(v);
        g
    });
    both("array_extend(arr, obj)", |lib| unsafe {
        let arr: libloading::Symbol<FnArray> = sym(lib, b"json_array");
        let obj: libloading::Symbol<FnObject> = sym(lib, b"json_object");
        let ext: libloading::Symbol<unsafe extern "C" fn(*mut c_void, *mut c_void) -> c_int> =
            sym(lib, b"json_array_extend");
        let del: libloading::Symbol<FnDelete> = sym(lib, b"json_delete");
        let a = arr();
        let o = obj();
        let g = ext(a, o);
        del(a);
        del(o);
        g
    });
    // set/append/insert with NULL value
    both("array_append_new(NULL val)", |lib| unsafe {
        let arr: libloading::Symbol<FnArray> = sym(lib, b"json_array");
        let ap: libloading::Symbol<unsafe extern "C" fn(*mut c_void, *mut c_void) -> c_int> =
            sym(lib, b"json_array_append_new");
        let del: libloading::Symbol<FnDelete> = sym(lib, b"json_delete");
        let a = arr();
        let g = ap(a, std::ptr::null_mut());
        del(a);
        g
    });
}

#[test]
fn equal_and_copy_null() {
    both("equal(NULL,NULL)", |lib| unsafe {
        let eq: libloading::Symbol<unsafe extern "C" fn(*const c_void, *const c_void) -> c_int> =
            sym(lib, b"json_equal");
        eq(std::ptr::null(), std::ptr::null())
    });
    both("equal(int,NULL)", |lib| unsafe {
        let int: libloading::Symbol<FnInteger> = sym(lib, b"json_integer");
        let eq: libloading::Symbol<unsafe extern "C" fn(*const c_void, *const c_void) -> c_int> =
            sym(lib, b"json_equal");
        let del: libloading::Symbol<FnDelete> = sym(lib, b"json_delete");
        let v = int(1);
        let g = eq(v, std::ptr::null());
        del(v);
        g
    });
    both("equal(int,real) diff type", |lib| unsafe {
        let int: libloading::Symbol<FnInteger> = sym(lib, b"json_integer");
        let real: libloading::Symbol<FnReal> = sym(lib, b"json_real");
        let eq: libloading::Symbol<unsafe extern "C" fn(*const c_void, *const c_void) -> c_int> =
            sym(lib, b"json_equal");
        let del: libloading::Symbol<FnDelete> = sym(lib, b"json_delete");
        let a = int(1);
        let b = real(1.0);
        let g = eq(a, b);
        del(a);
        del(b);
        g
    });
    both("copy(NULL)", |lib| unsafe {
        let f: libloading::Symbol<unsafe extern "C" fn(*mut c_void) -> *mut c_void> =
            sym(lib, b"json_copy");
        f(std::ptr::null_mut()).is_null()
    });
    both("deep_copy(NULL)", |lib| unsafe {
        let f: libloading::Symbol<unsafe extern "C" fn(*const c_void) -> *mut c_void> =
            sym(lib, b"json_deep_copy");
        f(std::ptr::null()).is_null()
    });
}

// ---------- load.c rows 58-83: parser errors w/ error code ----------

/// Load `input` and return (is_null, error_code, line, column, position, text, source).
fn load_err(
    lib: &libloading::Library,
    input: &[u8],
    flags: usize,
) -> (bool, i8, c_int, c_int, c_int, Vec<u8>, Vec<u8>) {
    unsafe {
        let loads: libloading::Symbol<FnLoadsErr> = sym(lib, b"json_loads");
        let del: libloading::Symbol<FnDelete> = sym(lib, b"json_delete");
        let mut err = JsonError::zeroed();
        let mut cin = input.to_vec();
        cin.push(0);
        let v = loads(cin.as_ptr() as *const c_char, flags, &mut err);
        let isnull = v.is_null();
        if !isnull {
            del(v);
        }
        // text[159] holds the error code; compare text bytes up to first NUL.
        let text_end = err.text[..159].iter().position(|&b| b == 0).unwrap_or(159);
        let src_end = err.source.iter().position(|&b| b == 0).unwrap_or(80);
        (
            isnull,
            err.code(),
            err.line,
            err.column,
            err.position,
            err.text[..text_end].to_vec(),
            err.source[..src_end].to_vec(),
        )
    }
}

fn assert_load_err(label: &str, input: &[u8], flags: usize) {
    let l = libs();
    let c = load_err(&l.c, input, flags);
    let r = load_err(&l.r, input, flags);
    assert_eq!(c, r, "[{label}] input={:?}\nC={c:?} Rust={r:?}", String::from_utf8_lossy(input));
}

#[test]
fn parser_errors() {
    // Row 58: NULL input
    both("loads(NULL)", |lib| unsafe {
        let loads: libloading::Symbol<FnLoadsErr> = sym(lib, b"json_loads");
        let mut err = JsonError::zeroed();
        let v = loads(std::ptr::null(), 0, &mut err);
        (v.is_null(), err.code())
    });
    // Various parser rejections with matching error codes:
    assert_load_err("empty", b"", 0); // premature_end_of_input
    assert_load_err("bare scalar no ANY", b"42", 0); // '[' or '{' expected
    assert_load_err("invalid utf8", b"[\"\xff\"]", 0);
    assert_load_err("newline in string", b"[\"a\nb\"]", 0);
    assert_load_err("control char", b"[\"a\x01b\"]", 0);
    assert_load_err("invalid escape", b"[\"a\\qb\"]", 0);
    assert_load_err("bad unicode", b"[\"\\u12zz\"]", 0);
    assert_load_err("int too big", b"999999999999999999999999999999", 0x4);
    assert_load_err("real overflow", b"1e400", 0x4);
    assert_load_err("string or } expected", b"{,}", 0);
    assert_load_err("colon expected", b"{\"a\" 1}", 0);
    assert_load_err("} expected", b"{\"a\":1 \"b\":2}", 0);
    assert_load_err("] expected", b"[1 2]", 0);
    assert_load_err("invalid token", b"[tru]", 0);
    assert_load_err("unexpected token", b"[}]", 0);
    assert_load_err("trailing garbage", b"[1] xyz", 0);
    assert_load_err("duplicate key reject", b"{\"a\":1,\"a\":2}", JSON_REJECT_DUPLICATES);
    assert_load_err("nul in key", b"{\"a\x00b\":1}", 0x10); // key with NUL even with ALLOW_NUL
    assert_load_err("nul char in string", b"[\"a\x00b\"]", 0); // without ALLOW_NUL
    // deep nesting past max depth (2048) — build 3000 '['
    let deep: Vec<u8> = std::iter::repeat(b'[').take(3000).collect();
    assert_load_err("stack overflow", &deep, 0);
}

// ---------- dump.c rows 84-88 ----------

#[test]
fn dump_errors() {
    // dumps of a bare scalar without ENCODE_ANY → NULL
    both("dumps(int) no ANY", |lib| unsafe {
        let int: libloading::Symbol<FnInteger> = sym(lib, b"json_integer");
        let dumps: libloading::Symbol<FnDumps> = sym(lib, b"json_dumps");
        let del: libloading::Symbol<FnDelete> = sym(lib, b"json_delete");
        let v = int(5);
        let s = dumps(v, 0);
        let isnull = s.is_null();
        if !isnull {
            libc_free(s as *mut c_void);
        }
        del(v);
        isnull
    });
    // dumps(NULL) → NULL
    both("dumps(NULL)", |lib| unsafe {
        let dumps: libloading::Symbol<FnDumps> = sym(lib, b"json_dumps");
        dumps(std::ptr::null(), 0).is_null()
    });
    // dumpb(int) no ANY → 0
    both("dumpb(int) no ANY", |lib| unsafe {
        let int: libloading::Symbol<FnInteger> = sym(lib, b"json_integer");
        let dumpb: libloading::Symbol<FnDumpb> = sym(lib, b"json_dumpb");
        let del: libloading::Symbol<FnDelete> = sym(lib, b"json_delete");
        let v = int(5);
        let mut buf = [0u8; 32];
        let n = dumpb(v, buf.as_mut_ptr() as *mut c_char, 32, 0);
        del(v);
        n
    });
}

// ---------- pack_unpack.c rows 89-94 ----------

#[test]
fn pack_unpack_errors() {
    type FnPack = unsafe extern "C" fn(*const c_char, ...) -> *mut c_void;
    type FnUnpack = unsafe extern "C" fn(*mut c_void, *const c_char, ...) -> c_int;

    // json_pack(NULL fmt) → NULL
    both("pack(NULL fmt)", |lib| unsafe {
        let pack: libloading::Symbol<FnPack> = sym(lib, b"json_pack");
        pack(std::ptr::null()).is_null()
    });
    // json_pack("") empty fmt → NULL
    both("pack(empty)", |lib| unsafe {
        let pack: libloading::Symbol<FnPack> = sym(lib, b"json_pack");
        pack(b"\0".as_ptr() as *const c_char).is_null()
    });
    // json_unpack(NULL root) → -1
    both("unpack(NULL root)", |lib| unsafe {
        let unpack: libloading::Symbol<FnUnpack> = sym(lib, b"json_unpack");
        unpack(std::ptr::null_mut(), b"i\0".as_ptr() as *const c_char, std::ptr::null_mut::<c_int>())
    });
}

// ---------- utf.c rows 95-97 ----------

#[test]
fn utf8_errors() {
    type FnEncode = unsafe extern "C" fn(i32, *mut c_char, *mut usize) -> c_int;
    both("utf8_encode oob", |lib| unsafe {
        let enc: libloading::Symbol<FnEncode> = sym(lib, b"utf8_encode");
        let mut buf = [0u8; 8];
        let mut sz = 0usize;
        let r = enc(0x110000, buf.as_mut_ptr() as *mut c_char, &mut sz);
        (r, sz)
    });
    both("utf8_encode negative", |lib| unsafe {
        let enc: libloading::Symbol<FnEncode> = sym(lib, b"utf8_encode");
        let mut buf = [0u8; 8];
        let mut sz = 0usize;
        let r = enc(-1, buf.as_mut_ptr() as *mut c_char, &mut sz);
        (r, sz)
    });
    both("utf8_check_string(bad)", |lib| unsafe {
        let f: libloading::Symbol<unsafe extern "C" fn(*const c_char, usize) -> c_int> =
            sym(lib, b"utf8_check_string");
        f(b"\xff\xfe".as_ptr() as *const c_char, 2)
    });
}
