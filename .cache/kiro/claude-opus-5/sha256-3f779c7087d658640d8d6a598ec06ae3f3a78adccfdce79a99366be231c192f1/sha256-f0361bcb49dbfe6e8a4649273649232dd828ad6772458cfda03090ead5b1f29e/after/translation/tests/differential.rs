// Differential tests: C `libdriver.so` vs Rust `libdriver.so`.
//
// Both libraries are always exercised THROUGH THEIR `.so` EXPORTS via
// `libloading` -- never by calling Rust functions directly -- so the
// `#[no_mangle]` / `extern "C"` wrappers are part of what is under test.
//
// Two layers:
//   * `examples/harness` is spawned once per library per row; it dlopen()s the
//     library, replays a call script, and lets the library's own `printf`/`puts`
//     write to the process stdout. The parent compares the two stdout buffers
//     byte-for-byte. This is the only way to compare stdio-buffered output
//     faithfully (both runs are a fresh process with a fresh stdout stream).
//   * `dlsym_*` tests resolve every exported symbol in-process with
//     `libloading` from both libraries, so symbol parity is asserted from
//     inside the test binary too.
//
// Phase A artifacts: SYMBOLS.md, ERRORS.md, CONFIGS.md in the crate root.

use std::ffi::{c_char, c_int};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) -- fixed seed per row for reproducibility.
// ---------------------------------------------------------------------------

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
    fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    /// Uniform in `0..n` (n > 0).
    fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
    fn shuffle<T>(&mut self, v: &mut [T]) {
        for i in (1..v.len()).rev() {
            let j = self.below((i + 1) as u64) as usize;
            v.swap(i, j);
        }
    }
}

// ---------------------------------------------------------------------------
// Artifact locations
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so() -> PathBuf {
    let p = manifest_dir().join("../c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C shared library not found at {p:?}. Build it with:\n  cd c_src && mkdir -p build && cd build \\\n    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
    );
    p
}

/// Directory holding the current profile's build output (`target/debug` or
/// `target/release`), derived from the test binary's own location
/// (`target/<profile>/deps/<test>`).
fn profile_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    exe.parent()
        .and_then(Path::parent)
        .expect("target/<profile>")
        .to_path_buf()
}

/// Every Rust `cdylib` available to test. The current profile's is required;
/// the other profile is included when present so both the `panic = "abort"`
/// release artifact and the debug artifact are covered.
fn rust_sos() -> Vec<PathBuf> {
    let here = profile_dir().join("libdriver.so");
    assert!(
        here.exists(),
        "Rust cdylib not found at {here:?}. Build it with `cargo build` / `cargo build --release`."
    );
    let mut out = vec![here];
    let target_root = profile_dir().parent().expect("target/").to_path_buf();
    for other in ["debug", "release"] {
        let p = target_root.join(other).join("libdriver.so");
        if p.exists() && !out.contains(&p) {
            out.push(p);
        }
    }
    out
}

fn harness() -> PathBuf {
    let p = profile_dir().join("examples/harness");
    assert!(
        p.exists(),
        "harness not found at {p:?}; `cargo test` should have built it (cargo build --examples)"
    );
    p
}

// ---------------------------------------------------------------------------
// Script construction + differential runner
// ---------------------------------------------------------------------------

fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push(char::from_digit((b >> 4) as u32, 16).unwrap());
        s.push(char::from_digit((b & 0xF) as u32, 16).unwrap());
    }
    s
}

#[derive(Clone, Debug)]
enum Op {
    PrintIntLine(i32),
    PrintLine(Option<Vec<u8>>),
    Bad,
    Good,
    Driver(i32),
    /// `driver` invoked through an `fn(i64)` signature: exercises what the C
    /// does when the argument register's upper half is dirty.
    DriverWide(i64),
}

impl Op {
    fn encode(&self, out: &mut String) {
        match self {
            Op::PrintIntLine(v) => {
                out.push_str("printIntLine ");
                out.push_str(&v.to_string());
            }
            Op::PrintLine(None) => out.push_str("printLine NULL"),
            Op::PrintLine(Some(b)) => {
                out.push_str("printLine HEX ");
                out.push_str(&hex(b));
            }
            Op::Bad => out.push_str("bad"),
            Op::Good => out.push_str("good"),
            Op::Driver(v) => {
                out.push_str("driver ");
                out.push_str(&v.to_string());
            }
            Op::DriverWide(v) => {
                out.push_str("driverWide ");
                out.push_str(&v.to_string());
            }
        }
        out.push('\n');
    }
}

fn encode(ops: &[Op]) -> String {
    let mut s = String::new();
    for op in ops {
        op.encode(&mut s);
    }
    s
}

/// Spawn the harness against `so` and return its raw stdout bytes.
fn run(so: &Path, script: &str) -> Vec<u8> {
    let mut child = Command::new(harness())
        .arg(so)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn harness for {so:?}: {e}"));
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(script.as_bytes())
        .expect("write script");
    let out = child.wait_with_output().expect("wait harness");
    assert!(
        out.status.success(),
        "harness failed for {so:?}: status {:?}\nstderr:\n{}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );
    out.stdout
}

fn describe(bytes: &[u8]) -> String {
    let shown: Vec<u8> = bytes.iter().copied().take(400).collect();
    format!(
        "{} bytes: {:?}{}",
        bytes.len(),
        String::from_utf8_lossy(&shown),
        if bytes.len() > 400 { " ..." } else { "" }
    )
}

/// The core assertion for every row: run the identical script through the C
/// `.so` and through every Rust `.so`, and require byte-identical stdout.
#[track_caller]
fn assert_same(row: &str, ops: &[Op]) {
    let script = encode(ops);
    let c_out = run(&c_so(), &script);
    for rust in rust_sos() {
        let r_out = run(&rust, &script);
        if c_out != r_out {
            let first = c_out
                .iter()
                .zip(r_out.iter())
                .position(|(a, b)| a != b)
                .unwrap_or_else(|| c_out.len().min(r_out.len()));
            let lo = first.saturating_sub(60);
            panic!(
                "[{row}] stdout divergence between C and Rust ({rust:?})\n\
                 first differing byte index: {first}\n\
                 C   : {}\n\
                 Rust: {}\n\
                 context C   : {:?}\n\
                 context Rust: {:?}\n\
                 ops around divergence: {:?}",
                describe(&c_out),
                describe(&r_out),
                String::from_utf8_lossy(&c_out[lo..(first + 60).min(c_out.len())]),
                String::from_utf8_lossy(&r_out[lo..(first + 60).min(r_out.len())]),
                &ops[..ops.len().min(8)],
            );
        }
    }
}

// ===========================================================================
// Phase D (symbol parity) -- asserted in-process with libloading, and by
// comparing the two dynamic symbol tables.
// ===========================================================================

const EXPORTS: [&str; 5] = ["printLine", "printIntLine", "bad", "good", "driver"];

#[test]
fn dlsym_all_exports_resolve_in_both_libraries() {
    let mut libs = vec![c_so()];
    libs.extend(rust_sos());
    for so in libs {
        // SAFETY: locally built, trusted shared object.
        let lib = unsafe { libloading::Library::new(&so) }
            .unwrap_or_else(|e| panic!("dlopen {so:?}: {e}"));
        unsafe {
            let _: libloading::Symbol<unsafe extern "C" fn(*const c_char)> =
                lib.get(b"printLine\0").unwrap_or_else(|e| panic!("{so:?} printLine: {e}"));
            let _: libloading::Symbol<unsafe extern "C" fn(c_int)> = lib
                .get(b"printIntLine\0")
                .unwrap_or_else(|e| panic!("{so:?} printIntLine: {e}"));
            let _: libloading::Symbol<unsafe extern "C" fn()> =
                lib.get(b"bad\0").unwrap_or_else(|e| panic!("{so:?} bad: {e}"));
            let _: libloading::Symbol<unsafe extern "C" fn()> =
                lib.get(b"good\0").unwrap_or_else(|e| panic!("{so:?} good: {e}"));
            let _: libloading::Symbol<unsafe extern "C" fn(c_int)> =
                lib.get(b"driver\0").unwrap_or_else(|e| panic!("{so:?} driver: {e}"));
        }
    }
}

fn defined_dynamic_symbols(so: &Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed on {so:?}");
    let text = String::from_utf8_lossy(&out.stdout);
    let mut syms: Vec<String> = text
        .lines()
        .filter_map(|l| {
            let f: Vec<&str> = l.split_whitespace().collect();
            if f.len() == 3 { Some(f[2].to_string()) } else { None }
        })
        .collect();
    syms.sort();
    syms.dedup();
    syms
}

#[test]
fn symbol_parity_c_so_vs_rust_so() {
    let c = defined_dynamic_symbols(&c_so());
    assert!(!c.is_empty(), "nm -D found no defined symbols in the C .so");
    for e in EXPORTS {
        assert!(c.contains(&e.to_string()), "C .so unexpectedly lacks {e}");
    }
    for rust in rust_sos() {
        let r = defined_dynamic_symbols(&rust);
        let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
        assert!(
            missing.is_empty(),
            "Rust .so {rust:?} is missing symbols exported by the C .so: {missing:?}"
        );
    }
}

#[test]
fn feature_matrix_is_only_default() {
    // CONFIGS.md claims the crate has no [features]; keep that claim honest.
    let toml = std::fs::read_to_string(manifest_dir().join("Cargo.toml")).expect("Cargo.toml");
    assert!(
        !toml.contains("[features]"),
        "Cargo.toml grew a [features] table; Phases B and C must be re-run for every combination"
    );
}

#[test]
fn phase_a_artifacts_exist() {
    for f in ["SYMBOLS.md", "ERRORS.md", "CONFIGS.md"] {
        let p = manifest_dir().join(f);
        assert!(p.exists(), "Phase A artifact missing: {p:?}");
    }
}

// ===========================================================================
// Phase B -- CONFIGS.md rows 1..18 (valid paths, randomized)
// ===========================================================================

#[test]
fn cfg_row01_print_int_line_random_full_range() {
    let mut rng = Rng::new(0x0000_0001_C0FF_EE01);
    let ops: Vec<Op> = (0..4000).map(|_| Op::PrintIntLine(rng.next_i32())).collect();
    assert_same("CONFIGS row 1", &ops);
}

#[test]
fn cfg_row02_print_int_line_small_magnitude() {
    let mut vals: Vec<i32> = (-1000..=1000).collect();
    Rng::new(0x0000_0002_C0FF_EE02).shuffle(&mut vals);
    let ops: Vec<Op> = vals.into_iter().map(Op::PrintIntLine).collect();
    assert_same("CONFIGS row 2", &ops);
}

fn int_boundaries() -> Vec<i32> {
    let mut v = vec![
        i32::MIN,
        i32::MIN + 1,
        i32::MIN + 2,
        -1,
        0,
        1,
        i32::MAX - 2,
        i32::MAX - 1,
        i32::MAX,
    ];
    // digit-count rollovers
    let mut p: i64 = 1;
    while p <= 1_000_000_000 {
        for d in [-1i64, 0, 1] {
            let c = p + d;
            if c <= i32::MAX as i64 {
                v.push(c as i32);
            }
            let c = -p + d;
            if c >= i32::MIN as i64 {
                v.push(c as i32);
            }
        }
        p *= 10;
    }
    // power-of-two boundaries
    for k in 0..31 {
        let p = 1i64 << k;
        for d in [-1i64, 0, 1] {
            let c = p + d;
            if c <= i32::MAX as i64 {
                v.push(c as i32);
            }
            let c = -p + d;
            if c >= i32::MIN as i64 {
                v.push(c as i32);
            }
        }
    }
    v.sort_unstable();
    v.dedup();
    v
}

#[test]
fn cfg_row03_print_int_line_boundaries() {
    let ops: Vec<Op> = int_boundaries().into_iter().map(Op::PrintIntLine).collect();
    assert!(ops.len() > 100, "boundary set unexpectedly small: {}", ops.len());
    assert_same("CONFIGS row 3", &ops);
}

fn random_ascii(rng: &mut Rng, len: usize) -> Vec<u8> {
    // printable ASCII, no NUL (a C string cannot carry an interior NUL)
    (0..len).map(|_| 0x20u8 + (rng.below(95) as u8)).collect()
}

fn random_raw(rng: &mut Rng, len: usize) -> Vec<u8> {
    (0..len).map(|_| 1u8 + (rng.below(255) as u8)).collect()
}

#[test]
fn cfg_row04_print_line_random_ascii() {
    let mut rng = Rng::new(0x0000_0004_C0FF_EE04);
    let ops: Vec<Op> = (0..1000)
        .map(|_| {
            let len = 1 + rng.below(64) as usize;
            Op::PrintLine(Some(random_ascii(&mut rng, len)))
        })
        .collect();
    assert_same("CONFIGS row 4", &ops);
}

#[test]
fn cfg_row05_print_line_random_raw_bytes() {
    let mut rng = Rng::new(0x0000_0005_C0FF_EE05);
    let ops: Vec<Op> = (0..1000)
        .map(|_| {
            let len = 1 + rng.below(64) as usize;
            Op::PrintLine(Some(random_raw(&mut rng, len)))
        })
        .collect();
    assert_same("CONFIGS row 5", &ops);
}

#[test]
fn cfg_row06_print_line_length_boundaries() {
    let mut rng = Rng::new(0x0000_0006_C0FF_EE06);
    let mut ops = Vec::new();
    for len in [0usize, 1, 2, 3, 15, 16, 17, 1023, 1024, 1025, 4095, 4096, 4097, 65536] {
        ops.push(Op::PrintLine(Some(random_ascii(&mut rng, len))));
        ops.push(Op::PrintLine(Some(vec![b'A'; len])));
    }
    assert_same("CONFIGS row 6", &ops);
}

#[test]
fn cfg_row07_print_line_special_content() {
    let payloads: Vec<&[u8]> = vec![
        b"\n",
        b"a\nb",
        b"\n\n\n",
        b"\t\ttabbed",
        b"carriage\rreturn",
        b"%s",
        b"%d %i %u %x",
        b"%n%n%n",
        b"%%",
        b"100%",
        b"%.*s",
        b"back\\slash",
        b"\"quoted\"",
        b"trailing\n",
        b" leading space",
        b"\x7f",
        b"\x01\x02\x03",
        b"\xc3\xa9\xe2\x82\xac",
        b"\xff\xfe\xfd",
        b"mixed \xff %s \n end",
    ];
    let ops: Vec<Op> = payloads
        .into_iter()
        .map(|p| Op::PrintLine(Some(p.to_vec())))
        .collect();
    assert_same("CONFIGS row 7", &ops);
}

#[test]
fn cfg_row08_print_line_null_interleaved() {
    let mut rng = Rng::new(0x0000_0008_C0FF_EE08);
    let ops: Vec<Op> = (0..500)
        .map(|_| {
            if rng.below(2) == 0 {
                Op::PrintLine(None)
            } else {
                let len = rng.below(32) as usize;
                Op::PrintLine(Some(random_ascii(&mut rng, len)))
            }
        })
        .collect();
    assert_same("CONFIGS row 8", &ops);
}

#[test]
fn cfg_row09_bad_direct_single() {
    assert_same("CONFIGS row 9", &[Op::Bad]);
}

#[test]
fn cfg_row10_bad_direct_repeated() {
    let ops: Vec<Op> = (0..200).map(|_| Op::Bad).collect();
    assert_same("CONFIGS row 10", &ops);
}

#[test]
fn cfg_row11_good_direct_single() {
    assert_same("CONFIGS row 11", &[Op::Good]);
}

#[test]
fn cfg_row12_good_direct_repeated() {
    let ops: Vec<Op> = (0..200).map(|_| Op::Good).collect();
    assert_same("CONFIGS row 12", &ops);
}

#[test]
fn cfg_row13_driver_true_branch_random() {
    let mut rng = Rng::new(0x0000_0013_C0FF_EE13);
    let mut ops: Vec<Op> = Vec::new();
    while ops.len() < 500 {
        let v = rng.next_i32();
        if v != 0 {
            ops.push(Op::Driver(v));
        }
    }
    assert_same("CONFIGS row 13", &ops);
}

#[test]
fn cfg_row14_driver_false_branch() {
    let ops: Vec<Op> = (0..200).map(|_| Op::Driver(0)).collect();
    assert_same("CONFIGS row 14", &ops);
}

#[test]
fn cfg_row15_driver_mixed_branches() {
    let mut rng = Rng::new(0x0000_0015_C0FF_EE15);
    let ops: Vec<Op> = (0..1000)
        .map(|_| {
            if rng.below(2) == 0 {
                Op::Driver(0)
            } else {
                Op::Driver(rng.next_i32() | 1)
            }
        })
        .collect();
    assert_same("CONFIGS row 15", &ops);
}

#[test]
fn cfg_row16_all_entry_points_interleaved() {
    let mut rng = Rng::new(0x0000_0016_C0FF_EE16);
    let ops: Vec<Op> = (0..3000)
        .map(|_| match rng.below(6) {
            0 => Op::PrintIntLine(rng.next_i32()),
            1 => {
                let len = rng.below(48) as usize;
                Op::PrintLine(Some(random_raw(&mut rng, len)))
            }
            2 => Op::PrintLine(None),
            3 => Op::Bad,
            4 => Op::Good,
            _ => Op::Driver(rng.next_i32()),
        })
        .collect();
    assert_same("CONFIGS row 16", &ops);
}

#[test]
fn cfg_row17_bad_good_alternating() {
    let mut ops = Vec::new();
    for i in 0..400 {
        if i % 2 == 0 {
            ops.push(Op::Bad);
        } else {
            ops.push(Op::Good);
        }
    }
    assert_same("CONFIGS row 17", &ops);
}

#[test]
fn cfg_row18_puts_printf_interleaved() {
    // printLine compiles to `puts` in the C build while printIntLine uses
    // `printf`; alternating them checks the two stdio paths share one stream in
    // the same order.
    let mut rng = Rng::new(0x0000_0018_C0FF_EE18);
    let ops: Vec<Op> = (0..1000)
        .map(|i| {
            if i % 2 == 0 {
                let len = rng.below(20) as usize;
                Op::PrintLine(Some(random_ascii(&mut rng, len)))
            } else {
                Op::PrintIntLine(rng.next_i32())
            }
        })
        .collect();
    assert_same("CONFIGS row 18", &ops);
}

// ===========================================================================
// Phase C -- ERRORS.md rows 1..15 (invalid / boundary inputs)
// ===========================================================================

#[test]
fn err_row01_print_line_null() {
    // The whole rejection surface of the library: the `if (line != NULL)` guard.
    // Bracketed by markers so "produced no output" is distinguished from
    // "produced nothing at all".
    let ops = vec![
        Op::PrintIntLine(111),
        Op::PrintLine(None),
        Op::PrintIntLine(222),
    ];
    let script = encode(&ops);
    let c_out = run(&c_so(), &script);
    assert_eq!(c_out, b"111\n222\n", "C printLine(NULL) must emit nothing");
    assert_same("ERRORS row 1", &ops);
}

#[test]
fn err_row02_print_line_empty() {
    let ops = vec![Op::PrintLine(Some(Vec::new()))];
    let c_out = run(&c_so(), &encode(&ops));
    assert_eq!(c_out, b"\n", "C printLine(\"\") must emit just a newline");
    assert_same("ERRORS row 2", &ops);
}

#[test]
fn err_row03_print_line_oversized() {
    let ops = vec![Op::PrintLine(Some(vec![b'Z'; 65536]))];
    let c_out = run(&c_so(), &encode(&ops));
    assert_eq!(c_out.len(), 65537, "C must emit all 65536 bytes plus newline");
    assert_same("ERRORS row 3", &ops);
}

#[test]
fn err_row04_print_line_format_metachars() {
    let ops = vec![
        Op::PrintLine(Some(b"%s %n %d %% %p %1000000d".to_vec())),
        Op::PrintLine(Some(b"%s%s%s%s%s%s%s%s".to_vec())),
        Op::PrintLine(Some(b"%n".to_vec())),
    ];
    let c_out = run(&c_so(), &encode(&ops));
    assert!(
        c_out.starts_with(b"%s %n %d %% %p %1000000d\n"),
        "C must treat the payload as data, got {}",
        describe(&c_out)
    );
    assert_same("ERRORS row 4", &ops);
}

#[test]
fn err_row05_print_line_non_utf8() {
    let bytes: Vec<u8> = (0x80u8..=0xFF).collect();
    let ops = vec![Op::PrintLine(Some(bytes.clone()))];
    let c_out = run(&c_so(), &encode(&ops));
    assert_eq!(c_out.len(), bytes.len() + 1);
    assert_eq!(&c_out[..bytes.len()], &bytes[..]);
    assert_same("ERRORS row 5", &ops);
}

#[test]
fn err_row06_print_int_line_int_min() {
    let ops = vec![Op::PrintIntLine(i32::MIN)];
    assert_eq!(run(&c_so(), &encode(&ops)), b"-2147483648\n");
    assert_same("ERRORS row 6", &ops);
}

#[test]
fn err_row07_print_int_line_int_max() {
    let ops = vec![Op::PrintIntLine(i32::MAX)];
    assert_eq!(run(&c_so(), &encode(&ops)), b"2147483647\n");
    assert_same("ERRORS row 7", &ops);
}

#[test]
fn err_row08_driver_zero_selects_bad() {
    // driver(0) reaches the defective bad(): alloca(10) but 40 bytes written.
    let ops = vec![Op::Driver(0)];
    assert_eq!(run(&c_so(), &encode(&ops)), b"0\n");
    assert_same("ERRORS row 8", &ops);
    // ...and the same via the direct export, which must agree with driver(0).
    assert_same("ERRORS row 8 (direct)", &[Op::Bad]);
}

#[test]
fn err_row09_driver_out_of_range_enum() {
    let ops = vec![Op::Driver(-1)];
    assert_eq!(run(&c_so(), &encode(&ops)), b"0\n");
    assert_same("ERRORS row 9", &ops);
}

#[test]
fn err_row10_driver_two() {
    let ops = vec![Op::Driver(2)];
    assert_eq!(run(&c_so(), &encode(&ops)), b"0\n");
    assert_same("ERRORS row 10", &ops);
}

#[test]
fn err_row11_driver_int_min() {
    let ops = vec![Op::Driver(i32::MIN)];
    assert_same("ERRORS row 11", &ops);
}

#[test]
fn err_row12_driver_int_max() {
    let ops = vec![Op::Driver(i32::MAX)];
    assert_same("ERRORS row 12", &ops);
}

#[test]
fn err_row13_driver_truncating_value() {
    // Values whose low 32 bits are 0 but whose 64-bit form is non-zero: the
    // callee sees `int useGood == 0` and must take the `bad` branch. Routed
    // through the harness so the result is byte-compared like every other row.
    let ops = vec![
        Op::DriverWide(0x1_0000_0000),
        Op::DriverWide(0x7FFF_FFFF_0000_0000),
        Op::DriverWide(-4_294_967_296), // 0xFFFF_FFFF_0000_0000
        Op::DriverWide(0x0000_0001_0000_0001),
        Op::DriverWide(0),
        Op::Driver(0),
    ];
    let c_out = run(&c_so(), &encode(&ops));
    assert_eq!(c_out, b"0\n0\n0\n0\n0\n0\n", "unexpected C baseline");
    assert_same("ERRORS row 13", &ops);
}

#[test]
fn err_row14_bad_repeated_overrun() {
    // bad()'s 30-byte overrun, repeated, must remain survivable and identical.
    let ops: Vec<Op> = (0..2000).map(|_| Op::Bad).collect();
    let c_out = run(&c_so(), &encode(&ops));
    assert_eq!(c_out.len(), 2 * 2000, "C bad() must print \"0\\n\" per call");
    assert_same("ERRORS row 14", &ops);
}

#[test]
fn err_row15_good_repeated() {
    let ops: Vec<Op> = (0..2000).map(|_| Op::Good).collect();
    let c_out = run(&c_so(), &encode(&ops));
    assert_eq!(c_out.len(), 2 * 2000);
    assert_same("ERRORS row 15", &ops);
}

// ===========================================================================
// Extra generic FFI boundaries (beyond the tables)
// ===========================================================================

#[test]
fn extra_print_line_unterminated_tail_pressure() {
    // A string whose only NUL is the final byte, at several page-crossing
    // lengths: catches any length/bounds assumption in the translation.
    let mut ops = Vec::new();
    for len in [4094usize, 4095, 4096, 8191, 8192, 8193] {
        ops.push(Op::PrintLine(Some(vec![b'q'; len])));
    }
    assert_same("extra: page-crossing strings", &ops);
}

#[test]
fn extra_driver_every_low_byte_value() {
    // Every value in -256..=256 through `driver`, so the zero/non-zero split is
    // checked exhaustively near the boundary rather than at one point.
    let ops: Vec<Op> = (-256i32..=256).map(Op::Driver).collect();
    assert_same("extra: driver -256..=256", &ops);
}

#[test]
fn extra_stress_mixed_long_run() {
    let mut rng = Rng::new(0xDEAD_BEEF_CAFE_1234);
    let ops: Vec<Op> = (0..8000)
        .map(|_| match rng.below(5) {
            0 => Op::PrintIntLine(rng.next_i32()),
            1 => {
                let len = rng.below(80) as usize;
                Op::PrintLine(Some(random_raw(&mut rng, len)))
            }
            2 => Op::Bad,
            3 => Op::Good,
            _ => Op::Driver(rng.next_i32()),
        })
        .collect();
    assert_same("extra: 8000-op mixed stress", &ops);
}
