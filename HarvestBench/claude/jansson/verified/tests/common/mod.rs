//! Shared harness for differential tests: loads BOTH the C .so and the Rust .so
//! via libloading and exposes symbols by name. Never calls Rust functions
//! directly.
#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::os::raw::{c_char, c_double, c_void};
use std::sync::OnceLock;

pub const C_SO: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libjansson.so");
pub const R_SO: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/target/debug/libjansson.so");

// json_int_t is long long (JSON_INTEGER_IS_LONG_LONG=1)
pub type JsonInt = i64;

pub struct Libs {
    pub c: Library,
    pub r: Library,
}

static LIBS: OnceLock<Libs> = OnceLock::new();

pub fn libs() -> &'static Libs {
    LIBS.get_or_init(|| unsafe {
        let c = Library::new(C_SO).expect("load C .so");
        let r = Library::new(R_SO).expect("load Rust .so");
        Libs { c, r }
    })
}

/// Get a symbol from a library, panicking with the name on failure.
pub unsafe fn sym<'a, T>(lib: &'a Library, name: &[u8]) -> Symbol<'a, T> {
    lib.get(name)
        .unwrap_or_else(|e| panic!("missing symbol {}: {e}", String::from_utf8_lossy(name)))
}

// ---- Common FFI signatures used across many tests ----
pub type FnLoads =
    unsafe extern "C" fn(*const c_char, usize, *mut c_void) -> *mut c_void;
pub type FnLoadb =
    unsafe extern "C" fn(*const c_char, usize, usize, *mut c_void) -> *mut c_void;
pub type FnDumps = unsafe extern "C" fn(*const c_void, usize) -> *mut c_char;
pub type FnDumpb =
    unsafe extern "C" fn(*const c_void, *mut c_char, usize, usize) -> usize;
pub type FnDelete = unsafe extern "C" fn(*mut c_void);
pub type FnPtrToInt = unsafe extern "C" fn(*const c_void) -> JsonInt;
pub type FnPtrToDouble = unsafe extern "C" fn(*const c_void) -> c_double;
pub type FnPtrToSize = unsafe extern "C" fn(*const c_void) -> usize;
pub type FnPtrToPtr = unsafe extern "C" fn(*const c_void) -> *mut c_void;
pub type FnFreeStr = unsafe extern "C" fn(*mut c_char);

/// Read a C string result (owned by lib) into a Rust Vec<u8>, using that lib's
/// jansson_free-equivalent path. Jansson dumps use jsonp_malloc; freeing must
/// go through the SAME allocator. We just copy bytes and leak (tests are short)
/// OR free via that lib's `free` — but jansson uses libc malloc by default so
/// libc free is fine. We copy and then call libc free.
pub unsafe fn cstr_to_vec(p: *const c_char) -> Option<Vec<u8>> {
    if p.is_null() {
        return None;
    }
    let mut len = 0usize;
    while *p.add(len) != 0 {
        len += 1;
    }
    Some(std::slice::from_raw_parts(p as *const u8, len).to_vec())
}

extern "C" {
    fn free(p: *mut c_void);
}

pub unsafe fn libc_free(p: *mut c_void) {
    free(p);
}

/// Given both libs, load `json_loads` + `json_dumps` + `json_delete` from each,
/// parse `input` with `flags_load`, dump with `flags_dump`, return the dumped
/// bytes (or None on NULL). Frees everything. Uses the given lib.
pub unsafe fn roundtrip(lib: &Library, input: &[u8], flags_load: usize, flags_dump: usize) -> Option<Vec<u8>> {
    let loads: Symbol<FnLoads> = sym(lib, b"json_loads");
    let dumps: Symbol<FnDumps> = sym(lib, b"json_dumps");
    let delete: Symbol<FnDelete> = sym(lib, b"json_delete");

    let mut cinput = input.to_vec();
    cinput.push(0);
    let v = loads(cinput.as_ptr() as *const c_char, flags_load, std::ptr::null_mut());
    if v.is_null() {
        return None;
    }
    let s = dumps(v, flags_dump);
    let out = cstr_to_vec(s);
    if !s.is_null() {
        libc_free(s as *mut c_void);
    }
    delete(v);
    out
}
