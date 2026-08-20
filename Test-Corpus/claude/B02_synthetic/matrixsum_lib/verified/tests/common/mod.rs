// Shared differential-test harness.
//
// Loads BOTH shared objects through `libloading` and exposes their exported
// symbols behind identical typed wrappers. The Rust implementation is NEVER
// called directly as a Rust crate -- it is always reached through
// `libmatrixsum_lib.so`'s `#[no_mangle]` exports, exactly as an external C
// caller would, so the export wrappers themselves are under test.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::path::PathBuf;

/// typedef struct { int *data; size_t size; size_t capacity; } DynamicArray;
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DynamicArray {
    pub data: *mut c_int,
    pub size: usize,
    pub capacity: usize,
}

pub type FnInitArray = unsafe extern "C" fn(usize) -> *mut DynamicArray;
pub type FnExpandArray = unsafe extern "C" fn(*mut DynamicArray) -> c_int;
pub type FnAddElement = unsafe extern "C" fn(*mut DynamicArray, c_int) -> c_int;
pub type FnFreeArray = unsafe extern "C" fn(*mut DynamicArray);
pub type FnProcessFlags = unsafe extern "C" fn(c_int) -> c_int;
pub type FnCalcChecksum = unsafe extern "C" fn() -> c_int;
pub type FnMatrixsum = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

/// A fully-resolved view of one implementation's exported ABI.
pub struct Api {
    pub name: &'static str,
    pub init_array: FnInitArray,
    pub expand_array: FnExpandArray,
    pub add_element: FnAddElement,
    pub free_array: FnFreeArray,
    pub process_flags: FnProcessFlags,
    pub calculate_matrix_checksum: FnCalcChecksum,
    pub matrixsum: FnMatrixsum,
    /// Address of the exported writable `int matrix[3][4]` data symbol
    /// (12 contiguous `int`s, row-major).
    pub matrix: *mut c_int,
}

pub const MATRIX_LEN: usize = 12;

/// Default contents of `int matrix[3][4]` as defined in c_src/src/lib.c.
pub const MATRIX_DEFAULT: [c_int; MATRIX_LEN] = [
    0x01, 0x02, 0x03, 0x04, 0x10, 0x20, 0x30, 0x40, 0xA1, 0xB2, 0xC3, 0xD4,
];

fn sym<T: Copy>(lib: &'static Library, name: &[u8]) -> T {
    unsafe {
        let s: Symbol<T> = lib
            .get(name)
            .unwrap_or_else(|e| panic!("symbol {:?} missing: {e}", String::from_utf8_lossy(name)));
        *s
    }
}

fn data_sym(lib: &'static Library, name: &[u8]) -> *mut c_int {
    unsafe {
        let s: Symbol<*mut c_int> = lib
            .get(name)
            .unwrap_or_else(|e| panic!("data symbol {:?} missing: {e}", String::from_utf8_lossy(name)));
        // `try_as_raw_ptr` yields the ADDRESS of the symbol itself, which is what
        // we want for a data symbol (dereferencing the Symbol would instead read
        // the first 8 bytes of `matrix` as if they were a pointer).
        s.try_as_raw_ptr()
            .expect("could not take raw address of data symbol") as *mut c_int
    }
}

impl Api {
    fn load(name: &'static str, path: PathBuf) -> Api {
        let lib = unsafe { Library::new(&path) }
            .unwrap_or_else(|e| panic!("failed to load {}: {e}", path.display()));
        // Leak so all resolved pointers are valid for the whole test run.
        let lib: &'static Library = Box::leak(Box::new(lib));
        Api {
            name,
            init_array: sym(lib, b"init_array\0"),
            expand_array: sym(lib, b"expand_array\0"),
            add_element: sym(lib, b"add_element\0"),
            free_array: sym(lib, b"free_array\0"),
            process_flags: sym(lib, b"process_flags\0"),
            calculate_matrix_checksum: sym(lib, b"calculate_matrix_checksum\0"),
            matrixsum: sym(lib, b"matrixsum\0"),
            matrix: data_sym(lib, b"matrix\0"),
        }
    }

    /// Overwrite the exported `matrix` data symbol.
    pub fn set_matrix(&self, values: &[c_int; MATRIX_LEN]) {
        unsafe { std::ptr::copy_nonoverlapping(values.as_ptr(), self.matrix, MATRIX_LEN) }
    }

    pub fn get_matrix(&self) -> [c_int; MATRIX_LEN] {
        let mut out = [0; MATRIX_LEN];
        unsafe { std::ptr::copy_nonoverlapping(self.matrix, out.as_mut_ptr(), MATRIX_LEN) }
        out
    }

    pub fn reset_matrix(&self) {
        self.set_matrix(&MATRIX_DEFAULT);
    }

    /// Read a handle's struct fields.
    pub fn read_handle(&self, h: *mut DynamicArray) -> DynamicArray {
        assert!(!h.is_null(), "[{}] null handle", self.name);
        unsafe { *h }
    }

    /// Read the INITIALIZED prefix (`[0, size)`) of a handle's buffer. Bytes past
    /// `size` come straight from `malloc` and are indeterminate, so they are
    /// deliberately not compared between implementations.
    pub fn read_elems(&self, h: *mut DynamicArray) -> Vec<c_int> {
        let s = self.read_handle(h);
        if s.size == 0 {
            return Vec::new();
        }
        assert!(!s.data.is_null(), "[{}] size>0 but data==NULL", self.name);
        unsafe { std::slice::from_raw_parts(s.data, s.size).to_vec() }
    }

    /// Struct fields + initialized contents, in a directly comparable form.
    /// The `data` pointer VALUE is intentionally excluded (heap addresses differ
    /// between the two libraries); only its null-ness is compared.
    pub fn snapshot(&self, h: *mut DynamicArray) -> Snapshot {
        let s = self.read_handle(h);
        Snapshot {
            data_is_null: s.data.is_null(),
            size: s.size,
            capacity: s.capacity,
            elems: self.read_elems(h),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    pub data_is_null: bool,
    pub size: usize,
    pub capacity: usize,
    pub elems: Vec<c_int>,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    let p = manifest_dir().join("c_src/build/libtranslated_rust.so");
    assert!(
        p.exists(),
        "C shared library not found at {}.\nBuild it with:\n  cd c_src && mkdir -p build && cd build \\\n    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

/// Locate the Rust cdylib next to the test executable (target/<profile>/).
///
/// The crate is `crate-type = ["cdylib"]` and no test links it as a Rust
/// library, so `cargo test` alone does NOT build the shared object. Build it on
/// demand (once per process) so that a bare `cargo test` is self-sufficient.
fn rust_so_path() -> PathBuf {
    static BUILD_ONCE: std::sync::Once = std::sync::Once::new();

    let exe = std::env::current_exe().expect("current_exe");
    // .../target/<profile>/deps/<test-bin>  ->  .../target/<profile>/
    let dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("test exe should live in target/<profile>/deps/");
    let p = dir.join("libmatrixsum_lib.so");

    if !p.exists() {
        BUILD_ONCE.call_once(|| {
            let profile_dir = dir.file_name().and_then(|s| s.to_str()).unwrap_or("debug");
            let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
            let mut cmd = std::process::Command::new(cargo);
            cmd.current_dir(manifest_dir())
                .arg("build")
                .arg("--offline")
                .arg("--no-default-features");
            if profile_dir == "release" {
                cmd.arg("--release");
            }
            // Enable whatever features this test binary was compiled with.
            let feats: Vec<&str> = FEATURE_LIST.iter().copied().collect();
            if !feats.is_empty() {
                cmd.arg("--features").arg(feats.join(","));
            }
            let status = cmd.status();
            eprintln!("[harness] built cdylib on demand ({profile_dir}): {status:?}");
        });
    }

    assert!(
        p.exists(),
        "Rust cdylib not found at {}. Build it with:\n  cargo build --no-default-features",
        p.display()
    );
    p
}

/// Features this test binary was compiled with, so the on-demand cdylib build
/// matches the configuration under test. `Cargo.toml` declares no `[features]`,
/// so this is empty; it is kept as the single place to extend if any are added.
const FEATURE_LIST: &[&str] = &[];

/// The pair under test. `c` is ground truth; `r` must match it exactly.
pub struct Pair {
    pub c: Api,
    pub r: Api,
}

pub fn load_pair() -> Pair {
    Pair {
        c: Api::load("C", c_so_path()),
        r: Api::load("RUST", rust_so_path()),
    }
}

impl Pair {
    /// Put both libraries' `matrix` symbol back to its initial state.
    pub fn reset_matrices(&self) {
        self.c.reset_matrix();
        self.r.reset_matrix();
    }
    pub fn set_matrices(&self, v: &[c_int; MATRIX_LEN]) {
        self.c.set_matrix(v);
        self.r.set_matrix(v);
    }
}

// ---------------------------------------------------------------------------
// `matrix` is a process-global data symbol inside each .so, and `dlopen` of the
// same path returns the same mapping, so all tests in one test binary share it.
// Any test that reads or writes `matrix` (which includes every `matrixsum` and
// `calculate_matrix_checksum` test) must hold this lock, otherwise parallel
// tests would race on it.
// ---------------------------------------------------------------------------
static MATRIX_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Run `f` with exclusive access to both libraries' `matrix` symbol, which is
/// reset to its default contents before and after.
pub fn with_matrix_lock<R>(pair: &Pair, f: impl FnOnce() -> R) -> R {
    let _guard = MATRIX_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    pair.reset_matrices();
    let out = f();
    pair.reset_matrices();
    out
}

// ---------------------------------------------------------------------------
// Deterministic RNG (splitmix64) -- fixed seed for reproducibility.
// ---------------------------------------------------------------------------
pub const SEED: u64 = 0x5EED_C0FF_EE12_34;

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
    /// Uniform over the full 32-bit range (so negatives and extremes appear).
    pub fn next_i32(&mut self) -> c_int {
        self.next_u64() as u32 as c_int
    }
    /// Small magnitude value, still signed.
    pub fn next_small(&mut self) -> c_int {
        (self.next_u64() % 2001) as i64 as c_int - 1000
    }
    /// Non-zero i32 (so it sets its permission flag).
    pub fn next_nonzero_i32(&mut self) -> c_int {
        loop {
            let v = self.next_i32();
            if v != 0 {
                return v;
            }
        }
    }
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
    /// Pick from a set of interesting boundary values, else a random one.
    pub fn interesting_i32(&mut self) -> c_int {
        const VALUES: [c_int; 12] = [
            0,
            1,
            -1,
            2,
            -2,
            c_int::MAX,
            c_int::MIN,
            c_int::MAX - 1,
            c_int::MIN + 1,
            0x0800_0000,
            0x7FFF_FFF0,
            -0x0800_0000,
        ];
        let k = self.below((VALUES.len() + 4) as u64) as usize;
        if k < VALUES.len() {
            VALUES[k]
        } else {
            self.next_i32()
        }
    }
}

/// Assert two results are identical, with a descriptive failure message.
#[macro_export]
macro_rules! assert_same {
    ($ctx:expr, $c:expr, $r:expr) => {{
        let cv = $c;
        let rv = $r;
        assert_eq!(
            cv, rv,
            "DIVERGENCE [{}]:\n  C    = {:?}\n  RUST = {:?}",
            $ctx, cv, rv
        );
    }};
}
