//! Shared differential-test harness.
//!
//! BOTH implementations are loaded as shared objects through `libloading` and
//! called across the FFI boundary — the Rust functions are never called
//! directly, so the `#[no_mangle] extern "C"` export wrappers are under test
//! too.

#![allow(dead_code)]
#![allow(non_camel_case_types)]

use libloading::{Library, Symbol};
use std::path::PathBuf;
use std::sync::OnceLock;

/// Mirror of `struct tflac_bitwriter` from `c_src/include/lib.h`.
///
/// x86-64 layout: val@0 (8), bits@8, pos@12, len@16, tot@20, buffer@24 (8)
/// => size 32, align 8.  No interior padding on this target, but the struct is
/// compared as raw bytes anyway so any padding difference would be caught.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
        Bitwriter {
            val: 0,
            bits: 0,
            pos: 0,
            len: 0,
            tot: 0,
            buffer: std::ptr::null_mut(),
        }
    }

    /// Raw byte view of the whole struct, padding included.
    pub fn as_bytes(&self) -> [u8; std::mem::size_of::<Bitwriter>()] {
        // SAFETY: reading our own POD struct as bytes.
        unsafe { std::mem::transmute_copy(self) }
    }
}

pub type BitwriterAddFn = unsafe extern "C" fn(*mut Bitwriter, u32, u64) -> std::ffi::c_int;

pub struct Impls {
    pub c_lib: Library,
    pub rust_lib: Library,
}

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    // Allows re-running the whole differential suite against a C reference
    // built at a different optimisation level (see run_tests.sh).
    if let Ok(p) = std::env::var("C_REF_SO") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "C_REF_SO={} does not exist", p.display());
        return p;
    }
    let build = repo_root().join("c_src").join("build");
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&build) {
        for e in entries.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("so") {
                candidates.push(p);
            }
        }
    }
    candidates.sort();
    candidates.into_iter().next().unwrap_or_else(|| {
        panic!(
            "no C .so found in {}. Build it first:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

fn find_rust_so() -> PathBuf {
    // The test binary lives in target/<profile>/deps/, so walk up to the
    // profile dir and look for the cdylib there.
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("target/<profile>")
        .to_path_buf();

    let names = ["libbitwriter_add_lib.so", "libtranslation.so"];
    for dir in [profile_dir.clone(), repo_root().join("translation/target/release")] {
        for n in names {
            let p = dir.join(n);
            if p.exists() {
                return p;
            }
        }
    }
    panic!(
        "Rust cdylib not found next to {} — run `cargo build` first",
        profile_dir.display()
    );
}

/// `cargo test` does **not** rebuild a `crate-type = ["cdylib"]` target, so the
/// `.so` under test can silently be stale — which would make every
/// differential test pass against old code.  Refuse to run in that case.
fn assert_so_fresh(rust_so: &std::path::Path) {
    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs");
    let mtime = |p: &std::path::Path| {
        std::fs::metadata(p)
            .and_then(|m| m.modified())
            .unwrap_or_else(|e| panic!("stat {}: {e}", p.display()))
    };
    let so_t = mtime(rust_so);
    let src_t = mtime(&src);
    assert!(
        so_t >= src_t,
        "STALE Rust cdylib: {} is older than {}.\n\
         `cargo test` does not rebuild cdylib targets — run\n\
         `cargo build --release --offline && cargo test --release --offline`\n\
         (or ./run_tests.sh) so the tests exercise the current source.",
        rust_so.display(),
        src.display()
    );
}

static IMPLS: OnceLock<Impls> = OnceLock::new();

pub fn impls() -> &'static Impls {
    IMPLS.get_or_init(|| {
        let c_path = find_c_so();
        let rust_path = find_rust_so();
        assert_so_fresh(&rust_path);
        // SAFETY: both libraries are plain C-ABI shared objects with no
        // constructors that could misbehave.
        let c_lib = unsafe { Library::new(&c_path) }
            .unwrap_or_else(|e| panic!("dlopen {}: {e}", c_path.display()));
        let rust_lib = unsafe { Library::new(&rust_path) }
            .unwrap_or_else(|e| panic!("dlopen {}: {e}", rust_path.display()));
        Impls { c_lib, rust_lib }
    })
}

pub fn c_add() -> Symbol<'static, BitwriterAddFn> {
    unsafe { impls().c_lib.get(b"bitwriter_add\0") }.expect("C bitwriter_add")
}

pub fn rust_add() -> Symbol<'static, BitwriterAddFn> {
    unsafe { impls().rust_lib.get(b"bitwriter_add\0") }.expect("Rust bitwriter_add")
}

/// Run one `bitwriter_add` call through both `.so`s starting from the same
/// state and assert the return value and the full 32-byte struct match.
#[track_caller]
pub fn check_call(state: Bitwriter, bits: u32, val: u64, ctx: &str) {
    let c = c_add();
    let r = rust_add();

    let mut cs = state;
    let mut rs = state;

    let rc_c = unsafe { c(&mut cs as *mut Bitwriter, bits, val) };
    let rc_r = unsafe { r(&mut rs as *mut Bitwriter, bits, val) };

    assert_eq!(
        rc_c, rc_r,
        "return value differs [{ctx}] in={state:?} bits={bits} val={val:#018x}"
    );
    assert_eq!(
        cs.as_bytes(),
        rs.as_bytes(),
        "struct differs [{ctx}] in={state:?} bits={bits} val={val:#018x}\n  C   ={cs:?}\n  Rust={rs:?}"
    );
    // Fields the C never touches must be preserved identically.
    assert_eq!(cs.pos, state.pos, "C modified pos [{ctx}]");
    assert_eq!(cs.len, state.len, "C modified len [{ctx}]");
    assert_eq!(cs.buffer, state.buffer, "C modified buffer [{ctx}]");
    assert_eq!(rs.pos, state.pos, "Rust modified pos [{ctx}]");
    assert_eq!(rs.len, state.len, "Rust modified len [{ctx}]");
    assert_eq!(rs.buffer, state.buffer, "Rust modified buffer [{ctx}]");
}

/// Drive a *sequence* of calls on the same writer through both `.so`s,
/// comparing state after every single call (pipeline / real-consumer pattern).
#[track_caller]
pub fn check_sequence(start: Bitwriter, ops: &[(u32, u64)], ctx: &str) {
    let c = c_add();
    let r = rust_add();

    let mut cs = start;
    let mut rs = start;

    for (i, &(bits, val)) in ops.iter().enumerate() {
        let before_c = cs;
        let rc_c = unsafe { c(&mut cs as *mut Bitwriter, bits, val) };
        let rc_r = unsafe { r(&mut rs as *mut Bitwriter, bits, val) };
        assert_eq!(rc_c, rc_r, "return differs [{ctx}] step {i}");
        assert_eq!(
            cs.as_bytes(),
            rs.as_bytes(),
            "struct differs [{ctx}] step {i} before={before_c:?} bits={bits} val={val:#018x}\n  C   ={cs:?}\n  Rust={rs:?}"
        );
    }
}

/// Deterministic xorshift64* PRNG — fixed seed, reproducible across runs.
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
    /// Uniform in `0..=hi`.
    pub fn below(&mut self, hi_inclusive: u32) -> u32 {
        if hi_inclusive == u32::MAX {
            self.next_u32()
        } else {
            self.next_u32() % (hi_inclusive + 1)
        }
    }
    /// A `u64` biased toward interesting bit patterns.
    pub fn interesting_u64(&mut self) -> u64 {
        const PATTERNS: [u64; 10] = [
            0,
            1,
            u64::MAX,
            1 << 63,
            0xAAAA_AAAA_AAAA_AAAA,
            0x5555_5555_5555_5555,
            0xFFFF_FFFF_0000_0000,
            0x0000_0000_FFFF_FFFF,
            0x8000_0000_0000_0001,
            0x7FFF_FFFF_FFFF_FFFE,
        ];
        let r = self.next_u64();
        match r % 4 {
            0 => PATTERNS[(r >> 8) as usize % PATTERNS.len()],
            _ => self.next_u64(),
        }
    }
    /// A `bits` argument biased toward boundary values.
    pub fn interesting_bits(&mut self) -> u32 {
        const BOUNDARIES: [u32; 12] =
            [0, 1, 2, 31, 32, 62, 63, 64, 65, 100, 0xFFFF, u32::MAX];
        let r = self.next_u64();
        match r % 3 {
            0 => BOUNDARIES[(r >> 8) as usize % BOUNDARIES.len()],
            1 => (r >> 16) as u32 % 65,
            _ => self.next_u32(),
        }
    }
    /// A `bw->bits` incoming state biased toward boundary values.
    pub fn interesting_state_bits(&mut self) -> u32 {
        const BOUNDARIES: [u32; 11] =
            [0, 1, 31, 32, 62, 63, 64, 65, 128, 1000, u32::MAX];
        let r = self.next_u64();
        match r % 3 {
            0 => BOUNDARIES[(r >> 8) as usize % BOUNDARIES.len()],
            1 => (r >> 16) as u32 % 65,
            _ => self.next_u32(),
        }
    }
    pub fn state(&mut self) -> Bitwriter {
        Bitwriter {
            val: self.interesting_u64(),
            bits: self.interesting_state_bits(),
            pos: self.next_u32(),
            len: self.next_u32(),
            tot: self.next_u32(),
            buffer: self.next_u64() as *mut u8,
        }
    }
}
