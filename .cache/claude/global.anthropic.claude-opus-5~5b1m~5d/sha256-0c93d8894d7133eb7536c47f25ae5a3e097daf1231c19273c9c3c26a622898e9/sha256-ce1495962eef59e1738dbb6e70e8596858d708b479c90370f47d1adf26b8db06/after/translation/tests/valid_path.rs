//! Phase B -- valid-path differential tests, gated on `CONFIGS.md`.
//!
//! Every row of `CONFIGS.md` is driven through BOTH shared objects via
//! `libloading` and the results compared byte-for-byte. Randomized rows use a
//! fixed-seed PRNG so failures reproduce.

mod common;

use common::*;

/// Reference index arithmetic, used only to *report* which table offset a
/// failing configuration selected (never as the oracle -- the oracle is the C
/// `.so`).
fn flat_offset(plane: u32, layer_bits: u32, rate: u32) -> i32 {
    plane as i32 * 45 + (layer_bits as i32 - 1) * 15 + rate as i32
}

// ---------------------------------------------------------------------------
// Rows 1-128: the full cross product plane x layer_bits x rate.
// ---------------------------------------------------------------------------

#[test]
fn rows_1_128_full_cross_product_randomized() {
    let p = load_pair();
    let mut rng = Rng::new(SEED);

    let mut row = 0usize;
    for plane in PLANES {
        for layer_bits in LAYER_BITS {
            for rate in RATE_NIBBLES {
                row += 1;
                let off = flat_offset(plane, layer_bits, rate);

                // Many randomized inputs per row: the don't-care bits of
                // h[1]/h[2], h[0], and the trailing bytes all vary.
                let mut seen: Option<u32> = None;
                for it in 0..ITERS {
                    let len = 3 + rng.below(29); // 3..=31 byte buffers
                    let mut buf = vec![0u8; len];
                    rng.fill(&mut buf);
                    buf[1] = make_h1(plane, layer_bits, rng.next_u8());
                    buf[2] = make_h2(rate, rng.next_u8());

                    let got = p.assert_same(
                        &buf,
                        &format!(
                            "CONFIGS row {row} (plane={plane} layer_bits={layer_bits} \
                             rate={rate} flat_offset={off}) iter {it}"
                        ),
                    );

                    // Same row => same value regardless of the ignored bits.
                    match seen {
                        None => seen = Some(got),
                        Some(prev) => assert_eq!(
                            prev, got,
                            "row {row}: result changed with ignored bits \
                             ({prev} vs {got}), h[1]={:#04x} h[2]={:#04x}",
                            buf[1], buf[2]
                        ),
                    }
                }
            }
        }
    }
    assert_eq!(row, 128, "must cover all 128 cross-product rows");
}

// ---------------------------------------------------------------------------
// Row 129: minimum sufficient buffer (exactly 3 bytes).
// ---------------------------------------------------------------------------

#[test]
fn row_129_minimum_three_byte_buffer() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 129);

    for plane in PLANES {
        for layer_bits in LAYER_BITS {
            for rate in RATE_NIBBLES {
                for _ in 0..32 {
                    let buf: Vec<u8> = vec![
                        rng.next_u8(),
                        make_h1(plane, layer_bits, rng.next_u8()),
                        make_h2(rate, rng.next_u8()),
                    ];
                    assert_eq!(buf.len(), 3);
                    p.assert_same(&buf, "CONFIGS row 129 (3-byte buffer)");
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 130: realistic 4-byte MP3 frame header.
// ---------------------------------------------------------------------------

#[test]
fn row_130_realistic_frame_header() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 130);

    // h[1] = 111 VV LL C  -> sync bits set, version, layer, CRC bit.
    for version in 0u8..4 {
        for layer in 0u8..4 {
            for crc in 0u8..2 {
                let h1 = 0xE0 | (version << 3) | (layer << 1) | crc;
                for rate in RATE_NIBBLES {
                    for _ in 0..16 {
                        let buf = vec![
                            0xFF,
                            h1,
                            make_h2(rate, rng.next_u8()),
                            rng.next_u8(), // h[3]: mode/emphasis, never read
                        ];
                        p.assert_same(
                            &buf,
                            &format!(
                                "CONFIGS row 130 (frame header version={version} \
                                 layer={layer} crc={crc} rate={rate})"
                            ),
                        );
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 131: large buffer, call at a random offset.
// ---------------------------------------------------------------------------

#[test]
fn row_131_large_buffer_random_offset() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 131);

    let mut buf = vec![0u8; 4096];
    rng.fill(&mut buf);

    for _ in 0..4000 {
        let off = rng.below(buf.len() - 3);
        let (a, b) = unsafe {
            let ptr = buf.as_ptr().add(off);
            (p.c.call(ptr), p.rust.call(ptr))
        };
        assert_eq!(
            a, b,
            "DIVERGENCE CONFIGS row 131 at offset {off}: C={a} Rust={b} \
             (h[1]={:#04x} h[2]={:#04x})",
            buf[off + 1],
            buf[off + 2]
        );
    }
}

// ---------------------------------------------------------------------------
// Row 132: every pointer alignment 0..=63.
// ---------------------------------------------------------------------------

#[test]
fn row_132_all_alignments() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 132);

    #[repr(align(64))]
    struct Aligned([u8; 256]);
    let mut a = Aligned([0u8; 256]);
    rng.fill(&mut a.0);

    for plane in PLANES {
        for layer_bits in LAYER_BITS {
            for rate in RATE_NIBBLES {
                for align in 0usize..64 {
                    a.0[align + 1] = make_h1(plane, layer_bits, rng.next_u8());
                    a.0[align + 2] = make_h2(rate, rng.next_u8());
                    let (x, y) = unsafe {
                        let ptr = a.0.as_ptr().add(align);
                        (p.c.call(ptr), p.rust.call(ptr))
                    };
                    assert_eq!(
                        x, y,
                        "DIVERGENCE CONFIGS row 132 align={align} \
                         (plane={plane} layer_bits={layer_bits} rate={rate})"
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 133/134: guard pages -- prove the read window is exactly h[1..=2].
// ---------------------------------------------------------------------------

/// Map `pages` readable+writable pages with an unmapped page on each side.
struct Guarded {
    base: *mut u8,
    total: usize,
    page: usize,
}

impl Guarded {
    fn new() -> Guarded {
        let page = unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as usize;
        let total = page * 3;
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
        // Only the middle page is accessible; pages 0 and 2 stay PROT_NONE.
        let mid = unsafe { (base as *mut u8).add(page) };
        let rc = unsafe { libc::mprotect(mid as *mut _, page, libc::PROT_READ | libc::PROT_WRITE) };
        assert_eq!(rc, 0, "mprotect failed");
        Guarded {
            base: base as *mut u8,
            total,
            page,
        }
    }
    /// Start of the single accessible page.
    fn usable(&self) -> *mut u8 {
        unsafe { self.base.add(self.page) }
    }
}

impl Drop for Guarded {
    fn drop(&mut self) {
        unsafe {
            libc::munmap(self.base as *mut _, self.total);
        }
    }
}

#[test]
fn row_133_h2_is_last_readable_byte() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 133);
    let g = Guarded::new();

    // Place the header so that h[2] is the final byte of the accessible page:
    // any read of h[3] or beyond faults.
    let h = unsafe { g.usable().add(g.page - 3) };

    for plane in PLANES {
        for layer_bits in LAYER_BITS {
            for rate in RATE_NIBBLES {
                unsafe {
                    *h = rng.next_u8();
                    *h.add(1) = make_h1(plane, layer_bits, rng.next_u8());
                    *h.add(2) = make_h2(rate, rng.next_u8());
                    p.assert_same_ptr(
                        h,
                        &format!(
                            "CONFIGS row 133 (h[2] at page end, plane={plane} \
                             layer_bits={layer_bits} rate={rate})"
                        ),
                    );
                }
            }
        }
    }
}

#[test]
fn row_134_h0_at_page_start() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 134);
    let g = Guarded::new();

    // h[0] is the first byte of the accessible page: any read before h[0]
    // faults.
    let h = g.usable();

    for plane in PLANES {
        for layer_bits in LAYER_BITS {
            for rate in RATE_NIBBLES {
                unsafe {
                    *h = rng.next_u8();
                    *h.add(1) = make_h1(plane, layer_bits, rng.next_u8());
                    *h.add(2) = make_h2(rate, rng.next_u8());
                    p.assert_same_ptr(
                        h,
                        &format!(
                            "CONFIGS row 134 (h[0] at page start, plane={plane} \
                             layer_bits={layer_bits} rate={rate})"
                        ),
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 135-137: bits/bytes the C never reads must not matter.
// ---------------------------------------------------------------------------

#[test]
fn row_135_ignored_bits_of_h1() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 135);

    for plane in PLANES {
        for layer_bits in LAYER_BITS {
            for rate in RATE_NIBBLES {
                let mut baseline: Option<u32> = None;
                // bit 0 and bits 4..7 = 2^5 = 32 combinations.
                for noise in 0u32..32 {
                    let bit0 = (noise & 1) as u8;
                    let hi = ((noise >> 1) as u8) << 4;
                    let h1 = ((plane as u8) << 3) | ((layer_bits as u8) << 1) | bit0 | hi;
                    let buf = vec![rng.next_u8(), h1, make_h2(rate, rng.next_u8())];
                    let got = p.assert_same(&buf, "CONFIGS row 135 (ignored h[1] bits)");
                    match baseline {
                        None => baseline = Some(got),
                        Some(b) => assert_eq!(
                            b, got,
                            "row 135: h[1] ignored bits changed the result \
                             ({b} -> {got}) at h[1]={h1:#04x}"
                        ),
                    }
                }
            }
        }
    }
}

#[test]
fn row_136_ignored_bits_of_h2() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 136);

    for plane in PLANES {
        for layer_bits in LAYER_BITS {
            for rate in RATE_NIBBLES {
                let mut baseline: Option<u32> = None;
                for low in 0u8..16 {
                    let h2 = ((rate as u8) << 4) | low;
                    let buf = vec![
                        rng.next_u8(),
                        make_h1(plane, layer_bits, rng.next_u8()),
                        h2,
                    ];
                    let got = p.assert_same(&buf, "CONFIGS row 136 (ignored h[2] bits)");
                    match baseline {
                        None => baseline = Some(got),
                        Some(b) => assert_eq!(
                            b, got,
                            "row 136: h[2] low nibble changed the result \
                             ({b} -> {got}) at h[2]={h2:#04x}"
                        ),
                    }
                }
            }
        }
    }
}

#[test]
fn row_137_h0_never_read() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 137);

    for plane in PLANES {
        for layer_bits in LAYER_BITS {
            for rate in RATE_NIBBLES {
                let h1 = make_h1(plane, layer_bits, rng.next_u8());
                let h2 = make_h2(rate, rng.next_u8());
                let mut baseline: Option<u32> = None;
                for h0 in 0u8..=255 {
                    let buf = vec![h0, h1, h2];
                    let got = p.assert_same(&buf, "CONFIGS row 137 (h[0] unread)");
                    match baseline {
                        None => baseline = Some(got),
                        Some(b) => assert_eq!(b, got, "row 137: h[0]={h0:#04x} changed the result"),
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 138: exhaustive over the whole (h[1], h[2]) input domain.
// ---------------------------------------------------------------------------

#[test]
fn row_138_exhaustive_h1_h2() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 138);

    let mut mismatches: Vec<(u8, u8, u32, u32)> = Vec::new();
    for h1 in 0u16..=255 {
        for h2 in 0u16..=255 {
            let mut buf = [0u8; 8];
            rng.fill(&mut buf);
            buf[1] = h1 as u8;
            buf[2] = h2 as u8;
            let (a, b) = unsafe { (p.c.call(buf.as_ptr()), p.rust.call(buf.as_ptr())) };
            if a != b {
                mismatches.push((h1 as u8, h2 as u8, a, b));
            }
        }
    }
    assert!(
        mismatches.is_empty(),
        "CONFIGS row 138: {} of 65536 (h[1],h[2]) pairs diverged; first 20: {:?}",
        mismatches.len(),
        &mismatches[..mismatches.len().min(20)]
    );
}

// ---------------------------------------------------------------------------
// Row 139: purity / no state across calls, interleaved between the two libs.
// ---------------------------------------------------------------------------

#[test]
fn row_139_pure_and_interleaved() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 139);

    // Record the value of every (h1, h2) once...
    let mut table = vec![0u32; 256 * 256];
    for h1 in 0usize..256 {
        for h2 in 0usize..256 {
            let buf = [0u8, h1 as u8, h2 as u8];
            table[h1 * 256 + h2] = unsafe { p.c.call(buf.as_ptr()) };
        }
    }

    // ...then hammer both libraries in random, interleaved order and check the
    // answers never drift.
    for _ in 0..20_000 {
        let h1 = rng.next_u8();
        let h2 = rng.next_u8();
        let buf = [rng.next_u8(), h1, h2];
        let expect = table[h1 as usize * 256 + h2 as usize];
        let (a, b) = unsafe { (p.rust.call(buf.as_ptr()), p.c.call(buf.as_ptr())) };
        assert_eq!(a, b, "row 139: C/Rust diverged at h1={h1:#04x} h2={h2:#04x}");
        assert_eq!(
            a, expect,
            "row 139: result drifted across calls at h1={h1:#04x} h2={h2:#04x}"
        );
    }
}

// ---------------------------------------------------------------------------
// Row 140: return width -- values above 255, and the zero case.
// ---------------------------------------------------------------------------

#[test]
fn row_140_return_value_width() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 140);

    // Read the return through a u64-returning signature: the C ABI leaves the
    // upper 32 bits of rax undefined for a 32-bit return, so only assert that
    // the two libraries agree on the 32-bit value, and that values > 255 are
    // not truncated anywhere.
    let mut max_seen = 0u32;
    let mut saw_zero = false;
    let mut saw_gt_255 = false;

    for plane in PLANES {
        for layer_bits in LAYER_BITS {
            for rate in RATE_NIBBLES {
                for _ in 0..8 {
                    let buf = vec![
                        rng.next_u8(),
                        make_h1(plane, layer_bits, rng.next_u8()),
                        make_h2(rate, rng.next_u8()),
                    ];
                    let v = p.assert_same(&buf, "CONFIGS row 140 (return width)");
                    max_seen = max_seen.max(v);
                    saw_zero |= v == 0;
                    saw_gt_255 |= v > 255;
                    // A u8-truncating bug would show up as v != 2*(v/2) parity
                    // or a wrapped value; the result is always even and <= 448.
                    assert_eq!(v % 2, 0, "result must be 2 * a table byte, got {v}");
                    assert!(v <= 448, "result must not exceed 2*224, got {v}");
                }
            }
        }
    }
    assert_eq!(max_seen, 448, "the 448 configuration must be reachable");
    assert!(saw_zero, "a zero result must be reachable");
    assert!(saw_gt_255, "a result above u8::MAX must be reachable");
}
