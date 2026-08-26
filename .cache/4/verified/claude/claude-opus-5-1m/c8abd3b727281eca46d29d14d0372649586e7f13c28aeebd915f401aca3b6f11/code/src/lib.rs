//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI (matches `nm -D` on the C shared object):
//!   * `hdr_bitrate`
//!
//! Translated from `c_src/src/lib.c` / `c_src/include/lib.h`.

use std::ffi::{c_uchar, c_uint};

/// Flat image of the C function's `static const uint8_t halfrate[2][3][15]`
/// table, surrounded by zero padding.
///
/// The C code indexes the table as
/// `halfrate[i][j][k]` with
/// `i = !!(h[1] & 0x8)`      in `0..=1`,
/// `j = ((h[1] >> 1) & 3) - 1` in `-1..=2`  (note: `-1` when the field is 0), and
/// `k = h[2] >> 4`            in `0..=15` (note: `15` is one past the last row entry).
///
/// Both `j == -1` and `k == 15` make the C code read *outside* the 90-byte
/// table (flat byte offsets `-15..=90` instead of `0..=89`). In the compiled C
/// shared object the table is emitted at the very start of `.rodata`, so those
/// out-of-range reads land on zero padding (the tail of the preceding
/// executable segment page, and the alignment padding in front of
/// `.eh_frame_hdr`, respectively) and therefore yield `0`.
///
/// This bug is *not* fixed here: the table is stored with 15 bytes of zero
/// padding in front and 15 bytes of zero padding behind, so the flat offset
/// arithmetic reproduces the C library's observable results (including `0` for
/// every out-of-range index) without any out-of-bounds access in Rust.
const PAD: usize = 15;

static HALFRATE_PADDED: [u8; PAD + 90 + PAD] = {
    let halfrate: [[[u8; 15]; 3]; 2] = [
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

    let mut flat = [0u8; PAD + 90 + PAD];
    let mut i = 0;
    while i < 2 {
        let mut j = 0;
        while j < 3 {
            let mut k = 0;
            while k < 15 {
                flat[PAD + i * 45 + j * 15 + k] = halfrate[i][j][k];
                k += 1;
            }
            j += 1;
        }
        i += 1;
    }
    flat
};

/// `unsigned hdr_bitrate(const uint8_t *h);`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hdr_bitrate(h: *const c_uchar) -> c_uint {
    // h[1] and h[2] are read exactly as the C code does.
    let h1 = unsafe { *h.add(1) };
    let h2 = unsafe { *h.add(2) };

    // i = !!((h[1]) & 0x8)
    let i: isize = if (h1 & 0x8) != 0 { 1 } else { 0 };
    // j = (((h[1]) >> 1) & 3) - 1   (may be -1, as in the C original)
    let j: isize = (((h1 >> 1) & 3) as isize) - 1;
    // k = ((h[2]) >> 4)             (may be 15, one past the end of a row)
    let k: isize = (h2 >> 4) as isize;

    let offset = PAD as isize + i * 45 + j * 15 + k;
    let value = HALFRATE_PADDED[offset as usize];

    2u32.wrapping_mul(value as c_uint)
}
