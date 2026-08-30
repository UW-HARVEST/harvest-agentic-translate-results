//! Level 2: `load_png_mem` — the public API, exercising `cp_chunk`, `cp_find`,
//! `cp_inflate`, `cp_unfilter`, `cp_paeth`, `cp_convert`, `cp_depalette` and
//! `cp_get_alpha_for_indexed_image`.

mod common;

use common::*;

fn compare(label: &str, data: &[u8]) -> usize {
    compare_png_input(label, &PngInput::new(data))
}

fn compare_len(label: &str, data: &[u8], len: usize) -> usize {
    compare_png_input(label, &PngInput::with_len(data, len))
}

/// Every generated fixture: all colour types, all filter types, palettes,
/// tRNS variants, multi-IDAT, compression strategies and the error cases.
/// Complete images must be fully deterministic, i.e. nothing may be skipped.
#[test]
fn all_png_fixtures() {
    let fixtures = png_fixtures();
    assert!(fixtures.len() > 100, "fixtures missing; run gen_fixtures.py");
    for (name, data) in fixtures {
        let skipped = compare(&name, &data);
        if !name.starts_with("err_") {
            assert_eq!(skipped, 0, "{name}: {skipped} pixels came from uninitialised heap");
        }
    }
}

/// The same fixtures fed twice in a row: `cp_error_reason` is a sticky global
/// in both libraries and must be updated (or left alone) identically.
#[test]
fn repeated_calls_keep_error_reason_in_sync() {
    let p = pair();
    for (name, data) in png_fixtures() {
        let input = PngInput::new(&data);
        for round in 0..2 {
            let c = run_load_png(&p.c, &input);
            let r = run_load_png(&p.rs, &input);
            assert_eq!(c.w, r.w, "{name} round{round}: w");
            assert_eq!(c.h, r.h, "{name} round{round}: h");
            assert_eq!(c.null, r.null, "{name} round{round}: null");
            assert_eq!(
                c.err.as_deref().map(String::from_utf8_lossy),
                r.err.as_deref().map(String::from_utf8_lossy),
                "{name} round{round}: cp_error_reason"
            );
        }
    }
}

/// Truncating a PNG usually makes `cp_find`/`cp_chunk` reject the incomplete
/// chunk, and both implementations must then reject at exactly the same point
/// with the same reason.
///
/// Some cut points leave a *complete* IDAT chunk holding a truncated stored
/// DEFLATE block; `cp_stored` then `memcpy`s `LEN` bytes out of the input
/// buffer without any bounds check and the C result depends on unrelated heap
/// contents. Those cases are detected and skipped — see
/// `compare_png_input_if_deterministic`.
#[test]
fn truncated_pngs() {
    let mut compared = 0usize;
    let mut skipped = 0usize;
    for (name, data) in png_fixtures() {
        if name.starts_with("err_") {
            continue;
        }
        let n = data.len();
        let mut cuts: Vec<usize> = vec![8, 9, 12, 16, 20, 24, 25, 32, 33];
        for d in 1..=8usize {
            if n >= d {
                cuts.push(n - d);
            }
        }
        cuts.push(n / 2);
        cuts.push(n / 4);
        cuts.push(3 * n / 4);
        cuts.retain(|&c| c <= n && c >= 8);
        cuts.sort();
        cuts.dedup();
        for cut in cuts {
            let input = PngInput::with_len(&data, cut);
            if compare_png_input_if_deterministic(&format!("{name} truncated@{cut}"), &input) {
                compared += 1;
            } else {
                skipped += 1;
            }
        }
    }
    println!("truncated_pngs: {compared} compared, {skipped} heap-dependent (skipped)");
    assert!(compared > 500, "too few comparable truncation cases: {compared}");
}

/// `png_length` larger than the real data (the slack is zero-filled and shared
/// by both libraries) and `png_length` values that stop mid-chunk.
#[test]
fn oversized_length() {
    for (name, data) in png_fixtures().into_iter().take(40) {
        for extra in [1usize, 4, 16, 63] {
            compare_len(
                &format!("{name} len+{extra}"),
                &data,
                data.len() + extra,
            );
        }
    }
}

/// Width/height combinations around the interesting boundaries, generated on
/// the fly for every colour type. `cp_out_size` multiplies `(w+1)*h*bpp`, so
/// 1-pixel-wide and 1-pixel-tall images are the fiddly cases.
#[test]
fn size_matrix() {
    for ct in [0u8, 2, 3, 4, 6] {
        for w in [1u32, 2, 3, 4, 5, 7, 8, 9, 15, 16, 17] {
            for h in [1u32, 2, 3, 5, 8, 9] {
                let data = synth_png(w, h, ct, (w * 31 + h * 7 + ct as u32) as u64);
                compare(&format!("synth_ct{ct}_{w}x{h}"), &data);
            }
        }
    }
}

/// Palette indices are used unchecked as `plte[c*3 .. c*3+2]`, and the tRNS
/// lookup compares `(uint32_t)index >= trns_len`. Walk every index against
/// every tRNS length boundary.
#[test]
fn palette_index_and_trns_boundaries() {
    for trns_len in [0usize, 1, 2, 3, 5, 16, 17, 255, 256] {
        // one row containing all 256 indices
        let rows: Vec<Vec<u8>> = vec![(0..=255u8).collect()];
        let mut plte = Vec::with_capacity(768);
        for i in 0..256 {
            plte.push((i * 3 + 1) as u8);
            plte.push((i * 5 + 2) as u8);
            plte.push((i * 7 + 3) as u8);
        }
        let trns: Vec<u8> = (0..trns_len).map(|i| ((i * 11 + 3) & 0xFF) as u8).collect();
        let data = build_png_raw(
            256,
            1,
            3,
            &rows.iter().map(|r| filter0(r)).collect::<Vec<_>>(),
            Some(&plte),
            Some(&trns),
        );
        compare(&format!("plte_all_idx_trns{trns_len}"), &data);
    }
    // tRNS present but PLTE absent -> error path
    let rows = vec![filter0(&(0..8u8).collect::<Vec<u8>>())];
    let data = build_png_raw(8, 1, 3, &rows, None, Some(&[1, 2, 3]));
    compare("plte_missing_with_trns", &data);
}

/// A palette shorter than 768 bytes: the C code indexes it unchecked. Both
/// implementations read the same out-of-bounds bytes from the same buffer, so
/// they must still agree.
#[test]
fn short_palette() {
    for plte_entries in [1usize, 2, 4, 8] {
        let mut plte = Vec::new();
        for i in 0..plte_entries {
            plte.push((i * 3 + 1) as u8);
            plte.push((i * 5 + 2) as u8);
            plte.push((i * 7 + 3) as u8);
        }
        let rows: Vec<u8> = (0..8u8).map(|i| i % plte_entries as u8).collect();
        let data = build_png_raw(8, 1, 3, &[filter0(&rows)], Some(&plte), None);
        compare(&format!("short_plte{plte_entries}"), &data);
    }
}

/// Every filter byte value 0..=255 on the first and on a later scanline. Values
/// above 4 must produce the "invalid filter byte found" error identically.
#[test]
fn all_filter_bytes() {
    for ct in [0u8, 2, 6] {
        let bpp = bpp_of(ct) as usize;
        let (w, h) = (6usize, 4usize);
        for fb in 0..=255u8 {
            for row in [0usize, 2] {
                let mut raw: Vec<Vec<u8>> = Vec::new();
                for y in 0..h {
                    let mut line = vec![if y == row { fb } else { 0u8 }];
                    for x in 0..w * bpp {
                        line.push(((x * 17 + y * 5 + 1) & 0xFF) as u8);
                    }
                    raw.push(line);
                }
                let data = build_png_raw(w as u32, h as u32, ct, &raw, None, None);
                compare(&format!("filterbyte_ct{ct}_{fb}_row{row}"), &data);
            }
        }
    }
}
