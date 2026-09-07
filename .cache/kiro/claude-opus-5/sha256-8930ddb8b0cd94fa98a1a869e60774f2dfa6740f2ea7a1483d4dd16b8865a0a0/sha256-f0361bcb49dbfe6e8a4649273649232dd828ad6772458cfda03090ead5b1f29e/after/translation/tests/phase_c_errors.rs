//! Phase C — error/boundary-path differential tests, one test per row of
//! `ERRORS.md`.
//!
//! The C library contains **no** rejection site (see `ERRORS.md` for the
//! mechanical derivation), so there is no error code or sentinel to compare.
//! What remains are the generic C-API boundaries; each one asserts C and Rust
//! agree *exactly*, and where the C result is analytically known (e.g. `len==0`
//! returns the seed) that value is asserted too, not merely "both agree".

mod common;
use common::{assert_same, assert_same_slice, Rng};

/// ERRORS B1 — `d == NULL` with `len == 0`: neither loop body runs, so the NULL
/// is never dereferenced. Must return the seed unchanged on both sides.
#[test]
fn phase_c_b1_null_pointer_zero_len() {
    for seed in [0x0000u16, 0x0001, 0x00FF, 0x0100, 0x7FFF, 0x8000, 0xFFFE, 0xFFFF] {
        let out = assert_same(&format!("B1 seed=0x{seed:04x}"), std::ptr::null(), 0, seed);
        assert_eq!(out, seed, "B1: NULL + len==0 must return the seed verbatim");
    }
    let mut rng = Rng::new(0xC001);
    for i in 0..5000 {
        let seed = rng.u16();
        let out = assert_same(&format!("B1 rand #{i}"), std::ptr::null(), 0, seed);
        assert_eq!(out, seed);
    }
}

/// ERRORS B2 — `len == 0` with a valid pointer.
#[test]
fn phase_c_b2_zero_len_valid_ptr() {
    let buf = [0xDEu8, 0xAD, 0xBE, 0xEF];
    let mut rng = Rng::new(0xC002);
    for i in 0..5000 {
        let seed = rng.u16();
        let out = assert_same(&format!("B2 #{i}"), buf.as_ptr(), 0, seed);
        assert_eq!(out, seed, "B2: len==0 must return the seed verbatim");
    }
}

/// ERRORS B3 — `len == 1`: one step past the empty case, tail loop only.
#[test]
fn phase_c_b3_len_one() {
    // Exhaustive over the whole (byte, seed) domain would be 2^24; sweep all
    // 256 bytes against a representative + random seed set instead.
    let mut rng = Rng::new(0xC003);
    for b in 0u16..=255 {
        let byte = b as u8;
        for seed in [0x0000u16, 0x00FF, 0xFF00, 0xFFFF] {
            assert_same_slice(&format!("B3 byte=0x{byte:02x} seed=0x{seed:04x}"), &[byte], seed);
        }
        for _ in 0..20 {
            let seed = rng.u16();
            assert_same_slice(&format!("B3 byte=0x{byte:02x} rand"), &[byte], seed);
        }
    }
}

/// ERRORS B4 — `len == 7`: one step *below* the `len >= 8` bulk threshold.
#[test]
fn phase_c_b4_len_seven_below_threshold() {
    let mut rng = Rng::new(0xC004);
    for i in 0..20_000 {
        let mut buf = [0u8; 7];
        rng.fill(&mut buf);
        let seed = rng.u16();
        assert_same_slice(&format!("B4 #{i}"), &buf, seed);
    }
    // Extremes of the data domain at the boundary length.
    for buf in [[0x00u8; 7], [0xFFu8; 7]] {
        for seed in [0x0000u16, 0xFFFF] {
            assert_same_slice("B4 extreme", &buf, seed);
        }
    }
}

/// ERRORS B5 — `len == 8`: exactly at the bulk threshold.
#[test]
fn phase_c_b5_len_eight_at_threshold() {
    let mut rng = Rng::new(0xC005);
    for i in 0..20_000 {
        let mut buf = [0u8; 8];
        rng.fill(&mut buf);
        let seed = rng.u16();
        assert_same_slice(&format!("B5 #{i}"), &buf, seed);
    }
    for buf in [[0x00u8; 8], [0xFFu8; 8]] {
        for seed in [0x0000u16, 0xFFFF] {
            assert_same_slice("B5 extreme", &buf, seed);
        }
    }
}

/// ERRORS B6 — `len == 9`: one step past the threshold (bulk + 1 tail byte).
#[test]
fn phase_c_b6_len_nine_past_threshold() {
    let mut rng = Rng::new(0xC006);
    for i in 0..20_000 {
        let mut buf = [0u8; 9];
        rng.fill(&mut buf);
        let seed = rng.u16();
        assert_same_slice(&format!("B6 #{i}"), &buf, seed);
    }
    for buf in [[0x00u8; 9], [0xFFu8; 9]] {
        for seed in [0x0000u16, 0xFFFF] {
            assert_same_slice("B6 extreme", &buf, seed);
        }
    }
}

/// ERRORS B7 / B8 — the minimum and maximum of the seed range, plus one step
/// either side by wrapping (`0xFFFF + 1 == 0x0000`, `0x0000 - 1 == 0xFFFF`);
/// the seed has no invalid encoding, so both extremes are legal input.
#[test]
fn phase_c_b7_b8_seed_range_extremes() {
    let mut rng = Rng::new(0xC007);
    let seeds = [
        0x0000u16,
        0x0001,
        0x00FF,
        0x0100,
        0x7FFF,
        0x8000,
        0xFF00,
        0xFFFE,
        0xFFFF,
        0xFFFFu16.wrapping_add(1), // == 0x0000, "one past" the range
        0x0000u16.wrapping_sub(1), // == 0xFFFF, "one before" the range
    ];
    for &seed in &seeds {
        for len in 0u32..=24 {
            for _ in 0..40 {
                let mut buf = vec![0u8; (len as usize).max(1)];
                rng.fill(&mut buf);
                assert_same(
                    &format!("B7/B8 seed=0x{seed:04x} len={len}"),
                    buf.as_ptr(),
                    len,
                    seed,
                );
            }
        }
    }
}

/// ERRORS B9 / B10 — data bytes at the minimum (0x00) and maximum (0xFF) table
/// index. A byte cannot exceed 0xFF, so 0xFF *is* the last in-range index and
/// index 255 is the last table slot: this is the "one step from out of range"
/// case for the table lookup.
#[test]
fn phase_c_b9_b10_byte_index_extremes() {
    let mut rng = Rng::new(0xC008);
    for (name, fill) in [("B9 0x00", 0x00u8), ("B10 0xFF", 0xFFu8)] {
        let buf = vec![fill; 128];
        for len in 0u32..=128 {
            for _ in 0..20 {
                let seed = rng.u16();
                assert_same(&format!("{name} len={len}"), buf.as_ptr(), len, seed);
            }
        }
    }
}

/// ERRORS B11 — every `tflac_crc16_tables[t][i]` slot is reached: for each of
/// the 8 bulk lanes, drive every byte value 0..=255 through that lane, and
/// drive every byte value through the tail lane too.
#[test]
fn phase_c_b11_every_table_slot_reachable() {
    let mut rng = Rng::new(0xC009);

    // Bulk lanes: lane k reads d[k] of each 8-byte block.
    for lane in 0usize..8 {
        let mut buf = vec![0u8; 256 * 8];
        for v in 0..256usize {
            buf[v * 8 + lane] = v as u8;
        }
        for seed in [0x0000u16, 0xFFFF, rng.u16(), rng.u16()] {
            assert_same_slice(&format!("B11 bulk lane={lane} seed=0x{seed:04x}"), &buf, seed);
        }
    }

    // Tail lane (tables[0]) with every byte value, at every tail length 1..=7.
    for taillen in 1usize..=7 {
        for v in 0..256usize {
            let buf = vec![v as u8; taillen];
            for seed in [0x0000u16, 0xFFFF] {
                assert_same_slice(&format!("B11 tail len={taillen} v={v}"), &buf, seed);
            }
        }
    }

    // Lane-7/lane-6 indices come from the *running CRC*, not the data, so sweep
    // all 65536 seeds through a single 8-byte block to cover both.
    let mut block = [0u8; 8];
    rng.fill(&mut block);
    for s in 0u32..=0xFFFF {
        assert_same_slice(&format!("B11 crc-lane seed=0x{s:04x}"), &block, s as u16);
    }
}

/// ERRORS B12 — large in-buffer `len` values, and `len` covering all 8 residues.
/// A `len` larger than the buffer is undefined behaviour in C (it would read out
/// of bounds), so it is deliberately NOT exercised; instead `len` is pushed to
/// large legal values.
#[test]
fn phase_c_b12_large_and_residue_lengths() {
    let mut rng = Rng::new(0xC00A);
    let mut buf = vec![0u8; 1024 * 1024 + 8];
    rng.fill(&mut buf);

    for r in 0u32..8 {
        for base in [0u32, 8, 4096, 65536, 1024 * 1024] {
            let len = base + r;
            if len as usize > buf.len() {
                continue;
            }
            let seed = rng.u16();
            assert_same(&format!("B12 len={len}"), buf.as_ptr(), len, seed);
        }
    }

    // Exactly the whole buffer.
    let seed = rng.u16();
    assert_same("B12 full", buf.as_ptr(), buf.len() as u32, seed);
}

/// ERRORS B13 — unaligned pointers: the Rust must read byte-wise like the C.
#[test]
fn phase_c_b13_unaligned_pointer() {
    let mut rng = Rng::new(0xC00B);
    let mut backing = vec![0u8; 4096];
    rng.fill(&mut backing);
    for off in 0usize..16 {
        for len in 0u32..=64 {
            if off + len as usize > backing.len() {
                continue;
            }
            for _ in 0..8 {
                let seed = rng.u16();
                let p = unsafe { backing.as_ptr().add(off) };
                assert_same(&format!("B13 off={off} len={len}"), p, len, seed);
            }
        }
    }
}

/// ERRORS B14 — out-of-range enum values across the FFI boundary.
///
/// The public API has **no enum parameter** (`const uint8_t*`, `uint32_t`,
/// `uint16_t`), so there is no invalid-variant int to smuggle across. The
/// closest analogue is that every bit pattern of the two integer parameters is a
/// valid input: this test drives the seed over its entire domain and `len` over
/// the low/high extremes of the `u32` domain that remain in-bounds for the
/// buffer, confirming there is no value the Rust rejects but the C accepts (or
/// vice versa).
#[test]
fn phase_c_b14_no_enum_full_integer_domain() {
    let mut rng = Rng::new(0xC00C);
    let mut buf = vec![0u8; 64];
    rng.fill(&mut buf);

    // Full u16 seed domain at several lengths.
    for len in [0u32, 1, 7, 8, 9, 16, 63, 64] {
        for s in 0u32..=0xFFFF {
            let ret_c_eq_rs = assert_same(&format!("B14 len={len} seed=0x{s:04x}"), buf.as_ptr(), len, s as u16);
            let _ = ret_c_eq_rs;
        }
    }

    // len == 0 must be accepted for every seed even with a null pointer; no
    // in-band error value exists, so the result is always a plain CRC.
    for s in 0u32..=0xFFFF {
        let out = assert_same("B14 null len=0", std::ptr::null(), 0, s as u16);
        assert_eq!(out, s as u16);
    }
}
