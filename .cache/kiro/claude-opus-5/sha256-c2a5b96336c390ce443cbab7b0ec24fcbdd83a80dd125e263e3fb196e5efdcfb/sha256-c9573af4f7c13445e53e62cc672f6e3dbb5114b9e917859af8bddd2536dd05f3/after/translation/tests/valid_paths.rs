//! Phase B — valid-path differential tests.
//!
//! One `#[test]` per row of `CONFIGS.md`, each driving BOTH the C `.so` and the
//! Rust `.so` through `libloading` with many seeded-random inputs.

mod common;

use common::{assert_identical, both, Case, Rng};

/// C1 — `bin_len = 0`, `hex_maxlen = 1` (minimum accepted).
#[test]
fn c1_empty_exact() {
    let (c, r) = both();
    let empty: [u8; 0] = [];
    for off in 0..8 {
        let mut case = Case::exact(&empty);
        case.hex_off = off;
        assert_identical(&c, &r, &case, &format!("C1 off={off}"));
    }
}

/// C2 — `bin_len = 0` with randomized slack; the tail must stay untouched.
#[test]
fn c2_empty_slack() {
    let (c, r) = both();
    let empty: [u8; 0] = [];
    let mut rng = Rng::new(0xC000_0002);
    for i in 0..512 {
        let hex_maxlen = rng.range(2, 4096);
        let mut case = Case::slack(&empty, hex_maxlen);
        case.fill = rng.next_u8();
        assert_identical(&c, &r, &case, &format!("C2 i={i} hex_maxlen={hex_maxlen}"));
    }
}

/// C3 — `bin = NULL` with `bin_len = 0`: accepted, never dereferenced.
#[test]
fn c3_null_bin_zero_len() {
    let (c, r) = both();
    let mut rng = Rng::new(0xC000_0003);
    for i in 0..256 {
        let hex_maxlen = if i == 0 { 1 } else { rng.range(1, 512) };
        let case = Case {
            bin: None,
            bin_len: 0,
            hex_maxlen,
            hex_alloc: hex_maxlen,
            hex_off: i % 8,
            bin_off: 0,
            fill: rng.next_u8(),
        };
        assert_identical(&c, &r, &case, &format!("C3 i={i} hex_maxlen={hex_maxlen}"));
    }
}

/// C4 — `bin_len = 1`, exhaustively all 256 byte values, exact `hex_maxlen`.
#[test]
fn c4_single_byte_exhaustive_exact() {
    let (c, r) = both();
    for b in 0u16..256 {
        let bin = [b as u8];
        let case = Case::exact(&bin);
        assert_identical(&c, &r, &case, &format!("C4 byte={:#04x}", b));
    }
}

/// C5 — `bin_len = 1`, all 256 byte values, randomized slack.
#[test]
fn c5_single_byte_exhaustive_slack() {
    let (c, r) = both();
    let mut rng = Rng::new(0xC000_0005);
    for b in 0u16..256 {
        for _ in 0..8 {
            let bin = [b as u8];
            let hex_maxlen = rng.range(4, 64);
            let mut case = Case::slack(&bin, hex_maxlen);
            case.fill = rng.next_u8();
            assert_identical(&c, &r, &case, &format!("C5 byte={:#04x} max={hex_maxlen}", b));
        }
    }
}

/// C6 — small random inputs, exact `hex_maxlen`.
#[test]
fn c6_small_random_exact() {
    let (c, r) = both();
    let mut rng = Rng::new(0xC000_0006);
    for i in 0..2000 {
        let n = rng.range(2, 16);
        let bin = rng.bytes(n);
        let case = Case::exact(&bin);
        assert_identical(&c, &r, &case, &format!("C6 i={i} n={n}"));
    }
}

/// C7 — small random inputs, randomized slack.
#[test]
fn c7_small_random_slack() {
    let (c, r) = both();
    let mut rng = Rng::new(0xC000_0007);
    for i in 0..2000 {
        let n = rng.range(2, 16);
        let bin = rng.bytes(n);
        let hex_maxlen = rng.range(n * 2 + 1, n * 2 + 128);
        let mut case = Case::slack(&bin, hex_maxlen);
        case.fill = rng.next_u8();
        assert_identical(&c, &r, &case, &format!("C7 i={i} n={n} max={hex_maxlen}"));
    }
}

/// C8 — medium random inputs, exact `hex_maxlen`.
#[test]
fn c8_medium_random_exact() {
    let (c, r) = both();
    let mut rng = Rng::new(0xC000_0008);
    for i in 0..300 {
        let n = rng.range(17, 4096);
        let bin = rng.bytes(n);
        let case = Case::exact(&bin);
        assert_identical(&c, &r, &case, &format!("C8 i={i} n={n}"));
    }
}

/// C9 — large random inputs with slack.
#[test]
fn c9_large_random_slack() {
    let (c, r) = both();
    let mut rng = Rng::new(0xC000_0009);
    for i in 0..40 {
        let n = rng.range(4096, 65536);
        let bin = rng.bytes(n);
        let hex_maxlen = rng.range(n * 2 + 1, n * 2 + 4096);
        let mut case = Case::slack(&bin, hex_maxlen);
        case.fill = rng.next_u8();
        assert_identical(&c, &r, &case, &format!("C9 i={i} n={n}"));
    }
}

/// Helper for the four nibble-class rows: builds bytes whose high nibble is
/// drawn from `hi` and whose low nibble is drawn from `lo`.
fn nibble_class_row(
    label: &str,
    seed: u64,
    hi: &[u8],
    lo: &[u8],
    check: fn(&[u8]) -> bool,
) {
    let (c, r) = both();
    let mut rng = Rng::new(seed);
    for i in 0..400 {
        let n = rng.range(1, 512);
        let bin: Vec<u8> = (0..n)
            .map(|_| {
                let h = hi[(rng.next_u64() % hi.len() as u64) as usize];
                let l = lo[(rng.next_u64() % lo.len() as u64) as usize];
                (h << 4) | l
            })
            .collect();
        // Sanity-check the generator against the intended digit classes using
        // the C output itself (the ground truth).
        let case = if i % 2 == 0 {
            Case::exact(&bin)
        } else {
            Case::slack(&bin, n * 2 + 1 + rng.range(1, 64))
        };
        assert_identical(&c, &r, &case, &format!("{label} i={i} n={n}"));

        if i == 0 {
            // Verify the row really exercises the intended digit classes.
            let mut out = vec![0u8; n * 2 + 1];
            unsafe {
                (c.bin2hex)(
                    out.as_mut_ptr().cast(),
                    n * 2 + 1,
                    bin.as_ptr(),
                    n,
                );
            }
            assert!(
                check(&out[..n * 2]),
                "{label}: generator does not produce the intended digit classes: {:?}",
                String::from_utf8_lossy(&out[..n * 2])
            );
        }
    }
}

const LOW: [u8; 10] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
const HIGH: [u8; 6] = [10, 11, 12, 13, 14, 15];

/// C10 — both nibbles `< 10`: every output character is a decimal digit.
#[test]
fn c10_both_nibbles_low() {
    nibble_class_row("C10", 0xC000_0010, &LOW, &LOW, |o| {
        o.iter().all(|&ch| ch.is_ascii_digit())
    });
}

/// C11 — high nibble `>= 10`, low nibble `< 10`: letter then digit.
#[test]
fn c11_hi_letter_lo_digit() {
    nibble_class_row("C11", 0xC000_0011, &HIGH, &LOW, |o| {
        o.chunks(2)
            .all(|p| (b'a'..=b'f').contains(&p[0]) && p[1].is_ascii_digit())
    });
}

/// C12 — high nibble `< 10`, low nibble `>= 10`: digit then letter.
#[test]
fn c12_hi_digit_lo_letter() {
    nibble_class_row("C12", 0xC000_0012, &LOW, &HIGH, |o| {
        o.chunks(2)
            .all(|p| p[0].is_ascii_digit() && (b'a'..=b'f').contains(&p[1]))
    });
}

/// C13 — both nibbles `>= 10`: every output character is `a`..`f`.
#[test]
fn c13_both_nibbles_high() {
    nibble_class_row("C13", 0xC000_0013, &HIGH, &HIGH, |o| {
        o.iter().all(|ch| (b'a'..=b'f').contains(ch))
    });
}

/// C14 — nibble-class transition points only.
#[test]
fn c14_boundary_bytes() {
    const POOL: [u8; 18] = [
        0x00, 0x01, 0x09, 0x0A, 0x0F, 0x10, 0x90, 0x99, 0x9A, 0x9F, 0xA0, 0xA9, 0xAA, 0xAF, 0xF0,
        0xF9, 0xFA, 0xFF,
    ];
    let (c, r) = both();
    let mut rng = Rng::new(0xC000_0014);
    for i in 0..1000 {
        let n = rng.range(1, 64);
        let bin = rng.bytes_from(n, &POOL);
        let case = if i % 3 == 0 {
            Case::exact(&bin)
        } else {
            Case::slack(&bin, n * 2 + 1 + rng.range(1, 32))
        };
        assert_identical(&c, &r, &case, &format!("C14 i={i} n={n}"));
    }
    // Also every boundary byte on its own.
    for &b in &POOL {
        let bin = [b];
        assert_identical(&c, &r, &Case::exact(&bin), &format!("C14 solo {b:#04x}"));
    }
}

/// C15 — `bin_len` at 255 / 256 / 257, exact and slack.
#[test]
fn c15_byte_count_boundaries() {
    let (c, r) = both();
    let mut rng = Rng::new(0xC000_0015);
    for n in [254usize, 255, 256, 257, 258, 511, 512, 513] {
        for i in 0..20 {
            let bin = rng.bytes(n);
            let case = if i % 2 == 0 {
                Case::exact(&bin)
            } else {
                Case::slack(&bin, n * 2 + 1 + rng.range(1, 256))
            };
            assert_identical(&c, &r, &case, &format!("C15 n={n} i={i}"));
        }
    }
}

/// C16 — `hex_maxlen = usize::MAX` (maximum slack). Only `bin_len * 2 + 1`
/// bytes are ever written, so a modest allocation is enough.
#[test]
fn c16_hex_maxlen_usize_max() {
    let (c, r) = both();
    let mut rng = Rng::new(0xC000_0016);
    for i in 0..500 {
        let n = rng.range(0, 64);
        let bin = rng.bytes(n);
        let case = Case {
            bin: Some(&bin),
            bin_len: n,
            hex_maxlen: usize::MAX,
            hex_alloc: n * 2 + 1,
            hex_off: i % 8,
            bin_off: i % 5,
            fill: rng.next_u8(),
        };
        assert_identical(&c, &r, &case, &format!("C16 i={i} n={n}"));
    }
    // And a few other huge-but-valid hex_maxlen values.
    for &hm in &[
        usize::MAX - 1,
        usize::MAX / 2,
        1usize << 60,
        0x7FFF_FFFF_FFFF_FFFF,
    ] {
        let bin = rng.bytes(8);
        let case = Case {
            bin: Some(&bin),
            bin_len: 8,
            hex_maxlen: hm,
            hex_alloc: 17,
            hex_off: 0,
            bin_off: 0,
            fill: 0xAA,
        };
        assert_identical(&c, &r, &case, &format!("C16 hex_maxlen={hm}"));
    }
}

/// C17 — misaligned `hex` and `bin` pointers.
#[test]
fn c17_misaligned_buffers() {
    let (c, r) = both();
    let mut rng = Rng::new(0xC000_0017);
    for hex_off in [0usize, 1, 2, 3, 5, 7, 9, 15] {
        for bin_off in [0usize, 1, 2, 3, 5, 7, 9, 15] {
            for _ in 0..12 {
                let n = rng.range(0, 96);
                let bin = rng.bytes(n);
                let extra = rng.range(0, 32);
                let case = Case {
                    bin: Some(&bin),
                    bin_len: n,
                    hex_maxlen: n * 2 + 1 + extra,
                    hex_alloc: n * 2 + 1 + extra,
                    hex_off,
                    bin_off,
                    fill: rng.next_u8(),
                };
                assert_identical(
                    &c,
                    &r,
                    &case,
                    &format!("C17 hex_off={hex_off} bin_off={bin_off} n={n}"),
                );
            }
        }
    }
}

/// C18 — repeated calls reusing one output buffer; the NUL lands at
/// `hex[bin_len * 2]` and later stale bytes must be left alone, identically.
#[test]
fn c18_buffer_reuse_sequence() {
    let (c, r) = both();
    let mut rng = Rng::new(0xC000_0018);
    for trial in 0..200 {
        let cap = 4096usize;
        let mut hex_c = vec![0xAAu8; cap + common::GUARD];
        let mut hex_r = vec![0xAAu8; cap + common::GUARD];
        // A descending-then-ascending sequence of lengths, so stale suffixes
        // from earlier calls remain visible.
        let mut lens: Vec<usize> = (0..8).map(|_| rng.range(0, 1000)).collect();
        lens.sort_unstable_by(|a, b| b.cmp(a));
        lens.push(rng.range(0, 1000));
        for (step, &n) in lens.iter().enumerate() {
            let bin = rng.bytes(n);
            let ret_c = unsafe {
                (c.bin2hex)(hex_c.as_mut_ptr().cast(), cap, bin.as_ptr(), n)
            };
            let ret_r = unsafe {
                (r.bin2hex)(hex_r.as_mut_ptr().cast(), cap, bin.as_ptr(), n)
            };
            assert_eq!(ret_c.cast::<u8>(), hex_c.as_mut_ptr(), "C18 C return ptr");
            assert_eq!(ret_r.cast::<u8>(), hex_r.as_mut_ptr(), "C18 Rust return ptr");
            assert_eq!(
                hex_c, hex_r,
                "C18 trial={trial} step={step} n={n}: buffers diverged"
            );
        }
    }
}

/// C19 — returned pointer identity, including misaligned and NULL-`bin` cases.
/// (`assert_identical` already checks this for every row; this row makes it
/// explicit and adds the extremes.)
#[test]
fn c19_return_pointer_identity() {
    let (c, r) = both();
    let mut rng = Rng::new(0xC000_0019);
    for i in 0..300 {
        let n = rng.range(0, 128);
        let bin = rng.bytes(n);
        let alloc = n * 2 + 1;
        let off = i % 16;
        let mut buf_c = vec![0u8; off + alloc + common::GUARD];
        let mut buf_r = vec![0u8; off + alloc + common::GUARD];
        let pc = unsafe { buf_c.as_mut_ptr().add(off) };
        let pr = unsafe { buf_r.as_mut_ptr().add(off) };
        let rc = unsafe { (c.bin2hex)(pc.cast(), alloc, bin.as_ptr(), n) };
        let rr = unsafe { (r.bin2hex)(pr.cast(), alloc, bin.as_ptr(), n) };
        assert_eq!(rc.cast::<u8>(), pc, "C19 i={i}: C return != hex arg");
        assert_eq!(rr.cast::<u8>(), pr, "C19 i={i}: Rust return != hex arg");
        assert!(!rr.is_null(), "C19 i={i}: Rust returned NULL");
        assert_eq!(buf_c, buf_r, "C19 i={i}: buffers diverged");
    }
    // NULL bin, zero length.
    let mut buf_c = [0xAAu8; 8];
    let mut buf_r = [0xAAu8; 8];
    let rc = unsafe { (c.bin2hex)(buf_c.as_mut_ptr().cast(), 8, std::ptr::null(), 0) };
    let rr = unsafe { (r.bin2hex)(buf_r.as_mut_ptr().cast(), 8, std::ptr::null(), 0) };
    assert_eq!(rc.cast::<u8>(), buf_c.as_mut_ptr());
    assert_eq!(rr.cast::<u8>(), buf_r.as_mut_ptr());
    assert_eq!(buf_c, buf_r);
}

/// C20 — the accept side of the `hex_maxlen <= bin_len * 2` guard, one step in:
/// `hex_maxlen == bin_len * 2 + 1` and `+ 2`.
#[test]
fn c20_accept_side_off_by_one() {
    let (c, r) = both();
    let mut rng = Rng::new(0xC000_0020);
    for n in 0usize..=64 {
        for extra in 1usize..=2 {
            let bin = rng.bytes(n);
            let case = Case {
                bin: Some(&bin),
                bin_len: n,
                hex_maxlen: n * 2 + extra,
                hex_alloc: n * 2 + extra,
                hex_off: 0,
                bin_off: 0,
                fill: 0xAA,
            };
            assert_identical(&c, &r, &case, &format!("C20 n={n} extra={extra}"));
        }
    }
}
