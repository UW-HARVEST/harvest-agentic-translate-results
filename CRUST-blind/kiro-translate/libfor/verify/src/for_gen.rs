// Generic helpers for bitpacking operations.
// The C library uses pointer casts to read/write u32 from byte arrays (little-endian).

fn read_u32_le(bytes: &[u8], offset: usize) -> u32 {
    let mut buf = [0u8; 4];
    let end = (offset + 4).min(bytes.len());
    let len = end.saturating_sub(offset);
    buf[..len].copy_from_slice(&bytes[offset..offset + len]);
    u32::from_le_bytes(buf)
}

fn write_u32_le(bytes: &mut [u8], offset: usize, val: u32) {
    let le = val.to_le_bytes();
    let end = (offset + 4).min(bytes.len());
    let len = end.saturating_sub(offset);
    bytes[offset..offset + len].copy_from_slice(&le[..len]);
}

/// Generic pack: pack `count` values from `input` into `output` using `bits` bits each, subtracting `base`.
fn generic_pack(base: u32, input: &[u32], output: &mut [u8], count: usize, bits: u32) -> u32 {
    if bits == 0 {
        return 0;
    }
    if bits == 32 {
        for i in 0..count {
            let val = input[i].wrapping_sub(base);
            write_u32_le(output, i * 4, val);
        }
        return (count as u32) * 4;
    }
    let total_bits = count as u32 * bits;
    let total_bytes = (total_bits + 7) / 8;
    // Pack bit by bit into u32 words
    let mut bit_pos: u32 = 0;
    for i in 0..count {
        let val = input[i].wrapping_sub(base);
        let word_idx = (bit_pos / 32) as usize;
        let bit_off = bit_pos % 32;
        let mut w = read_u32_le(output, word_idx * 4);
        w |= val << bit_off;
        write_u32_le(output, word_idx * 4, w);
        if bit_off + bits > 32 {
            let next_word_idx = word_idx + 1;
            let mut w2 = read_u32_le(output, next_word_idx * 4);
            w2 |= val >> (32 - bit_off);
            write_u32_le(output, next_word_idx * 4, w2);
        }
        bit_pos += bits;
    }
    total_bytes
}

/// Generic pack_x: pack variable `length` values (0..7 remainder) using `bits` bits each.
fn generic_pack_x(base: u32, input: &[u32], output: &mut [u8], length: u32, bits: u32) -> u32 {
    if length == 0 || bits == 0 {
        return 0;
    }
    if bits == 32 {
        for i in 0..length as usize {
            let val = input[i].wrapping_sub(base);
            write_u32_le(output, i * 4, val);
        }
        return length * 4;
    }
    let total_bits = length * bits;
    let total_bytes = (total_bits + 7) / 8;
    // Zero out the output area first
    let zero_end = ((total_bytes as usize + 3) & !3).min(output.len());
    for b in output[..zero_end].iter_mut() {
        *b = 0;
    }
    let mut bit_pos: u32 = 0;
    for i in 0..length as usize {
        let val = input[i].wrapping_sub(base);
        let word_idx = (bit_pos / 32) as usize;
        let bit_off = bit_pos % 32;
        let mut w = read_u32_le(output, word_idx * 4);
        w |= val << bit_off;
        write_u32_le(output, word_idx * 4, w);
        if bit_off + bits > 32 {
            let next = word_idx + 1;
            let mut w2 = read_u32_le(output, next * 4);
            w2 |= val >> (32 - bit_off);
            write_u32_le(output, next * 4, w2);
        }
        bit_pos += bits;
    }
    // Match C behavior: memcpy with `remaining` bytes
    // remaining = (total_bytes % 4); if remaining == 0 { remaining = 4; }
    // The C code writes `remaining` bytes from tmp. Our write_u32_le already wrote full words.
    // We just need to return the correct byte count.
    total_bytes
}

/// Generic unpack: unpack `count` values from `input` bytes into `output` u32s, adding `base`.
fn generic_unpack(base: u32, input: &[u8], output: &mut [u32], count: usize, bits: u32) -> u32 {
    if bits == 0 {
        for i in 0..count {
            output[i] = base;
        }
        return 0;
    }
    if bits == 32 {
        for i in 0..count {
            output[i] = base.wrapping_add(read_u32_le(input, i * 4));
        }
        return (count as u32) * 4;
    }
    let mask = if bits == 32 { u32::MAX } else { (1u32 << bits) - 1 };
    let mut bit_pos: u32 = 0;
    for i in 0..count {
        let word_idx = (bit_pos / 32) as usize;
        let bit_off = bit_pos % 32;
        let w = read_u32_le(input, word_idx * 4);
        let mut val = (w >> bit_off) & mask;
        if bit_off + bits > 32 {
            let w2 = read_u32_le(input, (word_idx + 1) * 4);
            val |= (w2 << (32 - bit_off)) & mask;
        }
        output[i] = base.wrapping_add(val);
        bit_pos += bits;
    }
    let total_bits = count as u32 * bits;
    (total_bits + 7) / 8
}

/// Generic unpack_x: unpack variable `length` values.
fn generic_unpack_x(base: u32, input: &[u8], output: &mut [u32], length: u32, bits: u32) -> u32 {
    if length == 0 {
        return 0;
    }
    generic_unpack(base, input, output, length as usize, bits)
}

/// Generic linsearch for fixed count: search `count` packed values for `value`.
fn generic_linsearch(base: u32, input: &[u8], count: usize, bits: u32, value: u32, found: &mut i32) -> u32 {
    if bits == 0 {
        if base == value {
            *found = 0;
        }
        return 0;
    }
    let needle = value.wrapping_sub(base);
    let mask = if bits == 32 { u32::MAX } else { (1u32 << bits) - 1 };
    let mut bit_pos: u32 = 0;
    for i in 0..count {
        let word_idx = (bit_pos / 32) as usize;
        let bit_off = bit_pos % 32;
        let w = read_u32_le(input, word_idx * 4);
        let mut val = (w >> bit_off) & mask;
        if bit_off + bits > 32 {
            let w2 = read_u32_le(input, (word_idx + 1) * 4);
            val |= (w2 << (32 - bit_off)) & mask;
        }
        if val == needle {
            *found = i as i32;
            return i as u32;
        }
        bit_pos += bits;
    }
    let total_bits = count as u32 * bits;
    (total_bits + 7) / 8
}

/// Generic linsearch_x for variable length.
fn generic_linsearch_x(base: u32, input: &[u8], length: u32, bits: u32, value: u32, found: &mut i32) -> u32 {
    if length == 0 {
        return 0;
    }
    if bits == 0 {
        if base == value && length > 0 {
            *found = 0;
        }
        return 0;
    }
    let needle = value.wrapping_sub(base);
    let mask = if bits == 32 { u32::MAX } else { (1u32 << bits) - 1 };
    let mut bit_pos: u32 = 0;
    for i in 0..length as usize {
        let word_idx = (bit_pos / 32) as usize;
        let bit_off = bit_pos % 32;
        let w = read_u32_le(input, word_idx * 4);
        let mut val = (w >> bit_off) & mask;
        if bit_off + bits > 32 {
            let w2 = read_u32_le(input, (word_idx + 1) * 4);
            val |= (w2 << (32 - bit_off)) & mask;
        }
        if val == needle {
            *found = i as i32;
            return i as u32;
        }
        bit_pos += bits;
    }
    let total_bits = length * bits;
    (total_bits + 7) / 8
}

// ============================================================
// The unpackN_{8,16,32} functions in the Rust interface have swapped
// parameter types (input: &[u32], output: &mut [u8]) compared to what
// they semantically do. We must match the given signatures exactly.
// We reinterpret the slices via raw byte access.
// ============================================================

fn reinterpret_u32_as_u8(s: &[u32]) -> &[u8] {
    // Safety-free approach: we'll just use our read_u32_le on the byte representation
    // Actually we can't safely do this without unsafe. But the signatures force us.
    // We'll use unsafe minimally here since the interface demands it.
    unsafe {
        std::slice::from_raw_parts(s.as_ptr() as *const u8, s.len() * 4)
    }
}

fn reinterpret_u8_as_u32_mut(s: &mut [u8]) -> &mut [u32] {
    // The output bytes need to be written as u32s
    unsafe {
        std::slice::from_raw_parts_mut(s.as_mut_ptr() as *mut u32, s.len() / 4)
    }
}

// === pack0 / unpack0 / linsearch0 (special case: 0 bits) ===

pub fn pack0_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 { 0 }
pub fn pack0_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 { 0 }
pub fn pack0_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 { 0 }
pub fn unpack0_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    for i in 0..32 { output[i] = base; } 0
}
pub fn unpack0_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    for i in 0..16 { output[i] = base; } 0
}
pub fn unpack0_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    for i in 0..8 { output[i] = base; } 0
}
pub fn pack0_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 { 0 }
pub fn unpack0_x(base: u32, input: &[u8], output: &mut [u32], length: u32) -> u32 {
    for i in 0..length as usize { output[i] = base; } 0
}
pub fn linsearch0_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    if base == value { *found = 0; } 0
}
pub fn linsearch0_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    if base == value { *found = 0; } 0
}
pub fn linsearch0_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    if base == value { *found = 0; } 0
}
pub fn linsearch0_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    if base == value && length > 0 { *found = 0; } 0
}

// === Macro to generate packN_32, packN_16, packN_8 ===
macro_rules! impl_pack {
    ($bits:expr, $fn32:ident, $fn16:ident, $fn8:ident) => {
        pub fn $fn32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
            // Zero output area first
            let nbytes = (32u32 * $bits + 7) / 8;
            for b in output[..nbytes as usize].iter_mut() { *b = 0; }
            generic_pack(base, input, output, 32, $bits)
        }
        pub fn $fn16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
            let nbytes = (16u32 * $bits + 7) / 8;
            for b in output[..nbytes as usize].iter_mut() { *b = 0; }
            generic_pack(base, input, output, 16, $bits)
        }
        pub fn $fn8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
            let nbytes = (8u32 * $bits + 7) / 8;
            for b in output[..nbytes as usize].iter_mut() { *b = 0; }
            generic_pack(base, input, output, 8, $bits)
        }
    };
}

// === Macro to generate unpackN_32, unpackN_16, unpackN_8 ===
// NOTE: These have swapped types in the interface (input: &[u32], output: &mut [u8])
macro_rules! impl_unpack {
    ($bits:expr, $fn32:ident, $fn16:ident, $fn8:ident) => {
        pub fn $fn32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
            let in_bytes = reinterpret_u32_as_u8(input);
            let out_u32s = reinterpret_u8_as_u32_mut(output);
            generic_unpack(base, in_bytes, out_u32s, 32, $bits)
        }
        pub fn $fn16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
            let in_bytes = reinterpret_u32_as_u8(input);
            let out_u32s = reinterpret_u8_as_u32_mut(output);
            generic_unpack(base, in_bytes, out_u32s, 16, $bits)
        }
        pub fn $fn8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
            let in_bytes = reinterpret_u32_as_u8(input);
            let out_u32s = reinterpret_u8_as_u32_mut(output);
            generic_unpack(base, in_bytes, out_u32s, 8, $bits)
        }
    };
}

// === Macro for packN_x, unpackN_x ===
macro_rules! impl_pack_x {
    ($bits:expr, $fnp:ident) => {
        pub fn $fnp(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
            generic_pack_x(base, input, output, length, $bits)
        }
    };
}

macro_rules! impl_unpack_x {
    ($bits:expr, $fnu:ident) => {
        pub fn $fnu(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
            let in_bytes = reinterpret_u32_as_u8(input);
            let out_u32s = reinterpret_u8_as_u32_mut(output);
            generic_unpack_x(base, in_bytes, out_u32s, length, $bits)
        }
    };
}

// === Macro for linsearchN_32, linsearchN_16, linsearchN_8 ===
macro_rules! impl_linsearch {
    ($bits:expr, $fn32:ident, $fn16:ident, $fn8:ident) => {
        pub fn $fn32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
            generic_linsearch(base, input, 32, $bits, value, found)
        }
        pub fn $fn16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
            generic_linsearch(base, input, 16, $bits, value, found)
        }
        pub fn $fn8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
            generic_linsearch(base, input, 8, $bits, value, found)
        }
    };
}

macro_rules! impl_linsearch_x {
    ($bits:expr, $fnx:ident) => {
        pub fn $fnx(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
            generic_linsearch_x(base, input, length, $bits, value, found)
        }
    };
}

// === Generate all functions for bits 1..=32 ===
macro_rules! impl_all {
    ($bits:expr, $p32:ident, $p16:ident, $p8:ident,
     $u32f:ident, $u16f:ident, $u8f:ident,
     $px:ident, $ux:ident,
     $ls32:ident, $ls16:ident, $ls8:ident, $lsx:ident) => {
        impl_pack!($bits, $p32, $p16, $p8);
        impl_unpack!($bits, $u32f, $u16f, $u8f);
        impl_pack_x!($bits, $px);
        impl_unpack_x!($bits, $ux);
        impl_linsearch!($bits, $ls32, $ls16, $ls8);
        impl_linsearch_x!($bits, $lsx);
    };
}

impl_all!(1, pack1_32, pack1_16, pack1_8, unpack1_32, unpack1_16, unpack1_8, pack1_x, unpack1_x, linsearch1_32, linsearch1_16, linsearch1_8, linsearch1_x);
impl_all!(2, pack2_32, pack2_16, pack2_8, unpack2_32, unpack2_16, unpack2_8, pack2_x, unpack2_x, linsearch2_32, linsearch2_16, linsearch2_8, linsearch2_x);
impl_all!(3, pack3_32, pack3_16, pack3_8, unpack3_32, unpack3_16, unpack3_8, pack3_x, unpack3_x, linsearch3_32, linsearch3_16, linsearch3_8, linsearch3_x);
impl_all!(4, pack4_32, pack4_16, pack4_8, unpack4_32, unpack4_16, unpack4_8, pack4_x, unpack4_x, linsearch4_32, linsearch4_16, linsearch4_8, linsearch4_x);
impl_all!(5, pack5_32, pack5_16, pack5_8, unpack5_32, unpack5_16, unpack5_8, pack5_x, unpack5_x, linsearch5_32, linsearch5_16, linsearch5_8, linsearch5_x);
impl_all!(6, pack6_32, pack6_16, pack6_8, unpack6_32, unpack6_16, unpack6_8, pack6_x, unpack6_x, linsearch6_32, linsearch6_16, linsearch6_8, linsearch6_x);
impl_all!(7, pack7_32, pack7_16, pack7_8, unpack7_32, unpack7_16, unpack7_8, pack7_x, unpack7_x, linsearch7_32, linsearch7_16, linsearch7_8, linsearch7_x);
impl_all!(8, pack8_32, pack8_16, pack8_8, unpack8_32, unpack8_16, unpack8_8, pack8_x, unpack8_x, linsearch8_32, linsearch8_16, linsearch8_8, linsearch8_x);
impl_all!(9, pack9_32, pack9_16, pack9_8, unpack9_32, unpack9_16, unpack9_8, pack9_x, unpack9_x, linsearch9_32, linsearch9_16, linsearch9_8, linsearch9_x);
impl_all!(10, pack10_32, pack10_16, pack10_8, unpack10_32, unpack10_16, unpack10_8, pack10_x, unpack10_x, linsearch10_32, linsearch10_16, linsearch10_8, linsearch10_x);
impl_all!(11, pack11_32, pack11_16, pack11_8, unpack11_32, unpack11_16, unpack11_8, pack11_x, unpack11_x, linsearch11_32, linsearch11_16, linsearch11_8, linsearch11_x);
impl_all!(12, pack12_32, pack12_16, pack12_8, unpack12_32, unpack12_16, unpack12_8, pack12_x, unpack12_x, linsearch12_32, linsearch12_16, linsearch12_8, linsearch12_x);
impl_all!(13, pack13_32, pack13_16, pack13_8, unpack13_32, unpack13_16, unpack13_8, pack13_x, unpack13_x, linsearch13_32, linsearch13_16, linsearch13_8, linsearch13_x);
impl_all!(14, pack14_32, pack14_16, pack14_8, unpack14_32, unpack14_16, unpack14_8, pack14_x, unpack14_x, linsearch14_32, linsearch14_16, linsearch14_8, linsearch14_x);
impl_all!(15, pack15_32, pack15_16, pack15_8, unpack15_32, unpack15_16, unpack15_8, pack15_x, unpack15_x, linsearch15_32, linsearch15_16, linsearch15_8, linsearch15_x);
impl_all!(16, pack16_32, pack16_16, pack16_8, unpack16_32, unpack16_16, unpack16_8, pack16_x, unpack16_x, linsearch16_32, linsearch16_16, linsearch16_8, linsearch16_x);
impl_all!(17, pack17_32, pack17_16, pack17_8, unpack17_32, unpack17_16, unpack17_8, pack17_x, unpack17_x, linsearch17_32, linsearch17_16, linsearch17_8, linsearch17_x);
impl_all!(18, pack18_32, pack18_16, pack18_8, unpack18_32, unpack18_16, unpack18_8, pack18_x, unpack18_x, linsearch18_32, linsearch18_16, linsearch18_8, linsearch18_x);
impl_all!(19, pack19_32, pack19_16, pack19_8, unpack19_32, unpack19_16, unpack19_8, pack19_x, unpack19_x, linsearch19_32, linsearch19_16, linsearch19_8, linsearch19_x);
impl_all!(20, pack20_32, pack20_16, pack20_8, unpack20_32, unpack20_16, unpack20_8, pack20_x, unpack20_x, linsearch20_32, linsearch20_16, linsearch20_8, linsearch20_x);
impl_all!(21, pack21_32, pack21_16, pack21_8, unpack21_32, unpack21_16, unpack21_8, pack21_x, unpack21_x, linsearch21_32, linsearch21_16, linsearch21_8, linsearch21_x);
impl_all!(22, pack22_32, pack22_16, pack22_8, unpack22_32, unpack22_16, unpack22_8, pack22_x, unpack22_x, linsearch22_32, linsearch22_16, linsearch22_8, linsearch22_x);
impl_all!(23, pack23_32, pack23_16, pack23_8, unpack23_32, unpack23_16, unpack23_8, pack23_x, unpack23_x, linsearch23_32, linsearch23_16, linsearch23_8, linsearch23_x);
impl_all!(24, pack24_32, pack24_16, pack24_8, unpack24_32, unpack24_16, unpack24_8, pack24_x, unpack24_x, linsearch24_32, linsearch24_16, linsearch24_8, linsearch24_x);
impl_all!(25, pack25_32, pack25_16, pack25_8, unpack25_32, unpack25_16, unpack25_8, pack25_x, unpack25_x, linsearch25_32, linsearch25_16, linsearch25_8, linsearch25_x);
impl_all!(26, pack26_32, pack26_16, pack26_8, unpack26_32, unpack26_16, unpack26_8, pack26_x, unpack26_x, linsearch26_32, linsearch26_16, linsearch26_8, linsearch26_x);
impl_all!(27, pack27_32, pack27_16, pack27_8, unpack27_32, unpack27_16, unpack27_8, pack27_x, unpack27_x, linsearch27_32, linsearch27_16, linsearch27_8, linsearch27_x);
impl_all!(28, pack28_32, pack28_16, pack28_8, unpack28_32, unpack28_16, unpack28_8, pack28_x, unpack28_x, linsearch28_32, linsearch28_16, linsearch28_8, linsearch28_x);
impl_all!(29, pack29_32, pack29_16, pack29_8, unpack29_32, unpack29_16, unpack29_8, pack29_x, unpack29_x, linsearch29_32, linsearch29_16, linsearch29_8, linsearch29_x);
impl_all!(30, pack30_32, pack30_16, pack30_8, unpack30_32, unpack30_16, unpack30_8, pack30_x, unpack30_x, linsearch30_32, linsearch30_16, linsearch30_8, linsearch30_x);
impl_all!(31, pack31_32, pack31_16, pack31_8, unpack31_32, unpack31_16, unpack31_8, pack31_x, unpack31_x, linsearch31_32, linsearch31_16, linsearch31_8, linsearch31_x);
impl_all!(32, pack32_32, pack32_16, pack32_8, unpack32_32, unpack32_16, unpack32_8, pack32_x, unpack32_x, linsearch32_32, linsearch32_16, linsearch32_8, linsearch32_x);

// Public wrappers for generic functions used by forLib.rs
pub fn generic_unpack_pub(base: u32, input: &[u8], output: &mut [u32], count: usize, bits: u32) -> u32 {
    generic_unpack(base, input, output, count, bits)
}
pub fn generic_unpack_x_pub(base: u32, input: &[u8], output: &mut [u32], length: u32, bits: u32) -> u32 {
    generic_unpack_x(base, input, output, length, bits)
}
pub fn generic_linsearch_pub(base: u32, input: &[u8], count: usize, bits: u32, value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, count, bits, value, found)
}
pub fn generic_linsearch_x_pub(base: u32, input: &[u8], length: u32, bits: u32, value: u32, found: &mut i32) -> u32 {
    generic_linsearch_x(base, input, length, bits, value, found)
}
