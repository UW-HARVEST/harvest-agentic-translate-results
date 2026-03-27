use libloading::{Library, Symbol};
use std::os::raw::{c_int, c_uint};

/// Capture stdout from a closure that writes to stdout via printf/C calls.
fn capture_stdout(f: impl FnOnce()) -> String {
    unsafe {
        libc::fflush(std::ptr::null_mut());
        let mut fds = [0i32; 2];
        assert_eq!(libc::pipe(fds.as_mut_ptr()), 0);
        let saved = libc::dup(1);
        libc::dup2(fds[1], 1);
        f();
        libc::fflush(std::ptr::null_mut());
        libc::dup2(saved, 1);
        libc::close(saved);
        libc::close(fds[1]);
        let mut buf = vec![0u8; 4096];
        let n = libc::read(fds[0], buf.as_mut_ptr() as *mut _, buf.len());
        libc::close(fds[0]);
        if n > 0 {
            buf.truncate(n as usize);
            String::from_utf8_lossy(&buf).into_owned()
        } else {
            String::new()
        }
    }
}

fn c_lib_path() -> std::path::PathBuf {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    std::path::PathBuf::from(manifest).join("c_src/build/libdriver.so")
}

#[test]
fn test_driver_outputs_match() {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("Failed to load C libdriver.so") };
    let c_driver: Symbol<unsafe extern "C" fn(c_uint, c_uint, bool, c_int)> =
        unsafe { c_lib.get(b"driver").expect("C driver symbol not found") };

    let cases: Vec<(c_uint, c_uint, bool, c_int)> = vec![
        (0, 0, false, 0),
        (1, 1, true, 1),
        (3, 7, true, -1),
        (5, 10, false, 42),
        (0xFFFFFFFF, 0xFFFFFFFF, true, i32::MIN),
        (2, 5, true, i32::MAX),
        (4, 8, false, -100),
    ];

    for (x, y, b, z) in &cases {
        let c_out = capture_stdout(|| unsafe { c_driver(*x, *y, *b, *z) });
        let rust_out = capture_stdout(|| driver::driver(*x, *y, *b, *z));
        assert_eq!(
            c_out, rust_out,
            "Mismatch for driver({}, {}, {}, {}): C={:?} Rust={:?}",
            x, y, b, z, c_out, rust_out
        );
    }
}

#[test]
fn test_print_foo_outputs_match() {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("Failed to load C libdriver.so") };

    // C print_foo takes a pointer to foo_t (8 bytes: bitfields u32 + z i32)
    // We'll call C driver and Rust print_foo with equivalent structs
    // But actually, let's call both C and Rust print_foo with the same struct bytes.
    type PrintFooFn = unsafe extern "C" fn(*const driver::foo_t);
    let c_print_foo: Symbol<PrintFooFn> =
        unsafe { c_lib.get(b"print_foo").expect("C print_foo symbol not found") };

    // Test cases: construct foo_t structs with known bitfield values
    // bitfields word: x at bits 0-1, y at bits 2-4, b at bit 5
    let cases: Vec<(c_uint, c_int)> = vec![
        (0, 0),                                    // all zero
        (3 | (7 << 2) | (1 << 5), -1),            // x=3, y=7, b=1, z=-1
        (1 | (2 << 2) | (0 << 5), 42),            // x=1, y=2, b=0, z=42
        (0 | (0 << 2) | (1 << 5), i32::MIN),      // x=0, y=0, b=1, z=MIN
        (2 | (5 << 2) | (1 << 5), i32::MAX),      // x=2, y=5, b=1, z=MAX
    ];

    for (bitfields, z) in &cases {
        let foo = driver::foo_t { bitfields: *bitfields, z: *z };
        let c_out = capture_stdout(|| unsafe { c_print_foo(&foo) });
        let rust_out = capture_stdout(|| unsafe { driver::print_foo(&foo) });
        assert_eq!(
            c_out, rust_out,
            "Mismatch for print_foo(bitfields=0x{:x}, z={}): C={:?} Rust={:?}",
            bitfields, z, c_out, rust_out
        );
    }
}
