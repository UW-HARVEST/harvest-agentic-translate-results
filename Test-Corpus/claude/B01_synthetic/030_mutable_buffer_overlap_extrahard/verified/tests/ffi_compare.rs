// Integration tests: load BOTH the C .so and the Rust .so via libloading and
// compare the outputs (return values, mutated buffers, and stdout bytes) of
// the exported FFI symbols.

use libloading::{Library, Symbol};
use std::ffi::OsStr;
use std::os::raw::c_int;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Library locations — both libraries are built as part of the test setup.
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    manifest_dir().join("c_src/build/libdriver_c.so")
}

fn rust_so_path() -> PathBuf {
    // Cargo sets OUT_DIR per-package; the cdylib lands in target/<profile>/.
    // We rely on the standard cargo layout.
    let mut p = manifest_dir();
    p.push("target");
    // tests run with the same profile cargo is invoked with; check both.
    let candidates = ["debug", "release"];
    for c in &candidates {
        let mut q = p.clone();
        q.push(c);
        q.push("libdriver.so");
        if q.exists() {
            return q;
        }
    }
    panic!("could not find libdriver.so in target/{{debug,release}}");
}

fn ensure_libs_built() {
    let c = c_so_path();
    assert!(
        c.exists(),
        "C shared lib not built at {:?}. Run: gcc -shared -fPIC -o c_src/build/libdriver_c.so c_src/src/main.c",
        c
    );
    let r = rust_so_path();
    assert!(r.exists(), "Rust shared lib not found at {:?}", r);
}

// Open a library and keep it alive for the duration of the call.
unsafe fn load_lib<P: AsRef<OsStr>>(p: P) -> Library {
    Library::new(p).expect("failed to dlopen library")
}

// ---------------------------------------------------------------------------
// Stdout capture: redirect fd 1 to a temp file, run a closure, restore.
// Both libraries' printf goes through the C stdio stream, so this captures
// output from either .so identically.
// ---------------------------------------------------------------------------

mod stdout_capture {
    use std::ffi::CString;
    use std::fs;
    use std::os::raw::{c_char, c_int};
    use std::sync::Mutex;

    extern "C" {
        fn dup(fd: c_int) -> c_int;
        fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
        fn close(fd: c_int) -> c_int;
        fn fflush(stream: *mut std::os::raw::c_void) -> c_int;
        fn open(path: *const c_char, flags: c_int, mode: c_int) -> c_int;
    }

    const O_WRONLY: c_int = 1;
    const O_CREAT: c_int = 64; // Linux x86_64
    const O_TRUNC: c_int = 512; // Linux x86_64

    // Serialize stdout-redirecting captures across the whole test binary so
    // the test harness's own writes (test names, ok/FAILED markers) and
    // concurrent test threads cannot leak into our captured file.
    static CAPTURE_LOCK: Mutex<()> = Mutex::new(());

    pub fn capture<F: FnOnce()>(f: F) -> Vec<u8> {
        let _guard = CAPTURE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        unsafe {
            // Flush any pending stdout from prior code paths.
            fflush(std::ptr::null_mut());

            // Make a unique temp filename in /tmp.
            let pid = std::process::id();
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.subsec_nanos())
                .unwrap_or(0);
            let path = format!("/tmp/ffi_capture_{}_{}.txt", pid, nanos);
            let cpath = CString::new(path.clone()).unwrap();

            // Save fd 1 and redirect to file.
            let saved = dup(1);
            assert!(saved >= 0, "dup(1) failed");
            let fd = open(cpath.as_ptr(), O_WRONLY | O_CREAT | O_TRUNC, 0o644);
            assert!(fd >= 0, "open temp failed");
            assert!(dup2(fd, 1) >= 0, "dup2 failed");
            close(fd);

            // Run closure (writes go to file).
            f();

            // Make sure C stdio buffer is flushed before we restore fd 1.
            fflush(std::ptr::null_mut());

            // Restore stdout.
            dup2(saved, 1);
            close(saved);

            // Read captured bytes.
            let bytes = fs::read(&path).unwrap_or_default();
            let _ = fs::remove_file(&path);
            bytes
        }
    }
}

// ---------------------------------------------------------------------------
// fma_array tests
// ---------------------------------------------------------------------------

type FmaArrayFn = unsafe extern "C" fn(
    *mut c_int,
    *const c_int,
    *const c_int,
    *const c_int,
    c_int,
);

fn run_fma(
    lib: &Library,
    out: &mut [c_int],
    mul1: &[c_int],
    mul2: &[c_int],
    add: &[c_int],
) {
    unsafe {
        let f: Symbol<FmaArrayFn> = lib.get(b"fma_array").expect("fma_array symbol");
        f(
            out.as_mut_ptr(),
            mul1.as_ptr(),
            mul2.as_ptr(),
            add.as_ptr(),
            out.len() as c_int,
        );
    }
}

fn compare_fma_case(
    name: &str,
    mul1: &[c_int],
    mul2: &[c_int],
    add: &[c_int],
) {
    assert_eq!(mul1.len(), mul2.len());
    assert_eq!(mul1.len(), add.len());
    let n = mul1.len();
    let c_lib = unsafe { load_lib(c_so_path()) };
    let r_lib = unsafe { load_lib(rust_so_path()) };

    let mut c_out = vec![0i32; n];
    let mut r_out = vec![0i32; n];

    run_fma(&c_lib, &mut c_out, mul1, mul2, add);
    run_fma(&r_lib, &mut r_out, mul1, mul2, add);

    assert_eq!(c_out, r_out, "fma_array mismatch in case '{}'", name);
}

#[test]
fn test_fma_array_zero_len() {
    ensure_libs_built();
    compare_fma_case("zero_len", &[], &[], &[]);
}

#[test]
fn test_fma_array_basic() {
    ensure_libs_built();
    compare_fma_case(
        "basic",
        &[1, 2, 3, 4, 5],
        &[10, 20, 30, 40, 50],
        &[100, 200, 300, 400, 500],
    );
}

#[test]
fn test_fma_array_signed_negatives() {
    ensure_libs_built();
    compare_fma_case(
        "negatives",
        &[-1, -2, -3, 4, -5],
        &[6, -7, 8, -9, 10],
        &[-11, 12, -13, 14, -15],
    );
}

#[test]
fn test_fma_array_overflow_wrap() {
    ensure_libs_built();
    // i32::MAX * 2 overflows; should wrap consistently in both.
    let big = i32::MAX;
    let small = i32::MIN;
    compare_fma_case(
        "overflow_wrap",
        &[big, small, big, small, 65535, -65535],
        &[2, 2, big, small, 65535, 65535],
        &[1, -1, 0, 0, 1, -1],
    );
}

#[test]
fn test_fma_array_full_100() {
    ensure_libs_built();
    let mul1: Vec<i32> = (0..100).collect();
    let mul2: Vec<i32> = (0..100).map(|i| i * 3 - 7).collect();
    let add: Vec<i32> = (0..100).map(|i| (i * i) - 50).collect();
    compare_fma_case("full_100", &mul1, &mul2, &add);
}

#[test]
fn test_fma_array_aliased_all_same() {
    ensure_libs_built();
    // Replicate the call pattern used by `driver`: out, mul1, mul2, add all
    // share the same buffer. We compare by passing the same pointer four
    // times to each library and checking the in-place result matches.
    let initial: Vec<i32> = vec![1, 2, 3, -4, 5, -6, 7, 8, 9, -10];
    let n = initial.len();

    let c_lib = unsafe { load_lib(c_so_path()) };
    let r_lib = unsafe { load_lib(rust_so_path()) };

    let mut c_buf = initial.clone();
    let mut r_buf = initial.clone();

    unsafe {
        let f_c: Symbol<FmaArrayFn> = c_lib.get(b"fma_array").unwrap();
        let f_r: Symbol<FmaArrayFn> = r_lib.get(b"fma_array").unwrap();
        f_c(
            c_buf.as_mut_ptr(),
            c_buf.as_ptr(),
            c_buf.as_ptr(),
            c_buf.as_ptr(),
            n as c_int,
        );
        f_r(
            r_buf.as_mut_ptr(),
            r_buf.as_ptr(),
            r_buf.as_ptr(),
            r_buf.as_ptr(),
            n as c_int,
        );
    }

    assert_eq!(c_buf, r_buf, "aliased fma_array mismatch");
}

// ---------------------------------------------------------------------------
// driver tests
// ---------------------------------------------------------------------------

type DriverFn = unsafe extern "C" fn(*mut c_int, c_int);

fn run_driver_capture(lib: &Library, input: &[c_int]) -> (Vec<i32>, Vec<u8>) {
    let mut buf = input.to_vec();
    let n = buf.len();
    let stdout_bytes = stdout_capture::capture(|| unsafe {
        let f: Symbol<DriverFn> = lib.get(b"driver").expect("driver symbol");
        f(buf.as_mut_ptr(), n as c_int);
    });
    (buf, stdout_bytes)
}

fn compare_driver_case(name: &str, input: &[c_int]) {
    let c_lib = unsafe { load_lib(c_so_path()) };
    let r_lib = unsafe { load_lib(rust_so_path()) };

    // Run C first, then Rust — order shouldn't matter, but be consistent.
    let (c_buf, c_out) = run_driver_capture(&c_lib, input);
    let (r_buf, r_out) = run_driver_capture(&r_lib, input);

    assert_eq!(c_buf, r_buf, "driver buffer mismatch in case '{}'", name);
    assert_eq!(
        c_out, r_out,
        "driver stdout mismatch in case '{}': C={:?} Rust={:?}",
        name,
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&r_out)
    );
}

#[test]
fn test_driver_zero_len() {
    ensure_libs_built();
    compare_driver_case("driver_zero", &[]);
}

#[test]
fn test_driver_small() {
    ensure_libs_built();
    compare_driver_case("driver_small", &[1, 2, 3, -4, 5]);
}

#[test]
fn test_driver_negatives_and_zero() {
    ensure_libs_built();
    compare_driver_case("driver_neg_zero", &[0, -1, 1, -2, 2, 0, 0, -100, 100]);
}

#[test]
fn test_driver_overflow_inputs() {
    ensure_libs_built();
    // After fma: x*x + x. For i32::MAX, this overflows; both should wrap the
    // same way (defined as wrap because the C is compiled with -fwrapv-like
    // behavior on x86_64 GCC for our purposes; if not, results still match
    // because both share the same compiler/codegen overflow semantics).
    compare_driver_case(
        "driver_overflow",
        &[i32::MAX, i32::MIN, 65535, -65535, 1_000_000, -1_000_000],
    );
}

#[test]
fn test_driver_full_100() {
    ensure_libs_built();
    let inp: Vec<i32> = (-50..50).collect();
    compare_driver_case("driver_full_100", &inp);
}
