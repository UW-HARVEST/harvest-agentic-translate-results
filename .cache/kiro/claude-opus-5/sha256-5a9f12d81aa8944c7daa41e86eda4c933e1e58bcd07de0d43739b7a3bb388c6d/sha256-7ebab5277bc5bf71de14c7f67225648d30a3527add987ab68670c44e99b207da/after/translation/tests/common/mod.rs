// Shared harness: loads BOTH the C .so and the Rust .so via libloading and
// exposes a uniform way to call each exported symbol.
//
// The libraries carry mutable file-scope state (`accumulator`, `multiplier`,
// `operation_count`).  dlopen() de-duplicates by file path, so to get a *fresh*
// copy of that state for each scenario we copy each .so to a unique temporary
// path before loading it.  That gives every `Pair` an independent state pair.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

pub type Fn2 = unsafe extern "C" fn(i32, i32) -> i32;
pub type Fn1 = unsafe extern "C" fn(i32) -> i32;
pub type Fn4 = unsafe extern "C" fn(i32, i32, i32, i32) -> i32;
pub type FnStr = unsafe extern "C" fn(*mut i8, i32);

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("manifest dir has a parent")
        .to_path_buf()
}

/// Directory holding the freshly built test artifacts (target/<profile>).
fn target_profile_dir() -> PathBuf {
    // current_exe = target/<profile>/deps/<test binary>
    let exe = std::env::current_exe().expect("current_exe");
    exe.parent()
        .and_then(|p| p.parent())
        .expect("target/<profile>")
        .to_path_buf()
}

pub fn c_so_path() -> PathBuf {
    let build_dir = repo_root().join("c_src").join("build");
    let mut found: Option<PathBuf> = None;
    let entries = std::fs::read_dir(&build_dir).unwrap_or_else(|e| {
        panic!(
            "cannot read {} ({e}); build the C library first:\n  cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build_dir.display()
        )
    });
    for e in entries.flatten() {
        let p = e.path();
        let name = p.file_name().unwrap_or_default().to_string_lossy().to_string();
        if name.starts_with("lib") && name.ends_with(".so") {
            found = Some(p);
        }
    }
    found.unwrap_or_else(|| panic!("no lib*.so in {}", build_dir.display()))
}

pub fn rust_so_path() -> PathBuf {
    let dir = target_profile_dir();
    let p = dir.join("libfindrep_lib.so");
    if p.exists() {
        return p;
    }
    // Fall back to whichever profile dir has it.
    for profile in ["debug", "release"] {
        let alt = repo_root()
            .join("translation")
            .join("target")
            .join(profile)
            .join("libfindrep_lib.so");
        if alt.exists() {
            return alt;
        }
    }
    panic!(
        "libfindrep_lib.so not found (looked in {}); run `cargo build` first",
        dir.display()
    )
}

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn unique_copy(src: &Path, tag: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("findrep_parity_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let dst = dir.join(format!("{tag}_{n}.so"));
    std::fs::copy(src, &dst).unwrap_or_else(|e| panic!("copy {} -> {}: {e}", src.display(), dst.display()));
    dst
}

/// A freshly-loaded (C, Rust) library pair with independent static state.
pub struct Pair {
    pub c: Library,
    pub rust: Library,
    _paths: (PathBuf, PathBuf),
}

impl Pair {
    pub fn fresh() -> Pair {
        let cp = unique_copy(&c_so_path(), "c");
        let rp = unique_copy(&rust_so_path(), "rust");
        unsafe {
            let c = Library::new(&cp).unwrap_or_else(|e| panic!("dlopen C {}: {e}", cp.display()));
            let rust =
                Library::new(&rp).unwrap_or_else(|e| panic!("dlopen Rust {}: {e}", rp.display()));
            Pair {
                c,
                rust,
                _paths: (cp, rp),
            }
        }
    }

    pub fn c_fn2(&self, name: &str) -> Symbol<'_, Fn2> {
        unsafe { self.c.get(name.as_bytes()).unwrap_or_else(|e| panic!("C {name}: {e}")) }
    }
    pub fn r_fn2(&self, name: &str) -> Symbol<'_, Fn2> {
        unsafe { self.rust.get(name.as_bytes()).unwrap_or_else(|e| panic!("Rust {name}: {e}")) }
    }
    pub fn c_fn1(&self, name: &str) -> Symbol<'_, Fn1> {
        unsafe { self.c.get(name.as_bytes()).unwrap_or_else(|e| panic!("C {name}: {e}")) }
    }
    pub fn r_fn1(&self, name: &str) -> Symbol<'_, Fn1> {
        unsafe { self.rust.get(name.as_bytes()).unwrap_or_else(|e| panic!("Rust {name}: {e}")) }
    }
    pub fn c_fn4(&self, name: &str) -> Symbol<'_, Fn4> {
        unsafe { self.c.get(name.as_bytes()).unwrap_or_else(|e| panic!("C {name}: {e}")) }
    }
    pub fn r_fn4(&self, name: &str) -> Symbol<'_, Fn4> {
        unsafe { self.rust.get(name.as_bytes()).unwrap_or_else(|e| panic!("Rust {name}: {e}")) }
    }
    pub fn c_fnstr(&self, name: &str) -> Symbol<'_, FnStr> {
        unsafe { self.c.get(name.as_bytes()).unwrap_or_else(|e| panic!("C {name}: {e}")) }
    }
    pub fn r_fnstr(&self, name: &str) -> Symbol<'_, FnStr> {
        unsafe { self.rust.get(name.as_bytes()).unwrap_or_else(|e| panic!("Rust {name}: {e}")) }
    }
}

/// Fixed-size byte buffer used as a `char[N]` destination, pre-filled with a
/// sentinel so we can detect any difference in how far each side writes.
pub const BUF: usize = 256;
pub const SENTINEL: u8 = 0xAB;

pub fn new_buf() -> [u8; BUF] {
    [SENTINEL; BUF]
}

pub fn new_buf_with(s: &[u8]) -> [u8; BUF] {
    let mut b = [SENTINEL; BUF];
    assert!(s.len() < BUF);
    b[..s.len()].copy_from_slice(s);
    b[s.len()] = 0;
    b
}

pub fn describe(b: &[u8; BUF]) -> String {
    let nul = b.iter().position(|&c| c == 0).unwrap_or(BUF);
    format!("{:?} (+{} trailing bytes)", String::from_utf8_lossy(&b[..nul]), BUF - nul)
}
