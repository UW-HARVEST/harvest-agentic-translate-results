//! Shared differential-test harness.
//!
//! Loads BOTH shared libraries through `libloading` and exposes a uniform
//! `Lib` handle. Rust functions are NEVER called directly — always through the
//! `.so` exports, so the `#[no_mangle]`/`extern "C"` wrappers are under test
//! too.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::os::raw::c_int;
use std::path::PathBuf;

/// Mirror of the C `DynamicArray`. Only used to *read back* fields of objects
/// allocated by whichever library is under test.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DynamicArray {
    pub data: *mut c_int,
    pub size: usize,
    pub capacity: usize,
}

/// Field snapshot that is comparable across libraries (the raw `data` pointer
/// value itself obviously differs, so we only record whether it is null).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArrView {
    pub data_null: bool,
    pub size: usize,
    pub capacity: usize,
}

pub type InitArrayFn = unsafe extern "C" fn(usize) -> *mut DynamicArray;
pub type ExpandArrayFn = unsafe extern "C" fn(*mut DynamicArray) -> c_int;
pub type AddElementFn = unsafe extern "C" fn(*mut DynamicArray, c_int) -> c_int;
pub type FreeArrayFn = unsafe extern "C" fn(*mut DynamicArray);
pub type ProcessFlagsFn = unsafe extern "C" fn(c_int) -> c_int;
pub type ChecksumFn = unsafe extern "C" fn() -> c_int;
pub type MatrixSumFn = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

pub struct Lib {
    pub name: &'static str,
    _lib: Library,
    pub init_array: InitArrayFn,
    pub expand_array: ExpandArrayFn,
    pub add_element: AddElementFn,
    pub free_array: FreeArrayFn,
    pub process_flags: ProcessFlagsFn,
    pub calculate_matrix_checksum: ChecksumFn,
    pub matrixsum: MatrixSumFn,
    /// The exported writable `int matrix[3][4]` data symbol.
    pub matrix: *mut c_int,
}

impl Lib {
    unsafe fn open(name: &'static str, path: &PathBuf) -> Lib {
        let lib = Library::new(path)
            .unwrap_or_else(|e| panic!("failed to dlopen {} ({}): {e}", name, path.display()));

        macro_rules! sym {
            ($t:ty, $n:literal) => {{
                let s: Symbol<$t> = lib
                    .get($n)
                    .unwrap_or_else(|e| panic!("{} missing symbol {:?}: {e}", name, $n));
                *s
            }};
        }

        let init_array = sym!(InitArrayFn, b"init_array\0");
        let expand_array = sym!(ExpandArrayFn, b"expand_array\0");
        let add_element = sym!(AddElementFn, b"add_element\0");
        let free_array = sym!(FreeArrayFn, b"free_array\0");
        let process_flags = sym!(ProcessFlagsFn, b"process_flags\0");
        let calculate_matrix_checksum = sym!(ChecksumFn, b"calculate_matrix_checksum\0");
        let matrixsum = sym!(MatrixSumFn, b"matrixsum\0");

        let matrix: *mut c_int = {
            // libloading requires the requested type to be pointer-sized, so
            // we ask for `*mut c_int` and then take the RAW symbol address
            // (which is the address of the `matrix` object itself) rather than
            // dereferencing it.
            let s: Symbol<*mut c_int> = lib
                .get(b"matrix\0")
                .unwrap_or_else(|e| panic!("{name} missing data symbol matrix: {e}"));
            s.into_raw().into_raw() as *mut c_int
        };

        Lib {
            name,
            _lib: lib,
            init_array,
            expand_array,
            add_element,
            free_array,
            process_flags,
            calculate_matrix_checksum,
            matrixsum,
            matrix,
        }
    }

    /// Read the 12 ints of this library's `matrix` global.
    pub fn read_matrix(&self) -> [c_int; 12] {
        let mut out = [0; 12];
        for (i, o) in out.iter_mut().enumerate() {
            *o = unsafe { *self.matrix.add(i) };
        }
        out
    }

    /// Overwrite the 12 ints of this library's `matrix` global.
    pub fn write_matrix(&self, vals: &[c_int; 12]) {
        for (i, v) in vals.iter().enumerate() {
            unsafe { *self.matrix.add(i) = *v };
        }
    }

    pub fn view(&self, arr: *mut DynamicArray) -> Option<ArrView> {
        if arr.is_null() {
            return None;
        }
        unsafe {
            Some(ArrView {
                data_null: (*arr).data.is_null(),
                size: (*arr).size,
                capacity: (*arr).capacity,
            })
        }
    }

    /// Read `n` elements out of the array's backing buffer.
    pub fn elements(&self, arr: *mut DynamicArray, n: usize) -> Vec<c_int> {
        assert!(!arr.is_null());
        unsafe {
            let d = (*arr).data;
            assert!(!d.is_null() || n == 0);
            (0..n).map(|i| *d.add(i)).collect()
        }
    }
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("manifest dir has a parent")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    let build = workspace_root().join("c_src").join("build");
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| {
            panic!(
                "cannot read {} ({e}). Build the C library first:\n  cd c_src && mkdir -p build \
                 && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
                build.display()
            )
        })
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    candidates.sort();
    candidates
        .pop()
        .unwrap_or_else(|| panic!("no .so found in {}", build.display()))
}

fn find_rust_so() -> PathBuf {
    // The integration test binary lives in target/<profile>/deps/, so walk up
    // to the profile dir and look for the cdylib next to it.
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("target/<profile>")
        .to_path_buf();
    let candidates = [
        profile_dir.join("libmatrixsum_lib.so"),
        workspace_root()
            .join("translation/target/release/libmatrixsum_lib.so"),
        workspace_root().join("translation/target/debug/libmatrixsum_lib.so"),
    ];
    for c in candidates.iter() {
        if c.exists() {
            return c.clone();
        }
    }
    panic!(
        "Rust cdylib not found. Looked in: {:?}. Run `cargo build` first.",
        candidates
    );
}

pub struct Pair {
    pub c: Lib,
    pub rs: Lib,
}

// `Lib` holds raw pointers (the `matrix` data symbol) and plain function
// pointers. Sharing them across threads is sound here because every test that
// touches mutable library state takes `lock()` first, which serialises access.
unsafe impl Send for Lib {}
unsafe impl Sync for Lib {}
unsafe impl Send for Pair {}
unsafe impl Sync for Pair {}

/// Global serialisation lock. The libraries own mutable process-wide state
/// (`matrix`) and a shared heap, so all differential tests run one at a time.
pub fn lock() -> std::sync::MutexGuard<'static, ()> {
    use std::sync::{Mutex, OnceLock};
    static M: OnceLock<Mutex<()>> = OnceLock::new();
    match M.get_or_init(|| Mutex::new(())).lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    }
}

/// Factory contents of `matrix` as written in the C source.
pub const FACTORY_MATRIX: [c_int; 12] = [
    0x01, 0x02, 0x03, 0x04, 0x10, 0x20, 0x30, 0x40, 0xA1, 0xB2, 0xC3, 0xD4,
];

/// Restore both libraries' `matrix` globals to the factory values.
pub fn reset_matrix(p: &Pair) {
    p.c.write_matrix(&FACTORY_MATRIX);
    p.rs.write_matrix(&FACTORY_MATRIX);
}

/// Open both libraries. Cached per-process so every test shares one handle and
/// so `matrix` mutations are visible in the order the tests perform them.
pub fn libs() -> &'static Pair {
    use std::sync::OnceLock;
    static PAIR: OnceLock<Pair> = OnceLock::new();
    PAIR.get_or_init(|| unsafe {
        Pair {
            c: Lib::open("C", &find_c_so()),
            rs: Lib::open("RUST", &find_rust_so()),
        }
    })
}

/// Deterministic xorshift64* PRNG — fixed seed, reproducible across runs.
pub struct Rng(u64);

pub const SEED: u64 = 0x5EED_1234_ABCD_F00D;

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(if seed == 0 { 1 } else { seed })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn next_i32(&mut self) -> c_int {
        (self.next_u64() >> 32) as u32 as i32
    }
    /// Random i32 biased toward interesting values (extremes, small, zero).
    pub fn spicy_i32(&mut self) -> c_int {
        match self.next_u64() % 8 {
            0 => 0,
            1 => 1,
            2 => -1,
            3 => i32::MAX,
            4 => i32::MIN,
            5 => (self.next_u64() % 256) as i32,
            6 => -((self.next_u64() % 256) as i32),
            _ => self.next_i32(),
        }
    }
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
}
