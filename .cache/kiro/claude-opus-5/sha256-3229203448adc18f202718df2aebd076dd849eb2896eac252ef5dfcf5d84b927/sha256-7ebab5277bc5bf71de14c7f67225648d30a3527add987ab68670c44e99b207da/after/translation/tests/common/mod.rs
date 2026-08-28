//! Shared harness: loads the C reference `.so` and the Rust `.so` and exposes
//! both `hsv_to_rgb` implementations behind an identical interface.
//!
//! The Rust side is *always* reached through `libloading`, i.e. through the
//! `#[no_mangle] extern "C"` export wrapper, exactly as an external C caller
//! would reach it. No Rust function is called directly.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use libloading::{Library, Symbol};

pub type HsvToRgbFn = unsafe extern "C" fn(*mut f32, *const f32);

/// Workspace root (the directory holding both `c_src/` and `translation/`).
fn project_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Locate the shared library the C sources were built into.
///
/// The CMake project name is derived from the name of the directory that
/// contains `c_src/`, so the file name is not fixed; glob for it instead.
fn c_library_path() -> PathBuf {
    let build_dir = project_root().join("c_src").join("build");
    assert!(
        build_dir.is_dir(),
        "C build directory {} is missing - build the C sources first",
        build_dir.display()
    );

    let mut candidates: Vec<PathBuf> = std::fs::read_dir(&build_dir)
        .expect("c_src/build must be readable")
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file()
                && path.extension().is_some_and(|ext| ext == "so")
                && path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with("lib"))
        })
        .collect();
    candidates.sort();

    candidates.into_iter().next().unwrap_or_else(|| {
        panic!(
            "no lib*.so found in {} - build the C sources first",
            build_dir.display()
        )
    })
}

/// Locate the `cdylib` produced for this crate, rebuilding it first.
fn rust_library_path() -> PathBuf {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(build_rust_library).clone()
}

/// Build the `cdylib` under test and return its path.
///
/// `cargo test` does not (re)build `cdylib` artifacts, and any copy left in
/// `target/<profile>/` or `target/<profile>/deps/` by an earlier `cargo build`
/// may be stale. To guarantee that the library under test corresponds to the
/// current sources, it is always rebuilt here, into a dedicated target
/// directory because the outer `cargo test` holds the lock on the main one.
fn build_rust_library() -> PathBuf {
    let file_name = format!(
        "{}hsv_to_rgb_lib{}",
        std::env::consts::DLL_PREFIX,
        std::env::consts::DLL_SUFFIX
    );

    let exe = std::env::current_exe().expect("test executable path must be known");
    // …/target/<profile>/deps/<test-binary>
    let profile_dir = exe
        .parent()
        .and_then(|deps| deps.parent())
        .expect("test binary must live in target/<profile>/deps");
    let release = profile_dir
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name == "release");

    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let nested_target = manifest_dir.join("target").join("cdylib-under-test");

    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let mut command = Command::new(cargo);
    command
        .current_dir(manifest_dir)
        .arg("build")
        .arg("--lib")
        .arg("--target-dir")
        .arg(&nested_target)
        .arg("--no-default-features");
    if release {
        command.arg("--release");
    }
    // Reproduce the feature selection of the running test binary so that the
    // cdylib under test matches the configuration being exercised.
    if !FEATURES.is_empty() {
        command.arg("--features").arg(FEATURES);
    }

    let status = command
        .status()
        .expect("failed to spawn `cargo build` for the cdylib under test");
    assert!(status.success(), "`cargo build --lib` failed: {status}");

    let built = nested_target
        .join(if release { "release" } else { "debug" })
        .join(&file_name);
    assert!(
        built.is_file(),
        "`cargo build --lib` did not produce {}",
        built.display()
    );
    built
}

/// Comma-separated list of the features enabled in this test build, assembled
/// at compile time so the nested `cargo build` uses the same configuration.
///
/// The crate currently declares no features, so this is always empty; the
/// `cfg` plumbing is kept so new features are picked up automatically.
const FEATURES: &str = "";

/// Both implementations, kept alive for the duration of the test.
pub struct Implementations {
    // Field order matters: the symbols borrow from the libraries, so the
    // libraries are stored as `Box`ed leaks to keep the borrow simple.
    pub c: HsvToRgbFn,
    pub rust: HsvToRgbFn,
    pub c_path: PathBuf,
    pub rust_path: PathBuf,
}

fn load(path: &Path) -> (&'static Library, HsvToRgbFn) {
    // Leaking the `Library` gives the resolved symbol a `'static` lifetime,
    // which keeps the harness free of self-referential structs. Test processes
    // are short-lived, so the leak is harmless.
    let library: &'static Library = Box::leak(Box::new(unsafe {
        Library::new(path).unwrap_or_else(|err| panic!("failed to load {}: {err}", path.display()))
    }));

    let symbol: Symbol<'static, HsvToRgbFn> = unsafe {
        library
            .get(b"hsv_to_rgb\0")
            .unwrap_or_else(|err| panic!("{} does not export hsv_to_rgb: {err}", path.display()))
    };

    (library, *symbol)
}

impl Implementations {
    pub fn load() -> Self {
        let c_path = c_library_path();
        let rust_path = rust_library_path();

        let (_c_lib, c) = load(&c_path);
        let (_rust_lib, rust) = load(&rust_path);

        Self {
            c,
            rust,
            c_path,
            rust_path,
        }
    }
}

/// Sentinel written around the 3-element output window so that any write past
/// the documented bounds is detected.
const GUARD: u32 = 0xDEAD_BEEF;
const GUARD_SLOTS: usize = 3;

/// Calls one implementation with guard values surrounding the output window.
///
/// Returns the three result words as raw bits plus the guard words, so that
/// comparisons are exact (including NaN payloads and the sign of zero).
fn call(f: HsvToRgbFn, input: [f32; 3]) -> ([u32; 3], [u32; GUARD_SLOTS * 2]) {
    let mut buffer = [f32::from_bits(GUARD); GUARD_SLOTS * 2 + 3];
    let src = input;

    unsafe {
        f(buffer.as_mut_ptr().add(GUARD_SLOTS), src.as_ptr());
    }

    let out = [
        buffer[GUARD_SLOTS].to_bits(),
        buffer[GUARD_SLOTS + 1].to_bits(),
        buffer[GUARD_SLOTS + 2].to_bits(),
    ];

    let mut guards = [0u32; GUARD_SLOTS * 2];
    for i in 0..GUARD_SLOTS {
        guards[i] = buffer[i].to_bits();
        guards[GUARD_SLOTS + i] = buffer[GUARD_SLOTS + 3 + i].to_bits();
    }

    (out, guards)
}

fn describe(bits: [u32; 3]) -> String {
    format!(
        "[{:#010x} ({}), {:#010x} ({}), {:#010x} ({})]",
        bits[0],
        f32::from_bits(bits[0]),
        bits[1],
        f32::from_bits(bits[1]),
        bits[2],
        f32::from_bits(bits[2])
    )
}

/// Runs both implementations on `input` and asserts byte-identical output.
pub fn assert_same(impls: &Implementations, input: [f32; 3]) {
    let (c_out, c_guards) = call(impls.c, input);
    let (rust_out, rust_guards) = call(impls.rust, input);

    let input_desc = format!(
        "h={} ({:#010x}), s={} ({:#010x}), v={} ({:#010x})",
        input[0],
        input[0].to_bits(),
        input[1],
        input[1].to_bits(),
        input[2],
        input[2].to_bits()
    );

    assert_eq!(
        c_out,
        rust_out,
        "output mismatch for {input_desc}\n  C   : {}\n  Rust: {}",
        describe(c_out),
        describe(rust_out)
    );

    for (i, guard) in c_guards.iter().enumerate() {
        assert_eq!(
            *guard, GUARD,
            "C implementation wrote outside the 3-float output window (guard {i}) for {input_desc}"
        );
    }
    for (i, guard) in rust_guards.iter().enumerate() {
        assert_eq!(
            *guard, GUARD,
            "Rust implementation wrote outside the 3-float output window (guard {i}) for {input_desc}"
        );
    }
}

/// Number of iterations for the randomised tests, scaled by the `FUZZ_SCALE`
/// environment variable so a heavier soak run can be requested without editing
/// the tests.
pub fn fuzz_iterations(base: u64) -> u64 {
    let scale: u64 = std::env::var("FUZZ_SCALE")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(1)
        .max(1);
    base.saturating_mul(scale)
}

/// Small deterministic PRNG (xorshift64*) so failures are reproducible.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed | 1)
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

    /// Uniform in `[0, 1)`.
    pub fn next_unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }

    /// Uniform in `[lo, hi)`.
    pub fn next_range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + self.next_unit() * (hi - lo)
    }
}
