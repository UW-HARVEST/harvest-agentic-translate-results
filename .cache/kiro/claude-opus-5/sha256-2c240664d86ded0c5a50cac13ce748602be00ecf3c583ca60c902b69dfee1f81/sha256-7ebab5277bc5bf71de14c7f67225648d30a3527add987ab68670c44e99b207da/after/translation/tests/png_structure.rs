//! Level 2b: chunk-walking (`cp_chunk`, `cp_find`, `cp_get_chunk_byte_length`)
//! and IHDR validation, plus a byte-mutation sweep over hand-built PNGs.

mod common;

use common::*;

fn cmp(label: &str, data: &[u8]) {
    compare_png_input(label, &PngInput::new(data));
}

fn cmp_gated(label: &str, data: &[u8]) -> bool {
    compare_png_input_if_deterministic(label, &PngInput::new(data))
}

fn rows(w: u32, h: u32, ct: u8) -> Vec<Vec<u8>> {
    let bpp = bpp_of(ct) as usize;
    (0..h)
        .map(|y| {
            let mut line = vec![0u8];
            for x in 0..w as usize * bpp {
                line.push(((x * 31 + y as usize * 17 + 7) & 0xFF) as u8);
            }
            line
        })
        .collect()
}

fn ihdr_bytes(w: u32, h: u32, depth: u8, ct: u8, comp: u8, filt: u8, inter: u8) -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(&w.to_be_bytes());
    v.extend_from_slice(&h.to_be_bytes());
    v.extend_from_slice(&[depth, ct, comp, filt, inter]);
    v
}

/// `cp_chunk` accepts an IHDR whose declared length is *larger* than 13 and
/// skips `len + 12` bytes; the payload is still read from `start + 8`.
#[test]
fn ihdr_length_variants() {
    let raw = rows(4, 3, 6);
    let payload: Vec<u8> = raw.iter().flatten().copied().collect();
    let z = zlib_stored(&payload);

    for extra in [0usize, 1, 7, 32] {
        let mut ih = ihdr_bytes(4, 3, 8, 6, 0, 0, 0);
        ih.extend(std::iter::repeat(0xEEu8).take(extra));
        let mut data = PNG_SIG.to_vec();
        data.extend_from_slice(&chunk(b"IHDR", &ih));
        data.extend_from_slice(&chunk(b"IDAT", &z));
        data.extend_from_slice(&chunk(b"IEND", b""));
        cmp(&format!("ihdr_len{}", 13 + extra), &data);
    }

    // len < 13 -> cp_chunk rejects (minlen check)
    for short in [0usize, 1, 12] {
        let ih = ihdr_bytes(4, 3, 8, 6, 0, 0, 0);
        let mut data = PNG_SIG.to_vec();
        data.extend_from_slice(&chunk(b"IHDR", &ih[..short]));
        data.extend_from_slice(&chunk(b"IDAT", &z));
        data.extend_from_slice(&chunk(b"IEND", b""));
        cmp(&format!("ihdr_short{short}"), &data);
    }

    // Declared length that wraps `len + 12` back into a small positive int, so
    // `cp_chunk` accepts the chunk and advances `png.p` into the middle of it.
    // (`len == 0xFFFFFFF4` is deliberately left out: `len + 12` is then 0 and
    // `cp_find` loops forever without advancing — the C original hangs, so
    // there is nothing to compare.)
    for len in [0xFFFF_FFFFu32, 0xFFFF_FFF5] {
        let mut data = PNG_SIG.to_vec();
        data.extend_from_slice(&len.to_be_bytes());
        data.extend_from_slice(b"IHDR");
        data.extend_from_slice(&ihdr_bytes(4, 3, 8, 6, 0, 0, 0));
        data.extend_from_slice(&[0, 0, 0, 0]);
        data.extend_from_slice(&chunk(b"IDAT", &z));
        data.extend_from_slice(&chunk(b"IEND", b""));
        cmp_gated(&format!("ihdr_wraplen{len:#x}"), &data);
    }
}

/// The `png->p + offset <= png->end` boundary in `cp_chunk` and the
/// `png->p <= png->end` boundary in `cp_find`, hit exactly.
#[test]
fn chunk_end_boundaries() {
    let raw = rows(3, 2, 0);
    let payload: Vec<u8> = raw.iter().flatten().copied().collect();
    let z = zlib_stored(&payload);
    let mut data = PNG_SIG.to_vec();
    data.extend_from_slice(&chunk(b"IHDR", &ihdr_bytes(3, 2, 8, 0, 0, 0, 0)));
    let after_ihdr = data.len();
    data.extend_from_slice(&chunk(b"IDAT", &z));
    let after_idat = data.len();
    data.extend_from_slice(&chunk(b"IEND", b""));

    // png_length exactly at each structural boundary
    for len in [
        after_ihdr - 1,
        after_ihdr,
        after_ihdr + 1,
        after_idat - 1,
        after_idat,
        after_idat + 1,
        data.len() - 1,
        data.len(),
    ] {
        let input = PngInput::with_len(&data, len);
        compare_png_input_if_deterministic(&format!("boundary_len{len}"), &input);
    }
}

/// PLTE / tRNS placement relative to IDAT. `load_png_mem` searches for PLTE
/// first, then tRNS, then IDAT, saving and restoring `png.p` in a way that
/// makes the *order* of the chunks observable.
#[test]
fn palette_chunk_placement() {
    let w = 6u32;
    let h = 4u32;
    let raw = rows(w, h, 3);
    let payload: Vec<u8> = raw.iter().flatten().copied().collect();
    let z = zlib_stored(&payload);
    let plte: Vec<u8> = (0..768).map(|i| ((i * 5 + 1) & 0xFF) as u8).collect();
    let trns: Vec<u8> = (0..32).map(|i| ((i * 9 + 2) & 0xFF) as u8).collect();

    let ihdr = chunk(b"IHDR", &ihdr_bytes(w, h, 8, 3, 0, 0, 0));
    let cp = chunk(b"PLTE", &plte);
    let ct = chunk(b"tRNS", &trns);
    let ci = chunk(b"IDAT", &z);
    let ce = chunk(b"IEND", b"");

    let orders: [(&str, Vec<&Vec<u8>>); 8] = [
        ("plte_trns_idat", vec![&cp, &ct, &ci]),
        ("trns_plte_idat", vec![&ct, &cp, &ci]),
        ("plte_idat_trns", vec![&cp, &ci, &ct]),
        ("trns_idat_plte", vec![&ct, &ci, &cp]),
        ("idat_plte_trns", vec![&ci, &cp, &ct]),
        ("idat_trns_plte", vec![&ci, &ct, &cp]),
        ("plte_only", vec![&cp, &ci]),
        ("trns_only", vec![&ct, &ci]),
    ];
    for (name, parts) in orders {
        let mut data = PNG_SIG.to_vec();
        data.extend_from_slice(&ihdr);
        for p in parts {
            data.extend_from_slice(p);
        }
        data.extend_from_slice(&ce);
        cmp_gated(&format!("order_{name}"), &data);
    }

    // duplicated PLTE / tRNS
    let mut data = PNG_SIG.to_vec();
    data.extend_from_slice(&ihdr);
    data.extend_from_slice(&cp);
    data.extend_from_slice(&chunk(b"PLTE", &vec![0x7Fu8; 768]));
    data.extend_from_slice(&ct);
    data.extend_from_slice(&chunk(b"tRNS", &vec![0x11u8; 8]));
    data.extend_from_slice(&ci);
    data.extend_from_slice(&ce);
    cmp_gated("dup_plte_trns", &data);
}

/// Several IDAT chunks, IDATs separated by other chunks (which makes
/// `cp_chunk` stop early), and zero-length IDATs.
#[test]
fn idat_gathering() {
    let w = 5u32;
    let h = 4u32;
    let raw = rows(w, h, 2);
    let payload: Vec<u8> = raw.iter().flatten().copied().collect();
    let z = zlib_stored(&payload);
    let ihdr = chunk(b"IHDR", &ihdr_bytes(w, h, 8, 2, 0, 0, 0));
    let ce = chunk(b"IEND", b"");

    for nparts in 1..=6usize {
        let per = (z.len() + nparts - 1) / nparts;
        let mut data = PNG_SIG.to_vec();
        data.extend_from_slice(&ihdr);
        for c in z.chunks(per.max(1)) {
            data.extend_from_slice(&chunk(b"IDAT", c));
        }
        data.extend_from_slice(&ce);
        cmp_gated(&format!("idat_split{nparts}"), &data);
    }

    // a foreign chunk between two IDATs: `cp_chunk` stops, so only the first
    // run of IDATs is gathered
    let half = z.len() / 2;
    let mut data = PNG_SIG.to_vec();
    data.extend_from_slice(&ihdr);
    data.extend_from_slice(&chunk(b"IDAT", &z[..half]));
    data.extend_from_slice(&chunk(b"junk", b"xyz"));
    data.extend_from_slice(&chunk(b"IDAT", &z[half..]));
    data.extend_from_slice(&ce);
    cmp_gated("idat_interrupted", &data);

    // zero-length IDATs before / after the real one
    let mut data = PNG_SIG.to_vec();
    data.extend_from_slice(&ihdr);
    data.extend_from_slice(&chunk(b"IDAT", b""));
    data.extend_from_slice(&chunk(b"IDAT", &z));
    data.extend_from_slice(&chunk(b"IDAT", b""));
    data.extend_from_slice(&ce);
    cmp_gated("idat_empty_around", &data);

    // only empty IDATs -> datalen 0
    let mut data = PNG_SIG.to_vec();
    data.extend_from_slice(&ihdr);
    data.extend_from_slice(&chunk(b"IDAT", b""));
    data.extend_from_slice(&ce);
    cmp_gated("idat_only_empty", &data);
}

/// IHDR dimension validation: `w = make32(ihdr) + 1` (so the stored width is
/// one less than what the code uses), the `w >= 1` / `h >= 1` checks and the
/// `(int64_t)w * h * 4 < INT_MAX` overflow check.
#[test]
fn dimension_validation() {
    let z = zlib_stored(&[0u8; 16]);
    let mk = |w: u32, h: u32, ct: u8| {
        let mut data = PNG_SIG.to_vec();
        data.extend_from_slice(&chunk(b"IHDR", &ihdr_bytes(w, h, 8, ct, 0, 0, 0)));
        data.extend_from_slice(&chunk(b"IDAT", &z));
        data.extend_from_slice(&chunk(b"IEND", b""));
        data
    };
    // width: stored value + 1 must be >= 1 as a signed int
    for w in [
        0xFFFF_FFFFu32, // -> 0        -> width < 1
        0x7FFF_FFFF,    // -> INT_MIN  -> width < 1
        0x8000_0000,    // -> negative -> width < 1
        0xFFFF_FFFE,    // -> -1       -> width < 1
    ] {
        cmp_gated(&format!("w{w:#x}"), &mk(w, 4, 6));
    }
    // height
    for h in [0u32, 0x8000_0000, 0xFFFF_FFFF, 0x7FFF_FFFF] {
        cmp_gated(&format!("h{h:#x}"), &mk(4, h, 6));
    }
    // (w+1) * h * 4 >= INT_MAX -> "image too large"
    for (w, h) in [
        (0xFFFFu32, 0xFFFFu32),
        (0x1FFF_FFFF, 1),
        (0x2000_0000, 1),
        (0x0FFF_FFFF, 8),
        (100_000, 100_000),
    ] {
        cmp_gated(&format!("big_{w:#x}x{h:#x}"), &mk(w, h, 6));
    }
    // just under the limit, but with no usable IDAT so nothing is decoded
    for (w, h) in [(0x0FFF_FFFEu32, 1u32), (0x03FF_FFFE, 4)] {
        let mut data = PNG_SIG.to_vec();
        data.extend_from_slice(&chunk(b"IHDR", &ihdr_bytes(w, h, 8, 6, 0, 0, 0)));
        data.extend_from_slice(&chunk(b"IEND", b""));
        cmp_gated(&format!("near_limit_{w:#x}x{h:#x}"), &data);
    }
}

/// Every bit depth and every colour type byte.
///
/// The colour-type sweep needs a deliberately small DEFLATE payload: changing
/// the colour type changes `bpp`, which moves `out` forward inside the
/// `malloc(pix_bytes)` buffer while `cp_stored` copies `LEN` bytes with no
/// output bounds check at all. A payload longer than `(w+1) * h * 1` would make
/// the C code corrupt the heap for `bpp == 1`, which would take the test process
/// down rather than produce a comparable result.
#[test]
fn header_field_sweep() {
    let (w, h) = (4u32, 3u32);
    let raw = rows(w, h, 6);
    let payload: Vec<u8> = raw.iter().flatten().copied().collect();
    let z_rgba = zlib_stored(&payload);
    // safe for any bpp: (w + 1) * h bytes
    let small: Vec<u8> = (0..((w + 1) * h) as usize)
        .map(|i| ((i * 23 + 5) & 0xFF) as u8)
        .collect();
    let z_small = zlib_stored(&small);

    let mk = |depth: u8, ct: u8, comp: u8, filt: u8, inter: u8, z: &[u8]| {
        let mut data = PNG_SIG.to_vec();
        data.extend_from_slice(&chunk(b"IHDR", &ihdr_bytes(w, h, depth, ct, comp, filt, inter)));
        data.extend_from_slice(&chunk(b"IDAT", z));
        data.extend_from_slice(&chunk(b"IEND", b""));
        data
    };
    for depth in 0..=255u8 {
        cmp_gated(&format!("depth{depth}"), &mk(depth, 6, 0, 0, 0, &z_rgba));
    }
    for ct in 0..=255u8 {
        cmp_gated(&format!("ct{ct}"), &mk(8, ct, 0, 0, 0, &z_small));
        // colour type 3 additionally needs a palette
        if ct == 3 {
            let plte: Vec<u8> = (0..768).map(|i| ((i * 3 + 7) & 0xFF) as u8).collect();
            let mut data = PNG_SIG.to_vec();
            data.extend_from_slice(&chunk(b"IHDR", &ihdr_bytes(w, h, 8, 3, 0, 0, 0)));
            data.extend_from_slice(&chunk(b"PLTE", &plte));
            data.extend_from_slice(&chunk(b"IDAT", &z_small));
            data.extend_from_slice(&chunk(b"IEND", b""));
            cmp_gated("ct3_with_plte", &data);
        }
    }
    for v in 0..=255u8 {
        cmp_gated(&format!("comp{v}"), &mk(8, 6, v, 0, 0, &z_rgba));
        cmp_gated(&format!("filt{v}"), &mk(8, 6, 0, v, 0, &z_rgba));
        cmp_gated(&format!("inter{v}"), &mk(8, 6, 0, 0, v, &z_rgba));
    }
}

/// Every byte of the 8-byte signature, wrong in turn.
#[test]
fn signature_sweep() {
    let raw = rows(2, 2, 6);
    let payload: Vec<u8> = raw.iter().flatten().copied().collect();
    let z = zlib_stored(&payload);
    let mut good = PNG_SIG.to_vec();
    good.extend_from_slice(&chunk(b"IHDR", &ihdr_bytes(2, 2, 8, 6, 0, 0, 0)));
    good.extend_from_slice(&chunk(b"IDAT", &z));
    good.extend_from_slice(&chunk(b"IEND", b""));
    cmp("sig_good", &good);
    for i in 0..8usize {
        for delta in [1u8, 0xFF, 0x80] {
            let mut d = good.clone();
            d[i] ^= delta;
            cmp(&format!("sig_bad{i}_{delta}"), &d);
        }
    }
}

/// zlib header validation: compression method, window size and the preset
/// dictionary flag, swept exhaustively over both header bytes.
#[test]
fn zlib_header_sweep() {
    let raw = rows(3, 3, 6);
    let payload: Vec<u8> = raw.iter().flatten().copied().collect();
    let z = zlib_stored(&payload);
    let ihdr = chunk(b"IHDR", &ihdr_bytes(3, 3, 8, 6, 0, 0, 0));
    for b0 in 0..=255u8 {
        let mut zz = z.clone();
        zz[0] = b0;
        let mut data = PNG_SIG.to_vec();
        data.extend_from_slice(&ihdr);
        data.extend_from_slice(&chunk(b"IDAT", &zz));
        data.extend_from_slice(&chunk(b"IEND", b""));
        cmp_gated(&format!("zlib_b0_{b0}"), &data);
    }
    for b1 in 0..=255u8 {
        let mut zz = z.clone();
        zz[1] = b1;
        let mut data = PNG_SIG.to_vec();
        data.extend_from_slice(&ihdr);
        data.extend_from_slice(&chunk(b"IDAT", &zz));
        data.extend_from_slice(&chunk(b"IEND", b""));
        cmp_gated(&format!("zlib_b1_{b1}"), &data);
    }
    // datalen < 6
    for n in 0..6usize {
        let mut data = PNG_SIG.to_vec();
        data.extend_from_slice(&ihdr);
        data.extend_from_slice(&chunk(b"IDAT", &z[..n]));
        data.extend_from_slice(&chunk(b"IEND", b""));
        cmp_gated(&format!("zlib_shortdata{n}"), &data);
    }
}

/// Single-byte mutation sweep over hand-built PNGs.
///
/// The DEFLATE payload is a *stored* block, so no mutation can produce a
/// corrupt Huffman table (which the reference C library answers with a live
/// `assert()` rather than a return value). The IHDR width/height fields and the
/// DEFLATE block-header byte are excluded for the same reason: they would make
/// the C code allocate gigabytes or switch to Huffman decoding.
#[test]
fn single_byte_mutations() {
    let mut compared = 0usize;
    let mut skipped = 0usize;
    for ct in [0u8, 2, 3, 4, 6] {
        let (w, h) = (5u32, 3u32);
        let raw = rows(w, h, ct);
        let plte: Option<Vec<u8>> = if ct == 3 {
            Some((0..768).map(|i| ((i * 3 + 5) & 0xFF) as u8).collect())
        } else {
            None
        };
        let trns: Option<Vec<u8>> = if ct == 3 {
            Some((0..16).map(|i| ((i * 7 + 1) & 0xFF) as u8).collect())
        } else {
            None
        };
        let base = build_png_raw(w, h, ct, &raw, plte.as_deref(), trns.as_deref());

        // Offset of the DEFLATE block-header byte inside the file: signature +
        // IHDR chunk + optional PLTE/tRNS chunks + IDAT header + 2 zlib bytes.
        let mut idat_payload_off = 8 + 25;
        if let Some(p) = plte.as_deref() {
            idat_payload_off += 12 + p.len();
        }
        if let Some(t) = trns.as_deref() {
            idat_payload_off += 12 + t.len();
        }
        idat_payload_off += 8;
        let block_header_off = idat_payload_off + 2;

        // Offsets of the two most significant bytes of every chunk length
        // field. `cp_chunk` stores `len + 12` in an `int`, so a length above
        // 0x7FFFFFF4 makes it negative, `png->p + offset <= png->end` still
        // passes and `png.p` is moved ~2 GiB backwards — the C original then
        // dereferences a wild pointer and segfaults.
        let mut excluded: Vec<usize> = Vec::new();
        {
            let mut q = 8usize;
            while q + 8 <= base.len() {
                excluded.push(q);
                excluded.push(q + 1);
                let clen =
                    u32::from_be_bytes([base[q], base[q + 1], base[q + 2], base[q + 3]]) as usize;
                q += clen + 12;
            }
        }

        for off in 0..base.len() {
            // 16..24: IHDR width/height — a mutation there can make the C code
            // ask for gigabytes or spin over a gigantic buffer.
            // 25: IHDR colour type — changes `bpp`, which moves `out` forward
            // while `cp_stored` still copies the original `LEN` bytes, smashing
            // the heap (covered safely by `header_field_sweep` instead).
            // `block_header_off`: switches the DEFLATE block to Huffman coding,
            // which the reference library answers with a live `assert()`.
            if off == block_header_off || (16..26).contains(&off) || excluded.contains(&off) {
                continue;
            }
            for delta in [0x01u8, 0x80, 0xFF] {
                let mut d = base.clone();
                d[off] ^= delta;
                let label = format!("mut_ct{ct}_off{off}_x{delta:02x}");
                if std::env::var_os("MUT_TRACE").is_some() {
                    use std::io::Write;
                    let mut e = std::io::stderr();
                    let _ = writeln!(e, "{label}");
                    let _ = e.flush();
                }
                if cmp_gated(&label, &d) {
                    compared += 1;
                } else {
                    skipped += 1;
                }
            }
        }
    }
    println!("single_byte_mutations: {compared} compared, {skipped} heap-dependent");
    assert!(compared > 1000, "too few comparable mutations: {compared}");
}
