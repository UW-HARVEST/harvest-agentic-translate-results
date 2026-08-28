//! Differential tests: every call goes through `dlopen`+`dlsym` on both the C
//! reference `.so` and the Rust `.so`. No Rust function is invoked directly.

mod common;

use common::*;
use std::ffi::c_int;

// ---------------------------------------------------------------------------
// Header helpers
// ---------------------------------------------------------------------------

fn is_mpeg1(hdr: &[u8; 4]) -> bool {
    hdr[1] & 0x8 != 0
}

fn gr_count(hdr: &[u8; 4]) -> u32 {
    let base = if hdr[3] & 0xC0 == 0xC0 { 1 } else { 2 };
    if is_mpeg1(hdr) { base * 2 } else { base }
}

fn sr_idx(hdr: &[u8; 4]) -> i32 {
    let raw = (((hdr[2] >> 2) & 3) as i32)
        + ((((hdr[1] >> 3) & 1) + ((hdr[1] >> 4) & 1)) as i32) * 3;
    raw - (raw != 0) as i32
}

/// The 16 distinct `(hdr[1] & 0x18, hdr[2] & 0x0C)` combinations, which together
/// cover every reachable `sr_idx` (0..=8) in both the MPEG1 and non-MPEG1 path.
fn header_matrix() -> Vec<[u8; 4]> {
    let mut v = Vec::new();
    for h1 in [0x00u8, 0x08, 0x10, 0x18] {
        for h2 in [0x00u8, 0x04, 0x08, 0x0C] {
            for h3 in [0x00u8, 0xC0] {
                v.push([0xFF, h1, h2, h3]);
            }
        }
    }
    v
}

// ---------------------------------------------------------------------------
// A side-info bitstream builder mirroring read_side_info's field layout
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
struct Gran {
    part_23_length: u32,
    big_values: u32,
    global_gain: u32,
    scalefac_compress: u32,
    window_switching: u32,
    block_type: u32,
    mixed: u32,
    tables: u32,
    subblock_gain: [u32; 3],
    region_count: [u32; 2],
    preflag: u32,
    scalefac_scale: u32,
    count1_table: u32,
}

impl Default for Gran {
    fn default() -> Self {
        Gran {
            part_23_length: 0,
            big_values: 0,
            global_gain: 0,
            scalefac_compress: 0,
            window_switching: 0,
            block_type: 0,
            mixed: 0,
            tables: 0,
            subblock_gain: [0; 3],
            region_count: [0, 0],
            preflag: 0,
            scalefac_scale: 0,
            count1_table: 0,
        }
    }
}

fn build(hdr: &[u8; 4], main_data_begin: u32, scfsi: u32, grans: &[Gran]) -> Vec<u8> {
    let gc = gr_count(hdr);
    let mut w = BitWriter::new();
    if is_mpeg1(hdr) {
        w.put(9, main_data_begin);
        w.put(7 + gc, scfsi);
    } else {
        w.put(8 + gc, main_data_begin << gc);
    }
    for g in grans.iter().take(gc as usize) {
        w.put(12, g.part_23_length);
        w.put(9, g.big_values);
        w.put(8, g.global_gain);
        w.put(if is_mpeg1(hdr) { 4 } else { 9 }, g.scalefac_compress);
        w.put(1, g.window_switching);
        if g.window_switching != 0 {
            w.put(2, g.block_type);
            w.put(1, g.mixed);
            w.put(10, g.tables);
            w.put(3, g.subblock_gain[0]);
            w.put(3, g.subblock_gain[1]);
            w.put(3, g.subblock_gain[2]);
        } else {
            w.put(15, g.tables);
            w.put(4, g.region_count[0]);
            w.put(3, g.region_count[1]);
        }
        if is_mpeg1(hdr) {
            w.put(1, g.preflag);
        }
        w.put(1, g.scalefac_scale);
        w.put(1, g.count1_table);
    }
    w.finish(128)
}

// ---------------------------------------------------------------------------
// 1. get_bits (static in C) exercised through main_data_begin and bs->pos
// ---------------------------------------------------------------------------

#[test]
fn get_bits_bit_alignment_and_limits() {
    let p = pair();
    let mut rng = Rng(0x1234_5678_9ABC_DEF1);
    for _ in 0..40 {
        let buf = rng.bytes(64);
        for hdr in header_matrix() {
            for pos in 0..24i32 {
                // sweep every limit near the first few reads so the early-out
                // `return 0` path is hit at each field boundary
                for limit in [0, 1, pos, pos + 1, pos + 8, pos + 9, pos + 10, pos + 11, 512] {
                    p.check("get_bits", &buf, pos, limit, &hdr);
                }
            }
        }
    }
}

#[test]
fn get_bits_all_start_offsets() {
    let p = pair();
    let mut rng = Rng(0xDEAD_BEEF_0000_0001);
    for _ in 0..20 {
        let buf = rng.bytes(64);
        for hdr in header_matrix() {
            for pos in 0..64i32 {
                p.check("offsets", &buf, pos, 512, &hdr);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 2. Scalefactor-band table selection and contents
// ---------------------------------------------------------------------------

#[test]
fn sfb_table_rows_match() {
    let p = pair();
    for hdr in header_matrix() {
        let idx = sr_idx(&hdr);
        if idx > 7 {
            continue; // C indexes past the array end; contents are not defined
        }
        // long block
        let g = Gran::default();
        let buf = build(&hdr, 0, 0, &[g; 4]);
        let limit = (buf.len() * 8) as c_int;
        assert_eq!(
            p.c.read_row(&buf, 0, limit, &hdr, 23),
            p.rs.read_row(&buf, 0, limit, &hdr, 23),
            "g_scf_long row {idx} (hdr {hdr:02X?})"
        );
        // short block
        let g = Gran {
            window_switching: 1,
            block_type: 2,
            mixed: 0,
            ..Default::default()
        };
        let buf = build(&hdr, 0, 0, &[g; 4]);
        assert_eq!(
            p.c.read_row(&buf, 0, limit, &hdr, 40),
            p.rs.read_row(&buf, 0, limit, &hdr, 40),
            "g_scf_short row {idx} (hdr {hdr:02X?})"
        );
        // mixed block
        let g = Gran {
            window_switching: 1,
            block_type: 2,
            mixed: 1,
            ..Default::default()
        };
        let buf = build(&hdr, 0, 0, &[g; 4]);
        assert_eq!(
            p.c.read_row(&buf, 0, limit, &hdr, 40),
            p.rs.read_row(&buf, 0, limit, &hdr, 40),
            "g_scf_mixed row {idx} (hdr {hdr:02X?})"
        );
    }
}

// ---------------------------------------------------------------------------
// 3. Structured side info covering every branch
// ---------------------------------------------------------------------------

#[test]
fn structured_all_branches() {
    let p = pair();
    let mut cases: Vec<Gran> = Vec::new();
    for ws in [0u32, 1] {
        for bt in 0..4u32 {
            for mixed in [0u32, 1] {
                for bv in [0u32, 1, 288, 289, 511] {
                    cases.push(Gran {
                        window_switching: ws,
                        block_type: bt,
                        mixed,
                        big_values: bv,
                        part_23_length: 0x555,
                        global_gain: 0xA7,
                        scalefac_compress: 499,
                        tables: 0x2AB,
                        subblock_gain: [1, 5, 7],
                        region_count: [9, 5],
                        preflag: 1,
                        scalefac_scale: 1,
                        count1_table: 1,
                        ..Default::default()
                    });
                }
            }
        }
    }
    // scalefac_compress straddling the 500 threshold used for preflag
    for sc in [0u32, 255, 499, 500, 501, 511] {
        cases.push(Gran {
            scalefac_compress: sc,
            ..Default::default()
        });
    }
    for hdr in header_matrix() {
        for g in &cases {
            for mdb in [0u32, 1, 7, 255, 511] {
                for scfsi in [0u32, 0xFFFF, 0x0F0F, 0x5A5A, 0x7FF] {
                    let buf = build(&hdr, mdb, scfsi, &[*g; 4]);
                    let full = (buf.len() * 8) as c_int;
                    for limit in [full, 40, 60, 100, 8] {
                        p.check("structured", &buf, 0, limit, &hdr);
                    }
                }
            }
        }
    }
}

#[test]
fn structured_heterogeneous_granules() {
    let p = pair();
    let mut rng = Rng(0xABCD_1234_5678_9999);
    let mk = |rng: &mut Rng| Gran {
        part_23_length: rng.below(4096),
        big_values: rng.below(512),
        global_gain: rng.below(256),
        scalefac_compress: rng.below(512),
        window_switching: rng.below(2),
        block_type: rng.below(4),
        mixed: rng.below(2),
        tables: rng.below(1 << 15),
        subblock_gain: [rng.below(8), rng.below(8), rng.below(8)],
        region_count: [rng.below(16), rng.below(8)],
        preflag: rng.below(2),
        scalefac_scale: rng.below(2),
        count1_table: rng.below(2),
    };
    for hdr in header_matrix() {
        for _ in 0..400 {
            let grans = [mk(&mut rng), mk(&mut rng), mk(&mut rng), mk(&mut rng)];
            let mdb = rng.below(512);
            let scfsi = rng.next_u32() & 0x7FF;
            let buf = build(&hdr, mdb, scfsi, &grans);
            let full = (buf.len() * 8) as c_int;
            let limit = match rng.below(4) {
                0 => full,
                1 => rng.below(full as u32 + 1) as c_int,
                2 => rng.below(64) as c_int,
                _ => full - rng.below(8) as c_int,
            };
            p.check("hetero", &buf, rng.below(9) as c_int, limit, &hdr);
        }
    }
}

// ---------------------------------------------------------------------------
// 4. Broad header sweep + random bitstream fuzzing
// ---------------------------------------------------------------------------

#[test]
fn all_header_bytes_sweep() {
    let p = pair();
    let mut rng = Rng(0x0F0F_0F0F_1111_2222);
    let payloads: Vec<Vec<u8>> = (0..6).map(|_| rng.bytes(96)).collect();
    let extra: Vec<Vec<u8>> = vec![vec![0x00; 96], vec![0xFF; 96], {
        let mut v = vec![0u8; 96];
        for (i, b) in v.iter_mut().enumerate() {
            *b = i as u8;
        }
        v
    }];
    for h1 in 0u32..256 {
        for h2 in 0u32..256 {
            let hdr = [0xFF, h1 as u8, h2 as u8, 0x00];
            for buf in payloads.iter().chain(extra.iter()) {
                p.check("hdr-sweep", buf, 0, 768, &hdr);
            }
        }
    }
    for h3 in 0u32..256 {
        for h1 in [0x00u8, 0x08, 0x10, 0x18, 0xFF] {
            let hdr = [0xFF, h1, 0x08, h3 as u8];
            for buf in payloads.iter().chain(extra.iter()) {
                p.check("hdr3-sweep", buf, 0, 768, &hdr);
                p.check("hdr3-sweep", buf, 5, 137, &hdr);
            }
        }
    }
}

#[test]
fn random_fuzz() {
    let p = pair();
    let mut rng = Rng(0x5EED_1234_9876_5432);
    for _ in 0..400_000 {
        let len = 8 + rng.below(88) as usize;
        let buf = rng.bytes(len);
        let hdr = [
            rng.next_u32() as u8,
            rng.next_u32() as u8,
            rng.next_u32() as u8,
            rng.next_u32() as u8,
        ];
        let max = (len * 8) as u32;
        let pos = rng.below(max) as c_int;
        // keep limit within the buffer so no read runs off the end
        let limit = match rng.below(3) {
            0 => max as c_int,
            1 => rng.below(max + 1) as c_int,
            _ => rng.below(48) as c_int,
        };
        p.check("fuzz", &buf, pos, limit, &hdr);
    }
}

/// `get_bits` computes the byte pointer *before* the limit check. With a
/// negative `pos` and a limit below it every read bails out early, so nothing is
/// dereferenced -- both libraries must agree on the resulting reader state.
#[test]
fn negative_position_never_reads() {
    let p = pair();
    let buf = [0u8; 64];
    for hdr in header_matrix() {
        for pos in [-1i32, -7, -8, -9, -64, -1000, -100_000] {
            for limit in [pos - 1, pos - 16, -200_000, i32::MIN / 2] {
                p.check("neg", &buf, pos, limit, &hdr);
            }
        }
    }
}

/// Full 16-bit sweep of the two header bytes that steer granule count and
/// MPEG version.
#[test]
fn header_1_and_3_full_sweep() {
    let p = pair();
    let mut rng = Rng(0x1357_9BDF_2468_ACE0);
    let payloads: Vec<Vec<u8>> = (0..3).map(|_| rng.bytes(96)).collect();
    for h1 in 0u32..256 {
        for h3 in 0u32..256 {
            let hdr = [0xFF, h1 as u8, 0x04, h3 as u8];
            for buf in &payloads {
                p.check("h1h3", buf, 0, 768, &hdr);
                p.check("h1h3", buf, 3, 91, &hdr);
            }
        }
    }
}

#[test]
fn limit_boundary_exhaustive() {
    let p = pair();
    let mut rng = Rng(0x9999_8888_7777_6666);
    for _ in 0..8 {
        let buf = rng.bytes(96);
        for hdr in header_matrix() {
            for limit in 0..300i32 {
                p.check("limit", &buf, 0, limit, &hdr);
            }
        }
    }
}
