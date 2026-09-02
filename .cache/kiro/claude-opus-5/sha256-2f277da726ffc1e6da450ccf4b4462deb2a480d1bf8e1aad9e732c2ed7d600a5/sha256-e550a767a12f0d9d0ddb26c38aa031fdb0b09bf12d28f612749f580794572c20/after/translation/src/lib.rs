//! Rust translation of `c_src/src/lib.c`.
//!
//! Public ABI (matches `nm -D` on the C shared library):
//!   * `hdr_bitrate`
//!
//! The C header declares no namespace-renaming macros, so the linker symbol is
//! identical to the source-level name.

use std::ffi::c_uint;

/// The C source declares the table as `static const uint8_t halfrate[2][3][15]`
/// and indexes it with values that can fall outside the declared bounds, so the
/// table is kept here as a flat 90-byte block and indexed with the same
/// arithmetic C uses (`i * 45 + j * 15 + k`). This preserves the original
/// row-crossing reads exactly instead of "fixing" them.
static HALFRATE: [u8; 90] = [
    // halfrate[0][0..3]
    0, 4, 8, 12, 16, 20, 24, 28, 32, 40, 48, 56, 64, 72, 80, //
    0, 4, 8, 12, 16, 20, 24, 28, 32, 40, 48, 56, 64, 72, 80, //
    0, 16, 24, 28, 32, 40, 48, 56, 64, 72, 80, 88, 96, 112, 128, //
    // halfrate[1][0..3]
    0, 16, 20, 24, 28, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160, //
    0, 16, 24, 28, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, //
    0, 16, 32, 48, 64, 80, 96, 112, 128, 144, 160, 176, 192, 208, 224,
];

/// `unsigned hdr_bitrate(const uint8_t *h)`
///
/// Reads only `h[1]` and `h[2]`, exactly like the C original:
///
/// ```c
/// return 2 * halfrate[!!((h[1]) & 0x8)][(((h[1]) >> 1) & 3) - 1][((h[2]) >> 4)];
/// ```
///
/// The middle index is `layer - 1`, which is `-1` for the reserved layer value
/// `0b00`, and the last index reaches 15 for the "bad" bitrate nibble. Both
/// escape the declared bounds of the innermost arrays; the original library
/// reads the neighbouring table bytes (or zero padding outside the table) in
/// those cases. The flat indexing below reproduces that, with reads that fall
/// outside the 90-byte table yielding 0 as they do in the C build.
///
/// # Safety
/// `h` must point to at least 3 readable bytes, as required by the C function.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hdr_bitrate(h: *const u8) -> c_uint {
    let h1 = unsafe { *h.add(1) };
    let h2 = unsafe { *h.add(2) };

    let i = isize::from(h1 & 0x8 != 0); // !!(h[1] & 0x8)
    let j = (isize::from((h1 >> 1) & 3)) - 1; // ((h[1] >> 1) & 3) - 1
    let k = isize::from(h2 >> 4); // h[2] >> 4

    let offset = i * 45 + j * 15 + k;

    let half = if offset >= 0 && (offset as usize) < HALFRATE.len() {
        HALFRATE[offset as usize]
    } else {
        0
    };

    2 * c_uint::from(half)
}
