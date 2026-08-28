//! Shared harness for the C-vs-Rust differential tests.
//!
//! Both libraries are loaded as shared objects with `libloading` and called
//! only through their exported `colourblind` symbol, so the `#[no_mangle]`
//! `extern "C"` wrapper is exercised exactly as an external consumer would
//! exercise it. Nothing in the Rust crate is called directly.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};

/// `void colourblind(cb_impairment, float*, float*, float*)`.
///
/// The first parameter is declared `u32` because gcc picks `unsigned int` as the
/// enum's compatible type (`cmpl $0x2 / ja` in the disassembly). Tests that need
/// to push a negative or 64-bit-dirty value use [`Lib::call_raw64`].
pub type ColourblindFn = unsafe extern "C" fn(u32, *mut f32, *mut f32, *mut f32);

/// Same symbol, re-declared with a 64-bit first parameter so a test can dirty
/// the upper half of `rdi` and prove the callee only looks at `edi`.
pub type ColourblindFn64 = unsafe extern "C" fn(u64, *mut f32, *mut f32, *mut f32);

/// Same symbol with a **signed** first parameter. A C enum accepts any `int`,
/// so this is how a real caller passes a negative impairment.
pub type ColourblindFnI32 = unsafe extern "C" fn(i32, *mut f32, *mut f32, *mut f32);

pub const CB_PROTANOPIA: u32 = 0;
pub const CB_DEUTERANOPIA: u32 = 1;
pub const CB_TRITANOPIA: u32 = 2;

/// The three valid impairments, in declaration order.
pub const VALID: [u32; 3] = [CB_PROTANOPIA, CB_DEUTERANOPIA, CB_TRITANOPIA];

pub fn impairment_name(i: u32) -> &'static str {
    match i {
        CB_PROTANOPIA => "cbProtanopia",
        CB_DEUTERANOPIA => "cbDeuteranopia",
        CB_TRITANOPIA => "cbTritanopia",
        _ => "<out-of-range>",
    }
}

/// One loaded shared object plus its resolved `colourblind` symbol.
pub struct Lib {
    /// Kept alive so the resolved symbol stays valid.
    _lib: Library,
    pub path: PathBuf,
    pub which: &'static str,
    f: ColourblindFn,
    f64_: ColourblindFn64,
    fi32: ColourblindFnI32,
}

impl Lib {
    fn open(which: &'static str, path: PathBuf) -> Lib {
        // SAFETY: the path points at one of the two libraries under test; both
        // are plain leaf libraries with no initialisers.
        let lib = unsafe { Library::new(&path) }
            .unwrap_or_else(|e| panic!("failed to dlopen {which} .so at {}: {e}", path.display()));
        let f = unsafe {
            let s: Symbol<ColourblindFn> = lib
                .get(b"colourblind\0")
                .unwrap_or_else(|e| panic!("{which} .so has no `colourblind` symbol: {e}"));
            *s
        };
        let f64_ = unsafe {
            let s: Symbol<ColourblindFn64> = lib.get(b"colourblind\0").unwrap();
            *s
        };
        let fi32 = unsafe {
            let s: Symbol<ColourblindFnI32> = lib.get(b"colourblind\0").unwrap();
            *s
        };
        Lib { _lib: lib, path, which, f, f64_, fi32 }
    }

    /// Calls `colourblind(imp, &mut rgb[0], &mut rgb[1], &mut rgb[2])`.
    pub fn call(&self, imp: u32, rgb: &mut [f32; 3]) {
        let p = rgb.as_mut_ptr();
        // SAFETY: three valid, distinct, aligned `*mut f32` into a live array.
        unsafe { (self.f)(imp, p, p.add(1), p.add(2)) }
    }

    /// Calls with caller-chosen pointers, so aliasing / misalignment /
    /// permutation / NULL layouts can be reproduced exactly.
    ///
    /// # Safety
    /// The caller guarantees the pointers are valid for the C's unconditional
    /// dereference, or accepts the fault (see the NULL rows in `ERRORS.md`).
    pub unsafe fn call_ptrs(&self, imp: u32, r: *mut f32, g: *mut f32, b: *mut f32) {
        (self.f)(imp, r, g, b)
    }

    /// Calls with a 64-bit first argument (dirty upper half of `rdi`).
    ///
    /// # Safety
    /// As [`Lib::call_ptrs`].
    pub unsafe fn call_raw64(&self, imp: u64, r: *mut f32, g: *mut f32, b: *mut f32) {
        (self.f64_)(imp, r, g, b)
    }

    /// Calls with a **signed** `int` impairment, as C permits.
    pub fn call_i32(&self, imp: i32, rgb: &mut [f32; 3]) {
        let p = rgb.as_mut_ptr();
        // SAFETY: three valid, distinct, aligned `*mut f32` into a live array.
        unsafe { (self.fi32)(imp, p, p.add(1), p.add(2)) }
    }
}

/// Locates the C `.so`. The CMake project names the library after the parent
/// directory, so the file name is not fixed — glob for it instead of hardcoding.
fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    let build = Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/build");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| {
            panic!(
                "cannot read {}: {e}\nBuild the C first:\n  cd c_src && mkdir -p build && cd build \
                 && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
                build.display()
            )
        })
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.extension().is_some_and(|x| x == "so")
                && p.file_name().is_some_and(|n| n.to_string_lossy().starts_with("lib"))
        })
        .collect();
    found.sort();
    assert_eq!(
        found.len(),
        1,
        "expected exactly one lib*.so in {}, found {found:?}",
        build.display()
    );
    found.pop().unwrap()
}

/// Locates the Rust `cdylib` for the *same* profile the test binary was built
/// with, by walking up from the test executable (`target/<profile>/deps/<test>`).
///
/// # Why this also *builds* the cdylib
///
/// The crate is `crate-type = ["cdylib"]` and no test target links against it
/// (deliberately — every call must cross the FFI boundary). Cargo therefore has
/// no reason to build the `cdylib` during `cargo test`, and will happily run the
/// suite against a **stale** `.so` left over from an earlier `cargo build`. That
/// makes the whole differential suite silently vacuous: edits to `src/lib.rs`
/// are simply not under test.
///
/// So the harness builds the library itself (once per process) and then asserts
/// the artifact is newer than every Rust source file. If either step fails the
/// tests fail loudly rather than passing against yesterday's binary.
fn rust_so_path() -> PathBuf {
    static ONCE: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    ONCE.get_or_init(build_and_locate_rust_so).clone()
}

fn build_and_locate_rust_so() -> PathBuf {
    let name = format!(
        "{}colourblind_lib{}",
        std::env::consts::DLL_PREFIX,
        std::env::consts::DLL_SUFFIX
    );

    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }

    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|deps| deps.parent())
        .expect("target/<profile>/deps/<test>")
        .to_path_buf();
    let profile_name = profile_dir
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "debug".to_string());

    // Rebuild the cdylib for exactly this profile.
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let mut cmd = std::process::Command::new(&cargo);
    cmd.arg("build").arg("--lib").arg("--manifest-path").arg(&manifest);
    match profile_name.as_str() {
        "debug" => {}
        "release" => {
            cmd.arg("--release");
        }
        other => {
            cmd.args(["--profile", other]);
        }
    }
    // Propagate the feature selection the test binary was compiled with, so the
    // cdylib under test matches the harness's configuration.
    if let Ok(f) = std::env::var("CB_BUILD_FEATURES") {
        cmd.arg("--no-default-features");
        if !f.is_empty() {
            cmd.args(["--features", &f]);
        }
    }
    // Avoid inheriting the parent cargo's job-server / target-dir overrides in a
    // way that would confuse the nested invocation.
    cmd.env_remove("CARGO_MAKEFLAGS");
    cmd.env_remove("RUSTC_WORKSPACE_WRAPPER");

    let out = cmd.output();
    match out {
        Ok(o) if o.status.success() => {}
        Ok(o) => panic!(
            "nested `cargo build --lib` failed (needed so the tests do not run against a \
             stale cdylib):\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&o.stdout),
            String::from_utf8_lossy(&o.stderr)
        ),
        Err(e) => panic!("could not spawn `{cargo} build --lib`: {e}"),
    }

    let p = profile_dir.join(&name);
    assert!(
        p.exists(),
        "Rust cdylib not found at {} even after building it",
        p.display()
    );

    // Freshness gate: the artifact must be at least as new as every Rust source.
    let so_mtime = std::fs::metadata(&p).and_then(|m| m.modified()).expect("so mtime");
    let src_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    for entry in std::fs::read_dir(&src_dir).expect("read src/") {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "rs") {
            let src_mtime = entry
                .metadata()
                .and_then(|m| m.modified())
                .expect("src mtime");
            assert!(
                so_mtime >= src_mtime,
                "STALE ARTIFACT: {} is older than {}. The differential suite would be \
                 testing an out-of-date library. Run `cargo build` for the `{profile_name}` \
                 profile and re-run the tests.",
                p.display(),
                path.display()
            );
        }
    }
    p
}

/// Loads both libraries. Called once per test (cheap: `dlopen` refcounts).
pub fn both() -> (Lib, Lib) {
    (Lib::open("C", c_so_path()), Lib::open("Rust", rust_so_path()))
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seed, reproducible across runs.
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5EED_C0DE_1234_5678;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    /// Uniform in `[0, 1)`.
    pub fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }
    /// Uniform over *all* 2^32 bit patterns, so NaNs, infinities and
    /// subnormals appear naturally.
    pub fn any_f32(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
    /// Log-uniform normal magnitude with a random sign, spanning the whole
    /// normal `f32` exponent range.
    pub fn wide_normal(&mut self) -> f32 {
        let exp = self.below(254) + 1; // 1..=254 -> a normal exponent
        let mantissa = self.next_u32() & 0x007F_FFFF;
        let sign = (self.next_u32() & 1) << 31;
        f32::from_bits(sign | (exp << 23) | mantissa)
    }
    pub fn subnormal(&mut self) -> f32 {
        let mantissa = (self.next_u32() & 0x007F_FFFF).max(1);
        let sign = (self.next_u32() & 1) << 31;
        f32::from_bits(sign | mantissa)
    }
    /// Quiet NaN with a random non-zero payload and a random sign.
    pub fn qnan(&mut self) -> f32 {
        let payload = (self.next_u32() & 0x003F_FFFF) | 0x0000_0001;
        let sign = (self.next_u32() & 1) << 31;
        f32::from_bits(sign | 0x7F80_0000 | 0x0040_0000 | payload)
    }
    /// Signalling NaN: exponent all ones, quiet bit clear, payload non-zero.
    pub fn snan(&mut self) -> f32 {
        let payload = (self.next_u32() & 0x003F_FFFF) | 0x0000_0001;
        let sign = (self.next_u32() & 1) << 31;
        f32::from_bits(sign | 0x7F80_0000 | payload)
    }
    /// Exact power of two, random sign, exponent in a range where the products
    /// stay finite.
    pub fn power_of_two(&mut self) -> f32 {
        let exp = self.below(60) + 60; // 60..=119 biased exponent
        let sign = (self.next_u32() & 1) << 31;
        f32::from_bits(sign | (exp << 23))
    }
}

// ---------------------------------------------------------------------------
// Bit-exact comparison
// ---------------------------------------------------------------------------

pub fn bits(v: &[f32; 3]) -> [u32; 3] {
    [v[0].to_bits(), v[1].to_bits(), v[2].to_bits()]
}

fn show(v: &[f32; 3]) -> String {
    format!(
        "[{:08x} {:08x} {:08x}] ({:e}, {:e}, {:e})",
        v[0].to_bits(),
        v[1].to_bits(),
        v[2].to_bits(),
        v[0],
        v[1],
        v[2]
    )
}

/// Asserts the two outputs are bit-identical, reporting the raw bit patterns
/// (so NaN payloads and signed zeros are visible) plus the exact input.
#[track_caller]
pub fn assert_same(ctx: &str, imp: u32, input: &[f32; 3], c_out: &[f32; 3], rust_out: &[f32; 3]) {
    if bits(c_out) != bits(rust_out) {
        panic!(
            "MISMATCH in {ctx}\n  impairment: {} ({imp})\n  input : {}\n  C     : {}\n  Rust  : {}",
            impairment_name(imp),
            show(input),
            show(c_out),
            show(rust_out),
        );
    }
}

/// Runs one differential comparison over three distinct, aligned pointers.
#[track_caller]
pub fn diff(c: &Lib, rust: &Lib, ctx: &str, imp: u32, input: [f32; 3]) {
    let mut a = input;
    let mut b = input;
    c.call(imp, &mut a);
    rust.call(imp, &mut b);
    assert_same(ctx, imp, &input, &a, &b);
}
