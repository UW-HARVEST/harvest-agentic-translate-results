//! Phase C — error-path / boundary differential tests.
//!
//! One test per row of `ERRORS.md`. The C API has no error channel (0 asserts,
//! 0 error returns, 1 `return` statement), so the "error surface" is the set of
//! degenerate / hostile / boundary inputs the C accepts, and the contract is
//! that Rust returns the *same value* rather than panicking, trapping, or
//! wrapping differently. Rows E10/E11 are genuine C UB and are excluded with
//! justification.

mod harness;

use harness::{Impls, Rng, SEED};

// ---------------------------------------------------------------------------
// E1 — len == 0 with a valid pointer: seed returned unchanged, no deref.
// ---------------------------------------------------------------------------
#[test]
fn e1_len_zero_valid_ptr() {
    let im = Impls::load();
    let data = [1u8, 2, 3, 4, 5, 6, 7, 8, 9];
    for seed in [0x0000u16, 0x0001, 0x1234, 0x7FFF, 0x8000, 0xFF00, 0x00FF, 0xFFFF] {
        let v = im.check(&data, 0, seed, "E1");
        assert_eq!(v, seed, "E1: len=0 must return the seed unchanged");
    }
}

// ---------------------------------------------------------------------------
// E2 — len == 0 with d == NULL: must not fault; must return the seed.
// ---------------------------------------------------------------------------
#[test]
fn e2_len_zero_null_ptr() {
    let im = Impls::load();
    for seed in [0x0000u16, 0x0001, 0x00FF, 0xFF00, 0x8000, 0x7FFF, 0xABCD] {
        let v = im.check_raw(std::ptr::null(), 0, seed, "E2 null ptr len=0");
        assert_eq!(v, seed, "E2: null + len=0 must return the seed unchanged");
    }
}

// ---------------------------------------------------------------------------
// E3 — len == 0, NULL, maximum seed.
// ---------------------------------------------------------------------------
#[test]
fn e3_len_zero_null_ptr_max_seed() {
    let im = Impls::load();
    let v = im.check_raw(std::ptr::null(), 0, 0xFFFF, "E3 null ptr max seed");
    assert_eq!(v, 0xFFFF);
}

// ---------------------------------------------------------------------------
// E4 — len == 0 with a deliberately bogus non-null (unmapped) pointer.
// The pointer must never be formed into a dereference or an offset.
// ---------------------------------------------------------------------------
#[test]
fn e4_len_zero_bogus_ptr() {
    let im = Impls::load();
    let bogus: [*const u8; 5] = [
        1usize as *const u8,
        7usize as *const u8,
        0xDEAD_BEEFusize as *const u8,
        usize::MAX as *const u8,
        (usize::MAX - 3) as *const u8,
    ];
    for &p in &bogus {
        for seed in [0x0000u16, 0xFFFF, 0x5A5A] {
            let v = im.check_raw(p, 0, seed, "E4 bogus ptr len=0");
            assert_eq!(v, seed, "E4: bogus ptr + len=0 must return the seed");
        }
    }
}

// ---------------------------------------------------------------------------
// E5 — `while (len--)` underflow: len wraps 0 -> 0xFFFFFFFF after the final
// test. Must not be observable and must not panic (debug builds included).
// ---------------------------------------------------------------------------
#[test]
fn e5_tail_len_underflow() {
    let im = Impls::load();
    let mut rng = Rng::new(SEED ^ 0x05);
    // Every length whose tail loop runs down to zero: residues 1..=7, both with
    // and without a preceding block.
    for len in [1u32, 2, 3, 4, 5, 6, 7, 9, 10, 15, 17, 23, 31, 63, 127] {
        let data = rng.bytes(len as usize);
        for _ in 0..64 {
            let seed = rng.next_u16();
            im.check(&data, len, seed, "E5 tail underflow");
        }
        for seed in [0x0000u16, 0xFFFF] {
            im.check(&data, len, seed, "E5 tail underflow extreme seed");
        }
    }
    // The minimal case, isolated.
    let one = [0xABu8];
    im.check(&one, 1, 0xFFFF, "E5 minimal len=1");
}

// ---------------------------------------------------------------------------
// E6 — crc == 0xFFFF entering the tail loop: `crc16 << 8` exceeds 16 bits in
// C (int promotion) and is truncated on assignment. Rust must truncate the
// same way and must not trip an overflow check.
// ---------------------------------------------------------------------------
#[test]
fn e6_seed_max_tail_shift_overflow() {
    let im = Impls::load();
    // Feed every byte value with the maximal seed through the tail loop, and
    // then chain so that intermediate crc values also reach the high range.
    for b in 0u16..=255 {
        let data = [b as u8];
        im.check(&data, 1, 0xFFFF, "E6 max seed tail");
        im.check(&data, 1, 0xFF00, "E6 high-byte seed tail");
        im.check(&data, 1, 0x00FF, "E6 low-byte seed tail");
    }
    // Multi-byte tails where crc stays large across iterations.
    let hi = vec![0xFFu8; 7];
    for len in 1u32..=7 {
        im.check(&hi, len, 0xFFFF, "E6 all-ff tail, max seed");
    }
    // Chained: keep feeding the previous (possibly >0x8000) result back in.
    let mut cc = 0xFFFFu16;
    let mut rc = 0xFFFFu16;
    let mut rng = Rng::new(SEED ^ 0x06);
    for _ in 0..5000 {
        let n = 1 + rng.below(7);
        let d = rng.bytes(n);
        cc = im.c_call(&d, n as u32, cc);
        rc = im.rust_call(&d, n as u32, rc);
        assert_eq!(cc, rc, "E6 chained tail overflow divergence");
    }
}

// ---------------------------------------------------------------------------
// E7 — crc == 0xFFFF entering the block loop: crc>>8 == 0xFF and
// crc & 0xFF == 0xFF, i.e. the LAST element of tables [7] and [6].
// ---------------------------------------------------------------------------
#[test]
fn e7_seed_max_block_max_table_index() {
    let im = Impls::load();
    // Choose d[0],d[1] so that (crc ^ (d0<<8|d1)) >> 8 == 0xFF and & 0xFF == 0xFF,
    // i.e. crc ^ word == 0xFFFF.
    for seed in [0x0000u16, 0x1234, 0xFFFF, 0x8000, 0x00FF, 0xFF00] {
        let word = seed ^ 0xFFFF; // makes the post-xor crc exactly 0xFFFF
        let mut blk = [0u8; 8];
        blk[0] = (word >> 8) as u8;
        blk[1] = (word & 0xFF) as u8;
        for tail in 2..8usize {
            blk[tail] = 0xFF;
        }
        im.check(&blk, 8, seed, "E7 max table index [7]/[6]");
        // and the minimum index (post-xor crc == 0x0000)
        let word0 = seed;
        let mut blk0 = [0u8; 8];
        blk0[0] = (word0 >> 8) as u8;
        blk0[1] = (word0 & 0xFF) as u8;
        im.check(&blk0, 8, seed, "E7 min table index [7]/[6]");
    }
    // Blanket: all 65536 seeds against an all-0xFF block already covered in
    // Phase B C11; here assert the specific max-index construction over a
    // randomized sweep too.
    let mut rng = Rng::new(SEED ^ 0x07);
    for _ in 0..4000 {
        let seed = rng.next_u16();
        let word = seed ^ 0xFFFF;
        let mut blk = rng.bytes(8);
        blk[0] = (word >> 8) as u8;
        blk[1] = (word & 0xFF) as u8;
        im.check(&blk, 8, seed, "E7 randomized max index");
    }
}

// ---------------------------------------------------------------------------
// E8 — data byte 0xFF in every table-indexed position: index 255 on tables
// [0]..[5]. Max in-range index; must not be an OOB index in Rust.
// ---------------------------------------------------------------------------
#[test]
fn e8_all_ff_data_max_table_index() {
    let im = Impls::load();
    let ff = vec![0xFFu8; 64];
    for len in 0u32..=64 {
        for seed in [0x0000u16, 0xFFFF, 0x00FF, 0xFF00, 0x8080] {
            im.check(&ff, len, seed, "E8 all 0xFF");
        }
    }
    // One 0xFF at a time in each of d[2]..d[7], zeros elsewhere.
    for slot in 2usize..8 {
        let mut blk = [0u8; 8];
        blk[slot] = 0xFF;
        for seed in [0x0000u16, 0xFFFF] {
            im.check(&blk, 8, seed, "E8 single 0xFF slot");
        }
    }
}

// ---------------------------------------------------------------------------
// E9 — tail-loop index `(crc16 >> 8) ^ *d`: exhaustively probe all 256 x 256
// operand pairs so the index covers 0..=255 with no OOB.
// ---------------------------------------------------------------------------
#[test]
fn e9_tail_index_operand_extremes() {
    let im = Impls::load();
    // hi byte of the seed x every data byte => every (crc>>8) ^ *d pair.
    for hi in 0u16..=255 {
        for b in 0u16..=255 {
            let data = [b as u8];
            // low byte of the seed varied too, over its extremes
            for lo in [0x00u16, 0x01, 0x80, 0xFF] {
                let seed = (hi << 8) | lo;
                let cv = im.c_call(&data, 1, seed);
                let rv = im.rust_call(&data, 1, seed);
                assert_eq!(
                    cv, rv,
                    "E9 DIVERGENCE: seed=0x{seed:04x} byte=0x{b:02x} \
                     idx=0x{:02x} C=0x{cv:04x} Rust=0x{rv:04x}",
                    (hi as u8) ^ (b as u8)
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// E10 / E11 — genuine C undefined behaviour, deliberately NOT executed.
//
// E10: `len` larger than the allocation -> the C reads past the buffer.
// E11: `d == NULL` with `len > 0`   -> the C dereferences NULL.
//
// Both crash the process in *both* implementations, so there is no observable
// value to compare; running them would abort the harness rather than prove
// anything. This test documents the exclusion and asserts the adjacent
// *defined* boundary instead: the largest len that is still in bounds.
// ---------------------------------------------------------------------------
#[test]
fn e10_e11_ub_excluded_largest_in_bounds_len() {
    let im = Impls::load();
    let mut rng = Rng::new(SEED ^ 0x0B);
    for n in [1usize, 7, 8, 9, 63, 64, 65, 1000] {
        let data = rng.bytes(n);
        // exactly len == buffer size: the last defined length
        for seed in [0x0000u16, 0xFFFF, 0x1357] {
            im.check(&data, n as u32, seed, "E10/E11 largest in-bounds len");
            // one step below the boundary, also defined
            im.check(&data, (n - 1) as u32, seed, "E10/E11 len-1");
        }
    }
}

// ---------------------------------------------------------------------------
// E12 — there is no enum parameter in this API, so the "out-of-range enum
// across FFI" class degenerates to "out-of-domain integer", which is
// impossible: every bit pattern of u32 len and u16 crc is valid input.
// This test pins that by feeding hostile/extreme integer bit patterns that a
// naive translation might treat as sentinels.
// ---------------------------------------------------------------------------
#[test]
fn e12_no_enum_full_integer_domain() {
    let im = Impls::load();
    // No `enum` anywhere in the C source (verified by grep) - assert the
    // header still declares only the integer-typed signature we tested.
    let hdr = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("c_src/include/lib.h"),
    )
    .expect("read lib.h");
    assert!(
        !hdr.contains("enum"),
        "E12 assumption broken: the C header now contains an enum; \
         ERRORS.md must be regenerated with per-variant rows"
    );

    let data = vec![0xA5u8; 256];
    // Seed values that look like sentinels/negative-int reinterpretations.
    let hostile_seeds = [
        0x0000u16, 0xFFFF, 0x8000, 0x7FFF, 0xFFFE, 0x0001, 0xDEAD, 0xBEEF, 0xCAFE,
    ];
    // Lengths that look like sentinels but are still <= buffer size.
    let hostile_lens = [0u32, 1, 2, 127, 128, 255, 256];
    for &seed in &hostile_seeds {
        for &len in &hostile_lens {
            im.check(&data, len, seed, "E12 hostile integer values");
        }
    }
}

// ---------------------------------------------------------------------------
// E13 — exhaustive sweep of the ENTIRE seed domain (all 65536 values), the
// "one past the valid range" test for a parameter with no invalid values.
// ---------------------------------------------------------------------------
#[test]
fn e13_exhaustive_all_65536_seeds() {
    let im = Impls::load();
    // A shape that runs both loops (block + tail) so each seed traverses the
    // whole function, not just one branch.
    let data: [u8; 11] = [0x00, 0xFF, 0x10, 0xEF, 0x7F, 0x80, 0x01, 0xFE, 0xAA, 0x55, 0xC3];
    for s in 0u32..=0xFFFF {
        let seed = s as u16;
        let cv = im.c_call(&data, 11, seed);
        let rv = im.rust_call(&data, 11, seed);
        assert_eq!(
            cv, rv,
            "E13 DIVERGENCE at seed=0x{seed:04x}: C=0x{cv:04x} Rust=0x{rv:04x}"
        );
        // and with len==0, where the seed is returned verbatim
        let c0 = im.c_call(&data, 0, seed);
        let r0 = im.rust_call(&data, 0, seed);
        assert_eq!(c0, r0);
        assert_eq!(c0, seed);
    }
}

// ---------------------------------------------------------------------------
// E14 — the exact loop-split boundaries: 6,7,8,9,15,16,17 (and 23,24,25).
// ---------------------------------------------------------------------------
#[test]
fn e14_loop_split_boundaries() {
    let im = Impls::load();
    let mut rng = Rng::new(SEED ^ 0x0E);
    let data = rng.bytes(64);
    for len in [0u32, 1, 6, 7, 8, 9, 15, 16, 17, 23, 24, 25, 31, 32, 33] {
        for seed in [0x0000u16, 0xFFFF, 0x00FF, 0xFF00, 0x1234] {
            im.check(&data, len, seed, "E14 loop split boundary");
        }
        for _ in 0..200 {
            let d = rng.bytes(len as usize);
            let seed = rng.next_u16();
            im.check(&d, len, seed, "E14 randomized boundary");
        }
    }
}

// ---------------------------------------------------------------------------
// E15 — seed passthrough composes: crc16(d,0,c) == c, for all c, and using it
// as a seed later is indistinguishable from passing c directly.
// ---------------------------------------------------------------------------
#[test]
fn e15_seed_passthrough_composition() {
    let im = Impls::load();
    let mut rng = Rng::new(SEED ^ 0x0F);
    let data = rng.bytes(128);
    for s in 0u32..=0xFFFF {
        let seed = s as u16;
        let c0 = im.c_call(&data, 0, seed);
        let r0 = im.rust_call(&data, 0, seed);
        assert_eq!(c0, seed, "E15: C len=0 is not the identity");
        assert_eq!(r0, seed, "E15: Rust len=0 is not the identity");
    }
    for _ in 0..1000 {
        let seed = rng.next_u16();
        let len = rng.below(129) as u32;
        let via_c = im.c_call(&data, len, im.c_call(&data, 0, seed));
        let via_r = im.rust_call(&data, len, im.rust_call(&data, 0, seed));
        let direct_c = im.c_call(&data, len, seed);
        let direct_r = im.rust_call(&data, len, seed);
        assert_eq!(via_c, direct_c);
        assert_eq!(via_r, direct_r);
        assert_eq!(via_c, via_r, "E15 composed mismatch");
    }
}

// ---------------------------------------------------------------------------
// Symbol-level sanity: the Rust .so must export `crc16` and must NOT export
// the internal tables (which are `static` in C, i.e. internal linkage).
// ---------------------------------------------------------------------------
#[test]
fn symbol_export_shape() {
    let im = Impls::load();

    // Guard against the stale-.so trap: `cargo test` does not rebuild a
    // cdylib, so the harness builds it into target/so-under-test/<profile>.
    // Assert we really loaded that freshly-built artifact, in this profile.
    let rp = im.rust_path.to_string_lossy().to_string();
    assert!(
        rp.contains("so-under-test"),
        "loaded the wrong Rust .so ({rp}); it must be the one the harness just built"
    );
    let want_profile = if std::env::current_exe()
        .map(|p| p.to_string_lossy().contains("/release/"))
        .unwrap_or(false)
    {
        "/release/"
    } else {
        "/debug/"
    };
    assert!(
        rp.contains(want_profile),
        "profile mismatch: test binary is {want_profile} but loaded {rp}"
    );

    unsafe {
        let lib = libloading::Library::new(&im.rust_path).unwrap();
        let f: Result<libloading::Symbol<harness::Crc16Fn>, _> = lib.get(b"crc16\0");
        assert!(f.is_ok(), "Rust .so must export crc16");
        let t: Result<libloading::Symbol<*const u8>, _> = lib.get(b"tflac_crc16_tables\0");
        assert!(
            t.is_err(),
            "tflac_crc16_tables is `static` in C (internal linkage); \
             the Rust .so must not export it either"
        );
    }
}
