//! Differential-test harness.
//!
//! Loads BOTH shared objects through `libloading` and calls only their exported
//! C symbols, so the `#[no_mangle] extern "C"` wrappers are exercised exactly as
//! an external consumer would exercise them. No Rust function is ever called
//! directly.
//!
//! Both libraries keep mutable file-scope state (`the_house`), so every test
//! that needs a pristine `{floors = 2, bedrooms = 5, bathrooms = 2.5}` house
//! must obtain a *freshly loaded* copy. `dlopen` refcounts by inode, so we copy
//! each `.so` to a uniquely named temp file first; that guarantees a brand-new
//! copy of the library's `.data`/`.bss`.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void};
use std::fs;
use std::io::Read;
use std::os::fd::AsRawFd;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use libloading::{Library, Symbol};

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    /// `fflush(NULL)` flushes every open output stream, including the `stdout`
    /// FILE* shared by the test process and both loaded libraries.
    fn fflush(stream: *mut c_void) -> c_int;
    fn setvbuf(stream: *mut c_void, buf: *mut c_char, mode: c_int, size: usize) -> c_int;
}

/// Serialises the process-global `fd 1` swap performed by [`capture`].
fn stdout_lock() -> &'static Mutex<()> {
    static L: OnceLock<Mutex<()>> = OnceLock::new();
    L.get_or_init(|| Mutex::new(()))
}

static UNIQ: AtomicU64 = AtomicU64::new(0);

fn tmp_dir() -> PathBuf {
    // Honours $TMPDIR.
    let d = std::env::temp_dir().join("driver-difftest");
    fs::create_dir_all(&d).expect("create temp dir");
    d
}

/// `<manifest>/../c_src/build/libdriver.so`
fn c_so_path() -> PathBuf {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("manifest parent")
        .join("c_src/build/libdriver.so");
    assert!(
        p.is_file(),
        "C shared library not found at {p:?}; build it with:\n  cd c_src && mkdir -p build && cd build \\\n    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
    );
    assert_c_not_stale(&p);
    p
}

/// The cdylib produced by the *current* cargo invocation. The test binary lives
/// in `target/<profile>/deps/`, so the cdylib is one directory up.
fn rust_so_path() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    let deps = exe.parent().expect("deps dir");
    let candidates = [
        deps.join("libdriver.so"),
        deps.parent().map(|p| p.join("libdriver.so")).unwrap_or_default(),
    ];
    for c in &candidates {
        if c.is_file() {
            assert_not_stale(c);
            return c.clone();
        }
    }
    panic!("Rust cdylib libdriver.so not found near {deps:?} (tried {candidates:?})");
}

/// `cargo test` does NOT necessarily relink the `cdylib` (the integration tests
/// do not link against it), so a stale `libdriver.so` could silently be tested
/// instead of the current source. Refuse to run in that case.
fn assert_not_stale(so: &PathBuf) {
    assert_not_stale_against(so, PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src"), "rs")
}

fn assert_not_stale_against(so: &PathBuf, src: PathBuf, ext: &str) {
    let mtime = |p: &std::path::Path| {
        fs::metadata(p)
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
    };
    let so_t = mtime(so);
    let mut newest = std::time::SystemTime::UNIX_EPOCH;
    let mut newest_path = src.clone();
    let mut stack = vec![src];
    while let Some(dir) = stack.pop() {
        let Ok(rd) = fs::read_dir(&dir) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().map(|x| x == ext).unwrap_or(false) {
                let t = mtime(&p);
                if t > newest {
                    newest = t;
                    newest_path = p;
                }
            }
        }
    }
    assert!(
        so_t >= newest,
        "STALE cdylib: {so:?} is older than {newest_path:?}.\n\
         `cargo test` does not relink the cdylib. Run `cargo build --release` \
         (or use ./run_all.sh) before testing, otherwise the tests would pass \
         against an out-of-date library."
    );
}

/// Same staleness guard for the C side: the built `libdriver.so` must be at
/// least as new as every `.c` file cmake compiles into it.
fn assert_c_not_stale(so: &PathBuf) {
    let c_src = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("manifest parent")
        .join("c_src/src");
    assert_not_stale_against(so, c_src, "c");
}

/// A freshly loaded, independent pair of libraries.
pub struct Pair {
    pub c: Library,
    pub r: Library,
    _files: (PathBuf, PathBuf),
}

/// Copy `src` to a unique path so `dlopen` is forced to create a new mapping
/// (new inode => new copy of the library's mutable globals).
fn unique_copy(src: &PathBuf, tag: &str, side: &str) -> PathBuf {
    let n = UNIQ.fetch_add(1, Ordering::SeqCst);
    let dst = tmp_dir().join(format!("lib{tag}_{side}_{n}_{}.so", std::process::id()));
    fs::copy(src, &dst).expect("copy .so");
    dst
}

/// Load a pristine C+Rust pair. Each call yields libraries whose `the_house`
/// is at its initial value.
pub fn fresh_pair(tag: &str) -> Pair {
    let cf = unique_copy(&c_so_path(), tag, "c");
    let rf = unique_copy(&rust_so_path(), tag, "r");
    let c = unsafe { Library::new(&cf) }.expect("dlopen C lib");
    let r = unsafe { Library::new(&rf) }.expect("dlopen Rust lib");
    Pair { c, r, _files: (cf, rf) }
}

pub type VoidIntFn<'a> = Symbol<'a, unsafe extern "C" fn(c_int)>;

impl Pair {
    pub fn c_fn(&self, name: &str) -> VoidIntFn<'_> {
        unsafe { self.c.get(name.as_bytes()) }
            .unwrap_or_else(|e| panic!("C lib missing symbol {name:?}: {e}"))
    }
    pub fn r_fn(&self, name: &str) -> VoidIntFn<'_> {
        unsafe { self.r.get(name.as_bytes()) }
            .unwrap_or_else(|e| panic!("Rust lib missing symbol {name:?}: {e}"))
    }
}

/// Redirect `fd 1` to a temp file, run `f`, flush, restore, and return the bytes
/// written. Both libraries `printf` to the process `stdout`, so this captures
/// their output verbatim (including any formatting/rounding differences).
pub fn capture<F: FnOnce()>(f: F) -> Vec<u8> {
    let _guard = stdout_lock().lock().unwrap_or_else(|e| e.into_inner());

    let n = UNIQ.fetch_add(1, Ordering::SeqCst);
    let path = tmp_dir().join(format!("out_{}_{n}.txt", std::process::id()));
    let file = fs::File::create(&path).expect("create capture file");

    unsafe {
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(file.as_raw_fd(), 1) >= 0, "dup2 onto fd 1 failed");

        f();

        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "restore fd 1 failed");
        close(saved);
    }
    drop(file);

    let mut buf = Vec::new();
    fs::File::open(&path)
        .expect("reopen capture file")
        .read_to_end(&mut buf)
        .expect("read capture file");
    let _ = fs::remove_file(&path);
    buf
}

/// One operation in a call sequence: which exported entry point, with which arg.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    Run(c_int),
    Driver(c_int),
}

/// Replay `ops` against one side of a pair, capturing its stdout.
fn play(lib: &Library, ops: &[Op]) -> Vec<u8> {
    let run: VoidIntFn = unsafe { lib.get(b"run") }.expect("symbol run");
    let driver: VoidIntFn = unsafe { lib.get(b"driver") }.expect("symbol driver");
    capture(|| unsafe {
        for op in ops {
            match *op {
                Op::Run(x) => run(x),
                Op::Driver(x) => driver(x),
            }
        }
    })
}

/// Run the identical call sequence against a pristine C lib and a pristine Rust
/// lib and assert their stdout is byte-identical.
#[track_caller]
pub fn assert_seq_matches(tag: &str, ops: &[Op]) {
    let p = fresh_pair(tag);
    let c_out = play(&p.c, ops);
    let r_out = play(&p.r, ops);
    if c_out != r_out {
        panic!(
            "[{tag}] stdout divergence for {} op(s) (first ops: {:?})\n--- C ({} bytes) ---\n{}\n--- RUST ({} bytes) ---\n{}\n--- first differing byte: {:?} ---",
            ops.len(),
            &ops[..ops.len().min(8)],
            c_out.len(),
            String::from_utf8_lossy(&c_out[..c_out.len().min(2000)]),
            r_out.len(),
            String::from_utf8_lossy(&r_out[..r_out.len().min(2000)]),
            c_out
                .iter()
                .zip(r_out.iter())
                .position(|(a, b)| a != b)
                .map(|i| (i, c_out[i] as char, r_out[i] as char)),
        );
    }
    assert!(
        !ops.is_empty() || c_out.is_empty(),
        "[{tag}] empty op sequence must produce no output, got {:?}",
        String::from_utf8_lossy(&c_out)
    );
}

/// Convenience: a batch of independent single-`run` calls, each from pristine
/// state, compared one argument at a time.
pub fn assert_each_run(tag: &str, args: impl IntoIterator<Item = c_int>) {
    for x in args {
        assert_seq_matches(&format!("{tag}_run_{x}"), &[Op::Run(x)]);
    }
}

/// Convenience: a batch of independent single-`driver` calls from pristine state.
pub fn assert_each_driver(tag: &str, args: impl IntoIterator<Item = c_int>) {
    for x in args {
        assert_seq_matches(&format!("{tag}_driver_{x}"), &[Op::Driver(x)]);
    }
}

/// Deterministic SplitMix64 — fixed seed, no external crates, reproducible.
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
    pub fn next_i32(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }
    /// Inclusive range.
    pub fn in_range(&mut self, lo: i32, hi: i32) -> i32 {
        debug_assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
}
