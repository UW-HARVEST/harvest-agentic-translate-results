//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Every test loads BOTH `.so`s through `libloading` and compares the whole
//! `calloc`'d region byte-for-byte. Randomized inputs use a fixed seed.

mod common;

use common::{Differ, Libs, Rng, SEED, n_bytes};

/* ================================================================== */
/* Harness liveness — make sure we are not vacuously comparing nothing */
/* ================================================================== */

#[test]
fn harness_is_live() {
    let libs = Libs::load();
    println!("C   .so: {}", libs.c_path.display());
    println!("Rust.so: {}", libs.rust_path.display());
    let d = Differ::new(&libs);

    // Known-good base64 vectors, asserted against the C ground truth so a
    // broken loader can never make the differential tests pass by accident.
    for (input, want) in [
        (&b"abc"[..], &b"YWJj"[..]),
        (&b"a"[..], &b"YQ=="[..]),
        (&b"ab"[..], &b"YWI="[..]),
        (&b"Man"[..], &b"TWFu"[..]),
        (&b"hello world"[..], &b"aGVsbG8gd29ybGQ="[..]),
    ] {
        let c = d
            .c_output(input.len() as i32, input)
            .expect("C returned NULL");
        assert_eq!(
            &c[..want.len()],
            want,
            "C ground truth for {:?}",
            String::from_utf8_lossy(input)
        );
        let r = d
            .rust_output(input.len() as i32, input)
            .expect("Rust returned NULL");
        assert_eq!(c, r, "C/Rust for {:?}", String::from_utf8_lossy(input));
    }

    // '+' (sextet 62) and '/' (sextet 63) really are reachable.
    let hi = d.c_output(3, &[0xFB, 0xFF, 0xFF]).unwrap();
    assert!(
        hi.contains(&b'+') || hi.contains(&b'/'),
        "expected +/ in {:?}",
        String::from_utf8_lossy(&hi)
    );

    // The differential comparator must really be comparing a non-empty region:
    // `comparable_len` has to cover the emitted bytes, not zero bytes.
    assert_eq!(common::comparable_len(3), 8);
    assert_eq!(common::comparable_len(11), 18);
    d.assert_same("liveness", 11, b"hello world");
    assert!(
        d.calls.get() > 0,
        "the differential comparator was never invoked"
    );
}

/* ================================================================== */
/* C1 / C2 / C3 — explicit mode, each padding class                    */
/* ================================================================== */

fn padding_class_row(ctx: &str, rem: usize, seed: u64) {
    let libs = Libs::load();
    let d = Differ::new(&libs);
    let mut rng = Rng::new(seed);

    let mut len = if rem == 0 { 3 } else { rem };
    while len <= 98 {
        assert_eq!(len % 3, rem % 3);
        for _ in 0..64 {
            let buf = rng.bytes(len);
            d.assert_same(ctx, len as i32, &buf);
        }
        len += 3;
    }
    println!("{ctx}: {} differential calls", d.calls.get());
}

#[test]
fn c1_len_mod3_eq0() {
    padding_class_row("C1 len%3==0", 0, SEED ^ 1);
}

#[test]
fn c2_len_mod3_eq1() {
    padding_class_row("C2 len%3==1", 1, SEED ^ 2);
}

#[test]
fn c3_len_mod3_eq2() {
    padding_class_row("C3 len%3==2", 2, SEED ^ 3);
}

/* ================================================================== */
/* C4 — exhaustive length sweep 1..=200                                */
/* ================================================================== */

#[test]
fn c4_length_sweep_1_to_200() {
    let libs = Libs::load();
    let d = Differ::new(&libs);
    let mut rng = Rng::new(SEED ^ 4);

    for len in 1usize..=200 {
        for _ in 0..24 {
            let buf = rng.bytes(len);
            d.assert_same("C4 length sweep", len as i32, &buf);
        }
    }
    println!("C4: {} differential calls", d.calls.get());
}

/* ================================================================== */
/* C5 — size == 1, all 256 byte values                                 */
/* ================================================================== */

#[test]
fn c5_size_one_all_bytes() {
    let libs = Libs::load();
    let d = Differ::new(&libs);
    for b in 0u16..=255 {
        d.assert_same("C5 size=1", 1, &[b as u8]);
    }
    assert_eq!(d.calls.get(), 256);
}

/* ================================================================== */
/* C6 — size == 2, all 65536 byte pairs                                */
/* ================================================================== */

#[test]
fn c6_size_two_all_pairs() {
    let libs = Libs::load();
    let d = Differ::new(&libs);
    for a in 0u16..=255 {
        for b in 0u16..=255 {
            d.assert_same("C6 size=2", 2, &[a as u8, b as u8]);
        }
    }
    assert_eq!(d.calls.get(), 65536);
}

/* ================================================================== */
/* C7 — size == 3, one complete group                                  */
/* ================================================================== */

#[test]
fn c7_size_three_full_group() {
    let libs = Libs::load();
    let d = Differ::new(&libs);
    let mut rng = Rng::new(SEED ^ 7);

    d.assert_same("C7 zeros", 3, &[0x00, 0x00, 0x00]);
    d.assert_same("C7 ones", 3, &[0xFF, 0xFF, 0xFF]);
    for _ in 0..20_000 {
        let buf = rng.bytes(3);
        d.assert_same("C7 random triple", 3, &buf);
    }
    println!("C7: {} differential calls", d.calls.get());
}

/* ================================================================== */
/* C8 — printable ASCII content                                        */
/* ================================================================== */

#[test]
fn c8_printable_ascii() {
    let libs = Libs::load();
    let d = Differ::new(&libs);
    let mut rng = Rng::new(SEED ^ 8);

    for _ in 0..3000 {
        let len = rng.range(1, 300) as usize;
        let buf = rng.bytes_in(len, 0x20, 0x7E);
        d.assert_same("C8 printable ascii", len as i32, &buf);
    }
    println!("C8: {} differential calls", d.calls.get());
}

/* ================================================================== */
/* C9 — high-bit bytes only (signed char -> unsigned char)             */
/* ================================================================== */

#[test]
fn c9_high_bit_bytes() {
    let libs = Libs::load();
    let d = Differ::new(&libs);
    let mut rng = Rng::new(SEED ^ 9);

    for _ in 0..3000 {
        let len = rng.range(1, 300) as usize;
        let buf = rng.bytes_in(len, 0x80, 0xFF);
        d.assert_same("C9 high-bit bytes", len as i32, &buf);
    }
    // every single high byte on its own, and as a full triple
    for b in 0x80u16..=0xFF {
        d.assert_same("C9 single high byte", 1, &[b as u8]);
        d.assert_same("C9 high triple", 3, &[b as u8, b as u8, b as u8]);
    }
    println!("C9: {} differential calls", d.calls.get());
}

/* ================================================================== */
/* C10 / C11 — all-zero and all-0xFF content                           */
/* ================================================================== */

#[test]
fn c10_all_zero_bytes() {
    let libs = Libs::load();
    let d = Differ::new(&libs);
    for len in 1usize..=150 {
        let buf = vec![0u8; len];
        d.assert_same("C10 all zero", len as i32, &buf);
    }
    // sanity: sextets are all 0 -> output is all 'A'
    let out = d.c_output(9, &vec![0u8; 9]).unwrap();
    assert_eq!(&out[..12], b"AAAAAAAAAAAA");
    println!("C10: {} differential calls", d.calls.get());
}

#[test]
fn c11_all_ff_bytes() {
    let libs = Libs::load();
    let d = Differ::new(&libs);
    for len in 1usize..=150 {
        let buf = vec![0xFFu8; len];
        d.assert_same("C11 all 0xFF", len as i32, &buf);
    }
    // sanity: sextets are all 63 -> encode()'s `return '/'` catch-all
    let out = d.c_output(9, &vec![0xFFu8; 9]).unwrap();
    assert_eq!(&out[..12], b"////////////");
    println!("C11: {} differential calls", d.calls.get());
}

/* ================================================================== */
/* C12 — every sextet value 0..=63 in every output position            */
/* ================================================================== */

#[test]
fn c12_all_sextets_in_all_positions() {
    let libs = Libs::load();
    let d = Differ::new(&libs);

    for s in 0u8..64 {
        // b4 = b1 >> 2
        d.assert_same("C12 pos0", 3, &[s << 2, 0x00, 0x00]);
        // b5 = ((b1 & 3) << 4) | (b2 >> 4)
        d.assert_same("C12 pos1", 3, &[(s >> 4) & 0x03, (s & 0x0F) << 4, 0x00]);
        // b6 = ((b2 & 0xf) << 2) | (b3 >> 6)
        d.assert_same("C12 pos2", 3, &[0x00, (s >> 2) & 0x0F, (s & 0x03) << 6]);
        // b7 = b3 & 0x3f
        d.assert_same("C12 pos3", 3, &[0x00, 0x00, s]);
    }

    // Pin down the two special branches of encode() explicitly.
    let plus = d.c_output(3, &[62 << 2, 0, 0]).unwrap();
    assert_eq!(plus[0], b'+', "sextet 62 must encode to '+'");
    let slash = d.c_output(3, &[63 << 2, 0, 0]).unwrap();
    assert_eq!(slash[0], b'/', "sextet 63 must encode to '/'");

    // and the three arithmetic branches at their boundaries
    for (s, want) in [
        (0u8, b'A'),
        (25, b'Z'),
        (26, b'a'),
        (51, b'z'),
        (52, b'0'),
        (61, b'9'),
        (62, b'+'),
        (63, b'/'),
    ] {
        let out = d.c_output(3, &[s << 2, 0, 0]).unwrap();
        assert_eq!(out[0], want, "sextet {s}");
        let rout = d.rust_output(3, &[s << 2, 0, 0]).unwrap();
        assert_eq!(out, rout, "sextet {s} C vs Rust");
    }
    println!("C12: {} differential calls", d.calls.get());
}

/* ================================================================== */
/* C13 — embedded NUL bytes are ordinary data in explicit mode         */
/* ================================================================== */

#[test]
fn c13_embedded_nuls_explicit_mode() {
    let libs = Libs::load();
    let d = Differ::new(&libs);
    let mut rng = Rng::new(SEED ^ 13);

    for _ in 0..3000 {
        let len = rng.range(1, 120) as usize;
        let mut buf = rng.bytes(len);
        // sprinkle NULs, including at the very first position
        let holes = rng.range(1, 6) as usize;
        for _ in 0..holes {
            let at = rng.below(len as u32) as usize;
            buf[at] = 0;
        }
        if rng.below(4) == 0 {
            buf[0] = 0;
        }
        d.assert_same("C13 embedded NULs", len as i32, &buf);
    }
    println!("C13: {} differential calls", d.calls.get());
}

/* ================================================================== */
/* C14 / C15 / C16 / C17 — strlen mode (size == 0)                     */
/* ================================================================== */

#[test]
fn c14_strlen_mode_random_ascii() {
    let libs = Libs::load();
    let d = Differ::new(&libs);
    let mut rng = Rng::new(SEED ^ 14);

    for _ in 0..3000 {
        let len = rng.range(0, 200) as usize;
        let mut buf = rng.bytes_in(len, 0x01, 0x7F);
        buf.push(0); // NUL terminator
        d.assert_same("C14 strlen mode ascii", 0, &buf);
    }
    println!("C14: {} differential calls", d.calls.get());
}

#[test]
fn c15_strlen_mode_high_bit() {
    let libs = Libs::load();
    let d = Differ::new(&libs);
    let mut rng = Rng::new(SEED ^ 15);

    for _ in 0..3000 {
        let len = rng.range(0, 200) as usize;
        let mut buf = rng.bytes_in(len, 0x80, 0xFF);
        buf.push(0);
        d.assert_same("C15 strlen mode high-bit", 0, &buf);
    }
    println!("C15: {} differential calls", d.calls.get());
}

#[test]
fn c16_strlen_mode_data_after_nul() {
    let libs = Libs::load();
    let d = Differ::new(&libs);
    let mut rng = Rng::new(SEED ^ 16);

    for _ in 0..3000 {
        let len = rng.range(0, 100) as usize;
        let mut buf = rng.bytes_in(len, 0x01, 0xFF);
        buf.push(0);
        // trailing garbage that must NOT be encoded
        let tail = rng.range(1, 50) as usize;
        buf.extend(rng.bytes_in(tail, 0x01, 0xFF));

        d.assert_same("C16 strlen mode, data after NUL", 0, &buf);

        // the measured length really is the prefix length
        let out = d.c_output(0, &buf).unwrap();
        let want_written = 4 * ((len + 2) / 3);
        assert!(
            out.len() >= want_written,
            "n must cover the emitted bytes (len={len})"
        );
        assert!(
            out[want_written..].iter().all(|&b| b == 0),
            "bytes past the emitted region must be calloc zeros (len={len})"
        );
    }
    println!("C16: {} differential calls", d.calls.get());
}

#[test]
fn c17_strlen_mode_each_padding_class() {
    let libs = Libs::load();
    let d = Differ::new(&libs);
    let mut rng = Rng::new(SEED ^ 17);

    for rem in 0..3usize {
        let mut len = if rem == 0 { 3 } else { rem };
        while len <= 99 {
            assert_eq!(len % 3, rem);
            for _ in 0..40 {
                let mut buf = rng.bytes_in(len, 0x01, 0xFF);
                buf.push(0);
                d.assert_same("C17 strlen mode padding class", 0, &buf);
            }
            len += 3;
        }
    }
    // and the empty string (measured length 0)
    d.assert_same("C17 empty string", 0, &[0u8]);
    println!("C17: {} differential calls", d.calls.get());
}

/* ================================================================== */
/* C18 — explicit size smaller than the real buffer (prefix encode)    */
/* ================================================================== */

#[test]
fn c18_size_smaller_than_buffer() {
    let libs = Libs::load();
    let d = Differ::new(&libs);
    let mut rng = Rng::new(SEED ^ 18);

    for _ in 0..4000 {
        let buf_len = rng.range(2, 300) as usize;
        let buf = rng.bytes(buf_len);
        let size = rng.range(1, buf_len as u32) as i32; // 1 ..= buf_len
        d.assert_same("C18 truncating prefix", size, &buf);
    }
    println!("C18: {} differential calls", d.calls.get());
}

/* ================================================================== */
/* C19 / C20 — large buffers                                           */
/* ================================================================== */

#[test]
fn c19_large_buffers() {
    let libs = Libs::load();
    let d = Differ::new(&libs);
    let mut rng = Rng::new(SEED ^ 19);

    for len in [4096usize, 65536, 1 << 20] {
        let buf = rng.bytes(len);
        d.assert_same("C19 large buffer", len as i32, &buf);
        // and in strlen mode over the same magnitude
        let mut s = rng.bytes_in(len, 0x01, 0xFF);
        s.push(0);
        d.assert_same("C19 large buffer strlen mode", 0, &s);
    }
    println!("C19: {} differential calls", d.calls.get());
}

#[test]
fn c20_large_buffers_with_padding() {
    let libs = Libs::load();
    let d = Differ::new(&libs);
    let mut rng = Rng::new(SEED ^ 20);

    for base in [4096usize, 65536, 1 << 20] {
        for delta in 0..3usize {
            // force each residue class at a large offset
            let len = base - (base % 3) + delta + 3;
            let buf = rng.bytes(len);
            assert_eq!(len % 3, delta);
            d.assert_same("C20 large buffer padding", len as i32, &buf);
        }
    }
    println!("C20: {} differential calls", d.calls.get());
}

/* ================================================================== */
/* C21 / C22 / C23 / C24 — negative sizes with n > 0 (loop skipped)    */
/* ================================================================== */

/// A negative `size` never enters the read loop (`0 < negative` is false), so
/// the well-defined result is an all-zero `calloc(1, n)` buffer.
fn negative_size_row(ctx: &str, sizes: &[i32], expected_n: &[i32]) {
    let libs = Libs::load();
    let d = Differ::new(&libs);
    let mut rng = Rng::new(SEED ^ 21);

    for (i, &size) in sizes.iter().enumerate() {
        assert_eq!(
            n_bytes(size),
            expected_n[i],
            "{ctx}: n for size={size} is not the documented value"
        );
        assert!(n_bytes(size) > 0 || n_bytes(size) == 0);

        // the source buffer content must be irrelevant, so randomize it
        for _ in 0..25 {
            let nlen = rng.range(1, 64) as usize;
            let buf = rng.bytes(nlen);
            d.assert_same(ctx, size, &buf);
        }

        // the C result must be all zeros (nothing is ever emitted)
        let buf = rng.bytes(16);
        let out = d.c_output(size, &buf).unwrap();
        assert_eq!(out.len(), expected_n[i] as usize);
        assert!(
            out.iter().all(|&b| b == 0),
            "{ctx}: size={size} must yield an all-zero buffer, got {out:?}"
        );
        assert_eq!(d.rust_output(size, &buf).unwrap(), out, "{ctx} size={size}");
    }
    println!("{ctx}: {} differential calls", d.calls.get());
}

#[test]
fn c21_small_negative_sizes() {
    // n = 3, 2, 0  (size = -3 gives calloc(1, 0), still non-NULL on glibc)
    negative_size_row("C21 small negative size", &[-1, -2, -3], &[3, 2, 0]);
}

#[test]
fn c22_negative_size_wrapping_to_small_positive_n() {
    negative_size_row(
        "C22 negative size, int wrap to small positive n",
        &[-1073741823, -1073741822, -1073741821, -1073741820, -1073741700],
        &[5, 6, 8, 9, 169],
    );
}

#[test]
fn c23_negative_size_wrapping_to_zero() {
    negative_size_row(
        "C23 size*4 wraps to exactly 0",
        &[i32::MIN, -(1 << 30)],
        &[4, 4],
    );
}

#[test]
fn c24_negative_size_wrapping_to_large_n() {
    negative_size_row(
        "C24 negative size, int wrap to large n",
        &[-1072991824],
        &[1_000_004],
    );
}

/* ================================================================== */
/* C25 — broad randomized fuzz over the whole well-defined input space */
/* ================================================================== */

#[test]
fn c25_randomized_fuzz_sweep() {
    let libs = Libs::load();
    let d = Differ::new(&libs);
    let mut rng = Rng::new(SEED ^ 25);

    let mut modes = [0u32; 6];
    for _ in 0..4000 {
        let mode = rng.below(6);
        modes[mode as usize] += 1;
        match mode {
            // strlen mode
            0 => {
                let len = rng.range(0, 400) as usize;
                let mut buf = rng.bytes_in(len, 0x01, 0xFF);
                buf.push(0);
                // sometimes append garbage past the NUL
                if rng.below(2) == 0 {
                    let t = rng.range(1, 30) as usize;
                    buf.extend(rng.bytes_in(t, 0x01, 0xFF));
                }
                d.assert_same("C25 strlen mode", 0, &buf);
            }
            // explicit mode, exactly the buffer length
            1 => {
                let len = rng.range(1, 400) as usize;
                let buf = rng.bytes(len);
                d.assert_same("C25 explicit exact", len as i32, &buf);
            }
            // explicit mode, truncating prefix
            2 => {
                let len = rng.range(2, 400) as usize;
                let buf = rng.bytes(len);
                let size = rng.range(1, len as u32) as i32;
                d.assert_same("C25 explicit prefix", size, &buf);
            }
            // negative size with n > 0
            3 => {
                let size = match rng.below(4) {
                    0 => -1,
                    1 => -2,
                    2 => -3,
                    _ => -(1 << 30) + rng.range(1, 100_000) as i32,
                };
                let n = n_bytes(size);
                if n < 0 {
                    continue;
                }
                let nlen = rng.range(1, 32) as usize;
                let buf = rng.bytes(nlen);
                d.assert_same("C25 negative n>0", size, &buf);
            }
            // negative size with n <= 0 (calloc must fail on both sides)
            4 => {
                let size = -(rng.range(4, 1_000_000) as i32);
                let nlen = rng.range(1, 32) as usize;
                let buf = rng.bytes(nlen);
                d.assert_same("C25 negative n<=0", size, &buf);
            }
            // positive size whose int overflow makes calloc fail
            _ => {
                let size = rng.range(536_870_912, 1_073_741_820) as i32;
                if n_bytes(size) > 0 {
                    continue; // would be C undefined behaviour, skip
                }
                let nlen = rng.range(1, 32) as usize;
                let buf = rng.bytes(nlen);
                d.assert_same("C25 positive overflow", size, &buf);
            }
        }
    }
    println!(
        "C25: {} differential calls, mode histogram {:?}",
        d.calls.get(),
        modes
    );
    assert!(modes.iter().all(|&c| c > 0), "every fuzz mode must be hit");
}
