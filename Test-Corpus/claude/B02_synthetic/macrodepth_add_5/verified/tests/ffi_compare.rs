//! FFI comparison harness — load both the C and the Rust shared libraries
//! and compare every exported function's behavior byte-for-byte.
//!
//! The active feature combination (e.g. `add,5`) determines which C library
//! we load; the C libraries are pre-built by `build_c_libs.sh` and live in
//! `<crate-root>/c_libs/lib_<op>_<n>.so`.

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int};
use std::os::raw::c_void;
use std::path::PathBuf;

// --- Compile-time selection of the C library that matches the active feature set ---

#[cfg(feature = "sub")]
const OP_NAME: &str = "sub";
#[cfg(all(feature = "mul", not(feature = "sub")))]
const OP_NAME: &str = "mul";
#[cfg(all(not(feature = "sub"), not(feature = "mul")))]
const OP_NAME: &str = "add";

#[cfg(feature = "0")]
const REPEAT_N: u32 = 0;
#[cfg(all(feature = "1", not(feature = "0")))]
const REPEAT_N: u32 = 1;
#[cfg(all(feature = "2", not(any(feature = "0", feature = "1"))))]
const REPEAT_N: u32 = 2;
#[cfg(all(feature = "3", not(any(feature = "0", feature = "1", feature = "2"))))]
const REPEAT_N: u32 = 3;
#[cfg(all(feature = "4", not(any(feature = "0", feature = "1", feature = "2", feature = "3"))))]
const REPEAT_N: u32 = 4;
#[cfg(all(
    feature = "5",
    not(any(feature = "0", feature = "1", feature = "2", feature = "3", feature = "4"))
))]
const REPEAT_N: u32 = 5;
#[cfg(all(
    feature = "6",
    not(any(
        feature = "0",
        feature = "1",
        feature = "2",
        feature = "3",
        feature = "4",
        feature = "5"
    ))
))]
const REPEAT_N: u32 = 6;
#[cfg(all(
    feature = "7",
    not(any(
        feature = "0",
        feature = "1",
        feature = "2",
        feature = "3",
        feature = "4",
        feature = "5",
        feature = "6"
    ))
))]
const REPEAT_N: u32 = 7;
#[cfg(not(any(
    feature = "0",
    feature = "1",
    feature = "2",
    feature = "3",
    feature = "4",
    feature = "5",
    feature = "6",
    feature = "7"
)))]
const REPEAT_N: u32 = 5;

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn rust_lib_path() -> PathBuf {
    // Cargo always writes cdylibs for tests under target/<profile>/.
    // CARGO_TARGET_DIR may be unset; default to <crate>/target.
    let mut p = crate_root();
    p.push("target");
    let release = p.join("release").join("libdriver.so");
    let debug = p.join("debug").join("libdriver.so");
    if release.exists() {
        release
    } else if debug.exists() {
        debug
    } else {
        // Fall back to release path; the test will bail with a clear message.
        release
    }
}

fn c_lib_path() -> PathBuf {
    let mut p = crate_root();
    p.push("c_libs");
    p.push(format!("lib_{}_{}.so", OP_NAME, REPEAT_N));
    p
}

// ---- libc bindings (we don't depend on the libc crate) ----
extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

use std::sync::Mutex;
static CAPTURE_LOCK: Mutex<()> = Mutex::new(());

/// Capture everything that gets written to stdout (fd 1) while `f` runs.
/// Routes fd 1 through a temp file so both C `printf` and Rust `println!`
/// land in the same buffer in order. Serialized via a global mutex so
/// concurrent tests don't trample each other's redirection.
fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    let _guard = CAPTURE_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    // Drain pending output so it doesn't bleed into our capture.
    let _ = std::io::Write::flush(&mut std::io::stdout());
    unsafe {
        fflush(std::ptr::null_mut());
    }

    use std::os::unix::io::AsRawFd;
    let tmp_path = std::env::temp_dir().join(format!(
        "ffi_capture_{}_{}.tmp",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    let tmp = std::fs::File::create(&tmp_path).expect("temp file");
    let tmp_fd = tmp.as_raw_fd();

    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    let rc = unsafe { dup2(tmp_fd, 1) };
    assert_eq!(rc, 1, "dup2(tmp_fd, 1) failed");

    f();

    let _ = std::io::Write::flush(&mut std::io::stdout());
    unsafe {
        fflush(std::ptr::null_mut());
    }

    // Restore stdout.
    unsafe {
        dup2(saved, 1);
        close(saved);
    }
    drop(tmp);

    let bytes = std::fs::read(&tmp_path).unwrap_or_default();
    let _ = std::fs::remove_file(&tmp_path);
    bytes
}

// ---- Function-pointer typedefs for symbols we'll resolve ----

type FnIntInt = unsafe extern "C" fn(c_int, c_int) -> c_int;
type FnIntOnly = unsafe extern "C" fn(c_int) -> c_int;

struct Loaded {
    _lib: Library,
    op_add: FnIntInt,
    op_sub: FnIntInt,
    op_mul: FnIntInt,
    helper_call: FnIntInt,
    helper_ptr: FnIntInt,
    use_generated: FnIntOnly,
    g_op_ptr: *const FnIntInt,
    g_op_name_ptr: *const *const c_char,
}

unsafe fn load_lib(path: &std::path::Path) -> Loaded {
    let lib = Library::new(path).unwrap_or_else(|e| {
        panic!("Failed to load library {:?}: {}", path, e);
    });
    // Use `into_raw` indirection — keep the library alive in the struct.
    let op_add: Symbol<FnIntInt> = lib.get(b"op_add\0").unwrap();
    let op_add = *op_add.into_raw();
    let op_sub: Symbol<FnIntInt> = lib.get(b"op_sub\0").unwrap();
    let op_sub = *op_sub.into_raw();
    let op_mul: Symbol<FnIntInt> = lib.get(b"op_mul\0").unwrap();
    let op_mul = *op_mul.into_raw();
    let helper_call: Symbol<FnIntInt> = lib.get(b"helper_call\0").unwrap();
    let helper_call = *helper_call.into_raw();
    let helper_ptr: Symbol<FnIntInt> = lib.get(b"helper_ptr\0").unwrap();
    let helper_ptr = *helper_ptr.into_raw();
    let use_generated: Symbol<FnIntOnly> = lib.get(b"use_generated\0").unwrap();
    let use_generated = *use_generated.into_raw();
    let g_op_sym: Symbol<*const FnIntInt> = lib.get(b"G_OP\0").unwrap();
    let g_op_ptr: *const FnIntInt = *g_op_sym.into_raw();
    let g_op_name_sym: Symbol<*const *const c_char> = lib.get(b"G_OP_NAME\0").unwrap();
    let g_op_name_ptr: *const *const c_char = *g_op_name_sym.into_raw();
    Loaded {
        _lib: lib,
        op_add,
        op_sub,
        op_mul,
        helper_call,
        helper_ptr,
        use_generated,
        g_op_ptr,
        g_op_name_ptr,
    }
}

fn loaders() -> (Loaded, Loaded) {
    let cpath = c_lib_path();
    let rpath = rust_lib_path();
    assert!(cpath.exists(), "C library missing: {:?}", cpath);
    assert!(rpath.exists(), "Rust library missing: {:?}", rpath);
    unsafe { (load_lib(&cpath), load_lib(&rpath)) }
}

// ---- Tests ----

const PAIRS: &[(c_int, c_int)] = &[
    (0, 0),
    (1, 1),
    (-1, 1),
    (3, 4),
    (-3, -4),
    (7, -2),
    (1000, 25),
    (-1000, -25),
    (i32::MAX, 1),
    (i32::MIN, -1),
    (i32::MAX, i32::MAX),
    (i32::MIN, i32::MIN),
    (123, 456),
    (-77, 13),
];

#[test]
fn op_add_matches() {
    let (c, r) = loaders();
    for &(a, b) in PAIRS {
        let cv = unsafe { (c.op_add)(a, b) };
        let rv = unsafe { (r.op_add)(a, b) };
        assert_eq!(cv, rv, "op_add({}, {})", a, b);
    }
}

#[test]
fn op_sub_matches() {
    let (c, r) = loaders();
    for &(a, b) in PAIRS {
        let cv = unsafe { (c.op_sub)(a, b) };
        let rv = unsafe { (r.op_sub)(a, b) };
        assert_eq!(cv, rv, "op_sub({}, {})", a, b);
    }
}

#[test]
fn op_mul_matches() {
    let (c, r) = loaders();
    for &(a, b) in PAIRS {
        let cv = unsafe { (c.op_mul)(a, b) };
        let rv = unsafe { (r.op_mul)(a, b) };
        assert_eq!(cv, rv, "op_mul({}, {})", a, b);
    }
}

#[test]
fn g_op_name_matches() {
    let (c, r) = loaders();
    // Both globals are `const char *`; deref the pointer-to-pointer to get the
    // C string and compare bytes.
    let cstr_c = unsafe {
        let p = *c.g_op_name_ptr;
        std::ffi::CStr::from_ptr(p)
    };
    let cstr_r = unsafe {
        let p = *r.g_op_name_ptr;
        std::ffi::CStr::from_ptr(p)
    };
    assert_eq!(cstr_c.to_bytes(), cstr_r.to_bytes());
    // And both should match the active feature's OP name.
    assert_eq!(cstr_c.to_bytes(), OP_NAME.as_bytes());
}

#[test]
fn g_op_function_matches() {
    let (c, r) = loaders();
    let fc: FnIntInt = unsafe { *c.g_op_ptr };
    let fr: FnIntInt = unsafe { *r.g_op_ptr };
    for &(a, b) in PAIRS {
        let cv = unsafe { fc(a, b) };
        let rv = unsafe { fr(a, b) };
        assert_eq!(cv, rv, "G_OP({}, {})", a, b);
    }
}

#[test]
fn helper_call_matches() {
    let (c, r) = loaders();
    for &(a, b) in PAIRS {
        let mut cv: c_int = 0;
        let cout = capture_stdout(|| unsafe { cv = (c.helper_call)(a, b) });
        let mut rv: c_int = 0;
        let rout = capture_stdout(|| unsafe { rv = (r.helper_call)(a, b) });
        assert_eq!(cv, rv, "helper_call({}, {}) return", a, b);
        assert_eq!(
            cout, rout,
            "helper_call({}, {}) stdout mismatch:\n C: {:?}\n R: {:?}",
            a, b,
            String::from_utf8_lossy(&cout),
            String::from_utf8_lossy(&rout)
        );
    }
}

#[test]
fn helper_ptr_matches() {
    let (c, r) = loaders();
    for &(a, b) in PAIRS {
        let mut cv: c_int = 0;
        let cout = capture_stdout(|| unsafe { cv = (c.helper_ptr)(a, b) });
        let mut rv: c_int = 0;
        let rout = capture_stdout(|| unsafe { rv = (r.helper_ptr)(a, b) });
        assert_eq!(cv, rv, "helper_ptr({}, {}) return", a, b);
        assert_eq!(
            cout, rout,
            "helper_ptr({}, {}) stdout mismatch:\n C: {:?}\n R: {:?}",
            a, b,
            String::from_utf8_lossy(&cout),
            String::from_utf8_lossy(&rout)
        );
    }
}

#[test]
fn use_generated_matches() {
    let (c, r) = loaders();
    // The C `accum_<OP>` function only handles n=0..=6 explicitly; default is
    // a no-op. Test exactly that range, plus a couple of "default" values.
    for n in [0i32, 1, 2, 3, 4, 5, 6, 7, 10, -1].iter().copied() {
        let mut cv: c_int = 0;
        let cout = capture_stdout(|| unsafe { cv = (c.use_generated)(n) });
        let mut rv: c_int = 0;
        let rout = capture_stdout(|| unsafe { rv = (r.use_generated)(n) });
        assert_eq!(cv, rv, "use_generated({}) return", n);
        assert_eq!(
            cout, rout,
            "use_generated({}) stdout mismatch:\n C: {:?}\n R: {:?}",
            n,
            String::from_utf8_lossy(&cout),
            String::from_utf8_lossy(&rout)
        );
    }
}
