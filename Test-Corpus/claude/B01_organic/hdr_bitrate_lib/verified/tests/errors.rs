//! Phase C — error-path differential tests, one test per `ERRORS.md` row.
//!
//! `hdr_bitrate` has NO explicit error surface (no `if`, no `assert`, no error
//! code, no sentinel — see `ERRORS.md`), so each row asserts that C and Rust
//! agree on the *same* concrete value for the invalid input, rather than merely
//! "both failed somehow".

mod harness;

use harness::{flat_offset, make_header, Libs, Rng, ITERS};

// E1 — bitrate index 15 (invalid) where flat lands on the next row's leading 0.
#[test]
fn err_e1_bitrate_index_15_neighbour_row() {
    let l = Libs::load();
    let mut rng = Rng::new(0x_E1);
    let mut n = 0;

    for i in 0..=1u32 {
        for layer in 0..=3u32 {
            let flat = flat_offset(i as i32, layer as i32, 15);
            // Exclude E2 (flat == 90, past the whole table) and E5 (flat == 0).
            if flat == 90 || flat == 0 {
                continue;
            }
            assert!(
                (0..90).contains(&flat),
                "E1 expects an in-table flat offset, got {flat}"
            );
            for _ in 0..ITERS {
                let h = make_header(i, layer, 15, &mut rng);
                let v = l.assert_eq_on(&h, &format!("E1 i={i} layer={layer} k=15 flat={flat}"));
                assert_eq!(
                    v, 0,
                    "E1: every table row begins with 0, so k=15 must yield 0 \
                     (i={i} layer={layer} flat={flat})"
                );
                n += 1;
            }
        }
    }
    assert!(n > 0, "E1 covered no cases");
    eprintln!("[E1] {n} calls matched, all returned 0");
}

// E2 — i=1, j=2, k=15 -> flat == 90: one byte PAST the end of the table.
// Genuine UB in the C; the Rust must reproduce the observed value.
#[test]
fn err_e2_bitrate_index_15_past_end_of_table() {
    let l = Libs::load();
    let mut rng = Rng::new(0x_E2);
    assert_eq!(flat_offset(1, 3, 15), 90, "E2 must be the flat==90 case");

    for _ in 0..ITERS {
        let h = make_header(1, 3, 15, &mut rng);
        l.assert_eq_on(&h, "E2 i=1 layer=3 k=15 flat=90 (past end of table)");
    }

    // Also sweep every h[1] in this family (h[1] & 0x0E == 0x0E) and every
    // low nibble of h[2]: all 512 (h1,h2) pairs that reach flat == 90.
    let mut n = 0;
    for h1 in 0..=255u8 {
        if h1 & 0x0E != 0x0E {
            continue;
        }
        for lo in 0..=15u8 {
            let h = [0xAA, h1, 0xF0 | lo, 0x55];
            l.assert_eq_on(&h, "E2 exhaustive flat=90 family");
            n += 1;
        }
    }
    assert_eq!(n, 512, "E2 family should be 32 h1 values * 16 low nibbles");
    eprintln!("[E2] all {n} inputs reaching flat==90 matched");
}

// E3 — reserved layer (j == -1) with i == 0: flat = k-15, i.e. -15..=-1,
// reading BEFORE the start of the table. Genuine UB in the C.
#[test]
fn err_e3_reserved_layer_reads_before_table() {
    let l = Libs::load();
    let mut rng = Rng::new(0x_E3);

    for k in 0..=14u32 {
        let flat = flat_offset(0, 0, k as i32);
        assert!(flat < 0, "E3 expects a negative flat offset, got {flat} for k={k}");
        for _ in 0..ITERS {
            let h = make_header(0, 0, k, &mut rng);
            l.assert_eq_on(&h, &format!("E3 i=0 layer=0 k={k} flat={flat} (before table)"));
        }
    }

    // Exhaustive over the whole family: h[1] & 0x0E == 0, k = 0..14.
    let mut n = 0;
    for h1 in 0..=255u8 {
        if h1 & 0x0E != 0x00 {
            continue;
        }
        for h2 in 0..=255u8 {
            if h2 >> 4 == 15 {
                continue; // that is E5, flat == 0
            }
            let h = [0x5A, h1, h2, 0xC3];
            l.assert_eq_on(&h, "E3 exhaustive negative-flat family");
            n += 1;
        }
    }
    assert_eq!(n, 32 * 240, "E3 family size");
    eprintln!("[E3] all {n} inputs reading before the table matched");
}

// E4 — reserved layer (j == -1) with i == 1: flat = 30 + k, still IN the table.
#[test]
fn err_e4_reserved_layer_aliases_row_0_2() {
    let l = Libs::load();
    let mut rng = Rng::new(0x_E4);

    // The expected values are 2 * halfrate[0][2][k].
    const ROW_0_2: [u32; 15] = [0, 16, 24, 28, 32, 40, 48, 56, 64, 72, 80, 88, 96, 112, 128];

    for k in 0..=14u32 {
        let flat = flat_offset(1, 0, k as i32);
        assert_eq!(flat, 30 + k as i32);
        for _ in 0..ITERS {
            let h = make_header(1, 0, k, &mut rng);
            let v = l.assert_eq_on(&h, &format!("E4 i=1 layer=0 k={k} flat={flat}"));
            assert_eq!(
                v,
                2 * ROW_0_2[k as usize],
                "E4: reserved layer with i=1 must alias halfrate[0][2][{k}]"
            );
        }
    }
    eprintln!("[E4] reserved layer with i=1 aliases row halfrate[0][2] in both");
}

// E5 — i=0, j=-1, k=15 -> flat == 0 exactly.
#[test]
fn err_e5_reserved_layer_k15_aliases_first_entry() {
    let l = Libs::load();
    let mut rng = Rng::new(0x_E5);
    assert_eq!(flat_offset(0, 0, 15), 0);

    for _ in 0..ITERS {
        let h = make_header(0, 0, 15, &mut rng);
        let v = l.assert_eq_on(&h, "E5 i=0 layer=0 k=15 flat=0");
        assert_eq!(v, 0, "E5: halfrate[0][0][0] == 0");
    }
}

// E6 — bitrate index 0 ("free"): a valid index whose entry is 0 in every row.
#[test]
fn err_e6_free_bitrate_index_zero() {
    let l = Libs::load();
    let mut rng = Rng::new(0x_E6);

    for i in 0..=1u32 {
        for layer in 0..=3u32 {
            for _ in 0..ITERS {
                let h = make_header(i, layer, 0, &mut rng);
                let v = l.assert_eq_on(&h, &format!("E6 i={i} layer={layer} k=0"));
                // For layer != 0, or (layer == 0 && i == 1), flat is in-table
                // and the entry is 0. For layer == 0 && i == 0 flat is -15
                // (E3), which also observes 0 in this build.
                assert_eq!(v, 0, "E6: bitrate index 0 must give 0 (i={i} layer={layer})");
            }
        }
    }
}

// E7 — NULL pointer. The C has no null check and unconditionally dereferences
// h[1]/h[2]; so does the Rust. Both must fault the same way. Run each in a
// child process so the test harness survives.
#[test]
fn err_e7_null_pointer_faults_in_both() {
    use std::os::unix::process::ExitStatusExt;

    // Child mode: perform the null call and let the fault happen.
    if let Ok(which) = std::env::var("HDR_NULL_PROBE") {
        let l = Libs::load();
        let f = match which.as_str() {
            "c" => l.c,
            "rust" => l.rust,
            other => panic!("bad HDR_NULL_PROBE={other}"),
        };
        // Defeat any constant folding, then dereference NULL.
        let p: *const u8 = std::ptr::null();
        let v = unsafe { f(p) };
        // Should never be reached.
        println!("UNEXPECTEDLY RETURNED {v}");
        std::process::exit(0);
    }

    let exe = std::env::current_exe().unwrap();
    let mut results = Vec::new();

    for which in ["c", "rust"] {
        let out = std::process::Command::new(&exe)
            .args(["--exact", "err_e7_null_pointer_faults_in_both", "--nocapture"])
            .env("HDR_NULL_PROBE", which)
            .env("RUST_BACKTRACE", "0")
            .output()
            .expect("spawn child probe");

        let signal = out.status.signal();
        let code = out.status.code();
        eprintln!("[E7] {which}: signal={signal:?} code={code:?}");
        results.push((signal, code));
    }

    let (c_sig, c_code) = results[0];
    let (r_sig, r_code) = results[1];

    assert_eq!(
        c_sig, r_sig,
        "E7: C and Rust must fault with the SAME signal on a NULL pointer \
         (C: signal={c_sig:?} code={c_code:?}, Rust: signal={r_sig:?} code={r_code:?})"
    );
    assert_eq!(
        c_sig,
        Some(libc_sigsegv()),
        "E7: expected SIGSEGV from the C implementation, got signal={c_sig:?} code={c_code:?}"
    );
}

fn libc_sigsegv() -> i32 {
    11 // SIGSEGV on Linux
}

// E8 — no bounds check: a buffer of exactly the 3 bytes the function reads is
// accepted, and the result matches. There is no length parameter to reject.
#[test]
fn err_e8_short_buffer_no_bounds_check() {
    let l = Libs::load();
    let mut rng = Rng::new(0x_E8);

    for _ in 0..ITERS {
        let i = rng.below(2);
        let layer = rng.below(4);
        let k = rng.below(16);
        let full = make_header(i, layer, k, &mut rng);

        // Exactly 3 bytes — the minimum the C reads. No error is returned.
        let exact: Vec<u8> = full[..3].to_vec();
        assert_eq!(exact.len(), 3);
        let (c, r) = unsafe { l.both_raw(exact.as_ptr()) };
        assert_eq!(c, r, "E8: 3-byte buffer i={i} layer={layer} k={k}");

        // Same answer as the 4-byte buffer: the 4th byte is never read.
        let (c4, r4) = l.both(&full);
        assert_eq!((c, r), (c4, r4), "E8: length changed the result");
    }
    eprintln!("[E8] exactly-3-byte buffers accepted with no rejection, results identical");
}

// E9 — out-of-range "enum-like" field values crossing the FFI boundary: neither
// the layer field nor the bitrate index is validated, so all 128 index triples
// (including the 2 * 16 with the reserved layer and the 2 * 4 with index 15)
// are accepted and must agree.
#[test]
fn err_e9_all_128_index_triples() {
    let l = Libs::load();
    let mut rng = Rng::new(0x_E9);
    let mut covered = std::collections::BTreeSet::new();

    for i in 0..=1u32 {
        for layer in 0..=3u32 {
            for k in 0..=15u32 {
                for _ in 0..64 {
                    let h = make_header(i, layer, k, &mut rng);
                    l.assert_eq_on(&h, &format!("E9 i={i} layer={layer} k={k}"));
                }
                covered.insert((i, layer, k));
            }
        }
    }
    assert_eq!(covered.len(), 128, "E9 must cover all 128 index triples");

    // Explicitly: the reserved layer value (0) and the reserved bitrate index
    // (15) are one step past the documented valid ranges (layer 1..=3,
    // index 0..=14) and are NOT rejected by either implementation.
    for i in 0..=1u32 {
        let h = make_header(i, 0, 7, &mut rng); // layer field 0 = reserved
        let (c, r) = l.both(&h);
        assert_eq!(c, r, "E9: reserved layer field 0 must agree");

        let h = make_header(i, 2, 15, &mut rng); // bitrate index 15 = 'bad'
        let (c, r) = l.both(&h);
        assert_eq!(c, r, "E9: reserved bitrate index 15 must agree");
    }
    eprintln!("[E9] all 128 index triples accepted identically (no validation in C)");
}

// E10 — bytes the C never reads must not influence the result.
#[test]
fn err_e10_unread_bytes_do_not_matter() {
    let l = Libs::load();
    let mut rng = Rng::new(0x_E10);

    for i in 0..=1u32 {
        for layer in 0..=3u32 {
            for k in 0..=15u32 {
                let clean = [0u8, ((i as u8) << 3) | ((layer as u8) << 1), (k as u8) << 4, 0];
                let (bc, br) = l.both(&clean);
                assert_eq!(bc, br);

                // h[0] and h[3..] set to every extreme, plus random values.
                for &fill in &[0x00u8, 0xFF, 0xAA, 0x55] {
                    let dirty = [fill, clean[1], clean[2], fill];
                    let (c, r) = l.both(&dirty);
                    assert_eq!(c, r, "E10 fill={fill:#04x} i={i} layer={layer} k={k}");
                    assert_eq!(
                        c, bc,
                        "E10: unread bytes changed the result (fill={fill:#04x})"
                    );
                }

                for _ in 0..16 {
                    let h = make_header(i, layer, k, &mut rng);
                    let (c, r) = l.both(&h);
                    assert_eq!(c, r);
                    assert_eq!(c, bc, "E10: don't-care bits changed the result");
                }
            }
        }
    }
}

// Generic boundary sweep required by Phase C: zero and maximal byte values.
#[test]
fn err_generic_extreme_byte_values() {
    let l = Libs::load();
    let extremes = [0x00u8, 0x01, 0x07, 0x08, 0x0E, 0x0F, 0x7F, 0x80, 0xF0, 0xFE, 0xFF];

    for &h0 in &extremes {
        for &h1 in &extremes {
            for &h2 in &extremes {
                for &h3 in &extremes {
                    let h = [h0, h1, h2, h3];
                    l.assert_eq_on(&h, "generic extremes");
                }
            }
        }
    }

    // All-zero and all-ones headers specifically.
    l.assert_eq_on(&[0, 0, 0, 0], "all zero");
    l.assert_eq_on(&[0xFF, 0xFF, 0xFF, 0xFF], "all ones");
}
