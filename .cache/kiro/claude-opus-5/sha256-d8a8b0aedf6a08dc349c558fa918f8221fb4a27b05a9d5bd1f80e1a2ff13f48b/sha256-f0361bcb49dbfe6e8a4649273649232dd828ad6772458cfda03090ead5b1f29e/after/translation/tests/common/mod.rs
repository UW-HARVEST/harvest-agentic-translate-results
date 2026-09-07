//! Shared plumbing for the C-vs-Rust differential tests.
//!
//! Both libraries are always reached through `libloading`, i.e. through their
//! exported `#[no_mangle]` / `extern "C"` symbols, exactly as an external
//! consumer would. Nothing in this module calls a Rust function directly.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_char;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// ABI types, transcribed from c_src/src/lib.c
// ---------------------------------------------------------------------------

/// ```c
/// typedef struct { int id; char name[32]; uint8_t flags; } DataBlock;
/// ```
#[repr(C)]
#[derive(Copy, Clone)]
pub struct DataBlock {
    pub id: i32,
    pub name: [c_char; 32],
    pub flags: u8,
}

/// ```c
/// typedef struct { int *data; size_t size; } MemoryBlock;
/// ```
#[repr(C)]
#[derive(Copy, Clone)]
pub struct MemoryBlock {
    pub data: *mut i32,
    pub size: usize,
}

pub type CreateBlockFn = unsafe extern "C" fn(i32, *const c_char, u8) -> DataBlock;
pub type AllocateBlockFn = unsafe extern "C" fn(usize, i32) -> *mut MemoryBlock;
pub type FreeBlockFn = unsafe extern "C" fn(*mut MemoryBlock);
pub type ComputeHashFn = unsafe extern "C" fn(*mut MemoryBlock, *mut MemoryBlock) -> i32;
pub type BetagammaFn = unsafe extern "C" fn(i32, i32, i32, i32) -> i32;

/// The five exported symbols of the library, resolved out of a `.so`.
pub struct Api {
    // Keep the library alive for as long as the function pointers are used.
    _lib: Library,
    pub create_block: CreateBlockFn,
    pub allocate_block: AllocateBlockFn,
    pub free_block: FreeBlockFn,
    pub compute_hash: ComputeHashFn,
    pub betagamma: BetagammaFn,
}

impl Api {
    pub fn load(path: &Path) -> Api {
        unsafe {
            let lib = Library::new(path)
                .unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", path.display()));
            macro_rules! sym {
                ($name:literal, $ty:ty) => {{
                    let s: Symbol<$ty> = lib.get(concat!($name, "\0").as_bytes()).unwrap_or_else(
                        |e| panic!("{} missing from {}: {e}", $name, path.display()),
                    );
                    *s.into_raw()
                }};
            }
            let create_block = sym!("create_block", CreateBlockFn);
            let allocate_block = sym!("allocate_block", AllocateBlockFn);
            let free_block = sym!("free_block", FreeBlockFn);
            let compute_hash = sym!("compute_hash", ComputeHashFn);
            let betagamma = sym!("betagamma", BetagammaFn);
            Api {
                _lib: lib,
                create_block,
                allocate_block,
                free_block,
                compute_hash,
                betagamma,
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Locating the two shared objects
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p
}

/// Path to the C `.so` produced by `c_src/CMakeLists.txt`. The CMake project
/// name is derived from the *parent directory's* name, so the file name is not
/// fixed; glob for it instead.
pub fn c_so_path() -> PathBuf {
    let build_dir = workspace_root().join("c_src/build");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&build_dir)
        .unwrap_or_else(|e| {
            panic!(
                "{} not readable ({e}); build the C library first:\n  cd c_src && mkdir -p build \
                 && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
                build_dir.display()
            )
        })
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with("lib") && n.ends_with(".so"))
                .unwrap_or(false)
        })
        .collect();
    found.sort();
    assert_eq!(
        found.len(),
        1,
        "expected exactly one lib*.so in {}, found {:?}",
        build_dir.display(),
        found
    );
    found.pop().unwrap()
}

/// Path to the Rust `cdylib`. Located relative to the test executable so it
/// works for both `debug` and `release` profiles.
pub fn rust_so_path() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    // <target>/<profile>/deps/<test-bin>
    let profile_dir = exe.parent().unwrap().parent().unwrap();
    let candidates = [
        profile_dir.join("libbetagamma_lib.so"),
        workspace_root().join("translation/target/release/libbetagamma_lib.so"),
        workspace_root().join("translation/target/debug/libbetagamma_lib.so"),
    ];
    for c in &candidates {
        if c.exists() {
            return c.clone();
        }
    }
    panic!(
        "libbetagamma_lib.so not found; looked in {:?}. Run `cargo build` first.",
        candidates
    );
}

pub fn load_both() -> (Api, Api) {
    (Api::load(&c_so_path()), Api::load(&rust_so_path()))
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) -- fixed seed, reproducible runs
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5DEE_CE66_D000_0001;

pub struct Rng(u64);

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
    pub fn next_i32(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }
    /// Uniform in `[0, n)`.
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
    /// A signed value biased towards "interesting" magnitudes, including the
    /// extremes where the C code's `int` arithmetic overflows.
    pub fn interesting_i32(&mut self) -> i32 {
        match self.below(8) {
            0 => 0,
            1 => i32::MAX,
            2 => i32::MIN,
            3 => -(self.below(64) as i32),
            4 => self.below(64) as i32,
            5 => i32::MAX - self.below(64) as i32,
            6 => i32::MIN + self.below(64) as i32,
            _ => self.next_i32(),
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Byte view of a `DataBlock` restricted to the fields C actually assigns.
///
/// `create_block` declares `DataBlock block;` uninitialised and writes only
/// `id`, the `strcpy`'d prefix of `name`, and `flags`. Everything else (the
/// tail of `name` past the NUL, and the 3 bytes of trailing padding) keeps
/// whatever was on the stack, so it is *not* part of the specified behaviour
/// and must not be compared.
pub fn defined_bytes(b: &DataBlock, name_len: usize) -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(&b.id.to_le_bytes());
    for i in 0..=name_len {
        v.push(b.name[i] as u8);
    }
    v.push(b.flags);
    v
}

/// Read back a `MemoryBlock` returned by `allocate_block` as `(size, elements)`.
pub unsafe fn read_block(mb: *mut MemoryBlock) -> Option<(usize, Vec<i32>)> {
    if mb.is_null() {
        return None;
    }
    let size = (*mb).size;
    let data = (*mb).data;
    assert!(!data.is_null(), "allocate_block returned non-null mb with null data");
    let mut v = Vec::with_capacity(size);
    for i in 0..size {
        v.push(*data.add(i));
    }
    Some((size, v))
}

/// Like [`read_block`] but only samples up to `max` indices, spread across the
/// array plus the first and last few. Needed because `allocate_block` can be
/// asked for hundreds of millions of elements, which the *test* cannot afford to
/// copy even though the library handles it fine.
pub unsafe fn read_block_sampled(mb: *mut MemoryBlock, max: usize) -> Option<(usize, Vec<(usize, i32)>)> {
    if mb.is_null() {
        return None;
    }
    let size = (*mb).size;
    let data = (*mb).data;
    assert!(!data.is_null(), "allocate_block returned non-null mb with null data");
    let mut idx: Vec<usize> = Vec::new();
    if size <= max {
        idx.extend(0..size);
    } else {
        let stride = size / max;
        let mut i = 0usize;
        while i < size && idx.len() < max {
            idx.push(i);
            i += stride.max(1);
        }
        for tail in [size - 1, size - 2, size - 3] {
            if !idx.contains(&tail) {
                idx.push(tail);
            }
        }
    }
    let v = idx.into_iter().map(|i| (i, *data.add(i))).collect();
    Some((size, v))
}
