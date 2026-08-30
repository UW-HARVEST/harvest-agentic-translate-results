//! Shared harness for the C-vs-Rust differential tests.
//!
//! Both implementations are reached **only** through their shared objects,
//! loaded with `libloading`. Nothing in `tests/` calls a Rust function directly,
//! so the `#[no_mangle] extern "C"` export wrappers and the `.data` placement of
//! the exported globals are part of what is under test.
//!
//! * C side:    `cbuild/libcdriver_<op>_<repeat>.so`, produced by
//!              `build_c_so.sh` from the unmodified `c_src/src/mdcore.c` with
//!              the same `-DOP=/-DREPEAT=` flags `c_src/CMakeLists.txt` uses.
//! * Rust side: `target/<profile>/libdriver.so`, the crate's `cdylib`.
//!
//! The C `.so` for the configuration matching the currently-enabled Cargo
//! features is chosen automatically (see `OP_NAME` / `REPEAT` below), so
//! `cargo test --no-default-features --features <combo>` picks up the right
//! reference library with no extra plumbing.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, CStr};
use std::path::{Path, PathBuf};

use libloading::{Library, Symbol};

/// The C type `int (*)(int, int)`.
pub type OpFn = unsafe extern "C" fn(c_int, c_int) -> c_int;

// ---------------------------------------------------------------------------
// Build-time configuration, resolved exactly like the crate itself resolves it
// ---------------------------------------------------------------------------

/// `STR(OP)` for the active feature set.
///
/// Cargo features are additive and cannot be made mutually exclusive, so the
/// crate documents a deterministic priority: `mul > sub > add`, defaulting to
/// `add` when no OP feature is on (matching `#ifndef OP` ⇒ `add`). The tests
/// must resolve it the same way in order to pick the matching C `.so`.
pub const OP_NAME: &str = if cfg!(feature = "mul") {
    "mul"
} else if cfg!(feature = "sub") {
    "sub"
} else {
    "add"
};

/// `REPEAT` for the active feature set: highest enabled number wins, default
/// `5` (matching `#ifndef REPEAT` ⇒ `5`).
pub const REPEAT: c_int = if cfg!(feature = "7") {
    7
} else if cfg!(feature = "6") {
    6
} else if cfg!(feature = "5") {
    5
} else if cfg!(feature = "4") {
    4
} else if cfg!(feature = "3") {
    3
} else if cfg!(feature = "2") {
    2
} else if cfg!(feature = "1") {
    1
} else if cfg!(feature = "0") {
    0
} else {
    5
};

/// `INIT_FOR(OP)`: `INIT_add == 0`, `INIT_sub == 0`, `INIT_mul == 1`.
pub const INIT_FOR: c_int = if cfg!(feature = "mul") { 1 } else { 0 };

// ---------------------------------------------------------------------------
// Locating the two shared objects
// ---------------------------------------------------------------------------

/// The workspace root (the directory holding `c_src/`, `translation/`, `cbuild/`).
pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

/// Cargo's target directory for this test run, derived from the test binary's
/// own location (`target/<profile>/deps/<test>`), so it honours
/// `CARGO_TARGET_DIR` and whichever profile is in use.
pub fn target_profile_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    exe.parent()
        .and_then(|deps| deps.parent())
        .expect("test binary should live in <target>/<profile>/deps/")
        .to_path_buf()
}

/// Path to the C reference `.so` for the active configuration.
pub fn c_so_path() -> PathBuf {
    let p = repo_root()
        .join("cbuild")
        .join(format!("libcdriver_{OP_NAME}_{REPEAT}.so"));
    assert!(
        p.is_file(),
        "missing C reference library {}\n\
         Build it first:  ./build_c_so.sh   (from the workspace root)",
        p.display()
    );
    p
}

/// Path to the C reference `driver` executable for the active configuration.
pub fn c_driver_path() -> PathBuf {
    let p = repo_root()
        .join("cbuild")
        .join(format!("cdriver_{OP_NAME}_{REPEAT}"));
    assert!(
        p.is_file(),
        "missing C driver executable {}\n\
         Build it first:  ./build_c_so.sh",
        p.display()
    );
    p
}

/// Path to the Rust `cdylib` under test.
pub fn rust_so_path() -> PathBuf {
    let p = target_profile_dir().join("libdriver.so");
    assert!(
        p.is_file(),
        "missing Rust cdylib {}\n\
         `cargo test` does not always emit the cdylib; build it first with the \
         same features, e.g.\n  cargo build --no-default-features --features <combo>",
        p.display()
    );
    p
}

/// Path to the Rust `driver` executable.
pub fn rust_driver_path() -> PathBuf {
    let p = target_profile_dir().join("driver");
    assert!(
        p.is_file(),
        "missing Rust driver executable {} — build it first with the same features",
        p.display()
    );
    p
}

// ---------------------------------------------------------------------------
// The loaded API surface (all 8 exported symbols)
// ---------------------------------------------------------------------------

/// Every symbol `nm -D` reports for the C `.so`, resolved out of one library.
///
/// `_lib` is kept alive for as long as the resolved pointers are used; the
/// function pointers and data addresses are copies of the `dlsym` results, so
/// they stay valid exactly as long as the handle does.
pub struct Api {
    /// Human-readable tag used in assertion messages (`"C"` / `"Rust"`).
    pub tag: &'static str,
    pub op_add: OpFn,
    pub op_sub: OpFn,
    pub op_mul: OpFn,
    pub helper_call: OpFn,
    pub helper_ptr: OpFn,
    pub use_generated: unsafe extern "C" fn(c_int) -> c_int,
    /// Address of the exported `int (*G_OP)(int,int)` object itself.
    pub g_op: *mut Option<OpFn>,
    /// Address of the exported `const char *G_OP_NAME` object itself.
    pub g_op_name: *mut *const c_char,
    _lib: Library,
}

impl Api {
    /// `dlopen` the library and resolve all eight exported symbols.
    ///
    /// # Panics
    /// If the library cannot be loaded or any symbol is missing — a missing
    /// symbol here *is* the Phase A/D failure we are looking for.
    pub fn load(path: &Path, tag: &'static str) -> Api {
        // SAFETY: the path points at one of the two libraries we just built;
        // loading runs its initialisers, which for both is only static init.
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()));

        // SAFETY: each symbol's declared type matches the C prototype in
        // c_src/src/mdmacros.h (all `int`-only signatures, no lifetimes).
        unsafe {
            let get_fn = |name: &[u8]| -> OpFn {
                let s: Symbol<OpFn> = lib
                    .get(name)
                    .unwrap_or_else(|e| panic!("{tag}: missing symbol {:?}: {e}", bstr(name)));
                *s
            };
            let op_add = get_fn(b"op_add\0");
            let op_sub = get_fn(b"op_sub\0");
            let op_mul = get_fn(b"op_mul\0");
            let helper_call = get_fn(b"helper_call\0");
            let helper_ptr = get_fn(b"helper_ptr\0");

            let ug: Symbol<unsafe extern "C" fn(c_int) -> c_int> = lib
                .get(b"use_generated\0")
                .unwrap_or_else(|e| panic!("{tag}: missing symbol use_generated: {e}"));
            let use_generated = *ug;

            let gop: Symbol<*mut Option<OpFn>> = lib
                .get(b"G_OP\0")
                .unwrap_or_else(|e| panic!("{tag}: missing symbol G_OP: {e}"));
            let g_op = *gop;

            let gname: Symbol<*mut *const c_char> = lib
                .get(b"G_OP_NAME\0")
                .unwrap_or_else(|e| panic!("{tag}: missing symbol G_OP_NAME: {e}"));
            let g_op_name = *gname;

            Api {
                tag,
                op_add,
                op_sub,
                op_mul,
                helper_call,
                helper_ptr,
                use_generated,
                g_op,
                g_op_name,
                _lib: lib,
            }
        }
    }

    /// The current value of the `G_OP` global.
    pub fn g_op_value(&self) -> Option<OpFn> {
        // SAFETY: `g_op` is the address of an 8-byte exported object in `.data`.
        unsafe { *self.g_op }
    }

    /// Overwrite the `G_OP` global — legal in C because the object lives in the
    /// writable `.data` section (`nm` type `D`).
    pub fn set_g_op(&self, v: Option<OpFn>) {
        // SAFETY: as above; the object is writable in both libraries.
        unsafe { *self.g_op = v }
    }

    /// Call through the `G_OP` global, as `mdmain.c` does (`int g = G_OP(a,b);`).
    pub fn call_g_op(&self, a: c_int, b: c_int) -> c_int {
        let f = self.g_op_value().expect("G_OP must not be null");
        // SAFETY: `f` came out of `G_OP`, whose C type is `int (*)(int,int)`.
        unsafe { f(a, b) }
    }

    /// `G_OP_NAME` read as a NUL-terminated C string.
    pub fn g_op_name_bytes(&self) -> Vec<u8> {
        // SAFETY: `*g_op_name` is a `const char *` to a NUL-terminated literal.
        unsafe { CStr::from_ptr(*self.g_op_name).to_bytes().to_vec() }
    }

    /// The current value of the `G_OP_NAME` pointer.
    pub fn g_op_name_ptr(&self) -> *const c_char {
        // SAFETY: address of an exported 8-byte object.
        unsafe { *self.g_op_name }
    }

    pub fn set_g_op_name(&self, v: *const c_char) {
        // SAFETY: writable `.data` object in both libraries.
        unsafe { *self.g_op_name = v }
    }

    /// Address of the `op_*` function that `OP_FN(OP)` selects for this build.
    pub fn selected_op(&self) -> OpFn {
        match OP_NAME {
            "mul" => self.op_mul,
            "sub" => self.op_sub,
            _ => self.op_add,
        }
    }
}

fn bstr(b: &[u8]) -> String {
    String::from_utf8_lossy(b.strip_suffix(b"\0").unwrap_or(b)).into_owned()
}

/// Load both libraries for the active configuration.
///
/// Also guards against a **stale** `target/<profile>/libdriver.so`: `cargo test`
/// does not reliably re-emit the `cdylib` when only the feature set changes, so
/// running `cargo test --features X` after `cargo build --features Y` can leave a
/// `.so` built for `Y` on disk. That would silently compare the wrong two
/// libraries, so it is detected here and reported as a build problem rather than
/// showing up as a confusing "divergence".
pub fn load_pair() -> (Api, Api) {
    let c = Api::load(&c_so_path(), "C");
    let r = Api::load(&rust_so_path(), "Rust");
    assert_stamp(&r);
    assert_stamp(&c);
    (c, r)
}

/// Verify a loaded library really was built for the active `(OP, REPEAT)`.
///
/// `G_OP_NAME` pins `OP`, and `helper_call(0, 0)` pins `REPEAT`: `op(0,0)` is `0`
/// for all three ops, so the return value is exactly the `RUN_LOOP` accumulator.
fn assert_stamp(api: &Api) {
    let name = api.g_op_name_bytes();
    assert_eq!(
        name,
        OP_NAME.as_bytes(),
        "{} library at the expected path was built for OP={:?}, but the active \
         feature set resolves to OP={OP_NAME}. Rebuild with the same features:\n  \
         cargo build --no-default-features --features <combo>",
        api.tag,
        String::from_utf8_lossy(&name)
    );

    let mut acc = INIT_FOR;
    let mut i: c_int = 0;
    while i < REPEAT {
        acc = if OP_NAME == "mul" {
            acc.wrapping_mul(i.wrapping_add(1))
        } else if OP_NAME == "sub" {
            acc.wrapping_sub(i)
        } else {
            acc.wrapping_add(i)
        };
        i += 1;
    }
    // SAFETY: `int helper_call(int, int)`.
    let got = unsafe { (api.helper_call)(0, 0) };
    assert_eq!(
        got, acc,
        "{} library was built for a different REPEAT: helper_call(0,0) == {got}, \
         but the active feature set resolves to REPEAT={REPEAT} (expected {acc}). \
         Rebuild with the same features.",
        api.tag
    );
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64) — fixed seed, so every run is reproducible
// ---------------------------------------------------------------------------

/// The seed used by every property-style sweep in the suite.
pub const SEED: u64 = 0x5EED_C0FF_EE00_1234;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
    }
    pub fn next_u64(&mut self) -> u64 {
        // SplitMix64
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// A full-range `int` — deliberately includes values that make the C
    /// arithmetic wrap.
    pub fn next_i32(&mut self) -> c_int {
        self.next_u64() as u32 as c_int
    }
    /// A value biased towards interesting magnitudes: sometimes tiny, sometimes
    /// near the `int` limits, sometimes uniform. Pure-uniform 32-bit values
    /// almost never produce small results, which would miss whole classes of
    /// bugs.
    pub fn next_i32_biased(&mut self) -> c_int {
        let r = self.next_u64();
        match r % 5 {
            0 => (r >> 8) as i8 as c_int,                        // tiny
            1 => (r >> 8) as i16 as c_int,                       // small
            2 => c_int::MAX - ((r >> 8) as u16 as c_int),         // near INT_MAX
            3 => c_int::MIN.wrapping_add((r >> 8) as u16 as c_int), // near INT_MIN
            _ => (r >> 8) as u32 as c_int,                       // uniform
        }
    }
}

/// The hand-picked corner values every `(a, b)` sweep includes in full
/// cross-product, on top of the randomised pairs.
pub const CORNERS: &[c_int] = &[
    0,
    1,
    -1,
    2,
    -2,
    3,
    -3,
    7,
    -7,
    i32::MAX,
    i32::MIN,
    i32::MAX - 1,
    i32::MIN + 1,
    0x7FFF,
    -0x8000,
    0x1_0000,
    -0x1_0000,
    46341,  // ceil(sqrt(INT_MAX)) — smallest square that overflows
    -46341,
    65536,
];

/// The `n` shapes for `use_generated`: every `switch` arm, both boundaries, and
/// the extremes. See `CONFIGS.md` axis 3 and `ERRORS.md` rows 1–7.
pub const N_SHAPES: &[c_int] = &[
    i32::MIN,
    i32::MIN + 1,
    -1000,
    -8,
    -7,
    -2,
    -1,
    0,
    1,
    2,
    3,
    4,
    5,
    6,
    7,
    8,
    9,
    1000,
    i32::MAX - 1,
    i32::MAX,
];

/// Assert two results agree, with a message naming the entry point and inputs.
#[track_caller]
pub fn same(what: &str, inputs: &str, c: c_int, rust: c_int) {
    assert_eq!(
        c, rust,
        "divergence in {what}({inputs}) [OP={OP_NAME} REPEAT={REPEAT}]: C={c} Rust={rust}"
    );
}
