// Integration tests: load the C and Rust shared libraries via libloading,
// invoke their exported symbols with identical inputs, and require
// byte-identical outputs (return values, globals, and stdout).
//
// The build script (build.rs) compiles the C side with the same OP / REPEAT
// values that the active Cargo features select for the Rust side. The
// resulting .so path is exposed via `C_LIB_PATH`.

use std::ffi::{c_char, CStr};
use std::os::raw::c_int;
use std::path::PathBuf;
use std::sync::Mutex;

use libloading::{Library, Symbol};

// Tests here serialize all stdout-capturing operations with this mutex.
// Each capture redirects the global file descriptor 1, so concurrent
// captures across threads would interleave or steal each other's bytes.
static STDOUT_LOCK: Mutex<()> = Mutex::new(());

const C_LIB_PATH: &str = env!("C_LIB_PATH");
const DRIVER_OP: &str = env!("DRIVER_OP");
const DRIVER_REPEAT_STR: &str = env!("DRIVER_REPEAT");

fn rust_lib_path() -> PathBuf {
    // CARGO_MANIFEST_DIR/target/{profile}/libdriver.so
    // For dev tests `profile` is `debug`. We accept either.
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let candidates = [
        manifest_dir.join("target").join("debug").join("libdriver.so"),
        manifest_dir.join("target").join("release").join("libdriver.so"),
    ];
    for c in &candidates {
        if c.exists() {
            return c.clone();
        }
    }
    panic!("could not find libdriver.so under target/{{debug,release}}; build it first");
}

unsafe fn load(path: &std::path::Path) -> Library {
    Library::new(path).unwrap_or_else(|e| panic!("failed to load {}: {}", path.display(), e))
}

/// Run `f` with stdout (fd 1) redirected into a temp file, then return the
/// bytes that were written. Flushes both Rust's stdout buffer and libc's
/// stdout buffer before restoring the original fd so we don't lose writes.
fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    use std::fs::File;
    use std::io::{Read, Seek, SeekFrom};
    use std::os::unix::io::{AsRawFd, FromRawFd, IntoRawFd};

    extern "C" {
        fn dup(oldfd: c_int) -> c_int;
        fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
        fn close(fd: c_int) -> c_int;
        fn fflush(stream: *mut libc_stub::FILE) -> c_int;
    }
    mod libc_stub {
        #[repr(C)]
        pub struct FILE {
            _private: [u8; 0],
        }
    }

    let _ = std::io::Write::flush(&mut std::io::stdout());
    unsafe {
        fflush(std::ptr::null_mut());
    }

    let saved_fd: c_int = unsafe { dup(1) };
    assert!(saved_fd >= 0, "dup(1) failed");

    let tmp = tempfile_in_target();
    let tmp_fd = tmp.as_raw_fd();
    let dup_rc = unsafe { dup2(tmp_fd, 1) };
    assert!(dup_rc >= 0, "dup2 failed");

    f();

    // Flush before swapping back.
    let _ = std::io::Write::flush(&mut std::io::stdout());
    unsafe {
        fflush(std::ptr::null_mut());
    }

    let dup_back = unsafe { dup2(saved_fd, 1) };
    assert!(dup_back >= 0, "dup2 restore failed");
    unsafe {
        close(saved_fd);
    }

    // Rewind the temp file and read what was written.
    let mut f: File = unsafe { File::from_raw_fd(tmp.into_raw_fd()) };
    let _ = f.seek(SeekFrom::Start(0));
    let mut buf = Vec::new();
    f.read_to_end(&mut buf).unwrap();
    buf
}

/// Make an unnamed temp file we can dup2 onto. Uses /tmp because target/ may
/// be on a slow filesystem; either works.
fn tempfile_in_target() -> std::fs::File {
    use std::fs::OpenOptions;
    let mut path = std::env::temp_dir();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    path.push(format!(
        "ffi_capture_{}_{}.tmp",
        std::process::id(),
        nanos
    ));
    let f = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .unwrap();
    // Best-effort unlink so the file disappears when the process exits.
    let _ = std::fs::remove_file(&path);
    f
}

// ---------- Helper: type-erased FFI signatures ----------

type FnIntInt = unsafe extern "C" fn(c_int, c_int) -> c_int;
type FnInt = unsafe extern "C" fn(c_int) -> c_int;
type GlobalFp = *const FnIntInt;
type GlobalCStr = *const *const c_char;

// ---------- Tests for the leaf op_* primitives ----------

fn pairs() -> Vec<(i32, i32)> {
    vec![
        (0, 0),
        (1, 2),
        (-1, 2),
        (3, -7),
        (i32::MAX, 1),
        (i32::MIN, -1),
        (12345, 6789),
        (-12345, 6789),
        (1000, 1000),
        (-1000, -1000),
        (0, i32::MIN),
        (i32::MIN, i32::MIN),
    ]
}

#[test]
fn op_add_matches() {
    let c_lib = unsafe { load(std::path::Path::new(C_LIB_PATH)) };
    let r_lib = unsafe { load(&rust_lib_path()) };
    let c: Symbol<FnIntInt> = unsafe { c_lib.get(b"op_add").unwrap() };
    let r: Symbol<FnIntInt> = unsafe { r_lib.get(b"op_add").unwrap() };
    for (a, b) in pairs() {
        let cv = unsafe { c(a, b) };
        let rv = unsafe { r(a, b) };
        assert_eq!(cv, rv, "op_add({a},{b})");
    }
}

#[test]
fn op_sub_matches() {
    let c_lib = unsafe { load(std::path::Path::new(C_LIB_PATH)) };
    let r_lib = unsafe { load(&rust_lib_path()) };
    let c: Symbol<FnIntInt> = unsafe { c_lib.get(b"op_sub").unwrap() };
    let r: Symbol<FnIntInt> = unsafe { r_lib.get(b"op_sub").unwrap() };
    for (a, b) in pairs() {
        let cv = unsafe { c(a, b) };
        let rv = unsafe { r(a, b) };
        assert_eq!(cv, rv, "op_sub({a},{b})");
    }
}

#[test]
fn op_mul_matches() {
    let c_lib = unsafe { load(std::path::Path::new(C_LIB_PATH)) };
    let r_lib = unsafe { load(&rust_lib_path()) };
    let c: Symbol<FnIntInt> = unsafe { c_lib.get(b"op_mul").unwrap() };
    let r: Symbol<FnIntInt> = unsafe { r_lib.get(b"op_mul").unwrap() };
    for (a, b) in pairs() {
        let cv = unsafe { c(a, b) };
        let rv = unsafe { r(a, b) };
        assert_eq!(cv, rv, "op_mul({a},{b})");
    }
}

// ---------- Globals ----------

#[test]
fn g_op_name_matches() {
    let c_lib = unsafe { load(std::path::Path::new(C_LIB_PATH)) };
    let r_lib = unsafe { load(&rust_lib_path()) };
    let c: Symbol<GlobalCStr> = unsafe { c_lib.get(b"G_OP_NAME").unwrap() };
    let r: Symbol<GlobalCStr> = unsafe { r_lib.get(b"G_OP_NAME").unwrap() };
    let c_str = unsafe { CStr::from_ptr(**c) };
    let r_str = unsafe { CStr::from_ptr(**r) };
    assert_eq!(c_str.to_bytes(), r_str.to_bytes(), "G_OP_NAME");
    assert_eq!(c_str.to_bytes(), DRIVER_OP.as_bytes(), "G_OP_NAME == feature");
}

#[test]
fn g_op_dispatches_same_function() {
    // G_OP is `int (*)(int,int)` — the address differs between libs, but the
    // function it dispatches to must produce identical results.
    let c_lib = unsafe { load(std::path::Path::new(C_LIB_PATH)) };
    let r_lib = unsafe { load(&rust_lib_path()) };
    let c: Symbol<GlobalFp> = unsafe { c_lib.get(b"G_OP").unwrap() };
    let r: Symbol<GlobalFp> = unsafe { r_lib.get(b"G_OP").unwrap() };
    let c_fp: FnIntInt = unsafe { **c };
    let r_fp: FnIntInt = unsafe { **r };
    for (a, b) in pairs() {
        let cv = unsafe { c_fp(a, b) };
        let rv = unsafe { r_fp(a, b) };
        assert_eq!(cv, rv, "G_OP({a},{b})");
    }
}

// ---------- Helpers (stdout matters) ----------

#[test]
fn helper_call_matches() {
    let _g = STDOUT_LOCK.lock().unwrap();
    let c_lib = unsafe { load(std::path::Path::new(C_LIB_PATH)) };
    let r_lib = unsafe { load(&rust_lib_path()) };
    let c: Symbol<FnIntInt> = unsafe { c_lib.get(b"helper_call").unwrap() };
    let r: Symbol<FnIntInt> = unsafe { r_lib.get(b"helper_call").unwrap() };

    for (a, b) in pairs() {
        let mut cv = 0;
        let c_out = capture_stdout(|| {
            cv = unsafe { c(a, b) };
        });
        let mut rv = 0;
        let r_out = capture_stdout(|| {
            rv = unsafe { r(a, b) };
        });
        assert_eq!(cv, rv, "helper_call return value mismatch ({a},{b})");
        assert_eq!(
            c_out, r_out,
            "helper_call({a},{b}) stdout mismatch:\nC={:?}\nRust={:?}",
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&r_out)
        );
    }
}

#[test]
fn helper_ptr_matches() {
    let _g = STDOUT_LOCK.lock().unwrap();
    let c_lib = unsafe { load(std::path::Path::new(C_LIB_PATH)) };
    let r_lib = unsafe { load(&rust_lib_path()) };
    let c: Symbol<FnIntInt> = unsafe { c_lib.get(b"helper_ptr").unwrap() };
    let r: Symbol<FnIntInt> = unsafe { r_lib.get(b"helper_ptr").unwrap() };

    for (a, b) in pairs() {
        let mut cv = 0;
        let c_out = capture_stdout(|| {
            cv = unsafe { c(a, b) };
        });
        let mut rv = 0;
        let r_out = capture_stdout(|| {
            rv = unsafe { r(a, b) };
        });
        assert_eq!(cv, rv, "helper_ptr return value mismatch ({a},{b})");
        assert_eq!(
            c_out, r_out,
            "helper_ptr({a},{b}) stdout mismatch:\nC={:?}\nRust={:?}",
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&r_out)
        );
    }
}

#[test]
fn use_generated_matches() {
    let _g = STDOUT_LOCK.lock().unwrap();
    let c_lib = unsafe { load(std::path::Path::new(C_LIB_PATH)) };
    let r_lib = unsafe { load(&rust_lib_path()) };
    let c: Symbol<FnInt> = unsafe { c_lib.get(b"use_generated").unwrap() };
    let r: Symbol<FnInt> = unsafe { r_lib.get(b"use_generated").unwrap() };

    // C `default: break` means n outside [0,6] returns the initial accumulator.
    for n in [-1i32, 0, 1, 2, 3, 4, 5, 6, 7, 8, 100] {
        let mut cv = 0;
        let c_out = capture_stdout(|| {
            cv = unsafe { c(n) };
        });
        let mut rv = 0;
        let r_out = capture_stdout(|| {
            rv = unsafe { r(n) };
        });
        assert_eq!(cv, rv, "use_generated({n}) return mismatch");
        assert_eq!(
            c_out, r_out,
            "use_generated({n}) stdout mismatch:\nC={:?}\nRust={:?}",
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&r_out)
        );
    }
}

// ---------- Sanity check: REPEAT and OP env vars are coherent ----------

#[test]
fn build_metadata_sane() {
    assert!(["add", "sub", "mul"].contains(&DRIVER_OP), "OP={}", DRIVER_OP);
    let r: i32 = DRIVER_REPEAT_STR.parse().unwrap();
    assert!((0..=7).contains(&r), "REPEAT={}", r);
}
