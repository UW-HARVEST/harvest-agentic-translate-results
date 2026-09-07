//! Phase B — valid-path differential tests, one test per CONFIGS.md row.
//! Every test calls BOTH the C `.so` and the Rust `.so` via `libloading`.

mod common;
use common::*;

const SEED: u64 = 0x5EED_1234_ABCD_0001;

// ---------------------------------------------------------------- C1
#[test]
fn c1_drop_empty_string() {
    check_drop(b"");
    assert_eq!(c().drop_offset(b"\0"), 0);
    assert_eq!(rust().drop_offset(b"\0"), 0);
}

// ---------------------------------------------------------------- C2
#[test]
fn c2_drop_random_ascii() {
    let mut rng = Rng::new(SEED);
    for _ in 0..2000 {
        let n = rng.range(1, 64);
        let mut v = Vec::new();
        for _ in 0..n {
            push_valid(&mut rng, &mut v, 1);
        }
        check_drop(&v);
    }
    // boundary ASCII values
    for b in [0x01u8, 0x0A, 0x7E, 0x7F] {
        check_drop(&[b]);
        check_drop(&[b, b, b]);
    }
}

// ---------------------------------------------------------------- C3
#[test]
fn c3_drop_valid_two_byte() {
    let mut rng = Rng::new(SEED + 3);
    for _ in 0..2000 {
        let n = rng.range(1, 20);
        let mut v = Vec::new();
        for _ in 0..n {
            push_valid(&mut rng, &mut v, 2);
        }
        check_drop(&v);
    }
    // exhaustive over the 2-byte space that matters
    for lead in 0xC0u8..=0xDF {
        for b1 in 0x00u8..=0xFF {
            check_drop(&[lead, b1]);
        }
    }
}

// ---------------------------------------------------------------- C4
#[test]
fn c4_drop_valid_three_byte_generic_leads() {
    let mut rng = Rng::new(SEED + 4);
    for lead in [0xE1u8, 0xE2, 0xE7, 0xEB, 0xEC, 0xEE] {
        for _ in 0..300 {
            let mut v = Vec::new();
            let n = rng.range(1, 8);
            for _ in 0..n {
                v.push(lead);
                v.push(rng.range(0x80, 0xBF) as u8);
                v.push(rng.range(0x80, 0xBF) as u8);
            }
            check_drop(&v);
        }
    }
}

// ------------------------------------------------------- C5 / C6 / C7
#[test]
fn c5_c6_c7_drop_three_byte_special_leads() {
    // Sweep b1 and b2 over the FULL byte range for the special leads.
    for lead in [0xE0u8, 0xED, 0xEF] {
        for b1 in 0x00u8..=0xFF {
            for b2 in [0x00u8, 0x41, 0x7F, 0x80, 0x9F, 0xA0, 0xBF, 0xC0, 0xFF] {
                check_drop(&[lead, b1, b2]);
            }
        }
    }
    // and with trailing content, to check the advance width
    for lead in [0xE0u8, 0xED, 0xEF] {
        for b1 in [0x80u8, 0x9F, 0xA0, 0xBF] {
            check_drop(&[lead, b1, 0x80, b'Z']);
        }
    }
}

// ---------------------------------------------------------------- C8
#[test]
fn c8_drop_valid_four_byte_generic_leads() {
    let mut rng = Rng::new(SEED + 8);
    for lead in [0xF1u8, 0xF2, 0xF3] {
        for _ in 0..400 {
            let mut v = Vec::new();
            let n = rng.range(1, 6);
            for _ in 0..n {
                v.push(lead);
                v.push(rng.range(0x80, 0xBF) as u8);
                v.push(rng.range(0x80, 0xBF) as u8);
                v.push(rng.range(0x80, 0xBF) as u8);
            }
            check_drop(&v);
        }
    }
}

// ----------------------------------------------------------- C9 / C10
#[test]
fn c9_c10_drop_four_byte_special_leads() {
    for lead in [0xF0u8, 0xF4] {
        for b1 in 0x00u8..=0xFF {
            for b2 in [0x00u8, 0x80, 0xBF, 0xC0] {
                for b3 in [0x00u8, 0x80, 0xBF, 0xC0] {
                    check_drop(&[lead, b1, b2, b3]);
                }
            }
        }
    }
    for lead in [0xF0u8, 0xF4] {
        for b1 in [0x80u8, 0x8F, 0x90, 0xBF] {
            check_drop(&[lead, b1, 0x80, 0x80, b'Z']);
        }
    }
}

// ---------------------------------------------------------------- C11
#[test]
fn c11_drop_rejected_leads_at_offset_zero() {
    let rejected: Vec<u8> = (0x80u8..=0xC1)
        .chain(0xF5u8..=0xFF)
        .collect();
    for &lead in &rejected {
        // At offset 0 the whole rest of the string is irrelevant: must be 0.
        for tail in [
            vec![],
            vec![0x80u8],
            vec![0x80, 0x80],
            vec![0x80, 0x80, 0x80],
            vec![b'A', b'B'],
        ] {
            let mut v = vec![lead];
            v.extend_from_slice(&tail);
            check_drop(&v);
            let mut s = v.clone();
            s.push(0);
            assert_eq!(c().drop_offset(&s), 0, "lead {lead:#04X} should be rejected");
        }
        // and at a non-zero offset, after valid content
        let mut v = b"ab".to_vec();
        v.push(lead);
        v.extend_from_slice(&[0x80, 0x80, 0x80]);
        check_drop(&v);
    }
}

// ---------------------------------------------------------------- C12
#[test]
fn c12_drop_exhaustive_one_and_two_bytes() {
    for b0 in 0x00u8..=0xFF {
        check_drop(&[b0]);
    }
    for b0 in 0x00u8..=0xFF {
        for b1 in 0x00u8..=0xFF {
            check_drop(&[b0, b1]);
        }
    }
}

#[test]
fn c12_drop_exhaustive_three_bytes() {
    // 16 777 216 inputs; both `.so`s are called for each one.
    let f_c = c();
    let f_r = rust();
    let mut buf = [0u8; 4];
    for b0 in 0x00u8..=0xFF {
        buf[0] = b0;
        for b1 in 0x00u8..=0xFF {
            buf[1] = b1;
            for b2 in 0x00u8..=0xFF {
                buf[2] = b2;
                buf[3] = 0;
                let a = f_c.drop_offset(&buf);
                let b = f_r.drop_offset(&buf);
                assert_eq!(
                    a, b,
                    "w_utf8_drop divergence at [{b0:02X} {b1:02X} {b2:02X}]: C={a} RUST={b}"
                );
            }
        }
    }
}

// ---------------------------------------------------------------- C14
#[test]
fn c14_drop_random_bytes() {
    let mut rng = Rng::new(SEED + 14);
    for _ in 0..20000 {
        let n = rng.range(0, 64);
        let mut v = Vec::with_capacity(n);
        for _ in 0..n {
            let b = rng.byte();
            v.push(if b == 0 { 1 } else { b });
        }
        check_drop(&v);
    }
    // biased towards structured-but-broken UTF-8
    for _ in 0..20000 {
        let mut v = Vec::new();
        let n = rng.range(0, 16);
        for _ in 0..n {
            match rng.range(0, 5) {
                0..=3 => push_valid_rand(&mut rng, &mut v),
                4 => v.push(invalid_lead(&mut rng)),
                _ => {
                    // a truncated valid sequence
                    let w = rng.range(2, 4) as u8;
                    let mut t = Vec::new();
                    push_valid(&mut rng, &mut t, w);
                    t.truncate(t.len() - 1);
                    v.extend_from_slice(&t);
                }
            }
        }
        check_drop(&v);
    }
}

// ---------------------------------------------------------------- C15
#[test]
fn c15_drop_long_inputs() {
    let mut rng = Rng::new(SEED + 15);
    for _ in 0..300 {
        let n = rng.range(1000, 9000);
        let mut v = Vec::with_capacity(n);
        for _ in 0..n {
            let b = rng.byte();
            v.push(if b == 0 { 1 } else { b });
        }
        check_drop(&v);
    }
}

// ----------------------------------------------------------- C16 / C17
#[test]
fn c16_c17_filter_fully_valid_strdup_fast_path() {
    let mut rng = Rng::new(SEED + 16);
    check_filter(b"", 0);
    check_filter(b"", 1);
    for width in 1u8..=4 {
        for _ in 0..500 {
            let mut v = Vec::new();
            let n = rng.range(1, 20);
            for _ in 0..n {
                push_valid(&mut rng, &mut v, width);
            }
            check_filter(&v, 0);
            check_filter(&v, 1);
            // fully-valid input must come back unchanged
            let mut s = v.clone();
            s.push(0);
            assert_eq!(c().filter(&s, 0).unwrap(), v);
            assert_eq!(rust().filter(&s, 0).unwrap(), v);
        }
    }
    // mixed widths
    for _ in 0..2000 {
        let mut v = Vec::new();
        let n = rng.range(1, 24);
        for _ in 0..n {
            push_valid_rand(&mut rng, &mut v);
        }
        check_filter(&v, 0);
        check_filter(&v, 1);
    }
}

// ----------------------------------------------------------- C18 / C19
#[test]
fn c18_c19_filter_invalid_at_offset_zero() {
    let mut rng = Rng::new(SEED + 18);
    for _ in 0..3000 {
        let mut v = vec![invalid_lead(&mut rng)];
        let n = rng.range(0, 10);
        for _ in 0..n {
            push_valid_rand(&mut rng, &mut v);
        }
        check_filter(&v, 0);
        check_filter(&v, 1);
    }
}

// ----------------------------------------------------------- C20 / C21
#[test]
fn c20_c21_filter_invalid_mid_string() {
    let mut rng = Rng::new(SEED + 20);
    for _ in 0..3000 {
        let mut v = Vec::new();
        for _ in 0..rng.range(1, 8) {
            push_valid_rand(&mut rng, &mut v);
        }
        v.push(invalid_lead(&mut rng));
        for _ in 0..rng.range(1, 8) {
            push_valid_rand(&mut rng, &mut v);
        }
        check_filter(&v, 0);
        check_filter(&v, 1);
    }
}

// ----------------------------------------------------------- C22 / C23
#[test]
fn c22_c23_filter_invalid_as_last_byte() {
    let mut rng = Rng::new(SEED + 22);
    for _ in 0..3000 {
        let mut v = Vec::new();
        for _ in 0..rng.range(0, 10) {
            push_valid_rand(&mut rng, &mut v);
        }
        v.push(invalid_lead(&mut rng));
        check_filter(&v, 0);
        check_filter(&v, 1);
    }
}

// ----------------------------------------------------------- C24 / C25
#[test]
fn c24_c25_filter_alternating_all_widths() {
    let mut rng = Rng::new(SEED + 24);
    for _ in 0..4000 {
        let mut v = Vec::new();
        for _ in 0..rng.range(1, 30) {
            push_valid_rand(&mut rng, &mut v);
            v.push(invalid_lead(&mut rng));
        }
        check_filter(&v, 0);
        check_filter(&v, 1);
    }
}

// ----------------------------------------------------------- C26 / C27
#[test]
fn c26_c27_filter_all_bytes_invalid() {
    let mut rng = Rng::new(SEED + 26);
    for n in [1usize, 2, 3, 7, 100, 500, 4095, 4096, 4097] {
        let mut v = Vec::with_capacity(n);
        for _ in 0..n {
            v.push(invalid_lead(&mut rng));
        }
        check_filter(&v, 0);
        check_filter(&v, 1);
        let mut s = v.clone();
        s.push(0);
        assert_eq!(c().filter(&s, 0).unwrap().len(), 0);
        assert_eq!(c().filter(&s, 1).unwrap().len(), 3 * n);
        assert_eq!(rust().filter(&s, 1).unwrap().len(), 3 * n);
    }
}

// ---------------------------------------------------------------- C28
#[test]
fn c28_filter_replacement_refill_boundaries() {
    // 4096 / 3 == 1365.33 -> the `repl < 3` refill fires around these counts.
    let counts = [
        1usize, 2, 3, 4, 1363, 1364, 1365, 1366, 1367, 1368, 2729, 2730, 2731, 2732, 2733, 4095,
        4096, 4097, 5460, 5461, 5462,
    ];
    for &n in &counts {
        let v = vec![0xFFu8; n];
        check_filter(&v, 0);
        check_filter(&v, 1);
        // interleaved with valid bytes so `i` and `size` diverge differently
        let mut w = Vec::new();
        for k in 0..n {
            w.push(if k % 3 == 0 { b'a' } else { 0xC0 });
        }
        check_filter(&w, 0);
        check_filter(&w, 1);
    }
}

// ---------------------------------------------------------------- C29
#[test]
fn c29_filter_many_realloc_cycles() {
    let mut rng = Rng::new(SEED + 29);
    for _ in 0..40 {
        let n = rng.range(5000, 20000);
        let mut v = Vec::with_capacity(n);
        for _ in 0..n {
            if rng.range(0, 3) == 0 {
                push_valid_rand(&mut rng, &mut v);
            } else {
                v.push(invalid_lead(&mut rng));
            }
        }
        check_filter(&v, 0);
        check_filter(&v, 1);
    }
}

// ---------------------------------------------------------------- C30
#[test]
fn c30_filter_truncated_sequences_at_end() {
    // Every lead byte, truncated at every possible point before the NUL.
    for lead in 0xC0u8..=0xFF {
        for extra in 0usize..=3 {
            let mut v = vec![lead];
            for _ in 0..extra {
                v.push(0x80);
            }
            check_all(&v);
            let mut w = b"prefix".to_vec();
            w.extend_from_slice(&v);
            check_all(&w);
        }
    }
}

// ---------------------------------------------------------------- C31
#[test]
fn c31_filter_random_bytes() {
    let mut rng = Rng::new(SEED + 31);
    for _ in 0..20000 {
        let n = rng.range(0, 64);
        let mut v = Vec::with_capacity(n);
        for _ in 0..n {
            let b = rng.byte();
            v.push(if b == 0 { 1 } else { b });
        }
        check_filter(&v, 0);
        check_filter(&v, 1);
    }
}

// ---------------------------------------------------------------- C32
#[test]
fn c32_filter_long_random_inputs() {
    let mut rng = Rng::new(SEED + 32);
    for _ in 0..200 {
        let n = rng.range(1000, 9000);
        let mut v = Vec::with_capacity(n);
        for _ in 0..n {
            let b = rng.byte();
            v.push(if b == 0 { 1 } else { b });
        }
        check_filter(&v, 0);
        check_filter(&v, 1);
    }
}

// ---------------------------------------------------------------- C33
#[test]
fn c33_filter_exhaustive_one_and_two_bytes() {
    for b0 in 0x00u8..=0xFF {
        check_filter(&[b0], 0);
        check_filter(&[b0], 1);
    }
    for b0 in 0x00u8..=0xFF {
        for b1 in 0x00u8..=0xFF {
            check_filter(&[b0, b1], 0);
            check_filter(&[b0, b1], 1);
        }
    }
}

#[test]
fn c33b_filter_exhaustive_three_bytes_sampled_flag() {
    // Full 3-byte space for both flag values.
    let f_c = c();
    let f_r = rust();
    let mut buf = [0u8; 4];
    for b0 in 0x00u8..=0xFF {
        buf[0] = b0;
        for b1 in 0x00u8..=0xFF {
            buf[1] = b1;
            for b2 in 0x00u8..=0xFF {
                buf[2] = b2;
                buf[3] = 0;
                for r in [0u8, 1] {
                    let a = f_c.filter(&buf, r);
                    let b = f_r.filter(&buf, r);
                    assert_eq!(
                        a, b,
                        "w_utf8_filter divergence at [{b0:02X} {b1:02X} {b2:02X}] r={r}"
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------- C35
#[test]
fn c35_composed_pipeline_filter_then_drop() {
    let mut rng = Rng::new(SEED + 35);
    for _ in 0..5000 {
        let n = rng.range(0, 48);
        let mut v = Vec::with_capacity(n);
        for _ in 0..n {
            let b = rng.byte();
            v.push(if b == 0 { 1 } else { b });
        }
        let mut s = v.clone();
        s.push(0);
        for r in [0u8, 1] {
            let out_c = c().filter(&s, r).unwrap();
            let out_r = rust().filter(&s, r).unwrap();
            assert_eq!(out_c, out_r, "stage 1 divergence for [{}] r={r}", hex(&v));

            // Feed the result of C's filter back through BOTH drops.
            let mut t = out_c.clone();
            t.push(0);
            let d_c = c().drop_offset(&t);
            let d_r = rust().drop_offset(&t);
            assert_eq!(d_c, d_r, "stage 2 divergence for [{}] r={r}", hex(&out_c));
            // The filtered output must itself be fully valid UTF-8.
            assert_eq!(
                d_c,
                out_c.len(),
                "filter output not fully valid: [{}]",
                hex(&out_c)
            );

            // ...and through the Rust `.so`'s output too.
            let mut u = out_r.clone();
            u.push(0);
            assert_eq!(c().drop_offset(&u), rust().drop_offset(&u));
        }
    }
}

// ---------------------------------------------------------------- C36
#[test]
fn c36_returned_buffer_is_freeable() {
    // `common::Impl::filter` already `free()`s every returned pointer using the
    // test process's libc `free`; running many allocations through both paths
    // (strdup fast path and malloc/realloc path) exercises that the pointers are
    // genuine libc heap pointers. A mismatched allocator would abort here.
    let mut rng = Rng::new(SEED + 36);
    for _ in 0..5000 {
        let mut valid = Vec::new();
        for _ in 0..rng.range(1, 12) {
            push_valid_rand(&mut rng, &mut valid);
        }
        check_filter(&valid, 0); // strdup path
        let mut broken = valid.clone();
        broken.push(0xC1);
        check_filter(&broken, 0); // malloc path
        check_filter(&broken, 1); // malloc + realloc path
    }
}
