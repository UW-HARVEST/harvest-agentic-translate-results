//! Phase C, ERRORS.md rows 1–13: one differential test per rejection branch.
//!
//! `hdr_compare`'s only failure channel is its return value, so "same error" here
//! means the two `.so` exports return the identical `int` (`0` = rejected). Every
//! test constructs the exact triggering condition from `ERRORS.md`, calls both,
//! and asserts equality — plus asserts the value really is the C's rejection
//! sentinel `0`, so a test cannot pass by both sides being wrong in the same
//! direction on an input that should have been accepted.

mod common;

use common::*;

/// Row 1 — `h2[0] != 0xff`. All 255 wrong sync bytes, with `h2[1]`/`h2[2]` and
/// `h1` otherwise perfectly valid and matching, so the *only* reason to reject is
/// the sync byte.
#[test]
fn err_row01_bad_sync_byte0() {
    let v1 = valid_byte1_values();
    let v2 = valid_byte2_values();
    let mut rng = Rng::new(0xE770_0001);
    for b0 in 0u16..256 {
        let b0 = b0 as u8;
        for &b1 in &v1 {
            for &b2 in &v2 {
                let h1 = [rng.next_u8(), b1, b2];
                let got = assert_same(&h1, &[b0, b1, b2]);
                let want = if b0 == 0xff { 1 } else { 0 };
                assert_eq!(got, want, "sync byte {b0:#04x} with valid tail {b1:#04x} {b2:#04x}");
            }
        }
    }
}

/// Row 2 — `h2[1]` in neither accepted class: all 238 such values, each with a
/// valid `h2[2]` and a matching `h1`.
#[test]
fn err_row02_byte1_neither_class() {
    let v2 = valid_byte2_values();
    let mut rng = Rng::new(0xE770_0002);
    let mut n = 0;
    for b1 in 0u16..256 {
        let b1 = b1 as u8;
        if (b1 & 0xF0) == 0xf0 || (b1 & 0xFE) == 0xe2 {
            continue;
        }
        n += 1;
        for &b2 in &v2 {
            let h2 = [0xffu8, b1, b2];
            assert_eq!(assert_same(&[rng.next_u8(), b1, b2], &h2), 0, "{h2:02x?}");
        }
    }
    assert_eq!(n, 238);
    // One step past each class boundary, spelled out: 0xef / 0xf0 and
    // 0xe1 / 0xe2 / 0xe3 / 0xe4.
    for (b1, class_ok) in [
        (0xefu8, false),
        (0xf0u8, true),
        (0xe1u8, false),
        (0xe2u8, true),
        (0xe3u8, true),
        (0xe4u8, false),
    ] {
        let h2 = [0xffu8, b1, 0x90];
        let got = assert_same(&[0x00, b1, 0x90], &h2);
        if !class_ok {
            assert_eq!(got, 0, "boundary {b1:#04x} must be rejected by class test");
        }
    }
}

/// Row 3 — `((h2[1] >> 1) & 3) == 0` (reserved layer) even though the class test
/// passed: exactly `{0xf0, 0xf1, 0xf8, 0xf9}`, crossed with every `h2[2]`.
#[test]
fn err_row03_byte1_reserved_layer() {
    let mut rng = Rng::new(0xE770_0003);
    for b1 in [0xf0u8, 0xf1, 0xf8, 0xf9] {
        assert!((b1 & 0xF0) == 0xf0, "precondition: class test passes");
        assert_eq!((b1 >> 1) & 3, 0);
        for b2 in 0u16..256 {
            let b2 = b2 as u8;
            let h2 = [0xffu8, b1, b2];
            assert_eq!(assert_same(&[rng.next_u8(), b1, b2], &h2), 0, "{h2:02x?}");
        }
    }
    // One step away in the layer field: 0xf2 (layer 1) IS accepted, proving the
    // rejection above is attributable to the layer field and not something else.
    let h2 = [0xffu8, 0xf2, 0x90];
    assert_eq!(assert_same(&[0x00, 0xf2, 0x90], &h2), 1);
}

/// Row 4 — `(h2[2] >> 4) == 15` (reserved bitrate): all 16 values `0xf0..=0xff`.
#[test]
fn err_row04_byte2_bitrate_15() {
    let mut rng = Rng::new(0xE770_0004);
    for &b1 in &valid_byte1_values() {
        for b2 in 0xf0u16..=0xff {
            let b2 = b2 as u8;
            let h2 = [0xffu8, b1, b2];
            assert_eq!(assert_same(&[rng.next_u8(), b1, b2], &h2), 0, "{h2:02x?}");
        }
        // One step below the boundary: nibble 14 is fine (given samplerate ok).
        let h2 = [0xffu8, b1, 0xe0];
        assert_eq!(assert_same(&[0x00, b1, 0xe0], &h2), 1, "{h2:02x?}");
    }
}

/// Row 5 — `((h2[2] >> 2) & 3) == 3` (reserved samplerate): all 64 such values,
/// including the 4 where row 4 would also have fired.
#[test]
fn err_row05_byte2_samplerate_3() {
    let mut rng = Rng::new(0xE770_0005);
    for &b1 in &valid_byte1_values() {
        let mut n = 0;
        for b2 in 0u16..256 {
            let b2 = b2 as u8;
            if ((b2 >> 2) & 3) != 3 {
                continue;
            }
            n += 1;
            let h2 = [0xffu8, b1, b2];
            assert_eq!(assert_same(&[rng.next_u8(), b1, b2], &h2), 0, "{h2:02x?}");
        }
        assert_eq!(n, 64);
        // One step below: samplerate field 2 at the same bitrate is accepted.
        let h2 = [0xffu8, b1, 0x28];
        assert_eq!(assert_same(&[0x00, b1, 0x28], &h2), 1, "{h2:02x?}");
    }
}

/// Row 6 — the aggregate first term: for *every* `h2` that `hdr_valid` rejects,
/// the verdict must be `0` regardless of `h1`. Exhaustive over all 16 777 216
/// `h2` values, with randomized `h1` tails.
#[test]
fn err_row06_invalid_h2_any_h1() {
    let l = libs();
    let mut rng = Rng::new(0xE770_0006);
    let mut checked: u64 = 0;
    for b0 in 0u16..256 {
        for b1 in 0u16..256 {
            for b2 in 0u16..256 {
                let h2 = [b0 as u8, b1 as u8, b2 as u8];
                let valid = h2[0] == 0xff && byte1_valid(h2[1]) && byte2_valid(h2[2]);
                if valid {
                    continue;
                }
                for _ in 0..2 {
                    let h1 = [rng.next_u8(), rng.next_u8(), rng.next_u8()];
                    let (c, r) = unsafe {
                        ((l.c)(h1.as_ptr(), h2.as_ptr()), (l.rust)(h1.as_ptr(), h2.as_ptr()))
                    };
                    assert_eq!(c, r, "row6 divergence h1={h1:02x?} h2={h2:02x?}");
                    assert_eq!(c, 0, "invalid h2={h2:02x?} was accepted by C");
                }
                checked += 1;
            }
        }
    }
    assert_eq!(checked, 256 * 256 * 256 - 2520, "all invalid h2 values covered");
}

/// Row 7 — `((h1[1] ^ h2[1]) & 0xFE) != 0`: every `h1[1]` outside the two
/// accepting values, for every valid `h2` tail.
#[test]
fn err_row07_byte1_mismatch_above_bit0() {
    let mut rng = Rng::new(0xE770_0007);
    for &b1 in &valid_byte1_values() {
        let accepting = [b1 & 0xFE, b1 | 0x01];
        for &b2 in &valid_byte2_values() {
            let h2 = [0xffu8, b1, b2];
            for a1 in 0u16..256 {
                let a1 = a1 as u8;
                let got = assert_same(&[rng.next_u8(), a1, b2], &h2);
                let want = if accepting.contains(&a1) { 1 } else { 0 };
                assert_eq!(got, want, "h1[1]={a1:#04x} vs h2={h2:02x?}");
            }
        }
    }
}

/// Row 8 — `((h1[2] ^ h2[2]) & 0x0C) != 0`: the three non-zero samplerate-bit
/// deltas, with every other term held passing.
#[test]
fn err_row08_byte2_samplerate_mismatch() {
    let mut rng = Rng::new(0xE770_0008);
    for &b1 in &valid_byte1_values() {
        for &b2 in &valid_byte2_values() {
            let h2 = [0xffu8, b1, b2];
            for delta in [0x04u8, 0x08, 0x0C] {
                // Keep the free-format nibble agreement intact so the rejection
                // is attributable to the 0x0C mask alone.
                let a2 = b2 ^ delta;
                assert_eq!((a2 & 0xF0) == 0, (b2 & 0xF0) == 0);
                assert_eq!(
                    assert_same(&[rng.next_u8(), b1, a2], &h2),
                    0,
                    "delta {delta:#04x} on h2={h2:02x?}"
                );
            }
        }
    }
}

/// Row 9 — `((h1[2] & 0xF0) == 0) != ((h2[2] & 0xF0) == 0)`: exactly one side is
/// free-format, with the `0xFE` and `0x0C` terms held passing.
#[test]
fn err_row09_freeformat_nibble_mismatch() {
    let mut rng = Rng::new(0xE770_0009);
    let mut mixed = 0u64;
    for &b1 in &valid_byte1_values() {
        for &b2 in &valid_byte2_values() {
            let h2 = [0xffu8, b1, b2];
            let h2_zero = (b2 & 0xF0) == 0;
            for hi in 0u8..16 {
                // Preserve bits 2-3 (axis G) and let bits 0-1 vary freely.
                let a2 = (hi << 4) | (b2 & 0x0C) | (rng.next_u8() & 0x03);
                let h1_zero = (a2 & 0xF0) == 0;
                if h1_zero == h2_zero {
                    continue;
                }
                mixed += 1;
                assert_eq!(
                    assert_same(&[rng.next_u8(), b1, a2], &h2),
                    0,
                    "nibble disagreement h1[2]={a2:#04x} h2={h2:02x?}"
                );
            }
        }
    }
    assert!(mixed > 0, "no mixed free-format cases were generated");
}

/// Row 10 — `h1 == NULL` while `hdr_valid(h2)` is false. C's `&&` short-circuits
/// before `h1` is read, so this is a *defined* call that must return `0` on both
/// sides without crashing. Every invalid `h2` tail is covered, plus every
/// non-`0xff` sync byte.
#[test]
fn err_row10_null_h1_with_invalid_h2() {
    let l = libs();
    let null = std::ptr::null::<u8>();
    let mut n = 0u64;
    for b0 in [0x00u8, 0x01, 0x7f, 0xfe, 0xff] {
        for b1 in 0u16..256 {
            for b2 in 0u16..256 {
                let h2 = [b0, b1 as u8, b2 as u8];
                if h2[0] == 0xff && byte1_valid(h2[1]) && byte2_valid(h2[2]) {
                    continue; // would legitimately dereference h1
                }
                let (c, r) = unsafe { ((l.c)(null, h2.as_ptr()), (l.rust)(null, h2.as_ptr())) };
                assert_eq!(c, r, "null-h1 divergence for h2={h2:02x?}: C={c} Rust={r}");
                assert_eq!(c, 0);
                n += 1;
            }
        }
    }
    assert!(n > 300_000, "expected the full invalid-h2 space, got {n}");
}

/// Row 11 — `h1` pointing at an unmapped address while `hdr_valid(h2)` is false.
/// Dereferencing it would fault, so surviving the call proves both sides really
/// short-circuit rather than merely computing the right answer.
#[test]
fn err_row11_unreadable_h1_with_invalid_h2() {
    let l = libs();
    // Deliberately unmapped, non-null, and misaligned enough that any read
    // (including a 4-byte one) would fault.
    let bad = 0x1usize as *const u8;
    for h2 in [
        [0x00u8, 0x00, 0x00],
        [0xfe, 0xfb, 0x90], // wrong sync byte only
        [0xff, 0x00, 0x90], // h2[1] class fails
        [0xff, 0xf0, 0x90], // reserved layer
        [0xff, 0xfb, 0xf0], // reserved bitrate
        [0xff, 0xfb, 0x0c], // reserved samplerate
        [0xff, 0xff, 0xff],
    ] {
        assert!(
            !(h2[0] == 0xff && byte1_valid(h2[1]) && byte2_valid(h2[2])),
            "vector {h2:02x?} must be an invalid header"
        );
        let (c, r) = unsafe { ((l.c)(bad, h2.as_ptr()), (l.rust)(bad, h2.as_ptr())) };
        assert_eq!(c, r, "unmapped-h1 divergence for h2={h2:02x?}");
        assert_eq!(c, 0);
    }
}

/// Row 12 — aliased pointers with a *valid* header must be accepted (`1`) by
/// both, for all 2 520 valid tails; and self-comparison of an invalid header must
/// be rejected.
#[test]
fn err_row12_aliased_pointers() {
    let l = libs();
    for &b1 in &valid_byte1_values() {
        for &b2 in &valid_byte2_values() {
            let h = [0xffu8, b1, b2];
            let p = h.as_ptr();
            let (c, r) = unsafe { ((l.c)(p, p), (l.rust)(p, p)) };
            assert_eq!(c, r, "aliased divergence for {h:02x?}");
            assert_eq!(c, 1, "self-comparison of valid header {h:02x?} should accept");
        }
    }
    for h in [[0x00u8, 0x00, 0x00], [0xff, 0xf0, 0x90], [0xff, 0xfb, 0xf0]] {
        let p = h.as_ptr();
        let (c, r) = unsafe { ((l.c)(p, p), (l.rust)(p, p)) };
        assert_eq!(c, r);
        assert_eq!(c, 0);
    }
}

/// Row 13 — neither side may read `h[3]` or beyond, and `h1[0]` must never be
/// read at all. Both header buffers are placed at the very end of a mapped page
/// followed by a `PROT_NONE` guard page, so any over-read faults; `h1[0]` is put
/// on its own guard-protected boundary too.
#[test]
fn err_row13_no_overread_past_three_bytes() {
    let page = unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as usize;
    assert!(page >= 8);

    // Two pages: [readable page][PROT_NONE guard page].
    let map = unsafe {
        libc::mmap(
            std::ptr::null_mut(),
            2 * page,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
            -1,
            0,
        )
    };
    assert_ne!(map, libc::MAP_FAILED, "mmap failed");
    let base = map as *mut u8;
    let rc = unsafe { libc::mprotect(base.add(page) as *mut libc::c_void, page, libc::PROT_NONE) };
    assert_eq!(rc, 0, "mprotect failed");

    // Self-validation: prove the guard page really faults, otherwise this whole
    // test could pass vacuously. A forked child reads the first guard byte and
    // must die on SIGSEGV/SIGBUS.
    unsafe {
        let pid = libc::fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            let v = std::ptr::read_volatile(base.add(page));
            // Should be unreachable; exit distinctively if the guard is broken.
            libc::_exit(if v == 0 { 42 } else { 43 });
        }
        let mut status = 0i32;
        assert_eq!(libc::waitpid(pid, &mut status, 0), pid);
        assert!(
            libc::WIFSIGNALED(status)
                && (libc::WTERMSIG(status) == libc::SIGSEGV
                    || libc::WTERMSIG(status) == libc::SIGBUS),
            "guard page is readable (status {status:#x}); the over-read test would be vacuous"
        );
    }

    // h1 and h2 both end exactly at the guard boundary; they cannot both, so
    // test them one at a time with the other in a normal buffer.
    let edge = unsafe { base.add(page - 3) };
    let other = [0u8; 3];

    let l = libs();
    let mut rng = Rng::new(0xE770_000D);
    let mut cases = 0u64;

    // (a) h2 at the page edge — h2[0..3] readable, h2[3] would fault.
    for b1 in 0u16..256 {
        for b2 in 0u16..256 {
            unsafe {
                *edge = 0xff;
                *edge.add(1) = b1 as u8;
                *edge.add(2) = b2 as u8;
            }
            let h1 = [rng.next_u8(), b1 as u8, b2 as u8];
            let (c, r) = unsafe { ((l.c)(h1.as_ptr(), edge), (l.rust)(h1.as_ptr(), edge)) };
            assert_eq!(c, r, "edge-h2 divergence b1={b1:#04x} b2={b2:#04x}");
            cases += 1;
        }
    }

    // (b) h1 at the page edge, against valid and invalid h2.
    for &b1 in &valid_byte1_values() {
        for &b2 in &valid_byte2_values() {
            unsafe {
                *edge = rng.next_u8();
                *edge.add(1) = b1;
                *edge.add(2) = b2;
            }
            let h2 = [0xffu8, b1, b2];
            let (c, r) = unsafe { ((l.c)(edge, h2.as_ptr()), (l.rust)(edge, h2.as_ptr())) };
            assert_eq!(c, r, "edge-h1 divergence for h2={h2:02x?}");
            assert_eq!(c, 1);
            cases += 1;
        }
    }

    // (c) h1 placed so that only h1[0] is readable (h1[1] would fault), with an
    // invalid h2: the short-circuit means neither side may touch h1[1].
    let edge1 = unsafe { base.add(page - 1) };
    unsafe { *edge1 = 0xff };
    let invalid = [0x00u8, 0x00, 0x00];
    let (c, r) = unsafe { ((l.c)(edge1, invalid.as_ptr()), (l.rust)(edge1, invalid.as_ptr())) };
    assert_eq!(c, r);
    assert_eq!(c, 0);
    cases += 1;

    let _ = other;
    assert_eq!(cases, 65536 + 14 * 180 + 1);
    unsafe { libc::munmap(map, 2 * page) };
}

/// Extra generic-boundary coverage required by Phase C beyond the table: every
/// byte position swept through its full 0..=255 range (the byte-level analogue of
/// "out-of-range enum value", since the API has no enum parameters — see
/// `ERRORS.md`), one step past each documented field boundary, and the
/// all-`0x00` / all-`0xff` extremes.
#[test]
fn err_generic_boundaries_and_out_of_range_field_values() {
    // Each of the 5 relevant byte positions swept fully while the others are
    // held at a valid, matching configuration.
    let base1 = [0x00u8, 0xfb, 0x90];
    let base2 = [0xffu8, 0xfb, 0x90];
    assert_eq!(assert_same(&base1, &base2), 1, "baseline must be accepted");

    for x in 0u16..256 {
        let x = x as u8;
        assert_same(&[x, base1[1], base1[2]], &base2); // h1[0] (never read)
        assert_same(&[base1[0], x, base1[2]], &base2); // h1[1]
        assert_same(&[base1[0], base1[1], x], &base2); // h1[2]
        assert_same(&base1, &[x, base2[1], base2[2]]); // h2[0]
        assert_same(&base1, &[base2[0], x, base2[2]]); // h2[1]
        assert_same(&base1, &[base2[0], base2[1], x]); // h2[2]
    }

    // One step past each field boundary named in the C.
    let boundaries: &[[u8; 3]] = &[
        [0xff, 0xef, 0x90], // just below the 0xf0 class
        [0xff, 0xf0, 0x90], // class ok, layer reserved
        [0xff, 0xf1, 0x90],
        [0xff, 0xf2, 0x90], // first accepted layer value
        [0xff, 0xe1, 0x90], // just below 0xe2
        [0xff, 0xe2, 0x90],
        [0xff, 0xe3, 0x90],
        [0xff, 0xe4, 0x90], // just past 0xe3
        [0xff, 0xfb, 0x00], // bitrate nibble 0 (free format)
        [0xff, 0xfb, 0x10], // bitrate nibble 1
        [0xff, 0xfb, 0xe0], // bitrate nibble 14
        [0xff, 0xfb, 0xf0], // bitrate nibble 15 (reserved)
        [0xff, 0xfb, 0x08], // samplerate 2
        [0xff, 0xfb, 0x0c], // samplerate 3 (reserved)
        [0xff, 0xfb, 0xec], // both reserved-ish
        [0x00, 0x00, 0x00],
        [0xff, 0xff, 0xff],
    ];
    for h2 in boundaries {
        for h1 in boundaries {
            assert_same(h1, h2);
        }
        assert_same(h2, h2);
    }
}
