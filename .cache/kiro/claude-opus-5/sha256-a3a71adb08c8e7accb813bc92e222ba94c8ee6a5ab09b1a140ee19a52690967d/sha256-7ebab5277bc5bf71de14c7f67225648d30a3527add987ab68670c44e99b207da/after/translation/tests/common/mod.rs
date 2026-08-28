//! Shared harness that loads both the C reference `.so` and the Rust `.so`
//! through `libloading` and exposes their exported symbols.
//!
//! Nothing in the Rust crate is called directly: every invocation goes through
//! `dlsym` on the produced shared object, so the `#[no_mangle]` export wrappers
//! are part of what gets tested.

use std::ffi::c_char;
use std::ffi::c_void;
use std::path::PathBuf;

use libloading::Library;
use libloading::Symbol;

unsafe extern "C" {
    /// Release a buffer produced by either library (both use the process libc).
    pub fn free(ptr: *mut c_void);
}

/// `char *custom_strdup(const char *)`
pub type CustomStrdupFn = unsafe extern "C" fn(*const c_char) -> *mut c_char;

/// Directory holding the workspace (parent of `translation/` and `c_src/`).
fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// The `target/<profile>` directory that the running test binary lives in.
///
/// Derived from the test executable path (`target/<profile>/deps/<name>-<hash>`)
/// so the Rust `.so` we load always matches the profile/features under test.
fn target_profile_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe must be available");
    exe.parent() // deps/
        .and_then(|p| p.parent()) // <profile>/
        .expect("test binary must live under target/<profile>/deps/")
        .to_path_buf()
}

/// Path to the C reference shared library, built by `c_src/CMakeLists.txt`.
fn c_library_path() -> PathBuf {
    let root = workspace_root();
    let candidates = [
        root.join("c_src/build/libdriver.so"),
        root.join("c_src/build/lib/libdriver.so"),
    ];
    for candidate in &candidates {
        if candidate.exists() {
            return candidate.clone();
        }
    }
    panic!(
        "C shared library not found; build it with:\n  cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .\ntried: {candidates:?}"
    );
}

/// Path to the Rust `cdylib` for the profile/feature set currently under test.
///
/// `cargo build` uplifts the artefact to `target/<profile>/`, while `cargo test`
/// leaves it in `target/<profile>/deps/`; both locations are checked.
fn rust_library_path() -> PathBuf {
    let profile_dir = target_profile_dir();
    let candidates = [
        profile_dir.join("deps/libdriver.so"),
        profile_dir.join("libdriver.so"),
    ];
    let mut newest: Option<(std::time::SystemTime, PathBuf)> = None;
    for candidate in &candidates {
        if let Ok(meta) = std::fs::metadata(candidate) {
            let mtime = meta.modified().unwrap_or(std::time::UNIX_EPOCH);
            if newest.as_ref().is_none_or(|(best, _)| mtime > *best) {
                newest = Some((mtime, candidate.clone()));
            }
        }
    }
    match newest {
        Some((_, path)) => path,
        None => panic!(
            "Rust cdylib not found; run `cargo build` for the same profile/features\ntried: {candidates:?}"
        ),
    }
}

/// Both implementations, loaded side by side for differential testing.
pub struct Both {
    // Kept alive: the function pointers below borrow from these handles.
    _c_lib: Library,
    _rust_lib: Library,
    c_strdup: CustomStrdupFn,
    rust_strdup: CustomStrdupFn,
}

impl Both {
    /// Loads the C and Rust shared objects and resolves every exported symbol.
    pub fn load() -> Self {
        let c_path = c_library_path();
        let rust_path = rust_library_path();

        // SAFETY: both paths point at shared objects built from this workspace;
        // neither runs initialisers with side effects.
        let c_lib = unsafe { Library::new(&c_path) }
            .unwrap_or_else(|e| panic!("failed to load {}: {e}", c_path.display()));
        let rust_lib = unsafe { Library::new(&rust_path) }
            .unwrap_or_else(|e| panic!("failed to load {}: {e}", rust_path.display()));

        // SAFETY: the signature matches `c_src/include/lib.h`.
        let c_strdup: Symbol<CustomStrdupFn> = unsafe { c_lib.get(b"custom_strdup\0") }
            .expect("C .so must export custom_strdup");
        let rust_strdup: Symbol<CustomStrdupFn> = unsafe { rust_lib.get(b"custom_strdup\0") }
            .expect("Rust .so must export custom_strdup");

        let c_strdup = *c_strdup;
        let rust_strdup = *rust_strdup;

        Self {
            _c_lib: c_lib,
            _rust_lib: rust_lib,
            c_strdup,
            rust_strdup,
        }
    }

    pub fn c_strdup(&self) -> CustomStrdupFn {
        self.c_strdup
    }

    pub fn rust_strdup(&self) -> CustomStrdupFn {
        self.rust_strdup
    }
}

/// Reads `len + 1` bytes (payload plus NUL terminator) from a returned buffer.
///
/// # Safety
///
/// `ptr` must be a non-null buffer holding at least `len + 1` readable bytes.
pub unsafe fn snapshot(ptr: *const c_char, len: usize) -> Vec<u8> {
    unsafe { std::slice::from_raw_parts(ptr as *const u8, len + 1) }.to_vec()
}

/// Runs both implementations over `input` (which must contain no interior NUL
/// before its final terminator being implied) and asserts byte-identical output.
///
/// `input` is the payload without a terminator; the harness appends the NUL.
pub fn assert_same(both: &Both, input: &[u8], label: &str) {
    let mut buf = Vec::with_capacity(input.len() + 1);
    buf.extend_from_slice(input);
    buf.push(0);

    let arg = buf.as_ptr() as *const c_char;

    // SAFETY: `arg` is a NUL-terminated buffer that outlives both calls.
    let c_out = unsafe { both.c_strdup()(arg) };
    let rust_out = unsafe { both.rust_strdup()(arg) };

    assert!(!c_out.is_null(), "[{label}] C returned NULL unexpectedly");
    assert!(
        !rust_out.is_null(),
        "[{label}] Rust returned NULL while C returned non-NULL"
    );

    // Distinct allocations: neither may alias the input or each other.
    assert_ne!(
        c_out as *const c_char, arg,
        "[{label}] C returned the input pointer"
    );
    assert_ne!(
        rust_out as *const c_char, arg,
        "[{label}] Rust returned the input pointer"
    );
    assert_ne!(
        c_out, rust_out,
        "[{label}] both libraries returned the same pointer"
    );

    // SAFETY: both buffers hold `input.len() + 1` bytes per the contract.
    let c_bytes = unsafe { snapshot(c_out, input.len()) };
    // SAFETY: as above.
    let rust_bytes = unsafe { snapshot(rust_out, input.len()) };

    assert_eq!(
        c_bytes, rust_bytes,
        "[{label}] output bytes differ (len {})",
        input.len()
    );
    assert_eq!(
        &c_bytes[..input.len()],
        input,
        "[{label}] C output does not match the input payload"
    );
    assert_eq!(
        c_bytes[input.len()], 0,
        "[{label}] missing NUL terminator in C output"
    );

    // SAFETY: both pointers came from `malloc` inside the loaded libraries.
    unsafe {
        free(c_out as *mut c_void);
        free(rust_out as *mut c_void);
    }
}

/// Deterministic pseudo-random byte generator (xorshift64*), used so failures
/// reproduce exactly. Never yields `0`, so payloads stay NUL-free.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed | 1)
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// A byte in `1..=255` (excludes NUL so it can appear inside a payload).
    pub fn next_nonzero_byte(&mut self) -> u8 {
        ((self.next_u64() >> 24) as u8 % 255) + 1
    }

    pub fn next_below(&mut self, bound: usize) -> usize {
        (self.next_u64() % bound as u64) as usize
    }
}
