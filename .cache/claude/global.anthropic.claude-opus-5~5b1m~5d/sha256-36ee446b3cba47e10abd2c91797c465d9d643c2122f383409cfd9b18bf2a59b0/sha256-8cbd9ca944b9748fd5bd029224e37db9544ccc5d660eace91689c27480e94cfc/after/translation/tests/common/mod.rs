//! Shared differential-test harness.
//!
//! BOTH implementations are loaded as shared objects through `libloading` and
//! called through the FFI boundary. The Rust function is *never* called
//! directly — it always goes through `libencode_quant_lib.so`'s exported
//! `encode_quant` symbol, so the `#[no_mangle] extern "C"` wrapper is under
//! test too.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

pub type EncodeQuantFn = unsafe extern "C" fn(
    std::ffi::c_int,
    std::ffi::c_int,
    std::ffi::c_int,
    std::ffi::c_int,
    std::ffi::c_int,
    std::ffi::c_int,
) -> std::ffi::c_int;

/// Repository root (parent of the `translation` crate directory).
fn repo_root() -> PathBuf {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    crate_dir
        .parent()
        .expect("crate dir has a parent")
        .to_path_buf()
}

/// Locate the C shared object. Its name derives from the parent directory
/// name in `CMakeLists.txt`, so we glob for any `.so` under `c_src/build`.
fn find_c_so() -> PathBuf {
    let build_dir = repo_root().join("c_src").join("build");
    assert!(
        build_dir.is_dir(),
        "C build dir {} missing — run:\n  cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        build_dir.display()
    );

    let mut found: Vec<PathBuf> = Vec::new();
    collect_so(&build_dir, &mut found, 0);
    found.sort();
    found
        .into_iter()
        .next()
        .unwrap_or_else(|| panic!("no .so found under {}", build_dir.display()))
}

fn collect_so(dir: &Path, out: &mut Vec<PathBuf>, depth: usize) {
    if depth > 3 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_so(&path, out, depth + 1);
        } else if path.extension().and_then(|e| e.to_str()) == Some("so") {
            out.push(path);
        }
    }
}

/// Locate the Rust cdylib next to the test binary.
fn find_rust_so() -> PathBuf {
    // The integration-test executable lives in `target/<profile>/deps/`;
    // the cdylib lives in `target/<profile>/`.
    let exe = std::env::current_exe().expect("current_exe");
    let mut dir = exe.parent().expect("deps dir").to_path_buf();
    if dir.file_name().and_then(|n| n.to_str()) == Some("deps") {
        dir.pop();
    }
    let candidate = dir.join("libencode_quant_lib.so");
    if candidate.is_file() {
        return candidate;
    }
    // Fall back to scanning both common profile dirs.
    for profile in ["debug", "release"] {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join(profile)
            .join("libencode_quant_lib.so");
        if p.is_file() {
            return p;
        }
    }
    panic!(
        "libencode_quant_lib.so not found near {} — run `cargo build` first",
        dir.display()
    );
}

/// Fail loudly if `so` is older than any of `sources`.
///
/// This guard exists because `cargo test` does NOT rebuild a `cdylib` target:
/// without it, the tests happily load a stale `.so` and every assertion passes
/// vacuously, hiding real divergences. Build with `./run_tests.sh` (or
/// `cargo build --release` before `cargo test --release`).
fn assert_fresh(so: &Path, sources: &[PathBuf], how_to_build: &str) {
    let so_mtime = std::fs::metadata(so)
        .and_then(|m| m.modified())
        .unwrap_or_else(|e| panic!("stat {}: {e}", so.display()));
    for src in sources {
        let Ok(src_mtime) = std::fs::metadata(src).and_then(|m| m.modified()) else {
            continue;
        };
        assert!(
            so_mtime >= src_mtime,
            "STALE SHARED OBJECT: {} is older than {}.\n\
             `cargo test` does not rebuild cdylib/C targets, so the tests would \
             be comparing against an out-of-date library and would pass \
             vacuously.\nRebuild with:\n  {how_to_build}",
            so.display(),
            src.display()
        );
    }
}

struct Libs {
    c: Library,
    rust: Library,
}

// Safety: the loaded libraries are leaked for the process lifetime and the
// functions they export are pure, so sharing them across threads is sound.
unsafe impl Send for Libs {}
unsafe impl Sync for Libs {}

fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| {
        let c_path = find_c_so();
        let rust_path = find_rust_so();

        // Guard against loading stale libraries (see assert_fresh).
        let root = repo_root();
        assert_fresh(
            &c_path,
            &[
                root.join("c_src").join("src").join("lib.c"),
                root.join("c_src").join("include").join("lib.h"),
            ],
            "cd c_src/build && cmake --build .",
        );
        assert_fresh(
            &rust_path,
            &[PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("src")
                .join("lib.rs")],
            "cd translation && cargo build --release   # (cargo test alone does NOT rebuild cdylibs)",
        );
        assert_ne!(
            std::fs::canonicalize(&c_path).unwrap(),
            std::fs::canonicalize(&rust_path).unwrap(),
            "C and Rust .so resolved to the SAME file — the comparison would be vacuous"
        );

        unsafe {
            Libs {
                c: Library::new(&c_path)
                    .unwrap_or_else(|e| panic!("loading {}: {e}", c_path.display())),
                rust: Library::new(&rust_path)
                    .unwrap_or_else(|e| panic!("loading {}: {e}", rust_path.display())),
            }
        }
    })
}

/// The two `encode_quant` implementations, both reached via `dlsym`.
pub struct Pair {
    pub c: EncodeQuantFn,
    pub rust: EncodeQuantFn,
}

pub fn pair() -> &'static Pair {
    static PAIR: OnceLock<Pair> = OnceLock::new();
    PAIR.get_or_init(|| {
        let l = libs();
        unsafe {
            let c: Symbol<EncodeQuantFn> = l
                .c
                .get(b"encode_quant\0")
                .expect("C .so does not export encode_quant");
            let rust: Symbol<EncodeQuantFn> = l
                .rust
                .get(b"encode_quant\0")
                .expect("Rust .so does not export encode_quant");
            Pair { c: *c, rust: *rust }
        }
    })
}

/// Call both implementations with identical arguments and assert the returned
/// `int`s are bit-for-bit identical.
#[track_caller]
pub fn check(uni: i32, step: i32, pred: i32, tgt: i32, tgt2: i32, lsbit: i32) {
    let p = pair();
    let c_out = unsafe { (p.c)(uni, step, pred, tgt, tgt2, lsbit) };
    let rust_out = unsafe { (p.rust)(uni, step, pred, tgt, tgt2, lsbit) };
    assert_eq!(
        c_out, rust_out,
        "divergence for encode_quant(uni={uni}, step={step}, pred={pred}, \
         tgt={tgt}, tgt2={tgt2}, lsbit={lsbit}): C={c_out} (0x{c_out:08x}) \
         Rust={rust_out} (0x{rust_out:08x})"
    );
}

/// Deterministic xorshift64* PRNG so every "randomized" row is reproducible.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(if seed == 0 { 0x9E3779B97F4A7C15 } else { seed })
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }

    pub fn next_i32(&mut self) -> i32 {
        (self.next_u64() >> 32) as u32 as i32
    }

    /// Uniform in `0..n`.
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }

    /// Uniform in `lo..=hi`.
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + self.below(span) as i64) as i32
    }

    /// Pick one element of a slice.
    pub fn pick<T: Copy>(&mut self, xs: &[T]) -> T {
        xs[self.below(xs.len() as u64) as usize]
    }

    /// A value biased toward interesting magnitudes: boundary constants,
    /// small values, and full-range noise.
    pub fn interesting_i32(&mut self) -> i32 {
        const SPECIALS: [i32; 17] = [
            0, 1, -1, 2, -2, 7, -7, 8, -8, 15, -15, 16, -16, 63, -63, i32::MAX, i32::MIN,
        ];
        match self.below(4) {
            0 => self.pick(&SPECIALS),
            1 => self.range_i32(-64, 64),
            2 => {
                // near a boundary
                let base = self.pick(&[i32::MIN, i32::MAX, 0]);
                base.wrapping_add(self.range_i32(-8, 8))
            }
            _ => self.next_i32(),
        }
    }

    /// An `lsbit` value covering all four modes plus junk.
    pub fn lsbit(&mut self) -> i32 {
        match self.below(6) {
            0 => 0,
            1 => 4,
            2 => self.range_i32(-10, 10) | 1,          // odd
            3 => self.range_i32(-10, 10) & !1,         // even (may be 0)
            4 => self.pick(&[i32::MIN, i32::MAX, -4, -1, 1, 2, 3, 5, 6, 8]),
            _ => self.next_i32(),
        }
    }
}

/// Convenience generators. These exist as *methods* so a single `&mut self`
/// borrow covers the whole tuple — building several arguments inline as
/// separate `&mut rng` uses would not borrow-check.
impl Rng {
    /// A "typical" `step`: moderate magnitude, either sign.
    pub fn typical_step(&mut self) -> i32 {
        self.range_i32(-100_000, 100_000)
    }

    /// A "typical" `pred` / `tgt` / `tgt2`.
    pub fn typical_target(&mut self) -> i32 {
        self.range_i32(-1_000_000, 1_000_000)
    }

    /// `(step, pred, tgt, tgt2)` all typical.
    pub fn typical_tail(&mut self) -> (i32, i32, i32, i32) {
        let step = self.typical_step();
        let pred = self.typical_target();
        let tgt = self.typical_target();
        let tgt2 = self.typical_target();
        (step, pred, tgt, tgt2)
    }

    /// `uni` with the given `uni & 7`, randomized upper bits.
    pub fn uni_low3(&mut self, low: i32) -> i32 {
        let upper = self.range_i32(-(1 << 20), 1 << 20) & !7;
        upper | (low & 7)
    }

    /// `uni` with the given `uni & 7` and the given `uni & 8`.
    pub fn uni_low4(&mut self, low3: i32, bit3_set: bool) -> i32 {
        let upper = self.range_i32(-(1 << 20), 1 << 20) & !15;
        let mut v = upper | (low3 & 7);
        if bit3_set {
            v |= 8;
        }
        v
    }

    /// `uni` strictly inside its 3-bit group (`uni & 7` in `1..=6`), so both
    /// the `+1` and `-1` neighbours survive the `~7` clamp.
    pub fn uni_mid(&mut self, bit3_set: bool) -> i32 {
        let low3 = self.range_i32(1, 6);
        self.uni_low4(low3, bit3_set)
    }

    /// `uni` with a randomized `uni & 7` and a randomized `uni & 8`.
    pub fn uni_any_low4(&mut self) -> i32 {
        let low3 = self.range_i32(0, 7);
        let bit3 = self.below(2) == 1;
        self.uni_low4(low3, bit3)
    }

    /// `uni` pinned to a group boundary (`uni & 7` is 0 or 7) with a
    /// randomized `uni & 8`.
    pub fn uni_boundary(&mut self) -> i32 {
        let low3 = if self.below(2) == 0 { 0 } else { 7 };
        let bit3 = self.below(2) == 1;
        self.uni_low4(low3, bit3)
    }

    /// Six independent `interesting_i32()` values; the 6th is an `lsbit`.
    pub fn interesting_args(&mut self) -> [i32; 6] {
        let a = self.interesting_i32();
        let b = self.interesting_i32();
        let c = self.interesting_i32();
        let d = self.interesting_i32();
        let e = self.interesting_i32();
        let f = self.lsbit();
        [a, b, c, d, e, f]
    }

    /// Five independent `interesting_i32()` values (caller supplies `lsbit`).
    pub fn interesting5(&mut self) -> [i32; 5] {
        let a = self.interesting_i32();
        let b = self.interesting_i32();
        let c = self.interesting_i32();
        let d = self.interesting_i32();
        let e = self.interesting_i32();
        [a, b, c, d, e]
    }

    /// Four independent `interesting_i32()` values.
    pub fn interesting4(&mut self) -> [i32; 4] {
        let a = self.interesting_i32();
        let b = self.interesting_i32();
        let c = self.interesting_i32();
        let d = self.interesting_i32();
        [a, b, c, d]
    }

    /// Six uniform full-range `i32`s.
    pub fn uniform_args(&mut self) -> [i32; 6] {
        let mut out = [0i32; 6];
        for slot in out.iter_mut() {
            *slot = self.next_i32();
        }
        out
    }

    /// Four uniform full-range `i32`s.
    pub fn uniform4(&mut self) -> [i32; 4] {
        let mut out = [0i32; 4];
        for slot in out.iter_mut() {
            *slot = self.next_i32();
        }
        out
    }
}

/// `check` over a 6-element argument array.
#[track_caller]
pub fn check_args(a: [i32; 6]) {
    check(a[0], a[1], a[2], a[3], a[4], a[5]);
}

/// All four `lsbit` modes, with representatives of each.
pub const LSBIT_MODES: [i32; 20] = [
    0, // A0
    4, // A4
    1, 3, 5, 7, 9, 11, -1, -3, -5, i32::MAX, // Aodd
    2, 6, 8, 10, 12, -2, -4, i32::MIN, // Aeven
];
