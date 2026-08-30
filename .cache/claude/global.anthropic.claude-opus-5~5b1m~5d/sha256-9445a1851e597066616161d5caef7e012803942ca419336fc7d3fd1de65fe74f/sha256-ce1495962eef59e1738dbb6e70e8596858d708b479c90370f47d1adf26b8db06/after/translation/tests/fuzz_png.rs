//! Randomized differential fuzzing of the `load_png_mem` container parser.

mod fuzzcommon;
mod harness;

use fuzzcommon::*;
use harness::make::*;
use harness::*;

/// Rounds of `800` cases each. Override with `FUZZ_ROUNDS=<n>` for a longer run.
fn rounds() -> u32 {
    std::env::var("FUZZ_ROUNDS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3)
}

/// Fuzzes the PNG container: signature, chunk headers, IHDR fields, chunk
/// ordering and the IDAT payload.
#[test]
fn fuzz_load_png_mem() {
    let pair = load_pair();
    let mut rng = Rng::new(0xC0DE);

    // a corpus of valid PNGs across every colour type
    let mut base: Vec<Vec<u8>> = Vec::new();
    for &ct in &[0u8, 2, 3, 4, 6] {
        let bpp = bpp_of(ct);
        for &(w, h) in &[(1usize, 1usize), (4, 3), (7, 5)] {
            for parts in [1usize, 2] {
                let filters: Vec<u8> = (0..h).map(|_| rng.below(5) as u8).collect();
                let raw = raw_scanlines(&mut rng, w, h, bpp, &filters);
                let mut spec = PngSpec::new(w as u32, h as u32, ct, deflate_literals(&raw));
                spec.idat_parts = parts;
                if ct == 3 {
                    spec.plte = Some(rng.bytes(768));
                    let tn = rng.below(200) as usize;
                    spec.trns = Some(rng.bytes(tn));
                }
                base.push(spec.build());
            }
        }
    }

    let mut total_compared = 0usize;
    let mut total_dropped = 0usize;
    for round in 0..rounds() {
        let mut cases = Vec::new();
        for i in 0..800 {
            let b = &base[rng.below(base.len() as u32) as usize];
            let m = clamp_image_size(mutate(&mut rng, b), b);
            let len = match rng.below(8) {
                0 => rng.below(m.len().max(1) as u32) as i32,
                1 => -(rng.below(64) as i32),
                _ => m.len() as i32,
            };
            let mut c = Case::png_len(format!("png r{round} #{i}"), m, len);
            c.digest = true;
            cases.push(c);
        }
        let rep = fuzz_same(&pair, &cases);
        eprintln!("  round {round}: compared {} dropped {}", rep.compared, rep.dropped);
        total_compared += rep.compared;
        total_dropped += rep.dropped;
    }
    eprintln!("fuzz_load_png_mem: compared {total_compared}, dropped {total_dropped}");
    assert!(total_compared > 700, "too few deterministic cases");
}
