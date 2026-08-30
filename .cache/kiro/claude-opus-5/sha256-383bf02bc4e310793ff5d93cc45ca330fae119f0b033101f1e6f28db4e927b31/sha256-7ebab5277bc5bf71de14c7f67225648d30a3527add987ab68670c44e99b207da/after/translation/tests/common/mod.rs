//! Shared harness for the differential C-vs-Rust tests.
//!
//! Both implementations are loaded as shared objects through `libloading` and
//! driven exclusively through their exported symbols, so the `#[no_mangle]` /
//! `#[export_name]` wrappers are part of what is under test.

#![allow(dead_code)]

use std::ffi::{c_int, c_uint, c_void};
use std::io::Write;
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

use libloading::{Library, Symbol};

/// `#define ARRAY_SIZE (256 * 1024)` from `c_src/src/long.c`.
pub const ARRAY_SIZE: usize = 256 * 1024;

/// One of the two implementations, loaded from disk.
pub struct Impl {
    /// Kept alive for as long as the symbols are used.
    _lib: Library,
    name: &'static str,
    array: *mut c_int,
    perform_expensive_operations: unsafe extern "C" fn(),
    long_exec: unsafe extern "C" fn(c_uint),
    fflush: unsafe extern "C" fn(*mut c_void) -> c_int,
    dup: unsafe extern "C" fn(c_int) -> c_int,
    dup2: unsafe extern "C" fn(c_int, c_int) -> c_int,
    close: unsafe extern "C" fn(c_int) -> c_int,
}

impl Impl {
    fn load(name: &'static str, path: &Path) -> Impl {
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("failed to dlopen {} ({}): {e}", path.display(), name));

        // For data symbols `Symbol<*mut T>` dereferences to the address of the
        // symbol itself, which is the base of the `array` global.
        let array: *mut c_int = unsafe {
            let sym: Symbol<*mut c_int> = lib
                .get(b"array\0")
                .unwrap_or_else(|e| panic!("{name}: missing `array` symbol: {e}"));
            *sym
        };
        assert!(!array.is_null(), "{name}: `array` resolved to NULL");

        let perform_expensive_operations = unsafe {
            let sym: Symbol<unsafe extern "C" fn()> = lib
                .get(b"perform_expensive_operations\0")
                .unwrap_or_else(|e| panic!("{name}: missing `perform_expensive_operations`: {e}"));
            *sym
        };

        let long_exec = unsafe {
            let sym: Symbol<unsafe extern "C" fn(c_uint)> = lib
                .get(b"long_exec\0")
                .unwrap_or_else(|e| panic!("{name}: missing `long_exec`: {e}"));
            *sym
        };

        // `fflush` comes from the libc that the object under test is linked
        // against, so the C `printf` buffer can be drained deterministically.
        let fflush = unsafe {
            let sym: Symbol<unsafe extern "C" fn(*mut c_void) -> c_int> = lib
                .get(b"fflush\0")
                .unwrap_or_else(|e| panic!("{name}: could not resolve `fflush`: {e}"));
            *sym
        };

        let dup = unsafe {
            let sym: Symbol<unsafe extern "C" fn(c_int) -> c_int> = lib
                .get(b"dup\0")
                .unwrap_or_else(|e| panic!("{name}: could not resolve `dup`: {e}"));
            *sym
        };
        let dup2 = unsafe {
            let sym: Symbol<unsafe extern "C" fn(c_int, c_int) -> c_int> = lib
                .get(b"dup2\0")
                .unwrap_or_else(|e| panic!("{name}: could not resolve `dup2`: {e}"));
            *sym
        };
        let close = unsafe {
            let sym: Symbol<unsafe extern "C" fn(c_int) -> c_int> = lib
                .get(b"close\0")
                .unwrap_or_else(|e| panic!("{name}: could not resolve `close`: {e}"));
            *sym
        };

        Impl {
            _lib: lib,
            name,
            array,
            perform_expensive_operations,
            long_exec,
            fflush,
            dup,
            dup2,
            close,
        }
    }

    pub fn name(&self) -> &'static str {
        self.name
    }

    /// Overwrite the whole global array.
    pub fn write_array(&self, values: &[c_int]) {
        assert_eq!(values.len(), ARRAY_SIZE);
        unsafe { std::ptr::copy_nonoverlapping(values.as_ptr(), self.array, ARRAY_SIZE) };
    }

    /// Snapshot the whole global array.
    pub fn read_array(&self) -> Vec<c_int> {
        let mut out = vec![0; ARRAY_SIZE];
        unsafe { std::ptr::copy_nonoverlapping(self.array, out.as_mut_ptr(), ARRAY_SIZE) };
        out
    }

    /// Raw bytes of the global array, for byte-for-byte comparison.
    pub fn read_array_bytes(&self) -> Vec<u8> {
        let mut out = vec![0u8; ARRAY_SIZE * std::mem::size_of::<c_int>()];
        unsafe { std::ptr::copy_nonoverlapping(self.array.cast::<u8>(), out.as_mut_ptr(), out.len()) };
        out
    }

    pub fn perform_expensive_operations(&self) {
        unsafe { (self.perform_expensive_operations)() };
    }

    pub fn long_exec(&self, seed: c_uint) {
        unsafe { (self.long_exec)(seed) };
    }

    pub fn flush_all(&self) {
        unsafe { (self.fflush)(std::ptr::null_mut()) };
    }

    /// Run `long_exec` with file descriptor 1 redirected into a temporary file
    /// and return everything the library printed. This is what makes the
    /// `printf("%d\n", ...)` at the tail of `long_exec` observable.
    pub fn capture_long_exec(&self, seed: c_uint) -> Vec<u8> {
        let tmp = target_dir().join(format!("difftest-stdout-{}-{seed}.bin", self.name));
        let bytes = {
            let file = std::fs::File::create(&tmp).expect("create capture file");
            let fd = file.as_raw_fd();

            // Drain anything already buffered on either side before swapping fd 1.
            std::io::stdout().flush().ok();
            self.flush_all();

            let saved = unsafe { (self.dup)(1) };
            assert!(saved >= 0, "dup(1) failed");
            assert!(unsafe { (self.dup2)(fd, 1) } >= 0, "dup2 onto stdout failed");

            self.long_exec(seed);

            self.flush_all();
            assert!(unsafe { (self.dup2)(saved, 1) } >= 0, "restoring stdout failed");
            unsafe { (self.close)(saved) };
            drop(file);

            std::fs::read(&tmp).expect("read capture file")
        };
        let _ = std::fs::remove_file(&tmp);
        bytes
    }
}

pub fn target_dir() -> PathBuf {
    // .../target/<profile>/deps/<test binary>
    let exe = std::env::current_exe().expect("current_exe");
    let deps = exe.parent().expect("deps dir");
    deps.parent().expect("profile dir").to_path_buf()
}

pub fn rust_so_path() -> PathBuf {
    let candidate = target_dir().join("liblong.so");
    assert!(
        candidate.exists(),
        "rust cdylib not found at {}\n\
         `cargo test` does not build the cdylib artifact by itself; run \
         ./run_difftests.sh (or `cargo build` with the same profile) first",
        candidate.display()
    );
    candidate
}

pub fn c_so_path() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = manifest.parent().expect("workspace root");
    let candidate = root.join("c_src").join("build").join("liblong.so");
    assert!(
        candidate.exists(),
        "C shared library not found at {}\n\
         build it with: cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        candidate.display()
    );
    candidate
}

/// The two implementations are copied to distinct file names first. The C
/// object carries `SONAME = liblong.so` while the Rust one carries none, so
/// they would not actually alias, but copying keeps the loader unambiguous.
fn load_both_uncached() -> (Impl, Impl) {
    let staging = target_dir().join("difftest-libs");
    std::fs::create_dir_all(&staging).expect("create staging dir");

    let c_dst = staging.join("liblong_c_impl.so");
    let rust_dst = staging.join("liblong_rust_impl.so");
    copy_if_changed(&c_so_path(), &c_dst);
    copy_if_changed(&rust_so_path(), &rust_dst);

    let c = Impl::load("C", &c_dst);
    let rust = Impl::load("Rust", &rust_dst);
    assert_ne!(
        c.array, rust.array,
        "the two libraries resolved to the same `array` — they were not loaded independently"
    );
    (c, rust)
}

/// Load an additional implementation from an arbitrary path, for comparing
/// alternative builds of the same source. The caller must already hold the lock
/// returned by [`load_both`], since all of these objects share one `stdout` and
/// each has its own mutable `array`.
pub fn load_extra(name: &'static str, path: &Path) -> Impl {
    Impl::load(name, path)
}

// `Impl` holds raw pointers into the loaded objects. They are only ever touched
// while the process-wide lock below is held, one thread at a time.
unsafe impl Send for Impl {}

static LIBS: OnceLock<Mutex<(Impl, Impl)>> = OnceLock::new();

/// Both libraries live in one address space and each exposes a *single* mutable
/// `array` global, so `cargo test`'s parallel threads must not touch them
/// concurrently. Every test acquires this lock for its whole body.
pub fn load_both() -> MutexGuard<'static, (Impl, Impl)> {
    LIBS.get_or_init(|| Mutex::new(load_both_uncached()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn copy_if_changed(src: &Path, dst: &Path) {
    let need = match (std::fs::metadata(src), std::fs::metadata(dst)) {
        (Ok(s), Ok(d)) => s.len() != d.len() || s.modified().ok() > d.modified().ok(),
        _ => true,
    };
    if need {
        // Remove first: overwriting a file that is already mapped would be bad.
        let _ = std::fs::remove_file(dst);
        std::fs::copy(src, dst).unwrap_or_else(|e| {
            panic!("copy {} -> {}: {e}", src.display(), dst.display())
        });
    }
}

/// Deterministic generator used to build test payloads. This is *not* meant to
/// mimic `rand()`; it only has to produce the same bytes for both libraries.
pub struct SplitMix64(pub u64);

impl SplitMix64 {
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    pub fn next_i32(&mut self) -> c_int {
        self.next_u64() as u32 as c_int
    }
}

/// libtest writes its own "test <name> has been running for over 60 seconds"
/// progress notices to file descriptor 1, which the capture in
/// `Impl::capture_long_exec` unavoidably picks up because `printf` writes there
/// too. Those notices are timing dependent, so they are removed before the two
/// recordings are compared; everything the library itself printed is kept
/// byte-for-byte.
pub fn strip_harness_noise(bytes: &[u8]) -> Vec<u8> {
    const NOISE: &[u8] = b"has been running for over";
    let mut out = Vec::with_capacity(bytes.len());
    for line in bytes.split_inclusive(|&b| b == b'\n') {
        if line.windows(NOISE.len()).any(|w| w == NOISE) {
            continue;
        }
        out.extend_from_slice(line);
    }
    out
}

/// Report the first differing element, with context, instead of dumping 1MB.
pub fn assert_arrays_equal(context: &str, c: &[c_int], rust: &[c_int]) {
    assert_eq!(c.len(), rust.len(), "{context}: length mismatch");
    if c == rust {
        return;
    }
    let idx = c
        .iter()
        .zip(rust.iter())
        .position(|(a, b)| a != b)
        .expect("slices differ");
    let diffs = c.iter().zip(rust.iter()).filter(|(a, b)| a != b).count();
    panic!(
        "{context}: {diffs} of {} elements differ; first at index {idx}: C = {} (0x{:08x}), Rust = {} (0x{:08x})",
        c.len(),
        c[idx],
        c[idx] as u32,
        rust[idx],
        rust[idx] as u32
    );
}
