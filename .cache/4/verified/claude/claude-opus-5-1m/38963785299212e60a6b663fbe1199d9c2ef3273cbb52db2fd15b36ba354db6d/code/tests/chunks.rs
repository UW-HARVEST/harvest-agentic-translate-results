//! Phase B/C extra — chunk-length pointer arithmetic.  CONFIGS.md rows 71-76.
//!
//! `cp_chunk` and `cp_find` advance the cursor differently, and the difference
//! is a signedness difference that a translation can easily get wrong:
//!
//! ```c
//! static const uint8_t *cp_chunk(cp_raw_png_t *png, const char *chunk, uint32_t minlen) {
//!   uint32_t len = cp_make32(png->p);
//!   ...
//!   int offset = len + 12;                      /* uint32 -> int : may go NEGATIVE */
//!   if (png->p + offset <= png->end) {          /* sign extended */
//!     png->p += offset;                         /* may move BACKWARDS */
//! ...
//! static const uint8_t *cp_find(cp_raw_png_t *png, const char *chunk, uint32_t minlen) {
//!   while (png->p < png->end) {
//!     uint32_t len = cp_make32(png->p);
//!     ...
//!     png->p += len + 12;                       /* uint32, ZERO extended */
//! ```
//!
//! So a declared chunk length `>= 0x7FFFFFF4` makes `cp_chunk` walk *backwards*
//! (and the IDAT collection loop is `cp_find` once, then `cp_chunk` repeatedly —
//! it can therefore spin), while `cp_find` always walks forwards.
//!
//! Everything here is compared with the paired fork driver, because these inputs
//! can make the C library loop forever, read wildly out of bounds, or
//! `memcpy` into `NULL`.

mod common;

use common::*;

/// Interesting declared chunk lengths around the `int` / `uint32_t` boundaries.
const LENS: [u32; 16] = [
    0,
    1,
    12,
    13,
    0x0000_FFFF,
    0x7FFF_FFF3, // offset = 0x7FFFFFFF (still positive)
    0x7FFF_FFF4, // offset = 0x80000000 -> INT_MIN, first negative offset
    0x7FFF_FFF5,
    0x7FFF_FFFF,
    0x8000_0000,
    0xC000_0000,
    0xFFFF_FFF3, // len + 12 wraps to 0xFFFFFFFF -> offset == -1
    0xFFFF_FFF4, // len + 12 wraps to 0           -> offset ==  0  (cp_chunk stalls)
    0xFFFF_FFF5, // len + 12 wraps to 1
    0xFFFF_FFF8, // len + 12 wraps to 4
    0xFFFF_FFFF,
];

/// Row 71 — `cp_chunk`'s signed `int offset = len + 12` on the **IHDR** chunk
/// (the only `cp_chunk` call that is not part of the IDAT loop).
#[test]
fn row_71_ihdr_declared_length() {
    let z = Spec::new(4, 4, 6).zlib_stream();
    for len in LENS {
        let mut c = Chunk::new(b"IHDR", ihdr(4, 4, 8, 6, 0, 0, 0));
        c.len_override = Some(len);
        let png = build_png(
            &PNG_SIG,
            &[
                c,
                Chunk::new(b"IDAT", z.clone()),
                Chunk::new(b"IEND", vec![]),
            ],
        );
        for plen in [png.len() as i32, png.len() as i32 - 1, 33, 0, -1] {
            diff_png_abort(&png, plen, &format!("ihdr len={len:#010X} png_length={plen}"));
        }
    }
}

/// Row 72 — the same declared lengths on a **PLTE** chunk, i.e. through
/// `cp_find`'s unsigned `png->p += len + 12`.
#[test]
fn row_72_plte_declared_length() {
    for len in LENS {
        for ct in [3u8, 6] {
            let mut s = Spec::new(4, 4, ct);
            let mut plte = Chunk::new(b"PLTE", (0..256 * 3).map(|i| i as u8).collect());
            plte.len_override = Some(len);
            s.plte = None;
            let z = s.zlib_stream();
            let png = build_png(
                &PNG_SIG,
                &[
                    Chunk::new(b"IHDR", ihdr(4, 4, 8, ct, 0, 0, 0)),
                    plte,
                    Chunk::new(b"IDAT", z),
                    Chunk::new(b"IEND", vec![]),
                ],
            );
            diff_png_abort(
                &png,
                png.len() as i32,
                &format!("plte len={len:#010X} ct={ct}"),
            );
        }
    }
}

/// Row 73 — the same on a **tRNS** chunk (also `cp_find`), and with the tRNS
/// length lying about the chunk size so `trns[index]` reads past it.
#[test]
fn row_73_trns_declared_length() {
    for len in LENS {
        let mut s = Spec::new(4, 4, 3);
        let mut trns = Chunk::new(b"tRNS", (0..16).map(|i| 0x10 + i as u8).collect());
        trns.len_override = Some(len);
        s.trns = None;
        let z = s.zlib_stream();
        let png = build_png(
            &PNG_SIG,
            &[
                Chunk::new(b"IHDR", ihdr(4, 4, 8, 3, 0, 0, 0)),
                Chunk::new(b"PLTE", (0..256 * 3).map(|i| i as u8).collect()),
                trns,
                Chunk::new(b"IDAT", z),
                Chunk::new(b"IEND", vec![]),
            ],
        );
        diff_png_abort(&png, png.len() as i32, &format!("trns len={len:#010X}"));
    }
}

/// Row 74 — the IDAT collection loop: `cp_find` once, then `cp_chunk`
/// repeatedly.  A declared length that makes `cp_chunk`'s offset zero or
/// negative can make the loop revisit the same chunk (or walk backwards), so the
/// `datalen` accumulation can loop, overflow `int`, or make `malloc` fail while
/// the *copy* loop still runs — `memcpy(NULL + offset, ...)`.
#[test]
fn row_74_idat_declared_length() {
    let z = Spec::new(4, 4, 6).zlib_stream();
    for len in LENS {
        for nidat in [1usize, 2, 3] {
            let mut chunks = vec![Chunk::new(b"IHDR", ihdr(4, 4, 8, 6, 0, 0, 0))];
            for i in 0..nidat {
                let mut c = Chunk::new(b"IDAT", z.clone());
                if i == nidat - 1 {
                    c.len_override = Some(len);
                }
                chunks.push(c);
            }
            chunks.push(Chunk::new(b"IEND", vec![]));
            let png = build_png(&PNG_SIG, &chunks);
            // Measured outcome classes for this row (C and Rust agree on all of
            // them):
            //   len = 0x0000000C, n = 1          -> SIGABRT (a live assert())
            //   len >= 0x7FFFFFF4, n >= 2        -> SIGSEGV (cp_chunk's `int`
            //                                       offset goes negative, so the
            //                                       cursor walks ~2 GiB backwards)
            //   len = 0xFFFFFFF4                 -> SIGALRM: `len + 12` wraps to
            //                                       0, so cp_chunk returns the
            //                                       same chunk forever and the
            //                                       IDAT loop *never terminates*
            //   everything else                  -> a normal rejection
            diff_png_abort(
                &png,
                png.len() as i32,
                &format!("idat len={len:#010X} n={nidat}"),
            );
        }
    }
}

/// Row 75 — many IDAT chunks with large declared lengths so that
/// `datalen += len` overflows `int`: `malloc(datalen)` then either returns NULL
/// (and the copy loop still runs) or allocates less than the copy loop writes.
#[test]
fn row_75_datalen_overflow() {
    let z = Spec::new(4, 4, 6).zlib_stream();
    for (n, len) in [
        (2usize, 0x4000_0000u32),
        (2, 0x7FFF_FFFF),
        (3, 0x3000_0000),
        (4, 0x2000_0000),
        (2, 0x8000_0000),
        (8, 0x1000_0000),
    ] {
        let mut chunks = vec![Chunk::new(b"IHDR", ihdr(4, 4, 8, 6, 0, 0, 0))];
        for _ in 0..n {
            let mut c = Chunk::new(b"IDAT", z.clone());
            c.len_override = Some(len);
            chunks.push(c);
        }
        chunks.push(Chunk::new(b"IEND", vec![]));
        let png = build_png(&PNG_SIG, &chunks);
        diff_png_abort(
            &png,
            png.len() as i32,
            &format!("datalen overflow n={n} len={len:#010X}"),
        );
    }
}

/// Row 76 — randomised chunk tables: random names, random declared lengths,
/// random ordering, random `png_length`.  Pure differential (paired forks).
#[test]
fn row_76_random_chunk_tables() {
    let mut rng = Rng::new(0x7601);
    let names: [&[u8; 4]; 8] = [
        b"IHDR", b"PLTE", b"tRNS", b"IDAT", b"IEND", b"gAMA", b"idat", b"\0\0\0\0",
    ];
    for iter in 0..250 {
        let ct = [0u8, 2, 3, 4, 6][rng.below(5) as usize];
        let s = Spec::new(4, 4, ct);
        let z = s.zlib_stream();
        let nchunks = rng.range(1, 6) as usize;
        let mut chunks = vec![Chunk::new(b"IHDR", ihdr(4, 4, 8, ct, 0, 0, 0))];
        for _ in 0..nchunks {
            let name = names[rng.below(8) as usize];
            let body = match rng.below(3) {
                0 => z.clone(),
                1 => {
                    let bn = rng.below(40) as usize;
                    rng.bytes(bn)
                }
                _ => Vec::new(),
            };
            let mut c = Chunk::new(name, body);
            if rng.below(3) == 0 {
                c.len_override = Some(if rng.below(2) == 0 {
                    LENS[rng.below(16) as usize]
                } else {
                    rng.u32()
                });
            }
            if rng.below(4) == 0 {
                c.crc_override = Some(rng.u32());
            }
            chunks.push(c);
        }
        let png = build_png(&PNG_SIG, &chunks);
        let plen = match rng.below(4) {
            0 => png.len() as i32,
            1 => rng.below(png.len() as u32 + 1) as i32,
            2 => png.len() as i32 + rng.below(128) as i32,
            _ => -(rng.below(32) as i32),
        };
        diff_png_abort(&png, plen, &format!("random chunks iter={iter} plen={plen}"));
    }
}
