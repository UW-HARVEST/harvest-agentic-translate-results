//! Shared differential-test harness.
//!
//! Both the C reference and the Rust translation are loaded as **shared
//! objects** through `libloading` and called through their exported
//! `extern "C"` symbols. The Rust functions are never called directly, so the
//! `#[no_mangle]` export wrappers are part of what is under test.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// ABI of `int div_euclid(int, int)` from `c_src/include/lib.h`.
pub type DivEuclidFn = unsafe extern "C" fn(i32, i32) -> i32;

pub const I32_MIN: i32 = -0x7fffffff - 1; // exactly how c_src spells INT_MIN
pub const I32_MAX: i32 = 0x7fffffff;

/// Locate the C `.so`. The CMake project name is derived from the parent
/// directory of `c_src`, so the file name is not stable -> glob for it.
fn find_c_so() -> PathBuf {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let build_dir = manifest
        .parent()
        .expect("manifest dir has a parent")
        .join("c_src")
        .join("build");

    let entries = fs::read_dir(&build_dir).unwrap_or_else(|e| {
        panic!(
            "cannot read {}: {e}. Build the C library first:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build_dir.display()
        )
    });

    let mut candidates: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
            name.starts_with("lib") && name.ends_with(".so") && p.is_file()
        })
        .collect();
    candidates.sort();

    assert_eq!(
        candidates.len(),
        1,
        "expected exactly one C shared object in {}, found {:?}",
        build_dir.display(),
        candidates
    );
    candidates.pop().unwrap()
}

/// Locate the Rust `cdylib`. `current_exe()` is
/// `<target>/<profile>/deps/<testname>-<hash>`, so the profile dir is two
/// levels up; this stays correct under a custom `CARGO_TARGET_DIR`.
///
/// `cargo test` does **not** build (or refresh) the `cdylib` artifact, so we
/// build it ourselves *unconditionally*. Building only when the file is missing
/// is not enough: a `.so` left over from an earlier `src/lib.rs` would be
/// silently tested instead of the current source. `cargo build` is a no-op when
/// already up to date, so this is cheap.
fn find_rust_so() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("<target>/<profile>/deps/<exe>")
        .to_path_buf();
    let so = profile_dir.join("libdiv_euclid_lib.so");

    let is_release = profile_dir
        .file_name()
        .and_then(|s| s.to_str())
        .map(|s| s == "release")
        .unwrap_or(false);

    let mut cmd = std::process::Command::new(
        std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string()),
    );
    cmd.current_dir(env!("CARGO_MANIFEST_DIR"))
        .arg("build")
        .arg("--offline")
        .arg("--lib");
    if is_release {
        cmd.arg("--release");
    }
    let out = cmd.output().expect("failed to spawn `cargo build`");
    assert!(
        out.status.success(),
        "`cargo build --offline --lib` failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );

    assert!(
        so.is_file(),
        "Rust cdylib still not found at {} after `cargo build`",
        so.display()
    );
    so
}

static FUNCS: OnceLock<(DivEuclidFn, DivEuclidFn)> = OnceLock::new();

/// `(c_div_euclid, rust_div_euclid)`, both resolved out of their `.so`.
pub fn funcs() -> (DivEuclidFn, DivEuclidFn) {
    *FUNCS.get_or_init(|| unsafe {
        let c_path = find_c_so();
        let r_path = find_rust_so();

        // Leaked so the `Symbol`s (and the code they point at) stay valid for
        // the whole test process.
        let c_lib: &'static Library = Box::leak(Box::new(
            Library::new(&c_path)
                .unwrap_or_else(|e| panic!("dlopen C {}: {e}", c_path.display())),
        ));
        let r_lib: &'static Library = Box::leak(Box::new(
            Library::new(&r_path)
                .unwrap_or_else(|e| panic!("dlopen Rust {}: {e}", r_path.display())),
        ));

        let c_sym: Symbol<DivEuclidFn> = c_lib
            .get(b"div_euclid\0")
            .expect("C .so must export `div_euclid`");
        let r_sym: Symbol<DivEuclidFn> = r_lib
            .get(b"div_euclid\0")
            .expect("Rust .so must export `div_euclid` (check #[no_mangle])");

        (*c_sym, *r_sym)
    })
}

/// Call both `.so`s with `(v1, v2)` and assert byte-identical results.
/// Returns the (agreed) result.
#[track_caller]
pub fn check(v1: i32, v2: i32) -> i32 {
    let (c, r) = funcs();
    let cv = unsafe { c(v1, v2) };
    let rv = unsafe { r(v1, v2) };
    assert_eq!(
        cv, rv,
        "DIVERGENCE div_euclid({v1}, {v2}): C returned {cv} (0x{cv:08x}), Rust returned {rv} (0x{rv:08x})"
    );
    cv
}

/// Assert both `.so`s agree *and* that the result equals `expected`, which is
/// read off the C source by hand. Catches "both wrong the same way" only for
/// the rows where the C result is independently derivable.
#[track_caller]
pub fn check_eq(v1: i32, v2: i32, expected: i32) {
    let got = check(v1, v2);
    assert_eq!(
        got, expected,
        "div_euclid({v1}, {v2}): both libs returned {got}, but the C source dictates {expected}"
    );
}

/// Bulk comparison that keeps going after the first divergence so a failure
/// report shows the *shape* of the bug, not just one sample.
pub struct Cmp {
    label: &'static str,
    calls: u64,
    total_mismatches: u64,
    mismatches: Vec<(i32, i32, i32, i32)>,
}

impl Cmp {
    pub fn new(label: &'static str) -> Self {
        Cmp { label, calls: 0, total_mismatches: 0, mismatches: Vec::new() }
    }

    pub fn feed(&mut self, v1: i32, v2: i32) {
        let (c, r) = funcs();
        let cv = unsafe { c(v1, v2) };
        let rv = unsafe { r(v1, v2) };
        self.calls += 1;
        if cv != rv {
            self.total_mismatches += 1;
            // Record only the first 40 so a failure report stays readable, but
            // keep counting all of them.
            if self.mismatches.len() < 40 {
                self.mismatches.push((v1, v2, cv, rv));
            }
        }
    }

    pub fn calls(&self) -> u64 {
        self.calls
    }

    #[track_caller]
    pub fn finish(self, min_calls: u64) {
        assert!(
            self.calls >= min_calls,
            "{}: only {} comparisons made, expected >= {} (test generated too few inputs)",
            self.label,
            self.calls,
            min_calls
        );
        if !self.mismatches.is_empty() {
            let mut msg = format!(
                "{}: {} divergence(s) out of {} comparisons (showing {}):\n",
                self.label,
                self.total_mismatches,
                self.calls,
                self.mismatches.len()
            );
            for (v1, v2, cv, rv) in &self.mismatches {
                msg.push_str(&format!("  div_euclid({v1}, {v2}) -> C={cv} Rust={rv}\n"));
            }
            panic!("{msg}");
        }
    }
}

/// PCG32 — deterministic, fixed-seed, reproducible across runs and platforms.
pub struct Pcg32 {
    state: u64,
    inc: u64,
}

impl Pcg32 {
    pub fn new(seed: u64) -> Self {
        let mut p = Pcg32 { state: 0, inc: (seed << 1) | 1 };
        p.next_u32();
        p.state = p.state.wrapping_add(0x853c_49e6_748f_ea9b ^ seed);
        p.next_u32();
        p
    }

    pub fn next_u32(&mut self) -> u32 {
        let old = self.state;
        self.state = old
            .wrapping_mul(6364136223846793005)
            .wrapping_add(self.inc);
        let xorshifted = (((old >> 18) ^ old) >> 27) as u32;
        let rot = (old >> 59) as u32;
        xorshifted.rotate_right(rot)
    }

    /// Uniform over the whole `i32` domain (every bit pattern reachable).
    pub fn i32_any(&mut self) -> i32 {
        self.next_u32() as i32
    }

    /// Uniform in `[lo, hi]` inclusive, computed in `i64` to avoid overflow.
    pub fn i32_in(&mut self, lo: i32, hi: i32) -> i32 {
        debug_assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        let r = ((self.next_u32() as u64) << 32 | self.next_u32() as u64) % span;
        (lo as i64 + r as i64) as i32
    }

    /// Strictly positive: `[1, INT_MAX]`.
    pub fn pos(&mut self) -> i32 {
        self.i32_in(1, I32_MAX)
    }

    /// Strictly negative but not `INT_MIN`: `[INT_MIN+1, -1]`.
    pub fn neg_nonmin(&mut self) -> i32 {
        self.i32_in(I32_MIN + 1, -1)
    }

    /// Non-negative: `[0, INT_MAX]`.
    pub fn nonneg(&mut self) -> i32 {
        self.i32_in(0, I32_MAX)
    }
}

/// Boundary representatives used across many rows: the values the C's
/// comparisons (`== 0`, `>= 0`, `!= INT_MIN`) can possibly distinguish, plus
/// one step either side of each.
pub const BOUNDARIES: &[i32] = &[
    I32_MIN,
    I32_MIN + 1,
    I32_MIN + 2,
    -1073741824, // -2^30
    -65536,
    -256,
    -3,
    -2,
    -1,
    0,
    1,
    2,
    3,
    256,
    65536,
    1073741824, // 2^30
    I32_MAX - 2,
    I32_MAX - 1,
    I32_MAX,
];
