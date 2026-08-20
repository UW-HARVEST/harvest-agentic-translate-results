//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Every assertion compares the C `.so` and the Rust `.so` through `dlsym`.

mod harness;

use harness::{flat_offset, make_header, Libs, Rng, ITERS};

/// Sanity: the two symbols really are two different implementations, i.e. we
/// are not accidentally comparing one library against itself (which would make
/// every test below vacuously pass).
#[test]
fn harness_loads_two_distinct_implementations() {
    let l = Libs::load();
    assert_ne!(
        l.c as usize, l.rust as usize,
        "C and Rust `hdr_bitrate` resolved to the SAME address — the differential \
         tests would be vacuous. C={}, Rust={}",
        l.c_path.display(),
        l.rust_path.display()
    );
    assert!(l.c_path != l.rust_path);
    eprintln!("C   : {}", l.c_path.display());
    eprintln!("Rust: {}", l.rust_path.display());
}

// ---------------------------------------------------------------------------
// Helper shared by the per-row tests C1..C7: sweep k over 0..=14 (the in-range
// bitrate indices) for one (i, layer) group, with randomized don't-care bits.
// ---------------------------------------------------------------------------
fn sweep_group(row: &str, i: u32, layer: u32, seed: u64) {
    let l = Libs::load();
    let mut rng = Rng::new(seed);
    let mut seen = Vec::new();

    for k in 0..=14u32 {
        for _ in 0..ITERS {
            let h = make_header(i, layer, k, &mut rng);
            let v = l.assert_eq_on(&h, &format!("{row} i={i} layer={layer} k={k}"));
            seen.push(v);
        }
    }

    // Every returned value must be an even multiple of 2 * table byte.
    for v in &seen {
        assert_eq!(v % 2, 0, "[{row}] result must be 2 * a table byte, got {v}");
        assert!(*v <= 448, "[{row}] result {v} exceeds 2 * 224");
    }
    eprintln!(
        "[{row}] i={i} layer={layer} j={} flat={}..{} — {} calls matched",
        layer as i32 - 1,
        flat_offset(i as i32, layer as i32, 0),
        flat_offset(i as i32, layer as i32, 14),
        seen.len()
    );
}

// C1..C6 — the six in-range (i, j) rows of `halfrate[2][3][15]`.

#[test]
fn config_c1_i0_j0_row() {
    sweep_group("C1", 0, 1, 0x_C1_00_0001);
}

#[test]
fn config_c2_i0_j1_row() {
    sweep_group("C2", 0, 2, 0x_C2_00_0002);
}

#[test]
fn config_c3_i0_j2_row() {
    sweep_group("C3", 0, 3, 0x_C3_00_0003);
}

#[test]
fn config_c4_i1_j0_row() {
    sweep_group("C4", 1, 1, 0x_C4_00_0004);
}

#[test]
fn config_c5_i1_j1_row() {
    sweep_group("C5", 1, 2, 0x_C5_00_0005);
}

#[test]
fn config_c6_i1_j2_row_contains_max_entry() {
    sweep_group("C6", 1, 3, 0x_C6_00_0006);

    // This row holds the largest table entry (224) at k = 14 -> 448.
    let l = Libs::load();
    let mut rng = Rng::new(0x_C6_FF);
    let h = make_header(1, 3, 14, &mut rng);
    let (c, r) = l.both(&h);
    assert_eq!(c, r);
    assert_eq!(c, 448, "C6 k=14 should be 2*224");
}

// C7 — out-of-range j (== -1) with i == 1: flat = 30 + k, still INSIDE the
// table, aliasing row halfrate[0][2].
#[test]
fn config_c7_reserved_layer_i1_aliases_in_table_row() {
    sweep_group("C7", 1, 0, 0x_C7_00_0007);

    let l = Libs::load();
    let mut rng = Rng::new(0x_C7_FF);
    // Must equal the (i=0, layer=3) row, which occupies the same flat bytes.
    for k in 0..=14u32 {
        let a = make_header(1, 0, k, &mut rng);
        let b = make_header(0, 3, k, &mut rng);
        let (ca, ra) = l.both(&a);
        let (cb, rb) = l.both(&b);
        assert_eq!(ca, ra, "C7 alias k={k}");
        assert_eq!(cb, rb, "C7 alias base k={k}");
        assert_eq!(
            ca, cb,
            "C7: (i=1,j=-1,k={k}) must alias (i=0,j=2,k={k}) in the C's flat layout"
        );
    }
}

// C8 — i=0, j=-1, k=15 -> flat == 0 exactly, aliasing halfrate[0][0][0].
#[test]
fn config_c8_reserved_layer_i0_k15_aliases_first_entry() {
    let l = Libs::load();
    let mut rng = Rng::new(0x_C8);
    for _ in 0..ITERS {
        let h = make_header(0, 0, 15, &mut rng);
        let v = l.assert_eq_on(&h, "C8 i=0 layer=0 k=15 flat=0");
        assert_eq!(v, 0, "C8: halfrate[0][0][0] is 0");
    }
    assert_eq!(flat_offset(0, 0, 15), 0);
}

// C9 — k == 15 for every (i, j): one past the last entry of the row.
#[test]
fn config_c9_bitrate_index_15_all_groups() {
    let l = Libs::load();
    let mut rng = Rng::new(0x_C9);
    for i in 0..=1u32 {
        for layer in 0..=3u32 {
            for _ in 0..ITERS {
                let h = make_header(i, layer, 15, &mut rng);
                l.assert_eq_on(&h, &format!("C9 i={i} layer={layer} k=15"));
            }
        }
    }
}

// C10 — boundary values of k (first and last in-range entry) for all 8 groups.
#[test]
fn config_c10_k_boundaries_all_groups() {
    let l = Libs::load();
    let mut rng = Rng::new(0x_C10);
    for i in 0..=1u32 {
        for layer in 0..=3u32 {
            for &k in &[0u32, 1, 13, 14] {
                for _ in 0..ITERS / 4 {
                    let h = make_header(i, layer, k, &mut rng);
                    let v = l.assert_eq_on(&h, &format!("C10 i={i} layer={layer} k={k}"));
                    if k == 0 {
                        // Every row of the table begins with 0.
                        assert_eq!(v, 0, "C10: k=0 is the 'free' index, table entry 0");
                    }
                }
            }
        }
    }
}

// C11 — all 128 index triples with randomized don't-care bits.
#[test]
fn config_c11_all_128_index_triples() {
    let l = Libs::load();
    let mut rng = Rng::new(0x_C11);
    let mut n = 0usize;
    for i in 0..=1u32 {
        for layer in 0..=3u32 {
            for k in 0..=15u32 {
                for _ in 0..ITERS / 4 {
                    let h = make_header(i, layer, k, &mut rng);
                    l.assert_eq_on(&h, &format!("C11 i={i} layer={layer} k={k}"));
                    n += 1;
                }
            }
        }
    }
    eprintln!("[C11] {n} calls across all 2*4*16 = 128 index triples matched");
}

// C12 — bits the C never reads must not change the result.
#[test]
fn config_c12_dont_care_bits_are_ignored() {
    let l = Libs::load();
    let mut rng = Rng::new(0x_C12);

    for i in 0..=1u32 {
        for layer in 0..=3u32 {
            for k in 0..=15u32 {
                // Reference: all don't-care bits zero.
                let base = [0u8, ((i as u8) << 3) | ((layer as u8) << 1), (k as u8) << 4, 0];
                let (bc, br) = l.both(&base);
                assert_eq!(bc, br, "C12 base i={i} layer={layer} k={k}");

                for _ in 0..32 {
                    let h = make_header(i, layer, k, &mut rng);
                    let (c, r) = l.both(&h);
                    assert_eq!(c, r, "C12 noisy i={i} layer={layer} k={k} h={h:02x?}");
                    assert_eq!(
                        c, bc,
                        "C12: don't-care bits changed the result for i={i} layer={layer} k={k}: \
                         base={bc} noisy={c} h={h:02x?}"
                    );
                }
            }
        }
    }
}

// C13 — EXHAUSTIVE over the entire reachable input space.
// hdr_bitrate reads only h[1] and h[2], so all 256*256 = 65536 pairs is the
// complete space of inputs that can change the result.
#[test]
fn config_exhaustive_all_65536_h1_h2_pairs() {
    let l = Libs::load();
    let mut rng = Rng::new(0xE0E0_1234_5678_9ABC);
    let mut mismatches = Vec::new();
    let mut c_all = Vec::with_capacity(65536);
    let mut r_all = Vec::with_capacity(65536);

    for h1 in 0..=255u8 {
        for h2 in 0..=255u8 {
            // h[0] and h[3] are never read -> seeded random each iteration.
            let noise = rng.next_u32();
            let h = [noise as u8, h1, h2, (noise >> 8) as u8];
            let (c, r) = l.both(&h);
            c_all.push(c);
            r_all.push(r);
            if c != r && mismatches.len() < 40 {
                mismatches.push((h1, h2, c, r));
            }
        }
    }

    assert!(
        mismatches.is_empty(),
        "EXHAUSTIVE DIVERGENCE ({} shown) (h1,h2,C,Rust): {mismatches:02x?}",
        mismatches.len()
    );
    assert_eq!(c_all, r_all, "exhaustive output vectors differ");
    assert_eq!(c_all.len(), 65536);
    eprintln!("[C13] all 65536 (h[1],h[2]) pairs matched byte-for-byte");
}

// C16 — the complete set of distinct return values must be identical.
#[test]
fn config_c16_full_output_value_set_matches() {
    let l = Libs::load();
    let mut c_set = std::collections::BTreeSet::new();
    let mut r_set = std::collections::BTreeSet::new();

    for h1 in 0..=255u8 {
        for h2 in 0..=255u8 {
            let h = [0u8, h1, h2, 0];
            let (c, r) = l.both(&h);
            c_set.insert(c);
            r_set.insert(r);
        }
    }

    assert_eq!(
        c_set, r_set,
        "the set of distinct results differs\nC   : {c_set:?}\nRust: {r_set:?}"
    );
    // All results are 2 * a byte from the table.
    for v in &c_set {
        assert_eq!(v % 2, 0);
        assert!(*v <= 448);
    }
    eprintln!("[C16] {} distinct return values, identical: {c_set:?}", c_set.len());
}

// C14 — pointer shape: offsets and every alignment inside a larger allocation.
#[test]
fn config_c14_pointer_offsets_and_alignments() {
    let l = Libs::load();
    let mut rng = Rng::new(0x_C14);
    let mut buf = vec![0u8; 128];

    for off in 0..16usize {
        for _ in 0..ITERS / 8 {
            let i = rng.below(2);
            let layer = rng.below(4);
            let k = rng.below(16);
            let h = make_header(i, layer, k, &mut rng);

            // Fill the whole buffer with noise, then plant the header at `off`.
            for b in buf.iter_mut() {
                *b = rng.next_u32() as u8;
            }
            buf[off..off + 4].copy_from_slice(&h);

            let p = unsafe { buf.as_ptr().add(off) };
            let (c, r) = unsafe { l.both_raw(p) };
            assert_eq!(
                c, r,
                "C14 off={off} align={} i={i} layer={layer} k={k}",
                p as usize % 8
            );

            // Must equal the same header in a standalone buffer.
            let (c2, _) = l.both(&h);
            assert_eq!(c, c2, "C14: result depended on pointer position (off={off})");
        }
    }
}

// C15 — statelessness / idempotence across repeated and interleaved calls.
#[test]
fn config_c15_stateless_and_idempotent() {
    let l = Libs::load();
    let mut rng = Rng::new(0x_C15);

    // Repeated identical calls.
    let h = make_header(1, 3, 14, &mut rng);
    let (first_c, first_r) = l.both(&h);
    assert_eq!(first_c, first_r);
    for _ in 0..1000 {
        let (c, r) = l.both(&h);
        assert_eq!((c, r), (first_c, first_r), "C15: not idempotent");
    }

    // Interleaved alternating inputs must not contaminate each other.
    let a = make_header(0, 1, 3, &mut rng);
    let b = make_header(1, 3, 12, &mut rng);
    let (ca, ra) = l.both(&a);
    let (cb, rb) = l.both(&b);
    assert_eq!(ca, ra);
    assert_eq!(cb, rb);
    for _ in 0..1000 {
        assert_eq!(l.both(&a), (ca, ra), "C15: interleaving changed result for a");
        assert_eq!(l.both(&b), (cb, rb), "C15: interleaving changed result for b");
    }

    // Fully random interleaving.
    for _ in 0..ITERS * 4 {
        let i = rng.below(2);
        let layer = rng.below(4);
        let k = rng.below(16);
        let h = make_header(i, layer, k, &mut rng);
        l.assert_eq_on(&h, "C15 random interleave");
    }
}
