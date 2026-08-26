//! Phase C — error/boundary-path differential tests.
//!
//! One test per row of `ERRORS.md`. The C library contains no error returns, no
//! asserts, no null checks and no enum parameters (verified by grep), so the
//! "error surface" is entirely made of degenerate/boundary inputs. Each test
//! constructs that exact condition, calls BOTH libraries via their exported
//! symbol, and asserts they produce the SAME result — including the same
//! *non*-failure where the C defines one (e.g. `len == 0` with a null pointer).

mod common;

use common::{libs, Rng, SEED};

// ---------------------------------------------------------------- E1
#[test]
fn e1_len_zero_valid_ptr_returns_seed() {
    let l = libs();
    let buf = [0xDEu8, 0xAD, 0xBE, 0xEF, 0x00, 0xFF, 0x55, 0xAA];
    for &seed in &[0x0000u16, 0x0001, 0x00FF, 0xFF00, 0xFFFF, 0x1234, 0xBEEF] {
        // SAFETY: len == 0, pointer is valid anyway.
        let c = unsafe { l.c_raw(buf.as_ptr(), 0, seed) };
        let r = unsafe { l.rust_raw(buf.as_ptr(), 0, seed) };
        assert_eq!(c, r, "E1 divergence at seed 0x{seed:04x}");
        assert_eq!(c, seed, "E1: C must return the seed unchanged for len=0");
        assert_eq!(r, seed, "E1: Rust must return the seed unchanged for len=0");
    }
    // Sweep every seed: len=0 is the identity for all 65536 of them.
    for seed in 0..=u16::MAX {
        let c = unsafe { l.c_raw(buf.as_ptr(), 0, seed) };
        let r = unsafe { l.rust_raw(buf.as_ptr(), 0, seed) };
        assert_eq!(c, r, "E1 divergence at seed 0x{seed:04x}");
        assert_eq!(c, seed);
    }
}

// ---------------------------------------------------------------- E2
#[test]
fn e2_len_zero_null_ptr_returns_seed() {
    let l = libs();
    // The C guards are `len >= 8` (false) and `len--` (evaluates 0 -> false),
    // so the null pointer is never dereferenced and the call is well defined.
    for &seed in &[0x0000u16, 0x0001, 0x00FF, 0xFF00, 0xFFFF, 0xBEEF] {
        // SAFETY: len == 0, so neither implementation dereferences `d`.
        let c = unsafe { l.c_raw(std::ptr::null(), 0, seed) };
        let r = unsafe { l.rust_raw(std::ptr::null(), 0, seed) };
        assert_eq!(
            c, r,
            "E2 divergence for (NULL, 0, 0x{seed:04x}): C=0x{c:04x} Rust=0x{r:04x}"
        );
        assert_eq!(c, seed, "E2: C returns the seed for (NULL, 0)");
        assert_eq!(r, seed, "E2: Rust must not crash and must return the seed");
    }
    // Also a dangling-but-unread pointer: still never dereferenced at len == 0.
    let dangling = 0x1usize as *const u8;
    for &seed in &[0x0000u16, 0xFFFF, 0x5AA5] {
        // SAFETY: len == 0, pointer never read.
        let c = unsafe { l.c_raw(dangling, 0, seed) };
        let r = unsafe { l.rust_raw(dangling, 0, seed) };
        assert_eq!(c, r, "E2 divergence for dangling ptr with len=0");
        assert_eq!(c, seed);
    }
}

// ---------------------------------------------------------------- E3
#[test]
fn e3_len_shorter_than_buffer_ignores_tail() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 0xE3);
    for _ in 0..400 {
        let total = 1 + rng.below(300);
        let data = rng.bytes(total);
        let use_len = rng.below(total + 1) as u32;
        let seed = rng.next_u16();

        // SAFETY: use_len <= total, so only readable bytes are touched.
        let c = unsafe { l.c_raw(data.as_ptr(), use_len, seed) };
        let r = unsafe { l.rust_raw(data.as_ptr(), use_len, seed) };
        assert_eq!(
            c, r,
            "E3 divergence: total={total} use_len={use_len} seed=0x{seed:04x}"
        );
        // Must equal the CRC of just that prefix.
        let prefix = l.c(&data[..use_len as usize], seed);
        assert_eq!(c, prefix, "E3: C must read exactly `len` bytes");
        assert_eq!(r, prefix, "E3: Rust must read exactly `len` bytes");
    }
}

// ---------------------------------------------------------------- E4
#[test]
fn e4_seed_boundary_values() {
    let l = libs();
    // 0xFFFF >> 8 == 0xFF is the last in-bounds index of a 256-entry table;
    // 0x0000 is the first. Exercise both through the block AND the tail path.
    let extremes = [0x0000u16, 0xFFFF, 0x00FF, 0xFF00, 0x8000, 0x0080];
    let mut rng = Rng::new(SEED ^ 0xE4);
    for &seed in &extremes {
        for len in 0..=40usize {
            let data = rng.bytes(len);
            let c = l.c(&data, seed);
            let r = l.rust(&data, seed);
            assert_eq!(
                c, r,
                "E4 divergence: seed=0x{seed:04x} len={len} C=0x{c:04x} Rust=0x{r:04x}"
            );
        }
    }
    // Worst case for the table index in the block loop: after
    // `crc ^= d[0]<<8 | d[1]`, force crc to 0xFFFF so both indices are 0xFF.
    for first_two in [[0x00u8, 0x00], [0xFF, 0xFF], [0xFF, 0x00], [0x00, 0xFF]] {
        for &seed in &extremes {
            let mut data = vec![0xFFu8; 8];
            data[0] = first_two[0];
            data[1] = first_two[1];
            l.assert_same(&data, seed, "E4 forced 0xFF/0x00 block indices");
        }
    }
}

// ---------------------------------------------------------------- E5
#[test]
fn e5_byte_boundary_values() {
    let l = libs();
    // Tail index is `(crc >> 8) ^ *d`: with crc>>8 == 0xFF and byte == 0xFF the
    // index is 0; with byte == 0x00 it is 0xFF. Both extremes of the table.
    for &seed in &[0xFF00u16, 0xFFFF, 0x0000, 0x00FF] {
        for &b in &[0x00u8, 0xFF, 0x01, 0x80, 0x7F] {
            for reps in 0..=9usize {
                let data = vec![b; reps];
                l.assert_same(&data, seed, &format!("E5 byte=0x{b:02x} reps={reps}"));
            }
        }
    }
    // Every byte value in every lane of the 8-byte block (d[0]..d[7]).
    for lane in 0..8usize {
        for b in 0..=255u8 {
            let mut data = [0x00u8; 8];
            data[lane] = b;
            for &seed in &[0x0000u16, 0xFFFF] {
                l.assert_same(&data, seed, &format!("E5 lane={lane} byte=0x{b:02x}"));
            }
            let mut data2 = [0xFFu8; 8];
            data2[lane] = b;
            for &seed in &[0x0000u16, 0xFFFF] {
                l.assert_same(&data2, seed, &format!("E5 lane={lane} byte=0x{b:02x} on 0xFF"));
            }
        }
    }
}

// ---------------------------------------------------------------- E6
#[test]
fn e6_len_off_by_one_boundaries() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 0xE6);
    // The structural boundaries of the block/tail split, and one step either
    // side of each: 0,1,7,8,9,15,16,17,23,24,25,...
    let mut lens: Vec<usize> = vec![0, 1, 2, 7, 8, 9];
    for k in 2..=9usize {
        lens.push(8 * k - 1);
        lens.push(8 * k);
        lens.push(8 * k + 1);
    }
    for &len in &lens {
        for _ in 0..128 {
            let data = rng.bytes(len);
            let seed = rng.next_u16();
            l.assert_same(&data, seed, &format!("E6 boundary len={len}"));
        }
        for &seed in &[0x0000u16, 0xFFFF, 0x00FF, 0xFF00] {
            let data = rng.bytes(len);
            l.assert_same(&data, seed, &format!("E6 boundary len={len} extreme seed"));
        }
    }
}

// ---------------------------------------------------------------- E7
#[test]
fn e7_len_zero_no_underflow_loop() {
    let l = libs();
    // `while (len--)` decrements 0 to 0xFFFFFFFF but must still exit. If the
    // Rust mistranslated this as a do/while or a wrapping loop the call would
    // read ~4 GiB and hang or crash; reaching the assert at all proves it exits.
    let buf = [0x42u8; 8];
    for &seed in &[0x0000u16, 0xFFFF, 0x1357] {
        // SAFETY: len == 0.
        let c = unsafe { l.c_raw(buf.as_ptr(), 0, seed) };
        let r = unsafe { l.rust_raw(buf.as_ptr(), 0, seed) };
        assert_eq!(c, r, "E7 divergence at len=0");
        assert_eq!(r, seed, "E7: Rust must return immediately for len=0");
    }
    // len == 7 is the largest value for which the block loop never runs, i.e.
    // the tail loop must run exactly 7 times and then exit (not wrap).
    for &seed in &[0x0000u16, 0xFFFF] {
        let c = unsafe { l.c_raw(buf.as_ptr(), 7, seed) };
        let r = unsafe { l.rust_raw(buf.as_ptr(), 7, seed) };
        assert_eq!(c, r, "E7 divergence at len=7");
    }
}

// ---------------------------------------------------------------- E8
#[test]
fn e8_large_length_no_counter_truncation() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 0xE8);
    // 1 MiB: far past any 16-bit counter, and includes a non-multiple-of-8 case
    // so the `len -= 8` accumulation and the tail split are both stressed.
    let data = rng.bytes(1024 * 1024 + 5);
    for &seed in &[0x0000u16, 0xFFFF] {
        for &len in &[
            65535usize,
            65536,
            65537,
            1024 * 1024,
            1024 * 1024 + 1,
            1024 * 1024 + 5,
        ] {
            // SAFETY: len <= data.len().
            let c = unsafe { l.c_raw(data.as_ptr(), len as u32, seed) };
            let r = unsafe { l.rust_raw(data.as_ptr(), len as u32, seed) };
            assert_eq!(
                c, r,
                "E8 divergence: len={len} seed=0x{seed:04x} C=0x{c:04x} Rust=0x{r:04x}"
            );
        }
    }
}

// ------------------------------------------------- generic FFI boundaries
//
// The C signature has no enum parameters, so there is no "out-of-range enum
// variant" to smuggle across the FFI boundary. What *can* be smuggled is an
// arbitrary `tflac_u16` seed (all 65536 values are valid — covered
// exhaustively by E1 and C10) and an arbitrary `tflac_u32` length. The tests
// below cover the remaining generic boundaries.

#[test]
fn generic_all_lengths_zero_through_600_agree() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 0xBEEF);
    let data = rng.bytes(600);
    for len in 0..=600usize {
        for &seed in &[0x0000u16, 0xFFFF, 0xA5A5] {
            // SAFETY: len <= data.len().
            let c = unsafe { l.c_raw(data.as_ptr(), len as u32, seed) };
            let r = unsafe { l.rust_raw(data.as_ptr(), len as u32, seed) };
            assert_eq!(c, r, "generic divergence at len={len} seed=0x{seed:04x}");
        }
    }
}

#[test]
fn generic_empty_slice_dangling_but_aligned_ptr() {
    let l = libs();
    // Rust's canonical "empty slice" pointer (NonNull::dangling) is a legal
    // argument when len == 0; make sure the export handles it like C does.
    let empty: &[u8] = &[];
    for &seed in &[0x0000u16, 0xFFFF, 0x0F0F] {
        let c = l.c(empty, seed);
        let r = l.rust(empty, seed);
        assert_eq!(c, r, "generic divergence on empty slice");
        assert_eq!(c, seed);
    }
}

#[test]
fn generic_repeated_calls_are_pure() {
    let l = libs();
    // No global/static mutable state in the C, so repeated identical calls must
    // return identical values on both sides (catches accidental Rust `static mut`
    // or lazily-initialised-table divergence).
    let mut rng = Rng::new(SEED ^ 0xF00D);
    let data = rng.bytes(37);
    let first_c = l.c(&data, 0x1234);
    let first_r = l.rust(&data, 0x1234);
    assert_eq!(first_c, first_r);
    for i in 0..1000 {
        // Interleave other calls to try to perturb any hidden state.
        let n = rng.next_u8() as usize % 40;
        let noise = rng.bytes(n);
        let s1 = rng.next_u16();
        let s2 = rng.next_u16();
        let _ = l.c(&noise, s1);
        let _ = l.rust(&noise, s2);
        assert_eq!(l.c(&data, 0x1234), first_c, "C not pure at iter {i}");
        assert_eq!(l.rust(&data, 0x1234), first_r, "Rust not pure at iter {i}");
    }
}
