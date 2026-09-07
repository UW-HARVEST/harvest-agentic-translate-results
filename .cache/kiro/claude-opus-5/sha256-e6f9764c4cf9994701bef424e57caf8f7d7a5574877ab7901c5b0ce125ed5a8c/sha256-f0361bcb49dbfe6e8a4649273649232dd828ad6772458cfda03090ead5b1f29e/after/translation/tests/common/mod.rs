//! Shared differential-testing harness.
//!
//! Both libraries are loaded as shared objects through `libloading` and called
//! only through their exported `extern "C"` symbols — the Rust functions are
//! never invoked directly, so the `#[no_mangle]` wrappers are under test too.
//!
//! Because the C code writes to the process-global `stdout`/`stderr` streams,
//! every call is wrapped in an fd-1/fd-2 capture. The capture is guarded by a
//! global mutex so that cargo's parallel test threads cannot interleave.

#![allow(dead_code)]

use std::ffi::{CString, c_char, c_int, c_void};
use std::io::Write;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

pub type FilePtr = *mut c_void;

pub type FnForward = unsafe extern "C" fn(c_int) -> c_int;
pub type FnOpen = unsafe extern "C" fn(*const c_char) -> FilePtr;
pub type FnDriver = unsafe extern "C" fn(c_int, *const c_char) -> c_int;

pub struct Lib {
    pub name: &'static str,
    _lib: libloading::Library,
    pub forward_goto_example: FnForward,
    pub open_with_cleanup: FnOpen,
    pub driver: FnDriver,
}

impl Lib {
    fn load(name: &'static str, path: &Path) -> Lib {
        let lib = unsafe { libloading::Library::new(path) }
            .unwrap_or_else(|e| panic!("failed to dlopen {} ({}): {e}", path.display(), name));
        unsafe {
            let forward_goto_example = *lib
                .get::<FnForward>(b"forward_goto_example\0")
                .expect("missing symbol forward_goto_example");
            let open_with_cleanup = *lib
                .get::<FnOpen>(b"open_with_cleanup\0")
                .expect("missing symbol open_with_cleanup");
            let driver = *lib
                .get::<FnDriver>(b"driver\0")
                .expect("missing symbol driver");
            Lib {
                name,
                _lib: lib,
                forward_goto_example,
                open_with_cleanup,
                driver,
            }
        }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `target/<profile>/` for whichever profile the test binary was built in.
fn profile_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    // .../target/<profile>/deps/<test-bin>
    exe.parent()
        .and_then(|p| p.parent())
        .expect("target/<profile>")
        .to_path_buf()
}

pub fn c_lib() -> &'static Lib {
    static L: OnceLock<Lib> = OnceLock::new();
    L.get_or_init(|| {
        let p = manifest_dir()
            .parent()
            .unwrap()
            .join("c_src/build/libdriver.so");
        assert!(
            p.exists(),
            "C shared library not built at {} — run the cmake build first",
            p.display()
        );
        Lib::load("C", &p)
    })
}

pub fn rust_lib() -> &'static Lib {
    static L: OnceLock<Lib> = OnceLock::new();
    L.get_or_init(|| {
        // `RUST_DRIVER_SO` lets the same suite be pointed at the release
        // artifact (which is built with `panic = "abort"` and optimizations).
        let p = match std::env::var_os("RUST_DRIVER_SO") {
            Some(v) => PathBuf::from(v),
            None => profile_dir().join("libdriver.so"),
        };
        assert!(
            p.exists(),
            "Rust cdylib not found at {} — run `cargo build` first",
            p.display()
        );
        Lib::load("Rust", &p)
    })
}

unsafe extern "C" {
    fn fflush(stream: *mut c_void) -> c_int;
    fn fclose(stream: *mut c_void) -> c_int;
    fn feof(stream: *mut c_void) -> c_int;
    fn ferror(stream: *mut c_void) -> c_int;
}

fn capture_lock() -> &'static Mutex<u64> {
    static M: OnceLock<Mutex<u64>> = OnceLock::new();
    M.get_or_init(|| Mutex::new(0))
}

/// Bytes written to fd 1 and fd 2 while a call was running, plus its result.
pub struct Out<T> {
    pub ret: T,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

/// Runs `f` with fd 1 and fd 2 redirected to fresh temp files and returns
/// everything that was written to them.
///
/// A panic inside `f` must not leave fd 1/2 pointing at the temp files (that
/// would swallow every later test's output and leak the capture files), so the
/// closure is run under `catch_unwind` and the panic is re-raised only after the
/// descriptors have been restored and the files cleaned up.
pub fn capture<T, F: FnOnce() -> T>(f: F) -> Out<T> {
    // Force both libraries to load *outside* the redirect, so a load failure
    // panics with the fds still intact.
    let _ = c_lib();
    let _ = rust_lib();

    let mut counter = capture_lock().lock().unwrap_or_else(|e| e.into_inner());
    *counter += 1;
    let tag = format!("{}-{}", std::process::id(), *counter);
    let dir = std::env::temp_dir();
    let p_out = dir.join(format!("difftest-out-{tag}"));
    let p_err = dir.join(format!("difftest-err-{tag}"));

    let ret;
    unsafe {
        // Flush anything the harness itself buffered before stealing the fds.
        fflush(std::ptr::null_mut());
        let _ = std::io::stdout().flush();
        let _ = std::io::stderr().flush();

        let save_out = libc::dup(1);
        let save_err = libc::dup(2);
        assert!(save_out >= 0 && save_err >= 0, "dup failed");

        {
            let f_out = std::fs::File::create(&p_out).expect("create stdout capture file");
            let f_err = std::fs::File::create(&p_err).expect("create stderr capture file");
            assert!(libc::dup2(f_out.as_raw_fd(), 1) >= 0, "dup2 stdout");
            assert!(libc::dup2(f_err.as_raw_fd(), 2) >= 0, "dup2 stderr");
        }

        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));

        fflush(std::ptr::null_mut());
        libc::dup2(save_out, 1);
        libc::dup2(save_err, 2);
        libc::close(save_out);
        libc::close(save_err);

        match r {
            Ok(v) => ret = v,
            Err(p) => {
                let _ = std::fs::remove_file(&p_out);
                let _ = std::fs::remove_file(&p_err);
                drop(counter);
                std::panic::resume_unwind(p);
            }
        }
    }

    let stdout = std::fs::read(&p_out).unwrap_or_default();
    let stderr = std::fs::read(&p_err).unwrap_or_default();
    let _ = std::fs::remove_file(&p_out);
    let _ = std::fs::remove_file(&p_err);
    drop(counter);
    Out {
        ret,
        stdout,
        stderr,
    }
}

fn show(b: &[u8]) -> String {
    let mut s = String::new();
    for &c in b {
        match c {
            b'\n' => s.push_str("\\n"),
            b'\r' => s.push_str("\\r"),
            0 => s.push_str("\\0"),
            0x20..=0x7e => s.push(c as char),
            _ => s.push_str(&format!("\\x{c:02x}")),
        }
    }
    s
}

pub fn assert_streams_eq(ctx: &str, c: &Out<impl std::fmt::Debug>, r: &Out<impl std::fmt::Debug>) {
    assert_eq!(
        c.stdout,
        r.stdout,
        "{ctx}: stdout differs\n  C   ({} bytes): {}\n  Rust({} bytes): {}",
        c.stdout.len(),
        show(&c.stdout),
        r.stdout.len(),
        show(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "{ctx}: stderr differs\n  C   ({} bytes): {}\n  Rust({} bytes): {}",
        c.stderr.len(),
        show(&c.stderr),
        r.stderr.len(),
        show(&r.stderr)
    );
}

// ---------------------------------------------------------------------------
// Differential drivers, one per exported symbol.
// ---------------------------------------------------------------------------

/// `forward_goto_example`: compares return value, stdout and stderr.
pub fn diff_forward(x: c_int) {
    let c = capture(|| unsafe { (c_lib().forward_goto_example)(x) });
    let r = capture(|| unsafe { (rust_lib().forward_goto_example)(x) });
    let ctx = format!("forward_goto_example({x})");
    assert_eq!(c.ret, r.ret, "{ctx}: return value differs");
    assert_streams_eq(&ctx, &c, &r);
}

/// `open_with_cleanup`: compares NULL-ness of the returned `FILE*`, its
/// `feof`/`ferror` state when non-NULL, stdout and stderr. Closes the handle
/// afterwards (the C returns it still open).
pub fn diff_open(raw_name: Option<&[u8]>) {
    let cs = raw_name.map(|b| CString::new(b).expect("filename contains interior NUL"));
    let ptr = cs
        .as_ref()
        .map(|c| c.as_ptr())
        .unwrap_or(std::ptr::null());

    let c = capture(|| unsafe { (c_lib().open_with_cleanup)(ptr) });
    let r = capture(|| unsafe { (rust_lib().open_with_cleanup)(ptr) });

    let ctx = format!(
        "open_with_cleanup({:?})",
        raw_name.map(|b| String::from_utf8_lossy(b).into_owned())
    );
    assert_eq!(
        c.ret.is_null(),
        r.ret.is_null(),
        "{ctx}: NULL-ness of returned FILE* differs (C={:?} Rust={:?})",
        c.ret,
        r.ret
    );
    if !c.ret.is_null() {
        unsafe {
            assert_eq!(
                feof(c.ret) != 0,
                feof(r.ret) != 0,
                "{ctx}: feof state differs"
            );
            assert_eq!(
                ferror(c.ret) != 0,
                ferror(r.ret) != 0,
                "{ctx}: ferror state differs"
            );
            fclose(c.ret);
            fclose(r.ret);
        }
    }
    assert_streams_eq(&ctx, &c, &r);
}

/// `driver`: compares return value, stdout and stderr.
pub fn diff_driver(num: c_int, raw_name: Option<&[u8]>) {
    let cs = raw_name.map(|b| CString::new(b).expect("filename contains interior NUL"));
    let ptr = cs
        .as_ref()
        .map(|c| c.as_ptr())
        .unwrap_or(std::ptr::null());

    let c = capture(|| unsafe { (c_lib().driver)(num, ptr) });
    let r = capture(|| unsafe { (rust_lib().driver)(num, ptr) });

    let ctx = format!(
        "driver({num}, {:?})",
        raw_name.map(|b| String::from_utf8_lossy(b).into_owned())
    );
    assert_eq!(c.ret, r.ret, "{ctx}: return value differs");
    assert_streams_eq(&ctx, &c, &r);
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) — fixed seed for reproducibility.
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5EED_C0DE_5EED_C0DE;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(if seed == 0 { 1 } else { seed })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    /// Uniform in `0..n` (n > 0).
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + self.below(span) as i64) as i32
    }
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 24) as u8
    }
    pub fn printable(&mut self) -> u8 {
        // 0x20..=0x7e
        0x20 + (self.below(0x5f) as u8)
    }
}

// ---------------------------------------------------------------------------
// Temp file helpers.
// ---------------------------------------------------------------------------

pub struct TempFile(pub PathBuf);

impl TempFile {
    pub fn with_bytes(tag: &str, bytes: &[u8]) -> TempFile {
        static N: OnceLock<Mutex<u64>> = OnceLock::new();
        let mut n = N.get_or_init(|| Mutex::new(0)).lock().unwrap();
        *n += 1;
        let p = std::env::temp_dir().join(format!(
            "difftest-in-{}-{}-{}",
            std::process::id(),
            tag,
            *n
        ));
        drop(n);
        std::fs::write(&p, bytes).expect("write temp input file");
        TempFile(p)
    }
    pub fn name_bytes(&self) -> Vec<u8> {
        self.0.as_os_str().as_encoded_bytes().to_vec()
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = std::fs::set_permissions(&self.0, std::fs::Permissions::from_mode(0o600));
        let _ = std::fs::remove_file(&self.0);
    }
}

use std::os::unix::fs::PermissionsExt;

/// Convenience: create a file with `bytes`, run both libraries' `open_with_cleanup`
/// on it and compare.
pub fn diff_open_file(tag: &str, bytes: &[u8]) {
    let f = TempFile::with_bytes(tag, bytes);
    diff_open(Some(&f.name_bytes()));
}

/// Convenience: same for `driver`.
pub fn diff_driver_file(tag: &str, num: c_int, bytes: &[u8]) {
    let f = TempFile::with_bytes(tag, bytes);
    diff_driver(num, Some(&f.name_bytes()));
}

// ---------------------------------------------------------------------------
// Random file-content generators covering the shapes in CONFIGS.md.
// ---------------------------------------------------------------------------

pub fn gen_lines(rng: &mut Rng, n_lines: usize, max_len: usize, trailing_newline: bool) -> Vec<u8> {
    let mut v = Vec::new();
    for i in 0..n_lines {
        let len = rng.below(max_len as u64 + 1) as usize;
        for _ in 0..len {
            v.push(rng.printable());
        }
        if i + 1 < n_lines || trailing_newline {
            v.push(b'\n');
        }
    }
    v
}

pub fn gen_binary(rng: &mut Rng, len: usize) -> Vec<u8> {
    (0..len).map(|_| rng.byte()).collect()
}
