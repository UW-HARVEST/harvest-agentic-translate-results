//! Shared harness: loads both the C reference `.so` and the Rust `.so` via
//! `libloading` and exposes matching symbol wrappers for each.
//!
//! Nothing here calls into the Rust crate directly -- every Rust-side call goes
//! through `dlopen`/`dlsym` on the produced cdylib, so the `#[no_mangle]`
//! export wrappers are under test too.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_uchar};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// `typedef struct { int id; char name[32]; uint8_t flags; } DataBlock;`
#[repr(C)]
#[derive(Clone, Copy)]
pub struct DataBlock {
    pub id: c_int,
    pub name: [c_char; 32],
    pub flags: c_uchar,
}

/// `typedef struct { int *data; size_t size; } MemoryBlock;`
#[repr(C)]
#[derive(Clone, Copy)]
pub struct MemoryBlock {
    pub data: *mut c_int,
    pub size: usize,
}

pub type CreateBlockFn = unsafe extern "C" fn(c_int, *const c_char, c_uchar) -> DataBlock;
pub type AllocateBlockFn = unsafe extern "C" fn(usize, c_int) -> *mut MemoryBlock;
pub type FreeBlockFn = unsafe extern "C" fn(*mut MemoryBlock);
pub type ComputeHashFn = unsafe extern "C" fn(*mut MemoryBlock, *mut MemoryBlock) -> c_int;
pub type BetagammaFn = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

/// One loaded implementation (either the C reference or the Rust translation).
pub struct Impl {
    pub name: &'static str,
    lib: Library,
}

impl Impl {
    fn open(name: &'static str, path: &Path) -> Impl {
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("failed to dlopen {} ({}): {e}", name, path.display()));
        Impl { name, lib }
    }

    fn sym<T>(&self, name: &[u8]) -> Symbol<'_, T> {
        unsafe { self.lib.get(name) }.unwrap_or_else(|e| {
            panic!(
                "{} does not export `{}`: {e}",
                self.name,
                String::from_utf8_lossy(name)
            )
        })
    }

    pub fn create_block(&self) -> Symbol<'_, CreateBlockFn> {
        self.sym(b"create_block\0")
    }
    pub fn allocate_block(&self) -> Symbol<'_, AllocateBlockFn> {
        self.sym(b"allocate_block\0")
    }
    pub fn free_block(&self) -> Symbol<'_, FreeBlockFn> {
        self.sym(b"free_block\0")
    }
    pub fn compute_hash(&self) -> Symbol<'_, ComputeHashFn> {
        self.sym(b"compute_hash\0")
    }
    pub fn betagamma(&self) -> Symbol<'_, BetagammaFn> {
        self.sym(b"betagamma\0")
    }
}

pub struct Pair {
    pub c: Impl,
    pub rs: Impl,
}

static PAIR: OnceLock<Pair> = OnceLock::new();

/// Workspace root (the directory holding `c_src/` and `translation/`).
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    let build = root().join("c_src/build");
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&build) {
        for e in entries.flatten() {
            let p = e.path();
            let is_so = p.extension().map(|x| x == "so").unwrap_or(false);
            let is_lib = p
                .file_name()
                .and_then(|f| f.to_str())
                .map(|f| f.starts_with("lib"))
                .unwrap_or(false);
            if is_so && is_lib {
                found.push(p);
            }
        }
    }
    found.sort();
    found.pop().unwrap_or_else(|| {
        panic!(
            "no C shared library found in {}; build it with:\n  \
             cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

/// Locate the Rust cdylib.
///
/// `cargo test` does not emit the `cdylib` artifact for a cdylib-only crate, so
/// if the library is not already present it is built on demand into a private
/// target directory (which keeps it clear of the lock held by the outer cargo
/// invocation). The active feature set is forwarded, so the loaded `.so` always
/// matches the feature combination under test.
fn find_rust_so() -> PathBuf {
    const LIB: &str = "libbetagamma_lib.so";

    // The test executable lives in `target/<profile>/deps/`.
    let exe = std::env::current_exe().expect("current_exe");
    let deps = exe.parent().expect("deps dir").to_path_buf();
    let profile_dir = if deps.file_name().and_then(|f| f.to_str()) == Some("deps") {
        deps.parent().expect("profile dir").to_path_buf()
    } else {
        deps.clone()
    };

    for cand in [profile_dir.join(LIB), deps.join(LIB)] {
        if cand.exists() {
            return cand;
        }
    }

    let release = profile_dir.file_name().and_then(|f| f.to_str()) == Some("release");
    let out_root = profile_dir.join("it-cdylib");
    let built = out_root
        .join(if release { "release" } else { "debug" })
        .join(LIB);

    let manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let mut cmd = std::process::Command::new(cargo);
    cmd.arg("build")
        .arg("--lib")
        .arg("--manifest-path")
        .arg(&manifest)
        .arg("--target-dir")
        .arg(&out_root)
        .arg("--no-default-features");
    if release {
        cmd.arg("--release");
    }
    let feats = active_features();
    if !feats.is_empty() {
        cmd.arg("--features").arg(feats.join(","));
    }

    let status = cmd
        .status()
        .unwrap_or_else(|e| panic!("failed to spawn cargo to build the cdylib: {e}"));
    assert!(
        status.success() && built.exists(),
        "could not build the Rust cdylib (expected {})",
        built.display()
    );
    built
}

/// The features enabled for this test build, recovered from the `CARGO_FEATURE_*`
/// environment variables cargo sets at compile time.
///
/// The crate declares no `[features]`, so this is normally empty; it exists so
/// the on-demand cdylib build tracks whatever feature combination the tests are
/// run under.
fn active_features() -> Vec<String> {
    let mut out = Vec::new();
    // Enumerated at compile time via `option_env!` would require knowing the
    // names up front; instead read the runtime environment, which cargo also
    // propagates to the test process for the package's own features.
    for (k, v) in std::env::vars() {
        if let Some(rest) = k.strip_prefix("CARGO_FEATURE_") {
            if v == "1" {
                out.push(rest.to_ascii_lowercase().replace('_', "-"));
            }
        }
    }
    out.sort();
    out
}

/// Load (once per test binary) both implementations.
pub fn pair() -> &'static Pair {
    PAIR.get_or_init(|| Pair {
        c: Impl::open("C .so", &find_c_so()),
        rs: Impl::open("Rust .so", &find_rust_so()),
    })
}

/// Paths of the two shared objects under comparison, as `(c, rust)`.
pub fn library_paths() -> (PathBuf, PathBuf) {
    (find_c_so(), find_rust_so())
}

/// Bytes of a `name` array up to (excluding) the first NUL.
///
/// The C `create_block` copies into an *uninitialised* stack struct, so bytes
/// past the terminator are indeterminate and deliberately not compared.
pub fn name_str(b: &DataBlock) -> Vec<u8> {
    let mut out = Vec::new();
    for &c in b.name.iter() {
        if c == 0 {
            break;
        }
        out.push(c as u8);
    }
    out
}

/// Raw byte image of a `DataBlock`, for byte-for-byte comparison when every
/// byte is known to be defined.
pub fn raw_bytes(b: &DataBlock) -> Vec<u8> {
    let p = b as *const DataBlock as *const u8;
    unsafe { std::slice::from_raw_parts(p, std::mem::size_of::<DataBlock>()) }.to_vec()
}
