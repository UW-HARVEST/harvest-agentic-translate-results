//! Shared harness for the C-vs-Rust differential tests.
//!
//! Both libraries are loaded as shared objects through `libloading`; neither is
//! ever called as a linked Rust function. That way the `#[no_mangle]`
//! `extern "C"` export wrapper is part of what is under test, exactly as an
//! external consumer would see it.

// Each test binary includes this whole module but uses only the parts it needs.
#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use libloading::{Library, Symbol};

/// ABI of the single exported entry point:
/// `void hsl_to_rgb(float *dest, const float *src);`
pub type HslToRgb = unsafe extern "C" fn(*mut f32, *const f32);

/// Workspace root: the directory that holds `c_src/` and `translation/`.
pub fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is `<root>/translation`.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

/// Locate the C shared object produced by `c_src/CMakeLists.txt`. The CMake
/// project name is derived from the *parent* directory name, so the file name is
/// not fixed; glob for it instead of hardcoding.
fn c_so_path() -> PathBuf {
    let build_dir = workspace_root().join("c_src").join("build");
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&build_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with("lib") && name.ends_with(".so") && path.is_file() {
                found.push(path);
            }
        }
    }
    found.sort();
    match found.len() {
        0 => panic!(
            "no C .so found in {}. Build it first:\n  cd c_src && mkdir -p build && cd build \\\n    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build_dir.display()
        ),
        _ => found.remove(0),
    }
}

/// Locate the Rust cdylib. Cargo places it next to the test executable's
/// grandparent (`target/<profile>/`), which is where the `cdylib` for this crate
/// lands too. Fall back to scanning both profiles.
fn rust_so_path() -> PathBuf {
    const SO: &str = "libhsl_to_rgb_lib.so";

    // `current_exe()` is `target/<profile>/deps/<test>-<hash>`.
    if let Ok(exe) = std::env::current_exe()
        && let Some(deps) = exe.parent()
    {
        for dir in [Some(deps), deps.parent()].into_iter().flatten() {
            let candidate = dir.join(SO);
            if candidate.is_file() {
                return candidate;
            }
        }
    }

    let target = workspace_root().join("translation").join("target");
    for profile in ["release", "debug"] {
        let candidate = target.join(profile).join(SO);
        if candidate.is_file() {
            return candidate;
        }
    }
    panic!("could not locate {SO}; run `cargo build` first");
}

/// Fail loudly if a `.so` is older than the sources it was built from.
///
/// This matters: `[lib] crate-type = ["cdylib"]` means there is no `rlib` for the
/// integration tests to link against, so `cargo test` does NOT rebuild the
/// cdylib. Without this guard the entire suite can pass green against a stale
/// library after a source edit. `verify.sh` always runs `cargo build` first;
/// this check makes a forgotten build a hard error instead of a false pass.
fn assert_fresh(so: &Path, sources: &[PathBuf]) {
    let so_time = std::fs::metadata(so)
        .and_then(|m| m.modified())
        .unwrap_or_else(|e| panic!("cannot stat {}: {e}", so.display()));

    for src in sources {
        let Ok(src_time) = std::fs::metadata(src).and_then(|m| m.modified()) else {
            continue;
        };
        if src_time > so_time {
            panic!(
                "STALE LIBRARY: {} is older than {}.\n\
                 `cargo test` does not rebuild a cdylib-only lib target, so this run \
                 would have tested outdated code.\n\
                 Rebuild first:  cargo build --release   (or run ./verify.sh)",
                so.display(),
                src.display()
            );
        }
    }
}

/// The two implementations under comparison, each reached only through `dlopen`.
pub struct Pair {
    _c_lib: Library,
    _rust_lib: Library,
    pub c: HslToRgb,
    pub rust: HslToRgb,
}

fn load_pair() -> Pair {
    let c_path = c_so_path();
    let rust_path = rust_so_path();

    let root = workspace_root();
    assert_fresh(
        &c_path,
        &[
            root.join("c_src").join("src").join("lib.c"),
            root.join("c_src").join("include").join("lib.h"),
        ],
    );
    assert_fresh(
        &rust_path,
        &[root.join("translation").join("src").join("lib.rs")],
    );

    // SAFETY: both paths point at shared objects built from this workspace.
    // Loading them runs their initialisers, which for these two libraries is
    // only the standard C/Rust runtime setup.
    unsafe {
        let c_lib = Library::new(&c_path)
            .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", c_path.display()));
        let rust_lib = Library::new(&rust_path)
            .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", rust_path.display()));

        let c_sym: Symbol<HslToRgb> = c_lib
            .get(b"hsl_to_rgb\0")
            .expect("C .so does not export hsl_to_rgb");
        let rust_sym: Symbol<HslToRgb> = rust_lib
            .get(b"hsl_to_rgb\0")
            .expect("Rust .so does not export hsl_to_rgb");

        let c = *c_sym;
        let rust = *rust_sym;

        Pair {
            _c_lib: c_lib,
            _rust_lib: rust_lib,
            c,
            rust,
        }
    }
}

/// Process-wide, leaked so the `Library` handles outlive every borrow.
pub fn pair() -> &'static Pair {
    static PAIR: OnceLock<&'static Pair> = OnceLock::new();
    PAIR.get_or_init(|| Box::leak(Box::new(load_pair())))
}

// ---------------------------------------------------------------------------
// Invocation helpers
// ---------------------------------------------------------------------------

/// Sentinel written into every `dest` slot before a call, so a missing store is
/// visible instead of silently reading a stale/zero value. `0xDEADBEEF` as an
/// `f32` bit pattern is a negative NaN, which no correct output can be by
/// accident.
pub const SENTINEL: u32 = 0xDEAD_BEEF;

/// Call one implementation with a fresh 3-float `dest` and return the raw bits.
fn invoke(f: HslToRgb, src: &[f32; 3]) -> [u32; 3] {
    let mut dest = [f32::from_bits(SENTINEL); 3];
    // SAFETY: `dest` has 3 writable and `src` 3 readable `f32`s, matching the
    // arity the C implementation hardcodes.
    unsafe { f(dest.as_mut_ptr(), src.as_ptr()) };
    [
        dest[0].to_bits(),
        dest[1].to_bits(),
        dest[2].to_bits(),
    ]
}

/// Render a triple of bit patterns with their float interpretation, for
/// failure messages.
pub fn show(bits: &[u32; 3]) -> String {
    let mut out = String::from("[");
    for (i, &b) in bits.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        out.push_str(&format!("{:#010x}({})", b, f32::from_bits(b)));
    }
    out.push(']');
    out
}

fn show_src(src: &[f32; 3]) -> String {
    format!(
        "h={:#010x}({}) s={:#010x}({}) l={:#010x}({})",
        src[0].to_bits(),
        src[0],
        src[1].to_bits(),
        src[1],
        src[2].to_bits(),
        src[2]
    )
}

/// Run one input through both `.so`s and assert bit-for-bit equality.
#[track_caller]
pub fn assert_same(row: &str, src: &[f32; 3]) {
    let p = pair();
    let c_out = invoke(p.c, src);
    let rust_out = invoke(p.rust, src);
    assert_eq!(
        c_out,
        rust_out,
        "\n[{row}] divergence\n  input: {}\n  C   : {}\n  Rust: {}\n",
        show_src(src),
        show(&c_out),
        show(&rust_out)
    );
}

/// Run a whole batch, reporting only the first divergence (with a count).
#[track_caller]
pub fn assert_same_batch<I: IntoIterator<Item = [f32; 3]>>(row: &str, inputs: I) {
    let p = pair();
    let mut n = 0usize;
    let mut failures = 0usize;
    let mut first: Option<String> = None;

    for src in inputs {
        n += 1;
        let c_out = invoke(p.c, &src);
        let rust_out = invoke(p.rust, &src);
        if c_out != rust_out {
            failures += 1;
            if first.is_none() {
                first = Some(format!(
                    "  input: {}\n  C   : {}\n  Rust: {}",
                    show_src(&src),
                    show(&c_out),
                    show(&rust_out)
                ));
            }
        }
    }

    assert!(n > 0, "[{row}] generated no inputs");
    if let Some(detail) = first {
        panic!("\n[{row}] {failures}/{n} inputs diverged\nfirst divergence:\n{detail}\n");
    }
}

// ---------------------------------------------------------------------------
// Aliasing-aware invocation (CONFIGS rows 27-30)
// ---------------------------------------------------------------------------

/// Call with `dest` and `src` pointing into the same 8-float buffer, at the
/// given element offsets, and return the whole buffer's bits afterwards. This
/// simultaneously checks the written values and that nothing outside
/// `dest[0..3]` was touched.
fn invoke_aliased(f: HslToRgb, src: &[f32; 3], dest_off: usize, src_off: usize) -> [u32; 8] {
    let mut buf = [f32::from_bits(SENTINEL); 8];
    buf[src_off] = src[0];
    buf[src_off + 1] = src[1];
    buf[src_off + 2] = src[2];
    // SAFETY: both windows `[off, off+3)` lie inside an 8-element array.
    unsafe {
        let base = buf.as_mut_ptr();
        f(base.add(dest_off), base.add(src_off) as *const f32);
    }
    let mut out = [0u32; 8];
    for i in 0..8 {
        out[i] = buf[i].to_bits();
    }
    out
}

#[track_caller]
pub fn assert_same_aliased_batch<I: IntoIterator<Item = [f32; 3]>>(
    row: &str,
    dest_off: usize,
    src_off: usize,
    inputs: I,
) {
    let p = pair();
    let mut n = 0usize;
    let mut failures = 0usize;
    let mut first: Option<String> = None;

    for src in inputs {
        n += 1;
        let c_out = invoke_aliased(p.c, &src, dest_off, src_off);
        let rust_out = invoke_aliased(p.rust, &src, dest_off, src_off);
        if c_out != rust_out {
            failures += 1;
            if first.is_none() {
                first = Some(format!(
                    "  input: {}\n  dest_off={dest_off} src_off={src_off}\n  C   : {c_out:#010x?}\n  Rust: {rust_out:#010x?}",
                    show_src(&src)
                ));
            }
        }
    }

    assert!(n > 0, "[{row}] generated no inputs");
    if let Some(detail) = first {
        panic!("\n[{row}] {failures}/{n} inputs diverged\nfirst divergence:\n{detail}\n");
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*), fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub const RNG_SEED: u64 = 0x0005_DEEC_E66D;

pub struct Rng(u64);

impl Rng {
    pub fn new(stream: u64) -> Self {
        // Mix the stream id into the fixed seed so different rows get different
        // sequences while every run is reproducible.
        let mut s = RNG_SEED ^ stream.wrapping_mul(0x9E37_79B9_7F4A_7C15);
        if s == 0 {
            s = 0x1234_5678_9ABC_DEF0;
        }
        Rng(s)
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    /// Uniform in `[0, 1)` with 24 bits of mantissa entropy.
    pub fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }

    /// Uniform in `[lo, hi)`.
    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + self.unit() * (hi - lo)
    }

    /// Any finite `f32`: draw a random bit pattern and reject non-finite ones.
    pub fn finite(&mut self) -> f32 {
        loop {
            let v = f32::from_bits(self.next_u32());
            if v.is_finite() {
                return v;
            }
        }
    }

    /// Any `f32` at all, including infinities, NaNs of every payload, and
    /// subnormals.
    pub fn any_bits(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }

    pub fn pick<T: Copy>(&mut self, items: &[T]) -> T {
        items[(self.next_u32() as usize) % items.len()]
    }
}

// ---------------------------------------------------------------------------
// Edge-pattern pool (CONFIGS axis N)
// ---------------------------------------------------------------------------

/// The interesting `f32` bit patterns, spelled out as bits so NaN payloads and
/// zero signs survive exactly. Includes quiet NaNs of both signs with distinct
/// payloads, signalling NaNs of both signs, both infinities, both zeros,
/// smallest subnormals, smallest normals, and `±f32::MAX`.
pub const EDGE_BITS: &[u32] = &[
    0x0000_0000, // +0.0
    0x8000_0000, // -0.0
    0x0000_0001, // +smallest subnormal
    0x8000_0001, // -smallest subnormal
    0x007F_FFFF, // +largest subnormal
    0x807F_FFFF, // -largest subnormal
    0x0080_0000, // +MIN_POSITIVE
    0x8080_0000, // -MIN_POSITIVE
    0x3F80_0000, // +1.0
    0xBF80_0000, // -1.0
    0x3F00_0000, // +0.5
    0x7F7F_FFFF, // +f32::MAX
    0xFF7F_FFFF, // -f32::MAX
    0x7F80_0000, // +inf
    0xFF80_0000, // -inf
    0x7FC0_0000, // +qNaN, payload 0
    0xFFC0_0000, // -qNaN, payload 0
    0x7FC0_1234, // +qNaN, payload 0x1234
    0xFFC0_1234, // -qNaN, payload 0x1234
    0x7FFF_FFFF, // +qNaN, all payload bits set
    0xFFFF_FFFF, // -qNaN, all payload bits set
    0x7FA0_0000, // +sNaN
    0xFFA0_0000, // -sNaN
    0x7F80_0001, // +sNaN, minimal payload
    0xFF80_0001, // -sNaN, minimal payload
];

pub fn edge_values() -> Vec<f32> {
    EDGE_BITS.iter().map(|&b| f32::from_bits(b)).collect()
}

/// Edge patterns excluding `±0.0` (used where the `s == 0` fast path must be
/// avoided).
pub fn edge_values_nonzero() -> Vec<f32> {
    EDGE_BITS
        .iter()
        .filter(|&&b| b != 0x0000_0000 && b != 0x8000_0000)
        .map(|&b| f32::from_bits(b))
        .collect()
}

/// The seven hue thresholds the C branches on.
pub const THRESHOLDS: &[f32] = &[0.0, 60.0, 120.0, 180.0, 240.0, 300.0, 360.0];

// ---------------------------------------------------------------------------
// Fatal-signal differential support (ERRORS rows 15-18)
// ---------------------------------------------------------------------------

/// Env var naming which null-pointer case a child process should perform.
pub const CRASH_CASE_ENV: &str = "HSL_DIFF_CRASH_CASE";
/// Env var selecting which implementation the child should crash in.
pub const CRASH_IMPL_ENV: &str = "HSL_DIFF_CRASH_IMPL";

/// The four unchecked memory-safety preconditions from `ERRORS.md`.
#[derive(Copy, Clone)]
pub enum CrashCase {
    /// Row 15: `src == NULL`, valid `dest`.
    NullSrc,
    /// Row 16: `dest == NULL`, valid `src` with `s != 0` (slow path).
    NullDest,
    /// Row 17: `dest == NULL` with `s == 0` (the early-return path still writes).
    NullDestFastPath,
    /// Row 18: both pointers `NULL`.
    BothNull,
}

impl CrashCase {
    pub fn tag(self) -> &'static str {
        match self {
            CrashCase::NullSrc => "null_src",
            CrashCase::NullDest => "null_dest",
            CrashCase::NullDestFastPath => "null_dest_fast",
            CrashCase::BothNull => "both_null",
        }
    }

    fn from_tag(tag: &str) -> Option<Self> {
        Some(match tag {
            "null_src" => CrashCase::NullSrc,
            "null_dest" => CrashCase::NullDest,
            "null_dest_fast" => CrashCase::NullDestFastPath,
            "both_null" => CrashCase::BothNull,
            _ => return None,
        })
    }
}

/// Executed in the CHILD process: perform the null-pointer call for real.
/// Never returns normally if the fault behaves as expected.
fn perform_crash(f: HslToRgb, case: CrashCase) {
    let mut dest = [0.0f32; 3];
    // `s = 0.5` for the slow path, `s = 0.0` for the fast path.
    let src_slow = [30.0f32, 0.5, 0.5];
    let src_fast = [30.0f32, 0.0, 0.5];

    // SAFETY: deliberately violating the (unchecked) pointer preconditions in a
    // throwaway child process, to observe that C and Rust fault identically.
    unsafe {
        match case {
            CrashCase::NullSrc => f(dest.as_mut_ptr(), std::ptr::null()),
            CrashCase::NullDest => f(std::ptr::null_mut(), src_slow.as_ptr()),
            CrashCase::NullDestFastPath => f(std::ptr::null_mut(), src_fast.as_ptr()),
            CrashCase::BothNull => f(std::ptr::null_mut(), std::ptr::null()),
        }
    }

    // Reached only if the null access did NOT fault. Distinguish that from a
    // signal death with a unique exit code.
    std::process::exit(77);
}

/// Call from the designated child-mode test. Returns `false` if this process is
/// the parent (no env var set) and the test body should be skipped.
pub fn run_as_crash_child_if_requested() -> bool {
    let Ok(tag) = std::env::var(CRASH_CASE_ENV) else {
        return false;
    };
    let case = CrashCase::from_tag(&tag).unwrap_or_else(|| panic!("unknown crash case {tag:?}"));
    let which = std::env::var(CRASH_IMPL_ENV).unwrap_or_default();

    let p = pair();
    let f = match which.as_str() {
        "c" => p.c,
        "rust" => p.rust,
        other => panic!("unknown impl {other:?}"),
    };
    perform_crash(f, case);
    true
}

/// Outcome of a child run.
#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Killed by this signal number.
    Signal(i32),
    /// Exited normally with this code (77 = the null access did not fault).
    Exit(i32),
}

fn spawn_child(case: CrashCase, which: &str) -> Outcome {
    use std::os::unix::process::ExitStatusExt;

    let exe = std::env::current_exe().expect("current_exe");
    let status = std::process::Command::new(exe)
        .args(["--exact", "crash_child", "--test-threads=1", "--quiet"])
        .env(CRASH_CASE_ENV, case.tag())
        .env(CRASH_IMPL_ENV, which)
        // Keep the child's harness output out of the parent's log.
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("failed to spawn crash child");

    match status.signal() {
        Some(sig) => Outcome::Signal(sig),
        None => Outcome::Exit(status.code().unwrap_or(-1)),
    }
}

/// Assert the C and the Rust `.so` die the SAME way for an unchecked-pointer
/// case, and that they actually fault rather than silently succeeding.
#[track_caller]
pub fn assert_same_fatal(row: &str, case: CrashCase) {
    let c_outcome = spawn_child(case, "c");
    let rust_outcome = spawn_child(case, "rust");

    assert_eq!(
        c_outcome, rust_outcome,
        "[{row}] {} : C gave {c_outcome:?} but Rust gave {rust_outcome:?}",
        case.tag()
    );
    assert_eq!(
        c_outcome,
        Outcome::Signal(11),
        "[{row}] {} : expected both to die with SIGSEGV(11), got {c_outcome:?}",
        case.tag()
    );
}
