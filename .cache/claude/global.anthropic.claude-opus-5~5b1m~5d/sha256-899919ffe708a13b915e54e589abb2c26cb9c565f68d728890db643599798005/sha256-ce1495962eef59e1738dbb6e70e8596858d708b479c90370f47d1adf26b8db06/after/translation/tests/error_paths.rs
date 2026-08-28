//! Phase C — error/rejection-path differential tests, one test per `ERRORS.md` row.
//!
//! The library has no error codes: every rejection is the C boolean `0`. The rows that
//! involve unreadable memory (`NULL`, truncated buffers) are compared by forking, so that
//! "C faults with SIGSEGV" vs "Rust faults with SIGSEGV" is asserted as the *same* outcome
//! rather than as "both failed somehow".

mod common;

use common::*;
use std::ffi::c_int;
use std::ptr;

/// E1 — `h2[0] != 0xFF`.
#[test]
fn e1_h2_byte0_not_ff() {
    for &b1 in valid_byte1().iter() {
        for &b2 in valid_byte2().iter() {
            for v in 0..=255u8 {
                if v == 0xFF {
                    continue;
                }
                let h2 = [v, b1, b2];
                let h1 = h2; // identical apart from byte 0, which is not read
                let got = diff3(&h1, &h2);
                assert_eq!(got, 0, "h2[0] = {v:#04X} must be rejected");
                assert_eq!(got, model(&h1, &h2));
            }
        }
    }
}

/// E2 — `h2[1]` in neither sync class.
#[test]
fn e2_h2_byte1_bad_sync_class() {
    let bad: Vec<u8> = (0..=255u8)
        .filter(|&v| (v & 0xF0) != 0xF0 && (v & 0xFE) != 0xE2)
        .collect();
    assert_eq!(bad.len(), 238, "sanity on the size of the bad-sync-class set");
    for &b1 in &bad {
        for &b2 in valid_byte2().iter() {
            let h2 = [0xFF, b1, b2];
            let h1 = [0xFF, b1, b2];
            let got = diff3(&h1, &h2);
            assert_eq!(got, 0, "h2[1] = {b1:#04X} is not a sync class, must be rejected");
            assert_eq!(got, model(&h1, &h2));
        }
    }
}

/// E3 — reserved layer index (`(h2[1] >> 1) & 3 == 0`).
#[test]
fn e3_h2_layer_reserved() {
    let reserved: Vec<u8> = (0..=255u8)
        .filter(|&v| ((v & 0xF0) == 0xF0 || (v & 0xFE) == 0xE2) && ((v >> 1) & 3) == 0)
        .collect();
    // 0xF0, 0xF1, 0xF8, 0xF9 pass the sync class but have layer == 0.
    assert_eq!(reserved, vec![0xF0, 0xF1, 0xF8, 0xF9]);
    for &b1 in &reserved {
        for &b2 in valid_byte2().iter() {
            let h2 = [0xFF, b1, b2];
            let got = diff3(&h2, &h2);
            assert_eq!(got, 0, "reserved layer in h2[1] = {b1:#04X} must be rejected");
            assert_eq!(got, model(&h2, &h2));
        }
    }
}

/// E4 — bad bitrate index (`h2[2] >> 4 == 15`).
#[test]
fn e4_h2_bitrate_index_15() {
    for &b1 in valid_byte1().iter() {
        for low in 0..16u8 {
            let b2 = 0xF0 | low;
            let h2 = [0xFF, b1, b2];
            let got = diff3(&h2, &h2);
            assert_eq!(got, 0, "bitrate index 15 (h2[2] = {b2:#04X}) must be rejected");
            assert_eq!(got, model(&h2, &h2));
        }
    }
}

/// E5 — reserved sample-rate index (`(h2[2] >> 2) & 3 == 3`).
#[test]
fn e5_h2_samplerate_reserved() {
    for &b1 in valid_byte1().iter() {
        for hi in 0..15u8 {
            for low in 0..4u8 {
                let b2 = (hi << 4) | 0x0C | low;
                assert_eq!((b2 >> 2) & 3, 3);
                let h2 = [0xFF, b1, b2];
                let got = diff3(&h2, &h2);
                assert_eq!(got, 0, "reserved samplerate (h2[2] = {b2:#04X}) must be rejected");
                assert_eq!(got, model(&h2, &h2));
            }
        }
    }
}

/// E6 — an invalid `h2` rejects regardless of `h1`: exhaustive over all 2^24 `h2` values.
#[test]
fn e6_invalid_h2_rejects_regardless_of_h1() {
    let l = libs();
    let mut h2 = [0u8; 3];
    let mut invalid_seen = 0u64;
    for h1 in H1_BATTERY.iter() {
        for v in (0..(1u32 << 24)).step_by(stride()) {
            h2[0] = v as u8;
            h2[1] = (v >> 8) as u8;
            h2[2] = (v >> 16) as u8;
            let valid = h2[0] == 0xff
                && ((h2[1] & 0xF0) == 0xf0 || (h2[1] & 0xFE) == 0xe2)
                && (((h2[1] >> 1) & 3) != 0)
                && ((h2[2] >> 4) != 15)
                && (((h2[2] >> 2) & 3) != 3);
            let a = unsafe { (l.c)(h1.as_ptr(), h2.as_ptr()) };
            let b = unsafe { (l.rs)(h1.as_ptr(), h2.as_ptr()) };
            if a != b {
                panic!("DIVERGENCE h1={h1:02X?} h2={h2:02X?}: C = {a}, Rust = {b}");
            }
            if !valid {
                if a != 0 {
                    panic!("invalid h2={h2:02X?} returned {a}");
                }
                invalid_seen += 1;
            }
        }
    }
    assert!(invalid_seen > 0);
}

/// E6b — the short-circuit contract: `h1` must not be dereferenced at all when `h2` is
/// invalid. Driven with `h1 = NULL` and with `h1` pointing into a `PROT_NONE` page.
#[test]
fn e6b_h1_never_dereferenced_when_h2_invalid() {
    let l = libs();

    // A PROT_NONE pointer: any read through it would fault.
    let page = unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as usize;
    let none = unsafe {
        libc::mmap(
            ptr::null_mut(),
            page,
            libc::PROT_NONE,
            libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
            -1,
            0,
        )
    };
    assert!(none != libc::MAP_FAILED);
    let none = none as *const u8;

    // Every distinct reason `hdr_valid` can fail.
    let invalid_h2: [[u8; 3]; 6] = [
        [0x00, 0xFB, 0x90], // E1: no sync byte
        [0xFF, 0x00, 0x90], // E2: bad sync class
        [0xFF, 0xF0, 0x90], // E3: reserved layer
        [0xFF, 0xF8, 0x90], // E3: reserved layer (other encoding)
        [0xFF, 0xFB, 0xF0], // E4: bitrate index 15
        [0xFF, 0xFB, 0x0C], // E5: reserved samplerate
    ];

    for h2 in invalid_h2.iter() {
        // In-process: nothing may be read from h1, so these must simply return 0.
        for h1 in [ptr::null::<u8>(), none] {
            let a = unsafe { (l.c)(h1, h2.as_ptr()) };
            let b = unsafe { (l.rs)(h1, h2.as_ptr()) };
            assert_eq!(a, 0, "C dereferenced h1 or returned {a} for invalid h2={h2:02X?}");
            assert_eq!(b, a, "DIVERGENCE for h1={h1:?} h2={h2:02X?}");
        }
        // And through the forked probe, so a fault would be visible as a signal.
        let out = assert_same_outcome("h1=NULL, invalid h2", ptr::null(), h2.as_ptr());
        assert_eq!(out, Outcome::Returned(0), "h2={h2:02X?}");
        let out = assert_same_outcome("h1=PROT_NONE, invalid h2", none, h2.as_ptr());
        assert_eq!(out, Outcome::Returned(0), "h2={h2:02X?}");
    }

    unsafe { libc::munmap(none as *mut libc::c_void, page) };
}

/// E7 — version/layer mismatch: all 256x256 `(h1[1], h2[1])` pairs.
#[test]
fn e7_byte1_high7_mismatch() {
    let l = libs();
    let mut mismatch_rejections = 0u64;
    for &b2 in valid_byte2().iter().step_by(7) {
        for a1 in 0..=255u8 {
            for c1 in 0..=255u8 {
                let h1 = [0x00, a1, b2];
                let h2 = [0xFF, c1, b2];
                let a = unsafe { (l.c)(h1.as_ptr(), h2.as_ptr()) };
                let b = unsafe { (l.rs)(h1.as_ptr(), h2.as_ptr()) };
                assert_eq!(a, b, "DIVERGENCE h1={h1:02X?} h2={h2:02X?}");
                assert_eq!(a, model(&h1, &h2));
                let valid = ((c1 & 0xF0) == 0xf0 || (c1 & 0xFE) == 0xe2) && ((c1 >> 1) & 3) != 0;
                if valid && ((a1 ^ c1) & 0xFE) != 0 {
                    assert_eq!(a, 0, "byte-1 mismatch must be rejected: {h1:02X?} {h2:02X?}");
                    mismatch_rejections += 1;
                }
            }
        }
    }
    assert!(mismatch_rejections > 0);
}

/// E8 — sample-rate mismatch: all 256x256 `(h1[2], h2[2])` pairs.
#[test]
fn e8_byte2_samplerate_mismatch() {
    let l = libs();
    let mut rejections = 0u64;
    for &b1 in valid_byte1().iter() {
        for a2 in 0..=255u8 {
            for c2 in 0..=255u8 {
                let h1 = [0x00, b1, a2];
                let h2 = [0xFF, b1, c2];
                let a = unsafe { (l.c)(h1.as_ptr(), h2.as_ptr()) };
                let b = unsafe { (l.rs)(h1.as_ptr(), h2.as_ptr()) };
                assert_eq!(a, b, "DIVERGENCE h1={h1:02X?} h2={h2:02X?}");
                assert_eq!(a, model(&h1, &h2));
                let h2_valid = (c2 >> 4) != 15 && ((c2 >> 2) & 3) != 3;
                if h2_valid && ((a2 ^ c2) & 0x0C) != 0 {
                    assert_eq!(a, 0, "samplerate mismatch must be rejected");
                    rejections += 1;
                }
            }
        }
    }
    assert!(rejections > 0);
}

/// E9 — free-format disagreement (exactly one of the two bitrate indices is zero).
#[test]
fn e9_byte2_freeformat_xor() {
    let l = libs();
    let mut rejections = 0u64;
    for &b1 in valid_byte1().iter() {
        for a2 in 0..=255u8 {
            for c2 in 0..=255u8 {
                let h1 = [0x00, b1, a2];
                let h2 = [0xFF, b1, c2];
                let a = unsafe { (l.c)(h1.as_ptr(), h2.as_ptr()) };
                let b = unsafe { (l.rs)(h1.as_ptr(), h2.as_ptr()) };
                assert_eq!(a, b, "DIVERGENCE h1={h1:02X?} h2={h2:02X?}");
                let h2_valid = (c2 >> 4) != 15 && ((c2 >> 2) & 3) != 3;
                let sr_ok = ((a2 ^ c2) & 0x0C) == 0;
                let ff_disagree = ((a2 & 0xF0) == 0) != ((c2 & 0xF0) == 0);
                if h2_valid && sr_ok && ff_disagree {
                    assert_eq!(
                        a, 0,
                        "free-format disagreement must be rejected: {h1:02X?} {h2:02X?}"
                    );
                    rejections += 1;
                }
            }
        }
    }
    assert!(rejections > 0, "the free-format branch was never exercised");
}

/// E10 — `h2 == NULL`.
#[test]
fn e10_h2_null_faults_in_both() {
    let h1: [u8; 3] = [0xFF, 0xFB, 0x90];
    let out = assert_same_outcome("h2=NULL", h1.as_ptr(), ptr::null());
    assert_eq!(
        out,
        Outcome::Signal(libc::SIGSEGV),
        "the C has no NULL check, both must fault identically"
    );
}

/// E11 — `h1 == NULL` with a *valid* `h2` (the short circuit does not protect `h1`).
#[test]
fn e11_h1_null_valid_h2_faults_in_both() {
    let h2: [u8; 3] = [0xFF, 0xFB, 0x90];
    let out = assert_same_outcome("h1=NULL, valid h2", ptr::null(), h2.as_ptr());
    assert_eq!(out, Outcome::Signal(libc::SIGSEGV));
}

/// E12 — both pointers `NULL`.
#[test]
fn e12_both_null_faults_in_both() {
    let out = assert_same_outcome("both NULL", ptr::null(), ptr::null());
    assert_eq!(out, Outcome::Signal(libc::SIGSEGV));
}

/// E13 — `h2` truncated after 1 readable byte.
#[test]
fn e13_h2_truncated_after_1_byte() {
    let g = GuardedBuf::new();
    let p2 = g.put_tail(&[0xFF]);
    let h1: [u8; 3] = [0xFF, 0xFB, 0x90];
    let out = assert_same_outcome("h2 has 1 readable byte", h1.as_ptr(), p2);
    assert_eq!(out, Outcome::Signal(libc::SIGSEGV), "reading h2[1] must fault");
}

/// E14 — `h2` truncated after 2 readable bytes.
#[test]
fn e14_h2_truncated_after_2_bytes() {
    let g = GuardedBuf::new();
    let p2 = g.put_tail(&[0xFF, 0xFB]);
    let h1: [u8; 3] = [0xFF, 0xFB, 0x90];
    let out = assert_same_outcome("h2 has 2 readable bytes", h1.as_ptr(), p2);
    assert_eq!(out, Outcome::Signal(libc::SIGSEGV), "reading h2[2] must fault");
}

/// E15 — `h1` truncated after 1 readable byte, `h2` valid.
#[test]
fn e15_h1_truncated_after_1_byte() {
    let g = GuardedBuf::new();
    let p1 = g.put_tail(&[0xFF]);
    let h2: [u8; 3] = [0xFF, 0xFB, 0x90];
    let out = assert_same_outcome("h1 has 1 readable byte", p1, h2.as_ptr());
    assert_eq!(out, Outcome::Signal(libc::SIGSEGV), "reading h1[1] must fault");
}

/// E16 — `h1` truncated after 2 readable bytes, matching `h2[1]` so `h1[2]` is reached.
#[test]
fn e16_h1_truncated_after_2_bytes() {
    let g = GuardedBuf::new();
    let p1 = g.put_tail(&[0x00, 0xFB]);
    let h2: [u8; 3] = [0xFF, 0xFB, 0x90];
    let out = assert_same_outcome("h1 has 2 readable bytes", p1, h2.as_ptr());
    assert_eq!(out, Outcome::Signal(libc::SIGSEGV), "reading h1[2] must fault");
}

/// E17 — zero readable bytes at all (pointer straight at the guard page).
#[test]
fn e17_h2_zero_readable_bytes() {
    let g = GuardedBuf::new();
    let p2 = g.put_tail(&[]);
    let h1: [u8; 3] = [0xFF, 0xFB, 0x90];
    let out = assert_same_outcome("h2 has 0 readable bytes", h1.as_ptr(), p2);
    assert_eq!(out, Outcome::Signal(libc::SIGSEGV), "reading h2[0] must fault");

    // Symmetric case: h1 has 0 readable bytes but h2 is invalid -> h1 is never touched.
    let invalid: [u8; 3] = [0x00, 0xFB, 0x90];
    let p1 = g.put_tail(&[]);
    let out = assert_same_outcome("h1 has 0 readable bytes, invalid h2", p1, invalid.as_ptr());
    assert_eq!(out, Outcome::Returned(0));
}

/// E18 — neither implementation may read past index 2.
#[test]
fn e18_no_read_past_index_2() {
    let g1 = GuardedBuf::new();
    let g2 = GuardedBuf::new();
    let mut rng = Rng::new(18_000);
    let vb1 = valid_byte1();
    let vb2 = valid_byte2();

    for i in 0..iters(4_000) as u32 {
        let h2 = if i % 2 == 0 {
            [0xFF, rng.pick(&vb1), rng.pick(&vb2)]
        } else {
            rng.bytes3()
        };
        let h1 = if i % 3 == 0 { h2 } else { rng.bytes3() };
        let p1 = g1.put_tail(&h1);
        let p2 = g2.put_tail(&h2);
        // If either implementation read index >= 3 this would die with SIGSEGV; compare the
        // full outcome so a fault in only one of them is caught.
        let out = assert_same_outcome("exactly 3 readable bytes each", p1, p2);
        let expect = model(&h1, &h2);
        assert_eq!(
            out,
            Outcome::Returned(expect),
            "h1={h1:02X?} h2={h2:02X?} must return {expect} without over-reading"
        );
    }
}

/// E19 — the cross-product of every out-of-range ("reserved") bit-field encoding.
#[test]
fn e19_all_reserved_field_encodings() {
    // layer: 0 reserved; bitrate: 15 bad; samplerate: 3 reserved.
    // Enumerate all combinations of {valid, reserved} for each field, on both headers, and
    // check every case where h2 carries at least one reserved encoding is rejected.
    let layers = [0u8, 1, 2, 3];
    let bitrates = [0u8, 1, 7, 14, 15];
    let samplerates = [0u8, 1, 2, 3];
    for &sync in &[0xF0u8, 0xE2u8] {
        for &l1 in &layers {
            for &b1r in &bitrates {
                for &s1 in &samplerates {
                    for &l2 in &layers {
                        for &b2r in &bitrates {
                            for &s2 in &samplerates {
                                let h1 = [0x00, (sync & 0xF9) | (l1 << 1), (b1r << 4) | (s1 << 2)];
                                let h2 = [0xFF, (sync & 0xF9) | (l2 << 1), (b2r << 4) | (s2 << 2)];
                                let got = diff3(&h1, &h2);
                                assert_eq!(got, model(&h1, &h2), "{h1:02X?} {h2:02X?}");

                                // Derive the fields back out of the bytes actually built,
                                // rather than trusting the encoder.
                                let layer2 = (h2[1] >> 1) & 3;
                                let bitrate2 = h2[2] >> 4;
                                let srate2 = (h2[2] >> 2) & 3;
                                if layer2 == 0 || bitrate2 == 15 || srate2 == 3 {
                                    assert_eq!(
                                        got, 0,
                                        "h2 with a reserved field encoding must be rejected: \
                                         {h2:02X?} (layer={layer2} bitrate={bitrate2} \
                                         samplerate={srate2})"
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// E20 — the return value is strictly the C boolean domain `{0, 1}`.
#[test]
fn e20_return_value_is_strictly_0_or_1() {
    let l = libs();
    let mut h2 = [0u8; 3];
    let h1_set: [[u8; 3]; 3] = [[0x00, 0x00, 0x00], [0xFF, 0xFB, 0x90], [0xFF, 0xFF, 0xFF]];
    for h1 in h1_set.iter() {
        for v in (0..(1u32 << 24)).step_by(stride()) {
            h2[0] = v as u8;
            h2[1] = (v >> 8) as u8;
            h2[2] = (v >> 16) as u8;
            let a = unsafe { (l.c)(h1.as_ptr(), h2.as_ptr()) };
            let b = unsafe { (l.rs)(h1.as_ptr(), h2.as_ptr()) };
            if a != b {
                panic!("DIVERGENCE h1={h1:02X?} h2={h2:02X?}: C = {a}, Rust = {b}");
            }
            if a != 0 && a != 1 {
                panic!("C returned non-boolean {a} for h1={h1:02X?} h2={h2:02X?}");
            }
            if b != 0 && b != 1 {
                panic!("Rust returned non-boolean {b} for h1={h1:02X?} h2={h2:02X?}");
            }
        }
    }
}

/// E21 — byte-level short circuit inside `hdr_valid`: when `h2[0] != 0xFF` the C stops
/// immediately, so a `h2` buffer with a *single* readable non-sync byte must NOT fault.
#[test]
fn e21_byte0_short_circuit_no_read_of_byte1() {
    let g = GuardedBuf::new();
    let h1: [u8; 3] = [0xFF, 0xFB, 0x90];
    for b0 in [0x00u8, 0x01, 0xFE, 0x7F, 0xF0] {
        let p2 = g.put_tail(&[b0]);
        let out = assert_same_outcome("h2[0] != 0xFF with 1 readable byte", h1.as_ptr(), p2);
        assert_eq!(
            out,
            Outcome::Returned(0),
            "h2[0] = {b0:#04X} must reject without reading h2[1]"
        );
    }
    // Control: with the sync byte present, reading h2[1] *must* fault.
    let p2 = g.put_tail(&[0xFF]);
    let out = assert_same_outcome("control: h2[0] == 0xFF, 1 readable byte", h1.as_ptr(), p2);
    assert_eq!(out, Outcome::Signal(libc::SIGSEGV));
}

/// E22 — byte-level short circuit inside `hdr_valid`: when `h2[1]` fails the sync-class or
/// the layer check the C stops before touching `h2[2]`, so 2 readable bytes must not fault.
#[test]
fn e22_byte1_short_circuit_no_read_of_byte2() {
    let g = GuardedBuf::new();
    let h1: [u8; 3] = [0xFF, 0xFB, 0x90];

    // Bad sync class (E2) and reserved layer (E3) both stop before h2[2].
    for b1 in [0x00u8, 0x01, 0x7F, 0xE0, 0xE1, 0xE4, 0xEF, 0xF0, 0xF1, 0xF8, 0xF9] {
        let p2 = g.put_tail(&[0xFF, b1]);
        let out = assert_same_outcome("h2[1] rejected with 2 readable bytes", h1.as_ptr(), p2);
        assert_eq!(
            out,
            Outcome::Returned(0),
            "h2 = [FF, {b1:02X}] must reject without reading h2[2]"
        );
    }
    // Control: an accepted h2[1] means h2[2] is read and must fault.
    for b1 in [0xE2u8, 0xE3, 0xF2, 0xFB, 0xFF] {
        let p2 = g.put_tail(&[0xFF, b1]);
        let out = assert_same_outcome("control: accepted h2[1], 2 readable bytes", h1.as_ptr(), p2);
        assert_eq!(
            out,
            Outcome::Signal(libc::SIGSEGV),
            "h2 = [FF, {b1:02X}] must go on to read h2[2]"
        );
    }
}

/// E23 — `h1[2]` is only read once `h1[1]` matched: with `h1` truncated to 2 readable bytes
/// and a *mismatching* `h1[1]`, the C stops before `h1[2]`, so there must be no fault.
#[test]
fn e23_h1_byte1_mismatch_short_circuits_before_byte2() {
    let g = GuardedBuf::new();
    let h2: [u8; 3] = [0xFF, 0xFB, 0x90];
    // 0x00 differs from 0xFB in bits 1..7 -> the byte-1 check fails, h1[2] is never read.
    for b1 in [0x00u8, 0x01, 0xF3, 0xFF] {
        let p1 = g.put_tail(&[0x00, b1]);
        let expect_fault = (b1 ^ h2[1]) & 0xFE == 0;
        let out = assert_same_outcome("h1 truncated, byte-1 mismatch", p1, h2.as_ptr());
        if expect_fault {
            assert_eq!(
                out,
                Outcome::Signal(libc::SIGSEGV),
                "h1[1] = {b1:02X} matches, so h1[2] must be read and fault"
            );
        } else {
            assert_eq!(
                out,
                Outcome::Returned(0),
                "h1[1] = {b1:02X} mismatches, so h1[2] must not be read"
            );
        }
    }
}

/// Extra generic-boundary coverage: unaligned / misaligned pointers, maximum-address-ish
/// pointers and a pointer that is non-null but wildly invalid.
#[test]
fn generic_boundaries_bad_pointers() {
    let h: [u8; 3] = [0xFF, 0xFB, 0x90];
    for &bad in &[1usize, 2, 3, 7, 0xFFFF_FFFF_FFFF_FFF8usize, 0xDEAD_BEEFusize] {
        let p = bad as *const u8;
        // Invalid h2 pointer -> both must fault the same way.
        let out = assert_same_outcome("bad h2 pointer", h.as_ptr(), p);
        assert!(
            matches!(out, Outcome::Signal(_)),
            "expected a fatal signal for h2 = {bad:#x}, got {out:?}"
        );
        // Invalid h1 pointer with a *valid* h2 -> both must fault the same way.
        let out = assert_same_outcome("bad h1 pointer", p, h.as_ptr());
        assert!(
            matches!(out, Outcome::Signal(_)),
            "expected a fatal signal for h1 = {bad:#x}, got {out:?}"
        );
        // Invalid h1 pointer with an *invalid* h2 -> short circuit, both return 0.
        let invalid: [u8; 3] = [0x00, 0xFB, 0x90];
        let a = unsafe { (libs().c)(p, invalid.as_ptr()) };
        let b = unsafe { (libs().rs)(p, invalid.as_ptr()) };
        assert_eq!((a, b), (0 as c_int, 0 as c_int), "short circuit broken for {bad:#x}");
    }
}
