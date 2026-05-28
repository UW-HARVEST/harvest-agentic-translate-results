// Integration test: compare C .so vs Rust .so byte-for-byte through FFI.
// Both libraries are loaded via libloading and called as an external caller would.
//
// Both static_sum and driver share an internal `static int sum` that persists
// across calls within a single library load. To keep the comparison deterministic,
// we run everything in a single #[test] (single thread) and drive both libraries
// with the same call sequence so their internal state evolves identically.

use libloading::{Library, Symbol};
use std::os::raw::c_int;
use std::path::PathBuf;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_lib_path() -> PathBuf {
    workspace_root().join("c_src/build/libStaticLoop.so")
}

fn rust_lib_path() -> PathBuf {
    let mut p = workspace_root();
    p.push("target");
    p.push("debug");
    p.push("libStaticLoop.so");
    p
}

// Helper: redirect stdout while running closure, return captured bytes.
fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    use std::io::Read;
    use std::os::unix::io::FromRawFd;

    unsafe {
        libc::fflush(std::ptr::null_mut());

        let saved = libc::dup(1);
        assert!(saved >= 0, "dup failed");

        let mut fds: [c_int; 2] = [0; 2];
        let r = libc::pipe(fds.as_mut_ptr());
        assert_eq!(r, 0, "pipe failed");

        let r = libc::dup2(fds[1], 1);
        assert!(r >= 0, "dup2 failed");
        libc::close(fds[1]);

        f();

        libc::fflush(std::ptr::null_mut());

        libc::dup2(saved, 1);
        libc::close(saved);

        let mut file = std::fs::File::from_raw_fd(fds[0]);
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).expect("read from pipe failed");
        buf
    }
}

#[test]
fn ffi_byte_for_byte_match() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("failed to load C .so");
        let r_lib = Library::new(rust_lib_path()).expect("failed to load Rust .so");

        let c_sum: Symbol<unsafe extern "C" fn(c_int) -> c_int> =
            c_lib.get(b"static_sum").unwrap();
        let r_sum: Symbol<unsafe extern "C" fn(c_int) -> c_int> =
            r_lib.get(b"static_sum").unwrap();
        let c_driver: Symbol<unsafe extern "C" fn(c_int)> =
            c_lib.get(b"driver").unwrap();
        let r_driver: Symbol<unsafe extern "C" fn(c_int)> =
            r_lib.get(b"driver").unwrap();

        // ---- 1. static_sum: identical update sequence on both libs ----
        // Each library starts with internal sum == 0. Drive both with the same
        // updates and verify byte-equal results at every step.
        let updates: &[c_int] = &[
            0, 1, 2, 3, 4, 5, -1, -2, 100, -50, 0, 7, 7, 7,
            i32::MAX / 4, -(i32::MAX / 4), 0, 1, -1,
            // negative excursions
            -1000, 500, -250, 125,
        ];
        for (idx, &u) in updates.iter().enumerate() {
            let cv = c_sum(u);
            let rv = r_sum(u);
            assert_eq!(cv, rv, "static_sum mismatch at step {} update {}", idx, u);
        }

        // ---- 2. static_sum: many zero-updates should preserve sum identically ----
        for _ in 0..50 {
            let cv = c_sum(0);
            let rv = r_sum(0);
            assert_eq!(cv, rv, "static_sum(0) idempotency mismatch");
        }

        // ---- 3. driver: byte-equal stdout output for various strides ----
        // driver internally calls static_sum 10 times. The static sum continues
        // from wherever it was; we just need both libs to be in identical state
        // before each call (which they are, since they've received the same input
        // sequence so far). After each driver call, both libs again have the
        // same internal state.
        for &stride in &[0, 1, 2, 3, -1, -5, 10, 100, -123, 7] {
            let c_out = capture_stdout(|| c_driver(stride));
            let r_out = capture_stdout(|| r_driver(stride));
            assert_eq!(
                c_out, r_out,
                "driver({}) output mismatch:\n C: {:?}\n R: {:?}",
                stride,
                String::from_utf8_lossy(&c_out),
                String::from_utf8_lossy(&r_out)
            );
        }

        // ---- 4. mixed: a few static_sum updates between driver calls ----
        for &(stride, extras) in &[
            (1i32, &[5i32, -3, 0][..]),
            (-2, &[100, -100, 50][..]),
            (3, &[][..]),
            (0, &[1, 2, 3, 4, 5][..]),
        ] {
            for &u in extras {
                let cv = c_sum(u);
                let rv = r_sum(u);
                assert_eq!(cv, rv, "static_sum mismatch in mixed phase update {}", u);
            }
            let c_out = capture_stdout(|| c_driver(stride));
            let r_out = capture_stdout(|| r_driver(stride));
            assert_eq!(
                c_out, r_out,
                "mixed driver({}) output mismatch", stride
            );
        }
    }
}
