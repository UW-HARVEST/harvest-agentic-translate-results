// Common helpers for integration tests that compare C and Rust .so outputs
// through the FFI boundary.

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_uint, c_void};
use std::path::PathBuf;

#[repr(C)]
#[derive(Debug)]
pub struct AlertData {
    pub rule: c_uint,
    pub level: c_uint,
    pub alertid: *mut c_char,
    pub date: *mut c_char,
    pub location: *mut c_char,
    pub comment: *mut c_char,
    pub group: *mut c_char,
    pub srcip: *mut c_char,
    pub srcport: c_int,
    pub dstip: *mut c_char,
    pub dstport: c_int,
    pub user: *mut c_char,
    pub filename: *mut c_char,
}

// Public function-pointer types
pub type FnGetAlertData = unsafe extern "C" fn(c_int, *mut libc::FILE) -> *mut AlertData;
pub type FnFreeAlertData = unsafe extern "C" fn(*mut AlertData);
pub type FnInitFileQueue = unsafe extern "C" fn(*mut c_void, *const libc::tm, c_int) -> c_int;
pub type FnReadFileMon =
    unsafe extern "C" fn(*mut c_void, *const libc::tm, c_uint) -> *mut AlertData;
pub type FnDriver =
    unsafe extern "C" fn(c_int, c_int, c_int, c_uint, c_int) -> *mut AlertData;
pub type FnOsCalloc = unsafe extern "C" fn(libc::size_t, libc::size_t) -> *mut c_void;
pub type FnOsRealloc = unsafe extern "C" fn(*mut c_void, libc::size_t) -> *mut c_void;
pub type FnOsStrdup = unsafe extern "C" fn(*const c_char) -> *mut c_char;
pub type FnMerror =
    unsafe extern "C" fn(*const c_char, *const c_char, c_int, *const c_char);

pub fn c_so_path() -> PathBuf {
    let manifest = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest).join("c_src/build/libdriver.so")
}

pub fn rust_so_path() -> PathBuf {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let exe = std::env::current_exe().unwrap();
    let mut dir = exe.parent().unwrap().to_path_buf();
    // current_exe is target/debug/deps/<test>; rust .so is target/debug/libdriver.so
    while dir.file_name().map(|s| s == "deps").unwrap_or(false) {
        dir.pop();
    }
    let candidate = dir.join("libdriver.so");
    if candidate.exists() {
        return candidate;
    }
    PathBuf::from(manifest).join("target/debug/libdriver.so")
}

pub fn load_c() -> Library {
    let path = c_so_path();
    unsafe { Library::new(&path).expect("failed to load C .so") }
}

pub fn load_rust() -> Library {
    let path = rust_so_path();
    unsafe { Library::new(&path).expect("failed to load Rust .so") }
}

pub unsafe fn sym<'lib, T>(lib: &'lib Library, name: &[u8]) -> Symbol<'lib, T> {
    unsafe { lib.get(name).expect("symbol not found") }
}

/// Create an in-memory FILE* containing the supplied bytes using fmemopen.
pub unsafe fn fmemopen_ro(bytes: &[u8]) -> *mut libc::FILE {
    unsafe {
        // fmemopen requires the buffer to live as long as the FILE; we leak it.
        let buf = libc::malloc(bytes.len()) as *mut u8;
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), buf, bytes.len());
        let mode = b"r\0".as_ptr() as *const c_char;
        libc::fmemopen(buf as *mut c_void, bytes.len(), mode)
    }
}

/// Read a NUL-terminated string from a (possibly null) C pointer.
pub unsafe fn cstr_to_string(p: *const c_char) -> Option<String> {
    if p.is_null() {
        return None;
    }
    unsafe {
        let len = libc::strlen(p);
        let slice = std::slice::from_raw_parts(p as *const u8, len);
        Some(String::from_utf8_lossy(slice).into_owned())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct AlertDataSnapshot {
    pub rule: c_uint,
    pub level: c_uint,
    pub alertid: Option<String>,
    pub date: Option<String>,
    pub location: Option<String>,
    pub comment: Option<String>,
    pub group: Option<String>,
    pub srcip: Option<String>,
    pub srcport: c_int,
    pub dstip: Option<String>,
    pub dstport: c_int,
    pub user: Option<String>,
    pub filename: Option<String>,
}

pub unsafe fn snapshot_alert(p: *const AlertData) -> Option<AlertDataSnapshot> {
    if p.is_null() {
        return None;
    }
    unsafe {
        let a = &*p;
        Some(AlertDataSnapshot {
            rule: a.rule,
            level: a.level,
            alertid: cstr_to_string(a.alertid),
            date: cstr_to_string(a.date),
            location: cstr_to_string(a.location),
            comment: cstr_to_string(a.comment),
            group: cstr_to_string(a.group),
            srcip: cstr_to_string(a.srcip),
            srcport: a.srcport,
            dstip: cstr_to_string(a.dstip),
            dstport: a.dstport,
            user: cstr_to_string(a.user),
            filename: cstr_to_string(a.filename),
        })
    }
}
