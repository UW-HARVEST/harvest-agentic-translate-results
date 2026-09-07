//! Phase C — error/rejection-path differential tests.
//!
//! `ERRORS.md` records that the C library has **no** rejection path: no error
//! return, no `assert`, no range check, no null check, no pointer parameter and
//! no enum. `tritanopia` is total over its 2^24-element input domain.
//!
//! The rows below are therefore the *generic* boundaries the task requires
//! regardless: minimum/maximum values, one-step-past-range values crossing the
//! FFI, out-of-range "enum"-style words, argument and return padding, and the
//! one genuinely UB-adjacent construct in the C — the out-of-range
//! float -> `unsigned char` conversion. Each asserts the two `.so`s agree
//! *exactly*, not merely that "both did something".

mod common;

use common::{denorm_pre_cast, scan_all_where, Harness, Rgb, Rng};

/// E1 — numeric minimum of every channel.
#[test]
fn err_e1_all_min() {
    let h = Harness::new();
    let x = Rgb::new(0, 0, 0);
    let c = h.c(x);
    let r = h.rust(x);
    assert_eq!(c, r, "E1: C {c:?} vs Rust {r:?}");
    eprintln!("E1 {{0,0,0}} -> {{{},{},{}}} (both)", c.r, c.g, c.b);
}

/// E2 — numeric maximum of every channel.
#[test]
fn err_e2_all_max() {
    let h = Harness::new();
    let x = Rgb::new(255, 255, 255);
    let c = h.c(x);
    let r = h.rust(x);
    assert_eq!(c, r, "E2: C {c:?} vs Rust {r:?}");
    eprintln!("E2 {{255,255,255}} -> {{{},{},{}}} (both)", c.r, c.g, c.b);
}

/// E3 — "one step past the valid range" for an `unsigned char` channel.
///
/// `unsigned char` has no invalid value, so the only way to express
/// out-of-range is to place a wider value in the argument register. The C reads
/// each channel with `movzbl`, so `256` must behave as `0` and `-1` as `255`.
/// Both libraries must agree on that, and on the low-byte-only semantics.
#[test]
fn err_e3_out_of_range_channel_wraps() {
    let h = Harness::new();
    // Channel R given 0x100 (256), 0x1FF (511) and 0xFF (-1 as u8) in a wider slot.
    let cases: [(u32, Rgb); 6] = [
        (0x0000_0100, Rgb::new(0, 1, 0)),  // R=256 -> low byte 0, spills 1 into G
        (0x0001_0000, Rgb::new(0, 0, 1)),  // one step past the G byte
        (0x00FF_FFFF, Rgb::new(255, 255, 255)),
        (0x0000_01FF, Rgb::new(255, 1, 0)),
        (0xFFFF_FFFF, Rgb::new(255, 255, 255)), // -1 everywhere + padding
        (0x0000_0000, Rgb::new(0, 0, 0)),
    ];
    for (word, equivalent) in cases {
        let c = h.c_raw(word) & 0x00FF_FFFF;
        let r = h.rust_raw(word) & 0x00FF_FFFF;
        assert_eq!(
            c, r,
            "E3 DIVERGENCE for argument word {word:#010x}: C {c:#08x} vs Rust {r:#08x}"
        );
        // And it must be the same as feeding the truncated byte triple.
        let s = h.c(equivalent);
        let packed = s.r as u32 | (s.g as u32) << 8 | (s.b as u32) << 16;
        assert_eq!(
            c, packed,
            "E3: C did not treat {word:#010x} as the byte triple {equivalent:?}"
        );
        let sr = h.rust(equivalent);
        assert_eq!(s, sr, "E3: struct-view divergence for {equivalent:?}");
    }
    eprintln!("E3: out-of-range channel words truncate to their low byte identically");
}

/// E4 — garbage in the unused 4th byte of the argument eightbyte.
#[test]
fn err_e4_argument_padding_ignored() {
    let h = Harness::new();
    let mut rng = Rng::seeded();
    for _ in 0..2000 {
        let x = rng.rgb();
        let base = x.r as u32 | (x.g as u32) << 8 | (x.b as u32) << 16;
        let clean_c = h.c_raw(base) & 0x00FF_FFFF;
        for pad in [0x00u32, 0x7F, 0x80, 0xDE, 0xFF] {
            let word = base | (pad << 24);
            let c = h.c_raw(word) & 0x00FF_FFFF;
            let r = h.rust_raw(word) & 0x00FF_FFFF;
            assert_eq!(c, clean_c, "E4: C changed with padding {pad:#04x} for {x:?}");
            assert_eq!(
                c, r,
                "E4 DIVERGENCE for {x:?} with padding {pad:#04x}: C {c:#08x} vs Rust {r:#08x}"
            );
        }
    }
    eprintln!("E4: 2000 x 5 padded argument words agreed");
}

/// E5 — an arbitrary 32-bit word reinterpreted as the struct.
///
/// `lib.h` declares no enum, so the closest analogue of "an out-of-range enum
/// value crossing the FFI" is an arbitrary machine word in the argument
/// register. The C validates nothing, so every one of the 2^32 words is
/// accepted; the Rust must accept them identically.
#[test]
fn err_e5_arbitrary_u32_as_struct() {
    let h = Harness::new();
    let mut rng = Rng::seeded();
    let mut words: Vec<u32> = vec![
        0x0000_0000,
        0xFFFF_FFFF,
        0x8000_0000,
        0x7FFF_FFFF,
        0xDEAD_BEEF,
        0xCAFE_BABE,
        0x0000_00FF,
        0x0000_FF00,
        0x00FF_0000,
        0xFF00_0000,
        0x8080_8080,
        0x0101_0101,
    ];
    words.extend((0..20_000).map(|_| rng.next_u32()));
    for w in words {
        let c = h.c_raw(w) & 0x00FF_FFFF;
        let r = h.rust_raw(w) & 0x00FF_FFFF;
        assert_eq!(
            c, r,
            "E5 DIVERGENCE for arbitrary word {w:#010x}: C {c:#08x} vs Rust {r:#08x}"
        );
    }
    eprintln!("E5: 20012 arbitrary 32-bit words accepted identically by both");
}

/// E6 — the return register's padding byte is unspecified; only R,G,B count.
#[test]
fn err_e6_return_padding_note() {
    let h = Harness::new();
    let mut rng = Rng::seeded();
    let mut c_pad_varies = false;
    let mut r_pad_varies = false;
    let (mut c_first_pad, mut r_first_pad) = (None, None);
    for _ in 0..5000 {
        let x = rng.rgb();
        let w = x.r as u32 | (x.g as u32) << 8 | (x.b as u32) << 16;
        let c = h.c_raw(w);
        let r = h.rust_raw(w);
        assert_eq!(
            c & 0x00FF_FFFF,
            r & 0x00FF_FFFF,
            "E6 DIVERGENCE in the significant bytes for {x:?}"
        );
        let (cp, rp) = (c >> 24, r >> 24);
        if *c_first_pad.get_or_insert(cp) != cp {
            c_pad_varies = true;
        }
        if *r_first_pad.get_or_insert(rp) != rp {
            r_pad_varies = true;
        }
    }
    eprintln!(
        "E6: 5000 results agreed on R,G,B. Return padding byte: C {:?}{}, Rust {:?}{} \
         (unspecified by the ABI, intentionally not asserted)",
        c_first_pad,
        if c_pad_varies { " (varies)" } else { "" },
        r_first_pad,
        if r_pad_varies { " (varies)" } else { "" }
    );
}

/// E7 — the out-of-range `float -> unsigned char` conversion.
///
/// This is the only place the C relies on implementation-defined behaviour, and
/// it is reachable from ordinary inputs. GCC emits
/// `cvttss2si %xmm0,%eax; mov %al,..`, i.e. truncate toward zero into `i32`
/// then take the low byte (a wrap, **not** a clamp). The Rust must reproduce
/// the wrap, so the results here must be far from the saturating values 0/255.
#[test]
fn err_e7_cast_out_of_range() {
    let h = Harness::new();

    // Canonical achievers, plus every wrapping input reachable from the cube.
    let mut inputs = vec![Rgb::new(0, 0, 255), Rgb::new(255, 255, 0), Rgb::new(0, 1, 255)];
    inputs.extend(scan_all_where(20_000, |x| {
        let (y, _, _) = denorm_pre_cast(x);
        y < 0.0 || y >= 256.0
    }));
    assert!(
        inputs.len() > 100,
        "E7: expected many out-of-range-cast inputs, found {}",
        inputs.len()
    );

    let mut saw_neg = false;
    let mut saw_ovf = false;
    for &x in &inputs {
        let c = h.c(x);
        let r = h.rust(x);
        assert_eq!(
            c, r,
            "E7 DIVERGENCE for {x:?} (out-of-range cast): C {c:?} vs Rust {r:?}"
        );
        let y = denorm_pre_cast(x).0;
        if y < 0.0 {
            saw_neg = true;
        }
        if y >= 256.0 {
            saw_ovf = true;
        }
    }
    assert!(saw_neg, "E7: no negative-wrap input exercised");
    assert!(saw_ovf, "E7: no overflow-wrap input exercised");

    // Demonstrate the behaviour is a wrap, not a clamp: {0,0,255} drives
    // y ~= -419.2, so truncation gives -419 whose low byte is 0x5D = 93.
    let probe = Rgb::new(0, 0, 255);
    let y = denorm_pre_cast(probe).0;
    let expect_wrap = ((y as i32) as u8, y as i64);
    let got = h.c(probe);
    assert_eq!(
        got.r, expect_wrap.0,
        "E7: C's cast of {y} was {} but truncate-then-wrap predicts {} \
         (i32 value {})",
        got.r, expect_wrap.0, expect_wrap.1
    );
    assert_ne!(
        got.r, 0,
        "E7: the C cast saturated to 0 instead of wrapping — the model is wrong"
    );
    eprintln!(
        "E7: {} out-of-range-cast inputs agreed; {{0,0,255}} y={y} -> R={} (wrap, not clamp)",
        inputs.len(),
        got.r
    );
}

/// E8/E9 — there is no pointer and no length parameter in the public API, so
/// null-pointer and zero/oversized-length inputs do not exist. Recorded as a
/// test so the row is explicitly accounted for rather than silently skipped.
#[test]
fn err_e8_e9_no_pointer_or_length_parameters() {
    let header = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("c_src/include/lib.h"),
    )
    .expect("cannot read c_src/include/lib.h");
    assert!(
        !header.contains('*'),
        "E8/E9: lib.h now contains a pointer — the null/length rows are no longer inapplicable:\n{header}"
    );
    assert!(
        !header.to_lowercase().contains("size") && !header.to_lowercase().contains("len"),
        "E8/E9: lib.h now declares a length parameter"
    );
    assert!(
        !header.contains("enum"),
        "E8/E9: lib.h now declares an enum — add out-of-range-variant rows"
    );
    eprintln!("E8/E9: lib.h exposes no pointer, no length and no enum — rows inapplicable");
}
