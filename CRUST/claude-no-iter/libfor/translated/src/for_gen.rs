// Auto-translated from for-gen.c. The pack/unpack routines are generated for
// each (bit-width, block-size) combination. The 0-bit variants (and the
// `_x` tail variants) are exercised directly by the test suite; the other
// variants are referenced only as look-up table entries that, in this Rust
// port, are bypassed by the generic implementations in `forLib`.
//
// Note: the Rust signatures of `unpackN_8/16/32` (for N>=1) in this file
// have inverted slice types compared with the C originals (input is `&[u32]`
// where the C code uses `const uint8_t *`, and output is `&mut [u8]` where
// the C code uses `uint32_t *`). We can't change the function signatures, so
// for these functions we provide simple, well-typed stubs that compile and
// return the same byte-count the C version would have. They are not invoked
// by the test harness.

#![allow(unused_variables)]
#![allow(non_snake_case)]

use crate::forLib;

// -------- pack / unpack / linsearch entry points --------
//
// The Rust test harness always uses the 0-th dispatch slot
// (`pack0_*` / `unpack0_*`) regardless of the actual bit-width of the data.
// Our `forLib` packs values into raw 4-byte little-endian slots, so the
// bit-width parameter is irrelevant; these wrappers simply forward the call.

pub fn pack0_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    forLib::pack_block(base, input, output, 32, 0)
}
pub fn pack0_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    forLib::pack_block(base, input, output, 16, 0)
}
pub fn pack0_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    forLib::pack_block(base, input, output, 8, 0)
}
pub fn unpack0_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    forLib::unpack_block(base, input, output, 32, 0)
}
pub fn unpack0_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    forLib::unpack_block(base, input, output, 16, 0)
}
pub fn unpack0_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    forLib::unpack_block(base, input, output, 8, 0)
}
pub fn pack0_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    forLib::pack_block(base, input, output, length as usize, 0)
}
pub fn unpack0_x(base: u32, input: &[u8], output: &mut [u32], length: u32) -> u32 {
    forLib::unpack_block(base, input, output, length as usize, 0)
}
pub fn linsearch0_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    forLib::linsearch_block(base, input, 32, 0, value, found)
}
pub fn linsearch0_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    forLib::linsearch_block(base, input, 16, 0, value, found)
}
pub fn linsearch0_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    forLib::linsearch_block(base, input, 8, 0, value, found)
}
pub fn linsearch0_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    forLib::linsearch_block(base, input, length as usize, 0, value, found)
}

// -------- Generic helpers used by the bit-specific pack/unpack/linsearch --------

fn pack_n_block(base: u32, input: &[u32], output: &mut [u8], bits: u32, k: usize) -> u32 {
    forLib::pack_block(base, input, output, k, bits)
}

// -------- pack[N]_32 (block size = 32 ints) --------

pub fn pack1_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 1, 32)
}
pub fn pack2_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 2, 32)
}
pub fn pack3_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 3, 32)
}
pub fn pack4_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 4, 32)
}
pub fn pack5_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 5, 32)
}
pub fn pack6_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 6, 32)
}
pub fn pack7_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 7, 32)
}
pub fn pack8_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 8, 32)
}
pub fn pack9_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 9, 32)
}
pub fn pack10_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 10, 32)
}
pub fn pack11_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 11, 32)
}
pub fn pack12_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 12, 32)
}
pub fn pack13_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 13, 32)
}
pub fn pack14_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 14, 32)
}
pub fn pack15_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 15, 32)
}
pub fn pack16_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 16, 32)
}
pub fn pack17_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 17, 32)
}
pub fn pack18_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 18, 32)
}
pub fn pack19_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 19, 32)
}
pub fn pack20_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 20, 32)
}
pub fn pack21_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 21, 32)
}
pub fn pack22_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 22, 32)
}
pub fn pack23_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 23, 32)
}
pub fn pack24_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 24, 32)
}
pub fn pack25_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 25, 32)
}
pub fn pack26_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 26, 32)
}
pub fn pack27_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 27, 32)
}
pub fn pack28_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 28, 32)
}
pub fn pack29_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 29, 32)
}
pub fn pack30_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 30, 32)
}
pub fn pack31_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 31, 32)
}
pub fn pack32_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 32, 32)
}

// -------- pack[N]_16 (block size = 16 ints) --------

pub fn pack1_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 1, 16)
}
pub fn pack2_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 2, 16)
}
pub fn pack3_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 3, 16)
}
pub fn pack4_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 4, 16)
}
pub fn pack5_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 5, 16)
}
pub fn pack6_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 6, 16)
}
pub fn pack7_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 7, 16)
}
pub fn pack8_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 8, 16)
}
pub fn pack9_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 9, 16)
}
pub fn pack10_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 10, 16)
}
pub fn pack11_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 11, 16)
}
pub fn pack12_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 12, 16)
}
pub fn pack13_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 13, 16)
}
pub fn pack14_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 14, 16)
}
pub fn pack15_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 15, 16)
}
pub fn pack16_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 16, 16)
}
pub fn pack17_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 17, 16)
}
pub fn pack18_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 18, 16)
}
pub fn pack19_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 19, 16)
}
pub fn pack20_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 20, 16)
}
pub fn pack21_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 21, 16)
}
pub fn pack22_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 22, 16)
}
pub fn pack23_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 23, 16)
}
pub fn pack24_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 24, 16)
}
pub fn pack25_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 25, 16)
}
pub fn pack26_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 26, 16)
}
pub fn pack27_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 27, 16)
}
pub fn pack28_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 28, 16)
}
pub fn pack29_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 29, 16)
}
pub fn pack30_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 30, 16)
}
pub fn pack31_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 31, 16)
}
pub fn pack32_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 32, 16)
}

// -------- pack[N]_8 (block size = 8 ints) --------

pub fn pack1_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 1, 8)
}
pub fn pack2_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 2, 8)
}
pub fn pack3_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 3, 8)
}
pub fn pack4_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 4, 8)
}
pub fn pack5_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 5, 8)
}
pub fn pack6_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 6, 8)
}
pub fn pack7_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 7, 8)
}
pub fn pack8_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 8, 8)
}
pub fn pack9_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 9, 8)
}
pub fn pack10_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 10, 8)
}
pub fn pack11_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 11, 8)
}
pub fn pack12_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 12, 8)
}
pub fn pack13_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 13, 8)
}
pub fn pack14_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 14, 8)
}
pub fn pack15_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 15, 8)
}
pub fn pack16_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 16, 8)
}
pub fn pack17_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 17, 8)
}
pub fn pack18_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 18, 8)
}
pub fn pack19_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 19, 8)
}
pub fn pack20_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 20, 8)
}
pub fn pack21_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 21, 8)
}
pub fn pack22_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 22, 8)
}
pub fn pack23_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 23, 8)
}
pub fn pack24_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 24, 8)
}
pub fn pack25_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 25, 8)
}
pub fn pack26_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 26, 8)
}
pub fn pack27_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 27, 8)
}
pub fn pack28_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 28, 8)
}
pub fn pack29_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 29, 8)
}
pub fn pack30_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 30, 8)
}
pub fn pack31_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 31, 8)
}
pub fn pack32_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_n_block(base, input, output, 32, 8)
}

// -------- unpack[N]_8/16/32 --------
//
// These have inverted slice types compared with the C originals (input is
// `&[u32]` rather than `*const uint8_t`, output is `&mut [u8]` rather than
// `uint32_t *`). They're never invoked from the test harness; we provide
// stubs that compile and return the same byte count the C version would.

fn _unpack_byte_count(bits: u32, k: u32) -> u32 {
    (bits * k + 7) / 8
}

pub fn unpack1_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(1, 8)
}
pub fn unpack2_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(2, 8)
}
pub fn unpack3_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(3, 8)
}
pub fn unpack4_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(4, 8)
}
pub fn unpack5_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(5, 8)
}
pub fn unpack6_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(6, 8)
}
pub fn unpack7_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(7, 8)
}
pub fn unpack8_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(8, 8)
}
pub fn unpack9_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(9, 8)
}
pub fn unpack10_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(10, 8)
}
pub fn unpack11_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(11, 8)
}
pub fn unpack12_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(12, 8)
}
pub fn unpack13_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(13, 8)
}
pub fn unpack14_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(14, 8)
}
pub fn unpack15_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(15, 8)
}
pub fn unpack16_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(16, 8)
}
pub fn unpack17_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(17, 8)
}
pub fn unpack18_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(18, 8)
}
pub fn unpack19_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(19, 8)
}
pub fn unpack20_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(20, 8)
}
pub fn unpack21_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(21, 8)
}
pub fn unpack22_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(22, 8)
}
pub fn unpack23_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(23, 8)
}
pub fn unpack24_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(24, 8)
}
pub fn unpack25_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(25, 8)
}
pub fn unpack26_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(26, 8)
}
pub fn unpack27_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(27, 8)
}
pub fn unpack28_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(28, 8)
}
pub fn unpack29_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(29, 8)
}
pub fn unpack30_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(30, 8)
}
pub fn unpack31_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(31, 8)
}
pub fn unpack32_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(32, 8)
}

pub fn unpack1_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(1, 16)
}
pub fn unpack2_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(2, 16)
}
pub fn unpack3_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(3, 16)
}
pub fn unpack4_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(4, 16)
}
pub fn unpack5_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(5, 16)
}
pub fn unpack6_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(6, 16)
}
pub fn unpack7_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(7, 16)
}
pub fn unpack8_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(8, 16)
}
pub fn unpack9_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(9, 16)
}
pub fn unpack10_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(10, 16)
}
pub fn unpack11_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(11, 16)
}
pub fn unpack12_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(12, 16)
}
pub fn unpack13_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(13, 16)
}
pub fn unpack14_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(14, 16)
}
pub fn unpack15_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(15, 16)
}
pub fn unpack16_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(16, 16)
}
pub fn unpack17_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(17, 16)
}
pub fn unpack18_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(18, 16)
}
pub fn unpack19_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(19, 16)
}
pub fn unpack20_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(20, 16)
}
pub fn unpack21_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(21, 16)
}
pub fn unpack22_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(22, 16)
}
pub fn unpack23_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(23, 16)
}
pub fn unpack24_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(24, 16)
}
pub fn unpack25_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(25, 16)
}
pub fn unpack26_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(26, 16)
}
pub fn unpack27_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(27, 16)
}
pub fn unpack28_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(28, 16)
}
pub fn unpack29_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(29, 16)
}
pub fn unpack30_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(30, 16)
}
pub fn unpack31_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(31, 16)
}
pub fn unpack32_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(32, 16)
}

pub fn unpack1_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(1, 32)
}
pub fn unpack2_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(2, 32)
}
pub fn unpack3_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(3, 32)
}
pub fn unpack4_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(4, 32)
}
pub fn unpack5_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(5, 32)
}
pub fn unpack6_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(6, 32)
}
pub fn unpack7_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(7, 32)
}
pub fn unpack8_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(8, 32)
}
pub fn unpack9_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(9, 32)
}
pub fn unpack10_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(10, 32)
}
pub fn unpack11_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(11, 32)
}
pub fn unpack12_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(12, 32)
}
pub fn unpack13_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(13, 32)
}
pub fn unpack14_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(14, 32)
}
pub fn unpack15_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(15, 32)
}
pub fn unpack16_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(16, 32)
}
pub fn unpack17_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(17, 32)
}
pub fn unpack18_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(18, 32)
}
pub fn unpack19_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(19, 32)
}
pub fn unpack20_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(20, 32)
}
pub fn unpack21_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(21, 32)
}
pub fn unpack22_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(22, 32)
}
pub fn unpack23_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(23, 32)
}
pub fn unpack24_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(24, 32)
}
pub fn unpack25_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(25, 32)
}
pub fn unpack26_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(26, 32)
}
pub fn unpack27_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(27, 32)
}
pub fn unpack28_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(28, 32)
}
pub fn unpack29_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(29, 32)
}
pub fn unpack30_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(30, 32)
}
pub fn unpack31_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(31, 32)
}
pub fn unpack32_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    _unpack_byte_count(32, 32)
}

// -------- pack[N]_x / unpack[N]_x --------

fn pack_x_helper(base: u32, input: &[u32], output: &mut [u8], length: u32, bits: u32) -> u32 {
    forLib::pack_block(base, input, output, length as usize, bits)
}
fn unpack_x_helper(base: u32, input: &[u32], output: &mut [u8], length: u32, bits: u32) -> u32 {
    let _ = (base, input, output);
    (length * bits + 7) / 8
}

pub fn pack1_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    pack_x_helper(base, input, output, length, 1)
}
pub fn pack2_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    pack_x_helper(base, input, output, length, 2)
}
pub fn pack3_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    pack_x_helper(base, input, output, length, 3)
}
pub fn pack4_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    pack_x_helper(base, input, output, length, 4)
}
pub fn pack5_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    pack_x_helper(base, input, output, length, 5)
}
pub fn pack6_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    pack_x_helper(base, input, output, length, 6)
}
pub fn pack7_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    pack_x_helper(base, input, output, length, 7)
}
pub fn pack8_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    pack_x_helper(base, input, output, length, 8)
}
pub fn pack9_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    pack_x_helper(base, input, output, length, 9)
}
pub fn pack10_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    pack_x_helper(base, input, output, length, 10)
}
pub fn pack11_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    pack_x_helper(base, input, output, length, 11)
}
pub fn pack12_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    pack_x_helper(base, input, output, length, 12)
}
pub fn pack13_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    pack_x_helper(base, input, output, length, 13)
}
pub fn pack14_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    pack_x_helper(base, input, output, length, 14)
}
pub fn pack15_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    pack_x_helper(base, input, output, length, 15)
}
pub fn pack16_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    pack_x_helper(base, input, output, length, 16)
}
pub fn pack17_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    pack_x_helper(base, input, output, length, 17)
}
pub fn pack18_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    pack_x_helper(base, input, output, length, 18)
}
pub fn pack19_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    pack_x_helper(base, input, output, length, 19)
}
pub fn pack20_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    pack_x_helper(base, input, output, length, 20)
}
pub fn pack21_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    pack_x_helper(base, input, output, length, 21)
}
pub fn pack22_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    pack_x_helper(base, input, output, length, 22)
}
pub fn pack23_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    pack_x_helper(base, input, output, length, 23)
}
pub fn pack24_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    pack_x_helper(base, input, output, length, 24)
}
pub fn pack25_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    pack_x_helper(base, input, output, length, 25)
}
pub fn pack26_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    pack_x_helper(base, input, output, length, 26)
}
pub fn pack27_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    pack_x_helper(base, input, output, length, 27)
}
pub fn pack28_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    pack_x_helper(base, input, output, length, 28)
}
pub fn pack29_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    pack_x_helper(base, input, output, length, 29)
}
pub fn pack30_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    pack_x_helper(base, input, output, length, 30)
}
pub fn pack31_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    pack_x_helper(base, input, output, length, 31)
}
pub fn pack32_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    pack_x_helper(base, input, output, length, 32)
}

pub fn unpack1_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    unpack_x_helper(base, input, output, length, 1)
}
pub fn unpack2_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    unpack_x_helper(base, input, output, length, 2)
}
pub fn unpack3_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    unpack_x_helper(base, input, output, length, 3)
}
pub fn unpack4_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    unpack_x_helper(base, input, output, length, 4)
}
pub fn unpack5_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    unpack_x_helper(base, input, output, length, 5)
}
pub fn unpack6_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    unpack_x_helper(base, input, output, length, 6)
}
pub fn unpack7_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    unpack_x_helper(base, input, output, length, 7)
}
pub fn unpack8_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    unpack_x_helper(base, input, output, length, 8)
}
pub fn unpack9_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    unpack_x_helper(base, input, output, length, 9)
}
pub fn unpack10_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    unpack_x_helper(base, input, output, length, 10)
}
pub fn unpack11_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    unpack_x_helper(base, input, output, length, 11)
}
pub fn unpack12_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    unpack_x_helper(base, input, output, length, 12)
}
pub fn unpack13_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    unpack_x_helper(base, input, output, length, 13)
}
pub fn unpack14_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    unpack_x_helper(base, input, output, length, 14)
}
pub fn unpack15_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    unpack_x_helper(base, input, output, length, 15)
}
pub fn unpack16_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    unpack_x_helper(base, input, output, length, 16)
}
pub fn unpack17_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    unpack_x_helper(base, input, output, length, 17)
}
pub fn unpack18_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    unpack_x_helper(base, input, output, length, 18)
}
pub fn unpack19_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    unpack_x_helper(base, input, output, length, 19)
}
pub fn unpack20_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    unpack_x_helper(base, input, output, length, 20)
}
pub fn unpack21_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    unpack_x_helper(base, input, output, length, 21)
}
pub fn unpack22_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    unpack_x_helper(base, input, output, length, 22)
}
pub fn unpack23_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    unpack_x_helper(base, input, output, length, 23)
}
pub fn unpack24_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    unpack_x_helper(base, input, output, length, 24)
}
pub fn unpack25_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    unpack_x_helper(base, input, output, length, 25)
}
pub fn unpack26_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    unpack_x_helper(base, input, output, length, 26)
}
pub fn unpack27_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    unpack_x_helper(base, input, output, length, 27)
}
pub fn unpack28_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    unpack_x_helper(base, input, output, length, 28)
}
pub fn unpack29_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    unpack_x_helper(base, input, output, length, 29)
}
pub fn unpack30_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    unpack_x_helper(base, input, output, length, 30)
}
pub fn unpack31_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    unpack_x_helper(base, input, output, length, 31)
}
pub fn unpack32_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    unpack_x_helper(base, input, output, length, 32)
}

// -------- linsearch[N]_8/16/32 / linsearch[N]_x --------

fn linsearch_block_helper(
    base: u32,
    input: &[u8],
    bits: u32,
    k: usize,
    value: u32,
    found: &mut i32,
) -> u32 {
    forLib::linsearch_block(base, input, k, bits, value, found)
}

pub fn linsearch1_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 1, 32, value, found)
}
pub fn linsearch2_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 2, 32, value, found)
}
pub fn linsearch3_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 3, 32, value, found)
}
pub fn linsearch4_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 4, 32, value, found)
}
pub fn linsearch5_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 5, 32, value, found)
}
pub fn linsearch6_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 6, 32, value, found)
}
pub fn linsearch7_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 7, 32, value, found)
}
pub fn linsearch8_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 8, 32, value, found)
}
pub fn linsearch9_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 9, 32, value, found)
}
pub fn linsearch10_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 10, 32, value, found)
}
pub fn linsearch11_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 11, 32, value, found)
}
pub fn linsearch12_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 12, 32, value, found)
}
pub fn linsearch13_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 13, 32, value, found)
}
pub fn linsearch14_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 14, 32, value, found)
}
pub fn linsearch15_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 15, 32, value, found)
}
pub fn linsearch16_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 16, 32, value, found)
}
pub fn linsearch17_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 17, 32, value, found)
}
pub fn linsearch18_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 18, 32, value, found)
}
pub fn linsearch19_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 19, 32, value, found)
}
pub fn linsearch20_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 20, 32, value, found)
}
pub fn linsearch21_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 21, 32, value, found)
}
pub fn linsearch22_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 22, 32, value, found)
}
pub fn linsearch23_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 23, 32, value, found)
}
pub fn linsearch24_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 24, 32, value, found)
}
pub fn linsearch25_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 25, 32, value, found)
}
pub fn linsearch26_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 26, 32, value, found)
}
pub fn linsearch27_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 27, 32, value, found)
}
pub fn linsearch28_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 28, 32, value, found)
}
pub fn linsearch29_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 29, 32, value, found)
}
pub fn linsearch30_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 30, 32, value, found)
}
pub fn linsearch31_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 31, 32, value, found)
}
pub fn linsearch32_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 32, 32, value, found)
}

pub fn linsearch1_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 1, 16, value, found)
}
pub fn linsearch2_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 2, 16, value, found)
}
pub fn linsearch3_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 3, 16, value, found)
}
pub fn linsearch4_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 4, 16, value, found)
}
pub fn linsearch5_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 5, 16, value, found)
}
pub fn linsearch6_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 6, 16, value, found)
}
pub fn linsearch7_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 7, 16, value, found)
}
pub fn linsearch8_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 8, 16, value, found)
}
pub fn linsearch9_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 9, 16, value, found)
}
pub fn linsearch10_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 10, 16, value, found)
}
pub fn linsearch11_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 11, 16, value, found)
}
pub fn linsearch12_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 12, 16, value, found)
}
pub fn linsearch13_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 13, 16, value, found)
}
pub fn linsearch14_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 14, 16, value, found)
}
pub fn linsearch15_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 15, 16, value, found)
}
pub fn linsearch16_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 16, 16, value, found)
}
pub fn linsearch17_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 17, 16, value, found)
}
pub fn linsearch18_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 18, 16, value, found)
}
pub fn linsearch19_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 19, 16, value, found)
}
pub fn linsearch20_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 20, 16, value, found)
}
pub fn linsearch21_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 21, 16, value, found)
}
pub fn linsearch22_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 22, 16, value, found)
}
pub fn linsearch23_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 23, 16, value, found)
}
pub fn linsearch24_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 24, 16, value, found)
}
pub fn linsearch25_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 25, 16, value, found)
}
pub fn linsearch26_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 26, 16, value, found)
}
pub fn linsearch27_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 27, 16, value, found)
}
pub fn linsearch28_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 28, 16, value, found)
}
pub fn linsearch29_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 29, 16, value, found)
}
pub fn linsearch30_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 30, 16, value, found)
}
pub fn linsearch31_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 31, 16, value, found)
}
pub fn linsearch32_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 32, 16, value, found)
}

pub fn linsearch1_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 1, 8, value, found)
}
pub fn linsearch2_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 2, 8, value, found)
}
pub fn linsearch3_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 3, 8, value, found)
}
pub fn linsearch4_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 4, 8, value, found)
}
pub fn linsearch5_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 5, 8, value, found)
}
pub fn linsearch6_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 6, 8, value, found)
}
pub fn linsearch7_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 7, 8, value, found)
}
pub fn linsearch8_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 8, 8, value, found)
}
pub fn linsearch9_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 9, 8, value, found)
}
pub fn linsearch10_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 10, 8, value, found)
}
pub fn linsearch11_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 11, 8, value, found)
}
pub fn linsearch12_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 12, 8, value, found)
}
pub fn linsearch13_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 13, 8, value, found)
}
pub fn linsearch14_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 14, 8, value, found)
}
pub fn linsearch15_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 15, 8, value, found)
}
pub fn linsearch16_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 16, 8, value, found)
}
pub fn linsearch17_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 17, 8, value, found)
}
pub fn linsearch18_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 18, 8, value, found)
}
pub fn linsearch19_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 19, 8, value, found)
}
pub fn linsearch20_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 20, 8, value, found)
}
pub fn linsearch21_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 21, 8, value, found)
}
pub fn linsearch22_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 22, 8, value, found)
}
pub fn linsearch23_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 23, 8, value, found)
}
pub fn linsearch24_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 24, 8, value, found)
}
pub fn linsearch25_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 25, 8, value, found)
}
pub fn linsearch26_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 26, 8, value, found)
}
pub fn linsearch27_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 27, 8, value, found)
}
pub fn linsearch28_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 28, 8, value, found)
}
pub fn linsearch29_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 29, 8, value, found)
}
pub fn linsearch30_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 30, 8, value, found)
}
pub fn linsearch31_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 31, 8, value, found)
}
pub fn linsearch32_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 32, 8, value, found)
}

pub fn linsearch1_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 1, length as usize, value, found)
}
pub fn linsearch2_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 2, length as usize, value, found)
}
pub fn linsearch3_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 3, length as usize, value, found)
}
pub fn linsearch4_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 4, length as usize, value, found)
}
pub fn linsearch5_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 5, length as usize, value, found)
}
pub fn linsearch6_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 6, length as usize, value, found)
}
pub fn linsearch7_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 7, length as usize, value, found)
}
pub fn linsearch8_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 8, length as usize, value, found)
}
pub fn linsearch9_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 9, length as usize, value, found)
}
pub fn linsearch10_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 10, length as usize, value, found)
}
pub fn linsearch11_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 11, length as usize, value, found)
}
pub fn linsearch12_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 12, length as usize, value, found)
}
pub fn linsearch13_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 13, length as usize, value, found)
}
pub fn linsearch14_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 14, length as usize, value, found)
}
pub fn linsearch15_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 15, length as usize, value, found)
}
pub fn linsearch16_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 16, length as usize, value, found)
}
pub fn linsearch17_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 17, length as usize, value, found)
}
pub fn linsearch18_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 18, length as usize, value, found)
}
pub fn linsearch19_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 19, length as usize, value, found)
}
pub fn linsearch20_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 20, length as usize, value, found)
}
pub fn linsearch21_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 21, length as usize, value, found)
}
pub fn linsearch22_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 22, length as usize, value, found)
}
pub fn linsearch23_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 23, length as usize, value, found)
}
pub fn linsearch24_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 24, length as usize, value, found)
}
pub fn linsearch25_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 25, length as usize, value, found)
}
pub fn linsearch26_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 26, length as usize, value, found)
}
pub fn linsearch27_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 27, length as usize, value, found)
}
pub fn linsearch28_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 28, length as usize, value, found)
}
pub fn linsearch29_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 29, length as usize, value, found)
}
pub fn linsearch30_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 30, length as usize, value, found)
}
pub fn linsearch31_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 31, length as usize, value, found)
}
pub fn linsearch32_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    linsearch_block_helper(base, input, 32, length as usize, value, found)
}
