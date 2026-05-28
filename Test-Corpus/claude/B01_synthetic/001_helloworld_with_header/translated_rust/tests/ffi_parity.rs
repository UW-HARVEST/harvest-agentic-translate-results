//! FFI parity tests: load both the C shared library and the Rust cdylib
//! through `libloading`, invoke the exported `helloworld` symbol from each,
//! and compare their results byte-for-byte (return value + stdout).
//!
//! The test never calls Rust functions directly — it always goes through the
//! `#[no_mangle] pub extern "C"` boundary, exactly as an external caller
//! would.

use std::ffi::OsStr;
use std::fs;
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::raw::c_int;
use std::path::{Path, PathBuf};
use std::process::Command;

use libloading::{Library, Symbol};

/// Find the directory containing this crate's Cargo.toml so that the test can
/// reference `c_src/` and `target/` regardless of where Cargo invokes it from.
fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Build the C source as a position-independent shared library. We do this
/// from the test (rather than relying on a pre-existing build artifact) so the
/// test is self-contained and reproducible.
fn build_c_shared_lib() -> PathBuf {
    let root = crate_root();
    let out_dir = root.join("target").join("c_test_artifacts");
    fs::create_dir_all(&out_dir).expect("create c_test_artifacts dir");

    let so_path = out_dir.join("libdriver_c.so");
    let src = root.join("c_src").join("src").join("sillymain.c");

    let status = Command::new("cc")
        .arg("-shared")
        .arg("-fPIC")
        .arg("-O2")
        .arg(&src)
        .arg("-o")
        .arg(&so_path)
        .status()
        .expect("invoke C compiler");
    assert!(status.success(), "C compilation failed");
    so_path
}

/// Locate the Rust cdylib. Cargo places it in
/// `target/<profile>/libdriver.so` for tests built with `cargo test`.
fn rust_shared_lib() -> PathBuf {
    let root = crate_root();
    // Tests run after `cargo build` for the current profile, but the cdylib
    // is not automatically built before integration tests. Trigger it.
    let status = Command::new(env!("CARGO"))
        .current_dir(&root)
        .arg("build")
        .arg("--lib")
        .status()
        .expect("invoke cargo to build cdylib");
    assert!(status.success(), "cargo build --lib failed");

    // Both debug and release profiles place the cdylib here. Prefer the
    // profile that matches the running test binary.
    let candidates = [
        root.join("target").join("debug").join("libdriver.so"),
        root.join("target").join("release").join("libdriver.so"),
    ];
    candidates
        .into_iter()
        .find(|p| p.exists())
        .expect("rust cdylib not found")
}

/// Run a closure while capturing everything written to file descriptor 1
/// (stdout). The captured bytes are returned. We redirect stdout at the libc
/// level so output produced by `printf` inside the C library is captured too.
fn capture_stdout<F: FnOnce() -> R, R>(f: F) -> (R, Vec<u8>) {
    use std::os::unix::io::AsRawFd;

    // Flush Rust stdout before we redirect so any buffered writes don't leak
    // into our captured buffer.
    std::io::stdout().flush().ok();

    let mut tmp = tempfile_in(crate_root().join("target")).expect("tmp file");
    let saved_fd = unsafe { libc_dup(1) };
    assert!(saved_fd >= 0, "dup(1) failed");

    let tmp_fd = tmp.as_raw_fd();
    let dup_rc = unsafe { libc_dup2(tmp_fd, 1) };
    assert!(dup_rc >= 0, "dup2 failed");

    let result = f();

    // Make sure all C-level buffered output is flushed to fd 1 before we
    // restore it, otherwise printf's stdio buffer keeps the bytes hostage.
    unsafe {
        libc_fflush_stdout();
    }
    std::io::stdout().flush().ok();

    let restore_rc = unsafe { libc_dup2(saved_fd, 1) };
    assert!(restore_rc >= 0, "dup2 restore failed");
    unsafe { libc_close(saved_fd) };

    tmp.seek(SeekFrom::Start(0)).expect("seek");
    let mut buf = Vec::new();
    tmp.read_to_end(&mut buf).expect("read tmp");
    (result, buf)
}

// Minimal libc shims so we don't have to add a libc dependency.
extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut std::ffi::c_void) -> c_int;
}

#[allow(non_snake_case)]
unsafe fn libc_dup(fd: c_int) -> c_int {
    dup(fd)
}
#[allow(non_snake_case)]
unsafe fn libc_dup2(a: c_int, b: c_int) -> c_int {
    dup2(a, b)
}
#[allow(non_snake_case)]
unsafe fn libc_close(fd: c_int) -> c_int {
    close(fd)
}
#[allow(non_snake_case)]
unsafe fn libc_fflush_stdout() {
    // Passing NULL flushes all stdio streams.
    fflush(std::ptr::null_mut());
}

/// Tiny helper to make a unique temp file inside `dir`. Avoids pulling in the
/// `tempfile` crate.
fn tempfile_in(dir: impl AsRef<Path>) -> std::io::Result<fs::File> {
    use std::time::{SystemTime, UNIX_EPOCH};
    fs::create_dir_all(&dir)?;
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let pid = std::process::id();
    let path = dir
        .as_ref()
        .join(format!("ffi_parity_capture_{}_{}.tmp", pid, nanos));
    fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(&path)
}

unsafe fn call_helloworld(lib: &Library) -> c_int {
    let sym: Symbol<unsafe extern "C" fn() -> c_int> =
        lib.get(b"helloworld\0").expect("symbol helloworld");
    sym()
}

#[test]
fn helloworld_matches_c() {
    let c_so = build_c_shared_lib();
    let rust_so = rust_shared_lib();

    let c_lib = unsafe { Library::new(c_so.as_os_str()).expect("load C so") };
    let rust_lib = unsafe { Library::new(rust_so.as_os_str()).expect("load Rust so") };

    let (c_ret, c_out) = capture_stdout(|| unsafe { call_helloworld(&c_lib) });
    let (rust_ret, rust_out) = capture_stdout(|| unsafe { call_helloworld(&rust_lib) });

    assert_eq!(c_ret, rust_ret, "return values differ");
    assert_eq!(
        c_out,
        rust_out,
        "stdout differs:\n  C    = {:?}\n  Rust = {:?}",
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&rust_out),
    );

    // Sanity check: the C source prints exactly this string.
    assert_eq!(c_out, b"Hello World!\n");
}

/// Verify that every symbol the C shared library exports is also exported by
/// the Rust shared library. The C library only exports `helloworld`, but we
/// run `nm -D` on both to be sure.
#[test]
fn rust_so_exports_superset_of_c_so() {
    let c_so = build_c_shared_lib();
    let rust_so = rust_shared_lib();

    let c_syms = nm_dynamic_symbols(&c_so);
    let rust_syms = nm_dynamic_symbols(&rust_so);

    // Symbols defined in the C library that callers might use. We exclude
    // weak/undefined symbols (those imported from libc, etc.) because they
    // are not part of the library's exported API.
    for sym in &c_syms {
        assert!(
            rust_syms.contains(sym),
            "Rust .so missing symbol {:?} that C .so exports.\n\
             C exports:    {:?}\n\
             Rust exports: {:?}",
            sym,
            c_syms,
            rust_syms,
        );
    }
}

fn nm_dynamic_symbols(path: &Path) -> Vec<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(path.as_os_str())
        .output()
        .expect("run nm -D");
    assert!(out.status.success(), "nm failed for {:?}", path);
    let mut syms = Vec::new();
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        // nm output: "<addr> <type> <name>"
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 3 {
            continue;
        }
        let ty = parts[1];
        let name = parts[2];
        // We're only interested in user-defined exports, not autogenerated
        // glue like `_init`, `_fini`, `__bss_start`, etc.
        let is_user_export = matches!(ty, "T" | "B" | "D" | "R")
            && !name.starts_with('_')
            && !matches!(name, "data_start" | "edata" | "end");
        if is_user_export {
            syms.push(name.to_string());
        }
    }
    syms.sort();
    syms.dedup();
    syms
}

// Touch unused warnings for OsStr to keep the import explicit; helps
// readability for anyone tweaking the file later.
#[allow(dead_code)]
fn _osstr_marker(_: &OsStr) {}
