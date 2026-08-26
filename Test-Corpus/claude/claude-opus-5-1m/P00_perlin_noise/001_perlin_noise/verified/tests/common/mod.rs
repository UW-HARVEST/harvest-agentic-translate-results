//! Shared scaffolding for the differential tests.
//!
//! Both implementations are loaded as *shared objects* with `libloading` and
//! called through their exported C symbols only -- the Rust translation is
//! never called directly, so the `#[no_mangle]` wrappers are covered too.
#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Exported C ABI of both shared objects
// ---------------------------------------------------------------------------

pub type FnNoise3Internal = unsafe extern "C" fn(f32, f32, f32, i32, i32, i32, u8) -> f32;
pub type FnNoise3 = unsafe extern "C" fn(f32, f32, f32, i32, i32, i32) -> f32;
pub type FnNoise3Seed = unsafe extern "C" fn(f32, f32, f32, i32, i32, i32, i32) -> f32;
pub type FnRidge = unsafe extern "C" fn(f32, f32, f32, f32, f32, f32, i32) -> f32;
pub type FnFbm = unsafe extern "C" fn(f32, f32, f32, f32, f32, i32) -> f32;
pub type FnInner =
    unsafe extern "C" fn(i32, f32, f32, f32, i32, i32, i32, i32, f32, f32, f32, i32) -> f32;

pub struct Api {
    pub name: &'static str,
    pub path: PathBuf,
    pub noise3_internal: FnNoise3Internal,
    pub noise3: FnNoise3,
    pub noise3_seed: FnNoise3Seed,
    pub ridge: FnRidge,
    pub fbm: FnFbm,
    pub turbulence: FnFbm,
    pub wrap_nonpow2: FnNoise3Internal,
    pub inner: FnInner,
}

unsafe fn load(name: &'static str, path: &Path) -> Api {
    // Leak the library: the function pointers below must stay valid forever.
    let lib = Box::leak(Box::new(
        libloading::Library::new(path)
            .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display())),
    ));
    macro_rules! sym {
        ($t:ty, $s:literal) => {{
            let s: libloading::Symbol<$t> = lib
                .get($s)
                .unwrap_or_else(|e| panic!("{} misses {}: {e}", path.display(), stringify!($s)));
            *s
        }};
    }
    Api {
        name,
        path: path.to_path_buf(),
        noise3_internal: sym!(FnNoise3Internal, b"stb_perlin_noise3_internal\0"),
        noise3: sym!(FnNoise3, b"stb_perlin_noise3\0"),
        noise3_seed: sym!(FnNoise3Seed, b"stb_perlin_noise3_seed\0"),
        ridge: sym!(FnRidge, b"stb_perlin_ridge_noise3\0"),
        fbm: sym!(FnFbm, b"stb_perlin_fbm_noise3\0"),
        turbulence: sym!(FnFbm, b"stb_perlin_turbulence_noise3\0"),
        wrap_nonpow2: sym!(FnNoise3Internal, b"stb_perlin_noise3_wrap_nonpow2\0"),
        inner: sym!(FnInner, b"inner\0"),
    }
}

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `target/<profile>` of the currently running test binary.
pub fn target_profile_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    // .../target/<profile>/deps/<test binary>
    exe.parent()
        .and_then(|p| p.parent())
        .expect("target dir")
        .to_path_buf()
}

/// Path of the Rust `cdylib` to load.
///
/// `cargo test` links the library target as an `rlib` for the test binaries but
/// does not necessarily emit the `cdylib` artefact, so the shared object is
/// built on demand.  The build uses a *private* target directory, which keeps it
/// from blocking on the build lock of the `cargo test` invocation that is
/// running us.
pub fn rust_so_path() -> PathBuf {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        let primary = target_profile_dir().join("libstb_perlin_cli.so");
        if !rust_so_stale(&primary) {
            return primary;
        }
        let release = target_profile_dir()
            .file_name()
            .map(|n| n == "release")
            .unwrap_or(false);
        let dir = manifest_dir().join("target/sotest");
        let built = dir
            .join(if release { "release" } else { "debug" })
            .join("libstb_perlin_cli.so");
        // `--offline` first (the crates the dev-dependencies need are cached);
        // fall back to a networked resolve if that is not possible.
        let mut ok = false;
        for offline in [true, false] {
            let mut cmd = Command::new(env!("CARGO"));
            cmd.arg("build").arg("--lib");
            if offline {
                cmd.arg("--offline");
            }
            if release {
                cmd.arg("--release");
            }
            cmd.arg("--target-dir").arg(&dir);
            cmd.current_dir(manifest_dir());
            let out = cmd.output().expect("running cargo build --lib");
            if out.status.success() {
                ok = true;
                break;
            }
            eprintln!(
                "cargo build --lib (offline={offline}) failed:\n{}",
                String::from_utf8_lossy(&out.stderr)
            );
        }
        assert!(ok, "could not build the cdylib");
        assert!(built.exists(), "{} was not produced", built.display());
        built
    })
    .clone()
}

/// True when `so` is missing or older than any file of the Rust translation.
fn rust_so_stale(so: &Path) -> bool {
    let modified = |p: &Path| {
        std::fs::metadata(p)
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
    };
    if !so.exists() {
        return true;
    }
    let so_time = modified(so);
    let src = manifest_dir().join("src");
    let mut newest = std::time::SystemTime::UNIX_EPOCH;
    let mut stack = vec![src];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else {
                newest = newest.max(modified(&p));
            }
        }
    }
    so_time < newest
}

pub fn c_so_path() -> PathBuf {
    manifest_dir().join("target/cdiff/libc_driver.so")
}

pub fn c_exe_path() -> PathBuf {
    manifest_dir().join("target/cdiff/c_driver")
}

pub fn rust_exe_path() -> PathBuf {
    target_profile_dir().join("driver")
}

pub fn so_main_runner_path() -> PathBuf {
    target_profile_dir().join("so_main_runner")
}

/// Builds the C reference artifacts (executable + shared object) once per
/// process, serialised across processes with an advisory lock -- `cargo test`
/// runs the test binaries in parallel and they would otherwise clobber each
/// other's output file.
fn ensure_c_built() {
    static ONCE: OnceLock<()> = OnceLock::new();
    ONCE.get_or_init(|| {
        use std::os::fd::AsRawFd;
        let dir = manifest_dir().join("target/cdiff");
        std::fs::create_dir_all(&dir).expect("create target/cdiff");
        let lock = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(dir.join(".build.lock"))
            .expect("open lock file");
        unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX) };
        // Another process may have completed the build while we waited.
        if stale(&c_so_path()) || stale(&c_exe_path()) {
            let script = manifest_dir().join("scripts/build_c_so.sh");
            let out = Command::new("bash")
                .arg(&script)
                .current_dir(manifest_dir())
                .output()
                .expect("running scripts/build_c_so.sh");
            assert!(
                out.status.success(),
                "scripts/build_c_so.sh failed:\n{}\n{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
        }
        unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_UN) };
    });
}

/// The C shared object, built if necessary.
pub fn c_so() -> PathBuf {
    if stale(&c_so_path()) {
        ensure_c_built();
    }
    c_so_path()
}

fn stale(artifact: &Path) -> bool {
    let src = manifest_dir().join("c_src/src/main.c");
    let header = manifest_dir().join("c_src/src/stb_perlin.h");
    let m = |p: &Path| {
        std::fs::metadata(p)
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
    };
    !artifact.exists() || m(artifact) < m(&src) || m(artifact) < m(&header)
}

pub fn c_api() -> &'static Api {
    static API: OnceLock<Api> = OnceLock::new();
    API.get_or_init(|| {
        if stale(&c_so_path()) || stale(&c_exe_path()) {
            ensure_c_built();
        }
        unsafe { load("C", &c_so_path()) }
    })
}

pub fn rust_api() -> &'static Api {
    static API: OnceLock<Api> = OnceLock::new();
    API.get_or_init(|| {
        let p = rust_so_path();
        assert!(
            p.exists(),
            "{} is missing -- run `cargo build` (the cdylib is what the tests load)",
            p.display()
        );
        unsafe { load("Rust", &p) }
    })
}

/// Ensures the C executable exists (used by the driver/CLI tests).
pub fn ensure_c_exe() -> PathBuf {
    if stale(&c_exe_path()) {
        ensure_c_built();
    }
    c_exe_path()
}

// ---------------------------------------------------------------------------
// Divergence collector
// ---------------------------------------------------------------------------

pub struct Diff {
    row: &'static str,
    cases: usize,
    failures: Vec<String>,
}

impl Diff {
    pub fn new(row: &'static str) -> Self {
        Diff {
            row,
            cases: 0,
            failures: Vec::new(),
        }
    }

    /// Compares one pair of results bit-for-bit.
    pub fn check(&mut self, ctx: impl std::fmt::Display, c: f32, rust: f32) {
        self.cases += 1;
        if c.to_bits() != rust.to_bits() && self.failures.len() < 20 {
            {
                self.failures.push(format!(
                    "  {ctx}\n      C    = {c:e} (0x{:08x})\n      Rust = {rust:e} (0x{:08x})",
                    c.to_bits(),
                    rust.to_bits()
                ));
            }
        }
    }

    /// Compares two byte strings (driver stdout).
    pub fn check_bytes(&mut self, ctx: impl std::fmt::Display, c: &[u8], rust: &[u8]) {
        self.cases += 1;
        if c != rust && self.failures.len() < 20 {
            {
                self.failures.push(format!(
                    "  {ctx}\n      C    = {:?}\n      Rust = {:?}",
                    String::from_utf8_lossy(c),
                    String::from_utf8_lossy(rust)
                ));
            }
        }
    }

    pub fn check_eq<T: std::fmt::Debug + PartialEq>(
        &mut self,
        ctx: impl std::fmt::Display,
        c: T,
        rust: T,
    ) {
        self.cases += 1;
        if c != rust && self.failures.len() < 20 {
            {
                self.failures.push(format!(
                    "  {ctx}\n      C    = {c:?}\n      Rust = {rust:?}"
                ));
            }
        }
    }

    pub fn cases(&self) -> usize {
        self.cases
    }

    #[track_caller]
    pub fn finish(self) {
        assert!(self.cases > 0, "{}: no cases were exercised", self.row);
        if !self.failures.is_empty() {
            panic!(
                "{}: {} of {} cases diverged:\n{}",
                self.row,
                self.failures.len(),
                self.cases,
                self.failures.join("\n")
            );
        }
        println!("{}: {} cases matched", self.row, self.cases);
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (splitmix64) + input generators
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed ^ 0x9e37_79b9_7f4a_7c15)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }

    pub fn below(&mut self, n: u32) -> u32 {
        assert!(n > 0);
        self.next_u32() % n
    }

    /// Inclusive range.
    pub fn range(&mut self, lo: i32, hi: i32) -> i32 {
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }

    pub fn boolean(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }

    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len() as u32) as usize]
    }

    pub fn seed_u8(&mut self) -> u8 {
        self.next_u32() as u8
    }

    /// A "nice" coordinate: `k + f/2^m` in `[-range, range]`, which is the shape
    /// real callers use (and which keeps the noise lattice interesting).
    pub fn coord(&mut self, range: i32) -> f32 {
        let int_part = self.range(-range, range) as f32;
        let frac = match self.below(4) {
            0 => 0.0,
            1 => 0.5,
            2 => (self.below(256) as f32) / 256.0,
            _ => f32::from_bits(0x3f00_0000 | (self.next_u32() & 0x007f_ffff)) - 1.0,
        };
        int_part + frac
    }

    /// Any finite `f32` (uniform over bit patterns, exponent capped so the
    /// value stays finite).
    pub fn finite_f32(&mut self) -> f32 {
        loop {
            let v = f32::from_bits(self.next_u32());
            if v.is_finite() {
                return v;
            }
        }
    }

    /// Any `f32` at all, including infinities, NaNs and subnormals.
    pub fn any_f32(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }

    /// Typical lacunarity/gain shapes.
    pub fn lac_gain(&mut self) -> f32 {
        match self.below(8) {
            0 => 0.0,
            1 => 1.0,
            2 => 2.0,
            3 => 0.5,
            4 => -2.0,
            5 => (self.range(-400, 400) as f32) / 100.0,
            6 => (self.range(1, 1000) as f32) / 1000.0,
            _ => (self.range(-8000, 8000) as f32) / 1000.0,
        }
    }
}

pub const SPECIAL_F32: &[f32] = &[
    0.0,
    -0.0,
    1.0,
    -1.0,
    0.5,
    -0.5,
    f32::MIN_POSITIVE,
    -f32::MIN_POSITIVE,
    f32::MAX,
    f32::MIN,
    f32::INFINITY,
    f32::NEG_INFINITY,
    f32::NAN,
    -f32::NAN,
    1e-45,  // smallest positive subnormal
    -1e-45,
    16777216.0,  // 2^24
    -16777216.0,
    2147483520.0,  // largest float below 2^31
    -2147483648.0, // exactly -2^31
    4294967296.0,  // 2^32 (outside int range)
    1e30,
    -1e30,
];

pub const SPECIAL_WRAPS: &[i32] = &[
    0,
    1,
    2,
    4,
    8,
    16,
    32,
    64,
    128,
    256,
    512,
    1024,
    3,
    5,
    7,
    100,
    255,
    257,
    -1,
    -2,
    -5,
    -256,
    i32::MAX,
    i32::MIN,
];

pub const SPECIAL_INTS: &[i32] = &[
    0,
    1,
    -1,
    2,
    -2,
    255,
    256,
    -255,
    -256,
    65535,
    i32::MAX,
    i32::MIN,
    i32::MAX - 1,
    i32::MIN + 1,
];

// ---------------------------------------------------------------------------
// Model of the C permutation tables, used to tell reproducible inputs of
// `stb_perlin_noise3_wrap_nonpow2` from undefined-behaviour ones.
// ---------------------------------------------------------------------------

fn parse_table(header: &str, decl: &str) -> Vec<u8> {
    let start = header
        .find(decl)
        .unwrap_or_else(|| panic!("{decl} not found in stb_perlin.h"));
    let body = &header[start + decl.len()..];
    let end = body.find('}').expect("table end");
    let mut out = Vec::new();
    for token in body[..end].split(|c: char| c == ',' || c.is_whitespace()) {
        let t = token.trim();
        if t.is_empty() || t.starts_with('/') || t.starts_with('{') {
            continue;
        }
        if let Ok(v) = t.parse::<u16>() {
            out.push(v as u8);
        }
    }
    assert!(out.len() >= 512, "{decl}: only {} values", out.len());
    out.truncate(512);
    out
}

/// The 1024-byte window that both C builds lay out identically:
/// `stb__perlin_randtab` followed by `stb__perlin_randtab_grad_idx`.
pub fn table_window() -> &'static [u8; 1024] {
    static W: OnceLock<[u8; 1024]> = OnceLock::new();
    W.get_or_init(|| {
        let header = std::fs::read_to_string(manifest_dir().join("c_src/src/stb_perlin.h"))
            .expect("read stb_perlin.h");
        let randtab = parse_table(&header, "stb__perlin_randtab[512] =");
        let grad = parse_table(&header, "stb__perlin_randtab_grad_idx[512] =");
        let mut w = [0u8; 1024];
        w[..512].copy_from_slice(&randtab);
        w[512..].copy_from_slice(&grad);
        w
    })
}

/// C's `(int)` conversion of a float on x86-64 (`cvttss2si`).
pub fn f32_to_i32(a: f32) -> i32 {
    if a.is_nan() || !(-2_147_483_648.0f32..2_147_483_648.0f32).contains(&a) {
        i32::MIN
    } else {
        a as i32
    }
}

/// `stb__perlin_fastfloor`
pub fn fastfloor(a: f32) -> i32 {
    let ai = f32_to_i32(a);
    if a < ai as f32 {
        ai.wrapping_sub(1)
    } else {
        ai
    }
}

/// Classification of a `stb_perlin_noise3_wrap_nonpow2` call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Nonpow2Class {
    /// Every table index stays inside the deterministic 1024-byte window, so
    /// the C result is reproducible and Rust must match it bit-for-bit.
    Reproducible,
    /// A table index leaves that window (or the index arithmetic traps): the C
    /// behaviour depends on what the linker put next to `.data`.
    Undefined,
}

/// Re-implements the index arithmetic of `stb_perlin_noise3_wrap_nonpow2` to
/// classify an input without calling either library.
pub fn classify_nonpow2(
    x: f32,
    y: f32,
    z: f32,
    x_wrap: i32,
    y_wrap: i32,
    z_wrap: i32,
    seed: u8,
) -> Nonpow2Class {
    let w = table_window();
    let px = fastfloor(x);
    let py = fastfloor(y);
    let pz = fastfloor(z);
    let x_wrap2 = if x_wrap != 0 { x_wrap } else { 256 };
    let y_wrap2 = if y_wrap != 0 { y_wrap } else { 256 };
    let z_wrap2 = if z_wrap != 0 { z_wrap } else { 256 };

    // `INT_MIN % -1` traps on x86 (the C library dies with SIGFPE).
    for (p, wr) in [(px, x_wrap2), (py, y_wrap2), (pz, z_wrap2)] {
        if p == i32::MIN && wr == -1 {
            return Nonpow2Class::Undefined;
        }
    }

    let axis = |p: i32, wrap: i32| -> (i32, i32) {
        let mut a0 = p.wrapping_rem(wrap);
        if a0 < 0 {
            a0 = a0.wrapping_add(wrap);
        }
        let a1 = a0.wrapping_add(1).wrapping_rem(wrap);
        (a0, a1)
    };
    let (x0, x1) = axis(px, x_wrap2);
    let (y0, y1) = axis(py, y_wrap2);
    let (z0, z1) = axis(pz, z_wrap2);

    // Every `stb__perlin_randtab[...]` read must land in 0..1024, and every
    // `stb__perlin_randtab_grad_idx[...]` read in 0..512.
    let mut ok = true;
    let mut randtab = |i: i64| -> i64 {
        if (0..1024).contains(&i) {
            i64::from(w[i as usize])
        } else {
            ok = false;
            0
        }
    };
    let seed = i64::from(seed);
    let mut r0 = randtab(i64::from(x0));
    r0 = randtab(r0 + seed);
    let mut r1 = randtab(i64::from(x1));
    r1 = randtab(r1 + seed);
    let r00 = randtab(r0 + i64::from(y0));
    let r01 = randtab(r0 + i64::from(y1));
    let r10 = randtab(r1 + i64::from(y0));
    let r11 = randtab(r1 + i64::from(y1));
    for (r, zz) in [
        (r00, z0),
        (r00, z1),
        (r01, z0),
        (r01, z1),
        (r10, z0),
        (r10, z1),
        (r11, z0),
        (r11, z1),
    ] {
        let idx = r + i64::from(zz);
        if !(0..512).contains(&idx) {
            ok = false;
        }
    }
    if ok {
        Nonpow2Class::Reproducible
    } else {
        Nonpow2Class::Undefined
    }
}

// ---------------------------------------------------------------------------
// Driver (`main`) helpers
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq)]
pub struct RunResult {
    pub stdout: Vec<u8>,
    /// `Some(code)` for a normal exit, `None` when killed by a signal.
    pub code: Option<i32>,
}

fn run_process(mut cmd: Command, input: &str) -> RunResult {
    use std::io::Write;
    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(input.as_bytes())
        .or_else(|e| {
            if e.kind() == std::io::ErrorKind::BrokenPipe {
                Ok(())
            } else {
                Err(e)
            }
        })
        .expect("write stdin");
    drop(child.stdin.take());
    let out = child.wait_with_output().expect("wait");
    RunResult {
        stdout: out.stdout,
        code: out.status.code(),
    }
}

pub fn run_c_driver(input: &str) -> RunResult {
    run_process(Command::new(ensure_c_exe()), input)
}

pub fn run_rust_driver(input: &str) -> RunResult {
    let exe = rust_exe_path();
    assert!(exe.exists(), "{} is missing -- run `cargo build`", exe.display());
    run_process(Command::new(exe), input)
}

/// Calls the `main` symbol *exported by a shared object* (through the
/// `so_main_runner` example, which dlopens the library).
pub fn run_so_main(so: &Path, input: &str) -> RunResult {
    let runner = so_main_runner_path();
    assert!(
        runner.exists(),
        "{} is missing -- run `cargo build`",
        runner.display()
    );
    let mut cmd = Command::new(runner);
    cmd.arg(so);
    run_process(cmd, input)
}

/// Runs one C function in a child process so that a `SIGFPE`/`SIGSEGV` inside
/// the C library cannot take the test harness down.  Returns the exit status of
/// the child and the printed float bits.
pub fn run_probe(so: &Path, func: &str, args: &[String]) -> (std::process::ExitStatus, Option<u32>) {
    let script = manifest_dir().join("scripts/probe.py");
    let out = Command::new("python3")
        .arg(script)
        .arg(so)
        .arg(func)
        .args(args)
        .stderr(Stdio::null())
        .output()
        .expect("run probe.py");
    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let bits = text
        .split_whitespace()
        .last()
        .and_then(|t| t.strip_prefix("0x"))
        .and_then(|t| u32::from_str_radix(t, 16).ok());
    (out.status, bits)
}

/// Formats float arguments the way `scripts/probe.py` expects them (C99 hex
/// floats, so no precision is lost).
pub fn hex_arg(v: f32) -> String {
    if v.is_nan() {
        // `float.fromhex` cannot parse NaNs; probe.py accepts plain literals.
        return if v.is_sign_negative() {
            "-nan".to_string()
        } else {
            "nan".to_string()
        };
    }
    if v.is_infinite() {
        return if v.is_sign_negative() {
            "-inf".to_string()
        } else {
            "inf".to_string()
        };
    }
    format!("{:?}", f64::from(v))
}
