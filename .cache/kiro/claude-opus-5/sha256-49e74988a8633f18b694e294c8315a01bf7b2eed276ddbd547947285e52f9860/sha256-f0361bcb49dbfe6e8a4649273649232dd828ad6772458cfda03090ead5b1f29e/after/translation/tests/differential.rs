//! Differential test harness: loads BOTH the C `.so` and the Rust `.so` through
//! `libloading` and compares their stdout byte-for-byte.
//!
//! The Rust implementation is NEVER called directly — always through
//! `dlopen`/`dlsym` on `libdriver.so`, exactly as an external C consumer would,
//! so the `#[no_mangle] extern "C"` export wrapper is under test too.
//!
//! Phase B rows come from `CONFIGS.md`, Phase C rows from `ERRORS.md`.

use std::ffi::c_int;
use std::io::Write;
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;

use libloading::{Library, Symbol};

// ---------------------------------------------------------------------------
// libc bits needed to capture the *C* stdout stream (fd 1).
//
// Both shared objects write through the process-wide libc `stdout`, so the only
// faithful way to capture their output is to redirect file descriptor 1 and
// flush the C streams. `fflush(NULL)` flushes every open output stream, which
// avoids needing the `stdout` global symbol.
// ---------------------------------------------------------------------------
extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut std::ffi::c_void) -> c_int;
}

/// `sizeof(house_t)` — `int floors; int bedrooms; double bathrooms;` on LP64.
const HOUSE_SIZE: usize = 16;
/// One `driver` call prints `2 * sizeof(house_t)` hex digits plus `'\n'`.
const LINE_LEN: usize = HOUSE_SIZE * 2;

/// Fixed seed so every randomized row is reproducible.
const SEED: u64 = 0x5EED_1234_5678_9ABC;

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_DRIVER_SO") {
        return PathBuf::from(p);
    }
    let p = crate_root().join("../c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C shared library not found at {p:?}.\nBuild it with:\n  cd c_src && mkdir -p build && cd build \\\n    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
    );
    p
}

fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_DRIVER_SO") {
        return PathBuf::from(p);
    }
    // Prefer whichever cdylib exists; both profiles export the same ABI.
    let root = crate_root();
    for profile in ["release", "debug"] {
        let p = root.join("target").join(profile).join("libdriver.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "Rust cdylib not found under {:?}/target/{{release,debug}}.\nBuild it with:  cd translation && cargo build --release",
        root
    );
}

/// `cargo test` does NOT rebuild a `crate-type = ["cdylib"]` artifact — only
/// `cargo build` does. Testing a stale `libdriver.so` silently passes even when
/// `src/lib.rs` has diverged, so refuse to run unless the `.so` is newer than
/// every source it is built from. Same check for the C side.
fn assert_artifacts_fresh() {
    fn newest(paths: &[PathBuf]) -> (std::time::SystemTime, PathBuf) {
        let mut best = (std::time::SystemTime::UNIX_EPOCH, PathBuf::new());
        for p in paths {
            if let Ok(m) = std::fs::metadata(p).and_then(|m| m.modified()) {
                if m > best.0 {
                    best = (m, p.clone());
                }
            }
        }
        best
    }
    fn rs_sources(dir: &PathBuf, out: &mut Vec<PathBuf>) {
        if let Ok(rd) = std::fs::read_dir(dir) {
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() {
                    rs_sources(&p, out);
                } else if p.extension().is_some_and(|x| x == "rs") {
                    out.push(p);
                }
            }
        }
    }

    let root = crate_root();
    let mut rust_srcs = vec![root.join("Cargo.toml")];
    rs_sources(&root.join("src"), &mut rust_srcs);
    let (src_t, src_p) = newest(&rust_srcs);
    let so = rust_so_path();
    let so_t = std::fs::metadata(&so).and_then(|m| m.modified()).unwrap();
    assert!(
        so_t >= src_t,
        "STALE ARTIFACT: {so:?} is older than {src_p:?}.\n\
         `cargo test` does not rebuild a cdylib. Run:\n  cd translation && cargo build --release && cargo test --release"
    );

    let c_srcs = vec![
        root.join("../c_src/src/driver.c"),
        root.join("../c_src/include/driver.h"),
        root.join("../c_src/CMakeLists.txt"),
    ];
    let (c_src_t, c_src_p) = newest(&c_srcs);
    let c_so = c_so_path();
    let c_so_t = std::fs::metadata(&c_so).and_then(|m| m.modified()).unwrap();
    assert!(
        c_so_t >= c_src_t,
        "STALE ARTIFACT: {c_so:?} is older than {c_src_p:?}. Rebuild the C library."
    );
}

struct Impls {
    _c_lib: Library,
    _rust_lib: Library,
    c_driver: Symbol<'static, unsafe extern "C" fn(c_int)>,
    rust_driver: Symbol<'static, unsafe extern "C" fn(c_int)>,
}

impl Impls {
    fn load() -> Self {
        // SAFETY: both paths point at shared objects we just built from the
        // sources under test; neither has non-trivial global constructors.
        unsafe {
            let c_lib = Library::new(c_so_path()).expect("dlopen C libdriver.so");
            let rust_lib = Library::new(rust_so_path()).expect("dlopen Rust libdriver.so");

            let c_driver: Symbol<unsafe extern "C" fn(c_int)> =
                c_lib.get(b"driver\0").expect("dlsym driver in C .so");
            let rust_driver: Symbol<unsafe extern "C" fn(c_int)> = rust_lib
                .get(b"driver\0")
                .expect("dlsym driver in Rust .so (missing #[no_mangle] export?)");

            // Extend the symbol lifetimes to 'static: the `Library` values are
            // kept alive in the same struct and never moved out, so the code
            // pages the symbols point into stay mapped for as long as the
            // symbols are used.
            let c_driver = std::mem::transmute::<
                Symbol<unsafe extern "C" fn(c_int)>,
                Symbol<'static, unsafe extern "C" fn(c_int)>,
            >(c_driver);
            let rust_driver = std::mem::transmute::<
                Symbol<unsafe extern "C" fn(c_int)>,
                Symbol<'static, unsafe extern "C" fn(c_int)>,
            >(rust_driver);

            Impls {
                _c_lib: c_lib,
                _rust_lib: rust_lib,
                c_driver,
                rust_driver,
            }
        }
    }
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

/// Redirect fd 1 to a temporary file, run `f`, restore fd 1, return the bytes
/// written. Flushes the C streams on both sides of the redirect so nothing
/// leaks across the boundary.
///
/// fd 1 is process-global, so captures must be serialized: libtest runs test
/// functions on multiple threads by default and two concurrent redirects would
/// steal each other's output.
fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    static CAPTURE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _guard = CAPTURE_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let mut path = std::env::temp_dir();
    path.push(format!(
        "driver-diff-{}-{:?}-{}.out",
        std::process::id(),
        std::thread::current().id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));

    let bytes = {
        let file = std::fs::File::create(&path).expect("create temp capture file");

        // SAFETY: plain POSIX fd juggling on descriptors we own.
        unsafe {
            let _ = std::io::stdout().flush();
            fflush(std::ptr::null_mut());

            let saved = dup(1);
            assert!(saved >= 0, "dup(1) failed");
            assert!(dup2(file.as_raw_fd(), 1) >= 0, "dup2 onto stdout failed");

            f();

            fflush(std::ptr::null_mut());
            assert!(dup2(saved, 1) >= 0, "dup2 restoring stdout failed");
            close(saved);
        }
        drop(file);
        std::fs::read(&path).expect("read temp capture file")
    };
    let _ = std::fs::remove_file(&path);
    bytes
}

// ---------------------------------------------------------------------------
// Differential comparison
// ---------------------------------------------------------------------------

/// Split a capture into `\n`-terminated lines, asserting exact framing.
fn split_lines(label: &str, buf: &[u8], expected: usize) -> Vec<Vec<u8>> {
    assert_eq!(
        buf.len(),
        expected * (LINE_LEN + 1),
        "{label}: expected {expected} lines of {} hex digits + newline, got {} bytes: {:?}",
        LINE_LEN,
        buf.len(),
        String::from_utf8_lossy(&buf[..buf.len().min(200)])
    );
    buf.chunks(LINE_LEN + 1)
        .map(|c| {
            assert_eq!(c[LINE_LEN], b'\n', "{label}: line not newline-terminated");
            c[..LINE_LEN].to_vec()
        })
        .collect()
}

/// Row 14 of `CONFIGS.md`: output framing invariants that must hold for every
/// single call in every row — lowercase hex, exact width, and the constant
/// `bedrooms`/`bathrooms` byte fingerprint.
fn assert_framing(row: &str, value: i32, line: &[u8]) {
    assert_eq!(line.len(), LINE_LEN, "{row}: wrong line width");
    for &b in line {
        assert!(
            b.is_ascii_digit() || (b'a'..=b'f').contains(&b),
            "{row}: value {value:#010x} produced non-lowercase-hex byte {b:#04x} in {:?}",
            String::from_utf8_lossy(line)
        );
    }
    // floors is at offset 0, little-endian.
    let floors_hex: String = value
        .to_le_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    assert_eq!(
        &line[0..8],
        floors_hex.as_bytes(),
        "{row}: floors image wrong for {value:#010x}"
    );
    // bedrooms == 3 at offset 4, bathrooms == 2.0 at offset 8.
    assert_eq!(&line[8..16], b"03000000", "{row}: bedrooms image changed");
    assert_eq!(
        &line[16..32],
        b"0000000000000040",
        "{row}: bathrooms image changed"
    );
}

/// The core differential assertion: run the same value list through both `.so`s
/// and require byte-identical stdout.
fn diff_row(impls: &Impls, row: &str, values: &[i32]) {
    assert!(!values.is_empty(), "{row}: empty value list");

    let c_out = capture_stdout(|| {
        for &v in values {
            unsafe { (impls.c_driver)(v) };
        }
    });
    let rust_out = capture_stdout(|| {
        for &v in values {
            unsafe { (impls.rust_driver)(v) };
        }
    });

    // Whole-buffer equality first: catches framing/ordering differences.
    if c_out != rust_out {
        let c_lines: Vec<&[u8]> = c_out.split(|&b| b == b'\n').collect();
        let r_lines: Vec<&[u8]> = rust_out.split(|&b| b == b'\n').collect();
        for (i, (cl, rl)) in c_lines.iter().zip(r_lines.iter()).enumerate() {
            if cl != rl {
                panic!(
                    "{row}: DIVERGENCE at call #{i} (input {:#010x} / {}):\n  C   : {:?}\n  Rust: {:?}",
                    values.get(i).copied().unwrap_or_default(),
                    values.get(i).copied().unwrap_or_default(),
                    String::from_utf8_lossy(cl),
                    String::from_utf8_lossy(rl)
                );
            }
        }
        panic!(
            "{row}: DIVERGENCE in total output length: C {} bytes vs Rust {} bytes",
            c_out.len(),
            rust_out.len()
        );
    }

    // Per-call structural checks (also validates the capture itself).
    let c_lines = split_lines("C", &c_out, values.len());
    let r_lines = split_lines("Rust", &rust_out, values.len());
    for (i, &v) in values.iter().enumerate() {
        assert_framing(row, v, &c_lines[i]);
        assert_framing(row, v, &r_lines[i]);
        assert_eq!(c_lines[i], r_lines[i], "{row}: line {i} differs for {v}");
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seed, reproducible rows.
// ---------------------------------------------------------------------------
struct SplitMix64(u64);

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        SplitMix64(seed)
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
    fn next_byte(&mut self) -> u8 {
        (self.next_u64() >> 33) as u8
    }
}

// ===========================================================================
// PHASE B — valid-path differential tests (one test per CONFIGS.md row)
// ===========================================================================

fn phase_b_row_01_zero() {
    diff_row(&Impls::load(), "CONFIGS row 1 (floors == 0)", &[0]);
}

fn phase_b_row_02_small_positive() {
    diff_row(
        &Impls::load(),
        "CONFIGS row 2 (small positive)",
        &[1, 2, 3, 7, 42, 127],
    );
}

fn phase_b_row_03_small_negative() {
    diff_row(
        &Impls::load(),
        "CONFIGS row 3 (small negative)",
        &[-1, -2, -3, -42, -128],
    );
}

fn phase_b_row_04_int_max() {
    diff_row(&Impls::load(), "CONFIGS row 4 (INT_MAX)", &[i32::MAX]);
}

fn phase_b_row_05_int_min() {
    diff_row(&Impls::load(), "CONFIGS row 5 (INT_MIN)", &[i32::MIN]);
}

fn phase_b_row_06_byte_lanes() {
    let vals: Vec<i32> = [0x0000_00ffu32, 0x0000_ff00, 0x00ff_0000, 0xff00_0000, 0xffff_ffff]
        .iter()
        .map(|&u| u as i32)
        .collect();
    diff_row(&Impls::load(), "CONFIGS row 6 (per-byte lanes)", &vals);
}

fn phase_b_row_07_embedded_zero_bytes() {
    let vals: Vec<i32> = [0x00ff_00ffu32, 0xff00_ff00, 0x0001_0000]
        .iter()
        .map(|&u| u as i32)
        .collect();
    diff_row(&Impls::load(), "CONFIGS row 7 (embedded zero bytes)", &vals);
}

fn phase_b_row_08_low_nibble_padding() {
    let vals: Vec<i32> = [0x0102_0304u32, 0x0f0f_0f0f, 0x0000_0001, 0x0900_0000]
        .iter()
        .map(|&u| u as i32)
        .collect();
    diff_row(&Impls::load(), "CONFIGS row 8 (%02x zero padding)", &vals);
}

fn phase_b_row_09_high_bit_bytes() {
    let vals: Vec<i32> = [0x8080_8080u32, 0xdead_beef, 0xcafe_babe, 0xffff_ff80]
        .iter()
        .map(|&u| u as i32)
        .collect();
    diff_row(
        &Impls::load(),
        "CONFIGS row 9 (bytes >= 0x80, unsigned char promotion)",
        &vals,
    );
}

fn phase_b_row_10_powers_of_two() {
    let mut vals = Vec::new();
    for k in 0..32u32 {
        let p = 1u32 << k;
        vals.push(p as i32);
        vals.push(p.wrapping_sub(1) as i32);
        vals.push((p as i32).wrapping_neg());
    }
    diff_row(
        &Impls::load(),
        "CONFIGS row 10 (powers of two and neighbours)",
        &vals,
    );
}

fn phase_b_row_11_random_full_range() {
    let mut rng = SplitMix64::new(SEED);
    let vals: Vec<i32> = (0..4096).map(|_| rng.next_i32()).collect();
    diff_row(
        &Impls::load(),
        "CONFIGS row 11 (4096 uniform random i32, seed fixed)",
        &vals,
    );
}

fn phase_b_row_12_random_byte_biased() {
    const EDGES: [u8; 8] = [0x00, 0x01, 0x0f, 0x10, 0x7f, 0x80, 0xfe, 0xff];
    let mut rng = SplitMix64::new(SEED ^ 0xA5A5_A5A5_A5A5_A5A5);
    let vals: Vec<i32> = (0..4096)
        .map(|_| {
            let b = [
                EDGES[(rng.next_byte() % 8) as usize],
                EDGES[(rng.next_byte() % 8) as usize],
                EDGES[(rng.next_byte() % 8) as usize],
                EDGES[(rng.next_byte() % 8) as usize],
            ];
            i32::from_le_bytes(b)
        })
        .collect();
    diff_row(
        &Impls::load(),
        "CONFIGS row 12 (4096 byte-biased random i32, seed fixed)",
        &vals,
    );
}

fn phase_b_row_13_repeated_and_interleaved() {
    let impls = Impls::load();
    let mut rng = SplitMix64::new(SEED ^ 0x1357_9BDF_1357_9BDF);
    let vals: Vec<i32> = (0..512).map(|_| rng.next_i32()).collect();

    // Batch form: 512 back-to-back calls in a single captured region.
    diff_row(&impls, "CONFIGS row 13 (repeated invocation)", &vals);

    // Interleaved form: C and Rust alternate inside one capture each, so any
    // per-call state leakage would show up as a shifted line.
    let c_out = capture_stdout(|| {
        for &v in &vals {
            unsafe { (impls.c_driver)(v) };
            unsafe { (impls.c_driver)(v) };
        }
    });
    let r_out = capture_stdout(|| {
        for &v in &vals {
            unsafe { (impls.rust_driver)(v) };
            unsafe { (impls.rust_driver)(v) };
        }
    });
    assert_eq!(c_out, r_out, "CONFIGS row 13: interleaved output diverged");

    // Calling the same value twice must give identical lines (no stale state).
    let lines = split_lines("interleaved", &r_out, vals.len() * 2);
    for pair in lines.chunks(2) {
        assert_eq!(pair[0], pair[1], "CONFIGS row 13: state leaked between calls");
    }

    // Cross-check: a C call followed by a Rust call on the same value produces
    // two identical lines in the same stream.
    let mixed = capture_stdout(|| {
        for &v in vals.iter().take(64) {
            unsafe { (impls.c_driver)(v) };
            unsafe { (impls.rust_driver)(v) };
        }
    });
    let mixed_lines = split_lines("mixed", &mixed, 128);
    for (i, pair) in mixed_lines.chunks(2).enumerate() {
        assert_eq!(
            pair[0], pair[1],
            "CONFIGS row 13: C and Rust disagree in shared stream at call {i} (input {:#010x})",
            vals[i]
        );
    }
}

fn phase_b_row_14_output_framing() {
    // `assert_framing` runs inside every row above; this test pins the exact
    // expected framing independently of the C implementation, so a *shared*
    // mistake (both sides wrong the same way) would still be caught.
    let impls = Impls::load();
    let out = capture_stdout(|| unsafe { (impls.c_driver)(0) });
    // floors=0 -> "00000000", bedrooms=3 -> "03000000",
    // bathrooms=2.0 -> IEEE-754 0x4000000000000000 little-endian -> "0000000000000040"
    assert_eq!(
        out,
        b"00000000030000000000000000000040\n".to_vec(),
        "framing golden mismatch: got {:?}",
        String::from_utf8_lossy(&out)
    );
    let rout = capture_stdout(|| unsafe { (impls.rust_driver)(0) });
    assert_eq!(out, rout, "Rust framing differs from C golden");
}

fn phase_b_row_15_low_level_entry_point_is_the_only_entry_point() {
    // CONFIGS axis 3: `print_hex` is `static` and therefore absent from the C
    // `.so`'s dynamic symbol table. Assert that so the "test only the
    // convenience wrapper" blind spot is explicitly ruled out: `driver` really
    // is the lowest-level exported entry point, and there is nothing else to
    // drive directly.
    let c_lib = unsafe { Library::new(c_so_path()) }.expect("dlopen C .so");
    let r_lib = unsafe { Library::new(rust_so_path()) }.expect("dlopen Rust .so");
    let c_has: bool = unsafe { c_lib.get::<*const ()>(b"print_hex\0") }.is_ok();
    let r_has: bool = unsafe { r_lib.get::<*const ()>(b"print_hex\0") }.is_ok();
    assert!(!c_has, "C .so unexpectedly exports print_hex");
    assert_eq!(
        c_has, r_has,
        "print_hex visibility differs between C and Rust .so"
    );
}

// ===========================================================================
// PHASE C — error-path differential tests (one test per ERRORS.md row)
//
// The C library has no rejection surface (see ERRORS.md derivation: zero
// returns, asserts, null checks, range checks or error enums). The differential
// assertion for each row is therefore that BOTH implementations accept the
// input, produce byte-identical stdout, and neither aborts.
// ===========================================================================

fn phase_c_row_01_zero_boundary() {
    diff_row(&Impls::load(), "ERRORS row 1 (floors == 0)", &[0]);
}

fn phase_c_row_02_minus_one_sentinel() {
    let impls = Impls::load();
    diff_row(&impls, "ERRORS row 2 (floors == -1 sentinel)", &[-1]);
    let out = capture_stdout(|| unsafe { (impls.c_driver)(-1) });
    assert!(
        out.starts_with(b"ffffffff"),
        "ERRORS row 2: -1 image wrong: {:?}",
        String::from_utf8_lossy(&out)
    );
}

fn phase_c_row_03_int_max_boundary() {
    let impls = Impls::load();
    diff_row(&impls, "ERRORS row 3 (INT_MAX)", &[i32::MAX]);
    let out = capture_stdout(|| unsafe { (impls.rust_driver)(i32::MAX) });
    assert!(out.starts_with(b"ffffff7f"), "ERRORS row 3: INT_MAX image wrong");
}

fn phase_c_row_04_int_min_boundary() {
    let impls = Impls::load();
    diff_row(&impls, "ERRORS row 4 (INT_MIN)", &[i32::MIN]);
    let out = capture_stdout(|| unsafe { (impls.rust_driver)(i32::MIN) });
    assert!(out.starts_with(b"00000080"), "ERRORS row 4: INT_MIN image wrong");
}

fn phase_c_row_05_one_past_int_max() {
    // No representable value exists one step past INT_MAX; the wrap-around bit
    // pattern 0x80000000 is what an out-of-range caller actually delivers.
    let impls = Impls::load();
    let v = 0x8000_0000u32 as i32;
    diff_row(&impls, "ERRORS row 5 (bit pattern 0x80000000)", &[v]);
    let a = capture_stdout(|| unsafe { (impls.c_driver)(v) });
    let b = capture_stdout(|| unsafe { (impls.c_driver)(i32::MIN) });
    assert_eq!(a, b, "ERRORS row 5: 0x80000000 must equal INT_MIN");
}

fn phase_c_row_06_one_below_int_min() {
    let impls = Impls::load();
    let v = 0x7fff_ffffu32 as i32;
    diff_row(&impls, "ERRORS row 6 (bit pattern 0x7fffffff)", &[v]);
    let a = capture_stdout(|| unsafe { (impls.rust_driver)(v) });
    let b = capture_stdout(|| unsafe { (impls.rust_driver)(i32::MAX) });
    assert_eq!(a, b, "ERRORS row 6: 0x7fffffff must equal INT_MAX");
}

fn phase_c_row_07_out_of_range_enum_style_values() {
    // A C `int` parameter accepts any 32-bit value. There is no enum in the
    // source, so "no valid variant" means every bit pattern must round-trip
    // identically. Sweep every single-byte-varying pattern plus classic
    // garbage/uninitialised markers, in all four byte lanes.
    let mut vals: Vec<i32> = Vec::new();
    for lane in 0..4u32 {
        for b in 0..=255u32 {
            vals.push(((b << (8 * lane)) as u32) as i32);
        }
    }
    for &u in &[
        0xdead_beefu32,
        0xcafe_babe,
        0xbaad_f00d,
        0xfeed_face,
        0xcdcd_cdcd,
        0xdddd_dddd,
        0xabab_abab,
        0x7f7f_7f7f,
        0x8000_0001,
        0x7fff_fffe,
    ] {
        vals.push(u as i32);
    }
    diff_row(
        &Impls::load(),
        "ERRORS row 7 (out-of-range/enum-style int values across the FFI boundary)",
        &vals,
    );
}

fn phase_c_row_08_no_pointer_parameter_to_null() {
    // ERRORS row 8: the null-pointer boundary is unreachable through the public
    // ABI. Prove it structurally: the exported symbol takes one `int` and the
    // header declares no pointer-taking function.
    let hdr = std::fs::read_to_string(crate_root().join("../c_src/include/driver.h"))
        .expect("read driver.h");
    let decls: Vec<&str> = hdr
        .lines()
        .filter(|l| l.contains('(') && l.trim_end().ends_with(';'))
        .collect();
    assert_eq!(
        decls.len(),
        1,
        "ERRORS row 8: header declares more than one function: {decls:?}"
    );
    assert!(
        !decls[0].contains('*'),
        "ERRORS row 8: public API now takes a pointer — a null-pointer differential test is required: {:?}",
        decls[0]
    );
    // Both .so files still agree on the one entry point.
    diff_row(&Impls::load(), "ERRORS row 8 (structural)", &[0, -1]);
}

fn phase_c_row_09_no_length_parameter() {
    // ERRORS row 9: zero/oversized length is unreachable — no length parameter
    // exists. The only length is the compile-time `sizeof(house_t)`, which we
    // pin here: every call emits exactly 2*16 hex digits + newline.
    let impls = Impls::load();
    for v in [0, 1, -1, i32::MIN, i32::MAX] {
        let c = capture_stdout(|| unsafe { (impls.c_driver)(v) });
        let r = capture_stdout(|| unsafe { (impls.rust_driver)(v) });
        assert_eq!(c.len(), LINE_LEN + 1, "ERRORS row 9: C length changed");
        assert_eq!(r.len(), LINE_LEN + 1, "ERRORS row 9: Rust length changed");
        assert_eq!(c, r, "ERRORS row 9: divergence for {v}");
    }
}

fn phase_c_row_10_print_hex_len_is_constant_16() {
    // ERRORS row 10: `print_hex`'s `len <= 0` / `len > sizeof` paths are
    // unreachable (internal linkage, single call site with a constant). Verified
    // indirectly: the trip count is always 16 for both implementations, over a
    // randomized sweep.
    let impls = Impls::load();
    let mut rng = SplitMix64::new(SEED ^ 0x0F0F_0F0F_0F0F_0F0F);
    for _ in 0..256 {
        let v = rng.next_i32();
        let c = capture_stdout(|| unsafe { (impls.c_driver)(v) });
        let r = capture_stdout(|| unsafe { (impls.rust_driver)(v) });
        assert_eq!(c.len(), 33, "ERRORS row 10: C emitted {} bytes for {v}", c.len());
        assert_eq!(r.len(), 33, "ERRORS row 10: Rust emitted {} bytes for {v}", r.len());
        assert_eq!(c, r, "ERRORS row 10: divergence for {v:#010x}");
    }
}

fn phase_c_row_11_no_state_corruption_across_calls() {
    // ERRORS row 11: `house` is a fresh `{0}` automatic each call. Hammer the
    // same value after adversarial neighbours and require identical output.
    let impls = Impls::load();
    let adversarial = [i32::MIN, i32::MAX, -1, 0, 0x0f0f_0f0fu32 as i32];
    for &probe in &[0, 1, -1, 0x1234_5678, i32::MIN] {
        let baseline = capture_stdout(|| unsafe { (impls.c_driver)(probe) });
        for &noise in &adversarial {
            let c = capture_stdout(|| unsafe {
                (impls.c_driver)(noise);
                (impls.c_driver)(probe);
            });
            let r = capture_stdout(|| unsafe {
                (impls.rust_driver)(noise);
                (impls.rust_driver)(probe);
            });
            assert_eq!(c, r, "ERRORS row 11: divergence after noise {noise}");
            assert!(
                c.ends_with(&baseline),
                "ERRORS row 11: {probe} output changed after {noise}"
            );
        }
    }
}

// ===========================================================================
// Custom harness. Rows run strictly sequentially in a single thread so that
// nothing else can write to fd 1 while a capture is active.
// ===========================================================================

type Row = (&'static str, fn());

const ROWS: &[Row] = &[
    ("phase_b_row_01_zero", phase_b_row_01_zero),
    ("phase_b_row_02_small_positive", phase_b_row_02_small_positive),
    ("phase_b_row_03_small_negative", phase_b_row_03_small_negative),
    ("phase_b_row_04_int_max", phase_b_row_04_int_max),
    ("phase_b_row_05_int_min", phase_b_row_05_int_min),
    ("phase_b_row_06_byte_lanes", phase_b_row_06_byte_lanes),
    ("phase_b_row_07_embedded_zero_bytes", phase_b_row_07_embedded_zero_bytes),
    ("phase_b_row_08_low_nibble_padding", phase_b_row_08_low_nibble_padding),
    ("phase_b_row_09_high_bit_bytes", phase_b_row_09_high_bit_bytes),
    ("phase_b_row_10_powers_of_two", phase_b_row_10_powers_of_two),
    ("phase_b_row_11_random_full_range", phase_b_row_11_random_full_range),
    ("phase_b_row_12_random_byte_biased", phase_b_row_12_random_byte_biased),
    ("phase_b_row_13_repeated_and_interleaved", phase_b_row_13_repeated_and_interleaved),
    ("phase_b_row_14_output_framing", phase_b_row_14_output_framing),
    ("phase_b_row_15_low_level_entry_point_is_the_only_entry_point", phase_b_row_15_low_level_entry_point_is_the_only_entry_point),
    ("phase_c_row_01_zero_boundary", phase_c_row_01_zero_boundary),
    ("phase_c_row_02_minus_one_sentinel", phase_c_row_02_minus_one_sentinel),
    ("phase_c_row_03_int_max_boundary", phase_c_row_03_int_max_boundary),
    ("phase_c_row_04_int_min_boundary", phase_c_row_04_int_min_boundary),
    ("phase_c_row_05_one_past_int_max", phase_c_row_05_one_past_int_max),
    ("phase_c_row_06_one_below_int_min", phase_c_row_06_one_below_int_min),
    ("phase_c_row_07_out_of_range_enum_style_values", phase_c_row_07_out_of_range_enum_style_values),
    ("phase_c_row_08_no_pointer_parameter_to_null", phase_c_row_08_no_pointer_parameter_to_null),
    ("phase_c_row_09_no_length_parameter", phase_c_row_09_no_length_parameter),
    ("phase_c_row_10_print_hex_len_is_constant_16", phase_c_row_10_print_hex_len_is_constant_16),
    ("phase_c_row_11_no_state_corruption_across_calls", phase_c_row_11_no_state_corruption_across_calls),
];

fn main() {
    // Allow `cargo test -- <substring>` style filtering.
    let filters: Vec<String> = std::env::args()
        .skip(1)
        .filter(|a| !a.starts_with('-'))
        .collect();

    let mut passed = 0usize;
    let mut failed: Vec<&str> = Vec::new();
    let mut skipped = 0usize;

    eprintln!("\nrunning {} differential rows (C .so vs Rust .so)\n", ROWS.len());
    assert_artifacts_fresh();
    eprintln!("  C    .so: {:?}", c_so_path());
    eprintln!("  Rust .so: {:?}\n", rust_so_path());
    for (name, f) in ROWS {
        if !filters.is_empty() && !filters.iter().any(|q| name.contains(q.as_str())) {
            skipped += 1;
            continue;
        }
        eprint!("test {name} ... ");
        let _ = std::io::stderr().flush();
        // Silence the default panic hook so a failing row prints once, in order.
        let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
        match res {
            Ok(()) => {
                eprintln!("ok");
                passed += 1;
            }
            Err(_) => {
                eprintln!("FAILED");
                failed.push(name);
            }
        }
    }

    eprintln!(
        "\nresult: {}. {} passed; {} failed; {} filtered out\n",
        if failed.is_empty() { "ok" } else { "FAILED" },
        passed,
        failed.len(),
        skipped
    );
    if !failed.is_empty() {
        eprintln!("failed rows:");
        for f in &failed {
            eprintln!("    {f}");
        }
        std::process::exit(1);
    }
}
