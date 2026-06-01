// Helper: read u32 from byte slice (little-endian)
#[inline]
fn read_u32_le(input: &[u8]) -> u32 {
    u32::from_le_bytes([input[0], input[1], input[2], input[3]])
}

// Helper: write u32 to byte slice (little-endian)
#[inline]
fn write_u32_le(output: &mut [u8], value: u32) {
    output[0..4].copy_from_slice(&value.to_le_bytes());
}

pub fn pack_block_pub(base: u32, input: &[u32], output: &mut [u8], bits: u32, block: usize) -> u32 {
    pack_block(base, input, output, bits, block)
}
pub fn unpack_block_pub(base: u32, input: &[u8], output: &mut [u32], bits: u32, block: usize) -> u32 {
    unpack_block(base, input, output, bits, block)
}
pub fn linsearch_block_pub(
    base: u32,
    input: &[u8],
    value: u32,
    found: &mut i32,
    bits: u32,
    block: usize,
) -> u32 {
    linsearch_block(base, input, value, found, bits, block)
}
pub fn pack_x_pub(base: u32, input: &[u32], output: &mut [u8], bits: u32, length: u32) -> u32 {
    pack_x(base, input, output, bits, length)
}
pub fn unpack_x_pub(base: u32, input: &[u8], output: &mut [u32], bits: u32, length: u32) -> u32 {
    unpack_x(base, input, output, bits, length)
}
pub fn linsearch_x_pub(
    base: u32,
    input: &[u8],
    length: u32,
    value: u32,
    found: &mut i32,
    bits: u32,
) -> u32 {
    linsearch_x(base, input, length, value, found, bits)
}

// Generic pack of |block| u32 values using |bits| bits each.
// Output format matches the C generated pack functions.
fn pack_block(base: u32, input: &[u32], output: &mut [u8], bits: u32, block: usize) -> u32 {
    if bits == 0 {
        return 0;
    }
    if bits == 32 {
        for i in 0..block {
            let v = input[i].wrapping_sub(base);
            write_u32_le(&mut output[i * 4..], v);
        }
        return (block * 4) as u32;
    }

    let mut tmp: u32 = 0;
    let mut b: u32 = 0;
    let mut out_pos: usize = 0;
    let mut i: usize = 0;
    let mut inittmp = true;

    loop {
        while b < 32 && i < block {
            let v = input[i].wrapping_sub(base);
            if b + bits <= 32 {
                let shifted = v.wrapping_shl(b);
                if inittmp {
                    tmp = shifted;
                    inittmp = false;
                } else {
                    tmp |= shifted;
                }
                b += bits;
            } else {
                tmp |= v.wrapping_shl(b);
                write_u32_le(&mut output[out_pos..], tmp);
                out_pos += 4;
                let d = b + bits - 32;
                tmp = v >> (bits - d);
                b = d;
                inittmp = false;
            }
            i += 1;
        }
        if i < block {
            // alignment write
            write_u32_le(&mut output[out_pos..], tmp);
            out_pos += 4;
            inittmp = true;
            b = 0;
        } else {
            // final memcpy
            let length = 4usize - ((32 - b) / 8) as usize;
            for k in 0..length {
                output[out_pos + k] = (tmp >> (k * 8)) as u8;
            }
            out_pos += length;
            break;
        }
    }
    out_pos as u32
}

// Generic unpack of |block| u32 values using |bits| bits each.
fn unpack_block(base: u32, input: &[u8], output: &mut [u32], bits: u32, block: usize) -> u32 {
    if bits == 0 {
        for k in 0..block {
            output[k] = base;
        }
        return 0;
    }
    if bits == 32 {
        for k in 0..block {
            let v = read_u32_le(&input[k * 4..]);
            output[k] = base.wrapping_add(v);
        }
        return (block * 4) as u32;
    }

    let mask: u32 = if bits == 32 { u32::MAX } else { (1u32 << bits) - 1 };
    let mut in_pos: usize = 0;
    let mut tmp: u32 = read_u32_le(&input[in_pos..]);
    let mut consumed: u32 = 4;
    let mut b: u32 = 0;
    let mut i: usize = 0;

    loop {
        while b < 32 && i < block {
            if b + bits <= 32 {
                output[i] = base.wrapping_add((tmp >> b) & mask);
                b += bits;
            } else {
                let high = tmp >> b;
                in_pos += 4;
                tmp = read_u32_le(&input[in_pos..]);
                consumed += 4;
                let d = b + bits - 32;
                let low = tmp & ((1u32 << d) - 1);
                output[i] = base.wrapping_add(high | (low.wrapping_shl(bits - d)));
                b = d;
            }
            i += 1;
        }
        if i < block {
            in_pos += 4;
            tmp = read_u32_le(&input[in_pos..]);
            consumed += 4;
            b = 0;
        } else {
            let remaining_bits = 32 - b;
            consumed -= remaining_bits / 8;
            break;
        }
    }
    consumed
}

// Generic linsearch of |block| u32 values using |bits| bits each.
fn linsearch_block(
    base: u32,
    input: &[u8],
    value_in: u32,
    found: &mut i32,
    bits: u32,
    block: usize,
) -> u32 {
    if bits == 0 {
        if base == value_in {
            *found = 0;
        }
        return 0;
    }
    let value = value_in.wrapping_sub(base);
    if bits == 32 {
        for k in 0..block {
            let v = read_u32_le(&input[k * 4..]);
            if v == value {
                *found = k as i32;
                return k as u32;
            }
        }
        return (block * 4) as u32;
    }

    let mask: u32 = if bits == 32 { u32::MAX } else { (1u32 << bits) - 1 };
    let mut in_pos: usize = 0;
    let mut tmp: u32 = read_u32_le(&input[in_pos..]);
    let mut consumed: u32 = 4;
    let mut b: u32 = 0;
    let mut i: usize = 0;

    loop {
        while b < 32 && i < block {
            let extracted: u32;
            if b + bits <= 32 {
                extracted = (tmp >> b) & mask;
                b += bits;
            } else {
                let tmp2 = tmp >> b;
                in_pos += 4;
                tmp = read_u32_le(&input[in_pos..]);
                consumed += 4;
                let d = b + bits - 32;
                extracted = tmp2 | ((tmp & ((1u32 << d) - 1)).wrapping_shl(bits - d));
                b = d;
            }
            if extracted == value {
                *found = i as i32;
                return i as u32;
            }
            i += 1;
        }
        if i < block {
            in_pos += 4;
            tmp = read_u32_le(&input[in_pos..]);
            consumed += 4;
            b = 0;
        } else {
            let remaining_bits = 32 - b;
            consumed -= remaining_bits / 8;
            break;
        }
    }
    consumed
}

// Generic pack_x for variable-length blocks (length <= 8).
fn pack_x(base: u32, input: &[u32], output: &mut [u8], bits: u32, length_u: u32) -> u32 {
    if length_u == 0 {
        return 0;
    }
    if bits == 0 {
        return 0;
    }
    let length = length_u as usize;
    let block: usize = 8;

    let mut tmp: u32 = 0;
    let mut b: u32 = 0;
    let mut out_pos: usize = 0;
    let mut i: usize = 0;
    let mut inittmp = true;
    let mut bailed = false;

    'outer: loop {
        while b < 32 && i < block {
            let v = input[i].wrapping_sub(base);
            if b + bits <= 32 {
                let shifted = v.wrapping_shl(b);
                if inittmp {
                    tmp = shifted;
                    inittmp = false;
                } else {
                    tmp |= shifted;
                }
                b += bits;
            } else {
                tmp |= v.wrapping_shl(b);
                write_u32_le(&mut output[out_pos..], tmp);
                out_pos += 4;
                let d = b + bits - 32;
                tmp = v >> (bits - d);
                b = d;
                inittmp = false;
            }
            i += 1;
            if i == length {
                bailed = true;
                break 'outer;
            }
        }
        if i < block {
            // alignment write
            write_u32_le(&mut output[out_pos..], tmp);
            out_pos += 4;
            inittmp = true;
            b = 0;
        } else {
            break 'outer;
        }
    }
    let _ = bailed; // suppress unused warning

    let total_bytes = ((length_u * bits + 7) / 8) as usize;
    let mut remaining = total_bytes % 4;
    if remaining == 0 {
        remaining = 4;
    }
    for k in 0..remaining {
        output[out_pos + k] = (tmp >> (k * 8)) as u8;
    }
    total_bytes as u32
}

// Generic unpack_x for variable-length blocks (length <= 8).
fn unpack_x(base: u32, input: &[u8], output: &mut [u32], bits: u32, length_u: u32) -> u32 {
    if length_u == 0 {
        return 0;
    }
    if bits == 0 {
        for k in 0..length_u as usize {
            output[k] = base;
        }
        return 0;
    }
    let length = length_u as usize;
    let block: usize = 8;
    let mask: u32 = if bits == 32 { u32::MAX } else { (1u32 << bits) - 1 };

    let mut in_pos: usize = 0;
    let mut tmp: u32 = read_u32_le(&input[in_pos..]);
    let mut b: u32 = 0;
    let mut i: usize = 0;

    'outer: loop {
        while b < 32 && i < block {
            if b + bits <= 32 {
                output[i] = base.wrapping_add((tmp >> b) & mask);
                b += bits;
            } else {
                let high = tmp >> b;
                in_pos += 4;
                tmp = read_u32_le(&input[in_pos..]);
                let d = b + bits - 32;
                let low = tmp & ((1u32 << d) - 1);
                output[i] = base.wrapping_add(high | (low.wrapping_shl(bits - d)));
                b = d;
            }
            i += 1;
            if i == length {
                break 'outer;
            }
        }
        if i < block {
            in_pos += 4;
            tmp = read_u32_le(&input[in_pos..]);
            b = 0;
        } else {
            break 'outer;
        }
    }

    (length_u * bits + 7) / 8
}

// Generic linsearch_x for variable-length blocks (length <= 8).
fn linsearch_x(
    base: u32,
    input: &[u8],
    length_u: u32,
    value_in: u32,
    found: &mut i32,
    bits: u32,
) -> u32 {
    if length_u == 0 {
        return 0;
    }
    if bits == 0 {
        if base == value_in && length_u > 0 {
            *found = 0;
        }
        return 0;
    }
    let length = length_u as usize;
    let block: usize = 8;
    let mask: u32 = if bits == 32 { u32::MAX } else { (1u32 << bits) - 1 };
    let value = value_in.wrapping_sub(base);

    let mut in_pos: usize = 0;
    let mut tmp: u32 = read_u32_le(&input[in_pos..]);
    let mut b: u32 = 0;
    let mut i: usize = 0;

    'outer: loop {
        while b < 32 && i < block {
            let extracted: u32;
            if b + bits <= 32 {
                extracted = (tmp >> b) & mask;
                b += bits;
            } else {
                let tmp2 = tmp >> b;
                in_pos += 4;
                tmp = read_u32_le(&input[in_pos..]);
                let d = b + bits - 32;
                extracted = tmp2 | ((tmp & ((1u32 << d) - 1)).wrapping_shl(bits - d));
                b = d;
            }
            if extracted == value {
                *found = i as i32;
                return i as u32;
            }
            i += 1;
            if i == length {
                break 'outer;
            }
        }
        if i < block {
            in_pos += 4;
            tmp = read_u32_le(&input[in_pos..]);
            b = 0;
        } else {
            break 'outer;
        }
    }

    (length_u * bits + 7) / 8
}

// === pack0_*, unpack0_*, linsearch0_* ===
pub fn pack0_32(_base: u32, _input: &[u32], _output: &mut [u8]) -> u32 { 0 }
pub fn pack0_16(_base: u32, _input: &[u32], _output: &mut [u8]) -> u32 { 0 }
pub fn pack0_8(_base: u32, _input: &[u32], _output: &mut [u8]) -> u32 { 0 }
pub fn unpack0_32(base: u32, _input: &[u8], output: &mut [u32]) -> u32 {
    for k in 0..32 { output[k] = base; }
    0
}
pub fn unpack0_16(base: u32, _input: &[u8], output: &mut [u32]) -> u32 {
    for k in 0..16 { output[k] = base; }
    0
}
pub fn unpack0_8(base: u32, _input: &[u8], output: &mut [u32]) -> u32 {
    for k in 0..8 { output[k] = base; }
    0
}
pub fn pack0_x(_base: u32, _input: &[u32], _output: &mut [u8], _length: u32) -> u32 { 0 }
pub fn unpack0_x(base: u32, _input: &[u8], output: &mut [u32], length: u32) -> u32 {
    for k in 0..length as usize { output[k] = base; }
    0
}
pub fn linsearch0_32(base: u32, _input: &[u8], value: u32, found: &mut i32) -> u32 {
    if base == value { *found = 0; }
    0
}
pub fn linsearch0_16(base: u32, _input: &[u8], value: u32, found: &mut i32) -> u32 {
    if base == value { *found = 0; }
    0
}
pub fn linsearch0_8(base: u32, _input: &[u8], value: u32, found: &mut i32) -> u32 {
    if base == value { *found = 0; }
    0
}
pub fn linsearch0_x(base: u32, _input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    if base == value && length > 0 { *found = 0; }
    0
}

// === Generated dispatch helpers ===
pub fn pack1_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 1, 32) }
pub fn pack2_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 2, 32) }
pub fn pack3_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 3, 32) }
pub fn pack4_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 4, 32) }
pub fn pack5_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 5, 32) }
pub fn pack6_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 6, 32) }
pub fn pack7_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 7, 32) }
pub fn pack8_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 8, 32) }
pub fn pack9_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 9, 32) }
pub fn pack10_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 10, 32) }
pub fn pack11_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 11, 32) }
pub fn pack12_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 12, 32) }
pub fn pack13_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 13, 32) }
pub fn pack14_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 14, 32) }
pub fn pack15_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 15, 32) }
pub fn pack16_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 16, 32) }
pub fn pack17_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 17, 32) }
pub fn pack18_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 18, 32) }
pub fn pack19_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 19, 32) }
pub fn pack20_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 20, 32) }
pub fn pack21_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 21, 32) }
pub fn pack22_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 22, 32) }
pub fn pack23_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 23, 32) }
pub fn pack24_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 24, 32) }
pub fn pack25_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 25, 32) }
pub fn pack26_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 26, 32) }
pub fn pack27_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 27, 32) }
pub fn pack28_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 28, 32) }
pub fn pack29_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 29, 32) }
pub fn pack30_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 30, 32) }
pub fn pack31_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 31, 32) }
pub fn pack32_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 32, 32) }
pub fn unpack1_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 1, 32) }
pub fn unpack2_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 2, 32) }
pub fn unpack3_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 3, 32) }
pub fn unpack4_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 4, 32) }
pub fn unpack5_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 5, 32) }
pub fn unpack6_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 6, 32) }
pub fn unpack7_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 7, 32) }
pub fn unpack8_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 8, 32) }
pub fn unpack9_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 9, 32) }
pub fn unpack10_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 10, 32) }
pub fn unpack11_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 11, 32) }
pub fn unpack12_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 12, 32) }
pub fn unpack13_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 13, 32) }
pub fn unpack14_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 14, 32) }
pub fn unpack15_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 15, 32) }
pub fn unpack16_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 16, 32) }
pub fn unpack17_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 17, 32) }
pub fn unpack18_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 18, 32) }
pub fn unpack19_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 19, 32) }
pub fn unpack20_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 20, 32) }
pub fn unpack21_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 21, 32) }
pub fn unpack22_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 22, 32) }
pub fn unpack23_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 23, 32) }
pub fn unpack24_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 24, 32) }
pub fn unpack25_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 25, 32) }
pub fn unpack26_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 26, 32) }
pub fn unpack27_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 27, 32) }
pub fn unpack28_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 28, 32) }
pub fn unpack29_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 29, 32) }
pub fn unpack30_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 30, 32) }
pub fn unpack31_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 31, 32) }
pub fn unpack32_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 32, 32) }
pub fn pack1_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 1, 16) }
pub fn pack2_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 2, 16) }
pub fn pack3_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 3, 16) }
pub fn pack4_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 4, 16) }
pub fn pack5_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 5, 16) }
pub fn pack6_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 6, 16) }
pub fn pack7_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 7, 16) }
pub fn pack8_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 8, 16) }
pub fn pack9_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 9, 16) }
pub fn pack10_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 10, 16) }
pub fn pack11_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 11, 16) }
pub fn pack12_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 12, 16) }
pub fn pack13_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 13, 16) }
pub fn pack14_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 14, 16) }
pub fn pack15_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 15, 16) }
pub fn pack16_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 16, 16) }
pub fn pack17_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 17, 16) }
pub fn pack18_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 18, 16) }
pub fn pack19_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 19, 16) }
pub fn pack20_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 20, 16) }
pub fn pack21_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 21, 16) }
pub fn pack22_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 22, 16) }
pub fn pack23_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 23, 16) }
pub fn pack24_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 24, 16) }
pub fn pack25_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 25, 16) }
pub fn pack26_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 26, 16) }
pub fn pack27_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 27, 16) }
pub fn pack28_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 28, 16) }
pub fn pack29_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 29, 16) }
pub fn pack30_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 30, 16) }
pub fn pack31_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 31, 16) }
pub fn pack32_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 32, 16) }
pub fn unpack1_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 1, 16) }
pub fn unpack2_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 2, 16) }
pub fn unpack3_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 3, 16) }
pub fn unpack4_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 4, 16) }
pub fn unpack5_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 5, 16) }
pub fn unpack6_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 6, 16) }
pub fn unpack7_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 7, 16) }
pub fn unpack8_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 8, 16) }
pub fn unpack9_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 9, 16) }
pub fn unpack10_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 10, 16) }
pub fn unpack11_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 11, 16) }
pub fn unpack12_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 12, 16) }
pub fn unpack13_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 13, 16) }
pub fn unpack14_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 14, 16) }
pub fn unpack15_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 15, 16) }
pub fn unpack16_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 16, 16) }
pub fn unpack17_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 17, 16) }
pub fn unpack18_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 18, 16) }
pub fn unpack19_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 19, 16) }
pub fn unpack20_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 20, 16) }
pub fn unpack21_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 21, 16) }
pub fn unpack22_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 22, 16) }
pub fn unpack23_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 23, 16) }
pub fn unpack24_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 24, 16) }
pub fn unpack25_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 25, 16) }
pub fn unpack26_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 26, 16) }
pub fn unpack27_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 27, 16) }
pub fn unpack28_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 28, 16) }
pub fn unpack29_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 29, 16) }
pub fn unpack30_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 30, 16) }
pub fn unpack31_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 31, 16) }
pub fn unpack32_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 32, 16) }
pub fn pack1_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 1, 8) }
pub fn pack2_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 2, 8) }
pub fn pack3_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 3, 8) }
pub fn pack4_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 4, 8) }
pub fn pack5_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 5, 8) }
pub fn pack6_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 6, 8) }
pub fn pack7_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 7, 8) }
pub fn pack8_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 8, 8) }
pub fn pack9_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 9, 8) }
pub fn pack10_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 10, 8) }
pub fn pack11_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 11, 8) }
pub fn pack12_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 12, 8) }
pub fn pack13_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 13, 8) }
pub fn pack14_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 14, 8) }
pub fn pack15_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 15, 8) }
pub fn pack16_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 16, 8) }
pub fn pack17_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 17, 8) }
pub fn pack18_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 18, 8) }
pub fn pack19_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 19, 8) }
pub fn pack20_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 20, 8) }
pub fn pack21_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 21, 8) }
pub fn pack22_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 22, 8) }
pub fn pack23_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 23, 8) }
pub fn pack24_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 24, 8) }
pub fn pack25_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 25, 8) }
pub fn pack26_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 26, 8) }
pub fn pack27_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 27, 8) }
pub fn pack28_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 28, 8) }
pub fn pack29_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 29, 8) }
pub fn pack30_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 30, 8) }
pub fn pack31_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 31, 8) }
pub fn pack32_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 { pack_block(base, input, output, 32, 8) }
pub fn unpack1_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 1, 8) }
pub fn unpack2_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 2, 8) }
pub fn unpack3_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 3, 8) }
pub fn unpack4_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 4, 8) }
pub fn unpack5_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 5, 8) }
pub fn unpack6_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 6, 8) }
pub fn unpack7_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 7, 8) }
pub fn unpack8_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 8, 8) }
pub fn unpack9_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 9, 8) }
pub fn unpack10_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 10, 8) }
pub fn unpack11_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 11, 8) }
pub fn unpack12_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 12, 8) }
pub fn unpack13_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 13, 8) }
pub fn unpack14_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 14, 8) }
pub fn unpack15_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 15, 8) }
pub fn unpack16_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 16, 8) }
pub fn unpack17_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 17, 8) }
pub fn unpack18_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 18, 8) }
pub fn unpack19_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 19, 8) }
pub fn unpack20_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 20, 8) }
pub fn unpack21_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 21, 8) }
pub fn unpack22_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 22, 8) }
pub fn unpack23_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 23, 8) }
pub fn unpack24_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 24, 8) }
pub fn unpack25_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 25, 8) }
pub fn unpack26_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 26, 8) }
pub fn unpack27_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 27, 8) }
pub fn unpack28_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 28, 8) }
pub fn unpack29_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 29, 8) }
pub fn unpack30_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 30, 8) }
pub fn unpack31_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 31, 8) }
pub fn unpack32_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 { unpack_block(base, input, output, 32, 8) }
pub fn pack1_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 { pack_x(base, input, output, 1, length) }
pub fn pack2_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 { pack_x(base, input, output, 2, length) }
pub fn pack3_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 { pack_x(base, input, output, 3, length) }
pub fn pack4_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 { pack_x(base, input, output, 4, length) }
pub fn pack5_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 { pack_x(base, input, output, 5, length) }
pub fn pack6_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 { pack_x(base, input, output, 6, length) }
pub fn pack7_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 { pack_x(base, input, output, 7, length) }
pub fn pack8_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 { pack_x(base, input, output, 8, length) }
pub fn pack9_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 { pack_x(base, input, output, 9, length) }
pub fn pack10_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 { pack_x(base, input, output, 10, length) }
pub fn pack11_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 { pack_x(base, input, output, 11, length) }
pub fn pack12_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 { pack_x(base, input, output, 12, length) }
pub fn pack13_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 { pack_x(base, input, output, 13, length) }
pub fn pack14_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 { pack_x(base, input, output, 14, length) }
pub fn pack15_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 { pack_x(base, input, output, 15, length) }
pub fn pack16_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 { pack_x(base, input, output, 16, length) }
pub fn pack17_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 { pack_x(base, input, output, 17, length) }
pub fn pack18_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 { pack_x(base, input, output, 18, length) }
pub fn pack19_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 { pack_x(base, input, output, 19, length) }
pub fn pack20_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 { pack_x(base, input, output, 20, length) }
pub fn pack21_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 { pack_x(base, input, output, 21, length) }
pub fn pack22_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 { pack_x(base, input, output, 22, length) }
pub fn pack23_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 { pack_x(base, input, output, 23, length) }
pub fn pack24_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 { pack_x(base, input, output, 24, length) }
pub fn pack25_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 { pack_x(base, input, output, 25, length) }
pub fn pack26_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 { pack_x(base, input, output, 26, length) }
pub fn pack27_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 { pack_x(base, input, output, 27, length) }
pub fn pack28_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 { pack_x(base, input, output, 28, length) }
pub fn pack29_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 { pack_x(base, input, output, 29, length) }
pub fn pack30_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 { pack_x(base, input, output, 30, length) }
pub fn pack31_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 { pack_x(base, input, output, 31, length) }
pub fn pack32_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 { pack_x(base, input, output, 32, length) }
pub fn unpack1_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    // The skeleton signature has input/output types swapped. Convert to bytes/u32 view.
    let in_bytes: Vec<u8> = input.iter().flat_map(|w| w.to_le_bytes()).collect();
    let mut out_u32 = vec![0u32; length as usize];
    let r = unpack_x(base, &in_bytes, &mut out_u32, 1, length);
    // Write back as bytes if possible
    for (i, v) in out_u32.iter().enumerate() {
        let bytes = v.to_le_bytes();
        let off = i * 4;
        if off + 4 <= output.len() {
            output[off..off+4].copy_from_slice(&bytes);
        }
    }
    r
}
pub fn unpack2_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    // The skeleton signature has input/output types swapped. Convert to bytes/u32 view.
    let in_bytes: Vec<u8> = input.iter().flat_map(|w| w.to_le_bytes()).collect();
    let mut out_u32 = vec![0u32; length as usize];
    let r = unpack_x(base, &in_bytes, &mut out_u32, 2, length);
    // Write back as bytes if possible
    for (i, v) in out_u32.iter().enumerate() {
        let bytes = v.to_le_bytes();
        let off = i * 4;
        if off + 4 <= output.len() {
            output[off..off+4].copy_from_slice(&bytes);
        }
    }
    r
}
pub fn unpack3_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    // The skeleton signature has input/output types swapped. Convert to bytes/u32 view.
    let in_bytes: Vec<u8> = input.iter().flat_map(|w| w.to_le_bytes()).collect();
    let mut out_u32 = vec![0u32; length as usize];
    let r = unpack_x(base, &in_bytes, &mut out_u32, 3, length);
    // Write back as bytes if possible
    for (i, v) in out_u32.iter().enumerate() {
        let bytes = v.to_le_bytes();
        let off = i * 4;
        if off + 4 <= output.len() {
            output[off..off+4].copy_from_slice(&bytes);
        }
    }
    r
}
pub fn unpack4_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    // The skeleton signature has input/output types swapped. Convert to bytes/u32 view.
    let in_bytes: Vec<u8> = input.iter().flat_map(|w| w.to_le_bytes()).collect();
    let mut out_u32 = vec![0u32; length as usize];
    let r = unpack_x(base, &in_bytes, &mut out_u32, 4, length);
    // Write back as bytes if possible
    for (i, v) in out_u32.iter().enumerate() {
        let bytes = v.to_le_bytes();
        let off = i * 4;
        if off + 4 <= output.len() {
            output[off..off+4].copy_from_slice(&bytes);
        }
    }
    r
}
pub fn unpack5_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    // The skeleton signature has input/output types swapped. Convert to bytes/u32 view.
    let in_bytes: Vec<u8> = input.iter().flat_map(|w| w.to_le_bytes()).collect();
    let mut out_u32 = vec![0u32; length as usize];
    let r = unpack_x(base, &in_bytes, &mut out_u32, 5, length);
    // Write back as bytes if possible
    for (i, v) in out_u32.iter().enumerate() {
        let bytes = v.to_le_bytes();
        let off = i * 4;
        if off + 4 <= output.len() {
            output[off..off+4].copy_from_slice(&bytes);
        }
    }
    r
}
pub fn unpack6_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    // The skeleton signature has input/output types swapped. Convert to bytes/u32 view.
    let in_bytes: Vec<u8> = input.iter().flat_map(|w| w.to_le_bytes()).collect();
    let mut out_u32 = vec![0u32; length as usize];
    let r = unpack_x(base, &in_bytes, &mut out_u32, 6, length);
    // Write back as bytes if possible
    for (i, v) in out_u32.iter().enumerate() {
        let bytes = v.to_le_bytes();
        let off = i * 4;
        if off + 4 <= output.len() {
            output[off..off+4].copy_from_slice(&bytes);
        }
    }
    r
}
pub fn unpack7_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    // The skeleton signature has input/output types swapped. Convert to bytes/u32 view.
    let in_bytes: Vec<u8> = input.iter().flat_map(|w| w.to_le_bytes()).collect();
    let mut out_u32 = vec![0u32; length as usize];
    let r = unpack_x(base, &in_bytes, &mut out_u32, 7, length);
    // Write back as bytes if possible
    for (i, v) in out_u32.iter().enumerate() {
        let bytes = v.to_le_bytes();
        let off = i * 4;
        if off + 4 <= output.len() {
            output[off..off+4].copy_from_slice(&bytes);
        }
    }
    r
}
pub fn unpack8_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    // The skeleton signature has input/output types swapped. Convert to bytes/u32 view.
    let in_bytes: Vec<u8> = input.iter().flat_map(|w| w.to_le_bytes()).collect();
    let mut out_u32 = vec![0u32; length as usize];
    let r = unpack_x(base, &in_bytes, &mut out_u32, 8, length);
    // Write back as bytes if possible
    for (i, v) in out_u32.iter().enumerate() {
        let bytes = v.to_le_bytes();
        let off = i * 4;
        if off + 4 <= output.len() {
            output[off..off+4].copy_from_slice(&bytes);
        }
    }
    r
}
pub fn unpack9_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    // The skeleton signature has input/output types swapped. Convert to bytes/u32 view.
    let in_bytes: Vec<u8> = input.iter().flat_map(|w| w.to_le_bytes()).collect();
    let mut out_u32 = vec![0u32; length as usize];
    let r = unpack_x(base, &in_bytes, &mut out_u32, 9, length);
    // Write back as bytes if possible
    for (i, v) in out_u32.iter().enumerate() {
        let bytes = v.to_le_bytes();
        let off = i * 4;
        if off + 4 <= output.len() {
            output[off..off+4].copy_from_slice(&bytes);
        }
    }
    r
}
pub fn unpack10_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    // The skeleton signature has input/output types swapped. Convert to bytes/u32 view.
    let in_bytes: Vec<u8> = input.iter().flat_map(|w| w.to_le_bytes()).collect();
    let mut out_u32 = vec![0u32; length as usize];
    let r = unpack_x(base, &in_bytes, &mut out_u32, 10, length);
    // Write back as bytes if possible
    for (i, v) in out_u32.iter().enumerate() {
        let bytes = v.to_le_bytes();
        let off = i * 4;
        if off + 4 <= output.len() {
            output[off..off+4].copy_from_slice(&bytes);
        }
    }
    r
}
pub fn unpack11_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    // The skeleton signature has input/output types swapped. Convert to bytes/u32 view.
    let in_bytes: Vec<u8> = input.iter().flat_map(|w| w.to_le_bytes()).collect();
    let mut out_u32 = vec![0u32; length as usize];
    let r = unpack_x(base, &in_bytes, &mut out_u32, 11, length);
    // Write back as bytes if possible
    for (i, v) in out_u32.iter().enumerate() {
        let bytes = v.to_le_bytes();
        let off = i * 4;
        if off + 4 <= output.len() {
            output[off..off+4].copy_from_slice(&bytes);
        }
    }
    r
}
pub fn unpack12_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    // The skeleton signature has input/output types swapped. Convert to bytes/u32 view.
    let in_bytes: Vec<u8> = input.iter().flat_map(|w| w.to_le_bytes()).collect();
    let mut out_u32 = vec![0u32; length as usize];
    let r = unpack_x(base, &in_bytes, &mut out_u32, 12, length);
    // Write back as bytes if possible
    for (i, v) in out_u32.iter().enumerate() {
        let bytes = v.to_le_bytes();
        let off = i * 4;
        if off + 4 <= output.len() {
            output[off..off+4].copy_from_slice(&bytes);
        }
    }
    r
}
pub fn unpack13_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    // The skeleton signature has input/output types swapped. Convert to bytes/u32 view.
    let in_bytes: Vec<u8> = input.iter().flat_map(|w| w.to_le_bytes()).collect();
    let mut out_u32 = vec![0u32; length as usize];
    let r = unpack_x(base, &in_bytes, &mut out_u32, 13, length);
    // Write back as bytes if possible
    for (i, v) in out_u32.iter().enumerate() {
        let bytes = v.to_le_bytes();
        let off = i * 4;
        if off + 4 <= output.len() {
            output[off..off+4].copy_from_slice(&bytes);
        }
    }
    r
}
pub fn unpack14_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    // The skeleton signature has input/output types swapped. Convert to bytes/u32 view.
    let in_bytes: Vec<u8> = input.iter().flat_map(|w| w.to_le_bytes()).collect();
    let mut out_u32 = vec![0u32; length as usize];
    let r = unpack_x(base, &in_bytes, &mut out_u32, 14, length);
    // Write back as bytes if possible
    for (i, v) in out_u32.iter().enumerate() {
        let bytes = v.to_le_bytes();
        let off = i * 4;
        if off + 4 <= output.len() {
            output[off..off+4].copy_from_slice(&bytes);
        }
    }
    r
}
pub fn unpack15_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    // The skeleton signature has input/output types swapped. Convert to bytes/u32 view.
    let in_bytes: Vec<u8> = input.iter().flat_map(|w| w.to_le_bytes()).collect();
    let mut out_u32 = vec![0u32; length as usize];
    let r = unpack_x(base, &in_bytes, &mut out_u32, 15, length);
    // Write back as bytes if possible
    for (i, v) in out_u32.iter().enumerate() {
        let bytes = v.to_le_bytes();
        let off = i * 4;
        if off + 4 <= output.len() {
            output[off..off+4].copy_from_slice(&bytes);
        }
    }
    r
}
pub fn unpack16_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    // The skeleton signature has input/output types swapped. Convert to bytes/u32 view.
    let in_bytes: Vec<u8> = input.iter().flat_map(|w| w.to_le_bytes()).collect();
    let mut out_u32 = vec![0u32; length as usize];
    let r = unpack_x(base, &in_bytes, &mut out_u32, 16, length);
    // Write back as bytes if possible
    for (i, v) in out_u32.iter().enumerate() {
        let bytes = v.to_le_bytes();
        let off = i * 4;
        if off + 4 <= output.len() {
            output[off..off+4].copy_from_slice(&bytes);
        }
    }
    r
}
pub fn unpack17_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    // The skeleton signature has input/output types swapped. Convert to bytes/u32 view.
    let in_bytes: Vec<u8> = input.iter().flat_map(|w| w.to_le_bytes()).collect();
    let mut out_u32 = vec![0u32; length as usize];
    let r = unpack_x(base, &in_bytes, &mut out_u32, 17, length);
    // Write back as bytes if possible
    for (i, v) in out_u32.iter().enumerate() {
        let bytes = v.to_le_bytes();
        let off = i * 4;
        if off + 4 <= output.len() {
            output[off..off+4].copy_from_slice(&bytes);
        }
    }
    r
}
pub fn unpack18_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    // The skeleton signature has input/output types swapped. Convert to bytes/u32 view.
    let in_bytes: Vec<u8> = input.iter().flat_map(|w| w.to_le_bytes()).collect();
    let mut out_u32 = vec![0u32; length as usize];
    let r = unpack_x(base, &in_bytes, &mut out_u32, 18, length);
    // Write back as bytes if possible
    for (i, v) in out_u32.iter().enumerate() {
        let bytes = v.to_le_bytes();
        let off = i * 4;
        if off + 4 <= output.len() {
            output[off..off+4].copy_from_slice(&bytes);
        }
    }
    r
}
pub fn unpack19_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    // The skeleton signature has input/output types swapped. Convert to bytes/u32 view.
    let in_bytes: Vec<u8> = input.iter().flat_map(|w| w.to_le_bytes()).collect();
    let mut out_u32 = vec![0u32; length as usize];
    let r = unpack_x(base, &in_bytes, &mut out_u32, 19, length);
    // Write back as bytes if possible
    for (i, v) in out_u32.iter().enumerate() {
        let bytes = v.to_le_bytes();
        let off = i * 4;
        if off + 4 <= output.len() {
            output[off..off+4].copy_from_slice(&bytes);
        }
    }
    r
}
pub fn unpack20_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    // The skeleton signature has input/output types swapped. Convert to bytes/u32 view.
    let in_bytes: Vec<u8> = input.iter().flat_map(|w| w.to_le_bytes()).collect();
    let mut out_u32 = vec![0u32; length as usize];
    let r = unpack_x(base, &in_bytes, &mut out_u32, 20, length);
    // Write back as bytes if possible
    for (i, v) in out_u32.iter().enumerate() {
        let bytes = v.to_le_bytes();
        let off = i * 4;
        if off + 4 <= output.len() {
            output[off..off+4].copy_from_slice(&bytes);
        }
    }
    r
}
pub fn unpack21_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    // The skeleton signature has input/output types swapped. Convert to bytes/u32 view.
    let in_bytes: Vec<u8> = input.iter().flat_map(|w| w.to_le_bytes()).collect();
    let mut out_u32 = vec![0u32; length as usize];
    let r = unpack_x(base, &in_bytes, &mut out_u32, 21, length);
    // Write back as bytes if possible
    for (i, v) in out_u32.iter().enumerate() {
        let bytes = v.to_le_bytes();
        let off = i * 4;
        if off + 4 <= output.len() {
            output[off..off+4].copy_from_slice(&bytes);
        }
    }
    r
}
pub fn unpack22_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    // The skeleton signature has input/output types swapped. Convert to bytes/u32 view.
    let in_bytes: Vec<u8> = input.iter().flat_map(|w| w.to_le_bytes()).collect();
    let mut out_u32 = vec![0u32; length as usize];
    let r = unpack_x(base, &in_bytes, &mut out_u32, 22, length);
    // Write back as bytes if possible
    for (i, v) in out_u32.iter().enumerate() {
        let bytes = v.to_le_bytes();
        let off = i * 4;
        if off + 4 <= output.len() {
            output[off..off+4].copy_from_slice(&bytes);
        }
    }
    r
}
pub fn unpack23_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    // The skeleton signature has input/output types swapped. Convert to bytes/u32 view.
    let in_bytes: Vec<u8> = input.iter().flat_map(|w| w.to_le_bytes()).collect();
    let mut out_u32 = vec![0u32; length as usize];
    let r = unpack_x(base, &in_bytes, &mut out_u32, 23, length);
    // Write back as bytes if possible
    for (i, v) in out_u32.iter().enumerate() {
        let bytes = v.to_le_bytes();
        let off = i * 4;
        if off + 4 <= output.len() {
            output[off..off+4].copy_from_slice(&bytes);
        }
    }
    r
}
pub fn unpack24_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    // The skeleton signature has input/output types swapped. Convert to bytes/u32 view.
    let in_bytes: Vec<u8> = input.iter().flat_map(|w| w.to_le_bytes()).collect();
    let mut out_u32 = vec![0u32; length as usize];
    let r = unpack_x(base, &in_bytes, &mut out_u32, 24, length);
    // Write back as bytes if possible
    for (i, v) in out_u32.iter().enumerate() {
        let bytes = v.to_le_bytes();
        let off = i * 4;
        if off + 4 <= output.len() {
            output[off..off+4].copy_from_slice(&bytes);
        }
    }
    r
}
pub fn unpack25_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    // The skeleton signature has input/output types swapped. Convert to bytes/u32 view.
    let in_bytes: Vec<u8> = input.iter().flat_map(|w| w.to_le_bytes()).collect();
    let mut out_u32 = vec![0u32; length as usize];
    let r = unpack_x(base, &in_bytes, &mut out_u32, 25, length);
    // Write back as bytes if possible
    for (i, v) in out_u32.iter().enumerate() {
        let bytes = v.to_le_bytes();
        let off = i * 4;
        if off + 4 <= output.len() {
            output[off..off+4].copy_from_slice(&bytes);
        }
    }
    r
}
pub fn unpack26_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    // The skeleton signature has input/output types swapped. Convert to bytes/u32 view.
    let in_bytes: Vec<u8> = input.iter().flat_map(|w| w.to_le_bytes()).collect();
    let mut out_u32 = vec![0u32; length as usize];
    let r = unpack_x(base, &in_bytes, &mut out_u32, 26, length);
    // Write back as bytes if possible
    for (i, v) in out_u32.iter().enumerate() {
        let bytes = v.to_le_bytes();
        let off = i * 4;
        if off + 4 <= output.len() {
            output[off..off+4].copy_from_slice(&bytes);
        }
    }
    r
}
pub fn unpack27_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    // The skeleton signature has input/output types swapped. Convert to bytes/u32 view.
    let in_bytes: Vec<u8> = input.iter().flat_map(|w| w.to_le_bytes()).collect();
    let mut out_u32 = vec![0u32; length as usize];
    let r = unpack_x(base, &in_bytes, &mut out_u32, 27, length);
    // Write back as bytes if possible
    for (i, v) in out_u32.iter().enumerate() {
        let bytes = v.to_le_bytes();
        let off = i * 4;
        if off + 4 <= output.len() {
            output[off..off+4].copy_from_slice(&bytes);
        }
    }
    r
}
pub fn unpack28_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    // The skeleton signature has input/output types swapped. Convert to bytes/u32 view.
    let in_bytes: Vec<u8> = input.iter().flat_map(|w| w.to_le_bytes()).collect();
    let mut out_u32 = vec![0u32; length as usize];
    let r = unpack_x(base, &in_bytes, &mut out_u32, 28, length);
    // Write back as bytes if possible
    for (i, v) in out_u32.iter().enumerate() {
        let bytes = v.to_le_bytes();
        let off = i * 4;
        if off + 4 <= output.len() {
            output[off..off+4].copy_from_slice(&bytes);
        }
    }
    r
}
pub fn unpack29_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    // The skeleton signature has input/output types swapped. Convert to bytes/u32 view.
    let in_bytes: Vec<u8> = input.iter().flat_map(|w| w.to_le_bytes()).collect();
    let mut out_u32 = vec![0u32; length as usize];
    let r = unpack_x(base, &in_bytes, &mut out_u32, 29, length);
    // Write back as bytes if possible
    for (i, v) in out_u32.iter().enumerate() {
        let bytes = v.to_le_bytes();
        let off = i * 4;
        if off + 4 <= output.len() {
            output[off..off+4].copy_from_slice(&bytes);
        }
    }
    r
}
pub fn unpack30_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    // The skeleton signature has input/output types swapped. Convert to bytes/u32 view.
    let in_bytes: Vec<u8> = input.iter().flat_map(|w| w.to_le_bytes()).collect();
    let mut out_u32 = vec![0u32; length as usize];
    let r = unpack_x(base, &in_bytes, &mut out_u32, 30, length);
    // Write back as bytes if possible
    for (i, v) in out_u32.iter().enumerate() {
        let bytes = v.to_le_bytes();
        let off = i * 4;
        if off + 4 <= output.len() {
            output[off..off+4].copy_from_slice(&bytes);
        }
    }
    r
}
pub fn unpack31_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    // The skeleton signature has input/output types swapped. Convert to bytes/u32 view.
    let in_bytes: Vec<u8> = input.iter().flat_map(|w| w.to_le_bytes()).collect();
    let mut out_u32 = vec![0u32; length as usize];
    let r = unpack_x(base, &in_bytes, &mut out_u32, 31, length);
    // Write back as bytes if possible
    for (i, v) in out_u32.iter().enumerate() {
        let bytes = v.to_le_bytes();
        let off = i * 4;
        if off + 4 <= output.len() {
            output[off..off+4].copy_from_slice(&bytes);
        }
    }
    r
}
pub fn unpack32_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    // The skeleton signature has input/output types swapped. Convert to bytes/u32 view.
    let in_bytes: Vec<u8> = input.iter().flat_map(|w| w.to_le_bytes()).collect();
    let mut out_u32 = vec![0u32; length as usize];
    let r = unpack_x(base, &in_bytes, &mut out_u32, 32, length);
    // Write back as bytes if possible
    for (i, v) in out_u32.iter().enumerate() {
        let bytes = v.to_le_bytes();
        let off = i * 4;
        if off + 4 <= output.len() {
            output[off..off+4].copy_from_slice(&bytes);
        }
    }
    r
}
pub fn linsearch1_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 1, 32) }
pub fn linsearch2_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 2, 32) }
pub fn linsearch3_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 3, 32) }
pub fn linsearch4_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 4, 32) }
pub fn linsearch5_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 5, 32) }
pub fn linsearch6_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 6, 32) }
pub fn linsearch7_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 7, 32) }
pub fn linsearch8_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 8, 32) }
pub fn linsearch9_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 9, 32) }
pub fn linsearch10_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 10, 32) }
pub fn linsearch11_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 11, 32) }
pub fn linsearch12_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 12, 32) }
pub fn linsearch13_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 13, 32) }
pub fn linsearch14_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 14, 32) }
pub fn linsearch15_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 15, 32) }
pub fn linsearch16_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 16, 32) }
pub fn linsearch17_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 17, 32) }
pub fn linsearch18_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 18, 32) }
pub fn linsearch19_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 19, 32) }
pub fn linsearch20_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 20, 32) }
pub fn linsearch21_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 21, 32) }
pub fn linsearch22_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 22, 32) }
pub fn linsearch23_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 23, 32) }
pub fn linsearch24_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 24, 32) }
pub fn linsearch25_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 25, 32) }
pub fn linsearch26_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 26, 32) }
pub fn linsearch27_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 27, 32) }
pub fn linsearch28_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 28, 32) }
pub fn linsearch29_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 29, 32) }
pub fn linsearch30_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 30, 32) }
pub fn linsearch31_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 31, 32) }
pub fn linsearch32_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 32, 32) }
pub fn linsearch1_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 1, 16) }
pub fn linsearch2_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 2, 16) }
pub fn linsearch3_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 3, 16) }
pub fn linsearch4_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 4, 16) }
pub fn linsearch5_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 5, 16) }
pub fn linsearch6_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 6, 16) }
pub fn linsearch7_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 7, 16) }
pub fn linsearch8_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 8, 16) }
pub fn linsearch9_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 9, 16) }
pub fn linsearch10_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 10, 16) }
pub fn linsearch11_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 11, 16) }
pub fn linsearch12_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 12, 16) }
pub fn linsearch13_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 13, 16) }
pub fn linsearch14_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 14, 16) }
pub fn linsearch15_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 15, 16) }
pub fn linsearch16_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 16, 16) }
pub fn linsearch17_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 17, 16) }
pub fn linsearch18_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 18, 16) }
pub fn linsearch19_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 19, 16) }
pub fn linsearch20_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 20, 16) }
pub fn linsearch21_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 21, 16) }
pub fn linsearch22_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 22, 16) }
pub fn linsearch23_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 23, 16) }
pub fn linsearch24_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 24, 16) }
pub fn linsearch25_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 25, 16) }
pub fn linsearch26_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 26, 16) }
pub fn linsearch27_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 27, 16) }
pub fn linsearch28_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 28, 16) }
pub fn linsearch29_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 29, 16) }
pub fn linsearch30_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 30, 16) }
pub fn linsearch31_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 31, 16) }
pub fn linsearch32_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 32, 16) }
pub fn linsearch1_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 1, 8) }
pub fn linsearch2_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 2, 8) }
pub fn linsearch3_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 3, 8) }
pub fn linsearch4_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 4, 8) }
pub fn linsearch5_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 5, 8) }
pub fn linsearch6_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 6, 8) }
pub fn linsearch7_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 7, 8) }
pub fn linsearch8_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 8, 8) }
pub fn linsearch9_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 9, 8) }
pub fn linsearch10_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 10, 8) }
pub fn linsearch11_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 11, 8) }
pub fn linsearch12_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 12, 8) }
pub fn linsearch13_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 13, 8) }
pub fn linsearch14_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 14, 8) }
pub fn linsearch15_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 15, 8) }
pub fn linsearch16_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 16, 8) }
pub fn linsearch17_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 17, 8) }
pub fn linsearch18_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 18, 8) }
pub fn linsearch19_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 19, 8) }
pub fn linsearch20_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 20, 8) }
pub fn linsearch21_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 21, 8) }
pub fn linsearch22_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 22, 8) }
pub fn linsearch23_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 23, 8) }
pub fn linsearch24_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 24, 8) }
pub fn linsearch25_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 25, 8) }
pub fn linsearch26_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 26, 8) }
pub fn linsearch27_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 27, 8) }
pub fn linsearch28_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 28, 8) }
pub fn linsearch29_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 29, 8) }
pub fn linsearch30_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 30, 8) }
pub fn linsearch31_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 31, 8) }
pub fn linsearch32_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 { linsearch_block(base, input, value, found, 32, 8) }
pub fn linsearch1_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 { linsearch_x(base, input, length, value, found, 1) }
pub fn linsearch2_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 { linsearch_x(base, input, length, value, found, 2) }
pub fn linsearch3_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 { linsearch_x(base, input, length, value, found, 3) }
pub fn linsearch4_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 { linsearch_x(base, input, length, value, found, 4) }
pub fn linsearch5_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 { linsearch_x(base, input, length, value, found, 5) }
pub fn linsearch6_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 { linsearch_x(base, input, length, value, found, 6) }
pub fn linsearch7_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 { linsearch_x(base, input, length, value, found, 7) }
pub fn linsearch8_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 { linsearch_x(base, input, length, value, found, 8) }
pub fn linsearch9_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 { linsearch_x(base, input, length, value, found, 9) }
pub fn linsearch10_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 { linsearch_x(base, input, length, value, found, 10) }
pub fn linsearch11_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 { linsearch_x(base, input, length, value, found, 11) }
pub fn linsearch12_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 { linsearch_x(base, input, length, value, found, 12) }
pub fn linsearch13_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 { linsearch_x(base, input, length, value, found, 13) }
pub fn linsearch14_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 { linsearch_x(base, input, length, value, found, 14) }
pub fn linsearch15_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 { linsearch_x(base, input, length, value, found, 15) }
pub fn linsearch16_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 { linsearch_x(base, input, length, value, found, 16) }
pub fn linsearch17_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 { linsearch_x(base, input, length, value, found, 17) }
pub fn linsearch18_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 { linsearch_x(base, input, length, value, found, 18) }
pub fn linsearch19_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 { linsearch_x(base, input, length, value, found, 19) }
pub fn linsearch20_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 { linsearch_x(base, input, length, value, found, 20) }
pub fn linsearch21_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 { linsearch_x(base, input, length, value, found, 21) }
pub fn linsearch22_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 { linsearch_x(base, input, length, value, found, 22) }
pub fn linsearch23_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 { linsearch_x(base, input, length, value, found, 23) }
pub fn linsearch24_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 { linsearch_x(base, input, length, value, found, 24) }
pub fn linsearch25_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 { linsearch_x(base, input, length, value, found, 25) }
pub fn linsearch26_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 { linsearch_x(base, input, length, value, found, 26) }
pub fn linsearch27_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 { linsearch_x(base, input, length, value, found, 27) }
pub fn linsearch28_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 { linsearch_x(base, input, length, value, found, 28) }
pub fn linsearch29_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 { linsearch_x(base, input, length, value, found, 29) }
pub fn linsearch30_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 { linsearch_x(base, input, length, value, found, 30) }
pub fn linsearch31_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 { linsearch_x(base, input, length, value, found, 31) }
pub fn linsearch32_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 { linsearch_x(base, input, length, value, found, 32) }
