//! Differential test harness: loads BOTH the C `.so` and the Rust `.so` with
//! `libloading` and compares `encode_quant` results through the FFI boundary.
//!
//! Nothing in this file calls the Rust implementation directly — the Rust code
//! is only ever reached through `dlsym("encode_quant")` on the built cdylib,
//! exactly as an external C consumer would, so the `#[no_mangle]`/`extern "C"`
//! export wrapper is under test too.

use std::ffi::c_int;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use libloading::{Library, Symbol};

type EncodeQuant = unsafe extern "C" fn(c_int, c_int, c_int, c_int, c_int, c_int) -> c_int;

// ---------------------------------------------------------------------------
// Library discovery + loading
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `<workdir>/c_src`
fn c_src_dir() -> PathBuf {
    manifest_dir().parent().unwrap().join("c_src")
}

fn find_so_in(dir: &Path, wanted_stem_contains: Option<&str>) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    let mut hits: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.extension().map(|x| x == "so").unwrap_or(false)
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("lib"))
                    .unwrap_or(false)
                && match wanted_stem_contains {
                    None => true,
                    Some(w) => p
                        .file_name()
                        .and_then(|n| n.to_str())
                        .map(|n| n.contains(w))
                        .unwrap_or(false),
                }
        })
        .collect();
    hits.sort();
    hits.into_iter().next()
}

/// Path to the C shared object, building it with CMake if it is not there yet.
fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO_PATH") {
        return PathBuf::from(p);
    }
    let build_dir = c_src_dir().join("build");
    if let Some(p) = find_so_in(&build_dir, None) {
        return p;
    }
    // Not built yet: build it (read-only w.r.t. the C sources; only adds build/).
    std::fs::create_dir_all(&build_dir).expect("cannot create c_src/build");
    let cfg = Command::new("cmake")
        .current_dir(&build_dir)
        .args(["..", "-DCMAKE_POSITION_INDEPENDENT_CODE=ON"])
        .output()
        .expect("failed to run `cmake` — is it installed?");
    assert!(
        cfg.status.success(),
        "cmake configure failed:\n{}",
        String::from_utf8_lossy(&cfg.stderr)
    );
    let bld = Command::new("cmake")
        .current_dir(&build_dir)
        .args(["--build", "."])
        .output()
        .expect("failed to run `cmake --build`");
    assert!(
        bld.status.success(),
        "cmake build failed:\n{}",
        String::from_utf8_lossy(&bld.stderr)
    );
    find_so_in(&build_dir, None).unwrap_or_else(|| {
        panic!(
            "no lib*.so found in {} after building the C library",
            build_dir.display()
        )
    })
}

/// Path to the Rust cdylib. Overridable with `RUST_SO_PATH`.
///
/// IMPORTANT: `cargo test` does **not** necessarily build the cdylib, because
/// `crate-type = ["cdylib"]` produces nothing an integration test links
/// against. So the harness builds it explicitly (`cargo build`, and
/// `--release` too) before locating it, and then refuses to run against an
/// artifact older than the sources — otherwise a stale `.so` would make the
/// whole differential suite pass vacuously.
fn rust_so_path() -> PathBuf {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        if let Ok(p) = std::env::var("RUST_SO_PATH") {
            let p = PathBuf::from(p);
            assert_not_stale(&p);
            return p;
        }

        // current_exe = target/<profile>/deps/<test>-<hash>
        let exe = std::env::current_exe().expect("current_exe");
        let profile_dir = exe
            .parent()
            .and_then(|p| p.parent())
            .expect("target/<profile>")
            .to_path_buf();
        let profile = profile_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("debug")
            .to_string();

        // Force the cdylib for THIS profile to exist and be current.
        build_cdylib(&profile);

        let p = find_so_in(&profile_dir, Some("encode_quant_lib")).unwrap_or_else(|| {
            panic!(
                "libencode_quant_lib.so still absent from {} after `cargo build`",
                profile_dir.display()
            )
        });
        assert_not_stale(&p);
        p
    })
    .clone()
}

/// Runs `cargo build` (optionally `--release`) for the cdylib target.
fn build_cdylib(profile: &str) {
    let mut cmd = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()));
    cmd.current_dir(manifest_dir()).arg("build").arg("--lib");
    if profile == "release" {
        cmd.arg("--release");
    }
    // Avoid inheriting the test run's RUSTFLAGS-driven fingerprint churn.
    match cmd.output() {
        Ok(out) if out.status.success() => {}
        Ok(out) => panic!(
            "`cargo build --lib` ({profile}) failed:\n{}\n{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ),
        Err(e) => panic!("could not spawn cargo to build the cdylib: {e}"),
    }
}

/// Guards against comparing against an out-of-date `.so`.
fn assert_not_stale(so: &Path) {
    assert!(
        so.exists(),
        "Rust .so {} does not exist; run `cargo build` first",
        so.display()
    );
    let so_mtime = std::fs::metadata(so)
        .and_then(|m| m.modified())
        .expect("mtime of .so");
    let src = manifest_dir().join("src");
    let mut newest: Option<(std::time::SystemTime, PathBuf)> = None;
    let mut stack = vec![src];
    while let Some(dir) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in rd.filter_map(|e| e.ok()) {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().map(|x| x == "rs").unwrap_or(false) {
                if let Ok(t) = std::fs::metadata(&p).and_then(|m| m.modified()) {
                    if newest.as_ref().map(|(nt, _)| t > *nt).unwrap_or(true) {
                        newest = Some((t, p));
                    }
                }
            }
        }
    }
    if let Some((src_mtime, src_path)) = newest {
        assert!(
            so_mtime >= src_mtime,
            "STALE ARTIFACT: {} is older than {}.\n\
             Comparing against a stale .so would make this suite pass vacuously.\n\
             Rebuild with `cargo build --release` (and `cargo build`) first.",
            so.display(),
            src_path.display()
        );
    }
}

struct Both {
    _c_lib: Library,
    _rust_lib: Library,
    c: EncodeQuant,
    rust: EncodeQuant,
}

// SAFETY: the loaded functions are pure leaf functions taking/returning ints;
// no per-thread state and no unloading occurs (the Libraries live forever in a
// OnceLock), so sharing across the test threads is sound.
unsafe impl Send for Both {}
unsafe impl Sync for Both {}

fn libs() -> &'static Both {
    static LIBS: OnceLock<Both> = OnceLock::new();
    LIBS.get_or_init(|| {
        let c_path = c_so_path();
        let rust_path = rust_so_path();
        unsafe {
            let c_lib = Library::new(&c_path)
                .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", c_path.display()));
            let rust_lib = Library::new(&rust_path)
                .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", rust_path.display()));
            let c_sym: Symbol<EncodeQuant> = c_lib
                .get(b"encode_quant\0")
                .expect("C .so does not export `encode_quant`");
            let rust_sym: Symbol<EncodeQuant> = rust_lib
                .get(b"encode_quant\0")
                .expect("Rust .so does not export `encode_quant`");
            let c = *c_sym;
            let rust = *rust_sym;
            Both {
                _c_lib: c_lib,
                _rust_lib: rust_lib,
                c,
                rust,
            }
        }
    })
}

// ---------------------------------------------------------------------------
// The differential assertion
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Args {
    uni: i32,
    step: i32,
    pred: i32,
    tgt: i32,
    tgt2: i32,
    lsbit: i32,
}

/// Calls BOTH `.so`s through FFI and asserts byte-identical results.
#[track_caller]
fn diff(row: &str, a: Args) -> i32 {
    let l = libs();
    let c = unsafe { (l.c)(a.uni, a.step, a.pred, a.tgt, a.tgt2, a.lsbit) };
    let r = unsafe { (l.rust)(a.uni, a.step, a.pred, a.tgt, a.tgt2, a.lsbit) };
    assert_eq!(
        c, r,
        "\n[{row}] DIVERGENCE\n  encode_quant(uni={}, step={}, pred={}, tgt={}, tgt2={}, lsbit={})\n  \
         C    = {c} (0x{c:08x})\n  Rust = {r} (0x{r:08x})\n",
        a.uni, a.step, a.pred, a.tgt, a.tgt2, a.lsbit
    );
    c
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*), fixed seed per row for reproducibility
// ---------------------------------------------------------------------------

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        // Avoid the zero state; mix so nearby seeds diverge immediately.
        Rng(seed
            .wrapping_mul(0x9E37_79B9_7F4A_7C15)
            .wrapping_add(0xD1B5_4A32_D192_ED03)
            | 1)
    }
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn i32_full(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }
    /// Uniform in `lo..=hi` (inclusive), works across the full i32 range.
    fn range(&mut self, lo: i32, hi: i32) -> i32 {
        debug_assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
    /// A value biased toward interesting magnitudes / boundaries.
    fn spicy_i32(&mut self) -> i32 {
        match self.next_u64() % 8 {
            0 => 0,
            1 => self.range(-16, 16),
            2 => self.range(-1024, 1024),
            3 => i32::MAX,
            4 => i32::MIN,
            5 => i32::MAX - self.range(0, 64),
            6 => i32::MIN + self.range(0, 64),
            _ => self.i32_full(),
        }
    }
}

// ---------------------------------------------------------------------------
// Axis helpers (derived from CONFIGS.md)
// ---------------------------------------------------------------------------

/// The four `lsbit` modes the C dispatches on.
#[derive(Clone, Copy, Debug)]
enum LsMode {
    Zero,
    Four,
    Odd,
    Even,
}

impl LsMode {
    fn all() -> [LsMode; 4] {
        [LsMode::Zero, LsMode::Four, LsMode::Odd, LsMode::Even]
    }
    /// A random `lsbit` value that lands in this mode.
    fn sample(self, rng: &mut Rng) -> i32 {
        match self {
            LsMode::Zero => 0,
            LsMode::Four => 4,
            LsMode::Odd => loop {
                let v = rng.i32_full() | 1; // odd => never 0, never 4
                if v != 0 && v != 4 {
                    return v;
                }
            },
            LsMode::Even => loop {
                let v = rng.i32_full() & !1; // even
                if v != 0 && v != 4 {
                    return v;
                }
            },
        }
    }
}

/// The three candidate-clamp shapes, expressed as the required `uni & 7`.
#[derive(Clone, Copy, Debug)]
enum Clamp {
    Low,  // uni & 7 == 0  -> uni2 clamped
    High, // uni & 7 == 7  -> uni1 clamped
    Mid,  // uni & 7 in 1..=6 -> neither
}

impl Clamp {
    fn all() -> [Clamp; 3] {
        [Clamp::Low, Clamp::High, Clamp::Mid]
    }
    fn low3(self, rng: &mut Rng) -> i32 {
        match self {
            Clamp::Low => 0,
            Clamp::High => 7,
            Clamp::Mid => rng.range(1, 6),
        }
    }
}

/// Builds a `uni` with the requested `uni & 7` class and `uni & 8` bit, and
/// random bits above bit 3 (so both signs of `uni` occur).
fn make_uni(rng: &mut Rng, clamp: Clamp, bit3: bool) -> i32 {
    let high = rng.i32_full() & !0xF;
    let mut u = high | clamp.low3(rng);
    if bit3 {
        u |= 8;
    }
    u
}

// ---------------------------------------------------------------------------
// PHASE B — valid-path differential tests, one per CONFIGS.md row
// ---------------------------------------------------------------------------

const N: usize = 2000;

/// Rows 1..=24: the full `lsbit-mode x clamp-shape x uni&8` cross-product,
/// each with randomized `step`/`pred`/`tgt`/`tgt2`.
#[test]
fn cfg_rows_1_to_24_lsmode_x_clamp_x_bit3() {
    let mut row_no = 0;
    for mode in LsMode::all() {
        for bit3 in [false, true] {
            for clamp in Clamp::all() {
                // Row numbering in CONFIGS.md is L x K x S; the exact index is
                // cosmetic, the point is that all 24 combinations are covered.
                row_no += 1;
                let name = format!("cfg{row_no} {mode:?}/{clamp:?}/bit3={bit3}");
                let mut rng = Rng::new(0xC0FFEE_0000 + row_no as u64);
                for _ in 0..N {
                    let a = Args {
                        uni: make_uni(&mut rng, clamp, bit3),
                        step: rng.spicy_i32(),
                        pred: rng.spicy_i32(),
                        tgt: rng.spicy_i32(),
                        tgt2: rng.spicy_i32(),
                        lsbit: mode.sample(&mut rng),
                    };
                    // Sanity: the generator really produced the intended shape.
                    assert_eq!((a.uni & 8) != 0, bit3, "generator bit3 mismatch");
                    diff(&name, a);
                }
            }
        }
    }
    assert_eq!(row_no, 24, "expected exactly 24 grid rows");
}

/// Row 25: `step == 0`.
#[test]
fn cfg_row_25_step_zero() {
    let mut rng = Rng::new(25);
    for mode in LsMode::all() {
        for _ in 0..N {
            let a = Args {
                uni: rng.spicy_i32(),
                step: 0,
                pred: rng.spicy_i32(),
                tgt: rng.spicy_i32(),
                tgt2: rng.spicy_i32(),
                lsbit: mode.sample(&mut rng),
            };
            diff("cfg25 step=0", a);
        }
    }
}

/// Row 26: small positive `step` (nominal codec range) over `uni` 0..=15.
#[test]
fn cfg_row_26_step_small_positive() {
    let mut rng = Rng::new(26);
    for mode in LsMode::all() {
        for uni in 0..=15 {
            for _ in 0..200 {
                let a = Args {
                    uni,
                    step: rng.range(1, 1024),
                    pred: rng.range(-4096, 4096),
                    tgt: rng.range(-4096, 4096),
                    tgt2: rng.range(-4096, 4096),
                    lsbit: mode.sample(&mut rng),
                };
                diff("cfg26 step=1..1024", a);
            }
        }
    }
}

/// Row 27: small negative `step` (`/8` truncates toward zero).
#[test]
fn cfg_row_27_step_small_negative() {
    let mut rng = Rng::new(27);
    for mode in LsMode::all() {
        for uni in 0..=15 {
            for _ in 0..200 {
                let a = Args {
                    uni,
                    step: rng.range(-1024, -1),
                    pred: rng.range(-4096, 4096),
                    tgt: rng.range(-4096, 4096),
                    tgt2: rng.range(-4096, 4096),
                    lsbit: mode.sample(&mut rng),
                };
                diff("cfg27 step=-1024..-1", a);
            }
        }
    }
}

/// Row 28: `step` big enough that `(2*(uni&7)+1)*step` overflows `int`.
#[test]
fn cfg_row_28_step_multiply_overflow() {
    let threshold = i32::MAX / 15; // 143165576
    let mut rng = Rng::new(28);
    for mode in LsMode::all() {
        for uni in 0..=15 {
            for _ in 0..200 {
                let mag = rng.range(threshold + 1, i32::MAX);
                let step = if rng.next_u64() & 1 == 0 {
                    mag
                } else {
                    mag.wrapping_neg()
                };
                let a = Args {
                    uni,
                    step,
                    pred: rng.spicy_i32(),
                    tgt: rng.spicy_i32(),
                    tgt2: rng.spicy_i32(),
                    lsbit: mode.sample(&mut rng),
                };
                diff("cfg28 step overflow", a);
            }
        }
    }
}

/// Rows 29 + 30: `step == INT_MAX` and `step == INT_MIN`.
#[test]
fn cfg_rows_29_30_step_extremes() {
    let mut rng = Rng::new(2930);
    for step in [i32::MAX, i32::MIN] {
        for mode in LsMode::all() {
            for uni in 0..=15 {
                for _ in 0..200 {
                    let a = Args {
                        uni,
                        step,
                        pred: rng.spicy_i32(),
                        tgt: rng.spicy_i32(),
                        tgt2: rng.spicy_i32(),
                        lsbit: mode.sample(&mut rng),
                    };
                    diff("cfg29/30 step extreme", a);
                }
            }
        }
    }
}

/// Row 31: negative `uni` — arithmetic `>>1` / `>>2` inside the `lsbit == 4`
/// branch, plus the other modes for contrast.
#[test]
fn cfg_row_31_negative_uni() {
    let mut rng = Rng::new(31);
    for mode in LsMode::all() {
        for _ in 0..4 * N {
            // Always negative.
            let uni = rng.range(i32::MIN, -1);
            let a = Args {
                uni,
                step: rng.spicy_i32(),
                pred: rng.spicy_i32(),
                tgt: rng.spicy_i32(),
                tgt2: rng.spicy_i32(),
                lsbit: mode.sample(&mut rng),
            };
            assert!(a.uni < 0);
            diff("cfg31 uni<0", a);
        }
    }
    // Plus a dense negative sweep under lsbit == 4 specifically.
    let mut rng = Rng::new(3131);
    for uni in -64..0 {
        for _ in 0..200 {
            diff(
                "cfg31 uni<0 lsbit=4 sweep",
                Args {
                    uni,
                    step: rng.spicy_i32(),
                    pred: rng.spicy_i32(),
                    tgt: rng.spicy_i32(),
                    tgt2: rng.spicy_i32(),
                    lsbit: 4,
                },
            );
        }
    }
}

/// Row 32: `uni` at the `i32` boundaries where `uni + 1` / `uni - 1` wrap.
#[test]
fn cfg_row_32_uni_boundaries() {
    let mut rng = Rng::new(32);
    let unis = [
        i32::MIN,
        i32::MIN + 1,
        i32::MIN + 7,
        i32::MIN + 8,
        -9,
        -8,
        -1,
        0,
        1,
        7,
        8,
        15,
        16,
        i32::MAX - 8,
        i32::MAX - 7,
        i32::MAX - 1,
        i32::MAX,
    ];
    for &uni in &unis {
        for mode in LsMode::all() {
            for _ in 0..500 {
                let a = Args {
                    uni,
                    step: rng.spicy_i32(),
                    pred: rng.spicy_i32(),
                    tgt: rng.spicy_i32(),
                    tgt2: rng.spicy_i32(),
                    lsbit: mode.sample(&mut rng),
                };
                diff("cfg32 uni boundary", a);
            }
        }
    }
}

// --- winner-selection rows (33..37) ------------------------------------------
//
// These need inputs that actually land in each branch of
//     if (d1 < d0) uni = uni1;
//     if (d2 < d0) uni = uni2;
// so the harness classifies randomized samples by recomputing d0/d1/d2 and
// bucketing them. The classifier is used ONLY for bucketing; the pass/fail
// assertion is always the C-vs-Rust FFI comparison in `diff`.

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
#[repr(usize)]
enum Winner {
    D0 = 0,
    D1Only = 1,
    D2Only = 2,
    Both = 3,
}

/// Recomputes the three distances the way `lib.c` does, purely to categorise a
/// sample into a `Winner` bucket.
fn distances(a: Args) -> (i32, i32, i32) {
    let mut uni = a.uni;
    let mut uni1 = uni.wrapping_add(1);
    let mut uni2 = uni.wrapping_sub(1);
    if ((uni ^ uni1) & !7i32) != 0 {
        uni1 = uni;
    }
    if ((uni ^ uni2) & !7i32) != 0 {
        uni2 = uni;
    }
    if a.lsbit != 0 {
        if a.lsbit == 4 {
            uni &= !1;
            uni1 &= !1;
            uni2 &= !1;
            uni |= (uni >> 1) & (uni >> 2) & 1;
            uni1 |= (uni1 >> 1) & (uni1 >> 2) & 1;
            uni2 |= (uni2 >> 1) & (uni2 >> 2) & 1;
        } else if (a.lsbit & 1) != 0 {
            uni |= 1;
            uni1 |= 1;
            uni2 |= 1;
        } else {
            uni &= !1;
            uni1 &= !1;
            uni2 &= !1;
        }
    }
    let dist = |u: i32| -> i32 {
        let mut d = 2i32
            .wrapping_mul(u & 7)
            .wrapping_add(1)
            .wrapping_mul(a.step)
            / 8;
        if (u & 8) != 0 {
            d = d.wrapping_neg();
        }
        let p = a.pred.wrapping_add(d);
        let mut dd = a.tgt.wrapping_sub(p);
        dd ^= dd >> 31;
        let mut d3 = a.tgt2.wrapping_sub(p);
        d3 ^= d3 >> 31;
        dd.wrapping_add(d3 >> 5)
    };
    let (d0, d1, d2) = (dist(uni), dist(uni1), dist(uni2));
    (d0, d1, d2)
}

fn classify(a: Args) -> Winner {
    let (d0, d1, d2) = distances(a);
    match (d1 < d0, d2 < d0) {
        (false, false) => Winner::D0,
        (true, false) => Winner::D1Only,
        (false, true) => Winner::D2Only,
        (true, true) => Winner::Both,
    }
}

fn raw_d1_d2(a: Args) -> (i32, i32) {
    let (_, d1, d2) = distances(a);
    (d1, d2)
}

/// Rows 33-36: each of the four winner-selection outcomes, including the
/// `Both` case where the C returns `uni2` even when `d1 < d2`.
///
/// Rows 33-35 are reachable with ordinary values. Row 36 (`d1 < d0 && d2 < d0`)
/// is NOT: for non-wrapping inputs the distance is a convex function of the
/// candidate index (a sum of two V-shaped terms composed with the monotone
/// index -> prediction map), so the middle candidate can never be strictly
/// worse than both neighbours. It becomes reachable only once the `int`
/// arithmetic wraps, which needs `pred`/`tgt` clustered at one end of the range
/// and `tgt2` at the other. A dedicated generator targets that region
/// (measured hit rate ~4.7%); moderate-range sampling finds it 0 times in 40M.
#[test]
fn cfg_rows_33_to_36_winner_selection() {
    // --- rows 33/34/35: reachable with moderate values -------------------
    let mut rng = Rng::new(3336);
    let mut counts = [0usize; 4];
    let target = 1500usize;
    let mut attempts = 0usize;
    while counts[0..3].iter().any(|&c| c < target) && attempts < 4_000_000 {
        attempts += 1;
        let a = Args {
            uni: rng.range(-32, 47),
            step: rng.range(-512, 512),
            pred: rng.range(-2048, 2048),
            tgt: rng.range(-2048, 2048),
            tgt2: rng.range(-2048, 2048),
            lsbit: LsMode::all()[(rng.next_u64() % 4) as usize].sample(&mut rng),
        };
        let w = classify(a);
        let idx = w as usize;
        if counts[idx] >= target {
            continue;
        }
        counts[idx] += 1;
        diff(&format!("cfg33-35 winner={w:?}"), a);
    }
    for (i, name) in ["D0", "D1Only", "D2Only"].iter().enumerate() {
        assert!(
            counts[i] >= target,
            "winner bucket {name} under-covered: {} < {target} (attempts={attempts})",
            counts[i]
        );
    }

    // --- row 36: `Both`, only reachable through wraparound ----------------
    // Fixed witnesses first (fully deterministic regression anchors).
    const BOTH_WITNESSES: [(i32, i32, i32, i32, i32, i32); 6] = [
        (1835, 306303866, -2147483552, -2147483148, 2147483647, 0),
        (1087534220, 747926775, -2147480062, -2147482649, 2147479706, 0),
        (889941585, 1003632041, -2147481234, -2147483618, 2147483594, 0),
        (-1315198243, 1757618584, -2147483240, -2147480857, 2147480045, 0),
        (-403337210, 1080349316, -2147483148, -2147481748, 2147480122, 0),
        (-177901436, 761202552, -2147482903, -2147483213, 2147483031, 0),
    ];
    for &(uni, step, pred, tgt, tgt2, lsbit) in &BOTH_WITNESSES {
        let a = Args {
            uni,
            step,
            pred,
            tgt,
            tgt2,
            lsbit,
        };
        assert_eq!(
            classify(a),
            Winner::Both,
            "hardcoded witness {a:?} is no longer in the `Both` bucket"
        );
        diff("cfg36 Both witness", a);
    }

    // Then randomized search in the reachable region, both polarities.
    let mut rng = Rng::new(36_0036);
    let mut both = 0usize;
    let mut both_d1_lt_d2 = 0usize; // the quirk: uni2 wins even though d1 is better
    let mut both_d2_lt_d1 = 0usize;
    let both_target = 1500usize;
    let mut attempts = 0usize;
    while both < both_target && attempts < 2_000_000 {
        attempts += 1;
        let low_high = attempts % 2 == 0;
        let (pred, tgt, tgt2) = if low_high {
            (
                i32::MIN + rng.range(0, 1 << 12),
                i32::MIN + rng.range(0, 1 << 12),
                i32::MAX - rng.range(0, 1 << 12),
            )
        } else {
            (
                i32::MAX - rng.range(0, 1 << 12),
                i32::MAX - rng.range(0, 1 << 12),
                i32::MIN + rng.range(0, 1 << 12),
            )
        };
        let a = Args {
            uni: rng.i32_full(),
            step: rng.range(1 << 24, i32::MAX),
            pred,
            tgt,
            tgt2,
            lsbit: LsMode::all()[(rng.next_u64() % 4) as usize].sample(&mut rng),
        };
        if classify(a) != Winner::Both {
            continue;
        }
        both += 1;
        // Track which sub-case we hit, to prove the quirk is really exercised.
        let (d1, d2) = raw_d1_d2(a);
        if d1 < d2 {
            both_d1_lt_d2 += 1;
        } else if d2 < d1 {
            both_d2_lt_d1 += 1;
        }
        diff("cfg36 Both random", a);
    }
    assert!(
        both >= both_target,
        "`Both` bucket under-covered: {both} < {both_target} (attempts={attempts})"
    );
    assert!(
        both_d1_lt_d2 >= 100,
        "the quirk sub-case (d1 < d2 yet C returns uni2) needs coverage, got {both_d1_lt_d2}"
    );
    assert!(
        both_d2_lt_d1 >= 100,
        "the d2 < d1 sub-case of `Both` needs coverage, got {both_d2_lt_d1}"
    );
}

/// Row 37: exact ties (`d1 == d0` / `d2 == d0`), where the strict `<` keeps
/// the original `uni`. `step == 0` and clamped candidates guarantee ties.
#[test]
fn cfg_row_37_ties() {
    let mut rng = Rng::new(37);
    // step == 0 => all three distances identical => both comparisons false.
    for mode in LsMode::all() {
        for _ in 0..N {
            let a = Args {
                uni: rng.range(-64, 64),
                step: 0,
                pred: rng.spicy_i32(),
                tgt: rng.spicy_i32(),
                tgt2: rng.spicy_i32(),
                lsbit: mode.sample(&mut rng),
            };
            assert_eq!(classify(a), Winner::D0, "step=0 must tie");
            diff("cfg37 tie via step=0", a);
        }
    }
    // Clamped candidate => that candidate's distance equals d0 exactly.
    let mut rng = Rng::new(3737);
    for (clamp, _which) in [(Clamp::Low, "uni2"), (Clamp::High, "uni1")] {
        for _ in 0..N {
            let bit3 = rng.next_u64() & 1 == 0;
            let a = Args {
                uni: make_uni(&mut rng, clamp, bit3),
                step: rng.range(1, 4096),
                pred: rng.range(-4096, 4096),
                tgt: rng.range(-4096, 4096),
                tgt2: rng.range(-4096, 4096),
                lsbit: 0,
            };
            diff("cfg37 tie via clamp", a);
        }
    }
}

/// Row 38: `tgt2 == tgt`.
#[test]
fn cfg_row_38_tgt2_equals_tgt() {
    let mut rng = Rng::new(38);
    for mode in LsMode::all() {
        for _ in 0..N {
            let t = rng.spicy_i32();
            let a = Args {
                uni: rng.spicy_i32(),
                step: rng.spicy_i32(),
                pred: rng.spicy_i32(),
                tgt: t,
                tgt2: t,
                lsbit: mode.sample(&mut rng),
            };
            diff("cfg38 tgt2==tgt", a);
        }
    }
}

/// Row 39: `tgt2` far from `tgt`, so the `d3 >> 5` secondary term dominates.
#[test]
fn cfg_row_39_tgt2_far_from_tgt() {
    let mut rng = Rng::new(39);
    for mode in LsMode::all() {
        for _ in 0..N {
            let tgt = rng.range(-1024, 1024);
            // Offset by >> 5 worth of magnitude so d3>>5 outweighs d0.
            let far = rng.range(1 << 16, 1 << 24);
            let tgt2 = if rng.next_u64() & 1 == 0 {
                tgt.wrapping_add(far)
            } else {
                tgt.wrapping_sub(far)
            };
            let a = Args {
                uni: rng.range(-32, 47),
                step: rng.range(-4096, 4096),
                pred: rng.range(-4096, 4096),
                tgt,
                tgt2,
                lsbit: mode.sample(&mut rng),
            };
            diff("cfg39 tgt2 far", a);
        }
    }
}

/// Row 40: `pred`/`tgt`/`tgt2` at the `i32` extremes so every intermediate
/// (`pred+diff`, `tgt-p0`, `d ^ (d>>31)`, `d0 + (d3>>5)`) wraps.
#[test]
fn cfg_row_40_extreme_values_wraparound() {
    let extremes = [
        i32::MIN,
        i32::MIN + 1,
        i32::MIN / 2,
        -1,
        0,
        1,
        i32::MAX / 2,
        i32::MAX - 1,
        i32::MAX,
    ];
    let mut rng = Rng::new(40);
    for &pred in &extremes {
        for &tgt in &extremes {
            for &tgt2 in &extremes {
                for mode in LsMode::all() {
                    for _ in 0..6 {
                        let a = Args {
                            uni: rng.spicy_i32(),
                            step: if rng.next_u64() & 1 == 0 {
                                rng.spicy_i32()
                            } else {
                                extremes[(rng.next_u64() % extremes.len() as u64) as usize]
                            },
                            pred,
                            tgt,
                            tgt2,
                            lsbit: mode.sample(&mut rng),
                        };
                        diff("cfg40 extremes", a);
                    }
                }
            }
        }
    }
}

/// Row 41: everything uniformly random over the whole `i32` domain.
#[test]
fn cfg_row_41_fully_random_full_i32() {
    let mut rng = Rng::new(41);
    for _ in 0..200_000 {
        let a = Args {
            uni: rng.i32_full(),
            step: rng.i32_full(),
            pred: rng.i32_full(),
            tgt: rng.i32_full(),
            tgt2: rng.i32_full(),
            lsbit: rng.i32_full(),
        };
        diff("cfg41 uniform random", a);
    }
    // Same again but biased toward boundaries in every slot.
    let mut rng = Rng::new(4141);
    for _ in 0..200_000 {
        let a = Args {
            uni: rng.spicy_i32(),
            step: rng.spicy_i32(),
            pred: rng.spicy_i32(),
            tgt: rng.spicy_i32(),
            tgt2: rng.spicy_i32(),
            lsbit: rng.spicy_i32(),
        };
        diff("cfg41 boundary-biased random", a);
    }
}

/// Row 42: exhaustive nominal domain `uni in 0..=15` x `lsbit in 0..=8`.
#[test]
fn cfg_row_42_exhaustive_nominal_domain() {
    let mut rng = Rng::new(42);
    for uni in 0..=15 {
        for lsbit in 0..=8 {
            for _ in 0..400 {
                let a = Args {
                    uni,
                    step: rng.spicy_i32(),
                    pred: rng.spicy_i32(),
                    tgt: rng.spicy_i32(),
                    tgt2: rng.spicy_i32(),
                    lsbit,
                };
                diff("cfg42 nominal uni x lsbit", a);
            }
        }
    }
}

/// Row 43: exhaustive `lsbit in -16..=16` (0, 4, -4, negative odd/even).
#[test]
fn cfg_row_43_exhaustive_lsbit_signed() {
    let mut rng = Rng::new(43);
    for lsbit in -16..=16 {
        for uni in 0..=15 {
            for _ in 0..120 {
                let a = Args {
                    uni,
                    step: rng.spicy_i32(),
                    pred: rng.spicy_i32(),
                    tgt: rng.spicy_i32(),
                    tgt2: rng.spicy_i32(),
                    lsbit,
                };
                diff("cfg43 signed lsbit sweep", a);
            }
        }
    }
}

/// Row 44: exhaustive low-bit shapes `uni in -32..=32` under all modes.
#[test]
fn cfg_row_44_exhaustive_low_bit_shapes() {
    let mut rng = Rng::new(44);
    for uni in -32..=32 {
        for mode in LsMode::all() {
            for _ in 0..200 {
                let a = Args {
                    uni,
                    step: rng.spicy_i32(),
                    pred: rng.spicy_i32(),
                    tgt: rng.spicy_i32(),
                    tgt2: rng.spicy_i32(),
                    lsbit: mode.sample(&mut rng),
                };
                diff("cfg44 low-bit shapes", a);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// PHASE C — error/rejection-path differential tests, one per ERRORS.md row
// ---------------------------------------------------------------------------
//
// The C library has no rejection path at all (see ERRORS.md): one `return`, no
// asserts, no pointers, no enums, no range checks. So each row below drives the
// invalid/degenerate input and asserts the two `.so`s return the IDENTICAL int
// — the strictly stronger form of "same error code or sentinel".
//
// E1 (null pointer) and E2 (zero/oversized length) are unrepresentable: the
// signature is six by-value `int`s. E11 (division by zero) is unreachable: the
// only divisor in the library is the literal 8. Those three rows are discharged
// by the signature/source and have no callable test.

/// E3: out-of-range `lsbit` "enum" values — every value with no documented
/// variant still has to be handled identically. Exhaustive over a wide window
/// plus random full-range values.
#[test]
fn err_e3_lsbit_out_of_range_exhaustive_modes() {
    let mut rng = Rng::new(0xE3);
    // Exhaustive over a window that covers all mode transitions.
    for lsbit in -64..=64 {
        for uni in 0..=15 {
            for _ in 0..40 {
                diff(
                    "E3 lsbit window",
                    Args {
                        uni,
                        step: rng.spicy_i32(),
                        pred: rng.spicy_i32(),
                        tgt: rng.spicy_i32(),
                        tgt2: rng.spicy_i32(),
                        lsbit,
                    },
                );
            }
        }
    }
    // Wild values with no valid variant, incl. the extremes.
    let wild = [
        i32::MIN,
        i32::MIN + 1,
        -1_000_000_007,
        -5,
        -3,
        -2,
        2,
        3,
        5,
        6,
        7,
        8,
        1_000_000_007,
        i32::MAX - 1,
        i32::MAX,
    ];
    for &lsbit in &wild {
        for _ in 0..2000 {
            diff(
                "E3 wild lsbit",
                Args {
                    uni: rng.spicy_i32(),
                    step: rng.spicy_i32(),
                    pred: rng.spicy_i32(),
                    tgt: rng.spicy_i32(),
                    tgt2: rng.spicy_i32(),
                    lsbit,
                },
            );
        }
    }
}

/// E4: negative `lsbit`, incl. `-4` (which does NOT take the `== 4` path) and
/// `INT_MIN` (even, so clear-LSB).
#[test]
fn err_e4_lsbit_negative_and_int_min() {
    let mut rng = Rng::new(0xE4);
    for &lsbit in &[-1i32, -2, -3, -4, -5, -7, -8, i32::MIN, i32::MIN + 1] {
        for uni in -16..=16 {
            for _ in 0..80 {
                diff(
                    "E4 negative lsbit",
                    Args {
                        uni,
                        step: rng.spicy_i32(),
                        pred: rng.spicy_i32(),
                        tgt: rng.spicy_i32(),
                        tgt2: rng.spicy_i32(),
                        lsbit,
                    },
                );
            }
        }
    }
}

/// E5: `uni` outside the nominal 0..15 quantizer range.
#[test]
fn err_e5_uni_out_of_nominal_range() {
    let mut rng = Rng::new(0xE5);
    for mode in LsMode::all() {
        // Negative and >= 16, plus fully random.
        for _ in 0..N {
            let uni = rng.range(i32::MIN, -1);
            diff(
                "E5 uni<0",
                Args {
                    uni,
                    step: rng.spicy_i32(),
                    pred: rng.spicy_i32(),
                    tgt: rng.spicy_i32(),
                    tgt2: rng.spicy_i32(),
                    lsbit: mode.sample(&mut rng),
                },
            );
        }
        for _ in 0..N {
            let uni = rng.range(16, i32::MAX);
            diff(
                "E5 uni>=16",
                Args {
                    uni,
                    step: rng.spicy_i32(),
                    pred: rng.spicy_i32(),
                    tgt: rng.spicy_i32(),
                    tgt2: rng.spicy_i32(),
                    lsbit: mode.sample(&mut rng),
                },
            );
        }
    }
    // Exactly one step past the documented range in both directions.
    for &uni in &[-1i32, 16] {
        for lsbit in -8..=8 {
            for _ in 0..200 {
                diff(
                    "E5 one past range",
                    Args {
                        uni,
                        step: rng.spicy_i32(),
                        pred: rng.spicy_i32(),
                        tgt: rng.spicy_i32(),
                        tgt2: rng.spicy_i32(),
                        lsbit,
                    },
                );
            }
        }
    }
}

/// E6: `step == 0` — the "zero length" analogue.
#[test]
fn err_e6_step_zero() {
    let mut rng = Rng::new(0xE6);
    for lsbit in -8..=8 {
        for uni in -16..=16 {
            for _ in 0..40 {
                diff(
                    "E6 step=0",
                    Args {
                        uni,
                        step: 0,
                        pred: rng.spicy_i32(),
                        tgt: rng.spicy_i32(),
                        tgt2: rng.spicy_i32(),
                        lsbit,
                    },
                );
            }
        }
    }
}

/// E7: negative `step`.
#[test]
fn err_e7_step_negative() {
    let mut rng = Rng::new(0xE7);
    for mode in LsMode::all() {
        for _ in 0..4 * N {
            let step = rng.range(i32::MIN, -1);
            diff(
                "E7 step<0",
                Args {
                    uni: rng.spicy_i32(),
                    step,
                    pred: rng.spicy_i32(),
                    tgt: rng.spicy_i32(),
                    tgt2: rng.spicy_i32(),
                    lsbit: mode.sample(&mut rng),
                },
            );
        }
    }
    // Small negative steps exercise `/8` truncation toward zero explicitly.
    for step in -32..0 {
        for uni in 0..=15 {
            for lsbit in [0, 4, 1, 2] {
                diff(
                    "E7 small step<0",
                    Args {
                        uni,
                        step,
                        pred: 0,
                        tgt: 0,
                        tgt2: 0,
                        lsbit,
                    },
                );
            }
        }
    }
}

/// E8: `step` oversized so the multiply overflows `int`.
#[test]
fn err_e8_step_multiply_overflow() {
    let mut rng = Rng::new(0xE8);
    let t = i32::MAX / 15;
    for uni in 0..=15 {
        for lsbit in [0i32, 4, 1, 2, -1, -2] {
            for _ in 0..300 {
                let mag = rng.range(t, i32::MAX);
                let step = if rng.next_u64() & 1 == 0 {
                    mag
                } else {
                    mag.wrapping_neg()
                };
                diff(
                    "E8 step multiply overflow",
                    Args {
                        uni,
                        step,
                        pred: rng.spicy_i32(),
                        tgt: rng.spicy_i32(),
                        tgt2: rng.spicy_i32(),
                        lsbit,
                    },
                );
            }
        }
    }
    // The precise overflow boundary for the largest multiplier (2*7+1 == 15).
    for step in [t - 1, t, t + 1, i32::MAX, i32::MIN, i32::MIN + 1] {
        for uni in 0..=15 {
            for lsbit in -4..=4 {
                diff(
                    "E8 overflow boundary",
                    Args {
                        uni,
                        step,
                        pred: 0,
                        tgt: 0,
                        tgt2: 0,
                        lsbit,
                    },
                );
            }
        }
    }
}

/// E9: `uni == INT_MAX` / `INT_MIN`, where `uni + 1` / `uni - 1` overflow.
#[test]
fn err_e9_uni_increment_decrement_overflow() {
    let mut rng = Rng::new(0xE9);
    for &uni in &[i32::MAX, i32::MIN] {
        for lsbit in -8..=8 {
            for _ in 0..400 {
                diff(
                    "E9 uni +/-1 overflow",
                    Args {
                        uni,
                        step: rng.spicy_i32(),
                        pred: rng.spicy_i32(),
                        tgt: rng.spicy_i32(),
                        tgt2: rng.spicy_i32(),
                        lsbit,
                    },
                );
            }
        }
    }
}

/// E10: `pred`/`tgt`/`tgt2` extremes — wrapping adds/subs, and the
/// `d ^ (d >> 31)` idiom mapping `INT_MIN -> INT_MAX` rather than `abs`.
#[test]
fn err_e10_pred_tgt_overflow_and_absminus1() {
    let mut rng = Rng::new(0xE10);
    let ext = [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX];
    for &pred in &ext {
        for &tgt in &ext {
            for &tgt2 in &ext {
                for &step in &[i32::MIN, -1, 0, 1, 8, i32::MAX] {
                    for lsbit in [0i32, 4, 1, 2, -3] {
                        diff(
                            "E10 extremes grid",
                            Args {
                                uni: rng.range(-16, 16),
                                step,
                                pred,
                                tgt,
                                tgt2,
                                lsbit,
                            },
                        );
                    }
                }
            }
        }
    }
    // Drive `tgt - p0 == INT_MIN` on purpose: pred+diff == 0 and tgt == INT_MIN.
    for lsbit in [0i32, 4, 1, 2] {
        for uni in 0..=15 {
            diff(
                "E10 forced INT_MIN distance",
                Args {
                    uni,
                    step: 0,
                    pred: 0,
                    tgt: i32::MIN,
                    tgt2: i32::MIN,
                    lsbit,
                },
            );
        }
    }
}

// ---------------------------------------------------------------------------
// High-volume sweeps
// ---------------------------------------------------------------------------
//
// A truly exhaustive test is impossible (2^192 inputs), so these two cover a
// dense *projection* of the domain and a very large random sample. Measured
// throughput of the differential harness is ~11M FFI call pairs/second, so both
// finish in seconds; each is additionally time-budgeted so it can never run
// away. Override the budget with `DIFF_SWEEP_SECS`.

fn sweep_budget() -> std::time::Duration {
    let secs: u64 = std::env::var("DIFF_SWEEP_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(120);
    std::time::Duration::from_secs(secs)
}

/// Dense exhaustive projection: every `uni` in the nominal range crossed with
/// every `lsbit` mode representative, swept over contiguous `step`/`tgt`/`tgt2`
/// ranges. Contiguous sweeps (rather than sampling) are what catch off-by-one
/// bugs in the `/8` quantization and the `>>5` tiebreak boundary.
#[test]
fn sweep_dense_exhaustive_projection() {
    let deadline = std::time::Instant::now() + sweep_budget();
    let mut calls: u64 = 0;
    let lsbits = [0i32, 4, 1, 2, 3, -1, -2, -4];
    'outer: for uni in -1..=16i32 {
        for &lsbit in &lsbits {
            for step in -72..=72i32 {
                for tgt in -40..=40i32 {
                    for tgt2s in -4..=4i32 {
                        let tgt2 = tgt2s * 37;
                        for &pred in &[0i32, -33, 91] {
                            diff(
                                "sweep dense projection",
                                Args {
                                    uni,
                                    step,
                                    pred,
                                    tgt,
                                    tgt2,
                                    lsbit,
                                },
                            );
                            calls += 1;
                        }
                    }
                }
                if std::time::Instant::now() > deadline {
                    break 'outer;
                }
            }
        }
    }
    println!("sweep_dense_exhaustive_projection: {calls} differential calls");
    assert!(calls > 1_000_000, "sweep did too little work: {calls}");
}

/// Very large random sample: uniform over the full `i32^6` domain, plus a
/// boundary-biased sample, plus a "one axis extreme, rest random" sample that
/// systematically pairs each argument's extremes with random values elsewhere.
#[test]
fn sweep_large_random_sample() {
    let deadline = std::time::Instant::now() + sweep_budget();
    let mut calls: u64 = 0;

    let mut rng = Rng::new(0x5EED_0001);
    for i in 0..10_000_000u64 {
        diff(
            "sweep uniform",
            Args {
                uni: rng.i32_full(),
                step: rng.i32_full(),
                pred: rng.i32_full(),
                tgt: rng.i32_full(),
                tgt2: rng.i32_full(),
                lsbit: rng.i32_full(),
            },
        );
        calls += 1;
        if i % 65_536 == 0 && std::time::Instant::now() > deadline {
            break;
        }
    }

    let mut rng = Rng::new(0x5EED_0002);
    for i in 0..10_000_000u64 {
        diff(
            "sweep boundary-biased",
            Args {
                uni: rng.spicy_i32(),
                step: rng.spicy_i32(),
                pred: rng.spicy_i32(),
                tgt: rng.spicy_i32(),
                tgt2: rng.spicy_i32(),
                lsbit: rng.spicy_i32(),
            },
        );
        calls += 1;
        if i % 65_536 == 0 && std::time::Instant::now() > deadline {
            break;
        }
    }

    // One axis pinned to an extreme, everything else random.
    let extremes = [i32::MIN, i32::MIN + 1, -1, 0, 1, 4, 7, 8, 15, i32::MAX - 1, i32::MAX];
    let mut rng = Rng::new(0x5EED_0003);
    for axis in 0..6usize {
        for &ext in &extremes {
            for _ in 0..20_000 {
                let mut v = [
                    rng.spicy_i32(),
                    rng.spicy_i32(),
                    rng.spicy_i32(),
                    rng.spicy_i32(),
                    rng.spicy_i32(),
                    rng.spicy_i32(),
                ];
                v[axis] = ext;
                diff(
                    "sweep pinned axis",
                    Args {
                        uni: v[0],
                        step: v[1],
                        pred: v[2],
                        tgt: v[3],
                        tgt2: v[4],
                        lsbit: v[5],
                    },
                );
                calls += 1;
            }
            if std::time::Instant::now() > deadline {
                break;
            }
        }
    }

    println!("sweep_large_random_sample: {calls} differential calls");
    assert!(calls > 1_000_000, "sweep did too little work: {calls}");
}

// ---------------------------------------------------------------------------
// PHASE D — symbol parity asserted from inside the test suite
// ---------------------------------------------------------------------------

/// Both `.so`s must export `encode_quant` (already implied by `libs()`, made
/// explicit here), and the C `.so` must export nothing the Rust one lacks.
#[test]
fn symbol_parity_c_vs_rust() {
    let c = c_so_path();
    let r = rust_so_path();

    let dyn_defined = |p: &Path| -> Vec<String> {
        let out = Command::new("nm")
            .args(["-D", "--defined-only", p.to_str().unwrap()])
            .output()
            .expect("`nm` not available");
        assert!(out.status.success(), "nm failed on {}", p.display());
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|l| l.split_whitespace().last().map(str::to_string))
            .filter(|s| !s.is_empty())
            .collect()
    };

    let c_syms = dyn_defined(&c);
    let r_syms = dyn_defined(&r);

    assert!(
        c_syms.contains(&"encode_quant".to_string()),
        "C .so must export encode_quant, got {c_syms:?}"
    );

    let missing: Vec<&String> = c_syms.iter().filter(|s| !r_syms.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by C .so but MISSING from Rust .so: {missing:?}\n  C: {c:?}\n  Rust: {r:?}"
    );

    // Both symbols must be dlsym-able and callable (exercises the export wrapper).
    let l = libs();
    let a = Args {
        uni: 5,
        step: 100,
        pred: 10,
        tgt: 90,
        tgt2: 95,
        lsbit: 4,
    };
    let cv = unsafe { (l.c)(a.uni, a.step, a.pred, a.tgt, a.tgt2, a.lsbit) };
    let rv = unsafe { (l.rust)(a.uni, a.step, a.pred, a.tgt, a.tgt2, a.lsbit) };
    assert_eq!(cv, rv);
}

/// Prints which artifacts were compared, so a passing run is auditable.
#[test]
fn report_loaded_artifacts() {
    println!("C    .so: {}", c_so_path().display());
    println!("Rust .so: {}", rust_so_path().display());
    assert!(c_so_path().exists());
    assert!(rust_so_path().exists());
}
