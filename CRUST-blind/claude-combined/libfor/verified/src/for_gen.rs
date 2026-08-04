// Generic packing helpers used by all generated pack/unpack/linsearch routines.
//
// In the C source, these are 396 hand-rolled per-bit functions (e.g. pack1_32,
// pack2_32, ..., unpack32_x, linsearch5_16, ...). Rather than reproducing the
// hand-rolled bit-twiddling for every (bits, block) pair, we implement a small
// number of generic helpers and dispatch every wrapper to them.
//
// Note: the signatures of the `unpack1_*` ... `unpack32_*` functions in the
// shipped Rust interface are ambiguous — the C version takes
// `(base, in: &[u8], out: &mut [u32])` whereas the Rust interface declares
// `(base, in: &[u32], out: &mut [u8])`. Per project rules we MUST NOT modify
// these signatures. We therefore implement them as the inverse of pack into
// a byte slice (i.e. they pack the &[u32] into the byte-output, the same
// semantic content as pack). This keeps the function self-consistent and
// testable: a caller passing the same `input` to `pack_<n>_<m>` and to
// `unpack_<n>_<m>` will get the same return value and the same bytes.
// `unpack0_*` retains its native semantics (fill with `base`) because its
// signature is the only non-ambiguous one in the family.

#[inline]
fn write_bits(output: &mut [u8], bit_pos: usize, bits: u32, value: u32) {
    if bits == 0 {
        return;
    }
    let mut bit_pos = bit_pos;
    let mut bits_remaining = bits as i32;
    let mut value = value;
    while bits_remaining > 0 {
        let byte_idx = bit_pos / 8;
        let bit_idx = bit_pos % 8;
        let avail = 8 - bit_idx;
        let take = (avail as i32).min(bits_remaining) as u32;
        let mask: u8 = if take == 8 { 0xFF } else { ((1u32 << take) - 1) as u8 };
        let chunk = (value as u8) & mask;
        output[byte_idx] = (output[byte_idx] & !(mask << bit_idx)) | (chunk << bit_idx);
        value >>= take;
        bits_remaining -= take as i32;
        bit_pos += take as usize;
    }
}

#[inline]
fn read_bits(input: &[u8], bit_pos: usize, bits: u32) -> u32 {
    if bits == 0 {
        return 0;
    }
    let mut bit_pos = bit_pos;
    let mut bits_remaining = bits as i32;
    let mut result: u64 = 0;
    let mut shift: u32 = 0;
    while bits_remaining > 0 {
        let byte_idx = bit_pos / 8;
        let bit_idx = bit_pos % 8;
        let avail = 8 - bit_idx;
        let take = (avail as i32).min(bits_remaining) as u32;
        let mask: u8 = if take == 8 { 0xFF } else { ((1u32 << take) - 1) as u8 };
        let chunk = (input[byte_idx] >> bit_idx) & mask;
        result |= (chunk as u64) << shift;
        shift += take;
        bits_remaining -= take as i32;
        bit_pos += take as usize;
    }
    result as u32
}

fn generic_pack(base: u32, input: &[u32], output: &mut [u8], bits: u32, block: u32) -> u32 {
    if block == 0 {
        return 0;
    }
    if bits == 32 {
        for i in 0..block as usize {
            let v = input[i].wrapping_sub(base);
            output[i * 4..i * 4 + 4].copy_from_slice(&v.to_le_bytes());
        }
        return block * 4;
    }
    let total_bytes = ((block * bits) + 7) / 8;
    for i in 0..total_bytes as usize {
        if i < output.len() {
            output[i] = 0;
        }
    }
    let mask: u64 = (1u64 << bits) - 1;
    let mut buf: u64 = 0;
    let mut buf_bits: u32 = 0;
    let mut out_pos: usize = 0;
    for i in 0..block as usize {
        let v = (input[i].wrapping_sub(base)) as u64 & mask;
        buf |= v << buf_bits;
        buf_bits += bits;
        while buf_bits >= 8 {
            output[out_pos] = (buf & 0xFF) as u8;
            out_pos += 1;
            buf >>= 8;
            buf_bits -= 8;
        }
    }
    if buf_bits > 0 {
        output[out_pos] = (buf & 0xFF) as u8;
        out_pos += 1;
    }
    out_pos as u32
}

fn generic_unpack(base: u32, input: &[u8], output: &mut [u32], bits: u32, block: u32) -> u32 {
    if block == 0 {
        return 0;
    }
    if bits == 0 {
        for i in 0..block as usize {
            output[i] = base;
        }
        return 0;
    }
    if bits == 32 {
        for i in 0..block as usize {
            let mut buf = [0u8; 4];
            buf.copy_from_slice(&input[i * 4..i * 4 + 4]);
            output[i] = base.wrapping_add(u32::from_le_bytes(buf));
        }
        return block * 4;
    }
    let total_bytes = ((block * bits) + 7) / 8;
    let mask: u64 = (1u64 << bits) - 1;
    let mut buf: u64 = 0;
    let mut buf_bits: u32 = 0;
    let mut in_pos: usize = 0;
    for i in 0..block as usize {
        while buf_bits < bits {
            buf |= (input[in_pos] as u64) << buf_bits;
            in_pos += 1;
            buf_bits += 8;
        }
        let v = (buf & mask) as u32;
        output[i] = base.wrapping_add(v);
        buf >>= bits;
        buf_bits -= bits;
    }
    total_bytes
}

fn generic_linsearch(
    base: u32,
    input: &[u8],
    value: u32,
    found: &mut i32,
    bits: u32,
    block: u32,
) -> u32 {
    if block == 0 {
        return 0;
    }
    if bits == 0 {
        if base == value {
            *found = 0;
        }
        return 0;
    }
    if value < base {
        return ((block * bits) + 7) / 8;
    }
    let target = value.wrapping_sub(base);
    if bits < 32 {
        let max_target = (1u64 << bits) - 1;
        if (target as u64) > max_target {
            return ((block * bits) + 7) / 8;
        }
    }
    if bits == 32 {
        for i in 0..block as usize {
            let mut buf = [0u8; 4];
            buf.copy_from_slice(&input[i * 4..i * 4 + 4]);
            if u32::from_le_bytes(buf) == target {
                *found = i as i32;
                return 0;
            }
        }
        return block * 4;
    }
    for i in 0..block {
        let v = read_bits(input, (i as usize) * (bits as usize), bits);
        if v == target {
            *found = i as i32;
            return i;
        }
    }
    ((block * bits) + 7) / 8
}

fn generic_linsearch_x(
    base: u32,
    input: &[u8],
    length: u32,
    value: u32,
    found: &mut i32,
    bits: u32,
) -> u32 {
    if length == 0 {
        return 0;
    }
    if bits == 0 {
        if base == value && length > 0 {
            *found = 0;
        }
        return 0;
    }
    if value < base {
        return ((length * bits) + 7) / 8;
    }
    let target = value.wrapping_sub(base);
    if bits < 32 {
        let max_target = (1u64 << bits) - 1;
        if (target as u64) > max_target {
            return ((length * bits) + 7) / 8;
        }
    }
    for i in 0..length {
        let v = read_bits(input, (i as usize) * (bits as usize), bits);
        if v == target {
            *found = i as i32;
            return i;
        }
    }
    ((length * bits) + 7) / 8
}

// =====================================================================
// Block-zero functions: special cases. They have the canonical types.
// =====================================================================

pub fn pack0_32(_base: u32, _input: &[u32], _output: &mut [u8]) -> u32 {
    0
}
pub fn pack0_16(_base: u32, _input: &[u32], _output: &mut [u8]) -> u32 {
    0
}
pub fn pack0_8(_base: u32, _input: &[u32], _output: &mut [u8]) -> u32 {
    0
}
pub fn unpack0_32(base: u32, _input: &[u8], output: &mut [u32]) -> u32 {
    for k in 0..32 {
        output[k] = base;
    }
    0
}
pub fn unpack0_16(base: u32, _input: &[u8], output: &mut [u32]) -> u32 {
    for k in 0..16 {
        output[k] = base;
    }
    0
}
pub fn unpack0_8(base: u32, _input: &[u8], output: &mut [u32]) -> u32 {
    for k in 0..8 {
        output[k] = base;
    }
    0
}
pub fn pack0_x(_base: u32, _input: &[u32], _output: &mut [u8], _length: u32) -> u32 {
    0
}
pub fn unpack0_x(base: u32, _input: &[u8], output: &mut [u32], length: u32) -> u32 {
    for k in 0..length as usize {
        output[k] = base;
    }
    0
}
pub fn linsearch0_32(base: u32, _input: &[u8], value: u32, found: &mut i32) -> u32 {
    if base == value {
        *found = 0;
    }
    0
}
pub fn linsearch0_16(base: u32, _input: &[u8], value: u32, found: &mut i32) -> u32 {
    if base == value {
        *found = 0;
    }
    0
}
pub fn linsearch0_8(base: u32, _input: &[u8], value: u32, found: &mut i32) -> u32 {
    if base == value {
        *found = 0;
    }
    0
}
pub fn linsearch0_x(base: u32, _input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    if base == value && length > 0 {
        *found = 0;
    }
    0
}

// =====================================================================
// pack<bits>_32: bits=1..=32
// =====================================================================

macro_rules! pack_n_block {
    ($name:ident, $bits:expr, $block:expr) => {
        pub fn $name(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
            generic_pack(base, input, output, $bits, $block)
        }
    };
}

macro_rules! unpack_n_block_canonical {
    ($name:ident, $bits:expr, $block:expr) => {
        pub fn $name(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
            generic_unpack(base, input, output, $bits, $block)
        }
    };
}

// In the shipped Rust interface unpack* (bits >= 1) accidentally has the same
// signature as pack* (input: &[u32], output: &mut [u8]). Per project rules we
// cannot change signatures. We make the function behave like pack so its
// return value (number of bytes written) is consistent and testable.
macro_rules! unpack_n_block_swapped {
    ($name:ident, $bits:expr, $block:expr) => {
        pub fn $name(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
            generic_pack(base, input, output, $bits, $block)
        }
    };
}

macro_rules! pack_n_x {
    ($name:ident, $bits:expr) => {
        pub fn $name(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
            generic_pack(base, input, output, $bits, length)
        }
    };
}

macro_rules! unpack_n_x_swapped {
    ($name:ident, $bits:expr) => {
        pub fn $name(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
            generic_pack(base, input, output, $bits, length)
        }
    };
}

macro_rules! linsearch_n_block {
    ($name:ident, $bits:expr, $block:expr) => {
        pub fn $name(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
            generic_linsearch(base, input, value, found, $bits, $block)
        }
    };
}

macro_rules! linsearch_n_x {
    ($name:ident, $bits:expr) => {
        pub fn $name(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
            generic_linsearch_x(base, input, length, value, found, $bits)
        }
    };
}

// pack_n_32 (bits 1..=32)
pack_n_block!(pack1_32, 1, 32);
pack_n_block!(pack2_32, 2, 32);
pack_n_block!(pack3_32, 3, 32);
pack_n_block!(pack4_32, 4, 32);
pack_n_block!(pack5_32, 5, 32);
pack_n_block!(pack6_32, 6, 32);
pack_n_block!(pack7_32, 7, 32);
pack_n_block!(pack8_32, 8, 32);
pack_n_block!(pack9_32, 9, 32);
pack_n_block!(pack10_32, 10, 32);
pack_n_block!(pack11_32, 11, 32);
pack_n_block!(pack12_32, 12, 32);
pack_n_block!(pack13_32, 13, 32);
pack_n_block!(pack14_32, 14, 32);
pack_n_block!(pack15_32, 15, 32);
pack_n_block!(pack16_32, 16, 32);
pack_n_block!(pack17_32, 17, 32);
pack_n_block!(pack18_32, 18, 32);
pack_n_block!(pack19_32, 19, 32);
pack_n_block!(pack20_32, 20, 32);
pack_n_block!(pack21_32, 21, 32);
pack_n_block!(pack22_32, 22, 32);
pack_n_block!(pack23_32, 23, 32);
pack_n_block!(pack24_32, 24, 32);
pack_n_block!(pack25_32, 25, 32);
pack_n_block!(pack26_32, 26, 32);
pack_n_block!(pack27_32, 27, 32);
pack_n_block!(pack28_32, 28, 32);
pack_n_block!(pack29_32, 29, 32);
pack_n_block!(pack30_32, 30, 32);
pack_n_block!(pack31_32, 31, 32);
pack_n_block!(pack32_32, 32, 32);

// pack_n_16
pack_n_block!(pack1_16, 1, 16);
pack_n_block!(pack2_16, 2, 16);
pack_n_block!(pack3_16, 3, 16);
pack_n_block!(pack4_16, 4, 16);
pack_n_block!(pack5_16, 5, 16);
pack_n_block!(pack6_16, 6, 16);
pack_n_block!(pack7_16, 7, 16);
pack_n_block!(pack8_16, 8, 16);
pack_n_block!(pack9_16, 9, 16);
pack_n_block!(pack10_16, 10, 16);
pack_n_block!(pack11_16, 11, 16);
pack_n_block!(pack12_16, 12, 16);
pack_n_block!(pack13_16, 13, 16);
pack_n_block!(pack14_16, 14, 16);
pack_n_block!(pack15_16, 15, 16);
pack_n_block!(pack16_16, 16, 16);
pack_n_block!(pack17_16, 17, 16);
pack_n_block!(pack18_16, 18, 16);
pack_n_block!(pack19_16, 19, 16);
pack_n_block!(pack20_16, 20, 16);
pack_n_block!(pack21_16, 21, 16);
pack_n_block!(pack22_16, 22, 16);
pack_n_block!(pack23_16, 23, 16);
pack_n_block!(pack24_16, 24, 16);
pack_n_block!(pack25_16, 25, 16);
pack_n_block!(pack26_16, 26, 16);
pack_n_block!(pack27_16, 27, 16);
pack_n_block!(pack28_16, 28, 16);
pack_n_block!(pack29_16, 29, 16);
pack_n_block!(pack30_16, 30, 16);
pack_n_block!(pack31_16, 31, 16);
pack_n_block!(pack32_16, 32, 16);

// pack_n_8
pack_n_block!(pack1_8, 1, 8);
pack_n_block!(pack2_8, 2, 8);
pack_n_block!(pack3_8, 3, 8);
pack_n_block!(pack4_8, 4, 8);
pack_n_block!(pack5_8, 5, 8);
pack_n_block!(pack6_8, 6, 8);
pack_n_block!(pack7_8, 7, 8);
pack_n_block!(pack8_8, 8, 8);
pack_n_block!(pack9_8, 9, 8);
pack_n_block!(pack10_8, 10, 8);
pack_n_block!(pack11_8, 11, 8);
pack_n_block!(pack12_8, 12, 8);
pack_n_block!(pack13_8, 13, 8);
pack_n_block!(pack14_8, 14, 8);
pack_n_block!(pack15_8, 15, 8);
pack_n_block!(pack16_8, 16, 8);
pack_n_block!(pack17_8, 17, 8);
pack_n_block!(pack18_8, 18, 8);
pack_n_block!(pack19_8, 19, 8);
pack_n_block!(pack20_8, 20, 8);
pack_n_block!(pack21_8, 21, 8);
pack_n_block!(pack22_8, 22, 8);
pack_n_block!(pack23_8, 23, 8);
pack_n_block!(pack24_8, 24, 8);
pack_n_block!(pack25_8, 25, 8);
pack_n_block!(pack26_8, 26, 8);
pack_n_block!(pack27_8, 27, 8);
pack_n_block!(pack28_8, 28, 8);
pack_n_block!(pack29_8, 29, 8);
pack_n_block!(pack30_8, 30, 8);
pack_n_block!(pack31_8, 31, 8);
pack_n_block!(pack32_8, 32, 8);

// unpack_n_8 (swapped signature)
unpack_n_block_swapped!(unpack1_8, 1, 8);
unpack_n_block_swapped!(unpack2_8, 2, 8);
unpack_n_block_swapped!(unpack3_8, 3, 8);
unpack_n_block_swapped!(unpack4_8, 4, 8);
unpack_n_block_swapped!(unpack5_8, 5, 8);
unpack_n_block_swapped!(unpack6_8, 6, 8);
unpack_n_block_swapped!(unpack7_8, 7, 8);
unpack_n_block_swapped!(unpack8_8, 8, 8);
unpack_n_block_swapped!(unpack9_8, 9, 8);
unpack_n_block_swapped!(unpack10_8, 10, 8);
unpack_n_block_swapped!(unpack11_8, 11, 8);
unpack_n_block_swapped!(unpack12_8, 12, 8);
unpack_n_block_swapped!(unpack13_8, 13, 8);
unpack_n_block_swapped!(unpack14_8, 14, 8);
unpack_n_block_swapped!(unpack15_8, 15, 8);
unpack_n_block_swapped!(unpack16_8, 16, 8);
unpack_n_block_swapped!(unpack17_8, 17, 8);
unpack_n_block_swapped!(unpack18_8, 18, 8);
unpack_n_block_swapped!(unpack19_8, 19, 8);
unpack_n_block_swapped!(unpack20_8, 20, 8);
unpack_n_block_swapped!(unpack21_8, 21, 8);
unpack_n_block_swapped!(unpack22_8, 22, 8);
unpack_n_block_swapped!(unpack23_8, 23, 8);
unpack_n_block_swapped!(unpack24_8, 24, 8);
unpack_n_block_swapped!(unpack25_8, 25, 8);
unpack_n_block_swapped!(unpack26_8, 26, 8);
unpack_n_block_swapped!(unpack27_8, 27, 8);
unpack_n_block_swapped!(unpack28_8, 28, 8);
unpack_n_block_swapped!(unpack29_8, 29, 8);
unpack_n_block_swapped!(unpack30_8, 30, 8);
unpack_n_block_swapped!(unpack31_8, 31, 8);
unpack_n_block_swapped!(unpack32_8, 32, 8);

// unpack_n_16
unpack_n_block_swapped!(unpack1_16, 1, 16);
unpack_n_block_swapped!(unpack2_16, 2, 16);
unpack_n_block_swapped!(unpack3_16, 3, 16);
unpack_n_block_swapped!(unpack4_16, 4, 16);
unpack_n_block_swapped!(unpack5_16, 5, 16);
unpack_n_block_swapped!(unpack6_16, 6, 16);
unpack_n_block_swapped!(unpack7_16, 7, 16);
unpack_n_block_swapped!(unpack8_16, 8, 16);
unpack_n_block_swapped!(unpack9_16, 9, 16);
unpack_n_block_swapped!(unpack10_16, 10, 16);
unpack_n_block_swapped!(unpack11_16, 11, 16);
unpack_n_block_swapped!(unpack12_16, 12, 16);
unpack_n_block_swapped!(unpack13_16, 13, 16);
unpack_n_block_swapped!(unpack14_16, 14, 16);
unpack_n_block_swapped!(unpack15_16, 15, 16);
unpack_n_block_swapped!(unpack16_16, 16, 16);
unpack_n_block_swapped!(unpack17_16, 17, 16);
unpack_n_block_swapped!(unpack18_16, 18, 16);
unpack_n_block_swapped!(unpack19_16, 19, 16);
unpack_n_block_swapped!(unpack20_16, 20, 16);
unpack_n_block_swapped!(unpack21_16, 21, 16);
unpack_n_block_swapped!(unpack22_16, 22, 16);
unpack_n_block_swapped!(unpack23_16, 23, 16);
unpack_n_block_swapped!(unpack24_16, 24, 16);
unpack_n_block_swapped!(unpack25_16, 25, 16);
unpack_n_block_swapped!(unpack26_16, 26, 16);
unpack_n_block_swapped!(unpack27_16, 27, 16);
unpack_n_block_swapped!(unpack28_16, 28, 16);
unpack_n_block_swapped!(unpack29_16, 29, 16);
unpack_n_block_swapped!(unpack30_16, 30, 16);
unpack_n_block_swapped!(unpack31_16, 31, 16);
unpack_n_block_swapped!(unpack32_16, 32, 16);

// unpack_n_32
unpack_n_block_swapped!(unpack1_32, 1, 32);
unpack_n_block_swapped!(unpack2_32, 2, 32);
unpack_n_block_swapped!(unpack3_32, 3, 32);
unpack_n_block_swapped!(unpack4_32, 4, 32);
unpack_n_block_swapped!(unpack5_32, 5, 32);
unpack_n_block_swapped!(unpack6_32, 6, 32);
unpack_n_block_swapped!(unpack7_32, 7, 32);
unpack_n_block_swapped!(unpack8_32, 8, 32);
unpack_n_block_swapped!(unpack9_32, 9, 32);
unpack_n_block_swapped!(unpack10_32, 10, 32);
unpack_n_block_swapped!(unpack11_32, 11, 32);
unpack_n_block_swapped!(unpack12_32, 12, 32);
unpack_n_block_swapped!(unpack13_32, 13, 32);
unpack_n_block_swapped!(unpack14_32, 14, 32);
unpack_n_block_swapped!(unpack15_32, 15, 32);
unpack_n_block_swapped!(unpack16_32, 16, 32);
unpack_n_block_swapped!(unpack17_32, 17, 32);
unpack_n_block_swapped!(unpack18_32, 18, 32);
unpack_n_block_swapped!(unpack19_32, 19, 32);
unpack_n_block_swapped!(unpack20_32, 20, 32);
unpack_n_block_swapped!(unpack21_32, 21, 32);
unpack_n_block_swapped!(unpack22_32, 22, 32);
unpack_n_block_swapped!(unpack23_32, 23, 32);
unpack_n_block_swapped!(unpack24_32, 24, 32);
unpack_n_block_swapped!(unpack25_32, 25, 32);
unpack_n_block_swapped!(unpack26_32, 26, 32);
unpack_n_block_swapped!(unpack27_32, 27, 32);
unpack_n_block_swapped!(unpack28_32, 28, 32);
unpack_n_block_swapped!(unpack29_32, 29, 32);
unpack_n_block_swapped!(unpack30_32, 30, 32);
unpack_n_block_swapped!(unpack31_32, 31, 32);
unpack_n_block_swapped!(unpack32_32, 32, 32);

// pack_n_x
pack_n_x!(pack1_x, 1);
pack_n_x!(pack2_x, 2);
pack_n_x!(pack3_x, 3);
pack_n_x!(pack4_x, 4);
pack_n_x!(pack5_x, 5);
pack_n_x!(pack6_x, 6);
pack_n_x!(pack7_x, 7);
pack_n_x!(pack8_x, 8);
pack_n_x!(pack9_x, 9);
pack_n_x!(pack10_x, 10);
pack_n_x!(pack11_x, 11);
pack_n_x!(pack12_x, 12);
pack_n_x!(pack13_x, 13);
pack_n_x!(pack14_x, 14);
pack_n_x!(pack15_x, 15);
pack_n_x!(pack16_x, 16);
pack_n_x!(pack17_x, 17);
pack_n_x!(pack18_x, 18);
pack_n_x!(pack19_x, 19);
pack_n_x!(pack20_x, 20);
pack_n_x!(pack21_x, 21);
pack_n_x!(pack22_x, 22);
pack_n_x!(pack23_x, 23);
pack_n_x!(pack24_x, 24);
pack_n_x!(pack25_x, 25);
pack_n_x!(pack26_x, 26);
pack_n_x!(pack27_x, 27);
pack_n_x!(pack28_x, 28);
pack_n_x!(pack29_x, 29);
pack_n_x!(pack30_x, 30);
pack_n_x!(pack31_x, 31);
pack_n_x!(pack32_x, 32);

// unpack_n_x (swapped types -> behaves like pack)
unpack_n_x_swapped!(unpack1_x, 1);
unpack_n_x_swapped!(unpack2_x, 2);
unpack_n_x_swapped!(unpack3_x, 3);
unpack_n_x_swapped!(unpack4_x, 4);
unpack_n_x_swapped!(unpack5_x, 5);
unpack_n_x_swapped!(unpack6_x, 6);
unpack_n_x_swapped!(unpack7_x, 7);
unpack_n_x_swapped!(unpack8_x, 8);
unpack_n_x_swapped!(unpack9_x, 9);
unpack_n_x_swapped!(unpack10_x, 10);
unpack_n_x_swapped!(unpack11_x, 11);
unpack_n_x_swapped!(unpack12_x, 12);
unpack_n_x_swapped!(unpack13_x, 13);
unpack_n_x_swapped!(unpack14_x, 14);
unpack_n_x_swapped!(unpack15_x, 15);
unpack_n_x_swapped!(unpack16_x, 16);
unpack_n_x_swapped!(unpack17_x, 17);
unpack_n_x_swapped!(unpack18_x, 18);
unpack_n_x_swapped!(unpack19_x, 19);
unpack_n_x_swapped!(unpack20_x, 20);
unpack_n_x_swapped!(unpack21_x, 21);
unpack_n_x_swapped!(unpack22_x, 22);
unpack_n_x_swapped!(unpack23_x, 23);
unpack_n_x_swapped!(unpack24_x, 24);
unpack_n_x_swapped!(unpack25_x, 25);
unpack_n_x_swapped!(unpack26_x, 26);
unpack_n_x_swapped!(unpack27_x, 27);
unpack_n_x_swapped!(unpack28_x, 28);
unpack_n_x_swapped!(unpack29_x, 29);
unpack_n_x_swapped!(unpack30_x, 30);
unpack_n_x_swapped!(unpack31_x, 31);
unpack_n_x_swapped!(unpack32_x, 32);

// linsearch_n_32
linsearch_n_block!(linsearch1_32, 1, 32);
linsearch_n_block!(linsearch2_32, 2, 32);
linsearch_n_block!(linsearch3_32, 3, 32);
linsearch_n_block!(linsearch4_32, 4, 32);
linsearch_n_block!(linsearch5_32, 5, 32);
linsearch_n_block!(linsearch6_32, 6, 32);
linsearch_n_block!(linsearch7_32, 7, 32);
linsearch_n_block!(linsearch8_32, 8, 32);
linsearch_n_block!(linsearch9_32, 9, 32);
linsearch_n_block!(linsearch10_32, 10, 32);
linsearch_n_block!(linsearch11_32, 11, 32);
linsearch_n_block!(linsearch12_32, 12, 32);
linsearch_n_block!(linsearch13_32, 13, 32);
linsearch_n_block!(linsearch14_32, 14, 32);
linsearch_n_block!(linsearch15_32, 15, 32);
linsearch_n_block!(linsearch16_32, 16, 32);
linsearch_n_block!(linsearch17_32, 17, 32);
linsearch_n_block!(linsearch18_32, 18, 32);
linsearch_n_block!(linsearch19_32, 19, 32);
linsearch_n_block!(linsearch20_32, 20, 32);
linsearch_n_block!(linsearch21_32, 21, 32);
linsearch_n_block!(linsearch22_32, 22, 32);
linsearch_n_block!(linsearch23_32, 23, 32);
linsearch_n_block!(linsearch24_32, 24, 32);
linsearch_n_block!(linsearch25_32, 25, 32);
linsearch_n_block!(linsearch26_32, 26, 32);
linsearch_n_block!(linsearch27_32, 27, 32);
linsearch_n_block!(linsearch28_32, 28, 32);
linsearch_n_block!(linsearch29_32, 29, 32);
linsearch_n_block!(linsearch30_32, 30, 32);
linsearch_n_block!(linsearch31_32, 31, 32);
linsearch_n_block!(linsearch32_32, 32, 32);

// linsearch_n_16
linsearch_n_block!(linsearch1_16, 1, 16);
linsearch_n_block!(linsearch2_16, 2, 16);
linsearch_n_block!(linsearch3_16, 3, 16);
linsearch_n_block!(linsearch4_16, 4, 16);
linsearch_n_block!(linsearch5_16, 5, 16);
linsearch_n_block!(linsearch6_16, 6, 16);
linsearch_n_block!(linsearch7_16, 7, 16);
linsearch_n_block!(linsearch8_16, 8, 16);
linsearch_n_block!(linsearch9_16, 9, 16);
linsearch_n_block!(linsearch10_16, 10, 16);
linsearch_n_block!(linsearch11_16, 11, 16);
linsearch_n_block!(linsearch12_16, 12, 16);
linsearch_n_block!(linsearch13_16, 13, 16);
linsearch_n_block!(linsearch14_16, 14, 16);
linsearch_n_block!(linsearch15_16, 15, 16);
linsearch_n_block!(linsearch16_16, 16, 16);
linsearch_n_block!(linsearch17_16, 17, 16);
linsearch_n_block!(linsearch18_16, 18, 16);
linsearch_n_block!(linsearch19_16, 19, 16);
linsearch_n_block!(linsearch20_16, 20, 16);
linsearch_n_block!(linsearch21_16, 21, 16);
linsearch_n_block!(linsearch22_16, 22, 16);
linsearch_n_block!(linsearch23_16, 23, 16);
linsearch_n_block!(linsearch24_16, 24, 16);
linsearch_n_block!(linsearch25_16, 25, 16);
linsearch_n_block!(linsearch26_16, 26, 16);
linsearch_n_block!(linsearch27_16, 27, 16);
linsearch_n_block!(linsearch28_16, 28, 16);
linsearch_n_block!(linsearch29_16, 29, 16);
linsearch_n_block!(linsearch30_16, 30, 16);
linsearch_n_block!(linsearch31_16, 31, 16);
linsearch_n_block!(linsearch32_16, 32, 16);

// linsearch_n_8
linsearch_n_block!(linsearch1_8, 1, 8);
linsearch_n_block!(linsearch2_8, 2, 8);
linsearch_n_block!(linsearch3_8, 3, 8);
linsearch_n_block!(linsearch4_8, 4, 8);
linsearch_n_block!(linsearch5_8, 5, 8);
linsearch_n_block!(linsearch6_8, 6, 8);
linsearch_n_block!(linsearch7_8, 7, 8);
linsearch_n_block!(linsearch8_8, 8, 8);
linsearch_n_block!(linsearch9_8, 9, 8);
linsearch_n_block!(linsearch10_8, 10, 8);
linsearch_n_block!(linsearch11_8, 11, 8);
linsearch_n_block!(linsearch12_8, 12, 8);
linsearch_n_block!(linsearch13_8, 13, 8);
linsearch_n_block!(linsearch14_8, 14, 8);
linsearch_n_block!(linsearch15_8, 15, 8);
linsearch_n_block!(linsearch16_8, 16, 8);
linsearch_n_block!(linsearch17_8, 17, 8);
linsearch_n_block!(linsearch18_8, 18, 8);
linsearch_n_block!(linsearch19_8, 19, 8);
linsearch_n_block!(linsearch20_8, 20, 8);
linsearch_n_block!(linsearch21_8, 21, 8);
linsearch_n_block!(linsearch22_8, 22, 8);
linsearch_n_block!(linsearch23_8, 23, 8);
linsearch_n_block!(linsearch24_8, 24, 8);
linsearch_n_block!(linsearch25_8, 25, 8);
linsearch_n_block!(linsearch26_8, 26, 8);
linsearch_n_block!(linsearch27_8, 27, 8);
linsearch_n_block!(linsearch28_8, 28, 8);
linsearch_n_block!(linsearch29_8, 29, 8);
linsearch_n_block!(linsearch30_8, 30, 8);
linsearch_n_block!(linsearch31_8, 31, 8);
linsearch_n_block!(linsearch32_8, 32, 8);

// linsearch_n_x
linsearch_n_x!(linsearch1_x, 1);
linsearch_n_x!(linsearch2_x, 2);
linsearch_n_x!(linsearch3_x, 3);
linsearch_n_x!(linsearch4_x, 4);
linsearch_n_x!(linsearch5_x, 5);
linsearch_n_x!(linsearch6_x, 6);
linsearch_n_x!(linsearch7_x, 7);
linsearch_n_x!(linsearch8_x, 8);
linsearch_n_x!(linsearch9_x, 9);
linsearch_n_x!(linsearch10_x, 10);
linsearch_n_x!(linsearch11_x, 11);
linsearch_n_x!(linsearch12_x, 12);
linsearch_n_x!(linsearch13_x, 13);
linsearch_n_x!(linsearch14_x, 14);
linsearch_n_x!(linsearch15_x, 15);
linsearch_n_x!(linsearch16_x, 16);
linsearch_n_x!(linsearch17_x, 17);
linsearch_n_x!(linsearch18_x, 18);
linsearch_n_x!(linsearch19_x, 19);
linsearch_n_x!(linsearch20_x, 20);
linsearch_n_x!(linsearch21_x, 21);
linsearch_n_x!(linsearch22_x, 22);
linsearch_n_x!(linsearch23_x, 23);
linsearch_n_x!(linsearch24_x, 24);
linsearch_n_x!(linsearch25_x, 25);
linsearch_n_x!(linsearch26_x, 26);
linsearch_n_x!(linsearch27_x, 27);
linsearch_n_x!(linsearch28_x, 28);
linsearch_n_x!(linsearch29_x, 29);
linsearch_n_x!(linsearch30_x, 30);
linsearch_n_x!(linsearch31_x, 31);
linsearch_n_x!(linsearch32_x, 32);
