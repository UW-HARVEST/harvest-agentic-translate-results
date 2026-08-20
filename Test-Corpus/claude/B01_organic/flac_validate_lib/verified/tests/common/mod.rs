//! Shared differential-test harness.
//!
//! Both the C shared object and the Rust shared object are loaded with
//! `libloading` and driven purely through their exported C symbols. No Rust
//! function is ever called directly, so the `#[no_mangle] extern "C"` wrappers
//! are part of what is under test.

#![allow(dead_code)]

use std::ffi::c_int;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use libloading::{Library, Symbol};

// ---------------------------------------------------------------------------
// struct tflac — raw 28-byte representation (padding included)
// ---------------------------------------------------------------------------

/// `sizeof(struct tflac)` on x86_64-linux-gnu (verified with the C compiler).
pub const TFLAC_SIZE: usize = 28;

/// Byte offsets of every field, verified against the C compiler via `offsetof`.
pub const OFF_BLOCKSIZE: usize = 0;
pub const OFF_SAMPLERATE: usize = 4;
pub const OFF_CHANNELS: usize = 8;
pub const OFF_BITDEPTH: usize = 12;
pub const OFF_CHANNEL_MODE: usize = 16;
pub const OFF_MAX_RICE_VALUE: usize = 17;
pub const OFF_MIN_PARTITION_ORDER: usize = 18;
pub const OFF_MAX_PARTITION_ORDER: usize = 19;
pub const OFF_PARTITION_ORDER: usize = 20;
pub const OFF_PAD: usize = 21; // 3 bytes of tail padding before cur_blocksize
pub const OFF_CUR_BLOCKSIZE: usize = 24;

/// A correctly-aligned raw image of `struct tflac`.
///
/// Working on raw bytes (instead of a `repr(C)` Rust struct) lets the tests
/// pre-seed and then compare the **padding** bytes too, so a stray write by
/// either implementation cannot hide.
#[repr(C, align(4))]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Raw(pub [u8; TFLAC_SIZE]);

impl std::fmt::Debug for Raw {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Raw({:02x?}) = {:?}", self.0, Fields::from_raw(*self))
    }
}

/// Field-level view of `struct tflac`, including the tail padding.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub struct Fields {
    pub blocksize: u32,
    pub samplerate: u32,
    pub channels: u32,
    pub bitdepth: u32,
    pub channel_mode: u8,
    pub max_rice_value: u8,
    pub min_partition_order: u8,
    pub max_partition_order: u8,
    pub partition_order: u8,
    pub pad: [u8; 3],
    pub cur_blocksize: u32,
}

impl std::fmt::Debug for Fields {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "tflac{{ blocksize:{}, samplerate:{}, channels:{}, bitdepth:{}, \
             channel_mode:{}, max_rice_value:{}, min_po:{}, max_po:{}, po:{}, \
             pad:{:02x?}, cur_blocksize:{} }}",
            self.blocksize,
            self.samplerate,
            self.channels,
            self.bitdepth,
            self.channel_mode,
            self.max_rice_value,
            self.min_partition_order,
            self.max_partition_order,
            self.partition_order,
            self.pad,
            self.cur_blocksize
        )
    }
}

impl Fields {
    pub fn to_raw(self) -> Raw {
        let mut b = [0u8; TFLAC_SIZE];
        b[OFF_BLOCKSIZE..OFF_BLOCKSIZE + 4].copy_from_slice(&self.blocksize.to_ne_bytes());
        b[OFF_SAMPLERATE..OFF_SAMPLERATE + 4].copy_from_slice(&self.samplerate.to_ne_bytes());
        b[OFF_CHANNELS..OFF_CHANNELS + 4].copy_from_slice(&self.channels.to_ne_bytes());
        b[OFF_BITDEPTH..OFF_BITDEPTH + 4].copy_from_slice(&self.bitdepth.to_ne_bytes());
        b[OFF_CHANNEL_MODE] = self.channel_mode;
        b[OFF_MAX_RICE_VALUE] = self.max_rice_value;
        b[OFF_MIN_PARTITION_ORDER] = self.min_partition_order;
        b[OFF_MAX_PARTITION_ORDER] = self.max_partition_order;
        b[OFF_PARTITION_ORDER] = self.partition_order;
        b[OFF_PAD..OFF_PAD + 3].copy_from_slice(&self.pad);
        b[OFF_CUR_BLOCKSIZE..OFF_CUR_BLOCKSIZE + 4]
            .copy_from_slice(&self.cur_blocksize.to_ne_bytes());
        Raw(b)
    }

    pub fn from_raw(r: Raw) -> Self {
        let g4 = |o: usize| u32::from_ne_bytes(r.0[o..o + 4].try_into().unwrap());
        Fields {
            blocksize: g4(OFF_BLOCKSIZE),
            samplerate: g4(OFF_SAMPLERATE),
            channels: g4(OFF_CHANNELS),
            bitdepth: g4(OFF_BITDEPTH),
            channel_mode: r.0[OFF_CHANNEL_MODE],
            max_rice_value: r.0[OFF_MAX_RICE_VALUE],
            min_partition_order: r.0[OFF_MIN_PARTITION_ORDER],
            max_partition_order: r.0[OFF_MAX_PARTITION_ORDER],
            partition_order: r.0[OFF_PARTITION_ORDER],
            pad: r.0[OFF_PAD..OFF_PAD + 3].try_into().unwrap(),
            cur_blocksize: g4(OFF_CUR_BLOCKSIZE),
        }
    }

    /// A struct that passes every validation check, used as a base to perturb.
    pub fn valid_base() -> Self {
        Fields {
            blocksize: 4096,
            samplerate: 44100,
            channels: 2,
            bitdepth: 16,
            channel_mode: 0,
            max_rice_value: 0,
            min_partition_order: 0,
            max_partition_order: 8,
            partition_order: 0xAA,
            pad: [0xAA; 3],
            cur_blocksize: 0xAAAA_AAAA,
        }
    }
}

// ---------------------------------------------------------------------------
// Loaded implementation
// ---------------------------------------------------------------------------

pub type FlacValidateFn = unsafe extern "C" fn(*mut u8) -> c_int;
pub type SizeMemoryFn = unsafe extern "C" fn(u32) -> u32;

pub struct Impl {
    pub name: &'static str,
    pub path: PathBuf,
    pub flac_validate: FlacValidateFn,
    pub tflac_size_memory: SizeMemoryFn,
}

impl Impl {
    /// Runs `flac_validate` on a copy of `f` and returns `(ret, resulting struct)`.
    pub fn validate(&self, f: Fields) -> (c_int, Fields) {
        let mut raw = f.to_raw();
        let ret = unsafe { (self.flac_validate)(raw.0.as_mut_ptr()) };
        (ret, Fields::from_raw(raw))
    }

    /// Runs `flac_validate` on a raw byte image (used for fully-random fuzzing).
    pub fn validate_raw(&self, r: Raw) -> (c_int, Raw) {
        let mut raw = r;
        let ret = unsafe { (self.flac_validate)(raw.0.as_mut_ptr()) };
        (ret, raw)
    }

    pub fn size_memory(&self, blocksize: u32) -> u32 {
        unsafe { (self.tflac_size_memory)(blocksize) }
    }
}

fn load(path: PathBuf, name: &'static str) -> Impl {
    let lib: &'static Library = Box::leak(Box::new(
        unsafe { Library::new(&path) }
            .unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", path.display())),
    ));
    unsafe {
        let fv: Symbol<'static, FlacValidateFn> = lib
            .get(b"flac_validate\0")
            .unwrap_or_else(|e| panic!("{} is missing symbol flac_validate: {e}", path.display()));
        let sm: Symbol<'static, SizeMemoryFn> = lib.get(b"tflac_size_memory\0").unwrap_or_else(|e| {
            panic!("{} is missing symbol tflac_size_memory: {e}", path.display())
        });
        Impl {
            name,
            path,
            flac_validate: *fv,
            tflac_size_memory: *sm,
        }
    }
}

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `<target>/debug` (the directory holding the test executable's profile output).
fn profile_dir() -> PathBuf {
    // current_exe() == <target>/<profile>/deps/<test-binary>
    let exe = std::env::current_exe().expect("current_exe");
    exe.parent()
        .and_then(Path::parent)
        .expect("profile dir")
        .to_path_buf()
}

fn target_dir() -> PathBuf {
    profile_dir().parent().expect("target dir").to_path_buf()
}

/// Path to the C shared object, building it with CMake if it is not there yet.
pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("HARVEST_C_SO") {
        return PathBuf::from(p);
    }
    let c_src = manifest_dir().join("c_src");
    let build = c_src.join("build");
    // CMake derives the project (and therefore library) name from the parent
    // directory name of c_src/, i.e. `translated_rust`.
    let candidates = ["libtranslated_rust.so", "libc_src.so"];
    for c in candidates {
        let p = build.join(c);
        if p.is_file() {
            return p;
        }
    }
    // Not built yet: build it.
    std::fs::create_dir_all(&build).expect("mkdir c_src/build");
    let cfg = Command::new("cmake")
        .current_dir(&build)
        .args(["..", "-DCMAKE_POSITION_INDEPENDENT_CODE=ON"])
        .output()
        .expect("run cmake configure");
    assert!(
        cfg.status.success(),
        "cmake configure failed:\n{}",
        String::from_utf8_lossy(&cfg.stderr)
    );
    let bld = Command::new("cmake")
        .current_dir(&build)
        .args(["--build", "."])
        .output()
        .expect("run cmake build");
    assert!(
        bld.status.success(),
        "cmake build failed:\n{}",
        String::from_utf8_lossy(&bld.stderr)
    );
    for c in candidates {
        let p = build.join(c);
        if p.is_file() {
            return p;
        }
    }
    let found: Vec<_> = std::fs::read_dir(&build)
        .map(|d| d.filter_map(|e| e.ok()).map(|e| e.file_name()).collect())
        .unwrap_or_default();
    panic!("no C .so found in {}; contents: {found:?}", build.display());
}

/// Builds `libflac_validate_lib.so` for the given cargo profile if it is not
/// already present, and returns its path.
///
/// `cargo test` only builds the *test* targets, not the `cdylib` artifact, so
/// the harness makes sure both profile artifacts exist and then exercises each
/// of them against C:
///
/// * `release` — the artifact that actually ships (`panic = "abort"`,
///   `debug_assertions` off). Signal-level tests use this one.
/// * `debug`  — `debug_assertions` + overflow checks on, so any place where the
///   Rust code would panic while C silently wraps is caught.
fn rust_so_for_profile(profile: &str, extra_args: &[&str]) -> PathBuf {
    let env_key = format!("HARVEST_RUST_{}_SO", profile.to_uppercase());
    if let Ok(p) = std::env::var(&env_key) {
        return PathBuf::from(p);
    }
    let p = target_dir().join(profile).join("libflac_validate_lib.so");
    if p.is_file() {
        return p;
    }
    let mut cmd = Command::new(env!("CARGO"));
    cmd.current_dir(manifest_dir()).arg("build");
    cmd.args(extra_args);
    let out = cmd.output().expect("run cargo build");
    assert!(
        out.status.success(),
        "cargo build {extra_args:?} failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(p.is_file(), "{} still missing", p.display());
    p
}

/// Path to the *release* Rust shared object (the shipped artifact).
pub fn rust_release_so_path() -> PathBuf {
    rust_so_for_profile("release", &["--release"])
}

/// Path to the *debug* Rust shared object (overflow/debug assertions enabled).
pub fn rust_debug_so_path() -> PathBuf {
    rust_so_for_profile("debug", &[])
}

pub struct Pair {
    pub c: Impl,
    /// Every Rust `.so` that exists (the test profile's own build, plus the
    /// release build). Each is compared against C independently.
    pub rust: Vec<Impl>,
}

static PAIR: OnceLock<Pair> = OnceLock::new();

/// The C implementation plus every Rust implementation, loaded once per process.
pub fn pair() -> &'static Pair {
    PAIR.get_or_init(|| {
        let c = load(c_so_path(), "C");
        let mut rust = Vec::new();
        for (path, name) in [
            (rust_release_so_path(), "rust[release]"),
            (rust_debug_so_path(), "rust[debug]"),
        ] {
            if !rust.iter().any(|i: &Impl| i.path == path) {
                rust.push(load(path, name));
            }
        }
        assert!(!rust.is_empty(), "no Rust .so could be located");
        Pair { c, rust }
    })
}

// ---------------------------------------------------------------------------
// Assertions
// ---------------------------------------------------------------------------

/// Differentially runs `flac_validate` for `f` on C and every Rust `.so`.
pub fn check_validate(row: &str, f: Fields) {
    let p = pair();
    let (cret, cout) = p.c.validate(f);
    for r in &p.rust {
        let (rret, rout) = r.validate(f);
        assert_eq!(
            cret, rret,
            "[{row}] flac_validate return mismatch ({} vs {})\n  input: {f:?}\n  C ret={cret} out={cout:?}\n  R ret={rret} out={rout:?}",
            p.c.name, r.name
        );
        assert_eq!(
            cout.to_raw(), rout.to_raw(),
            "[{row}] flac_validate struct mismatch ({} vs {})\n  input: {f:?}\n  C ret={cret} out={cout:?}\n  R ret={rret} out={rout:?}",
            p.c.name, r.name
        );
    }
}

/// Same, but on a fully arbitrary raw 28-byte image.
pub fn check_validate_raw(row: &str, raw: Raw) {
    let p = pair();
    let (cret, cout) = p.c.validate_raw(raw);
    for r in &p.rust {
        let (rret, rout) = r.validate_raw(raw);
        assert_eq!(
            cret, rret,
            "[{row}] flac_validate return mismatch ({} vs {})\n  input: {raw:?}\n  C ret={cret} out={cout:?}\n  R ret={rret} out={rout:?}",
            p.c.name, r.name
        );
        assert_eq!(
            cout, rout,
            "[{row}] flac_validate struct mismatch ({} vs {})\n  input: {raw:?}\n  C ret={cret} out={cout:?}\n  R ret={rret} out={rout:?}",
            p.c.name, r.name
        );
    }
}

/// Differentially runs `flac_validate` and additionally asserts the exact
/// return code the C code is documented (in `ERRORS.md`) to produce.
pub fn check_validate_ret(row: &str, f: Fields, expect: c_int) {
    check_validate(row, f);
    let (cret, _) = pair().c.validate(f);
    assert_eq!(
        cret, expect,
        "[{row}] C returned {cret}, ERRORS.md/CONFIGS.md expects {expect}\n  input: {f:?}"
    );
}

pub fn check_size_memory(row: &str, blocksize: u32) {
    let p = pair();
    let cv = p.c.size_memory(blocksize);
    for r in &p.rust {
        let rv = r.size_memory(blocksize);
        assert_eq!(
            cv, rv,
            "[{row}] tflac_size_memory({blocksize}) mismatch: {}={cv} {}={rv}",
            p.c.name, r.name
        );
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) — fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed })
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
    pub fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }
    /// Uniform in `lo..=hi`.
    pub fn range_u32(&mut self, lo: u32, hi: u32) -> u32 {
        debug_assert!(lo <= hi);
        let span = (hi - lo) as u64 + 1;
        lo + (self.next_u64() % span) as u32
    }
    pub fn range_u8(&mut self, lo: u8, hi: u8) -> u8 {
        self.range_u32(lo as u32, hi as u32) as u8
    }
    pub fn pick<T: Copy>(&mut self, xs: &[T]) -> T {
        xs[(self.next_u64() % xs.len() as u64) as usize]
    }
    pub fn fill(&mut self, buf: &mut [u8]) {
        for b in buf.iter_mut() {
            *b = self.next_u8();
        }
    }
}

/// A random struct whose "output" fields and padding are poisoned, so that any
/// difference in which bytes get written shows up.
pub fn poisoned(rng: &mut Rng, mut f: Fields) -> Fields {
    f.partition_order = rng.next_u8();
    f.pad = [rng.next_u8(), rng.next_u8(), rng.next_u8()];
    f.cur_blocksize = rng.next_u32();
    f
}

/// Number of randomized iterations per property row (override with
/// `HARVEST_ITERS` for a quick run).
pub fn iters(default: usize) -> usize {
    std::env::var("HARVEST_ITERS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}
