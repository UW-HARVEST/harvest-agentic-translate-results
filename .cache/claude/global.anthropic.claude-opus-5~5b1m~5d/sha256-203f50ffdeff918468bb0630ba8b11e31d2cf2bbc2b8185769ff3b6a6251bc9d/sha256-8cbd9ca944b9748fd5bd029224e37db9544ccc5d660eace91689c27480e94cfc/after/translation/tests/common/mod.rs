//! Shared harness: loads BOTH the C `.so` and the Rust `.so` with `libloading`
//! and drives them exclusively through their exported symbols.
//!
//! Nothing in here calls a Rust function directly — every call goes through
//! `dlsym`, exactly like an external C consumer, so the `#[no_mangle]`
//! `extern "C"` wrappers are part of what is under test.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_void;
use std::path::PathBuf;

pub const ARRAY_SIZE: usize = 256 * 1024;

extern "C" {
    fn dup(oldfd: i32) -> i32;
    fn dup2(oldfd: i32, newfd: i32) -> i32;
    fn close(fd: i32) -> i32;
    fn fflush(stream: *mut c_void) -> i32;
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ has a parent")
        .to_path_buf()
}

pub fn c_so_path() -> PathBuf {
    let p = workspace_root().join("c_src/build/liblong.so");
    assert!(
        p.exists(),
        "C shared library not built: {}\nBuild it with:\n  cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

pub fn rust_so_path() -> PathBuf {
    let root = workspace_root().join("translation/target");
    for profile in ["release", "debug"] {
        let p = root.join(profile).join("liblong.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "Rust shared library not built under {}. Build it with: cargo build --release",
        root.display()
    );
}

/// `dlopen` refcounts by (path, inode): loading the *same* file twice in one
/// process hands back the *same* mapping, which means two tests running in
/// parallel would share the single global `array`. Copying the `.so` to a
/// unique path first gives every `Impl` a private, independent instance, so
/// tests are hermetic (and `array` really is freshly zeroed on load).
fn private_copy(src: &std::path::Path, tag: &str) -> PathBuf {
    static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dst = std::env::temp_dir().join(format!(
        "liblong_{tag}_{}_{}_{n}.so",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::copy(src, &dst)
        .unwrap_or_else(|e| panic!("copy {} -> {}: {e}", src.display(), dst.display()));
    dst
}

/// One loaded implementation, addressed only through its exported symbols.
pub struct Impl {
    pub name: &'static str,
    _lib: Library,
    tmp: Option<PathBuf>,
    array: *mut i32,
    perform: unsafe extern "C" fn(),
    long_exec: unsafe extern "C" fn(u32),
    /// Raw address of `long_exec`, so tests can also call it through
    /// deliberately mis-shaped function-pointer types (junk-argument tests).
    pub long_exec_addr: *const c_void,
    pub perform_addr: *const c_void,
}

impl Impl {
    pub fn load(name: &'static str, path: &std::path::Path) -> Impl {
        unsafe {
            let lib = Library::new(path).unwrap_or_else(|e| panic!("dlopen {}: {e}", path.display()));

            let array: Symbol<*mut i32> = lib.get(b"array\0").expect("symbol `array`");
            let array = *array;
            assert!(!array.is_null(), "{name}: `array` resolved to NULL");

            let perform: Symbol<unsafe extern "C" fn()> = lib
                .get(b"perform_expensive_operations\0")
                .expect("symbol `perform_expensive_operations`");
            let perform = *perform;
            let perform_addr = perform as *const c_void;

            let le: Symbol<unsafe extern "C" fn(u32)> =
                lib.get(b"long_exec\0").expect("symbol `long_exec`");
            let long_exec = *le;
            let long_exec_addr = long_exec as *const c_void;

            Impl {
                name,
                _lib: lib,
                tmp: None,
                array,
                perform,
                long_exec,
                long_exec_addr,
                perform_addr,
            }
        }
    }

    /// Load a private (hermetic) instance: the `.so` is copied to a unique path
    /// so `dlopen` cannot alias it with any other instance in this process.
    pub fn load_private(name: &'static str, path: &std::path::Path, tag: &str) -> Impl {
        let tmp = private_copy(path, tag);
        let mut me = Impl::load(name, &tmp);
        me.tmp = Some(tmp);
        me
    }

    pub fn c() -> Impl {
        Impl::load_private("C", &c_so_path(), "c")
    }

    pub fn rust() -> Impl {
        Impl::load_private("Rust", &rust_so_path(), "rs")
    }

    /// The exported global `array`, as a slice.
    pub fn array(&self) -> &[i32] {
        unsafe { std::slice::from_raw_parts(self.array, ARRAY_SIZE) }
    }

    pub fn array_mut(&self) -> &mut [i32] {
        unsafe { std::slice::from_raw_parts_mut(self.array, ARRAY_SIZE) }
    }

    /// The exported global `array`, as raw bytes (byte-for-byte comparison).
    pub fn array_bytes(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.array as *const u8, ARRAY_SIZE * 4) }
    }

    pub fn set_array(&self, src: &[i32]) {
        assert_eq!(src.len(), ARRAY_SIZE);
        self.array_mut().copy_from_slice(src);
    }

    pub unsafe fn perform(&self) {
        (self.perform)()
    }

    pub unsafe fn long_exec(&self, seed: u32) {
        (self.long_exec)(seed)
    }

    /// Call `long_exec` capturing everything it writes to fd 1.
    pub unsafe fn long_exec_capture(&self, seed: u32) -> Vec<u8> {
        capture_stdout(|| (self.long_exec)(seed))
    }
}

impl Drop for Impl {
    fn drop(&mut self) {
        if let Some(p) = self.tmp.take() {
            let _ = std::fs::remove_file(p);
        }
    }
}

/// Redirect fd 1 to a temp file for the duration of `f`, then return the bytes.
///
/// The loaded `.so` uses the very same glibc `stdout` FILE as this process, so
/// `fflush(NULL)` before restoring is enough to make the capture exact.
pub unsafe fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    let dir = std::env::temp_dir();
    let path = dir.join(format!(
        "long_stdout_{}_{:?}.txt",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let file = std::fs::File::create(&path).expect("create capture file");
    let fd = {
        use std::os::unix::io::AsRawFd;
        file.as_raw_fd()
    };

    fflush(std::ptr::null_mut());
    let saved = dup(1);
    assert!(saved >= 0, "dup(1) failed");
    assert!(dup2(fd, 1) >= 0, "dup2 failed");

    f();

    fflush(std::ptr::null_mut());
    assert!(dup2(saved, 1) >= 0, "restore dup2 failed");
    close(saved);
    drop(file);

    let out = std::fs::read(&path).expect("read capture file");
    let _ = std::fs::remove_file(&path);
    out
}

/// Deterministic PRNG (SplitMix64) so every "randomized" input is reproducible.
pub struct Rng(pub u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
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
    /// A value drawn from the "hard" distribution: boundary constants,
    /// multiples of 7, powers of two, and full-range randoms.
    pub fn hard_i32(&mut self) -> i32 {
        let r = self.next_u32();
        match r % 10 {
            0 => 0,
            1 => 1,
            2 => -1,
            3 => i32::MIN,
            4 => i32::MAX,
            5 => (self.next_i32() / 7).wrapping_mul(7),
            6 => 1i32.wrapping_shl(self.next_u32() % 32),
            7 => -(1i32.wrapping_shl(self.next_u32() % 32)),
            8 => (self.next_u32() % 15) as i32 - 7,
            _ => self.next_i32(),
        }
    }
}

/// Compare the two `array` globals byte-for-byte, reporting the first
/// divergence with full context.
#[track_caller]
pub fn assert_arrays_eq(c: &Impl, r: &Impl, what: &str) {
    let cb = c.array_bytes();
    let rb = r.array_bytes();
    if cb == rb {
        return;
    }
    let ca = c.array();
    let ra = r.array();
    let mut diffs = 0usize;
    let mut first = None;
    for i in 0..ARRAY_SIZE {
        if ca[i] != ra[i] {
            diffs += 1;
            if first.is_none() {
                first = Some(i);
            }
        }
    }
    let i = first.unwrap();
    panic!(
        "{what}: `array` diverged in {diffs}/{ARRAY_SIZE} elements.\n\
         first at index {i}: C = {} (0x{:08x}), Rust = {} (0x{:08x})",
        ca[i], ca[i] as u32, ra[i], ra[i] as u32
    );
}

/// Run one `perform_expensive_operations` differential: seed both `array`
/// globals with `input`, call both, compare byte-for-byte.
#[track_caller]
pub fn diff_perform(c: &Impl, r: &Impl, input: &[i32], what: &str) {
    c.set_array(input);
    r.set_array(input);
    unsafe {
        c.perform();
        r.perform();
    }
    assert_arrays_eq(c, r, what);
}
