//! Shared harness: loads the C `.so` and the Rust `.so` through `libloading`
//! and captures the C `stdout` stream so printf output can be compared too.

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::path::PathBuf;
use std::sync::OnceLock;

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

pub type OpFn = unsafe extern "C" fn(c_int, c_int, *mut c_void) -> c_int;
pub type GotomachFn = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation dir has a parent")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    let build = workspace_root().join("c_src/build");
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&build) {
        for e in entries.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("so") {
                candidates.push(p);
            }
        }
    }
    candidates.sort();
    candidates.pop().unwrap_or_else(|| {
        panic!(
            "no .so found in {} — build the C library first",
            build.display()
        )
    })
}

fn find_rust_so() -> PathBuf {
    // The test binary lives in target/<profile>/deps/, so walk up from there.
    let mut dir = std::env::current_exe().expect("current_exe");
    dir.pop(); // deps
    if dir.file_name().and_then(|s| s.to_str()) == Some("deps") {
        dir.pop();
    }
    let candidate = dir.join("libgotomach_lib.so");
    assert!(
        candidate.exists(),
        "rust cdylib not found at {} — run `cargo build` for this profile first",
        candidate.display()
    );
    candidate
}

pub struct Libs {
    pub c: Library,
    pub rust: Library,
}

pub fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| unsafe {
        let c = Library::new(find_c_so()).expect("load C .so");
        let rust = Library::new(find_rust_so()).expect("load Rust .so");
        Libs { c, rust }
    })
}

/// Every symbol the C `.so` exports must exist in the Rust `.so` too.
pub const EXPORTED_SYMBOLS: &[&str] =
    &["gotomach", "process_value", "double_value", "triple_value"];

pub fn op(lib: &'static Library, name: &str) -> OpFn {
    unsafe {
        let sym: Symbol<OpFn> = lib
            .get(format!("{name}\0").as_bytes())
            .unwrap_or_else(|e| panic!("missing symbol {name}: {e}"));
        *sym
    }
}

pub fn gotomach(lib: &'static Library) -> GotomachFn {
    unsafe {
        let sym: Symbol<GotomachFn> = lib.get(b"gotomach\0").expect("missing symbol gotomach");
        *sym
    }
}

/// Runs `f`, capturing everything written to file descriptor 1 (the C `stdout`
/// both libraries print to). Returns `(value, captured_bytes)`.
pub fn capture_stdout<T, F: FnOnce() -> T>(f: F) -> (T, Vec<u8>) {
    // Redirecting fd 1 is process-global, so serialise it across test threads.
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    unsafe {
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");

        let tmp_path = std::env::temp_dir().join(format!(
            "c2rust-capture-{}-{:?}.txt",
            std::process::id(),
            std::thread::current().id()
        ));
        // Reuse one file per thread: truncate + rewind instead of create/unlink
        // on every call, which matters for the exhaustive sweeps.
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&tmp_path)
            .expect("open capture file");
        let fd = {
            use std::os::fd::AsRawFd;
            file.as_raw_fd()
        };
        assert!(dup2(fd, 1) >= 0, "dup2 failed");

        let value = f();

        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "restore dup2 failed");
        close(saved);

        use std::io::{Read, Seek};
        let mut file = file;
        file.rewind().expect("rewind capture file");
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).expect("read capture file");
        (value, bytes)
    }
}

/// Calls the same C-ABI symbol in both libraries and asserts the return value
/// and the emitted stdout bytes are identical.
pub fn compare_gotomach(iterations: c_int, seed: c_int, mode: c_int, threshold: c_int) {
    let l = libs();
    let cf = gotomach(&l.c);
    let rf = gotomach(&l.rust);

    let (cv, cout) = capture_stdout(|| unsafe { cf(iterations, seed, mode, threshold) });
    let (rv, rout) = capture_stdout(|| unsafe { rf(iterations, seed, mode, threshold) });

    assert_eq!(
        cv, rv,
        "return value mismatch for gotomach({iterations}, {seed}, {mode}, {threshold})"
    );
    assert_eq!(
        String::from_utf8_lossy(&cout),
        String::from_utf8_lossy(&rout),
        "stdout mismatch for gotomach({iterations}, {seed}, {mode}, {threshold})"
    );
    assert_eq!(
        cout, rout,
        "stdout bytes mismatch for gotomach({iterations}, {seed}, {mode}, {threshold})"
    );
    // gotomach always logs at least one line; guard against a vacuous pass
    // where the capture machinery silently swallowed both sides.
    assert!(
        !cout.is_empty(),
        "captured no stdout at all for gotomach({iterations}, {seed}, {mode}, {threshold})"
    );
}

pub fn compare_op(name: &str, value: c_int, unused: c_int, ctx: *mut c_void) {
    let l = libs();
    let cf = op(&l.c, name);
    let rf = op(&l.rust, name);
    let (cv, cout) = capture_stdout(|| unsafe { cf(value, unused, ctx) });
    let (rv, rout) = capture_stdout(|| unsafe { rf(value, unused, ctx) });
    assert_eq!(cv, rv, "{name}({value}, {unused}) return mismatch");
    assert_eq!(cout, rout, "{name}({value}, {unused}) stdout mismatch");
}

/// Deterministic PRNG so the fuzz cases are reproducible.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed | 1)
    }
    pub fn next_u32(&mut self) -> u32 {
        // xorshift64*
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        ((x.wrapping_mul(0x2545_F491_4F6C_DD1D)) >> 32) as u32
    }
    pub fn range(&mut self, lo: i64, hi: i64) -> i64 {
        let span = (hi - lo + 1) as u64;
        lo + (self.next_u32() as u64 % span) as i64
    }
}
