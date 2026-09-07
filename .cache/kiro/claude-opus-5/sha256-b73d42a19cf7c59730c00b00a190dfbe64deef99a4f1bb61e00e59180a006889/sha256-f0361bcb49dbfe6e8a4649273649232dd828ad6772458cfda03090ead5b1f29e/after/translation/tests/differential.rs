//! Differential test harness: loads BOTH the C `.so` and the Rust `.so` through
//! `libloading` and compares their stdout byte-for-byte across the FFI boundary.
//!
//! No Rust function is ever called directly — every Rust invocation goes through
//! `dlsym("driver")` on `target/{debug,release}/libdriver.so`, so the
//! `#[no_mangle] extern "C"` export wrapper is under test too.
//!
//! Because the library's only observable effect is bytes written to libc's
//! `stdout`, output is captured at the *file-descriptor* level (`dup`/`dup2` on
//! fd 1) and `fflush(NULL)`ed, which captures the C and the Rust library
//! identically — both write through the same `FILE *stdout`.
//!
//! This target sets `harness = false` (see Cargo.toml): the stock libtest
//! harness writes progress text to fd 1 from other threads, which would be
//! captured as if the library had emitted it. The runner in `main` executes
//! every case sequentially so nothing else can write to fd 1 mid-capture.

use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::io::Write;
use std::os::fd::AsRawFd;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

type DriverFn = unsafe extern "C" fn(c_int);

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Impl {
    /// `c_src/build/libdriver.so`
    C,
    /// `translation/target/release/libdriver.so`
    RustRelease,
    /// `translation/target/debug/libdriver.so`
    RustDebug,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn so_path(which: Impl) -> PathBuf {
    match which {
        Impl::C => std::env::var_os("C_DRIVER_SO")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                manifest_dir()
                    .parent()
                    .expect("manifest dir has a parent")
                    .join("c_src/build/libdriver.so")
            }),
        Impl::RustRelease => manifest_dir().join("target/release/libdriver.so"),
        Impl::RustDebug => manifest_dir().join("target/debug/libdriver.so"),
    }
}

fn library(which: Impl) -> &'static Library {
    static C: OnceLock<Library> = OnceLock::new();
    static R_REL: OnceLock<Library> = OnceLock::new();
    static R_DBG: OnceLock<Library> = OnceLock::new();

    let slot = match which {
        Impl::C => &C,
        Impl::RustRelease => &R_REL,
        Impl::RustDebug => &R_DBG,
    };
    slot.get_or_init(|| {
        let path = so_path(which);
        assert!(
            path.is_file(),
            "{which:?} shared object not found at {}.\n\
             Build it first:\n  \
             C:     cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .\n  \
             Rust:  cd translation && cargo build && cargo build --release",
            path.display()
        );
        unsafe { Library::new(&path) }
            .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()))
    })
}

/// Resolve `driver` via `dlsym` on the given shared object.
fn driver_of(which: Impl) -> DriverFn {
    let lib = library(which);
    let sym: Symbol<'static, DriverFn> = unsafe { lib.get(b"driver\0") }
        .unwrap_or_else(|e| panic!("dlsym(\"driver\") failed for {which:?}: {e}"));
    *sym
}

// ---------------------------------------------------------------------------
// fd-level stdout capture (process-global, hence serialized)
// ---------------------------------------------------------------------------

static CAPTURE_LOCK: Mutex<()> = Mutex::new(());
static CAPTURE_SEQ: AtomicU64 = AtomicU64::new(0);

fn capture(f: &mut dyn FnMut()) -> Vec<u8> {
    // Poison-tolerant: a panic inside one case must not disable capture for the
    // rest of the suite.
    let _guard = CAPTURE_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let path = std::env::temp_dir().join(format!(
        "driver_diff_{}_{}.out",
        std::process::id(),
        CAPTURE_SEQ.fetch_add(1, Ordering::SeqCst)
    ));
    let file =
        std::fs::File::create(&path).unwrap_or_else(|e| panic!("create {}: {e}", path.display()));
    let fd = file.as_raw_fd();

    // Drain anything already pending on either buffer so it is not misattributed.
    let _ = std::io::stdout().flush();
    unsafe { fflush(std::ptr::null_mut()) };

    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { dup2(fd, 1) } >= 0, "dup2 onto stdout failed");

    f();

    // Flush the library's stdio writes into the redirected fd before restoring.
    unsafe { fflush(std::ptr::null_mut()) };
    assert!(unsafe { dup2(saved, 1) } >= 0, "dup2 restore failed");
    unsafe { close(saved) };

    drop(file);
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let _ = std::fs::remove_file(&path);
    bytes
}

/// Output of a single `driver(x)` call on the given implementation.
fn run_one(which: Impl, x: c_int) -> Vec<u8> {
    let f = driver_of(which);
    capture(&mut || unsafe { f(x) })
}

/// Output of `driver` applied to each of `xs` in order, in ONE stdout window.
fn run_many(which: Impl, xs: &[c_int]) -> Vec<u8> {
    let f = driver_of(which);
    capture(&mut || {
        for &x in xs {
            unsafe { f(x) }
        }
    })
}

fn show(b: &[u8]) -> String {
    String::from_utf8_lossy(b).escape_debug().to_string()
}

/// Core differential assertion: C `.so` and Rust `.so` must emit identical bytes.
#[track_caller]
fn assert_same_as_c(rust: Impl, x: c_int) {
    let c = run_one(Impl::C, x);
    let r = run_one(rust, x);
    assert_eq!(
        c,
        r,
        "divergence for driver({x}) [x as u32 = {:#010x}] between C and {rust:?}\n  C   : \"{}\"\n  Rust: \"{}\"",
        x as u32,
        show(&c),
        show(&r)
    );
}

#[track_caller]
fn assert_all_same_as_c(rust: Impl, xs: &[c_int]) {
    for &x in xs {
        assert_same_as_c(rust, x);
    }
}

#[track_caller]
fn assert_same(x: c_int) {
    assert_same_as_c(Impl::RustRelease, x);
}

#[track_caller]
fn assert_all_same(xs: &[c_int]) {
    assert_all_same_as_c(Impl::RustRelease, xs);
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seed for reproducibility
// ---------------------------------------------------------------------------

const SEED: u64 = 0x2545_F491_4F6C_DD1D;

struct Rng(u64);

impl Rng {
    fn new(stream: u64) -> Self {
        Rng(SEED ^ stream.wrapping_mul(0x9E37_79B9_7F4A_7C15))
    }
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    /// Uniform in `lo..=hi` for small ranges.
    fn range_u8(&mut self, lo: u8, hi: u8) -> u8 {
        let span = (hi - lo) as u32 + 1;
        lo + (self.next_u32() % span) as u8
    }
}

fn from_bytes(b: [u8; 4]) -> c_int {
    i32::from_ne_bytes(b) as c_int
}

/// Shared randomized corpus for CONFIGS.md rows 16, 21 and 22 (fixed seed).
fn random_full_range_corpus() -> Vec<c_int> {
    let mut rng = Rng::new(16);
    (0..8192).map(|_| rng.next_u32() as c_int).collect()
}

// ===========================================================================
// Phase B — valid-path differential tests, one per CONFIGS.md row
// ===========================================================================
mod phase_b {
    use super::*;

    pub fn cfg_row01_all_zero_bytes() {
        assert_same(0);
    }

    pub fn cfg_row02_all_high_bytes() {
        assert_same(-1); // 0xffffffff
    }

    pub fn cfg_row03_int_max() {
        assert_same(i32::MAX);
    }

    pub fn cfg_row04_int_min() {
        assert_same(i32::MIN);
    }

    pub fn cfg_row05_single_low_nibble_byte_each_position() {
        let mut xs = Vec::new();
        for pos in 0..4u32 {
            for v in 1u32..=0x0f {
                xs.push((v << (8 * pos)) as c_int);
            }
        }
        assert_all_same(&xs);
    }

    pub fn cfg_row06_single_mid_byte_each_position() {
        let mut rng = Rng::new(6);
        let mut xs = Vec::new();
        for pos in 0..4u32 {
            // class edges plus randomized interior values
            for v in [0x10u32, 0x11, 0x7e, 0x7f] {
                xs.push((v << (8 * pos)) as c_int);
            }
            for _ in 0..64 {
                let v = rng.range_u8(0x10, 0x7f) as u32;
                xs.push((v << (8 * pos)) as c_int);
            }
        }
        assert_all_same(&xs);
    }

    pub fn cfg_row07_single_high_byte_each_position() {
        let mut rng = Rng::new(7);
        let mut xs = Vec::new();
        for pos in 0..4u32 {
            for v in [0x80u32, 0x81, 0xfe, 0xff] {
                xs.push((v << (8 * pos)) as c_int);
            }
            for _ in 0..64 {
                let v = rng.range_u8(0x80, 0xff) as u32;
                xs.push((v << (8 * pos)) as c_int);
            }
        }
        assert_all_same(&xs);
    }

    pub fn cfg_row08_all_four_classes_every_permutation() {
        // one representative of each %02x value class
        let classes: [u8; 4] = [0x00, 0x0a, 0x5f, 0xc3];
        let mut xs = Vec::new();
        for a in 0..4 {
            for b in 0..4 {
                for c in 0..4 {
                    for d in 0..4 {
                        if a == b || a == c || a == d || b == c || b == d || c == d {
                            continue; // permutations only
                        }
                        xs.push(from_bytes([classes[a], classes[b], classes[c], classes[d]]));
                    }
                }
            }
        }
        assert_eq!(xs.len(), 24, "expected all 24 permutations");
        assert_all_same(&xs);
    }

    pub fn cfg_row09_all_bytes_low_nibble() {
        let mut rng = Rng::new(9);
        let mut xs = vec![from_bytes([0x01, 0x01, 0x01, 0x01]), from_bytes([0x0f; 4])];
        for _ in 0..256 {
            xs.push(from_bytes([
                rng.range_u8(0x01, 0x0f),
                rng.range_u8(0x01, 0x0f),
                rng.range_u8(0x01, 0x0f),
                rng.range_u8(0x01, 0x0f),
            ]));
        }
        assert_all_same(&xs);
    }

    pub fn cfg_row10_all_bytes_high() {
        let mut rng = Rng::new(10);
        let mut xs = vec![from_bytes([0x80; 4]), from_bytes([0xff; 4])];
        for _ in 0..256 {
            xs.push(from_bytes([
                rng.range_u8(0x80, 0xff),
                rng.range_u8(0x80, 0xff),
                rng.range_u8(0x80, 0xff),
                rng.range_u8(0x80, 0xff),
            ]));
        }
        assert_all_same(&xs);
    }

    pub fn cfg_row11_exhaustive_byte0() {
        // Every possible byte value in position 0: covers the 0x0f/0x10 and
        // 0x7f/0x80 class edges exhaustively, with a fixed non-trivial high half.
        let xs: Vec<c_int> = (0u32..=0xff)
            .map(|v| (0x5a_c3_10_00u32 | v) as c_int)
            .collect();
        assert_all_same(&xs);
        // and again with the rest of the word zeroed
        let xs: Vec<c_int> = (0u32..=0xff).map(|v| v as c_int).collect();
        assert_all_same(&xs);
    }

    pub fn cfg_row12_single_bit_set() {
        let xs: Vec<c_int> = (0..32).map(|k| (1u32 << k) as c_int).collect();
        assert_all_same(&xs);
    }

    pub fn cfg_row13_bit_masks() {
        let mut xs = Vec::new();
        for k in 0..=32u32 {
            let m: u32 = if k == 32 { u32::MAX } else { (1u32 << k) - 1 };
            xs.push(m as c_int);
            xs.push(!m as c_int);
        }
        assert_all_same(&xs);
    }

    pub fn cfg_row14_random_negative() {
        let mut rng = Rng::new(14);
        let xs: Vec<c_int> = (0..2048)
            .map(|_| (rng.next_u32() | 0x8000_0000) as c_int)
            .collect();
        assert!(xs.iter().all(|&x| x < 0));
        assert_all_same(&xs);
    }

    pub fn cfg_row15_random_positive() {
        let mut rng = Rng::new(15);
        let xs: Vec<c_int> = (0..2048)
            .map(|_| ((rng.next_u32() & 0x7fff_ffff) | 1) as c_int)
            .collect();
        assert!(xs.iter().all(|&x| x > 0));
        assert_all_same(&xs);
    }

    pub fn cfg_row16_random_full_range() {
        assert_all_same(&random_full_range_corpus());
    }

    pub fn cfg_row17_dense_small_magnitude() {
        let xs: Vec<c_int> = (-1024..=1024).collect();
        assert_all_same(&xs);
    }

    pub fn cfg_row18_output_shape_invariant() {
        let mut rng = Rng::new(18);
        for _ in 0..512 {
            let x = rng.next_u32() as c_int;
            let c = run_one(Impl::C, x);
            let r = run_one(Impl::RustRelease, x);
            assert_eq!(c, r, "shape row: divergence for driver({x})");
            // print_hex is reached with len == sizeof(int) == 4 -> 8 hex + '\n'
            assert_eq!(c.len(), 9, "C output for driver({x}) was \"{}\"", show(&c));
            assert_eq!(*c.last().unwrap(), b'\n');
            assert!(
                c[..8]
                    .iter()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(b)),
                "non lowercase-hex byte in \"{}\"",
                show(&c)
            );
        }
    }

    pub fn cfg_row19_many_calls_one_stream() {
        let mut rng = Rng::new(19);
        let xs: Vec<c_int> = (0..256).map(|_| rng.next_u32() as c_int).collect();
        let c = run_many(Impl::C, &xs);
        let r = run_many(Impl::RustRelease, &xs);
        assert_eq!(c.len(), 9 * xs.len(), "unexpected C stream length");
        assert_eq!(
            c,
            r,
            "multi-call stream divergence\n  C   : \"{}\"\n  Rust: \"{}\"",
            show(&c),
            show(&r)
        );
    }

    pub fn cfg_row20_interleaved_same_stream() {
        let mut rng = Rng::new(20);
        let xs: Vec<c_int> = (0..128).map(|_| rng.next_u32() as c_int).collect();

        let cf = driver_of(Impl::C);
        let rf = driver_of(Impl::RustRelease);

        // C and Rust alternating in ONE stdout window.
        let interleaved = capture(&mut || {
            for (i, &x) in xs.iter().enumerate() {
                if i % 2 == 0 {
                    unsafe { cf(x) }
                } else {
                    unsafe { rf(x) }
                }
            }
        });
        // Reference: the same sequence produced entirely by C.
        let all_c = run_many(Impl::C, &xs);
        assert_eq!(
            interleaved,
            all_c,
            "interleaving C and Rust on the shared FILE *stdout diverges from all-C\n  mixed: \"{}\"\n  all C: \"{}\"",
            show(&interleaved),
            show(&all_c)
        );

        // And the mirror: Rust first in each pair.
        let interleaved2 = capture(&mut || {
            for (i, &x) in xs.iter().enumerate() {
                if i % 2 == 0 {
                    unsafe { rf(x) }
                } else {
                    unsafe { cf(x) }
                }
            }
        });
        assert_eq!(interleaved2, all_c);
    }

    pub fn cfg_row21_debug_profile_artifact() {
        assert_all_same_as_c(Impl::RustDebug, &random_full_range_corpus());
    }

    pub fn cfg_row22_release_profile_artifact() {
        assert_all_same_as_c(Impl::RustRelease, &random_full_range_corpus());
    }
}

// ===========================================================================
// Phase C — error-path differential tests, one per ERRORS.md row
// ===========================================================================
//
// `driver` returns `void` and performs no validation whatsoever (see ERRORS.md:
// the exhaustive grep for `return`/`assert`/`NULL`/range checks matches nothing
// in the C source). Each row therefore asserts the C's ACTUAL behaviour for that
// boundary input — that it does NOT reject — and that Rust reaches the identical
// non-rejecting result byte-for-byte, pinned to an exact expected string rather
// than merely "both did something".
mod phase_c {
    use super::*;

    #[track_caller]
    fn assert_exact(x: c_int, expected: &str) {
        let c = run_one(Impl::C, x);
        let r = run_one(Impl::RustRelease, x);
        assert_eq!(
            c,
            expected.as_bytes(),
            "C sentinel changed for driver({x}): got \"{}\"",
            show(&c)
        );
        assert_eq!(
            c,
            r,
            "error-path divergence for driver({x})\n  C   : \"{}\"\n  Rust: \"{}\"",
            show(&c),
            show(&r)
        );
    }

    /// Row 1 — `x = INT_MIN`, one step past the negative end of the range.
    pub fn err_row1_int_min() {
        assert_exact(i32::MIN, "00000080\n");
    }

    /// Row 2 — `x = INT_MAX`, one step past the positive end of the range.
    pub fn err_row2_int_max() {
        assert_exact(i32::MAX, "ffffff7f\n");
    }

    /// Row 3 — `x = 0`, the "zero length / empty" analogue of the sole argument.
    pub fn err_row3_zero() {
        assert_exact(0, "00000000\n");
    }

    /// Row 4 — all bits set. Guards the `(unsigned char *)` cast: a signed
    /// `char` would make `%02x` print `ffffffff` per byte (32 chars total).
    pub fn err_row4_all_bits_set() {
        assert_exact(-1, "ffffffff\n");
    }

    /// Row 5 — out-of-range enum-like values crossing the FFI boundary. A C
    /// `enum` parameter accepts any `int`, so a value with no valid variant is a
    /// real input; here the `int` slot is fed unsigned and over-wide values.
    pub fn err_row5_out_of_range_enum_like() {
        // unsigned value with the sign bit set
        assert_exact(0xFFFF_FFFFu32 as c_int, "ffffffff\n");
        assert_exact(0x8000_0000u32 as c_int, "00000080\n");
        // 64-bit values truncated into the 32-bit `int` slot
        for wide in [
            0x1_0000_0000u64,
            0x1_0000_00FFu64,
            0xFFFF_FFFF_FFFF_FFFFu64,
            0xDEAD_BEEF_CAFE_BABEu64,
        ] {
            let x = wide as u32 as c_int;
            let c = run_one(Impl::C, x);
            let r = run_one(Impl::RustRelease, x);
            assert_eq!(c, r, "divergence for truncated wide value {wide:#018x}");
            assert_eq!(c.len(), 9);
        }
        // enum-style ordinals just past a plausible variant range, both signs
        for x in [
            -2i32,
            -1,
            0,
            1,
            2,
            3,
            4,
            5,
            1000,
            -1000,
            i32::MIN + 1,
            i32::MAX - 1,
        ] {
            assert_same(x);
        }
    }

    /// Row 6 — `print_hex` (`static`; the `len <= 0` / null-`p` conditions) is
    /// unreachable from the public ABI. Proven by symbol absence in BOTH shared
    /// objects: neither exports it, so no external caller can trigger it.
    pub fn err_row6_print_hex_not_reachable() {
        for which in [Impl::C, Impl::RustRelease, Impl::RustDebug] {
            let lib = library(which);
            let sym: Result<Symbol<'_, DriverFn>, _> = unsafe { lib.get(b"print_hex\0") };
            assert!(
                sym.is_err(),
                "{which:?} unexpectedly exports the static helper `print_hex`"
            );
        }
    }
}

// ===========================================================================
// Phase D — symbol parity, asserted from inside the test suite
// ===========================================================================
mod phase_d {
    use super::*;

    /// Every symbol the C `.so` exports must be resolvable in the Rust `.so`,
    /// and actually callable through it (this is what exercises the
    /// `#[no_mangle] extern "C"` export wrapper rather than just its name).
    pub fn symbol_parity_driver_resolves_in_all_artifacts() {
        for which in [Impl::C, Impl::RustRelease, Impl::RustDebug] {
            let lib = library(which);
            let sym: Symbol<'_, DriverFn> = unsafe { lib.get(b"driver\0") }
                .unwrap_or_else(|e| panic!("{which:?} does not export `driver`: {e}"));
            let f = *sym;
            let out = capture(&mut || unsafe { f(0x1234_5678) });
            assert_eq!(
                out, b"78563412\n",
                "{which:?}'s exported `driver` is not callable as expected"
            );
        }
    }

    fn defined(path: &std::path::Path) -> Vec<String> {
        let out = std::process::Command::new("nm")
            .args(["-D", "--defined-only", "--format=posix"])
            .arg(path)
            .output();
        let out = match out {
            Ok(o) if o.status.success() => o,
            _ => return Vec::new(), // nm unavailable: handled by the caller
        };
        let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|l| l.split_whitespace().next().map(str::to_owned))
            .collect();
        v.sort();
        v.dedup();
        v
    }

    /// `nm -D --defined-only` parity, computed live so the artifacts cannot drift.
    pub fn nm_defined_symbol_sets_match() {
        let c = defined(&so_path(Impl::C));
        if c.is_empty() {
            eprintln!("  (note) `nm` unavailable or produced no output; skipping nm diff");
            return;
        }
        for rust in [Impl::RustRelease, Impl::RustDebug] {
            let r = defined(&so_path(rust));
            let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
            assert!(
                missing.is_empty(),
                "{rust:?} is missing C-exported symbols: {missing:?}"
            );
        }
        assert_eq!(c, vec!["driver".to_string()], "C export set changed");
    }
}

// ===========================================================================
// Sequential runner (harness = false)
// ===========================================================================

fn cases() -> Vec<(&'static str, fn())> {
    vec![
        // Phase D first: behaviour tests are meaningless if the ABI is missing.
        (
            "phase_d::symbol_parity_driver_resolves_in_all_artifacts",
            phase_d::symbol_parity_driver_resolves_in_all_artifacts as fn(),
        ),
        (
            "phase_d::nm_defined_symbol_sets_match",
            phase_d::nm_defined_symbol_sets_match as fn(),
        ),
        // Phase B — CONFIGS.md rows 1..22
        (
            "phase_b::cfg_row01_all_zero_bytes",
            phase_b::cfg_row01_all_zero_bytes as fn(),
        ),
        (
            "phase_b::cfg_row02_all_high_bytes",
            phase_b::cfg_row02_all_high_bytes as fn(),
        ),
        ("phase_b::cfg_row03_int_max", phase_b::cfg_row03_int_max as fn()),
        ("phase_b::cfg_row04_int_min", phase_b::cfg_row04_int_min as fn()),
        (
            "phase_b::cfg_row05_single_low_nibble_byte_each_position",
            phase_b::cfg_row05_single_low_nibble_byte_each_position as fn(),
        ),
        (
            "phase_b::cfg_row06_single_mid_byte_each_position",
            phase_b::cfg_row06_single_mid_byte_each_position as fn(),
        ),
        (
            "phase_b::cfg_row07_single_high_byte_each_position",
            phase_b::cfg_row07_single_high_byte_each_position as fn(),
        ),
        (
            "phase_b::cfg_row08_all_four_classes_every_permutation",
            phase_b::cfg_row08_all_four_classes_every_permutation as fn(),
        ),
        (
            "phase_b::cfg_row09_all_bytes_low_nibble",
            phase_b::cfg_row09_all_bytes_low_nibble as fn(),
        ),
        (
            "phase_b::cfg_row10_all_bytes_high",
            phase_b::cfg_row10_all_bytes_high as fn(),
        ),
        (
            "phase_b::cfg_row11_exhaustive_byte0",
            phase_b::cfg_row11_exhaustive_byte0 as fn(),
        ),
        (
            "phase_b::cfg_row12_single_bit_set",
            phase_b::cfg_row12_single_bit_set as fn(),
        ),
        ("phase_b::cfg_row13_bit_masks", phase_b::cfg_row13_bit_masks as fn()),
        (
            "phase_b::cfg_row14_random_negative",
            phase_b::cfg_row14_random_negative as fn(),
        ),
        (
            "phase_b::cfg_row15_random_positive",
            phase_b::cfg_row15_random_positive as fn(),
        ),
        (
            "phase_b::cfg_row16_random_full_range",
            phase_b::cfg_row16_random_full_range as fn(),
        ),
        (
            "phase_b::cfg_row17_dense_small_magnitude",
            phase_b::cfg_row17_dense_small_magnitude as fn(),
        ),
        (
            "phase_b::cfg_row18_output_shape_invariant",
            phase_b::cfg_row18_output_shape_invariant as fn(),
        ),
        (
            "phase_b::cfg_row19_many_calls_one_stream",
            phase_b::cfg_row19_many_calls_one_stream as fn(),
        ),
        (
            "phase_b::cfg_row20_interleaved_same_stream",
            phase_b::cfg_row20_interleaved_same_stream as fn(),
        ),
        (
            "phase_b::cfg_row21_debug_profile_artifact",
            phase_b::cfg_row21_debug_profile_artifact as fn(),
        ),
        (
            "phase_b::cfg_row22_release_profile_artifact",
            phase_b::cfg_row22_release_profile_artifact as fn(),
        ),
        // Phase C — ERRORS.md rows 1..6
        ("phase_c::err_row1_int_min", phase_c::err_row1_int_min as fn()),
        ("phase_c::err_row2_int_max", phase_c::err_row2_int_max as fn()),
        ("phase_c::err_row3_zero", phase_c::err_row3_zero as fn()),
        (
            "phase_c::err_row4_all_bits_set",
            phase_c::err_row4_all_bits_set as fn(),
        ),
        (
            "phase_c::err_row5_out_of_range_enum_like",
            phase_c::err_row5_out_of_range_enum_like as fn(),
        ),
        (
            "phase_c::err_row6_print_hex_not_reachable",
            phase_c::err_row6_print_hex_not_reachable as fn(),
        ),
    ]
}

fn main() {
    // Optional substring filter, mirroring `cargo test -- <filter>`.
    let filter: Option<String> = std::env::args().skip(1).find(|a| !a.starts_with("--"));

    let selected: Vec<(&'static str, fn())> = cases()
        .into_iter()
        .filter(|(n, _)| match filter.as_deref() {
            Some(f) => n.contains(f),
            None => true,
        })
        .collect();

    println!("\nrunning {} differential cases", selected.len());
    let mut failed: Vec<&str> = Vec::new();
    for (name, f) in &selected {
        print!("test {name} ... ");
        let _ = std::io::stdout().flush();
        match catch_unwind(AssertUnwindSafe(f)) {
            Ok(()) => println!("ok"),
            Err(_) => {
                println!("FAILED");
                failed.push(name);
            }
        }
        let _ = std::io::stdout().flush();
    }

    println!();
    if failed.is_empty() {
        println!("test result: ok. {} passed; 0 failed", selected.len());
    } else {
        println!("failures:");
        for f in &failed {
            println!("    {f}");
        }
        println!(
            "\ntest result: FAILED. {} passed; {} failed",
            selected.len() - failed.len(),
            failed.len()
        );
        std::process::exit(1);
    }
}
