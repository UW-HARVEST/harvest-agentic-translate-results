//! Differential test: loads the C shared library and the Rust cdylib via
//! `libloading` and compares the observable behaviour of every exported
//! symbol through the FFI boundary.
//!
//! The public API (c_src/include/driver.h) is a single function:
//!
//!     void driver(int x, int y);
//!
//! It has no return value; its entire observable effect is what it writes to
//! stdout (`printf("%d", x | ~y)` followed by `puts("")`). So the comparison
//! is done by capturing file descriptor 1 around each call and comparing the
//! captured bytes.

use std::ffi::{c_int, OsStr};
use std::io::{Read, Seek, SeekFrom};
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use libloading::{Library, Symbol};

/// fd 1 redirection is process-global, so serialize anything that captures it.
static FD_LOCK: Mutex<()> = Mutex::new(());

type DriverFn = unsafe extern "C" fn(c_int, c_int);

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn c_library_path() -> PathBuf {
    let p = workspace_root().join("c_src/build/libdriver.so");
    assert!(
        p.is_file(),
        "C shared library not found at {}. Build it with:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

fn rust_library_path() -> PathBuf {
    // Allow pinning a specific artifact (e.g. the release cdylib).
    if let Ok(p) = std::env::var("DRIVER_RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.is_file(), "DRIVER_RUST_SO={} is not a file", p.display());
        return p;
    }
    // Otherwise prefer whatever profile this test run built, then fall back.
    let target = workspace_root().join("translation/target");
    let candidates = ["debug", "release"];
    let mut found: Option<PathBuf> = None;
    for c in candidates {
        let p = target.join(c).join("libdriver.so");
        if p.is_file() {
            found = Some(p);
            break;
        }
    }
    found.unwrap_or_else(|| {
        panic!(
            "Rust cdylib not found under {}. Build it with `cargo build`.",
            target.display()
        )
    })
}

fn load(path: &Path) -> Library {
    unsafe { Library::new(<&OsStr>::from(path.as_os_str())) }
        .unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", path.display()))
}

/// Run `f` with fd 1 redirected into a temporary file and return the bytes
/// written to it. All C streams are flushed before fd 1 is restored, so
/// buffered `printf`/`puts` output is included.
fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    let _guard = FD_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let mut tmp = tempfile();
    let tmp_fd = tmp.as_raw_fd();

    unsafe {
        // Flush anything already pending so it lands on the real stdout.
        libc::fflush(std::ptr::null_mut());

        let saved = libc::dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(libc::dup2(tmp_fd, 1) >= 0, "dup2 onto stdout failed");

        f();

        // Flush the redirected (now fully-buffered) stdout before restoring.
        libc::fflush(std::ptr::null_mut());
        assert!(libc::dup2(saved, 1) >= 0, "restoring stdout failed");
        libc::close(saved);
    }

    let mut out = Vec::new();
    tmp.seek(SeekFrom::Start(0)).expect("seek temp file");
    tmp.read_to_end(&mut out).expect("read temp file");
    out
}

fn tempfile() -> std::fs::File {
    // O_TMPFILE-free, portable: create-and-unlink in the target dir.
    let dir = std::env::temp_dir();
    let name = format!(
        "driver-difftest-{}-{:?}.tmp",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let path = dir.join(name);
    let file = std::fs::OpenOptions::new()
        .create_new(true)
        .read(true)
        .write(true)
        .open(&path)
        .expect("create temp file");
    // Unlink immediately; the open handle keeps it alive.
    let _ = std::fs::remove_file(&path);
    file
}

/// Interesting `int` values: identities, boundaries, sign-bit patterns, and a
/// deterministic pseudo-random spread.
fn test_values() -> Vec<i32> {
    let mut v = vec![
        0,
        1,
        -1,
        2,
        -2,
        7,
        -7,
        10,
        -10,
        42,
        -42,
        99,
        -99,
        100,
        -100,
        127,
        -128,
        255,
        256,
        -256,
        1023,
        1024,
        32767,
        -32768,
        32768,
        65535,
        65536,
        -65536,
        123456789,
        -123456789,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
        0x5555_5555u32 as i32,
        0xAAAA_AAAAu32 as i32,
        0x7FFF_FFFEu32 as i32,
        0x8000_0001u32 as i32,
        0x0F0F_0F0Fu32 as i32,
        0xF0F0_F0F0u32 as i32,
    ];

    // Deterministic LCG spread over the full int range.
    let mut state: u64 = 0x2545_F491_4F6C_DD1D;
    for _ in 0..64 {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        v.push((state >> 32) as u32 as i32);
    }

    v
}

#[test]
fn driver_matches_c_for_all_inputs() {
    let c_lib = load(&c_library_path());
    let rust_lib = load(&rust_library_path());

    let c_driver: Symbol<DriverFn> =
        unsafe { c_lib.get(b"driver\0") }.expect("C .so must export `driver`");
    let rust_driver: Symbol<DriverFn> =
        unsafe { rust_lib.get(b"driver\0") }.expect("Rust .so must export `driver`");

    let values = test_values();

    // Full cross product of the interesting values.
    let mut cases: Vec<(i32, i32)> = Vec::with_capacity(values.len() * values.len());
    for &x in &values {
        for &y in &values {
            cases.push((x, y));
        }
    }

    let mut checked = 0usize;
    for (x, y) in cases {
        let c_out = capture_stdout(|| unsafe { c_driver(x, y) });
        let rust_out = capture_stdout(|| unsafe { rust_driver(x, y) });

        assert_eq!(
            c_out,
            rust_out,
            "driver({x}, {y}) mismatch:\n  C   : {:?} ({})\n  Rust: {:?} ({})",
            String::from_utf8_lossy(&c_out),
            c_out.len(),
            String::from_utf8_lossy(&rust_out),
            rust_out.len()
        );
        checked += 1;
    }

    assert!(checked > 0, "no cases were checked");
    eprintln!("driver: {checked} input pairs matched byte-for-byte");
}

/// Sanity check that the capture harness actually observes output and that the
/// output is the expected `printf("%d", x | ~y)` + newline. This guards against
/// a false pass where both libraries produce nothing.
#[test]
fn capture_harness_observes_expected_bytes() {
    let c_lib = load(&c_library_path());
    let c_driver: Symbol<DriverFn> =
        unsafe { c_lib.get(b"driver\0") }.expect("C .so must export `driver`");

    for (x, y) in [(0i32, 0i32), (5, 3), (-1, -1), (i32::MIN, i32::MAX)] {
        let out = capture_stdout(|| unsafe { c_driver(x, y) });
        let expected = format!("{}\n", x | !y);
        assert_eq!(
            String::from_utf8_lossy(&out),
            expected,
            "harness did not capture expected bytes for driver({x}, {y})"
        );
    }
}

/// Repeated / interleaved calls: verifies there is no hidden state difference
/// (e.g. static buffers) between the two implementations.
#[test]
fn driver_interleaved_calls_match() {
    let c_lib = load(&c_library_path());
    let rust_lib = load(&rust_library_path());

    let c_driver: Symbol<DriverFn> = unsafe { c_lib.get(b"driver\0") }.unwrap();
    let rust_driver: Symbol<DriverFn> = unsafe { rust_lib.get(b"driver\0") }.unwrap();

    let seq: Vec<(i32, i32)> = (0..200)
        .map(|i| {
            let x = (i * 2_654_435_761u64 as i64) as i32;
            let y = (i * 40_503) as i32 - 1_000_000;
            (x, y)
        })
        .collect();

    // Batch all calls into a single capture per implementation so the whole
    // output stream (including buffering behaviour) is compared at once.
    let c_out = capture_stdout(|| {
        for &(x, y) in &seq {
            unsafe { c_driver(x, y) };
        }
    });
    let rust_out = capture_stdout(|| {
        for &(x, y) in &seq {
            unsafe { rust_driver(x, y) };
        }
    });

    assert_eq!(
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&rust_out),
        "batched output streams differ"
    );
}
