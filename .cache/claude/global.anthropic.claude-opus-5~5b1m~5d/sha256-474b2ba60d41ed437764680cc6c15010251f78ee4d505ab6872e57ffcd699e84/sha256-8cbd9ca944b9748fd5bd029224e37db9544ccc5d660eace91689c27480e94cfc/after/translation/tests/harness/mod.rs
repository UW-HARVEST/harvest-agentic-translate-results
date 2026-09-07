//! Shared differential-test harness.
//!
//! Loads BOTH shared libraries via `libloading` and calls every function
//! through its exported C ABI symbol. The Rust crate is NEVER called
//! directly — only through `translation/target/{release,debug}/libdriver.so`,
//! exactly as an external C consumer would, so the `#[no_mangle]` export
//! wrappers are under test too.
//!
//! The library's only observable output channel is `stdout` (every public
//! function returns `void`), so "compare outputs" means: redirect file
//! descriptor 1 to a temp file, run the calls, and compare the resulting
//! bytes byte-for-byte.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, CString};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

pub struct Libs {
    pub c: Library,
    pub r: Library,
    pub c_path: PathBuf,
    pub r_path: PathBuf,
}

fn find_c_so() -> PathBuf {
    if let Ok(p) = std::env::var("C_DRIVER_SO") {
        return PathBuf::from(p);
    }
    // cwd for integration tests is the package root (translation/).
    let candidates = [
        "../c_src/build/libdriver.so",
        "../c_src/build/lib/libdriver.so",
        "c_src/build/libdriver.so",
    ];
    for c in candidates {
        let p = PathBuf::from(c);
        if p.exists() {
            return p;
        }
    }
    panic!(
        "could not locate the C libdriver.so; build it with:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
    );
}

fn find_rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_DRIVER_SO") {
        return PathBuf::from(p);
    }
    // Prefer whichever exists; if both exist prefer the newer one so that
    // `cargo test` and `cargo test --release` both do the right thing.
    let rel = PathBuf::from("target/release/libdriver.so");
    let dbg = PathBuf::from("target/debug/libdriver.so");
    match (rel.exists(), dbg.exists()) {
        (true, true) => {
            let m = |p: &PathBuf| {
                std::fs::metadata(p)
                    .and_then(|m| m.modified())
                    .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
            };
            if m(&rel) >= m(&dbg) {
                rel
            } else {
                dbg
            }
        }
        (true, false) => rel,
        (false, true) => dbg,
        (false, false) => panic!(
            "could not locate the Rust libdriver.so; build it with:\n  \
             cd translation && cargo build --release"
        ),
    }
}

pub fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| {
        let c_path = find_c_so();
        let r_path = find_rust_so();
        // SAFETY: both are plain C-ABI shared objects with no init side effects.
        let c = unsafe { Library::new(&c_path) }
            .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", c_path.display()));
        let r = unsafe { Library::new(&r_path) }
            .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", r_path.display()));
        Libs {
            c,
            r,
            c_path,
            r_path,
        }
    })
}

// ---------------------------------------------------------------------------
// Typed symbol accessors (these exercise the exported names themselves)
// ---------------------------------------------------------------------------

pub type FnDriver = unsafe extern "C" fn(c_int);
pub type FnVoid = unsafe extern "C" fn();
pub type FnHex = unsafe extern "C" fn(c_char);
pub type FnLine = unsafe extern "C" fn(*const c_char);

pub fn sym_driver<'a>(l: &'a Library) -> Symbol<'a, FnDriver> {
    unsafe { l.get(b"driver\0") }.expect("symbol `driver` missing")
}
pub fn sym_good<'a>(l: &'a Library) -> Symbol<'a, FnVoid> {
    unsafe { l.get(b"good\0") }.expect("symbol `good` missing")
}
pub fn sym_bad<'a>(l: &'a Library) -> Symbol<'a, FnVoid> {
    unsafe { l.get(b"bad\0") }.expect("symbol `bad` missing")
}
pub fn sym_hex<'a>(l: &'a Library) -> Symbol<'a, FnHex> {
    unsafe { l.get(b"printHexCharLine\0") }.expect("symbol `printHexCharLine` missing")
}
pub fn sym_line<'a>(l: &'a Library) -> Symbol<'a, FnLine> {
    unsafe { l.get(b"printLine\0") }.expect("symbol `printLine` missing")
}

// ---------------------------------------------------------------------------
// stdout capture (fd-level, so it catches libc printf/puts from both .so's)
// ---------------------------------------------------------------------------

// fd 1 redirection is process-global, so serialize captures across the
// harness's parallel test threads.
fn capture_lock() -> &'static Mutex<()> {
    static M: OnceLock<Mutex<()>> = OnceLock::new();
    M.get_or_init(|| Mutex::new(()))
}

/// fd 1 is process-global, so libtest's own progress text ("test foo ... ok")
/// would land inside our capture files if the harness ran tests in parallel.
/// `.cargo/config.toml` sets `RUST_TEST_THREADS=1`; verify it took effect.
fn assert_single_threaded_harness() {
    static CHECKED: OnceLock<()> = OnceLock::new();
    CHECKED.get_or_init(|| {
        let n = std::env::var("RUST_TEST_THREADS").unwrap_or_default();
        assert_eq!(
            n, "1",
            "these differential tests redirect the process-global fd 1 and must run \
             single-threaded.\nRun them as:  RUST_TEST_THREADS=1 cargo test  \
             (or `cargo test -- --test-threads=1`).\n\
             translation/.cargo/config.toml sets this automatically."
        );
    });
}

/// Run `f`, returning every byte it wrote to file descriptor 1.
pub fn capture<F: FnOnce()>(f: F) -> Vec<u8> {
    assert_single_threaded_harness();
    let _guard = capture_lock().lock().unwrap_or_else(|e| e.into_inner());

    let mut tmp = std::env::temp_dir();
    tmp.push(format!(
        "driver_diff_{}_{:?}.out",
        std::process::id(),
        std::thread::current().id()
    ));
    let cpath = CString::new(tmp.as_os_str().as_encoded_bytes()).unwrap();

    // Flush Rust's own stdout buffer (libtest progress text) as well as the C
    // stdio buffers, so nothing pending lands in our capture file.
    use std::io::Write;
    let _ = std::io::stdout().flush();

    unsafe {
        libc::fflush(std::ptr::null_mut());

        let saved = libc::dup(1);
        assert!(saved >= 0, "dup(1) failed");

        let fd = libc::open(
            cpath.as_ptr(),
            libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC,
            0o600 as libc::c_int,
        );
        assert!(fd >= 0, "open({}) failed", tmp.display());

        assert!(libc::dup2(fd, 1) >= 0, "dup2 onto stdout failed");

        // Run the library calls.
        f();

        // Flush the C stdio buffers before restoring, otherwise buffered
        // bytes would land on the restored fd instead of the capture file.
        libc::fflush(std::ptr::null_mut());

        assert!(libc::dup2(saved, 1) >= 0, "dup2 restore failed");
        libc::close(saved);

        // Read the capture back.
        assert!(libc::lseek(fd, 0, libc::SEEK_SET) == 0, "lseek failed");
        let mut out = Vec::new();
        let mut buf = [0u8; 8192];
        loop {
            let n = libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len());
            if n < 0 {
                panic!("read from capture file failed");
            }
            if n == 0 {
                break;
            }
            out.extend_from_slice(&buf[..n as usize]);
        }
        libc::close(fd);
        let _ = std::fs::remove_file(&tmp);
        out
    }
}

// ---------------------------------------------------------------------------
// The differential assertion
// ---------------------------------------------------------------------------

fn show(b: &[u8]) -> String {
    let mut s = String::new();
    for &c in b.iter().take(512) {
        match c {
            b'\n' => s.push_str("\\n"),
            0x20..=0x7e => s.push(c as char),
            _ => s.push_str(&format!("\\x{c:02x}")),
        }
    }
    if b.len() > 512 {
        s.push_str(&format!("... ({} bytes total)", b.len()));
    }
    s
}

/// Run `body` against the C `.so` and against the Rust `.so` and assert the
/// captured stdout bytes are identical. Returns the (identical) bytes.
pub fn diff(label: &str, body: impl Fn(&Library)) -> Vec<u8> {
    let l = libs();
    let out_c = capture(|| body(&l.c));
    let out_r = capture(|| body(&l.r));
    if out_c != out_r {
        panic!(
            "DIVERGENCE in [{label}]\n  C   ({} bytes): \"{}\"\n  Rust({} bytes): \"{}\"",
            out_c.len(),
            show(&out_c),
            out_r.len(),
            show(&out_r),
        );
    }
    out_c
}

/// Like [`diff`] but also asserts the C output equals `expected` (documents
/// the ground-truth bytes recorded in ERRORS.md / CONFIGS.md).
pub fn diff_eq(label: &str, expected: &[u8], body: impl Fn(&Library)) {
    let got = diff(label, body);
    assert_eq!(
        got,
        expected,
        "[{label}] C ground truth changed: expected \"{}\", got \"{}\"",
        show(expected),
        show(&got)
    );
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub const SEED: u64 = 0x5EED_1234_ABCD_0001;

    pub fn new() -> Self {
        Rng(Self::SEED)
    }
    pub fn with_seed(s: u64) -> Self {
        Rng(s)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    pub fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }
    /// Uniform-ish in `0..n`.
    pub fn below(&mut self, n: u64) -> u64 {
        assert!(n > 0);
        self.next_u64() % n
    }
}

/// A NUL-terminated byte buffer that can hold arbitrary interior bytes,
/// including interior NULs (which `CString` forbids).
pub fn cbuf(bytes: &[u8]) -> Vec<u8> {
    let mut v = bytes.to_vec();
    v.push(0);
    v
}

/// Random string of `len` bytes drawn from `0x01..=0xFF` (no interior NUL).
pub fn rand_bytes_no_nul(rng: &mut Rng, len: usize) -> Vec<u8> {
    (0..len)
        .map(|_| {
            let mut b = rng.next_u8();
            if b == 0 {
                b = 1;
            }
            b
        })
        .collect()
}

/// Random printable-ASCII string of `len` bytes.
pub fn rand_ascii(rng: &mut Rng, len: usize) -> Vec<u8> {
    (0..len)
        .map(|_| 0x20u8 + (rng.below(0x7f - 0x20) as u8))
        .collect()
}
