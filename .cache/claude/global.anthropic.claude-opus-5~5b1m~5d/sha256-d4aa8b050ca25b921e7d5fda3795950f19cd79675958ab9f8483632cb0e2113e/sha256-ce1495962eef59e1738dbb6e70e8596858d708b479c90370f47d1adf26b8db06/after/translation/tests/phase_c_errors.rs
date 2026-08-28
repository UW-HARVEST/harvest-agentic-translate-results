//! Phase C — error / boundary-path differential tests.
//!
//! One test per row of `ERRORS.md`. The C library has **no** error returns (a
//! single unconditional `return crc16;`, zero asserts, zero null checks, zero
//! range checks — see `ERRORS.md` for the mechanical grep), so these rows are the
//! degenerate, boundary and contract-edge conditions, plus the generic C-API
//! boundaries Phase C mandates. Each asserts the two `.so`s produce the *same
//! value*, not merely that both "failed somehow".

mod common;

use common::{assert_same, assert_same_raw, pair, Rng, SEED_EXTREMES};

/// Row 1 — `len == 0` with a valid pointer: `d` is never dereferenced and the
/// seed passes through untouched. Exhaustive over all 65536 seeds.
#[test]
fn err_len_zero_valid_ptr_returns_seed_unchanged() {
    let p = pair();
    let buf = [0xDEu8; 16];
    for seed in 0..=u16::MAX {
        let c = unsafe { p.c.crc16_raw(buf.as_ptr(), 0, seed) };
        let r = unsafe { p.rust.crc16_raw(buf.as_ptr(), 0, seed) };
        assert_eq!(c, r, "len=0 seed=0x{seed:04x}: C=0x{c:04x} Rust=0x{r:04x}");
        assert_eq!(c, seed, "C must return the seed unchanged when len==0");
        assert_eq!(r, seed, "Rust must return the seed unchanged when len==0");
    }
}

/// Row 2 — `len == 0` with a **null** pointer. The C code's `while (len >= 8)`
/// and `while (len--)` both fail before any dereference, so this is well defined
/// in practice and must not fault. The Rust wrapper must short-circuit *before*
/// `slice::from_raw_parts`, which is UB on null even for a zero length.
#[test]
fn err_len_zero_null_ptr_returns_seed_unchanged() {
    let p = pair();
    for seed in [0x0000u16, 0x0001, 0x00FF, 0x0100, 0x7FFF, 0x8000, 0xFFFE, 0xFFFF, 0x1234] {
        let c = unsafe { p.c.crc16_raw(std::ptr::null(), 0, seed) };
        let r = unsafe { p.rust.crc16_raw(std::ptr::null(), 0, seed) };
        assert_eq!(c, r, "null/len=0 seed=0x{seed:04x}: C=0x{c:04x} Rust=0x{r:04x}");
        assert_eq!(c, seed);
    }
    // and exhaustively over all seeds
    for seed in 0..=u16::MAX {
        let c = unsafe { p.c.crc16_raw(std::ptr::null(), 0, seed) };
        let r = unsafe { p.rust.crc16_raw(std::ptr::null(), 0, seed) };
        assert_eq!(c, r, "null/len=0 seed=0x{seed:04x}");
    }
}

/// Row 3 — `len == 0` with a **wild / unmapped, non-null** pointer. Proves the
/// zero-length short-circuit happens before any pointer use in both languages:
/// if either dereferenced `d`, this would segfault and the test binary would die.
#[test]
// The dangling/unmapped pointers are the entire point of this test.
#[allow(clippy::manual_dangling_ptr)]
fn err_len_zero_wild_ptr_no_deref() {
    let p = pair();
    let wild: [*const u8; 5] = [
        1usize as *const u8,
        0xDEAD_BEEFusize as *const u8,
        usize::MAX as *const u8,
        (usize::MAX - 7) as *const u8,
        0x1000usize as *const u8,
    ];
    for &ptr in &wild {
        for &seed in &SEED_EXTREMES {
            let c = unsafe { p.c.crc16_raw(ptr, 0, seed) };
            let r = unsafe { p.rust.crc16_raw(ptr, 0, seed) };
            assert_eq!(c, r, "wild ptr {ptr:?} len=0 seed=0x{seed:04x}");
            assert_eq!(c, seed, "wild ptr with len=0 must return the seed");
        }
    }
}

/// Row 4 — `len ∈ 1..=7`: strictly below the slice-by-8 threshold, so the wide
/// loop body never executes. One step below the block boundary.
#[test]
fn err_len_below_slice_by_8_threshold() {
    let mut rng = Rng::fixed(104);
    for len in 1usize..=7 {
        for &seed in &SEED_EXTREMES {
            for trial in 0..64 {
                let data = rng.bytes(len);
                assert_same(&data, seed, &format!("below-8 len={len} seed=0x{seed:04x} t={trial}"));
            }
        }
        // exhaustive over all seeds at a fixed buffer
        let data: Vec<u8> = (0..len).map(|i| (0xA5u8).wrapping_add(i as u8)).collect();
        for seed in 0..=u16::MAX {
            assert_same(&data, seed, &format!("below-8 exhaustive len={len}"));
        }
    }
}

/// Row 5 — `len == 8` exactly: the first value that enters the wide loop (one
/// step past the `len < 8` range). Loop runs once, `len -= 8` -> 0, tail runs 0x.
#[test]
fn err_len_exactly_eight_boundary() {
    let mut rng = Rng::fixed(105);
    let data: [u8; 8] = [0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88];
    for seed in 0..=u16::MAX {
        assert_same(&data, seed, "len=8 exhaustive seeds");
    }
    for trial in 0..2000 {
        let d = rng.bytes(8);
        let seed = rng.next_u16();
        assert_same(&d, seed, &format!("len=8 random t={trial}"));
    }
}

/// Row 6 — the 7 / 8 / 9 triple straddling the branch boundary, on the *same*
/// buffer and seed, so the three different code-path mixes are directly compared.
#[test]
fn err_len_seven_eight_nine_straddle() {
    let mut rng = Rng::fixed(106);
    for trial in 0..3000 {
        let buf = rng.bytes(9);
        let seed = rng.next_u16();
        for len in [7usize, 8, 9] {
            assert_same(&buf[..len], seed, &format!("straddle len={len} t={trial}"));
        }
    }
    for &seed in &SEED_EXTREMES {
        for pat in [0x00u8, 0xFF, 0x80, 0x7F] {
            let buf = [pat; 9];
            for len in [7usize, 8, 9] {
                assert_same(
                    &buf[..len],
                    seed,
                    &format!("straddle len={len} pat=0x{pat:02x} seed=0x{seed:04x}"),
                );
            }
        }
    }
}

/// Row 7 — the tail loop's `while (len--)` **unsigned wraparound**. When the
/// condition finally fails, C still post-decrements, wrapping `len` from 0 to
/// `UINT32_MAX`. `len` is dead afterwards, so this has no observable effect.
///
/// A Rust translation that mis-modelled the post-decrement as a loop bound would
/// try to iterate ~4 billion times and read far out of bounds. This test asserts
/// the correct value comes back, and that it comes back *promptly* (a wall-clock
/// bound that a 4-billion-iteration loop could not meet).
#[test]
fn err_tail_loop_len_postdecrement_wrap_is_dead() {
    let mut rng = Rng::fixed(107);
    let start = std::time::Instant::now();

    // Every length whose tail loop terminates by exhausting `len` (residues 1..7
    // as well as 0, at several wide counts).
    for len in 0usize..=64 {
        for trial in 0..32 {
            let data = rng.bytes(len);
            let seed = rng.next_u16();
            assert_same(&data, seed, &format!("wrap len={len} t={trial}"));
        }
    }

    let elapsed = start.elapsed();
    assert!(
        elapsed < std::time::Duration::from_secs(30),
        "tail-loop length handling looks wrong: {} calls took {elapsed:?}; a \
         UINT32_MAX wraparound loop would blow this bound",
        65 * 32 * 2
    );
}

/// Row 8 — seed extremes and byte boundaries. Verifies the `crc << 8` truncation
/// (C promotes to `int`, shifts, then truncates on assignment to `tflac_u16`) and
/// both seed-derived table indices, which can never leave `0..=255`.
#[test]
fn err_seed_extremes_and_byte_boundaries() {
    let boundary_seeds: Vec<u16> = {
        let mut v: Vec<u16> = SEED_EXTREMES.to_vec();
        // every seed whose high or low byte is at an extreme
        for hi in [0x00u16, 0x01, 0x7F, 0x80, 0xFE, 0xFF] {
            for lo in [0x00u16, 0x01, 0x7F, 0x80, 0xFE, 0xFF] {
                v.push((hi << 8) | lo);
            }
        }
        v.sort_unstable();
        v.dedup();
        v
    };

    let mut rng = Rng::fixed(108);
    for &seed in &boundary_seeds {
        for len in 0usize..=17 {
            for trial in 0..24 {
                let data = rng.bytes(len);
                assert_same(&data, seed, &format!("seed=0x{seed:04x} len={len} t={trial}"));
            }
            for pat in [0x00u8, 0xFF] {
                assert_same(
                    &vec![pat; len],
                    seed,
                    &format!("seed=0x{seed:04x} len={len} pat=0x{pat:02x}"),
                );
            }
        }
    }
}

/// Row 9 — every byte value `0x00..=0xFF` in **each** of the 8 wide-block lanes,
/// including the maximum table index 255 (one step past the last-but-one slot).
/// A mis-ordered or truncated table row is only visible in its own lane.
#[test]
fn err_all_byte_values_in_every_lane() {
    // Extreme surroundings, so a lane's contribution cannot be masked.
    for &fill in &[0x00u8, 0xFF] {
        for lane in 0..8usize {
            for v in 0..=255u8 {
                let mut d = [fill; 8];
                d[lane] = v;
                for &seed in &[0x0000u16, 0xFFFF, 0x00FF, 0xFF00, 0x8001] {
                    assert_same(
                        &d,
                        seed,
                        &format!("lane={lane} v=0x{v:02x} fill=0x{fill:02x} seed=0x{seed:04x}"),
                    );
                }
            }
        }
    }
    // The tail loop's table index is `(crc >> 8) ^ byte`; sweep it over the full
    // 0..=255 range including 255 itself, for a single-byte input.
    for idx in 0..=255u8 {
        for hi in 0..=255u8 {
            let byte = idx ^ hi;
            let seed = (hi as u16) << 8;
            assert_same(&[byte], seed, &format!("tail idx=0x{idx:02x} hi=0x{hi:02x}"));
        }
    }
}

/// Row 10 — "out-of-range enum value" analogue.
///
/// The API has **no enum parameter** (`grep -cE '\b(enum|switch|case)\b'` over
/// the C source == 0), so there is no invalid-variant class here. The nearest
/// equivalent is passing arbitrary unconstrained bit patterns through the only
/// non-pointer scalars — `len: u32` and `crc: u16` — including `len` values with
/// the sign bit set. Both languages must accept every bit pattern without UB.
#[test]
fn err_no_enum_params_unconstrained_scalars() {
    // Static proof of the premise, kept next to the test that relies on it.
    let c_src = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/src/lib.c"),
    )
    .expect("read c_src/src/lib.c");
    let c_hdr = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/include/lib.h"),
    )
    .expect("read c_src/include/lib.h");
    for (name, text) in [("lib.c", &c_src), ("lib.h", &c_hdr)] {
        for kw in ["enum", "switch", "case ", "assert", "errno"] {
            assert!(
                !text.contains(kw),
                "{name} unexpectedly contains `{kw}` — ERRORS.md row 10 must be revisited"
            );
        }
    }
    // Exactly one `return`, and it is unconditional.
    assert_eq!(c_src.matches("return").count(), 1, "C should have exactly one return");

    // Fuzz the scalars over their whole representable range (with `len` bounded
    // by the real buffer, since exceeding it is UB in the C original too).
    let mut rng = Rng::fixed(110);
    let mut buf = vec![0u8; 4096];
    rng.fill(&mut buf);
    for trial in 0..20_000 {
        let len = rng.below(4097) as u32;
        let seed = rng.next_u16(); // full u16 range, no valid/invalid distinction
        let ctx = format!("scalar-fuzz t={trial} len={len} seed=0x{seed:04x}");
        unsafe { assert_same_raw(buf.as_ptr(), len, seed, &ctx) };
    }
}

/// Row 11 — large *valid* lengths: proves no premature truncation of `len` hides
/// in the Rust wrapper (e.g. a `u32 -> u16` narrowing), across all 8 residues at
/// 64 KiB scale.
#[test]
fn err_large_valid_lengths_no_len_truncation() {
    let mut rng = Rng::fixed(111);
    let mut buf = vec![0u8; 64 * 1024 + 16];
    rng.fill(&mut buf);
    let n = buf.len();
    for len in (n - 16)..=n {
        for &seed in &[0x0000u16, 0xFFFF, 0x5A5A] {
            assert_same(&buf[..len], seed, &format!("large len={len} seed=0x{seed:04x}"));
        }
    }
    for &len in &[1024usize, 4096, 16384, 32768, 65536] {
        for &seed in &SEED_EXTREMES {
            assert_same(&buf[..len], seed, &format!("large len={len} seed=0x{seed:04x}"));
        }
    }
}

/// Row 12 — `len` above the `u16` range: `0xFFFF`, `0x1_0000` and neighbours. A
/// `len as u16` bug in the wrapper would silently shorten the message here.
#[test]
fn err_len_above_u16_range() {
    let mut rng = Rng::fixed(112);
    let mut buf = vec![0u8; 0x1_0000 + 32];
    rng.fill(&mut buf);
    for &len in &[
        0xFFFDusize, 0xFFFE, 0xFFFF, 0x1_0000, 0x1_0001, 0x1_0002, 0x1_0007, 0x1_0008, 0x1_0009,
        0x1_000F, 0x1_0010, 0x1_001F, 0x1_0020,
    ] {
        for &seed in &SEED_EXTREMES {
            let ctx = format!("above-u16 len={len} seed=0x{seed:04x}");
            assert_same(&buf[..len], seed, &ctx);
            // Also via the raw entry point, so `len` is not slice-derived.
            unsafe { assert_same_raw(buf.as_ptr(), len as u32, seed, &ctx) };
        }
    }
}

// ---------------------------------------------------------------------------
// Addendum to rows 10 / 12: `len` with the sign bit set.
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn mmap(
        addr: *mut core::ffi::c_void,
        len: usize,
        prot: i32,
        flags: i32,
        fd: i32,
        off: i64,
    ) -> *mut core::ffi::c_void;
    fn munmap(addr: *mut core::ffi::c_void, len: usize) -> i32;
}

const PROT_READ: i32 = 0x1;
const MAP_PRIVATE: i32 = 0x02;
const MAP_ANONYMOUS: i32 = 0x20;
const MAP_NORESERVE: i32 = 0x4000;

/// `len >= 0x8000_0000` — the only inputs that can distinguish a correct
/// zero-extending `len as usize` from a sign-extending `len as i32 as usize`
/// (which would produce `0xFFFF_FFFF_8000_0000` and read wildly out of bounds).
///
/// Uses a 2 GiB read-only `MAP_NORESERVE` anonymous mapping: on Linux, read
/// faults on a private anonymous mapping resolve to the shared zero page, so this
/// costs page tables rather than 2 GiB of RAM.
#[test]
fn err_len_with_sign_bit_set_no_sign_extension() {
    const LEN: usize = 0x8000_0000; // 2 GiB, sign bit of a u32 set
    let total = LEN + 4096;

    let base = unsafe {
        mmap(
            std::ptr::null_mut(),
            total,
            PROT_READ,
            MAP_PRIVATE | MAP_ANONYMOUS | MAP_NORESERVE,
            -1,
            0,
        )
    };
    if base as isize == -1 || base.is_null() {
        eprintln!("SKIP: could not mmap {total} bytes; cannot exercise len >= 0x8000_0000");
        return;
    }
    let ptr = base as *const u8;

    for &(len, seed) in &[
        (0x8000_0000u32, 0x0000u16),
        (0x8000_0000, 0xFFFF),
        (0x8000_0001, 0x1234),
        (0x8000_0007, 0xABCD),
        (0x8000_0008, 0xFFFF),
    ] {
        let ctx = format!("sign-bit len=0x{len:08x} seed=0x{seed:04x}");
        let v = unsafe { assert_same_raw(ptr, len, seed, &ctx) };
        eprintln!("  {ctx} -> 0x{v:04x} (C == Rust)");
    }

    unsafe { munmap(base, total) };
}
