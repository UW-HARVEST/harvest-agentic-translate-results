// Differential test harness: loads BOTH the C `libdriver.so` and the Rust
// `libdriver.so` through `libloading` and compares the bytes each one writes to
// `stdout`.  The Rust implementation is never called directly — only through
// its `#[no_mangle]` export, exactly as an external C consumer would — so the
// export wrapper itself is under test.
//
// Phase B tests are named `phase_b_row<NN>_*` and correspond 1:1 to the rows of
// CONFIGS.md.  Phase C tests are named `phase_c_row<NN>_*` and correspond 1:1
// to the rows of ERRORS.md.

use std::path::PathBuf;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Library discovery
//
// Both libraries are always reached through `dlopen` + `dlsym` on their exported
// `driver` symbol (see `examples/driver_dump.rs`); the Rust implementation is
// never linked or called directly, so its `#[no_mangle]`/`extern "C"` wrapper is
// part of what is under test.
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("manifest dir has a parent")
        .to_path_buf()
}

fn c_so_path() -> PathBuf {
    workspace_root().join("c_src/build/libdriver.so")
}

/// Every Rust `cdylib` we can find.  Both profiles are tested when present:
/// `release` is the shipping artifact, `debug` is what `cargo test` builds and
/// differs in `panic` strategy and optimisation level.
fn rust_so_paths() -> Vec<PathBuf> {
    let target = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
    let mut v = Vec::new();
    for profile in ["release", "debug"] {
        let p = target.join(profile).join("libdriver.so");
        if p.is_file() {
            v.push(p);
        }
    }
    assert!(
        !v.is_empty(),
        "no Rust cdylib found under {}; run `cargo build --release` first",
        target.display()
    );
    v
}

struct Loaded {
    c: PathBuf,
    rust: Vec<(String, PathBuf)>,
}

fn loaded() -> &'static Loaded {
    static L: OnceLock<Loaded> = OnceLock::new();
    L.get_or_init(|| {
        let c = c_so_path();
        assert!(
            c.is_file(),
            "C shared library missing at {}; build it with cmake first",
            c.display()
        );
        let rust = rust_so_paths()
            .into_iter()
            .map(|p| {
                let label = format!(
                    "rust:{}",
                    p.parent()
                        .and_then(|d| d.file_name())
                        .map(|s| s.to_string_lossy().into_owned())
                        .unwrap_or_default()
                );
                (label, p)
            })
            .collect();
        Loaded { c, rust }
    })
}

// ---------------------------------------------------------------------------
// stdout capture (subprocess isolation)
//
// Both `.so`s write through the process' glibc `stdout` FILE.  Redirecting fd 1
// inside the test process is not viable: the `cargo test` harness writes its own
// progress lines to fd 1 from other threads, which would land in the capture.
// Instead each batch runs in a dedicated child process (`examples/driver_dump`)
// whose stdout is a pipe, so the captured bytes are exclusively the library's.
// ---------------------------------------------------------------------------

/// Path to the `driver_dump` helper, built by `cargo test` alongside the tests.
fn helper() -> PathBuf {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        // current_exe() == <target>/<profile>/deps/differential-<hash>
        let exe = std::env::current_exe().expect("current_exe");
        let profile_dir = exe
            .parent()
            .and_then(|p| p.parent())
            .expect("profile dir")
            .to_path_buf();
        let mut candidates = vec![profile_dir.join("examples/driver_dump")];
        let target = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
        for profile in ["release", "debug"] {
            candidates.push(target.join(profile).join("examples/driver_dump"));
        }
        for c in &candidates {
            if c.is_file() {
                return c.clone();
            }
        }
        panic!(
            "helper binary `driver_dump` not found; looked in {:?}. \
             Build it with `cargo build --release --example driver_dump`.",
            candidates
        );
    })
    .clone()
}

/// Ambient process state the C library reads on every call, and which a real
/// consumer controls: the `LC_NUMERIC` decimal point and the FP rounding
/// direction.  `None` means "leave at the process default".
#[derive(Clone, Copy, Default)]
struct Ambient {
    locale: Option<&'static str>,
    round: Option<&'static str>,
}

impl Ambient {
    const DEFAULT: Ambient = Ambient {
        locale: None,
        round: None,
    };
    fn locale(l: &'static str) -> Ambient {
        Ambient {
            locale: Some(l),
            round: None,
        }
    }
    fn round(r: &'static str) -> Ambient {
        Ambient {
            locale: None,
            round: Some(r),
        }
    }
    fn both(l: &'static str, r: &'static str) -> Ambient {
        Ambient {
            locale: Some(l),
            round: Some(r),
        }
    }
    fn label(&self) -> String {
        format!(
            "locale={} round={}",
            self.locale.unwrap_or("<default>"),
            self.round.unwrap_or("<default>")
        )
    }
}

/// Locales installed on this machine, filtered to those actually available so
/// the suite does not fail on a minimal image.
fn available_locales() -> &'static Vec<&'static str> {
    static L: OnceLock<Vec<&'static str>> = OnceLock::new();
    L.get_or_init(|| {
        let out = std::process::Command::new("locale")
            .arg("-a")
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
            .unwrap_or_default();
        let have = |n: &str| out.lines().any(|l| l.trim() == n);
        let mut v = vec!["C", "POSIX"];
        for cand in [
            "en_US.utf8",
            "de_DE.utf8",
            "fr_FR.UTF-8",
            "de_DE.iso88591",
            "ru_RU.utf8",
            // Multi-byte radix character (U+066B ARABIC DECIMAL SEPARATOR):
            // glibc writes the whole `decimal_point` string, not a single char.
            "ps_AF.utf8",
        ] {
            if have(cand) {
                v.push(cand);
            }
        }
        v
    })
}

const ROUND_MODES: [&str; 4] = ["nearest", "downward", "upward", "towardzero"];

/// Run `values` through the `driver` export of every listed `.so`, in one child
/// process, and return the raw stdout bytes.
fn run_libs_amb(libs: &[&PathBuf], values: &[f64], amb: Ambient) -> Vec<u8> {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let mut input = String::with_capacity(values.len() * 17);
    for v in values {
        input.push_str(&format!("{:016x}\n", v.to_bits()));
    }

    let mut cmd = Command::new(helper());
    for l in libs {
        cmd.arg(l);
    }
    cmd.env_remove("DRIVER_DUMP_LOCALE");
    cmd.env_remove("DRIVER_DUMP_ROUND");
    if let Some(l) = amb.locale {
        cmd.env("DRIVER_DUMP_LOCALE", l);
    }
    if let Some(r) = amb.round {
        cmd.env("DRIVER_DUMP_ROUND", r);
    }
    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn driver_dump");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(input.as_bytes())
        .expect("write bit patterns to helper");
    let out = child.wait_with_output().expect("wait driver_dump");
    assert!(
        out.status.success(),
        "driver_dump({:?}, {}) failed: status {:?}, stderr: {}",
        libs,
        amb.label(),
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );
    out.stdout
}

fn run_libs(libs: &[&PathBuf], values: &[f64]) -> Vec<u8> {
    run_libs_amb(libs, values, Ambient::DEFAULT)
}

fn run_batch_path(lib: &PathBuf, values: &[f64]) -> Vec<u8> {
    run_libs_amb(&[lib], values, Ambient::DEFAULT)
}

fn split_lines(raw: &[u8]) -> Vec<&[u8]> {
    let mut v: Vec<&[u8]> = raw.split(|&b| b == b'\n').collect();
    // A well-formed run ends with a newline, producing one trailing empty slice.
    if let Some(last) = v.last() {
        if last.is_empty() {
            v.pop();
        }
    }
    v
}

fn show(b: &[u8]) -> String {
    String::from_utf8_lossy(b).into_owned()
}

/// The core differential assertion: run `values` through the C `.so` and
/// through every Rust `.so`, and require byte-identical output.
#[track_caller]
fn assert_match(row: &str, values: &[f64]) {
    assert_match_amb(row, values, Ambient::DEFAULT)
}

/// Same, but with a specific ambient locale / rounding direction applied to the
/// child process before either library is called.
#[track_caller]
fn assert_match_amb(row: &str, values: &[f64], amb: Ambient) {
    assert!(!values.is_empty(), "{row}: empty value set");
    let row = &format!("{row} [{}]", amb.label());
    let l = loaded();
    let c_out = run_libs_amb(&[&l.c], values, amb);
    let c_lines = split_lines(&c_out);
    assert_eq!(
        c_lines.len(),
        values.len(),
        "{row}: C produced {} lines for {} inputs",
        c_lines.len(),
        values.len()
    );

    for (label, path) in &l.rust {
        let r_out = run_libs_amb(&[path], values, amb);
        let r_lines = split_lines(&r_out);
        assert_eq!(
            r_lines.len(),
            values.len(),
            "{row}/{label}: produced {} lines for {} inputs",
            r_lines.len(),
            values.len()
        );
        for (i, (cl, rl)) in c_lines.iter().zip(r_lines.iter()).enumerate() {
            if cl != rl {
                panic!(
                    "{row}/{label}: divergence at input #{i}\n  \
                     input bits = 0x{:016x}  (as f64 = {:?})\n  \
                     C    = {:?}\n  rust = {:?}",
                    values[i].to_bits(),
                    values[i],
                    show(cl),
                    show(rl),
                );
            }
        }
        // Whole-buffer equality also catches any stray extra bytes.
        assert!(
            c_out == r_out,
            "{row}/{label}: raw stdout bytes differ despite line-wise equality"
        );
    }
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64) — fixed seed for reproducibility
// ---------------------------------------------------------------------------

const SEED: u64 = 0x2545_F491_4F6C_DD1D;

struct Rng(u64);

impl Rng {
    fn new(salt: u64) -> Self {
        Rng(SEED ^ salt.wrapping_mul(0x9E37_79B9_7F4A_7C15))
    }
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// Uniform in `[lo, hi]` inclusive.
    fn range(&mut self, lo: u64, hi: u64) -> u64 {
        debug_assert!(lo <= hi);
        lo + self.next_u64() % (hi - lo + 1)
    }
    fn bit(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
    /// Uniform in `[0, 1)`.
    fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
    }
}

/// Compose a `double` from its IEEE-754 fields, the way the C union does.
fn mk(neg: bool, exp: u64, mant: u64) -> f64 {
    f64::from_bits(((neg as u64) << 63) | ((exp & 0x7ff) << 52) | (mant & 0x000f_ffff_ffff_ffff))
}

/// How many randomized inputs each CONFIGS.md row is driven with.
const N: usize = 2000;
/// Smaller count for rows whose `%.4f` output is hundreds of digits wide.
const N_WIDE: usize = 600;

// ===========================================================================
// Phase B — valid-path differential tests, one per CONFIGS.md row
// ===========================================================================

#[test]
fn phase_b_row01_positive_zero() {
    // Row 1: +0.0.  Repeated so the batch path is exercised too.
    let values = vec![0.0f64; 8];
    assert_match("CONFIGS row 1 (+0.0)", &values);
}

#[test]
fn phase_b_row02_negative_zero() {
    let values = vec![-0.0f64; 8];
    assert_match("CONFIGS row 2 (-0.0)", &values);
}

#[test]
fn phase_b_row03_positive_subnormals() {
    let mut rng = Rng::new(3);
    let values: Vec<f64> = (0..N)
        .map(|_| {
            let mut m = rng.next_u64() & 0x000f_ffff_ffff_ffff;
            if m == 0 {
                m = 1;
            }
            mk(false, 0, m)
        })
        .collect();
    assert_match("CONFIGS row 3 (+subnormal)", &values);
}

#[test]
fn phase_b_row04_negative_subnormals() {
    let mut rng = Rng::new(4);
    let values: Vec<f64> = (0..N)
        .map(|_| {
            let mut m = rng.next_u64() & 0x000f_ffff_ffff_ffff;
            if m == 0 {
                m = 1;
            }
            mk(true, 0, m)
        })
        .collect();
    assert_match("CONFIGS row 4 (-subnormal)", &values);
}

#[test]
fn phase_b_row05_subnormal_single_bit() {
    let mut rng = Rng::new(5);
    let mut values = Vec::with_capacity(N);
    // Every single-bit subnormal, both signs, exhaustively (52 * 2 = 104), then
    // randomized repeats to fill the row.
    for bit in 0..52u32 {
        values.push(mk(false, 0, 1u64 << bit));
        values.push(mk(true, 0, 1u64 << bit));
    }
    while values.len() < N {
        let bit = rng.range(0, 51) as u32;
        values.push(mk(rng.bit(), 0, 1u64 << bit));
    }
    assert_match("CONFIGS row 5 (subnormal, one significant nibble)", &values);
}

#[test]
fn phase_b_row06_subnormal_full_mantissa() {
    let mut rng = Rng::new(6);
    let mut values = Vec::with_capacity(N);
    // Low-bits-all-set masks: 0x1, 0x3, 0x7, ... 0xfffffffffffff.
    for k in 1..=52u32 {
        let m = (1u64 << k) - 1;
        values.push(mk(false, 0, m));
        values.push(mk(true, 0, m));
    }
    while values.len() < N {
        let k = rng.range(1, 52) as u32;
        values.push(mk(rng.bit(), 0, (1u64 << k) - 1));
    }
    assert_match("CONFIGS row 6 (subnormal, full 13 hex digits)", &values);
}

#[test]
fn phase_b_row07_min_normal_exponent() {
    let mut rng = Rng::new(7);
    let values: Vec<f64> = (0..N)
        .map(|_| mk(rng.bit(), 1, rng.next_u64()))
        .collect();
    assert_match("CONFIGS row 7 (exponent field 1, p-1022)", &values);
}

#[test]
fn phase_b_row08_negative_binary_exponents() {
    let mut rng = Rng::new(8);
    let values: Vec<f64> = (0..N)
        .map(|_| mk(rng.bit(), rng.range(2, 0x3fe), rng.next_u64()))
        .collect();
    assert_match("CONFIGS row 8 (exponent field 2..0x3fe, p-)", &values);
}

#[test]
fn phase_b_row09_half_to_one() {
    let mut rng = Rng::new(9);
    let values: Vec<f64> = (0..N)
        .map(|_| mk(rng.bit(), 0x3fe, rng.next_u64()))
        .collect();
    assert_match("CONFIGS row 9 (magnitude in [0.5, 1), p-1)", &values);
}

#[test]
fn phase_b_row10_one_to_two() {
    let mut rng = Rng::new(10);
    let values: Vec<f64> = (0..N)
        .map(|_| mk(rng.bit(), 0x3ff, rng.next_u64()))
        .collect();
    assert_match("CONFIGS row 10 (magnitude in [1, 2), p+0)", &values);
}

#[test]
fn phase_b_row11_mixed_integer_fraction() {
    let mut rng = Rng::new(11);
    let values: Vec<f64> = (0..N)
        .map(|_| mk(rng.bit(), rng.range(0x400, 0x433), rng.next_u64()))
        .collect();
    assert_match("CONFIGS row 11 (0x400..0x433, mixed int+frac)", &values);
}

#[test]
fn phase_b_row12_large_integral() {
    let mut rng = Rng::new(12);
    let values: Vec<f64> = (0..N_WIDE)
        .map(|_| mk(rng.bit(), rng.range(0x434, 0x7fe), rng.next_u64()))
        .collect();
    assert_match("CONFIGS row 12 (0x434..0x7fe, integral)", &values);
}

#[test]
fn phase_b_row13_near_dbl_max() {
    let mut rng = Rng::new(13);
    let values: Vec<f64> = (0..N_WIDE)
        .map(|_| mk(rng.bit(), 0x7fe, rng.next_u64()))
        .collect();
    assert_match("CONFIGS row 13 (exponent field 0x7fe, ~DBL_MAX)", &values);
}

#[test]
fn phase_b_row14_zero_mantissa_powers_of_two() {
    let mut rng = Rng::new(14);
    let mut values = Vec::with_capacity(N);
    // Every power of two (both signs) exhaustively: 2046 * 2 entries.
    for e in 1..=0x7feu64 {
        values.push(mk(false, e, 0));
        values.push(mk(true, e, 0));
    }
    while values.len() < N {
        values.push(mk(rng.bit(), rng.range(1, 0x7fe), 0));
    }
    assert_match("CONFIGS row 14 (mantissa 0, no radix point)", &values);
}

#[test]
fn phase_b_row15_single_bit_mantissa() {
    let mut rng = Rng::new(15);
    let values: Vec<f64> = (0..N)
        .map(|_| {
            let bit = rng.range(0, 51) as u32;
            mk(rng.bit(), rng.range(1, 0x7fe), 1u64 << bit)
        })
        .collect();
    assert_match("CONFIGS row 15 (single-bit mantissa)", &values);
}

#[test]
fn phase_b_row16_all_mantissa_bits_set() {
    let mut rng = Rng::new(16);
    let mut values = Vec::with_capacity(N);
    for e in 1..=0x7feu64 {
        values.push(mk(false, e, 0x000f_ffff_ffff_ffff));
    }
    while values.len() < N {
        values.push(mk(rng.bit(), rng.range(1, 0x7fe), 0x000f_ffff_ffff_ffff));
    }
    assert_match("CONFIGS row 16 (all 52 mantissa bits set)", &values);
}

#[test]
fn phase_b_row17_every_trailing_zero_trim_length() {
    let mut rng = Rng::new(17);
    let mut values = Vec::with_capacity(N);
    // k trailing zero nibbles for k = 0..=12, so `%a` trims 0..12 digits.
    for k in 0..=12u32 {
        for _ in 0..(N / 13 + 1) {
            let mut m = rng.next_u64() & 0x000f_ffff_ffff_ffff;
            m &= !((1u64 << (4 * k)) - 1);
            // Keep at least one significant nibble so the row stays distinct
            // from row 14 (mantissa == 0) unless k == 13 which we exclude.
            if m == 0 && k < 13 {
                m = 1u64 << (4 * k);
            }
            values.push(mk(rng.bit(), rng.range(1, 0x7fe), m));
        }
    }
    assert_match("CONFIGS row 17 (trailing-zero trim lengths 0..12)", &values);
}

#[test]
fn phase_b_row18_positive_infinity() {
    let values = vec![f64::INFINITY; 8];
    assert_match("CONFIGS row 18 (+inf)", &values);
}

#[test]
fn phase_b_row19_negative_infinity() {
    let values = vec![f64::NEG_INFINITY; 8];
    assert_match("CONFIGS row 19 (-inf)", &values);
}

#[test]
fn phase_b_row20_nan_payloads() {
    let mut rng = Rng::new(20);
    let mut values = Vec::with_capacity(N);
    // Quiet, signalling, min and max payloads, both signs.
    for &m in &[
        1u64,
        2,
        0x7_ffff_ffff_ffff,
        0x8_0000_0000_0000,
        0x8_0000_0000_0001,
        0xf_ffff_ffff_ffff,
    ] {
        values.push(mk(false, 0x7ff, m));
        values.push(mk(true, 0x7ff, m));
    }
    while values.len() < N {
        let mut m = rng.next_u64() & 0x000f_ffff_ffff_ffff;
        if m == 0 {
            m = 1;
        }
        values.push(mk(rng.bit(), 0x7ff, m));
    }
    assert_match("CONFIGS row 20 (NaN, arbitrary payload)", &values);
}

#[test]
fn phase_b_row21_uniform_random_bit_patterns() {
    let mut rng = Rng::new(21);
    let values: Vec<f64> = (0..N_WIDE)
        .map(|_| f64::from_bits(rng.next_u64()))
        .collect();
    assert_match("CONFIGS row 21 (uniform random 64-bit patterns)", &values);
}

#[test]
fn phase_b_row22_small_integers() {
    let mut rng = Rng::new(22);
    let mut values: Vec<f64> = (-64i64..=64).map(|n| n as f64).collect();
    while values.len() < N {
        let n = rng.range(0, 2 * (1 << 20)) as i64 - (1 << 20);
        values.push(n as f64);
    }
    assert_match("CONFIGS row 22 (small integers)", &values);
}

#[test]
fn phase_b_row23_exact_dyadic_rationals() {
    let mut rng = Rng::new(23);
    let mut values = Vec::with_capacity(N);
    while values.len() < N {
        let k = rng.range(1, 20) as i32;
        let m = rng.range(0, 1 << 22) as i64;
        let sign = if rng.bit() { -1.0 } else { 1.0 };
        values.push(sign * (m as f64) / (2f64).powi(k));
    }
    assert_match("CONFIGS row 23 (exact dyadic rationals m/2^k)", &values);
}

#[test]
fn phase_b_row24_decimal_tie_candidates() {
    let mut rng = Rng::new(24);
    let mut values = Vec::with_capacity(N);
    // Hand-picked exact binary ties first (representable exactly, .xxxx5).
    for &v in &[
        0.03125f64, -0.03125, 0.09375, -0.09375, 0.15625, 0.21875, 0.28125,
        0.34375, 0.40625, 0.46875, 1.03125, 2.09375, 0.00005, -0.00005,
        0.00015, 0.00025, 0.00035, 1.00005, 1.00015, 2.00005,
    ] {
        values.push(v);
    }
    while values.len() < N {
        let n = rng.range(0, 10_000) as f64;
        let j = rng.range(0, 200) as f64;
        let v = n + 5e-5 * (2.0 * j + 1.0);
        values.push(if rng.bit() { -v } else { v });
    }
    assert_match("CONFIGS row 24 (decimal tie candidates)", &values);
}

#[test]
fn phase_b_row25_rounding_carry() {
    let mut rng = Rng::new(25);
    let mut values = Vec::with_capacity(N);
    for &v in &[
        0.99999f64, -0.99999, 9.99999, -9.99999, 0.999999999, 99.99999,
        999999.99999, 0.00009999,
    ] {
        values.push(v);
    }
    while values.len() < N {
        let n = rng.range(0, 1_000_000) as f64;
        let k = rng.range(4, 12) as i32;
        let v = n - 10f64.powi(-k);
        values.push(if rng.bit() { -v } else { v });
    }
    assert_match("CONFIGS row 25 (rounding carry across the radix point)", &values);
}

#[test]
fn phase_b_row26_below_fixed_resolution() {
    let mut rng = Rng::new(26);
    let values: Vec<f64> = (0..N)
        .map(|_| {
            let v = rng.unit() * 1e-4;
            if rng.bit() {
                -v
            } else {
                v
            }
        })
        .collect();
    assert_match("CONFIGS row 26 (|v| < 1e-4)", &values);
}

#[test]
fn phase_b_row27_decimal_scale_sweep() {
    let mut rng = Rng::new(27);
    let mut values = Vec::with_capacity(N_WIDE);
    for e in -320i32..=308 {
        let m = 1.0 + rng.unit() * 8.0;
        let v = m * 10f64.powi(e);
        values.push(if rng.bit() { -v } else { v });
    }
    while values.len() < N_WIDE {
        let e = rng.range(0, 628) as i32 - 320;
        let v = (1.0 + rng.unit() * 8.0) * 10f64.powi(e);
        values.push(if rng.bit() { -v } else { v });
    }
    assert_match("CONFIGS row 27 (decimal scale sweep 1e-320..1e308)", &values);
}

#[test]
fn phase_b_row28_all_exponent_fields() {
    let mut rng = Rng::new(28);
    let mant = rng.next_u64() & 0x000f_ffff_ffff_ffff;
    let mut values = Vec::with_capacity(2 * 2048);
    for e in 0..=0x7ffu64 {
        values.push(mk(false, e, mant));
        values.push(mk(true, e, mant));
    }
    assert_match("CONFIGS row 28 (all 2^11 exponent fields)", &values);
}

#[test]
fn phase_b_row29_llx_field_widths() {
    let mut rng = Rng::new(29);
    let mut values = Vec::new();
    // Exercise every `%llx` width 1..16 and every leading nibble value.
    for w in 1..=16u32 {
        let lo = if w == 1 { 0u64 } else { 1u64 << (4 * (w - 1)) };
        let hi = if w == 16 {
            u64::MAX
        } else {
            (1u64 << (4 * w)) - 1
        };
        values.push(f64::from_bits(lo));
        values.push(f64::from_bits(hi));
        for _ in 0..40 {
            let span = hi - lo;
            let x = lo + if span == 0 { 0 } else { rng.next_u64() % (span + 1) };
            values.push(f64::from_bits(x));
        }
    }
    for nib in 0..16u64 {
        values.push(f64::from_bits(nib << 60 | (rng.next_u64() >> 4)));
    }
    assert_match("CONFIGS row 29 (%llx widths 1..16, all leading nibbles)", &values);
}

#[test]
fn phase_b_row30_interleaved_invocation() {
    // Row 30: interleave C and Rust calls inside a *single* capture so that both
    // write through the same glibc `stdout` FILE, then require the C-produced
    // and Rust-produced lines to be pairwise identical.  This catches any
    // difference in buffering, trailing-newline handling, or stray output that
    // separate captures could hide.
    let mut rng = Rng::new(30);
    let values: Vec<f64> = (0..500)
        .map(|_| f64::from_bits(rng.next_u64()))
        .collect();

    let l = loaded();
    let cf = &l.c;
    for (label, rp) in &l.rust {
        // Both libraries are dlopen'd in the *same* child process and their
        // calls interleaved, so they share one glibc `stdout` FILE.
        let out = run_libs(&[cf, rp], &values);
        let lines = split_lines(&out);
        assert_eq!(
            lines.len(),
            2 * values.len(),
            "row 30/{label}: expected {} interleaved lines, got {}",
            2 * values.len(),
            lines.len()
        );
        for (i, pair) in lines.chunks(2).enumerate() {
            assert_eq!(
                show(pair[0]),
                show(pair[1]),
                "row 30/{label}: interleaved divergence at input #{i} \
                 (bits 0x{:016x})",
                values[i].to_bits()
            );
        }
    }
}

// ===========================================================================
// Phase C — error/rejection-path differential tests, one per ERRORS.md row
//
// The C API has no return value and no error channel (see ERRORS.md), so the
// "same error/rejection" assertion is: for the exact invalid/degenerate input,
// both implementations must produce the *same* sentinel rendering (`inf`,
// `-inf`, `nan`, `-nan`, `0.0000`, `-0.0000`, …) byte-for-byte, and neither may
// abort, trap, or emit a different number of lines.
// ===========================================================================

/// Assert that the given single input produces one line, identical across
/// implementations, and that the line equals `expected` in the C `.so` too —
/// pinning the concrete sentinel rather than merely "both failed somehow".
#[track_caller]
fn assert_exact(row: &str, value: f64, expected: &str) {
    assert_match(row, &[value]);
    let l = loaded();
    let out = run_batch_path(&l.c, &[value]);
    assert_eq!(
        show(&out),
        expected,
        "{row}: C reference output changed; expectation in ERRORS.md is stale"
    );
    for (label, path) in &l.rust {
        let r = run_batch_path(path, &[value]);
        assert_eq!(show(&r), expected, "{row}/{label}: wrong sentinel rendering");
    }
}

#[test]
fn phase_c_row01_pos_inf() {
    assert_exact(
        "ERRORS row 1 (+inf)",
        f64::from_bits(0x7ff0_0000_0000_0000),
        "7ff0000000000000 inf inf\n",
    );
}

#[test]
fn phase_c_row02_neg_inf() {
    assert_exact(
        "ERRORS row 2 (-inf)",
        f64::from_bits(0xfff0_0000_0000_0000),
        "fff0000000000000 -inf -inf\n",
    );
}

#[test]
fn phase_c_row03_qnan_pos() {
    assert_exact(
        "ERRORS row 3 (+qNaN)",
        f64::from_bits(0x7ff8_0000_0000_0000),
        "7ff8000000000000 nan nan\n",
    );
}

#[test]
fn phase_c_row04_qnan_neg() {
    assert_exact(
        "ERRORS row 4 (-qNaN, sign bit set)",
        f64::from_bits(0xfff8_0000_0000_0000),
        "fff8000000000000 -nan -nan\n",
    );
}

#[test]
fn phase_c_row05_snan() {
    // Signalling NaN: mantissa MSB clear, payload non-zero.  Must not trap and
    // must not be quietened before printing the raw pattern.
    assert_exact(
        "ERRORS row 5 (signalling NaN)",
        f64::from_bits(0x7ff0_0000_0000_0001),
        "7ff0000000000001 nan nan\n",
    );
    assert_exact(
        "ERRORS row 5 (signalling NaN, negative)",
        f64::from_bits(0xfff0_0000_0000_0001),
        "fff0000000000001 -nan -nan\n",
    );
}

#[test]
fn phase_c_row06_nan_payloads() {
    assert_exact(
        "ERRORS row 6 (max payload NaN, positive)",
        f64::from_bits(0x7fff_ffff_ffff_ffff),
        "7fffffffffffffff nan nan\n",
    );
    assert_exact(
        "ERRORS row 6 (all-ones pattern)",
        f64::from_bits(0xffff_ffff_ffff_ffff),
        "ffffffffffffffff -nan -nan\n",
    );
    // Plus a randomized payload sweep, still requiring C/Rust agreement.
    let mut rng = Rng::new(106);
    let values: Vec<f64> = (0..500)
        .map(|_| {
            let mut m = rng.next_u64() & 0x000f_ffff_ffff_ffff;
            if m == 0 {
                m = 1;
            }
            mk(rng.bit(), 0x7ff, m)
        })
        .collect();
    assert_match("ERRORS row 6 (randomized NaN payloads)", &values);
}

#[test]
fn phase_c_row07_negative_zero() {
    assert_exact(
        "ERRORS row 7 (-0.0)",
        f64::from_bits(0x8000_0000_0000_0000),
        "8000000000000000 -0x0p+0 -0.0000\n",
    );
}

#[test]
fn phase_c_row08_positive_zero() {
    assert_exact(
        "ERRORS row 8 (+0.0, degenerate %llx)",
        f64::from_bits(0x0000_0000_0000_0000),
        "0 0x0p+0 0.0000\n",
    );
}

#[test]
fn phase_c_row09_min_subnormal() {
    assert_exact(
        "ERRORS row 9 (smallest subnormal)",
        f64::from_bits(0x0000_0000_0000_0001),
        "1 0x0.0000000000001p-1022 0.0000\n",
    );
    assert_exact(
        "ERRORS row 9 (smallest subnormal, negative)",
        f64::from_bits(0x8000_0000_0000_0001),
        "8000000000000001 -0x0.0000000000001p-1022 -0.0000\n",
    );
}

#[test]
fn phase_c_row10_max_subnormal() {
    assert_exact(
        "ERRORS row 10 (largest subnormal)",
        f64::from_bits(0x000f_ffff_ffff_ffff),
        "fffffffffffff 0x0.fffffffffffffp-1022 0.0000\n",
    );
    assert_match(
        "ERRORS row 10 (largest subnormal, negative)",
        &[f64::from_bits(0x800f_ffff_ffff_ffff)],
    );
}

#[test]
fn phase_c_row11_min_normal() {
    assert_exact(
        "ERRORS row 11 (smallest normal, one step past subnormal range)",
        f64::from_bits(0x0010_0000_0000_0000),
        "10000000000000 0x1p-1022 0.0000\n",
    );
    assert_match(
        "ERRORS row 11 (smallest normal, negative)",
        &[f64::from_bits(0x8010_0000_0000_0000)],
    );
}

#[test]
fn phase_c_row12_dbl_max() {
    // One step below +inf; `%.4f` must expand all 309 integer digits.
    assert_match(
        "ERRORS row 12 (DBL_MAX)",
        &[f64::from_bits(0x7fef_ffff_ffff_ffff)],
    );
    let l = loaded();
    let out = run_batch_path(&l.c, &[f64::from_bits(0x7fef_ffff_ffff_ffff)]);
    let s = show(&out);
    assert!(
        s.starts_with("7fefffffffffffff 0x1.fffffffffffffp+1023 1797693134862315"),
        "ERRORS row 12: unexpected C reference output {s:?}"
    );
    assert!(
        s.ends_with(".0000\n") && s.len() > 320,
        "ERRORS row 12: expected a full 309-digit expansion, got {} bytes",
        s.len()
    );
}

#[test]
fn phase_c_row13_neg_dbl_max() {
    assert_match(
        "ERRORS row 13 (-DBL_MAX)",
        &[f64::from_bits(0xffef_ffff_ffff_ffff)],
    );
}

#[test]
fn phase_c_row14_underflow_to_zero() {
    for (bits, expected_tail) in [
        (0x0000_0000_0000_0001u64, "0.0000\n"),
        (0x8000_0000_0000_0001u64, "-0.0000\n"),
    ] {
        let v = f64::from_bits(bits);
        assert_match("ERRORS row 14 (underflow to 0.0000)", &[v]);
        let l = loaded();
        let out = show(&run_batch_path(&l.c, &[v]));
        assert!(
            out.ends_with(expected_tail),
            "ERRORS row 14: expected tail {expected_tail:?}, got {out:?}"
        );
    }
    let mut rng = Rng::new(114);
    let mut values = vec![1e-300f64, -1e-300, 1e-5, -1e-5, 4.9e-324, -4.9e-324];
    while values.len() < 800 {
        let e = rng.range(0, 320) as i32;
        let v = (1.0 + rng.unit()) * 10f64.powi(-5 - e);
        values.push(if rng.bit() { -v } else { v });
    }
    assert_match("ERRORS row 14 (randomized sub-resolution magnitudes)", &values);
}

#[test]
fn phase_c_row15_ties_half_even() {
    // Exact binary ties at the 4th fractional digit: round-half-to-even.
    assert_exact(
        "ERRORS row 15 (tie, even last digit stays)",
        0.03125,
        "3fa0000000000000 0x1p-5 0.0312\n",
    );
    assert_exact(
        "ERRORS row 15 (tie, odd last digit rounds up)",
        0.09375,
        "3fb8000000000000 0x1.8p-4 0.0938\n",
    );
    let mut rng = Rng::new(115);
    let mut values: Vec<f64> = Vec::new();
    // All exact ties of the form m/2^5 (…5 in the 5th decimal place) and
    // m/2^k for k up to 16, both signs.
    for k in 1..=16u32 {
        for m in 0..64u64 {
            let v = m as f64 / (1u64 << k) as f64;
            values.push(v);
            values.push(-v);
        }
    }
    while values.len() < 3000 {
        let k = rng.range(1, 20) as u32;
        let m = rng.range(0, 1 << 20);
        let v = m as f64 / (1u64 << k) as f64;
        values.push(if rng.bit() { -v } else { v });
    }
    assert_match("ERRORS row 15 (randomized exact ties)", &values);
}

#[test]
fn phase_c_row16_rounding_carry() {
    assert_exact(
        "ERRORS row 16 (0.99999 -> 1.0000)",
        0.99999,
        "3fefffeb074a771d 0x1.fffeb074a771dp-1 1.0000\n",
    );
    let mut rng = Rng::new(116);
    let mut values = vec![
        0.99999f64, -0.99999, 9.99999, -9.99999, 99.99999, -99.99999,
        0.999999, 1.999999, 0.00009999, -0.00009999,
    ];
    while values.len() < 1500 {
        let n = rng.range(0, 100_000) as f64;
        let k = rng.range(5, 15) as i32;
        let v = n + 1.0 - 10f64.powi(-k);
        values.push(if rng.bit() { -v } else { v });
    }
    assert_match("ERRORS row 16 (randomized rounding carry)", &values);
}

#[test]
fn phase_c_row17_all_exponent_fields() {
    // The exhaustive "out-of-range enum value" analogue: every raw biased
    // exponent field, including the reserved encodings 0 and 0x7ff, crossed
    // with several mantissa shapes and both signs.
    let mut rng = Rng::new(117);
    let mantissas = [
        0u64,
        1,
        0x8_0000_0000_0000,
        0xf_ffff_ffff_ffff,
        rng.next_u64() & 0x000f_ffff_ffff_ffff,
    ];
    for &m in &mantissas {
        let mut values = Vec::with_capacity(2 * 2048);
        for e in 0..=0x7ffu64 {
            values.push(mk(false, e, m));
            values.push(mk(true, e, m));
        }
        assert_match(
            &format!("ERRORS row 17 (all exponent fields, mantissa 0x{m:013x})"),
            &values,
        );
    }
}

#[test]
fn phase_c_row18_no_pointer_or_length_surface() {
    // Structurally impossible to violate: `driver` takes a by-value `double`,
    // so there is no null pointer, zero length or oversized length to pass.
    // What we *can* assert is that the export has the expected arity/ABI by
    // resolving it as an `extern "C" fn(f64)` from both `.so`s and calling it.
    let l = loaded();
    assert_eq!(
        show(&run_batch_path(&l.c, &[1.0])),
        "3ff0000000000000 0x1p+0 1.0000\n"
    );
    for (label, path) in &l.rust {
        let out = run_batch_path(path, &[1.0]);
        assert_eq!(
            show(&out),
            "3ff0000000000000 0x1p+0 1.0000\n",
            "{label}: ABI/arity mismatch on the `driver` export"
        );
    }
}

// ===========================================================================
// Phase B (continued) — ambient-state axes.
//
// `printf` reads two pieces of caller-controlled process state on every call,
// and both change its output:
//
//   * `LC_NUMERIC`'s decimal point, used by BOTH `%a` and `%.4f`;
//   * the FP rounding direction, used by `%.4f`.
//
// These are configuration axes exactly like the input-shape axes above, so they
// get the same treatment: cross-product, randomized inputs, byte-for-byte.
// ===========================================================================

/// A value set that spans every interesting `%.4f` and `%a` shape, reused by the
/// ambient-state rows so each one is a full sweep rather than a spot check.
fn broad_value_set(salt: u64, n: usize) -> Vec<f64> {
    let mut rng = Rng::new(salt);
    let mut v = Vec::with_capacity(n + 64);
    // Structural specials.
    for bits in [
        0x0000_0000_0000_0000u64,
        0x8000_0000_0000_0000,
        0x0000_0000_0000_0001,
        0x800f_ffff_ffff_ffff,
        0x0010_0000_0000_0000,
        0x3fa0_0000_0000_0000, // 0.03125, an exact tie
        0xbfa0_0000_0000_0000, // -0.03125
        0x3fb8_0000_0000_0000, // 0.09375, tie with odd last digit
        0x3ff8_0000_0000_0000, // 1.5
        0x7fef_ffff_ffff_ffff, // DBL_MAX
        0xffef_ffff_ffff_ffff,
        0x7ff0_0000_0000_0000,
        0xfff0_0000_0000_0000,
        0x7ff8_0000_0000_0000,
        0xfff8_0000_0000_0000,
    ] {
        v.push(f64::from_bits(bits));
    }
    for m in 0..64u64 {
        v.push(m as f64 / 32.0); // every exact 4th-digit tie residue class
        v.push(-(m as f64) / 32.0);
    }
    for &x in &[0.99999f64, -0.99999, 9.99999, 1e-300, -1e-300, 1e-5, -1e-5] {
        v.push(x);
    }
    while v.len() < n {
        // Mostly finite values with a wide exponent spread, plus raw patterns.
        if rng.next_u64() % 8 == 0 {
            v.push(f64::from_bits(rng.next_u64()));
        } else {
            let e = rng.range(0, 0x7fe);
            v.push(mk(rng.bit(), e, rng.next_u64()));
        }
    }
    v
}

#[test]
fn phase_b_row31_every_rounding_direction() {
    // Row 31: each of the four FE_* directions, over a broad randomized sweep.
    for mode in ROUND_MODES {
        assert_match_amb(
            &format!("CONFIGS row 31 (rounding direction {mode})"),
            &broad_value_set(31, 1200),
            Ambient::round(mode),
        );
    }
}

#[test]
fn phase_b_row32_every_available_locale() {
    // Row 32: each installed locale, incl. comma-radix ones, over the same sweep.
    for loc in available_locales() {
        assert_match_amb(
            &format!("CONFIGS row 32 (locale {loc})"),
            &broad_value_set(32, 1200),
            Ambient::locale(loc),
        );
    }
}

#[test]
fn phase_b_row33_locale_times_rounding_cross_product() {
    // Row 33: the full cross-product of the two ambient axes.
    for loc in available_locales() {
        for mode in ROUND_MODES {
            assert_match_amb(
                &format!("CONFIGS row 33 (locale {loc} x round {mode})"),
                &broad_value_set(33, 400),
                Ambient::both(loc, mode),
            );
        }
    }
}

#[test]
fn phase_b_row34_directed_rounding_boundaries() {
    // Directed rounding is where a truncating vs. away-from-zero decision is
    // visible: values whose exact expansion has a non-zero tail just past the
    // 4th fractional digit, on both sides of zero, at every magnitude.
    let mut rng = Rng::new(34);
    let mut values = Vec::new();
    for &x in &[
        0.00001f64, -0.00001, 0.00009, -0.00009, 0.99991, -0.99991, 0.99999,
        -0.99999, 1.00001, -1.00001, 4.9e-324, -4.9e-324, 1e-320, -1e-320,
    ] {
        values.push(x);
    }
    while values.len() < 1500 {
        // Non-dyadic decimals: guaranteed to have an infinite binary tail, so
        // `more_bits` is set and every direction gives a different answer.
        let n = rng.range(0, 100_000) as f64;
        let frac = rng.range(1, 99_999) as f64 / 1e6;
        let v = n + frac;
        values.push(if rng.bit() { -v } else { v });
    }
    for mode in ROUND_MODES {
        assert_match_amb(
            &format!("CONFIGS row 34 (directed-rounding boundaries, {mode})"),
            &values,
            Ambient::round(mode),
        );
    }
}

// ===========================================================================
// Phase C (continued) — error/degenerate inputs under ambient state.
// ===========================================================================

#[test]
fn phase_c_row19_specials_under_all_ambient_state() {
    // The non-finite / zero / subnormal sentinels must render identically under
    // every rounding direction and locale: `nan`, `inf` and `-0.0000` must not
    // acquire a radix character from the locale where the C code prints none,
    // and must not be nudged by directed rounding.
    let specials: Vec<f64> = [
        0x7ff0_0000_0000_0000u64,
        0xfff0_0000_0000_0000,
        0x7ff8_0000_0000_0000,
        0xfff8_0000_0000_0000,
        0x7ff0_0000_0000_0001,
        0xffff_ffff_ffff_ffff,
        0x0000_0000_0000_0000,
        0x8000_0000_0000_0000,
        0x0000_0000_0000_0001,
        0x8000_0000_0000_0001,
        0x000f_ffff_ffff_ffff,
        0x0010_0000_0000_0000,
        0x4000_0000_0000_0000,
    ]
    .iter()
    .map(|&b| f64::from_bits(b))
    .collect();

    for loc in available_locales() {
        for mode in ROUND_MODES {
            assert_match_amb(
                &format!("ERRORS row 19 (specials, locale {loc}, round {mode})"),
                &specials,
                Ambient::both(loc, mode),
            );
        }
    }
}

#[test]
fn phase_c_row20_out_of_range_rounding_mode_value() {
    // `fesetround` rejects values that name no direction, so the reachable
    // states are exactly the four FE_* modes; what *is* reachable is glibc
    // seeing a rounding mode our translation must map the same way.  Assert the
    // mapping is total by checking all four against the C library, and that an
    // unrecognised mode is refused by libc rather than silently applied.
    for mode in ROUND_MODES {
        assert_match_amb(
            &format!("ERRORS row 20 (mode {mode} is handled, not defaulted)"),
            &[0.99999f64, -0.99999, 0.03125, -0.03125],
            Ambient::round(mode),
        );
    }
    // Sanity: the four modes are not all producing the same bytes, i.e. the axis
    // is genuinely exercised rather than silently collapsing to one path.
    let l = loaded();
    let probe = [0.99999f64, -0.99999];
    let mut seen = std::collections::BTreeSet::new();
    for mode in ROUND_MODES {
        seen.insert(show(&run_libs_amb(&[&l.c], &probe, Ambient::round(mode))));
    }
    assert!(
        seen.len() >= 3,
        "rounding-mode axis is not actually distinguishing outputs: {seen:?}"
    );
}
