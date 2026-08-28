//! Phase C -- error-path differential tests, gated on `ERRORS.md`.
//!
//! `hdr_bitrate` has no explicit error surface (no `if`, no `assert`, no
//! sentinel), so its "rejections" are the unguarded out-of-bounds table
//! accesses its two undefended bit-fields can produce, plus the generic FFI
//! boundary conditions. Each `ERRORS.md` row below asserts that C and Rust
//! produce the *same* result -- the same value, or the same fatal signal -- not
//! merely that both "failed somehow".

mod common;

use common::*;

/// Build the 3-byte header selecting a (plane, layer_bits, rate) triple.
fn hdr(plane: u32, layer_bits: u32, rate: u32, rng: &mut Rng) -> Vec<u8> {
    vec![
        rng.next_u8(),
        make_h1(plane, layer_bits, rng.next_u8()),
        make_h2(rate, rng.next_u8()),
    ]
}

/// Assert C and Rust agree *and* that the shared answer equals the value
/// `ERRORS.md` documents for this row (so a wrong analysis cannot hide behind
/// two implementations agreeing).
#[track_caller]
fn check(p: &Pair, plane: u32, layer_bits: u32, rate: u32, expect: u32, row: &str) {
    let mut rng = Rng::new(SEED ^ (plane as u64) << 40 ^ (layer_bits as u64) << 20 ^ rate as u64);
    for _ in 0..64 {
        let buf = hdr(plane, layer_bits, rate, &mut rng);
        let got = p.assert_same(&buf, row);
        assert_eq!(
            got, expect,
            "{row}: C+Rust agree on {got} but ERRORS.md documents {expect} \
             (plane={plane} layer_bits={layer_bits} rate={rate})"
        );
    }
}

// ===========================================================================
// Row 1 -- flat offset -15: read before the whole `halfrate` object.
// ===========================================================================

#[test]
fn errors_row_1_offset_minus_15() {
    let p = load_pair();
    // plane 0, layer bits 00 (reserved -> inner index -1), rate 0.
    check(&p, 0, 0, 0, 0, "ERRORS row 1 (flat offset -15, before object)");
}

// ===========================================================================
// Row 2 -- flat offsets -14 ..= -1, all before the object.
// ===========================================================================

#[test]
fn errors_row_2_offsets_minus_14_to_minus_1() {
    let p = load_pair();
    for rate in 1..15 {
        let off = -15 + rate as i32;
        check(
            &p,
            0,
            0,
            rate,
            0,
            &format!("ERRORS row 2 (flat offset {off}, before object)"),
        );
    }
}

// ===========================================================================
// Row 3 -- flat offset 90: one byte past the end of the object.
// ===========================================================================

#[test]
fn errors_row_3_offset_90_past_end() {
    let p = load_pair();
    // plane 1, layer bits 11 (inner index 2), rate 15 -> 45 + 30 + 15 = 90.
    check(&p, 1, 3, 15, 0, "ERRORS row 3 (flat offset 90, past end)");
}

// ===========================================================================
// Row 4 -- the -15/+15 cancellation lands back on halfrate[0][0][0].
// ===========================================================================

#[test]
fn errors_row_4_negative_index_cancels() {
    let p = load_pair();
    check(&p, 0, 0, 15, 0, "ERRORS row 4 (offset -15 + 15 = 0)");
}

// ===========================================================================
// Row 5 -- reserved layer with plane 1 silently reads the other plane's row.
// ===========================================================================

#[test]
fn errors_row_5_reserved_layer_reads_wrong_plane() {
    let p = load_pair();
    // offset = 45 - 15 + rate = 30 + rate  ->  halfrate[0][2][rate]
    let plane0_row2: [u32; 15] = [0, 16, 24, 28, 32, 40, 48, 56, 64, 72, 80, 88, 96, 112, 128];
    for rate in 0..15u32 {
        check(
            &p,
            1,
            0,
            rate,
            2 * plane0_row2[rate as usize],
            &format!("ERRORS row 5 (plane 1, reserved layer, rate {rate}, offset {})", 30 + rate),
        );
    }
}

// ===========================================================================
// Row 6 -- reserved layer, plane 1, bad rate -> offset 45 = halfrate[1][0][0].
// ===========================================================================

#[test]
fn errors_row_6_reserved_layer_bad_rate() {
    let p = load_pair();
    check(&p, 1, 0, 15, 0, "ERRORS row 6 (offset 45)");
}

// ===========================================================================
// Row 7 -- bad rate nibble (15) for plane 0, layers 01/10/11.
// ===========================================================================

#[test]
fn errors_row_7_bad_rate_plane0() {
    let p = load_pair();
    for layer_bits in 1..4u32 {
        let off = (layer_bits as i32 - 1) * 15 + 15;
        check(
            &p,
            0,
            layer_bits,
            15,
            0,
            &format!("ERRORS row 7 (plane 0, layer bits {layer_bits:02b}, bad rate, offset {off})"),
        );
    }
}

// ===========================================================================
// Row 8 -- bad rate nibble (15) for plane 1, layers 01/10.
// ===========================================================================

#[test]
fn errors_row_8_bad_rate_plane1() {
    let p = load_pair();
    for layer_bits in 1..3u32 {
        let off = 45 + (layer_bits as i32 - 1) * 15 + 15;
        check(
            &p,
            1,
            layer_bits,
            15,
            0,
            &format!("ERRORS row 8 (plane 1, layer bits {layer_bits:02b}, bad rate, offset {off})"),
        );
    }
}

// ===========================================================================
// Row 9 -- free-format rate nibble 0 for all 8 plane x layer combinations.
// ===========================================================================

#[test]
fn errors_row_9_free_format_rate_zero() {
    let p = load_pair();
    // Every row of the table starts with 0, and the reserved-layer cases land
    // either before the object (plane 0) or on halfrate[0][2][0] == 0.
    for plane in PLANES {
        for layer_bits in LAYER_BITS {
            check(
                &p,
                plane,
                layer_bits,
                0,
                0,
                &format!("ERRORS row 9 (free format, plane {plane}, layer bits {layer_bits:02b})"),
            );
        }
    }
}

// ===========================================================================
// Row 10 -- widest in-range result (448) must not be truncated by the ABI.
// ===========================================================================

#[test]
fn errors_row_10_max_value_448() {
    let p = load_pair();
    check(&p, 1, 3, 14, 448, "ERRORS row 10 (max value 448 > u8::MAX)");
}

// ===========================================================================
// Row 11 -- out-of-range "enum" values across the FFI boundary, exhaustively.
// ===========================================================================

#[test]
fn errors_row_11_out_of_range_field_values_exhaustive() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 0xE11);

    // Every bit pattern of the two decoded bytes is a reachable input, and two
    // of the field encodings (layer bits 00, rate nibble 15) have no valid
    // variant at all. Sweep the entire domain.
    let mut diffs = 0usize;
    let mut first: Option<(u8, u8, u32, u32)> = None;
    for h1 in 0u16..=255 {
        for h2 in 0u16..=255 {
            let buf = [rng.next_u8(), h1 as u8, h2 as u8];
            let (a, b) = unsafe { (p.c.call(buf.as_ptr()), p.rust.call(buf.as_ptr())) };
            if a != b {
                diffs += 1;
                first.get_or_insert((h1 as u8, h2 as u8, a, b));
            }
        }
    }
    assert_eq!(
        diffs, 0,
        "ERRORS row 11: {diffs} divergences over 65536 field encodings; first: {first:?}"
    );

    // And specifically the two no-valid-variant encodings, on their own.
    for plane in PLANES {
        for rate in RATE_NIBBLES {
            let buf = hdr(plane, 0, rate, &mut rng); // layer bits 00: no variant
            p.assert_same(&buf, "ERRORS row 11 (reserved layer encoding)");
        }
        for layer_bits in LAYER_BITS {
            let buf = hdr(plane, layer_bits, 15, &mut rng); // rate 15: no variant
            p.assert_same(&buf, "ERRORS row 11 (bad rate encoding)");
        }
    }
}

// ===========================================================================
// Row 12 -- bits the C never reads must never change the answer.
// ===========================================================================

#[test]
fn errors_row_12_ignored_bits_are_ignored() {
    let p = load_pair();

    for plane in PLANES {
        for layer_bits in LAYER_BITS {
            for rate in RATE_NIBBLES {
                let mut baseline: Option<u32> = None;
                for noise1 in 0u8..=255 {
                    for noise2 in [0u8, 1, 7, 8, 15] {
                        let h1 = ((plane as u8) << 3) | ((layer_bits as u8) << 1) | (noise1 & 0xF1);
                        let h2 = ((rate as u8) << 4) | noise2;
                        let buf = [noise1, h1, h2];
                        let got = p.assert_same(&buf, "ERRORS row 12 (ignored bits)");
                        match baseline {
                            None => baseline = Some(got),
                            Some(b) => assert_eq!(
                                b, got,
                                "ERRORS row 12: ignored bits changed the result \
                                 ({b} -> {got}) at h[1]={h1:#04x} h[2]={h2:#04x}"
                            ),
                        }
                    }
                }
            }
        }
    }
}

// ===========================================================================
// Row 13 -- nothing past h[2] is read (guard page).
// ===========================================================================

#[test]
fn errors_row_13_no_read_past_h2() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 13);

    let page = unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as usize;
    let total = page * 2;
    let base = unsafe {
        libc::mmap(
            std::ptr::null_mut(),
            total,
            libc::PROT_NONE,
            libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
            -1,
            0,
        )
    };
    assert_ne!(base, libc::MAP_FAILED, "mmap failed");
    // First page readable/writable, second page left PROT_NONE.
    let rc = unsafe { libc::mprotect(base, page, libc::PROT_READ | libc::PROT_WRITE) };
    assert_eq!(rc, 0, "mprotect failed");

    // h[2] is the last accessible byte.
    let h = unsafe { (base as *mut u8).add(page - 3) };
    for plane in PLANES {
        for layer_bits in LAYER_BITS {
            for rate in RATE_NIBBLES {
                unsafe {
                    *h = rng.next_u8();
                    *h.add(1) = make_h1(plane, layer_bits, rng.next_u8());
                    *h.add(2) = make_h2(rate, rng.next_u8());
                    p.assert_same_ptr(h, "ERRORS row 13 (h[2] last accessible byte)");
                }
            }
        }
    }

    unsafe { libc::munmap(base, total) };
}

// ===========================================================================
// Row 14 -- NULL pointer: both must die with the same fatal signal.
// ===========================================================================

/// Helper test: only does anything when `HDR_NULL_TARGET` is set, in which case
/// it deliberately calls the selected implementation with a NULL pointer and is
/// expected to be killed by a signal. Invoked as a child process by
/// `errors_row_14_null_pointer_same_signal`.
#[test]
fn null_pointer_child() {
    let target = match std::env::var("HDR_NULL_TARGET") {
        Ok(v) => v,
        Err(_) => return, // normal test run: no-op
    };
    let p = load_pair();
    let imp = match target.as_str() {
        "c" => &p.c,
        "rust" => &p.rust,
        other => panic!("unknown HDR_NULL_TARGET {other}"),
    };
    eprintln!("calling {} hdr_bitrate(NULL)", imp.name);
    let v = unsafe { imp.call(std::ptr::null()) };
    // Reaching here means no fault occurred; report the value so the parent can
    // compare it against the other implementation instead of a signal.
    println!("HDR_NULL_RESULT={v}");
}

#[test]
fn errors_row_14_null_pointer_same_signal() {
    use std::os::unix::process::ExitStatusExt;

    // Make sure both .so files exist before forking children.
    let _ = load_pair();

    let exe = std::env::current_exe().expect("current_exe");
    let run = |target: &str| -> (Option<i32>, Option<i32>, String) {
        let out = std::process::Command::new(&exe)
            .args(["--exact", "null_pointer_child", "--nocapture", "--test-threads=1"])
            .env("HDR_NULL_TARGET", target)
            .output()
            .expect("spawn child");
        let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
        (out.status.code(), out.status.signal(), stdout)
    };

    let (c_code, c_sig, c_out) = run("c");
    let (r_code, r_sig, r_out) = run("rust");

    // The C dereferences NULL+1 with no null check, so a fatal signal is the
    // expected observable behaviour; whatever it is, Rust must match it.
    assert_eq!(
        c_sig, r_sig,
        "ERRORS row 14: NULL produced different fatal signals \
         (C: signal={c_sig:?} code={c_code:?}, Rust: signal={r_sig:?} code={r_code:?})"
    );

    if c_sig.is_none() {
        // Neither faulted: then they must at least have returned the same value.
        let pick = |s: &str| -> Option<String> {
            s.lines()
                .find(|l| l.starts_with("HDR_NULL_RESULT="))
                .map(|l| l.to_string())
        };
        assert_eq!(
            pick(&c_out),
            pick(&r_out),
            "ERRORS row 14: NULL returned different values"
        );
    } else {
        assert_eq!(
            c_sig,
            Some(libc::SIGSEGV),
            "ERRORS row 14: expected SIGSEGV from the unchecked NULL deref, got {c_sig:?}"
        );
    }
}

// ===========================================================================
// Row 15 -- arbitrary / unaligned pointer positions.
// ===========================================================================

#[test]
fn errors_row_15_unaligned_pointers() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 15);

    #[repr(align(64))]
    struct Aligned([u8; 128]);
    let mut a = Aligned([0u8; 128]);

    for align in 0usize..64 {
        for _ in 0..64 {
            rng.fill(&mut a.0);
            let (x, y) = unsafe {
                let ptr = a.0.as_ptr().add(align);
                (p.c.call(ptr), p.rust.call(ptr))
            };
            assert_eq!(
                x, y,
                "ERRORS row 15: divergence at alignment {align} \
                 (h[1]={:#04x} h[2]={:#04x})",
                a.0[align + 1],
                a.0[align + 2]
            );
        }
    }
}

// ===========================================================================
// Row 16 -- buffer extent: minimum (3 bytes) vs very large.
// ===========================================================================

#[test]
fn errors_row_16_buffer_extents() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 16);

    for plane in PLANES {
        for layer_bits in LAYER_BITS {
            for rate in RATE_NIBBLES {
                let h0 = rng.next_u8();
                let h1 = make_h1(plane, layer_bits, rng.next_u8());
                let h2 = make_h2(rate, rng.next_u8());

                let small = vec![h0, h1, h2];
                let v_small = p.assert_same(&small, "ERRORS row 16 (3-byte buffer)");

                let mut big = vec![0u8; 1 << 16];
                rng.fill(&mut big);
                big[0] = h0;
                big[1] = h1;
                big[2] = h2;
                let v_big = p.assert_same(&big, "ERRORS row 16 (64 KiB buffer)");

                assert_eq!(
                    v_small, v_big,
                    "ERRORS row 16: buffer extent changed the result \
                     ({v_small} vs {v_big}) at h[1]={h1:#04x} h[2]={h2:#04x}"
                );
            }
        }
    }
}
