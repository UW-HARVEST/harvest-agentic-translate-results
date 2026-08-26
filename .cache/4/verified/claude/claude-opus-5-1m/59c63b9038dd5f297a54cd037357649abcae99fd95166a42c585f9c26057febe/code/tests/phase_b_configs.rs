//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row
//! group. Every case calls BOTH the C `.so` and the Rust `.so` through
//! `libloading` and compares all 24 struct bytes.

mod common;

use common::*;

/// Sanity: the struct layout the tests use must match the C compiler's.
/// (`sizeof=24 align=4`, `frame_header` at offset 16.)
#[test]
fn layout_matches_c() {
    assert_eq!(std::mem::size_of::<Tflac>(), 24, "sizeof(struct tflac)");
    assert_eq!(std::mem::align_of::<Tflac>(), 4, "alignof(struct tflac)");
    let t = Tflac::default();
    let base = &t as *const Tflac as usize;
    assert_eq!(&t.samplerate as *const u32 as usize - base, 0);
    assert_eq!(&t.channels as *const u32 as usize - base, 4);
    assert_eq!(&t.bitdepth as *const u32 as usize - base, 8);
    assert_eq!(&t.channel_mode as *const u8 as usize - base, 12);
    assert_eq!(&t.frame_header as *const u32 as usize - base, 16);
    assert_eq!(&t.cur_blocksize as *const u32 as usize - base, 20);
}

/// Both shared objects load and export the symbol.
#[test]
fn both_libraries_export_the_symbol() {
    let d = Diff::load();
    assert!(!(d.c as usize == 0), "C symbol resolved to null");
    assert!(!(d.rust as usize == 0), "Rust symbol resolved to null");
    eprintln!("C   .so: {}", c_so_path().display());
    eprintln!("Rust.so: {}", rust_so_path().display());
}

// ---------------------------------------------------------------------------
// R1..R15 — `cur_blocksize` classes B1..B15
// ---------------------------------------------------------------------------

#[test]
fn cfg_blocksize_classes() {
    let mut d = Diff::load();
    let mut rng = Rng::new(0xB10C_5123);

    // B1..B13: the 13 explicit `case` literals, each with 2000 randomized
    // values for every other field.
    for &bs in BS_LITERALS.iter() {
        for _ in 0..2000 {
            let mut t = rng.tflac();
            t.cur_blocksize = bs;
            d.check(&format!("B literal {bs}"), t);
        }
    }

    // B14: `default:` with `cur_blocksize <= 256` -- exhaustive over 0..=256.
    for bs in 0u32..=256 {
        for _ in 0..8 {
            let mut t = rng.tflac();
            t.cur_blocksize = bs;
            d.check(&format!("B14 bs={bs}"), t);
        }
    }

    // B15: `default:` with `cur_blocksize > 256`.
    let gt: [u32; 10] = [
        257, 258, 575, 577, 1151, 65535, 65536, 0x7FFF_FFFF, 0x8000_0000, 0xFFFF_FFFF,
    ];
    for &bs in gt.iter() {
        for _ in 0..2000 {
            let mut t = rng.tflac();
            t.cur_blocksize = bs;
            d.check(&format!("B15 bs={bs}"), t);
        }
    }
    for _ in 0..20000 {
        let mut t = rng.tflac();
        t.cur_blocksize = 257 + rng.below(0xFFFF_FFFE - 257);
        d.check("B15 random >256", t);
    }

    d.finish("R1..R15 blocksize classes");
}

// ---------------------------------------------------------------------------
// R16..R32 — `samplerate` classes S1..S17
// ---------------------------------------------------------------------------

#[test]
fn cfg_samplerate_classes() {
    let mut d = Diff::load();
    let mut rng = Rng::new(0x5A47_E001);

    // S1..S11: the 11 explicit `case` literals.
    for &sr in SR_LITERALS.iter() {
        for _ in 0..2000 {
            let mut t = rng.tflac();
            t.samplerate = sr;
            d.check(&format!("S literal {sr}"), t);
        }
    }

    // S12: %1000 == 0 and /1000 < 256 -> 0x0C. Exhaustive over k = 0..=255.
    for k in 0u32..=255 {
        let sr = k * 1000;
        if SR_LITERALS.contains(&sr) {
            continue;
        }
        for _ in 0..20 {
            let mut t = rng.tflac();
            t.samplerate = sr;
            d.check(&format!("S12 sr={sr}"), t);
        }
    }

    // S13: %1000 == 0 and /1000 >= 256 -> no bits.
    for _ in 0..20000 {
        let k = 256 + rng.below(4_294_966 - 256); // k*1000 must not overflow u32
        let sr = k * 1000;
        let mut t = rng.tflac();
        t.samplerate = sr;
        d.check(&format!("S13 sr={sr}"), t);
    }
    for &sr in [256000u32, 257000, 300000, 1_000_000, 4_294_000_000, 4_294_967_000].iter() {
        for _ in 0..500 {
            let mut t = rng.tflac();
            t.samplerate = sr;
            d.check(&format!("S13 fixed sr={sr}"), t);
        }
    }

    // S14: %1000 != 0 and < 65536 -> 0x0D.
    for _ in 0..40000 {
        let sr = rng.below(65536);
        if sr % 1000 == 0 || SR_LITERALS.contains(&sr) {
            continue;
        }
        let mut t = rng.tflac();
        t.samplerate = sr;
        d.check(&format!("S14 sr={sr}"), t);
    }
    for &sr in [1u32, 2, 999, 1001, 22051, 44101, 65534, 65535].iter() {
        for _ in 0..500 {
            let mut t = rng.tflac();
            t.samplerate = sr;
            d.check(&format!("S14 fixed sr={sr}"), t);
        }
    }

    // S15: %1000 != 0, >= 65536, %10 == 0, /10 < 65536 -> 0x0E.
    for _ in 0..20000 {
        let sr = (6554 + rng.below(65536 - 6554)) * 10; // 65540 ..= 655350
        if sr % 1000 == 0 || sr < 65536 {
            continue;
        }
        let mut t = rng.tflac();
        t.samplerate = sr;
        d.check(&format!("S15 sr={sr}"), t);
    }
    for &sr in [65540u32, 88200, 176410, 655340, 655350].iter() {
        for _ in 0..500 {
            let mut t = rng.tflac();
            t.samplerate = sr;
            d.check(&format!("S15 fixed sr={sr}"), t);
        }
    }

    // S16: %1000 != 0, >= 65536, %10 == 0, /10 >= 65536 -> no bits.
    for _ in 0..20000 {
        let sr = (65536 + rng.below(429_496_729 - 65536)) * 10;
        if sr % 1000 == 0 {
            continue;
        }
        let mut t = rng.tflac();
        t.samplerate = sr;
        d.check(&format!("S16 sr={sr}"), t);
    }
    for &sr in [655360u32, 655370, 4_294_967_290, 1_234_567_890].iter() {
        for _ in 0..500 {
            let mut t = rng.tflac();
            t.samplerate = sr;
            d.check(&format!("S16 fixed sr={sr}"), t);
        }
    }

    // S17: %1000 != 0, >= 65536, %10 != 0 -> no bits.
    for _ in 0..40000 {
        let sr = 65536 + rng.below(0xFFFF_FFFF - 65536);
        if sr % 10 == 0 {
            continue;
        }
        let mut t = rng.tflac();
        t.samplerate = sr;
        d.check(&format!("S17 sr={sr}"), t);
    }
    for &sr in [65537u32, 65539, 96001, 0x7FFF_FFFF, 0xFFFF_FFFF].iter() {
        for _ in 0..500 {
            let mut t = rng.tflac();
            t.samplerate = sr;
            d.check(&format!("S17 fixed sr={sr}"), t);
        }
    }

    d.finish("R16..R32 samplerate classes");
}

// ---------------------------------------------------------------------------
// R33..R41 — channel classes C1..C8
// ---------------------------------------------------------------------------

#[test]
fn cfg_channel_classes() {
    let mut d = Diff::load();
    let mut rng = Rng::new(0xC4A1_9E15);

    // C1..C3 + R36: INDEPENDENT with the legal FLAC counts 1..=8.
    for ch in 1u32..=8 {
        for _ in 0..3000 {
            let mut t = rng.tflac();
            t.channels = ch;
            // any raw u8 whose `% 4` is 0 (INDEPENDENT after the C's fold)
            t.channel_mode = rng.next_u8() & 0xFC;
            d.check(&format!("C1..C3 channels={ch}"), t);
        }
    }

    // C4: channels == 0 -> unsigned underflow.
    for _ in 0..5000 {
        let mut t = rng.tflac();
        t.channels = 0;
        t.channel_mode = rng.next_u8() & 0xFC;
        d.check("C4 channels=0", t);
    }

    // C5: channels > 9 -> overflows the 4-bit channel-assignment field.
    let big: [u32; 12] = [
        9, 10, 15, 16, 17, 255, 256, 4096, 0x0FFF_FFFF, 0x1000_0000, 0x1000_0001, 0xFFFF_FFFF,
    ];
    for &ch in big.iter() {
        for _ in 0..2000 {
            let mut t = rng.tflac();
            t.channels = ch;
            t.channel_mode = rng.next_u8() & 0xFC;
            d.check(&format!("C5 channels={ch}"), t);
        }
    }
    for _ in 0..20000 {
        let mut t = rng.tflac();
        t.channels = 9 + rng.below(0xFFFF_FFF0);
        t.channel_mode = rng.next_u8() & 0xFC;
        d.check("C5 random channels", t);
    }

    // C6..C8: the three joint-stereo modes; `channels` must be ignored, so it
    // is randomized (including 0 and 0xFFFFFFFF) for each.
    for residue in 1u8..=3 {
        for _ in 0..8000 {
            let mut t = rng.tflac();
            // any raw u8 whose `% 4` equals `residue`
            t.channel_mode = (rng.next_u8() & 0xFC).wrapping_add(residue);
            let extra = rng.next_u32();
            let pool = [0u32, 1, 2, 3, 8, 9, 0xFFFF_FFFF, extra];
            t.channels = rng.pick(&pool);
            d.check(&format!("C{} mode%4={residue}", residue + 5), t);
        }
    }

    d.finish("R33..R41 channel classes");
}

// ---------------------------------------------------------------------------
// R42..R47 — `bitdepth` classes D1..D7
// ---------------------------------------------------------------------------

#[test]
fn cfg_bitdepth_classes() {
    let mut d = Diff::load();
    let mut rng = Rng::new(0xB17D_3901);

    // D1..D6: the 6 explicit `case` literals.
    for &bd in BD_LITERALS.iter() {
        for _ in 0..3000 {
            let mut t = rng.tflac();
            t.bitdepth = bd;
            d.check(&format!("D literal {bd}"), t);
        }
    }

    // D7: `default:` -- exhaustive over 0..=256 minus the listed values ...
    for bd in 0u32..=256 {
        if BD_LITERALS.contains(&bd) {
            continue;
        }
        for _ in 0..8 {
            let mut t = rng.tflac();
            t.bitdepth = bd;
            d.check(&format!("D7 bd={bd}"), t);
        }
    }
    // ... plus large values (the `switch` is on u32, so no 8-bit aliasing).
    for &bd in [
        0x100u32, 0x108, 0x110, 33, 63, 64, 1000, 0x7FFF_FFFF, 0x8000_0000, 0xFFFF_FFFF,
    ]
    .iter()
    {
        for _ in 0..1000 {
            let mut t = rng.tflac();
            t.bitdepth = bd;
            d.check(&format!("D7 large bd={bd}"), t);
        }
    }
    for _ in 0..20000 {
        let mut t = rng.tflac();
        t.bitdepth = rng.next_u32();
        d.check("D7 random bd", t);
    }

    d.finish("R42..R47 bitdepth classes");
}

// ---------------------------------------------------------------------------
// R48..R50 — the complete 15 x 17 x 8 x 7 cross-product
// ---------------------------------------------------------------------------

#[test]
fn cfg_full_cross_product() {
    let mut d = Diff::load();
    let mut rng = Rng::new(0xC7055_0048);
    let mut n = 0usize;

    for &bs in BS_CLASS_REPS.iter() {
        for &sr in SR_CLASS_REPS.iter() {
            for &(mode, ch) in CH_CLASS_REPS.iter() {
                for &bd in BD_CLASS_REPS.iter() {
                    let mut t = Tflac::new(sr, ch, bd, mode, bs);
                    // deterministic non-zero padding / prior frame_header
                    t.pad = [rng.next_u8(), rng.next_u8(), rng.next_u8()];
                    t.frame_header = rng.next_u32();
                    d.check(
                        &format!("cross bs={bs} sr={sr} mode={mode} ch={ch} bd={bd}"),
                        t,
                    );
                    n += 1;
                }
            }
        }
    }

    assert_eq!(
        n,
        BS_CLASS_REPS.len() * SR_CLASS_REPS.len() * CH_CLASS_REPS.len() * BD_CLASS_REPS.len(),
        "cross-product size"
    );
    assert_eq!(n, 14280, "expected 15*17*8*7 combinations");
    d.finish("R48..R50 full cross-product");
}

// ---------------------------------------------------------------------------
// R51..R52 — all 256 raw `channel_mode` values
// ---------------------------------------------------------------------------

#[test]
fn cfg_channel_mode_exhaustive_u8() {
    let mut d = Diff::load();
    let mut rng = Rng::new(0x0DE0_5555);

    // R51: every raw u8 mode x every channel-class `channels` value.
    for mode in 0u16..=255 {
        for &(_, ch) in CH_CLASS_REPS.iter() {
            for &bs in [0u32, 192, 4096, 0xFFFF_FFFF].iter() {
                let mut t = Tflac::new(rng.pick(&SR_CLASS_REPS), ch, rng.pick(&BD_CLASS_REPS), mode as u8, bs);
                t.frame_header = rng.next_u32();
                t.pad = [0xAA, 0x55, 0xF0];
                d.check(&format!("R51 mode={mode} ch={ch} bs={bs}"), t);
            }
        }
        // R52: same mode with everything else randomized.
        for _ in 0..200 {
            let mut t = rng.tflac();
            t.channel_mode = mode as u8;
            d.check(&format!("R52 mode={mode}"), t);
        }
    }

    d.finish("R51..R52 channel_mode exhaustive u8");
}

// ---------------------------------------------------------------------------
// R53 — `samplerate` exhaustive 0..=200_000
// ---------------------------------------------------------------------------

#[test]
fn cfg_samplerate_exhaustive_0_200000() {
    let mut d = Diff::load();
    let mut rng = Rng::new(0x5EE0_1234);
    for sr in 0u32..=200_000 {
        let i = sr as usize;
        let (mode, ch) = CH_CLASS_REPS[i % CH_CLASS_REPS.len()];
        let mut t = Tflac::new(
            sr,
            ch,
            BD_CLASS_REPS[i % BD_CLASS_REPS.len()],
            mode,
            BS_CLASS_REPS[i % BS_CLASS_REPS.len()],
        );
        t.frame_header = rng.next_u32();
        t.pad = [rng.next_u8(), rng.next_u8(), rng.next_u8()];
        d.check(&format!("R53 sr={sr}"), t);
    }
    d.finish("R53 samplerate exhaustive 0..=200000");
}

// ---------------------------------------------------------------------------
// R54 — `cur_blocksize` exhaustive 0..=70_000
// ---------------------------------------------------------------------------

#[test]
fn cfg_blocksize_exhaustive_0_70000() {
    let mut d = Diff::load();
    let mut rng = Rng::new(0xB50E_4321);
    for bs in 0u32..=70_000 {
        let i = bs as usize;
        let (mode, ch) = CH_CLASS_REPS[i % CH_CLASS_REPS.len()];
        let mut t = Tflac::new(
            SR_CLASS_REPS[i % SR_CLASS_REPS.len()],
            ch,
            BD_CLASS_REPS[i % BD_CLASS_REPS.len()],
            mode,
            bs,
        );
        t.frame_header = rng.next_u32();
        t.pad = [rng.next_u8(), rng.next_u8(), rng.next_u8()];
        d.check(&format!("R54 bs={bs}"), t);
    }
    d.finish("R54 cur_blocksize exhaustive 0..=70000");
}

// ---------------------------------------------------------------------------
// R55 — `bitdepth` exhaustive 0..=1000
// ---------------------------------------------------------------------------

#[test]
fn cfg_bitdepth_exhaustive_0_1000() {
    let mut d = Diff::load();
    let mut rng = Rng::new(0xBD00_7777);
    for bd in 0u32..=1000 {
        for k in 0..20 {
            let i = bd as usize + k;
            let (mode, ch) = CH_CLASS_REPS[i % CH_CLASS_REPS.len()];
            let mut t = Tflac::new(
                SR_CLASS_REPS[i % SR_CLASS_REPS.len()],
                ch,
                bd,
                mode,
                BS_CLASS_REPS[i % BS_CLASS_REPS.len()],
            );
            t.frame_header = rng.next_u32();
            d.check(&format!("R55 bd={bd}"), t);
        }
    }
    d.finish("R55 bitdepth exhaustive 0..=1000");
}

// ---------------------------------------------------------------------------
// R56 — uniform random fuzz over the whole input space
// ---------------------------------------------------------------------------

#[test]
fn cfg_fuzz_uniform_random() {
    let mut d = Diff::load();
    let mut rng = Rng::new(0xF022_2222);
    for _ in 0..400_000 {
        let t = rng.tflac();
        d.check("R56 uniform fuzz", t);
    }
    d.finish("R56 uniform random fuzz");
}

// ---------------------------------------------------------------------------
// R57 — structured random fuzz: class representatives + boundary jitter
// ---------------------------------------------------------------------------

#[test]
fn cfg_fuzz_structured_random() {
    let mut d = Diff::load();
    let mut rng = Rng::new(0x57A0_9999);

    let jitter: [i64; 9] = [0, 1, -1, 2, -2, 10, -10, 1000, -1000];

    fn jit(rng: &mut Rng, jitter: &[i64], base: u32) -> u32 {
        let j = jitter[(rng.next_u64() % jitter.len() as u64) as usize];
        (base as i64).wrapping_add(j) as u32
    }

    for _ in 0..400_000 {
        let bs_base = rng.pick(&BS_CLASS_REPS);
        let bs = jit(&mut rng, &jitter, bs_base);
        let sr_base = rng.pick(&SR_CLASS_REPS);
        let sr = jit(&mut rng, &jitter, sr_base);
        let bd_base = rng.pick(&BD_CLASS_REPS);
        let bd = jit(&mut rng, &jitter, bd_base);
        let (mode0, ch0) = rng.pick(&CH_CLASS_REPS);
        let mode = mode0.wrapping_add(rng.next_u8() & 0x0F);
        let ch = jit(&mut rng, &jitter, ch0);

        let mut t = Tflac::new(sr, ch, bd, mode, bs);
        t.frame_header = rng.next_u32();
        t.pad = [rng.next_u8(), rng.next_u8(), rng.next_u8()];
        d.check("R57 structured fuzz", t);
    }
    d.finish("R57 structured random fuzz");
}

// ---------------------------------------------------------------------------
// R58 — realistic FLAC encoder configuration matrix
// ---------------------------------------------------------------------------

#[test]
fn cfg_realistic_flac_matrix() {
    let mut d = Diff::load();
    let mut rng = Rng::new(0x4FAC_1111);

    let rates: [u32; 8] = [8000, 16000, 22050, 24000, 44100, 48000, 96000, 192000];
    let counts: [u32; 4] = [1, 2, 6, 8];
    let depths: [u32; 4] = [8, 16, 24, 32];
    let blocks: [u32; 6] = [192, 576, 1152, 4096, 4608, 32768];

    for &sr in rates.iter() {
        for &ch in counts.iter() {
            for &bd in depths.iter() {
                for &bs in blocks.iter() {
                    for mode in 0u8..4 {
                        let mut t = Tflac::new(sr, ch, bd, mode, bs);
                        t.frame_header = rng.next_u32();
                        t.pad = [rng.next_u8(), rng.next_u8(), rng.next_u8()];
                        d.check(&format!("R58 {sr}/{ch}/{bd}/{bs}/{mode}"), t);
                    }
                }
            }
        }
    }
    d.finish("R58 realistic FLAC matrix");
}

// ---------------------------------------------------------------------------
// R59 — repeated invocation on the same struct
// ---------------------------------------------------------------------------

#[test]
fn cfg_repeated_invocation_idempotent() {
    let d = Diff::load();
    let mut rng = Rng::new(0x2EBE_A7ED);
    let mut cases = 0usize;

    for _ in 0..50_000 {
        let input = rng.tflac();
        let mut c_state = input;
        let mut rust_state = input;
        for round in 0..3 {
            unsafe {
                (d.c)(&mut c_state as *mut Tflac);
                (d.rust)(&mut rust_state as *mut Tflac);
            }
            cases += 1;
            if c_state.as_bytes() != rust_state.as_bytes() {
                panic!(
                    "R59 round {round}: C {c_state:?} != Rust {rust_state:?} for input {input:?}"
                );
            }
        }
        // Line 12 is `=`, not `|=`, so the operation is idempotent in C; the
        // Rust must be too.
        let mut once = input;
        unsafe { (d.c)(&mut once as *mut Tflac) };
        assert_eq!(
            once.frame_header, c_state.frame_header,
            "C is expected to be idempotent (line 12 assigns, does not OR)"
        );
    }
    eprintln!("R59 repeated invocation: {cases} cases, 0 mismatches");
}

// ---------------------------------------------------------------------------
// R60 — prior `frame_header` contents must be discarded
// ---------------------------------------------------------------------------

#[test]
fn cfg_prior_frame_header_ignored() {
    let mut d = Diff::load();
    let mut rng = Rng::new(0x9819_0060);

    let priors: [u32; 6] = [
        0x0000_0000,
        0xFFFF_FFFF,
        0xDEAD_BEEF,
        0xAAAA_AAAA,
        0x5555_5555,
        0x0000_0001,
    ];

    for _ in 0..20_000 {
        let base = rng.tflac();
        let mut expected: Option<[u8; 24]> = None;
        for &p in priors.iter() {
            let mut t = base;
            t.frame_header = p;
            let c = d.check_and_get(&format!("R60 prior=0x{p:08X}"), t);
            // Independent of the prior value, apart from `frame_header` itself.
            let mut norm = c.as_bytes();
            norm[16..20].copy_from_slice(&c.frame_header.to_ne_bytes());
            match &expected {
                None => expected = Some(norm),
                Some(e) => assert_eq!(
                    *e, norm,
                    "C result must not depend on the prior frame_header value"
                ),
            }
        }
    }
    d.finish("R60 prior frame_header ignored");
}

// ---------------------------------------------------------------------------
// R61 — padding bytes must survive
// ---------------------------------------------------------------------------

#[test]
fn cfg_padding_preserved() {
    let mut d = Diff::load();
    let mut rng = Rng::new(0xDAD0_0061);

    let pads: [[u8; 3]; 5] = [
        [0x00, 0x00, 0x00],
        [0xFF, 0xFF, 0xFF],
        [0xDE, 0xAD, 0xBE],
        [0x01, 0x02, 0x03],
        [0x80, 0x7F, 0xC3],
    ];
    for _ in 0..20_000 {
        let base = rng.tflac();
        let mut expected: Option<u32> = None;
        for &p in pads.iter() {
            let mut t = base;
            t.pad = p;
            let c = d.check_and_get("R61 padding", t);
            assert_eq!(c.pad, p, "C must not touch the padding bytes");
            // The C reads `channel_mode` as a single `tflac_u8`, so the result
            // must not depend on the 3 padding bytes that follow it.
            match expected {
                None => expected = Some(c.frame_header),
                Some(e) => assert_eq!(
                    e, c.frame_header,
                    "C result must be independent of the padding bytes (pad={p:02X?})"
                ),
            }
        }
    }
    d.finish("R61 padding preserved");
}
