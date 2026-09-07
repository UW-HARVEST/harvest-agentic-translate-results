//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Both implementations are reached only through their `.so` exports.

mod common;

use common::{assert_same, assert_same_ptr, libs, make_h1, make_h2, Rng, ROWS};

/// Number of randomized repetitions per (row, nibble) cell.
const REPS: usize = 64;

/// Rows 1..8: every `(plane, layer)` selector x all 16 bitrate nibbles, with
/// randomized ignored bits and randomized surrounding buffer bytes.
fn sweep_row(plane: u8, raw_layer: u8, seed: u64) {
    let mut rng = Rng::new(seed);
    for nibble in 0u8..16 {
        for _ in 0..REPS {
            let h1 = make_h1(plane, raw_layer, &mut rng);
            let h2 = make_h2(nibble, &mut rng);
            // random junk in the bytes the C never reads
            let mut buf = [0u8; 8];
            for b in buf.iter_mut() {
                *b = rng.next_u8();
            }
            buf[1] = h1;
            buf[2] = h2;
            assert_same(
                &buf,
                &format!("plane={plane} raw_layer={raw_layer} nibble={nibble}"),
            );
        }
    }
}

#[test]
fn cfg01_plane0_layer_reserved() {
    sweep_row(0, 0, 0x1111_0001);
}

#[test]
fn cfg02_plane0_layer3() {
    sweep_row(0, 1, 0x1111_0002);
}

#[test]
fn cfg03_plane0_layer2() {
    sweep_row(0, 2, 0x1111_0003);
}

#[test]
fn cfg04_plane0_layer1() {
    sweep_row(0, 3, 0x1111_0004);
}

#[test]
fn cfg05_plane1_layer_reserved() {
    sweep_row(1, 0, 0x1111_0005);
}

#[test]
fn cfg06_plane1_layer3() {
    sweep_row(1, 1, 0x1111_0006);
}

#[test]
fn cfg07_plane1_layer2() {
    sweep_row(1, 2, 0x1111_0007);
}

#[test]
fn cfg08_plane1_layer1() {
    sweep_row(1, 3, 0x1111_0008);
}

/// Row 9: boundary bitrate nibbles crossed with all 8 row selectors.
#[test]
fn cfg09_boundary_nibbles_all_rows() {
    let mut rng = Rng::new(0x1111_0009);
    for &(plane, raw_layer) in ROWS.iter() {
        for &nibble in &[0u8, 1, 13, 14, 15] {
            for _ in 0..REPS {
                let buf = [rng.next_u8(), make_h1(plane, raw_layer, &mut rng), make_h2(nibble, &mut rng)];
                assert_same(&buf, &format!("boundary p={plane} l={raw_layer} n={nibble}"));
            }
        }
    }
}

/// Row 10: the ignored bits (`h[0]`, `h[1]` bits 0 & 4..7, `h[2]` low nibble)
/// must not change the answer, and each variant must still match C.
#[test]
fn cfg10_ignored_bits_invariant() {
    let mut rng = Rng::new(0x1111_000A);
    for _ in 0..4096 {
        let h1 = rng.next_u8();
        let h2 = rng.next_u8();

        let significant1 = h1 & 0x0E; // plane bit + layer field
        let significant2 = h2 & 0xF0; // bitrate nibble

        let cleared = [0u8, significant1, significant2];
        let all_set = [0xFFu8, significant1 | 0xF1, significant2 | 0x0F];
        let random = [rng.next_u8(), h1, h2];

        let a = assert_same(&cleared, "ignored bits cleared");
        let b = assert_same(&all_set, "ignored bits set");
        let c = assert_same(&random, "ignored bits random");
        assert_eq!(a, b, "ignored bits changed result: {cleared:02x?} vs {all_set:02x?}");
        assert_eq!(a, c, "ignored bits changed result: {cleared:02x?} vs {random:02x?}");
    }
}

/// Row 11: buffer of exactly the 3 bytes the C touches.
#[test]
fn cfg11_exact_three_byte_buffer() {
    let mut rng = Rng::new(0x1111_000B);
    for _ in 0..8192 {
        let buf = vec![rng.next_u8(), rng.next_u8(), rng.next_u8()];
        assert_same(&buf, "exact 3-byte buffer");
    }
}

/// Row 12: header at a non-zero offset inside a larger buffer, the way a stream
/// walker calls it.
#[test]
fn cfg12_header_at_offset_in_stream() {
    let mut rng = Rng::new(0x1111_000C);
    let mut stream = vec![0u8; 4096];
    for b in stream.iter_mut() {
        *b = rng.next_u8();
    }
    for _ in 0..8192 {
        let off = rng.below((stream.len() - 3) as u64) as usize;
        assert_same(&stream[off..off + 3], &format!("stream offset {off}"));
    }
}

/// Row 13: deliberately misaligned (odd-address) header.
#[test]
fn cfg13_misaligned_pointer() {
    let mut rng = Rng::new(0x1111_000D);
    let mut backing = vec![0u8; 64];
    for _ in 0..8192 {
        for b in backing.iter_mut() {
            *b = rng.next_u8();
        }
        // Find an odd address inside the allocation.
        let base = backing.as_ptr() as usize;
        let start = if base % 2 == 0 { 1 } else { 0 };
        for extra in [0usize, 2, 4, 6] {
            let off = start + extra;
            let p = unsafe { backing.as_ptr().add(off) };
            assert_eq!((p as usize) % 2, 1, "expected odd address");
            assert_same_ptr(p, &format!("misaligned offset {off}"));
        }
    }
}

/// Rows 14 & 15: guard pages proving neither implementation reads outside
/// `h[1..=2]`.
mod guard {
    use super::*;

    struct Mapping {
        base: *mut u8,
        len: usize,
    }

    impl Drop for Mapping {
        fn drop(&mut self) {
            unsafe {
                libc::munmap(self.base as *mut libc::c_void, self.len);
            }
        }
    }

    fn page_size() -> usize {
        unsafe { libc::sysconf(libc::_SC_PAGESIZE) as usize }
    }

    /// Map two pages RW, then flip one of them to `PROT_NONE`.
    fn two_pages(none_page: usize) -> Mapping {
        let ps = page_size();
        let len = 2 * ps;
        let base = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                len,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
                -1,
                0,
            )
        };
        assert_ne!(base, libc::MAP_FAILED, "mmap failed");
        let base = base as *mut u8;
        let m = Mapping { base, len };
        let rc = unsafe {
            libc::mprotect(
                base.add(none_page * ps) as *mut libc::c_void,
                ps,
                libc::PROT_NONE,
            )
        };
        assert_eq!(rc, 0, "mprotect failed");
        m
    }

    /// Row 14: `h[0..=2]` occupy the last 3 bytes of a readable page; the next
    /// page is `PROT_NONE`, so any read of `h[3]` or beyond segfaults.
    #[test]
    fn cfg14_no_read_past_h2() {
        let ps = page_size();
        let m = two_pages(1); // second page is PROT_NONE
        let mut rng = Rng::new(0x1111_000E);
        for _ in 0..4096 {
            let p = unsafe { m.base.add(ps - 3) };
            unsafe {
                *p = rng.next_u8();
                *p.add(1) = rng.next_u8();
                *p.add(2) = rng.next_u8();
            }
            assert_same_ptr(p, "header abutting a PROT_NONE page");
        }
    }

    /// Row 15: `h[0]` is the last byte of a `PROT_NONE` page and `h[1..=2]` are
    /// the first bytes of the readable page, so any read of `h[0]` segfaults.
    #[test]
    fn cfg15_h0_never_read() {
        let ps = page_size();
        let m = two_pages(0); // first page is PROT_NONE
        let mut rng = Rng::new(0x1111_000F);
        for _ in 0..4096 {
            // h points one byte before the readable page.
            let p = unsafe { m.base.add(ps - 1) };
            unsafe {
                *p.add(1) = rng.next_u8();
                *p.add(2) = rng.next_u8();
            }
            assert_same_ptr(p, "h[0] inside a PROT_NONE page");
        }
    }
}

/// Row 16: exhaustive sweep of every `(h[1], h[2])` byte pair — the complete
/// cross-product of every axis the C branches on.
#[test]
fn cfg16_exhaustive_all_65536() {
    let l = libs();
    let mut diffs: Vec<String> = Vec::new();
    for a in 0u16..256 {
        for b in 0u16..256 {
            let buf = [0xAAu8, a as u8, b as u8];
            let (c, r) = unsafe { ((l.c)(buf.as_ptr()), (l.rust)(buf.as_ptr())) };
            if c != r {
                if diffs.len() < 20 {
                    diffs.push(format!("h1={a:#04x} h2={b:#04x}: C={c} Rust={r}"));
                }
            }
        }
    }
    assert!(
        diffs.is_empty(),
        "exhaustive sweep diverged ({} shown):\n{}",
        diffs.len(),
        diffs.join("\n")
    );
}

/// Row 17: interleaved / repeated calls — no hidden state, no lazy-init ordering
/// dependence, results are idempotent.
#[test]
fn cfg17_no_hidden_state() {
    let l = libs();
    let mut rng = Rng::new(0x1111_0011);
    let mut buf = [0u8; 3];
    for _ in 0..8192 {
        for b in buf.iter_mut() {
            *b = rng.next_u8();
        }
        unsafe {
            // Rust first this time, then C, then both again.
            let r1 = (l.rust)(buf.as_ptr());
            let c1 = (l.c)(buf.as_ptr());
            let c2 = (l.c)(buf.as_ptr());
            let r2 = (l.rust)(buf.as_ptr());
            assert_eq!(r1, r2, "Rust not idempotent for {buf:02x?}");
            assert_eq!(c1, c2, "C not idempotent for {buf:02x?}");
            assert_eq!(c1, r1, "divergence (interleaved) for {buf:02x?}");
        }
    }
}

/// Cross-check the whole 128-entry decode against an independent reference
/// derived by hand from the C table, so a *shared* bug in both loaders would
/// still be caught.
#[test]
fn cfg_reference_table_crosscheck() {
    // halfrate[2][3][15] verbatim from c_src/src/lib.c
    const HALFRATE: [[[u8; 15]; 3]; 2] = [
        [
            [0, 4, 8, 12, 16, 20, 24, 28, 32, 40, 48, 56, 64, 72, 80],
            [0, 4, 8, 12, 16, 20, 24, 28, 32, 40, 48, 56, 64, 72, 80],
            [0, 16, 24, 28, 32, 40, 48, 56, 64, 72, 80, 88, 96, 112, 128],
        ],
        [
            [0, 16, 20, 24, 28, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160],
            [0, 16, 24, 28, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192],
            [0, 16, 32, 48, 64, 80, 96, 112, 128, 144, 160, 176, 192, 208, 224],
        ],
    ];
    // Flat image with zero padding, matching the observed C behaviour for the
    // out-of-bounds indices (empirically confirmed against the built .so).
    let mut flat = [0u8; 15 + 90 + 16];
    for p in 0..2 {
        for l in 0..3 {
            for r in 0..15 {
                flat[15 + p * 45 + l * 15 + r] = HALFRATE[p][l][r];
            }
        }
    }

    for h1 in 0u16..256 {
        for h2 in 0u16..256 {
            let plane = ((h1 & 0x8) != 0) as isize;
            let layer = (((h1 >> 1) & 3) as isize) - 1;
            let rate = (h2 >> 4) as isize;
            let off = plane * 45 + layer * 15 + rate;
            let expect = 2 * flat[(15 + off) as usize] as u32;
            let buf = [0x5Au8, h1 as u8, h2 as u8];
            let got = assert_same(&buf, &format!("reference h1={h1:#04x} h2={h2:#04x}"));
            assert_eq!(
                got, expect,
                "both libs disagree with the hand-derived reference for h1={h1:#04x} h2={h2:#04x}"
            );
        }
    }
}
