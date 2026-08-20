//! Phase B — differential tests for `load_png_mem`.  CONFIGS.md rows 27-68.

mod common;

use common::*;

const COLOUR_TYPES: [u8; 5] = [0, 2, 3, 4, 6];

/// Differential check plus (where possible) a cross-check against an
/// independent RFC-2083 reference decoder, so a "both wrong the same way"
/// result cannot masquerade as success.
fn ok_png(spec: &Spec, label: &str) {
    let png = spec.build();
    let r = diff_png(&png, label);
    assert!(r.ok, "[{label}] expected a successful decode, got err={:?}", r.err);
    assert_eq!(r.w, spec.w as i32, "[{label}] width");
    assert_eq!(r.h, spec.h as i32, "[{label}] height");
    if let Some(expect) = reference_rgba(spec) {
        assert_eq!(
            r.pixels.len(),
            expect.len(),
            "[{label}] pixel buffer length"
        );
        if r.pixels != expect {
            let i = r
                .pixels
                .iter()
                .zip(expect.iter())
                .position(|(a, b)| a != b)
                .unwrap();
            panic!(
                "[{label}] differs from the reference decoder at byte {i}: got {:?} want {:?}",
                &r.pixels[i..(i + 8).min(r.pixels.len())],
                &expect[i..(i + 8).min(expect.len())]
            );
        }
    }
}

// ---------------------------------------------------------------------------
// rows 27-37: colour type / bpp / palette surface
// ---------------------------------------------------------------------------

/// Rows 27-30 — the four direct colour types (`cp_convert` cases 1..4).
#[test]
fn row_27_30_direct_colour_types() {
    let mut rng = Rng::new(0x2701);
    for ct in [0u8, 2, 4, 6] {
        for iter in 0..30 {
            let w = rng.range(1, 40);
            let h = rng.range(1, 40);
            let mut s = Spec::new(w, h, ct);
            s.payload = rng.bytes((w * h) as usize * bpp_of(ct) + 7);
            s.filters = vec![0];
            ok_png(&s, &format!("ct={ct} {w}x{h} iter={iter}"));
        }
    }
}

/// Row 31 — indexed with a full 256-entry palette, no tRNS (alpha = 255).
#[test]
fn row_31_indexed_no_trns() {
    let mut rng = Rng::new(0x3101);
    for iter in 0..30 {
        let (w, h) = (rng.range(1, 40), rng.range(1, 40));
        let mut s = Spec::new(w, h, 3);
        s.payload = rng.bytes((w * h) as usize + 5);
        s.plte = Some(rng.bytes(256 * 3));
        s.trns = None;
        ok_png(&s, &format!("indexed {w}x{h} iter={iter}"));
    }
}

/// Rows 32-34, 36 — indexed with tRNS of every interesting length.
#[test]
fn row_32_36_indexed_trns_lengths() {
    let mut rng = Rng::new(0x3201);
    for &tl in &[0usize, 1, 2, 17, 128, 255, 256, 257, 300, 512] {
        for iter in 0..6 {
            let (w, h) = (rng.range(1, 24), rng.range(1, 24));
            let mut s = Spec::new(w, h, 3);
            s.payload = rng.bytes((w * h) as usize + 3);
            s.plte = Some(rng.bytes(256 * 3));
            s.trns = Some(rng.bytes(tl));
            ok_png(&s, &format!("indexed trns_len={tl} {w}x{h} iter={iter}"));
        }
    }
}

/// Row 35 — PLTE shorter than the largest index used.  The C code reads past
/// the chunk; both implementations must read the *same* bytes of the *same*
/// buffer, so the result still has to match exactly.
#[test]
fn row_35_short_palette() {
    let mut rng = Rng::new(0x3501);
    for &pl in &[3usize, 6, 30, 3 * 100, 3 * 255] {
        for iter in 0..6 {
            let (w, h) = (rng.range(1, 16), rng.range(1, 16));
            let mut s = Spec::new(w, h, 3);
            s.payload = rng.bytes((w * h) as usize + 3); // indices 0..255
            s.plte = Some(rng.bytes(pl));
            s.trns = None;
            // no reference cross-check: the C code reads out of bounds here
            let png = s.build();
            let r = diff_png(&png, &format!("short plte={pl} iter={iter}"));
            assert!(r.ok, "expected success, err={:?}", r.err);
        }
    }
}

/// Row 37 — PLTE / tRNS present for a *non*-indexed colour type: ignored by
/// `cp_convert`, but they still move the internal `first` pointer.
#[test]
fn row_37_palette_on_direct_colour_type() {
    let mut rng = Rng::new(0x3701);
    for ct in [0u8, 2, 4, 6] {
        for (with_plte, with_trns) in [(true, false), (false, true), (true, true)] {
            let (w, h) = (rng.range(1, 20), rng.range(1, 20));
            let mut s = Spec::new(w, h, ct);
            s.payload = rng.bytes((w * h) as usize * bpp_of(ct) + 5);
            s.plte = if with_plte { Some(rng.bytes(256 * 3)) } else { None };
            s.trns = if with_trns { Some(rng.bytes(6)) } else { None };
            ok_png(&s, &format!("ct={ct} plte={with_plte} trns={with_trns}"));
        }
    }
}

// ---------------------------------------------------------------------------
// rows 38-45: the filter surface
// ---------------------------------------------------------------------------

/// Rows 38-42 — a uniform filter type for every row, for all colour types.
#[test]
fn row_38_42_uniform_filters() {
    let mut rng = Rng::new(0x3801);
    for f in 0..=4u8 {
        for ct in COLOUR_TYPES {
            for iter in 0..8 {
                let (w, h) = (rng.range(1, 24), rng.range(1, 24));
                let mut s = Spec::new(w, h, ct);
                s.filters = vec![f];
                s.payload = rng.bytes((w * h) as usize * bpp_of(ct) + 11);
                ok_png(&s, &format!("filter={f} ct={ct} {w}x{h} iter={iter}"));
            }
        }
    }
}

/// Row 43 — a random filter type per row.
#[test]
fn row_43_random_filters() {
    let mut rng = Rng::new(0x4301);
    for ct in COLOUR_TYPES {
        for iter in 0..25 {
            let (w, h) = (rng.range(1, 30), rng.range(1, 32));
            let mut s = Spec::new(w, h, ct);
            s.filters = (0..h).map(|_| rng.below(5) as u8).collect();
            s.payload = rng.bytes((w * h) as usize * bpp_of(ct) + 13);
            ok_png(&s, &format!("randfilter ct={ct} {w}x{h} iter={iter}"));
        }
    }
}

/// Row 44 — `h == 1`: the `for (y = 1; y < h; ...)` loop never runs, so only
/// the special-cased row 0 is exercised.
#[test]
fn row_44_single_row() {
    let mut rng = Rng::new(0x4401);
    for f in 0..=4u8 {
        for ct in COLOUR_TYPES {
            for w in [1u32, 2, 3, 4, 7, 16, 33] {
                let mut s = Spec::new(w, 1, ct);
                s.filters = vec![f];
                s.payload = rng.bytes(w as usize * bpp_of(ct) + 3);
                ok_png(&s, &format!("h=1 f={f} ct={ct} w={w}"));
            }
        }
    }
}

/// Row 45 — `w == 1`: `len == bpp`, so the `for (x = bpp; x < len; ...)` loops
/// never run either.
#[test]
fn row_45_single_column() {
    let mut rng = Rng::new(0x4501);
    for f in 0..=4u8 {
        for ct in COLOUR_TYPES {
            for h in [1u32, 2, 3, 8, 40] {
                let mut s = Spec::new(1, h, ct);
                s.filters = vec![f];
                s.payload = rng.bytes(h as usize * bpp_of(ct) + 3);
                ok_png(&s, &format!("w=1 f={f} ct={ct} h={h}"));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// rows 46-52: image shapes
// ---------------------------------------------------------------------------

/// Row 46 — 1x1, the smallest legal image, all colour types & filters.
#[test]
fn row_46_one_by_one() {
    let mut rng = Rng::new(0x4601);
    for ct in COLOUR_TYPES {
        for f in 0..=4u8 {
            for _ in 0..4 {
                let mut s = Spec::new(1, 1, ct);
                s.filters = vec![f];
                s.payload = rng.bytes(bpp_of(ct) + 1);
                ok_png(&s, &format!("1x1 ct={ct} f={f}"));
            }
        }
    }
}

/// Rows 47-48 — 1xN and Nx1.
#[test]
fn row_47_48_thin_images() {
    let mut rng = Rng::new(0x4701);
    for ct in COLOUR_TYPES {
        for n in [1u32, 2, 3, 5, 13, 64] {
            for (w, h) in [(1, n), (n, 1)] {
                let mut s = Spec::new(w, h, ct);
                s.filters = (0..h).map(|_| rng.below(5) as u8).collect();
                s.payload = rng.bytes((w * h) as usize * bpp_of(ct) + 7);
                ok_png(&s, &format!("thin ct={ct} {w}x{h}"));
            }
        }
    }
}

/// Rows 49-50 — the `(w+1)*h*4` vs `(w+1)*h*bpp` boundary for bpp == 1
/// (`out` overlaps the converted pixels differently for w < 3, w == 3, w > 3).
#[test]
fn row_49_50_out_overlap_boundary() {
    let mut rng = Rng::new(0x4901);
    for ct in [0u8, 3] {
        for w in [1u32, 2, 3, 4, 5] {
            for h in [1u32, 2, 3, 7, 16, 33] {
                let mut s = Spec::new(w, h, ct);
                s.filters = (0..h).map(|_| rng.below(5) as u8).collect();
                s.payload = rng.bytes((w * h) as usize + 5);
                ok_png(&s, &format!("overlap ct={ct} {w}x{h}"));
            }
        }
    }
    // bpp == 2 has the same kind of boundary at w == 1
    for w in [1u32, 2, 3] {
        for h in [1u32, 4, 17] {
            let mut s = Spec::new(w, h, 4);
            s.filters = vec![1, 2, 3, 4, 0];
            s.payload = rng.bytes((w * h) as usize * 2 + 5);
            ok_png(&s, &format!("overlap2 {w}x{h}"));
        }
    }
}

/// Row 51 — larger images.
#[test]
fn row_51_larger_images() {
    let mut rng = Rng::new(0x5101);
    for &(w, h) in &[(64u32, 64u32), (127, 33), (256, 3), (3, 256), (300, 17)] {
        for ct in COLOUR_TYPES {
            let mut s = Spec::new(w, h, ct);
            s.filters = (0..h).map(|_| rng.below(5) as u8).collect();
            s.payload = rng.bytes((w * h) as usize * bpp_of(ct) + 17);
            s.deflate = Deflate::Flate2(6);
            ok_png(&s, &format!("large ct={ct} {w}x{h}"));
        }
    }
}

/// Row 52 — randomised w x h x colour type x filter x compressor.
#[test]
fn row_52_randomised_cross_product() {
    let mut rng = Rng::new(0x5201);
    for iter in 0..300 {
        let ct = COLOUR_TYPES[rng.below(5) as usize];
        let w = rng.range(1, 60);
        let h = rng.range(1, 40);
        let mut s = Spec::new(w, h, ct);
        s.filters = (0..h).map(|_| rng.below(5) as u8).collect();
        s.payload = rng.bytes((w * h) as usize * bpp_of(ct) + 19);
        s.deflate = match rng.below(4) {
            0 => Deflate::Fixed,
            1 => Deflate::Dynamic { rle: rng.below(2) == 0 },
            2 => Deflate::Flate2(rng.range(1, 9)),
            _ => Deflate::Stored,
        };
        if ct == 3 {
            s.plte = Some(rng.bytes(256 * 3));
            s.trns = if rng.below(2) == 0 {
                let tl = rng.range(0, 256) as usize;
                Some(rng.bytes(tl))
            } else {
                None
            };
        }
        s.n_idat = rng.range(1, 5) as usize;
        ok_png(&s, &format!("cross iter={iter} ct={ct} {w}x{h}"));
    }
}

// ---------------------------------------------------------------------------
// rows 53-68: container / chunk surface
// ---------------------------------------------------------------------------

/// Rows 53-55 — IDAT splitting, including zero-length IDAT chunks.
#[test]
fn row_53_55_idat_splitting() {
    let mut rng = Rng::new(0x5301);
    for n in 1..=17usize {
        for empty in [false, true] {
            let ct = COLOUR_TYPES[rng.below(5) as usize];
            let (w, h) = (rng.range(2, 24), rng.range(2, 24));
            let mut s = Spec::new(w, h, ct);
            s.filters = (0..h).map(|_| rng.below(5) as u8).collect();
            s.payload = rng.bytes((w * h) as usize * bpp_of(ct) + 7);
            s.deflate = Deflate::Flate2(6);
            s.n_idat = n;
            s.empty_idats = empty;
            ok_png(&s, &format!("idat n={n} empty={empty} ct={ct}"));
        }
    }
}

/// Row 56 — non-contiguous IDATs.  `load_png_mem` finds the first IDAT with
/// `cp_find` and then walks the run with `cp_chunk`, which insists that the
/// *next* chunk is an IDAT too.  So a non-IDAT chunk terminates the run and
/// every later IDAT is silently dropped.
///
/// The complete zlib stream is put into the leading run and pure junk into the
/// IDATs behind the gap; the decode therefore has to succeed and produce
/// exactly the reference pixels.  (A test that truncates the *real* stream
/// instead cannot be written differentially: the C code then unfilters
/// never-written `malloc` memory, so its result depends on heap garbage.)
#[test]
fn row_56_non_contiguous_idats() {
    let mut rng = Rng::new(0x5601);
    for lead in 1..=4usize {
        for ct in COLOUR_TYPES {
            let (w, h) = (rng.range(2, 20), rng.range(2, 20));
            let mut s = Spec::new(w, h, ct);
            s.filters = (0..h).map(|_| rng.below(5) as u8).collect();
            s.payload = rng.bytes((w * h) as usize * bpp_of(ct) + 3);
            s.deflate = Deflate::Flate2(6);
            if ct == 3 {
                s.plte = Some(rng.bytes(256 * 3));
            }

            let z = s.zlib_stream();
            let mut chunks: Vec<Chunk> = vec![Chunk::new(
                b"IHDR",
                ihdr(w, h, 8, ct, 0, 0, 0),
            )];
            if let Some(p) = &s.plte {
                chunks.push(Chunk::new(b"PLTE", p.clone()));
            }
            // the complete stream, spread over `lead` contiguous IDATs
            chunks.extend(split_idat(&z, lead));
            // ...then a gap, then IDATs that must be ignored entirely
            chunks.push(Chunk::new(b"gAMA", vec![0, 1, 2, 3]));
            chunks.push(Chunk::new(b"IDAT", rng.bytes(37)));
            chunks.push(Chunk::new(b"IDAT", rng.bytes(11)));
            chunks.push(Chunk::new(b"IEND", vec![]));

            let png = build_png(&PNG_SIG, &chunks);
            let label = format!("idat gap lead={lead} ct={ct} {w}x{h}");
            let r = diff_png(&png, &label);
            assert!(r.ok, "[{label}] expected success, err={:?}", r.err);
            let expect = reference_rgba(&s).unwrap();
            assert_eq!(r.pixels, expect, "[{label}] IDATs after the gap must be ignored");
            // and the identical input must terminate identically
            diff_png_forked(&png, png.len() as i32, &format!("{label} forked"));
        }
    }

    // An unknown chunk *before* the first IDAT is skipped by `cp_find`.
    for ct in COLOUR_TYPES {
        let (w, h) = (rng.range(2, 20), rng.range(2, 20));
        let mut s = Spec::new(w, h, ct);
        s.filters = (0..h).map(|_| rng.below(5) as u8).collect();
        s.payload = rng.bytes((w * h) as usize * bpp_of(ct) + 3);
        s.pre_chunks = vec![Chunk::new(b"gAMA", vec![0, 1, 2, 3])];
        ok_png(&s, &format!("gap-before-idat ct={ct}"));
    }
}

/// Rows 57-58 — unknown ancillary chunks in every position.
#[test]
fn row_57_58_unknown_chunks() {
    let mut rng = Rng::new(0x5701);
    let extras = || {
        vec![
            Chunk::new(b"gAMA", vec![0, 1, 134, 160]),
            Chunk::new(b"sRGB", vec![0]),
            Chunk::new(b"tEXt", b"Comment\0hello world".to_vec()),
            Chunk::new(b"bKGD", vec![0, 0, 0, 0, 0, 0]),
            Chunk::new(b"pHYs", vec![0, 0, 11, 18, 0, 0, 11, 18, 1]),
        ]
    };
    for ct in COLOUR_TYPES {
        for (pre, mid) in [(true, false), (false, true), (true, true)] {
            let (w, h) = (rng.range(1, 24), rng.range(1, 24));
            let mut s = Spec::new(w, h, ct);
            s.filters = (0..h).map(|_| rng.below(5) as u8).collect();
            s.payload = rng.bytes((w * h) as usize * bpp_of(ct) + 7);
            if pre {
                s.pre_chunks = extras();
            }
            if mid {
                s.mid_chunks = extras();
            }
            if ct == 3 {
                let tl = rng.range(0, 256) as usize;
                s.trns = Some(rng.bytes(tl));
            }
            ok_png(&s, &format!("extras ct={ct} pre={pre} mid={mid}"));
        }
    }
}

/// Row 59 — tRNS *before* PLTE: `cp_find(PLTE)` moves the cursor past PLTE, so
/// the following `cp_find(tRNS)` cannot see the earlier tRNS and `trns` stays
/// NULL (every pixel therefore gets alpha 255).
#[test]
fn row_59_trns_before_plte() {
    let mut rng = Rng::new(0x5901);
    for iter in 0..12 {
        let (w, h) = (rng.range(1, 20), rng.range(1, 20));
        let mut s = Spec::new(w, h, 3);
        s.payload = rng.bytes((w * h) as usize + 3);
        s.plte = Some(rng.bytes(256 * 3));
        s.trns = Some(rng.bytes(256));
        s.order = Order::TrnsBeforePlte;
        let png = s.build();
        let r = diff_png(&png, &format!("trns-before-plte iter={iter}"));
        assert!(r.ok, "err={:?}", r.err);
        // the C quirk: tRNS is ignored, so all alphas are 0xFF
        let mut expect = s.clone();
        expect.trns = None;
        let want = reference_rgba(&expect).unwrap();
        assert_eq!(r.pixels, want, "tRNS must be ignored when it precedes PLTE");
    }
}

/// Row 60 — tRNS *after* the IDATs: `first` is advanced past them, so the IDAT
/// scan finds nothing and the zlib check rejects the file.
#[test]
fn row_60_trns_after_idat() {
    let mut rng = Rng::new(0x6001);
    for iter in 0..8 {
        let (w, h) = (rng.range(1, 20), rng.range(1, 20));
        let mut s = Spec::new(w, h, 3);
        s.payload = rng.bytes((w * h) as usize + 3);
        s.plte = Some(rng.bytes(256 * 3));
        s.trns = Some(rng.bytes(64));
        s.order = Order::TrnsAfterIdat;
        let png = s.build();
        let r = diff_png(&png, &format!("trns-after-idat iter={iter}"));
        assert!(!r.ok, "expected rejection");
        assert_eq!(
            r.err.as_deref(),
            Some("corrupt zlib structure in DEFLATE stream")
        );
    }
    // PLTE after the IDATs behaves the same way for the IDAT scan
    for iter in 0..8 {
        let (w, h) = (rng.range(1, 20), rng.range(1, 20));
        let mut s = Spec::new(w, h, 3);
        s.payload = rng.bytes((w * h) as usize + 3);
        s.plte = Some(rng.bytes(256 * 3));
        s.trns = None;
        s.order = Order::PlteAfterIdat;
        let png = s.build();
        diff_png(&png, &format!("plte-after-idat iter={iter}"));
    }
}

/// Row 61 — IEND present/absent, trailing garbage, `png_length` > real size.
#[test]
fn row_61_trailing_and_iend() {
    let mut rng = Rng::new(0x6101);
    for ct in COLOUR_TYPES {
        for iend in [true, false] {
            for trailing in [0usize, 1, 4, 37, 1024] {
                let (w, h) = (rng.range(1, 16), rng.range(1, 16));
                let mut s = Spec::new(w, h, ct);
                s.filters = (0..h).map(|_| rng.below(5) as u8).collect();
                s.payload = rng.bytes((w * h) as usize * bpp_of(ct) + 7);
                s.iend = iend;
                s.trailing = rng.bytes(trailing);
                ok_png(&s, &format!("tail ct={ct} iend={iend} n={trailing}"));
            }
        }
    }
    // png_length larger than the PNG but still inside the buffer
    for ct in COLOUR_TYPES {
        let (w, h) = (rng.range(1, 16), rng.range(1, 16));
        let mut s = Spec::new(w, h, ct);
        s.payload = rng.bytes((w * h) as usize * bpp_of(ct) + 7);
        let mut png = s.build();
        let real = png.len();
        png.extend(std::iter::repeat(0u8).take(4096));
        for extra in [0usize, 1, 7, 64, 4096] {
            diff_png_len(
                &png,
                (real + extra) as i32,
                &format!("len+{extra} ct={ct}"),
            );
        }
    }
}

/// Rows 62-63 — the zlib header byte space that the C code accepts.
#[test]
fn row_62_63_zlib_header() {
    let mut rng = Rng::new(0x6201);
    for cinfo in 0..8u8 {
        for flevel in 0..4u8 {
            // FCHECK is *never* validated, so sweep it too
            for fcheck in [0u8, 1, 0x1F] {
                let (w, h) = (rng.range(1, 12), rng.range(1, 12));
                let mut s = Spec::new(w, h, 2);
                s.payload = rng.bytes((w * h) as usize * 3 + 3);
                s.cmf = (cinfo << 4) | 0x08;
                s.flg = (flevel << 6) | fcheck; // FDICT (0x20) clear
                ok_png(
                    &s,
                    &format!("zlib cinfo={cinfo} flevel={flevel} fcheck={fcheck}"),
                );
            }
        }
    }
}

/// Rows 64-66 — every DEFLATE flavour inside a PNG.
#[test]
fn row_64_66_deflate_flavours() {
    let mut rng = Rng::new(0x6401);
    let flavours: Vec<Deflate> = vec![
        Deflate::Stored,
        Deflate::Fixed,
        Deflate::Dynamic { rle: false },
        Deflate::Dynamic { rle: true },
    ]
    .into_iter()
    .chain((0..=9u32).map(Deflate::Flate2))
    .collect();
    for d in flavours {
        for ct in COLOUR_TYPES {
            for iter in 0..3 {
                let (w, h) = (rng.range(1, 30), rng.range(1, 20));
                let mut s = Spec::new(w, h, ct);
                s.filters = (0..h).map(|_| rng.below(5) as u8).collect();
                s.payload = rng.bytes((w * h) as usize * bpp_of(ct) + 7);
                s.deflate = d;
                let png = s.build();
                let label = format!("deflate {d:?} ct={ct} {w}x{h} iter={iter}");
                let r = diff_png(&png, &label);
                // `Deflate::Stored` on a raw stream >= 64 KiB (or a flate2 level
                // that falls back to several stored blocks) is rejected by C,
                // see ERRORS.md row 26.  Everything else must decode.
                if r.ok {
                    if let Some(expect) = reference_rgba(&s) {
                        assert_eq!(r.pixels, expect, "[{label}] vs reference decoder");
                    }
                } else {
                    assert_eq!(
                        r.err.as_deref(),
                        Some("Stored block extends beyond end of input stream."),
                        "[{label}] unexpected rejection"
                    );
                }
            }
        }
    }
}

/// Row 67 — garbage chunk CRCs and a garbage adler32 are silently accepted.
#[test]
fn row_67_no_checksum_validation() {
    let mut rng = Rng::new(0x6701);
    for ct in COLOUR_TYPES {
        for iter in 0..6 {
            let (w, h) = (rng.range(1, 20), rng.range(1, 20));
            let mut s = Spec::new(w, h, ct);
            s.filters = (0..h).map(|_| rng.below(5) as u8).collect();
            s.payload = rng.bytes((w * h) as usize * bpp_of(ct) + 7);
            s.bad_crc = true;
            s.adler_override = Some(rng.u32());
            ok_png(&s, &format!("nocrc ct={ct} iter={iter}"));
        }
    }
}

/// Row 68 — IHDR longer than 13 bytes (still `>= minlen`).
#[test]
fn row_68_long_ihdr() {
    let mut rng = Rng::new(0x6801);
    for extra in [0usize, 1, 2, 7, 64, 255] {
        for ct in COLOUR_TYPES {
            let (w, h) = (rng.range(1, 16), rng.range(1, 16));
            let mut s = Spec::new(w, h, ct);
            s.filters = (0..h).map(|_| rng.below(5) as u8).collect();
            s.payload = rng.bytes((w * h) as usize * bpp_of(ct) + 7);
            s.ihdr_extra = extra;
            ok_png(&s, &format!("ihdr+{extra} ct={ct}"));
        }
    }
}
