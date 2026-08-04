// Frame-of-Reference encoding helpers.
//
// The C source `c_src/for-gen.c` contains hundreds of hand-unrolled
// pack/unpack/linsearch functions. The Rust implementation below uses generic
// implementations that match the C semantics bit-for-bit.

// ---------- bit stream helpers ----------

/// Pack `n` values with `bits` bits each into the output byte stream.
/// Returns the number of bytes written.
fn generic_pack(n: usize, bits: u32, base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    if bits == 0 || n == 0 {
        return 0;
    }
    let total_bytes = ((n as u32 * bits) + 7) / 8;
    // Zero-out the destination bytes (the C version overwrites complete u32s
    // with `*out = tmp`; the trailing partial word is overwritten by memcpy
    // with `remaining` bytes from a tmp accumulator that is zero-initialized
    // for the bits past the last value, so the result is the same).
    for byte in output[..total_bytes as usize].iter_mut() {
        *byte = 0;
    }

    let mut bit_pos: u64 = 0;
    for i in 0..n {
        let v = input[i].wrapping_sub(base);
        let mut bits_left = bits;
        let mut value = v;
        while bits_left > 0 {
            let byte_index = (bit_pos / 8) as usize;
            let bit_in_byte = (bit_pos % 8) as u32;
            let space_in_byte = 8 - bit_in_byte;
            let chunk = bits_left.min(space_in_byte);
            let mask = if chunk == 32 {
                0xFFFFFFFFu32
            } else {
                (1u32 << chunk) - 1
            };
            let chunk_bits = (value & mask) as u8;
            output[byte_index] |= chunk_bits << bit_in_byte;
            value >>= chunk;
            bits_left -= chunk;
            bit_pos += chunk as u64;
        }
    }
    total_bytes
}

/// Unpack `n` values with `bits` bits each from the input byte stream into
/// `output[0..n]`. Returns the number of bytes consumed.
fn generic_unpack(n: usize, bits: u32, base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    if n == 0 {
        return 0;
    }
    if bits == 0 {
        for i in 0..n {
            output[i] = base;
        }
        return 0;
    }
    let total_bytes = ((n as u32 * bits) + 7) / 8;

    let mut bit_pos: u64 = 0;
    for i in 0..n {
        let mut value: u32 = 0;
        let mut bits_collected = 0u32;
        while bits_collected < bits {
            let byte_index = (bit_pos / 8) as usize;
            let bit_in_byte = (bit_pos % 8) as u32;
            let space_in_byte = 8 - bit_in_byte;
            let chunk = (bits - bits_collected).min(space_in_byte);
            let byte_val = if byte_index < input.len() {
                input[byte_index]
            } else {
                0
            };
            let mask = if chunk == 8 { 0xFFu8 } else { (1u8 << chunk) - 1 };
            let chunk_bits = ((byte_val >> bit_in_byte) & mask) as u32;
            value |= chunk_bits << bits_collected;
            bits_collected += chunk;
            bit_pos += chunk as u64;
        }
        output[i] = base.wrapping_add(value);
    }
    total_bytes
}

/// Linear search through `n` packed values, looking for `value`.
/// Returns the bytes consumed; sets `*found` to the index if found.
fn generic_linsearch(
    n: usize,
    bits: u32,
    base: u32,
    input: &[u8],
    value: u32,
    found: &mut i32,
) -> u32 {
    if n == 0 {
        return 0;
    }
    if bits == 0 {
        if base == value {
            *found = 0;
        }
        return 0;
    }
    let total_bytes = ((n as u32 * bits) + 7) / 8;
    let target = value.wrapping_sub(base);

    let mut bit_pos: u64 = 0;
    for i in 0..n {
        let mut v: u32 = 0;
        let mut bits_collected = 0u32;
        while bits_collected < bits {
            let byte_index = (bit_pos / 8) as usize;
            let bit_in_byte = (bit_pos % 8) as u32;
            let space_in_byte = 8 - bit_in_byte;
            let chunk = (bits - bits_collected).min(space_in_byte);
            let byte_val = if byte_index < input.len() {
                input[byte_index]
            } else {
                0
            };
            let mask = if chunk == 8 { 0xFFu8 } else { (1u8 << chunk) - 1 };
            let chunk_bits = ((byte_val >> bit_in_byte) & mask) as u32;
            v |= chunk_bits << bits_collected;
            bits_collected += chunk;
            bit_pos += chunk as u64;
        }
        if v == target {
            *found = i as i32;
            // C code returns the bit_pos so far rounded up to bytes when
            // matching exactly (it returns the byte offset where this value
            // started). However, the high-level for_linear_search_bits checks
            // `if found >= 0` BEFORE adding the consumed bytes for this
            // block. So returning 0 / the current consumed bytes is fine —
            // the higher level wrapper looks at `found` immediately.
            return 0;
        }
    }
    total_bytes
}

// ---------- public function bodies ----------

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

// Macro to define a single pack function
macro_rules! def_pack {
    ($name:ident, $count:expr, $bits:expr) => {
        pub fn $name(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
            generic_pack($count, $bits, base, input, output)
        }
    };
}

// Macro to define a single unpack function (note: signature uses (&[u32], &mut [u8])
// per the source file convention for non-zero bit unpacks).
macro_rules! def_unpack {
    ($name:ident, $count:expr, $bits:expr) => {
        pub fn $name(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
            // Reinterpret u32 input as a byte stream (little-endian) then decode.
            let in_bytes: Vec<u8> = input
                .iter()
                .flat_map(|w| w.to_le_bytes().into_iter())
                .collect();
            let mut out_words = vec![0u32; $count];
            let consumed = generic_unpack($count, $bits, base, &in_bytes, &mut out_words);
            for (i, &w) in out_words.iter().enumerate() {
                let bytes = w.to_le_bytes();
                let off = i * 4;
                if off + 4 <= output.len() {
                    output[off..off + 4].copy_from_slice(&bytes);
                }
            }
            consumed
        }
    };
}

// Macro for linsearch
macro_rules! def_linsearch {
    ($name:ident, $count:expr, $bits:expr) => {
        pub fn $name(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
            generic_linsearch($count, $bits, base, input, value, found)
        }
    };
}

// Macro for pack_x
macro_rules! def_pack_x {
    ($name:ident, $bits:expr) => {
        pub fn $name(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
            generic_pack(length as usize, $bits, base, input, output)
        }
    };
}

// Macro for unpack_x (signature uses &[u32], &mut [u8])
macro_rules! def_unpack_x {
    ($name:ident, $bits:expr) => {
        pub fn $name(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
            let in_bytes: Vec<u8> = input
                .iter()
                .flat_map(|w| w.to_le_bytes().into_iter())
                .collect();
            let mut out_words = vec![0u32; length as usize];
            let consumed = generic_unpack(length as usize, $bits, base, &in_bytes, &mut out_words);
            for (i, &w) in out_words.iter().enumerate() {
                let bytes = w.to_le_bytes();
                let off = i * 4;
                if off + 4 <= output.len() {
                    output[off..off + 4].copy_from_slice(&bytes);
                }
            }
            consumed
        }
    };
}

// Macro for linsearch_x
macro_rules! def_linsearch_x {
    ($name:ident, $bits:expr) => {
        pub fn $name(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
            generic_linsearch(length as usize, $bits, base, input, value, found)
        }
    };
}

// ============= pack_N_32 (N = 1..32) =============
def_pack!(pack1_32, 32, 1);
def_pack!(pack2_32, 32, 2);
def_pack!(pack3_32, 32, 3);
def_pack!(pack4_32, 32, 4);
def_pack!(pack5_32, 32, 5);
def_pack!(pack6_32, 32, 6);
def_pack!(pack7_32, 32, 7);
def_pack!(pack8_32, 32, 8);
def_pack!(pack9_32, 32, 9);
def_pack!(pack10_32, 32, 10);
def_pack!(pack11_32, 32, 11);
def_pack!(pack12_32, 32, 12);
def_pack!(pack13_32, 32, 13);
def_pack!(pack14_32, 32, 14);
def_pack!(pack15_32, 32, 15);
def_pack!(pack16_32, 32, 16);
def_pack!(pack17_32, 32, 17);
def_pack!(pack18_32, 32, 18);
def_pack!(pack19_32, 32, 19);
def_pack!(pack20_32, 32, 20);
def_pack!(pack21_32, 32, 21);
def_pack!(pack22_32, 32, 22);
def_pack!(pack23_32, 32, 23);
def_pack!(pack24_32, 32, 24);
def_pack!(pack25_32, 32, 25);
def_pack!(pack26_32, 32, 26);
def_pack!(pack27_32, 32, 27);
def_pack!(pack28_32, 32, 28);
def_pack!(pack29_32, 32, 29);
def_pack!(pack30_32, 32, 30);
def_pack!(pack31_32, 32, 31);
def_pack!(pack32_32, 32, 32);

// ============= pack_N_16 =============
def_pack!(pack1_16, 16, 1);
def_pack!(pack2_16, 16, 2);
def_pack!(pack3_16, 16, 3);
def_pack!(pack4_16, 16, 4);
def_pack!(pack5_16, 16, 5);
def_pack!(pack6_16, 16, 6);
def_pack!(pack7_16, 16, 7);
def_pack!(pack8_16, 16, 8);
def_pack!(pack9_16, 16, 9);
def_pack!(pack10_16, 16, 10);
def_pack!(pack11_16, 16, 11);
def_pack!(pack12_16, 16, 12);
def_pack!(pack13_16, 16, 13);
def_pack!(pack14_16, 16, 14);
def_pack!(pack15_16, 16, 15);
def_pack!(pack16_16, 16, 16);
def_pack!(pack17_16, 16, 17);
def_pack!(pack18_16, 16, 18);
def_pack!(pack19_16, 16, 19);
def_pack!(pack20_16, 16, 20);
def_pack!(pack21_16, 16, 21);
def_pack!(pack22_16, 16, 22);
def_pack!(pack23_16, 16, 23);
def_pack!(pack24_16, 16, 24);
def_pack!(pack25_16, 16, 25);
def_pack!(pack26_16, 16, 26);
def_pack!(pack27_16, 16, 27);
def_pack!(pack28_16, 16, 28);
def_pack!(pack29_16, 16, 29);
def_pack!(pack30_16, 16, 30);
def_pack!(pack31_16, 16, 31);
def_pack!(pack32_16, 16, 32);

// ============= pack_N_8 =============
def_pack!(pack1_8, 8, 1);
def_pack!(pack2_8, 8, 2);
def_pack!(pack3_8, 8, 3);
def_pack!(pack4_8, 8, 4);
def_pack!(pack5_8, 8, 5);
def_pack!(pack6_8, 8, 6);
def_pack!(pack7_8, 8, 7);
def_pack!(pack8_8, 8, 8);
def_pack!(pack9_8, 8, 9);
def_pack!(pack10_8, 8, 10);
def_pack!(pack11_8, 8, 11);
def_pack!(pack12_8, 8, 12);
def_pack!(pack13_8, 8, 13);
def_pack!(pack14_8, 8, 14);
def_pack!(pack15_8, 8, 15);
def_pack!(pack16_8, 8, 16);
def_pack!(pack17_8, 8, 17);
def_pack!(pack18_8, 8, 18);
def_pack!(pack19_8, 8, 19);
def_pack!(pack20_8, 8, 20);
def_pack!(pack21_8, 8, 21);
def_pack!(pack22_8, 8, 22);
def_pack!(pack23_8, 8, 23);
def_pack!(pack24_8, 8, 24);
def_pack!(pack25_8, 8, 25);
def_pack!(pack26_8, 8, 26);
def_pack!(pack27_8, 8, 27);
def_pack!(pack28_8, 8, 28);
def_pack!(pack29_8, 8, 29);
def_pack!(pack30_8, 8, 30);
def_pack!(pack31_8, 8, 31);
def_pack!(pack32_8, 8, 32);

// ============= unpack_N_32 (signature: input &[u8], output &mut [u32]) =============
// Note: the source-of-truth signature in src/for_gen.rs for unpackN_32, where N >= 1,
// uses the *(u32, &[u32], &mut [u8])* shape, but the lib-level dispatcher in
// forLib.rs needs the standard *(u32, &[u8], &mut [u32])* shape. We expose both
// public functions matching the source file signatures, plus a private helper
// module that exposes the "natural" signatures used by forLib.rs.

def_unpack!(unpack1_32, 32, 1);
def_unpack!(unpack2_32, 32, 2);
def_unpack!(unpack3_32, 32, 3);
def_unpack!(unpack4_32, 32, 4);
def_unpack!(unpack5_32, 32, 5);
def_unpack!(unpack6_32, 32, 6);
def_unpack!(unpack7_32, 32, 7);
def_unpack!(unpack8_32, 32, 8);
def_unpack!(unpack9_32, 32, 9);
def_unpack!(unpack10_32, 32, 10);
def_unpack!(unpack11_32, 32, 11);
def_unpack!(unpack12_32, 32, 12);
def_unpack!(unpack13_32, 32, 13);
def_unpack!(unpack14_32, 32, 14);
def_unpack!(unpack15_32, 32, 15);
def_unpack!(unpack16_32, 32, 16);
def_unpack!(unpack17_32, 32, 17);
def_unpack!(unpack18_32, 32, 18);
def_unpack!(unpack19_32, 32, 19);
def_unpack!(unpack20_32, 32, 20);
def_unpack!(unpack21_32, 32, 21);
def_unpack!(unpack22_32, 32, 22);
def_unpack!(unpack23_32, 32, 23);
def_unpack!(unpack24_32, 32, 24);
def_unpack!(unpack25_32, 32, 25);
def_unpack!(unpack26_32, 32, 26);
def_unpack!(unpack27_32, 32, 27);
def_unpack!(unpack28_32, 32, 28);
def_unpack!(unpack29_32, 32, 29);
def_unpack!(unpack30_32, 32, 30);
def_unpack!(unpack31_32, 32, 31);
def_unpack!(unpack32_32, 32, 32);

// ============= unpack_N_16 =============
def_unpack!(unpack1_16, 16, 1);
def_unpack!(unpack2_16, 16, 2);
def_unpack!(unpack3_16, 16, 3);
def_unpack!(unpack4_16, 16, 4);
def_unpack!(unpack5_16, 16, 5);
def_unpack!(unpack6_16, 16, 6);
def_unpack!(unpack7_16, 16, 7);
def_unpack!(unpack8_16, 16, 8);
def_unpack!(unpack9_16, 16, 9);
def_unpack!(unpack10_16, 16, 10);
def_unpack!(unpack11_16, 16, 11);
def_unpack!(unpack12_16, 16, 12);
def_unpack!(unpack13_16, 16, 13);
def_unpack!(unpack14_16, 16, 14);
def_unpack!(unpack15_16, 16, 15);
def_unpack!(unpack16_16, 16, 16);
def_unpack!(unpack17_16, 16, 17);
def_unpack!(unpack18_16, 16, 18);
def_unpack!(unpack19_16, 16, 19);
def_unpack!(unpack20_16, 16, 20);
def_unpack!(unpack21_16, 16, 21);
def_unpack!(unpack22_16, 16, 22);
def_unpack!(unpack23_16, 16, 23);
def_unpack!(unpack24_16, 16, 24);
def_unpack!(unpack25_16, 16, 25);
def_unpack!(unpack26_16, 16, 26);
def_unpack!(unpack27_16, 16, 27);
def_unpack!(unpack28_16, 16, 28);
def_unpack!(unpack29_16, 16, 29);
def_unpack!(unpack30_16, 16, 30);
def_unpack!(unpack31_16, 16, 31);
def_unpack!(unpack32_16, 16, 32);

// ============= unpack_N_8 =============
def_unpack!(unpack1_8, 8, 1);
def_unpack!(unpack2_8, 8, 2);
def_unpack!(unpack3_8, 8, 3);
def_unpack!(unpack4_8, 8, 4);
def_unpack!(unpack5_8, 8, 5);
def_unpack!(unpack6_8, 8, 6);
def_unpack!(unpack7_8, 8, 7);
def_unpack!(unpack8_8, 8, 8);
def_unpack!(unpack9_8, 8, 9);
def_unpack!(unpack10_8, 8, 10);
def_unpack!(unpack11_8, 8, 11);
def_unpack!(unpack12_8, 8, 12);
def_unpack!(unpack13_8, 8, 13);
def_unpack!(unpack14_8, 8, 14);
def_unpack!(unpack15_8, 8, 15);
def_unpack!(unpack16_8, 8, 16);
def_unpack!(unpack17_8, 8, 17);
def_unpack!(unpack18_8, 8, 18);
def_unpack!(unpack19_8, 8, 19);
def_unpack!(unpack20_8, 8, 20);
def_unpack!(unpack21_8, 8, 21);
def_unpack!(unpack22_8, 8, 22);
def_unpack!(unpack23_8, 8, 23);
def_unpack!(unpack24_8, 8, 24);
def_unpack!(unpack25_8, 8, 25);
def_unpack!(unpack26_8, 8, 26);
def_unpack!(unpack27_8, 8, 27);
def_unpack!(unpack28_8, 8, 28);
def_unpack!(unpack29_8, 8, 29);
def_unpack!(unpack30_8, 8, 30);
def_unpack!(unpack31_8, 8, 31);
def_unpack!(unpack32_8, 8, 32);

// ============= pack_x =============
def_pack_x!(pack1_x, 1);
def_pack_x!(pack2_x, 2);
def_pack_x!(pack3_x, 3);
def_pack_x!(pack4_x, 4);
def_pack_x!(pack5_x, 5);
def_pack_x!(pack6_x, 6);
def_pack_x!(pack7_x, 7);
def_pack_x!(pack8_x, 8);
def_pack_x!(pack9_x, 9);
def_pack_x!(pack10_x, 10);
def_pack_x!(pack11_x, 11);
def_pack_x!(pack12_x, 12);
def_pack_x!(pack13_x, 13);
def_pack_x!(pack14_x, 14);
def_pack_x!(pack15_x, 15);
def_pack_x!(pack16_x, 16);
def_pack_x!(pack17_x, 17);
def_pack_x!(pack18_x, 18);
def_pack_x!(pack19_x, 19);
def_pack_x!(pack20_x, 20);
def_pack_x!(pack21_x, 21);
def_pack_x!(pack22_x, 22);
def_pack_x!(pack23_x, 23);
def_pack_x!(pack24_x, 24);
def_pack_x!(pack25_x, 25);
def_pack_x!(pack26_x, 26);
def_pack_x!(pack27_x, 27);
def_pack_x!(pack28_x, 28);
def_pack_x!(pack29_x, 29);
def_pack_x!(pack30_x, 30);
def_pack_x!(pack31_x, 31);
def_pack_x!(pack32_x, 32);

// ============= unpack_x =============
def_unpack_x!(unpack1_x, 1);
def_unpack_x!(unpack2_x, 2);
def_unpack_x!(unpack3_x, 3);
def_unpack_x!(unpack4_x, 4);
def_unpack_x!(unpack5_x, 5);
def_unpack_x!(unpack6_x, 6);
def_unpack_x!(unpack7_x, 7);
def_unpack_x!(unpack8_x, 8);
def_unpack_x!(unpack9_x, 9);
def_unpack_x!(unpack10_x, 10);
def_unpack_x!(unpack11_x, 11);
def_unpack_x!(unpack12_x, 12);
def_unpack_x!(unpack13_x, 13);
def_unpack_x!(unpack14_x, 14);
def_unpack_x!(unpack15_x, 15);
def_unpack_x!(unpack16_x, 16);
def_unpack_x!(unpack17_x, 17);
def_unpack_x!(unpack18_x, 18);
def_unpack_x!(unpack19_x, 19);
def_unpack_x!(unpack20_x, 20);
def_unpack_x!(unpack21_x, 21);
def_unpack_x!(unpack22_x, 22);
def_unpack_x!(unpack23_x, 23);
def_unpack_x!(unpack24_x, 24);
def_unpack_x!(unpack25_x, 25);
def_unpack_x!(unpack26_x, 26);
def_unpack_x!(unpack27_x, 27);
def_unpack_x!(unpack28_x, 28);
def_unpack_x!(unpack29_x, 29);
def_unpack_x!(unpack30_x, 30);
def_unpack_x!(unpack31_x, 31);
def_unpack_x!(unpack32_x, 32);

// ============= linsearch_N_32 =============
def_linsearch!(linsearch1_32, 32, 1);
def_linsearch!(linsearch2_32, 32, 2);
def_linsearch!(linsearch3_32, 32, 3);
def_linsearch!(linsearch4_32, 32, 4);
def_linsearch!(linsearch5_32, 32, 5);
def_linsearch!(linsearch6_32, 32, 6);
def_linsearch!(linsearch7_32, 32, 7);
def_linsearch!(linsearch8_32, 32, 8);
def_linsearch!(linsearch9_32, 32, 9);
def_linsearch!(linsearch10_32, 32, 10);
def_linsearch!(linsearch11_32, 32, 11);
def_linsearch!(linsearch12_32, 32, 12);
def_linsearch!(linsearch13_32, 32, 13);
def_linsearch!(linsearch14_32, 32, 14);
def_linsearch!(linsearch15_32, 32, 15);
def_linsearch!(linsearch16_32, 32, 16);
def_linsearch!(linsearch17_32, 32, 17);
def_linsearch!(linsearch18_32, 32, 18);
def_linsearch!(linsearch19_32, 32, 19);
def_linsearch!(linsearch20_32, 32, 20);
def_linsearch!(linsearch21_32, 32, 21);
def_linsearch!(linsearch22_32, 32, 22);
def_linsearch!(linsearch23_32, 32, 23);
def_linsearch!(linsearch24_32, 32, 24);
def_linsearch!(linsearch25_32, 32, 25);
def_linsearch!(linsearch26_32, 32, 26);
def_linsearch!(linsearch27_32, 32, 27);
def_linsearch!(linsearch28_32, 32, 28);
def_linsearch!(linsearch29_32, 32, 29);
def_linsearch!(linsearch30_32, 32, 30);
def_linsearch!(linsearch31_32, 32, 31);
def_linsearch!(linsearch32_32, 32, 32);

// ============= linsearch_N_16 =============
def_linsearch!(linsearch1_16, 16, 1);
def_linsearch!(linsearch2_16, 16, 2);
def_linsearch!(linsearch3_16, 16, 3);
def_linsearch!(linsearch4_16, 16, 4);
def_linsearch!(linsearch5_16, 16, 5);
def_linsearch!(linsearch6_16, 16, 6);
def_linsearch!(linsearch7_16, 16, 7);
def_linsearch!(linsearch8_16, 16, 8);
def_linsearch!(linsearch9_16, 16, 9);
def_linsearch!(linsearch10_16, 16, 10);
def_linsearch!(linsearch11_16, 16, 11);
def_linsearch!(linsearch12_16, 16, 12);
def_linsearch!(linsearch13_16, 16, 13);
def_linsearch!(linsearch14_16, 16, 14);
def_linsearch!(linsearch15_16, 16, 15);
def_linsearch!(linsearch16_16, 16, 16);
def_linsearch!(linsearch17_16, 16, 17);
def_linsearch!(linsearch18_16, 16, 18);
def_linsearch!(linsearch19_16, 16, 19);
def_linsearch!(linsearch20_16, 16, 20);
def_linsearch!(linsearch21_16, 16, 21);
def_linsearch!(linsearch22_16, 16, 22);
def_linsearch!(linsearch23_16, 16, 23);
def_linsearch!(linsearch24_16, 16, 24);
def_linsearch!(linsearch25_16, 16, 25);
def_linsearch!(linsearch26_16, 16, 26);
def_linsearch!(linsearch27_16, 16, 27);
def_linsearch!(linsearch28_16, 16, 28);
def_linsearch!(linsearch29_16, 16, 29);
def_linsearch!(linsearch30_16, 16, 30);
def_linsearch!(linsearch31_16, 16, 31);
def_linsearch!(linsearch32_16, 16, 32);

// ============= linsearch_N_8 =============
def_linsearch!(linsearch1_8, 8, 1);
def_linsearch!(linsearch2_8, 8, 2);
def_linsearch!(linsearch3_8, 8, 3);
def_linsearch!(linsearch4_8, 8, 4);
def_linsearch!(linsearch5_8, 8, 5);
def_linsearch!(linsearch6_8, 8, 6);
def_linsearch!(linsearch7_8, 8, 7);
def_linsearch!(linsearch8_8, 8, 8);
def_linsearch!(linsearch9_8, 8, 9);
def_linsearch!(linsearch10_8, 8, 10);
def_linsearch!(linsearch11_8, 8, 11);
def_linsearch!(linsearch12_8, 8, 12);
def_linsearch!(linsearch13_8, 8, 13);
def_linsearch!(linsearch14_8, 8, 14);
def_linsearch!(linsearch15_8, 8, 15);
def_linsearch!(linsearch16_8, 8, 16);
def_linsearch!(linsearch17_8, 8, 17);
def_linsearch!(linsearch18_8, 8, 18);
def_linsearch!(linsearch19_8, 8, 19);
def_linsearch!(linsearch20_8, 8, 20);
def_linsearch!(linsearch21_8, 8, 21);
def_linsearch!(linsearch22_8, 8, 22);
def_linsearch!(linsearch23_8, 8, 23);
def_linsearch!(linsearch24_8, 8, 24);
def_linsearch!(linsearch25_8, 8, 25);
def_linsearch!(linsearch26_8, 8, 26);
def_linsearch!(linsearch27_8, 8, 27);
def_linsearch!(linsearch28_8, 8, 28);
def_linsearch!(linsearch29_8, 8, 29);
def_linsearch!(linsearch30_8, 8, 30);
def_linsearch!(linsearch31_8, 8, 31);
def_linsearch!(linsearch32_8, 8, 32);

// ============= linsearch_x =============
def_linsearch_x!(linsearch1_x, 1);
def_linsearch_x!(linsearch2_x, 2);
def_linsearch_x!(linsearch3_x, 3);
def_linsearch_x!(linsearch4_x, 4);
def_linsearch_x!(linsearch5_x, 5);
def_linsearch_x!(linsearch6_x, 6);
def_linsearch_x!(linsearch7_x, 7);
def_linsearch_x!(linsearch8_x, 8);
def_linsearch_x!(linsearch9_x, 9);
def_linsearch_x!(linsearch10_x, 10);
def_linsearch_x!(linsearch11_x, 11);
def_linsearch_x!(linsearch12_x, 12);
def_linsearch_x!(linsearch13_x, 13);
def_linsearch_x!(linsearch14_x, 14);
def_linsearch_x!(linsearch15_x, 15);
def_linsearch_x!(linsearch16_x, 16);
def_linsearch_x!(linsearch17_x, 17);
def_linsearch_x!(linsearch18_x, 18);
def_linsearch_x!(linsearch19_x, 19);
def_linsearch_x!(linsearch20_x, 20);
def_linsearch_x!(linsearch21_x, 21);
def_linsearch_x!(linsearch22_x, 22);
def_linsearch_x!(linsearch23_x, 23);
def_linsearch_x!(linsearch24_x, 24);
def_linsearch_x!(linsearch25_x, 25);
def_linsearch_x!(linsearch26_x, 26);
def_linsearch_x!(linsearch27_x, 27);
def_linsearch_x!(linsearch28_x, 28);
def_linsearch_x!(linsearch29_x, 29);
def_linsearch_x!(linsearch30_x, 30);
def_linsearch_x!(linsearch31_x, 31);
def_linsearch_x!(linsearch32_x, 32);

// ----- Internal dispatch helpers used by forLib.rs ----------------------
//
// The public `unpackN_M` / `unpackN_x` functions in this file have the
// awkward signature `(base, &[u32], &mut [u8])` (matching the source
// `for_gen.rs` literally). The high-level routines in `forLib.rs` need the
// "natural" signature `(base, &[u8], &mut [u32])`, so this private module
// provides that.

pub mod unpack_dispatch {
    use super::generic_unpack;

    pub fn un16(bits: u32, base: u32, input: &[u8], output: &mut [u32]) -> u32 {
        generic_unpack(16, bits, base, input, output)
    }
    pub fn un8(bits: u32, base: u32, input: &[u8], output: &mut [u32]) -> u32 {
        generic_unpack(8, bits, base, input, output)
    }
    pub fn unx(bits: u32, base: u32, input: &[u8], output: &mut [u32], length: u32) -> u32 {
        generic_unpack(length as usize, bits, base, input, output)
    }
    pub fn un32(bits: u32, base: u32, input: &[u8], output: &mut [u32]) -> u32 {
        generic_unpack(32, bits, base, input, output)
    }
}

pub mod linsearch_dispatch {
    use super::generic_linsearch;
    pub fn ls_n(bits: u32, n: u32, base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
        generic_linsearch(n as usize, bits, base, input, value, found)
    }
    pub fn ls_x(bits: u32, base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
        generic_linsearch(length as usize, bits, base, input, value, found)
    }
}
