//! Differential test: loads BOTH the C shared library and the Rust cdylib via
//! `libloading` and compares `bitwriter_add` results byte-for-byte.
//!
//! Nothing is called directly on the Rust crate; every call crosses the FFI
//! boundary through the exported `bitwriter_add` symbol, so the `#[no_mangle]`
//! wrapper is exercised too.

use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};

/// Mirrors `struct tflac_bitwriter` from c_src/include/lib.h.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq)]
struct Bw {
    val: u64,
    bits: u32,
    pos: u32,
    len: u32,
    tot: u32,
    buffer: *mut u8,
}

impl std::fmt::Debug for Bw {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Bw")
            .field("val", &format_args!("0x{:016x}", self.val))
            .field("bits", &self.bits)
            .field("pos", &self.pos)
            .field("len", &self.len)
            .field("tot", &self.tot)
            .field("buffer", &self.buffer)
            .finish()
    }
}

impl Bw {
    /// Raw byte view, so the comparison is genuinely byte-for-byte
    /// (padding included).
    fn bytes(&self) -> [u8; std::mem::size_of::<Bw>()] {
        unsafe { std::mem::transmute_copy(self) }
    }
}

type BitwriterAdd = unsafe extern "C" fn(*mut Bw, u32, u64) -> std::ffi::c_int;

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ has a parent")
        .to_path_buf()
}

fn find_c_lib() -> PathBuf {
    // Allows pointing the comparison at an alternately-compiled C library
    // (e.g. a different optimization level) without touching c_src/.
    if let Some(p) = std::env::var_os("C_LIB_PATH") {
        return PathBuf::from(p);
    }
    let build = workspace_root().join("c_src/build");
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}. Build the C library first.", build.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with("lib") && n.ends_with(".so"))
                .unwrap_or(false)
        })
        .collect();
    candidates.sort();
    candidates
        .pop()
        .unwrap_or_else(|| panic!("no lib*.so found in {}", build.display()))
}

fn find_rust_lib() -> PathBuf {
    const NAME: &str = "libbitwriter_add_lib.so";
    // The integration test binary lives in target/<profile>/deps/, so the
    // cdylib is normally one directory up (or right next to it).
    let exe = std::env::current_exe().expect("current_exe");
    let deps_dir = exe.parent().expect("target/<profile>/deps");
    let profile_dir = deps_dir.parent().expect("target/<profile>");

    let mut candidates = vec![profile_dir.join(NAME), deps_dir.join(NAME)];
    // Fall back to the sibling profile directories: `cargo test --release`
    // does not always emit the cdylib before the test binary runs.
    if let Some(target_dir) = profile_dir.parent() {
        for profile in ["release", "debug"] {
            candidates.push(target_dir.join(profile).join(NAME));
            candidates.push(target_dir.join(profile).join("deps").join(NAME));
        }
    }
    for c in &candidates {
        if c.exists() {
            return c.clone();
        }
    }
    panic!(
        "{NAME} not found (looked in {}). Run `cargo build` (and `cargo build --release`) first.",
        profile_dir.display()
    );
}

struct Pair {
    _c_lib: Library,
    _rust_lib: Library,
    c: BitwriterAdd,
    rust: BitwriterAdd,
}

impl Pair {
    fn load() -> Pair {
        unsafe {
            let c_lib = Library::new(find_c_lib()).expect("load C .so");
            let rust_lib = Library::new(find_rust_lib()).expect("load Rust .so");
            let c: Symbol<BitwriterAdd> =
                c_lib.get(b"bitwriter_add\0").expect("C bitwriter_add");
            let rust: Symbol<BitwriterAdd> =
                rust_lib.get(b"bitwriter_add\0").expect("Rust bitwriter_add");
            let c = *c;
            let rust = *rust;
            Pair {
                _c_lib: c_lib,
                _rust_lib: rust_lib,
                c,
                rust,
            }
        }
    }

    /// Runs one case against both libraries and asserts full byte parity of
    /// the returned int and of the mutated struct.
    fn check(&self, init: Bw, bits: u32, val: u64) {
        let mut c_bw = init;
        let mut r_bw = init;
        let c_ret = unsafe { (self.c)(&mut c_bw, bits, val) };
        let r_ret = unsafe { (self.rust)(&mut r_bw, bits, val) };

        assert_eq!(
            c_ret, r_ret,
            "return value mismatch: init={init:?} bits={bits} val=0x{val:016x}"
        );
        assert_eq!(
            c_bw.bytes(),
            r_bw.bytes(),
            "struct mismatch: init={init:?} bits={bits} val=0x{val:016x}\n  C   = {c_bw:?}\n  RUST= {r_bw:?}"
        );
    }
}

fn base(val: u64, bits: u32) -> Bw {
    Bw {
        val,
        bits,
        pos: 0x11223344,
        len: 0x55667788,
        tot: 0,
        buffer: std::ptr::null_mut(),
    }
}

const INTERESTING_VALS: &[u64] = &[
    0,
    1,
    2,
    0xff,
    0x100,
    0x7fff_ffff,
    0x8000_0000,
    0xffff_ffff,
    0x1_0000_0000,
    0x5555_5555_5555_5555,
    0xaaaa_aaaa_aaaa_aaaa,
    0x8000_0000_0000_0000,
    0x7fff_ffff_ffff_ffff,
    0xffff_ffff_ffff_ffff,
    0xdead_beef_cafe_babe,
    0x0123_4567_89ab_cdef,
];

/// Deterministic xorshift64* so cases are reproducible.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
}

#[test]
fn symbol_is_exported_by_both() {
    // Loading alone proves both .so files export `bitwriter_add`.
    let _ = Pair::load();
}

/// Ordinary, in-range usage: bits 0..=64, fresh writer.
#[test]
fn fresh_writer_all_bit_widths() {
    let p = Pair::load();
    for bits in 0u32..=64 {
        for &val in INTERESTING_VALS {
            p.check(base(0, 0), bits, val);
        }
    }
}

/// Sweep every starting `bw->bits` against every `bits` width, which covers
/// the loop-entry boundary (`bw->bits + bits >= 64`) from both sides.
#[test]
fn full_bits_cross_product() {
    let p = Pair::load();
    for start_bits in 0u32..=70 {
        for bits in 0u32..=70 {
            for &val in INTERESTING_VALS {
                p.check(base(0xdead_beef_0000_0000, start_bits), bits, val);
                p.check(base(0, start_bits), bits, val);
            }
        }
    }
}

/// `bits` values far above the operand width, where C performs
/// out-of-range shifts and the `64 - bits` subtraction underflows.
#[test]
fn oversized_bit_counts() {
    let p = Pair::load();
    let widths: Vec<u32> = (64u32..=200)
        .chain([255, 256, 257, 511, 512, 1000, 4095, 4096])
        .chain([
            u32::MAX,
            u32::MAX - 1,
            u32::MAX - 63,
            0x8000_0000,
            0x8000_0001,
            0xffff_ffc0,
            0xffff_ffc1,
        ])
        .collect();
    for &bits in &widths {
        for start_bits in [0u32, 1, 31, 32, 63, 64, 65, 100, 0xffff_ffff] {
            for &val in INTERESTING_VALS {
                p.check(base(0x0f0f_0f0f_0f0f_0f0f, start_bits), bits, val);
            }
        }
    }
}

/// `bw->bits` values that make `64 - bw->bits - 1` underflow, and states that
/// drive the `i < 100` loop guard.
#[test]
fn writer_bits_underflow_and_loop_guard() {
    let p = Pair::load();
    for start_bits in [
        0u32,
        63,
        64,
        65,
        66,
        127,
        128,
        1000,
        0x7fff_ffff,
        0x8000_0000,
        0xffff_fffe,
        0xffff_ffff,
    ] {
        for bits in [0u32, 1, 8, 32, 63, 64, 65, 128, 0xffff_ffff] {
            for &val in INTERESTING_VALS {
                p.check(base(0xffff_ffff_ffff_ffff, start_bits), bits, val);
                p.check(base(0x1, start_bits), bits, val);
            }
        }
    }
}

/// `tot` near overflow, to confirm the wrapping `bw->tot += bits`.
#[test]
fn tot_overflow() {
    let p = Pair::load();
    for tot in [0u32, 1, 0x7fff_ffff, 0xffff_fff0, 0xffff_ffff] {
        for bits in [0u32, 1, 16, 64, 65, 0xffff_ffff] {
            let mut init = base(0, 0);
            init.tot = tot;
            p.check(init, bits, 0xdead_beef_cafe_babe);
        }
    }
}

/// Non-zero `pos`/`len`/`buffer` must be passed through untouched.
#[test]
fn untouched_fields_preserved() {
    let p = Pair::load();
    let mut scratch = [0u8; 8];
    for bits in [0u32, 1, 33, 64, 65, 200] {
        let init = Bw {
            val: 0xa5a5_a5a5_a5a5_a5a5,
            bits: 5,
            pos: 0xdead_beef,
            len: 0xfeed_face,
            tot: 12345,
            buffer: scratch.as_mut_ptr(),
        };
        p.check(init, bits, 0x1234_5678_9abc_def0);
    }
}

/// Repeated calls on the same writer: the running state must stay in lockstep.
#[test]
fn sequential_calls_stay_in_lockstep() {
    let p = Pair::load();
    let mut rng = Rng(0x1234_5678_9abc_def0);

    for round in 0..200 {
        let mut c_bw = base(0, 0);
        let mut r_bw = c_bw;
        c_bw.tot = 0;
        r_bw.tot = 0;
        for step in 0..40 {
            let val = rng.next();
            // Mix of realistic widths and pathological ones.
            let bits = match (round + step) % 5 {
                0 => (rng.next() % 65) as u32,
                1 => (rng.next() % 33) as u32,
                2 => (rng.next() % 129) as u32,
                3 => rng.next() as u32,
                _ => (rng.next() % 9) as u32,
            };
            let c_ret = unsafe { (p.c)(&mut c_bw, bits, val) };
            let r_ret = unsafe { (p.rust)(&mut r_bw, bits, val) };
            assert_eq!(c_ret, r_ret, "round {round} step {step}");
            assert_eq!(
                c_bw.bytes(),
                r_bw.bytes(),
                "round {round} step {step} bits={bits} val=0x{val:016x}\n  C   = {c_bw:?}\n  RUST= {r_bw:?}"
            );
        }
    }
}

/// Broad randomized fuzz over the whole input space, including the struct's
/// starting state.
#[test]
fn randomized_fuzz() {
    let p = Pair::load();
    let mut rng = Rng(0xdead_beef_cafe_babe);
    for _ in 0..200_000 {
        let val = rng.next();
        let start_val = rng.next();
        let bits = match rng.next() % 4 {
            0 => (rng.next() % 65) as u32,
            1 => (rng.next() % 256) as u32,
            2 => rng.next() as u32,
            _ => (rng.next() % 5) as u32,
        };
        let start_bits = match rng.next() % 4 {
            0 => (rng.next() % 65) as u32,
            1 => (rng.next() % 256) as u32,
            2 => rng.next() as u32,
            _ => 0,
        };
        let init = Bw {
            val: start_val,
            bits: start_bits,
            pos: rng.next() as u32,
            len: rng.next() as u32,
            tot: rng.next() as u32,
            buffer: std::ptr::null_mut(),
        };
        p.check(init, bits, val);
    }
}
