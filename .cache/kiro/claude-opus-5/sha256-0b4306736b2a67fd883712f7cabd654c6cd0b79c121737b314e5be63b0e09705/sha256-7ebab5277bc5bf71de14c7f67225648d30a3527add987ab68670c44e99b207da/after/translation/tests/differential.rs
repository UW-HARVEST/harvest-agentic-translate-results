//! Differential test: loads BOTH the C `libString_Slice.so` and the Rust
//! `libString_Slice.so` through `libloading` and compares the exported
//! `slice` symbol's return value *and* everything it writes to stdout.
//!
//! Nothing is called directly in-process; every invocation goes through the
//! dynamic-symbol boundary, so the `#[no_mangle]` wrapper is under test too.

use std::ffi::{CString, c_char, c_int};
use std::path::PathBuf;
use std::sync::Mutex;

use libloading::{Library, Symbol};

type SliceFn = unsafe extern "C" fn(*mut c_char, *mut c_int, *mut c_int) -> c_int;

/// stdout redirection is process-global, so only one call may be in flight.
static STDOUT_LOCK: Mutex<()> = Mutex::new(());

fn c_so() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p.join("c_src/build/libString_Slice.so")
}

fn rust_so() -> PathBuf {
    // The integration-test executable lives in target/<profile>/deps/,
    // so the cdylib is two levels up from the test binary.
    let mut p = PathBuf::from(std::env::current_exe().expect("current_exe"));
    p.pop();
    if p.ends_with("deps") {
        p.pop();
    }
    p.join("libString_Slice.so")
}

/// `cargo test` does not build the `cdylib` artifact, only the test binaries,
/// so make sure a fresh one exists before we dlopen it.
fn ensure_rust_so_built() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let profile_release = rust_so().to_string_lossy().contains("/release/");
        let mut cmd = std::process::Command::new(
            std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()),
        );
        cmd.current_dir(env!("CARGO_MANIFEST_DIR")).arg("build").arg("--lib");
        if profile_release {
            cmd.arg("--release");
        }
        // Mirror the feature selection the test binary itself was built with.
        cmd.arg("--no-default-features");
        let feats = enabled_features();
        if !feats.is_empty() {
            cmd.arg("--features").arg(feats.join(","));
        }
        match cmd.output() {
            Ok(o) if o.status.success() => {}
            Ok(o) => eprintln!(
                "warning: rebuilding cdylib failed:\n{}",
                String::from_utf8_lossy(&o.stderr)
            ),
            Err(e) => eprintln!("warning: could not run cargo: {e}"),
        }
    });
}

/// Features active in this test binary. The crate currently declares none,
/// but keeping this here means new features are picked up automatically.
fn enabled_features() -> Vec<&'static str> {
    #[allow(unused_mut)]
    let mut v: Vec<&'static str> = Vec::new();
    v
}

struct Impls {
    _c_lib: Library,
    _rust_lib: Library,
    c: SliceFn,
    rust: SliceFn,
}

impl Impls {
    fn load() -> Impls {
        ensure_rust_so_built();
        let c_path = c_so();
        let r_path = rust_so();
        assert!(c_path.exists(), "missing C library: {}", c_path.display());
        assert!(
            r_path.exists(),
            "missing Rust cdylib: {} (run `cargo build` first)",
            r_path.display()
        );

        unsafe {
            let c_lib = Library::new(&c_path).expect("dlopen C .so");
            let rust_lib = Library::new(&r_path).expect("dlopen Rust .so");
            let c: Symbol<SliceFn> = c_lib.get(b"slice\0").expect("C `slice` symbol");
            let rust: Symbol<SliceFn> = rust_lib.get(b"slice\0").expect("Rust `slice` symbol");
            let c = *c;
            let rust = *rust;
            Impls {
                _c_lib: c_lib,
                _rust_lib: rust_lib,
                c,
                rust,
            }
        }
    }
}

/// Run `f` with fd 1 redirected to a temp file; return (retval, stdout bytes).
fn capture<F: FnOnce() -> c_int>(f: F) -> (c_int, Vec<u8>) {
    let _guard = STDOUT_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let path = std::env::temp_dir().join(format!(
        "slice_capture_{}_{:?}.txt",
        std::process::id(),
        std::thread::current().id()
    ));
    let cpath = CString::new(path.to_str().unwrap()).unwrap();

    unsafe {
        // Flush anything already buffered so it lands on the real stdout.
        libc::fflush(std::ptr::null_mut());

        let saved = libc::dup(1);
        assert!(saved >= 0, "dup(1) failed");

        let fd = libc::open(
            cpath.as_ptr(),
            libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC,
            0o600,
        );
        assert!(fd >= 0, "open temp file failed");
        assert!(libc::dup2(fd, 1) >= 0, "dup2 failed");

        let rc = f();

        libc::fflush(std::ptr::null_mut());
        libc::dup2(saved, 1);
        libc::close(saved);

        libc::lseek(fd, 0, libc::SEEK_SET);
        let mut out = Vec::new();
        let mut chunk = [0u8; 8192];
        loop {
            let n = libc::read(fd, chunk.as_mut_ptr() as *mut libc::c_void, chunk.len());
            if n <= 0 {
                break;
            }
            out.extend_from_slice(&chunk[..n as usize]);
        }
        libc::close(fd);
        libc::unlink(cpath.as_ptr());

        (rc, out)
    }
}

/// Invoke one implementation with a fresh copy of the inputs.
fn invoke(
    f: SliceFn,
    s: &[u8],
    start: Option<c_int>,
    stop: Option<c_int>,
) -> (c_int, Vec<u8>, Option<c_int>, Option<c_int>, Vec<u8>) {
    let mut buf: Vec<u8> = s.to_vec();
    buf.push(0); // NUL terminator
    let mut st = start.unwrap_or(0);
    let mut sp = stop.unwrap_or(0);

    let start_ptr = if start.is_some() {
        &mut st as *mut c_int
    } else {
        std::ptr::null_mut()
    };
    let stop_ptr = if stop.is_some() {
        &mut sp as *mut c_int
    } else {
        std::ptr::null_mut()
    };

    let bufptr = buf.as_mut_ptr() as *mut c_char;
    let (rc, out) = capture(|| unsafe { f(bufptr, start_ptr, stop_ptr) });

    (
        rc,
        out,
        start.map(|_| st),
        stop.map(|_| sp),
        buf, // check the input string was not mutated differently
    )
}

fn describe(s: &[u8], start: Option<c_int>, stop: Option<c_int>) -> String {
    format!(
        "str={:?} (len={}) start={:?} stop={:?}",
        String::from_utf8_lossy(s),
        s.len(),
        start,
        stop
    )
}

fn check(impls: &Impls, s: &[u8], start: Option<c_int>, stop: Option<c_int>) {
    let c = invoke(impls.c, s, start, stop);
    let r = invoke(impls.rust, s, start, stop);

    let ctx = describe(s, start, stop);
    assert_eq!(c.0, r.0, "return value mismatch: {ctx}");
    assert_eq!(
        c.1,
        r.1,
        "stdout mismatch: {ctx}\n  C   = {:?}\n  Rust= {:?}",
        String::from_utf8_lossy(&c.1),
        String::from_utf8_lossy(&r.1)
    );
    assert_eq!(c.2, r.2, "start out-param mismatch: {ctx}");
    assert_eq!(c.3, r.3, "stop out-param mismatch: {ctx}");
    assert_eq!(c.4, r.4, "input buffer mutation mismatch: {ctx}");
}

// ---------------------------------------------------------------------------
// Test cases, lowest level first.
//
// `slice` is the only exported symbol, so everything is exercised through it.
// All checks live in a single #[test] on purpose: the capture redirects the
// process-wide fd 1, and libtest's own progress output ("test foo ... ok")
// would otherwise be written into the captured buffer by a concurrent test.
// ---------------------------------------------------------------------------

fn case_exhaustive_small_strings(impls: &Impls) {
    let strings: &[&[u8]] = &[
        b"",
        b"a",
        b"ab",
        b"abc",
        b"hello, world",
        b"0123456789",
        b"  leading and trailing  ",
        b"tab\there",
        b"pct%sfmt%d%n", // format specifiers in the *data*
        b"newline\nin\nmiddle",
        &[0xff, 0xfe, 0x80, 0x01, 0x7f],
        b"\xe2\x82\xacuro", // multi-byte UTF-8
    ];

    for s in strings {
        let len = s.len() as c_int;
        // Every start/stop in a window around the string bounds, plus NULL.
        let mut candidates: Vec<Option<c_int>> = vec![None];
        for v in -3..=(len + 3) {
            candidates.push(Some(v));
        }
        for start in &candidates {
            for stop in &candidates {
                check(impls, s, *start, *stop);
            }
        }
    }
}

fn case_extreme_values(impls: &Impls) {
    let extremes = [
        c_int::MIN,
        c_int::MIN + 1,
        -1,
        0,
        1,
        c_int::MAX - 1,
        c_int::MAX,
        i16::MAX as c_int,
        u16::MAX as c_int,
    ];
    let strings: &[&[u8]] = &[b"", b"x", b"abcdefghij"];

    for s in strings {
        for start in extremes {
            for stop in extremes {
                check(impls, s, Some(start), Some(stop));
            }
            check(impls, s, Some(start), None);
        }
        for stop in extremes {
            check(impls, s, None, Some(stop));
        }
        check(impls, s, None, None);
    }
}

fn case_embedded_nul_bytes(impls: &Impls) {
    // strlen stops at the first NUL; both implementations must agree.
    let strings: &[&[u8]] = &[b"abc\0def", b"\0hidden", b"trailing\0", b"a\0\0\0b"];
    for s in strings {
        for start in [None, Some(-1), Some(0), Some(1), Some(2), Some(3), Some(4)] {
            for stop in [None, Some(-1), Some(0), Some(1), Some(2), Some(3), Some(9)] {
                check(impls, s, start, stop);
            }
        }
    }
}

fn case_long_strings(impls: &Impls) {
    for len in [255usize, 256, 1024, 4095, 4096, 4097, 70_000] {
        let s: Vec<u8> = (0..len).map(|i| b'a' + (i % 26) as u8).collect();
        let l = len as c_int;
        check(impls, &s, None, None);
        check(impls, &s, Some(0), Some(l));
        check(impls, &s, Some(l / 2), None);
        check(impls, &s, Some(l / 3), Some(2 * l / 3));
        check(impls, &s, Some(l), None);
        check(impls, &s, Some(l - 1), Some(l));
        check(impls, &s, Some(l), Some(l));
        check(impls, &s, None, Some(l));
        check(impls, &s, Some(-1), None);
        check(impls, &s, None, Some(l + 1));
    }
}

fn case_randomized_fuzz(impls: &Impls) {
    // Deterministic xorshift so failures are reproducible.
    let mut state: u64 = 0x2545_F491_4F6C_DD1D;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };

    let pick = |r: u64, len: usize| -> Option<c_int> {
        match r % 8 {
            0 => None,
            1 => Some(-((r as c_int).rem_euclid(5)) - 1),
            2 => Some(len as c_int),
            3 => Some(len as c_int + 1),
            4 => Some(0),
            5 => Some(c_int::MIN),
            6 => Some(c_int::MAX),
            _ => Some((r % (len as u64 + 1)) as c_int),
        }
    };

    for _ in 0..3000 {
        let len = (next() % 33) as usize;
        // Full byte range including NUL and high bytes.
        let s: Vec<u8> = (0..len).map(|_| (next() % 256) as u8).collect();
        let start = pick(next(), len);
        let stop = pick(next(), len);
        check(impls, &s, start, stop);
    }
}

#[test]
fn c_and_rust_agree() {
    // Loading succeeds only if both .so files export `slice` under the exact
    // same name, so this also covers the #[no_mangle] export wrapper.
    let impls = Impls::load();
    case_exhaustive_small_strings(&impls);
    case_extreme_values(&impls);
    case_embedded_nul_bytes(&impls);
    case_long_strings(&impls);
    case_randomized_fuzz(&impls);
}
