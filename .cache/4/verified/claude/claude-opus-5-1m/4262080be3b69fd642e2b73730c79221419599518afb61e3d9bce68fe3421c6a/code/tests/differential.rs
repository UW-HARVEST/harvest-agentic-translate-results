//! Differential tests: the C shared library vs. the Rust shared library.
//!
//! Both libraries are loaded at run time with `libloading` and driven **only**
//! through their exported C symbols (`run`, `driver`). The Rust functions are
//! never called directly, so the `#[no_mangle] extern "C"` wrappers and the
//! `cdylib` ABI are part of what is under test.
//!
//! The library's only observable output is what `print_the_house` writes to
//! stdout via libc `printf`, and its only state is the file-scope
//! `static house_t the_house`, which persists across calls. Tests therefore:
//!
//!   * obtain *fresh* state by `dlopen`ing a byte-copy of the `.so` under a
//!     unique path (a distinct inode, so glibc loads an independent instance
//!     with its own copy of the globals — asserted by `cfg_c0_*`);
//!   * capture stdout at the file-descriptor level (`dup`/`dup2` + `fflush(NULL)`),
//!     which is the only way to see output produced inside libc;
//!   * compare the captured bytes of C and Rust after *every* call.
//!
//! The target sets `harness = false` and runs the cases sequentially from its
//! own `main`, so results never depend on `--test-threads`.

#![allow(clippy::missing_safety_doc)]

use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::fs;
use std::io::Write;
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

// ---------------------------------------------------------------------------
// libc bits needed to capture what the loaded libraries print
// ---------------------------------------------------------------------------

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    /// `fflush(NULL)` flushes *all* open output streams, including the `stdout`
    /// buffer that both `.so`s write into through `printf`.
    fn fflush(stream: *mut c_void) -> c_int;
}

/// Serializes fd-1/fd-2 redirection. The runner is sequential, so this is a
/// belt-and-braces guard for anyone calling the helpers from a thread.
static IO_LOCK: Mutex<()> = Mutex::new(());
/// Makes every temporary path unique.
static UNIQ: AtomicU64 = AtomicU64::new(0);

fn uniq() -> u64 {
    UNIQ.fetch_add(1, Ordering::SeqCst)
}

fn tmp_dir() -> PathBuf {
    std::env::var_os("TMPDIR")
        .map(PathBuf::from)
        .filter(|p| p.is_dir())
        .unwrap_or_else(std::env::temp_dir)
}

// ---------------------------------------------------------------------------
// Locating the two shared objects
// ---------------------------------------------------------------------------

/// Newest modification time among the sources under `dir` with one of `exts`.
fn newest_source_mtime(dir: &PathBuf, exts: &[&str]) -> std::time::SystemTime {
    fn walk(dir: &PathBuf, exts: &[&str], newest: &mut std::time::SystemTime) {
        let Ok(entries) = fs::read_dir(dir) else { return };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                // `c_src/build` holds build output, not source.
                if path.file_name().is_some_and(|n| n == "build" || n == "target") {
                    continue;
                }
                walk(&path, exts, newest);
            } else if path
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| exts.contains(&e))
            {
                if let Ok(m) = entry.metadata().and_then(|m| m.modified()) {
                    if m > *newest {
                        *newest = m;
                    }
                }
            }
        }
    }
    let mut newest = std::time::UNIX_EPOCH;
    walk(dir, exts, &mut newest);
    newest
}

/// Guards against the trap that `cargo test` does **not** rebuild a
/// `crate-type = ["cdylib"]` artifact: a stale `.so` would make every
/// differential test pass vacuously. Verified by mutation testing.
fn assert_not_stale(so: &PathBuf, src_dir: &PathBuf, exts: &[&str], rebuild_hint: &str) {
    let so_mtime = fs::metadata(so)
        .and_then(|m| m.modified())
        .unwrap_or_else(|e| panic!("stat {}: {e}", so.display()));
    let src_mtime = newest_source_mtime(src_dir, exts);
    assert!(
        so_mtime >= src_mtime,
        "STALE ARTIFACT: {} is older than the sources in {}.\n\
         Differential results would be meaningless. Rebuild with:\n  {}",
        so.display(),
        src_dir.display(),
        rebuild_hint
    );
}

fn c_so_path() -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let p = root.join("c_src/build/libdriver.so");
    assert!(
        p.is_file(),
        "C shared library not found at {}\nbuild it with:\n  cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    assert_not_stale(
        &p,
        &root.join("c_src"),
        &["c", "h"],
        "cd c_src/build && cmake --build .",
    );
    p
}

fn rust_so_path() -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut candidate: Option<PathBuf> = None;

    // The test executable lives in target/<profile>/deps/, the cdylib in
    // target/<profile>/.
    if let Ok(exe) = std::env::current_exe() {
        if let Some(profile_dir) = exe.parent().and_then(|deps| deps.parent()) {
            let p = profile_dir.join("libdriver.so");
            if p.is_file() {
                candidate = Some(p);
            }
        }
    }
    if candidate.is_none() {
        for profile in ["debug", "release"] {
            let p = root.join("target").join(profile).join("libdriver.so");
            if p.is_file() {
                candidate = Some(p);
                break;
            }
        }
    }

    let p = candidate.expect(
        "Rust cdylib libdriver.so not found. `cargo test` does not build cdylib artifacts; \
         run `cargo build` (or ./run_all.sh) first",
    );
    // `cargo test` leaves the cdylib untouched, so an out-of-date `.so` is the
    // most likely reason for a suspiciously green run. Fail loudly instead.
    assert_not_stale(
        &p,
        &root.join("src"),
        &["rs"],
        "cargo build   # cargo test does NOT rebuild cdylib artifacts",
    );
    p
}

// ---------------------------------------------------------------------------
// Entry points of the shared ABI
// ---------------------------------------------------------------------------

/// The complete set of symbols exported by the C `.so` (see `SYMBOLS.md`).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Entry {
    /// `void run(int extra_bedrooms)` — the low-level entry point (not in `driver.h`).
    Run,
    /// `void driver(int x)` — the public wrapper, calls `run` twice.
    Driver,
}

impl Entry {
    fn sym(self) -> &'static [u8] {
        match self {
            Entry::Run => b"run\0",
            Entry::Driver => b"driver\0",
        }
    }
    fn name(self) -> &'static str {
        match self {
            Entry::Run => "run",
            Entry::Driver => "driver",
        }
    }
    /// Number of `print_the_house` lines one call is expected to emit.
    fn lines(self) -> usize {
        match self {
            Entry::Run => 4,
            Entry::Driver => 8,
        }
    }
}

const BOTH_ENTRIES: [Entry; 2] = [Entry::Run, Entry::Driver];

// ---------------------------------------------------------------------------
// A freshly loaded library instance (its own copy of `the_house`)
// ---------------------------------------------------------------------------

struct Instance {
    lib: Library,
    which: &'static str,
}

impl Instance {
    /// `dlopen`s a private byte-copy of `src` so the instance starts from the
    /// initial value of `static house_t the_house`.
    fn fresh(which: &'static str, src: &PathBuf) -> Instance {
        let dst = tmp_dir().join(format!(
            "driverdiff_{}_{}_{}.so",
            which,
            std::process::id(),
            uniq()
        ));
        fs::copy(src, &dst).unwrap_or_else(|e| panic!("copy {} -> {}: {e}", src.display(), dst.display()));
        // Default libloading flags are RTLD_LAZY | RTLD_LOCAL, so the two
        // libraries cannot interpose each other's `run`.
        let lib = unsafe { Library::new(&dst) }
            .unwrap_or_else(|e| panic!("dlopen {}: {e}", dst.display()));
        // Unlinking now keeps $TMPDIR clean; the mapping stays valid.
        let _ = fs::remove_file(&dst);
        Instance { lib, which }
    }

    fn call(&self, entry: Entry, arg: i32) {
        unsafe {
            let f: Symbol<unsafe extern "C" fn(c_int)> = self
                .lib
                .get(entry.sym())
                .unwrap_or_else(|e| panic!("{} .so does not export `{}`: {e}", self.which, entry.name()));
            f(arg as c_int);
        }
    }

    fn has_symbol(&self, sym: &[u8]) -> bool {
        unsafe {
            self.lib
                .get::<unsafe extern "C" fn()>(sym)
                .map(|_| true)
                .unwrap_or(false)
        }
    }
}

/// A C instance and a Rust instance, both in the same (fresh) state.
struct Pair {
    c: Instance,
    rust: Instance,
}

fn fresh_pair() -> Pair {
    Pair {
        c: Instance::fresh("C", &c_so_path()),
        rust: Instance::fresh("Rust", &rust_so_path()),
    }
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

/// Everything a call to the library observably produced.
#[derive(PartialEq, Eq, Clone)]
struct Output {
    stdout: Vec<u8>,
    /// Captured too, so a panic message / abort diagnostic from the Rust side
    /// (where the C side prints nothing) is caught as a divergence.
    stderr: Vec<u8>,
}

/// Runs `f` with fds 1 and 2 redirected to temporary files and returns what was
/// written. This is the only way to observe output produced inside libc.
fn capture<F: FnOnce()>(f: F) -> Output {
    let _guard = IO_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let id = uniq();
    let pid = std::process::id();
    let out_path = tmp_dir().join(format!("driverdiff_out_{pid}_{id}.txt"));
    let err_path = tmp_dir().join(format!("driverdiff_err_{pid}_{id}.txt"));
    let out_file =
        fs::File::create(&out_path).unwrap_or_else(|e| panic!("create {}: {e}", out_path.display()));
    let err_file =
        fs::File::create(&err_path).unwrap_or_else(|e| panic!("create {}: {e}", err_path.display()));

    // Flush Rust's own buffers so harness output does not land in the files.
    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();

    unsafe {
        fflush(std::ptr::null_mut());
        let saved_out = dup(1);
        let saved_err = dup(2);
        assert!(saved_out >= 0 && saved_err >= 0, "dup of fd 1/2 failed");
        assert!(dup2(out_file.as_raw_fd(), 1) >= 0, "dup2 onto fd 1 failed");
        assert!(dup2(err_file.as_raw_fd(), 2) >= 0, "dup2 onto fd 2 failed");

        f();

        // Push everything libc buffered into the files *before* restoring.
        fflush(std::ptr::null_mut());
        assert!(dup2(saved_out, 1) >= 0, "restoring fd 1 failed");
        assert!(dup2(saved_err, 2) >= 0, "restoring fd 2 failed");
        close(saved_out);
        close(saved_err);
    }

    drop(out_file);
    drop(err_file);
    let stdout = fs::read(&out_path).unwrap_or_else(|e| panic!("read {}: {e}", out_path.display()));
    let stderr = fs::read(&err_path).unwrap_or_else(|e| panic!("read {}: {e}", err_path.display()));
    let _ = fs::remove_file(&out_path);
    let _ = fs::remove_file(&err_path);
    Output { stdout, stderr }
}

fn show(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).replace('\n', "\\n")
}

/// Calls `entry(arg)` on both instances, asserts byte-identical stdout *and*
/// stderr, and returns the (shared) stdout.
fn step(pair: &Pair, entry: Entry, arg: i32, ctx: &str) -> Vec<u8> {
    let c_out = capture(|| pair.c.call(entry, arg));
    let rust_out = capture(|| pair.rust.call(entry, arg));

    assert_eq!(
        c_out.stdout,
        rust_out.stdout,
        "\nstdout divergence in {ctx}: {}({arg}) [arg bits 0x{:08x}]\n  C   : {}\n  Rust: {}\n",
        entry.name(),
        arg as u32,
        show(&c_out.stdout),
        show(&rust_out.stdout)
    );
    // The C library never writes to stderr; neither may the Rust one (a panic
    // message or overflow diagnostic would show up here).
    assert_eq!(
        c_out.stderr,
        rust_out.stderr,
        "\nstderr divergence in {ctx}: {}({arg})\n  C   : {}\n  Rust: {}\n",
        entry.name(),
        show(&c_out.stderr),
        show(&rust_out.stderr)
    );
    assert!(
        c_out.stderr.is_empty(),
        "\n{ctx}: unexpected stderr output: {}",
        show(&c_out.stderr)
    );
    assert_eq!(
        c_out.stdout.iter().filter(|b| **b == b'\n').count(),
        entry.lines(),
        "\n{ctx}: {}({arg}) produced unexpected line count: {}",
        entry.name(),
        show(&c_out.stdout)
    );
    c_out.stdout
}

/// Runs a whole sequence of calls on one fresh pair, comparing after each call.
fn run_sequence(seq: &[(Entry, i32)], ctx: &str) -> Vec<Vec<u8>> {
    let pair = fresh_pair();
    seq.iter()
        .enumerate()
        .map(|(i, (entry, arg))| step(&pair, *entry, *arg, &format!("{ctx} step {i}")))
        .collect()
}

// ---------------------------------------------------------------------------
// deterministic PRNG (fixed seed => reproducible)
// ---------------------------------------------------------------------------

const SEED: u64 = 0x5EED_1234_ABCD_0001;

struct Rng(u64);

impl Rng {
    fn new(salt: u64) -> Rng {
        Rng(SEED ^ salt.wrapping_mul(0x9E37_79B9_7F4A_7C15))
    }
    fn next_u64(&mut self) -> u64 {
        // splitmix64
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn i32_full(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }
    /// Uniform in `[-bound, bound]`.
    fn i32_small(&mut self, bound: i32) -> i32 {
        let span = 2u64 * bound as u64 + 1;
        (self.next_u64() % span) as i64 as i32 - bound
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
    fn entry(&mut self) -> Entry {
        if self.next_u64() & 1 == 0 {
            Entry::Run
        } else {
            Entry::Driver
        }
    }
    /// A mix of interesting and random values.
    fn mixed_arg(&mut self) -> i32 {
        match self.below(10) {
            0 => 0,
            1 => 1,
            2 => -1,
            3 => i32::MAX,
            4 => i32::MIN,
            5 => i32::MAX - 1,
            6 => i32::MIN + 1,
            7 => self.i32_small(5),
            8 => self.i32_small(1_000_000),
            _ => self.i32_full(),
        }
    }
}

// ===========================================================================
// Phase A / D — symbol parity through the loaders and through `nm -D`
// ===========================================================================

fn sym_exported_symbols_match() {
    let pair = fresh_pair();
    for entry in BOTH_ENTRIES {
        assert!(pair.c.has_symbol(entry.sym()), "C .so misses `{}`", entry.name());
        assert!(
            pair.rust.has_symbol(entry.sym()),
            "Rust .so misses `{}`",
            entry.name()
        );
    }
    // `static` C entities must stay unexported on both sides.
    for hidden in [
        &b"add_floor\0"[..],
        b"add_bedrooms\0",
        b"add_floor_to_the_house\0",
        b"print_the_house\0",
        b"the_house\0",
    ] {
        assert_eq!(
            pair.c.has_symbol(hidden),
            pair.rust.has_symbol(hidden),
            "visibility of hidden symbol {:?} differs between C and Rust",
            String::from_utf8_lossy(hidden)
        );
    }
}

fn sym_nm_dynamic_symbol_sets_are_equal() {
    fn defined(path: &PathBuf) -> Vec<String> {
        let out = std::process::Command::new("nm")
            .args(["-D", "--defined-only"])
            .arg(path)
            .output()
            .expect("running nm");
        assert!(out.status.success(), "nm failed on {}", path.display());
        let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|l| l.split_whitespace().last().map(str::to_string))
            .collect();
        v.sort();
        v.dedup();
        v
    }

    let c = defined(&c_so_path());
    let rust = defined(&rust_so_path());
    assert_eq!(c, vec!["driver".to_string(), "run".to_string()]);
    let missing: Vec<&String> = c.iter().filter(|s| !rust.contains(s)).collect();
    assert!(missing.is_empty(), "symbols missing from Rust .so: {missing:?}");
    assert_eq!(c, rust, "dynamic symbol sets differ");
}

// ===========================================================================
// Phase B — valid-path rows from CONFIGS.md
// ===========================================================================

/// C0: fresh instances really are fresh, and the two libraries are isolated.
fn cfg_c0_fresh_state_isolation() {
    const FIRST_LINE: &str = "The house has 2 floors, 5 bedrooms, and 2.5 bathrooms\n";

    // Two independent fresh pairs must produce identical first output.
    let a = run_sequence(&[(Entry::Run, 0)], "C0/a");
    let b = run_sequence(&[(Entry::Run, 0)], "C0/b");
    assert_eq!(a, b, "a freshly dlopen'd copy did not start from the initial state");
    assert!(
        String::from_utf8_lossy(&a[0]).starts_with(FIRST_LINE),
        "unexpected initial state: {}",
        show(&a[0])
    );

    // Within one pair the state must advance (proving the singleton persists)…
    let pair = fresh_pair();
    let s1 = step(&pair, Entry::Run, 0, "C0/persist 1");
    let s2 = step(&pair, Entry::Run, 0, "C0/persist 2");
    assert_ne!(s1, s2, "state did not persist across calls");

    // …while a brand-new pair is unaffected by the calls above.
    let fresh_again = run_sequence(&[(Entry::Run, 0)], "C0/independent");
    assert_eq!(fresh_again[0], s1, "instances are not independent");
}

fn cfg_c1_run_fresh_zero() {
    run_sequence(&[(Entry::Run, 0)], "C1");
}

fn cfg_c2_run_fresh_plus_minus_one() {
    for arg in [1, -1] {
        run_sequence(&[(Entry::Run, arg)], "C2");
    }
}

fn cfg_c3_run_fresh_random_small() {
    let mut rng = Rng::new(3);
    for i in 0..200 {
        let arg = rng.i32_small(1_000);
        run_sequence(&[(Entry::Run, arg)], &format!("C3/{i}"));
    }
}

fn cfg_c4_run_fresh_random_full() {
    let mut rng = Rng::new(4);
    for i in 0..200 {
        let arg = rng.i32_full();
        run_sequence(&[(Entry::Run, arg)], &format!("C4/{i}"));
    }
}

fn cfg_c5_driver_fresh_zero() {
    let out = run_sequence(&[(Entry::Driver, 0)], "C5");
    assert_eq!(out[0].iter().filter(|b| **b == b'\n').count(), 8);
}

fn cfg_c6_driver_fresh_random_small() {
    let mut rng = Rng::new(6);
    for i in 0..200 {
        let arg = rng.i32_small(1_000);
        run_sequence(&[(Entry::Driver, arg)], &format!("C6/{i}"));
    }
}

fn cfg_c7_driver_fresh_random_full() {
    let mut rng = Rng::new(7);
    for i in 0..200 {
        let arg = rng.i32_full();
        run_sequence(&[(Entry::Driver, arg)], &format!("C7/{i}"));
    }
}

fn cfg_c8_run_sequence_small() {
    let mut rng = Rng::new(8);
    let seq: Vec<(Entry, i32)> = (0..300).map(|_| (Entry::Run, rng.i32_small(1_000))).collect();
    run_sequence(&seq, "C8");
}

fn cfg_c9_run_sequence_full() {
    let mut rng = Rng::new(9);
    let seq: Vec<(Entry, i32)> = (0..300).map(|_| (Entry::Run, rng.i32_full())).collect();
    run_sequence(&seq, "C9");
}

fn cfg_c10_driver_sequence_small() {
    let mut rng = Rng::new(10);
    let seq: Vec<(Entry, i32)> = (0..300).map(|_| (Entry::Driver, rng.i32_small(1_000))).collect();
    run_sequence(&seq, "C10");
}

fn cfg_c11_driver_sequence_full() {
    let mut rng = Rng::new(11);
    let seq: Vec<(Entry, i32)> = (0..300).map(|_| (Entry::Driver, rng.i32_full())).collect();
    run_sequence(&seq, "C11");
}

fn cfg_c12_mixed_random_interleave() {
    let mut rng = Rng::new(12);
    let seq: Vec<(Entry, i32)> = (0..300).map(|_| (rng.entry(), rng.mixed_arg())).collect();
    run_sequence(&seq, "C12");
}

/// C13: steer `bedrooms` onto exact boundary values, then one step past each.
fn cfg_c13_bedrooms_boundary_walk() {
    // Fresh state has bedrooms = 5.
    let walk: Vec<(Entry, i32)> = vec![
        (Entry::Run, -5),          // -> 0
        (Entry::Run, -1),          // -> -1
        (Entry::Run, 2),           // -> 1
        (Entry::Run, i32::MAX - 1),// -> INT_MAX
        (Entry::Run, 1),           // -> overflow, one step past INT_MAX
        (Entry::Run, -1),          // -> back to INT_MAX
        (Entry::Run, i32::MIN),    // -> overflow
        (Entry::Run, 0),
        (Entry::Run, i32::MIN),    // -> overflow again
        (Entry::Run, -1),          // one step past INT_MIN
        (Entry::Run, i32::MAX),
    ];
    run_sequence(&walk, "C13");
}

/// C14: `floors`/`bathrooms` grow through 1-, 2-, 3- and 4-digit `%d`/`%.1f` shapes.
fn cfg_c14_digit_width_growth() {
    let seq: Vec<(Entry, i32)> = (0..1_200).map(|_| (Entry::Run, 0)).collect();
    let outs = run_sequence(&seq, "C14");
    let last = String::from_utf8_lossy(outs.last().unwrap()).to_string();
    assert!(
        last.contains("1202 floors") && last.contains("1202.5 bathrooms"),
        "unexpected accumulated state: {}",
        last.replace('\n', "\\n")
    );
}

fn cfg_c15_driver_twice() {
    let mut rng = Rng::new(15);
    for i in 0..60 {
        let arg = rng.i32_full();
        run_sequence(&[(Entry::Driver, arg), (Entry::Driver, arg)], &format!("C15/{i}"));
    }
}

/// C16: `run`-then-`driver` vs `driver`-then-`run` from fresh state.
fn cfg_c16_order_sensitivity() {
    let mut rng = Rng::new(16);
    for i in 0..40 {
        let arg = rng.mixed_arg();
        let a = run_sequence(&[(Entry::Run, arg), (Entry::Driver, arg)], &format!("C16/rd/{i}"));
        let b = run_sequence(&[(Entry::Driver, arg), (Entry::Run, arg)], &format!("C16/dr/{i}"));
        // Both orders already compared C vs Rust; they must also differ from
        // each other, which proves the sequences really exercise distinct state.
        assert_ne!(a[0], b[0], "run and driver produced the same first output");
    }
}

/// C17: `bathrooms` at large magnitude still formats identically under `%.1f`.
fn cfg_c17_bathrooms_magnitude() {
    let seq: Vec<(Entry, i32)> = (0..2_000).map(|_| (Entry::Run, 1)).collect();
    let outs = run_sequence(&seq, "C17");
    let last = String::from_utf8_lossy(outs.last().unwrap()).to_string();
    assert!(
        last.contains("2002.5 bathrooms"),
        "unexpected bathrooms magnitude: {}",
        last.replace('\n', "\\n")
    );
}

/// C18: raw bit patterns × both entry points, each from a fresh instance.
fn cfg_c18_raw_bit_pattern_matrix() {
    const PATTERNS: [u32; 8] = [
        0x0000_0000,
        0x0000_0001,
        0x7FFF_FFFE,
        0x7FFF_FFFF,
        0x8000_0000,
        0x8000_0001,
        0xDEAD_BEEF,
        0xFFFF_FFFF,
    ];
    for entry in BOTH_ENTRIES {
        for bits in PATTERNS {
            run_sequence(
                &[(entry, bits as i32)],
                &format!("C18/{}/0x{bits:08x}", entry.name()),
            );
        }
    }
}

/// C19: property-style sweep over the full cross-product of the axes.
fn cfg_c19_property_sweep() {
    let mut rng = Rng::new(19);
    for i in 0..60 {
        let len = 1 + rng.below(8) as usize;
        let seq: Vec<(Entry, i32)> = (0..len).map(|_| (rng.entry(), rng.mixed_arg())).collect();
        run_sequence(&seq, &format!("C19/{i}"));
    }
}

/// C20: process-level flush-at-exit parity.
///
/// Every other case flushes explicitly with `fflush(NULL)`. Here a child process
/// loads one library, calls it and simply returns from `main`, so the buffered
/// `printf` output is only written by libc's exit handling. The Rust translation
/// deliberately calls libc `printf` (instead of `std::io::stdout`) so that this
/// buffering/flush-at-exit behaviour matches; this case proves it.
fn cfg_c20_exit_flush_parity() {
    fn child_output(which: &str, entry: Entry, arg: i32) -> Vec<u8> {
        let exe = std::env::current_exe().expect("current_exe");
        let out = std::process::Command::new(exe)
            .env(CHILD_ENV, format!("{which},{},{arg}", entry.name()))
            .output()
            .expect("spawning child");
        assert!(
            out.status.success(),
            "child ({which}, {}, {arg}) failed: {:?}, stderr: {}",
            entry.name(),
            out.status,
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(
            out.stderr.is_empty(),
            "child ({which}) wrote to stderr: {}",
            show(&out.stderr)
        );
        out.stdout
    }

    for entry in BOTH_ENTRIES {
        for arg in [0i32, 7, -3, i32::MAX, i32::MIN, 0xDEAD_BEEFu32 as i32] {
            let c = child_output("c", entry, arg);
            let rust = child_output("rust", entry, arg);
            assert_eq!(
                c,
                rust,
                "\nexit-flush divergence for {}({arg})\n  C   : {}\n  Rust: {}\n",
                entry.name(),
                show(&c),
                show(&rust)
            );
            assert_eq!(
                c.iter().filter(|b| **b == b'\n').count(),
                entry.lines(),
                "unexpected child line count: {}",
                show(&c)
            );
        }
    }
}

// ===========================================================================
// Phase C — error-path rows from ERRORS.md
// ===========================================================================

/// E1: `run(INT_MAX)` — signed overflow of `bedrooms` from the fresh state.
fn err_e1_run_int_max() {
    let outs = run_sequence(&[(Entry::Run, i32::MAX)], "E1");
    let text = String::from_utf8_lossy(&outs[0]).to_string();
    // 5 + INT_MAX wraps to -2147483644 (this is what the C .so does; asserted
    // only after the C/Rust byte comparison in `step` has already passed).
    assert!(
        text.contains(&format!("{} bedrooms", 5i32.wrapping_add(i32::MAX))),
        "E1 unexpected: {}",
        text.replace('\n', "\\n")
    );
}

/// E2: `run(INT_MIN)`.
fn err_e2_run_int_min() {
    let outs = run_sequence(&[(Entry::Run, i32::MIN)], "E2");
    let text = String::from_utf8_lossy(&outs[0]).to_string();
    assert!(
        text.contains(&format!("{} bedrooms", 5i32.wrapping_add(i32::MIN))),
        "E2 unexpected: {}",
        text.replace('\n', "\\n")
    );
}

/// E3: `driver(INT_MAX)` — overflow twice, the second time from wrapped state.
fn err_e3_driver_int_max() {
    let outs = run_sequence(&[(Entry::Driver, i32::MAX)], "E3");
    assert_eq!(outs[0].iter().filter(|b| **b == b'\n').count(), 8);
}

/// E4: `driver(INT_MIN)`.
fn err_e4_driver_int_min() {
    let outs = run_sequence(&[(Entry::Driver, i32::MIN)], "E4");
    assert_eq!(outs[0].iter().filter(|b| **b == b'\n').count(), 8);
}

/// E5: overflow accumulated across many calls rather than within one call.
fn err_e5_accumulated_overflow() {
    let big = i32::MAX / 3;
    let seq: Vec<(Entry, i32)> = (0..40).map(|_| (Entry::Run, big)).collect();
    run_sequence(&seq, "E5");
    // Same in the negative direction, and mixed.
    let seq: Vec<(Entry, i32)> = (0..40).map(|_| (Entry::Run, i32::MIN / 3)).collect();
    run_sequence(&seq, "E5/neg");
    let seq: Vec<(Entry, i32)> = (0..40)
        .map(|i| (Entry::Driver, if i % 2 == 0 { i32::MAX / 2 } else { i32::MIN / 2 }))
        .collect();
    run_sequence(&seq, "E5/mixed");
}

/// E6: the degenerate `0` argument leaves `bedrooms` untouched.
fn err_e6_zero_argument() {
    let outs = run_sequence(&[(Entry::Run, 0), (Entry::Driver, 0)], "E6");
    assert!(String::from_utf8_lossy(&outs[0]).contains("5 bedrooms"));
}

/// E7: every "out-of-range enum"-style raw bit pattern is in-domain for `int`.
fn err_e7_raw_bit_patterns() {
    // Values that would be invalid variants of a hypothetical C enum; C accepts
    // any int, and Rust must behave identically.
    const VALUES: [i32; 10] = [
        -1,
        i32::MAX,
        i32::MIN,
        i32::MIN + 1,
        i32::MAX - 1,
        0xDEAD_BEEFu32 as i32,
        0x8000_0000u32 as i32,
        0x7FFF_FFFF,
        12345,
        -99999,
    ];
    for entry in BOTH_ENTRIES {
        for v in VALUES {
            run_sequence(&[(entry, v)], &format!("E7/{}/{v}", entry.name()));
        }
    }
}

/// E8: land `bedrooms` exactly on 0 / -1 / INT_MIN / INT_MAX, then step past.
fn err_e8_boundary_landings() {
    // bedrooms starts at 5.
    run_sequence(&[(Entry::Run, -5), (Entry::Run, 0)], "E8/zero");
    run_sequence(&[(Entry::Run, -6), (Entry::Run, 0)], "E8/minus-one");
    run_sequence(&[(Entry::Run, i32::MAX - 5), (Entry::Run, 1)], "E8/int-max-then-past");
    run_sequence(&[(Entry::Run, i32::MIN + 5), (Entry::Run, -1)], "E8/int-min-then-past");
    run_sequence(
        &[(Entry::Driver, i32::MAX - 5), (Entry::Driver, 1)],
        "E8/driver-int-max",
    );
}

/// E14: `%.1f` of `bathrooms` after large growth stays byte-identical.
fn err_e14_bathrooms_growth() {
    let seq: Vec<(Entry, i32)> = (0..500).map(|_| (Entry::Driver, 0)).collect();
    let outs = run_sequence(&seq, "E14");
    let last = String::from_utf8_lossy(outs.last().unwrap()).to_string();
    assert!(
        last.contains("bathrooms"),
        "E14 unexpected: {}",
        last.replace('\n', "\\n")
    );
}

// ===========================================================================
// Runner
// ===========================================================================
//
// This target sets `harness = false` on purpose. The differential method has to
// redirect the process-wide fds 1 and 2 to see what libc `printf` writes, and
// libtest's own progress output goes to fd 1 from a different thread — under the
// default multi-threaded harness those progress lines land inside a capture and
// produce bogus "divergences". Running the cases sequentially from our own
// `main` makes every result deterministic and `--test-threads`-independent.

/// (case name, function) — one entry per row of `CONFIGS.md` / `ERRORS.md`.
const CASES: &[(&str, fn())] = &[
    // Phase A / D — symbol parity
    ("sym_exported_symbols_match", sym_exported_symbols_match),
    ("sym_nm_dynamic_symbol_sets_are_equal", sym_nm_dynamic_symbol_sets_are_equal),
    // Phase B — CONFIGS.md rows C0..C19
    ("cfg_c0_fresh_state_isolation", cfg_c0_fresh_state_isolation),
    ("cfg_c1_run_fresh_zero", cfg_c1_run_fresh_zero),
    ("cfg_c2_run_fresh_plus_minus_one", cfg_c2_run_fresh_plus_minus_one),
    ("cfg_c3_run_fresh_random_small", cfg_c3_run_fresh_random_small),
    ("cfg_c4_run_fresh_random_full", cfg_c4_run_fresh_random_full),
    ("cfg_c5_driver_fresh_zero", cfg_c5_driver_fresh_zero),
    ("cfg_c6_driver_fresh_random_small", cfg_c6_driver_fresh_random_small),
    ("cfg_c7_driver_fresh_random_full", cfg_c7_driver_fresh_random_full),
    ("cfg_c8_run_sequence_small", cfg_c8_run_sequence_small),
    ("cfg_c9_run_sequence_full", cfg_c9_run_sequence_full),
    ("cfg_c10_driver_sequence_small", cfg_c10_driver_sequence_small),
    ("cfg_c11_driver_sequence_full", cfg_c11_driver_sequence_full),
    ("cfg_c12_mixed_random_interleave", cfg_c12_mixed_random_interleave),
    ("cfg_c13_bedrooms_boundary_walk", cfg_c13_bedrooms_boundary_walk),
    ("cfg_c14_digit_width_growth", cfg_c14_digit_width_growth),
    ("cfg_c15_driver_twice", cfg_c15_driver_twice),
    ("cfg_c16_order_sensitivity", cfg_c16_order_sensitivity),
    ("cfg_c17_bathrooms_magnitude", cfg_c17_bathrooms_magnitude),
    ("cfg_c18_raw_bit_pattern_matrix", cfg_c18_raw_bit_pattern_matrix),
    ("cfg_c19_property_sweep", cfg_c19_property_sweep),
    ("cfg_c20_exit_flush_parity", cfg_c20_exit_flush_parity),
    // Phase C — ERRORS.md rows E1..E14
    ("err_e1_run_int_max", err_e1_run_int_max),
    ("err_e2_run_int_min", err_e2_run_int_min),
    ("err_e3_driver_int_max", err_e3_driver_int_max),
    ("err_e4_driver_int_min", err_e4_driver_int_min),
    ("err_e5_accumulated_overflow", err_e5_accumulated_overflow),
    ("err_e6_zero_argument", err_e6_zero_argument),
    ("err_e7_raw_bit_patterns", err_e7_raw_bit_patterns),
    ("err_e8_boundary_landings", err_e8_boundary_landings),
    ("err_e14_bathrooms_growth", err_e14_bathrooms_growth),
];

/// Set by `cfg_c20_exit_flush_parity` to turn this binary into a one-shot child
/// that calls a single entry point and lets libc flush stdout at exit.
const CHILD_ENV: &str = "DRIVER_DIFF_CHILD";

/// Child mode: `<c|rust>,<run|driver>,<arg>`. Prints nothing itself, so whatever
/// reaches stdout came from the loaded library.
fn child_main(spec: &str) {
    let mut parts = spec.split(',');
    let which = parts.next().expect("child spec: which");
    let entry_name = parts.next().expect("child spec: entry");
    let arg: i32 = parts.next().expect("child spec: arg").parse().expect("child spec: arg");

    let path = match which {
        "c" => c_so_path(),
        "rust" => rust_so_path(),
        other => panic!("child spec: unknown library {other}"),
    };
    let entry = match entry_name {
        "run" => Entry::Run,
        "driver" => Entry::Driver,
        other => panic!("child spec: unknown entry {other}"),
    };

    let lib = unsafe { Library::new(&path) }.unwrap_or_else(|e| panic!("dlopen {}: {e}", path.display()));
    unsafe {
        let f: Symbol<unsafe extern "C" fn(c_int)> =
            lib.get(entry.sym()).unwrap_or_else(|e| panic!("get {}: {e}", entry.name()));
        f(arg as c_int);
    }
    // Keep the library mapped and do NOT flush: returning from `main` must be
    // what pushes the buffered `printf` output out, in both implementations.
    std::mem::forget(lib);
}

/// Accepts optional substring filters; libtest-style flags such as
/// `--test-threads=1` or `--nocapture` are accepted and ignored so the target
/// stays a drop-in `cargo test` citizen.
fn parse_filters() -> Vec<String> {
    let mut filters = Vec::new();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg.starts_with('-') {
            // Flags that take a separate value.
            if matches!(arg.as_str(), "--test-threads" | "--skip" | "--format" | "--logfile") {
                let _ = args.next();
            }
            continue;
        }
        filters.push(arg);
    }
    filters
}

fn main() {
    if let Ok(spec) = std::env::var(CHILD_ENV) {
        child_main(&spec);
        return;
    }

    let filters = parse_filters();
    let selected: Vec<&(&str, fn())> = CASES
        .iter()
        .filter(|(name, _)| filters.is_empty() || filters.iter().any(|f| name.contains(f.as_str())))
        .collect();

    println!("\nrunning {} differential cases (sequential)", selected.len());
    let mut failed: Vec<&str> = Vec::new();
    let started = std::time::Instant::now();

    for (name, case) in &selected {
        print!("test {name} ... ");
        let _ = std::io::stdout().flush();
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(case)) {
            Ok(()) => println!("ok"),
            Err(_) => {
                println!("FAILED");
                failed.push(name);
            }
        }
        let _ = std::io::stdout().flush();
    }

    let passed = selected.len() - failed.len();
    println!(
        "\ntest result: {}. {passed} passed; {} failed; 0 ignored; 0 measured; {} filtered out; \
         finished in {:.2}s",
        if failed.is_empty() { "ok" } else { "FAILED" },
        failed.len(),
        CASES.len() - selected.len(),
        started.elapsed().as_secs_f64()
    );

    if !failed.is_empty() {
        println!("failures:");
        for name in &failed {
            println!("    {name}");
        }
        std::process::exit(101);
    }
}
