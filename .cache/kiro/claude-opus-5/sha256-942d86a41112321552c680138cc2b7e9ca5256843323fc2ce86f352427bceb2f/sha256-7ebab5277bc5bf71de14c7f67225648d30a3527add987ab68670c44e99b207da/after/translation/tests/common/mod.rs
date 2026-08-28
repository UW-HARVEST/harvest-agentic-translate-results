//! Shared harness: loads the C and the Rust shared objects side by side and
//! provides stdout capture so that both the return values *and* the emitted
//! bytes can be compared.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// libc bits we need for the stdout capture
// ---------------------------------------------------------------------------

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn malloc(size: usize) -> *mut c_void;
    fn free(p: *mut c_void);
}

/// `fflush(NULL)` flushes every open output stream, which avoids having to
/// resolve the `stdout` global (its symbol name differs between libcs).
fn flush_all() {
    unsafe {
        fflush(std::ptr::null_mut());
    }
}

// ---------------------------------------------------------------------------
// Mirror of the C `ProcessState` so tests can inspect/construct states.
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy)]
pub struct StateView {
    pub flags: u32,
    pub data: u32,
    pub buffer: *mut c_char,
    pub capacity: c_int,
}

impl StateView {
    pub fn flag1(&self) -> u32 {
        self.flags & 1
    }
    pub fn flag2(&self) -> u32 {
        (self.flags >> 1) & 1
    }
    pub fn flag3(&self) -> u32 {
        (self.flags >> 2) & 1
    }
    pub fn counter(&self) -> u32 {
        (self.flags >> 3) & 0x1F
    }
    pub fn mode(&self) -> u32 {
        (self.flags >> 8) & 0x7
    }
    pub fn status(&self) -> u32 {
        (self.flags >> 11) & 0x1F
    }
    pub fn reserved(&self) -> u32 {
        (self.flags >> 16) & 0xFFFF
    }
}

/// Copy of the observable state, with the pointer replaced by the bytes it
/// points at, so two implementations can be compared field by field.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct StateSnapshot {
    pub flags: u32,
    pub data: u32,
    pub capacity: c_int,
    pub buffer_null: bool,
    pub buffer_cstr: Vec<u8>,
}

// ---------------------------------------------------------------------------
// Library discovery
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    // translation/ -> repository root
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("manifest dir has a parent")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    // Allows pointing the suite at a differently-optimised C build.
    if let Ok(p) = std::env::var("C2RUST_C_SO") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "C2RUST_C_SO={} does not exist", p.display());
        return p;
    }
    let build = workspace_root().join("c_src").join("build");
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
    candidates.into_iter().next().unwrap_or_else(|| {
        panic!(
            "no .so found in {}; build the C library first:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

fn find_rust_so() -> PathBuf {
    // target/<profile>/deps/<test-bin> -> target/<profile>
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("test exe lives in target/<profile>/deps")
        .to_path_buf();

    // `cargo test` does not build `cdylib` artifacts, so build the shared
    // object here.  This must run unconditionally: a stale `.so` left over
    // from an earlier run would otherwise be tested instead of the current
    // sources.  The outer cargo has already released its build lock by the
    // time tests run, and an up-to-date build is a no-op.
    build_cdylib(&profile_dir);

    let direct = profile_dir.join("libconfusion_lib.so");
    if direct.exists() {
        return direct;
    }
    panic!(
        "Rust cdylib not found at {} even after `cargo build`.",
        direct.display()
    );
}

fn build_cdylib(profile_dir: &Path) {
    let target_dir = profile_dir
        .parent()
        .expect("profile dir has a parent")
        .to_path_buf();
    let profile_name = profile_dir
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("debug")
        .to_string();

    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let mut cmd = std::process::Command::new(cargo);
    cmd.arg("build")
        .arg("--lib")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .env("CARGO_TARGET_DIR", &target_dir)
        .env_remove("RUSTFLAGS");
    if profile_name != "debug" {
        cmd.arg("--profile").arg(&profile_name);
    }
    // Mirror the feature selection the test binary itself was compiled with.
    let features = enabled_features();
    cmd.arg("--no-default-features");
    if !features.is_empty() {
        cmd.arg("--features").arg(features.join(","));
    }

    let out = cmd.output().expect("spawn cargo build for the cdylib");
    if !out.status.success() {
        panic!(
            "cargo build --lib failed:\n{}\n{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
    }
}

/// Features active in this test binary.  The crate currently declares none, so
/// this is empty; the hook keeps the harness correct if any are added later.
fn enabled_features() -> Vec<String> {
    Vec::new()
}

// ---------------------------------------------------------------------------
// Function pointer table for one implementation
// ---------------------------------------------------------------------------

type FnCreateState = unsafe extern "C" fn(c_int, c_int) -> *mut StateView;
type FnDestroyState = unsafe extern "C" fn(*mut StateView);
type FnProcessBuffer = unsafe extern "C" fn(*mut StateView, c_char) -> c_int;
type FnUpdateFlags = unsafe extern "C" fn(*mut StateView, c_int);
type FnConfuseTypes = unsafe extern "C" fn(*mut StateView, c_int) -> c_int;
type FnConfusion = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

pub struct Impl {
    pub name: &'static str,
    _lib: Library,
    pub create_state: FnCreateState,
    pub destroy_state: FnDestroyState,
    pub process_buffer: FnProcessBuffer,
    pub update_flags: FnUpdateFlags,
    pub confuse_types: FnConfuseTypes,
    pub confusion: FnConfusion,
}

impl Impl {
    unsafe fn load(name: &'static str, path: &Path) -> Impl {
        let lib = Library::new(path).unwrap_or_else(|e| panic!("loading {}: {e}", path.display()));
        macro_rules! sym {
            ($t:ty, $n:literal) => {{
                let s: Symbol<$t> = lib
                    .get($n)
                    .unwrap_or_else(|e| panic!("{} missing symbol {}: {e}", name,
                                               String::from_utf8_lossy($n)));
                *s
            }};
        }
        let create_state = sym!(FnCreateState, b"create_state\0");
        let destroy_state = sym!(FnDestroyState, b"destroy_state\0");
        let process_buffer = sym!(FnProcessBuffer, b"process_buffer\0");
        let update_flags = sym!(FnUpdateFlags, b"update_flags\0");
        let confuse_types = sym!(FnConfuseTypes, b"confuse_types\0");
        let confusion = sym!(FnConfusion, b"confusion\0");
        Impl {
            name,
            _lib: lib,
            create_state,
            destroy_state,
            process_buffer,
            update_flags,
            confuse_types,
            confusion,
        }
    }
}

pub struct Pair {
    pub c: Impl,
    pub rs: Impl,
}

static PAIR: OnceLock<Pair> = OnceLock::new();

pub fn pair() -> &'static Pair {
    PAIR.get_or_init(|| unsafe {
        Pair {
            c: Impl::load("C", &find_c_so()),
            rs: Impl::load("Rust", &find_rust_so()),
        }
    })
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

/// Runs `f` with fd 1 redirected into a temporary file and returns
/// `(result, captured_bytes)`.
///
/// Both shared objects resolve `printf` to the one `libc.so.6` mapped into
/// this process, so they share the same `stdout` stream; flushing before and
/// after is enough to get an exact byte capture.
pub fn capture<T, F: FnOnce() -> T>(f: F) -> (T, Vec<u8>) {
    use std::io::{Read, Seek, SeekFrom};
    use std::os::unix::io::AsRawFd;

    // fd 1 is process-wide state, so only one capture may be in flight.
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());

    flush_all();

    let mut tmp = tempfile();
    let tmp_fd = tmp.as_raw_fd();

    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { dup2(tmp_fd, 1) } >= 0, "dup2 failed");

    let result = f();

    flush_all();

    assert!(unsafe { dup2(saved, 1) } >= 0, "dup2 restore failed");
    unsafe { close(saved) };

    let mut buf = Vec::new();
    tmp.seek(SeekFrom::Start(0)).expect("seek");
    tmp.read_to_end(&mut buf).expect("read capture");
    (result, buf)
}

fn tempfile() -> std::fs::File {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "c2rust-capture-{}-{}-{}.txt",
        std::process::id(),
        n,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .subsec_nanos()
    ));
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .expect("create temp capture file");
    // Unlink immediately; the fd keeps it alive.
    let _ = std::fs::remove_file(&path);
    file
}

// ---------------------------------------------------------------------------
// State helpers
// ---------------------------------------------------------------------------

/// Builds a heap `ProcessState` (via libc `malloc`, so either library may
/// free it) with fully controlled contents.
pub struct OwnedState {
    pub ptr: *mut StateView,
    buf: *mut c_void,
}

impl OwnedState {
    /// `buffer` is written as a NUL-terminated C string into a fresh
    /// allocation of `alloc_len` bytes (padded with `filler`).
    pub fn new(flags: u32, data: u32, capacity: c_int, buffer: Option<(&[u8], usize, u8)>) -> Self {
        unsafe {
            let ptr = malloc(std::mem::size_of::<StateView>()) as *mut StateView;
            assert!(!ptr.is_null());
            std::ptr::write_bytes(ptr as *mut u8, 0, std::mem::size_of::<StateView>());
            (*ptr).flags = flags;
            (*ptr).data = data;
            (*ptr).capacity = capacity;
            let mut buf = std::ptr::null_mut();
            match buffer {
                None => (*ptr).buffer = std::ptr::null_mut(),
                Some((bytes, alloc_len, filler)) => {
                    let len = alloc_len.max(bytes.len() + 1);
                    buf = malloc(len);
                    assert!(!buf.is_null());
                    std::ptr::write_bytes(buf as *mut u8, filler, len);
                    std::ptr::copy_nonoverlapping(bytes.as_ptr(), buf as *mut u8, bytes.len());
                    *(buf as *mut u8).add(bytes.len()) = 0;
                    (*ptr).buffer = buf as *mut c_char;
                }
            }
            OwnedState { ptr, buf }
        }
    }
}

impl Drop for OwnedState {
    fn drop(&mut self) {
        unsafe {
            if !self.buf.is_null() {
                free(self.buf);
            }
            if !self.ptr.is_null() {
                free(self.ptr as *mut c_void);
            }
        }
    }
}

pub fn snapshot(p: *const StateView) -> Option<StateSnapshot> {
    if p.is_null() {
        return None;
    }
    unsafe {
        let s = &*p;
        let buffer_null = s.buffer.is_null();
        let mut cstr = Vec::new();
        if !buffer_null {
            let mut i = 0isize;
            loop {
                let b = *(s.buffer.offset(i)) as u8;
                if b == 0 {
                    break;
                }
                cstr.push(b);
                i += 1;
                assert!(i < 1 << 20, "unterminated buffer");
            }
        }
        Some(StateSnapshot {
            flags: s.flags,
            data: s.data,
            capacity: s.capacity,
            buffer_null,
            buffer_cstr: cstr,
        })
    }
}

pub fn show(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// Asserts two stdout captures are byte-identical, with a readable diff.
#[track_caller]
pub fn assert_stdout_eq(ctx: &str, c: &[u8], rs: &[u8]) {
    if c != rs {
        let first_diff = c
            .iter()
            .zip(rs.iter())
            .position(|(a, b)| a != b)
            .unwrap_or(c.len().min(rs.len()));
        panic!(
            "stdout mismatch for {ctx}\n\
             first differing byte offset: {first_diff}\n\
             --- C ({} bytes) ---\n{}\n\
             --- Rust ({} bytes) ---\n{}\n",
            c.len(),
            show(c),
            rs.len(),
            show(rs)
        );
    }
}
