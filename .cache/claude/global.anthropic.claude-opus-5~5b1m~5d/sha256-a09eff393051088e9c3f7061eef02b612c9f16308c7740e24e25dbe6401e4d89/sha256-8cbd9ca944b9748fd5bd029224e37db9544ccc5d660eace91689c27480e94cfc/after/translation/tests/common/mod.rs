// Shared harness for the C-vs-Rust differential tests.
//
// Both shared objects are loaded with `libloading` and driven ONLY through
// their exported C symbols, so the `#[no_mangle]` export wrappers are part of
// what is under test.
//
// `driver` communicates exclusively through stdout (libc `printf`), so the
// harness redirects file descriptor 1 to a temporary file around each batch of
// calls and reads the bytes back. Both `.so`s resolve `printf` against the same
// libc in this process, so they share the same `FILE *stdout` and the same
// buffering behaviour; `fflush(NULL)` is issued before the descriptor is
// restored.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::io::Read;
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

pub type DriverFn = unsafe extern "C" fn(f32);

/// fd 1 redirection is process-global, so all capturing tests are serialised.
fn capture_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    match LOCK.get_or_init(|| Mutex::new(())).lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Path to a FRESHLY BUILT Rust `cdylib`.
///
/// `cargo test` deliberately does NOT rebuild a `crate-type = ["cdylib"]`
/// target (integration tests cannot link one), so `target/<profile>/libdriver.so`
/// can be arbitrarily stale — which would silently make every differential test
/// pass against an old binary. The harness therefore builds the cdylib itself
/// into a SEPARATE target directory (separate dir => separate build lock, so no
/// deadlock against the cargo invocation that is running these tests).
pub fn rust_so_path() -> &'static PathBuf {
    static SO: OnceLock<PathBuf> = OnceLock::new();
    SO.get_or_init(|| {
        let exe = std::env::current_exe().expect("current_exe");
        let profile = exe
            .parent()
            .and_then(|deps| deps.parent())
            .and_then(|p| p.file_name())
            .map(|s| s.to_string_lossy().into_owned())
            .expect("target/<profile>");

        let target_dir = manifest_dir().join("target/difftest");
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
        let mut cmd = std::process::Command::new(cargo);
        cmd.current_dir(manifest_dir())
            .arg("build")
            .arg("--offline")
            .arg("--lib")
            .arg("--target-dir")
            .arg(&target_dir);
        if profile != "debug" {
            cmd.arg("--profile").arg(&profile);
        }
        // Forward the feature selection of the run under test (set by
        // `run_all_configs.sh`) so every feature combo is honoured.
        if let Ok(extra) = std::env::var("DIFFTEST_CARGO_ARGS") {
            for a in extra.split_whitespace() {
                cmd.arg(a);
            }
        }
        let status = cmd.status().expect("spawn cargo build for the cdylib");
        assert!(status.success(), "failed to build the Rust cdylib under test");

        let p = target_dir.join(&profile).join("libdriver.so");
        assert!(p.exists(), "Rust cdylib not found at {p:?}");

        // Belt and braces: the freshly built artefact must be newer than the source.
        let src = manifest_dir().join("src/lib.rs");
        let (so_m, src_m) = (
            std::fs::metadata(&p).and_then(|m| m.modified()).unwrap(),
            std::fs::metadata(&src).and_then(|m| m.modified()).unwrap(),
        );
        assert!(
            so_m >= src_m,
            "Rust cdylib {p:?} is older than src/lib.rs — stale artefact"
        );
        p
    })
}

pub fn c_so_path() -> PathBuf {
    let p = manifest_dir()
        .parent()
        .expect("workspace root")
        .join("c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C shared library not found at {p:?}; build it with:\n  cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
    );
    // The C .so is the ground truth; refuse to compare against a stale one.
    let src = manifest_dir().parent().unwrap().join("c_src/src/driver.c");
    let so_m = std::fs::metadata(&p).and_then(|m| m.modified()).unwrap();
    let src_m = std::fs::metadata(&src).and_then(|m| m.modified()).unwrap();
    assert!(
        so_m >= src_m,
        "C shared library {p:?} is older than {src:?} — rebuild it with `cmake --build c_src/build`"
    );
    p
}

pub struct Libs {
    pub c: Library,
    pub rust: Library,
}

pub fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| unsafe {
        Libs {
            c: Library::new(c_so_path()).expect("dlopen C .so"),
            rust: Library::new(rust_so_path()).expect("dlopen Rust .so"),
        }
    })
}

pub fn c_driver() -> Symbol<'static, DriverFn> {
    unsafe { libs().c.get(b"driver\0").expect("C driver symbol") }
}

pub fn rust_driver() -> Symbol<'static, DriverFn> {
    unsafe { libs().rust.get(b"driver\0").expect("Rust driver symbol") }
}

/// Runs `body` with fd 1 pointed at a scratch file and returns everything that
/// was written to it.
fn capture<F: FnOnce()>(body: F) -> Vec<u8> {
    let _guard = capture_lock();

    let dir = std::env::temp_dir();
    let path = dir.join(format!(
        "driver-capture-{}-{:?}.bin",
        std::process::id(),
        std::thread::current().id()
    ));
    let file = std::fs::File::create(&path).expect("create capture file");

    // Flush BOTH stdio layers before stealing fd 1, otherwise libtest's own
    // buffered progress text ("test foo ... ", which has no trailing newline
    // and so lingers in Rust's LineWriter) would be flushed into our capture.
    let _ = std::io::Write::flush(&mut std::io::stdout());
    let _ = std::io::Write::flush(&mut std::io::stderr());

    unsafe {
        libc::fflush(std::ptr::null_mut()); // flush anything already pending
        let saved = libc::dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(libc::dup2(file.as_raw_fd(), 1) >= 0, "dup2 failed");

        body();

        let _ = std::io::Write::flush(&mut std::io::stdout());
        libc::fflush(std::ptr::null_mut()); // flush the library's output
        assert!(libc::dup2(saved, 1) >= 0, "restore dup2 failed");
        libc::close(saved);
    }
    drop(file);

    let mut out = Vec::new();
    std::fs::File::open(&path)
        .expect("reopen capture file")
        .read_to_end(&mut out)
        .expect("read capture file");
    let _ = std::fs::remove_file(&path);
    out
}

/// Captures the stdout produced by an arbitrary sequence of library calls.
pub fn run_batch_with<F: FnOnce()>(body: F) -> Vec<u8> {
    capture(body)
}

/// Calls `f(x)` for every `x` and returns the captured stdout bytes.
pub fn run_batch(f: &Symbol<'static, DriverFn>, xs: &[f32]) -> Vec<u8> {
    capture(|| {
        for &x in xs {
            unsafe { f(x) }
        }
    })
}

/// What the C source is specified to print: the 4 bytes of the object
/// representation, ascending address order, `%02x` each, then `\n`.
pub fn expected_line(x: f32) -> Vec<u8> {
    let bits = x.to_bits().to_le_bytes();
    let mut s = String::new();
    for b in bits {
        s.push_str(&format!("{b:02x}"));
    }
    s.push('\n');
    s.into_bytes()
}

pub fn expected_batch(xs: &[f32]) -> Vec<u8> {
    let mut v = Vec::new();
    for &x in xs {
        v.extend_from_slice(&expected_line(x));
    }
    v
}

fn describe(xs: &[f32], byte_index: usize) -> String {
    // every call emits exactly 9 bytes
    let idx = byte_index / 9;
    match xs.get(idx) {
        Some(x) => format!(
            "first divergence at output byte {byte_index} => input #{idx} = {x:?} (bits 0x{:08x})",
            x.to_bits()
        ),
        None => format!("first divergence at output byte {byte_index} (past end of inputs)"),
    }
}

/// The core assertion: C and Rust, both called through their `.so` exports,
/// must produce byte-identical stdout for the same inputs. Also cross-checks
/// against the format derived from the C source.
#[track_caller]
pub fn assert_same(label: &str, xs: &[f32]) {
    let c = c_driver();
    let r = rust_driver();

    let c_out = run_batch(&c, xs);
    let r_out = run_batch(&r, xs);

    if c_out != r_out {
        let at = c_out
            .iter()
            .zip(r_out.iter())
            .position(|(a, b)| a != b)
            .unwrap_or_else(|| c_out.len().min(r_out.len()));
        panic!(
            "[{label}] C/Rust stdout mismatch ({} vs {} bytes): {}\n  C   : {:?}\n  Rust: {:?}",
            c_out.len(),
            r_out.len(),
            describe(xs, at),
            String::from_utf8_lossy(&c_out[at.saturating_sub(9)..(at + 18).min(c_out.len())]),
            String::from_utf8_lossy(&r_out[at.saturating_sub(9)..(at + 18).min(r_out.len())]),
        );
    }

    let expected = expected_batch(xs);
    assert_eq!(
        c_out,
        expected,
        "[{label}] C output does not match the format derived from driver.c \
         (harness bug or unexpected platform)"
    );
    assert_eq!(c_out.len(), xs.len() * 9, "[{label}] unexpected byte count");
}

#[track_caller]
pub fn assert_same_bits(label: &str, bits: &[u32]) {
    let xs: Vec<f32> = bits.iter().copied().map(f32::from_bits).collect();
    assert_same(label, &xs);
}

/// Deterministic xorshift64* PRNG — fixed seed, reproducible across runs.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(if seed == 0 { 0x9e3779b97f4a7c15 } else { seed })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
}
