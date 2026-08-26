//! Shared differential-test harness.
//!
//! Both implementations are loaded as shared objects through `libloading` and
//! called only through their exported `merge_sort` symbol, so the
//! `#[no_mangle] extern "C"` wrapper is part of what is under test. The Rust
//! functions are never called directly.

#![allow(dead_code)]

use std::path::{Path, PathBuf};

/// `sizeof(spritebatch_sprite_t)` as reported by gcc for `c_src/include/lib.h`.
pub const ELEM_SIZE: usize = 16;

/// Mirror of the C `spritebatch_sprite_t`.
///
/// ```c
/// typedef struct spritebatch_sprite_t {
///     unsigned long long texture_id;   // offset 0
///     int sort_bits;                   // offset 8
/// } spritebatch_sprite_t;              // size 16, align 8  => padding at 12..16
/// ```
///
/// The 4 tail padding bytes are modelled as an explicit `pad` field so that
/// tests can *control* and *compare* them. gcc -O0 compiles the whole-struct
/// assignment `b[k] = a[i]` as two 8-byte moves
/// (`mov 0x8(%rax),%rdx` / `mov %rdx,0x8(%rcx)`), i.e. it copies the padding
/// quadword too, so padding is observable and must match.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub struct Sprite {
    pub texture_id: u64,
    pub sort_bits: i32,
    pub pad: u32,
}

impl std::fmt::Debug for Sprite {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{{tex:{:#x} sb:{} pad:{:#x}}}",
            self.texture_id, self.sort_bits, self.pad
        )
    }
}

pub type MergeSortFn = unsafe extern "C" fn(*mut Sprite, *mut Sprite, i32);

/// A loaded implementation. The `Library` is kept alive alongside the symbol.
pub struct Imp {
    pub name: &'static str,
    pub path: PathBuf,
    pub merge_sort: MergeSortFn,
    _lib: libloading::Library,
}

impl Imp {
    /// Calls the library's exported `merge_sort` through the FFI boundary.
    ///
    /// # Safety
    /// Caller guarantees the pointers/size satisfy whatever contract the case
    /// under test intends (out-of-contract cases are run in child processes).
    pub unsafe fn call(&self, a: *mut Sprite, b: *mut Sprite, size: i32) {
        unsafe { (self.merge_sort)(a, b, size) }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Path to the C shared object built by `c_src/CMakeLists.txt`.
pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    let dir = manifest_dir().join("c_src/build");
    // CMake names the library after the *parent* directory of c_src.
    for name in ["libtranslated_rust.so", "libc_src.so"] {
        let cand = dir.join(name);
        if cand.exists() {
            return cand;
        }
    }
    // Fall back to whatever single .so is in there.
    if let Ok(rd) = std::fs::read_dir(&dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("so") {
                return p;
            }
        }
    }
    panic!(
        "C shared library not found in {}.\nBuild it with:\n  cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        dir.display()
    );
}

/// Path to the Rust `cdylib`.
///
/// Derived from `current_exe()` (`target/<profile>/deps/<test>-<hash>`) so it
/// works with a custom `CARGO_TARGET_DIR` and with any profile. Overridable
/// with `RUST_SO=` so the same suite can be pointed at the release artifact.
pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    const LIB: &str = "libmerge_sort_lib.so";
    let exe = std::env::current_exe().expect("current_exe");
    // .../target/<profile>/deps/<test-bin>
    if let Some(deps) = exe.parent() {
        let cand = deps.join(LIB);
        if cand.exists() {
            return cand;
        }
        if let Some(profile) = deps.parent() {
            let cand = profile.join(LIB);
            if cand.exists() {
                return cand;
            }
        }
    }
    for p in ["target/debug", "target/release"] {
        let cand = manifest_dir().join(p).join(LIB);
        if cand.exists() {
            return cand;
        }
    }
    panic!(
        "Rust cdylib {LIB} not found near {}.\n\
         `cargo test` does NOT build a cdylib-only lib target, so build it first:\n\
           cargo build            # then re-run cargo test\n\
         or point the suite at an artifact explicitly with RUST_SO=<path>.",
        exe.display()
    );
}

/// Guards against silently differential-testing a **stale** `.so`: if
/// `src/lib.rs` is newer than the shared object, the suite would be validating
/// an old build. Both sides are checked against their own sources.
fn assert_fresh(so: &Path, sources: &[&str]) {
    let so_mtime = match std::fs::metadata(so).and_then(|m| m.modified()) {
        Ok(t) => t,
        Err(_) => return,
    };
    for src in sources {
        let p = manifest_dir().join(src);
        if let Ok(t) = std::fs::metadata(&p).and_then(|m| m.modified()) {
            if t > so_mtime {
                panic!(
                    "STALE ARTIFACT: {} is newer than {}.\n\
                     Rebuild before testing (cargo build / cmake --build c_src/build).",
                    p.display(),
                    so.display()
                );
            }
        }
    }
}

fn load(name: &'static str, path: PathBuf) -> Imp {
    unsafe {
        let lib = libloading::Library::new(&path)
            .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", path.display()));
        let sym: libloading::Symbol<MergeSortFn> = lib
            .get(b"merge_sort\0")
            .unwrap_or_else(|e| panic!("dlsym merge_sort in {} failed: {e}", path.display()));
        let merge_sort = *sym;
        Imp {
            name,
            path,
            merge_sort,
            _lib: lib,
        }
    }
}

/// Loads the C implementation via `dlopen`.
pub fn load_c() -> Imp {
    let p = c_so_path();
    assert_fresh(&p, &["c_src/src/lib.c", "c_src/include/lib.h"]);
    load("C", p)
}

/// Loads the Rust implementation via `dlopen`.
pub fn load_rust() -> Imp {
    let p = rust_so_path();
    assert_fresh(&p, &["src/lib.rs"]);
    load("Rust", p)
}

/// Loads a specific `.so` by path (used by child processes).
pub fn load_path(name: &'static str, p: &Path) -> Imp {
    load(name, p.to_path_buf())
}

// ---------------------------------------------------------------------------
// Byte-level comparison helpers
// ---------------------------------------------------------------------------

/// Reinterprets a sprite slice as raw bytes, *including* the tail padding.
pub fn as_bytes(s: &[Sprite]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(s.as_ptr() as *const u8, s.len() * ELEM_SIZE) }
}

pub fn hex(s: &[Sprite]) -> String {
    as_bytes(s).iter().map(|b| format!("{b:02x}")).collect()
}

/// Asserts two buffers are byte-for-byte identical; on failure reports the
/// first differing byte plus surrounding element context.
pub fn assert_bytes_eq(what: &str, ctx: &str, c: &[Sprite], r: &[Sprite]) {
    assert_eq!(
        c.len(),
        r.len(),
        "{ctx}: {what} length mismatch (harness bug)"
    );
    let (cb, rb) = (as_bytes(c), as_bytes(r));
    if cb == rb {
        return;
    }
    let idx = cb
        .iter()
        .zip(rb)
        .position(|(x, y)| x != y)
        .expect("slices differ but no differing byte");
    let elem = idx / ELEM_SIZE;
    let lo = elem.saturating_sub(2);
    let hi = (elem + 3).min(c.len());
    panic!(
        "DIVERGENCE in `{what}` ({ctx})\n\
         first differing byte: offset {idx} (element {elem}, byte {} within element)\n\
         C   [{lo}..{hi}] = {:?}\n\
         Rust[{lo}..{hi}] = {:?}\n\
         C    bytes = {}\n\
         Rust bytes = {}",
        idx % ELEM_SIZE,
        &c[lo..hi],
        &r[lo..hi],
        hex(c),
        hex(r),
    );
}

// ---------------------------------------------------------------------------
// Deterministic RNG (splitmix64) — fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5EED_1234;

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
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    /// Uniform-ish in `0..n`.
    pub fn below(&mut self, n: u64) -> u64 {
        if n == 0 { 0 } else { self.next_u64() % n }
    }
}
