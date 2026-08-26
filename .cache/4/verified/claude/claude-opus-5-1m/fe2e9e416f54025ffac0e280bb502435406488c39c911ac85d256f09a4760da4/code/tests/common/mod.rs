//! Shared differential-testing harness.
//!
//! Both the C `.so` and the Rust `.so` are loaded with `libloading` and driven
//! exclusively through their exported `bitwriter_add` symbol. The Rust crate is
//! **never** linked directly, so the `#[unsafe(no_mangle)] extern "C"` wrapper
//! and the `#[repr(C)]` struct ABI are part of what is under test.

#![allow(dead_code)]

use std::ffi::c_int;
use std::path::{Path, PathBuf};

/// Mirror of `struct tflac_bitwriter` from `c_src/include/lib.h`.
///
/// x86-64 layout: `val`@0 `bits`@8 `pos`@12 `len`@16 `tot`@20 `buffer`@24,
/// size 32, align 8, **no interior padding** — which makes the raw-byte
/// comparison in [`Bw::bytes`] well defined.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bw {
    pub val: u64,
    pub bits: u32,
    pub pos: u32,
    pub len: u32,
    pub tot: u32,
    pub buffer: *mut u8,
}

impl Bw {
    pub const fn zeroed() -> Self {
        Bw { val: 0, bits: 0, pos: 0, len: 0, tot: 0, buffer: std::ptr::null_mut() }
    }

    /// The full 32-byte object representation, used for byte-exact comparison.
    pub fn bytes(&self) -> [u8; 32] {
        let mut out = [0u8; 32];
        unsafe {
            std::ptr::copy_nonoverlapping(
                self as *const Bw as *const u8,
                out.as_mut_ptr(),
                std::mem::size_of::<Bw>(),
            );
        }
        out
    }
}

pub type BitwriterAddFn = unsafe extern "C" fn(*mut Bw, u32, u64) -> c_int;

/// One loaded implementation (C or Rust).
pub struct Impl {
    /// Kept alive so `f` stays valid; must be dropped last.
    _lib: libloading::Library,
    pub f: BitwriterAddFn,
    pub which: &'static str,
    pub path: PathBuf,
}

impl Impl {
    fn open(path: &Path, which: &'static str) -> Impl {
        let lib = unsafe { libloading::Library::new(path) }
            .unwrap_or_else(|e| panic!("failed to dlopen {} ({}): {e}", path.display(), which));
        let f: BitwriterAddFn = unsafe {
            *lib.get::<BitwriterAddFn>(b"bitwriter_add\0").unwrap_or_else(|e| {
                panic!("{} .so does not export `bitwriter_add`: {e}", which)
            })
        };
        Impl { _lib: lib, f, which, path: path.to_path_buf() }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `c_src/build/libtranslated_rust.so`, built by CMake.
pub fn c_so_path() -> PathBuf {
    let base = manifest_dir().join("c_src").join("build");
    for name in ["libtranslated_rust.so", "libc_src.so"] {
        let p = base.join(name);
        if p.exists() {
            return p;
        }
    }
    // Fall back to whatever single .so CMake produced.
    if let Ok(rd) = std::fs::read_dir(&base) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().is_some_and(|x| x == "so") {
                return p;
            }
        }
    }
    panic!(
        "C shared library not found under {}. Build it with:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        base.display()
    )
}

/// The Rust `cdylib`, located relative to the running test executable so that
/// it is picked up from the *same* profile (`debug` / `release`) under test.
pub fn rust_so_path() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    // .../target/<profile>/deps/<testbin>
    let deps = exe.parent().expect("deps dir");
    let profile = deps.parent().expect("profile dir");
    for dir in [profile, deps] {
        let p = dir.join("libbitwriter_add_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "Rust cdylib libbitwriter_add_lib.so not found in {} or {}",
        profile.display(),
        deps.display()
    )
}

/// Guard against testing a stale artefact.
///
/// `cargo test` builds the *test* targets but does **not** necessarily refresh
/// the `cdylib` artefact, so without this check an edit to `src/lib.rs` can be
/// silently ignored and the whole suite can "pass" against the previous
/// library. Refuse to run in that case.
fn assert_not_stale(so: &Path, src: &Path, how_to_fix: &str) {
    let so_t = match std::fs::metadata(so).and_then(|m| m.modified()) {
        Ok(t) => t,
        Err(_) => return,
    };
    let src_t = match std::fs::metadata(src).and_then(|m| m.modified()) {
        Ok(t) => t,
        Err(_) => return,
    };
    assert!(
        so_t >= src_t,
        "STALE ARTEFACT: {} is older than {}.\n\
         The differential tests would be comparing against an out-of-date \
         library and could pass spuriously.\nRebuild with: {}",
        so.display(),
        src.display(),
        how_to_fix
    );
}

pub fn load_c() -> Impl {
    let so = c_so_path();
    assert_not_stale(
        &so,
        &manifest_dir().join("c_src").join("src").join("lib.c"),
        "cd c_src/build && cmake --build .",
    );
    Impl::open(&so, "C")
}

pub fn load_rust() -> Impl {
    let so = rust_so_path();
    assert_not_stale(
        &so,
        &manifest_dir().join("src").join("lib.rs"),
        "cargo build --no-default-features   (before `cargo test`)",
    );
    Impl::open(&so, "Rust")
}

/// Both implementations, loaded once per test.
pub struct Pair {
    pub c: Impl,
    pub rust: Impl,
}

pub fn load_pair() -> Pair {
    Pair { c: load_c(), rust: load_rust() }
}

/// Outcome of one `bitwriter_add` invocation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Outcome {
    pub rc: c_int,
    pub post: [u8; 32],
}

impl Pair {
    /// Run a single call on both implementations from the identical pre-state.
    pub fn call(&self, pre: &Bw, bits: u32, val: u64) -> (Outcome, Outcome) {
        let mut cbw = *pre;
        let mut rbw = *pre;
        let crc = unsafe { (self.c.f)(&mut cbw, bits, val) };
        let rrc = unsafe { (self.rust.f)(&mut rbw, bits, val) };
        (Outcome { rc: crc, post: cbw.bytes() }, Outcome { rc: rrc, post: rbw.bytes() })
    }

    /// Assert one call agrees byte-for-byte; `ctx` names the CONFIGS/ERRORS row.
    pub fn assert_same(&self, ctx: &str, pre: &Bw, bits: u32, val: u64) {
        let (c, r) = self.call(pre, bits, val);
        if c != r {
            panic!(
                "DIVERGENCE [{ctx}]\n  \
                 pre     = val={:#018x} bits={} pos={} len={} tot={} buffer={:?}\n  \
                 bits    = {bits} ({bits:#x})\n  \
                 val     = {val:#018x}\n  \
                 C    rc={} post={}\n  \
                 Rust rc={} post={}\n  \
                 field-wise: {}",
                pre.val,
                pre.bits,
                pre.pos,
                pre.len,
                pre.tot,
                pre.buffer,
                c.rc,
                hex(&c.post),
                r.rc,
                hex(&r.post),
                fieldwise(&c.post, &r.post),
            );
        }
    }

    /// Assert a *sequence* of calls agrees, carrying `bw` state forward in both
    /// implementations independently (CONFIGS row 24).
    pub fn assert_same_sequence(&self, ctx: &str, pre: &Bw, steps: &[(u32, u64)]) {
        let mut cbw = *pre;
        let mut rbw = *pre;
        for (i, &(bits, val)) in steps.iter().enumerate() {
            let crc = unsafe { (self.c.f)(&mut cbw, bits, val) };
            let rrc = unsafe { (self.rust.f)(&mut rbw, bits, val) };
            if crc != rrc || cbw.bytes() != rbw.bytes() {
                panic!(
                    "DIVERGENCE [{ctx}] at step {i} of {}\n  \
                     bits={bits} ({bits:#x}) val={val:#018x}\n  \
                     C    rc={crc} post={}\n  \
                     Rust rc={rrc} post={}\n  \
                     field-wise: {}",
                    steps.len(),
                    hex(&cbw.bytes()),
                    hex(&rbw.bytes()),
                    fieldwise(&cbw.bytes(), &rbw.bytes()),
                );
            }
        }
    }
}

pub fn hex(b: &[u8; 32]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect::<Vec<_>>().join("")
}

/// Human-readable per-field diff of two 32-byte struct images.
pub fn fieldwise(c: &[u8; 32], r: &[u8; 32]) -> String {
    let u64at = |b: &[u8; 32], o: usize| u64::from_ne_bytes(b[o..o + 8].try_into().unwrap());
    let u32at = |b: &[u8; 32], o: usize| u32::from_ne_bytes(b[o..o + 4].try_into().unwrap());
    let mut out = Vec::new();
    macro_rules! chk64 {
        ($name:literal, $off:expr) => {
            if u64at(c, $off) != u64at(r, $off) {
                out.push(format!(
                    "{}: C={:#018x} Rust={:#018x}",
                    $name,
                    u64at(c, $off),
                    u64at(r, $off)
                ));
            }
        };
    }
    macro_rules! chk32 {
        ($name:literal, $off:expr) => {
            if u32at(c, $off) != u32at(r, $off) {
                out.push(format!(
                    "{}: C={} ({:#010x}) Rust={} ({:#010x})",
                    $name,
                    u32at(c, $off),
                    u32at(c, $off),
                    u32at(r, $off),
                    u32at(r, $off)
                ));
            }
        };
    }
    chk64!("val", 0);
    chk32!("bits", 8);
    chk32!("pos", 12);
    chk32!("len", 16);
    chk32!("tot", 20);
    chk64!("buffer", 24);
    if out.is_empty() {
        "(fields equal — difference is in padding/rc)".to_string()
    } else {
        out.join(", ")
    }
}

/// Deterministic SplitMix64 — fixed seeds keep every randomised row reproducible.
pub struct Rng(pub u64);

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
    /// Uniform in `0..n` (n > 0).
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
    /// Inclusive range.
    pub fn range(&mut self, lo: u32, hi: u32) -> u32 {
        lo + self.below(hi - lo + 1)
    }

    /// A `val` argument biased towards structurally interesting bit patterns.
    pub fn interesting_u64(&mut self) -> u64 {
        match self.below(10) {
            0 => 0,
            1 => u64::MAX,
            2 => 1,
            3 => 0xAAAA_AAAA_AAAA_AAAA,
            4 => 0x5555_5555_5555_5555,
            5 => 1u64 << (self.below(64)),
            6 => u64::MAX >> (self.below(64)),
            7 => u64::MAX << (self.below(64)),
            8 => self.next_u64() & 1, // exercises the `mask` bit-0 clear
            _ => self.next_u64(),
        }
    }

    /// A `bits` argument biased towards the boundaries the C code branches on.
    pub fn interesting_bits(&mut self) -> u32 {
        match self.below(12) {
            0 => 0,
            1 => 1,
            2 => 63,
            3 => 64,
            4 => 65,
            5 => 62,
            6 => 128,
            7 => u32::MAX,
            8 => self.range(0, 64),
            9 => self.range(60, 200),
            10 => u32::MAX - self.below(200),
            _ => self.next_u32(),
        }
    }

    /// A `bw->bits` pre-state biased towards the boundaries.
    pub fn interesting_bwbits(&mut self) -> u32 {
        match self.below(10) {
            0 => 0,
            1 => 63,
            2 => 64,
            3 => 65,
            4 => 62,
            5 => u32::MAX,
            6 => self.range(0, 64),
            7 => self.range(0, 200),
            8 => u32::MAX - self.below(200),
            _ => self.next_u32(),
        }
    }

    /// A fully random pre-state (CONFIGS row 25).
    pub fn interesting_pre(&mut self) -> Bw {
        Bw {
            val: self.interesting_u64(),
            bits: self.interesting_bwbits(),
            pos: self.next_u32(),
            len: self.next_u32(),
            tot: match self.below(4) {
                0 => 0,
                1 => u32::MAX,
                2 => u32::MAX - self.below(300),
                _ => self.next_u32(),
            },
            buffer: match self.below(3) {
                0 => std::ptr::null_mut(),
                1 => 0xDEAD_BEEF_0000_1000u64 as *mut u8,
                _ => self.next_u64() as *mut u8,
            },
        }
    }
}

/// Number of bits in `tflac_uint`.
pub const UINT_BITS: u32 = 64;
