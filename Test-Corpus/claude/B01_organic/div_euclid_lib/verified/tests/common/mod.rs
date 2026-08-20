//! Shared differential-test harness.
//!
//! Both implementations are loaded as *shared objects* with `libloading` and
//! called only through their exported `div_euclid` symbol — the Rust side is
//! never called directly as a Rust function, so the `#[unsafe(no_mangle)]
//! extern "C"` wrapper and the C ABI are part of what is being tested.

#![allow(dead_code)]

use std::ffi::c_int;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

/// The C signature exactly as declared in `c_src/include/lib.h`.
pub type DivFn = extern "C" fn(c_int, c_int) -> c_int;
/// The same symbol viewed through a 64-bit signature, used to poke dirty upper
/// halves into the argument registers (ABI-level boundary test).
pub type DivFnRaw = extern "C" fn(i64, i64) -> i64;

pub const I32_MIN: i32 = i32::MIN; // -0x7fffffff - 1
pub const I32_MAX: i32 = i32::MAX;

pub struct Libs {
    pub c_path: PathBuf,
    pub rust_path: PathBuf,
    pub c: DivFn,
    pub rust: DivFn,
    pub c_raw: DivFnRaw,
    pub rust_raw: DivFnRaw,
}

static LIBS: OnceLock<Libs> = OnceLock::new();

pub fn libs() -> &'static Libs {
    LIBS.get_or_init(|| {
        let c_path = c_so_path();
        let rust_path = rust_so_path();
        let (c, c_raw) = load(&c_path);
        let (rust, rust_raw) = load(&rust_path);
        Libs {
            c_path,
            rust_path,
            c,
            rust,
            c_raw,
            rust_raw,
        }
    })
}

fn load(path: &Path) -> (DivFn, DivFnRaw) {
    // Leaked on purpose: the returned function pointers must stay valid for the
    // whole process lifetime and the library must never be unloaded.
    let lib: &'static libloading::Library = Box::leak(Box::new(unsafe {
        libloading::Library::new(path)
            .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()))
    }));
    let f: DivFn = unsafe {
        *lib.get::<DivFn>(b"div_euclid\0")
            .unwrap_or_else(|e| panic!("dlsym(div_euclid) in {}: {e}", path.display()))
    };
    let f_raw: DivFnRaw = unsafe {
        *lib.get::<DivFnRaw>(b"div_euclid\0")
            .unwrap_or_else(|e| panic!("dlsym(div_euclid) in {}: {e}", path.display()))
    };
    (f, f_raw)
}

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Path to the C reference `.so`. Always (re)built so a stale artifact can
/// never produce a false pass. `c_src/` itself is never modified — cmake only
/// writes into the `c_src/build/` output directory.
pub fn c_so_path() -> PathBuf {
    let so = manifest_dir().join("c_src/build/libtranslated_rust.so");
    let build = manifest_dir().join("c_src/build");
    std::fs::create_dir_all(&build).expect("create c_src/build");
    run(
        Command::new("cmake")
            .arg("..")
            .arg("-DCMAKE_POSITION_INDEPENDENT_CODE=ON")
            .current_dir(&build),
        "cmake configure",
    );
    run(
        Command::new("cmake")
            .args(["--build", "."])
            .current_dir(&build),
        "cmake build",
    );
    assert!(so.exists(), "C library not produced at {}", so.display());
    so
}

/// Path to the Rust cdylib for the profile the test binary itself was built
/// with. A cdylib-only crate is not built automatically by `cargo test`, so the
/// build is always driven from here (also guarantees the `.so` is never stale
/// with respect to `src/lib.rs`).
pub fn rust_so_path() -> PathBuf {
    const NAME: &str = "libdiv_euclid_lib.so";

    // .../target/<profile>/deps/<test-binary>
    let exe = std::env::current_exe().expect("current_exe");
    let mut dir = exe.parent().expect("exe dir").to_path_buf();
    if dir.file_name().map(|n| n == "deps").unwrap_or(false) {
        dir.pop();
    }
    let profile_release = dir.file_name().map(|n| n == "release").unwrap_or(false);
    let candidate = dir.join(NAME);

    // Build it (same profile and same feature set as the test binary).
    let mut cmd = Command::new(env!("CARGO"));
    cmd.arg("build").arg("--offline");
    if profile_release {
        cmd.arg("--release");
    }
    if cfg!(feature = "default") {
        cmd.arg("--features").arg("default");
    } else {
        cmd.arg("--no-default-features");
    }
    cmd.current_dir(manifest_dir());
    run(&mut cmd, "cargo build (cdylib)");

    for p in [
        candidate,
        manifest_dir().join("target/debug").join(NAME),
        manifest_dir().join("target/release").join(NAME),
    ] {
        if p.exists() {
            return p;
        }
    }
    panic!("Rust cdylib {NAME} not found; run `cargo build` first");
}

fn run(cmd: &mut Command, what: &str) {
    let out = cmd
        .output()
        .unwrap_or_else(|e| panic!("spawning {what} failed: {e}"));
    assert!(
        out.status.success(),
        "{what} failed: {:?}\nstdout:\n{}\nstderr:\n{}",
        out.status,
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

// ---------------------------------------------------------------------------
// deterministic PRNG (SplitMix64) — no external crate, reproducible seeds
// ---------------------------------------------------------------------------

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
    /// Uniform over the whole `i32` range (every bit pattern reachable).
    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    /// Uniform in `[lo, hi]` (inclusive), `lo <= hi`.
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        debug_assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
    /// Uniform in `[1, hi]`.
    pub fn pos_i32(&mut self, hi: i32) -> i32 {
        self.range_i32(1, hi)
    }
}

// ---------------------------------------------------------------------------
// differential assertions
// ---------------------------------------------------------------------------

/// Call both `.so` exports and require byte-identical results.
#[track_caller]
pub fn assert_same(row: &str, v1: i32, v2: i32) -> i32 {
    let l = libs();
    let c = (l.c)(v1, v2);
    let r = (l.rust)(v1, v2);
    assert_eq!(
        c, r,
        "[{row}] divergence for div_euclid({v1}, {v2}): C={c} (0x{c:08x}) Rust={r} (0x{r:08x})"
    );
    // byte-for-byte on the raw representation as well
    assert_eq!(
        c.to_ne_bytes(),
        r.to_ne_bytes(),
        "[{row}] byte representation differs for div_euclid({v1}, {v2})"
    );
    c
}

/// Same, but through the 64-bit ABI view with dirty upper halves.
#[track_caller]
pub fn assert_same_raw(row: &str, a: i64, b: i64) {
    let l = libs();
    let c = (l.c_raw)(a, b);
    let r = (l.rust_raw)(a, b);
    assert_eq!(
        c as i32, r as i32,
        "[{row}] divergence (raw ABI) for div_euclid(0x{a:016x}, 0x{b:016x}): C={c:#x} Rust={r:#x}"
    );
}

pub fn assert_all_same(row: &str, pairs: impl IntoIterator<Item = (i32, i32)>) {
    let mut n = 0usize;
    for (v1, v2) in pairs {
        assert_same(row, v1, v2);
        n += 1;
    }
    assert!(n > 0, "[{row}] generated no inputs");
}

/// The curated boundary set: every value the C code special-cases plus their
/// immediate neighbours, powers of two, and small magnitudes.
pub fn boundary_values() -> Vec<i32> {
    let mut v: Vec<i32> = Vec::new();
    // small magnitudes around zero
    for x in -20i32..=20 {
        v.push(x);
    }
    // powers of two and their neighbours, both signs
    for k in 0..31u32 {
        let p = 1i32 << k;
        for d in [-1i32, 0, 1] {
            if let Some(x) = p.checked_add(d) {
                v.push(x);
                v.push(x.wrapping_neg());
            }
        }
    }
    // INT_MIN / INT_MAX neighbourhoods
    for d in 0..6i32 {
        v.push(I32_MIN.wrapping_add(d));
        v.push(I32_MAX - d);
    }
    // a few mid-range odd/prime-ish values
    v.extend_from_slice(&[
        3, 7, 11, 13, 97, 1_000, 1_001, 65_535, 65_536, 65_537, 1_000_000_007, 1_431_655_765,
        -3, -7, -11, -13, -97, -1_000, -1_001, -65_535, -65_536, -65_537, -1_000_000_007,
        -1_431_655_765,
    ]);
    v.sort_unstable();
    v.dedup();
    v
}
