//! Differential tests: C `libStaticAlias.so` vs Rust `libStaticAlias.so`.
//!
//! BOTH libraries are loaded with `libloading` and driven exclusively through
//! their exported `extern "C"` symbols, so the `#[no_mangle]` wrappers are part
//! of what is under test. No Rust function is ever called directly.
//!
//! ## Why everything runs under one global lock
//!
//! `static_alias` owns a function-local `static int inner = 1;`. Each `.so` has
//! its own copy of that hidden state, and both start at `1` when the process
//! starts. The suite keeps the two copies bit-identical by only ever performing
//! **lockstep pairs**: a C call is immediately followed by the same Rust call
//! while the global mutex is held. Because every mutation is applied to both
//! libraries in the same order, the two `inner` values stay equal no matter
//! which order `cargo test` happens to schedule the tests in. `stdout` (fd 1)
//! is also a process-global resource, so the same lock serialises capture.

#![allow(clippy::missing_safety_doc)]

use std::ffi::{c_int, c_void};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};

// ---------------------------------------------------------------------------
// libc bits the harness itself needs (not part of the code under test)
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(code: c_int) -> !;
}

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

type StaticAliasFn = unsafe extern "C" fn(*mut c_int) -> *mut c_int;
type DriverFn = unsafe extern "C" fn(c_int, c_int);

/// One loaded implementation of the library.
struct Impl {
    name: &'static str,
    static_alias: StaticAliasFn,
    driver: DriverFn,
    _lib: libloading::Library,
}

impl Impl {
    fn load(name: &'static str, path: &Path) -> Impl {
        let lib = unsafe { libloading::Library::new(path) }
            .unwrap_or_else(|e| panic!("failed to dlopen {} ({}): {e}", path.display(), name));
        let static_alias: StaticAliasFn = unsafe {
            *lib.get::<StaticAliasFn>(b"static_alias\0")
                .unwrap_or_else(|e| panic!("{name}: missing symbol `static_alias`: {e}"))
        };
        let driver: DriverFn = unsafe {
            *lib.get::<DriverFn>(b"driver\0")
                .unwrap_or_else(|e| panic!("{name}: missing symbol `driver`: {e}"))
        };
        Impl { name, static_alias, driver, _lib: lib }
    }
}

struct Harness {
    c: Impl,
    rs: Impl,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `<workdir>/c_src/build/libStaticAlias.so`, built by cmake if absent.
fn c_so_path() -> PathBuf {
    let root = manifest_dir().parent().expect("crate has a parent dir").to_path_buf();
    let so = root.join("c_src/build/libStaticAlias.so");
    if !so.exists() {
        let build = root.join("c_src/build");
        std::fs::create_dir_all(&build).expect("mkdir c_src/build");
        let cfg = std::process::Command::new("cmake")
            .current_dir(&build)
            .args(["..", "-DCMAKE_POSITION_INDEPENDENT_CODE=ON"])
            .status()
            .expect("run cmake");
        assert!(cfg.success(), "cmake configure failed");
        let b = std::process::Command::new("cmake")
            .current_dir(&build)
            .args(["--build", "."])
            .status()
            .expect("run cmake --build");
        assert!(b.success(), "cmake --build failed");
    }
    assert!(so.exists(), "C shared library not found at {}", so.display());
    so
}

/// The Rust `cdylib` under test.
///
/// `cargo test` does **not** build a `cdylib`-only library target (there is no
/// rlib for the integration test to link against), so relying on whatever
/// happens to sit in `target/<profile>/` silently loads a stale `.so` — and a
/// stale `.so` makes every differential assertion meaningless. The harness
/// therefore builds the library itself, into an isolated `--target-dir` so it
/// cannot contend with the `cargo test` invocation that is running it, and then
/// asserts the result is newer than the sources.
fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("STATICALIAS_SO") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "STATICALIAS_SO={} does not exist", p.display());
        return p;
    }

    let dir = manifest_dir();
    let target_dir = dir.join("target/testdylib");
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let mut cmd = std::process::Command::new(cargo);
    cmd.current_dir(&dir)
        .arg("build")
        .arg("--lib")
        .arg("--target-dir")
        .arg(&target_dir);
    // Mirror the profile of the test binary itself.
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        cmd.arg("--release");
        "release"
    };
    // Feature selection for the combination currently under test.
    if let Ok(extra) = std::env::var("STATICALIAS_CARGO_FEATURE_ARGS") {
        for a in extra.split_whitespace() {
            cmd.arg(a);
        }
    }
    let status = cmd.status().expect("failed to run cargo build --lib");
    assert!(status.success(), "cargo build --lib for the cdylib failed");

    let so = target_dir.join(profile).join("libStaticAlias.so");
    assert!(so.exists(), "cdylib not produced at {}", so.display());

    // Guard against loading something stale.
    let so_mtime = std::fs::metadata(&so).and_then(|m| m.modified()).expect("so mtime");
    let src_mtime = std::fs::metadata(dir.join("src/lib.rs"))
        .and_then(|m| m.modified())
        .expect("src mtime");
    assert!(
        so_mtime >= src_mtime,
        "{} is older than src/lib.rs — refusing to test a stale library",
        so.display()
    );
    so
}

fn harness() -> MutexGuard<'static, Harness> {
    static H: OnceLock<Mutex<Harness>> = OnceLock::new();
    H.get_or_init(|| {
        Mutex::new(Harness {
            c: Impl::load("C", &c_so_path()),
            rs: Impl::load("RUST", &rust_so_path()),
        })
    })
    .lock()
    .unwrap_or_else(|poisoned| poisoned.into_inner())
}

// ---------------------------------------------------------------------------
// Deterministic RNG (xorshift64*) — fixed seed for reproducibility
// ---------------------------------------------------------------------------

const SEED: u64 = 0x5A11_A11A_5EED_0001;

struct Rng(u64);

impl Rng {
    fn new(salt: u64) -> Rng {
        Rng(SEED ^ salt.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn i32_any(&mut self) -> i32 {
        (self.next_u64() >> 32) as u32 as i32
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
    /// A value drawn from the classes the C code distinguishes (axis C).
    fn value_class(&mut self) -> i32 {
        match self.below(10) {
            0 => i32::MIN,
            1 => i32::MAX,
            2 => 0,
            3 => 1,
            4 => -1,
            5 => (self.below(64) as i32) - 32,
            6 => self.below(1 << 20) as i32,
            7 => -(self.below(1 << 20) as i32),
            8 => i32::MAX - (self.below(4) as i32),
            _ => self.i32_any(),
        }
    }
}

// ---------------------------------------------------------------------------
// Observation helpers — identical code path applied to both implementations
// ---------------------------------------------------------------------------

/// Which object the returned pointer designates, expressed *relatively* so that
/// C and Rust observations are comparable.
///
/// * `0` — the very pointer that was passed in (`return outer;`)
/// * `1` — one of the caller's own cells, but not the one passed in
/// * `2` — an object outside the caller's cells, i.e. `&inner`
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
struct Step {
    ident: u8,
    ret_val: i32,
    cells: [i32; 4],
}

fn ident_of(ret: *mut c_int, passed: *mut c_int, cells: &[*mut c_int]) -> u8 {
    if ret == passed {
        0
    } else if cells.contains(&ret) {
        1
    } else {
        2
    }
}

/// Drive `static_alias` exactly the way `driver` does: chain the returned
/// pointer straight back into the next call. `initial` seeds the caller's cell.
fn chain(f: StaticAliasFn, initial: i32, steps: usize) -> Vec<Step> {
    let mut cell: c_int = initial;
    let cell_ptr: *mut c_int = &raw mut cell;
    let mut cur: *mut c_int = cell_ptr;
    let mut out = Vec::with_capacity(steps);
    for _ in 0..steps {
        let passed = cur;
        let ret = unsafe { f(passed) };
        let ident = ident_of(ret, passed, &[cell_ptr]);
        let ret_val = unsafe { *ret };
        out.push(Step { ident, ret_val, cells: [cell, 0, 0, 0] });
        cur = ret;
    }
    out
}

/// One call on a fresh caller-owned cell (no chaining).
fn one(f: StaticAliasFn, v: i32) -> Step {
    let mut cell: c_int = v;
    let cell_ptr: *mut c_int = &raw mut cell;
    let ret = unsafe { f(cell_ptr) };
    let ident = ident_of(ret, cell_ptr, &[cell_ptr]);
    let ret_val = unsafe { *ret };
    Step { ident, ret_val, cells: [cell, 0, 0, 0] }
}

/// Round-robin over several independent caller cells (axis B, distinct objects).
fn interleave(f: StaticAliasFn, seeds: [i32; 4], rounds: usize) -> Vec<Step> {
    let mut cells: [c_int; 4] = seeds;
    let ptrs: [*mut c_int; 4] = [
        &raw mut cells[0],
        &raw mut cells[1],
        &raw mut cells[2],
        &raw mut cells[3],
    ];
    let mut out = Vec::with_capacity(rounds * 4);
    for r in 0..rounds {
        for i in 0..4 {
            let passed = ptrs[(r + i) % 4];
            let ret = unsafe { f(passed) };
            let ident = ident_of(ret, passed, &ptrs);
            let ret_val = unsafe { *ret };
            out.push(Step { ident, ret_val, cells });
        }
    }
    out
}

/// Non-destructive probe of the private `inner`: `INT_MIN` is `< inner` for
/// every `inner > INT_MIN`, so the else-branch runs, `inner` is left untouched
/// and the caller's cell receives `INT_MIN + inner`.
///
/// Returned as a raw `Step` so that the pathological `inner == INT_MIN` case
/// (where the probe *does* mutate) is still compared between the two libraries
/// instead of being silently reinterpreted.
fn probe_inner(f: StaticAliasFn) -> Step {
    one(f, i32::MIN)
}

// ---------------------------------------------------------------------------
// stdout capture (driver prints with libc printf)
// ---------------------------------------------------------------------------

fn capture<F: FnOnce()>(f: F) -> Vec<u8> {
    static N: AtomicU32 = AtomicU32::new(0);
    let path = std::env::temp_dir().join(format!(
        "staticalias-cap-{}-{}.txt",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    let file = std::fs::File::create(&path).expect("create capture file");
    let fd = {
        use std::os::fd::AsRawFd;
        file.as_raw_fd()
    };

    // Flush anything already buffered in libc's stdout so it is not captured.
    unsafe { fflush(std::ptr::null_mut()) };
    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { dup2(fd, 1) } >= 0, "dup2 onto stdout failed");

    f();

    unsafe { fflush(std::ptr::null_mut()) };
    assert!(unsafe { dup2(saved, 1) } >= 0, "restore stdout failed");
    unsafe { close(saved) };
    drop(file);

    let bytes = std::fs::read(&path).expect("read capture file");
    let _ = std::fs::remove_file(&path);
    bytes
}

fn show(bytes: &[u8]) -> String {
    let s = String::from_utf8_lossy(bytes);
    if s.len() <= 400 {
        s.into_owned()
    } else {
        format!("{}…[{} bytes total]…{}", &s[..200], bytes.len(), &s[s.len() - 120..])
    }
}

// ---------------------------------------------------------------------------
// Lockstep differential primitives
// ---------------------------------------------------------------------------

impl Harness {
    /// Run `body` against the C impl, then against the Rust impl, and assert the
    /// observations are identical. Because both run before the lock is released
    /// the two hidden `inner` values advance in lockstep.
    fn diff<T, F>(&mut self, what: &str, body: F)
    where
        T: PartialEq + std::fmt::Debug,
        F: Fn(&Impl) -> T,
    {
        let c = body(&self.c);
        let r = body(&self.rs);
        assert_eq!(
            c, r,
            "{what}: {} and {} diverged\n  {}   = {c:?}\n  {} = {r:?}",
            self.c.name, self.rs.name, self.c.name, self.rs.name
        );
        // Hidden-state parity after every operation (CONFIGS.md row 18).
        let pc = probe_inner(self.c.static_alias);
        let pr = probe_inner(self.rs.static_alias);
        assert_eq!(pc, pr, "{what}: private `inner` diverged after the operation");
    }

    /// Same, but for operations whose observable output is the `stdout` byte
    /// stream produced by `driver`.
    fn diff_stdout<F>(&mut self, what: &str, body: F)
    where
        F: Fn(&Impl),
    {
        let c_out = capture(|| body(&self.c));
        let r_out = capture(|| body(&self.rs));
        assert!(
            c_out == r_out,
            "{what}: stdout differs ({} vs {} bytes)\n  C    = {}\n  RUST = {}",
            c_out.len(),
            r_out.len(),
            show(&c_out),
            show(&r_out)
        );
        let pc = probe_inner(self.c.static_alias);
        let pr = probe_inner(self.rs.static_alias);
        assert_eq!(pc, pr, "{what}: private `inner` diverged after the operation");
    }

    /// Current value of the private `inner`, as seen through the C library
    /// (both libraries are always in lockstep, and the probe is applied to both
    /// so they stay that way).
    ///
    /// The `INT_MIN` probe is normally non-destructive, but `inner` itself can
    /// legitimately *become* `INT_MIN` (repeated doubling walks the powers of
    /// two straight onto it). In that one case `INT_MIN >= inner` holds, the
    /// then-branch runs and `inner` becomes `INT_MIN + INT_MIN == 0`. That
    /// happens identically in both libraries, so lockstep is preserved; the
    /// probe just reports the post-call value instead.
    fn inner_now(&mut self) -> i32 {
        let pc = probe_inner(self.c.static_alias);
        let pr = probe_inner(self.rs.static_alias);
        assert_eq!(pc, pr, "private `inner` diverged");
        if pc.ident == 0 {
            // else-branch: `inner` untouched, cell holds INT_MIN + inner.
            pc.ret_val.wrapping_sub(i32::MIN)
        } else {
            // then-branch: `inner` was INT_MIN and now equals the returned value.
            pc.ret_val
        }
    }
}

// ===========================================================================
// Phase B — CONFIGS.md rows
// ===========================================================================

#[test]
fn cfg_01_then_equal_fresh() {
    let mut h = harness();
    // Row 1: *outer == inner exactly (the `==` half of `>=`), fresh cell.
    for _ in 0..8 {
        let v = h.inner_now();
        h.diff("cfg_01 *outer == inner", |i| one(i.static_alias, v));
    }
}

#[test]
fn cfg_02_then_greater_fresh_random() {
    let mut h = harness();
    let mut rng = Rng::new(2);
    for _ in 0..200 {
        let inner = h.inner_now();
        // Anything >= inner takes the then-branch.
        let delta = rng.below(1 << 20) as i32;
        let v = inner.wrapping_add(delta);
        let v = if v < inner { i32::MAX } else { v };
        h.diff("cfg_02 *outer > inner", |i| one(i.static_alias, v));
    }
}

#[test]
fn cfg_03_else_boundary_minus_one() {
    let mut h = harness();
    for _ in 0..8 {
        let v = h.inner_now().wrapping_sub(1);
        h.diff("cfg_03 *outer == inner-1", |i| one(i.static_alias, v));
    }
}

#[test]
fn cfg_04_else_below_random() {
    let mut h = harness();
    let mut rng = Rng::new(4);
    for _ in 0..200 {
        let inner = h.inner_now();
        let raw = rng.value_class();
        // Force the value strictly below `inner` so the else-branch is taken.
        let v = if raw < inner { raw } else { inner.wrapping_sub(1 + (rng.below(1000) as i32)) };
        h.diff("cfg_04 *outer < inner", |i| one(i.static_alias, v));
    }
}

#[test]
fn cfg_05_self_alias_chain() {
    let mut h = harness();
    // Row 5: feed the returned `&inner` straight back in => `inner += *(&inner)`.
    for _ in 0..40 {
        let v = h.inner_now();
        h.diff("cfg_05 self-aliased doubling", |i| chain(i.static_alias, v, 4));
    }
}

#[test]
fn cfg_06_chained_outer_flip() {
    let mut h = harness();
    // Row 6: start below `inner` (else-branch returns `outer`), then the same
    // cell now holds old+inner >= inner, so the next call flips to then.
    for _ in 0..40 {
        let v = h.inner_now().wrapping_sub(1);
        h.diff("cfg_06 chained outer then flip", |i| chain(i.static_alias, v, 3));
    }
}

#[test]
fn cfg_07_long_chain_random() {
    let mut h = harness();
    let mut rng = Rng::new(7);
    for _ in 0..20 {
        let v = rng.value_class();
        h.diff("cfg_07 long random chain", |i| chain(i.static_alias, v, 500));
    }
}

#[test]
fn cfg_08_negative_inner_states() {
    let mut h = harness();
    let mut rng = Rng::new(8);
    // Force `inner` through the overflow wrap so it becomes negative, then
    // exercise randomized values on both sides of the negative boundary.
    h.diff("cfg_08 force wrap", |i| one(i.static_alias, i32::MAX));
    h.diff("cfg_08 force wrap 2", |i| one(i.static_alias, i32::MAX));
    for _ in 0..200 {
        let inner = h.inner_now();
        let v = match rng.below(4) {
            0 => inner,
            1 => inner.wrapping_sub(1),
            2 => inner.wrapping_add(1),
            _ => rng.value_class(),
        };
        h.diff("cfg_08 negative inner", |i| one(i.static_alias, v));
    }
}

#[test]
fn cfg_09_multi_object_interleave() {
    let mut h = harness();
    let mut rng = Rng::new(9);
    for _ in 0..30 {
        let seeds = [rng.value_class(), rng.value_class(), rng.value_class(), rng.value_class()];
        h.diff("cfg_09 interleaved objects", |i| interleave(i.static_alias, seeds, 8));
    }
}

#[test]
fn cfg_10_driver_zero() {
    let mut h = harness();
    let mut rng = Rng::new(10);
    for _ in 0..20 {
        let v = rng.value_class();
        h.diff_stdout("cfg_10 driver(_, 0)", move |i| unsafe { (i.driver)(v, 0) });
    }
}

#[test]
fn cfg_11_driver_single_random() {
    let mut h = harness();
    let mut rng = Rng::new(11);
    for _ in 0..150 {
        let inner = h.inner_now();
        let v = match rng.below(6) {
            0 => inner,
            1 => inner.wrapping_sub(1),
            2 => inner.wrapping_add(1),
            _ => rng.value_class(),
        };
        h.diff_stdout("cfg_11 driver(v, 1)", move |i| unsafe { (i.driver)(v, 1) });
    }
}

#[test]
fn cfg_12_driver_two_random() {
    let mut h = harness();
    let mut rng = Rng::new(12);
    for _ in 0..150 {
        let inner = h.inner_now();
        let v = match rng.below(6) {
            0 => inner,
            1 => inner.wrapping_sub(1),
            2 => inner.wrapping_add(1),
            _ => rng.value_class(),
        };
        h.diff_stdout("cfg_12 driver(v, 2)", move |i| unsafe { (i.driver)(v, 2) });
    }
}

#[test]
fn cfg_13_driver_many_random() {
    let mut h = harness();
    let mut rng = Rng::new(13);
    for _ in 0..120 {
        let v = rng.value_class();
        let n = 3 + (rng.below(62) as c_int);
        h.diff_stdout("cfg_13 driver(v, many)", move |i| unsafe { (i.driver)(v, n) });
    }
}

#[test]
fn cfg_14_driver_else_dominant() {
    let mut h = harness();
    let mut rng = Rng::new(14);
    for _ in 0..80 {
        let inner = h.inner_now();
        // Well below `inner` so the else-branch runs first; the running sum then
        // climbs until it crosses `inner` and flips to the then-branch.
        let v = inner.wrapping_sub(1 + (rng.below(1 << 16) as i32));
        let n = 2 + (rng.below(40) as c_int);
        h.diff_stdout("cfg_14 driver else-dominant", move |i| unsafe { (i.driver)(v, n) });
    }
}

#[test]
fn cfg_15_driver_oversized() {
    let mut h = harness();
    // Row 15: oversized iteration count; `inner` overflows and wraps mid-stream.
    for v in [1, -1, i32::MIN, i32::MAX] {
        h.diff_stdout("cfg_15 driver oversized", move |i| unsafe { (i.driver)(v, 100_000) });
    }
}

#[test]
fn cfg_16_mixed_pipeline_random() {
    let mut h = harness();
    let mut rng = Rng::new(16);
    // Row 16: alternate low-level and wrapper calls so each observes state left
    // behind by the other (the composed-pipeline bug class).
    for _ in 0..100 {
        let a = rng.value_class();
        let b = rng.value_class();
        let n = rng.below(9) as c_int;
        let steps = 1 + rng.below(5) as usize;
        h.diff("cfg_16 low-level leg", move |i| chain(i.static_alias, a, steps));
        h.diff_stdout("cfg_16 wrapper leg", move |i| unsafe { (i.driver)(b, n) });
        h.diff("cfg_16 low-level leg 2", move |i| one(i.static_alias, a));
    }
}

#[test]
fn cfg_17_driver_after_negative_inner() {
    let mut h = harness();
    let mut rng = Rng::new(17);
    h.diff("cfg_17 force wrap", |i| one(i.static_alias, i32::MAX));
    h.diff("cfg_17 force wrap 2", |i| one(i.static_alias, i32::MAX));
    for _ in 0..80 {
        let inner = h.inner_now();
        let v = if rng.below(2) == 0 { inner.wrapping_add(rng.i32_any()) } else { rng.value_class() };
        let n = rng.below(24) as c_int;
        h.diff_stdout("cfg_17 driver after negative inner", move |i| unsafe { (i.driver)(v, n) });
    }
}

#[test]
fn cfg_18_hidden_inner_parity() {
    let mut h = harness();
    let mut rng = Rng::new(18);
    // Row 18: the private `inner` itself must match. `diff`/`diff_stdout` probe
    // it after every operation; here it is asserted directly and repeatedly
    // while the state is churned.
    for _ in 0..100 {
        let before_c = probe_inner(h.c.static_alias);
        let before_r = probe_inner(h.rs.static_alias);
        assert_eq!(before_c, before_r, "cfg_18: inner diverged before churn");
        let v = rng.value_class();
        h.diff("cfg_18 churn", move |i| chain(i.static_alias, v, 3));
        let after_c = probe_inner(h.c.static_alias);
        let after_r = probe_inner(h.rs.static_alias);
        assert_eq!(after_c, after_r, "cfg_18: inner diverged after churn");
    }
}

// ===========================================================================
// Phase C — ERRORS.md rows
// ===========================================================================

/// Run `f` in a forked child and report `(exited_normally, code_or_signal)`.
fn signal_of<F: FnOnce()>(f: F) -> (bool, c_int) {
    unsafe { fflush(std::ptr::null_mut()) };
    let pid = unsafe { fork() };
    assert!(pid >= 0, "fork failed");
    if pid == 0 {
        f();
        unsafe { _exit(0) };
    }
    let mut status: c_int = 0;
    let got = unsafe { waitpid(pid, &raw mut status, 0) };
    assert_eq!(got, pid, "waitpid failed");
    let termsig = status & 0x7f;
    if termsig == 0 {
        (true, (status >> 8) & 0xff)
    } else {
        (false, termsig)
    }
}

#[test]
fn err_01_static_alias_null_pointer() {
    let h = harness();
    // Row 1: no null check exists in C, so `*outer` faults. Both libraries must
    // die from the same fatal signal. Done in a forked child so the test
    // process survives; the child touches nothing else.
    let c = signal_of(|| {
        let f = h.c.static_alias;
        let p = unsafe { f(std::ptr::null_mut()) };
        std::hint::black_box(p);
    });
    let r = signal_of(|| {
        let f = h.rs.static_alias;
        let p = unsafe { f(std::ptr::null_mut()) };
        std::hint::black_box(p);
    });
    assert_eq!(c, r, "err_01: null-pointer outcome differs (C={c:?}, RUST={r:?})");
    assert_eq!(c, (false, 11), "err_01: expected SIGSEGV from both, got {c:?}");
}

#[test]
fn err_02_driver_zero_iterations() {
    let mut h = harness();
    let before = h.inner_now();
    for v in [0, 1, -1, i32::MIN, i32::MAX, 12345] {
        h.diff_stdout("err_02 driver(v, 0)", move |i| unsafe { (i.driver)(v, 0) });
    }
    // Silent no-op: nothing printed (asserted equal above, and both empty), and
    // `inner` unchanged.
    let out = capture(|| unsafe { (h.c.driver)(7, 0) });
    assert!(out.is_empty(), "err_02: C printed {out:?} for 0 iterations");
    let out = capture(|| unsafe { (h.rs.driver)(7, 0) });
    assert!(out.is_empty(), "err_02: Rust printed {out:?} for 0 iterations");
    assert_eq!(before, h.inner_now(), "err_02: 0 iterations must not touch `inner`");
}

#[test]
fn err_03_driver_negative_iterations() {
    let mut h = harness();
    let before = h.inner_now();
    for n in [-1, -2, -100, i32::MIN] {
        for v in [0, 1, -1, i32::MIN, i32::MAX] {
            h.diff_stdout("err_03 driver(v, negative)", move |i| unsafe { (i.driver)(v, n) });
        }
    }
    let out = capture(|| unsafe { (h.c.driver)(7, i32::MIN) });
    assert!(out.is_empty(), "err_03: C printed {out:?} for negative iterations");
    let out = capture(|| unsafe { (h.rs.driver)(7, i32::MIN) });
    assert!(out.is_empty(), "err_03: Rust printed {out:?} for negative iterations");
    assert_eq!(before, h.inner_now(), "err_03: negative iterations must not touch `inner`");
}

#[test]
fn err_04_static_alias_then_overflow() {
    let mut h = harness();
    // Row 4: `inner += INT_MAX` overflows. Whatever the C build produces
    // (two's-complement wrap at -O0) the Rust must reproduce exactly.
    for _ in 0..16 {
        h.diff("err_04 then-branch overflow", |i| one(i.static_alias, i32::MAX));
    }
    for _ in 0..16 {
        let inner = h.inner_now();
        let v = if inner >= 0 { i32::MAX } else { inner };
        h.diff("err_04 then-branch overflow chain", move |i| chain(i.static_alias, v, 3));
    }
}

#[test]
fn err_05_static_alias_min_probe() {
    let mut h = harness();
    // Row 5: INT_MIN argument, else-branch, pointer identity is the caller's.
    for _ in 0..16 {
        let inner = h.inner_now();
        let s = {
            let c = one(h.c.static_alias, i32::MIN);
            let r = one(h.rs.static_alias, i32::MIN);
            assert_eq!(c, r, "err_05: INT_MIN observation differs");
            c
        };
        assert_eq!(s.ident, 0, "err_05: expected the caller's own pointer back");
        assert_eq!(s.ret_val, i32::MIN.wrapping_add(inner), "err_05: wrong sum");
        assert_eq!(h.inner_now(), inner, "err_05: else-branch must not touch `inner`");
    }
}

#[test]
fn err_06_static_alias_else_underflow() {
    let mut h = harness();
    // Drive `inner` negative through the row-4 wraparound, then underflow the
    // else-branch addition.
    h.diff("err_06 wrap inner", |i| one(i.static_alias, i32::MAX));
    h.diff("err_06 wrap inner 2", |i| one(i.static_alias, i32::MAX));
    let mut rng = Rng::new(106);
    for _ in 0..100 {
        let inner = h.inner_now();
        // Need *outer < inner and *outer + inner to wrap below INT_MIN.
        let v = if inner < 0 {
            inner.wrapping_sub(1 + (rng.below(1 << 16) as i32))
        } else {
            i32::MIN.wrapping_add(rng.below(1 << 16) as i32)
        };
        h.diff("err_06 else-branch underflow", move |i| one(i.static_alias, v));
    }
}

#[test]
fn err_07_static_alias_self_alias_doubling() {
    let mut h = harness();
    // Row 7: `outer == &inner`; must read-then-add so the result is 2*inner.
    for _ in 0..30 {
        let inner = h.inner_now();
        let c = chain(h.c.static_alias, inner, 3);
        let r = chain(h.rs.static_alias, inner, 3);
        assert_eq!(c, r, "err_07: self-aliased chain differs");
        // First call: *outer == inner -> then-branch, returns &inner (ident 2).
        assert_eq!(c[0].ident, 2, "err_07: first call should return &inner");
        assert_eq!(c[0].ret_val, inner.wrapping_add(inner), "err_07: first doubling wrong");
        // Second call is genuinely self-aliased: inner += *(&inner).
        assert_eq!(c[1].ident, 0, "err_07: self-aliased call returns the same pointer");
        assert_eq!(c[1].ret_val, c[0].ret_val.wrapping_add(c[0].ret_val), "err_07: doubling wrong");
    }
}

#[test]
fn err_08_static_alias_equal_boundary() {
    let mut h = harness();
    for _ in 0..30 {
        let inner = h.inner_now();
        let c = one(h.c.static_alias, inner);
        let r = one(h.rs.static_alias, inner);
        assert_eq!(c, r, "err_08: `==` boundary observation differs");
        assert_eq!(c.ident, 2, "err_08: `>=` must include `==` (then-branch, &inner)");
        assert_eq!(c.ret_val, inner.wrapping_add(inner), "err_08: wrong doubled value");
        assert_eq!(c.cells[0], inner, "err_08: caller's cell must be untouched");
    }
}

#[test]
fn err_09_static_alias_below_boundary() {
    let mut h = harness();
    for _ in 0..30 {
        let inner = h.inner_now();
        let v = inner.wrapping_sub(1);
        let c = one(h.c.static_alias, v);
        let r = one(h.rs.static_alias, v);
        assert_eq!(c, r, "err_09: boundary-1 observation differs");
        assert_eq!(c.ident, 0, "err_09: else-branch returns the caller's pointer");
        assert_eq!(c.ret_val, v.wrapping_add(inner), "err_09: wrong sum");
        assert_eq!(h.inner_now(), inner, "err_09: `inner` must be unchanged");
    }
}

#[test]
fn err_10_driver_extreme_values() {
    let mut h = harness();
    for v in [i32::MIN, i32::MAX, i32::MIN + 1, i32::MAX - 1, 0, -1, 1] {
        for n in [1, 2, 3, 17, 64, 257] {
            h.diff_stdout("err_10 driver extremes", move |i| unsafe { (i.driver)(v, n) });
        }
    }
}

#[test]
fn err_11_driver_oversized_iterations() {
    let mut h = harness();
    h.diff_stdout("err_11 driver oversized", |i| unsafe { (i.driver)(1, 100_000) });
    h.diff_stdout("err_11 driver oversized min", |i| unsafe { (i.driver)(i32::MIN, 100_000) });
}

#[test]
fn err_12_no_enum_arbitrary_int_fuzz() {
    let mut h = harness();
    let mut rng = Rng::new(112);
    // No enum exists in the API, so every `int` parameter is fuzzed with
    // arbitrary bit patterns across the FFI boundary.
    for _ in 0..300 {
        let v = rng.i32_any();
        h.diff("err_12 static_alias arbitrary int", move |i| one(i.static_alias, v));
    }
    for _ in 0..150 {
        let v = rng.i32_any();
        // `iterations` is fuzzed too, but bounded so the suite terminates; the
        // sign bit and small magnitudes are what the C actually branches on.
        let n = (rng.i32_any() % 48) as c_int;
        h.diff_stdout("err_12 driver arbitrary int", move |i| unsafe { (i.driver)(v, n) });
    }
}

// ===========================================================================
// Phase D — symbol parity, enforced as a test
// ===========================================================================

fn defined_dynamic_symbols(so: &Path) -> Vec<String> {
    let out = std::process::Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .expect("run nm -D");
    assert!(out.status.success(), "nm -D failed on {}", so.display());
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        .collect();
    v.sort();
    v.dedup();
    v
}

#[test]
fn phase_d_symbol_parity() {
    // Serialise with the rest of the suite so the isolated cdylib build in
    // `rust_so_path` cannot race another test's.
    let _guard = harness();
    let c = defined_dynamic_symbols(&c_so_path());
    let rs = defined_dynamic_symbols(&rust_so_path());

    assert!(
        c.iter().any(|s| s == "static_alias") && c.iter().any(|s| s == "driver"),
        "sanity: C .so should export both public functions, got {c:?}"
    );

    let missing: Vec<&String> = c.iter().filter(|s| !rs.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing {} symbol(s) exported by the C .so: {missing:?}",
        missing.len()
    );
}

#[test]
fn phase_d_no_unresolved_own_symbols() {
    let _guard = harness();
    // Every undefined symbol in the Rust .so must be satisfied by libc / the
    // Rust runtime, i.e. nothing from the library's own surface may dangle.
    // `dlopen` with RTLD_NOW would fail outright on a genuinely unresolved
    // symbol, so re-opening the library with eager binding is the check.
    let so = rust_so_path();
    let lib = unsafe {
        libloading::os::unix::Library::open(Some(&so), libloading::os::unix::RTLD_NOW)
    }
    .unwrap_or_else(|e| panic!("RTLD_NOW dlopen of {} failed (unresolved symbol): {e}", so.display()));
    drop(lib);

    let so = c_so_path();
    let lib = unsafe {
        libloading::os::unix::Library::open(Some(&so), libloading::os::unix::RTLD_NOW)
    }
    .unwrap_or_else(|e| panic!("RTLD_NOW dlopen of the C {} failed: {e}", so.display()));
    drop(lib);
}
