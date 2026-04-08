// Generic helpers for bit-packing/unpacking Frame of Reference encoding.
// The byte buffer is treated as a stream of little-endian u32 words.

// Pack `count` values from `input` into `output` at `bits` bits each.
// Returns bytes written.
fn generic_pack(base: u32, input: &[u32], output: &mut [u8], bits: u32, count: usize) -> u32 {
    if bits == 0 || count == 0 {
        return 0;
    }
    if bits == 32 {
        for i in 0..count {
            let v = input[i].wrapping_sub(base);
            let off = i * 4;
            output[off..off + 4].copy_from_slice(&v.to_le_bytes());
        }
        return (count * 4) as u32;
    }
    let mask = if bits >= 32 { u32::MAX } else { (1u32 << bits) - 1 };
    let total_bits = count as u64 * bits as u64;
    let total_bytes = ((total_bits + 7) / 8) as usize;
    // Zero out destination
    for b in output[..total_bytes].iter_mut() { *b = 0; }
    let mut bit_pos: u64 = 0;
    for i in 0..count {
        let v = (input[i].wrapping_sub(base)) & mask;
        let byte_off = (bit_pos / 8) as usize;
        let bit_off = (bit_pos % 8) as u32;
        // We may need to write across up to 5 bytes for worst case (bit_off=7, bits=32)
        let mut remaining = bits;
        let mut shift = 0u32;
        let mut bo = byte_off;
        let mut boff = bit_off;
        while remaining > 0 {
            let space = 8 - boff;
            let write_bits = remaining.min(space);
            let byte_mask = ((1u32 << write_bits) - 1) as u8;
            output[bo] |= (((v >> shift) as u8) & byte_mask) << boff;
            remaining -= write_bits;
            shift += write_bits;
            bo += 1;
            boff = 0;
        }
        bit_pos += bits as u64;
    }
    total_bytes as u32
}

// Unpack `count` values from `input` into `output` at `bits` bits each.
// Returns bytes consumed.
fn generic_unpack(base: u32, input: &[u8], output: &mut [u32], bits: u32, count: usize) -> u32 {
    if bits == 0 {
        for i in 0..count { output[i] = base; }
        return 0;
    }
    if count == 0 { return 0; }
    if bits == 32 {
        for i in 0..count {
            let off = i * 4;
            let v = u32::from_le_bytes([input[off], input[off+1], input[off+2], input[off+3]]);
            output[i] = base.wrapping_add(v);
        }
        return (count * 4) as u32;
    }
    let mask = (1u32 << bits) - 1;
    let mut bit_pos: u64 = 0;
    for i in 0..count {
        let byte_off = (bit_pos / 8) as usize;
        let bit_off = (bit_pos % 8) as u32;
        // Read enough bytes
        let mut v: u64 = 0;
        let bytes_needed = ((bit_off + bits + 7) / 8) as usize;
        for j in 0..bytes_needed {
            if byte_off + j < input.len() {
                v |= (input[byte_off + j] as u64) << (j * 8);
            }
        }
        output[i] = base.wrapping_add(((v >> bit_off) as u32) & mask);
        bit_pos += bits as u64;
    }
    let total_bits = count as u64 * bits as u64;
    ((total_bits + 7) / 8) as u32
}

// packx: pack variable length (up to 8), returns bytes written
fn generic_packx(base: u32, input: &[u32], output: &mut [u8], bits: u32, length: u32) -> u32 {
    if length == 0 || bits == 0 { return 0; }
    generic_pack(base, input, output, bits, length as usize)
}

// unpackx: unpack variable length (up to 8), returns bytes consumed
fn generic_unpackx(base: u32, input: &[u8], output: &mut [u32], bits: u32, length: u32) -> u32 {
    if length == 0 || bits == 0 {
        for i in 0..length as usize { output[i] = base; }
        return 0;
    }
    generic_unpack(base, input, output, bits, length as usize)
}

// Linear search in packed data for `value`. Sets `found` to index if found.
// Returns: when found, returns the index; when not found, returns bytes consumed.
fn generic_linsearch(base: u32, input: &[u8], bits: u32, count: usize, value: u32, found: &mut i32) -> u32 {
    if bits == 0 {
        if value == base && count > 0 { *found = 0; }
        return 0;
    }
    if bits == 32 {
        let target = value.wrapping_sub(base);
        for i in 0..count {
            let off = i * 4;
            let v = u32::from_le_bytes([input[off], input[off+1], input[off+2], input[off+3]]);
            if v == target {
                *found = i as i32;
                return 0;
            }
        }
        return (count * 4) as u32;
    }
    let mask = (1u32 << bits) - 1;
    let target = value.wrapping_sub(base);
    let mut bit_pos: u64 = 0;
    for i in 0..count {
        let byte_off = (bit_pos / 8) as usize;
        let bit_off = (bit_pos % 8) as u32;
        let mut v: u64 = 0;
        let bytes_needed = ((bit_off + bits + 7) / 8) as usize;
        for j in 0..bytes_needed {
            if byte_off + j < input.len() {
                v |= (input[byte_off + j] as u64) << (j * 8);
            }
        }
        let decoded = ((v >> bit_off) as u32) & mask;
        if decoded == target {
            *found = i as i32;
            return i as u32;
        }
        bit_pos += bits as u64;
    }
    let total_bits = count as u64 * bits as u64;
    ((total_bits + 7) / 8) as u32
}

// linsearchx: linear search variable length
fn generic_linsearchx(base: u32, input: &[u8], length: u32, bits: u32, value: u32, found: &mut i32) -> u32 {
    if length == 0 { return 0; }
    generic_linsearch(base, input, bits, length as usize, value, found)
}

// ============ pack0 / unpack0 / linsearch0 special cases ============

pub fn pack0_32(_base: u32, _input: &[u32], _output: &mut [u8]) -> u32 { 0 }
pub fn pack0_16(_base: u32, _input: &[u32], _output: &mut [u8]) -> u32 { 0 }
pub fn pack0_8(_base: u32, _input: &[u32], _output: &mut [u8]) -> u32 { 0 }
pub fn unpack0_32(base: u32, _input: &[u8], output: &mut [u32]) -> u32 {
    for k in 0..32 { output[k] = base; } 0
}
pub fn unpack0_16(base: u32, _input: &[u8], output: &mut [u32]) -> u32 {
    for k in 0..16 { output[k] = base; } 0
}
pub fn unpack0_8(base: u32, _input: &[u8], output: &mut [u32]) -> u32 {
    for k in 0..8 { output[k] = base; } 0
}
pub fn pack0_x(_base: u32, _input: &[u32], _output: &mut [u8], _length: u32) -> u32 { 0 }
pub fn unpack0_x(base: u32, _input: &[u8], output: &mut [u32], length: u32) -> u32 {
    for k in 0..length as usize { output[k] = base; } 0
}
pub fn linsearch0_32(base: u32, _input: &[u8], value: u32, found: &mut i32) -> u32 {
    if value == base { *found = 0; } 0
}
pub fn linsearch0_16(base: u32, _input: &[u8], value: u32, found: &mut i32) -> u32 {
    if value == base { *found = 0; } 0
}
pub fn linsearch0_8(base: u32, _input: &[u8], value: u32, found: &mut i32) -> u32 {
    if value == base { *found = 0; } 0
}
pub fn linsearch0_x(_base: u32, _input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    if length == 0 { return 0; }
    if value == _base { *found = 0; } 0
}
pub fn pack1_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 1, 32)
}
pub fn pack1_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 1, 16)
}
pub fn pack1_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 1, 8)
}
pub fn unpack1_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 1, 32)
}
pub fn unpack1_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 1, 16)
}
pub fn unpack1_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 1, 8)
}
pub fn pack1_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    generic_packx(base, input, output, 1, length)
}
pub fn unpack1_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpackx(base, inp_bytes, out_u32, 1, length)
}
pub fn linsearch1_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 1, 32, value, found)
}
pub fn linsearch1_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 1, 16, value, found)
}
pub fn linsearch1_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 1, 8, value, found)
}
pub fn linsearch1_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    generic_linsearchx(base, input, length, 1, value, found)
}
pub fn pack2_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 2, 32)
}
pub fn pack2_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 2, 16)
}
pub fn pack2_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 2, 8)
}
pub fn unpack2_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 2, 32)
}
pub fn unpack2_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 2, 16)
}
pub fn unpack2_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 2, 8)
}
pub fn pack2_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    generic_packx(base, input, output, 2, length)
}
pub fn unpack2_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpackx(base, inp_bytes, out_u32, 2, length)
}
pub fn linsearch2_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 2, 32, value, found)
}
pub fn linsearch2_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 2, 16, value, found)
}
pub fn linsearch2_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 2, 8, value, found)
}
pub fn linsearch2_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    generic_linsearchx(base, input, length, 2, value, found)
}
pub fn pack3_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 3, 32)
}
pub fn pack3_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 3, 16)
}
pub fn pack3_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 3, 8)
}
pub fn unpack3_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 3, 32)
}
pub fn unpack3_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 3, 16)
}
pub fn unpack3_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 3, 8)
}
pub fn pack3_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    generic_packx(base, input, output, 3, length)
}
pub fn unpack3_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpackx(base, inp_bytes, out_u32, 3, length)
}
pub fn linsearch3_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 3, 32, value, found)
}
pub fn linsearch3_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 3, 16, value, found)
}
pub fn linsearch3_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 3, 8, value, found)
}
pub fn linsearch3_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    generic_linsearchx(base, input, length, 3, value, found)
}
pub fn pack4_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 4, 32)
}
pub fn pack4_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 4, 16)
}
pub fn pack4_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 4, 8)
}
pub fn unpack4_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 4, 32)
}
pub fn unpack4_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 4, 16)
}
pub fn unpack4_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 4, 8)
}
pub fn pack4_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    generic_packx(base, input, output, 4, length)
}
pub fn unpack4_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpackx(base, inp_bytes, out_u32, 4, length)
}
pub fn linsearch4_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 4, 32, value, found)
}
pub fn linsearch4_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 4, 16, value, found)
}
pub fn linsearch4_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 4, 8, value, found)
}
pub fn linsearch4_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    generic_linsearchx(base, input, length, 4, value, found)
}
pub fn pack5_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 5, 32)
}
pub fn pack5_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 5, 16)
}
pub fn pack5_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 5, 8)
}
pub fn unpack5_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 5, 32)
}
pub fn unpack5_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 5, 16)
}
pub fn unpack5_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 5, 8)
}
pub fn pack5_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    generic_packx(base, input, output, 5, length)
}
pub fn unpack5_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpackx(base, inp_bytes, out_u32, 5, length)
}
pub fn linsearch5_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 5, 32, value, found)
}
pub fn linsearch5_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 5, 16, value, found)
}
pub fn linsearch5_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 5, 8, value, found)
}
pub fn linsearch5_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    generic_linsearchx(base, input, length, 5, value, found)
}
pub fn pack6_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 6, 32)
}
pub fn pack6_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 6, 16)
}
pub fn pack6_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 6, 8)
}
pub fn unpack6_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 6, 32)
}
pub fn unpack6_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 6, 16)
}
pub fn unpack6_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 6, 8)
}
pub fn pack6_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    generic_packx(base, input, output, 6, length)
}
pub fn unpack6_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpackx(base, inp_bytes, out_u32, 6, length)
}
pub fn linsearch6_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 6, 32, value, found)
}
pub fn linsearch6_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 6, 16, value, found)
}
pub fn linsearch6_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 6, 8, value, found)
}
pub fn linsearch6_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    generic_linsearchx(base, input, length, 6, value, found)
}
pub fn pack7_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 7, 32)
}
pub fn pack7_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 7, 16)
}
pub fn pack7_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 7, 8)
}
pub fn unpack7_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 7, 32)
}
pub fn unpack7_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 7, 16)
}
pub fn unpack7_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 7, 8)
}
pub fn pack7_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    generic_packx(base, input, output, 7, length)
}
pub fn unpack7_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpackx(base, inp_bytes, out_u32, 7, length)
}
pub fn linsearch7_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 7, 32, value, found)
}
pub fn linsearch7_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 7, 16, value, found)
}
pub fn linsearch7_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 7, 8, value, found)
}
pub fn linsearch7_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    generic_linsearchx(base, input, length, 7, value, found)
}
pub fn pack8_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 8, 32)
}
pub fn pack8_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 8, 16)
}
pub fn pack8_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 8, 8)
}
pub fn unpack8_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 8, 32)
}
pub fn unpack8_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 8, 16)
}
pub fn unpack8_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 8, 8)
}
pub fn pack8_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    generic_packx(base, input, output, 8, length)
}
pub fn unpack8_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpackx(base, inp_bytes, out_u32, 8, length)
}
pub fn linsearch8_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 8, 32, value, found)
}
pub fn linsearch8_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 8, 16, value, found)
}
pub fn linsearch8_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 8, 8, value, found)
}
pub fn linsearch8_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    generic_linsearchx(base, input, length, 8, value, found)
}
pub fn pack9_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 9, 32)
}
pub fn pack9_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 9, 16)
}
pub fn pack9_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 9, 8)
}
pub fn unpack9_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 9, 32)
}
pub fn unpack9_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 9, 16)
}
pub fn unpack9_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 9, 8)
}
pub fn pack9_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    generic_packx(base, input, output, 9, length)
}
pub fn unpack9_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpackx(base, inp_bytes, out_u32, 9, length)
}
pub fn linsearch9_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 9, 32, value, found)
}
pub fn linsearch9_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 9, 16, value, found)
}
pub fn linsearch9_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 9, 8, value, found)
}
pub fn linsearch9_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    generic_linsearchx(base, input, length, 9, value, found)
}
pub fn pack10_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 10, 32)
}
pub fn pack10_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 10, 16)
}
pub fn pack10_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 10, 8)
}
pub fn unpack10_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 10, 32)
}
pub fn unpack10_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 10, 16)
}
pub fn unpack10_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 10, 8)
}
pub fn pack10_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    generic_packx(base, input, output, 10, length)
}
pub fn unpack10_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpackx(base, inp_bytes, out_u32, 10, length)
}
pub fn linsearch10_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 10, 32, value, found)
}
pub fn linsearch10_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 10, 16, value, found)
}
pub fn linsearch10_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 10, 8, value, found)
}
pub fn linsearch10_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    generic_linsearchx(base, input, length, 10, value, found)
}
pub fn pack11_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 11, 32)
}
pub fn pack11_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 11, 16)
}
pub fn pack11_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 11, 8)
}
pub fn unpack11_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 11, 32)
}
pub fn unpack11_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 11, 16)
}
pub fn unpack11_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 11, 8)
}
pub fn pack11_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    generic_packx(base, input, output, 11, length)
}
pub fn unpack11_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpackx(base, inp_bytes, out_u32, 11, length)
}
pub fn linsearch11_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 11, 32, value, found)
}
pub fn linsearch11_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 11, 16, value, found)
}
pub fn linsearch11_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 11, 8, value, found)
}
pub fn linsearch11_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    generic_linsearchx(base, input, length, 11, value, found)
}
pub fn pack12_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 12, 32)
}
pub fn pack12_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 12, 16)
}
pub fn pack12_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 12, 8)
}
pub fn unpack12_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 12, 32)
}
pub fn unpack12_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 12, 16)
}
pub fn unpack12_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 12, 8)
}
pub fn pack12_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    generic_packx(base, input, output, 12, length)
}
pub fn unpack12_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpackx(base, inp_bytes, out_u32, 12, length)
}
pub fn linsearch12_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 12, 32, value, found)
}
pub fn linsearch12_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 12, 16, value, found)
}
pub fn linsearch12_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 12, 8, value, found)
}
pub fn linsearch12_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    generic_linsearchx(base, input, length, 12, value, found)
}
pub fn pack13_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 13, 32)
}
pub fn pack13_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 13, 16)
}
pub fn pack13_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 13, 8)
}
pub fn unpack13_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 13, 32)
}
pub fn unpack13_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 13, 16)
}
pub fn unpack13_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 13, 8)
}
pub fn pack13_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    generic_packx(base, input, output, 13, length)
}
pub fn unpack13_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpackx(base, inp_bytes, out_u32, 13, length)
}
pub fn linsearch13_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 13, 32, value, found)
}
pub fn linsearch13_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 13, 16, value, found)
}
pub fn linsearch13_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 13, 8, value, found)
}
pub fn linsearch13_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    generic_linsearchx(base, input, length, 13, value, found)
}
pub fn pack14_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 14, 32)
}
pub fn pack14_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 14, 16)
}
pub fn pack14_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 14, 8)
}
pub fn unpack14_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 14, 32)
}
pub fn unpack14_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 14, 16)
}
pub fn unpack14_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 14, 8)
}
pub fn pack14_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    generic_packx(base, input, output, 14, length)
}
pub fn unpack14_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpackx(base, inp_bytes, out_u32, 14, length)
}
pub fn linsearch14_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 14, 32, value, found)
}
pub fn linsearch14_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 14, 16, value, found)
}
pub fn linsearch14_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 14, 8, value, found)
}
pub fn linsearch14_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    generic_linsearchx(base, input, length, 14, value, found)
}
pub fn pack15_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 15, 32)
}
pub fn pack15_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 15, 16)
}
pub fn pack15_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 15, 8)
}
pub fn unpack15_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 15, 32)
}
pub fn unpack15_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 15, 16)
}
pub fn unpack15_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 15, 8)
}
pub fn pack15_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    generic_packx(base, input, output, 15, length)
}
pub fn unpack15_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpackx(base, inp_bytes, out_u32, 15, length)
}
pub fn linsearch15_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 15, 32, value, found)
}
pub fn linsearch15_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 15, 16, value, found)
}
pub fn linsearch15_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 15, 8, value, found)
}
pub fn linsearch15_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    generic_linsearchx(base, input, length, 15, value, found)
}
pub fn pack16_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 16, 32)
}
pub fn pack16_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 16, 16)
}
pub fn pack16_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 16, 8)
}
pub fn unpack16_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 16, 32)
}
pub fn unpack16_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 16, 16)
}
pub fn unpack16_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 16, 8)
}
pub fn pack16_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    generic_packx(base, input, output, 16, length)
}
pub fn unpack16_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpackx(base, inp_bytes, out_u32, 16, length)
}
pub fn linsearch16_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 16, 32, value, found)
}
pub fn linsearch16_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 16, 16, value, found)
}
pub fn linsearch16_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 16, 8, value, found)
}
pub fn linsearch16_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    generic_linsearchx(base, input, length, 16, value, found)
}
pub fn pack17_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 17, 32)
}
pub fn pack17_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 17, 16)
}
pub fn pack17_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 17, 8)
}
pub fn unpack17_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 17, 32)
}
pub fn unpack17_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 17, 16)
}
pub fn unpack17_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 17, 8)
}
pub fn pack17_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    generic_packx(base, input, output, 17, length)
}
pub fn unpack17_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpackx(base, inp_bytes, out_u32, 17, length)
}
pub fn linsearch17_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 17, 32, value, found)
}
pub fn linsearch17_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 17, 16, value, found)
}
pub fn linsearch17_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 17, 8, value, found)
}
pub fn linsearch17_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    generic_linsearchx(base, input, length, 17, value, found)
}
pub fn pack18_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 18, 32)
}
pub fn pack18_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 18, 16)
}
pub fn pack18_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 18, 8)
}
pub fn unpack18_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 18, 32)
}
pub fn unpack18_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 18, 16)
}
pub fn unpack18_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 18, 8)
}
pub fn pack18_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    generic_packx(base, input, output, 18, length)
}
pub fn unpack18_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpackx(base, inp_bytes, out_u32, 18, length)
}
pub fn linsearch18_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 18, 32, value, found)
}
pub fn linsearch18_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 18, 16, value, found)
}
pub fn linsearch18_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 18, 8, value, found)
}
pub fn linsearch18_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    generic_linsearchx(base, input, length, 18, value, found)
}
pub fn pack19_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 19, 32)
}
pub fn pack19_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 19, 16)
}
pub fn pack19_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 19, 8)
}
pub fn unpack19_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 19, 32)
}
pub fn unpack19_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 19, 16)
}
pub fn unpack19_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 19, 8)
}
pub fn pack19_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    generic_packx(base, input, output, 19, length)
}
pub fn unpack19_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpackx(base, inp_bytes, out_u32, 19, length)
}
pub fn linsearch19_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 19, 32, value, found)
}
pub fn linsearch19_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 19, 16, value, found)
}
pub fn linsearch19_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 19, 8, value, found)
}
pub fn linsearch19_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    generic_linsearchx(base, input, length, 19, value, found)
}
pub fn pack20_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 20, 32)
}
pub fn pack20_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 20, 16)
}
pub fn pack20_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 20, 8)
}
pub fn unpack20_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 20, 32)
}
pub fn unpack20_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 20, 16)
}
pub fn unpack20_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 20, 8)
}
pub fn pack20_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    generic_packx(base, input, output, 20, length)
}
pub fn unpack20_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpackx(base, inp_bytes, out_u32, 20, length)
}
pub fn linsearch20_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 20, 32, value, found)
}
pub fn linsearch20_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 20, 16, value, found)
}
pub fn linsearch20_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 20, 8, value, found)
}
pub fn linsearch20_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    generic_linsearchx(base, input, length, 20, value, found)
}
pub fn pack21_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 21, 32)
}
pub fn pack21_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 21, 16)
}
pub fn pack21_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 21, 8)
}
pub fn unpack21_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 21, 32)
}
pub fn unpack21_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 21, 16)
}
pub fn unpack21_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 21, 8)
}
pub fn pack21_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    generic_packx(base, input, output, 21, length)
}
pub fn unpack21_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpackx(base, inp_bytes, out_u32, 21, length)
}
pub fn linsearch21_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 21, 32, value, found)
}
pub fn linsearch21_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 21, 16, value, found)
}
pub fn linsearch21_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 21, 8, value, found)
}
pub fn linsearch21_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    generic_linsearchx(base, input, length, 21, value, found)
}
pub fn pack22_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 22, 32)
}
pub fn pack22_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 22, 16)
}
pub fn pack22_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 22, 8)
}
pub fn unpack22_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 22, 32)
}
pub fn unpack22_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 22, 16)
}
pub fn unpack22_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 22, 8)
}
pub fn pack22_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    generic_packx(base, input, output, 22, length)
}
pub fn unpack22_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpackx(base, inp_bytes, out_u32, 22, length)
}
pub fn linsearch22_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 22, 32, value, found)
}
pub fn linsearch22_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 22, 16, value, found)
}
pub fn linsearch22_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 22, 8, value, found)
}
pub fn linsearch22_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    generic_linsearchx(base, input, length, 22, value, found)
}
pub fn pack23_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 23, 32)
}
pub fn pack23_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 23, 16)
}
pub fn pack23_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 23, 8)
}
pub fn unpack23_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 23, 32)
}
pub fn unpack23_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 23, 16)
}
pub fn unpack23_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 23, 8)
}
pub fn pack23_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    generic_packx(base, input, output, 23, length)
}
pub fn unpack23_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpackx(base, inp_bytes, out_u32, 23, length)
}
pub fn linsearch23_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 23, 32, value, found)
}
pub fn linsearch23_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 23, 16, value, found)
}
pub fn linsearch23_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 23, 8, value, found)
}
pub fn linsearch23_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    generic_linsearchx(base, input, length, 23, value, found)
}
pub fn pack24_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 24, 32)
}
pub fn pack24_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 24, 16)
}
pub fn pack24_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 24, 8)
}
pub fn unpack24_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 24, 32)
}
pub fn unpack24_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 24, 16)
}
pub fn unpack24_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 24, 8)
}
pub fn pack24_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    generic_packx(base, input, output, 24, length)
}
pub fn unpack24_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpackx(base, inp_bytes, out_u32, 24, length)
}
pub fn linsearch24_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 24, 32, value, found)
}
pub fn linsearch24_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 24, 16, value, found)
}
pub fn linsearch24_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 24, 8, value, found)
}
pub fn linsearch24_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    generic_linsearchx(base, input, length, 24, value, found)
}
pub fn pack25_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 25, 32)
}
pub fn pack25_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 25, 16)
}
pub fn pack25_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 25, 8)
}
pub fn unpack25_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 25, 32)
}
pub fn unpack25_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 25, 16)
}
pub fn unpack25_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 25, 8)
}
pub fn pack25_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    generic_packx(base, input, output, 25, length)
}
pub fn unpack25_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpackx(base, inp_bytes, out_u32, 25, length)
}
pub fn linsearch25_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 25, 32, value, found)
}
pub fn linsearch25_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 25, 16, value, found)
}
pub fn linsearch25_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 25, 8, value, found)
}
pub fn linsearch25_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    generic_linsearchx(base, input, length, 25, value, found)
}
pub fn pack26_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 26, 32)
}
pub fn pack26_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 26, 16)
}
pub fn pack26_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 26, 8)
}
pub fn unpack26_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 26, 32)
}
pub fn unpack26_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 26, 16)
}
pub fn unpack26_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 26, 8)
}
pub fn pack26_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    generic_packx(base, input, output, 26, length)
}
pub fn unpack26_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpackx(base, inp_bytes, out_u32, 26, length)
}
pub fn linsearch26_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 26, 32, value, found)
}
pub fn linsearch26_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 26, 16, value, found)
}
pub fn linsearch26_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 26, 8, value, found)
}
pub fn linsearch26_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    generic_linsearchx(base, input, length, 26, value, found)
}
pub fn pack27_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 27, 32)
}
pub fn pack27_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 27, 16)
}
pub fn pack27_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 27, 8)
}
pub fn unpack27_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 27, 32)
}
pub fn unpack27_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 27, 16)
}
pub fn unpack27_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 27, 8)
}
pub fn pack27_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    generic_packx(base, input, output, 27, length)
}
pub fn unpack27_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpackx(base, inp_bytes, out_u32, 27, length)
}
pub fn linsearch27_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 27, 32, value, found)
}
pub fn linsearch27_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 27, 16, value, found)
}
pub fn linsearch27_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 27, 8, value, found)
}
pub fn linsearch27_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    generic_linsearchx(base, input, length, 27, value, found)
}
pub fn pack28_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 28, 32)
}
pub fn pack28_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 28, 16)
}
pub fn pack28_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 28, 8)
}
pub fn unpack28_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 28, 32)
}
pub fn unpack28_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 28, 16)
}
pub fn unpack28_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 28, 8)
}
pub fn pack28_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    generic_packx(base, input, output, 28, length)
}
pub fn unpack28_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpackx(base, inp_bytes, out_u32, 28, length)
}
pub fn linsearch28_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 28, 32, value, found)
}
pub fn linsearch28_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 28, 16, value, found)
}
pub fn linsearch28_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 28, 8, value, found)
}
pub fn linsearch28_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    generic_linsearchx(base, input, length, 28, value, found)
}
pub fn pack29_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 29, 32)
}
pub fn pack29_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 29, 16)
}
pub fn pack29_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 29, 8)
}
pub fn unpack29_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 29, 32)
}
pub fn unpack29_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 29, 16)
}
pub fn unpack29_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 29, 8)
}
pub fn pack29_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    generic_packx(base, input, output, 29, length)
}
pub fn unpack29_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpackx(base, inp_bytes, out_u32, 29, length)
}
pub fn linsearch29_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 29, 32, value, found)
}
pub fn linsearch29_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 29, 16, value, found)
}
pub fn linsearch29_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 29, 8, value, found)
}
pub fn linsearch29_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    generic_linsearchx(base, input, length, 29, value, found)
}
pub fn pack30_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 30, 32)
}
pub fn pack30_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 30, 16)
}
pub fn pack30_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 30, 8)
}
pub fn unpack30_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 30, 32)
}
pub fn unpack30_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 30, 16)
}
pub fn unpack30_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 30, 8)
}
pub fn pack30_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    generic_packx(base, input, output, 30, length)
}
pub fn unpack30_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpackx(base, inp_bytes, out_u32, 30, length)
}
pub fn linsearch30_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 30, 32, value, found)
}
pub fn linsearch30_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 30, 16, value, found)
}
pub fn linsearch30_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 30, 8, value, found)
}
pub fn linsearch30_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    generic_linsearchx(base, input, length, 30, value, found)
}
pub fn pack31_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 31, 32)
}
pub fn pack31_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 31, 16)
}
pub fn pack31_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 31, 8)
}
pub fn unpack31_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 31, 32)
}
pub fn unpack31_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 31, 16)
}
pub fn unpack31_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 31, 8)
}
pub fn pack31_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    generic_packx(base, input, output, 31, length)
}
pub fn unpack31_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpackx(base, inp_bytes, out_u32, 31, length)
}
pub fn linsearch31_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 31, 32, value, found)
}
pub fn linsearch31_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 31, 16, value, found)
}
pub fn linsearch31_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 31, 8, value, found)
}
pub fn linsearch31_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    generic_linsearchx(base, input, length, 31, value, found)
}
pub fn pack32_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 32, 32)
}
pub fn pack32_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 32, 16)
}
pub fn pack32_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, 32, 8)
}
pub fn unpack32_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 32, 32)
}
pub fn unpack32_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 32, 16)
}
pub fn unpack32_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpack(base, inp_bytes, out_u32, 32, 8)
}
pub fn pack32_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    generic_packx(base, input, output, 32, length)
}
pub fn unpack32_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    let inp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const u8, input.len() * 4) };
    let out_u32: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u32, output.len() / 4) };
    generic_unpackx(base, inp_bytes, out_u32, 32, length)
}
pub fn linsearch32_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 32, 32, value, found)
}
pub fn linsearch32_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 32, 16, value, found)
}
pub fn linsearch32_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, 32, 8, value, found)
}
pub fn linsearch32_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    generic_linsearchx(base, input, length, 32, value, found)
}

// Dispatch functions used by forLib.rs

pub fn dispatch_pack32(bits: u32, base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, bits, 32)
}
pub fn dispatch_pack16(bits: u32, base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, bits, 16)
}
pub fn dispatch_pack8(bits: u32, base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    generic_pack(base, input, output, bits, 8)
}
pub fn dispatch_packx(bits: u32, base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    generic_packx(base, input, output, bits, length)
}
pub fn dispatch_unpack32(bits: u32, base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    generic_unpack(base, input, output, bits, 32)
}
pub fn dispatch_unpack16(bits: u32, base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    generic_unpack(base, input, output, bits, 16)
}
pub fn dispatch_unpack8(bits: u32, base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    generic_unpack(base, input, output, bits, 8)
}
pub fn dispatch_unpackx(bits: u32, base: u32, input: &[u8], output: &mut [u32], length: u32) -> u32 {
    generic_unpackx(base, input, output, bits, length)
}
pub fn dispatch_linsearch32(bits: u32, base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, bits, 32, value, found)
}
pub fn dispatch_linsearch16(bits: u32, base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, bits, 16, value, found)
}
pub fn dispatch_linsearch8(bits: u32, base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    generic_linsearch(base, input, bits, 8, value, found)
}
pub fn dispatch_linsearchx(bits: u32, base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    generic_linsearchx(base, input, length, bits, value, found)
}
