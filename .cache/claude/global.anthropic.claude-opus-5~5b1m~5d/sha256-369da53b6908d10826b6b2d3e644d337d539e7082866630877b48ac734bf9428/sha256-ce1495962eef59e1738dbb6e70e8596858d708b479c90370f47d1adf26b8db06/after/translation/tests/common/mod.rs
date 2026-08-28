//! Shared differential-test harness.
//!
//! BOTH implementations are loaded as shared objects through `libloading` and
//! called through their exported C ABI symbol `bitwriter_add`.  The Rust
//! implementation is *never* called directly as a Rust function — it is always
//! reached via `dlopen`/`dlsym` on `libbitwriter_add_lib.so`, exactly as an
//! external C consumer would, so the `#[no_mangle] extern "C"` wrapper and the
//! `#[repr(C)]` struct layout are under test too.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// struct tflac_bitwriter  (c_src/include/lib.h)
// ---------------------------------------------------------------------------

/// Mirror of the C `struct tflac_bitwriter`.
///
/// x86-64 layout: val@0(8) bits@8(4) pos@12(4) len@16(4) tot@20(4) buffer@24(8),
/// sizeof == 32, alignof == 8.
#[repr(C)]
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Bitwriter {
    pub val: u64,
    pub bits: u32,
    pub pos: u32,
    pub len: u32,
    pub tot: u32,
    pub buffer: *mut u8,
}

impl Bitwriter {
    pub const fn zeroed() -> Self {
        Bitwriter { val: 0, bits: 0, pos: 0, len: 0, tot: 0, buffer: std::ptr::null_mut() }
    }

    pub fn new(val: u64, bits: u32, pos: u32, len: u32, tot: u32, buffer: usize) -> Self {
        Bitwriter { val, bits, pos, len, tot, buffer: buffer as *mut u8 }
    }
}

/// `int bitwriter_add(tflac_bitwriter *bw, tflac_u32 bits, tflac_uint val);`
pub type BitwriterAddFn = unsafe extern "C" fn(*mut Bitwriter, u32, u64) -> std::ffi::c_int;

// ---------------------------------------------------------------------------
// .so discovery + loading
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `c_src/build/lib<parent-dir-name>.so` — the name is derived from the parent
/// directory name by `c_src/CMakeLists.txt`, so glob for it instead of guessing.
pub fn c_so_path() -> PathBuf {
    let build_dir = manifest_dir().join("../c_src/build");
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build_dir) {
        for e in rd.flatten() {
            let p = e.path();
            let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if name.starts_with("lib") && name.ends_with(".so") {
                found.push(p);
            }
        }
    }
    found.sort();
    match found.into_iter().next() {
        Some(p) => p,
        None => panic!(
            "C shared library not found in {}.\nBuild it first:\n  cd c_src && mkdir -p build \
             && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build_dir.display()
        ),
    }
}

/// `target/{debug,release}/libbitwriter_add_lib.so` — the crate's own cdylib.
pub fn rust_so_path() -> PathBuf {
    let name = "libbitwriter_add_lib.so";
    let base = manifest_dir().join("target");
    // Prefer the profile the tests themselves were built with.
    let order: [&str; 2] =
        if cfg!(debug_assertions) { ["debug", "release"] } else { ["release", "debug"] };
    for profile in order {
        let p = base.join(profile).join(name);
        if p.is_file() {
            return p;
        }
    }
    panic!(
        "Rust cdylib {} not found under {}.\nBuild it first:  cargo build && cargo build --release",
        name,
        base.display()
    );
}

/// A loaded implementation: owns the `Library` and a cached raw fn pointer.
pub struct Impl {
    _lib: Library,
    f: BitwriterAddFn,
    pub name: &'static str,
}

impl Impl {
    pub fn load(path: &Path, name: &'static str) -> Impl {
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()));
        let f = {
            let sym: Symbol<BitwriterAddFn> = unsafe { lib.get(b"bitwriter_add\0") }
                .unwrap_or_else(|e| panic!("dlsym(bitwriter_add) in {}: {e}", path.display()));
            *sym
        };
        Impl { _lib: lib, f, name }
    }

    #[inline]
    pub fn call(&self, bw: &mut Bitwriter, bits: u32, val: u64) -> i32 {
        unsafe { (self.f)(bw as *mut Bitwriter, bits, val) }
    }

    /// The raw `dlsym`'d function pointer, for callers that need to pass a
    /// deliberately invalid `bw` (e.g. NULL) without going through `&mut`.
    #[inline]
    pub fn raw(&self) -> BitwriterAddFn {
        self.f
    }
}

/// The C `.so` and the Rust `.so`, both loaded via `libloading`.
pub struct Pair {
    pub c: Impl,
    pub rust: Impl,
}

pub fn load_pair() -> Pair {
    Pair {
        c: Impl::load(&c_so_path(), "C"),
        rust: Impl::load(&rust_so_path(), "Rust"),
    }
}

// ---------------------------------------------------------------------------
// differential comparison
// ---------------------------------------------------------------------------

/// Run one `bitwriter_add` call on both implementations starting from the same
/// initial state and compare the return value plus **all six** struct fields.
///
/// Returns `Err(report)` on any divergence.
pub fn diff_one(
    p: &Pair,
    init: Bitwriter,
    bits: u32,
    val: u64,
) -> Result<(Bitwriter, i32), String> {
    let mut cs = init;
    let mut rs = init;
    let rc_c = p.c.call(&mut cs, bits, val);
    let rc_r = p.rust.call(&mut rs, bits, val);

    let mut bad: Vec<String> = Vec::new();
    if rc_c != rc_r {
        bad.push(format!("return: C={rc_c} Rust={rc_r}"));
    }
    if cs.val != rs.val {
        bad.push(format!("val:  C=0x{:016x} Rust=0x{:016x}", cs.val, rs.val));
    }
    if cs.bits != rs.bits {
        bad.push(format!("bits: C={} Rust={}", cs.bits, rs.bits));
    }
    if cs.pos != rs.pos {
        bad.push(format!("pos:  C={} Rust={}", cs.pos, rs.pos));
    }
    if cs.len != rs.len {
        bad.push(format!("len:  C={} Rust={}", cs.len, rs.len));
    }
    if cs.tot != rs.tot {
        bad.push(format!("tot:  C={} Rust={}", cs.tot, rs.tot));
    }
    if cs.buffer != rs.buffer {
        bad.push(format!("buffer: C={:?} Rust={:?}", cs.buffer, rs.buffer));
    }

    if bad.is_empty() {
        Ok((cs, rc_c))
    } else {
        Err(format!(
            "DIVERGENCE\n  input : bits={bits} (0x{bits:08x})  val=0x{val:016x}\n  \
             initial: val=0x{:016x} bits={} pos={} len={} tot={} buffer={:?}\n  {}",
            init.val,
            init.bits,
            init.pos,
            init.len,
            init.tot,
            init.buffer,
            bad.join("\n  ")
        ))
    }
}

/// Assert a whole batch of cases; reports the first divergence and the count.
pub struct Checker<'a> {
    pub p: &'a Pair,
    pub cases: u64,
    pub failures: Vec<String>,
}

impl<'a> Checker<'a> {
    pub fn new(p: &'a Pair) -> Self {
        Checker { p, cases: 0, failures: Vec::new() }
    }

    pub fn check(&mut self, init: Bitwriter, bits: u32, val: u64) {
        self.cases += 1;
        if let Err(e) = diff_one(self.p, init, bits, val) {
            if self.failures.len() < 10 {
                self.failures.push(e);
            }
        }
    }

    /// Sequential/stateful run: both writers advance together, compared after
    /// every single call (composed-pipeline drift detection).
    pub fn check_sequence(&mut self, start: Bitwriter, steps: &[(u32, u64)]) {
        let mut cs = start;
        let mut rs = start;
        for (i, &(bits, val)) in steps.iter().enumerate() {
            self.cases += 1;
            let before_c = cs;
            let rc_c = self.p.c.call(&mut cs, bits, val);
            let rc_r = self.p.rust.call(&mut rs, bits, val);
            if rc_c != rc_r || cs != rs {
                if self.failures.len() < 10 {
                    self.failures.push(format!(
                        "SEQUENCE DIVERGENCE at step {i}\n  input : bits={bits} val=0x{val:016x}\n  \
                         state before: val=0x{:016x} bits={} tot={}\n  \
                         C   after: rc={rc_c} val=0x{:016x} bits={} pos={} len={} tot={}\n  \
                         Rust after: rc={rc_r} val=0x{:016x} bits={} pos={} len={} tot={}",
                        before_c.val, before_c.bits, before_c.tot,
                        cs.val, cs.bits, cs.pos, cs.len, cs.tot,
                        rs.val, rs.bits, rs.pos, rs.len, rs.tot,
                    ));
                }
                // resynchronise so one divergence doesn't cascade into 2000
                rs = cs;
            }
        }
    }

    pub fn finish(self, row: &str) {
        assert!(
            self.failures.is_empty(),
            "[{row}] {} of {} cases diverged.\n\n{}",
            self.failures.len(),
            self.cases,
            self.failures.join("\n\n")
        );
        assert!(self.cases > 0, "[{row}] no cases were generated");
        eprintln!("[{row}] OK: {} differential cases matched", self.cases);
    }
}

// ---------------------------------------------------------------------------
// deterministic RNG (SplitMix64) — fixed seeds, reproducible
// ---------------------------------------------------------------------------

pub struct Rng(pub u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed)
    }

    #[inline]
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    #[inline]
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    /// Uniform in `0..n` (n > 0).
    #[inline]
    pub fn below(&mut self, n: u32) -> u32 {
        assert!(n > 0);
        (self.next_u64() % n as u64) as u32
    }

    /// Inclusive range `lo..=hi`.
    #[inline]
    pub fn range(&mut self, lo: u32, hi: u32) -> u32 {
        assert!(lo <= hi);
        if lo == 0 && hi == u32::MAX {
            return self.next_u32();
        }
        lo + self.below(hi - lo + 1)
    }

    /// A `u64` biased towards interesting bit patterns.
    pub fn interesting_u64(&mut self) -> u64 {
        match self.next_u64() % 10 {
            0 => 0,
            1 => u64::MAX,
            2 => 1,
            3 => 1u64 << (self.next_u64() % 64),
            4 => !(1u64 << (self.next_u64() % 64)),
            5 => 0x0000_0000_FFFF_FFFF,
            6 => 0xFFFF_FFFF_0000_0000,
            7 => 0xAAAA_AAAA_AAAA_AAAA,
            8 => 0x5555_5555_5555_5555,
            _ => self.next_u64(),
        }
    }

    /// A `u32` biased towards the boundary values the C branches on.
    pub fn interesting_bits(&mut self) -> u32 {
        const EDGE: [u32; 14] = [
            0, 1, 63, 64, 65, 127, 128, 255, 256, 1000, 0x7FFF_FFFF, 0x8000_0000, 0xFFFF_FFFE,
            0xFFFF_FFFF,
        ];
        match self.next_u64() % 3 {
            0 => EDGE[(self.next_u64() % EDGE.len() as u64) as usize],
            1 => self.range(0, 70),
            _ => self.next_u32(),
        }
    }
}
