//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Every call goes through `dlopen`ed exports of BOTH the C `.so` and the Rust `.so`.

mod common;

use common::*;
use std::ffi::c_int;

/// Encodes a header byte 1 for sync class A (`0xF0..0xFF`).
fn b1_class_a(version_bits: u8, layer: u8, crc: u8) -> u8 {
    0xF0 | ((version_bits & 3) << 2) | ((layer & 3) << 1) | (crc & 1)
}

/// Encodes header byte 2 from (bitrate index, samplerate index, pad/priv bits).
fn b2(bitrate: u8, samplerate: u8, low: u8) -> u8 {
    ((bitrate & 0x0F) << 4) | ((samplerate & 3) << 2) | (low & 3)
}

fn sweep_identical(layer: u8, samplerate: u8, rng: &mut Rng) {
    for version_bits in 0..4u8 {
        for crc in 0..2u8 {
            for bitrate in 1..15u8 {
                for low in 0..4u8 {
                    let h: [u8; 3] = [0xFF, b1_class_a(version_bits, layer, crc), b2(bitrate, samplerate, low)];
                    let got = diff3(&h, &h);
                    assert_eq!(got, model(&h, &h), "model mismatch for {h:02X?}");
                    assert_eq!(got, 1, "a valid header must match itself: {h:02X?}");
                    // plus randomized byte-0 noise on h1 (never read)
                    let h1 = [rng.next_u8(), h[1], h[2]];
                    assert_eq!(diff3(&h1, &h), 1);
                }
            }
        }
    }
}

#[test]
fn c1_class_a_layer1_norm_sr0_identical() {
    sweep_identical(1, 0, &mut Rng::seeded());
}

#[test]
fn c2_class_a_layer2_norm_sr1_identical() {
    sweep_identical(2, 1, &mut Rng::new(2));
}

#[test]
fn c3_class_a_layer3_norm_sr2_identical() {
    sweep_identical(3, 2, &mut Rng::new(3));
}

fn class_b_identical(b1: u8) {
    for samplerate in 0..3u8 {
        for bitrate in 1..15u8 {
            for low in 0..4u8 {
                let h: [u8; 3] = [0xFF, b1, b2(bitrate, samplerate, low)];
                let got = diff3(&h, &h);
                assert_eq!(got, model(&h, &h));
                assert_eq!(got, 1, "class-B header must match itself: {h:02X?}");
            }
        }
    }
}

#[test]
fn c4_class_b_e2_identical() {
    class_b_identical(0xE2);
}

#[test]
fn c5_class_b_e3_identical() {
    class_b_identical(0xE3);
}

#[test]
fn c6_both_free_format() {
    let mut rng = Rng::new(6);
    for &b1v in valid_byte1().iter() {
        for sr in 0..3u8 {
            for low1 in 0..4u8 {
                for low2 in 0..4u8 {
                    let h2 = [0xFF, b1v, b2(0, sr, low2)];
                    let h1 = [rng.next_u8(), b1v, b2(0, sr, low1)];
                    let got = diff3(&h1, &h2);
                    assert_eq!(got, model(&h1, &h2));
                    assert_eq!(got, 1, "both free-format must match: {h1:02X?} {h2:02X?}");
                }
            }
        }
    }
}

#[test]
fn c7_both_non_free_different_indices() {
    let mut rng = Rng::new(7);
    for &b1v in valid_byte1().iter() {
        for sr in 0..3u8 {
            for br1 in 1..15u8 {
                for br2 in 1..15u8 {
                    let h2 = [0xFF, b1v, b2(br2, sr, rng.next_u8() & 3)];
                    let h1 = [rng.next_u8(), b1v, b2(br1, sr, rng.next_u8() & 3)];
                    let got = diff3(&h1, &h2);
                    assert_eq!(got, model(&h1, &h2));
                    assert_eq!(
                        got, 1,
                        "different non-free bitrates must still match: {h1:02X?} {h2:02X?}"
                    );
                }
            }
        }
    }
}

#[test]
fn c8_one_free_h2() {
    for &b1v in valid_byte1().iter() {
        for sr in 0..3u8 {
            for br1 in 1..15u8 {
                let h2 = [0xFF, b1v, b2(0, sr, 0)];
                let h1 = [0x00, b1v, b2(br1, sr, 0)];
                let got = diff3(&h1, &h2);
                assert_eq!(got, model(&h1, &h2));
                assert_eq!(got, 0, "free vs non-free must be rejected: {h1:02X?} {h2:02X?}");
            }
        }
    }
}

#[test]
fn c9_one_free_h1() {
    for &b1v in valid_byte1().iter() {
        for sr in 0..3u8 {
            for br2 in 1..15u8 {
                let h2 = [0xFF, b1v, b2(br2, sr, 0)];
                let h1 = [0x00, b1v, b2(0, sr, 0)];
                let got = diff3(&h1, &h2);
                assert_eq!(got, model(&h1, &h2));
                assert_eq!(got, 0, "non-free vs free must be rejected: {h1:02X?} {h2:02X?}");
            }
        }
    }
}

#[test]
fn c10_crc_bit_ignored() {
    for &b1v in valid_byte1().iter() {
        for &b2v in valid_byte2().iter() {
            let h2 = [0xFF, b1v, b2v];
            let h1 = [0x00, b1v ^ 0x01, b2v];
            let got = diff3(&h1, &h2);
            assert_eq!(got, model(&h1, &h2));
            // Flipping bit 0 of byte 1 is masked out by 0xFE -> still a match.
            assert_eq!(got, 1, "CRC bit must be ignored: {h1:02X?} {h2:02X?}");
        }
    }
}

#[test]
fn c11_padding_private_bits_ignored() {
    for &b1v in valid_byte1().iter() {
        for br in 0..15u8 {
            for sr in 0..3u8 {
                for low1 in 0..4u8 {
                    for low2 in 0..4u8 {
                        let h2 = [0xFF, b1v, b2(br, sr, low2)];
                        let h1 = [0x00, b1v, b2(br, sr, low1)];
                        let got = diff3(&h1, &h2);
                        assert_eq!(got, model(&h1, &h2));
                        assert_eq!(got, 1, "pad/private bits must be ignored: {h1:02X?} {h2:02X?}");
                    }
                }
            }
        }
    }
}

#[test]
fn c12_byte1_single_bit_flips() {
    for &b1v in valid_byte1().iter() {
        for &b2v in valid_byte2().iter() {
            for bit in 1..8u8 {
                let h2 = [0xFF, b1v, b2v];
                let h1 = [0x00, b1v ^ (1 << bit), b2v];
                let got = diff3(&h1, &h2);
                assert_eq!(got, model(&h1, &h2));
                assert_eq!(got, 0, "byte-1 bit {bit} flip must reject: {h1:02X?} {h2:02X?}");
            }
        }
    }
}

#[test]
fn c13_byte2_single_bit_flips() {
    for &b1v in valid_byte1().iter() {
        for &b2v in valid_byte2().iter() {
            for bit in 0..8u8 {
                let h2 = [0xFF, b1v, b2v];
                let h1 = [0x00, b1v, b2v ^ (1 << bit)];
                let got = diff3(&h1, &h2);
                assert_eq!(got, model(&h1, &h2), "{h1:02X?} {h2:02X?}");
            }
        }
    }
}

#[test]
fn c14_h1_byte0_never_read() {
    for &b1v in valid_byte1().iter() {
        for &b2v in valid_byte2().iter() {
            let h2 = [0xFF, b1v, b2v];
            let mut first = None;
            for v in 0..=255u8 {
                let h1 = [v, b1v, b2v];
                let got = diff3(&h1, &h2);
                assert_eq!(got, model(&h1, &h2));
                match first {
                    None => first = Some(got),
                    Some(f) => assert_eq!(f, got, "h1[0] = {v:#04X} changed the result"),
                }
            }
        }
    }
}

#[test]
fn c15_h2_byte0_sweep() {
    for &b1v in valid_byte1().iter() {
        for &b2v in valid_byte2().iter() {
            for v in 0..=255u8 {
                let h2 = [v, b1v, b2v];
                let h1 = [0x00, b1v, b2v];
                let got = diff3(&h1, &h2);
                assert_eq!(got, model(&h1, &h2));
                assert_eq!(got, (v == 0xFF) as c_int, "h2[0] = {v:#04X}");
            }
        }
    }
}

#[test]
fn c16_aliased_same_pointer_exhaustive() {
    // All 2^24 headers, h1 == h2 (same pointer). Result must equal hdr_valid(h2).
    let l = libs();
    let mut buf = [0u8; 3];
    for v in (0..(1u32 << 24)).step_by(stride()) {
        buf[0] = v as u8;
        buf[1] = (v >> 8) as u8;
        buf[2] = (v >> 16) as u8;
        let p = buf.as_ptr();
        let a = unsafe { (l.c)(p, p) };
        let b = unsafe { (l.rs)(p, p) };
        if a != b {
            panic!("DIVERGENCE aliased {buf:02X?}: C = {a}, Rust = {b}");
        }
        debug_assert!(a == 0 || a == 1);
        let m = model(&buf, &buf);
        if a != m {
            panic!("model mismatch aliased {buf:02X?}: so = {a}, model = {m}");
        }
    }
}

#[test]
fn c17_overlapping_views() {
    let l = libs();
    let mut rng = Rng::new(17);
    for _ in 0..iters(200_000) {
        let mut buf = [0u8; 8];
        for b in buf.iter_mut() {
            *b = rng.next_u8();
        }
        // Bias toward interesting content.
        if rng.next_u64() & 1 == 0 {
            buf[0] = 0xFF;
            buf[1] = rng.pick(&valid_byte1());
            buf[2] = rng.pick(&valid_byte2());
        }
        for (o1, o2) in [(0usize, 1usize), (1, 0), (0, 0), (2, 1), (1, 2), (3, 3)] {
            let p1 = unsafe { buf.as_ptr().add(o1) };
            let p2 = unsafe { buf.as_ptr().add(o2) };
            let got = unsafe { diff_ptr(l, p1, p2, || format!("buf={buf:02X?} o1={o1} o2={o2}")) };
            let a1 = [buf[o1], buf[o1 + 1], buf[o1 + 2]];
            let a2 = [buf[o2], buf[o2 + 1], buf[o2 + 2]];
            assert_eq!(got, model(&a1, &a2));
        }
    }
}

#[test]
fn c18_page_end_guarded_buffers() {
    // Byte index 2 is the last readable byte; a read at index >= 3 would fault.
    let g1 = GuardedBuf::new();
    let g2 = GuardedBuf::new();
    let l = libs();
    let mut rng = Rng::new(18);

    let vb1 = valid_byte1();
    let vb2 = valid_byte2();

    for i in 0..iters(100_000) as u32 {
        let (h1, h2) = if i % 3 == 0 {
            let h2 = [0xFF, rng.pick(&vb1), rng.pick(&vb2)];
            ([rng.next_u8(), h2[1], h2[2]], h2)
        } else if i % 3 == 1 {
            ([0xFF, rng.pick(&vb1), rng.pick(&vb2)], [0xFF, rng.pick(&vb1), rng.pick(&vb2)])
        } else {
            (rng.bytes3(), rng.bytes3())
        };
        let p1 = g1.put_tail(&h1);
        let p2 = g2.put_tail(&h2);
        let got = unsafe { diff_ptr(l, p1, p2, || format!("guarded h1={h1:02X?} h2={h2:02X?}")) };
        assert_eq!(got, model(&h1, &h2));
        // Same inputs on the heap must give the same answer.
        assert_eq!(got, diff3(&h1, &h2));
    }
}

#[test]
fn c19_unaligned_offsets() {
    let l = libs();
    let mut rng = Rng::new(19);
    let mut buf = [0u8; 32];
    for _ in 0..iters(100_000) {
        for b in buf.iter_mut() {
            *b = rng.next_u8();
        }
        buf[8] = 0xFF;
        buf[9] = rng.pick(&valid_byte1());
        buf[10] = rng.pick(&valid_byte2());
        for o1 in 0..9usize {
            for o2 in 0..9usize {
                let p1 = unsafe { buf.as_ptr().add(o1) };
                let p2 = unsafe { buf.as_ptr().add(16 + o2) };
                // Mirror h2 content at 16+o2 so it is often valid.
                let got = unsafe {
                    diff_ptr(l, p1, p2, || format!("buf={buf:02X?} o1={o1} o2={o2}"))
                };
                let a1 = [buf[o1], buf[o1 + 1], buf[o1 + 2]];
                let a2 = [buf[16 + o2], buf[16 + o2 + 1], buf[16 + o2 + 2]];
                assert_eq!(got, model(&a1, &a2));
            }
        }
    }
}

#[test]
fn c21_h2_exhaustive_vs_h1_battery() {
    let l = libs();
    let mut h2 = [0u8; 3];
    for h1 in H1_BATTERY.iter() {
        for v in (0..(1u32 << 24)).step_by(stride()) {
            h2[0] = v as u8;
            h2[1] = (v >> 8) as u8;
            h2[2] = (v >> 16) as u8;
            let a = unsafe { (l.c)(h1.as_ptr(), h2.as_ptr()) };
            let b = unsafe { (l.rs)(h1.as_ptr(), h2.as_ptr()) };
            if a != b {
                panic!("DIVERGENCE h1={h1:02X?} h2={h2:02X?}: C = {a}, Rust = {b}");
            }
            let m = model(h1, &h2);
            if a != m {
                panic!("model mismatch h1={h1:02X?} h2={h2:02X?}: so = {a}, model = {m}");
            }
        }
    }
}

fn random_campaign(seed: u64, n: u64, gen: impl Fn(&mut Rng) -> ([u8; 3], [u8; 3])) {
    let l = libs();
    let mut rng = Rng::new(seed);
    for _ in 0..iters(n) {
        let (h1, h2) = gen(&mut rng);
        let a = unsafe { (l.c)(h1.as_ptr(), h2.as_ptr()) };
        let b = unsafe { (l.rs)(h1.as_ptr(), h2.as_ptr()) };
        if a != b {
            panic!("DIVERGENCE h1={h1:02X?} h2={h2:02X?}: C = {a}, Rust = {b}");
        }
        assert!(a == 0 || a == 1, "non-boolean result {a} for {h1:02X?} {h2:02X?}");
        let m = model(&h1, &h2);
        if a != m {
            panic!("model mismatch h1={h1:02X?} h2={h2:02X?}: so = {a}, model = {m}");
        }
        // Argument order is NOT symmetric in the C: check the swapped order too (row C30).
        let a2 = unsafe { (l.c)(h2.as_ptr(), h1.as_ptr()) };
        let b2 = unsafe { (l.rs)(h2.as_ptr(), h1.as_ptr()) };
        if a2 != b2 {
            panic!("DIVERGENCE (swapped) h1={h2:02X?} h2={h1:02X?}: C = {a2}, Rust = {b2}");
        }
        assert_eq!(a2, model(&h2, &h1));
    }
}

#[test]
fn c23_random_uniform() {
    random_campaign(23, 2_000_000, |rng| (rng.bytes3(), rng.bytes3()));
}

#[test]
fn c24_random_valid_h2_random_h1() {
    let vb1 = valid_byte1();
    let vb2 = valid_byte2();
    random_campaign(24, 2_000_000, move |rng| {
        let h2 = [0xFF, rng.pick(&vb1), rng.pick(&vb2)];
        (rng.bytes3(), h2)
    });
}

#[test]
fn c25_random_near_match_mutations() {
    let vb1 = valid_byte1();
    let vb2 = valid_byte2();
    random_campaign(25, 2_000_000, move |rng| {
        let h2 = if rng.next_u64() & 3 == 0 {
            rng.bytes3()
        } else {
            [0xFF, rng.pick(&vb1), rng.pick(&vb2)]
        };
        let mut h1 = h2;
        let flips = 1 + rng.below(3);
        for _ in 0..flips {
            let bit = rng.below(24) as u32;
            h1[(bit / 8) as usize] ^= 1u8 << (bit % 8);
        }
        (h1, h2)
    });
}

#[test]
fn c26_random_boundary_alphabet() {
    random_campaign(26, 2_000_000, |rng| {
        let pick = |r: &mut Rng| r.pick(&BOUNDARY_BYTES);
        (
            [pick(rng), pick(rng), pick(rng)],
            [pick(rng), pick(rng), pick(rng)],
        )
    });
}

#[test]
fn c27_layer_bitrate_samplerate_cross() {
    // All layers (incl. reserved 0) x all bitrate indices (incl. 15) x all samplerate
    // indices (incl. reserved 3) x both sync classes, self-comparison.
    let mut b1s: Vec<u8> = (0xF0u8..=0xFF).collect();
    b1s.extend_from_slice(&[0xE0, 0xE1, 0xE2, 0xE3, 0xE4, 0x7F, 0x00]);
    for &b1v in &b1s {
        for br in 0..16u8 {
            for sr in 0..4u8 {
                for low in 0..4u8 {
                    let h = [0xFF, b1v, b2(br, sr, low)];
                    let got = diff3(&h, &h);
                    assert_eq!(got, model(&h, &h), "{h:02X?}");
                }
            }
        }
    }
}

#[test]
fn c28_two_header_field_tuple_cross() {
    let mut b1s: Vec<u8> = (0xF0u8..=0xFF).collect();
    b1s.extend_from_slice(&[0xE0, 0xE1, 0xE2, 0xE3, 0xE4]);
    let b2s: Vec<u8> = (0..16u8)
        .flat_map(|br| (0..4u8).map(move |sr| b2(br, sr, 0)))
        .collect();
    for &a1 in &b1s {
        for &c1 in &b1s {
            for &a2 in &b2s {
                for &c2 in &b2s {
                    let h1 = [0x00, a1, a2];
                    let h2 = [0xFF, c1, c2];
                    let got = diff3(&h1, &h2);
                    assert_eq!(got, model(&h1, &h2), "{h1:02X?} {h2:02X?}");
                }
            }
        }
    }
}

#[test]
fn c29_realworld_header_matrix() {
    for h1 in REALWORLD.iter() {
        for h2 in REALWORLD.iter() {
            let got = diff3(h1, h2);
            assert_eq!(got, model(h1, h2), "{h1:02X?} {h2:02X?}");
        }
    }
    // Self-comparison of every real header must be a match.
    for h in REALWORLD.iter() {
        assert_eq!(diff3(h, h), 1, "{h:02X?} should match itself");
    }
}

#[test]
fn c30_argument_order_asymmetry() {
    // Explicit asymmetry check on the real-world matrix plus a randomized sweep. The C's
    // validity test only looks at h2, so the two orders genuinely differ.
    let mut asymmetric_seen = false;
    for h1 in REALWORLD.iter() {
        for h2 in REALWORLD.iter() {
            let ab = diff3(h1, h2);
            let ba = diff3(h2, h1);
            assert_eq!(ab, model(h1, h2));
            assert_eq!(ba, model(h2, h1));
        }
    }
    let mut rng = Rng::new(30);
    let vb1 = valid_byte1();
    let vb2 = valid_byte2();
    for i in 0..iters(500_000) as u32 {
        // Mix of fully random pairs and near-match pairs (where only one side is a valid
        // reference header) — the latter is where the C's one-sided validity check bites.
        let (h1, h2) = match i % 4 {
            0 => (rng.bytes3(), rng.bytes3()),
            1 => {
                let h2 = [0xFF, rng.pick(&vb1), rng.pick(&vb2)];
                ([rng.next_u8(), h2[1], h2[2]], h2)
            }
            2 => {
                let h2 = [0xFF, rng.pick(&vb1), rng.pick(&vb2)];
                let mut h1 = h2;
                let bit = rng.below(24) as u32;
                h1[(bit / 8) as usize] ^= 1u8 << (bit % 8);
                (h1, h2)
            }
            _ => {
                let h2 = [0xFF, rng.next_u8(), rng.next_u8()];
                ([0xFF, rng.next_u8(), rng.next_u8()], h2)
            }
        };
        let ab = diff3(&h1, &h2);
        let ba = diff3(&h2, &h1);
        assert_eq!(ab, model(&h1, &h2));
        assert_eq!(ba, model(&h2, &h1));
        if ab != ba {
            asymmetric_seen = true;
        }
    }
    assert!(
        asymmetric_seen,
        "expected to observe at least one asymmetric pair, the campaign is not exercising it"
    );
}

#[test]
fn c31_return_exactly_0_or_1() {
    let l = libs();
    let mut rng = Rng::new(31);
    let mut saw_zero = false;
    let mut saw_one = false;
    for _ in 0..iters(1_000_000) {
        let h1 = rng.bytes3();
        let h2 = if rng.next_u64() & 1 == 0 {
            [0xFF, rng.pick(&valid_byte1()), rng.pick(&valid_byte2())]
        } else {
            rng.bytes3()
        };
        let a = unsafe { (l.c)(h1.as_ptr(), h2.as_ptr()) };
        let b = unsafe { (l.rs)(h1.as_ptr(), h2.as_ptr()) };
        assert_eq!(a, b, "h1={h1:02X?} h2={h2:02X?}");
        assert!(a == 0 || a == 1, "C returned {a}, not a C boolean");
        assert!(b == 0 || b == 1, "Rust returned {b}, not a C boolean");
        saw_zero |= a == 0;
        saw_one |= a == 1;
    }
    assert!(saw_zero && saw_one, "campaign must observe both outcomes");
}

#[test]
fn c32_no_hidden_state_interleaved() {
    // Alternate C / Rust / C on the same dlopen handles; a stateful divergence would show
    // up as a mismatch between the first and the second C call.
    let l = libs();
    let mut rng = Rng::new(32);
    for _ in 0..iters(500_000) {
        let h1 = rng.bytes3();
        let h2 = [0xFF, rng.pick(&valid_byte1()), rng.pick(&valid_byte2())];
        let c1 = unsafe { (l.c)(h1.as_ptr(), h2.as_ptr()) };
        let r1 = unsafe { (l.rs)(h1.as_ptr(), h2.as_ptr()) };
        let c2 = unsafe { (l.c)(h1.as_ptr(), h2.as_ptr()) };
        let r2 = unsafe { (l.rs)(h1.as_ptr(), h2.as_ptr()) };
        assert_eq!((c1, r1), (c2, r2), "hidden state for {h1:02X?} {h2:02X?}");
        assert_eq!(c1, r1, "h1={h1:02X?} h2={h2:02X?}");
    }
}

#[test]
fn c33_h1_byte0_invariance_randomized() {
    let l = libs();
    let mut rng = Rng::new(33);
    for _ in 0..iters(4096) {
        let h2 = if rng.next_u64() & 1 == 0 {
            [0xFF, rng.pick(&valid_byte1()), rng.pick(&valid_byte2())]
        } else {
            rng.bytes3()
        };
        let (b1v, b2v) = (rng.next_u8(), rng.next_u8());
        let mut baseline: Option<c_int> = None;
        for v in 0..=255u8 {
            let h1 = [v, b1v, b2v];
            let a = unsafe { (l.c)(h1.as_ptr(), h2.as_ptr()) };
            let b = unsafe { (l.rs)(h1.as_ptr(), h2.as_ptr()) };
            assert_eq!(a, b, "h1={h1:02X?} h2={h2:02X?}");
            match baseline {
                None => baseline = Some(a),
                Some(f) => assert_eq!(f, a, "h1[0]={v:#04X} changed the result for h2={h2:02X?}"),
            }
        }
    }
}
