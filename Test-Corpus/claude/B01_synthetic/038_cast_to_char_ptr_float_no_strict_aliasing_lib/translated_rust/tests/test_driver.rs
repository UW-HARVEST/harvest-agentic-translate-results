use libloading::{Library, Symbol};
use std::fs::File;
use std::io::Read;
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::sync::Mutex;

// Stdout redirection is a process-global resource; serialize tests that touch it.
static STDOUT_LOCK: Mutex<()> = Mutex::new(());

type DriverFn = unsafe extern "C" fn(f32);

fn c_lib_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("c_src/build/libdriver.so");
    p
}

fn rust_lib_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // Try debug first, then release.
    let mut debug = p.clone();
    debug.push("target/debug/libdriver.so");
    if debug.exists() {
        return debug;
    }
    p.push("target/release/libdriver.so");
    p
}

/// Capture everything written to stdout (including stdio writes from C printf)
/// during the closure by redirecting fd 1 to a temp file. Returns the bytes.
fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    // Flush rust-side stdout
    use std::io::Write as _;
    std::io::stdout().flush().ok();

    // libc-level flush of stdout buffer too
    unsafe {
        libc::fflush(std::ptr::null_mut());
    }

    // Create a temp file we can read from later
    let tmp_path = format!(
        "/tmp/captured_stdout_{}_{}.txt",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let tmp = File::create(&tmp_path).expect("create temp file");

    // Save original stdout fd
    let orig_stdout = unsafe { libc::dup(1) };
    assert!(orig_stdout >= 0, "dup failed");

    // Redirect stdout to the temp file
    unsafe {
        let new_fd = tmp.as_raw_fd();
        let ret = libc::dup2(new_fd, 1);
        assert!(ret >= 0, "dup2 failed");
    }
    drop(tmp); // close our reference; stdout still points at it via fd 1

    // Run the closure
    f();

    // Flush before restoring
    unsafe {
        libc::fflush(std::ptr::null_mut());
    }
    std::io::stdout().flush().ok();

    // Restore original stdout
    unsafe {
        libc::dup2(orig_stdout, 1);
        libc::close(orig_stdout);
    }

    // Read what was written
    let mut buf = Vec::new();
    File::open(&tmp_path)
        .expect("open temp file for reading")
        .read_to_end(&mut buf)
        .expect("read temp file");
    std::fs::remove_file(&tmp_path).ok();
    buf
}

fn call_driver_capture(lib_path: &std::path::Path, x: f32) -> Vec<u8> {
    unsafe {
        let lib = Library::new(lib_path).unwrap_or_else(|e| {
            panic!("failed to load {:?}: {}", lib_path, e)
        });
        let driver: Symbol<DriverFn> = lib.get(b"driver").expect("symbol driver");
        let out = capture_stdout(|| {
            driver(x);
        });
        // Library is dropped at the end of this block
        let _ = lib;
        out
    }
}

fn check_input(x: f32) {
    let c_path = c_lib_path();
    let r_path = rust_lib_path();
    assert!(c_path.exists(), "C lib not found at {:?}", c_path);
    assert!(r_path.exists(), "Rust lib not found at {:?}", r_path);

    let _g = STDOUT_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let c_out = call_driver_capture(&c_path, x);
    let r_out = call_driver_capture(&r_path, x);
    assert_eq!(
        c_out, r_out,
        "Mismatch for x={:?}: C={:?} Rust={:?}",
        x,
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&r_out),
    );
}

#[test]
fn driver_zero() {
    check_input(0.0_f32);
}

#[test]
fn driver_neg_zero() {
    check_input(-0.0_f32);
}

#[test]
fn driver_one() {
    check_input(1.0_f32);
}

#[test]
fn driver_neg_one() {
    check_input(-1.0_f32);
}

#[test]
fn driver_pi() {
    check_input(std::f32::consts::PI);
}

#[test]
fn driver_inf() {
    check_input(f32::INFINITY);
}

#[test]
fn driver_neg_inf() {
    check_input(f32::NEG_INFINITY);
}

#[test]
fn driver_nan() {
    check_input(f32::NAN);
}

#[test]
fn driver_min_positive() {
    check_input(f32::MIN_POSITIVE);
}

#[test]
fn driver_max() {
    check_input(f32::MAX);
}

#[test]
fn driver_min() {
    check_input(f32::MIN);
}

#[test]
fn driver_epsilon() {
    check_input(f32::EPSILON);
}

#[test]
fn driver_assorted_bits() {
    let bits: [u32; 12] = [
        0x00000001, 0x80000001, 0x7f7fffff, 0xff7fffff, 0x3f800000, 0xbf800000,
        0x40490fdb, 0xdeadbeef, 0xcafebabe, 0x12345678, 0x55555555, 0xaaaaaaaa,
    ];
    for b in bits.iter() {
        let x = f32::from_bits(*b);
        check_input(x);
    }
}

