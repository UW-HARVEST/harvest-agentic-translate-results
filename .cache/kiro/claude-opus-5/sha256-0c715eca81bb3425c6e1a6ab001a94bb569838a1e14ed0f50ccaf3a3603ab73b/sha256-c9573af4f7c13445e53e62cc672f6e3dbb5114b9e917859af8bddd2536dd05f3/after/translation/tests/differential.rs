//! Differential tests: load BOTH the C `.so` and the Rust `.so` with
//! `libloading` and compare their behaviour through the FFI boundary.
//!
//! No Rust function is ever called directly — every call goes through
//! `dlsym("next_double")` on the built `cdylib`, exactly as an external C
//! consumer would, so the `#[no_mangle] extern "C"` wrapper is under test too.
//!
//! Observable output of `next_double` is TWO things, and both are compared
//! byte-for-byte on every single call:
//!   1. the returned `double`, compared by its 64 raw bits (`to_bits`), so
//!      `+0.0`/`-0.0` and any NaN payload would be caught;
//!   2. the 16 bytes of the caller's `cn_rnd_t`, which the function mutates
//!      (it is an in/out parameter).

use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Harness
// ---------------------------------------------------------------------------

/// Mirrors `typedef struct cn_rnd_t { uint64_t state[2]; } cn_rnd_t;`
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct CnRnd {
    pub state: [u64; 2],
}

impl CnRnd {
    fn new(s0: u64, s1: u64) -> Self {
        CnRnd { state: [s0, s1] }
    }
}

/// `double next_double(cn_rnd_t *rnd)`
type NextDouble = unsafe extern "C" fn(*mut CnRnd) -> f64;

pub struct Lib {
    /// Kept alive so the function pointer below stays valid.
    _lib: libloading::Library,
    pub name: &'static str,
    pub next_double: NextDouble,
}

impl Lib {
    fn open(path: &Path, name: &'static str) -> Lib {
        let lib = unsafe { libloading::Library::new(path) }
            .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()));
        let f = unsafe {
            let sym: libloading::Symbol<NextDouble> = lib
                .get(b"next_double\0")
                .unwrap_or_else(|e| panic!("dlsym(next_double) in {}: {e}", path.display()));
            *sym
        };
        Lib {
            _lib: lib,
            name,
            next_double: f,
        }
    }

    /// One call. Returns `(result bits, post-call state)`.
    fn call(&self, st: &mut CnRnd) -> u64 {
        (unsafe { (self.next_double)(st as *mut CnRnd) }).to_bits()
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The C shared object produced by `c_src/build`.
fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("DIFF_C_SO") {
        return PathBuf::from(p);
    }
    let build = manifest_dir().join("../c_src/build");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| {
            panic!(
                "cannot read {} ({e}); build the C library first:\n  cd c_src && mkdir -p build \
                 && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
                build.display()
            )
        })
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    found.sort();
    assert_eq!(
        found.len(),
        1,
        "expected exactly one .so in {}, found {found:?}",
        build.display()
    );
    found.pop().unwrap()
}

/// The Rust `cdylib`. Release preferred (that is what `run_all.sh` builds).
/// `DIFF_RUST_SO` overrides, so the same suite can be pointed at the debug
/// `cdylib` as well.
fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("DIFF_RUST_SO") {
        return PathBuf::from(p);
    }
    let base = std::env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| manifest_dir().join("target"));
    for profile in ["release", "debug"] {
        let p = base.join(profile).join("libnext_double_lib.so");
        if p.is_file() {
            return p;
        }
    }
    panic!(
        "libnext_double_lib.so not found under {}; run `cargo build --release` first",
        base.display()
    );
}

fn libs() -> (Lib, Lib) {
    (
        Lib::open(&c_so_path(), "C"),
        Lib::open(&rust_so_path(), "Rust"),
    )
}

/// Deterministic PRNG for the property-style rows. Fixed seed => reproducible.
struct SplitMix64(u64);

const SEED: u64 = 0x243F_6A88_85A3_08D3;

impl SplitMix64 {
    fn new() -> Self {
        SplitMix64(SEED)
    }
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

/// Run `n` successive calls on both libraries starting from the same state and
/// assert every returned bit pattern and every intermediate state matches.
#[track_caller]
fn assert_seq(c: &Lib, r: &Lib, s0: u64, s1: u64, n: usize, row: &str) {
    let mut sc = CnRnd::new(s0, s1);
    let mut sr = CnRnd::new(s0, s1);
    for i in 0..n {
        let bc = c.call(&mut sc);
        let br = r.call(&mut sr);
        assert_eq!(
            bc, br,
            "[{row}] call #{i} from state ({s0:#018x}, {s1:#018x}): \
             return bits differ: C={bc:#018x} ({}) vs Rust={br:#018x} ({})",
            f64::from_bits(bc),
            f64::from_bits(br)
        );
        assert_eq!(
            sc.state, sr.state,
            "[{row}] call #{i} from state ({s0:#018x}, {s1:#018x}): \
             post-call state differs: C={:#x?} vs Rust={:#x?}",
            sc.state, sr.state
        );
        // Row 28 invariant, checked on every call of every row.
        let v = f64::from_bits(bc);
        assert!(
            v >= 0.0 && v < 1.0 && !v.is_nan() && bc >> 63 == 0,
            "[{row}] call #{i}: result {v} (bits {bc:#018x}) outside [0.0, 1.0)"
        );
    }
}

#[track_caller]
fn assert_one(c: &Lib, r: &Lib, s0: u64, s1: u64, row: &str) {
    assert_seq(c, r, s0, s1, 1, row);
}

/// The C round function, replicated here ONLY to construct states that hit a
/// desired internal `value` (rows 11-16). Never used as an oracle: every
/// assertion still compares the two `.so`s against each other, and the
/// construction is independently validated against the C library's own output.
fn inv_rshift(y: u64, s: u32) -> u64 {
    let mut x = 0u64;
    let mut sh = 0u32;
    while sh < 64 {
        x ^= y >> sh;
        sh += s;
    }
    x
}

fn inv_lshift(y: u64, s: u32) -> u64 {
    let mut x = 0u64;
    let mut sh = 0u32;
    while sh < 64 {
        x ^= y << sh;
        sh += s;
    }
    x
}

/// Find `state[0]` such that, with the given `state[1] = y`, the first call's
/// internal `value` (`x_out + y`) equals `want_value`.
fn state0_for_value(y: u64, want_value: u64) -> u64 {
    let x3 = want_value.wrapping_sub(y); // undo `return x + y`
    let x2 = x3 ^ y ^ (y >> 26); // undo `x ^= y ^ (y >> 26)`
    let x1 = inv_rshift(x2, 17); // undo `x ^= x >> 17`
    inv_lshift(x1, 23) // undo `x ^= x << 23`
}

/// The `double` the C code must produce for a given internal `value`.
fn expected_bits_for_value(value: u64) -> u64 {
    let mantissa = value >> 12;
    (f64::from_bits((1023u64 << 52) | mantissa) - 1.0).to_bits()
}

// ---------------------------------------------------------------------------
// Phase B — valid-path differential tests, one per CONFIGS.md row
// ---------------------------------------------------------------------------

mod configs {
    use super::*;

    #[test]
    fn row01_all_zero_state_single_call() {
        let (c, r) = libs();
        assert_one(&c, &r, 0, 0, "row 1");
    }

    #[test]
    fn row02_all_zero_state_long_sequence() {
        let (c, r) = libs();
        assert_seq(&c, &r, 0, 0, 1000, "row 2");
    }

    #[test]
    fn row03_x_one_y_zero() {
        let (c, r) = libs();
        assert_seq(&c, &r, 1, 0, 64, "row 3");
    }

    #[test]
    fn row04_x_zero_y_one() {
        let (c, r) = libs();
        assert_seq(&c, &r, 0, 1, 64, "row 4");
    }

    #[test]
    fn row05_both_words_max() {
        let (c, r) = libs();
        assert_seq(&c, &r, u64::MAX, u64::MAX, 64, "row 5");
    }

    #[test]
    fn row06_x_max_y_zero() {
        let (c, r) = libs();
        assert_seq(&c, &r, u64::MAX, 0, 64, "row 6");
    }

    #[test]
    fn row07_x_zero_y_max() {
        let (c, r) = libs();
        assert_seq(&c, &r, 0, u64::MAX, 64, "row 7");
    }

    #[test]
    fn row08_sign_bit_only() {
        let (c, r) = libs();
        assert_seq(&c, &r, 1u64 << 63, 1u64 << 63, 64, "row 8");
    }

    /// S1 boundary: `x << 23` shifts every set bit out, so the first xor step
    /// is `x ^= 0`.
    #[test]
    fn row09_left_shift_loses_all_bits() {
        let (c, r) = libs();
        for k in 41..64u32 {
            assert_seq(&c, &r, 1u64 << k, 0, 4, "row 9");
        }
        // and a value whose bits are all >= 41
        assert_seq(&c, &r, 0xFFFF_FE00_0000_0000, 0, 4, "row 9");
    }

    /// S2 and S3 degenerate simultaneously: `x >> 17 == 0` and `y >> 26 == 0`.
    #[test]
    fn row10_both_right_shifts_vanish() {
        let (c, r) = libs();
        let mut rng = SplitMix64::new();
        for _ in 0..2000 {
            let x = rng.next_u64() & ((1u64 << 17) - 1);
            let y = rng.next_u64() & ((1u64 << 26) - 1);
            assert_one(&c, &r, x, y, "row 10");
        }
        assert_one(&c, &r, (1 << 17) - 1, (1 << 26) - 1, "row 10");
    }

    /// S4 = wrap: `x_out + y` overflows 64 bits.
    #[test]
    fn row11_sum_wraps() {
        let (c, r) = libs();
        let y = 1u64 << 63;
        let mut wrapped = 0usize;
        for want in [0u64, 1, 0xFFF, 0x1000, 0x8000_0000_0000_0000, u64::MAX] {
            let x = state0_for_value(y, want);
            // validate the construction against the C library itself
            let mut st = CnRnd::new(x, y);
            let got = c.call(&mut st);
            assert_eq!(
                got,
                expected_bits_for_value(want),
                "row 11: construction for value {want:#x} did not land"
            );
            // `st.state[1]` is `x_out`; the internal sum is `x_out + y`.
            if st.state[1].checked_add(y).is_none() {
                wrapped += 1;
            }
            assert_one(&c, &r, x, y, "row 11");
        }
        assert!(
            wrapped >= 4,
            "row 11: expected several wrapping sums, saw {wrapped}"
        );
    }

    /// S4 = no wrap.
    #[test]
    fn row12_sum_does_not_wrap() {
        let (c, r) = libs();
        let y = 1u64;
        let want = u64::MAX;
        let x = state0_for_value(y, want);
        let mut st = CnRnd::new(x, y);
        let got = c.call(&mut st);
        assert_eq!(got, expected_bits_for_value(want), "row 12: construction");
        assert!(
            st.state[1].checked_add(y).is_some(),
            "row 12: expected no wrap"
        );
        assert_one(&c, &r, x, y, "row 12");
    }

    /// S5: mantissa 0 => the result must be exactly `+0.0`.
    #[test]
    fn row13_mantissa_zero_gives_plus_zero() {
        let (c, r) = libs();
        for y in [0u64, 1, 12345, u64::MAX] {
            let x = state0_for_value(y, 0);
            let mut st = CnRnd::new(x, y);
            let bits = c.call(&mut st);
            assert_eq!(bits, 0u64, "row 13: C must return +0.0 for value == 0");
            assert_one(&c, &r, x, y, "row 13");
        }
    }

    /// S5 low-12-bit boundary: `value == 0xFFF` still has mantissa 0.
    #[test]
    fn row14_value_0xfff_still_plus_zero() {
        let (c, r) = libs();
        for y in [0u64, 7, u64::MAX] {
            let x = state0_for_value(y, 0xFFF);
            let mut st = CnRnd::new(x, y);
            assert_eq!(c.call(&mut st), 0u64, "row 14: expected +0.0");
            assert_one(&c, &r, x, y, "row 14");
        }
        // one step past the boundary: value == 0x1000 -> mantissa 1, non-zero
        let y = 0u64;
        let x = state0_for_value(y, 0x1000);
        let mut st = CnRnd::new(x, y);
        assert_ne!(c.call(&mut st), 0u64, "row 14: value 0x1000 must not be 0.0");
        assert_one(&c, &r, x, y, "row 14");
    }

    /// S5 upper end: mantissa all ones => largest producible value, still < 1.0.
    #[test]
    fn row15_mantissa_all_ones() {
        let (c, r) = libs();
        for y in [0u64, 0xDEAD_BEEF, u64::MAX] {
            for want in [0xFFFF_FFFF_FFFF_F000u64, u64::MAX] {
                let x = state0_for_value(y, want);
                let mut st = CnRnd::new(x, y);
                let bits = c.call(&mut st);
                assert_eq!(
                    bits,
                    expected_bits_for_value(want),
                    "row 15: construction for {want:#x}"
                );
                let v = f64::from_bits(bits);
                assert!(v < 1.0 && v > 0.999_999_999, "row 15: got {v}");
                assert_one(&c, &r, x, y, "row 15");
            }
        }
    }

    /// S6: the low 12 bits of `value` are discarded, so states differing only
    /// there must give the same `double` but different write-back state.
    #[test]
    fn row16_low_12_bits_discarded() {
        let (c, r) = libs();
        let mut rng = SplitMix64::new();
        for _ in 0..500 {
            let y = rng.next_u64();
            let base = rng.next_u64() & !0xFFFu64;
            let lo = rng.next_u64() & 0xFFF;
            let xa = state0_for_value(y, base);
            let xb = state0_for_value(y, base | lo);

            let mut sa = CnRnd::new(xa, y);
            let mut sb = CnRnd::new(xb, y);
            let ba = c.call(&mut sa);
            let bb = c.call(&mut sb);
            assert_eq!(ba, bb, "row 16: low 12 bits must not affect the result");

            assert_one(&c, &r, xa, y, "row 16");
            assert_one(&c, &r, xb, y, "row 16");
        }
    }

    #[test]
    fn row17_randomized_uniform_states() {
        let (c, r) = libs();
        let mut rng = SplitMix64::new();
        for _ in 0..20_000 {
            let (x, y) = (rng.next_u64(), rng.next_u64());
            assert_one(&c, &r, x, y, "row 17");
        }
    }

    #[test]
    fn row18_randomized_long_sequences() {
        let (c, r) = libs();
        let mut rng = SplitMix64::new();
        for _ in 0..200 {
            let (x, y) = (rng.next_u64(), rng.next_u64());
            assert_seq(&c, &r, x, y, 1000, "row 18");
        }
    }

    #[test]
    fn row19_randomized_y_zero() {
        let (c, r) = libs();
        let mut rng = SplitMix64::new();
        for _ in 0..5000 {
            assert_one(&c, &r, rng.next_u64(), 0, "row 19");
        }
    }

    #[test]
    fn row20_randomized_x_zero() {
        let (c, r) = libs();
        let mut rng = SplitMix64::new();
        for _ in 0..5000 {
            assert_one(&c, &r, 0, rng.next_u64(), "row 20");
        }
    }

    #[test]
    fn row21_randomized_sparse_single_bit() {
        let (c, r) = libs();
        let mut rng = SplitMix64::new();
        for _ in 0..5000 {
            let x = 1u64 << (rng.next_u64() % 64);
            let y = 1u64 << (rng.next_u64() % 64);
            assert_seq(&c, &r, x, y, 3, "row 21");
        }
        // exhaustive single-bit cross product: 64 x 64
        for i in 0..64u32 {
            for j in 0..64u32 {
                assert_one(&c, &r, 1u64 << i, 1u64 << j, "row 21");
            }
        }
    }

    #[test]
    fn row22_shift_boundary_values() {
        let (c, r) = libs();
        let mut vals: Vec<u64> = vec![0, 1, 2, u64::MAX, u64::MAX - 1];
        for k in [1u32, 12, 16, 17, 18, 23, 25, 26, 27, 40, 41, 42, 52, 63] {
            vals.push(1u64 << k);
            vals.push((1u64 << k) - 1);
            vals.push(u64::MAX << k);
            vals.push(u64::MAX >> k);
        }
        for &x in &vals {
            for &y in &vals {
                assert_seq(&c, &r, x, y, 2, "row 22");
            }
        }
        let mut rng = SplitMix64::new();
        for _ in 0..5000 {
            let x = vals[(rng.next_u64() as usize) % vals.len()];
            let y = vals[(rng.next_u64() as usize) % vals.len()];
            assert_one(&c, &r, x, y, "row 22");
        }
    }

    #[test]
    fn row23_low_entropy_states() {
        let (c, r) = libs();
        let mut rng = SplitMix64::new();
        for _ in 0..5000 {
            let x = rng.next_u64() & 0xFFF;
            let y = rng.next_u64() & 0xFFF;
            assert_seq(&c, &r, x, y, 4, "row 23");
        }
    }

    /// S9: two live instances, interleaved. Catches any hidden global state.
    #[test]
    fn row24_two_interleaved_instances() {
        let (c, r) = libs();
        let mut rng = SplitMix64::new();
        let (a0, a1) = (rng.next_u64(), rng.next_u64());
        let (b0, b1) = (rng.next_u64(), rng.next_u64());

        let mut ca = CnRnd::new(a0, a1);
        let mut cb = CnRnd::new(b0, b1);
        let mut ra = CnRnd::new(a0, a1);
        let mut rb = CnRnd::new(b0, b1);

        for i in 0..2000 {
            let (bc, br) = (c.call(&mut ca), r.call(&mut ra));
            assert_eq!(bc, br, "row 24: instance A, call #{i}");
            assert_eq!(ca.state, ra.state, "row 24: instance A state, call #{i}");

            let (bc, br) = (c.call(&mut cb), r.call(&mut rb));
            assert_eq!(bc, br, "row 24: instance B, call #{i}");
            assert_eq!(cb.state, rb.state, "row 24: instance B state, call #{i}");
        }

        // Independent progress: also compare against a fresh single-stream run.
        assert_seq(&c, &r, a0, a1, 2000, "row 24/A-alone");
        assert_seq(&c, &r, b0, b1, 2000, "row 24/B-alone");
    }

    /// S10: stack vs heap storage of the struct.
    #[test]
    fn row25_heap_and_stack_storage() {
        let (c, r) = libs();
        let mut rng = SplitMix64::new();
        for _ in 0..1000 {
            let (x, y) = (rng.next_u64(), rng.next_u64());

            let mut stack_c = CnRnd::new(x, y);
            let mut heap_c = Box::new(CnRnd::new(x, y));
            let mut stack_r = CnRnd::new(x, y);
            let mut heap_r = Box::new(CnRnd::new(x, y));

            let bsc = c.call(&mut stack_c);
            let bhc = unsafe { (c.next_double)(&mut *heap_c as *mut CnRnd) }.to_bits();
            let bsr = r.call(&mut stack_r);
            let bhr = unsafe { (r.next_double)(&mut *heap_r as *mut CnRnd) }.to_bits();

            assert_eq!(bsc, bhc, "row 25: C stack vs heap");
            assert_eq!(bsr, bhr, "row 25: Rust stack vs heap");
            assert_eq!(bsc, bsr, "row 25: C vs Rust (stack)");
            assert_eq!(bhc, bhr, "row 25: C vs Rust (heap)");
            assert_eq!(stack_c.state, stack_r.state, "row 25: stack state");
            assert_eq!(heap_c.state, heap_r.state, "row 25: heap state");
        }
    }

    /// S7/row 26: hand both libraries the identical raw 16-byte buffer and
    /// require the identical byte image out. Also pins `cn_rnd_t` field order.
    #[test]
    fn row26_raw_byte_image_parity() {
        let (c, r) = libs();
        assert_eq!(std::mem::size_of::<CnRnd>(), 16, "cn_rnd_t must be 16 bytes");
        assert_eq!(std::mem::align_of::<CnRnd>(), 8, "cn_rnd_t must be 8-aligned");

        let mut rng = SplitMix64::new();
        for _ in 0..2000 {
            let mut bytes = [0u8; 16];
            for b in bytes.iter_mut() {
                *b = (rng.next_u64() & 0xFF) as u8;
            }
            let mut bc = bytes;
            let mut br = bytes;
            let rc = unsafe { (c.next_double)(bc.as_mut_ptr() as *mut CnRnd) }.to_bits();
            let rr = unsafe { (r.next_double)(br.as_mut_ptr() as *mut CnRnd) }.to_bits();
            assert_eq!(rc, rr, "row 26: return bits for byte image {bytes:02x?}");
            assert_eq!(bc, br, "row 26: post-call byte image for {bytes:02x?}");
        }

        // field order: state[0] is the first 8 bytes (little-endian host)
        let s = CnRnd::new(0x0102_0304_0506_0708, 0x1112_1314_1516_1718);
        let raw: [u8; 16] = unsafe { std::mem::transmute(s) };
        assert_eq!(&raw[..8], &0x0102_0304_0506_0708u64.to_ne_bytes());
        assert_eq!(&raw[8..], &0x1112_1314_1516_1718u64.to_ne_bytes());
    }

    /// Row 27: each library is deterministic on its own (no clock/entropy).
    #[test]
    fn row27_determinism_within_each_library() {
        let (c, r) = libs();
        let mut rng = SplitMix64::new();
        for _ in 0..1000 {
            let (x, y) = (rng.next_u64(), rng.next_u64());
            for lib in [&c, &r] {
                let mut a = CnRnd::new(x, y);
                let mut b = CnRnd::new(x, y);
                let ba = lib.call(&mut a);
                let bb = lib.call(&mut b);
                assert_eq!(ba, bb, "row 27: {} not deterministic", lib.name);
                assert_eq!(a.state, b.state, "row 27: {} state differs", lib.name);
            }
        }
    }

    /// Row 28: the range invariant, asserted directly on a wide random sample
    /// (it is also asserted inside `assert_seq` for every other row).
    #[test]
    fn row28_range_invariant() {
        let (c, r) = libs();
        let mut rng = SplitMix64::new();
        let mut saw_zero = false;
        for _ in 0..20_000 {
            let (x, y) = (rng.next_u64(), rng.next_u64());
            let mut sc = CnRnd::new(x, y);
            let mut sr = CnRnd::new(x, y);
            let vc = f64::from_bits(c.call(&mut sc));
            let vr = f64::from_bits(r.call(&mut sr));
            assert_eq!(vc.to_bits(), vr.to_bits());
            assert!((0.0..1.0).contains(&vc), "C out of range: {vc}");
            assert!((0.0..1.0).contains(&vr), "Rust out of range: {vr}");
            assert!(!vc.is_nan() && !vr.is_nan());
            saw_zero |= vc == 0.0;
        }
        // the constructed rows already cover +0.0; this is informational
        let _ = saw_zero;
    }
}

// ---------------------------------------------------------------------------
// Phase C — error-path differential tests, one per ERRORS.md row
// ---------------------------------------------------------------------------

mod errors {
    use super::*;

    /// Name of the null-deref test, used to re-invoke this binary as a child.
    const NULL_TEST: &str = "errors::b1_null_pointer_same_fatal_signal";
    const CHILD_VAR: &str = "DIFF_NULL_CHILD";

    /// ERRORS.md main table: the C source contains no rejection at all, so
    /// nothing is ever refused. Structural check that both libraries accept
    /// every class of state input and return a normal value.
    #[test]
    fn e00_no_explicit_error_surface_in_c() {
        let (c, r) = libs();
        let mut rng = SplitMix64::new();
        let mut states: Vec<(u64, u64)> = vec![
            (0, 0),
            (0, u64::MAX),
            (u64::MAX, 0),
            (u64::MAX, u64::MAX),
            (1, 1),
            (1u64 << 63, 0),
        ];
        for _ in 0..5000 {
            states.push((rng.next_u64(), rng.next_u64()));
        }
        for (x, y) in states {
            let mut sc = CnRnd::new(x, y);
            let mut sr = CnRnd::new(x, y);
            let bc = c.call(&mut sc);
            let br = r.call(&mut sr);
            // "no rejection" == always a finite value in [0,1), never a
            // sentinel like NaN / -1.0 / HUGE_VAL, for both libraries.
            let vc = f64::from_bits(bc);
            assert!(vc.is_finite() && (0.0..1.0).contains(&vc), "C rejected? {vc}");
            assert_eq!(bc, br, "state ({x:#x}, {y:#x})");
            assert_eq!(sc.state, sr.state, "state ({x:#x}, {y:#x})");
        }
    }

    /// B1: `rnd == NULL`. The C code dereferences unconditionally, so both
    /// libraries must die the same way. Done in forked child processes (this
    /// test binary re-invoked with `CHILD_VAR` set) so the harness survives.
    #[test]
    fn b1_null_pointer_same_fatal_signal() {
        use std::os::unix::process::ExitStatusExt;
        use std::process::Command;

        // Child mode: perform the null call and die.
        if let Ok(which) = std::env::var(CHILD_VAR) {
            let lib = match which.as_str() {
                "c" => Lib::open(&c_so_path(), "C"),
                "rust" => Lib::open(&rust_so_path(), "Rust"),
                other => panic!("bad {CHILD_VAR}={other}"),
            };
            let v = unsafe { (lib.next_double)(std::ptr::null_mut()) };
            // If we get here the library did NOT fault: report that as exit 42
            // together with the value, so the parent can compare.
            eprintln!("{} returned {v} for NULL", lib.name);
            std::process::exit(42);
        }

        let run = |which: &str| -> (Option<i32>, Option<i32>) {
            let exe = std::env::current_exe().expect("current_exe");
            let out = Command::new(exe)
                .args(["--exact", NULL_TEST, "--test-threads=1", "--nocapture"])
                .env(CHILD_VAR, which)
                .output()
                .expect("spawn child");
            (out.status.code(), out.status.signal())
        };

        let (c_code, c_sig) = run("c");
        let (r_code, r_sig) = run("rust");

        assert_eq!(
            c_sig, r_sig,
            "NULL pointer: C died with signal {c_sig:?} but Rust with {r_sig:?}"
        );
        assert_eq!(
            c_code, r_code,
            "NULL pointer: C exited with {c_code:?} but Rust with {r_code:?}"
        );
        // Neither may return an error code instead of faulting: the C has no
        // error return at all (see ERRORS.md).
        assert_ne!(
            c_code,
            Some(42),
            "C unexpectedly returned normally for NULL; the differential \
             expectation in ERRORS.md needs revisiting"
        );
        assert_eq!(
            c_sig,
            Some(11),
            "expected SIGSEGV from the unguarded deref, got {c_sig:?}"
        );
    }

    /// B2: `next_double` has no length/size/count parameter, so there is no
    /// zero-length or oversized-length input to test. Asserted structurally:
    /// the symbol is callable with exactly one 16-byte struct pointer.
    #[test]
    fn b2_no_length_parameter_exists() {
        let (c, r) = libs();
        assert_eq!(
            std::mem::size_of::<NextDouble>(),
            std::mem::size_of::<*const ()>()
        );
        // A single argument, a single 16-byte object: nothing else to size.
        assert_eq!(std::mem::size_of::<CnRnd>(), 16);
        assert_one(&c, &r, 0xDEAD_BEEF_CAFE_F00D, 0x0BAD_C0DE_DEAD_10CC, "B2");
    }

    /// B3: the extremes of the valid range and their neighbours. Every
    /// `uint64_t` is valid, so "one past the range" does not exist; the
    /// wrap-around neighbours are exercised instead.
    #[test]
    fn b3_extremes_and_neighbours_are_valid() {
        let (c, r) = libs();
        let edge = [
            0u64,
            1,
            2,
            u64::MAX,
            u64::MAX - 1,
            u64::MAX - 2,
            1u64 << 63,
            (1u64 << 63) - 1,
            (1u64 << 63) + 1,
        ];
        for &x in &edge {
            for &y in &edge {
                assert_seq(&c, &r, x, y, 8, "B3");
            }
        }
    }

    /// B4: the public API declares no enum and takes no integer parameter, so
    /// there is no out-of-range enum discriminant that could cross the FFI
    /// boundary. Verified against the header text itself.
    #[test]
    fn b4_no_enum_in_public_api() {
        let hdr = std::fs::read_to_string(manifest_dir().join("../c_src/include/lib.h"))
            .expect("read lib.h");
        assert!(
            !hdr.contains("enum"),
            "lib.h gained an enum; ERRORS.md row B4 must be revisited:\n{hdr}"
        );
        let src = std::fs::read_to_string(manifest_dir().join("../c_src/src/lib.c"))
            .expect("read lib.c");
        assert!(!src.contains("enum"), "lib.c gained an enum:\n{src}");

        // The only parameter is a struct pointer; there is no int to abuse.
        let (c, r) = libs();
        assert_one(&c, &r, 42, 42, "B4");
    }

    /// B5: neither library may touch memory outside the 16-byte struct.
    #[test]
    fn b5_no_out_of_bounds_struct_access() {
        let (c, r) = libs();
        const GUARD: u64 = 0xA5A5_A5A5_A5A5_A5A5;
        let mut rng = SplitMix64::new();

        for _ in 0..1000 {
            let (x, y) = (rng.next_u64(), rng.next_u64());
            let mut run = |lib: &Lib| -> ([u64; 6], u64) {
                let mut buf: [u64; 6] = [GUARD, GUARD, x, y, GUARD, GUARD];
                let p = unsafe { buf.as_mut_ptr().add(2) } as *mut CnRnd;
                let bits = unsafe { (lib.next_double)(p) }.to_bits();
                (buf, bits)
            };
            let (bufc, bc) = run(&c);
            let (bufr, br) = run(&r);

            for i in [0usize, 1, 4, 5] {
                assert_eq!(bufc[i], GUARD, "C clobbered guard word {i}");
                assert_eq!(bufr[i], GUARD, "Rust clobbered guard word {i}");
            }
            assert_eq!(bc, br, "B5: return bits");
            assert_eq!(bufc, bufr, "B5: full buffer image");
        }
    }

    /// B6: heap vs stack address of the struct.
    #[test]
    fn b6_heap_and_stack_addresses_agree() {
        let (c, r) = libs();
        let mut rng = SplitMix64::new();
        for _ in 0..500 {
            let (x, y) = (rng.next_u64(), rng.next_u64());
            let mut heap_c = vec![CnRnd::new(x, y)];
            let mut heap_r = vec![CnRnd::new(x, y)];
            let mut st_c = CnRnd::new(x, y);
            let mut st_r = CnRnd::new(x, y);

            let hc = unsafe { (c.next_double)(heap_c.as_mut_ptr()) }.to_bits();
            let hr = unsafe { (r.next_double)(heap_r.as_mut_ptr()) }.to_bits();
            let scb = c.call(&mut st_c);
            let srb = r.call(&mut st_r);

            assert_eq!(hc, hr, "B6: heap C vs Rust");
            assert_eq!(scb, srb, "B6: stack C vs Rust");
            assert_eq!(hc, scb, "B6: address must not matter (C)");
            assert_eq!(hr, srb, "B6: address must not matter (Rust)");
            assert_eq!(heap_c[0].state, heap_r[0].state);
            assert_eq!(heap_c[0].state, st_c.state);
        }
    }

    /// B7: the all-zero state is a fixed point — `0.0` forever, state unchanged.
    #[test]
    fn b7_all_zero_state_is_a_fixed_point() {
        let (c, r) = libs();
        let mut sc = CnRnd::new(0, 0);
        let mut sr = CnRnd::new(0, 0);
        for i in 0..10_000 {
            let bc = c.call(&mut sc);
            let br = r.call(&mut sr);
            assert_eq!(bc, 0u64, "B7: C call #{i} must be +0.0, got {bc:#x}");
            assert_eq!(br, 0u64, "B7: Rust call #{i} must be +0.0, got {br:#x}");
            assert_eq!(sc.state, [0, 0], "B7: C state moved");
            assert_eq!(sr.state, [0, 0], "B7: Rust state moved");
        }
    }
}
