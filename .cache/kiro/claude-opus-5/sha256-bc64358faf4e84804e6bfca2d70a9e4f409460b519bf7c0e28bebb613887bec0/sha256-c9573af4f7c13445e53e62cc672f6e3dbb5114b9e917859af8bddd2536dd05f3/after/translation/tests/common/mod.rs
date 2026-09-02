//! Shared differential-test harness.
#![allow(dead_code)] // each test file uses a different subset of the helpers
//!
//! Loads BOTH shared libraries through `libloading` and calls `tool_basename`
//! only through the exported C symbol — never by calling the Rust function
//! directly — so the `#[no_mangle] extern "C"` wrapper is under test too.

use libloading::{Library, Symbol};
use std::ffi::c_char;
use std::path::PathBuf;
use std::sync::OnceLock;

pub type ToolBasename = unsafe extern "C" fn(*mut c_char) -> *mut c_char;

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn c_so_path() -> PathBuf {
    workspace_root().join("c_src/build/libdriver.so")
}

/// Directory of the `.so` for the SAME profile as the running test binary.
///
/// `current_exe()` is `<...>/target/<profile>/deps/<test>-<hash>`, so the
/// `cdylib` belongs in `<...>/target/<profile>/`.
fn profile_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    exe.parent()
        .and_then(|deps| deps.parent())
        .expect("test binary should live in target/<profile>/deps/")
        .to_path_buf()
}

/// The Rust `cdylib` to load, as an immutable per-process snapshot.
///
/// IMPORTANT: `cargo test` does **not** build a `crate-type = ["cdylib"]`
/// artifact (integration tests cannot link one), so `target/<profile>/libdriver.so`
/// may be missing or STALE while the tests run. Silently loading a stale `.so`,
/// or one from another profile, would produce false passes — a mutated
/// `src/lib.rs` would still "pass". And because the build happens while tests are
/// running, cargo could replace the file underneath a `dlopen`.
///
/// So this function, once per test process:
///   1. takes an exclusive file lock so concurrent test binaries do not race;
///   2. builds the `cdylib` for the current profile and feature set;
///   3. refuses to continue if the artifact is missing or older than the sources;
///   4. copies it to a private snapshot and returns that path, so nothing can
///      change the library under a running test.
///
/// No cross-profile fallback is permitted.
fn rust_so_path() -> PathBuf {
    // A child process (Phase C victim) is handed the parent's snapshot directly.
    if let Some(p) = std::env::var_os("TB_RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "TB_RUST_SO points at a missing file: {p:?}");
        return p;
    }

    let dir = profile_dir();
    let so = dir.join("libdriver.so");
    let is_release = dir.file_name().and_then(|s| s.to_str()) == Some("release");
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    let _guard = FileLock::acquire(&dir.join(".tool_basename_test.lock"));

    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let mut cmd = std::process::Command::new(cargo);
    cmd.arg("build").arg("--quiet").arg("--lib");
    if is_release {
        cmd.arg("--release");
    }
    // Match the feature selection of the current test run.
    if let Ok(feats) = std::env::var("TB_TEST_FEATURES") {
        if feats == "__none__" {
            cmd.arg("--no-default-features");
        } else if !feats.is_empty() {
            cmd.arg("--no-default-features").arg("--features").arg(feats);
        }
    }
    let built = cmd
        .current_dir(&manifest)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::inherit())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    assert!(
        so.exists(),
        "Rust cdylib missing at {so:?} (nested `cargo build` {}). \
         `cargo test` does not build cdylib artifacts; run `cargo build{}` first.",
        if built {
            "succeeded but produced nothing"
        } else {
            "failed"
        },
        if is_release { " --release" } else { "" }
    );

    // Freshness backstop: the artifact must be at least as new as every Rust
    // source file. (Cargo.toml is deliberately NOT checked — cargo does not
    // relink the lib when only `[dev-dependencies]` change, so its mtime is a
    // false staleness signal.)
    let so_mtime = so.metadata().and_then(|m| m.modified()).expect("so mtime");
    for src in rust_sources(&manifest.join("src")) {
        if let Ok(m) = src.metadata().and_then(|m| m.modified()) {
            assert!(
                m <= so_mtime,
                "STALE ARTIFACT: {src:?} is newer than {so:?}. The tests would be \
                 comparing against an out-of-date library. Run `cargo build{}` and re-run.",
                if is_release { " --release" } else { "" }
            );
        }
    }

    // Immutable snapshot for this process. Removing older snapshots while holding
    // the lock is safe: on Linux a dlopen'd file keeps its inode alive.
    if let Ok(rd) = std::fs::read_dir(&dir) {
        for e in rd.flatten() {
            let n = e.file_name();
            let n = n.to_string_lossy();
            if n.starts_with("tb-snapshot-") && n.ends_with(".so") {
                let _ = std::fs::remove_file(e.path());
            }
        }
    }
    let snap = dir.join(format!("tb-snapshot-{}.so", std::process::id()));
    std::fs::copy(&so, &snap).unwrap_or_else(|e| panic!("snapshot {so:?} -> {snap:?}: {e}"));
    snap
}

/// Every `.rs` file under `dir`, recursively.
fn rust_sources(dir: &std::path::Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                out.extend(rust_sources(&p));
            } else if p.extension().and_then(|s| s.to_str()) == Some("rs") {
                out.push(p);
            }
        }
    }
    out
}

/// Minimal advisory `flock` guard (unlocks on drop).
struct FileLock(std::fs::File);

impl FileLock {
    fn acquire(path: &std::path::Path) -> Self {
        use std::os::unix::io::AsRawFd;
        let f = std::fs::OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(path)
            .unwrap_or_else(|e| panic!("open lock {path:?}: {e}"));
        // Safety: valid fd for the lifetime of the call.
        let rc = unsafe { libc::flock(f.as_raw_fd(), libc::LOCK_EX) };
        assert_eq!(rc, 0, "flock {path:?} failed");
        FileLock(f)
    }
}

impl Drop for FileLock {
    fn drop(&mut self) {
        use std::os::unix::io::AsRawFd;
        unsafe { libc::flock(self.0.as_raw_fd(), libc::LOCK_UN) };
    }
}

pub struct Libs {
    _c: Library,
    _rust: Library,
    pub c_basename: ToolBasename,
    pub rust_basename: ToolBasename,
}

// Safety: raw fn pointers into two libraries that are kept alive for the whole
// process (leaked into a OnceLock) and whose code is reentrant and stateless.
unsafe impl Send for Libs {}
unsafe impl Sync for Libs {}

fn load() -> Libs {
    let cp = c_so_path();
    let rp = rust_so_path();
    assert!(
        cp.exists(),
        "C shared library missing at {cp:?}; build it with:\n  cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
    );

    unsafe {
        let c = Library::new(&cp).unwrap_or_else(|e| panic!("dlopen {cp:?}: {e}"));
        let rust = Library::new(&rp).unwrap_or_else(|e| panic!("dlopen {rp:?}: {e}"));

        let c_sym: Symbol<ToolBasename> = c
            .get(b"tool_basename\0")
            .expect("C .so does not export tool_basename");
        let r_sym: Symbol<ToolBasename> = rust
            .get(b"tool_basename\0")
            .expect("Rust .so does not export tool_basename");

        let c_basename = *c_sym;
        let rust_basename = *r_sym;

        Libs {
            _c: c,
            _rust: rust,
            c_basename,
            rust_basename,
        }
    }
}

pub fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(load)
}

/// Paths actually loaded, for the self-check test.
pub fn loaded_paths() -> (PathBuf, PathBuf) {
    (c_so_path(), rust_so_path())
}

/// Outcome of one call, captured in a comparable form.
#[derive(Debug, PartialEq, Eq)]
pub struct Outcome {
    /// Byte offset of the returned pointer from the start of the input buffer.
    /// `None` if the returned pointer lies outside the buffer (a real bug).
    pub offset: Option<isize>,
    /// The returned NUL-terminated string, bytes excluding the terminator.
    pub result: Vec<u8>,
    /// The input buffer after the call, to prove it was not mutated.
    pub buf_after: Vec<u8>,
}

/// Call `f` with a fresh, exactly-sized, NUL-terminated copy of `input`.
///
/// # Safety
/// `input` must not contain an interior NUL (otherwise the "string" would end
/// early and `buf_after` comparison would be meaningless); callers here comply.
pub fn call(f: ToolBasename, input: &[u8]) -> Outcome {
    debug_assert!(
        !input.contains(&0),
        "test inputs must not contain interior NUL"
    );
    let mut buf: Vec<u8> = Vec::with_capacity(input.len() + 1);
    buf.extend_from_slice(input);
    buf.push(0);

    let base = buf.as_mut_ptr() as *mut c_char;
    let ret = unsafe { f(base) };

    let offset = if ret.is_null() {
        None
    } else {
        let off = unsafe { ret.offset_from(base) };
        if off < 0 || off as usize >= buf.len() {
            None
        } else {
            Some(off)
        }
    };

    let result = match offset {
        Some(off) => {
            let start = off as usize;
            let end = buf[start..]
                .iter()
                .position(|&b| b == 0)
                .map(|p| start + p)
                .unwrap_or(buf.len());
            buf[start..end].to_vec()
        }
        None => Vec::new(),
    };

    Outcome {
        offset,
        result,
        buf_after: buf[..buf.len() - 1].to_vec(),
    }
}

/// Assert C and Rust agree byte-for-byte for `input`, and that the contract
/// holds (pointer inside the caller's buffer, buffer unmodified) — CONFIGS row 18.
pub fn assert_same(input: &[u8], ctx: &str) {
    let l = libs();
    let c = call(l.c_basename, input);
    let r = call(l.rust_basename, input);

    assert_eq!(
        c, r,
        "DIVERGENCE [{ctx}]\n  input  = {}\n  C      = {c:?}\n  Rust   = {r:?}",
        show(input)
    );

    // Contract checks, verified against the C's own behaviour.
    assert!(
        c.offset.is_some(),
        "[{ctx}] C returned a pointer outside the input buffer for {}",
        show(input)
    );
    let off = c.offset.unwrap() as usize;
    assert!(
        off <= input.len(),
        "[{ctx}] offset {off} past end of {} byte input",
        input.len()
    );
    assert_eq!(
        c.buf_after,
        input,
        "[{ctx}] C mutated the input buffer for {}",
        show(input)
    );
    assert_eq!(
        r.buf_after,
        input,
        "[{ctx}] Rust mutated the input buffer for {}",
        show(input)
    );
    // The returned string must be the suffix at `offset`.
    assert_eq!(
        c.result,
        &input[off..],
        "[{ctx}] result is not the suffix at offset {off} for {}",
        show(input)
    );
}

pub fn show(input: &[u8]) -> String {
    if input.len() > 96 {
        format!(
            "<{} bytes> {:?}...{:?}",
            input.len(),
            &input[..32],
            &input[input.len() - 32..]
        )
    } else {
        format!("{input:?} ({:?})", String::from_utf8_lossy(input))
    }
}

/// SplitMix64 — deterministic, fixed-seed PRNG so failures are reproducible.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// Uniform in `0..n` (n > 0).
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
    pub fn range(&mut self, lo: usize, hi_inclusive: usize) -> usize {
        lo + self.below(hi_inclusive - lo + 1)
    }
    /// A non-NUL byte, never `/` or `\`.
    pub fn plain_byte(&mut self) -> u8 {
        loop {
            let b = (self.next_u64() & 0xFF) as u8;
            if b != 0 && b != b'/' && b != b'\\' {
                return b;
            }
        }
    }
    /// A printable ASCII byte, never `/` or `\`.
    pub fn ascii_byte(&mut self) -> u8 {
        loop {
            let b = self.range(0x20, 0x7E) as u8;
            if b != b'/' && b != b'\\' {
                return b;
            }
        }
    }
    /// Any non-NUL byte, `/` and `\` included.
    pub fn any_byte(&mut self) -> u8 {
        (self.range(1, 255)) as u8
    }
}

pub const SEED: u64 = 0x5EED_1234_ABCD_0001;

/// Build a string of `len` plain (non-separator) bytes.
pub fn plain(rng: &mut Rng, len: usize, ascii_only: bool) -> Vec<u8> {
    (0..len)
        .map(|_| {
            if ascii_only {
                rng.ascii_byte()
            } else {
                rng.plain_byte()
            }
        })
        .collect()
}
