// Integration test: load both the C-built libdriver.so and the Rust-built
// libdriver.so via libloading, invoke `driver(int)` for various inputs,
// capture stdout, and assert byte-for-byte equality.

use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::fs::File;
use std::io::Read;
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;

type DriverFn = unsafe extern "C" fn(c_int);

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_lib_path() -> PathBuf {
    project_root().join("c_src/build/libdriver.so")
}

fn rust_lib_path() -> PathBuf {
    // Try release first, then debug.
    let release = project_root().join("target/release/libdriver.so");
    if release.exists() {
        return release;
    }
    project_root().join("target/debug/libdriver.so")
}

/// Run `f` with stdout redirected to a pipe; return whatever f wrote to fd 1.
fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    // Flush Rust's stdout buffer first.
    use std::io::Write;
    let _ = std::io::stdout().flush();

    // libc-equivalents via raw syscalls aren't great in pure std; use libc.
    // We'll do it with std + nix-style: pipe, dup, dup2.
    // Use the libc crate? It's not a dep. Instead implement via /tmp file.
    //
    // Strategy: dup current stdout fd, open a temp file, dup2 file fd to 1,
    // call f, flush C stdout via fflush(NULL) by calling libc::fflush.
    // Then dup2 saved fd back to 1, read the temp file.

    // Use `tempfile` — also not a dep. We'll create a unique file path.
    let pid = std::process::id();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("rust_driver_capture_{}_{}.bin", pid, nanos));

    let saved_fd: i32;
    let new_fd: i32;
    unsafe {
        saved_fd = dup(1);
        assert!(saved_fd >= 0, "dup failed");
    }

    let file = File::create(&path).expect("create capture file");
    new_fd = file.as_raw_fd();
    unsafe {
        let r = dup2(new_fd, 1);
        assert!(r >= 0, "dup2 failed");
    }
    drop(file);

    // Run the user function.
    f();

    // Flush both Rust's and C's stdout.
    let _ = std::io::stdout().flush();
    unsafe {
        fflush(std::ptr::null_mut());
    }

    // Restore stdout.
    unsafe {
        let r = dup2(saved_fd, 1);
        assert!(r >= 0, "dup2 restore failed");
        close(saved_fd);
    }

    // Read the captured bytes.
    let mut buf = Vec::new();
    let mut f = File::open(&path).expect("open capture file for read");
    f.read_to_end(&mut buf).expect("read capture file");
    let _ = std::fs::remove_file(&path);
    buf
}

extern "C" {
    fn dup(oldfd: i32) -> i32;
    fn dup2(oldfd: i32, newfd: i32) -> i32;
    fn close(fd: i32) -> i32;
    fn fflush(stream: *mut core::ffi::c_void) -> i32;
}

unsafe fn load_driver(path: &PathBuf) -> (Library, *const ()) {
    let lib = Library::new(path).unwrap_or_else(|e| panic!("loading {:?}: {}", path, e));
    let sym: Symbol<DriverFn> = lib.get(b"driver\0").expect("missing `driver` symbol");
    let raw = *sym.into_raw() as *const ();
    (lib, raw)
}

#[test]
fn driver_outputs_match_for_many_inputs() {
    assert!(
        c_lib_path().exists(),
        "C lib not built at {:?} — build c_src first",
        c_lib_path()
    );
    assert!(
        rust_lib_path().exists(),
        "Rust lib not built at {:?} — run `cargo build --release`",
        rust_lib_path()
    );

    let inputs: Vec<c_int> = vec![
        0, 1, -1, 2, 3, 7, 42, -42, 100, 1000, 12345, -12345,
        i32::MAX, i32::MIN, i32::MAX - 1, i32::MIN + 1,
    ];

    unsafe {
        let (c_lib, c_fn_ptr) = load_driver(&c_lib_path());
        let (rust_lib, rust_fn_ptr) = load_driver(&rust_lib_path());

        let c_fn: DriverFn = std::mem::transmute(c_fn_ptr);
        let rust_fn: DriverFn = std::mem::transmute(rust_fn_ptr);

        for &x in &inputs {
            let c_out = capture_stdout(|| c_fn(x));
            let rust_out = capture_stdout(|| rust_fn(x));
            assert_eq!(
                c_out, rust_out,
                "driver({}) output mismatch:\n  C   = {:?}\n  Rust= {:?}",
                x,
                String::from_utf8_lossy(&c_out),
                String::from_utf8_lossy(&rust_out),
            );
        }

        drop(c_lib);
        drop(rust_lib);
    }
}
