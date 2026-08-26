// Shared differential-test harness.
//
// Both the C shared library and the Rust cdylib are loaded with `libloading`
// and every function is reached through `dlsym`. The Rust crate is NEVER linked
// or called directly, so these tests exercise the real `#[no_mangle]` /
// `extern "C"` export wrappers exactly as an external C consumer would.
//
// All five entry points are `void`-returning and communicate only by writing to
// stdout via libc `printf`/`puts`, so "identical behaviour" is observed by
// redirecting fd 1 to a temp file around each call group and comparing the raw
// bytes.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, CString};
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

/// The five exported symbols, as raw C function pointers pulled out via `dlsym`.
pub struct Lib {
    pub name: &'static str,
    pub print_line: unsafe extern "C" fn(*const c_char),
    pub print_int_line: unsafe extern "C" fn(c_int),
    pub bad: unsafe extern "C" fn(),
    pub good: unsafe extern "C" fn(),
    pub driver: unsafe extern "C" fn(c_int),
}

impl Lib {
    fn load(name: &'static str, path: &PathBuf) -> Lib {
        assert!(
            path.exists(),
            "shared library not found: {}\n\
             build the C side with:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .\n\
             build the Rust side with:\n  cargo build",
            path.display()
        );

        // Leaked on purpose: the raw fn pointers below stay valid for the whole
        // test process and the library must never be unloaded.
        let lib = unsafe { libloading::Library::new(path) }
            .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()));
        let lib: &'static libloading::Library = Box::leak(Box::new(lib));

        unsafe fn sym<T: Copy>(lib: &libloading::Library, n: &[u8]) -> T {
            let s: libloading::Symbol<T> = lib
                .get(n)
                .unwrap_or_else(|e| panic!("dlsym({}) failed: {e}", String::from_utf8_lossy(n)));
            *s
        }

        unsafe {
            Lib {
                name,
                print_line: sym(lib, b"printLine\0"),
                print_int_line: sym(lib, b"printIntLine\0"),
                bad: sym(lib, b"bad\0"),
                good: sym(lib, b"good\0"),
                driver: sym(lib, b"driver\0"),
            }
        }
    }
}

pub struct Libs {
    pub c: Lib,
    pub rust: Lib,
}

/// Loads both libraries once per test process.
pub fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = manifest.join("c_src/build/libdriver.so");

        // target/<profile>/deps/<test-exe>  ->  target/<profile>/libdriver.so
        let exe = std::env::current_exe().expect("current_exe");
        let rust_path = exe
            .parent()
            .and_then(|p| p.parent())
            .expect("target/<profile>")
            .join("libdriver.so");

        // CRITICAL: `cargo test` does NOT rebuild a cdylib-only lib target,
        // because the integration tests dlopen it instead of linking it. Without
        // this guard the whole suite happily validates a stale `.so` and every
        // test passes vacuously (verified: 10/10 deliberately injected bugs went
        // undetected against a stale artifact).
        assert_fresh(&rust_path, &manifest.join("src"), &["rs"], "cargo build");
        assert_fresh(
            &c_path,
            &manifest.join("c_src"),
            &["c", "h"],
            "cd c_src/build && cmake --build .",
        );

        Libs {
            c: Lib::load("C", &c_path),
            rust: Lib::load("Rust", &rust_path),
        }
    })
}

/// Panics if `artifact` is older than any source file under `src_dir` with one
/// of `exts`, i.e. if the shared library under test is out of date.
fn assert_fresh(artifact: &PathBuf, src_dir: &PathBuf, exts: &[&str], rebuild_cmd: &str) {
    let art = std::fs::metadata(artifact)
        .and_then(|m| m.modified())
        .unwrap_or_else(|e| panic!("stat {}: {e}", artifact.display()));

    let mut newest: Option<(PathBuf, std::time::SystemTime)> = None;
    let mut stack = vec![src_dir.clone()];
    while let Some(dir) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else { continue };
        for entry in rd.flatten() {
            let p = entry.path();
            if p.is_dir() {
                // Skip build outputs; only real sources matter.
                if p.file_name().is_some_and(|n| n == "build") {
                    continue;
                }
                stack.push(p);
            } else if p
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| exts.contains(&e))
            {
                if let Ok(t) = entry.metadata().and_then(|m| m.modified()) {
                    if newest.as_ref().is_none_or(|(_, best)| t > *best) {
                        newest = Some((p, t));
                    }
                }
            }
        }
    }

    if let Some((path, t)) = newest {
        assert!(
            art >= t,
            "STALE ARTIFACT — refusing to run: the tests would validate an \
             out-of-date library and pass vacuously.\n  \
             artifact: {}\n  newer source: {}\n  \
             rebuild with: {rebuild_cmd}\n  \
             (note: `cargo test` alone does NOT rebuild a cdylib-only lib target)",
            artifact.display(),
            path.display(),
        );
    }
}

// Redirecting the process-wide fd 1 is inherently global, so captures are
// serialized even though `cargo test` runs test fns on multiple threads.
static CAPTURE_LOCK: Mutex<()> = Mutex::new(());
static SEQ: AtomicU64 = AtomicU64::new(0);

/// Runs `f` with fd 1 redirected to a temp file and returns the exact bytes it
/// wrote. Flushes libc stdio on both sides of the swap so that output produced
/// through the C `FILE *stdout` (shared by both `.so`s) is attributed correctly.
pub fn capture<F: FnOnce()>(f: F) -> Vec<u8> {
    let _guard = CAPTURE_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let mut path = std::env::temp_dir();
    path.push(format!(
        "driver_capture_{}_{}.bin",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::SeqCst)
    ));
    let c_path = CString::new(path.to_str().expect("utf-8 temp path")).unwrap();

    unsafe {
        // Don't let previously buffered output land in our capture file.
        let _ = std::io::stdout().flush();
        libc::fflush(std::ptr::null_mut());

        let saved = libc::dup(1);
        assert!(saved >= 0, "dup(1) failed");

        let fd = libc::open(
            c_path.as_ptr(),
            libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC,
            0o600,
        );
        assert!(fd >= 0, "open({}) failed", path.display());
        assert!(libc::dup2(fd, 1) >= 0, "dup2 failed");

        f();

        // Flush what the library buffered, then restore the real stdout.
        libc::fflush(std::ptr::null_mut());
        libc::dup2(saved, 1);
        libc::close(saved);
        libc::close(fd);
    }

    let data = std::fs::read(&path)
        .unwrap_or_else(|e| panic!("reading capture {}: {e}", path.display()));
    let _ = std::fs::remove_file(&path);

    detect_leak(&data);
    data
}

/// Fails loudly if libtest's progress output ended up inside a capture window.
///
/// Without this, concurrent libtest output shows up as a bogus "DIVERGENCE"
/// where the C side has extra bytes like `test cfg_16_bad_single ... `, which
/// looks like a translation bug but is purely a harness artifact. This is a
/// detector, never a filter: it panics rather than scrubbing the bytes, so it
/// cannot hide a genuine mismatch.
fn detect_leak(data: &[u8]) {
    const SIGNATURES: [&[u8]; 4] = [b"test result:", b"\ntest ", b"running ", b" ... "];
    for sig in SIGNATURES {
        if data.windows(sig.len()).any(|w| w == sig) {
            panic!(
                "test-harness output leaked into a capture window (found {:?}).\n\
                 The captured bytes are not trustworthy, so no comparison was made.\n\
                 Captures redirect the process-wide fd 1 and therefore require a\n\
                 single test thread. Run with RUST_TEST_THREADS=1 (set for you in\n\
                 .cargo/config.toml) or `cargo test -- --test-threads=1`.",
                String::from_utf8_lossy(sig),
            );
        }
    }
}

/// Runs the same closure against the C lib and the Rust lib and asserts the
/// captured stdout bytes are identical.
pub fn assert_same<F>(what: &str, mut run: F)
where
    F: FnMut(&Lib),
{
    let l = libs();
    let c_out = capture(|| run(&l.c));
    let r_out = capture(|| run(&l.rust));

    if c_out != r_out {
        panic!(
            "DIVERGENCE in {what}\n  C    ({:>6} bytes): {}\n  Rust ({:>6} bytes): {}\n  first diff at byte {}",
            c_out.len(),
            pretty(&c_out),
            r_out.len(),
            pretty(&r_out),
            first_diff(&c_out, &r_out)
                .map(|i| i.to_string())
                .unwrap_or_else(|| "n/a (length only)".into()),
        );
    }
}

fn first_diff(a: &[u8], b: &[u8]) -> Option<usize> {
    a.iter().zip(b.iter()).position(|(x, y)| x != y)
}

/// Escaped, length-capped rendering so failures stay readable for 1 MiB inputs.
fn pretty(b: &[u8]) -> String {
    const MAX: usize = 200;
    let mut s = String::new();
    for &x in b.iter().take(MAX) {
        match x {
            b'\n' => s.push_str("\\n"),
            b'\r' => s.push_str("\\r"),
            b'\t' => s.push_str("\\t"),
            0x20..=0x7e => s.push(x as char),
            _ => s.push_str(&format!("\\x{x:02x}")),
        }
    }
    if b.len() > MAX {
        s.push_str(&format!("... (+{} more)", b.len() - MAX));
    }
    format!("\"{s}\"")
}

/// Deterministic SplitMix64 so every randomized row is reproducible.
pub struct Rng(u64);

impl Rng {
    pub const SEED: u64 = 0x243F_6A88_85A3_08D3;

    pub fn new() -> Rng {
        Rng(Self::SEED)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    pub fn next_i32(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }

    /// Uniform in `0..n`.
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }

    /// A non-NUL byte, so it can safely appear inside a C string.
    pub fn nonzero_byte(&mut self) -> u8 {
        (self.below(255) + 1) as u8
    }
}

/// NUL-terminates `bytes` (which must not contain an interior NUL).
pub fn cstr(bytes: &[u8]) -> Vec<u8> {
    assert!(!bytes.contains(&0), "interior NUL");
    let mut v = Vec::with_capacity(bytes.len() + 1);
    v.extend_from_slice(bytes);
    v.push(0);
    v
}
