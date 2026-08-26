//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`.  Both implementations are reached only
//! through their shared objects (`libloading`), so the `#[no_mangle]` export
//! wrapper is exercised as well.  Randomized rows use a fixed-seed SplitMix64
//! PRNG, so a failure is always reproducible.

mod common;

use common::*;

/// C1 — empty input, exact-minimum destination (`hex_maxlen == 1`), sentinel
/// buffer: only `hex[0] = 0` may be written.
#[test]
fn c1_empty_exact_min() {
    for fill in [0x00u8, 0xAA, 0xFF, b'Z'] {
        // buffer bigger than hex_maxlen so an over-write would be visible
        diff_call("C1", &[], 0, 0, 1, 8, 0, fill);
    }
    diff_exact("C1/exact", &[]);
}

/// C2 — empty input with `bin == NULL` (legal: never dereferenced) and a
/// generous destination.
#[test]
fn c2_empty_null_bin() {
    let f = impls();
    for hex_maxlen in [1usize, 2, 3, 16, SIZE_MAX] {
        let mut c_buf = [0xAAu8; 16];
        let mut r_buf = [0xAAu8; 16];
        unsafe {
            let c_ret = (f.c)(c_buf.as_mut_ptr().cast(), hex_maxlen, std::ptr::null(), 0);
            let r_ret = (f.r)(r_buf.as_mut_ptr().cast(), hex_maxlen, std::ptr::null(), 0);
            assert_eq!(c_ret as *const u8, c_buf.as_ptr());
            assert_eq!(r_ret as *const u8, r_buf.as_ptr());
        }
        assert_eq!(c_buf, r_buf, "C2 mismatch for hex_maxlen={hex_maxlen}");
    }
}

/// C3 — `bin_len == 1`, exact-minimum buffer, **exhaustive** over all 256 byte
/// values (covers the full A3×A4 nibble-branch cross product).
#[test]
fn c3_single_byte_exhaustive() {
    for b in 0u8..=255 {
        diff_exact(&format!("C3/{b:#04x}"), &[b]);
    }
}

/// C4 — `bin_len == 1` with slack + sentinel fill, randomized bytes.
#[test]
fn c4_single_byte_slack() {
    let mut rng = Rng::new(0x0405_0607);
    for i in 0..512 {
        let b = rng.next_u8();
        let slack = rng.range(1, 16);
        diff_slack(&format!("C4/{i}"), &[b], slack, 0xAA);
    }
}

/// C5 — `bin_len == 2`, exact-minimum buffer, **exhaustive** over all 65 536
/// two-byte inputs.
#[test]
fn c5_two_bytes_exhaustive() {
    for hi in 0u8..=255 {
        for lo in 0u8..=255 {
            diff_exact("C5", &[hi, lo]);
        }
    }
}

/// C6 — odd length (3), `hex_maxlen == 2*bin_len + 2` (one spare byte),
/// sentinel-filled buffer.
#[test]
fn c6_odd_len_min_plus_one() {
    let mut rng = Rng::new(0x0601);
    for i in 0..512 {
        let mut bin = [0u8; 3];
        rng.fill(&mut bin);
        diff_slack(&format!("C6/{i}"), &bin, 1, 0x5A);
    }
}

fn nibble_class_test(label: &str, seed: u64, hi_lo: (bool, bool)) {
    let mut rng = Rng::new(seed);
    let (hi_letter, lo_letter) = hi_lo;
    for i in 0..256 {
        let len = rng.range(1, 32);
        let mut bin = vec![0u8; len];
        for b in bin.iter_mut() {
            let h = if hi_letter { rng.range(10, 15) } else { rng.below(10) } as u8;
            let l = if lo_letter { rng.range(10, 15) } else { rng.below(10) } as u8;
            *b = (h << 4) | l;
        }
        diff_exact(&format!("{label}/{i}"), &bin);
        diff_slack(&format!("{label}/{i}+slack"), &bin, 3, 0xAA);
    }
}

/// C7 — high nibble < 10 and low nibble < 10 (both digits).
#[test]
fn c7_nibbles_digit_digit() {
    nibble_class_test("C7", 0x0701, (false, false));
}

/// C8 — high nibble < 10, low nibble >= 10 (digit + letter).
#[test]
fn c8_nibbles_digit_letter() {
    nibble_class_test("C8", 0x0801, (false, true));
}

/// C9 — high nibble >= 10, low nibble < 10 (letter + digit).
#[test]
fn c9_nibbles_letter_digit() {
    nibble_class_test("C9", 0x0901, (true, false));
}

/// C10 — high nibble >= 10 and low nibble >= 10 (both letters).
#[test]
fn c10_nibbles_letter_letter() {
    nibble_class_test("C10", 0x0a01, (true, true));
}

const BOUNDARY_BYTES: [u8; 16] = [
    0x00, 0x09, 0x0A, 0x0F, 0x90, 0x99, 0x9A, 0x9F, 0xA0, 0xA9, 0xAA, 0xAF, 0xF0, 0xF9, 0xFA, 0xFF,
];

/// C11 — nibble-branch boundary values, as one buffer and individually.
#[test]
fn c11_boundary_bytes() {
    diff_exact("C11/all", &BOUNDARY_BYTES);
    diff_slack("C11/all+slack", &BOUNDARY_BYTES, 7, 0xAA);
    for b in BOUNDARY_BYTES {
        diff_exact(&format!("C11/{b:#04x}"), &[b]);
        diff_slack(&format!("C11/{b:#04x}+slack"), &[b], 2, 0xFF);
    }
    // every boundary byte repeated, at several lengths
    for b in BOUNDARY_BYTES {
        for len in [1usize, 2, 3, 8, 17] {
            diff_exact(&format!("C11/{b:#04x}x{len}"), &vec![b; len]);
        }
    }
}

/// C12 — randomized lengths `1..=64`, exact-minimum destination, 2 000 trials.
#[test]
fn c12_random_exact_min() {
    let mut rng = Rng::new(0x1200_0001);
    for i in 0..2000 {
        let len = rng.range(1, 64);
        let mut bin = vec![0u8; len];
        rng.fill(&mut bin);
        diff_exact(&format!("C12/{i}"), &bin);
    }
}

/// C13 — randomized lengths `1..=64` with random slack and sentinel fill,
/// 2 000 trials (detects any write past `hex[2*bin_len]`).
#[test]
fn c13_random_with_slack() {
    let mut rng = Rng::new(0x1300_0001);
    for i in 0..2000 {
        let len = rng.range(1, 64);
        let mut bin = vec![0u8; len];
        rng.fill(&mut bin);
        let slack = rng.range(1, 64);
        let fill = [0x00u8, 0xAA, 0xFF, 0x5A][rng.below(4)];
        // hex_maxlen is min+slack but the buffer is even larger, so a write past
        // hex_maxlen would also be caught.
        let need = len * 2 + 1;
        diff_call(
            &format!("C13/{i}"),
            &bin,
            0,
            len,
            need + slack,
            need + slack + 8,
            0,
            fill,
        );
    }
}

/// C14 — lengths around the 255/256 byte boundary.
#[test]
fn c14_lengths_around_256() {
    let mut rng = Rng::new(0x1400_0001);
    for len in [254usize, 255, 256, 257, 258] {
        for i in 0..32 {
            let mut bin = vec![0u8; len];
            rng.fill(&mut bin);
            diff_exact(&format!("C14/{len}/{i}"), &bin);
            diff_slack(&format!("C14/{len}/{i}+slack"), &bin, 5, 0xAA);
        }
    }
}

/// C15 — large inputs.
#[test]
fn c15_large_inputs() {
    let mut rng = Rng::new(0x1500_0001);
    for len in [1024usize, 4096, 65536] {
        for i in 0..4 {
            let mut bin = vec![0u8; len];
            rng.fill(&mut bin);
            diff_exact(&format!("C15/{len}/{i}"), &bin);
        }
    }
}

/// C16 — `hex_maxlen == SIZE_MAX` (oversized, but accepted) with small inputs.
#[test]
fn c16_hex_maxlen_size_max() {
    let mut rng = Rng::new(0x1600_0001);
    for i in 0..256 {
        let len = rng.range(0, 32);
        let mut bin = vec![0u8; len];
        rng.fill(&mut bin);
        let buf_total = len * 2 + 1 + 8;
        // hex_maxlen deliberately absurd; the C code only compares it.
        diff_call(&format!("C16/{i}"), &bin, 0, len, SIZE_MAX, buf_total, 0, 0xAA);
        diff_call(
            &format!("C16b/{i}"),
            &bin,
            0,
            len,
            SIZE_MAX - 1,
            buf_total,
            0,
            0x5A,
        );
    }
}

/// C17 — `bin` at unaligned offsets `1..=8` inside its allocation.
#[test]
fn c17_unaligned_source() {
    let mut rng = Rng::new(0x1700_0001);
    for off in 1..=8usize {
        for i in 0..64 {
            let len = rng.range(1, 40);
            let mut src = vec![0u8; off + len];
            rng.fill(&mut src);
            let need = len * 2 + 1;
            diff_call(&format!("C17/{off}/{i}"), &src, off, len, need, need, 0, 0x00);
            diff_call(
                &format!("C17/{off}/{i}+slack"),
                &src,
                off,
                len,
                need + 4,
                need + 4,
                0,
                0xAA,
            );
        }
    }
}

/// C18 — `hex` at offsets `1..=8` inside a larger sentinel buffer; the bytes
/// before `hex` and after `hex[2*bin_len]` must stay untouched (`diff_call`
/// compares the whole buffer, so any stray write diverges from C).
#[test]
fn c18_offset_destination() {
    let mut rng = Rng::new(0x1800_0001);
    for off in 1..=8usize {
        for i in 0..64 {
            let len = rng.range(0, 40);
            let mut bin = vec![0u8; len];
            rng.fill(&mut bin);
            let need = len * 2 + 1;
            let buf_total = off + need + 8;
            diff_call(
                &format!("C18/{off}/{i}"),
                &bin,
                0,
                len,
                need,
                buf_total,
                off,
                0xAA,
            );
            diff_call(
                &format!("C18/{off}/{i}+slack"),
                &bin,
                0,
                len,
                need + 3,
                buf_total,
                off,
                0x5A,
            );
        }
    }
}

/// C19 — return-pointer identity is asserted inside `diff_call` for every row;
/// this test pins it down explicitly for a few shapes.
#[test]
fn c19_return_pointer_identity() {
    let f = impls();
    let mut rng = Rng::new(0x1900_0001);
    for _ in 0..64 {
        let len = rng.range(0, 16);
        let mut bin = vec![0u8; len];
        rng.fill(&mut bin);
        for off in [0usize, 1, 3, 8] {
            let mut c_buf = vec![0xAAu8; off + len * 2 + 1];
            let mut r_buf = vec![0xAAu8; off + len * 2 + 1];
            unsafe {
                let c_hex = c_buf.as_mut_ptr().add(off);
                let r_hex = r_buf.as_mut_ptr().add(off);
                let c_ret = (f.c)(c_hex.cast(), len * 2 + 1, bin.as_ptr(), len);
                let r_ret = (f.r)(r_hex.cast(), len * 2 + 1, bin.as_ptr(), len);
                assert_eq!(c_ret as *mut u8, c_hex, "C19: C return != hex");
                assert_eq!(r_ret as *mut u8, r_hex, "C19: Rust return != hex");
            }
            assert_eq!(c_buf, r_buf, "C19 output mismatch");
        }
    }
}

/// C20 — statelessness: many repeated calls on the same buffer, and
/// C→Rust→C interleaving into one shared buffer.
#[test]
fn c20_statelessness_and_interleaving() {
    let f = impls();
    let mut rng = Rng::new(0x2000_0001);

    // repeated calls into the same buffer must keep producing the same bytes
    for _ in 0..64 {
        let len = rng.range(0, 24);
        let mut bin = vec![0u8; len];
        rng.fill(&mut bin);
        let need = len * 2 + 1;
        let mut c_buf = vec![0xAAu8; need + 4];
        let mut r_buf = vec![0xAAu8; need + 4];
        let mut first: Option<Vec<u8>> = None;
        for _ in 0..100 {
            unsafe {
                (f.c)(c_buf.as_mut_ptr().cast(), need, bin.as_ptr(), len);
                (f.r)(r_buf.as_mut_ptr().cast(), need, bin.as_ptr(), len);
            }
            assert_eq!(c_buf, r_buf, "C20 repeated-call mismatch");
            match &first {
                None => first = Some(c_buf.clone()),
                Some(v) => assert_eq!(v, &c_buf, "C20: C is not stateless"),
            }
        }
    }

    // interleaved into ONE shared buffer: Rust must leave exactly what C left
    for _ in 0..256 {
        let len = rng.range(0, 24);
        let mut bin = vec![0u8; len];
        rng.fill(&mut bin);
        let need = len * 2 + 1;
        let mut buf = vec![0xAAu8; need + 4];
        unsafe {
            (f.c)(buf.as_mut_ptr().cast(), need, bin.as_ptr(), len);
            let after_c = buf.clone();
            // scramble, then let Rust rewrite it
            rng.fill(&mut buf);
            (f.r)(buf.as_mut_ptr().cast(), need, bin.as_ptr(), len);
            assert_eq!(
                &after_c[..need],
                &buf[..need],
                "C20 interleaved mismatch (len={len})"
            );
            // and back to C over Rust's output
            (f.c)(buf.as_mut_ptr().cast(), need, bin.as_ptr(), len);
            assert_eq!(&after_c[..need], &buf[..need], "C20 C-over-Rust mismatch");
        }
    }
}

/// C21 — value extremes: all-zero and all-`0xFF` inputs.
#[test]
fn c21_value_extremes() {
    for len in [1usize, 2, 7, 8, 63, 64] {
        diff_exact(&format!("C21/00x{len}"), &vec![0x00u8; len]);
        diff_exact(&format!("C21/FFx{len}"), &vec![0xFFu8; len]);
        diff_slack(&format!("C21/00x{len}+slack"), &vec![0x00u8; len], 4, 0xAA);
        diff_slack(&format!("C21/FFx{len}+slack"), &vec![0xFFu8; len], 4, 0x00);
    }
}

/// C22 — aliasing: `hex` and `bin` inside the same allocation.  The C code reads
/// `bin[i]` *after* previous iterations may have overwritten it, so the exact
/// per-iteration read/write order is observable here.
#[test]
fn c22_aliased_buffers() {
    let f = impls();
    let mut rng = Rng::new(0x2200_0001);

    // relative placements of hex w.r.t. bin (signed offset in bytes)
    let placements: [isize; 7] = [0, 1, 2, -1, -2, -7, 8];

    for trial in 0..512 {
        let len = rng.range(1, 24);
        let need = len * 2 + 1;
        let pad = 32;
        // one allocation holding both bin and hex regions
        let total = pad + len + need + pad;
        let mut base = vec![0u8; total];
        rng.fill(&mut base);
        let bin_at = pad;

        for &rel in placements.iter() {
            let hex_at = (bin_at as isize + rel) as usize;
            if hex_at + need > total {
                continue;
            }
            let mut c_buf = base.clone();
            let mut r_buf = base.clone();
            unsafe {
                let cp = c_buf.as_mut_ptr();
                let rp = r_buf.as_mut_ptr();
                let c_ret = (f.c)(cp.add(hex_at).cast(), need, cp.add(bin_at), len);
                let r_ret = (f.r)(rp.add(hex_at).cast(), need, rp.add(bin_at), len);
                assert_eq!(c_ret as *const u8, cp.add(hex_at) as *const u8);
                assert_eq!(r_ret as *const u8, rp.add(hex_at) as *const u8);
            }
            assert_eq!(
                c_buf, r_buf,
                "C22 aliased mismatch (trial={trial} len={len} rel={rel})"
            );
        }
    }
}

/// C23 — concurrent use: the C function has no state, so 8 threads hammering
/// both implementations on private buffers must agree with each other.
#[test]
fn c23_concurrent_use() {
    let mut handles = Vec::new();
    for t in 0..8u64 {
        handles.push(std::thread::spawn(move || {
            let mut rng = Rng::new(0x2300_0000 + t);
            for i in 0..500 {
                let len = rng.range(0, 48);
                let mut bin = vec![0u8; len];
                rng.fill(&mut bin);
                if i % 3 == 0 {
                    diff_exact(&format!("C23/{t}/{i}"), &bin);
                } else {
                    diff_slack(&format!("C23/{t}/{i}"), &bin, rng.range(1, 8), 0xAA);
                }
            }
        }));
    }
    for h in handles {
        h.join().expect("worker thread panicked");
    }
}
