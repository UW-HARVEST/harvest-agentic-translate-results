//! Shared differential-test harness.
//!
//! Loads BOTH shared objects through `libloading` and calls only their exported
//! symbols. The Rust implementation is never called directly as a Rust
//! function — always through `dlopen`/`dlsym` on `libtritanopia_lib.so`, exactly
//! as an external C consumer would, so the `#[no_mangle] extern "C"` wrapper and
//! its ABI are part of what is under test.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// `typedef struct cb_rgb_255 { unsigned char R, G, B; }`
#[repr(C)]
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub struct Rgb255 {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb255 {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}

pub type TritFn = unsafe extern "C" fn(Rgb255) -> Rgb255;
/// Same entry point viewed as its raw register ABI: on x86-64 SysV a 3-byte
/// aggregate is class INTEGER, so it travels in one general-purpose register
/// (`RDI`) and returns in `RAX`. Used to probe garbage in the unused bytes.
pub type TritRawFn = unsafe extern "C" fn(u64) -> u64;

pub struct Libs {
    // Keep the handles alive for the process lifetime; the fn pointers below
    // borrow from the mapped code.
    _c_lib: Library,
    _r_lib: Library,
    pub c: TritFn,
    pub r: TritFn,
    pub c_raw: TritRawFn,
    pub r_raw: TritRawFn,
    pub c_path: PathBuf,
    pub r_path: PathBuf,
}

// `Library` is Send+Sync and bare `fn` pointers are Send+Sync.
unsafe impl Send for Libs {}
unsafe impl Sync for Libs {}

static LIBS: OnceLock<Libs> = OnceLock::new();

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Locate the C `.so`. The CMake project name is derived from the *parent*
/// directory name (`cmake_path(GET parent FILENAME project_name)`), so the file
/// name is environment-dependent — glob instead of hardcoding.
fn find_c_so() -> PathBuf {
    let build_dir = manifest_dir().join("../c_src/build");
    let mut hits: Vec<PathBuf> = std::fs::read_dir(&build_dir)
        .unwrap_or_else(|e| {
            panic!(
                "cannot read {}: {e}\nBuild the C library first:\n  \
                 cd c_src && mkdir -p build && cd build && \
                 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
                build_dir.display()
            )
        })
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension().and_then(|s| s.to_str()) == Some("so")
                && p.file_name()
                    .and_then(|s| s.to_str())
                    .is_some_and(|n| n.starts_with("lib"))
        })
        .collect();
    hits.sort();
    assert_eq!(
        hits.len(),
        1,
        "expected exactly one lib*.so in {}, found {:?}",
        build_dir.display(),
        hits
    );
    hits.pop().unwrap()
}

/// Locate the Rust `cdylib` for the *same* profile as this test binary.
///
/// The test executable lives at `target/<profile>/deps/<name>-<hash>`, and the
/// cdylib is emitted to `target/<profile>/`, so we walk up from `current_exe`.
/// This keeps `cargo test` and `cargo test --release` each testing their own
/// artifact (they differ: `panic = "abort"` and optimisation apply only to
/// `release`).
fn find_rust_so() -> PathBuf {
    const NAME: &str = "libtritanopia_lib.so";
    let exe = std::env::current_exe().expect("current_exe");
    let mut dir: Option<&Path> = exe.parent();
    while let Some(d) = dir {
        let cand = d.join(NAME);
        if cand.is_file() {
            return cand;
        }
        // stop once we climb past `target/`
        if d.file_name().and_then(|s| s.to_str()) == Some("target") {
            break;
        }
        dir = d.parent();
    }
    // Fallbacks for unusual layouts.
    for p in ["target/release", "target/debug"] {
        let cand = manifest_dir().join(p).join(NAME);
        if cand.is_file() {
            return cand;
        }
    }
    panic!(
        "could not locate {NAME} near {} — run `cargo build` first",
        exe.display()
    );
}

/// Guard against testing a STALE artifact.
///
/// `cargo test --test <name>` does **not** rebuild the `cdylib`: an integration
/// test has no Rust-level dependency on a `crate-type = ["cdylib"]` library (the
/// tests reach it only via `dlopen`), so Cargo happily runs against an old
/// `.so`. That silently turns a real divergence into a green run — it was
/// observed doing exactly that during this verification.
///
/// So: refuse to run if the loaded library is older than any of its sources.
fn assert_fresh(lib: &Path, sources: &[PathBuf], what: &str, howto: &str) {
    let lib_mtime = std::fs::metadata(lib)
        .and_then(|m| m.modified())
        .unwrap_or_else(|e| panic!("stat {}: {e}", lib.display()));
    for s in sources {
        let Ok(src_mtime) = std::fs::metadata(s).and_then(|m| m.modified()) else {
            continue;
        };
        assert!(
            lib_mtime >= src_mtime,
            "STALE {what}: {}\n  is older than its source {}\n  \
             Tests would have run against outdated code. Rebuild with:\n    {howto}",
            lib.display(),
            s.display()
        );
    }
}

/// Load both libraries once per test process.
pub fn libs() -> &'static Libs {
    LIBS.get_or_init(|| {
        let c_path = find_c_so();
        let r_path = find_rust_so();

        let md = manifest_dir();
        assert_fresh(
            &r_path,
            &[md.join("src/lib.rs"), md.join("Cargo.toml")],
            "Rust cdylib",
            "cargo build --release   (or `cargo build` for the dev profile)",
        );
        assert_fresh(
            &c_path,
            &[
                md.join("../c_src/src/lib.c"),
                md.join("../c_src/include/lib.h"),
            ],
            "C shared library",
            "cd c_src/build && cmake --build .",
        );

        // RTLD_LOCAL (libloading's default) keeps the two identically-named
        // `tritanopia` symbols from shadowing one another; each is resolved
        // through its own handle.
        let c_lib = unsafe { Library::new(&c_path) }
            .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", c_path.display()));
        let r_lib = unsafe { Library::new(&r_path) }
            .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", r_path.display()));

        let (c, c_raw) = unsafe {
            let s: Symbol<TritFn> = c_lib
                .get(b"tritanopia\0")
                .expect("C .so does not export `tritanopia`");
            let sr: Symbol<TritRawFn> = c_lib.get(b"tritanopia\0").unwrap();
            (*s, *sr)
        };
        let (r, r_raw) = unsafe {
            let s: Symbol<TritFn> = r_lib
                .get(b"tritanopia\0")
                .expect("Rust .so does not export `tritanopia`");
            let sr: Symbol<TritRawFn> = r_lib.get(b"tritanopia\0").unwrap();
            (*s, *sr)
        };

        Libs {
            _c_lib: c_lib,
            _r_lib: r_lib,
            c,
            r,
            c_raw,
            r_raw,
            c_path,
            r_path,
        }
    })
}

/// Call C through its `.so`.
pub fn call_c(x: Rgb255) -> Rgb255 {
    unsafe { (libs().c)(x) }
}

/// Call Rust through its `.so`.
pub fn call_r(x: Rgb255) -> Rgb255 {
    unsafe { (libs().r)(x) }
}

/// Assert C and Rust agree byte-for-byte on one input.
#[track_caller]
pub fn assert_same(x: Rgb255) {
    let c = call_c(x);
    let r = call_r(x);
    assert_eq!(
        c, r,
        "divergence for input ({},{},{}): C={:?} Rust={:?}",
        x.r, x.g, x.b, c, r
    );
}

/// Assert agreement over an iterator of inputs, reporting the first few
/// divergences together with a total count rather than dying on the first one.
#[track_caller]
pub fn assert_same_all<I: IntoIterator<Item = Rgb255>>(label: &str, inputs: I) -> usize {
    let mut n = 0usize;
    let mut bad = 0usize;
    let mut first: Vec<String> = Vec::new();
    for x in inputs {
        n += 1;
        let c = call_c(x);
        let r = call_r(x);
        if c != r {
            bad += 1;
            if first.len() < 10 {
                first.push(format!(
                    "  in=({:3},{:3},{:3})  C=({:3},{:3},{:3})  Rust=({:3},{:3},{:3})",
                    x.r, x.g, x.b, c.r, c.g, c.b, r.r, r.g, r.b
                ));
            }
        }
    }
    assert!(
        bad == 0,
        "{label}: {bad} of {n} inputs diverge; first divergences:\n{}",
        first.join("\n")
    );
    n
}

/// xorshift64* — deterministic, seeded, no external dependency.
pub struct Rng(u64);

impl Rng {
    pub const fn new(seed: u64) -> Self {
        // avoid the zero fixed point
        Self(if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 33) as u8
    }
    pub fn next_rgb(&mut self) -> Rgb255 {
        let v = self.next_u64();
        // take well-separated bits for the three channels
        Rgb255::new((v >> 8) as u8, (v >> 24) as u8, (v >> 40) as u8)
    }
}

/// All 2^24 inputs, in a fixed order.
pub fn all_inputs() -> impl Iterator<Item = Rgb255> {
    (0u32..(1 << 24)).map(|i| {
        Rgb255::new(
            ((i >> 16) & 0xff) as u8,
            ((i >> 8) & 0xff) as u8,
            (i & 0xff) as u8,
        )
    })
}
