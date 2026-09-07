//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Every test loads BOTH shared objects via `libloading` and compares their
//! output byte-for-byte. Randomized rows use the fixed seed `common::SEED`.

mod common;

use common::{assert_same_in_buffer, load_pair, AlignedBuf, Pair, Rng, TflacMd5, SEED};

/// Extra randomized inputs layered on top of every named row, so no row is
/// validated by a single hand-picked value.
const PER_ROW_RANDOM: usize = 512;

fn random_sweep(pair: &Pair, label: &str, n: usize) {
    let mut rng = Rng::new(SEED);
    for i in 0..n {
        let m = rng.next_md5();
        pair.assert_same(&format!("{label}/rand#{i}"), &m);
    }
}

// ---------------------------------------------------------------- C1 .. C2 --

#[test]
fn cfg_c1_all_zero() {
    let pair = load_pair();
    pair.assert_same("C1", &TflacMd5::new(0, 0, 0, 0));
    // Independently confirm the shape the C code must produce.
    assert_eq!(pair.c.digest(&TflacMd5::new(0, 0, 0, 0)), [0u8; 16]);
    random_sweep(&pair, "C1", PER_ROW_RANDOM);
}

#[test]
fn cfg_c2_all_ones() {
    let pair = load_pair();
    let m = TflacMd5::new(u32::MAX, u32::MAX, u32::MAX, u32::MAX);
    pair.assert_same("C2", &m);
    assert_eq!(pair.c.digest(&m), [0xFFu8; 16]);
    random_sweep(&pair, "C2", PER_ROW_RANDOM);
}

// ---------------------------------------------------------------- C3 .. C6 --
// Isolate one field at a time: pins each struct offset to its output range.

#[test]
fn cfg_c3_isolate_a() {
    let pair = load_pair();
    let m = TflacMd5::new(0xDEAD_BEEF, 0, 0, 0);
    pair.assert_same("C3", &m);
    let out = pair.c.digest(&m);
    assert_eq!(&out[0..4], &[0xEF, 0xBE, 0xAD, 0xDE]);
    assert_eq!(&out[4..16], &[0u8; 12]);
    let mut rng = Rng::new(SEED ^ 3);
    for i in 0..PER_ROW_RANDOM {
        pair.assert_same(&format!("C3/rand#{i}"), &TflacMd5::new(rng.next_u32(), 0, 0, 0));
    }
}

#[test]
fn cfg_c4_isolate_b() {
    let pair = load_pair();
    let m = TflacMd5::new(0, 0xDEAD_BEEF, 0, 0);
    pair.assert_same("C4", &m);
    let out = pair.c.digest(&m);
    assert_eq!(&out[4..8], &[0xEF, 0xBE, 0xAD, 0xDE]);
    assert_eq!(&out[0..4], &[0u8; 4]);
    assert_eq!(&out[8..16], &[0u8; 8]);
    let mut rng = Rng::new(SEED ^ 4);
    for i in 0..PER_ROW_RANDOM {
        pair.assert_same(&format!("C4/rand#{i}"), &TflacMd5::new(0, rng.next_u32(), 0, 0));
    }
}

#[test]
fn cfg_c5_isolate_c() {
    let pair = load_pair();
    let m = TflacMd5::new(0, 0, 0xDEAD_BEEF, 0);
    pair.assert_same("C5", &m);
    let out = pair.c.digest(&m);
    assert_eq!(&out[8..12], &[0xEF, 0xBE, 0xAD, 0xDE]);
    assert_eq!(&out[0..8], &[0u8; 8]);
    assert_eq!(&out[12..16], &[0u8; 4]);
    let mut rng = Rng::new(SEED ^ 5);
    for i in 0..PER_ROW_RANDOM {
        pair.assert_same(&format!("C5/rand#{i}"), &TflacMd5::new(0, 0, rng.next_u32(), 0));
    }
}

#[test]
fn cfg_c6_isolate_d() {
    let pair = load_pair();
    let m = TflacMd5::new(0, 0, 0, 0xDEAD_BEEF);
    pair.assert_same("C6", &m);
    let out = pair.c.digest(&m);
    assert_eq!(&out[12..16], &[0xEF, 0xBE, 0xAD, 0xDE]);
    assert_eq!(&out[0..12], &[0u8; 12]);
    let mut rng = Rng::new(SEED ^ 6);
    for i in 0..PER_ROW_RANDOM {
        pair.assert_same(&format!("C6/rand#{i}"), &TflacMd5::new(0, 0, 0, rng.next_u32()));
    }
}

// --------------------------------------------------------------------- C7 --

#[test]
fn cfg_c7_full_byte_permutation() {
    let pair = load_pair();
    // Every one of the 16 output bytes is distinct, so any field-order or
    // shift-order transposition changes the result.
    let m = TflacMd5::new(0x0403_0201, 0x0807_0605, 0x0C0B_0A09, 0x100F_0E0D);
    pair.assert_same("C7", &m);
    let out = pair.c.digest(&m);
    assert_eq!(
        out,
        [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16],
        "C reference layout is little-endian per field, fields in a,b,c,d order"
    );
    // Rotate the distinct pattern through all 16 byte slots.
    for rot in 0..16u32 {
        let base = 1 + rot;
        let w = |k: u32| -> u32 {
            u32::from_le_bytes([
                (base + 4 * k) as u8,
                (base + 4 * k + 1) as u8,
                (base + 4 * k + 2) as u8,
                (base + 4 * k + 3) as u8,
            ])
        };
        pair.assert_same(&format!("C7/rot{rot}"), &TflacMd5::new(w(0), w(1), w(2), w(3)));
    }
    random_sweep(&pair, "C7", PER_ROW_RANDOM);
}

// --------------------------------------------------------------------- C8 --

#[test]
fn cfg_c8_high_bit_and_sign_patterns() {
    let pair = load_pair();
    let pats: [u32; 10] = [
        0x8080_8080,
        0x8000_0000,
        0x0000_0080,
        0x7F7F_7F7F,
        0x7FFF_FFFF,
        0xFF00_FF00,
        0x00FF_00FF,
        0x8000_0001,
        0xFFFF_FF80,
        0x80FF_7F00,
    ];
    // Cross every pattern into every field position, plus all-same.
    for (i, &p) in pats.iter().enumerate() {
        pair.assert_same(&format!("C8/same#{i}"), &TflacMd5::new(p, p, p, p));
        pair.assert_same(&format!("C8/a#{i}"), &TflacMd5::new(p, 0, 0, 0));
        pair.assert_same(&format!("C8/b#{i}"), &TflacMd5::new(0, p, 0, 0));
        pair.assert_same(&format!("C8/c#{i}"), &TflacMd5::new(0, 0, p, 0));
        pair.assert_same(&format!("C8/d#{i}"), &TflacMd5::new(0, 0, 0, p));
        for (j, &q) in pats.iter().enumerate() {
            pair.assert_same(&format!("C8/mix#{i}x{j}"), &TflacMd5::new(p, q, p ^ q, !q));
        }
    }
}

// --------------------------------------------------------------------- C9 --

#[test]
fn cfg_c9_single_bit_sweep() {
    let pair = load_pair();
    // Exhaustive over all 128 struct bit positions, each set alone, and each
    // cleared alone from all-ones.
    for bit in 0..128u32 {
        let word = bit / 32;
        let shift = bit % 32;
        let v = 1u32 << shift;
        let set = TflacMd5::new(
            if word == 0 { v } else { 0 },
            if word == 1 { v } else { 0 },
            if word == 2 { v } else { 0 },
            if word == 3 { v } else { 0 },
        );
        pair.assert_same(&format!("C9/set{bit}"), &set);

        let clr = TflacMd5::new(
            if word == 0 { !v } else { u32::MAX },
            if word == 1 { !v } else { u32::MAX },
            if word == 2 { !v } else { u32::MAX },
            if word == 3 { !v } else { u32::MAX },
        );
        pair.assert_same(&format!("C9/clr{bit}"), &clr);
    }
}

// -------------------------------------------------------------------- C10 --

#[test]
fn cfg_c10_random_full_range() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED);
    for i in 0..20_000usize {
        let m = rng.next_md5();
        pair.assert_same(&format!("C10/#{i}"), &m);
    }
}

// -------------------------------------------------------------------- C11 --

#[test]
fn cfg_c11_misaligned_out() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 11);
    // `out` is `uint8_t*`, so any alignment is legal for the C.
    for out_off in [0usize, 1, 2, 3, 5, 7] {
        for i in 0..256 {
            let m = rng.next_md5();
            let run = |imp: &common::Impl| -> Vec<u8> {
                let mut buf = AlignedBuf::new(64, 0xA5);
                unsafe {
                    imp.call(&m as *const TflacMd5, buf.as_mut_ptr().add(out_off));
                }
                buf.bytes().to_vec()
            };
            let (gc, gr) = (run(&pair.c), run(&pair.rust));
            assert_eq!(gc, gr, "[C11/off{out_off}/#{i}] {m:?}\n C={gc:02x?}\n R={gr:02x?}");
            // The 16 written bytes must sit exactly at out_off..out_off+16.
            assert_eq!(&gc[..out_off], &vec![0xA5u8; out_off][..]);
            assert!(gc[out_off + 16..].iter().all(|&b| b == 0xA5));
        }
    }
}

// -------------------------------------------------------------------- C12 --

#[test]
fn cfg_c12_misaligned_m() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 12);
    for m_off in [0usize, 1, 2, 3, 4, 5, 6, 7] {
        for i in 0..256 {
            let m = rng.next_md5();
            let run = |imp: &common::Impl| -> [u8; 16] {
                let mut src = AlignedBuf::new(32, 0);
                src.put_md5(m_off, &m);
                let mut out = [0u8; 16];
                unsafe {
                    imp.call(
                        src.as_mut_ptr().add(m_off) as *const TflacMd5,
                        out.as_mut_ptr(),
                    );
                }
                out
            };
            let (gc, gr) = (run(&pair.c), run(&pair.rust));
            assert_eq!(gc, gr, "[C12/moff{m_off}/#{i}] {m:?}\n C={gc:02x?}\n R={gr:02x?}");
        }
    }
}

// -------------------------------------------------------------------- C13 --

#[test]
fn cfg_c13_oversized_out_tail_untouched() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 13);
    for i in 0..1024 {
        let m = rng.next_md5();
        let run = |imp: &common::Impl| -> Vec<u8> {
            let mut buf = AlignedBuf::new(64, 0xAA);
            unsafe { imp.call(&m as *const TflacMd5, buf.as_mut_ptr()) };
            buf.bytes().to_vec()
        };
        let (gc, gr) = (run(&pair.c), run(&pair.rust));
        assert_eq!(gc, gr, "[C13/#{i}] {m:?}");
        assert!(
            gc[16..].iter().all(|&b| b == 0xAA),
            "[C13/#{i}] C wrote past 16 bytes: {gc:02x?}"
        );
        assert!(
            gr[16..].iter().all(|&b| b == 0xAA),
            "[C13/#{i}] Rust wrote past 16 bytes: {gr:02x?}"
        );
    }
}

// -------------------------------------------------------------------- C15+ --
// Aliasing: neither C parameter is `restrict`, so `out` may overlap `*m`.

#[test]
fn cfg_c15_full_alias_out_equals_m() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 15);
    for i in 0..512 {
        let m = rng.next_md5();
        assert_same_in_buffer(&pair, &format!("C15/#{i}"), 48, 16, 16, &m, 0x5C);
    }
}

#[test]
fn cfg_c16_alias_plus_one() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 16);
    for i in 0..512 {
        let m = rng.next_md5();
        assert_same_in_buffer(&pair, &format!("C16/#{i}"), 48, 16, 17, &m, 0x5C);
    }
}

#[test]
fn cfg_c17_alias_plus_four() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 17);
    for i in 0..512 {
        let m = rng.next_md5();
        assert_same_in_buffer(&pair, &format!("C17/#{i}"), 48, 16, 20, &m, 0x5C);
    }
}

#[test]
fn cfg_c18_alias_plus_eight_and_twelve() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 18);
    for i in 0..512 {
        let m = rng.next_md5();
        assert_same_in_buffer(&pair, &format!("C18a/#{i}"), 48, 16, 24, &m, 0x5C);
        assert_same_in_buffer(&pair, &format!("C18b/#{i}"), 48, 16, 28, &m, 0x5C);
    }
}

#[test]
fn cfg_c19_reverse_alias_minus_eight() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 19);
    for i in 0..512 {
        let m = rng.next_md5();
        // out starts 8 bytes before m, so out's tail overlaps a and b.
        assert_same_in_buffer(&pair, &format!("C19/#{i}"), 48, 16, 8, &m, 0x5C);
    }
}

#[test]
fn cfg_c20_alias_offset_sweep_random() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 20);
    // m sits at offset 16 in a 48-byte buffer; sweep out over -16..=+16.
    for out_off in 0..=32usize {
        for i in 0..64 {
            let m = rng.next_md5();
            assert_same_in_buffer(
                &pair,
                &format!("C20/out{out_off}/#{i}"),
                48,
                16,
                out_off,
                &m,
                0x5C,
            );
        }
    }
    // And a randomized walk over both offsets, including misaligned m.
    for i in 0..2_000 {
        let m = rng.next_md5();
        let m_off = rng.below(17) as usize;
        let out_off = rng.below(33) as usize;
        assert_same_in_buffer(
            &pair,
            &format!("C20/rand#{i}(m{m_off},o{out_off})"),
            48,
            m_off,
            out_off,
            &m,
            0x5C,
        );
    }
}

// -------------------------------------------------------------------- C21 --

#[test]
fn cfg_c21_repeated_calls_same_buffer() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 21);
    for i in 0..512 {
        let m1 = rng.next_md5();
        let m2 = rng.next_md5();
        let run = |imp: &common::Impl| -> Vec<[u8; 16]> {
            let mut out = [0u8; 16];
            let mut seq = Vec::new();
            for m in [&m1, &m2, &m1] {
                unsafe { imp.call(m as *const TflacMd5, out.as_mut_ptr()) };
                seq.push(out);
            }
            seq
        };
        let (gc, gr) = (run(&pair.c), run(&pair.rust));
        assert_eq!(gc, gr, "[C21/#{i}]");
        // No residual state: the third call reproduces the first exactly.
        assert_eq!(gc[0], gc[2], "[C21/#{i}] C is not stateless");
        assert_eq!(gr[0], gr[2], "[C21/#{i}] Rust is not stateless");
    }
}

// -------------------------------------------------------------------- C22 --

#[test]
fn cfg_c22_stateless_and_call_order_independent() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 22);
    for i in 0..512 {
        let m = rng.next_md5();

        // Two separate buffers, same struct.
        let a_c = pair.c.digest(&m);
        let b_c = pair.c.digest(&m);
        let a_r = pair.rust.digest(&m);
        let b_r = pair.rust.digest(&m);
        assert_eq!(a_c, b_c, "[C22/#{i}] C not deterministic");
        assert_eq!(a_r, b_r, "[C22/#{i}] Rust not deterministic");
        assert_eq!(a_c, a_r, "[C22/#{i}] divergence");

        // Rust-then-C ordering must agree with C-then-Rust.
        let r_first = pair.rust.digest(&m);
        let c_second = pair.c.digest(&m);
        assert_eq!(r_first, c_second, "[C22/#{i}] order-dependent divergence");

        // The input struct must be unmodified by either implementation
        // (parameter is `const tflac_md5 *`).
        let mut probe = m;
        let mut sink = [0u8; 16];
        unsafe { pair.c.call(&probe as *const TflacMd5, sink.as_mut_ptr()) };
        assert_eq!(probe, m, "[C22/#{i}] C mutated its const input");
        unsafe { pair.rust.call(&probe as *const TflacMd5, sink.as_mut_ptr()) };
        assert_eq!(probe, m, "[C22/#{i}] Rust mutated its const input");
        probe.a ^= 0; // keep `probe` mutable-used
    }
}

// ------------------------------------------------- non-vacuity of C15..C20 --

/// The aliasing rows would be worthless if the C produced the same bytes with
/// and without overlap. This proves they discriminate: it builds the "naive"
/// model (read all four fields ONCE up front, then write 16 bytes) and shows
/// the real C disagrees with it under overlap — so any Rust implementation that
/// hoisted its loads (as the original `&tflac_md5` + `copy_from_slice` version
/// did) would be caught by C15–C20.
#[test]
fn cfg_aliasing_rows_are_discriminating() {
    let pair = load_pair();
    let m = TflacMd5::new(0x0403_0201, 0x0807_0605, 0x0C0B_0A09, 0x100F_0E0D);

    // Model of a hoisted-load implementation.
    fn hoisted_model(buf_len: usize, m_off: usize, out_off: usize, m: &TflacMd5, fill: u8) -> Vec<u8> {
        let mut buf = vec![fill; buf_len];
        let mut raw = [0u8; 16];
        raw[0..4].copy_from_slice(&m.a.to_le_bytes());
        raw[4..8].copy_from_slice(&m.b.to_le_bytes());
        raw[8..12].copy_from_slice(&m.c.to_le_bytes());
        raw[12..16].copy_from_slice(&m.d.to_le_bytes());
        buf[m_off..m_off + 16].copy_from_slice(&raw);
        // All four fields read up front from the pre-call state, then written.
        let snapshot: [u8; 16] = buf[m_off..m_off + 16].try_into().unwrap();
        buf[out_off..out_off + 16].copy_from_slice(&snapshot);
        buf
    }

    fn c_actual(imp: &common::Impl, buf_len: usize, m_off: usize, out_off: usize, m: &TflacMd5, fill: u8) -> Vec<u8> {
        let mut buf = AlignedBuf::new(buf_len, fill);
        buf.put_md5(m_off, m);
        // SAFETY: both windows are inside the buffer; overlap is legal.
        unsafe {
            imp.call(
                buf.as_mut_ptr().add(m_off) as *const TflacMd5,
                buf.as_mut_ptr().add(out_off),
            )
        };
        buf.bytes().to_vec()
    }

    let mut differing = 0usize;
    // Overlap offsets where the cascade must show up (out shifted off m).
    for out_off in [8usize, 15, 17, 20, 24, 28] {
        let real = c_actual(&pair.c, 48, 16, out_off, &m, 0x5C);
        let naive = hoisted_model(48, 16, out_off, &m, 0x5C);
        if real != naive {
            differing += 1;
        }
        // Whatever the C does, Rust must do the same.
        let rust_real = c_actual(&pair.rust, 48, 16, out_off, &m, 0x5C);
        assert_eq!(real, rust_real, "[non-vacuity/out{out_off}] Rust != C");
    }
    assert!(
        differing >= 4,
        "aliasing rows are vacuous: the C agreed with the hoisted-load model in \
         all but {} of the overlap cases, so C15-C20 could not catch a hoisting bug",
        6 - differing
    );

    // Concretely: with out = m+1 the C cascades one byte across the window,
    // because each store feeds the next iteration's read.
    let real = c_actual(&pair.c, 48, 16, 17, &m, 0x5C);
    assert_eq!(
        &real[17..33],
        &[real[16]; 16],
        "expected the byte-propagation cascade the per-byte reload produces"
    );
    let rust_real = c_actual(&pair.rust, 48, 16, 17, &m, 0x5C);
    assert_eq!(real, rust_real, "Rust must cascade identically");
}
