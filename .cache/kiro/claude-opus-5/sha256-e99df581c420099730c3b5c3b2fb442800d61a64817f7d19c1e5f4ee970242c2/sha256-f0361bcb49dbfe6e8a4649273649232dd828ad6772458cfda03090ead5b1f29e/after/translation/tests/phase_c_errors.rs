//! Phase C — error/rejection-path differential tests.
//!
//! One test per `ERRORS.md` row (1-8) plus the generic FFI boundary rows
//! (G1, G4, G6, G7). These tests also discharge `CONFIGS.md` rows 19-23
//! (the invalid-`h2` configurations), 28 and 29.
//!
//! This library has no error code channel: `hdr_compare` returns `int` 1 or 0.
//! So "same error" means the same rejection sentinel `0` *and* the same
//! return-value domain, asserted on every call by `Pair::assert_same`.

mod common;

use common::*;

const N: usize = 20_000;

// ---------------------------------------------------------------------------
// ERRORS row 1 / CONFIGS row 19 — h2[0] != 0xff
// ---------------------------------------------------------------------------

#[test]
fn err01_bad_sync_byte() {
    let p = Pair::load();
    let mut rng = Rng::seeded();
    let mut count = 0usize;
    for sync in 0u16..=255 {
        if sync as u8 == 0xff {
            continue;
        }
        for _ in 0..64 {
            // rest of h2 deliberately made *otherwise perfectly valid*, so the
            // sync byte is the only reason for rejection
            let h2 = [
                sync as u8,
                rng.pick(&valid_b1_values()),
                rng.pick(&valid_b2_values()),
            ];
            let h1 = matching_h1(&h2, &mut rng);
            assert_eq!(
                p.assert_same(&h1, &h2, "err01"),
                0,
                "bad sync {sync:#04x} must reject: h1={h1:02x?} h2={h2:02x?}"
            );
            count += 1;
        }
    }
    assert_eq!(count, 255 * 64);
}

// ---------------------------------------------------------------------------
// ERRORS row 2 / CONFIGS row 20 — h2[1] in neither accepted class
// ---------------------------------------------------------------------------

#[test]
fn err02_byte1_in_neither_class() {
    let p = Pair::load();
    let mut rng = Rng::seeded();
    let bad: Vec<u8> = (0u16..=255)
        .map(|v| v as u8)
        .filter(|&b| !class_f(b) && !class_e(b))
        .collect();
    // 256 - 16 (0xF0..0xFF) - 2 (0xE2,0xE3) = 238
    assert_eq!(bad.len(), 238, "byte-1 rejection set size changed");
    for &b1 in &bad {
        for _ in 0..32 {
            let h2 = [0xff, b1, rng.pick(&valid_b2_values())];
            let h1 = matching_h1(&h2, &mut rng);
            assert_eq!(
                p.assert_same(&h1, &h2, "err02"),
                0,
                "byte1 {b1:#04x} is in neither class and must reject"
            );
            // also with an h1 that agrees bit-for-bit
            assert_eq!(p.assert_same(&h2, &h2, "err02_alias"), 0);
        }
    }
}

// ---------------------------------------------------------------------------
// ERRORS row 3 / CONFIGS row 21 — layer field == 0
// ---------------------------------------------------------------------------

#[test]
fn err03_layer_zero() {
    let p = Pair::load();
    let mut rng = Rng::seeded();
    let zero_layer: Vec<u8> = (0u16..=255)
        .map(|v| v as u8)
        .filter(|&b| (class_f(b) || class_e(b)) && layer(b) == 0)
        .collect();
    assert_eq!(
        zero_layer,
        vec![0xF0, 0xF1, 0xF8, 0xF9],
        "layer-zero set changed"
    );
    for &b1 in &zero_layer {
        for _ in 0..N / 4 {
            let h2 = [0xff, b1, rng.pick(&valid_b2_values())];
            let h1 = matching_h1(&h2, &mut rng);
            assert_eq!(
                p.assert_same(&h1, &h2, "err03"),
                0,
                "layer 0 (byte1={b1:#04x}) must reject"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// ERRORS row 4 / CONFIGS row 22 — h2[2] >> 4 == 15
// ---------------------------------------------------------------------------

#[test]
fn err04_nibble_all_ones() {
    let p = Pair::load();
    let mut rng = Rng::seeded();
    for b2 in 0xF0u16..=0xFF {
        let b2 = b2 as u8;
        assert_eq!(nibble(b2), 15);
        for _ in 0..1_024 {
            let h2 = [0xff, rng.pick(&valid_b1_values()), b2];
            let h1 = matching_h1(&h2, &mut rng);
            assert_eq!(
                p.assert_same(&h1, &h2, "err04"),
                0,
                "nibble 15 (byte2={b2:#04x}) must reject"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// ERRORS row 5 / CONFIGS row 23 — (h2[2] >> 2) & 3 == 3
// ---------------------------------------------------------------------------

#[test]
fn err05_sbits_three() {
    let p = Pair::load();
    let mut rng = Rng::seeded();
    let bad: Vec<u8> = (0u16..=255)
        .map(|v| v as u8)
        .filter(|&b| sbits(b) == 3 && nibble(b) != 15) // isolate this reason
        .collect();
    assert_eq!(bad.len(), 15 * 4, "S==3 (nibble != 15) set size changed");
    for &b2 in &bad {
        for _ in 0..256 {
            let h2 = [0xff, rng.pick(&valid_b1_values()), b2];
            let h1 = matching_h1(&h2, &mut rng);
            assert_eq!(
                p.assert_same(&h1, &h2, "err05"),
                0,
                "S==3 (byte2={b2:#04x}) must reject"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// ERRORS rows 6, 7, 8 — the three comparison branches of hdr_compare itself.
// Exhaustive in the differing mask, so every distinct way to trip each branch
// is covered rather than a sample.
// ---------------------------------------------------------------------------

#[test]
fn err06_byte1_mask_fe_mismatch_exhaustive_masks() {
    let p = Pair::load();
    let mut rng = Rng::seeded();
    for diff in 1u16..=255 {
        let diff = diff as u8;
        if diff & 0xFE == 0 {
            continue; // that is row 10 (ignored bit), not a rejection
        }
        for _ in 0..16 {
            let h2 = [
                0xff,
                rng.pick(&valid_b1_values()),
                rng.pick(&valid_b2_values()),
            ];
            let mut h1 = matching_h1(&h2, &mut rng);
            h1[1] ^= diff;
            assert_eq!(
                p.assert_same(&h1, &h2, "err06"),
                0,
                "byte1 xor {diff:#04x} must reject: h1={h1:02x?} h2={h2:02x?}"
            );
        }
    }
}

#[test]
fn err07_byte2_mask_0c_mismatch_exhaustive() {
    let p = Pair::load();
    let mut rng = Rng::seeded();
    // Exhaustive over (h2[2], h1[2]) pairs where the 0x0C bits differ.
    for b2h2 in 0u16..=255 {
        let b2h2 = b2h2 as u8;
        if nibble(b2h2) == 15 || sbits(b2h2) == 3 {
            continue; // h2 would already be invalid for another reason
        }
        for b2h1 in 0u16..=255 {
            let b2h1 = b2h1 as u8;
            if (b2h1 ^ b2h2) & 0x0C == 0 {
                continue;
            }
            let h2 = [0xff, rng.pick(&valid_b1_values()), b2h2];
            let h1 = [rng.u8(), h2[1] & 0xFE, b2h1];
            assert_eq!(
                p.assert_same(&h1, &h2, "err07"),
                0,
                "byte2 0x0C mismatch must reject: h1={h1:02x?} h2={h2:02x?}"
            );
        }
    }
}

#[test]
fn err08_high_nibble_zeroness_xor_exhaustive() {
    let p = Pair::load();
    let mut rng = Rng::seeded();
    let mut cases = 0usize;
    for b2h2 in 0u16..=255 {
        let b2h2 = b2h2 as u8;
        if nibble(b2h2) == 15 || sbits(b2h2) == 3 {
            continue;
        }
        for b2h1 in 0u16..=255 {
            let b2h1 = b2h1 as u8;
            // isolate row 8: 0x0C bits MUST agree, zeroness MUST disagree
            if (b2h1 ^ b2h2) & 0x0C != 0 {
                continue;
            }
            if ((b2h1 & 0xF0) == 0) == ((b2h2 & 0xF0) == 0) {
                continue;
            }
            let h2 = [0xff, rng.pick(&valid_b1_values()), b2h2];
            let h1 = [rng.u8(), h2[1] | (rng.u8() & 0x01), b2h1];
            assert_eq!(
                p.assert_same(&h1, &h2, "err08"),
                0,
                "zeroness disagreement must reject: h1={h1:02x?} h2={h2:02x?}"
            );
            cases += 1;
        }
    }
    assert!(cases > 0, "row 8 isolation produced no cases");
    eprintln!("err08: {cases} isolated cases");
}

// ---------------------------------------------------------------------------
// G1 / CONFIGS row 29 — NULL h1 with an invalid h2 must not be dereferenced.
// ---------------------------------------------------------------------------

#[test]
fn g01_null_h1_with_invalid_h2() {
    let p = Pair::load();
    // One invalid h2 per rejection reason (ERRORS rows 1-5).
    let invalid: [[u8; 3]; 6] = [
        [0x00, 0xfb, 0x90], // row 1: bad sync
        [0xff, 0x12, 0x90], // row 2: byte1 neither class
        [0xff, 0xf0, 0x90], // row 3: layer 0
        [0xff, 0xf9, 0x90], // row 3: layer 0, other value
        [0xff, 0xfb, 0xf4], // row 4: nibble 15
        [0xff, 0xfb, 0x9c], // row 5: S == 3
    ];
    for h2 in &invalid {
        assert!(!h2_is_valid(h2));
        let r = p.assert_same_raw(std::ptr::null(), h2.as_ptr(), "g01");
        assert_eq!(r, 0, "NULL h1 with invalid h2={h2:02x?} must return 0");
    }
}

// ---------------------------------------------------------------------------
// G4 / CONFIGS row 28 — read extent: nothing past byte 2 is ever touched.
// The header is placed at the very end of a mapped page whose successor page
// is PROT_NONE, so any over-read faults the test process for BOTH libraries
// identically. Also verifies the short-circuit read order: when h2 is invalid
// early, later bytes are not read.
// ---------------------------------------------------------------------------

mod guard {
    use std::ffi::c_void;

    unsafe extern "C" {
        fn mmap(
            addr: *mut c_void,
            length: usize,
            prot: i32,
            flags: i32,
            fd: i32,
            offset: i64,
        ) -> *mut c_void;
        fn mprotect(addr: *mut c_void, len: usize, prot: i32) -> i32;
        fn munmap(addr: *mut c_void, len: usize) -> i32;
    }

    const PROT_NONE: i32 = 0;
    const PROT_READ: i32 = 1;
    const PROT_WRITE: i32 = 2;
    const MAP_PRIVATE: i32 = 0x02;
    const MAP_ANONYMOUS: i32 = 0x20;

    /// Two pages; the second is made PROT_NONE. Returns (base, page_size).
    pub struct Guarded {
        base: *mut u8,
        page: usize,
    }

    impl Guarded {
        pub fn new() -> Guarded {
            let page = 4096usize;
            let base = unsafe {
                mmap(
                    std::ptr::null_mut(),
                    page * 2,
                    PROT_READ | PROT_WRITE,
                    MAP_PRIVATE | MAP_ANONYMOUS,
                    -1,
                    0,
                )
            };
            assert!(
                base as isize != -1 && !base.is_null(),
                "mmap failed for the guard-page test"
            );
            let base = base as *mut u8;
            let rc = unsafe { mprotect(base.add(page) as *mut c_void, page, PROT_NONE) };
            assert_eq!(rc, 0, "mprotect(PROT_NONE) failed");
            Guarded { base, page }
        }

        /// Write `n` bytes so that byte `n-1` is the last readable byte before
        /// the guard page, and return a pointer to the first of them.
        pub fn place(&self, bytes: &[u8]) -> *const u8 {
            let n = bytes.len();
            unsafe {
                let dst = self.base.add(self.page - n);
                std::ptr::copy_nonoverlapping(bytes.as_ptr(), dst, n);
                dst
            }
        }
    }

    impl Drop for Guarded {
        fn drop(&mut self) {
            unsafe {
                munmap(self.base as *mut c_void, self.page * 2);
            }
        }
    }
}

#[test]
fn g04_no_read_past_byte_two() {
    let p = Pair::load();
    let g1 = guard::Guarded::new();
    let g2 = guard::Guarded::new();

    // Sanity: the guard really is unreadable would abort the process, so we do
    // not probe it; we only rely on it to catch an over-read.
    let cases: [([u8; 3], [u8; 3]); 8] = [
        ([0xff, 0xfb, 0x90], [0xff, 0xfb, 0x90]), // valid, accept
        ([0xa5, 0xfa, 0x91], [0xff, 0xfb, 0x90]), // valid h2, accept (ignored bits)
        ([0x00, 0x00, 0x00], [0xff, 0xfb, 0x90]), // valid h2, reject on byte1
        ([0xff, 0xfb, 0x9c], [0xff, 0xfb, 0x9c]), // invalid: S == 3
        ([0xff, 0xfb, 0xf0], [0xff, 0xfb, 0xf0]), // invalid: nibble 15
        ([0xff, 0xf0, 0x90], [0xff, 0xf0, 0x90]), // invalid: layer 0
        ([0xff, 0x11, 0x90], [0xff, 0x11, 0x90]), // invalid: byte1 class
        ([0x00, 0x00, 0x00], [0x00, 0x00, 0x00]), // invalid: sync
    ];
    for (h1, h2) in &cases {
        let expect = p.assert_same(h1, h2, "g04_baseline");
        let ph1 = g1.place(h1);
        let ph2 = g2.place(h2);
        let got = p.assert_same_raw(ph1, ph2, "g04_guarded");
        assert_eq!(
            got, expect,
            "guarded placement changed the result for h1={h1:02x?} h2={h2:02x?}"
        );
    }

    // Also place only 2 bytes of h2 before the guard, with byte 1 chosen so the
    // class test fails: the C short-circuits before loading h2[2], so neither
    // implementation may touch the guard page. (If either read h2[2] the process
    // would SIGSEGV and the test would fail loudly.)
    let two = g2.place(&[0xff, 0x11]);
    let r = p.assert_same_raw(std::ptr::null(), two, "g04_shortcircuit_byte2");
    assert_eq!(r, 0);

    // And with only 1 byte placed and a wrong sync byte: h2[1] must not be read.
    let one = g2.place(&[0xAA]);
    let r = p.assert_same_raw(std::ptr::null(), one, "g04_shortcircuit_byte1");
    assert_eq!(r, 0);
}

// ---------------------------------------------------------------------------
// G6 — return-value domain, and G7 — unaligned pointers.
// (assert_same already checks the C result is in {0,1} on every call; this test
// makes the check explicit over a broad random sample and asserts the Rust side
// too.)
// ---------------------------------------------------------------------------

#[test]
fn g06_g07_return_domain_and_unaligned() {
    let p = Pair::load();
    let mut rng = Rng::seeded();
    let mut buf = [0u8; 64];
    for _ in 0..100_000 {
        let h2 = [
            if rng.below(2) == 0 { 0xff } else { rng.u8() },
            rng.u8(),
            rng.u8(),
        ];
        let h1 = [rng.u8(), rng.u8(), rng.u8()];
        let expect = p.assert_same(&h1, &h2, "g06");
        assert!(expect == 0 || expect == 1);

        // odd (unaligned) offsets
        let o1 = 1 + (rng.below(7) * 2) as usize;
        let o2 = 32 + 1 + (rng.below(7) * 2) as usize;
        buf[o1..o1 + 3].copy_from_slice(&h1);
        buf[o2..o2 + 3].copy_from_slice(&h2);
        let got = unsafe {
            p.assert_same_raw(buf.as_ptr().add(o1), buf.as_ptr().add(o2), "g07")
        };
        assert_eq!(got, expect, "unaligned placement changed the result");
        assert!(got == 0 || got == 1);
    }
}

// ---------------------------------------------------------------------------
// G5 — there is no enum/flag parameter in this API, so "out-of-range enum
// value" has no analogue. The closest thing is that EVERY byte value is in
// range for both parameters; that is covered exhaustively by row24/row25.
// This test documents the reasoning and asserts the header really declares no
// such parameter, so the row cannot silently rot if the C API grows one.
// ---------------------------------------------------------------------------

#[test]
fn g05_no_enum_or_flag_parameter_in_the_api() {
    let hdr = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("c_src/include/lib.h"),
    )
    .expect("cannot read c_src/include/lib.h");
    assert!(
        !hdr.contains("enum"),
        "the C header now declares an enum; ERRORS.md G5 must be revisited:\n{hdr}"
    );
    // The only declared function takes exactly two pointer parameters.
    assert!(
        hdr.contains("int hdr_compare(const uint8_t *h1, const uint8_t *h2);"),
        "the C API changed shape; revisit ERRORS.md / CONFIGS.md:\n{hdr}"
    );
    let decls = hdr
        .lines()
        .filter(|l| l.contains('(') && l.trim_end().ends_with(';'))
        .count();
    assert_eq!(decls, 1, "more than one public function is now declared");
}
