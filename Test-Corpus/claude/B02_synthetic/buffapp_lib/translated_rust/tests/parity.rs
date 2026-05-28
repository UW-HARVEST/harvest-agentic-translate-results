// Integration test: load both C and Rust .so files via libloading and
// compare their outputs through the FFI boundary for byte-for-byte parity.

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int};
use std::os::raw::c_void;
use std::path::PathBuf;

#[repr(C)]
struct StringBuffer {
    data: *mut c_char,
    capacity: c_int,
    length: c_int,
}

fn c_lib_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("c_src");
    p.push("build");
    p.push("libtranslated_rust.so");
    p
}

fn rust_lib_path() -> PathBuf {
    // Use the cdylib output. Tests may run under either debug or release.
    let mut base = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    base.push("target");

    let release = base.join("release").join("libbuffapp_lib.so");
    if release.exists() {
        return release;
    }
    let debug = base.join("debug").join("libbuffapp_lib.so");
    debug
}

fn load_libs() -> (Library, Library) {
    let c_path = c_lib_path();
    let r_path = rust_lib_path();
    assert!(
        c_path.exists(),
        "C library not built; expected {}",
        c_path.display()
    );
    assert!(
        r_path.exists(),
        "Rust library not built; expected {}",
        r_path.display()
    );
    unsafe {
        let c_lib = Library::new(&c_path).expect("failed to load C lib");
        let r_lib = Library::new(&r_path).expect("failed to load Rust lib");
        (c_lib, r_lib)
    }
}

type CreateBufferFn = unsafe extern "C" fn(c_int) -> *mut StringBuffer;
type AppendToBufferFn = unsafe extern "C" fn(*mut StringBuffer, *const c_char) -> c_int;
type DestroyBufferFn = unsafe extern "C" fn(*mut StringBuffer);
type GetOperationNameFn = unsafe extern "C" fn(c_int) -> *const c_char;
type PerformOperationFn = unsafe extern "C" fn(c_int, c_int, *const c_char) -> c_int;
type BuffappFn = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

unsafe fn cstr_eq(a: *const c_char, b: *const c_char) -> bool {
    if a.is_null() || b.is_null() {
        return a == b;
    }
    let mut i = 0isize;
    loop {
        let ca = *a.offset(i);
        let cb = *b.offset(i);
        if ca != cb {
            return false;
        }
        if ca == 0 {
            return true;
        }
        i += 1;
    }
}

unsafe fn cstr_to_owned(p: *const c_char) -> Vec<u8> {
    if p.is_null() {
        return vec![];
    }
    let mut out = Vec::new();
    let mut i = 0isize;
    loop {
        let c = *p.offset(i) as u8;
        if c == 0 {
            return out;
        }
        out.push(c);
        i += 1;
    }
}

#[test]
fn test_get_operation_name_parity() {
    let (c_lib, r_lib) = load_libs();
    unsafe {
        let c_fn: Symbol<GetOperationNameFn> = c_lib.get(b"get_operation_name").unwrap();
        let r_fn: Symbol<GetOperationNameFn> = r_lib.get(b"get_operation_name").unwrap();

        for op in [-1, 0, 1, 2, 3, 4, 5, 100, -100] {
            let cp = c_fn(op);
            let rp = r_fn(op);
            assert!(
                cstr_eq(cp, rp),
                "mismatch for op_code={}: C={:?} Rust={:?}",
                op,
                cstr_to_owned(cp),
                cstr_to_owned(rp)
            );
        }
    }
}

#[test]
fn test_perform_operation_parity() {
    let (c_lib, r_lib) = load_libs();
    unsafe {
        let c_fn: Symbol<PerformOperationFn> = c_lib.get(b"perform_operation").unwrap();
        let r_fn: Symbol<PerformOperationFn> = r_lib.get(b"perform_operation").unwrap();

        let ops: &[&[u8]] = &[
            b"add\0",
            b"subtract\0",
            b"multiply\0",
            b"divide\0",
            b"unknown\0",
            b"\0",
        ];

        let inputs = [
            (0i32, 0i32),
            (1, 2),
            (-3, 4),
            (10, 5),
            (10, 0),
            (-10, 3),
            (7, -1),
            (100, 25),
            (i32::MAX / 4, 2),
            (-1234, 56),
        ];

        for op in ops {
            for (a, b) in inputs.iter() {
                // Skip the divide-overflow case to avoid SIGFPE in C.
                if op == b"divide\0" && *b == -1 && *a == i32::MIN {
                    continue;
                }
                let cv = c_fn(*a, *b, op.as_ptr() as *const c_char);
                let rv = r_fn(*a, *b, op.as_ptr() as *const c_char);
                assert_eq!(
                    cv, rv,
                    "mismatch for op={:?} a={} b={}: C={} Rust={}",
                    std::str::from_utf8(op).unwrap_or("?"),
                    a,
                    b,
                    cv,
                    rv
                );
            }
        }
    }
}

#[test]
fn test_buffer_lifecycle_parity() {
    let (c_lib, r_lib) = load_libs();
    unsafe {
        let c_create: Symbol<CreateBufferFn> = c_lib.get(b"create_buffer").unwrap();
        let c_append: Symbol<AppendToBufferFn> = c_lib.get(b"append_to_buffer").unwrap();
        let c_destroy: Symbol<DestroyBufferFn> = c_lib.get(b"destroy_buffer").unwrap();

        let r_create: Symbol<CreateBufferFn> = r_lib.get(b"create_buffer").unwrap();
        let r_append: Symbol<AppendToBufferFn> = r_lib.get(b"append_to_buffer").unwrap();
        let r_destroy: Symbol<DestroyBufferFn> = r_lib.get(b"destroy_buffer").unwrap();

        for &cap in &[1, 4, 16, 32, 64, 128] {
            let cb = c_create(cap);
            let rb = r_create(cap);
            assert!(!cb.is_null() && !rb.is_null());

            // Compare initial state
            assert_eq!((*cb).capacity, (*rb).capacity);
            assert_eq!((*cb).length, (*rb).length);

            // Append several strings
            let pieces: &[&[u8]] = &[
                b"hello\0",
                b", world\0",
                b"!\0",
                b" some longer text that forces growth beyond initial capacity\0",
                b"\0",
                b"more\0",
            ];

            for s in pieces {
                let p = s.as_ptr() as *const c_char;
                let cr = c_append(cb, p);
                let rr = r_append(rb, p);
                assert_eq!(cr, rr, "append return mismatch for {:?}", s);
                assert_eq!((*cb).length, (*rb).length, "length mismatch");
                assert_eq!(
                    (*cb).capacity,
                    (*rb).capacity,
                    "capacity mismatch after appending {:?}",
                    s
                );
                let cdata = cstr_to_owned((*cb).data as *const c_char);
                let rdata = cstr_to_owned((*rb).data as *const c_char);
                assert_eq!(cdata, rdata, "buffer data mismatch after appending {:?}", s);
            }

            c_destroy(cb);
            r_destroy(rb);
        }
    }
}

#[test]
fn test_buffapp_parity() {
    let (c_lib, r_lib) = load_libs();
    unsafe {
        let c_fn: Symbol<BuffappFn> = c_lib.get(b"buffapp").unwrap();
        let r_fn: Symbol<BuffappFn> = r_lib.get(b"buffapp").unwrap();

        let cases: &[(c_int, c_int, c_int, c_int)] = &[
            (0, 0, 0, 0),
            (1, 2, 3, 4),
            (5, 0, 5, 0),
            (-1, -2, -3, -4),
            (10, 5, 2, 3),
            (100, 50, 25, 5),
            (7, 8, 9, 10),
            (3, 4, 5, 6),
            (-10, 5, -2, 8),
            (1000, 1, 4, 2),
            (4, 2, 8, 4),
            (123, 456, 789, 12),
        ];

        for &(a, b, c, d) in cases {
            let cv = c_fn(a, b, c, d);
            let rv = r_fn(a, b, c, d);
            assert_eq!(
                cv, rv,
                "buffapp mismatch for ({}, {}, {}, {}): C={} Rust={}",
                a, b, c, d, cv, rv
            );
        }
    }
}

// Suppress unused warning for the c_void alias if not needed.
#[allow(dead_code)]
fn _ensure_c_void_used(_p: *mut c_void) {}
