//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Every test loads BOTH the C `.so` and the Rust `.so` via `libloading` and
//! compares their results byte-for-byte. The Rust side is only ever reached
//! through its exported `md5_digest` symbol.

mod common;

use common::*;

// ---------------------------------------------------------------------------
// C1..C4 — fixed value shapes, disjoint aligned buffers.
// ---------------------------------------------------------------------------

fn assert_same_16(libs: &Libs, m: Md5, label: &str) {
    // Two prefills, so "not stored at all" cannot masquerade as "stored 0".
    for prefill in [0x00u8, 0xAA] {
        let c = digest16_prefill(libs, Impl::C, m, prefill);
        let r = digest16_prefill(libs, Impl::Rust, m, prefill);
        assert_eq!(
            c, r,
            "{label}: mismatch for {m:08x?} (prefill {prefill:#04x})\n C={c:02x?}\n R={r:02x?}"
        );
    }
}

#[test]
fn cfg_c1_all_zero() {
    let libs = Libs::load();
    assert_same_16(&libs, Md5 { a: 0, b: 0, c: 0, d: 0 }, "C1");
}

#[test]
fn cfg_c2_all_ones() {
    let libs = Libs::load();
    let m = Md5 {
        a: 0xFFFF_FFFF,
        b: 0xFFFF_FFFF,
        c: 0xFFFF_FFFF,
        d: 0xFFFF_FFFF,
    };
    assert_same_16(&libs, m, "C2");
}

#[test]
fn cfg_c3_byte_distinct_ascending() {
    let libs = Libs::load();
    let m = Md5 {
        a: 0x0403_0201,
        b: 0x0807_0605,
        c: 0x0C0B_0A09,
        d: 0x100F_0E0D,
    };
    assert_same_16(&libs, m, "C3");
    // Pin the absolute expectation too: little-endian byte order 1..=16.
    let c = digest16(&libs, Impl::C, m);
    assert_eq!(c, [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]);
}

#[test]
fn cfg_c4_high_bit_set() {
    let libs = Libs::load();
    // Catches an arithmetic (signed) shift or an i32 intermediate.
    for v in [0x8000_0000u32, 0xFFFF_FF80, 0x8080_8080, 0x8000_0001] {
        assert_same_16(&libs, Md5 { a: v, b: v, c: v, d: v }, "C4");
    }
}

#[test]
fn cfg_c5_single_byte_isolated_sweep() {
    let libs = Libs::load();
    // For each of the 4 fields x each of the 4 byte lanes, exactly one non-zero
    // byte. Independently pins every (offset, shift) pair: a swapped field or a
    // swapped shift moves the single set byte to a different output index.
    for field in 0..4 {
        for lane in 0..4 {
            for probe in [0x01u8, 0xFF, 0x80] {
                let v = (probe as u32) << (8 * lane);
                let mut m = Md5::default();
                match field {
                    0 => m.a = v,
                    1 => m.b = v,
                    2 => m.c = v,
                    _ => m.d = v,
                }
                let label = format!("C5 field={field} lane={lane} probe={probe:#04x}");
                assert_same_16(&libs, m, &label);
                // Absolute check: the set byte must land at output index
                // field*4 + lane (little-endian).
                let c = digest16(&libs, Impl::C, m);
                let mut want = [0u8; 16];
                want[field * 4 + lane] = probe;
                assert_eq!(c, want, "{label}: C itself deviated from LE expectation");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C6, C7 — randomized value shapes.
// ---------------------------------------------------------------------------

#[test]
fn cfg_c6_randomized_full_range() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED);
    for i in 0..2000 {
        let m = rng.md5();
        assert_same_16(&libs, m, &format!("C6 iter={i}"));
    }
}

#[test]
fn cfg_c7_randomized_sparse_boundary_bytes() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 0x5555_5555_5555_5555);
    for i in 0..2000 {
        let m = rng.md5_sparse();
        assert_same_16(&libs, m, &format!("C7 iter={i}"));
    }
}

// ---------------------------------------------------------------------------
// C8..C10 — alignment axes. The C reads the source with 32-bit loads; a port
// that assumes 4-byte alignment (or uses wider SIMD loads) breaks here.
// ---------------------------------------------------------------------------

#[test]
fn cfg_c8_misaligned_out() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 0x0808_0808);
    for out_pad in 0..=8usize {
        for i in 0..64 {
            let m = if i % 2 == 0 { rng.md5() } else { rng.md5_sparse() };
            let sc = Scenario {
                buf_len: 128,
                m_off: 0,
                out_off: 64 + out_pad,
                fill: 0xAA,
                src: md5_to_le_bytes(m),
            };
            sc.assert_match(&libs, &format!("C8 out_pad={out_pad} iter={i}"));
        }
    }
}

#[test]
fn cfg_c9_misaligned_m() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 0x0909_0909);
    for m_pad in 1..=8usize {
        for i in 0..64 {
            let m = if i % 2 == 0 { rng.md5() } else { rng.md5_sparse() };
            let sc = Scenario {
                buf_len: 128,
                m_off: m_pad,
                out_off: 64,
                fill: 0xAA,
                src: md5_to_le_bytes(m),
            };
            sc.assert_match(&libs, &format!("C9 m_pad={m_pad} iter={i}"));
        }
    }
}

#[test]
fn cfg_c10_both_misaligned() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 0x1010_1010);
    for m_pad in 0..=8usize {
        for out_pad in 0..=8usize {
            for i in 0..8 {
                let m = if i % 2 == 0 { rng.md5() } else { rng.md5_sparse() };
                let sc = Scenario {
                    buf_len: 128,
                    m_off: m_pad,
                    out_off: 64 + out_pad,
                    fill: 0xAA,
                    src: md5_to_le_bytes(m),
                };
                sc.assert_match(
                    &libs,
                    &format!("C10 m_pad={m_pad} out_pad={out_pad} iter={i}"),
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C11..C14 — ALIASING. `md5_digest` takes `const tflac_md5 *` and `tflac_u8 *`
// with no `restrict`, and a `uint8_t` store may alias a `uint32_t` object, so
// the C compiler MUST reload the source field before every byte store. Verified
// in the disassembly at -O0 and -O2 alike. Overlapping buffers are therefore
// well-defined, observable input, and a port that snapshots `*m` up front
// diverges here.
// ---------------------------------------------------------------------------

#[test]
fn cfg_c11_overlap_exact() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 0x1111_1111);
    for i in 0..400 {
        let m = if i % 2 == 0 { rng.md5() } else { rng.md5_sparse() };
        let sc = Scenario {
            buf_len: 128,
            m_off: 32,
            out_off: 32, // out == (tflac_u8 *)m
            fill: 0xAA,
            src: md5_to_le_bytes(m),
        };
        sc.assert_match(&libs, &format!("C11 iter={i}"));
    }
}

#[test]
fn cfg_c12_overlap_forward() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 0x1212_1212);
    for d in 1..=15usize {
        for i in 0..64 {
            let m = if i % 2 == 0 { rng.md5() } else { rng.md5_sparse() };
            let sc = Scenario {
                buf_len: 128,
                m_off: 32,
                out_off: 32 + d,
                fill: 0xAA,
                src: md5_to_le_bytes(m),
            };
            sc.assert_match(&libs, &format!("C12 d=+{d} iter={i}"));
        }
    }
}

#[test]
fn cfg_c13_overlap_backward() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 0x1313_1313);
    for d in 1..=15usize {
        for i in 0..64 {
            let m = if i % 2 == 0 { rng.md5() } else { rng.md5_sparse() };
            let sc = Scenario {
                buf_len: 128,
                m_off: 32,
                out_off: 32 - d,
                fill: 0xAA,
                src: md5_to_le_bytes(m),
            };
            sc.assert_match(&libs, &format!("C13 d=-{d} iter={i}"));
        }
    }
}

#[test]
fn cfg_c14_adjacent_but_disjoint() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 0x1414_1414);
    for d in 16..=31usize {
        for i in 0..16 {
            let m = if i % 2 == 0 { rng.md5() } else { rng.md5_sparse() };
            for &out_off in [64 + d, 64 - d].iter() {
                let sc = Scenario {
                    buf_len: 192,
                    m_off: 64,
                    out_off,
                    fill: 0xAA,
                    src: md5_to_le_bytes(m),
                };
                sc.assert_match(&libs, &format!("C14 d={d} out_off={out_off} iter={i}"));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C15, C16 — write extent and "actually stored".
// ---------------------------------------------------------------------------

#[test]
fn cfg_c15_write_extent_is_exactly_16() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 0x1515_1515);
    for i in 0..500 {
        let m = rng.md5();
        let sc = Scenario {
            buf_len: 128,
            m_off: 0,
            out_off: 64,
            fill: 0x5A,
            src: md5_to_le_bytes(m),
        };
        // Whole-buffer comparison already catches over-writes, but assert the
        // absolute invariant too: only [64, 80) may differ from the fill.
        let got = sc.run(&libs, Impl::Rust);
        let got_c = sc.run(&libs, Impl::C);
        assert_eq!(got, got_c, "C15 iter={i}: buffers differ");
        for (idx, &b) in got.iter().enumerate() {
            if !(64..80).contains(&idx) && !(0..16).contains(&idx) {
                assert_eq!(b, 0x5A, "C15 iter={i}: byte {idx} outside out[0..16] was modified");
            }
        }
    }
}

#[test]
fn cfg_c16_all_zero_input_overwrites_sentinel() {
    let libs = Libs::load();
    let m = Md5::default();
    for prefill in [0xAAu8, 0xFF, 0x01] {
        let c = digest16_prefill(&libs, Impl::C, m, prefill);
        let r = digest16_prefill(&libs, Impl::Rust, m, prefill);
        assert_eq!(c, r, "C16 prefill={prefill:#04x}");
        assert_eq!(c, [0u8; 16], "C16: all 16 bytes must be stored as zero");
    }
}

// ---------------------------------------------------------------------------
// C17, C18 — guard-page proofs that the read/write extent is exactly 16 bytes.
// A 17th access hits a PROT_NONE page and would kill the process.
// ---------------------------------------------------------------------------

#[test]
fn cfg_c17_no_source_overread() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 0x1717_1717);
    for which in [Impl::C, Impl::Rust] {
        for i in 0..200 {
            let m = rng.md5();
            let g = GuardedPage::new();
            g.write_at_end(16, &md5_to_le_bytes(m)); // last 16 bytes of the page
            let mut out = [0u8; 16];
            let f = libs.digest(which);
            unsafe { f(g.end_minus(16) as *const Md5, out.as_mut_ptr()) };
            assert_eq!(
                out,
                md5_to_le_bytes(m),
                "C17 {} iter={i}: wrong bytes read from page-end source",
                which.name()
            );
        }
    }
}

#[test]
fn cfg_c18_no_dest_overwrite() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 0x1818_1818);
    for which in [Impl::C, Impl::Rust] {
        for i in 0..200 {
            let m = rng.md5();
            let g = GuardedPage::new();
            g.fill_end(16, 0xAA);
            let f = libs.digest(which);
            unsafe { f(&m as *const Md5, g.end_minus(16)) };
            assert_eq!(
                g.read_end(16),
                md5_to_le_bytes(m).to_vec(),
                "C18 {} iter={i}: wrong bytes written to page-end dest",
                which.name()
            );
        }
    }
}

// ---------------------------------------------------------------------------
// C19 — no hidden state between calls.
// ---------------------------------------------------------------------------

#[test]
fn cfg_c19_repeated_calls_no_hidden_state() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 0x1919_1919);
    let fc = libs.digest(Impl::C);
    let fr = libs.digest(Impl::Rust);
    let mut out_c = [0u8; 16];
    let mut out_r = [0u8; 16];
    for i in 0..500 {
        let m = rng.md5();
        // Reuse the SAME out buffers across all iterations, never clearing them.
        unsafe { fc(&m as *const Md5, out_c.as_mut_ptr()) };
        unsafe { fr(&m as *const Md5, out_r.as_mut_ptr()) };
        assert_eq!(out_c, out_r, "C19 iter={i}");
        assert_eq!(out_c, md5_to_le_bytes(m), "C19 iter={i}: stale bytes leaked");
    }
}

// ---------------------------------------------------------------------------
// C20 — cross-product of alignment and overlap displacement.
// ---------------------------------------------------------------------------

#[test]
fn cfg_c20_alignment_x_overlap_cross_product() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 0x2020_2020);
    const BASE: usize = 96;
    for i in 0..4000 {
        let m = if i % 3 == 0 { rng.md5_sparse() } else { rng.md5() };
        let m_off = BASE + rng.below(8); // random alignment
        let disp = rng.below(63) as isize - 31; // -31 ..= +31
        let out_off = (m_off as isize + disp) as usize;
        let sc = Scenario {
            buf_len: 256,
            m_off,
            out_off,
            fill: if i % 2 == 0 { 0xAA } else { 0x00 },
            src: md5_to_le_bytes(m),
        };
        sc.assert_match(&libs, &format!("C20 iter={i} m_off={m_off} disp={disp}"));
    }
}

// ---------------------------------------------------------------------------
// Type-layout parity (SYMBOLS.md).
// ---------------------------------------------------------------------------

#[test]
fn layout_parity_struct_tflac_md5() {
    assert_eq!(core::mem::size_of::<Md5>(), 16);
    assert_eq!(core::mem::align_of::<Md5>(), 4);
    let m = Md5 {
        a: 0x0403_0201,
        b: 0x0807_0605,
        c: 0x0C0B_0A09,
        d: 0x100F_0E0D,
    };
    // The C reads the struct through 32-bit loads at offsets 0/4/8/12, so a
    // round-trip through the C .so proves the Rust mirror has the same layout.
    let libs = Libs::load();
    assert_eq!(
        digest16(&libs, Impl::C, m),
        [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16],
        "struct layout mismatch between Rust mirror and C"
    );
}
