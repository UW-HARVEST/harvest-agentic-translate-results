//! Differential test: loads BOTH the C `libdriver.so` and the Rust
//! `libdriver.so` through `libloading` and compares the bytes each one writes
//! to stdout for identical inputs.
//!
//! Neither implementation is ever called directly from Rust code; both go
//! through `dlopen`/`dlsym`, so the `#[no_mangle]` export wrapper is exercised
//! exactly as an external C caller would exercise it.
//!
//! Everything lives in a single `#[test]` on purpose: verifying `driver`
//! requires temporarily pointing file descriptor 1 at a file, and libtest's
//! own progress output would otherwise land inside the captured bytes when
//! tests run in parallel.

use std::ffi::{c_int, c_void};
use std::io::Read;
use std::path::{Path, PathBuf};

use libloading::{Library, Symbol};

// ---------------------------------------------------------------------------
// libc bits needed to capture what the shared objects write to file descriptor
// 1. `printf` inside each .so writes through the process-wide stdio `stdout`,
// so the only reliable way to observe it is to swap out fd 1.
// ---------------------------------------------------------------------------
unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    /// `fflush(NULL)` drains every open output stream, which is required
    /// because a redirected fd 1 makes stdio fully buffered.
    fn fflush(stream: *mut c_void) -> c_int;
}

/// Runs `f`, returning every byte it wrote to stdout.
fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    let mut tmp = std::env::temp_dir();
    tmp.push(format!("driver_capture_{}.bin", std::process::id()));

    unsafe {
        // Make sure nothing already buffered leaks into our capture.
        fflush(std::ptr::null_mut());

        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");

        let file = std::fs::File::create(&tmp).expect("create temp capture file");
        let fd = {
            use std::os::fd::AsRawFd;
            file.as_raw_fd()
        };
        assert!(dup2(fd, 1) >= 0, "dup2 onto stdout failed");

        f();

        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "restoring stdout failed");
        close(saved);
        drop(file);
    }

    let mut bytes = Vec::new();
    std::fs::File::open(&tmp)
        .expect("reopen temp capture file")
        .read_to_end(&mut bytes)
        .expect("read temp capture file");
    let _ = std::fs::remove_file(&tmp);
    bytes
}

// ---------------------------------------------------------------------------
// Library discovery
// ---------------------------------------------------------------------------

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    let root = crate_root();
    let workspace_root = root.parent().expect("translation/ has a parent");
    let candidates = [
        workspace_root.join("c_src/build/libdriver.so"),
        workspace_root.join("c_src/build/libdriver.dylib"),
    ];
    for c in &candidates {
        if c.is_file() {
            return c.clone();
        }
    }
    panic!(
        "C shared library not found; build it with:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .\n\
         looked in: {candidates:?}"
    );
}

fn rust_library_path() -> PathBuf {
    // Allows pointing the harness at a specific artifact (e.g. the `release`
    // cdylib, which is built with `panic = "abort"` and cannot host libtest).
    if let Some(p) = std::env::var_os("DRIVER_RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.is_file(), "DRIVER_RUST_SO does not point at a file: {p:?}");
        return p;
    }

    // `cargo test` always builds the cdylib for the active profile; prefer the
    // artifact that sits alongside the test binary, then fall back.
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        // .../target/<profile>/deps/<test binary>
        if let Some(profile_dir) = exe.parent().and_then(Path::parent) {
            candidates.push(profile_dir.join("libdriver.so"));
            candidates.push(profile_dir.join("libdriver.dylib"));
        }
    }
    for profile in ["release", "debug"] {
        candidates.push(crate_root().join(format!("target/{profile}/libdriver.so")));
        candidates.push(crate_root().join(format!("target/{profile}/libdriver.dylib")));
    }
    for c in &candidates {
        if c.is_file() {
            return c.clone();
        }
    }
    panic!("Rust cdylib not found; looked in: {candidates:?}");
}

type DriverFn = unsafe extern "C" fn(c_int);

// ---------------------------------------------------------------------------
// The test
// ---------------------------------------------------------------------------

#[test]
fn driver_matches_c_implementation() {
    let c_lib = unsafe { Library::new(c_library_path()) }.expect("dlopen C library");
    let rust_lib = unsafe { Library::new(rust_library_path()) }.expect("dlopen Rust cdylib");

    // The Rust .so must export `driver` under exactly the name the C .so uses.
    let c_driver: Symbol<DriverFn> =
        unsafe { c_lib.get(b"driver\0") }.expect("C .so exports `driver`");
    let rust_driver: Symbol<DriverFn> =
        unsafe { rust_lib.get(b"driver\0") }.expect("Rust .so exports `driver`");

    let run_both = |x: c_int| -> (Vec<u8>, Vec<u8>) {
        let c_out = capture_stdout(|| unsafe { c_driver(x) });
        let rust_out = capture_stdout(|| unsafe { rust_driver(x) });
        (c_out, rust_out)
    };
    let check = |x: c_int| {
        let (c_out, rust_out) = run_both(x);
        assert_eq!(
            c_out,
            rust_out,
            "driver({x}) mismatch:\n  C   : {:?}\n  Rust: {:?}",
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&rust_out),
        );
    };

    // --- Harness sanity: a silent-on-both-sides bug must not pass. ---------
    {
        let (c_out, rust_out) = run_both(0x1234_5678);
        assert!(!c_out.is_empty(), "C produced no output");
        assert_eq!(c_out, rust_out);

        // Two lowercase hex digits per byte of `int`, then '\n'.
        assert_eq!(c_out.len(), 2 * std::mem::size_of::<c_int>() + 1);
        assert_eq!(*c_out.last().unwrap(), b'\n');
        assert!(
            c_out[..c_out.len() - 1]
                .iter()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(b)),
            "expected lowercase hex, got {:?}",
            String::from_utf8_lossy(&c_out)
        );
    }

    // --- Edge values ------------------------------------------------------
    let edges: [c_int; 22] = [
        0,
        1,
        -1,
        2,
        -2,
        c_int::MAX,
        c_int::MIN,
        c_int::MAX - 1,
        c_int::MIN + 1,
        0x0000_00ff,
        0x0000_ff00,
        0x00ff_0000,
        -0x0100_0000, // 0xff000000
        0x7f7f_7f7f,
        0x0102_0304,
        0x1234_5678,
        -0x1234_5678,
        0x0000_000a,
        0x0f0f_0f0f,
        0x5555_5555,
        -0x5555_5556, // 0xaaaaaaaa
        0x0000_0080,
    ];
    for x in edges {
        check(x);
    }

    // --- Every byte value in every byte position --------------------------
    // Covers `%02x` zero padding and the no-sign-extension behaviour of the
    // variadic `unsigned char` -> `int` promotion (bytes >= 0x80).
    for shift in 0..std::mem::size_of::<c_int>() {
        for b in 0u32..=255 {
            check((b << (8 * shift as u32)) as c_int);
        }
    }

    // --- Deterministic pseudo-random sweep (xorshift, no extra crates) ----
    let mut state: u32 = 0x9e37_79b9;
    for _ in 0..2000 {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        check(state as c_int);
    }

    // --- Repeated / interleaved calls stay in lockstep --------------------
    // `print_hex` keeps no state, so alternating the two implementations must
    // not perturb either one.
    for x in [-7i32, 0, 7, 0x0bad_f00du32 as i32] {
        let mut prev: Option<(Vec<u8>, Vec<u8>)> = None;
        for _ in 0..3 {
            let (c_out, rust_out) = run_both(x);
            assert_eq!(c_out, rust_out, "driver({x}) mismatch on repeat");
            if let Some((pc, pr)) = &prev {
                assert_eq!(pc, &c_out, "C output not deterministic for {x}");
                assert_eq!(pr, &rust_out, "Rust output not deterministic for {x}");
            }
            prev = Some((c_out, rust_out));
        }
    }

    // --- Several calls inside one capture --------------------------------
    // Confirms newline placement matches when outputs are concatenated rather
    // than observed in isolation.
    {
        let xs: [c_int; 4] = [0, -1, 0x2a, c_int::MIN];
        let c_out = capture_stdout(|| {
            for x in xs {
                unsafe { c_driver(x) };
            }
        });
        let rust_out = capture_stdout(|| {
            for x in xs {
                unsafe { rust_driver(x) };
            }
        });
        assert_eq!(c_out, rust_out);
        assert_eq!(c_out.iter().filter(|b| **b == b'\n').count(), xs.len());
    }
}
