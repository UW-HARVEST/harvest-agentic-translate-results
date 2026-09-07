//! Shared differential-test harness.
//!
//! Both the C `.so` and the Rust `.so` are loaded with `libloading` and called
//! only through their exported `wcscat` symbol, exactly as an external consumer
//! would. The Rust functions are never called directly, so the `#[no_mangle]`
//! `extern "C"` wrapper is part of what is under test.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// `wchar_t` on the C build target (Linux/x86-64 glibc): 4-byte **signed**.
pub type WcharT = i32;

pub type WcscatFn = unsafe extern "C" fn(*mut WcharT, usize, *const WcharT) -> i32;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Locate the C shared library. The CMake project (and therefore the `.so`)
/// is named after the parent directory, so scan for it instead of hardcoding.
fn find_c_lib() -> PathBuf {
    if let Ok(p) = std::env::var("C_LIB") {
        return PathBuf::from(p);
    }
    let build_dir = manifest_dir().join("../c_src/build");
    let mut found: Option<PathBuf> = None;
    if let Ok(entries) = std::fs::read_dir(&build_dir) {
        for e in entries.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("so") {
                found = Some(p);
                break;
            }
        }
    }
    found.unwrap_or_else(|| {
        panic!(
            "no C .so found in {}. Build it first:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build_dir.display()
        )
    })
}

/// Locate the Rust cdylib that corresponds to the profile this test binary was
/// built under. The test executable lives in `target/<profile>/deps/`, so its
/// grandparent directory is `target/<profile>/`.
fn find_rust_lib() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_LIB") {
        return PathBuf::from(p);
    }
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir: &Path = exe
        .parent()
        .and_then(|deps| deps.parent())
        .expect("target/<profile>");
    let candidate = profile_dir.join("libwcscat_lib.so");
    if candidate.exists() {
        return candidate;
    }
    for prof in ["debug", "release"] {
        let c = manifest_dir().join("target").join(prof).join("libwcscat_lib.so");
        if c.exists() {
            return c;
        }
    }
    panic!(
        "libwcscat_lib.so not found near {}.\n\
         `cargo test` does NOT build a `cdylib` target, so the .so must be built \
         explicitly first:\n  cargo build --offline   (or use ./run_tests.sh)",
        profile_dir.display()
    );
}

fn newest_mtime(paths: &[PathBuf]) -> Option<std::time::SystemTime> {
    paths
        .iter()
        .filter_map(|p| std::fs::metadata(p).ok()?.modified().ok())
        .max()
}

/// Hard guard against the single most dangerous failure mode of this harness:
/// `cargo test` does **not** build `crate-type = ["cdylib"]` artifacts, so
/// without an explicit `cargo build` the tests would happily dlopen a **stale**
/// `.so` and report a green run while the current `src/lib.rs` is broken.
/// Refuse to run if the loaded `.so` is older than the Rust sources.
fn assert_rust_lib_fresh(so: &Path) {
    if std::env::var("ALLOW_STALE_RUST_LIB").is_ok() {
        return;
    }
    let src_dir = manifest_dir().join("src");
    let mut sources = vec![manifest_dir().join("Cargo.toml")];
    if let Ok(entries) = std::fs::read_dir(&src_dir) {
        for e in entries.flatten() {
            if e.path().extension().and_then(|s| s.to_str()) == Some("rs") {
                sources.push(e.path());
            }
        }
    }

    let so_time = std::fs::metadata(so)
        .and_then(|m| m.modified())
        .unwrap_or_else(|e| panic!("cannot stat {}: {e}", so.display()));

    if let Some(src_time) = newest_mtime(&sources) {
        assert!(
            so_time >= src_time,
            "STALE Rust .so detected -- refusing to run a vacuous differential test.\n  \
             .so : {} ({:?})\n  src : newer than the .so ({:?})\n\n\
             `cargo test` does not rebuild a cdylib. Rebuild first:\n  \
             cargo build --offline\nor run ./run_tests.sh which does it for you.",
            so.display(),
            so_time,
            src_time
        );
    }
}

pub struct Libs {
    _c: Library,
    _rust: Library,
    pub c_wcscat: WcscatFn,
    pub rust_wcscat: WcscatFn,
}

// The loaded function pointers are plain `extern "C"` code with no shared
// mutable state; the `Library` handles are kept alive for the process lifetime.
unsafe impl Sync for Libs {}
unsafe impl Send for Libs {}

static LIBS: OnceLock<Libs> = OnceLock::new();

pub fn libs() -> &'static Libs {
    LIBS.get_or_init(|| unsafe {
        let c_path = find_c_lib();
        let rust_path = find_rust_lib();
        assert_rust_lib_fresh(&rust_path);
        eprintln!(
            "[harness] C   .so: {}\n[harness] Rust .so: {}",
            c_path.display(),
            rust_path.display()
        );

        let c = Library::new(&c_path)
            .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", c_path.display()));
        let rust = Library::new(&rust_path)
            .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", rust_path.display()));

        // Resolve `wcscat` from each handle explicitly. `dlsym` on a library
        // handle searches that library (and its dependencies) first, so the C
        // handle yields the C definition and the Rust handle the Rust one --
        // even though the name collides with libc's `wcscat`.
        let c_sym: Symbol<WcscatFn> = c
            .get(b"wcscat\0")
            .expect("C .so does not export `wcscat`");
        let rust_sym: Symbol<WcscatFn> = rust
            .get(b"wcscat\0")
            .expect("Rust .so does not export `wcscat`");

        let c_wcscat = *c_sym;
        let rust_wcscat = *rust_sym;

        // Sanity check: the two resolved addresses must differ, otherwise both
        // handles collapsed onto the same (e.g. libc) definition and every
        // comparison below would be vacuous.
        assert_ne!(
            c_wcscat as usize, rust_wcscat as usize,
            "C and Rust `wcscat` resolved to the SAME address -- the differential \
             test would be meaningless"
        );

        Libs {
            _c: c,
            _rust: rust,
            c_wcscat,
            rust_wcscat,
        }
    })
}

/// One differential test case.
#[derive(Clone, Debug)]
pub struct Case {
    /// Initial contents of the destination buffer, including any bytes past
    /// `num_elem` (used to prove the C never writes outside the window).
    pub dst: Vec<WcharT>,
    /// Pass a NULL `dst` instead of a pointer to the buffer above.
    pub dst_null: bool,
    /// The `numElem` argument, passed through verbatim (including 0 and
    /// deliberately oversized values).
    pub num_elem: usize,
    /// Source string contents. Must be NUL-terminated by the caller when a
    /// terminated string is intended; passed verbatim otherwise.
    pub src: Vec<WcharT>,
    /// Pass a NULL `src`.
    pub src_null: bool,
}

impl Case {
    /// `dst` and a NUL-terminated `src`, with `num_elem == dst.len()`.
    pub fn new(dst: Vec<WcharT>, src_body: &[WcharT]) -> Self {
        let num_elem = dst.len();
        let mut src = src_body.to_vec();
        src.push(0);
        Case {
            dst,
            dst_null: false,
            num_elem,
            src,
            src_null: false,
        }
    }

    pub fn num_elem(mut self, n: usize) -> Self {
        self.num_elem = n;
        self
    }
    pub fn dst_null(mut self) -> Self {
        self.dst_null = true;
        self
    }
    pub fn src_null(mut self) -> Self {
        self.src_null = true;
        self
    }
    /// Provide `src` verbatim, without appending a terminator.
    pub fn raw_src(mut self, src: Vec<WcharT>) -> Self {
        self.src = src;
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Outcome {
    pub ret: i32,
    /// The full destination buffer after the call (unchanged when NULL was passed).
    pub dst: Vec<WcharT>,
}

fn invoke(f: WcscatFn, case: &Case) -> Outcome {
    // A private copy of the destination per invocation, so C and Rust each get
    // identical starting state.
    let mut dst = case.dst.clone();
    let src = case.src.clone();

    let dst_ptr: *mut WcharT = if case.dst_null {
        std::ptr::null_mut()
    } else {
        dst.as_mut_ptr()
    };
    let src_ptr: *const WcharT = if case.src_null {
        std::ptr::null()
    } else {
        src.as_ptr()
    };

    let ret = unsafe { f(dst_ptr, case.num_elem, src_ptr) };
    Outcome { ret, dst }
}

/// Run one case through both libraries and assert byte-identical results.
#[track_caller]
pub fn assert_same(label: &str, case: &Case) {
    let l = libs();
    let c = invoke(l.c_wcscat, case);
    let r = invoke(l.rust_wcscat, case);

    assert_eq!(
        c.ret, r.ret,
        "{label}: return code mismatch (C={} Rust={})\n  num_elem={} dst_null={} src_null={}\n  \
         dst_in={:?}\n  src_in={:?}",
        c.ret, r.ret, case.num_elem, case.dst_null, case.src_null, case.dst, case.src
    );

    assert_eq!(
        c.dst, r.dst,
        "{label}: destination buffer mismatch\n  num_elem={} dst_null={} src_null={}\n  \
         dst_in ={:?}\n  src_in ={:?}\n  C_out  ={:?}\n  Rustout={:?}",
        case.num_elem, case.dst_null, case.src_null, case.dst, case.src, c.dst, r.dst
    );

    // Byte-for-byte comparison of the raw memory images, not just the i32 values.
    let c_bytes: &[u8] = unsafe {
        std::slice::from_raw_parts(c.dst.as_ptr() as *const u8, std::mem::size_of_val(&c.dst[..]))
    };
    let r_bytes: &[u8] = unsafe {
        std::slice::from_raw_parts(r.dst.as_ptr() as *const u8, std::mem::size_of_val(&r.dst[..]))
    };
    assert_eq!(c_bytes, r_bytes, "{label}: raw byte image mismatch");
}

/// Deterministic xorshift64* PRNG so every randomized row is reproducible.
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
    /// Uniform-ish value in `lo..=hi`.
    pub fn range(&mut self, lo: usize, hi: usize) -> usize {
        assert!(lo <= hi);
        lo + (self.next_u64() % ((hi - lo + 1) as u64)) as usize
    }
    pub fn i32(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }
    /// Any `i32` except 0 (a valid non-terminating `wchar_t`).
    pub fn nonzero_i32(&mut self) -> i32 {
        loop {
            let v = self.i32();
            if v != 0 {
                return v;
            }
        }
    }
    /// Non-zero ASCII, the "ordinary text" domain.
    pub fn ascii(&mut self) -> i32 {
        self.range(1, 127) as i32
    }
}

/// The element-value domains the C code can be fed (Axis 5 in CONFIGS.md).
#[derive(Copy, Clone, Debug)]
pub enum Domain {
    Ascii,
    /// Values above the BMP -- would be truncated by a 16-bit `wchar_t`.
    AboveBmp,
    /// Negative / high-bit-set -- would compare wrongly if `wchar_t` were unsigned.
    Negative,
    /// The whole `i32` domain except 0.
    AnyNonZero,
}

impl Domain {
    pub fn gen(self, rng: &mut Rng) -> WcharT {
        match self {
            Domain::Ascii => rng.ascii(),
            Domain::AboveBmp => match rng.range(0, 3) {
                0 => 0x1_0000,
                1 => 0x10_FFFF,
                2 => rng.range(0x1_0000, 0x10_FFFF) as i32,
                _ => 0xFFFF,
            },
            Domain::Negative => match rng.range(0, 3) {
                0 => -1,
                1 => i32::MIN,
                2 => i32::MAX,
                _ => -(rng.range(1, 1 << 30) as i32),
            },
            Domain::AnyNonZero => rng.nonzero_i32(),
        }
    }

    pub fn vec(self, rng: &mut Rng, len: usize) -> Vec<WcharT> {
        (0..len).map(|_| self.gen(rng)).collect()
    }

    pub fn all() -> [Domain; 4] {
        [
            Domain::Ascii,
            Domain::AboveBmp,
            Domain::Negative,
            Domain::AnyNonZero,
        ]
    }
}

/// Build a destination buffer of `capacity` elements whose first NUL is at
/// index `nul_at` (or which contains no NUL at all when `nul_at >= capacity`),
/// with a non-zero garbage tail after the NUL when `garbage_tail` is set.
pub fn make_dst(
    rng: &mut Rng,
    capacity: usize,
    nul_at: usize,
    domain: Domain,
    garbage_tail: bool,
) -> Vec<WcharT> {
    let mut v = Vec::with_capacity(capacity);
    for i in 0..capacity {
        if i == nul_at {
            v.push(0);
        } else if i < nul_at {
            v.push(domain.gen(rng));
        } else if garbage_tail {
            v.push(domain.gen(rng));
        } else {
            v.push(0);
        }
    }
    v
}
