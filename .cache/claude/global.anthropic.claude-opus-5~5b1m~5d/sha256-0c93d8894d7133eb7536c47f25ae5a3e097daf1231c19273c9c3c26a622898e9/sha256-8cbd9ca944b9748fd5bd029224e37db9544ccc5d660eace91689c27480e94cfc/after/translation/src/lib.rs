//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI (from `c_src/include/lib.h`):
//!     unsigned hdr_bitrate(const uint8_t *h);
//!
//! The C implementation indexes a `static const uint8_t halfrate[2][3][15]`
//! table with values that can fall *outside* the declared bounds of the inner
//! dimensions:
//!
//!   * the layer index `(((h[1]) >> 1) & 3) - 1` is `-1` when the two layer
//!     bits are `00` (the "reserved" layer value),
//!   * the bitrate index `h[2] >> 4` is `15` when the bitrate field is the
//!     "bad" value `1111`.
//!
//! The C code does not validate either field, so those cases perform reads
//! past the row (or, for `-1` combined with the first plane, before the whole
//! object). This is *not* fixed here: the behaviour of the compiled C library
//! is reproduced exactly by laying the table out as one flat, contiguous
//! buffer surrounded by zero padding, exactly matching what the C object does:
//!
//!   * a row-relative index of `15` lands on element `0` of the following row,
//!     which is always `0` in this table (and, for the very last row, on the
//!     zero padding that follows the object in `.rodata`),
//!   * a flat offset of `-15 ..= -1` (plane `0`, layer index `-1`) lands on the
//!     zero bytes that precede the table in the mapped image.
//!
//! Both therefore yield `0`, i.e. a returned bitrate of `0`.

use std::ffi::{c_uchar, c_uint};

/// `static const uint8_t halfrate[2][3][15]` from `hdr_bitrate`.
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

/// Elements per innermost row of `halfrate`.
const ROW: usize = 15;
/// Elements per plane of `halfrate` (`3 * ROW`).
const PLANE: usize = 3 * ROW;
/// Total number of bytes occupied by `halfrate` (`2 * PLANE`).
const TABLE_BYTES: usize = 2 * PLANE;

/// Zero bytes modelled *before* the table (reached when the layer index is
/// `-1` while the plane index is `0`; the smallest reachable flat offset is
/// `-15`).
const PAD_LO: usize = ROW;
/// Zero bytes modelled *after* the table (the largest reachable flat offset is
/// `TABLE_BYTES`, i.e. one byte past the end).
const PAD_HI: usize = ROW + 1;

/// Flat, zero-padded image of `halfrate`.
const TABLE_LEN: usize = PAD_LO + TABLE_BYTES + PAD_HI;

const fn flatten_halfrate() -> [u8; TABLE_LEN] {
    let mut table = [0u8; TABLE_LEN];
    let mut plane = 0;
    while plane < 2 {
        let mut layer = 0;
        while layer < 3 {
            let mut rate = 0;
            while rate < ROW {
                table[PAD_LO + plane * PLANE + layer * ROW + rate] = HALFRATE[plane][layer][rate];
                rate += 1;
            }
            layer += 1;
        }
        plane += 1;
    }
    table
}

static HALFRATE_FLAT: [u8; TABLE_LEN] = flatten_halfrate();

/// `unsigned hdr_bitrate(const uint8_t *h)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hdr_bitrate(h: *const c_uchar) -> c_uint {
    // The C code only ever reads h[1] and h[2].
    let h1 = unsafe { *h.add(1) } as u32;
    let h2 = unsafe { *h.add(2) } as u32;

    // !!((h[1]) & 0x8)
    let plane = ((h1 & 0x8) != 0) as isize;
    // (((h[1]) >> 1) & 3) - 1   -- may be -1
    let layer = (((h1 >> 1) & 3) as i32 - 1) as isize;
    // ((h[2]) >> 4)             -- may be 15
    let rate = (h2 >> 4) as isize;

    let offset = plane * PLANE as isize + layer * ROW as isize + rate;
    debug_assert!(offset >= -(PAD_LO as isize) && offset < (TABLE_BYTES + PAD_HI) as isize);
    let index = (PAD_LO as isize + offset) as usize;

    2 * HALFRATE_FLAT[index] as c_uint
}
