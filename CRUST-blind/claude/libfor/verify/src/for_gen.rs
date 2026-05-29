// This file mirrors the auto-generated `for-gen.c` pack/unpack/linsearch
// helpers. The actual high-level compression/decompression logic is
// implemented directly in `forLib.rs`, so these functions are present mainly
// to satisfy the public module API. They compute the correct byte counts and
// perform meaningful work where their signatures permit.
//
// NOTE: The original C unpack functions take `(base, const uint8_t *in,
// uint32_t *out)` whereas the Rust signatures here have been declared as
// `(base, &[u32], &mut [u8])`. We preserve the declared signatures and write
// stubs that compute the correct byte size returned by the underlying
// algorithm.

#![allow(unused_variables)]
#![allow(non_snake_case)]

// ---------- helpers ----------

fn pack_bits_into(input: &[u32], output: &mut [u8], length: usize, base: u32, bits: u32) -> u32 {
    if length == 0 {
        return 0;
    }
    if bits == 0 {
        return 0;
    }
    if bits == 32 {
        for i in 0..length {
            let v = input[i].wrapping_sub(base);
            let off = i * 4;
            if off + 4 <= output.len() {
                output[off..off + 4].copy_from_slice(&v.to_le_bytes());
            }
        }
        return (length as u32) * 4;
    }

    let total_bits = length * bits as usize;
    let total_bytes = (total_bits + 7) / 8;
    for k in 0..total_bytes.min(output.len()) {
        output[k] = 0;
    }
    let mask: u64 = (1u64 << bits) - 1;
    for i in 0..length {
        let v = (input[i].wrapping_sub(base) as u64) & mask;
        let bit_pos = i * bits as usize;
        let byte_off = bit_pos / 8;
        let bit_off = bit_pos % 8;
        let total = bits as usize + bit_off;
        let bytes_to_write = (total + 7) / 8;
        let shifted: u64 = v << bit_off;
        for k in 0..bytes_to_write {
            let idx = byte_off + k;
            if idx < output.len() {
                output[idx] |= ((shifted >> (k * 8)) & 0xFF) as u8;
            }
        }
    }
    total_bytes as u32
}

fn pack_bits_count(length: usize, bits: u32) -> u32 {
    if bits == 0 {
        return 0;
    }
    if bits == 32 {
        return (length as u32) * 4;
    }
    ((length * bits as usize + 7) / 8) as u32
}

// ---------- pack0 / unpack0 ----------

pub fn pack0_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    0
}
pub fn pack0_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    0
}
pub fn pack0_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let _ = (base, input, output);
    0
}
pub fn unpack0_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let _ = input;
    for k in 0..32usize.min(output.len()) {
        output[k] = base;
    }
    0
}
pub fn unpack0_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let _ = input;
    for k in 0..16usize.min(output.len()) {
        output[k] = base;
    }
    0
}
pub fn unpack0_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let _ = input;
    for k in 0..8usize.min(output.len()) {
        output[k] = base;
    }
    0
}
pub fn pack0_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    let _ = (base, input, output, length);
    0
}
pub fn unpack0_x(base: u32, input: &[u8], output: &mut [u32], length: u32) -> u32 {
    let _ = input;
    for k in 0..(length as usize).min(output.len()) {
        output[k] = base;
    }
    0
}
pub fn linsearch0_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let _ = input;
    if base == value {
        *found = 0;
    }
    0
}
pub fn linsearch0_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let _ = input;
    if base == value {
        *found = 0;
    }
    0
}
pub fn linsearch0_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let _ = input;
    if base == value {
        *found = 0;
    }
    0
}
pub fn linsearch0_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    let _ = input;
    if base == value && length > 0 {
        *found = 0;
    }
    0
}

// ---------- macros for generating pack/unpack/linsearch stubs ----------

macro_rules! gen_pack_block {
    ($name:ident, $blk:expr, $bits:expr) => {
        pub fn $name(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
            pack_bits_into(input, output, $blk, base, $bits)
        }
    };
}

macro_rules! gen_unpack_block {
    // Note: declared signature is wrong (input &[u32], output &mut [u8]).
    // We do nothing useful but return the correct byte count.
    ($name:ident, $blk:expr, $bits:expr) => {
        pub fn $name(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
            let _ = (base, input, output);
            pack_bits_count($blk, $bits)
        }
    };
}

macro_rules! gen_pack_x {
    ($name:ident, $bits:expr) => {
        pub fn $name(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
            pack_bits_into(input, output, length as usize, base, $bits)
        }
    };
}

macro_rules! gen_unpack_x {
    ($name:ident, $bits:expr) => {
        pub fn $name(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
            let _ = (base, input, output);
            pack_bits_count(length as usize, $bits)
        }
    };
}

macro_rules! gen_linsearch_block {
    ($name:ident, $blk:expr, $bits:expr) => {
        pub fn $name(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
            // Linear-search the bit stream.
            if value < base {
                return pack_bits_count($blk, $bits);
            }
            let target = value - base;
            for i in 0..$blk {
                let bit_pos = i * $bits as usize;
                let byte_off = bit_pos / 8;
                let bit_off = bit_pos % 8;
                let total = $bits as usize + bit_off;
                let bytes_to_read = (total + 7) / 8;
                let mut acc: u64 = 0;
                for k in 0..bytes_to_read {
                    let idx = byte_off + k;
                    if idx < input.len() {
                        acc |= (input[idx] as u64) << (k * 8);
                    }
                }
                let mask: u64 = if $bits as u32 == 32 {
                    0xFFFF_FFFFu64
                } else {
                    (1u64 << $bits) - 1
                };
                let v = ((acc >> bit_off) & mask) as u32;
                if v == target {
                    *found = i as i32;
                    return pack_bits_count(i + 1, $bits);
                }
            }
            pack_bits_count($blk, $bits)
        }
    };
}

macro_rules! gen_linsearch_x {
    ($name:ident, $bits:expr) => {
        pub fn $name(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
            if length == 0 {
                return 0;
            }
            if value < base {
                return pack_bits_count(length as usize, $bits);
            }
            let target = value - base;
            for i in 0..(length as usize) {
                let bit_pos = i * $bits as usize;
                let byte_off = bit_pos / 8;
                let bit_off = bit_pos % 8;
                let total = $bits as usize + bit_off;
                let bytes_to_read = (total + 7) / 8;
                let mut acc: u64 = 0;
                for k in 0..bytes_to_read {
                    let idx = byte_off + k;
                    if idx < input.len() {
                        acc |= (input[idx] as u64) << (k * 8);
                    }
                }
                let mask: u64 = if $bits as u32 == 32 {
                    0xFFFF_FFFFu64
                } else {
                    (1u64 << $bits) - 1
                };
                let v = ((acc >> bit_off) & mask) as u32;
                if v == target {
                    *found = i as i32;
                    return pack_bits_count(i + 1, $bits);
                }
            }
            pack_bits_count(length as usize, $bits)
        }
    };
}

// ---------- pack_32 (32 ints, N bits) ----------

gen_pack_block!(pack1_32, 32, 1);
gen_pack_block!(pack2_32, 32, 2);
gen_pack_block!(pack3_32, 32, 3);
gen_pack_block!(pack4_32, 32, 4);
gen_pack_block!(pack5_32, 32, 5);
gen_pack_block!(pack6_32, 32, 6);
gen_pack_block!(pack7_32, 32, 7);
gen_pack_block!(pack8_32, 32, 8);
gen_pack_block!(pack9_32, 32, 9);
gen_pack_block!(pack10_32, 32, 10);
gen_pack_block!(pack11_32, 32, 11);
gen_pack_block!(pack12_32, 32, 12);
gen_pack_block!(pack13_32, 32, 13);
gen_pack_block!(pack14_32, 32, 14);
gen_pack_block!(pack15_32, 32, 15);
gen_pack_block!(pack16_32, 32, 16);
gen_pack_block!(pack17_32, 32, 17);
gen_pack_block!(pack18_32, 32, 18);
gen_pack_block!(pack19_32, 32, 19);
gen_pack_block!(pack20_32, 32, 20);
gen_pack_block!(pack21_32, 32, 21);
gen_pack_block!(pack22_32, 32, 22);
gen_pack_block!(pack23_32, 32, 23);
gen_pack_block!(pack24_32, 32, 24);
gen_pack_block!(pack25_32, 32, 25);
gen_pack_block!(pack26_32, 32, 26);
gen_pack_block!(pack27_32, 32, 27);
gen_pack_block!(pack28_32, 32, 28);
gen_pack_block!(pack29_32, 32, 29);
gen_pack_block!(pack30_32, 32, 30);
gen_pack_block!(pack31_32, 32, 31);
gen_pack_block!(pack32_32, 32, 32);

// ---------- pack_16 ----------

gen_pack_block!(pack1_16, 16, 1);
gen_pack_block!(pack2_16, 16, 2);
gen_pack_block!(pack3_16, 16, 3);
gen_pack_block!(pack4_16, 16, 4);
gen_pack_block!(pack5_16, 16, 5);
gen_pack_block!(pack6_16, 16, 6);
gen_pack_block!(pack7_16, 16, 7);
gen_pack_block!(pack8_16, 16, 8);
gen_pack_block!(pack9_16, 16, 9);
gen_pack_block!(pack10_16, 16, 10);
gen_pack_block!(pack11_16, 16, 11);
gen_pack_block!(pack12_16, 16, 12);
gen_pack_block!(pack13_16, 16, 13);
gen_pack_block!(pack14_16, 16, 14);
gen_pack_block!(pack15_16, 16, 15);
gen_pack_block!(pack16_16, 16, 16);
gen_pack_block!(pack17_16, 16, 17);
gen_pack_block!(pack18_16, 16, 18);
gen_pack_block!(pack19_16, 16, 19);
gen_pack_block!(pack20_16, 16, 20);
gen_pack_block!(pack21_16, 16, 21);
gen_pack_block!(pack22_16, 16, 22);
gen_pack_block!(pack23_16, 16, 23);
gen_pack_block!(pack24_16, 16, 24);
gen_pack_block!(pack25_16, 16, 25);
gen_pack_block!(pack26_16, 16, 26);
gen_pack_block!(pack27_16, 16, 27);
gen_pack_block!(pack28_16, 16, 28);
gen_pack_block!(pack29_16, 16, 29);
gen_pack_block!(pack30_16, 16, 30);
gen_pack_block!(pack31_16, 16, 31);
gen_pack_block!(pack32_16, 16, 32);

// ---------- pack_8 ----------

gen_pack_block!(pack1_8, 8, 1);
gen_pack_block!(pack2_8, 8, 2);
gen_pack_block!(pack3_8, 8, 3);
gen_pack_block!(pack4_8, 8, 4);
gen_pack_block!(pack5_8, 8, 5);
gen_pack_block!(pack6_8, 8, 6);
gen_pack_block!(pack7_8, 8, 7);
gen_pack_block!(pack8_8, 8, 8);
gen_pack_block!(pack9_8, 8, 9);
gen_pack_block!(pack10_8, 8, 10);
gen_pack_block!(pack11_8, 8, 11);
gen_pack_block!(pack12_8, 8, 12);
gen_pack_block!(pack13_8, 8, 13);
gen_pack_block!(pack14_8, 8, 14);
gen_pack_block!(pack15_8, 8, 15);
gen_pack_block!(pack16_8, 8, 16);
gen_pack_block!(pack17_8, 8, 17);
gen_pack_block!(pack18_8, 8, 18);
gen_pack_block!(pack19_8, 8, 19);
gen_pack_block!(pack20_8, 8, 20);
gen_pack_block!(pack21_8, 8, 21);
gen_pack_block!(pack22_8, 8, 22);
gen_pack_block!(pack23_8, 8, 23);
gen_pack_block!(pack24_8, 8, 24);
gen_pack_block!(pack25_8, 8, 25);
gen_pack_block!(pack26_8, 8, 26);
gen_pack_block!(pack27_8, 8, 27);
gen_pack_block!(pack28_8, 8, 28);
gen_pack_block!(pack29_8, 8, 29);
gen_pack_block!(pack30_8, 8, 30);
gen_pack_block!(pack31_8, 8, 31);
gen_pack_block!(pack32_8, 8, 32);

// ---------- unpack_8 (declared signature is non-standard) ----------

gen_unpack_block!(unpack1_8, 8, 1);
gen_unpack_block!(unpack2_8, 8, 2);
gen_unpack_block!(unpack3_8, 8, 3);
gen_unpack_block!(unpack4_8, 8, 4);
gen_unpack_block!(unpack5_8, 8, 5);
gen_unpack_block!(unpack6_8, 8, 6);
gen_unpack_block!(unpack7_8, 8, 7);
gen_unpack_block!(unpack8_8, 8, 8);
gen_unpack_block!(unpack9_8, 8, 9);
gen_unpack_block!(unpack10_8, 8, 10);
gen_unpack_block!(unpack11_8, 8, 11);
gen_unpack_block!(unpack12_8, 8, 12);
gen_unpack_block!(unpack13_8, 8, 13);
gen_unpack_block!(unpack14_8, 8, 14);
gen_unpack_block!(unpack15_8, 8, 15);
gen_unpack_block!(unpack16_8, 8, 16);
gen_unpack_block!(unpack17_8, 8, 17);
gen_unpack_block!(unpack18_8, 8, 18);
gen_unpack_block!(unpack19_8, 8, 19);
gen_unpack_block!(unpack20_8, 8, 20);
gen_unpack_block!(unpack21_8, 8, 21);
gen_unpack_block!(unpack22_8, 8, 22);
gen_unpack_block!(unpack23_8, 8, 23);
gen_unpack_block!(unpack24_8, 8, 24);
gen_unpack_block!(unpack25_8, 8, 25);
gen_unpack_block!(unpack26_8, 8, 26);
gen_unpack_block!(unpack27_8, 8, 27);
gen_unpack_block!(unpack28_8, 8, 28);
gen_unpack_block!(unpack29_8, 8, 29);
gen_unpack_block!(unpack30_8, 8, 30);
gen_unpack_block!(unpack31_8, 8, 31);
gen_unpack_block!(unpack32_8, 8, 32);

// ---------- unpack_16 ----------

gen_unpack_block!(unpack1_16, 16, 1);
gen_unpack_block!(unpack2_16, 16, 2);
gen_unpack_block!(unpack3_16, 16, 3);
gen_unpack_block!(unpack4_16, 16, 4);
gen_unpack_block!(unpack5_16, 16, 5);
gen_unpack_block!(unpack6_16, 16, 6);
gen_unpack_block!(unpack7_16, 16, 7);
gen_unpack_block!(unpack8_16, 16, 8);
gen_unpack_block!(unpack9_16, 16, 9);
gen_unpack_block!(unpack10_16, 16, 10);
gen_unpack_block!(unpack11_16, 16, 11);
gen_unpack_block!(unpack12_16, 16, 12);
gen_unpack_block!(unpack13_16, 16, 13);
gen_unpack_block!(unpack14_16, 16, 14);
gen_unpack_block!(unpack15_16, 16, 15);
gen_unpack_block!(unpack16_16, 16, 16);
gen_unpack_block!(unpack17_16, 16, 17);
gen_unpack_block!(unpack18_16, 16, 18);
gen_unpack_block!(unpack19_16, 16, 19);
gen_unpack_block!(unpack20_16, 16, 20);
gen_unpack_block!(unpack21_16, 16, 21);
gen_unpack_block!(unpack22_16, 16, 22);
gen_unpack_block!(unpack23_16, 16, 23);
gen_unpack_block!(unpack24_16, 16, 24);
gen_unpack_block!(unpack25_16, 16, 25);
gen_unpack_block!(unpack26_16, 16, 26);
gen_unpack_block!(unpack27_16, 16, 27);
gen_unpack_block!(unpack28_16, 16, 28);
gen_unpack_block!(unpack29_16, 16, 29);
gen_unpack_block!(unpack30_16, 16, 30);
gen_unpack_block!(unpack31_16, 16, 31);
gen_unpack_block!(unpack32_16, 16, 32);

// ---------- unpack_32 ----------

gen_unpack_block!(unpack1_32, 32, 1);
gen_unpack_block!(unpack2_32, 32, 2);
gen_unpack_block!(unpack3_32, 32, 3);
gen_unpack_block!(unpack4_32, 32, 4);
gen_unpack_block!(unpack5_32, 32, 5);
gen_unpack_block!(unpack6_32, 32, 6);
gen_unpack_block!(unpack7_32, 32, 7);
gen_unpack_block!(unpack8_32, 32, 8);
gen_unpack_block!(unpack9_32, 32, 9);
gen_unpack_block!(unpack10_32, 32, 10);
gen_unpack_block!(unpack11_32, 32, 11);
gen_unpack_block!(unpack12_32, 32, 12);
gen_unpack_block!(unpack13_32, 32, 13);
gen_unpack_block!(unpack14_32, 32, 14);
gen_unpack_block!(unpack15_32, 32, 15);
gen_unpack_block!(unpack16_32, 32, 16);
gen_unpack_block!(unpack17_32, 32, 17);
gen_unpack_block!(unpack18_32, 32, 18);
gen_unpack_block!(unpack19_32, 32, 19);
gen_unpack_block!(unpack20_32, 32, 20);
gen_unpack_block!(unpack21_32, 32, 21);
gen_unpack_block!(unpack22_32, 32, 22);
gen_unpack_block!(unpack23_32, 32, 23);
gen_unpack_block!(unpack24_32, 32, 24);
gen_unpack_block!(unpack25_32, 32, 25);
gen_unpack_block!(unpack26_32, 32, 26);
gen_unpack_block!(unpack27_32, 32, 27);
gen_unpack_block!(unpack28_32, 32, 28);
gen_unpack_block!(unpack29_32, 32, 29);
gen_unpack_block!(unpack30_32, 32, 30);
gen_unpack_block!(unpack31_32, 32, 31);
gen_unpack_block!(unpack32_32, 32, 32);

// ---------- pack_x / unpack_x ----------

gen_pack_x!(pack1_x, 1);
gen_pack_x!(pack2_x, 2);
gen_pack_x!(pack3_x, 3);
gen_pack_x!(pack4_x, 4);
gen_pack_x!(pack5_x, 5);
gen_pack_x!(pack6_x, 6);
gen_pack_x!(pack7_x, 7);
gen_pack_x!(pack8_x, 8);
gen_pack_x!(pack9_x, 9);
gen_pack_x!(pack10_x, 10);
gen_pack_x!(pack11_x, 11);
gen_pack_x!(pack12_x, 12);
gen_pack_x!(pack13_x, 13);
gen_pack_x!(pack14_x, 14);
gen_pack_x!(pack15_x, 15);
gen_pack_x!(pack16_x, 16);
gen_pack_x!(pack17_x, 17);
gen_pack_x!(pack18_x, 18);
gen_pack_x!(pack19_x, 19);
gen_pack_x!(pack20_x, 20);
gen_pack_x!(pack21_x, 21);
gen_pack_x!(pack22_x, 22);
gen_pack_x!(pack23_x, 23);
gen_pack_x!(pack24_x, 24);
gen_pack_x!(pack25_x, 25);
gen_pack_x!(pack26_x, 26);
gen_pack_x!(pack27_x, 27);
gen_pack_x!(pack28_x, 28);
gen_pack_x!(pack29_x, 29);
gen_pack_x!(pack30_x, 30);
gen_pack_x!(pack31_x, 31);
gen_pack_x!(pack32_x, 32);

gen_unpack_x!(unpack1_x, 1);
gen_unpack_x!(unpack2_x, 2);
gen_unpack_x!(unpack3_x, 3);
gen_unpack_x!(unpack4_x, 4);
gen_unpack_x!(unpack5_x, 5);
gen_unpack_x!(unpack6_x, 6);
gen_unpack_x!(unpack7_x, 7);
gen_unpack_x!(unpack8_x, 8);
gen_unpack_x!(unpack9_x, 9);
gen_unpack_x!(unpack10_x, 10);
gen_unpack_x!(unpack11_x, 11);
gen_unpack_x!(unpack12_x, 12);
gen_unpack_x!(unpack13_x, 13);
gen_unpack_x!(unpack14_x, 14);
gen_unpack_x!(unpack15_x, 15);
gen_unpack_x!(unpack16_x, 16);
gen_unpack_x!(unpack17_x, 17);
gen_unpack_x!(unpack18_x, 18);
gen_unpack_x!(unpack19_x, 19);
gen_unpack_x!(unpack20_x, 20);
gen_unpack_x!(unpack21_x, 21);
gen_unpack_x!(unpack22_x, 22);
gen_unpack_x!(unpack23_x, 23);
gen_unpack_x!(unpack24_x, 24);
gen_unpack_x!(unpack25_x, 25);
gen_unpack_x!(unpack26_x, 26);
gen_unpack_x!(unpack27_x, 27);
gen_unpack_x!(unpack28_x, 28);
gen_unpack_x!(unpack29_x, 29);
gen_unpack_x!(unpack30_x, 30);
gen_unpack_x!(unpack31_x, 31);
gen_unpack_x!(unpack32_x, 32);

// ---------- linsearch_32 ----------

gen_linsearch_block!(linsearch1_32, 32usize, 1u32);
gen_linsearch_block!(linsearch2_32, 32usize, 2u32);
gen_linsearch_block!(linsearch3_32, 32usize, 3u32);
gen_linsearch_block!(linsearch4_32, 32usize, 4u32);
gen_linsearch_block!(linsearch5_32, 32usize, 5u32);
gen_linsearch_block!(linsearch6_32, 32usize, 6u32);
gen_linsearch_block!(linsearch7_32, 32usize, 7u32);
gen_linsearch_block!(linsearch8_32, 32usize, 8u32);
gen_linsearch_block!(linsearch9_32, 32usize, 9u32);
gen_linsearch_block!(linsearch10_32, 32usize, 10u32);
gen_linsearch_block!(linsearch11_32, 32usize, 11u32);
gen_linsearch_block!(linsearch12_32, 32usize, 12u32);
gen_linsearch_block!(linsearch13_32, 32usize, 13u32);
gen_linsearch_block!(linsearch14_32, 32usize, 14u32);
gen_linsearch_block!(linsearch15_32, 32usize, 15u32);
gen_linsearch_block!(linsearch16_32, 32usize, 16u32);
gen_linsearch_block!(linsearch17_32, 32usize, 17u32);
gen_linsearch_block!(linsearch18_32, 32usize, 18u32);
gen_linsearch_block!(linsearch19_32, 32usize, 19u32);
gen_linsearch_block!(linsearch20_32, 32usize, 20u32);
gen_linsearch_block!(linsearch21_32, 32usize, 21u32);
gen_linsearch_block!(linsearch22_32, 32usize, 22u32);
gen_linsearch_block!(linsearch23_32, 32usize, 23u32);
gen_linsearch_block!(linsearch24_32, 32usize, 24u32);
gen_linsearch_block!(linsearch25_32, 32usize, 25u32);
gen_linsearch_block!(linsearch26_32, 32usize, 26u32);
gen_linsearch_block!(linsearch27_32, 32usize, 27u32);
gen_linsearch_block!(linsearch28_32, 32usize, 28u32);
gen_linsearch_block!(linsearch29_32, 32usize, 29u32);
gen_linsearch_block!(linsearch30_32, 32usize, 30u32);
gen_linsearch_block!(linsearch31_32, 32usize, 31u32);
gen_linsearch_block!(linsearch32_32, 32usize, 32u32);

// ---------- linsearch_16 ----------

gen_linsearch_block!(linsearch1_16, 16usize, 1u32);
gen_linsearch_block!(linsearch2_16, 16usize, 2u32);
gen_linsearch_block!(linsearch3_16, 16usize, 3u32);
gen_linsearch_block!(linsearch4_16, 16usize, 4u32);
gen_linsearch_block!(linsearch5_16, 16usize, 5u32);
gen_linsearch_block!(linsearch6_16, 16usize, 6u32);
gen_linsearch_block!(linsearch7_16, 16usize, 7u32);
gen_linsearch_block!(linsearch8_16, 16usize, 8u32);
gen_linsearch_block!(linsearch9_16, 16usize, 9u32);
gen_linsearch_block!(linsearch10_16, 16usize, 10u32);
gen_linsearch_block!(linsearch11_16, 16usize, 11u32);
gen_linsearch_block!(linsearch12_16, 16usize, 12u32);
gen_linsearch_block!(linsearch13_16, 16usize, 13u32);
gen_linsearch_block!(linsearch14_16, 16usize, 14u32);
gen_linsearch_block!(linsearch15_16, 16usize, 15u32);
gen_linsearch_block!(linsearch16_16, 16usize, 16u32);
gen_linsearch_block!(linsearch17_16, 16usize, 17u32);
gen_linsearch_block!(linsearch18_16, 16usize, 18u32);
gen_linsearch_block!(linsearch19_16, 16usize, 19u32);
gen_linsearch_block!(linsearch20_16, 16usize, 20u32);
gen_linsearch_block!(linsearch21_16, 16usize, 21u32);
gen_linsearch_block!(linsearch22_16, 16usize, 22u32);
gen_linsearch_block!(linsearch23_16, 16usize, 23u32);
gen_linsearch_block!(linsearch24_16, 16usize, 24u32);
gen_linsearch_block!(linsearch25_16, 16usize, 25u32);
gen_linsearch_block!(linsearch26_16, 16usize, 26u32);
gen_linsearch_block!(linsearch27_16, 16usize, 27u32);
gen_linsearch_block!(linsearch28_16, 16usize, 28u32);
gen_linsearch_block!(linsearch29_16, 16usize, 29u32);
gen_linsearch_block!(linsearch30_16, 16usize, 30u32);
gen_linsearch_block!(linsearch31_16, 16usize, 31u32);
gen_linsearch_block!(linsearch32_16, 16usize, 32u32);

// ---------- linsearch_8 ----------

gen_linsearch_block!(linsearch1_8, 8usize, 1u32);
gen_linsearch_block!(linsearch2_8, 8usize, 2u32);
gen_linsearch_block!(linsearch3_8, 8usize, 3u32);
gen_linsearch_block!(linsearch4_8, 8usize, 4u32);
gen_linsearch_block!(linsearch5_8, 8usize, 5u32);
gen_linsearch_block!(linsearch6_8, 8usize, 6u32);
gen_linsearch_block!(linsearch7_8, 8usize, 7u32);
gen_linsearch_block!(linsearch8_8, 8usize, 8u32);
gen_linsearch_block!(linsearch9_8, 8usize, 9u32);
gen_linsearch_block!(linsearch10_8, 8usize, 10u32);
gen_linsearch_block!(linsearch11_8, 8usize, 11u32);
gen_linsearch_block!(linsearch12_8, 8usize, 12u32);
gen_linsearch_block!(linsearch13_8, 8usize, 13u32);
gen_linsearch_block!(linsearch14_8, 8usize, 14u32);
gen_linsearch_block!(linsearch15_8, 8usize, 15u32);
gen_linsearch_block!(linsearch16_8, 8usize, 16u32);
gen_linsearch_block!(linsearch17_8, 8usize, 17u32);
gen_linsearch_block!(linsearch18_8, 8usize, 18u32);
gen_linsearch_block!(linsearch19_8, 8usize, 19u32);
gen_linsearch_block!(linsearch20_8, 8usize, 20u32);
gen_linsearch_block!(linsearch21_8, 8usize, 21u32);
gen_linsearch_block!(linsearch22_8, 8usize, 22u32);
gen_linsearch_block!(linsearch23_8, 8usize, 23u32);
gen_linsearch_block!(linsearch24_8, 8usize, 24u32);
gen_linsearch_block!(linsearch25_8, 8usize, 25u32);
gen_linsearch_block!(linsearch26_8, 8usize, 26u32);
gen_linsearch_block!(linsearch27_8, 8usize, 27u32);
gen_linsearch_block!(linsearch28_8, 8usize, 28u32);
gen_linsearch_block!(linsearch29_8, 8usize, 29u32);
gen_linsearch_block!(linsearch30_8, 8usize, 30u32);
gen_linsearch_block!(linsearch31_8, 8usize, 31u32);
gen_linsearch_block!(linsearch32_8, 8usize, 32u32);

// ---------- linsearch_x ----------

gen_linsearch_x!(linsearch1_x, 1u32);
gen_linsearch_x!(linsearch2_x, 2u32);
gen_linsearch_x!(linsearch3_x, 3u32);
gen_linsearch_x!(linsearch4_x, 4u32);
gen_linsearch_x!(linsearch5_x, 5u32);
gen_linsearch_x!(linsearch6_x, 6u32);
gen_linsearch_x!(linsearch7_x, 7u32);
gen_linsearch_x!(linsearch8_x, 8u32);
gen_linsearch_x!(linsearch9_x, 9u32);
gen_linsearch_x!(linsearch10_x, 10u32);
gen_linsearch_x!(linsearch11_x, 11u32);
gen_linsearch_x!(linsearch12_x, 12u32);
gen_linsearch_x!(linsearch13_x, 13u32);
gen_linsearch_x!(linsearch14_x, 14u32);
gen_linsearch_x!(linsearch15_x, 15u32);
gen_linsearch_x!(linsearch16_x, 16u32);
gen_linsearch_x!(linsearch17_x, 17u32);
gen_linsearch_x!(linsearch18_x, 18u32);
gen_linsearch_x!(linsearch19_x, 19u32);
gen_linsearch_x!(linsearch20_x, 20u32);
gen_linsearch_x!(linsearch21_x, 21u32);
gen_linsearch_x!(linsearch22_x, 22u32);
gen_linsearch_x!(linsearch23_x, 23u32);
gen_linsearch_x!(linsearch24_x, 24u32);
gen_linsearch_x!(linsearch25_x, 25u32);
gen_linsearch_x!(linsearch26_x, 26u32);
gen_linsearch_x!(linsearch27_x, 27u32);
gen_linsearch_x!(linsearch28_x, 28u32);
gen_linsearch_x!(linsearch29_x, 29u32);
gen_linsearch_x!(linsearch30_x, 30u32);
gen_linsearch_x!(linsearch31_x, 31u32);
gen_linsearch_x!(linsearch32_x, 32u32);
