// Common loader for the C and Rust shared libraries.
// We dlopen both and load matching symbols, then compare results byte-for-byte.

use libloading::{Library, Symbol};
use std::os::raw::{c_char, c_int};
use std::path::PathBuf;

#[repr(C)]
#[derive(Debug)]
pub struct IntVec {
    pub data: *mut c_int,
    pub len: usize,
    pub cap: usize,
}

impl Default for IntVec {
    fn default() -> Self {
        IntVec {
            data: std::ptr::null_mut(),
            len: 0,
            cap: 0,
        }
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct Program {
    pub code: *const c_int,
    pub n: usize,
    pub ip: usize,
}

impl Default for Program {
    fn default() -> Self {
        Program {
            code: std::ptr::null(),
            n: 0,
            ip: 0,
        }
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct VM {
    pub stack: IntVec,
    pub trace: IntVec,
    pub steps: c_int,
}

impl Default for VM {
    fn default() -> Self {
        VM {
            stack: IntVec::default(),
            trace: IntVec::default(),
            steps: 0,
        }
    }
}

pub struct Lib {
    lib: Library,
}

impl Lib {
    pub unsafe fn open(path: &str) -> Self {
        let lib = Library::new(path).unwrap_or_else(|e| panic!("dlopen {}: {}", path, e));
        Lib { lib }
    }

    pub unsafe fn sym<T: Copy>(&self, name: &[u8]) -> Symbol<T> {
        self.lib
            .get(name)
            .unwrap_or_else(|e| panic!("missing sym {}: {}", String::from_utf8_lossy(name), e))
    }
}

pub fn c_so_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libcdriver.so")
}

pub fn rust_so_path() -> PathBuf {
    // Use the path of the test binary's parent's parent's .../debug/libdriver.so.
    // But during `cargo test`, OUT_DIR/CARGO_TARGET_TMPDIR is unreliable.
    // Use CARGO_MANIFEST_DIR + target/debug.
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/libdriver.so");
    if p.exists() {
        return p;
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/libdriver.so")
}

pub unsafe fn open_libs() -> (Lib, Lib) {
    let c_path = c_so_path();
    let r_path = rust_so_path();
    assert!(
        c_path.exists(),
        "C .so not built at {:?}; build it first",
        c_path
    );
    assert!(
        r_path.exists(),
        "Rust .so not built at {:?}; run cargo build first",
        r_path
    );
    let c = Lib::open(c_path.to_str().unwrap());
    let r = Lib::open(r_path.to_str().unwrap());
    (c, r)
}

// ---- Function-pointer types matching the C API ----

pub type FnTarget = unsafe extern "C" fn(c_int) -> c_int;
pub type FnCallOnce = unsafe extern "C" fn(c_int) -> c_int;
pub type FnProcessStream = unsafe extern "C" fn(*const c_int, usize) -> c_int;

pub type FnIvInit = unsafe extern "C" fn(*mut IntVec);
pub type FnIvFree = unsafe extern "C" fn(*mut IntVec);
pub type FnIvReserve = unsafe extern "C" fn(*mut IntVec, usize) -> bool;
pub type FnIvPush = unsafe extern "C" fn(*mut IntVec, c_int) -> bool;
pub type FnIvPop = unsafe extern "C" fn(*mut IntVec, *mut c_int) -> bool;
pub type FnIvPeek = unsafe extern "C" fn(*const IntVec, c_int) -> c_int;

pub type FnProgInit = unsafe extern "C" fn(*mut Program, *const c_int, usize);
pub type FnProgFetch = unsafe extern "C" fn(*mut Program, *mut c_int) -> bool;

pub type FnVmInit = unsafe extern "C" fn(*mut VM);
pub type FnVmFree = unsafe extern "C" fn(*mut VM);
pub type FnVmTrace = unsafe extern "C" fn(*mut VM, c_int);
pub type FnRunEngine = unsafe extern "C" fn(c_int, *const c_int, usize, *mut VM) -> c_int;

#[allow(dead_code)]
pub fn slice_from_intvec(v: &IntVec) -> &[c_int] {
    if v.data.is_null() || v.len == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(v.data, v.len) }
    }
}
