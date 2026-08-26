//! Differential test harness: C `.so` vs Rust `.so`, both loaded with
//! `libloading` and driven **only** through their exported `extern "C"` symbols.
//!
//! The library under test (`void driver(int, int, int)`) has no return value; its
//! entire observable behaviour is the byte stream it prints to `stdout` through
//! the C runtime. Therefore every comparison here captures the process-wide
//! `stdout` file descriptor around the FFI call and compares the two captured
//! byte streams for exact equality.
//!
//! Layout:
//!   * `capture` / `Libs`   -- harness plumbing
//!   * `Rng`                -- fixed-seed xorshift64* so runs are reproducible
//!   * `cfg_row*`           -- Phase B, one function per CONFIGS.md row
//!   * `err_row*`           -- Phase C, one function per ERRORS.md row
//!   * `main`               -- runs every row sequentially, reports, sets exit code

use std::ffi::c_int;
use std::ffi::c_void;
use std::fs;
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::process::Command;

use libloading::{Library, Symbol};

/// `void driver(int x, int local_y, int z)`
type DriverFn = unsafe extern "C" fn(c_int, c_int, c_int);

// Provided by libc, which Rust `std` already links against.
unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    /// `fflush(NULL)` flushes *every* open C output stream.
    fn fflush(stream: *mut c_void) -> c_int;
}

const STDOUT_FD: c_int = 1;

// ---------------------------------------------------------------------------
// Locating the two shared libraries
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Build `c_src` with CMake if it has not been built yet, and return the C `.so`.
///
/// Nothing inside `c_src/` is modified; CMake only writes into `c_src/build/`.
fn c_so_path() -> PathBuf {
    let build_dir = manifest_dir().join("c_src/build");
    let so = build_dir.join("libdriver.so");
    if so.exists() {
        return so;
    }
    fs::create_dir_all(&build_dir).expect("create c_src/build");
    let conf = Command::new("cmake")
        .current_dir(&build_dir)
        .args(["..", "-DCMAKE_POSITION_INDEPENDENT_CODE=ON"])
        .status()
        .expect("run cmake (configure)");
    assert!(conf.success(), "cmake configure failed");
    let build = Command::new("cmake")
        .current_dir(&build_dir)
        .args(["--build", "."])
        .status()
        .expect("run cmake (build)");
    assert!(build.success(), "cmake build failed");
    assert!(so.exists(), "cmake did not produce {}", so.display());
    so
}

/// `"release"` or `"debug"`, taken from this test executable's own path so the
/// nested `cargo build` below targets the very same profile directory.
fn profile_dir_name() -> &'static str {
    let exe = std::env::current_exe().expect("current_exe");
    if exe
        .components()
        .any(|c| c.as_os_str() == std::ffi::OsStr::new("release"))
    {
        "release"
    } else {
        "debug"
    }
}

/// Feature flags to forward to the nested `cargo build`.
///
/// `Cargo.toml` declares `[features] default = []` and no other feature, so
/// `--no-default-features`, the default set, and `--all-features` all select the
/// exact same code. Passing `--no-default-features` is therefore correct for
/// every one of the three cargo invocations, and it keeps the nested build's
/// feature unification identical no matter how the tests were launched.
fn feature_args() -> Vec<&'static str> {
    let mut args = vec!["--no-default-features"];
    if profile_dir_name() == "release" {
        args.push("--release");
    }
    args
}

/// **`cargo test` does not build a cdylib-only lib target.**
///
/// An integration test can only be linked against `lib`/`rlib`/`dylib` targets,
/// so Cargo has no reason to produce `libdriver.so` for `cargo test` -- it will
/// happily run these tests against a `.so` left behind by some earlier
/// `cargo build`. That silently turns every comparison into a test of stale
/// code. So build the cdylib explicitly here, then refuse to run if the artifact
/// is still older than the sources.
fn build_rust_cdylib() {
    let status = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
        .current_dir(manifest_dir())
        .arg("build")
        .arg("--offline")
        .arg("--lib")
        .args(feature_args())
        .status()
        .expect("run `cargo build --lib` for the cdylib under test");
    assert!(status.success(), "`cargo build --lib` failed");
}

fn newest_source_mtime() -> std::time::SystemTime {
    let mut newest = std::time::SystemTime::UNIX_EPOCH;
    let mut stack = vec![manifest_dir().join("src")];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).expect("read src dir") {
            let entry = entry.expect("dir entry");
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if let Ok(m) = entry.metadata().and_then(|m| m.modified()) {
                newest = newest.max(m);
            }
        }
    }
    newest
}

/// Locate the freshly built Rust cdylib, preferring Cargo's "uplifted" copy in
/// `target/<profile>/` over the hashed one in `target/<profile>/deps/`.
fn rust_so_path() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    let mut fallback = None;
    for dir in exe.ancestors().skip(1) {
        let candidate = dir.join("libdriver.so");
        if !candidate.exists() {
            continue;
        }
        if dir.file_name() == Some(std::ffi::OsStr::new("deps")) {
            fallback.get_or_insert(candidate);
        } else {
            return candidate;
        }
    }
    fallback.unwrap_or_else(|| {
        panic!(
            "could not find the Rust cdylib `libdriver.so` near {}",
            exe.display()
        )
    })
}

/// Guard against the stale-artifact trap described on `build_rust_cdylib`.
fn assert_rust_so_is_fresh(so: &Path) {
    let so_mtime = fs::metadata(so)
        .and_then(|m| m.modified())
        .expect("stat the Rust .so");
    let src_mtime = newest_source_mtime();
    assert!(
        so_mtime >= src_mtime,
        "{} is OLDER than src/ -- the differential run would be testing stale code",
        so.display()
    );
}

struct Libs {
    _c_lib: Library,
    _rust_lib: Library,
    c_driver: DriverFn,
    rust_driver: DriverFn,
}

impl Libs {
    fn load() -> Libs {
        let c_path = c_so_path();
        let rust_path = rust_so_path();
        unsafe {
            let c_lib = Library::new(&c_path)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", c_path.display()));
            let rust_lib = Library::new(&rust_path)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", rust_path.display()));
            let c_sym: Symbol<DriverFn> = c_lib
                .get(b"driver\0")
                .expect("C .so does not export `driver`");
            let rust_sym: Symbol<DriverFn> = rust_lib
                .get(b"driver\0")
                .expect("Rust .so does not export `driver`");
            let c_driver = *c_sym;
            let rust_driver = *rust_sym;
            Libs {
                _c_lib: c_lib,
                _rust_lib: rust_lib,
                c_driver,
                rust_driver,
            }
        }
    }
}

/// Load a *fresh* instance of a library by copying it to a unique path first.
///
/// `dlopen` hands back the already-loaded handle for a path that is open, which
/// would reuse the existing `static int y`. Copying gives genuinely fresh
/// library state (`y == 123`) so the pre-first-call initial value is observable.
fn load_fresh(src: &Path, tag: &str) -> (Library, DriverFn) {
    let dst = std::env::temp_dir().join(format!("driver_fresh_{tag}_{}.so", std::process::id()));
    let _ = fs::remove_file(&dst);
    fs::copy(src, &dst).unwrap_or_else(|e| panic!("copy {} -> {}: {e}", src.display(), dst.display()));
    unsafe {
        let lib = Library::new(&dst).unwrap_or_else(|e| panic!("dlopen {}: {e}", dst.display()));
        let sym: Symbol<DriverFn> = lib.get(b"driver\0").expect("fresh .so exports `driver`");
        let f = *sym;
        (lib, f)
    }
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

/// Run `f` with fd 1 redirected to a temporary file and return the bytes written.
///
/// Both libraries print through the *same* glibc `stdout` FILE (they share
/// `libc.so.6` with this process), so flushing C streams on both sides of the
/// redirect is what makes the capture exact.
fn capture<F: FnOnce()>(f: F) -> Vec<u8> {
    // Nothing of ours may be sitting in a buffer when the swap happens.
    std::io::stdout().flush().ok();
    unsafe {
        fflush(std::ptr::null_mut());
    }

    let path = std::env::temp_dir().join(format!("driver_capture_{}.txt", std::process::id()));
    let mut file = fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(true)
        .open(&path)
        .expect("open capture file");

    let saved = unsafe { dup(STDOUT_FD) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(
        unsafe { dup2(file.as_raw_fd(), STDOUT_FD) } >= 0,
        "dup2 onto fd 1 failed"
    );

    f();

    // Flush the C streams *before* fd 1 goes back to its old target, otherwise
    // buffered bytes would be written to the restored destination instead.
    unsafe {
        fflush(std::ptr::null_mut());
        assert!(dup2(saved, STDOUT_FD) >= 0, "dup2 restore failed");
        close(saved);
    }

    let mut out = Vec::new();
    file.seek(SeekFrom::Start(0)).expect("rewind capture file");
    file.read_to_end(&mut out).expect("read capture file");
    out
}

// ---------------------------------------------------------------------------
// Comparison helpers
// ---------------------------------------------------------------------------

type RowResult = Result<(), String>;

fn show(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).escape_debug().to_string()
}

/// Call both libraries with the same argument list and require byte equality.
///
/// A batch is captured as a single stream, which also pins down message ordering
/// and stdio chunking across consecutive calls.
fn compare_batch(libs: &Libs, args: &[(i32, i32, i32)]) -> RowResult {
    let c_out = capture(|| {
        for &(x, y, z) in args {
            unsafe { (libs.c_driver)(x, y, z) };
        }
    });
    let rust_out = capture(|| {
        for &(x, y, z) in args {
            unsafe { (libs.rust_driver)(x, y, z) };
        }
    });

    if c_out.is_empty() {
        return Err(format!(
            "capture harness produced NO output for the C library with args {args:?} -- \
             the comparison would be vacuous"
        ));
    }
    if c_out != rust_out {
        let head = args.iter().take(8).collect::<Vec<_>>();
        return Err(format!(
            "output mismatch for args {head:?}{}\n     C: \"{}\"\n  Rust: \"{}\"",
            if args.len() > 8 { " (truncated)" } else { "" },
            show(&c_out),
            show(&rust_out)
        ));
    }
    Ok(())
}

/// Compare one triple at a time so a divergence names the exact input.
fn compare_each(libs: &Libs, args: &[(i32, i32, i32)]) -> RowResult {
    for &a in args {
        compare_batch(libs, &[a])?;
    }
    Ok(())
}

/// Additionally pin the exact bytes the C library is expected to print, so a
/// silently broken capture cannot make a row pass by comparing "" with "".
fn compare_expect(libs: &Libs, arg: (i32, i32, i32), expected: &str) -> RowResult {
    let (x, y, z) = arg;
    let c_out = capture(|| unsafe { (libs.c_driver)(x, y, z) });
    let rust_out = capture(|| unsafe { (libs.rust_driver)(x, y, z) });
    if c_out != expected.as_bytes() {
        return Err(format!(
            "C library output for {arg:?} does not match the transcript derived from driver.c\n\
             expected: \"{}\"\n  actual: \"{}\"",
            expected.escape_debug(),
            show(&c_out)
        ));
    }
    if rust_out != c_out {
        return Err(format!(
            "output mismatch for {arg:?}\n     C: \"{}\"\n  Rust: \"{}\"",
            show(&c_out),
            show(&rust_out)
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*)
// ---------------------------------------------------------------------------

struct Rng(u64);

impl Rng {
    fn new() -> Rng {
        Rng(0x2545_F491_4F6C_DD1D)
    }
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    /// Uniform over the whole `i32` range.
    fn next_i32(&mut self) -> i32 {
        (self.next_u64() >> 32) as u32 as i32
    }
    /// Uniform over the whole `i32` range except `forbidden`.
    fn next_i32_except(&mut self, forbidden: i32) -> i32 {
        loop {
            let v = self.next_i32();
            if v != forbidden {
                return v;
            }
        }
    }
    fn pick<T: Copy>(&mut self, items: &[T]) -> T {
        items[(self.next_u64() % items.len() as u64) as usize]
    }
}

/// Values worth hammering: the accepted constants, their neighbours, extremes.
const INTERESTING: [i32; 11] = [
    i32::MIN,
    i32::MIN + 1,
    -2,
    -1,
    0,
    1,
    2,
    3,
    4,
    i32::MAX - 1,
    i32::MAX,
];

const SAMPLES: usize = 200;

// Transcripts derived straight from c_src/src/driver.c.
const T_OK: &str = "Ok!\nResult: 0\n";
const T_X: &str = "Error: x != 1\nOperation failed\nResult: 1\n";
const T_Y: &str = "Error: x == 1 but y != 2\nOperation failed\nResult: 2\n";
const T_Z: &str = "Error: x == 1 and y == 2, but z != 3\nOperation failed\nResult: 3\n";

// ===========================================================================
// Phase B -- CONFIGS.md rows
// ===========================================================================

/// Row 1: x == 1, y == 2, z == 3 -- the single success path.
fn cfg_row01_all_match(libs: &Libs) -> RowResult {
    compare_expect(libs, (1, 2, 3), T_OK)?;
    // Repeat: the success path must stay reachable and stateless.
    compare_each(libs, &[(1, 2, 3); 8])
}

/// Row 2: x == 1, y == 2, z != 3 (randomized z).
fn cfg_row02_x1_y2_zbad(libs: &Libs) -> RowResult {
    let mut rng = Rng::new();
    let args: Vec<_> = (0..SAMPLES)
        .map(|_| (1, 2, rng.next_i32_except(3)))
        .collect();
    compare_expect(libs, (1, 2, 0), T_Z)?;
    compare_each(libs, &args)
}

/// Row 3: x == 1, y != 2, z == 3 (randomized y).
fn cfg_row03_x1_ybad_z3(libs: &Libs) -> RowResult {
    let mut rng = Rng::new();
    let args: Vec<_> = (0..SAMPLES)
        .map(|_| (1, rng.next_i32_except(2), 3))
        .collect();
    compare_expect(libs, (1, 0, 3), T_Y)?;
    compare_each(libs, &args)
}

/// Row 4: x == 1, y != 2, z != 3 (both randomized) -- the y check must win.
fn cfg_row04_x1_ybad_zbad(libs: &Libs) -> RowResult {
    let mut rng = Rng::new();
    let args: Vec<_> = (0..SAMPLES)
        .map(|_| (1, rng.next_i32_except(2), rng.next_i32_except(3)))
        .collect();
    compare_each(libs, &args)
}

/// Row 5: x != 1, y == 2, z == 3 (randomized x).
fn cfg_row05_xbad_y2_z3(libs: &Libs) -> RowResult {
    let mut rng = Rng::new();
    let args: Vec<_> = (0..SAMPLES)
        .map(|_| (rng.next_i32_except(1), 2, 3))
        .collect();
    compare_expect(libs, (0, 2, 3), T_X)?;
    compare_each(libs, &args)
}

/// Row 6: x != 1, y == 2, z != 3.
fn cfg_row06_xbad_y2_zbad(libs: &Libs) -> RowResult {
    let mut rng = Rng::new();
    let args: Vec<_> = (0..SAMPLES)
        .map(|_| (rng.next_i32_except(1), 2, rng.next_i32_except(3)))
        .collect();
    compare_each(libs, &args)
}

/// Row 7: x != 1, y != 2, z == 3.
fn cfg_row07_xbad_ybad_z3(libs: &Libs) -> RowResult {
    let mut rng = Rng::new();
    let args: Vec<_> = (0..SAMPLES)
        .map(|_| (rng.next_i32_except(1), rng.next_i32_except(2), 3))
        .collect();
    compare_each(libs, &args)
}

/// Row 8: nothing matches.
fn cfg_row08_all_bad(libs: &Libs) -> RowResult {
    let mut rng = Rng::new();
    let args: Vec<_> = (0..SAMPLES)
        .map(|_| {
            (
                rng.next_i32_except(1),
                rng.next_i32_except(2),
                rng.next_i32_except(3),
            )
        })
        .collect();
    compare_each(libs, &args)
}

/// Compare a fresh instance of each library over a call sequence.
fn compare_fresh(sequence: &[(i32, i32, i32)], tag: &str) -> RowResult {
    let c_src = c_so_path();
    let rust_src = rust_so_path();
    let (c_lib, c_fn) = load_fresh(&c_src, &format!("c_{tag}"));
    let (rust_lib, rust_fn) = load_fresh(&rust_src, &format!("rust_{tag}"));

    let c_out = capture(|| {
        for &(x, y, z) in sequence {
            unsafe { c_fn(x, y, z) };
        }
    });
    let rust_out = capture(|| {
        for &(x, y, z) in sequence {
            unsafe { rust_fn(x, y, z) };
        }
    });
    drop(c_lib);
    drop(rust_lib);

    if c_out.is_empty() {
        return Err("fresh-load capture produced no C output".to_string());
    }
    if c_out != rust_out {
        return Err(format!(
            "fresh-load mismatch for {sequence:?}\n     C: \"{}\"\n  Rust: \"{}\"",
            show(&c_out),
            show(&rust_out)
        ));
    }
    Ok(())
}

/// Row 9: very first call after a fresh load -- `static int y = 123` is live
/// until `driver` overwrites it.
fn cfg_row09_first_call_after_fresh_load(_libs: &Libs) -> RowResult {
    compare_fresh(&[(1, 2, 3)], "r09")
}

/// Row 10: first call with `local_y == 123`, i.e. equal to the static
/// initializer -- distinguishes "initial value read" from "assignment happened".
fn cfg_row10_first_call_y_equals_initial(_libs: &Libs) -> RowResult {
    compare_fresh(&[(1, 123, 3)], "r10a")?;
    // And the mirror: a fresh instance whose first call passes y == 2 must still
    // succeed even though the static was 123 a moment earlier.
    compare_fresh(&[(1, 2, 3), (1, 123, 3), (1, 2, 3)], "r10b")
}

/// Row 11: long randomized session against one loaded instance -- catches
/// divergent handling of the persistent `static y`.
fn cfg_row11_session_state_carryover(libs: &Libs) -> RowResult {
    let mut rng = Rng::new();
    let session: Vec<_> = (0..400)
        .map(|_| {
            (
                rng.pick(&INTERESTING),
                rng.pick(&INTERESTING),
                rng.pick(&INTERESTING),
            )
        })
        .collect();
    // Whole session as one stream: ordering and carry-over must match exactly.
    compare_batch(libs, &session)?;
    // A fresh pair of instances must reproduce the same session too.
    compare_fresh(&session, "r11")
}

/// Row 12: full 11x11x11 grid of boundary magnitudes.
fn cfg_row12_boundary_grid(libs: &Libs) -> RowResult {
    let mut args = Vec::with_capacity(INTERESTING.len().pow(3));
    for &x in &INTERESTING {
        for &y in &INTERESTING {
            for &z in &INTERESTING {
                args.push((x, y, z));
            }
        }
    }
    assert_eq!(args.len(), 1331);
    compare_each(libs, &args)
}

/// Row 13: alternating success / failure calls.
fn cfg_row13_alternating_good_bad(libs: &Libs) -> RowResult {
    let mut rng = Rng::new();
    let mut args = Vec::new();
    for _ in 0..60 {
        args.push((1, 2, 3));
        args.push((
            rng.pick(&INTERESTING),
            rng.pick(&INTERESTING),
            rng.pick(&INTERESTING),
        ));
    }
    compare_batch(libs, &args)?;
    compare_each(libs, &args)
}

/// Row 14: unrestricted uniform fuzz over the whole 32-bit domain.
fn cfg_row14_unrestricted_fuzz(libs: &Libs) -> RowResult {
    let mut rng = Rng::new();
    let args: Vec<_> = (0..SAMPLES * 2)
        .map(|_| (rng.next_i32(), rng.next_i32(), rng.next_i32()))
        .collect();
    compare_each(libs, &args)
}

/// Row 15: x pinned to 1 and y/z drawn from a biased set, so stages 2 and 3 are
/// hit densely instead of almost always tripping on stage 1.
fn cfg_row15_deep_stage_biased_fuzz(libs: &Libs) -> RowResult {
    let mut rng = Rng::new();
    let biased = [2i32, 3, 0, 1, -1, i32::MIN, i32::MAX];
    let args: Vec<_> = (0..SAMPLES * 2)
        .map(|_| (1, rng.pick(&biased), rng.pick(&biased)))
        .collect();
    compare_each(libs, &args)
}

/// Row 16: many calls inside one capture with no intervening flush -- checks the
/// stdio chunking of the Rust `printf` route against the C `printf`/`puts` mix.
fn cfg_row16_unflushed_bulk_interleaving(libs: &Libs) -> RowResult {
    let mut rng = Rng::new();
    let args: Vec<_> = (0..600)
        .map(|_| {
            (
                rng.pick(&[1i32, 0, 1, 1]),
                rng.pick(&[2i32, 5, 2, 2]),
                rng.pick(&[3i32, 7, 3, 3]),
            )
        })
        .collect();
    compare_batch(libs, &args)
}

// ===========================================================================
// Phase C -- ERRORS.md rows
// ===========================================================================

/// Row 1: x != 1.
fn err_row01_x_not_1(libs: &Libs) -> RowResult {
    compare_expect(libs, (0, 2, 3), T_X)?;
    let mut rng = Rng::new();
    let args: Vec<_> = (0..SAMPLES)
        .map(|_| (rng.next_i32_except(1), 2, 3))
        .collect();
    compare_each(libs, &args)
}

/// Row 2: x == 1 && y != 2.
fn err_row02_y_not_2(libs: &Libs) -> RowResult {
    compare_expect(libs, (1, 0, 3), T_Y)?;
    let mut rng = Rng::new();
    let args: Vec<_> = (0..SAMPLES)
        .map(|_| (1, rng.next_i32_except(2), 3))
        .collect();
    compare_each(libs, &args)
}

/// Row 3: x == 1 && y == 2 && z != 3.
fn err_row03_z_not_3(libs: &Libs) -> RowResult {
    compare_expect(libs, (1, 2, 0), T_Z)?;
    let mut rng = Rng::new();
    let args: Vec<_> = (0..SAMPLES)
        .map(|_| (1, 2, rng.next_i32_except(3)))
        .collect();
    compare_each(libs, &args)
}

/// Row 4: x check must win over the y check (only one Error line).
fn err_row04_x_check_wins_over_y(libs: &Libs) -> RowResult {
    compare_expect(libs, (0, 0, 3), T_X)?;
    compare_expect(libs, (i32::MIN, i32::MAX, 3), T_X)
}

/// Row 5: x check must win over the z check.
fn err_row05_x_check_wins_over_z(libs: &Libs) -> RowResult {
    compare_expect(libs, (2, 2, 4), T_X)?;
    compare_expect(libs, (i32::MAX, 2, i32::MIN), T_X)
}

/// Row 6: y check must win over the z check.
fn err_row06_y_check_wins_over_z(libs: &Libs) -> RowResult {
    compare_expect(libs, (1, 1, 4), T_Y)?;
    compare_expect(libs, (1, i32::MIN, i32::MAX), T_Y)
}

/// Row 7: x one step either side of the only accepted value.
fn err_row07_x_off_by_one(libs: &Libs) -> RowResult {
    compare_expect(libs, (0, 2, 3), T_X)?;
    compare_expect(libs, (2, 2, 3), T_X)?;
    compare_expect(libs, (-1, 2, 3), T_X)
}

/// Row 8: y one step either side of 2.
fn err_row08_y_off_by_one(libs: &Libs) -> RowResult {
    compare_expect(libs, (1, 1, 3), T_Y)?;
    compare_expect(libs, (1, 3, 3), T_Y)?;
    compare_expect(libs, (1, -2, 3), T_Y)
}

/// Row 9: z one step either side of 3.
fn err_row09_z_off_by_one(libs: &Libs) -> RowResult {
    compare_expect(libs, (1, 2, 2), T_Z)?;
    compare_expect(libs, (1, 2, 4), T_Z)?;
    compare_expect(libs, (1, 2, -3), T_Z)
}

/// Row 10: extreme ints in every position. No range check exists in the C, so
/// these must behave exactly like any other non-matching value.
fn err_row10_extreme_ints(libs: &Libs) -> RowResult {
    let extremes = [i32::MIN, i32::MIN + 1, -1, 0, i32::MAX - 1, i32::MAX];
    let mut args = Vec::new();
    for &v in &extremes {
        args.push((v, 2, 3));
        args.push((1, v, 3));
        args.push((1, 2, v));
        args.push((v, v, v));
    }
    compare_each(libs, &args)?;
    // Pin the transcripts for the extremes in each slot.
    compare_expect(libs, (i32::MIN, 2, 3), T_X)?;
    compare_expect(libs, (i32::MAX, 2, 3), T_X)?;
    compare_expect(libs, (1, i32::MIN, 3), T_Y)?;
    compare_expect(libs, (1, i32::MAX, 3), T_Y)?;
    compare_expect(libs, (1, 2, i32::MIN), T_Z)?;
    compare_expect(libs, (1, 2, i32::MAX), T_Z)
}

/// Row 11: an argument triple with no valid meaning at all -- the C-enum
/// analogue for this API, where each parameter has exactly one accepted value.
fn err_row11_no_valid_variant(libs: &Libs) -> RowResult {
    compare_expect(libs, (0x7fff_ffff, i32::MIN, 12345), T_X)?;
    let mut rng = Rng::new();
    let args: Vec<_> = (0..SAMPLES)
        .map(|_| {
            (
                rng.next_i32_except(1),
                rng.next_i32_except(2),
                rng.next_i32_except(3),
            )
        })
        .collect();
    compare_each(libs, &args)
}

/// Row 12: `driver` returns `void`; it has no failure channel and must return
/// normally for every input, including the ones that print an error.
fn err_row12_void_return_never_traps(libs: &Libs) -> RowResult {
    // If either library aborted / longjmp'd / panicked across FFI, the process
    // would die here rather than reach the sentinel comparison below.
    let mut rng = Rng::new();
    let args: Vec<_> = (0..SAMPLES)
        .map(|_| (rng.next_i32(), rng.next_i32(), rng.next_i32()))
        .collect();
    compare_batch(libs, &args)?;
    // Still alive and still in agreement afterwards.
    compare_expect(libs, (1, 2, 3), T_OK)
}

/// Row 13: the `y = local_y` assignment happens before validation and is never
/// rolled back on an error path. Prove both libraries carry that same state.
fn err_row13_state_not_rolled_back(libs: &Libs) -> RowResult {
    // Error call sets y = 99 as a side effect, then a call whose only difference
    // is a *matching* y proves the static was written, not restored.
    let seq = [
        (0, 99, 3),   // fails at stage 1, but y := 99
        (1, 2, 3),    // y := 2  -> Ok!
        (1, 99, 3),   // y := 99 -> stage 2 error
        (1, 2, 999),  // y := 2  -> stage 3 error
        (1, 2, 3),    // Ok! again
    ];
    compare_batch(libs, &seq)?;
    compare_each(libs, &seq)?;
    compare_fresh(&seq, "r13")
}

// ===========================================================================
// Runner
// ===========================================================================

type Row = (&'static str, fn(&Libs) -> RowResult);

fn main() {
    build_rust_cdylib();
    let rust_so = rust_so_path();
    assert_rust_so_is_fresh(&rust_so);

    let libs = Libs::load();

    println!("C    .so: {}", c_so_path().display());
    println!("Rust .so: {}", rust_so.display());
    println!();

    let rows: &[Row] = &[
        // Phase B -- CONFIGS.md
        ("cfg_row01_all_match", cfg_row01_all_match),
        ("cfg_row02_x1_y2_zbad", cfg_row02_x1_y2_zbad),
        ("cfg_row03_x1_ybad_z3", cfg_row03_x1_ybad_z3),
        ("cfg_row04_x1_ybad_zbad", cfg_row04_x1_ybad_zbad),
        ("cfg_row05_xbad_y2_z3", cfg_row05_xbad_y2_z3),
        ("cfg_row06_xbad_y2_zbad", cfg_row06_xbad_y2_zbad),
        ("cfg_row07_xbad_ybad_z3", cfg_row07_xbad_ybad_z3),
        ("cfg_row08_all_bad", cfg_row08_all_bad),
        (
            "cfg_row09_first_call_after_fresh_load",
            cfg_row09_first_call_after_fresh_load,
        ),
        (
            "cfg_row10_first_call_y_equals_initial",
            cfg_row10_first_call_y_equals_initial,
        ),
        (
            "cfg_row11_session_state_carryover",
            cfg_row11_session_state_carryover,
        ),
        ("cfg_row12_boundary_grid", cfg_row12_boundary_grid),
        (
            "cfg_row13_alternating_good_bad",
            cfg_row13_alternating_good_bad,
        ),
        ("cfg_row14_unrestricted_fuzz", cfg_row14_unrestricted_fuzz),
        (
            "cfg_row15_deep_stage_biased_fuzz",
            cfg_row15_deep_stage_biased_fuzz,
        ),
        (
            "cfg_row16_unflushed_bulk_interleaving",
            cfg_row16_unflushed_bulk_interleaving,
        ),
        // Phase C -- ERRORS.md
        ("err_row01_x_not_1", err_row01_x_not_1),
        ("err_row02_y_not_2", err_row02_y_not_2),
        ("err_row03_z_not_3", err_row03_z_not_3),
        (
            "err_row04_x_check_wins_over_y",
            err_row04_x_check_wins_over_y,
        ),
        (
            "err_row05_x_check_wins_over_z",
            err_row05_x_check_wins_over_z,
        ),
        (
            "err_row06_y_check_wins_over_z",
            err_row06_y_check_wins_over_z,
        ),
        ("err_row07_x_off_by_one", err_row07_x_off_by_one),
        ("err_row08_y_off_by_one", err_row08_y_off_by_one),
        ("err_row09_z_off_by_one", err_row09_z_off_by_one),
        ("err_row10_extreme_ints", err_row10_extreme_ints),
        ("err_row11_no_valid_variant", err_row11_no_valid_variant),
        (
            "err_row12_void_return_never_traps",
            err_row12_void_return_never_traps,
        ),
        (
            "err_row13_state_not_rolled_back",
            err_row13_state_not_rolled_back,
        ),
    ];

    let mut failures = Vec::new();
    for &(name, f) in rows {
        match f(&libs) {
            Ok(()) => println!("test {name} ... ok"),
            Err(msg) => {
                println!("test {name} ... FAILED");
                failures.push((name, msg));
            }
        }
    }

    println!();
    if failures.is_empty() {
        println!("differential result: ok. {} rows passed.", rows.len());
    } else {
        println!("failures:");
        for (name, msg) in &failures {
            println!("---- {name} ----\n{msg}\n");
        }
        println!(
            "differential result: FAILED. {} passed; {} failed.",
            rows.len() - failures.len(),
            failures.len()
        );
        std::process::exit(1);
    }
}
