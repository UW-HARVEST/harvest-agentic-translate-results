//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Both libraries are driven only through
//! their `.so` exports. `md5_digest` is the lowest-level (and only) public
//! entry point, so every row calls it directly.

mod common;

use common::*;

/// Row 0 (prerequisite): the harness's view of the struct ABI matches the C
/// one. If this is wrong every other row is meaningless.
#[test]
fn layout_matches_c_abi() {
    assert_eq!(std::mem::size_of::<Md5>(), 16, "sizeof(struct tflac_md5)");
    assert_eq!(std::mem::align_of::<Md5>(), 4, "alignof(struct tflac_md5)");
    let m = Md5::new(0, 0, 0, 0);
    let base = &m as *const Md5 as usize;
    assert_eq!(&m.a as *const u32 as usize - base, 0, "offsetof(a)");
    assert_eq!(&m.b as *const u32 as usize - base, 4, "offsetof(b)");
    assert_eq!(&m.c as *const u32 as usize - base, 8, "offsetof(c)");
    assert_eq!(&m.d as *const u32 as usize - base, 12, "offsetof(d)");
}

/// Row 1 — all words zero.
#[test]
fn row01_all_zero() {
    let m = Md5::new(0, 0, 0, 0);
    assert_same("row01", &m);
    assert_eq!(call(Impl::C, &m), [0u8; 16], "C must fully overwrite the poison");
}

/// Row 2 — all words all-ones.
#[test]
fn row02_all_ones() {
    let m = Md5::new(u32::MAX, u32::MAX, u32::MAX, u32::MAX);
    assert_same("row02", &m);
    assert_eq!(call(Impl::C, &m), [0xFFu8; 16]);
}

/// Rows 3-6 — isolate each field, randomized values, and confirm the field
/// lands in its own 4-byte window and disturbs no other window.
#[test]
fn row03_06_isolated_fields() {
    let mut rng = Rng::new(0xB0_0000_03);
    for field in 0..4usize {
        for _ in 0..ITERS {
            let v = rng.next_u32();
            let mut m = Md5::default();
            match field {
                0 => m.a = v,
                1 => m.b = v,
                2 => m.c = v,
                _ => m.d = v,
            }
            let row = format!("row{:02}", 3 + field);
            assert_same(&row, &m);
            // The other 12 bytes must be zero in BOTH (derived from the C:
            // each word is stored to a fixed disjoint window).
            let (c, r) = both(&m);
            for (i, (&cb, &rb)) in c.iter().zip(r.iter()).enumerate() {
                let owner = i / 4;
                if owner != field {
                    assert_eq!(cb, 0, "[{row}] C wrote non-zero at out[{i}] for field {field}");
                    assert_eq!(rb, 0, "[{row}] Rust wrote non-zero at out[{i}] for field {field}");
                }
            }
        }
    }
}

/// Row 7 — one-hot over all 128 struct bits: pins every (word, shift) pair.
#[test]
fn row07_one_hot_all_128_bits() {
    for bit in 0..128usize {
        let word = bit / 32;
        let shift = bit % 32;
        let v = 1u32 << shift;
        let mut m = Md5::default();
        match word {
            0 => m.a = v,
            1 => m.b = v,
            2 => m.c = v,
            _ => m.d = v,
        }
        assert_same("row07", &m);
        // Exactly one output bit must be set, at the position the C shifts imply.
        let c = call(Impl::C, &m);
        let set: Vec<(usize, u8)> = c
            .iter()
            .enumerate()
            .filter(|&(_, &b)| b != 0)
            .map(|(i, &b)| (i, b))
            .collect();
        assert_eq!(set.len(), 1, "bit {bit}: expected exactly one non-zero out byte, got {set:?}");
        let (idx, byte) = set[0];
        assert_eq!(idx, word * 4 + shift / 8, "bit {bit}: wrong output byte index");
        assert_eq!(byte, 1u8 << (shift % 8), "bit {bit}: wrong bit within the byte");
    }
}

/// Row 8 — byte-lane-hot values pin the four shift amounts per word.
#[test]
fn row08_byte_lane_hot() {
    const LANES: [u32; 4] = [0x0000_00FF, 0x0000_FF00, 0x00FF_0000, 0xFF00_0000];
    for word in 0..4usize {
        for (lane, &v) in LANES.iter().enumerate() {
            let mut m = Md5::default();
            match word {
                0 => m.a = v,
                1 => m.b = v,
                2 => m.c = v,
                _ => m.d = v,
            }
            assert_same("row08", &m);
            let c = call(Impl::C, &m);
            let mut expect = [0u8; 16];
            expect[word * 4 + lane] = 0xFF;
            assert_eq!(c, expect, "word {word} lane {lane}");
        }
    }
}

/// Row 9 — distinct sentinel per output byte; a swapped field or lane is
/// immediately visible.
#[test]
fn row09_distinct_lane_sentinels() {
    let m = Md5::new(0x0302_0100, 0x0706_0504, 0x0B0A_0908, 0x0F0E_0D0C);
    assert_same("row09", &m);
    let c = call(Impl::C, &m);
    let expect: [u8; 16] = core::array::from_fn(|i| i as u8);
    assert_eq!(c, expect, "C little-endian lane order");
    assert_eq!(call(Impl::Rust, &m), expect, "Rust little-endian lane order");
}

/// Row 10 — full cross-product of the boundary word values over all 4 fields
/// (14^4 = 38416 combinations).
#[test]
fn row10_boundary_cross_product() {
    for &a in &BOUNDARY_WORDS {
        for &b in &BOUNDARY_WORDS {
            for &c in &BOUNDARY_WORDS {
                for &d in &BOUNDARY_WORDS {
                    assert_same("row10", &Md5::new(a, b, c, d));
                }
            }
        }
    }
}

/// Row 11 — uniform random struct values, fixed seed.
#[test]
fn row11_randomized_words() {
    let mut rng = Rng::new(0x5EED_0011);
    for _ in 0..(ITERS * 8) {
        assert_same("row11", &rng.next_md5());
    }
}

/// Row 12 — struct built from a raw random 16-byte image (exercises the
/// `#[repr(C)]` layout the same way a C caller filling the struct by memcpy
/// would).
#[test]
fn row12_random_raw_images() {
    let mut rng = Rng::new(0x5EED_0012);
    for _ in 0..(ITERS * 2) {
        let img = rng.next_image();
        let m = Md5::from_image(&img);
        assert_same("row12", &m);
        // The 16 output bytes must be exactly the struct's memory image.
        assert_eq!(call(Impl::C, &m), img, "C output != struct image");
        assert_eq!(call(Impl::Rust, &m), img, "Rust output != struct image");
    }
}

/// Row 13 — `out` at every byte offset of a larger arena, aligned and not.
#[test]
fn row13_out_at_every_offset() {
    let cf = digest(Impl::C);
    let rf = digest(Impl::Rust);
    let mut rng = Rng::new(0x5EED_0013);
    for _ in 0..ITERS {
        let m = rng.next_md5();
        for off in 0..9usize {
            let mut arena_c = [0xAAu8; 32];
            let mut arena_r = [0xAAu8; 32];
            unsafe {
                cf(&m as *const Md5, arena_c.as_mut_ptr().add(off));
                rf(&m as *const Md5, arena_r.as_mut_ptr().add(off));
            }
            assert_eq!(
                arena_c, arena_r,
                "[row13] offset {off} divergence for {m:?}\n  C   : {arena_c:02x?}\n  Rust: {arena_r:02x?}"
            );
            // Bytes outside the 16-byte window must still be poison.
            for i in 0..32 {
                if i < off || i >= off + 16 {
                    assert_eq!(arena_c[i], 0xAA, "[row13] C touched arena[{i}] (off {off})");
                    assert_eq!(arena_r[i], 0xAA, "[row13] Rust touched arena[{i}] (off {off})");
                }
            }
        }
    }
}

/// Row 14 — buffer reuse across different inputs: the C is stateless, so a
/// later call must fully replace the earlier result with no accumulation.
#[test]
fn row14_buffer_reuse_no_hidden_state() {
    let cf = digest(Impl::C);
    let rf = digest(Impl::Rust);
    let mut rng = Rng::new(0x5EED_0014);
    let mut buf_c = [0xAAu8; 16];
    let mut buf_r = [0xAAu8; 16];
    for _ in 0..(ITERS * 4) {
        let m = rng.next_md5();
        unsafe {
            cf(&m as *const Md5, buf_c.as_mut_ptr());
            rf(&m as *const Md5, buf_r.as_mut_ptr());
        }
        assert_eq!(buf_c, buf_r, "[row14] divergence on reused buffer for {m:?}");
        // Must equal the fresh-buffer result: no dependence on prior contents.
        assert_eq!(buf_c, call(Impl::C, &m), "[row14] C result depends on prior buffer contents");
        assert_eq!(buf_r, call(Impl::Rust, &m), "[row14] Rust result depends on prior buffer contents");
    }
}

/// Row 15 — purity: the same input twice yields the same bytes, in both libs,
/// and `m` itself is not modified (the parameter is `const`).
#[test]
fn row15_idempotent_and_input_untouched() {
    let mut rng = Rng::new(0x5EED_0015);
    for _ in 0..ITERS {
        let m = rng.next_md5();
        let snapshot = m;
        let c1 = call(Impl::C, &m);
        let c2 = call(Impl::C, &m);
        let r1 = call(Impl::Rust, &m);
        let r2 = call(Impl::Rust, &m);
        assert_eq!(c1, c2, "[row15] C not idempotent");
        assert_eq!(r1, r2, "[row15] Rust not idempotent");
        assert_eq!(c1, r1, "[row15] divergence for {m:?}");
        assert_eq!(m, snapshot, "[row15] const input was mutated");
    }
}

/// Row 16 — every one of the 16 bytes is written; nothing is left as poison.
/// Checked under several poison patterns so a "leaves 0x00 behind" bug cannot
/// hide behind a zero-valued word.
#[test]
fn row16_all_16_bytes_written() {
    let mut rng = Rng::new(0x5EED_0016);
    for poison in [0x00u8, 0xFF, 0xAA, 0x55] {
        for _ in 0..ITERS {
            let m = rng.next_md5();
            let c = call_poisoned(Impl::C, &m, poison);
            let r = call_poisoned(Impl::Rust, &m, poison);
            assert_eq!(c, r, "[row16] poison {poison:#04x} divergence for {m:?}");
            // Cross-check against the other poison value: if a byte were left
            // unwritten the two results would differ.
            let other = !poison;
            assert_eq!(
                c,
                call_poisoned(Impl::C, &m, other),
                "[row16] C left a byte unwritten"
            );
            assert_eq!(
                r,
                call_poisoned(Impl::Rust, &m, other),
                "[row16] Rust left a byte unwritten"
            );
        }
    }
}

/// Row 17 — `out` aliases the struct storage itself, fully and PARTIALLY. The C
/// has no `restrict` and no aliasing check, and `tflac_u8*` is exempt from
/// strict aliasing, so each store can feed the next load; whatever cascade the
/// C's read/store interleaving produces is the ground truth.
///
/// Full aliasing (`out == m`) is a fixed point and therefore proves nothing on
/// its own — the partial-overlap sub-case below is what actually pins the
/// per-byte reload behavior.
#[test]
fn row17_out_aliases_m() {
    let cf = digest(Impl::C);
    let rf = digest(Impl::Rust);
    let mut rng = Rng::new(0x5EED_0017);

    // 17a — exact aliasing: out == (u8*)m.
    for _ in 0..ITERS {
        let img = rng.next_image();
        let mut mc = Md5::from_image(&img);
        let mut mr = Md5::from_image(&img);
        unsafe {
            cf(&mc as *const Md5, (&mut mc as *mut Md5).cast::<u8>());
            rf(&mr as *const Md5, (&mut mr as *mut Md5).cast::<u8>());
        }
        let ic = unsafe { *(&mc as *const Md5).cast::<[u8; 16]>() };
        let ir = unsafe { *(&mr as *const Md5).cast::<[u8; 16]>() };
        assert_eq!(
            ic, ir,
            "[row17a] aliased in/out divergence for image {img:02x?}\n  C   : {ic:02x?}\n  Rust: {ir:02x?}"
        );
    }

    // 17b — partial overlap: out starts 1..=15 bytes into the struct image, so
    // stores overwrite words that have not been read yet.
    for shift in 1..16usize {
        for _ in 0..256 {
            let img = rng.next_image();
            let mut ac = [0u8; 48];
            let mut ar = [0u8; 48];
            ac[..16].copy_from_slice(&img);
            ar[..16].copy_from_slice(&img);
            unsafe {
                cf(ac.as_ptr().cast::<Md5>(), ac.as_mut_ptr().add(shift));
                rf(ar.as_ptr().cast::<Md5>(), ar.as_mut_ptr().add(shift));
            }
            assert_eq!(
                ac, ar,
                "[row17b] partial overlap +{shift} divergence for {img:02x?}\n  C   : {ac:02x?}\n  Rust: {ar:02x?}"
            );
        }
    }

    // 17c — reverse partial overlap: the struct sits AFTER the output window,
    // so stores trail behind the reads.
    for shift in 1..16usize {
        for _ in 0..256 {
            let img = rng.next_image();
            let mut ac = [0u8; 48];
            let mut ar = [0u8; 48];
            ac[shift..shift + 16].copy_from_slice(&img);
            ar[shift..shift + 16].copy_from_slice(&img);
            unsafe {
                cf(ac.as_ptr().add(shift).cast::<Md5>(), ac.as_mut_ptr());
                rf(ar.as_ptr().add(shift).cast::<Md5>(), ar.as_mut_ptr());
            }
            assert_eq!(
                ac, ar,
                "[row17c] reverse overlap -{shift} divergence for {img:02x?}\n  C   : {ac:02x?}\n  Rust: {ar:02x?}"
            );
        }
    }
}

/// Row 18 — `m` reached through a misaligned pointer.
#[test]
fn row18_misaligned_m() {
    let cf = digest(Impl::C);
    let rf = digest(Impl::Rust);
    let mut rng = Rng::new(0x5EED_0018);
    for _ in 0..ITERS {
        let img = rng.next_image();
        for off in [1usize, 2, 3, 5, 6, 7] {
            let mut arena = [0u8; 32];
            arena[off..off + 16].copy_from_slice(&img);
            let mp = unsafe { arena.as_ptr().add(off) }.cast::<Md5>();
            let mut oc = [0xAAu8; 16];
            let mut or = [0xAAu8; 16];
            unsafe {
                cf(mp, oc.as_mut_ptr());
                rf(mp, or.as_mut_ptr());
            }
            assert_eq!(oc, or, "[row18] misaligned m (off {off}) divergence for {img:02x?}");
            assert_eq!(oc, img, "[row18] C misaligned read wrong");
        }
    }
}
