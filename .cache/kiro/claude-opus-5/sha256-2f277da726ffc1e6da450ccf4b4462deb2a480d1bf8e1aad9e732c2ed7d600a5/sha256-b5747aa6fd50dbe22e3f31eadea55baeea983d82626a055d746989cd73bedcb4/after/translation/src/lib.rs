//! Translation of `c_src/src/lib.c` (MPEG audio frame header bitrate lookup).
//!
//! The C source is:
//!
//! ```c
//! unsigned hdr_bitrate(const uint8_t *h) {
//!     static const uint8_t halfrate[2][3][15] = { ... };
//!     return 2 *
//!            halfrate[!!((h[1]) & 0x8)][(((h[1]) >> 1) & 3) - 1][((h[2]) >> 4)];
//! }
//! ```
//!
//! Note the two out-of-bounds cases that are deliberately *not* fixed here,
//! because the task is to reproduce the C behaviour rather than correct it:
//!
//! * `(((h[1]) >> 1) & 3) - 1` yields `-1` when the layer bits are `00`
//!   (the "reserved" layer), indexing one whole 15-byte row *before* the
//!   selected `[2]` plane.
//! * `(h[2]) >> 4` yields `0..=15`, but the innermost dimension is only 15
//!   wide, so index `15` runs one element past the end of its row.
//!
//! Combining the extremes, the flat byte offset into the 90-byte table spans
//! `-15 ..= 90`. To keep those reads memory-safe (the C would read whatever
//! the linker happened to place next to the table, which is not reproducible
//! anyway) the table is embedded in a zero-padded buffer: 15 bytes of padding
//! in front and 1 byte behind. Every offset the C can compute therefore lands
//! inside the padded buffer, and all in-bounds offsets read exactly the same
//! values as the C table.

#![allow(clippy::identity_op)]

use std::ffi::{c_int, c_uint};

/// The table exactly as written in the C source.
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

/// Lowest flat offset the C indexing can produce (`0*45 + -1*15 + 0`).
const MIN_OFFSET: isize = -15;
/// Highest flat offset the C indexing can produce (`1*45 + 2*15 + 15`).
const MAX_OFFSET: isize = 90;

const PAD_BEFORE: usize = (-MIN_OFFSET) as usize;
const TABLE_LEN: usize = PAD_BEFORE + (MAX_OFFSET as usize) + 1;

/// `HALFRATE` flattened row-major, offset by [`PAD_BEFORE`] zero bytes.
const TABLE: [u8; TABLE_LEN] = build_table();

const fn build_table() -> [u8; TABLE_LEN] {
    let mut table = [0u8; TABLE_LEN];
    let mut a = 0;
    while a < 2 {
        let mut b = 0;
        while b < 3 {
            let mut c = 0;
            while c < 15 {
                table[PAD_BEFORE + a * 45 + b * 15 + c] = HALFRATE[a][b][c];
                c += 1;
            }
            b += 1;
        }
        a += 1;
    }
    table
}

/// # Safety
///
/// `h` must point to at least 3 readable bytes, matching the C contract
/// (the C dereferences `h[1]` and `h[2]`).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hdr_bitrate(h: *const u8) -> c_uint {
    // C promotes the uint8_t loads to int before the bit twiddling.
    let h1 = unsafe { *h.add(1) } as c_int;
    let h2 = unsafe { *h.add(2) } as c_int;

    // !!((h[1]) & 0x8)
    let plane = c_int::from(h1 & 0x8 != 0);
    // (((h[1]) >> 1) & 3) - 1   -- can be -1
    let row = ((h1 >> 1) & 3) - 1;
    // ((h[2]) >> 4)             -- can be 15
    let col = h2 >> 4;

    let offset = plane * 45 + row * 15 + col;
    let value = TABLE[(offset as isize + PAD_BEFORE as isize) as usize] as c_int;

    (2 * value) as c_uint
}
