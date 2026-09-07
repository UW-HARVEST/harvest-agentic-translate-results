// Shared harness: loads BOTH the C `.so` and the Rust `.so` with `libloading`
// and exposes their exports through identical function-pointer tables, so every
// assertion in the test-suite is a true differential FFI comparison. No Rust
// function is ever called directly.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::os::raw::c_int;
use std::path::PathBuf;

pub type SizeT = usize;

/// Mirror of the C `DynamicArray` (`int *data; size_t size; size_t capacity;`).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DynamicArray {
    pub data: *mut c_int,
    pub size: SizeT,
    pub capacity: SizeT,
}

impl DynamicArray {
    pub fn stack(capacity: SizeT) -> Self {
        DynamicArray { data: std::ptr::null_mut(), size: 0, capacity }
    }
}

/// A field-by-field snapshot of a `DynamicArray` that is comparable across the
/// two libraries (the `data` pointer value itself obviously differs, so only its
/// null-ness is compared, alongside the buffer contents).
#[derive(Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub is_null: bool,
    pub data_is_null: bool,
    pub size: SizeT,
    pub capacity: SizeT,
    pub elements: Vec<c_int>,
}

pub struct Lib {
    pub name: &'static str,
    _lib: &'static Library,
    pub init_array: unsafe extern "C" fn(SizeT) -> *mut DynamicArray,
    pub expand_array: unsafe extern "C" fn(*mut DynamicArray) -> c_int,
    pub add_element: unsafe extern "C" fn(*mut DynamicArray, c_int) -> c_int,
    pub free_array: unsafe extern "C" fn(*mut DynamicArray),
    pub process_flags: unsafe extern "C" fn(c_int) -> c_int,
    pub calculate_matrix_checksum: unsafe extern "C" fn() -> c_int,
    pub matrixsum: unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int,
    /// Address of the exported mutable `int matrix[3][4]` global.
    pub matrix: *mut [[c_int; 4]; 3],
}

impl Lib {
    fn open(name: &'static str, path: &PathBuf) -> Lib {
        let lib: &'static Library = Box::leak(Box::new(
            unsafe { Library::new(path) }
                .unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", path.display())),
        ));
        unsafe {
            macro_rules! f {
                ($sym:literal, $t:ty) => {{
                    let s: Symbol<$t> = lib
                        .get($sym)
                        .unwrap_or_else(|e| panic!("{name}: missing symbol {:?}: {e}", $sym));
                    *s
                }};
            }
            let matrix_sym: Symbol<*mut [[c_int; 4]; 3]> = lib
                .get(b"matrix\0")
                .unwrap_or_else(|e| panic!("{name}: missing data symbol `matrix`: {e}"));
            Lib {
                name,
                _lib: lib,
                init_array: f!(b"init_array\0", unsafe extern "C" fn(SizeT) -> *mut DynamicArray),
                expand_array: f!(
                    b"expand_array\0",
                    unsafe extern "C" fn(*mut DynamicArray) -> c_int
                ),
                add_element: f!(
                    b"add_element\0",
                    unsafe extern "C" fn(*mut DynamicArray, c_int) -> c_int
                ),
                free_array: f!(b"free_array\0", unsafe extern "C" fn(*mut DynamicArray)),
                process_flags: f!(b"process_flags\0", unsafe extern "C" fn(c_int) -> c_int),
                calculate_matrix_checksum: f!(
                    b"calculate_matrix_checksum\0",
                    unsafe extern "C" fn() -> c_int
                ),
                matrixsum: f!(
                    b"matrixsum\0",
                    unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int
                ),
                matrix: *matrix_sym,
            }
        }
    }

    /// Read `n` elements out of an array's buffer (empty if the buffer is null).
    pub unsafe fn read(&self, arr: *mut DynamicArray, n: SizeT) -> Vec<c_int> {
        if arr.is_null() || (*arr).data.is_null() {
            return Vec::new();
        }
        (0..n).map(|i| *(*arr).data.add(i)).collect()
    }

    pub unsafe fn snapshot(&self, arr: *mut DynamicArray) -> Snapshot {
        if arr.is_null() {
            return Snapshot {
                is_null: true,
                data_is_null: true,
                size: 0,
                capacity: 0,
                elements: Vec::new(),
            };
        }
        let size = (*arr).size;
        Snapshot {
            is_null: false,
            data_is_null: (*arr).data.is_null(),
            size,
            capacity: (*arr).capacity,
            // Only `size` elements are initialized by the C code, so reading
            // beyond that would compare uninitialized heap bytes.
            elements: self.read(arr, size),
        }
    }

    pub unsafe fn get_matrix(&self) -> [[c_int; 4]; 3] {
        std::ptr::read(self.matrix)
    }

    pub unsafe fn set_matrix(&self, m: [[c_int; 4]; 3]) {
        std::ptr::write(self.matrix, m);
    }
}

/// The two libraries under comparison, plus a direct handle on libc's `free`
/// so tests can release buffers they detach from a descriptor (both `.so`s use
/// the very same allocator, which is a prerequisite for parity).
pub struct Pair {
    pub c: Lib,
    pub rust: Lib,
    pub libc_free: unsafe extern "C" fn(*mut std::os::raw::c_void),
}

impl Pair {
    pub unsafe fn free_buf(&self, p: *mut c_int) {
        (self.libc_free)(p as *mut std::os::raw::c_void);
    }
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
}

fn find_c_so() -> PathBuf {
    let dir = workspace_root().join("c_src/build");
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| {
            panic!(
                "cannot read {} ({e}). Build the C library first:\n  \
                 cd c_src && mkdir -p build && cd build && \
                 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
                dir.display()
            )
        })
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.extension().map(|x| x == "so").unwrap_or(false)
                && p.file_name().unwrap().to_string_lossy().starts_with("lib")
        })
        .collect();
    candidates.sort();
    assert_eq!(
        candidates.len(),
        1,
        "expected exactly one C shared library in {}, found {:?}",
        dir.display(),
        candidates
    );
    candidates.pop().unwrap()
}

fn find_rust_so() -> PathBuf {
    // The integration test binary lives in target/<profile>/deps/, so the
    // cdylib built alongside it is one directory up.
    let exe = std::env::current_exe().expect("current_exe");
    let deps = exe.parent().expect("deps dir");
    let profile = deps.parent().expect("profile dir");
    for dir in [profile.to_path_buf(), deps.to_path_buf()] {
        let p = dir.join("libmatrixsum_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "libmatrixsum_lib.so not found near {}. Run `cargo build` before `cargo test`.",
        profile.display()
    );
}

pub fn libs() -> Pair {
    let libc: &'static Library = Box::leak(Box::new(
        unsafe { Library::new("libc.so.6") }.expect("dlopen libc.so.6"),
    ));
    let free_sym: Symbol<unsafe extern "C" fn(*mut std::os::raw::c_void)> =
        unsafe { libc.get(b"free\0") }.expect("libc `free`");
    Pair {
        c: Lib::open("C", &find_c_so()),
        rust: Lib::open("Rust", &find_rust_so()),
        libc_free: *free_sym,
    }
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64) — fixed seeds keep every property test
// byte-for-byte reproducible.
// ---------------------------------------------------------------------------
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
    pub fn i32(&mut self) -> c_int {
        self.next_u64() as u32 as i32
    }
    /// Uniform in `0..n`.
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
    /// Uniform in `lo..=hi`.
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> c_int {
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + self.below(span) as i64) as i32
    }
    /// A value biased towards interesting edge cases.
    pub fn spicy_i32(&mut self) -> c_int {
        match self.below(10) {
            0 => 0,
            1 => 1,
            2 => -1,
            3 => i32::MAX,
            4 => i32::MIN,
            5 => i32::MAX / 2,
            6 => i32::MIN / 2,
            7 => self.range_i32(-16, 16),
            _ => self.i32(),
        }
    }
}

/// `matrix` is a single mutable global inside each `.so`, shared by every test
/// thread in this process (and read by `calculate_matrix_checksum`, hence by
/// `matrixsum`). Any test that reads or writes it must hold this lock, or a
/// concurrent mutation from another test would look like a C/Rust divergence.
pub fn matrix_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    match LOCK.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/// Interesting `size_t` values around the multiply-overflow boundaries.
pub const SIZE_EDGE: &[SizeT] = &[
    0,
    1,
    2,
    3,
    4,
    7,
    8,
    64,
    1 << 16,
    1 << 20,
    1 << 50,
    usize::MAX / 4,
    usize::MAX / 4 + 1,
    usize::MAX / 4 + 2,
    usize::MAX / 2,
    usize::MAX / 2 + 1,
    usize::MAX - 1,
    usize::MAX,
];
