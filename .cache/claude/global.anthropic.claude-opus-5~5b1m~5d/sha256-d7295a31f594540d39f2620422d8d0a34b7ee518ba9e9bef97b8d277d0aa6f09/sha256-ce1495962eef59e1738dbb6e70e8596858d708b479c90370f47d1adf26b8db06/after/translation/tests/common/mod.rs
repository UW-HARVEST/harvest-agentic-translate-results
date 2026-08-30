// Shared differential-testing harness.
//
// BOTH the C `.so` and the Rust `.so` are loaded with `libloading` and every
// call goes through the exported `driver` symbol, so the `#[no_mangle]`
// `extern "C"` wrapper is exercised exactly as an external consumer would.
// Rust functions are never called directly.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::CString;
use std::os::raw::c_int;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

pub type DriverFn = unsafe extern "C" fn(c_int, c_int, c_int);

// ---------------------------------------------------------------------------
// Library discovery + loading
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `c_src/build/libdriver.so`, built by CMake.
pub fn c_so_path() -> PathBuf {
    let p = manifest_dir()
        .parent()
        .expect("crate has a parent directory")
        .join("c_src/build/libdriver.so");
    assert!(
        p.is_file(),
        "C shared library not found at {}\nBuild it with:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    // The C object is the ground truth, so it must not be stale either.
    if let (Ok(so_t), Ok(src_t)) = (
        std::fs::metadata(&p).and_then(|m| m.modified()),
        std::fs::metadata(
            manifest_dir()
                .parent()
                .unwrap()
                .join("c_src/src/driver.c"),
        )
        .and_then(|m| m.modified()),
    ) {
        assert!(
            so_t >= src_t,
            "STALE ARTIFACT: {} is older than c_src/src/driver.c; rebuild the C library.",
            p.display()
        );
    }
    p
}

/// The Rust `cdylib`, from the same profile directory as this test binary.
///
/// IMPORTANT: `cargo test` does **not** necessarily rebuild or re-uplift a
/// `crate-type = ["cdylib"]` library, because no test target links against it.
/// Loading `target/<profile>/libdriver.so` can therefore silently pick up a
/// STALE object and make every differential test pass vacuously. The staleness
/// guard below turns that failure mode into a loud error; `run_all.sh` always
/// runs `cargo build` before `cargo test` to keep it satisfied.
pub fn rust_so_path() -> PathBuf {
    // .../target/<profile>/deps/<test-bin>  ->  .../target/<profile>
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(Path::parent)
        .expect("test binary lives in target/<profile>/deps");

    let mut found = None;
    let direct = profile_dir.join("libdriver.so");
    if direct.is_file() {
        found = Some(direct);
    } else {
        // Fall back to the other profile dir if cargo laid things out differently.
        for cand in ["target/release/libdriver.so", "target/debug/libdriver.so"] {
            let p = manifest_dir().join(cand);
            if p.is_file() {
                found = Some(p);
                break;
            }
        }
    }

    let so = found.unwrap_or_else(|| {
        panic!(
            "Rust cdylib not found (looked in {} and target/{{release,debug}}).\n\
             Build it with: cargo build --release",
            profile_dir.display()
        )
    });

    assert_not_stale(&so);
    so
}

/// Panic if the `.so` is older than any Rust source or the manifest.
fn assert_not_stale(so: &Path) {
    let so_mtime = std::fs::metadata(so)
        .and_then(|m| m.modified())
        .expect("stat the Rust .so");

    let mut newest: Option<(PathBuf, std::time::SystemTime)> = None;
    let mut consider = |p: PathBuf| {
        if let Ok(t) = std::fs::metadata(&p).and_then(|m| m.modified()) {
            if newest.as_ref().is_none_or(|(_, best)| t > *best) {
                newest = Some((p, t));
            }
        }
    };
    consider(manifest_dir().join("Cargo.toml"));
    if let Ok(entries) = std::fs::read_dir(manifest_dir().join("src")) {
        for e in entries.flatten() {
            let p = e.path();
            if p.extension().is_some_and(|x| x == "rs") {
                consider(p);
            }
        }
    }

    if let Some((newest_path, newest_mtime)) = newest {
        assert!(
            so_mtime >= newest_mtime,
            "STALE ARTIFACT: {} is older than {}.\n\
             `cargo test` does not always rebuild a cdylib-only library, so the \
             differential tests would be comparing against an out-of-date \
             object and could pass vacuously.\n\
             Run `./run_all.sh`, or `cargo build --release` before `cargo test --release`.",
            so.display(),
            newest_path.display()
        );
    }
}

fn load(path: &Path) -> Library {
    // SAFETY: loading a well-formed shared object built from this repository.
    unsafe { Library::new(path) }
        .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()))
}

static C_LIB: OnceLock<Library> = OnceLock::new();
static RUST_LIB: OnceLock<Library> = OnceLock::new();

pub fn c_lib() -> &'static Library {
    C_LIB.get_or_init(|| load(&c_so_path()))
}

pub fn rust_lib() -> &'static Library {
    RUST_LIB.get_or_init(|| load(&rust_so_path()))
}

fn driver_sym(lib: &'static Library) -> Symbol<'static, DriverFn> {
    // SAFETY: `driver` has signature `void driver(int, int, int)` in
    // c_src/include/driver.h; `DriverFn` matches it.
    unsafe { lib.get(b"driver\0") }.expect("`driver` symbol must be exported")
}

// ---------------------------------------------------------------------------
// stdout capture
//
// `driver` returns `void`; its only observable effect is the byte stream it
// writes to stdout via libc `printf`/`puts`. The C `.so`, the Rust `.so` and
// this test binary all share one dynamically-linked libc, hence one `stdout`
// FILE object, so redirecting fd 1 here captures both.
// ---------------------------------------------------------------------------

/// fd 1 is process-global, so captures must not overlap.
static CAPTURE_LOCK: Mutex<()> = Mutex::new(());

fn unique_tmp_path() -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "driver_diff_{}_{}_{}.out",
        std::process::id(),
        n,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ))
}

/// Run `f` with fd 1 redirected to a temporary file and return everything
/// written to stdout while it ran.
pub fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    let _guard = CAPTURE_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let path = unique_tmp_path();
    let cpath = CString::new(path.as_os_str().as_encoded_bytes()).expect("no NUL in temp path");

    // Push out anything Rust's own (independently buffered) stdout is holding,
    // so it cannot be stolen into our capture file.
    {
        use std::io::Write;
        let _ = std::io::stdout().flush();
    }

    // SAFETY: plain POSIX fd juggling; every fd used is checked below.
    let captured = unsafe {
        // Same for the C stdio buffers.
        libc::fflush(std::ptr::null_mut());

        let saved = libc::dup(1);
        assert!(saved >= 0, "dup(1) failed");

        let tmp_fd = libc::open(
            cpath.as_ptr(),
            libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC,
            0o600 as libc::c_int,
        );
        assert!(tmp_fd >= 0, "open({}) failed", path.display());

        assert!(libc::dup2(tmp_fd, 1) >= 0, "dup2(tmp, 1) failed");

        f();

        // Flush every stdio stream (passing NULL flushes all of them) so the
        // library's buffered output lands in the file before we look.
        libc::fflush(std::ptr::null_mut());

        assert!(libc::dup2(saved, 1) >= 0, "dup2(saved, 1) failed");
        libc::close(saved);
        libc::close(tmp_fd);

        std::fs::read(&path).expect("read capture file")
    };

    let _ = std::fs::remove_file(&path);
    captured
}

// ---------------------------------------------------------------------------
// Differential drivers
// ---------------------------------------------------------------------------

/// Call `driver` in `lib` once, returning its stdout bytes.
pub fn run_one(lib: &'static Library, x: i32, y: i32, z: i32) -> Vec<u8> {
    let f = driver_sym(lib);
    capture_stdout(|| {
        // SAFETY: signature verified against driver.h.
        unsafe { f(x, y, z) }
    })
}

/// Call `driver` in `lib` once for each triple, in order, returning the
/// concatenated stdout. Used to probe the persistent file-scope `static y`.
pub fn run_seq(lib: &'static Library, calls: &[(i32, i32, i32)]) -> Vec<u8> {
    let f = driver_sym(lib);
    capture_stdout(|| {
        for &(x, y, z) in calls {
            // SAFETY: signature verified against driver.h.
            unsafe { f(x, y, z) }
        }
    })
}

fn show(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).escape_debug().to_string()
}

/// Assert C and Rust produce byte-identical stdout for `driver(x, y, z)`.
/// Returns the (shared) output so callers can additionally pin exact bytes.
pub fn assert_same(x: i32, y: i32, z: i32) -> Vec<u8> {
    let c_out = run_one(c_lib(), x, y, z);
    let r_out = run_one(rust_lib(), x, y, z);
    assert_eq!(
        c_out,
        r_out,
        "stdout mismatch for driver({x}, {y}, {z})\n  C   : \"{}\"\n  Rust: \"{}\"",
        show(&c_out),
        show(&r_out)
    );
    // Cross-check against the model transcribed from the C source.
    let model = expected_output(x, y, z);
    assert_eq!(
        c_out,
        model.as_bytes(),
        "driver({x}, {y}, {z}): C output disagrees with the model derived from \
         c_src/src/driver.c\n  got   : \"{}\"\n  model : \"{}\"",
        show(&c_out),
        model.escape_debug()
    );
    c_out
}

/// Byte-identical stdout for a whole call sequence (state-persistence check).
pub fn assert_same_seq(calls: &[(i32, i32, i32)]) -> Vec<u8> {
    let c_out = run_seq(c_lib(), calls);
    let r_out = run_seq(rust_lib(), calls);
    assert_eq!(
        c_out,
        r_out,
        "stdout mismatch for call sequence {calls:?}\n  C   : \"{}\"\n  Rust: \"{}\"",
        show(&c_out),
        show(&r_out)
    );
    c_out
}

/// C-and-Rust agree AND the bytes are exactly `expected`.
pub fn assert_same_and_eq(x: i32, y: i32, z: i32, expected: &str) {
    let c_out = run_one(c_lib(), x, y, z);
    let r_out = run_one(rust_lib(), x, y, z);
    assert_eq!(
        c_out,
        r_out,
        "stdout mismatch for driver({x}, {y}, {z})\n  C   : \"{}\"\n  Rust: \"{}\"",
        show(&c_out),
        show(&r_out)
    );
    assert_eq!(
        c_out,
        expected.as_bytes(),
        "driver({x}, {y}, {z}) produced unexpected bytes\n  got     : \"{}\"\n  expected: \"{}\"",
        show(&c_out),
        expected.escape_debug()
    );
}

// ---------------------------------------------------------------------------
// Expected-output model, transcribed from c_src/src/driver.c
// ---------------------------------------------------------------------------

/// The exact stdout the C source produces for `driver(x, y, z)`.
/// Used as an independent third opinion, not as the primary oracle.
pub fn expected_output(x: i32, y: i32, z: i32) -> String {
    if x != 1 {
        "Error: x != 1\nOperation failed\nResult: 1\n".to_string()
    } else if y != 2 {
        "Error: x == 1 but y != 2\nOperation failed\nResult: 2\n".to_string()
    } else if z != 3 {
        "Error: x == 1 and y == 2, but z != 3\nOperation failed\nResult: 3\n".to_string()
    } else {
        "Ok!\nResult: 0\n".to_string()
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*), fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x2025_0828_D817_ACE1;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn next_i32(&mut self) -> i32 {
        (self.next_u64() >> 32) as u32 as i32
    }
    /// Uniform in `0..n`.
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
    /// A value that is *not* `forbidden`, drawn from the whole `i32` range.
    pub fn i32_except(&mut self, forbidden: i32) -> i32 {
        loop {
            let v = self.next_i32();
            if v != forbidden {
                return v;
            }
        }
    }
    /// Biased draw: often one of the interesting constants, otherwise random.
    pub fn interesting_i32(&mut self) -> i32 {
        const POOL: [i32; 14] = [
            i32::MIN,
            i32::MIN + 1,
            -3,
            -2,
            -1,
            0,
            1,
            2,
            3,
            4,
            5,
            123,
            i32::MAX - 1,
            i32::MAX,
        ];
        if self.next_u64() % 4 == 0 {
            self.next_i32()
        } else {
            POOL[(self.next_u64() as usize) % POOL.len()]
        }
    }
}
