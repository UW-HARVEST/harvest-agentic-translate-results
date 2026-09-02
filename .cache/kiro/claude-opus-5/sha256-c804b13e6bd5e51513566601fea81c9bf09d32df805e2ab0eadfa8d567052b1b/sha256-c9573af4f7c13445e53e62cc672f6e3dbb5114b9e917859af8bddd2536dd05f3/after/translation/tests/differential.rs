//! Differential tests: load the C `.so` and the Rust `.so` through
//! `libloading` and compare `call_predict` byte-for-byte.
//!
//! Nothing in this file calls a Rust function directly; every Rust invocation
//! goes through a `libloading` symbol lookup on the built cdylib, so the
//! `#[unsafe(no_mangle)] extern "C"` export wrapper is under test too.
//!
//! Layout of the checks:
//!   * `configs_*` — Phase B, one test per row of `CONFIGS.md`.
//!   * `errors_*`  — Phase C, one test per row of `ERRORS.md`.
//!   * `symbols_*` — Phase D, `nm -D` parity.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use libloading::{Library, Symbol};

type CallPredict = unsafe extern "C" fn(std::ffi::c_int) -> std::ffi::c_int;

// ---------------------------------------------------------------------------
// locating and loading the two shared objects
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    // translation/ -> parent is the working directory holding c_src/ and translation/
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// The C shared library, built by CMake. Its name is derived from the parent
/// directory name by `c_src/CMakeLists.txt`, so it is discovered rather than
/// hard-coded.
fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    let build = workspace_root().join("c_src/build");
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("so") {
                found.push(p);
            }
        }
    }
    found.sort();
    assert_eq!(
        found.len(),
        1,
        "expected exactly one C .so in {}, found {:?}. Build it with: \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        build.display(),
        found
    );
    found.pop().unwrap()
}

/// Every Rust cdylib artifact we can find (debug and release). Both are tested
/// when present: optimisation level can in principle affect the function-pointer
/// identity comparisons `call_predict` performs, so both are worth covering.
fn rust_so_paths() -> Vec<PathBuf> {
    if let Ok(p) = std::env::var("RUST_SO") {
        return vec![PathBuf::from(p)];
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let name = "libcall_predict_lib.so";
    let mut v = Vec::new();
    for profile in ["release", "debug"] {
        let p = root.join("target").join(profile).join(name);
        if p.is_file() {
            v.push(p);
        }
    }
    assert!(
        !v.is_empty(),
        "no Rust cdylib found under {}/target/{{release,debug}}/{name}; \
         run `cargo build --release` first",
        root.display()
    );
    v
}

struct Loaded {
    _libs: Vec<Library>,
    /// (label, function pointer)
    rust: Vec<(String, CallPredict)>,
    c: CallPredict,
    c_label: String,
}

fn loaded() -> &'static Loaded {
    static CELL: OnceLock<Loaded> = OnceLock::new();
    CELL.get_or_init(|| {
        let mut libs = Vec::new();

        let cp = c_so_path();
        let clib = unsafe { Library::new(&cp) }
            .unwrap_or_else(|e| panic!("failed to load C .so {}: {e}", cp.display()));
        let c: CallPredict = {
            let s: Symbol<CallPredict> = unsafe { clib.get(b"call_predict\0") }
                .expect("C .so does not export `call_predict`");
            *s
        };
        let c_label = cp.display().to_string();
        libs.push(clib);

        let mut rust = Vec::new();
        for rp in rust_so_paths() {
            let rlib = unsafe { Library::new(&rp) }
                .unwrap_or_else(|e| panic!("failed to load Rust .so {}: {e}", rp.display()));
            let f: CallPredict = {
                let s: Symbol<CallPredict> = unsafe { rlib.get(b"call_predict\0") }
                    .unwrap_or_else(|e| {
                        panic!("Rust .so {} does not export `call_predict`: {e}", rp.display())
                    });
                *s
            };
            rust.push((rp.display().to_string(), f));
            libs.push(rlib);
        }

        Loaded { _libs: libs, rust, c, c_label }
    })
}

// ---------------------------------------------------------------------------
// the differential assertion
// ---------------------------------------------------------------------------

/// Call `call_predict(pfcn)` in the C `.so` and in every Rust `.so`, and require
/// the returned `int`s to be identical both as integers and as raw byte images.
#[track_caller]
fn diff(pfcn: i32) {
    let l = loaded();
    let cv = unsafe { (l.c)(pfcn) };
    for (label, f) in &l.rust {
        let rv = unsafe { f(pfcn) };
        assert_eq!(
            cv.to_le_bytes(),
            rv.to_le_bytes(),
            "byte mismatch for pfcn={pfcn}: C({}) -> {cv} {:02x?} vs Rust({label}) -> {rv} {:02x?}",
            l.c_label,
            cv.to_le_bytes(),
            rv.to_le_bytes()
        );
        assert_eq!(cv, rv, "value mismatch for pfcn={pfcn} (Rust .so {label})");
    }
}

#[track_caller]
fn diff_all(values: impl IntoIterator<Item = i32>) {
    for v in values {
        diff(v);
    }
}

/// Deterministic PRNG (SplitMix64) so every randomised row is reproducible from
/// its fixed seed without pulling in a dependency.
struct Rng(u64);
impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed)
    }
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn next_i32(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }
    /// Uniform in `lo..=hi` (inclusive), for `lo <= hi`.
    fn range(&mut self, lo: i32, hi: i32) -> i32 {
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
}

const RANDOM_DRAWS: usize = 4000;

// ---------------------------------------------------------------------------
// Phase B — CONFIGS.md rows 1..20
// ---------------------------------------------------------------------------

// Rows 1..12: each in-band value selects a distinct `_Pfn*` helper address.
// Each is hammered with many repeated calls interleaved with random neighbours
// so the row is not "one hand-picked scalar".
#[track_caller]
fn in_band_row(pfcn: i32, seed: u64) {
    diff(pfcn);
    let mut rng = Rng::new(seed);
    for _ in 0..RANDOM_DRAWS {
        // the row's own value, then a random other value, then the row again:
        // catches any state left behind by a previous call.
        diff(pfcn);
        diff(rng.next_i32());
        diff(pfcn);
    }
}

#[test]
fn configs_row01_pfcn_0() {
    in_band_row(0, 0x0000_0001);
}
#[test]
fn configs_row02_pfcn_1() {
    in_band_row(1, 0x0000_0002);
}
#[test]
fn configs_row03_pfcn_2() {
    in_band_row(2, 0x0000_0003);
}
#[test]
fn configs_row04_pfcn_3() {
    in_band_row(3, 0x0000_0004);
}
#[test]
fn configs_row05_pfcn_4() {
    in_band_row(4, 0x0000_0005);
}
#[test]
fn configs_row06_pfcn_5() {
    in_band_row(5, 0x0000_0006);
}
#[test]
fn configs_row07_pfcn_6() {
    in_band_row(6, 0x0000_0007);
}
#[test]
fn configs_row08_pfcn_7() {
    in_band_row(7, 0x0000_0008);
}
#[test]
fn configs_row09_pfcn_8() {
    in_band_row(8, 0x0000_0009);
}
#[test]
fn configs_row10_pfcn_9() {
    in_band_row(9, 0x0000_000A);
}
#[test]
fn configs_row11_pfcn_10() {
    in_band_row(10, 0x0000_000B);
}
#[test]
fn configs_row12_pfcn_11() {
    in_band_row(11, 0x0000_000C);
}

/// Row 13 — the `12..=15` band: recognised by `BTAC1C2_PredictSample`'s switch
/// but *not* by `call_predict`'s, so it must still report `0`.
#[test]
fn configs_row13_band_12_to_15() {
    diff_all(12..=15);
    // and each one repeatedly, in both orders
    for _ in 0..500 {
        diff_all(12..=15);
        diff_all((12..=15).rev());
    }
}

/// Row 14 — randomised positive out-of-band `16..=1023`.
#[test]
fn configs_row14_random_positive_out_of_band() {
    let mut rng = Rng::new(0xC0FF_EE14);
    for _ in 0..RANDOM_DRAWS {
        diff(rng.range(16, 1023));
    }
}

/// Row 15 — randomised negative values.
#[test]
fn configs_row15_random_negative() {
    let mut rng = Rng::new(0xC0FF_EE15);
    for _ in 0..RANDOM_DRAWS {
        diff(rng.range(-1_000_000, -1));
    }
}

/// Row 16 — randomised over the full `i32` domain.
#[test]
fn configs_row16_random_full_i32() {
    let mut rng = Rng::new(0xC0FF_EE16);
    for _ in 0..(RANDOM_DRAWS * 8) {
        diff(rng.next_i32());
    }
}

/// Row 17 — signed boundaries.
#[test]
fn configs_row17_signed_boundaries() {
    diff_all([
        i32::MIN,
        i32::MIN + 1,
        i32::MIN + 2,
        -2,
        -1,
        0,
        1,
        2,
        i32::MAX - 2,
        i32::MAX - 1,
        i32::MAX,
    ]);
}

/// Row 18 — exhaustive sweep of the small-value neighbourhood. This covers
/// every distinguished band and both edges of each, with no sampling gaps.
#[test]
fn configs_row18_exhaustive_small_neighbourhood() {
    diff_all(-4096..=4096);
}

/// Row 19 — repeated / interleaved invocation order proves statelessness.
#[test]
fn configs_row19_interleaved_order_statelessness() {
    let sequence: Vec<i32> = vec![
        0, 0, 11, 11, 12, 0, -1, 0, 5, 5, i32::MAX, 5, 10, 11, 12, 13, 14, 15, 16, 15, 11, 0,
        i32::MIN, 0,
    ];
    for _ in 0..2000 {
        diff_all(sequence.iter().copied());
    }
    // ascending then descending over the whole interesting range
    diff_all(-20..=20);
    diff_all((-20..=20).rev());
}

/// Row 20 — concurrent invocation from several threads. Neither implementation
/// has shared mutable state, so results must be stable under contention.
#[test]
fn configs_row20_concurrent() {
    let l = loaded();
    let c = l.c;
    let rust: Vec<(String, CallPredict)> = l.rust.clone_pairs();
    let mut handles = Vec::new();
    for t in 0..8u64 {
        let rust = rust.clone();
        handles.push(std::thread::spawn(move || {
            let mut rng = Rng::new(0xABCD_0000 + t);
            for _ in 0..20_000 {
                let v = if t % 2 == 0 { rng.range(-32, 32) } else { rng.next_i32() };
                let cv = unsafe { c(v) };
                for (label, f) in &rust {
                    let rv = unsafe { f(v) };
                    assert_eq!(
                        cv.to_le_bytes(),
                        rv.to_le_bytes(),
                        "thread {t}: mismatch for pfcn={v}: C -> {cv}, Rust({label}) -> {rv}"
                    );
                }
            }
        }));
    }
    for h in handles {
        h.join().expect("worker thread panicked");
    }
}

/// Helper so row 20 can move the symbol list into threads. `CallPredict` is a
/// plain `extern "C" fn` pointer, which is `Send`; the `Library` handles stay
/// alive in the `OnceLock` for the whole process.
trait ClonePairs {
    fn clone_pairs(&self) -> Vec<(String, CallPredict)>;
}
impl ClonePairs for Vec<(String, CallPredict)> {
    fn clone_pairs(&self) -> Vec<(String, CallPredict)> {
        self.iter().map(|(s, f)| (s.clone(), *f)).collect()
    }
}

// ---------------------------------------------------------------------------
// Phase C — ERRORS.md rows 1..9 (rows 10..12 are unreachable via the ABI; see
// ERRORS.md for why, and `errors_row10_12_unreachable_by_construction` below
// for the structural check that stands in for them).
// ---------------------------------------------------------------------------

/// Every error row asserts the *same specific* result, not merely "both
/// rejected": `call_predict`'s rejection sentinel is the literal `0` it returns
/// when the `default:` label is taken.
#[track_caller]
fn expect_rejection(pfcn: i32) {
    let l = loaded();
    let cv = unsafe { (l.c)(pfcn) };
    assert_eq!(cv, 0, "C is expected to reject pfcn={pfcn} with 0, got {cv}");
    diff(pfcn);
}

/// Row 1 — `pfcn == -1`, immediately below the valid band.
#[test]
fn errors_row01_negative_one() {
    expect_rejection(-1);
}

/// Row 2 — `pfcn == 12`, one step past the top of `call_predict`'s band.
#[test]
fn errors_row02_one_past_top() {
    expect_rejection(12);
}

/// Row 3 — the `12..=15` band handled by the big switch but not by `call_predict`.
#[test]
fn errors_row03_partially_handled_band() {
    for v in 12..=15 {
        expect_rejection(v);
    }
}

/// Row 4 — `pfcn == 16`, past the widest band any switch in the file names.
#[test]
fn errors_row04_sixteen() {
    expect_rejection(16);
}

/// Row 5 — `INT_MIN`, the extreme out-of-range value across the FFI boundary.
/// A C `enum` parameter accepts any `int`; this is the value with no variant.
#[test]
fn errors_row05_int_min() {
    expect_rejection(i32::MIN);
}

/// Row 6 — `INT_MAX`.
#[test]
fn errors_row06_int_max() {
    expect_rejection(i32::MAX);
}

/// Row 7 — neighbours of both extremes.
#[test]
fn errors_row07_extreme_neighbours() {
    for v in [i32::MIN + 1, i32::MIN + 2, i32::MAX - 1, i32::MAX - 2] {
        expect_rejection(v);
    }
}

/// Row 8 — out-of-range values that alias a valid code in their low bits. A
/// translation that masked (`pfcn & 15`) instead of comparing would return `1`
/// here while the C returns `0`.
#[test]
fn errors_row08_low_bit_aliases() {
    let mut aliases: Vec<i32> = vec![
        0x1000_0000,
        0x0000_0100, // 256   -> low nibble 0
        0x0000_1000, // 4096  -> low nibble 0
        0x7FFF_FFF0,
        -4,  // 0xFFFF_FFFC, low nibble 12
        -16, // low nibble 0
        -1_048_576,
        1 << 20,
        (1 << 20) + 11,
        0x0002_000B,
    ];
    // systematically: every valid code v in 0..=11, offset by each power-of-two
    // multiple of 16 up to 2^30, both signs.
    for v in 0..=11i32 {
        let mut step: i64 = 16;
        while step <= (1i64 << 30) {
            aliases.push((step + v as i64) as i32);
            aliases.push((-step + v as i64) as i32);
            step *= 2;
        }
    }
    for v in aliases {
        expect_rejection(v);
    }
}

/// Row 9 — `BTAC1C2_GetPredictFunc`'s `default:` arm. It is `static`, so its
/// only observable consequence is that the pointer it returns matches none of
/// the `_Pfn*` helpers; that is exactly the `0` seen for every out-of-band
/// value. Verified over the whole exhaustive small neighbourhood plus randoms,
/// asserting BOTH that C returns 0 and that Rust returns the same.
#[test]
fn errors_row09_dispatcher_default_arm() {
    for v in -4096..=4096i32 {
        if (0..=11).contains(&v) {
            continue;
        }
        expect_rejection(v);
    }
    let mut rng = Rng::new(0xDEFA_0009);
    for _ in 0..RANDOM_DRAWS {
        let v = rng.next_i32();
        if (0..=11).contains(&v) {
            continue;
        }
        expect_rejection(v);
    }
}

/// Rows 10–12 — unreachable through the ABI, checked structurally: the C `.so`
/// exports nothing that can accept a `psamp`, an `idx` or a `ridx`, so no
/// caller can trigger those paths. If a future change exported such a symbol
/// this test fails and the rows must be given real differential tests.
#[test]
fn errors_row10_12_unreachable_by_construction() {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", "--format=posix"])
        .arg(c_so_path())
        .output()
        .expect("failed to run nm on the C .so");
    assert!(out.status.success(), "nm failed: {:?}", out);
    let text = String::from_utf8_lossy(&out.stdout);
    let names: Vec<&str> = text
        .lines()
        .filter_map(|l| l.split_whitespace().next())
        .filter(|n| !n.is_empty())
        .collect();
    assert_eq!(
        names,
        vec!["call_predict"],
        "the C .so's exported surface changed; ERRORS.md rows 10-12 were \
         discharged as unreachable on the basis that `call_predict` is the only \
         export and it takes a single `int`. Re-derive ERRORS.md."
    );
}

/// The generic boundary sweep every C API deserves, independent of the table:
/// a dense walk of every value within 64 of each distinguished edge.
#[test]
fn errors_generic_boundary_sweep() {
    let edges = [i32::MIN, -1, 0, 11, 12, 15, 16, i32::MAX];
    for e in edges {
        for d in -64i64..=64 {
            let v = (e as i64).saturating_add(d);
            let v = v.clamp(i32::MIN as i64, i32::MAX as i64) as i32;
            diff(v);
        }
    }
}

// ---------------------------------------------------------------------------
// Phase D — symbol parity, asserted from inside the test suite
// ---------------------------------------------------------------------------

fn nm_defined(path: &Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", "--format=posix"])
        .arg(path)
        .output()
        .unwrap_or_else(|e| panic!("failed to run nm on {}: {e}", path.display()));
    assert!(out.status.success(), "nm failed on {}: {:?}", path.display(), out);
    let text = String::from_utf8_lossy(&out.stdout);
    let mut v: Vec<String> = text
        .lines()
        .filter_map(|l| l.split_whitespace().next())
        .filter(|n| !n.is_empty())
        .map(str::to_owned)
        .collect();
    v.sort();
    v.dedup();
    v
}

/// Every symbol the C `.so` exports must also be exported by every Rust `.so`.
#[test]
fn symbols_c_exports_are_all_present_in_rust() {
    let c = nm_defined(&c_so_path());
    assert!(!c.is_empty(), "C .so exports nothing; build is wrong");
    for rp in rust_so_paths() {
        let r = nm_defined(&rp);
        let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
        assert!(
            missing.is_empty(),
            "Rust .so {} is missing C symbols: {:?}",
            rp.display(),
            missing
        );
    }
}

/// And each of those symbols must be resolvable as a live `call_predict`-shaped
/// entry point through `dlsym`, not merely present in the table.
#[test]
fn symbols_are_callable_through_dlsym() {
    let c = nm_defined(&c_so_path());
    assert_eq!(c, vec!["call_predict".to_string()]);
    for rp in rust_so_paths() {
        let lib = unsafe { Library::new(&rp) }.expect("load rust .so");
        let s: Symbol<CallPredict> =
            unsafe { lib.get(b"call_predict\0") }.expect("dlsym call_predict");
        // exercise it so a table entry pointing at nothing cannot pass
        let _ = unsafe { s(0) };
    }
}

// ---------------------------------------------------------------------------
// Exhaustive closure of the entire input domain
// ---------------------------------------------------------------------------

/// `call_predict` takes a single `int` and is pure, so the input domain is
/// finite and small enough to enumerate completely: this walks **every one of
/// the 2^32 possible `pfcn` values** through both `.so`s and compares. Once this
/// passes there is no remaining untested input, which subsumes every sampled row
/// above. Parallelised across threads to stay well inside the time budget.
#[test]
fn exhaustive_entire_i32_domain() {
    let l = loaded();
    let c = l.c;
    let rust = l.rust.clone_pairs();

    const THREADS: i64 = 16;
    const LO: i64 = i32::MIN as i64;
    const HI: i64 = i32::MAX as i64;
    const TOTAL: i64 = HI - LO + 1;
    let chunk = TOTAL / THREADS;

    let mut handles = Vec::new();
    for t in 0..THREADS {
        let rust = rust.clone();
        let start = LO + t * chunk;
        let end = if t == THREADS - 1 { HI } else { LO + (t + 1) * chunk - 1 };
        handles.push(std::thread::spawn(move || {
            let mut v = start;
            while v <= end {
                let x = v as i32;
                let cv = unsafe { c(x) };
                for (label, f) in &rust {
                    let rv = unsafe { f(x) };
                    if cv != rv {
                        panic!(
                            "exhaustive mismatch at pfcn={x}: C -> {cv} {:02x?} vs \
                             Rust({label}) -> {rv} {:02x?}",
                            cv.to_le_bytes(),
                            rv.to_le_bytes()
                        );
                    }
                }
                v += 1;
            }
        }));
    }
    for h in handles {
        h.join().expect("exhaustive worker thread panicked");
    }
}

/// Diagnostic: report which shared objects the suite is actually comparing, and
/// require that at least one Rust artifact is present. Run with `--nocapture` to
/// see the paths.
#[test]
fn harness_reports_loaded_artifacts() {
    let l = loaded();
    println!("C   .so: {}", l.c_label);
    for (label, _) in &l.rust {
        println!("Rust .so: {label}");
    }
    println!("rust artifacts compared: {}", l.rust.len());
    assert!(!l.rust.is_empty());
}
