// Translation of c_src/src/lib.c
//
// Original C signature:
//   char *bin2hex(char *hex, size_t hex_maxlen, const uint8_t *bin, size_t bin_len);
//
// This function converts a binary buffer to a NUL-terminated lowercase hex
// string written into `hex`. The C implementation aborts when:
//   - bin_len >= SIZE_MAX / 2, or
//   - hex_maxlen <= bin_len * 2 (i.e., not enough room for 2*bin_len hex chars
//     plus the trailing NUL byte).
//
// The encoding uses a branchless trick relying on C's unsigned-int wrap-around
// arithmetic. We faithfully reproduce that with `wrapping_*` operations on u32.

/// Convert `bin` to NUL-terminated hex, writing into `hex`. Returns the same
/// slice. Aborts the process on the same conditions as the C version.
pub fn bin2hex<'a>(hex: &'a mut [u8], bin: &[u8]) -> &'a mut [u8] {
    let bin_len = bin.len();
    let hex_maxlen = hex.len();

    // Match the C abort conditions exactly. The first compares against
    // SIZE_MAX / 2 (the literal in the C source is 2^64-1 even though
    // size_t may be 32-bit elsewhere; usize::MAX matches the platform).
    if bin_len >= usize::MAX / 2 || hex_maxlen <= bin_len.wrapping_mul(2) {
        std::process::abort();
    }

    let mut i: usize = 0;
    while i < bin_len {
        let c: u32 = (bin[i] & 0x0f) as u32;
        let b: u32 = (bin[i] >> 4) as u32;

        // (((c - 10U) >> 8) & ~38U) — unsigned wrap arithmetic.
        let c_adj: u32 = (c.wrapping_sub(10) >> 8) & !38u32;
        let b_adj: u32 = (b.wrapping_sub(10) >> 8) & !38u32;

        let lo: u32 = 87u32.wrapping_add(c).wrapping_add(c_adj) & 0xff;
        let hi: u32 = 87u32.wrapping_add(b).wrapping_add(b_adj) & 0xff;

        let mut x: u32 = (lo << 8) | hi;
        hex[i * 2] = x as u8;
        x >>= 8;
        hex[i * 2 + 1] = x as u8;
        i += 1;
    }
    hex[i * 2] = 0;
    hex
}
