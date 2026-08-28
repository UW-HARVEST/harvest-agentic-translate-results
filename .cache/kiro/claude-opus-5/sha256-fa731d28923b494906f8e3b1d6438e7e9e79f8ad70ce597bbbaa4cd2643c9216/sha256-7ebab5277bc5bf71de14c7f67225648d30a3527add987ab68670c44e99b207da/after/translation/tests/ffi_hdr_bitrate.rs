//! Differential tests for `hdr_bitrate`, the single public entry point of
//! `c_src/include/lib.h`.
//!
//! Both implementations are loaded from their shared objects with `libloading`
//! and invoked through the exported `hdr_bitrate` symbol, so the Rust
//! `#[no_mangle] extern "C"` wrapper is on the hot path for every assertion.
//!
//! Structure, lowest level first:
//!
//! 1. the flat table lookup, driven directly by `(plane, row, col)`;
//! 2. the bit decoding of `h[1]` / `h[2]` into `(plane, row, col)`;
//! 3. the well-defined inputs, exhaustively;
//! 4. the inputs where the C indexing leaves the 90-byte table;
//! 5. every distinct input, exhaustively;
//! 6. which bytes of the header are actually read;
//! 7. realistic MPEG frame headers.

mod common;

use common::{flat_offset, is_defined, load_both, Impl};

/// The `halfrate` table, transcribed from `c_src/src/lib.c`.
const HALFRATE: [[[u8; 15]; 3]; 2] = [
    [
        [0, 4, 8, 12, 16, 20, 24, 28, 32, 40, 48, 56, 64, 72, 80],
        [0, 4, 8, 12, 16, 20, 24, 28, 32, 40, 48, 56, 64, 72, 80],
        [0, 16, 24, 28, 32, 40, 48, 56, 64, 72, 80, 88, 96, 112, 128],
    ],
    [
        [0, 16, 20, 24, 28, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160],
        [0, 16, 24, 28, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192],
        [0, 16, 32, 48, 64, 80, 96, 112, 128, 144, 160, 176, 192, 208, 224],
    ],
];

/// Build the `h[1]` byte that selects `plane` and `row`.
///
/// ```text
/// plane = !!(h[1] & 0x8)
/// row   = ((h[1] >> 1) & 3) - 1
/// ```
///
/// So bit 3 carries `plane` and bits 2..1 carry `row + 1`. Bits 0 and 4..7 are
/// unused by the C and are supplied by the caller as `noise`.
fn make_h1(plane: u8, row_plus_one: u8, noise: u8) -> u8 {
    assert!(plane <= 1);
    assert!(row_plus_one <= 3);
    ((plane & 1) << 3) | ((row_plus_one & 3) << 1) | (noise & 0xE1)
}

/// Build the `h[2]` byte that selects `col = h[2] >> 4`.
fn make_h2(col: u8, noise: u8) -> u8 {
    assert!(col <= 15);
    (col << 4) | (noise & 0x0F)
}

fn assert_same(c: &Impl, rust: &Impl, header: &[u8], what: &str) -> u32 {
    let cv = c.hdr_bitrate(header);
    let rv = rust.hdr_bitrate(header);
    assert_eq!(
        cv, rv,
        "mismatch for {what} (header = {header:02x?}): C returned {cv}, Rust returned {rv}"
    );
    cv
}

// ---------------------------------------------------------------------------
// Level 1: the table lookup itself
// ---------------------------------------------------------------------------

/// Every in-table `(plane, row, col)` triple must yield `2 * halfrate[..]`,
/// and C and Rust must agree. This pins the table contents themselves rather
/// than only checking that the two implementations agree with each other.
#[test]
fn table_lookup_matches_c_source_table() {
    let (c, rust) = load_both();

    for plane in 0u8..2 {
        for row in 0u8..3 {
            for col in 0u8..15 {
                let header = [0x00, make_h1(plane, row + 1, 0), make_h2(col, 0), 0x00];
                let got = assert_same(
                    &c,
                    &rust,
                    &header,
                    &format!("plane={plane} row={row} col={col}"),
                );
                let expected = 2 * u32::from(HALFRATE[plane as usize][row as usize][col as usize]);
                assert_eq!(
                    got, expected,
                    "plane={plane} row={row} col={col}: expected 2*{} = {expected}, got {got}",
                    HALFRATE[plane as usize][row as usize][col as usize]
                );
            }
        }
    }
}

/// The return value is always `2 * <table byte>`, so it is always even and,
/// for in-table reads, at most `2 * 224`.
#[test]
fn results_are_even_and_bounded_for_defined_inputs() {
    let (c, rust) = load_both();

    for h1 in 0u8..=255 {
        for h2 in 0u8..=255 {
            if !is_defined(h1, h2) {
                continue;
            }
            let header = [0xFF, h1, h2, 0x00];
            let v = assert_same(&c, &rust, &header, "defined input");
            assert_eq!(v % 2, 0, "h1={h1:#04x} h2={h2:#04x}: {v} is odd");
            assert!(v <= 2 * 224, "h1={h1:#04x} h2={h2:#04x}: {v} out of range");
        }
    }
}

// ---------------------------------------------------------------------------
// Level 2: bit decoding of h[1] and h[2]
// ---------------------------------------------------------------------------

/// `plane` comes from bit 3 of `h[1]`, `row` from bits 2..1, `col` from the
/// high nibble of `h[2]`. Bit 0 and bits 7..4 of `h[1]` and the low nibble of
/// `h[2]` must not influence the result. Verified against C for a spread of
/// noise patterns.
#[test]
fn ignored_header_bits_do_not_change_result() {
    let (c, rust) = load_both();
    let noises: [u8; 6] = [0x00, 0xFF, 0xA5, 0x5A, 0xE1, 0x10];

    for plane in 0u8..2 {
        for row_plus_one in 0u8..4 {
            for col in 0u8..16 {
                let baseline = {
                    let header = [0x00, make_h1(plane, row_plus_one, 0), make_h2(col, 0), 0x00];
                    assert_same(&c, &rust, &header, "baseline")
                };
                for &n1 in &noises {
                    for &n2 in &noises {
                        let header =
                            [0x00, make_h1(plane, row_plus_one, n1), make_h2(col, n2), 0x00];
                        let v = assert_same(&c, &rust, &header, "noisy header");
                        assert_eq!(
                            v, baseline,
                            "noise n1={n1:#04x} n2={n2:#04x} changed the result for \
                             plane={plane} row+1={row_plus_one} col={col}"
                        );
                    }
                }
            }
        }
    }
}

/// `plane` is derived with `!!(h[1] & 0x8)`, so it saturates to 1 for any
/// non-zero masked value. Because the mask is a single bit this is only
/// 0 or 1, but the two planes must be distinguishable.
#[test]
fn plane_bit_selects_between_the_two_planes() {
    let (c, rust) = load_both();

    // row 2 (layer bits 11), col 1: plane 0 -> 2*16 = 32, plane 1 -> 2*16 = 32.
    // Use col 2 instead, where the planes differ: 24 vs 32 -> 48 vs 64.
    let lo = assert_same(&c, &rust, &[0, make_h1(0, 3, 0), make_h2(2, 0), 0], "plane 0");
    let hi = assert_same(&c, &rust, &[0, make_h1(1, 3, 0), make_h2(2, 0), 0], "plane 1");
    assert_eq!(lo, 2 * 24);
    assert_eq!(hi, 2 * 32);
    assert_ne!(lo, hi);
}

// ---------------------------------------------------------------------------
// Level 3: exhaustive over the well-defined inputs
// ---------------------------------------------------------------------------

/// All `(h[1], h[2])` pairs whose flat offset lands inside the real 90-byte
/// table. These are the inputs where the C behaviour is fully specified by the
/// C standard, so they must match unconditionally.
#[test]
fn exhaustive_defined_inputs_match() {
    let (c, rust) = load_both();
    let mut checked = 0usize;

    for h1 in 0u8..=255 {
        for h2 in 0u8..=255 {
            if !is_defined(h1, h2) {
                continue;
            }
            let header = [0xFF, h1, h2, 0xAA];
            assert_same(&c, &rust, &header, "defined input");
            checked += 1;
        }
    }

    // 65536 total inputs. Undefined ones are:
    //   * plane 0, row -1, col 0..=14  -> offsets -15..=-1
    //     h[1] has bit3=0 and bits2..1=00, leaving 5 free bits -> 32 values;
    //     h[2] has col 0..=14, low nibble free            -> 240 values.  7680
    //   * plane 1, row 2, col 15       -> offset 90
    //     32 h[1] values * 16 h[2] values                              =   512
    // 65536 - 7680 - 512 = 57344.
    assert_eq!(checked, 57_344, "unexpected number of defined inputs");
}

// ---------------------------------------------------------------------------
// Level 4: the inputs where the C indexing leaves the table
// ---------------------------------------------------------------------------

/// The C computes `row = ((h[1] >> 1) & 3) - 1`, which is `-1` when the layer
/// bits are `00`, and `col = h[2] >> 4`, which reaches `15` although the
/// innermost dimension is only 15 wide. Flat offsets therefore span
/// `-15 ..= 90`; the ones outside `0..90` read past the table.
///
/// Those reads are still fully deterministic for a given build (the table sits
/// at the start of `.rodata` and is surrounded by zero padding), so the
/// translation is required to reproduce them too.
#[test]
fn out_of_table_reads_match() {
    let (c, rust) = load_both();
    let mut seen_negative = 0usize;
    let mut seen_past_end = 0usize;

    for h1 in 0u8..=255 {
        for h2 in 0u8..=255 {
            let off = flat_offset(h1, h2);
            if (0..90).contains(&off) {
                continue;
            }
            let header = [0xFF, h1, h2, 0x00];
            assert_same(
                &c,
                &rust,
                &header,
                &format!("out-of-table read at flat offset {off}"),
            );
            if off < 0 {
                seen_negative += 1;
            } else {
                seen_past_end += 1;
            }
        }
    }

    assert!(seen_negative > 0, "no negative offsets exercised");
    assert!(seen_past_end > 0, "no past-the-end offsets exercised");
}

/// Spot-check the exact boundary offsets: the most negative (`-15`), the last
/// negative (`-1`) and the first past the end (`90`).
#[test]
fn boundary_offsets_match() {
    let (c, rust) = load_both();

    // plane 0, row -1 (layer bits 00), col 0 -> flat offset -15.
    let h = [0xFF, make_h1(0, 0, 0), make_h2(0, 0), 0x00];
    assert_eq!(flat_offset(h[1], h[2]), -15);
    assert_same(&c, &rust, &h, "flat offset -15");

    // plane 0, row -1, col 14 -> flat offset -1.
    let h = [0xFF, make_h1(0, 0, 0), make_h2(14, 0), 0x00];
    assert_eq!(flat_offset(h[1], h[2]), -1);
    assert_same(&c, &rust, &h, "flat offset -1");

    // plane 1, row 2, col 15 -> flat offset 90.
    let h = [0xFF, make_h1(1, 3, 0), make_h2(15, 0), 0x00];
    assert_eq!(flat_offset(h[1], h[2]), 90);
    assert_same(&c, &rust, &h, "flat offset 90");

    // plane 0, row -1, col 15 -> flat offset 0, i.e. back inside the table.
    let h = [0xFF, make_h1(0, 0, 0), make_h2(15, 0), 0x00];
    assert_eq!(flat_offset(h[1], h[2]), 0);
    assert_eq!(assert_same(&c, &rust, &h, "flat offset 0"), 0);

    // plane 1, row -1, col 0 -> flat offset 30, i.e. halfrate[0][2][0].
    let h = [0xFF, make_h1(1, 0, 0), make_h2(0, 0), 0x00];
    assert_eq!(flat_offset(h[1], h[2]), 30);
    assert_same(&c, &rust, &h, "flat offset 30");
}

// ---------------------------------------------------------------------------
// Level 5: exhaustive over every distinct input
// ---------------------------------------------------------------------------

/// Only `h[1]` and `h[2]` are read, so `256 * 256` headers cover the entire
/// input space of the function. Every one of them must match byte for byte.
#[test]
fn exhaustive_all_inputs_match() {
    let (c, rust) = load_both();
    let mut mismatches: Vec<String> = Vec::new();

    for h1 in 0u8..=255 {
        for h2 in 0u8..=255 {
            let header = [0xFF, h1, h2, 0x00];
            let cv = c.hdr_bitrate(&header);
            let rv = rust.hdr_bitrate(&header);
            if cv != rv {
                mismatches.push(format!(
                    "h1={h1:#04x} h2={h2:#04x} offset={} C={cv} Rust={rv}",
                    flat_offset(h1, h2)
                ));
            }
        }
    }

    assert!(
        mismatches.is_empty(),
        "{} of 65536 inputs mismatched:\n{}",
        mismatches.len(),
        mismatches
            .iter()
            .take(40)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

// ---------------------------------------------------------------------------
// Level 6: which header bytes are read
// ---------------------------------------------------------------------------

/// The C only dereferences `h[1]` and `h[2]`. Varying `h[0]` (and anything
/// beyond `h[2]`) must not change the result in either implementation.
#[test]
fn only_bytes_one_and_two_are_read() {
    let (c, rust) = load_both();

    for h1 in [0x00u8, 0x02, 0x04, 0x06, 0x08, 0x0A, 0x0C, 0x0E, 0xFB] {
        for h2 in [0x00u8, 0x10, 0x50, 0x90, 0xE0, 0xF0, 0xFF] {
            let baseline = {
                let header = [0x00, h1, h2, 0x00];
                assert_same(&c, &rust, &header, "baseline")
            };
            for &h0 in &[0x00u8, 0xFF, 0x55, 0xAA] {
                for &tail in &[0x00u8, 0xFF, 0x33] {
                    let header = [h0, h1, h2, tail, tail, tail];
                    let v = assert_same(&c, &rust, &header, "padded header");
                    assert_eq!(
                        v, baseline,
                        "h0={h0:#04x}/tail={tail:#04x} changed the result for \
                         h1={h1:#04x} h2={h2:#04x}"
                    );
                }
            }
        }
    }
}

/// The pointer is read through `h[1]` / `h[2]`, so the function must work for
/// a header that starts at an arbitrary offset inside a larger buffer,
/// including unaligned starts.
#[test]
fn unaligned_and_offset_headers_match() {
    let (c, rust) = load_both();
    let mut buf = [0u8; 64];
    for (i, b) in buf.iter_mut().enumerate() {
        *b = (i as u8).wrapping_mul(37).wrapping_add(11);
    }

    for start in 0..buf.len() - 3 {
        let slice = &buf[start..];
        let cv = c.hdr_bitrate(slice);
        let rv = rust.hdr_bitrate(slice);
        assert_eq!(
            cv, rv,
            "mismatch at buffer offset {start} (bytes {:02x?})",
            &slice[..3]
        );
    }
}

// ---------------------------------------------------------------------------
// Level 7: realistic MPEG frame headers
// ---------------------------------------------------------------------------

/// Real MPEG-1/2/2.5 audio frame headers, where `h[1]` bit 3 is the MPEG
/// version bit, bits 2..1 are the layer and the high nibble of `h[2]` is the
/// bitrate index.
#[test]
fn realistic_mpeg_headers_match() {
    let (c, rust) = load_both();

    // (description, four header bytes)
    let cases: &[(&str, [u8; 4])] = &[
        ("MPEG1 Layer III 128kbps 44.1kHz", [0xFF, 0xFB, 0x90, 0x00]),
        ("MPEG1 Layer III 320kbps 44.1kHz", [0xFF, 0xFB, 0xE0, 0x00]),
        ("MPEG1 Layer III 32kbps 44.1kHz", [0xFF, 0xFB, 0x10, 0x00]),
        ("MPEG1 Layer III free format", [0xFF, 0xFB, 0x00, 0x00]),
        ("MPEG1 Layer III bad bitrate idx", [0xFF, 0xFB, 0xF0, 0x00]),
        ("MPEG1 Layer II 192kbps", [0xFF, 0xFD, 0xB0, 0x00]),
        ("MPEG1 Layer I 448kbps", [0xFF, 0xFF, 0xE0, 0x00]),
        ("MPEG1 reserved layer", [0xFF, 0xF9, 0x90, 0x00]),
        ("MPEG2 Layer III 64kbps 22.05kHz", [0xFF, 0xF3, 0x90, 0x00]),
        ("MPEG2 Layer II 48kbps", [0xFF, 0xF5, 0x50, 0x00]),
        ("MPEG2 Layer I 256kbps", [0xFF, 0xF7, 0xE0, 0x00]),
        ("MPEG2 reserved layer", [0xFF, 0xF1, 0x90, 0x00]),
        ("MPEG2.5 Layer III", [0xFF, 0xE3, 0x80, 0x00]),
        ("all zero header", [0x00, 0x00, 0x00, 0x00]),
        ("all ones header", [0xFF, 0xFF, 0xFF, 0xFF]),
    ];

    for (name, header) in cases {
        assert_same(&c, &rust, header, name);
    }
}

/// The canonical MPEG-1 Layer III 128 kbps header must decode to 128, which
/// anchors the whole table to a value that can be checked independently of
/// either implementation.
#[test]
fn known_good_value_is_128kbps() {
    let (c, rust) = load_both();
    let header = [0xFFu8, 0xFB, 0x90, 0x00];
    let v = assert_same(&c, &rust, &header, "MPEG1 Layer III 128kbps");
    assert_eq!(v, 128, "expected 128 kbps, got {v}");
}
