// Integration test comparing the C and Rust shared libraries through FFI.
// Both libraries are loaded via libloading and their exported symbols are
// invoked. Outputs are compared byte-for-byte.

use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::os::raw::c_uint;
use std::path::PathBuf;
use std::sync::Mutex;

// Tests share the per-library global `array` symbol. Run serially.
static SERIAL: Mutex<()> = Mutex::new(());

const ARRAY_SIZE: usize = 256 * 1024;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("c_src/build/liblong.so")
}

fn rust_lib_path() -> PathBuf {
    // Cargo places the cdylib in target/<profile>/liblong.so.
    // Use the OUT_DIR / target dir derived from the test binary location.
    // CARGO_MANIFEST_DIR/target/<profile>/liblong.so
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // Try debug then release.
    let debug = manifest.join("target/debug/liblong.so");
    if debug.exists() {
        return debug;
    }
    manifest.join("target/release/liblong.so")
}

unsafe fn load_lib(p: &PathBuf) -> Library {
    unsafe { Library::new(p).expect("failed to load library") }
}

unsafe fn array_ptr(lib: &Library) -> *mut c_int {
    // libloading's Symbol<T> dereferences such that, when T is a pointer
    // type, `*sym` yields the address of the symbol. This is the pointer
    // to the start of the `array` global.
    let sym: Symbol<*mut c_int> =
        unsafe { lib.get(b"array\0").expect("missing `array` symbol") };
    *sym
}

unsafe fn get_perform_expensive(
    lib: &Library,
) -> Symbol<'_, unsafe extern "C" fn()> {
    unsafe { lib.get(b"perform_expensive_operations\0").unwrap() }
}

unsafe fn get_long_exec(
    lib: &Library,
) -> Symbol<'_, unsafe extern "C" fn(c_uint)> {
    unsafe { lib.get(b"long_exec\0").unwrap() }
}

fn read_array(ptr: *const c_int, n: usize) -> Vec<c_int> {
    unsafe { std::slice::from_raw_parts(ptr, n).to_vec() }
}

fn write_array(ptr: *mut c_int, data: &[c_int]) {
    unsafe {
        std::ptr::copy_nonoverlapping(data.as_ptr(), ptr, data.len());
    }
}

/// Capture stdout written by the closure. Uses POSIX dup2 to redirect
/// fd 1 to a tempfile, then restores it.
fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    use std::io::{Read, Seek, SeekFrom};
    use std::os::unix::io::AsRawFd;

    // Make sure existing stdio buffers are flushed.
    unsafe {
        libc_flush_stdout();
    }

    let tmp = tempfile_open();
    let tmp_fd = tmp.as_raw_fd();

    let saved_fd = unsafe { libc_dup(1) };
    assert!(saved_fd >= 0, "dup failed");

    let r = unsafe { libc_dup2(tmp_fd, 1) };
    assert!(r >= 0, "dup2 failed");

    f();

    // Flush libc stdout (and our stdout).
    unsafe {
        libc_flush_stdout();
    }

    // Restore stdout.
    let r = unsafe { libc_dup2(saved_fd, 1) };
    assert!(r >= 0, "dup2 restore failed");
    unsafe { libc_close(saved_fd) };

    let mut tmp = tmp;
    tmp.seek(SeekFrom::Start(0)).unwrap();
    let mut buf = Vec::new();
    tmp.read_to_end(&mut buf).unwrap();
    buf
}

fn tempfile_open() -> std::fs::File {
    // Use std::fs to create a temp file.
    let dir = std::env::temp_dir();
    let pid = std::process::id();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = dir.join(format!("longtest-{}-{}.out", pid, nanos));
    std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .unwrap()
}

// Minimal libc hooks via FFI to avoid pulling in the libc crate.
unsafe extern "C" {
    fn dup(fd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut core::ffi::c_void) -> c_int;
}

#[allow(non_snake_case)]
unsafe fn libc_dup(fd: c_int) -> c_int {
    unsafe { dup(fd) }
}
unsafe fn libc_dup2(o: c_int, n: c_int) -> c_int {
    unsafe { dup2(o, n) }
}
unsafe fn libc_close(fd: c_int) -> c_int {
    unsafe { close(fd) }
}
unsafe fn libc_flush_stdout() {
    unsafe {
        // Passing NULL flushes all open streams.
        fflush(core::ptr::null_mut());
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn test_perform_expensive_operations_matches() {
    let _guard = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    // Initialize identical inputs in both libraries' `array` symbols,
    // call perform_expensive_operations on each, and compare results.
    let c_lib = unsafe { load_lib(&c_lib_path()) };
    let r_lib = unsafe { load_lib(&rust_lib_path()) };

    let c_arr_ptr = unsafe { array_ptr(&c_lib) };
    let r_arr_ptr = unsafe { array_ptr(&r_lib) };

    // Build a deterministic input.
    let input: Vec<c_int> = (0..ARRAY_SIZE)
        .map(|i| (i as u32).wrapping_mul(2654435761) as i32)
        .collect();

    write_array(c_arr_ptr, &input);
    write_array(r_arr_ptr, &input);

    let c_fn = unsafe { get_perform_expensive(&c_lib) };
    let r_fn = unsafe { get_perform_expensive(&r_lib) };

    unsafe {
        c_fn();
        r_fn();
    }

    let c_out = read_array(c_arr_ptr, ARRAY_SIZE);
    let r_out = read_array(r_arr_ptr, ARRAY_SIZE);

    assert_eq!(c_out.len(), r_out.len());
    for i in 0..ARRAY_SIZE {
        assert_eq!(
            c_out[i], r_out[i],
            "mismatch at index {}: c={} rust={}",
            i, c_out[i], r_out[i]
        );
    }
}

#[test]
fn test_perform_expensive_operations_multiple_iterations() {
    let _guard = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    // Run the same inner kernel a handful of times. This exercises the
    // composition of the operation (the data the second call sees is
    // produced by the first call) without invoking long_exec's full
    // 2000-iteration cost.
    let c_lib = unsafe { load_lib(&c_lib_path()) };
    let r_lib = unsafe { load_lib(&rust_lib_path()) };

    let c_arr_ptr = unsafe { array_ptr(&c_lib) };
    let r_arr_ptr = unsafe { array_ptr(&r_lib) };

    // Use a varied seed-like input including negative and edge values.
    let input: Vec<c_int> = (0..ARRAY_SIZE)
        .map(|i| {
            let v = (i as i64) * 1103515245 + 12345;
            v as i32
        })
        .collect();

    write_array(c_arr_ptr, &input);
    write_array(r_arr_ptr, &input);

    let c_fn = unsafe { get_perform_expensive(&c_lib) };
    let r_fn = unsafe { get_perform_expensive(&r_lib) };

    for _ in 0..3 {
        unsafe {
            c_fn();
            r_fn();
        }
    }

    let c_out = read_array(c_arr_ptr, ARRAY_SIZE);
    let r_out = read_array(r_arr_ptr, ARRAY_SIZE);

    for i in 0..ARRAY_SIZE {
        assert_eq!(
            c_out[i], r_out[i],
            "mismatch at index {}: c={} rust={}",
            i, c_out[i], r_out[i]
        );
    }
}

// long_exec runs 2000 outer iterations and is extremely expensive
// (≈285s in release mode each). Marked #[ignore] so it doesn't run by
// default; opt-in with `cargo test -- --ignored --test-threads=1`.
#[test]
#[ignore]
fn test_long_exec_stdout_matches() {
    let _guard = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    // long_exec runs 2000 iterations × 256K elements × 100 inner ops.
    // Capture stdout from both implementations and compare.
    let c_lib = unsafe { load_lib(&c_lib_path()) };
    let r_lib = unsafe { load_lib(&rust_lib_path()) };

    let c_long = unsafe { get_long_exec(&c_lib) };
    let r_long = unsafe { get_long_exec(&r_lib) };

    let seed: c_uint = 12345;

    let c_out = capture_stdout(|| unsafe { c_long(seed) });
    let r_out = capture_stdout(|| unsafe { r_long(seed) });

    assert_eq!(c_out, r_out, "stdout differs");
    // Ensure something was actually printed (printf("%d\n", ...))
    assert!(!c_out.is_empty(), "no output captured");
}
